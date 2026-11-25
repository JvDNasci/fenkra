// SPDX-License-Identifier: LGPL-2.1-or-later
//! Cena de apresentação na interface (B182): o nó Cena na árvore, o painel dele na coluna da
//! esquerda e a tradução da cena para o estilo da vista.
//!
//! O painel segue a ordem do padrão de comando (B319): suporte (o que recebe a aparência), modo
//! (câmeras, luzes, ambiente ou aparências), parâmetros do modo e avançado. Tudo vai para
//! `Document::cena` e `Document::views`; o fim do quadro registra o desfazer, como em qualquer edição.
//! A vista lê dali a luz principal, o fundo, o chão e o reflexo (`render_efeitos::CenaVista`).

use crate::app::{App, Sel};
use crate::i18n::tr;
use crate::render::{Camera, ViewStyle, Viewport};
use eframe::egui::{self, RichText};
use eframe::egui_wgpu;
use fenkra_core::cena::{self, AlvoAparencia, Cena, Fundo, TipoLuz};
use fenkra_core::{Appearance, Document, NamedView, APPEARANCE_PRESETS};
use std::cell::RefCell;

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
enum Modo {
    #[default]
    Cameras,
    Luzes,
    Ambiente,
    Aparencias,
}

#[derive(Default)]
struct Estado {
    aberto: bool,
    modo: Modo,
    preset: usize,
    demo_lido: bool,
}

thread_local! {
    static ST: RefCell<Estado> = RefCell::new(Estado::default());
}

/// O painel da cena está aberto na coluna.
pub fn aberto() -> bool {
    ST.with(|s| s.borrow().aberto)
}

pub fn abrir() {
    ST.with(|s| s.borrow_mut().aberto = true);
}

pub fn fechar() {
    ST.with(|s| s.borrow_mut().aberto = false);
}

// ---------------------------------------------------------------- câmeras

/// Vista salva com a câmera de agora.
pub fn vista_de(nome: String, c: &Camera) -> NamedView {
    NamedView { name: nome, target: c.target.to_array().map(|v| v as f64), yaw: c.yaw as f64, pitch: c.pitch as f64, dist: c.dist as f64, ortho: c.ortho }
}

/// Câmera exata de uma vista salva (sem reenquadrar: a câmera da cena é a mesma das imagens de passo).
pub fn camera_de(v: &NamedView) -> Camera {
    Camera { target: glam::Vec3::from(v.target.map(|x| x as f32)), yaw: v.yaw as f32, pitch: v.pitch as f32, dist: v.dist as f32, ortho: v.ortho }
}

/// Câmera ativa da cena, para quem desenha a peça fora da vista (imagens de passo, render).
pub fn camera_da_cena(doc: &Document) -> Option<Camera> {
    doc.cena.as_ref()?.camera(&doc.views).map(camera_de)
}

/// Guarda a câmera de agora com nome livre e a deixa ativa na cena.
pub fn guardar_camera(doc: &mut Document, c: &Camera) -> String {
    let nome = cena::nome_de_camera(&doc.views, tr("Câmera"));
    doc.views.push(vista_de(nome.clone(), c));
    doc.cena.get_or_insert_with(Cena::default).camera_ativa = Some(nome.clone());
    nome
}

// ---------------------------------------------------------------- vista

