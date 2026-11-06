// SPDX-License-Identifier: LGPL-2.1-or-later
//! B290: manipulação no 3D de Multi-seções, Concordância e Conectar. Número e sentido por seção,
//! conectores que correm sobre as curvas, setas de tangência e tensão, etiqueta de continuidade e a
//! troca das bordas que o Conectar deduziu. Os valores moram no comando: aqui só se lê o que o cálculo
//! publicou na nota e se escreve de volta.

use crate::app::App;
use crate::i18n::tr;
use crate::render::{Camera, Scene};
use eframe::egui::{self, Pos2, Rect, Stroke, Vec2};
use fenkra_core::*;
use fenkra_solidos::geo3d::Curve3;
use fenkra_solidos::{loft, RecomputeResult, RefNote};
use glam::Vec3;

fn v3(p: [f64; 3]) -> Vec3 {
    Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32)
}

/// Fração da polilinha mais perto do ponteiro, medida na tela: é o ponteiro que anda, não a peça.
pub fn fracao_na_tela(pts: &[[f64; 3]], projetar: &dyn Fn(Vec3) -> Option<Pos2>, m: Pos2) -> Option<f64> {
    let total: f32 = pts.windows(2).map(|w| (v3(w[1]) - v3(w[0])).length()).sum();
    if total <= 0. {
        return None;
    }
    let (mut acc, mut melhor) = (0f32, None::<(f32, f32)>);
    for w in pts.windows(2) {
        let (a3, b3) = (v3(w[0]), v3(w[1]));
        let l = (b3 - a3).length();
        if let (Some(a), Some(b)) = (projetar(a3), projetar(b3)) {
            let ab = b - a;
            let t = ((m - a).dot(ab) / ab.length_sq().max(1e-6)).clamp(0., 1.);
            let d = (a + ab * t - m).length();
            if melhor.is_none_or(|x| d < x.0) {
                melhor = Some((d, acc + t * l));
            }
        }
        acc += l;
    }
    melhor.map(|x| (x.1 / total) as f64)
}

/// Seções publicadas pelo cálculo, se o comando aberto é um multi-seções.
fn secoes<'a>(doc: &Document, r: &'a RecomputeResult, id: Id) -> Option<&'a [Curve3]> {
    matches!(doc.reference(id)?.kind, RefKind::SurfLoft { .. }).then(|| r.notes.get(&id).map(|n| n.secoes.as_slice())).flatten().filter(|s| !s.is_empty())
}

/// Isocurvas, conectores e bordas candidatas, na camada de traço por cima da peça.
pub fn decorar(doc: &Document, r: &RecomputeResult, f: &crate::previa::Fantasma, s: &mut Scene) {
    let cor = crate::previa::cor_linha(f.cor);
    let Some(nota) = r.notes.get(&f.id) else { return };
    for linha in &nota.iso {
        let p: Vec<[f32; 3]> = linha.iter().map(|q| v3(*q).to_array()).collect();
        crate::render::push_wide(&mut s.wide, &p, [cor[0], cor[1], cor[2], 0.55], 1., 0);
    }
    if let (Some(sec), Some(RefKind::SurfLoft { opts, .. })) = (secoes(doc, r, f.id), doc.reference(f.id).map(|x| &x.kind)) {
        for k in &opts.connectors {
            let p: Vec<[f32; 3]> = loft::pontos_do_conector(sec, k).iter().map(|q| v3(*q).to_array()).collect();
            crate::render::push_wide(&mut s.wide, &p, crate::previa::cor_linha(f.entrada), 2.5, 0);
        }
    }
    for (lado, bordas) in nota.bordas.iter().enumerate() {
        for (i, b) in bordas.iter().enumerate() {
            let p: Vec<[f32; 3]> = b.iter().map(|q| v3(*q).to_array()).collect();
            let usada = nota.borda_usada[lado] == i;
            crate::render::push_wide(&mut s.wide, &p, if usada { crate::previa::cor_linha(f.entrada) } else { [cor[0], cor[1], cor[2], 0.45] }, if usada { 4. } else { 1.5 }, if usada { 0 } else { 1 });
        }
    }
}

