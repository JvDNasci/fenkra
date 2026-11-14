// SPDX-License-Identifier: LGPL-2.1-or-later
//! Posições nomeadas da montagem (B123), no painel do estudo de movimento.
//!
//! A lista grava a situação de agora com nome e tempo de transição; cada linha tem ir, subir,
//! descer, atualizar e apagar. Tocar calcula o percurso pela lista e o entrega à mesma reprodução
//! do estudo, onde o cursor de tempo percorre as posições. Exportar grava a tabela do percurso
//! (`animacao.csv`) e uma imagem por quadro da área 3D (`quadro_0001.png`…), que um programa livre
//! de vídeo, como o FFmpeg, junta num filme.

use crate::app::App;
use crate::asm_ui::AsmState;
use crate::i18n::tr;
use eframe::egui::{self, RichText};
use fenkra_core::Id;
use fenkra_montagem::kin::Frame;
use fenkra_montagem::{posicoes, Placement};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Quadros por segundo do percurso que se toca e se exporta.
pub const QUADROS_POR_SEGUNDO: f64 = 24.;
/// Quadros da interface entre pôr a montagem no lugar e pedir a imagem, depois de a vista assentar.
const ASSENTAR: u8 = 3;

/// Exportação em andamento: um quadro por vez, como a gravação dos roteiros de ajuda.
pub struct Exportacao {
    pub pasta: PathBuf,
    quadros: Vec<Frame>,
    k: usize,
    espera: u8,
    pedida: bool,
    captura: Arc<Mutex<Option<Arc<egui::ColorImage>>>>,
    antes: Vec<(Id, Placement)>,
}

/// Estado do painel das posições, guardado na montagem aberta.
#[derive(Default)]
pub struct Estado {
    /// área 3D do último quadro, para recortar as imagens da exportação
    pub area_3d: Option<egui::Rect>,
    pub exportando: Option<Exportacao>,
}

enum Acao {
    Gravar,
    Ir(usize),
    Mover(usize, usize),
    Atualizar(usize),
    Apagar(usize),
    Tocar,
    Exportar,
}

