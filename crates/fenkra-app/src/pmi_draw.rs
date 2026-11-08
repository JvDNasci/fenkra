// SPDX-License-Identifier: LGPL-2.1-or-later
//! Desenho das anotações 3D com cara de desenho técnico: quadro de controle geométrico (ISO 1101)
//! em casas, símbolo de referência com triângulo, cotas com tolerâncias empilhadas, chamadas com
//! ponta escolhida, legenda em tabela. O estilo de cada anotação herda o estilo geral.

use eframe::egui::{self, Color32, FontFamily, FontId, Painter, Pos2, Rect, Stroke};
use fenkra_core::gdt::{FcfLinha, Modif, Parte};
use fenkra_core::pmi::*;
use std::cell::RefCell;

/// Tela de desenho da anotação: junta as formas num sistema local e no fim pinta na tela como
/// estão (de frente) ou deitadas num plano (cada vértice projetado pela câmera).
pub struct Cv<'a> {
    p: &'a Painter,
    shapes: RefCell<Vec<egui::Shape>>,
}

impl<'a> Cv<'a> {
    pub fn new(p: &'a Painter) -> Self {
        Cv { p, shapes: RefCell::new(vec![]) }
    }
    pub fn add(&self, s: egui::Shape) -> usize {
        let mut v = self.shapes.borrow_mut();
        v.push(s);
        v.len() - 1
    }
    pub fn set(&self, i: usize, s: egui::Shape) {
        if let Some(x) = self.shapes.borrow_mut().get_mut(i) {
            *x = s;
        }
    }
    pub fn extend(&self, it: impl IntoIterator<Item = egui::Shape>) {
        self.shapes.borrow_mut().extend(it);
    }
    pub fn layout_no_wrap(&self, t: String, f: FontId, c: Color32) -> std::sync::Arc<egui::Galley> {
        self.p.layout_no_wrap(t, f, c)
    }
    pub fn galley(&self, pos: Pos2, g: std::sync::Arc<egui::Galley>, c: Color32) {
        self.add(egui::Shape::galley(pos, g, c));
    }
    pub fn text(&self, pos: Pos2, a: egui::Align2, t: &str, f: FontId, c: Color32) -> Rect {
        let g = self.layout_no_wrap(t.to_string(), f, c);
        let r = a.anchor_size(pos, g.size());
        self.galley(r.min, g, c);
        r
    }
    pub fn rect_filled(&self, r: Rect, round: f32, c: Color32) {
        self.add(egui::Shape::rect_filled(r, round, c));
    }
    pub fn rect_stroke(&self, r: Rect, round: f32, s: Stroke, k: egui::StrokeKind) {
        self.add(egui::Shape::rect_stroke(r, round, s, k));
    }
    pub fn line_segment(&self, pts: [Pos2; 2], s: Stroke) {
        self.add(egui::Shape::line_segment(pts, s));
    }
    pub fn circle_filled(&self, c: Pos2, r: f32, col: Color32) {
        self.add(egui::Shape::circle_filled(c, r, col));
    }
    pub fn circle_stroke(&self, c: Pos2, r: f32, s: Stroke) {
        self.add(egui::Shape::circle_stroke(c, r, s));
    }
    pub fn len(&self) -> usize {
        self.shapes.borrow().len()
    }
    pub fn layout_job(&self, job: egui::text::LayoutJob) -> std::sync::Arc<egui::Galley> {
        self.p.layout_job(job)
    }
    /// Leva cada vértice do que foi desenhado desde `start` por `f` (girar/espelhar o rótulo).
    pub fn transform_from(&self, start: usize, f: impl Fn(Pos2) -> Pos2) {
        let ctx = self.p.ctx().clone();
        let tex = ctx.fonts(|x| x.font_image_size());
        let mut tess = egui::epaint::Tessellator::new(ctx.pixels_per_point(), egui::epaint::TessellationOptions::default(), tex, vec![]);
        for s in self.shapes.borrow_mut().iter_mut().skip(start) {
            if matches!(s, egui::Shape::Noop) {
                continue;
            }
            let mut m = egui::Mesh::default();
            tess.tessellate_shape(std::mem::replace(s, egui::Shape::Noop), &mut m);
            for v in &mut m.vertices {
                v.pos = f(v.pos);
            }
            *s = egui::Shape::mesh(m);
        }
    }
    /// Pinta como está (coordenadas já de tela).
    pub fn flush(self) {
        self.p.extend(self.shapes.into_inner());
    }
    /// Pinta deitado: `map` leva cada ponto local à tela (None = atrás da câmera). Devolve o
    /// retângulo ocupado na tela.
    pub fn flush_mapped(self, map: impl Fn(Pos2) -> Option<Pos2>) -> Option<Rect> {
        let ctx = self.p.ctx().clone();
        let ppp = ctx.pixels_per_point();
        let tex = ctx.fonts(|f| f.font_image_size());
        let mut tess = egui::epaint::Tessellator::new(ppp, egui::epaint::TessellationOptions::default(), tex, vec![]);
        let mut bb: Option<Rect> = None;
        for s in self.shapes.into_inner() {
            if matches!(s, egui::Shape::Noop) {
                continue;
            }
            let mut m = egui::Mesh::default();
            tess.tessellate_shape(s, &mut m);
            if m.vertices.is_empty() {
                continue;
            }
            let mut ok = true;
            for v in &mut m.vertices {
                match map(v.pos) {
                    Some(q) => {
                        v.pos = q;
                        bb = Some(bb.map_or(Rect::from_min_max(q, q), |r| r.union(Rect::from_min_max(q, q))));
                    }
                    None => {
                        ok = false;
                        break;
                    }
                }
            }
            if ok {
                self.p.add(egui::Shape::mesh(m));
            }
        }
        bb
    }
}

/// Cor de realce da anotação selecionada.
pub const HL: Color32 = Color32::from_rgb(255, 150, 0);

/// Estilo já resolvido (anotação → estilo geral → padrão).
#[derive(Clone)]
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
    // moldura
    pub frame_col: Color32,
    pub frame_width: f32,
    pub frame_dash: u8,
    pub frame_fixed: f32,
    // texto e posição
    pub justify: u8,
    pub anchor: u8,
    pub spacing: f32,
    /// rotação (rad, anti-horária na tela) e espelho
    pub angle: f32,
    pub mirror: bool,
    pub nudge: egui::Vec2,
    pub apply_scale: bool,
    // chamada
    pub leader_on: bool,
    pub leader_col: Color32,
    pub leader_width: f32,
    pub leader_dash: u8,
    /// placa de leitura do rótulo 3D sem fundo (cor do painel do tema) e o texto forte do tema, para a
    /// regra de contraste: nota azul-escura sobre peça escura ou no tema escuro deixava de ler
    pub leitura: Option<(Color32, Color32)>,
}

