// SPDX-License-Identifier: LGPL-2.1-or-later
//! Fabricação da chapa na interface: nesting em chapas comerciais (B116) e simulação da sequência
//! de dobra (B173). Janela livre, aberta só quando pedida, com prévia desenhada pelas cores do tema.
//!
//! O estado vive na memória do egui (`Est`); os motores estão em `fenkra_solidos::nesting` e
//! `fenkra_solidos::chapa_dobra_sim`. Valores de punção, matriz, máquina e material são do usuário:
//! a janela não traz tabela embutida, só campos.

use crate::i18n::tr;
use eframe::egui::{self, RichText};
use fenkra_core::regras::EstiloRegras;
use fenkra_core::Document;
use fenkra_solidos::chapa_dobra_sim as sim;
use fenkra_solidos::nesting::{self as nest, ChapaComercial, Layout, Opcoes, PecaNesting};
use fenkra_solidos::planificada::{self, Planificada};

const ID: &str = "chapa_fab_estado";

/// Campos de punção: nome, ângulo, raio da ponta, altura, largura do corpo.
type LinhaPuncao = [String; 5];
/// Campos de matriz: nome, abertura do V, ângulo do V, altura, ombro.
type LinhaMatriz = [String; 5];

#[derive(Clone)]
struct Est {
    aberta: bool,
    aba: u8,
    // nesting
    pecas: Vec<PecaNesting>,
    qtd: usize,
    larg: f64,
    alt: f64,
    margem: f64,
    espaco: f64,
    girar: bool,
    esforco: usize,
    res: Option<(Layout, Opcoes)>,
    ver: usize,
    // dobra
    plana: Option<Planificada>,
    puncoes: Vec<LinhaPuncao>,
    matrizes: Vec<LinhaMatriz>,
    altura_livre: String,
    material: [String; 4],
    sequencia: Vec<usize>,
    sim: Option<sim::Simulacao>,
    nos: Option<(usize, bool)>,
    msg: Option<(String, bool)>,
}

impl Default for Est {
    fn default() -> Self {
        Est {
            aberta: false,
            aba: 0,
            pecas: vec![],
            qtd: 1,
            larg: 3000.,
            alt: 1250.,
            margem: 5.,
            espaco: 3.,
            girar: true,
            esforco: 1,
            res: None,
            ver: 0,
            plana: None,
            puncoes: vec![],
            matrizes: vec![],
            altura_livre: String::new(),
            material: ["material".into(), String::new(), String::new(), "1".into()],
            sequencia: vec![],
            sim: None,
            nos: None,
            msg: None,
        }
    }
}

/// Abre a janela (botão da janela de chapa e `FENKRA_DEMO_FAB`).
pub fn abrir(ctx: &egui::Context) {
    ctx.data_mut(|m| m.get_temp_mut_or_default::<Est>(egui::Id::new(ID)).aberta = true);
}

/// Estado de demonstração para capturas (`FENKRA_DEMO_FAB=nesting|dobra`). Os valores de ferramenta
/// são de exemplo, só para a captura; o programa não traz tabela de ferramentas.
pub fn demo(ctx: &egui::Context, doc: &Document, modo: &str) {
    ctx.data_mut(|m| {
        let e = m.get_temp_mut_or_default::<Est>(egui::Id::new(ID));
        e.aberta = true;
        e.aba = u8::from(modo == "dobra" || modo == "inviavel");
        e.puncoes = vec![["Punção exemplo".into(), "88".into(), "1".into(), "100".into(), "30".into()]];
        e.matrizes = vec![["Matriz exemplo".into(), "8*espessura".into(), "88".into(), "60".into(), "20".into()]];
        e.altura_livre = "300".into();
        e.material = ["Material exemplo".into(), "250".into(), "200000".into(), "1".into()];
        if let Ok(p) = planificada::planificar(doc) {
            if let Ok(n) = PecaNesting::da_planificada(&doc.name, &p, 12) {
                e.pecas.push(n);
            }
            e.plana = Some(p);
        }
        e.larg = 600.;
        e.alt = 400.;
        e.margem = 5.;
        e.esforco = 6;
    });
    // roda uma vez o que a captura mostra
    ctx.data_mut(|m| {
        let e = m.get_temp_mut_or_default::<Est>(egui::Id::new(ID));
        if modo == "inviavel" {
            // B173: matriz larga demais para a bainha curta, na ordem da tabela: a dobra
            // inviável aparece marcada no 3D
            e.matrizes[0][1] = "40".into();
            e.sequencia = e.plana.as_ref().map(ordem_do_usuario).unwrap_or_default();
            simular(e);
        } else if modo == "dobra" {
            sugerir(e, doc);
        } else {
            rodar_nesting(e);
        }
    });
}

