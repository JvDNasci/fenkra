// SPDX-License-Identifier: LGPL-2.1-or-later
//! Janela "Engrenagens", B52: a aba **Eixo** (eixo escalonado, chaveta e resistência do eixo) saiu
//! da aba Projeto, para cada aba caber numa rolagem; e cada parâmetro ganhou dica com esquema.
//!
//! - **Dica com esquema**: passar o mouse no rótulo de um campo mostra o texto e um desenho com a
//!   grandeza realçada. Módulo, dentes, ângulo de pressão, correção e folga usam o esquema do dente
//!   (o mesmo da aba Projeto); largura, furo, hélice e comprimento usam a vista lateral; os campos do
//!   eixo usam o perfil de um eixo de dois trechos.
//! - **Resistência do eixo**: torque, momento fletor, escoamento e segurança exigida dão, por trecho,
//!   a seção líquida, as tensões, a segurança e o diâmetro mínimo
//!   ([`fenkra_solidos::eixos_resistencia`]).

use crate::app::fmt;
use crate::asm_ui::GearsUi;
use crate::i18n::tr;
use eframe::egui::{self, Pos2, Rect, RichText, Stroke, Vec2};
use fenkra_solidos::eixos_resistencia::{verificar_eixo, EntradaEixo};
use fenkra_solidos::{CalculoEngrenagem, ParamEngrenagem, TipoEngrenagem};

/// O que a vista lateral realça.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lateral {
    Largura,
    Furo,
    Helice,
    Comprimento,
}

/// O que o esquema do eixo realça.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CotaEixo {
    Diametro,
    Comprimento,
    Chanfro,
    Rasgo,
    InicioRasgo,
    Torque,
    Momento,
}

