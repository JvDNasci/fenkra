// SPDX-License-Identifier: LGPL-2.1-or-later
//! Origem da montagem na árvore e seleção igual à das peças (B22). Ctrl soma e tira, Shift pega a
//! faixa, Alt+clique sobe da peça para o grupo dela, e ocultar e apagar valem para a seleção inteira.

use crate::app::App;
use crate::asm_ui::{node_action, AsmState};
use crate::i18n::tr;
use eframe::egui;
use fenkra_core::{Id, OriginAxis, OriginPlane};
use fenkra_montagem::proposta::{Classe, Elemento};
use fenkra_montagem::*;

/// Item da origem da montagem. Fica fora da seleção de peças: ele serve de referência.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ItemOrigem {
    Plano(OriginPlane),
    Eixo(OriginAxis),
    Ponto,
}

const ITENS: [ItemOrigem; 7] = [
    ItemOrigem::Plano(OriginPlane::XY),
    ItemOrigem::Plano(OriginPlane::XZ),
    ItemOrigem::Plano(OriginPlane::YZ),
    ItemOrigem::Eixo(OriginAxis::X),
    ItemOrigem::Eixo(OriginAxis::Y),
    ItemOrigem::Eixo(OriginAxis::Z),
    ItemOrigem::Ponto,
];

impl ItemOrigem {
    fn icone(self) -> &'static str {
        match self {
            ItemOrigem::Plano(_) => "plano",
            ItemOrigem::Eixo(_) => "linha_eixo",
            ItemOrigem::Ponto => "ponto_centro",
        }
    }

    pub fn nome(self) -> String {
        match self {
            ItemOrigem::Plano(o) => format!("{} {}", tr("Plano"), crate::scene::plane_name(o)),
            ItemOrigem::Eixo(a) => format!("{} {a:?}", tr("Eixo")),
            ItemOrigem::Ponto => tr("Ponto de origem").to_string(),
        }
    }

    pub fn elemento(self) -> Elemento {
        let (geom, classe) = match self {
            ItemOrigem::Plano(o) => {
                let f = fenkra_solidos::origin_frame(o);
                (Geom::Plane { p: f.origin, n: f.normal }, Classe::Face)
            }
            ItemOrigem::Eixo(a) => (Geom::Axis { p: [0.; 3], d: fenkra_solidos::origin_axis(a) }, Classe::ArestaReta),
            ItemOrigem::Ponto => (Geom::Point([0.; 3]), Classe::Vertice),
        };
        Elemento::novo(MateRef { comp: ORIGEM, geom, pick: None }, classe)
    }
}

/// Nó "Origem" da árvore: planos, eixos e ponto, para usar como referência das restrições.
pub fn arvore(st: &mut AsmState, ui: &mut egui::Ui) {
    let mut clicado = None;
    let mut ver = !st.asm.origem_oculta;
    let marcado = st.origem_sel;
    let mut trocou = false;
    let _ = crate::tree_ui::grupo(ui, "asm_arvore_origem", Some("plano"), tr("Origem"), false).corpo(ui, |ui| {
        for item in ITENS {
            let r = crate::tree_ui::linha(ui, item.icone(), marcado == Some(item), item.nome()).on_hover_text(tr("Escolha e abra uma restrição, ou clique aqui com o comando aberto: vira o elemento dele"));
            if r.clicked() {
                clicado = Some(item);
            }
        }
        crate::tree_ui::marca(ui);
        trocou = ui.checkbox(&mut ver, tr("mostrar origem")).changed();
    });
    if trocou {
        st.push_undo();
        st.asm.origem_oculta = !ver;
        st.dirty = true;
    }
    if let Some(item) = clicado {
        if st.cmd.is_some() {
            crate::asm_cmd::receber_origem(st, item);
        } else {
            st.origem_sel = (st.origem_sel != Some(item)).then_some(item);
        }
    }
}

/// Clique numa linha da árvore: sem tecla escolhe só ela, Ctrl soma ou tira, Shift pega a faixa desde
/// o último clicado. `sel` fica no último da seleção, que é o do manipulador.
pub fn clique(sel: &mut Option<Id>, multi: &mut Vec<Id>, ordem: &[Id], id: Id, ctrl: bool, shift: bool) {
    if ctrl {
        if multi.is_empty() {
            multi.extend(sel.iter().copied().filter(|s| ordem.contains(s)));
        }
        match multi.iter().position(|x| *x == id) {
            Some(k) => {
                multi.remove(k);
            }
            None => multi.push(id),
        }
        *sel = multi.last().copied();
    } else if shift {
        let a = sel.and_then(|s| ordem.iter().position(|x| *x == s));
        let b = ordem.iter().position(|x| *x == id);
        *multi = match (a, b) {
            (Some(a), Some(b)) => ordem[a.min(b)..=a.max(b)].to_vec(),
            _ => vec![id],
        };
        *sel = Some(id);
    } else {
        multi.clear();
        *sel = Some(id);
    }
}

/// Alt+clique: a peça; de novo, o grupo dela; e assim subindo até o topo.
pub fn subir_nivel(a: &Assembly, sel: Option<Id>, comp: Id) -> Id {
    let mut c = comp;
    while let Some(s) = sel {
        if c == s {
            return a.node(c).and_then(|n| n.parent).filter(|p| a.node(*p).is_some()).unwrap_or(c);
        }
        match a.node(c).and_then(|n| n.parent) {
            Some(p) => c = p,
            None => break,
        }
    }
    comp
}

