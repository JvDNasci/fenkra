// SPDX-License-Identifier: LGPL-2.1-or-later
//! Repetição de componentes (B263): posições das cópias e a ligação com a geometria das peças.
//!
//! Como funciona:
//! 1. cada posição da repetição é um [`Passo`], com a chave (i, j) e o movimento rígido que leva o
//!    original até ali. A chave é a posição e não a ordem: pular a cópia 3 continua pulando a mesma
//!    cópia quando a quantidade muda, igual ao padrão de recursos da peça;
//! 2. a direção não precisa ser um vetor digitado. Com uma referência (aresta, eixo ou face de uma
//!    peça) ela vem da peça a cada atualização: o sentido local levado pela posição da peça. Mover a
//!    peça ou mudar a geometria dela muda a direção, e [`sincronizar`] relê a geometria pelo ponto
//!    clicado, com a mesma regra das restrições;
//! 3. no modo Pelos furos, os pontos vêm de um padrão de furos da peça. Quem decide quais furos existem
//!    é a peça, e por isso uma instância pulada no padrão dela não recebe cópia aqui.

use crate::load::{full_path, geom_at, PartCache};
use crate::*;
use fenkra_core::{Document, Feature, FeatureKind};
use fenkra_solidos::padrao;
use std::path::Path;

/// Uma posição da repetição.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Passo {
    /// (i, j): índice em cada direção; circular e pelos furos usam (i, 0). O original é (0, 0)
    pub chave: (u32, u32),
    pub desloc: V3,
    /// rotação em volta de `centro` (vetor de rotação, em radianos)
    pub giro: V3,
    pub centro: V3,
    pub pulada: bool,
}

impl Passo {
    pub fn aplicar(&self, p: &Placement) -> Placement {
        p.moved(self.desloc, self.giro, self.centro)
    }
}

/// Padrão de furos que a repetição pode seguir.
#[derive(Clone, PartialEq, Debug)]
pub struct PadraoFuros {
    pub feature: Id,
    pub nome: String,
    /// bocas dos furos que existem, no sistema da peça
    pub pontos: Vec<V3>,
}

/// Padrões de furos de uma peça: recurso de padrão que repete furos (o que ele pulou fica de fora) e
/// furo com vários centros.
pub fn padroes_de_furos(doc: &Document) -> Vec<PadraoFuros> {
    let mut v = vec![];
    let linhas = fenkra_solidos::hole_table(doc);
    for (_, f) in doc.active_features() {
        let f: &Feature = f;
        match &f.kind {
            FeatureKind::Pattern(_) => {
                let pontos: Vec<V3> = padrao::furos_do_padrao(doc, f.id).into_iter().filter(|i| !i.pulada).map(|i| i.centro).collect();
                if pontos.len() > 1 {
                    v.push(PadraoFuros { feature: f.id, nome: f.name.clone(), pontos });
                }
            }
            FeatureKind::Hole(_) => {
                let pontos: Vec<V3> = linhas.iter().filter(|r| r.feature == f.id).map(|r| r.pos).collect();
                if pontos.len() > 1 {
                    v.push(PadraoFuros { feature: f.id, nome: f.name.clone(), pontos });
                }
            }
            _ => {}
        }
    }
    v
}

/// Sentido e ponto da referência, no mundo.
fn no_mundo(a: &Assembly, r: &MateRef) -> Option<(V3, V3)> {
    let pl = if r.comp == ORIGEM { Placement::default() } else { a.node(r.comp)?.placement };
    let (p, d) = match r.geom {
        Geom::Plane { p, n } => (r.pick.unwrap_or(p), n),
        Geom::Axis { p, d } => (p, d),
        Geom::Point(_) => return None,
    };
    Some((pl.point(p), unit(pl.dir(d))))
}

fn com_sinal(d: V3, inverter: bool) -> V3 {
    let s = if inverter { -1. } else { 1. };
    d.map(|v| v * s)
}

impl Assembly {
    /// Sentido de uma direção da repetição: o da referência, se ela ainda resolve; senão o guardado.
    fn sentido(&self, r: Option<&MateRef>, guardado: V3, inverter: bool) -> V3 {
        com_sinal(r.and_then(|r| no_mundo(self, r)).map_or(unit(guardado), |x| x.1), inverter)
    }

