// SPDX-License-Identifier: LGPL-2.1-or-later
//! Inserir com miniatura e peça no cursor (B262).
//!
//! O painel lista peças recentes e as da pasta da montagem, com miniatura, e mantém o diálogo do
//! sistema como opção. Clicar numa miniatura põe a peça "na mão": ela segue o cursor como fantasma e
//! cada clique na vista coloca uma cópia. Esc encerra sem deixar nada além do que já foi colocado.
//! Cada cópia é um passo de desfazer. A primeira peça da montagem não segue o cursor: ela ancora o
//! conjunto, então o fantasma fica na origem com o selo "fixa na origem".
//!
//! O fantasma sai pelo `Painter`, como o do catálogo: reconstruir a cena a cada movimento do cursor
//! custaria uma subida de malha inteira por quadro. Peça muito pesada é desenhada com parte dos
//! triângulos, o que basta para ver onde ela vai cair.

use crate::app::App;
use crate::biblioteca_ui::{self, Malha};
use crate::i18n::tr;
use crate::paineis::{self, Encaixe, Painel};
use crate::render::Camera;
use eframe::egui::{self, Pos2, Rect, RichText, Sense, Stroke};
use fenkra_core::Id;
use fenkra_montagem::insercao::{self, Escolha};
use fenkra_montagem::{links, Assembly, Placement, V3};
use fenkra_project::qualidade_modelo::{self, Medidas, Momento, Perfil, PreviaInsercao, Relatorio};
use glam::Vec3;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub const PAINEL: &str = "inserir_peca";
/// Limite de triângulos do fantasma: acima disso o desenho por quadro deixa o cursor lento.
const MAX_TRIS: usize = 40_000;
/// Lado da miniatura, em pontos.
const LADO: f32 = 92.;
/// Arquivos da pasta mostrados no painel.
const MAX_PASTA: usize = 48;

/// Peça na mão: o que vai para o documento no clique e o que se desenha até lá.
pub struct NaMao {
    pub escolha: Escolha,
    pub malha: Arc<Malha>,
    /// ponto da peça que fica sob o cursor: centro da caixa em X e Y, base em Z
    pub apoio: V3,
    /// triângulos que cada cópia soma à montagem
    pub triangulos: u64,
    /// medidas da peça pelo recálculo, para o checador de qualidade do PLM (B305); vazio em subconjunto
    pub medidas: Option<Medidas>,
    /// identidade do documento da peça, para achar as exceções dela no projeto
    pub uid: String,
}

/// Comando Inserir aberto.
pub struct Estado {
    /// `true` para subconjunto (.fka), `false` para peça (.fkp)
    pub sub: bool,
    /// grupo escolhido ao abrir: as cópias nascem dentro dele
    pub pai: Option<Id>,
    pub filtro: String,
    pub mao: Option<NaMao>,
    /// arquivos que o diálogo trouxe e ainda não entraram: Esc passa para o próximo
    pub fila: Vec<PathBuf>,
    pub colocadas: usize,
    /// último componente colocado: vira a seleção ao encerrar. Durante o comando a seleção fica vazia
    /// para o manipulador não aparecer e roubar o clique
    pub ultima: Option<Id>,
    /// posição fixa do cursor, em fração da vista: captura de tela sem mouse (`FENKRA_DEMO_ASM`)
    pub cursor_demo: Option<[f32; 2]>,
    /// perfil de qualidade do projeto, lido ao abrir o painel (B305)
    pub perfil: Option<Perfil>,
    /// as próximas cópias entram na versão simplificada (B125)
    pub simplificada: bool,
    /// motivo pelo qual o perfil de qualidade do projeto barra a peça da mão (B305); vazio = pode entrar
    pub bloqueada: Option<String>,
    /// um clique na vista foi recusado pelo perfil: quem desenha avisa, para o clique não sumir mudo
    pub recusada: bool,
    /// documento aberto já pedido à fila de miniaturas (B362)
    chave_aberta: String,
}

impl Estado {
    pub fn novo(sub: bool, pai: Option<Id>) -> Self {
        Estado { sub, pai, filtro: String::new(), mao: None, fila: vec![], colocadas: 0, ultima: None, cursor_demo: None, perfil: None, simplificada: false, bloqueada: None, recusada: false, chave_aberta: String::new() }
    }
}

/// Abre o comando. A seleção de um grupo decide onde as peças entram.
pub fn abrir(st: &mut crate::asm_ui::AsmState, sub: bool) {
    let pai = st.sel.filter(|s| st.asm.node(*s).is_some_and(|n| matches!(n.kind, fenkra_montagem::NodeKind::Group)));
    st.sel = None;
    st.inserir = Some(Estado::novo(sub, pai));
}

fn ext(sub: bool) -> &'static str {
    if sub {
        "fka"
    } else {
        "fkp"
    }
}

fn nome_do(p: &Path) -> String {
    p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| tr("Peça").to_string())
}

fn tem_extensao(p: &Path, e: &str) -> bool {
    p.extension().is_some_and(|x| x.eq_ignore_ascii_case(e))
}

/// Recentes que ainda existem e o resto da pasta da montagem, sem repetir nem listar a própria
/// montagem. O filtro olha o nome do arquivo.
fn listar(recentes: &[String], pasta: Option<&Path>, atual: Option<&Path>, sub: bool, filtro: &str) -> (Vec<PathBuf>, Vec<PathBuf>) {
    let e = ext(sub);
    let f = filtro.trim().to_lowercase();
    let passa = |p: &Path| tem_extensao(p, e) && Some(p) != atual && (f.is_empty() || nome_do(p).to_lowercase().contains(&f));
    let r: Vec<PathBuf> = recentes.iter().map(PathBuf::from).filter(|p| passa(p) && p.exists()).collect();
    let mut d: Vec<PathBuf> = pasta
        .and_then(|p| std::fs::read_dir(p).ok())
        .map(|it| it.flatten().map(|x| x.path()).filter(|p| passa(p) && !r.contains(p)).collect())
        .unwrap_or_default();
    d.sort();
    d.truncate(MAX_PASTA);
    (r, d)
}

