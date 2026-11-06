// SPDX-License-Identifier: LGPL-2.1-or-later
//! Estrutural: corte de encaixe normal entre membros para laser ou plasma em tubo (B155).
//!
//! Vive no cartão do canto ("Cantos da estrutura") quando o tratamento é Encaixe. O cartão escolhe
//! a parede que a curva acompanha, o orçamento de folga (abertura de raiz máxima) e se o corte é
//! perpendicular à parede (laser de 3 eixos) ou com chanfro, e exporta o contorno do tubo aberto em
//! DXF e SVG. No 3D, a curva aparece como prévia fantasma (a ponta que sai e a curva que fica),
//! colorida pela parede em vigor; ao passar o mouse num ponto, a parede, a abertura e o chanfro
//! daquele ponto; ao editar a folga, um rótulo flutuante com o valor em mm.
//!
//! Tudo calculado por `fenkra_solidos::st_encaixe_doc`; aqui só há interface. Os parâmetros
//! (parede, orçamento, perpendicular) ficam na sessão; a folga é a do canto, que já mora no documento.

use crate::app::App;
use crate::i18n::tr;
use crate::theme;
use eframe::egui::{self, Color32, RichText};
use fenkra_solidos::st_cantos::Canto;
use fenkra_solidos::st_dobra_saida;
use fenkra_solidos::st_encaixe::{ModoParede, Opcoes, Parede};
use fenkra_solidos::st_encaixe_doc::{encaixes_da_estrutura, MembroComEncaixe};
use std::sync::Arc;

const OPC_ID: &str = "st_encaixe_opcoes";
const FOCO_ID: &str = "st_encaixe_foco_folga";
const VISUAL_ID: &str = "st_encaixe_visual";

/// Um ponto da curva como o 3D precisa.
#[derive(Clone)]
struct PontoVisual {
    p3: [f64; 3],
    /// onde a ponta nominal acaba (plano do nó), para o fantasma do material que sai
    nominal: [f64; 3],
    parede: Parede,
    excede: bool,
    dica: String,
}

#[derive(Clone)]
struct Visual {
    no: [f64; 3],
    pontos: Vec<PontoVisual>,
    folga: f64,
    editando_folga: bool,
}

/// Apaga a prévia do quadro anterior.
pub fn limpar(ctx: &egui::Context) {
    ctx.data_mut(|d| d.insert_temp::<Option<Arc<Visual>>>(egui::Id::new(VISUAL_ID), None));
}

/// A interface lembra, entre quadros, se o campo da folga está com o foco.
pub fn marcar_foco_folga(ctx: &egui::Context, tem_foco: bool) {
    ctx.data_mut(|d| d.insert_temp(egui::Id::new(FOCO_ID), tem_foco));
}

fn opcoes(ctx: &egui::Context) -> Opcoes {
    ctx.data(|d| d.get_temp::<Opcoes>(egui::Id::new(OPC_ID))).unwrap_or_default()
}

/// Cor da parede em vigor, só pelo tema.
pub fn cor_da_parede(p: Parede, excede: bool, v: &egui::Visuals) -> Color32 {
    if excede {
        return theme::st_err();
    }
    match p {
        Parede::Externa => theme::accent(v),
        Parede::Interna => theme::st_warn(),
        Parede::Media => theme::st_ok(),
        Parede::Livre => v.weak_text_color(),
    }
}

fn nome_da_parede(p: Parede) -> &'static str {
    match p {
        Parede::Externa => "parede externa",
        Parede::Interna => "parede interna",
        Parede::Media => "média das paredes",
        Parede::Livre => "sem contato",
    }
}

/// A ponta de `canto` que é cortada: o membro cortado é o que não continua.
fn do_canto<'a>(todos: &'a [MembroComEncaixe], c: &Canto) -> Option<(&'a MembroComEncaixe, bool)> {
    let continua = c.continua?;
    let cortada = c.linhas.iter().copied().find(|l| *l != continua)?;
    let m = todos.iter().find(|m| (m.feature, m.linha) == cortada)?;
    let perto = |p: [f64; 3]| (0..3).all(|k| (p[k] - c.ponto[k]).abs() < 1e-3);
    if m.ponta1.as_ref().is_some_and(|p| perto(p.no)) {
        Some((m, false))
    } else if m.ponta2.as_ref().is_some_and(|p| perto(p.no)) {
        Some((m, true))
    } else {
        None
    }
}

fn salvar(ui: &egui::Ui, nome: &str, filtro: (&str, &str), bytes: Vec<u8>) {
    let id = egui::Id::new("st_encaixe_msg");
    let Some(caminho) = rfd::FileDialog::new().add_filter(filtro.0, &[filtro.1]).set_file_name(nome).save_file() else { return };
    let msg = match std::fs::write(&caminho, bytes) {
        Ok(()) => format!("{} {}", tr("Salvo em"), caminho.display()),
        Err(e) => format!("{} {e}", tr("Falha ao salvar:")),
    };
    ui.ctx().data_mut(|d| d.insert_temp(id, msg));
}

