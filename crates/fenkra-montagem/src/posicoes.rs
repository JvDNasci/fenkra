// SPDX-License-Identifier: LGPL-2.1-or-later
//! Posições nomeadas da montagem (B123): "aberta", "fechada", "meio curso".
//!
//! Uma posição guarda o valor de cada grau de liberdade das juntas e a cota de cada restrição de
//! distância ou ângulo, mais o lugar de cada componente no momento em que foi gravada. Os valores são
//! a posição de verdade; os lugares só servem de partida para o solucionador, para que ele volte ao
//! mesmo ramo do mecanismo (uma biela tem duas montagens possíveis para o mesmo giro da manivela).
//!
//! Voltar a uma posição põe os valores como comandos provisórios e resolve. O comando gravado na
//! junta volta depois, exceto quando ele era um número solto: aí a posição troca o número, como faz
//! com a cota de uma restrição.
//!
//! As posições formam uma lista com ordem. Cada uma tem um tempo de transição, o tempo para chegar a
//! ela saindo da anterior. Percorrer a lista gera quadros no tempo, com os valores interpolados e
//! suavizados no começo e no fim de cada trecho; o mesmo percurso serve para tocar, para o cursor de
//! tempo e para exportar a animação.

use crate::kin::{medir_grau, Frame, Grau, JointKind};
use crate::*;

/// Valor de um grau de liberdade de uma junta, na referência dos limites (graus ou mm).
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct ValorJunta {
    pub junta: Id,
    pub grau: Grau,
    pub valor: f64,
}

/// Cota de uma restrição de distância ou ângulo, como texto (aceita expressão).
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct ValorRestricao {
    pub restricao: Id,
    pub valor: String,
}

/// Uma posição nomeada.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Posicao {
    pub nome: String,
    #[serde(default)]
    pub juntas: Vec<ValorJunta>,
    #[serde(default)]
    pub restricoes: Vec<ValorRestricao>,
    /// lugar de cada componente ao gravar: só a partida do solucionador
    #[serde(default)]
    pub lugares: Vec<(Id, Placement)>,
    /// segundos para chegar a esta posição saindo da anterior da lista
    #[serde(default = "transicao_padrao")]
    pub transicao: f64,
}

fn transicao_padrao() -> f64 {
    TRANSICAO
}

/// Tempo de transição de uma posição nova, em segundos.
pub const TRANSICAO: f64 = 2.;

/// Os graus que uma posição guarda: os que têm medida e caminho até o valor. A inclinação da
/// esférica não tem caminho único, então fica com o lugar gravado.
fn graus_guardados(k: &JointKind) -> impl Iterator<Item = Grau> + '_ {
    k.graus().iter().copied().filter(|g| *g != Grau::Inclinacao)
}

/// O comando é um número solto (e não uma lei em `t` nem uma fórmula com parâmetro)?
fn numero_solto(e: &str) -> bool {
    e.trim().replace(',', ".").parse::<f64>().is_ok()
}

/// Diferença angular pelo caminho mais curto, em graus.
fn giro_curto(de: f64, para: f64) -> f64 {
    let mut d = (para - de) % 360.;
    if d > 180. {
        d -= 360.;
    }
    if d <= -180. {
        d += 360.;
    }
    d
}

/// Suavização do trecho: parte e chega devagar, passa pelo meio na metade do tempo.
pub fn suave(u: f64) -> f64 {
    let u = u.clamp(0., 1.);
    u * u * (3. - 2. * u)
}

impl Assembly {
    /// Posição com a situação de agora: valores das juntas, cotas das restrições e os lugares.
    pub fn gravar_posicao(&self, nome: &str) -> Posicao {
        let mut juntas = vec![];
        for j in self.joints.iter().filter(|j| !j.suppressed) {
            let (Some(pa), Some(pb)) = (self.lugar(j.a.comp), self.lugar(j.b.comp)) else { continue };
            for g in graus_guardados(&j.kind) {
                juntas.push(ValorJunta { junta: j.id, grau: g, valor: medir_grau(j, g, &pa, &pb) });
            }
        }
        let restricoes = self
            .mates
            .iter()
            .filter(|m| !m.suppressed)
            .filter_map(|m| match &m.kind {
                MateKind::Distance(e) | MateKind::Angle(e) => Some(ValorRestricao { restricao: m.id, valor: e.clone() }),
                _ => None,
            })
            .collect();
        let lugares = self.nodes.iter().filter(|n| !matches!(n.kind, NodeKind::Group)).map(|n| (n.id, n.placement)).collect();
        Posicao { nome: nome.to_string(), juntas, restricoes, lugares, transicao: TRANSICAO }
    }

