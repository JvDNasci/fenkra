// SPDX-License-Identifier: LGPL-2.1-or-later
//! Caixa do conjunto de fixadores (B375). O modelo mora em `fenkra_montagem::conjuntos`; aqui ficam o
//! painel, as marcas na vista, a cerca e as regras por cor.
//!
//! Enquanto a caixa está aberta, cada mudança vale de verdade: o conjunto é sincronizado e as peças
//! aparecem na vista, e é isso que serve de prévia. Concluir guarda um passo de desfazer só; Cancelar
//! volta a montagem que a caixa recebeu.

use crate::app::{fmt, App};
use crate::asm_ui::AsmState;
use crate::i18n::tr;
use crate::paineis::{self, Encaixe, Painel};
use crate::picking::Pick;
use crate::render::Camera;
use eframe::egui::{self, Color32, Pos2, Rect, RichText, Sense, Stroke};
use fenkra_core::Id;
use fenkra_montagem::conjuntos::{self, Alvo, ConjuntoFixadores, Disponivel, Lado, RegraCor, Sistema, TipoAlvo};
use fenkra_montagem::pilhas::{ItemPilha, Nivel, Recalculo};
use fenkra_montagem::{Assembly, NodeKind, Placement, ORIGEM};
use fenkra_solidos::biblioteca::{self, Familia, TipoFixador};
use glam::Vec3;

pub const PAINEL: &str = "conjunto_fixadores";
pub const PAINEL_REGRAS: &str = "regras_cor";
/// Quão perto da marca o clique precisa cair, em pontos de tela.
const ALCANCE: f32 = 12.;
/// Altura inicial da caixa.
const ALTURA: f32 = 760.;
/// Altura inicial do cadastro de regras: cabem as linhas de fábrica e os botões.
const ALTURA_REGRAS: f32 = 560.;

/// O que o clique na vista faz.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Modo {
    /// clicar numa marca de furo ou ponto põe ou tira o alvo
    Marcas,
    /// clicar numa face cria um ponto ali
    Ponto,
    /// dois cliques: início e sentido
    Vetor,
    /// arrastar pega tudo o que ficar dentro
    Cerca,
}

pub struct Estado {
    pub id: Id,
    /// como a montagem estava ao abrir: é para onde Cancelar volta
    pub antes: Assembly,
    pub modo: Modo,
    /// furos e pontos que dá para escolher
    pub lista: Vec<Disponivel>,
    /// primeiro clique do vetor: peça e lugar no sistema dela
    pub vetor: Option<(Id, [f64; 3])>,
    /// item preso ao ponteiro
    pub arrasto: Option<(Lado, usize)>,
    pub diag: Recalculo,
    /// cerca desenhada à mão na captura, em fração da vista
    pub cerca_demo: Option<[f32; 4]>,
}

/// Abre a caixa no conjunto pedido, ou num novo.
pub fn abrir(st: &mut AsmState, id: Option<Id>) -> Result<(), String> {
    let dir = st.dir().ok_or("salve a montagem antes: as peças dos fixadores são gravadas na pasta dela")?;
    let antes = st.asm.clone();
    let id = id.filter(|i| st.asm.conjuntos.iter().any(|c| c.id == *i)).unwrap_or_else(|| conjuntos::criar(&mut st.asm, tr("Conjunto")));
    let lista = conjuntos::disponiveis(&st.asm, &dir, &mut st.cache);
    st.conjunto = Some(Estado { id, antes, modo: Modo::Marcas, lista, vetor: None, arrasto: None, diag: Recalculo::default(), cerca_demo: None });
    Ok(())
}

/// Leva o conjunto para a montagem e resolve.
pub fn aplicar(st: &mut AsmState, e: &mut Estado) {
    let Some(dir) = st.dir() else { return };
    e.diag = conjuntos::sincronizar(&mut st.asm, &dir, &mut st.cache, biblioteca::publica(), e.id);
    st.pilhas_diag = e.diag.clone();
    st.modified = true;
    st.solve();
}

/// Concluir: um passo de desfazer, com a montagem de antes da caixa.
fn concluir(st: &mut AsmState, e: Estado) {
    if st.asm != e.antes {
        st.undo.push(e.antes);
        st.redo.clear();
    }
    st.fantasma_novo = true;
}

/// Cancelar: tudo volta, inclusive o conjunto recém-criado.
fn cancelar(st: &mut AsmState, e: Estado) {
    st.asm = e.antes;
    st.solve();
    st.dirty = true;
}

/// Para onde vai o item solto na altura `y`: `faixas` são as linhas na ordem da tela, com o lado e a
/// posição que cada uma representa. Acima do meio da linha entra antes dela; abaixo, depois.
pub fn destino(faixas: &[(Lado, usize, f32, f32)], y: f32) -> Option<(Lado, usize)> {
    let perto = faixas.iter().min_by(|a, b| dist_faixa(a, y).total_cmp(&dist_faixa(b, y)))?;
    let meio = (perto.2 + perto.3) / 2.;
    Some((perto.0, if y < meio { perto.1 } else { perto.1 + 1 }))
}

fn dist_faixa(f: &(Lado, usize, f32, f32), y: f32) -> f32 {
    if y < f.2 {
        f.2 - y
    } else if y > f.3 {
        y - f.3
    } else {
        0.
    }
}

/// Os que caem dentro da cerca, dos pontos já projetados.
pub fn dentro_da_cerca(projetados: &[Option<Pos2>], cerca: Rect) -> Vec<usize> {
    projetados.iter().enumerate().filter(|(_, p)| p.is_some_and(|p| cerca.contains(p))).map(|(i, _)| i).collect()
}

/// Lugar de um alvo agora: a peça pode ter andado.
fn no_mundo(a: &Assembly, t: &TipoAlvo) -> Option<([f64; 3], [f64; 3])> {
    let (p, d) = t.local();
    if t.comp() == ORIGEM {
        return Some((p, d));
    }
    let pl = a.node(t.comp())?.placement;
    Some((pl.point(p), pl.dir(d)))
}

fn projetar(cam: &Camera, rect: Rect, q: [f64; 3]) -> Option<Pos2> {
    cam.project(rect, Vec3::new(q[0] as f32, q[1] as f32, q[2] as f32))
}

use crate::theme::cor_do_dado as cor32;

fn familias(f: impl Fn(TipoFixador) -> bool) -> Vec<&'static Familia> {
    biblioteca::familias().iter().filter(|x| f(x.tipo)).collect()
}

