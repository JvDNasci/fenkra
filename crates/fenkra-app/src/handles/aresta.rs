// SPDX-License-Identifier: LGPL-2.1-or-later
//! Setas do filete e do chanfro no 3D (B289): início e fim do trecho parcial, e a seta que inverte a
//! face que recebe a primeira medida.
//!
//! Como funciona: o comando guarda as arestas pelo ponto médio. A malha de antes do comando ainda tem
//! a aresta e as duas faces dela (`edge_faces`); com ela a prévia acha o começo da aresta (de onde o
//! trecho é medido, em mm) e a face da primeira medida. O kernel pega as faces da aresta na ordem em
//! que a topologia as lista e troca as duas quando a aresta está invertida; a malha lista as faces na
//! mesma ordem, então a seta aponta para a face que de fato recebe a primeira medida.

use super::*;
use crate::arestas_previa::afasta_da_aresta;

/// Início e fim do trecho parcial (campos 44 e 45 de `solidos::aplicar`).
pub(super) const TRECHO_INICIO: u8 = 44;
pub(super) const TRECHO_FIM: u8 = 45;

/// Malha de antes do comando: nela a aresta escolhida ainda existe.
fn malha_antes<'a>(app: &'a App, alvo: Id) -> Option<&'a fenkra_kernel::Mesh> {
    app.session
        .as_ref()
        .and_then(|s| s.corpos_antes.iter().find(|c| c.collector == alvo).map(|c| &c.mesh))
        .or_else(|| app.result.as_ref().and_then(|r| r.bodies.iter().find(|b| b.collector == alvo).map(|b| &b.mesh)))
}

fn op_de(doc: &Document, id: Id) -> Option<(&EdgeOp, Id)> {
    let f = doc.feature(id)?;
    match &f.kind {
        FeatureKind::Fillet(o) | FeatureKind::Chamfer(o) => Some((o, f.target)),
        _ => None,
    }
}

/// Começo e direção de uma aresta reta (a ponta de partida do trecho parcial). `None` se ela é curva:
/// medir em mm numa curva não tem uma direção só.
pub(crate) fn reta_da_aresta(m: &fenkra_kernel::Mesh, e: usize) -> Option<(Vec3, Vec3, f32)> {
    let poly: Vec<Vec3> = crate::picking::edge_polyline_mesh(m, e).into_iter().map(v3).collect();
    let (a, b) = (*poly.first()?, *poly.last()?);
    let len: f32 = poly.windows(2).map(|w| (w[1] - w[0]).length()).sum();
    let corda = (b - a).length();
    (len > 1e-4 && corda >= len * 0.995).then(|| ((a), (b - a) / corda, len))
}

/// Setas do início e do fim do trecho parcial, partindo do começo da primeira aresta escolhida.
pub(super) fn alcas_trecho(app: &App, id: Id) -> Vec<(u8, Linear)> {
    let Some((o, alvo)) = op_de(&app.open.doc, id) else { return vec![] };
    let (Some([ini, fim]), EdgeSel::Edges(mids)) = (&o.extra.partial, &o.edges) else { return vec![] };
    let (Some(mid), Some(m)) = (mids.first(), malha_antes(app, alvo)) else { return vec![] };
    let Some((e, d)) = crate::entradas::aresta_perto(m, *mid) else { return vec![] };
    if d > 1.0 {
        return vec![];
    }
    let Some((base, dir, _)) = reta_da_aresta(m, e) else { return vec![] };
    vec![
        (TRECHO_INICIO, Linear { base, dir, k: 1., expr: ini.clone(), label: "Início do trecho" }),
        (TRECHO_FIM, Linear { base, dir, k: 1., expr: fim.clone(), label: "Fim do trecho" }),
    ]
}

/// Onde sai a seta de inversão da aresta `e` e para onde ela aponta: do meio da aresta, pela face que
/// recebe a primeira medida (`invertida` troca as duas faces). Devolve também o índice dessa face.
pub(crate) fn seta_da_primeira_medida(m: &fenkra_kernel::Mesh, e: usize, invertida: bool) -> Option<(Vec3, Vec3, usize)> {
    let [fa, fb] = m.edge_faces.get(e).copied().filter(|p| p[0] != fenkra_kernel::NO_FACE && p[1] != fenkra_kernel::NO_FACE)?;
    let primeira = (if invertida { fb } else { fa }) as usize;
    let ponto = super::solidos::ponto_na_aresta(m, e, 0.5)?;
    let poly: Vec<Vec3> = crate::picking::edge_polyline_mesh(m, e).into_iter().map(v3).collect();
    let t = (*poly.last()? - *poly.first()?).try_normalize().or_else(|| (poly.get(1)? - poly[0]).try_normalize())?;
    let n = super::solidos::normal_do_indice(m, primeira, [ponto.x as f64, ponto.y as f64, ponto.z as f64])?;
    Some((ponto, afasta_da_aresta(m, primeira, ponto, t, n)?, primeira))
}

