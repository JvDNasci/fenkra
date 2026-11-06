// SPDX-License-Identifier: LGPL-2.1-or-later
//! Tabelas configuráveis (B295): colunas escolhidas e ordenadas, ordenação pelo cabeçalho, divisão
//! em blocos lado a lado, renumeração e etiquetas de furo (A1, A2…) nas vistas.
//!
//! A tabela crua (`Models::table`) continua sendo a verdade dos dados; aqui só se escolhe o que
//! mostrar. Cada linha mostrada lembra de qual linha crua veio (`origem`), e é por isso que a
//! etiqueta de um furo na vista acha a linha certa mesmo depois de ordenar ou renumerar.

use crate::source::{Models, TableData};
use crate::{Drawing, Id, Table, TableKind, V2};
use serde::{Deserialize, Serialize};

/// Configuração de uma tabela. O padrão mostra tudo, na ordem de sempre.
#[derive(Clone, PartialEq, Debug, Default, Serialize, Deserialize)]
pub struct ConfigTabela {
    /// colunas mostradas, na ordem (pelo nome do cabeçalho); vazio = todas
    #[serde(default)]
    pub colunas: Vec<String>,
    /// coluna de ordenação e se é crescente
    #[serde(default)]
    pub ordenar: Option<(String, bool)>,
    /// linhas por bloco; os blocos ficam lado a lado (0 = tabela inteira)
    #[serde(default)]
    pub dividir: usize,
    /// com `dividir`, cada bloco vai para uma folha (a da tabela e as seguintes) em vez de ficar
    /// ao lado; o título ganha "(1/3)"
    #[serde(default)]
    pub em_folhas: bool,
    /// numera de novo a primeira coluna (Item, Nº, Furo) na ordem mostrada
    #[serde(default)]
    pub renumerar: bool,
    /// tabela de furos: vista cujo plano dá as coordenadas X e Y (e onde as etiquetas ficam)
    #[serde(default)]
    pub vista_origem: Option<Id>,
    /// tabela de furos: etiquetas A1, A2… nas vistas
    #[serde(default)]
    pub etiquetas: bool,
    /// linha crua em destaque (vinda do clique numa etiqueta); não é gravada
    #[serde(skip)]
    pub linha_sel: Option<usize>,
}

/// Tabela pronta para desenhar.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TabelaFinal {
    pub data: TableData,
    /// linha crua de cada linha mostrada
    pub origem: Vec<usize>,
    /// todos os cabeçalhos da tabela crua (para a lista de colunas do painel)
    pub todas: Vec<String>,
}

fn numero(s: &str) -> Option<f64> {
    s.trim().trim_end_matches('°').replace(',', ".").parse().ok()
}

/// Compara dois textos de célula: número com número, senão texto, com "A2" antes de "A10".
fn compara(a: &str, b: &str) -> std::cmp::Ordering {
    if let (Some(x), Some(y)) = (numero(a), numero(b)) {
        return x.total_cmp(&y);
    }
    let (pa, na) = prefixo(a);
    let (pb, nb) = prefixo(b);
    pa.cmp(&pb).then(na.cmp(&nb)).then(a.cmp(b))
}

/// Letras do começo e o número que vem depois ("A12" → ("A", 12)).
fn prefixo(s: &str) -> (String, u64) {
    let letras: String = s.chars().take_while(|c| !c.is_ascii_digit()).collect();
    let n = s[letras.len()..].chars().take_while(|c| c.is_ascii_digit()).collect::<String>().parse().unwrap_or(0);
    (letras, n)
}

/// Primeira coluna que guarda o número da linha: Item, Nº ou Furo.
fn coluna_de_numero(h: &str) -> bool {
    matches!(h, "Item" | "Nº" | "Furo")
}

