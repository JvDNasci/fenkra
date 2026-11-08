// SPDX-License-Identifier: LGPL-2.1-or-later
//! Sketch como fonte de geometria: extrair e publicar curvas, perfis e pontos de um sketch.
//!
//! Tudo passa pelo mecanismo comum de referências: uma curva de sketch é `CurveRef::SketchCurve`,
//! um ponto é `PointRef::SketchPoint`. Extrair cria um elemento associativo da pasta de geometria
//! (cópia de curva, junção de curvas ou cópia de ponto); publicar é marcar esse elemento. Assim o
//! que vem do sketch vale em qualquer comando que aceite curva, ponto ou plano, e acompanha o
//! sketch quando ele muda.
//!
//! Os perfis saem da geometria resolvida (pontas que se tocam), e não só dos pontos compartilhados,
//! porque uma coincidência entre pontos distintos também fecha um contorno.

use fenkra_core::sketch::Curve;
use fenkra_core::{CurveRef, Document, FeatureKind, Id, PointRef, RefKind};
use fenkra_solidos::geo3d::{chain_indices, tangent_chain, Curve3};
use fenkra_solidos::RecomputeResult;

type V3 = [f64; 3];

/// Folga para duas pontas contarem como ligadas (a polilinha do sketch vem em f32).
const FOLGA: f64 = 1e-3;

/// Como a extração junta as curvas vizinhas à curva clicada.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Juntar {
    /// só a curva clicada
    Nada,
    /// tudo o que se liga pelas pontas
    Ponto,
    /// continua enquanto a tangente for contínua
    Tangente,
}

/// Curvas do sketch que viram geometria (sem construção e sem pontos soltos), com a polilinha no mundo.
fn curvas(r: &RecomputeResult, sketch: Id) -> Vec<(Id, Vec<V3>)> {
    let Some(sk) = r.sketches.iter().find(|s| s.feature == sketch) else { return vec![] };
    sk.ents
        .iter()
        .filter(|(_, cons, p)| !*cons && p.len() >= 2)
        .map(|(c, _, p)| (*c, p.iter().map(|q| q.map(f64::from)).collect()))
        .collect()
}

fn curvas3(cs: &[(Id, Vec<V3>)]) -> Option<Vec<Curve3>> {
    cs.iter().map(|(_, p)| Curve3::poly(p.clone(), false)).collect::<Result<Vec<_>, _>>().ok()
}

/// A curva clicada e as vizinhas que `modo` alcança, na ordem do sketch.
pub fn cadeia(r: &RecomputeResult, sketch: Id, curve: Id, modo: Juntar) -> Vec<CurveRef> {
    let so = vec![CurveRef::SketchCurve { sketch, curve }];
    if modo == Juntar::Nada {
        return so;
    }
    let cs = curvas(r, sketch);
    let (Some(i), Some(c3)) = (cs.iter().position(|(c, _)| *c == curve), curvas3(&cs)) else { return so };
    let idx = match modo {
        Juntar::Tangente => tangent_chain(&c3, i, FOLGA),
        _ => chain_indices(&c3, i, FOLGA),
    };
    idx.into_iter().map(|j| CurveRef::SketchCurve { sketch, curve: cs[j].0 }).collect()
}

/// Perfis do sketch: cada grupo de curvas ligadas pelas pontas (um contorno fechado, ou uma linha
/// aberta) é um conjunto.
pub fn perfis(r: &RecomputeResult, sketch: Id) -> Vec<Vec<CurveRef>> {
    let cs = curvas(r, sketch);
    let Some(c3) = curvas3(&cs) else { return cs.iter().map(|(c, _)| vec![CurveRef::SketchCurve { sketch, curve: *c }]).collect() };
    let mut usado = vec![false; cs.len()];
    let mut out = vec![];
    for i in 0..cs.len() {
        if usado[i] {
            continue;
        }
        let idx = chain_indices(&c3, i, FOLGA);
        for j in &idx {
            usado[*j] = true;
        }
        out.push(idx.into_iter().map(|j| CurveRef::SketchCurve { sketch, curve: cs[j].0 }).collect());
    }
    out
}

/// Pontos soltos do sketch (os de furo), que não fazem parte de curva nenhuma.
pub fn pontos_soltos(doc: &Document, sketch: Id) -> Vec<PointRef> {
    let Some(FeatureKind::Sketch(s)) = doc.feature(sketch).map(|f| &f.kind) else { return vec![] };
    s.data
        .curves
        .iter()
        .filter(|c| !c.construction)
        .filter_map(|c| if let Curve::Point { p } = c.geom { Some(PointRef::SketchPoint { sketch, point: p }) } else { None })
        .collect()
}

/// O elemento que representa um conjunto de curvas: cópia de uma, ou junção de várias.
pub fn kind_de_curvas(set: &[CurveRef]) -> Option<RefKind> {
    match set {
        [] => None,
        [c] => Some(RefKind::CurveCopy { curve: c.clone() }),
        _ => Some(RefKind::CurveJoin { curves: set.to_vec() }),
    }
}

