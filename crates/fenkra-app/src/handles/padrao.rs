// SPDX-License-Identifier: LGPL-2.1-or-later
//! Manipulação 3D do padrão (B91): cada instância é um ponto clicável na prévia (clicar pula ou
//! restaura a instância), seta de espaçamento nas duas direções do padrão linear e arco de ângulo
//! no circular. O plano do espelho tem o desenho translúcido com rótulo em `previa.rs`.
//!
//! Como funciona: o padrão guarda só as posições (i, j) puladas em `Pattern::skip`. A prévia pede ao
//! kernel as instâncias (`instancias_a_partir_de`), parte do centro do corpo de antes do comando, e
//! desenha um ponto em cada uma. O clique só liga ou desliga a posição na lista; o recálculo tira a
//! cópia, e a lista com X no painel mostra o mesmo estado.

use super::*;
use fenkra_solidos::padrao::{instancias_a_partir_de, InstanciaPadrao};

/// Espaçamento (direção 1 e 2) e ângulo do padrão: campos extras em `solidos::aplicar`.
pub(super) const ESPACO: u8 = 40;
pub(super) const ESPACO2: u8 = 41;
pub(super) const ARCO: u8 = 42;
/// Fator da escala uniforme (B288), junto dos outros campos novos.
pub(super) const ESCALA: u8 = 43;

/// Liga ou desliga o pulo da instância `chave`. A original (0, 0) nunca se pula. Devolve se pulou.
pub(crate) fn alternar_pulo(pt: &mut Pattern, chave: (u32, u32)) -> Option<bool> {
    if chave == (0, 0) {
        return None;
    }
    match pt.skip.iter().position(|k| *k == chave) {
        Some(i) => {
            pt.skip.remove(i);
            Some(false)
        }
        None => {
            pt.skip.push(chave);
            Some(true)
        }
    }
}

/// Centro da caixa das faces que os comandos de origem do padrão criaram: é a forma que o padrão
/// copia, e as instâncias saem dali. Sem faces dessas (comando de origem suprimido), `None`.
pub(crate) fn centro_das_fontes(m: &fenkra_kernel::Mesh, donos: &[Id], fontes: &[Id]) -> Option<([f64; 3], [f64; 3])> {
    let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
    for (fi, dono) in donos.iter().enumerate().filter(|(_, d)| fontes.contains(d)) {
        let i0 = *m.face_ranges.get(fi)? as usize;
        let i1 = m.face_ranges.get(fi + 1).map(|x| *x as usize).unwrap_or(m.indices.len());
        let _ = dono;
        for i in &m.indices[i0..i1] {
            let k = *i as usize * 3;
            for c in 0..3 {
                let x = m.positions[k + c] as f64;
                lo[c] = lo[c].min(x);
                hi[c] = hi[c].max(x);
            }
        }
    }
    (lo[0] <= hi[0]).then(|| ([0, 1, 2].map(|i| (lo[i] + hi[i]) / 2.), [0, 1, 2].map(|i| (hi[i] - lo[i]) / 2.)))
}

/// De onde as instâncias saem: o centro da forma copiada; se ela não está no resultado, o centro do
/// corpo de antes do comando.
fn origem(app: &App, alvo: Id) -> Option<([f64; 3], [f64; 3])> {
    if let Some(Feature { kind: FeatureKind::Pattern(p), .. }) = app.open.doc.features.iter().find(|f| f.target == alvo && matches!(&f.kind, FeatureKind::Pattern(_)) && app.session.as_ref().is_some_and(|s| s.id == f.id)) {
        if let Some(b) = app.result.as_ref().and_then(|r| r.bodies.iter().find(|b| b.collector == alvo)) {
            if let Some(c) = centro_das_fontes(&b.mesh, &b.face_owner, &p.sources) {
                return Some(c);
            }
        }
    }
    let malha = app
        .session
        .as_ref()
        .and_then(|s| s.corpos_antes.iter().find(|c| c.collector == alvo).map(|c| &c.mesh))
        .or_else(|| app.result.as_ref().and_then(|r| r.bodies.iter().find(|b| b.collector == alvo).map(|b| &b.mesh)))?;
    caixa_da_malha(malha)
}

