// SPDX-License-Identifier: LGPL-2.1-or-later
//! Gravação dos roteiros de mídia de ajuda (B257): `FENKRA_DEMO_GRAVAR=<comando>`.
//!
//! O roteiro (`docs/media/roteiros/<comando>.json`) manda o app montar o caso, abrir o comando como
//! novo e mover os campos no tempo do roteiro. O tempo é contado em quadros (`n / fps`), nunca pelo
//! relógio: a cada quadro o app aplica os valores, espera o cálculo e a cena assentarem, pede a
//! captura ao eframe e só então avança. Os quadros saem reduzidos a 360x220 em PPM, que o
//! `scripts/media/montar.py` junta em GIF.

use super::*;
use crate::midia::{self, casos, Roteiro};
use std::sync::{Arc, Mutex};

/// Quanto tempo real a cena precisa parada antes da captura (varredura de recursos, GPU).
const ASSENTAR_S: f64 = 0.7;

enum Fase {
    Janela(u32),
    Base,
    Aplicar,
    Assentar(f64),
    Capturando,
}

pub struct Gravador {
    roteiro: Roteiro,
    pasta: std::path::PathBuf,
    quadro: usize,
    fase: Fase,
    id: Option<Id>,
    base: Option<serde_json::Value>,
    /// B185: caso de cota; o roteiro anima a lista de anotações em vez de um comando
    pmi: Option<Vec<fenkra_core::pmi::Pmi>>,
    captura: Arc<Mutex<Option<Arc<egui::ColorImage>>>>,
}

fn pasta_roteiros() -> std::path::PathBuf {
    std::env::var("FENKRA_DEMO_ROTEIROS").map(Into::into).unwrap_or_else(|_| midia::pasta_docs().join("roteiros"))
}

fn ler_roteiro() -> Option<Result<Roteiro, String>> {
    let nome = std::env::var("FENKRA_DEMO_GRAVAR").ok()?;
    let arq = pasta_roteiros().join(format!("{nome}.json"));
    Some(std::fs::read_to_string(&arq).map_err(|e| format!("{}: {e}", arq.display())).and_then(|t| Roteiro::de_json(&t)))
}

/// Tema do roteiro, para valer antes de a interface ser montada.
pub fn tema() -> Option<String> {
    ler_roteiro()?.ok().map(|r| r.tema)
}

impl Gravador {
    /// Prepara a gravação se `FENKRA_DEMO_GRAVAR` está definida; erro de roteiro vai para a saída de erro.
    pub fn de_ambiente() -> Option<Gravador> {
        let r = match ler_roteiro()? {
            Ok(r) => r,
            Err(e) => {
                eprintln!("gravar: {e}");
                std::process::exit(2);
            }
        };
        let pasta = std::env::var("FENKRA_DEMO_GRAVAR_SAIDA").map(Into::into).unwrap_or_else(|_| std::env::temp_dir().join("fenkra-midia").join(&r.comando));
        let _ = std::fs::remove_dir_all(&pasta);
        if let Err(e) = std::fs::create_dir_all(&pasta) {
            eprintln!("gravar: {}: {e}", pasta.display());
            std::process::exit(2);
        }
        Some(Gravador { roteiro: r, pasta, quadro: 0, fase: Fase::Janela(0), id: None, base: None, pmi: None, captura: Default::default() })
    }
}

impl App {
    /// Documento de partida do roteiro (chamado ao criar o app).
    pub(super) fn gravar_preparar(&mut self) {
        let Some(g) = self.gravador.as_ref() else { return };
        // B185: caso de cota do PMI 3D
        if let Some((doc, lista)) = casos::montar_pmi(&g.roteiro.caso) {
            self.load_doc(doc, None, None);
            self.started = true;
            self.settings.animacoes = false;
            if let Some(g) = self.gravador.as_mut() {
                g.base = serde_json::to_value(&lista).ok();
                g.pmi = Some(lista);
            }
            return;
        }
        let Some((doc, kind)) = casos::montar(&g.roteiro.caso) else {
            eprintln!("gravar: caso {} desconhecido", g.roteiro.caso);
            std::process::exit(2);
        };
        self.load_doc(doc, None, None);
        self.started = true;
        self.settings.animacoes = false;
        if let Some(g) = self.gravador.as_mut() {
            g.base = serde_json::to_value(&kind).ok();
        }
        self.gravar_caso = Some(kind);
    }

