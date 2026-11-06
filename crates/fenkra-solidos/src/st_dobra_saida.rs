// SPDX-License-Identifier: LGPL-2.1-or-later
//! Saída dos desenhos de corte e gabarito de dobra de tubo (B329, B334): SVG, DXF (R12, ASCII) e
//! PDF vetorial em A4, tudo em milímetros e escala 1:1.
//!
//! O PDF divide o desenho em folhas A4 quando ele não cabe numa só, com marcas de canto e o número
//! de cada parte, para colar as folhas. O gerador é mínimo e próprio: só linhas, retângulos de
//! recorte e texto em fonte padrão do leitor.

use crate::st_dobra_corte::{Camada, Desenho};
use std::fmt::Write as _;

const MARGEM_SVG: f64 = 5.;

/// SVG em milímetros (tamanho real: `width` e `height` em mm, Y de cima para baixo).
pub fn svg(d: &Desenho) -> String {
    let (lo, hi) = d.caixa();
    let (w, h) = (hi[0] - lo[0] + 2. * MARGEM_SVG, hi[1] - lo[1] + 2. * MARGEM_SVG);
    let tx = |x: f64| x - lo[0] + MARGEM_SVG;
    let ty = |y: f64| hi[1] - y + MARGEM_SVG;
    let mut s = String::new();
    let _ = writeln!(s, r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w:.3}mm" height="{h:.3}mm" viewBox="0 0 {w:.3} {h:.3}">"#);
    let _ = writeln!(s, "<title>{}</title>", esc(&d.titulo));
    let _ = writeln!(s, r#"<rect width="{w:.3}" height="{h:.3}" fill="white"/>"#);
    for camada in [Camada::Contorno, Camada::Dobra, Camada::Marcacao, Camada::Corte, Camada::Cota] {
        let _ = writeln!(s, r#"<g id="{}" fill="none" stroke="{}" stroke-width="0.25" stroke-linecap="round" stroke-linejoin="round">"#, camada.nome(), camada.cor_svg());
        for t in d.tracos.iter().filter(|t| t.camada == camada) {
            let pts: Vec<String> = t.pts.iter().map(|p| format!("{:.3},{:.3}", tx(p[0]), ty(p[1]))).collect();
            let dash = if t.tracejado { r#" stroke-dasharray="2 1""# } else { "" };
            let tag = if t.fechado { "polygon" } else { "polyline" };
            let _ = writeln!(s, r#"<{tag} points="{}"{dash}/>"#, pts.join(" "));
        }
        for r in d.rotulos.iter().filter(|r| r.camada == camada) {
            let _ = writeln!(s, r#"<text x="{:.3}" y="{:.3}" font-family="Arial, Helvetica, sans-serif" font-size="{:.3}" fill="{}" stroke="none">{}</text>"#, tx(r.pos[0]), ty(r.pos[1]), r.altura, camada.cor_svg(), esc(&r.texto));
        }
        s.push_str("</g>\n");
    }
    s.push_str("</svg>\n");
    s
}

fn esc(t: &str) -> String {
    t.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// Texto para DXF R12: sem acentos (a fonte padrão do formato é ASCII) e com `%%d` no grau.
fn ascii(t: &str) -> String {
    t.chars()
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' => "a".to_string(),
            'Á' | 'À' | 'Â' | 'Ã' => "A".into(),
            'é' | 'è' | 'ê' => "e".into(),
            'É' | 'È' | 'Ê' => "E".into(),
            'í' | 'ì' => "i".into(),
            'Í' => "I".into(),
            'ó' | 'ò' | 'ô' | 'õ' => "o".into(),
            'Ó' | 'Ô' | 'Õ' => "O".into(),
            'ú' | 'ù' | 'ü' => "u".into(),
            'Ú' => "U".into(),
            'ç' => "c".into(),
            'Ç' => "C".into(),
            '°' => "%%d".into(),
            '·' | '–' | '—' | '•' => "-".into(),
            '×' => "x".into(),
            c if c.is_ascii() => c.to_string(),
            _ => "?".into(),
        })
        .collect()
}

/// DXF R12 (ASCII): uma camada por tipo, `LINE` por segmento e `TEXT` para os rótulos. Coordenadas
/// em milímetros, Y para cima, como o desenho.
pub fn dxf(d: &Desenho) -> String {
    let mut s = String::new();
    let par = |s: &mut String, c: i32, v: &str| {
        let _ = write!(s, "{c}\n{v}\n");
    };
    par(&mut s, 0, "SECTION");
    par(&mut s, 2, "HEADER");
    par(&mut s, 9, "$ACADVER");
    par(&mut s, 1, "AC1009");
    par(&mut s, 9, "$INSUNITS");
    par(&mut s, 70, "4");
    par(&mut s, 0, "ENDSEC");
    par(&mut s, 0, "SECTION");
    par(&mut s, 2, "TABLES");
    par(&mut s, 0, "TABLE");
    par(&mut s, 2, "LTYPE");
    par(&mut s, 70, "2");
    par(&mut s, 0, "LTYPE");
    par(&mut s, 2, "CONTINUOUS");
    par(&mut s, 70, "0");
    par(&mut s, 3, "Solid line");
    par(&mut s, 72, "65");
    par(&mut s, 73, "0");
    par(&mut s, 40, "0.0");
    par(&mut s, 0, "LTYPE");
    par(&mut s, 2, "DASHED");
    par(&mut s, 70, "0");
    par(&mut s, 3, "Dashed");
    par(&mut s, 72, "65");
    par(&mut s, 73, "2");
    par(&mut s, 40, "3.0");
    par(&mut s, 49, "2.0");
    par(&mut s, 49, "-1.0");
    par(&mut s, 0, "ENDTAB");
    par(&mut s, 0, "TABLE");
    par(&mut s, 2, "LAYER");
    par(&mut s, 70, "5");
    for c in [Camada::Corte, Camada::Marcacao, Camada::Dobra, Camada::Contorno, Camada::Cota] {
        par(&mut s, 0, "LAYER");
        par(&mut s, 2, c.nome());
        par(&mut s, 70, "0");
        par(&mut s, 62, &c.cor_dxf().to_string());
        par(&mut s, 6, "CONTINUOUS");
    }
    par(&mut s, 0, "ENDTAB");
    par(&mut s, 0, "ENDSEC");
    par(&mut s, 0, "SECTION");
    par(&mut s, 2, "ENTITIES");
    for t in &d.tracos {
        let n = t.pts.len();
        let segs = if t.fechado && n > 2 { n } else { n.saturating_sub(1) };
        for i in 0..segs {
            let (a, b) = (t.pts[i], t.pts[(i + 1) % n]);
            par(&mut s, 0, "LINE");
            par(&mut s, 8, t.camada.nome());
            if t.tracejado {
                par(&mut s, 6, "DASHED");
            }
            for (k, v) in [(10, a[0]), (20, a[1]), (30, 0.), (11, b[0]), (21, b[1]), (31, 0.)] {
                par(&mut s, k, &format!("{v:.4}"));
            }
        }
    }
    for r in &d.rotulos {
        par(&mut s, 0, "TEXT");
        par(&mut s, 8, r.camada.nome());
        for (k, v) in [(10, r.pos[0]), (20, r.pos[1]), (30, 0.), (40, r.altura)] {
            par(&mut s, k, &format!("{v:.4}"));
        }
        par(&mut s, 1, &ascii(&r.texto));
    }
    par(&mut s, 0, "ENDSEC");
    par(&mut s, 0, "EOF");
    s
}

// ------------------------------------------------------------------------- PDF

const A4: [f64; 2] = [210., 297.];
const MARGEM_PDF: f64 = 10.;
const RODAPE: f64 = 8.;
const MM_PT: f64 = 72. / 25.4;

fn cor_pdf(c: Camada) -> &'static str {
    match c {
        Camada::Corte => "0.83 0 0",
        Camada::Marcacao => "0 0.31 0.78",
        Camada::Dobra => "0 0.55 0.24",
        Camada::Contorno => "0.5 0.5 0.5",
        Camada::Cota => "0 0 0",
    }
}

fn texto_pdf(t: &str) -> Vec<u8> {
    let mut out = vec![b'('];
    for c in t.chars() {
        match c {
            '(' | ')' | '\\' => {
                out.push(b'\\');
                out.push(c as u8);
            }
            '·' => out.push(0xB7),
            '–' => out.push(0x96),
            '—' => out.push(0x97),
            c if (c as u32) < 256 => out.push(c as u8),
            _ => out.push(b'?'),
        }
    }
    out.push(b')');
    out
}

/// Quantas folhas A4 o desenho ocupa: (colunas, linhas, retrato?).
pub fn folhas(d: &Desenho) -> (usize, usize, bool) {
    let (lo, hi) = d.caixa();
    let (w, h) = (hi[0] - lo[0], hi[1] - lo[1]);
    let util = |retrato: bool| {
        let p = if retrato { A4 } else { [A4[1], A4[0]] };
        [p[0] - 2. * MARGEM_PDF, p[1] - 2. * MARGEM_PDF - RODAPE]
    };
    let conta = |retrato: bool| {
        let u = util(retrato);
        ((w / u[0]).ceil().max(1.) as usize, (h / u[1]).ceil().max(1.) as usize)
    };
    let (cr, lr) = conta(true);
    let (cp, lp) = conta(false);
    if cr * lr <= cp * lp {
        (cr, lr, true)
    } else {
        (cp, lp, false)
    }
}

/// PDF em A4 vetorial, escala 1:1, com as folhas necessárias. Cada folha leva marcas de canto e
/// "parte i de n".
pub fn pdf(d: &Desenho) -> Vec<u8> {
    let (lo, hi) = d.caixa();
    let (cols, linhas, retrato) = folhas(d);
    let pag = if retrato { A4 } else { [A4[1], A4[0]] };
    let util = [pag[0] - 2. * MARGEM_PDF, pag[1] - 2. * MARGEM_PDF - RODAPE];
    let total = cols * linhas;
    let mut objs: Vec<Vec<u8>> = vec![];
    objs.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    objs.push(vec![]);
    objs.push(b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".to_vec());
    let mut kids = vec![];
    for lin in 0..linhas {
        for col in 0..cols {
            let n = lin * cols + col + 1;
            // região do desenho nesta folha (a linha 0 é a de cima)
            let x0 = lo[0] + col as f64 * util[0];
            let y1 = hi[1] - lin as f64 * util[1];
            let y0 = y1 - util[1];
            // origem da folha: canto inferior esquerdo da área útil
            let (ox, oy) = (MARGEM_PDF, MARGEM_PDF + RODAPE);
            let mut c: Vec<u8> = vec![];
            let w = |c: &mut Vec<u8>, t: &str| c.extend_from_slice(t.as_bytes());
            w(&mut c, &format!("{MM_PT:.6} 0 0 {MM_PT:.6} 0 0 cm 1 J 1 j 0.25 w\n"));
            // marcas de canto da área útil
            w(&mut c, "0 0 0 RG 0.2 w\n");
            for (cx, cy, sx, sy) in [(ox, oy, 1., 1.), (ox + util[0], oy, -1., 1.), (ox, oy + util[1], 1., -1.), (ox + util[0], oy + util[1], -1., -1.)] {
                w(&mut c, &format!("{:.3} {:.3} m {:.3} {:.3} l S {:.3} {:.3} m {:.3} {:.3} l S\n", cx, cy, cx + sx * 4., cy, cx, cy, cx, cy + sy * 4.));
            }
            // recorte à área útil
            w(&mut c, &format!("q {ox:.3} {oy:.3} {:.3} {:.3} re W n\n", util[0], util[1]));
            let (dx, dy) = (ox - x0, oy - y0);
            for t in &d.tracos {
                w(&mut c, &format!("{} RG {} w [{}] 0 d ", cor_pdf(t.camada), if t.camada == Camada::Corte { "0.35" } else { "0.25" }, if t.tracejado { "2 1" } else { "" }));
                for (i, p) in t.pts.iter().enumerate() {
                    w(&mut c, &format!("{:.3} {:.3} {} ", p[0] + dx, p[1] + dy, if i == 0 { "m" } else { "l" }));
                }
                w(&mut c, if t.fechado { "h S\n" } else { "S\n" });
            }
            w(&mut c, "[] 0 d\n");
            for r in &d.rotulos {
                w(&mut c, &format!("{} rg BT /F1 {:.3} Tf {:.3} {:.3} Td ", cor_pdf(r.camada), r.altura / 0.72, r.pos[0] + dx, r.pos[1] + dy));
                c.extend(texto_pdf(&r.texto));
                w(&mut c, " Tj ET\n");
            }
            w(&mut c, "Q\n");
            // rodapé
            w(&mut c, &format!("0 g BT /F1 3 Tf {ox:.3} {:.3} Td ", MARGEM_PDF + 1.));
            c.extend(texto_pdf(&format!("{} - parte {n} de {total} (coluna {} de {cols}, linha {} de {linhas}) - imprimir a 100%", d.titulo, col + 1, lin + 1)));
            w(&mut c, " Tj ET\n");
            let page_id = objs.len() + 1;
            let content_id = objs.len() + 2;
            kids.push(page_id);
            objs.push(format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {:.2} {:.2}] /Resources << /Font << /F1 3 0 R >> >> /Contents {content_id} 0 R >>", pag[0] * MM_PT, pag[1] * MM_PT).into_bytes());
            let mut st = format!("<< /Length {} >>\nstream\n", c.len()).into_bytes();
            st.extend(c);
            st.extend_from_slice(b"\nendstream");
            objs.push(st);
        }
    }
    objs[1] = format!("<< /Type /Pages /Kids [{}] /Count {} >>", kids.iter().map(|i| format!("{i} 0 R")).collect::<Vec<_>>().join(" "), kids.len()).into_bytes();
    let mut out = b"%PDF-1.4\n".to_vec();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::st_dobra_corte::Desenho;

    fn exemplo(largura: f64) -> Desenho {
        let mut d = Desenho { titulo: "Teste".into(), ..Default::default() };
        d.retangulo(Camada::Corte, 0., 0., largura, 40.);
        d.linha(Camada::Dobra, [0., 20.], [largura, 20.], true);
        d.texto(Camada::Cota, [2., 2.], 3., "Tubo 30x1,2 · 90° ação");
        d
    }

    #[test]
    fn svg_tem_tamanho_real_em_mm() {
        let s = svg(&exemplo(100.));
        // 100 mm de desenho + 2 margens de 5 mm
        assert!(s.contains(r#"width="110.000mm""#), "{s}");
        assert!(s.contains(r#"id="CORTE""#) && s.contains("<polygon"));
        assert!(s.contains("stroke-dasharray"));
    }

    #[test]
    fn dxf_tem_camadas_linhas_e_texto() {
        let s = dxf(&exemplo(100.));
        for nome in ["CORTE", "MARCACAO", "DOBRA", "CONTORNO", "COTA"] {
            assert!(s.contains(&format!("\n2\n{nome}\n")), "camada {nome}");
        }
        // retângulo fechado = 4 linhas, mais a linha de dobra
        assert_eq!(s.matches("\nLINE\n").count(), 5);
        assert!(s.contains("TEXT") && s.contains("%%d") && s.contains("acao"));
        assert!(s.ends_with("0\nEOF\n"));
        // as coordenadas saem sem arredondar além de 0,0001 mm
        assert!(s.contains("100.0000"));
    }

    #[test]
    fn pdf_cabe_numa_folha_ou_divide_em_varias() {
        let curto = pdf(&exemplo(100.));
        let txt = String::from_utf8_lossy(&curto);
        assert!(txt.starts_with("%PDF-1.4") && txt.trim_end().ends_with("%%EOF"));
        assert!(txt.contains("/Count 1"));
        // A4 em pontos: 595,28 x 841,89
        assert!(txt.contains("/MediaBox [0 0 595.28 841.89]"), "{}", &txt[..300.min(txt.len())]);
        // 500 mm de largura não cabem em 190 mm de área útil: várias folhas
        let longo = pdf(&exemplo(500.));
        let t2 = String::from_utf8_lossy(&longo);
        assert!(!t2.contains("/Count 1 "));
        let (c, l, _) = folhas(&exemplo(500.));
        assert!(c * l >= 2);
        assert!(t2.contains(&format!("/Count {}", c * l)));
        // o xref aponta para os objetos
        let pos: usize = t2.rsplit("startxref\n").next().unwrap().lines().next().unwrap().parse().unwrap();
        assert!(longo[pos..].starts_with(b"xref"));
    }
}
