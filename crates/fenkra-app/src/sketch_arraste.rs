// SPDX-License-Identifier: LGPL-2.1-or-later
//! Arraste direto no sketch (B272): setas de grau de liberdade e pegas de valor.
//!
//! As setas vêm de `SolveReport::liberdade`, o espaço nulo do Jacobiano que o solver já calcula para
//! contar os graus de liberdade: elas mostram o que o solver realmente deixa, e não um palpite pelo
//! tipo da curva. As pegas mudam o valor de uma cota existente; quem move a geometria continua sendo
//! o solver, então a cota nunca sai do que está desenhado.

use crate::sketch_ui::Ent;
use eframe::egui::{self, Color32, Pos2, Stroke};
use fenkra_core::sketch::{Curve, Kind, Liberdade};
use fenkra_core::{Id, SketchData};

type P = [f64; 2];

fn sub(a: P, b: P) -> P {
    [a[0] - b[0], a[1] - b[1]]
}
fn dot(a: P, b: P) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}
fn len(a: P) -> f64 {
    a[0].hypot(a[1])
}
fn unit(a: P) -> P {
    let l = len(a).max(1e-12);
    [a[0] / l, a[1] / l]
}

/// Seta de movimento possível: onde ela nasce e para que lado aponta (nos dois sentidos).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Seta {
    pub at: P,
    pub dir: P,
}

/// Setas do elemento sob o cursor. Elemento totalmente restrito não tem nenhuma.
pub fn setas(sk: &SketchData, lib: &Liberdade, e: Ent, cursor: Option<P>) -> Vec<Seta> {
    let de_ponto = |p: Id| -> Vec<Seta> { lib.pontos.get(&p).map_or(vec![], |d| d.iter().map(|dir| Seta { at: sk.pos(p), dir: *dir }).collect()) };
    match e {
        Ent::Pt(p) => de_ponto(p),
        Ent::Cs(_) => vec![],
        Ent::Cv(c) => {
            let Some(curva) = sk.curve(c) else { return vec![] };
            let mut v: Vec<Seta> = SketchData::curve_points(&curva.geom).into_iter().flat_map(de_ponto).collect();
            // o raio não é ponto: a seta dele sai da borda, onde o cursor está, na direção do centro
            if let (Curve::Circle { c: ctr, r }, true) = (&curva.geom, lib.raios.contains(&c)) {
                let centro = sk.pos(*ctr);
                let dir = cursor.map_or([1., 0.], |q| unit(sub(q, centro)));
                v.push(Seta { at: [centro[0] + dir[0] * r, centro[1] + dir[1] * r], dir });
            }
            v
        }
    }
}

/// Desenha a seta de duas pontas, com tamanho fixo na tela.
pub fn desenha_seta(painter: &egui::Painter, tela: &dyn Fn(P) -> Pos2, s: Seta, cor: Color32) {
    const META: f32 = 15.;
    const PONTA: f32 = 5.;
    let (o, q) = (tela(s.at), tela([s.at[0] + s.dir[0], s.at[1] + s.dir[1]]));
    let d = (q - o).normalized();
    if d == egui::Vec2::ZERO || !d.x.is_finite() {
        return;
    }
    let st = Stroke::new(1.6, cor);
    let n = d.rot90();
    painter.line_segment([o - d * META, o + d * META], st);
    for lado in [1f32, -1.] {
        let fim = o + d * META * lado;
        painter.line_segment([fim, fim - d * PONTA * lado + n * PONTA * 0.7], st);
        painter.line_segment([fim, fim - d * PONTA * lado - n * PONTA * 0.7], st);
    }
}

/// Como o cursor vira valor da cota.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Regra {
    /// filete: posição na bissetriz a partir do canto `v`. `abas` são as direções das linhas a partir
    /// dele, `meio` a metade do ângulo, `maximo` o maior raio e `pts` as duas pontas e o centro do arco
    Filete { v: P, abas: [P; 2], meio: f64, maximo: f64, pts: [Id; 3] },
    /// distância até a linha que passa por `o` com direção `d`
    Linha { o: P, d: P },
    /// distância até o centro, vezes `k` (raio ou diâmetro)
    Centro { o: P, k: f64 },
}