/// Seção "Posições" do painel do estudo. Erro de exportação vai para o aviso do painel.
pub fn secao(st: &mut AsmState, ui: &mut egui::Ui) {
    crate::editor::section(ui, "Posições");
    let mut acao = None;
    let n = st.asm.posicoes.len();
    if n == 0 {
        ui.label(RichText::new(tr("Ponha o mecanismo numa posição e grave: ela guarda o valor de cada junta e de cada cota.")).weak().small());
    }
    let mut mudou = false;
    for i in 0..n {
        ui.horizontal(|ui| {
            let p = &mut st.asm.posicoes[i];
            ui.label(RichText::new(format!("{}.", i + 1)).weak());
            mudou |= ui.add(egui::TextEdit::singleline(&mut p.nome).desired_width(64.)).on_hover_text(tr("Nome da posição")).changed();
            if i > 0 {
                let r = ui.add(crate::ui_ext::dv(&mut p.transicao).range(0.0..=600.0).speed(0.1).suffix(" s").max_decimals(2).dica(tr("Tempo para chegar a esta posição saindo da anterior")));
                mudou |= r.changed();
            } else {
                ui.label(RichText::new(tr("início")).weak().small()).on_hover_text(tr("A primeira posição é o começo do percurso: não tem transição"));
            }
            if ui.small_button("➜").on_hover_text(tr("Ir para esta posição")).clicked() {
                acao = Some(Acao::Ir(i));
            }
            if ui.add_enabled(i > 0, egui::Button::new("⏶").small()).on_hover_text(tr("Subir na lista")).clicked() {
                acao = Some(Acao::Mover(i, i - 1));
            }
            if ui.add_enabled(i + 1 < n, egui::Button::new("⏷").small()).on_hover_text(tr("Descer na lista")).clicked() {
                acao = Some(Acao::Mover(i, i + 1));
            }
            if ui.small_button("⟳").on_hover_text(tr("Atualizar: a posição passa a guardar a situação de agora")).clicked() {
                acao = Some(Acao::Atualizar(i));
            }
            if ui.small_button("🗑").on_hover_text(tr("Apagar a posição")).clicked() {
                acao = Some(Acao::Apagar(i));
            }
        });
    }
    ui.horizontal_wrapped(|ui| {
        if ui.button(tr("+ Gravar posição")).on_hover_text(tr("Guarda a situação de agora no fim da lista")).clicked() {
            acao = Some(Acao::Gravar);
        }
        let dois = n >= 2;
        if ui.add_enabled(dois, egui::Button::new(tr("▶ Tocar posições"))).on_hover_text(tr("Percorre a lista na ordem, com o tempo de transição de cada uma; o cursor de tempo anda por ela")).clicked() {
            acao = Some(Acao::Tocar);
        }
        let livre = st.posicoes_ui.exportando.is_none();
        if ui.add_enabled(dois && livre, egui::Button::new(tr("Exportar animação…"))).on_hover_text(tr("Grava a tabela do percurso e uma imagem por quadro da área 3D, para juntar num vídeo")).clicked() {
            acao = Some(Acao::Exportar);
        }
    });
    if n >= 2 {
        ui.label(RichText::new(format!("{} {} s", tr("Percurso:"), crate::app::fmt(posicoes::duracao(&st.asm.posicoes)))).weak().small());
    }
    if let Some(e) = &st.posicoes_ui.exportando {
        ui.label(RichText::new(format!("{} {}/{}", tr("Exportando quadro"), e.k + 1, e.quadros.len())).small());
    }
    if mudou {
        st.modified = true;
    }
    let Some(acao) = acao else { return };
    match acao {
        Acao::Gravar => {
            st.push_undo();
            st.asm.nova_posicao("");
        }
        Acao::Ir(i) => {
            st.push_undo();
            let p = st.asm.posicoes[i].clone();
            st.report = posicoes::aplicar(&mut st.asm, &p);
            st.dirty = true;
        }
        Acao::Mover(de, para) => {
            st.push_undo();
            st.asm.mover_posicao(de, para);
        }
        Acao::Atualizar(i) => {
            st.push_undo();
            st.asm.atualizar_posicao(i);
        }
        Acao::Apagar(i) => {
            st.push_undo();
            st.asm.posicoes.remove(i);
        }
        Acao::Tocar => tocar(st),
        Acao::Exportar => {
            let pasta = std::env::var("FENKRA_DEMO_EXPORTAR_ANIMACAO").ok().map(PathBuf::from).or_else(|| rfd::FileDialog::new().set_title(tr("Pasta para a animação")).pick_folder());
            if let Some(pasta) = pasta {
                if let Err(e) = exportar(st, pasta) {
                    st.aviso = Some((e, true));
                }
            }
        }
    }
}

/// Calcula o percurso pela lista e o põe na reprodução do estudo.
pub fn tocar(st: &mut AsmState) {
    if st.sim_rest.is_none() {
        st.sim_rest = Some(st.asm.nodes.iter().map(|n| (n.id, n.placement)).collect());
    }
    let lista = st.asm.posicoes.clone();
    st.sim_frames = posicoes::percorrer(&mut st.asm, &lista, QUADROS_POR_SEGUNDO);
    st.sim_t1 = posicoes::duracao(&lista).max(1e-3);
    st.sim_t = 0.;
    st.sim_play = true;
}