/// Malha do fantasma a partir dos corpos calculados. Acima do limite, pula triângulos em passo fixo.
fn malha_de<'a>(malhas: impl Iterator<Item = &'a fenkra_kernel::Mesh>) -> Option<Malha> {
    let malhas: Vec<&fenkra_kernel::Mesh> = malhas.collect();
    let total: usize = malhas.iter().map(|m| m.indices.len() / 3).sum();
    let passo = total.div_ceil(MAX_TRIS).max(1);
    let (mut tris, mut normais, mut segs) = (vec![], vec![], vec![]);
    let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    for m in malhas {
        let v = |i: u32| Vec3::new(m.positions[3 * i as usize], m.positions[3 * i as usize + 1], m.positions[3 * i as usize + 2]);
        for p in m.positions.chunks_exact(3) {
            let q = Vec3::new(p[0], p[1], p[2]);
            lo = lo.min(q);
            hi = hi.max(q);
        }
        for t in m.indices.chunks_exact(3).step_by(passo) {
            let (a, b, c) = (v(t[0]), v(t[1]), v(t[2]));
            normais.push((b - a).cross(c - a).normalize_or_zero());
            tris.push([a, b, c]);
        }
        // só as arestas vivas: as de filete viram um emaranhado no fantasma
        let nseg = m.edges.len() / 6;
        for e in (0..m.edge_ranges.len()).filter(|e| m.edge_smooth.get(*e).is_none_or(|x| *x == 0)) {
            let (s0, s1) = (m.edge_ranges[e] as usize, m.edge_ranges.get(e + 1).map_or(nseg, |x| *x as usize));
            for s in m.edges[s0 * 6..s1 * 6].chunks_exact(6) {
                segs.push([Vec3::new(s[0], s[1], s[2]), Vec3::new(s[3], s[4], s[5])]);
            }
        }
    }
    (!tris.is_empty()).then(|| Malha { tris, normais, segs, centro: (lo + hi) / 2., raio: ((hi - lo) / 2.).length().max(1e-3) })
}

fn triangulos_de<'a>(corpos: impl Iterator<Item = &'a fenkra_solidos::BodyResult>) -> u64 {
    corpos.map(|b| b.mesh.indices.len() as u64 / 3).sum()
}

/// Quanto a montagem soma com mais uma cópia da peça na mão, e se passa do teto do perfil.
fn previa_de(st: &crate::asm_ui::AsmState, perfil: &Perfil, mao: &NaMao) -> PreviaInsercao {
    let atuais = st.view.as_ref().map_or(0, |v| triangulos_de(v.bodies.iter()));
    qualidade_modelo::previa_insercao(perfil, atuais, &Medidas { triangulos: mao.triangulos, ..Default::default() }, 1)
}

/// Perfil fora de um projeto: mostra o que encontrou e não barra. Barrar pede uma exceção com nome e
/// motivo, e sem projeto não há onde registrá-la: a peça ficava na mão, e o clique não fazia nada (B362).
pub(crate) fn perfil_sem_projeto() -> Perfil {
    let mut p = Perfil::padrao();
    for l in &mut p.regras {
        l.gravidade = qualidade_modelo::Gravidade::Avisa;
    }
    p
}

/// Arquivo da peça aberta: o dela ou, no projeto, o do documento.
fn caminho_aberto(app: &App) -> Option<PathBuf> {
    let do_projeto = || {
        let (p, id) = (app.project.as_ref()?, app.open.entry?);
        p.entry(id).ok().map(|e| p.root.join(&e.file))
    };
    app.open.file.clone().or_else(do_projeto)
}

/// A peça aberta ainda não está inteira no disco?
fn aberta_por_salvar(app: &App) -> bool {
    app.open.frozen.is_none() && (app.open.modified || caminho_aberto(app).is_none())
}

/// Põe a peça aberta na mão. A montagem guarda o caminho do arquivo, então quem está por salvar é
/// salvo antes, como no Ctrl+S; quem desiste do diálogo fica sem nada na mão.
pub(crate) fn pegar_aberta(app: &mut App, est: &mut Estado) -> Result<(), String> {
    if aberta_por_salvar(app) {
        app.save_pub();
    }
    let f = caminho_aberto(app).filter(|_| !aberta_por_salvar(app)).ok_or_else(|| tr("A peça precisa estar salva para entrar na montagem").to_string())?;
    let st = app.asm.as_mut().ok_or("montagem fechada")?;
    pegar(st, est, &f)?;
    app.settings.add_recent_file(&f);
    Ok(())
}

/// O checador de qualidade do PLM para a peça da mão, "ao inserir na montagem" (B305). Com a peça no
/// projeto, valem as exceções dela (nome e motivo, no diário); fora dele, só o perfil.
pub fn relatorio_da_mao(projeto: Option<&fenkra_project::Project>, perfil: &Perfil, mao: &NaMao) -> Option<Relatorio> {
    let medidas = mao.medidas.as_ref()?;
    let id = projeto.and_then(|p| p.index.documents.iter().find(|d| !mao.uid.is_empty() && d.uid == mao.uid).map(|d| d.id));
    match (projeto, id) {
        (Some(p), Some(id)) => p.avaliar_qualidade(id, medidas, Momento::InserirNaMontagem).ok(),
        _ => Some(qualidade_modelo::avaliar(perfil, medidas, Momento::InserirNaMontagem, &[])),
    }
}

