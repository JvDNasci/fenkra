// SPDX-License-Identifier: LGPL-2.1-or-later
//! B514: campos do painel do comando aberto. Cada campo de seleção (ponto, curva, eixo, plano,
//! superfície, lista de elementos) e cada parâmetro numérico se registra aqui na ordem em que o
//! painel o desenha. Daí saem três regras, iguais em todos os comandos:
//!
//! - um campo é o ativo: ao abrir, o primeiro campo vazio (ou, sem nenhum vazio, a primeira lista);
//!   o clique no 3D entra nele, sem botão de "usar a seleção";
//! - Tab passa ao campo seguinte e Shift+Tab volta; o parâmetro numérico que recebe a vez ganha o
//!   foco do teclado. Enter faz o mesmo enquanto houver um campo de seleção mais adiante; depois
//!   dele o Enter volta a concluir o comando;
//! - campo de um elemento só avança sozinho depois do clique.
//!
//! O painel é de modo imediato: a ordem completa dos campos só é conhecida no fim do quadro, então
//! as decisões que dependem do campo seguinte usam a lista do quadro anterior.
//!
//! Quem consome o clique: o primeiro campo vazio continua sendo preenchido por
//! `App::fill_from_selection` (que sabe trocar face por plano, aresta circular por centro…). Quando o
//! ativo é uma lista, ou um campo que a pessoa escolheu com Tab ou clique fora dessa ordem, o painel
//! consome o clique e o `App` não preenche nada (`painel_consome`).

use eframe::egui;

/// O que o campo recebe.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tipo {
    /// Campo de seleção no 3D. `varios` = lista que acumula a cada clique.
    Selecao { vazio: bool, varios: bool },
    /// Parâmetro numérico (caixa de expressão).
    Numero,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
enum Foco {
    /// Primeiro campo vazio; sem nenhum, a primeira lista; sem lista, nenhum.
    #[default]
    Auto,
    /// Campo escolhido pela pessoa (Tab, Enter, clique no rótulo) ou pelo avanço depois do clique.
    Campo(egui::Id),
    /// Passou do último campo: nada ativo até a pessoa escolher de novo.
    Nenhum,
}

#[derive(Clone, Default, Debug)]
struct Quadro {
    dono: u64,
    passo: u64,
    /// `inicio` foi chamado neste quadro (painel de um comando aberto)
    sessao: bool,
    atual: Vec<(egui::Id, Tipo)>,
    anterior: Vec<(egui::Id, Tipo)>,
    foco: Foco,
    /// parâmetro que deve ganhar o foco do teclado no fim do quadro
    teclado: Option<egui::Id>,
    /// tirar o foco do teclado de quem estiver com ele (a vez passou a um campo de seleção)
    soltar: bool,
    /// assinatura da seleção do 3D vista no quadro anterior
    selecao: u64,
    /// a seleção mudou neste quadro: há clique novo para o campo ativo
    novos: bool,
    consumiu: bool,
    avancou: bool,
    /// o Enter deste quadro é do painel (decidido com a lista do quadro anterior)
    enter_do_painel: bool,
    /// editor de um item de lista aberto: os campos dele não entram na ordem do painel
    suspenso: bool,
    /// publicados no fim do quadro, para o `App` ler no quadro seguinte
    consome: bool,
    enter: bool,
    tab: bool,
}

fn chave() -> egui::Id {
    egui::Id::new("campos_do_painel_b514")
}

fn com<R>(ctx: &egui::Context, f: impl FnOnce(&mut Quadro) -> R) -> R {
    ctx.data_mut(|d| f(d.get_temp_mut_or_default::<Quadro>(chave())))
}

/// Troca de quadro: o que foi registrado no quadro anterior vira a lista de referência.
fn girar(q: &mut Quadro, passo: u64) {
    if q.passo == passo {
        return;
    }
    if !q.atual.is_empty() || q.sessao {
        q.anterior = std::mem::take(&mut q.atual);
    }
    q.passo = passo;
    q.sessao = false;
    q.novos = false;
    q.consumiu = false;
    q.avancou = false;
    q.enter_do_painel = false;
}

fn vazio(t: Tipo) -> bool {
    matches!(t, Tipo::Selecao { vazio: true, .. })
}

fn lista(t: Tipo) -> bool {
    matches!(t, Tipo::Selecao { varios: true, .. })
}

fn selecao(t: Tipo) -> bool {
    matches!(t, Tipo::Selecao { .. })
}

