// SPDX-License-Identifier: LGPL-2.1-or-later
//! Construtor guiado de rugosidade e solda (B251): cada campo fica onde está no símbolo, os valores
//! comuns são botões e uma prévia grande mostra o símbolo como ele sai na folha (as mesmas
//! primitivas do PDF, SVG e DXF). Os dados são os da nota; sem nenhum extra a nota volta a ser a
//! que o texto antigo criava (`simbolos::normalizar`).

use super::*;
use fenkra_core::pmi::{WeldContour, WeldKind};
use fenkra_desenho::simbolos::{self as sb, Comprimento, RugTipo};

/// Preenchimento como malha em leque: sem o anti-serrilhado do polígono convexo, que faz espinhos
/// nos triângulos finos do sombreado.
pub(super) fn preencher(pintor: &egui::Painter, pts: Vec<Pos2>, cor: Color32) {
    if pts.len() < 3 {
        return;
    }
    let mut m = egui::Mesh::default();
    for p in &pts {
        m.colored_vertex(*p, cor);
    }
    for k in 1..pts.len() - 1 {
        m.add_triangle(0, k as u32, k as u32 + 1);
    }
    pintor.add(egui::Shape::mesh(m));
}

/// Primitivas do símbolo da nota, com a ponta da chamada e a posição zeradas (só o símbolo).
fn itens_do_simbolo(n: &Note, th: f64) -> Vec<Item> {
    let mut d = Drawing::new("", Format::A4);
    d.text_height = th;
    let mut m = n.clone();
    m.pos = [0., 0.];
    m.leader = None;
    layout::note_items(&d, &m, &mut Models::new(None))
}

/// Caixa (mm) de todas as primitivas.
fn caixa(items: &[Item]) -> Option<(V2, V2)> {
    let mut lo = [f64::MAX; 2];
    let mut hi = [f64::MIN; 2];
    let mut add = |p: V2| {
        for k in 0..2 {
            lo[k] = lo[k].min(p[k]);
            hi[k] = hi[k].max(p[k]);
        }
    };
    for it in items {
        match &it.prim {
            Prim::Line { pts, .. } | Prim::Fill { pts } => pts.iter().for_each(|p| add(*p)),
            Prim::Text { pos, text, size, align, .. } => {
                let w = layout::text_width(text, *size);
                let x0 = match align {
                    Align::Left => pos[0],
                    Align::Center => pos[0] - w / 2.,
                    Align::Right => pos[0] - w,
                };
                add([x0, pos[1]]);
                add([x0 + w, pos[1] + size]);
            }
        }
    }
    (lo[0] <= hi[0]).then_some((lo, hi))
}

/// Prévia grande do símbolo, no papel (fundo branco, traço preto), ajustada ao quadro.
pub(super) fn previa(ui: &mut egui::Ui, n: &Note, th: f64) {
    let items = itens_do_simbolo(n, th);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width().clamp(200., 420.), 90.), egui::Sense::hover());
    let pintor = ui.painter_at(rect);
    pintor.rect_filled(rect, 4., Color32::WHITE);
    pintor.rect_stroke(rect, 4., Stroke::new(1., ui.visuals().widgets.noninteractive.bg_stroke.color), egui::StrokeKind::Inside);
    let Some((lo, hi)) = caixa(&items) else { return };
    let (w, h) = ((hi[0] - lo[0]).max(1.), (hi[1] - lo[1]).max(1.));
    let z = ((rect.width() as f64 - 24.) / w).min((rect.height() as f64 - 24.) / h).clamp(0.5, 40.) as f32;
    let centro = rect.center();
    let c = [(lo[0] + hi[0]) / 2., (lo[1] + hi[1]) / 2.];
    let to = |p: V2| Pos2::new(centro.x + (p[0] - c[0]) as f32 * z, centro.y - (p[1] - c[1]) as f32 * z);
    for it in &items {
        match &it.prim {
            Prim::Line { pts, style } => {
                let pts: Vec<Pos2> = pts.iter().map(|p| to(*p)).collect();
                let stroke = Stroke::new((style_px(*style, z) * 1.3).max(1.2), Color32::BLACK);
                let dash = style.dash();
                if dash.is_empty() {
                    pintor.add(egui::Shape::line(pts, stroke));
                } else {
                    pintor.extend(egui::Shape::dashed_line(&pts, stroke, (dash[0] as f32 * z).max(3.), (dash[1] as f32 * z).max(2.)));
                }
            }
            Prim::Fill { pts } => preencher(&pintor, pts.iter().map(|p| to(*p)).collect(), Color32::BLACK),
            Prim::Text { pos, text, size, align, .. } => {
                let px = (*size as f32 * z / 0.72).max(6.);
                let g = pintor.layout_no_wrap(text.clone(), egui::FontId::proportional(px), Color32::BLACK);
                let fx = match align {
                    Align::Left => 0.,
                    Align::Center => -g.size().x / 2.,
                    Align::Right => -g.size().x,
                };
                pintor.galley(to(*pos) + Vec2::new(fx, -px * 0.95), g, Color32::BLACK);
            }
        }
    }
}