/// Fontes disponíveis para as anotações (registradas na partida).
pub fn fonts() -> Vec<String> {
    crate::app::PMI_FONTS.get().cloned().unwrap_or_default()
}

fn to_pos(v: &[[f64; 2]]) -> Vec<Pos2> {
    v.iter().map(|q| egui::pos2(q[0] as f32, q[1] as f32)).collect()
}

impl Pen {
    pub fn new(l: &PmiStyle, g: &PmiStyle, sel: bool) -> Pen {
        let num = |a: f32, b: f32, d: f32| if a > 0. { a } else if b > 0. { b } else { d };
        let font = if !l.font.is_empty() { l.font.clone() } else if !g.font.is_empty() { g.font.clone() } else { "Arial".into() };
        let all = fonts();
        // negrito: a variante "<fonte> negrito" (ou a Arial negrito)
        let font = if l.bold || g.bold {
            [format!("{font} negrito"), "Arial negrito".to_string()].into_iter().find(|f| all.contains(f)).unwrap_or(font)
        } else {
            font
        };
        let fam = if all.contains(&font) { FontFamily::Name(font.as_str().into()) } else { FontFamily::Proportional };
        let col = |a: Option<[u8; 3]>, b: Option<[u8; 3]>, d: [u8; 3]| {
            let x = a.or(b).unwrap_or(d);
            Color32::from_rgb(x[0], x[1], x[2])
        };
        let line_rgb = l.color.or(g.color).unwrap_or([20, 40, 110]);
        let line = Color32::from_rgb(line_rgb[0], line_rgb[1], line_rgb[2]);
        Pen {
            fam,
            size: num(l.size, g.size, 13.),
            line,
            text: col(l.text_color, g.text_color, [12, 14, 24]),
            width: num(l.line_width, g.line_width, 1.3),
            arrow: if l.arrow != PmiArrow::Padrao { l.arrow } else if g.arrow != PmiArrow::Padrao { g.arrow } else { PmiArrow::Cheia },
            bg: if l.bg != PmiBg::Padrao { l.bg } else { g.bg },
            frame: if l.frame != PmiFrame::Padrao { l.frame } else { g.frame },
            sel,
            frame_col: col(l.frame_color, g.frame_color, line_rgb),
            frame_width: num(l.frame_width, g.frame_width, 0.),
            frame_dash: if l.frame_dash != 0 { l.frame_dash } else { g.frame_dash },
            frame_fixed: num(l.frame_fixed, g.frame_fixed, 0.),
            justify: if l.justify != 0 { l.justify } else { g.justify },
            anchor: if l.anchor != 4 { l.anchor.min(8) } else { g.anchor.min(8) },
            spacing: num(l.line_spacing, g.line_spacing, 0.),
            angle: (if l.angle != 0. { l.angle } else { g.angle }).to_radians(),
            mirror: l.mirror || g.mirror,
            nudge: egui::vec2(l.nudge[0], l.nudge[1]),
            apply_scale: l.apply_scale || g.apply_scale,
            leader_on: !(l.hide_leader || g.hide_leader),
            leader_col: col(l.leader_color, g.leader_color, line_rgb),
            leader_width: num(l.leader_width, g.leader_width, 0.),
            leader_dash: if l.leader_dash != 0 { l.leader_dash } else { g.leader_dash },
            leitura: None,
        }
    }
    /// O mesmo estilo com o texto (e medidas de texto) vezes `ks` e as linhas vezes `kw`.
    pub fn scaled(&self, ks: f32, kw: f32) -> Pen {
        let mut p = self.clone();
        p.size *= ks;
        p.frame_fixed *= ks;
        p.nudge *= ks;
        p.width = (p.width * kw).max(0.6);
        p.frame_width *= kw;
        p.leader_width *= kw;
        p
    }
    pub fn stroke(&self) -> Stroke {
        Stroke::new(if self.sel { self.width + 1. } else { self.width }, if self.sel { HL } else { self.line })
    }
    pub fn thin(&self) -> Stroke {
        Stroke::new((self.width * 0.75).max(0.8), if self.sel { HL } else { self.line })
    }
    fn frame_stroke(&self) -> Stroke {
        let w = if self.frame_width > 0. { self.frame_width } else { self.width };
        Stroke::new(if self.sel { w + 1. } else { w }, if self.sel { HL } else { self.frame_col })
    }
    fn leader_stroke(&self) -> Stroke {
        let w = if self.leader_width > 0. { self.leader_width } else { self.width };
        Stroke::new(if self.sel { w + 1. } else { w }, if self.sel { HL } else { self.leader_col })
    }
    pub fn font(&self, k: f32) -> FontId {
        FontId::new(self.size * k, self.fam.clone())
    }
    fn fill(&self, kind_default: PmiBg) -> Option<Color32> {
        match if self.bg == PmiBg::Padrao { kind_default } else { self.bg } {
            PmiBg::Branco => Some(Color32::from_rgba_unmultiplied(255, 255, 255, 235)),
            PmiBg::Escuro => Some(Color32::from_rgba_unmultiplied(40, 30, 22, 225)),
            // opaco: esconde a geometria atrás
            PmiBg::Recorte => Some(Color32::from_rgb(250, 250, 248)),
            _ => None,
        }
    }
    fn text_col(&self, fill: Option<Color32>) -> Color32 {
        match fill {
            Some(f) if f.r() < 100 => Color32::WHITE,
            _ => self.text,
        }
    }
    /// Fundo e cor do texto do bloco. Sem fundo escolhido (padrão sem placa) e com a placa de leitura
    /// ligada, o rótulo ganha a placa translúcida do tema e o texto segue a regra de contraste.
    fn fundo_e_texto(&self, kind_default: PmiBg) -> (Option<Color32>, Color32) {
        let fill = self.fill(kind_default);
        match (fill, self.leitura) {
            (None, Some((placa, forte))) if self.bg == PmiBg::Padrao => {
                let f = |c: Color32| [c.r() as f32 / 255., c.g() as f32 / 255., c.b() as f32 / 255.];
                let t = crate::contraste::texto_sobre(f(self.text), f(placa), f(forte));
                let texto = if t == f(self.text) { self.text } else { forte };
                (Some(placa.gamma_multiply(crate::contraste::OPACIDADE_PLACA)), texto)
            }
            _ => (fill, self.text_col(fill)),
        }
    }
    fn frame_of(&self, kind_default: PmiFrame) -> PmiFrame {
        if self.frame == PmiFrame::Padrao { kind_default } else { self.frame }
    }