/// Altura e diâmetro de um item, para o esquema.
fn medidas(item: &ItemPilha, parafuso: &str) -> Option<(f32, f32)> {
    let f = biblioteca::publica().familia(&item.familia)?;
    let t = f.tamanho(item.tamanho.as_deref().unwrap_or(parafuso))?;
    let h = t.valor("h").or(t.valor("m"))?;
    let d = t.valor("d2").or(t.valor("s"))?;
    Some((h as f32, d as f32))
}

fn tipo_do_item(item: &ItemPilha) -> &'static str {
    match biblioteca::publica().familia(&item.familia).map(|f| f.tipo) {
        Some(TipoFixador::Porca) => "Porca",
        _ => "Arruela",
    }
}

/// Uma linha da pilha: alça de arraste, família, tamanho e tirar.
fn linha_item(ui: &mut egui::Ui, c: &mut ConjuntoFixadores, lado: Lado, i: usize, e: &mut Estado, faixas: &mut Vec<(Lado, usize, f32, f32)>) -> bool {
    let mut tirar = false;
    let itens = familias(|t| matches!(t, TipoFixador::Arruela | TipoFixador::Porca));
    let salt = (c.id.0, lado == Lado::Superior, i);
    let r = ui.horizontal(|ui| {
        let (alca, resp) = ui.allocate_exact_size(egui::vec2(16., 20.), Sense::drag());
        let cor = if resp.hovered() || e.arrasto == Some((lado, i)) { crate::theme::accent(ui.visuals()) } else { ui.visuals().weak_text_color() };
        for k in [-4., 0., 4.] {
            ui.painter().hline(alca.x_range().shrink(3.), alca.center().y + k, Stroke::new(1.4, cor));
        }
        if resp.drag_started() {
            e.arrasto = Some((lado, i));
        }
        resp.on_hover_text(tr("Arraste para mudar a ordem, ou para a outra pilha."));
        let item = &mut c.lista_mut(lado)[i];
        let atual = biblioteca::publica().familia(&item.familia).map_or(item.familia.clone(), |f| f.norma.clone());
        ui.label(RichText::new(tr(tipo_do_item(item))).small());
        egui::ComboBox::from_id_salt(("item_fam", salt)).width(92.).selected_text(atual).show_ui(ui, |ui| {
            for f in &itens {
                let nome = format!("{} · {}", f.norma, tr(f.tipo.rotulo()));
                if ui.selectable_label(f.id == item.familia, nome).clicked() {
                    item.familia = f.id.clone();
                }
            }
        });
        let tam = item.tamanho.clone().unwrap_or_else(|| tr("Acompanha").to_string());
        egui::ComboBox::from_id_salt(("item_tam", salt)).width(76.).selected_text(tam).show_ui(ui, |ui| {
            if ui.selectable_label(item.tamanho.is_none(), tr("Acompanha")).on_hover_text(tr("Fica no mesmo tamanho do parafuso.")).clicked() {
                item.tamanho = None;
            }
            for t in biblioteca::publica().familia(&item.familia).map(|f| f.tamanhos.as_slice()).unwrap_or_default() {
                if ui.selectable_label(item.tamanho.as_deref() == Some(&t.designacao), &t.designacao).clicked() {
                    item.tamanho = Some(t.designacao.clone());
                }
            }
        });
        tirar = ui.small_button("✖").on_hover_text(tr("Tira este item da pilha.")).clicked();
    });
    faixas.push((lado, i, r.response.rect.top(), r.response.rect.bottom()));
    tirar
}

/// Uma das duas pilhas, com os botões de acrescentar. Lista vazia ainda recebe o que for arrastado.
fn pilha(ui: &mut egui::Ui, c: &mut ConjuntoFixadores, lado: Lado, e: &mut Estado, faixas: &mut Vec<(Lado, usize, f32, f32)>) {
    let mut tirar = None;
    for i in 0..c.lista(lado).len() {
        if linha_item(ui, c, lado, i, e, faixas) {
            tirar = Some(i);
        }
    }
    if let Some(i) = tirar {
        c.lista_mut(lado).remove(i);
    }
    let r = ui.horizontal(|ui| {
        if ui.small_button(format!("+ {}", tr("Arruela"))).on_hover_text(tr("Acrescenta uma arruela no fim desta pilha.")).clicked() {
            c.lista_mut(lado).push(ItemPilha { familia: "iso7089".into(), tamanho: None });
        }
        if ui.small_button(format!("+ {}", tr("Porca"))).on_hover_text(tr("Acrescenta uma porca no fim desta pilha.")).clicked() {
            c.lista_mut(lado).push(ItemPilha { familia: "iso4032".into(), tamanho: None });
        }
    });
    if c.lista(lado).is_empty() {
        faixas.push((lado, 0, r.response.rect.top(), r.response.rect.top()));
    }
}

/// Esquema da pilha, em escala: cabeça, itens de cima, peças, itens de baixo e a ponta.
fn esquema(ui: &mut egui::Ui, c: &ConjuntoFixadores) {
    let (r, _) = ui.allocate_exact_size(egui::vec2(96., 210.), Sense::hover());
    let v = ui.visuals();
    let (peca, item, linha) = (v.widgets.inactive.bg_fill, crate::theme::accent(v), v.text_color());
    let p = ui.painter_at(r);
    let d = c.tamanho.trim_start_matches('M').parse::<f32>().unwrap_or(6.);
    let sup: Vec<(f32, f32)> = c.superior.iter().filter_map(|i| medidas(i, &c.tamanho)).collect();
    let inf: Vec<(f32, f32)> = c.inferior.iter().filter_map(|i| medidas(i, &c.tamanho)).collect();
    let cabeca = (0.8 * d, 1.7 * d);
    let pega = 2.5 * d;
    let total = cabeca.0 + sup.iter().map(|x| x.0).sum::<f32>() + pega + inf.iter().map(|x| x.0).sum::<f32>() + d;
    let largura = sup.iter().chain(&inf).map(|x| x.1).fold(cabeca.1, f32::max);
    let k = ((r.height() - 8.) / total).min((r.width() - 16.) / largura);
    let cx = r.center().x;
    let mut y = r.top() + 4.;
    let bloco = |y: &mut f32, h: f32, d: f32, cor: Color32| {
        let q = Rect::from_center_size(Pos2::new(cx, *y + h * k / 2.), egui::vec2(d * k, h * k));
        p.rect_filled(q, 1., cor);
        p.rect_stroke(q, 1., Stroke::new(1., linha), egui::StrokeKind::Middle);
        *y += h * k;
    };
    let topo = y + cabeca.0 * k;
    bloco(&mut y, cabeca.0, cabeca.1, linha.gamma_multiply(0.45));
    for s in &sup {
        bloco(&mut y, s.0, s.1, item);
    }
    let faixa = Rect::from_min_size(Pos2::new(r.left(), y), egui::vec2(r.width(), pega * k));
    p.rect_filled(faixa, 0., peca);
    y += pega * k;
    for s in &inf {
        bloco(&mut y, s.0, s.1, item);
    }
    let corpo = Rect::from_min_max(Pos2::new(cx - d * k / 2., topo), Pos2::new(cx + d * k / 2., y + d * k));
    p.rect_stroke(corpo, 0., Stroke::new(1., linha), egui::StrokeKind::Middle);
}

