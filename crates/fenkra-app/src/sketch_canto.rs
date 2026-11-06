// SPDX-License-Identifier: LGPL-2.1-or-later
//! Filete e chanfro de sketch (B32): canto entre duas linhas, em um ou vários lugares.
//!
//! O canto antigo some junto com as restrições presas nele, a menos que se peça para mantê-lo: aí ele
//! fica como ponto de construção sobre as duas linhas, e as cotas continuam medindo até lá. Cantos
//! seguidos com o mesmo valor não criam cota: ficam iguais ao primeiro por restrição.

use crate::app::fmt;
use crate::sketch_ui::{d2, line_ends, lines_at, Ent};
use fenkra_core::sketch::{Curve, Kind, SketchData};
use fenkra_core::Id;

/// Como o chanfro mede o corte.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ModoChanfro {
    #[default]
    Iguais,
    DuasDistancias,
    DistanciaAngulo,
}

/// Cotas de um canto feito: a principal e, no chanfro de duas medidas, a outra.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Canto {
    pub cota: Id,
    pub segunda: Option<Id>,
}

/// O que se pede de um canto.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Pedido {
    pub chanfro: bool,
    /// raio do filete ou distância do chanfro; sem valor, o que couber
    pub valor: Option<f64>,
    pub modo: ModoChanfro,
    /// segunda distância; sem valor, repete a primeira
    pub segundo: Option<f64>,
    /// ângulo do chanfro em graus, medido na primeira linha
    pub angulo: f64,
    pub manter: bool,
    /// linha que recebe a primeira distância; sem ela, a primeira do sketch
    pub primeira: Option<Id>,
}

impl Pedido {
    pub fn filete(raio: Option<f64>) -> Self {
        Self { chanfro: false, valor: raio, modo: ModoChanfro::Iguais, segundo: None, angulo: 45., manter: false, primeira: None }
    }
    pub fn chanfro(dist: Option<f64>) -> Self {
        Self { chanfro: true, ..Self::filete(dist) }
    }
}

/// Canto formado por duas linhas selecionadas antes de escolher Filete ou Chanfro.
pub(crate) fn corner_da_selecao(sk: &SketchData, sel: &[Ent]) -> Option<Id> {
    let [Ent::Cv(c1), Ent::Cv(c2)] = sel else { return None };
    let (a1, b1) = line_ends(sk, *c1)?;
    let (a2, b2) = line_ends(sk, *c2)?;
    [a1, b1].into_iter().find(|p| (*p == a2 || *p == b2) && lines_at(sk, *p).len() == 2)
}

/// Canto indicado pelo que está sob o cursor: o próprio ponto ou, numa linha, a ponta mais próxima.
/// `None` se o que está ali não é geometria; `Err` diz por que aquele elemento não serve.
pub(crate) fn corner_of(sk: &SketchData, e: Ent, cursor: [f64; 2]) -> Option<Result<Id, &'static str>> {
    let p = match e {
        Ent::Pt(p) => p,
        Ent::Cv(c) => match line_ends(sk, c) {
            Some((a, b)) => {
                if d2(sk.pos(a), cursor) <= d2(sk.pos(b), cursor) {
                    a
                } else {
                    b
                }
            }
            None => return Some(Err("filete e chanfro funcionam entre duas linhas")),
        },
        Ent::Cs(_) => return None,
    };
    Some(if lines_at(sk, p).len() == 2 { Ok(p) } else { Err("esse ponto não é um canto entre duas linhas") })
}

/// Linha do canto `p` que recebe a primeira distância: a que está sob o cursor ou, sobre o ponto, a
/// mais perto dele. Passar o mouse para a outra linha troca o lado do chanfro.
pub(crate) fn primeira_linha(sk: &SketchData, e: Ent, p: Id, cursor: [f64; 2]) -> Option<Id> {
    let linhas = lines_at(sk, p);
    if let Ent::Cv(c) = e {
        if linhas.iter().any(|l| l.0 == c) {
            return Some(c);
        }
    }
    let pc = sk.pos(p);
    let afastado = |q: Id| {
        let (o, l) = (sk.pos(q), d2(sk.pos(q), pc).max(1e-12));
        (((cursor[0] - pc[0]) * (o[1] - pc[1]) - (cursor[1] - pc[1]) * (o[0] - pc[0])) / l).abs()
    };
    linhas.iter().min_by(|a, b| afastado(a.1).total_cmp(&afastado(b.1))).map(|l| l.0)
}

/// Recuo do canto nas duas linhas. Limitado para sobrar linha: um valor repetido de um canto maior
/// não pode consumir o lado inteiro.
pub(crate) fn corner_setback(want: Option<f64>, l1: f64, l2: f64) -> f64 {
    let room = l1.min(l2);
    want.filter(|t| *t > 0.).map_or(0.3 * room, |t| t.min(0.9 * room))
}