/// Desenho da dica de cada rótulo de campo da aba Projeto.
enum Desenho {
    /// esquema do dente com a grandeza do cálculo (rótulo da grade) realçada
    Dente(&'static str),
    Lateral(Lateral),
}

/// Texto e desenho da dica de um campo da aba Projeto (rótulo em português, como na janela).
fn dica_do_campo(rotulo: &str) -> Option<(&'static str, Desenho)> {
    Some(match rotulo {
        "Módulo" => ("Tamanho do dente: o passo no primitivo é π·m. Duas engrenagens só engrenam com o mesmo módulo.", Desenho::Dente("Passo circular")),
        "Dentes do pinhão (z1)" | "Dentes da coroa (z2)" | "Dentes da coroa interna (z2)" => ("Número de dentes: com o módulo dá o diâmetro primitivo d = m·z.", Desenho::Dente("Diâmetro primitivo d")),
        "Ângulo de pressão" => ("Inclinação do flanco no primitivo: define o círculo base db = d·cos α, de onde a evolvente nasce.", Desenho::Dente("Diâmetro base db")),
        "Ângulo de hélice (sinal = sentido)" => ("Inclinação dos dentes em relação ao eixo; o sinal dá o sentido (+ = direita). O par usa ângulos de sinal contrário.", Desenho::Lateral(Lateral::Helice)),
        "Largura" => ("Largura da face do dente, ao longo do eixo.", Desenho::Lateral(Lateral::Largura)),
        "Folga entre dentes (par)" => ("Folga do par, tirada da espessura do dente no primitivo; metade em cada peça.", Desenho::Dente("Espessura do dente")),
        "Furo central" => ("Diâmetro do furo do cubo; 0 = sem furo.", Desenho::Lateral(Lateral::Furo)),
        "Correção do perfil (x)" => ("Desloca a ferramenta de x·m: x positivo engrossa o pé e sobe o topo (da = d + 2m(1 + x)).", Desenho::Dente("Diâmetro de topo da")),
        "Comprimento da cremalheira" => ("Comprimento da barra; os dentes que cabem nele ficam centrados.", Desenho::Lateral(Lateral::Comprimento)),
        _ => return None,
    })
}

/// Conteúdo da dica de um campo: texto e esquema.
pub fn dica_parametro(ui: &mut egui::Ui, rotulo: &str, p: &ParamEngrenagem) {
    let Some((texto, desenho)) = dica_do_campo(rotulo) else { return };
    ui.set_max_width(300.);
    ui.label(tr(texto));
    match desenho {
        Desenho::Dente(grandeza) => match fenkra_solidos::calculo_engrenagem(p) {
            Ok(c) if matches!(p.tipo, TipoEngrenagem::Reta | TipoEngrenagem::Helicoidal) => {
                crate::engrenagens_ui::marcar(ui.ctx(), grandeza);
                crate::engrenagens_ui::esquema(ui, p, &c);
            }
            _ => lateral(ui, Lateral::Largura, p.tipo),
        },
        Desenho::Lateral(l) => lateral(ui, l, p.tipo),
    }
}

/// Rótulo de campo com a dica de esquema.
pub fn rotulo(ui: &mut egui::Ui, texto: &str, p: &ParamEngrenagem) -> egui::Response {
    let r = ui.label(tr(texto));
    if dica_do_campo(texto).is_some() {
        r.on_hover_ui(|ui| dica_parametro(ui, texto, p))
    } else {
        r
    }
}

fn seta(painter: &egui::Painter, a: Pos2, b: Pos2, st: Stroke) {
    painter.line_segment([a, b], st);
    let d = (b - a).normalized() * 6.;
    let n = Vec2::new(-d.y, d.x) * 0.5;
    for (p, s) in [(a, 1.), (b, -1.)] {
        painter.line_segment([p, p + d * s + n], st);
        painter.line_segment([p, p + d * s - n], st);
    }
}

fn cores(ui: &egui::Ui) -> (Stroke, Stroke, Stroke) {
    let v = ui.visuals();
    (Stroke::new(1.4, v.text_color()), Stroke::new(1., v.weak_text_color()), Stroke::new(2.4, crate::theme::accent(v)))
}

/// Vista lateral da engrenagem (corte pelo eixo) com a cota realçada.
pub fn lateral(ui: &mut egui::Ui, realce: Lateral, tipo: TipoEngrenagem) {
    let (resp, painter) = ui.allocate_painter(egui::vec2(260., 120.), egui::Sense::hover());
    painter.rect_filled(resp.rect, 4., ui.visuals().extreme_bg_color);
    let (linha, fraca, acento) = cores(ui);
    let r = resp.rect.shrink2(Vec2::new(70., 18.));
    if tipo == TipoEngrenagem::Cremalheira || realce == Lateral::Comprimento {
        // barra vista de frente: dentes no topo
        let barra = Rect::from_min_max(Pos2::new(resp.rect.left() + 20., r.top() + 18.), Pos2::new(resp.rect.right() - 20., r.bottom()));
        painter.rect_stroke(barra, 0., linha, egui::StrokeKind::Inside);
        let n = 9;
        for k in 0..n {
            let x = barra.left() + barra.width() * (k as f32 + 0.5) / n as f32;
            painter.line_segment([Pos2::new(x - 5., barra.top()), Pos2::new(x - 2., barra.top() - 12.)], linha);
            painter.line_segment([Pos2::new(x + 2., barra.top() - 12.), Pos2::new(x + 5., barra.top())], linha);
            painter.line_segment([Pos2::new(x - 2., barra.top() - 12.), Pos2::new(x + 2., barra.top() - 12.)], linha);
        }
        let y = barra.bottom() + 9.;
        seta(&painter, Pos2::new(barra.left(), y), Pos2::new(barra.right(), y), if realce == Lateral::Comprimento { acento } else { fraca });
        painter.text(Pos2::new(barra.center().x, y + 2.), egui::Align2::CENTER_TOP, "L", egui::FontId::proportional(12.), linha.color);
        return;
    }
    // corte: retângulo da largura b × diâmetro de topo, com o furo tracejado no meio
    painter.rect_stroke(r, 0., linha, egui::StrokeKind::Inside);
    let furo = r.height() * 0.22;
    for y in [r.center().y - furo, r.center().y + furo] {
        painter.add(egui::Shape::dashed_line(&[Pos2::new(r.left(), y), Pos2::new(r.right(), y)], if realce == Lateral::Furo { acento } else { fraca }, 5., 3.));
    }
    painter.add(egui::Shape::dashed_line(&[Pos2::new(r.left() - 12., r.center().y), Pos2::new(r.right() + 12., r.center().y)], fraca, 9., 3.));
    match realce {
        Lateral::Largura => {
            let y = r.top() - 8.;
            seta(&painter, Pos2::new(r.left(), y), Pos2::new(r.right(), y), acento);
            painter.text(Pos2::new(r.center().x, y - 2.), egui::Align2::CENTER_BOTTOM, "b", egui::FontId::proportional(12.), linha.color);
        }
        Lateral::Furo => {
            let x = r.right() + 16.;
            seta(&painter, Pos2::new(x, r.center().y - furo), Pos2::new(x, r.center().y + furo), acento);
            painter.text(Pos2::new(x + 4., r.center().y), egui::Align2::LEFT_CENTER, "Ø", egui::FontId::proportional(12.), linha.color);
        }
        Lateral::Helice => {
            // dentes inclinados vistos de cima, com o ângulo β contra o eixo
            for k in 0..5 {
                let x = r.left() + r.width() * (k as f32 + 0.2) / 5.;
                painter.line_segment([Pos2::new(x, r.bottom()), Pos2::new(x + r.width() * 0.25, r.top())], acento);
            }
            painter.text(Pos2::new(r.right() + 6., r.top()), egui::Align2::LEFT_TOP, "β", egui::FontId::proportional(13.), linha.color);
        }
        Lateral::Comprimento => {}
    }
}

/// Perfil de um eixo de dois trechos, com o rasgo no segundo, e a cota realçada.
pub fn esquema_eixo(ui: &mut egui::Ui, realce: CotaEixo) {
    let (resp, painter) = ui.allocate_painter(egui::vec2(260., 110.), egui::Sense::hover());
    painter.rect_filled(resp.rect, 4., ui.visuals().extreme_bg_color);
    let (linha, fraca, acento) = cores(ui);
    let st = |c: CotaEixo| if realce == c { acento } else { fraca };
    let cy = resp.rect.center().y;
    let x0 = resp.rect.left() + 24.;
    let (t1, t2) = (Rect::from_min_max(Pos2::new(x0, cy - 16.), Pos2::new(x0 + 80., cy + 16.)), Rect::from_min_max(Pos2::new(x0 + 80., cy - 24.), Pos2::new(x0 + 200., cy + 24.)));
    painter.rect_stroke(t1, 0., linha, egui::StrokeKind::Inside);
    painter.rect_stroke(t2, 0., linha, egui::StrokeKind::Inside);
    painter.add(egui::Shape::dashed_line(&[Pos2::new(x0 - 10., cy), Pos2::new(t2.right() + 10., cy)], fraca, 9., 3.));
    // chanfro na ponta direita
    let c = 6.;
    painter.line_segment([Pos2::new(t2.right() - c, t2.top()), Pos2::new(t2.right(), t2.top() + c)], st(CotaEixo::Chanfro));
    // rasgo
    let rasgo = Rect::from_min_max(Pos2::new(t2.left() + 30., t2.top()), Pos2::new(t2.left() + 90., t2.top() + 7.));
    painter.rect_stroke(rasgo, 3., st(CotaEixo::Rasgo), egui::StrokeKind::Inside);
    let fonte = egui::FontId::proportional(11.);
    match realce {
        CotaEixo::Diametro => {
            seta(&painter, Pos2::new(t2.center().x + 30., t2.top()), Pos2::new(t2.center().x + 30., t2.bottom()), acento);
            painter.text(Pos2::new(t2.center().x + 34., cy + 10.), egui::Align2::LEFT_CENTER, "Ø", fonte, linha.color);
        }
        CotaEixo::Comprimento => {
            seta(&painter, Pos2::new(t2.left(), t2.bottom() + 9.), Pos2::new(t2.right(), t2.bottom() + 9.), acento);
            painter.text(Pos2::new(t2.center().x, t2.bottom() + 11.), egui::Align2::CENTER_TOP, "L", fonte, linha.color);
        }
        CotaEixo::InicioRasgo => {
            seta(&painter, Pos2::new(t2.left(), t2.top() - 8.), Pos2::new(rasgo.left(), t2.top() - 8.), acento);
        }
        CotaEixo::Rasgo => {
            seta(&painter, Pos2::new(rasgo.left(), t2.top() - 8.), Pos2::new(rasgo.right(), t2.top() - 8.), acento);
        }
        CotaEixo::Torque => {
            // seta curva em volta do eixo, na ponta esquerda
            let pts: Vec<Pos2> = (0..=16).map(|i| {
                let a = std::f32::consts::PI * (0.2 + 1.4 * i as f32 / 16.);
                Pos2::new(x0 - 4. + 7. * a.cos(), cy + 26. * a.sin())
            }).collect();
            painter.add(egui::Shape::line(pts, acento));
            painter.text(Pos2::new(x0 + 6., t1.bottom() + 4.), egui::Align2::LEFT_TOP, "T", fonte, linha.color);
        }
        CotaEixo::Momento => {
            // carga no meio do trecho maior e apoios nas pontas: o eixo flete
            seta(&painter, Pos2::new(t2.center().x, t2.top() - 22.), Pos2::new(t2.center().x, t2.top() - 2.), acento);
            for x in [t1.left() + 6., t2.right() - 6.] {
                painter.line_segment([Pos2::new(x - 6., t2.bottom() + 10.), Pos2::new(x + 6., t2.bottom() + 10.)], acento);
            }
            painter.text(Pos2::new(t2.center().x + 6., t2.top() - 14.), egui::Align2::LEFT_CENTER, "M", fonte, linha.color);
        }
        CotaEixo::Chanfro => {}
    }
}

fn dica_eixo(cab: &str) -> Option<(&'static str, CotaEixo)> {
    Some(match cab {
        "Ø" => ("Diâmetro do trecho.", CotaEixo::Diametro),
        "L" => ("Comprimento do trecho, ao longo do eixo.", CotaEixo::Comprimento),
        "Chanfro" => ("Chanfro de 45° na aresta do fim do trecho, quando ela é externa (ponta ou ombro para um trecho menor).", CotaEixo::Chanfro),
        "Rasgo" | "L rasgo" => ("Rasgo de chaveta DIN 6885 forma A: comprimento total, com as pontas arredondadas.", CotaEixo::Rasgo),
        "Início" => ("Distância do começo do trecho até o começo do rasgo.", CotaEixo::InicioRasgo),
        "Torque" => ("Torque que o eixo transmite (por exemplo, o do pinhão na aba Verificar).", CotaEixo::Torque),
        "Momento fletor" => ("Maior momento fletor do eixo (força × distância ao apoio). A conta usa o mesmo valor em todos os trechos.", CotaEixo::Momento),
        "Escoamento" => ("Limite de escoamento do material do eixo; aço 1045 laminado fica perto de 310 MPa.", CotaEixo::Diametro),
        "Segurança exigida" => ("Segurança mínima; o trecho abaixo dela fica marcado.", CotaEixo::Diametro),
        _ => return None,
    })
}

fn rotulo_eixo(ui: &mut egui::Ui, cab: &str, forte: bool) {
    let t = if forte { RichText::new(tr(cab)).strong() } else { RichText::new(tr(cab)) };
    let r = ui.label(t);
    if let Some((texto, cota)) = dica_eixo(cab) {
        r.on_hover_ui(|ui| {
            ui.set_max_width(280.);
            ui.label(tr(texto));
            esquema_eixo(ui, cota);
        });
    }
}

/// Aba Eixo: trechos, recursos de seção, chaveta e resistência. Devolve (salvar eixo, salvar chaveta).
pub fn aba_eixo(ui: &mut egui::Ui, g: &mut GearsUi) -> (bool, Option<(f64, f64)>) {
    let (mut salvar_eixo, mut salvar_chaveta) = (false, None);
    ui.label(RichText::new(tr("Eixo escalonado ao longo de Z (a partir de z = 0), com chanfros e rasgos de chaveta DIN 6885 forma A cortados em +X. A chaveta vem da tabela pelo diâmetro de cada trecho.")).weak().small());
    ui.horizontal(|ui| {
        ui.selectable_value(&mut g.eixo_grafico, false, tr("Tabela"));
        ui.selectable_value(&mut g.eixo_grafico, true, tr("Gráfico")).on_hover_text(tr("Mostra o eixo no 3D com alças nas arestas dos trechos: arraste uma alça para mudar L ou Ø e a grade acompanha."));
    });
    crate::eixo_ui::perfil_lateral(ui, g);
    let mut remover: Option<usize> = None;
    egui::Grid::new("grade_eixo").num_columns(8).spacing([8., 3.]).show(ui, |ui| {
        for h in ["Ø", "L", "Chanfro", "Rasgo", "L rasgo", "Início", "Chaveta", ""] {
            rotulo_eixo(ui, h, true);
        }
        ui.end_row();
        for (i, t) in g.eixo.trechos.iter_mut().enumerate() {
            for (v, r, vel) in [(&mut t.diametro, 1.0..=1000., 0.5), (&mut t.comprimento, 0.1..=10000., 1.), (&mut t.chanfro_fim, 0.0..=100., 0.1)] {
                let resp = ui.add(crate::ui_ext::dv(v).range(r).speed(vel).max_decimals(3));
                crate::ui_ext::wheel(ui, &resp, v);
            }
            let mut tem = t.rasgo.is_some();
            if ui.checkbox(&mut tem, "").changed() {
                t.rasgo = tem.then(|| fenkra_solidos::RasgoChaveta { comprimento: t.comprimento.min(20.), desloc_inicio: 0. });
            }
            match t.rasgo.as_mut() {
                Some(r) => {
                    let resp = ui.add(crate::ui_ext::dv(&mut r.comprimento).range(0.1..=10000.).speed(1.).max_decimals(3));
                    crate::ui_ext::wheel(ui, &resp, &mut r.comprimento);
                    let resp = ui.add(crate::ui_ext::dv(&mut r.desloc_inicio).range(0.0..=10000.).speed(1.).max_decimals(3));
                    crate::ui_ext::wheel(ui, &resp, &mut r.desloc_inicio);
                }
                None => {
                    ui.label("");
                    ui.label("");
                }
            }
            match fenkra_solidos::biblioteca::chaveta_para_eixo(t.diametro) {
                Some(c) => ui.label(format!("{}×{} · t1 {} · t2 {}", fmt(c.b), fmt(c.h), fmt(c.t1), fmt(c.t2))),
                None => ui.label(RichText::new(tr("fora da tabela")).weak()),
            };
            if ui.small_button("✖").on_hover_text(tr("Remover trecho")).clicked() {
                remover = Some(i);
            }
            ui.end_row();
        }
    });
    if let Some(i) = remover {
        if g.eixo.trechos.len() > 1 {
            g.eixo.trechos.remove(i);
            crate::eixo_ui::remover_trecho(&mut g.recursos, i);
            g.trecho_sel = Some(i.saturating_sub(1).min(g.eixo.trechos.len() - 1));
        }
    }
    ui.horizontal_wrapped(|ui| {
        if ui.button(tr("+ Trecho")).clicked() {
            let mut novo = g.eixo.trechos.last().copied().unwrap_or(fenkra_solidos::TrechoEixo { diametro: 20., comprimento: 30., chanfro_fim: 0., rasgo: None });
            novo.rasgo = None;
            g.eixo.trechos.push(novo);
        }
        ui.checkbox(&mut g.eixo.furo_centro, tr("Furos de centro nas pontas"));
    });
    ui.horizontal_wrapped(|ui| {
        for k in 0..g.eixo.trechos.len() {
            ui.selectable_value(&mut g.trecho_sel, Some(k), format!("{} {}", tr("Trecho"), k + 1));
        }
    });
    crate::eixo_ui::recursos_do_trecho(ui, g);
    let eixo_ok = g.eixo.validar().and_then(|c| fenkra_solidos::eixos_recursos::validar_recursos(&g.eixo, &g.recursos).map(|_| c));
    if let Err(e) = &eixo_ok {
        ui.label(RichText::new(format!("⚠ {e}")).color(ui.visuals().error_fg_color));
    }
    resistencia(ui, g);
    // a chaveta é a do primeiro trecho com rasgo, no maior comprimento da série que cabe no rasgo
    let chaveta = g.eixo.trechos.iter().find_map(|t| t.rasgo.map(|r| (t.diametro, fenkra_solidos::eixos::comprimento_chaveta_ate(r.comprimento))));
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(format!("{}: {} mm", tr("Comprimento total"), fmt(g.eixo.comprimento_total()))).small());
        if ui.add_enabled(eixo_ok.is_ok(), egui::Button::new(tr("Salvar eixo..."))).on_hover_text(tr("Salva o eixo (.fkp) com os trechos, chanfros e rasgos acima.")).clicked() {
            salvar_eixo = true;
        }
        let b = ui.add_enabled(eixo_ok.is_ok() && matches!(chaveta, Some((_, Some(_)))), egui::Button::new(tr("Salvar chaveta...")));
        if b.on_hover_text(tr("Salva a chaveta DIN 6885 A do primeiro trecho com rasgo, no maior comprimento da série que cabe no rasgo.")).clicked() {
            if let Some((d, Some(l))) = chaveta {
                salvar_chaveta = Some((d, l));
            }
        }
    });
    (salvar_eixo, salvar_chaveta)
}

