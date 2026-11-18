// SPDX-License-Identifier: LGPL-2.1-or-later
//! Layout das barras de comando e tamanho dos painéis de borda (B502, B503, ADR 0104).
//!
//! Cada barra (um grupo de comandos do sistema ou uma caixa montada pela pessoa) tem um lugar: uma das
//! quatro bordas da janela (topo, esquerda, direita, embaixo) ou flutuando num ponto da tela. Fechar a
//! barra no "x" não apaga o lugar: ela continua listada no menu Janela e volta onde estava.
//!
//! O layout é guardado por bancada nas preferências (`Settings::barras`): trocar de bancada carrega o
//! layout dela. Restaurar volta a bancada ao padrão (recursos à direita, navegação embaixo) e não apaga
//! as caixas próprias, só a posição delas.
//!
//! Também mora aqui `painel_na_borda`, o painel de borda que muda de tamanho arrastando a beira e
//! devolve a medida nova para gravar, usado pelas quatro barras e pela coluna da esquerda.

use crate::settings::{Bench, Dock, Group, Settings};
use eframe::egui::{self, Pos2, Rect, Vec2};
use serde::{Deserialize, Serialize};

/// Distância do ponteiro até a borda para a barra solta encaixar nela.
pub const ALCANCE: f32 = 48.;
/// Espessura da prévia de encaixe desenhada na borda.
pub const ESPESSURA_PREVIA: f32 = 40.;

/// Barra na tela: grupo do sistema ou caixa própria (índice em `Settings::custom.groups`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Barra {
    Grupo(Group),
    Minha(usize),
}

/// Como a barra fica gravada: a caixa própria vai pelo nome, que sobrevive a apagar outra caixa.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum ChaveBarra {
    Grupo(Group),
    Minha(String),
}

fn falso(v: &bool) -> bool {
    !*v
}

/// Lugar de uma barra numa bancada.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct PosBarra {
    pub barra: ChaveBarra,
    /// borda em que encaixa, ou `Flutuante`; nunca `Oculta` (fechar é `fechada`, que guarda o lugar)
    pub borda: Dock,
    /// canto da barra flutuante, em pontos da tela
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pos: Option<[f32; 2]>,
    #[serde(default, skip_serializing_if = "falso")]
    pub fechada: bool,
}

/// Bordas que levam painel e mudam de tamanho.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Borda {
    Topo,
    Esquerda,
    Direita,
    Baixo,
}

impl Borda {
    pub const TODAS: [Borda; 4] = [Borda::Topo, Borda::Esquerda, Borda::Direita, Borda::Baixo];

    /// Topo e baixo medem altura; esquerda e direita, largura.
    pub fn horizontal(self) -> bool {
        matches!(self, Borda::Topo | Borda::Baixo)
    }

    pub fn doca(self) -> Dock {
        match self {
            Borda::Topo => Dock::Topo,
            Borda::Esquerda => Dock::Lateral,
            Borda::Direita => Dock::Direita,
            Borda::Baixo => Dock::Baixo,
        }
    }
}

/// Tamanho que a pessoa deu a cada barra de borda. `None`: o tamanho automático (cabe o conteúdo).
#[derive(Clone, Copy, PartialEq, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct TamPaineis {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topo: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub esquerda: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direita: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub baixo: Option<f32>,
}

impl TamPaineis {
    pub fn de(&self, b: Borda) -> Option<f32> {
        match b {
            Borda::Topo => self.topo,
            Borda::Esquerda => self.esquerda,
            Borda::Direita => self.direita,
            Borda::Baixo => self.baixo,
        }
    }

    fn de_mut(&mut self, b: Borda) -> &mut Option<f32> {
        match b {
            Borda::Topo => &mut self.topo,
            Borda::Esquerda => &mut self.esquerda,
            Borda::Direita => &mut self.direita,
            Borda::Baixo => &mut self.baixo,
        }
    }
}

/// Layout das barras de uma bancada: só o que difere do padrão é gravado.
#[derive(Clone, PartialEq, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct LayoutBarras {
    pub bancada: Bench,
    pub barras: Vec<PosBarra>,
    pub tamanhos: TamPaineis,
}