/// Setas de tangência do multi-seções e de tensão da concordância, como alças lineares:
/// (campo, base, direção, milímetros por unidade do valor, expressão, rótulo).
pub type Seta = (u8, Vec3, Vec3, f32, String, &'static str);

pub fn setas(doc: &Document, r: &RecomputeResult, id: Id) -> Vec<Seta> {
    let Some(k) = doc.reference(id).map(|x| &x.kind) else { return vec![] };
    let nota = r.notes.get(&id);
    match k {
        RefKind::SurfLoft { opts, closed: false, .. } if opts.guides.is_empty() => {
            let Some(q) = secoes(doc, r, id).and_then(loft::setas_de_tangencia) else { return vec![] };
            let valor = |e: &String| if e.trim().is_empty() { "0".to_string() } else { e.clone() };
            q.iter().enumerate().map(|(i, (b, d))| (1 + i as u8, v3(*b), v3(*d), 1., valor(&opts.tangents[i]), "Tangência")).collect()
        }
        RefKind::SurfBlend { cont, tension, tension2, .. } | RefKind::SurfConnect { cont, tension, tension2, .. } if *cont != Continuity::G0 => {
            let t = [tension.clone(), tension2.clone().unwrap_or_else(|| tension.clone())];
            nota.map_or(vec![], |n| n.tensoes.iter().enumerate().take(2).map(|(i, (b, d, corda))| (1 + i as u8, v3(*b), v3(*d), *corda as f32, t[i].clone(), ["Tensão 1", "Tensão 2"][i])).collect())
        }
        _ => vec![],
    }
}

/// A tensão é um múltiplo da corda e a escala é um fator, não comprimentos: a cota delas sai sem "mm".
pub fn sem_unidade(rotulo: &str) -> bool {
    matches!(rotulo, "Tensão 1" | "Tensão 2" | "Escala" | "Escala X" | "Escala Y" | "Escala Z")
}

/// Grava o valor da seta. Devolve se o campo era daqui.
pub fn escrever(doc: &mut Document, id: Id, campo: u8, valor: &str) -> bool {
    let Some(rf) = doc.references.iter_mut().find(|r| r.id == id) else { return false };
    match &mut rf.kind {
        RefKind::SurfLoft { opts, .. } if (1..=2).contains(&campo) => opts.tangents[campo as usize - 1] = valor.to_string(),
        RefKind::SurfBlend { tension, tension2, .. } | RefKind::SurfConnect { tension, tension2, .. } if (1..=2).contains(&campo) => {
            // a primeira mexida numa seta separa os dois lados
            if tension2.is_none() {
                *tension2 = Some(tension.clone());
            }
            *(if campo == 1 { tension } else { tension2.as_mut().unwrap() }) = valor.to_string();
        }
        _ => return false,
    }
    true
}

/// Próxima continuidade da etiqueta.
pub fn proxima(c: Continuity) -> Continuity {
    match c {
        Continuity::G0 => Continuity::G1,
        Continuity::G1 => Continuity::G2,
        _ => Continuity::G0,
    }
}

/// Nome da continuidade na etiqueta do 3D.
pub fn nome_da_continuidade(c: Continuity) -> &'static str {
    match c {
        Continuity::G0 => "G0 Encosta",
        Continuity::G1 => "G1 Tangente",
        _ => "G2 Curvatura",
    }
}

/// Clique numa borda candidata do Conectar: o ponto médio dela passa a mandar naquele lado.
pub fn trocar_borda(doc: &mut Document, id: Id, lado: usize, p: [f64; 3]) {
    if let Some(RefKind::SurfConnect { near, .. }) = doc.references.iter_mut().find(|r| r.id == id).map(|r| &mut r.kind) {
        near[lado] = Some(p);
    }
}