    /// Texto em várias linhas com a justificação e o espaçamento do estilo.
    fn layout(&self, p: &Cv, txt: &str, k: f32, c: Color32) -> std::sync::Arc<egui::Galley> {
        let mut job = egui::text::LayoutJob::simple(txt.to_string(), self.font(k), c, f32::INFINITY);
        job.halign = match self.justify {
            1 => egui::Align::Center,
            2 => egui::Align::RIGHT,
            _ => egui::Align::LEFT,
        };
        if self.spacing > 0. {
            for s in &mut job.sections {
                s.format.line_height = Some(self.size * k * 1.15 * self.spacing);
            }
        }
        p.layout_job(job)
    }
    /// Pinta o texto com o canto de cima à esquerda (visual) em `tl`.
    fn put(&self, p: &Cv, tl: Pos2, g: std::sync::Arc<egui::Galley>, c: Color32) {
        p.galley(tl - g.rect.min.to_vec2(), g, c);
    }

    /// Retângulo de tamanho `sz` com o ponto de ancoragem do estilo em `at` (mais o deslocamento).
    fn place(&self, at: Pos2, mut sz: egui::Vec2) -> Rect {
        sz.x = sz.x.max(self.frame_fixed);
        let (col, row) = ((self.anchor % 3) as f32, (self.anchor / 3) as f32);
        Rect::from_min_size(at + self.nudge - egui::vec2(sz.x * col / 2., sz.y * row / 2.), sz)
    }

    pub fn turned(&self) -> bool {
        self.angle.abs() > 1e-4 || self.mirror
    }
    fn rot(d: egui::Vec2, a: f32) -> egui::Vec2 {
        let (s, c) = a.sin_cos();
        egui::vec2(d.x * c - d.y * s, d.x * s + d.y * c)
    }
    /// Ponto local (não girado) → tela: espelha e gira em volta de `c`.
    fn xf(&self, q: Pos2, c: Pos2) -> Pos2 {
        let mut d = q - c;
        if self.mirror {
            d.x = -d.x;
        }
        c + Self::rot(d, -self.angle)
    }
    pub fn xf_inv(&self, q: Pos2, c: Pos2) -> Pos2 {
        let mut d = Self::rot(q - c, self.angle);
        if self.mirror {
            d.x = -d.x;
        }
        c + d
    }
    /// Gira/espelha o que foi desenhado desde `start` em volta de `c`.
    fn turn(&self, p: &Cv, start: usize, c: Pos2) {
        if self.turned() {
            p.transform_from(start, |q| self.xf(q, c));
        }
    }
    /// Retângulo que envolve `r` depois de girado (para clicar no rótulo).
    pub fn outer(&self, r: Rect) -> Rect {
        if !self.turned() {
            return r;
        }
        let c = r.center();
        let pts = [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom()].map(|q| self.xf(q, c));
        Rect::from_points(&pts)
    }

    /// Linha (aberta ou fechada) com o tipo de linha escolhido.
    fn outline_shapes(&self, pts: &[Pos2], closed: bool, st: Stroke, dash: u8) -> Vec<egui::Shape> {
        let pat = dash_pattern(dash);
        if pat.is_empty() || pts.len() < 2 {
            return vec![if closed { egui::Shape::closed_line(pts.to_vec(), st) } else { egui::Shape::line(pts.to_vec(), st) }];
        }
        let k = st.width.max(1.) as f64;
        let pat: Vec<f64> = pat.iter().map(|x| x * k).collect();
        let v: Vec<[f64; 2]> = pts.iter().map(|q| [q.x as f64, q.y as f64]).collect();
        dashed(&v, closed, &pat).iter().map(|s| egui::Shape::line(to_pos(s), st)).collect()
    }

    /// Moldura do tipo `frame` em volta de `r`, com o fundo; devolve o retângulo de fora.
    fn draw_frame(&self, p: &Cv, r: Rect, frame: PmiFrame, fill: Option<Color32>) -> Rect {
        let paths = frame.paths(r.left() as f64, r.top() as f64, r.right() as f64, r.bottom() as f64);
        let mut bb = r;
        if let Some(f) = fill {
            match paths.iter().find(|x| x.1) {
                Some((pts, _)) => {
                    p.add(egui::Shape::convex_polygon(to_pos(pts), f, Stroke::NONE));
                }
                None => p.rect_filled(r, 1., f),
            }
        }
        for (pts, closed) in &paths {
            let v = to_pos(pts);
            bb = bb.union(Rect::from_points(&v));
            p.extend(self.outline_shapes(&v, *closed, self.frame_stroke(), self.frame_dash));
        }
        if paths.is_empty() && self.sel {
            p.rect_stroke(r, 2., Stroke::new(1., HL), egui::StrokeKind::Outside);
        }
        bb
    }

    /// Ponta da chamada em `tip`, vindo de `from`.
    pub fn arrow(&self, p: &Cv, tip: Pos2, from: Pos2) {
        p.extend(self.arrow_shapes(tip, from, self.stroke()));
    }

    fn arrow_shapes(&self, tip: Pos2, from: Pos2, st: Stroke) -> Vec<egui::Shape> {
        self.arrow_shapes_with(self.arrow, tip, from, st)
    }

    /// Como `arrow_shapes`, com a ponta escolhida (cada chamada da nota pode ter a sua, B368).
    fn arrow_shapes_with(&self, arrow: PmiArrow, tip: Pos2, from: Pos2, st: Stroke) -> Vec<egui::Shape> {
        let d = (tip - from).normalized();
        if !d.x.is_finite() || (tip - from).length() < 2. {
            return vec![];
        }
        let n = egui::vec2(-d.y, d.x);
        let (s, w) = (self.size * 0.8, self.size * 0.27);
        let col = st.color;
        match arrow {
            PmiArrow::Cheia | PmiArrow::Padrao => vec![egui::Shape::convex_polygon(vec![tip, tip - d * s + n * w, tip - d * s - n * w], col, Stroke::NONE)],
            PmiArrow::Aberta => vec![egui::Shape::line(vec![tip - d * s + n * w, tip, tip - d * s - n * w], st)],
            PmiArrow::Ponto => vec![egui::Shape::circle_filled(tip, self.size * 0.22, col)],
            PmiArrow::Barra => {
                let k = (d + n).normalized() * s * 0.45;
                vec![egui::Shape::line_segment([tip - k, tip + k], Stroke::new(st.width + 0.6, col))]
            }
            PmiArrow::Nenhuma => vec![],
        }
    }

