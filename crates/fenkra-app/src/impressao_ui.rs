// SPDX-License-Identifier: LGPL-2.1-or-later
//! Painel Preparar impressão (B119): mesa, conferência, assentar numa face, mapa de balanço e saída.
//! Aberto só durante o comando. A prévia é desenhada sobre a peça: a mesa e as faces sem apoio vão
//! para a sobreposição da cena, no lugar em que a peça foi modelada.

use crate::app::App;
use crate::i18n::tr;
use crate::ui_ext::dv;
use eframe::egui::{self, Rect, RichText};
use fenkra_conversor::impressao::{abrir_no_fatiador, arquivo_para_fatiador, gravar_3mf, gravar_stl, planejar, OpcoesImpressao, PlanoImpressao};
use fenkra_core::Id;
use std::collections::HashMap;

const LARGURA: f32 = 320.;
/// Teto de triângulos no mapa: passou disso, a sobreposição pesa mais que a peça.
const TETO_MAPA: usize = 150_000;

pub struct Painel {
    pub opcoes: OpcoesImpressao,
    /// normal da face que vai para a mesa, por coletor
    assentos: HashMap<Id, [f64; 3]>,
    plano: Result<PlanoImpressao, String>,
    /// opção mudou: o plano refaz quando a pessoa soltar o campo
    sujo: bool,
    mapa: bool,
}

#[derive(Clone, Debug, PartialEq)]
enum Acao {
    Nada,
    Fechar,
    Gravar3mf,
    GravarStl,
    Fatiador,
    /// assentar o corpo do coletor na face escolhida na viewport
    Assentar(Id),
    /// assentar o corpo do coletor na maior face plana
    MaiorFace(Id),
    Soltar(Id),
    EscolherFatiador,
}

fn numero(ui: &mut egui::Ui, nome: &str, v: &mut f64, faixa: std::ops::RangeInclusive<f64>, sufixo: &str, dica: &str) -> bool {
    ui.horizontal(|ui| {
        ui.add_sized([104., 18.], egui::Label::new(tr(nome)));
        ui.add(dv(v).speed(0.1).range(faixa).suffix(sufixo).dica(tr(dica))).changed()
    })
    .inner
}

fn texto(ui: &mut egui::Ui, nome: &str, v: &mut String) {
    ui.horizontal(|ui| {
        ui.add_sized([104., 18.], egui::Label::new(tr(nome)));
        ui.add(egui::TextEdit::singleline(v).desired_width(180.));
    });
}

