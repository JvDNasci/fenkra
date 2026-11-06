// SPDX-License-Identifier: LGPL-2.1-or-later
//! Estrutural: aparar e estender uma ponta de membro até um limite (B37), dentro da janela
//! "Cantos da estrutura". O usuário escolhe a ponta, a ação e o limite (um plano ou o eixo de outro
//! membro); o 3D mostra o limite destacado, a seta do lado que sobra, o trecho que sai em vermelho
//! translúcido e o que entra em verde. "Aplicar" grava o prolongamento no comando, com desfazer.

use crate::app::App;
use crate::i18n::tr;
use crate::theme;
use eframe::egui::{self, Color32, RichText, Stroke};
use fenkra_core::{FeatureKind, Id};
use fenkra_solidos::st_frames::{member_frames, MemberFrame};
use fenkra_solidos::st_limite::{self, Acao, Limite, Ponta, Resultado};

type Acao3d = Box<dyn FnOnce(&mut fenkra_core::Document)>;

#[derive(Clone, Copy, PartialEq)]
enum TipoLimite {
    Plano,
    Membro,
}

#[derive(Clone)]
struct Estado {
    membro: Option<(Id, usize)>,
    ponta: Ponta,
    acao: Acao,
    tipo: TipoLimite,
    eixo: usize,
    posicao: f64,
    posicao_texto: String,
    outro: Option<(Id, usize)>,
    alem: bool,
}

impl Default for Estado {
    fn default() -> Self {
        let mut e = Estado { membro: None, ponta: Ponta::Fim, acao: Acao::Aparar, tipo: TipoLimite::Plano, eixo: 2, posicao: 0., posicao_texto: String::new(), outro: None, alem: false };
        // captura sem mouse: FENKRA_DEMO_LIMITE=<x|y|z>:<mm>[:estender]
        if let Ok(v) = std::env::var("FENKRA_DEMO_LIMITE") {
            let mut it = v.split(':');
            if let Some(eixo) = it.next().and_then(|a| ["x", "y", "z"].iter().position(|n| *n == a)) {
                e.eixo = eixo;
            }
            if let Some(p) = it.next().and_then(|p| p.parse::<f64>().ok()) {
                e.posicao = p;
                e.posicao_texto = format!("{p:.1}");
            }
            if it.next() == Some("estender") {
                e.acao = Acao::Estender;
            }
        }
        e
    }
}

/// O que o 3D desenha: guardado entre a janela e o viewport.
#[derive(Clone)]
struct Previa {
    res: Resultado,
    limite: Limite,
    largura: f64,
}

fn id_estado() -> egui::Id {
    egui::Id::new("st_limite_estado")
}
fn id_previa() -> egui::Id {
    egui::Id::new("st_limite_previa")
}

/// Apaga a prévia (a janela fechou ou a seção está recolhida).
pub fn limpar(ctx: &egui::Context) {
    ctx.data_mut(|d| d.remove::<Previa>(id_previa()));
}

fn linhas_de_membros(app: &App) -> Vec<(Id, usize, MemberFrame)> {
    let d = app.doc();
    let mut out = vec![];
    for (k, f) in d.features.iter().enumerate() {
        if !d.is_active(k) || !matches!(f.kind, FeatureKind::StructMember(_)) {
            continue;
        }
        out.extend(member_frames(d, f.id).into_iter().map(|fr| (f.id, fr.line, fr)));
    }
    out
}

fn nome(app: &App, id: Id, linha: usize) -> String {
    format!("{} · {} {}", app.doc().name_of(id), tr("linha"), linha + 1)
}

