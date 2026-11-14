// SPDX-License-Identifier: LGPL-2.1-or-later
//! Alvos de soltura da árvore (B446).
//!
//! Antes só a linha de um recurso, a pasta de geometria e o PMI liam a soltura; o resto da árvore
//! deixava o item cair sem efeito, e o clique da soltura ainda selecionava o alvo. Agora cada linha
//! que pode receber algo passa por [`ligar`]: durante o arraste, o alvo que aceita ganha um contorno
//! e o que recusa troca o cursor por "proibido" e mostra o motivo; ao soltar, o que aceita vira uma
//! ação e o que recusa vira um aviso. A regra de cada soltura mora no documento
//! (`fenkra_core::arvore_soltura`), e aqui ela é testada num clone antes de valer.

use crate::app::{DragItem, TreeAction};
use crate::i18n::tr;
use eframe::egui;
use fenkra_core::{Document, Id, RecusaSoltura};

/// Onde o item foi solto.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Alvo {
    /// pasta de geometria ou de comandos
    Pasta(Id),
    /// nó de um corpo (coletor)
    Corpo(Id),
    /// linha de um parâmetro: o arrastado vai para o lugar dele
    Parametro(Id),
    /// cabeça de um grupo da árvore (`externas`, `publicacoes`, `pmi`, `capturas`, `parametros`)
    /// ou o nó "Grupos vazios" (`vazios`)
    Grupo(&'static str),
}

impl Alvo {
    fn id(self) -> Option<Id> {
        match self {
            Alvo::Pasta(i) | Alvo::Corpo(i) | Alvo::Parametro(i) => Some(i),
            Alvo::Grupo(_) => None,
        }
    }
}

/// Aplica a soltura no documento. Recusada, o documento fica como estava.
pub(crate) fn aplicar(d: &mut Document, item: Id, alvo: Alvo) -> Result<(), RecusaSoltura> {
    match alvo {
        Alvo::Pasta(p) => d.soltar_em_pasta(item, p),
        Alvo::Corpo(c) => d.soltar_no_corpo(item, c),
        Alvo::Parametro(p) => d.mover_parametro(item, p),
        Alvo::Grupo("parametros") => {
            // na cabeça do grupo, o parâmetro vai para o fim da lista
            let ultimo = d.params.last().map(|p| p.id).ok_or(RecusaSoltura::NaoAceita)?;
            d.mover_parametro(item, ultimo)
        }
        Alvo::Grupo("publicacoes") => {
            // soltar em Publicações publica o item, como o "📤 Publicar" do menu
            if let Some(r) = d.references.iter_mut().find(|r| r.id == item) {
                r.published = true;
            } else if let Some(p) = d.params.iter_mut().find(|p| p.id == item) {
                p.published = true;
            } else if let Some(c) = d.collectors.iter_mut().find(|c| c.id == item) {
                c.published = true;
            } else {
                return Err(RecusaSoltura::NaoAceita);
            }
            Ok(())
        }
        Alvo::Grupo(_) => Err(RecusaSoltura::NaoAceita),
    }
}

/// A soltura seria aceita? Testa num clone, sem tocar o documento.
pub(crate) fn testar(doc: &Document, item: Id, alvo: Alvo) -> Result<(), RecusaSoltura> {
    if alvo.id() == Some(item) {
        return Err(RecusaSoltura::NaoAceita);
    }
    aplicar(&mut doc.clone(), item, alvo)
}

/// Motivo da recusa, traduzido.
pub(crate) fn motivo(e: &RecusaSoltura) -> String {
    let com = |modelo: &str, nome: &str| tr(modelo).replace("{}", nome);
    let m = match e {
        RecusaSoltura::NaoEComando => tr("Só comandos da linha do tempo entram aqui").to_string(),
        RecusaSoltura::SoGeometriaLivre => tr("Comando da linha do tempo não vai para pasta de geometria").to_string(),
        RecusaSoltura::PastaDentroDelaMesma => tr("A pasta não pode ir para dentro dela mesma").to_string(),
        RecusaSoltura::PastaDeOutroCorpo(c) => com("A pasta é do corpo {}", c),
        RecusaSoltura::JaNoCorpo(c) => com("Já está no corpo {}", c),
        RecusaSoltura::Referencia => tr("Geometria de referência não entra num corpo: fica numa pasta de geometria").to_string(),
        RecusaSoltura::UsaOutroCorpo(n) => com("Usa o resultado de {}, de outro corpo", n),
        RecusaSoltura::UsadoNoCorpo(n) => com("{} usa este comando e fica no corpo atual", n),
        RecusaSoltura::Ordem(n) => com("A ordem violaria a dependência de {}", n),
        RecusaSoltura::SemSolidoNoCorpo(c) => com("O corpo {} não tem sólido antes deste comando para ele modificar", c),
        RecusaSoltura::NaoAceita => tr("Este lugar não guarda este item").to_string(),
        RecusaSoltura::NaoEncontrado => tr("Objeto não encontrado").to_string(),
    };
    format!("{}: {m}", tr("Não dá para soltar aqui"))
}

