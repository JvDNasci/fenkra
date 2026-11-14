// SPDX-License-Identifier: LGPL-2.1-or-later
//! Configurações do usuário (persistidas em %APPDATA%\Fenkra\config.json).

use crate::render::{DisplayMode, Msaa, ViewStyle};
use eframe::egui;
use fenkra_core::{PresetMalha, QualidadeMalha};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use crate::i18n::tr;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum Theme {
    #[default]
    Escuro,
    Claro,
}

/// Grupos de comandos (blocos da barra). Cada um pode ficar no topo, na lateral, flutuando ou oculto.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Group {
    Sketch,
    Solidos,
    Acabamento,
    Referencias,
    Organizacao,
    Selecao,
    Vista,
    Padroes,
    // bancada de superfícies
    /// criação de pontos, linhas, planos e curvas à mão (bancada de superfícies)
    Wireframe,
    Superficies,
    Operacoes,
    /// comandos híbridos superfície ↔ sólido (nas duas bancadas)
    Hibrido,
    /// análise de superfície (zebra, curvatura) — nas duas bancadas
    Analise,
    // montagem
    Montagem,
    Restricoes,
    Cinematica,
    /// chapa metálica
    Chapa,
    /// membros estruturais
    Estrutural,
    /// primitivas da forma livre
    FormaLivre,
    /// detalhamento 3D (PMI)
    Pmi,
    // detalhamento 2D
    DesArquivo,
    DesVistas,
    DesCotas,
    DesAnotacoes,
    DesTabelas,
    // modo sketch
    SkCriacao,
    SkTransformar,
    SkCotagem,
    SkRestricoes,
    SkFerramentas,
    // B436: grupos por tipo de operação (organizacao.rs)
    Booleanas,
    Primitivas,
    EdicaoDireta,
    Malha,
    ChDobras,
    ChCorte,
    ChTransformar,
    StLigacoes,
    StSolda,
    StSaidas,
    AsmRecursos,
    Biblioteca,
    DesGerar,
}

impl Group {
    pub const ALL: [Group; 43] = [
        Group::DesArquivo,
        Group::DesVistas,
        Group::DesCotas,
        Group::DesAnotacoes,
        Group::DesTabelas,
        Group::Chapa,
        Group::Estrutural,
        Group::FormaLivre,
        Group::Pmi,
        Group::Montagem,
        Group::Restricoes,
        Group::Cinematica,
        Group::Sketch,
        Group::Solidos,
        Group::Acabamento,
        Group::Padroes,
        Group::Wireframe,
        Group::Superficies,
        Group::Operacoes,
        Group::Hibrido,
        Group::Referencias,
        Group::Analise,
        Group::Organizacao,
        Group::Selecao,
        Group::Vista,
        Group::SkCriacao,
        Group::SkTransformar,
        Group::SkCotagem,
        Group::SkRestricoes,
        Group::SkFerramentas,
        Group::Booleanas,
        Group::Primitivas,
        Group::EdicaoDireta,
        Group::Malha,
        Group::ChDobras,
        Group::ChCorte,
        Group::ChTransformar,
        Group::StLigacoes,
        Group::StSolda,
        Group::StSaidas,
        Group::AsmRecursos,
        Group::Biblioteca,
        Group::DesGerar,
    ];
    /// Em que bancada o grupo aparece: os grupos de sketch valem em toda bancada que desenha sketch;
    /// os demais seguem a ordem da bancada (B436, `organizacao::ordem`).
    pub fn in_bench(self, b: Bench) -> bool {
        if self.sketch_only() {
            return matches!(b, Bench::Solidos | Bench::Superficies);
        }
        crate::organizacao::ordem(b).contains(&self)
    }
    /// O grupo é do trabalho atual e fica à vista na faixa; os demais vão para o botão "Mais".
    pub fn do_contexto(self, b: Bench, sketching: bool) -> bool {
        if sketching {
            return self.sketch_only();
        }
        crate::organizacao::no_contexto(b).contains(&self)
    }
    /// Grupos que só aparecem com um sketch em edição.
    /// Navegação (vista e seleção): mora na barra de baixo, não na de recursos (B434).
    pub fn de_navegacao(self) -> bool {
        matches!(self, Group::Vista | Group::Selecao)
    }

    pub fn sketch_only(self) -> bool {
        matches!(self, Group::SkCriacao | Group::SkTransformar | Group::SkCotagem | Group::SkRestricoes | Group::SkFerramentas)
    }
    pub fn label(self) -> &'static str {
        match self {
            Group::Sketch => tr("Sketch"),
            Group::Solidos => tr("Recursos de sketch"),
            Group::Acabamento => tr("Acabamento"),
            Group::Referencias => tr("Referências"),
            Group::Organizacao => tr("Organização"),
            Group::Selecao => tr("Seleção"),
            Group::Vista => tr("Vista"),
            Group::Padroes => tr("Transformações"),
            Group::Superficies => tr("Superfícies"),
            Group::Wireframe => tr("Wireframe (pontos, linhas, planos, curvas)"),
            Group::Operacoes => tr("Operações"),
            Group::Hibrido => tr("Superfície e volume"),
            Group::Analise => tr("Análise"),
            Group::Montagem => tr("Estrutura do produto"),
            Group::Restricoes => tr("Restrições e movimento"),
            Group::Cinematica => tr("Cinemática"),
            Group::Chapa => tr("Paredes"),
            Group::Estrutural => tr("Membros"),
            Group::FormaLivre => tr("Forma livre"),
            Group::Pmi => tr("Detalhamento 3D (PMI)"),
            Group::DesArquivo => tr("Folha"),
            Group::DesVistas => tr("Vistas"),
            Group::DesCotas => tr("Cotas"),
            Group::DesAnotacoes => tr("Anotações e símbolos"),
            Group::DesTabelas => tr("Tabelas"),
            Group::SkCriacao => tr("Criação"),
            Group::SkTransformar => tr("Transformar"),
            Group::SkCotagem => tr("Cotagem"),
            Group::SkRestricoes => tr("Restrições"),
            Group::SkFerramentas => tr("Ferramentas do sketch"),
            Group::Booleanas => tr("Booleanas"),
            Group::Primitivas => tr("Primitivas"),
            Group::EdicaoDireta => tr("Edição direta"),
            Group::Malha => tr("Malha e importação"),
            Group::ChDobras => tr("Dobras"),
            Group::ChCorte => tr("Corte e estampagem"),
            Group::ChTransformar => tr("Transformações da chapa"),
            Group::StLigacoes => tr("Ligações"),
            Group::StSolda => tr("Solda"),
            Group::StSaidas => tr("Saídas"),
            Group::AsmRecursos => tr("Recursos de montagem"),
            Group::Biblioteca => tr("Biblioteca"),
            Group::DesGerar => tr("Gerar cotas"),
        }
    }
}