/// Seção da janela dos cantos. `novo` recebe a mudança no documento; `aviso` o texto do aviso.
pub fn secao(ui: &mut egui::Ui, app: &App, novo: &mut Option<Acao3d>, aviso: &mut Option<String>) {
    let linhas = linhas_de_membros(app);
    let aberto = std::env::var("FENKRA_DEMO_LIMITE").is_ok();
    let cab = egui::CollapsingHeader::new(tr("Aparar / estender até limite")).id_salt("st_limite_cab").default_open(aberto).show(ui, |ui| {
        if linhas.is_empty() {
            ui.label(RichText::new(tr("Nenhum membro na estrutura.")).weak());
            return false;
        }
        let mut e: Estado = ui.ctx().data(|d| d.get_temp(id_estado())).unwrap_or_default();
        if e.membro.is_none_or(|m| !linhas.iter().any(|l| (l.0, l.1) == m)) {
            e.membro = Some((linhas[0].0, linhas[0].1));
        }
        let atual = e.membro.and_then(|m| linhas.iter().find(|l| (l.0, l.1) == m)).unwrap();
        ui.horizontal_wrapped(|ui| {
            ui.label(tr("Membro"));
            egui::ComboBox::from_id_salt("st_limite_membro").selected_text(nome(app, atual.0, atual.1)).width(190.).show_ui(ui, |ui| {
                for l in &linhas {
                    if ui.selectable_label(Some((l.0, l.1)) == e.membro, nome(app, l.0, l.1)).clicked() {
                        e.membro = Some((l.0, l.1));
                    }
                }
            });
        });
        ui.horizontal(|ui| {
            ui.selectable_value(&mut e.ponta, Ponta::Inicio, tr("Início")).on_hover_text(tr("A ponta onde a linha guia começa."));
            ui.selectable_value(&mut e.ponta, Ponta::Fim, tr("Fim")).on_hover_text(tr("A ponta onde a linha guia termina."));
            ui.separator();
            ui.selectable_value(&mut e.acao, Acao::Aparar, tr("Aparar")).on_hover_text(tr("Encurta o membro até o limite; o trecho que sai aparece em vermelho."));
            ui.selectable_value(&mut e.acao, Acao::Estender, tr("Estender")).on_hover_text(tr("Alonga o membro até o limite; o trecho que entra aparece em verde."));
        });
        ui.horizontal(|ui| {
            ui.label(tr("Limite"));
            ui.selectable_value(&mut e.tipo, TipoLimite::Plano, tr("Plano")).on_hover_text(tr("Um plano perpendicular a um dos eixos do mundo, numa posição em mm."));
            ui.selectable_value(&mut e.tipo, TipoLimite::Membro, tr("Outro membro")).on_hover_text(tr("A ponta para na face do outro membro, medida a partir do eixo dele."));
        });
        let fr = &atual.2;
        let mut recuo = 0.;
        let limite = match e.tipo {
            TipoLimite::Plano => {
                ui.horizontal(|ui| {
                    ui.label(tr("Plano"));
                    for (i, n) in ["X", "Y", "Z"].iter().enumerate() {
                        ui.selectable_value(&mut e.eixo, i, *n);
                    }
                    if e.posicao_texto.is_empty() {
                        let p = if e.ponta == Ponta::Inicio { fr.start } else { fr.end };
                        e.posicao = p[e.eixo];
                        e.posicao_texto = format!("{:.1}", e.posicao);
                    }
                    let r = ui.add(egui::TextEdit::singleline(&mut e.posicao_texto).desired_width(70.)).on_hover_text(tr("Posição do plano ao longo do eixo, em mm."));
                    if r.changed() {
                        if let Ok(v) = e.posicao_texto.trim().replace(',', ".").parse::<f64>() {
                            e.posicao = v;
                        }
                    }
                    ui.label("mm");
                });
                let mut n = [0.; 3];
                n[e.eixo] = 1.;
                let mut p = [0.; 3];
                p[e.eixo] = e.posicao;
                Some(Limite::Plano { ponto: p, normal: n })
            }
            TipoLimite::Membro => {
                let outros: Vec<&(Id, usize, MemberFrame)> = linhas.iter().filter(|l| (l.0, l.1) != (atual.0, atual.1)).collect();
                if outros.is_empty() {
                    ui.label(RichText::new(tr("Não há outro membro.")).weak());
                    None
                } else {
                    if e.outro.is_none_or(|o| !outros.iter().any(|l| (l.0, l.1) == o)) {
                        e.outro = Some((outros[0].0, outros[0].1));
                    }
                    let o = outros.iter().find(|l| Some((l.0, l.1)) == e.outro).unwrap();
                    egui::ComboBox::from_id_salt("st_limite_outro").selected_text(nome(app, o.0, o.1)).width(190.).show_ui(ui, |ui| {
                        for l in &outros {
                            if ui.selectable_label(Some((l.0, l.1)) == e.outro, nome(app, l.0, l.1)).clicked() {
                                e.outro = Some((l.0, l.1));
                            }
                        }
                    });
                    // a face do outro perfil: meia largura no sentido do membro que chega
                    recuo = o.2.w.min(o.2.h) / 2.;
                    Some(Limite::Eixo { a: o.2.start, b: o.2.end, recuo })
                }
            }
        };
        ui.checkbox(&mut e.alem, tr("Prolongar se não alcança")).on_hover_text(tr("Quando o limite fica do lado contrário ao da ação (aparar que precisa alongar, ou o inverso), aplica mesmo assim."));
        let mut tem_previa = false;
        if let Some(l) = limite {
            match st_limite::calcular(fr, e.ponta, e.acao, &l, e.alem) {
                Ok(r) => {
                    tem_previa = true;
                    let _ = recuo;
                    let texto = if r.alcanca {
                        format!("{} {:+.2} mm", tr("Prolongamento novo:"), r.ext_novo)
                    } else {
                        format!("{} {:+.2} mm", tr("Não alcança: o limite pede"), r.ext_novo)
                    };
                    let cor = if r.alcanca { ui.visuals().text_color() } else { theme::st_warn() };
                    ui.label(RichText::new(texto).color(cor));
                    ui.label(RichText::new(format!("{} {:+.2} mm", tr("Hoje:"), r.ext_atual)).weak().small());
                    let pode = (r.alcanca || e.alem) && (r.ext_aplicado - r.ext_atual).abs() > 1e-6;
                    let (id, ponta, valor) = (atual.0, e.ponta, r.ext_aplicado);
                    if ui.add_enabled(pode, egui::Button::new(tr("Aplicar"))).on_hover_text(tr("Grava o prolongamento no membro. Desfazer volta num passo.")).clicked() {
                        *novo = Some(Box::new(move |d| {
                            st_limite::aplicar(d, id, ponta, valor);
                        }));
                        *aviso = Some(tr("Ponta ajustada até o limite").to_string());
                    }
                    let largura = fr.w.max(fr.h);
                    ui.ctx().data_mut(|d| d.insert_temp(id_previa(), Previa { res: r, limite: l, largura }));
                }
                Err(msg) => {
                    ui.label(RichText::new(format!("⛔ {}", &msg)).color(theme::st_err()));
                }
            }
        }
        ui.ctx().data_mut(|d| d.insert_temp(id_estado(), e));
        tem_previa
    });
    if !cab.body_returned.unwrap_or(false) {
        limpar(ui.ctx());
    }
}

