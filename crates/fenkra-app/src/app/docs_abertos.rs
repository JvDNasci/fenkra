// SPDX-License-Identifier: LGPL-2.1-or-later
//! Vários documentos abertos na mesma janela (B363, ADR 0083).
//!
//! Como funciona:
//! - O documento da frente continua nos campos de sempre do `App` (`open`, `hist`, `result`, `sel`,
//!   `session`, `sketch`, `asm`, `dwg`...). Nada no resto do programa precisa saber que há outros.
//! - Cada documento de fundo é um `DocAberto`: os mesmos campos, tirados do `App` por
//!   `std::mem::replace`. Trocar é mover os campos da frente para um `DocAberto` e mover os do
//!   escolhido para o `App`; nada é clonado nem recalculado, então voltar a uma peça mostra o mesmo
//!   resultado, a mesma seleção, a mesma câmera e o mesmo desfazer de quando ela saiu.
//! - Só o documento da frente calcula. Se ele sai com um cálculo em andamento, o pedido é cancelado e o
//!   documento fica "a recalcular": ao voltar, pede de novo. A geração do `Worker` sobe a cada troca,
//!   então um resultado atrasado do documento que saiu nunca cai no que entrou.
//! - A ordem das abas é a da barra; a ordem de uso decide quem volta para a frente quando a aba ativa
//!   fecha. Com mais de `MAX_COM_RESULTADO` documentos, os de fundo mais antigos soltam a malha
//!   calculada (ficam "a recalcular") e guardam só o documento e o desfazer.
//! - Abrir, Novo e os exemplos acrescentam uma aba; abrir um arquivo que já está numa aba só a traz
//!   para a frente. Fechar uma aba com alterações por salvar pergunta, e só por ela.

use super::*;
use crate::settings::Bench;

/// Documentos de fundo que guardam o resultado calculado; os mais antigos além disso o soltam.
pub(crate) const MAX_COM_RESULTADO: usize = 8;

// ------------------------------------------------------------ abas (lógica pura)

struct Aba<T> {
    id: u64,
    /// estado guardado; `None` na aba da frente, cujo estado mora no `App`
    guardado: Option<T>,
}

/// Lista das abas: a da frente e as de fundo com o estado guardado de cada uma.
pub(crate) struct Abas<T> {
    abas: Vec<Aba<T>>,
    ativa: u64,
    proximo: u64,
    /// ids do uso mais recente para o mais antigo
    uso: Vec<u64>,
}

impl<T> Abas<T> {
    pub fn nova() -> Self {
        Abas { abas: vec![Aba { id: 1, guardado: None }], ativa: 1, proximo: 2, uso: vec![1] }
    }

    pub fn ativa(&self) -> u64 {
        self.ativa
    }

    pub fn len(&self) -> usize {
        self.abas.len()
    }

    /// Ids na ordem da barra.
    pub fn ids(&self) -> Vec<u64> {
        self.abas.iter().map(|a| a.id).collect()
    }

    pub fn guardado(&self, id: u64) -> Option<&T> {
        self.abas.iter().find(|a| a.id == id)?.guardado.as_ref()
    }

    pub fn guardado_mut(&mut self, id: u64) -> Option<&mut T> {
        self.abas.iter_mut().find(|a| a.id == id)?.guardado.as_mut()
    }

    fn usar(&mut self, id: u64) {
        self.uso.retain(|&u| u != id);
        self.uso.insert(0, id);
    }

    fn posicao(&self, id: u64) -> Option<usize> {
        self.abas.iter().position(|a| a.id == id)
    }

    /// Abre uma aba nova logo depois da ativa e a põe na frente; `saindo` é o estado da que vai para o
    /// fundo. Devolve o id da nova.
    pub fn abrir(&mut self, saindo: T) -> u64 {
        let i = self.posicao(self.ativa).unwrap_or(self.abas.len().saturating_sub(1));
        if let Some(a) = self.abas.get_mut(i) {
            a.guardado = Some(saindo);
        }
        let id = self.proximo;
        self.proximo += 1;
        self.abas.insert((i + 1).min(self.abas.len()), Aba { id, guardado: None });
        self.ativa = id;
        self.usar(id);
        id
    }

    /// Põe `id` na frente: guarda `saindo` na aba ativa e devolve o estado da escolhida. Se `id` já é a
    /// ativa ou não existe, devolve `saindo` de volta em `Err`.
    pub fn trocar(&mut self, id: u64, saindo: T) -> Result<T, T> {
        if id == self.ativa {
            return Err(saindo);
        }
        let Some(j) = self.posicao(id) else { return Err(saindo) };
        let Some(entrando) = self.abas[j].guardado.take() else { return Err(saindo) };
        if let Some(i) = self.posicao(self.ativa) {
            self.abas[i].guardado = Some(saindo);
        }
        self.ativa = id;
        self.usar(id);
        Ok(entrando)
    }

    /// Fecha a aba ativa. Devolve o estado da que vem para a frente (a usada por último), ou `None`
    /// quando era a única: aí fica uma aba nova, vazia, para o programa preencher.
    pub fn fechar_ativa(&mut self) -> Option<T> {
        let fechada = self.ativa;
        self.uso.retain(|&u| u != fechada);
        if self.abas.len() <= 1 {
            let id = self.proximo;
            self.proximo += 1;
            self.abas = vec![Aba { id, guardado: None }];
            self.ativa = id;
            self.uso = vec![id];
            return None;
        }
        self.abas.retain(|a| a.id != fechada);
        let seguinte = self.uso.first().copied().filter(|u| self.posicao(*u).is_some()).unwrap_or(self.abas[0].id);
        self.ativa = seguinte;
        self.usar(seguinte);
        self.guardado_mut_take(seguinte)
    }

