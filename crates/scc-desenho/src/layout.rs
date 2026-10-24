//! Folha → primitivas (mm, origem no canto inferior esquerdo, y para cima). A tela e as
//! exportações (PDF, SVG, DXF) desenham a mesma lista.

use crate::source::{circle, Models, Proj};
use crate::*;

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
}

/// Largura aproximada do texto (fonte sem serifa).
pub fn text_width(t: &str, size: f64) -> f64 {
    t.chars().count() as f64 * size * 0.62
}

struct Out {
    items: Vec<Item>,
    owner: Option<Id>,
}

impl Out {
    fn line(&mut self, pts: Vec<V2>, style: Style) {
        if pts.len() >= 2 {
            self.items.push(Item { owner: self.owner, prim: Prim::Line { pts, style } });
        }
    }
    fn seg(&mut self, a: V2, b: V2, style: Style) {
        self.line(vec![a, b], style);
    }
    fn rect(&mut self, lo: V2, hi: V2, style: Style) {
        self.line(vec![lo, [hi[0], lo[1]], hi, [lo[0], hi[1]], lo], style);
    }
    fn text(&mut self, pos: V2, text: impl Into<String>, size: f64, angle: f64, align: Align) {
        let text = text.into();
        if !text.is_empty() {
            self.items.push(Item { owner: self.owner, prim: Prim::Text { pos, text, size, angle, align } });
        }
    }
    /// seta com a ponta em `tip` apontando na direção `dir`
    fn arrow(&mut self, tip: V2, dir: V2, size: f64) {
        let l = (dir[0] * dir[0] + dir[1] * dir[1]).sqrt().max(1e-12);
        let d = [dir[0] / l, dir[1] / l];
        let n = [-d[1], d[0]];
        let b = [tip[0] - d[0] * size, tip[1] - d[1] * size];
        let w = size * 0.18;
        self.items.push(Item { owner: self.owner, prim: Prim::Fill { pts: vec![tip, [b[0] + n[0] * w, b[1] + n[1] * w], [b[0] - n[0] * w, b[1] - n[1] * w]] } });
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
    let mut o = Out { items: vec![], owner: None };
    let Some(sh) = d.sheet(sheet) else { return vec![] };
    let fields = sheet_fields(d, sh, m);
    frame(&mut o, d, sh, &fields);
    let th = d.text_height;
    let views: Vec<&View> = d.views.iter().filter(|v| v.sheet == sheet).collect();
    for v in &views {
        let p = m.proj(d, v.id);
        o.owner = Some(v.id);
        for (pl, k) in &p.lines {
            let style = match k {
                3 => Style::Hidden,
                4 => Style::Thin,
                _ => Style::Visible,
            };
            o.line(pl.iter().map(|q| to_sheet(v, &p, *q)).collect(), style);
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
        o.owner = Some(dm.id);
        dimension(&mut o, dm, &pts, &dim_text(d, dm, val), th);
    }
    // notas e símbolos
    for n in d.notes.iter().filter(|n| n.sheet == sheet) {
        o.owner = Some(n.id);
        // notas também aceitam campos automáticos ({codigo}, {material}…)
        let mut n2 = n.clone();
        n2.text = crate::resolve(&n.text, &fields);
        note(&mut o, &n2);
    }
    // tabelas
    for t in d.tables.iter().filter(|t| t.sheet == sheet) {
        o.owner = Some(t.id);
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

fn dimension(o: &mut Out, dm: &Dim, pts: &[V2], text: &str, th: f64) {
    let a = th * 0.9;
    let gap = 1.;
    let over = 1.5;
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
            let dot = |a: V2, b: V2| a[0] * b[0] + a[1] * b[1];
            // linha de cota a `offset` do ponto mais afastado no sentido da normal
            let base = dot(p1, n).max(dot(p2, n));
            let base = if dm.offset >= 0. { base + dm.offset } else { dot(p1, n).min(dot(p2, n)) + dm.offset };
            let q1 = v2add(p1, v2mul(n, base - dot(p1, n)));
            let q2 = v2add(p2, v2mul(n, base - dot(p2, n)));
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
            if v2len(d) > 2. * a + 0.5 {
                o.arrow(q1, v2mul(d, -1.), a);
                o.arrow(q2, d, a);
            } else {
                // cota pequena: setas por fora
                o.arrow(q1, d, a);
                o.arrow(q2, v2mul(d, -1.), a);
                o.seg(q2, v2add(q2, v2mul(v2unit(d), 2. * a)), Style::Thin);
            }
            let mid = v2mul(v2add(q1, q2), 0.5);
            // texto paralelo à cota, legível (de baixo ou da direita)
            let mut ang = u[1].atan2(u[0]).to_degrees();
            let mut nn = n;
            if ang > 90.01 || ang < -89.99 {
                ang -= 180. * ang.signum();
                nn = v2mul(n, -1.);
            }
            let up = if dot(nn, n) * sg > 0. { 0.8 } else { -(th + 0.8) };
            let rn = [-(ang.to_radians().sin()), ang.to_radians().cos()];
            o.text(v2add(mid, v2mul(rn, up)), text.to_string(), th, ang, Align::Center);
        }
        DimKind::Radius | DimKind::Diameter => {
            let (c, p) = (pts[0], pts[1]);
            let u = v2unit(v2sub(p, c));
            let r = v2len(v2sub(p, c));
            let out_len = dm.offset.max(4.);
            let tail = v2add(p, v2mul(u, out_len));
            if dm.kind == DimKind::Diameter {
                let p0 = v2sub(c, v2mul(u, r));
                o.seg(p0, tail, Style::Thin);
                o.arrow(p0, v2mul(u, -1.), a);
            } else {
                o.seg(c, tail, Style::Thin);
            }
            o.arrow(p, u, a);
            let shelf = v2add(tail, [if u[0] >= 0. { text_width(text, th) + 1. } else { -(text_width(text, th) + 1.) }, 0.]);
            o.seg(tail, shelf, Style::Thin);
            let al = if u[0] >= 0. { Align::Left } else { Align::Right };
            o.text([tail[0] + if u[0] >= 0. { 0.5 } else { -0.5 }, tail[1] + 0.8], text.to_string(), th, 0., al);
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
            let arc: Vec<V2> = (0..=32).map(|i| {
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
            o.arrow(arc[0], [s * a1.sin(), -s * a1.cos()], a);
            o.arrow(arc[32], [-s * a2.sin(), s * a2.cos()], a);
            o.line(arc, Style::Thin);
            let am = (a1 + a2) / 2.;
            let tp = [c[0] + (r + th) * am.cos(), c[1] + (r + th) * am.sin() - th / 2.];
            o.text(tp, text.to_string(), th, 0., Align::Center);
        }
    }
}

fn note(o: &mut Out, n: &Note) {
    let s = n.size;
    match n.kind {
        NoteKind::Text => {
            let lines: Vec<&str> = n.text.lines().collect();
            for (i, l) in lines.iter().enumerate() {
                o.text([n.pos[0], n.pos[1] - i as f64 * s * 1.5], l.to_string(), s, 0., Align::Left);
            }
            if let Some(t) = n.leader {
                let w = lines.iter().map(|l| text_width(l, s)).fold(0., f64::max);
                let start = if t[0] > n.pos[0] + w / 2. { [n.pos[0] + w + 1., n.pos[1] + s / 2.] } else { [n.pos[0] - 1., n.pos[1] + s / 2.] };
                o.seg(start, t, Style::Thin);
                o.arrow(t, v2sub(t, start), s * 0.9);
            }
        }
        NoteKind::Balloon => {
            let r = s * 1.6;
            o.line(circle(n.pos, r), Style::Thin);
            o.text([n.pos[0], n.pos[1] - s / 2.], n.text.clone(), s, 0., Align::Center);
            if let Some(t) = n.leader {
                let u = v2unit(v2sub(t, n.pos));
                o.seg(v2add(n.pos, v2mul(u, r)), t, Style::Thin);
                o.items.push(Item { owner: o.owner, prim: Prim::Fill { pts: circle(t, 0.6) } });
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
                o.seg(p, t, Style::Thin);
                o.arrow(t, v2sub(t, p), s * 0.9);
            }
        }
        NoteKind::Weld => {
            // símbolo ISO 2553 (sistema A): linha de referência a partir da junção `pos`
            use scc_core::pmi::{weld_symbol, WeldExtra, WeldKind, WeldPrim};
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
                    WeldPrim::Fill(v) => o.items.push(Item { owner: o.owner, prim: Prim::Fill { pts: v.into_iter().map(m).collect() } }),
                    WeldPrim::Text(q, t, al) => o.text(m(q), t, s, 0., if al < 0 { Align::Right } else if al == 0 { Align::Center } else { Align::Left }),
                }
            }
            if let Some(t) = n.leader {
                o.seg(p, t, Style::Thin);
                o.arrow(t, v2sub(t, p), s * 0.9);
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