    /// Grava a situação de agora como posição nova, no fim da lista. Nome vazio ganha "Posição N".
    pub fn nova_posicao(&mut self, nome: &str) -> usize {
        let nome = if nome.trim().is_empty() { format!("Posição {}", self.posicoes.len() + 1) } else { nome.trim().to_string() };
        let p = self.gravar_posicao(&nome);
        self.posicoes.push(p);
        self.posicoes.len() - 1
    }

    /// Atualizar: a posição `i` passa a guardar a situação de agora, com o mesmo nome e transição.
    pub fn atualizar_posicao(&mut self, i: usize) -> bool {
        let Some(p) = self.posicoes.get(i) else { return false };
        let (nome, transicao) = (p.nome.clone(), p.transicao);
        self.posicoes[i] = Posicao { transicao, ..self.gravar_posicao(&nome) };
        true
    }

    /// Reordenar: leva a posição `de` para o lugar `para` da lista.
    pub fn mover_posicao(&mut self, de: usize, para: usize) -> bool {
        if de >= self.posicoes.len() || para >= self.posicoes.len() || de == para {
            return false;
        }
        let p = self.posicoes.remove(de);
        self.posicoes.insert(para, p);
        true
    }
}

/// Põe a montagem na posição `p` e resolve. Juntas e restrições que não existem mais são ignoradas.
pub fn aplicar(a: &mut Assembly, p: &Posicao) -> crate::solve::SolveReport {
    // partida: os lugares de quando a posição foi gravada (peça fixa não sai do lugar)
    for (id, pl) in &p.lugares {
        if let Some(n) = a.node_mut(*id).filter(|n| !n.fixed) {
            n.placement = *pl;
        }
    }
    for v in &p.restricoes {
        if let Some(m) = a.mates.iter_mut().find(|m| m.id == v.restricao) {
            if let MateKind::Distance(e) | MateKind::Angle(e) = &mut m.kind {
                *e = v.valor.clone();
            }
        }
    }
    // comandos provisórios: o de antes volta depois de resolver, salvo o número solto, que a
    // posição troca (como a cota de uma restrição)
    let mut devolver = vec![];
    for v in &p.juntas {
        let Some(j) = a.joints.iter_mut().find(|j| j.id == v.junta && !j.suppressed) else { continue };
        let texto = format!("{}", v.valor);
        let campo = match v.grau {
            Grau::Giro => &mut j.drive_angle,
            Grau::Curso => &mut j.drive_len,
            // posição no plano e na trajetória não têm comando: ficam com o lugar gravado
            _ => continue,
        };
        let antes = campo.replace(texto);
        if !antes.as_deref().is_some_and(numero_solto) {
            devolver.push((v.junta, v.grau, antes));
        }
    }
    let rep = crate::solve::solve(a);
    for (id, g, antes) in devolver {
        if let Some(j) = a.joints.iter_mut().find(|j| j.id == id) {
            match g {
                Grau::Giro => j.drive_angle = antes,
                _ => j.drive_len = antes,
            }
        }
    }
    rep
}

/// Posição no meio do caminho de `a` até `b` (`u` de 0 a 1, já suavizado). Giro vai pelo caminho
/// mais curto; cota que não é número troca na metade. Sem lugares: a partida é o quadro anterior.
pub fn entre(a: &Posicao, b: &Posicao, u: f64) -> Posicao {
    let juntas = b
        .juntas
        .iter()
        .map(|vb| {
            let va = a.juntas.iter().find(|x| x.junta == vb.junta && x.grau == vb.grau).map_or(vb.valor, |x| x.valor);
            let d = if vb.grau == Grau::Giro { giro_curto(va, vb.valor) } else { vb.valor - va };
            ValorJunta { valor: va + d * u, ..*vb }
        })
        .collect();
    let num = |s: &str| s.trim().replace(',', ".").parse::<f64>().ok();
    let restricoes = b
        .restricoes
        .iter()
        .map(|rb| {
            let ra = a.restricoes.iter().find(|x| x.restricao == rb.restricao);
            let valor = match (ra.and_then(|r| num(&r.valor)), num(&rb.valor)) {
                (Some(x), Some(y)) => format!("{}", x + (y - x) * u),
                _ if u < 0.5 => ra.map_or(rb.valor.clone(), |r| r.valor.clone()),
                _ => rb.valor.clone(),
            };
            ValorRestricao { restricao: rb.restricao, valor }
        })
        .collect();
    Posicao { nome: String::new(), juntas, restricoes, lugares: vec![], transicao: b.transicao }
}

