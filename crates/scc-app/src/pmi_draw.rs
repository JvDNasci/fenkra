//! Desenho das anotações 3D com cara de desenho técnico: quadro de controle geométrico (ISO 1101)
//! em casas, símbolo de referência com triângulo, cotas com tolerâncias empilhadas, chamadas com
//! ponta escolhida, legenda em tabela. O estilo de cada anotação herda o estilo geral.

use eframe::egui::{self, Color32, FontFamily, FontId, Painter, Pos2, Rect, Stroke};
use scc_core::pmi::*;

/// Cor de realce da anotação selecionada.
pub const HL: Color32 = Color32::from_rgb(255, 150, 0);

/// Estilo já resolvido (anotação → estilo geral → padrão).
pub struct Pen {
    pub fam: FontFamily,
    pub size: f32,
    pub line: Color32,
    pub text: Color32,
    pub width: f32,
    pub arrow: PmiArrow,
    pub bg: PmiBg,
    pub frame: PmiFrame,
    pub sel: bool,
}

/// Fontes disponíveis para as anotações (registradas na partida).
pub fn fonts() -> Vec<String> {
    crate::app::PMI_FONTS.get().cloned().unwrap_or_default()
}

impl Pen {
    pub fn new(l: &PmiStyle, g: &PmiStyle, sel: bool) -> Pen {
        let num = |a: f32, b: f32, d: f32| if a > 0. { a } else if b > 0. { b } else { d };
        let font = if !l.font.is_empty() { l.font.clone() } else if !g.font.is_empty() { g.font.clone() } else { "Arial".into() };
        let fam = if fonts().contains(&font) { FontFamily::Name(font.as_str().into()) } else { FontFamily::Proportional };
        let col = |a: Option<[u8; 3]>, b: Option<[u8; 3]>, d: [u8; 3]| {
            let x = a.or(b).unwrap_or(d);
            Color32::from_rgb(x[0], x[1], x[2])
        };
        Pen {
            fam,
            size: num(l.size, g.size, 13.),
            line: col(l.color, g.color, [20, 40, 110]),
            text: col(l.text_color, g.text_color, [12, 14, 24]),
            width: num(l.line_width, g.line_width, 1.3),
            arrow: if l.arrow != PmiArrow::Padrao { l.arrow } else if g.arrow != PmiArrow::Padrao { g.arrow } else { PmiArrow::Cheia },
            bg: if l.bg != PmiBg::Padrao { l.bg } else { g.bg },
            frame: if l.frame != PmiFrame::Padrao { l.frame } else { g.frame },
            sel,
        }
    }
    pub fn stroke(&self) -> Stroke {
        Stroke::new(if self.sel { self.width + 1. } else { self.width }, if self.sel { HL } else { self.line })
    }
    pub fn thin(&self) -> Stroke {
        Stroke::new((self.width * 0.75).max(0.8), if self.sel { HL } else { self.line })
    }
    pub fn font(&self, k: f32) -> FontId {
        FontId::new(self.size * k, self.fam.clone())
    }
    fn fill(&self, kind_default: PmiBg) -> Option<Color32> {
        match if self.bg == PmiBg::Padrao { kind_default } else { self.bg } {
            PmiBg::Branco => Some(Color32::from_rgba_unmultiplied(255, 255, 255, 235)),
            PmiBg::Escuro => Some(Color32::from_rgba_unmultiplied(40, 30, 22, 225)),
            _ => None,
        }
    }
    fn text_col(&self, fill: Option<Color32>) -> Color32 {
        match fill {
            Some(f) if f.r() < 100 => Color32::WHITE,
            _ => self.text,
        }
    }
    fn frame_of(&self, kind_default: PmiFrame) -> PmiFrame {
        if self.frame == PmiFrame::Padrao { kind_default } else { self.frame }
    }

    /// Ponta da chamada em `tip`, vindo de `from`.
    pub fn arrow(&self, p: &Painter, tip: Pos2, from: Pos2) {
        p.extend(self.arrow_shapes(tip, from));
    }