/// Seção "Corte de encaixe" do cartão do canto. Só aparece com o tratamento Encaixe.
pub fn cartao(ui: &mut egui::Ui, app: &App, c: &Canto) {
    let ctx = ui.ctx().clone();
    let mut opc = opcoes(&ctx);
    let id_cab = egui::Id::new(("st_encaixe_cab", (c.ponto[0] * 10.) as i64, (c.ponto[1] * 10.) as i64, (c.ponto[2] * 10.) as i64));
    egui::CollapsingHeader::new(tr("Corte de encaixe (laser ou plasma em tubo)")).id_salt(id_cab).default_open(true).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(tr("Parede"));
            egui::ComboBox::from_id_salt("st_encaixe_parede").selected_text(tr(opc.modo.nome())).width(150.).show_ui(ui, |ui| {
                for m in ModoParede::TODOS {
                    ui.selectable_value(&mut opc.modo, m, tr(m.nome()));
                }
            })
            .response
            .on_hover_text(tr("Automático: ponto a ponto, a parede que toca primeiro; nunca invade o outro tubo e dá a menor abertura. Externa, interna e média escolhem uma só, e a invasão, se houver, é avisada."));
        });
        ui.horizontal(|ui| {
            ui.label(tr("Orçamento de folga"));
            let r = ui.add(crate::ui_ext::dv(&mut opc.orcamento).speed(0.05).range(0.0..=20.0).suffix(" mm").max_decimals(2).dica(tr("Abertura de raiz máxima admissível, medida na normal do outro tubo. Os trechos acima dela ficam em vermelho: só um chanfro resolve.")));
            crate::ui_ext::wheel(ui, &r, &mut opc.orcamento);
        });
        ui.checkbox(&mut opc.perpendicular, tr("Corte perpendicular à parede")).on_hover_text(tr("Ligado: laser de 3 eixos, uma curva só para a espessura inteira (sobra abertura de raiz). Desligado: com chanfro, a face de corte acompanha o outro tubo e a abertura é zero."));
        ctx.data_mut(|d| d.insert_temp(egui::Id::new(OPC_ID), opc));

        let todos = encaixes_da_estrutura(app.doc(), &opc);
        let Some((m, fim)) = do_canto(&todos, c) else {
            ui.label(RichText::new(tr("Sem curva neste canto: a seção ainda não é tubo ou barra, ou os eixos são paralelos.")).weak());
            return;
        };
        let ponta = if fim { m.ponta2.as_ref() } else { m.ponta1.as_ref() }.unwrap();
        let k = &ponta.corte;
        let [fe, fi, fm, fl] = k.fracao_por_parede();
        ui.horizontal_wrapped(|ui| {
            for (p, f) in [(Parede::Externa, fe), (Parede::Interna, fi), (Parede::Media, fm), (Parede::Livre, fl)] {
                if f <= 0. {
                    continue;
                }
                ui.label(RichText::new(format!("● {} {:.0}%", tr(nome_da_parede(p)), f * 100.)).color(cor_da_parede(p, false, ui.visuals())));
                ui.add_space(6.);
            }
        });
        ui.label(format!("{} {:.2} mm · {} {:.2} mm", tr("Comprimento de corte:"), k.comprimento_corte(), tr("abertura máxima:"), k.folga_max()));
        if opc.perpendicular {
            let ac = k.trechos_acima();
            if ac.is_empty() {
                ui.label(RichText::new(tr("Toda a curva cabe no orçamento de folga.")).color(theme::st_ok()));
            } else {
                ui.label(RichText::new(format!("⚠ {} {} · {}", ac.len(), tr("trecho(s) acima do orçamento"), tr("use chanfro ou aumente o orçamento"))).color(theme::st_err()));
            }
            if k.invasao_max() > 1e-6 {
                ui.label(RichText::new(format!("⛔ {} {:.2} mm", tr("A parede escolhida invade o outro tubo em até"), k.invasao_max())).color(theme::st_err()));
            }
        } else {
            ui.label(format!("{} {:.1}°", tr("Chanfro necessário, no máximo:"), k.bisel_max()));
        }
        let nome = m.designacao.replace(' ', "_");
        ui.horizontal_wrapped(|ui| {
            let d = m.desenho();
            if ui.button(tr("Contorno (DXF)")).on_hover_text(tr("Tubo aberto com a curva de encaixe das pontas, para laser de tubo. Camada CORTE, em milímetros.")).clicked() {
                salvar(ui, &format!("{nome}-encaixe.dxf"), ("DXF", "dxf"), st_dobra_saida::dxf(&d).into_bytes());
            }
            if ui.button(tr("Contorno (SVG)")).on_hover_text(tr("O mesmo contorno em SVG, em milímetros reais.")).clicked() {
                salvar(ui, &format!("{nome}-encaixe.svg"), ("SVG", "svg"), st_dobra_saida::svg(&d).into_bytes());
            }
        });
        if let Some(msg) = ctx.data(|d| d.get_temp::<String>(egui::Id::new("st_encaixe_msg"))) {
            ui.label(RichText::new(msg).weak().small());
        }
        ui.label(RichText::new(format!("⚠ {}", tr("Sem protótipo: a validar em peça real."))).weak().small());

        // dados para o 3D
        let dir = ponta.dir;
        let pontos = k
            .pontos
            .iter()
            .map(|p| PontoVisual {
                p3: p.p3,
                nominal: [p.p3[0] - dir[0] * p.lambda, p.p3[1] - dir[1] * p.lambda, p.p3[2] - dir[2] * p.lambda],
                parede: p.parede,
                excede: p.excede,
                dica: format!(
                    "{} · {} {:.2} mm · {} {:.1}°",
                    tr(nome_da_parede(p.parede)),
                    tr("abertura"),
                    p.folga_raiz,
                    if k.opc.perpendicular { tr("chanfro ideal") } else { tr("chanfro") },
                    if k.opc.perpendicular { p.bisel_ideal.abs() } else { p.bisel.abs() }
                ),
            })
            .collect();
        let editando_folga = ctx.data(|d| d.get_temp::<bool>(egui::Id::new(FOCO_ID))).unwrap_or(false) || std::env::var("FENKRA_DEMO_ENCAIXE").is_ok_and(|v| v == "folga");
        let folga = c.folga;
        ctx.data_mut(|d| d.insert_temp(egui::Id::new(VISUAL_ID), Some(Arc::new(Visual { no: c.ponto, pontos, folga, editando_folga }))));
    });
}

