// SPDX-License-Identifier: LGPL-2.1-or-later
//! B371: seção "Anotações do 3D" da vista selecionada. Mostra a lista do que o modelo tem, com o
//! que cada item vira na folha (ou por que fica de fora), deixa a pessoa marcar e importa só o
//! escolhido. As regras ficam em `fenkra_desenho::importar`.

use super::*;
use fenkra_desenho::importar::{self, Como};

fn nome_do_destino(c: &Como) -> String {
    match c {
        Como::Cota(k) => format!("{} {}", tr("Cota"), tr(k.label()).to_lowercase()),
        Como::Nota(k) => tr(match k {
            NoteKind::Text => "Nota",
            NoteKind::Balloon => "Balão",
            NoteKind::Roughness => "Rugosidade",
            NoteKind::Weld => "Solda",
            NoteKind::Gdt => "Tolerância geométrica",
            NoteKind::Datum => "Referência",
            NoteKind::CenterMark => "Marca de centro",
        })
        .to_string(),
    }
}

/// Seção da vista `vista`. Devolve `true` quando importou algo.
pub fn secao(ui: &mut egui::Ui, st: &mut DwgState, vista: Id) -> bool {
    let mut importou = false;
    let cabecalho = egui::CollapsingHeader::new(tr("Anotações do 3D")).id_salt(("importar_pmi", vista.0)).default_open(std::env::var("FENKRA_DEMO_DWG").as_deref() == Ok("importar"));
    cabecalho.show(ui, |ui| {
        let lista = importar::reconhecer(&st.d, &mut st.models, vista);
        if lista.is_empty() {
            ui.label(RichText::new(tr("Nenhuma anotação 3D nesta origem.")).weak().small());
            return;
        }
        ui.label(RichText::new(tr("Confira a lista: o que não cabe na folha fica de fora. O que entra continua ligado ao modelo e atualiza com ele.")).weak().small());
        // ids desmarcados guardados na memória da interface (por vista)
        let chave = egui::Id::new(("importar_pmi_off", vista.0));
        let mut off: Vec<u64> = ui.data_mut(|d| d.get_temp::<Vec<u64>>(chave).unwrap_or_default());
        let mut escolhidas = vec![];
        for c in &lista {
            ui.horizontal(|ui| match &c.destino {
                Ok(como) => {
                    let mut on = !off.contains(&c.pmi.0);
                    if ui.checkbox(&mut on, &c.nome).changed() {
                        off.retain(|x| *x != c.pmi.0);
                        if !on {
                            off.push(c.pmi.0);
                        }
                    }
                    ui.label(RichText::new(format!("→ {}", nome_do_destino(como))).weak().small());
                    if on {
                        escolhidas.push(c.pmi);
                    }
                }
                Err(f) => {
                    ui.add_enabled(false, egui::Checkbox::new(&mut false, &c.nome));
                    ui.label(RichText::new(format!("× {}", tr(f.texto()))).weak().small());
                }
            });
        }
        ui.data_mut(|d| d.insert_temp(chave, off));
        if ui.add_enabled(!escolhidas.is_empty(), egui::Button::new(format!("{} ({})", tr("Importar selecionadas"), escolhidas.len()))).on_hover_text(tr("Cria na folha as anotações marcadas, ligadas ao modelo")).clicked() {
            let rel = importar::importar(&mut st.d, &mut st.models, vista, &escolhidas);
            importou = !(rel.cotas.is_empty() && rel.notas.is_empty());
            st.sel = rel.cotas.first().or(rel.notas.first()).copied().or(st.sel);
        }
    });
    importou
}
