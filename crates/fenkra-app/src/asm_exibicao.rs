// SPDX-License-Identifier: LGPL-2.1-or-later
//! Painel de exibição da montagem (B286): Estados, Explosão e Animação em abas. A lista de passos
//! mora em `fenkra_montagem::display`; aqui ficam a lista na tela, a seta do manipulador que vira
//! passo, as linhas pontilhadas no 3D e os quadros-chave arrastáveis.

use crate::app::{fmt, App};
use crate::asm_ui::AsmState;
use crate::i18n::tr;
use crate::paineis::{self, Encaixe, Painel};
use crate::render::Camera;
use crate::ui_ext::dv;
use eframe::egui::{self, Pos2, Rect, RichText, Stroke};
use fenkra_core::Id;
use fenkra_montagem::display::{auto_explode, Animation, Cam, Explosao, Key, ModoPasso, PassoExplosao};
use fenkra_montagem::{unit, V3};
use glam::Vec3;
use std::collections::HashMap;

pub const PAINEL: &str = "exibicao";

/// Segundos de cada passo na animação gerada.
const SEGUNDOS_POR_PASSO: f64 = 1.5;

/// Tamanho do traço e do espaço das linhas de explosão (px).
const TRACO: f32 = 6.;
const VAO: f32 = 4.;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Aba {
    #[default]
    Estados,
    Explosao,
    Animacao,
}

/// O que o painel lembra entre um quadro e outro.
#[derive(Clone, Debug, Default)]
pub struct Exibicao {
    pub aba: Aba,
    /// explosão escolhida na lista da montagem
    pub explosao: usize,
    pub passo: Option<usize>,
    /// seta do manipulador em arrasto: a peça e o deslocamento dela no começo
    pub seta: Option<(Id, V3)>,
    /// alça do passo em arrasto: a distância no começo
    pub alca: Option<f64>,
    /// refaz a explosão quando a malha chegar: sem ela não há centro de peça
    pub reaplicar: bool,
}

fn v3(p: V3) -> Vec3 {
    Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32)
}

/// Centro de cada peça montada. Peça sem malha carregada usa a origem dela.
fn centros(st: &AsmState) -> HashMap<Id, V3> {
    let mut c = crate::asm_ui::centers(st);
    for n in &st.asm.nodes {
        c.entry(n.id).or_insert(n.placement.origin);
    }
    c
}

/// Leva a explosão escolhida para o que está na tela.
pub fn aplicar(st: &mut AsmState) {
    let c = centros(st);
    if let Some(e) = st.asm.explosoes.get(st.exibicao.explosao) {
        st.live.offsets = e.offsets(&c);
        st.live.linhas = if e.linhas { e.linhas(&c) } else { vec![] };
        st.dirty = true;
    }
}

/// Explosão escolhida, criando a primeira se não houver nenhuma.
fn explosao_atual(st: &mut AsmState) -> &mut Explosao {
    if st.asm.explosoes.is_empty() {
        st.asm.explosoes.push(Explosao::nova(tr("Explosão 1")));
    }
    st.exibicao.explosao = st.exibicao.explosao.min(st.asm.explosoes.len() - 1);
    &mut st.asm.explosoes[st.exibicao.explosao]
}

/// Soltou a seta do manipulador: o que ela andou vira um passo das peças escolhidas.
pub fn soltar_seta(st: &mut AsmState, id: Id, inicio: V3, escolhidas: Vec<Id>) {
    let agora = st.live.offset(id);
    let delta = [0, 1, 2].map(|k| agora[k] - inicio[k]);
    let mut comps = escolhidas;
    if !comps.contains(&id) {
        comps = vec![id];
    }
    st.push_undo();
    let e = explosao_atual(st);
    if let Some(i) = e.passo_da_seta(comps, delta) {
        st.exibicao.passo = Some(i);
        st.modified = true;
    }
    aplicar(st);
}

/// Texto curto do passo na lista.
fn resumo(p: &PassoExplosao) -> String {
    format!("{} · {} mm · {} {}", tr(p.modo.label()), fmt(p.modo.dist()), p.comps.len(), tr("peça(s)"))
}

