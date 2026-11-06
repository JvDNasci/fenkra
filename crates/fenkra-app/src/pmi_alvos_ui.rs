// SPDX-License-Identifier: LGPL-2.1-or-later
//! Painéis do PMI 3D para alvos de referência (B189), rugosidade estruturada e sistema da solda
//! (B190), mais o comando que gera a solda dos cordões da bancada Estrutural.

use crate::app::App;
use crate::i18n::tr;
use eframe::egui::{self, RichText};
use fenkra_core::pmi::{Pmi, PmiKind, SistemaSolda, WeldContour, WeldExtra};
use fenkra_core::pmi_acabamento::{self as ac, AvisoRug, Lay, NormaRug, Remocao, RugExtra};
use fenkra_core::pmi_alvos::{self as al, AlvoMovel, AlvoRef, Aviso, FormaAlvo};
use fenkra_core::Id;

fn campo_num(ui: &mut egui::Ui, rotulo: &str, v: &mut f64, dica: &str) -> bool {
    let mut mudou = false;
    ui.horizontal(|ui| {
        ui.label(tr(rotulo));
        mudou = ui.add(crate::ui_ext::dv(v).speed(0.5).range(0.0..=100000.0).suffix(" mm")).on_hover_text(tr(dica)).changed();
    });
    mudou
}

/// Mensagem do aviso da regra 3-2-1, traduzida.
pub fn texto_aviso(a: &Aviso) -> String {
    match a {
        Aviso::SemReferencia { letra } => format!("{} {letra}: {}", tr("Referência"), tr("não existe; crie a referência com essa letra")),
        Aviso::Falta { letra, tem, pede } => format!("{} {letra}: {tem} {} {pede} ({})", tr("Referência"), tr("de"), tr("regra 3-2-1: faltam alvos")),
        Aviso::Sobra { letra, tem, pede } => format!("{} {letra}: {tem} {} {pede} ({})", tr("Referência"), tr("de"), tr("regra 3-2-1: alvos demais")),
        Aviso::Colineares { letra } => format!("{} {letra}: {}", tr("Referência"), tr("os três pontos estão em linha reta e não definem um plano")),
        Aviso::ReferenciaExtra { letra } => format!("{} {letra}: {}", tr("Referência"), tr("a regra 3-2-1 só tem primária, secundária e terciária")),
        Aviso::NumeroRepetido { id } => format!("{id}: {}", tr("número repetido na mesma referência")),
        Aviso::Invalido { id } => format!("{id}: {}", tr("dimensão, curso ou direção inválidos")),
    }
}