    /// Sentidos unitários das duas direções, com o sinal do Inverter.
    pub fn direcoes(&self, p: &CompPattern, _vars: &HashMap<String, f64>) -> (V3, V3) {
        match &p.kind {
            PatternKind::Linear { dir, dir2, .. } => (self.sentido(p.ref1.as_ref(), *dir, p.inverter), self.sentido(p.ref2.as_ref(), dir2.unwrap_or([0., 1., 0.]), p.inverter2)),
            PatternKind::Circular { axis, .. } => (self.sentido(p.ref1.as_ref(), *axis, p.inverter), [0.; 3]),
            PatternKind::Furos { .. } => ([0.; 3], [0.; 3]),
        }
    }

    /// Todas as posições, sem o original. As puladas vêm marcadas: a prévia mostra a marca delas.
    pub fn passos(&self, p: &CompPattern, vars: &HashMap<String, f64>) -> Vec<Passo> {
        let num = |e: &String| expr::eval_str(e, vars).unwrap_or(1.);
        let contagem = |e: &String| num(e).round().max(1.) as u32;
        let mut v = vec![];
        let mut por = |chave: (u32, u32), desloc: V3, giro: V3, centro: V3| {
            if chave != (0, 0) {
                v.push(Passo { chave, desloc, giro, centro, pulada: p.pular.contains(&chave) });
            }
        };
        match &p.kind {
            PatternKind::Linear { dir, count, spacing, dir2, count2, spacing2 } => {
                let d1 = self.sentido(p.ref1.as_ref(), *dir, p.inverter).map(|x| x * num(spacing));
                let (n2, d2) = match dir2 {
                    Some(u) => (contagem(count2), self.sentido(p.ref2.as_ref(), *u, p.inverter2).map(|x| x * num(spacing2))),
                    None => (1, [0.; 3]),
                };
                for j in 0..n2 {
                    for i in 0..contagem(count) {
                        por((i, j), [0, 1, 2].map(|k| d1[k] * i as f64 + d2[k] * j as f64), [0.; 3], [0.; 3]);
                    }
                }
            }
            PatternKind::Circular { origin, axis, count, angle } => {
                let n = contagem(count);
                let total = num(angle);
                let passo = if (total.abs() - 360.).abs() < 1e-9 { total / n as f64 } else { total / (n.max(2) - 1) as f64 };
                let (centro, a) = match p.ref1.as_ref().and_then(|r| no_mundo(self, r)) {
                    Some((c, d)) => (c, com_sinal(d, p.inverter)),
                    None => (*origin, com_sinal(unit(*axis), p.inverter)),
                };
                for i in 0..n {
                    por((i, 0), [0.; 3], a.map(|x| x * (passo * i as f64).to_radians()), centro);
                }
            }
            PatternKind::Furos { comp, pontos, .. } => {
                let (Some(alvo), Some(src)) = (self.node(*comp), self.node(p.source)) else { return v };
                let mundo: Vec<V3> = pontos.iter().map(|q| alvo.placement.point(*q)).collect();
                let dist = |q: &V3| (0..3).map(|k| (q[k] - src.placement.origin[k]).powi(2)).sum::<f64>();
                // o furo em que o original está é a base; ele não recebe cópia
                let Some(base) = (0..mundo.len()).min_by(|a, b| dist(&mundo[*a]).total_cmp(&dist(&mundo[*b]))) else { return v };
                for (i, q) in mundo.iter().enumerate() {
                    // a chave segue a ordem dos furos, com o da base no lugar do zero
                    let k = if i == base { 0 } else if i < base { i as u32 + 1 } else { i as u32 };
                    por((k, 0), [0, 1, 2].map(|c| q[c] - mundo[base][c]), [0.; 3], [0.; 3]);
                }
            }
        }
        v
    }

    /// Original e os outros componentes repetidos junto, que ainda existem.
    pub fn fontes(&self, p: &CompPattern) -> Vec<Id> {
        std::iter::once(p.source).chain(p.extras.iter().copied()).filter(|id| self.node(*id).is_some()).collect()
    }