    fn arrow_shapes(&self, tip: Pos2, from: Pos2) -> Vec<egui::Shape> {
        let d = (tip - from).normalized();
        if !d.x.is_finite() || (tip - from).length() < 2. {
            return vec![];
        }
        let n = egui::vec2(-d.y, d.x);
        let (s, w) = (self.size * 0.8, self.size * 0.27);
        let col = if self.sel { HL } else { self.line };
        match self.arrow {
            PmiArrow::Cheia | PmiArrow::Padrao => vec![egui::Shape::convex_polygon(vec![tip, tip - d * s + n * w, tip - d * s - n * w], col, Stroke::NONE)],
            PmiArrow::Aberta => vec![egui::Shape::line(vec![tip - d * s + n * w, tip, tip - d * s - n * w], self.stroke())],
            PmiArrow::Ponto => vec![egui::Shape::circle_filled(tip, self.size * 0.22, col)],
            PmiArrow::Barra => {
                let k = (d + n).normalized() * s * 0.45;
                vec![egui::Shape::line_segment([tip - k, tip + k], Stroke::new(self.width + 0.6, col))]
            }
            PmiArrow::Nenhuma => vec![],
        }
    }

    /// Lugar reservado na pintura para a chamada ficar por baixo da caixa (desenhada depois).
    pub fn reserve(p: &Painter) -> egui::layers::ShapeIdx {
        p.add(egui::Shape::Noop)
    }

    /// Chamada da caixa `r` até `anchor`: sai do lado da caixa voltado para o ponto, com um
    /// patamar curto, e fica por baixo da caixa (no lugar reservado). Nunca cruza a caixa.
    pub fn leader(&self, p: &Painter, slot: egui::layers::ShapeIdx, r: Rect, anchor: Pos2) {
        let c = r.center();
        let d = anchor - c;
        if r.expand(2.).contains(anchor) {
            return;
        }
        let land = self.size * 0.9;
        // lado: compara a direção com a proporção da caixa
        let (att, out) = if d.x.abs() * r.height() >= d.y.abs() * r.width() {
            if d.x < 0. { (egui::pos2(r.left(), c.y), egui::vec2(-land, 0.)) } else { (egui::pos2(r.right(), c.y), egui::vec2(land, 0.)) }
        } else if d.y < 0. {
            (egui::pos2(c.x, r.top()), egui::vec2(0., -land * 0.7))
        } else {
            (egui::pos2(c.x, r.bottom()), egui::vec2(0., land * 0.7))
        };
        let knee = att + out;
        let mut v = vec![egui::Shape::line(vec![att, knee, anchor], self.stroke())];
        v.extend(self.arrow_shapes(anchor, knee));
        p.set(slot, egui::Shape::Vec(v));
    }

    /// Bloco de texto (várias linhas) centrado em `at`, com moldura e fundo conforme o tipo.
    pub fn text_block(&self, p: &Painter, at: Pos2, txt: &str, frame_default: PmiFrame, bg_default: PmiBg) -> Rect {
        let fill = self.fill(bg_default);
        let tc = self.text_col(fill);
        let g = p.layout_no_wrap(txt.to_string(), self.font(1.), tc);
        let pad = egui::vec2(self.size * 0.45, self.size * 0.25);
        let r = Rect::from_center_size(at, g.size() + pad * 2.);
        let frame = self.frame_of(frame_default);
        let round = if frame == PmiFrame::Arredondada { (r.height() / 2.).min(r.width() / 2.) } else { 1. };
        if let Some(f) = fill {
            p.rect_filled(r, round, f);
        }
        if matches!(frame, PmiFrame::Caixa | PmiFrame::Arredondada) {
            p.rect_stroke(r, round, self.stroke(), egui::StrokeKind::Inside);
        } else if self.sel {
            p.rect_stroke(r, 2., Stroke::new(1., HL), egui::StrokeKind::Outside);
        }
        p.galley(r.min + pad, g, tc);
        r
    }