/// Desenha, em cada aresta escolhida de um filete assimétrico ou chanfro de duas medidas, uma seta que
/// aponta para a face da primeira medida; clicar nela troca a face (`flips`).
pub(super) fn inverter(app: &mut App, ui: &mut egui::Ui, rect: Rect, cam: &Camera, id: Id) {
    let Some((o, alvo)) = op_de(&app.open.doc, id) else { return };
    if !matches!(o.mode, EdgeMode::TwoDist(_) | EdgeMode::DistAngle(_) | EdgeMode::Asym(_)) {
        return;
    }
    let EdgeSel::Edges(mids) = &o.edges else { return };
    let (mids, extra) = (mids.clone(), o.extra.clone());
    let Some(m) = malha_antes(app, alvo) else { return };
    let cor = crate::theme::accent(ui.visuals());
    let painter = ui.painter_at(rect);
    let mut trocar = None;
    for (i, mid) in mids.iter().enumerate() {
        let Some((e, d)) = crate::entradas::aresta_perto(m, *mid) else { continue };
        if d > 1.0 {
            continue;
        }
        let Some((p, dir, _)) = seta_da_primeira_medida(m, e, extra.flip(i)) else { continue };
        let (Some(sp), Some(sq)) = (cam.project(rect, p), cam.project(rect, p + dir)) else { continue };
        let Some(d2) = (sq - sp).try_normalized_or_none() else { continue };
        let (ponta, n) = (sp + d2 * 38., egui::vec2(-d2.y, d2.x));
        let r = Rect::from_two_pos(sp, ponta).expand(PEGA / 2.);
        let resp = ui.interact(r, egui::Id::unique(("filete_inverte", id.0, i)), egui::Sense::click());
        let c = if resp.hovered() { crate::theme::st_warn() } else { cor };
        painter.line_segment([sp, ponta], Stroke::new(3., c));
        painter.add(egui::Shape::convex_polygon(vec![ponta + d2 * 10., ponta + n * 6., ponta - n * 6.], c, Stroke::NONE));
        painter.circle_filled(sp, 4., c);
        let g = painter.layout_no_wrap(format!("1ª · {}", i + 1), egui::FontId::proportional(11.), c);
        painter.galley(ponta + d2 * 12. - egui::vec2(g.size().x / 2., g.size().y / 2.), g, c);
        let resp = resp.on_hover_text(tr("Esta seta aponta para a face que recebe a primeira medida. Clique para trocar de face."));
        if resp.clicked() {
            trocar = Some(i);
        }
    }
    if let Some(i) = trocar {
        if let Some(f) = app.open.doc.feature_mut(id) {
            if let FeatureKind::Fillet(o) | FeatureKind::Chamfer(o) = &mut f.kind {
                o.extra.flips.resize(o.extra.flips.len().max(i + 1), false);
                o.extra.flips[i] = !o.extra.flips[i];
            }
        }
    }
}

trait Normalizada {
    fn try_normalized_or_none(self) -> Option<egui::Vec2>;
}

