// SPDX-License-Identifier: LGPL-2.1-or-later
//! B114/B115: janela "Análise de pórtico". Os membros retos viram barras
//! (`fenkra_solidos::st_portico_doc`); aqui se marcam os apoios de cada nó, as cargas nos nós e nas
//! linhas e o peso próprio, que ficam guardados na peça. O resultado mostra, por barra, os maiores
//! esforços e a flecha, e no 3D o diagrama escolhido (N, V, M ou a deformada) desenhado sobre a barra,
//! os apoios e as cargas. A verificação pela NBR 8800 (B115) usa os mesmos esforços.

use crate::app::App;
use crate::i18n::tr;
use crate::render::Camera;
use eframe::egui::{self, RichText, Stroke};
use fenkra_core::Id;
use fenkra_solidos::st_portico::{Portico, Resultado};
use fenkra_solidos::st_portico_doc::{analisar_documento, secoes_aco, CasoPortico};
use fenkra_solidos::st_verifica_aco::{relatorio, verificar, Verificacao};
use glam::Vec3;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Diagrama {
    Normal,
    Cortante,
    #[default]
    Momento,
    Deformada,
}

impl Diagrama {
    const TODOS: [Diagrama; 4] = [Diagrama::Normal, Diagrama::Cortante, Diagrama::Momento, Diagrama::Deformada];
    fn nome(self) -> &'static str {
        match self {
            Diagrama::Normal => "Normal (N)",
            Diagrama::Cortante => "Cortante (V)",
            Diagrama::Momento => "Momento (M)",
            Diagrama::Deformada => "Deformada",
        }
    }
}

type Analise = Result<(Portico, Vec<(Id, usize)>, Resultado), String>;

#[derive(Clone)]
struct Estado {
    aberta: bool,
    diagrama: Diagrama,
    /// nó e linha em edição
    no: usize,
    linha: usize,
    fy: f64,
    k: f64,
    verificar: bool,
    analise: Option<Arc<Analise>>,
    verif: Option<Arc<Vec<Verificacao>>>,
    chave: Option<(u64, CasoPortico, u64, u64)>,
}

impl Default for Estado {
    fn default() -> Self {
        Estado { aberta: false, diagrama: Diagrama::Momento, no: 0, linha: 0, fy: 250., k: 1., verificar: true, analise: None, verif: None, chave: None }
    }
}

fn id() -> egui::Id {
    egui::Id::unique("st_portico")
}

pub fn abrir(ctx: &egui::Context) {
    let mut e: Estado = ctx.data(|d| d.get_temp(id())).unwrap_or_default();
    e.aberta = true;
    ctx.data_mut(|d| d.insert_temp(id(), e));
}

fn tipo_de(g: Option<[bool; 6]>) -> usize {
    match g {
        None => 0,
        Some([true, true, true, false, false, false]) => 1,
        Some([true, true, true, true, true, true]) => 2,
        Some([false, false, true, false, false, false]) => 3,
        Some(_) => 4,
    }
}

const TIPOS: [&str; 5] = ["Livre", "Rótula", "Engaste", "Rolete (só Z)", "Outro"];

fn graus(t: usize) -> Option<[bool; 6]> {
    match t {
        1 => Some([true, true, true, false, false, false]),
        2 => Some([true; 6]),
        3 => Some([false, false, true, false, false, false]),
        _ => None,
    }
}

fn campo(ui: &mut egui::Ui, nome: &str, v: &mut f64, sufixo: &str, dica: &str) {
    ui.label(tr(nome));
    let r = ui.add(crate::ui_ext::dv(v).speed(1.).suffix(sufixo).max_decimals(3).dica(tr(dica)));
    crate::ui_ext::wheel(ui, &r, v);
    ui.end_row();
}

