// SPDX-License-Identifier: LGPL-2.1-or-later
//! Árvore navegável do produto em forma de grafo (B364), além da lista em linhas do painel do
//! projeto.
//!
//! Como funciona:
//! 1. o grafo vem de `Project::arvore_produto` (um nó por arquivo, arestas com quantidade). A tela o
//!    desdobra em árvore: cada **ocorrência** é identificada pelo caminho de nós desde a raiz, então
//!    a mesma peça em duas montagens aparece nas duas, e abrir uma não abre a outra;
//! 2. o que aparece aberto é decidido por "níveis" (abre tudo até o nível N) mais as exceções que o
//!    clique grava (abrir um nó além de N, ou fechar um antes). Montagem grande não trava: só a parte
//!    aberta é posta na tela, com um teto de caixas;
//! 3. cada nó tem miniatura e PN. A miniatura sai do pacote `.fkv` do nó (peça: o da versão atual;
//!    montagem: o da assinatura), rasterizada numa linha de execução própria; quem não tem pacote
//!    ganha um quando se pede "Gerar miniaturas", e o pacote fica para o visualizador (B339);
//! 4. ao passar o mouse o nó cresce num cartão com PN, nome, estado, revisão e onde é usado. A cor
//!    da borda é a regra do estado: liberado (peça liberada, ou montagem com todas as peças
//!    liberadas) ou não; tudo pelas cores do tema;
//! 5. clique abre ou fecha o nível; duplo clique entra na montagem (a trilha no topo volta) ou abre a
//!    peça; o botão direito tem abrir, entrar e ver no navegador.

use super::*;
use fenkra_pacote::miniatura::{de_pacote, Miniatura};
use fenkra_project::arvore_produto::{ArvoreProduto, EstadoNo, NoProduto, TipoNo};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::path::Path;
use std::sync::Mutex;

const LARG: f32 = 86.;
const ALT: f32 = 98.;
const MINI: f32 = 62.;
const ESP_X: f32 = 16.;
const ESP_Y: f32 = 56.;
/// Teto de caixas postas na tela de uma vez.
const MAX_CAIXAS: usize = 3000;
/// Lado da miniatura em pixels.
const LADO: u32 = 96;

pub(super) struct ArvoreUi {
    raiz: PathBuf,
    arvore: ArvoreProduto,
    /// trilha: caminho do nó em foco desde uma raiz; vazio mostra todas as raízes
    foco: Vec<usize>,
    niveis: usize,
    abertos: HashSet<Vec<usize>>,
    fechados: HashSet<Vec<usize>>,
    desloc: egui::Vec2,
    zoom: f32,
    enquadrar: bool,
    busca: String,
    texturas: HashMap<String, egui::TextureHandle>,
    pedidas: HashSet<String>,
    falhas: HashMap<String, String>,
    /// força o cartão de um nó (PN), para captura sem mouse
    cartao_fixo: Option<String>,
}

struct Pronta {
    raiz: PathBuf,
    no: NoProduto,
    /// pacote gerado agora, para gravar no projeto
    novo: Option<Vec<u8>>,
    mini: Result<Miniatura, String>,
}

enum Trabalho {
    Ler(PathBuf),
    Peca(Document),
    Arquivo(PathBuf),
}

static ESTADO: Mutex<Option<ArvoreUi>> = Mutex::new(None);
static PRONTAS: Mutex<Vec<Pronta>> = Mutex::new(Vec::new());
static ANDAMENTO: AtomicUsize = AtomicUsize::new(0);

/// Uma caixa posta na tela.
struct Caixa {
    caminho: Vec<usize>,
    no: usize,
    pos: egui::Pos2,
    quantidade: u32,
    filhos: usize,
    aberta: bool,
    ciclo: bool,
}

enum Acao {
    Alternar(Vec<usize>, bool),
    Entrar(Vec<usize>),
    Foco(usize),
    Abrir(usize),
    Ver(usize),
    Comparar(usize),
}

fn opcoes(p: &Project) -> fenkra_pacote::Opcoes {
    if p.opcoes_visualizacao().simplificada {
        fenkra_pacote::Opcoes { decimar: Some(0.3), capturas: false, ..Default::default() }
    } else {
        fenkra_pacote::Opcoes::default()
    }
}

impl ArvoreUi {
    fn novo(p: &Project) -> ArvoreUi {
        ArvoreUi {
            raiz: p.root.clone(),
            arvore: p.arvore_produto(),
            foco: vec![],
            niveis: 2,
            abertos: HashSet::new(),
            fechados: HashSet::new(),
            desloc: egui::Vec2::ZERO,
            zoom: 1.,
            enquadrar: true,
            busca: String::new(),
            texturas: HashMap::new(),
            pedidas: HashSet::new(),
            falhas: HashMap::new(),
            cartao_fixo: None,
        }
    }

    fn aberto(&self, caminho: &[usize]) -> bool {
        // o caminho começa no nó de topo desenhado (raiz ou foco): o nível é relativo a ele
        let ate_nivel = caminho.len() < self.niveis.max(1);
        (ate_nivel && !self.fechados.contains(caminho)) || self.abertos.contains(caminho)
    }