/// Pega de valor da curva selecionada.
#[derive(Clone, Debug, PartialEq)]
pub struct Pega {
    pub cota: Id,
    pub at: P,
    pub rotulo: &'static str,
    /// a cota guarda uma expressão: o valor dela vem de outro lugar e o arraste não a sobrescreve
    pub trancada: bool,
    /// pontos que ficam parados enquanto o valor muda
    pub apoios: Vec<Id>,
    regra: Regra,
}

fn cota_de(sk: &SketchData, teste: impl Fn(&Kind) -> bool) -> Option<(Id, bool)> {
    sk.constraints.iter().filter(|c| !c.reference).find(|c| teste(&c.kind)).map(|c| {
        let numero = c.kind.clone().value_mut().is_some_and(|t| t.trim().replace(',', ".").parse::<f64>().is_ok());
        (c.id, !numero)
    })
}

/// Linhas tangentes ao arco em cada ponta: (ponta oposta da linha, direção do canto para ela).
fn linhas_do_filete(sk: &SketchData, arco: Id, s: Id, e: Id) -> Option<[(Id, Id); 2]> {
    let tangentes: Vec<Id> = sk.constraints.iter().filter_map(|c| match c.kind {
        Kind::Tangent(l, a) if a == arco => Some(l),
        Kind::Tangent(a, l) if a == arco => Some(l),
        _ => None,
    }).collect();
    let linha_em = |ponta: Id| {
        tangentes.iter().find_map(|l| match sk.curve(*l).map(|c| &c.geom) {
            Some(Curve::Line { a, b }) if *a == ponta => Some((*b, ponta)),
            Some(Curve::Line { a, b }) if *b == ponta => Some((*a, ponta)),
            _ => None,
        })
    };
    Some([linha_em(s)?, linha_em(e)?])
}

/// Filete: arco com cota de raio e tangente às duas linhas que chegam nas pontas.
fn pega_filete(sk: &SketchData, arco: Id) -> Option<Pega> {
    let Curve::Arc { c, s, e } = sk.curve(arco)?.geom else { return None };
    let (cota, trancada) = cota_de(sk, |k| matches!(k, Kind::Radius(x, _) if *x == arco))?;
    let [(l1, t1), (l2, t2)] = linhas_do_filete(sk, arco, s, e)?;
    let (f1, p1, f2, p2) = (sk.pos(l1), sk.pos(t1), sk.pos(l2), sk.pos(t2));
    let (u1, u2) = (unit(sub(f1, p1)), unit(sub(f2, p2)));
    // o canto apagado pelo filete: onde as duas linhas se cruzariam
    let det = u1[0] * u2[1] - u1[1] * u2[0];
    if det.abs() < 1e-9 {
        return None;
    }
    let w = sub(p2, p1);
    let t = (w[0] * u2[1] - w[1] * u2[0]) / det;
    let v = [p1[0] + u1[0] * t, p1[1] + u1[1] * t];
    let (a1, a2) = (unit(sub(f1, v)), unit(sub(f2, v)));
    let meio = dot(a1, a2).clamp(-1., 1.).acos() / 2.;
    Some(Pega {
        cota,
        at: SketchData::arc_mid(sk.pos(c), sk.pos(s), sk.pos(e)),
        rotulo: "raio",
        trancada,
        apoios: vec![l1, l2],
        // a linha mais curta limita o recuo r/tan(θ/2)
        regra: Regra::Filete { v, abas: [a1, a2], meio, maximo: len(sub(f1, v)).min(len(sub(f2, v))) * meio.tan(), pts: [t1, t2, c] },
    })
}

