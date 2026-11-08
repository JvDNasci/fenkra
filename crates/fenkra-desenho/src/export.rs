// SPDX-License-Identifier: LGPL-2.1-or-later
//! Exportação das folhas: PDF (uma página por folha, vetorial), SVG e DXF (R2000, uma folha).

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

/// DXF R2000 de uma folha: camadas por tipo de linha, curvas amostradas de volta a `ARC` e `CIRCLE`,
/// setas e sombreado como `HATCH` e textos como `MTEXT`.
pub fn dxf(items: &[Item]) -> String {
    use fenkra_dxf::{cor_indexada, reconhecer, Anexo, Camada, Desenho, Entidade, TipoLinha, Unidade, TOLERANCIA_ARCO};
    let camada = |st: Style| match st {
        Style::Visible => "VISIVEL",
        Style::Thin => "FINA",
        Style::Hidden => "OCULTA",
        Style::Center => "CENTRO",
        Style::Thick => "MOLDURA",
    };
    let mut d = Desenho {
        unidade: Unidade::Milimetro,
        tipos_linha: vec![TipoLinha { nome: "HIDDEN".into(), padrao: vec![3., -1.5] }, TipoLinha { nome: "CENTER".into(), padrao: vec![8., -1.5, 1., -1.5] }],
        camadas: vec![
            Camada { espessura: Some(50), ..Camada::nova("VISIVEL", 7, "Continuous") },
            Camada { espessura: Some(25), ..Camada::nova("FINA", 3, "Continuous") },
            Camada { espessura: Some(25), ..Camada::nova("OCULTA", 4, "HIDDEN") },
            Camada { espessura: Some(25), ..Camada::nova("CENTRO", 1, "CENTER") },
            Camada { espessura: Some(70), ..Camada::nova("MOLDURA", 7, "Continuous") },
        ],
        ..Default::default()
    };
    for it in items {
        let mut por = |nome: &str, e: Entidade| {
            let mut item = fenkra_dxf::Item::novo(nome, e);
            item.cor = it.color.map(cor_indexada);
            item.espessura = it.width.map(|w| (w * 100.).round() as i16);
            d.itens.push(item);
        };
        match &it.prim {
            Prim::Line { pts, style } => {
                for e in reconhecer(pts, TOLERANCIA_ARCO) {
                    por(camada(*style), e);
                }
            }
            Prim::Fill { pts } if pts.len() >= 3 => por("FINA", Entidade::Hachura { lacos: vec![pts.clone()] }),
            Prim::Fill { .. } => {}
            Prim::Text { pos, text, size, angle, align } => {
                let anexo = match align {
                    Align::Left => Anexo::Esquerda,
                    Align::Center => Anexo::Centro,
                    Align::Right => Anexo::Direita,
                };
                por("FINA", Entidade::Texto { pos: *pos, altura: *size, angulo: *angle, anexo, texto: text.clone() });
            }
        }
    }
    fenkra_dxf::gravar(&d)
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
    // B393: vista de montagem ainda calculando (ou só com a prévia por malha) não vai para o arquivo
    let mut pendente = false;
    for v in &d.views {
        let p = m.proj(d, v.id);
        pendente |= p.calculando || p.previa;
    }
    if pendente {
        return Err("há vistas ainda em cálculo: espere a barra de progresso terminar e exporte de novo".into());
    }
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
