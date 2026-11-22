// SPDX-License-Identifier: LGPL-2.1-or-later
//! ADR 0107, onda 0: a caixa de seleção universal em árvore, o widget único de todo campo de seleção
//! de vários itens (a evolução de `geo_ui::bag`, que agora a chama com as opções de sempre).
//!
//! - **Nós por origem**: quando o campo diz de onde cada item vem (`Caixa::origem`), os itens ficam
//!   sob um nó por origem ("Sketch 2", "Extrusão 1", "Wireframe"), como numa pequena árvore. Em campo
//!   ordenado a lista é plana e numerada, com ▲▼.
//! - **Entra do 3D e da árvore de recursos** pelo mesmo caminho (`picked`).
//! - **Clique no item marca; Ctrl soma; Shift marca o intervalo; Del ou X tira os marcados.**
//! - **Validação na hora, com a mensagem no campo**: tipo errado e excesso são recusados com o motivo
//!   (`caixa_selecao::somar`), nada de erro só depois do OK.
//! - **Junção na hora como subitem**: em campo de curvas, itens unidos aparecem como subitens de um nó
//!   "Junção" dentro do próprio campo; botão direito "Unir" e "Separar". A lista fica gravada no dado do
//!   comando (`Document::campos`), sem recurso Junção escondido na árvore.
//! - **Cadeia tangente como recurso do campo**: a opção "seguir tangentes" e o duplo clique no item
//!   pegam a cadeia (o app expande com o código de sempre e grava a junção).
//!
//! Cores só pelo tema.

use crate::caixa_selecao::{self as cs, Marcas, Pedido};
use crate::contrato::{Candidato, Entrada, TipoEntrada};
use crate::geo_ui::id_do_campo;
use crate::i18n::tr;
use eframe::egui::{self, RichText};
use fenkra_core::{DadosCampo, Id};

/// Tudo o que descreve um campo de seleção de vários itens (menos os itens, que vão à parte).
pub struct Caixa<'a, T> {
    pub salt: &'a str,
    pub label: &'a str,
    pub min: usize,
    /// o campo no descritor do comando, quando o comando o declara
    pub entrada: Option<&'a Entrada>,
    /// comando dono do campo (para gravar as junções)
    pub dono: Option<Id>,
    /// dados do campo no documento (junções e cadeia tangente)
    pub dados: Option<&'a DadosCampo>,
    /// tipos do que a seleção atual tem, para recusar com mensagem o que o campo não aceita
    pub clicados: Vec<TipoEntrada>,
    pub nome: &'a dyn Fn(&T) -> String,
    /// de onde o item vem (nó da árvore); `None` = lista plana
    pub origem: Option<&'a dyn Fn(&T) -> Option<String>>,
    /// o item como o validador do campo o vê
    pub cand: Option<&'a dyn Fn(&T) -> Candidato>,
    pub blank: &'a dyn Fn() -> T,
    pub vazio: Option<T>,
    pub mesmo: &'a dyn Fn(&T, &T) -> bool,
    /// B577: o documento, para a árvore de subelementos (face, aresta, vértice) do modelo calculado
    pub navegar: Option<&'a fenkra_core::Document>,
}

#[derive(Clone, Copy)]
enum Linha {
    Item(usize),
    Juncao(usize),
}

/// As linhas da caixa na ordem de exibição. Os itens de uma junção saem da lista solta e aparecem sob o
/// nó da junção, que fica onde estava o primeiro membro.
fn linhas(n: usize, dados: &DadosCampo, com_juncao: bool) -> Vec<Linha> {
    let mut vistos = vec![false; dados.juncoes.len()];
    let mut v = vec![];
    for i in 0..n {
        match dados.grupo_de(i).filter(|_| com_juncao) {
            Some(g) => {
                if !vistos[g] {
                    vistos[g] = true;
                    v.push(Linha::Juncao(g));
                }
            }
            None => v.push(Linha::Item(i)),
        }
    }
    v
}

/// O que as linhas de uma caixa precisam mudar durante o quadro.
struct Acoes {
    del: Option<usize>,
    mover: Option<(usize, isize)>,
    cadeia: Option<usize>,
    aberto: Option<usize>,
}