/// Aplica a configuração a uma tabela crua. `tipo` decide como renumerar (furos viram A1, A2…).
pub fn aplicar(cru: &TableData, cfg: &ConfigTabela, tipo: &TableKind) -> TabelaFinal {
    let todas = cru.headers.clone();
    // colunas: as pedidas que existem, na ordem pedida; nenhuma válida = todas
    let mut idx: Vec<usize> = cfg.colunas.iter().filter_map(|n| cru.headers.iter().position(|h| h == n)).collect();
    idx.dedup();
    if idx.is_empty() {
        idx = (0..cru.headers.len()).collect();
    }
    let mut ordem: Vec<usize> = (0..cru.rows.len()).collect();
    if let Some((nome, asc)) = &cfg.ordenar {
        if let Some(c) = cru.headers.iter().position(|h| h == nome) {
            let cel = |r: usize| cru.rows[r].get(c).map(String::as_str).unwrap_or("");
            ordem.sort_by(|a, b| {
                let o = compara(cel(*a), cel(*b));
                if *asc {
                    o
                } else {
                    o.reverse()
                }
            });
        }
    }
    let headers: Vec<String> = idx.iter().map(|i| cru.headers[*i].clone()).collect();
    let rows: Vec<Vec<String>> = ordem
        .iter()
        .enumerate()
        .map(|(k, r)| {
            idx.iter()
                .map(|c| {
                    let h = &cru.headers[*c];
                    let celula = cru.rows[*r].get(*c).cloned().unwrap_or_default();
                    if *c == 0 && cfg.renumerar && coluna_de_numero(h) {
                        if *tipo == TableKind::Holes {
                            format!("A{}", k + 1)
                        } else {
                            (k + 1).to_string()
                        }
                    } else {
                        celula
                    }
                })
                .collect()
        })
        .collect();
    TabelaFinal { data: TableData { headers, rows }, origem: ordem, todas }
}

/// Divide as linhas em blocos de no máximo `n` (0 = um bloco só).
pub fn blocos(rows: &[Vec<String>], n: usize) -> Vec<&[Vec<String>]> {
    if n == 0 || rows.len() <= n {
        return vec![rows];
    }
    rows.chunks(n).collect()
}

/// Quantos blocos a tabela tem (um bloco só quando `dividir` é 0 ou cabe tudo).
pub fn n_blocos(linhas: usize, dividir: usize) -> usize {
    if dividir == 0 || linhas <= dividir { 1 } else { linhas.div_ceil(dividir) }
}

/// Blocos que a tabela desenha na folha `sheet`: todos lado a lado na folha dela, ou, com
/// `em_folhas`, o bloco `k` na k-ésima folha a partir da dela. `None` se nada vai nessa folha.
pub fn blocos_na_folha(d: &Drawing, t: &Table, sheet: Id, total: usize) -> Option<std::ops::Range<usize>> {
    if !t.cfg.em_folhas || t.cfg.dividir == 0 {
        return (t.sheet == sheet).then_some(0..total);
    }
    let i0 = d.sheets.iter().position(|s| s.id == t.sheet)?;
    let i = d.sheets.iter().position(|s| s.id == sheet)?;
    (i >= i0 && i - i0 < total).then(|| (i - i0)..(i - i0 + 1))
}

/// Etiqueta de furo: o "A3" da linha crua `i`, ou da posição mostrada quando renumerada.
pub fn rotulo_furo(cfg: &ConfigTabela, f: &TabelaFinal, linha_crua: usize) -> String {
    let pos = f.origem.iter().position(|o| *o == linha_crua).unwrap_or(linha_crua);
    format!("A{}", if cfg.renumerar { pos + 1 } else { linha_crua + 1 })
}

/// Coordenadas de um furo no plano da vista de origem: X e Y do plano e Z ao longo da normal.
fn no_plano(ax: &crate::source::Axes, p: [f64; 3]) -> [f64; 3] {
    let q = ax.project(p);
    [q[0], q[1], crate::source::dot(p, ax.toward)]
}

