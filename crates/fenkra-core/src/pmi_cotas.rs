// SPDX-License-Identifier: LGPL-2.1-or-later
//! Cotas por par de alvos (B185): projeção ao eixo das cotas ordenadas, cadeia e base.
//!
//! Cadeia e base não são tipos novos: cada trecho é uma cota linear (ou ordenada) comum, com alvos,
//! diagnóstico e tolerância próprios. O que muda é como nascem: a cadeia liga cada ponto ao seguinte
//! na mesma altura; a base parte de uma origem comum e empilha as linhas de cota para não se cruzarem.

use crate::pmi::{Pmi, PmiKind, PmiTarget};
use crate::Id;

/// Uma cota por gerar: âncoras, alvos de cada âncora e deslocamento da linha de cota.
#[derive(Clone, PartialEq, Debug)]
pub struct Trecho {
    pub anchors: Vec<[f64; 3]>,
    pub targets: Vec<Option<PmiTarget>>,
    pub offset: [f64; 3],
}

/// Leva a segunda âncora da cota ordenada para o eixo: a linha de cota fica paralela a X, Y ou Z e o
/// comprimento desenhado é a componente medida. Outros tipos ficam como estão.
pub fn project_to_axis(kind: &PmiKind, a: &mut [[f64; 3]]) {
    if let (PmiKind::Ordinate { axis }, true) = (kind, a.len() >= 2) {
        let k = (*axis).min(2) as usize;
        let mut q = a[0];
        q[k] = a[1][k];
        a[1] = q;
    }
}

fn mult(v: [f64; 3], k: f64) -> [f64; 3] {
    [v[0] * k, v[1] * k, v[2] * k]
}

/// Cadeia: n pontos geram n−1 trechos consecutivos, todos com o mesmo deslocamento.
/// Menos de dois pontos não gera nada.
pub fn chain(points: &[([f64; 3], Option<PmiTarget>)], offset: [f64; 3]) -> Vec<Trecho> {
    points
        .windows(2)
        .map(|w| Trecho { anchors: vec![w[0].0, w[1].0], targets: vec![w[0].1, w[1].1], offset })
        .collect()
}

/// Base: a primeira entrada é a origem comum; cada outro ponto gera um trecho a partir dela, com a
/// linha de cota `step` mais afastada que a anterior ao longo de `dir`, para as linhas empilharem.
pub fn baseline(points: &[([f64; 3], Option<PmiTarget>)], dir: [f64; 3], step: f64) -> Vec<Trecho> {
    let Some((o, ot)) = points.first().copied() else { return vec![] };
    points[1..]
        .iter()
        .enumerate()
        .map(|(i, (p, t))| Trecho { anchors: vec![o, *p], targets: vec![ot, *t], offset: mult(dir, step * (i + 1) as f64) })
        .collect()
}