pub(crate) fn caixa_da_malha(m: &fenkra_kernel::Mesh) -> Option<([f64; 3], [f64; 3])> {
    let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
    for p in m.positions.chunks_exact(3) {
        for i in 0..3 {
            lo[i] = lo[i].min(p[i] as f64);
            hi[i] = hi[i].max(p[i] as f64);
        }
    }
    (lo[0] <= hi[0]).then(|| ([0, 1, 2].map(|i| (lo[i] + hi[i]) / 2.), [0, 1, 2].map(|i| (hi[i] - lo[i]) / 2.)))
}

/// Instâncias do padrão aberto, já com o centro de origem e a meia altura da forma copiada., já com o centro de origem.
pub(crate) fn instancias(app: &App, id: Id) -> (Vec<InstanciaPadrao>, f32) {
    let Some(f) = app.open.doc.feature(id) else { return (vec![], 0.) };
    let Some((c, meia)) = origem(app, f.target) else { return (vec![], 0.) };
    (instancias_a_partir_de(&app.open.doc, id, c), meia[2] as f32)
}

/// Setas de espaçamento: direção 1 sempre, direção 2 quando o padrão tem a segunda.
pub(super) fn alcas(app: &App, id: Id) -> Vec<(u8, Linear)> {
    let Some(f) = app.open.doc.feature(id) else { return vec![] };
    let FeatureKind::Pattern(p) = &f.kind else { return vec![] };
    let PatternKind::Linear { dir, spacing, dir2, spacing2, total, .. } = &p.kind else { return vec![] };
    let (Some(r), Some((c, _))) = (app.result.as_ref(), origem(app, f.target)) else { return vec![] };
    let base = v3(c);
    let mut out = vec![];
    if let Some((_, d)) = axis_pos(r, &app.open.doc, dir) {
        out.push((ESPACO, Linear { base, dir: d, k: 1., expr: spacing.clone(), label: if *total { "Comprimento total" } else { "Espaçamento" } }));
    }
    if let Some(a2) = dir2 {
        if let Some((_, d)) = axis_pos(r, &app.open.doc, a2) {
            out.push((ESPACO2, Linear { base, dir: d, k: 1., expr: spacing2.clone(), label: "Espaçamento 2" }));
        }
    }
    out
}

/// Arco do ângulo total do padrão circular, em volta do eixo, a partir do centro do corpo.
pub(super) fn arco(app: &mut App, ui: &mut egui::Ui, rect: Rect, cam: &Camera, id: Id) {
    let Some(f) = app.open.doc.feature(id) else { return };
    let FeatureKind::Pattern(p) = &f.kind else { return };
    let PatternKind::Circular { axis, angle, .. } = &p.kind else { return };
    let (angle, axis, alvo) = (angle.clone(), axis.clone(), f.target);
    let (Some(r), Some((c, _))) = (app.result.as_ref(), origem(app, alvo)) else { return };
    let Some(ax) = axis_pos(r, &app.open.doc, &axis) else { return };
    arc_handle(app, ui, rect, cam, id, v3(c), ax, &angle, false, ARCO, (1., 360.));
}