/// Seção "Resistência do eixo": entradas e tabela por trecho.
fn resistencia(ui: &mut egui::Ui, g: &mut GearsUi) {
    crate::editor::section(ui, "Resistência do eixo");
    let e: &mut EntradaEixo = &mut g.verificar.eixo;
    egui::Grid::new("entradas_eixo").num_columns(2).spacing([10., 3.]).show(ui, |ui| {
        for (cab, v, suf, vel) in [("Torque", &mut e.torque_nm, " N·m", 0.5), ("Momento fletor", &mut e.momento_nm, " N·m", 0.5), ("Escoamento", &mut e.escoamento_mpa, " MPa", 5.), ("Segurança exigida", &mut e.fs_exigido, "", 0.05)] {
            rotulo_eixo(ui, cab, false);
            let resp = ui.add(crate::ui_ext::dv(v).range(0.0..=1e6).speed(vel).max_decimals(2).suffix(suf));
            crate::ui_ext::wheel(ui, &resp, v);
            ui.end_row();
        }
    });
    match verificar_eixo(&g.eixo, &g.recursos, e) {
        Err(msg) => {
            ui.label(RichText::new(format!("⚠ {msg}")).color(ui.visuals().error_fg_color));
        }
        Ok(r) => {
            egui::Grid::new("resultado_eixo").num_columns(6).spacing([10., 2.]).striped(true).show(ui, |ui| {
                for h in ["Trecho", "Ø líquido", "σ flexão", "τ torção", "σ equivalente", "Segurança"] {
                    ui.label(RichText::new(tr(h)).strong().small());
                }
                ui.end_row();
                for t in &r.trechos {
                    ui.label(format!("{}", t.trecho + 1));
                    let d = ui.label(fmt((t.diametro_liquido * 100.).round() / 100.));
                    if !t.motivo.is_empty() {
                        d.on_hover_text(tr(&t.motivo));
                    }
                    for x in [t.flexao_mpa, t.torcao_mpa, t.equivalente_mpa] {
                        ui.label(fmt((x * 10.).round() / 10.));
                    }
                    let fs = if t.seguranca.is_finite() { fmt((t.seguranca * 100.).round() / 100.) } else { "∞".into() };
                    if t.ok {
                        ui.label(fs);
                    } else {
                        ui.label(RichText::new(format!("⚠ {fs}")).color(ui.visuals().error_fg_color));
                    }
                    ui.end_row();
                }
            });
            ui.label(RichText::new(format!("{}: Ø {} mm · {}", tr("Diâmetro líquido mínimo"), fmt((r.diametro_minimo * 100.).round() / 100.), tr("verificação estática (von Mises), sem concentração de tensão nem fadiga"))).small());
        }
    }
}