fn nome_do_alvo(st: &AsmState, a: &Alvo) -> String {
    let tipo = match a.tipo {
        TipoAlvo::Furo { .. } => tr("Furo"),
        TipoAlvo::Ponto { .. } => tr("Ponto"),
        TipoAlvo::Vetor { .. } => tr("Vetor"),
    };
    if a.tipo.comp() == ORIGEM {
        return format!("{tipo} · {}", tr("Origem"));
    }
    format!("{tipo} · {}", st.asm.name_of(a.tipo.comp()))
}

fn sinal(diag: &Recalculo, alvo: &Alvo) -> (&'static str, Color32, String) {
    let d = diag.de(alvo.pilha.unwrap_or(alvo.id));
    let texto = d.map(|x| x.texto.clone()).unwrap_or_default();
    match d.map(|x| x.nivel) {
        Some(Nivel::Erro) => ("✖", crate::theme::st_err(), texto),
        Some(Nivel::Aviso) => ("⚠", crate::theme::st_warn(), texto),
        _ => ("✔", crate::theme::st_ok(), texto),
    }
}

/// Lista dos alvos do conjunto. Sem furo, dá para inverter o sentido e dar a pega à mão.
fn lista_de_alvos(ui: &mut egui::Ui, st: &AsmState, c: &mut ConjuntoFixadores, e: &Estado) {
    let mut tirar = None;
    egui::ScrollArea::vertical().id_salt("conjunto_alvos").max_height(130.).auto_shrink([false, true]).show(ui, |ui| {
        for (i, a) in c.alvos.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                let (s, cor, texto) = sinal(&e.diag, a);
                let r = ui.label(RichText::new(s).color(cor));
                if !texto.is_empty() {
                    r.on_hover_text(texto);
                }
                if let Some(k) = a.cor {
                    let (q, _) = ui.allocate_exact_size(egui::vec2(10., 10.), Sense::hover());
                    ui.painter().circle_filled(q.center(), 4.5, cor32(k));
                }
                ui.label(RichText::new(nome_do_alvo(st, a)).small());
                if a.tipo.sem_furo() {
                    if ui.small_button("⇅").on_hover_text(tr("Inverte o sentido: a cabeça passa para o outro lado das peças.")).clicked() {
                        match &mut a.tipo {
                            TipoAlvo::Ponto { dir, .. } | TipoAlvo::Vetor { dir, .. } => *dir = dir.map(|x| -x),
                            TipoAlvo::Furo { .. } => {}
                        }
                    }
                    let mut manual = a.pega.is_some();
                    if ui.checkbox(&mut manual, tr("Pega")).on_hover_text(tr("Ligado: a pega é o valor digitado, contado do ponto. Desligado: sai do material que a reta cruza.")).changed() {
                        a.pega = manual.then_some(10.);
                    }
                    if let Some(g) = a.pega.as_mut() {
                        ui.add(crate::ui_ext::dv(g).speed(0.5).range(0.0..=1000.).suffix(" mm").dica(tr("Espessura que o prendedor aperta neste alvo.")));
                    }
                }
                if ui.small_button("✖").on_hover_text(tr("Tira este alvo e o prendedor dele.")).clicked() {
                    tirar = Some(i);
                }
            });
        }
    });
    if let Some(i) = tirar {
        c.alvos.remove(i);
    }
}

fn parafuso(ui: &mut egui::Ui, c: &mut ConjuntoFixadores) {
    let lista = familias(TipoFixador::e_parafuso);
    let fam = biblioteca::publica().familia(&c.familia);
    ui.horizontal(|ui| {
        ui.label(tr("Parafuso")).on_hover_text(tr("A família do parafuso, com as medidas da norma."));
        egui::ComboBox::from_id_salt("conjunto_familia").width(120.).selected_text(fam.map_or(c.familia.clone(), |f| f.norma.clone())).show_ui(ui, |ui| {
            for f in &lista {
                if ui.selectable_label(f.id == c.familia, format!("{} · {}", f.norma, f.nome)).clicked() {
                    c.familia = f.id.clone();
                }
            }
        });
    });
    ui.horizontal(|ui| {
        ui.label(tr("Diâmetro")).on_hover_text(tr("Vale para todos os alvos, menos os que seguem a regra da cor."));
        egui::ComboBox::from_id_salt("conjunto_tamanho").width(70.).selected_text(c.tamanho.clone()).show_ui(ui, |ui| {
            for t in fam.map(|f| f.tamanhos.as_slice()).unwrap_or_default() {
                if ui.selectable_label(t.designacao == c.tamanho, &t.designacao).clicked() {
                    c.tamanho = t.designacao.clone();
                }
            }
        });
        ui.checkbox(&mut c.por_cor, tr("Seguir a cor")).on_hover_text(tr("Cada alvo com cor usa o diâmetro da regra dessa cor."));
    });
    ui.horizontal(|ui| {
        let mut auto = c.comprimento.is_none();
        if ui.checkbox(&mut auto, tr("Comprimento automático")).on_hover_text(tr("Sai da pega, das duas pilhas e da rosca que sobra, no próximo valor da série.")).changed() {
            c.comprimento = if auto { None } else { Some(30.) };
        }
        if let Some(l) = c.comprimento {
            let serie = fam.and_then(|f| f.tamanho(&c.tamanho).map(|t| f.comprimentos_de(t))).unwrap_or_default();
            egui::ComboBox::from_id_salt("conjunto_comprimento").width(64.).selected_text(fmt(l)).show_ui(ui, |ui| {
                for x in serie {
                    if ui.selectable_label(x == l, fmt(x)).clicked() {
                        c.comprimento = Some(x);
                    }
                }
            });
        }
    });
}