/// Botões de valores comuns: o clicado devolve o texto.
fn botoes(ui: &mut egui::Ui, valores: &[&str], atual: &str, dica: &str) -> Option<String> {
    let mut escolhido = None;
    ui.horizontal_wrapped(|ui| {
        for v in valores {
            if ui.selectable_label(atual.trim() == *v, *v).on_hover_text(dica).clicked() {
                escolhido = Some(v.to_string());
            }
        }
    });
    escolhido
}

/// Construtor da rugosidade: campos nas posições do símbolo ISO 1302.
pub(super) fn rugosidade(ui: &mut egui::Ui, n: &mut Note) {
    let (mut param, mut valor) = sb::rug_partes(&n.text);
    let mut r = n.rough.clone().unwrap_or_default();
    ui.label(RichText::new(tr("Tipo do símbolo")).strong());
    ui.horizontal(|ui| {
        for t in RugTipo::ALL {
            ui.selectable_value(&mut r.tipo, t, tr(t.label()));
        }
    });
    ui.add_space(4.);
    egui::Grid::new("rug_campos").num_columns(2).spacing([8., 6.]).show(ui, |ui| {
        ui.label(tr("Em cima da barra"));
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut r.amostragem).desired_width(44.).hint_text("2,5")).on_hover_text(tr("Comprimento de amostragem (mm), antes do valor: 2,5/Ra 1,6"));
            ui.label("/");
            for p in sb::PARAMETROS {
                if ui.selectable_label(param == p, p).on_hover_text(tr("Parâmetro de rugosidade")).clicked() {
                    param = p.to_string();
                }
            }
            ui.add(egui::TextEdit::singleline(&mut valor).desired_width(56.).hint_text("1,6")).on_hover_text(tr("Valor da rugosidade em µm"));
        });
        ui.end_row();
        // B149: acabamento em várias linhas (outras exigências acima da primeira)
        ui.label(tr("Outras exigências"));
        let mut outras = r.outras.join("\n");
        if ui.add(egui::TextEdit::multiline(&mut outras).desired_rows(2).desired_width(180.).hint_text("Rz 6,3")).on_hover_text(tr("Uma exigência por linha, empilhadas acima da primeira (ISO 1302)")).changed() {
            r.outras = outras.lines().map(|l| l.to_string()).collect();
        }
        ui.end_row();
        ui.label(tr("Embaixo da barra"));
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut r.metodo).desired_width(96.).hint_text(tr("método"))).on_hover_text(tr("Processo de fabricação, escrito abaixo da barra"));
            ui.add(egui::TextEdit::singleline(&mut r.sobremedida).desired_width(44.).hint_text("0,3")).on_hover_text(tr("Sobremedida de usinagem (mm)"));
        });
        ui.end_row();
        ui.label(tr("Direção das estrias"));
        ui.horizontal(|ui| {
            if ui.selectable_label(r.direcao.is_empty(), "—").on_hover_text(tr("Sem indicação de direção")).clicked() {
                r.direcao.clear();
            }
            for d in sb::DIRECOES {
                if ui.selectable_label(r.direcao == d, d).on_hover_text(tr("Direção das estrias: = paralela, ⊥ perpendicular, X cruzada, M multidirecional, C circular, R radial")).clicked() {
                    r.direcao = d.to_string();
                }
            }
        });
        ui.end_row();
    });
    ui.add_space(2.);
    ui.label(RichText::new(tr("Valores comuns (µm)")).weak().small());
    if let Some(v) = botoes(ui, &sb::VALORES, &valor, &tr("Escolhe o valor da rugosidade")) {
        valor = v;
        if param.is_empty() {
            param = "Ra".into();
        }
    }
    ui.label(RichText::new(tr("Métodos comuns")).weak().small());
    if let Some(m) = botoes(ui, &sb::METODOS, &r.metodo, &tr("Escolhe o processo de fabricação")) {
        r.metodo = m;
    }
    ui.label(RichText::new(tr("Amostragem comum (mm)")).weak().small());
    if let Some(a) = botoes(ui, &sb::AMOSTRAGENS, &r.amostragem, &tr("Escolhe o comprimento de amostragem")) {
        r.amostragem = a;
    }
    n.text = sb::rug_texto(&param, &valor);
    n.rough = Some(r);
    sb::normalizar(n);
}