/// Duração do percurso pela lista: a soma das transições, menos a da primeira (ela é o começo).
pub fn duracao(lista: &[Posicao]) -> f64 {
    lista.iter().skip(1).map(|p| p.transicao.max(0.)).sum()
}

/// Em que trecho o tempo `t` cai: índice da posição de chegada e a fração andada (sem suavizar).
pub fn trecho(lista: &[Posicao], t: f64) -> Option<(usize, f64)> {
    if lista.len() < 2 {
        return (!lista.is_empty()).then_some((0, 1.));
    }
    let mut ini = 0.;
    for (k, p) in lista.iter().enumerate().skip(1) {
        let d = p.transicao.max(0.);
        if t < ini + d || k + 1 == lista.len() {
            let u = if d <= 0. { 1. } else { ((t - ini) / d).clamp(0., 1.) };
            return Some((k, u));
        }
        ini += d;
    }
    None
}

/// Posição no tempo `t` do percurso.
pub fn no_tempo(lista: &[Posicao], t: f64) -> Option<Posicao> {
    let (k, u) = trecho(lista, t)?;
    if k == 0 {
        return Some(lista[0].clone());
    }
    Some(entre(&lista[k - 1], &lista[k], suave(u)))
}

/// Quadros do percurso pela lista, `por_segundo` quadros a cada segundo. Cada quadro parte do
/// anterior, então o mecanismo segue o mesmo ramo. A montagem volta como estava.
pub fn percorrer(a: &mut Assembly, lista: &[Posicao], por_segundo: f64) -> Vec<Frame> {
    let Some(primeira) = lista.first() else { return vec![] };
    let keep = a.clone();
    let total = duracao(lista);
    let n = ((total * por_segundo.max(1e-3)).ceil() as usize).max(1);
    let mut out = Vec::with_capacity(n + 1);
    let rep = aplicar(a, primeira);
    out.push(Frame { t: 0., placements: a.nodes.iter().map(|n| (n.id, n.placement)).collect(), ok: rep.converged });
    for k in 1..=n {
        let t = total * k as f64 / n as f64;
        let Some(p) = no_tempo(lista, t) else { break };
        let rep = aplicar(a, &p);
        out.push(Frame { t, placements: a.nodes.iter().map(|n| (n.id, n.placement)).collect(), ok: rep.converged });
    }
    *a = keep;
    out
}

/// Ângulos de Euler (graus, ordem Z-Y-X: guinada, arfagem, rolagem) da rotação do lugar. Cada linha
/// de `rot` é a imagem de um eixo (ver [`Placement::dir`]), então a matriz de colunas é a transposta.
fn euler_zyx(rot: &[[f64; 3]; 3]) -> V3 {
    let r = [0, 1, 2].map(|i| [0, 1, 2].map(|j| rot[j][i]));
    let arf = (-r[2][0]).clamp(-1., 1.).asin();
    let (gui, rol) = if r[2][0].abs() < 1. - 1e-12 { (r[1][0].atan2(r[0][0]), r[2][1].atan2(r[2][2])) } else { ((-r[0][1]).atan2(r[1][1]), 0.) };
    [gui.to_degrees(), arf.to_degrees(), rol.to_degrees()]
}