    fn guardado_mut_take(&mut self, id: u64) -> Option<T> {
        self.abas.iter_mut().find(|a| a.id == id)?.guardado.take()
    }

    /// Aba vizinha na ordem da barra (Ctrl+Tab para frente, Ctrl+Shift+Tab para trás), dando a volta.
    pub fn vizinha(&self, para_tras: bool) -> Option<u64> {
        let n = self.abas.len();
        if n < 2 {
            return None;
        }
        let i = self.posicao(self.ativa)?;
        let j = if para_tras { (i + n - 1) % n } else { (i + 1) % n };
        Some(self.abas[j].id)
    }

    /// Abas de fundo além das `max` usadas mais recentemente: as que devem soltar a malha.
    pub fn excedentes(&self, max: usize) -> Vec<u64> {
        self.uso.iter().filter(|&&u| u != self.ativa).skip(max).copied().collect()
    }
}

// ------------------------------------------------------------ estado de um documento de fundo

/// Tipo do documento, para o ícone e o nome da aba.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Tipo {
    Peca,
    Montagem,
    Desenho,
}

impl Tipo {
    fn icone(self) -> &'static str {
        match self {
            Tipo::Peca => "banc_solidos",
            Tipo::Montagem => "banc_montagem",
            Tipo::Desenho => "banc_desenho",
        }
    }

    fn nome(self) -> &'static str {
        match self {
            Tipo::Peca => "Peça",
            Tipo::Montagem => "Montagem",
            Tipo::Desenho => "Desenho",
        }
    }
}

/// Tudo o que é de um documento e sai de cena com ele.
pub struct DocAberto {
    pub(crate) open: OpenDoc,
    pub(crate) hist: crate::historico::Historico,
    pub(crate) result: Option<RecomputeResult>,
    result_doc: Option<Document>,
    sent_doc: Option<Document>,
    pub(crate) sel: Sel,
    multi: Vec<Id>,
    picks: Vec<crate::picking::Pick>,
    session: Option<Session>,
    sketch: Option<crate::sketch_ui::SketchEdit>,
    asm: Option<crate::asm_ui::AsmState>,
    dwg: Option<crate::dwg_ui::DwgState>,
    forma: crate::forma_ui::Estado,
    bench: Bench,
    camera: Option<crate::render::Camera>,
    errors_prev: usize,
    /// saiu com cálculo pendente ou soltou a malha: pede de novo ao voltar
    pub(crate) a_recalcular: bool,
}

impl DocAberto {
    /// Documento de fundo só com o documento e o desfazer (o resto vazio).
    pub(crate) fn de(open: OpenDoc, hist: crate::historico::Historico, bench: Bench) -> Self {
        DocAberto {
            open,
            hist,
            result: None,
            result_doc: None,
            sent_doc: None,
            sel: Sel::None,
            multi: vec![],
            picks: vec![],
            session: None,
            sketch: None,
            asm: None,
            dwg: None,
            forma: Default::default(),
            bench,
            camera: None,
            errors_prev: 0,
            a_recalcular: true,
        }
    }

    fn tipo(&self) -> Tipo {
        tipo_de(self.bench, self.asm.is_some(), self.dwg.is_some())
    }

    fn nome(&self) -> String {
        nome_de(self.tipo(), &self.open, self.asm.as_ref(), self.dwg.as_ref())
    }

    /// Alguma coisa deste documento (peça, montagem ou desenho) por salvar?
    pub(crate) fn modificado(&self) -> bool {
        (self.open.modified && self.open.frozen.is_none()) || self.asm.as_ref().is_some_and(|s| s.modified) || self.dwg.as_ref().is_some_and(|s| s.modified)
    }

    fn arquivos(&self) -> Vec<PathBuf> {
        arquivos_de(&self.open, self.asm.as_ref(), self.dwg.as_ref())
    }

    fn arquivo_principal(&self) -> Option<PathBuf> {
        principal_de(self.tipo(), &self.open, self.asm.as_ref(), self.dwg.as_ref())
    }

    /// Solta a malha calculada (limite de memória): ao voltar, recalcula.
    pub(crate) fn soltar_resultado(&mut self) {
        if self.result.take().is_some() {
            self.result_doc = None;
            self.a_recalcular = true;
        }
    }
}

fn tipo_de(bench: Bench, asm: bool, dwg: bool) -> Tipo {
    match bench {
        Bench::Montagem if asm => Tipo::Montagem,
        Bench::Desenho if dwg => Tipo::Desenho,
        _ => Tipo::Peca,
    }
}

fn nome_de(t: Tipo, open: &OpenDoc, asm: Option<&crate::asm_ui::AsmState>, dwg: Option<&crate::dwg_ui::DwgState>) -> String {
    match (t, asm, dwg) {
        (Tipo::Montagem, Some(s), _) => s.asm.name.clone(),
        (Tipo::Desenho, _, Some(s)) => s.d.name.clone(),
        _ => open.doc.name.clone(),
    }
}

fn arquivos_de(open: &OpenDoc, asm: Option<&crate::asm_ui::AsmState>, dwg: Option<&crate::dwg_ui::DwgState>) -> Vec<PathBuf> {
    open.file.iter().cloned().chain(asm.and_then(|s| s.file.clone())).chain(dwg.and_then(|s| s.path.clone())).collect()
}

