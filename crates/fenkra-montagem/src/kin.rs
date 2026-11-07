// SPDX-License-Identifier: LGPL-2.1-or-later
//! Cinemática: juntas (graus de liberdade permitidos), comandos (ângulo/curso por valor ou lei
//! no tempo `t`), acoplamentos (engrenagem, pinhão-cremalheira, cabo) e simulação por quadros
//! com velocidade e aceleração.
//!
//! Cada junta liga um sistema local (origem, eixo z, referência x) de cada componente. Os
//! resíduos entram no mesmo solucionador das restrições.

use crate::*;
use std::collections::HashMap;

/// Sistema local de uma junta no componente: origem, eixo (z) e direção de referência (x ⟂ z).
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct JFrame {
    pub o: V3,
    pub z: V3,
    pub x: V3,
}

impl JFrame {
    /// Sistema a partir de um ponto e um eixo (x escolhido perpendicular, estável).
    pub fn from_axis(o: V3, z: V3) -> JFrame {
        let z = unit(z);
        let h = if z[2].abs() < 0.9 { [0., 0., 1.] } else { [1., 0., 0.] };
        JFrame { o, z, x: unit(cross(h, z)) }
    }
    /// Origem, eixo z e eixo x no mundo.
    pub fn world(&self, p: &Placement) -> (V3, V3, V3) {
        (p.point(self.o), unit(p.dir(self.z)), unit(p.dir(self.x)))
    }
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct JRef {
    pub comp: Id,
    pub frame: JFrame,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum JointKind {
    /// rotação em torno do eixo (retenção axial)
    Revolute,
    /// translação ao longo do eixo (sem girar)
    Prismatic,
    /// rotação e translação no mesmo eixo
    Cylindrical,
    /// rotação e avanço ligados pelo passo (mm por volta)
    Screw { pitch: String },
    /// três rotações livres, sem translação
    Spherical,
    /// duas translações e uma rotação sobre o plano (z = normal)
    Planar,
    /// sem graus de liberdade
    Rigid,
    /// o ponto (origem de B) segue a curva (polilinha no sistema de A)
    PointCurve { curve: Vec<V3> },
    /// came: o seguidor (origem de B, com rolete de raio `radius`) fica encostado no perfil
    /// (polilinha fechada no sistema de A) — com uma prismática no seguidor e uma revolução no
    /// came, vira o mecanismo came-seguidor
    Cam { profile: Vec<V3>, radius: String },
}

impl JointKind {
    pub fn label(&self) -> &'static str {
        match self {
            JointKind::Revolute => "Revolução",
            JointKind::Prismatic => "Prismática",
            JointKind::Cylindrical => "Cilíndrica",
            JointKind::Screw { .. } => "Parafuso",
            JointKind::Spherical => "Esférica",
            JointKind::Planar => "Planar",
            JointKind::Rigid => "Rígida",
            JointKind::PointCurve { .. } => "Ponto em curva",
            JointKind::Cam { .. } => "Came",
        }
    }
    /// Pode receber comando de ângulo / de curso.
    pub fn drives(&self) -> (bool, bool) {
        match self {
            JointKind::Revolute | JointKind::Screw { .. } => (true, false),
            JointKind::Prismatic => (false, true),
            JointKind::Cylindrical => (true, true),
            _ => (false, false),
        }
    }
    /// Graus de liberdade que a junta deixa e que têm medida. O came não tem nenhum próprio: o
    /// movimento dele vem das outras juntas do mecanismo.
    pub fn graus(&self) -> &'static [Grau] {
        match self {
            JointKind::Revolute | JointKind::Screw { .. } => &[Grau::Giro],
            JointKind::Prismatic => &[Grau::Curso],
            JointKind::Cylindrical => &[Grau::Giro, Grau::Curso],
            JointKind::Spherical => &[Grau::Inclinacao, Grau::Giro],
            JointKind::Planar => &[Grau::CursoX, Grau::CursoY, Grau::Giro],
            JointKind::PointCurve { .. } => &[Grau::Trajeto],
            JointKind::Rigid | JointKind::Cam { .. } => &[],
        }
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Joint {
    pub id: Id,
    pub name: String,
    pub kind: JointKind,
    pub a: JRef,
    pub b: JRef,
    /// ângulo (graus) e curso (mm) medidos na criação: os comandos são relativos a eles
    pub angle0: f64,
    pub len0: f64,
    /// comando de ângulo (graus, expressão; pode usar `t`)
    #[serde(default)]
    pub drive_angle: Option<String>,
    /// comando de curso (mm, expressão; pode usar `t`)
    #[serde(default)]
    pub drive_len: Option<String>,
    #[serde(default)]
    pub suppressed: bool,
    /// faixa de movimento de cada grau de liberdade: o solucionador a impõe como desigualdade
    #[serde(default)]
    pub limites: Limites,
    /// posição no plano (x, y) e na trajetória, medidas na criação: o zero dos limites desses graus
    #[serde(default)]
    pub zero: [f64; 3],
    /// de onde cada lado veio: quando a peça muda, o sistema da junta anda com o elemento (B99)
    #[serde(default)]
    pub fontes: [Option<MateRef>; 2],
    /// giro acumulado em graus, com as voltas (ver [`giro_continuo`])
    #[serde(default)]
    pub giro: f64,
}

/// Um extremo da faixa. Desligar não apaga o valor: ligar de novo volta ao que estava.
#[derive(Clone, Copy, PartialEq, Debug, Default, Serialize, Deserialize)]
pub struct Limite {
    pub ligado: bool,
    pub valor: f64,
}

/// Mínimo e máximo de um grau de liberdade, cada um com a sua caixa.
#[derive(Clone, Copy, PartialEq, Debug, Default, Serialize, Deserialize)]
pub struct Faixa {
    pub min: Limite,
    pub max: Limite,
}

impl Faixa {
    /// Faixa com os dois extremos ligados.
    pub fn nova(min: f64, max: f64) -> Self {
        Faixa { min: Limite { ligado: true, valor: min }, max: Limite { ligado: true, valor: max } }
    }

    pub fn ligada(&self) -> bool {
        self.min.ligado || self.max.ligado
    }

    fn lo(&self) -> f64 {
        if self.min.ligado {
            self.min.valor
        } else {
            f64::NEG_INFINITY
        }
    }

    fn hi(&self) -> f64 {
        if self.max.ligado {
            self.max.valor
        } else {
            f64::INFINITY
        }
    }

    /// `v` trazido para dentro da faixa. O `clamp` da biblioteca entra em pânico com mínimo acima do
    /// máximo, e a pessoa pode digitar isso.
    pub fn prender(&self, v: f64) -> f64 {
        v.max(self.lo()).min(self.hi())
    }

    /// Extremos ligados, e o padrão no lugar dos desligados.
    pub fn ou(&self, padrao: (f64, f64)) -> (f64, f64) {
        (if self.min.ligado { self.min.valor } else { padrao.0 }, if self.max.ligado { self.max.valor } else { padrao.1 })
    }

    /// Quanto `v` passou da faixa, com sinal: zero dentro dela. No grau cíclico, com os dois extremos
    /// ligados, vale a volta mais curta: 270° e −90° são a mesma posição, e a medida só vai a ±180°.
    /// Com um extremo só a faixa não fecha no círculo, e a medida conta como está.
    pub fn desvio(&self, v: f64, ciclico: bool) -> f64 {
        let (lo, hi) = (self.lo(), self.hi());
        let fora = |x: f64| {
            if x < lo {
                x - lo
            } else if x > hi {
                x - hi
            } else {
                0.
            }
        };
        if !(ciclico && self.min.ligado && self.max.ligado) {
            return fora(v);
        }
        [v, v - 360., v + 360.].map(fora).into_iter().min_by(|a, b| a.abs().total_cmp(&b.abs())).unwrap_or(0.)
    }

    /// `v` está encostado num extremo ligado?
    pub fn encostado(&self, v: f64, tol: f64) -> bool {
        (self.min.ligado && (v - self.min.valor).abs() <= tol) || (self.max.ligado && (v - self.max.valor).abs() <= tol)
    }
}

/// Faixa como os arquivos anteriores gravavam: `null` ou `[mínimo, máximo]`.
fn faixa_lida<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Faixa, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Lida {
        Atual(Faixa),
        Antiga(Option<(f64, f64)>),
    }
    Ok(match Lida::deserialize(d)? {
        Lida::Atual(f) => f,
        Lida::Antiga(Some((lo, hi))) => Faixa::nova(lo, hi),
        Lida::Antiga(None) => Faixa::default(),
    })
}

/// Grau de liberdade de uma junta, do jeito que se mede e se limita.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Grau {
    /// giro em volta do eixo z da junta
    Giro,
    /// avanço ao longo do eixo z
    Curso,
    /// posição no plano da junta, na direção x
    CursoX,
    /// posição no plano da junta, na direção y
    CursoY,
    /// ângulo entre os dois eixos z: um cone que limita os dois tombamentos da esférica
    Inclinacao,
    /// comprimento andado sobre a trajetória
    Trajeto,
}

