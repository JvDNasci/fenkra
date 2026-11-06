// SPDX-License-Identifier: LGPL-2.1-or-later
//! Comando de restrição ou junta antes de existir (B244). Os dois elementos, o tipo e os parâmetros
//! ficam aqui, fora da montagem. `Comando::resolver` faz tudo numa cópia: acrescenta o item, resolve
//! e devolve a cópia com as peças que se moveram. A montagem de verdade só muda quando a pessoa
//! aceita e a cópia toma o lugar dela, então cancelar não tem nada a desfazer e o que aparece como
//! fantasma é exatamente o que será gravado.

use crate::kin::{JFrame, JRef, JointKind, Limites};
use crate::solve::{self, SolveReport};
use crate::*;

/// O que foi clicado, como a pessoa reconhece.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Classe {
    Face,
    ArestaReta,
    ArestaCurva { circular: bool },
    Vertice,
}

/// Um elemento escolhido no 3D, com o que a junta pode precisar além da geometria.
#[derive(Clone, PartialEq, Debug)]
pub struct Elemento {
    pub r: MateRef,
    pub classe: Classe,
    /// polilinha da aresta, no sistema da peça (trajetória do ponto em curva)
    pub curva: Vec<V3>,
    /// contorno da seção da peça pelo plano da aresta, no sistema da peça (perfil do came)
    pub perfil: Option<Vec<V3>>,
    /// raio da aresta circular (rolete do seguidor)
    pub raio: Option<f64>,
}

impl Elemento {
    pub fn novo(r: MateRef, classe: Classe) -> Self {
        Elemento { r, classe, curva: vec![], perfil: None, raio: None }
    }
}

/// O modo do comando. Trocar o modo não mexe nos elementos já escolhidos.
#[derive(Clone, PartialEq, Debug)]
pub enum Tipo {
    /// o tipo mais lógico para o par
    Automatica,
    Restricao(MateKind),
    /// posiciona pelo par e não guarda a restrição
    Alinhar,
    /// a junta que a geometria dos dois elementos pede
    JuntaAutomatica,
    Junta(JointKind),
}

/// Onde fica a origem da junta numa peça.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum ModoOrigem {
    /// no elemento clicado
    #[default]
    EmPonto,
    /// no meio de duas faces paralelas
    EntreFaces,
    /// no cruzamento de duas arestas retas
    IntersecaoArestas,
}

impl ModoOrigem {
    /// Quantos elementos este modo pede na caixa.
    pub fn elementos(self) -> usize {
        if self == ModoOrigem::EmPonto {
            1
        } else {
            2
        }
    }
}

/// Alinhamento da peça B em relação à A, na criação da junta.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct Alinhamento {
    /// giro inicial em volta do eixo da junta (graus)
    pub angulo: f64,
    /// deslocamento da origem de B nos eixos da junta de A (mm)
    pub desloc: [f64; 3],
    /// vira a peça: o eixo de B passa a apontar ao contrário
    pub inverter: bool,
}

impl Tipo {
    pub fn e_junta(&self) -> bool {
        matches!(self, Tipo::Junta(_) | Tipo::JuntaAutomatica)
    }
    pub fn label(&self) -> &'static str {
        match self {
            Tipo::Automatica | Tipo::JuntaAutomatica => "Automática",
            Tipo::Alinhar => "Alinhar",
            Tipo::Restricao(k) => k.label(),
            Tipo::Junta(k) => k.label(),
        }
    }
}

/// Uma peça que a proposta tira do lugar.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Movido {
    pub comp: Id,
    pub antes: Placement,
    pub depois: Placement,
}

impl Movido {
    /// Transformação que leva o que está na posição de antes para a posição final.
    pub fn delta(&self) -> Placement {
        self.depois.then(&self.antes.inverse())
    }
}

/// Resultado de resolver a proposta numa cópia da montagem.
#[derive(Clone, Debug)]
pub struct Previa {
    pub copia: Assembly,
    /// a restrição ou junta dentro da cópia (o Alinhar não deixa item nenhum)
    pub id: Option<Id>,
    pub movidos: Vec<Movido>,
    pub report: SolveReport,
    /// restrições que passaram a falhar por causa da proposta (as que já falhavam não contam)
    pub novas_falhas: Vec<Id>,
    /// valor medido (ou digitado) de Distância e Ângulo, para o campo do painel
    pub valor: Option<String>,
    pub aceita_alinhamento: bool,
    pub aceita_lado: bool,
}

impl Previa {
    /// A própria proposta ficou sem solução, ou derrubou outra restrição.
    pub fn falhou(&self) -> bool {
        !self.novas_falhas.is_empty()
    }
}

/// Comando pendente.
#[derive(Clone, PartialEq, Debug)]
pub struct Comando {
    pub tipo: Tipo,
    pub caixas: [Option<Elemento>; 2],
    /// segundo elemento das caixas cuja origem pede dois
    pub segundos: [Option<Elemento>; 2],
    pub modos: [ModoOrigem; 2],
    pub alinhamento: Alinhamento,
    pub limites: Limites,
    /// caixa que recebe o próximo clique
    pub ativa: usize,
    /// valor digitado; sem ele vale a medida de agora
    pub valor: Option<String>,
    pub inv_alinhamento: bool,
    pub inv_lado: bool,
    /// nome pedido no avançado (vazio: nome automático)
    pub nome: String,
    pub suprimida: bool,
}

impl Comando {
    pub fn novo(tipo: Tipo) -> Self {
        Comando { tipo, caixas: [None, None], segundos: [None, None], modos: Default::default(), alinhamento: Default::default(), limites: Default::default(), ativa: 0, valor: None, inv_alinhamento: false, inv_lado: false, nome: String::new(), suprimida: false }
    }

    /// A trajetória e o seguidor se reconhecem pelo tipo: o clique vale em qualquer ordem.
    fn caixa_pelo_tipo(&self, e: &Elemento) -> Option<usize> {
        match (&self.tipo, e.classe) {
            (Tipo::Junta(JointKind::PointCurve { .. } | JointKind::Cam { .. }), Classe::Vertice) => Some(1),
            (Tipo::Junta(JointKind::PointCurve { .. }), Classe::ArestaReta | Classe::ArestaCurva { .. }) => Some(0),
            (Tipo::Junta(JointKind::Cam { .. }), Classe::ArestaReta | Classe::ArestaCurva { circular: false }) => Some(0),
            // círculo pode ser o rolete ou um came excêntrico: fica com a caixa que estiver livre
            (Tipo::Junta(JointKind::Cam { .. }), Classe::ArestaCurva { circular: true }) => Some(if self.caixas[1].is_none() { 1 } else { 0 }),
            _ => None,
        }
    }

    /// Nomes das duas caixas no modo atual.
    pub fn rotulos(&self) -> [&'static str; 2] {
        match self.tipo {
            Tipo::Junta(JointKind::PointCurve { .. } | JointKind::Cam { .. }) => ["Trajetória", "Seguidor"],
            _ => ["Elemento 1", "Elemento 2"],
        }
    }

    /// A caixa `k` tem o que o modo de origem dela pede?
    pub fn cheia(&self, k: usize) -> bool {
        self.caixas[k].is_some() && (self.modos[k].elementos() == 1 || self.segundos[k].is_some())
    }

    /// Falta o segundo elemento da origem nesta caixa.
    pub fn pede_segundo(&self, k: usize) -> bool {
        self.caixas[k].is_some() && !self.cheia(k)
    }