    /// Uma etapa por quadro da interface.
    pub(super) fn gravar_passo(&mut self, ctx: &egui::Context) {
        let Some(mut g) = self.gravador.take() else { return };
        ctx.request_repaint();
        let agora = ctx.input(|i| i.time);
        // câmera fixa do roteiro, sobre qualquer enquadramento automático
        if g.id.is_some() {
            let [yaw, pitch, dist, x, y, z] = g.roteiro.camera;
            if let Some(vp) = self.viewport.as_mut() {
                vp.camera.yaw = yaw.to_radians();
                vp.camera.pitch = pitch.to_radians();
                vp.camera.dist = dist;
                vp.camera.target = glam::Vec3::new(x, y, z);
            }
            self.fit_pending = false;
            self.reframe_pending = false;
        }
        match g.fase {
            Fase::Janela(n) => {
                let [w, h] = g.roteiro.janela;
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(w, h)));
                g.fase = if n >= 8 { Fase::Base } else { Fase::Janela(n + 1) };
            }
            Fase::Base => {
                if self.result.is_some() && !self.calculando() {
                    if let Some(lista) = g.pmi.take() {
                        g.id = lista.first().map(|p| p.id);
                        self.open.doc.pmi = lista;
                        self.scene_dirty = true;
                        g.fase = Fase::Aplicar;
                    } else if let Some(kind) = self.gravar_caso.take() {
                        let snapshot = self.open.doc.clone();
                        let id = self.open.doc.add_feature(kind);
                        // B60: roteiro `concluido` mostra o sólido pronto, sem painel nem fantasma
                        if !g.roteiro.concluido {
                            self.start_session(id, true, Some(snapshot));
                        }
                        g.id = Some(id);
                        self.request_recompute();
                        g.fase = Fase::Aplicar;
                    }
                }
            }
            Fase::Aplicar => {
                let t = g.roteiro.tempo(g.quadro);
                let mut mudou = false;
                let cota = casos::montar_pmi(&g.roteiro.caso).is_some();
                if let (true, Some(base)) = (cota, g.base.clone()) {
                    let mut v = base;
                    if let Err(e) = g.roteiro.aplicar(&mut v, t) {
                        eprintln!("gravar: {e}");
                        std::process::exit(3);
                    }
                    match serde_json::from_value::<Vec<fenkra_core::pmi::Pmi>>(v) {
                        Ok(lista) => {
                            if self.open.doc.pmi != lista {
                                self.open.doc.pmi = lista;
                                self.scene_dirty = true;
                            }
                        }
                        Err(e) => {
                            eprintln!("gravar: campo do roteiro deixou a anotação inválida: {e}");
                            std::process::exit(3);
                        }
                    }
                } else if let (Some(id), Some(base)) = (g.id, g.base.clone()) {
                    let mut v = base;
                    if let Err(e) = g.roteiro.aplicar(&mut v, t) {
                        eprintln!("gravar: {e}");
                        std::process::exit(3);
                    }
                    match serde_json::from_value::<FeatureKind>(v) {
                        Ok(novo) => {
                            if let Some(f) = self.open.doc.feature_mut(id) {
                                if f.kind != novo {
                                    f.kind = novo;
                                    mudou = true;
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("gravar: campo do roteiro deixou o comando inválido: {e}");
                            std::process::exit(3);
                        }
                    }
                }
                // B60: campos de outro comando ou de uma referência da peça (ferramenta da booleana,
                // plano de término)
                if !cota {
                    match g.roteiro.aplicar_doc(&mut self.open.doc, t) {
                        Ok(m) => mudou |= m,
                        Err(e) => {
                            eprintln!("gravar: {e}");
                            std::process::exit(3);
                        }
                    }
                }
                if mudou {
                    self.request_recompute();
                    self.scene_dirty = true;
                }
                g.fase = Fase::Assentar(agora);
            }
            Fase::Assentar(desde) => {
                if self.calculando() {
                    g.fase = Fase::Assentar(agora);
                } else if agora - desde >= ASSENTAR_S {
                    let slot = g.captura.clone();
                    ctx.request_screenshot(move |img| {
                        *slot.lock().unwrap() = Some(img);
                    });
                    g.fase = Fase::Capturando;
                }
            }
            Fase::Capturando => {
                let img = g.captura.lock().unwrap().take();
                if let Some(img) = img {
                    if let Err(e) = salvar_quadro(&g, &img, ctx) {
                        eprintln!("gravar: {e}");
                        std::process::exit(4);
                    }
                    g.quadro += 1;
                    if g.quadro >= g.roteiro.quadros() {
                        let _ = std::fs::write(g.pasta.join("pronto.txt"), format!("{}\n", g.quadro));
                        // sai direto: o fechamento gracioso pode travar com a máquina carregada e a
                        // esteira espera o processo terminar
                        std::process::exit(0);
                    } else {
                        g.fase = Fase::Aplicar;
                    }
                }
            }
        }
        self.gravador = Some(g);
    }
}

