// SPDX-License-Identifier: LGPL-2.1-or-later
//! Peças que as calculadoras de projeto inserem na montagem (B110): a mola helicoidal de compressão,
//! a barra da viga e o par eixo e cubo do ajuste forçado. Cada peça sai com as medidas que o painel
//! calculou, como parâmetros editáveis, e leva nas propriedades o resumo do cálculo (rótulo de
//! cálculo preliminar incluído).
//!
//! - **Mola**: varredura de um círculo do diâmetro do fio ao longo de uma hélice de raio `D/2`. O
//!   número de espiras é o total da extremidade escolhida (`Nt = Na` simples, `Na + 2` esquadradas) e
//!   o passo é uniforme, `p = (L0 − d) / Nt`, com a hélice de `z = d/2` a `z = L0 − d/2`: a mola
//!   cabe entre `z = 0` e `z = L0`. É um modelo **simplificado**: as espiras das pontas não são
//!   encostadas nem retificadas no 3D (a conta usa a extremidade certa).
//! - **Viga**: a seção extrudada pelo vão ao longo de Z (retangular centrada, circular ou tubo).
//! - **Ajuste forçado**: eixo de diâmetro nominal `d`, com o comprimento da união mais 20 mm, e cubo
//!   anel `D × d` com o comprimento da união. A geometria é a **nominal**; o ajuste ISO 286 e a
//!   interferência vão nas propriedades da peça, onde o desenho e a lista de peças os leem.

use crate::calculo_projeto::{calcular_ajuste, calcular_mola, EntradaAjuste, EntradaMola, Secao, AVISO_PRELIMINAR};
use crate::fixadores::Montador;
use fenkra_core::sketch::Curve;
use fenkra_core::*;

fn nomear(m: &mut Montador, id: Id, nome: &str) {
    if let Some(f) = m.doc.features.iter_mut().find(|f| f.id == id) {
        f.name = nome.into();
    }
}

fn numero(v: f64) -> String {
    let s = format!("{v:.3}");
    s.trim_end_matches('0').trim_end_matches('.').replace('.', ",")
}

/// Mola helicoidal de compressão no comprimento livre, eixo Z, de `z = 0` a `z = L0`.
pub fn mola_documento(e: &EntradaMola) -> Result<Document, String> {
    let r = calcular_mola(e)?;
    let nt = e.extremidade.espiras_totais(e.espiras_ativas);
    let altura = e.comprimento_livre - e.d_fio;
    let passo = altura / nt;
    if passo <= 1.05 * e.d_fio {
        return Err(format!("no comprimento livre as espiras encostam (passo {:.2} mm para fio de {:.2} mm): aumente o comprimento livre ou reduza as espiras", passo, e.d_fio));
    }
    let mut m = Montador::novo(&format!("Mola de compressão Ø{} × Ø{} × {}", numero(e.d_fio), numero(e.d_medio), numero(e.comprimento_livre)));
    m.doc.props.material = "Aço para molas".into();
    m.doc.props.part_type = PartType::Comprada;
    m.param("d", &e.d_fio.to_string())?;
    m.param("D", &e.d_medio.to_string())?;
    m.param("L0", &e.comprimento_livre.to_string())?;
    m.doc.add_param("Nt", &nt.to_string(), Unit::Numero).map_err(|e| e.to_string())?;
    m.param("passo", "(L0 - d) / Nt")?;
    let base = m.doc.add_geometry(RefKind::PointCoords { x: "0".into(), y: "0".into(), z: "d/2".into(), frame: None });
    let helice = m.doc.add_geometry(RefKind::Helix { origin: PointRef::Ref(base), axis: AxisRef::Origin(OriginAxis::Z), radius: "D/2".into(), pitch: "passo".into(), height: "L0 - d".into(), taper: "0".into(), left: false });
    let plano = m.doc.add_geometry(RefKind::PlaneNormalCurve { curve: CurveRef::Ref(helice), at: OnCurve::Ratio("0".into()), reverse: false, point: None, project: false });
    for (id, nome) in [(base, "Base_helice"), (helice, "Helice"), (plano, "Plano_fio")] {
        if let Some(rf) = m.doc.references.iter_mut().find(|rf| rf.id == id) {
            rf.name = nome.into();
            rf.visible = false;
        }
    }
    let mut data = SketchData::default();
    {
        let mut al = || m.doc.alloc();
        let c = data.add_point(&mut al, 0., 0.);
        data.add_curve(&mut al, Curve::Circle { c, r: e.d_fio / 2. }, false);
    }
    let sk = m.sketch("Fio", PlaneRef::Datum(plano), data);
    let id = m.doc.add_feature(FeatureKind::Sweep(Sweep { sketch: sk, spine: CurveRef::Ref(helice), mode: SweepMode::Corrigido, op: BoolOp::Add, thin: None }));
    nomear(&mut m, id, "Espiras");
    let resumo = [
        ("Cálculo", AVISO_PRELIMINAR.to_string()),
        ("Constante", format!("{:.2} N/mm", r.constante)),
        ("Espiras ativas", numero(e.espiras_ativas)),
        ("Extremidade", e.extremidade.rotulo().to_string()),
        ("Altura sólida", format!("{:.2} mm", r.altura_solida)),
        ("FS no trabalho", format!("{:.2}", r.fs_trabalho)),
    ];
    m.doc.props.custom.extend(resumo.into_iter().map(|(k, v)| (k.to_string(), v)));
    Ok(m.doc)
}

