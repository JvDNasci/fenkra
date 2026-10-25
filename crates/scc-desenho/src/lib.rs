//! Detalhamento 2D (módulo separado): o desenho referencia peças e montagens por arquivo e gera
//! as vistas por projeção com linhas ocultas — mudou o 3D, o desenho acompanha.
//!
//! - [`Drawing`]: folhas, vistas (padrão, projetada, corte, detalhe, planificada), cotas com
//!   âncoras 3D (associativas), notas/símbolos, tabelas (BOM, furos, dobras, corte) e legenda.
//! - [`layout`]: tudo vira primitivas na folha (mm) — a tela e as exportações usam as mesmas.
//! - [`export`]: PDF, SVG e DXF.

pub mod export;
pub mod layout;
pub mod source;

use scc_core::Id;
use serde::{Deserialize, Serialize};

pub type V2 = [f64; 2];
pub type V3 = [f64; 3];

/// Formato da folha (ISO 216 e ANSI Y14.1).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Format {
    A4,
    A3,
    A2,
    A1,
    A0,
    AnsiA,
    AnsiB,
    AnsiC,
    AnsiD,
    AnsiE,
}

impl Format {
    pub const ALL: [Format; 10] = [Format::A4, Format::A3, Format::A2, Format::A1, Format::A0, Format::AnsiA, Format::AnsiB, Format::AnsiC, Format::AnsiD, Format::AnsiE];
    /// (largura, altura) em paisagem, mm.
    pub fn size(self) -> V2 {
        match self {
            Format::A4 => [297., 210.],
            Format::A3 => [420., 297.],
            Format::A2 => [594., 420.],
            Format::A1 => [841., 594.],
            Format::A0 => [1189., 841.],
            Format::AnsiA => [279.4, 215.9],
            Format::AnsiB => [431.8, 279.4],
            Format::AnsiC => [558.8, 431.8],
            Format::AnsiD => [863.6, 558.8],
            Format::AnsiE => [1117.6, 863.6],
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Format::A4 => "A4",
            Format::A3 => "A3",
            Format::A2 => "A2",
            Format::A1 => "A1",
            Format::A0 => "A0",
            Format::AnsiA => "ANSI A (Carta)",
            Format::AnsiB => "ANSI B",
            Format::AnsiC => "ANSI C",
            Format::AnsiD => "ANSI D",
            Format::AnsiE => "ANSI E",
        }
    }
}

/// Modelo da legenda (carimbo).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum TitleStyle {
    /// ISO 7200: 180 mm, campos de identificação e responsabilidade
    #[default]
    Iso7200,
    /// ABNT NBR 10068/10582: 175 mm, desenhista/verificado/aprovado, unidade e símbolo do diedro
    Abnt,
    /// ANSI Y14.1: bloco de título com SIZE/DWG NO/REV e quadro de tolerâncias gerais
    Ansi,
    /// compacta (A4 e croquis)
    Simples,
}

impl TitleStyle {
    pub const ALL: [TitleStyle; 4] = [TitleStyle::Iso7200, TitleStyle::Abnt, TitleStyle::Ansi, TitleStyle::Simples];
    pub fn label(self) -> &'static str {
        match self {
            TitleStyle::Iso7200 => "ISO 7200",
            TitleStyle::Abnt => "ABNT NBR 10068",
            TitleStyle::Ansi => "ANSI Y14.1",
            TitleStyle::Simples => "Simples",
        }
    }
}

/// Modelos de folha prontos: formato, orientação e legenda.
pub const TEMPLATES: [(&str, Format, bool, TitleStyle); 9] = [
    ("A4 retrato — ABNT", Format::A4, true, TitleStyle::Abnt),
    ("A4 paisagem — simples", Format::A4, false, TitleStyle::Simples),
    ("A3 paisagem — ABNT", Format::A3, false, TitleStyle::Abnt),
    ("A3 paisagem — ISO 7200", Format::A3, false, TitleStyle::Iso7200),
    ("A2 paisagem — ISO 7200", Format::A2, false, TitleStyle::Iso7200),
    ("A1 paisagem — ABNT", Format::A1, false, TitleStyle::Abnt),
    ("A0 paisagem — ISO 7200", Format::A0, false, TitleStyle::Iso7200),
    ("ANSI A retrato — ANSI", Format::AnsiA, true, TitleStyle::Ansi),
    ("ANSI B paisagem — ANSI", Format::AnsiB, false, TitleStyle::Ansi),
];