/// Lê o arquivo e monta a malha: a peça pelo cache da montagem, o subconjunto compondo a vista dele.
fn carregar(st: &mut crate::asm_ui::AsmState, arquivo: &Path, sub: bool) -> Result<(Malha, u64), String> {
    let (malha, triangulos) = if sub {
        let txt = std::fs::read_to_string(arquivo).map_err(|e| e.to_string())?;
        let a = Assembly::from_json(&txt)?;
        let mut erros = vec![];
        let (_, r) = fenkra_montagem::view::compose(&a, arquivo.parent(), &mut st.cache, &mut erros);
        (malha_de(r.bodies.iter().map(|b| &b.mesh)), triangulos_de(r.bodies.iter()))
    } else {
        let lp = st.cache.get(arquivo, false)?;
        (malha_de(lp.result.bodies.iter().map(|b| &b.mesh)), triangulos_de(lp.result.bodies.iter()))
    };
    malha.map(|m| (m, triangulos)).ok_or_else(|| format!("{}: {}", nome_do(arquivo), tr("sem corpo para mostrar")))
}

/// Põe o arquivo na mão.
pub fn pegar(st: &mut crate::asm_ui::AsmState, est: &mut Estado, arquivo: &Path) -> Result<(), String> {
    if est.sub && st.file.as_ref().is_some_and(|a| links::depende_de(arquivo, a, None)) {
        return Err(tr("Referência circular: o subconjunto já contém esta montagem").to_string());
    }
    let (malha, triangulos) = carregar(st, arquivo, est.sub)?;
    // medidas da peça para o checador do PLM; o subconjunto só conta triângulos (as peças dele já passaram)
    let (medidas, uid) = if est.sub { (None, String::new()) } else { st.cache.get(arquivo, false).map(|lp| (Some(crate::app::medidas_do_resultado(&lp.result, &lp.doc)), lp.doc.uid.0.clone())).unwrap_or((None, String::new())) };
    let (c, raio) = (malha.centro, malha.raio);
    let base = malha.tris.iter().flatten().map(|p| p.z).fold(f32::MAX, f32::min).min(c.z + raio);
    est.mao = Some(NaMao {
        escolha: Escolha { path: st.rel_path(arquivo), nome: nome_do(arquivo), sub: est.sub },
        malha: Arc::new(malha),
        apoio: [c.x as f64, c.y as f64, base as f64],
        triangulos,
        medidas,
        uid,
    });
    est.bloqueada = None;
    Ok(())
}

/// Coloca uma cópia, com um passo de desfazer só dela.
pub fn colocar_copia(st: &mut crate::asm_ui::AsmState, est: &mut Estado, origem: V3) -> Option<Id> {
    // o perfil de qualidade do projeto barrou a peça: nada entra até haver exceção registrada (B305)
    if est.bloqueada.is_some() {
        return None;
    }
    let e = est.mao.as_ref()?.escolha.clone();
    st.push_undo();
    let id = insercao::colocar(&mut st.asm, &e, origem, est.pai);
    if let Some(n) = st.asm.node_mut(id) {
        n.simplificada = est.simplificada;
    }
    st.dirty = true;
    est.colocadas += 1;
    est.ultima = Some(id);
    Some(id)
}

/// Esc: larga a peça da mão; o próximo arquivo do diálogo entra, e sem fila o comando fecha.
/// Devolve `false` quando o comando acabou.
fn encerrar_mao(st: &mut crate::asm_ui::AsmState, est: &mut Estado) -> Result<bool, String> {
    est.mao = None;
    while !est.fila.is_empty() {
        let f = est.fila.remove(0);
        pegar(st, est, &f)?;
        return Ok(true);
    }
    Ok(false)
}

fn fechar(st: &mut crate::asm_ui::AsmState, est: Estado) {
    st.sel = est.ultima.filter(|id| st.asm.node(*id).is_some()).or(est.pai);
}

/// Esc com o comando aberto.
pub fn teclas(app: &mut App, ctx: &egui::Context) {
    if !app.asm.as_ref().is_some_and(|s| s.inserir.is_some()) || ctx.egui_wants_keyboard_input() {
        return;
    }
    if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
        let aviso = esc(app.asm.as_mut().expect("montagem aberta"));
        if let Some(e) = aviso {
            app.toast(e, true);
        }
    }
}

fn esc(st: &mut crate::asm_ui::AsmState) -> Option<String> {
    let mut est = st.inserir.take()?;
    let (segue, erro) = match encerrar_mao(st, &mut est) {
        Ok(s) => (s, None),
        Err(e) => (!est.fila.is_empty(), Some(e)),
    };
    if segue {
        st.inserir = Some(est);
    } else {
        fechar(st, est);
    }
    erro
}

/// Com o comando aberto o clique na vista é da colocação: nada de seleção por baixo.
pub fn apos_viewport(app: &mut App) -> bool {
    if !app.asm.as_ref().is_some_and(|s| s.inserir.is_some()) {
        return false;
    }
    app.picks.clear();
    true
}

/// Quadrinho de uma peça, com a miniatura girando devagar ao passar o mouse.
fn azulejo(ui: &mut egui::Ui, app_prev: &crate::examples::PreviewCache, arquivo: &Path, escolhido: bool) -> egui::Response {
    let previa = tem_extensao(arquivo, "fkp").then(|| {
        let f = arquivo.to_path_buf();
        app_prev.get(&format!("arq:{}", arquivo.display()), move || std::fs::read_to_string(&f).ok().and_then(|s| fenkra_core::Document::from_json(&s).ok()))
    });
    azulejo_com(ui, previa, nome_do(arquivo), &arquivo.to_string_lossy(), escolhido)
}

