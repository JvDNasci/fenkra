// SPDX-License-Identifier: LGPL-2.1-or-later
//! Estrutural: alças 3D do comando Membro (B253). Setas duplas de deslocamento X/Y da seção, arco de
//! rotação que gruda a cada 45 graus, alças de prolongar o início e o fim com a leitura em mm, e os
//! botões de espelhar a seção e alinhar a uma aresta. Arrastar grava a mesma expressão que o painel
//! edita: valor digitado e valor arrastado são o mesmo campo; expressão e parâmetro travam a alça.

use crate::app::{fmt, App};
use crate::handles::PEGA;
use crate::i18n::tr;
use crate::render::Camera;
use eframe::egui::{self, Color32, Rect, Stroke};
use fenkra_core::*;
use fenkra_solidos::st_frames::{align_angle, member_frames, MemberFrame};
use glam::Vec3;

/// Campo do membro que uma alça mexe.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Campo {
    OffX,
    OffY,
    Ext1,
    Ext2,
    Angulo,
}

impl Campo {
    fn rotulo(self) -> &'static str {
        match self {
            Campo::OffX => "Deslocamento X",
            Campo::OffY => "Deslocamento Y",
            Campo::Ext1 => "Prolongar início",
            Campo::Ext2 => "Prolongar fim",
            Campo::Angulo => "Rotação",
        }
    }
    fn unidade(self) -> &'static str {
        if self == Campo::Angulo {
            "°"
        } else {
            "mm"
        }
    }
}

/// A expressão do campo no membro.
pub fn expressao(m: &StructMember, c: Campo) -> &str {
    match c {
        Campo::OffX => &m.off_x,
        Campo::OffY => &m.off_y,
        Campo::Ext1 => &m.ext1,
        Campo::Ext2 => &m.ext2,
        Campo::Angulo => &m.angle,
    }
}

/// Grava o valor no campo, na mesma forma que o painel (`fmt`): arrastar e digitar dão o mesmo texto.
pub fn gravar(m: &mut StructMember, c: Campo, valor: f64) {
    let s = fmt(valor);
    match c {
        Campo::OffX => m.off_x = s,
        Campo::OffY => m.off_y = s,
        Campo::Ext1 => m.ext1 = s,
        Campo::Ext2 => m.ext2 = s,
        Campo::Angulo => m.angle = s,
    }
}

/// A alça só arrasta quando o campo é um número: expressão ou parâmetro fica travado, para não
/// quebrar o vínculo (a leitura continua na tela).
pub fn arrastavel(expr: &str) -> bool {
    expr.trim().replace(',', ".").parse::<f64>().is_ok()
}

/// Parâmetro `t` (mm) do ponto da reta `base + dir·t` mais perto do raio `ro + rd·s` do mouse.
pub fn param_no_eixo(ro: Vec3, rd: Vec3, base: Vec3, dir: Vec3) -> Option<f64> {
    let w0 = ro - base;
    let (a, b, c) = (rd.dot(rd), rd.dot(dir), dir.dot(dir));
    let (d, e) = (rd.dot(w0), dir.dot(w0));
    let den = a * c - b * b;
    if den.abs() < 1e-9 {
        return None;
    }
    Some(((a * e - b * d) / den) as f64)
}

/// Ângulo (graus, em (-180, 180]) do ponto onde o raio do mouse corta o plano da seção, medido de
/// `x0` em torno de `z` no mesmo sentido em que o comando gira a seção.
pub fn angulo_no_plano(ro: Vec3, rd: Vec3, centro: Vec3, z: Vec3, x0: Vec3) -> Option<f64> {
    let den = rd.dot(z);
    if den.abs() < 1e-6 {
        return None;
    }
    let s = (centro - ro).dot(z) / den;
    let v = ro + rd * s - centro;
    let y0 = z.cross(x0);
    Some(v.dot(y0).atan2(v.dot(x0)).to_degrees() as f64)
}