/// Campos automáticos aceitos nos textos da legenda e das notas.
pub const FIELDS: [(&str, &str); 14] = [
    ("{codigo}", "código (PN) da peça/montagem"),
    ("{descricao}", "descrição (ou nome)"),
    ("{material}", "material"),
    ("{massa}", "massa (kg, pelo material)"),
    ("{volume}", "volume (cm³)"),
    ("{escala}", "escala da primeira vista"),
    ("{folha}", "número da folha"),
    ("{folhas}", "total de folhas"),
    ("{formato}", "formato da folha"),
    ("{data}", "data de hoje"),
    ("{autor}", "usuário do computador"),
    ("{arquivo}", "nome do arquivo de origem"),
    ("{revisao}", "revisão da legenda"),
    ("{titulo}", "título do desenho"),
];

/// Troca os campos `{chave}` pelos valores.
pub fn resolve(text: &str, fields: &std::collections::HashMap<String, String>) -> String {
    let mut out = text.to_string();
    if out.contains('{') {
        for (k, v) in fields {
            out = out.replace(&format!("{{{k}}}"), v);
        }
    }
    out
}

/// Data de hoje (dd/mm/aaaa).
pub fn today() -> String {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) as i64;
    let z = secs / 86400 + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    format!("{d:02}/{m:02}/{y}")
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Sheet {
    pub id: Id,
    pub name: String,
    pub format: Format,
    /// retrato (A4 em pé)
    #[serde(default)]
    pub portrait: bool,
    /// zoneamento da margem (números e letras, ISO 5457)
    #[serde(default = "yes")]
    pub zones: bool,
}

impl Sheet {
    pub fn size(&self) -> V2 {
        let s = self.format.size();
        if self.portrait { [s[1], s[0]] } else { s }
    }
}

/// Origem da geometria da vista: arquivo de peça ou montagem (relativo ao desenho ou absoluto).
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Source {
    pub path: String,
    /// estado de exibição da montagem (coletor de exibição) — vazio = como está
    #[serde(default)]
    pub display_state: String,
    /// componentes da montagem escondidos nesta vista (ids dos nós)
    #[serde(default)]
    pub hidden: Vec<u64>,
}

/// Direção padrão (1º diedro, como na ABNT/ISO E).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Dir {
    Frente,
    Tras,
    Topo,
    Base,
    Esquerda,
    Direita,
    Iso,
}