fn principal_de(t: Tipo, open: &OpenDoc, asm: Option<&crate::asm_ui::AsmState>, dwg: Option<&crate::dwg_ui::DwgState>) -> Option<PathBuf> {
    match t {
        Tipo::Montagem => asm.and_then(|s| s.file.clone()),
        Tipo::Desenho => dwg.and_then(|s| s.path.clone()),
        Tipo::Peca => open.file.clone(),
    }
}

/// Mesmo arquivo? Compara o caminho canônico quando dá (letras de unidade, `..`).
fn mesmo_arquivo(a: &Path, b: &Path) -> bool {
    a == b || matches!((a.canonicalize(), b.canonicalize()), (Ok(x), Ok(y)) if x == y)
}

use std::path::Path;

/// Estado dos documentos abertos guardado no `App`.
pub struct Documentos {
    pub(crate) abas: Abas<DocAberto>,
    /// troca pedida por um clique: feita no começo do quadro seguinte, antes de qualquer desenho
    pedido: Option<Pedido>,
    demo_feita: bool,
}

#[derive(Clone, Copy, Debug)]
enum Pedido {
    Trocar(u64),
    Fechar(u64),
}

impl Default for Documentos {
    fn default() -> Self {
        Documentos { abas: Abas::nova(), pedido: None, demo_feita: false }
    }
}

/// Linha da lista de documentos, para a barra, o rodapé e o menu Janela.
struct Linha {
    id: u64,
    ativa: bool,
    tipo: Tipo,
    /// nome guardado no documento (o mesmo para duas cópias do mesmo exemplo)
    nome: String,
    modificado: bool,
    arquivo: Option<PathBuf>,
    /// o que a aba mostra: único entre as abas (B463), preenchido por [`rotulos_distintos`]
    exibido: String,
}

/// B463: o nome que a aba mostra. Documento com arquivo aparece pelo arquivo (`placa`), que é o que
/// a pessoa escolheu ao salvar; sem arquivo, pelo nome do documento. Nomes que ainda se repetem
/// ganham a pasta do arquivo, quando ela desempata, ou um número na ordem da barra (a primeira
/// fica sem número), para nenhuma aba ficar igual a outra.
fn rotulos_distintos(linhas: &[Linha]) -> Vec<String> {
    let base: Vec<String> = linhas.iter().map(|l| l.arquivo.as_ref().and_then(|p| p.file_stem()).map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| l.nome.clone())).collect();
    let pasta = |i: usize| linhas[i].arquivo.as_ref().and_then(|p| p.parent()).and_then(|p| p.file_name()).map(|s| s.to_string_lossy().into_owned());
    (0..linhas.len())
        .map(|i| {
            let iguais: Vec<usize> = (0..linhas.len()).filter(|&j| base[j] == base[i]).collect();
            if iguais.len() < 2 {
                return base[i].clone();
            }
            if let Some(p) = pasta(i) {
                if iguais.iter().filter(|&&j| pasta(j).as_ref() == Some(&p)).count() == 1 {
                    return format!("{} — {p}", base[i]);
                }
            }
            match iguais.iter().position(|&j| j == i) {
                Some(0) | None => base[i].clone(),
                Some(k) => format!("{} ({})", base[i], k + 1),
            }
        })
        .collect()
}

// ------------------------------------------------------------ troca no App

impl App {
    /// Tira o documento da frente para o fundo, deixando campos vazios no lugar.
    fn tirar_ativo(&mut self) -> DocAberto {
        // o Scan só olha: o documento sai na posição de antes dele
        crate::scan::sair(self);
        let ocupado = self.gen_done < self.gen_sent;
        if ocupado {
            self.worker.cancelar();
        }
        let vazio = OpenDoc { doc: Document::new("Peça"), entry: None, frozen: None, file: None, modified: false, ramo: None };
        let result = self.result.take();
        DocAberto {
            open: std::mem::replace(&mut self.open, vazio),
            hist: std::mem::replace(&mut self.hist, crate::historico::Historico::new(self.settings.historico_mb)),
            a_recalcular: ocupado || result.is_none(),
            result,
            result_doc: self.result_doc.take(),
            sent_doc: self.sent_doc.take(),
            sel: std::mem::take(&mut self.sel),
            multi: self.multi.take(),
            picks: std::mem::take(&mut self.picks),
            session: self.session.take(),
            sketch: self.sketch.take(),
            asm: self.asm.take(),
            dwg: self.dwg.take(),
            forma: std::mem::take(&mut self.forma),
            bench: self.settings.bench,
            camera: self.viewport.as_ref().map(|v| v.camera),
            errors_prev: std::mem::take(&mut self.errors_prev),
        }
    }