/// Índice do campo ativo na lista dada.
fn ativo_em(l: &[(egui::Id, Tipo)], foco: Foco) -> Option<usize> {
    let auto = || l.iter().position(|x| vazio(x.1)).or_else(|| l.iter().position(|x| lista(x.1)));
    match foco {
        Foco::Nenhum => None,
        Foco::Campo(k) => l.iter().position(|x| x.0 == k).or_else(auto),
        Foco::Auto => auto(),
    }
}

/// Foco no campo seguinte (`passo` = 1) ou anterior (-1) ao ativo, na lista do quadro anterior.
fn mover(q: &mut Quadro, de: Option<usize>, passo: i32) {
    let n = q.anterior.len() as i32;
    if n == 0 {
        return;
    }
    let alvo = match de {
        Some(i) => i as i32 + passo,
        None if passo > 0 => 0,
        None => n - 1,
    };
    if alvo >= n {
        q.foco = Foco::Nenhum;
        q.soltar = true;
        q.teclado = None;
        return;
    }
    let (id, t) = q.anterior[alvo.max(0) as usize];
    q.foco = Foco::Campo(id);
    if t == Tipo::Numero {
        q.teclado = Some(id);
        q.soltar = false;
    } else {
        q.teclado = None;
        q.soltar = true;
    }
}

/// Há um campo de seleção depois do índice dado?
fn selecao_depois(l: &[(egui::Id, Tipo)], i: usize) -> bool {
    l.iter().skip(i + 1).any(|x| selecao(x.1))
}

/// Começo do painel do comando `dono`. `assinatura` resume a seleção atual do 3D (0 = vazia): quando
/// ela muda, o campo ativo recebe o que foi clicado. Também trata o Tab do quadro.
pub fn inicio(ctx: &egui::Context, dono: u64, assinatura: u64) {
    let passo = ctx.cumulative_pass_nr();
    let texto_fora = ctx.text_edit_focused();
    let focado = ctx.memory(|m| m.focused());
    let (tab, volta, enter) = ctx.input(|i| (i.key_pressed(egui::Key::Tab) && i.modifiers.is_none(), i.key_pressed(egui::Key::Tab) && i.modifiers.shift_only(), i.key_pressed(egui::Key::Enter) && i.modifiers.is_none()));
    let mut cancelar_navegacao = false;
    com(ctx, |q| {
        // o painel deixou de ser desenhado por uns quadros (o comando fechou e abriu de novo): recomeça
        let voltou = passo > q.passo + 2;
        girar(q, passo);
        if q.dono != dono || voltou {
            *q = Quadro { dono, passo, selecao: assinatura, ..Default::default() };
        } else {
            q.novos = assinatura != q.selecao && assinatura != 0;
            q.selecao = assinatura;
        }
        q.sessao = true;
        q.enter_do_painel = enter && q.enter && !texto_fora;
        // Tab só é do painel quando ninguém de fora está digitando (um parâmetro do painel pode)
        let nosso = focado.is_some_and(|f| q.anterior.iter().any(|x| x.0 == f));
        if (tab || volta) && !q.anterior.is_empty() && (!texto_fora || nosso) {
            let de = ativo_em(&q.anterior, q.foco);
            mover(q, de, if volta { -1 } else { 1 });
            q.avancou = true;
            cancelar_navegacao = true;
        }
    });
    if cancelar_navegacao {
        ctx.input_mut(|i| {
            i.consume_key(egui::Modifiers::NONE, egui::Key::Tab);
            i.consume_key(egui::Modifiers::SHIFT, egui::Key::Tab);
        });
        // o egui já marcou a navegação dele no começo do quadro: desfaz antes dos campos do painel
        ctx.memory_mut(|m| m.move_focus(egui::FocusDirection::None));
    }
}

/// Fim do painel. Devolve se algum campo consumiu a seleção do 3D (o `App` então a limpa, para o
/// clique seguinte, mesmo no mesmo elemento, chegar como novo).
pub fn fim(ctx: &egui::Context) -> bool {
    let enter = ctx.input(|i| i.key_pressed(egui::Key::Enter) && i.modifiers.is_none());
    let (consumiu, teclado, soltar) = com(ctx, |q| {
        if q.enter_do_painel && enter && !q.avancou {
            let de = ativo_em(&q.anterior, q.foco);
            mover(q, de, 1);
            q.avancou = true;
        }
        // decisões do próximo quadro, com a lista completa deste
        let l = &q.atual;
        let i = ativo_em(l, q.foco);
        let primeiro_vazio = l.iter().position(|x| vazio(x.1));
        q.consome = q.sessao && i.is_some_and(|i| lista(l[i].1) || (selecao(l[i].1) && Some(i) != primeiro_vazio));
        q.enter = q.sessao && i.is_some_and(|i| selecao(l[i].1) && selecao_depois(l, i));
        q.tab = q.sessao && !l.is_empty();
        (q.consumiu, q.teclado.take(), std::mem::take(&mut q.soltar))
    });
    if let Some(id) = teclado {
        ctx.memory_mut(|m| m.request_focus(id));
    } else if soltar {
        ctx.memory_mut(|m| {
            if let Some(f) = m.focused() {
                m.surrender_focus(f);
            }
        });
    }
    if teclado.is_some() || soltar {
        ctx.request_repaint();
    }
    consumiu
}

