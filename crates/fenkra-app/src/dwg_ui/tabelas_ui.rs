// SPDX-License-Identifier: LGPL-2.1-or-later
//! Tabelas configuráveis na interface (B295): lista de colunas com ordem, ordenação, divisão,
//! renumeração e etiquetas de furo. O clique no cabeçalho ordena e o clique numa etiqueta de furo
//! (na vista) seleciona a linha da tabela. Os dados e a geometria vêm de `fenkra_desenho::tabela`.

use super::*;
use fenkra_desenho::tabela;

/// Controles da tabela selecionada. `todas` são os cabeçalhos da tabela crua; `vistas` as vistas da
/// folha (para a vista de origem dos furos).
pub fn controles(ui: &mut egui::Ui, t: &mut Table, todas: &[String], vistas: &[(Id, String)]) {
    ui.separator();
    ui.label(RichText::new(tr("Colunas")).strong());
    let cfg = &mut t.cfg;
    // lista de trabalho: as mostradas na ordem escolhida, depois as escondidas
    let mut mostradas: Vec<String> = if cfg.colunas.is_empty() { todas.to_vec() } else { cfg.colunas.iter().filter(|c| todas.contains(c)).cloned().collect() };
    if mostradas.is_empty() {
        mostradas = todas.to_vec();
    }
    let escondidas: Vec<String> = todas.iter().filter(|c| !mostradas.contains(c)).cloned().collect();
    let mut mudou = false;
    let mut mover: Option<(usize, isize)> = None;
    let mut tirar: Option<usize> = None;
    let so_uma = mostradas.len() == 1;
    for (i, c) in mostradas.iter().enumerate() {
        ui.horizontal(|ui| {
            let mut on = true;
            if ui.add_enabled(!so_uma, egui::Checkbox::new(&mut on, tr(c))).on_hover_text(tr("tirar a coluna: a tabela reflui")).changed() {
                tirar = Some(i);
            }
            if ui.add_enabled(i > 0, egui::Button::new("▲").small()).on_hover_text(tr("subir a coluna")).clicked() {
                mover = Some((i, -1));
            }
            if ui.add_enabled(i + 1 < mostradas.len(), egui::Button::new("▼").small()).on_hover_text(tr("descer a coluna")).clicked() {
                mover = Some((i, 1));
            }
        });
    }
    let mut nova = mostradas.clone();
    if let Some(i) = tirar {
        nova.remove(i);
        mudou = true;
    }
    if let Some((i, d)) = mover {
        nova.swap(i, (i as isize + d) as usize);
        mudou = true;
    }
    for c in &escondidas {
        let mut on = false;
        if ui.checkbox(&mut on, tr(c)).on_hover_text(tr("mostrar a coluna")).changed() {
            nova.push(c.clone());
            mudou = true;
        }
    }
    if mudou {
        cfg.colunas = if nova.len() == todas.len() && nova.iter().zip(todas).all(|(a, b)| a == b) { vec![] } else { nova };
    }
    ui.add_space(4.);
    ui.horizontal(|ui| {
        ui.label(tr("Ordenar por"));
        let atual = cfg.ordenar.as_ref().map(|(c, _)| c.clone());
        egui::ComboBox::from_id_salt(("tab_ordem", t.id.0)).selected_text(atual.clone().map(|c| tr(&c).to_string()).unwrap_or_else(|| tr("nenhuma").to_string())).show_ui(ui, |ui| {
            if ui.selectable_label(atual.is_none(), tr("nenhuma")).clicked() {
                cfg.ordenar = None;
            }
            for c in todas {
                if ui.selectable_label(atual.as_ref() == Some(c), tr(c)).clicked() {
                    cfg.ordenar = Some((c.clone(), true));
                }
            }
        });
        if let Some((_, asc)) = cfg.ordenar.as_mut() {
            if ui.small_button(if *asc { "↑" } else { "↓" }).on_hover_text(tr("crescente ou decrescente (clicar no cabeçalho na folha também ordena)")).clicked() {
                *asc = !*asc;
            }
        }
    });
    ui.horizontal(|ui| {
        ui.label(tr("Linhas por bloco")).on_hover_text(tr("0 mostra a tabela inteira; mais que isso divide em blocos lado a lado"));
        ui.add(crate::ui_ext::dv(&mut cfg.dividir).range(0..=200).speed(0.2));
    });
    ui.add_enabled_ui(cfg.dividir > 0, |ui| {
        crate::editor::check(ui, &mut cfg.em_folhas, "Cada bloco numa folha");
    })
    .response
    .on_hover_text(tr("Com linhas por bloco, o primeiro bloco fica nesta folha e os outros nas folhas seguintes (criadas se faltarem)"));
    crate::editor::check(ui, &mut cfg.renumerar, "Renumerar na ordem mostrada");
    if t.kind == TableKind::Holes {
        ui.separator();
        ui.horizontal(|ui| {
            ui.label(tr("Vista de origem"));
            let nome = t.cfg.vista_origem.and_then(|v| vistas.iter().find(|x| x.0 == v)).map(|x| x.1.clone()).unwrap_or_else(|| tr("coordenadas do modelo").to_string());
            egui::ComboBox::from_id_salt(("tab_vista", t.id.0)).selected_text(nome).show_ui(ui, |ui| {
                if ui.selectable_label(t.cfg.vista_origem.is_none(), tr("coordenadas do modelo")).clicked() {
                    t.cfg.vista_origem = None;
                }
                for (id, n) in vistas {
                    if ui.selectable_label(t.cfg.vista_origem == Some(*id), n).clicked() {
                        t.cfg.vista_origem = Some(*id);
                    }
                }
            });
        });
        ui.label(RichText::new(tr("Com vista de origem, X e Y são medidos no plano dela e Z ao longo da direção de visada.")).weak().small());
        crate::editor::check(ui, &mut t.cfg.etiquetas, "Etiquetas dos furos nas vistas (A1, A2…)");
        ui.label(RichText::new(tr("Clique numa etiqueta na vista para destacar a linha da tabela.")).weak().small());
    }
}