/// Tempo do quadro-chave depois de arrastá-lo `dx` pixels numa linha do tempo de `largura` pixels.
fn tempo_arrastado(t: f64, dx: f32, largura: f32, duracao: f64) -> f64 {
    (t + dx as f64 / largura.max(1.) as f64 * duracao).max(0.)
}

fn modo_com(modo: ModoPasso, novo: u8, comps: &[Id], c: &HashMap<Id, V3>) -> ModoPasso {
    let (dir, dist) = (modo.dir(), modo.dist());
    match novo {
        0 => ModoPasso::Linear { dir, dist },
        1 => {
            let pts: Vec<V3> = comps.iter().filter_map(|i| c.get(i).copied()).collect();
            let k = pts.len().max(1) as f64;
            let centro = [0, 1, 2].map(|i| pts.iter().map(|p| p[i]).sum::<f64>() / k);
            ModoPasso::Radial { centro, eixo: dir, dist }
        }
        _ => ModoPasso::Espacado { dir, dist },
    }
}

fn com_direcao(modo: &mut ModoPasso, d: V3) {
    match modo {
        ModoPasso::Linear { dir, .. } | ModoPasso::Espacado { dir, .. } => *dir = d,
        ModoPasso::Radial { eixo, .. } => *eixo = d,
    }
}

/// Campos do passo escolhido. Devolve se algo mudou.
fn detalhe(ui: &mut egui::Ui, p: &mut PassoExplosao, c: &HashMap<Id, V3>, selecao: &[Id]) -> bool {
    let antes = p.clone();
    ui.horizontal(|ui| {
        ui.label(tr("Nome"));
        ui.text_edit_singleline(&mut p.nome);
    });
    ui.horizontal_wrapped(|ui| {
        for (k, nome, dica) in [(0u8, "Linear", "Todas as peças andam a mesma distância."), (1, "Radial", "Cada peça se afasta do eixo, pela reta que sai dele."), (2, "Espaçamento automático", "Em fila: a última anda a distância toda, e as outras se espaçam por igual.")] {
            let atual = matches!((k, p.modo), (0, ModoPasso::Linear { .. }) | (1, ModoPasso::Radial { .. }) | (2, ModoPasso::Espacado { .. }));
            if ui.selectable_label(atual, tr(nome)).on_hover_text(tr(dica)).clicked() && !atual {
                p.modo = modo_com(p.modo, k, &p.comps, c);
            }
        }
    });
    let mut d = p.modo.dist();
    ui.add(dv(&mut d).speed(1.).range(0.0..=1e6).suffix(" mm").rotulo(tr("Distância")).dica(tr("Quanto o passo afasta as peças. Também muda pela alça no 3D.")));
    p.modo.set_dist(d);
    ui.horizontal_wrapped(|ui| {
        ui.label(tr(if matches!(p.modo, ModoPasso::Radial { .. }) { "Eixo" } else { "Direção" }));
        for (t, v) in [("X", [1., 0., 0.]), ("Y", [0., 1., 0.]), ("Z", [0., 0., 1.])] {
            if ui.small_button(t).on_hover_text(tr("Aponta o passo neste eixo da montagem")).clicked() {
                com_direcao(&mut p.modo, v);
            }
        }
        if ui.small_button("⇄").on_hover_text(tr("Inverte o sentido")).clicked() {
            let v = p.modo.dir().map(|x| -x);
            com_direcao(&mut p.modo, v);
        }
    });
    ui.horizontal(|ui| {
        ui.label(format!("{} {}", p.comps.len(), tr("peça(s)")));
        if ui.add_enabled(!selecao.is_empty(), egui::Button::new(tr("Usar a seleção")).small()).on_hover_text(tr("Troca as peças do passo pelas escolhidas agora")).clicked() {
            p.comps = selecao.to_vec();
        }
    });
    *p != antes
}