    /// Grava a repetição montada no comando: o rascunho vira a repetição `id`, ou uma nova.
    pub fn gravar_repeticao(&mut self, rascunho: &CompPattern, id: Option<Id>) -> Id {
        let id = match id.filter(|i| self.patterns.iter().any(|p| p.id == *i)) {
            Some(i) => i,
            None => {
                let i = self.alloc();
                let name = self.unique("Repetição");
                self.patterns.push(CompPattern { id: i, name, instances: vec![], ..rascunho.clone() });
                i
            }
        };
        if let Some(p) = self.patterns.iter_mut().find(|p| p.id == id) {
            let (name, instances) = (p.name.clone(), std::mem::take(&mut p.instances));
            *p = CompPattern { id, name, instances, ..rascunho.clone() };
        }
        self.update_patterns();
        id
    }
}

/// Relê o que vem das peças: as referências de direção, pelo ponto clicado, e os pontos do padrão de
/// furos seguido. Depois refaz as cópias.
pub fn sincronizar(a: &mut Assembly, cache: &mut PartCache, dir: Option<&Path>) {
    if a.patterns.is_empty() {
        return;
    }
    let caminho = |a: &Assembly, id: Id| match a.node(id).map(|n| &n.kind) {
        Some(NodeKind::Part { path }) => Some(full_path(dir, path)),
        _ => None,
    };
    for k in 0..a.patterns.len() {
        for lado in 0..2 {
            let p = &a.patterns[k];
            let r = if lado == 0 { p.ref1.as_ref() } else { p.ref2.as_ref() };
            let Some((pick, comp)) = r.and_then(|r| Some((r.pick?, r.comp))) else { continue };
            let Some(arq) = caminho(a, comp) else { continue };
            let Some(g) = cache.get(&arq, true).ok().and_then(|lp| lp.shape.as_ref().and_then(|s| geom_at(s, pick).ok())) else { continue };
            let p = &mut a.patterns[k];
            let r = if lado == 0 { p.ref1.as_mut() } else { p.ref2.as_mut() };
            if let Some(r) = r.filter(|r| std::mem::discriminant(&r.geom) == std::mem::discriminant(&g)) {
                r.geom = g;
            }
        }
        if let PatternKind::Furos { comp, feature, .. } = a.patterns[k].kind.clone() {
            let Some(arq) = caminho(a, comp) else { continue };
            let Ok(lp) = cache.get(&arq, false) else { continue };
            let novos = padroes_de_furos(&lp.doc).into_iter().find(|f| f.feature == feature).map(|f| f.pontos);
            if let (Some(n), PatternKind::Furos { pontos, .. }) = (novos, &mut a.patterns[k].kind) {
                *pontos = n;
            }
        }
    }
    a.update_patterns();
}

#[cfg(test)]
mod tests {
    use super::*;
    use fenkra_core::{sketch::Curve, AxisRef, BoolOp, Extent, Extrude, FeatureKind, Hole, HoleKind, OriginAxis, OriginPlane, Pattern, PatternKind as Pk, PlaneRef, Sketch, SketchData};
    use std::path::PathBuf;

