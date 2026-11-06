// SPDX-License-Identifier: LGPL-2.1-or-later
//! Estrutural: dobra de tubo no nó (B326, B327, B330). O editor do nó (modo, raio, regra, ajustes e
//! o resultado do cálculo), o resumo do tubo dobrado na ficha do membro e a janela das regras de
//! dobra cadastráveis.
//!
//! Alongamento e afinamento saem como estimativa: o rótulo e a dica dizem que não são valor de
//! engenharia. As regras são do usuário; o programa não traz tabela de material nem de norma.

use crate::app::{fmt, App};
use crate::i18n::tr;
use crate::theme;
use eframe::egui::{self, RichText};
use fenkra_core::*;
use fenkra_solidos::st_dobra::{nome_modo, ResultadoDobra, TuboDobrado, AVISO_ESTIMATIVA};
use std::collections::HashMap;

/// Tubos dobrados do último cálculo, para o painel do membro (que não enxerga o documento calculado).
#[derive(Clone, Default)]
pub struct TubosCache(pub Vec<TuboDobrado>);

pub fn guardar(ctx: &egui::Context, doc: &Document) {
    // desenvolvimento: no exemplo FENKRA_DEMO_DOBRA o quadro repinta sempre, para a altura do painel assentar na captura
    if std::env::var_os("FENKRA_DEMO_DOBRA").is_some() {
        ctx.request_repaint();
    }
    ctx.data_mut(|d| d.insert_temp(egui::Id::new("st_tubos_dobrados"), TubosCache(fenkra_solidos::tubos_dobrados(doc))));
}

fn tubos(ui: &egui::Ui) -> Vec<TuboDobrado> {
    ui.ctx().data(|d| d.get_temp::<TubosCache>(egui::Id::new("st_tubos_dobrados"))).unwrap_or_default().0
}

const TIPOS_TUBO: [SectionKind; 3] = [SectionKind::TuboQuadrado, SectionKind::TuboRetangular, SectionKind::TuboRedondo];

/// O membro é tubo (a dobra só vale em seção fechada)?
pub fn e_tubo(k: SectionKind) -> bool {
    TIPOS_TUBO.contains(&k)
}

/// Sugestão inicial de raio ao escolher a dobra: 2,5 vezes a maior medida da seção. Só garante uma
/// curva que não se cruza; não é regra de projeto (quem manda é a regra cadastrada ou o usuário).
pub fn raio_inicial(m: &StructMember, vars: &HashMap<String, f64>) -> String {
    let n = if m.kind == SectionKind::TuboRetangular { 2 } else { 1 };
    let maior = m.dims.iter().take(n).filter_map(|e| fenkra_core::expr::eval_str(e, vars).ok()).fold(0., f64::max);
    fmt(if maior > 0. { (2.5 * maior * 10.).round() / 10. } else { 100. })
}

fn modo_dica(m: BendMode) -> &'static str {
    match m {
        BendMode::Dobradeira => "Dobra por dobradeira ou rolos: o tubo vira um sólido curvo, de uma peça só.",
        BendMode::VComum => "V comum: canto vivo, rasgo em V com a parede externa de dobradiça. O 3D mostra o tubo fechado; os rasgos estão no tubo aberto.",
        BendMode::VComRaio => "V com raio: canto curvo pelo gabarito de serralheria. O 3D mostra o tubo fechado; o contorno do corte está no tubo aberto.",
        BendMode::CorteDobradeira => "Corte em V fechado por ferramenta, com o retorno elástico compensado. O 3D mostra o tubo fechado; os rasgos estão no tubo aberto.",
        BendMode::CorteSimples => "Corte simples: esquadria nas duas pontas (gomos no tubo redondo) e solda. Gera dois membros.",
    }
}

fn rotulo_do_raio(m: BendMode) -> &'static str {
    match m {
        BendMode::Dobradeira => "Raio de centro",
        BendMode::VComum => "Raio interno da dobradiça",
        BendMode::VComRaio => "Raio externo do canto",
        BendMode::CorteDobradeira => "Raio interno da dobradiça",
        BendMode::CorteSimples => "Raio de referência",
    }
}