/// Gruda nos múltiplos de 45° quando o ângulo chega a `tol` graus de um deles.
pub fn gruda_45(a: f64, tol: f64) -> f64 {
    let m = (a / 45.).round() * 45.;
    if (a - m).abs() <= tol {
        m
    } else {
        a
    }
}

/// Ângulo arrastado: passo de 1° (0,1° com o passo fino) e ímã de 45° (desligado no passo fino).
pub fn angulo_arrastado(bruto: f64, fino: bool) -> f64 {
    if fino {
        (bruto * 10.).round() / 10.
    } else {
        gruda_45(bruto.round(), 4.)
    }
}

/// Valor arrastado ao longo de um eixo: `t` em mm, arredondado ao `passo`.
pub fn valor_ao_passo(t: f64, passo: f64) -> f64 {
    (t / passo).round() * passo
}

/// Direção (unitária) das arestas escolhidas na viewport, na ordem da seleção.
fn direcoes_das_arestas(app: &App) -> Vec<[f64; 3]> {
    let Some(r) = app.result.as_ref() else { return vec![] };
    app.picks
        .iter()
        .filter_map(|p| match p {
            crate::picking::Pick::Edge { body, edge, .. } => {
                let l = crate::picking::edge_polyline(r, *body, *edge);
                fenkra_solidos::geo3d::unit(fenkra_solidos::geo3d::sub(*l.last()?, *l.first()?))
            }
            _ => None,
        })
        .collect()
}

fn v3(p: [f64; 3]) -> Vec3 {
    Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32)
}

/// O membro em edição (cópia), se o comando aberto é um Membro.
fn membro(app: &App, id: Id) -> Option<StructMember> {
    match app.open.doc.feature(id).map(|f| &f.kind) {
        Some(FeatureKind::StructMember(m)) => Some(m.clone()),
        _ => None,
    }
}

fn escrever(app: &mut App, id: Id, c: Campo, v: f64) {
    if let Some(FeatureKind::StructMember(m)) = app.open.doc.feature_mut(id).map(|f| &mut f.kind) {
        gravar(m, c, v);
    }
}

/// Rótulo com a leitura do valor, junto da alça.
fn leitura(ui: &egui::Ui, rect: Rect, at: egui::Pos2, texto: String, cor: Color32) {
    let painter = ui.painter_at(rect);
    let g = painter.layout_no_wrap(texto, egui::FontId::proportional(12.), ui.visuals().strong_text_color());
    let r = Rect::from_min_size(at, g.size() + egui::vec2(10., 6.));
    painter.rect_filled(r, 4., ui.visuals().extreme_bg_color);
    painter.rect_stroke(r, 4., Stroke::new(1., cor), egui::StrokeKind::Inside);
    painter.galley(r.min + egui::vec2(5., 3.), g, ui.visuals().strong_text_color());
}

/// Texto da leitura: valor com unidade, ou a expressão travada.
fn texto_leitura(c: Campo, expr: &str, valor: f64, trava: bool) -> String {
    if trava {
        format!("🔒 {} = {} {}", expr.trim(), fmt(valor), c.unidade())
    } else {
        format!("{} {} {}", tr(c.rotulo()), fmt(valor), c.unidade())
    }
}