/// Um ponto clicável por instância. Devolve a instância clicada, se houve clique.
pub(super) fn pontos(app: &mut App, ui: &mut egui::Ui, rect: Rect, cam: &Camera, id: Id) {
    let (lista, meia_altura) = instancias(app, id);
    let mut clicada = None;
    let (ok, err) = (crate::theme::accent(ui.visuals()), crate::theme::st_err());
    let painter = ui.painter_at(rect);
    for (n, inst) in lista.iter().enumerate() {
        if inst.chave == (0, 0) {
            continue;
        }
        // o ponto fica no topo da cópia: o centro dela é onde a seta de espaçamento e o arco terminam
        let Some(q) = cam.project(rect, v3(inst.centro) + Vec3::Z * (meia_altura + 2.)) else { continue };
        let r = Rect::from_center_size(q, egui::vec2(PEGA, PEGA));
        let resp = ui.interact(r, egui::Id::unique(("padrao_inst", id.0, inst.chave.0, inst.chave.1)), egui::Sense::click());
        let cor = if inst.pulada { err } else { ok };
        let raio = if resp.hovered() { 9. } else { 7. };
        if inst.pulada {
            // pulada: anel vazio com um X, para dar vontade de restaurar
            painter.circle_stroke(q, raio, Stroke::new(2., cor));
            painter.line_segment([q + egui::vec2(-4., -4.), q + egui::vec2(4., 4.)], Stroke::new(2., cor));
            painter.line_segment([q + egui::vec2(-4., 4.), q + egui::vec2(4., -4.)], Stroke::new(2., cor));
        } else {
            painter.circle_filled(q, raio, cor);
            painter.circle_stroke(q, raio, Stroke::new(1.5, ui.visuals().extreme_bg_color));
        }
        painter.text(q + egui::vec2(0., -raio - 8.), egui::Align2::CENTER_CENTER, format!("{}", n + 1), egui::FontId::proportional(11.), cor);
        let dica = if inst.pulada { tr("Instância pulada: clique para restaurar") } else { tr("Clique para pular esta instância") };
        let resp = resp.on_hover_text(dica);
        if resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if resp.clicked() {
            clicada = Some(inst.chave);
        }
    }
    if let Some(ch) = clicada {
        if let Some(Feature { kind: FeatureKind::Pattern(p), .. }) = app.open.doc.feature_mut(id) {
            alternar_pulo(p, ch);
        }
    }
}

/// Tudo do padrão que mora no overlay: pontos (as setas e o arco entram pelo fluxo normal).
pub(super) fn overlay(app: &mut App, ui: &mut egui::Ui, rect: Rect, cam: &Camera, id: Id) {
    if !matches!(app.open.doc.feature(id).map(|f| &f.kind), Some(FeatureKind::Pattern(_))) {
        return;
    }
    pontos(app, ui, rect, cam, id);
    arco(app, ui, rect, cam, id);
}

#[cfg(test)]
mod tests {
    use super::*;
    use fenkra_solidos::recompute;

    const XY: PlaneRef = PlaneRef::Origin(OriginPlane::XY);