/// Bancada (módulo) ativa: muda os grupos de comandos; o modelo é o mesmo.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum Bench {
    #[default]
    Solidos,
    Superficies,
    Montagem,
    Chapa,
    Estrutural,
    /// detalhamento 2D (módulo separado: arquivo .fkd)
    Desenho,
    /// modelagem orgânica por subdivisão
    FormaLivre,
}

/// Família de perfis cadastrada: tipo de seção, tamanhos e a regra de criação (ancoragem e rotação
/// padrão quando o membro nasce do arame 3D).
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct UserFamily {
    pub name: String,
    pub kind: fenkra_core::SectionKind,
    pub sizes: Vec<(String, Vec<f64>)>,
    #[serde(default)]
    pub anchor: u8,
    #[serde(default)]
    pub angle: f64,
}

/// Estilo de navegação da câmera (botões do mouse).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum NavStyle {
    #[default]
    Fenkra,
    /// Meio gira, Ctrl+meio move, zoom invertido
    MeioGira,
    /// Shift+meio gira, meio move (absorve dois nomes antigos)
    ShiftMeioGira,
    /// Meio move, meio+direito gira
    MeioMove,
    Blender,
    /// Direito gira, Ctrl+direito ou meio move
    DireitoGira,
    FreeCad,
    /// Direito gira, Shift+direito move, Ctrl+direito dá zoom
    DireitoGiraShift,
}

impl NavStyle {
    pub const ALL: [NavStyle; 8] = [
        NavStyle::Fenkra,
        NavStyle::MeioGira,
        NavStyle::ShiftMeioGira,
        NavStyle::MeioMove,
        NavStyle::FreeCad,
        NavStyle::Blender,
        NavStyle::DireitoGira,
        NavStyle::DireitoGiraShift,
    ];
    pub fn label(self) -> &'static str {
        match self {
            NavStyle::Fenkra => tr("Fenkra"),
            NavStyle::MeioGira => tr("Meio gira, Ctrl+meio move"),
            NavStyle::ShiftMeioGira => tr("Shift+meio gira, meio move"),
            NavStyle::MeioMove => tr("Meio move, meio+direito gira"),
            NavStyle::Blender => tr("Estilo Blender"),
            NavStyle::DireitoGira => tr("Direito gira, Ctrl+direito move"),
            NavStyle::FreeCad => tr("Estilo FreeCAD (CAD)"),
            NavStyle::DireitoGiraShift => tr("Direito gira, Shift+direito move"),
        }
    }
    pub fn help(self) -> &'static str {
        match self {
            NavStyle::Fenkra => tr("Girar: botão esquerdo ou do meio · Mover: direito ou Shift+arrastar · Zoom: roda"),
            NavStyle::MeioGira => tr("Girar: botão do meio · Mover: Ctrl+meio · Zoom: roda (invertida)"),
            NavStyle::ShiftMeioGira => tr("Girar: Shift+meio · Mover: botão do meio · Zoom: roda"),
            NavStyle::MeioMove => tr("Mover: botão do meio · Girar: meio+direito · Zoom: roda"),
            NavStyle::Blender => tr("Girar: botão do meio · Mover: Shift+meio · Zoom: roda ou Ctrl+meio"),
            NavStyle::DireitoGira => tr("Girar: botão direito · Mover: Ctrl+direito ou meio · Zoom: roda"),
            NavStyle::FreeCad => tr("Mover: botão do meio · Girar: meio + esquerdo ou direito · Zoom: roda"),
            NavStyle::DireitoGiraShift => tr("Girar: botão direito · Mover: Shift+direito · Zoom: roda ou Ctrl+direito"),
        }
    }
}

/// Hash FNV-1a 64 do nome em minúsculas. Os nomes antigos de estilos e predefinições, que citavam
/// produtos de terceiros, ficam só como hash: o `config.json` salvo ainda é lido e a regra do projeto
/// (nenhuma marca no código) vale. Ver B202 e B310.
pub(crate) fn hash_nome(n: &str) -> u64 {
    let mut x: u64 = 0xcbf2_9ce4_8422_2325;
    for b in n.to_lowercase().bytes() {
        x = (x ^ b as u64).wrapping_mul(0x0100_0000_01b3);
    }
    x
}