    /// Põe o elemento na caixa certa e avança para a que falta. Devolve a caixa usada.
    pub fn receber(&mut self, e: Elemento) -> usize {
        let k = self.caixa_pelo_tipo(&e).unwrap_or(self.ativa);
        if self.pede_segundo(k) {
            self.segundos[k] = Some(e);
        } else {
            self.caixas[k] = Some(e);
            self.segundos[k] = None;
        }
        self.ativa = match (0..2).find(|i| !self.cheia(*i)) {
            Some(i) => i,
            None => k,
        };
        k
    }

    /// Esvazia uma caixa e a deixa pronta para o próximo clique.
    pub fn limpar_caixa(&mut self, k: usize) {
        self.caixas[k] = None;
        self.segundos[k] = None;
        self.ativa = k;
    }

    /// Troca o conteúdo das duas caixas (came de círculo clicado na ordem errada).
    pub fn trocar_caixas(&mut self) {
        self.caixas.swap(0, 1);
        self.segundos.swap(0, 1);
        self.modos.swap(0, 1);
    }

    /// Muda o modo de origem de uma caixa. O segundo elemento era do modo anterior.
    pub fn trocar_modo_origem(&mut self, k: usize, m: ModoOrigem) {
        if self.modos[k] != m {
            self.modos[k] = m;
            self.segundos[k] = None;
            if !self.cheia(k) {
                self.ativa = k;
            }
        }
    }

    /// Etapa em que o comando está: 0 e 1 pedem um elemento; 2 tem tudo.
    pub fn etapa(&self) -> usize {
        (0..2).filter(|k| self.cheia(*k)).count()
    }

    pub fn completo(&self) -> bool {
        self.etapa() == 2
    }

    /// Muda o modo sem repetir a seleção. O valor digitado era do modo anterior e não vale mais.
    pub fn trocar_tipo(&mut self, tipo: Tipo) {
        if tipo != self.tipo {
            self.tipo = tipo;
            self.valor = None;
            self.inv_alinhamento = false;
            self.inv_lado = false;
        }
    }

    /// Volta às caixas vazias para o próximo par, mantendo o modo (Enter aceita e continua).
    pub fn continuar(&mut self) {
        *self = Comando::novo(self.tipo.clone());
    }

    /// Inverte pela tecla: o alinhamento, se o par tiver; se não, o lado. Devolve se mudou algo.
    pub fn inverter(&mut self, previa: Option<&Previa>) -> bool {
        if self.tipo.e_junta() {
            self.alinhamento.inverter = !self.alinhamento.inverter;
            return true;
        }
        match previa {
            Some(p) if p.aceita_alinhamento => self.inv_alinhamento = !self.inv_alinhamento,
            Some(p) if p.aceita_lado => self.inv_lado = !self.inv_lado,
            _ => return false,
        }
        true
    }

    /// Resolve a proposta numa cópia de `base`, que fica intacta.
    pub fn resolver(&self, base: &Assembly) -> Result<Previa, String> {
        let [Some(a), Some(b)] = &self.caixas else { return Err("faltam elementos".into()) };
        if !self.completo() {
            return Err("faltam elementos".into());
        }
        if a.r.comp == b.r.comp {
            return Err("Os dois elementos são do mesmo componente".into());
        }
        let mut copia = base.clone();
        let (id, valor, aceita_alinhamento, aceita_lado) = match &self.tipo {
            Tipo::Junta(k) => (self.montar_junta(&mut copia, k, false)?, None, false, false),
            Tipo::JuntaAutomatica => {
                let (k, trocar) = junta_automatica(a, b);
                (self.montar_junta(&mut copia, &k, trocar)?, None, false, false)
            }
            _ => self.montar_restricao(&mut copia, a, b)?,
        };
        if !self.nome.trim().is_empty() {
            if let Some(m) = copia.mates.iter_mut().find(|m| m.id == id) {
                m.name = self.nome.trim().to_string();
            }
            if let Some(j) = copia.joints.iter_mut().find(|j| j.id == id) {
                j.name = self.nome.trim().to_string();
            }
        }
        if self.suprimida {
            if let Some(m) = copia.mates.iter_mut().find(|m| m.id == id) {
                m.suppressed = true;
            }
            if let Some(j) = copia.joints.iter_mut().find(|j| j.id == id) {
                j.suppressed = true;
            }
        }
        let report = solve::solve(&mut copia);
        let so_posicionar = self.tipo == Tipo::Alinhar;
        if so_posicionar {
            copia.mates.retain(|m| m.id != id);
        }
        let ja_falhavam = solve::solve(&mut base.clone()).failed;
        let novas_falhas = report.failed.iter().copied().filter(|f| !ja_falhavam.contains(f)).collect();
        let movidos = movidos(base, &copia);
        Ok(Previa { copia, id: (!so_posicionar).then_some(id), movidos, report, novas_falhas, valor, aceita_alinhamento, aceita_lado })
    }

    fn montar_restricao(&self, c: &mut Assembly, a: &Elemento, b: &Elemento) -> Result<(Id, Option<String>, bool, bool), String> {
        let lugar = |id: Id| c.lugar(id).unwrap_or_default();
        let kind = match &self.tipo {
            Tipo::Restricao(k) => k.clone(),
            _ => tipo_automatico(&a.r.geom, &b.r.geom, &lugar(a.r.comp), &lugar(b.r.comp)),
        };
        if !par_valido(&kind, &a.r.geom, &b.r.geom) {
            return Err("este par de elementos não define a restrição (escolha planos ou eixos)".into());
        }
        let id = c.add_mate(kind, a.r, b.r);
        // engrenagem e cremalheira: o giro de agora é o zero
        c.init_motion(id);
        // distância e ângulo: a medida de agora, para a peça não saltar
        c.iniciar_medida(id);
        if let (Some(v), Some(m)) = (&self.valor, c.mates.iter_mut().find(|m| m.id == id)) {
            match &mut m.kind {
                MateKind::Distance(e) | MateKind::Angle(e) => *e = v.clone(),
                MateKind::Gear { ratio: e, .. } | MateKind::RackPinion { mm_per_rev: e, .. } => *e = v.clone(),
                _ => {}
            }
        }
        if self.inv_alinhamento {
            c.inverter_alinhamento(id);
        }
        if self.inv_lado {
            c.inverter_lado(id);
        }
        let m = c.mates.iter().find(|m| m.id == id).ok_or("restrição sumiu")?;
        let valor = match &m.kind {
            MateKind::Distance(e) | MateKind::Angle(e) => Some(e.clone()),
            MateKind::Gear { ratio: e, .. } | MateKind::RackPinion { mm_per_rev: e, .. } => Some(e.clone()),
            _ => None,
        };
        Ok((id, valor, m.aceita_alinhamento(), m.aceita_lado()))
    }
}

impl Comando {
    /// Geometria da origem da caixa `k`, no sistema da peça dela.
    fn geom_da_origem(&self, k: usize) -> Result<Geom, String> {
        let a = self.caixas[k].as_ref().ok_or("faltam elementos")?;
        let Some(b) = &self.segundos[k] else { return Ok(a.r.geom) };
        if a.r.comp != b.r.comp {
            return Err("As duas referências da origem são da mesma peça".into());
        }
        origem_composta(self.modos[k], &a.r.geom, &b.r.geom)
    }

