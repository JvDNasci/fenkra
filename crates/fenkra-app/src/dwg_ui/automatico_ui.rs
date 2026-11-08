// SPDX-License-Identifier: LGPL-2.1-or-later
//! B144: comando Desenho automático. Sem vistas na folha, cria as vistas padrão pela origem (o
//! mesmo do assistente de Novo desenho); depois aplica as regras de `fenkra_desenho::automatico`
//! (contorno, furos, marcas de centro e tabela de furos) e conta o que fez num aviso.

use super::*;
use fenkra_desenho::automatico;

/// Executa o automático na folha atual. Devolve a mensagem e se é erro.
pub(super) fn executar(st: &mut DwgState) -> (String, bool) {
    if !st.d.views.iter().any(|v| v.sheet == st.sheet) {
        // folha vazia: as vistas padrão, como no Novo desenho
        let Some(src) = st.source.clone() else {
            return (tr("Falta a origem: escolha uma peça ou montagem salva").to_string(), true);
        };
        if st.d.views.is_empty() && st.d.sheets.len() == 1 {
            let mut n = fenkra_desenho::novo::NovoDesenho::padrao(&st.d.name, Some(src));
            n.formato = st.d.sheets[0].format;
            n.retrato = st.d.sheets[0].portrait;
            n.projecao = st.d.projection;
            n.norma = st.d.title.style;
            let mut novo = n.criar(&mut st.models);
            novo.title = st.d.title.clone();
            novo.uid = st.d.uid.clone();
            st.d = novo;
            st.sheet = st.d.sheets[0].id;
        } else {
            st.d.add_standard_set(st.sheet, src, [200., 170.], 1., 90.);
        }
        st.invalidate();
    }
    let r = automatico::completar(&mut st.d, st.sheet, &mut st.models);
    st.invalidate();
    let mut msg = format!("{}: {} {}, {} {}{}", tr("Desenho automático"), r.cotas, tr("cotas"), r.centros, tr("marcas de centro"), if r.tabela { format!(", {}", tr("tabela de furos")) } else { String::new() });
    for a in &r.avisos {
        msg.push_str(&format!(" · {}", tr(a)));
    }
    (msg, !r.avisos.is_empty() && r.cotas == 0)
}

/// Desenvolvimento (`FENKRA_DEMO_DWG_AUTO=1`): esvazia o desenho de exemplo, mantém a origem e roda o
/// automático, como na primeira vez numa folha nova. Nada disso grava no disco.
pub(super) fn demo(st: &mut DwgState) {
    if std::env::var("FENKRA_DEMO_DWG_AUTO").is_err() {
        return;
    }
    st.source = st.d.views.first().map(|v| v.source.clone()).or(st.source.clone());
    let s = st.sheet;
    st.d.views.clear();
    st.d.dims.clear();
    st.d.notes.retain(|n| n.sheet != s || n.kind == NoteKind::Text);
    st.d.tables.clear();
    st.d.centers.clear();
    let (m, e) = executar(st);
    st.toast = Some((m, e));
    st.sel = None;
}