    /// Põe `d` na frente, exatamente como saiu.
    fn por_ativo(&mut self, d: DocAberto) {
        // o que aponta para itens do documento que saiu não vale para o que entra
        self.sketch_wait_plane = false;
        self.begin_sketch_pending = None;
        self.open_pending = None;
        self.align_pending = None;
        self.hover_pick = None;
        self.ctx_pick = None;
        self.ctx_actions.clear();
        self.hover_item = None;
        self.renaming = None;
        self.props_target = None;
        self.delete_dialog = None;
        self.repeat = None;
        self.extract = None;
        self.pmi_pending = None;
        self.pmi_focus = None;
        self.comparacao = None;
        self.chapa_anim = None;
        self.hidden_env = false;
        self.st_falhas.clear();
        self.st_cut_hi.clear();
        self.st_cantos.clear();
        self.st_cantos_sel = None;
        self.tree_order.borrow_mut().clear();
        self.last_order.borrow_mut().clear();
        self.multi_anchor.set(None);
        self.polir = Default::default();
        let DocAberto { open, hist, result, result_doc, sent_doc, sel, multi, picks, session, sketch, asm, dwg, forma, bench, camera, errors_prev, a_recalcular } = d;
        self.open = open;
        self.hist = hist;
        self.result = result;
        self.result_doc = result_doc;
        self.sent_doc = sent_doc;
        self.sel = sel;
        *self.multi.borrow_mut() = multi;
        self.picks = picks;
        self.session = session;
        self.sketch = sketch;
        self.asm = asm;
        self.dwg = dwg;
        self.forma = forma;
        self.settings.bench = bench;
        self.errors_prev = errors_prev;
        // um resultado atrasado do documento que saiu tem geração menor e é descartado
        self.gen_sent += 1;
        self.gen_done = self.gen_sent;
        self.doc_trocado = true;
        self.scene_dirty = true;
        match (camera, self.viewport.is_some()) {
            (Some(c), true) => {
                if let Some(vp) = self.viewport.as_mut() {
                    vp.camera = c;
                }
                self.fit_pending = false;
            }
            _ => self.fit_now(),
        }
        if a_recalcular || self.result.is_none() {
            self.request_recompute();
        }
    }

    fn aliviar_memoria(&mut self) {
        for id in self.documentos.abas.excedentes(MAX_COM_RESULTADO) {
            if let Some(d) = self.documentos.abas.guardado_mut(id) {
                d.soltar_resultado();
            }
        }
    }

    /// Põe a aba `id` na frente. Devolve se trocou.
    pub(crate) fn trocar_documento(&mut self, id: u64) -> bool {
        if id == self.documentos.abas.ativa() || self.documentos.abas.guardado(id).is_none() {
            return false;
        }
        let saindo = self.tirar_ativo();
        match self.documentos.abas.trocar(id, saindo) {
            Ok(entrando) => {
                self.por_ativo(entrando);
                self.aliviar_memoria();
                true
            }
            Err(saindo) => {
                self.por_ativo(saindo);
                false
            }
        }
    }

    /// Fecha a aba da frente sem perguntar (a pergunta já foi respondida). A última aba fechada volta
    /// para a tela inicial com uma peça vazia.
    pub(super) fn fechar_ativo_sem_perguntar(&mut self) {
        match self.documentos.abas.fechar_ativa() {
            Some(d) => self.por_ativo(d),
            None => {
                let mut d = DocAberto::de(OpenDoc { doc: Document::new("Peça"), entry: None, frozen: None, file: None, modified: false, ramo: None }, crate::historico::Historico::new(self.settings.historico_mb), Bench::Solidos);
                d.a_recalcular = true;
                self.por_ativo(d);
                self.started = false;
                self.inicio_pelo_menu = false;
            }
        }
    }

    /// Fecha a aba `id`, perguntando antes se ela tem alterações por salvar.
    pub(crate) fn fechar_documento(&mut self, id: u64) {
        if id != self.documentos.abas.ativa() && !self.trocar_documento(id) {
            return;
        }
        self.com_confirmacao(Acao::FecharDocumento);
    }

    /// Alguma aba de fundo com alterações por salvar? Devolve a primeira.
    pub(super) fn fundo_modificado(&self) -> Option<u64> {
        let abas = &self.documentos.abas;
        abas.ids().into_iter().find(|&id| abas.guardado(id).is_some_and(DocAberto::modificado))
    }

    /// A aba da frente tem algo que valha guardar? Sem nada (a peça de exemplo do começo, nunca
    /// tocada), abrir outro documento ocupa o lugar dela em vez de abrir aba nova.
    fn ativo_descartavel(&self) -> bool {
        !self.started && !self.inicio_pelo_menu && !self.open.modified && self.open.file.is_none() && self.open.entry.is_none() && self.asm.is_none() && self.dwg.is_none() && !self.hist.pode_desfazer()
    }

    /// Aba que já tem `p` aberto.
    fn aba_com_arquivo(&self, p: &Path) -> Option<u64> {
        if arquivos_de(&self.open, self.asm.as_ref(), self.dwg.as_ref()).iter().any(|a| mesmo_arquivo(a, p)) {
            return Some(self.documentos.abas.ativa());
        }
        let abas = &self.documentos.abas;
        abas.ids().into_iter().find(|&id| abas.guardado(id).is_some_and(|d| d.arquivos().iter().any(|a| mesmo_arquivo(a, p))))
    }

    /// Abrir, Novo e exemplos: acrescentam uma aba (ou trazem a que já tem o arquivo). Devolve `true`
    /// quando cuidou da ação; `false` para as demais, que seguem o caminho de sempre.
    pub(super) fn abrir_em_aba(&mut self, acao: &Acao) -> bool {
        match acao {
            Acao::NovaPeca | Acao::Exemplo | Acao::ExemploNomeado(_) | Acao::AbrirCaminho(_) => {}
            Acao::AbrirDialogo => {
                // escolhe o arquivo antes, para reaproveitar a aba que já o tiver
                if let Some(p) = rfd::FileDialog::new().add_filter("Peça Fenkra", &["fkp"]).pick_file() {
                    self.abrir_em_aba(&Acao::AbrirCaminho(p));
                }
                return true;
            }
            _ => return false,
        }
        if let Acao::AbrirCaminho(p) = acao {
            if let Some(id) = self.aba_com_arquivo(p) {
                self.trocar_documento(id);
                self.started = true;
                return true;
            }
        }
        if self.ativo_descartavel() {
            self.rodar_acao(acao.clone());
            return true;
        }
        let bench = self.settings.bench;
        let saindo = self.tirar_ativo();
        self.documentos.abas.abrir(saindo);
        // aba nova parte de uma peça vazia na bancada de peça
        let mut nova = DocAberto::de(OpenDoc { doc: Document::new("Peça"), entry: None, frozen: None, file: None, modified: false, ramo: None }, crate::historico::Historico::new(self.settings.historico_mb), if matches!(bench, Bench::Montagem | Bench::Desenho) { Bench::Solidos } else { bench });
        nova.a_recalcular = false;
        self.por_ativo(nova);
        self.doc_trocado = false;
        self.rodar_acao(acao.clone());
        let carregou = self.doc_trocado || self.asm.is_some() || self.dwg.is_some();
        self.doc_trocado = true;
        if !carregou {
            // o arquivo não abriu (erro já avisado): a aba vazia sai e a anterior volta
            self.fechar_ativo_sem_perguntar();
            self.started = true;
        }
        self.aliviar_memoria();
        true
    }

