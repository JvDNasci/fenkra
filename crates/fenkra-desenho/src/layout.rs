//! Folha → primitivas (mm, origem no canto inferior esquerdo, y para cima). A tela e as
//! exportações (PDF, SVG, DXF) desenham a mesma lista.

use crate::source::{circle, Models, Proj};
use crate::*;
use fenkra_core::pmi::{dash_pattern, dashed, PmiArrow, PmiBg, PmiFrame, PmiStyle};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Style {
    /// aresta visível (0,5)
    Visible,
    /// linha fina: cotas, hachuras, chamadas (0,25)
    Thin,
    /// aresta oculta (tracejada)
    Hidden,
    /// linha de centro / de corte (traço-ponto)
    Center,
    /// moldura e linha de corte (0,7)
    Thick,
}

impl Style {
    pub fn width(self) -> f64 {
        match self {
            Style::Visible => 0.5,
            Style::Thin | Style::Hidden | Style::Center => 0.25,
            Style::Thick => 0.7,
        }
    }
    /// traços (mm) — vazio = contínua
    pub fn dash(self) -> &'static [f64] {
        match self {
            Style::Hidden => &[3., 1.5],
            Style::Center => &[8., 1.5, 1., 1.5],
            _ => &[],
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Align {
    Left,
    Center,
    Right,
}

#[derive(Clone, PartialEq, Debug)]
pub enum Prim {
    Line { pts: Vec<V2>, style: Style },
    /// polígono cheio (pontas de seta)
    Fill { pts: Vec<V2> },
    /// texto: `pos` é a linha de base no alinhamento dado; ângulo em graus
    Text { pos: V2, text: String, size: f64, angle: f64, align: Align },
}

/// Primitiva e o objeto do desenho a que pertence (para seleção na tela).
#[derive(Clone, PartialEq, Debug)]
pub struct Item {
    pub owner: Option<Id>,
    pub prim: Prim,
    /// cor própria (None = preto)
    pub color: Option<[u8; 3]>,
    /// espessura própria (mm; None = a do tipo de linha)
    pub width: Option<f64>,
    /// texto em negrito
    pub bold: bool,
}

/// Largura aproximada do texto (fonte sem serifa).
pub fn text_width(t: &str, size: f64) -> f64 {
    t.chars().count() as f64 * size * 0.62
}

#[derive(Default)]
struct Out {
    items: Vec<Item>,
    owner: Option<Id>,
    /// cor e espessura das linhas, cor do texto, negrito e ponta das setas do objeto atual
    color: Option<[u8; 3]>,
    width: Option<f64>,
    tcolor: Option<[u8; 3]>,
    bold: bool,
    arrow_kind: PmiArrow,
}

impl Out {
    /// Começa outro objeto (estilo de volta ao padrão).
    fn reset(&mut self, owner: Option<Id>) {
        self.owner = owner;
        self.color = None;
        self.width = None;
        self.tcolor = None;
        self.bold = false;
        self.arrow_kind = PmiArrow::Padrao;
    }
    /// Cor, espessura (mm), negrito e setas de um estilo de anotação.
    fn style(&mut self, st: &PmiStyle) {
        self.color = st.color;
        self.width = (st.line_width > 0.).then_some(st.line_width as f64);
        self.tcolor = st.text_color.or(st.color);
        self.bold = st.bold;
        self.arrow_kind = st.arrow;
    }
    fn push(&mut self, prim: Prim, text: bool) {
        let color = if text { self.tcolor } else { self.color };
        self.items.push(Item { owner: self.owner, prim, color, width: if text { None } else { self.width }, bold: text && self.bold });
    }
    fn line(&mut self, pts: Vec<V2>, style: Style) {
        if pts.len() >= 2 {
            self.push(Prim::Line { pts, style }, false);
        }
    }
    fn fill(&mut self, pts: Vec<V2>) {
        if pts.len() >= 3 {
            self.push(Prim::Fill { pts }, false);
        }
    }
    fn seg(&mut self, a: V2, b: V2, style: Style) {
        self.line(vec![a, b], style);
    }
    fn rect(&mut self, lo: V2, hi: V2, style: Style) {
        self.line(vec![lo, [hi[0], lo[1]], hi, [lo[0], hi[1]], lo], style);
    }
    /// Linha com tipo de linha das anotações (0 contínua, 1 tracejada…).
    fn dline(&mut self, pts: Vec<V2>, closed: bool, style: Style, dash: u8) {
        let pat = dash_pattern(dash);
        if pat.is_empty() {
            let mut pts = pts;
            if closed && !pts.is_empty() {
                pts.push(pts[0]);
            }
            self.line(pts, style);
            return;
        }
        let pat: Vec<f64> = pat.iter().map(|x| x * 0.5).collect();
        for piece in dashed(&pts, closed, &pat) {
            self.line(piece, style);
        }
    }
    fn text(&mut self, pos: V2, text: impl Into<String>, size: f64, angle: f64, align: Align) {
        let text = text.into();
        if !text.is_empty() {
            self.push(Prim::Text { pos, text, size, angle, align }, true);
        }
    }
    /// seta com a ponta em `tip` apontando na direção `dir` (tipo de ponta do estilo atual)
    fn arrow(&mut self, tip: V2, dir: V2, size: f64) {
        let l = (dir[0] * dir[0] + dir[1] * dir[1]).sqrt().max(1e-12);
        let d = [dir[0] / l, dir[1] / l];
        let n = [-d[1], d[0]];
        let b = [tip[0] - d[0] * size, tip[1] - d[1] * size];
        let w = size * 0.18;
        match self.arrow_kind {
            PmiArrow::Padrao | PmiArrow::Cheia => self.fill(vec![tip, [b[0] + n[0] * w, b[1] + n[1] * w], [b[0] - n[0] * w, b[1] - n[1] * w]]),
            PmiArrow::Aberta => {
                let w = w * 1.4;
                self.line(vec![[b[0] + n[0] * w, b[1] + n[1] * w], tip, [b[0] - n[0] * w, b[1] - n[1] * w]], Style::Thin)
            }
            PmiArrow::Ponto => self.fill(circle(tip, size * 0.22)),
            PmiArrow::Barra => {
                let k = v2mul(v2unit(v2add(d, n)), size * 0.45);
                self.seg(v2sub(tip, k), v2add(tip, k), Style::Visible)
            }
            PmiArrow::Nenhuma => {}
        }
    }
    /// Gira (graus, anti-horário) e espelha em volta de `c` o que foi desenhado desde `start`.
    fn turn(&mut self, start: usize, c: V2, deg: f64, mirror: bool) {
        if deg.abs() < 1e-9 && !mirror {
            return;
        }
        let (s, co) = deg.to_radians().sin_cos();
        let f = |p: V2| {
            let mut d = v2sub(p, c);
            if mirror {
                d[0] = -d[0];
            }
            [c[0] + d[0] * co - d[1] * s, c[1] + d[0] * s + d[1] * co]
        };
        for it in &mut self.items[start..] {
            match &mut it.prim {
                Prim::Line { pts, .. } | Prim::Fill { pts } => {
                    for p in pts.iter_mut() {
                        *p = f(*p);
                    }
                }
                Prim::Text { pos, angle, align, .. } => {
                    *pos = f(*pos);
                    *angle += deg;
                    if mirror {
                        *align = match align {
                            Align::Left => Align::Right,
                            Align::Right => Align::Left,
                            Align::Center => Align::Center,
                        };
                    }
                }
            }
        }
    }
}

fn v2sub(a: V2, b: V2) -> V2 {
    [a[0] - b[0], a[1] - b[1]]
}
fn v2add(a: V2, b: V2) -> V2 {
    [a[0] + b[0], a[1] + b[1]]
}
fn v2mul(a: V2, k: f64) -> V2 {
    [a[0] * k, a[1] * k]
}
fn v2len(a: V2) -> f64 {
    (a[0] * a[0] + a[1] * a[1]).sqrt()
}
fn v2unit(a: V2) -> V2 {
    let l = v2len(a).max(1e-12);
    [a[0] / l, a[1] / l]
}

/// Ponto da vista (mm do modelo) → folha.
pub fn to_sheet(v: &View, p: &Proj, q: V2) -> V2 {
    [v.pos[0] + (q[0] - p.center[0]) * v.scale, v.pos[1] + (q[1] - p.center[1]) * v.scale]
}

/// Folha → ponto da vista (mm do modelo).
pub fn from_sheet(v: &View, p: &Proj, s: V2) -> V2 {
    [p.center[0] + (s[0] - v.pos[0]) / v.scale, p.center[1] + (s[1] - v.pos[1]) / v.scale]
}

/// Retângulo da vista na folha.
pub fn view_rect(v: &View, p: &Proj) -> (V2, V2) {
    let a = to_sheet(v, p, p.lo);
    let b = to_sheet(v, p, p.hi);
    ([a[0].min(b[0]) - 2., a[1].min(b[1]) - 2.], [a[0].max(b[0]) + 2., a[1].max(b[1]) + 2.])
}

/// Mantém as vistas projetadas alinhadas com a mãe (x para superior/inferior, y para laterais).
pub fn align_views(d: &mut Drawing) {
    let pos: Vec<(Id, V2)> = d.views.iter().map(|v| (v.id, v.pos)).collect();
    let scale: Vec<(Id, f64)> = d.views.iter().map(|v| (v.id, v.scale)).collect();
    for v in &mut d.views {
        if let ViewKind::Projected { parent, dir } = &v.kind {
            let Some(pp) = pos.iter().find(|x| x.0 == *parent).map(|x| x.1) else { continue };
            match dir {
                Dir::Topo | Dir::Base => v.pos[0] = pp[0],
                Dir::Esquerda | Dir::Direita | Dir::Tras => v.pos[1] = pp[1],
                _ => {}
            }
            if let Some(s) = scale.iter().find(|x| x.0 == *parent) {
                v.scale = s.1;
            }
        }
        if let ViewKind::Section { parent, .. } = &v.kind {
            if let Some(s) = scale.iter().find(|x| x.0 == *parent) {
                v.scale = s.1;
            }
        }
    }
}

/// Valor de uma cota (mm do modelo ou graus) e os pontos na folha.
pub fn dim_value(dm: &Dim, v: &View, p: &Proj) -> Option<(f64, Vec<V2>)> {
    let ax = p.axes?;
    let pts: Vec<V2> = dm.anchors.iter().map(|a| to_sheet(v, p, ax.project(*a))).collect();
    let q: Vec<V2> = dm.anchors.iter().map(|a| ax.project(*a)).collect();
    let val = match dm.kind {
        DimKind::Horizontal if q.len() >= 2 => (q[1][0] - q[0][0]).abs(),
        DimKind::Vertical if q.len() >= 2 => (q[1][1] - q[0][1]).abs(),
        DimKind::Aligned | DimKind::Radius if q.len() >= 2 => v2len(v2sub(q[1], q[0])),
        DimKind::Diameter if q.len() >= 2 => 2. * v2len(v2sub(q[1], q[0])),
        DimKind::Angle if q.len() >= 3 => {
            let (a, b) = (v2sub(q[1], q[0]), v2sub(q[2], q[0]));
            (a[0] * b[1] - a[1] * b[0]).atan2(a[0] * b[0] + a[1] * b[1]).abs().to_degrees()
        }
        _ => return None,
    };
    Some((val, pts))
}

/// Texto da cota: valor (ou texto próprio, onde "<>" vira o valor) e tolerância.
pub fn dim_text(d: &Drawing, dm: &Dim, val: f64) -> String {
    let n = d.num(val);
    let base = match dm.kind {
        DimKind::Radius => format!("R{n}"),
        DimKind::Diameter => format!("Ø{n}"),
        DimKind::Angle => format!("{n}°"),
        _ => n,
    };
    let t = if dm.text.is_empty() { base } else { dm.text.replace("<>", &base) };
    if dm.tol.is_empty() {
        t
    } else {
        format!("{t} {}", dm.tol)
    }
}

/// Todas as primitivas da folha.
pub fn sheet(d: &Drawing, sheet: Id, m: &mut Models) -> Vec<Item> {
    let mut o = Out::default();
    let Some(sh) = d.sheet(sheet) else { return vec![] };
    let fields = sheet_fields(d, sh, m);
    frame(&mut o, d, sh, &fields);
    let th = d.text_height;
    let views: Vec<&View> = d.views.iter().filter(|v| v.sheet == sheet).collect();
    for v in &views {
        let p = m.proj(d, v.id);
        o.reset(Some(v.id));
        // sombreado (por baixo das arestas)
        if v.render != Render::Arestas {
            for (t, g) in &p.shade {
                o.color = Some([*g, *g, *g]);
                o.fill(t.iter().map(|q| to_sheet(v, &p, *q)).collect());
            }
        }
        o.color = v.color;
        for (pl, k) in &p.lines {
            let style = match k {
                3 => Style::Hidden,
                4 | 6 => Style::Thin,
                5 => Style::Center,
                _ => Style::Visible,
            };
            // só sombreado: sem arestas (a borda do detalhe fica)
            if v.render == Render::Sombreado && *k != 4 {
                continue;
            }
            o.width = (style == Style::Visible && v.line_width > 0.).then_some(v.line_width);
            o.line(pl.iter().map(|q| to_sheet(v, &p, *q)).collect(), style);
        }
        o.width = None;
        o.color = None;
        for (q, t) in &p.labels {
            let s = to_sheet(v, &p, *q);
            o.text([s[0] + 1., s[1] + 1.], t.clone(), th * 0.7, 0., Align::Left);
        }
        // hachura a 45° (espaçamento fixo na folha)
        if !p.hatch.is_empty() {
            let loops: Vec<Vec<V2>> = p.hatch.iter().map(|lp| lp.iter().map(|q| to_sheet(v, &p, *q)).collect()).collect();
            for lp in &loops {
                o.line(lp.clone(), Style::Visible);
            }
            for (a, b) in hatch(&loops, 2.5) {
                o.seg(a, b, Style::Thin);
            }
        }
        let (lo, hi) = view_rect(v, &p);
        if let Some(e) = &p.error {
            o.text([lo[0], hi[1] + 2.], format!("! {e}"), th * 0.8, 0., Align::Left);
        }
        if v.label {
            let lbl = match &v.kind {
                ViewKind::Section { label, .. } => format!("CORTE {label}-{label} ({})", scale_text(v.scale)),
                ViewKind::Detail { label, .. } => format!("DETALHE {label} ({})", scale_text(v.scale)),
                _ => format!("{} ({})", v.name, scale_text(v.scale)),
            };
            o.text([(lo[0] + hi[0]) / 2., lo[1] - th * 1.8], lbl, th, 0., Align::Center);
        }
        // marcas nas vistas mãe: linha de corte e círculo de detalhe
        for c in d.views.iter() {
            match &c.kind {
                ViewKind::Section { parent, a, b, label } if *parent == v.id => {
                    o.owner = Some(c.id);
                    let (sa, sb) = (to_sheet(v, &p, *a), to_sheet(v, &p, *b));
                    o.seg(sa, sb, Style::Center);
                    let u = v2unit(v2sub(sb, sa));
                    // seta para o lado que se vê (oposto ao observador)
                    let n = [u[1], -u[0]];
                    for (e, s) in [(sa, -1.), (sb, 1.)] {
                        let e2 = v2add(e, v2mul(u, s * 4.));
                        o.seg(e, e2, Style::Thick);
                        let tail = v2add(e2, v2mul(n, -6.));
                        o.seg(e2, tail, Style::Thin);
                        o.arrow(e2, n, 3.);
                        o.text(v2add(v2add(tail, v2mul(u, s * 2.5)), [0., -th / 2.]), label.clone(), th * 1.2, 0., Align::Center);
                    }
                    o.owner = Some(v.id);
                }
                ViewKind::Detail { parent, center, radius, label } if *parent == v.id => {
                    o.owner = Some(c.id);
                    let pts = circle(*center, *radius).into_iter().map(|q| to_sheet(v, &p, q)).collect();
                    o.line(pts, Style::Thin);
                    let at = to_sheet(v, &p, [center[0] + radius * 0.75, center[1] + radius * 0.75]);
                    o.text([at[0] + 1., at[1] + 1.], label.clone(), th * 1.2, 0., Align::Left);
                    o.owner = Some(v.id);
                }
                _ => {}
            }
        }
    }
    // cotas
    for dm in &d.dims {
        let Some(v) = d.view(dm.view).filter(|v| v.sheet == sheet) else { continue };
        let p = m.proj(d, v.id);
        let Some((val, pts)) = dim_value(dm, v, &p) else { continue };
        o.reset(Some(dm.id));
        let st = dm.style.over(&v.ann);
        o.style(&st);
        let th = if st.size > 0. { st.size as f64 } else { th };
        let text = format!("{}{}{}", st.prefix, dim_text(d, dm, val), st.suffix);
        dimension(&mut o, dm, &pts, &text, th);
    }
    // notas e símbolos
    for n in d.notes.iter().filter(|n| n.sheet == sheet) {
        o.reset(Some(n.id));
        // notas também aceitam campos automáticos ({codigo}, {material}…)
        let mut n2 = n.clone();
        n2.text = crate::resolve(&n.text, &fields);
        // presa à vista: a ponta da chamada segue o ponto do modelo
        let view = n.view.and_then(|id| d.view(id));
        if let (Some(v), Some(a3)) = (view, n.anchor3) {
            let p = m.proj(d, v.id);
            if let Some(ax) = p.axes {
                n2.leader = Some(to_sheet(v, &p, ax.project(a3)));
            }
        }
        n2.style = view.map(|v| n.style.over(&v.ann)).unwrap_or_else(|| n.style.clone());
        o.style(&n2.style);
        note(&mut o, &n2);
    }
    // tabelas
    for t in d.tables.iter().filter(|t| t.sheet == sheet) {
        o.reset(Some(t.id));
        let data = m.table(t, d);
        table(&mut o, t.pos, t.kind.label(), &data.headers, &data.rows, th * 0.8);
    }
    o.items
}

/// Valores dos campos automáticos da folha (propriedades da origem da primeira vista).
pub fn sheet_fields(d: &Drawing, sh: &Sheet, m: &mut Models) -> std::collections::HashMap<String, String> {
    let mut f = std::collections::HashMap::new();
    let first = d.views.iter().find(|v| v.sheet == sh.id).or(d.views.first());
    let idx = d.sheets.iter().position(|s| s.id == sh.id).unwrap_or(0) + 1;
    let (pn, desc, mat) = first.and_then(|v| m.props(&v.source)).unwrap_or_default();
    let vol = first.and_then(|v| m.load(&v.source).shape.as_ref().map(|s| s.mass().volume)).unwrap_or(0.);
    f.insert("codigo".into(), pn);
    f.insert("descricao".into(), if desc.is_empty() { d.name.clone() } else { desc });
    f.insert("material".into(), mat);
    f.insert("volume".into(), if vol > 0. { d.num(vol / 1000.) } else { String::new() });
    f.insert("massa".into(), if vol > 0. { format!("{} kg", d.num(vol / 1000. * d.title.density / 1000.)) } else { String::new() });
    f.insert("escala".into(), first.map(|v| scale_text(v.scale)).unwrap_or_else(|| "1:1".into()));
    f.insert("folha".into(), idx.to_string());
    f.insert("folhas".into(), d.sheets.len().to_string());
    f.insert("formato".into(), sh.format.label().to_string());
    f.insert("data".into(), crate::today());
    f.insert("autor".into(), std::env::var("USERNAME").or_else(|_| std::env::var("USER")).unwrap_or_default());
    f.insert("arquivo".into(), first.map(|v| v.source.path.rsplit(['/', '\\']).next().unwrap_or("").to_string()).unwrap_or_default());
    f.insert("revisao".into(), d.title.revision.clone());
    f.insert("titulo".into(), d.name.clone());
    f
}

/// Célula da legenda: posição (mm a partir do canto inferior esquerdo da legenda), tamanho,
/// rótulo e valor (texto com campos automáticos).
struct Cell<'a> {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    label: &'a str,
    value: String,
    size: f64,
}

fn frame(o: &mut Out, d: &Drawing, sh: &Sheet, fields: &std::collections::HashMap<String, String>) {
    let [w, h] = sh.size();
    o.rect([0., 0.], [w, h], Style::Thin);
    // margem de arquivamento à esquerda (ISO 5457)
    let (l, r) = (20., 10.);
    o.rect([l, r], [w - r, h - r], Style::Thick);
    // marcas de centro
    for (a, b) in [([w / 2., 0.], [w / 2., r]), ([w / 2., h - r], [w / 2., h]), ([0., h / 2.], [l, h / 2.]), ([w - r, h / 2.], [w, h / 2.])] {
        o.seg(a, b, Style::Thick);
    }
    // zoneamento: números na horizontal, letras na vertical (de cima para baixo), ~50 mm cada
    if sh.zones {
        let (iw, ih) = (w - l - r, h - 2. * r);
        let nx = ((iw / 50.).round() as usize).max(2);
        let ny = ((ih / 50.).round() as usize).max(2);
        for i in 0..=nx {
            let x = l + iw * i as f64 / nx as f64;
            if i > 0 && i < nx {
                o.seg([x, r - 4.], [x, r], Style::Thin);
                o.seg([x, h - r], [x, h - r + 4.], Style::Thin);
            }
            if i < nx {
                let xc = l + iw * (i as f64 + 0.5) / nx as f64;
                o.text([xc, r - 3.6], (i + 1).to_string(), 2.5, 0., Align::Center);
                o.text([xc, h - r + 1.2], (i + 1).to_string(), 2.5, 0., Align::Center);
            }
        }
        let letters = "ABCDEFGHJKLMNPQRSTUVWXYZ";
        for j in 0..=ny {
            let y = r + ih * j as f64 / ny as f64;
            if j > 0 && j < ny {
                o.seg([l - 4., y], [l, y], Style::Thin);
                o.seg([w - r, y], [w - r + 4., y], Style::Thin);
            }
            if j < ny {
                let yc = h - r - ih * (j as f64 + 0.5) / ny as f64 - 1.2;
                let c = letters.chars().nth(j % letters.len()).unwrap().to_string();
                o.text([l - 2., yc], c.clone(), 2.5, 0., Align::Center);
                o.text([w - r + 2.4, yc], c, 2.5, 0., Align::Center);
            }
        }
        // linha de corte das zonas a 4 mm da moldura
        o.rect([l - 4., r - 4.], [w - r + 4., h - r + 4.], Style::Thin);
    }
    let t = &d.title;
    let v = |s: &str| crate::resolve(s, fields);
    let get = |k: &str| fields.get(k).cloned().unwrap_or_default();
    let c = |x: f64, y: f64, w: f64, h: f64, label: &'static str, value: String| Cell { x, y, w, h, label, value, size: 3.0 };
    let (bw, cells, diedro): (f64, Vec<Cell>, Option<(f64, f64, f64)>) = match t.style {
        TitleStyle::Iso7200 => (
            180.,
            vec![
                Cell { size: 4.2, ..c(0., 27., 70., 9., "", v(&t.company)) },
                Cell { size: 4.2, ..c(70., 27., 110., 9., "TÍTULO", v(&t.title)) },
                c(0., 18., 60., 9., "CÓDIGO", v(&t.pn)),
                c(60., 18., 60., 9., "MATERIAL", v(&t.material)),
                c(120., 18., 30., 9., "ESCALA", get("escala")),
                c(150., 18., 30., 9., "FORMATO", get("formato")),
                c(0., 9., 60., 9., "DESENHO", v(&t.drawn_by)),
                c(60., 9., 30., 9., "VERIFICADO", v(&t.checked_by)),
                c(90., 9., 30., 9., "DATA", v(&t.date)),
                c(120., 9., 30., 9., "REVISÃO", v(&t.revision)),
                c(150., 9., 30., 9., "FOLHA", format!("{}/{}", get("folha"), get("folhas"))),
                c(0., 0., 130., 9., "OBSERVAÇÕES", v(&t.notes)),
                c(130., 0., 50., 9., "MASSA", get("massa")),
            ],
            None,
        ),
        TitleStyle::Abnt => (
            175.,
            vec![
                Cell { size: 4.2, ..c(0., 36., 70., 14., "", v(&t.company)) },
                Cell { size: 4.0, ..c(70., 36., 105., 14., "TÍTULO", v(&t.title)) },
                c(0., 27., 70., 9., "DESENHO Nº", v(&t.pn)),
                c(70., 27., 65., 9., "MATERIAL", v(&t.material)),
                c(135., 27., 20., 9., "ESCALA", get("escala")),
                c(155., 27., 20., 9., "UNIDADE", "mm".into()),
                c(0., 18., 45., 9., "DESENHISTA", v(&t.drawn_by)),
                c(45., 18., 45., 9., "VERIFICADO", v(&t.checked_by)),
                c(90., 18., 45., 9., "APROVADO", v(&t.approved_by)),
                c(135., 18., 40., 9., "DATA", v(&t.date)),
                c(0., 9., 25., 9., "REVISÃO", v(&t.revision)),
                c(25., 9., 25., 9., "FOLHA", format!("{}/{}", get("folha"), get("folhas"))),
                c(50., 9., 25., 9., "FORMATO", get("formato")),
                c(75., 9., 40., 9., "MASSA", get("massa")),
                c(0., 0., 115., 9., "OBSERVAÇÕES", v(&t.notes)),
                c(115., 0., 60., 18., "PROJEÇÃO", String::new()),
            ],
            Some((115., 0., 60.)),
        ),
        TitleStyle::Ansi => (
            178.,
            vec![
                Cell { size: 4.2, ..c(0., 26., 100., 14., "", v(&t.company)) },
                Cell { size: 4.0, ..c(100., 26., 78., 14., "TITLE", v(&t.title)) },
                c(0., 17., 20., 9., "SIZE", get("formato").replace("ANSI ", "").chars().take(1).collect()),
                c(20., 17., 30., 9., "CAGE CODE", String::new()),
                c(50., 17., 98., 9., "DWG NO", v(&t.pn)),
                c(148., 17., 30., 9., "REV", v(&t.revision)),
                c(0., 8., 40., 9., "SCALE", get("escala")),
                c(40., 8., 50., 9., "WEIGHT", get("massa")),
                c(90., 8., 88., 9., "SHEET", format!("{} OF {}", get("folha"), get("folhas"))),
                c(0., 0., 60., 8., "DRAWN", v(&t.drawn_by)),
                c(60., 0., 60., 8., "CHECKED", v(&t.checked_by)),
                c(120., 0., 58., 8., "DATE", v(&t.date)),
            ],
            None,
        ),
        TitleStyle::Simples => (
            150.,
            vec![
                Cell { size: 3.6, ..c(0., 9., 150., 9., "TÍTULO", v(&t.title)) },
                c(0., 0., 60., 9., "CÓDIGO", v(&t.pn)),
                c(60., 0., 30., 9., "ESCALA", get("escala")),
                c(90., 0., 30., 9., "DATA", v(&t.date)),
                c(120., 0., 30., 9., "FOLHA", format!("{}/{}", get("folha"), get("folhas"))),
            ],
            None,
        ),
    };
    let x0 = w - r - bw;
    let y0 = r;
    let top = cells.iter().map(|k| k.y + k.h).fold(0., f64::max);
    o.rect([x0, y0], [w - r, y0 + top], Style::Thick);
    for k in &cells {
        o.rect([x0 + k.x, y0 + k.y], [x0 + k.x + k.w, y0 + k.y + k.h], Style::Thin);
        if !k.label.is_empty() {
            o.text([x0 + k.x + 1.2, y0 + k.y + k.h - 2.4], k.label.to_string(), 1.8, 0., Align::Left);
        }
        // texto que não cabe diminui
        let mut s = k.size;
        while s > 1.6 && text_width(&k.value, s) > k.w - 2.4 {
            s -= 0.2;
        }
        let ty = if k.label.is_empty() { y0 + k.y + (k.h - s) / 2. } else { y0 + k.y + 1.6 };
        o.text([x0 + k.x + 1.2, ty], k.value.clone(), s, 0., Align::Left);
    }
    // quadro de tolerâncias gerais (ANSI): à esquerda da legenda
    if t.style == TitleStyle::Ansi {
        let (bx, bh) = (x0 - 70., top);
        o.rect([bx, y0], [x0, y0 + bh], Style::Thick);
        for (i, line) in ["UNLESS OTHERWISE SPECIFIED:", "DIMENSIONS ARE IN MILLIMETERS", "TOLERANCES: X.X ±0.1  X.XX ±0.05", "ANGLES ±0.5°", "FIRST ANGLE PROJECTION"].iter().enumerate() {
            o.text([bx + 1.5, y0 + bh - 5. - i as f64 * 5.5], line.to_string(), 2.0, 0., Align::Left);
        }
    }
    // símbolo do 1º diedro: tronco de cone de frente e de lado
    let (sx, sy, sw) = diedro.map(|(x, y, ww)| (x0 + x, y0 + y, ww)).unwrap_or((w - r - 30., y0 + top + 2., 28.));
    let cx = sx + sw * 0.32;
    let cy = sy + 8.;
    o.line(vec![[cx - 7., cy - 3.], [cx + 3., cy - 5.], [cx + 3., cy + 5.], [cx - 7., cy + 3.], [cx - 7., cy - 3.]], Style::Visible);
    let ex = sx + sw * 0.72;
    o.line(crate::source::circle([ex, cy], 5.), Style::Visible);
    o.line(crate::source::circle([ex, cy], 3.), Style::Visible);
    o.text([sx + sw / 2., sy + 1.2], "1º diedro".to_string(), 2.2, 0., Align::Center);
}

/// Segmentos de hachura a 45° dentro dos contornos (regra par/ímpar).
pub fn hatch(loops: &[Vec<V2>], step: f64) -> Vec<(V2, V2)> {
    // gira -45° para hachurar com linhas horizontais
    let (c, s) = (std::f64::consts::FRAC_1_SQRT_2, std::f64::consts::FRAC_1_SQRT_2);
    let rot = |p: V2| [p[0] * c + p[1] * s, -p[0] * s + p[1] * c];
    let back = |p: V2| [p[0] * c - p[1] * s, p[0] * s + p[1] * c];
    let edges: Vec<(V2, V2)> = loops
        .iter()
        .flat_map(|lp| {
            let r: Vec<V2> = lp.iter().map(|p| rot(*p)).collect();
            let n = r.len();
            (0..n).map(move |i| (r[i], r[(i + 1) % n]))
        })
        .collect();
    if edges.is_empty() {
        return vec![];
    }
    let ymin = edges.iter().map(|e| e.0[1].min(e.1[1])).fold(f64::MAX, f64::min);
    let ymax = edges.iter().map(|e| e.0[1].max(e.1[1])).fold(f64::MIN, f64::max);
    let mut out = vec![];
    let mut y = (ymin / step).floor() * step + step * 0.5;
    while y < ymax {
        let mut xs: Vec<f64> = edges
            .iter()
            .filter(|(a, b)| (a[1] <= y) != (b[1] <= y))
            .map(|(a, b)| a[0] + (y - a[1]) / (b[1] - a[1]) * (b[0] - a[0]))
            .collect();
        xs.sort_by(f64::total_cmp);
        for pair in xs.chunks(2) {
            if let [x1, x2] = pair {
                out.push((back([*x1, y]), back([*x2, y])));
            }
        }
        y += step;
    }
    out
}

fn dot2(a: V2, b: V2) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}