/// Estado de um campo de seleção neste quadro.
#[derive(Clone, Copy, Debug, Default)]
pub struct Estado {
    /// é o campo ativo (realçado; o clique no 3D entra nele)
    pub ativo: bool,
    /// há clique novo para ele consumir (só quando o `App` deixou o clique para o painel)
    pub novo: bool,
}

/// Registra um campo de seleção. `vazio` = ainda sem elemento; `varios` = lista que acumula.
pub fn selecao_campo(ctx: &egui::Context, id: egui::Id, vazio_agora: bool, varios: bool) -> Estado {
    let passo = ctx.cumulative_pass_nr();
    com(ctx, |q| {
        girar(q, passo);
        if q.suspenso {
            return Estado::default();
        }
        let tipo = Tipo::Selecao { vazio: vazio_agora, varios };
        // campo de um elemento escolhido pela pessoa que acabou de receber o clique: avança sozinho
        let antes = q.anterior.iter().position(|x| x.0 == id);
        let encheu = !varios && !vazio_agora && antes.is_some_and(|i| vazio(q.anterior[i].1));
        if q.foco == Foco::Campo(id) && encheu {
            mover(q, antes, 1);
        }
        let ativo = match q.foco {
            Foco::Nenhum => false,
            Foco::Campo(k) if q.anterior.iter().any(|x| x.0 == k) || q.atual.iter().any(|x| x.0 == k) => k == id,
            _ => {
                let vazio_antes = q.atual.iter().any(|x| vazio(x.1));
                if vazio_agora {
                    !vazio_antes
                } else if varios {
                    // a lista só é a ativa se nenhum campo estiver vazio (o vazio vem primeiro)
                    let vazio_depois = q.anterior.iter().any(|x| x.0 != id && vazio(x.1) && !q.atual.iter().any(|y| y.0 == x.0));
                    !vazio_antes && !vazio_depois && !q.atual.iter().any(|x| lista(x.1))
                } else {
                    false
                }
            }
        };
        q.atual.push((id, tipo));
        Estado { ativo, novo: ativo && q.novos && q.sessao && !q.consumiu }
    })
}

/// Liga/desliga o registro (o editor de um item de lista desenha campos que não são do painel).
pub fn suspender(ctx: &egui::Context, sim: bool) {
    com(ctx, |q| q.suspenso = sim);
}

/// O campo usou o clique novo.
pub fn consumido(ctx: &egui::Context) {
    com(ctx, |q| q.consumiu = true);
}

/// Campo de um elemento só que acabou de receber o clique pelo painel: a vez passa ao seguinte.
pub fn avancar(ctx: &egui::Context, id: egui::Id) {
    com(ctx, |q| {
        let i = q.anterior.iter().position(|x| x.0 == id);
        if i.is_some() {
            mover(q, i, 1);
        }
    });
    ctx.request_repaint();
}

/// A pessoa clicou no rótulo do campo: ele passa a ser o ativo.
pub fn focar(ctx: &egui::Context, id: egui::Id) {
    com(ctx, |q| {
        q.foco = Foco::Campo(id);
        q.soltar = true;
    });
    ctx.request_repaint();
}

/// Registra um parâmetro numérico (a caixa de texto `resp`). Clicar nele o faz ativo; Enter dentro
/// dele passa ao campo seguinte enquanto houver um campo de seleção mais adiante.
pub fn numero(ctx: &egui::Context, resp: &egui::Response) {
    let passo = ctx.cumulative_pass_nr();
    let enter = ctx.input(|i| i.key_pressed(egui::Key::Enter));
    // lidos fora do `com`: a resposta consulta a memória do contexto, que fica travada lá dentro
    let (ganhou, perdeu) = (resp.gained_focus(), resp.lost_focus());
    com(ctx, |q| {
        girar(q, passo);
        if !q.sessao || q.suspenso {
            return;
        }
        let id = resp.id;
        if ganhou {
            q.foco = Foco::Campo(id);
        }
        if perdeu && enter && !q.avancou {
            if let Some(i) = q.anterior.iter().position(|x| x.0 == id) {
                if selecao_depois(&q.anterior, i) {
                    mover(q, Some(i), 1);
                    q.avancou = true;
                }
            }
        } else if perdeu && !q.avancou && q.foco == Foco::Campo(id) {
            // saiu do parâmetro clicando em outro lugar: a vez volta ao campo automático, para o
            // clique seguinte no 3D não se perder num campo que não recebe seleção
            q.foco = Foco::Auto;
        }
        q.atual.push((id, Tipo::Numero));
    });
}