    /// Chapa de `l` × 40 × `h` com um furo em (10, 20) e padrão de furos em X, a cada 20 mm.
    fn chapa(l: f64, h: f64, quantos: u32, pular: Vec<(u32, u32)>) -> Document {
        let mut d = Document::new("Chapa");
        let mut n = 7_200_000u64;
        let mut al = || {
            n += 1;
            Id(n)
        };
        let mut s = SketchData::default();
        s.add_rect(&mut al, [0., 0.], [l, 40.], false);
        let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data: s }));
        d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind(h.to_string()), reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
        let topo = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), &h.to_string());
        let mut c = SketchData::default();
        let p = c.add_point(&mut al, 10., 20.);
        c.add_curve(&mut al, Curve::Point { p }, false);
        let sc = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Datum(topo), data: c }));
        let furo = d.add_feature(FeatureKind::Hole(Hole { sketch: sc, kind: HoleKind::Simples, diameter: "4".into(), depth: Extent::ThroughAll, thread: None, thread_modeled: false, standard: None, points: vec![], dir: None, reverse: false, tip: None, taper: "0".into(), left_hand: false, extra: Default::default() }));
        d.add_feature(FeatureKind::Pattern(Pattern {
            kind: Pk::Linear { dir: AxisRef::Origin(OriginAxis::X), count: quantos.to_string(), spacing: "20".into(), dir2: None, count2: "1".into(), spacing2: "0".into(), total: false },
            sources: vec![furo],
            skip: pular,
        }));
        d
    }

    fn linear(n: &str, s: &str) -> PatternKind {
        PatternKind::Linear { dir: [1., 0., 0.], count: n.into(), spacing: s.into(), dir2: None, count2: "2".into(), spacing2: "20".into() }
    }

    fn componente(a: &mut Assembly, nome: &str, o: V3) -> Id {
        a.add_component(nome, NodeKind::Part { path: "p.fkp".into() }, Placement { origin: o, ..Default::default() }, None)
    }

    fn origens(a: &Assembly, id: Id) -> Vec<V3> {
        let p = a.patterns.iter().find(|p| p.id == id).unwrap();
        p.instances.iter().map(|i| a.node(*i).unwrap().placement.origin).collect()
    }

    fn pasta(nome: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("fenkra_rep_{nome}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn pular_tira_a_copia_da_posicao_e_sobrevive_a_mudar_a_quantidade() {
        let mut a = Assembly::new("t");
        let s = componente(&mut a, "S", [0.; 3]);
        let id = a.add_pattern(s, linear("4", "10"));
        assert_eq!(origens(&a, id).len(), 3);
        a.patterns[0].pular = vec![(2, 0)];
        a.update_patterns();
        assert_eq!(origens(&a, id), vec![[10., 0., 0.], [30., 0., 0.]]);
        if let PatternKind::Linear { count, .. } = &mut a.patterns[0].kind {
            *count = "6".into();
        }
        a.update_patterns();
        assert_eq!(origens(&a, id).iter().map(|o| o[0]).collect::<Vec<_>>(), vec![10., 30., 40., 50.], "a mesma posição continua pulada");
    }

    #[test]
    fn segunda_direcao_faz_grade_e_inverter_troca_o_sentido() {
        let mut a = Assembly::new("t");
        let s = componente(&mut a, "S", [0.; 3]);
        let id = a.add_pattern(s, PatternKind::Linear { dir: [1., 0., 0.], count: "3".into(), spacing: "10".into(), dir2: Some([0., 1., 0.]), count2: "2".into(), spacing2: "7".into() });
        assert_eq!(origens(&a, id).len(), 5);
        assert!(origens(&a, id).contains(&[20., 7., 0.]));
        a.patterns[0].inverter = true;
        a.patterns[0].inverter2 = true;
        a.update_patterns();
        assert!(origens(&a, id).contains(&[-20., -7., 0.]));
    }

    #[test]
    fn varios_componentes_repetem_juntos() {
        let mut a = Assembly::new("t");
        let s = componente(&mut a, "S", [0.; 3]);
        let e = componente(&mut a, "E", [0., 5., 0.]);
        let mut r = CompPattern::rascunho(s, linear("3", "10"));
        r.extras = vec![e];
        let id = a.gravar_repeticao(&r, None);
        assert_eq!(origens(&a, id), vec![[10., 0., 0.], [10., 5., 0.], [20., 0., 0.], [20., 5., 0.]]);
        assert_eq!(a.fontes(&a.patterns[0]), vec![s, e]);
    }

    #[test]
    fn direcao_da_referencia_acompanha_a_peca() {
        let mut a = Assembly::new("t");
        let guia = componente(&mut a, "Guia", [100., 0., 0.]);
        let s = componente(&mut a, "S", [0.; 3]);
        let mut r = CompPattern::rascunho(s, linear("2", "10"));
        r.ref1 = Some(MateRef { comp: guia, geom: Geom::Axis { p: [0.; 3], d: [1., 0., 0.] }, pick: None });
        let id = a.gravar_repeticao(&r, None);
        assert!((origens(&a, id)[0][0] - 10.).abs() < 1e-9);
        // a peça de referência gira 90° em Z: a direção vai junto
        let g = a.node(guia).unwrap().placement.moved([0.; 3], [0., 0., std::f64::consts::FRAC_PI_2], [100., 0., 0.]);
        a.node_mut(guia).unwrap().placement = g;
        a.update_patterns();
        let o = origens(&a, id)[0];
        assert!(o[0].abs() < 1e-9 && (o[1] - 10.).abs() < 1e-9, "{o:?}");
    }

    #[test]
    fn circular_pelo_eixo_clicado() {
        let mut a = Assembly::new("t");
        let guia = componente(&mut a, "Guia", [50., 0., 0.]);
        let s = componente(&mut a, "S", [60., 0., 0.]);
        let mut r = CompPattern::rascunho(s, PatternKind::Circular { origin: [0.; 3], axis: [1., 0., 0.], count: "4".into(), angle: "360".into() });
        r.ref1 = Some(MateRef { comp: guia, geom: Geom::Axis { p: [0.; 3], d: [0., 0., 1.] }, pick: None });
        let id = a.gravar_repeticao(&r, None);
        let o = origens(&a, id)[0];
        assert!((o[0] - 50.).abs() < 1e-9 && (o[1] - 10.).abs() < 1e-9, "gira em volta do eixo da peça: {o:?}");
    }

    #[test]
    fn mudar_a_geometria_da_referencia_atualiza_a_direcao() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let d = pasta("geom");
        std::fs::write(d.join("p.fkp"), chapa(100., 5., 2, vec![]).to_json()).unwrap();
        let mut a = Assembly::new("t");
        let guia = componente(&mut a, "Guia", [0.; 3]);
        let s = componente(&mut a, "S", [0., 0., 50.]);
        let mut c = PartCache::default();
        // o ponto (50, 30, 5) está na face de cima da chapa, normal +Z
        let mut r = CompPattern::rascunho(s, linear("2", "10"));
        r.ref1 = Some(MateRef { comp: guia, geom: Geom::Plane { p: [50., 30., 5.], n: [0., 0., 1.] }, pick: Some([50., 30., 5.]) });
        let id = a.gravar_repeticao(&r, None);
        sincronizar(&mut a, &mut c, Some(&d));
        assert!((origens(&a, id)[0][2] - 60.).abs() < 1e-9);
        // a peça encurta e engrossa: o mesmo ponto agora fica na face da ponta, normal +X
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(d.join("p.fkp"), chapa(50., 10., 2, vec![]).to_json()).unwrap();
        c.forget(&d.join("p.fkp"));
        sincronizar(&mut a, &mut c, Some(&d));
        let o = origens(&a, id)[0];
        assert!((o[0] - 10.).abs() < 1e-6 && (o[2] - 50.).abs() < 1e-6, "a direção agora é a da peça: {o:?}");
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn pelos_furos_segue_o_padrao_e_respeita_o_que_ele_pulou() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let d = pasta("furos");
        let doc = chapa(120., 5., 5, vec![(2, 0)]);
        std::fs::write(d.join("p.fkp"), doc.to_json()).unwrap();
        let padroes = padroes_de_furos(&doc);
        assert_eq!(padroes.len(), 1);
        assert_eq!(padroes[0].pontos.len(), 4, "o furo pulado na peça fica de fora");
        let mut a = Assembly::new("t");
        let base = componente(&mut a, "Chapa", [0., 0., 0.]);
        let parafuso = a.add_component("Parafuso", NodeKind::Part { path: "p.fkp".into() }, Placement { origin: [10., 20., 5.], ..Default::default() }, None);
        let mut r = CompPattern::rascunho(parafuso, PatternKind::Furos { comp: base, feature: padroes[0].feature, pontos: padroes[0].pontos.clone() });
        let id = a.gravar_repeticao(&r, None);
        let xs: Vec<f64> = origens(&a, id).iter().map(|o| o[0]).collect();
        assert_eq!(xs.len(), 3);
        for (x, esperado) in xs.iter().zip([30., 70., 90.]) {
            assert!((x - esperado).abs() < 1e-6, "{xs:?}");
        }
        // a peça passa a pular outro furo: a repetição acompanha ao sincronizar
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(d.join("p.fkp"), chapa(120., 5., 5, vec![(1, 0)]).to_json()).unwrap();
        let mut c = PartCache::default();
        sincronizar(&mut a, &mut c, Some(&d));
        let xs: Vec<f64> = origens(&a, id).iter().map(|o| o[0]).collect();
        assert_eq!(xs.len(), 3);
        assert!((xs[0] - 50.).abs() < 1e-6, "{xs:?}");
        // pular uma cópia aqui, além do que a peça pulou
        r = a.patterns[0].clone();
        r.pular = vec![(1, 0)];
        a.gravar_repeticao(&r, Some(id));
        assert_eq!(origens(&a, id).len(), 2);
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn json_antigo_abre_sem_os_campos_novos() {
        let antigo = r#"{"Linear":{"dir":[1.0,0.0,0.0],"count":"3","spacing":"20"}}"#;
        let k: PatternKind = serde_json::from_str(antigo).unwrap();
        assert!(matches!(k, PatternKind::Linear { dir2: None, .. }));
    }
}