fn opcoes(e: &Est) -> Opcoes {
    Opcoes { chapa: ChapaComercial { largura: e.larg, altura: e.alt, margem: e.margem }, espaco: e.espaco, rotacoes: if e.girar { vec![0., 90., 180., 270.] } else { vec![0.] }, esforco: e.esforco.max(1) }
}

fn rodar_nesting(e: &mut Est) {
    if e.pecas.is_empty() {
        e.msg = Some((tr("Adicione ao menos uma peça.").to_string(), true));
        return;
    }
    let op = opcoes(e);
    let l = nest::dispor(&e.pecas, &op, None);
    e.msg = (!l.nao_coube.is_empty()).then(|| (format!("{} {}", l.nao_coube.len(), tr("cópia(s) não cabem na chapa")), true));
    e.ver = 0;
    e.res = Some((l, op));
}

fn num(s: &str, esp: f64) -> f64 {
    fenkra_core::regras::avalia(s, &Default::default(), (fenkra_core::estilo_chapa::VAR_ESPESSURA, esp)).unwrap_or(f64::NAN)
}

/// Monta o ferramental e o material dos campos; erro em texto quando falta valor do usuário.
fn ferramental(e: &Est, esp: f64) -> Result<(sim::Ferramental, sim::Material), String> {
    let mut f = sim::Ferramental::default();
    for l in &e.puncoes {
        let est = EstiloRegras::novo(&l[0]).com("angulo", &l[1]).com("raio_ponta", &l[2]).com("altura", &l[3]).com("largura_corpo", &l[4]);
        f.puncoes.push(sim::puncao_de(&est, esp)?);
    }
    for l in &e.matrizes {
        let est = EstiloRegras::novo(&l[0]).com("abertura_v", &l[1]).com("angulo_v", &l[2]).com("altura", &l[3]).com("ombro", &l[4]);
        f.matrizes.push(sim::matriz_de(&est, esp)?);
    }
    f.altura_livre = if e.altura_livre.trim().is_empty() { 0. } else { num(&e.altura_livre, esp) };
    let m = &e.material;
    let est = EstiloRegras::novo(&m[0]).com("escoamento", &m[1]).com("modulo", &m[2]).com("calibracao", &m[3]);
    Ok((f, sim::material_de(&est, esp)?))
}

fn sugerir(e: &mut Est, doc: &Document) {
    if e.plana.is_none() {
        match planificada::planificar(doc) {
            Ok(p) => e.plana = Some(p),
            Err(er) => {
                e.msg = Some((er, true));
                return;
            }
        }
    }
    let Some(p) = e.plana.clone() else { return };
    match ferramental(e, p.espessura).and_then(|(f, m)| sim::sugerir_ordem(&p, &f, &m, None, 20_000)) {
        Ok(Some(s)) => {
            e.sequencia = s.sequencia;
            e.nos = Some((s.nos, s.truncada));
            e.sim = Some(s.simulacao);
            e.msg = None;
        }
        Ok(None) => {
            e.nos = None;
            e.msg = Some((tr("Nenhuma ordem viável com este ferramental: veja o motivo simulando a ordem da tabela de dobras.").to_string(), true));
            e.sequencia = ordem_do_usuario(&p);
            simular(e);
        }
        Err(er) => e.msg = Some((er, true)),
    }
}

/// B304: a sequência na ordem dos glifos (número repetido mantém a posição do mapa entre eles).
pub fn ordem_do_usuario(p: &Planificada) -> Vec<usize> {
    let mut v: Vec<usize> = (0..p.dobras.len()).collect();
    v.sort_by_key(|i| (p.dobras[*i].ordem, *i));
    v
}