fn dimension(o: &mut Out, dm: &Dim, pts: &[V2], text: &str, th: f64) {
    let a = th * 0.9;
    let gap = 1.;
    let over = 1.5;
    // setas: quais e de que lado
    let (first, second) = match dm.arrows {
        3 => (true, false),
        4 => (false, true),
        5 => (false, false),
        _ => (true, true),
    };
    match dm.kind {
        DimKind::Horizontal | DimKind::Vertical | DimKind::Aligned => {
            let (p1, p2) = (pts[0], pts[1]);
            // direção da cota e normal (para o lado do afastamento)
            let (u, n) = match dm.kind {
                DimKind::Horizontal => ([1., 0.], [0., 1.]),
                DimKind::Vertical => ([0., 1.], [-1., 0.]),
                _ => {
                    let u = v2unit(v2sub(p2, p1));
                    (u, [-u[1], u[0]])
                }
            };
            // linha de cota a `offset` do ponto mais afastado no sentido da normal
            let base = dot2(p1, n).max(dot2(p2, n));
            let base = if dm.offset >= 0. { base + dm.offset } else { dot2(p1, n).min(dot2(p2, n)) + dm.offset };
            let q1 = v2add(p1, v2mul(n, base - dot2(p1, n)));
            let q2 = v2add(p2, v2mul(n, base - dot2(p2, n)));
            let sg = if dm.offset >= 0. { 1. } else { -1. };
            for (p, q) in [(p1, q1), (p2, q2)] {
                let dir = v2unit(v2sub(q, p));
                if v2len(v2sub(q, p)) > gap {
                    o.seg(v2add(p, v2mul(dir, gap)), v2add(q, v2mul(n, over * sg)), Style::Thin);
                } else {
                    o.seg(q, v2add(q, v2mul(n, over * sg)), Style::Thin);
                }
            }
            o.seg(q1, q2, Style::Thin);
            let d = v2sub(q2, q1);
            let ud = v2unit(d);
            let inside = match dm.arrows {
                1 => true,
                2 => false,
                _ => v2len(d) > 2. * a + 0.5,
            };
            if inside {
                if first {
                    o.arrow(q1, v2mul(d, -1.), a);
                }
                if second {
                    o.arrow(q2, d, a);
                }
            } else {
                // setas por fora, com um trecho de linha além de cada uma
                if first {
                    o.arrow(q1, d, a);
                    o.seg(q1, v2sub(q1, v2mul(ud, 2. * a)), Style::Thin);
                }
                if second {
                    o.arrow(q2, v2mul(d, -1.), a);
                    o.seg(q2, v2add(q2, v2mul(ud, 2. * a)), Style::Thin);
                }
            }
            let mid = v2mul(v2add(q1, q2), 0.5);
            // texto paralelo à cota, legível (de baixo ou da direita)
            let mut ang = u[1].atan2(u[0]).to_degrees();
            let mut nn = n;
            if ang > 90.01 || ang < -89.99 {
                ang -= 180. * ang.signum();
                nn = v2mul(n, -1.);
            }
            let up = if dot2(nn, n) * sg > 0. { 0.8 } else { -(th + 0.8) };
            let rn = [-(ang.to_radians().sin()), ang.to_radians().cos()];
            let mut tp = v2add(mid, v2mul(rn, up));
            if let Some(off) = dm.text_pos {
                // texto arrastado: a linha de cota se estende até ele (ou uma chamada, se saiu de perto)
                tp = v2add(tp, off);
                let t = dot2(v2sub(tp, q1), ud);
                let side = dot2(v2sub(tp, q1), n).abs();
                let half = text_width(text, th) / 2.;
                if side < th * 2.5 {
                    if t + half > v2len(d) {
                        o.seg(q2, v2add(q1, v2mul(ud, t + half)), Style::Thin);
                    }
                    if t - half < 0. {
                        o.seg(q1, v2add(q1, v2mul(ud, t - half)), Style::Thin);
                    }
                } else {
                    let near = v2add(q1, v2mul(ud, t.clamp(0., v2len(d))));
                    o.seg(near, v2add(tp, v2mul(rn, th * 0.4)), Style::Thin);
                }
            }
            o.text(tp, text.to_string(), th, ang, Align::Center);
        }
        DimKind::Radius | DimKind::Diameter => {
            let (c, p) = (pts[0], pts[1]);
            let u = v2unit(v2sub(p, c));
            let r = v2len(v2sub(p, c));
            let out_len = dm.offset.max(4.);
            let tail0 = v2add(p, v2mul(u, out_len));
            // texto arrastado: o cotovelo vai junto
            let tail = dm.text_pos.map(|off| v2add(tail0, off)).unwrap_or(tail0);
            if dm.kind == DimKind::Diameter {
                let p0 = v2sub(c, v2mul(u, r));
                o.seg(p0, p, Style::Thin);
                if first && dm.arrows != 5 {
                    o.arrow(p0, v2mul(u, -1.), a);
                }
            } else {
                o.seg(c, p, Style::Thin);
            }
            o.seg(p, tail, Style::Thin);
            if second && dm.arrows != 5 {
                if dm.arrows == 2 {
                    // por fora: a seta chega no arco vindo de dentro
                    o.arrow(p, v2mul(u, -1.), a);
                } else {
                    o.arrow(p, u, a);
                }
            }
            let right = tail[0] >= p[0] - 1e-9;
            let shelf = v2add(tail, [if right { text_width(text, th) + 1. } else { -(text_width(text, th) + 1.) }, 0.]);
            o.seg(tail, shelf, Style::Thin);
            let al = if right { Align::Left } else { Align::Right };
            o.text([tail[0] + if right { 0.5 } else { -0.5 }, tail[1] + 0.8], text.to_string(), th, 0., al);
        }
        DimKind::Angle => {
            let (c, p1, p2) = (pts[0], pts[1], pts[2]);
            let r = v2len(v2sub(p1, c)).min(v2len(v2sub(p2, c))).max(5.) * 0.6 + dm.offset;
            let a1 = (p1[1] - c[1]).atan2(p1[0] - c[0]);
            let mut a2 = (p2[1] - c[1]).atan2(p2[0] - c[0]);
            let mut sweep = a2 - a1;
            while sweep <= -std::f64::consts::PI {
                sweep += std::f64::consts::TAU;
            }
            while sweep > std::f64::consts::PI {
                sweep -= std::f64::consts::TAU;
            }
            a2 = a1 + sweep;
            let arc: Vec<V2> = (0..=32)
                .map(|i| {
                    let t = a1 + (a2 - a1) * i as f64 / 32.;
                    [c[0] + r * t.cos(), c[1] + r * t.sin()]
                })
                .collect();
            for (p, ang) in [(p1, a1), (p2, a2)] {
                let e = [c[0] + (r + over) * ang.cos(), c[1] + (r + over) * ang.sin()];
                if v2len(v2sub(p, c)) < r {
                    o.seg(p, e, Style::Thin);
                }
            }
            let s = sweep.signum();
            if first {
                o.arrow(arc[0], [s * a1.sin(), -s * a1.cos()], a);
            }
            if second {
                o.arrow(arc[32], [-s * a2.sin(), s * a2.cos()], a);
            }
            o.line(arc, Style::Thin);
            let am = (a1 + a2) / 2.;
            let mut tp = [c[0] + (r + th) * am.cos(), c[1] + (r + th) * am.sin() - th / 2.];
            if let Some(off) = dm.text_pos {
                tp = v2add(tp, off);
            }
            o.text(tp, text.to_string(), th, 0., Align::Center);
        }
    }
}

