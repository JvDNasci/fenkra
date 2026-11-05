// SPDX-License-Identifier: LGPL-2.1-or-later
//! Cota inteligente na viewport (B268): do que foi escolhido (e do que está sob o cursor) sai o tipo
//! de cota. A regra vive em `fenkra_core::pmi_infer`; aqui ficam a leitura dos elementos da cena, o
//! estado do Tab e o texto que acompanha o cursor.

use crate::i18n::tr;
use crate::picking::Pick;
use eframe::egui::Color32;
use fenkra_core::pmi::{PmiKind, TargetClass};
use fenkra_core::pmi_infer::{angle_role, Inference, Item, Reason, Verdict};
use fenkra_solidos::RecomputeResult;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Índice do comando da barra que infere o tipo.
pub const SMART: u8 = 17;

/// Quantas vezes o Tab foi apertado no comando aberto.
static TAB_N: AtomicUsize = AtomicUsize::new(0);

pub fn tab_reset() {
    TAB_N.store(0, Ordering::Relaxed);
}

pub fn tab_next() {
    TAB_N.fetch_add(1, Ordering::Relaxed);
}

pub fn tab_n() -> usize {
    TAB_N.load(Ordering::Relaxed)
}

/// Elemento escolhido para a inferência: alvo e, se for aresta reta, as pontas.
pub fn item_of(r: &RecomputeResult, p: &Pick) -> Option<Item> {
    let t = crate::pmi_ui::pick_target(r, p)?;
    let seg = match (*p, t.class) {
        (Pick::Edge { body, edge, .. }, TargetClass::Aresta) => {
            let pts = crate::picking::edge_polyline(r, body, edge);
            straight(&pts)
        }
        _ => None,
    };
    Some(Item { t, seg })
}

/// Pontas da polilinha se ela é reta (desvio máximo de 0,1 % do comprimento).
fn straight(pts: &[[f64; 3]]) -> Option<([f64; 3], [f64; 3])> {
    let (a, b) = (*pts.first()?, *pts.last()?);
    let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let l = (u[0] * u[0] + u[1] * u[1] + u[2] * u[2]).sqrt();
    if l < 1e-9 {
        return None;
    }
    let ok = pts.iter().all(|p| {
        let w = [p[0] - a[0], p[1] - a[1], p[2] - a[2]];
        let c = [u[1] * w[2] - u[2] * w[1], u[2] * w[0] - u[0] * w[2], u[0] * w[1] - u[1] * w[0]];
        (c[0] * c[0] + c[1] * c[1] + c[2] * c[2]).sqrt() / l <= l * 1e-3
    });
    ok.then_some((a, b))
}

pub fn items(r: &RecomputeResult, picks: &[Pick]) -> Vec<Item> {
    picks.iter().filter_map(|p| item_of(r, p)).collect()
}

/// Tipo escolhido para a seleção, já com o Tab aplicado; `Linear` enquanto nada se decide.
pub fn kind_for(inf: &Inference) -> PmiKind {
    inf.kind(tab_n()).unwrap_or(PmiKind::Linear)
}

/// Linhas de texto junto ao cursor: tipo reconhecido (com a cor de validade) e o papel do
/// próximo clique. `n` é quantos elementos já estão escolhidos (sem o que está sob o cursor).
pub fn cursor_lines(inf: &Inference, n: usize) -> Vec<(String, Color32)> {
    let ok = crate::theme::st_ok();
    let warn = crate::theme::st_warn();
    let err = crate::theme::st_err();
    let mut v = vec![];
    match inf.verdict {
        Verdict::Empty => v.push((tr("aponte para um elemento: o tipo da cota sai da escolha").to_string(), warn)),
        Verdict::Pending => v.push((tr("escolha o segundo elemento").to_string(), warn)),
        Verdict::Provisional | Verdict::Complete => {
            let k = kind_for(inf);
            v.push((format!("{}: {}", tr("Cota reconhecida"), tr(&k.variant_label())), ok));
            if inf.variants.len() > 1 {
                v.push((tr("Tab alterna a variante").to_string(), warn));
            }
            if inf.verdict == Verdict::Provisional {
                v.push((tr("Enter confirma; outro clique muda o tipo").to_string(), warn));
            }
            if k == PmiKind::Angle {
                v.push((format!("{} {}", tr("próximo:"), tr(angle_role(n.min(2)))), warn));
            }
        }
        Verdict::Invalid(r) => {
            v.push((tr(r.message()).to_string(), err));
            if let Reason::AngleDegenerate { vertex } = r {
                v.push((format!("{}: {}º {}", tr(angle_role(0)), vertex + 1, tr("clique")), err));
            }
        }
    }
    v
}