impl Grau {
    pub fn label(self) -> &'static str {
        match self {
            Grau::Giro => "Ângulo",
            Grau::Curso => "Curso",
            Grau::CursoX => "Posição X",
            Grau::CursoY => "Posição Y",
            Grau::Inclinacao => "Inclinação",
            Grau::Trajeto => "Trajeto",
        }
    }
    /// Grau medido em graus.
    pub fn angular(self) -> bool {
        matches!(self, Grau::Giro | Grau::Inclinacao)
    }
    pub fn unidade(self) -> &'static str {
        if self.angular() {
            "°"
        } else {
            " mm"
        }
    }
    /// Faixa que o limite ganha ao ser ligado pela primeira vez.
    pub fn padrao(self) -> (f64, f64) {
        match self {
            Grau::Giro => (-90., 90.),
            Grau::Inclinacao => (0., 45.),
            Grau::Trajeto => (0., 100.),
            _ => (-50., 50.),
        }
    }
}

/// Mínimo e máximo de cada grau de liberdade da junta (graus e mm, relativos à criação).
#[derive(Clone, Copy, PartialEq, Debug, Default, Serialize, Deserialize)]
pub struct Limites {
    #[serde(default, deserialize_with = "faixa_lida")]
    pub angulo: Faixa,
    #[serde(default, deserialize_with = "faixa_lida")]
    pub curso: Faixa,
    #[serde(default)]
    pub curso_x: Faixa,
    #[serde(default)]
    pub curso_y: Faixa,
    #[serde(default)]
    pub inclinacao: Faixa,
    #[serde(default)]
    pub trajeto: Faixa,
}

impl Limites {
    pub fn faixa(&self, g: Grau) -> &Faixa {
        match g {
            Grau::Giro => &self.angulo,
            Grau::Curso => &self.curso,
            Grau::CursoX => &self.curso_x,
            Grau::CursoY => &self.curso_y,
            Grau::Inclinacao => &self.inclinacao,
            Grau::Trajeto => &self.trajeto,
        }
    }
    pub fn faixa_mut(&mut self, g: Grau) -> &mut Faixa {
        match g {
            Grau::Giro => &mut self.angulo,
            Grau::Curso => &mut self.curso,
            Grau::CursoX => &mut self.curso_x,
            Grau::CursoY => &mut self.curso_y,
            Grau::Inclinacao => &mut self.inclinacao,
            Grau::Trajeto => &mut self.trajeto,
        }
    }
}

