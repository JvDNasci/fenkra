// SPDX-License-Identifier: LGPL-2.1-or-later
//! Caixa de seleção padrão (B555): a lista de elementos de um comando com vários itens (arestas do
//! filete, faces da casca, curvas do loft...). Funciona como uma pequena árvore: o clique no item o
//! marca, Ctrl+clique soma ou tira da marcação, Shift+clique marca o intervalo desde o último item
//! clicado e Del (ou Backspace) tira os marcados. A lógica de marcar e remover mora aqui, sem egui,
//! para ser testada; `geo_ui::bag` só desenha e liga as teclas.
//!
//! Os marcados também podem acender no 3D: quem sabe a posição de cada item chama
//! `publicar_realce` e a viewport desenha pelo `desenhar_realce`.

use crate::picking::Pick;
use eframe::egui;

/// Itens marcados da caixa (índices na lista) e o item de partida do intervalo (Shift).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Marcas {
    pub marcados: Vec<usize>,
    pub ancora: Option<usize>,
}

impl Marcas {
    /// Clique no item `i`: simples marca só ele; Ctrl soma ou tira; Shift marca o intervalo da
    /// âncora até `i` (com Ctrl, soma o intervalo ao que já estava marcado).
    pub fn clicar(&mut self, i: usize, ctrl: bool, shift: bool) {
        if shift {
            let a = self.ancora.unwrap_or(i);
            if !ctrl {
                self.marcados.clear();
            }
            for k in a.min(i)..=a.max(i) {
                if !self.marcados.contains(&k) {
                    self.marcados.push(k);
                }
            }
            // o Shift não move a âncora: o intervalo seguinte parte do mesmo item
            self.ancora.get_or_insert(i);
        } else if ctrl {
            if let Some(p) = self.marcados.iter().position(|x| *x == i) {
                self.marcados.remove(p);
            } else {
                self.marcados.push(i);
            }
            self.ancora = Some(i);
        } else {
            self.marcados = vec![i];
            self.ancora = Some(i);
        }
        self.marcados.sort_unstable();
    }

    pub fn esta(&self, i: usize) -> bool {
        self.marcados.contains(&i)
    }

    pub fn vazia(&self) -> bool {
        self.marcados.is_empty()
    }

    pub fn limpar(&mut self) {
        *self = Self::default();
    }

    /// A lista mudou de tamanho: some o que ficou além do fim.
    pub fn ajustar(&mut self, len: usize) {
        self.marcados.retain(|i| *i < len);
        if self.ancora.is_some_and(|a| a >= len) {
            self.ancora = None;
        }
    }
}

/// Tira os itens marcados, do fim para o começo (os índices anteriores continuam valendo). Abaixo do
/// mínimo o item volta ao marcador de vazio (`vazio`), como o ✖ de cada linha. Devolve quantos itens
/// foram tirados ou esvaziados e limpa a marcação.
pub fn remover_marcados<T: Clone>(items: &mut Vec<T>, marcas: &mut Marcas, min: usize, vazio: Option<&T>) -> usize {
    let mut n = 0;
    let mut idx = std::mem::take(&mut marcas.marcados);
    idx.sort_unstable();
    idx.dedup();
    for i in idx.into_iter().rev() {
        if i < items.len() {
            crate::geo_ui::tirar_da_lista(items, i, min, vazio);
            n += 1;
        }
    }
    marcas.limpar();
    n
}

fn chave_marcas(salt: &str) -> egui::Id {
    egui::Id::new(("caixa_marcas", salt))
}

/// Marcação guardada de uma caixa (vazia se nunca foi clicada).
pub fn marcas(ctx: &egui::Context, salt: &str) -> Marcas {
    ctx.data(|d| d.get_temp::<Marcas>(chave_marcas(salt))).unwrap_or_default()
}

/// Guarda a marcação de uma caixa e avisa a janela que há itens marcados (o Del é da caixa, não da
/// árvore de comandos).
pub fn guardar_marcas(ctx: &egui::Context, salt: &str, m: Marcas) {
    if !m.vazia() {
        let passo = ctx.cumulative_pass_nr();
        ctx.data_mut(|d| d.insert_temp(egui::Id::new("caixa_marcas_vivas"), passo));
    }
    ctx.data_mut(|d| d.insert_temp(chave_marcas(salt), m));
}

/// Há uma caixa com itens marcados desenhada neste quadro ou no anterior: o Del vai para ela.
pub fn ha_marcados(ctx: &egui::Context) -> bool {
    let passo = ctx.cumulative_pass_nr();
    ctx.data(|d| d.get_temp::<u64>(egui::Id::new("caixa_marcas_vivas"))).is_some_and(|p| passo.saturating_sub(p) <= 2)
}

/// Del ou Backspace pressionado sem modificador e sem campo de texto com o foco.
pub fn tecla_de_remover(ctx: &egui::Context) -> bool {
    !ctx.text_edit_focused() && ctx.input(|i| (i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace)) && !i.modifiers.any())
}