/// Lugar padrão: navegação (vista e seleção) embaixo, o resto à direita (ADR 0094 e 0104).
pub fn lugar_padrao(b: &ChaveBarra) -> Dock {
    match b {
        ChaveBarra::Grupo(g) if g.de_navegacao() => Dock::Baixo,
        _ => Dock::Direita,
    }
}

/// Borda onde o ponteiro solta a barra, ou `None` (fica flutuando). `tela` é a área útil da janela,
/// abaixo do menu; acima dela ainda vale como topo. Perto de duas bordas, vence a mais próxima.
pub fn zona(p: Pos2, tela: Rect) -> Option<Dock> {
    let distancias = [
        (Dock::Topo, p.y - tela.top()),
        (Dock::Baixo, tela.bottom() - p.y),
        (Dock::Lateral, p.x - tela.left()),
        (Dock::Direita, tela.right() - p.x),
    ];
    distancias.into_iter().filter(|(_, d)| *d <= ALCANCE).min_by(|a, b| a.1.total_cmp(&b.1)).map(|(d, _)| d)
}

/// Retângulo da prévia: a faixa da borda onde a barra vai encaixar.
pub fn previa(d: Dock, tela: Rect) -> Option<Rect> {
    let e = ESPESSURA_PREVIA;
    Some(match d {
        Dock::Topo => Rect::from_min_size(tela.min, egui::vec2(tela.width(), e)),
        Dock::Baixo => Rect::from_min_max(egui::pos2(tela.left(), tela.bottom() - e), tela.max),
        Dock::Lateral => Rect::from_min_size(tela.min, egui::vec2(e, tela.height())),
        Dock::Direita => Rect::from_min_max(egui::pos2(tela.right() - e, tela.top()), tela.max),
        _ => return None,
    })
}

/// Barra flutuante dentro da tela: quem trocou de monitor ou de resolução não perde a barra.
pub fn dentro_da_tela(pos: Pos2, tam: Vec2, tela: Rect) -> Pos2 {
    crate::paineis::dentro(pos, tam, tela)
}

/// Põe um comando na caixa própria, na posição `onde` (ou no fim). Se ele já está lá, só muda de lugar.
pub fn por_na_caixa(itens: &mut Vec<String>, chave: &str, onde: Option<usize>) {
    let antes = itens.iter().position(|k| k == chave);
    if let Some(i) = antes {
        itens.remove(i);
    }
    let mut j = onde.unwrap_or(itens.len());
    if let (Some(i), Some(_)) = (antes, onde) {
        if i < j {
            j -= 1;
        }
    }
    itens.insert(j.min(itens.len()), chave.to_string());
}

/// Tira o comando da caixa própria (arrastado para fora dela).
pub fn tirar_da_caixa(itens: &mut Vec<String>, i: usize) {
    if i < itens.len() {
        itens.remove(i);
    }
}

impl Settings {
    fn layout_atual(&self) -> Option<&LayoutBarras> {
        self.barras.iter().find(|l| l.bancada == self.bench)
    }

    fn layout_atual_mut(&mut self) -> &mut LayoutBarras {
        let b = self.bench;
        if let Some(i) = self.barras.iter().position(|l| l.bancada == b) {
            return &mut self.barras[i];
        }
        self.barras.push(LayoutBarras { bancada: b, ..Default::default() });
        self.barras.last_mut().expect("acabou de entrar")
    }

    /// Chave gravada da barra. Caixa própria que não existe mais não tem chave.
    pub fn chave_da_barra(&self, b: Barra) -> Option<ChaveBarra> {
        match b {
            Barra::Grupo(g) => Some(ChaveBarra::Grupo(g)),
            Barra::Minha(i) => self.custom.groups.get(i).map(|c| ChaveBarra::Minha(c.name.clone())),
        }
    }

    /// Nome da barra para a tela.
    pub fn nome_da_barra(&self, b: Barra) -> String {
        match b {
            Barra::Grupo(g) => g.label().to_string(),
            Barra::Minha(i) => self.custom.groups.get(i).map(|c| c.name.clone()).unwrap_or_default(),
        }
    }