impl NavStyle {
    /// Nome gravado no config.json.
    pub fn nome(self) -> &'static str {
        match self {
            NavStyle::Fenkra => "Fenkra",
            NavStyle::MeioGira => "MeioGira",
            NavStyle::ShiftMeioGira => "ShiftMeioGira",
            NavStyle::MeioMove => "MeioMove",
            NavStyle::Blender => "Blender",
            NavStyle::DireitoGira => "DireitoGira",
            NavStyle::FreeCad => "FreeCad",
            NavStyle::DireitoGiraShift => "DireitoGiraShift",
        }
    }

    /// Lê um nome gravado, novo ou antigo; nome desconhecido cai no padrão, sem erro.
    pub fn de_nome(n: &str) -> NavStyle {
        if let Some(v) = NavStyle::ALL.into_iter().find(|v| v.nome() == n) {
            return v;
        }
        if n == "SolidCadCraft" {
            return NavStyle::Fenkra;
        }
        const LEGADO: [(u64, NavStyle); 6] = [
            (0x8c735c72d6e7e616, NavStyle::MeioGira),
            (0x557ec50bf0c4fc1d, NavStyle::ShiftMeioGira),
            (0x1e2cf6bab7d7815c, NavStyle::ShiftMeioGira),
            (0xc1d4e83b1408f651, NavStyle::MeioMove),
            (0x4f20146f6028e905, NavStyle::DireitoGira),
            (0xb267bd5e49fff919, NavStyle::DireitoGiraShift),
        ];
        let h = hash_nome(n);
        LEGADO.iter().find(|(k, _)| *k == h).map(|(_, v)| *v).unwrap_or_default()
    }
}

impl Serialize for NavStyle {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.nome())
    }
}

impl<'de> Deserialize<'de> for NavStyle {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Ok(NavStyle::de_nome(&String::deserialize(d)?))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Dock {
    Topo,
    Lateral,
    Flutuante,
    Oculta,
    /// fixa na lateral direita
    Direita,
}

impl Dock {
    /// O que a pessoa pode escolher (B434): o grupo aparece na barra dele ou fica oculto. Topo, lateral
    /// esquerda e flutuante continuam no formato do arquivo, para ler preferências antigas.
    pub const ALL: [Dock; 2] = [Dock::Direita, Dock::Oculta];
    pub fn label(self) -> &'static str {
        match self {
            Dock::Oculta => tr("Oculta"),
            _ => tr("Na barra"),
        }
    }
}

/// Filtro de exibição da viewport.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Filter {
    pub corpos: bool,
    pub sketches: bool,
    pub pontos: bool,
    pub planos_origem: bool,
    pub planos_ref: bool,
    pub eixos: bool,
    pub rotulos: bool,
}

impl Default for Filter {
    fn default() -> Self {
        Filter { corpos: true, sketches: true, pontos: true, planos_origem: true, planos_ref: true, eixos: true, rotulos: true }
    }
}

/// Qualidade gráfica: finura da malha, anti-serrilhamento e iluminação da viewport.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Graficos {
    /// ajuste de qualidade da malha (peça e montagem)
    pub preset: PresetMalha,
    /// valores da qualidade "Personalizada"
    pub livre: QualidadeMalha,
    pub msaa: Msaa,
    /// durante órbita, pan e zoom desenha sem anti-serrilhamento e volta ao soltar
    pub simplificar_ao_girar: bool,
    /// iluminação de três pontos com ambiente hemisférico e brilho suave
    pub iluminacao_realista: bool,
    /// B139: sombra suave, oclusão de ambiente e PBR, com o nível de qualidade
    pub efeitos: crate::render_efeitos::Efeitos,
}

impl Default for Graficos {
    fn default() -> Self {
        Graficos { preset: PresetMalha::Normal, livre: QualidadeMalha::NORMAL, msaa: Msaa::X4, simplificar_ao_girar: false, iluminacao_realista: true, efeitos: Default::default() }
    }
}

impl Graficos {
    /// Qualidade da malha em uso.
    pub fn qualidade(&self) -> QualidadeMalha {
        self.preset.qualidade(self.livre)
    }

    /// Escolhe um ajuste de qualidade. Na Personalizada parte dos valores do ajuste anterior; a
    /// iluminação realista acompanha: desligada no Rascunho, ligada nos demais.
    pub fn escolher(&mut self, p: PresetMalha) {
        if p == PresetMalha::Personalizada && self.preset != PresetMalha::Personalizada {
            self.livre = self.qualidade();
        }
        self.preset = p;
        match p {
            PresetMalha::Rascunho => self.iluminacao_realista = false,
            PresetMalha::Normal | PresetMalha::Alta => self.iluminacao_realista = true,
            PresetMalha::Personalizada => {}
        }
    }
}

/// Versão atual do layout da tela. 1: coluna da esquerda e painéis só na direita ou na base (B433).
/// 2: barras de recursos à direita, só com ícones, e navegação embaixo (B434).
pub const LAYOUT_VERSAO: u8 = 2;