/// As linhas passam a terminar nos pontos novos.
fn retarget_corner(sk: &mut SketchData, p: Id, ends: [(Id, Id); 2]) {
    for (lid, tp) in ends {
        if let Some(fenkra_core::sketch::SkCurve { geom: Curve::Line { a, b }, .. }) = sk.curves.iter_mut().find(|c| c.id == lid) {
            if *a == p {
                *a = tp
            } else {
                *b = tp
            }
        }
    }
}

/// O comprimento cotado de uma linha era até o canto. Mantido o canto, a medida vira a distância da
/// ponta até ele: a linha encurtou, mas o que a cota queria dizer continua de pé.
fn cota_ate_o_canto(sk: &mut SketchData, p: Id, linhas: [(Id, Id); 2]) {
    for k in &mut sk.constraints {
        if let Kind::Length(l, v) = &k.kind {
            if let Some((_, ponta)) = linhas.iter().find(|x| x.0 == *l) {
                k.kind = Kind::Distance(*ponta, p, v.clone());
            }
        }
    }
}

/// Filete no canto `p`.
fn fillet_corner(sk: &mut SketchData, al: &mut impl FnMut() -> Id, p: Id, pedido: &Pedido) -> Result<Canto, &'static str> {
    let lines = lines_at(sk, p);
    let [(l1id, e1), (l2id, e2)] = lines[..] else { return Err("esse ponto não é um canto entre duas linhas") };
    let pc = sk.pos(p);
    let (q1, q2) = (sk.pos(e1), sk.pos(e2));
    let unit = |q: [f64; 2]| {
        let l = d2(q, pc).max(1e-12);
        ([(q[0] - pc[0]) / l, (q[1] - pc[1]) / l], l)
    };
    let ((u1, l1), (u2, l2)) = (unit(q1), unit(q2));
    let th = (u1[0] * u2[0] + u1[1] * u2[1]).clamp(-1., 1.).acos();
    if th < 1e-3 || (std::f64::consts::PI - th) < 1e-3 {
        return Err("as linhas são colineares");
    }
    let t = corner_setback(pedido.valor.map(|r| r / (th / 2.).tan()), l1, l2);
    let r = t * (th / 2.).tan();
    let bis = {
        let b = [u1[0] + u2[0], u1[1] + u2[1]];
        let l = (b[0] * b[0] + b[1] * b[1]).sqrt().max(1e-12);
        [b[0] / l, b[1] / l]
    };
    let dc = t / (th / 2.).cos();
    let ctr = [pc[0] + bis[0] * dc, pc[1] + bis[1] * dc];
    let t1 = sk.add_point(al, pc[0] + u1[0] * t, pc[1] + u1[1] * t);
    let t2 = sk.add_point(al, pc[0] + u2[0] * t, pc[1] + u2[1] * t);
    let cc = sk.add_point(al, ctr[0], ctr[1]);
    retarget_corner(sk, p, [(l1id, t1), (l2id, t2)]);
    let (s1, s2) = (sk.pos(t1), sk.pos(t2));
    let ccw = (s1[0] - ctr[0]) * (s2[1] - ctr[1]) - (s1[1] - ctr[1]) * (s2[0] - ctr[0]) > 0.;
    let arc = sk.add_curve(al, if ccw { Curve::Arc { c: cc, s: t1, e: t2 } } else { Curve::Arc { c: cc, s: t2, e: t1 } }, false);
    sk.add_constraint(al, Kind::Tangent(l1id, arc));
    sk.add_constraint(al, Kind::Tangent(l2id, arc));
    let cota = sk.add_constraint(al, Kind::Radius(arc, fmt(r)));
    if pedido.manter {
        cota_ate_o_canto(sk, p, [(l1id, e1), (l2id, e2)]);
        sk.add_curve(al, Curve::Point { p }, true);
        sk.add_constraint(al, Kind::PointOnCurve(p, l1id));
        sk.add_constraint(al, Kind::PointOnCurve(p, l2id));
    } else if !sk.curves.iter().any(|c| SketchData::curve_points(&c.geom).contains(&p)) {
        // o canto antigo some se ninguém mais o usa
        sk.points.retain(|x| x.id != p || x.fixed);
        sk.prune_constraints();
    }
    Ok(Canto { cota, segunda: None })
}