/// Põe a cena do documento no estilo da vista e sobe a imagem panorâmica quando ela muda. O fundo de
/// cor ou degradê só vale nos modos sombreados: análise, arame e linhas ocultas ficam com o do tema.
/// Com imagem panorâmica, o fundo do estilo vira a cor média dela, e a regra de contraste da peça
/// (B425) e a cor do chão passam a valer contra essa cor. Devolve o erro de leitura da imagem, uma vez.
pub fn no_estilo(doc: &Document, style: &mut ViewStyle, vp: &mut Viewport, rs: &egui_wgpu::RenderState) -> Option<String> {
    let caminho = doc.cena.as_ref().and_then(|c| match &c.ambiente.fundo {
        Fundo::Panoramica { caminho, .. } if !caminho.trim().is_empty() => Some(caminho.trim().to_string()),
        _ => None,
    });
    let mut erro = None;
    let carregada = vp.panoramica.as_ref().map(|p| p.0.clone());
    // a mesma imagem que falhou não é lida de novo a cada quadro
    let falhou = caminho.is_some() && FALHOU.with(|f| f.borrow().as_deref() == caminho.as_deref());
    if carregada != caminho && !falhou {
        if let Err(e) = vp.set_panoramica(rs, caminho.as_deref()) {
            FALHOU.with(|f| *f.borrow_mut() = caminho.clone());
            erro = Some(format!("{}: {e}", tr("Não foi possível ler a imagem panorâmica")));
        } else {
            FALHOU.with(|f| *f.borrow_mut() = None);
        }
    }
    let Some(c) = &doc.cena else { return erro };
    style.cena = crate::render_efeitos::CenaVista::de(c);
    let sombreado = matches!(style.mode, crate::render::DisplayMode::SombreadoArestas | crate::render::DisplayMode::Sombreado | crate::render::DisplayMode::SemiRenderizado);
    if sombreado {
        match (&c.ambiente.fundo, &vp.panoramica) {
            (Fundo::Cor(k), _) => {
                (style.bg_top, style.bg_bottom, style.gradient) = (*k, *k, false);
            }
            (Fundo::Degrade(baixo, cima), _) => {
                (style.bg_top, style.bg_bottom, style.gradient) = (*cima, *baixo, true);
            }
            (Fundo::Panoramica { .. }, Some((_, media))) => {
                (style.bg_top, style.bg_bottom, style.gradient) = (*media, *media, false);
            }
            _ => {}
        }
    } else {
        style.cena.panoramica = None;
    }
    erro
}

thread_local! {
    static FALHOU: RefCell<Option<String>> = const { RefCell::new(None) };
}

// ---------------------------------------------------------------- árvore

/// Linha "Cena" na árvore da peça. Devolve se foi clicada (abre o painel).
pub fn no_da_arvore(doc: &Document, ui: &mut egui::Ui) -> bool {
    crate::tree_ui::marca(ui);
    let texto = match &doc.cena {
        Some(c) => {
            let luzes = c.luzes.iter().filter(|l| l.ligada).count();
            let camera = c.camera(&doc.views).map_or(String::new(), |v| format!(" · {}", v.name));
            format!("{} · {luzes} {}{camera}{}", tr("Cena"), tr("luzes"), if c.publicada { format!(" · {}", tr("publicada")) } else { String::new() })
        }
        None => format!("{} ({})", tr("Cena"), tr("de fábrica")),
    };
    crate::tree_ui::linha(ui, "cena", aberto(), texto)
        .on_hover_text(tr("Câmeras salvas, luzes, fundo, chão e aparências de apresentação. A vista, as imagens de passo e o render usam a mesma cena."))
        .clicked()
}

// ---------------------------------------------------------------- painel

/// Alvo da aparência pela escolha atual: a face clicada (referência estável) ou o corpo da árvore.
fn alvo_da_selecao(app: &App) -> Option<(AlvoAparencia, String)> {
    for p in &app.picks {
        if let Some(k) = app.face_key_of(Some(p)) {
            return Some((AlvoAparencia::Face(k), format!("{} ({})", tr("face"), app.open.doc.name_of(k.owner))));
        }
    }
    if let Sel::Collector(id) = app.sel {
        let nome = app.open.doc.collectors.iter().find(|c| c.id == id).map_or_else(String::new, |c| c.name.clone());
        return Some((AlvoAparencia::Corpo(id), format!("{} ({nome})", tr("corpo"))));
    }
    None
}