/// Offset: a cota que a ferramenta deixou, ligando a curva à sua origem.
fn pega_offset(sk: &SketchData, c: Id) -> Option<Pega> {
    match sk.curve(c)?.geom {
        Curve::Line { a, b } => {
            let base = sk.constraints.iter().find_map(|k| match k.kind {
                Kind::Parallel(x, y) if y == c => Some(x),
                _ => None,
            })?;
            let (cota, trancada) = cota_de(sk, |k| matches!(k, Kind::PointLineDistance(p, l, _) if *l == base && (*p == a || *p == b)))?;
            let Curve::Line { a: o1, b: o2 } = sk.curve(base)?.geom else { return None };
            let (pa, pb) = (sk.pos(a), sk.pos(b));
            Some(Pega {
                cota,
                at: [(pa[0] + pb[0]) / 2., (pa[1] + pb[1]) / 2.],
                rotulo: "distância",
                trancada,
                apoios: vec![o1, o2],
                regra: Regra::Linha { o: sk.pos(o1), d: unit(sub(sk.pos(o2), sk.pos(o1))) },
            })
        }
        Curve::Circle { c: ctr, r } => {
            // círculo com outro de mesmo centro: o offset compartilha o ponto do centro
            sk.curves.iter().find(|k| k.id != c && matches!(k.geom, Curve::Circle { c: x, .. } | Curve::Arc { c: x, .. } if x == ctr))?;
            let (cota, trancada) = cota_de(sk, |k| matches!(k, Kind::Diameter(x, _) if *x == c))?;
            let o = sk.pos(ctr);
            Some(Pega { cota, at: [o[0] + r, o[1]], rotulo: "diâmetro", trancada, apoios: vec![ctr], regra: Regra::Centro { o, k: 2. } })
        }
        Curve::Arc { c: ctr, s, e } => {
            sk.curves.iter().find(|k| k.id != c && matches!(k.geom, Curve::Circle { c: x, .. } | Curve::Arc { c: x, .. } if x == ctr))?;
            let (cota, trancada) = cota_de(sk, |k| matches!(k, Kind::Radius(x, _) if *x == c))?;
            let o = sk.pos(ctr);
            Some(Pega { cota, at: SketchData::arc_mid(o, sk.pos(s), sk.pos(e)), rotulo: "raio", trancada, apoios: vec![ctr], regra: Regra::Centro { o, k: 1. } })
        }
        _ => None,
    }
}

/// Pega da curva selecionada, se ela for um filete ou um offset. Só com uma curva selecionada: com
/// mais de uma, não há um valor só para mudar.
pub fn pega_da_selecao(sk: &SketchData, sel: &[Ent]) -> Option<Pega> {
    let [Ent::Cv(c)] = sel else { return None };
    pega_filete(sk, *c).or_else(|| pega_offset(sk, *c))
}

/// Valor da cota com a pega em `q`.
pub fn valor_da_pega(p: &Pega, q: P) -> f64 {
    const MINIMO: f64 = 0.01;
    match p.regra {
        Regra::Filete { v, abas, meio, maximo, .. } => {
            // na bissetriz, o centro fica a r/sen(θ/2) do canto e o arco, r antes disso
            let bis = unit([abas[0][0] + abas[1][0], abas[0][1] + abas[1][1]]);
            (dot(sub(q, v), bis) / (1. / meio.sin() - 1.)).clamp(MINIMO, maximo.max(MINIMO))
        }
        Regra::Linha { o, d } => {
            let w = sub(q, o);
            (w[0] * d[1] - w[1] * d[0]).abs().max(MINIMO)
        }
        Regra::Centro { o, k } => (len(sub(q, o)) * k).max(MINIMO),
    }
}