/// Recuos do chanfro em cada linha.
fn recuos(pedido: &Pedido, l1: f64, l2: f64, u1: [f64; 2], u2: [f64; 2]) -> Result<(f64, f64), &'static str> {
    match pedido.modo {
        ModoChanfro::Iguais => {
            let t = corner_setback(pedido.valor, l1, l2);
            Ok((t, t))
        }
        ModoChanfro::DuasDistancias => {
            let t1 = pedido.valor.filter(|t| *t > 0.).map_or(0.3 * l1.min(l2), |t| t.min(0.9 * l1));
            Ok((t1, pedido.segundo.filter(|t| *t > 0.).unwrap_or(t1).min(0.9 * l2)))
        }
        ModoChanfro::DistanciaAngulo => {
            // triângulo do corte: ângulo `th` na primeira linha, `fi` no canto. Pela lei dos senos,
            // o outro recuo é t1 sen(th) / sen(fi + th); a soma passar de 180 graus não fecha
            let fi = (u1[0] * u2[0] + u1[1] * u2[1]).clamp(-1., 1.).acos();
            let th = pedido.angulo.to_radians();
            if th <= 1e-3 || fi + th >= std::f64::consts::PI - 1e-3 {
                return Err("esse ângulo não fecha o corte neste canto");
            }
            let t1 = pedido.valor.filter(|t| *t > 0.).unwrap_or(0.3 * l1.min(l2));
            let t2 = t1 * th.sin() / (fi + th).sin();
            // o que não couber encolhe nas duas linhas: o ângulo é o que foi pedido
            let k = (0.9 * l1 / t1).min(0.9 * l2 / t2).min(1.);
            Ok((t1 * k, t2 * k))
        }
    }
}

/// Chanfro no canto `p`, com a medida que `pedido.modo` diz.
fn chamfer_corner(sk: &mut SketchData, al: &mut impl FnMut() -> Id, p: Id, pedido: &Pedido) -> Result<Canto, &'static str> {
    let mut lines = lines_at(sk, p);
    if lines.len() != 2 {
        return Err("esse ponto não é um canto entre duas linhas");
    }
    if pedido.primeira == Some(lines[1].0) {
        lines.swap(0, 1);
    }
    let ((l1id, e1), (l2id, e2)) = (lines[0], lines[1]);
    let pc = sk.pos(p);
    let (q1, q2) = (sk.pos(e1), sk.pos(e2));
    let (l1, l2) = (d2(q1, pc).max(1e-12), d2(q2, pc).max(1e-12));
    let u1 = [(q1[0] - pc[0]) / l1, (q1[1] - pc[1]) / l1];
    let u2 = [(q2[0] - pc[0]) / l2, (q2[1] - pc[1]) / l2];
    if (u1[0] * u2[1] - u1[1] * u2[0]).abs() < 1e-3 {
        return Err("as linhas são colineares");
    }
    let (r1, r2) = recuos(pedido, l1, l2, u1, u2)?;
    let t1 = sk.add_point(al, pc[0] + u1[0] * r1, pc[1] + u1[1] * r1);
    let t2 = sk.add_point(al, pc[0] + u2[0] * r2, pc[1] + u2[1] * r2);
    retarget_corner(sk, p, [(l1id, t1), (l2id, t2)]);
    let corte = sk.add_curve(al, Curve::Line { a: t1, b: t2 }, false);
    // o canto vira construção: as linhas de t1 e t2 até ele carregam as medidas. A primeira nasce de
    // t1 para o canto porque a cota de ângulo compara direções, e as duas têm de sair do mesmo ponto
    if pedido.manter {
        cota_ate_o_canto(sk, p, [(l1id, e1), (l2id, e2)]);
    }
    let cpt = if pedido.manter { p } else { sk.add_point(al, pc[0], pc[1]) };
    let c1 = sk.add_curve(al, Curve::Line { a: t1, b: cpt }, true);
    let c2 = sk.add_curve(al, Curve::Line { a: cpt, b: t2 }, true);
    sk.add_constraint(al, Kind::Parallel(c1, l1id));
    sk.add_constraint(al, Kind::Parallel(c2, l2id));
    sk.add_constraint(al, Kind::PointOnCurve(t1, l1id));
    sk.add_constraint(al, Kind::PointOnCurve(t2, l2id));
    let segunda = match pedido.modo {
        ModoChanfro::Iguais => {
            sk.add_constraint(al, Kind::Equal(c1, c2));
            None
        }
        ModoChanfro::DuasDistancias => Some(sk.add_constraint(al, Kind::Length(c2, fmt(r2)))),
        ModoChanfro::DistanciaAngulo => Some(sk.add_constraint(al, Kind::Angle(c1, corte, fmt(pedido.angulo)))),
    };
    let cota = sk.add_constraint(al, Kind::Length(c1, fmt(r1)));
    if !pedido.manter && !sk.curves.iter().any(|c| SketchData::curve_points(&c.geom).contains(&p)) {
        sk.points.retain(|x| x.id != p || x.fixed);
        sk.prune_constraints();
    }
    Ok(Canto { cota, segunda })
}

/// Um canto com a própria cota.
pub(crate) fn um_canto(sk: &mut SketchData, al: &mut impl FnMut() -> Id, p: Id, pedido: &Pedido) -> Result<Canto, &'static str> {
    if pedido.chanfro {
        chamfer_corner(sk, al, p, pedido)
    } else {
        fillet_corner(sk, al, p, pedido)
    }
}

fn kind_de(sk: &SketchData, id: Id) -> Option<Kind> {
    sk.constraints.iter().find(|x| x.id == id).map(|x| x.kind.clone())
}

/// Medida atual de uma cota.
pub(crate) fn medida(sk: &SketchData, id: Id) -> Option<f64> {
    sk.measure(&kind_de(sk, id)?)
}