/// Cópia associativa de um ponto. Não há um tipo só para isso: o ponto entre ele e ele mesmo, na
/// proporção zero, é o próprio ponto e segue a origem.
pub fn kind_de_ponto(p: PointRef) -> RefKind {
    RefKind::PointBetween { a: p, b: p, ratio: "0".into() }
}

/// Mesmo conjunto de curvas, em qualquer ordem (para não publicar duas vezes o mesmo perfil).
fn mesmo(a: &RefKind, b: &RefKind) -> bool {
    match (a, b) {
        (RefKind::CurveJoin { curves: x }, RefKind::CurveJoin { curves: y }) => x.len() == y.len() && x.iter().all(|c| y.contains(c)),
        _ => a == b,
    }
}

/// Cria (ou reaproveita) os elementos de um sketch inteiro: um por perfil e um por ponto solto.
/// Com `publicar`, eles ficam publicados com o nome do sketch: `Sketch.1`, `Sketch.2`… para os
/// perfis e `Sketch.p1`… para os pontos. Devolve os elementos e quantos são novos.
pub fn extrair_sketch(doc: &mut Document, r: &RecomputeResult, sketch: Id, publicar: bool) -> (Vec<Id>, usize) {
    let Some(nome) = doc.feature(sketch).filter(|f| matches!(f.kind, FeatureKind::Sketch(_))).map(|f| f.name.clone()) else { return (vec![], 0) };
    let mut kinds: Vec<(RefKind, String)> = vec![];
    for (i, set) in perfis(r, sketch).iter().enumerate() {
        if let Some(k) = kind_de_curvas(set) {
            kinds.push((k, format!("{nome}.{}", i + 1)));
        }
    }
    for (i, p) in pontos_soltos(doc, sketch).into_iter().enumerate() {
        kinds.push((kind_de_ponto(p), format!("{nome}.p{}", i + 1)));
    }
    let (mut ids, mut novos) = (vec![], 0);
    for (k, base) in kinds {
        // o mesmo perfil já extraído: publicar de novo só marca, extrair de novo não duplica
        if let Some(e) = doc.references.iter_mut().find(|e| mesmo(&e.kind, &k) && (e.published || !publicar)) {
            e.published |= publicar;
            ids.push(e.id);
            continue;
        }
        let id = doc.add_geometry(k);
        let nome_livre = if doc.name_taken(&base) { doc.unique_name(&format!("{base}_")) } else { base };
        if let Some(e) = doc.references.iter_mut().find(|e| e.id == id) {
            e.name = nome_livre;
            e.published = publicar;
        }
        ids.push(id);
        novos += 1;
    }
    (ids, novos)
}

/// Um elemento publicado para a curva (com a cadeia que `modo` junta) ou o ponto de sketch clicado.
pub fn publicar_elemento(doc: &mut Document, k: RefKind) -> Id {
    if let Some(e) = doc.references.iter_mut().find(|e| mesmo(&e.kind, &k)) {
        e.published = true;
        return e.id;
    }
    let id = doc.add_geometry(k);
    if let Some(e) = doc.references.iter_mut().find(|e| e.id == id) {
        e.published = true;
    }
    id
}

/// Plano do sketch, para os campos de plano que recebem um elemento dele.
pub fn plano(doc: &Document, sketch: Id) -> Option<fenkra_core::PlaneRef> {
    match doc.feature(sketch).map(|f| &f.kind) {
        Some(FeatureKind::Sketch(s)) => Some(s.plane),
        _ => None,
    }
}

/// Desenvolvimento (`FENKRA_DEMO_JANELA=sketch_publicar`): sketch com um retângulo, uma linha solta e
/// um ponto, já publicado como faria o menu do sketch. Devolve o documento e a última publicação.
pub fn documento_demo() -> (Document, Id) {
    use fenkra_core::{OriginPlane, PlaneRef, Sketch, SketchData};
    let mut d = Document::new("Publicar sketch");
    let mut data = SketchData::default();
    let mut n = 7_200_000u64;
    let mut al = || {
        n += 1;
        Id(n)
    };
    data.add_rect(&mut al, [-40., -20.], [40., 20.], false);
    let (a, b) = (data.add_point(&mut al, -40., 40.), data.add_point(&mut al, 40., 55.));
    data.add_curve(&mut al, Curve::Line { a, b }, false);
    let p = data.add_point(&mut al, 0., 0.);
    data.add_curve(&mut al, Curve::Point { p }, false);
    let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
    let r = fenkra_solidos::recompute(&d);
    let (ids, _) = extrair_sketch(&mut d, &r, sk, true);
    let ultimo = ids.last().copied().unwrap_or(sk);
    (d, ultimo)
}

#[cfg(test)]
mod testes {
    use super::*;
    use fenkra_core::{BoolOp, Extent, Extrude, LinkedGeo, OriginPlane, PlaneRef, Sketch, SketchData};
    use fenkra_solidos::Status;
    use fenkra_solidos::{publications, recompute};