/// Resultado do cálculo só para a dica não refazer a conta (exposto para os testes).
pub fn calculo_da_dica(p: &ParamEngrenagem) -> Option<CalculoEngrenagem> {
    fenkra_solidos::calculo_engrenagem(p).ok()
}

pub static TEXTOS: &[(&str, &str)] = &[
    ("Eixo", "Shaft"),
    ("Eixo escalonado, recursos de seção, chaveta e resistência do eixo.", "Stepped shaft, section features, key and shaft strength."),
    ("Tamanho do dente: o passo no primitivo é π·m. Duas engrenagens só engrenam com o mesmo módulo.", "Tooth size: the pitch on the pitch circle is π·m. Two gears only mesh with the same module."),
    ("Número de dentes: com o módulo dá o diâmetro primitivo d = m·z.", "Number of teeth: with the module it gives the pitch diameter d = m·z."),
    ("Inclinação do flanco no primitivo: define o círculo base db = d·cos α, de onde a evolvente nasce.", "Flank slope at the pitch circle: defines the base circle db = d·cos α, where the involute starts."),
    ("Inclinação dos dentes em relação ao eixo; o sinal dá o sentido (+ = direita). O par usa ângulos de sinal contrário.", "Tooth slope relative to the axis; the sign gives the hand (+ = right). The mating gear uses the opposite sign."),
    ("Largura da face do dente, ao longo do eixo.", "Tooth face width, along the axis."),
    ("Folga do par, tirada da espessura do dente no primitivo; metade em cada peça.", "Backlash of the pair, taken from the tooth thickness at the pitch circle; half on each part."),
    ("Diâmetro do furo do cubo; 0 = sem furo.", "Hub bore diameter; 0 = no bore."),
    ("Desloca a ferramenta de x·m: x positivo engrossa o pé e sobe o topo (da = d + 2m(1 + x)).", "Shifts the cutter by x·m: positive x thickens the root and raises the tip (da = d + 2m(1 + x))."),
    ("Comprimento da barra; os dentes que cabem nele ficam centrados.", "Bar length; the teeth that fit are centred on it."),
    ("Diâmetro do trecho.", "Step diameter."),
    ("Comprimento do trecho, ao longo do eixo.", "Step length, along the axis."),
    ("Chanfro de 45° na aresta do fim do trecho, quando ela é externa (ponta ou ombro para um trecho menor).", "45° chamfer on the end edge of the step, when it is external (shaft end or shoulder to a smaller step)."),
    ("Rasgo de chaveta DIN 6885 forma A: comprimento total, com as pontas arredondadas.", "DIN 6885 form A keyway: total length, with rounded ends."),
    ("Distância do começo do trecho até o começo do rasgo.", "Distance from the start of the step to the start of the keyway."),
    ("Torque", "Torque"),
    ("Momento fletor", "Bending moment"),
    ("Escoamento", "Yield strength"),
    ("Torque que o eixo transmite (por exemplo, o do pinhão na aba Verificar).", "Torque transmitted by the shaft (for example, the pinion torque in the Check tab)."),
    ("Maior momento fletor do eixo (força × distância ao apoio). A conta usa o mesmo valor em todos os trechos.", "Largest bending moment on the shaft (force × distance to the support). The check uses the same value on every step."),
    ("Limite de escoamento do material do eixo; aço 1045 laminado fica perto de 310 MPa.", "Yield strength of the shaft material; hot-rolled 1045 steel is close to 310 MPa."),
    ("Segurança mínima; o trecho abaixo dela fica marcado.", "Minimum safety factor; a step below it is flagged."),
    ("Resistência do eixo", "Shaft strength"),
    ("Ø líquido", "Net Ø"),
    ("σ flexão", "σ bending"),
    ("τ torção", "τ torsion"),
    ("σ equivalente", "σ equivalent"),
    ("Segurança", "Safety factor"),
    ("Diâmetro líquido mínimo", "Minimum net diameter"),
    ("verificação estática (von Mises), sem concentração de tensão nem fadiga", "static check (von Mises), without stress concentration or fatigue"),
    ("rasgo de chaveta", "keyway"),
    ("anel elástico", "retaining ring"),
    ("ranhura", "groove"),
    ("rosca", "thread"),
    ("furo transversal", "cross hole"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn textos_tem_traducao_e_nao_repetem() {
        let mut vistos = std::collections::HashSet::new();
        for (pt, en) in TEXTOS {
            assert!(!en.is_empty() && vistos.insert(*pt), "{pt}");
        }
    }

    /// Todo campo da aba Projeto e toda coluna do eixo têm dica traduzida com esquema.
    #[test]
    fn cada_parametro_tem_dica_com_esquema() {
        let campos = ["Módulo", "Dentes do pinhão (z1)", "Dentes da coroa (z2)", "Dentes da coroa interna (z2)", "Ângulo de pressão", "Ângulo de hélice (sinal = sentido)", "Largura", "Folga entre dentes (par)", "Furo central", "Correção do perfil (x)", "Comprimento da cremalheira"];
        for c in campos {
            let (t, _) = dica_do_campo(c).unwrap_or_else(|| panic!("{c} sem dica"));
            assert!(TEXTOS.iter().any(|(pt, _)| *pt == t), "{t}");
        }
        for c in ["Ø", "L", "Chanfro", "Rasgo", "L rasgo", "Início", "Torque", "Momento fletor", "Escoamento", "Segurança exigida"] {
            let (t, _) = dica_eixo(c).unwrap_or_else(|| panic!("{c} sem dica"));
            assert!(TEXTOS.iter().any(|(pt, _)| *pt == t), "{t}");
        }
        // as grandezas do esquema do dente existem na grade do cálculo
        for c in campos {
            if let Some((_, Desenho::Dente(g))) = dica_do_campo(c) {
                assert!(crate::engrenagens_ui::Grandeza::do_rotulo(g).is_some(), "{g}");
            }
        }
        assert!(calculo_da_dica(&ParamEngrenagem::default()).is_some());
    }

    /// A dica, a vista lateral, o esquema do eixo e a aba Eixo desenham sem pânico.
    #[test]
    fn desenhos_e_aba_eixo_rodam() {
        let ctx = egui::Context::default();
        let mut g = GearsUi::default();
        let mut saida = ctx.run_ui(Default::default(), |ui| {
            for c in ["Módulo", "Largura", "Furo central", "Ângulo de hélice (sinal = sentido)", "Comprimento da cremalheira"] {
                dica_parametro(ui, c, &ParamEngrenagem::default());
            }
            for c in [CotaEixo::Diametro, CotaEixo::Comprimento, CotaEixo::Chanfro, CotaEixo::Rasgo, CotaEixo::InicioRasgo, CotaEixo::Torque, CotaEixo::Momento] {
                esquema_eixo(ui, c);
            }
            let (s, k) = aba_eixo(ui, &mut g);
            assert!(!s && k.is_none());
        });
        saida.textures_delta.clear();
    }
}