/// Começa a exportação: a tabela sai já, e as imagens, um quadro por vez nos próximos quadros.
pub fn exportar(st: &mut AsmState, pasta: PathBuf) -> Result<(), String> {
    std::fs::create_dir_all(&pasta).map_err(|e| format!("{}: {e}", pasta.display()))?;
    let lista = st.asm.posicoes.clone();
    let quadros = posicoes::percorrer(&mut st.asm, &lista, QUADROS_POR_SEGUNDO);
    std::fs::write(pasta.join("animacao.csv"), posicoes::tabela(&st.asm, &quadros)).map_err(|e| e.to_string())?;
    st.sim_play = false;
    let antes = st.asm.nodes.iter().map(|n| (n.id, n.placement)).collect();
    st.posicoes_ui.exportando = Some(Exportacao { pasta, quadros, k: 0, espera: 0, pedida: false, captura: Default::default(), antes });
    Ok(())
}

/// Um passo da exportação por quadro da interface: põe o quadro, espera a vista assentar, pede a
/// imagem e grava. No fim, devolve a montagem como estava.
pub fn passo(app: &mut App, ctx: &egui::Context) {
    let Some(st) = app.asm.as_mut() else { return };
    let Some(mut e) = st.posicoes_ui.exportando.take() else { return };
    ctx.request_repaint();
    let Some(q) = e.quadros.get(e.k) else {
        for (id, p) in &e.antes {
            if let Some(n) = st.asm.node_mut(*id) {
                n.placement = *p;
            }
        }
        st.dirty = true;
        let msg = format!("{} {} → {}", e.quadros.len(), tr("quadros exportados"), e.pasta.display());
        app.toast(msg, false);
        if std::env::var("FENKRA_DEMO_EXPORTAR_ANIMACAO").is_ok() {
            let _ = std::fs::write(e.pasta.join("pronto.txt"), format!("{}\n", e.quadros.len()));
        }
        return;
    };
    if e.pedida {
        let img = e.captura.lock().unwrap().take();
        if let Some(img) = img {
            if let Err(err) = salvar(&e, &img, st.posicoes_ui.area_3d, ctx) {
                app.toast(err, true);
                return;
            }
            e.k += 1;
            e.pedida = false;
            e.espera = 0;
        }
    } else if e.espera == 0 {
        let mut mudou = false;
        for n in &mut st.asm.nodes {
            if let Some(p) = q.placements.get(&n.id) {
                if n.placement != *p {
                    n.placement = *p;
                    mudou = true;
                }
            }
        }
        st.dirty |= mudou;
        // a marca da trajetória no 3D acompanha o quadro exportado
        st.sim_t = q.t;
        e.espera = 1;
    } else if st.vista_em_dia() {
        e.espera += 1;
        if e.espera > ASSENTAR {
            let slot = e.captura.clone();
            ctx.request_screenshot(move |img| {
                *slot.lock().unwrap() = Some(img);
            });
            e.pedida = true;
        }
    }
    if let Some(st) = app.asm.as_mut() {
        st.posicoes_ui.exportando = Some(e);
    }
}

/// Grava a área 3D da captura como `quadro_NNNN.png`.
fn salvar(e: &Exportacao, img: &egui::ColorImage, area: Option<egui::Rect>, ctx: &egui::Context) -> Result<(), String> {
    let [w, h] = img.size;
    let bytes: Vec<u8> = img.pixels.iter().flat_map(|c| c.to_srgba_unmultiplied()).collect();
    let cheia = image::RgbaImage::from_raw(w as u32, h as u32, bytes).ok_or("captura com tamanho inconsistente")?;
    let k = w as f32 / ctx.content_rect().width().max(1.);
    let parte = match area {
        Some(r) => {
            let (x, y) = (((r.left() * k).round() as u32).min(w as u32 - 1), ((r.top() * k).round() as u32).min(h as u32 - 1));
            let (rw, rh) = (((r.width() * k).round() as u32).min(w as u32 - x).max(1), ((r.height() * k).round() as u32).min(h as u32 - y).max(1));
            image::imageops::crop_imm(&cheia, x, y, (rw & !1).max(2).min(w as u32 - x), (rh & !1).max(2).min(h as u32 - y)).to_image()
        }
        None => cheia,
    };
    parte.save(e.pasta.join(format!("quadro_{:04}.png", e.k + 1))).map_err(|err| err.to_string())
}