    /// Acrescenta a junta na cópia. O x de B começa como o x de A projetado no plano de B, para o
    /// zero do ângulo ser comum às duas peças. `trocar`: a geometria pediu A e B na ordem contrária.
    fn montar_junta(&self, c: &mut Assembly, k: &JointKind, trocar: bool) -> Result<Id, String> {
        let (ia, ib) = if trocar { (1, 0) } else { (0, 1) };
        let (a, b) = (self.caixas[ia].as_ref().ok_or("faltam elementos")?, self.caixas[ib].as_ref().ok_or("faltam elementos")?);
        let (ga, gb) = (self.geom_da_origem(ia)?, self.geom_da_origem(ib)?);
        let lugar = |c: &Assembly, id: Id| c.lugar(id).unwrap_or_default();
        let (pa, pb) = (lugar(c, a.r.comp), lugar(c, b.r.comp));
        let fa = sistema_com(&ga, &pa, &gb, &pb);
        let mut fb = sistema_com(&gb, &pb, &ga, &pa);
        let al = self.alinhamento;
        let (oa, za, xa) = fa.world(&pa);
        let ya = cross(za, xa);
        let (ob, zb, xb) = fb.world(&pb);
        // quem anda antes de medir: a peça B, ou a A se B estiver presa. O solucionador só olha o eixo
        // (cross(za, zb) zera nos dois sentidos), então virar e girar precisam vir da posição de saída
        let presa = |c: &Assembly, id: Id| id == ORIGEM || c.node(id).is_none_or(|n| n.fixed);
        let (movel, sinal, em) = if presa(c, b.r.comp) { (a.r.comp, -1., oa) } else { (b.r.comp, 1., ob) };
        let giro = |c: &mut Assembly, w: V3, t: V3| {
            if let Some(n) = c.node_mut(movel) {
                n.placement = n.placement.moved(t.map(|v| v * sinal), w.map(|v| v * sinal), em);
            }
        };
        if al.inverter {
            giro(c, xb.map(|v| v * std::f64::consts::PI), [0.; 3]);
        }
        if al.angulo != 0. {
            giro(c, zb.map(|v| v * al.angulo.to_radians()), [0.; 3]);
        }
        // eixo que mede o curso a partir da posição de agora: Z desloca a peça; nos outros o z do
        // sistema precisa coincidir, e o deslocamento vai para a origem de B
        let curso_medido = matches!(k, JointKind::Revolute | JointKind::Prismatic | JointKind::Cylindrical | JointKind::Screw { .. });
        if curso_medido && al.desloc[2] != 0. {
            giro(c, [0.; 3], za.map(|v| v * al.desloc[2]));
        }
        let dz = if curso_medido { 0. } else { al.desloc[2] };
        let d = [0, 1, 2].map(|i| xa[i] * al.desloc[0] + ya[i] * al.desloc[1] + za[i] * dz);
        // eixos já paralelos: a peça vai direto para o lugar. Deixar para o solucionador custaria um
        // giro à toa, porque ele divide o caminho entre as seis incógnitas
        let (_, zb2, _) = fb.world(&lugar(c, b.r.comp));
        if norm(cross(za, zb2)) < 1e-9 {
            let (ob2, ..) = fb.world(&lugar(c, b.r.comp));
            let falta = sub(oa, ob2);
            let ao_longo = dot(falta, za);
            let radial = [0, 1, 2].map(|i| falta[i] - za[i] * ao_longo);
            let leva = match k {
                JointKind::Revolute | JointKind::Prismatic | JointKind::Cylindrical | JointKind::Screw { .. } => Some(radial),
                JointKind::Spherical | JointKind::Rigid => Some(falta),
                _ => None,
            };
            if let Some(t) = leva {
                giro(c, [0.; 3], [0, 1, 2].map(|i| t[i] + d[i]));
            }
        }
        let pb = lugar(c, b.r.comp);
        fb.o = sub(fb.o, pb.dir_to_local(d));
        let pa = lugar(c, a.r.comp);
        let xw = pb.dir_to_local(pa.dir(fa.x));
        let k2 = dot(xw, fb.z);
        let px = [xw[0] - fb.z[0] * k2, xw[1] - fb.z[1] * k2, xw[2] - fb.z[2] * k2];
        if norm(px) > 1e-6 {
            fb.x = unit(px);
        }
        let kind = match k {
            JointKind::PointCurve { .. } if a.curva.len() < 2 => return Err("Ponto em curva: a trajetória é uma aresta".into()),
            JointKind::PointCurve { .. } => JointKind::PointCurve { curve: a.curva.clone() },
            JointKind::Cam { radius, .. } => {
                let Some(perfil) = a.perfil.clone() else { return Err("Came: o perfil é uma aresta do came".into()) };
                let raio = b.raio.map(|r| format!("{r:.3}")).unwrap_or_else(|| radius.clone());
                JointKind::Cam { profile: perfil, radius: raio }
            }
            k => k.clone(),
        };
        let id = c.add_joint(kind, JRef { comp: a.r.comp, frame: fa }, JRef { comp: b.r.comp, frame: fb });
        if let Some(j) = c.joints.iter_mut().find(|j| j.id == id) {
            j.limites = self.limites;
        }
        Ok(id)
    }
}

/// Origem que depende de duas referências da mesma peça.
fn origem_composta(modo: ModoOrigem, a: &Geom, b: &Geom) -> Result<Geom, String> {
    match (modo, a, b) {
        (ModoOrigem::EntreFaces, Geom::Plane { p: p1, n: n1 }, Geom::Plane { p: p2, n: n2 }) => {
            if norm(cross(unit(*n1), unit(*n2))) > 1e-6 {
                return Err("Entre duas faces: as faces precisam ser paralelas".into());
            }
            // o meio entre os planos, na reta que passa pelo primeiro ponto clicado
            let n = unit(*n1);
            let d = dot(sub(*p2, *p1), n) / 2.;
            Ok(Geom::Plane { p: [0, 1, 2].map(|i| p1[i] + n[i] * d), n })
        }
        (ModoOrigem::IntersecaoArestas, Geom::Axis { p: p1, d: d1 }, Geom::Axis { p: p2, d: d2 }) => {
            let (u, v) = (unit(*d1), unit(*d2));
            let n = cross(u, v);
            if norm(n) < 1e-6 {
                return Err("Interseção de arestas: as arestas são paralelas".into());
            }
            // ponto mais perto das duas retas; se não se cruzam, não há interseção
            let w = sub(*p2, *p1);
            let t = dot(cross(w, v), n) / dot(n, n);
            let s = dot(cross(w, u), n) / dot(n, n);
            let q1 = [0, 1, 2].map(|i| p1[i] + u[i] * t);
            let q2 = [0, 1, 2].map(|i| p2[i] + v[i] * s);
            if norm(sub(q1, q2)) > 1e-4 {
                return Err("Interseção de arestas: as arestas não se cruzam".into());
            }
            Ok(Geom::Axis { p: q1, d: unit(n) })
        }
        (ModoOrigem::EntreFaces, ..) => Err("Entre duas faces: escolha duas faces planas".into()),
        (ModoOrigem::IntersecaoArestas, ..) => Err("Interseção de arestas: escolha duas arestas retas".into()),
        (ModoOrigem::EmPonto, a, _) => Ok(*a),
    }
}

/// Junta que a geometria pede. O segundo valor diz que A e B precisam trocar de lugar: a trajetória é
/// sempre a peça A.
pub fn junta_automatica(a: &Elemento, b: &Elemento) -> (JointKind, bool) {
    use Geom::*;
    let reta = |e: &Elemento| e.classe == Classe::ArestaReta;
    let curva = |e: &Elemento| matches!(e.classe, Classe::ArestaCurva { circular: false });
    match (a.r.geom, b.r.geom) {
        _ if curva(a) && b.classe == Classe::Vertice => (JointKind::PointCurve { curve: vec![] }, false),
        _ if curva(b) && a.classe == Classe::Vertice => (JointKind::PointCurve { curve: vec![] }, true),
        // duas arestas retas são um trilho; furo, pino e aresta circular são um eixo de giro
        (Axis { .. }, Axis { .. }) if reta(a) && reta(b) => (JointKind::Prismatic, false),
        (Axis { .. }, Axis { .. }) => (JointKind::Revolute, false),
        (Axis { .. }, Plane { .. }) | (Plane { .. }, Axis { .. }) => (JointKind::Revolute, false),
        (Plane { .. }, Plane { .. }) | (Point(_), Plane { .. }) | (Plane { .. }, Point(_)) => (JointKind::Planar, false),
        (Point(_), Axis { .. }) | (Axis { .. }, Point(_)) => (JointKind::Cylindrical, false),
        (Point(_), Point(_)) => (JointKind::Spherical, false),
    }
}