fn chave_realce() -> egui::Id {
    egui::Id::new("caixa_realce")
}

/// Pontos médios (arestas) dos itens marcados, para a viewport acender. Lista vazia apaga o realce.
pub fn publicar_realce(ctx: &egui::Context, pontos: Vec<[f64; 3]>) {
    let passo = ctx.cumulative_pass_nr();
    ctx.data_mut(|d| d.insert_temp(chave_realce(), (passo, pontos)));
}

/// Desenha, por cima da viewport, as arestas marcadas na caixa (só enquanto a caixa está aberta).
pub fn desenhar_realce(ctx: &egui::Context, painter: &egui::Painter, r: &fenkra_solidos::RecomputeResult, cam: &crate::render::Camera, rect: egui::Rect) {
    let passo = ctx.cumulative_pass_nr();
    let Some((p, pontos)) = ctx.data(|d| d.get_temp::<(u64, Vec<[f64; 3]>)>(chave_realce())) else { return };
    if passo.saturating_sub(p) > 2 {
        return;
    }
    for q in pontos {
        for (bi, b) in r.bodies.iter().enumerate() {
            let m = &b.mesh;
            let d = |v: &[f32]| (0..3).map(|k| (v[k] as f64 - q[k]).powi(2)).sum::<f64>().sqrt();
            if let Some(e) = m.edge_mids.chunks_exact(3).position(|v| d(v) < 1e-3) {
                crate::picking::draw(painter, r, cam, rect, &Pick::Edge { body: bi, edge: e, mid: q.map(|x| x as f32) }, true);
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lista() -> Vec<i32> {
        vec![10, 11, 12, 13, 14]
    }

    #[test]
    fn clique_simples_marca_so_o_item() {
        let mut m = Marcas::default();
        m.clicar(1, false, false);
        m.clicar(3, false, false);
        assert_eq!(m.marcados, vec![3]);
        assert_eq!(m.ancora, Some(3));
    }

    #[test]
    fn ctrl_soma_e_tira_da_marcacao() {
        let mut m = Marcas::default();
        m.clicar(1, false, false);
        m.clicar(3, true, false);
        assert_eq!(m.marcados, vec![1, 3]);
        m.clicar(1, true, false);
        assert_eq!(m.marcados, vec![3]);
    }

    #[test]
    fn shift_marca_o_intervalo_a_partir_da_ancora() {
        let mut m = Marcas::default();
        m.clicar(1, false, false);
        m.clicar(3, false, true);
        assert_eq!(m.marcados, vec![1, 2, 3]);
        // outro Shift parte da mesma âncora e troca o intervalo
        m.clicar(0, false, true);
        assert_eq!(m.marcados, vec![0, 1]);
        // Ctrl+Shift soma ao que já estava
        m.clicar(4, true, true);
        assert_eq!(m.marcados, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn shift_sem_ancora_marca_so_o_clicado() {
        let mut m = Marcas::default();
        m.clicar(2, false, true);
        assert_eq!(m.marcados, vec![2]);
    }

    #[test]
    fn del_tira_os_marcados_e_mantem_a_ordem() {
        let mut it = lista();
        let mut m = Marcas::default();
        m.clicar(1, false, false);
        m.clicar(3, true, false);
        let n = remover_marcados(&mut it, &mut m, 0, None);
        assert_eq!(n, 2);
        assert_eq!(it, vec![10, 12, 14]);
        assert!(m.vazia());
    }

    #[test]
    fn del_abaixo_do_minimo_volta_ao_marcador_de_vazio() {
        let mut it = vec![7, 8];
        let mut m = Marcas::default();
        m.clicar(0, false, false);
        m.clicar(1, false, true);
        let n = remover_marcados(&mut it, &mut m, 2, Some(&-1));
        assert_eq!(n, 2);
        assert_eq!(it, vec![-1, -1], "com mínimo 2 os itens viram vazios em vez de sumir");
    }

    #[test]
    fn del_misto_tira_ate_o_minimo_e_esvazia_o_resto() {
        let mut it = vec![1, 2, 3];
        let mut m = Marcas::default();
        m.clicar(0, false, false);
        m.clicar(2, false, true);
        remover_marcados(&mut it, &mut m, 2, Some(&-1));
        // 3 -> removido (3 > 2); depois 2 itens = mínimo, então 2 e 1 viram vazio
        assert_eq!(it, vec![-1, -1]);
    }

    #[test]
    fn ajustar_descarta_o_que_passou_do_fim() {
        let mut m = Marcas::default();
        m.clicar(4, false, false);
        m.clicar(1, true, false);
        m.ajustar(3);
        assert_eq!(m.marcados, vec![1]);
        assert_eq!(m.ancora, Some(1));
    }
}