/// Documento de demonstração: pórtico de dois pilares de 3 m e viga de 4 m em tubo quadrado
/// 100x5, engastado na base, com 2 N/mm na viga, 5 kN horizontais no topo e peso próprio.
pub fn demo_se_pedido(app: &mut App) {
    if std::env::var("FENKRA_DEMO_PORTICO").is_err() {
        return;
    }
    use fenkra_core::*;
    let s = |x: &str| x.to_string();
    let mut d = Document::new("Pórtico");
    let pts = [[0., 0., 0.], [0., 0., 3000.], [4000., 0., 3000.], [4000., 0., 0.]];
    let mut paths = vec![];
    for w in pts.windows(2) {
        paths.push(CurveRef::Ref(d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed(w[0]), b: PointRef::Fixed(w[1]), start: s("0"), end: s("0") })));
    }
    let id = d.add_feature(FeatureKind::StructMember(StructMember {
        paths,
        family: s("Tubo quadrado"),
        designation: s("100x5"),
        kind: SectionKind::TuboQuadrado,
        dims: vec![s("100"), s("5")],
        anchor: 0,
        angle: s("0"),
        off_x: s("0"),
        off_y: s("0"),
        ext1: s("0"),
        ext2: s("0"),
        corner: CornerMode::MeiaEsquadria,
        gap: s("0"),
        joints: vec![],
        material: String::new(),
        mirror: false,
    }));
    for r in &mut d.references {
        r.visible = false;
    }
    let mut c = CasoPortico { peso_proprio: true, ..Default::default() };
    c.por_apoio(pts[0], Some([true; 6]));
    c.por_apoio(pts[3], Some([true; 6]));
    c.por_q((id, 1), [0., 0., -2.]);
    c.por_carga(pts[1], [5000., 0., 0., 0., 0., 0.]);
    c.gravar(&mut d);
    app.load_doc_pub(d, None);
    app.settings.bench = crate::settings::Bench::Estrutural;
    app.started = true;
}