fn dica_do_raio(m: BendMode) -> &'static str {
    match m {
        BendMode::Dobradeira => "Distância do centro da curva ao eixo do tubo. Tem de passar da metade da dimensão da seção no plano da dobra; as pernas recuam R·tan(ângulo/2) no nó.",
        BendMode::VComum | BendMode::CorteDobradeira => "Raio interno da dobradiça (parede externa do V). Com mais de um rasgo, é o raio da linha de articulações.",
        BendMode::VComRaio => "Raio da face externa da placa curvada. Perto da altura do tubo; fora de 1 a 4,6 vezes a altura o modo avisa.",
        BendMode::CorteSimples => "Só entra nos gomos do tubo redondo (raio da curva que os gomos aproximam). Esquadria única não usa.",
    }
}

/// Campo de texto com expressão. Devolve verdadeiro quando o usuário termina de editar (saiu do campo).
fn campo(ui: &mut egui::Ui, rotulo: &str, dica: &str, s: &mut String, vars: &HashMap<String, f64>, unidade: &str, vazio: &str) -> bool {
    let mut mudou = false;
    ui.horizontal(|ui| {
        ui.add_sized([150., 18.], egui::Label::new(tr(rotulo)).truncate()).on_hover_text(tr(dica));
        let valor = fenkra_core::expr::eval_str(s, vars);
        let ruim = !s.trim().is_empty() && valor.is_err();
        let mut te = egui::TextEdit::singleline(s).desired_width(90.).hint_text(tr(vazio));
        if ruim {
            te = te.text_color(theme::st_err());
        }
        let r = ui.add(te).on_hover_text(tr(dica));
        mudou |= r.lost_focus();
        if !unidade.is_empty() {
            ui.label(RichText::new(unidade).weak());
        }
        if let (false, Ok(v)) = (s.trim().is_empty(), valor) {
            if s.trim().parse::<f64>().is_err() {
                ui.label(RichText::new(format!("= {}", fmt(v))).weak());
            }
        }
    });
    mudou
}

