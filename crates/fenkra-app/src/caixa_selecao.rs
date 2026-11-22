// SPDX-License-Identifier: LGPL-2.1-or-later
//! Caixa de seleção padrão (B555): a lista de elementos de um comando com vários itens (arestas do
//! filete, faces da casca, curvas do loft...). Funciona como uma pequena árvore: o clique no item o
//! marca, Ctrl+clique soma ou tira da marcação, Shift+clique marca o intervalo desde o último item
//! clicado e Del (ou Backspace) tira os marcados. A lógica de marcar e remover mora aqui, sem egui,
//! para ser testada; `geo_ui::bag` só desenha e liga as teclas.
//!
//! ADR 0107, onda 0: a caixa é o widget único de todo campo de seleção. Aqui mora a parte sem egui
//! da caixa universal em árvore: somar o clique com validação (tipo errado e excesso recusados com
//! mensagem por campo), reordenar quando o descritor pede ordem, e levar as junções (subitens
//! dentro do comando) junto quando a lista muda. O desenho está em `caixa_arvore.rs`.
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

// ---------------------------------------------------------------- somar com validação

use crate::contrato::{Candidato, Entrada};

/// O que aconteceu ao somar o que foi clicado a um campo.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Soma {
    /// a primeira recusa (tipo errado, campo cheio, validador): o campo mostra a mensagem
    pub recusa: Option<String>,
    /// índices dos itens que entraram
    pub adicionados: Vec<usize>,
    /// quantos itens já estavam e saíram (clicar de novo num escolhido o tira)
    pub tirados: usize,
}

/// O clique entra no campo. Item que já está sai (como sempre); item novo passa pelo descritor: o
/// tipo, a quantidade que ainda cabe e o validador. Recusado não entra, e o motivo volta em
/// `Soma::recusa`. `cand` descreve cada item para o validador (sem ele, só o máximo é conferido).
#[allow(clippy::too_many_arguments)]
pub fn somar<T: Clone + PartialEq>(items: &mut Vec<T>, picked: &[T], min: usize, vazio: Option<&T>, mesmo: &impl Fn(&T, &T) -> bool, entrada: Option<&Entrada>, cand: Option<&dyn Fn(&T) -> Candidato>) -> Soma {
    let mut soma = Soma::default();
    let e_vazio = |y: &T| vazio == Some(y);
    for x in picked {
        if let Some(k) = items.iter().position(|y| !e_vazio(y) && mesmo(y, x)) {
            crate::geo_ui::tirar_da_lista(items, k, min, vazio);
            soma.tirados += 1;
            continue;
        }
        if let Some(e) = entrada {
            let reais: Vec<&T> = items.iter().filter(|y| !e_vazio(y)).collect();
            let erro = match cand {
                Some(f) => {
                    let no_campo: Vec<Candidato> = reais.iter().map(|y| f(y)).collect();
                    e.validar(&f(x), &no_campo, reais.len()).err()
                }
                None => e.cheio(reais.len()),
            };
            if let Some(m) = erro {
                soma.recusa.get_or_insert_with(|| m.texto());
                continue;
            }
        }
        let pos = match items.iter().position(e_vazio) {
            Some(k) => {
                items[k] = x.clone();
                k
            }
            None => {
                items.push(x.clone());
                items.len() - 1
            }
        };
        soma.adicionados.push(pos);
    }
    soma
}

/// Para onde cada item da lista `antes` foi parar em `depois` (`None` = saiu). Serve para as junções e
/// as marcas seguirem os itens quando a lista muda.
pub fn onde_foram<T>(antes: &[T], depois: &[T], mesmo: &impl Fn(&T, &T) -> bool) -> Vec<Option<usize>> {
    let mut usados = vec![false; depois.len()];
    antes
        .iter()
        .map(|a| {
            let k = (0..depois.len()).find(|k| !usados[*k] && mesmo(a, &depois[*k]));
            if let Some(k) = k {
                usados[k] = true;
            }
            k
        })
        .collect()
}

/// Sobe (`-1`) ou desce (`+1`) o item `i` na lista ordenada. Devolve o índice novo (ou `None` na ponta).
pub fn mover<T>(items: &mut [T], i: usize, passo: isize) -> Option<usize> {
    let j = i as isize + passo;
    if i >= items.len() || j < 0 || j as usize >= items.len() {
        return None;
    }
    items.swap(i, j as usize);
    Some(j as usize)
}

// ---------------------------------------------------------------- pedidos ao app

/// O que a caixa pede ao app (o painel só enxerga o documento de leitura): gravar os dados do campo e
/// expandir a cadeia tangente de um item.
#[derive(Clone, Debug, PartialEq)]
pub enum Pedido {
    /// grava as junções e a opção "seguir tangentes" do campo no documento
    Dados(fenkra_core::DadosCampo),
    /// duplo clique: o item do campo puxa a cadeia tangente dele
    Cadeia { dono: fenkra_core::Id, campo: String, indice: usize },
}

fn chave_pedidos() -> egui::Id {
    egui::Id::new("caixa_pedidos")
}

