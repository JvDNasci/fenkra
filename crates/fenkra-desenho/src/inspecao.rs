// SPDX-License-Identifier: LGPL-2.1-or-later
//! B117: plano de inspeção e relatório de primeiro artigo (FAI).
//!
//! Cada requisito que se mede na peça vira uma característica numerada: as cotas (menos as de
//! referência, entre parênteses), os quadros de tolerância geométrica e as rugosidades. A numeração
//! segue a leitura da folha: folha por folha, zona por zona (linha de cima para baixo, coluna da
//! esquerda para a direita), como o balão na folha e a linha da tabela mostram.
//!
//! Os limites saem da tolerância da cota: ± ou +/− direto; classe ISO 286 ("H7", "g6") pelos
//! desvios da norma; sem tolerância escrita, a tolerância geral ISO 2768 citada nas notas (por
//! exemplo "ISO 2768-m"). A cota de inspeção (valor no oval) é marcada para conferência de 100%.
//!
//! Saídas: a tabela de características na folha, os balões numerados, CSV e XLSX no leiaute do
//! formulário 3 da AS9102 (resultados das características).

use crate::layout::dim_value;
use crate::source::Models;
use crate::{DimKind, Drawing, NoteKind, TolMode, V2};
use fenkra_core::Id;
use fenkra_tolerancias::ajustes::ClasseTol;
use fenkra_tolerancias::geral::ClasseGeral;
use serde::{Deserialize, Serialize};

/// Opções do plano de inspeção do desenho.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpcoesInspecao {
    /// desenha o balão numerado junto de cada característica
    #[serde(default)]
    pub baloes: bool,
}

/// Que tipo de requisito a característica é.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TipoCarac {
    Dimensao,
    Geometrica,
    Rugosidade,
}

impl TipoCarac {
    pub fn label(self) -> &'static str {
        match self {
            TipoCarac::Dimensao => "Dimensão",
            TipoCarac::Geometrica => "Tolerância geométrica",
            TipoCarac::Rugosidade => "Rugosidade",
        }
    }
}

/// Uma característica do plano de inspeção.
#[derive(Clone, Debug, PartialEq)]
pub struct Caracteristica {
    pub numero: usize,
    /// cota ou nota de onde vem
    pub dono: Id,
    pub folha: Id,
    /// zona da folha (letra da linha e número da coluna): "B3"
    pub zona: String,
    pub tipo: TipoCarac,
    /// requisito como está escrito na folha: "Ø10 H7", "⌖ 0,05 A B", "Ra 1,6"
    pub requisito: String,
    pub nominal: Option<f64>,
    pub minimo: Option<f64>,
    pub maximo: Option<f64>,
    /// "mm", "°" ou "µm"
    pub unidade: &'static str,
    /// cota de inspeção (valor no oval): conferir em todas as peças
    pub cem_por_cento: bool,
    /// ponto da folha onde ela está (a numeração segue esta posição)
    pub pos: V2,
}

/// Zona da folha (mesma divisão do zoneamento desenhado na moldura: ~50 mm por zona).
pub fn zona(tamanho: V2, p: V2) -> String {
    let (w, h) = (tamanho[0], tamanho[1]);
    let (l, r) = (20., 10.);
    let (iw, ih) = (w - l - r, h - 2. * r);
    let nx = ((iw / 50.).round() as usize).max(2);
    let ny = ((ih / 50.).round() as usize).max(2);
    let i = (((p[0] - l) / iw * nx as f64).floor().max(0.) as usize).min(nx - 1);
    let j = (((h - r - p[1]) / ih * ny as f64).floor().max(0.) as usize).min(ny - 1);
    let letras = "ABCDEFGHJKLMNPQRSTUVWXYZ";
    format!("{}{}", letras.chars().nth(j % letras.len()).unwrap_or('A'), i + 1)
}

fn ordem_de_zona(tamanho: V2, p: V2) -> (usize, usize, i64) {
    let z = zona(tamanho, p);
    let letras = "ABCDEFGHJKLMNPQRSTUVWXYZ";
    let mut c = z.chars();
    let linha = c.next().and_then(|x| letras.find(x)).unwrap_or(0);
    let coluna: usize = c.as_str().parse().unwrap_or(0);
    // dentro da zona: de cima para baixo, depois da esquerda para a direita
    (linha, coluna, (-p[1] * 10.) as i64 * 100_000 + (p[0] * 10.) as i64)
}