    /// Lugar reservado na pintura para a chamada ficar por baixo da caixa (desenhada depois).
    pub fn reserve(p: &Cv) -> usize {
        p.add(egui::Shape::Noop)
    }

    /// O mesmo estilo em unidades do plano (0,1 mm): texto e linhas com medidas reais.
    pub fn on_plane(&self) -> Pen {
        let mut p = self.scaled(0.35 * 10., 1.8);
        p.apply_scale = false;
        p
    }

    /// Chamada da caixa `r` até `anchor`: sai do lado da caixa voltado para o ponto, com um
    /// patamar curto, e fica por baixo da caixa (no lugar reservado). Nunca cruza a caixa.
    /// Com o rótulo girado, o lado é escolhido no sistema da caixa.
    pub fn leader(&self, p: &Cv, slot: usize, r: Rect, anchor: Pos2) {
        self.leader_route(p, slot, r, &[anchor]);
    }

    /// Chamada da caixa `r` até a última posição de `route`, passando pelos vértices intermediários
    /// (B23). Sai do lado da caixa voltado para o primeiro ponto do caminho e a seta fica na ponta.
    pub fn leader_route(&self, p: &Cv, slot: usize, r: Rect, route: &[Pos2]) {
        self.leader_route_ex(p, slot, r, route, 0, PmiArrow::Padrao);
    }

    /// Ponto da caixa `r` de onde sai a chamada, o joelho do patamar e o lado usado. `side` 0 é
    /// automático (o lado voltado para o primeiro ponto do caminho); 1 esquerda, 2 direita, 3 topo,
    /// 4 base. Devolve já no sistema da tela (caixa girada ou espelhada incluída).
    pub fn leader_start(&self, r: Rect, toward: Pos2, side: u8) -> (Pos2, Pos2, u8) {
        let c = r.center();
        let a = if self.turned() { self.xf_inv(toward, c) } else { toward };
        let d = a - c;
        let land = self.size * 0.9;
        let side = if side == 0 {
            if d.x.abs() * r.height() >= d.y.abs() * r.width() {
                if d.x < 0. { 1 } else { 2 }
            } else if d.y < 0. {
                3
            } else {
                4
            }
        } else {
            side
        };
        let (att, out) = match side {
            1 => (egui::pos2(r.left(), c.y), egui::vec2(-land, 0.)),
            2 => (egui::pos2(r.right(), c.y), egui::vec2(land, 0.)),
            3 => (egui::pos2(c.x, r.top()), egui::vec2(0., -land * 0.7)),
            _ => (egui::pos2(c.x, r.bottom()), egui::vec2(0., land * 0.7)),
        };
        if self.turned() { (self.xf(att, c), self.xf(att + out, c), side) } else { (att, att + out, side) }
    }

    /// Mesma chamada com a ancoragem (`side`) e a ponta próprias da seta (B368).
    pub fn leader_route_ex(&self, p: &Cv, slot: usize, r: Rect, route: &[Pos2], side: u8, arrow: PmiArrow) {
        if !self.leader_on {
            return;
        }
        let (Some(&anchor), Some(&tip)) = (route.first(), route.last()) else { return };
        let c = r.center();
        let a = if self.turned() { self.xf_inv(anchor, c) } else { anchor };
        if route.len() == 1 && side == 0 && r.expand(2.).contains(a) {
            return;
        }
        let (att, knee, _) = self.leader_start(r, anchor, side);
        let st = self.leader_stroke();
        let mut pts = vec![att, knee];
        pts.extend_from_slice(route);
        let mut v = self.outline_shapes(&pts, false, st, self.leader_dash);
        let arrow = if arrow == PmiArrow::Padrao { self.arrow } else { arrow };
        v.extend(self.arrow_shapes_with(arrow, tip, pts[pts.len() - 2], st));
        p.set(slot, egui::Shape::Vec(v));
    }

    /// Bloco de texto (várias linhas) no ponto `at` (conforme o ponto de ancoragem), com
    /// moldura e fundo conforme o tipo.
    pub fn text_block(&self, p: &Cv, at: Pos2, txt: &str, frame_default: PmiFrame, bg_default: PmiBg) -> Rect {
        let start = p.len();
        let (fill, tc) = self.fundo_e_texto(bg_default);
        let g = self.layout(p, txt, 1., tc);
        let pad = egui::vec2(self.size * 0.45, self.size * 0.25);
        let r = self.place(at, g.size() + pad * 2.);
        let bb = self.draw_frame(p, r, self.frame_of(frame_default), fill);
        let inner = r.shrink2(pad);
        let x = match self.justify {
            1 => inner.center().x - g.size().x / 2.,
            2 => inner.right() - g.size().x,
            _ => inner.left(),
        };
        self.put(p, egui::pos2(x, inner.top()), g, tc);
        self.turn(p, start, bb.center());
        bb
    }

    /// Texto de cota: valor sem moldura (salvo pedido) e tolerâncias diferentes empilhadas à direita.
    pub fn dim_text(&self, p: &Cv, at: Pos2, main: &str, stack: Option<&(String, String)>) -> Rect {
        let Some((up, lo)) = stack else { return self.text_block(p, at, main, PmiFrame::Nenhuma, PmiBg::Nenhum) };
        let start = p.len();
        let fill = self.fill(PmiBg::Nenhum);
        let tc = self.text_col(fill);
        let g = p.layout_no_wrap(main.to_string(), self.font(1.), tc);
        let gu = p.layout_no_wrap(up.clone(), self.font(0.62), tc);
        let gl = p.layout_no_wrap(lo.clone(), self.font(0.62), tc);
        let w = g.size().x + 3. + gu.size().x.max(gl.size().x);
        let h = g.size().y.max(gu.size().y + gl.size().y);
        let r = self.place(at, egui::vec2(w + 8., h + 4.));
        let bb = self.draw_frame(p, r, self.frame_of(PmiFrame::Nenhuma), fill);
        let x0 = r.left() + 4.;
        p.galley(egui::pos2(x0, r.center().y - g.size().y / 2.), g.clone(), tc);
        let x1 = x0 + g.size().x + 3.;
        p.galley(egui::pos2(x1, r.top() + 2.), gu.clone(), tc);
        p.galley(egui::pos2(x1, r.top() + 2. + gu.size().y), gl, tc);
        self.turn(p, start, bb.center());
        bb
    }

