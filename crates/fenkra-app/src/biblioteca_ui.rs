// SPDX-License-Identifier: LGPL-2.1-or-later
//! Navegador do catálogo de normas (B241) e inserção de item avulso (B242), mais o fantasma dos
//! candidatos do preenchimento de furos (B248).
//!
//! O painel `Biblioteca` é um painel livre e encaixável (como os de comando): busca, chips de filtro,
//! árvore Norma > Tipo, ficha da família com esquema das cotas e tabela de tamanhos, e uma prévia 3D
//! que gira com o arraste. Os dados e a lógica de busca ficam em `fenkra_solidos::catalogo`, sem
//! interface, e é lá que os testes de "toda família abre a ficha" moram.
//!
//! Para inserir, a pessoa arrasta uma linha da tabela para a viewport (ou aperta "Inserir em furo" e
//! clica). O fantasma translúcido segue o cursor e salta para o furo cujo diâmetro casa com o tamanho;
//! Tab inverte o sentido, a alça na ponta do parafuso escolhe o comprimento só entre os valores da
//! série, e Alt na hora de soltar abre a ficha em vez de inserir. Quem insere é
//! `fenkra_montagem::avulso`, com desfazer pelo mesmo passo da montagem.
//!
//! O desenho 3D do fantasma e da prévia sai pelo `Painter` do egui: a malha do item é pequena, fica em
//! cache e não precisa reconstruir a cena da viewport a cada quadro do cursor.

use crate::app::{fmt, App};
use crate::i18n::tr;
use crate::paineis::{self, Encaixe, Painel};
use crate::render::Camera;
use eframe::egui::{self, Color32, Pos2, Rect, RichText, Sense, Stroke};
use fenkra_montagem::avulso::{self, FuroAlvo, ModoInsercao, ParamAvulso, PosicaoLivre};
use fenkra_montagem::fixadores::{Candidato, EstadoFuro};
use fenkra_solidos::biblioteca::{self, Familia, TipoFixador};
use fenkra_solidos::catalogo::{self, Cabeca, Filtro, Preferencias, Sistema};
use glam::Vec3;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// Nome do painel nas preferências de posição.
pub const PAINEL: &str = "biblioteca";
/// Raio, em pixels, em que o fantasma salta para um furo.
const ALCANCE_PX: f32 = 56.;

// ---------------------------------------------------------------- malha do item

/// Triângulos e arestas de um item da biblioteca, no sistema local dele (eixo Z, apoio em z = 0).
pub struct Malha {
    pub tris: Vec<[Vec3; 3]>,
    /// normal de cada triângulo
    pub normais: Vec<Vec3>,
    pub segs: Vec<[Vec3; 2]>,
    pub centro: Vec3,
    pub raio: f32,
}

thread_local! {
    static MALHAS: RefCell<HashMap<String, Option<Rc<Malha>>>> = RefCell::new(HashMap::new());
}

/// Malha do item (comprimento só para parafusos). Gera uma vez e guarda; `None` se o kernel falhar.
pub fn malha_do_item(familia: &str, tamanho: &str, comprimento: Option<f64>) -> Option<Rc<Malha>> {
    let chave = format!("{familia}|{tamanho}|{}", comprimento.map(|l| format!("{l}")).unwrap_or_default());
    MALHAS.with(|m| {
        if let Some(r) = m.borrow().get(&chave) {
            return r.clone();
        }
        let r = gerar_malha(familia, tamanho, comprimento).map(Rc::new);
        m.borrow_mut().insert(chave, r.clone());
        r
    })
}

fn gerar_malha(familia: &str, tamanho: &str, comprimento: Option<f64>) -> Option<Malha> {
    let doc = fenkra_solidos::fixadores::fixador_documento(familia, tamanho, comprimento).ok()?;
    let r = fenkra_solidos::recompute(&doc);
    let v3 = |s: &[f32], i: u32| Vec3::new(s[3 * i as usize], s[3 * i as usize + 1], s[3 * i as usize + 2]);
    let (mut tris, mut normais, mut segs) = (vec![], vec![], vec![]);
    for b in &r.bodies {
        let m = &b.mesh;
        for t in m.indices.chunks_exact(3) {
            let (a, bb, c) = (v3(&m.positions, t[0]), v3(&m.positions, t[1]), v3(&m.positions, t[2]));
            let n = (bb - a).cross(c - a).normalize_or_zero();
            tris.push([a, bb, c]);
            normais.push(n);
        }
        for s in m.edges.chunks_exact(6) {
            segs.push([Vec3::new(s[0], s[1], s[2]), Vec3::new(s[3], s[4], s[5])]);
        }
    }
    if tris.is_empty() {
        return None;
    }
    let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    for t in &tris {
        for p in t {
            lo = lo.min(*p);
            hi = hi.max(*p);
        }
    }
    Some(Malha { tris, normais, segs, centro: (lo + hi) / 2., raio: ((hi - lo) / 2.).length().max(1e-3) })
}

/// Desenha a malha no `painter`. `xf` leva o ponto e a direção do sistema local para o espaço de
/// desenho, `olho` é a direção em que a câmera olha nesse espaço, `proj` leva o espaço à tela.
/// Faces de costas ficam de fora (os itens são sólidos fechados); o resto sai de trás para a frente.
#[allow(clippy::too_many_arguments)]
fn desenhar(painter: &egui::Painter, m: &Malha, ponto: &dyn Fn(Vec3) -> Vec3, dir: &dyn Fn(Vec3) -> Vec3, olho: Vec3, proj: &dyn Fn(Vec3) -> Option<Pos2>, cor: Color32, alfa: f32, linha: Option<Stroke>) {
    let luz = (-olho * 0.8 + Vec3::new(0.35, -0.3, 0.55).normalize() * 0.5).normalize();
    let mut ordem: Vec<(f32, usize)> = vec![];
    for (i, t) in m.tris.iter().enumerate() {
        let n = dir(m.normais[i]);
        if n.dot(olho) >= 0. {
            continue;
        }
        let c = (ponto(t[0]) + ponto(t[1]) + ponto(t[2])) / 3.;
        ordem.push((c.dot(olho), i));
    }
    ordem.sort_by(|a, b| b.0.total_cmp(&a.0));
    let mut malha = egui::Mesh::default();
    for (_, i) in ordem {
        let t = &m.tris[i];
        let (Some(a), Some(b), Some(c)) = (proj(ponto(t[0])), proj(ponto(t[1])), proj(ponto(t[2]))) else { continue };
        let n = dir(m.normais[i]);
        let lum = 0.5 + 0.5 * n.dot(luz).max(0.);
        // escurece para o preto sem mexer na opacidade, e só depois aplica a transparência
        let col = cor.lerp_to_gamma(Color32::BLACK, 1. - lum.clamp(0.35, 1.)).gamma_multiply(alfa.clamp(0., 1.));
        let base = malha.vertices.len() as u32;
        malha.colored_vertex(a, col);
        malha.colored_vertex(b, col);
        malha.colored_vertex(c, col);
        malha.add_triangle(base, base + 1, base + 2);
    }
    painter.add(egui::Shape::mesh(malha));
    if let Some(st) = linha {
        for s in &m.segs {
            // só as arestas voltadas para quem olha: o ponto médio tem de estar do lado da frente do centro
            let (Some(a), Some(b)) = (proj(ponto(s[0])), proj(ponto(s[1]))) else { continue };
            painter.line_segment([a, b], st);
        }
    }
}

fn v3(p: [f64; 3]) -> Vec3 {
    Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32)
}

/// Pose (origem e eixos locais) como função ponto/direção do local para o mundo.
fn no_mundo(pl: fenkra_montagem::Placement) -> (impl Fn(Vec3) -> Vec3, impl Fn(Vec3) -> Vec3) {
    let ponto = move |p: Vec3| v3(pl.point([p.x as f64, p.y as f64, p.z as f64]));
    let dir = move |p: Vec3| v3(pl.dir([p.x as f64, p.y as f64, p.z as f64]));
    (ponto, dir)
}

/// Fantasma translúcido de um item na viewport.
#[allow(clippy::too_many_arguments)]
pub(crate) fn fantasma(painter: &egui::Painter, rect: Rect, cam: &Camera, m: &Malha, pl: fenkra_montagem::Placement, cor: Color32, alfa: f32) {
    let (ponto, dir) = no_mundo(pl);
    let olho = cam.ray(rect, rect.center()).1;
    let proj = |p: Vec3| cam.project(rect, p);
    let borda = Stroke::new(1., cor.gamma_multiply(0.9));
    desenhar(painter, m, &ponto, &dir, olho, &proj, cor, alfa, Some(borda));
}

// ---------------------------------------------------------------- estado

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Aba {
    #[default]
    Catalogo,
    Favoritos,
    Recentes,
}

/// Onde o item entra (B374): num furo (restrito), num ponto da superfície de uma peça ou solto no
/// espaço. Fora do furo o item vira peça comum da montagem, sem restrição, para posicionar depois.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum Alvo {
    #[default]
    Furo,
    Ponto,
    Solto,
}

impl Alvo {
    pub const TODOS: [Alvo; 3] = [Alvo::Furo, Alvo::Ponto, Alvo::Solto];

    pub fn rotulo(self) -> &'static str {
        match self {
            Alvo::Furo => "Em furo",
            Alvo::Ponto => "Em ponto",
            Alvo::Solto => "Solto",
        }
    }

    pub fn dica(self) -> &'static str {
        match self {
            Alvo::Furo => "Clique e depois num furo da viewport. O fantasma salta para o furo que casa; Tab inverte o sentido; Alt ao clicar abre a ficha; Esc sai. O item entra restrito, sem graus de liberdade.",
            Alvo::Ponto => "Clique e depois numa superfície da viewport. O item pousa no ponto apontado, com o eixo na normal da face, como peça livre para posicionar depois. Alt ao clicar abre a ficha; Esc sai.",
            Alvo::Solto => "Clique e depois num lugar da viewport. O item cai solto no espaço, como peça livre para posicionar depois. Alt ao clicar abre a ficha; Esc sai.",
        }
    }

    fn ajuda(self) -> &'static str {
        match self {
            Alvo::Furo => "Tab inverte o sentido · Alt abre a ficha · Esc sai",
            Alvo::Ponto => "Aponte uma superfície e clique · Alt abre a ficha · Esc sai",
            Alvo::Solto => "Clique onde o item deve cair · Alt abre a ficha · Esc sai",
        }
    }
}

/// Inserção em andamento: o item escolhido segue o cursor até um furo.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Insercao {
    /// o botão do mouse segue apertado desde a tabela: soltar decide
    pub arrastando: bool,
}

pub struct Estado {
    pub filtro: Filtro,
    pub aba: Aba,
    pub familia: Option<String>,
    pub tamanho: Option<String>,
    /// comprimento pedido; `None` deixa o furo escolher (o menor da série que atravessa)
    pub comprimento: Option<f64>,
    /// rebite: espessura da pilha pedida (mm); o L sai dela (pega + 1,5·d, na série)
    pub pega: Option<f64>,
    pub prefs: Preferencias,
    prefs_lidas: bool,
    pub yaw: f32,
    pub pitch: f32,
    pub modo: ModoInsercao,
    pub inserindo: Option<Insercao>,
    /// onde o próximo clique (ou o arrasto da tabela) põe o item
    pub alvo: Alvo,
    /// o botão "Adicionar à montagem" foi apertado: a viewport põe o item solto no centro da vista
    pub adicionar_agora: bool,
    /// posição fixa do cursor, em fração da vista: captura de tela sem mouse (`FENKRA_DEMO_ASM`)
    pub cursor_demo: Option<[f32; 2]>,
    /// mantém "Fontes e licença" aberta: captura de tela sem mouse (`FENKRA_DEMO_ASM=biblioteca_fontes`)
    pub fontes_abertas_demo: bool,
    pub inverter: bool,
    /// furos do mundo, calculados ao armar a inserção e refeitos depois de inserir
    furos: Option<Vec<FuroAlvo>>,
    /// a ficha pediu foco (Alt ao soltar): o painel rola até ela
    pub foco_ficha: bool,
}