fn aba_estados(ui: &mut egui::Ui, st: &mut AsmState, cam: Option<Cam>, cam_nova: &mut Option<Cam>) {
    ui.label(RichText::new(tr("Guardam o que aparece, cores, explosão e câmera. O detalhamento usa estes estados como coletores de exibição.")).weak().small());
    let mut apply = None;
    let mut del = None;
    for (i, s) in st.asm.display_states.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut s.name).desired_width(150.));
            if ui.small_button(tr("▶ aplicar")).clicked() {
                apply = Some(i);
            }
            if ui.small_button("✖").clicked() {
                del = Some(i);
            }
        });
    }
    if let Some(i) = apply {
        st.live = st.asm.display_states[i].clone();
        // o estado salvo guarda quem fica oculto: aplica na árvore também
        for n in &mut st.asm.nodes {
            n.visible = !st.live.hidden.contains(&n.id);
        }
        *cam_nova = st.live.camera;
        st.dirty = true;
    }
    if let Some(i) = del {
        st.asm.display_states.remove(i);
    }
    if ui.button(tr("💾 Salvar estado atual")).on_hover_text(tr("Leva a explosão e as linhas para o desenho")).clicked() {
        let mut s = st.asm.snapshot_state(&format!("{} {}", tr("Estado"), st.asm.display_states.len() + 1));
        s.offsets = st.live.offsets.clone();
        s.linhas = st.live.linhas.clone();
        s.camera = cam;
        st.asm.display_states.push(s);
        st.modified = true;
    }
}

/// O que a lista de passos pediu.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Lista {
    Mover(usize, usize),
    Apagar(usize),
    Escolher(usize),
    Suprimir(usize),
}

fn lista_de_passos(ui: &mut egui::Ui, e: &Explosao, escolhido: Option<usize>) -> Option<Lista> {
    let mut pedido = None;
    let n = e.passos.len();
    for (i, p) in e.passos.iter().enumerate() {
        let linha = ui.horizontal(|ui| {
            ui.dnd_drag_source(egui::Id::unique(("passo_explosao", i)), i, |ui| {
                ui.label(RichText::new("≡").weak());
            })
            .response
            .on_hover_text(tr("Arraste para mudar a ordem"));
            let mut ligado = !p.suprimido;
            if ui.checkbox(&mut ligado, "").on_hover_text(tr("Desligado: o passo fica guardado e as peças não andam")).changed() {
                pedido = Some(Lista::Suprimir(i));
            }
            let texto = if p.suprimido { RichText::new(&p.nome).strikethrough().weak() } else { RichText::new(&p.nome) };
            if ui.selectable_label(escolhido == Some(i), texto).on_hover_text(resumo(p)).clicked() {
                pedido = Some(Lista::Escolher(i));
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("✖").on_hover_text(tr("Apaga o passo")).clicked() {
                    pedido = Some(Lista::Apagar(i));
                }
                if ui.add_enabled(i + 1 < n, egui::Button::new("⬇").small()).on_hover_text(tr("Desce na ordem")).clicked() {
                    pedido = Some(Lista::Mover(i, i + 1));
                }
                if ui.add_enabled(i > 0, egui::Button::new("⬆").small()).on_hover_text(tr("Sobe na ordem")).clicked() {
                    pedido = Some(Lista::Mover(i, i - 1));
                }
            });
        });
        if let Some(de) = linha.response.dnd_release_payload::<usize>() {
            pedido = Some(Lista::Mover(*de, i));
        }
    }
    pedido
}

/// Faz o que a lista pediu. Só escolher não muda a montagem, e por isso não gasta um desfazer.
fn pedir(st: &mut AsmState, i: usize, p: Lista) {
    if let Lista::Escolher(k) = p {
        st.exibicao.passo = Some(k);
        return;
    }
    st.push_undo();
    let e = &mut st.asm.explosoes[i];
    match p {
        Lista::Mover(de, para) => {
            e.mover_passo(de, para);
            st.exibicao.passo = Some(para);
        }
        Lista::Apagar(k) => {
            e.passos.remove(k);
            st.exibicao.passo = None;
        }
        Lista::Suprimir(k) => e.passos[k].suprimido = !e.passos[k].suprimido,
        Lista::Escolher(_) => {}
    }
    st.modified = true;
    aplicar(st);
}