/// Desenha o quadrinho. `previa`: `None` não é peça; `Some(None)` a miniatura ainda não chegou.
fn azulejo_com(ui: &mut egui::Ui, previa: Option<Option<Arc<crate::examples::Preview>>>, nome: String, dica: &str, escolhido: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(LADO, LADO + 20.), Sense::click());
    // B321: anel de foco quando o Tab chega aqui
    let resp = crate::teclado::anel(ui, resp);
    let v = ui.visuals().clone();
    let quadro = Rect::from_min_size(rect.min, egui::vec2(LADO, LADO));
    let fundo = if resp.hovered() || escolhido { v.widgets.hovered.weak_bg_fill } else { v.extreme_bg_color };
    let painter = ui.painter_at(rect);
    painter.rect_filled(quadro, 4., fundo);
    if escolhido {
        painter.rect_stroke(quadro, 4., Stroke::new(1.5, crate::theme::accent(&v)), egui::StrokeKind::Inside);
    }
    if let Some(pv) = previa {
        match pv {
            Some(p) => {
                let giro = if resp.hovered() {
                    ui.ctx().request_repaint();
                    ui.input(|i| i.time) as f32 * 0.8
                } else {
                    0.6
                };
                crate::examples::paint_preview(&painter, quadro.shrink(4.), &p, giro);
            }
            None => {
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(250));
                painter.text(quadro.center(), egui::Align2::CENTER_CENTER, "…", egui::FontId::proportional(18.), v.weak_text_color());
            }
        }
    } else {
        painter.text(quadro.center(), egui::Align2::CENTER_CENTER, ".fka", egui::FontId::proportional(15.), v.weak_text_color());
    }
    let curto: String = if nome.chars().count() > 14 { nome.chars().take(13).chain(['…']).collect() } else { nome };
    painter.text(egui::pos2(rect.center().x, rect.bottom() - 3.), egui::Align2::CENTER_BOTTOM, curto, egui::FontId::proportional(11.), v.text_color());
    resp.on_hover_text(dica.to_string())
}

enum Acao {
    Pegar(PathBuf),
    PegarAberta,
    Procurar,
    Fechar,
}

fn grade(ui: &mut egui::Ui, previas: &crate::examples::PreviewCache, lista: &[PathBuf], na_mao: Option<&str>, acao: &mut Option<Acao>) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(6., 6.);
        for f in lista {
            let escolhido = na_mao.is_some_and(|n| n == nome_do(f));
            if azulejo(ui, previas, f, escolhido).clicked() {
                *acao = Some(Acao::Pegar(f.clone()));
            }
        }
    });
}