const CHAVE_SOLTANDO: &str = "fk_arvore_soltando";

/// Marca, no começo da árvore, se este quadro solta um item arrastado. Nesse quadro nenhum clique
/// da árvore seleciona: soltar não é clicar no alvo.
pub(crate) fn inicio_do_quadro(ctx: &egui::Context) {
    let soltando = egui::DragAndDrop::has_payload_of_type::<DragItem>(ctx) && ctx.input(|i| i.pointer.any_released());
    ctx.data_mut(|d| d.insert_temp(egui::Id::new(CHAVE_SOLTANDO), soltando));
}

/// Este quadro solta um item arrastado (ver [`inicio_do_quadro`])?
pub(crate) fn soltando(ctx: &egui::Context) -> bool {
    ctx.data(|d| d.get_temp(egui::Id::new(CHAVE_SOLTANDO))).unwrap_or(false)
}

/// Liga o alvo `alvo` à resposta `r`: realce ou cursor de proibido durante o arraste e, ao soltar,
/// a ação ou o aviso com o motivo.
pub(crate) fn ligar(ui: &egui::Ui, r: &egui::Response, doc: &Document, alvo: Alvo, actions: &mut Vec<TreeAction>) {
    let Some(p) = r.dnd_hover_payload::<DragItem>() else { return };
    let item = p.0;
    let res = testar(doc, item, alvo);
    match &res {
        Ok(()) => {
            ui.painter().rect_stroke(r.rect.expand(2.), 3., egui::Stroke::new(1.5, crate::theme::accent(ui.visuals())), egui::StrokeKind::Outside);
        }
        Err(e) => {
            ui.ctx().set_cursor_icon(egui::CursorIcon::NoDrop);
            r.show_tooltip_text(motivo(e));
        }
    }
    if r.dnd_release_payload::<DragItem>().is_some() {
        actions.push(match res {
            Ok(()) => TreeAction::Soltar(item, alvo),
            Err(e) => TreeAction::SolturaRecusada(motivo(&e)),
        });
    }
}

/// A cabeça de um grupo da árvore como alvo: o retângulo da primeira linha desenhada entre `topo` e
/// o cursor atual. Os grupos são desenhados por funções diferentes (PMI e capturas em `pmi_ui`); o
/// alvo cobre a linha sem mexer nelas.
pub(crate) fn ligar_cabeca(ui: &mut egui::Ui, topo: f32, k: &'static str, doc: &Document, actions: &mut Vec<TreeAction>) {
    let h = ui.spacing().interact_size.y;
    let fim = ui.cursor().top();
    if fim <= topo {
        return;
    }
    let rect = egui::Rect::from_min_max(egui::pos2(ui.min_rect().left(), topo), egui::pos2(ui.max_rect().right(), (topo + h).min(fim)));
    let r = ui.interact(rect, ui.id().with(("soltura_grupo", k)), egui::Sense::hover());
    ligar(ui, &r, doc, Alvo::Grupo(k), actions);
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn grupos_aceitam_o_que_guardam_e_recusam_o_resto() {
        let mut d = fenkra_solidos::example_document();
        let topo = d.references.iter().find(|r| r.name == "PlanoTopo").unwrap().id;
        let l = d.params[0].id;
        let furo = d.features.iter().find(|f| f.name == "Furos M6").unwrap().id;
        // publicar por soltura
        assert!(testar(&d, topo, Alvo::Grupo("publicacoes")).is_ok());
        aplicar(&mut d, topo, Alvo::Grupo("publicacoes")).unwrap();
        assert!(d.reference(topo).unwrap().published);
        // um comando sólido em Capturas, PMI ou Publicações: recusa, com motivo
        for g in ["capturas", "pmi", "publicacoes", "externas", "vazios"] {
            let e = testar(&d, furo, Alvo::Grupo(g)).unwrap_err();
            assert_eq!(e, RecusaSoltura::NaoAceita, "{g}");
            assert!(motivo(&e).contains("Não dá para soltar aqui"));
        }
        // parâmetro na cabeça de Parâmetros vai para o fim
        aplicar(&mut d, l, Alvo::Grupo("parametros")).unwrap();
        assert_eq!(d.params.last().unwrap().id, l);
        // soltar algo em si mesmo nunca vale
        let pasta = d.folders.iter().find(|f| f.name == "Acabamentos").unwrap().id;
        assert!(testar(&d, pasta, Alvo::Pasta(pasta)).is_err());
    }
}