    /// Quadro de controle geométrico: a primeira casa (símbolo) ocupa todas as linhas; cada linha
    /// tem as casas de tolerância e de referências. `at` é o ponto de ancoragem do quadro.
    pub fn fcf(&self, p: &Cv, at: Pos2, ch: fenkra_core::pmi::GdtChar, linhas: &[FcfLinha], chamada: &str) -> Rect {
        let start = p.len();
        let h = self.size * 1.7;
        let pad = self.size * 0.5;
        let gap = self.size * 0.25;
        let fill = self.fill(PmiBg::Branco);
        let tc = self.text_col(fill);
        let st = self.frame_stroke();
        let w0 = h;
        // cada parte da célula: texto (galley) ou modificador (traço vetorial ou sigla)
        enum Pt {
            T(std::sync::Arc<egui::Galley>),
            M(Modif, Option<std::sync::Arc<egui::Galley>>),
        }
        let mod_w = self.size * 1.15;
        let pw = |q: &Pt| match q {
            Pt::T(g) => g.size().x,
            Pt::M(_, Some(g)) => g.size().x,
            Pt::M(_, None) => mod_w,
        };
        let cel_w = |c: &Vec<Pt>| (c.iter().map(pw).sum::<f32>() + gap * c.len().saturating_sub(1) as f32 + pad * 2.).max(h * 0.9);
        let cells: Vec<Vec<Vec<Pt>>> = linhas
            .iter()
            .map(|l| {
                l.celulas
                    .iter()
                    .map(|c| {
                        c.iter()
                            .map(|q| match q {
                                Parte::Texto(t) => Pt::T(p.layout_no_wrap(t.clone(), self.font(1.), tc)),
                                Parte::Mod(m) if m.circulado().is_some() => Pt::M(*m, None),
                                Parte::Mod(m) => Pt::M(*m, Some(p.layout_no_wrap(m.texto(), self.font(0.9), tc))),
                            })
                            .collect()
                    })
                    .collect()
            })
            .collect();
        let row_w: Vec<f32> = cells.iter().map(|r| r.iter().map(cel_w).sum()).collect();
        let total_w = w0 + row_w.iter().cloned().fold(0., f32::max);
        let frame_h = h * linhas.len().max(1) as f32;
        let cg = if chamada.is_empty() { None } else { Some(p.layout_no_wrap(chamada.to_string(), self.font(0.9), tc)) };
        let up = cg.as_ref().map(|g| g.size().y + self.size * 0.2).unwrap_or(0.);
        let r = self.place(at, egui::vec2(total_w, frame_h + up));
        let top0 = r.top() + up;
        if let Some(g) = cg {
            p.galley(egui::pos2(r.left(), r.top()), g, tc);
        }
        // símbolo da característica: um por grupo (composto: a casa vale para todas as linhas)
        let mut i = 0;
        while i < linhas.len() {
            let mut j = i + 1;
            while j < linhas.len() && linhas[j].sym.is_none() {
                j += 1;
            }
            let sym = linhas[i].sym.unwrap_or(ch);
            let box_ = Rect::from_min_size(egui::pos2(r.left(), top0 + h * i as f32), egui::vec2(w0, h * (j - i) as f32));
            if let Some(f) = fill {
                p.rect_filled(box_, 0., f);
            }
            p.extend(self.outline_shapes(&rect_pts(box_), true, st, self.frame_dash));
            self.simbolo(p, &sym.tracado(), box_.center(), h * 0.62, st.color);
            i = j;
        }
        for (i, row) in cells.iter().enumerate() {
            let top = top0 + h * i as f32;
            let row_rect = Rect::from_min_size(egui::pos2(r.left() + w0, top), egui::vec2(row_w[i], h));
            if let Some(f) = fill {
                p.rect_filled(row_rect, 0., f);
            }
            p.extend(self.outline_shapes(&rect_pts(row_rect), true, st, self.frame_dash));
            let mut x = r.left() + w0;
            for (k, c) in row.iter().enumerate() {
                let cw = cel_w(c);
                if k > 0 {
                    p.extend(self.outline_shapes(&[egui::pos2(x, top), egui::pos2(x, top + h)], false, st, self.frame_dash));
                }
                let uso = c.iter().map(pw).sum::<f32>() + gap * c.len().saturating_sub(1) as f32;
                let mut qx = x + (cw - uso) / 2.;
                for q in c {
                    match q {
                        Pt::T(g) => {
                            p.galley(egui::pos2(qx, top + (h - g.size().y) / 2.), g.clone(), tc);
                        }
                        Pt::M(m, Some(g)) => {
                            let _ = m;
                            p.galley(egui::pos2(qx, top + (h - g.size().y) / 2.), g.clone(), tc);
                        }
                        Pt::M(m, None) => {
                            self.simbolo(p, &m.tracado(), egui::pos2(qx + mod_w / 2., top + h / 2.), mod_w, st.color);
                        }
                    }
                    qx += pw(q) + gap;
                }
                x += cw;
            }
        }
        let mut out = r;
        out.max.x = r.left() + total_w;
        self.turn(p, start, out.center());
        out
    }

    /// Desenha um símbolo vetorial (quadro unitário, y para cima) centrado em `c`, de lado `lado`.
    fn simbolo(&self, p: &Cv, tracos: &[fenkra_core::gdt::Traco], c: Pos2, lado: f32, col: Color32) {
        let st = Stroke::new((self.size * 0.09).max(1.), col);
        for t in tracos {
            let pts: Vec<Pos2> = t.pts.iter().map(|q| egui::pos2(c.x + q[0] as f32 * lado, c.y - q[1] as f32 * lado)).collect();
            if t.cheio {
                p.add(egui::Shape::convex_polygon(pts, col, Stroke::NONE));
            } else if t.fechado {
                p.add(egui::Shape::closed_line(pts, st));
            } else {
                p.add(egui::Shape::line(pts, st));
            }
        }
    }