fn aba_explosao(ui: &mut egui::Ui, st: &mut AsmState, selecao: &[Id]) {
    let c = centros(st);
    // ---- qual explosão
    ui.horizontal_wrapped(|ui| {
        let nomes: Vec<String> = st.asm.explosoes.iter().map(|e| e.nome.clone()).collect();
        if !nomes.is_empty() {
            st.exibicao.explosao = st.exibicao.explosao.min(nomes.len() - 1);
            let antes = st.exibicao.explosao;
            egui::ComboBox::from_id_salt("asm_explosao_sel").selected_text(&nomes[antes]).show_ui(ui, |ui| {
                for (i, n) in nomes.iter().enumerate() {
                    ui.selectable_value(&mut st.exibicao.explosao, i, n);
                }
            });
            if st.exibicao.explosao != antes {
                st.exibicao.passo = None;
                aplicar(st);
            }
        }
        if ui.small_button(tr("+ nova")).on_hover_text(tr("Começa uma lista de passos vazia")).clicked() {
            st.push_undo();
            let n = st.asm.explosoes.len() + 1;
            st.asm.explosoes.push(Explosao::nova(&format!("{} {n}", tr("Explosão"))));
            st.exibicao.explosao = n - 1;
            st.exibicao.passo = None;
            aplicar(st);
        }
    });
    ui.horizontal_wrapped(|ui| {
        if ui.button(tr("Explodir")).on_hover_text(tr("Aplica os passos à vista")).clicked() {
            aplicar(st);
        }
        if ui.button(tr("Juntar tudo (sem explosão)")).clicked() {
            st.live.offsets.clear();
            st.live.linhas.clear();
            st.explode_factor = 0.;
            st.dirty = true;
        }
        if ui.button(tr("Animar os passos")).on_hover_text(tr("Cria uma animação que abre a montagem um passo por vez")).clicked() {
            if let Some(e) = st.asm.explosoes.get(st.exibicao.explosao) {
                let base = st.asm.snapshot_state(tr("Montada"));
                st.asm.animations.push(e.animacao(&base, &c, SEGUNDOS_POR_PASSO));
                st.anim_sel = st.asm.animations.len() - 1;
                st.anim_t = 0.;
                st.exibicao.aba = Aba::Animacao;
                st.modified = true;
            }
        }
    });
    crate::editor::check(ui, &mut st.explode_mode, "arrastar a seta do manipulador cria um passo").on_hover_text(tr("Escolha a peça e arraste a seta: ao soltar, nasce um passo com as peças escolhidas"));
    if let Some(i) = st.asm.explosoes.get(st.exibicao.explosao).map(|_| st.exibicao.explosao) {
        let e = &mut st.asm.explosoes[i];
        ui.horizontal(|ui| {
            ui.label(tr("Nome"));
            ui.text_edit_singleline(&mut e.nome);
        });
        let antes = e.linhas;
        crate::editor::check(ui, &mut e.linhas, "linhas de explosão").on_hover_text(tr("Linhas pontilhadas ligando cada peça ao lugar dela; seguem para o desenho"));
        let linhas_mudou = e.linhas != antes;
        ui.separator();
        if e.passos.is_empty() {
            ui.label(RichText::new(tr("Nenhum passo: ligue a opção acima e arraste a seta de uma peça.")).weak().small());
        }
        let pedido = lista_de_passos(ui, &st.asm.explosoes[i], st.exibicao.passo);
        if let Some(p) = pedido {
            pedir(st, i, p);
        }
        if linhas_mudou {
            aplicar(st);
        }
        if let Some(k) = st.exibicao.passo.filter(|k| *k < st.asm.explosoes[i].passos.len()) {
            ui.separator();
            let mut p = st.asm.explosoes[i].passos[k].clone();
            if detalhe(ui, &mut p, &c, selecao) {
                st.push_undo();
                st.asm.explosoes[i].passos[k] = p;
                st.modified = true;
                aplicar(st);
            }
        }
    }
    ui.separator();
    ui.horizontal_wrapped(|ui| {
        let r = ui.add(crate::ui_ext::slider(&mut st.explode_factor, 0.0..=3.0).text(tr("afastamento")));
        if ui.button(tr("Gerar passos")).on_hover_text(tr("Um passo por peça, afastando do centro da montagem pelo fator")).clicked() {
            st.push_undo();
            let factor = st.explode_factor;
            let auto = auto_explode(&st.asm, &c, factor);
            let e = explosao_atual(st);
            for (id, d) in auto {
                e.passo_da_seta(vec![id], d);
            }
            st.modified = true;
            aplicar(st);
        }
        let _ = r;
    });
}