/// Barra da viga: a seção extrudada pelo comprimento ao longo de Z, de `z = 0` a `z = comprimento`.
pub fn viga_documento(secao: &Secao, comprimento: f64) -> Result<Document, String> {
    if !(comprimento.is_finite() && comprimento > 0.) {
        return Err("o vão da viga precisa ser positivo".into());
    }
    let (nome, dims): (String, Vec<(&str, f64)>) = match *secao {
        Secao::Retangular { b, h } => (format!("Barra retangular {} × {} × {}", numero(b), numero(h), numero(comprimento)), vec![("b", b), ("h", h)]),
        Secao::Circular { d } => (format!("Barra redonda Ø{} × {}", numero(d), numero(comprimento)), vec![("d", d)]),
        Secao::Tubo { d_ext, d_int } => {
            if d_int >= d_ext {
                return Err("o diâmetro interno do tubo precisa ser menor que o externo".into());
            }
            (format!("Tubo Ø{} × Ø{} × {}", numero(d_ext), numero(d_int), numero(comprimento)), vec![("D", d_ext), ("di", d_int)])
        }
        Secao::Generica { .. } => return Err("a seção genérica só tem I e c, sem forma: escolha retangular, circular ou tubo para inserir a barra".into()),
    };
    secao.propriedades()?;
    let mut m = Montador::novo(&nome);
    m.doc.props.material = "Aço 1020".into();
    m.doc.props.part_type = PartType::Fabricada;
    for (k, v) in &dims {
        m.param(k, &v.to_string())?;
    }
    m.param("L", &comprimento.to_string())?;
    let mut data = SketchData::default();
    {
        let mut al = || m.doc.alloc();
        match *secao {
            Secao::Retangular { b, h } => {
                data.add_rect(&mut al, [-b / 2., -h / 2.], [b / 2., h / 2.], false);
            }
            Secao::Circular { d } => {
                let c = data.add_point(&mut al, 0., 0.);
                data.add_curve(&mut al, Curve::Circle { c, r: d / 2. }, false);
            }
            Secao::Tubo { d_ext, d_int } => {
                let c = data.add_point(&mut al, 0., 0.);
                data.add_curve(&mut al, Curve::Circle { c, r: d_ext / 2. }, false);
                if d_int > 0. {
                    data.add_curve(&mut al, Curve::Circle { c, r: d_int / 2. }, false);
                }
            }
            Secao::Generica { .. } => unreachable!(),
        }
    }
    let sk = m.sketch("Secao", PlaneRef::Origin(OriginPlane::XY), data);
    let id = m.doc.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind("L".into()), reverse: false, taper: "0".into(), thin: None, thin_side: 0, dir: None, ends: Default::default() }));
    nomear(&mut m, id, "Barra");
    m.doc.props.custom.push(("Cálculo".into(), AVISO_PRELIMINAR.into()));
    Ok(m.doc)
}

/// Comprimento do eixo do ajuste: a união mais 10 mm de cada lado.
pub const SOBRA_EIXO_AJUSTE: f64 = 10.;