    /// Símbolo de referência: triângulo cheio apoiado na peça em `anchor`, haste até o quadrado
    /// com a letra em `at`.
    pub fn datum(&self, p: &Cv, anchor: Pos2, at: Pos2, letter: &str) -> Rect {
        let side = self.size * 1.75;
        let b = Rect::from_center_size(at + self.nudge, egui::vec2(side, side));
        let n = (b.center() - anchor).normalized();
        let m = egui::vec2(-n.y, n.x);
        let col = if self.sel { HL } else { self.line };
        let apex = anchor + n * self.size * 0.8;
        p.add(egui::Shape::convex_polygon(vec![anchor + m * self.size * 0.5, apex, anchor - m * self.size * 0.5], col, Stroke::NONE));
        // a haste chega no meio do lado do quadrado mais perto
        let c = b.center();
        let e = if (apex.x - c.x).abs() > (apex.y - c.y).abs() { egui::pos2(if apex.x < c.x { b.left() } else { b.right() }, c.y) } else { egui::pos2(c.x, if apex.y < c.y { b.top() } else { b.bottom() }) };
        p.extend(self.outline_shapes(&[apex, e], false, self.leader_stroke(), self.leader_dash));
        let fill = self.fill(PmiBg::Branco);
        if let Some(f) = fill {
            p.rect_filled(b, 0., f);
        }
        p.extend(self.outline_shapes(&rect_pts(b), true, self.frame_stroke(), self.frame_dash));
        p.text(b.center(), egui::Align2::CENTER_CENTER, letter, self.font(1.1), self.text_col(fill));
        b
    }

    /// Símbolo de rugosidade (√ com barra) com o texto em cima da barra; devolve o retângulo e o
    /// vértice de baixo (onde a chamada encosta).
    pub fn roughness(&self, p: &Cv, at: Pos2, txt: &str) -> (Rect, Pos2) {
        let at = at + self.nudge;
        // o V acompanha o texto na proporção da norma (cerca de 2,5 vezes a altura do texto)
        let h = self.size * 1.8;
        let fill = self.fill(PmiBg::Nenhum);
        let tc = self.text_col(fill);
        let g = self.layout(p, txt, 1.0, tc);
        let v = at + egui::vec2(-h * 0.25, h * 0.55);
        let l = v + egui::vec2(-h * 0.45, -h * 0.55);
        let top = v + egui::vec2(h * 0.6, -h * 1.45);
        let bar_end = top + egui::vec2(g.size().x + h * 0.4, 0.);
        let tl = top + egui::vec2(h * 0.25, -g.size().y - 1.);
        let r = Rect::from_two_pos(l, bar_end).union(Rect::from_min_size(tl, g.size())).expand(3.);
        if let Some(f) = fill {
            p.rect_filled(r, 1., f);
        }
        p.add(egui::Shape::line(vec![l, v, top, bar_end], self.stroke()));
        self.put(p, tl, g, tc);
        if self.sel {
            p.rect_stroke(r, 2., Stroke::new(1., HL), egui::StrokeKind::Outside);
        }
        (r, v)
    }

    /// Legenda: cada linha do texto numa faixa da tabela.
    pub fn table(&self, p: &Cv, at: Pos2, txt: &str) -> Rect {
        let start = p.len();
        let fill = self.fill(PmiBg::Branco);
        let tc = self.text_col(fill);
        let lines: Vec<&str> = txt.lines().collect();
        let gs: Vec<_> = lines.iter().enumerate().map(|(i, l)| p.layout_no_wrap(l.to_string(), self.font(if i == 0 { 1.15 } else { 0.92 }), tc)).collect();
        let pad = self.size * 0.5;
        let w = gs.iter().map(|g| g.size().x).fold(0., f32::max) + pad * 2.;
        let hs: Vec<f32> = gs.iter().map(|g| g.size().y + pad).collect();
        let r = self.place(at, egui::vec2(w, hs.iter().sum::<f32>().max(self.size)));
        if let Some(f) = fill {
            p.rect_filled(r, 0., f);
        }
        let mut y = r.top();
        for (i, g) in gs.into_iter().enumerate() {
            if i > 0 {
                p.line_segment([egui::pos2(r.left(), y), egui::pos2(r.right(), y)], self.thin());
            }
            let x = match self.justify {
                1 => r.center().x - g.size().x / 2.,
                2 => r.right() - pad - g.size().x,
                _ => r.left() + pad,
            };
            p.galley(egui::pos2(x, y + pad / 2.), g, tc);
            y += hs[i];
        }
        let fs = self.frame_stroke();
        p.extend(self.outline_shapes(&rect_pts(r), true, Stroke::new(fs.width + 0.6, fs.color), self.frame_dash));
        self.turn(p, start, r.center());
        r
    }

    /// Caminhos vetoriais do núcleo (unidades de `h`, y para cima) → tela, com a origem em `o`.
    fn caminhos(&self, p: &Cv, paths: &[(Vec<[f64; 2]>, bool)], o: Pos2, k: f32, st: Stroke) -> Rect {
        let mut r = Rect::from_center_size(o, egui::vec2(1., 1.));
        for (pts, fecha) in paths {
            let mut v: Vec<Pos2> = pts.iter().map(|q| o + egui::vec2(q[0] as f32 * k, -(q[1] as f32) * k)).collect();
            v.iter().for_each(|q| r.extend_with(*q));
            if *fecha && v.len() > 2 {
                v.push(v[0]);
            }
            p.add(egui::Shape::line(v, st));
        }
        r
    }

    /// Alvo de referência (B189): quadro redondo dividido (dimensão em cima, letra e número embaixo)
    /// em `at`, marca da forma sobre o ponto `anchor` e, para o alvo móvel, a seta do curso até
    /// `curso`. Devolve o retângulo do quadro.
    pub fn datum_target(&self, p: &Cv, at: Pos2, anchor: Pos2, curso: Option<Pos2>, alvo: &fenkra_core::pmi_alvos::AlvoRef) -> Rect {
        use fenkra_core::pmi_alvos::FormaAlvo;
        let rad = self.size * 1.35;
        let c = at + self.nudge;
        let col = if self.sel { HL } else { self.line };
        // chamada do quadro até o ponto
        let d = (anchor - c).normalized();
        if d.x.is_finite() && (anchor - c).length() > rad {
            p.extend(self.outline_shapes(&[c + d * rad, anchor], false, self.leader_stroke(), self.leader_dash));
        }
        // marca da forma sobre a superfície
        let k = self.size * 0.55;
        let h = 2.0;
        let dim = match alvo.forma {
            FormaAlvo::Circulo | FormaAlvo::CirculoCurva => [3.2, 0.],
            FormaAlvo::Linha => [4.5, 0.],
            _ => [4.5, 3.],
        };
        let st = Stroke::new(if self.sel { self.width + 1. } else { self.width.max(1.2) }, col);
        let ext = self.caminhos(p, &fenkra_core::pmi_alvos::simbolo(alvo.forma, h, dim), anchor, k, st);
        if alvo.restrita {
            // área restrita: contorno tracejado em volta da marca
            let pts = [ext.left_top(), ext.right_top(), ext.right_bottom(), ext.left_bottom(), ext.left_top()].map(|q| q + (q - ext.center()).normalized() * 3.);
            p.extend(self.outline_shapes(&pts, true, self.leader_stroke(), 2));
        }
        if let Some(fim) = curso {
            p.line_segment([anchor, fim], st);
            self.arrow_into(p, fim, anchor, st);
            self.arrow_into(p, anchor, fim, st);
        }
        // quadro dividido
        let fill = self.fill(PmiBg::Branco);
        if let Some(f) = fill {
            p.circle_filled(c, rad, f);
        }
        p.circle_stroke(c, rad, self.frame_stroke());
        p.line_segment([c - egui::vec2(rad, 0.), c + egui::vec2(rad, 0.)], self.frame_stroke());
        let tc = self.text_col(fill);
        let (cima, baixo) = (alvo.texto_dimensoes(), alvo.identificador());
        let mut texto = |t: &str, y: f32| {
            let g = p.layout_no_wrap(t.to_string(), self.font(0.8), tc);
            let tl = Pos2::new(c.x - g.size().x / 2., c.y + y - g.size().y / 2.);
            p.galley(tl, g, tc);
        };
        if !cima.is_empty() {
            texto(&cima, -rad * 0.5);
        }
        texto(&baixo, rad * 0.5);
        Rect::from_center_size(c, egui::vec2(rad * 2., rad * 2.))
    }