fn aba_animacao(ui: &mut egui::Ui, st: &mut AsmState, cam: Option<Cam>) {
    if st.asm.animations.is_empty() {
        if ui.button(tr("+ Nova animação")).clicked() {
            st.asm.animations.push(Animation { name: "Animação 1".into(), keys: vec![] });
        }
        return;
    }
    st.anim_sel = st.anim_sel.min(st.asm.animations.len() - 1);
    ui.horizontal_wrapped(|ui| {
        let names: Vec<String> = st.asm.animations.iter().map(|a| a.name.clone()).collect();
        egui::ComboBox::from_id_salt("anim_sel").selected_text(&names[st.anim_sel]).show_ui(ui, |ui| {
            for (i, n) in names.iter().enumerate() {
                ui.selectable_value(&mut st.anim_sel, i, n);
            }
        });
        if ui.small_button(tr("+ nova")).clicked() {
            let n = st.asm.animations.len() + 1;
            st.asm.animations.push(Animation { name: format!("Animação {n}"), keys: vec![] });
            st.anim_sel = n - 1;
        }
    });
    let a = &mut st.asm.animations[st.anim_sel];
    ui.horizontal(|ui| {
        ui.label(tr("Nome"));
        ui.text_edit_singleline(&mut a.name);
    });
    let dur = a.duration().max(0.1);
    ui.horizontal(|ui| {
        if ui.button(if st.anim_play { "⏸" } else { "▶" }).clicked() {
            st.anim_play = !st.anim_play;
        }
        if ui.button("⏮").clicked() {
            st.anim_t = 0.;
        }
        ui.add(crate::ui_ext::slider(&mut st.anim_t, 0.0..=dur).text("t (s)"));
    });
    // linha do tempo: arrastar um quadro muda o instante dele
    let (r, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 26.), egui::Sense::hover());
    let p = ui.painter_at(r);
    let v = ui.visuals().clone();
    p.rect_filled(r, 4., v.extreme_bg_color);
    let cor = crate::theme::accent(&v);
    for (i, k) in a.keys.iter_mut().enumerate() {
        let x = r.left() + (k.t / dur) as f32 * r.width();
        let alca = ui.interact(Rect::from_center_size(egui::pos2(x, r.center().y), egui::vec2(14., r.height())), egui::Id::unique(("quadro_chave", i)), egui::Sense::drag()).on_hover_text(tr("Arraste para mudar o instante deste quadro"));
        if alca.dragged() {
            k.t = tempo_arrastado(k.t, alca.drag_delta().x, r.width(), dur);
            st.modified = true;
        }
        let tam = if alca.hovered() || alca.dragged() { 8. } else { 6. };
        p.add(egui::Shape::convex_polygon(vec![egui::pos2(x, r.center().y - tam), egui::pos2(x + tam, r.center().y), egui::pos2(x, r.center().y + tam), egui::pos2(x - tam, r.center().y)], cor, Stroke::NONE));
    }
    let xt = r.left() + (st.anim_t / dur) as f32 * r.width();
    p.line_segment([egui::pos2(xt, r.top()), egui::pos2(xt, r.bottom())], Stroke::new(2., v.text_color()));
    let mut kdel = None;
    for (i, k) in a.keys.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("◆ {}", k.state.name)).small());
            let campo = ui.add(dv(&mut k.t).range(0.0..=600.0).speed(0.05).suffix(" s"));
            crate::ui_ext::wheel(ui, &campo, &mut k.t);
            if ui.small_button("✖").clicked() {
                kdel = Some(i);
            }
        });
    }
    if let Some(i) = kdel {
        a.keys.remove(i);
    }
    if ui.button(tr("◆ Quadro-chave aqui (estado atual + câmera)")).clicked() {
        let mut s = st.asm.snapshot_state(&format!("quadro {}", st.asm.animations[st.anim_sel].keys.len() + 1));
        s.offsets = st.live.offsets.clone();
        s.camera = cam;
        let t = if st.asm.animations[st.anim_sel].keys.is_empty() { 0. } else { st.anim_t.max(st.asm.animations[st.anim_sel].duration() + 1.) };
        st.asm.animations[st.anim_sel].keys.push(Key { t, state: s });
        st.modified = true;
    }
    ui.label(RichText::new(tr("Monte a cena (oculte peças, exploda, gire a câmera) e grave um quadro; repita em outro tempo. A animação interpola a explosão e a câmera.")).weak().small());
}