impl Default for Estado {
    fn default() -> Self {
        Estado {
            filtro: Filtro::default(),
            aba: Aba::Catalogo,
            familia: None,
            tamanho: None,
            comprimento: None,
            pega: None,
            prefs: Preferencias::default(),
            prefs_lidas: false,
            yaw: -0.6,
            pitch: 0.5,
            modo: ModoInsercao::Padrao,
            inserindo: None,
            alvo: Alvo::Furo,
            adicionar_agora: false,
            cursor_demo: None,
            fontes_abertas_demo: false,
            inverter: false,
            furos: None,
            foco_ficha: false,
        }
    }
}

impl Estado {
    /// Escolhe uma família (e um tamanho, ou o primeiro) e registra no histórico.
    pub fn escolher(&mut self, familia: &str, tamanho: Option<&str>) {
        let Some(f) = biblioteca::familia(familia) else { return };
        let t = tamanho.and_then(|t| f.tamanho(t)).or(f.tamanhos.first()).map(|t| t.designacao.clone());
        if self.familia.as_deref() != Some(familia) || self.tamanho != t {
            self.comprimento = None;
        }
        self.familia = Some(familia.to_string());
        self.tamanho = t.clone();
        self.furos = None;
        if let Some(t) = t {
            self.prefs.registrar(familia, &t);
            salvar_prefs(&self.prefs);
        }
    }

    pub fn escolhida(&self) -> Option<(&'static Familia, &'static biblioteca::Tamanho)> {
        let f = biblioteca::familia(self.familia.as_deref()?)?;
        Some((f, f.tamanho(self.tamanho.as_deref()?)?))
    }
}

fn arquivo_prefs() -> Option<std::path::PathBuf> {
    std::env::var_os("APPDATA").map(|a| std::path::PathBuf::from(a).join("Fenkra").join("biblioteca.json"))
}

fn carregar_prefs() -> Preferencias {
    let mut p = arquivo_prefs().and_then(|a| std::fs::read_to_string(a).ok()).map(|t| Preferencias::de_json(&t)).unwrap_or_default();
    p.limpar(biblioteca::familias());
    p
}

fn salvar_prefs(p: &Preferencias) {
    // os testes não escrevem nas preferências de quem os roda
    if cfg!(test) {
        return;
    }
    if let Some(a) = arquivo_prefs() {
        if let Some(pai) = a.parent() {
            let _ = std::fs::create_dir_all(pai);
        }
        let _ = std::fs::write(a, p.para_json());
    }
}

// ---------------------------------------------------------------- painel

fn chip(ui: &mut egui::Ui, ativo: &mut bool, texto: &str, dica: &str) {
    if ui.selectable_label(*ativo, tr(texto)).on_hover_text(tr(dica)).clicked() {
        *ativo = !*ativo;
    }
}

/// Desenha o painel do catálogo, quando aberto. Chamado a cada quadro com a viewport como `area`.
pub fn painel(app: &mut App, ctx: &egui::Context, area: Rect) {
    if app.settings.bench != crate::settings::Bench::Montagem {
        return;
    }
    let Some(mut est) = app.asm.as_mut().and_then(|s| s.catalogo.take()) else { return };
    if !est.prefs_lidas {
        est.prefs = carregar_prefs();
        est.prefs_lidas = true;
    }
    let cfg = Painel { nome: PAINEL.into(), titulo: tr("Biblioteca de normas").to_string(), padrao: paineis::ao_lado_da_arvore(area), largura: 380., redimensiona: true };
    let posicao = app.settings.painel(PAINEL, Encaixe::Esquerda);
    let mut fechar = false;
    let saida = paineis::mostrar(ctx, area, &cfg, posicao, |ui| {
        fechar = crate::app::done_bar(ui);
        conteudo(ui, &mut est);
    });
    if let Some(p) = saida.movido {
        app.settings.guardar_painel(PAINEL, p);
    }
    if fechar {
        est.inserindo = None;
        salvar_prefs(&est.prefs);
    } else if let Some(st) = app.asm.as_mut() {
        st.catalogo = Some(est);
    }
}

fn conteudo(ui: &mut egui::Ui, est: &mut Estado) {
    // O painel (`paineis::mostrar`) limita a altura do corpo à altura útil da janela. Passado esse teto,
    // o egui não tem mais onde pôr as linhas de `horizontal`/`horizontal_wrapped`: elas colapsam em cima
    // umas das outras (Fontes, licença e os botões de inserir ficavam encavalados ao rolar). Aqui toda
    // área rolável de dentro tem altura própria, então o conteúdo cresce livre e a rolagem é a do painel.
    ui.set_max_height(f32::INFINITY);
    // busca
    ui.horizontal(|ui| {
        let r = ui.add(egui::TextEdit::singleline(&mut est.filtro.busca).hint_text(tr("Buscar: M8 sextavado, ISO 4762, 8.8…")).desired_width(f32::INFINITY));
        r.on_hover_text(tr("Escreva o tamanho e o tipo juntos: “M8 sextavado” já escolhe o M8. Acentos e maiúsculas não importam."));
    });
    // chips
    ui.horizontal_wrapped(|ui| {
        let mut m = est.filtro.sistema == Some(Sistema::Metrico);
        let mut p = est.filtro.sistema == Some(Sistema::Polegada);
        chip(ui, &mut m, "Métrico", "Só famílias com tamanhos métricos (M3, M6…).");
        chip(ui, &mut p, "Polegada", "Só famílias em polegada (ASME). Os dados só entram conferidos em duas fontes.");
        est.filtro.sistema = if m && est.filtro.sistema != Some(Sistema::Metrico) { Some(Sistema::Metrico) } else if p && est.filtro.sistema != Some(Sistema::Polegada) { Some(Sistema::Polegada) } else if !m && !p { None } else { est.filtro.sistema };
        ui.separator();
        for c in Cabeca::TODAS {
            let mut a = est.filtro.cabeca == Some(c);
            chip(ui, &mut a, c.rotulo(), "Filtra pelo tipo de cabeça do parafuso.");
            est.filtro.cabeca = if a { Some(c) } else if est.filtro.cabeca == Some(c) { None } else { est.filtro.cabeca };
        }
        ui.separator();
        for mat in ["Aço", "Aço inoxidável"] {
            let mut a = est.filtro.material.as_deref() == Some(mat);
            chip(ui, &mut a, mat, "Filtra pelo material da peça gerada.");
            est.filtro.material = if a { Some(mat.to_string()) } else if est.filtro.material.as_deref() == Some(mat) { None } else { est.filtro.material.clone() };
        }
        if !est.filtro.vazio() && ui.small_button(tr("Limpar")).clicked() {
            est.filtro = Filtro::default();
        }
    });
    ui.horizontal(|ui| {
        ui.selectable_value(&mut est.aba, Aba::Catalogo, tr("Catálogo"));
        ui.selectable_value(&mut est.aba, Aba::Favoritos, format!("★ {}", tr("Favoritos")));
        ui.selectable_value(&mut est.aba, Aba::Recentes, tr("Recentes"));
    });
    let mut escolha: Option<(String, Option<String>)> = None;
    egui::ScrollArea::vertical().id_salt("biblioteca_lista").max_height(170.).auto_shrink([false, true]).show(ui, |ui| match est.aba {
        Aba::Catalogo => {
            let achados = catalogo::buscar(biblioteca::familias(), &est.filtro);
            if achados.is_empty() {
                ui.label(RichText::new(tr("Nada encontrado com esses filtros.")).weak());
            } else if est.filtro.vazio() {
                let todas: Vec<&Familia> = achados.iter().map(|a| a.familia).collect();
                for no in catalogo::arvore(&todas) {
                    egui::CollapsingHeader::new(no.entidade.clone()).default_open(true).show(ui, |ui| {
                        for t in &no.tipos {
                            egui::CollapsingHeader::new(tr(t.tipo.rotulo())).id_salt((&no.entidade, t.tipo.rotulo())).default_open(true).show(ui, |ui| {
                                for f in &t.familias {
                                    linha_familia(ui, est, f, None, &mut escolha);
                                }
                            });
                        }
                    });
                }
            } else {
                for a in &achados {
                    linha_familia(ui, est, a.familia, a.tamanho.as_deref(), &mut escolha);
                }
            }
        }
        Aba::Favoritos => {
            let favoritas: Vec<&Familia> = biblioteca::familias().iter().filter(|f| est.prefs.e_favorita(&f.id)).collect();
            if favoritas.is_empty() {
                ui.label(RichText::new(tr("Marque a estrela na ficha de uma família para vê-la aqui.")).weak());
            }
            for f in favoritas {
                linha_familia(ui, est, f, None, &mut escolha);
            }
        }
        Aba::Recentes => {
            if est.prefs.recentes.is_empty() {
                ui.label(RichText::new(tr("Os itens que você escolher aparecem aqui.")).weak());
            }
            for r in est.prefs.recentes.clone() {
                if let Some(f) = biblioteca::familia(&r.familia) {
                    linha_familia(ui, est, f, Some(&r.tamanho), &mut escolha);
                }
            }
        }
    });
    if let Some((f, t)) = escolha {
        est.escolher(&f, t.as_deref());
    }
    ui.separator();
    ficha_ui(ui, est);
}

fn linha_familia(ui: &mut egui::Ui, est: &Estado, f: &Familia, tamanho: Option<&str>, escolha: &mut Option<(String, Option<String>)>) {
    let sel = est.familia.as_deref() == Some(f.id.as_str());
    let estrela = if est.prefs.e_favorita(&f.id) { "★ " } else { "" };
    let sufixo = tamanho.map(|t| format!(" · {t}")).unwrap_or_default();
    let texto = format!("{estrela}{} · {}{sufixo}", f.norma, tr(f.tipo.rotulo()));
    if ui.selectable_label(sel, texto).on_hover_text(f.nome.as_str()).clicked() {
        *escolha = Some((f.id.clone(), tamanho.map(str::to_string)));
    }
}