/// Painel do conjunto.
pub fn painel(app: &mut App, ctx: &egui::Context, area: Rect) {
    if app.settings.bench != crate::settings::Bench::Montagem {
        return;
    }
    painel_regras(app, ctx, area);
    let padrao_usuario = app.settings.regras_cor.clone();
    let Some(st) = app.asm.as_mut() else { return };
    let Some(mut e) = st.conjunto.take() else { return };
    if st.asm.regras_cor.is_empty() {
        st.asm.regras_cor = padrao_usuario;
    }
    let Some(mut c) = st.asm.conjuntos.iter().find(|c| c.id == e.id).cloned() else { return };
    let original = c.clone();
    let cfg = Painel { nome: PAINEL.into(), titulo: tr("Conjunto de fixadores").to_string(), padrao: paineis::ao_lado_da_arvore(area), largura: 400., redimensiona: true };
    // a caixa nasce com altura para mostrar as duas pilhas sem rolar
    let mut posicao = app.settings.painel(PAINEL, Encaixe::Esquerda);
    posicao.altura = posicao.altura.or(Some(ALTURA.min(area.height() - 16.)));
    let regras = conjuntos::regras_da(&st.asm);
    let (mut fechar, mut sair, mut por_cor, mut abrir_regras) = (false, false, None, false);
    let mut faixas = vec![];
    let saida = paineis::mostrar(ctx, area, &cfg, posicao, |ui| {
        ui.horizontal(|ui| {
            fechar = ui.add(egui::Button::new(RichText::new(tr("✔ Concluir")).strong()).min_size(egui::vec2(100., 26.))).clicked();
            sair = ui.button(tr("Cancelar")).on_hover_text(tr("Desfaz tudo o que mudou desde que a caixa abriu.")).clicked();
        });
        ui.separator();
        ui.horizontal(|ui| {
            ui.label(tr("Nome"));
            ui.add(egui::TextEdit::singleline(&mut c.nome).desired_width(220.));
        });
        ui.label(RichText::new(tr("Alvos")).strong()).on_hover_text(tr("Onde cada prendedor entra. Ponto e vetor não precisam de furo: o prendedor atravessa as peças, e o furo será feito na hora."));
        ui.horizontal_wrapped(|ui| {
            for (m, t, d) in [
                (Modo::Marcas, "Furos e pontos", "Clique nas marcas da vista para pôr ou tirar um alvo."),
                (Modo::Ponto, "Ponto na face", "Clique numa face: o prendedor entra ali, para dentro do material."),
                (Modo::Vetor, "Vetor", "Dois cliques: o início e para onde o prendedor aponta."),
                (Modo::Cerca, "Cerca", "Arraste na vista: tudo o que ficar dentro vira alvo."),
            ] {
                if ui.selectable_label(e.modo == m, tr(t)).on_hover_text(tr(d)).clicked() {
                    e.modo = m;
                    e.vetor = None;
                }
            }
        });
        if e.vetor.is_some() {
            ui.label(RichText::new(tr("Clique no ponto para onde o prendedor aponta.")).small().color(crate::theme::accent(ui.visuals())));
        }
        ui.horizontal(|ui| {
            ui.menu_button(tr("Selecionar por cor"), |ui| {
                for r in &regras {
                    let n = conjuntos::por_cor(&e.lista, r.cor).len();
                    ui.add_enabled_ui(n > 0, |ui| {
                        ui.horizontal(|ui| {
                            let (q, _) = ui.allocate_exact_size(egui::vec2(12., 12.), Sense::hover());
                            ui.painter().circle_filled(q.center(), 5.5, cor32(r.cor));
                            if ui.button(format!("{}  ({n})", r.nome)).clicked() {
                                por_cor = Some(r.cor);
                                ui.close();
                            }
                        });
                    });
                }
            })
            .response
            .on_hover_text(tr("Põe no conjunto todos os furos e pontos da cor escolhida."));
            if ui.button(tr("Regras por cor")).on_hover_text(tr("Abre o cadastro das cores e dos diâmetros.")).clicked() {
                abrir_regras = true;
            }
        });
        if c.alvos.is_empty() {
            ui.label(RichText::new(tr("Nenhum alvo ainda: escolha furos ou pontos na vista.")).weak().small());
        }
        lista_de_alvos(ui, st, &mut c, &e);
        ui.separator();
        parafuso(ui, &mut c);
        ui.separator();
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(286.);
                ui.label(RichText::new(tr("Sob a cabeça")).strong()).on_hover_text(tr("De cima para baixo: o primeiro encosta na cabeça, o último na peça."));
                pilha(ui, &mut c, Lado::Superior, &mut e, &mut faixas);
                ui.add_space(4.);
                ui.label(RichText::new(tr("Contracabeça")).strong()).on_hover_text(tr("De cima para baixo: o primeiro encosta na peça, o último fica na ponta do parafuso."));
                pilha(ui, &mut c, Lado::Inferior, &mut e, &mut faixas);
            });
            esquema(ui, &c);
        });
        if let Some(y) = e.arrasto.and_then(|_| ui.ctx().pointer_latest_pos()).map(|p| p.y) {
            if let Some(f) = faixas.iter().min_by(|a, b| dist_faixa(a, y).total_cmp(&dist_faixa(b, y))) {
                let linha = if y < (f.2 + f.3) / 2. { f.2 } else { f.3 };
                ui.painter().hline(ui.min_rect().x_range(), linha, Stroke::new(2., crate::theme::accent(ui.visuals())));
            }
        }
        egui::CollapsingHeader::new(tr("Avançado")).id_salt("conjunto_avancado").show(ui, |ui| {
            ui.add(crate::ui_ext::dv(&mut c.vao).speed(0.1).range(0.0..=200.).suffix(" mm").rotulo(tr("Vão máximo na pega")).dica(tr("Em ponto e vetor, o maior espaço vazio entre duas peças que ainda conta como a mesma pega.")));
            ui.horizontal(|ui| {
                ui.label(tr("Folga do furo")).on_hover_text(tr("A folga usada para reconhecer o tamanho do furo."));
                for (k, t) in ["Fina", "Normal", "Folgada"].into_iter().enumerate() {
                    if ui.selectable_label(c.folga as usize == k, tr(t)).clicked() {
                        c.folga = k as u8;
                    }
                }
            });
        });
    });
    if let Some(p) = saida.movido {
        app.settings.guardar_painel(PAINEL, p);
    }
    // soltou o item: a ordem nova vale neste quadro
    if ctx.input(|i| i.pointer.any_released()) {
        if let (Some((lado, i)), Some(p)) = (e.arrasto.take(), ctx.pointer_latest_pos()) {
            if let Some((para, j)) = destino(&faixas, p.y) {
                c.mover_item(lado, i, para, j);
            }
        }
    }
    if sair {
        return cancelar(st, e);
    }
    if let Some(k) = por_cor {
        let novos: Vec<_> = conjuntos::por_cor(&e.lista, k).into_iter().map(|i| (e.lista[i].tipo.clone(), e.lista[i].cor)).collect();
        c.acrescentar(&mut st.asm, novos);
    }
    if abrir_regras {
        st.regras_abertas = true;
    }
    if c != original {
        if let Some(x) = st.asm.conjuntos.iter_mut().find(|x| x.id == c.id) {
            *x = c;
        }
        aplicar(st, &mut e);
    }
    if fechar {
        return concluir(st, e);
    }
    st.conjunto = Some(e);
}