/// Com "cada bloco numa folha" ligado, cria as folhas de continuação que faltarem (mesmo formato da
/// folha da tabela). Devolve `true` se criou alguma.
pub fn garantir_folhas(st: &mut DwgState, id: Id) -> bool {
    let Some(t) = st.d.tables.iter().find(|t| t.id == id).cloned() else { return false };
    if !t.cfg.em_folhas || t.cfg.dividir == 0 {
        return false;
    }
    let total = tabela::n_blocos(tabela::dados(&mut st.models, &t, &st.d).data.rows.len(), t.cfg.dividir);
    let Some(i0) = st.d.sheets.iter().position(|s| s.id == t.sheet) else { return false };
    let (formato, retrato) = (st.d.sheets[i0].format, st.d.sheets[i0].portrait);
    let mut criou = false;
    while st.d.sheets.len() < i0 + total {
        let nova = st.d.add_sheet(formato);
        if let Some(sh) = st.d.sheets.iter_mut().find(|s| s.id == nova) {
            sh.portrait = retrato;
        }
        criou = true;
    }
    if criou {
        st.modified = true;
    }
    criou
}

/// Clique na folha (sem ferramenta): num cabeçalho de tabela selecionada ordena por ele; numa
/// etiqueta de furo seleciona a tabela e destaca a linha. `true` se o clique foi usado.
pub fn clique(st: &mut DwgState, q: V2) -> bool {
    let th = st.d.text_height * 0.8;
    let ids: Vec<Id> = st.d.tables.iter().filter(|t| t.sheet == st.sheet || t.cfg.em_folhas).map(|t| t.id).collect();
    // cabeçalho da tabela já selecionada
    if let Some(id) = st.sel.filter(|s| ids.contains(s)) {
        let t = st.d.tables.iter().find(|t| t.id == id).cloned().unwrap();
        let fin = tabela::dados(&mut st.models, &t, &st.d);
        if let Some(col) = {
            let total = tabela::n_blocos(fin.data.rows.len(), t.cfg.dividir);
            tabela::blocos_na_folha(&st.d, &t, st.sheet, total).and_then(|f| layout::cabecalho_em(t.pos, t.kind.label(), &fin, &t.cfg, th, q, f))
        } {
            if let Some(tm) = st.d.tables.iter_mut().find(|x| x.id == id) {
                tabela::alterna_ordem(&mut tm.cfg, &col);
            }
            st.modified = true;
            return true;
        }
    }
    // etiqueta de furo
    for id in ids {
        let t = st.d.tables.iter().find(|t| t.id == id).cloned().unwrap();
        if !t.cfg.etiquetas {
            continue;
        }
        let fin = tabela::dados(&mut st.models, &t, &st.d);
        let ets = tabela::etiquetas(&mut st.models, &st.d, &t, &fin);
        if let Some(e) = tabela::etiqueta_em(&ets, q, 1.5) {
            let linha = e.linha;
            for tm in st.d.tables.iter_mut() {
                tm.cfg.linha_sel = (tm.id == id).then_some(linha);
            }
            st.sel = Some(id);
            return true;
        }
    }
    for tm in st.d.tables.iter_mut() {
        tm.cfg.linha_sel = None;
    }
    false
}