/// O campo ativo do painel aberto recebe o clique do 3D (lista, ou campo fora da ordem dos vazios):
/// o `App` não deve preencher pelos campos vazios.
pub fn painel_consome(ctx: &egui::Context) -> bool {
    ctx.data(|d| d.get_temp::<Quadro>(chave())).is_some_and(|q| q.consome && q.passo + 2 >= ctx.cumulative_pass_nr())
}

/// O Enter (sem campo de texto) passa ao campo seguinte em vez de concluir o comando.
pub fn enter_avanca(ctx: &egui::Context) -> bool {
    ctx.data(|d| d.get_temp::<Quadro>(chave())).is_some_and(|q| q.enter && q.passo + 2 >= ctx.cumulative_pass_nr())
}

/// O Tab é do painel do comando (não do atalho de ocultar).
pub fn tab_do_painel(ctx: &egui::Context) -> bool {
    ctx.data(|d| d.get_temp::<Quadro>(chave())).is_some_and(|q| q.tab && q.passo + 2 >= ctx.cumulative_pass_nr())
}

/// Assinatura da seleção para `inicio` (0 = nada selecionado).
pub fn assinatura<T: std::fmt::Debug>(itens: &[T]) -> u64 {
    use std::hash::{Hash, Hasher};
    if itens.is_empty() {
        return 0;
    }
    let mut h = std::collections::hash_map::DefaultHasher::new();
    format!("{itens:?}").hash(&mut h);
    h.finish().max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn campos(l: &[(u64, Tipo)]) -> Vec<(egui::Id, Tipo)> {
        l.iter().map(|(k, t)| (egui::Id::new(k), *t)).collect()
    }

    const VAZIO: Tipo = Tipo::Selecao { vazio: true, varios: false };
    const CHEIO: Tipo = Tipo::Selecao { vazio: false, varios: false };
    const LISTA: Tipo = Tipo::Selecao { vazio: false, varios: true };

    #[test]
    fn ativo_automatico_e_o_primeiro_vazio_senao_a_lista() {
        let l = campos(&[(1, CHEIO), (2, LISTA), (3, VAZIO), (4, Tipo::Numero)]);
        assert_eq!(ativo_em(&l, Foco::Auto), Some(2));
        let l = campos(&[(1, CHEIO), (2, LISTA), (3, CHEIO), (4, Tipo::Numero)]);
        assert_eq!(ativo_em(&l, Foco::Auto), Some(1));
        let l = campos(&[(1, CHEIO), (4, Tipo::Numero)]);
        assert_eq!(ativo_em(&l, Foco::Auto), None, "tudo preenchido e sem lista: nada ativo, o Enter conclui");
    }

    #[test]
    fn tab_anda_pelos_campos_e_o_numero_pede_o_teclado() {
        let mut q = Quadro { anterior: campos(&[(1, LISTA), (2, LISTA), (3, Tipo::Numero)]), ..Default::default() };
        let de = ativo_em(&q.anterior, q.foco);
        assert_eq!(de, Some(0));
        mover(&mut q, de, 1);
        assert_eq!(q.foco, Foco::Campo(egui::Id::new(2)));
        assert!(q.soltar && q.teclado.is_none());
        let de = ativo_em(&q.anterior, q.foco);
        mover(&mut q, de, 1);
        assert_eq!(q.teclado, Some(egui::Id::new(3)), "o parâmetro recebe o foco do teclado");
        let de = ativo_em(&q.anterior, q.foco);
        mover(&mut q, de, 1);
        assert_eq!(q.foco, Foco::Nenhum, "depois do último, nada ativo");
        let de = ativo_em(&q.anterior, q.foco);
        mover(&mut q, de, -1);
        assert_eq!(q.foco, Foco::Campo(egui::Id::new(3)), "Shift+Tab a partir do fim volta ao último");
    }
}

#[cfg(test)]
#[path = "campos_tests.rs"]
mod aceite;