/// Cadastro das regras: cor, nome e diâmetro, com os padrões em polegada e métrico.
fn painel_regras(app: &mut App, ctx: &egui::Context, area: Rect) {
    let Some(st) = app.asm.as_mut().filter(|s| s.regras_abertas) else { return };
    let cfg = Painel { nome: PAINEL_REGRAS.into(), titulo: tr("Regras por cor").to_string(), padrao: paineis::abaixo_do_cubo(area, 320.), largura: 340., redimensiona: true };
    let mut posicao = app.settings.painel(PAINEL_REGRAS, Encaixe::Direita);
    posicao.altura = posicao.altura.or(Some(ALTURA_REGRAS.min(area.height() - 16.)));
    let mut regras = conjuntos::regras_da(&st.asm);
    let antes = regras.clone();
    let (mut fechar, mut guardar) = (false, false);
    let saida = paineis::mostrar(ctx, area, &cfg, posicao, |ui| {
        fechar = crate::app::done_bar(ui);
        ui.label(RichText::new(tr("Cada cor quer dizer um diâmetro. Furos e pontos com essa cor podem ser escolhidos juntos e seguem esse diâmetro.")).weak().small());
        let mut tirar = None;
        egui::ScrollArea::vertical().max_height(ui.available_height() - 40.).auto_shrink([false, true]).show(ui, |ui| {
            egui::Grid::new("regras_cor_tabela").striped(true).num_columns(5).show(ui, |ui| {
                for t in ["Cor", "Nome", "Ø (mm)", "Sistema", ""] {
                    ui.label(RichText::new(tr(t)).small().strong());
                }
                ui.end_row();
                for (i, r) in regras.iter_mut().enumerate() {
                    ui.color_edit_button_srgb(&mut r.cor).on_hover_text(tr("A cor dos furos e pontos desta regra."));
                    ui.add(egui::TextEdit::singleline(&mut r.nome).desired_width(56.)).on_hover_text(tr("Como você chama este diâmetro: 1/8, M3."));
                    ui.add(crate::ui_ext::dv(&mut r.diametro).speed(0.01).range(0.1..=100.).dica(tr("Diâmetro nominal em milímetros.")));
                    let t = if r.sistema == Sistema::Polegada { "Polegada" } else { "Métrico" };
                    if ui.small_button(tr(t)).on_hover_text(tr("Clique para trocar entre polegada e métrico.")).clicked() {
                        r.sistema = if r.sistema == Sistema::Polegada { Sistema::Metrico } else { Sistema::Polegada };
                    }
                    if ui.small_button("✖").on_hover_text(tr("Apaga esta regra.")).clicked() {
                        tirar = Some(i);
                    }
                    ui.end_row();
                }
            });
        });
        if let Some(i) = tirar {
            regras.remove(i);
        }
        ui.horizontal(|ui| {
            if ui.button(tr("Nova regra")).on_hover_text(tr("Acrescenta uma linha para cadastrar outra cor.")).clicked() {
                regras.push(RegraCor { cor: [128, 128, 128], nome: "M6".into(), diametro: 6., sistema: Sistema::Metrico });
            }
            if ui.button(tr("Padrão de fábrica")).on_hover_text(tr("Volta às frações de polegada e aos métricos.")).clicked() {
                regras = conjuntos::regras_padrao();
            }
            guardar = ui.button(tr("Guardar como meu padrão")).on_hover_text(tr("Toda montagem nova começa com estas regras.")).clicked();
        });
    });
    if let Some(p) = saida.movido {
        app.settings.guardar_painel(PAINEL_REGRAS, p);
    }
    if regras != antes {
        st.asm.regras_cor = regras.clone();
        st.modified = true;
    }
    if fechar {
        st.regras_abertas = false;
    }
    if guardar {
        app.settings.regras_cor = regras;
        app.settings.save();
    }
}

/// Com a caixa aberta, o clique na peça é dela. `Ponto` e `Vetor` usam o lugar clicado.
pub fn apos_viewport(app: &mut App) -> bool {
    if !app.asm.as_ref().is_some_and(|s| s.conjunto.is_some()) {
        return false;
    }
    let picks = std::mem::take(&mut app.picks);
    let st = app.asm.as_mut().expect("montagem aberta");
    let Some(mut e) = st.conjunto.take() else { return true };
    if let Some(p) = picks.into_iter().next_back() {
        clique_na_peca(st, &mut e, p);
    }
    st.conjunto = Some(e);
    true
}

/// Peça, lugar e normal para fora, no mundo. Aresta e vértice não têm normal.
fn lugar_do_clique(st: &AsmState, p: Pick) -> Option<(Id, [f64; 3], Option<[f64; 3]>)> {
    let (body, q, n) = match p {
        Pick::Face { body, point, normal, .. } => (body, point, Some(normal.map(|v| v as f64))),
        Pick::Edge { body, mid, .. } => (body, mid, None),
        Pick::Vertex { body, pos } => (body, pos, None),
        _ => return None,
    };
    Some((st.comp_de(body)?, q.map(|v| v as f64), n))
}