impl Painel {
    fn ui(&mut self, ui: &mut egui::Ui, fatiador: &mut String, tem_face: bool) -> Acao {
        let mut acao = Acao::Nada;
        let o = &mut self.opcoes;
        let mut mudou = false;
        ui.label(RichText::new(tr("Malha")).strong());
        mudou |= numero(ui, "Desvio da malha", &mut o.desvio, 0.001..=2., " mm", "Quanto a malha pode se afastar da superfície exata. Menor é mais fiel e mais pesado.");
        mudou |= numero(ui, "Ângulo da malha", &mut o.angulo, 1.0..=60., " °", "Ângulo máximo entre triângulos vizinhos numa superfície curva. Menor deixa a curva mais lisa.");
        ui.add_space(4.);
        ui.label(RichText::new(tr("Mesa")).strong());
        for (i, eixo) in ["Largura", "Profundidade", "Altura"].iter().enumerate() {
            mudou |= numero(ui, eixo, &mut o.mesa[i], 10.0..=2000., " mm", "Volume de impressão da máquina.");
        }
        mudou |= numero(ui, "Espaço entre peças", &mut o.espaco, 0.0..=50., " mm", "Distância mínima entre as peças na mesa.");
        mudou |= numero(ui, "Balanço máximo", &mut o.balanco, 0.0..=89., " °", "Inclinação que a impressora vence sem suporte, medida da vertical. Face mais deitada que isto aparece no mapa.");
        ui.checkbox(&mut self.mapa, tr("Mostrar a mesa e o mapa de balanço")).on_hover_text(tr("Desenha sobre a peça as faces sem apoio e o contorno da mesa."));
        ui.add_space(4.);
        ui.label(RichText::new(tr("Conferência")).strong());
        match &self.plano {
            Err(e) => {
                ui.colored_label(crate::theme::st_err(), e);
            }
            Ok(p) => {
                for c in &p.corpos {
                    let cor = if !c.coube || !c.conferencia.pronta() { crate::theme::st_err() } else if c.conferencia.achados.is_empty() { crate::theme::st_ok() } else { crate::theme::st_warn() };
                    ui.colored_label(cor, RichText::new(&c.nome).strong());
                    ui.label(RichText::new(format!("{} {} · {} {:.1}%", c.unida.tris.len(), tr("triângulos"), tr("balanço"), c.balanco.fracao * 100.)).small());
                    if !c.coube {
                        ui.colored_label(crate::theme::st_err(), RichText::new(tr("A peça não cabe na mesa.")).small());
                    }
                    for a in c.conferencia.achados.iter().filter(|a| c.coube || **a != fenkra_malha::impressao::Achado::ForaDaMesa) {
                        ui.colored_label(if a.grave() { crate::theme::st_err() } else { crate::theme::st_warn() }, RichText::new(tr(a.texto())).small());
                    }
                    if c.coube && c.conferencia.achados.is_empty() {
                        ui.colored_label(crate::theme::st_ok(), RichText::new(tr("Pronta para imprimir.")).small());
                    }
                    ui.horizontal_wrapped(|ui| {
                        if ui.add_enabled(tem_face, egui::Button::new(tr("Assentar na face"))).on_hover_text(tr("Selecione uma face da peça na viewport: ela vai para a mesa.")).on_disabled_hover_text(tr("Selecione uma face da peça na viewport: ela vai para a mesa.")).clicked() {
                            acao = Acao::Assentar(c.coletor);
                        }
                        if ui.button(tr("Maior face")).on_hover_text(tr("Deita a peça na maior face plana.")).clicked() {
                            acao = Acao::MaiorFace(c.coletor);
                        }
                        if self.assentos.contains_key(&c.coletor) && ui.button(tr("Como modelada")).on_hover_text(tr("Volta a peça à orientação do modelo.")).clicked() {
                            acao = Acao::Soltar(c.coletor);
                        }
                    });
                }
            }
        }
        ui.add_space(4.);
        let o = &mut self.opcoes;
        egui::CollapsingHeader::new(tr("Arquivo")).id_salt("impressao_arquivo").default_open(false).show(ui, |ui| {
            texto(ui, "Título", &mut o.titulo);
            texto(ui, "Autor", &mut o.autor);
            texto(ui, "Descrição", &mut o.descricao);
            ui.checkbox(&mut o.miniatura, tr("Miniatura")).on_hover_text(tr("Grava no 3MF uma imagem da peça, que o gerenciador de arquivos e o fatiador mostram."));
            ui.checkbox(&mut o.cores, tr("Cores por face")).on_hover_text(tr("Grava a cor de cada face como material do 3MF, sem partir a malha."));
        });
        ui.horizontal(|ui| {
            ui.add_sized([104., 18.], egui::Label::new(tr("Fatiador")));
            ui.add(egui::TextEdit::singleline(fatiador).desired_width(130.)).on_hover_text(tr("Programa que abre o 3MF. O Fenkra só o inicia, em outro processo."));
            if ui.button("…").on_hover_text(tr("Procurar o programa")).clicked() {
                acao = Acao::EscolherFatiador;
            }
        });
        ui.add_space(6.);
        let tem = self.plano.as_ref().is_ok_and(|p| p.corpos.iter().any(|c| c.coube));
        ui.horizontal_wrapped(|ui| {
            if ui.add_enabled(tem, egui::Button::new(tr("Exportar 3MF…"))).on_hover_text(tr("Grava em milímetros, com os corpos na mesa, metadados e cores.")).clicked() {
                acao = Acao::Gravar3mf;
            }
            if ui.add_enabled(tem, egui::Button::new(tr("Exportar STL…"))).on_hover_text(tr("Grava STL binário com todos os corpos na mesa, sem cor nem unidade.")).clicked() {
                acao = Acao::GravarStl;
            }
            if ui.add_enabled(tem && !fatiador.trim().is_empty(), egui::Button::new(tr("Abrir no fatiador"))).on_hover_text(tr("Grava um 3MF temporário e abre no programa escolhido.")).clicked() {
                acao = Acao::Fatiador;
            }
            if ui.button(tr("Fechar")).clicked() {
                acao = Acao::Fechar;
            }
        });
        self.sujo |= mudou;
        acao
    }
}