/// Painel do comando, aberto só enquanto ele dura.
pub fn painel(app: &mut App, ctx: &egui::Context, area: Rect) {
    if app.settings.bench != crate::settings::Bench::Montagem {
        return;
    }
    let Some(st) = app.asm.as_mut() else { return };
    let Some(mut est) = st.inserir.take() else { return };
    let titulo = if est.sub { tr("Inserir subconjunto") } else { tr("Inserir peça") };
    let cfg = Painel { nome: PAINEL.into(), titulo: titulo.to_string(), padrao: paineis::ao_lado_da_arvore(area), largura: 320., redimensiona: true };
    let posicao = app.settings.painel(PAINEL, Encaixe::Esquerda);
    let (mut recentes, mut pasta) = listar(&app.settings.recent_files, st.dir().as_deref(), st.file.as_deref(), est.sub, &est.filtro);
    let primeira = insercao::fixa_na_origem(&st.asm);
    let perfil = est.perfil.get_or_insert_with(|| app.project.as_ref().map_or_else(perfil_sem_projeto, |p| p.perfil_qualidade())).clone();
    let previa = est.mao.as_ref().map(|m| previa_de(st, &perfil, m));
    let qualidade = est.mao.as_ref().and_then(|m| relatorio_da_mao(app.project.as_ref(), &perfil, m));
    est.bloqueada = qualidade.as_ref().filter(|r| r.bloqueia()).map(|r| r.resumo_bloqueios());
    let mut acao = None;
    let previas = &app.previews;
    // a peça aberta entra sem precisar estar nos recentes nem na pasta da montagem (B362)
    let filtro = est.filtro.trim().to_lowercase();
    let nome_aberta = app.open.doc.name.clone();
    let tem_aberta = !est.sub && !app.open.doc.features.is_empty() && (filtro.is_empty() || nome_aberta.to_lowercase().contains(&filtro));
    let arquivo_aberta = caminho_aberto(app);
    let por_salvar = aberta_por_salvar(app);
    if tem_aberta {
        recentes.retain(|p| Some(p) != arquivo_aberta.as_ref());
        pasta.retain(|p| Some(p) != arquivo_aberta.as_ref());
    }
    let chave_aberta = format!("aberta:{nome_aberta}:{}:{por_salvar}", app.open.doc.features.len());
    let mut doc_aberta = (tem_aberta && est.chave_aberta != chave_aberta).then(|| app.open.doc.clone());
    if tem_aberta {
        est.chave_aberta = chave_aberta.clone();
    }
    let saida = paineis::mostrar(ctx, area, &cfg, posicao, |ui| {
        if crate::app::done_bar(ui) {
            acao = Some(Acao::Fechar);
        }
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut est.filtro).hint_text(tr("Filtrar por nome")).desired_width(150.));
            if ui.button(tr("Procurar no disco…")).on_hover_text(tr("Abre o diálogo do sistema; aceita vários arquivos, colocados um depois do outro.")).clicked() {
                acao = Some(Acao::Procurar);
            }
        });
        let nome_mao = est.mao.as_ref().map(|m| m.escolha.nome.clone());
        match &nome_mao {
            Some(n) => {
                ui.label(RichText::new(format!("{}: {n} · {} {}", tr("Na mão"), est.colocadas, tr("colocada(s)"))).strong());
                ui.label(RichText::new(tr("Clique na vista para colocar; cada clique cria uma cópia. Esc encerra.")).small().weak());
            }
            None => {
                ui.label(RichText::new(tr("Clique em uma miniatura para pegar a peça.")).small().weak());
            }
        }
        if let Some(r) = qualidade.as_ref().filter(|r| !r.dispensado && !r.violacoes.is_empty()) {
            for v in &r.violacoes {
                let cor = match v.efetiva() {
                    qualidade_modelo::Gravidade::Bloqueia => ui.visuals().error_fg_color,
                    qualidade_modelo::Gravidade::Avisa => crate::theme::st_warn(),
                    qualidade_modelo::Gravidade::Informa => ui.visuals().weak_text_color(),
                };
                let mut linha = format!("{} {} ({:.0} / {:.0})", tr(v.efetiva().rotulo()), tr(v.regra.rotulo()), v.medido, v.limite);
                if let Some(e) = &v.excecao {
                    linha += &format!(" · {} {} ({})", tr("exceção de"), e.nome, e.motivo);
                }
                ui.label(RichText::new(linha).small().color(cor));
            }
            if est.bloqueada.is_some() {
                ui.label(RichText::new(tr("O perfil de qualidade do projeto barra esta peça. Registre uma exceção com nome e motivo no painel do projeto para inseri-la.")).small().color(ui.visuals().error_fg_color));
            }
        }
        if let Some(p) = previa.as_ref().filter(|p| p.oferecer_simplificada) {
            ui.label(RichText::new(format!("{}: {:.0} {} ({} {:.0})", tr("A montagem passa do limite de triângulos do perfil"), p.soma_triangulos, tr("triângulos"), tr("limite"), p.limite)).small().color(crate::theme::st_warn()));
            let rotulo = if est.simplificada { tr("Inserir versão completa") } else { tr("Inserir versão simplificada") };
            if ui.button(rotulo).on_hover_text(tr("As próximas cópias entram com a malha mais grossa. A peça é a mesma: muda só o desenho.")).clicked() {
                est.simplificada = !est.simplificada;
            }
        }
        if primeira {
            ui.label(RichText::new(tr("A primeira peça ancora a montagem: fica fixa na origem.")).small().color(crate::theme::st_warn()));
        }
        ui.separator();
        egui::ScrollArea::vertical().max_height(420.).auto_shrink([false, true]).show(ui, |ui| {
            if tem_aberta {
                ui.label(RichText::new(if por_salvar { tr("Peça aberta: salva ao pegar") } else { tr("Peça aberta") }).small().strong());
                let doc = doc_aberta.take();
                let previa = Some(previas.get(&chave_aberta, move || doc));
                let dica = if por_salvar { tr("Salva a peça e a põe na mão.").to_string() } else { arquivo_aberta.as_ref().map(|f| f.display().to_string()).unwrap_or_default() };
                let escolhido = nome_mao.as_deref().is_some_and(|n| arquivo_aberta.as_deref().is_some_and(|f| nome_do(f) == n));
                if azulejo_com(ui, previa, nome_aberta.clone(), &dica, escolhido).clicked() {
                    acao = Some(Acao::PegarAberta);
                }
                ui.add_space(6.);
            }
            if !tem_aberta && recentes.is_empty() && pasta.is_empty() {
                ui.label(RichText::new(tr("Nenhuma peça recente nem nesta pasta: use Procurar no disco.")).weak());
            }
            if !recentes.is_empty() {
                ui.label(RichText::new(tr("Recentes")).small().strong());
                grade(ui, previas, &recentes, nome_mao.as_deref(), &mut acao);
            }
            if !pasta.is_empty() {
                ui.add_space(6.);
                ui.label(RichText::new(tr("Nesta pasta")).small().strong());
                grade(ui, previas, &pasta, nome_mao.as_deref(), &mut acao);
            }
        });
    });
    if let Some(p) = saida.movido {
        app.settings.guardar_painel(PAINEL, p);
    }
    let mut aviso = None;
    if matches!(acao, Some(Acao::PegarAberta)) {
        est.fila.clear();
        aviso = pegar_aberta(app, &mut est).err();
    }
    let st = app.asm.as_mut().expect("montagem aberta");
    match acao {
        Some(Acao::Fechar) => {
            fechar(st, est);
            return;
        }
        Some(Acao::Pegar(f)) => {
            est.fila.clear();
            aviso = pegar(st, &mut est, &f).err();
            if aviso.is_none() {
                app.settings.add_recent_file(&f);
            }
        }
        Some(Acao::Procurar) => {
            let (filtro, e) = if est.sub { ("Montagem", "fka") } else { ("Peça", "fkp") };
            if let Some(mut files) = rfd::FileDialog::new().add_filter(filtro, &[e]).pick_files().filter(|f| !f.is_empty()) {
                let f = files.remove(0);
                est.fila = files;
                aviso = pegar(st, &mut est, &f).err();
                if aviso.is_none() {
                    app.settings.add_recent_file(&f);
                }
            }
        }
        Some(Acao::PegarAberta) | None => {}
    }
    app.asm.as_mut().expect("montagem aberta").inserir = Some(est);
    if let Some(a) = aviso {
        app.toast(a, true);
    }
}

fn v3(p: V3) -> Vec3 {
    Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32)
}

fn d3(v: Vec3) -> V3 {
    [v.x as f64, v.y as f64, v.z as f64]
}

/// Origem que a peça teria com o cursor em `pos`.
fn origem_no_cursor(rect: Rect, cam: &Camera, pos: Pos2, apoio: V3) -> V3 {
    let (o, d) = cam.ray(rect, pos);
    insercao::origem_para(apoio, insercao::ponto_do_cursor(d3(o), d3(d), d3(cam.target), cam.dist as f64))
}