/// Imagem panorâmica de exemplo (céu em degradê, sol e chão), gerada para capturas sem arquivo.
fn panoramica_de_exemplo() -> Option<String> {
    let (w, h) = (1024u32, 512u32);
    let img = image::RgbaImage::from_fn(w, h, |x, y| {
        let v = y as f32 / h as f32;
        let u = x as f32 / w as f32;
        let sol = (((u - 0.62) * 2.).powi(2) + ((v - 0.28) * 1.).powi(2)).sqrt();
        let c = if v < 0.5 {
            let t = v / 0.5;
            let base = [70. + 120. * t, 120. + 95. * t, 190. + 50. * t];
            let s = (1. - (sol / 0.06).min(1.)) * 255.;
            [base[0].max(s), base[1].max(s), base[2].max(s * 0.9)]
        } else {
            let t = (v - 0.5) / 0.5;
            [120. - 60. * t, 110. - 55. * t, 95. - 45. * t]
        };
        image::Rgba([c[0] as u8, c[1] as u8, c[2] as u8, 255])
    });
    let p = std::env::temp_dir().join("fenkra_panoramica_exemplo.png");
    img.save(&p).ok()?;
    Some(p.display().to_string())
}

/// `FENKRA_DEMO_CENA=chao|reflexo|luz|cores|panoramica|camera`: abre o painel com uma cena de
/// exemplo, para capturas sem mouse. `camera` guarda a vista, gira a câmera e volta pela câmera salva.
fn demo(app: &mut App) {
    let Ok(v) = std::env::var("FENKRA_DEMO_CENA") else { return };
    if app.result.is_none() || app.viewport.is_none() || ST.with(|s| s.borrow().demo_lido) {
        return;
    }
    ST.with(|s| s.borrow_mut().demo_lido = true);
    let mut c = app.open.doc.cena.clone().unwrap_or_default();
    let modo = match v.as_str() {
        "chao" | "reflexo" => {
            c.ambiente.chao = true;
            c.ambiente.reflexo_ligado = v == "reflexo";
            c.ambiente.reflexo = 0.45;
            Modo::Ambiente
        }
        "luz" => {
            if let Some(l) = c.luzes.first_mut() {
                l.tipo = TipoLuz::Direcional;
                l.direcao = [-0.55, 0.35, 0.75];
                l.cor = [255, 200, 150];
                l.intensidade = 1.3;
            }
            c.ambiente.chao = true;
            Modo::Luzes
        }
        "cores" => {
            if let Some(b) = app.open.doc.collectors.first() {
                c.definir_aparencia(AlvoAparencia::Corpo(b.id), APPEARANCE_PRESETS[1].1);
            }
            let chaves: Vec<_> = app.result.as_ref().and_then(|r| r.bodies.first()).map(|b| (0..b.face_owner.len()).filter_map(|f| b.face_key(f)).collect()).unwrap_or_default();
            if let Some(k) = chaves.iter().find(|k| k.role == 1).or(chaves.first()) {
                c.definir_aparencia(AlvoAparencia::Face(*k), APPEARANCE_PRESETS[5].1);
            }
            Modo::Aparencias
        }
        "panoramica" => {
            if let Some(p) = panoramica_de_exemplo() {
                c.ambiente.fundo = Fundo::Panoramica { caminho: p, rotacao_graus: 0., exposicao: 1. };
            }
            c.ambiente.chao = true;
            c.ambiente.reflexo_ligado = true;
            Modo::Ambiente
        }
        _ => Modo::Cameras,
    };
    app.open.doc.cena = Some(c);
    if let Some(cam) = app.viewport.as_ref().map(|v| v.camera) {
        guardar_camera(&mut app.open.doc, &cam);
        if v == "camera" {
            // gira para longe e volta pela câmera salva: a captura tem de sair igual à vista guardada
            if let Some(vp) = app.viewport.as_mut() {
                vp.camera.yaw += 1.9;
                vp.camera.pitch = -0.2;
                vp.camera.dist *= 2.;
                if let Some(c) = camera_da_cena(&app.open.doc) {
                    vp.camera = c;
                }
            }
        }
    }
    app.scene_dirty = true;
    ST.with(|s| {
        let mut e = s.borrow_mut();
        e.aberto = true;
        e.modo = modo;
    });
}

/// Antes de desenhar a coluna: lê o pedido de demonstração.
pub fn preparar(app: &mut App) {
    demo(app);
}