/// Retângulo de tamanho (w, h) com o ponto de ancoragem do estilo em `at` (y para cima).
fn anchored(at: V2, w: f64, h: f64, st: &PmiStyle) -> (V2, V2) {
    let (col, row) = ((st.anchor.min(8) % 3) as f64, (st.anchor.min(8) / 3) as f64);
    // deslocamento fino: x para a direita, y para baixo (como na tela)
    let x0 = at[0] + st.nudge[0] as f64 - w * col / 2.;
    let top = at[1] - st.nudge[1] as f64 + h * row / 2.;
    ([x0, top - h], [x0 + w, top])
}

/// Contorno da moldura (y para cima) em volta de lo–hi.
fn frame_paths(f: PmiFrame, lo: V2, hi: V2) -> Vec<(Vec<V2>, bool)> {
    f.paths(lo[0], -hi[1], hi[0], -lo[1]).into_iter().map(|(v, c)| (v.into_iter().map(|q| [q[0], -q[1]]).collect(), c)).collect()
}

/// Fundo e moldura de uma caixa de anotação; devolve o retângulo de fora.
fn frame_box(o: &mut Out, lo: V2, hi: V2, st: &PmiStyle, frame: PmiFrame, bg: PmiBg) -> (V2, V2) {
    let paths = frame_paths(frame, lo, hi);
    let (col, w) = (o.color, o.width);
    let fill = match bg {
        PmiBg::Branco | PmiBg::Recorte => Some([255, 255, 255]),
        PmiBg::Escuro => Some([40, 30, 22]),
        _ => None,
    };
    if let Some(c) = fill {
        o.color = Some(c);
        match paths.iter().find(|p| p.1) {
            Some((pts, _)) => o.fill(pts.clone()),
            None => o.fill(vec![lo, [hi[0], lo[1]], hi, [lo[0], hi[1]]]),
        }
    }
    o.color = st.frame_color.or(col);
    if st.frame_width > 0. {
        o.width = Some(st.frame_width as f64);
    }
    let (mut a, mut b) = (lo, hi);
    for (pts, closed) in paths {
        for p in &pts {
            a = [a[0].min(p[0]), a[1].min(p[1])];
            b = [b[0].max(p[0]), b[1].max(p[1])];
        }
        o.dline(pts, closed, Style::Thin, st.frame_dash);
    }
    o.color = col;
    o.width = w;
    (a, b)
}