/// Monta as anotações de um conjunto de trechos, numeradas a partir de `first_id`, todas na mesma
/// pasta (a cadeia e a base se apagam e se movem juntas pela árvore).
pub fn build(kind: &PmiKind, parts: &[Trecho], first_id: u64, name: &str, folder: &str) -> Vec<Pmi> {
    parts
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let mut anchors = t.anchors.clone();
            project_to_axis(kind, &mut anchors);
            Pmi {
                id: Id(first_id + i as u64),
                name: format!("{name} {}", i + 1),
                kind: kind.clone(),
                anchors,
                offset: t.offset,
                text: String::new(),
                tol_plus: String::new(),
                tol_minus: String::new(),
                visible: true,
                plane: None,
                style: Default::default(),
                folder: folder.to_string(),
                targets: t.targets.clone(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pmi::{GapMode, TargetClass};

    fn pt(x: f64, y: f64, z: f64) -> ([f64; 3], Option<PmiTarget>) {
        ([x, y, z], None)
    }

    fn furo(x: f64, y: f64, r: f64) -> PmiTarget {
        PmiTarget { class: TargetClass::FaceCilindrica, point: [x, y, 0.], dir: [0., 0., 1.], radius: r, concave: true }
    }

    fn pmi(kind: PmiKind, targets: Vec<Option<PmiTarget>>) -> Pmi {
        Pmi { id: Id(1), name: "t".into(), kind, anchors: vec![], offset: [0.; 3], text: String::new(), tol_plus: String::new(), tol_minus: String::new(), visible: true, plane: None, style: Default::default(), folder: String::new(), targets }
    }

    #[test]
    fn ordenada_mede_so_o_eixo() {
        let o = [10., 20., 30.];
        let p = [14., 50., 35.];
        for (axis, esperado) in [(0u8, 4.), (1, 30.), (2, 5.)] {
            let pm = pmi(PmiKind::Ordinate { axis }, vec![]);
            assert_eq!(pm.measure(&[o, p]), Some(esperado));
        }
        // a projeção deixa a linha de cota paralela ao eixo e com o mesmo valor
        let mut a = [o, p];
        project_to_axis(&PmiKind::Ordinate { axis: 1 }, &mut a);
        assert_eq!(a[1], [10., 50., 30.]);
        assert_eq!(pmi(PmiKind::Linear, vec![]).measure(&a), Some(30.));
        // o sinal não importa: medir para trás dá o mesmo
        assert_eq!(pmi(PmiKind::Ordinate { axis: 0 }, vec![]).measure(&[p, o]), Some(4.));
        // outros tipos não mudam
        let mut b = [o, p];
        project_to_axis(&PmiKind::Linear, &mut b);
        assert_eq!(b, [o, p]);
    }

    #[test]
    fn cota_entre_furos_tem_tres_medidas() {
        let (a, b) = (furo(0., 0., 3.), furo(40., 30., 5.));
        // centros a 50; interior 50 − 3 − 5; exterior 50 + 3 + 5
        for (mode, esperado) in [(GapMode::Centers, 50.), (GapMode::Inner, 42.), (GapMode::Outer, 58.)] {
            let pm = pmi(PmiKind::Gap { mode }, vec![Some(a), Some(b)]);
            assert!((pm.measure_targets(&[Some(a), Some(b)]).unwrap() - esperado).abs() < 1e-9, "{mode:?}");
        }
        // o desnível ao longo do eixo dos furos não conta
        let alto = PmiTarget { point: [40., 30., 17.], ..furo(40., 30., 5.) };
        let pm = pmi(PmiKind::Gap { mode: GapMode::Centers }, vec![Some(a), Some(alto)]);
        assert!((pm.measure_targets(&[Some(a), Some(alto)]).unwrap() - 50.).abs() < 1e-9);
        // sem alvos circulares cai nas âncoras (centro a centro)
        let pm = pmi(PmiKind::Gap { mode: GapMode::Outer }, vec![]);
        assert_eq!(pm.measure(&[[0., 0., 0.], [3., 4., 0.]]), Some(5.));
        assert_eq!(pm.measure_targets(&[None, None]), None);
        assert_eq!(pm.display(Some(58.)), "58");
    }

    #[test]
    fn comprimento_de_arco() {
        // quarto de volta de raio 10: 10·π/2
        let pm = pmi(PmiKind::ArcLength, vec![]);
        let v = pm.measure(&[[0., 0., 0.], [10., 0., 0.], [0., 10., 0.]]).unwrap();
        assert!((v - 10. * std::f64::consts::FRAC_PI_2).abs() < 1e-9);
        // meia volta: 10·π
        let v = pm.measure(&[[0., 0., 0.], [10., 0., 0.], [-10., 0., 0.]]).unwrap();
        assert!((v - 10. * std::f64::consts::PI).abs() < 1e-9);
        assert_eq!(pm.display(Some(15.71)), "⌒ 15,71");
        assert_eq!(PmiKind::ArcLength.anchors(), 3);
    }

    #[test]
    fn variante_de_medida_com_tab() {
        let mut k = PmiKind::Linear;
        let mut vistos = vec![k.variant_label()];
        for _ in 0..6 {
            k = k.next_variant();
            vistos.push(k.variant_label());
        }
        // linear, X, Y, Z, centro, interior, exterior e de volta ao começo
        assert_eq!(vistos[0], "Cota linear");
        assert_eq!(vistos[1], "Cota ordenada X");
        assert_eq!(vistos[3], "Cota ordenada Z");
        assert_eq!(vistos[4], "Cota entre círculos: centro a centro");
        assert_eq!(vistos[6], "Cota entre círculos: exterior");
        assert_eq!(k.next_variant(), PmiKind::Linear);
        assert_eq!(PmiKind::Radius.next_variant(), PmiKind::Diameter);
        assert_eq!(PmiKind::Diameter.next_variant(), PmiKind::Radius);
        assert!(PmiKind::Linear.has_variants());
        assert!(!PmiKind::Angle.has_variants());
        assert!(!PmiKind::Note.has_variants());
    }

    #[test]
    fn cadeia_liga_pontos_consecutivos() {
        let pts = [pt(0., 0., 0.), pt(10., 0., 0.), pt(25., 0., 0.), pt(45., 0., 0.)];
        let t = chain(&pts, [0., 0., 12.]);
        assert_eq!(t.len(), 3);
        let ps = build(&PmiKind::Linear, &t, 100, "Cadeia", "Cadeia 1");
        let valores: Vec<f64> = ps.iter().map(|p| p.measure(&p.anchors).unwrap()).collect();
        assert_eq!(valores, vec![10., 15., 20.]);
        // a soma da cadeia é a distância total
        assert_eq!(valores.iter().sum::<f64>(), 45.);
        assert_eq!(ps.iter().map(|p| p.id.0).collect::<Vec<_>>(), vec![100, 101, 102]);
        assert!(ps.iter().all(|p| p.folder == "Cadeia 1" && p.offset == [0., 0., 12.]));
        assert!(chain(&pts[..1], [0.; 3]).is_empty());
        assert!(chain(&[], [0.; 3]).is_empty());
    }

    #[test]
    fn base_parte_da_mesma_origem_e_empilha() {
        let pts = [pt(5., 0., 0.), pt(15., 0., 0.), pt(30., 0., 0.), pt(50., 0., 0.)];
        let t = baseline(&pts, [0., 0., 1.], 8.);
        assert_eq!(t.len(), 3);
        let ps = build(&PmiKind::Ordinate { axis: 0 }, &t, 1, "Base", "");
        let valores: Vec<f64> = ps.iter().map(|p| p.measure(&p.anchors).unwrap()).collect();
        assert_eq!(valores, vec![10., 25., 45.]);
        assert!(ps.iter().all(|p| p.anchors[0] == [5., 0., 0.]));
        // as linhas de cota sobem de 8 em 8 mm para não se cruzarem
        assert_eq!(ps.iter().map(|p| p.offset[2]).collect::<Vec<_>>(), vec![8., 16., 24.]);
        assert!(baseline(&[], [0., 0., 1.], 8.).is_empty());
        assert!(baseline(&pts[..1], [0., 0., 1.], 8.).is_empty());
    }

    #[test]
    fn tolerancia_vale_nas_cotas_novas() {
        let mut pm = pmi(PmiKind::Ordinate { axis: 2 }, vec![]);
        pm.tol_plus = "0,1".into();
        pm.tol_minus = "0,1".into();
        assert_eq!(pm.display(Some(12.)), "12 ±0,1");
        pm.tol_minus = "0,2".into();
        let (main, stack) = pm.display_parts(Some(12.));
        assert_eq!(main, "12");
        assert_eq!(stack, Some(("+0,1".to_string(), "−0,2".to_string())));
    }
}