/// Construtor da solda (ISO 2553, sistema A): lado da seta e outro lado com o tipo de cordão em
/// botões, medida, comprimento (quantidade, comprimento, passo), contorno e solda em campo.
pub(super) fn solda(ui: &mut egui::Ui, n: &mut Note) {
    let mut w = n.weld.clone().unwrap_or_else(sb::solda_padrao);
    let (mut seta, mut outro) = sb::lados(&w);
    let lado = |ui: &mut egui::Ui, titulo: &str, dica: &str, atual: &mut Option<WeldKind>| {
        ui.label(RichText::new(tr(titulo)).strong()).on_hover_text(tr(dica));
        ui.horizontal_wrapped(|ui| {
            if ui.selectable_label(atual.is_none(), "—").on_hover_text(tr("Sem cordão deste lado")).clicked() {
                *atual = None;
            }
            for k in WeldKind::ALL {
                if ui.selectable_label(*atual == Some(k), k.symbol()).on_hover_text(tr(k.label())).clicked() {
                    *atual = Some(k);
                }
            }
        });
    };
    lado(ui, "Lado da seta", "Cordão desenhado sobre a linha cheia: o lado para onde a seta aponta", &mut seta);
    lado(ui, "Outro lado", "Cordão desenhado sob a linha tracejada: o lado oposto ao da seta", &mut outro);
    if seta.is_none() && outro.is_none() {
        // sem cordão nenhum não há símbolo: volta ao que estava
        (seta, outro) = sb::lados(&w);
    }
    sb::definir_lados(&mut w, seta, outro);
    ui.add_space(4.);
    let mut medida = n.text.clone();
    egui::Grid::new("solda_campos").num_columns(2).spacing([8., 6.]).show(ui, |ui| {
        ui.label(tr("Medida"));
        ui.add(egui::TextEdit::singleline(&mut medida).desired_width(70.).hint_text("a4")).on_hover_text(tr("Medida da seção, à esquerda do cordão: a = garganta, z = perna"));
        ui.end_row();
        ui.label(tr("Comprimento"));
        let mut c = Comprimento::ler(&w.extra.length);
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut c.n).desired_width(30.).hint_text("n")).on_hover_text(tr("Quantidade de trechos"));
            ui.label("×");
            ui.add(egui::TextEdit::singleline(&mut c.l).desired_width(50.).hint_text("l")).on_hover_text(tr("Comprimento de cada trecho (mm)"));
            ui.label("(");
            ui.add(egui::TextEdit::singleline(&mut c.e).desired_width(50.).hint_text(tr("passo"))).on_hover_text(tr("Passo entre os trechos (mm), entre parênteses"));
            ui.label(")");
        });
        w.extra.length = c.compor();
        ui.end_row();
        ui.label(tr("Acabamento"));
        ui.horizontal(|ui| {
            for c in [WeldContour::Nenhum, WeldContour::Plano, WeldContour::Convexo, WeldContour::Concavo] {
                ui.selectable_value(&mut w.extra.contour, c, tr(sb::contorno_label(c))).on_hover_text(tr("Forma da face do cordão depois do acabamento"));
            }
        });
        ui.end_row();
        ui.label(tr("Cauda (processo)"));
        ui.add(egui::TextEdit::singleline(&mut w.extra.process).desired_width(130.).hint_text("135 / ISO 5817-C")).on_hover_text(tr("Processo de soldagem (ISO 4063) e norma de qualidade, na cauda da seta"));
        ui.end_row();
    });
    ui.label(RichText::new(tr("Medidas comuns")).weak().small());
    if let Some(m) = botoes(ui, &sb::MEDIDAS_SOLDA, &medida, &tr("Escolhe a medida do cordão")) {
        medida = m;
    }
    ui.horizontal(|ui| {
        ui.checkbox(&mut w.all_around, tr("todo o contorno")).on_hover_text(tr("Círculo na dobra da seta: solda em todo o contorno da peça"));
        ui.checkbox(&mut w.extra.field, tr("solda de campo")).on_hover_text(tr("Bandeira: a solda é feita na obra, não na oficina"));
    });
    n.text = medida;
    n.weld = Some(w);
    sb::normalizar(n);
}