    fn linhas(&self) -> Vec<Linha> {
        let abas = &self.documentos.abas;
        let mut linhas = abas
            .ids()
            .into_iter()
            .filter_map(|id| {
                if id == abas.ativa() {
                    let tipo = tipo_de(self.settings.bench, self.asm.is_some(), self.dwg.is_some());
                    Some(Linha { id, ativa: true, tipo, nome: nome_de(tipo, &self.open, self.asm.as_ref(), self.dwg.as_ref()), modificado: self.algo_modificado(), arquivo: principal_de(tipo, &self.open, self.asm.as_ref(), self.dwg.as_ref()), exibido: String::new() })
                } else {
                    let d = abas.guardado(id)?;
                    Some(Linha { id, ativa: false, tipo: d.tipo(), nome: d.nome(), modificado: d.modificado(), arquivo: d.arquivo_principal(), exibido: String::new() })
                }
            })
            .collect::<Vec<_>>();
        let rotulos = rotulos_distintos(&linhas);
        for (l, r) in linhas.iter_mut().zip(rotulos) {
            l.exibido = r;
        }
        linhas
    }

    /// Arquivos das abas (para "Abrir em nova janela" do menu Janela).
    pub(super) fn arquivos_abertos(&self) -> Vec<PathBuf> {
        self.linhas().into_iter().filter_map(|l| l.arquivo).collect()
    }

    /// Começo do quadro: aplica a troca pedida, o Ctrl+Tab e a demonstração `FENKRA_DEMO_DOCS`.
    pub(super) fn documentos_inicio_do_quadro(&mut self, ctx: &egui::Context) {
        self.demo_documentos();
        if !self.confirmando() {
            let (tras, frente) = ctx.input_mut(|i| (i.consume_key(egui::Modifiers::CTRL | egui::Modifiers::SHIFT, egui::Key::Tab), i.consume_key(egui::Modifiers::CTRL, egui::Key::Tab)));
            if tras || frente {
                if let Some(id) = self.documentos.abas.vizinha(tras) {
                    self.documentos.pedido = Some(Pedido::Trocar(id));
                }
            }
        }
        match self.documentos.pedido.take() {
            Some(Pedido::Trocar(id)) => {
                self.trocar_documento(id);
            }
            Some(Pedido::Fechar(id)) => self.fechar_documento(id),
            None => {}
        }
    }

    /// Desenvolvimento: `FENKRA_DEMO_DOCS=<n>` abre os `n` primeiros exemplos em abas e marca o
    /// primeiro como editado (parâmetro L de 120 para 150), para a captura da barra.
    fn demo_documentos(&mut self) {
        if self.documentos.demo_feita {
            return;
        }
        self.documentos.demo_feita = true;
        let Some(n) = std::env::var("FENKRA_DEMO_DOCS").ok().and_then(|v| v.parse::<usize>().ok()) else { return };
        // B463: `FENKRA_DEMO_DOCS_IGUAIS=1` abre o mesmo exemplo em todas as abas (nomes repetidos)
        let mut exemplos = crate::examples::all();
        if std::env::var("FENKRA_DEMO_DOCS_IGUAIS").is_ok() {
            exemplos = (0..n.max(1)).map(|_| crate::examples::all().remove(0)).collect();
        }
        for (i, e) in exemplos.into_iter().take(n.max(1)).enumerate() {
            if i > 0 {
                let saindo = self.tirar_ativo();
                self.documentos.abas.abrir(saindo);
            }
            self.load_doc(e.doc, None, None);
            self.settings.bench = e.bench;
            if i == 0 {
                let antes = self.open.doc.clone();
                if let Some(p) = self.open.doc.params.iter_mut().find(|p| p.name == "L") {
                    p.expr = "150".into();
                }
                self.hist.registrar(antes, &self.open.doc.clone());
                self.open.modified = true;
                self.request_recompute();
            }
        }
        self.started = true;
        if let Some(id) = std::env::var("FENKRA_DEMO_DOCS_ATIVO").ok().and_then(|v| v.parse::<usize>().ok()).and_then(|i| self.documentos.abas.ids().get(i).copied()) {
            self.trocar_documento(id);
        }
    }