/// O que a pessoa tinha arrumado antes de uma migração de layout. Fica no arquivo de preferências,
/// para nada se perder e para a disposição antiga poder ser consultada (ADR 0094).
#[derive(Clone, PartialEq, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct LayoutAnterior {
    pub versao: u8,
    pub paineis: Vec<crate::paineis::PainelSalvo>,
    pub docks: Vec<(Group, Dock)>,
    pub editor_left: bool,
    pub show_labels: bool,
    pub icon_size: f32,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// árvore solta: arrastar o fundo a desloca (travada por padrão)
    pub arvore_solta: bool,
    /// da árvore só ficam os controles do topo
    pub arvore_minimizada: bool,
    /// árvore fora da tela até voltar pelo menu Exibir
    pub arvore_fechada: bool,
    /// largura da coluna da esquerda (árvore e painel do comando), B433
    pub coluna_largura: f32,
    /// até que versão do layout estas preferências já foram migradas (`migrar_layout`). O padrão é
    /// zero porque o `config.json` só guarda o que difere do padrão: arquivo antigo não traz a chave
    pub layout_versao: u8,
    /// a disposição como estava antes da migração de layout, guardada inteira (ADR 0094)
    pub layout_anterior: Option<LayoutAnterior>,
    /// últimas opções do preparo de impressão
    #[serde(default)]
    pub impressao: fenkra_conversor::impressao::OpcoesImpressao,
    /// programa do fatiador externo
    #[serde(default)]
    pub fatiador: String,
    /// pasta do LibreDWG (dwg2dxf e dxf2dwg), programa externo do DWG (B211); vazia: procura no PATH
    #[serde(default)]
    pub libredwg: String,
    /// últimas opções do painel de importação
    #[serde(default)]
    pub opcoes_importar: fenkra_conversor::OpcoesImportar,
    /// últimas opções do painel de exportação
    #[serde(default)]
    pub opcoes_exportar: fenkra_conversor::OpcoesExportar,
    pub theme: Theme,
    /// escala da interface (fontes e controles)
    pub ui_scale: f32,
    pub gradient: bool,
    pub bg_top: [u8; 3],
    pub bg_bottom: [u8; 3],
    pub display: DisplayMode,
    /// B34: o que o mapa de curvatura pinta
    #[serde(default)]
    pub curv_tipo: crate::sup_analise::TipoCurvatura,
    /// B34: raio que recebe a última cor do mapa, em mm
    #[serde(default = "raio_da_curvatura")]
    pub curv_raio: f32,
    /// B180: direção de abertura do mapa de inclinação (0 X, 1 Y, 2 Z), o sentido e o ângulo mínimo
    #[serde(default = "eixo_da_inclinacao")]
    pub incl_eixo: u8,
    #[serde(default)]
    pub incl_inverte: bool,
    #[serde(default = "minimo_da_inclinacao")]
    pub incl_min: f32,
    /// B180: zebra, isofotas e plano de corte das análises
    #[serde(default)]
    pub analise_sup: crate::sup_qualidade::AjustesAnalise,
    pub edges_in_render: bool,
    /// arestas de tangência visíveis (padrão: escondidas, como numa peça real)
    pub tangent_edges: bool,
    pub ortho: bool,
    pub filter: Filter,
    pub docks: Vec<(Group, Dock)>,
    /// variação escolhida em cada família de comandos (chave → índice)
    pub families: Vec<(String, usize)>,
    pub nav: NavStyle,
    pub invert_zoom: bool,
    /// a roda dá zoom em direção ao ponto sob o cursor (desligado: o alvo da câmera fica parado)
    pub zoom_no_cursor: bool,
    /// filtro de seleção na viewport
    pub sel_faces: bool,
    pub sel_edges: bool,
    pub sel_vertices: bool,
    /// projetos abertos recentemente (pastas)
    pub recent: Vec<String>,
    /// peças, montagens e desenhos abertos/salvos recentemente (arquivos)
    pub recent_files: Vec<String>,
    pub show_start: bool,
    pub icon_size: f32,
    pub show_labels: bool,
    /// dica simples: só nome e frase, sem animação nem passo a passo (B377)
    pub dica_simples: bool,
    /// B60: atraso da primeira camada da dica, em milissegundos
    pub atraso_dica_ms: u32,
    // cotagem (usado pelo sketch e pelo desenho)
    pub dim_standard: String,
    pub dim_decimals: u8,
    pub dim_text_height: f32,
    // sketch
    pub grid: bool,
    pub grid_step: f32,
    pub snaps: bool,
    pub auto_constraints: bool,
    /// o apoio de um snap (meio, sobre a curva, quadrante, interseção) vira restrição
    pub snap_restringe: bool,
    /// ponto solto sobre outro ponto vira um ponto só
    pub juntar_ao_soltar: bool,
    pub auto_dims: bool,
    /// valor digitado no campo do cursor vira cota
    pub dim_on_type: bool,
    /// atalhos de teclado (predefinição + trocas)
    pub keys: crate::keymap::Keymap,
    /// roda de comandos: conteúdo trocado pelo usuário e uso aprendido (B256)
    pub roda: crate::roda::Config,
    /// interface em inglês
    pub english: bool,
    /// personalização dos grupos de comandos
    pub custom: Customization,
    /// famílias de perfis cadastradas pelo usuário (biblioteca estrutural)
    #[serde(default)]
    pub user_profiles: Vec<UserFamily>,
    /// bancada ativa (Modelagem Sólida ou Superfícies)
    #[serde(default)]
    pub bench: Bench,
    /// recálculo automático a cada mudança (desligado: só com "Atualizar")
    pub auto_update: bool,
    /// teto de memória do histórico de desfazer, em MB (0 = sem teto)
    pub historico_mb: u32,
    /// painel de edição à esquerda (junto da árvore) em vez da direita
    pub editor_left: bool,
    /// sketch novo num plano (não numa face do corpo) vai para a pasta de geometria
    pub sketch_in_geometry: bool,
    /// símbolo dos pontos de referência: 0 cruz 3D, 1 asterisco, 2 X, 3 círculo, 4 quadrado
    pub point_style: u8,
    /// tamanho do símbolo do ponto (px)
    pub point_size: f32,
    /// espessura (px) e tipo (0 contínua, 1 tracejada, 2 traço-ponto, 3 pontilhada) das linhas e curvas de referência
    pub ref_line_width: f32,
    pub ref_line_dash: u8,
    /// marca dos centros de furo no sketch: 0 asterisco, 1 cruz, 2 círculo com cruz
    pub hole_mark: u8,
    /// seções da árvore ocultadas pelo usuário (chaves de `TREE_SECTIONS`)
    pub tree_hidden: Vec<String>,
    /// seleção em caixa: 0 com Ctrl + arrastar, 1 janela, 2 cruzada, 3 pela direção (arrastar sem Ctrl)
    pub box_mode: u8,
    /// regras de diâmetro por cor que toda montagem nova recebe; vazio usa as de fábrica
    pub regras_cor: Vec<fenkra_montagem::conjuntos::RegraCor>,
    /// roda do mouse nos campos numéricos: 0 só depois de clicar, 1 só com Ctrl, 2 sempre (B227)
    pub wheel_mode: u8,
    /// cores salvas pelo usuário (nome, cor)
    pub user_colors: Vec<(String, [u8; 3])>,
    /// tema dos menus: 0 padrão, 1 grafite (preto/cinza), 2 carmim (preto/vermelho), 3 azul
    pub menu_theme: u8,
    /// tema da interface (pronto ou do usuário)
    pub theme_name: String,
    /// estilo geral das anotações 3D (o que cada anotação deixa em "padrão")
    pub pmi_style: fenkra_core::pmi::PmiStyle,
    /// furos flangeados cadastrados pelo usuário (nome, diâmetro, altura do colar)
    pub flanged_holes: Vec<(String, f64, f64)>,
    /// última macro escrita na janela de automação
    pub macro_src: String,
    /// qualidade gráfica (malha, anti-serrilhamento, iluminação)
    pub graficos: Graficos,
    /// modo leve (B236): combina os ajustes de gráficos para PC fraco
    pub modo_leve: bool,
    /// ajustes de antes de ligar o modo leve
    pub leve_anterior: Option<crate::perfil::Anterior>,
    /// transições da interface e da câmera (desligadas no modo leve)
    pub animacoes: bool,
    /// B347: no máximo quantos quadros por segundo enquanto algo se move
    pub limite_quadros: crate::desempenho::Limite,
    /// B347: sincronismo vertical (cada quadro espera a tela atualizar)
    pub vsync: bool,
    /// B347: mostra os quadros por segundo no canto da vista
    pub contador_fps: bool,
    /// acessibilidade básica (B321): paleta segura, reduzir movimento e escala da fonte
    pub acessibilidade: crate::acessibilidade::Acessibilidade,
    /// mouse 3D (B322): sensibilidade, inversão por eixo, eixo dominante e ação dos botões
    pub mouse3d: crate::mouse3d::Mouse3dConfig,
    /// a sugestão do modo leve já foi mostrada (uma vez só)
    pub sugestao_leve_vista: bool,
    /// curso, perfil de fluxo e progresso das aulas (B323, B324)
    pub curso: crate::curso::EstadoCurso,
    /// posição dos painéis livres, por bancada
    pub paineis: Vec<crate::paineis::PainelSalvo>,
    /// grupos fora do contexto que o usuário deixou sempre na faixa
    pub barra_fixos: Vec<Group>,
}