/// Curva medida pela cota de raio ou comprimento: é ela que os cantos seguintes igualam.
fn curva_da(k: &Kind) -> Option<Id> {
    match k {
        Kind::Radius(c, _) | Kind::Length(c, _) => Some(*c),
        _ => None,
    }
}

/// Filete ou chanfro em vários cantos, com o mesmo valor. O primeiro leva a cota; os outros ficam
/// iguais a ele por restrição, então editar uma cota muda a cadeia toda. `base`: o canto que já existe
/// e comanda os novos. Canto em que o valor não cabe fica como estava. Devolve o canto que comanda e
/// quantos sobraram. No chanfro por ângulo cada canto guarda a sua cota de ângulo, porque a restrição
/// de igualdade compara comprimentos e raios.
pub(crate) fn em_lote(sk: &mut SketchData, al: &mut impl FnMut() -> Id, cantos: &[Id], pedido: &Pedido, base: Option<Canto>) -> Result<(Canto, usize), &'static str> {
    let (mestre, resto) = match base {
        Some(m) => (m, cantos),
        None => {
            let (p, r) = cantos.split_first().ok_or("não há canto entre duas linhas na seleção")?;
            (um_canto(sk, al, *p, pedido)?, r)
        }
    };
    let k0 = kind_de(sk, mestre.cota).ok_or("a cota do canto sumiu")?;
    let (Some(valor), Some(curva)) = (sk.measure(&k0), curva_da(&k0)) else { return Err("a cota do canto sumiu") };
    let segunda = mestre.segunda.and_then(|id| kind_de(sk, id));
    let medida2 = segunda.as_ref().and_then(|k| sk.measure(k));
    let curva2 = segunda.as_ref().and_then(curva_da);
    let igual = Pedido {
        valor: Some(valor),
        segundo: medida2.filter(|_| curva2.is_some()).or(pedido.segundo),
        angulo: medida2.filter(|_| curva2.is_none()).unwrap_or(pedido.angulo),
        ..*pedido
    };
    let bate = |g: &SketchData, id: Option<Id>, m: Option<f64>| match (id, m) {
        (Some(id), Some(m)) => medida(g, id).is_some_and(|x| (x - m).abs() < 1e-6),
        _ => true,
    };
    let mut sobra = 0;
    for p in resto {
        // tenta numa cópia: o recuo encolhe o valor que não cabe, e aqui ele tem de ser igual
        let mut g = sk.clone();
        let feito = um_canto(&mut g, al, *p, &igual).ok().filter(|c| bate(&g, Some(c.cota), Some(valor)) && bate(&g, c.segunda, medida2));
        let Some(c) = feito else {
            sobra += 1;
            continue;
        };
        let Some(nova) = kind_de(&g, c.cota).as_ref().and_then(curva_da) else { continue };
        g.constraints.retain(|x| x.id != c.cota);
        g.add_constraint(al, Kind::Equal(curva, nova));
        if let (Some(m2), Some(id2)) = (curva2, c.segunda) {
            if let Some(n2) = kind_de(&g, id2).as_ref().and_then(curva_da) {
                g.constraints.retain(|x| x.id != id2);
                g.add_constraint(al, Kind::Equal(m2, n2));
            }
        }
        *sk = g;
    }
    Ok((mestre, sobra))
}