/// Bloco de texto com moldura, fundo, justificação, espaçamento, ancoragem, ângulo e espelho.
/// Devolve o retângulo (antes de girar).
fn text_box(o: &mut Out, at: V2, lines: &[String], s: f64, st: &PmiStyle, frame_default: PmiFrame, bg_default: PmiBg) -> (V2, V2) {
    let start = o.items.len();
    let ls = s * 1.5 * if st.line_spacing > 0. { st.line_spacing as f64 } else { 1. };
    let tw = lines.iter().map(|l| text_width(l, s)).fold(0., f64::max);
    let pad = [s * 0.45, s * 0.4];
    let w = (tw + 2. * pad[0]).max(st.frame_fixed as f64);
    let h = s + ls * (lines.len().max(1) - 1) as f64 + 2. * pad[1];
    let (lo, hi) = anchored(at, w, h, st);
    let frame = if st.frame != PmiFrame::Padrao { st.frame } else { frame_default };
    let bg = if st.bg != PmiBg::Padrao { st.bg } else { bg_default };
    let (bl, bh) = frame_box(o, lo, hi, st, frame, bg);
    let tc = o.tcolor;
    if bg == PmiBg::Escuro {
        o.tcolor = Some([255, 255, 255]);
    }
    for (i, l) in lines.iter().enumerate() {
        let y = hi[1] - pad[1] - s - i as f64 * ls;
        let (x, al) = match st.justify {
            1 => ((lo[0] + hi[0]) / 2., Align::Center),
            2 => (hi[0] - pad[0], Align::Right),
            _ => (lo[0] + pad[0], Align::Left),
        };
        o.text([x, y], l.clone(), s, 0., al);
    }
    o.tcolor = tc;
    let c = v2mul(v2add(bl, bh), 0.5);
    o.turn(start, c, st.angle as f64, st.mirror);
    (bl, bh)
}