/// Classe da tolerância geral ISO 2768 citada nas notas do desenho ("2768-m", "2768-mK").
pub fn tolerancia_geral(d: &Drawing) -> Option<ClasseGeral> {
    let textos = d.notes.iter().map(|n| n.text.as_str()).chain(std::iter::once(d.title.notes.as_str()));
    for t in textos {
        if let Some(i) = t.find("2768-") {
            let resto = &t[i + 5..];
            if let Some(c) = resto.chars().next() {
                if let Some(g) = ClasseGeral::parse(&c.to_string()) {
                    return Some(g);
                }
            }
        }
    }
    None
}

fn numero(s: &str) -> Option<f64> {
    s.trim().replace(',', ".").parse().ok()
}

/// Limites da cota de valor `v` pela tolerância escrita. `None` se não há como saber.
pub fn limites(d: &Drawing, dm: &crate::Dim, v: f64) -> Option<(f64, f64)> {
    match dm.tol_mode {
        TolMode::Simetrica if dm.tol_a > 0. => return Some((v - dm.tol_a, v + dm.tol_a)),
        TolMode::Bilateral if dm.tol_a > 0. || dm.tol_b > 0. => return Some((v - dm.tol_b, v + dm.tol_a)),
        _ => {}
    }
    let t = dm.tol.trim();
    if !t.is_empty() {
        if let Some(c) = ClasseTol::parse(t) {
            return c.limites_mm(v).map(|(max, min)| (min, max));
        }
        if let Some(a) = t.strip_prefix('±').and_then(numero) {
            return Some((v - a, v + a));
        }
        // "+0,1/-0,05" ou "+0,1/0"
        if let Some((sup, inf)) = t.split_once('/') {
            if let (Some(a), Some(b)) = (numero(sup.trim_start_matches('+')), numero(inf)) {
                return Some((v + a.min(b), v + a.max(b)));
            }
        }
        return None;
    }
    // sem tolerância escrita: a geral do desenho
    let g = tolerancia_geral(d)?;
    match dm.kind {
        DimKind::Angle => None,
        _ => g.linear_mm(v).map(|a| (v - a, v + a)),
    }
}

/// O que entra no plano: a cota que não é de referência, o quadro de tolerância e a rugosidade.
fn candidatos(d: &Drawing, m: &mut Models) -> Vec<Caracteristica> {
    let mut out = vec![];
    for dm in &d.dims {
        if dm.opcoes.parenteses {
            continue;
        }
        let Some(v) = d.view(dm.view) else { continue };
        let p = m.proj(d, v.id);
        let Some((val, pts)) = dim_value(dm, v, &p) else { continue };
        let n = pts.len().max(1) as f64;
        let mut pos = pts.iter().fold([0., 0.], |a, q| [a[0] + q[0] / n, a[1] + q[1] / n]);
        if let Some(t) = dm.text_pos {
            pos = [pos[0] + t[0], pos[1] + t[1]];
        }
        let lim = limites(d, dm, val);
        out.push(Caracteristica {
            numero: 0,
            dono: dm.id,
            folha: v.sheet,
            zona: String::new(),
            tipo: TipoCarac::Dimensao,
            requisito: crate::layout::dim_text(d, dm, val),
            nominal: Some(val),
            minimo: lim.map(|l| l.0),
            maximo: lim.map(|l| l.1),
            unidade: if dm.kind == DimKind::Angle { "°" } else { "mm" },
            cem_por_cento: dm.opcoes.inspecao,
            pos,
        });
    }
    for n in &d.notes {
        let (tipo, requisito, maximo, unidade) = match n.kind {
            NoteKind::Gdt => {
                let Some(g) = &n.gdt else { continue };
                let req = format!("{} {} {}", g.ch.symbol(), g.tol, g.datums).trim().to_string();
                (TipoCarac::Geometrica, req, numero(g.tol.trim_start_matches('Ø')), "mm")
            }
            NoteKind::Roughness => {
                let v = n.text.split_whitespace().last().and_then(numero);
                (TipoCarac::Rugosidade, n.text.clone(), v, "µm")
            }
            _ => continue,
        };
        // posição do símbolo (não a ponta da chamada): é ali que o balão fica
        out.push(Caracteristica { numero: 0, dono: n.id, folha: n.sheet, zona: String::new(), tipo, requisito, nominal: None, minimo: None, maximo, unidade, cem_por_cento: false, pos: n.pos });
    }
    out
}