fn ficha_ui(ui: &mut egui::Ui, est: &mut Estado) {
    let Some((fam, _)) = est.escolhida() else {
        ui.label(RichText::new(tr("Escolha uma família na lista para ver a ficha e a prévia 3D.")).weak());
        return;
    };
    let ficha = match catalogo::ficha(fam) {
        Ok(f) => f,
        Err(e) => {
            ui.label(RichText::new(format!("⚠ {e}")).color(crate::theme::st_warn()));
            return;
        }
    };
    ui.horizontal(|ui| {
        let fav = est.prefs.e_favorita(&fam.id);
        if ui.button(if fav { "★" } else { "☆" }).on_hover_text(tr("Favorito: a família aparece na aba Favoritos.")).clicked() {
            est.prefs.alternar_favorito(&fam.id);
            salvar_prefs(&est.prefs);
        }
        ui.label(RichText::new(format!("{} · {}", ficha.norma, ficha.revisao)).strong());
    });
    ui.label(RichText::new(&ficha.titulo).small());
    ui.label(RichText::new(format!("{} · {} · {}", tr(ficha.sistema.rotulo()), ficha.material, tr(ficha.cabeca.rotulo()))).small().weak());
    let tamanho_atual = est.tamanho.clone().unwrap_or_default();
    let linha_atual = ficha.linhas.iter().find(|l| l.designacao == tamanho_atual).or(ficha.linhas.first());
    ui.horizontal(|ui| {
        if let Some(l) = linha_atual {
            let (rect, resp) = ui.allocate_exact_size(egui::vec2(150., 150.), Sense::drag());
            previa_3d(ui, rect, &resp, est, fam, &l.designacao);
            ui.vertical(|ui| {
                esquema(ui, &ficha, l, est.comprimento);
            });
        }
    });
    // curso do cilindro ou comprimento do trilho: livre entre o menor e o maior da série, com os
    // valores usuais no menu
    if matches!(fam.tipo, TipoFixador::CilindroPneumatico | TipoFixador::TrilhoLinear) {
        let trilho = fam.tipo == TipoFixador::TrilhoLinear;
        if let Some(t) = est.tamanho.as_deref().and_then(|t| fam.tamanho(t)) {
            let serie = fam.comprimentos_de(t);
            if let (Some(&lo), Some(&hi)) = (serie.first(), serie.last()) {
                ui.horizontal(|ui| {
                    ui.label(tr(if trilho { "Comprimento do trilho" } else { "Curso da haste" }));
                    let mut c = est.comprimento.unwrap_or_else(|| serie.iter().copied().find(|c| *c >= 100.).unwrap_or(lo));
                    let r = ui.add(crate::ui_ext::dv(&mut c).range(lo..=hi).suffix(" mm"));
                    egui::ComboBox::from_id_salt("biblioteca_curso").selected_text(tr("Padrão")).show_ui(ui, |ui| {
                        for l in &serie {
                            if ui.selectable_label(est.comprimento == Some(*l), fmt(*l)).clicked() {
                                c = *l;
                            }
                        }
                    });
                    est.comprimento = Some(c);
                    r
                })
                .inner
                .on_hover_text(tr(if trilho {
                    "Comprimento do trilho em mm, livre até o máximo do fabricante. Os furos ficam no passo P, com a mesma sobra nas duas pontas."
                } else {
                    "Curso da haste em mm. A norma não fixa o curso: o menu traz os cursos padrão do fabricante, e qualquer valor entre o menor e o maior vale."
                }));
            }
        }
    }
    // comprimento
    if fam.tipo.e_parafuso() {
        if let Some(t) = est.tamanho.as_deref().and_then(|t| fam.tamanho(t)) {
            let serie = fam.comprimentos_de(t);
            ui.horizontal(|ui| {
                ui.label(tr("Comprimento"));
                let atual = est.comprimento.map(fmt).unwrap_or_else(|| tr("Pelo furo").to_string());
                egui::ComboBox::from_id_salt("biblioteca_comprimento").selected_text(atual).show_ui(ui, |ui| {
                    ui.selectable_value(&mut est.comprimento, None, tr("Pelo furo"));
                    for l in &serie {
                        ui.selectable_value(&mut est.comprimento, Some(*l), fmt(*l));
                    }
                });
            })
            .response
            .on_hover_text(tr("Só existem os comprimentos que a norma publica para o tamanho. “Pelo furo” escolhe o menor que atravessa a espessura."));
        }
    }
    // rebite: a pessoa dá a pega (espessura da pilha) e o L sai dela; "Pelo furo" deixa a espessura do furo mandar
    if fam.tipo == TipoFixador::Rebite {
        if let Some(t) = est.tamanho.as_deref().and_then(|t| fam.tamanho(t)) {
            let d = t.diametro_nominal().unwrap_or(0.);
            ui.horizontal(|ui| {
                ui.label(tr("Pega"));
                let mut pelo_furo = est.pega.is_none();
                let mut pega = est.pega.unwrap_or(d);
                ui.add_enabled_ui(!pelo_furo, |ui| ui.add(crate::ui_ext::dv(&mut pega).range(0.1..=60.).suffix(" mm")))
                    .inner
                    .on_hover_text(tr("Espessura total das chapas que o rebite une. O comprimento é a pega mais 1,5 diâmetro, arredondado para cima na série de 1/16 pol."));
                ui.checkbox(&mut pelo_furo, tr("Pelo furo")).on_hover_text(tr("A pega é a espessura que o furo atravessa."));
                est.pega = if pelo_furo { None } else { Some(pega) };
                est.comprimento = est.pega.and_then(|g| biblioteca::rebite_pela_pega(fam, t, g));
                match (est.pega, est.comprimento) {
                    (Some(_), Some(l)) => ui.label(RichText::new(format!("L {}", fmt(l))).small()),
                    (Some(_), None) => ui.label(RichText::new(tr("Pega grossa demais para a série")).small().color(crate::theme::st_warn())),
                    _ => ui.label(""),
                };
            });
        }
    }
    // tabela: uma linha por tamanho
    egui::ScrollArea::vertical().id_salt("biblioteca_tabela").max_height(150.).auto_shrink([false, true]).show(ui, |ui| {
        egui::Grid::new("biblioteca_grade").striped(true).num_columns(ficha.cotas.len() + 2).show(ui, |ui| {
            ui.label(RichText::new(tr("Tamanho")).strong());
            for c in &ficha.cotas {
                ui.label(RichText::new(format!("{} ({})", c.simbolo, c.unidade)).strong().small()).on_hover_text(tr(&c.nome));
            }
            ui.label(RichText::new(tr("Fontes")).strong().small());
            ui.end_row();
            let mut novo: Option<String> = None;
            for l in &ficha.linhas {
                let sel = l.designacao == tamanho_atual;
                let r = ui.selectable_label(sel, &l.designacao);
                let arrasto = ui.interact(r.rect, r.id.with("arrasto"), Sense::drag());
                if r.clicked() {
                    novo = Some(l.designacao.clone());
                }
                if arrasto.drag_started() {
                    novo = Some(l.designacao.clone());
                    est.inserindo = Some(Insercao { arrastando: true });
                    est.furos = None;
                }
                arrasto.on_hover_text(tr("Arraste para a viewport e solte: o item entra onde o modo escolhido abaixo manda (furo, ponto ou solto)."));
                for v in &l.valores {
                    ui.label(RichText::new(fmt(*v)).small());
                }
                ui.label(RichText::new(format!("{}×", l.confirmado_por)).small().weak());
                ui.end_row();
            }
            if let Some(t) = novo {
                let f = fam.id.clone();
                est.escolher(&f, Some(&t));
            }
        });
    });
    // fontes
    egui::CollapsingHeader::new(tr("Fontes e licença")).open(est.fontes_abertas_demo.then_some(true)).show(ui, |ui| {
        ui.label(RichText::new(format!("{}: {} · {}", tr("Nível"), ficha.nivel, ficha.licenca)).small());
        for f in &ficha.fontes {
            ui.label(RichText::new(format!("• {}", f.rotulo)).small());
            ui.label(RichText::new(&f.url).small().weak());
        }
    });
    // inserção
    ui.separator();
    ui.horizontal(|ui| {
        ui.radio_value(&mut est.modo, ModoInsercao::Padrao, tr("Padrão")).on_hover_text(tr("A peça é compartilhada: todos os itens iguais usam o mesmo arquivo."));
        ui.radio_value(&mut est.modo, ModoInsercao::Personalizado, tr("Personalizado")).on_hover_text(tr("Grava uma cópia só deste item, que você pode editar sem mexer nos outros."));
    });
    ui.label(RichText::new(tr("Onde inserir")).small().weak());
    ui.horizontal_wrapped(|ui| {
        for alvo in Alvo::TODOS {
            let ativo = est.inserindo.is_some() && est.alvo == alvo;
            let marca = match alvo {
                Alvo::Furo => "⌖",
                Alvo::Ponto => "•",
                Alvo::Solto => "✥",
            };
            if ui.selectable_label(ativo, format!("{marca} {}", tr(alvo.rotulo()))).on_hover_text(tr(alvo.dica())).clicked() {
                est.alvo = alvo;
                est.inserindo = if ativo { None } else { Some(Insercao { arrastando: false }) };
                est.furos = None;
            }
        }
    });
    ui.horizontal_wrapped(|ui| {
        if ui.button(tr("Adicionar à montagem")).on_hover_text(tr("Põe o item agora na montagem, como peça livre no centro da vista, sem precisar clicar na viewport. Depois é só posicionar.")).clicked() {
            est.adicionar_agora = true;
        }
    });
    if est.inserindo.is_some() {
        ui.label(RichText::new(tr(est.alvo.ajuda())).small().weak());
    }
}

/// Prévia 3D: arrastar gira.
fn previa_3d(ui: &mut egui::Ui, rect: Rect, resp: &egui::Response, est: &mut Estado, fam: &Familia, tamanho: &str) {
    if resp.dragged() {
        let d = resp.drag_delta();
        est.yaw -= d.x * 0.012;
        est.pitch = (est.pitch + d.y * 0.012).clamp(-1.5, 1.5);
    }
    resp.clone().on_hover_text(tr("Arraste para girar a prévia."));
    let p = ui.painter_at(rect);
    p.rect_filled(rect, 6., ui.visuals().extreme_bg_color);
    p.rect_stroke(rect, 6., Stroke::new(1., ui.visuals().widgets.noninteractive.bg_stroke.color), egui::StrokeKind::Inside);
    let Some(m) = malha_do_item(&fam.id, tamanho, if fam.tipo.usa_comprimento() { est.comprimento } else { None }) else {
        p.text(rect.center(), egui::Align2::CENTER_CENTER, tr("sem prévia"), egui::FontId::proportional(12.), ui.visuals().weak_text_color());
        return;
    };
    let (sy, cy) = est.yaw.sin_cos();
    let (sp, cp) = est.pitch.sin_cos();
    let c = m.centro;
    // local → vista: gira em Z (yaw) e inclina em X (pitch); x à direita, y para dentro, z para cima
    let dir = move |v: Vec3| {
        let (x1, y1, z1) = (v.x * cy - v.y * sy, v.x * sy + v.y * cy, v.z);
        Vec3::new(x1, y1 * cp - z1 * sp, y1 * sp + z1 * cp)
    };
    let ponto = move |v: Vec3| dir(v - c);
    let escala = rect.width() * 0.44 / m.raio;
    let centro = rect.center();
    let proj = move |v: Vec3| Some(Pos2::new(centro.x + v.x * escala, centro.y - v.z * escala));
    let cor = crate::theme::accent(ui.visuals());
    desenhar(&p, &m, &ponto, &dir, Vec3::Y, &proj, cor, 1., Some(Stroke::new(1., ui.visuals().text_color().gamma_multiply(0.7))));
}