/// Painel de exibição.
pub fn painel(app: &mut App, ctx: &egui::Context, area: Rect) {
    if app.settings.bench != crate::settings::Bench::Montagem {
        return;
    }
    let cam = app.camera_get();
    let posicao = app.settings.painel(PAINEL, Encaixe::Direita);
    let selecao = app.asm.as_ref().map_or(vec![], |st| {
        let mut s = st.comps_of(&app.picks);
        s.extend(st.multi.iter().copied());
        s.extend(st.sel.filter(|id| st.asm.node(*id).is_some()));
        s.sort_by_key(|i| i.0);
        s.dedup();
        s
    });
    let Some(st) = app.asm.as_mut() else { return };
    if !st.display_open {
        return;
    }
    if st.exibicao.reaplicar && st.view.is_some() {
        st.exibicao.reaplicar = false;
        aplicar(st);
    }
    let before = st.live.clone();
    let mut cam_nova = None;
    let mut fechar = false;
    let cfg = Painel { nome: PAINEL.into(), titulo: tr("Exibição").to_string(), padrao: paineis::abaixo_do_cubo(area, 360.), largura: 360., redimensiona: true };
    let saida = paineis::mostrar(ctx, area, &cfg, posicao, |ui| {
        if crate::app::done_bar(ui) {
            fechar = true;
        }
        ui.horizontal(|ui| {
            for (a, nome, dica) in [(Aba::Estados, "Estados", "O que aparece, com que cor e de que ponto de vista."), (Aba::Explosao, "Explosão", "Passos que afastam as peças, com linhas automáticas."), (Aba::Animacao, "Animação", "Quadros-chave numa linha do tempo.")] {
                if ui.selectable_label(st.exibicao.aba == a, RichText::new(tr(nome)).strong()).on_hover_text(tr(dica)).clicked() {
                    st.exibicao.aba = a;
                }
            }
        });
        ui.separator();
        egui::ScrollArea::vertical().max_height(area.height() * 0.7).auto_shrink([false, true]).show(ui, |ui| match st.exibicao.aba {
            Aba::Estados => aba_estados(ui, st, cam, &mut cam_nova),
            Aba::Explosao => aba_explosao(ui, st, &selecao),
            Aba::Animacao => aba_animacao(ui, st, cam),
        });
    });
    // reprodução
    if st.anim_play {
        if let Some(a) = st.asm.animations.get(st.anim_sel) {
            st.anim_t += ctx.input(|i| i.stable_dt) as f64;
            let d = a.duration();
            if st.anim_t > d {
                st.anim_t = 0.;
            }
            if let Some(s) = a.at(st.anim_t) {
                for n in &mut st.asm.nodes {
                    n.visible = !s.hidden.contains(&n.id);
                }
                cam_nova = s.camera;
                st.live = s;
                st.dirty = true;
            }
            ctx.request_repaint();
        }
    }
    if st.live != before {
        st.dirty = true;
    }
    if fechar {
        st.display_open = false;
        st.anim_play = false;
        st.explode_mode = false;
    }
    if let Some(p) = saida.movido {
        app.settings.guardar_painel(PAINEL, p);
    }
    if let Some(c) = cam_nova {
        app.camera_set(c);
    }
}