pub fn window(app: &mut App, ctx: &egui::Context) {
    let mut e: Estado = ctx.data(|d| d.get_temp(id())).unwrap_or_default();
    // desenvolvimento: FENKRA_DEMO_PORTICO=<n|v|m|d> abre o pórtico de exemplo com a janela e esse diagrama
    static DEMO: std::sync::Once = std::sync::Once::new();
    DEMO.call_once(|| {
        if let Ok(v) = std::env::var("FENKRA_DEMO_PORTICO") {
            e.aberta = true;
            e.diagrama = match v.as_str() {
                "n" => Diagrama::Normal,
                "v" => Diagrama::Cortante,
                "d" => Diagrama::Deformada,
                _ => Diagrama::Momento,
            };
        }
    });
    if !e.aberta {
        ctx.data_mut(|d| d.insert_temp(id(), e));
        return;
    }
    let caso = CasoPortico::ler(app.doc());
    let chave = (app.geracao(), caso.clone(), e.fy.to_bits(), e.k.to_bits());
    if e.chave.as_ref() != Some(&chave) {
        let a = analisar_documento(app.doc());
        e.verif = a.as_ref().ok().and_then(|(p, o, r)| secoes_aco(app.doc(), o, e.fy).ok().map(|s| Arc::new(verificar(p, r, &s, e.fy, e.k))));
        e.analise = Some(Arc::new(a));
        e.chave = Some(chave);
    }
    let mut novo = caso.clone();
    let mut aberta = true;
    let analise = e.analise.clone();
    egui::Window::new(tr("Análise de pórtico")).id(egui::Id::unique("janela_portico")).open(&mut aberta).default_size([520., 600.]).resizable(true).show(ctx, |ui| {
        // o modelo de barras sai do documento mesmo sem apoio, para listar os nós
        let modelo = fenkra_solidos::st_portico::modelo_da_estrutura(app.doc());
        let (p, origem) = match &modelo {
            Ok(m) => (&m.0, &m.1),
            Err(err) => {
                ui.label(RichText::new(err).color(crate::theme::st_err()));
                return;
            }
        };
        ui.label(RichText::new(format!("{} {} · {} {}", p.nos.len(), tr("nós"), p.barras.len(), tr("barras"))).weak());
        ui.checkbox(&mut novo.peso_proprio, tr("Peso próprio")).on_hover_text(tr("Soma o peso de cada barra (área × densidade do material × g), para baixo."));
        egui::CollapsingHeader::new(tr("Apoios")).id_salt("portico_apoios").default_open(true).show(ui, |ui| {
            egui::ScrollArea::vertical().id_salt("portico_apoios_rol").max_height(130.).show(ui, |ui| {
                egui::Grid::new("portico_apoios_grade").num_columns(2).striped(true).show(ui, |ui| {
                    for (k, n) in p.nos.iter().enumerate() {
                        ui.label(format!("{} {} ({:.0}, {:.0}, {:.0})", tr("Nó"), k + 1, n.p[0], n.p[1], n.p[2]));
                        let atual = tipo_de(novo.apoio_em(n.p));
                        let mut t = atual;
                        egui::ComboBox::from_id_salt(("portico_apoio", k)).selected_text(tr(TIPOS[t])).show_ui(ui, |ui| {
                            for (i, nome) in TIPOS.iter().enumerate().take(4) {
                                ui.selectable_value(&mut t, i, tr(nome));
                            }
                        });
                        if t != atual {
                            novo.por_apoio(n.p, graus(t));
                        }
                        ui.end_row();
                    }
                });
            });
        });
        egui::CollapsingHeader::new(tr("Cargas")).id_salt("portico_cargas").default_open(true).show(ui, |ui| {
            e.no = e.no.min(p.nos.len().saturating_sub(1));
            egui::ComboBox::from_label(tr("Carga no nó")).selected_text(format!("{} {}", tr("Nó"), e.no + 1)).show_ui(ui, |ui| {
                for k in 0..p.nos.len() {
                    ui.selectable_value(&mut e.no, k, format!("{} {}", tr("Nó"), k + 1));
                }
            });
            if let Some(n) = p.nos.get(e.no) {
                let mut f = novo.carga_em(n.p);
                let antes = f;
                egui::Grid::new("portico_carga_no").num_columns(2).show(ui, |ui| {
                    campo(ui, "Fx", &mut f[0], " N", "Força no nó ao longo de X global.");
                    campo(ui, "Fy", &mut f[1], " N", "Força no nó ao longo de Y global.");
                    campo(ui, "Fz", &mut f[2], " N", "Força no nó ao longo de Z global (negativo para baixo).");
                });
                if f != antes {
                    novo.por_carga(n.p, f);
                }
            }
            // linhas de membro (uma carga por linha vale em todos os trechos dela)
            let mut linhas: Vec<(Id, usize)> = origem.clone();
            linhas.dedup();
            e.linha = e.linha.min(linhas.len().saturating_sub(1));
            let nome = |l: (Id, usize)| format!("{} · {} {}", app.doc().name_of(l.0), tr("linha"), l.1 + 1);
            egui::ComboBox::from_label(tr("Carga distribuída na linha")).selected_text(linhas.get(e.linha).map(|l| nome(*l)).unwrap_or_default()).show_ui(ui, |ui| {
                for (k, l) in linhas.iter().enumerate() {
                    ui.selectable_value(&mut e.linha, k, nome(*l));
                }
            });
            if let Some(l) = linhas.get(e.linha) {
                let mut q = novo.q_da_linha(*l);
                let antes = q;
                egui::Grid::new("portico_carga_linha").num_columns(2).show(ui, |ui| {
                    campo(ui, "qx", &mut q[0], " N/mm", "Carga distribuída ao longo de X global, por comprimento de barra.");
                    campo(ui, "qy", &mut q[1], " N/mm", "Carga distribuída ao longo de Y global, por comprimento de barra.");
                    campo(ui, "qz", &mut q[2], " N/mm", "Carga distribuída ao longo de Z global, por comprimento de barra (negativo para baixo).");
                });
                if q != antes {
                    novo.por_q(*l, q);
                }
            }
        });
        ui.separator();
        ui.horizontal(|ui| {
            ui.label(tr("Diagrama no 3D"));
            for d in Diagrama::TODOS {
                ui.selectable_value(&mut e.diagrama, d, tr(d.nome()));
            }
        });
        match analise.as_deref() {
            Some(Ok((_, _, r))) => {
                egui::Grid::new("portico_resultado").striped(true).num_columns(5).show(ui, |ui| {
                    for h in ["Barra", "N máx. (N)", "V máx. (N)", "M máx. (N·m)", "Flecha (mm)"] {
                        ui.label(RichText::new(tr(h)).strong());
                    }
                    ui.end_row();
                    for (k, b) in r.barras.iter().enumerate() {
                        ui.label(format!("{}", k + 1));
                        ui.label(format!("{:.0}", b.maximo(|q| q.n)));
                        ui.label(format!("{:.0}", b.maximo(|q| q.vy.hypot(q.vz))));
                        ui.label(format!("{:.1}", b.maximo(|q| q.my.hypot(q.mz)) / 1000.));
                        ui.label(format!("{:.2}", b.maximo(|q| q.v.hypot(q.w))));
                        ui.end_row();
                    }
                });
                let soma = r.reacoes.iter().fold([0.; 3], |a, x| [a[0] + x[0], a[1] + x[1], a[2] + x[2]]);
                ui.label(RichText::new(format!("{}: Rx {:.0} N · Ry {:.0} N · Rz {:.0} N", tr("Soma das reações"), soma[0], soma[1], soma[2])).small());
            }
            Some(Err(err)) => {
                ui.label(RichText::new(err).color(crate::theme::st_warn()));
            }
            None => {}
        }
        // B115: verificação das barras
        egui::CollapsingHeader::new(tr("Verificação NBR 8800")).id_salt("portico_verif").default_open(true).show(ui, |ui| {
            egui::Grid::new("portico_verif_campos").num_columns(2).show(ui, |ui| {
                ui.label(tr("fy"));
                let r = ui.add(crate::ui_ext::dv(&mut e.fy).speed(1.).range(100.0..=700.0).suffix(" MPa").dica(tr("Resistência ao escoamento do aço (ASTM A36: 250 MPa).")));
                crate::ui_ext::wheel(ui, &r, &mut e.fy);
                ui.end_row();
                ui.label(tr("K"));
                let r = ui.add(crate::ui_ext::dv(&mut e.k).speed(0.05).range(0.5..=2.5).max_decimals(2).dica(tr("Coeficiente de flambagem: 1 biarticulada, 0,5 biengastada, 2 em balanço.")));
                crate::ui_ext::wheel(ui, &r, &mut e.k);
                ui.end_row();
            });
            ui.checkbox(&mut e.verificar, tr("Cor da utilização no 3D")).on_hover_text(tr("Pinta cada barra pela utilização: verde até 0,9, laranja até 1, vermelho acima."));
            match e.verif.as_deref() {
                Some(v) => {
                    egui::ScrollArea::vertical().id_salt("portico_verif_rol").max_height(160.).show(ui, |ui| {
                        ui.label(RichText::new(relatorio(v, e.fy)).small().monospace());
                    });
                }
                None => {
                    ui.label(RichText::new(tr("Sem verificação: a análise não rodou ou a seção não entra (U, T, cantoneira).")).weak());
                }
            }
        });
        ui.label(RichText::new(tr("Análise linear de barras (Euler-Bernoulli); não substitui o cálculo de um engenheiro.")).weak().small());
    });
    if novo != caso {
        let antes = app.open.doc.clone();
        novo.gravar(&mut app.open.doc);
        app.push_undo_pub(antes);
    }
    e.aberta = aberta;
    ctx.data_mut(|d| d.insert_temp(id(), e));
}

