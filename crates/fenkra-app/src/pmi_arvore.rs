// SPDX-License-Identifier: LGPL-2.1-or-later
//! Capturas e anotações na árvore (B368). As linhas usam os mesmos caminhos das linhas de recurso
//! (`row_label` para seleção Ctrl/Shift, F2 e realce; `drag_item` para arrastar), sem uma segunda
//! implementação. Aqui ficam só o que é próprio do PMI: reordenar por arraste e o comando
//! "Criar captura" da família do detalhamento 3D.

use crate::app::App;
use crate::i18n::tr;
use fenkra_core::pmi::PmiCapture;
use fenkra_core::Id;

/// Leva os itens `mover` (na ordem em que já estão) para a posição de `sobre`: quem desce fica
/// depois dele, quem sobe fica antes, como nas linhas de recurso. Devolve se algo mudou.
pub fn reordenar<T>(lista: &mut Vec<T>, id_de: &dyn Fn(&T) -> Id, mover: &[Id], sobre: Id) -> bool {
    if mover.contains(&sobre) {
        return false;
    }
    let Some(destino) = lista.iter().position(|x| id_de(x) == sobre) else { return false };
    let primeiro = lista.iter().position(|x| mover.contains(&id_de(x)));
    let Some(primeiro) = primeiro else { return false };
    let desce = primeiro < destino;
    let antes: Vec<Id> = lista.iter().map(id_de).collect();
    let mut grupo = vec![];
    let mut i = 0;
    while i < lista.len() {
        if mover.contains(&id_de(&lista[i])) {
            grupo.push(lista.remove(i));
        } else {
            i += 1;
        }
    }
    let Some(pos) = lista.iter().position(|x| id_de(x) == sobre) else { return false };
    let ponto = if desce { pos + 1 } else { pos };
    for (k, g) in grupo.into_iter().enumerate() {
        lista.insert(ponto + k, g);
    }
    lista.iter().map(id_de).collect::<Vec<_>>() != antes
}

/// Comando "Criar captura" do grupo Detalhamento 3D: câmera + anotações visíveis, e, no segundo
/// botão, também o que está oculto no modelo. A captura nasce numerada em sequência.
pub fn criar_captura(app: &mut App, com_modelo: bool) {
    let Some(c) = app.camera_get() else {
        app.toast(tr("Abra uma vista para capturar").to_string(), true);
        return;
    };
    let view = fenkra_core::NamedView { name: String::new(), target: c.target, yaw: c.yaw, pitch: c.pitch, dist: c.dist, ortho: c.ortho };
    let shown: Vec<Id> = crate::pmi_ui::list(app).iter().filter(|p| p.visible).map(|p| p.id).collect();
    let hidden = if com_modelo { crate::pmi_ui::oculto_agora(app) } else { vec![] };
    let id = Id(app.next_pmi_id());
    let n = crate::pmi_ui::capturas_mut(app).len() + 1;
    let nome = format!("{} {n}", tr("Captura"));
    crate::pmi_ui::capturas_mut(app).push(PmiCapture { id, name: nome.clone(), view, shown, hidden, with_model: com_modelo });
    if app.in_asm() {
        app.asm.as_mut().unwrap().modified = true;
    }
    *app.multi.borrow_mut() = vec![id];
    app.toast(format!("{nome}: {}", tr("câmera e anotações guardadas; F2 renomeia")), false);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(v: &[u64]) -> Vec<Id> {
        v.iter().map(|x| Id(*x)).collect()
    }

    #[test]
    fn arrastar_para_baixo_fica_depois_e_para_cima_fica_antes() {
        let mut l = ids(&[1, 2, 3, 4]);
        assert!(reordenar(&mut l, &|x| *x, &[Id(1)], Id(3)));
        assert_eq!(l, ids(&[2, 3, 1, 4]), "quem desce fica depois do alvo");
        assert!(reordenar(&mut l, &|x| *x, &[Id(4)], Id(2)));
        assert_eq!(l, ids(&[4, 2, 3, 1]), "quem sobe fica antes do alvo");
        assert!(!reordenar(&mut l, &|x| *x, &[Id(2)], Id(2)), "soltar em si mesmo não muda");
    }

    #[test]
    fn grupo_selecionado_anda_junto_mantendo_a_ordem() {
        let mut l = ids(&[1, 2, 3, 4, 5]);
        assert!(reordenar(&mut l, &|x| *x, &[Id(1), Id(3)], Id(5)));
        assert_eq!(l, ids(&[2, 4, 5, 1, 3]));
        let mut m = ids(&[1, 2, 3]);
        assert!(!reordenar(&mut m, &|x| *x, &[Id(2)], Id(9)), "alvo que não existe");
    }
}