fn clique_na_peca(st: &mut AsmState, e: &mut Estado, p: Pick) {
    let Some((comp, q, n)) = lugar_do_clique(st, p) else { return };
    let Some(pl) = st.asm.node(comp).map(|x| x.placement) else { return };
    let local = pl.to_local(q);
    let tipo = match (e.modo, e.vetor) {
        (Modo::Ponto, _) => {
            let Some(n) = n else { return };
            TipoAlvo::Ponto { comp, pos: local, dir: pl.inverse().dir(n.map(|x| -x)) }
        }
        (Modo::Vetor, None) => {
            e.vetor = Some((comp, local));
            return;
        }
        (Modo::Vetor, Some((c0, l0))) => {
            let Some(p0) = st.asm.node(c0).map(|x| x.placement) else { return };
            let d = fenkra_montagem::sub(q, p0.point(l0));
            if fenkra_montagem::norm(d) < 1e-6 {
                return;
            }
            e.vetor = None;
            TipoAlvo::Vetor { comp: c0, pos: l0, dir: p0.inverse().dir(fenkra_montagem::unit(d)) }
        }
        _ => return,
    };
    acrescentar(st, e, vec![(tipo, None)]);
}

/// Põe os alvos no conjunto e leva para a montagem.
pub fn acrescentar(st: &mut AsmState, e: &mut Estado, novos: Vec<(TipoAlvo, Option<[u8; 3]>)>) -> usize {
    let Some(mut c) = st.asm.conjuntos.iter().find(|c| c.id == e.id).cloned() else { return 0 };
    let n = c.acrescentar(&mut st.asm, novos);
    if let Some(x) = st.asm.conjuntos.iter_mut().find(|x| x.id == c.id) {
        *x = c;
    }
    if n > 0 {
        aplicar(st, e);
    }
    n
}

/// A cerca vale para a caixa aberta no modo Cerca.
pub fn cerca_ligada(asm: &Option<AsmState>) -> bool {
    asm.as_ref().and_then(|s| s.conjunto.as_ref()).is_some_and(|e| e.modo == Modo::Cerca)
}

/// Fechou a cerca: tudo o que ficou dentro vira alvo.
pub fn fechar_cerca(app: &mut App, cerca: Rect, rect: Rect, cam: &Camera) {
    let Some(st) = app.asm.as_mut() else { return };
    let Some(mut e) = st.conjunto.take() else { return };
    let n = selecionar_na_cerca(st, &mut e, cerca, rect, cam);
    st.conjunto = Some(e);
    app.toast(format!("{n} {}", tr("alvo(s) na cerca")), false);
}

pub fn selecionar_na_cerca(st: &mut AsmState, e: &mut Estado, cerca: Rect, rect: Rect, cam: &Camera) -> usize {
    let pontos: Vec<Option<Pos2>> = e.lista.iter().map(|d| no_mundo(&st.asm, &d.tipo).and_then(|q| projetar(cam, rect, q.0))).collect();
    let novos = dentro_da_cerca(&pontos, cerca).into_iter().map(|i| (e.lista[i].tipo.clone(), e.lista[i].cor)).collect();
    acrescentar(st, e, novos)
}

/// Clique numa marca: põe ou tira o alvo.
pub fn clique_nas_marcas(app: &mut App, ctx: &egui::Context, layer: egui::LayerId, rect: Rect, cam: &Camera) {
    if !ctx.input(|i| i.pointer.primary_clicked()) {
        return;
    }
    let Some(q) = ctx.pointer_latest_pos().filter(|p| rect.contains(*p) && ctx.layer_id_at(*p).is_none_or(|l| l == layer)) else { return };
    let Some(st) = app.asm.as_mut() else { return };
    let Some(mut e) = st.conjunto.take() else { return };
    if e.modo == Modo::Marcas {
        let perto = e.lista.iter().enumerate().filter_map(|(i, d)| no_mundo(&st.asm, &d.tipo).and_then(|w| projetar(cam, rect, w.0)).map(|s| (i, s.distance(q)))).filter(|x| x.1 <= ALCANCE).min_by(|a, b| a.1.total_cmp(&b.1));
        if let Some((i, _)) = perto {
            let d = e.lista[i].clone();
            let ja = st.asm.conjuntos.iter().find(|c| c.id == e.id).and_then(|c| c.alvos.iter().position(|a| a.tipo.mesmo_lugar(&d.tipo)));
            match ja {
                Some(k) => {
                    if let Some(c) = st.asm.conjuntos.iter_mut().find(|c| c.id == e.id) {
                        c.alvos.remove(k);
                    }
                    aplicar(st, &mut e);
                }
                None => {
                    acrescentar(st, &mut e, vec![(d.tipo, d.cor)]);
                }
            }
        }
    }
    st.conjunto = Some(e);
}

/// Marcas na vista: círculo vazio é o que dá para escolher; cheio é alvo. Sem furo, a reta mostra por
/// onde o prendedor atravessa.
pub fn overlay(app: &App, ui: &mut egui::Ui, rect: Rect, cam: &Camera) {
    let Some(st) = app.asm.as_ref() else { return };
    let Some(e) = st.conjunto.as_ref() else { return };
    let Some(c) = st.asm.conjuntos.iter().find(|c| c.id == e.id) else { return };
    let p = ui.painter_at(rect);
    let (tinta, acento) = (ui.visuals().text_color(), crate::theme::accent(ui.visuals()));
    for d in &e.lista {
        let Some(s) = no_mundo(&st.asm, &d.tipo).and_then(|w| projetar(cam, rect, w.0)) else { continue };
        let cor = d.cor.map_or(tinta, cor32);
        if c.alvos.iter().any(|a| a.tipo.mesmo_lugar(&d.tipo)) {
            p.circle_filled(s, 5.5, cor);
            p.circle_stroke(s, 7., Stroke::new(1.4, acento));
        } else {
            p.circle_stroke(s, 5.5, Stroke::new(1.6, cor));
        }
    }
    for a in c.alvos.iter().filter(|a| a.tipo.sem_furo()) {
        let Some((w, u)) = no_mundo(&st.asm, &a.tipo) else { continue };
        let l = a.pilha.and_then(|id| st.asm.pilhas.iter().find(|x| x.id == id)).map_or(20., |x| x.comprimento);
        let ponta = [0, 1, 2].map(|k| w[k] + u[k] * l);
        let (Some(s0), Some(s1)) = (projetar(cam, rect, w), projetar(cam, rect, ponta)) else { continue };
        let (_, cor, _) = sinal(&e.diag, a);
        p.line_segment([s0, s1], Stroke::new(1.6, cor));
        p.circle_filled(s1, 3., cor);
    }
    if let Some(f) = e.cerca_demo {
        let b = Rect::from_min_max(rect.lerp_inside(egui::vec2(f[0], f[1])), rect.lerp_inside(egui::vec2(f[2], f[3])));
        desenhar_cerca(ui, b);
    }
}