    /// Raízes desenhadas: as do produto, ou só o nó em foco.
    fn topo(&self) -> Vec<usize> {
        match self.foco.last() {
            Some(&n) => vec![n],
            None => self.arvore.raizes.clone(),
        }
    }

    /// Posições das caixas (em pontos, antes do zoom) e as ligações pai → filho entre elas.
    fn dispor(&self) -> (Vec<Caixa>, Vec<(usize, usize)>, bool) {
        let mut caixas = vec![];
        let mut ligacoes = vec![];
        let mut x = 0.;
        let mut cortado = false;
        for r in self.topo() {
            self.por_no(vec![r], 1, &mut x, &mut caixas, &mut ligacoes, &mut cortado);
            x += ESP_X * 2.;
        }
        (caixas, ligacoes, cortado)
    }

    #[allow(clippy::too_many_arguments)]
    fn por_no(&self, caminho: Vec<usize>, quantidade: u32, x: &mut f32, caixas: &mut Vec<Caixa>, ligacoes: &mut Vec<(usize, usize)>, cortado: &mut bool) -> f32 {
        let no = *caminho.last().unwrap();
        let ciclo = caminho[..caminho.len() - 1].contains(&no);
        let n = &self.arvore.nos[no];
        let aberta = !ciclo && !n.filhos.is_empty() && self.aberto(&caminho);
        let y = (caminho.len() - 1) as f32 * (ALT + ESP_Y);
        let eu = caixas.len();
        caixas.push(Caixa { caminho: caminho.clone(), no, pos: egui::pos2(0., y), quantidade, filhos: n.filhos.len(), aberta, ciclo });
        let centro = if aberta && caixas.len() < MAX_CAIXAS {
            let mut centros = vec![];
            for a in &n.filhos {
                if caixas.len() >= MAX_CAIXAS {
                    *cortado = true;
                    break;
                }
                let mut c = caminho.clone();
                c.push(a.filho);
                let filho = caixas.len();
                centros.push(self.por_no(c, a.quantidade, x, caixas, ligacoes, cortado));
                ligacoes.push((eu, filho));
            }
            match (centros.first(), centros.last()) {
                (Some(a), Some(b)) => (a + b) / 2.,
                _ => {
                    *x += LARG + ESP_X;
                    *x - ESP_X - LARG / 2.
                }
            }
        } else {
            if aberta {
                *cortado = true;
            }
            let c = *x + LARG / 2.;
            *x += LARG + ESP_X;
            c
        };
        caixas[eu].pos.x = centro - LARG / 2.;
        centro
    }

    fn achado(&self, n: &NoProduto) -> bool {
        let b = self.busca.trim().to_lowercase();
        !b.is_empty() && (n.pn.to_lowercase().contains(&b) || n.nome.to_lowercase().contains(&b))
    }
}

/// Cor da regra de estado, pelo tema.
fn cor_no(v: &egui::Visuals, a: &ArvoreProduto, i: usize) -> egui::Color32 {
    match a.nos[i].estado {
        EstadoNo::Fora => v.weak_text_color(),
        EstadoNo::Documento(fenkra_project::DocState::Obsoleto) => v.weak_text_color(),
        _ if a.liberado(i) => v.hyperlink_color,
        _ => v.warn_fg_color,
    }
}

fn texto_estado(a: &ArvoreProduto, i: usize) -> String {
    match a.nos[i].estado {
        EstadoNo::Documento(e) => tr(e.label()).to_string(),
        EstadoNo::Derivado { liberadas, total } if total > 0 && liberadas == total => tr("Liberado (todas as peças)").to_string(),
        EstadoNo::Derivado { liberadas, total } => format!("{} ({liberadas}/{total} {})", tr("Não liberado"), tr("peças liberadas")),
        EstadoNo::Fora => tr("Fora do projeto").to_string(),
    }
}

/// Cria os trabalhos de miniatura. `gerar` também cria o pacote de quem ainda não tem.
fn pedir(p: &Project, st: &mut ArvoreUi, gerar: bool) -> usize {
    let mut trabalhos = vec![];
    for n in &st.arvore.nos {
        if st.texturas.contains_key(&n.assinatura) || st.pedidas.contains(&n.assinatura) || (!gerar && st.falhas.contains_key(&n.assinatura)) || !n.existe {
            continue;
        }
        let t = match p.visualizacao_do_no(n) {
            Some(c) => Trabalho::Ler(c),
            None if !gerar => continue,
            None => match (n.id, &n.versao) {
                (Some(_), Some(h)) => match p.load_version(h) {
                    Ok(d) => Trabalho::Peca(d),
                    Err(e) => {
                        st.falhas.insert(n.assinatura.clone(), e.to_string());
                        continue;
                    }
                },
                _ => Trabalho::Arquivo(n.arquivo.clone()),
            },
        };
        st.pedidas.insert(n.assinatura.clone());
        st.falhas.remove(&n.assinatura);
        trabalhos.push((n.clone(), t));
    }
    let k = trabalhos.len();
    if k == 0 {
        return 0;
    }
    ANDAMENTO.fetch_add(k, Ordering::SeqCst);
    let (raiz, opc) = (p.root.clone(), opcoes(p));
    std::thread::spawn(move || {
        for (no, t) in trabalhos {
            let (novo, bytes) = match t {
                Trabalho::Ler(c) => (None, std::fs::read(&c).map_err(|e| e.to_string())),
                Trabalho::Peca(d) => match fenkra_pacote::de_peca(&d, &opc).and_then(|x| x.bytes()) {
                    Ok(b) => (Some(b.clone()), Ok(b)),
                    Err(e) => (None, Err(e)),
                },
                Trabalho::Arquivo(c) => match fenkra_pacote::de_arquivo(&c, &opc).and_then(|x| x.bytes()) {
                    Ok(b) => (Some(b.clone()), Ok(b)),
                    Err(e) => (None, Err(e)),
                },
            };
            let mini = bytes.and_then(|b| de_pacote(&b, LADO));
            PRONTAS.lock().unwrap_or_else(|e| e.into_inner()).push(Pronta { raiz: raiz.clone(), no, novo, mini });
            ANDAMENTO.fetch_sub(1, Ordering::SeqCst);
        }
    });
    k
}