/// Esquema das cotas: vista lateral com o valor de cada medida do tamanho escolhido ao lado.
fn esquema(ui: &mut egui::Ui, ficha: &catalogo::Ficha, l: &catalogo::LinhaTamanho, comprimento: Option<f64>) {
    let valor = |s: &str| ficha.cotas.iter().position(|c| c.simbolo == s).map(|i| l.valores[i]);
    // o cilindro tem doze cotas: a lista de valores pede mais altura
    let altura = if ficha.tipo == TipoFixador::CilindroPneumatico { 215. } else { 150. };
    let (rect, _) = ui.allocate_exact_size(egui::vec2(190., altura), Sense::hover());
    let p = ui.painter_at(rect);
    let traco = Stroke::new(1.5, ui.visuals().text_color());
    let fino = Stroke::new(1., ui.visuals().weak_text_color());
    let fonte = egui::FontId::proportional(11.);
    let cor_texto = crate::theme::accent(ui.visuals());
    let d = l.d_nominal;
    let e = |s: f64| s * 1.1547;
    let mut notas: Vec<(String, f64)> = vec![];
    match ficha.tipo {
        TipoFixador::Prisioneiro => {
            // haste única: ponta fixa bm em cima (entra na peça roscada), rosca b do lado da porca embaixo
            let bm = valor("bm").unwrap_or(d);
            let ls = comprimento.or(l.comprimentos.iter().copied().find(|x| *x >= 4. * d)).or(l.comprimentos.last().copied()).unwrap_or(d * 4.);
            let esc = ((rect.height() - 40.) / (bm + ls) as f32).min((rect.width() * 0.5 - 20.) / d as f32);
            let cx = rect.left() + 70.;
            let topo = rect.top() + 14.;
            let (sw, hb, sl) = (d as f32 * esc, bm as f32 * esc, ls as f32 * esc);
            p.rect_stroke(Rect::from_min_size(Pos2::new(cx - sw / 2., topo), egui::vec2(sw, hb + sl)), 0., traco, egui::StrokeKind::Middle);
            // a face da peça roscada fica no fim da ponta fixa
            p.line_segment([Pos2::new(cx - sw, topo + hb), Pos2::new(cx + sw, topo + hb)], fino);
            p.line_segment([Pos2::new(cx, topo - 6.), Pos2::new(cx, topo + hb + sl + 6.)], fino);
            notas.push((format!("d {}", fmt(d)), 0.));
            notas.push((format!("bm {}", fmt(bm)), 1.));
            notas.push((format!("L {}", fmt(ls)), 2.));
            if let Some(b) = valor("b") {
                notas.push((format!("b {}", fmt(b)), 3.));
            }
            if let Some(pp) = valor("P") {
                notas.push((format!("P {}", fmt(pp)), 4.));
            }
        }
        TipoFixador::ParafusoAllen | TipoFixador::ParafusoSextavado | TipoFixador::ParafusoCruz => {
            let escareada = ficha.cabeca == catalogo::Cabeca::Escareada;
            let k = valor("k").unwrap_or(d * 0.6);
            let larg = valor("dk").unwrap_or_else(|| e(valor("s").unwrap_or(d * 1.6)));
            let ls = comprimento.or(l.comprimentos.iter().copied().find(|x| *x >= 4. * d)).or(l.comprimentos.last().copied()).unwrap_or(d * 4.);
            // no escareado o L conta a cabeça
            let total = if escareada { ls } else { k + ls };
            let esc = ((rect.height() - 40.) / total as f32).min((rect.width() * 0.5 - 20.) / larg as f32);
            let cx = rect.left() + 70.;
            let topo = rect.top() + 14.;
            let (hw, hh, sw) = (larg as f32 * esc, k as f32 * esc, d as f32 * esc);
            let sl = (total - k) as f32 * esc;
            if escareada {
                let pts = vec![Pos2::new(cx - hw / 2., topo), Pos2::new(cx + hw / 2., topo), Pos2::new(cx + sw / 2., topo + hh), Pos2::new(cx - sw / 2., topo + hh)];
                p.add(egui::Shape::closed_line(pts, traco));
            } else {
                p.rect_stroke(Rect::from_min_size(Pos2::new(cx - hw / 2., topo), egui::vec2(hw, hh)), 1., traco, egui::StrokeKind::Middle);
            }
            let haste = Rect::from_min_size(Pos2::new(cx - sw / 2., topo + hh), egui::vec2(sw, sl));
            p.rect_stroke(haste, 0., traco, egui::StrokeKind::Middle);
            // ombro: a rosca menor dr continua depois do ombro
            if let (Some(dr), Some(b)) = (valor("dr"), valor("b")) {
                let (rw, rl) = (dr as f32 * esc, (b as f32 * esc).min(rect.bottom() - haste.bottom() - 4.).max(0.));
                p.rect_stroke(Rect::from_min_size(Pos2::new(cx - rw / 2., haste.bottom()), egui::vec2(rw, rl)), 0., traco, egui::StrokeKind::Middle);
            }
            p.line_segment([Pos2::new(cx, topo - 6.), Pos2::new(cx, haste.bottom() + 6.)], fino);
            notas.push((format!("{} {}", if valor("dk").is_some() { "dk" } else { "e" }, fmt(larg)), 0.));
            notas.push((format!("k {}", fmt(k)), 1.));
            notas.push((format!("d {}", fmt(d)), 2.));
            notas.push((format!("L {}", fmt(ls)), 3.));
            if let Some(s) = valor("s") {
                notas.push((format!("s {}", fmt(s)), 4.));
            }
            if let Some(pp) = valor("P") {
                notas.push((format!("P {}", fmt(pp)), 5.));
            }
            if let Some(b) = valor("b") {
                notas.push((format!("b {}", fmt(b)), 6.));
            }
        }
        TipoFixador::Rebite => {
            // vista lateral: cabeça (calota ou cone de 100°) e corpo; no escareado o L conta a cabeça
            let escareada = ficha.cabeca == catalogo::Cabeca::Escareada;
            let dk = valor("dk").unwrap_or(d * 2.);
            let k = valor("k").unwrap_or_else(|| fenkra_solidos::rebites::altura_escareada(dk, d));
            let ls = comprimento.or(l.comprimentos.iter().copied().find(|x| *x >= 2.5 * d)).unwrap_or(d * 3.);
            let total = if escareada { ls } else { k + ls };
            let esc = ((rect.height() - 40.) / total as f32).min((rect.width() * 0.5 - 20.) / dk as f32);
            let cx = rect.left() + 70.;
            let topo = rect.top() + 14.;
            let (hw, hh, sw) = (dk as f32 * esc, k as f32 * esc, d as f32 * esc);
            let pts: Vec<Pos2> = if escareada {
                vec![Pos2::new(cx - hw / 2., topo), Pos2::new(cx + hw / 2., topo), Pos2::new(cx + sw / 2., topo + hh), Pos2::new(cx - sw / 2., topo + hh)]
            } else {
                // calota: arco de -hw/2 a hw/2 com flecha hh
                (0..=16).map(|i| {
                    let x = -1. + i as f32 / 8.;
                    Pos2::new(cx + x * hw / 2., topo + hh * x * x)
                }).collect()
            };
            p.add(egui::Shape::closed_line(pts, traco));
            let haste = Rect::from_min_size(Pos2::new(cx - sw / 2., topo + hh), egui::vec2(sw, (total - k) as f32 * esc));
            p.rect_stroke(haste, 0., traco, egui::StrokeKind::Middle);
            p.line_segment([Pos2::new(cx, topo - 6.), Pos2::new(cx, haste.bottom() + 6.)], fino);
            notas.push((format!("dk {}", fmt(dk)), 0.));
            notas.push((format!("k {}", fmt(k)), 1.));
            notas.push((format!("d {}", fmt(d)), 2.));
            notas.push((format!("L {}", fmt(ls)), 3.));
            notas.push((format!("{} {}", tr("pega"), fmt(ls - fenkra_solidos::biblioteca::SOBRA_REBITE * d)), 4.));
        }
        TipoFixador::Porca => {
            let (s, m) = (valor("s").unwrap_or(d * 1.6), valor("m").unwrap_or(d * 0.8));
            let esc = ((rect.height() - 40.) / m as f32).min((rect.width() * 0.5 - 20.) / e(s) as f32);
            let cx = rect.left() + 70.;
            let r = Rect::from_center_size(Pos2::new(cx, rect.top() + 60.), egui::vec2(e(s) as f32 * esc, m as f32 * esc));
            p.rect_stroke(r, 1., traco, egui::StrokeKind::Middle);
            p.line_segment([Pos2::new(cx, r.top() - 8.), Pos2::new(cx, r.bottom() + 8.)], fino);
            notas.push((format!("s {}", fmt(s)), 0.));
            notas.push((format!("m {}", fmt(m)), 1.));
            notas.push((format!("d {}", fmt(d)), 2.));
        }
        TipoFixador::Rolamento => {
            // corte pelo eixo: anel externo, faixa das esferas e anel interno, nas proporções do modelo
            let (dd, b) = (valor("D").unwrap_or(d * 2.), valor("B").unwrap_or(d * 0.5));
            let (d2, d1) = (dd - (dd - d) / 3., d + (dd - d) / 3.);
            let esc = ((rect.width() * 0.6 - 10.) / dd as f32).min((rect.height() - 50.) / b as f32);
            let cx = rect.left() + 62.;
            let cy = rect.top() + 70.;
            let hb = b as f32 * esc / 2.;
            let x = |v: f64| v as f32 * esc / 2.;
            for lado in [-1., 1.] {
                let faixa = |r0: f64, r1: f64, meia: f32| Rect::from_two_pos(Pos2::new(cx + lado * x(r0), cy - meia), Pos2::new(cx + lado * x(r1), cy + meia));
                p.rect_stroke(faixa(d2, dd, hb), 0., traco, egui::StrokeKind::Middle);
                p.rect_stroke(faixa(d, d1, hb), 0., traco, egui::StrokeKind::Middle);
                p.rect_stroke(faixa(d1, d2, hb / 2.), 0., fino, egui::StrokeKind::Middle);
            }
            p.line_segment([Pos2::new(cx, cy - hb - 10.), Pos2::new(cx, cy + hb + 10.)], fino);
            notas.push((format!("d {}", fmt(d)), 0.));
            notas.push((format!("D {}", fmt(dd)), 1.));
            notas.push((format!("B {}", fmt(b)), 2.));
        }
        TipoFixador::CilindroPneumatico if valor("XC").is_some() => {
            // minicilindro ISO 6432 de lado: olhal EW, tubo D, nariz BE, haste MM até XC e a rosca KK por AM
            let v = |s: &str, padrao: f64| valor(s).unwrap_or(padrao);
            let (xc, wf, cd, ew, dd, be, mm, kk, am) = (v("XC", d * 6.), v("WF", d * 2.), v("CD", d * 0.5), v("EW", d), v("D", d * 1.6), v("BE", d * 1.2), v("MM", d * 0.4), v("KK", d * 0.4), v("AM", d * 1.2));
            let curso = comprimento.or(l.comprimentos.iter().copied().find(|c| *c >= 50.)).unwrap_or(50.);
            let total = cd + xc + am;
            let esc = (160. / total as f32).min(60. / dd as f32);
            let x0 = rect.left() + 8.;
            let cy = rect.top() + 60.;
            let caixa = |za: f64, zb: f64, alt: f64| Rect::from_two_pos(Pos2::new(x0 + (za + cd) as f32 * esc, cy - alt as f32 * esc / 2.), Pos2::new(x0 + (zb + cd) as f32 * esc, cy + alt as f32 * esc / 2.));
            p.rect_stroke(caixa(-cd, cd, ew), 1., traco, egui::StrokeKind::Middle);
            p.rect_stroke(caixa(cd, xc - wf, dd), 2., traco, egui::StrokeKind::Middle);
            p.rect_stroke(caixa(xc - wf, xc - wf / 2., be), 0., fino, egui::StrokeKind::Middle);
            p.rect_stroke(caixa(xc - wf, xc, mm), 0., traco, egui::StrokeKind::Middle);
            p.rect_stroke(caixa(xc, xc + am, kk), 0., traco, egui::StrokeKind::Middle);
            p.line_segment([Pos2::new(x0 - 4., cy), Pos2::new(x0 + total as f32 * esc + 6., cy)], fino);
            notas.push((format!("Ø {}", fmt(d)), 0.));
            notas.push((format!("XC {}", fmt(xc)), 1.));
            notas.push((format!("{} {}", tr("curso"), fmt(curso)), 2.));
        }
        TipoFixador::CilindroPneumatico => {
            // vista lateral: corpo E × L8, ressalto B, haste MM até WH + curso e a rosca KK por AM
            let v = |s: &str, padrao: f64| valor(s).unwrap_or(padrao);
            let (e_, l8, wh, mm, kk, am, b) = (v("E", d * 1.4), v("L8", d * 3.), v("WH", d), v("MM", d * 0.4), v("KK", d * 0.3), v("AM", d * 0.7), v("B", d));
            let curso = comprimento.or(l.comprimentos.iter().copied().find(|c| *c >= 100.)).unwrap_or(100.);
            let total = l8 + wh + am;
            let esc = (105. / total as f32).min(60. / e_ as f32);
            let x0 = rect.left() + 8.;
            let cy = rect.top() + 60.;
            let caixa = |xa: f64, xb: f64, alt: f64| Rect::from_two_pos(Pos2::new(x0 + xa as f32 * esc, cy - alt as f32 * esc / 2.), Pos2::new(x0 + xb as f32 * esc, cy + alt as f32 * esc / 2.));
            p.rect_stroke(caixa(0., l8, e_), 1., traco, egui::StrokeKind::Middle);
            p.rect_stroke(caixa(l8, l8 + (wh / 4.).min(4.), b), 0., fino, egui::StrokeKind::Middle);
            p.rect_stroke(caixa(l8, l8 + wh, mm), 0., traco, egui::StrokeKind::Middle);
            p.rect_stroke(caixa(l8 + wh, total, kk), 0., traco, egui::StrokeKind::Middle);
            p.line_segment([Pos2::new(x0 - 4., cy), Pos2::new(x0 + total as f32 * esc + 6., cy)], fino);
            notas.push((format!("Ø {}", fmt(d)), 0.));
            notas.push((format!("{} {}", tr("curso"), fmt(curso)), 1.));
        }
        TipoFixador::TrilhoLinear => {
            // corte do trilho: perfil WR × HR com um furo escareado D × h e o passante dh
            let v = |s: &str, padrao: f64| valor(s).unwrap_or(padrao);
            let (wr, hr, dd, h, dh) = (v("WR", d), v("HR", d), v("D", d * 0.5), v("h", d * 0.35), v("dh", d * 0.3));
            let esc = (100. / wr as f32).min(110. / hr as f32);
            let cx = rect.left() + 60.;
            let base = rect.top() + 130.;
            let x = |m: f64| m as f32 * esc / 2.;
            let alt = |m: f64| m as f32 * esc;
            p.rect_stroke(Rect::from_two_pos(Pos2::new(cx - x(wr), base - alt(hr)), Pos2::new(cx + x(wr), base)), 0., traco, egui::StrokeKind::Middle);
            p.rect_stroke(Rect::from_two_pos(Pos2::new(cx - x(dd), base - alt(hr)), Pos2::new(cx + x(dd), base - alt(hr - h))), 0., fino, egui::StrokeKind::Middle);
            p.rect_stroke(Rect::from_two_pos(Pos2::new(cx - x(dh), base - alt(hr - h)), Pos2::new(cx + x(dh), base)), 0., fino, egui::StrokeKind::Middle);
            notas.push((format!("L {}", fmt(comprimento.unwrap_or(100.))), 0.));
        }
        TipoFixador::Arruela => {
            let (d1, d2, h) = (valor("d1").unwrap_or(d), valor("d2").unwrap_or(d * 2.), valor("h").unwrap_or(d * 0.2));
            let esc = ((rect.width() * 0.5 - 20.) / d2 as f32).min(40. / h as f32);
            let cx = rect.left() + 70.;
            let cy = rect.top() + 60.;
            let (ow, ih, hh) = (d2 as f32 * esc, d1 as f32 * esc, h as f32 * esc);
            for lado in [-1., 1.] {
                let x0 = cx + lado * ih / 2.;
                let x1 = cx + lado * ow / 2.;
                p.rect_stroke(Rect::from_two_pos(Pos2::new(x0, cy - hh / 2.), Pos2::new(x1, cy + hh / 2.)), 0., traco, egui::StrokeKind::Middle);
            }
            p.line_segment([Pos2::new(cx, cy - hh - 10.), Pos2::new(cx, cy + hh + 10.)], fino);
            notas.push((format!("d1 {}", fmt(d1)), 0.));
            notas.push((format!("d2 {}", fmt(d2)), 1.));
            notas.push((format!("h {}", fmt(h)), 2.));
        }
    }
    // toda letra da ficha aparece no esquema, mesmo as que o desenho não cota (dr, dc, c...)
    let mut prox = notas.iter().map(|(_, i)| *i).fold(-1., f64::max) + 1.;
    for (c, v) in ficha.cotas.iter().zip(&l.valores) {
        if !notas.iter().any(|(t, _)| t.split_whitespace().next() == Some(c.simbolo.as_str())) {
            notas.push((format!("{} {}", c.simbolo, fmt(*v)), prox));
            prox += 1.;
        }
    }
    for (texto, i) in notas {
        p.text(Pos2::new(rect.left() + 120., rect.top() + 8. + i as f32 * 16.), egui::Align2::LEFT_TOP, texto, fonte.clone(), cor_texto);
    }
}