    fn pino(count: &str, spacing: &str) -> (Document, Id) {
        let mut d = Document::new("padrão");
        let mut data = SketchData::default();
        let mut al = || d.alloc();
        data.add_rect(&mut al, [0., 0.], [10., 10.], false);
        let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: XY, data }));
        let b = d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind("10".into()), reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
        let p = d.add_feature(FeatureKind::Pattern(Pattern {
            kind: PatternKind::Linear { dir: AxisRef::Origin(OriginAxis::X), count: count.into(), spacing: spacing.into(), dir2: None, count2: "1".into(), spacing2: "0".into(), total: false },
            sources: vec![b],
            skip: vec![],
        }));
        (d, p)
    }

    #[test]
    fn centro_das_fontes_e_o_do_pino_e_nao_o_da_base_inteira() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        // base de 100 × 26 com um pino de 10 × 10 × 10 em x = 8..18: o centro do corpo está longe do pino
        let mut d = Document::new("base e pino");
        let mut dados = SketchData::default();
        let mut al = || d.alloc();
        dados.add_rect(&mut al, [-40., -8.], [60., 18.], false);
        let sb = d.add_feature(FeatureKind::Sketch(Sketch { plane: XY, data: dados }));
        d.add_feature(FeatureKind::Extrude(Extrude { sketch: sb, op: BoolOp::Add, extent: Extent::Blind("2".into()), reverse: true, taper: "0".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
        let mut dados = SketchData::default();
        let mut al = || d.alloc();
        dados.add_rect(&mut al, [8., 0.], [18., 10.], false);
        let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: XY, data: dados }));
        let pino = d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind("10".into()), reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
        let r = recompute(&d);
        let b = &r.bodies[0];
        let (c, meia) = centro_das_fontes(&b.mesh, &b.face_owner, &[pino]).expect("faces do pino");
        assert!((meia[0] - 5.).abs() < 1e-3 && (meia[1] - 5.).abs() < 1e-3, "{meia:?}");
        // o pino cobre x 8..18, y 0..10 e, fundido à base, só as faces laterais e o topo são dele: z 0..10
        assert!((c[0] - 13.).abs() < 1e-3 && (c[1] - 5.).abs() < 1e-3 && c[2] > 4. && c[2] < 6., "{c:?}");
        assert!(centro_das_fontes(&b.mesh, &b.face_owner, &[Id(999_999)]).is_none());
    }

    #[test]
    fn clicar_no_ponto_pula_e_clicar_de_novo_restaura() {
        let (mut d, p) = pino("4", "20");
        let Some(Feature { kind: FeatureKind::Pattern(pt), .. }) = d.feature_mut(p) else { panic!() };
        assert_eq!(alternar_pulo(pt, (2, 0)), Some(true));
        assert_eq!(pt.skip, vec![(2, 0)]);
        assert_eq!(alternar_pulo(pt, (0, 0)), None, "a original não se pula");
        assert_eq!(alternar_pulo(pt, (2, 0)), Some(false));
        assert!(pt.skip.is_empty());
    }

    #[test]
    fn pontos_da_previa_caem_no_centro_de_cada_copia_e_o_clique_tira_a_copia() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (mut d, p) = pino("4", "20");
        // centro do corpo de antes: o pino de 10 × 10 × 10
        let lista = instancias_a_partir_de(&d, p, [5., 5., 5.]);
        assert_eq!(lista.len(), 4);
        for (i, inst) in lista.iter().enumerate() {
            assert!((inst.centro[0] - (5. + 20. * i as f64)).abs() < 1e-9 && !inst.pulada);
        }
        let volume = |d: &Document| -> f64 { recompute(d).bodies.iter().map(|b| b.mass.volume).sum() };
        assert!((volume(&d) - 4000.).abs() < 1e-3);
        // o clique no ponto da terceira cópia grava a posição e o recálculo tira só ela
        let Some(Feature { kind: FeatureKind::Pattern(pt), .. }) = d.feature_mut(p) else { panic!() };
        alternar_pulo(pt, lista[2].chave);
        assert!((volume(&d) - 3000.).abs() < 1e-3);
        let de_novo = instancias_a_partir_de(&d, p, [5., 5., 5.]);
        assert!(de_novo[2].pulada && !de_novo[1].pulada, "{de_novo:?}");
        // restaurar volta ao volume cheio
        let Some(Feature { kind: FeatureKind::Pattern(pt), .. }) = d.feature_mut(p) else { panic!() };
        alternar_pulo(pt, (2, 0));
        assert!((volume(&d) - 4000.).abs() < 1e-3);
    }

    #[test]
    fn seta_de_espacamento_e_arco_gravam_o_campo_certo() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (mut d, p) = pino("3", "20");
        assert!(solidos::aplicar(&mut d, p, ESPACO, "30".into()));
        let volume = |d: &Document| -> f64 { recompute(d).bodies.iter().map(|b| b.mass.volume).sum() };
        let r = recompute(&d);
        let (mut lo, mut hi) = (f32::MAX, f32::MIN);
        for b in &r.bodies {
            for q in b.mesh.positions.chunks_exact(3) {
                lo = lo.min(q[0]);
                hi = hi.max(q[0]);
            }
        }
        // três pinos de 10 de largura a cada 30: de 0 a 70
        assert!(lo.abs() < 1e-4 && (hi - 70.).abs() < 1e-4, "{lo} {hi}");
        assert!((volume(&d) - 3000.).abs() < 1e-3);
        // o padrão linear não tem ângulo; o circular não tem espaçamento
        assert!(!solidos::aplicar(&mut d, p, ARCO, "90".into()));
        let mut c = d.clone();
        if let Some(Feature { kind: FeatureKind::Pattern(pt), .. }) = c.feature_mut(p) {
            pt.kind = PatternKind::Circular { axis: AxisRef::Origin(OriginAxis::Z), count: "4".into(), angle: "360".into() };
        }
        assert!(solidos::aplicar(&mut c, p, ARCO, "180".into()));
        assert!(!solidos::aplicar(&mut c, p, ESPACO, "5".into()));
        let Some(Feature { kind: FeatureKind::Pattern(Pattern { kind: PatternKind::Circular { angle, .. }, .. }), .. }) = c.feature(p) else { panic!() };
        assert_eq!(angle, "180");
    }
}