/// Componentes que uma ação em lote atinge.
pub fn alvos(st: &AsmState, app_picks: &[crate::picking::Pick]) -> Vec<Id> {
    let mut out: Vec<Id> = st.multi.clone();
    for c in st.sel.into_iter().filter(|s| st.asm.node(*s).is_some()).chain(st.comps_of(app_picks)) {
        if !out.contains(&c) {
            out.push(c);
        }
    }
    out
}

/// Mostra ou oculta como um grupo só: basta uma visível para a seleção inteira sumir.
pub fn alternar_visivel(a: &mut Assembly, ids: &[Id]) {
    let ocultar = ids.iter().any(|i| a.node(*i).is_some_and(|n| n.visible));
    for i in ids {
        if let Some(n) = a.node_mut(*i) {
            n.visible = !ocultar;
        }
    }
}

/// Ocultar (1) ou apagar (0) a seleção, com um desfazer só. Uma peça só cai na ação de sempre.
pub fn em_lote(app: &mut App, k: u8) {
    let Some(st) = app.asm.as_ref() else { return };
    let ids = alvos(st, &app.picks);
    if ids.len() < 2 {
        if let Some(id) = ids.first().copied().or(st.sel) {
            node_action(app, id, k);
        }
        return;
    }
    let Some(st) = app.asm.as_mut() else { return };
    st.push_undo();
    if k == 0 {
        for id in &ids {
            st.asm.delete(*id);
        }
        st.sel = None;
        st.multi.clear();
        app.picks.clear();
    } else {
        alternar_visivel(&mut st.asm, &ids);
    }
    st.dirty = true;
}

/// Esc sem comando aberto limpa a seleção de peças.
pub fn teclas(app: &mut App, ctx: &egui::Context) {
    let Some(st) = app.asm.as_mut() else { return };
    if st.cmd.is_none() && !ctx.egui_wants_keyboard_input() && !st.editing() && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        st.multi.clear();
        st.origem_sel = None;
        if st.sel.is_some_and(|s| st.asm.node(s).is_some()) {
            st.sel = None;
        }
        st.fantasma_novo = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids() -> Vec<Id> {
        vec![Id(1), Id(2), Id(3), Id(4)]
    }

    #[test]
    fn clique_simples_troca_e_ctrl_soma_e_tira() {
        let (mut sel, mut multi, ordem) = (None, vec![], ids());
        clique(&mut sel, &mut multi, &ordem, Id(2), false, false);
        assert_eq!((sel, multi.len()), (Some(Id(2)), 0));
        clique(&mut sel, &mut multi, &ordem, Id(3), true, false);
        assert_eq!(multi, vec![Id(2), Id(3)], "o que estava escolhido entra na soma");
        assert_eq!(sel, Some(Id(3)));
        clique(&mut sel, &mut multi, &ordem, Id(2), true, false);
        assert_eq!(multi, vec![Id(3)]);
        clique(&mut sel, &mut multi, &ordem, Id(1), false, false);
        assert!(multi.is_empty());
    }

    #[test]
    fn shift_pega_a_faixa_nos_dois_sentidos() {
        let (mut sel, mut multi, ordem) = (Some(Id(3)), vec![], ids());
        clique(&mut sel, &mut multi, &ordem, Id(1), false, true);
        assert_eq!(multi, vec![Id(1), Id(2), Id(3)]);
        sel = Some(Id(2));
        clique(&mut sel, &mut multi, &ordem, Id(4), false, true);
        assert_eq!(multi, vec![Id(2), Id(3), Id(4)]);
    }

    #[test]
    fn alt_sobe_da_peca_para_o_grupo() {
        let mut a = Assembly::new("m");
        let g1 = a.add_group("Externo", None);
        let g2 = a.add_group("Interno", Some(g1));
        let p = a.add_component("Peça", NodeKind::Part { path: "p.fkp".into() }, Placement::default(), Some(g2));
        assert_eq!(subir_nivel(&a, None, p), p);
        assert_eq!(subir_nivel(&a, Some(p), p), g2);
        assert_eq!(subir_nivel(&a, Some(g2), p), g1);
        assert_eq!(subir_nivel(&a, Some(g1), p), g1, "o topo fica no topo");
        let solta = a.add_component("Solta", NodeKind::Part { path: "p.fkp".into() }, Placement::default(), None);
        assert_eq!(subir_nivel(&a, Some(g1), solta), solta, "outra peça começa por ela");
    }

    #[test]
    fn ocultar_em_lote_vale_para_todas() {
        let mut a = Assembly::new("m");
        let p = a.add_component("A", NodeKind::Part { path: "p.fkp".into() }, Placement::default(), None);
        let q = a.add_component("B", NodeKind::Part { path: "p.fkp".into() }, Placement::default(), None);
        a.node_mut(q).unwrap().visible = false;
        alternar_visivel(&mut a, &[p, q]);
        assert!(!a.node(p).unwrap().visible && !a.node(q).unwrap().visible, "uma visível: todas somem");
        alternar_visivel(&mut a, &[p, q]);
        assert!(a.node(p).unwrap().visible && a.node(q).unwrap().visible);
    }

    #[test]
    fn itens_da_origem_tem_nome_e_tipo_certos() {
        for i in ITENS {
            assert!(!i.nome().is_empty());
            assert_eq!(i.elemento().r.comp, ORIGEM);
        }
        assert!(matches!(ItemOrigem::Plano(OriginPlane::XY).elemento().r.geom, Geom::Plane { n, .. } if n == [0., 0., 1.]));
        assert!(matches!(ItemOrigem::Eixo(OriginAxis::Y).elemento().r.geom, Geom::Axis { d, .. } if d == [0., 1., 0.]));
        assert!(matches!(ItemOrigem::Ponto.elemento().r.geom, Geom::Point(_)));
    }
}