/// O retângulo da cerca, com as cores do tema.
pub fn desenhar_cerca(ui: &egui::Ui, b: Rect) {
    let a = crate::theme::accent(ui.visuals());
    ui.painter().rect_filled(b, 0., a.gamma_multiply(0.15));
    ui.painter().rect_stroke(b, 0., Stroke::new(1.2, a), egui::StrokeKind::Middle);
}

/// Enter conclui e Esc cancela, sem texto em edição.
pub fn teclas(app: &mut App, ctx: &egui::Context) {
    if ctx.egui_wants_keyboard_input() {
        return;
    }
    let Some(st) = app.asm.as_mut().filter(|s| s.conjunto.is_some()) else { return };
    let (ok, esc) = ctx.input(|i| (i.key_pressed(egui::Key::Enter), i.key_pressed(egui::Key::Escape)));
    if !ok && !esc {
        return;
    }
    let Some(e) = st.conjunto.take() else { return };
    if ok {
        concluir(st, e);
    } else {
        cancelar(st, e);
    }
}

/// Linha do conjunto na árvore: simples, mas é por ela que se volta a editar tudo.
pub fn linha_na_arvore(ui: &mut egui::Ui, st: &AsmState, c: &ConjuntoFixadores, act: &mut Option<(Id, u8)>, new_sel: &mut Option<Id>) {
    crate::tree_ui::marca(ui);
    let r = ui
        .horizontal(|ui| {
            ui.add(egui::Image::new(crate::icons::get("asm_conjunto")).fit_to_exact_size(egui::vec2(14., 14.)));
            let n = c.alvos.len();
            ui.selectable_label(st.sel == Some(c.id), RichText::new(format!("{}  ({} · {n} {})", c.nome, if c.por_cor { tr("por cor") } else { c.tamanho.as_str() }, tr(if n == 1 { "alvo" } else { "alvos" }))).small())
        })
        .inner;
    if st.sel == Some(c.id) {
        crate::tree_ui::achar(r.rect);
    }
    if r.clicked() {
        *new_sel = Some(c.id);
    }
    if r.double_clicked() {
        *act = Some((c.id, 41));
    }
    let r = r.on_hover_text(tr("Duplo clique abre a caixa do conjunto de novo."));
    r.context_menu(|ui| {
        if ui.button(tr("Editar conjunto")).clicked() {
            *act = Some((c.id, 41));
            ui.close();
        }
        if ui.button(tr("🗑 Apagar   Del")).clicked() {
            *act = Some((c.id, 42));
            ui.close();
        }
    });
}

/// Placa de exemplo: furos e pontos com a cor do comando que os criou.
fn placa_demo(esp: f64, furos: &[([f64; 2], [u8; 3])], pontos: &[([f64; 2], [u8; 3])]) -> fenkra_core::Document {
    use fenkra_core::{sketch::Curve, Appearance, BoolOp, Document, Extent, Extrude, FeatureKind, Finish, Hole, HoleKind, OriginPlane, PlaneRef, Sketch, SketchData};
    let mut d = Document::new("Placa");
    let mut n = 7_700_000u64;
    let mut al = || {
        n += 1;
        Id(n)
    };
    let mut base = SketchData::default();
    base.add_rect(&mut al, [-45., -30.], [45., 30.], false);
    let sb = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data: base }));
    d.add_feature(FeatureKind::Extrude(Extrude { sketch: sb, op: BoolOp::Add, extent: Extent::Blind(esp.to_string()), reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
    let topo = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), &esp.to_string());
    let tinta = |color| Some(Appearance { color, finish: Finish::default(), texture: None });
    for (q, cor) in furos {
        let mut c = SketchData::default();
        let p = c.add_point(&mut al, q[0], q[1]);
        c.add_curve(&mut al, Curve::Point { p }, false);
        let sc = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Datum(topo), data: c }));
        let h = d.add_feature(FeatureKind::Hole(Hole { sketch: sc, kind: HoleKind::Simples, diameter: "6.6".into(), depth: Extent::ThroughAll, thread: None, thread_modeled: false, standard: None, points: vec![], dir: None, reverse: false, tip: None, taper: "0".into(), left_hand: false, extra: Default::default() }));
        if let Some(f) = d.feature_mut(h) {
            f.appearance = tinta(*cor);
        }
    }
    for (q, cor) in pontos {
        let mut c = SketchData::default();
        let p = c.add_point(&mut al, q[0], q[1]);
        c.add_curve(&mut al, Curve::Point { p }, false);
        let sc = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Datum(topo), data: c }));
        if let Some(f) = d.feature_mut(sc) {
            f.appearance = tinta(*cor);
        }
    }
    d
}

