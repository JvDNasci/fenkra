// SPDX-License-Identifier: LGPL-2.1-or-later
//! B198: painel "Regra plástica" e os campos dos comandos que a seguem.
//!
//! O painel é livre e encaixável, abre pelo botão da aba Análise e fecha em Fechar. Escolhe
//! uma regra da biblioteca (as de fábrica e as da pasta `%APPDATA%\Fenkra\regras_plasticas`), mostra
//! e edita os números da regra que está na peça, grava na biblioteca (a versão sobe) e oferece
//! trazer uma versão mais nova da biblioteca. A regra é dado da peça (`Document::plastico`): muda com
//! desfazer e vai no arquivo.
//!
//! Nos comandos (casca, nervura, saída) o campo governado pela regra mostra a
//! fórmula `plastico_*` com cadeado; o número próprio só entra em modo avançado ([`campo`]). Um
//! comando novo criado com a regra na peça já nasce com a fórmula ([`ao_criar`]).

use crate::app::App;
use crate::i18n::tr;
use eframe::egui::{self, Rect, RichText};
use fenkra_core::regra_plastica::{self as rp, Grandeza, RegraPlastica};
use fenkra_core::{Document, FeatureKind, Id};
use std::cell::RefCell;
use std::collections::HashMap;

#[derive(Default)]
struct Estado {
    aberto: bool,
    biblioteca: Vec<RegraPlastica>,
    erros: Vec<String>,
    carregada: bool,
    escolhida: usize,
    aviso: Option<(String, bool)>,
    demo_lido: bool,
}

thread_local! {
    static ST: RefCell<Estado> = RefCell::new(Estado::default());
}

pub fn abrir() {
    ST.with(|s| s.borrow_mut().aberto = true);
}

pub fn aberto() -> bool {
    ST.with(|s| s.borrow().aberto)
}

/// Pasta da biblioteca do usuário.
pub fn pasta() -> Option<std::path::PathBuf> {
    std::env::var_os("APPDATA").map(|a| std::path::PathBuf::from(a).join("Fenkra").join("regras_plasticas"))
}

fn carrega(e: &mut Estado, forcar: bool) {
    if e.carregada && !forcar {
        return;
    }
    e.carregada = true;
    let (v, erros) = match pasta() {
        Some(p) => rp::carregar_pasta(&p),
        None => (RegraPlastica::de_fabrica(), vec![]),
    };
    e.biblioteca = v;
    e.erros = erros;
}

/// Comando recém-criado com a regra na peça: os campos governados nascem com a fórmula da regra.
pub fn ao_criar(doc: &mut Document, id: Id) {
    if doc.plastico.is_none() {
        return;
    }
    let Some(f) = doc.features.iter_mut().find(|f| f.id == id) else { return };
    let fo = |g| RegraPlastica::formula(g).to_string();
    match &mut f.kind {
        FeatureKind::Shell(s) => s.thickness = fo(Grandeza::Parede),
        FeatureKind::Stiffener(s) => s.thickness = fo(Grandeza::Nervura),
        FeatureKind::Draft(d) => d.angle = fo(Grandeza::Saida),
        _ => {}
    }
}

/// Campo de um comando governado pela regra. Sem regra na peça é o campo de sempre. Com regra e sem
/// modo avançado, o campo fica preso à fórmula e mostra o valor; em modo avançado aceita número
/// próprio, com o botão para voltar à regra.
pub fn campo(ui: &mut egui::Ui, regra: Option<&RegraPlastica>, rotulo: &str, texto: &mut String, g: Grandeza, vars: &HashMap<String, f64>, unidade: &str) {
    let Some(r) = regra else {
        crate::editor::expr(ui, rotulo, texto, vars, unidade);
        return;
    };
    let formula = RegraPlastica::formula(g);
    if !r.avancado {
        if texto.trim() != formula {
            // sobrescrita feita antes (ou valor de antes da regra): sem modo avançado, a regra vale
            *texto = formula.to_string();
        }
        let v = vars.get(formula).copied().unwrap_or(0.);
        ui.horizontal(|ui| {
            ui.add_sized([96., 18.], egui::Label::new(tr(rotulo)));
            ui.label(RichText::new(format!("🔒 {} {unidade}", crate::app::fmt(v))).strong()).on_hover_text(format!("{} {}\n{}", tr("Segue a regra plástica"), r.nome, tr("Para escrever outro valor, ligue o modo avançado no painel Regra plástica.")));
            ui.label(RichText::new(formula).small().weak());
        });
        return;
    }
    crate::editor::expr(ui, rotulo, texto, vars, unidade);
    let mut aviso = None;
    if let Ok(v) = fenkra_core::expr::eval_str(texto, vars) {
        aviso = r.confere(g, v);
    }
    ui.horizontal(|ui| {
        if texto.trim() != formula && ui.small_button(tr("Seguir a regra")).on_hover_text(tr("Volta o campo para a fórmula da regra plástica.")).clicked() {
            *texto = formula.to_string();
        }
        if let Some(a) = aviso {
            ui.label(RichText::new(a).small().color(crate::theme::st_warn()));
        }
    });
}