/// Posições exatas da geometria com o valor `r`, quando se sabe calculá-las. O filete precisa disso:
/// com os pontos dele ainda no lugar antigo, o solver prefere girar as linhas a escorregar pelas
/// tangências, e o canto sairia do lugar.
pub fn posicoes_da_pega(p: &Pega, r: f64) -> Vec<(Id, P)> {
    let Regra::Filete { v, abas, meio, pts, .. } = p.regra else { return vec![] };
    let recuo = r / meio.tan();
    let bis = unit([abas[0][0] + abas[1][0], abas[0][1] + abas[1][1]]);
    let ao_longo = |d: P, l: f64| [v[0] + d[0] * l, v[1] + d[1] * l];
    vec![(pts[0], ao_longo(abas[0], recuo)), (pts[1], ao_longo(abas[1], recuo)), (pts[2], ao_longo(bis, r / meio.sin()))]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sketch_ui::{filete_com_raio, offset_ate};
    use fenkra_core::sketch::Curve;
    use std::collections::HashMap;

    fn ids() -> impl FnMut() -> Id {
        let mut n = 100;
        move || {
            n += 1;
            Id(n)
        }
    }

    fn canto(al: &mut impl FnMut() -> Id) -> (SketchData, Id) {
        let mut s = SketchData::default();
        let a = s.add_point(al, 40., 0.);
        let p = s.add_point(al, 0., 0.);
        let b = s.add_point(al, 0., 30.);
        s.add_curve(al, Curve::Line { a, b: p }, false);
        s.add_curve(al, Curve::Line { a: p, b }, false);
        (s, p)
    }

    #[test]
    fn pega_do_filete_devolve_o_raio_no_lugar_onde_esta_e_cresce_para_o_centro() {
        let mut al = ids();
        let (mut s, p) = canto(&mut al);
        let cota = filete_com_raio(&mut s, &mut al, p, 5.);
        let arco = s.curves.iter().find(|c| matches!(c.geom, Curve::Arc { .. })).unwrap().id;
        let pega = pega_da_selecao(&s, &[Ent::Cv(arco)]).expect("filete tem pega");
        assert_eq!(pega.cota, cota);
        assert!(!pega.trancada);
        assert!((valor_da_pega(&pega, pega.at) - 5.).abs() < 1e-6, "{}", valor_da_pega(&pega, pega.at));
        let longe = [pega.at[0] + 3., pega.at[1] + 3.];
        assert!(valor_da_pega(&pega, longe) > 5.);
        // o recuo não passa do tamanho da linha mais curta: em 90°, raio 30
        assert!((valor_da_pega(&pega, [500., 500.]) - 30.).abs() < 1e-6);
        assert!(valor_da_pega(&pega, [-50., -50.]) > 0.);
    }

    #[test]
    fn levar_o_raio_pela_pega_mantem_as_tangencias() {
        let mut al = ids();
        let (mut s, p) = canto(&mut al);
        filete_com_raio(&mut s, &mut al, p, 5.);
        let arco = s.curves.iter().find(|c| matches!(c.geom, Curve::Arc { .. })).unwrap().id;
        let pega = pega_da_selecao(&s, &[Ent::Cv(arco)]).unwrap();
        let vars = HashMap::new();
        let v = valor_da_pega(&pega, [pega.at[0] + 2., pega.at[1] + 2.]);
        *s.constraints.iter_mut().find(|c| c.id == pega.cota).unwrap().kind.value_mut().unwrap() = v.to_string();
        for (id, q) in posicoes_da_pega(&pega, v) {
            let pt = s.point_mut(id).unwrap();
            (pt.x, pt.y) = (q[0], q[1]);
        }
        let rep = s.solve(&vars, &pega.apoios);
        assert!(rep.converged, "{rep:?}");
        assert!((s.radius_of(arco).unwrap() - v).abs() < 1e-3);
        // a nova pega fica sob o cursor e as linhas continuam nos eixos: o canto não saiu do lugar
        let nova = pega_da_selecao(&s, &[Ent::Cv(arco)]).unwrap();
        assert!((nova.at[0] - pega.at[0] - 2.).abs() < 1e-3, "{:?} {:?}", nova.at, pega.at);
        for c in &s.curves {
            if let Curve::Line { a, b } = c.geom {
                let (pa, pb) = (s.pos(a), s.pos(b));
                assert!(pa[0].abs().min(pa[1].abs()) < 1e-6 && pb[0].abs().min(pb[1].abs()) < 1e-6, "linha girou: {pa:?} {pb:?}");
            }
        }
    }

    #[test]
    fn pega_do_offset_de_linha_e_de_circulo() {
        let mut al = ids();
        let mut s = SketchData::default();
        let a = s.add_point(&mut al, 0., 0.);
        let b = s.add_point(&mut al, 40., 0.);
        let base = s.add_curve(&mut al, Curve::Line { a, b }, false);
        let cota = offset_ate(&mut s, &mut al, base, [10., 8.]);
        let nova = s.curves.last().unwrap().id;
        let pega = pega_da_selecao(&s, &[Ent::Cv(nova)]).expect("offset tem pega");
        assert_eq!(pega.cota, cota);
        assert!((valor_da_pega(&pega, pega.at) - 8.).abs() < 1e-6);
        assert!((valor_da_pega(&pega, [33., 12.5]) - 12.5).abs() < 1e-6);
        assert!(pega_da_selecao(&s, &[Ent::Cv(base)]).is_none(), "a origem não é o offset");

        let c = s.add_point(&mut al, 0., 80.);
        let circ = s.add_curve(&mut al, Curve::Circle { c, r: 10. }, false);
        assert!(pega_da_selecao(&s, &[Ent::Cv(circ)]).is_none(), "círculo solto não é offset");
        offset_ate(&mut s, &mut al, circ, [15., 80.]);
        let externo = s.curves.last().unwrap().id;
        let pega = pega_da_selecao(&s, &[Ent::Cv(externo)]).unwrap();
        assert!((valor_da_pega(&pega, pega.at) - 30.).abs() < 1e-6, "o diâmetro no lugar da pega");
    }

    #[test]
    fn cota_com_expressao_fica_trancada() {
        let mut al = ids();
        let (mut s, p) = canto(&mut al);
        let cota = filete_com_raio(&mut s, &mut al, p, 5.);
        *s.constraints.iter_mut().find(|c| c.id == cota).unwrap().kind.value_mut().unwrap() = "L/2".into();
        let arco = s.curves.iter().find(|c| matches!(c.geom, Curve::Arc { .. })).unwrap().id;
        assert!(pega_da_selecao(&s, &[Ent::Cv(arco)]).unwrap().trancada);
    }

    #[test]
    fn seta_some_quando_o_elemento_fica_restrito() {
        let mut al = ids();
        let mut s = SketchData::default();
        let (p, l) = s.add_rect(&mut al, [0., 0.], [30., 20.], false);
        let vars = HashMap::new();
        let lado = Ent::Cv(l[0]);
        assert!(!setas(&s, &s.clone().solve(&vars, &[]).liberdade, lado, None).is_empty());
        s.add_constraint(&mut al, Kind::Fixed(p[0]));
        s.add_constraint(&mut al, Kind::Length(l[0], "30".into()));
        s.add_constraint(&mut al, Kind::Length(l[1], "20".into()));
        let rep = s.solve(&vars, &[]);
        assert_eq!(rep.dof, 0);
        for e in l.iter().map(|c| Ent::Cv(*c)).chain(p.iter().map(|q| Ent::Pt(*q))) {
            assert!(setas(&s, &rep.liberdade, e, None).is_empty(), "{e:?}");
        }
    }

    #[test]
    fn raio_livre_ganha_seta_na_borda_sob_o_cursor() {
        let mut al = ids();
        let mut s = SketchData::default();
        let c = s.add_point(&mut al, 0., 0.);
        s.point_mut(c).unwrap().fixed = true;
        let k = s.add_curve(&mut al, Curve::Circle { c, r: 10. }, false);
        let lib = s.clone().solve(&HashMap::new(), &[]).liberdade;
        let v = setas(&s, &lib, Ent::Cv(k), Some([0., 25.]));
        assert_eq!(v.len(), 1);
        assert!(v[0].at[1] > 9.9 && v[0].dir[1] > 0.99, "{v:?}");
    }
}