fn simular(e: &mut Est) {
    let Some(p) = e.plana.clone() else { return };
    match ferramental(e, p.espessura).and_then(|(f, m)| sim::simular(&p, &e.sequencia, &f, &m, None)) {
        Ok(s) => e.sim = Some(s),
        Err(er) => e.msg = Some((er, true)),
    }
}

/// Janela de fabricação. Chamada a cada quadro; só desenha quando aberta.
pub fn janela(ctx: &egui::Context, doc: &Document) {
    let id = egui::Id::new(ID);
    // captura de desenvolvimento: abre já com um exemplo, uma vez
    if let Ok(modo) = std::env::var("FENKRA_DEMO_FAB") {
        let k = egui::Id::new("chapa_fab_demo_feito");
        if !ctx.data(|m| m.get_temp::<bool>(k)).unwrap_or(false) {
            ctx.data_mut(|m| m.insert_temp(k, true));
            demo(ctx, doc, &modo);
        }
    }
    let mut e: Est = ctx.data(|m| m.get_temp(id)).unwrap_or_default();
    if !e.aberta {
        return;
    }
    let mut aberta = true;
    egui::Window::new(tr("Fabricação da chapa")).id(egui::Id::new("chapa_fab_janela")).open(&mut aberta).default_width(520.).resizable(true).show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut e.aba, 0, tr("Nesting")).on_hover_text(tr("Dispõe as planificadas em chapas comerciais, minimizando a sobra, e grava um DXF por chapa."));
            ui.selectable_value(&mut e.aba, 1, tr("Sequência de dobra")).on_hover_text(tr("Simula a ordem das dobras com o seu punção e a sua matriz: colisão da aba, aba mínima e retorno elástico."));
        });
        ui.separator();
        if e.aba == 0 {
            aba_nesting(ui, &mut e, doc);
        } else {
            aba_dobra(ui, &mut e, doc);
        }
        if let Some((m, erro)) = &e.msg {
            let cor = if *erro { ui.visuals().error_fg_color } else { ui.visuals().text_color() };
            ui.label(RichText::new(m).small().color(cor));
        }
    });
    e.aberta = aberta;
    ctx.data_mut(|m| m.insert_temp(id, e));
}

