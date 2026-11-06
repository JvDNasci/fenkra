// SPDX-License-Identifier: LGPL-2.1-or-later
//! Padrão de interface obrigatório (B319): o que todo comando novo precisa trazer. Falha quando um
//! comando vem sem ícone da família nova, sem tradução ou sem dica, ou quando entra cor fixa nova.
//!
//! As listas abaixo são a dívida de quando a regra nasceu e só podem encolher. O código antigo ainda
//! tem cores fixas e textos sem tradução; exigir zero hoje travaria tudo, e não exigir nada deixaria a
//! dívida crescer. Quem quitar parte dela baixa o número aqui.

use crate::i18n::tem_traducao;
use crate::toolbar::{all_entries, Entry, Item};
use std::collections::BTreeMap;

/// Cores fixas toleradas por arquivo. Arquivo fora da lista não pode ter nenhuma.
const CORES_FIXAS: &[(&str, usize)] = &[
    ("app.rs", 42),
    ("asm_ui.rs", 13),
    ("dwg_ui.rs", 12),
    ("editor.rs", 10),
    ("feat_ui.rs", 4),
    ("geo_ui.rs", 5),
    ("gizmo.rs", 7),
    ("handles.rs", 2),
    ("picking.rs", 4),
    ("pmi_draw.rs", 6),
    ("pmi_ui.rs", 3),
    ("roles.rs", 1),
    ("scene.rs", 4),
    ("sketch_ui.rs", 18),
    ("st_ui.rs", 1),
    ("toolbar.rs", 18),
];

/// Textos entre `tr("...")` ainda sem tradução, somando o código todo.
const SEM_TRADUCAO: usize = 56;

/// Aqui as cores são a definição do tema, ou o próprio texto da regra.
const DONOS_DA_COR: &[&str] = &["theme.rs", "padrao.rs"];

fn itens() -> Vec<Item> {
    all_entries()
        .into_iter()
        .flat_map(|e| match e {
            Entry::One(i) => vec![i],
            Entry::Family(_, v) => v,
        })
        .collect()
}

fn fontes() -> Vec<(String, String)> {
    let pasta = concat!(env!("CARGO_MANIFEST_DIR"), "/src");
    let mut v: Vec<(String, String)> = std::fs::read_dir(pasta)
        .expect("pasta src")
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().is_some_and(|x| x == "rs"))
        .map(|e| (e.file_name().to_string_lossy().into_owned(), std::fs::read_to_string(e.path()).expect("fonte")))
        .collect();
    v.sort();
    v
}

/// O código sem os testes do fim do arquivo: teste pode usar cor fixa à vontade.
fn sem_testes(fonte: &str) -> &str {
    fonte.split("#[cfg(test)]\nmod tests").next().and_then(|a| a.split("#[cfg(test)]\r\nmod tests").next()).unwrap_or(fonte)
}

fn cores_fixas(fonte: &str) -> usize {
    let f = sem_testes(fonte);
    ["Color32::from_rgb(", "Color32::from_rgba_unmultiplied(", "Color32::from_rgba_premultiplied(", "C::from_rgb("].iter().map(|p| f.matches(p).count()).sum()
}

fn textos_de_tr(fonte: &str) -> Vec<&str> {
    sem_testes(fonte).split("tr(\"").skip(1).filter_map(|t| t.split('"').next()).filter(|t| !t.is_empty() && !t.contains('\\')).collect()
}

#[test]
fn todo_comando_tem_icone_da_familia_nova() {
    let faltam: Vec<&str> = itens().iter().map(|i| i.icon).filter(|n| !crate::icons::existe(n)).collect();
    assert!(faltam.is_empty(), "sem ícone em assets/icons2: {faltam:?}");
}

#[test]
fn todo_comando_tem_traducao() {
    let faltam: Vec<&str> = itens().iter().flat_map(|i| [i.name, i.brief]).filter(|t| !tem_traducao(t)).collect();
    assert!(faltam.len() <= COMANDOS_SEM_TRADUCAO, "{} comandos sem tradução em i18n.rs (limite {COMANDOS_SEM_TRADUCAO}): {faltam:?}", faltam.len());
}

/// Nomes e descrições de comandos sem tradução quando a regra nasceu.
const COMANDOS_SEM_TRADUCAO: usize = 0;

#[test]
fn todo_comando_tem_dica() {
    let faltam: Vec<&str> = itens().iter().filter(|i| i.brief.trim().is_empty() || i.advanced.trim().is_empty()).map(|i| i.name).collect();
    assert!(faltam.is_empty(), "sem dica: {faltam:?}");
}

#[test]
fn nenhuma_cor_fixa_nova() {
    let limite: BTreeMap<&str, usize> = CORES_FIXAS.iter().copied().collect();
    let mut acima = vec![];
    for (nome, fonte) in fontes() {
        if DONOS_DA_COR.contains(&nome.as_str()) {
            continue;
        }
        let n = cores_fixas(&fonte);
        let max = limite.get(nome.as_str()).copied().unwrap_or(0);
        if n > max {
            acima.push(format!("{nome}: {n} (limite {max})"));
        }
    }
    assert!(acima.is_empty(), "cor fixa nova; use as cores de theme.rs: {acima:?}");
}

#[test]
fn texto_novo_vem_com_traducao() {
    let mut faltam: Vec<String> = fontes().iter().filter(|(n, _)| n != "padrao.rs").flat_map(|(_, f)| textos_de_tr(f)).filter(|t| !tem_traducao(t)).map(str::to_string).collect();
    faltam.sort();
    faltam.dedup();
    assert!(faltam.len() <= SEM_TRADUCAO, "{} textos sem tradução em i18n.rs (limite {SEM_TRADUCAO}): {faltam:?}", faltam.len());
}

#[test]
fn a_regra_enxerga_o_que_deve_proibir() {
    assert_eq!(cores_fixas("let a = Color32::from_rgb(1, 2, 3);"), 1);
    assert_eq!(cores_fixas("x\n#[cfg(test)]\nmod tests { Color32::from_rgb(1, 2, 3) }"), 0);
    assert_eq!(textos_de_tr(r#"ui.label(tr("Olá")); tr("Mundo")"#), ["Olá", "Mundo"]);
    assert!(!tem_traducao("texto que ninguém traduziu"));
}