    fn arrow_into(&self, p: &Cv, tip: Pos2, from: Pos2, st: Stroke) {
        p.extend(self.arrow_shapes(tip, from, st));
    }

    /// Rugosidade estruturada (B190): símbolo vetorial do núcleo (remoção, contorno, textura de área)
    /// com os campos nas posições da norma. Devolve o retângulo e o vértice de baixo (onde a chamada
    /// encosta).
    pub fn roughness_ext(&self, p: &Cv, at: Pos2, l: &fenkra_core::pmi_acabamento::RugLinhas, x: &fenkra_core::pmi_acabamento::RugExtra) -> (Rect, Pos2) {
        let v = at + self.nudge;
        // unidade do símbolo: o V tem 2 unidades de altura, então 1,6 do tamanho dá cerca de 3,2 vezes o texto
        let k = self.size * 1.6;
        let col = if self.sel { HL } else { self.line };
        let st = Stroke::new(if self.sel { self.width + 1. } else { self.width }, col);
        let fill = self.fill(PmiBg::Nenhum);
        let tc = self.text_col(fill);
        let mut r = self.caminhos(p, &fenkra_core::pmi_acabamento::simbolo(x, 1.0), v, k, st);
        // barra: x de 1,0 a 4,0 (em h); y = 2,0
        let barra_esq = v + egui::vec2(1.0 * k, -2.0 * k);
        let barra_dir = v + egui::vec2(4.0 * k, -2.0 * k);
        let mut texto = |t: &str, tl: Pos2| {
            let g = p.layout_no_wrap(t.to_string(), self.font(1.0), tc);
            let b = Rect::from_min_size(tl, g.size());
            r = r.union(b);
            p.galley(tl, g, tc);
        };
        // valores em cima da barra, do último para o primeiro, subindo
        // com o segundo traço da ISO 21920 (0,25 unidade acima da barra) o texto sobe mais um pouco
        let alt = self.size * 1.3;
        let folga = if x.norma == fenkra_core::pmi_acabamento::NormaRug::Iso21920 { 0.3 * k } else { 0. };
        for (i, t) in l.valores.iter().rev().enumerate() {
            texto(t, barra_esq + egui::vec2(self.size * 0.2, -alt * (i as f32 + 1.) - 1. - folga));
        }
        if !l.metodo.is_empty() {
            texto(&l.metodo, barra_esq + egui::vec2(self.size * 0.2, 2.));
        }
        if !l.lay.is_empty() {
            texto(&l.lay, barra_dir + egui::vec2(self.size * 0.3, 2.));
        }
        if !l.sobremetal.is_empty() {
            texto(&l.sobremetal, v + egui::vec2(-k * 1.6, -k * 0.2));
        }
        if !l.amostragem.is_empty() {
            texto(&l.amostragem, barra_dir + egui::vec2(self.size * 0.3, -alt - 1.));
        }
        if self.sel {
            p.rect_stroke(r.expand(2.), 2., Stroke::new(1., HL), egui::StrokeKind::Outside);
        }
        (r, v)
    }

    /// Balão de item; devolve o retângulo e o raio.
    pub fn balloon(&self, p: &Cv, at: Pos2, item: &str) -> (Rect, f32) {
        let rad = self.size * 1.0 + 3.5 * item.chars().count().saturating_sub(1) as f32;
        let fill = self.fill(PmiBg::Branco);
        if let Some(f) = fill {
            p.circle_filled(at, rad, f);
        }
        if self.frame_dash == 0 {
            p.circle_stroke(at, rad, self.frame_stroke());
        } else {
            let pts: Vec<Pos2> = (0..48).map(|i| at + egui::Vec2::angled(i as f32 / 48. * std::f32::consts::TAU) * rad).collect();
            p.extend(self.outline_shapes(&pts, true, self.frame_stroke(), self.frame_dash));
        }
        p.text(at, egui::Align2::CENTER_CENTER, item, self.font(1.), self.text_col(fill));
        (Rect::from_center_size(at, egui::vec2(rad * 2., rad * 2.)), rad)
    }
}

fn rect_pts(r: Rect) -> [Pos2; 4] {
    [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom()]
}