impl Normalizada for egui::Vec2 {
    fn try_normalized_or_none(self) -> Option<egui::Vec2> {
        (self.length() > 1e-3).then(|| self.normalized())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fenkra_solidos::recompute;

    const XY: PlaneRef = PlaneRef::Origin(OriginPlane::XY);

    /// Caixa 40 × 20 × 10 com o canto em (0, 0, 0).
    fn caixa() -> Document {
        let mut d = Document::new("caixa");
        let mut data = SketchData::default();
        let mut al = || d.alloc();
        data.add_rect(&mut al, [0., 0.], [40., 20.], false);
        let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: XY, data }));
        d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind("10".into()), reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
        d
    }

    const TOPO: [f64; 3] = [20., 20., 10.];

    fn op(valor: &str, mode: EdgeMode, extra: EdgeExtra) -> EdgeOp {
        EdgeOp { value: valor.into(), edges: EdgeSel::Edges(vec![TOPO]), mode, prop: EdgeProp::Tangente, keys: vec![], extra }
    }

    /// O quanto cada uma das duas faces perdeu na aresta: (na face de cima, na face do fundo).
    fn recuos(d: &Document) -> (f64, f64) {
        let r = recompute(d);
        let mut y_topo = f32::MIN;
        let mut z_fundo = f32::MIN;
        for b in &r.bodies {
            for p in b.mesh.positions.chunks_exact(3) {
                if (p[2] - 10.).abs() < 1e-4 {
                    y_topo = y_topo.max(p[1]);
                }
                if (p[1] - 20.).abs() < 1e-4 {
                    z_fundo = z_fundo.max(p[2]);
                }
            }
        }
        (20. - y_topo as f64, 10. - z_fundo as f64)
    }

    /// A seta aponta para a face que, no kernel, recebe a primeira medida, e a inversão troca de face.
    #[test]
    fn seta_de_inversao_aponta_para_a_face_da_primeira_medida() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let antes = recompute(&caixa());
        let m = &antes.bodies[0].mesh;
        let (e, _) = crate::entradas::aresta_perto(m, TOPO).expect("aresta");
        for invertida in [false, true] {
            let mut d = caixa();
            d.add_feature(FeatureKind::Fillet(op("2", EdgeMode::Asym("8".into()), EdgeExtra { flips: vec![invertida], ..Default::default() })));
            let (topo, fundo) = recuos(&d);
            let (_, dir, _) = seta_da_primeira_medida(m, e, invertida).expect("seta");
            // a face da primeira medida é a de cima (a seta vai para -y) ou a do fundo (a seta vai para -z)
            let primeira_e_a_de_cima = dir.y < -0.9;
            let primeira_e_a_do_fundo = dir.z < -0.9;
            assert!(primeira_e_a_de_cima ^ primeira_e_a_do_fundo, "{dir:?}");
            let (da_primeira, da_outra) = if primeira_e_a_de_cima { (topo, fundo) } else { (fundo, topo) };
            assert!((da_primeira - 2.).abs() < 0.05 && (da_outra - 8.).abs() < 0.05, "invertida={invertida}: a face da seta recuou {da_primeira}, a outra {da_outra}");
        }
        // inverter troca a face da seta
        let (_, a, fa) = seta_da_primeira_medida(m, e, false).unwrap();
        let (_, b, fb) = seta_da_primeira_medida(m, e, true).unwrap();
        assert_ne!(fa, fb);
        assert!(a.dot(b).abs() < 0.1, "as setas das duas faces são perpendiculares: {a:?} {b:?}");
    }

    #[test]
    fn setas_do_trecho_partem_do_comeco_da_aresta_e_gravam_inicio_e_fim() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let mut d = caixa();
        let f = d.add_feature(FeatureKind::Fillet(op("2", EdgeMode::Constant, EdgeExtra { partial: Some(["5".into(), "25".into()]), ..Default::default() })));
        let antes = recompute(&caixa());
        let m = &antes.bodies[0].mesh;
        let (e, _) = crate::entradas::aresta_perto(m, TOPO).expect("aresta");
        let (base, dir, len) = reta_da_aresta(m, e).expect("aresta reta");
        assert!((len - 40.).abs() < 1e-3 && dir.x.abs() > 0.99, "{base:?} {dir:?} {len}");
        // o recorte do trecho: arrastar as duas pontas grava início e fim, e o volume muda como o trecho manda
        assert!(solidos::aplicar(&mut d, f, TRECHO_INICIO, "10".into()));
        assert!(solidos::aplicar(&mut d, f, TRECHO_FIM, "30".into()));
        let Some(FeatureKind::Fillet(o)) = d.feature(f).map(|x| x.kind.clone()) else { panic!() };
        assert_eq!(o.extra.partial, Some(["10".into(), "30".into()]));
        let volume = |d: &Document| -> f64 { recompute(d).bodies.iter().map(|b| b.mass.volume).sum() };
        let perda = 40. * 20. * 10. - volume(&d);
        // 20 mm de filete de raio 2: (1 − π/4) · r² · comprimento, mais o fecho reto nas pontas
        let esperado = (1. - std::f64::consts::FRAC_PI_4) * 4. * 20.;
        assert!((perda - esperado).abs() < 0.15 * esperado + 0.5, "perda {perda} esperada ≈ {esperado}");
        // sem trecho parcial a seta não tem campo para gravar: recusa em vez de gravar errado
        if let Some(FeatureKind::Fillet(o)) = d.feature_mut(f).map(|x| &mut x.kind) {
            o.extra.partial = None;
        }
        assert!(!solidos::aplicar(&mut d, f, TRECHO_INICIO, "1".into()));
    }
}