/// Painel do alvo de referência. Devolve `true` se algo mudou.
pub fn alvo_ui(ui: &mut egui::Ui, alvo: &mut AlvoRef, lista: &[Pmi], id: Id) -> bool {
    let mut m = false;
    let letras: Vec<String> = lista.iter().filter_map(|p| if let PmiKind::Datum { letter } = &p.kind { Some(letter.clone()) } else { None }).collect();
    ui.horizontal(|ui| {
        ui.label(tr("Referência"));
        egui::ComboBox::from_id_salt("alvo_letra").selected_text(alvo.letra.clone()).width(60.).show_ui(ui, |ui| {
            for l in &letras {
                m |= ui.selectable_value(&mut alvo.letra, l.clone(), l).changed();
            }
        })
        .response
        .on_hover_text(tr("Letra da referência (datum) a que o alvo pertence. Só aparecem as referências já criadas."));
        ui.label(tr("Número"));
        m |= ui.add(crate::ui_ext::dv(&mut alvo.numero).range(1..=99)).on_hover_text(tr("Número do alvo dentro da referência: A1, A2, A3…")).changed();
    });
    ui.horizontal(|ui| {
        ui.label(tr("Forma"));
        egui::ComboBox::from_id_salt("alvo_forma").selected_text(tr(alvo.forma.label())).show_ui(ui, |ui| {
            for f in FormaAlvo::ALL {
                m |= ui.selectable_value(&mut alvo.forma, f, tr(f.label())).changed();
            }
        })
        .response
        .on_hover_text(tr("Ponto, linha ou área de contato com o dispositivo de fixação."));
    });
    match alvo.forma.dimensoes() {
        1 => {
            let (r, d) = match alvo.forma {
                FormaAlvo::Linha => ("Comprimento", "Comprimento da linha de contato."),
                FormaAlvo::Circulo => ("Diâmetro", "Diâmetro da área circular de contato."),
                _ => ("Raio", "Raio do arco de contato."),
            };
            m |= campo_num(ui, r, &mut alvo.dim[0], d);
        }
        2 => {
            m |= campo_num(ui, "Largura", &mut alvo.dim[0], "Dimensão do retângulo ao longo da orientação.");
            m |= campo_num(ui, "Altura", &mut alvo.dim[1], "Dimensão do retângulo na outra direção.");
        }
        _ => {}
    }
    m |= ui.checkbox(&mut alvo.restrita, tr("Área restrita")).on_hover_text(tr("O contorno limita a região da superfície que vale como referência; o desenho mostra o contorno tracejado.")).changed();
    let mut movel = alvo.movel.is_some();
    if ui.checkbox(&mut movel, tr("Alvo móvel")).on_hover_text(tr("O apoio desliza numa direção por um curso até tocar a peça. Não conta na regra 3-2-1.")).changed() {
        alvo.movel = movel.then_some(AlvoMovel { direcao: [0., 0., 1.], curso: 5. });
        m = true;
    }
    if let Some(mv) = &mut alvo.movel {
        ui.horizontal(|ui| {
            ui.label(tr("Direção"));
            for (k, n) in ["X", "Y", "Z"].iter().enumerate() {
                m |= ui.add(crate::ui_ext::dv(&mut mv.direcao[k]).speed(0.05).prefix(*n)).on_hover_text(tr("Sentido do movimento do apoio (não precisa ser unitário).")).changed();
            }
        });
        m |= campo_num(ui, "Curso", &mut mv.curso, "Distância que o apoio percorre.");
    }
    // cotas ligadas: aparecem enquadradas (cota básica)
    let dims: Vec<(Id, String)> = lista.iter().filter(|p| p.kind.is_dimension()).map(|p| (p.id, p.name.clone())).collect();
    if !dims.is_empty() {
        ui.label(RichText::new(tr("Cotas ligadas (ficam enquadradas)")).small());
        egui::ScrollArea::vertical().max_height(80.).id_salt("alvo_cotas").show(ui, |ui| {
            for (d, n) in &dims {
                let mut on = alvo.cotas.contains(d);
                if ui.checkbox(&mut on, n).on_hover_text(tr("A cota que localiza o alvo é básica: aparece dentro de um quadro.")).changed() {
                    if on {
                        alvo.cotas.push(*d);
                    } else {
                        alvo.cotas.retain(|x| x != d);
                    }
                    m = true;
                }
            }
        });
    }
    let _ = id;
    avisos_321(ui, lista);
    m
}

/// Avisos da regra 3-2-1 de toda a lista.
pub fn avisos_321(ui: &mut egui::Ui, lista: &[Pmi]) {
    let v = al::verificar_321(lista);
    if v.is_empty() {
        if lista.iter().any(|p| matches!(p.kind, PmiKind::DatumTarget { .. })) {
            ui.label(RichText::new(format!("✔ {}", tr("regra 3-2-1 atendida"))).small().color(crate::theme::st_ok()));
        }
        return;
    }
    for a in &v {
        ui.label(RichText::new(format!("⚠ {}", texto_aviso(a))).small().color(crate::theme::st_warn()));
    }
}

fn texto_aviso_rug(a: &AvisoRug) -> String {
    match a {
        AvisoRug::ValorInvalido(v) => format!("{} \"{v}\"", tr("valor que não é número:")),
        AvisoRug::LimitesInvertidos => tr("o limite inferior é maior que o superior").to_string(),
        AvisoRug::ParametroDeOutraFamilia(p) => format!("{p}: {}", tr("parâmetro de outra família (perfil usa R, textura de área usa S)")),
        AvisoRug::SobremetalSemRemocao => tr("remoção proibida e sobremetal se contradizem").to_string(),
        AvisoRug::SemEspecificacao => tr("o símbolo não diz nada: preencha um valor ou o processo").to_string(),
    }
}