/// Comandos ocultos, ordem própria em cada grupo e grupos criados pelo usuário.
#[derive(Clone, PartialEq, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Customization {
    /// chaves das entradas escondidas
    pub hidden: Vec<String>,
    /// ordem das entradas por grupo (chaves)
    pub order: Vec<(Group, Vec<String>)>,
    pub groups: Vec<CustomGroup>,
}

/// Grupo de comandos montado pelo usuário (fica na barra de cima).
#[derive(Clone, PartialEq, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct CustomGroup {
    pub name: String,
    pub items: Vec<String>,
    /// aparece no modo sketch (senão, na modelagem)
    pub sketch: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            arvore_solta: false,
            arvore_minimizada: false,
            arvore_fechada: false,
            coluna_largura: crate::paineis::LARGURA_COLUNA,
            layout_versao: 0,
            layout_anterior: None,
            impressao: Default::default(),
            fatiador: String::new(),
            libredwg: String::new(),
            opcoes_importar: Default::default(),
            opcoes_exportar: Default::default(),
            theme: Theme::Claro,
            ui_scale: 1.0,
            gradient: true,
            bg_top: crate::theme::default_def().bg_top,
            bg_bottom: crate::theme::default_def().bg_bottom,
            display: DisplayMode::SombreadoArestas,
            curv_tipo: Default::default(),
            curv_raio: raio_da_curvatura(),
            incl_eixo: eixo_da_inclinacao(),
            incl_inverte: false,
            incl_min: minimo_da_inclinacao(),
            analise_sup: Default::default(),
            edges_in_render: true,
            tangent_edges: false,
            ortho: false,
            filter: Filter::default(),
            docks: Group::ALL.iter().map(|g| (*g, Dock::Direita)).collect(),
            families: vec![],
            nav: NavStyle::Fenkra,
            invert_zoom: false,
            zoom_no_cursor: true,
            sel_faces: true,
            sel_edges: true,
            sel_vertices: true,
            recent: vec![],
            recent_files: vec![],
            show_start: true,
            icon_size: 30.,
            show_labels: true,
            dica_simples: false,
            atraso_dica_ms: crate::ajuda_f1::ATRASO_PADRAO_MS,
            dim_standard: "ABNT NBR 10126".into(),
            dim_decimals: 2,
            dim_text_height: 3.5,
            grid: true,
            grid_step: 10.,
            snaps: true,
            auto_constraints: true,
            snap_restringe: true,
            juntar_ao_soltar: true,
            auto_dims: false,
            dim_on_type: true,
            keys: Default::default(),
            roda: Default::default(),
            english: false,
            custom: Default::default(),
            user_profiles: vec![],
            macro_src: String::new(),
            bench: Default::default(),
            auto_update: true,
            historico_mb: 128,
            editor_left: true,
            sketch_in_geometry: true,
            point_style: 1,
            point_size: 7.,
            ref_line_width: 1.5,
            ref_line_dash: 0,
            hole_mark: 0,
            tree_hidden: vec![],
            box_mode: 0,
            regras_cor: vec![],
            wheel_mode: 0,
            user_colors: vec![],
            menu_theme: 4,
            theme_name: crate::theme::PADRAO.into(),
            pmi_style: Default::default(),
            flanged_holes: vec![],
            graficos: Default::default(),
            modo_leve: false,
            leve_anterior: None,
            animacoes: true,
            limite_quadros: Default::default(),
            vsync: true,
            contador_fps: false,
            acessibilidade: Default::default(),
            mouse3d: Default::default(),
            sugestao_leve_vista: false,
            curso: Default::default(),
            paineis: vec![],
            barra_fixos: vec![],
        }
    }
}