/// Gira/espelha um ponto como o rótulo (para achar o lado da chamada).
fn turn_pt(p: V2, c: V2, deg: f64, mirror: bool, inverse: bool) -> V2 {
    let (s, co) = (if inverse { -deg } else { deg }).to_radians().sin_cos();
    let mut d = v2sub(p, c);
    if mirror && !inverse {
        d[0] = -d[0];
    }
    let mut r = [d[0] * co - d[1] * s, d[0] * s + d[1] * co];
    if mirror && inverse {
        r[0] = -r[0];
    }
    v2add(c, r)
}

/// Chamada da caixa lo–hi até `tip`: sai do lado voltado para a ponta, com um patamar.
fn leader_box(o: &mut Out, lo: V2, hi: V2, tip: V2, s: f64, st: &PmiStyle) {
    if st.hide_leader {
        return;
    }
    let c = v2mul(v2add(lo, hi), 0.5);
    let (ang, mir) = (st.angle as f64, st.mirror);
    let a = turn_pt(tip, c, ang, mir, true);
    if a[0] > lo[0] - 0.5 && a[0] < hi[0] + 0.5 && a[1] > lo[1] - 0.5 && a[1] < hi[1] + 0.5 {
        return;
    }
    let d = v2sub(a, c);
    let (w, h) = (hi[0] - lo[0], hi[1] - lo[1]);
    let land = s * 1.2;
    let (att, out) = if d[0].abs() * h >= d[1].abs() * w {
        if d[0] < 0. { ([lo[0], c[1]], [-land, 0.]) } else { ([hi[0], c[1]], [land, 0.]) }
    } else if d[1] > 0. {
        ([c[0], hi[1]], [0., land * 0.7])
    } else {
        ([c[0], lo[1]], [0., -land * 0.7])
    };
    let att2 = turn_pt(att, c, ang, mir, false);
    let knee = turn_pt(v2add(att, out), c, ang, mir, false);
    leader_line(o, vec![att2, knee, tip], s, st, true);
}