/// `FENKRA_DEMO_ASM=conjunto|conjunto_regras|conjunto_cerca`: duas placas, com a caixa aberta numa pilha
/// de cinco itens, alvos por furo, ponto e vetor; o cadastro das regras; ou a cerca em cima dos alvos.
pub fn demo(st: &mut AsmState, nome: &str) {
    let Some(dir) = st.dir() else { return };
    let (azul, laranja) = ([80, 130, 230], [240, 150, 60]);
    let furos = [([-30., 15.], azul), ([-30., -15.], azul), ([0., 15.], laranja)];
    let pontos = [([0., -15.], laranja), ([30., 15.], azul), ([30., -15.], azul)];
    let cima = placa_demo(8., &furos, &pontos);
    let baixo = placa_demo(8., &furos, &[]);
    for (arq, doc) in [("demo_cima.fkp", &cima), ("demo_baixo.fkp", &baixo)] {
        let _ = std::fs::write(dir.join(arq), doc.to_json());
        st.cache.forget(&dir.join(arq));
    }
    let mut a = Assembly::new("Conjunto de fixadores");
    let comp = a.add_component("Tampa", NodeKind::Part { path: "demo_cima.fkp".into() }, Placement { origin: [0., 0., 8.], ..Default::default() }, None);
    a.add_component("Base", NodeKind::Part { path: "demo_baixo.fkp".into() }, Placement::default(), None);
    st.asm = a;
    st.dirty = true;
    if abrir(st, None).is_err() {
        return;
    }
    let Some(mut e) = st.conjunto.take() else { return };
    if let Some(c) = st.asm.conjuntos.iter_mut().find(|c| c.id == e.id) {
        c.nome = tr("Tampa e base").to_string();
        let item = |f: &str, t: Option<&str>| ItemPilha { familia: f.into(), tamanho: t.map(str::to_string) };
        c.superior = vec![item("iso7089", None), item("iso7089", Some("M8")), item("iso4035", None)];
        c.inferior = vec![item("iso7089", None), item("iso4032", None)];
    }
    if nome == "conjunto_cerca" {
        e.modo = Modo::Cerca;
        e.cerca_demo = Some([0.30, 0.25, 0.72, 0.75]);
    } else {
        let azuis = conjuntos::por_cor(&e.lista, azul);
        let novos: Vec<_> = azuis.iter().map(|i| (e.lista[*i].tipo.clone(), e.lista[*i].cor)).collect();
        acrescentar(st, &mut e, novos);
        let s = 25_f64.to_radians();
        acrescentar(st, &mut e, vec![(TipoAlvo::Ponto { comp, pos: [0., 0., 8.], dir: [0., 0., -1.] }, None), (TipoAlvo::Vetor { comp, pos: [-14., 0., 8.], dir: [s.sin(), 0., -s.cos()] }, None)]);
    }
    st.regras_abertas = nome == "conjunto_regras";
    st.conjunto = Some(e);
    st.dirty = true;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn soltar_acima_ou_abaixo_do_meio_da_linha_escolhe_a_posicao() {
        let f = [(Lado::Superior, 0, 0., 20.), (Lado::Superior, 1, 20., 40.), (Lado::Inferior, 0, 80., 100.)];
        assert_eq!(destino(&f, 5.), Some((Lado::Superior, 0)));
        assert_eq!(destino(&f, 15.), Some((Lado::Superior, 1)));
        assert_eq!(destino(&f, 35.), Some((Lado::Superior, 2)));
        assert_eq!(destino(&f, 85.), Some((Lado::Inferior, 0)));
        assert_eq!(destino(&f, 500.), Some((Lado::Inferior, 1)), "fora das linhas vale a mais próxima");
        assert_eq!(destino(&[], 5.), None);
    }

    #[test]
    fn soltar_em_lista_vazia_entra_nela() {
        let f = [(Lado::Superior, 0, 0., 20.), (Lado::Inferior, 0, 60., 60.)];
        assert_eq!(destino(&f, 59.), Some((Lado::Inferior, 0)));
    }

    #[test]
    fn a_cerca_pega_so_o_que_fica_dentro() {
        let pts = [Some(Pos2::new(10., 10.)), Some(Pos2::new(50., 50.)), None, Some(Pos2::new(90., 20.))];
        let cerca = Rect::from_two_pos(Pos2::new(0., 0.), Pos2::new(60., 60.));
        assert_eq!(dentro_da_cerca(&pts, cerca), vec![0, 1]);
    }

    fn montagem_de_demo(nome: &str) -> (AsmState, std::path::PathBuf) {
        let d = std::env::temp_dir().join(format!("fenkra_conjunto_ui_{nome}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        (AsmState::new(Assembly::new("t"), Some(d.join("c.fka"))), d)
    }

    #[test]
    fn a_demo_monta_o_conjunto_e_cancelar_volta_tudo() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (mut st, d) = montagem_de_demo("cancelar");
        demo(&mut st, "conjunto");
        let e = st.conjunto.take().unwrap();
        let c = st.asm.conjuntos.iter().find(|c| c.id == e.id).unwrap();
        assert_eq!(c.alvos.len(), 6, "quatro azuis, um ponto e um vetor");
        assert!(c.alvos.iter().all(|a| a.pilha.is_some()), "{:?}", e.diag.diagnosticos);
        assert!(!e.diag.diagnosticos.iter().any(|x| x.nivel == Nivel::Erro), "{:?}", e.diag.diagnosticos);
        cancelar(&mut st, e);
        assert!(st.asm.conjuntos.is_empty() && st.asm.pilhas.is_empty());
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn concluir_e_um_desfazer_so_e_reabrir_edita_o_mesmo_conjunto() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (mut st, d) = montagem_de_demo("concluir");
        demo(&mut st, "conjunto");
        let e = st.conjunto.take().unwrap();
        let id = e.id;
        let n = st.undo.len();
        concluir(&mut st, e);
        assert_eq!(st.undo.len(), n + 1);
        abrir(&mut st, Some(id)).unwrap();
        let mut e = st.conjunto.take().unwrap();
        assert_eq!(e.id, id);
        assert_eq!(st.asm.conjuntos.len(), 1, "editar não cria outro");
        let antes = st.asm.pilhas.iter().map(|p| p.pecas.len()).collect::<Vec<_>>();
        st.asm.conjuntos[0].superior.remove(0);
        aplicar(&mut st, &mut e);
        assert!(st.asm.pilhas.iter().map(|p| p.pecas.len()).zip(antes).all(|(a, b)| a + 1 == b), "a pilha mudou em todos");
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn cerca_na_camera_real_acrescenta_alvos_e_nao_repete() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (mut st, d) = montagem_de_demo("cerca");
        demo(&mut st, "conjunto_cerca");
        let mut e = st.conjunto.take().unwrap();
        assert_eq!(st.asm.conjuntos[0].alvos.len(), 0);
        let rect = Rect::from_min_size(Pos2::ZERO, egui::vec2(800., 600.));
        let cam = Camera { dist: 300., ..Default::default() };
        let tudo = rect;
        let n = selecionar_na_cerca(&mut st, &mut e, tudo, rect, &cam);
        assert_eq!(n, e.lista.len(), "a vista inteira pega tudo");
        assert!(n >= 6, "{n}");
        assert_eq!(selecionar_na_cerca(&mut st, &mut e, tudo, rect, &cam), 0, "de novo não duplica");
        let vazia = Rect::from_min_size(Pos2::new(1., 1.), egui::vec2(2., 2.));
        assert_eq!(selecionar_na_cerca(&mut st, &mut e, vazia, rect, &cam), 0);
        let _ = std::fs::remove_dir_all(d);
    }
}