/// Selo sobre o ponto: uma etiqueta com fundo do tema, legível sobre qualquer peça.
fn selo(painter: &egui::Painter, em: Pos2, texto: &str, v: &egui::Visuals) {
    let cor = crate::theme::accent(v);
    let galeria = painter.layout_no_wrap(texto.to_string(), egui::FontId::proportional(12.), cor);
    let caixa = Rect::from_min_size(em + egui::vec2(14., -30.), galeria.size()).expand2(egui::vec2(7., 3.));
    painter.rect_filled(caixa, 4., v.window_fill);
    painter.rect_stroke(caixa, 4., Stroke::new(1., cor), egui::StrokeKind::Inside);
    painter.galley(caixa.min + egui::vec2(7., 3.), galeria, cor);
}

/// Um quadro do comando na vista: desenha o fantasma e coloca no clique. Devolve o componente criado.
fn quadro(st: &mut crate::asm_ui::AsmState, est: &mut Estado, ui: &mut egui::Ui, rect: Rect, cam: &Camera) -> Option<Id> {
    let Some(m) = est.mao.as_ref() else { return None };
    let ctx = ui.ctx().clone();
    let v = ctx.global_style().visuals.clone();
    let fixa = insercao::fixa_na_origem(&st.asm);
    let cursor = match est.cursor_demo {
        Some(f) => Some(rect.min + egui::vec2(rect.width() * f[0], rect.height() * f[1])),
        None => ctx.input(|i| i.pointer.latest_pos()).filter(|p| rect.contains(*p) && ctx.layer_id_at(*p).is_some_and(|l| l.order == egui::Order::Background)),
    };
    let painter = ui.painter_at(rect);
    let cor = crate::theme::accent(&v);
    let origem = match (fixa, cursor) {
        (true, _) => Some([0.; 3]),
        (false, Some(p)) => Some(origem_no_cursor(rect, cam, p, m.apoio)),
        (false, None) => None,
    };
    if let Some(o) = origem {
        biblioteca_ui::fantasma(&painter, rect, cam, &m.malha, Placement { origin: o, ..Default::default() }, cor, 0.42);
        if fixa {
            if let Some(em) = cam.project(rect, v3(o)) {
                painter.circle_filled(em, 4., cor);
                selo(&painter, em, tr("fixa na origem"), &v);
            }
        }
        ctx.request_repaint();
    }
    let clicou = ctx.input(|i| i.pointer.primary_clicked());
    if clicou && cursor.is_some() {
        est.recusada = est.bloqueada.is_some();
        return colocar_copia(st, est, origem?);
    }
    None
}

/// Fantasma e colocação, chamados com a viewport.
pub fn overlay(app: &mut App, ui: &mut egui::Ui, rect: Rect, cam: &Camera) {
    if app.settings.bench != crate::settings::Bench::Montagem {
        return;
    }
    let Some(st) = app.asm.as_mut() else { return };
    let Some(mut est) = st.inserir.take() else { return };
    let vazia = insercao::fixa_na_origem(&st.asm);
    let novo = quadro(st, &mut est, ui, rect, cam);
    let recusa = std::mem::take(&mut est.recusada).then(|| est.bloqueada.clone()).flatten();
    st.inserir = Some(est);
    if let Some(motivo) = recusa {
        app.toast(format!("{}: {motivo}", tr("Esta peça não entra na montagem")), true);
    }
    if novo.is_some() && vazia {
        app.fit_now();
    }
    // a peça colocada entra na cena só depois do `refresh`: pede o próximo quadro
    if novo.is_some() {
        ui.ctx().request_repaint();
    }
}