/// Relação entre duas juntas: engrenagem (ângulo × ângulo), pinhão-cremalheira (ângulo → curso)
/// ou cabo (curso × curso).
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum CouplingKind {
    Gear,
    RackPinion,
    Cable,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Coupling {
    pub id: Id,
    pub name: String,
    pub kind: CouplingKind,
    pub a: Id,
    pub b: Id,
    /// razão (engrenagem: −z1/z2; pinhão: mm por volta; cabo: curso b / curso a)
    pub ratio: String,
}

fn wrap180(a: f64) -> f64 {
    let mut a = a % 360.;
    if a > 180. {
        a -= 360.;
    }
    if a <= -180. {
        a += 360.;
    }
    a
}

/// Ângulo de B em relação a A em torno do eixo de A (graus).
fn angle_of(j: &Joint, pa: &Placement, pb: &Placement) -> f64 {
    let (_, za, xa) = j.a.frame.world(pa);
    let (_, _, xb) = j.b.frame.world(pb);
    dot(cross(xa, xb), za).atan2(dot(xa, xb)).to_degrees()
}

/// Põe `v`, medido em ±180°, na volta mais perto de `referencia`.
pub(crate) fn desenrolar(referencia: f64, v: f64) -> f64 {
    referencia + wrap180(v - referencia)
}

/// Até onde um acoplamento encadeado é seguido: um ciclo de acoplamentos não pode prender a conta.
const ELOS_MAX: u32 = 16;

/// De que volta partir para ler o giro da junta. Um comando diz a volta sozinho, e o movido por
/// acoplamento herda a de quem o aciona. O giro guardado fica para quem anda à mão.
fn referencia_giro(a: &Assembly, j: &Joint, pl: &HashMap<Id, Placement>, vars: &HashMap<String, f64>, drives: bool, elos: u32) -> f64 {
    let num = |e: &String| expr::eval_str(e, vars).unwrap_or(0.);
    if drives {
        if let Some(e) = &j.drive_angle {
            return j.limites.angulo.prender(num(e));
        }
        if let (JointKind::Screw { pitch }, Some(e)) = (&j.kind, &j.drive_len) {
            let passo = num(pitch);
            if passo != 0. {
                return 360. * j.limites.curso.prender(num(e)) / passo;
            }
        }
    }
    if elos < ELOS_MAX {
        let motora = a.couplings.iter().filter(|c| c.b == j.id && matches!(c.kind, CouplingKind::Gear)).find_map(|c| {
            let ja = a.joints.iter().find(|x| x.id == c.a)?;
            Some(num(&c.ratio) * giro_continuo(a, ja, pl, vars, drives, elos + 1)?)
        });
        if let Some(g) = motora {
            return g;
        }
    }
    j.giro
}

/// Ângulo da junta com as voltas, relativo à criação. A medida vem de um `atan2` e só existe em
/// ±180°: isso basta para comparar direções, mas uma razão precisa do giro de verdade. Com 11:82, o
/// pinhão em 225° é lido como −135°, e a coroa teria de pular de −24° para +18°.
pub(crate) fn giro_continuo(a: &Assembly, j: &Joint, pl: &HashMap<Id, Placement>, vars: &HashMap<String, f64>, drives: bool, elos: u32) -> Option<f64> {
    let (medido, _) = measure(j, pl)?;
    Some(desenrolar(referencia_giro(a, j, pl, vars, drives, elos), medido))
}

/// Guarda as voltas de cada junta na posição resolvida: a próxima solução parte daqui, mesmo sem
/// comando, e a volta vai junto no arquivo.
pub(crate) fn guardar_giros(a: &mut Assembly, pl: &HashMap<Id, Placement>, vars: &HashMap<String, f64>) {
    let novos: Vec<f64> = a.joints.iter().map(|j| giro_continuo(a, j, pl, vars, true, 0).unwrap_or(j.giro)).collect();
    for (j, g) in a.joints.iter_mut().zip(novos) {
        j.giro = g;
    }
}

/// Curso de B em relação a A ao longo do eixo de A (mm).
fn len_of(j: &Joint, pa: &Placement, pb: &Placement) -> f64 {
    let (oa, za, _) = j.a.frame.world(pa);
    let (ob, _, _) = j.b.frame.world(pb);
    dot(sub(ob, oa), za)
}

/// Comprimento da polilinha do começo até o ponto dela mais perto de `q`.
fn arco_ate(curva: &[V3], q: V3) -> f64 {
    let (mut melhor, mut andado, mut achado) = (f64::MAX, 0., 0.);
    for w in curva.windows(2) {
        let e = sub(w[1], w[0]);
        let l = norm(e);
        let t = (dot(sub(q, w[0]), e) / (l * l).max(1e-18)).clamp(0., 1.);
        let d = norm(sub(q, [0, 1, 2].map(|i| w[0][i] + e[i] * t)));
        if d < melhor {
            melhor = d;
            achado = andado + l * t;
        }
        andado += l;
    }
    achado
}

/// Comprimento total da polilinha.
pub fn comprimento_curva(curva: &[V3]) -> f64 {
    curva.windows(2).map(|w| norm(sub(w[1], w[0]))).sum()
}

/// Ponto da polilinha a `s` mm do começo. Fora dela, a ponta mais perto: a junta não sai da curva.
pub fn ponto_no_arco(curva: &[V3], s: f64) -> Option<V3> {
    let (primeiro, ultimo) = (*curva.first()?, *curva.last()?);
    if s <= 0. {
        return Some(primeiro);
    }
    let mut andado = 0.;
    for w in curva.windows(2) {
        let e = sub(w[1], w[0]);
        let l = norm(e);
        if andado + l >= s && l > 0. {
            let t = (s - andado) / l;
            return Some([0, 1, 2].map(|i| w[0][i] + e[i] * t));
        }
        andado += l;
    }
    Some(ultimo)
}

/// Posição no plano da junta e na trajetória, sem descontar o zero da criação.
fn posicao_crua(j: &Joint, pa: &Placement, pb: &Placement) -> [f64; 3] {
    let (oa, za, xa) = j.a.frame.world(pa);
    let (ob, _, _) = j.b.frame.world(pb);
    let d = sub(ob, oa);
    let s = if let JointKind::PointCurve { curve } = &j.kind { arco_ate(curve, pa.to_local(ob)) } else { 0. };
    [dot(d, xa), dot(d, cross(za, xa)), s]
}

/// Valor de um grau de liberdade agora, na mesma referência dos limites (graus ou mm).
pub fn medir_grau(j: &Joint, g: Grau, pa: &Placement, pb: &Placement) -> f64 {
    let cru = posicao_crua(j, pa, pb);
    match g {
        Grau::Giro => wrap180(angle_of(j, pa, pb) - j.angle0),
        Grau::Curso => len_of(j, pa, pb) - j.len0,
        Grau::CursoX => cru[0] - j.zero[0],
        Grau::CursoY => cru[1] - j.zero[1],
        Grau::Trajeto => cru[2] - j.zero[2],
        Grau::Inclinacao => {
            let (za, zb) = (j.a.frame.world(pa).1, j.b.frame.world(pb).1);
            norm(cross(za, zb)).atan2(dot(za, zb)).to_degrees()
        }
    }
}

/// Encostou no limite: abaixo disto (graus ou mm) a junta conta como parada no batente.
const TOL_BATENTE: f64 = 1e-4;

/// A junta está parada em algum limite ligado? Não é falha: é o batente fazendo o papel dele.
pub fn no_batente(j: &Joint, pl: &HashMap<Id, Placement>) -> bool {
    let (Some(pa), Some(pb)) = (pl.get(&j.a.comp), pl.get(&j.b.comp)) else { return false };
    !j.suppressed && j.kind.graus().iter().any(|g| j.limites.faixa(*g).encostado(medir_grau(j, *g, pa, pb), TOL_BATENTE))
}

/// Resíduos de desigualdade: um por grau limitado, zero dentro da faixa e igual ao que passou dela
/// fora. O quadrado disso tem derivada contínua na borda, então o mínimos quadrados para ali sem
/// tratamento especial. Ficam fora de [`residuals`] porque um batente não tira grau de liberdade:
/// quem conta GDL não pode ver essas linhas, nem quando a junta está encostada.
pub fn residuos_limite(a: &Assembly, pl: &HashMap<Id, Placement>, r: &mut Vec<f64>, per: &mut Vec<(Id, usize, usize)>, juntas: &[usize]) {
    for j in juntas.iter().map(|i| &a.joints[*i]).filter(|j| !j.suppressed) {
        let (Some(pa), Some(pb)) = (pl.get(&j.a.comp), pl.get(&j.b.comp)) else { continue };
        let s = r.len();
        for g in j.kind.graus() {
            let f = j.limites.faixa(*g);
            if f.ligada() {
                let d = f.desvio(medir_grau(j, *g, pa, pb), *g == Grau::Giro);
                // mesma escala do comando de ângulo: 1° pesa como 0,17 mm
                r.push(if g.angular() { d.to_radians() * 10. } else { d });
            }
        }
        if r.len() > s {
            per.push((j.id, s, r.len()));
        }
    }
}

/// Ângulo e curso atuais da junta, relativos à criação (o que os comandos medem).
pub fn measure(j: &Joint, pl: &HashMap<Id, Placement>) -> Option<(f64, f64)> {
    let (pa, pb) = (pl.get(&j.a.comp)?, pl.get(&j.b.comp)?);
    Some((wrap180(angle_of(j, pa, pb) - j.angle0), len_of(j, pa, pb) - j.len0))
}

impl Assembly {
    /// Cria a junta medindo o ângulo e o curso atuais (que viram o zero dos comandos).
    pub fn add_joint(&mut self, kind: JointKind, a: JRef, b: JRef) -> Id {
        let id = self.alloc();
        let name = self.unique_pub(kind.label());
        let pl = |c: Id| self.node(c).map(|n| n.placement).unwrap_or_default();
        let mut j = Joint { id, name, kind, a, b, angle0: 0., len0: 0., drive_angle: None, drive_len: None, suppressed: false, limites: Limites::default(), zero: [0.; 3], fontes: [None, None], giro: 0. };
        j.angle0 = angle_of(&j, &pl(a.comp), &pl(b.comp));
        j.len0 = len_of(&j, &pl(a.comp), &pl(b.comp));
        j.zero = posicao_crua(&j, &pl(a.comp), &pl(b.comp));
        self.joints.push(j);
        id
    }

    pub fn add_coupling(&mut self, kind: CouplingKind, a: Id, b: Id, ratio: &str) -> Id {
        let id = self.alloc();
        let name = self.unique_pub(match kind {
            CouplingKind::Gear => "Engrenagem",
            CouplingKind::RackPinion => "Pinhão-cremalheira",
            CouplingKind::Cable => "Cabo",
        });
        self.couplings.push(Coupling { id, name, kind, a, b, ratio: ratio.into() });
        id
    }

    /// Mecanismo automático: cada par de componentes com restrições vira uma junta equivalente
    /// (eixo + face = revolução; só eixo = cilíndrica; só faces = planar; pontos = esférica).
    /// As restrições convertidas saem da lista. Devolve as juntas criadas.
    pub fn joints_from_mates(&mut self) -> Vec<Id> {
        let mut pairs: Vec<(Id, Id)> = vec![];
        for m in self.mates.iter().filter(|m| !m.suppressed) {
            let k = if m.a.comp.0 < m.b.comp.0 { (m.a.comp, m.b.comp) } else { (m.b.comp, m.a.comp) };
            if !pairs.contains(&k) {
                pairs.push(k);
            }
        }
        let mut made = vec![];
        for (c1, c2) in pairs {
            let ms: Vec<Mate> = self.mates.iter().filter(|m| !m.suppressed && ((m.a.comp == c1 && m.b.comp == c2) || (m.a.comp == c2 && m.b.comp == c1))).cloned().collect();
            let axis = ms.iter().find(|m| matches!(m.a.geom, Geom::Axis { .. }) && matches!(m.b.geom, Geom::Axis { .. }) && matches!(m.kind, MateKind::Coincident { .. }));
            let plane = ms.iter().find(|m| matches!(m.a.geom, Geom::Plane { .. }) && matches!(m.b.geom, Geom::Plane { .. }) && matches!(m.kind, MateKind::Coincident { .. } | MateKind::Contact));
            let point = ms.iter().find(|m| matches!(m.a.geom, Geom::Point(_)) && matches!(m.b.geom, Geom::Point(_)));
            let fr = |g: &Geom| match *g {
                Geom::Axis { p, d } => JFrame::from_axis(p, d),
                Geom::Plane { p, n } => JFrame::from_axis(p, n),
                Geom::Point(p) => JFrame::from_axis(p, [0., 0., 1.]),
            };
            let (kind, src) = match (axis, plane, point) {
                (Some(ax), Some(_), _) => (JointKind::Revolute, ax),
                (Some(ax), None, _) => (JointKind::Cylindrical, ax),
                (None, Some(pl), _) => (JointKind::Planar, pl),
                (None, None, Some(pt)) => (JointKind::Spherical, pt),
                _ => continue,
            };
            let src = src.clone();
            let id = self.add_joint(kind, JRef { comp: src.a.comp, frame: fr(&src.a.geom) }, JRef { comp: src.b.comp, frame: fr(&src.b.geom) });
            // o x de B começa alinhado ao de A (mesmo zero de ângulo)
            let ids: Vec<Id> = ms.iter().map(|m| m.id).collect();
            self.mates.retain(|m| !ids.contains(&m.id));
            made.push(id);
        }
        made
    }
}

/// Leva a junta ao valor `v` do grau `g` pelo solucionador: quem está preso nela vem junto, e o que
/// as restrições e os limites não deixam, fica. Sem comando gravado: depois disso a junta segue livre.
pub fn levar_a(a: &mut Assembly, id: Id, g: Grau, v: f64) -> crate::solve::SolveReport {
    let Some(i) = a.joints.iter().position(|j| j.id == id) else { return Default::default() };
    let (ang, len) = (a.joints[i].drive_angle.clone(), a.joints[i].drive_len.clone());
    match g {
        Grau::Giro => a.joints[i].drive_angle = Some(valor_cota(v)),
        Grau::Curso => a.joints[i].drive_len = Some(valor_cota(v)),
        Grau::CursoX | Grau::CursoY => {
            let j = a.joints[i].clone();
            let (Some(pa), Some(pb)) = (a.lugar(j.a.comp), a.lugar(j.b.comp)) else { return Default::default() };
            let (_, za, xa) = j.a.frame.world(&pa);
            let d = if g == Grau::CursoX { xa } else { cross(za, xa) };
            let falta = v - medir_grau(&j, g, &pa, &pb);
            let peca_b_presa = a.node(j.b.comp).is_none_or(|n| n.fixed);
            let (comp, lugar, k) = if peca_b_presa { (j.a.comp, pa, -falta) } else { (j.b.comp, pb, falta) };
            return crate::solve::solve_toward(a, comp, lugar.moved(d.map(|e| e * k), [0.; 3], lugar.origin), 0.05);
        }
        Grau::Trajeto => {
            let j = a.joints[i].clone();
            let JointKind::PointCurve { curve } = &j.kind else { return crate::solve::solve(a) };
            let (Some(pa), Some(pb)) = (a.lugar(j.a.comp), a.lugar(j.b.comp)) else { return Default::default() };
            let Some(alvo) = ponto_no_arco(curve, v + j.zero[2]) else { return crate::solve::solve(a) };
            // a trajetória é a da peça A: quem anda é o seguidor, ou a própria curva se ele estiver preso
            let falta = sub(pa.point(alvo), pb.point(j.b.frame.o));
            let seguidor_preso = a.node(j.b.comp).is_none_or(|n| n.fixed);
            let (comp, lugar, k) = if seguidor_preso { (j.a.comp, pa, -1.) } else { (j.b.comp, pb, 1.) };
            return crate::solve::solve_toward(a, comp, lugar.moved(falta.map(|e| e * k), [0.; 3], lugar.origin), 0.05);
        }
        Grau::Inclinacao => return crate::solve::solve(a),
    }
    let rep = crate::solve::solve(a);
    a.joints[i].drive_angle = ang;
    a.joints[i].drive_len = len;
    rep
}

/// O grau aceita valor digitado? A inclinação não tem um caminho único até o valor, e o comando
/// gravado na junta já manda no ângulo e no curso.
pub fn aceita_levar(j: &Joint, g: Grau) -> bool {
    match g {
        Grau::Giro => j.drive_angle.is_none(),
        Grau::Curso => j.drive_len.is_none(),
        Grau::CursoX | Grau::CursoY | Grau::Trajeto => true,
        Grau::Inclinacao => false,
    }
}

/// Resíduos das juntas e acoplamentos (entram no solucionador). `drives = false` ignora os
/// comandos (para contar os graus de liberdade do mecanismo).
pub(crate) fn residuals(a: &Assembly, pl: &HashMap<Id, Placement>, vars: &HashMap<String, f64>, drives: bool, r: &mut Vec<f64>, per: &mut Vec<(Id, usize, usize)>, escopo: &crate::solve::Escopo) {
    let num = |e: &String| expr::eval_str(e, vars).unwrap_or(0.);
    for j in escopo.juntas.iter().map(|i| &a.joints[*i]).filter(|j| !j.suppressed) {
        let (Some(pa), Some(pb)) = (pl.get(&j.a.comp), pl.get(&j.b.comp)) else { continue };
        let s = r.len();
        let (oa, za, _) = j.a.frame.world(pa);
        let (ob, zb, _) = j.b.frame.world(pb);
        let d = sub(ob, oa);
        let along = dot(d, za);
        let perp = [d[0] - za[0] * along, d[1] - za[1] * along, d[2] - za[2] * along];
        let ang = || wrap180(angle_of(j, pa, pb) - j.angle0);
        match &j.kind {
            JointKind::Revolute => {
                r.extend(cross(za, zb));
                r.extend(perp);
                r.push(along - j.len0);
            }
            JointKind::Prismatic => {
                r.extend(cross(za, zb));
                r.extend(perp);
                r.push(ang());
            }
            JointKind::Cylindrical => {
                r.extend(cross(za, zb));
                r.extend(perp);
            }
            JointKind::Screw { pitch } => {
                r.extend(cross(za, zb));
                r.extend(perp);
                // avanço = passo × voltas: a rosca dá muitas voltas, e a medida só vê meia
                let giro = giro_continuo(a, j, pl, vars, drives, 0).unwrap_or_else(ang);
                r.push(along - j.len0 - num(pitch) * giro / 360.);
            }
            JointKind::Spherical => r.extend(d),
            JointKind::Planar => {
                r.extend(cross(za, zb));
                r.push(along);
            }
            JointKind::Rigid => {
                r.extend(cross(za, zb));
                r.push(ang());
                r.extend(d);
            }
            JointKind::PointCurve { curve } => {
                // distância do ponto à curva (no sistema de A)
                let q = pa.to_local(ob);
                let mut best = (f64::MAX, [0.; 3]);
                for w in curve.windows(2) {
                    let (p0, p1) = (w[0], w[1]);
                    let e = sub(p1, p0);
                    let t = (dot(sub(q, p0), e) / dot(e, e).max(1e-18)).clamp(0., 1.);
                    let c = [p0[0] + e[0] * t, p0[1] + e[1] * t, p0[2] + e[2] * t];
                    let dd = norm(sub(q, c));
                    if dd < best.0 {
                        best = (dd, c);
                    }
                }
                r.extend(pa.dir(sub(q, best.1)));
            }
            JointKind::Cam { profile, radius } => {
                // contato: distância do centro do rolete ao perfil fechado = raio do rolete
                let q = pa.to_local(ob);
                let n = profile.len();
                let mut best = f64::MAX;
                for i in 0..n {
                    let (p0, p1) = (profile[i], profile[(i + 1) % n]);
                    let e = sub(p1, p0);
                    let t = (dot(sub(q, p0), e) / dot(e, e).max(1e-18)).clamp(0., 1.);
                    best = best.min(norm(sub(q, [p0[0] + e[0] * t, p0[1] + e[1] * t, p0[2] + e[2] * t])));
                }
                r.push(best - num(radius));
            }
        }
        if drives {
            if let Some(e) = &j.drive_angle {
                // comando além do limite para no batente, em vez de brigar com ele
                r.push(wrap180(ang() - j.limites.angulo.prender(num(e))) * std::f64::consts::PI / 180. * 10.);
            }
            if let Some(e) = &j.drive_len {
                r.push(along - j.len0 - j.limites.curso.prender(num(e)));
            }
        }
        per.push((j.id, s, r.len()));
    }
    for c in escopo.acoplamentos.iter().map(|i| &a.couplings[*i]) {
        let (Some(ja), Some(jb)) = (a.joints.iter().find(|j| j.id == c.a), a.joints.iter().find(|j| j.id == c.b)) else { continue };
        let (Some((_, la)), Some((ab, lb))) = (measure(ja, pl), measure(jb, pl)) else { continue };
        let Some(aa) = giro_continuo(a, ja, pl, vars, drives, 0) else { continue };
        let k = num(&c.ratio);
        let s = r.len();
        r.push(match c.kind {
            CouplingKind::Gear => wrap180(ab - k * aa) / 10.,
            CouplingKind::RackPinion => lb - k * aa / 360.,
            CouplingKind::Cable => lb - k * la,
        });
        per.push((c.id, s, r.len()));
    }
}

/// Um quadro da simulação: tempo e posições dos componentes.
#[derive(Clone, Debug)]
pub struct Frame {
    pub t: f64,
    pub placements: HashMap<Id, Placement>,
    pub ok: bool,
}

/// Simula de `t0` a `t1` em `steps` passos (os comandos podem usar `t`). A montagem volta ao
/// estado inicial no fim.
pub fn simulate(a: &mut Assembly, t0: f64, t1: f64, steps: usize) -> Vec<Frame> {
    let keep = a.clone();
    let mut out = vec![];
    for k in 0..=steps.max(1) {
        let t = t0 + (t1 - t0) * k as f64 / steps.max(1) as f64;
        let mut vars = HashMap::new();
        vars.insert("t".to_string(), t);
        let rep = crate::solve::solve_with(a, &vars);
        out.push(Frame { t, placements: a.nodes.iter().map(|n| (n.id, n.placement)).collect(), ok: rep.converged });
    }
    *a = keep;
    out
}

/// Velocidade e aceleração (diferenças finitas) de um ponto local do componente ao longo dos quadros.
pub fn kinematics_of(frames: &[Frame], comp: Id, local: V3) -> Vec<(f64, V3, f64, f64)> {
    let pos: Vec<(f64, V3)> = frames.iter().filter_map(|f| f.placements.get(&comp).map(|p| (f.t, p.point(local)))).collect();
    let mut out = vec![];
    for i in 0..pos.len() {
        let v = |i: usize, j: usize| {
            let dt = (pos[j].0 - pos[i].0).max(1e-12);
            let d = sub(pos[j].1, pos[i].1);
            [d[0] / dt, d[1] / dt, d[2] / dt]
        };
        let (i0, i1) = (i.saturating_sub(1), (i + 1).min(pos.len() - 1));
        let vel = if i1 > i0 { v(i0, i1) } else { [0.; 3] };
        let acc = if i > 0 && i + 1 < pos.len() {
            let (va, vb) = (v(i - 1, i), v(i, i + 1));
            let dt = ((pos[i + 1].0 - pos[i - 1].0) / 2.).max(1e-12);
            norm(sub(vb, va)) / dt
        } else {
            0.
        };
        out.push((pos[i].0, pos[i].1, norm(vel), acc));
    }
    out
}

/// Análise do mecanismo: graus de liberdade sem os comandos e com eles.
pub fn analyze(a: &Assembly) -> (usize, usize) {
    (crate::solve::dof_with(a, false), crate::solve::dof_with(a, true))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solve::solve_with;

    fn manivela() -> (Assembly, Id, Id) {
        let mut a = Assembly::new("manivela");
        let base = a.add_component("Base", NodeKind::Part { path: "b".into() }, Placement::default(), None);
        let braco = a.add_component("Braço", NodeKind::Part { path: "c".into() }, Placement::default(), None);
        let f = JFrame::from_axis([0.; 3], [0., 0., 1.]);
        let j = a.add_joint(JointKind::Revolute, JRef { comp: base, frame: f }, JRef { comp: braco, frame: f });
        (a, braco, j)
    }

    /// Limita o giro da manivela.
    fn com_limite(a: &mut Assembly, j: Id, lo: f64, hi: f64) {
        a.joints.iter_mut().find(|x| x.id == j).unwrap().limites.angulo = Faixa::nova(lo, hi);
    }

    fn giro(a: &Assembly, j: Id) -> f64 {
        let pl: HashMap<Id, Placement> = a.nodes.iter().map(|n| (n.id, n.placement)).collect();
        measure(a.joints.iter().find(|x| x.id == j).unwrap(), &pl).unwrap().0
    }

    #[test]
    fn comando_alem_do_limite_para_no_batente_sem_falhar() {
        let (mut a, _, j) = manivela();
        com_limite(&mut a, j, -90., 90.);
        a.joints.iter_mut().find(|x| x.id == j).unwrap().drive_angle = Some("120".into());
        let rep = solve_with(&mut a, &HashMap::new());
        assert!(rep.converged, "{rep:?}");
        assert!((giro(&a, j) - 90.).abs() < 1e-6, "{}", giro(&a, j));
        assert_eq!(rep.no_limite, vec![j], "fica marcada como parada no limite");
    }

    #[test]
    fn arrastar_alem_do_limite_para_na_marca() {
        let (mut a, braco, j) = manivela();
        com_limite(&mut a, j, -30., 45.);
        let inicio = a.node(braco).unwrap().placement;
        // puxa a peça em passos, como o arraste faz, até 80° dando a volta pelo eixo
        for g in (5..=80).step_by(5) {
            let alvo = inicio.moved([0.; 3], [0., 0., (g as f64).to_radians()], [0.; 3]);
            let rep = crate::solve::solve_toward(&mut a, braco, alvo, 0.05);
            assert!(rep.converged, "{g}: {rep:?}");
        }
        assert!((giro(&a, j) - 45.).abs() < 1e-5, "{}", giro(&a, j));
    }

    #[test]
    fn peca_fora_da_faixa_volta_para_o_limite_mais_perto() {
        let (mut a, _, j) = manivela();
        a.joints.iter_mut().find(|x| x.id == j).unwrap().drive_angle = Some("60".into());
        solve_with(&mut a, &HashMap::new());
        a.joints.iter_mut().find(|x| x.id == j).unwrap().drive_angle = None;
        com_limite(&mut a, j, -10., 20.);
        assert!(solve_with(&mut a, &HashMap::new()).converged);
        assert!((giro(&a, j) - 20.).abs() < 1e-6, "{}", giro(&a, j));
    }

    #[test]
    fn caixa_desligada_nao_limita_e_nao_perde_o_valor() {
        let (mut a, _, j) = manivela();
        com_limite(&mut a, j, -10., 20.);
        a.joints.iter_mut().find(|x| x.id == j).unwrap().limites.angulo.max.ligado = false;
        a.joints.iter_mut().find(|x| x.id == j).unwrap().drive_angle = Some("60".into());
        let rep = solve_with(&mut a, &HashMap::new());
        assert!(rep.no_limite.is_empty());
        assert!((giro(&a, j) - 60.).abs() < 1e-6);
        a.joints.iter_mut().find(|x| x.id == j).unwrap().limites.angulo.max.ligado = true;
        solve_with(&mut a, &HashMap::new());
        assert!((giro(&a, j) - 20.).abs() < 1e-6, "o valor guardado volta a valer");
    }

    #[test]
    fn limite_nao_tira_grau_de_liberdade_nem_encostado() {
        let (mut a, _, j) = manivela();
        com_limite(&mut a, j, -90., 0.);
        let rep = solve_with(&mut a, &HashMap::new());
        assert_eq!(rep.no_limite, vec![j]);
        assert_eq!(analyze(&a).0, 1);
        assert_eq!(rep.dof, 1, "o batente não desconta, nem com a junta encostada nele");
    }

    #[test]
    fn restricao_que_pede_alem_do_limite_e_conflito_de_verdade() {
        let (mut a, braco, j) = manivela();
        a.node_mut(a.nodes[0].id).unwrap().fixed = true;
        com_limite(&mut a, j, -30., 30.);
        // o braço precisa ficar a 90°: o eixo x dele contra o x do mundo
        let n = |comp, v: V3| MateRef { comp, geom: Geom::Plane { p: [0.; 3], n: v }, pick: None, viva: None };
        let m = a.add_mate(MateKind::Angle("90".into()), n(a.nodes[0].id, [1., 0., 0.]), n(braco, [1., 0., 0.]));
        let rep = solve_with(&mut a, &HashMap::new());
        assert!(!rep.converged, "{rep:?}");
        assert!(rep.failed.contains(&j) && rep.failed.contains(&m), "{rep:?}");
    }

    #[test]
    fn prismatica_limita_o_curso() {
        let mut a = Assembly::new("trilho");
        let b = a.add_component("Base", NodeKind::Part { path: "b".into() }, Placement::default(), None);
        a.node_mut(b).unwrap().fixed = true;
        let c = a.add_component("Carro", NodeKind::Part { path: "c".into() }, Placement::default(), None);
        let f = JFrame::from_axis([0.; 3], [1., 0., 0.]);
        let j = a.add_joint(JointKind::Prismatic, JRef { comp: b, frame: f }, JRef { comp: c, frame: f });
        a.joints[0].limites.curso = Faixa::nova(0., 25.);
        a.joints[0].drive_len = Some("40".into());
        let rep = solve_with(&mut a, &HashMap::new());
        assert!(rep.converged && rep.no_limite == vec![j], "{rep:?}");
        assert!((a.node(c).unwrap().placement.origin[0] - 25.).abs() < 1e-6);
    }

    #[test]
    fn planar_limita_cada_direcao() {
        let mut a = Assembly::new("mesa");
        let b = a.add_component("Base", NodeKind::Part { path: "b".into() }, Placement::default(), None);
        a.node_mut(b).unwrap().fixed = true;
        let c = a.add_component("Mesa", NodeKind::Part { path: "c".into() }, Placement { origin: [5., 0., 0.], ..Default::default() }, None);
        let f = JFrame { o: [0.; 3], z: [0., 0., 1.], x: [1., 0., 0.] };
        let j = a.add_joint(JointKind::Planar, JRef { comp: b, frame: f }, JRef { comp: c, frame: f });
        assert_eq!(a.joints[0].zero[0], 5., "o zero é a posição da criação");
        a.joints[0].limites.curso_x = Faixa::nova(-1., 3.);
        a.joints[0].limites.curso_y = Faixa::nova(-2., 2.);
        levar_a(&mut a, j, Grau::CursoX, 10.);
        levar_a(&mut a, j, Grau::CursoY, -10.);
        let p = a.node(c).unwrap().placement.origin;
        assert!((p[0] - 8.).abs() < 1e-5 && (p[1] + 2.).abs() < 1e-5, "{p:?}");
    }

    #[test]
    fn esferica_limita_o_cone() {
        let mut a = Assembly::new("bola");
        let b = a.add_component("Base", NodeKind::Part { path: "b".into() }, Placement::default(), None);
        a.node_mut(b).unwrap().fixed = true;
        let c = a.add_component("Haste", NodeKind::Part { path: "c".into() }, Placement::default(), None);
        let f = JFrame::from_axis([0.; 3], [0., 0., 1.]);
        a.add_joint(JointKind::Spherical, JRef { comp: b, frame: f }, JRef { comp: c, frame: f });
        a.joints[0].limites.inclinacao.max = Limite { ligado: true, valor: 20. };
        let alvo = Placement::default().moved([0.; 3], [50f64.to_radians(), 0., 0.], [0.; 3]);
        assert!(crate::solve::solve_toward(&mut a, c, alvo, 0.05).converged);
        let z = a.node(c).unwrap().placement.dir([0., 0., 1.]);
        assert!((z[2].acos().to_degrees() - 20.).abs() < 1e-4, "{z:?}");
    }

    #[test]
    fn ponto_em_curva_limita_o_trajeto() {
        let mut a = Assembly::new("trilho curvo");
        let b = a.add_component("Base", NodeKind::Part { path: "b".into() }, Placement::default(), None);
        a.node_mut(b).unwrap().fixed = true;
        let c = a.add_component("Cursor", NodeKind::Part { path: "c".into() }, Placement { origin: [10., 0., 0.], ..Default::default() }, None);
        let f = JFrame::from_axis([0.; 3], [0., 0., 1.]);
        let curva = vec![[0., 0., 0.], [50., 0., 0.], [50., 50., 0.]];
        a.add_joint(JointKind::PointCurve { curve: curva }, JRef { comp: b, frame: f }, JRef { comp: c, frame: f });
        assert_eq!(a.joints[0].zero[2], 10.);
        a.joints[0].limites.trajeto.max = Limite { ligado: true, valor: 15. };
        let alvo = Placement { origin: [40., 0., 0.], ..Default::default() };
        assert!(crate::solve::solve_toward(&mut a, c, alvo, 0.05).converged);
        assert!((a.node(c).unwrap().placement.origin[0] - 25.).abs() < 1e-4, "{:?}", a.node(c).unwrap().placement.origin);
    }

    #[test]
    fn trajeto_leva_ao_valor_para_no_limite_e_marca_anda_na_curva() {
        let mut a = Assembly::new("trilho curvo");
        let b = a.add_component("Base", NodeKind::Part { path: "b".into() }, Placement { origin: [0., 0., 7.], ..Default::default() }, None);
        a.node_mut(b).unwrap().fixed = true;
        let c = a.add_component("Cursor", NodeKind::Part { path: "c".into() }, Placement { origin: [10., 0., 7.], ..Default::default() }, None);
        let f = JFrame::from_axis([0.; 3], [0., 0., 1.]);
        let id = a.add_joint(JointKind::PointCurve { curve: vec![[0., 0., 0.], [50., 0., 0.], [50., 50., 0.]] }, JRef { comp: b, frame: f }, JRef { comp: c, frame: f });
        a.joints[0].limites.trajeto = Faixa::nova(-5., 60.);
        assert!(aceita_levar(&a.joints[0], Grau::Trajeto));
        // dobra a curva: 10 + 55 = 65 mm andados, 15 depois do canto
        assert!(levar_a(&mut a, id, Grau::Trajeto, 55.).converged);
        let o = a.node(c).unwrap().placement.origin;
        assert!(norm(sub(o, [50., 15., 7.])) < 1e-3, "{o:?}");
        // além do limite para no batente, sem falhar
        let rep = levar_a(&mut a, id, Grau::Trajeto, 500.);
        assert!(rep.converged && rep.no_limite.contains(&id), "{rep:?}");
        let o = a.node(c).unwrap().placement.origin;
        assert!(norm(sub(o, [50., 20., 7.])) < 1e-3, "{o:?}");
        // a marca do trajeto é a curva no mundo, com o mesmo zero da junta
        let m = crate::simbolos::marcas_limite(&a, &a.joints[0]);
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].grau, Grau::Trajeto);
        assert!(norm(sub(m[0].ponto(-5., 0.), [5., 0., 7.])) < 1e-9);
        assert!(norm(sub(m[0].ponto(60., 0.), [50., 20., 7.])) < 1e-9);
        assert!((m[0].atual - 60.).abs() < 1e-3);
    }

    #[test]
    fn animar_o_ponto_em_curva_fica_entre_os_limites() {
        use crate::proposta::{animar, Classe, Comando, Elemento, OpcoesAnimar, Tipo};
        let mut a = Assembly::new("trilho curvo");
        let b = a.add_component("Base", NodeKind::Part { path: "b".into() }, Placement::default(), None);
        a.node_mut(b).unwrap().fixed = true;
        let c = a.add_component("Cursor", NodeKind::Part { path: "c".into() }, Placement { origin: [10., 0., 0.], ..Default::default() }, None);
        let curva = vec![[0., 0., 0.], [50., 0., 0.], [50., 50., 0.]];
        let mut trilho = Elemento::novo(MateRef { comp: b, geom: Geom::Axis { p: [0.; 3], d: [1., 0., 0.] }, pick: None, viva: None }, Classe::ArestaCurva { circular: false });
        trilho.curva = curva;
        let mut cmd = Comando::novo(Tipo::Junta(JointKind::PointCurve { curve: vec![] }));
        cmd.receber(trilho);
        cmd.receber(Elemento::novo(MateRef { comp: c, geom: Geom::Point([0.; 3]), pick: None, viva: None }, Classe::Vertice));
        cmd.limites.trajeto = Faixa::nova(-4., 20.);
        let previa = cmd.resolver(&a).unwrap();
        let quadros = animar(&a, &previa, &OpcoesAnimar::default()).unwrap();
        let xs: Vec<f64> = quadros.iter().flat_map(|q| q.iter().filter(|m| m.comp == c).map(|m| m.depois.origin[0])).collect();
        let (lo, hi) = xs.iter().fold((f64::MAX, f64::MIN), |(l, h), x| (l.min(*x), h.max(*x)));
        assert!((lo - 6.).abs() < 0.2 && (hi - 30.).abs() < 0.2, "{lo} {hi}");
    }

    #[test]
    fn giro_ciclico_vale_a_volta_mais_curta() {
        let f = Faixa::nova(0., 270.);
        assert_eq!(f.desvio(-100., true), 0., "−100° é o mesmo lugar que 260°");
        assert_eq!(f.desvio(-45., true), -45., "−45° fica a 45° do mínimo");
        assert_eq!(Faixa::nova(-170., 170.).desvio(175., true), 5.);
        let so_max = Faixa { max: Limite { ligado: true, valor: 170. }, ..Default::default() };
        assert_eq!(so_max.desvio(-175., true), 0., "com um extremo só a faixa não fecha no círculo");
    }

    #[test]
    fn minimo_acima_do_maximo_nao_entra_em_panico() {
        let (mut a, _, j) = manivela();
        com_limite(&mut a, j, 30., -30.);
        a.joints[0].drive_angle = Some("10".into());
        let rep = solve_with(&mut a, &HashMap::new());
        assert!(rep.failed.contains(&j), "{rep:?}");
    }

    #[test]
    fn arquivo_antigo_com_faixa_em_par_ainda_abre() {
        let l: Limites = serde_json::from_str(r#"{"angulo":[-45.0,90.0],"curso":null}"#).unwrap();
        assert_eq!(l.angulo, Faixa::nova(-45., 90.));
        assert!(!l.curso.ligada());
        let volta: Limites = serde_json::from_str(&serde_json::to_string(&l).unwrap()).unwrap();
        assert_eq!(volta, l);
    }

    #[test]
    fn levar_a_nao_deixa_comando_gravado() {
        let (mut a, _, j) = manivela();
        com_limite(&mut a, j, -90., 90.);
        assert!(levar_a(&mut a, j, Grau::Giro, 37.).converged);
        assert!((giro(&a, j) - 37.).abs() < 1e-6);
        assert!(a.joints[0].drive_angle.is_none());
        assert!(solve_with(&mut a, &HashMap::new()).converged);
        assert!((giro(&a, j) - 37.).abs() < 1e-6, "a peça fica onde foi deixada");
    }

    #[test]
    fn revolucao_com_comando_de_angulo() {
        let (mut a, braco, j) = manivela();
        let (sem, _) = analyze(&a);
        assert_eq!(sem, 1, "revolução deixa 1 GDL");
        a.joints.iter_mut().find(|x| x.id == j).unwrap().drive_angle = Some("90".into());
        let rep = solve_with(&mut a, &HashMap::new());
        assert!(rep.converged, "{rep:?}");
        assert_eq!(analyze(&a).1, 0, "com o comando: 0 GDL");
        let x = a.node(braco).unwrap().placement.dir([1., 0., 0.]);
        assert!(norm(sub(x, [0., 1., 0.])) < 1e-6, "{x:?}");
    }

    #[test]
    fn lei_no_tempo_e_velocidade() {
        let (mut a, braco, j) = manivela();
        // gira 90° por segundo
        a.joints.iter_mut().find(|x| x.id == j).unwrap().drive_angle = Some("90*t".into());
        let frames = simulate(&mut a, 0., 1., 10);
        assert!(frames.iter().all(|f| f.ok));
        let k = kinematics_of(&frames, braco, [10., 0., 0.]);
        // ponto a 10 mm do eixo: v = ω r = (π/2) × 10
        let v = k[5].2;
        assert!((v - std::f64::consts::FRAC_PI_2 * 10.).abs() < 0.3, "{v}");
    }

    #[test]
    fn parafuso_e_engrenagem() {
        let mut a = Assembly::new("par");
        let base = a.add_component("Base", NodeKind::Part { path: "b".into() }, Placement::default(), None);
        let porca = a.add_component("Porca", NodeKind::Part { path: "p".into() }, Placement::default(), None);
        let f = JFrame::from_axis([0.; 3], [0., 0., 1.]);
        let js = a.add_joint(JointKind::Screw { pitch: "2".into() }, JRef { comp: base, frame: f }, JRef { comp: porca, frame: f });
        a.joints.iter_mut().find(|x| x.id == js).unwrap().drive_angle = Some("90".into());
        assert!(solve_with(&mut a, &HashMap::new()).converged);
        assert!((a.node(porca).unwrap().placement.origin[2] - 0.5).abs() < 1e-6, "um quarto de volta × passo 2 = 0,5 mm");
        // engrenagem 1:−2 entre duas revoluções
        let mut g = Assembly::new("eng");
        let base = g.add_component("Base", NodeKind::Part { path: "b".into() }, Placement::default(), None);
        let e1 = g.add_component("E1", NodeKind::Part { path: "e1".into() }, Placement::default(), None);
        let e2 = g.add_component("E2", NodeKind::Part { path: "e2".into() }, Placement { origin: [30., 0., 0.], ..Default::default() }, None);
        let j1 = g.add_joint(JointKind::Revolute, JRef { comp: base, frame: f }, JRef { comp: e1, frame: f });
        let f2 = JFrame::from_axis([30., 0., 0.], [0., 0., 1.]);
        let j2 = g.add_joint(JointKind::Revolute, JRef { comp: base, frame: f2 }, JRef { comp: e2, frame: JFrame::from_axis([0.; 3], [0., 0., 1.]) });
        g.add_coupling(CouplingKind::Gear, j1, j2, "-0.5");
        g.joints.iter_mut().find(|x| x.id == j1).unwrap().drive_angle = Some("60".into());
        assert!(solve_with(&mut g, &HashMap::new()).converged);
        let pl: HashMap<Id, Placement> = g.nodes.iter().map(|n| (n.id, n.placement)).collect();
        let (a2, _) = measure(g.joints.iter().find(|x| x.id == j2).unwrap(), &pl).unwrap();
        assert!((a2 + 30.).abs() < 1e-4, "{a2}");
    }

    /// Base, pinhão e coroa em duas revoluções ligadas por um acoplamento de engrenagem.
    fn par(razao: &str) -> (Assembly, Id, Id) {
        let mut g = Assembly::new("par");
        let base = g.add_component("Base", NodeKind::Part { path: "b".into() }, Placement::default(), None);
        g.node_mut(base).unwrap().fixed = true;
        let e1 = g.add_component("Pinhão", NodeKind::Part { path: "e1".into() }, Placement::default(), None);
        let e2 = g.add_component("Coroa", NodeKind::Part { path: "e2".into() }, Placement { origin: [30., 0., 0.], ..Default::default() }, None);
        let f = JFrame::from_axis([0.; 3], [0., 0., 1.]);
        let j1 = g.add_joint(JointKind::Revolute, JRef { comp: base, frame: f }, JRef { comp: e1, frame: f });
        let j2 = g.add_joint(JointKind::Revolute, JRef { comp: base, frame: JFrame::from_axis([30., 0., 0.], [0., 0., 1.]) }, JRef { comp: e2, frame: f });
        g.add_coupling(CouplingKind::Gear, j1, j2, razao);
        (g, j1, j2)
    }

    fn junta(a: &mut Assembly, j: Id) -> &mut Joint {
        a.joints.iter_mut().find(|x| x.id == j).unwrap()
    }

    /// Giro da junta no quadro, pela posição das peças.
    fn giro_no_quadro(a: &Assembly, j: Id, q: &Frame) -> f64 {
        measure(a.joints.iter().find(|x| x.id == j).unwrap(), &q.placements).unwrap().0
    }

    #[test]
    fn engrenagem_11_82_acompanha_a_volta_inteira_do_pinhao() {
        let (mut g, j1, j2) = par("-11/82");
        junta(&mut g, j1).drive_angle = Some("t".into());
        // duas voltas, de 45° em 45°: passa por 180° e por 360°
        let quadros = simulate(&mut g, 0., 720., 16);
        for q in &quadros {
            assert!(q.ok, "t = {}", q.t);
            let esperado = -11. / 82. * q.t;
            let lido = giro_no_quadro(&g, j2, q);
            assert!((lido - esperado).abs() < 1e-3, "t = {}: {lido}, esperado {esperado}", q.t);
        }
        let volta = quadros.iter().find(|q| q.t == 360.).unwrap();
        assert!((giro_no_quadro(&g, j2, volta) + 48.293).abs() < 1e-2, "uma volta do pinhão");
    }

    #[test]
    fn o_giro_guardado_segura_a_coroa_sem_comando() {
        let (mut g, j1, j2) = par("-11/82");
        levar_a(&mut g, j1, Grau::Giro, 225.);
        assert!((junta(&mut g, j1).giro - 225.).abs() < 1e-6, "{}", junta(&mut g, j1).giro);
        // comando desligado: a próxima solução e o arquivo sabem em que volta o pinhão está
        let mut lida = Assembly::from_json(&g.to_json()).unwrap();
        let rep = solve_with(&mut lida, &HashMap::new());
        assert!(rep.converged, "{rep:?}");
        let pl: HashMap<Id, Placement> = lida.nodes.iter().map(|n| (n.id, n.placement)).collect();
        let (coroa, _) = measure(junta(&mut lida, j2), &pl).unwrap();
        assert!((coroa + 225. * 11. / 82.).abs() < 1e-3, "{coroa}");
    }

    #[test]
    fn engrenagens_em_cadeia_herdam_a_volta_da_motora() {
        let (mut g, j1, j2) = par("-1");
        let base = g.nodes[0].id;
        let e3 = g.add_component("Terceira", NodeKind::Part { path: "e3".into() }, Placement { origin: [60., 0., 0.], ..Default::default() }, None);
        let f = JFrame::from_axis([0.; 3], [0., 0., 1.]);
        let j3 = g.add_joint(JointKind::Revolute, JRef { comp: base, frame: JFrame::from_axis([60., 0., 0.], [0., 0., 1.]) }, JRef { comp: e3, frame: f });
        g.add_coupling(CouplingKind::Gear, j2, j3, "-0.25");
        junta(&mut g, j1).drive_angle = Some("t".into());
        for q in &simulate(&mut g, 0., 360., 8) {
            assert!(q.ok, "t = {}", q.t);
            let (lido, esperado) = (giro_no_quadro(&g, j3, q), 0.25 * q.t);
            assert!((lido - esperado).abs() < 1e-3, "t = {}: {lido}, esperado {esperado}", q.t);
        }
    }

    #[test]
    fn cremalheira_anda_alem_de_meia_volta_do_pinhao() {
        let mut g = Assembly::new("cremalheira");
        let base = g.add_component("Base", NodeKind::Part { path: "b".into() }, Placement::default(), None);
        g.node_mut(base).unwrap().fixed = true;
        let pinhao = g.add_component("Pinhão", NodeKind::Part { path: "p".into() }, Placement::default(), None);
        let barra = g.add_component("Barra", NodeKind::Part { path: "c".into() }, Placement::default(), None);
        let fz = JFrame::from_axis([0.; 3], [0., 0., 1.]);
        let fx = JFrame::from_axis([0.; 3], [1., 0., 0.]);
        let jr = g.add_joint(JointKind::Revolute, JRef { comp: base, frame: fz }, JRef { comp: pinhao, frame: fz });
        let jp = g.add_joint(JointKind::Prismatic, JRef { comp: base, frame: fx }, JRef { comp: barra, frame: fx });
        g.add_coupling(CouplingKind::RackPinion, jr, jp, "100");
        junta(&mut g, jr).drive_angle = Some("t".into());
        for q in &simulate(&mut g, 0., 540., 12) {
            assert!(q.ok, "t = {}", q.t);
            let x = q.placements[&barra].origin[0];
            assert!((x - 100. * q.t / 360.).abs() < 1e-3, "t = {}: {x}", q.t);
        }
    }

    #[test]
    fn rosca_avanca_por_varias_voltas() {
        let mut a = Assembly::new("rosca");
        let base = a.add_component("Base", NodeKind::Part { path: "b".into() }, Placement::default(), None);
        a.node_mut(base).unwrap().fixed = true;
        let porca = a.add_component("Porca", NodeKind::Part { path: "p".into() }, Placement::default(), None);
        let f = JFrame::from_axis([0.; 3], [0., 0., 1.]);
        let js = a.add_joint(JointKind::Screw { pitch: "2".into() }, JRef { comp: base, frame: f }, JRef { comp: porca, frame: f });
        junta(&mut a, js).drive_angle = Some("t".into());
        for q in &simulate(&mut a, 0., 1080., 24) {
            assert!(q.ok, "t = {}", q.t);
            let z = q.placements[&porca].origin[2];
            assert!((z - 2. * q.t / 360.).abs() < 1e-4, "t = {}: {z}", q.t);
        }
        // pelo curso: a rosca é levada a 5 voltas de uma vez
        junta(&mut a, js).drive_angle = None;
        junta(&mut a, js).drive_len = Some("10".into());
        assert!(solve_with(&mut a, &HashMap::new()).converged);
        assert!((a.node(porca).unwrap().placement.origin[2] - 10.).abs() < 1e-4);
    }

    #[test]
    fn came_e_seguidor() {
        // came circular excêntrico (R20, centro a 5 mm do eixo) girando; seguidor desliza em y
        let mut a = Assembly::new("came");
        let base = a.add_component("Base", NodeKind::Part { path: "b".into() }, Placement::default(), None);
        a.node_mut(base).unwrap().fixed = true;
        let came = a.add_component("Came", NodeKind::Part { path: "c".into() }, Placement::default(), None);
        let y0 = (400f64 - 25.).sqrt();
        let seg = a.add_component("Seguidor", NodeKind::Part { path: "s".into() }, Placement { origin: [0., y0, 0.], ..Default::default() }, None);
        let fz = JFrame::from_axis([0.; 3], [0., 0., 1.]);
        let jr = a.add_joint(JointKind::Revolute, JRef { comp: base, frame: fz }, JRef { comp: came, frame: fz });
        let fy = JFrame::from_axis([0.; 3], [0., 1., 0.]);
        a.add_joint(JointKind::Prismatic, JRef { comp: base, frame: fy }, JRef { comp: seg, frame: fy });
        let profile: Vec<V3> = (0..360).map(|i| {
            let t = (i as f64).to_radians();
            [5. + 20. * t.cos(), 20. * t.sin(), 0.]
        })
        .collect();
        a.add_joint(JointKind::Cam { profile, radius: "0".into() }, JRef { comp: came, frame: fz }, JRef { comp: seg, frame: fz });
        assert_eq!(analyze(&a).0, 1, "came-seguidor: 1 GDL (o giro do came)");
        a.joints.iter_mut().find(|x| x.id == jr).unwrap().drive_angle = Some("90".into());
        let rep = solve_with(&mut a, &HashMap::new());
        assert!(rep.converged, "{rep:?}");
        let y = a.node(seg).unwrap().placement.origin[1];
        assert!((y - 25.).abs() < 0.05, "seguidor no topo do came: {y}");
    }

    #[test]
    fn mecanismo_a_partir_das_restricoes() {
        let mut a = Assembly::new("auto");
        let c1 = a.add_component("A", NodeKind::Part { path: "a".into() }, Placement::default(), None);
        let c2 = a.add_component("B", NodeKind::Part { path: "b".into() }, Placement::default(), None);
        let ax = Geom::Axis { p: [0.; 3], d: [0., 0., 1.] };
        a.add_mate(MateKind::Coincident { flip: false }, MateRef { comp: c1, geom: ax, pick: None, viva: None }, MateRef { comp: c2, geom: ax, pick: None, viva: None });
        a.add_mate(MateKind::Contact, MateRef { comp: c1, geom: Geom::Plane { p: [0., 0., 5.], n: [0., 0., 1.] }, pick: None, viva: None }, MateRef { comp: c2, geom: Geom::Plane { p: [0., 0., 5.], n: [0., 0., -1.] }, pick: None, viva: None });
        let js = a.joints_from_mates();
        assert_eq!(js.len(), 1);
        assert!(a.mates.is_empty());
        assert_eq!(a.joints[0].kind, JointKind::Revolute);
    }
}