fn v3(q: [f64; 3]) -> Vec3 {
    Vec3::new(q[0] as f32, q[1] as f32, q[2] as f32)
}

/// No 3D: os diagramas sobre as barras, os apoios e as cargas.
pub fn overlay(ui: &mut egui::Ui, rect: egui::Rect, cam: &Camera) {
    let Some(e) = ui.ctx().data(|d| d.get_temp::<Estado>(id())) else { return };
    if !e.aberta {
        return;
    }
    let Some(Ok((p, _, r))) = e.analise.as_deref() else { return };
    let painter = ui.painter_at(rect);
    let proj = |q: [f64; 3]| cam.project(rect, v3(q));
    let add = |a: [f64; 3], b: [f64; 3], k: f64| [a[0] + b[0] * k, a[1] + b[1] * k, a[2] + b[2] * k];
    // tamanho da estrutura para a escala dos diagramas
    let (lo, hi) = p.nos.iter().fold(([f64::MAX; 3], [f64::MIN; 3]), |(lo, hi), n| ([0, 1, 2].map(|i| lo[i].min(n.p[i])), [0, 1, 2].map(|i| hi[i].max(n.p[i]))));
    let tam = (0..3).map(|i| hi[i] - lo[i]).fold(0., f64::max).max(1.);
    let valor = |q: &fenkra_solidos::st_portico::Esforcos, plano_y: bool| -> f64 {
        match e.diagrama {
            Diagrama::Normal => q.n,
            Diagrama::Cortante => if plano_y { q.vy } else { q.vz },
            Diagrama::Momento => if plano_y { q.mz } else { q.my },
            Diagrama::Deformada => 0.,
        }
    };
    // o plano de cada barra: o que tem o maior esforço do diagrama
    let planos: Vec<bool> = r.barras.iter().map(|b| b.maximo(|q| valor(q, true)) >= b.maximo(|q| valor(q, false))).collect();
    let maximo = r.barras.iter().zip(&planos).map(|(b, &py)| b.maximo(|q| valor(q, py))).fold(0., f64::max);
    let flecha_max = r.barras.iter().map(|b| b.maximo(|q| q.v.hypot(q.w))).fold(0., f64::max);
    let cor = crate::theme::accent(ui.visuals());
    for (k, (b, rb)) in p.barras.iter().zip(&r.barras).enumerate() {
        let a = p.nos[b.i].p;
        let [ex, ey, ez] = rb.eixos;
        // barra pintada pela utilização
        if e.verificar {
            if let Some(v) = e.verif.as_deref().and_then(|v| v.iter().find(|x| x.barra == k)) {
                let c = if v.utilizacao > 1. { crate::theme::st_err() } else if v.utilizacao > 0.9 { crate::theme::st_warn() } else { crate::theme::st_ok() };
                if let (Some(pa), Some(pb)) = (proj(a), proj(p.nos[b.j].p)) {
                    painter.line_segment([pa, pb], Stroke::new(4., c.gamma_multiply(0.7)));
                }
            }
        }
        let mut linha: Vec<egui::Pos2> = vec![];
        let plano_y = planos[k];
        for (i, q) in rb.diagrama.iter().enumerate() {
            let base = add(a, ex, q.x);
            let ponta = if e.diagrama == Diagrama::Deformada {
                let esc = if flecha_max > 1e-12 { 0.08 * tam / flecha_max } else { 0. };
                add(add(base, ey, q.v * esc), ez, q.w * esc)
            } else {
                let esc = if maximo > 1e-12 { 0.12 * tam / maximo } else { 0. };
                // o momento fica do lado tracionado (sinal da convenção: Mz > 0 traciona −y)
                let lado = if plano_y { geo_neg(ey) } else { ez };
                let dir = if e.diagrama == Diagrama::Momento { lado } else if plano_y { ey } else { ez };
                add(base, dir, valor(q, plano_y) * esc)
            };
            if let Some(s) = proj(ponta) {
                linha.push(s);
                if e.diagrama != Diagrama::Deformada && i % 4 == 0 {
                    if let Some(sb) = proj(base) {
                        painter.line_segment([sb, s], Stroke::new(1., cor.gamma_multiply(0.45)));
                    }
                }
            }
        }
        painter.add(egui::Shape::line(linha, Stroke::new(2., cor)));
        // rótulo do maior valor da barra
        let (mut melhor, mut onde) = (0f64, None);
        for q in &rb.diagrama {
            let v = if e.diagrama == Diagrama::Deformada { q.v.hypot(q.w) } else { valor(q, plano_y) };
            if v.abs() > melhor.abs() {
                melhor = v;
                onde = Some(add(a, ex, q.x));
            }
        }
        if let Some(s) = onde.and_then(proj) {
            let txt = match e.diagrama {
                Diagrama::Normal | Diagrama::Cortante => format!("{:.0} N", melhor),
                Diagrama::Momento => format!("{:.1} N·m", melhor / 1000.),
                Diagrama::Deformada => format!("{:.2} mm", melhor),
            };
            painter.text(s + egui::vec2(4., -4.), egui::Align2::LEFT_BOTTOM, txt, egui::FontId::proportional(12.), ui.visuals().text_color());
        }
    }
    // apoios: triângulo (rótula), quadrado (engaste), círculo (rolete)
    for n in &p.nos {
        let Some(s) = proj(n.p) else { continue };
        let c = ui.visuals().text_color();
        match tipo_de(n.apoio.iter().any(|&x| x).then_some(n.apoio)) {
            0 => {}
            1 => {
                painter.add(egui::Shape::convex_polygon(vec![s, s + egui::vec2(-8., 12.), s + egui::vec2(8., 12.)], c.gamma_multiply(0.5), Stroke::new(1.5, c)));
            }
            2 => {
                painter.rect_stroke(egui::Rect::from_center_size(s + egui::vec2(0., 6.), egui::vec2(16., 12.)), 0., Stroke::new(2., c), egui::StrokeKind::Middle);
            }
            _ => {
                painter.circle_stroke(s + egui::vec2(0., 7.), 6., Stroke::new(1.5, c));
            }
        }
        // carga no nó: seta de 10% do tamanho na direção da força
        let f = [n.carga[0], n.carga[1], n.carga[2]];
        let l = (f[0] * f[0] + f[1] * f[1] + f[2] * f[2]).sqrt();
        if l > 1e-9 {
            if let Some(o) = proj(add(n.p, f, -0.1 * tam / l)) {
                painter.arrow(o, s - o, Stroke::new(2., crate::theme::st_warn()));
            }
        }
    }
}

fn geo_neg(v: [f64; 3]) -> [f64; 3] {
    [-v[0], -v[1], -v[2]]
}