impl Dir {
    pub const ALL: [Dir; 7] = [Dir::Frente, Dir::Tras, Dir::Topo, Dir::Base, Dir::Esquerda, Dir::Direita, Dir::Iso];
    /// (direção para o observador, eixo x da folha)
    pub fn axes(self) -> (V3, V3) {
        let s = std::f64::consts::FRAC_1_SQRT_2;
        let k = 1. / 3f64.sqrt();
        match self {
            Dir::Frente => ([0., -1., 0.], [1., 0., 0.]),
            Dir::Tras => ([0., 1., 0.], [-1., 0., 0.]),
            Dir::Topo => ([0., 0., 1.], [1., 0., 0.]),
            Dir::Base => ([0., 0., -1.], [1., 0., 0.]),
            Dir::Esquerda => ([-1., 0., 0.], [0., -1., 0.]),
            Dir::Direita => ([1., 0., 0.], [0., 1., 0.]),
            Dir::Iso => ([k, -k, k], [s, s, 0.]),
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Dir::Frente => "Frente",
            Dir::Tras => "Trás",
            Dir::Topo => "Superior",
            Dir::Base => "Inferior",
            Dir::Esquerda => "Lateral esquerda",
            Dir::Direita => "Lateral direita",
            Dir::Iso => "Isométrica",
        }
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum ViewKind {
    Standard(Dir),
    /// vista projetada de outra (alinhada a ela; a direção vem da posição: acima, ao lado…)
    Projected { parent: Id, dir: Dir },
    /// corte pela linha a–b da vista mãe (coordenadas da vista mãe, mm do modelo)
    Section { parent: Id, a: V2, b: V2, label: String },
    /// detalhe ampliado de um círculo da vista mãe
    Detail { parent: Id, center: V2, radius: f64, label: String },
    /// chapa planificada vista de cima
    Flat,
    /// esquema cinemático da montagem: juntas (símbolos), barras entre as juntas de cada peça e
    /// terra nas peças fixas
    Schematic(Dir),
    /// direção livre (ex.: a câmera do 3D): para o observador e eixo x da folha
    Camera { toward: V3, x: V3 },
}

/// Estilo de renderização da vista.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum Render {
    /// só arestas (com ou sem ocultas)
    #[default]
    Arestas,
    /// faces sombreadas em cinza, sem arestas
    Sombreado,
    /// faces sombreadas e arestas visíveis por cima
    SombreadoArestas,
}

impl Render {
    pub const ALL: [Render; 3] = [Render::Arestas, Render::Sombreado, Render::SombreadoArestas];
    pub fn label(self) -> &'static str {
        match self {
            Render::Arestas => "Arestas",
            Render::Sombreado => "Sombreado",
            Render::SombreadoArestas => "Sombreado com arestas",
        }
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct View {
    pub id: Id,
    pub sheet: Id,
    pub name: String,
    pub source: Source,
    pub kind: ViewKind,
    /// centro da vista na folha (mm)
    pub pos: V2,
    /// escala (1 = 1:1; 0,5 = 1:2)
    pub scale: f64,
    #[serde(default)]
    pub hidden_lines: bool,
    #[serde(default = "yes")]
    pub tangent_edges: bool,
    #[serde(default = "yes")]
    pub label: bool,
    #[serde(default)]
    pub render: Render,
    /// cor e espessura (mm) das arestas visíveis — None/0 = padrão
    #[serde(default)]
    pub color: Option<[u8; 3]>,
    #[serde(default)]
    pub line_width: f64,
    /// estilo herdado pelas cotas e anotações desta vista
    #[serde(default)]
    pub ann: scc_core::pmi::PmiStyle,
}

fn yes() -> bool {
    true
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum DimKind {
    Horizontal,
    Vertical,
    Aligned,
    Radius,
    Diameter,
    Angle,
}

impl DimKind {
    pub fn label(self) -> &'static str {
        match self {
            DimKind::Horizontal => "Horizontal",
            DimKind::Vertical => "Vertical",
            DimKind::Aligned => "Alinhada",
            DimKind::Radius => "Raio",
            DimKind::Diameter => "Diâmetro",
            DimKind::Angle => "Ângulo",
        }
    }
}

/// Cota numa vista. Âncoras em 3D (modelo): a cota acompanha a peça quando ela muda.
/// Raio/diâmetro: âncoras = centro e um ponto do arco. Ângulo: vértice e um ponto de cada lado.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Dim {
    pub id: Id,
    pub view: Id,
    pub kind: DimKind,
    pub anchors: Vec<V3>,
    /// afastamento da linha de cota (mm na folha)
    pub offset: f64,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub tol: String,
    /// estilo próprio (cor, espessura, ponta das setas, texto…) — padrão herda da vista
    #[serde(default)]
    pub style: scc_core::pmi::PmiStyle,
    /// deslocamento do texto em relação ao lugar automático (mm na folha)
    #[serde(default)]
    pub text_pos: Option<V2>,
    /// setas: 0 automático, 1 por dentro, 2 por fora, 3 só a primeira, 4 só a segunda, 5 nenhuma
    #[serde(default)]
    pub arrows: u8,
}

impl Default for Dim {
    fn default() -> Self {
        Dim { id: Id(0), view: Id(0), kind: DimKind::Horizontal, anchors: vec![], offset: 8., text: String::new(), tol: String::new(), style: Default::default(), text_pos: None, arrows: 0 }
    }
}

/// Setas da cota.
pub const DIM_ARROWS: [&str; 6] = ["automático", "por dentro", "por fora", "só a primeira", "só a segunda", "sem setas"];

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum NoteKind {
    Text,
    /// balão com número de item
    Balloon,
    Roughness,
    Weld,
    /// marca de centro / linha de centro
    CenterMark,
    /// quadro de controle geométrico (ISO 1101) — dados em `Note::gdt`
    Gdt,
    /// símbolo de referência (datum): o texto é a letra
    Datum,
}

/// Quadro de controle geométrico de uma nota.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct DwgGdt {
    pub ch: scc_core::pmi::GdtChar,
    pub tol: String,
    pub datums: String,
    #[serde(default)]
    pub extra: scc_core::pmi::GdtExtra,
}