/// Características do desenho, numeradas na ordem de leitura.
pub fn caracteristicas(d: &Drawing, m: &mut Models) -> Vec<Caracteristica> {
    let mut todas = candidatos(d, m);
    let ordem_folha = |id: Id| d.sheets.iter().position(|s| s.id == id).unwrap_or(usize::MAX);
    let tam = |id: Id| d.sheet(id).map(|s| s.size()).unwrap_or([420., 297.]);
    todas.sort_by_key(|c| (ordem_folha(c.folha), ordem_de_zona(tam(c.folha), c.pos)));
    for (i, c) in todas.iter_mut().enumerate() {
        c.numero = i + 1;
        c.zona = zona(tam(c.folha), c.pos);
    }
    todas
}

fn fmt(d: &Drawing, v: Option<f64>) -> String {
    v.map(|x| {
        let s = format!("{:.4}", x);
        let s = s.trim_end_matches('0').trim_end_matches('.').to_string();
        if d.decimals > 0 {
            s.replace('.', ",")
        } else {
            s
        }
    })
    .unwrap_or_default()
}

/// Cabeçalhos da tabela (folha) e das planilhas, com o número do campo do formulário 3 da AS9102.
pub const CABECALHOS: [&str; 9] = ["Nº", "Folha", "Zona", "Tipo", "Requisito", "Mínimo", "Máximo", "Unidade", "Inspeção"];

/// Linhas do plano, na ordem dos cabeçalhos.
pub fn linhas(d: &Drawing, cs: &[Caracteristica]) -> Vec<Vec<String>> {
    cs.iter()
        .map(|c| {
            let folha = d.sheets.iter().position(|s| s.id == c.folha).map(|i| (i + 1).to_string()).unwrap_or_default();
            vec![
                c.numero.to_string(),
                folha,
                c.zona.clone(),
                c.tipo.label().to_string(),
                c.requisito.clone(),
                fmt(d, c.minimo),
                fmt(d, c.maximo),
                c.unidade.to_string(),
                if c.cem_por_cento { "100%".into() } else { String::new() },
            ]
        })
        .collect()
}

/// Dados da tabela de características na folha.
pub fn tabela(d: &Drawing, m: &mut Models) -> crate::source::TableData {
    let cs = caracteristicas(d, m);
    crate::source::TableData { headers: CABECALHOS.iter().map(|s| s.to_string()).collect(), rows: linhas(d, &cs) }
}

/// Colunas do formulário 3 da AS9102 (números dos campos da norma) com o que o desenho já sabe;
/// resultado, ferramenta e não conformidade ficam para quem mede.
const FORM3: [&str; 11] = [
    "5. Nº da característica",
    "6. Local de referência (folha/zona)",
    "7. Designador",
    "8. Requisito",
    "Nominal",
    "Mínimo",
    "Máximo",
    "9. Resultado",
    "10. Ferramenta",
    "11. Não conformidade",
    "14. Observações",
];

fn linhas_form3(d: &Drawing, cs: &[Caracteristica]) -> Vec<Vec<String>> {
    cs.iter()
        .map(|c| {
            let folha = d.sheets.iter().position(|s| s.id == c.folha).map(|i| i + 1).unwrap_or(0);
            vec![
                c.numero.to_string(),
                format!("F{folha} {}", c.zona),
                if c.cem_por_cento { "100%".into() } else { String::new() },
                format!("{} ({})", c.requisito, c.unidade),
                fmt(d, c.nominal),
                fmt(d, c.minimo),
                fmt(d, c.maximo),
                String::new(),
                String::new(),
                String::new(),
                c.tipo.label().to_string(),
            ]
        })
        .collect()
}