/// Opções do botão Animar.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct OpcoesAnimar {
    /// usa os limites da junta, quando houver, no lugar da faixa padrão
    pub limites: bool,
    /// faixa padrão: ± graus e ± mm
    pub giro: f64,
    pub curso: f64,
    pub quadros: usize,
}

impl Default for OpcoesAnimar {
    fn default() -> Self {
        OpcoesAnimar { limites: true, giro: 60., curso: 30., quadros: 72 }
    }
}

/// Valor na fase `s` (−1 a 1): de 0 até o máximo e até o mínimo, passando pelo zero. Faixa que não
/// contém o zero vai e volta entre as pontas.
fn varrer(faixa: (f64, f64), s: f64) -> f64 {
    let (lo, hi) = faixa;
    if lo <= 0. && hi >= 0. {
        if s >= 0. {
            hi * s
        } else {
            -lo * s
        }
    } else {
        (lo + hi) / 2. + (hi - lo) / 2. * s
    }
}

/// Quadros da peça indo e voltando pela junta da prévia, sem mexer em nada: cada quadro traz as peças
/// que saem do lugar em relação a `base`. Sem comando nem janela de simulação: a junta recebe um
/// valor por quadro e a cópia é resolvida.
pub fn animar(base: &Assembly, previa: &Previa, op: &OpcoesAnimar) -> Result<Vec<Vec<Movido>>, String> {
    let id = previa.id.ok_or("Sem junta para animar")?;
    let j = previa.copia.joints.iter().find(|j| j.id == id).ok_or("Sem junta para animar")?.clone();
    let mut c = previa.copia.clone();
    if j.suppressed {
        return Err("A junta está suprimida".into());
    }
    let n = op.quadros.max(8);
    let fase = |k: usize| (std::f64::consts::TAU * k as f64 / n as f64).sin();
    let faixa_giro = if op.limites { j.limites.angulo.ou((-op.giro, op.giro)) } else { (-op.giro, op.giro) };
    let faixa_curso = if op.limites { j.limites.curso.ou((-op.curso, op.curso)) } else { (-op.curso, op.curso) };
    let mut quadros = Vec::with_capacity(n);
    let (ca, cl) = j.kind.drives();
    // o came gira pela junta da peça dele, e o seguidor acompanha
    let motor = if matches!(j.kind, JointKind::Cam { .. }) { c.joints.iter().find(|o| o.id != id && (o.a.comp == j.a.comp || o.b.comp == j.a.comp) && o.kind.drives().0).map(|o| o.id) } else { None };
    if matches!(j.kind, JointKind::Rigid) {
        return Err("A junta rígida não tem movimento para animar".into());
    }
    if matches!(j.kind, JointKind::Cam { .. }) && motor.is_none() {
        return Err("O came precisa de uma junta que o faça girar".into());
    }
    if ca || cl || motor.is_some() {
        for k in 0..n {
            let s = fase(k);
            for o in c.joints.iter_mut() {
                if Some(o.id) == motor {
                    o.drive_angle = Some(valor_cota(360. * k as f64 / n as f64));
                } else if o.id == id && motor.is_none() {
                    o.drive_angle = ca.then(|| valor_cota(varrer(faixa_giro, s)));
                    o.drive_len = cl.then(|| valor_cota(varrer(faixa_curso, s)));
                }
            }
            solve::solve(&mut c);
            quadros.push(movidos(base, &c));
        }
        return Ok(quadros);
    }
    // sem comando na junta: puxa a peça B pelo movimento que ela tem
    let comp = if c.node(j.b.comp).is_some_and(|n| !n.fixed) { j.b.comp } else { j.a.comp };
    let inicio = c.lugar(comp).ok_or("Sem peça para animar")?;
    let pa = c.lugar(j.a.comp).unwrap_or_default();
    let (o, z, x) = j.a.frame.world(&pa);
    for k in 0..n {
        let s = fase(k);
        let alvo = match &j.kind {
            JointKind::Spherical => inicio.moved([0.; 3], x.map(|v| v * varrer(faixa_giro, s).to_radians()), o),
            JointKind::Planar => inicio.moved(x.map(|v| v * varrer(faixa_curso, s)), z.map(|v| v * varrer(faixa_giro, s).to_radians()), o),
            JointKind::PointCurve { curve } => {
                // de ponta a ponta da curva, a partir do ponto mais perto
                let u = (s + 1.) / 2. * (curve.len() - 1) as f64;
                let (i, t) = ((u as usize).min(curve.len() - 2), u.fract());
                let q = [0, 1, 2].map(|e| curve[i][e] + (curve[i + 1][e] - curve[i][e]) * t);
                let atual = c.lugar(j.b.comp).unwrap_or_default().point(j.b.frame.o);
                let d = sub(pa.point(q), atual);
                c.lugar(comp).unwrap_or(inicio).moved(d, [0.; 3], o)
            }
            _ => inicio,
        };
        solve::solve_toward(&mut c, comp, alvo, 0.05);
        quadros.push(movidos(base, &c));
    }
    Ok(quadros)
}

/// Peças que mudaram de lugar da montagem `base` para `copia`.
pub fn movidos(base: &Assembly, copia: &Assembly) -> Vec<Movido> {
    let longe = |p: &Placement, q: &Placement| norm(sub(p.origin, q.origin)) > 1e-7 || (0..3).any(|k| norm(sub(p.rot[k], q.rot[k])) > 1e-7);
    copia
        .nodes
        .iter()
        .filter_map(|n| {
            let antes = base.node(n.id)?.placement;
            longe(&antes, &n.placement).then_some(Movido { comp: n.id, antes, depois: n.placement })
        })
        .collect()
}

/// Restrição automática: o tipo mais lógico para o par escolhido.
pub fn tipo_automatico(a: &Geom, b: &Geom, wa: &Placement, wb: &Placement) -> MateKind {
    match (a, b) {
        // faces de frente uma para a outra: contato; mesmo sentido: coincidência
        (Geom::Plane { n: na, .. }, Geom::Plane { n: nb, .. }) if dot(wa.dir(*na), wb.dir(*nb)) < 0. => MateKind::Contact,
        _ => MateKind::Coincident { flip: false },
    }
}

/// Sistema da junta a partir da geometria: o eixo ou a normal vira o z.
pub fn sistema(g: &Geom) -> JFrame {
    match *g {
        Geom::Axis { p, d } => JFrame::from_axis(p, d),
        Geom::Plane { p, n } => JFrame::from_axis(p, n),
        Geom::Point(p) => JFrame::from_axis(p, [0., 0., 1.]),
    }
}