/// Campos do painel da cena, dentro da coluna da esquerda.
pub fn painel(app: &mut App, ui: &mut egui::Ui) {
    let mut c = app.open.doc.cena.clone().unwrap_or_default();
    let cam = app.viewport.as_ref().map(|v| v.camera);
    let alvo = alvo_da_selecao(app);
    let mut ir: Option<Camera> = None;
    let mut guardar = false;
    let mut apagar = None;
    let mut vistas = app.open.doc.views.clone();
    let tema = (app.settings.bg_top, app.settings.bg_bottom);
    ST.with(|st| {
        let mut e = st.borrow_mut();
        // 1. suporte
        ui.label(RichText::new(tr("Suporte")).strong());
        let txt = match &alvo {
            Some((_, n)) => format!("{}: {n}", tr("Recebe a aparência")),
            None => tr("Escolha uma face no 3D ou um corpo na árvore para dar aparência.").to_string(),
        };
        ui.label(RichText::new(txt).small()).on_hover_text(tr("A aparência da face guarda a referência estável dela: continua na face certa depois de editar o comando."));
        ui.separator();
        // 2. modo
        ui.label(RichText::new(tr("Modo")).strong());
        ui.horizontal_wrapped(|ui| {
            ui.selectable_value(&mut e.modo, Modo::Cameras, tr("Câmeras")).on_hover_text(tr("Vistas guardadas com nome. A ativa é a das imagens de passo e do render."));
            ui.selectable_value(&mut e.modo, Modo::Luzes, tr("Luzes")).on_hover_text(tr("A luz principal projeta a sombra suave da vista."));
            ui.selectable_value(&mut e.modo, Modo::Ambiente, tr("Ambiente")).on_hover_text(tr("Fundo (tema, cor, degradê ou imagem panorâmica) e chão com reflexo e sombra."));
            ui.selectable_value(&mut e.modo, Modo::Aparencias, tr("Aparências")).on_hover_text(tr("Cor e acabamento de apresentação por corpo ou face."));
        });
        ui.separator();
        // 3. parâmetros
        ui.label(RichText::new(tr("Parâmetros")).strong());
        match e.modo {
            Modo::Cameras => cameras(ui, &mut c, &mut vistas, cam.is_some(), &mut ir, &mut guardar, &mut apagar),
            Modo::Luzes => luzes(ui, &mut c),
            Modo::Ambiente => ambiente(ui, &mut c, tema),
            Modo::Aparencias => aparencias(ui, &mut c, alvo.as_ref().map(|a| a.0), &mut e.preset),
        }
        // 4. avançado
        ui.add_space(4.);
        egui::CollapsingHeader::new(tr("Avançado")).id_salt("cena_avancado").show(ui, |ui| {
            ui.checkbox(&mut c.publicada, tr("Publicar a cena")).on_hover_text(tr("A montagem e as outras peças podem usar esta cena nas imagens de passo e no render."));
            ui.add(crate::ui_ext::slider(&mut c.ambiente.ambiente, 0.0..=2.0).text(tr("Luz de ambiente"))).on_hover_text(tr("Quanto o céu e o chão clareiam as faces que a luz principal não alcança."));
            if let Fundo::Panoramica { rotacao_graus, exposicao, .. } = &mut c.ambiente.fundo {
                ui.add(crate::ui_ext::slider(rotacao_graus, -180.0..=180.0).suffix("°").text(tr("Giro da imagem"))).on_hover_text(tr("Gira a imagem panorâmica em torno do eixo vertical."));
                ui.add(crate::ui_ext::slider(exposicao, 0.2..=3.0).text(tr("Exposição"))).on_hover_text(tr("Clareia ou escurece a imagem panorâmica e o reflexo dela."));
            }
            if ui.button(tr("Voltar à cena de fábrica")).on_hover_text(tr("Tira luzes, fundo, chão e aparências da cena. As câmeras salvas ficam.")).clicked() {
                c = Cena::default();
            }
        });
        ui.label(RichText::new(tr("Chão, reflexo e sombra no chão aparecem nos modos sombreados. No nível Rápido e ao girar a vista o reflexo sai, para a vista continuar leve.")).weak().small());
    });
    // câmeras: vistas do documento
    if let Some(i) = apagar {
        if c.camera_ativa.as_deref() == vistas.get(i).map(|v| v.name.as_str()) {
            c.camera_ativa = None;
        }
        vistas.remove(i);
    }
    // nome trocado: a câmera ativa segue a vista renomeada
    if let Some(i) = app.open.doc.views.iter().position(|v| Some(&v.name) == c.camera_ativa.as_ref()) {
        if let Some(v) = vistas.get(i).filter(|_| apagar.is_none()) {
            c.camera_ativa = Some(v.name.clone());
        }
    }
    if vistas != app.open.doc.views {
        app.open.doc.views = vistas;
    }
    let fabrica = app.open.doc.cena.is_none() && c == Cena::default();
    if !fabrica && app.open.doc.cena.as_ref() != Some(&c) {
        app.open.doc.cena = Some(c);
    }
    if let (true, Some(k)) = (guardar, cam) {
        guardar_camera(&mut app.open.doc, &k);
    }
    if let (Some(alvo), Some(vp)) = (ir, app.viewport.as_mut()) {
        let antes = vp.camera;
        vp.camera = alvo;
        vp.animate_from(antes);
    }
}