    /// Lugar da barra na bancada atual (o padrão, se a pessoa nunca a mexeu nesta bancada).
    pub fn lugar_da_barra(&self, b: Barra) -> PosBarra {
        let Some(chave) = self.chave_da_barra(b) else {
            return PosBarra { barra: ChaveBarra::Minha(String::new()), borda: Dock::Direita, pos: None, fechada: true };
        };
        self.layout_atual().and_then(|l| l.barras.iter().find(|p| p.barra == chave)).cloned().unwrap_or_else(|| PosBarra { borda: lugar_padrao(&chave), barra: chave, pos: None, fechada: false })
    }

    fn guardar_lugar(&mut self, p: PosBarra) {
        let padrao = PosBarra { borda: lugar_padrao(&p.barra), barra: p.barra.clone(), pos: None, fechada: false };
        let l = self.layout_atual_mut();
        match l.barras.iter().position(|x| x.barra == p.barra) {
            // o que volta ao padrão sai do arquivo: restaurar e não mexer dão o mesmo layout gravado
            Some(i) if p == padrao => {
                l.barras.remove(i);
            }
            Some(i) => l.barras[i] = p,
            None if p != padrao => l.barras.push(p),
            None => {}
        }
        self.limpar_layout_vazio();
    }

    fn limpar_layout_vazio(&mut self) {
        self.barras.retain(|l| !l.barras.is_empty() || l.tamanhos != TamPaineis::default());
    }

    /// Onde a barra fica agora: a borda, `Flutuante` ou `Oculta` (fechada).
    pub fn doca_da_barra(&self, b: Barra) -> Dock {
        let p = self.lugar_da_barra(b);
        if p.fechada { Dock::Oculta } else { p.borda }
    }

    /// Leva a barra para uma borda, para flutuar (no último ponto onde flutuou) ou fecha (`Oculta`).
    /// Fechar guarda a borda: reabrir pela lista devolve a barra onde ela estava.
    pub fn mover_barra(&mut self, b: Barra, d: Dock) {
        let mut p = self.lugar_da_barra(b);
        if self.chave_da_barra(b).is_none() {
            return;
        }
        match d {
            Dock::Oculta => p.fechada = true,
            d => {
                p.borda = d;
                p.fechada = false;
            }
        }
        self.guardar_lugar(p);
    }

    /// Reabre (ou fecha) a barra sem mudar o lugar dela.
    pub fn abrir_barra(&mut self, b: Barra, aberta: bool) {
        if self.chave_da_barra(b).is_none() {
            return;
        }
        let mut p = self.lugar_da_barra(b);
        p.fechada = !aberta;
        self.guardar_lugar(p);
    }

    /// Fim do arraste pela alça: numa borda, encaixa; fora delas, flutua com o canto em `canto`.
    pub fn soltar_barra(&mut self, b: Barra, alvo: Option<Dock>, canto: Pos2) {
        if self.chave_da_barra(b).is_none() {
            return;
        }
        let mut p = self.lugar_da_barra(b);
        p.fechada = false;
        match alvo {
            Some(d) if d != Dock::Oculta && d != Dock::Flutuante => p.borda = d,
            _ => {
                p.borda = Dock::Flutuante;
                p.pos = Some([canto.x, canto.y]);
            }
        }
        self.guardar_lugar(p);
    }

    /// Canto gravado da barra flutuante.
    pub fn canto_da_barra(&self, b: Barra) -> Option<Pos2> {
        self.lugar_da_barra(b).pos.map(|[x, y]| egui::pos2(x, y))
    }

    /// Barras visíveis numa borda (ou flutuando), na ordem da bancada; no modo sketch, as do sketch.
    /// Vista e seleção seguem a regra delas (seleção só fora do sketch e na bancada que a tem).
    pub fn barras_em(&self, d: Dock, sketching: bool) -> Vec<Barra> {
        self.barras_da_bancada(sketching).into_iter().filter(|b| self.doca_da_barra(*b) == d).collect()
    }