    /// Retângulo 40 x 10, uma linha solta e um ponto de furo, no XY.
    fn doc() -> (Document, Id, Id) {
        let mut d = Document::new("t");
        let mut data = SketchData::default();
        let mut n = 7_100_000u64;
        let mut al = || {
            n += 1;
            Id(n)
        };
        let (_, l) = data.add_rect(&mut al, [0., 0.], [40., 10.], false);
        let a = data.add_point(&mut al, 0., 30.);
        let b = data.add_point(&mut al, 25., 30.);
        data.add_curve(&mut al, Curve::Line { a, b }, false);
        let p = data.add_point(&mut al, 20., 5.);
        data.add_curve(&mut al, Curve::Point { p }, false);
        let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
        (d, sk, l[0])
    }

    #[test]
    fn perfis_juntam_o_retangulo_e_separam_a_linha_solta() {
        let (d, sk, _) = doc();
        let r = recompute(&d);
        let ps = perfis(&r, sk);
        let mut tam: Vec<usize> = ps.iter().map(|p| p.len()).collect();
        tam.sort();
        assert_eq!(tam, vec![1, 4]);
        assert_eq!(pontos_soltos(&d, sk).len(), 1);
    }

    #[test]
    fn cadeia_por_ponto_alcanca_o_contorno_e_sem_propagacao_so_a_curva() {
        let (d, sk, l0) = doc();
        let r = recompute(&d);
        assert_eq!(cadeia(&r, sk, l0, Juntar::Nada).len(), 1);
        assert_eq!(cadeia(&r, sk, l0, Juntar::Ponto).len(), 4);
        // cantos vivos: a tangente não continua
        assert_eq!(cadeia(&r, sk, l0, Juntar::Tangente).len(), 1);
    }

    /// O reporte: publicar o sketch tem de dar o contorno fechado (que outra peça pode extrudar),
    /// a linha e o ponto, e acompanhar o sketch quando ele muda.
    #[test]
    fn publicar_sketch_publica_perfil_fechado_linha_e_ponto_associativos() {
        let (mut d, sk, _) = doc();
        let r = recompute(&d);
        let (ids, novos) = extrair_sketch(&mut d, &r, sk, true);
        assert_eq!((ids.len(), novos), (3, 3));
        let r = recompute(&d);
        assert!(r.status.values().all(|s| matches!(s, Status::Ok)), "{:?}", r.status);
        let pubs = publications(&d);
        assert_eq!(pubs.len(), 3, "{:?}", pubs.iter().map(|p| &p.0).collect::<Vec<_>>());
        let nome = d.feature(sk).unwrap().name.clone();
        let contorno = pubs.iter().find_map(|(n, g)| match g {
            LinkedGeo::Curve(p) if p.len() > 4 || n == &format!("{nome}.1") || n == &format!("{nome}.2") => Some(p.clone()),
            _ => None,
        });
        let contorno = contorno.expect("perfil publicado");
        let caixa = contorno.iter().fold(([f64::MAX; 3], [f64::MIN; 3]), |(mut lo, mut hi), p| {
            for i in 0..3 {
                lo[i] = lo[i].min(p[i]);
                hi[i] = hi[i].max(p[i]);
            }
            (lo, hi)
        });
        let perto = |a: [f64; 3], b: [f64; 3]| (0..3).all(|i| (a[i] - b[i]).abs() < 1e-3);
        assert!(perto(caixa.0, [0., 0., 0.]) && perto(caixa.1, [40., 10., 0.]), "{caixa:?}");
        assert!(pubs.iter().any(|(n, g)| n == &format!("{nome}.p1") && matches!(g, LinkedGeo::Point(p) if perto(*p, [20., 5., 0.]))));
        // publicar de novo não duplica
        let r = recompute(&d);
        assert_eq!(extrair_sketch(&mut d, &r, sk, true).1, 0);
        assert_eq!(publications(&d).len(), 3);
    }

    /// O perfil extraído do sketch é um contorno fechado de verdade: vira sólido numa extrusão.
    #[test]
    fn perfil_extraido_do_sketch_extruda_como_solido() {
        let (mut d, sk, _) = doc();
        let r = recompute(&d);
        let (ids, _) = extrair_sketch(&mut d, &r, sk, false);
        let r = recompute(&d);
        let contorno = ids.iter().copied().find(|id| matches!(d.reference(*id).map(|e| &e.kind), Some(RefKind::CurveJoin { curves }) if curves.len() == 4)).unwrap();
        assert!(r.curves.iter().any(|c| c.id == contorno));
        d.add_feature(FeatureKind::Extrude(Extrude { sketch: contorno, op: BoolOp::Add, extent: Extent::Blind("5".into()), reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
        let r = recompute(&d);
        assert!(r.status.values().all(|s| matches!(s, Status::Ok)), "{:?}", r.status);
        assert_eq!(r.bodies.len(), 1);
        assert!((r.bodies[0].mass.volume - 2000.).abs() < 1e-3, "{}", r.bodies[0].mass.volume);
    }
}