/// Editor do nó de dobra entre as linhas `a` e `b` do comando `me`.
pub fn editor_do_no(ui: &mut egui::Ui, doc: &Document, me: Id, m: &mut StructMember, joint: usize, vars: &HashMap<String, f64>) {
    let (a, b) = (m.joints[joint].mine, m.joints[joint].other_path);
    let kind = m.kind;
    let material = if m.material.trim().is_empty() { doc.props.material.clone() } else { m.material.clone() };
    let nomes_regras: Vec<String> = doc.tube_rules.iter().filter(|r| fenkra_solidos::st_dobra::regra_serve(r, kind, &material)).map(|r| r.name.clone()).collect();
    let todos = tubos(ui);
    let tubo = todos.iter().filter(|t| t.feature == me).find(|t| t.dobras.iter().any(|d| d.linhas == (a, b) || d.linhas == (b, a)));
    // só a primeira dobra de cada tubo escolhe o lado: as seguintes seguem a seção que chega
    let primeira = tubo.is_none_or(|t| t.dobras.first().is_some_and(|d| d.linhas == (a, b) || d.linhas == (b, a)));
    let calc = tubo.and_then(|t| t.dobras.iter().find(|d| d.linhas == (a, b) || d.linhas == (b, a)).cloned());
    let Some(bend) = m.joints[joint].bend.as_mut() else { return };
    ui.add_space(4.);
    ui.label(RichText::new(format!("{} {} ↔ {}", tr("Dobra de tubo · linhas"), a.min(b) + 1, a.max(b) + 1)).strong());
    if let Some(c) = &calc {
        ui.label(RichText::new(format!("{} {}°", tr("Ângulo da dobra:"), fmt(c.resultado.theta))).weak());
    }
    // modo
    ui.horizontal(|ui| {
        ui.add_sized([150., 18.], egui::Label::new(tr("Modo"))).on_hover_text(tr("Como o tubo faz a curva neste nó."));
        egui::ComboBox::from_id_salt(("st_dobra_modo", joint)).selected_text(tr(nome_modo(bend.mode))).width(170.).show_ui(ui, |ui| {
            for mo in BendMode::ALL {
                let redondo_sem_v = kind == SectionKind::TuboRedondo && matches!(mo, BendMode::VComum | BendMode::VComRaio | BendMode::CorteDobradeira);
                let r = ui.add_enabled(!redondo_sem_v, egui::Button::selectable(bend.mode == mo, tr(nome_modo(mo)))).on_hover_text(tr(modo_dica(mo)));
                if r.clicked() {
                    bend.mode = mo;
                }
            }
        });
    })
    .response
    .on_hover_text(tr(modo_dica(bend.mode)));
    campo(ui, rotulo_do_raio(bend.mode), dica_do_raio(bend.mode), &mut bend.radius, vars, "mm", "obrigatório");
    if kind == SectionKind::TuboRetangular && !primeira {
        ui.label(RichText::new(tr("A seção segue a da dobra anterior; o lado no plano vem da primeira dobra do tubo.")).weak().small());
    } else if kind == SectionKind::TuboRetangular {
        ui.horizontal(|ui| {
            ui.add_sized([150., 18.], egui::Label::new(tr("Dimensão no plano"))).on_hover_text(tr("Qual lado do tubo retangular fica no plano da dobra. A seção das pernas é girada para isso; a rotação do membro não vale nas pernas dobradas."));
            ui.selectable_value(&mut bend.plane_dim, 0, tr("largura")).on_hover_text(tr("A largura (primeira cota) fica no plano da dobra."));
            ui.selectable_value(&mut bend.plane_dim, 1, tr("altura")).on_hover_text(tr("A altura (segunda cota) fica no plano da dobra."));
        });
    }
    // regra
    ui.horizontal(|ui| {
        ui.add_sized([150., 18.], egui::Label::new(tr("Regra de dobra"))).on_hover_text(tr("Tabela cadastrada com raio mínimo, retorno elástico, linha neutra e alongamento admissível. Os valores são seus; sem regra, nada vem de tabela."));
        let atual = if bend.rule.is_empty() { tr("(nenhuma)").to_string() } else { bend.rule.clone() };
        egui::ComboBox::from_id_salt(("st_dobra_regra", joint)).selected_text(atual).width(170.).show_ui(ui, |ui| {
            if ui.selectable_label(bend.rule.is_empty(), tr("(nenhuma)")).clicked() {
                bend.rule.clear();
            }
            for n in &nomes_regras {
                if ui.selectable_label(bend.rule == *n, n).clicked() {
                    bend.rule = n.clone();
                }
            }
            // a regra citada que não serve mais continua visível, para o usuário ver o erro
            if !bend.rule.is_empty() && !nomes_regras.contains(&bend.rule) {
                ui.label(RichText::new(format!("{} {}", bend.rule, tr("(não vale aqui)"))).color(theme::st_err()));
            }
        });
        if ui.small_button("✎").on_hover_text(tr("Abrir o cadastro de regras de dobra")).clicked() {
            ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("st_regras_aberta"), true));
        }
    });
    egui::CollapsingHeader::new(tr("Ajustes do nó")).id_salt(("st_dobra_ajustes", joint)).default_open(false).show(ui, |ui| {
        if matches!(bend.mode, BendMode::VComum | BendMode::VComRaio | BendMode::CorteDobradeira) {
            campo(ui, "K da placa", "Fator K da placa externa nos modos em V. Vazio usa o da regra; sem regra, a linha neutra de volume constante (sem fonte).", &mut bend.k, vars, "", "da regra");
        }
        campo(ui, "Retorno elástico", "Quanto a máquina dobra além do ângulo pedido, em graus. Vazio usa o da regra; sem regra, zero.", &mut bend.springback, vars, "°", "da regra");
        if matches!(bend.mode, BendMode::VComum | BendMode::CorteDobradeira) {
            campo(ui, "Rasgos em V", "Número de rasgos. 1 é o V comum; mais de um é o V múltiplo, em que o raio vira o da linha de articulações.", &mut bend.cuts, vars, "", "1");
        }
        if bend.mode == BendMode::VComRaio {
            campo(ui, "Folga de raiz", "Folga de raiz da solda do canto com gabarito. Vazio usa a da regra.", &mut bend.root_gap, vars, "mm", "da regra");
        }
        if bend.mode == BendMode::CorteSimples && kind == SectionKind::TuboRedondo {
            campo(ui, "Gomos", "Número de juntas do tubo redondo. 1 é uma esquadria; mais juntas aproximam a curva (só entram na lista de corte).", &mut bend.segments, vars, "", "1");
        }
    });
    match &calc {
        Some(c) => {
            resultado_ui(ui, &c.resultado, c.inicio, c.fim);
            if let Some(t) = tubo {
                crate::st_dobra_corte_ui::painel(ui, t, c, joint);
            }
        }
        None => {
            ui.label(RichText::new(tr("Sem cálculo ainda: preencha o raio.")).weak());
        }
    }
}