// ---------------------------------------------------------------- inserção na viewport (B242)

/// Resultado de um quadro da inserção: aviso para a tela.
type Aviso = Option<(String, bool)>;

/// Fantasma do item que segue o cursor, salto para o furo, Tab, alça de comprimento e a inserção.
/// Chamado com a viewport; sem inserção armada não faz nada.
pub fn overlay(app: &mut App, ui: &mut egui::Ui, rect: Rect, cam: &Camera) {
    if app.settings.bench != crate::settings::Bench::Montagem {
        return;
    }
    let Some(st) = app.asm.as_mut() else { return };
    let Some(mut est) = st.catalogo.take() else { return };
    let mut aviso = if est.inserindo.is_some() { quadro_da_insercao(st, &mut est, ui, rect, cam) } else { None };
    if std::mem::take(&mut est.adicionar_agora) {
        aviso = adicionar_agora(st, &mut est, cam);
    }
    st.catalogo = Some(est);
    if let Some((msg, erro)) = aviso {
        app.toast(msg, erro);
    }
}

fn quadro_da_insercao(st: &mut crate::asm_ui::AsmState, est: &mut Estado, ui: &mut egui::Ui, rect: Rect, cam: &Camera) -> Aviso {
    let ctx = ui.ctx().clone();
    let Some(ins) = est.inserindo else { return None };
    let Some((fam, t)) = est.escolhida() else {
        est.inserindo = None;
        return Some((tr("Escolha um item no catálogo antes de inserir").to_string(), true));
    };
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        est.inserindo = None;
        return None;
    }
    if est.alvo != Alvo::Furo {
        return quadro_livre(st, est, ui, rect, cam, ins, fam, t);
    }
    if est.furos.is_none() {
        let dir = st.dir();
        est.furos = Some(avulso::furos_do_mundo(&st.asm, dir.as_deref(), &mut st.cache, &[]));
    }
    let pos = ctx.input(|i| i.pointer.latest_pos());
    let sobre_viewport = pos.is_some_and(|p| rect.contains(p) && ctx.layer_id_at(p).is_some_and(|l| l.order == egui::Order::Background));
    let solto = ctx.input(|i| i.pointer.primary_released());
    let clicou = ctx.input(|i| i.pointer.primary_clicked());
    let alt = ctx.input(|i| i.modifiers.alt);
    if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Tab)) {
        est.inverter = !est.inverter;
    }
    let Some(pos) = pos.filter(|_| sobre_viewport) else {
        if ins.arrastando && solto {
            est.inserindo = None;
        }
        return None;
    };
    // furo mais perto do cursor entre os que casam com o tamanho escolhido
    let furos = est.furos.clone().unwrap_or_default();
    let mut melhor: Option<(f32, &FuroAlvo, (fenkra_montagem::V3, fenkra_montagem::V3))> = None;
    for f in furos.iter().filter(|f| avulso::casa(fam, t, f)) {
        let Ok(pose) = avulso::pose(f, est.inverter).or_else(|_| avulso::pose(f, false)) else { continue };
        let Some(s) = cam.project(rect, v3(pose.0)) else { continue };
        let dist = s.distance(pos);
        if dist <= ALCANCE_PX && melhor.as_ref().is_none_or(|m| dist < m.0) {
            melhor = Some((dist, f, pose));
        }
    }
    let painter = ui.painter_at(rect);
    let acento = crate::theme::accent(&ctx.global_style().visuals);
    let aviso_cor = crate::theme::st_warn();
    let parafuso = fam.tipo.atravessa_furo();
    // comprimento do que se desenha: o pedido, ou o que o furo pede
    let comp = if parafuso { est.comprimento.or_else(|| melhor.as_ref().and_then(|m| avulso::comprimento_sugerido(fam, t, m.1))).or_else(|| fam.comprimentos_de(t).first().copied()) } else { None };
    let malha = malha_do_item(&fam.id, &t.designacao, comp);
    let mut sobre_alca = false;
    let mut mudou: Option<f64> = None;
    match (&melhor, &malha) {
        (Some((_, f, (entrada, u))), Some(m)) => {
            fantasma(&painter, rect, cam, m, avulso::posicao_do_item(parafuso, *entrada, *u), acento, 0.4);
            // ponto publicado não tem diâmetro: o rótulo só diz o nome do ponto
            let diam = if f.diametro > 0. { format!(" Ø{}", fmt(f.diametro)) } else { String::new() };
            let (rotulo, base) = (format!("{}{} → {}{diam}", t.designacao, comp.map(|l| format!("x{}", fmt(l))).unwrap_or_default(), f.nome), cam.project(rect, v3(*entrada)).unwrap_or(pos));
            painter.text(base + egui::vec2(16., -18.), egui::Align2::LEFT_BOTTOM, rotulo, egui::FontId::proportional(12.), acento);
            // alça de comprimento na ponta: só para na série do tamanho
            if let (true, Some(l), false) = (parafuso, comp, ins.arrastando) {
                let ponta = [0, 1, 2].map(|i| entrada[i] + u[i] * l);
                if let (Some(c), Some(c2)) = (cam.project(rect, v3(ponta)), cam.project(rect, v3([0, 1, 2].map(|i| ponta[i] + u[i] * 10.)))) {
                    let na_tela = c2 - c;
                    let px_mm = na_tela.length() / 10.;
                    let id = egui::Id::unique("alca_avulso");
                    let resp = ui.interact(Rect::from_center_size(c, egui::vec2(20., 20.)), id, Sense::click_and_drag());
                    sobre_alca = resp.hovered() || resp.dragged();
                    painter.circle_filled(c, 6.5, if sobre_alca { aviso_cor } else { acento });
                    if sobre_alca {
                        painter.text(c + egui::vec2(12., -12.), egui::Align2::LEFT_BOTTOM, format!("L {}", fmt(l)), egui::FontId::proportional(12.), aviso_cor);
                    }
                    if resp.drag_started() {
                        ctx.data_mut(|d| d.insert_temp(id, l));
                    }
                    if resp.dragged() && px_mm > 1e-3 && na_tela.length() > 1e-3 {
                        let delta = resp.drag_delta().dot(na_tela / na_tela.length()) / px_mm;
                        let alvo = ctx.data(|d| d.get_temp::<f64>(id)).unwrap_or(l) + delta as f64;
                        ctx.data_mut(|d| d.insert_temp(id, alvo));
                        mudou = fenkra_montagem::fixadores::mais_proximo(&fam.comprimentos_de(t), alvo);
                    }
                }
            }
        }
        (None, Some(m)) => {
            // sem furo que case: o fantasma acompanha o cursor, em âmbar, parado a uma distância fixa
            let (o, d) = cam.ray(rect, pos);
            let origem = o + d * cam.dist;
            let pl = fenkra_montagem::Placement { origin: [origem.x as f64, origem.y as f64, origem.z as f64], ..Default::default() };
            let (ponto, dir) = no_mundo(pl);
            let olho = cam.ray(rect, rect.center()).1;
            let proj = |p: Vec3| cam.project(rect, p);
            desenhar(&painter, m, &ponto, &dir, olho, &proj, aviso_cor, 0.3, Some(Stroke::new(1., aviso_cor)));
            let n = furos.iter().filter(|f| avulso::casa(fam, t, f)).count();
            let texto = if n == 0 { format!("{} {} {}", tr("nenhum furo casa com"), t.designacao, tr("nesta montagem")) } else { tr("aproxime de um furo").to_string() };
            painter.text(pos + egui::vec2(16., -18.), egui::Align2::LEFT_BOTTOM, texto, egui::FontId::proportional(12.), aviso_cor);
        }
        _ => {}
    }
    if let Some(l) = mudou {
        est.comprimento = Some(l);
    }
    // decisão: soltar (arrasto) ou clicar (armado)
    let decide = if ins.arrastando { solto } else { clicou && !sobre_alca };
    if !decide {
        return None;
    }
    if ins.arrastando {
        est.inserindo = None;
    }
    if alt {
        est.foco_ficha = true;
        est.inserindo = None;
        return Some((tr("Ficha aberta no painel Biblioteca").to_string(), false));
    }
    let Some((_, furo, _)) = melhor else {
        return Some((tr("Solte o item sobre um furo que case com o tamanho escolhido").to_string(), true));
    };
    let furo = furo.clone();
    let Some(dir) = st.dir() else { return Some((tr("Salve a montagem antes: as peças dos fixadores são gravadas na pasta dela").to_string(), true)) };
    let modificada = st.modified;
    st.push_undo();
    let p = ParamAvulso { familia: fam.id.clone(), tamanho: t.designacao.clone(), comprimento: comp, furo, inverter: est.inverter, modo: est.modo };
    match avulso::inserir_avulso(&mut st.asm, &dir, biblioteca::publica(), &p) {
        Ok(r) => {
            st.solve();
            est.furos = None;
            est.prefs.registrar(&fam.id, &t.designacao);
            salvar_prefs(&est.prefs);
            let gdl = st.comp_dof.get(&r.comp).copied().unwrap_or(0);
            Some((format!("{} {} · {} {gdl}", tr("Inserido"), r.nome, tr("graus de liberdade")), false))
        }
        Err(e) => {
            if let Some(antes) = st.undo.pop() {
                st.asm = antes;
            }
            st.modified = modificada;
            st.dirty = true;
            Some((e, true))
        }
    }
}