/// Uma alça linear: ponta arrastável com a haste saindo de `haste`; o valor sai da posição do mouse
/// medida no eixo `origem_eixo + dir·t`, menos `deslocamento_da_ponta` (a ponta fica adiante do
/// valor). Devolve o novo valor, ao passo do zoom, quando o mouse arrasta.
#[allow(clippy::too_many_arguments)]
fn alca_linear(ui: &mut egui::Ui, rect: Rect, cam: &Camera, id: Id, salt: u8, c: Campo, origem_eixo: Vec3, dir: Vec3, haste: Vec3, ponta: Vec3, expr: &str, valor: f64, deslocamento_da_ponta: f64) -> Option<f64> {
    let (sb, st) = (cam.project(rect, haste)?, cam.project(rect, ponta)?);
    let (o1, s1) = (cam.project(rect, origem_eixo)?, cam.project(rect, origem_eixo + dir)?);
    let ppm = (s1 - o1).length().max(1e-3);
    let trava = !arrastavel(expr);
    let hid = egui::Id::unique(("st_alca", id.0, salt));
    let hr = ui.interact(Rect::from_center_size(st, egui::vec2(PEGA, PEGA)), hid, if trava { egui::Sense::hover() } else { egui::Sense::drag() });
    let ativo = hr.hovered() || hr.dragged();
    let cor = if ativo { crate::theme::st_warn() } else { crate::theme::accent(ui.visuals()) };
    let painter = ui.painter_at(rect);
    let d = if (st - sb).length() > 1. { (st - sb).normalized() } else { (s1 - o1).normalized() };
    let n = egui::vec2(-d.y, d.x);
    painter.line_segment([sb, st], Stroke::new(2.5, cor));
    painter.add(egui::Shape::convex_polygon(vec![st + d * 14., st + n * 7., st - n * 7.], cor, Stroke::NONE));
    painter.circle_stroke(st, PEGA / 2., Stroke::new(1.5, cor));
    leitura(ui, rect, st + n * 22. + d * 10., texto_leitura(c, expr, valor, trava), cor);
    if hr.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
        let m = ui.input(|i| i.pointer.interact_pos())?;
        let (ro, rd) = cam.ray(rect, m);
        let t = param_no_eixo(ro, rd, origem_eixo, dir)?;
        let passo = crate::ui_ext::passo_3d(ui, ppm);
        return Some(valor_ao_passo(t - deslocamento_da_ponta, passo));
    }
    if hr.hovered() && !trava {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
    }
    None
}

/// Setas duplas de deslocamento X e Y, no ponto de ancoragem da seção.
fn alcas_de_deslocamento(app: &mut App, ui: &mut egui::Ui, rect: Rect, cam: &Camera, id: Id, m: &StructMember, fr: &MemberFrame) {
    let c = (v3(fr.start) + v3(fr.end)) * 0.5;
    let (x, y) = (v3(fr.x), v3(fr.y));
    let p = c + x * fr.off[0] as f32 + y * fr.off[1] as f32;
    let meia = (fr.w.max(fr.h) * 0.75).max(12.);
    // cada seta dupla tem duas pontas; o eixo de medida passa pelo centro da linha, deslocado pelo outro eixo
    let eixos = [(Campo::OffX, x, y * fr.off[1] as f32, fr.off[0], 0u8), (Campo::OffY, y, x * fr.off[0] as f32, fr.off[1], 2u8)];
    for (campo, dir, lado, valor, salt) in eixos {
        for (k, sinal) in [1f64, -1.].into_iter().enumerate() {
            let ponta = p + dir * (sinal * meia) as f32;
            if let Some(v) = alca_linear(ui, rect, cam, id, salt + k as u8, campo, c + lado, dir, p, ponta, expressao(m, campo), valor, sinal * meia) {
                escrever(app, id, campo, v);
            }
        }
    }
}