/// Linha de chamada com o estilo (cor, espessura, tipo de linha) e a ponta em `pts.last()`.
fn leader_line(o: &mut Out, pts: Vec<V2>, s: f64, st: &PmiStyle, arrow: bool) {
    if st.hide_leader || pts.len() < 2 {
        return;
    }
    let (col, w) = (o.color, o.width);
    o.color = st.leader_color.or(col);
    if st.leader_width > 0. {
        o.width = Some(st.leader_width as f64);
    }
    let n = pts.len();
    let (tip, from) = (pts[n - 1], pts[n - 2]);
    o.dline(pts, false, Style::Thin, st.leader_dash);
    if arrow {
        o.arrow(tip, v2sub(tip, from), s * 0.9);
    }
    o.color = col;
    o.width = w;
}

/// Quadro de controle geométrico (ISO 1101) com o ponto de ancoragem em `at`; devolve a caixa.
fn fcf(o: &mut Out, at: V2, sym: &str, rows: &[Vec<String>], s: f64, st: &PmiStyle) -> (V2, V2) {
    let start = o.items.len();
    let h = s * 2.;
    let pad = s * 0.5;
    let w0 = (text_width(sym, s * 1.2) + 2. * pad).max(h);
    let cw = |t: &str| (text_width(t, s) + 2. * pad).max(h * 0.9);
    let row_w: Vec<f64> = rows.iter().map(|r| r.iter().map(|c| cw(c)).sum()).collect();
    let total_w = w0 + row_w.iter().cloned().fold(0., f64::max);
    let total_h = h * rows.len().max(1) as f64;
    let (lo, hi) = anchored(at, total_w, total_h, st);
    let bg = if st.bg != PmiBg::Padrao { st.bg } else { PmiBg::Nenhum };
    frame_box(o, lo, [lo[0] + w0, hi[1]], st, PmiFrame::Caixa, bg);
    o.text([lo[0] + w0 / 2., hi[1] - total_h / 2. - s * 0.6], sym.to_string(), s * 1.2, 0., Align::Center);
    for (i, row) in rows.iter().enumerate() {
        let top = hi[1] - h * i as f64;
        let x0 = lo[0] + w0;
        frame_box(o, [x0, top - h], [x0 + row_w[i], top], st, PmiFrame::Caixa, bg);
        let mut x = x0;
        for (k, c) in row.iter().enumerate() {
            let w = cw(c);
            if k > 0 {
                let col = o.color;
                o.color = st.frame_color.or(col);
                o.dline(vec![[x, top], [x, top - h]], false, Style::Thin, st.frame_dash);
                o.color = col;
            }
            o.text([x + w / 2., top - h / 2. - s / 2.], c.clone(), s, 0., Align::Center);
            x += w;
        }
    }
    let c = v2mul(v2add(lo, hi), 0.5);
    o.turn(start, c, st.angle as f64, st.mirror);
    (lo, hi)
}