/// O resultado de uma dobra: números, estimativas marcadas, avisos e de onde vem cada valor.
pub fn resultado_ui(ui: &mut egui::Ui, r: &ResultadoDobra, inicio: f64, fim: f64) {
    let linha = |ui: &mut egui::Ui, nome: &str, valor: String, dica: &str| {
        ui.horizontal(|ui| {
            ui.add_sized([150., 16.], egui::Label::new(RichText::new(tr(nome)).weak())).on_hover_text(tr(dica));
            ui.label(valor);
        });
    };
    for e in &r.erros {
        ui.label(RichText::new(format!("⛔ {e}")).color(theme::st_err()));
    }
    if !r.erros.is_empty() {
        return;
    }
    linha(ui, "Comprimento do arco", format!("{} mm", fmt(r.arco)), "Comprimento da curva ao longo da linha neutra (dobradeira) ou da placa dobrada (modos em V).");
    linha(ui, "Dedução de dobra", format!("{} mm", fmt(r.deducao)), "O quanto o corte encurta em relação às duas pernas medidas até o nó. Comprimento de corte = soma das pernas + ajuste das dobras.");
    linha(ui, "Início da dobra no tubo reto", format!("{} → {} mm", fmt(inicio), fmt(fim)), "Onde a dobra começa e termina no tubo desenvolvido, contado da ponta da primeira perna.");
    if r.tangente > 0. {
        linha(ui, "Recuo no nó", format!("{} mm", fmt(r.tangente)), "Distância do cruzamento das linhas guia ao início da curva, em cada perna.");
    }
    if r.angulo_maquina != r.theta {
        linha(ui, "Ângulo da máquina", format!("{}° ({:+}°)", fmt(r.angulo_maquina), r.retorno.v), "Ângulo de dobra pedido mais o retorno elástico: a peça volta ao ângulo do desenho.");
    }
    if let Some(v) = r.placa {
        linha(ui, "Placa curvada", format!("{} mm", fmt(v)), "Comprimento plano da parede externa curvada no canto com gabarito.");
    }
    if let Some(v) = r.folga_plana {
        linha(ui, "Folga plana do corte", format!("{} mm", fmt(v)), "Abertura no alto do corte das laterais com a peça ainda plana; fecha a zero ao dobrar.");
    }
    if let (Some(w), Some(l)) = (r.abertura_v, r.lado_v) {
        linha(ui, "Abertura / lado do V", format!("{} / {} mm", fmt(w), fmt(l)), "Abertura do V na face e comprimento do seu lado.");
    }
    if let Some(p) = r.passo_v {
        linha(ui, "Passo entre rasgos", format!("{} mm", fmt(p)), "Distância entre os centros de dois V seguidos (V múltiplo).");
    }
    if let Some(s) = r.solda {
        linha(ui, "Solda da junta", format!("{} mm", fmt(s)), "Comprimento total de solda estimado nesta junta.");
    }
    if let Some(d) = r.dif_ext_int {
        linha(ui, "Externo menos interno", format!("{} mm", fmt(d)), "Diferença entre os comprimentos externo e interno de cada perna no corte em esquadria.");
    }
    if let Some((meio, ponta)) = r.gomos {
        linha(ui, "Gomos (meio / ponta)", format!("{} / {} mm", fmt(meio), fmt(ponta)), "Comprimento das peças intermediárias e das peças das pontas dos gomos.");
    }
    // estimativas: sempre marcadas
    if r.alongamento.is_some() || r.afinamento_ext.is_some() {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(tr("≈ estimativa")).color(theme::st_warn())).on_hover_text(tr(AVISO_ESTIMATIVA));
            let pc = |v: f64| format!("{:+.0}%", v * 100.);
            if let Some(v) = r.alongamento {
                ui.label(format!("{} {:.0}%", tr("fibra externa estica"), v * 100.));
            }
            if let (Some(e), Some(i)) = (r.afinamento_ext, r.espessamento_int) {
                ui.label(format!("· {} {} · {} {}", tr("parede externa"), pc(e), tr("interna"), pc(i)));
            }
        })
        .response
        .on_hover_text(tr(AVISO_ESTIMATIVA));
        ui.label(RichText::new(tr("Alongamento e afinamento são estimativas de um modelo simples, sem fonte confirmada: não são valores de engenharia.")).weak().small());
    }
    for a in &r.avisos {
        ui.label(RichText::new(format!("⚠ {a}")).color(theme::st_warn()));
    }
    if !r.origens.is_empty() {
        egui::CollapsingHeader::new(tr("De onde vêm os valores")).id_salt(("st_dobra_origens", r.arco.to_bits())).show(ui, |ui| {
            for (campo, origem) in &r.origens {
                ui.label(RichText::new(format!("{}: {}", tr(campo), tr(origem))).weak().small());
            }
        });
    }
}