pub fn pedir(ctx: &egui::Context, p: Pedido) {
    ctx.data_mut(|d| d.get_temp_mut_or_default::<Vec<Pedido>>(chave_pedidos()).push(p));
}

/// Os pedidos feitos neste quadro, para o app aplicar.
pub fn drenar(ctx: &egui::Context) -> Vec<Pedido> {
    ctx.data_mut(|d| std::mem::take(d.get_temp_mut_or_default::<Vec<Pedido>>(chave_pedidos())))
}

fn chave_recusa(salt: &str) -> egui::Id {
    egui::Id::new(("caixa_recusa", salt))
}

/// Mensagem de validação do campo (a última recusa); vazia depois de um clique aceito.
pub fn recusa(ctx: &egui::Context, salt: &str) -> Option<String> {
    ctx.data(|d| d.get_temp::<Option<String>>(chave_recusa(salt))).flatten()
}

pub fn guardar_recusa(ctx: &egui::Context, salt: &str, m: Option<String>) {
    ctx.data_mut(|d| d.insert_temp(chave_recusa(salt), m));
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

/// Del, Backspace ou X pressionado sem modificador e sem campo de texto com o foco.
pub fn tecla_de_remover(ctx: &egui::Context) -> bool {
    !ctx.text_edit_focused() && ctx.input(|i| (i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace) || i.key_pressed(egui::Key::X)) && !i.modifiers.any())
}

/// Consome as teclas de remover, para o resto do app não agir sobre elas.
pub fn consumir_teclas_de_remover(ctx: &egui::Context) {
    ctx.input_mut(|i| {
        i.consume_key(egui::Modifiers::NONE, egui::Key::Delete);
        i.consume_key(egui::Modifiers::NONE, egui::Key::Backspace);
        i.consume_key(egui::Modifiers::NONE, egui::Key::X);
    });
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

    fn cand_i32(x: &i32) -> Candidato {
        Candidato { tipo: if *x < 100 { crate::contrato::TipoEntrada::Aresta } else { crate::contrato::TipoEntrada::Face }, origem: Some(fenkra_core::Id((*x / 10) as u64)), nome: format!("item {x}") }
    }

    #[test]
    fn somar_entra_tira_e_recusa_tipo_errado() {
        use crate::contrato::{Cardinalidade, TipoEntrada};
        let e = Entrada::nova("arestas", "Arestas", TipoEntrada::Aresta, Cardinalidade::Varios { min: 1, max: Some(3) }, 0);
        let mut it: Vec<i32> = vec![];
        let c: &dyn Fn(&i32) -> Candidato = &cand_i32;
        let s = somar(&mut it, &[1, 2], 0, None, &|a: &i32, b: &i32| a == b, Some(&e), Some(c));
        assert_eq!((it.clone(), s.adicionados.clone(), s.recusa.clone()), (vec![1, 2], vec![0, 1], None));
        // uma face (>= 100) não entra numa caixa de arestas, e a mensagem diz o porquê
        let s = somar(&mut it, &[150], 0, None, &|a: &i32, b: &i32| a == b, Some(&e), Some(c));
        assert_eq!(it, vec![1, 2]);
        let m = s.recusa.expect("recusa");
        assert!(m.contains("aresta") && m.contains("face"), "{m}");
        // clicar de novo num escolhido o tira
        let s = somar(&mut it, &[1], 0, None, &|a: &i32, b: &i32| a == b, Some(&e), Some(c));
        assert_eq!((it.clone(), s.tirados), (vec![2], 1));
        // o máximo de 3 itens: o quarto é recusado
        somar(&mut it, &[3, 4], 0, None, &|a: &i32, b: &i32| a == b, Some(&e), Some(c));
        assert_eq!(it, vec![2, 3, 4]);
        let s = somar(&mut it, &[5], 0, None, &|a: &i32, b: &i32| a == b, Some(&e), Some(c));
        assert_eq!(it.len(), 3);
        assert!(s.recusa.unwrap().contains('3'));
    }

    #[test]
    fn somar_ocupa_o_item_vazio_antes_de_acrescentar() {
        let mut it = vec![-1, 7];
        let s = somar(&mut it, &[9], 2, Some(&-1), &|a: &i32, b: &i32| a == b, None, None);
        assert_eq!((it, s.adicionados), (vec![9, 7], vec![0]));
    }

    #[test]
    fn onde_foram_segue_itens_ao_tirar_e_reordenar() {
        let antes = vec![10, 11, 12, 13];
        let depois = vec![13, 11, 12];
        assert_eq!(onde_foram(&antes, &depois, &|a: &i32, b: &i32| a == b), vec![None, Some(1), Some(2), Some(0)]);
    }

    #[test]
    fn mover_troca_com_o_vizinho_e_para_nas_pontas() {
        let mut it = vec!['a', 'b', 'c'];
        assert_eq!(mover(&mut it, 1, -1), Some(0));
        assert_eq!(it, vec!['b', 'a', 'c']);
        assert_eq!(mover(&mut it, 0, -1), None);
        assert_eq!(mover(&mut it, 2, 1), None);
        assert_eq!(mover(&mut it, 1, 1), Some(2));
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