    /// Barra de documentos (abas) logo abaixo do topo; aparece com dois ou mais documentos abertos.
    pub(super) fn barra_de_documentos(&mut self, root: &mut egui::Ui) {
        if self.documentos.abas.len() < 2 {
            return;
        }
        let linhas = self.linhas();
        let mut pedido = None;
        let fundo = root.visuals().panel_fill;
        egui::Panel::top("barra_de_documentos").resizable(false).frame(egui::Frame::new().fill(fundo).inner_margin(egui::Margin::symmetric(6, 2))).show(root, |ui| {
            egui::ScrollArea::horizontal().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.;
                    for l in &linhas {
                        if let Some(p) = aba_ui(ui, l) {
                            pedido = Some(p);
                        }
                    }
                });
            });
        });
        self.aplicar_pedido_da_lista(pedido);
    }

    fn aplicar_pedido_da_lista(&mut self, p: Option<PedidoLista>) {
        match p {
            Some(PedidoLista::Trocar(id)) => self.documentos.pedido = Some(Pedido::Trocar(id)),
            Some(PedidoLista::Fechar(id)) => self.documentos.pedido = Some(Pedido::Fechar(id)),
            Some(PedidoLista::NovaJanela(p)) => super::janelas::abrir_em_nova_janela_avisando(self, &p),
            None => {}
        }
    }

    /// Rodapé: um botão com a quantidade de documentos que abre a mesma lista da barra.
    pub(crate) fn documentos_no_rodape(&mut self, ui: &mut egui::Ui) {
        let n = self.documentos.abas.len();
        if n < 2 {
            return;
        }
        let linhas = self.linhas();
        let mut pedido = None;
        let r = ui.small_button(format!("🗐 {n}")).on_hover_text(format!("{}\n{}", tr("Documentos abertos"), tr("Ctrl+Tab passa para o próximo; Ctrl+Shift+Tab volta.")));
        egui::Popup::menu(&r).show(|ui| {
            pedido = lista_ui(ui, &linhas);
        });
        self.aplicar_pedido_da_lista(pedido);
    }

    /// Parte do menu Janela: os documentos abertos, próximo e fechar.
    pub(super) fn menu_documentos(&mut self, ui: &mut egui::Ui) {
        let linhas = self.linhas();
        ui.label(RichText::new(tr("Documentos abertos")).small().weak());
        let mut pedido = lista_ui(ui, &linhas);
        let varios = linhas.len() > 1;
        if ui.add_enabled(varios, egui::Button::new(tr("Próximo documento")).shortcut_text("Ctrl+Tab")).clicked() {
            pedido = self.documentos.abas.vizinha(false).map(PedidoLista::Trocar);
            ui.close();
        }
        if ui.add_enabled(varios, egui::Button::new(tr("Documento anterior")).shortcut_text("Ctrl+Shift+Tab")).clicked() {
            pedido = self.documentos.abas.vizinha(true).map(PedidoLista::Trocar);
            ui.close();
        }
        if ui.button(tr("Fechar documento")).on_hover_text(tr("Fecha o documento da frente; pergunta antes se ele tem alterações por salvar.")).clicked() {
            pedido = Some(PedidoLista::Fechar(self.documentos.abas.ativa()));
            ui.close();
        }
        if pedido.is_some() {
            ui.close();
        }
        self.aplicar_pedido_da_lista(pedido);
    }
}

enum PedidoLista {
    Trocar(u64),
    Fechar(u64),
    NovaJanela(PathBuf),
}

fn rotulo(l: &Linha) -> String {
    format!("{}{}", l.exibido, if l.modificado { " •" } else { "" })
}

fn dica(l: &Linha) -> String {
    let onde = l.arquivo.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| tr("ainda não salvo").to_string());
    let estado = if l.modificado { tr("com alterações por salvar") } else { tr("sem alterações por salvar") };
    format!("{} — {}\n{onde}\n{estado}\n{}", tr(l.tipo.nome()), l.nome, tr("Ctrl+Tab passa para o próximo; Ctrl+Shift+Tab volta."))
}

/// Uma aba da barra: ícone do tipo, nome, ponto de modificado e fechar.
fn aba_ui(ui: &mut egui::Ui, l: &Linha) -> Option<PedidoLista> {
    let mut pedido = None;
    let destaque = crate::theme::accent(ui.visuals());
    let fill = if l.ativa { destaque.gamma_multiply(0.22) } else { ui.visuals().faint_bg_color };
    let stroke = if l.ativa { egui::Stroke::new(1., destaque) } else { egui::Stroke::new(1., ui.visuals().widgets.noninteractive.bg_stroke.color) };
    egui::Frame::new().fill(fill).stroke(stroke).corner_radius(4.).inner_margin(egui::Margin::symmetric(6, 2)).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.;
            let img = egui::Image::new(crate::icons::get(l.tipo.icone())).fit_to_exact_size(egui::vec2(14., 14.));
            let mut texto = RichText::new(rotulo(l));
            if l.ativa {
                texto = texto.strong();
            }
            let r = ui.add(egui::Button::image_and_text(img, texto).frame(false).selected(l.ativa)).on_hover_text(dica(l));
            if r.clicked() && !l.ativa {
                pedido = Some(PedidoLista::Trocar(l.id));
            }
            if r.middle_clicked() {
                pedido = Some(PedidoLista::Fechar(l.id));
            }
            r.context_menu(|ui| {
                if ui.button(tr("Fechar documento")).clicked() {
                    pedido = Some(PedidoLista::Fechar(l.id));
                    ui.close();
                }
                match &l.arquivo {
                    Some(p) => {
                        if ui.button(tr("Abrir em nova janela")).clicked() {
                            pedido = Some(PedidoLista::NovaJanela(p.clone()));
                            ui.close();
                        }
                    }
                    None => {
                        ui.add_enabled(false, egui::Button::new(tr("Abrir em nova janela"))).on_disabled_hover_text(tr("Salve o documento para abri-lo em outra janela."));
                    }
                }
            });
            if ui.add(egui::Button::new(RichText::new("✕").small()).frame(false)).on_hover_text(tr("Fechar documento")).clicked() {
                pedido = Some(PedidoLista::Fechar(l.id));
            }
        });
    });
    pedido
}