fn seta(p: &egui::Painter, de: egui::Pos2, para: egui::Pos2, cor: Color32) {
    p.line_segment([de, para], Stroke::new(3., cor));
    let v = para - de;
    if v.length() < 4. {
        return;
    }
    let u = v.normalized();
    let n = egui::vec2(-u.y, u.x);
    p.add(egui::Shape::convex_polygon(vec![para, para - u * 11. + n * 5., para - u * 11. - n * 5.], cor, Stroke::NONE));
}

/// Desenha a prévia no viewport: o limite, o trecho que sai, o que entra e a seta do lado que sobra.
pub fn draw(ui: &egui::Ui, rect: egui::Rect, vp: &crate::render::Viewport) {
    let Some(pv) = ui.ctx().data(|d| d.get_temp::<Previa>(id_previa())) else { return };
    let proj = |p: [f64; 3]| vp.project(rect, glam::Vec3::from(p.map(|v| v as f32)));
    let pintor = ui.painter();
    let acento = theme::accent(ui.visuals());
    let r = &pv.res;
    let tam = vp.bounds.map(|(a, b)| (b - a).max_element() as f64).unwrap_or(200.).max(60.);
    // limite destacado
    match pv.limite {
        Limite::Plano { normal, .. } => {
            let n = fenkra_solidos::geo3d::unit(normal).unwrap_or([0., 0., 1.]);
            let ref_ = if n[2].abs() > 0.9 { [1., 0., 0.] } else { [0., 0., 1.] };
            let u = fenkra_solidos::geo3d::unit(fenkra_kernel::cross(n, ref_)).unwrap_or([1., 0., 0.]);
            let v = fenkra_kernel::cross(n, u);
            let h = tam * 0.35;
            let c = r.ponto_limite;
            let canto = |a: f64, b: f64| {
                let p = [c[0] + u[0] * a + v[0] * b, c[1] + u[1] * a + v[1] * b, c[2] + u[2] * a + v[2] * b];
                proj(p)
            };
            if let (Some(p1), Some(p2), Some(p3), Some(p4)) = (canto(-h, -h), canto(h, -h), canto(h, h), canto(-h, h)) {
                pintor.add(egui::Shape::convex_polygon(vec![p1, p2, p3, p4], acento.gamma_multiply(0.18), Stroke::new(2., acento)));
            }
        }
        Limite::Eixo { a, b, .. } => {
            if let (Some(p), Some(q)) = (proj(a), proj(b)) {
                pintor.line_segment([p, q], Stroke::new(4., acento));
            }
        }
    }
    // trecho que sai (vermelho translúcido) e o que entra (verde translúcido)
    for (trecho, cor) in [(r.removido, theme::st_err()), (r.acrescentado, theme::st_ok())] {
        if let Some((a, b)) = trecho {
            if let (Some(p), Some(q)) = (proj(a), proj(b)) {
                pintor.line_segment([p, q], Stroke::new(16., cor.gamma_multiply(0.45)));
                pintor.line_segment([p, q], Stroke::new(2., cor));
            }
        }
    }
    // seta do lado que sobra, a partir do ponto do limite
    let comp = tam * 0.12;
    let ponta = [r.ponto_limite[0] + r.lado_que_sobra[0] * comp, r.ponto_limite[1] + r.lado_que_sobra[1] * comp, r.ponto_limite[2] + r.lado_que_sobra[2] * comp];
    if let (Some(p), Some(q)) = (proj(r.ponto_limite), proj(ponta)) {
        seta(pintor, p, q, acento);
        pintor.text(q + egui::vec2(8., -6.), egui::Align2::LEFT_BOTTOM, tr("sobra"), egui::FontId::proportional(crate::acessibilidade::fonte_3d(12.)), acento);
        pintor.circle_filled(p, 5., acento);
    }
    if let Some(q) = proj(r.ponta_atual) {
        let txt = format!("{:+.1} mm", r.ext_novo);
        pintor.text(q + egui::vec2(10., 10.), egui::Align2::LEFT_TOP, txt, egui::FontId::proportional(crate::acessibilidade::fonte_3d(12.)), ui.visuals().text_color());
    }
}