/// Peça que tem corpo e um comando em erro: o comando final cita um sketch que não existe.
fn peca_com_erro() -> fenkra_core::Document {
    use fenkra_core::{BoolOp, Extent, Extrude, FeatureKind};
    let mut d = fenkra_solidos::example_document();
    d.add_feature(FeatureKind::Extrude(Extrude { sketch: Id(999_999_999), op: BoolOp::Add, extent: Extent::Blind("5".into()), reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
    d
}

/// `FENKRA_DEMO_ASM=inserir|inserir_vazia|inserir_limite`: peça na mão, com o cursor parado para a
/// captura. Em `inserir_limite` o perfil tem teto baixo, para o aviso aparecer.
pub fn demo(st: &mut crate::asm_ui::AsmState, nome: &str) {
    let vazia = nome == "inserir_vazia";
    let Some(dir) = st.dir() else { return };
    for (i, e) in crate::examples::all().into_iter().take(5).enumerate() {
        let _ = std::fs::write(dir.join(format!("exemplo_{i}.fkp")), e.doc.to_json());
    }
    if vazia {
        st.asm.nodes.retain(|n| matches!(n.kind, fenkra_montagem::NodeKind::Group));
        st.dirty = true;
    }
    abrir(st, false);
    // `inserir_mouse`: só o painel; os cliques vêm de `FENKRA_DEMO_PAINEL`, pelo caminho do mouse de
    // verdade, sem o cursor fixo que pula o filtro de camada
    if nome == "inserir_mouse" {
        let _ = std::fs::write(dir.join("barrada.fkp"), peca_com_erro().to_json());
        return;
    }
    let Some(mut est) = st.inserir.take() else { return };
    let _ = pegar(st, &mut est, &dir.join("placa.fkp"));
    est.cursor_demo = Some([0.72, 0.62]);
    if nome == "inserir_limite" {
        est.perfil = Some(Perfil { montagem_triangulos: 1000., ..Perfil::padrao() });
    }
    st.inserir = Some(est);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asm_ui::AsmState;
    use egui::{pos2, vec2};
    use fenkra_montagem::NodeKind;

    #[test]
    fn passar_do_teto_oferece_a_versao_simplificada_e_a_copia_nasce_com_ela() {
        let (mut st, d) = montagem("teto");
        let mut est = Estado::novo(false, None);
        pegar(&mut st, &mut est, &d.join("placa.fkp")).unwrap();
        let mao = est.mao.as_ref().unwrap();
        assert!(mao.triangulos > 0);
        let folgado = Perfil { montagem_triangulos: mao.triangulos as f64 * 10., ..Perfil::padrao() };
        let apertado = Perfil { montagem_triangulos: mao.triangulos as f64 / 2., ..Perfil::padrao() };
        assert!(!previa_de(&st, &folgado, mao).oferecer_simplificada);
        assert!(previa_de(&st, &apertado, mao).oferecer_simplificada);
        let completa = colocar_copia(&mut st, &mut est, [0.; 3]).unwrap();
        est.simplificada = true;
        let simples = colocar_copia(&mut st, &mut est, [200., 0., 0.]).unwrap();
        assert!(!st.asm.node(completa).unwrap().simplificada);
        assert!(st.asm.node(simples).unwrap().simplificada);
    }

    /// B305: o perfil de qualidade do projeto roda "ao inserir na montagem". Peça acima do limite
    /// bloqueante é barrada; a exceção com nome e motivo, gravada no diário, a libera.
    #[test]
    fn perfil_do_projeto_barra_a_peca_pesada_e_a_excecao_registrada_libera() {
        use fenkra_project::qualidade_modelo::{Gravidade, Regra};
        let (mut st, d) = montagem("perfil_barra");
        let mut p = fenkra_project::Project::create(&d.join("proj"), "P").unwrap();
        let id = p.add_document(fenkra_solidos::example_document()).unwrap();
        // a peça da montagem é o documento do projeto (mesmo uid), como numa pasta de trabalho real
        std::fs::write(d.join("placa.fkp"), p.load(id).unwrap().to_json()).unwrap();
        let mut est = Estado::novo(false, None);
        pegar(&mut st, &mut est, &d.join("placa.fkp")).unwrap();
        assert!(est.mao.as_ref().unwrap().medidas.is_some_and(|m| m.triangulos > 0));
        let mut rig = Perfil::rigoroso();
        let l = rig.regras.iter_mut().find(|l| l.regra == Regra::Triangulos).unwrap();
        l.limite = 1.;
        l.gravidade = Gravidade::Bloqueia;
        p.definir_perfil_qualidade(rig.clone()).unwrap();
        let r = relatorio_da_mao(Some(&p), &rig, est.mao.as_ref().unwrap()).unwrap();
        assert!(r.bloqueia(), "a peça passa de 1 triângulo");
        est.bloqueada = Some(r.resumo_bloqueios());
        assert!(colocar_copia(&mut st, &mut est, [0.; 3]).is_none(), "barrada pelo perfil");
        assert_eq!(pecas(&st), 0);
        // exceção com nome e motivo: libera e continua visível no relatório
        p.conceder_excecao_qualidade(id, Regra::Triangulos, "Ana", "peça de catálogo").unwrap();
        let r = relatorio_da_mao(Some(&p), &rig, est.mao.as_ref().unwrap()).unwrap();
        assert!(!r.bloqueia());
        assert_eq!(r.violacoes[0].excecao.as_ref().map(|e| e.nome.as_str()), Some("Ana"));
        est.bloqueada = None;
        assert!(colocar_copia(&mut st, &mut est, [0.; 3]).is_some());
        assert_eq!(pecas(&st), 1);
        // fora do projeto não há exceção: só o perfil decide
        let sem = relatorio_da_mao(None, &rig, est.mao.as_ref().unwrap()).unwrap();
        assert!(sem.bloqueia());
    }

    /// B362: sem projeto não há onde registrar exceção, então peça com comando em erro entra com aviso.
    #[test]
    fn peca_com_comando_em_erro_entra_fora_do_projeto_e_o_aviso_fica() {
        let (mut st, d) = montagem("erro_sem_projeto");
        std::fs::write(d.join("erro.fkp"), peca_com_erro().to_json()).unwrap();
        let mut est = Estado::novo(false, None);
        pegar(&mut st, &mut est, &d.join("erro.fkp")).unwrap();
        let mao = est.mao.as_ref().unwrap();
        // o padrão do projeto barra essa peça: é o que deixava o clique mudo sem projeto
        assert!(relatorio_da_mao(None, &Perfil::padrao(), mao).unwrap().bloqueia());
        let r = relatorio_da_mao(None, &perfil_sem_projeto(), mao).unwrap();
        assert!(!r.bloqueia() && !r.violacoes.is_empty(), "avisa sem barrar");
        assert!(colocar_copia(&mut st, &mut est, [0.; 3]).is_some());
        assert_eq!(pecas(&st), 1);
    }

    /// Com projeto o perfil segue barrando, mas o clique recusado deixa rastro para o aviso.
    #[test]
    fn clique_barrado_pelo_perfil_nao_fica_mudo() {
        let (mut st, d) = montagem("clique_barrado");
        let mut est = Estado::novo(false, None);
        pegar(&mut st, &mut est, &d.join("placa.fkp")).unwrap();
        est.bloqueada = Some("Referências em erro".into());
        st.inserir = Some(est);
        let ctx = egui::Context::default();
        clicar(&ctx, &mut st, pos2(500., 300.));
        let est = st.inserir.as_ref().unwrap();
        assert!(est.recusada, "quem desenha recebe o pedido de avisar");
        assert_eq!(pecas(&st), 0);
    }

    fn montagem(nome: &str) -> (AsmState, PathBuf) {
        let d = std::env::temp_dir().join(format!("fenkra_inserir_{nome}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).expect("pasta");
        std::fs::write(d.join("placa.fkp"), fenkra_solidos::example_document().to_json()).expect("peça");
        (AsmState::new(Assembly::new("t"), Some(d.join("conjunto.fka"))), d)
    }

    fn pecas(st: &AsmState) -> usize {
        st.asm.nodes.iter().filter(|n| !matches!(n.kind, NodeKind::Group)).count()
    }

    /// Passa um quadro da vista com a câmera padrão, recebendo os eventos do mouse.
    fn passo(ctx: &egui::Context, st: &mut AsmState, evs: Vec<egui::Event>) {
        let tela = Rect::from_min_size(pos2(0., 0.), vec2(1000., 700.));
        let raw = egui::RawInput { screen_rect: Some(tela), events: evs, ..Default::default() };
        let mut saida = ctx.run_ui(raw, |ui| {
            let mut est = st.inserir.take().expect("comando aberto");
            quadro(st, &mut est, ui, tela, &Camera::default());
            st.inserir = Some(est);
        });
        saida.textures_delta.clear();
    }

    fn clicar(ctx: &egui::Context, st: &mut AsmState, p: Pos2) {
        let botao = |pressed| egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() };
        passo(ctx, st, vec![egui::Event::PointerMoved(p)]);
        passo(ctx, st, vec![botao(true)]);
        passo(ctx, st, vec![botao(false)]);
        passo(ctx, st, vec![]);
    }

    #[test]
    fn tres_cliques_tres_copias_e_desfazer_tira_uma_por_vez() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (mut st, d) = montagem("tres");
        abrir(&mut st, false);
        let mut est = st.inserir.take().unwrap();
        pegar(&mut st, &mut est, &d.join("placa.fkp")).expect("pegar");
        st.inserir = Some(est);
        assert_eq!(pecas(&st), 0, "pegar não mexe no documento");
        let ctx = egui::Context::default();
        passo(&ctx, &mut st, vec![]);
        for p in [pos2(300., 300.), pos2(500., 350.), pos2(700., 400.)] {
            clicar(&ctx, &mut st, p);
        }
        assert_eq!(pecas(&st), 3);
        let ids: Vec<Id> = st.asm.nodes.iter().map(|n| n.id).collect();
        let (a, b, c) = (st.asm.node(ids[0]).unwrap(), st.asm.node(ids[1]).unwrap(), st.asm.node(ids[2]).unwrap());
        assert!(a.fixed && a.placement.origin == [0.; 3], "a primeira fica fixa na origem");
        assert!(!b.fixed && !c.fixed);
        assert!((b.placement.origin[0] - c.placement.origin[0]).abs() + (b.placement.origin[1] - c.placement.origin[1]).abs() > 1., "cada clique cai num ponto");
        // a peça calcula: status, volume e caixa da composição
        st.refresh();
        assert!(st.errors.is_empty(), "{:?}", st.errors);
        assert_eq!(st.view.as_ref().unwrap().bodies.len(), 3);
        assert_eq!(st.undo.len(), 3, "um desfazer por cópia");
        for falta in [2, 1, 0] {
            let antes = st.undo.pop().unwrap();
            st.asm = antes;
            assert_eq!(pecas(&st), falta);
        }
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn cancelar_com_a_peca_na_mao_nao_deixa_nada_na_arvore() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (mut st, d) = montagem("cancelar");
        abrir(&mut st, false);
        let mut est = st.inserir.take().unwrap();
        pegar(&mut st, &mut est, &d.join("placa.fkp")).expect("pegar");
        st.inserir = Some(est);
        let ctx = egui::Context::default();
        passo(&ctx, &mut st, vec![egui::Event::PointerMoved(pos2(300., 300.))]);
        assert!(esc(&mut st).is_none());
        assert!(st.inserir.is_none(), "Esc encerra");
        assert_eq!(pecas(&st), 0);
        assert!(st.undo.is_empty() && !st.modified, "nada a desfazer");
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn fila_do_dialogo_passa_para_o_proximo_arquivo_no_esc() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (mut st, d) = montagem("fila");
        std::fs::copy(d.join("placa.fkp"), d.join("outra.fkp")).expect("cópia");
        abrir(&mut st, false);
        let mut est = st.inserir.take().unwrap();
        pegar(&mut st, &mut est, &d.join("placa.fkp")).expect("pegar");
        est.fila = vec![d.join("outra.fkp")];
        st.inserir = Some(est);
        esc(&mut st);
        assert_eq!(st.inserir.as_ref().unwrap().mao.as_ref().unwrap().escolha.nome, "outra");
        esc(&mut st);
        assert!(st.inserir.is_none());
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn lista_nao_repete_recente_nem_a_propria_montagem_e_filtra() {
        let (_, d) = montagem("lista");
        std::fs::write(d.join("eixo.fkp"), "{}").expect("peça");
        std::fs::write(d.join("conjunto.fka"), "{}").expect("montagem");
        let recente = d.join("placa.fkp").to_string_lossy().to_string();
        let sumiu = d.join("apagada.fkp").to_string_lossy().to_string();
        let (r, p) = listar(&[recente.clone(), sumiu], Some(&d), None, false, "");
        assert_eq!(r, vec![d.join("placa.fkp")], "recente apagado sai");
        assert_eq!(p, vec![d.join("eixo.fkp")], "o recente não repete na pasta");
        let (r, p) = listar(&[recente], Some(&d), None, false, "EIX");
        assert!(r.is_empty());
        assert_eq!(p.len(), 1);
        let atual = d.join("conjunto.fka");
        let (_, p) = listar(&[], Some(&d), Some(&atual), true, "");
        assert!(p.is_empty(), "a montagem não se oferece como subconjunto dela mesma");
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn malha_pesada_fica_no_limite_sem_perder_a_caixa() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let mut m = fenkra_kernel::Mesh::default();
        let n = MAX_TRIS * 3;
        for i in 0..n {
            m.positions.extend_from_slice(&[i as f32, 0., 0., i as f32, 1., 0., i as f32, 0., 1.]);
            let b = 3 * i as u32;
            m.indices.extend_from_slice(&[b, b + 1, b + 2]);
        }
        let g = malha_de(std::iter::once(&m)).expect("malha");
        assert!(g.tris.len() <= MAX_TRIS);
        assert!((g.centro.x - (n - 1) as f32 / 2.).abs() < 1., "a caixa olha todos os vértices");
    }
}