/// Arco de rotação em volta do eixo do membro, com o ímã de 45°.
fn alca_de_rotacao(app: &mut App, ui: &mut egui::Ui, rect: Rect, cam: &Camera, id: Id, m: &StructMember, fr: &MemberFrame) {
    let c = (v3(fr.start) + v3(fr.end)) * 0.5;
    let (z, x0) = (v3(fr.z), v3(fr.x0));
    let y0 = z.cross(x0);
    let raio = (fr.w.max(fr.h) * 0.5 + 18.) as f32;
    let em = |a: f64| c + (x0 * (a.to_radians() as f32).cos() + y0 * (a.to_radians() as f32).sin()) * raio;
    let trava = !arrastavel(&m.angle);
    let painter = ui.painter_at(rect);
    // marcas dos múltiplos de 45° (as paradas do ímã)
    let marca = ui.visuals().weak_text_color();
    for k in -3..=4 {
        if let (Some(a), Some(b)) = (cam.project(rect, c + (em(k as f64 * 45.) - c) * 0.93), cam.project(rect, c + (em(k as f64 * 45.) - c) * 1.07)) {
            painter.line_segment([a, b], Stroke::new(1., marca));
        }
    }
    let fim = fr.angle;
    let passos = 48;
    let pts: Vec<egui::Pos2> = (0..=passos).filter_map(|i| cam.project(rect, em(fim * i as f64 / passos as f64))).collect();
    let Some(ponta) = cam.project(rect, em(fim)) else { return };
    let hid = egui::Id::unique(("st_alca_rot", id.0));
    let hr = ui.interact(Rect::from_center_size(ponta, egui::vec2(PEGA, PEGA)), hid, if trava { egui::Sense::hover() } else { egui::Sense::drag() });
    let cor = if hr.hovered() || hr.dragged() { crate::theme::st_warn() } else { crate::theme::accent(ui.visuals()) };
    painter.add(egui::Shape::line(pts, Stroke::new(2.5, cor)));
    painter.circle_stroke(ponta, 9., Stroke::new(1.5, cor));
    painter.circle_filled(ponta, 4., cor);
    leitura(ui, rect, ponta + egui::vec2(14., -10.), texto_leitura(Campo::Angulo, &m.angle, fim, trava), cor);
    if hr.dragged() {
        if let Some(mouse) = ui.input(|i| i.pointer.interact_pos()) {
            let (ro, rd) = cam.ray(rect, mouse);
            if let Some(bruto) = angulo_no_plano(ro, rd, c, z, x0) {
                let fino = ui.input(|i| i.modifiers.shift);
                escrever(app, id, Campo::Angulo, angulo_arrastado(bruto, fino));
            }
        }
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    }
}

/// Alças de prolongar o início e o fim, com a leitura em mm.
fn alcas_de_prolongar(app: &mut App, ui: &mut egui::Ui, rect: Rect, cam: &Camera, id: Id, m: &StructMember, fr: &MemberFrame) {
    let z = v3(fr.z);
    let (a, b) = (v3(fr.start), v3(fr.end));
    // início: o valor cresce para trás do começo da linha; fim: para a frente do fim
    let casos = [(Campo::Ext1, a, -z, fr.ext.0, 4u8), (Campo::Ext2, b, z, fr.ext.1, 5u8)];
    for (campo, base, dir, valor, salt) in casos {
        let ponta = base + dir * valor as f32;
        if let Some(v) = alca_linear(ui, rect, cam, id, salt, campo, base, dir, base, ponta, expressao(m, campo), valor, 0.) {
            escrever(app, id, campo, v);
        }
    }
}

/// Botões de espelhar a seção e alinhar a uma aresta, junto do centro do membro.
fn botoes(app: &mut App, ui: &mut egui::Ui, rect: Rect, cam: &Camera, id: Id, m: &StructMember, fr: &MemberFrame) {
    let c = (v3(fr.start) + v3(fr.end)) * 0.5;
    let Some(s) = cam.project(rect, c) else { return };
    let fg = ui.visuals().strong_text_color();
    let espelhar = crate::handles::tag(ui, rect, s + egui::vec2(70., -40.), format!("⇋ {}{}", tr("Espelhar a seção"), if m.mirror { " ✓" } else { "" }), fg, ("st_espelhar", id.0))
        .on_hover_text(tr("Inverte a seção de lado (perfis U, L e T): o contorno vira imagem no espelho em torno do eixo vertical da seção."));
    if espelhar.clicked() {
        if let Some(FeatureKind::StructMember(mm)) = app.open.doc.feature_mut(id).map(|f| &mut f.kind) {
            mm.mirror = !mm.mirror;
        }
    }
    let arestas = direcoes_das_arestas(app);
    let alinhar = crate::handles::tag(ui, rect, s + egui::vec2(70., -16.), format!("∥ {}", tr("Alinhar à aresta")), if arestas.is_empty() { ui.visuals().weak_text_color() } else { fg }, ("st_alinhar", id.0))
        .on_hover_text(tr("Gira a seção para o eixo X dela ficar paralelo à aresta selecionada no 3D. Selecione uma aresta e clique."));
    if alinhar.clicked() {
        match arestas.first().and_then(|e| align_angle(fr.z, fr.x0, *e)) {
            Some(a) => escrever(app, id, Campo::Angulo, (a * 1000.).round() / 1000.),
            None => app.toast(tr("Selecione no 3D uma aresta que não seja paralela ao membro"), true),
        }
    }
}