impl Default for DwgGdt {
    fn default() -> Self {
        DwgGdt { ch: scc_core::pmi::GdtChar::Planeza, tol: "0,05".into(), datums: String::new(), extra: Default::default() }
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Note {
    pub id: Id,
    pub sheet: Id,
    pub kind: NoteKind,
    pub pos: V2,
    #[serde(default)]
    pub leader: Option<V2>,
    pub text: String,
    #[serde(default = "size35")]
    pub size: f64,
    /// símbolo de solda completo (ISO 2553); `None` = filete com a medida do texto
    #[serde(default)]
    pub weld: Option<DwgWeld>,
    #[serde(default)]
    pub gdt: Option<DwgGdt>,
    /// estilo (moldura, cor, negrito, justificação, ancoragem, chamada…)
    #[serde(default)]
    pub style: scc_core::pmi::PmiStyle,
    /// vista a que pertence e ponto do modelo onde a chamada encosta (associativo: acompanha
    /// a peça e a vista)
    #[serde(default)]
    pub view: Option<Id>,
    #[serde(default)]
    pub anchor3: Option<V3>,
}

impl Default for Note {
    fn default() -> Self {
        Note { id: Id(0), sheet: Id(0), kind: NoteKind::Text, pos: [0., 0.], leader: None, text: String::new(), size: 3.5, weld: None, gdt: None, style: Default::default(), view: None, anchor3: None }
    }
}

/// Símbolo de solda de uma nota (o texto da nota é a medida da seção).
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct DwgWeld {
    pub kind: scc_core::pmi::WeldKind,
    pub all_around: bool,
    #[serde(default)]
    pub extra: scc_core::pmi::WeldExtra,
}

fn size35() -> f64 {
    3.5
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum TableKind {
    /// lista de materiais da montagem
    Bom,
    Holes,
    Bends,
    CutList,
}

impl TableKind {
    pub fn label(&self) -> &'static str {
        match self {
            TableKind::Bom => "Lista de materiais",
            TableKind::Holes => "Tabela de furos",
            TableKind::Bends => "Tabela de dobras",
            TableKind::CutList => "Lista de corte",
        }
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Table {
    pub id: Id,
    pub sheet: Id,
    pub kind: TableKind,
    pub source: Source,
    /// canto superior esquerdo (mm)
    pub pos: V2,
    /// nível da lista de peças/corte de montagem
    #[serde(default)]
    pub level: scc_montagem::bom::BomLevel,
}

/// Legenda (carimbo).
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct TitleBlock {
    pub company: String,
    pub title: String,
    pub pn: String,
    pub material: String,
    pub drawn_by: String,
    pub date: String,
    pub revision: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub style: TitleStyle,
    #[serde(default)]
    pub checked_by: String,
    #[serde(default)]
    pub approved_by: String,
    /// massa específica para {massa} (g/cm³)
    #[serde(default = "steel")]
    pub density: f64,
}

fn steel() -> f64 {
    7.85
}

impl Default for TitleBlock {
    fn default() -> Self {
        // campos automáticos: vêm das propriedades da peça/montagem da primeira vista
        TitleBlock {
            company: "SolidCadCraft".into(),
            title: "{descricao}".into(),
            pn: "{codigo}".into(),
            material: "{material}".into(),
            drawn_by: "{autor}".into(),
            date: "{data}".into(),
            revision: "A".into(),
            notes: String::new(),
            style: TitleStyle::Iso7200,
            checked_by: String::new(),
            approved_by: String::new(),
            density: 7.85,
        }
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Drawing {
    pub format_version: u32,
    pub name: String,
    next_id: u64,
    pub sheets: Vec<Sheet>,
    pub views: Vec<View>,
    #[serde(default)]
    pub dims: Vec<Dim>,
    #[serde(default)]
    pub notes: Vec<Note>,
    #[serde(default)]
    pub tables: Vec<Table>,
    #[serde(default)]
    pub title: TitleBlock,
    /// altura do texto das cotas (mm) e casas decimais
    #[serde(default = "size35")]
    pub text_height: f64,
    #[serde(default = "one")]
    pub decimals: u8,
}

fn one() -> u8 {
    1
}

impl Drawing {
    pub fn new(name: &str, format: Format) -> Self {
        let mut d = Drawing {
            format_version: 1,
            name: name.into(),
            next_id: 1,
            sheets: vec![],
            views: vec![],
            dims: vec![],
            notes: vec![],
            tables: vec![],
            title: TitleBlock::default(),
            text_height: 3.5,
            decimals: 1,
        };
        d.add_sheet(format);
        d
    }

    pub fn alloc(&mut self) -> Id {
        let id = Id(self.next_id);
        self.next_id += 1;
        id
    }

    pub fn add_sheet(&mut self, format: Format) -> Id {
        let id = self.alloc();
        let n = self.sheets.len() + 1;
        self.sheets.push(Sheet { id, name: format!("Folha {n}"), format, portrait: false, zones: true });
        id
    }

    pub fn sheet(&self, id: Id) -> Option<&Sheet> {
        self.sheets.iter().find(|s| s.id == id)
    }

    pub fn view(&self, id: Id) -> Option<&View> {
        self.views.iter().find(|v| v.id == id)
    }

    pub fn add_view(&mut self, sheet: Id, source: Source, kind: ViewKind, pos: V2, scale: f64) -> Id {
        let id = self.alloc();
        let name = match &kind {
            ViewKind::Standard(d) | ViewKind::Projected { dir: d, .. } => d.label().to_string(),
            ViewKind::Section { label, .. } => format!("Corte {label}-{label}"),
            ViewKind::Detail { label, .. } => format!("Detalhe {label}"),
            ViewKind::Flat => "Planificada".into(),
            ViewKind::Schematic(_) => "Esquema cinemático".into(),
            ViewKind::Camera { .. } => "Vista da câmera".into(),
        };
        self.views.push(View { id, sheet, name, source, kind, pos, scale, hidden_lines: false, tangent_edges: true, label: true, render: Render::Arestas, color: None, line_width: 0., ann: Default::default() });
        id
    }

    /// Três vistas (frente, superior, lateral esquerda — 1º diedro: a superior fica abaixo da
    /// frente e a lateral esquerda à direita) e a isométrica à direita da lateral. Folha com y
    /// para cima.
    pub fn add_standard_set(&mut self, sheet: Id, source: Source, center: V2, scale: f64, gap: f64) -> Vec<Id> {
        let f = self.add_view(sheet, source.clone(), ViewKind::Standard(Dir::Frente), center, scale);
        let t = self.add_view(sheet, source.clone(), ViewKind::Projected { parent: f, dir: Dir::Topo }, [center[0], center[1] - gap], scale);
        let l = self.add_view(sheet, source.clone(), ViewKind::Projected { parent: f, dir: Dir::Esquerda }, [center[0] + gap, center[1]], scale);
        let i = self.add_view(sheet, source, ViewKind::Standard(Dir::Iso), [center[0] + 2. * gap, center[1]], scale);
        vec![f, t, l, i]
    }

    /// Apaga uma vista (com as filhas e as cotas), cota, nota ou tabela.
    pub fn delete(&mut self, id: Id) {
        let mut kill = vec![id];
        loop {
            let more: Vec<Id> = self
                .views
                .iter()
                .filter(|v| !kill.contains(&v.id))
                .filter(|v| match &v.kind {
                    ViewKind::Projected { parent, .. } | ViewKind::Section { parent, .. } | ViewKind::Detail { parent, .. } => kill.contains(parent),
                    _ => false,
                })
                .map(|v| v.id)
                .collect();
            if more.is_empty() {
                break;
            }
            kill.extend(more);
        }
        self.views.retain(|v| !kill.contains(&v.id));
        self.dims.retain(|d| !kill.contains(&d.id) && !kill.contains(&d.view));
        self.notes.retain(|n| n.id != id && !n.view.is_some_and(|v| kill.contains(&v)));
        self.tables.retain(|t| t.id != id);
        if self.sheets.len() > 1 && self.sheets.iter().any(|s| s.id == id) {
            self.sheets.retain(|s| s.id != id);
            let first = self.sheets[0].id;
            for v in &mut self.views {
                if v.sheet == id {
                    v.sheet = first;
                }
            }
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    pub fn from_json(s: &str) -> Result<Self, String> {
        serde_json::from_str(s).map_err(|e| e.to_string())
    }

    /// Texto de um número com as casas decimais do desenho (vírgula decimal).
    pub fn num(&self, v: f64) -> String {
        let s = format!("{:.*}", self.decimals as usize, v);
        let s = if s.contains('.') { s.trim_end_matches('0').trim_end_matches('.').to_string() } else { s };
        s.replace('.', ",")
    }
}

/// Escala em texto (1:2, 2:1…).
pub fn scale_text(s: f64) -> String {
    if s >= 1. {
        format!("{}:1", trim(s))
    } else {
        format!("1:{}", trim(1. / s))
    }
}

fn trim(v: f64) -> String {
    let s = format!("{v:.2}");
    s.trim_end_matches('0').trim_end_matches('.').replace('.', ",")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::Models;

    #[test]
    fn desenho_ida_e_volta_json() {
        let mut d = Drawing::new("Teste", Format::A3);
        let s = d.sheets[0].id;
        let ids = d.add_standard_set(s, Source { path: "p.sccpart".into(), display_state: String::new(), hidden: vec![] }, [120., 180.], 1., 90.);
        assert_eq!(ids.len(), 4);
        let d2 = Drawing::from_json(&d.to_json()).unwrap();
        assert_eq!(d, d2);
        // esquema cinemático de uma manivela (revolução na base fixa)
        use scc_montagem::kin::{JFrame, JRef, JointKind};
        use scc_montagem::{Assembly, NodeKind, Placement};
        let dir = std::env::temp_dir().join("scc_desenho_esquema");
        std::fs::create_dir_all(&dir).unwrap();
        let mut a = Assembly::new("Manivela");
        let b = a.add_component("Base", NodeKind::Part { path: "b.sccpart".into() }, Placement::default(), None);
        a.node_mut(b).unwrap().fixed = true;
        let c = a.add_component("Braço", NodeKind::Part { path: "c.sccpart".into() }, Placement::default(), None);
        let f = JFrame::from_axis([0.; 3], [0., -1., 0.]);
        a.add_joint(JointKind::Revolute, JRef { comp: b, frame: f }, JRef { comp: c, frame: f });
        std::fs::write(dir.join("m.sccasm"), a.to_json()).unwrap();
        let mut e = Drawing::new("Esquema", Format::A4);
        let sh = e.sheets[0].id;
        let v = e.add_view(sh, Source { path: "m.sccasm".into(), display_state: String::new(), hidden: vec![] }, ViewKind::Schematic(Dir::Frente), [100., 100.], 1.);
        let mut m = source::Models::new(Some(dir));
        let p = m.proj(&e, v);
        assert!(p.error.is_none(), "{:?}", p.error);
        assert!(p.lines.len() >= 3, "símbolo da junta + terra: {}", p.lines.len());
        // apagar a frente leva as projetadas junto
        d.delete(ids[0]);
        assert_eq!(d.views.len(), 1);
        assert_eq!(d.num(12.50), "12,5");
        assert_eq!(scale_text(0.5), "1:2");
        assert_eq!(scale_text(2.), "2:1");
    }

    #[test]
    fn hachura_de_um_quadrado() {
        let sq = vec![[0., 0.], [10., 0.], [10., 10.], [0., 10.]];
        let h = layout::hatch(&[sq], 2.);
        assert!(h.len() >= 6, "{}", h.len());
        for (a, b) in h {
            for p in [a, b] {
                assert!(p[0] > -1e-6 && p[0] < 10. + 1e-6 && p[1] > -1e-6 && p[1] < 10. + 1e-6);
            }
        }
        let c = source::clip_circle(&[[-10., 0.], [10., 0.]], [0., 0.], 5.);
        assert_eq!(c.len(), 1);
        assert!((c[0][0][0] + 5.).abs() < 1e-9 && (c[0][1][0] - 5.).abs() < 1e-9);
    }

    #[test]
    fn vistas_corte_detalhe_cotas_tabelas_e_exportacao() {
        if !scc_kernel::AVAILABLE {
            return;
        }
        let dir = std::env::temp_dir().join("scc_desenho_teste");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("placa.sccpart"), scc_solidos::example_document().to_json()).unwrap();
        let src = Source { path: "placa.sccpart".into(), display_state: String::new(), hidden: vec![] };
        let mut d = Drawing::new("Placa", Format::A3);
        let s = d.sheets[0].id;
        let ids = d.add_standard_set(s, src.clone(), [130., 190.], 1., 100.);
        let mut m = Models::new(Some(dir.clone()));
        let front = m.proj(&d, ids[0]);
        assert!(front.error.is_none(), "{:?}", front.error);
        assert!(front.lines.len() > 4, "frente com arestas");
        let top = m.proj(&d, ids[1]);
        assert!(!top.lines.is_empty());
        // frente e superior têm o mesmo centro em x (alinhadas)
        assert!((front.center[0] - top.center[0]).abs() < 1e-6);
        // corte horizontal no meio da frente: tem hachura
        let (lo, hi) = (front.lo, front.hi);
        let ym = (lo[1] + hi[1]) / 2.;
        let sec = d.add_view(s, src.clone(), ViewKind::Section { parent: ids[0], a: [lo[0] - 5., ym], b: [hi[0] + 5., ym], label: "A".into() }, [330., 100.], 1.);
        let ps = m.proj(&d, sec);
        assert!(ps.error.is_none(), "{:?}", ps.error);
        assert!(!ps.hatch.is_empty(), "corte sem hachura");
        let det = d.add_view(s, src.clone(), ViewKind::Detail { parent: ids[0], center: [lo[0], lo[1]], radius: 10., label: "B".into() }, [330., 240.], 2.);
        let pd = m.proj(&d, det);
        assert!(pd.lines.len() >= 2);
        // cota horizontal entre os cantos da frente = largura da placa
        let ax = front.axes.unwrap();
        let a1 = ax.unproject([lo[0], lo[1]]);
        let a2 = ax.unproject([hi[0], lo[1]]);
        let id = d.alloc();
        d.dims.push(Dim { id, view: ids[0], kind: DimKind::Horizontal, anchors: vec![a1, a2], offset: -8., text: String::new(), tol: String::new(), ..Default::default() });
        let v = d.view(ids[0]).unwrap().clone();
        let (val, _) = layout::dim_value(&d.dims[0], &v, &front).unwrap();
        assert!((val - (hi[0] - lo[0])).abs() < 1e-6);
        let nid = d.alloc();
        d.notes.push(Note { id: nid, sheet: s, kind: NoteKind::Balloon, pos: [250., 260.], leader: Some([200., 220.]), text: "1".into(), size: 3.5, weld: None, ..Default::default() });
        let tid = d.alloc();
        d.tables.push(Table { id: tid, sheet: s, kind: TableKind::Holes, source: src.clone(), pos: [25., 120.], level: Default::default() });
        let tid2 = d.alloc();
        d.tables.push(Table { id: tid2, sheet: s, kind: TableKind::Bom, source: src, pos: [25., 60.], level: Default::default() });
        let t = m.table(&d.tables[1], &d);
        assert_eq!(t.rows.len(), 1);
        // GD&T presa à vista, referência, nota com moldura e cota com texto solto
        let a3 = ax.unproject([hi[0] - 5., hi[1]]);
        let gid = d.alloc();
        d.notes.push(Note { id: gid, sheet: s, kind: NoteKind::Gdt, pos: [200., 280.], leader: None, gdt: Some(DwgGdt::default()), view: Some(ids[0]), anchor3: Some(a3), ..Default::default() });
        let fid = d.alloc();
        let style = scc_core::pmi::PmiStyle { frame: scc_core::pmi::PmiFrame::Losango, frame_dash: 1, angle: 30., bold: true, color: Some([200, 0, 0]), ..Default::default() };
        d.notes.push(Note { id: fid, sheet: s, kind: NoteKind::Text, pos: [60., 40.], text: "NOTA".into(), style, ..Default::default() });
        d.dims[0].text_pos = Some([0., 12.]);
        d.dims[0].arrows = 2;
        let items = layout::sheet(&d, s, &mut m);
        assert!(items.len() > 50);
        // planificada de chapa com uma aba: linha de dobra (centro) e o rótulo
        {
            let mut cd = scc_core::Document::new("chapa");
            cd.sheet = Some(scc_core::SheetParams::default());
            let mut data = scc_core::SketchData::default();
            let mut n = 9_000_000u64;
            let mut al = || {
                n += 1;
                Id(n)
            };
            data.add_rect(&mut al, [0., 0.], [100., 50.], false);
            let sk = cd.add_feature(scc_core::FeatureKind::Sketch(scc_core::Sketch { plane: scc_core::PlaneRef::Origin(scc_core::OriginPlane::XY), data }));
            cd.add_feature(scc_core::FeatureKind::SmWall(scc_core::SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }));
            cd.add_feature(scc_core::FeatureKind::SmFlange(scc_core::SmFlange { edge: [100., 25., 2.], length: "20".into(), angle: "90".into(), radius: None, off1: "0".into(), off2: "0".into(), flip: false, relief: None, relief_kind: 0, side: 0, miter: false }));
            std::fs::write(dir.join("chapa.sccpart"), cd.to_json()).unwrap();
            let fv = d.add_view(s, Source { path: "chapa.sccpart".into(), display_state: String::new(), hidden: vec![] }, ViewKind::Flat, [200., 60.], 1.);
            let pf = m.proj(&d, fv);
            assert!(pf.error.is_none(), "{:?}", pf.error);
            assert!(pf.lines.iter().any(|l| l.1 == 5), "linha de dobra na planificada");
            assert_eq!(pf.labels.len(), 1);
            assert!(pf.labels[0].1.contains("90°"), "{}", pf.labels[0].1);
        }
        // a chamada da GD&T termina na aresta projetada
        let tip = layout::to_sheet(d.view(ids[0]).unwrap(), &front, ax.project(a3));
        assert!(items.iter().any(|i| i.owner == Some(gid) && matches!(&i.prim, layout::Prim::Fill { pts } if pts.iter().any(|p| (p[0] - tip[0]).abs() < 1e-6 && (p[1] - tip[1]).abs() < 1e-6))), "ponta da GD&T fora da aresta");
        // nota: moldura tracejada (vários traços), texto girado em negrito e vermelho
        let nf: Vec<&layout::Item> = items.iter().filter(|i| i.owner == Some(fid)).collect();
        assert!(nf.iter().filter(|i| matches!(i.prim, layout::Prim::Line { .. })).count() > 4);
        assert!(nf.iter().any(|i| i.bold && i.color == Some([200, 0, 0]) && matches!(&i.prim, layout::Prim::Text { angle, .. } if (angle - 30.).abs() < 1e-9)));
        // vista sombreada tem triângulos cinza
        let iso = ids[3];
        d.views.iter_mut().find(|v| v.id == iso).unwrap().render = Render::SombreadoArestas;
        let items2 = layout::sheet(&d, s, &mut m);
        assert!(items2.iter().filter(|i| i.owner == Some(iso) && matches!(i.prim, layout::Prim::Fill { .. }) && i.color.is_some()).count() > 10);
        assert!(items.iter().any(|i| matches!(&i.prim, layout::Prim::Text { text, .. } if text.starts_with("CORTE A-A"))));
        let p = dir.join("placa.pdf");
        export::save(&d, s, &mut m, p.to_str().unwrap()).unwrap();
        let bytes = std::fs::read(&p).unwrap();
        assert!(bytes.starts_with(b"%PDF-1.4") && bytes.ends_with(b"%%EOF\n"));
        let sv = dir.join("placa.svg");
        export::save(&d, s, &mut m, sv.to_str().unwrap()).unwrap();
        assert!(std::fs::read_to_string(&sv).unwrap().contains("<polyline"));
        let dx = dir.join("placa.dxf");
        export::save(&d, s, &mut m, dx.to_str().unwrap()).unwrap();
        let txt = std::fs::read_to_string(&dx).unwrap();
        assert!(txt.contains("LINE") && txt.trim_end().ends_with("EOF"));
    }
}