/// Dados finais de uma tabela do desenho: crua, com as coordenadas da vista de origem (furos) e a
/// configuração aplicada.
pub fn dados(m: &mut Models, t: &Table, d: &Drawing) -> TabelaFinal {
    let mut cru = m.table(t, d);
    if t.kind == TableKind::Holes {
        if let Some(ax) = t.cfg.vista_origem.filter(|v| d.view(*v).is_some()).and_then(|v| m.proj(d, v).axes) {
            let furos = m.furos(&t.source);
            let col = |n: &str| cru.headers.iter().position(|h| h == n);
            if let (Some(cx), Some(cy), Some(cz)) = (col("X"), col("Y"), col("Z")) {
                for (r, h) in cru.rows.iter_mut().zip(furos.iter()) {
                    let q = no_plano(&ax, h.pos);
                    r[cx] = d.num(q[0]);
                    r[cy] = d.num(q[1]);
                    r[cz] = d.num(q[2]);
                }
            }
        }
    }
    aplicar(&cru, &t.cfg, &t.kind)
}

/// Etiqueta de furo posta numa vista.
#[derive(Clone, Debug, PartialEq)]
pub struct Etiqueta {
    pub tabela: Id,
    pub view: Id,
    /// linha crua da tabela (índice do furo)
    pub linha: usize,
    pub texto: String,
    /// posição do texto na folha (mm)
    pub pos: V2,
    /// centro do furo na folha, para a linha de chamada
    pub furo: V2,
}

/// Etiquetas dos furos de uma tabela, nas vistas onde eles aparecem como círculo (o eixo do furo
/// aponta para o observador). Com vista de origem escolhida, só nela.
pub fn etiquetas(m: &mut Models, d: &Drawing, t: &Table, f: &TabelaFinal) -> Vec<Etiqueta> {
    if t.kind != TableKind::Holes || !t.cfg.etiquetas {
        return vec![];
    }
    let furos = m.furos(&t.source);
    let mut out = vec![];
    let vistas: Vec<Id> = d
        .views
        .iter()
        .filter(|v| v.sheet == t.sheet && v.source.path == t.source.path)
        .filter(|v| t.cfg.vista_origem.is_none_or(|o| o == v.id))
        .map(|v| v.id)
        .collect();
    for vid in vistas {
        let Some(v) = d.view(vid) else { continue };
        let p = m.proj(d, vid);
        let Some(ax) = p.axes else { continue };
        for (i, h) in furos.iter().enumerate() {
            // furo visto de frente ou de trás: eixo paralelo à direção de visada
            if crate::source::dot(h.dir, ax.toward).abs() < 0.99 {
                continue;
            }
            let c = crate::layout::to_sheet(v, &p, ax.project(h.pos));
            let r = (h.diameter / 2.) * v.scale;
            let s = d.text_height;
            // ao lado do furo, para cima e para a direita, fora do círculo
            let pos = [c[0] + r + s * 0.6, c[1] + r + s * 0.4];
            out.push(Etiqueta { tabela: t.id, view: vid, linha: i, texto: rotulo_furo(&t.cfg, f, i), pos, furo: c });
        }
    }
    out
}

/// A etiqueta (se houver) debaixo do ponto `q` da folha: o clique nela escolhe a linha da tabela.
pub fn etiqueta_em<'a>(ets: &'a [Etiqueta], q: V2, tol: f64) -> Option<&'a Etiqueta> {
    ets.iter().find(|e| {
        let w = e.texto.chars().count() as f64 * 2.;
        q[0] >= e.pos[0] - tol && q[0] <= e.pos[0] + w + tol && q[1] >= e.pos[1] - tol && q[1] <= e.pos[1] + 3.5 + tol
    })
}