#[allow(clippy::too_many_arguments)]
fn linha_item<T: Clone + PartialEq + Send + Sync + 'static>(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    c: &Caixa<T>,
    items: &mut [T],
    i: usize,
    marcas: &mut Marcas,
    dados: &mut DadosCampo,
    a: &mut Acoes,
    ordenado: bool,
    com_juncao: bool,
    campo: egui::Id,
    edit: &mut dyn FnMut(&mut egui::Ui, usize, &mut T),
    prefixo: Option<&str>,
) {
    // B555: o item marcado ganha o fundo de seleção do tema
    let fundo = if marcas.esta(i) { ui.visuals().selection.bg_fill } else { egui::Color32::TRANSPARENT };
    let n_itens = items.len();
    egui::Frame::NONE.fill(fundo).corner_radius(3.).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("{}.", i + 1)).weak());
            let mut n = (c.nome)(&items[i]);
            if let Some(p) = prefixo.and_then(|p| n.strip_prefix(&format!("{p} · ")).map(|r| r.to_string())) {
                n = p;
            }
            let w = (ui.available_width() - if ordenado { 88. } else { 52. }).max(40.);
            let vazio = c.vazio.as_ref() == Some(&items[i]);
            let txt = if vazio { RichText::new(n).weak() } else { RichText::new(n) };
            let r = ui.add_sized([w, 18.], egui::Label::new(txt).truncate().sense(egui::Sense::click())).on_hover_text(tr("Clique marca; Ctrl soma ou tira da marcação; Shift marca o intervalo; Del ou X tira os marcados."));
            if r.clicked() {
                let (ctrl, shift) = ui.input(|k| (k.modifiers.command, k.modifiers.shift));
                marcas.clicar(i, ctrl, shift);
                crate::campos::focar(ctx, campo);
            }
            if r.double_clicked() && com_juncao {
                a.cadeia = Some(i);
            }
            r.context_menu(|ui| {
                if !marcas.esta(i) {
                    marcas.clicar(i, false, false);
                }
                if com_juncao {
                    let alvo = marcas.marcados.clone();
                    if alvo.len() >= 2 && ui.button(tr("Unir (junção)")).on_hover_text(tr("Une os itens marcados numa junção, como subitens dentro deste campo.")).clicked() {
                        dados.unir(&alvo);
                        ui.close();
                    }
                    if alvo.iter().any(|k| dados.grupo_de(*k).is_some()) && ui.button(tr("Separar")).on_hover_text(tr("Desfaz a junção dos itens marcados.")).clicked() {
                        dados.separar(&alvo);
                        ui.close();
                    }
                }
                if ui.button(tr("Tirar")).clicked() {
                    a.del = Some(i);
                    ui.close();
                }
            });
            if ordenado {
                if ui.add_enabled(i > 0, egui::Button::new("▲").small()).on_hover_text(tr("Sobe este item na ordem")).clicked() {
                    a.mover = Some((i, -1));
                }
                if ui.add_enabled(i + 1 < n_itens, egui::Button::new("▼").small()).on_hover_text(tr("Desce este item na ordem")).clicked() {
                    a.mover = Some((i, 1));
                }
            }
            if ui.small_button("✎").on_hover_text(tr("Trocar este item pela lista ▾")).clicked() {
                a.aberto = if a.aberto == Some(i) { None } else { Some(i) };
            }
            if ui.small_button("✖").on_hover_text(tr("remover")).clicked() {
                a.del = Some(i);
            }
        });
    });
    if a.aberto == Some(i) {
        // o editor do item não é um campo do painel: não entra na ordem do Tab
        crate::campos::suspender(ctx, true);
        ui.indent(("item", i), |ui| edit(ui, i, &mut items[i]));
        crate::campos::suspender(ctx, false);
    }
}