    /// Texto de cota: valor sem moldura (salvo pedido) e tolerâncias diferentes empilhadas à direita.
    pub fn dim_text(&self, p: &Painter, at: Pos2, main: &str, stack: Option<&(String, String)>) -> Rect {
        let Some((up, lo)) = stack else { return self.text_block(p, at, main, PmiFrame::Nenhuma, PmiBg::Nenhum) };
        let fill = self.fill(PmiBg::Nenhum);
        let tc = self.text_col(fill);
        let g = p.layout_no_wrap(main.to_string(), self.font(1.), tc);
        let gu = p.layout_no_wrap(up.clone(), self.font(0.62), tc);
        let gl = p.layout_no_wrap(lo.clone(), self.font(0.62), tc);
        let w = g.size().x + 3. + gu.size().x.max(gl.size().x);
        let h = g.size().y.max(gu.size().y + gl.size().y);
        let r = Rect::from_center_size(at, egui::vec2(w + 8., h + 4.));
        if let Some(f) = fill {
            p.rect_filled(r, 1., f);
        }
        if matches!(self.frame_of(PmiFrame::Nenhuma), PmiFrame::Caixa | PmiFrame::Arredondada) {
            p.rect_stroke(r, 1., self.stroke(), egui::StrokeKind::Inside);
        } else if self.sel {
            p.rect_stroke(r, 2., Stroke::new(1., HL), egui::StrokeKind::Outside);
        }
        let x0 = r.left() + 4.;
        p.galley(egui::pos2(x0, r.center().y - g.size().y / 2.), g.clone(), tc);
        let x1 = x0 + g.size().x + 3.;
        p.galley(egui::pos2(x1, r.top() + 2.), gu.clone(), tc);
        p.galley(egui::pos2(x1, r.top() + 2. + gu.size().y), gl, tc);
        r
    }

    /// Quadro de controle geométrico: a primeira casa (símbolo) ocupa todas as linhas; cada linha
    /// tem as casas de tolerância e de referências. `at` é o centro do quadro.
    pub fn fcf(&self, p: &Painter, at: Pos2, sym: &str, rows: &[Vec<String>]) -> Rect {
        let h = self.size * 1.7;
        let pad = self.size * 0.5;
        let fill = self.fill(PmiBg::Branco);
        let tc = self.text_col(fill);
        let gs = p.layout_no_wrap(sym.to_string(), self.font(1.25), tc);
        let w0 = (gs.size().x + pad * 2.).max(h);
        let cells: Vec<Vec<std::sync::Arc<egui::Galley>>> = rows.iter().map(|r| r.iter().map(|c| p.layout_no_wrap(c.clone(), self.font(1.), tc)).collect()).collect();
        let row_w: Vec<f32> = cells.iter().map(|r| r.iter().map(|g| (g.size().x + pad * 2.).max(h * 0.9)).sum()).collect();
        let total_w = w0 + row_w.iter().cloned().fold(0., f32::max);
        let total_h = h * rows.len().max(1) as f32;
        let r = Rect::from_center_size(at, egui::vec2(total_w, total_h));
        let st = self.stroke();
        // fundo e casa do símbolo
        if let Some(f) = fill {
            p.rect_filled(Rect::from_min_size(r.min, egui::vec2(w0, total_h)), 0., f);
        }
        p.rect_stroke(Rect::from_min_size(r.min, egui::vec2(w0, total_h)), 0., st, egui::StrokeKind::Middle);
        p.galley(egui::pos2(r.left() + (w0 - gs.size().x) / 2., r.top() + (total_h - gs.size().y) / 2.), gs, tc);
        for (i, row) in cells.iter().enumerate() {
            let top = r.top() + h * i as f32;
            let row_rect = Rect::from_min_size(egui::pos2(r.left() + w0, top), egui::vec2(row_w[i], h));
            if let Some(f) = fill {
                p.rect_filled(row_rect, 0., f);
            }
            p.rect_stroke(row_rect, 0., st, egui::StrokeKind::Middle);
            let mut x = r.left() + w0;
            for (k, g) in row.iter().enumerate() {
                let cw = (g.size().x + pad * 2.).max(h * 0.9);
                if k > 0 {
                    p.line_segment([egui::pos2(x, top), egui::pos2(x, top + h)], st);
                }
                p.galley(egui::pos2(x + (cw - g.size().x) / 2., top + (h - g.size().y) / 2.), g.clone(), tc);
                x += cw;
            }
        }
        let mut out = r;
        out.max.x = r.left() + total_w;
        out
    }