fn csv_campo(s: &str) -> String {
    if s.contains([';', '"', '\n']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// CSV do formulário 3 (separador ";", que abre direto nas planilhas em português).
pub fn csv(d: &Drawing, cs: &[Caracteristica]) -> String {
    let mut out = String::new();
    out.push_str(&FORM3.iter().map(|s| csv_campo(s)).collect::<Vec<_>>().join(";"));
    out.push('\n');
    for l in linhas_form3(d, cs) {
        out.push_str(&l.iter().map(|s| csv_campo(s)).collect::<Vec<_>>().join(";"));
        out.push('\n');
    }
    out
}

fn xml(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn coluna(i: usize) -> String {
    let mut n = i + 1;
    let mut s = String::new();
    while n > 0 {
        let r = (n - 1) % 26;
        s.insert(0, (b'A' + r as u8) as char);
        n = (n - 1) / 26;
    }
    s
}

/// XLSX do formulário 3: cabeçalho da peça (formulário 1 resumido) e a tabela das características.
pub fn xlsx(d: &Drawing, cs: &[Caracteristica], codigo: &str, nome: &str) -> Vec<u8> {
    let mut linhas: Vec<Vec<String>> = vec![
        vec!["Relatório de primeiro artigo — formulário 3 (AS9102): resultados das características".into()],
        vec!["Código da peça".into(), codigo.into(), "Nome da peça".into(), nome.into(), "Revisão".into(), d.title.revision.clone()],
        vec![],
        FORM3.iter().map(|s| s.to_string()).collect(),
    ];
    linhas.extend(linhas_form3(d, cs));
    let mut folha = String::from(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData>"#);
    for (r, l) in linhas.iter().enumerate() {
        folha.push_str(&format!(r#"<row r="{}">"#, r + 1));
        for (c, v) in l.iter().enumerate() {
            let celula = format!("{}{}", coluna(c), r + 1);
            // números (nominal e limites) vão como número, o resto como texto
            match v.replace(',', ".").parse::<f64>() {
                Ok(x) if r >= 4 && (4..=6).contains(&c) => folha.push_str(&format!(r#"<c r="{celula}"><v>{x}</v></c>"#)),
                _ => folha.push_str(&format!(r#"<c r="{celula}" t="inlineStr"><is><t>{}</t></is></c>"#, xml(v))),
            }
        }
        folha.push_str("</row>");
    }
    folha.push_str("</sheetData></worksheet>");
    let arquivos: [(&str, String); 5] = [
        ("[Content_Types].xml", r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/><Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/></Types>"#.into()),
        ("_rels/.rels", r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#.into()),
        ("xl/workbook.xml", r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="Formulario 3" sheetId="1" r:id="rId1"/></sheets></workbook>"#.into()),
        ("xl/_rels/workbook.xml.rels", r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#.into()),
        ("xl/worksheets/sheet1.xml", folha),
    ];
    zip_sem_compressao(&arquivos.iter().map(|(n, c)| (*n, c.as_bytes())).collect::<Vec<_>>())
}

fn crc32(b: &[u8]) -> u32 {
    let mut c = 0xFFFF_FFFFu32;
    for x in b {
        c ^= *x as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { (c >> 1) ^ 0xEDB8_8320 } else { c >> 1 };
        }
    }
    !c
}

/// ZIP sem compressão (método 0): o bastante para o XLSX, sem dependência nova.
pub fn zip_sem_compressao(arquivos: &[(&str, &[u8])]) -> Vec<u8> {
    let mut out = vec![];
    let mut central = vec![];
    let le16 = |v: &mut Vec<u8>, x: u16| v.extend_from_slice(&x.to_le_bytes());
    let le32 = |v: &mut Vec<u8>, x: u32| v.extend_from_slice(&x.to_le_bytes());
    for (nome, dados) in arquivos {
        let (crc, n, off) = (crc32(dados), dados.len() as u32, out.len() as u32);
        le32(&mut out, 0x0403_4b50);
        for x in [20u16, 0, 0, 0, 0x21] {
            le16(&mut out, x);
        }
        le32(&mut out, crc);
        le32(&mut out, n);
        le32(&mut out, n);
        le16(&mut out, nome.len() as u16);
        le16(&mut out, 0);
        out.extend_from_slice(nome.as_bytes());
        out.extend_from_slice(dados);
        le32(&mut central, 0x0201_4b50);
        for x in [20u16, 20, 0, 0, 0, 0x21] {
            le16(&mut central, x);
        }
        le32(&mut central, crc);
        le32(&mut central, n);
        le32(&mut central, n);
        le16(&mut central, nome.len() as u16);
        for x in [0u16, 0, 0, 0] {
            le16(&mut central, x);
        }
        le32(&mut central, 0);
        le32(&mut central, off);
        central.extend_from_slice(nome.as_bytes());
    }
    let (ini, tam) = (out.len() as u32, central.len() as u32);
    out.extend_from_slice(&central);
    le32(&mut out, 0x0605_4b50);
    for x in [0u16, 0, arquivos.len() as u16, arquivos.len() as u16] {
        le16(&mut out, x);
    }
    le32(&mut out, tam);
    le32(&mut out, ini);
    le16(&mut out, 0);
    out
}