/// Linhas pontilhadas, e a alça do passo escolhido.
pub fn overlay(app: &mut App, ui: &mut egui::Ui, rect: Rect, cam: &Camera) {
    let Some(st) = app.asm.as_mut() else { return };
    let v = ui.visuals().clone();
    let p = ui.painter_at(rect);
    let traco = Stroke::new(1.4, v.text_color().gamma_multiply(0.75));
    for l in &st.live.linhas {
        if st.live.hidden.contains(&l.comp) {
            continue;
        }
        let pontos: Vec<Pos2> = l.pontos.iter().filter_map(|q| cam.project(rect, v3(*q))).collect();
        p.extend(egui::Shape::dashed_line(&pontos, traco, TRACO, VAO));
    }
    if !st.display_open || st.exibicao.aba != Aba::Explosao {
        return;
    }
    let Some(k) = st.exibicao.passo else { return };
    let c = centros(st);
    let Some(e) = st.asm.explosoes.get(st.exibicao.explosao) else { return };
    let Some((onde, dir)) = e.seta(k, &c) else { return };
    let dist = e.passos[k].modo.dist();
    let Some(em) = cam.project(rect, v3(onde)) else { return };
    let Some((_, na_tela, ppm)) = crate::gizmo::screen_axis(cam, rect, v3(onde), v3(unit(dir))) else { return };
    let cor = crate::theme::accent(&v);
    let ponta = em + na_tela * 34.;
    crate::gizmo::arrow(&p, em, ponta, cor);
    let r = ui.interact(Rect::from_center_size(ponta, egui::vec2(crate::handles::PEGA, crate::handles::PEGA)), egui::Id::unique(("alca_passo", k)), egui::Sense::drag()).on_hover_text(tr("Arraste para mudar a distância do passo"));
    p.circle_filled(ponta, if r.hovered() || r.dragged() { 6.5 } else { 4.5 }, cor);
    p.text(ponta + egui::vec2(9., -9.), egui::Align2::LEFT_BOTTOM, format!("{} mm", fmt(dist)), egui::FontId::proportional(12.), crate::theme::readable(cor, &v));
    if r.drag_started() {
        st.push_undo();
        st.exibicao.alca = Some(dist);
    }
    if let (Some(d0), true) = (st.exibicao.alca, r.dragged()) {
        let (Some(m), Some(o)) = ui.input(|i| (i.pointer.interact_pos(), i.pointer.press_origin())) else { return };
        let passo = crate::ui_ext::passo_3d(ui, ppm);
        let novo = ((d0 + ((m - o).dot(na_tela) / ppm) as f64) / passo).round() * passo;
        let i = st.exibicao.explosao;
        st.asm.explosoes[i].passos[k].modo.set_dist(novo.max(0.));
        st.modified = true;
        aplicar(st);
    }
    if r.drag_stopped() {
        st.exibicao.alca = None;
    }
}