/// Um canto, seguindo `mestre` se houver: aí não nasce cota, e o valor que não couber é recusado.
pub(crate) fn canto_seguinte(sk: &mut SketchData, al: &mut impl FnMut() -> Id, p: Id, pedido: &Pedido, mestre: Option<Canto>) -> Result<Canto, &'static str> {
    match em_lote(sk, al, &[p], pedido, mestre)? {
        (_, 1..) => Err("este canto não comporta esse valor"),
        (c, _) => Ok(c),
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::sketch_ui::{changed_curves, ghost_ids};
    use std::collections::HashMap;

    /// Ids sem repetição entre alocadores: o teste monta o sketch num e continua em outro.
    fn ids() -> impl FnMut() -> Id {
        static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1000);
        || Id(N.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
    }

    /// Recalcula o sketch extrudado em `h`: status bom, volume e tamanho da caixa envolvente.
    pub(crate) fn extruda(data: SketchData, h: f64) -> (bool, f64, [f64; 3]) {
        use fenkra_core::*;
        let mut doc = Document::new("Teste");
        let sketch = doc.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
        let k = FeatureKind::Extrude(Extrude { sketch, op: BoolOp::Add, extent: Extent::Blind(h.to_string()), reverse: false, taper: "0".into(), thin: None, thin_side: 0, dir: None, ends: Default::default() });
        let r = fenkra_solidos::previa_comando(&doc, k);
        let ok = matches!(r.status, fenkra_solidos::Status::Ok);
        let Some(m) = r.malha else { return (false, 0., [0.; 3]) };
        let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
        for q in m.positions.chunks_exact(3) {
            for i in 0..3 {
                lo[i] = lo[i].min(q[i] as f64);
                hi[i] = hi[i].max(q[i] as f64);
            }
        }
        (ok, r.volume, [hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]])
    }

    /// Duas linhas, com o canto em (0, 0).
    fn canto() -> (SketchData, Id, Id, Id) {
        let (mut sk, mut al) = (SketchData::default(), ids());
        let (a, p, q) = (sk.add_point(&mut al, 40., 0.), sk.add_point(&mut al, 0., 0.), sk.add_point(&mut al, 0., 30.));
        let l1 = sk.add_curve(&mut al, Curve::Line { a, b: p }, false);
        let l2 = sk.add_curve(&mut al, Curve::Line { a: p, b: q }, false);
        (sk, l1, l2, p)
    }

    /// Retângulo de 40 por 30 totalmente restrito, com o canto `p[0]` preso. As cotas vão do canto até
    /// os lados opostos, então continuam medindo o mesmo depois que os lados encurtam.
    fn retangulo() -> (SketchData, [Id; 4], [Id; 4]) {
        let (mut sk, mut al) = (SketchData::default(), ids());
        let (p, l) = sk.add_rect(&mut al, [0., 0.], [40., 30.], false);
        sk.add_constraint(&mut al, Kind::Fixed(p[0]));
        sk.add_constraint(&mut al, Kind::PointLineDistance(p[0], l[1], "40".into()));
        sk.add_constraint(&mut al, Kind::PointLineDistance(p[0], l[2], "30".into()));
        (sk, p, l)
    }

    /// O mesmo retângulo, com largura e altura cotadas no comprimento dos lados que chegam a `p[1]`.
    fn retangulo_de_comprimentos() -> (SketchData, [Id; 4]) {
        let (mut sk, mut al) = (SketchData::default(), ids());
        let (p, l) = sk.add_rect(&mut al, [0., 0.], [40., 30.], false);
        sk.add_constraint(&mut al, Kind::Fixed(p[0]));
        sk.add_constraint(&mut al, Kind::Length(l[0], "40".into()));
        sk.add_constraint(&mut al, Kind::Length(l[1], "30".into()));
        (sk, p)
    }

    fn caixa(sk: &SketchData) -> [f64; 4] {
        let mut b = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];
        for c in sk.curves.iter().filter(|c| !c.construction && !matches!(c.geom, Curve::Point { .. })) {
            for q in crate::sketch_ui::curve_uv(sk, c) {
                b = [b[0].min(q[0]), b[1].min(q[1]), b[2].max(q[0]), b[3].max(q[1])];
            }
        }
        b
    }

    /// Área do contorno, pelo polígono que o sombreado usa.
    fn area(sk: &SketchData) -> f64 {
        let q = crate::sketch_sombra::poligono(&sk.loops().0[0]);
        let n = q.len();
        ((0..n).map(|i| q[i][0] * q[(i + 1) % n][1] - q[(i + 1) % n][0] * q[i][1]).sum::<f64>() / 2.).abs()
    }

    #[test]
    fn filete_aceita_a_linha_e_repete_o_raio() {
        let (sk, l1, _, p) = canto();
        // clicar na linha perto do canto resolve para o canto; longe, a ponta solta explica a recusa
        assert_eq!(corner_of(&sk, Ent::Cv(l1), [3., 0.]), Some(Ok(p)));
        assert!(matches!(corner_of(&sk, Ent::Cv(l1), [38., 0.]), Some(Err(_))));
        let mut g = sk.clone();
        let c = um_canto(&mut g, &mut ghost_ids(), p, &Pedido::filete(Some(5.))).unwrap();
        assert!((medida(&g, c.cota).unwrap() - 5.).abs() < 1e-9);
        assert!(g.solve(&HashMap::new(), &[]).converged);
        assert!(g.point(p).is_none(), "o canto antigo saiu");
        let gh = changed_curves(&sk, &g);
        assert_eq!(gh.len(), 3, "arco e as duas linhas encurtadas");
        // sem valor a repetir, 30% do lado menor; com valor grande demais, sobra linha
        let mut g = sk.clone();
        um_canto(&mut g, &mut ghost_ids(), p, &Pedido::chanfro(None)).unwrap();
        assert!(g.solve(&HashMap::new(), &[]).converged);
        assert!((corner_setback(None, 40., 30.) - 9.).abs() < 1e-9);
        assert!((corner_setback(Some(500.), 40., 30.) - 27.).abs() < 1e-9);
    }

    #[test]
    fn filete_e_chanfro_percorrem_a_cadeia_com_um_valor_so() {
        for pedido in [Pedido::filete(Some(5.)), Pedido::chanfro(Some(5.))] {
            let mut al = ids();
            let mut sk = SketchData::default();
            let (_, rect) = sk.add_rect(&mut al, [0., 0.], [40., 30.], false);
            let cantos = crate::sketch_cadeia::cantos(&sk, &rect);
            assert_eq!(cantos.len(), 4);
            let (dim, sobra) = em_lote(&mut sk, &mut al, &cantos, &pedido, None).unwrap();
            assert_eq!(sobra, 0);
            let iguais = sk.constraints.iter().filter(|k| matches!(k.kind, Kind::Equal(..))).count();
            assert!(iguais >= 3, "{iguais}");
            assert_eq!(sk.constraints.iter().filter(|k| matches!(k.kind, Kind::Radius(..))).count(), usize::from(!pedido.chanfro));
            let (loops, _) = sk.loops();
            assert_eq!(loops.len(), 1, "o contorno segue fechado");
            // uma cota só comanda os quatro cantos
            let k = sk.constraints.iter_mut().find(|x| x.id == dim.cota).unwrap();
            *k.kind.value_mut().unwrap() = "3".into();
            assert!(sk.solve(&HashMap::new(), &[]).converged);
            for c in sk.curves.iter().filter(|c| matches!(c.geom, Curve::Arc { .. })) {
                assert!((sk.radius_of(c.id).unwrap() - 3.).abs() < 1e-6);
            }
        }
    }

    #[test]
    fn canto_em_que_o_valor_nao_cabe_fica_como_estava() {
        let mut al = ids();
        let mut sk = SketchData::default();
        let (_, rect) = sk.add_rect(&mut al, [0., 0.], [40., 10.], false);
        let cantos = crate::sketch_cadeia::cantos(&sk, &rect);
        // 8 de recuo em cada ponta do lado de 10 não cabem: o segundo canto desse lado é recusado
        let (_, sobra) = em_lote(&mut sk, &mut al, &cantos, &Pedido::filete(Some(8.)), None).unwrap();
        assert!(sobra >= 1, "{sobra}");
    }

    #[test]
    fn duas_linhas_selecionadas_dao_o_canto_do_filete() {
        let (sk, l1, l2, p) = canto();
        assert_eq!(corner_da_selecao(&sk, &[Ent::Cv(l1), Ent::Cv(l2)]), Some(p));
        assert_eq!(corner_da_selecao(&sk, &[Ent::Cv(l1)]), None);
    }

    /// Filetes dados um a um: cada clique depois do primeiro segue o mestre.
    #[test]
    fn filetes_seguidos_ficam_iguais_ao_primeiro_com_uma_cota_so() {
        let (mut sk, p, _) = retangulo();
        let mut al = ids();
        let pedido = Pedido::filete(Some(5.));
        let mestre = canto_seguinte(&mut sk, &mut al, p[2], &pedido, None).unwrap();
        for q in [p[1], p[3]] {
            assert_eq!(canto_seguinte(&mut sk, &mut al, q, &pedido, Some(mestre)).unwrap(), mestre);
        }
        assert_eq!(sk.constraints.iter().filter(|k| matches!(k.kind, Kind::Radius(..))).count(), 1, "uma cota só");
        let r = sk.solve(&HashMap::new(), &[]);
        assert!(r.converged && r.dof == 0 && r.failing.is_empty(), "{r:?}");
        // a cota do primeiro comanda os outros, e a caixa não muda
        *sk.constraints.iter_mut().find(|x| x.id == mestre.cota).unwrap().kind.value_mut().unwrap() = "7".into();
        assert!(sk.solve(&HashMap::new(), &[]).converged);
        let arcos: Vec<Id> = sk.curves.iter().filter(|c| matches!(c.geom, Curve::Arc { .. })).map(|c| c.id).collect();
        assert_eq!(arcos.len(), 3);
        assert!(arcos.iter().all(|a| (sk.radius_of(*a).unwrap() - 7.).abs() < 1e-6));
        let b = caixa(&sk);
        assert!(b[0].abs() < 1e-6 && b[1].abs() < 1e-6 && (b[2] - 40.).abs() < 1e-6 && (b[3] - 30.).abs() < 1e-6, "{b:?}");
        // três cantos arredondados: tira (1 - pi/4) r2 de cada um
        let a = 1200. - 3. * (1. - std::f64::consts::FRAC_PI_4) * 49.;
        assert!((area(&sk) - a).abs() < 0.2, "{} {a}", area(&sk));
    }

    #[test]
    fn filetes_seguidos_e_chanfro_de_duas_medidas_viram_solido() {
        let (mut sk, p, _) = retangulo();
        let mut al = ids();
        let pedido = Pedido::filete(Some(5.));
        let mestre = canto_seguinte(&mut sk, &mut al, p[2], &pedido, None).unwrap();
        canto_seguinte(&mut sk, &mut al, p[3], &pedido, Some(mestre)).unwrap();
        // chanfro de 6 por 3 no canto de baixo à direita: tira um triângulo de área 9
        um_canto(&mut sk, &mut al, p[1], &Pedido { modo: ModoChanfro::DuasDistancias, segundo: Some(3.), ..Pedido::chanfro(Some(6.)) }).unwrap();
        let r = sk.solve(&HashMap::new(), &[]);
        assert!(r.converged && r.failing.is_empty(), "{r:?}");
        let (ok, volume, caixa) = extruda(sk, 10.);
        assert!(ok);
        let area = 1200. - 2. * (1. - std::f64::consts::FRAC_PI_4) * 25. - 9.;
        assert!((volume - area * 10.).abs() < 1e-2, "{volume} {}", area * 10.);
        assert!((caixa[0] - 40.).abs() < 1e-3 && (caixa[1] - 30.).abs() < 1e-3 && (caixa[2] - 10.).abs() < 1e-3, "{caixa:?}");
    }

    #[test]
    fn canto_seguinte_recusa_o_valor_que_nao_cabe_sem_mexer_no_sketch() {
        let (mut sk, p, _) = retangulo();
        let mut al = ids();
        let pedido = Pedido::filete(Some(15.));
        let mestre = canto_seguinte(&mut sk, &mut al, p[2], &pedido, None).unwrap();
        // o lado de 30 ficou com 15: o recuo de 15 o consumiria todo
        let antes = sk.clone();
        assert!(canto_seguinte(&mut sk, &mut al, p[1], &pedido, Some(mestre)).is_err());
        assert_eq!(sk, antes);
    }

    #[test]
    fn chanfro_de_duas_distancias_mede_cada_linha_e_troca_de_lado_pela_linha_do_cursor() {
        let (sk, l1, l2, p) = canto();
        let pedido = Pedido { modo: ModoChanfro::DuasDistancias, segundo: Some(3.), ..Pedido::chanfro(Some(8.)) };
        for (primeira, x, y) in [(l1, 8., 3.), (l2, 3., 8.)] {
            let (mut g, mut al) = (sk.clone(), ids());
            let c = um_canto(&mut g, &mut al, p, &Pedido { primeira: Some(primeira), ..pedido }).unwrap();
            assert!(g.solve(&HashMap::new(), &[]).converged);
            assert!((medida(&g, c.cota).unwrap() - 8.).abs() < 1e-9);
            assert!((medida(&g, c.segunda.unwrap()).unwrap() - 3.).abs() < 1e-9);
            assert!(g.points.iter().any(|q| (q.x - x).abs() < 1e-9 && q.y.abs() < 1e-9), "{x} na linha de baixo");
            assert!(g.points.iter().any(|q| q.x.abs() < 1e-9 && (q.y - y).abs() < 1e-9), "{y} na linha de cima");
        }
        assert_eq!(primeira_linha(&sk, Ent::Cv(l2), p, [0., 2.]), Some(l2));
        assert_eq!(primeira_linha(&sk, Ent::Pt(p), p, [3., 0.5]), Some(l1), "no ponto, a linha mais perto do cursor");
        assert_eq!(primeira_linha(&sk, Ent::Pt(p), p, [0.5, 3.]), Some(l2));
    }

    #[test]
    fn chanfro_de_distancia_e_angulo_segue_a_lei_dos_senos() {
        let (mut sk, l1, l2, p) = canto();
        let mut al = ids();
        // as linhas ficam no lugar, para a conta falar só do corte
        sk.add_constraint(&mut al, Kind::Horizontal(l1));
        sk.add_constraint(&mut al, Kind::Vertical(l2));
        for (l, ponta) in [(l1, 0), (l2, 1)] {
            let (a, b) = line_ends(&sk, l).unwrap();
            sk.add_constraint(&mut al, Kind::Fixed([a, b][ponta]));
        }
        // canto reto: 30 graus na primeira linha e 10 de recuo dão 10 tan(30) na outra
        let pedido = Pedido { modo: ModoChanfro::DistanciaAngulo, angulo: 30., primeira: Some(l1), ..Pedido::chanfro(Some(10.)) };
        let (mut g, mut al) = (sk.clone(), ids());
        let c = um_canto(&mut g, &mut al, p, &pedido).unwrap();
        assert!(g.solve(&HashMap::new(), &[]).converged);
        let y = 10. * 30f64.to_radians().tan();
        assert!(g.points.iter().any(|q| q.x.abs() < 1e-9 && (q.y - y).abs() < 1e-6), "{:?}", g.points);
        assert!((medida(&g, c.segunda.unwrap()).unwrap() - 30.).abs() < 1e-6, "a cota do ângulo mede o que foi pedido");
        // a cota comanda: 45 graus igualam os recuos
        *g.constraints.iter_mut().find(|x| x.id == c.segunda.unwrap()).unwrap().kind.value_mut().unwrap() = "45".into();
        assert!(g.solve(&HashMap::new(), &[]).converged);
        assert!(g.points.iter().any(|q| q.x.abs() < 1e-6 && (q.y - 10.).abs() < 1e-5), "{:?}", g.points);
        // 90 graus de canto mais 90 de corte nunca se encontram
        let (mut g, mut al) = (sk.clone(), ids());
        assert!(um_canto(&mut g, &mut al, p, &Pedido { angulo: 95., ..pedido }).is_err());
        // o que não cabe encolhe nas duas, mantendo o ângulo
        let (mut g, mut al) = (sk.clone(), ids());
        let c = um_canto(&mut g, &mut al, p, &Pedido { valor: Some(500.), ..pedido }).unwrap();
        assert!((medida(&g, c.cota).unwrap() - 36.).abs() < 1e-9, "90% da primeira linha");
        assert!(g.solve(&HashMap::new(), &[]).converged);
    }

    #[test]
    fn chanfros_de_duas_medidas_em_lote_tem_uma_cota_de_cada() {
        let mut al = ids();
        let mut sk = SketchData::default();
        let (_, rect) = sk.add_rect(&mut al, [0., 0.], [40., 30.], false);
        let cantos = crate::sketch_cadeia::cantos(&sk, &rect);
        let pedido = Pedido { modo: ModoChanfro::DuasDistancias, segundo: Some(3.), ..Pedido::chanfro(Some(5.)) };
        let (m, sobra) = em_lote(&mut sk, &mut al, &cantos, &pedido, None).unwrap();
        assert_eq!(sobra, 0);
        assert_eq!(sk.constraints.iter().filter(|k| matches!(k.kind, Kind::Length(..))).count(), 2, "uma cota por medida");
        assert!(sk.solve(&HashMap::new(), &[]).converged);
        *sk.constraints.iter_mut().find(|x| x.id == m.segunda.unwrap()).unwrap().kind.value_mut().unwrap() = "4".into();
        assert!(sk.solve(&HashMap::new(), &[]).converged);
        let quatro = sk.curves.iter().filter(|c| c.construction).filter(|c| {
            let Curve::Line { a, b } = c.geom else { return false };
            (d2(sk.pos(a), sk.pos(b)) - 4.).abs() < 1e-6
        });
        assert_eq!(quatro.count(), 4, "a segunda medida vale nos quatro cantos");
    }

    /// Com o canto mantido, a cota presa nele e as cotas de comprimento continuam valendo.
    #[test]
    fn manter_canto_conserva_as_cotas_do_canto() {
        for chanfro in [false, true] {
            let (mut sk, p) = retangulo_de_comprimentos();
            let mut al = ids();
            let furo = sk.add_point(&mut al, 55., 15.);
            sk.add_curve(&mut al, Curve::Point { p: furo }, false);
            let preso = sk.add_constraint(&mut al, Kind::DistanceX(p[1], furo, "15".into()));
            sk.add_constraint(&mut al, Kind::DistanceY(p[1], furo, "15".into()));
            let pedido = Pedido { manter: true, ..if chanfro { Pedido::chanfro(Some(5.)) } else { Pedido::filete(Some(5.)) } };
            um_canto(&mut sk, &mut al, p[1], &pedido).unwrap();
            assert!(sk.point(p[1]).is_some(), "o canto ficou");
            assert!(sk.constraints.iter().any(|c| c.id == preso), "a cota do canto ficou");
            let r = sk.solve(&HashMap::new(), &[]);
            assert!(r.converged && r.failing.is_empty(), "{r:?}");
            assert_eq!(r.dof, 0, "{chanfro}: {r:?}");
            // os lados cotados continuam medindo até o canto: a largura muda e a caixa e o furo seguem
            let b = caixa(&sk);
            assert!((b[2] - 40.).abs() < 1e-6 && (b[3] - 30.).abs() < 1e-6, "{b:?}");
            let cota = sk.constraints.iter_mut().find(|c| matches!(&c.kind, Kind::Distance(_, _, v) if v == "40")).expect("comprimento virou distância até o canto");
            *cota.kind.value_mut().unwrap() = "60".into();
            let r = sk.solve(&HashMap::new(), &[]);
            assert!(r.converged && r.failing.is_empty(), "{r:?}");
            assert!((sk.pos(p[1])[0] - 60.).abs() < 1e-6 && sk.pos(p[1])[1].abs() < 1e-6, "{:?}", sk.pos(p[1]));
            assert!((sk.pos(furo)[0] - 75.).abs() < 1e-6, "{:?}", sk.pos(furo));
            let b = caixa(&sk);
            assert!((b[2] - 60.).abs() < 1e-6 && (b[3] - 30.).abs() < 1e-6, "{b:?}");
            assert_eq!(sk.loops().0.len(), 1, "o ponto de construção não entra no perfil");
            assert_eq!(sk.hole_points().len(), 1, "nem vira furo");
        }
    }

    #[test]
    fn sem_manter_o_canto_sai_com_as_suas_cotas() {
        let (mut sk, p, _) = retangulo();
        let mut al = ids();
        let furo = sk.add_point(&mut al, 25., 15.);
        let preso = sk.add_constraint(&mut al, Kind::DistanceX(p[2], furo, "15".into()));
        um_canto(&mut sk, &mut al, p[2], &Pedido::filete(Some(5.))).unwrap();
        assert!(sk.point(p[2]).is_none());
        assert!(!sk.constraints.iter().any(|c| c.id == preso));
    }
}