/// Painel da rugosidade estruturada.
pub fn rugosidade_ui(ui: &mut egui::Ui, ra: &mut String, process: &mut String, x: &mut RugExtra) {
    ui.horizontal(|ui| {
        ui.label(tr("Parâmetro"));
        egui::ComboBox::from_id_salt("rug_param").selected_text(x.parametro().to_string()).width(60.).show_ui(ui, |ui| {
            for p in ["Ra", "Rz", "Rq", "Rt", "Sa", "Sq", "Sz"] {
                if ui.selectable_label(x.parametro() == p, p).clicked() {
                    x.param = if p == "Ra" || p == "Sa" { String::new() } else { p.to_string() };
                    x.area = p.starts_with('S');
                }
            }
        })
        .response
        .on_hover_text(tr("Ra, Rz, Rq e Rt medem o perfil; Sa, Sq e Sz medem a textura da área."));
    });
    ui.horizontal(|ui| {
        ui.label(tr("Limite superior"));
        ui.add(egui::TextEdit::singleline(ra).desired_width(60.)).on_hover_text(tr("Valor máximo do parâmetro, em µm."));
        ui.label(tr("inferior"));
        ui.add(egui::TextEdit::singleline(&mut x.limite_inf).desired_width(60.)).on_hover_text(tr("Valor mínimo, quando há faixa. Vazio = só o limite superior."));
    });
    ui.checkbox(&mut x.maximo, tr("Regra do máximo")).on_hover_text(tr("Nenhuma medida pode passar do valor, em vez de aceitar até 16 % acima."));
    ui.horizontal(|ui| {
        ui.label(tr("Amostragem"));
        ui.add(egui::TextEdit::singleline(&mut x.amostragem).desired_width(50.)).on_hover_text(tr("Comprimento de amostragem, em mm."));
        ui.label(tr("Sobremetal"));
        ui.add(egui::TextEdit::singleline(&mut x.sobremetal).desired_width(50.)).on_hover_text(tr("Sobremetal de usinagem, em mm."));
    });
    ui.horizontal(|ui| {
        ui.label(tr("Estrias"));
        egui::ComboBox::from_id_salt("rug_lay").selected_text(if x.lay == Lay::Nenhuma { tr("nenhuma").to_string() } else { x.lay.simbolo().to_string() }).width(70.).show_ui(ui, |ui| {
            for l in Lay::ALL {
                ui.selectable_value(&mut x.lay, l, format!("{} {}", l.simbolo(), tr(l.label())));
            }
        })
        .response
        .on_hover_text(tr("Direção das estrias da superfície em relação ao plano de projeção."));
    });
    ui.horizontal(|ui| {
        ui.label(tr("Remoção"));
        egui::ComboBox::from_id_salt("rug_rem").selected_text(tr(x.remocao.label())).show_ui(ui, |ui| {
            for r in Remocao::ALL {
                ui.selectable_value(&mut x.remocao, r, tr(r.label()));
            }
        })
        .response
        .on_hover_text(tr("Obrigatória: a superfície é usinada. Proibida: fica como saiu do processo anterior."));
    });
    ui.checkbox(&mut x.todo_contorno, tr("Todo o contorno")).on_hover_text(tr("A mesma exigência vale para todas as superfícies do contorno da peça."));
    ui.checkbox(&mut x.area, tr("Textura de área")).on_hover_text(tr("Parâmetros S (ISO 25178) em vez do perfil. Troca o parâmetro para Sa."));
    if x.area && !x.parametro().starts_with('S') {
        x.param = String::new();
    }
    ui.horizontal(|ui| {
        ui.label(tr("Norma"));
        egui::ComboBox::from_id_salt("rug_norma").selected_text(x.norma.label()).show_ui(ui, |ui| {
            for n in NormaRug::ALL {
                ui.selectable_value(&mut x.norma, n, n.label());
            }
        })
        .response
        .on_hover_text(tr("Muda a posição dos campos e o traço do símbolo; os valores são os mesmos."));
    });
    ui.horizontal(|ui| {
        ui.label(tr("Processo"));
        ui.text_edit_singleline(process).on_hover_text(tr("Processo de fabricação, abaixo da barra."));
    });
    ui.label(RichText::new(ac::resumo(ra, process, x)).weak());
    for a in ac::validar(ra, process, x) {
        ui.label(RichText::new(format!("⚠ {}", texto_aviso_rug(&a))).small().color(crate::theme::st_warn()));
    }
    ui.label(RichText::new(tr("Na troca de dados vai como PMI gráfico: o AP242 não tem entidade semântica para rugosidade.")).weak().small());
}