/// Lista vertical (rodapé e menu Janela): escolher troca; o ✕ fecha.
fn lista_ui(ui: &mut egui::Ui, linhas: &[Linha]) -> Option<PedidoLista> {
    let mut pedido = None;
    for l in linhas {
        ui.horizontal(|ui| {
            let img = egui::Image::new(crate::icons::get(l.tipo.icone())).fit_to_exact_size(egui::vec2(14., 14.));
            if ui.add(egui::Button::image_and_text(img, rotulo(l)).selected(l.ativa)).on_hover_text(dica(l)).clicked() {
                if !l.ativa {
                    pedido = Some(PedidoLista::Trocar(l.id));
                }
                ui.close();
            }
            if ui.small_button("✕").on_hover_text(tr("Fechar documento")).clicked() {
                pedido = Some(PedidoLista::Fechar(l.id));
                ui.close();
            }
        });
    }
    pedido
}

#[cfg(test)]
mod tests {
    use super::*;

    fn linha(nome: &str, arquivo: Option<&str>) -> Linha {
        Linha { id: 0, ativa: false, tipo: Tipo::Peca, nome: nome.into(), modificado: false, arquivo: arquivo.map(PathBuf::from), exibido: String::new() }
    }

    fn sem_repeticao(r: &[String]) {
        let mut v = r.to_vec();
        v.sort();
        v.dedup();
        assert_eq!(v.len(), r.len(), "{r:?}");
    }

    /// B463: duas abas nunca mostram o mesmo nome. O arquivo salvo dá o nome; cópias sem arquivo
    /// ganham número na ordem da barra; arquivos de mesmo nome em pastas diferentes, a pasta.
    #[test]
    fn abas_com_o_mesmo_nome_ficam_distintas() {
        // o caso da rodada 2: o exemplo salvo como placa.fkp e outro exemplo ainda sem arquivo
        let r = rotulos_distintos(&[linha("Placa de exemplo", Some("C:/teste/placa.fkp")), linha("Placa de exemplo", None)]);
        assert_eq!(r, vec!["placa", "Placa de exemplo"]);
        // dois exemplos sem arquivo: a primeira fica, a segunda ganha número
        let r = rotulos_distintos(&[linha("Placa de exemplo", None), linha("Placa de exemplo", None), linha("Flange", None)]);
        assert_eq!(r, vec!["Placa de exemplo", "Placa de exemplo (2)", "Flange"]);
        sem_repeticao(&r);
        // mesmo arquivo em pastas diferentes: a pasta desempata
        let r = rotulos_distintos(&[linha("A", Some("C:/obra1/placa.fkp")), linha("B", Some("C:/obra2/placa.fkp"))]);
        assert_eq!(r, vec!["placa — obra1", "placa — obra2"]);
        // arquivo e documento sem arquivo com o mesmo nome: a pasta marca um, o número o outro
        let r = rotulos_distintos(&[linha("x", Some("C:/a/placa.fkp")), linha("placa", None)]);
        assert_eq!(r, vec!["placa — a", "placa (2)"]);
        sem_repeticao(&r);
    }

    #[test]
    fn abas_abrem_trocam_e_fecham_pela_ordem_de_uso() {
        // estado de cada aba: o nome do documento
        let mut a: Abas<&str> = Abas::nova();
        assert_eq!(a.len(), 1);
        let b = a.abrir("A"); // A vai para o fundo, B na frente
        let c = a.abrir("B"); // B vai para o fundo, C na frente
        assert_eq!(a.ids(), vec![1, b, c]);
        assert_eq!(a.ativa(), c);
        // trocar devolve o estado guardado e guarda o que sai
        assert_eq!(a.trocar(1, "C"), Ok("A"));
        assert_eq!(a.guardado(c), Some(&"C"));
        assert_eq!(a.guardado(1), None);
        // trocar para a própria ativa não mexe em nada
        assert_eq!(a.trocar(1, "x"), Err("x"));
        // Ctrl+Tab segue a barra e dá a volta
        assert_eq!(a.vizinha(false), Some(b));
        assert_eq!(a.vizinha(true), Some(c));
        // fechar a ativa (A) traz a usada por último antes dela: C
        assert_eq!(a.fechar_ativa(), Some("C"));
        assert_eq!(a.ativa(), c);
        assert_eq!(a.ids(), vec![b, c]);
        assert_eq!(a.fechar_ativa(), Some("B"));
        // a última aba fechada deixa uma aba nova e vazia
        assert_eq!(a.fechar_ativa(), None);
        assert_eq!(a.len(), 1);
        assert!(a.vizinha(false).is_none());
    }

    #[test]
    fn limite_de_memoria_solta_os_de_fundo_mais_antigos() {
        let mut a: Abas<u32> = Abas::nova();
        for i in 0..11 {
            a.abrir(i);
        }
        // 12 abas: a da frente e 11 de fundo; as 3 usadas há mais tempo soltam a malha
        let ex = a.excedentes(MAX_COM_RESULTADO);
        assert_eq!(ex.len(), 3);
        assert!(ex.contains(&1) && !ex.contains(&a.ativa()));
    }

    fn caixa(r: &RecomputeResult) -> ([f32; 3], [f32; 3]) {
        let mut lo = [f32::MAX; 3];
        let mut hi = [f32::MIN; 3];
        for b in &r.bodies {
            for p in b.mesh.positions.chunks(3) {
                for k in 0..3 {
                    lo[k] = lo[k].min(p[k]);
                    hi[k] = hi[k].max(p[k]);
                }
            }
        }
        (lo, hi)
    }