/// Editor do estilo (na anotação, "padrão" herda o estilo geral). Abas como nas propriedades
/// de texto dos CADs: Texto, Moldura, Posição, Fundo, Chamada, Linhas.
pub fn style_ui(ui: &mut egui::Ui, st: &mut PmiStyle, inherit: bool) {
    use crate::i18n::tr;
    let def = if inherit { tr("padrão (geral)") } else { tr("padrão") };
    let fmt_def = |dec: usize| move |v: f64, _: std::ops::RangeInclusive<usize>| if v <= 0. { tr("padrão").to_string() } else { format!("{v:.dec$}") };
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
    let dash_combo = |ui: &mut egui::Ui, salt: &str, v: &mut u8| {
        egui::ComboBox::from_id_salt((salt, inherit)).selected_text(tr(dash_label(*v))).width(110.).show_ui(ui, |ui| {
            for k in 0..=4u8 {
                ui.selectable_value(v, k, tr(dash_label(k)));
            }
        });
    };
    let head = |t: &str, open: bool| egui::CollapsingHeader::new(tr(t)).id_salt((t.to_string(), inherit)).default_open(open);
    head("Texto", true).show(ui, |ui| {
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
            ui.toggle_value(&mut st.bold, egui::RichText::new("B").strong()).on_hover_text(tr("negrito"));
        });
        ui.horizontal(|ui| {
            ui.label(tr("Altura"));
            let r = ui.add(crate::ui_ext::dv(&mut st.size).range(0.0..=80.).speed(0.2).custom_formatter(fmt_def(1)));
            crate::ui_ext::wheel(ui, &r, &mut st.size);
            ui.label(tr("Espaçamento"));
            let r = ui.add(crate::ui_ext::dv(&mut st.line_spacing).range(0.0..=4.).speed(0.02).custom_formatter(|v, _| if v <= 0. { tr("normal").to_string() } else { format!("{v:.2}×") }));
            crate::ui_ext::wheel(ui, &r, &mut st.line_spacing);
        });
        ui.horizontal(|ui| {
            ui.label(tr("Justificação"));
            for (k, n) in [(0u8, "⬅ esquerda"), (1, "↔ centro"), (2, "➡ direita")] {
                ui.selectable_value(&mut st.justify, k, tr(n));
            }
        });
        color_row(ui, "cor do texto", &mut st.text_color, [12, 14, 24]);
    });
    head("Moldura", false).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(tr("Forma"));
            egui::ComboBox::from_id_salt(("pmi_moldura", inherit)).selected_text(tr(st.frame.label())).width(150.).show_ui(ui, |ui| {
                for f in PmiFrame::ALL {
                    ui.selectable_value(&mut st.frame, f, tr(f.label()));
                }
            });
        });
        ui.horizontal(|ui| {
            ui.label(tr("Espessura"));
            let r = ui.add(crate::ui_ext::dv(&mut st.frame_width).range(0.0..=6.).speed(0.05).custom_formatter(fmt_def(2)));
            crate::ui_ext::wheel(ui, &r, &mut st.frame_width);
            ui.label(tr("Tipo de linha"));
            dash_combo(ui, "pmi_moldura_traco", &mut st.frame_dash);
        });
        color_row(ui, "cor da moldura", &mut st.frame_color, [20, 40, 110]);
        ui.horizontal(|ui| {
            let mut fixed = st.frame_fixed > 0.;
            if ui.checkbox(&mut fixed, tr("tamanho fixo")).changed() {
                st.frame_fixed = if fixed { 80. } else { 0. };
            }
            if fixed {
                let r = ui.add(crate::ui_ext::dv(&mut st.frame_fixed).range(1.0..=2000.).speed(0.5).suffix(" pt"));
                crate::ui_ext::wheel(ui, &r, &mut st.frame_fixed);
            }
        });
    });
    head("Posição", false).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(tr("Ponto de ancoragem"));
            egui::Grid::new(("pmi_ancora_grade", inherit)).spacing(egui::vec2(2., 2.)).show(ui, |ui| {
                for (k, s) in ["◤", "▲", "◥", "◀", "●", "▶", "◣", "▼", "◢"].iter().enumerate() {
                    ui.selectable_value(&mut st.anchor, k as u8, *s);
                    if k % 3 == 2 {
                        ui.end_row();
                    }
                }
            });
        });
        ui.horizontal(|ui| {
            ui.label(tr("Deslocamento"));
            ui.add(crate::ui_ext::dv(&mut st.nudge[0]).speed(0.5).prefix("X "));
            ui.add(crate::ui_ext::dv(&mut st.nudge[1]).speed(0.5).prefix("Y "));
        });
        ui.horizontal(|ui| {
            ui.label(tr("Ângulo"));
            let r = ui.add(crate::ui_ext::dv(&mut st.angle).range(-360.0..=360.).speed(1.).suffix("°"));
            crate::ui_ext::wheel(ui, &r, &mut st.angle);
            for a in [0., 90., 180., 270.] {
                if ui.small_button(format!("{a}°")).clicked() {
                    st.angle = a;
                }
            }
        });
        ui.checkbox(&mut st.mirror, tr("espelhar"));
    });
    head("Fundo", false).show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            for (b, n) in [(PmiBg::Padrao, "padrão"), (PmiBg::Nenhum, "nenhum"), (PmiBg::Branco, "branco"), (PmiBg::Escuro, "escuro"), (PmiBg::Recorte, "opaco (esconde a geometria)")] {
                ui.selectable_value(&mut st.bg, b, tr(n));
            }
        });
    });
    head("Chamada", false).show(ui, |ui| {
        let mut show = !st.hide_leader;
        if ui.checkbox(&mut show, tr("mostrar a chamada")).changed() {
            st.hide_leader = !show;
        }
        ui.horizontal_wrapped(|ui| {
            ui.label(tr("Ponta"));
            for (a, n) in [(PmiArrow::Padrao, "padrão"), (PmiArrow::Cheia, "▶ cheia"), (PmiArrow::Aberta, "▷ aberta"), (PmiArrow::Ponto, "● ponto"), (PmiArrow::Barra, "⁄ barra"), (PmiArrow::Nenhuma, "nenhuma")] {
                ui.selectable_value(&mut st.arrow, a, tr(n));
            }
        });
        ui.horizontal(|ui| {
            ui.label(tr("Espessura"));
            let r = ui.add(crate::ui_ext::dv(&mut st.leader_width).range(0.0..=6.).speed(0.05).custom_formatter(fmt_def(2)));
            crate::ui_ext::wheel(ui, &r, &mut st.leader_width);
            ui.label(tr("Tipo de linha"));
            dash_combo(ui, "pmi_chamada_traco", &mut st.leader_dash);
        });
        color_row(ui, "cor da chamada", &mut st.leader_color, [20, 40, 110]);
    });
    head("Linhas e escala", false).show(ui, |ui| {
        color_row(ui, "cor das linhas", &mut st.color, [20, 40, 110]);
        ui.horizontal(|ui| {
            ui.label(tr("Espessura"));
            let r = ui.add(crate::ui_ext::dv(&mut st.line_width).range(0.0..=6.).speed(0.05).custom_formatter(fmt_def(2)));
            crate::ui_ext::wheel(ui, &r, &mut st.line_width);
        });
        ui.checkbox(&mut st.apply_scale, tr("aplicar escala (o texto acompanha o zoom)"));
    });
}