    /// Símbolo de referência: triângulo cheio apoiado na peça em `anchor`, haste até o quadrado
    /// com a letra em `at`.
    pub fn datum(&self, p: &Painter, anchor: Pos2, at: Pos2, letter: &str) -> Rect {
        let side = self.size * 1.75;
        let b = Rect::from_center_size(at, egui::vec2(side, side));
        let n = (at - anchor).normalized();
        let m = egui::vec2(-n.y, n.x);
        let col = if self.sel { HL } else { self.line };
        let apex = anchor + n * self.size * 0.8;
        p.add(egui::Shape::convex_polygon(vec![anchor + m * self.size * 0.5, apex, anchor - m * self.size * 0.5], col, Stroke::NONE));
        // a haste chega no meio do lado do quadrado mais perto
        let c = b.center();
        let e = if (apex.x - c.x).abs() > (apex.y - c.y).abs() { egui::pos2(if apex.x < c.x { b.left() } else { b.right() }, c.y) } else { egui::pos2(c.x, if apex.y < c.y { b.top() } else { b.bottom() }) };
        p.line_segment([apex, e], self.stroke());
        let fill = self.fill(PmiBg::Branco);
        if let Some(f) = fill {
            p.rect_filled(b, 0., f);
        }
        p.rect_stroke(b, 0., self.stroke(), egui::StrokeKind::Middle);
        p.text(b.center(), egui::Align2::CENTER_CENTER, letter, self.font(1.1), self.text_col(fill));
        b
    }

    /// Símbolo de rugosidade (√ com barra) com o texto em cima da barra; devolve o retângulo e o
    /// vértice de baixo (onde a chamada encosta).
    pub fn roughness(&self, p: &Painter, at: Pos2, txt: &str) -> (Rect, Pos2) {
        let h = self.size * 1.3;
        let fill = self.fill(PmiBg::Nenhum);
        let tc = self.text_col(fill);
        let g = p.layout_no_wrap(txt.to_string(), self.font(0.9), tc);
        let v = at + egui::vec2(-h * 0.25, h * 0.55);
        let l = v + egui::vec2(-h * 0.45, -h * 0.55);
        let top = v + egui::vec2(h * 0.6, -h * 1.45);
        let bar_end = top + egui::vec2(g.size().x + h * 0.4, 0.);
        let r = Rect::from_two_pos(l, bar_end).union(Rect::from_min_size(top + egui::vec2(h * 0.25, -g.size().y - 1.), g.size())).expand(3.);
        if let Some(f) = fill {
            p.rect_filled(r, 1., f);
        }
        p.add(egui::Shape::line(vec![l, v, top, bar_end], self.stroke()));
        p.galley(top + egui::vec2(h * 0.25, -g.size().y - 1.), g, tc);
        if self.sel {
            p.rect_stroke(r, 2., Stroke::new(1., HL), egui::StrokeKind::Outside);
        }
        (r, v)
    }

    /// Legenda: cada linha do texto numa faixa da tabela.
    pub fn table(&self, p: &Painter, at: Pos2, txt: &str) -> Rect {
        let fill = self.fill(PmiBg::Branco);
        let tc = self.text_col(fill);
        let lines: Vec<&str> = txt.lines().collect();
        let gs: Vec<_> = lines.iter().enumerate().map(|(i, l)| p.layout_no_wrap(l.to_string(), self.font(if i == 0 { 1.15 } else { 0.92 }), tc)).collect();
        let pad = self.size * 0.5;
        let w = gs.iter().map(|g| g.size().x).fold(0., f32::max) + pad * 2.;
        let hs: Vec<f32> = gs.iter().map(|g| g.size().y + pad).collect();
        let r = Rect::from_center_size(at, egui::vec2(w, hs.iter().sum::<f32>().max(self.size)));
        if let Some(f) = fill {
            p.rect_filled(r, 0., f);
        }
        let mut y = r.top();
        for (i, g) in gs.into_iter().enumerate() {
            if i > 0 {
                p.line_segment([egui::pos2(r.left(), y), egui::pos2(r.right(), y)], self.thin());
            }
            p.galley(egui::pos2(r.left() + pad, y + pad / 2.), g, tc);
            y += hs[i];
        }
        p.rect_stroke(r, 0., Stroke::new(self.stroke().width + 0.6, self.stroke().color), egui::StrokeKind::Middle);
        r
    }

