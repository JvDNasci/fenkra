// SPDX-License-Identifier: LGPL-2.1-or-later
//! Painéis dos comandos do corpo de malha (B170). A prévia é o próprio corpo na viewport: o comando
//! recalcula a cada mudança e a malha guardada por conteúdo deixa isso barato.

use crate::editor::{check, expr, icon_choice, section};
use crate::i18n::tr;
use eframe::egui::{self, RichText};
use fenkra_core::malha::{MeshBoolean, MeshDecimate, MeshOp, MeshOpKind, MeshRepair};
use fenkra_core::{BoolOp, Document, FeatureKind, Id, MeshImport};
use fenkra_solidos::corpo_malha::{corpos_de_malha, desvio_malha_brep, embutir};
use std::collections::HashMap;
use std::path::Path;

type Vars = HashMap<String, f64>;

/// Comando de malha pedido pela barra.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AcaoMalha {
    Reparar,
    Decimar,
    Booleana,
    ParaBrep,
    DeBrep,
}

/// Coletor que tem comando de malha ou corpo: o outro lado de uma booleana.
fn outro_coletor(d: &Document) -> Option<Id> {
    let ativo = d.active_collector;
    let ocupado = |c: Id| d.features.iter().any(|f| f.target == c && !matches!(f.kind, FeatureKind::Sketch(_)));
    d.collectors.iter().map(|c| c.id).find(|c| *c != ativo && ocupado(*c))
}

/// Operação nova com os valores de partida. Devolve a mensagem quando falta o segundo corpo.
pub fn nova_op(a: AcaoMalha, d: &Document) -> Result<MeshOpKind, &'static str> {
    Ok(match a {
        AcaoMalha::Reparar => MeshOpKind::Reparar(MeshRepair::default()),
        AcaoMalha::Decimar => MeshOpKind::Decimar(MeshDecimate::default()),
        AcaoMalha::Booleana => {
            let tool = outro_coletor(d).ok_or("Crie outro coletor de corpos com o corpo ferramenta primeiro")?;
            MeshOpKind::Booleana(MeshBoolean { tool, op: BoolOp::Cut, keep_tool: false })
        }
        AcaoMalha::ParaBrep => MeshOpKind::ParaBrep { solid: true },
        AcaoMalha::DeBrep => MeshOpKind::DeBrep { desvio: "0".into(), angulo: "15".into() },
    })
}

/// Lê um arquivo para a peça aberta como corpo de malha. Coletor ocupado ganha um vizinho, para a malha
/// não se misturar com o sólido.
pub fn importar_para_doc(d: &mut Document, p: &Path) -> Result<Id, String> {
    let cena = fenkra_malha::ler_arquivo(p).map_err(|e| e.to_string())?;
    let partes = cena.no_lugar();
    let malha = fenkra_malha::juntar(&partes).soldada();
    let ativo = d.active_collector;
    if d.features.iter().any(|f| f.target == ativo && !matches!(f.kind, FeatureKind::Sketch(_))) {
        let nome = p.file_stem().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "Malha".into());
        d.active_collector = d.add_collector(&nome);
    }
    let dados = embutir(d, &malha);
    Ok(d.add_feature(FeatureKind::MeshImport(MeshImport { path: p.to_string_lossy().into(), solid: false, corpo: true, dados })))
}

fn dica(ui: &mut egui::Ui, texto: &str) {
    ui.label(RichText::new(tr(texto)).weak().small());
}

