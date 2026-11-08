// SPDX-License-Identifier: LGPL-2.1-or-later
//! B117: plano de inspeção na bancada Desenho. Liga e desliga os balões numerados das
//! características, grava o formulário 3 (AS9102) em CSV ou XLSX e mostra o resumo nas
//! propriedades do desenho. A regra (o que entra, a ordem, os limites) mora em
//! `fenkra_desenho::inspecao`; a tabela na folha é uma tabela comum do tipo "Características".

use super::*;
use fenkra_desenho::inspecao;

/// Tabela de características não depende da peça: a origem fica vazia.
pub(super) fn origem_vazia() -> Source {
    Source { path: String::new(), display_state: String::new(), hidden: vec![] }
}

/// Liga ou desliga os balões numerados.
pub(super) fn alternar_baloes(st: &mut DwgState) {
    st.d.inspecao.baloes = !st.d.inspecao.baloes;
}

/// Grava o formulário 3 em CSV (`xlsx` falso) ou XLSX. Devolve a mensagem do aviso e se é erro.
pub(super) fn exportar(st: &mut DwgState, xlsx: bool) -> Option<(String, bool)> {
    let cs = inspecao::caracteristicas(&st.d, &mut st.models);
    if cs.is_empty() {
        return Some((tr("O desenho não tem características para inspecionar: cote a peça antes").to_string(), true));
    }
    let ext = if xlsx { "xlsx" } else { "csv" };
    let nome = format!("{} - inspecao.{ext}", st.d.name);
    let destino = std::env::var("FENKRA_DEMO_EXPORTAR_ARQUIVO").ok().map(PathBuf::from);
    let caminho = destino.or_else(|| rfd::FileDialog::new().add_filter(if xlsx { "XLSX" } else { "CSV" }, &[ext]).set_file_name(nome).save_file())?;
    let sh = st.d.sheets[0].clone();
    let campos = layout::sheet_fields(&st.d, &sh, &mut st.models);
    let codigo = campos.get("codigo").cloned().unwrap_or_default();
    let titulo = campos.get("descricao").cloned().unwrap_or_else(|| st.d.name.clone());
    let bytes = if xlsx { inspecao::xlsx(&st.d, &cs, &codigo, &titulo) } else { inspecao::csv(&st.d, &cs).into_bytes() };
    Some(match std::fs::write(&caminho, bytes) {
        Ok(()) => (format!("{} ({} {})", tr("Plano de inspeção gravado"), cs.len(), tr("características")), false),
        Err(e) => (e.to_string(), true),
    })
}

/// Seção nas propriedades do desenho: balões e contagem do plano.
pub(super) fn props(ui: &mut egui::Ui, st: &mut DwgState) {
    ui.separator();
    ui.label(RichText::new(tr("Plano de inspeção")).strong());
    let mut b = st.d.inspecao.baloes;
    if ui.checkbox(&mut b, tr("Balões numerados das características")).on_hover_text(tr("um número junto de cada cota, tolerância geométrica e rugosidade, na mesma ordem da tabela de características e do relatório")).changed() {
        st.d.inspecao.baloes = b;
    }
    let n = inspecao::caracteristicas(&st.d, &mut st.models).len();
    ui.label(RichText::new(format!("{n} {}", tr("características (cotas de referência ficam de fora)"))).weak().small());
    ui.horizontal(|ui| {
        if ui.button(tr("CSV")).on_hover_text(tr("formulário 3 (resultados das características), separado por ponto e vírgula")).clicked() {
            st.toast = exportar(st, false);
        }
        if ui.button(tr("XLSX")).on_hover_text(tr("formulário 3 numa planilha, com o código, o nome e a revisão da peça")).clicked() {
            st.toast = exportar(st, true);
        }
    });
}

/// Desenvolvimento (`FENKRA_DEMO_DWG_INSPECAO=1`): liga os balões e põe a tabela de características
/// na folha de exemplo, com a primeira cota em ±0,1 e de inspeção. Nada disso grava no disco.
pub(super) fn demo(st: &mut DwgState) {
    if std::env::var("FENKRA_DEMO_DWG_INSPECAO").is_err() {
        return;
    }
    st.d.inspecao.baloes = true;
    if let Some(dm) = st.d.dims.first_mut() {
        dm.tol_mode = TolMode::Simetrica;
        dm.tol_a = 0.1;
        dm.opcoes.inspecao = true;
    }
    let id = st.d.alloc();
    st.d.tables.push(Table { id, sheet: st.sheet, kind: TableKind::Caracteristicas, source: origem_vazia(), pos: [226., 100.], level: Default::default(), cfg: Default::default() });
}