/// Aplica ao comando o apoio de cada borda clicada, quando o campo do apoio está vazio.
pub fn aplicar_apoios(k: &mut RefKind, seed: &crate::geo_ui::Seed) {
    let apoio = |c: &CurveRef| seed.apoios.iter().find(|(x, _)| x == c).map(|x| x.1);
    match k {
        RefKind::SurfBlend { a, sa, b, sb, .. } => {
            *sa = sa.or_else(|| apoio(a));
            *sb = sb.or_else(|| apoio(b));
        }
        RefKind::SurfRuled { curves, support, .. } => *support = support.or_else(|| curves.iter().find_map(apoio)),
        // aresta clicada no Conectar: a superfície dela ocupa o lado vazio e a borda fica escolhida
        RefKind::SurfConnect { a, b, near, .. } => {
            for (c, s) in &seed.apoios {
                let meio = c.edge_pts().and_then(|p| p.get(p.len() / 2).copied());
                if let Some(lado) = [&*a, &*b].iter().position(|x| **x == *s) {
                    near[lado] = meio.or(near[lado]);
                } else if *a == crate::slots::surf() {
                    (*a, near[0]) = (*s, meio);
                } else if *b == crate::slots::surf() {
                    (*b, near[1]) = (*s, meio);
                }
            }
        }
        _ => {}
    }
}

/// Número, sentido, conectores, bordas candidatas e etiqueta do comando aberto.
pub fn overlay(app: &mut App, ui: &mut egui::Ui, rect: Rect, cam: &Camera, id: Id) {
    let Some(r) = app.result.as_ref() else { return };
    let doc = &app.open.doc;
    let (destaque, fundo) = (crate::theme::accent(ui.visuals()), ui.visuals().extreme_bg_color);
    let forte = ui.visuals().strong_text_color();
    let pintor = ui.painter_at(rect);
    let pega = egui::vec2(crate::handles::PEGA * 0.7, crate::handles::PEGA * 0.7);
    let (mut inverter, mut mover, mut ciclo, mut troca) = (None, None, false, None);
    if let (Some(sec), Some(RefKind::SurfLoft { opts, .. })) = (secoes(doc, r, id), doc.reference(id).map(|x| &x.kind)) {
        for (i, c) in sec.iter().enumerate() {
            let virada = nota_sentido(r, id, i);
            let (inicio, passo) = if virada { (c.end(), c.pts[c.pts.len().saturating_sub(2)]) } else { (c.start(), c.pts[1.min(c.pts.len() - 1)]) };
            let Some(q) = cam.project(rect, v3(inicio)) else { continue };
            // seta do sentido: sai do início da seção, ao longo dela
            if let Some(d) = cam.project(rect, v3(passo)).map(|p| p - q).filter(|d| d.length() > 1e-3) {
                crate::gizmo::arrow(&pintor, q, q + d.normalized() * 30., crate::theme::st_ok());
            }
            let n = q + egui::vec2(-16., -16.);
            let hr = ui.interact(Rect::from_center_size(n, pega), egui::Id::unique(("secao_3d", id.0, i)), egui::Sense::click());
            let cor = if hr.hovered() { forte } else { crate::theme::st_ok() };
            pintor.circle_filled(n, 10., fundo);
            pintor.circle_stroke(n, 10., Stroke::new(2., cor));
            pintor.text(n, egui::Align2::CENTER_CENTER, format!("{}", i + 1), egui::FontId::proportional(12.), cor);
            if hr.on_hover_text(tr("Seção na ordem do comando. Clique para inverter o sentido dela.")).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                inverter = Some(i);
            }
        }
        for (k, fr) in opts.connectors.iter().enumerate() {
            for (i, q3) in loft::pontos_do_conector(sec, fr).iter().enumerate() {
                let Some(q) = cam.project(rect, v3(*q3)) else { continue };
                let hr = ui.interact(Rect::from_center_size(q, pega), egui::Id::unique(("conector_3d", id.0, k, i)), egui::Sense::drag());
                let cor = if hr.hovered() || hr.dragged() { forte } else { crate::theme::st_warn() };
                pintor.circle_filled(q, 6., cor);
                pintor.circle_stroke(q, 6., Stroke::new(1.5, fundo));
                if hr.dragged() {
                    if let Some(m) = ui.input(|x| x.pointer.interact_pos()) {
                        mover = fracao_na_tela(&sec[i].pts, &|p| cam.project(rect, p), m).map(|f| (k, i, f));
                    }
                }
                let _ = hr.on_hover_text(tr("Arraste ao longo da curva para desfazer a torção.")).on_hover_cursor(egui::CursorIcon::Grab);
            }
        }
    }
    if let Some(n) = r.notes.get(&id) {
        for (lado, bordas) in n.bordas.iter().enumerate() {
            for (i, b) in bordas.iter().enumerate() {
                if n.borda_usada[lado] == i {
                    continue;
                }
                let m = b[b.len() / 2];
                let Some(q) = cam.project(rect, v3(m)) else { continue };
                let hr = ui.interact(Rect::from_center_size(q, pega), egui::Id::unique(("borda_3d", id.0, lado, i)), egui::Sense::click());
                pintor.circle_stroke(q, 6., Stroke::new(2., if hr.hovered() { forte } else { destaque }));
                if hr.on_hover_text(tr("Clique para usar esta borda.")).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    troca = Some((lado, m));
                }
            }
        }
        // etiqueta de continuidade no meio da ponte
        if let (Some(c), [a, b, ..]) = (continuidade(doc, id), n.tensoes.as_slice()) {
            if let Some(q) = cam.project(rect, (v3(a.0) + v3(b.0)) / 2.) {
                let t = pintor.layout_no_wrap(tr(nome_da_continuidade(c)).to_string(), egui::FontId::proportional(12.), destaque);
                let caixa = Rect::from_center_size(q, t.size() + Vec2::new(12., 6.));
                let hr = ui.interact(caixa, egui::Id::unique(("cont_3d", id.0)), egui::Sense::click());
                pintor.rect_filled(caixa, 4., fundo);
                pintor.rect_stroke(caixa, 4., Stroke::new(if hr.hovered() { 2. } else { 1. }, destaque), egui::StrokeKind::Inside);
                pintor.galley(caixa.min + egui::vec2(6., 3.), t, destaque);
                ciclo = hr.on_hover_text(tr("Clique para trocar a continuidade.")).on_hover_cursor(egui::CursorIcon::PointingHand).clicked();
            }
        }
    }
    let rf = app.open.doc.references.iter_mut().find(|x| x.id == id).map(|x| &mut x.kind);
    match rf {
        Some(RefKind::SurfLoft { sections, opts, .. }) => {
            if let Some(i) = inverter {
                opts.reversed.resize(sections.len(), false);
                opts.reversed[i] = !opts.reversed[i];
            }
            if let Some((k, i, f)) = mover {
                opts.connectors[k][i] = f;
            }
        }
        Some(RefKind::SurfBlend { cont, .. } | RefKind::SurfConnect { cont, .. }) if ciclo => *cont = proxima(*cont),
        _ => {}
    }
    if let Some((lado, m)) = troca {
        trocar_borda(&mut app.open.doc, id, lado, m);
    }
}