/// Resumo do tubo dobrado na ficha do membro: desenvolvido, pernas e cada dobra.
pub fn ficha_do_tubo(ui: &mut egui::Ui, me: Id) {
    let mine: Vec<TuboDobrado> = tubos(ui).into_iter().filter(|t| t.feature == me).collect();
    if mine.is_empty() {
        return;
    }
    crate::editor::section(ui, "Tubo dobrado");
    for (n, t) in mine.iter().enumerate() {
        ui.label(RichText::new(format!("{} {} · {} {} mm", tr("Tubo"), n + 1, tr("desenvolvido"), fmt(t.desenvolvido))).strong());
        ui.label(RichText::new(format!("{} {} mm · {} {}", tr("pernas"), t.pernas.iter().map(|p| fmt(*p)).collect::<Vec<_>>().join(" + "), tr("dobras"), t.resumo())).weak().small());
        if t.so_corte_simples() {
            ui.label(RichText::new(tr("Corte simples: as pernas são peças separadas; o desenvolvido não se aplica.")).weak().small());
        }
        for e in &t.erros {
            ui.label(RichText::new(format!("⛔ {e}")).color(theme::st_err()));
        }
    }
}

// ------------------------------------------------------------------------- regras cadastráveis

#[derive(Clone, Default)]
struct Rascunho {
    base: Vec<TubeRule>,
    texto: Vec<TubeRule>,
    escolhida: usize,
}

fn secao_nome(s: Option<SectionKind>) -> &'static str {
    match s {
        None => "Qualquer",
        Some(SectionKind::TuboQuadrado) => "Tubo quadrado",
        Some(SectionKind::TuboRetangular) => "Tubo retangular",
        Some(SectionKind::TuboRedondo) => "Tubo redondo",
        Some(_) => "Qualquer",
    }
}