    fn volume(r: &RecomputeResult) -> f64 {
        r.bodies.iter().map(|b| b.mass.volume).sum()
    }

    fn aberto(doc: Document) -> DocAberto {
        let r = fenkra_solidos::recompute(&doc);
        let mut d = DocAberto::de(OpenDoc { doc, entry: None, frozen: None, file: None, modified: false, ramo: None }, crate::historico::Historico::new(64), Bench::Solidos);
        d.result = Some(r);
        d.a_recalcular = false;
        d
    }

    /// Duas peças abertas, uma editada: cada uma guarda o seu estado ao alternar, e fechar pergunta só
    /// pela editada.
    #[test]
    fn duas_pecas_guardam_o_proprio_estado_ao_alternar() {
        let exemplos = crate::examples::all();
        let placa = exemplos[0].doc.clone();
        let flange = exemplos[1].doc.clone();
        let mut placa = aberto(placa);
        let flange = aberto(flange);
        let (v_placa, cx_placa) = (volume(placa.result.as_ref().unwrap()), caixa(placa.result.as_ref().unwrap()));
        let (v_flange, cx_flange) = (volume(flange.result.as_ref().unwrap()), caixa(flange.result.as_ref().unwrap()));
        assert!(v_placa > 1. && v_flange > 1.);
        assert!((v_placa - v_flange).abs() > 1.);

        // editar a placa: L de 120 para 150, com desfazer
        let antes = placa.open.doc.clone();
        placa.open.doc.params.iter_mut().find(|p| p.name == "L").unwrap().expr = "150".into();
        placa.hist.registrar(antes, &placa.open.doc.clone());
        placa.open.modified = true;
        placa.result = Some(fenkra_solidos::recompute(&placa.open.doc));
        let r = placa.result.as_ref().unwrap();
        assert!(r.status.values().all(|s| matches!(s, fenkra_solidos::Status::Ok | fenkra_solidos::Status::Inactive)), "{:?}", r.status);
        let v_editada = volume(r);
        let cx_editada = caixa(r);
        assert!(v_editada > v_placa * 1.1, "{v_editada} {v_placa}");
        assert!((cx_editada.1[0] - cx_editada.0[0]) - (cx_placa.1[0] - cx_placa.0[0]) > 25.);
        let status_placa = r.status.clone();
        let status_flange = flange.result.as_ref().unwrap().status.clone();

        // placa na frente, flange no fundo; alterna três vezes
        let mut abas: Abas<DocAberto> = Abas::nova();
        let mut frente = placa;
        let id_flange = abas.abrir(frente);
        frente = flange;
        let id_placa = 1;
        for _ in 0..3 {
            frente = abas.trocar(id_placa, frente).ok().unwrap();
            assert_eq!(frente.open.doc.name, exemplos[0].doc.name);
            frente = abas.trocar(id_flange, frente).ok().unwrap();
            assert_eq!(frente.open.doc.name, exemplos[1].doc.name);
        }
        // a flange (na frente) e a placa (no fundo) continuam com o próprio resultado, sem recalcular
        let rf = frente.result.as_ref().unwrap();
        assert_eq!(rf.status, status_flange);
        assert!((volume(rf) - v_flange).abs() < 1e-9);
        assert_eq!(caixa(rf), cx_flange);
        assert!(!frente.a_recalcular && !frente.modificado() && !frente.hist.pode_desfazer());
        let placa = abas.guardado(id_placa).unwrap();
        let rp = placa.result.as_ref().unwrap();
        assert_eq!(rp.status, status_placa);
        assert!((volume(rp) - v_editada).abs() < 1e-9);
        assert_eq!(caixa(rp), cx_editada);
        assert!(placa.hist.pode_desfazer());
        assert_eq!(placa.open.doc.params.iter().find(|p| p.name == "L").unwrap().expr, "150");

        // fechar pergunta só pela editada
        assert!(placa.modificado());
        assert!(!frente.modificado());
        let modificadas: Vec<u64> = abas.ids().into_iter().filter(|&id| abas.guardado(id).is_some_and(DocAberto::modificado)).collect();
        assert_eq!(modificadas, vec![id_placa]);

        // desfazer na placa volta ao volume original
        let mut placa = abas.trocar(id_placa, frente).ok().unwrap();
        let atual = placa.open.doc.clone();
        let (doc, _) = placa.hist.desfazer(atual).unwrap();
        let r = fenkra_solidos::recompute(&doc);
        assert!((volume(&r) - v_placa).abs() < 1e-6);
        placa.open.doc = doc;

        // soltar a malha marca para recalcular, sem perder documento nem desfazer
        placa.soltar_resultado();
        assert!(placa.result.is_none() && placa.a_recalcular && placa.hist.pode_refazer());
    }

    #[test]
    fn textos_dos_documentos_abertos_tem_traducao() {
        for t in [
            "Documentos abertos",
            "Ctrl+Tab passa para o próximo; Ctrl+Shift+Tab volta.",
            "Próximo documento",
            "Documento anterior",
            "Fechar documento",
            "Fecha o documento da frente; pergunta antes se ele tem alterações por salvar.",
            "ainda não salvo",
            "com alterações por salvar",
            "sem alterações por salvar",
            "Peça",
            "Montagem",
            "Desenho",
        ] {
            assert!(crate::i18n::tem_traducao(t), "{t}");
        }
    }
}
