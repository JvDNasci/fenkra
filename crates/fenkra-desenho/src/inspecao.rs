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
//! Saídas: a tabela de características na folha, os balões numerados, CSV do formulário 3 da AS9102
//! (resultados das características) e XLSX com os formulários 1 (identificação da peça), 2 (materiais
//! e processos) e 3. As medições voltam de um CSV preenchido e mostram a situação de cada uma.

use crate::layout::dim_value;
use crate::source::Models;
use crate::{DimKind, Drawing, NoteKind, TolMode, V2};
use fenkra_core::Id;
use fenkra_tolerancias::ajustes::ClasseTol;
use fenkra_tolerancias::geral::ClasseGeral;
use serde::{Deserialize, Serialize};

/// Opções do plano de inspeção do desenho.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct OpcoesInspecao {
    /// desenha o balão numerado junto de cada característica
    #[serde(default)]
    pub baloes: bool,
    /// resultados importados do relatório preenchido, pelo número da característica
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub medicoes: Vec<Medicao>,
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

/// Dados da tabela de características na folha. Com medições importadas, ganha as colunas do valor
/// medido e da situação.
pub fn tabela(d: &Drawing, m: &mut Models) -> crate::source::TableData {
    let cs = caracteristicas(d, m);
    let mut headers: Vec<String> = CABECALHOS.iter().map(|s| s.to_string()).collect();
    let mut rows = linhas(d, &cs);
    if !d.inspecao.medicoes.is_empty() {
        headers.extend(["Medido".to_string(), "Situação".to_string()]);
        for (r, c) in rows.iter_mut().zip(&cs) {
            let v = medida(d, c.numero);
            r.push(v.unwrap_or("").to_string());
            r.push(v.map(|v| situacao(c, v).label().to_string()).unwrap_or_default());
        }
    }
    crate::source::TableData { headers, rows }
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
                medida(d, c.numero).unwrap_or("").to_string(),
                String::new(),
                medida(d, c.numero).filter(|v| situacao(c, v) == Situacao::NaoConforme).map(|_| "Sim".to_string()).unwrap_or_default(),
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

/// Formulário 1 da AS9102 (identificação da peça): rótulo do campo e o que o desenho já sabe. Os
/// números são os dos campos da norma; os textos são nossos.
pub fn form1(d: &Drawing, cs: &[Caracteristica], codigo: &str, nome: &str, montagem: bool) -> Vec<(String, String)> {
    let (medidas, nc, faltam) = resumo(d, cs);
    let conforme = if medidas == 0 {
        String::new()
    } else if nc == 0 && faltam == 0 {
        "Sim".into()
    } else if nc > 0 {
        format!("Não ({nc} não conformes)")
    } else {
        format!("Incompleto ({faltam} sem medida)")
    };
    let t = &d.title;
    let pn = if codigo.is_empty() { t.pn.clone() } else { codigo.to_string() };
    vec![
        ("1. Código da peça".into(), pn.clone()),
        ("2. Nome da peça".into(), if nome.is_empty() { t.title.clone() } else { nome.to_string() }),
        ("3. Número de série".into(), String::new()),
        ("4. Número do relatório".into(), String::new()),
        ("5. Revisão da peça".into(), t.revision.clone()),
        ("6. Número do desenho".into(), if pn.is_empty() { d.name.clone() } else { pn }),
        ("7. Revisão do desenho".into(), t.revision.clone()),
        ("8. Alterações incluídas".into(), String::new()),
        ("9. Referência do processo de fabricação".into(), String::new()),
        ("10. Organização".into(), t.company.clone()),
        ("11. Código do fornecedor".into(), String::new()),
        ("12. Pedido de compra".into(), String::new()),
        ("13. Peça ou montagem".into(), if montagem { "Montagem".into() } else { "Peça".into() }),
        ("14. Completo ou parcial".into(), "Completo".into()),
        ("15. Características no relatório".into(), cs.len().to_string()),
        ("19. Resultado conforme".into(), conforme),
        ("20. Preparado por".into(), t.drawn_by.clone()),
        ("21. Data".into(), t.date.clone()),
        ("22. Revisado por".into(), t.checked_by.clone()),
        ("24. Aprovado por".into(), t.approved_by.clone()),
    ]
}

/// Cabeçalhos do formulário 2 da AS9102 (materiais, processos especiais e ensaios).
pub const FORM2: [&str; 7] = ["5. Material ou processo", "6. Especificação", "7. Código", "8. Fornecedor", "9. Aprovação do cliente", "10. Certificado", "14. Observações"];

/// Normas que marcam uma nota como especificação de material ou processo.
const NORMAS: [&str; 9] = ["ISO", "NBR", "DIN", "EN ", "ASTM", "SAE", "AMS", "ASME", "JIS"];

/// Linhas do formulário 2: o material da legenda e cada linha de nota que cita uma norma (tratamento,
/// pintura, solda...). Fornecedor, aprovação e certificado ficam para quem recebe a peça.
pub fn form2(d: &Drawing) -> Vec<Vec<String>> {
    let mut out = vec![];
    if !d.title.material.trim().is_empty() {
        out.push(vec![d.title.material.trim().to_string(), String::new(), String::new(), String::new(), String::new(), String::new(), "Material".into()]);
    }
    let notas = d.notes.iter().filter(|n| n.kind == NoteKind::Text).map(|n| n.text.as_str()).chain(std::iter::once(d.title.notes.as_str()));
    for t in notas {
        for l in t.lines().map(str::trim).filter(|l| !l.is_empty()) {
            // a tolerância geral não é processo: já está no formulário 3
            if l.contains("2768") {
                continue;
            }
            if let Some(norma) = NORMAS.iter().find_map(|n| l.find(n).map(|i| l[i..].split([',', ';', ')']).next().unwrap_or("").trim().to_string())) {
                out.push(vec![l.to_string(), norma, String::new(), String::new(), String::new(), String::new(), "Processo ou especificação".into()]);
            }
        }
    }
    out
}

/// Uma folha da planilha em XML. `numero` diz quais células (linha, coluna) vão como número.
fn planilha(linhas: &[Vec<String>], numero: &dyn Fn(usize, usize) -> bool) -> String {
    let mut folha = String::from(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData>"#);
    for (r, l) in linhas.iter().enumerate() {
        folha.push_str(&format!(r#"<row r="{}">"#, r + 1));
        for (c, v) in l.iter().enumerate() {
            let celula = format!("{}{}", coluna(c), r + 1);
            match v.replace(',', ".").parse::<f64>() {
                Ok(x) if numero(r, c) => folha.push_str(&format!(r#"<c r="{celula}"><v>{x}</v></c>"#)),
                _ => folha.push_str(&format!(r#"<c r="{celula}" t="inlineStr"><is><t>{}</t></is></c>"#, xml(v))),
            }
        }
        folha.push_str("</row>");
    }
    folha.push_str("</sheetData></worksheet>");
    folha
}

/// XLSX com os três formulários da AS9102 em três abas: identificação da peça (1), materiais e
/// processos (2) e resultados das características (3), com as medições importadas.
pub fn xlsx(d: &Drawing, cs: &[Caracteristica], codigo: &str, nome: &str) -> Vec<u8> {
    xlsx_completo(d, cs, codigo, nome, false)
}

/// Como `xlsx`, dizendo se o desenho é de montagem (campo 13 do formulário 1).
pub fn xlsx_completo(d: &Drawing, cs: &[Caracteristica], codigo: &str, nome: &str, montagem: bool) -> Vec<u8> {
    let f1: Vec<Vec<String>> = std::iter::once(vec!["Relatório de primeiro artigo — formulário 1 (AS9102): identificação da peça".to_string()])
        .chain(std::iter::once(vec![]))
        .chain(form1(d, cs, codigo, nome, montagem).into_iter().map(|(a, b)| vec![a, b]))
        .collect();
    let mut f2: Vec<Vec<String>> = vec![vec!["Relatório de primeiro artigo — formulário 2 (AS9102): materiais, processos especiais e ensaios".into()], vec![], FORM2.iter().map(|s| s.to_string()).collect()];
    f2.extend(form2(d));
    let mut f3: Vec<Vec<String>> = vec![
        vec!["Relatório de primeiro artigo — formulário 3 (AS9102): resultados das características".into()],
        vec!["Código da peça".into(), codigo.into(), "Nome da peça".into(), nome.into(), "Revisão".into(), d.title.revision.clone()],
        vec![],
        FORM3.iter().map(|s| s.to_string()).collect(),
    ];
    f3.extend(linhas_form3(d, cs));
    // nominal, limites e resultado numérico vão como número
    let folhas = [("Formulario 1", planilha(&f1, &|_, _| false)), ("Formulario 2", planilha(&f2, &|_, _| false)), ("Formulario 3", planilha(&f3, &|r, c| r >= 4 && (4..=7).contains(&c)))];
    let mut tipos = String::from(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>"#);
    let mut abas = String::new();
    let mut rels = String::from(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#);
    for (i, (nome_aba, _)) in folhas.iter().enumerate() {
        let k = i + 1;
        tipos.push_str(&format!(r#"<Override PartName="/xl/worksheets/sheet{k}.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>"#));
        abas.push_str(&format!(r#"<sheet name="{nome_aba}" sheetId="{k}" r:id="rId{k}"/>"#));
        rels.push_str(&format!(r#"<Relationship Id="rId{k}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet{k}.xml"/>"#));
    }
    tipos.push_str("</Types>");
    rels.push_str("</Relationships>");
    let livro = format!(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets>{abas}</sheets></workbook>"#);
    let mut arquivos: Vec<(String, String)> = vec![
        ("[Content_Types].xml".into(), tipos),
        ("_rels/.rels".into(), r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#.into()),
        ("xl/workbook.xml".into(), livro),
        ("xl/_rels/workbook.xml.rels".into(), rels),
    ];
    for (i, (_, x)) in folhas.into_iter().enumerate() {
        arquivos.push((format!("xl/worksheets/sheet{}.xml", i + 1), x));
    }
    zip_sem_compressao(&arquivos.iter().map(|(n, c)| (n.as_str(), c.as_bytes())).collect::<Vec<_>>())
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

/// Valor medido de uma característica, como veio do relatório preenchido.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Medicao {
    pub numero: usize,
    pub valor: String,
}

/// Como a medida fica contra os limites.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Situacao {
    Conforme,
    NaoConforme,
    /// sem limites no desenho para comparar (o medido fica registrado)
    SemLimite,
    /// o resultado não é um número ("ok", "visual"): fica como está
    Texto,
}

impl Situacao {
    pub fn label(self) -> &'static str {
        match self {
            Situacao::Conforme => "Conforme",
            Situacao::NaoConforme => "Não conforme",
            Situacao::SemLimite => "Sem limite",
            Situacao::Texto => "Anotado",
        }
    }
}

/// Situação do valor medido contra os limites da característica. A rugosidade e a tolerância
/// geométrica só têm máximo.
pub fn situacao(c: &Caracteristica, valor: &str) -> Situacao {
    let Some(v) = numero(valor.trim().trim_start_matches('Ø')) else { return Situacao::Texto };
    match (c.minimo, c.maximo) {
        (None, None) => Situacao::SemLimite,
        (lo, hi) => {
            let folga = 1e-9;
            if lo.is_some_and(|l| v < l - folga) || hi.is_some_and(|h| v > h + folga) {
                Situacao::NaoConforme
            } else {
                Situacao::Conforme
            }
        }
    }
}

/// Medida registrada para a característica `numero`.
pub fn medida(d: &Drawing, numero: usize) -> Option<&str> {
    d.inspecao.medicoes.iter().find(|m| m.numero == numero).map(|m| m.valor.as_str())
}

fn separar(linha: &str, sep: char) -> Vec<String> {
    // campos entre aspas podem ter o separador dentro
    let mut out = vec![];
    let mut atual = String::new();
    let mut aspas = false;
    let mut cs = linha.chars().peekable();
    while let Some(c) = cs.next() {
        match c {
            '"' if aspas && cs.peek() == Some(&'"') => {
                atual.push('"');
                cs.next();
            }
            '"' => aspas = !aspas,
            c if c == sep && !aspas => out.push(std::mem::take(&mut atual)),
            c => atual.push(c),
        }
    }
    out.push(atual);
    out.into_iter().map(|s| s.trim().to_string()).collect()
}

/// Lê as medições de um CSV: o formulário 3 que o Fenkra grava, preenchido na coluna "9. Resultado"
/// (separador ";" ou ","), ou uma planilha simples de duas colunas, número e valor. Linhas sem número
/// ou sem resultado ficam de fora.
pub fn importar_medicoes(texto: &str) -> Result<Vec<Medicao>, String> {
    let linhas: Vec<&str> = texto.lines().filter(|l| !l.trim().is_empty()).collect();
    let Some(primeira) = linhas.first() else { return Err("arquivo vazio".into()) };
    let sep = if primeira.matches(';').count() >= primeira.matches(',').count() { ';' } else { ',' };
    let cab = separar(primeira, sep);
    let achar = |prefixos: &[&str]| cab.iter().position(|h| prefixos.iter().any(|p| h.to_lowercase().starts_with(p)));
    let (col_n, col_v, inicio) = match (achar(&["5.", "nº", "n°", "numero", "número", "char"]), achar(&["9.", "resultado", "medido", "valor", "result"])) {
        (Some(n), Some(v)) => (n, v, 1),
        // sem cabeçalho: número e valor nas duas primeiras colunas
        _ if separar(primeira, sep).first().is_some_and(|s| s.parse::<usize>().is_ok()) => (0, 1, 0),
        _ => return Err("não achei as colunas do número da característica e do resultado".into()),
    };
    let mut out: Vec<Medicao> = vec![];
    for l in &linhas[inicio..] {
        let campos = separar(l, sep);
        let (Some(n), Some(v)) = (campos.get(col_n).and_then(|s| s.parse::<usize>().ok()), campos.get(col_v).filter(|v| !v.is_empty())) else { continue };
        match out.iter_mut().find(|m| m.numero == n) {
            Some(m) => m.valor = v.clone(),
            None => out.push(Medicao { numero: n, valor: v.clone() }),
        }
    }
    if out.is_empty() {
        return Err("nenhuma linha com número e resultado".into());
    }
    Ok(out)
}

/// Resumo das medições: (medidas, não conformes, sem medida).
pub fn resumo(d: &Drawing, cs: &[Caracteristica]) -> (usize, usize, usize) {
    let mut r = (0, 0, 0);
    for c in cs {
        match medida(d, c.numero) {
            Some(v) => {
                r.0 += 1;
                if situacao(c, v) == Situacao::NaoConforme {
                    r.1 += 1;
                }
            }
            None => r.2 += 1,
        }
    }
    r
}