fn rotulo(ui: &egui::Ui, pos: egui::Pos2, texto: &str) {
    let fonte = egui::FontId::proportional(crate::acessibilidade::fonte_3d(12.));
    let g = ui.painter().layout_no_wrap(texto.to_string(), fonte, ui.visuals().text_color());
    let r = egui::Rect::from_min_size(pos, g.size()).expand(4.);
    ui.painter().rect_filled(r, 3., ui.visuals().extreme_bg_color.gamma_multiply(0.92));
    ui.painter().rect_stroke(r, 3., egui::Stroke::new(1., ui.visuals().widgets.noninteractive.bg_stroke.color), egui::StrokeKind::Inside);
    ui.painter().galley(pos, g, ui.visuals().text_color());
}

/// Prévia no 3D do corte do canto escolhido: fantasma do material que sai, curva colorida pela
/// parede em vigor, dica no ponto sob o mouse e rótulo da folga durante a edição.
pub fn desenhar(ui: &egui::Ui, rect: egui::Rect, vp: &crate::render::Viewport) {
    let Some(v) = ui.ctx().data(|d| d.get_temp::<Option<Arc<Visual>>>(egui::Id::new(VISUAL_ID))).flatten() else { return };
    let proj = |p: [f64; 3]| vp.project(rect, glam::Vec3::from(p.map(|x| x as f32)));
    let tela: Vec<Option<egui::Pos2>> = v.pontos.iter().map(|p| proj(p.p3)).collect();
    // fantasma: a ponta nominal e o material que a curva tira
    let fantasma = ui.visuals().weak_text_color().gamma_multiply(0.45);
    let aneis: Vec<egui::Pos2> = v.pontos.iter().filter_map(|p| proj(p.nominal)).collect();
    if aneis.len() > 2 {
        let mut a = aneis.clone();
        a.push(aneis[0]);
        ui.painter().add(egui::Shape::line(a, egui::Stroke::new(1., fantasma)));
    }
    let passo = (v.pontos.len() / 36).max(1);
    for (i, p) in v.pontos.iter().enumerate().step_by(passo) {
        if let (Some(a), Some(b)) = (proj(p.nominal), tela[i]) {
            ui.painter().line_segment([a, b], egui::Stroke::new(1., fantasma));
        }
    }
    // curva, trecho a trecho, pela parede em vigor
    for i in 0..v.pontos.len() {
        let j = (i + 1) % v.pontos.len();
        if let (Some(a), Some(b)) = (tela[i], tela[j]) {
            let p = &v.pontos[i];
            ui.painter().line_segment([a, b], egui::Stroke::new(if p.excede { 4. } else { 3. }, cor_da_parede(p.parede, p.excede, ui.visuals())));
        }
    }
    // ponto sob o mouse
    if let Some(m) = ui.input(|i| i.pointer.hover_pos()).filter(|m| rect.contains(*m)) {
        let perto = tela.iter().enumerate().filter_map(|(i, t)| t.map(|t| (i, t.distance(m)))).filter(|(_, d)| *d < 14.).min_by(|a, b| a.1.total_cmp(&b.1));
        if let Some((i, _)) = perto {
            let p = &v.pontos[i];
            let c = cor_da_parede(p.parede, p.excede, ui.visuals());
            if let Some(t) = tela[i] {
                ui.painter().circle_stroke(t, 7., egui::Stroke::new(2.5, c));
                ui.painter().circle_filled(t, 3., c);
                rotulo(ui, t + egui::vec2(12., -26.), &p.dica);
            }
        }
    }
    if v.editando_folga {
        if let Some(t) = proj(v.no) {
            rotulo(ui, t + egui::vec2(16., 10.), &format!("{} {:.2} mm", tr("folga"), v.folga));
        }
    }
}