/// Sistema de `g`, levando a direção de `outra` quando `g` é um ponto: ponto no cilindro gira e desliza
/// no eixo do cilindro, não no z da peça.
fn sistema_com(g: &Geom, p: &Placement, outra: &Geom, q: &Placement) -> JFrame {
    match (*g, *outra) {
        (Geom::Point(o), Geom::Axis { d, .. } | Geom::Plane { n: d, .. }) => JFrame::from_axis(o, p.dir_to_local(q.dir(d))),
        _ => sistema(g),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Base fixa na origem e peça solta 30 acima, com faces de frente uma para a outra.
    fn par() -> (Assembly, Id, Id) {
        let mut a = Assembly::new("par");
        let f = a.add_component("base", NodeKind::Part { path: "a.fkp".into() }, Placement::default(), None);
        a.node_mut(f).unwrap().fixed = true;
        let m = a.add_component("tampa", NodeKind::Part { path: "b.fkp".into() }, Placement { origin: [3., 4., 30.], ..Default::default() }, None);
        (a, f, m)
    }

    fn face(comp: Id, p: V3, n: V3) -> Elemento {
        Elemento::novo(MateRef { comp, geom: Geom::Plane { p, n }, pick: None }, Classe::Face)
    }

    fn eixo(comp: Id, p: V3, d: V3) -> Elemento {
        Elemento::novo(MateRef { comp, geom: Geom::Axis { p, d }, pick: None }, Classe::ArestaCurva { circular: true })
    }

    fn contato(f: Id, m: Id) -> Comando {
        let mut c = Comando::novo(Tipo::Restricao(MateKind::Contact));
        c.receber(face(f, [0., 0., 10.], [0., 0., 1.]));
        c.receber(face(m, [0., 0., 0.], [0., 0., -1.]));
        c
    }

    #[test]
    fn resolver_nao_toca_a_montagem() {
        let (a, f, m) = par();
        let antes = a.to_json();
        let p = contato(f, m).resolver(&a).unwrap();
        assert_eq!(a.to_json(), antes, "a prévia mexeu na montagem");
        assert!(a.mates.is_empty());
        assert_eq!(p.copia.mates.len(), 1);
        assert_eq!(p.movidos.len(), 1);
        assert_eq!(p.movidos[0].comp, m);
        assert!((p.movidos[0].depois.origin[2] - 10.).abs() < 1e-6, "encosta na face de cima");
        assert!(!p.falhou());
    }

    #[test]
    fn aceitar_grava_o_que_o_fantasma_mostrou() {
        let (mut a, f, m) = par();
        let p = contato(f, m).resolver(&a).unwrap();
        let final_previa = p.movidos[0].depois;
        a = p.copia;
        solve::solve(&mut a);
        assert!(norm(sub(a.node(m).unwrap().placement.origin, final_previa.origin)) < 1e-6);
    }

    #[test]
    fn trocar_o_tipo_nao_repete_a_selecao() {
        let (a, f, m) = par();
        let mut c = contato(f, m);
        let caixas = c.caixas.clone();
        c.trocar_tipo(Tipo::Restricao(MateKind::Distance("0".into())));
        assert_eq!(c.caixas, caixas);
        assert!(c.completo());
        let p = c.resolver(&a).unwrap();
        assert!(matches!(&p.copia.mates[0].kind, MateKind::Distance(v) if v == "20"), "nasce com a medida de agora");
        assert!(p.movidos.is_empty(), "distância medida não salta");
        c.trocar_tipo(Tipo::Junta(JointKind::Planar));
        assert!(c.resolver(&a).unwrap().copia.joints.len() == 1);
    }

    #[test]
    fn valor_digitado_move_a_previa() {
        let (a, f, m) = par();
        let mut c = contato(f, m);
        c.trocar_tipo(Tipo::Restricao(MateKind::Distance("0".into())));
        c.valor = Some("5".into());
        let p = c.resolver(&a).unwrap();
        assert!((p.movidos[0].depois.origin[2] - 15.).abs() < 1e-6);
        assert_eq!(p.valor.as_deref(), Some("5"));
    }

    #[test]
    fn tab_inverte_e_a_previa_acompanha() {
        let (a, f, m) = par();
        let mut c = contato(f, m);
        c.trocar_tipo(Tipo::Restricao(MateKind::Coincident { flip: true }));
        let p0 = c.resolver(&a).unwrap();
        let z0 = p0.copia.node(m).unwrap().placement.dir([0., 0., 1.])[2];
        assert!(c.inverter(Some(&p0)));
        let p1 = c.resolver(&a).unwrap();
        let z1 = p1.copia.node(m).unwrap().placement.dir([0., 0., 1.])[2];
        assert!((z0 - z1).abs() > 1.5, "a peça vira: {z0} {z1}");
    }

    #[test]
    fn tab_sem_alinhamento_passa_para_o_lado() {
        let (a, f, m) = par();
        let mut c = Comando::novo(Tipo::Restricao(MateKind::Distance("0".into())));
        c.receber(face(f, [0., 0., 10.], [0., 0., 1.]));
        c.receber(Elemento::novo(MateRef { comp: m, geom: Geom::Point([0., 0., 0.]), pick: None }, Classe::Vertice));
        c.valor = Some("20".into());
        let p0 = c.resolver(&a).unwrap();
        assert!(!p0.aceita_alinhamento && p0.aceita_lado);
        assert!(c.inverter(Some(&p0)));
        let p1 = c.resolver(&a).unwrap();
        assert!((p1.copia.node(m).unwrap().placement.origin[2] + 10.).abs() < 1e-6, "passa para baixo da face");
    }

    #[test]
    fn enter_continua_com_as_caixas_vazias_no_mesmo_modo() {
        let (_, f, m) = par();
        let mut c = contato(f, m);
        c.valor = Some("1".into());
        c.continuar();
        assert_eq!(c.etapa(), 0);
        assert_eq!(c.ativa, 0);
        assert_eq!(c.tipo, Tipo::Restricao(MateKind::Contact));
        assert_eq!(c.valor, None);
    }

    #[test]
    fn clique_vai_para_a_caixa_ativa_e_troca_so_ela() {
        let (_, f, m) = par();
        let mut c = contato(f, m);
        c.ativa = 0;
        let nova = face(f, [1., 1., 1.], [1., 0., 0.]);
        c.receber(nova.clone());
        assert_eq!(c.caixas[0], Some(nova));
        assert!(c.caixas[1].is_some());
        c.limpar_caixa(1);
        assert_eq!((c.ativa, c.etapa()), (1, 1));
    }

    #[test]
    fn conflito_fica_na_previa() {
        let (mut a, f, m) = par();
        a.node_mut(m).unwrap().fixed = true;
        let p = contato(f, m).resolver(&a).unwrap();
        assert!(p.falhou(), "as duas peças presas: não dá para encostar");
        assert!(p.movidos.is_empty());
        assert!(a.mates.is_empty());
    }

    #[test]
    fn par_sem_sentido_e_mesma_peca_recusam() {
        let (a, f, m) = par();
        let mut c = Comando::novo(Tipo::Restricao(MateKind::Parallel));
        c.receber(Elemento::novo(MateRef { comp: f, geom: Geom::Point([0.; 3]), pick: None }, Classe::Vertice));
        c.receber(face(m, [0.; 3], [0., 0., 1.]));
        assert!(c.resolver(&a).is_err());
        let mut c = Comando::novo(Tipo::Automatica);
        c.receber(face(f, [0.; 3], [0., 0., 1.]));
        c.receber(face(f, [1., 0., 0.], [0., 0., 1.]));
        assert!(c.resolver(&a).is_err());
    }

    #[test]
    fn alinhar_posiciona_e_nao_guarda() {
        let (a, f, m) = par();
        let mut c = contato(f, m);
        c.trocar_tipo(Tipo::Alinhar);
        let p = c.resolver(&a).unwrap();
        assert!(p.copia.mates.is_empty() && p.id.is_none());
        assert_eq!(p.movidos.len(), 1);
    }

    #[test]
    fn automatica_escolhe_pelas_normais() {
        let (a, f, m) = par();
        let mut c = contato(f, m);
        c.trocar_tipo(Tipo::Automatica);
        assert_eq!(c.resolver(&a).unwrap().copia.mates[0].kind, MateKind::Contact);
    }

    #[test]
    fn junta_entra_na_copia_e_prende_o_eixo() {
        let (a, f, m) = par();
        let mut c = Comando::novo(Tipo::Junta(JointKind::Revolute));
        c.receber(eixo(f, [10., 0., 0.], [0., 0., 1.]));
        c.receber(eixo(m, [0., 0., 0.], [0., 0., 1.]));
        let p = c.resolver(&a).unwrap();
        assert!(a.joints.is_empty());
        assert_eq!(p.copia.joints.len(), 1);
        let o = p.copia.node(m).unwrap().placement.origin;
        assert!((o[0] - 10.).abs() < 1e-5 && o[1].abs() < 1e-5, "eixos colineares: {o:?}");
        assert!((o[2] - 30.).abs() < 1e-5, "mantém a altura de agora");
        assert_eq!(solve::dof(&p.copia), 1);
    }

    #[test]
    fn junta_de_trajetoria_exige_aresta() {
        let (a, f, m) = par();
        let mut c = Comando::novo(Tipo::Junta(JointKind::PointCurve { curve: vec![] }));
        c.receber(eixo(f, [0.; 3], [0., 0., 1.]));
        c.receber(Elemento::novo(MateRef { comp: m, geom: Geom::Point([0.; 3]), pick: None }, Classe::Vertice));
        assert!(c.resolver(&a).is_err());
    }

    #[test]
    fn peca_presa_na_origem_da_montagem() {
        let (a, _, m) = par();
        let mut c = Comando::novo(Tipo::Restricao(MateKind::Contact));
        c.receber(face(ORIGEM, [0.; 3], [0., 0., 1.]));
        c.receber(face(m, [0.; 3], [0., 0., -1.]));
        let p = c.resolver(&a).unwrap();
        assert!(p.copia.node(m).unwrap().placement.origin[2].abs() < 1e-6, "encosta no plano XY");
        assert!(!p.falhou());
        assert_eq!(p.movidos.len(), 1);
        assert_eq!(p.copia.name_of(ORIGEM), "Origem");
        // gravada e lida de volta, a referência continua na origem
        let lida = Assembly::from_json(&p.copia.to_json()).unwrap();
        assert_eq!(lida.mates[0].a.comp, ORIGEM);
    }

    #[test]
    fn distancia_e_junta_medem_a_partir_da_origem() {
        let (a, _, m) = par();
        let mut c = Comando::novo(Tipo::Restricao(MateKind::Distance("0".into())));
        c.receber(face(ORIGEM, [0.; 3], [0., 0., 1.]));
        c.receber(face(m, [0.; 3], [0., 0., 1.]));
        let p = c.resolver(&a).unwrap();
        assert!(matches!(&p.copia.mates[0].kind, MateKind::Distance(v) if v == "30"));
        let mut c = Comando::novo(Tipo::Junta(JointKind::Revolute));
        c.receber(eixo(ORIGEM, [0.; 3], [0., 0., 1.]));
        c.receber(eixo(m, [0.; 3], [0., 0., 1.]));
        let p = c.resolver(&a).unwrap();
        let o = p.copia.node(m).unwrap().placement.origin;
        assert!(o[0].abs() < 1e-5 && o[1].abs() < 1e-5, "gira em volta do eixo Z da montagem: {o:?}");
    }

    fn peca(comp: Id, classe: Classe, geom: Geom) -> Elemento {
        Elemento::novo(MateRef { comp, geom, pick: None }, classe)
    }

    fn revolucao_entre_furos(f: Id, m: Id) -> Comando {
        let mut c = Comando::novo(Tipo::JuntaAutomatica);
        c.receber(eixo(f, [10., 0., 0.], [0., 0., 1.]));
        c.receber(eixo(m, [0., 0., 0.], [0., 0., 1.]));
        c
    }

    #[test]
    fn automatica_pela_geometria() {
        let (_, f, m) = par();
        let furo = |c| peca(c, Classe::Face, Geom::Axis { p: [0.; 3], d: [0., 0., 1.] });
        let reta = |c| peca(c, Classe::ArestaReta, Geom::Axis { p: [0.; 3], d: [1., 0., 0.] });
        let plana = |c| peca(c, Classe::Face, Geom::Plane { p: [0.; 3], n: [0., 0., 1.] });
        let ponto = |c| peca(c, Classe::Vertice, Geom::Point([0.; 3]));
        let mut curva = peca(f, Classe::ArestaCurva { circular: false }, Geom::Axis { p: [0.; 3], d: [0., 0., 1.] });
        curva.curva = vec![[0.; 3], [1., 1., 0.], [2., 0., 0.]];
        assert_eq!(junta_automatica(&furo(f), &furo(m)).0, JointKind::Revolute);
        assert_eq!(junta_automatica(&reta(f), &reta(m)).0, JointKind::Prismatic);
        assert_eq!(junta_automatica(&plana(f), &plana(m)).0, JointKind::Planar);
        assert_eq!(junta_automatica(&ponto(f), &ponto(m)).0, JointKind::Spherical);
        assert_eq!(junta_automatica(&ponto(f), &furo(m)).0, JointKind::Cylindrical);
        assert_eq!(junta_automatica(&furo(f), &plana(m)).0, JointKind::Revolute);
        assert_eq!(junta_automatica(&curva, &ponto(m)), (JointKind::PointCurve { curve: vec![] }, false));
        assert_eq!(junta_automatica(&ponto(m), &curva), (JointKind::PointCurve { curve: vec![] }, true));
    }

    #[test]
    fn revolucao_entre_dois_furos_sai_com_o_tipo_certo_e_anima() {
        let (a, f, m) = par();
        let c = revolucao_entre_furos(f, m);
        let p = c.resolver(&a).unwrap();
        assert_eq!(p.copia.joints[0].kind, JointKind::Revolute);
        assert_eq!(solve::dof(&p.copia), 1);
        let q = animar(&a, &p, &OpcoesAnimar::default()).unwrap();
        assert_eq!(q.len(), 72);
        // a peça gira em volta do eixo em (10, 0): o ponto dela fica na mesma distância, e o ângulo vai e volta
        let giro = |k: usize| q[k].iter().find(|x| x.comp == m).map(|x| x.depois.dir([1., 0., 0.])).unwrap();
        let ang = |v: V3| v[1].atan2(v[0]).to_degrees();
        assert!((ang(giro(18)) - 60.).abs() < 1e-3, "topo da ida: {}", ang(giro(18)));
        assert!((ang(giro(54)) + 60.).abs() < 1e-3, "topo da volta: {}", ang(giro(54)));
        assert!(ang(giro(1)).abs() < 12., "começa em zero");
        assert!(a.joints.is_empty() && p.copia.joints[0].drive_angle.is_none(), "animar não grava comando");
    }

    #[test]
    fn animar_inclui_os_limites_quando_pedido() {
        let (a, f, m) = par();
        let mut c = revolucao_entre_furos(f, m);
        c.limites.angulo = crate::kin::Faixa::nova(-10., 110.);
        let p = c.resolver(&a).unwrap();
        assert_eq!(p.copia.joints[0].limites.angulo, crate::kin::Faixa::nova(-10., 110.));
        let ang = |q: &Vec<Vec<Movido>>, k: usize| {
            let v = q[k].iter().find(|x| x.comp == m).unwrap().depois.dir([1., 0., 0.]);
            v[1].atan2(v[0]).to_degrees()
        };
        let q = animar(&a, &p, &OpcoesAnimar::default()).unwrap();
        assert!((ang(&q, 18) - 110.).abs() < 1e-3);
        assert!((ang(&q, 54) + 10.).abs() < 1e-3);
        let q = animar(&a, &p, &OpcoesAnimar { limites: false, ..Default::default() }).unwrap();
        assert!((ang(&q, 18) - 60.).abs() < 1e-3, "sem a opção vale a faixa padrão");
    }

    #[test]
    fn prismatica_anima_curso_e_rigida_recusa() {
        let (a, f, m) = par();
        let mut c = Comando::novo(Tipo::Junta(JointKind::Prismatic));
        c.receber(eixo(f, [0.; 3], [0., 0., 1.]));
        c.receber(eixo(m, [0.; 3], [0., 0., 1.]));
        let p = c.resolver(&a).unwrap();
        let q = animar(&a, &p, &OpcoesAnimar::default()).unwrap();
        let z = q[18].iter().find(|x| x.comp == m).unwrap().depois.origin[2];
        assert!((z - 60.).abs() < 1e-4, "30 de altura mais 30 de curso: {z}");
        c.trocar_tipo(Tipo::Junta(JointKind::Rigid));
        let p = c.resolver(&a).unwrap();
        assert!(animar(&a, &p, &OpcoesAnimar::default()).is_err());
    }

    #[test]
    fn esferica_e_planar_animam_pela_puxada() {
        let (a, f, m) = par();
        let mut c = Comando::novo(Tipo::Junta(JointKind::Spherical));
        c.receber(peca(f, Classe::Vertice, Geom::Point([0., 0., 10.])));
        c.receber(peca(m, Classe::Vertice, Geom::Point([0., 0., 0.])));
        let p = c.resolver(&a).unwrap();
        let q = animar(&a, &p, &OpcoesAnimar::default()).unwrap();
        assert!(q.iter().flatten().any(|x| x.comp == m && norm(sub(x.depois.dir([0., 0., 1.]), [0., 0., 1.])) > 0.5), "a peça gira");
        let mut c = Comando::novo(Tipo::Junta(JointKind::Planar));
        c.receber(face(f, [0., 0., 10.], [0., 0., 1.]));
        c.receber(face(m, [0.; 3], [0., 0., 1.]));
        let p = c.resolver(&a).unwrap();
        let q = animar(&a, &p, &OpcoesAnimar::default()).unwrap();
        let repouso = p.copia.node(m).unwrap().placement.origin;
        assert!(q.iter().flatten().any(|x| x.comp == m && norm(sub(x.depois.origin, repouso)) > 5.), "a peça anda no plano");
        assert!(q.iter().flatten().all(|x| x.comp != m || (x.depois.origin[2] - repouso[2]).abs() < 1e-5), "sem sair dele");
    }

    #[test]
    fn origem_entre_duas_faces_fica_no_meio() {
        let (a, f, m) = par();
        let mut c = Comando::novo(Tipo::Junta(JointKind::Revolute));
        c.trocar_modo_origem(0, ModoOrigem::EntreFaces);
        assert_eq!(c.receber(face(f, [0., 0., 0.], [0., 0., 1.])), 0);
        assert!(c.pede_segundo(0) && c.ativa == 0, "a mesma caixa pede a segunda face");
        assert!(!c.completo());
        assert_eq!(c.receber(face(f, [3., 0., 20.], [0., 0., -1.])), 0);
        assert!(c.cheia(0) && c.ativa == 1);
        c.receber(eixo(m, [0.; 3], [0., 0., 1.]));
        let p = c.resolver(&a).unwrap();
        assert_eq!(p.copia.joints[0].a.frame.o, [0., 0., 10.], "meio dos planos");
    }

    #[test]
    fn origem_recusa_faces_nao_paralelas_e_peca_diferente() {
        let (a, f, m) = par();
        let mut c = Comando::novo(Tipo::Junta(JointKind::Revolute));
        c.trocar_modo_origem(0, ModoOrigem::EntreFaces);
        c.receber(face(f, [0.; 3], [0., 0., 1.]));
        c.receber(face(f, [0.; 3], [1., 0., 0.]));
        c.receber(eixo(m, [0.; 3], [0., 0., 1.]));
        assert!(c.resolver(&a).unwrap_err().contains("paralelas"));
        c.limpar_caixa(0);
        c.receber(face(f, [0.; 3], [0., 0., 1.]));
        c.receber(face(m, [0.; 3], [0., 0., 1.]));
        assert!(c.resolver(&a).unwrap_err().contains("mesma peça"));
    }

    #[test]
    fn origem_na_intersecao_de_duas_arestas() {
        let (a, f, m) = par();
        let aresta = |c, p, d| peca(c, Classe::ArestaReta, Geom::Axis { p, d });
        let mut c = Comando::novo(Tipo::Junta(JointKind::Revolute));
        c.trocar_modo_origem(0, ModoOrigem::IntersecaoArestas);
        c.receber(aresta(f, [0., 5., 0.], [1., 0., 0.]));
        c.receber(aresta(f, [7., 0., 0.], [0., 1., 0.]));
        c.receber(eixo(m, [0.; 3], [0., 0., 1.]));
        let p = c.resolver(&a).unwrap();
        let fr = p.copia.joints[0].a.frame;
        assert!(norm(sub(fr.o, [7., 5., 0.])) < 1e-9, "{:?}", fr.o);
        assert!(norm(cross(fr.z, [0., 0., 1.])) < 1e-9, "o eixo sai do plano das arestas");
        // arestas reversas não têm interseção
        c.limpar_caixa(0);
        c.receber(aresta(f, [0., 5., 0.], [1., 0., 0.]));
        c.receber(aresta(f, [7., 0., 3.], [0., 1., 0.]));
        assert!(c.resolver(&a).unwrap_err().contains("não se cruzam"));
    }

    #[test]
    fn deslocamentos_e_angulo_do_alinhamento() {
        let (a, f, m) = par();
        let mut c = Comando::novo(Tipo::Junta(JointKind::Revolute));
        c.receber(eixo(f, [0.; 3], [0., 0., 1.]));
        c.receber(eixo(m, [0.; 3], [0., 0., 1.]));
        let base = c.resolver(&a).unwrap().copia.joints[0].a.frame;
        assert!(norm(base.x) > 0.);
        c.alinhamento.desloc = [5., 0., 2.];
        let p = c.resolver(&a).unwrap();
        let o = p.copia.node(m).unwrap().placement.origin;
        // a peça gira na junta: o deslocamento em X fixa o raio, e o Z sobe no eixo
        assert!((o[0].hypot(o[1]) - 5.).abs() < 1e-5, "{o:?}");
        assert!((o[2] - 32.).abs() < 1e-5, "{o:?}");
        assert!(!p.falhou());
        c.alinhamento = Alinhamento { angulo: 30., ..Default::default() };
        let p = c.resolver(&a).unwrap();
        let x = p.copia.node(m).unwrap().placement.dir([1., 0., 0.]);
        assert!((dot(x, [1., 0., 0.]) - 30f64.to_radians().cos()).abs() < 1e-5);
    }

    #[test]
    fn deslocamento_em_z_da_esferica_vai_para_o_sistema() {
        let (a, f, m) = par();
        let mut c = Comando::novo(Tipo::Junta(JointKind::Spherical));
        c.receber(peca(f, Classe::Vertice, Geom::Point([0.; 3])));
        c.receber(peca(m, Classe::Vertice, Geom::Point([0.; 3])));
        c.alinhamento.desloc = [0., 0., 7.];
        let p = c.resolver(&a).unwrap();
        assert!((norm(p.copia.node(m).unwrap().placement.origin) - 7.).abs() < 1e-5, "o ponto fica a 7 da rótula");
    }

    #[test]
    fn inverter_vira_a_peca_na_junta() {
        let (a, f, m) = par();
        let mut c = Comando::novo(Tipo::Junta(JointKind::Revolute));
        c.receber(eixo(f, [0.; 3], [0., 0., 1.]));
        c.receber(eixo(m, [0.; 3], [0., 0., 1.]));
        let z = |p: &Previa| p.copia.node(m).unwrap().placement.dir([0., 0., 1.])[2];
        let p0 = c.resolver(&a).unwrap();
        assert!((z(&p0) - 1.).abs() < 1e-6);
        assert!(c.inverter(Some(&p0)), "Tab também vale na junta");
        let p1 = c.resolver(&a).unwrap();
        assert!((z(&p1) + 1.).abs() < 1e-6, "o eixo aponta ao contrário");
        assert!(!p1.falhou());
    }

    #[test]
    fn inverter_com_b_presa_vira_a_peca_a() {
        let (mut a, f, m) = par();
        a.node_mut(f).unwrap().fixed = false;
        a.node_mut(m).unwrap().fixed = true;
        let mut c = Comando::novo(Tipo::Junta(JointKind::Revolute));
        c.receber(eixo(f, [0.; 3], [0., 0., 1.]));
        c.receber(eixo(m, [0.; 3], [0., 0., 1.]));
        c.alinhamento.inverter = true;
        let p = c.resolver(&a).unwrap();
        assert!((p.copia.node(f).unwrap().placement.dir([0., 0., 1.])[2] + 1.).abs() < 1e-6);
        assert_eq!(p.copia.node(m).unwrap().placement, a.node(m).unwrap().placement);
    }

    fn aresta_curva(comp: Id) -> Elemento {
        let mut e = peca(comp, Classe::ArestaCurva { circular: false }, Geom::Axis { p: [0.; 3], d: [0., 0., 1.] });
        e.curva = vec![[0., 0., 0.], [20., 0., 0.], [40., 10., 0.]];
        e.perfil = Some(vec![[0., 0., 0.], [40., 0., 0.], [40., 40., 0.], [0., 40., 0.]]);
        e
    }

    #[test]
    fn trajetoria_e_seguidor_aceitam_qualquer_ordem() {
        let (a, f, m) = par();
        let vertice = peca(m, Classe::Vertice, Geom::Point([0.; 3]));
        for ordem in [false, true] {
            let mut c = Comando::novo(Tipo::Junta(JointKind::PointCurve { curve: vec![] }));
            assert_eq!(c.rotulos(), ["Trajetória", "Seguidor"]);
            if ordem {
                assert_eq!(c.receber(vertice.clone()), 1);
                assert_eq!(c.ativa, 0);
                assert_eq!(c.receber(aresta_curva(f)), 0);
            } else {
                assert_eq!(c.receber(aresta_curva(f)), 0);
                assert_eq!(c.receber(vertice.clone()), 1);
            }
            let p = c.resolver(&a).unwrap();
            assert!(matches!(&p.copia.joints[0].kind, JointKind::PointCurve { curve } if curve.len() == 3));
            assert_eq!(p.copia.joints[0].a.comp, f);
        }
        let mut c = Comando::novo(Tipo::Junta(JointKind::Cam { profile: vec![], radius: "0".into() }));
        let mut rolete = peca(m, Classe::ArestaCurva { circular: true }, Geom::Axis { p: [0.; 3], d: [0., 0., 1.] });
        rolete.raio = Some(4.);
        assert_eq!(c.receber(rolete), 1, "círculo: rolete");
        assert_eq!(c.receber(aresta_curva(f)), 0);
        let p = c.resolver(&a).unwrap();
        assert!(matches!(&p.copia.joints[0].kind, JointKind::Cam { radius, .. } if radius == "4.000"));
    }

    #[test]
    fn trocar_as_caixas_do_came() {
        let (_, f, m) = par();
        let mut c = Comando::novo(Tipo::Junta(JointKind::Cam { profile: vec![], radius: "0".into() }));
        c.caixas = [Some(eixo(f, [0.; 3], [0., 0., 1.])), Some(eixo(m, [0.; 3], [0., 0., 1.]))];
        c.trocar_caixas();
        assert_eq!(c.caixas[0].as_ref().unwrap().r.comp, m);
    }

    #[test]
    fn came_anima_pela_junta_do_came() {
        let (mut a, f, m) = par();
        // o came (m) gira na base; o seguidor desliza em cima dele
        let seg = a.add_component("seguidor", NodeKind::Part { path: "c.fkp".into() }, Placement { origin: [20., 0., 30.], ..Default::default() }, None);
        let fr = JFrame::from_axis([0.; 3], [0., 0., 1.]);
        a.add_joint(JointKind::Revolute, JRef { comp: f, frame: JFrame::from_axis([0., 0., 30.], [0., 0., 1.]) }, JRef { comp: m, frame: JFrame::from_axis([-3., -4., 0.], [0., 0., 1.]) });
        a.add_joint(JointKind::Prismatic, JRef { comp: f, frame: JFrame::from_axis([20., 0., 30.], [1., 0., 0.]) }, JRef { comp: seg, frame: JFrame::from_axis([0.; 3], [1., 0., 0.]) });
        let _ = fr;
        let quadrado: Vec<V3> = vec![[17., 16., 0.], [-23., 16., 0.], [-23., -24., 0.], [17., -24., 0.]];
        let mut came = peca(m, Classe::ArestaCurva { circular: false }, Geom::Axis { p: [-3., -4., 0.], d: [0., 0., 1.] });
        came.perfil = Some(quadrado);
        let mut c = Comando::novo(Tipo::Junta(JointKind::Cam { profile: vec![], radius: "0".into() }));
        c.receber(came);
        c.receber(peca(seg, Classe::Vertice, Geom::Point([0.; 3])));
        let p = c.resolver(&a).unwrap();
        let q = animar(&a, &p, &OpcoesAnimar::default()).unwrap();
        let x: Vec<f64> = q.iter().filter_map(|m| m.iter().find(|i| i.comp == seg)).map(|i| i.depois.origin[0]).collect();
        let (lo, hi) = (x.iter().copied().fold(f64::MAX, f64::min), x.iter().copied().fold(f64::MIN, f64::max));
        assert!(hi - lo > 1., "o seguidor sobe e desce com o came: {lo} {hi}");
        let sem = Comando { caixas: c.caixas.clone(), ..c.clone() };
        let mut b = a.clone();
        b.joints.clear();
        assert!(animar(&b, &sem.resolver(&b).unwrap(), &OpcoesAnimar::default()).is_err(), "sem junta no came não há o que girar");
    }

    #[test]
    fn junta_entre_eixos_paralelos_nao_gira_a_peca_a_toa() {
        let (a, f, m) = par();
        let c = revolucao_entre_furos(f, m);
        let p = c.resolver(&a).unwrap();
        let pl = p.copia.node(m).unwrap().placement;
        assert!(norm(sub(pl.dir([1., 0., 0.]), [1., 0., 0.])) < 1e-9, "só translada");
        assert!(norm(sub(pl.origin, [10., 0., 30.])) < 1e-6);
        let mut c = c;
        c.alinhamento.desloc = [5., 0., 0.];
        let p = c.resolver(&a).unwrap();
        let x = p.copia.joints[0].a.frame.x;
        let o = p.copia.node(m).unwrap().placement.origin;
        assert!(norm(sub(o, [10. + 5. * x[0], 5. * x[1], 30.])) < 1e-6, "vai para o raio pedido, na direção do X da junta: {o:?}");
    }

    #[test]
    fn nome_e_suprimir_do_avancado() {
        let (a, f, m) = par();
        let mut c = contato(f, m);
        c.nome = " Tampa fechada ".into();
        c.suprimida = true;
        let p = c.resolver(&a).unwrap();
        assert_eq!(p.copia.mates[0].name, "Tampa fechada");
        assert!(p.copia.mates[0].suppressed);
        assert!(p.movidos.is_empty(), "suprimida não move");
    }
}