/// Janela do cadastro de regras de dobra de tubo (B330).
pub fn janela_regras(app: &mut App, ctx: &egui::Context) {
    let aberta_id = egui::Id::new("st_regras_aberta");
    if !ctx.data(|d| d.get_temp::<bool>(aberta_id)).unwrap_or(false) {
        return;
    }
    let est_id = egui::Id::new("st_regras_rascunho");
    let mut est: Rascunho = ctx.data(|d| d.get_temp(est_id)).unwrap_or_default();
    if est.base != app.doc().tube_rules {
        est = Rascunho { base: app.doc().tube_rules.clone(), texto: app.doc().tube_rules.clone(), escolhida: est.escolhida };
    }
    let vars: HashMap<String, f64> = app.doc().eval_params().unwrap_or_default();
    let mut aberta = true;
    let mut confirmar = false;
    let fechar = egui::Id::unique(("concluir_janela", "st_regras"));
    if ctx.data_mut(|m| m.remove_temp::<bool>(fechar)).unwrap_or(false) {
        aberta = false;
    }
    egui::Window::new(tr("📐 Regras de dobra de tubo")).id(egui::Id::new("janela_regras_tubo")).open(&mut aberta).default_size([520., 520.]).resizable(true).show(ctx, |ui| {
        if crate::app::done_bar(ui) {
            ui.ctx().data_mut(|m| m.insert_temp(fechar, true));
        }
        ui.label(RichText::new(tr("Os valores são seus: o programa não traz tabela de material nem de norma. Cada regra diz de onde vieram os números.")).weak().small());
        ui.horizontal(|ui| {
            if ui.button(tr("＋ Nova regra")).on_hover_text(tr("Cria uma regra vazia: preencha só o que você sabe.")).clicked() {
                let mut n = est.texto.len() + 1;
                while est.texto.iter().any(|r| r.name == format!("{} {n}", tr("Regra"))) {
                    n += 1;
                }
                est.texto.push(TubeRule { name: format!("{} {n}", tr("Regra")), ..Default::default() });
                est.escolhida = est.texto.len() - 1;
                confirmar = true;
            }
            let tem = est.escolhida < est.texto.len();
            if ui.add_enabled(tem, egui::Button::new(tr("⧉ Duplicar"))).clicked() {
                let mut r = est.texto[est.escolhida].clone();
                r.name = format!("{} {}", r.name, tr("(cópia)"));
                est.texto.push(r);
                est.escolhida = est.texto.len() - 1;
                confirmar = true;
            }
            if ui.add_enabled(tem, egui::Button::new(tr("🗑 Excluir"))).on_hover_text(tr("As dobras que usavam a regra passam a avisar que ela não existe mais.")).clicked() {
                est.texto.remove(est.escolhida);
                est.escolhida = est.escolhida.saturating_sub(1);
                confirmar = true;
            }
        });
        if est.texto.is_empty() {
            ui.label(RichText::new(tr("Nenhuma regra. Crie uma para guardar raio mínimo, retorno elástico e alongamento admissível do seu processo.")).weak());
            return;
        }
        ui.horizontal_wrapped(|ui| {
            for (i, r) in est.texto.iter().enumerate() {
                if ui.selectable_label(est.escolhida == i, &r.name).clicked() {
                    est.escolhida = i;
                }
            }
        });
        ui.separator();
        let i = est.escolhida.min(est.texto.len() - 1);
        let r = &mut est.texto[i];
        let mut commit = false;
        let vars = &vars;
        ui.horizontal(|ui| {
            ui.add_sized([150., 18.], egui::Label::new(tr("Nome"))).on_hover_text(tr("Nome da regra; é o que a dobra escolhe."));
            commit |= ui.add(egui::TextEdit::singleline(&mut r.name).desired_width(200.)).lost_focus();
        });
        ui.horizontal(|ui| {
            ui.add_sized([150., 18.], egui::Label::new(tr("Material"))).on_hover_text(tr("Material a que a regra se aplica, em texto livre. Vazio vale para qualquer."));
            commit |= ui.add(egui::TextEdit::singleline(&mut r.material).desired_width(200.)).lost_focus();
        });
        ui.horizontal(|ui| {
            ui.add_sized([150., 18.], egui::Label::new(tr("Seção"))).on_hover_text(tr("Tipo de tubo a que a regra se aplica."));
            egui::ComboBox::from_id_salt("st_regra_secao").selected_text(tr(secao_nome(r.section))).show_ui(ui, |ui| {
                for s in [None, Some(SectionKind::TuboQuadrado), Some(SectionKind::TuboRetangular), Some(SectionKind::TuboRedondo)] {
                    if ui.selectable_label(r.section == s, tr(secao_nome(s))).clicked() {
                        r.section = s;
                        commit = true;
                    }
                }
            });
        });
        ui.horizontal(|ui| {
            ui.add_sized([150., 18.], egui::Label::new(tr("Ferramenta"))).on_hover_text(tr("Processo ou ferramenta: mandril, rolos, dobradeira. Só informativo."));
            commit |= ui.add(egui::TextEdit::singleline(&mut r.tool).desired_width(200.)).lost_focus();
        });
        commit |= campo(ui, "Raio mínimo", "Raio de centro mínimo em mm. A dobra avisa se for menor.", &mut r.min_radius, vars, "mm", "");
        commit |= campo(ui, "Raio mínimo em múltiplos", "Raio de centro mínimo como múltiplo do diâmetro (redondo) ou da dimensão no plano da dobra.", &mut r.min_radius_x, vars, "×", "");
        commit |= campo(ui, "Deslocamento da neutra", "Quanto a linha neutra se desloca para dentro da curva, em mm. Encurta o comprimento desenvolvido.", &mut r.neutral_shift, vars, "mm", "");
        commit |= campo(ui, "Retorno elástico", "Graus que a máquina dobra além do ângulo pedido.", &mut r.springback, vars, "°", "");
        commit |= campo(ui, "Folga", "Folga em mm usada pelo processo.", &mut r.clearance, vars, "mm", "");
        commit |= campo(ui, "Alongamento admissível", "Alongamento máximo da fibra externa, em fração (0,25 = 25%). Dá o raio mínimo de centro (meia dimensão ÷ alongamento).", &mut r.max_strain, vars, "", "");
        commit |= campo(ui, "K da placa", "Fator K da placa externa nos modos em V.", &mut r.plate_k, vars, "", "");
        commit |= campo(ui, "Folga de raiz", "Folga de raiz da solda no canto com gabarito, em mm.", &mut r.root_gap, vars, "mm", "");
        commit |= campo(ui, "Raio mínimo da articulação", "Raio mínimo da linha de articulações do V múltiplo, em mm.", &mut r.min_hinge_radius, vars, "mm", "");
        ui.horizontal(|ui| {
            ui.add_sized([150., 18.], egui::Label::new(tr("Fonte dos valores"))).on_hover_text(tr("De onde vêm os números: ensaio, norma aberta, regra interna. Aparece ao lado do aviso."));
            commit |= ui.add(egui::TextEdit::singleline(&mut r.source).desired_width(260.).hint_text(tr("ensaio, regra interna…"))).lost_focus();
        });
        confirmar = confirmar || commit;
    });
    if est.texto != app.doc().tube_rules && (confirmar || !aberta) {
        let antes = app.open.doc.clone();
        // renomear uma regra leva junto as dobras que a usavam
        if est.texto.len() == est.base.len() {
            for (velha, nova) in est.base.iter().zip(&est.texto) {
                if velha.name != nova.name {
                    renomear_em_dobras(&mut app.open.doc, &velha.name, &nova.name);
                }
            }
        }
        app.open.doc.tube_rules = est.texto.clone();
        app.push_undo_pub(antes);
        est.base = est.texto.clone();
    }
    if aberta {
        ctx.data_mut(|d| d.insert_temp(est_id, est));
    } else {
        ctx.data_mut(|d| {
            d.remove_temp::<Rascunho>(est_id);
            d.insert_temp(aberta_id, false);
        });
    }
}