// ---------------------------------------------------------------- inserção em ponto e solto (B374)

fn d3(v: Vec3) -> fenkra_montagem::V3 {
    [v.x as f64, v.y as f64, v.z as f64]
}

/// Comprimento do parafuso fora de furo: o pedido, ou o da série mais perto de quatro diâmetros.
fn comprimento_fora_do_furo(est: &Estado, fam: &Familia, t: &biblioteca::Tamanho) -> Option<f64> {
    if !fam.tipo.atravessa_furo() {
        return None;
    }
    est.comprimento.or_else(|| avulso::comprimento_livre(fam, t))
}

/// Grupo de destino da peça livre: o grupo selecionado na árvore, ou a raiz.
fn grupo_de_destino(st: &crate::asm_ui::AsmState) -> Option<fenkra_core::Id> {
    st.sel.filter(|s| st.asm.node(*s).is_some_and(|n| matches!(n.kind, fenkra_montagem::NodeKind::Group)))
}

/// As malhas da vista como o raio as enxerga: cada cópia de peça pela malha do protótipo e a posição
/// dela (na vista montada em segundo plano os corpos das cópias não trazem malha própria), e os corpos
/// sem cópia, que já estão no mundo. Cópia de componente oculto não entra.
fn malhas_da_cena(st: &crate::asm_ui::AsmState) -> Vec<(&fenkra_kernel::Mesh, fenkra_montagem::Placement)> {
    let mut v = vec![];
    let Some(vista) = st.view.as_ref() else { return v };
    // o coletor k é o da cópia k enquanto nenhum corpo foi recortado (as duas listas crescem juntas)
    let visivel = |k: usize| st.view_doc.collectors.len() != st.instancias.len() || st.view_doc.collectors.get(k).is_none_or(|c| c.visible);
    let mut com_copia = vec![false; vista.bodies.len()];
    for (k, i) in st.instancias.iter().enumerate() {
        if let Some(f) = com_copia.get_mut(i.corpo) {
            *f = true;
        }
        if visivel(k) {
            v.push((i.prototipo.fina(), i.posicao));
        }
    }
    for (b, _) in vista.bodies.iter().zip(&com_copia).filter(|(_, c)| !**c) {
        v.push((&b.mesh, fenkra_montagem::Placement::default()));
    }
    v
}

/// Põe o item na montagem como peça livre (sem restrição), com um passo de desfazer só dele.
fn inserir_livre(st: &mut crate::asm_ui::AsmState, est: &mut Estado, fam: &Familia, t: &biblioteca::Tamanho, onde: PosicaoLivre) -> Aviso {
    let Some(dir) = st.dir() else { return Some((tr("Salve a montagem antes: as peças dos fixadores são gravadas na pasta dela").to_string(), true)) };
    let modificada = st.modified;
    let pai = grupo_de_destino(st);
    st.push_undo();
    match avulso::inserir_livre(&mut st.asm, &dir, biblioteca::publica(), &fam.id, &t.designacao, est.comprimento, est.modo, onde, pai) {
        Ok(r) => {
            st.solve();
            st.dirty = true;
            est.prefs.registrar(&fam.id, &t.designacao);
            salvar_prefs(&est.prefs);
            Some((format!("{} {} · {}", tr("Inserido como peça"), r.nome, tr("livre para posicionar")), false))
        }
        Err(e) => {
            if let Some(antes) = st.undo.pop() {
                st.asm = antes;
            }
            st.modified = modificada;
            st.dirty = true;
            Some((e, true))
        }
    }
}

/// Botão "Adicionar à montagem": o item entra solto no centro da vista, sem clique na viewport.
fn adicionar_agora(st: &mut crate::asm_ui::AsmState, est: &mut Estado, cam: &Camera) -> Aviso {
    let Some((fam, t)) = est.escolhida() else { return Some((tr("Escolha um item no catálogo antes de inserir").to_string(), true)) };
    let alvo = d3(cam.target);
    inserir_livre(st, est, fam, t, PosicaoLivre::Solto { origem: alvo })
}

/// Um quadro da inserção em ponto ou solto: o fantasma segue o cursor (no ponto da superfície apontada,
/// ou no espaço) e o clique, ou soltar o arrasto da tabela, põe o item como peça livre.
#[allow(clippy::too_many_arguments)]
fn quadro_livre(st: &mut crate::asm_ui::AsmState, est: &mut Estado, ui: &mut egui::Ui, rect: Rect, cam: &Camera, ins: Insercao, fam: &'static Familia, t: &'static biblioteca::Tamanho) -> Aviso {
    let ctx = ui.ctx().clone();
    let pos = ctx.input(|i| i.pointer.latest_pos());
    let sobre_viewport = pos.is_some_and(|p| rect.contains(p) && ctx.layer_id_at(p).is_some_and(|l| l.order == egui::Order::Background));
    let solto = ctx.input(|i| i.pointer.primary_released());
    let clicou = ctx.input(|i| i.pointer.primary_clicked());
    let alt = ctx.input(|i| i.modifiers.alt);
    let demo = est.cursor_demo.map(|f| rect.min + egui::vec2(rect.width() * f[0], rect.height() * f[1]));
    let Some(pos) = demo.or(pos.filter(|_| sobre_viewport)) else {
        if ins.arrastando && solto {
            est.inserindo = None;
        }
        return None;
    };
    let (o, d) = cam.ray(rect, pos);
    let parafuso = fam.tipo.atravessa_furo();
    let comp = comprimento_fora_do_furo(est, fam, t);
    let malha = malha_do_item(&fam.id, &t.designacao, comp);
    let painter = ui.painter_at(rect);
    let acento = crate::theme::accent(&ctx.global_style().visuals);
    let aviso_cor = crate::theme::st_warn();
    let nome = format!("{}{}", t.designacao, comp.map(|l| format!("x{}", fmt(l))).unwrap_or_default());
    let onde: Option<PosicaoLivre> = match est.alvo {
        Alvo::Ponto => avulso::ponto_na_cena(malhas_da_cena(st), d3(o), d3(d)).map(|(ponto, normal)| PosicaoLivre::Ponto { ponto, normal }),
        _ => Some(PosicaoLivre::Solto { origem: fenkra_montagem::insercao::ponto_do_cursor(d3(o), d3(d), d3(cam.target), cam.dist as f64) }),
    };
    if let Some(m) = &malha {
        match onde {
            Some(PosicaoLivre::Ponto { ponto, normal }) => {
                let u = [-normal[0], -normal[1], -normal[2]];
                fantasma(&painter, rect, cam, m, avulso::posicao_do_item(parafuso, ponto, u), acento, 0.4);
                if let Some(base) = cam.project(rect, v3(ponto)) {
                    painter.circle_filled(base, 3.5, acento);
                    painter.text(base + egui::vec2(16., -18.), egui::Align2::LEFT_BOTTOM, format!("{nome} → {}", tr("ponto")), egui::FontId::proportional(12.), acento);
                }
            }
            Some(PosicaoLivre::Solto { origem }) => {
                let pl = fenkra_montagem::Placement { origin: origem, ..Default::default() };
                fantasma(&painter, rect, cam, m, pl, acento, 0.4);
                painter.text(pos + egui::vec2(16., -18.), egui::Align2::LEFT_BOTTOM, format!("{nome} → {}", tr("solto")), egui::FontId::proportional(12.), acento);
            }
            None => {
                // nenhuma superfície sob o cursor: marca âmbar, sem fantasma
                painter.circle_stroke(pos, 7., Stroke::new(1.5, aviso_cor));
                painter.text(pos + egui::vec2(16., -18.), egui::Align2::LEFT_BOTTOM, tr("aponte uma superfície"), egui::FontId::proportional(12.), aviso_cor);
            }
        }
    }
    ctx.request_repaint();
    let decide = if ins.arrastando { solto } else { clicou };
    if !decide {
        return None;
    }
    if ins.arrastando {
        est.inserindo = None;
    }
    if alt {
        est.foco_ficha = true;
        est.inserindo = None;
        return Some((tr("Ficha aberta no painel Biblioteca").to_string(), false));
    }
    let Some(onde) = onde else { return Some((tr("Solte o item sobre uma superfície de peça").to_string(), true)) };
    let aviso = inserir_livre(st, est, fam, t, onde);
    // arrasto da tabela: o comando acabou, a peça fica selecionada para o manipulador posicionar
    if ins.arrastando && aviso.as_ref().is_some_and(|a| !a.1) {
        st.sel = st.asm.nodes.last().map(|n| n.id);
    }
    aviso
}

// ---------------------------------------------------------------- candidatos do preenchimento (B248)

/// Fantasma translúcido do parafuso de cada pilha candidata, com o rótulo (M6x35). Clicar no
/// fantasma tira a pilha da lista; clicar na marca da pilha tirada a devolve.
pub fn candidatos_overlay(app: &mut App, ui: &mut egui::Ui, rect: Rect, cam: &Camera) {
    if app.settings.bench != crate::settings::Bench::Montagem {
        return;
    }
    let Some(f) = app.asm.as_mut().and_then(|s| s.fasteners_open.as_mut()) else { return };
    let painter = ui.painter_at(rect);
    let acento = crate::theme::accent(&ui.ctx().global_style().visuals);
    let cinza = ui.visuals().weak_text_color();
    let mut alternar: Option<String> = None;
    for (i, c) in f.candidatos.iter().enumerate() {
        match &c.estado {
            EstadoFuro::Entra { tamanho, comprimento, ponta, .. } => {
                let (Some(a), Some(b)) = (cam.project(rect, v3(c.cabeca)), cam.project(rect, v3(*ponta))) else { continue };
                if c.excluido {
                    painter.circle_stroke(a, 10., Stroke::new(1.5, cinza));
                    painter.text(a + egui::vec2(12., -10.), egui::Align2::LEFT_BOTTOM, format!("{} {}", c.rotulo, tr("(fora da lista)")), egui::FontId::proportional(11.), cinza);
                } else if let Some(m) = malha_do_item(&f.op.familia_parafuso, tamanho, Some(*comprimento)) {
                    fantasma(&painter, rect, cam, &m, avulso::posicao_do_item(true, c.cabeca, c.sentido), acento, 0.38);
                    let nome = c.componentes.join(" + ");
                    painter.text(a + egui::vec2(12., -12.), egui::Align2::LEFT_BOTTOM, format!("{} · {nome}", c.rotulo), egui::FontId::proportional(12.), acento);
                }
                let area = Rect::from_two_pos(a, b).expand(11.);
                let r = ui.interact(area, egui::Id::unique(("candidato", i)), Sense::click());
                if r.clicked() {
                    alternar = Some(c.chave.clone());
                }
                r.on_hover_text(tr(if c.excluido { "Clique para devolver esta pilha à lista." } else { "Clique no fantasma para tirar esta pilha da lista." }));
            }
            _ => {}
        }
    }
    if let Some(chave) = alternar {
        match f.op.excluidas.iter().position(|k| *k == chave) {
            Some(i) => {
                f.op.excluidas.remove(i);
            }
            None => f.op.excluidas.push(chave),
        }
    }
}