/// A seção `i` é percorrida ao contrário?
fn nota_sentido(r: &RecomputeResult, id: Id, i: usize) -> bool {
    r.notes.get(&id).and_then(|n| n.sentidos.get(i).copied()).unwrap_or(false)
}

fn continuidade(doc: &Document, id: Id) -> Option<Continuity> {
    match doc.reference(id).map(|x| &x.kind) {
        Some(RefKind::SurfBlend { cont, .. } | RefKind::SurfConnect { cont, .. }) => Some(*cont),
        _ => None,
    }
}

/// Bloco do painel do multi-seções: conectores, isocurvas e o que vale nas pontas.
pub fn painel_do_loft(ui: &mut egui::Ui, sections: usize, opts: &mut LoftOpts, closed: bool, nota: Option<&RefNote>) {
    opts.connectors.retain(|k| k.len() == sections);
    ui.horizontal(|ui| {
        ui.label(tr("Conectores"));
        if ui.button(tr("Adicionar")).on_hover_text(tr("Liga um ponto de cada seção; arraste os pontos no 3D ao longo das curvas.")).clicked() {
            if let Some(n) = nota.filter(|n| n.secoes.len() == sections) {
                let f = (opts.connectors.len() as f64 * 0.25).fract();
                opts.connectors.push(loft::conector_novo(&n.secoes, f));
            }
        }
    });
    let mut tirar = None;
    for k in 0..opts.connectors.len() {
        ui.horizontal(|ui| {
            ui.label(format!("{} {}", tr("Conector"), k + 1));
            if ui.small_button("✕").on_hover_text(tr("Tira este conector.")).clicked() {
                tirar = Some(k);
            }
        });
    }
    if let Some(k) = tirar {
        opts.connectors.remove(k);
    }
    ui.add(crate::ui_ext::slider(&mut opts.iso, 0..=20).text(tr("Isocurvas"))).on_hover_text(tr("Linhas da superfície por direção, para ler o fluxo e achar torção."));
    if !closed && opts.guides.is_empty() {
        ui.label(egui::RichText::new(tr("Puxe as setas das pontas para a superfície sair na normal da seção. Zero deixa a ponta livre.")).weak().small());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geo_ui::Seed;

    #[test]
    fn fracao_segue_o_ponteiro_sobre_a_curva() {
        // projeção de frente: x, y da peça viram a tela
        let p = |q: Vec3| Some(egui::pos2(q.x, q.y));
        let c = [[0., 0., 0.], [100., 0., 0.], [100., 100., 0.]];
        assert!((fracao_na_tela(&c, &p, egui::pos2(50., 30.)).unwrap() - 0.25).abs() < 1e-5);
        assert!((fracao_na_tela(&c, &p, egui::pos2(130., 50.)).unwrap() - 0.75).abs() < 1e-5);
    }

    #[test]
    fn seta_separa_as_tensoes_na_primeira_mexida() {
        let mut d = Document::new("t");
        let k = RefKind::SurfBlend { a: CurveRef::default(), sa: None, b: CurveRef::default(), sb: None, cont: Continuity::G1, tension: "1".into(), tension2: None };
        let id = d.add_geometry(k);
        assert!(escrever(&mut d, id, 2, "1.5"));
        let Some(RefKind::SurfBlend { tension, tension2, .. }) = d.reference(id).map(|r| &r.kind) else { panic!() };
        assert_eq!((tension.as_str(), tension2.as_deref()), ("1", Some("1.5")));
    }

    /// Duas folhas e as arestas de frente uma para a outra, como o clique as entrega ao comando.
    fn folhas_e_arestas() -> (Document, Id, Id, Seed) {
        let t = |x: &str| x.to_string();
        let mut d = Document::new("folhas");
        let mut folha = |a: [f64; 3], b: [f64; 3]| {
            let c = CurveRef::Ref(d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed(a), b: PointRef::Fixed(b), start: t("0"), end: t("0") }));
            d.add_geometry(RefKind::SurfExtrude { curve: c, dir: AxisRef::Origin(OriginAxis::Y), length: t("40"), length2: t("0"), until: None, until2: None })
        };
        let (a, b) = (folha([-40., 0., 0.], [0., 0., 0.]), folha([30., 0., 20.], [70., 0., 20.]));
        let r = fenkra_solidos::recompute(&d);
        let mut seed = Seed::default();
        for (s, x) in [(a, 0f64), (b, 30.)] {
            let body = crate::picking::SURF_BASE + r.surfaces.iter().position(|q| q.id == s).unwrap();
            let arestas = crate::picking::mesh_of(&r, body).unwrap().edge_ranges.len();
            let edge = (0..arestas).find(|e| crate::picking::edge_polyline(&r, body, *e).iter().all(|p| (p[0] - x).abs() < 1e-4)).expect("aresta");
            let c = CurveRef::Edge(crate::picking::edge_polyline(&r, body, edge));
            seed.apoios.push((c.clone(), crate::sup3d::apoio_da_aresta(&r, body, edge).expect("apoio")));
            seed.curves.push(c);
        }
        (d, a, b, seed)
    }

    /// Aceite: a concordância nasce de duas arestas clicadas, sem curva de Contorno no meio.
    #[test]
    fn concordancia_nasce_da_aresta_clicada_e_deduz_o_apoio() {
        let (mut d, a, b, seed) = folhas_e_arestas();
        let antes = d.references.len();
        let k = crate::geo_ui::GeoKind::SurfBlend.make(&seed);
        assert!(matches!(&k, RefKind::SurfBlend { sa: Some(x), sb: Some(y), .. } if *x == SurfRef::Ref(a) && *y == SurfRef::Ref(b)), "{k:?}");
        let id = d.add_geometry(k);
        assert_eq!(d.references.len(), antes + 1, "nenhuma curva a mais");
        let r = fenkra_solidos::recompute(&d);
        assert!(matches!(r.status.get(&id), Some(fenkra_solidos::Status::Ok)), "{:?}", r.status.get(&id));
        // a seta de cada lado mede a tensão vezes a corda da ponte
        let s = setas(&d, &r, id);
        assert_eq!(s.iter().map(|x| x.0).collect::<Vec<_>>(), [1, 2]);
        let corda = (900f32 + 400.).sqrt();
        assert!(s.iter().all(|x| (x.3 - corda).abs() < 0.01), "{:?}", s.iter().map(|x| x.3).collect::<Vec<_>>());
        // sai da folha 1 em +x e entra na folha 2 vindo de -x: as duas setas apontam para a ponte
        assert!(s[0].2.x > 0.99 && s[1].2.x < -0.99, "{:?} {:?}", s[0].2, s[1].2);
    }

    #[test]
    fn conectar_aceita_a_aresta_e_guarda_a_borda() {
        let (mut d, a, b, mut seed) = folhas_e_arestas();
        // troca a primeira borda pela de trás, em x = -40
        let r = fenkra_solidos::recompute(&d);
        let body = crate::picking::SURF_BASE + r.surfaces.iter().position(|q| q.id == a).unwrap();
        let arestas = crate::picking::mesh_of(&r, body).unwrap().edge_ranges.len();
        let edge = (0..arestas).find(|e| crate::picking::edge_polyline(&r, body, *e).iter().all(|p| (p[0] + 40.).abs() < 1e-4)).expect("aresta");
        seed.apoios[0].0 = CurveRef::Edge(crate::picking::edge_polyline(&r, body, edge));
        let k = crate::geo_ui::GeoKind::SurfConnect.make(&seed);
        assert!(matches!(&k, RefKind::SurfConnect { a: x, b: y, near: [Some(_), Some(_)], .. } if *x == SurfRef::Ref(a) && *y == SurfRef::Ref(b)), "{k:?}");
        let id = d.add_geometry(k);
        let r = fenkra_solidos::recompute(&d);
        assert!(matches!(r.status.get(&id), Some(fenkra_solidos::Status::Ok)), "{:?}", r.status.get(&id));
        let m = &r.surfaces.iter().find(|s| s.id == id).unwrap().mesh;
        let menor = m.positions.chunks_exact(3).map(|p| p[0]).fold(f32::MAX, f32::min);
        assert!(menor < -39., "a ponte sai da borda clicada: {menor}");
    }

    #[test]
    fn etiqueta_percorre_as_continuidades() {
        assert_eq!(proxima(proxima(proxima(Continuity::G0))), Continuity::G0);
    }

    #[test]
    fn apoio_vem_da_borda_clicada_e_nao_passa_por_cima_do_escolhido() {
        let borda = CurveRef::Edge(vec![[0.; 3], [1., 0., 0.]]);
        let outra = CurveRef::Edge(vec![[0., 5., 0.], [1., 5., 0.]]);
        let s = SurfRef::Ref(Id(7));
        let seed = Seed { apoios: vec![(borda.clone(), s)], ..Default::default() };
        let mut k = RefKind::SurfBlend { a: borda.clone(), sa: None, b: outra.clone(), sb: None, cont: Continuity::G1, tension: "1".into(), tension2: None };
        aplicar_apoios(&mut k, &seed);
        let RefKind::SurfBlend { sa, sb, .. } = &k else { panic!() };
        assert_eq!((*sa, *sb), (Some(s), None));
        let dono = SurfRef::Ref(Id(9));
        let mut k = RefKind::SurfBlend { a: borda, sa: Some(dono), b: outra, sb: None, cont: Continuity::G1, tension: "1".into(), tension2: None };
        aplicar_apoios(&mut k, &seed);
        assert!(matches!(k, RefKind::SurfBlend { sa: Some(x), .. } if x == dono));
    }
}