    /// Balão de item; devolve o retângulo e o raio.
    pub fn balloon(&self, p: &Painter, at: Pos2, item: &str) -> (Rect, f32) {
        let rad = self.size * 1.0 + 3.5 * item.chars().count().saturating_sub(1) as f32;
        let fill = self.fill(PmiBg::Branco);
        if let Some(f) = fill {
            p.circle_filled(at, rad, f);
        }
        p.circle_stroke(at, rad, self.stroke());
        p.text(at, egui::Align2::CENTER_CENTER, item, self.font(1.), self.text_col(fill));
        (Rect::from_center_size(at, egui::vec2(rad * 2., rad * 2.)), rad)
    }

}

/// Editor do estilo (na anotação, "padrão" herda o estilo geral).
pub fn style_ui(ui: &mut egui::Ui, st: &mut PmiStyle, inherit: bool) {
    use crate::i18n::tr;
    let def = if inherit { tr("padrão (geral)") } else { tr("padrão") };
    ui.horizontal(|ui| {
        ui.label(tr("Fonte"));
        let cur = if st.font.is_empty() { def.to_string() } else { st.font.clone() };
        egui::ComboBox::from_id_salt(("pmi_fonte", inherit)).selected_text(cur).width(150.).show_ui(ui, |ui| {
            ui.selectable_value(&mut st.font, String::new(), def);
            for f in fonts() {
                let t = egui::RichText::new(&f).family(FontFamily::Name(f.as_str().into()));
                ui.selectable_value(&mut st.font, f.clone(), t);
            }
        });
        ui.label(tr("Altura"));
        let r = ui.add(egui::DragValue::new(&mut st.size).range(0.0..=80.).speed(0.2).custom_formatter(|v, _| if v <= 0. { "padrão".into() } else { format!("{v:.1}") }));
        crate::ui_ext::wheel(ui, &r, &mut st.size);
    });
    let color_row = |ui: &mut egui::Ui, label: &str, c: &mut Option<[u8; 3]>, d: [u8; 3]| {
        ui.horizontal(|ui| {
            let mut own = c.is_some();
            if ui.checkbox(&mut own, tr(label)).changed() {
                *c = own.then_some(d);
            }
            if let Some(x) = c {
                ui.color_edit_button_srgb(x);
            }
        });
    };
    color_row(ui, "cor das linhas", &mut st.color, [20, 40, 110]);
    color_row(ui, "cor do texto", &mut st.text_color, [12, 14, 24]);
    ui.horizontal(|ui| {
        ui.label(tr("Espessura"));
        let r = ui.add(egui::DragValue::new(&mut st.line_width).range(0.0..=6.).speed(0.05).custom_formatter(|v, _| if v <= 0. { "padrão".into() } else { format!("{v:.2}") }));
        crate::ui_ext::wheel(ui, &r, &mut st.line_width);
    });
    ui.horizontal_wrapped(|ui| {
        ui.label(tr("Ponta"));
        for (a, n) in [(PmiArrow::Padrao, "padrão"), (PmiArrow::Cheia, "▶ cheia"), (PmiArrow::Aberta, "▷ aberta"), (PmiArrow::Ponto, "● ponto"), (PmiArrow::Barra, "⁄ barra"), (PmiArrow::Nenhuma, "nenhuma")] {
            ui.selectable_value(&mut st.arrow, a, tr(n));
        }
    });
    ui.horizontal_wrapped(|ui| {
        ui.label(tr("Moldura"));
        for (f, n) in [(PmiFrame::Padrao, "padrão"), (PmiFrame::Caixa, "caixa"), (PmiFrame::Arredondada, "arredondada"), (PmiFrame::Nenhuma, "nenhuma")] {
            ui.selectable_value(&mut st.frame, f, tr(n));
        }
    });
    ui.horizontal_wrapped(|ui| {
        ui.label(tr("Fundo"));
        for (b, n) in [(PmiBg::Padrao, "padrão"), (PmiBg::Nenhum, "nenhum"), (PmiBg::Branco, "branco"), (PmiBg::Escuro, "escuro")] {
            ui.selectable_value(&mut st.bg, b, tr(n));
        }
    });
}