/// Tabela da animação para planilha ou outro programa: uma linha por quadro e componente, com o
/// tempo, a origem (mm) e os ângulos de Euler Z-Y-X (graus). Separador ponto e vírgula.
pub fn tabela(a: &Assembly, quadros: &[Frame]) -> String {
    let mut s = String::from("t_s;componente;x_mm;y_mm;z_mm;rz_graus;ry_graus;rx_graus;resolvido\n");
    let comps: Vec<&Node> = a.nodes.iter().filter(|n| !matches!(n.kind, NodeKind::Group)).collect();
    for q in quadros {
        for n in &comps {
            let Some(p) = q.placements.get(&n.id) else { continue };
            let e = euler_zyx(&p.rot);
            let f = |x: f64| format!("{:.4}", if x.abs() < 5e-5 { 0. } else { x });
            s += &format!("{};{};{};{};{};{};{};{};{}\n", f(q.t), n.name.replace(';', ","), f(p.origin[0]), f(p.origin[1]), f(p.origin[2]), f(e[0]), f(e[1]), f(e[2]), if q.ok { "sim" } else { "não" });
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kin::{JFrame, JRef};
    use crate::load::PartCache;
    use crate::view::compor;
    use fenkra_core::{BoolOp, Document, Extent, Extrude, FeatureKind, OriginPlane, PlaneRef, Sketch, SketchData};
    use std::path::{Path, PathBuf};

    fn pasta(nome: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("fenkra_posicoes_{nome}"));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    /// Barra de 100 × 10 × 10 que sai da origem ao longo de X.
    fn barra() -> Document {
        let mut d = Document::new("Barra");
        let mut n = 8_300_000u64;
        let mut al = || {
            n += 1;
            Id(n)
        };
        let mut s = SketchData::default();
        s.add_rect(&mut al, [0., -5.], [100., 5.], false);
        let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data: s }));
        d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind("10".into()), reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
        d
    }

    /// Base fixa, braço numa revolução em Z pela origem e um bloco que corre no braço (prismática
    /// em X), com a distância do bloco à ponta dada por uma restrição de distância do ponto do bloco ao plano da ponta.
    fn braco(dir: &Path) -> (Assembly, Id, Id, Id) {
        std::fs::write(dir.join("barra.fkp"), barra().to_json()).unwrap();
        let mut a = Assembly::new("Braço");
        let base = a.add_component("Base", NodeKind::Part { path: "barra.fkp".into() }, Placement { origin: [0., 0., -10.], ..Default::default() }, None);
        a.node_mut(base).unwrap().fixed = true;
        let br = a.add_component("Braço", NodeKind::Part { path: "barra.fkp".into() }, Placement::default(), None);
        let bloco = a.add_component("Bloco", NodeKind::Part { path: "barra.fkp".into() }, Placement { origin: [0., 0., 10.], ..Default::default() }, None);
        let giro = a.add_joint(JointKind::Revolute, JRef { comp: base, frame: JFrame::from_axis([0., 0., 10.], [0., 0., 1.]) }, JRef { comp: br, frame: JFrame::from_axis([0.; 3], [0., 0., 1.]) });
        a.add_joint(JointKind::Prismatic, JRef { comp: br, frame: JFrame::from_axis([0., 0., 10.], [1., 0., 0.]) }, JRef { comp: bloco, frame: JFrame::from_axis([0.; 3], [1., 0., 0.]) });
        let r = |c: Id, geom: Geom| MateRef { comp: c, geom, pick: None, viva: None };
        let d = a.add_mate(MateKind::Distance("0".into()), r(br, Geom::Plane { p: [0., 0., 10.], n: [1., 0., 0.] }), r(bloco, Geom::Point([0.; 3])));
        assert!(crate::solve::solve(&mut a).converged);
        (a, giro, d, bloco)
    }

    fn caixas(a: &Assembly, dir: &Path, cache: &mut PartCache) -> Vec<([f32; 3], [f32; 3])> {
        compor(a, Some(dir), cache, &mut vec![], None).unwrap().caixas
    }

    fn igual(x: &[([f32; 3], [f32; 3])], y: &[([f32; 3], [f32; 3])]) -> bool {
        x.len() == y.len() && x.iter().zip(y).all(|(p, q)| (0..3).all(|k| (p.0[k] - q.0[k]).abs() < 1e-2 && (p.1[k] - q.1[k]).abs() < 1e-2))
    }

    fn cota(a: &mut Assembly, m: Id, v: &str) {
        if let Some(MateKind::Distance(e)) = a.mates.iter_mut().find(|x| x.id == m).map(|x| &mut x.kind) {
            *e = v.into();
        }
    }

    #[test]
    fn gravar_e_voltar_deixa_status_e_caixa_iguais() {
        let dir = pasta("voltar");
        let (mut a, giro, d, bloco) = braco(&dir);
        let mut cache = PartCache::default();
        let geo = fenkra_kernel::AVAILABLE;

        // fechada: braço em X, bloco na base do braço
        let fechada = a.nova_posicao("Fechada");
        let caixa_fechada = if geo { caixas(&a, &dir, &mut cache) } else { vec![] };

        // aberta: braço a 90° e bloco 60 mm para fora
        cota(&mut a, d, "60");
        let rep = crate::kin::levar_a(&mut a, giro, Grau::Giro, 90.);
        assert!(rep.converged && rep.failed.is_empty());
        let aberta = a.nova_posicao("Aberta");
        let o = a.node(bloco).unwrap().placement.origin;
        assert!(o[0].abs() < 1e-4 && (o[1] - 60.).abs() < 1e-4, "bloco 60 mm em Y: {o:?}");
        let caixa_aberta = if geo { caixas(&a, &dir, &mut cache) } else { vec![] };
        if geo {
            assert!(!igual(&caixa_fechada, &caixa_aberta), "as duas posições são diferentes");
        }

        // a lista vai no arquivo da montagem
        let mut a = Assembly::from_json(&a.to_json()).unwrap();
        assert_eq!(a.posicoes.iter().map(|p| p.nome.as_str()).collect::<Vec<_>>(), ["Fechada", "Aberta"]);

        // de volta à fechada: o status e a caixa são os de quando foi gravada
        let p = a.posicoes[fechada].clone();
        let rep = aplicar(&mut a, &p);
        assert!(rep.converged && rep.failed.is_empty(), "{rep:?}");
        assert!(a.joints.iter().all(|j| j.drive_angle.is_none() && j.drive_len.is_none()), "a junta segue livre depois");
        assert_eq!(a.mates.iter().find(|m| m.id == d).unwrap().kind, MateKind::Distance("0".into()));
        let o = a.node(bloco).unwrap().placement.origin;
        assert!(o[0].abs() < 1e-4 && o[1].abs() < 1e-4, "bloco de volta: {o:?}");
        if geo {
            assert!(igual(&caixas(&a, &dir, &mut cache), &caixa_fechada));
        }
        // e a aberta também, partindo de outro lugar
        let p = a.posicoes[aberta].clone();
        let rep = aplicar(&mut a, &p);
        assert!(rep.converged && rep.failed.is_empty());
        if geo {
            assert!(igual(&caixas(&a, &dir, &mut cache), &caixa_aberta));
        }
    }

    #[test]
    fn atualizar_e_reordenar() {
        let dir = pasta("ordem");
        let (mut a, giro, _, _) = braco(&dir);
        a.nova_posicao("A");
        a.nova_posicao("");
        a.nova_posicao("C");
        assert_eq!(a.posicoes[1].nome, "Posição 2");
        a.posicoes[1].transicao = 5.;
        crate::kin::levar_a(&mut a, giro, Grau::Giro, 45.);
        assert!(a.atualizar_posicao(1));
        let v = a.posicoes[1].juntas.iter().find(|v| v.junta == giro).unwrap().valor;
        assert!((v - 45.).abs() < 1e-6, "atualizada com o giro de agora: {v}");
        assert_eq!((a.posicoes[1].nome.as_str(), a.posicoes[1].transicao), ("Posição 2", 5.), "nome e transição ficam");
        assert!(a.mover_posicao(2, 0));
        assert_eq!(a.posicoes.iter().map(|p| p.nome.as_str()).collect::<Vec<_>>(), ["C", "A", "Posição 2"]);
        assert!(!a.mover_posicao(0, 9));
    }

    #[test]
    fn comando_numerico_e_trocado_e_lei_no_tempo_volta() {
        let dir = pasta("comandos");
        let (mut a, giro, _, _) = braco(&dir);
        a.joints.iter_mut().find(|j| j.id == giro).unwrap().drive_angle = Some("30".into());
        crate::solve::solve(&mut a);
        let p30 = a.gravar_posicao("30");
        a.joints.iter_mut().find(|j| j.id == giro).unwrap().drive_angle = Some("10*t".into());
        let p = Posicao { juntas: p30.juntas.iter().map(|v| if v.junta == giro { ValorJunta { valor: 70., ..*v } } else { *v }).collect(), ..p30.clone() };
        aplicar(&mut a, &p);
        assert_eq!(a.joints.iter().find(|j| j.id == giro).unwrap().drive_angle.as_deref(), Some("10*t"), "a lei no tempo é da junta");
        a.joints.iter_mut().find(|j| j.id == giro).unwrap().drive_angle = Some("5".into());
        aplicar(&mut a, &p);
        assert_eq!(a.joints.iter().find(|j| j.id == giro).unwrap().drive_angle.as_deref(), Some("70"), "número solto vira o da posição");
    }

    #[test]
    fn percurso_segue_a_conta_feita_a_mao() {
        let dir = pasta("percurso");
        let (mut a, giro, d, bloco) = braco(&dir);
        a.nova_posicao("Fechada");
        cota(&mut a, d, "60");
        crate::kin::levar_a(&mut a, giro, Grau::Giro, 90.);
        a.nova_posicao("Aberta");
        a.posicoes[1].transicao = 4.;
        a.nova_posicao("Fechada de novo");
        a.posicoes[2] = Posicao { nome: "Fechada de novo".into(), transicao: 2., ..a.posicoes[0].clone() };
        let lista = a.posicoes.clone();
        assert_eq!(duracao(&lista), 6.);
        let antes = a.clone();
        let q = percorrer(&mut a, &lista, 10.);
        assert_eq!(a, antes, "a montagem volta como estava");
        assert_eq!(q.len(), 61);
        assert!(q.iter().all(|f| f.ok));
        // à mão: no tempo t do primeiro trecho, u = t/4 e s = u²(3 − 2u); o braço está a 90·s graus
        // e o bloco a 60·s mm do pivô, então ele fica em (60s·cos(90s°), 60s·sen(90s°))
        for (t, s) in [(1., 0.15625), (2., 0.5), (3., 0.84375), (4., 1.)] {
            let f = q.iter().find(|f| (f.t - t).abs() < 1e-9).unwrap();
            let o = f.placements[&bloco].origin;
            let (r, ang) = (60. * s, (90. * s as f64).to_radians());
            assert!((o[0] - r * ang.cos()).abs() < 1e-3 && (o[1] - r * ang.sin()).abs() < 1e-3, "t = {t}: {o:?}");
        }
        // o segundo trecho volta pelo mesmo caminho em 2 s
        let f = q.iter().find(|f| (f.t - 5.).abs() < 1e-9).unwrap().placements[&bloco].origin;
        let ang = 45f64.to_radians();
        assert!((f[0] - 30. * ang.cos()).abs() < 1e-3 && (f[1] - 30. * ang.sin()).abs() < 1e-3, "{f:?}");
        // o cursor de tempo cai no trecho certo
        assert_eq!(trecho(&lista, 1.), Some((1, 0.25)));
        assert_eq!(trecho(&lista, 5.), Some((2, 0.5)));
        assert_eq!(trecho(&lista, 99.), Some((2, 1.)));
    }

    #[test]
    fn tabela_tem_uma_linha_por_quadro_e_componente() {
        let dir = pasta("tabela");
        let (mut a, giro, _, _) = braco(&dir);
        a.nova_posicao("A");
        crate::kin::levar_a(&mut a, giro, Grau::Giro, 90.);
        a.nova_posicao("B");
        a.posicoes[1].transicao = 1.;
        let lista = a.posicoes.clone();
        let q = percorrer(&mut a, &lista, 4.);
        let t = tabela(&a, &q);
        let linhas: Vec<&str> = t.lines().collect();
        assert_eq!(linhas.len(), 1 + 5 * 3);
        let ultima_braco = linhas.iter().rev().find(|l| l.contains(";Braço;")).unwrap();
        assert!(ultima_braco.starts_with("1.0000;Braço;") && ultima_braco.contains(";90.0000;0.0000;0.0000;sim"), "{ultima_braco}");
    }

    #[test]
    fn junta_apagada_nao_atrapalha() {
        let dir = pasta("apagada");
        let (mut a, giro, _, _) = braco(&dir);
        let p = a.gravar_posicao("x");
        a.joints.retain(|j| j.id != giro);
        let rep = aplicar(&mut a, &p);
        assert!(rep.residual.is_finite());
    }
}