    /// Todas as barras da bancada (abertas e fechadas): o que o menu Janela lista.
    pub fn barras_da_bancada(&self, sketching: bool) -> Vec<Barra> {
        let comandos = if sketching { crate::organizacao::DO_SKETCH.to_vec() } else { crate::organizacao::ordem(self.bench) };
        let navegacao = [Group::Vista, Group::Selecao].into_iter().filter(|g| if sketching { *g == Group::Vista } else { g.in_bench(self.bench) });
        let mut v: Vec<Barra> = comandos.into_iter().filter(|g| !g.de_navegacao()).chain(navegacao).map(Barra::Grupo).collect();
        v.extend(self.custom.groups.iter().enumerate().filter(|(_, c)| c.sketch == sketching).map(|(i, _)| Barra::Minha(i)));
        v
    }

    /// Volta as barras e os tamanhos da bancada atual ao padrão. As caixas próprias continuam.
    pub fn restaurar_barras(&mut self) {
        let b = self.bench;
        self.barras.retain(|l| l.bancada != b);
    }

    /// Volta todas as bancadas ao padrão. As caixas próprias continuam (só a posição delas volta).
    pub fn restaurar_todas_as_barras(&mut self) {
        self.barras.clear();
    }

    /// Tamanho gravado da barra de uma borda, nesta bancada.
    pub fn tamanho_da_barra(&self, b: Borda) -> Option<f32> {
        self.layout_atual().and_then(|l| l.tamanhos.de(b))
    }

    pub fn guardar_tamanho_da_barra(&mut self, b: Borda, tam: Option<f32>) {
        *self.layout_atual_mut().tamanhos.de_mut(b) = tam;
        self.limpar_layout_vazio();
    }
}

// ---------------------------------------------------------------------------------------------
// Painel de borda que muda de tamanho

fn id_geracao() -> egui::Id {
    egui::Id::unique("fenkra_geracao_dos_paineis")
}

fn id_redimensionando(nome: &str) -> egui::Id {
    egui::Id::unique(("fenkra_redimensionando", nome))
}

/// Depois de restaurar, os painéis esquecem o tamanho que o egui guardou e voltam ao padrão.
pub fn esquecer_tamanhos(ctx: &egui::Context) {
    ctx.data_mut(|d| {
        let g = d.get_temp::<u32>(id_geracao()).unwrap_or(0);
        d.insert_temp(id_geracao(), g.wrapping_add(1));
    });
}