fn numero(ui: &mut egui::Ui, rotulo: &str, v: &mut f64, unidade: &str, dica: &str) -> bool {
    ui.horizontal(|ui| {
        ui.add_sized([150., 18.], egui::Label::new(tr(rotulo)));
        ui.add(crate::ui_ext::dv(v).speed(0.05).suffix(format!(" {unidade}")).dica(tr(dica))).changed()
    })
    .inner
}

/// Regra de exemplo na peça para a captura sem mouse (`FENKRA_DEMO_PLASTICO=1`, `avancado` ou
/// `aviso`, que baixa a parede máxima para 2,2 mm e faz a casca de 2,5 mm do exemplo avisar).
fn demo(app: &mut App, modo: &str) {
    if app.open.doc.plastico.is_some() {
        return;
    }
    let mut r = RegraPlastica::de_fabrica().into_iter().next();
    if let Some(r) = &mut r {
        r.avancado = modo == "avancado";
        if modo == "aviso" {
            r.espessura_max = 2.2;
        }
    }
    app.open.doc.plastico = r;
    app.request_recompute();
}

pub fn painel(app: &mut App, ctx: &egui::Context, area: Rect) {
    use crate::paineis::{self, Painel};
    let demo_modo = ST.with(|st| {
        let mut e = st.borrow_mut();
        if e.demo_lido {
            return None;
        }
        e.demo_lido = true;
        let m = std::env::var("FENKRA_DEMO_PLASTICO").ok();
        if m.is_some() {
            e.aberto = true;
        }
        m
    });
    if let Some(m) = demo_modo {
        demo(app, &m);
    }
    if !aberto() || app.sem_peca_em_edicao() {
        return;
    }
    let cfg = Painel { nome: "regra_plastica".into(), titulo: tr("Regra plástica").to_string(), padrao: paineis::abaixo_do_cubo(area, 340.), largura: 340., redimensiona: false };
    let antes_doc = app.open.doc.clone();
    let mut regra = app.open.doc.plastico.clone();
    let (mut mudou, mut fechar, mut tirar) = (false, false, false);
    // recursos que saem da regra: o recálculo já pôs o aviso no status
    let fora: Vec<(String, String)> = match (&app.result, &regra) {
        (Some(r), Some(_)) => app.open.doc.features.iter().filter_map(|f| r.status.get(&f.id).and_then(|s| s.message()).filter(|m| m.contains("Regra plástica")).map(|m| (f.name.clone(), m.to_string()))).collect(),
        _ => vec![],
    };
    ST.with(|st| {
        let mut e = st.borrow_mut();
        carrega(&mut e, false);
        let posicao = app.settings.painel(&cfg.nome, paineis::Encaixe::Livre);
        let saida = paineis::mostrar(ctx, area, &cfg, posicao, |ui| {
            ui.label(RichText::new(tr("Espessura, saída e raio que os recursos de plástico seguem. A regra fica na peça e vai junto no arquivo.")).small().weak());
            ui.horizontal(|ui| {
                let nomes: Vec<String> = e.biblioteca.iter().map(|r| format!("{} (v{})", r.nome, r.versao)).collect();
                let i = e.escolhida.min(nomes.len().saturating_sub(1));
                egui::ComboBox::from_id_salt("pl_bib").width(180.).selected_text(nomes.get(i).cloned().unwrap_or_default()).show_ui(ui, |ui| {
                    for (k, n) in nomes.iter().enumerate() {
                        ui.selectable_value(&mut e.escolhida, k, n);
                    }
                }).response.on_hover_text(tr("Regras da biblioteca: as de fábrica e as que você gravou."));
                if ui.button(tr("Usar na peça")).on_hover_text(tr("Copia a regra escolhida para a peça. Os comandos que seguem a regra recalculam.")).clicked() {
                    if let Some(r) = e.biblioteca.get(i) {
                        regra = Some(r.clone());
                        mudou = true;
                    }
                }
            });
            let Some(r) = regra.as_mut() else {
                ui.label(RichText::new(tr("A peça não tem regra plástica.")).italics());
                if ui.button(tr("Fechar")).clicked() {
                    fechar = true;
                }
                return;
            };
            ui.separator();
            ui.label(RichText::new(format!("{}  ·  v{}  ·  {}", r.nome, r.versao, r.material)).strong());
            if let Some(n) = rp::mais_nova(r, &e.biblioteca) {
                let v = n.versao;
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("{} v{v}", tr("A biblioteca tem a versão"))).small().color(crate::theme::st_warn()));
                    if ui.small_button(tr("Trazer")).on_hover_text(tr("Troca a regra da peça pela versão mais nova da biblioteca.")).clicked() {
                        let av = r.avancado;
                        *r = n.clone();
                        r.avancado = av;
                        mudou = true;
                    }
                });
            }
            ui.horizontal(|ui| {
                if ui.button(tr("Guardar na biblioteca")).on_hover_text(tr("Grava a regra na pasta da biblioteca com a versão seguinte.")).clicked() {
                    match pasta().ok_or_else(|| "APPDATA".to_string()).and_then(|p| rp::guardar(&p, r)) {
                        Ok(v) => {
                            r.versao = v;
                            mudou = true;
                            e.aviso = Some((format!("{} v{v}", tr("Gravada como")), false));
                        }
                        Err(x) => e.aviso = Some((x, true)),
                    }
                    carrega(&mut e, true);
                }
                if ui.button(tr("Tirar da peça")).on_hover_text(tr("A peça fica sem regra; os comandos guardam as fórmulas, que ficam sem valor até uma regra voltar.")).clicked() {
                    tirar = true;
                }
                if ui.button(tr("Fechar")).clicked() {
                    fechar = true;
                }
            });
            if let Some((t, erro)) = &e.aviso {
                ui.label(RichText::new(t).small().color(if *erro { crate::theme::st_err() } else { crate::theme::st_ok() }));
            }
            for x in &e.erros {
                ui.label(RichText::new(x).small().color(crate::theme::st_warn()));
            }
            let mut m = false;
            m |= numero(ui, "Espessura nominal", &mut r.espessura, "mm", "Parede que a casca, a parede fina e os recursos usam.");
            m |= numero(ui, "Ângulo de saída", &mut r.saida, "°", "Saída que o comando Saída e os recursos usam.");
            m |= numero(ui, "Raio nominal", &mut r.raio, "mm", "Raio dos filetes e dos cantos dos recursos.");
            egui::CollapsingHeader::new(tr("Limites")).id_salt("pl_lim").default_open(false).show(ui, |ui| {
                m |= numero(ui, "Parede mínima", &mut r.espessura_min, "mm", "Abaixo disso o plástico não enche: o comando avisa na árvore.");
                m |= numero(ui, "Parede máxima", &mut r.espessura_max, "mm", "Acima disso a parede chupa e o ciclo fica longo.");
                m |= numero(ui, "Saída mínima", &mut r.saida_min, "°", "Saída abaixo disso prende no molde.");
                m |= numero(ui, "Raio mínimo", &mut r.raio_min, "mm", "Canto mais vivo que isso concentra tensão.");
                m |= numero(ui, "Nervura / parede", &mut r.nervura_fracao, "", "Espessura da nervura como fração da parede nominal.");
                m |= numero(ui, "Nervura máxima", &mut r.nervura_max, "", "Nervura acima desta fração da parede marca a face oposta.");
            });
            if m {
                mudou = true;
            }
            if let Err(x) = r.valida() {
                ui.label(RichText::new(x).small().color(crate::theme::st_err()));
            }
            mudou |= ui.checkbox(&mut r.avancado, tr("Modo avançado (sobrescrever nos comandos)")).on_hover_text(tr("Deixa escrever um número próprio nos comandos no lugar da regra. O diagnóstico na árvore continua.")).changed();
            if fora.is_empty() {
                ui.label(RichText::new(tr("Todos os recursos seguem a regra.")).small().color(crate::theme::st_ok()));
            } else {
                ui.label(RichText::new(format!("{} {}", fora.len(), tr("recurso(s) fora da regra:"))).small().color(crate::theme::st_warn()));
                for (n, msg) in &fora {
                    ui.label(RichText::new(format!("{n}: {msg}")).small().weak());
                }
            }
        });
        if let Some(m) = saida.movido {
            app.settings.guardar_painel(&cfg.nome, m);
        }
        if fechar {
            e.aberto = false;
        }
    });
    if tirar {
        regra = None;
        mudou = true;
    }
    if mudou {
        app.open.doc.plastico = regra;
        app.push_undo_pub(antes_doc);
    }
}