/// Texto de uma linha da lista de candidatos da janela de fixadores.
pub fn texto_candidato(c: &Candidato) -> (String, Color32) {
    let nome = c.componentes.join(" + ");
    match &c.estado {
        EstadoFuro::Entra { .. } if c.excluido => (format!("{}  {nome}  — {}", c.rotulo, tr("fora da lista")), Color32::GRAY),
        EstadoFuro::Entra { .. } => (format!("{}  {nome}", c.rotulo), crate::theme::st_ok()),
        EstadoFuro::JaTem => (format!("{}  — {}", nome, tr("já tem fixador")), Color32::GRAY),
        EstadoFuro::SemTamanho(m) => (m.split_once(": ").map(|(_, r)| r.to_string()).unwrap_or_else(|| m.clone()), crate::theme::st_warn()),
        EstadoFuro::Excluido => (nome, Color32::GRAY),
    }
}

/// `FENKRA_DEMO_ASM=biblioteca[_inserido|_ponto|_solto]`: o painel aberto com o parafuso M6 escolhido.
/// Use `FENKRA_DEMO_PAINEL=rolar:x,y,dy` para rolar o painel. `_inserido` já traz um item em cada modo
/// (furo, ponto e solto); `_ponto` e `_solto` armam o modo com o cursor parado, para ver o fantasma.
pub fn demo(st: &mut crate::asm_ui::AsmState, nome: &str) {
    let mut est = Estado::default();
    est.escolher("iso4762", Some("M6"));
    est.comprimento = Some(16.);
    est.fontes_abertas_demo = nome == "biblioteca_fontes";
    // B241: `FENKRA_DEMO_BUSCA=<texto>` já escreve a busca do navegador, como a pessoa digitaria
    if let Ok(b) = std::env::var("FENKRA_DEMO_BUSCA") {
        est.filtro.busca = b;
    }
    match nome {
        "biblioteca_ponto" | "biblioteca_solto" => {
            est.alvo = if nome.ends_with("ponto") { Alvo::Ponto } else { Alvo::Solto };
            est.inserindo = Some(Insercao { arrastando: false });
            est.cursor_demo = Some(if nome.ends_with("ponto") { [0.541, 0.538] } else { [0.62, 0.55] });
        }
        "biblioteca_inserido" => {
            if let Some(dir) = st.dir() {
                let lib = biblioteca::publica();
                let mut erros = vec![];
                // em furo: o primeiro furo da montagem a que o M6 serve, se houver
                let furos = avulso::furos_do_mundo(&st.asm, Some(&dir), &mut st.cache, &[]);
                let (fam, t) = (biblioteca::familia("iso4762"), biblioteca::familia("iso4762").and_then(|x| x.tamanho("M6")));
                if let (Some(fam), Some(t)) = (fam, t) {
                    if let Some(f) = furos.into_iter().find(|f| avulso::casa(fam, t, f)) {
                        let p = ParamAvulso { familia: "iso4762".into(), tamanho: "M6".into(), comprimento: None, furo: f, inverter: false, modo: est.modo };
                        let _ = avulso::inserir_avulso(&mut st.asm, &dir, lib, &p);
                    }
                }
                // em ponto: no alto da primeira peça, de cima para baixo
                let (_, vista) = fenkra_montagem::view::compose(&st.asm, Some(&dir), &mut st.cache, &mut erros);
                if let Some((ponto, normal)) = avulso::ponto_na_cena(vista.bodies.iter().map(|b| (&b.mesh, fenkra_montagem::Placement::default())), [18., 8., 300.], [0., 0., -1.]) {
                    let _ = avulso::inserir_livre(&mut st.asm, &dir, lib, "iso7089", "M6", None, est.modo, PosicaoLivre::Ponto { ponto, normal }, None);
                }
                // solto: no espaço, ao lado
                let _ = avulso::inserir_livre(&mut st.asm, &dir, lib, "iso4762", "M8", Some(30.), est.modo, PosicaoLivre::Solto { origem: [90., -60., 40.] }, None);
                st.solve();
                st.dirty = true;
            }
        }
        // B396: ficha do rolamento 6010 aberta e os dois do kart (50×80 e 25×47) soltos ao lado
        "biblioteca_rolamento" => {
            est.escolher("iso15_60", Some("6010"));
            est.comprimento = None;
            if let Some(dir) = st.dir() {
                let lib = biblioteca::publica();
                let _ = avulso::inserir_livre(&mut st.asm, &dir, lib, "iso15_60", "6010", None, est.modo, PosicaoLivre::Solto { origem: [110., -60., 40.] }, None);
                let _ = avulso::inserir_livre(&mut st.asm, &dir, lib, "iso15_60", "6005", None, est.modo, PosicaoLivre::Solto { origem: [10., -150., 40.] }, None);
                st.solve();
                st.dirty = true;
            }
        }
        // `biblioteca_ficha_<id>`: a ficha da família aberta no tamanho do meio e esse item solto na
        // montagem (B51, B398). `biblioteca_b51` larga as famílias novas lado a lado.
        n if n.starts_with("biblioteca_ficha_") || n == "biblioteca_b51" || n == "biblioteca_b44" || n == "biblioteca_b46" || n == "biblioteca_b43" || n == "biblioteca_b45" => {
            let lib = biblioteca::publica();
            let ids: Vec<&str> = match n.strip_prefix("biblioteca_ficha_") {
                Some(id) => vec![id],
                // B44 e B45: rolamentos novos e o cilindro ISO 15552, com a ficha do cilindro aberta
                None if n == "biblioteca_b44" => vec!["iso15552", "iso15_68", "iso15_69", "iso15_160", "iso15_72", "iso15_nu2", "iso355_302"],
                None if n == "biblioteca_b46" => vec!["hiwin_hgr"],
                // B43: rebites, porcas e parafuso aeronáuticos, com a ficha do rebite universal aberta
                // B45: minicilindro ISO 6432 e os ISO 15552 grandes, com a ficha do minicilindro aberta
                None if n == "biblioteca_b45" => vec!["iso6432", "iso15552"],
                None if n == "biblioteca_b43" => vec!["an470", "an426", "an310", "an315", "ms21042", "an3"],
                None => vec!["iso10642", "iso7045", "iso7379", "din938", "din6923", "din6798a", "din25201", "asme_b18_3"],
            };
            let meio = |id: &str| lib.familia(id).and_then(|f| f.tamanhos.get(f.tamanhos.len() / 2)).map(|t| t.designacao.clone());
            if let Some(t) = meio(ids[0]) {
                est.escolher(ids[0], Some(t.as_str()));
                est.comprimento = None;
            }
            if let Some(dir) = st.dir() {
                for (i, id) in ids.iter().enumerate() {
                    if let Some(t) = meio(id) {
                        let origem = [i as f64 * 40. + 20., -70., 40.];
                        let _ = avulso::inserir_livre(&mut st.asm, &dir, lib, id, &t, None, est.modo, PosicaoLivre::Solto { origem }, None);
                    }
                }
                st.solve();
                st.dirty = true;
            }
        }
        _ => {}
    }
    st.catalogo = Some(est);
}

#[cfg(test)]
mod tests {
    use super::*;
    use fenkra_montagem::avulso::posicao_do_item;