fn cameras(ui: &mut egui::Ui, c: &mut Cena, vistas: &mut [NamedView], tem_vista: bool, ir: &mut Option<Camera>, guardar: &mut bool, apagar: &mut Option<usize>) {
    if ui.add_enabled(tem_vista, egui::Button::new(tr("Guardar a vista atual"))).on_hover_text(tr("Guarda posição, giro, distância e projeção da vista com um nome, e a deixa ativa.")).clicked() {
        *guardar = true;
    }
    for (i, v) in vistas.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            let ativa = c.camera_ativa.as_deref() == Some(v.name.as_str());
            if ui.radio(ativa, "").on_hover_text(tr("Câmera das imagens de passo e do render")).clicked() {
                c.camera_ativa = Some(v.name.clone());
            }
            ui.add(egui::TextEdit::singleline(&mut v.name).desired_width(120.)).on_hover_text(tr("Nome da câmera"));
            if ui.small_button(tr("Ir")).on_hover_text(tr("Leva a vista para esta câmera")).clicked() {
                *ir = Some(camera_de(v));
            }
            if ui.small_button("✖").on_hover_text(tr("Apagar a câmera")).clicked() {
                *apagar = Some(i);
            }
        });
    }
    if vistas.is_empty() {
        ui.label(RichText::new(tr("Nenhuma câmera guardada: a vista livre vale para tudo.")).weak().small());
    }
}

fn luzes(ui: &mut egui::Ui, c: &mut Cena) {
    if c.luzes.is_empty() {
        c.luzes.push(Default::default());
    }
    let l = &mut c.luzes[0];
    ui.checkbox(&mut l.ligada, tr("Luz principal ligada")).on_hover_text(tr("Desligada, só a luz de ambiente e o preenchimento iluminam a peça, e não há sombra."));
    ui.horizontal(|ui| {
        ui.label(tr("Direção")).on_hover_text(tr("Presa à câmera gira com a vista; fixa fica parada no mundo e a sombra fica no lugar."));
        ui.selectable_value(&mut l.tipo, TipoLuz::DaCamera, tr("Presa à câmera"));
        ui.selectable_value(&mut l.tipo, TipoLuz::Direcional, tr("Fixa no mundo"));
    });
    if l.tipo == TipoLuz::Direcional {
        let d = glam::Vec3::from(l.direcao).normalize_or(glam::Vec3::Z);
        let (mut az, mut el) = (d.y.atan2(d.x).to_degrees(), d.z.clamp(-1., 1.).asin().to_degrees());
        let a = ui.add(crate::ui_ext::slider(&mut az, -180.0..=180.0).suffix("°").text(tr("Azimute"))).on_hover_text(tr("Ângulo da luz em volta do eixo vertical, a partir de X."));
        let b = ui.add(crate::ui_ext::slider(&mut el, 5.0..=90.0).suffix("°").text(tr("Elevação"))).on_hover_text(tr("Altura da luz acima do horizonte: baixa dá sombra comprida."));
        if a.changed() || b.changed() {
            let (az, el) = (az.to_radians(), el.to_radians());
            l.direcao = [el.cos() * az.cos(), el.cos() * az.sin(), el.sin()];
        }
    }
    ui.add(crate::ui_ext::slider(&mut l.intensidade, 0.0..=2.5).text(tr("Intensidade"))).on_hover_text(tr("1 é a força de fábrica da luz principal."));
    ui.horizontal(|ui| {
        ui.label(tr("Cor da luz")).on_hover_text(tr("Tinge a luz principal; branco é o de fábrica."));
        ui.color_edit_button_srgb(&mut l.cor);
    });
}