fn renomear_em_dobras(doc: &mut Document, velha: &str, nova: &str) {
    for f in &mut doc.features {
        if let FeatureKind::StructMember(m) = &mut f.kind {
            for b in m.joints.iter_mut().filter_map(|j| j.bend.as_mut()) {
                if b.rule == velha {
                    b.rule = nova.to_string();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renomear_regra_leva_as_dobras() {
        let mut d = Document::new("x");
        let mk = |rule: &str| StructJoint { mine: 1, other: Id(1), other_path: 0, mode: CornerMode::Dobra, first: false, gap: None, bend: Some(TubeBend { rule: rule.into(), ..Default::default() }) };
        let m = StructMember {
            paths: vec![],
            family: String::new(),
            designation: String::new(),
            kind: SectionKind::TuboRedondo,
            dims: vec![],
            anchor: 0,
            angle: "0".into(),
            off_x: "0".into(),
            off_y: "0".into(),
            ext1: "0".into(),
            ext2: "0".into(),
            corner: CornerMode::MeiaEsquadria,
            gap: "0".into(),
            joints: vec![mk("a"), mk("b")],
            material: String::new(),
            mirror: false,
        };
        let f = d.add_feature(FeatureKind::StructMember(m));
        renomear_em_dobras(&mut d, "a", "novo");
        let Some(FeatureKind::StructMember(m)) = d.feature(f).map(|f| &f.kind) else { unreachable!() };
        assert_eq!(m.joints[0].bend.as_ref().unwrap().rule, "novo");
        assert_eq!(m.joints[1].bend.as_ref().unwrap().rule, "b");
    }

    #[test]
    fn raio_inicial_passa_da_metade_da_seccao() {
        let mut m = StructMember {
            paths: vec![],
            family: String::new(),
            designation: String::new(),
            kind: SectionKind::TuboRetangular,
            dims: vec!["60".into(), "30".into(), "3".into()],
            anchor: 0,
            angle: "0".into(),
            off_x: "0".into(),
            off_y: "0".into(),
            ext1: "0".into(),
            ext2: "0".into(),
            corner: CornerMode::MeiaEsquadria,
            gap: "0".into(),
            joints: vec![],
            material: String::new(),
            mirror: false,
        };
        let v = HashMap::new();
        // 2,5 vezes a maior das duas medidas da seção
        assert_eq!(raio_inicial(&m, &v), "150");
        m.kind = SectionKind::TuboRedondo;
        assert_eq!(raio_inicial(&m, &v), "150");
        m.dims.clear();
        assert_eq!(raio_inicial(&m, &v), "100");
    }
}