/// A caixa de seleção universal. `items` é a lista do campo no comando, `picked` o que o clique do 3D
/// ou da árvore de recursos ofereceu a este campo, `edit` desenha o editor de um item (✎).
pub fn caixa<T: Clone + PartialEq + Send + Sync + 'static>(ui: &mut egui::Ui, c: Caixa<T>, items: &mut Vec<T>, picked: &[T], edit: &mut dyn FnMut(&mut egui::Ui, usize, &mut T)) {
    let ctx = ui.ctx().clone();
    let id = id_do_campo(c.salt);
    crate::campos::nomear(&ctx, id, c.salt);
    let e_vazio = |x: &T| c.vazio.as_ref() == Some(x);
    let reais = items.iter().filter(|x| !e_vazio(x)).count();
    // vazio = abaixo do mínimo (uma lista opcional, de mínimo zero, nunca toma a vez de um campo vazio)
    let est = crate::campos::selecao_campo(&ctx, id, reais < c.min, true);
    let ordenado = c.entrada.is_some_and(|e| e.ordenado);
    let com_juncao = c.entrada.is_some_and(|e| e.juncao) && c.dono.is_some();
    let antes_do_quadro = items.clone();
    let mut dados: DadosCampo = match (c.dados, c.dono, c.entrada) {
        (Some(d), _, _) => d.clone(),
        (None, Some(dono), Some(e)) => DadosCampo::novo(dono, e.id),
        _ => DadosCampo::novo(Id(0), c.salt),
    };
    let dados_antes = dados.clone();

    if est.novo {
        if !picked.is_empty() {
            let soma = cs::somar(items, picked, c.min, c.vazio.as_ref(), &c.mesmo, c.entrada, c.cand);
            // cadeia tangente: vários itens num clique só (a cadeia) entram unidos, como subitens
            if com_juncao && dados.tangentes && soma.adicionados.len() >= 2 {
                dados.unir(&soma.adicionados);
            }
            cs::guardar_recusa(&ctx, c.salt, soma.recusa);
            crate::campos::consumido(&ctx);
            // os índices mudaram: a marcação da caixa recomeça (B555)
            cs::guardar_marcas(&ctx, c.salt, Default::default());
        } else if let Some(m) = c.entrada.and_then(|e| crate::contrato::recusa_por_tipo(e, &c.clicados)) {
            // clicou em algo que o campo não aceita: não entra, e o campo diz por quê
            cs::guardar_recusa(&ctx, c.salt, Some(m.texto()));
            crate::campos::consumido(&ctx);
        }
    }
    let mut marcas = cs::marcas(&ctx, c.salt);
    marcas.ajustar(items.len());
    let reais = items.iter().filter(|x| !e_vazio(x)).count();
    let tipo_cor = c.entrada.map(|e| e.tipo.espera()).unwrap_or(crate::entradas::Espera::Superficie);
    let cor = if est.ativo { tipo_cor.cor() } else { ui.visuals().widgets.noninteractive.bg_stroke.color };
    // pendências das ondas 1 e 2 (ADR 0107): o rótulo e a dica do campo declarado saem do descritor
    let rotulo = c.entrada.map_or(c.label, |e| e.rotulo);
    let dica = c.entrada.map(|e| e.dica).filter(|d| !d.is_empty()).map(|d| format!("{}

", tr(d))).unwrap_or_default();
    ui.horizontal(|ui| {
        let txt = format!("{} ({reais})", tr(rotulo));
        let r = if est.ativo {
            ui.add(egui::Label::new(RichText::new(format!("▶ {txt}")).strong().color(cor)).sense(egui::Sense::click())).on_hover_text(format!("{dica}{}", tr("Campo ativo: cada clique no 3D ou na árvore soma um elemento; clicar de novo num escolhido o tira.")))
        } else {
            ui.add(egui::Label::new(txt).sense(egui::Sense::click())).on_hover_text(format!("{dica}{}", tr("Clique para escolher este campo: o próximo clique no 3D vem para cá (Tab passa ao seguinte).")))
        };
        if r.clicked() {
            crate::campos::focar(&ctx, id);
        }
        if ordenado {
            ui.label(RichText::new(tr("a ordem importa")).weak().small());
        }
        if com_juncao {
            let mut t = dados.tangentes;
            if ui.checkbox(&mut t, tr("seguir tangentes")).on_hover_text(tr("Um clique numa curva leva junto as vizinhas tangentes, que entram unidas como subitens de uma junção. O duplo clique no item faz o mesmo uma vez.")).changed() {
                dados.tangentes = t;
            }
        }
    });

    let aberto_key = egui::Id::new(("lista_item_aberto", c.salt));
    let mut acoes = Acoes { del: None, mover: None, cadeia: None, aberto: ctx.data(|d| d.get_temp::<Option<usize>>(aberto_key)).flatten() };
    let mut novo = false;
    let quadro = egui::Frame::NONE.stroke(egui::Stroke::new(if est.ativo { 1.5 } else { 1. }, cor)).corner_radius(4.).inner_margin(4.);
    let resp = quadro
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            if items.is_empty() {
                ui.label(RichText::new(tr("nenhum elemento: clique no 3D ou na árvore")).weak().small());
            }
            // nós por origem: os itens consecutivos da mesma origem ficam sob o mesmo nó (a ordem dos
            // dados não muda); em campo ordenado a lista é plana e numerada
            let rows = linhas(items.len(), &dados, com_juncao);
            let origem_de = |l: &Linha, items: &[T], dados: &DadosCampo| -> Option<String> {
                let f = c.origem?;
                if ordenado {
                    return None;
                }
                let i = match l {
                    Linha::Item(i) => *i,
                    Linha::Juncao(g) => *dados.juncoes[*g].first()?,
                };
                f(&items[i])
            };
            let mut corridas: Vec<(Option<String>, Vec<Linha>)> = vec![];
            for l in rows {
                let o = origem_de(&l, items, &dados);
                match corridas.last_mut() {
                    Some((ou, v)) if *ou == o => v.push(l),
                    _ => corridas.push((o, vec![l])),
                }
            }
            for (ci, (origem, ls)) in corridas.iter().enumerate() {
                let mut desenha = |ui: &mut egui::Ui| {
                    for l in ls {
                        match *l {
                            Linha::Item(i) => linha_item(ui, &ctx, &c, items, i, &mut marcas, &mut dados, &mut acoes, ordenado, com_juncao, id, edit, origem.as_deref()),
                            Linha::Juncao(g) => {
                                let membros = dados.juncoes[g].clone();
                                let n_itens = items.len();
                                let validos: Vec<usize> = membros.iter().copied().filter(|k| *k < n_itens).collect();
                                let h = egui::CollapsingHeader::new(RichText::new(format!("⛓ {} ({})", tr("Junção"), membros.len())).strong()).id_salt((c.salt, "juncao", g)).default_open(true).show(ui, |ui| {
                                    for k in validos.iter().copied() {
                                        linha_item(ui, &ctx, &c, items, k, &mut marcas, &mut dados, &mut acoes, ordenado, com_juncao, id, edit, origem.as_deref());
                                    }
                                });
                                h.header_response.context_menu(|ui| {
                                    if ui.button(tr("Separar")).clicked() {
                                        dados.separar(&membros);
                                        ui.close();
                                    }
                                });
                            }
                        }
                    }
                };
                match origem {
                    Some(o) => {
                        egui::CollapsingHeader::new(RichText::new(format!("{o} ({})", ls.len())).strong()).id_salt((c.salt, "origem", ci)).default_open(true).show(ui, |ui| desenha(ui));
                    }
                    None => desenha(ui),
                }
            }
            if let Some(m) = cs::recusa(&ctx, c.salt) {
                ui.label(RichText::new(format!("✖ {m}")).small().color(crate::theme::st_err()));
            }
            // B577: cada nível do modelo (elemento, face, aresta, vértice) clicável e expansível
            if let (Some(doc), Some(e)) = (c.navegar, c.entrada) {
                if crate::caixa_sub::vale_para(e.tipo) {
                    crate::caixa_sub::navegador(ui, &ctx, doc, c.salt);
                }
            }
            if est.ativo {
                ui.label(RichText::new(tr("Clique no 3D ou na árvore para somar; clique de novo num escolhido para tirar.")).weak().small());
            }
            ui.horizontal_wrapped(|ui| {
                if ui.small_button(tr("+ pela lista")).on_hover_text(tr("Acrescenta um item para escolher pela lista ▾, sem o 3D.")).clicked() {
                    novo = true;
                }
                if reais > 0 && ui.small_button(tr("Limpar")).clicked() {
                    items.truncate(c.min);
                    if let Some(v) = &c.vazio {
                        for x in items.iter_mut() {
                            *x = v.clone();
                        }
                    }
                    acoes.aberto = None;
                    marcas.limpar();
                    cs::guardar_recusa(&ctx, c.salt, None);
                }
                if com_juncao {
                    let m = marcas.marcados.clone();
                    if ui.add_enabled(m.len() >= 2, egui::Button::new(format!("⛓ {}", tr("Unir marcados"))).small()).on_hover_text(tr("Une os itens marcados numa junção (botão direito no item também).")).clicked() {
                        dados.unir(&m);
                    }
                    if ui.add_enabled(m.iter().any(|k| dados.grupo_de(*k).is_some()), egui::Button::new(tr("Separar")).small()).clicked() {
                        dados.separar(&m);
                    }
                }
            });
        })
        .response;
    if resp.interact(egui::Sense::click()).clicked() {
        crate::campos::focar(&ctx, id);
    }
    if let Some(i) = acoes.del {
        crate::geo_ui::tirar_da_lista(items, i, c.min, c.vazio.as_ref());
        acoes.aberto = None;
        // o índice tirado desloca os seguintes: a marcação recomeça
        marcas.limpar();
    }
    if let Some((i, passo)) = acoes.mover {
        if let Some(j) = cs::mover(items, i, passo) {
            marcas.limpar();
            marcas.clicar(j, false, false);
        }
    }
    // B555: Del, Backspace ou X tiram os marcados, só com a caixa ativa ou sob o mouse e sem texto em edição
    let sobre = ctx.input(|i| i.pointer.hover_pos()).is_some_and(|p| resp.rect.contains(p));
    if !marcas.vazia() && (est.ativo || sobre) && cs::tecla_de_remover(&ctx) {
        cs::remover_marcados(items, &mut marcas, c.min, c.vazio.as_ref());
        acoes.aberto = None;
        cs::consumir_teclas_de_remover(&ctx);
    }
    marcas.ajustar(items.len());
    cs::guardar_marcas(&ctx, c.salt, marcas);
    if novo {
        items.push((c.blank)());
        acoes.aberto = Some(items.len() - 1);
    }
    ctx.data_mut(|d| d.insert_temp(aberto_key, acoes.aberto));

    // as junções seguem os itens quando a lista muda (item tirado, reordenado, trocado)
    if com_juncao && *items != antes_do_quadro {
        dados.remapear(&cs::onde_foram(&antes_do_quadro, items, &c.mesmo));
    }
    if let (true, Some(dono), Some(e)) = (com_juncao, c.dono, c.entrada) {
        if dados != dados_antes {
            cs::pedir(&ctx, Pedido::Dados(DadosCampo { dono, campo: e.id.into(), ..dados }));
        }
        if let Some(i) = acoes.cadeia {
            cs::pedir(&ctx, Pedido::Cadeia { dono, campo: e.id.into(), indice: i });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linhas_agrupam_os_membros_da_juncao_no_lugar_do_primeiro() {
        let mut d = DadosCampo::novo(Id(1), "g_rc");
        d.unir(&[1, 3]);
        let l = linhas(5, &d, true);
        let txt: Vec<String> = l
            .iter()
            .map(|x| match x {
                Linha::Item(i) => format!("i{i}"),
                Linha::Juncao(g) => format!("j{g}"),
            })
            .collect();
        assert_eq!(txt, vec!["i0", "j0", "i2", "i4"]);
        // sem campo de junção, a lista é a de sempre
        assert_eq!(linhas(5, &d, false).len(), 5);
    }

    fn quadro_de_teste(f: impl FnOnce(&mut egui::Ui)) {
        let ctx = egui::Context::default();
        let raw = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(600., 600.))), ..Default::default() };
        let mut f = Some(f);
        let mut saida = ctx.run_ui(raw, |root| {
            let mut ui = root.new_child(egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(egui::pos2(8., 8.), egui::vec2(380., 580.))));
            if let Some(f) = f.take() {
                f(&mut ui);
            }
        });
        saida.textures_delta.clear();
    }

    #[test]
    fn a_caixa_desenha_em_arvore_sem_entrar_em_panico() {
        quadro_de_teste(|ui| {
            let mut items = vec![1, 2, 3, 4];
            let mut d = DadosCampo::novo(Id(9), "g_rc");
            d.unir(&[1, 2]);
            let e = Entrada::nova("g_rc", "Curvas", TipoEntrada::Curva, crate::contrato::Cardinalidade::Varios { min: 1, max: None }, 4).com_juncao();
            let nome = |x: &i32| format!("Sketch {} · linha {}", x / 2, x);
            let origem = |x: &i32| Some(format!("Sketch {}", x / 2));
            let blank = || 0;
            let mesmo = |a: &i32, b: &i32| a == b;
            let c = Caixa { salt: "g_rc", label: "Curvas", min: 1, entrada: Some(&e), dono: Some(Id(9)), dados: Some(&d), clicados: vec![], nome: &nome, origem: Some(&origem), cand: None, blank: &blank, vazio: None, mesmo: &mesmo, navegar: None };
            caixa(ui, c, &mut items, &[], &mut |_, _, _| {});
            assert_eq!(items, vec![1, 2, 3, 4]);
        });
    }

    fn tecla(key: egui::Key) -> egui::Event {
        egui::Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::NONE }
    }

    /// Um quadro da caixa "g_x" (lista de números) como o painel a desenha.
    fn quadro_da_caixa(ctx: &egui::Context, dono_sel: u64, items: &mut Vec<i32>, clicados: Vec<TipoEntrada>, eventos: Vec<egui::Event>) {
        let raw = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(600., 600.))), events: eventos, ..Default::default() };
        let mut saida = ctx.run_ui(raw, |root| {
            let mut ui = root.new_child(egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(egui::pos2(8., 8.), egui::vec2(380., 580.))));
            crate::campos::inicio(ui.ctx(), 1, dono_sel);
            let e = Entrada::nova("g_x", "Arestas", TipoEntrada::Aresta, crate::contrato::Cardinalidade::Varios { min: 0, max: Some(4) }, items.len());
            let nome = |x: &i32| format!("item {x}");
            let blank = || 0;
            let mesmo = |a: &i32, b: &i32| a == b;
            let c = Caixa { salt: "g_x", label: "Arestas", min: 0, entrada: Some(&e), dono: Some(Id(1)), dados: None, clicados: clicados.clone(), nome: &nome, origem: None, cand: None, blank: &blank, vazio: None, mesmo: &mesmo, navegar: None };
            caixa(&mut ui, c, items, &[], &mut |_, _, _| {});
            crate::campos::fim(ui.ctx());
        });
        saida.textures_delta.clear();
    }

    #[test]
    fn del_e_x_tiram_os_marcados_da_caixa() {
        let ctx = egui::Context::default();
        let mut items = vec![1, 2, 3];
        quadro_da_caixa(&ctx, 0, &mut items, vec![], vec![]);
        cs::guardar_marcas(&ctx, "g_x", Marcas { marcados: vec![1], ancora: Some(1) });
        quadro_da_caixa(&ctx, 0, &mut items, vec![], vec![tecla(egui::Key::Delete)]);
        assert_eq!(items, vec![1, 3], "Del tira o item marcado");
        cs::guardar_marcas(&ctx, "g_x", Marcas { marcados: vec![0, 1], ancora: Some(0) });
        quadro_da_caixa(&ctx, 0, &mut items, vec![], vec![tecla(egui::Key::X)]);
        assert!(items.is_empty(), "X também tira os marcados: {items:?}");
    }

    #[test]
    fn clique_de_tipo_errado_nao_entra_e_o_campo_diz_por_que() {
        let ctx = egui::Context::default();
        let mut items = vec![1];
        quadro_da_caixa(&ctx, 0, &mut items, vec![], vec![]);
        // a seleção mudou para um ponto: a caixa de arestas não o aceita
        quadro_da_caixa(&ctx, 7, &mut items, vec![TipoEntrada::Ponto], vec![]);
        assert_eq!(items, vec![1]);
        let m = cs::recusa(&ctx, "g_x").expect("mensagem no campo");
        assert!(m.contains("aresta") && m.contains("ponto"), "{m}");
    }
}