/// `tema`: fundo do tema (em cima, embaixo).
fn ambiente(ui: &mut egui::Ui, c: &mut Cena, tema: ([u8; 3], [u8; 3])) {
    let a = &mut c.ambiente;
    let nome = |f: &Fundo| match f {
        Fundo::Tema => tr("Do tema"),
        Fundo::Cor(_) => tr("Cor"),
        Fundo::Degrade(..) => tr("Degradê"),
        Fundo::Panoramica { .. } => tr("Imagem panorâmica"),
    };
    ui.horizontal(|ui| {
        ui.label(tr("Fundo")).on_hover_text(tr("O fundo do tema garante a peça distinta dele; os outros valem só nesta peça e nos modos sombreados."));
        egui::ComboBox::from_id_salt("cena_fundo").selected_text(nome(&a.fundo)).show_ui(ui, |ui| {
            // cor e degradê nascem das cores do tema, e a pessoa ajusta a partir delas
            for f in [Fundo::Tema, Fundo::Cor(tema.1), Fundo::Degrade(tema.1, tema.0), Fundo::Panoramica { caminho: String::new(), rotacao_graus: 0., exposicao: 1. }] {
                let igual = std::mem::discriminant(&f) == std::mem::discriminant(&a.fundo);
                if ui.selectable_label(igual, nome(&f)).clicked() && !igual {
                    a.fundo = f;
                }
            }
        });
    });
    match &mut a.fundo {
        Fundo::Cor(cor) => {
            ui.horizontal(|ui| {
                ui.label(tr("Cor do fundo"));
                ui.color_edit_button_srgb(cor).on_hover_text(tr("Cor lisa atrás da peça."));
            });
        }
        Fundo::Degrade(baixo, cima) => {
            ui.horizontal(|ui| {
                ui.label(tr("Embaixo"));
                ui.color_edit_button_srgb(baixo).on_hover_text(tr("Cor da base do degradê."));
                ui.label(tr("Em cima"));
                ui.color_edit_button_srgb(cima).on_hover_text(tr("Cor do topo do degradê."));
            });
        }
        Fundo::Panoramica { caminho, .. } => {
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(caminho).desired_width(170.)).on_hover_text(tr("Imagem panorâmica (equirretangular) em PNG ou JPG: vira o fundo e o reflexo das faces."));
                if ui.button(tr("Escolher…")).on_hover_text(tr("Procura a imagem no computador.")).clicked() {
                    if let Some(p) = crate::dialogo_dono::novo().add_filter(tr("Imagem"), &["png", "jpg", "jpeg"]).pick_file() {
                        *caminho = p.display().to_string();
                    }
                }
            });
        }
        Fundo::Tema => {}
    }
    ui.checkbox(&mut a.chao, tr("Chão")).on_hover_text(tr("Plano sob a peça, na altura mais baixa dela. Some quando a câmera fica abaixo dele."));
    ui.add_enabled_ui(a.chao, |ui| {
        ui.checkbox(&mut a.reflexo_ligado, tr("Reflexo no chão")).on_hover_text(tr("O chão reflete a peça. Desenha a peça mais uma vez; sai no nível Rápido e ao girar."));
        ui.add_enabled_ui(a.reflexo_ligado, |ui| {
            ui.add(crate::ui_ext::slider(&mut a.reflexo, 0.05..=1.0).text(tr("Força do reflexo"))).on_hover_text(tr("Pouco: chão quase fosco. Muito: chão de espelho."));
        });
        ui.checkbox(&mut a.sombra_no_chao, tr("Sombra no chão")).on_hover_text(tr("O chão recebe a sombra da luz principal. Precisa da sombra suave ligada e de um nível acima do Rápido."));
    });
}