fn note(o: &mut Out, n: &Note) {
    let s = n.size;
    let st = &n.style;
    match n.kind {
        NoteKind::Text => {
            let lines: Vec<String> = n.text.lines().map(|l| format!("{}{l}{}", st.prefix, st.suffix)).collect();
            let (lo, hi) = text_box(o, n.pos, &lines, s, st, PmiFrame::Nenhuma, PmiBg::Nenhum);
            if let Some(t) = n.leader {
                leader_box(o, lo, hi, t, s, st);
            }
        }
        NoteKind::Balloon if st.frame != PmiFrame::Padrao => {
            // balão com outra moldura (quadrado, losango…)
            let (lo, hi) = text_box(o, n.pos, &[n.text.clone()], s, st, PmiFrame::Circulo, PmiBg::Nenhum);
            if let Some(t) = n.leader {
                leader_box(o, lo, hi, t, s, st);
            }
        }
        NoteKind::Balloon => {
            let r = s * 1.6;
            let col = o.color;
            o.color = st.frame_color.or(col);
            o.dline(circle(n.pos, r), true, Style::Thin, st.frame_dash);
            o.color = col;
            o.text([n.pos[0], n.pos[1] - s / 2.], n.text.clone(), s, 0., Align::Center);
            if let Some(t) = n.leader {
                let u = v2unit(v2sub(t, n.pos));
                leader_line(o, vec![v2add(n.pos, v2mul(u, r)), t], s, st, false);
                if !st.hide_leader {
                    o.fill(circle(t, 0.6));
                }
            }
        }
        NoteKind::Roughness => {
            // √ com a barra: valor em cima
            let p = n.pos;
            let h = s * 1.4;
            o.line(vec![[p[0] - h * 0.6, p[1] + h * 0.6], p, [p[0] + h * 0.9, p[1] + h * 1.6], [p[0] + h * 0.9 + text_width(&n.text, s) + 1., p[1] + h * 1.6]], Style::Thin);
            o.seg([p[0] - h * 0.6, p[1] + h * 0.6], [p[0] + h * 0.35, p[1] + h * 0.6], Style::Thin);
            o.text([p[0] + h * 0.9 + 0.5, p[1] + h * 1.6 + 0.6], n.text.clone(), s, 0., Align::Left);
            if let Some(t) = n.leader {
                leader_line(o, vec![p, t], s, st, true);
            }
        }
        NoteKind::Weld => {
            // símbolo ISO 2553 (sistema A): linha de referência a partir da junção `pos`
            use fenkra_core::pmi::{weld_symbol, WeldExtra, WeldKind, WeldPrim};
            let p = n.pos;
            let prims = match &n.weld {
                Some(w) => weld_symbol(w.kind, &n.text, w.all_around, &w.extra, s),
                None => weld_symbol(WeldKind::Filete, &n.text, false, &WeldExtra::default(), s),
            };
            let m = |q: [f64; 2]| [p[0] + q[0], p[1] + q[1]];
            for pr in prims {
                match pr {
                    WeldPrim::Line(v) => o.line(v.into_iter().map(m).collect(), Style::Thin),
                    WeldPrim::Dashed(v) => o.line(v.into_iter().map(m).collect(), Style::Hidden),
                    WeldPrim::Fill(v) => o.fill(v.into_iter().map(m).collect()),
                    WeldPrim::Text(q, t, al) => o.text(m(q), t, s, 0., if al < 0 { Align::Right } else if al == 0 { Align::Center } else { Align::Left }),
                }
            }
            if let Some(t) = n.leader {
                leader_line(o, vec![p, t], s, st, true);
            }
        }
        NoteKind::CenterMark => {
            let r = if n.text.trim().is_empty() { 5. } else { n.text.trim().replace(',', ".").parse::<f64>().unwrap_or(5.) };
            let p = n.pos;
            o.seg([p[0] - 1.2, p[1]], [p[0] + 1.2, p[1]], Style::Thin);
            o.seg([p[0], p[1] - 1.2], [p[0], p[1] + 1.2], Style::Thin);
            o.seg([p[0] - r - 2., p[1]], [p[0] - 2., p[1]], Style::Center);
            o.seg([p[0] + 2., p[1]], [p[0] + r + 2., p[1]], Style::Center);
            o.seg([p[0], p[1] - r - 2.], [p[0], p[1] - 2.], Style::Center);
            o.seg([p[0], p[1] + 2.], [p[0], p[1] + r + 2.], Style::Center);
        }
        NoteKind::Gdt => {
            let g = n.gdt.clone().unwrap_or_default();
            let modifier = if g.extra.modifier.trim().is_empty() { String::new() } else { format!(" {}", g.extra.modifier.trim()) };
            let cell = |t: &str| format!("{}{}{modifier}", if g.extra.zone_dia { "Ø" } else { "" }, t.trim());
            let row = |t: &str, d: &str| -> Vec<String> {
                let mut v = vec![cell(t)];
                v.extend(d.split(|c: char| c == '|' || c == ',' || c.is_whitespace()).filter(|x| !x.is_empty()).map(String::from));
                v
            };
            let mut rows = vec![row(&g.tol, &g.datums)];
            if let Some((t2, d2)) = &g.extra.second {
                rows.push(row(t2, d2));
            }
            let (lo, hi) = fcf(o, n.pos, g.ch.symbol(), &rows, s, st);
            if let Some(t) = n.leader {
                leader_box(o, lo, hi, t, s, st);
            }
        }
        NoteKind::Datum => {
            let side = s * 2.;
            let c = [n.pos[0] + st.nudge[0] as f64, n.pos[1] - st.nudge[1] as f64];
            let (lo, hi) = ([c[0] - side / 2., c[1] - side / 2.], [c[0] + side / 2., c[1] + side / 2.]);
            if let Some(t) = n.leader {
                // triângulo cheio apoiado na peça e a haste até o lado mais perto do quadro
                let dir = v2unit(v2sub(c, t));
                let m = [-dir[1], dir[0]];
                let apex = v2add(t, v2mul(dir, s * 0.9));
                let col = o.color;
                o.color = st.leader_color.or(col);
                o.fill(vec![v2add(t, v2mul(m, s * 0.55)), apex, v2sub(t, v2mul(m, s * 0.55))]);
                o.color = col;
                let e = if (apex[0] - c[0]).abs() > (apex[1] - c[1]).abs() {
                    [if apex[0] < c[0] { lo[0] } else { hi[0] }, c[1]]
                } else {
                    [c[0], if apex[1] < c[1] { lo[1] } else { hi[1] }]
                };
                leader_line(o, vec![apex, e], s, st, false);
            }
            let bg = if st.bg != PmiBg::Padrao { st.bg } else { PmiBg::Nenhum };
            frame_box(o, lo, hi, st, PmiFrame::Caixa, bg);
            o.text([c[0], c[1] - s * 0.55], n.text.clone(), s * 1.1, 0., Align::Center);
        }
    }
}