/// Fundos sugeridos: (nome, topo, base).
pub const BACKGROUNDS: [(&str, [u8; 3], [u8; 3]); 6] = [
    ("Grafite", [74, 84, 100], [26, 29, 34]),
    ("Azul noite", [52, 82, 128], [14, 22, 38]),
    ("Estúdio claro", [236, 240, 246], [178, 188, 204]),
    ("Céu", [176, 206, 236], [238, 242, 246]),
    ("Papel", [250, 250, 248], [250, 250, 248]),
    ("Preto", [0, 0, 0], [0, 0, 0]),
];

fn raio_da_curvatura() -> f32 {
    10.
}

fn eixo_da_inclinacao() -> u8 {
    2
}

fn minimo_da_inclinacao() -> f32 {
    1.
}

impl Settings {
    fn path() -> Option<PathBuf> {
        std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join("Fenkra").join("config.json"))
    }

    /// Lê as camadas (programa, empresa, projeto, usuário) e devolve as configurações em vigor (B100).
    pub fn load() -> Self {
        crate::config_camadas::carregar()
    }

    /// Quem tinha um tema que não existe mais (removido) cai no padrão, com o fundo da viewport dele.
    pub fn migrar_tema(mut self) -> Self {
        crate::theme::reload_user();
        if crate::theme::find(&self.theme_name).is_none() {
            let d = crate::theme::default_def();
            self.theme_name = d.name.clone();
            self.theme = if d.dark { Theme::Escuro } else { Theme::Claro };
            self.bg_top = d.bg_top;
            self.bg_bottom = d.bg_bottom;
        }
        self
    }

    /// Leva a disposição salva para o layout atual (ADR 0094), sem perder nada: a disposição antiga
    /// fica inteira em `layout_anterior`, e cada painel guarda tamanho, minimizado e fechado.
    ///
    /// Versão 2 (B434): as barras de comandos que estavam no topo, na lateral esquerda ou flutuando vão
    /// para a barra da direita; a oculta continua oculta.
    ///
    /// Versão 1 (B433): painel livre ou na esquerda vai para a direita (largo, para a base), o do topo
    /// vai para a base, janela nativa continua janela. O painel do comando passa a morar na coluna da
    /// esquerda, e a largura que a pessoa tinha dado a ele vira a largura da coluna.
    pub fn migrar_layout(mut self) -> Self {
        use crate::paineis::Encaixe;
        if self.layout_versao >= LAYOUT_VERSAO {
            return self;
        }
        if self.layout_anterior.is_none() {
            self.layout_anterior = Some(LayoutAnterior { versao: self.layout_versao, paineis: self.paineis.clone(), docks: self.docks.clone(), editor_left: self.editor_left, show_labels: self.show_labels, icon_size: self.icon_size });
        }
        if self.layout_versao < 1 {
            let mut largura_do_comando = None;
            for p in &mut self.paineis {
                if p.nome == "comando" {
                    if p.posicao.encaixe != Encaixe::Janela {
                        p.posicao.encaixe = Encaixe::Esquerda;
                        p.posicao.pos = None;
                    }
                    if p.bancada == self.bench || largura_do_comando.is_none() {
                        largura_do_comando = p.posicao.largura.or(largura_do_comando);
                    }
                } else {
                    p.posicao.encaixe = p.posicao.encaixe.efetivo_para(p.posicao.largura.unwrap_or(0.));
                }
            }
            if let Some(l) = largura_do_comando {
                self.coluna_largura = l.clamp(240., 560.);
            }
        }
        if self.layout_versao < 2 {
            // B434: toda barra visível vai para o lugar dela (direita ou base); a oculta continua oculta
            for (_, d) in &mut self.docks {
                if *d != Dock::Oculta {
                    *d = Dock::Direita;
                }
            }
        }
        self.layout_versao = LAYOUT_VERSAO;
        self
    }

    pub fn save(&self) {
        // execuções de demonstração/teste (variáveis FENKRA_DEMO_*, FENKRA_ABRIR) não mexem nas preferências
        if std::env::vars().any(|(k, _)| k.starts_with("FENKRA_DEMO") || k == "FENKRA_ABRIR" || k == "FENKRA_BANCADA") {
            return;
        }
        // com backup do arquivo anterior; se falhar, o aviso aparece na interface (B100)
        let _ = crate::config_camadas::gravar(self);
    }

    pub fn family(&self, key: &str) -> usize {
        self.families.iter().find(|(k, _)| k == key).map(|(_, i)| *i).unwrap_or(0)
    }
    pub fn set_family(&mut self, key: &str, i: usize) {
        match self.families.iter_mut().find(|(k, _)| k == key) {
            Some(e) => e.1 = i,
            None => self.families.push((key.to_string(), i)),
        }
    }
    pub fn add_recent(&mut self, path: &str) {
        self.recent.retain(|p| p != path);
        self.recent.insert(0, path.to_string());
        self.recent.truncate(8);
    }

    pub fn add_recent_file(&mut self, path: &std::path::Path) {
        let p = path.to_string_lossy().to_string();
        self.recent_files.retain(|x| *x != p);
        self.recent_files.insert(0, p);
        self.recent_files.truncate(12);
        self.save();
    }

    /// Onde o grupo fica: na barra dele (recursos à direita; vista e seleção embaixo) ou oculto. Doca
    /// antiga (topo, lateral esquerda, flutuante) vale como "na barra" (ADR 0094).
    pub fn dock(&self, g: Group) -> Dock {
        match self.docks.iter().find(|(x, _)| *x == g).map(|(_, d)| *d) {
            Some(Dock::Oculta) => Dock::Oculta,
            _ => Dock::Direita,
        }
    }

    pub fn set_dock(&mut self, g: Group, d: Dock) {
        match self.docks.iter_mut().find(|(x, _)| *x == g) {
            Some(e) => e.1 = d,
            None => self.docks.push((g, d)),
        }
    }

    pub fn style(&self) -> ViewStyle {
        ViewStyle {
            mode: self.display,
            bg_top: self.bg_top,
            bg_bottom: self.bg_bottom,
            gradient: self.gradient,
            edges_in_render: self.edges_in_render,
            msaa: self.graficos.msaa.amostras(),
            simplificar: self.graficos.simplificar_ao_girar,
            navegando: false,
            leve: self.modo_leve,
            realista: self.graficos.iluminacao_realista,
            secao: None,
            mapa: None,
            efeitos: crate::render_efeitos::demo(self.graficos.efeitos),
        }
    }

    pub fn apply(&self, ctx: &egui::Context) {
        let def = crate::theme::find(&self.theme_name).unwrap_or_else(crate::theme::default_def);
        crate::theme::apply(ctx, &def);
        ctx.set_zoom_factor(self.ui_scale.clamp(0.7, 2.0));
        crate::acessibilidade::definir(&self.acessibilidade);
        crate::acessibilidade::aplicar_fonte(ctx, &self.acessibilidade);
        // sem animações (modo leve) as transições do egui terminam no mesmo quadro
        ctx.global_style_mut(|s| s.animation_time = if self.animacoes && !self.acessibilidade.reduzir_movimento { egui::Style::default().animation_time } else { 0.0 });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// B433/ADR 0094: a disposição salva migra para o layout novo sem perder nada. Painel livre vai
    /// para a direita, o largo e o do topo vão para a base, janela nativa continua; tamanho, minimizado
    /// e fechado ficam; a largura do painel do comando vira a da coluna; a disposição antiga fica
    /// guardada inteira; migrar duas vezes não muda mais nada.
    #[test]
    fn layout_antigo_migra_sem_perder_nada() {
        use crate::paineis::{Encaixe, PainelSalvo, Posicao};
        let salvo = |b: Bench, n: &str, p: Posicao| PainelSalvo { bancada: b, nome: n.into(), posicao: p };
        let mut s = Settings::default();
        assert_eq!(s.layout_versao, 0, "o padrão é zero: arquivo antigo não traz a chave");
        s.editor_left = false;
        s.docks = Group::ALL.iter().map(|g| (*g, Dock::Topo)).collect();
        s.set_dock(Group::Sketch, Dock::Flutuante);
        s.set_dock(Group::Solidos, Dock::Lateral);
        s.set_dock(Group::Padroes, Dock::Oculta);
        s.paineis = vec![
            salvo(Bench::Solidos, "comando", Posicao { encaixe: Encaixe::Livre, pos: Some([420., 180.]), largura: Some(380.), ..Default::default() }),
            salvo(Bench::Solidos, "features", Posicao { encaixe: Encaixe::Livre, pos: Some([500., 300.]), largura: Some(310.), altura: Some(222.), minimizado: true, fechado: true }),
            salvo(Bench::Estrutural, "lista", Posicao { encaixe: Encaixe::Livre, pos: Some([18., 22.]), largura: Some(650.), ..Default::default() }),
            salvo(Bench::Montagem, "bom", Posicao { encaixe: Encaixe::Topo, pos: Some([300., 0.]), ..Default::default() }),
            salvo(Bench::Montagem, "sim", Posicao { encaixe: Encaixe::Esquerda, ..Default::default() }),
            salvo(Bench::Montagem, "comando", Posicao { encaixe: Encaixe::Janela, pos: Some([1950., 200.]), largura: Some(320.), altura: Some(460.), ..Default::default() }),
            salvo(Bench::Chapa, "dfm", Posicao { encaixe: Encaixe::Base, pos: Some([320., 640.]), ..Default::default() }),
        ];
        let antes = s.clone();
        // vai e volta do arquivo como um config antigo: sem as chaves novas
        let mut v = serde_json::to_value(&s).unwrap();
        for k in ["layout_versao", "layout_anterior", "coluna_largura"] {
            v.as_object_mut().unwrap().remove(k);
        }
        let m = serde_json::from_value::<Settings>(v).unwrap().migrar_layout();
        assert_eq!(m.layout_versao, LAYOUT_VERSAO);
        let de = |b: Bench, n: &str| m.paineis.iter().find(|p| p.bancada == b && p.nome == n).unwrap().posicao;
        assert_eq!(m.paineis.len(), antes.paineis.len(), "nenhum painel some");
        let f = de(Bench::Solidos, "features");
        assert_eq!((f.encaixe, f.largura, f.altura, f.minimizado, f.fechado), (Encaixe::Direita, Some(310.), Some(222.), true, true));
        assert_eq!(de(Bench::Estrutural, "lista").encaixe, Encaixe::Base, "painel largo vai para a base");
        assert_eq!(de(Bench::Estrutural, "lista").largura, Some(650.));
        assert_eq!(de(Bench::Montagem, "bom").encaixe, Encaixe::Base, "o que estava no topo vai para a base");
        assert_eq!(de(Bench::Montagem, "sim").encaixe, Encaixe::Direita);
        assert_eq!(de(Bench::Chapa, "dfm"), antes.paineis[6].posicao, "o que já estava na base não muda");
        assert_eq!(de(Bench::Montagem, "comando"), antes.paineis[5].posicao, "janela em outro monitor continua lá");
        assert_eq!(de(Bench::Solidos, "comando").encaixe, Encaixe::Esquerda, "o comando mora na coluna");
        assert_eq!(m.coluna_largura, 380., "a largura que o painel do comando tinha vira a da coluna");
        // B434: as barras vão para o lugar delas e a que estava oculta continua oculta
        assert_eq!(m.dock(Group::Sketch), Dock::Direita);
        assert_eq!(m.dock(Group::Padroes), Dock::Oculta);
        assert!(m.docks.iter().all(|(_, d)| matches!(d, Dock::Direita | Dock::Oculta)), "{:?}", m.docks);
        assert_eq!(m.docks.len(), antes.docks.len(), "nenhum grupo some da lista");
        // nada migrado fica livre sobre a peça
        assert!(m.paineis.iter().all(|p| p.nome == "comando" || matches!(p.posicao.encaixe, Encaixe::Direita | Encaixe::Base)));
        // a disposição antiga fica guardada inteira
        let velho = m.layout_anterior.clone().expect("guardado");
        assert_eq!((velho.versao, &velho.paineis, &velho.docks, velho.editor_left), (0, &antes.paineis, &antes.docks, false));
        // todo o resto das preferências fica como estava
        let mut iguais = m.clone();
        iguais.paineis = antes.paineis.clone();
        iguais.docks = antes.docks.clone();
        iguais.coluna_largura = antes.coluna_largura;
        iguais.layout_versao = 0;
        iguais.layout_anterior = None;
        assert_eq!(iguais, antes);
        // migrar de novo, ou depois de ir e voltar do arquivo, não muda nada
        let de_novo: Settings = serde_json::from_str(&serde_json::to_string(&m).unwrap()).unwrap();
        assert_eq!(de_novo.clone().migrar_layout(), m);
    }

    #[test]
    fn configuracao_antiga_sem_graficos_usa_os_padroes() {
        let s: Settings = serde_json::from_str(r#"{"ui_scale": 1.2, "grid": false}"#).unwrap();
        assert_eq!(s.graficos, Graficos::default());
        assert_eq!(s.graficos.qualidade(), QualidadeMalha::NORMAL);
        assert_eq!(s.graficos.msaa, Msaa::X4);
        assert!(s.graficos.iluminacao_realista && !s.graficos.simplificar_ao_girar);
        // campos soltos da seção também têm padrão
        let g: Settings = serde_json::from_str(r#"{"graficos": {"preset": "Alta"}}"#).unwrap();
        assert_eq!(g.graficos.qualidade(), QualidadeMalha::ALTA);
        assert_eq!(g.graficos.msaa, Msaa::X4);
    }

    /// Nomes antigos (montados em pedaços para o código não citar marca) migram para o equivalente.
    #[test]
    fn config_com_estilo_e_predefinicao_antigos_migra_sem_erro() {
        let nome = |a: &str, b: &str| [a, b].concat();
        let casos = [
            (nome("Solid", "Works"), NavStyle::MeioGira, crate::keymap::Preset::VistasCtrl),
            (nome("Cat", "ia"), NavStyle::MeioMove, crate::keymap::Preset::PoucosAtalhos),
            (nome("Fus", "ion"), NavStyle::ShiftMeioGira, crate::keymap::Preset::LetraF6),
            (nome("Onsh", "ape"), NavStyle::DireitoGira, crate::keymap::Preset::VistasShift),
            (nome("Inven", "tor"), NavStyle::ShiftMeioGira, crate::keymap::Preset::LetraHome),
        ];
        for (antigo, nav, preset) in casos {
            let j = format!(r#"{{"nav": "{antigo}", "keys": {{"preset": "{antigo}"}}}}"#);
            let s: Settings = serde_json::from_str(&j).unwrap();
            assert_eq!(s.nav, nav, "{antigo}");
            assert_eq!(s.keys.preset, preset, "{antigo}");
        }
        let rhino: Settings = serde_json::from_str(&format!(r#"{{"nav": "{}"}}"#, nome("Rhi", "no"))).unwrap();
        assert_eq!(rhino.nav, NavStyle::DireitoGiraShift);
        // nome desconhecido não derruba a leitura
        let x: Settings = serde_json::from_str(r#"{"nav": "Qualquer", "keys": {"preset": "Outro"}}"#).unwrap();
        assert_eq!((x.nav, x.keys.preset), (NavStyle::Fenkra, crate::keymap::Preset::Fenkra));
    }

    #[test]
    fn nomes_novos_de_estilo_e_predefinicao_vao_e_voltam() {
        for n in NavStyle::ALL {
            let j = serde_json::to_string(&n).unwrap();
            assert_eq!(serde_json::from_str::<NavStyle>(&j).unwrap(), n);
        }
        for p in crate::keymap::Preset::ALL {
            let j = serde_json::to_string(&p).unwrap();
            assert_eq!(serde_json::from_str::<crate::keymap::Preset>(&j).unwrap(), p);
        }
    }

    #[test]
    fn graficos_vao_e_voltam_do_arquivo() {
        let mut s = Settings::default();
        s.graficos.escolher(PresetMalha::Personalizada);
        s.graficos.livre = QualidadeMalha { desvio_rel: 0.001, angulo_graus: 15. };
        s.graficos.msaa = Msaa::X8;
        s.graficos.simplificar_ao_girar = true;
        let volta: Settings = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(volta.graficos, s.graficos);
        assert_eq!(volta.graficos.qualidade(), QualidadeMalha { desvio_rel: 0.001, angulo_graus: 15. });
    }

    #[test]
    fn escolher_preset_ajusta_a_iluminacao_e_parte_do_anterior() {
        let mut g = Graficos::default();
        g.escolher(PresetMalha::Rascunho);
        assert!(!g.iluminacao_realista);
        g.escolher(PresetMalha::Alta);
        assert!(g.iluminacao_realista);
        g.escolher(PresetMalha::Personalizada);
        assert_eq!(g.qualidade(), QualidadeMalha::ALTA, "a Personalizada parte dos valores do ajuste anterior");
    }
}