fn aparencias(ui: &mut egui::Ui, c: &mut Cena, alvo: Option<AlvoAparencia>, preset: &mut usize) {
    *preset = (*preset).min(APPEARANCE_PRESETS.len() - 1);
    ui.horizontal(|ui| {
        ui.label(tr("Aparência")).on_hover_text(tr("Material de apresentação: cor e acabamento."));
        egui::ComboBox::from_id_salt("cena_preset").selected_text(tr(APPEARANCE_PRESETS[*preset].0)).show_ui(ui, |ui| {
            for (i, (n, _)) in APPEARANCE_PRESETS.iter().enumerate() {
                ui.selectable_value(preset, i, tr(n));
            }
        });
    });
    let app: Appearance = APPEARANCE_PRESETS[*preset].1;
    ui.horizontal(|ui| {
        if let Some(a) = alvo {
            if ui.button(tr("Aplicar à escolha")).on_hover_text(tr("Dá a aparência à face ou ao corpo escolhido.")).clicked() {
                c.definir_aparencia(a, app);
            }
            if ui.add_enabled(c.aparencias.iter().any(|x| x.alvo == a), egui::Button::new(tr("Tirar"))).on_hover_text(tr("Volta a escolha à aparência do modelo.")).clicked() {
                c.tirar_aparencia(a);
            }
        } else {
            ui.add_enabled(false, egui::Button::new(tr("Aplicar à escolha"))).on_disabled_hover_text(tr("Escolha uma face no 3D ou um corpo na árvore para dar aparência."));
        }
    });
    let mut tirar = None;
    for (i, a) in c.aparencias.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            ui.color_edit_button_srgb(&mut a.aparencia.color).on_hover_text(tr("Cor de apresentação deste alvo"));
            ui.label(match a.alvo {
                AlvoAparencia::Corpo(_) => tr("corpo"),
                AlvoAparencia::Face(_) => tr("face"),
            });
            ui.label(RichText::new(tr(a.aparencia.finish.label())).weak());
            if ui.small_button("✖").on_hover_text(tr("Tirar esta aparência")).clicked() {
                tirar = Some(i);
            }
        });
    }
    if let Some(i) = tirar {
        c.aparencias.remove(i);
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    /// A câmera nomeada guardada e relida do arquivo restaura a vista: mesma posição do olho, mesmo
    /// alvo e a mesma matriz de vista e projeção, em perspectiva e em ortográfica.
    #[test]
    fn camera_nomeada_restaura_a_vista() {
        for cam in [Camera { target: glam::Vec3::new(12., -4., 30.), yaw: 2.3, pitch: -0.4, dist: 412.5, ortho: false }, Camera { target: glam::Vec3::new(-1., 2., 3.), yaw: -0.785, pitch: 1.2, dist: 80., ortho: true }] {
            let mut d = Document::new("Peça");
            let nome = guardar_camera(&mut d, &cam);
            // outra vista no meio não muda a ativa
            d.views.insert(0, vista_de("Outra".into(), &Camera::default()));
            let lido = Document::from_json(&d.to_json()).unwrap();
            assert_eq!(lido.cena.as_ref().and_then(|c| c.camera_ativa.clone()), Some(nome));
            let volta = camera_da_cena(&lido).expect("câmera ativa");
            assert!((volta.eye() - cam.eye()).length() < 1e-3, "{:?} contra {:?}", volta.eye(), cam.eye());
            assert_eq!(volta.ortho, cam.ortho);
            let (a, b) = (volta.view_proj(1.6).to_cols_array(), cam.view_proj(1.6).to_cols_array());
            assert!(a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-5));
        }
    }

    /// Duas câmeras guardadas ganham nomes diferentes e a última fica ativa.
    #[test]
    fn camera_nova_tem_nome_livre_e_fica_ativa() {
        let mut d = Document::new("Peça");
        let a = guardar_camera(&mut d, &Camera::default());
        let b = guardar_camera(&mut d, &Camera { yaw: 1., ..Camera::default() });
        assert_ne!(a, b);
        assert_eq!(d.cena.as_ref().unwrap().camera_ativa.as_deref(), Some(b.as_str()));
        assert_eq!(camera_da_cena(&d).map(|c| c.yaw), Some(1.));
    }
}