    #[test]
    fn toda_familia_tem_malha_de_previa() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        for f in biblioteca::familias() {
            for t in [f.tamanhos.first().unwrap(), f.tamanhos.last().unwrap()] {
                let m = malha_do_item(&f.id, &t.designacao, None).unwrap_or_else(|| panic!("{} {} sem malha", f.id, t.designacao));
                assert!(!m.tris.is_empty() && m.raio > 0.5, "{} {}", f.id, t.designacao);
                assert_eq!(m.tris.len(), m.normais.len());
            }
        }
    }

    #[test]
    fn fantasma_do_parafuso_fica_na_pose_da_peca_inserida() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        // furo na face de cima (z = 10), entrando para baixo: cabeça acima da face, haste dentro do furo
        let m = malha_do_item("iso4762", "M6", Some(20.)).unwrap();
        let (entrada, u) = ([0., 0., 10.], [0., 0., -1.]);
        let (ponto, _) = no_mundo(posicao_do_item(true, entrada, u));
        let zs: Vec<f32> = m.tris.iter().flatten().map(|p| ponto(*p).z).collect();
        let (lo, hi) = (zs.iter().cloned().fold(f32::MAX, f32::min), zs.iter().cloned().fold(f32::MIN, f32::max));
        assert!((lo - (10. - 20.)).abs() < 0.05, "ponta da haste a 20 mm da face: {lo}");
        assert!(hi > 10. + 5. && hi < 10. + 7., "cabeça M6 tem 6 mm acima da face: {hi}");
        // a arruela fica do lado de fora da peça: o corpo dela sobe a partir da face
        let a = malha_do_item("iso7089", "M6", None).unwrap();
        let (ponto, _) = no_mundo(posicao_do_item(false, entrada, u));
        let zs: Vec<f32> = a.tris.iter().flatten().map(|p| ponto(*p).z).collect();
        assert!(zs.iter().all(|z| *z >= 10. - 0.01), "arruela abaixo da face");
    }

    #[test]
    fn escolher_a_familia_leva_ao_primeiro_tamanho_e_registra_no_historico() {
        let mut e = Estado::default();
        e.escolher("iso4017", Some("M8"));
        assert_eq!((e.familia.as_deref(), e.tamanho.as_deref()), (Some("iso4017"), Some("M8")));
        e.comprimento = Some(30.);
        e.escolher("iso4762", None);
        assert_eq!(e.tamanho.as_deref(), Some("M3"), "sem tamanho pedido, o primeiro da família");
        assert_eq!(e.comprimento, None, "o comprimento não atravessa famílias");
        assert_eq!(e.prefs.recentes[0].familia, "iso4762");
        assert_eq!(e.prefs.recentes.len(), 2);
        // desconhecida: nada muda
        e.escolher("naoexiste", None);
        assert_eq!(e.familia.as_deref(), Some("iso4762"));
    }

    // ---------------------------------------------------------------- B374: os três modos pela interface

    use crate::asm_ui::AsmState;
    use fenkra_montagem::{Assembly, NodeKind, Placement};

    fn montagem(nome: &str) -> AsmState {
        let d = std::env::temp_dir().join(format!("fenkra_biblioteca_{nome}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).expect("pasta");
        std::fs::write(d.join("placa.fkp"), fenkra_solidos::example_document().to_json()).expect("peça");
        let mut a = Assembly::new("t");
        a.add_component("Placa", NodeKind::Part { path: "placa.fkp".into() }, Placement::default(), None);
        let mut st = AsmState::new(a, Some(d.join("conjunto.fka")));
        st.refresh();
        st
    }

    fn pecas(st: &AsmState) -> usize {
        st.asm.nodes.iter().filter(|n| !matches!(n.kind, NodeKind::Group)).count()
    }

    fn tela() -> Rect {
        Rect::from_min_size(egui::pos2(0., 0.), egui::vec2(1000., 700.))
    }

    /// Passa um quadro da viewport com a câmera padrão; devolve o aviso que a inserção deu.
    fn quadro(ctx: &egui::Context, st: &mut AsmState, est: &mut Estado, evs: Vec<egui::Event>) -> Aviso {
        quadro_com(ctx, st, est, evs, &Camera::default())
    }

    fn quadro_com(ctx: &egui::Context, st: &mut AsmState, est: &mut Estado, evs: Vec<egui::Event>, cam: &Camera) -> Aviso {
        let raw = egui::RawInput { screen_rect: Some(tela()), events: evs, ..Default::default() };
        let mut aviso = None;
        let mut saida = ctx.run_ui(raw, |ui| {
            aviso = quadro_da_insercao(st, est, ui, tela(), cam);
        });
        saida.textures_delta.clear();
        aviso
    }

    fn clicar(ctx: &egui::Context, st: &mut AsmState, est: &mut Estado, p: Pos2) -> Aviso {
        clicar_com(ctx, st, est, p, &Camera::default())
    }

    fn clicar_com(ctx: &egui::Context, st: &mut AsmState, est: &mut Estado, p: Pos2, cam: &Camera) -> Aviso {
        let botao = |pressed| egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() };
        quadro_com(ctx, st, est, vec![egui::Event::PointerMoved(p)], cam);
        quadro_com(ctx, st, est, vec![botao(true)], cam);
        let aviso = quadro_com(ctx, st, est, vec![botao(false)], cam);
        quadro_com(ctx, st, est, vec![], cam);
        aviso
    }

    fn armar(alvo: Alvo, familia: &str, tamanho: &str, comprimento: Option<f64>) -> Estado {
        let mut est = Estado::default();
        est.escolher(familia, Some(tamanho));
        est.comprimento = comprimento;
        est.alvo = alvo;
        est.inserindo = Some(Insercao { arrastando: false });
        est
    }

    #[test]
    fn em_ponto_o_clique_pousa_o_item_na_superficie_apontada_sem_restricao() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let mut st = montagem("ponto");
        let mut est = armar(Alvo::Ponto, "iso4762", "M6", Some(20.));
        let ctx = egui::Context::default();
        quadro(&ctx, &mut st, &mut est, vec![]);
        let cam = Camera::default();
        // o centro da vista fica sobre a placa de exemplo: o raio da câmera acerta uma face
        let p = tela().center();
        let (o, d) = cam.ray(tela(), p);
        let (ponto, normal) = avulso::ponto_na_cena(malhas_da_cena(&st), d3(o), d3(d)).expect("a placa está no centro da vista");
        let aviso = clicar(&ctx, &mut st, &mut est, p);
        assert!(aviso.as_ref().is_some_and(|a| !a.1), "{aviso:?}");
        assert_eq!(pecas(&st), 2, "a placa e o parafuso");
        assert!(st.asm.mates.is_empty(), "em ponto não nasce restrição");
        let novo = st.asm.nodes.last().unwrap();
        assert!(novo.parent.is_none(), "sem grupo selecionado, entra na raiz");
        assert!(novo.placement.origin.iter().zip(ponto).all(|(a, b)| (a - b).abs() < 1e-6), "origem no ponto apontado: {:?} {ponto:?}", novo.placement.origin);
        let z = novo.placement.dir([0., 0., 1.]);
        assert!(z.iter().zip(normal).all(|(a, b)| (a - b).abs() < 1e-6), "eixo do parafuso na normal da face: {z:?} {normal:?}");
        assert!(est.inserindo.is_some(), "armado: o próximo clique insere outro");
        // um passo de desfazer só dele
        assert_eq!(st.undo.len(), 1);
        st.asm = st.undo.pop().unwrap();
        assert_eq!(pecas(&st), 1);
    }

    #[test]
    fn em_ponto_clicar_no_vazio_avisa_e_nao_insere() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let mut st = montagem("ponto_vazio");
        let mut est = armar(Alvo::Ponto, "iso4762", "M6", None);
        let ctx = egui::Context::default();
        quadro(&ctx, &mut st, &mut est, vec![]);
        let aviso = clicar(&ctx, &mut st, &mut est, egui::pos2(6., 6.));
        assert!(aviso.is_some_and(|a| a.1), "erro: nenhuma superfície sob o cursor");
        assert_eq!(pecas(&st), 1);
        assert!(st.undo.is_empty());
    }

    #[test]
    fn solto_o_clique_larga_o_item_no_espaco_com_os_eixos_do_mundo_dentro_do_grupo_selecionado() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let mut st = montagem("solto");
        let grupo = st.asm.add_group("Ferragens", None);
        st.sel = Some(grupo);
        let mut est = armar(Alvo::Solto, "iso7089", "M6", None);
        let ctx = egui::Context::default();
        quadro(&ctx, &mut st, &mut est, vec![]);
        let cam = Camera::default();
        let p = egui::pos2(300., 250.);
        let (o, d) = cam.ray(tela(), p);
        let esperado = fenkra_montagem::insercao::ponto_do_cursor(d3(o), d3(d), d3(cam.target), cam.dist as f64);
        let aviso = clicar(&ctx, &mut st, &mut est, p);
        assert!(aviso.as_ref().is_some_and(|a| !a.1), "{aviso:?}");
        assert_eq!(pecas(&st), 2);
        let novo = st.asm.nodes.last().unwrap();
        assert_eq!(novo.parent, Some(grupo));
        assert!(novo.placement.origin.iter().zip(esperado).all(|(a, b)| (a - b).abs() < 1e-6), "{:?} {esperado:?}", novo.placement.origin);
        assert_eq!(novo.placement.dir([0., 0., 1.]), [0., 0., 1.]);
        assert!(st.asm.mates.is_empty());
        assert!(st.dir().unwrap().join("fixadores").join("iso7089_M6.fkp").exists(), "a peça da biblioteca foi gravada");
    }

    #[test]
    fn em_furo_continua_restrito_e_o_botao_adiciona_solto_no_centro_da_vista() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let mut st = montagem("furo");
        let dir = st.dir().unwrap();
        let furos = avulso::furos_do_mundo(&st.asm, Some(&dir), &mut st.cache, &[]);
        let fam = biblioteca::familia("iso4762").unwrap();
        let Some((furo, tam)) = furos.iter().find_map(|f| avulso::tamanhos_que_casam(fam, f).first().map(|t| (f.clone(), t.designacao.clone()))) else { panic!("a placa de exemplo tem furo que casa com algum tamanho: {furos:?}") };
        let mut est = armar(Alvo::Furo, "iso4762", &tam, None);
        let ctx = egui::Context::default();
        quadro(&ctx, &mut st, &mut est, vec![]);
        let cam = Camera::default();
        let boca = cam.project(tela(), v3(furo.boca)).expect("a boca do furo aparece");
        let aviso = clicar(&ctx, &mut st, &mut est, boca);
        assert!(aviso.as_ref().is_some_and(|a| !a.1), "{aviso:?}");
        assert_eq!(st.asm.mates.len(), 3, "concêntrico, trava de giro e contato");
        assert_eq!(st.comp_dof.get(&st.asm.nodes.last().unwrap().id).copied().unwrap_or(0), 0, "restrito: sem graus de liberdade");
        // "Adicionar à montagem": peça livre no centro da vista, sem clique
        est.inserindo = None;
        est.adicionar_agora = true;
        let aviso = adicionar_agora(&mut st, &mut est, &cam);
        assert!(aviso.is_some_and(|a| !a.1));
        assert_eq!(pecas(&st), 3);
        assert_eq!(st.asm.mates.len(), 3, "a peça livre não ganha restrição");
        let livre = st.asm.nodes.last().unwrap();
        assert!(livre.placement.origin.iter().zip(d3(cam.target)).all(|(a, b)| (a - b).abs() < 1e-6));
    }

    #[test]
    fn o_conteudo_do_painel_nao_se_encavala_sob_o_teto_de_altura_da_janela() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        // o painel limita o corpo à altura útil: simula um teto bem baixo, como o de uma janela pequena
        let altura = |teto: f32| {
            let mut est = Estado::default();
            est.escolher("iso4762", Some("M6"));
            est.fontes_abertas_demo = true;
            let ctx = egui::Context::default();
            let mut h = 0.;
            for _ in 0..3 {
                let raw = egui::RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(500., 2000.))), ..Default::default() };
                let mut saida = ctx.run_ui(raw, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.set_max_height(teto);
                        conteudo(ui, &mut est);
                        h = ui.min_rect().height();
                    });
                });
                saida.textures_delta.clear();
            }
            h
        };
        let (livre, baixo) = (altura(5000.), altura(150.));
        assert!(livre > 450., "a ficha inteira tem bem mais que o teto simulado: {livre}");
        assert!((livre - baixo).abs() < 1., "o teto de altura não pode apertar o conteúdo: livre {livre}, com teto {baixo}");
    }

    #[test]
    fn em_ponto_acha_a_segunda_copia_mesmo_quando_a_vista_so_traz_a_malha_do_prototipo() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let mut st = montagem("copias");
        st.asm.add_component("Placa de cima", NodeKind::Part { path: "placa.fkp".into() }, Placement { origin: [140., 20., 30.], ..Default::default() }, None);
        st.dirty = true;
        st.refresh();
        assert_eq!(st.instancias.len(), 2);
        // a composição em segundo plano entrega um corpo só para as duas cópias: a malha do protótipo
        st.view.as_mut().unwrap().bodies.truncate(1);
        let mut est = armar(Alvo::Ponto, "iso7089", "M6", None);
        let ctx = egui::Context::default();
        // câmera que enquadra as duas placas, como a do app
        let cam = Camera { target: Vec3::new(70., 10., 20.), dist: 450., ..Camera::default() };
        quadro_com(&ctx, &mut st, &mut est, vec![], &cam);
        // pontos da face de cima da segunda placa (z = 40), fora do rasgo e dos furos de canto
        let candidatos = [[140., -10., 40.], [140., 50., 40.], [100., 20., 40.], [180., 20., 40.], [110., 45., 40.]];
        let alvo = candidatos.iter().find_map(|c| {
            let px = cam.project(tela(), v3(*c))?;
            let (o, d) = cam.ray(tela(), px);
            let (pt, _) = avulso::ponto_na_cena(malhas_da_cena(&st), d3(o), d3(d))?;
            ((pt[2] - 40.).abs() < 0.01).then_some((px, pt))
        });
        let (px, pt) = alvo.expect("algum ponto da segunda placa aparece sem nada na frente");
        let aviso = clicar_com(&ctx, &mut st, &mut est, px, &cam);
        assert!(aviso.as_ref().is_some_and(|a| !a.1), "{aviso:?}");
        let novo = st.asm.nodes.last().unwrap();
        assert!(novo.placement.origin.iter().zip(pt).all(|(a, b)| (a - b).abs() < 1e-6) && (novo.placement.origin[2] - 40.).abs() < 0.01, "{:?} {pt:?}", novo.placement.origin);
        // sem a malha da cópia, o raio só veria a placa da origem: a segunda placa não existiria para ele
        assert!(avulso::ponto_na_cena(st.view.as_ref().unwrap().bodies.iter().map(|b| (&b.mesh, fenkra_montagem::Placement::default())), d3(cam.ray(tela(), px).0), d3(cam.ray(tela(), px).1)).is_none_or(|(p, _)| (p[2] - 40.).abs() > 0.01));
    }
}