/// Tabela com título e cabeçalho; `pos` = canto superior esquerdo. Devolve o tamanho.
fn table(o: &mut Out, pos: V2, title: &str, headers: &[String], rows: &[Vec<String>], s: f64) -> V2 {
    let ncol = headers.len().max(rows.iter().map(|r| r.len()).max().unwrap_or(0));
    if ncol == 0 {
        return [0., 0.];
    }
    let mut w = vec![0f64; ncol];
    for r in std::iter::once(headers).chain(rows.iter().map(|r| r.as_slice())) {
        for (i, c) in r.iter().enumerate() {
            w[i] = w[i].max(text_width(c, s) + 3.);
        }
    }
    let total: f64 = w.iter().sum::<f64>().max(text_width(title, s) + 4.);
    let k = total / w.iter().sum::<f64>();
    for x in &mut w {
        *x *= k;
    }
    let rh = s * 1.9;
    let nrow = rows.len() + 2;
    let h = rh * nrow as f64;
    o.rect([pos[0], pos[1] - h], [pos[0] + total, pos[1]], Style::Visible);
    for i in 1..nrow {
        let y = pos[1] - rh * i as f64;
        o.seg([pos[0], y], [pos[0] + total, y], if i <= 2 { Style::Visible } else { Style::Thin });
    }
    let mut x = pos[0];
    for (i, cw) in w.iter().enumerate() {
        if i > 0 {
            o.seg([x, pos[1] - rh], [x, pos[1] - h], Style::Thin);
        }
        x += cw;
    }
    o.text([pos[0] + total / 2., pos[1] - rh + rh * 0.3], title.to_string(), s, 0., Align::Center);
    for (ri, r) in std::iter::once(headers).chain(rows.iter().map(|r| r.as_slice())).enumerate() {
        let y = pos[1] - rh * (ri + 2) as f64 + rh * 0.3;
        let mut x = pos[0];
        for (i, c) in r.iter().enumerate() {
            o.text([x + w[i] / 2., y], c.clone(), s, 0., Align::Center);
            x += w[i];
        }
    }
    [total, h]
}

/// Retângulo (folha) de cada objeto, para seleção: une as primitivas do mesmo dono.
pub fn owner_boxes(items: &[Item]) -> Vec<(Id, V2, V2)> {
    let mut out: Vec<(Id, V2, V2)> = vec![];
    for it in items {
        let Some(id) = it.owner else { continue };
        let pts: Vec<V2> = match &it.prim {
            Prim::Line { pts, .. } | Prim::Fill { pts } => pts.clone(),
            Prim::Text { pos, text, size, align, .. } => {
                let w = text_width(text, *size);
                let x0 = match align {
                    Align::Left => pos[0],
                    Align::Center => pos[0] - w / 2.,
                    Align::Right => pos[0] - w,
                };
                vec![[x0, pos[1]], [x0 + w, pos[1] + size]]
            }
        };
        let e = match out.iter_mut().find(|e| e.0 == id) {
            Some(e) => e,
            None => {
                out.push((id, [f64::MAX; 2], [f64::MIN; 2]));
                out.last_mut().unwrap()
            }
        };
        for p in pts {
            e.1 = [e.1[0].min(p[0]), e.1[1].min(p[1])];
            e.2 = [e.2[0].max(p[0]), e.2[1].max(p[1])];
        }
    }
    out
}