/// Painel numa borda, que muda de tamanho arrastando a beira. `salvo`: o tamanho que a pessoa deu;
/// sem ele, `automatico` (que pode mudar de um quadro para outro, como a largura que cabe os ícones).
/// Devolve o retângulo e, quando a pessoa acaba de soltar a beira, a medida nova para gravar.
pub fn painel_na_borda<R>(root: &mut egui::Ui, nome: &str, borda: Borda, salvo: Option<f32>, automatico: f32, faixa: std::ops::RangeInclusive<f32>, quadro: egui::Frame, conteudo: impl FnOnce(&mut egui::Ui) -> R) -> (R, Rect, Option<f32>) {
    let ctx = root.ctx().clone();
    let geracao = ctx.data(|d| d.get_temp::<u32>(id_geracao())).unwrap_or(0);
    let (lo, hi) = (*faixa.start(), *faixa.end());
    let tam = salvo.unwrap_or(automatico).clamp(lo, hi);
    // sem tamanho gravado, o id muda junto com o automático: o padrão novo vale no quadro seguinte
    let sal = (nome.to_string(), geracao, salvo.is_none().then(|| tam.to_bits()));
    let painel = match borda {
        Borda::Topo => egui::Panel::top(sal.clone()),
        Borda::Baixo => egui::Panel::bottom(sal.clone()),
        Borda::Esquerda => egui::Panel::left(sal.clone()),
        Borda::Direita => egui::Panel::right(sal.clone()),
    };
    let id_beira = root.scope_id().with_salt(egui::IdSalt::new(sal)).with("__resize");
    // o painel do egui mede o conteúdo: ocupar o retângulo inteiro faz valer o tamanho escolhido
    let r = painel.resizable(true).default_size(tam).size_range(lo..=hi).frame(quadro).show(root, |ui| {
        let tudo = ui.max_rect();
        let saida = conteudo(ui);
        ui.expand_to_include_rect(tudo);
        saida
    });
    let medida = if borda.horizontal() { r.response.rect.height() } else { r.response.rect.width() };
    let marca = id_redimensionando(nome);
    if ctx.is_being_dragged(id_beira) {
        ctx.data_mut(|d| d.insert_temp(marca, true));
    }
    let arrastou = ctx.data(|d| d.get_temp::<bool>(marca)).unwrap_or(false);
    let novo = if arrastou && !ctx.input(|i| i.pointer.any_down()) {
        ctx.data_mut(|d| d.remove::<bool>(marca));
        Some(medida.clamp(lo, hi))
    } else {
        None
    };
    (r.inner, r.response.rect, novo)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grupo_da_bancada(s: &Settings) -> Group {
        s.barras_em(Dock::Direita, false).into_iter().find_map(|b| match b {
            Barra::Grupo(g) => Some(g),
            _ => None,
        }).expect("a bancada tem grupo à direita")
    }

    /// B502 (aceite): soltar uma barra deixa flutuando onde soltou; encaixar em outra borda; fechar no
    /// "x" e reabrir pela lista na mesma borda; tudo gravado nas preferências por bancada; restaurar
    /// volta ao layout padrão serializado, com igualdade exata.
    #[test]
    fn soltar_encaixar_fechar_restaurar_e_conferir_o_gravado() {
        let mut s = Settings { bench: Bench::Superficies, ..Default::default() };
        let padrao = serde_json::to_value(&s).unwrap();
        let g = grupo_da_bancada(&s);
        let b = Barra::Grupo(g);
        assert_eq!(s.doca_da_barra(b), Dock::Direita);
        // soltar fora das bordas: flutua com o canto onde soltou
        s.soltar_barra(b, None, egui::pos2(500., 300.));
        assert_eq!(s.doca_da_barra(b), Dock::Flutuante);
        assert_eq!(s.canto_da_barra(b), Some(egui::pos2(500., 300.)));
        assert!(s.barras_em(Dock::Flutuante, false).contains(&b));
        assert!(!s.barras_em(Dock::Direita, false).contains(&b));
        // arrastar até o topo: encaixa lá
        let tela = Rect::from_min_size(egui::pos2(0., 30.), egui::vec2(1600., 900.));
        let alvo = zona(egui::pos2(800., 40.), tela);
        assert_eq!(alvo, Some(Dock::Topo));
        s.soltar_barra(b, alvo, egui::pos2(780., 35.));
        assert_eq!(s.doca_da_barra(b), Dock::Topo);
        assert!(s.barras_em(Dock::Topo, false).contains(&b));
        // fechar no "x": some da tela, mas continua listada e guarda a borda
        s.mover_barra(b, Dock::Oculta);
        assert_eq!(s.doca_da_barra(b), Dock::Oculta);
        assert!(!s.barras_em(Dock::Topo, false).contains(&b));
        assert!(s.barras_da_bancada(false).contains(&b), "barra fechada continua na lista do menu");
        s.abrir_barra(b, true);
        assert_eq!(s.doca_da_barra(b), Dock::Topo, "reabrir devolve a barra onde ela estava");
        // gravado: vai e volta do arquivo igual, e o lugar é desta bancada
        let lido: Settings = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(lido, s);
        assert_eq!(lido.doca_da_barra(b), Dock::Topo);
        assert!(lido.barras.iter().any(|l| l.bancada == Bench::Superficies && l.barras.iter().any(|p| p.barra == ChaveBarra::Grupo(g) && p.borda == Dock::Topo)));
        let mut outra = lido.clone();
        outra.bench = Bench::Solidos;
        assert_eq!(outra.doca_da_barra(b), lugar_padrao(&ChaveBarra::Grupo(g)), "cada bancada tem o seu layout");
        // restaurar: igual ao padrão serializado
        s.restaurar_barras();
        assert_eq!(serde_json::to_value(&s).unwrap(), padrao);
        assert_eq!(s.doca_da_barra(b), Dock::Direita);
    }

    /// Encaixar em cada uma das quatro bordas, e as bordas da prévia ficam onde a barra vai cair.
    #[test]
    fn as_quatro_bordas_recebem_barra_com_previa() {
        let tela = Rect::from_min_size(egui::pos2(0., 30.), egui::vec2(1600., 900.));
        let casos = [(egui::pos2(800., 35.), Dock::Topo), (egui::pos2(800., 925.), Dock::Baixo), (egui::pos2(10., 500.), Dock::Lateral), (egui::pos2(1590., 500.), Dock::Direita)];
        let mut s = Settings::default();
        let b = Barra::Grupo(grupo_da_bancada(&s));
        for (p, d) in casos {
            assert_eq!(zona(p, tela), Some(d), "{p:?}");
            let r = previa(d, tela).unwrap();
            assert!(r.expand(1.).contains(p), "a prévia de {d:?} fica sob o ponteiro: {r:?}");
            s.soltar_barra(b, zona(p, tela), p);
            assert_eq!(s.doca_da_barra(b), d);
            assert!(s.barras_em(d, false).contains(&b));
        }
        // no meio da tela não há encaixe: flutua
        assert_eq!(zona(egui::pos2(800., 500.), tela), None);
        // acima da área útil (no menu) ainda é o topo
        assert_eq!(zona(egui::pos2(800., 10.), tela), Some(Dock::Topo));
        // flutuante fora da tela volta para dentro
        let p = dentro_da_tela(egui::pos2(3000., -50.), egui::vec2(200., 60.), tela);
        assert!(tela.contains(p) && tela.contains(p + egui::vec2(200., 60.)), "{p:?}");
    }

    /// Restaurar tudo volta todas as bancadas sem apagar as caixas próprias; a caixa própria também
    /// solta, encaixa e fecha como as do sistema, e os comandos entram e saem dela arrastados.
    #[test]
    fn caixa_propria_e_restaurar_tudo() {
        let mut s = Settings::default();
        s.custom.groups.push(crate::settings::CustomGroup { name: "Minha caixa".into(), items: vec![], sketch: false });
        let caixa = Barra::Minha(0);
        assert!(s.barras_da_bancada(false).contains(&caixa));
        assert_eq!(s.doca_da_barra(caixa), Dock::Direita);
        let (a, b, c) = ("cmd|A", "cmd|B", "cmd|C");
        let itens = &mut s.custom.groups[0].items;
        por_na_caixa(itens, a, None);
        por_na_caixa(itens, b, None);
        por_na_caixa(itens, c, Some(0));
        assert_eq!(itens, &vec![c.to_string(), a.into(), b.into()]);
        // o mesmo comando arrastado de novo só muda de lugar, não duplica
        por_na_caixa(itens, c, Some(3));
        assert_eq!(itens, &vec![a.to_string(), b.into(), c.into()]);
        tirar_da_caixa(itens, 1);
        assert_eq!(itens, &vec![a.to_string(), c.into()]);
        s.mover_barra(caixa, Dock::Lateral);
        assert_eq!(s.doca_da_barra(caixa), Dock::Lateral);
        s.bench = Bench::Montagem;
        s.mover_barra(Barra::Grupo(Group::Vista), Dock::Topo);
        s.restaurar_todas_as_barras();
        assert!(s.barras.is_empty());
        assert_eq!(s.custom.groups.len(), 1, "restaurar tudo não apaga a caixa própria");
        assert_eq!(s.custom.groups[0].items.len(), 2);
        s.bench = Bench::Solidos;
        assert_eq!(s.doca_da_barra(caixa), Dock::Direita);
        // caixa apagada: o índice velho não mexe em nada
        s.custom.groups.clear();
        s.mover_barra(caixa, Dock::Topo);
        assert!(s.barras.is_empty());
    }

    /// Roda um quadro com o painel de borda e devolve (retângulo, medida para gravar).
    fn quadro(ctx: &egui::Context, eventos: Vec<egui::Event>, borda: Borda, salvo: Option<f32>) -> (Rect, Option<f32>) {
        let tela = Rect::from_min_size(egui::pos2(0., 0.), egui::vec2(1200., 800.));
        let raw = egui::RawInput { screen_rect: Some(tela), events: eventos, ..Default::default() };
        let mut saida = (Rect::NOTHING, None);
        let mut o = ctx.run_ui(raw, |root| {
            let (_, r, novo) = painel_na_borda(root, "teste", borda, salvo, 120., 40.0..=600., egui::Frame::new(), |ui| {
                ui.label("conteúdo");
            });
            saida = (r, novo);
        });
        o.textures_delta.clear();
        saida
    }

    fn botao(pos: Pos2, pressed: bool) -> egui::Event {
        egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() }
    }

    /// B503 (aceite): cada painel de borda (topo, esquerda, direita, baixo) muda de tamanho arrastando
    /// a beira, e a medida nova vem para gravar só quando o botão solta; com o tamanho gravado, o painel
    /// abre com ele. Gravado nas preferências por bancada, vai e volta do arquivo.
    #[test]
    fn cada_painel_muda_de_tamanho_pela_beira_e_fica_gravado() {
        let mut s = Settings::default();
        for borda in Borda::TODAS {
            let ctx = egui::Context::default();
            let (r, _) = quadro(&ctx, vec![], borda, None);
            assert!((if borda.horizontal() { r.height() } else { r.width() } - 120.).abs() < 1.5, "{borda:?} nasce no automático: {r:?}");
            // a beira do painel, por dentro da tela, e o sentido em que ele cresce
            let (beira, cresce) = match borda {
                Borda::Topo => (egui::pos2(600., r.bottom()), egui::vec2(0., 1.)),
                Borda::Baixo => (egui::pos2(600., r.top()), egui::vec2(0., -1.)),
                Borda::Esquerda => (egui::pos2(r.right(), 400.), egui::vec2(1., 0.)),
                Borda::Direita => (egui::pos2(r.left(), 400.), egui::vec2(-1., 0.)),
            };
            let fim = beira + cresce * 80.;
            assert_eq!(quadro(&ctx, vec![egui::Event::PointerMoved(beira)], borda, None).1, None);
            assert_eq!(quadro(&ctx, vec![botao(beira, true)], borda, None).1, None);
            let mut gravar = None;
            for k in 1..=4 {
                let p = beira + cresce * (20. * k as f32);
                let (_, n) = quadro(&ctx, vec![egui::Event::PointerMoved(p)], borda, None);
                assert_eq!(n, None, "{borda:?}: não grava com o botão apertado");
            }
            for ev in [vec![botao(fim, false)], vec![]] {
                if let (_, Some(n)) = quadro(&ctx, ev, borda, None) {
                    gravar = Some(n);
                }
            }
            let n = gravar.unwrap_or_else(|| panic!("{borda:?}: soltar a beira devolve a medida"));
            assert!((n - 200.).abs() < 3., "{borda:?}: {n}");
            s.guardar_tamanho_da_barra(borda, Some(n));
            // gravado: um contexto novo (app reaberto) abre o painel com a medida
            let lido: Settings = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
            assert_eq!(lido.tamanho_da_barra(borda), Some(n));
            let ctx2 = egui::Context::default();
            let (r2, nada) = quadro(&ctx2, vec![], borda, lido.tamanho_da_barra(borda));
            assert_eq!(nada, None);
            assert!((if borda.horizontal() { r2.height() } else { r2.width() } - n).abs() < 1.5, "{borda:?}: {r2:?}");
        }
        // cada bancada guarda os seus; restaurar volta ao automático
        s.bench = Bench::Chapa;
        assert_eq!(s.tamanho_da_barra(Borda::Direita), None);
        s.bench = Bench::Solidos;
        s.restaurar_barras();
        assert!(Borda::TODAS.iter().all(|b| s.tamanho_da_barra(*b).is_none()));
    }
}