/// Ordenação pelo clique no cabeçalho: nenhuma → crescente → decrescente → nenhuma.
pub fn alterna_ordem(cfg: &mut ConfigTabela, coluna: &str) {
    cfg.ordenar = match cfg.ordenar.take() {
        Some((c, true)) if c == coluna => Some((c, false)),
        Some((c, false)) if c == coluna => None,
        _ => Some((coluna.to_string(), true)),
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cru() -> TableData {
        TableData {
            headers: ["Furo", "X", "Y", "Ø"].map(String::from).to_vec(),
            rows: vec![["A1", "10", "5", "8"], ["A2", "30", "5", "6"], ["A3", "20", "5", "10"], ["A10", "5", "5", "6"]].iter().map(|r| r.iter().map(|s| s.to_string()).collect()).collect(),
        }
    }

    #[test]
    fn tirar_uma_coluna_reflui_a_tabela() {
        let cfg = ConfigTabela { colunas: ["Furo", "X", "Ø"].map(String::from).to_vec(), ..Default::default() };
        let f = aplicar(&cru(), &cfg, &TableKind::Holes);
        assert_eq!(f.data.headers, ["Furo", "X", "Ø"]);
        assert!(f.data.rows.iter().all(|r| r.len() == 3));
        assert_eq!(f.data.rows[0], ["A1", "10", "8"]);
        // a ordem pedida vale: Ø antes de X
        let cfg = ConfigTabela { colunas: ["Furo", "Ø", "X"].map(String::from).to_vec(), ..Default::default() };
        assert_eq!(aplicar(&cru(), &cfg, &TableKind::Holes).data.rows[1], ["A2", "6", "30"]);
        // todas as colunas pedidas inexistentes: volta a mostrar tudo, nunca tabela vazia
        let cfg = ConfigTabela { colunas: vec!["nada".into()], ..Default::default() };
        assert_eq!(aplicar(&cru(), &cfg, &TableKind::Holes).data.headers.len(), 4);
    }

    #[test]
    fn ordena_por_numero_e_renumera_e_a_etiqueta_acha_a_linha_crua() {
        let mut cfg = ConfigTabela::default();
        alterna_ordem(&mut cfg, "X");
        let f = aplicar(&cru(), &cfg, &TableKind::Holes);
        let xs: Vec<&str> = f.data.rows.iter().map(|r| r[1].as_str()).collect();
        assert_eq!(xs, ["5", "10", "20", "30"], "número, não texto");
        assert_eq!(f.origem, [3, 0, 2, 1]);
        alterna_ordem(&mut cfg, "X");
        assert_eq!(aplicar(&cru(), &cfg, &TableKind::Holes).origem, [1, 2, 0, 3]);
        alterna_ordem(&mut cfg, "X");
        assert!(cfg.ordenar.is_none());
        // sem renumerar o furo A10 continua A10; renumerado, vira A1 na primeira posição
        cfg.ordenar = Some(("X".into(), true));
        let f = aplicar(&cru(), &cfg, &TableKind::Holes);
        assert_eq!(f.data.rows[0][0], "A10");
        assert_eq!(rotulo_furo(&cfg, &f, 3), "A4");
        cfg.renumerar = true;
        let f = aplicar(&cru(), &cfg, &TableKind::Holes);
        assert_eq!(f.data.rows[0][0], "A1");
        assert_eq!(rotulo_furo(&cfg, &f, 3), "A1", "a etiqueta do furo acompanha a linha");
        assert_eq!(f.origem[0], 3, "a linha A1 é o furo cru 3");
    }

    #[test]
    fn nome_com_numero_ordena_pelo_numero() {
        let t = TableData { headers: vec!["Item".into()], rows: ["A10", "A2", "A1"].iter().map(|s| vec![s.to_string()]).collect() };
        let cfg = ConfigTabela { ordenar: Some(("Item".into(), true)), ..Default::default() };
        let f = aplicar(&t, &cfg, &TableKind::Bom);
        assert_eq!(f.data.rows.iter().map(|r| r[0].as_str()).collect::<Vec<_>>(), ["A1", "A2", "A10"]);
    }

    #[test]
    fn divide_em_blocos() {
        let f = aplicar(&cru(), &ConfigTabela::default(), &TableKind::Holes);
        let b = blocos(&f.data.rows, 3);
        assert_eq!(b.len(), 2);
        assert_eq!((b[0].len(), b[1].len()), (3, 1));
        assert_eq!(blocos(&f.data.rows, 0).len(), 1);
    }
}