/// Painel do comando de malha.
pub fn painel(ui: &mut egui::Ui, doc: &Document, me: Id, m: &mut MeshOp, vars: &Vars) {
    match &mut m.op {
        MeshOpKind::Reparar(r) => {
            section(ui, "Reparar malha");
            expr(ui, "Solda", &mut r.solda, vars, "mm");
            dica(ui, "Vértices mais perto que isto viram um só. Zero junta só os idênticos.");
            check(ui, &mut r.nao_manifold, "separar não manifold");
            dica(ui, "Peças que se tocam por uma aresta ou um ponto ficam separadas, sem perder triângulo.");
            check(ui, &mut r.normais, "corrigir normais");
            dica(ui, "Vira as faces que discordam da vizinha e a casca que estiver do avesso.");
            check(ui, &mut r.furos, "fechar furos");
            if r.furos {
                ui.horizontal(|ui| {
                    ui.add_sized([96., 18.], egui::Label::new(tr("Maior furo")));
                    ui.add(crate::ui_ext::dv(&mut r.furo_maximo).speed(1.).range(0..=100_000).suffix(tr(" arestas")).dica(tr("Furo com mais arestas que isto fica aberto. Zero fecha todos.")));
                });
            }
            check(ui, &mut r.soltas, "tirar partes soltas");
            if r.soltas {
                expr(ui, "Parte mínima", &mut r.solta_minima, vars, "");
                dica(ui, "Fica quem tem pelo menos esta fração da área da maior parte.");
            }
        }
        MeshOpKind::Decimar(d) => {
            section(ui, "Decimar malha");
            expr(ui, "Proporção", &mut d.proporcao, vars, "");
            dica(ui, "Fração dos triângulos que fica, de 0 a 1. O contorno aberto não encolhe.");
            expr(ui, "Desvio máximo", &mut d.desvio, vars, "mm");
            dica(ui, "Para antes da proporção se a superfície for se afastar mais que isto. Zero desliga.");
        }
        MeshOpKind::Booleana(b) => {
            section(ui, "Operação");
            let opts = [
                ("op_adicionar", "Unir", "Junta os dois corpos", b.op == BoolOp::Add),
                ("op_remover", "Subtrair", "Tira o corpo ferramenta deste", b.op == BoolOp::Cut),
                ("op_intersecao", "Interseção", "Fica só o volume comum", b.op == BoolOp::Intersect),
            ];
            if let Some(i) = icon_choice(ui, &opts) {
                b.op = [BoolOp::Add, BoolOp::Cut, BoolOp::Intersect][i];
            }
            ui.horizontal(|ui| {
                ui.add_sized([96., 18.], egui::Label::new(tr("Corpo ferramenta")));
                let nome = doc.collectors.iter().find(|x| x.id == b.tool).map(|x| x.name.clone()).unwrap_or_default();
                egui::ComboBox::from_id_salt("malha_ferramenta").selected_text(nome).width(170.).show_ui(ui, |ui| {
                    for c in &doc.collectors {
                        ui.selectable_value(&mut b.tool, c.id, &c.name);
                    }
                });
            });
            check(ui, &mut b.keep_tool, "manter o corpo ferramenta visível");
            dica(ui, "Os dois corpos precisam ser cascas fechadas. Corpo B-rep entra pela malha dele.");
        }
        MeshOpKind::ParaBrep { solid } => {
            section(ui, "Malha para sólido");
            check(ui, solid, "fechar em sólido quando a malha for fechada");
            dica(ui, "Cada triângulo vira uma face plana; malhas grandes ficam pesadas (máx. 200 mil triângulos).");
        }
        MeshOpKind::DeBrep { desvio, angulo } => {
            section(ui, "Sólido para malha");
            expr(ui, "Desvio da malha", desvio, vars, "mm");
            dica(ui, "Quanto a malha pode se afastar da superfície exata. Zero usa um milésimo da diagonal.");
            expr(ui, "Ângulo da malha", angulo, vars, "°");
            dica(ui, "Ângulo máximo entre triângulos vizinhos numa superfície curva.");
        }
    }
    conferir(ui, doc, me);
}

/// Painel do corpo de malha importado.
pub fn painel_importar(ui: &mut egui::Ui, doc: &Document, me: Id, m: &MeshImport) {
    section(ui, "Corpo de malha");
    ui.label(RichText::new(&m.path).small());
    dica(ui, "A malha fica guardada no documento, sem teto de triângulos: o arquivo pode sair do lugar.");
    conferir(ui, doc, me);
}

/// Texto do diagnóstico do corpo de um coletor.
fn diagnostico(doc: &Document, coletor: Id) -> Vec<String> {
    let corpos = corpos_de_malha(doc);
    let Some(c) = corpos.iter().find(|c| c.coletor == coletor) else { return vec![tr("O corpo de malha ainda não existe neste ponto.").to_string()] };
    let d = c.entrada.malha.diagnostico_completo();
    let linha = |nome: &str, n: usize| format!("{}: {n}", tr(nome));
    vec![
        linha("Triângulos", d.triangulos),
        linha("Vértices", d.vertices),
        linha("Partes", d.componentes),
        linha("Furos", d.furos),
        linha("Arestas livres", d.arestas_livres),
        linha("Não manifold", d.arestas_nao_manifold + d.vertices_nao_manifold),
        linha("Faces viradas", d.arestas_invertidas),
        linha("Degenerados", d.degenerados + d.duplicados),
        linha("Cruzamentos", d.autointersecoes.unwrap_or(0)),
        tr(if d.estanque() { "Estanque" } else { "Não estanque" }).to_string(),
    ]
}