/// Papel do clique `k` quando a cota é angular, para a legenda do cursor do comando Ângulo.
pub fn angle_step_lines(n_picked: usize, bad: Option<Reason>) -> Vec<(String, Color32)> {
    let mut v = vec![(format!("{}: {}", tr("Clique"), tr(angle_role(n_picked.min(2)))), crate::theme::st_ok())];
    if let Some(r) = bad {
        v.push((tr(r.message()).to_string(), crate::theme::st_err()));
        v.push((format!("{}: {}º {}", tr(angle_role(0)), 1, tr("clique")), crate::theme::st_err()));
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aresta_reta_so_vale_se_for_reta() {
        assert!(straight(&[[0., 0., 0.], [5., 0., 0.], [10., 0., 0.]]).is_some());
        assert!(straight(&[[0., 0., 0.], [5., 3., 0.], [10., 0., 0.]]).is_none());
        assert!(straight(&[[1., 1., 1.]]).is_none());
    }

    #[test]
    fn legenda_do_cursor_diz_vertice_e_pontos_do_angulo() {
        let l = angle_step_lines(0, None);
        assert!(l[0].0.contains("vértice"));
        assert!(angle_step_lines(1, None)[0].0.contains("ponto 1"));
        assert!(angle_step_lines(2, None)[0].0.contains("ponto 2"));
        let l = angle_step_lines(2, Some(Reason::AngleDegenerate { vertex: 0 }));
        assert!(l.iter().any(|(t, _)| t.contains("1º")));
    }
}

#[cfg(test)]
mod tests_selecao_real {
    use super::*;
    use fenkra_core::pmi_infer::infer;
    use fenkra_core::pmi_infer::Reason;

    fn exemplo(nome: &str) -> RecomputeResult {
        let e = crate::examples::all().into_iter().find(|e| e.name == nome).unwrap();
        fenkra_solidos::recompute(&e.doc)
    }

    fn faces(r: &RecomputeResult, classe: TargetClass) -> Vec<Pick> {
        let mut v = vec![];
        for (bi, b) in r.bodies.iter().enumerate() {
            for f in 0..b.mesh.face_info.len() {
                let p = Pick::Face { body: bi, face: f, owner: fenkra_core::Id(1), point: [0.; 3], normal: [0.; 3] };
                if crate::pmi_ui::pick_target(r, &p).is_some_and(|t| t.class == classe) {
                    v.push(p);
                }
            }
        }
        v
    }

    fn vertices(r: &RecomputeResult) -> Vec<Pick> {
        r.bodies.iter().enumerate().flat_map(|(bi, b)| b.mesh.vertices.chunks_exact(3).map(move |c| Pick::Vertex { body: bi, pos: [c[0], c[1], c[2]] })).collect()
    }

    fn kind_tab(inf: &Inference, n: usize) -> PmiKind {
        inf.kind(n).unwrap()
    }

    #[test]
    fn furo_do_flange_vira_diametro_e_dois_furos_viram_distancia_entre_eixos() {
        let r = exemplo("Flange");
        let furos: Vec<Pick> = faces(&r, TargetClass::FaceCilindrica).into_iter().filter(|p| crate::pmi_ui::pick_target(&r, p).is_some_and(|t| t.concave && (t.radius - 4.).abs() < 0.1)).collect();
        assert!(furos.len() >= 2, "o flange tem furação em círculo: {}", furos.len());
        let um = infer(&items(&r, &furos[..1]));
        assert_eq!(um.verdict, Verdict::Provisional);
        assert_eq!(kind_tab(&um, 0), PmiKind::Diameter);
        assert_eq!(kind_tab(&um, 1), PmiKind::Radius);
        let dois = infer(&items(&r, &furos[..2]));
        assert_eq!(dois.verdict, Verdict::Complete);
        assert!(matches!(kind_tab(&dois, 0), PmiKind::Gap { .. }));
    }

    #[test]
    fn faces_paralelas_dao_distancia_e_perpendiculares_sao_recusadas() {
        let r = exemplo("Placa simples");
        let planas = faces(&r, TargetClass::FacePlana);
        let dir = |p: &Pick| crate::pmi_ui::pick_target(&r, p).unwrap().dir;
        let (mut paralelo, mut obliquo) = (None, None);
        for (i, a) in planas.iter().enumerate() {
            for b in &planas[i + 1..] {
                let d: f64 = dir(a).iter().zip(dir(b)).map(|(x, y)| x * y).sum();
                if d.abs() > 0.9999 {
                    paralelo.get_or_insert([*a, *b]);
                } else if d.abs() < 0.01 {
                    obliquo.get_or_insert([*a, *b]);
                }
            }
        }
        let inf = infer(&items(&r, &paralelo.expect("a placa tem faces paralelas")));
        assert_eq!((inf.verdict, kind_tab(&inf, 0)), (Verdict::Complete, PmiKind::Linear));
        let inf = infer(&items(&r, &obliquo.expect("a placa tem faces perpendiculares")));
        assert_eq!(inf.verdict, Verdict::Invalid(Reason::PlanesNotParallel));
    }

    #[test]
    fn tres_vertices_viram_angulo_e_repetidos_dizem_qual_clique_e_o_vertice() {
        let r = exemplo("Placa simples");
        let v = vertices(&r);
        assert!(v.len() >= 8);
        let dois = infer(&items(&r, &v[..2]));
        assert_eq!(dois.verdict, Verdict::Provisional);
        assert_eq!(kind_tab(&dois, 0), PmiKind::Linear);
        // um canto e dois vizinhos dele em ângulo reto
        let pos = |p: &Pick| match *p {
            Pick::Vertex { pos, .. } => pos.map(|x| x as f64),
            _ => unreachable!(),
        };
        let mut achou = false;
        'o: for a in &v {
            for b in &v {
                for c in &v {
                    let (pa, pb, pc) = (pos(a), pos(b), pos(c));
                    let (u, w) = ([pb[0] - pa[0], pb[1] - pa[1], pb[2] - pa[2]], [pc[0] - pa[0], pc[1] - pa[1], pc[2] - pa[2]]);
                    let nu = u.iter().map(|x| x * x).sum::<f64>().sqrt();
                    let nw = w.iter().map(|x| x * x).sum::<f64>().sqrt();
                    if nu < 1. || nw < 1. {
                        continue;
                    }
                    let cos = (u[0] * w[0] + u[1] * w[1] + u[2] * w[2]) / (nu * nw);
                    if cos.abs() < 1e-6 {
                        let inf = infer(&items(&r, &[*a, *b, *c]));
                        assert_eq!((inf.verdict, kind_tab(&inf, 0)), (Verdict::Complete, PmiKind::Angle));
                        achou = true;
                        break 'o;
                    }
                }
            }
        }
        assert!(achou);
        let inf = infer(&items(&r, &[v[0], v[0], v[1]]));
        let Verdict::Invalid(rz) = inf.verdict else { panic!("{inf:?}") };
        assert_eq!(rz, Reason::AngleDegenerate { vertex: 0 });
        let linhas = cursor_lines(&inf, 3);
        assert!(linhas.iter().any(|(t, _)| t.contains("vértice")), "{linhas:?}");
    }
}