fn inversa_rigida(m: &[f64; 12]) -> [f64; 12] {
    let t = [m[3], m[7], m[11]];
    let r = |i: usize, j: usize| m[j * 4 + i];
    let d = |i: usize| -(r(i, 0) * t[0] + r(i, 1) * t[1] + r(i, 2) * t[2]);
    [r(0, 0), r(0, 1), r(0, 2), d(0), r(1, 0), r(1, 1), r(1, 2), d(1), r(2, 0), r(2, 1), r(2, 2), d(2)]
}

/// Mesa e faces sem apoio. A peça não muda de lugar na viewport: quem se move é a mesa, pela matriz
/// inversa de cada corpo.
pub fn na_cena(p: Option<&Painel>, s: &mut crate::render::Scene) {
    let Some(p) = p.filter(|p| p.mapa) else { return };
    let Ok(plano) = &p.plano else { return };
    let rgba = |c: egui::Color32| [c.r() as f32 / 255., c.g() as f32 / 255., c.b() as f32 / 255., 1.];
    let (aviso, ok) = (rgba(crate::theme::st_warn()), rgba(crate::theme::st_ok()));
    let f32s = |q: [f64; 3]| q.map(|v| v as f32);
    let mesa = p.opcoes.mesa;
    let cantos = [[0., 0., 0.], [mesa[0], 0., 0.], [mesa[0], mesa[1], 0.], [0., mesa[1], 0.]];
    for c in plano.corpos.iter().filter(|c| c.coube).take(1) {
        let volta = inversa_rigida(&c.posicao);
        for i in 0..4 {
            for q in [cantos[i], cantos[(i + 1) % 4]] {
                s.overlay.push(crate::render::LineVertex { pos: f32s(fenkra_malha::cena::aplicar(&volta, q)), col: ok });
            }
        }
    }
    for c in &plano.corpos {
        for t in c.balanco.triangulos.iter().take(TETO_MAPA) {
            let [a, b, d] = c.unida.tris[*t as usize].map(|i| f32s(c.unida.pos[i as usize]));
            for pos in [a, b, b, d, d, a] {
                s.overlay.push(crate::render::LineVertex { pos, col: aviso });
            }
        }
    }
}

impl App {
    pub fn abrir_impressao(&mut self) {
        let opcoes = self.settings.impressao.clone();
        let plano = planejar(&self.open.doc, &opcoes, &HashMap::new()).map_err(|e| e.to_string());
        self.impressao = Some(Painel { opcoes, assentos: HashMap::new(), plano, sujo: false, mapa: true });
        self.started = true;
    }

    /// Normal da face escolhida na viewport.
    fn normal_escolhida(&self) -> Option<[f64; 3]> {
        self.picks.iter().find_map(|p| if let crate::picking::Pick::Face { normal, .. } = p { Some(normal.map(f64::from)) } else { None })
    }