fn aba_nesting(ui: &mut egui::Ui, e: &mut Est, doc: &Document) {
    crate::editor::section(ui, "Peças");
    ui.horizontal(|ui| {
        ui.add(crate::ui_ext::dv(&mut e.qtd).range(1..=500).prefix(format!("{} ", tr("quantidade"))));
        if ui.button(tr("Adicionar peça atual")).on_hover_text(tr("Planifica a chapa aberta e acrescenta à lista com a quantidade ao lado. Troque de documento e adicione mais para dispor peças diferentes juntas.")).clicked() {
            match planificada::planificar(doc).and_then(|p| PecaNesting::da_planificada(&doc.name, &p, e.qtd)) {
                Ok(p) => {
                    e.pecas.push(p);
                    e.res = None;
                    e.msg = None;
                }
                Err(er) => e.msg = Some((er, true)),
            }
        }
    });
    let mut tirar = None;
    for (i, p) in e.pecas.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            ui.label(RichText::new(&p.nome).small());
            ui.add(crate::ui_ext::dv(&mut p.quantidade).range(1..=500).prefix("× "));
            ui.checkbox(&mut p.girar, tr("girar")).on_hover_text(tr("Desligado fixa a peça na orientação original (sentido do fio do material)."));
            if ui.small_button("✕").clicked() {
                tirar = Some(i);
            }
        });
    }
    if let Some(i) = tirar {
        e.pecas.remove(i);
        e.res = None;
    }
    crate::editor::section(ui, "Chapa comercial");
    egui::Grid::new("fab_chapa").num_columns(4).show(ui, |ui| {
        ui.label(tr("Largura"));
        ui.add(crate::ui_ext::dv(&mut e.larg).range(50.0..=20000.0).suffix(" mm")).on_hover_text(tr("Dimensão da chapa comercial em mm."));
        ui.label(tr("Altura"));
        ui.add(crate::ui_ext::dv(&mut e.alt).range(50.0..=20000.0).suffix(" mm")).on_hover_text(tr("Dimensão da chapa comercial em mm."));
        ui.end_row();
        ui.label(tr("Margem"));
        ui.add(crate::ui_ext::dv(&mut e.margem).range(0.0..=100.0).suffix(" mm")).on_hover_text(tr("Faixa da borda que fica sem peça (pinça e refilo)."));
        ui.label(tr("Espaço"));
        ui.add(crate::ui_ext::dv(&mut e.espaco).range(0.0..=50.0).suffix(" mm")).on_hover_text(tr("Distância mínima entre os contornos de duas peças (folga de corte)."));
        ui.end_row();
    });
    ui.horizontal(|ui| {
        ui.checkbox(&mut e.girar, tr("Permitir rotação")).on_hover_text(tr("Testa 0, 90, 180 e 270 graus por peça. Desligado, usa só a orientação original."));
        ui.add(crate::ui_ext::dv(&mut e.esforco).range(1..=200).prefix(format!("{} ", tr("tentativas")))).on_hover_text(tr("Melhor esforço: repete a disposição com outras ordens das peças e guarda a melhor. Mais tentativas demoram mais."));
    });
    ui.horizontal(|ui| {
        if ui.button(tr("Dispor")).clicked() {
            rodar_nesting(e);
        }
        let tem = e.res.as_ref().is_some_and(|(l, _)| !l.chapas.is_empty());
        if ui.add_enabled(tem, egui::Button::new(tr("Exportar DXF por chapa…"))).on_hover_text(tr("Grava um DXF por chapa mais o relatório de aproveitamento em CSV, na pasta escolhida.")).clicked() {
            if let (Some(pasta), Some((l, op))) = (rfd::FileDialog::new().pick_folder(), &e.res) {
                match nest::exportar(&e.pecas, l, op, &pasta, "nesting") {
                    Ok(v) => e.msg = Some((format!("{} {}", v.len(), tr("arquivos gravados")), false)),
                    Err(er) => e.msg = Some((er, true)),
                }
            }
        }
    });
    let Some((l, op)) = &e.res else { return };
    crate::editor::section(ui, "Aproveitamento");
    ui.label(RichText::new(format!("{} {} · {:.1}%", l.chapas.len(), tr("chapa(s)"), 100. * l.aproveitamento_total(op))).strong());
    if l.chapas.is_empty() {
        return;
    }
    e.ver = e.ver.min(l.chapas.len() - 1);
    ui.horizontal_wrapped(|ui| {
        for i in 0..l.chapas.len() {
            if ui.selectable_label(e.ver == i, format!("{} ({:.0}%)", i + 1, 100. * l.aproveitamento(i, op))).clicked() {
                e.ver = i;
            }
        }
    });
    desenha(ui, &e.pecas, l, e.ver, op);
}

/// Prévia da chapa: só cores do tema (fundo, seleção, erro).
fn desenha(ui: &mut egui::Ui, pecas: &[PecaNesting], l: &Layout, i: usize, op: &Opcoes) {
    let larg = ui.available_width().clamp(200., 700.);
    let esc = larg / op.chapa.largura as f32;
    let alt = (op.chapa.altura as f32 * esc).min(420.);
    let esc = esc.min(alt / op.chapa.altura as f32);
    let (r, _) = ui.allocate_exact_size(egui::vec2(op.chapa.largura as f32 * esc, op.chapa.altura as f32 * esc), egui::Sense::hover());
    let pt = ui.painter_at(r);
    let v = ui.visuals();
    pt.rect_filled(r, 0., v.extreme_bg_color);
    pt.rect_stroke(r, 0., v.widgets.noninteractive.fg_stroke, egui::StrokeKind::Inside);
    let mut mapa = |p: [f64; 2]| egui::pos2(r.left() + p[0] as f32 * esc, r.bottom() - p[1] as f32 * esc);
    for k in &l.chapas[i].colocacoes {
        let poli: Vec<egui::Pos2> = pecas[k.peca].poligono_na_chapa(k).iter().map(|q| mapa(*q)).collect();
        pt.add(egui::Shape::mesh(preenche(&poli, v.selection.bg_fill)));
        pt.add(egui::Shape::closed_line(poli, v.selection.stroke));
    }
}