/// Desenha e trata as alças do Membro em edição (a primeira linha guia mostra as alças; o valor vale
/// para todas as linhas do comando).
pub fn overlay(app: &mut App, ui: &mut egui::Ui, rect: Rect, cam: &Camera, id: Id) {
    let Some(m) = membro(app, id) else { return };
    let Some(fr) = member_frames(&app.open.doc, id).into_iter().next() else { return };
    alcas_de_deslocamento(app, ui, rect, cam, id, &m, &fr);
    // a seção pode ter mudado com as alças acima: relê antes das seguintes
    let (m, fr) = match (membro(app, id), member_frames(&app.open.doc, id).into_iter().next()) {
        (Some(m), Some(f)) => (m, f),
        _ => return,
    };
    alca_de_rotacao(app, ui, rect, cam, id, &m, &fr);
    alcas_de_prolongar(app, ui, rect, cam, id, &m, &fr);
    botoes(app, ui, rect, cam, id, &m, &fr);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn membro_de_teste(off_x: &str, angle: &str) -> (Document, Id) {
        let s = |x: &str| x.to_string();
        let mut d = Document::new("t");
        let l = CurveRef::Ref(d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed([0., 0., 0.]), b: PointRef::Fixed([0., 0., 300.]), start: s("0"), end: s("0") }));
        let id = d.add_feature(FeatureKind::StructMember(StructMember {
            paths: vec![l],
            family: s("Tubo quadrado"),
            designation: s("40x3"),
            kind: SectionKind::TuboQuadrado,
            dims: vec![s("40"), s("3")],
            anchor: 0,
            angle: s(angle),
            off_x: s(off_x),
            off_y: s("0"),
            ext1: s("0"),
            ext2: s("0"),
            corner: CornerMode::MeiaEsquadria,
            gap: s("0"),
            joints: vec![],
            material: String::new(),
            mirror: false,
        }));
        (d, id)
    }

    fn campo_do(d: &Document, id: Id) -> StructMember {
        match &d.feature(id).unwrap().kind {
            FeatureKind::StructMember(m) => m.clone(),
            _ => unreachable!(),
        }
    }

    #[test]
    fn o_ima_de_45_gruda_perto_e_solta_longe() {
        assert_eq!(gruda_45(43.2, 4.), 45.);
        assert_eq!(gruda_45(-91.0, 4.), -90.);
        assert_eq!(gruda_45(135.5, 4.), 135.);
        assert_eq!(gruda_45(30., 4.), 30.);
        // arrasto: passo de 1°, ímã ligado; com o passo fino fica em décimos e sem ímã
        assert_eq!(angulo_arrastado(44.6, false), 45.);
        assert_eq!(angulo_arrastado(20.4, false), 20.);
        assert_eq!(angulo_arrastado(44.64, true), 44.6);
    }

    #[test]
    fn o_mouse_acha_o_ponto_do_eixo_e_o_angulo_do_plano() {
        // raio vertical sobre o ponto x = 17 de um eixo X: t = 17
        let t = param_no_eixo(Vec3::new(17., 5., 100.), Vec3::new(0., 0., -1.), Vec3::ZERO, Vec3::X).unwrap();
        assert!((t - 17.).abs() < 1e-9);
        assert!(param_no_eixo(Vec3::new(0., 5., 100.), Vec3::X, Vec3::ZERO, Vec3::X).is_none(), "raio paralelo ao eixo não mede");
        // ângulo: o ponto (cos 60°, sin 60°) no plano z = 0 visto de cima mede 60°, no sentido do comando
        let a = angulo_no_plano(Vec3::new(0.5, 0.866_025_4, 50.), Vec3::new(0., 0., -1.), Vec3::ZERO, Vec3::Z, Vec3::X).unwrap();
        assert!((a - 60.).abs() < 1e-4, "{a}");
        let g = fenkra_solidos::geo3d::rotate([1., 0., 0.], [0., 0., 1.], 60f64.to_radians());
        assert!((g[1] / g[0] - 60f64.to_radians().tan()).abs() < 1e-9, "o sentido bate com o do comando");
    }

    #[test]
    fn arrastar_e_digitar_dao_o_mesmo_valor_no_membro() {
        // arrasto: o mouse aponta o ponto x = 17,4 do eixo; passo de 1 mm
        let (mut d, id) = membro_de_teste("0", "0");
        let t = param_no_eixo(Vec3::new(17.4, 3., 90.), Vec3::new(0., 0., -1.), Vec3::ZERO, Vec3::X).unwrap();
        let v = valor_ao_passo(t, 1.);
        if let Some(FeatureKind::StructMember(m)) = d.feature_mut(id).map(|f| &mut f.kind) {
            gravar(m, Campo::OffX, v);
        }
        let arrastado = campo_do(&d, id);
        // digitado: o painel grava o texto "17" no mesmo campo
        let (mut d2, id2) = membro_de_teste("0", "0");
        if let Some(FeatureKind::StructMember(m)) = d2.feature_mut(id2).map(|f| &mut f.kind) {
            m.off_x = "17".into();
        }
        assert_eq!(arrastado.off_x, campo_do(&d2, id2).off_x);
        let f1 = member_frames(&d, id).remove(0);
        let f2 = member_frames(&d2, id2).remove(0);
        assert_eq!(f1.off, f2.off);
        assert_eq!(f1.off, [17., 0.]);
        // ângulo: 44° gruda em 45° e o texto é o mesmo que o painel mostraria para 45
        let (mut d3, id3) = membro_de_teste("0", "0");
        if let Some(FeatureKind::StructMember(m)) = d3.feature_mut(id3).map(|f| &mut f.kind) {
            gravar(m, Campo::Angulo, angulo_arrastado(44.2, false));
        }
        assert_eq!(member_frames(&d3, id3)[0].angle, 45.);
    }

    #[test]
    fn expressao_e_parametro_travam_a_alca() {
        assert!(arrastavel("12,5") && arrastavel(" 12.5 ") && arrastavel("-3"));
        assert!(!arrastavel("L/2") && !arrastavel("largura") && !arrastavel("2*3"));
        // a alça lê o valor avaliado mesmo travada
        let (mut d, id) = membro_de_teste("L/2", "0");
        d.add_param("L", "30", Unit::Comprimento).unwrap();
        let fr = member_frames(&d, id).remove(0);
        assert_eq!(fr.off[0], 15.);
        assert!(!arrastavel(expressao(&campo_do(&d, id), Campo::OffX)));
    }

    #[test]
    fn a_ponta_da_alca_de_prolongar_cai_na_projecao_da_camera() {
        // com a câmera padrão, a ponta de uma alça a 25 mm do eixo projeta em um ponto que o raio devolve
        let cam = Camera::default();
        let rect = Rect::from_min_size(egui::pos2(0., 0.), egui::vec2(800., 600.));
        let p = Vec3::new(0., 0., 25.);
        let tela = cam.project(rect, p).unwrap();
        let (ro, rd) = cam.ray(rect, tela);
        let t = param_no_eixo(ro, rd, Vec3::ZERO, Vec3::Z).unwrap();
        assert!((t - 25.).abs() < 0.05, "{t}");
    }
}
