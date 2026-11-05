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
use fenkra_montagem::avulso::{self, FuroAlvo, ModoInsercao, ParamAvulso};
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
fn fantasma(painter: &egui::Painter, rect: Rect, cam: &Camera, m: &Malha, pl: fenkra_montagem::Placement, cor: Color32, alfa: f32) {
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
    pub prefs: Preferencias,
    prefs_lidas: bool,
    pub yaw: f32,
    pub pitch: f32,
    pub modo: ModoInsercao,
    pub inserindo: Option<Insercao>,
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
            prefs: Preferencias::default(),
            prefs_lidas: false,
            yaw: -0.6,
            pitch: 0.5,
            modo: ModoInsercao::Padrao,
            inserindo: None,
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
        chip(ui, &mut p, "Polegada", "Só famílias em polegada. A biblioteca pública ainda não traz nenhuma: os dados só entram conferidos em duas fontes.");
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
                arrasto.on_hover_text(tr("Arraste para a viewport: o item salta para o furo que casa."));
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
    ui.collapsing(tr("Fontes e licença"), |ui| {
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
    ui.horizontal(|ui| {
        let ativo = est.inserindo.is_some();
        if ui.selectable_label(ativo, format!("⌖ {}", tr("Inserir em furo"))).on_hover_text(tr("Clique e depois num furo da viewport. O fantasma salta para o furo que casa; Tab inverte o sentido; Alt ao clicar abre a ficha; Esc sai.")).clicked() {
            est.inserindo = if ativo { None } else { Some(Insercao { arrastando: false }) };
            est.furos = None;
        }
    });
    if est.inserindo.is_some() {
        ui.label(RichText::new(tr("Tab inverte o sentido · Alt abre a ficha · Esc sai")).small().weak());
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
    let Some(m) = malha_do_item(&fam.id, tamanho, if fam.tipo.e_parafuso() { est.comprimento } else { None }) else {
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
    let (rect, _) = ui.allocate_exact_size(egui::vec2(190., 150.), Sense::hover());
    let p = ui.painter_at(rect);
    let traco = Stroke::new(1.5, ui.visuals().text_color());
    let fino = Stroke::new(1., ui.visuals().weak_text_color());
    let fonte = egui::FontId::proportional(11.);
    let cor_texto = crate::theme::accent(ui.visuals());
    let d = l.d_nominal;
    let e = |s: f64| s * 1.1547;
    let mut notas: Vec<(String, f64)> = vec![];
    match ficha.tipo {
        TipoFixador::ParafusoAllen | TipoFixador::ParafusoSextavado => {
            let k = valor("k").unwrap_or(d * 0.6);
            let larg = valor("dk").unwrap_or_else(|| e(valor("s").unwrap_or(d * 1.6)));
            let ls = comprimento.or(l.comprimentos.iter().copied().find(|x| *x >= 4. * d)).or(l.comprimentos.last().copied()).unwrap_or(d * 4.);
            let esc = ((rect.height() - 40.) / (k + ls) as f32).min((rect.width() * 0.5 - 20.) / larg as f32);
            let cx = rect.left() + 70.;
            let topo = rect.top() + 14.;
            let (hw, hh, sw, sl) = (larg as f32 * esc, k as f32 * esc, d as f32 * esc, ls as f32 * esc);
            let cabeca = Rect::from_min_size(Pos2::new(cx - hw / 2., topo), egui::vec2(hw, hh));
            let haste = Rect::from_min_size(Pos2::new(cx - sw / 2., topo + hh), egui::vec2(sw, sl));
            p.rect_stroke(cabeca, 1., traco, egui::StrokeKind::Middle);
            p.rect_stroke(haste, 0., traco, egui::StrokeKind::Middle);
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
    let aviso = if est.inserindo.is_some() { quadro_da_insercao(st, &mut est, ui, rect, cam) } else { None };
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
    let parafuso = fam.tipo.e_parafuso();
    // comprimento do que se desenha: o pedido, ou o que o furo pede
    let comp = if parafuso { est.comprimento.or_else(|| melhor.as_ref().and_then(|m| avulso::comprimento_sugerido(fam, t, m.1))).or_else(|| fam.comprimentos_de(t).first().copied()) } else { None };
    let malha = malha_do_item(&fam.id, &t.designacao, comp);
    let mut sobre_alca = false;
    let mut mudou: Option<f64> = None;
    match (&melhor, &malha) {
        (Some((_, f, (entrada, u))), Some(m)) => {
            fantasma(&painter, rect, cam, m, avulso::posicao_do_item(parafuso, *entrada, *u), acento, 0.4);
            let (rotulo, base) = (format!("{}{} → {} Ø{}", t.designacao, comp.map(|l| format!("x{}", fmt(l))).unwrap_or_default(), f.nome, fmt(f.diametro)), cam.project(rect, v3(*entrada)).unwrap_or(pos));
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
}