/// Preenchimento de polígono simples (côncavo inclusive) por corte de orelhas.
fn preenche(p: &[egui::Pos2], cor: egui::Color32) -> egui::Mesh {
    let mut m = egui::Mesh::default();
    let n = p.len();
    if n < 3 {
        return m;
    }
    for q in p {
        m.colored_vertex(*q, cor);
    }
    let area: f32 = (0..n).map(|i| p[i].x * p[(i + 1) % n].y - p[(i + 1) % n].x * p[i].y).sum();
    let mut idx: Vec<usize> = (0..n).collect();
    let cruz = |a: egui::Pos2, b: egui::Pos2, c: egui::Pos2| (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
    let mut guarda = 0;
    while idx.len() > 3 && guarda < n * n {
        guarda += 1;
        let k = idx.len();
        let mut cortou = false;
        for j in 0..k {
            let (a, b, c) = (idx[(j + k - 1) % k], idx[j], idx[(j + 1) % k]);
            let o = cruz(p[a], p[b], p[c]);
            if o * area <= 0. {
                continue;
            }
            let dentro = idx.iter().any(|&t| {
                t != a && t != b && t != c && {
                    let (d1, d2, d3) = (cruz(p[a], p[b], p[t]), cruz(p[b], p[c], p[t]), cruz(p[c], p[a], p[t]));
                    (d1 >= 0. && d2 >= 0. && d3 >= 0.) || (d1 <= 0. && d2 <= 0. && d3 <= 0.)
                }
            });
            if !dentro {
                m.add_triangle(a as u32, b as u32, c as u32);
                idx.remove(j);
                cortou = true;
                break;
            }
        }
        if !cortou {
            break;
        }
    }
    if idx.len() == 3 {
        m.add_triangle(idx[0] as u32, idx[1] as u32, idx[2] as u32);
    }
    m
}

fn aba_dobra(ui: &mut egui::Ui, e: &mut Est, doc: &Document) {
    ui.label(RichText::new(tr("Os valores de punção, matriz, máquina e material são seus: o Fenkra não traz tabela de ferramentas. Fórmulas podem citar espessura (ex.: 8*espessura).")).small().weak());
    crate::editor::section(ui, "Punções");
    egui::Grid::new("fab_punc").num_columns(5).show(ui, |ui| {
        for h in ["Nome", "Ângulo", "Raio da ponta", "Altura", "Largura do corpo"] {
            ui.label(RichText::new(tr(h)).small().strong());
        }
        ui.end_row();
        for l in e.puncoes.iter_mut() {
            for c in l.iter_mut() {
                ui.add(egui::TextEdit::singleline(c).desired_width(70.));
            }
            ui.end_row();
        }
    });
    if ui.small_button(tr("+ punção")).on_hover_text(tr("Acrescenta uma linha para cadastrar um punção do seu ferramental.")).clicked() {
        e.puncoes.push(["punção".into(), String::new(), String::new(), String::new(), String::new()]);
    }
    crate::editor::section(ui, "Matrizes");
    egui::Grid::new("fab_mat").num_columns(5).show(ui, |ui| {
        for h in ["Nome", "Abertura do V", "Ângulo do V", "Altura", "Ombro"] {
            ui.label(RichText::new(tr(h)).small().strong());
        }
        ui.end_row();
        for l in e.matrizes.iter_mut() {
            for c in l.iter_mut() {
                ui.add(egui::TextEdit::singleline(c).desired_width(70.));
            }
            ui.end_row();
        }
    });
    if ui.small_button(tr("+ matriz")).on_hover_text(tr("Acrescenta uma linha para cadastrar uma matriz do seu ferramental.")).clicked() {
        e.matrizes.push(["matriz".into(), String::new(), String::new(), String::new(), String::new()]);
    }
    crate::editor::section(ui, "Máquina e material");
    egui::Grid::new("fab_maq").num_columns(2).show(ui, |ui| {
        ui.label(tr("Altura livre da máquina"));
        ui.add(egui::TextEdit::singleline(&mut e.altura_livre).desired_width(90.)).on_hover_text(tr("Altura acima dos ombros da matriz, com o punção embaixo, em mm. Vazio ignora este teste."));
        ui.end_row();
        ui.label(tr("Escoamento (MPa)"));
        ui.add(egui::TextEdit::singleline(&mut e.material[1]).desired_width(90.)).on_hover_text(tr("Tensão de escoamento do material, da sua ficha técnica."));
        ui.end_row();
        ui.label(tr("Módulo de elasticidade (MPa)"));
        ui.add(egui::TextEdit::singleline(&mut e.material[2]).desired_width(90.)).on_hover_text(tr("Módulo de elasticidade do material, da sua ficha técnica."));
        ui.end_row();
        ui.label(tr("Calibração do retorno"));
        ui.add(egui::TextEdit::singleline(&mut e.material[3]).desired_width(90.)).on_hover_text(tr("Fator que multiplica o retorno elástico calculado. Ajuste com dobras de teste na sua máquina: 1 usa a fórmula pura."));
        ui.end_row();
    });
    ui.horizontal(|ui| {
        if ui.button(tr("Sugerir ordem")).on_hover_text(tr("Busca limitada: só avança por dobras viáveis e guarda a ordem com menos trocas de ferramenta e viradas da chapa.")).clicked() {
            e.plana = None;
            sugerir(e, doc);
        }
        if ui.add_enabled(!e.sequencia.is_empty(), egui::Button::new(tr("Simular a ordem atual"))).clicked() {
            simular(e);
        }
    });
    if let Some((n, trunc)) = e.nos {
        ui.label(RichText::new(format!("{n} {}{}", tr("nós visitados"), if trunc { format!(" ({})", tr("busca truncada no limite")) } else { String::new() })).small().weak());
    }
    let Some(s) = &e.sim else { return };
    crate::editor::section(ui, "Passos");
    let erro = ui.visuals().error_fg_color;
    let mut sobe = None;
    for (n, p) in s.passos.iter().enumerate() {
        ui.horizontal(|ui| {
            let t = format!("{}. {} {} · {:.0}° (+{:.1}°) · {} {:.0}/{:.0} mm · {} / {}{}", n + 1, tr("dobra"), p.ordem_dobra, p.alvo, p.retorno, tr("aba"), p.aba, p.aba_minima, p.puncao, p.matriz, if p.vira { format!(" · {}", tr("virar")) } else { String::new() });
            ui.label(if p.viavel() { RichText::new(t).small() } else { RichText::new(t).small().color(erro) });
            if n > 0 && ui.small_button("↑").clicked() {
                sobe = Some(n);
            }
        });
        for q in &p.problemas {
            ui.label(RichText::new(format!("   ✖ {}", q.texto)).small().color(erro));
        }
        for q in &p.nao_verificado {
            ui.label(RichText::new(format!("   ? {q}")).small().weak());
        }
    }
    ui.label(RichText::new(format!("{} · {} {} · {} {}", if s.viavel { tr("Sequência viável") } else { tr("Sequência inviável") }, s.troca_ferramenta, tr("trocas de ferramenta"), s.viradas, tr("viradas"))).strong());
    if let Some(n) = sobe {
        e.sequencia.swap(n, n - 1);
        simular(e);
    }
}

/// Dobra inviável marcada no 3D: `indice` acha a linha de dobra, `ordem` é o número que o usuário
/// vê (pode repetir, B304) e `texto` diz o passo e o primeiro motivo.
#[derive(Clone, Debug, PartialEq)]
pub struct MarcaInviavel {
    /// índice da dobra em `Planificada::dobras` e em `RecomputeResult::bend_lines`
    pub indice: usize,
    pub ordem: usize,
    pub texto: String,
}

/// Marcas das dobras inviáveis da última simulação, uma por dobra (a primeira vez que ela falha).
pub fn marcas_inviaveis(s: &sim::Simulacao) -> Vec<MarcaInviavel> {
    let mut out: Vec<MarcaInviavel> = vec![];
    for (n, p) in s.passos.iter().enumerate() {
        if p.viavel() || out.iter().any(|m| m.indice == p.dobra) {
            continue;
        }
        let motivo = p.problemas.first().map(|q| q.texto.as_str()).unwrap_or_default();
        out.push(MarcaInviavel { indice: p.dobra, ordem: p.ordem_dobra, texto: format!("✖ {} {} · {} {}: {motivo}", tr("passo"), n + 1, tr("dobra"), p.ordem_dobra) });
    }
    out
}

/// B173: com a janela aberta na sequência de dobra, a dobra inviável aparece no 3D, na peça dobrada
/// ou planificada, como a linha de dobra engrossada na cor de erro do tema e o motivo ao lado.
pub fn overlay_inviaveis(ui: &mut egui::Ui, rect: egui::Rect, cam: &crate::render::Camera, r: &fenkra_solidos::RecomputeResult) {
    let e: Option<Est> = ui.ctx().data(|m| m.get_temp(egui::Id::new(ID)));
    let Some(s) = e.filter(|e| e.aberta && e.aba == 1).and_then(|e| e.sim) else { return };
    let painter = ui.painter_at(rect);
    let cor = crate::theme::st_err();
    let v = |p: [f64; 3]| glam::Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32);
    for m in marcas_inviaveis(&s) {
        let Some(b) = r.bend_lines.get(m.indice) else { continue };
        let linha: Vec<[f64; 3]> = if b.curva.len() > 2 { b.curva.clone() } else { b.center.to_vec() };
        let pts: Vec<egui::Pos2> = linha.iter().filter_map(|q| cam.project(rect, v(*q))).collect();
        if pts.len() < 2 {
            continue;
        }
        painter.add(egui::Shape::line(pts.clone(), egui::Stroke::new(4., cor)));
        // começo e fim da zona dobrada, finos, para ver a faixa que não dobra
        for q in [b.start, b.end].into_iter().filter(|_| b.curva.len() <= 2) {
            if let (Some(a), Some(c)) = (cam.project(rect, v(q[0])), cam.project(rect, v(q[1]))) {
                painter.line_segment([a, c], egui::Stroke::new(1.5, cor));
            }
        }
        let meio = pts[pts.len() / 2];
        let g = painter.layout_no_wrap(m.texto, egui::FontId::proportional(crate::acessibilidade::fonte_3d(12.)), ui.visuals().text_color());
        let lr = egui::Rect::from_min_size(meio + egui::vec2(10., 8.), g.size() + egui::vec2(10., 6.));
        let lr = crate::chapa_manip_ui::reserva_rotulo(ui, lr, rect);
        painter.rect_filled(lr, 4., ui.visuals().extreme_bg_color);
        painter.rect_stroke(lr, 4., egui::Stroke::new(1.5, cor), egui::StrokeKind::Inside);
        painter.galley(lr.min + egui::vec2(5., 3.), g, ui.visuals().text_color());
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn passo(ordem: usize, problema: Option<&str>) -> sim::PassoSim {
        sim::PassoSim {
            dobra: ordem - 1,
            ordem_dobra: ordem,
            puncao: "p".into(),
            matriz: "m".into(),
            alvo: 90.,
            sobredobra: 0.,
            retorno: 0.,
            aba: 20.,
            aba_minima: 10.,
            vira: false,
            problemas: problema.map(|t| vec![sim::Problema { tipo: sim::Tipo::PernaNoPuncao, texto: t.into() }]).unwrap_or_default(),
            nao_verificado: vec![],
        }
    }

    /// B173: só a dobra inviável vira marca, com o passo e o motivo, uma vez por dobra.
    #[test]
    fn marca_so_as_dobras_inviaveis_com_o_motivo() {
        let s = sim::Simulacao { passos: vec![passo(2, None), passo(1, Some("a perna bate no punção")), passo(3, Some("aba curta")), passo(1, Some("de novo"))], viavel: false, troca_ferramenta: 0, viradas: 0 };
        let m = marcas_inviaveis(&s);
        assert_eq!(m.iter().map(|x| x.ordem).collect::<Vec<_>>(), vec![1, 3]);
        assert_eq!(m.iter().map(|x| x.indice).collect::<Vec<_>>(), vec![0, 2]);
        assert!(m[0].texto.contains("2") && m[0].texto.contains("a perna bate no punção"), "{}", m[0].texto);
        assert!(m[1].texto.contains("aba curta"));
        let ok = sim::Simulacao { passos: vec![passo(1, None)], viavel: true, troca_ferramenta: 0, viradas: 0 };
        assert!(marcas_inviaveis(&ok).is_empty());
    }
}
