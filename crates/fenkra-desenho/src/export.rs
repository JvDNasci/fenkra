//! Exportação das folhas: PDF (uma página por folha, vetorial), SVG e DXF (R12, uma folha).

use crate::layout::{Align, Item, Prim, Style};
use crate::*;
use std::fmt::Write as _;

/// SVG de uma folha (mm; y da folha para cima → y do SVG para baixo).
pub fn svg(items: &[Item], size: V2) -> String {
    let [w, h] = size;
    let mut s = String::new();
    let _ = writeln!(s, r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}mm" height="{h}mm" viewBox="0 0 {w} {h}">"#);
    let _ = writeln!(s, r#"<rect width="{w}" height="{h}" fill="white"/>"#);
    let fy = |y: f64| h - y;
    for it in items {
        let col = it.color.map(|c| format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2])).unwrap_or_else(|| "black".into());
        match &it.prim {
            Prim::Line { pts, style } => {
                let d: Vec<String> = pts.iter().map(|p| format!("{:.3},{:.3}", p[0], fy(p[1]))).collect();
                let dash = style.dash();
                let da = if dash.is_empty() { String::new() } else { format!(r#" stroke-dasharray="{}""#, dash.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(" ")) };
                let _ = writeln!(s, r#"<polyline points="{}" fill="none" stroke="{col}" stroke-width="{}" stroke-linecap="round" stroke-linejoin="round"{da}/>"#, d.join(" "), it.width.unwrap_or(style.width()));
            }
            Prim::Fill { pts } => {
                let d: Vec<String> = pts.iter().map(|p| format!("{:.3},{:.3}", p[0], fy(p[1]))).collect();
                let _ = writeln!(s, r#"<polygon points="{}" fill="{col}"/>"#, d.join(" "));
            }
            Prim::Text { pos, text, size, angle, align } => {
                let anchor = match align {
                    Align::Left => "start",
                    Align::Center => "middle",
                    Align::Right => "end",
                };
                let t = text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
                let _ = writeln!(
                    s,
                    r#"<text x="{:.3}" y="{:.3}" font-family="Arial, Helvetica, sans-serif" font-size="{size}" fill="{col}"{} text-anchor="{anchor}" transform="rotate({:.3} {:.3} {:.3})">{t}</text>"#,
                    pos[0],
                    fy(pos[1]),
                    if it.bold { r#" font-weight="bold""# } else { "" },
                    -angle,
                    pos[0],
                    fy(pos[1])
                );
            }
        }
    }
    s.push_str("</svg>\n");
    s
}

/// DXF R12 (ASCII) de uma folha: camadas por tipo de linha, textos com estilo padrão.
pub fn dxf(items: &[Item]) -> String {
    let mut s = String::new();
    let pair = |s: &mut String, c: i32, v: &str| {
        let _ = write!(s, "{c}\n{v}\n");
    };
    // tabelas de tipos de linha e camadas
    pair(&mut s, 0, "SECTION");
    pair(&mut s, 2, "TABLES");
    pair(&mut s, 0, "TABLE");
    pair(&mut s, 2, "LTYPE");
    pair(&mut s, 70, "3");
    for (name, pat) in [("CONTINUOUS", vec![]), ("HIDDEN", vec![3., -1.5]), ("CENTER", vec![8., -1.5, 1., -1.5])] {
        pair(&mut s, 0, "LTYPE");
        pair(&mut s, 2, name);
        pair(&mut s, 70, "0");
        pair(&mut s, 3, name);
        pair(&mut s, 72, "65");
        pair(&mut s, 73, &pat.len().to_string());
        pair(&mut s, 40, &format!("{}", pat.iter().map(|x: &f64| x.abs()).sum::<f64>()));
        for x in pat {
            pair(&mut s, 49, &x.to_string());
        }
    }
    pair(&mut s, 0, "ENDTAB");
    pair(&mut s, 0, "TABLE");
    pair(&mut s, 2, "LAYER");
    pair(&mut s, 70, "5");
    for (name, color, lt) in [("VISIVEL", "7", "CONTINUOUS"), ("FINA", "3", "CONTINUOUS"), ("OCULTA", "4", "HIDDEN"), ("CENTRO", "1", "CENTER"), ("MOLDURA", "7", "CONTINUOUS")] {
        pair(&mut s, 0, "LAYER");
        pair(&mut s, 2, name);
        pair(&mut s, 70, "0");
        pair(&mut s, 62, color);
        pair(&mut s, 6, lt);
    }
    pair(&mut s, 0, "ENDTAB");
    pair(&mut s, 0, "ENDSEC");
    pair(&mut s, 0, "SECTION");
    pair(&mut s, 2, "ENTITIES");
    let layer = |st: Style| match st {
        Style::Visible => "VISIVEL",
        Style::Thin => "FINA",
        Style::Hidden => "OCULTA",
        Style::Center => "CENTRO",
        Style::Thick => "MOLDURA",
    };
    for it in items {
        match &it.prim {
            Prim::Line { pts, style } => {
                for w in pts.windows(2) {
                    pair(&mut s, 0, "LINE");
                    pair(&mut s, 8, layer(*style));
                    pair(&mut s, 10, &format!("{:.4}", w[0][0]));
                    pair(&mut s, 20, &format!("{:.4}", w[0][1]));
                    pair(&mut s, 30, "0");
                    pair(&mut s, 11, &format!("{:.4}", w[1][0]));
                    pair(&mut s, 21, &format!("{:.4}", w[1][1]));
                    pair(&mut s, 31, "0");
                }
            }
            Prim::Fill { pts } => {
                // seta: SOLID (3 vértices; o 4º repete o 3º)
                if pts.len() >= 3 {
                    pair(&mut s, 0, "SOLID");
                    pair(&mut s, 8, "FINA");
                    let q = [pts[0], pts[1], pts[2], pts[2]];
                    for (i, p) in q.iter().enumerate() {
                        pair(&mut s, 10 + i as i32, &format!("{:.4}", p[0]));
                        pair(&mut s, 20 + i as i32, &format!("{:.4}", p[1]));
                        pair(&mut s, 30 + i as i32, "0");
                    }
                }
            }
            Prim::Text { pos, text, size, angle, align } => {
                pair(&mut s, 0, "TEXT");
                pair(&mut s, 8, "FINA");
                pair(&mut s, 10, &format!("{:.4}", pos[0]));
                pair(&mut s, 20, &format!("{:.4}", pos[1]));
                pair(&mut s, 30, "0");
                pair(&mut s, 40, &format!("{size}"));
                pair(&mut s, 1, &dxf_text(text));
                pair(&mut s, 50, &format!("{angle:.3}"));
                let h = match align {
                    Align::Left => 0,
                    Align::Center => 1,
                    Align::Right => 2,
                };
                if h != 0 {
                    pair(&mut s, 72, &h.to_string());
                    pair(&mut s, 11, &format!("{:.4}", pos[0]));
                    pair(&mut s, 21, &format!("{:.4}", pos[1]));
                    pair(&mut s, 31, "0");
                }
            }
        }
    }
    pair(&mut s, 0, "ENDSEC");
    pair(&mut s, 0, "EOF");
    s
}

/// Caracteres especiais do DXF (%%c diâmetro, %%d grau, %%p mais-menos).
fn dxf_text(t: &str) -> String {
    t.replace('Ø', "%%c").replace('°', "%%d").replace('±', "%%p")
}

/// Texto em WinAnsi (Latin-1 nos acentos) para a fonte padrão do PDF; o resto vira '?'.
fn pdf_str(t: &str) -> Vec<u8> {
    let mut out = vec![b'('];
    for c in t.chars() {
        let b = match c {
            '(' | ')' | '\\' => {
                out.push(b'\\');
                c as u8
            }
            c if (c as u32) < 256 => c as u8,
            '•' => 0x95,
            '–' => 0x96,
            '—' => 0x97,
            _ => b'?',
        };
        out.push(b);
    }
    out.push(b')');
    out
}

/// PDF vetorial com uma página por folha (mm → pt).
pub fn pdf(pages: &[(V2, Vec<Item>)]) -> Vec<u8> {
    let k = 72. / 25.4;
    let mut objs: Vec<Vec<u8>> = vec![];
    // 1 catálogo, 2 páginas, 3 e 4 fontes; depois cada página e seu conteúdo
    objs.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    objs.push(vec![]);
    objs.push(b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".to_vec());
    objs.push(b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold /Encoding /WinAnsiEncoding >>".to_vec());
    let mut kids = vec![];
    for (size, items) in pages {
        let mut c: Vec<u8> = vec![];
        let _ = write!(Bytes(&mut c), "{k:.6} 0 0 {k:.6} 0 0 cm 1 J 1 j\n");
        for it in items {
            let [r, g, b] = it.color.unwrap_or([0, 0, 0]).map(|x| x as f64 / 255.);
            let _ = write!(Bytes(&mut c), "{r:.3} {g:.3} {b:.3} RG {r:.3} {g:.3} {b:.3} rg ");
            match &it.prim {
                Prim::Line { pts, style } => {
                    let dash = style.dash();
                    let _ = write!(Bytes(&mut c), "{} w [{}] 0 d ", it.width.unwrap_or(style.width()), dash.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(" "));
                    for (i, p) in pts.iter().enumerate() {
                        let _ = write!(Bytes(&mut c), "{:.3} {:.3} {} ", p[0], p[1], if i == 0 { "m" } else { "l" });
                    }
                    c.extend_from_slice(b"S\n");
                }
                Prim::Fill { pts } => {
                    for (i, p) in pts.iter().enumerate() {
                        let _ = write!(Bytes(&mut c), "{:.3} {:.3} {} ", p[0], p[1], if i == 0 { "m" } else { "l" });
                    }
                    c.extend_from_slice(b"h f\n");
                }
                Prim::Text { pos, text, size, angle, align } => {
                    // Helvetica: largura média ~0,55 da altura (só para centralizar/alinhar)
                    let w = crate::layout::text_width(text, *size) * 1.1;
                    let dx = match align {
                        Align::Left => 0.,
                        Align::Center => -w / 2.,
                        Align::Right => -w,
                    };
                    let (s, co) = angle.to_radians().sin_cos();
                    let x = pos[0] + dx * co;
                    let y = pos[1] + dx * s;
                    // fonte em pt dentro do espaço em mm: tamanho em mm / 0,72 (altura de maiúscula)
                    let fs = size / 0.72;
                    let f = if it.bold { "F2" } else { "F1" };
                    let _ = write!(Bytes(&mut c), "BT /{f} {fs:.3} Tf {co:.6} {s:.6} {:.6} {co:.6} {x:.3} {y:.3} Tm ", -s);
                    c.extend(pdf_str(text));
                    c.extend_from_slice(b" Tj ET\n");
                }
            }
        }
        let content_id = objs.len() + 2;
        let page_id = objs.len() + 1;
        kids.push(page_id);
        objs.push(format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {:.2} {:.2}] /Resources << /Font << /F1 3 0 R /F2 4 0 R >> >> /Contents {content_id} 0 R >>", size[0] * k, size[1] * k).into_bytes());
        let mut st = format!("<< /Length {} >>\nstream\n", c.len()).into_bytes();
        st.extend(c);
        st.extend_from_slice(b"\nendstream");
        objs.push(st);
    }
    objs[1] = format!("<< /Type /Pages /Kids [{}] /Count {} >>", kids.iter().map(|i| format!("{i} 0 R")).collect::<Vec<_>>().join(" "), kids.len()).into_bytes();
    let mut out = b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n".to_vec();
    let mut offs = vec![];
    for (i, o) in objs.iter().enumerate() {
        offs.push(out.len());
        out.extend(format!("{} 0 obj\n", i + 1).into_bytes());
        out.extend(o);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend(format!("xref\n0 {}\n0000000000 65535 f \n", objs.len() + 1).into_bytes());
    for o in offs {
        out.extend(format!("{o:010} 00000 n \n").into_bytes());
    }
    out.extend(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n", objs.len() + 1).into_bytes());
    out
}

struct Bytes<'a>(&'a mut Vec<u8>);

impl std::fmt::Write for Bytes<'_> {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.0.extend_from_slice(s.as_bytes());
        Ok(())
    }
}

/// Exporta pelo sufixo: .pdf (todas as folhas), .svg ou .dxf (a folha dada).
pub fn save(d: &Drawing, sheet: Id, m: &mut crate::source::Models, path: &str) -> Result<(), String> {
    let lower = path.to_lowercase();
    let r = if lower.ends_with(".pdf") {
        let pages: Vec<(V2, Vec<Item>)> = d.sheets.iter().map(|s| (s.size(), crate::layout::sheet(d, s.id, m))).collect();
        std::fs::write(path, pdf(&pages))
    } else {
        let sh = d.sheet(sheet).ok_or("folha não encontrada")?;
        let items = crate::layout::sheet(d, sheet, m);
        if lower.ends_with(".svg") {
            std::fs::write(path, svg(&items, sh.size()))
        } else if lower.ends_with(".dxf") {
            std::fs::write(path, dxf(&items))
        } else {
            return Err("formato não suportado (use .pdf, .svg ou .dxf)".into());
        }
    };
    r.map_err(|e| e.to_string())
}