/// Eixo e cubo do ajuste forçado, com o eixo em Z de `z = 0` a `L + 20` e o cubo centrado nele.
/// Devolve (eixo, cubo).
pub fn ajuste_documentos(e: &EntradaAjuste) -> Result<(Document, Document), String> {
    let r = calcular_ajuste(e)?;
    let (furo, eixo) = e.ajuste.split_once('/').ok_or("ajuste no formato furo/eixo, como H7/p6")?;
    let rotulo = format!("Ø{} {}", numero(e.d), e.ajuste);
    let interf = format!("{:.0} a {:.0} µm", r.interf_min_um, r.interf_max_um);
    let comum = |m: &mut Montador, tol: &str| {
        m.doc.props.custom.push(("Ajuste".into(), rotulo.clone()));
        m.doc.props.custom.push(("Tolerância".into(), format!("Ø{} {tol}", numero(e.d))));
        m.doc.props.custom.push(("Interferência".into(), interf.clone()));
        m.doc.props.custom.push(("Cálculo".into(), AVISO_PRELIMINAR.into()));
    };
    let l_eixo = e.comprimento + 2. * SOBRA_EIXO_AJUSTE;
    let mut me = Montador::novo(&format!("Eixo do ajuste Ø{} {}", numero(e.d), eixo));
    me.doc.props.part_type = PartType::Fabricada;
    me.param("d", &e.d.to_string())?;
    me.param("L", &l_eixo.to_string())?;
    me.prim("Eixo", PrimKind::Cilindro, PlaneRef::Origin(OriginPlane::XY), ["d/2", "L", "0", "0"], BoolOp::Add)?;
    if e.d_furo_eixo > 0. {
        me.param("di", &e.d_furo_eixo.to_string())?;
        let pl = me.plano("Plano_furo", "-1")?;
        me.prim("Furo", PrimKind::Cilindro, pl, ["di/2", "L+2", "0", "0"], BoolOp::Cut)?;
    }
    comum(&mut me, eixo);
    let mut mc = Montador::novo(&format!("Cubo do ajuste Ø{} × Ø{} {}", numero(e.d_cubo), numero(e.d), furo));
    mc.doc.props.part_type = PartType::Fabricada;
    mc.param("D", &e.d_cubo.to_string())?;
    mc.param("d", &e.d.to_string())?;
    mc.param("L", &e.comprimento.to_string())?;
    mc.prim("Cubo", PrimKind::Cilindro, PlaneRef::Origin(OriginPlane::XY), ["D/2", "L", "0", "0"], BoolOp::Add)?;
    let pl = mc.plano("Plano_furo", "-1")?;
    mc.prim("Furo", PrimKind::Cilindro, pl, ["d/2", "L+2", "0", "0"], BoolOp::Cut)?;
    comum(&mut mc, furo);
    Ok((me.doc, mc.doc))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calculo_projeto::Extremidade;
    use crate::{recompute, Status};
    use std::f64::consts::PI;

    fn medir(doc: &Document) -> (f64, [f64; 3], [f64; 3]) {
        let r = recompute(doc);
        assert!(r.param_error.is_none(), "{:?}", r.param_error);
        for f in &doc.features {
            assert_eq!(r.status[&f.id], Status::Ok, "{} / {}", doc.name, f.name);
        }
        assert_eq!(r.bodies.len(), 1, "{}", doc.name);
        let b = &r.bodies[0];
        let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
        for p in b.mesh.positions.chunks_exact(3) {
            for i in 0..3 {
                lo[i] = lo[i].min(p[i] as f64);
                hi[i] = hi[i].max(p[i] as f64);
            }
        }
        (b.mass.volume, lo, hi)
    }

    /// Mola padrão do painel: fio 2, D 16, Na 8, esquadrada e retificada (Nt 10), L0 60.
    /// À mão: passo (60 − 2)/10 = 5,8; fio por espira √((π·16)² + 5,8²) = 50,598; volume
    /// π·2²/4 · 10 · 50,598 = 1589,6 mm³; caixa ±(16 + 2)/2 = ±9 em X e Y e de 0 a 60 em Z.
    #[test]
    fn mola_tem_o_volume_do_fio_e_cabe_no_comprimento_livre() {
        let e = EntradaMola::default();
        assert_eq!(e.extremidade.espiras_totais(e.espiras_ativas), 10.);
        let fio = 10. * ((PI * 16.).powi(2) + 5.8_f64.powi(2)).sqrt();
        assert!((fio - 505.98).abs() < 0.01, "{fio}");
        let d = mola_documento(&e).unwrap();
        assert!(d.props.custom.iter().any(|(k, v)| k == "Constante" && v.contains("N/mm")));
        // espiras que encostariam são recusadas
        assert!(mola_documento(&EntradaMola { comprimento_livre: 20.5, espiras_ativas: 9., extremidade: Extremidade::Simples, ..e.clone() }).is_err());
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (v, lo, hi) = medir(&d);
        let esperado = PI * 4. / 4. * fio;
        assert!((v - esperado).abs() / esperado < 0.01, "volume {v} contra {esperado}");
        for i in 0..2 {
            assert!((hi[i] - 9.).abs() < 0.1 && (lo[i] + 9.).abs() < 0.1, "{lo:?} {hi:?}");
        }
        assert!(lo[2] > -0.05 && lo[2] < 0.1 && hi[2] < 60.05 && hi[2] > 59.9, "{lo:?} {hi:?}");
    }

    /// Barras: retangular 20 × 40 × 500 = 400 000 mm³; redonda Ø30 × 500 = π 15² 500; tubo Ø30/Ø26.
    #[test]
    fn viga_insere_a_barra_da_secao() {
        assert!(viga_documento(&Secao::Generica { i: 1e4, c: 10. }, 500.).is_err());
        assert!(viga_documento(&Secao::Tubo { d_ext: 20., d_int: 25. }, 500.).is_err());
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (v, lo, hi) = medir(&viga_documento(&Secao::Retangular { b: 20., h: 40. }, 500.).unwrap());
        assert!((v - 400_000.).abs() < 1., "{v}");
        assert!((hi[0] - 10.).abs() < 1e-3 && (hi[1] - 20.).abs() < 1e-3 && (hi[2] - 500.).abs() < 1e-3 && lo[2].abs() < 1e-3);
        let (vc, ..) = medir(&viga_documento(&Secao::Circular { d: 30. }, 500.).unwrap());
        assert!((vc - PI * 225. * 500.).abs() / vc < 0.005, "{vc}");
        let (vt, lot, hit) = medir(&viga_documento(&Secao::Tubo { d_ext: 30., d_int: 26. }, 500.).unwrap());
        let esperado = PI * (225. - 169.) * 500.;
        assert!((vt - esperado).abs() / esperado < 0.005, "{vt} contra {esperado}");
        assert!((hit[0] - 15.).abs() < 0.05 && (lot[0] + 15.).abs() < 0.05);
    }

    /// Ajuste Ø40 H7/p6, cubo Ø80 × 50: eixo π 20² · 70, cubo π (40² − 20²) · 50; o ajuste nas propriedades.
    #[test]
    fn ajuste_insere_eixo_e_cubo_com_o_ajuste_nas_propriedades() {
        let e = EntradaAjuste::default();
        let (eixo, cubo) = ajuste_documentos(&e).unwrap();
        let prop = |d: &Document, k: &str| d.props.custom.iter().find(|(c, _)| c == k).map(|(_, v)| v.clone());
        assert_eq!(prop(&eixo, "Ajuste").as_deref(), Some("Ø40 H7/p6"));
        assert_eq!(prop(&eixo, "Tolerância").as_deref(), Some("Ø40 p6"));
        assert_eq!(prop(&cubo, "Tolerância").as_deref(), Some("Ø40 H7"));
        assert!(prop(&cubo, "Interferência").is_some_and(|v| v.ends_with("µm")));
        assert!(ajuste_documentos(&EntradaAjuste { ajuste: "H7".into(), ..Default::default() }).is_err());
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (ve, _, he) = medir(&eixo);
        assert!((ve - PI * 400. * 70.).abs() / ve < 0.005 && (he[2] - 70.).abs() < 1e-3, "{ve}");
        let (vc, _, hc) = medir(&cubo);
        assert!((vc - PI * 1200. * 50.).abs() / vc < 0.005 && (hc[0] - 40.).abs() < 0.05, "{vc}");
    }
}