/// `FENKRA_DEMO_ASM=explosao|explosao_anim|exibicao`: o painel aberto nas placas de exemplo.
pub fn demo(st: &mut AsmState, nome: &str) {
    let ids: Vec<Id> = st.asm.nodes.iter().filter(|n| !matches!(n.kind, fenkra_montagem::NodeKind::Group)).map(|n| n.id).collect();
    let [_, tampa] = ids[..] else { return };
    let mut e = Explosao::nova(tr("Explosão 1"));
    e.passo_da_seta(vec![tampa], [0., 0., 60.]);
    e.passo_da_seta(vec![tampa], [50., 0., 0.]);
    st.asm.explosoes.push(e);
    st.display_open = true;
    st.exibicao.reaplicar = true;
    st.exibicao.passo = Some(1);
    st.exibicao.aba = match nome {
        "exibicao" => Aba::Estados,
        "explosao_anim" => Aba::Animacao,
        _ => Aba::Explosao,
    };
    if nome == "explosao_anim" {
        let c = centros(st);
        let base = st.asm.snapshot_state(tr("Montada"));
        let a = st.asm.explosoes[0].animacao(&base, &c, SEGUNDOS_POR_PASSO);
        st.asm.animations.push(a);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use fenkra_montagem::{Assembly, NodeKind, Placement};

    fn estado() -> (AsmState, Id, Id) {
        let mut a = Assembly::new("par");
        let b = a.add_component("base", NodeKind::Part { path: "a.fkp".into() }, Placement::default(), None);
        let t = a.add_component("tampa", NodeKind::Part { path: "b.fkp".into() }, Placement { origin: [0., 0., 10.], ..Default::default() }, None);
        (AsmState::new(a, None), b, t)
    }

    #[test]
    fn soltar_a_seta_cria_o_passo_com_as_pecas_escolhidas_e_tem_desfazer() {
        let (mut st, b, t) = estado();
        st.live.set_offset(t, [0., 0., 40.]);
        soltar_seta(&mut st, t, [0.; 3], vec![b, t]);
        let e = &st.asm.explosoes[0];
        assert_eq!(e.passos.len(), 1);
        assert_eq!(e.passos[0].comps, vec![b, t]);
        assert_eq!(st.exibicao.passo, Some(0));
        assert_eq!(st.live.offset(b), [0., 0., 40.], "o grupo inteiro anda");
        assert_eq!(st.live.linhas.len(), 2);
        assert!(!st.undo.is_empty());
    }

    #[test]
    fn peca_arrastada_fora_da_selecao_anda_sozinha() {
        let (mut st, b, t) = estado();
        st.live.set_offset(t, [5., 0., 0.]);
        soltar_seta(&mut st, t, [0.; 3], vec![b]);
        assert_eq!(st.asm.explosoes[0].passos[0].comps, vec![t]);
    }

    #[test]
    fn soltar_sem_andar_nao_cria_passo() {
        let (mut st, _, t) = estado();
        soltar_seta(&mut st, t, [0.; 3], vec![t]);
        assert!(st.asm.explosoes[0].passos.is_empty());
        assert_eq!(st.exibicao.passo, None);
    }

    #[test]
    fn lista_reordena_e_suprime_refazendo_a_vista_e_escolher_nao_gasta_desfazer() {
        let (mut st, _, t) = estado();
        let mut e = Explosao::nova("E");
        e.passo_da_seta(vec![t], [0., 0., 10.]);
        e.passo_da_seta(vec![t], [3., 0., 0.]);
        st.asm.explosoes.push(e);
        aplicar(&mut st);
        assert_eq!(st.live.offset(t), [3., 0., 10.]);
        let n = st.undo.len();
        pedir(&mut st, 0, Lista::Escolher(1));
        assert_eq!(st.undo.len(), n);
        pedir(&mut st, 0, Lista::Suprimir(1));
        assert_eq!(st.live.offset(t), [0., 0., 10.]);
        pedir(&mut st, 0, Lista::Suprimir(1));
        pedir(&mut st, 0, Lista::Mover(1, 0));
        assert_eq!(st.asm.explosoes[0].passos[0].nome, "Passo 2");
        assert_eq!(st.exibicao.passo, Some(0));
        assert_eq!(st.live.linhas[0].pontos[1], [3., 0., 10.], "a ordem nova muda o caminho");
        pedir(&mut st, 0, Lista::Apagar(0));
        assert_eq!(st.live.offset(t), [0., 0., 10.]);
        assert_eq!(st.exibicao.passo, None);
    }

    #[test]
    fn quadro_arrastado_anda_pela_escala_e_nao_vai_antes_do_zero() {
        assert!((tempo_arrastado(1., 50., 100., 4.) - 3.).abs() < 1e-9);
        assert_eq!(tempo_arrastado(1., -500., 100., 4.), 0.);
    }

    #[test]
    fn trocar_o_modo_guarda_direcao_e_distancia() {
        let c: HashMap<Id, V3> = [(Id(1), [0., 0., 0.]), (Id(2), [10., 0., 0.])].into();
        let m = ModoPasso::Linear { dir: [0., 0., 1.], dist: 12. };
        let r = modo_com(m, 1, &[Id(1), Id(2)], &c);
        assert_eq!(r, ModoPasso::Radial { centro: [5., 0., 0.], eixo: [0., 0., 1.], dist: 12. });
        assert_eq!(modo_com(r, 2, &[Id(1)], &c), ModoPasso::Espacado { dir: [0., 0., 1.], dist: 12. });
    }

    #[test]
    fn textos_do_painel_tem_traducao() {
        for t in ["Estados", "Explosão", "Animação", "Espaçamento automático", "Linear", "Radial", "Distância", "Direção", "Eixo", "Usar a seleção", "Gerar passos", "Animar os passos", "Explodir", "peça(s)", "Arraste para mudar a ordem", "Arraste para mudar o instante deste quadro", "Arraste para mudar a distância do passo", "arrastar a seta do manipulador cria um passo", "linhas de explosão", "Nenhum passo: ligue a opção acima e arraste a seta de uma peça."] {
            assert!(crate::i18n::tem_traducao(t), "{t}");
        }
    }
}