/// Conferência sob demanda. Mede por botão porque percorre a malha inteira: por quadro, travaria.
fn conferir(ui: &mut egui::Ui, doc: &Document, me: Id) {
    let Some(alvo) = doc.features.iter().find(|f| f.id == me).map(|f| f.target) else { return };
    let chave = egui::Id::new(("malha_conferir", me));
    section(ui, "Conferir");
    if ui.button(tr("Diagnóstico")).on_hover_text(tr("Conta furos, não manifold, faces viradas, partes e cruzamentos do corpo.")).clicked() {
        let linhas = diagnostico(doc, alvo);
        ui.ctx().data_mut(|d| d.insert_temp(chave, linhas));
    }
    let brep: Vec<_> = doc.collectors.iter().filter(|c| c.id != alvo).collect();
    if !brep.is_empty() {
        let escolha = egui::Id::new(("malha_comparar", me));
        let mut atual: Id = ui.ctx().data(|d| d.get_temp(escolha)).unwrap_or(brep[0].id);
        ui.horizontal(|ui| {
            ui.add_sized([96., 18.], egui::Label::new(tr("Comparar com")));
            let nome = brep.iter().find(|c| c.id == atual).map(|c| c.name.clone()).unwrap_or_default();
            egui::ComboBox::from_id_salt(("malha_comparar_cb", me)).selected_text(nome).width(170.).show_ui(ui, |ui| {
                for c in &brep {
                    ui.selectable_value(&mut atual, c.id, &c.name);
                }
            });
        });
        ui.ctx().data_mut(|d| d.insert_temp(escolha, atual));
        if ui.button(tr("Medir desvio")).on_hover_text(tr("Distância entre o corpo de malha e o corpo B-rep, nos dois sentidos.")).clicked() {
            let linhas = match desvio_malha_brep(doc, alvo, atual) {
                Ok(r) => vec![
                    format!("{}: {:.4} mm", tr("Desvio máximo"), r.ida.maximo.max(r.volta.maximo)),
                    format!("{}: {:.4} mm", tr("Desvio médio"), r.ida.media),
                    format!("{}: {:.4} mm", tr("Desvio RMS"), r.ida.rms),
                    format!("{}: {:.4} mm", tr("Incerteza"), r.incerteza),
                ],
                Err(e) => vec![e],
            };
            ui.ctx().data_mut(|d| d.insert_temp(chave, linhas));
        }
    }
    let linhas: Option<Vec<String>> = ui.ctx().data(|d| d.get_temp(chave));
    for l in linhas.unwrap_or_default() {
        ui.label(RichText::new(l).small());
    }
}

/// Esfera ondulada de exemplo para as capturas, com a calota aberta quando o modo pede reparo.
fn malha_demo(aberta: bool) -> fenkra_malha::Malha {
    use std::f64::consts::PI;
    let n = 120usize;
    let mut m = fenkra_malha::Malha::default();
    let raio = |th: f64, ph: f64| 40. + 4. * (5. * ph).sin() * (6. * th).sin();
    let ponto = |th: f64, ph: f64| {
        let r = raio(th, ph);
        [r * th.sin() * ph.cos(), r * th.sin() * ph.sin(), 45. + r * th.cos()]
    };
    m.pos.push(ponto(0., 0.));
    for i in 1..n {
        for j in 0..2 * n {
            m.pos.push(ponto(PI * i as f64 / n as f64, PI * j as f64 / n as f64));
        }
    }
    m.pos.push(ponto(PI, 0.));
    let w = 2 * n;
    let k = |i: usize, j: usize| (1 + (i - 1) * w + j % w) as u32;
    let sul = (m.pos.len() - 1) as u32;
    for j in 0..w {
        if !aberta {
            m.tris.push([0, k(1, j), k(1, j + 1)]);
        }
        m.tris.push([sul, k(n - 1, j + 1), k(n - 1, j)]);
        for i in 1..n - 1 {
            if aberta && i < 12 {
                continue;
            }
            m.tris.push([k(i, j), k(i + 1, j), k(i + 1, j + 1)]);
            m.tris.push([k(i, j), k(i + 1, j + 1), k(i, j + 1)]);
        }
    }
    m
}

/// Monta o documento das capturas e devolve o comando que fica aberto.
pub fn demo_preparar(d: &mut Document, modo: &str) -> Option<Id> {
    let malha = malha_demo(modo == "reparar");
    let dados = embutir(d, &malha);
    let importado = d.add_feature(FeatureKind::MeshImport(MeshImport { path: "exemplo.stl".into(), solid: false, corpo: true, dados }));
    let op = match modo {
        "importar" => return Some(importado),
        "reparar" => MeshOpKind::Reparar(MeshRepair::default()),
        "decimar" => MeshOpKind::Decimar(MeshDecimate { proporcao: "0.05".into(), desvio: "0".into() }),
        "booleana" => {
            let alvo = d.active_collector;
            let tool = d.add_collector("Ferramenta");
            d.active_collector = tool;
            let mut caixa = fenkra_malha::cena::cubo("Ferramenta", 60.);
            caixa.pos.iter_mut().for_each(|p| *p = [p[0] - 5., p[1] - 5., p[2] + 40.]);
            let dados = embutir(d, &caixa);
            d.add_feature(FeatureKind::MeshImport(MeshImport { path: "ferramenta.stl".into(), solid: false, corpo: true, dados }));
            d.active_collector = alvo;
            MeshOpKind::Booleana(MeshBoolean { tool, op: BoolOp::Cut, keep_tool: false })
        }
        _ => return Some(importado),
    };
    Some(d.add_feature(FeatureKind::MeshOp(MeshOp { op })))
}