impl App {
    /// Abre (ou atualiza) a janela da árvore do produto.
    pub(super) fn abrir_arvore_produto(&mut self) {
        let Some(p) = self.project.as_ref() else { return };
        let mut g = ESTADO.lock().unwrap_or_else(|e| e.into_inner());
        let mut st = ArvoreUi::novo(p);
        if let Some(velho) = g.take().filter(|v| v.raiz == p.root) {
            // atualizar mantém o que já foi visto e a navegação
            st.texturas = velho.texturas;
            st.niveis = velho.niveis;
            st.zoom = velho.zoom;
            st.desloc = velho.desloc;
            st.enquadrar = false;
            st.busca = velho.busca;
        }
        pedir(p, &mut st, p.opcoes_visualizacao().gerar_ao_salvar);
        *g = Some(st);
    }

    /// Desenvolvimento (`FENKRA_DEMO_PROJETO=arvore`): kart com subconjuntos, peças liberadas e em
    /// trabalho, a janela aberta e as miniaturas geradas. `FENKRA_DEMO_ARVORE_CARTAO=<PN>` fixa o
    /// cartão de um nó, como se o mouse estivesse em cima; `FENKRA_DEMO_ARVORE_FOCO=<nome>` entra
    /// numa montagem.
    pub(super) fn demo_arvore_produto(&mut self) {
        let pasta = std::env::temp_dir().join(format!("fenkra-demo-arvore-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&pasta);
        let Ok(p) = montar_demo(&pasta) else { return };
        self.project = Some(p);
        self.left = LeftTab::Projeto;
        self.started = true;
        self.abrir_arvore_produto();
        let mut g = ESTADO.lock().unwrap_or_else(|e| e.into_inner());
        if let (Some(st), Some(p)) = (g.as_mut(), self.project.as_ref()) {
            st.niveis = 3;
            pedir(p, st, true);
            st.cartao_fixo = std::env::var("FENKRA_DEMO_ARVORE_CARTAO").ok();
            if let Ok(nome) = std::env::var("FENKRA_DEMO_ARVORE_FOCO") {
                if let Some(i) = st.arvore.nos.iter().position(|n| n.nome == nome) {
                    let r = st.arvore.raizes.iter().copied().find(|&r| r == i || st.arvore.alcanca(r, i));
                    st.foco = r.filter(|&r| r != i).map_or(vec![i], |r| vec![r, i]);
                }
            }
        }
    }

    /// Janela da árvore do produto, a cada quadro.
    pub(super) fn janela_arvore_produto(&mut self, ctx: &egui::Context) {
        // recolhe as miniaturas prontas e grava os pacotes novos no projeto
        let prontas: Vec<Pronta> = std::mem::take(&mut *PRONTAS.lock().unwrap_or_else(|e| e.into_inner()));
        let mut g = ESTADO.lock().unwrap_or_else(|e| e.into_inner());
        let Some(st) = g.as_mut() else { return };
        let Some(p) = self.project.as_ref().filter(|p| p.root == st.raiz) else {
            *g = None;
            return;
        };
        for pr in prontas.into_iter().filter(|x| x.raiz == st.raiz) {
            st.pedidas.remove(&pr.no.assinatura);
            if let Some(b) = &pr.novo {
                if let Err(e) = p.gravar_visualizacao_no(&pr.no, b) {
                    st.falhas.insert(pr.no.assinatura.clone(), e.to_string());
                }
            }
            match pr.mini {
                Ok(m) => {
                    let img = egui::ColorImage::from_rgba_unmultiplied([m.lado as usize, m.lado as usize], &m.rgba);
                    let t = ctx.load_texture(format!("arvore-{}", pr.no.assinatura), img, egui::TextureOptions::LINEAR);
                    st.texturas.insert(pr.no.assinatura.clone(), t);
                }
                Err(e) => {
                    st.falhas.insert(pr.no.assinatura.clone(), e);
                }
            }
        }
        if ANDAMENTO.load(Ordering::SeqCst) > 0 {
            ctx.request_repaint_after(std::time::Duration::from_millis(200));
        }
        let mut aberta = true;
        let mut acoes: Vec<Acao> = vec![];
        let mut atualizar = false;
        let mut gerar = false;
        egui::Window::new(tr("Árvore do produto"))
            .id(egui::Id::new("arvore_produto"))
            .open(&mut aberta)
            .default_size([920., 560.])
            .min_size([360., 260.])
            .resizable(true)
            .collapsible(true)
            .show(ctx, |ui| {
                barra(ui, st, &mut acoes, &mut atualizar, &mut gerar);
                ui.separator();
                tela(ui, st, &mut acoes);
            });
        if !aberta {
            *g = None;
            return;
        }
        if gerar && pedir(p, st, true) == 0 {
            drop(g);
            return self.toast(tr("Todas as miniaturas já estão prontas."), false);
        }
        let mut abrir: Option<PathBuf> = None;
        let mut abrir_doc: Option<u64> = None;
        let mut ver: Option<(NoProduto, bool)> = None;
        for a in acoes {
            match a {
                Acao::Alternar(c, estava) => {
                    if estava {
                        st.abertos.remove(&c);
                        st.fechados.insert(c);
                    } else {
                        st.fechados.remove(&c);
                        st.abertos.insert(c);
                    }
                }
                Acao::Entrar(c) => {
                    let mut f = st.foco.clone();
                    if f.is_empty() {
                        f = c;
                    } else {
                        f.extend_from_slice(&c[1..]);
                    }
                    st.foco = f;
                    st.abertos.clear();
                    st.fechados.clear();
                    st.enquadrar = true;
                }
                Acao::Foco(k) => {
                    st.foco.truncate(k);
                    st.abertos.clear();
                    st.fechados.clear();
                    st.enquadrar = true;
                }
                Acao::Abrir(i) => match st.arvore.nos[i].id {
                    Some(id) => abrir_doc = Some(id),
                    None => abrir = Some(st.arvore.nos[i].arquivo.clone()),
                },
                Acao::Ver(i) => ver = Some((st.arvore.nos[i].clone(), false)),
                Acao::Comparar(i) => ver = Some((st.arvore.nos[i].clone(), true)),
            }
        }
        if atualizar {
            let foco = st.foco.clone();
            drop(g);
            self.abrir_arvore_produto();
            if let Some(st) = ESTADO.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
                st.foco = foco.into_iter().take_while(|&i| i < st.arvore.nos.len()).collect();
            }
        } else {
            drop(g);
        }
        if let Some(id) = abrir_doc {
            self.open_entry(id, None);
        }
        if let Some(c) = abrir {
            self.open_path(&c);
        }
        if let Some((n, comparar)) = ver {
            self.ver_no_navegador(n, comparar);
        }
    }

    /// "Ver no navegador" de um nó (B339): a peça abre pela versão atual; a montagem, pelo pacote da
    /// assinatura dela, gerado se ainda não existe. `comparar` abre junto o pacote anterior: na peça, o
    /// da versão de antes; na montagem, o da assinatura anterior (antes da última mudança de baixo).
    fn ver_no_navegador(&mut self, n: NoProduto, comparar: bool) {
        if let (Some(id), Some(h)) = (n.id, n.versao.clone()) {
            let antes = comparar.then(|| self.project.as_ref().and_then(|p| p.entry(id).ok()).and_then(|e| e.versions.iter().rev().map(|v| v.hash.clone()).find(|x| *x != h))).flatten();
            if comparar && antes.is_none() {
                return self.toast(tr("Não há versão anterior para comparar."), true);
            }
            return self.visualizar_versao(id, &h, antes.as_deref());
        }
        let Some(p) = self.project.as_ref() else { return };
        let anterior = if comparar {
            match p.visualizacao_anterior_do_no(&n).map(std::fs::read) {
                Some(Ok(b)) => Some(b),
                _ => return self.toast(tr("Não há pacote anterior desta montagem para comparar. Ele existe depois que uma peça de baixo muda e a visualização é gerada de novo."), true),
            }
        } else {
            None
        };
        let pagina = |bytes: &[u8]| -> std::io::Result<PathBuf> {
            let mut meta = serde_json::json!({"nome": n.nome, "pn": n.pn, "estado": match n.estado { EstadoNo::Derivado { liberadas, total } => format!("{liberadas}/{total} {}", tr("peças liberadas")), _ => String::new() }});
            if anterior.is_some() {
                meta["comparando"] = serde_json::json!({"antes": tr("anterior"), "depois": tr("atual")});
            }
            let html = fenkra_pacote::pagina_html_completa(Some(bytes), anterior.as_deref(), Some(&meta.to_string()));
            let arq = p.root.join(fenkra_project::visualizacao::PASTA).join("paginas").join(format!("{}.html", n.assinatura));
            std::fs::create_dir_all(arq.parent().unwrap())?;
            std::fs::write(&arq, html)?;
            Ok(arq)
        };
        let r = match p.visualizacao_do_no(&n).map(std::fs::read) {
            Some(Ok(b)) => pagina(&b).map_err(|e| e.to_string()),
            Some(Err(e)) => Err(e.to_string()),
            None => fenkra_pacote::de_arquivo(&n.arquivo, &opcoes(p)).and_then(|x| x.bytes()).and_then(|b| {
                p.gravar_visualizacao_no(&n, &b).map_err(|e| e.to_string())?;
                pagina(&b).map_err(|e| e.to_string())
            }),
        };
        match r.and_then(|arq| super::visualizador_plm::abrir_no_sistema(&arq).map_err(|e| e.to_string())) {
            Ok(()) => self.toast(tr("Aberto no navegador"), false),
            Err(e) => self.toast(format!("{}: {e}", tr("Falha ao abrir a visualização")), true),
        }
    }
}

fn barra(ui: &mut egui::Ui, st: &mut ArvoreUi, acoes: &mut Vec<Acao>, atualizar: &mut bool, gerar: &mut bool) {
    ui.horizontal_wrapped(|ui| {
        // trilha de navegação nos níveis
        if ui.selectable_label(st.foco.is_empty(), tr("Produto")).on_hover_text(tr("Volta para todas as montagens de topo e peças soltas do projeto.")).clicked() {
            acoes.push(Acao::Foco(0));
        }
        for (k, &i) in st.foco.iter().enumerate() {
            ui.label("›");
            if ui.selectable_label(k + 1 == st.foco.len(), &st.arvore.nos[i].nome).clicked() {
                acoes.push(Acao::Foco(k + 1));
            }
        }
        ui.separator();
        ui.label(tr("Níveis"));
        let antes = st.niveis;
        ui.add(crate::ui_ext::dv(&mut st.niveis).range(1..=12)).on_hover_text(tr("Quantos níveis aparecem abertos. Clique num nó para abrir ou fechar só ele."));
        if st.niveis != antes {
            st.abertos.clear();
            st.fechados.clear();
        }
        if ui.button(tr("Expandir tudo")).on_hover_text(tr("Abre todos os níveis do produto (ou da montagem em foco).")).clicked() {
            st.niveis = st.topo().iter().map(|&r| st.arvore.niveis(r)).max().unwrap_or(1);
            st.abertos.clear();
            st.fechados.clear();
            st.enquadrar = true;
        }
        if ui.button(tr("Enquadrar")).on_hover_text(tr("Ajusta o zoom para caber a árvore inteira.")).clicked() {
            st.enquadrar = true;
        }
        ui.add(egui::TextEdit::singleline(&mut st.busca).hint_text(tr("PN ou nome")).desired_width(110.)).on_hover_text(tr("Destaca os nós cujo PN ou nome contém o texto."));
        let faltam = st.arvore.nos.iter().filter(|n| n.existe && !st.texturas.contains_key(&n.assinatura)).count();
        let andamento = ANDAMENTO.load(Ordering::SeqCst);
        if andamento > 0 {
            ui.spinner();
            ui.label(RichText::new(format!("{andamento} {}", tr("miniaturas em preparo"))).small().weak());
        } else if faltam > 0 && ui.button(format!("{} ({faltam})", tr("Gerar miniaturas"))).on_hover_text(tr("Gera o pacote de visualização .fkv de quem ainda não tem e tira dele a miniatura. O pacote fica para o visualizador.")).clicked() {
            *gerar = true;
        }
        if ui.button(tr("Atualizar")).on_hover_text(tr("Relê as montagens e o índice do projeto.")).clicked() {
            *atualizar = true;
        }
    });
    // legenda da regra de cor
    ui.horizontal_wrapped(|ui| {
        let v = ui.visuals().clone();
        for (cor, rotulo) in [(v.hyperlink_color, "Liberado"), (v.warn_fg_color, "Não liberado"), (v.weak_text_color(), "Fora do projeto ou obsoleto")] {
            let (r, _) = ui.allocate_exact_size(egui::vec2(12., 12.), egui::Sense::hover());
            ui.painter().rect_stroke(r.shrink(1.), 2., egui::Stroke::new(2., cor), egui::StrokeKind::Inside);
            ui.label(RichText::new(tr(rotulo)).small());
        }
        let total = st.arvore.nos.len();
        let pecas = st.arvore.nos.iter().filter(|n| n.tipo == TipoNo::Peca).count();
        ui.label(RichText::new(format!("· {} {} · {} {}", total - pecas, tr("montagens"), pecas, tr("peças"))).small().weak());
        let achados = st.arvore.nos.iter().filter(|n| st.achado(n)).count();
        if !st.busca.trim().is_empty() {
            ui.label(RichText::new(format!("· {achados} {}", tr("achados"))).small().strong());
        }
    });
}

fn tela(ui: &mut egui::Ui, st: &mut ArvoreUi, acoes: &mut Vec<Acao>) {
    let (caixas, ligacoes, cortado) = st.dispor();
    let (resp, pintor) = ui.allocate_painter(ui.available_size(), egui::Sense::click_and_drag());
    let area = resp.rect;
    let v = ui.visuals().clone();
    pintor.rect_filled(area, 4., v.extreme_bg_color);
    if caixas.is_empty() {
        pintor.text(area.center(), egui::Align2::CENTER_CENTER, tr("O projeto não tem peças nem montagens."), egui::FontId::proportional(14.), v.weak_text_color());
        return;
    }
    // enquadrar: zoom e deslocamento para caber tudo
    let (mut lo, mut hi) = (egui::pos2(f32::MAX, f32::MAX), egui::pos2(f32::MIN, f32::MIN));
    for c in &caixas {
        lo = lo.min(c.pos);
        hi = hi.max(c.pos + egui::vec2(LARG, ALT));
    }
    if st.enquadrar {
        let tam = hi - lo;
        st.zoom = ((area.width() - 40.) / tam.x).min((area.height() - 40.) / tam.y).clamp(0.2, 1.4);
        st.desloc = egui::vec2((area.width() - tam.x * st.zoom) / 2., 20.) - lo.to_vec2() * st.zoom;
        st.enquadrar = false;
    }
    // arrastar o fundo desloca; a roda aproxima em torno do cursor
    if resp.dragged() {
        st.desloc += resp.drag_delta();
    }
    if let Some(m) = resp.hover_pos() {
        let rolar = ui.input(|i| i.smooth_scroll_delta.y + i.zoom_delta().ln() * 300.);
        if rolar.abs() > 0.1 {
            let novo = (st.zoom * (rolar * 0.002).exp()).clamp(0.15, 3.);
            let ponto = (m - area.min - st.desloc) / st.zoom;
            st.zoom = novo;
            st.desloc = m - area.min - ponto * novo;
        }
    }
    let z = st.zoom;
    let tela_de = |p: egui::Pos2| area.min + st.desloc + p.to_vec2() * z;
    let pintor = pintor.with_clip_rect(area);
    // ligações em cotovelo: desce do pai, corre na horizontal, desce ao filho
    let traco = egui::Stroke::new((1.4 * z).max(1.), v.widgets.noninteractive.bg_stroke.color);
    for &(a, b) in &ligacoes {
        let pa = tela_de(caixas[a].pos + egui::vec2(LARG / 2., ALT));
        let pb = tela_de(caixas[b].pos + egui::vec2(LARG / 2., 0.));
        let meio = (pa.y + pb.y) / 2.;
        pintor.line_segment([pa, egui::pos2(pa.x, meio)], traco);
        pintor.line_segment([egui::pos2(pa.x, meio), egui::pos2(pb.x, meio)], traco);
        pintor.line_segment([egui::pos2(pb.x, meio), pb], traco);
        if caixas[b].quantidade > 1 {
            pintor.text(pb + egui::vec2(4. * z, -3. * z), egui::Align2::LEFT_BOTTOM, format!("×{}", caixas[b].quantidade), egui::FontId::proportional((11. * z).max(8.)), v.text_color());
        }
    }
    let mut cartao: Option<(usize, egui::Rect)> = None;
    for (k, c) in caixas.iter().enumerate() {
        let r = egui::Rect::from_min_size(tela_de(c.pos), egui::vec2(LARG, ALT) * z);
        if !r.intersects(area) {
            continue;
        }
        let n = &st.arvore.nos[c.no];
        let cor = cor_no(&v, &st.arvore, c.no);
        let id = ui.id().with(("arvore_no", &c.caminho));
        let rr = ui.interact(r.intersect(area), id, egui::Sense::click());
        let fundo = if rr.hovered() { v.widgets.hovered.weak_bg_fill } else { v.widgets.inactive.weak_bg_fill };
        pintor.rect_filled(r, 6. * z, fundo);
        let largura = if st.achado(n) { 3.5 } else { 2. };
        pintor.rect_stroke(r, 6. * z, egui::Stroke::new(largura * z.max(0.5), if st.achado(n) { v.selection.stroke.color } else { cor }), egui::StrokeKind::Inside);
        if st.achado(n) {
            pintor.rect_stroke(r.expand(3. * z), 8. * z, egui::Stroke::new(1.5, v.selection.bg_fill), egui::StrokeKind::Outside);
        }
        // miniatura, pintada com a cor do texto do tema (a imagem é só luz)
        let mr = egui::Rect::from_center_size(r.center_top() + egui::vec2(0., (6. + MINI / 2.) * z), egui::vec2(MINI, MINI) * z);
        match st.texturas.get(&n.assinatura) {
            Some(t) => {
                pintor.image(t.id(), mr, egui::Rect::from_min_max(egui::pos2(0., 0.), egui::pos2(1., 1.)), tinta(&v));
            }
            None => {
                let marca = if !n.existe {
                    "⚠"
                } else if st.pedidas.contains(&n.assinatura) {
                    "…"
                } else if n.tipo == TipoNo::Montagem {
                    "▦"
                } else {
                    "◇"
                };
                pintor.text(mr.center(), egui::Align2::CENTER_CENTER, marca, egui::FontId::proportional(26. * z), v.weak_text_color());
            }
        }
        if n.tipo == TipoNo::Montagem {
            pintor.text(r.left_top() + egui::vec2(5., 4.) * z, egui::Align2::LEFT_TOP, "▦", egui::FontId::proportional((11. * z).max(7.)), cor);
        }
        if z > 0.45 {
            let pn = if n.pn.is_empty() { n.nome.clone() } else { n.pn.clone() };
            let pn: String = if pn.chars().count() > 12 { pn.chars().take(11).chain(std::iter::once('…')).collect() } else { pn };
            pintor.text(r.center_bottom() - egui::vec2(0., 17. * z), egui::Align2::CENTER_CENTER, pn, egui::FontId::monospace(11. * z), v.strong_text_color());
        }
        if c.filhos > 0 {
            let marca = if c.ciclo {
                "↺".to_string()
            } else if c.aberta {
                "−".to_string()
            } else {
                format!("+{}", c.filhos)
            };
            pintor.text(r.center_bottom() - egui::vec2(0., 5. * z), egui::Align2::CENTER_CENTER, marca, egui::FontId::proportional((10. * z).max(7.)), v.weak_text_color());
        }
        if rr.clicked() && c.filhos > 0 && !c.ciclo {
            acoes.push(Acao::Alternar(c.caminho.clone(), c.aberta));
        }
        if rr.double_clicked() {
            acoes.push(if n.tipo == TipoNo::Montagem && !c.ciclo { Acao::Entrar(c.caminho.clone()) } else { Acao::Abrir(c.no) });
        }
        rr.context_menu(|ui| {
            if ui.button(tr("Abrir")).clicked() {
                acoes.push(Acao::Abrir(c.no));
                ui.close();
            }
            if n.tipo == TipoNo::Montagem && ui.button(tr("Entrar no conjunto")).clicked() {
                acoes.push(Acao::Entrar(c.caminho.clone()));
                ui.close();
            }
            if ui.add_enabled(n.existe, egui::Button::new(tr("Ver no navegador"))).clicked() {
                acoes.push(Acao::Ver(c.no));
                ui.close();
            }
            if ui.add_enabled(n.existe, egui::Button::new(tr("Comparar com a anterior"))).on_hover_text(tr("Abre no navegador o pacote atual e o anterior, com o que foi adicionado, removido, movido e alterado.")).clicked() {
                acoes.push(Acao::Comparar(c.no));
                ui.close();
            }
        });
        let fixo = st.cartao_fixo.as_ref().is_some_and(|f| *f == n.pn) && cartao.is_none();
        if rr.hovered() || fixo {
            cartao = Some((k, r));
        }
    }
    if cortado {
        pintor.text(area.left_bottom() + egui::vec2(8., -8.), egui::Align2::LEFT_BOTTOM, tr("Árvore grande: só parte dos nós abertos foi posta. Entre numa montagem para ver o resto."), egui::FontId::proportional(12.), v.warn_fg_color);
    }
    if let Some((k, r)) = cartao {
        cartao_do_no(ui, st, &caixas[k], r);
    }
}

/// O nó crescido ao passar o mouse: miniatura grande, PN, nome, estado e onde é usado.
fn cartao_do_no(ui: &egui::Ui, st: &ArvoreUi, c: &Caixa, r: egui::Rect) {
    let a = &st.arvore;
    let n = &a.nos[c.no];
    let v = ui.visuals();
    let cor = cor_no(v, a, c.no);
    let pos = r.center() - egui::vec2(120., 70.);
    egui::Area::new(ui.id().with(("arvore_cartao", &c.caminho))).order(egui::Order::Tooltip).fixed_pos(pos).interactable(false).show(ui.ctx(), |ui| {
        egui::Frame::popup(ui.style()).stroke(egui::Stroke::new(2., cor)).show(ui, |ui| {
            ui.set_width(240.);
            ui.horizontal(|ui| {
                match st.texturas.get(&n.assinatura) {
                    Some(t) => {
                        ui.add(egui::Image::new(t).fit_to_exact_size(egui::vec2(96., 96.)).tint(tinta(ui.visuals())));
                    }
                    None => {
                        ui.add_sized([96., 96.], egui::Label::new(RichText::new(if n.tipo == TipoNo::Montagem { "▦" } else { "◇" }).size(40.).weak()));
                    }
                }
                ui.vertical(|ui| {
                    ui.label(RichText::new(if n.pn.is_empty() { tr("sem PN").to_string() } else { n.pn.clone() }).monospace().strong());
                    ui.label(&n.nome);
                    ui.label(RichText::new(texto_estado(a, c.no)).color(cor).strong());
                    if let Some(rev) = &n.revisao {
                        ui.label(RichText::new(format!("{} {rev}", tr("Rev"))).small());
                    }
                });
            });
            ui.separator();
            ui.label(RichText::new(tr(if n.tipo == TipoNo::Montagem { "Montagem" } else { "Peça" })).small().weak());
            if c.quantidade > 1 {
                ui.label(RichText::new(format!("{} ×{}", tr("Quantidade no pai"), c.quantidade)).small());
            }
            if let Some(&r0) = c.caminho.first() {
                if r0 != c.no {
                    ui.label(RichText::new(format!("{} {}: {}", tr("Instâncias em"), a.nos[r0].nome, a.quantidade_em(r0, c.no))).small());
                }
            }
            if !n.pais.is_empty() {
                let nomes: Vec<&str> = n.pais.iter().map(|&p| a.nos[p].nome.as_str()).collect();
                ui.label(RichText::new(format!("{}: {}", tr("Onde é usado"), nomes.join(", "))).small());
            }
            if n.tipo == TipoNo::Montagem {
                ui.label(RichText::new(format!("{} {} · {} {}", n.filhos.len(), tr("itens diretos"), a.pecas_abaixo(c.no).len(), tr("peças distintas"))).small());
            }
            if !n.existe {
                ui.label(RichText::new(tr("O arquivo não foi encontrado.")).color(ui.visuals().error_fg_color).small());
            } else if let Some(e) = st.falhas.get(&n.assinatura) {
                ui.label(RichText::new(format!("{}: {e}", tr("Sem miniatura"))).small().weak());
            }
            ui.label(RichText::new(tr("Clique: abre ou fecha o nível · duplo clique: entra ou abre · botão direito: mais")).small().weak());
        });
    });
}

/// Cor que pinta a miniatura (que é só luz). No tema escuro, a cor do texto (clara); no claro, o texto
/// fraco, um tom médio: com a cor do texto escuro as faces ficariam todas quase pretas e a luz sumiria.
fn tinta(v: &egui::Visuals) -> egui::Color32 {
    if v.dark_mode {
        v.text_color()
    } else {
        v.weak_text_color()
    }
}

/// Projeto de exemplo do kart para a captura da árvore.
fn montar_demo(pasta: &Path) -> fenkra_project::PResult<Project> {
    use fenkra_core::{BoolOp, FeatureKind, OriginPlane, PlaneRef, PrimKind, Primitive};
    use fenkra_montagem::{Assembly, NodeKind, Placement};
    std::fs::create_dir_all(pasta)?;
    let mut p = Project::create(pasta, "Kart")?;
    let prim = |nome: &str, kind: PrimKind, a: &str, b: &str, c: &str| {
        let s = |x: &str| x.to_string();
        let mut d = Document::new(nome);
        d.add_feature(FeatureKind::Primitive(Primitive { kind, plane: PlaneRef::Origin(OriginPlane::XY), a: s(a), b: s(b), c: s(c), d: s("0"), op: BoolOp::Add }));
        d
    };
    let chassi = p.add_document(fenkra_solidos::example_document())?;
    let eixo = p.add_document(prim("Eixo", PrimKind::Cilindro, "10", "180", "0"))?;
    let roda = p.add_document(prim("Roda", PrimKind::Cilindro, "60", "40", "0"))?;
    let cubo = p.add_document(prim("Cubo", PrimKind::Cilindro, "22", "50", "0"))?;
    let banco = p.add_document(prim("Banco", PrimKind::Caixa, "60", "50", "40"))?;
    let volante = p.add_document(prim("Volante", PrimKind::Cilindro, "70", "8", "0"))?;
    let coluna = p.add_document(prim("Coluna", PrimKind::Caixa, "12", "12", "140"))?;
    let _calco = p.add_document(prim("Calço avulso", PrimKind::Caixa, "30", "30", "4"))?;
    for id in [chassi, eixo, roda, banco] {
        p.release(id, "liberada")?;
    }
    let rel = |p: &Project, id: u64| p.entry(id).map(|e| e.file.replace('\\', "/"));
    let em = |x: f64, y: f64, z: f64| Placement { origin: [x, y, z], ..Placement::default() };
    let mut conj_roda = Assembly::new("Conjunto roda");
    conj_roda.add_component("Roda", NodeKind::Part { path: rel(&p, roda)? }, em(0., 0., 0.), None);
    conj_roda.add_component("Cubo", NodeKind::Part { path: rel(&p, cubo)? }, em(0., 0., 0.), None);
    std::fs::write(pasta.join("conjunto_roda.fka"), conj_roda.to_json())?;
    let mut eixo_tras = Assembly::new("Eixo traseiro");
    eixo_tras.add_component("Eixo", NodeKind::Part { path: rel(&p, eixo)? }, em(0., 0., 0.), None);
    eixo_tras.add_component("Roda esquerda", NodeKind::SubAssembly { path: "conjunto_roda.fka".into() }, em(0., -90., 0.), None);
    eixo_tras.add_component("Roda direita", NodeKind::SubAssembly { path: "conjunto_roda.fka".into() }, em(0., 90., 0.), None);
    std::fs::write(pasta.join("eixo_traseiro.fka"), eixo_tras.to_json())?;
    let mut direcao = Assembly::new("Direção");
    direcao.add_component("Volante", NodeKind::Part { path: rel(&p, volante)? }, em(0., 0., 140.), None);
    direcao.add_component("Coluna", NodeKind::Part { path: rel(&p, coluna)? }, em(0., 0., 0.), None);
    std::fs::write(pasta.join("direcao.fka"), direcao.to_json())?;
    let mut kart = Assembly::new("Kart");
    kart.add_component("Chassi", NodeKind::Part { path: rel(&p, chassi)? }, em(0., 0., 0.), None);
    kart.add_component("Eixo traseiro", NodeKind::SubAssembly { path: "eixo_traseiro.fka".into() }, em(-50., 0., 0.), None);
    kart.add_component("Roda dianteira E", NodeKind::SubAssembly { path: "conjunto_roda.fka".into() }, em(50., -90., 0.), None);
    kart.add_component("Roda dianteira D", NodeKind::SubAssembly { path: "conjunto_roda.fka".into() }, em(50., 90., 0.), None);
    kart.add_component("Banco", NodeKind::Part { path: rel(&p, banco)? }, em(0., 0., 20.), None);
    kart.add_component("Direção", NodeKind::SubAssembly { path: "direcao.fka".into() }, em(40., 0., 20.), None);
    std::fs::write(pasta.join("kart.fka"), kart.to_json())?;
    Ok(p)
}