    pub fn painel_impressao(&mut self, ctx: &egui::Context, area: Rect) {
        use crate::paineis::{self, Encaixe, Painel as Cfg};
        let Some(mut d) = self.impressao.take() else { return };
        let cfg = Cfg { nome: "impressao".into(), titulo: tr("Preparar impressão").to_string(), padrao: paineis::abaixo_do_cubo(area, LARGURA), largura: LARGURA, redimensiona: false };
        let posicao = self.settings.painel("impressao", Encaixe::Direita);
        let tem_face = self.normal_escolhida().is_some();
        let mut fatiador = self.settings.fatiador.clone();
        let saida = paineis::mostrar(ctx, area, &cfg, posicao, |ui| d.ui(ui, &mut fatiador, tem_face));
        self.settings.fatiador = fatiador;
        if let Some(p) = saida.movido {
            self.settings.guardar_painel("impressao", p);
        }
        let mut refazer = false;
        let nome = self.open.doc.name.clone();
        let mut acao = saida.conteudo.unwrap_or(Acao::Nada);
        // desenvolvimento: grava sozinho, para conferir a saída sem mouse
        static GRAVOU: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
        if let (Acao::Nada, Ok(_)) = (&acao, std::env::var("FENKRA_DEMO_IMPRESSAO_ARQUIVO")) {
            if ctx.cumulative_pass_nr() > 30 && !GRAVOU.swap(true, std::sync::atomic::Ordering::Relaxed) {
                acao = Acao::Gravar3mf;
            }
        }
        match acao {
            Acao::Nada => {}
            Acao::Fechar => {
                self.settings.impressao = d.opcoes;
                return;
            }
            Acao::Assentar(c) => {
                if let Some(n) = self.normal_escolhida() {
                    d.assentos.insert(c, n);
                    refazer = true;
                }
            }
            Acao::MaiorFace(c) => {
                let base = d.plano.as_ref().ok().and_then(|p| p.corpos.iter().find(|x| x.coletor == c)).and_then(|x| fenkra_malha::impressao::melhor_base(&x.unida));
                if let Some(n) = base {
                    d.assentos.insert(c, n);
                    refazer = true;
                }
            }
            Acao::Soltar(c) => {
                d.assentos.remove(&c);
                refazer = true;
            }
            Acao::EscolherFatiador => {
                if let Some(p) = rfd::FileDialog::new().pick_file() {
                    self.settings.fatiador = p.to_string_lossy().into();
                }
            }
            Acao::Gravar3mf | Acao::GravarStl => {
                let stl = acao == Acao::GravarStl;
                let extensao = if stl { "stl" } else { "3mf" };
                let demo = std::env::var("FENKRA_DEMO_IMPRESSAO_ARQUIVO").ok().map(std::path::PathBuf::from);
                let destino = demo.or_else(|| rfd::FileDialog::new().add_filter(extensao.to_uppercase(), &[extensao]).set_file_name(format!("{nome}.{extensao}")).save_file());
                if let (Some(caminho), Ok(p)) = (destino, &d.plano) {
                    let r = if stl { gravar_stl(p, &nome, &caminho) } else { gravar_3mf(p, &d.opcoes, &nome, &caminho) };
                    match r {
                        Ok(()) => self.toast(format!("Exportado: {}", caminho.display()), false),
                        Err(e) => self.toast(format!("Falha ao exportar: {e}"), true),
                    }
                }
            }
            Acao::Fatiador => {
                if let Ok(p) = &d.plano {
                    let arquivo = arquivo_para_fatiador(&nome);
                    let pronto = arquivo.parent().map_or(Ok(()), std::fs::create_dir_all).map_err(|e| e.to_string()).and_then(|()| gravar_3mf(p, &d.opcoes, &nome, &arquivo).map_err(|e| e.to_string())).and_then(|()| abrir_no_fatiador(&self.settings.fatiador, &arquivo).map_err(|e| e.to_string()));
                    if let Err(e) = pronto {
                        self.toast(e, true);
                    }
                }
            }
        }
        // refaz só com o campo solto: arrastar a mesa não recalcula a conferência a cada pixel
        let solto = !ctx.input(|i| i.pointer.any_down()) && !ctx.egui_wants_keyboard_input();
        if refazer || (d.sujo && solto) {
            d.plano = planejar(&self.open.doc, &d.opcoes, &d.assentos).map_err(|e| e.to_string());
            d.sujo = false;
        }
        self.impressao = Some(d);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inversa_desfaz_a_matriz_da_mesa() {
        let m = fenkra_malha::impressao::giro_para_baixo([1., 2., 3.]);
        let mut m = m;
        (m[3], m[7], m[11]) = (5., -7., 9.);
        let v = inversa_rigida(&m);
        let p = [3., 4., 5.];
        let q = fenkra_malha::cena::aplicar(&v, fenkra_malha::cena::aplicar(&m, p));
        assert!((0..3).all(|i| (p[i] - q[i]).abs() < 1e-9), "{q:?}");
    }

    #[test]
    fn textos_da_conferencia_tem_traducao() {
        use fenkra_malha::impressao::Achado::*;
        crate::i18n::set_english(true);
        for a in [NaoEstanque, NaoManifold, Normais, PartesSoltas, Cruzamentos, ForaDaMesa] {
            assert_ne!(tr(a.texto()), a.texto());
        }
        crate::i18n::set_english(false);
    }
}