/// Recorta a área do roteiro, reduz a 360x220 e grava `q0000.ppm`. O primeiro quadro também deixa
/// a janela inteira em `janela.png`, para ajustar o recorte.
fn salvar_quadro(g: &Gravador, img: &egui::ColorImage, ctx: &egui::Context) -> Result<(), String> {
    let [w, h] = img.size;
    let bytes: Vec<u8> = img.pixels.iter().flat_map(|p| p.to_array()).collect();
    let cheia = image::RgbaImage::from_raw(w as u32, h as u32, bytes).ok_or("captura com tamanho inconsistente")?;
    if g.quadro == 0 {
        cheia.save(g.pasta.join("janela.png")).map_err(|e| e.to_string())?;
    }
    // pixels por ponto: a captura sai em pixels físicos
    let k = w as f32 / ctx.content_rect().width().max(1.);
    let [x, y, rw, rh] = g.roteiro.recorte.map(|v| v * k);
    let (x, y) = ((x.round() as u32).min(w as u32 - 1), (y.round() as u32).min(h as u32 - 1));
    let (rw, rh) = ((rw.round() as u32).min(w as u32 - x), (rh.round() as u32).min(h as u32 - y));
    if rw == 0 || rh == 0 {
        return Err("recorte fora da captura".into());
    }
    let parte = image::imageops::crop_imm(&cheia, x, y, rw, rh).to_image();
    let peq = image::imageops::resize(&parte, midia::LARGURA, midia::ALTURA, image::imageops::FilterType::Triangle);
    let mut ppm = format!("P6\n{} {}\n255\n", midia::LARGURA, midia::ALTURA).into_bytes();
    for p in peq.pixels() {
        ppm.extend_from_slice(&p.0[..3]);
    }
    std::fs::write(g.pasta.join(format!("q{:04}.ppm", g.quadro)), ppm).map_err(|e| e.to_string())?;
    if g.quadro == g.roteiro.quadros() / 2 {
        peq.save(g.pasta.join("amostra.png")).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod testes {
    use super::*;

    /// Todo roteiro versionado monta o caso e o campo animado existe no comando: pega erro de
    /// digitação do ponteiro sem abrir janela.
    #[test]
    fn roteiros_do_repositorio_montam_e_aplicam_os_campos() {
        let roteiros = midia::ler_roteiros(&midia::pasta_docs().join("roteiros")).expect("roteiros");
        assert!(roteiros.len() >= 10, "esperava ao menos dez roteiros, achei {}", roteiros.len());
        for (arq, r) in roteiros {
            let nome = arq.file_stem().unwrap().to_string_lossy().to_string();
            assert_eq!(nome, r.comando, "o arquivo deve se chamar como o comando");
            // B185: caso de cota; a lista de anotações tem de mudar e continuar válida
            if let Some((_, lista)) = casos::montar_pmi(&r.caso) {
                let base = serde_json::to_value(&lista).unwrap();
                let (mut ini, mut fim) = (base.clone(), base);
                r.aplicar(&mut ini, 0.).unwrap_or_else(|e| panic!("{nome}: {e}"));
                r.aplicar(&mut fim, r.duracao).unwrap_or_else(|e| panic!("{nome}: {e}"));
                let a: Vec<fenkra_core::pmi::Pmi> = serde_json::from_value(ini).unwrap_or_else(|e| panic!("{nome}: {e}"));
                let b: Vec<fenkra_core::pmi::Pmi> = serde_json::from_value(fim).unwrap_or_else(|e| panic!("{nome}: {e}"));
                assert_ne!(a, b, "{nome}: o roteiro não move nada");
                continue;
            }
            let (doc, kind) = casos::montar(&r.caso).unwrap_or_else(|| panic!("{nome}: caso {} desconhecido", r.caso));
            let base = serde_json::to_value(&kind).unwrap();
            let mut ini = base.clone();
            let mut fim = base.clone();
            r.aplicar(&mut ini, 0.).unwrap_or_else(|e| panic!("{nome}: {e}"));
            r.aplicar(&mut fim, r.duracao).unwrap_or_else(|e| panic!("{nome}: {e}"));
            let k0: FeatureKind = serde_json::from_value(ini).unwrap_or_else(|e| panic!("{nome}: valor inicial inválido: {e}"));
            let k1: FeatureKind = serde_json::from_value(fim).unwrap_or_else(|e| panic!("{nome}: valor final inválido: {e}"));
            // B60: o que se move pode ser um comando ou uma referência da peça do caso
            let (mut d0, mut d1) = (doc.clone(), doc);
            r.aplicar_doc(&mut d0, 0.).unwrap_or_else(|e| panic!("{nome}: {e}"));
            r.aplicar_doc(&mut d1, r.duracao).unwrap_or_else(|e| panic!("{nome}: {e}"));
            assert!(k0 != k1 || d0 != d1, "{nome}: o roteiro não move nada");
        }
    }

    /// O recompute real de cada caso, no início e no fim da animação, fecha sem erro e muda o
    /// volume: a mídia mostra o campo fazendo efeito.
    #[test]
    fn cada_roteiro_recalcula_nos_extremos_e_muda_o_volume() {
        let roteiros = midia::ler_roteiros(&midia::pasta_docs().join("roteiros")).expect("roteiros");
        let mut falhas = vec![];
        for (_, r) in roteiros {
            // as cotas não mudam o sólido (o caso de cota tem teste próprio em `casos`)
            let Some((doc0, kind)) = casos::montar(&r.caso) else { continue };
            let base = serde_json::to_value(&kind).unwrap();
            let mut vol = vec![];
            let mut centro = vec![];
            for t in [0., r.duracao] {
                let mut v = base.clone();
                r.aplicar(&mut v, t).unwrap();
                let mut d = doc0.clone();
                r.aplicar_doc(&mut d, t).unwrap();
                let id = d.add_feature(serde_json::from_value(v).unwrap());
                let res = fenkra_solidos::recompute(&d);
                if matches!(res.status.get(&id), Some(fenkra_solidos::Status::Error(_)) | None) || res.status.values().any(|s| matches!(s, fenkra_solidos::Status::Error(_))) {
                    falhas.push(format!("{}: erro em t={t}: {:?}", r.comando, res.status));
                }
                vol.push(res.bodies.iter().map(|b| b.mass.volume).sum::<f64>());
                let c = res.bodies.iter().fold([0.; 3], |a, b| [a[0] + b.mass.center[0] * b.mass.volume, a[1] + b.mass.center[1] * b.mass.volume, a[2] + b.mass.center[2] * b.mass.volume]);
                centro.push(c.map(|x| x / vol.last().unwrap().max(1e-9)));
            }
            // a rosca cosmética não altera o sólido: o comprimento aparece na prévia, não no volume
            let so_cosmetico = r.comando == "rosca";
            // B60: mover e girar o corpo não mudam o volume: a mídia mostra o corpo saindo do lugar
            let deslocou = (0..3).any(|k| (centro[0][k] - centro[1][k]).abs() > 0.1);
            if !(so_cosmetico || deslocou || (vol[0] - vol[1]).abs() > 1e-6 * vol[0].abs().max(1.)) {
                falhas.push(format!("{}: volume e posição não mudam ({vol:?})", r.comando));
            }
        }
        // todas as falhas numa mensagem só
        assert!(falhas.is_empty(), "{falhas:#?}");
    }
}