/// Campos de solda além dos básicos (sistema, acabamento, penetração).
pub fn solda_sistema_ui(ui: &mut egui::Ui, x: &mut WeldExtra) {
    ui.separator();
    ui.horizontal(|ui| {
        ui.label(tr("Sistema"));
        egui::ComboBox::from_id_salt("solda_sistema").selected_text(tr(x.sistema.label())).show_ui(ui, |ui| {
            for s in SistemaSolda::ALL {
                ui.selectable_value(&mut x.sistema, s, tr(s.label()));
            }
        })
        .response
        .on_hover_text(tr("A: linha cheia e tracejada. B: duas linhas cheias. AWS: uma linha, lado da seta embaixo e o outro em cima."));
    });
    if x.contour != WeldContour::Nenhum {
        ui.horizontal(|ui| {
            ui.label(tr("Acabamento"));
            ui.add(egui::TextEdit::singleline(&mut x.acabamento).desired_width(30.)).on_hover_text(tr("Letra do método de acabamento do contorno: G esmerilhado, C usinado, M, R laminado, H martelado."));
        });
    }
    ui.checkbox(&mut x.penetracao, tr("Penetração total")).on_hover_text(tr("Meia-lua cheia do lado oposto ao da junta: o metal atravessa a junta."));
    ui.label(RichText::new(tr("Na troca de dados vai como PMI gráfico: o AP242 não tem entidade semântica para solda.")).weak().small());
}

/// Solda dos cordões da bancada Estrutural: um símbolo por cordão, com a medida da perna, o
/// comprimento (e o passo, se intermitente) e o contorno do perfil. Refaz os já gerados.
pub fn solda_dos_cordoes(app: &mut App) {
    let cordoes = fenkra_solidos::weld_beads(&app.open.doc);
    if cordoes.is_empty() {
        app.toast(tr("Nenhum cordão de solda no modelo: crie um cordão na bancada Estrutural"), true);
        return;
    }
    let nomes: Vec<String> = cordoes.iter().map(|c| app.open.doc.features.iter().find(|f| f.id == c.feature).map(|f| f.name.clone()).unwrap_or_default()).collect();
    let antes = app.open.doc.clone();
    app.push_undo_pub(antes);
    let (mut novos, mut refeitos) = (0, 0);
    for (c, nome) in cordoes.iter().zip(nomes) {
        let Some(ponto) = c.caminhos.first().and_then(|k| k.pts.get(k.pts.len() / 2)) else { continue };
        let a = *ponto;
        let titulo = format!("{} {}", tr("Solda do cordão"), nome);
        // medida, contorno e n×l (e) saem do mesmo cálculo do símbolo do desenho (B285)
        let (size, dw) = fenkra_desenho::solda_cordao::solda_do_cordao(c);
        let kind = PmiKind::Weld { kind: dw.kind, size, all_around: c.caminhos.iter().all(|k| k.fechado), extra: dw.extra };
        if let Some(p) = crate::pmi_ui::list_mut(app).iter_mut().find(|p| p.name == titulo) {
            p.kind = kind;
            p.anchors = vec![a];
            refeitos += 1;
        } else {
            let id = Id(app.next_pmi_id());
            let pm = Pmi { id, name: titulo, kind, anchors: vec![a], offset: [15., 0., 15.], text: String::new(), tol_plus: String::new(), tol_minus: String::new(), visible: true, plane: None, style: Default::default(), folder: String::new(), targets: vec![] };
            crate::pmi_ui::list_mut(app).push(pm);
            novos += 1;
        }
    }
    app.toast(format!("{novos} {}, {refeitos} {}", tr("símbolos de solda criados"), tr("refeitos")), false);
}
