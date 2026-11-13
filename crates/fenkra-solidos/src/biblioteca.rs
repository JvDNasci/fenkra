// SPDX-License-Identifier: LGPL-2.1-or-later
//! Biblioteca pública (ADR 0008): famílias de fixadores lidas de arquivos JSON abertos.
//!
//! Os arquivos ficam em `biblioteca/fixadores/` na raiz do repositório, um por família, e são
//! embutidos no binário com `include_str!` (continuam editáveis no repositório). Cada família guarda
//! a norma, as fontes das medidas, o nível de uso (A/B/C/X, ver `docs/biblioteca/fontes-de-dados.md`)
//! e a licença dos dados. As medidas são fatos; a geometria nasce em [`crate::fixadores`].
//!
//! Camadas: hoje só existe a [`Camada::Publica`]. A estrutura já separa "camada" de "família" para
//! que as camadas empresa e usuário/projeto entrem depois, sobrepondo a pública pelo `id`.

use crate::hardware;
use serde::Deserialize;
use std::collections::{BTreeMap, HashSet};
use std::sync::OnceLock;

/// Origem de uma biblioteca. As de cima sobrepõem as de baixo (pública → empresa → usuário).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Camada {
    #[default]
    Publica,
    Empresa,
    Usuario,
}

impl Camada {
    pub fn rotulo(self) -> &'static str {
        match self {
            Camada::Publica => "pública",
            Camada::Empresa => "empresa",
            Camada::Usuario => "usuário",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TipoFixador {
    ParafusoAllen,
    ParafusoSextavado,
    Porca,
    Arruela,
    /// rolamento de esferas pelas medidas de contorno (furo d, externo D, largura B)
    Rolamento,
    /// parafuso de máquina com recesso cruzado na cabeça (ISO 7045)
    ParafusoCruz,
    /// haste roscada nas duas pontas, sem cabeça (DIN 938, 939)
    Prisioneiro,
    /// cilindro pneumático pelas medidas de interface (ISO 15552); o comprimento é o curso
    CilindroPneumatico,
    /// trilho de guia linear por série de fabricante; o comprimento é o do trilho
    TrilhoLinear,
    /// rebite sólido (AN470, AN426); o comprimento L sai da pega: L = pega + 1,5·d, na série
    Rebite,
    /// patim (bloco) de guia linear, que corre sobre o trilho do mesmo tamanho
    PatimLinear,
    /// porca de esferas com flange (DIN 69051-5)
    PorcaEsferas,
    /// fuso de esferas (o eixo); o comprimento é o do fuso
    FusoEsferas,
    /// terminal rotular macho (cabeça com esfera furada e haste roscada), dos braços da direção
    TerminalRotular,
}

/// Geradores de geometria nomeados. Uma família escolhe o gerador pelo nome (`"gerador"` no JSON);
/// nova família que usa um gerador existente é só um arquivo de dados, sem código.
pub const GERADORES: &[(&str, TipoFixador)] = &[
    ("parafuso_allen", TipoFixador::ParafusoAllen),
    ("parafuso_sextavado", TipoFixador::ParafusoSextavado),
    ("porca", TipoFixador::Porca),
    ("arruela", TipoFixador::Arruela),
    ("rolamento", TipoFixador::Rolamento),
    ("parafuso_cruz", TipoFixador::ParafusoCruz),
    ("prisioneiro", TipoFixador::Prisioneiro),
    ("cilindro_pneumatico", TipoFixador::CilindroPneumatico),
    ("cilindro_redondo", TipoFixador::CilindroPneumatico),
    ("trilho_linear", TipoFixador::TrilhoLinear),
    ("rebite_universal", TipoFixador::Rebite),
    ("rebite_escareado", TipoFixador::Rebite),
    ("patim_linear", TipoFixador::PatimLinear),
    ("porca_esferas", TipoFixador::PorcaEsferas),
    ("fuso_esferas", TipoFixador::FusoEsferas),
    ("terminal_rotular", TipoFixador::TerminalRotular),
    // geradores de forma própria sobre um tipo já existente: o tipo diz como a montagem usa a peça
    ("parafuso_escareado", TipoFixador::ParafusoAllen),
    ("parafuso_ombro", TipoFixador::ParafusoAllen),
    ("porca_flangeada", TipoFixador::Porca),
];

/// Campos que o gerador precisa, além dos do tipo.
pub fn campos_do_gerador(gerador: &str) -> &'static [&'static str] {
    match gerador {
        "parafuso_ombro" => &["dr", "b"],
        "porca_flangeada" => &["dc", "c"],
        "prisioneiro" => &["b", "bm"],
        "rebite_universal" => &["k"],
        // perfil quadrado com tirantes (ISO 15552) ou tubo redondo com olhal (ISO 6432)
        "cilindro_pneumatico" => &["TG", "WH", "B", "L8", "RT", "E"],
        "cilindro_redondo" => &["BE", "WF", "XC", "EW", "CD", "D"],
        _ => &[],
    }
}

impl TipoFixador {
    pub fn rotulo(self) -> &'static str {
        match self {
            TipoFixador::ParafusoAllen => "Parafuso Allen",
            TipoFixador::ParafusoSextavado => "Parafuso sextavado",
            TipoFixador::Porca => "Porca",
            TipoFixador::Arruela => "Arruela",
            TipoFixador::Rolamento => "Rolamento",
            TipoFixador::ParafusoCruz => "Parafuso de cabeça com cruz",
            TipoFixador::Prisioneiro => "Prisioneiro",
            TipoFixador::CilindroPneumatico => "Cilindro pneumático",
            TipoFixador::TrilhoLinear => "Trilho de guia linear",
            TipoFixador::Rebite => "Rebite",
            TipoFixador::PatimLinear => "Patim de guia linear",
            TipoFixador::PorcaEsferas => "Porca de esferas",
            TipoFixador::FusoEsferas => "Fuso de esferas",
            TipoFixador::TerminalRotular => "Terminal rotular",
        }
    }

    /// Nome do gerador padrão deste tipo.
    pub fn gerador(self) -> &'static str {
        GERADORES.iter().find(|(_, t)| *t == self).map(|(n, _)| *n).unwrap_or("")
    }

    /// Tipos cuja peça muda de comprimento pela escolha da pessoa: o L do parafuso ou o curso do
    /// cilindro, sempre dentro da série da família.
    pub fn usa_comprimento(self) -> bool {
        self.e_parafuso() || matches!(self, TipoFixador::CilindroPneumatico | TipoFixador::TrilhoLinear | TipoFixador::Rebite | TipoFixador::FusoEsferas)
    }

    /// Entra no furo como parafuso: corpo para dentro, cabeça para fora e L pela espessura (o rebite
    /// também, com o L pela pega).
    pub fn atravessa_furo(self) -> bool {
        self.e_parafuso() || self == TipoFixador::Rebite
    }

    pub fn e_parafuso(self) -> bool {
        matches!(self, TipoFixador::ParafusoAllen | TipoFixador::ParafusoSextavado | TipoFixador::ParafusoCruz | TipoFixador::Prisioneiro)
    }

    /// Campos que a geometria do tipo precisa em todo tamanho.
    pub fn campos_obrigatorios(self) -> &'static [&'static str] {
        match self {
            // "b" é opcional: a cabeça abaulada (ISO 7380-1) não publica comprimento roscado de referência
            TipoFixador::ParafusoAllen => &["P", "dk", "k", "s"],
            TipoFixador::ParafusoSextavado => &["P", "s", "k"],
            TipoFixador::Porca => &["s", "m"],
            TipoFixador::Arruela => &["d1", "d2", "h"],
            // o furo d é o diâmetro nominal do tamanho
            TipoFixador::Rolamento => &["D", "B"],
            TipoFixador::ParafusoCruz => &["P", "dk", "k"],
            TipoFixador::Prisioneiro => &["P"],
            TipoFixador::CilindroPneumatico => &["MM", "KK", "AM"],
            TipoFixador::TrilhoLinear => &["WR", "HR", "D", "h", "dh", "P"],
            TipoFixador::Rebite => &["dk"],
            TipoFixador::PatimLinear => &["H", "H1", "W", "B", "C", "L", "M", "prof", "WR", "HR"],
            TipoFixador::PorcaEsferas => &["P", "D", "A", "B", "L", "H"],
            TipoFixador::FusoEsferas => &["P"],
            // o furo da esfera e a rosca da haste são o diâmetro nominal do tamanho
            TipoFixador::TerminalRotular => &["dk", "C1", "dE", "B", "h", "dh", "l1"],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Fonte {
    pub rotulo: String,
    pub url: String,
    /// página ou tabela do documento, quando a fonte é paginada
    #[serde(default)]
    pub pagina: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Tamanho {
    /// Designação livre como a pessoa escreve: "M6", "1/4-20 UNC", "#10-32", "-06", "Ø4,8".
    pub designacao: String,
    /// Valor original, quando difere da designação (ex.: "1/4 pol" para um tamanho guardado em mm).
    #[serde(default)]
    pub original: Option<String>,
    /// Diâmetro nominal em mm. Obrigatório quando a designação não é "M<diâmetro>".
    #[serde(default)]
    pub d_nominal: Option<f64>,
    /// medidas em mm, uma por nome de `Familia::campos`
    pub valores: BTreeMap<String, f64>,
    /// quantas fontes independentes trazem estes valores
    pub confirmado_por: u32,
    #[serde(default)]
    pub nota: Option<String>,
    /// Série de comprimentos L deste tamanho; vazia = vale a série da família.
    #[serde(default)]
    pub comprimentos: Vec<f64>,
    /// Faixa de espessura da pilha (pega) que o prendedor aceita, em mm.
    #[serde(default)]
    pub grip_min: Option<f64>,
    #[serde(default)]
    pub grip_max: Option<f64>,
    /// Faixa de diâmetro do furo que o prendedor aceita, em mm. Sem ela, vale a faixa de folga
    /// fina a folgada do tamanho métrico (ver [`Familia::faixa_furo`]).
    #[serde(default)]
    pub furo_min: Option<f64>,
    #[serde(default)]
    pub furo_max: Option<f64>,
    /// Número de peça do fabricante ou da norma, quando o tamanho tem um.
    #[serde(default)]
    pub pn: Option<String>,
    /// Massa nominal conferida em fonte, em gramas (itens sem comprimento: porca e arruela).
    #[serde(default)]
    pub massa_g: Option<f64>,
}

impl Tamanho {
    pub fn valor(&self, campo: &str) -> Option<f64> {
        self.valores.get(campo).copied()
    }

    /// Chave de busca: minúsculas, sem espaços e com vírgula decimal virando ponto.
    pub fn chave_de(texto: &str) -> String {
        texto.chars().filter(|c| !c.is_whitespace()).map(|c| if c == ',' { '.' } else { c.to_ascii_lowercase() }).collect()
    }

    pub fn chave(&self) -> String {
        Self::chave_de(&self.designacao)
    }

    /// Diâmetro nominal em mm: o declarado, ou o da designação métrica ("M6" → 6).
    pub fn diametro_nominal(&self) -> Option<f64> {
        self.d_nominal.or_else(|| self.designacao.strip_prefix('M')?.parse::<f64>().ok()).filter(|d| d.is_finite() && *d > 0.)
    }
}

/// Unidades aceitas nos campos (`unidades` da família). Sem declaração, o campo é mm.
const UNIDADES: &[&str] = &["mm", "grau", "g", "adimensional"];

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(try_from = "FamiliaBruta")]
pub struct Familia {
    pub id: String,
    pub nome: String,
    pub norma: String,
    /// revisão da norma de onde vêm as medidas ("2004", "1987")
    pub revisao: String,
    /// versão dos dados desta família; sobe a cada correção de medida
    pub versao: u32,
    pub tipo: TipoFixador,
    /// nome do gerador de geometria (ver [`GERADORES`]); sem ele no JSON, o padrão do tipo
    pub gerador: String,
    /// nível de uso da fonte: A, B, C ou X
    pub nivel: String,
    pub licenca: String,
    pub fontes: Vec<Fonte>,
    /// classe de resistência ou dureza ("8.8", "A2-70"); define o material da peça gerada
    pub classe: Option<String>,
    /// acabamento de superfície ("Zincado"); só informativo
    pub acabamento: Option<String>,
    /// nomes das medidas guardadas em cada tamanho
    pub campos: Vec<String>,
    /// unidade de cada medida (campo ausente = mm)
    pub unidades: BTreeMap<String, String>,
    pub tamanhos: Vec<Tamanho>,
    /// série de comprimentos L padrão (só parafusos); cada tamanho pode ter a sua
    pub comprimentos: Vec<f64>,
    /// camada de onde a família veio
    pub camada: Camada,
    /// Medidas **ilustrativas**, escolhidas pelo Fenkra só para dar forma coerente à peça, sem fonte
    /// de norma ou catálogo (B397). A família diz isso na ficha e na dica; os tamanhos ficam com
    /// `confirmado_por` 0, porque nenhuma fonte os confirma.
    pub ilustrativo: bool,
}

#[derive(Deserialize)]
struct FamiliaBruta {
    id: String,
    nome: String,
    norma: String,
    #[serde(default)]
    revisao: String,
    #[serde(default = "um")]
    versao: u32,
    #[serde(default)]
    tipo: Option<TipoFixador>,
    #[serde(default)]
    gerador: Option<String>,
    nivel: String,
    licenca: String,
    fontes: Vec<Fonte>,
    #[serde(default)]
    classe: Option<String>,
    #[serde(default)]
    acabamento: Option<String>,
    campos: Vec<String>,
    #[serde(default)]
    unidades: BTreeMap<String, String>,
    tamanhos: Vec<Tamanho>,
    #[serde(default)]
    comprimentos: Vec<f64>,
    #[serde(default)]
    ilustrativo: bool,
}

fn um() -> u32 {
    1
}

impl TryFrom<FamiliaBruta> for Familia {
    type Error = String;

    fn try_from(b: FamiliaBruta) -> Result<Self, String> {
        let por_nome = match &b.gerador {
            Some(g) => Some(GERADORES.iter().find(|(n, _)| n == g).map(|(_, t)| *t).ok_or_else(|| format!("{}: gerador '{g}' desconhecido", b.id))?),
            None => None,
        };
        let tipo = match (por_nome, b.tipo) {
            (Some(a), Some(c)) if a != c => return Err(format!("{}: 'tipo' e 'gerador' discordam", b.id)),
            (Some(t), _) | (None, Some(t)) => t,
            (None, None) => return Err(format!("{}: informe o gerador da família", b.id)),
        };
        Ok(Familia {
            id: b.id,
            nome: b.nome,
            norma: b.norma,
            revisao: b.revisao,
            versao: b.versao,
            tipo,
            gerador: b.gerador.unwrap_or_else(|| tipo.gerador().to_string()),
            nivel: b.nivel,
            licenca: b.licenca,
            fontes: b.fontes,
            classe: b.classe,
            acabamento: b.acabamento,
            campos: b.campos,
            unidades: b.unidades,
            tamanhos: b.tamanhos,
            comprimentos: b.comprimentos,
            camada: Camada::Publica,
            ilustrativo: b.ilustrativo,
        })
    }
}

/// Material da peça pela classe de resistência (texto da propriedade "material" do documento).
pub fn material_da_classe(classe: &str) -> String {
    let c = classe.trim();
    let up = c.to_ascii_uppercase();
    if up.starts_with("A2") || up.starts_with("A4") {
        format!("Aço inoxidável {c}")
    } else if c.chars().next().is_some_and(|x| x.is_ascii_digit()) {
        format!("Aço classe {c}")
    } else {
        format!("Aço {c}")
    }
}

/// Densidade usada para conferir a massa dos itens (aço), em g/mm³.
pub const DENSIDADE_ACO_G_MM3: f64 = 7.85e-3;

impl Familia {
    /// Busca por designação livre, sem diferenciar maiúsculas, espaços nem vírgula decimal;
    /// também acha pelo valor original ("1/4 pol").
    pub fn tamanho(&self, designacao: &str) -> Option<&Tamanho> {
        let k = Tamanho::chave_de(designacao);
        self.tamanhos.iter().find(|t| t.chave() == k || t.original.as_deref().is_some_and(|o| Tamanho::chave_de(o) == k))
    }

    /// Unidade do campo (mm quando não declarada).
    pub fn unidade(&self, campo: &str) -> &str {
        self.unidades.get(campo).map(String::as_str).unwrap_or("mm")
    }

    /// Série de comprimentos L do tamanho: a dele, ou a da família. Não há mais regra 2d–10d: a
    /// série é a que a norma ou o fabricante publica para o tamanho.
    pub fn comprimentos_de(&self, t: &Tamanho) -> Vec<f64> {
        if t.comprimentos.is_empty() { self.comprimentos.clone() } else { t.comprimentos.clone() }
    }

    /// Menor e maior comprimento da série do tamanho.
    pub fn limites_de_comprimento(&self, t: &Tamanho) -> Option<(f64, f64)> {
        let s = self.comprimentos_de(t);
        Some((*s.first()?, *s.last()?))
    }

    /// Faixa de diâmetros de furo que o prendedor aceita: a declarada no tamanho ou, nos métricos,
    /// da folga fina à folgada da tabela de furos.
    pub fn faixa_furo(&self, t: &Tamanho) -> Option<(f64, f64)> {
        match (t.furo_min, t.furo_max) {
            (Some(a), Some(b)) => Some((a, b)),
            _ => hardware::metric(&t.designacao).map(|m| (m.clearance[0], m.clearance[2])),
        }
    }

    /// Tamanhos cuja faixa de furo contém `d` (com folga `tol`), do menor para o maior.
    pub fn tamanhos_para_furo(&self, d: f64, tol: f64) -> Vec<&Tamanho> {
        let mut v: Vec<&Tamanho> = self.tamanhos.iter().filter(|t| self.faixa_furo(t).is_some_and(|(a, b)| d >= a - tol && d <= b + tol)).collect();
        v.sort_by(|a, b| a.diametro_nominal().partial_cmp(&b.diametro_nominal()).unwrap_or(std::cmp::Ordering::Equal));
        v
    }

    pub fn validar(&self) -> Result<(), String> {
        let er = |m: &str| Err(format!("{}: {m}", self.id));
        if self.id.trim().is_empty() || self.nome.trim().is_empty() || self.norma.trim().is_empty() || self.licenca.trim().is_empty() {
            return er("id, nome, norma e licença são obrigatórios");
        }
        if self.revisao.trim().is_empty() {
            return er("informe a revisão da norma");
        }
        if self.versao == 0 {
            return er("versao deve ser >= 1");
        }
        if !["A", "B", "C", "X"].contains(&self.nivel.as_str()) {
            return er("nível deve ser A, B, C ou X");
        }
        if self.fontes.is_empty() || self.fontes.iter().any(|f| f.rotulo.trim().is_empty() || !f.url.starts_with("http")) {
            return er("cada família cita ao menos uma fonte, com rótulo e URL");
        }
        if self.nivel == "C" && self.fontes.len() < 2 {
            return er("nível C exige ao menos duas fontes");
        }
        // medida ilustrativa é nossa (nível A) e nunca se passa por conferida
        if self.ilustrativo && (self.nivel != "A" || self.tamanhos.iter().any(|t| t.confirmado_por != 0)) {
            return er("família ilustrativa: nível A e confirmado_por 0 em todo tamanho");
        }
        if self.campos.is_empty() {
            return er("sem campos");
        }
        for c in self.tipo.campos_obrigatorios().iter().chain(campos_do_gerador(&self.gerador)) {
            if !self.campos.iter().any(|x| x == c) {
                return er(&format!("falta o campo '{c}' exigido pelo tipo {} (gerador {})", self.tipo.rotulo(), self.gerador));
            }
        }
        for (c, u) in &self.unidades {
            if !self.campos.contains(c) {
                return er(&format!("unidade de '{c}', que não é um campo"));
            }
            if !UNIDADES.contains(&u.as_str()) {
                return er(&format!("unidade '{u}' de '{c}' desconhecida"));
            }
        }
        if self.tamanhos.is_empty() {
            return er("sem tamanhos");
        }
        let crescente = |v: &[f64]| v.iter().all(|l| l.is_finite() && *l > 0.) && v.windows(2).all(|w| w[0] < w[1]);
        let faixa_ok = |a: Option<f64>, b: Option<f64>| match (a, b) {
            (None, None) => true,
            (Some(a), Some(b)) => a.is_finite() && b.is_finite() && a > 0. && a <= b,
            _ => false,
        };
        let mut vistos = HashSet::new();
        for t in &self.tamanhos {
            if t.designacao.trim().is_empty() {
                return er("designação vazia");
            }
            if t.diametro_nominal().is_none() {
                return er(&format!("designação '{}': informe d_nominal em mm", t.designacao));
            }
            if !vistos.insert(t.chave()) {
                return er(&format!("tamanho {} repetido", t.designacao));
            }
            if t.confirmado_por == 0 && !self.ilustrativo {
                return er(&format!("{}: confirmado_por deve ser >= 1", t.designacao));
            }
            if t.valores.len() != self.campos.len() {
                return er(&format!("{}: os valores não batem com os campos", t.designacao));
            }
            for c in &self.campos {
                match t.valor(c) {
                    Some(v) if v.is_finite() && v > 0. => {}
                    Some(_) => return er(&format!("{}: '{c}' deve ser positivo", t.designacao)),
                    None => return er(&format!("{}: falta o valor de '{c}'", t.designacao)),
                }
            }
            if !faixa_ok(t.grip_min, t.grip_max) {
                return er(&format!("{}: grip_min e grip_max vêm juntos, positivos e em ordem", t.designacao));
            }
            if !faixa_ok(t.furo_min, t.furo_max) {
                return er(&format!("{}: furo_min e furo_max vêm juntos, positivos e em ordem", t.designacao));
            }
            if t.massa_g.is_some_and(|m| !(m.is_finite() && m > 0.)) {
                return er(&format!("{}: massa_g deve ser positiva", t.designacao));
            }
            if !t.comprimentos.is_empty() && !crescente(&t.comprimentos) {
                return er(&format!("{}: comprimentos devem ser positivos e crescentes", t.designacao));
            }
        }
        if !self.comprimentos.is_empty() && !crescente(&self.comprimentos) {
            return er("comprimentos devem ser positivos e crescentes");
        }
        if self.tipo.usa_comprimento() {
            for t in &self.tamanhos {
                if self.comprimentos_de(t).is_empty() {
                    return er(&format!("{}: {} sem série de comprimentos", t.designacao, self.tipo.rotulo()));
                }
            }
        }
        Ok(())
    }
}

/// Conjunto de famílias de uma camada (ou o resultado de mesclar camadas).
#[derive(Clone, Debug, PartialEq)]
pub struct Biblioteca {
    pub camada: Camada,
    pub familias: Vec<Familia>,
}

impl Biblioteca {
    /// Lê as famílias de textos JSON (um por família). Valida cada uma e a unicidade dos `id`.
    pub fn de_jsons(camada: Camada, textos: &[(&str, &str)]) -> Result<Self, String> {
        let mut familias: Vec<Familia> = Vec::with_capacity(textos.len());
        for (arquivo, texto) in textos {
            let mut f: Familia = serde_json::from_str(texto).map_err(|e| format!("{arquivo}: {e}"))?;
            f.validar().map_err(|e| format!("{arquivo}: {e}"))?;
            if familias.iter().any(|o| o.id == f.id) {
                return Err(format!("{arquivo}: id '{}' repetido", f.id));
            }
            f.camada = camada;
            familias.push(f);
        }
        Ok(Biblioteca { camada, familias })
    }

    pub fn familia(&self, id: &str) -> Option<&Familia> {
        self.familias.iter().find(|f| f.id == id)
    }

    pub fn validar(&self) -> Result<(), String> {
        let mut ids = HashSet::new();
        for f in &self.familias {
            f.validar()?;
            if !ids.insert(f.id.as_str()) {
                return Err(format!("id '{}' repetido", f.id));
            }
        }
        Ok(())
    }

    /// Mescla camadas da mais baixa para a mais alta: uma família de cima com o mesmo `id`
    /// substitui a de baixo inteira (não há mistura de tamanhos entre camadas).
    pub fn mesclar(camadas: &[Biblioteca]) -> Biblioteca {
        let mut ordem = camadas.to_vec();
        ordem.sort_by_key(|b| b.camada);
        let mut familias: Vec<Familia> = vec![];
        for b in &ordem {
            for f in &b.familias {
                match familias.iter_mut().find(|o| o.id == f.id) {
                    Some(o) => *o = f.clone(),
                    None => familias.push(f.clone()),
                }
            }
        }
        Biblioteca { camada: ordem.last().map(|b| b.camada).unwrap_or_default(), familias }
    }
}

/// Arquivos da camada pública, embutidos em tempo de compilação.
const PUBLICA: &[(&str, &str)] = &[
    ("iso4762.json", include_str!("../../../biblioteca/fixadores/iso4762.json")),
    ("iso4017.json", include_str!("../../../biblioteca/fixadores/iso4017.json")),
    ("din933.json", include_str!("../../../biblioteca/fixadores/din933.json")),
    ("iso4032.json", include_str!("../../../biblioteca/fixadores/iso4032.json")),
    ("din934.json", include_str!("../../../biblioteca/fixadores/din934.json")),
    ("iso7089.json", include_str!("../../../biblioteca/fixadores/iso7089.json")),
    ("iso4014.json", include_str!("../../../biblioteca/fixadores/iso4014.json")),
    ("iso4035.json", include_str!("../../../biblioteca/fixadores/iso4035.json")),
    ("iso7380_1.json", include_str!("../../../biblioteca/fixadores/iso7380_1.json")),
    ("din985.json", include_str!("../../../biblioteca/fixadores/din985.json")),
    ("din127b.json", include_str!("../../../biblioteca/fixadores/din127b.json")),
    ("iso10642.json", include_str!("../../../biblioteca/fixadores/iso10642.json")),
    ("iso7045.json", include_str!("../../../biblioteca/fixadores/iso7045.json")),
    ("din6923.json", include_str!("../../../biblioteca/fixadores/din6923.json")),
    ("din6798a.json", include_str!("../../../biblioteca/fixadores/din6798a.json")),
    ("din25201.json", include_str!("../../../biblioteca/fixadores/din25201.json")),
    ("din938.json", include_str!("../../../biblioteca/fixadores/din938.json")),
    ("din939.json", include_str!("../../../biblioteca/fixadores/din939.json")),
    ("iso7379.json", include_str!("../../../biblioteca/fixadores/iso7379.json")),
    ("asme_b18_3.json", include_str!("../../../biblioteca/fixadores/asme_b18_3.json")),
    ("an3.json", include_str!("../../../biblioteca/fixadores/an3.json")),
    ("an310.json", include_str!("../../../biblioteca/fixadores/an310.json")),
    ("an315.json", include_str!("../../../biblioteca/fixadores/an315.json")),
    ("ms21042.json", include_str!("../../../biblioteca/fixadores/ms21042.json")),
    ("an470.json", include_str!("../../../biblioteca/fixadores/an470.json")),
    ("an426.json", include_str!("../../../biblioteca/fixadores/an426.json")),
    ("iso15_60.json", include_str!("../../../biblioteca/rolamentos/iso15_60.json")),
    ("iso15_62.json", include_str!("../../../biblioteca/rolamentos/iso15_62.json")),
    ("iso15_63.json", include_str!("../../../biblioteca/rolamentos/iso15_63.json")),
    ("iso15_68.json", include_str!("../../../biblioteca/rolamentos/iso15_68.json")),
    ("iso15_69.json", include_str!("../../../biblioteca/rolamentos/iso15_69.json")),
    ("iso15_160.json", include_str!("../../../biblioteca/rolamentos/iso15_160.json")),
    ("iso15_72.json", include_str!("../../../biblioteca/rolamentos/iso15_72.json")),
    ("iso15_nu2.json", include_str!("../../../biblioteca/rolamentos/iso15_nu2.json")),
    ("iso355_302.json", include_str!("../../../biblioteca/rolamentos/iso355_302.json")),
    ("iso15552.json", include_str!("../../../biblioteca/pneumatica/iso15552.json")),
    ("iso6432.json", include_str!("../../../biblioteca/pneumatica/iso6432.json")),
    ("iso6432_mr3.json", include_str!("../../../biblioteca/pneumatica/iso6432_mr3.json")),
    ("hiwin_hgr.json", include_str!("../../../biblioteca/guias/hiwin_hgr.json")),
    ("hiwin_hgh.json", include_str!("../../../biblioteca/guias/hiwin_hgh.json")),
    ("din69051_porca.json", include_str!("../../../biblioteca/guias/din69051_porca.json")),
    ("din69051_fuso.json", include_str!("../../../biblioteca/guias/din69051_fuso.json")),
    ("terminal_rotular_ilustrativo.json", include_str!("../../../biblioteca/articulacoes/terminal_rotular_ilustrativo.json")),
];

/// A biblioteca pública embutida. Os arquivos são validados nos testes; aqui um arquivo inválido
/// é erro de programação e interrompe com a mensagem do arquivo.
pub fn publica() -> &'static Biblioteca {
    static B: OnceLock<Biblioteca> = OnceLock::new();
    B.get_or_init(|| Biblioteca::de_jsons(Camada::Publica, PUBLICA).unwrap_or_else(|e| panic!("biblioteca pública inválida: {e}")))
}

/// Todas as famílias de fixadores (hoje só a camada pública).
pub fn familias() -> &'static [Familia] {
    &publica().familias
}

pub fn familia(id: &str) -> Option<&'static Familia> {
    publica().familia(id)
}

/// Sobra do rebite sólido além da pilha, em diâmetros: é o material que forma a cabeça de fechamento.
pub const SOBRA_REBITE: f64 = 1.5;

/// Comprimento L de um rebite para a pega `pega` (espessura da pilha, mm): o menor da série do tamanho
/// com `L >= pega + 1,5·d`. `None` quando a pega pede mais que o maior da série.
pub fn rebite_pela_pega(fam: &Familia, t: &Tamanho, pega: f64) -> Option<f64> {
    let d = t.diametro_nominal()?;
    let alvo = pega + SOBRA_REBITE * d;
    fam.comprimentos_de(t).into_iter().find(|l| *l >= alvo - 1e-6)
}

/// Tamanho de uma família pela designação ("M6").
pub fn tamanho(familia_id: &str, designacao: &str) -> Option<&'static Tamanho> {
    familia(familia_id)?.tamanho(designacao)
}

/// Faixa de diâmetros de eixo (acima de `d_acima`, até `d_ate`) e a chaveta que serve a ela.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
pub struct Chaveta {
    pub d_acima: f64,
    pub d_ate: f64,
    /// largura
    pub b: f64,
    /// altura
    pub h: f64,
    /// profundidade do rasgo no eixo
    pub t1: f64,
    /// profundidade do rasgo no cubo
    pub t2: f64,
}

/// Tabela de chavetas paralelas (DIN 6885-1) com a série de comprimentos.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct TabelaChavetas {
    pub id: String,
    pub nome: String,
    pub norma: String,
    pub nivel: String,
    pub licenca: String,
    pub fontes: Vec<String>,
    /// quem conferiu os valores contra uma segunda fonte (vazio = ainda ninguém)
    #[serde(default)]
    pub confirmado_por: Vec<String>,
    pub faixas: Vec<Chaveta>,
    pub comprimentos: Vec<f64>,
}

impl TabelaChavetas {
    pub fn validar(&self) -> Result<(), String> {
        let er = |m: &str| Err(format!("{}: {m}", self.id));
        if self.id.trim().is_empty() || self.nome.trim().is_empty() || self.norma.trim().is_empty() || self.licenca.trim().is_empty() {
            return er("id, nome, norma e licença são obrigatórios");
        }
        if !["A", "B", "C", "X"].contains(&self.nivel.as_str()) {
            return er("nível deve ser A, B, C ou X");
        }
        if self.fontes.is_empty() || self.fontes.iter().any(|f| f.trim().is_empty()) {
            return er("cite ao menos uma fonte");
        }
        if self.faixas.is_empty() {
            return er("sem faixas");
        }
        for (i, f) in self.faixas.iter().enumerate() {
            let medidas = [f.d_acima, f.d_ate, f.b, f.h, f.t1, f.t2];
            if medidas.iter().any(|v| !v.is_finite() || *v <= 0.) {
                return er(&format!("faixa {}–{}: todas as medidas devem ser positivas", f.d_acima, f.d_ate));
            }
            if f.d_ate <= f.d_acima || f.t1 >= f.h || f.t2 >= f.h {
                return er(&format!("faixa {}–{}: faixa ou profundidades incoerentes", f.d_acima, f.d_ate));
            }
            if i > 0 && self.faixas[i - 1].d_ate != f.d_acima {
                return er(&format!("faixa {}–{}: não continua a anterior", f.d_acima, f.d_ate));
            }
        }
        if self.comprimentos.is_empty() || self.comprimentos.iter().any(|l| !(*l > 0.)) || self.comprimentos.windows(2).any(|w| w[0] >= w[1]) {
            return er("comprimentos devem ser positivos e crescentes");
        }
        Ok(())
    }
}

const CHAVETAS_DIN6885A: &str = include_str!("../../../biblioteca/chavetas/din6885a.json");

/// Tabela de chavetas forma A embutida (validada nos testes; inválida = erro de programação).
pub fn tabela_chavetas() -> &'static TabelaChavetas {
    static T: OnceLock<TabelaChavetas> = OnceLock::new();
    T.get_or_init(|| {
        let t: TabelaChavetas = serde_json::from_str(CHAVETAS_DIN6885A).unwrap_or_else(|e| panic!("din6885a.json: {e}"));
        t.validar().unwrap_or_else(|e| panic!("tabela de chavetas inválida: {e}"));
        t
    })
}

/// Chaveta da faixa do diâmetro de eixo `d` (acima de `d_acima`, até `d_ate`); `None` fora da tabela.
pub fn chaveta_para_eixo(d: f64) -> Option<Chaveta> {
    tabela_chavetas().faixas.iter().find(|f| d > f.d_acima && d <= f.d_ate).copied()
}

/// Série de comprimentos padrão de chaveta.
pub fn comprimentos_chaveta() -> &'static [f64] {
    &tabela_chavetas().comprimentos
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tabela_de_chavetas() {
        let t = tabela_chavetas();
        assert_eq!(t.faixas.len(), 13);
        assert_eq!(t.nivel, "C");
        assert!(!t.confirmado_por.is_empty() && t.licenca.starts_with("CC BY 4.0"));
        // B52: t2 (rasgo do cubo) agora com segunda fonte gratuita; o que segue com uma só diz PENDENTE
        assert!(t.confirmado_por.iter().any(|c| c.starts_with("t1 e t2 de 2×2 a 14×9") && c.contains("fasten.it") && c.contains("Winco")));
        assert!(!t.confirmado_por.iter().any(|c| c.starts_with("t2: só na fonte 1")));
        // B52: de 16×10 a 22×14 a segunda fonte independente (TU Dortmund para t1, Sass KG para t2)
        // fecha a tabela inteira: nenhuma linha fica PENDENTE de segunda fonte
        assert!(t.confirmado_por.iter().any(|c| c.starts_with("t1 e t2 de 16×10 a 22×14") && c.contains("Dortmund") && c.contains("Sass")));
        assert!(!t.confirmado_por.iter().any(|c| c.contains("PENDENTE")));
        for (d, b, h, t1, t2) in [(55., 16., 10., 6., 4.3), (60., 18., 11., 7., 4.4), (70., 20., 12., 7.5, 4.9), (80., 22., 14., 9., 5.4)] {
            let c = chaveta_para_eixo(d).unwrap();
            assert_eq!((c.b, c.h, c.t1, c.t2), (b, h, t1, t2), "Ø{d}");
        }
        let c = chaveta_para_eixo(20.).unwrap();
        assert_eq!((c.b, c.h, c.t1, c.t2), (6., 6., 3.5, 2.8));
        let c = chaveta_para_eixo(40.).unwrap();
        assert_eq!((c.b, c.h, c.t1), (12., 8., 5.));
        // limites: "acima de" exclui, "até" inclui
        assert_eq!(chaveta_para_eixo(8.).unwrap().b, 2.);
        assert_eq!(chaveta_para_eixo(8.01).unwrap().b, 3.);
        assert!(chaveta_para_eixo(6.).is_none() && chaveta_para_eixo(85.1).is_none());
        assert_eq!(chaveta_para_eixo(85.).unwrap().h, 14.);
        assert_eq!(comprimentos_chaveta().len(), 28);
    }

    #[test]
    fn todas_as_familias_carregam_e_validam() {
        let b = publica();
        // 42 e o terminal rotular ilustrativo (B397)
        assert_eq!(b.familias.len(), 43);
        b.validar().unwrap();
        for id in ["iso4762", "iso4017", "din933", "iso4032", "din934", "iso7089"] {
            let f = familia(id).unwrap_or_else(|| panic!("{id} ausente"));
            assert_eq!(f.tamanhos.len(), 9, "{id}");
            assert_eq!(f.nivel, "C");
            assert!(f.licenca.starts_with("CC BY 4.0"));
        }
        // 4762, 4017, 933, 4014, 7380-1, 10642, 7045, 938, 939, 7379, B18.3 e AN
        assert_eq!(familias().iter().filter(|f| f.tipo.e_parafuso()).count(), 12);
    }

    #[test]
    fn porca_autotravante_din985_traz_altura_total_e_chave() {
        let f = familia("din985").unwrap();
        assert_eq!(f.tipo, TipoFixador::Porca);
        assert!(f.tamanhos.iter().all(|t| t.confirmado_por >= 2), "só dados em duas fontes");
        let t = tamanho("din985", "M8").unwrap();
        assert_eq!((t.valor("s"), t.valor("m")), (Some(13.), Some(8.)));
        // mais alta que a porca baixa e que a comum não: altura total do anel de poliamida
        assert!(tamanho("din985", "M8").unwrap().valor("m") > tamanho("iso4035", "M8").unwrap().valor("m"));
    }

    #[test]
    fn familias_do_b51_trazem_conferencia_e_dados_publicados() {
        // ISO 4014: mesma cabeça da ISO 4017 (s = 16 e 18 em M10 e M12), mas com o comprimento roscado b
        let t = tamanho("iso4014", "M10").unwrap();
        assert_eq!((t.valor("s"), t.valor("k"), t.valor("b")), (Some(16.), Some(6.4), Some(26.)));
        // rosca parcial: todo comprimento da série é maior que b
        for t in &familia("iso4014").unwrap().tamanhos {
            let b = t.valor("b").unwrap();
            assert!(familia("iso4014").unwrap().comprimentos_de(t).iter().all(|l| *l > b), "{}", t.designacao);
        }
        // ISO 4035: porca baixa (m = 6 em M12, contra 10,8 da porca normal)
        let t = tamanho("iso4035", "M12").unwrap();
        assert_eq!((t.valor("s"), t.valor("m")), (Some(18.), Some(6.)));
        // ISO 7380-1: sem "b"; calota dk x k
        let t = tamanho("iso7380_1", "M8").unwrap();
        assert_eq!((t.valor("dk"), t.valor("k"), t.valor("s"), t.valor("b")), (Some(14.), Some(4.4), Some(5.), None));
        // toda linha nova: duas fontes nas cotas e a nota diz o que foi (ou não foi) conferido
        for id in ["iso4014", "iso4035", "iso7380_1"] {
            let f = familia(id).unwrap();
            assert!(f.fontes.len() >= 2, "{id}");
            for t in &f.tamanhos {
                assert!(t.confirmado_por >= 2 && t.nota.as_deref().is_some_and(|n| n.contains("fontes") || n.contains("fonte")), "{id} {}", t.designacao);
            }
        }
    }

    #[test]
    fn familias_novas_do_b51_tem_gerador_proprio_e_duas_fontes() {
        let ids = ["iso10642", "iso7045", "din6923", "din6798a", "din25201", "din938", "din939", "iso7379", "asme_b18_3"];
        for id in ids {
            let f = familia(id).unwrap_or_else(|| panic!("{id} ausente"));
            assert!(f.fontes.len() >= 2, "{id}");
            assert!(f.tamanhos.iter().all(|t| t.confirmado_por >= 2 && t.nota.is_some()), "{id}");
        }
        let g = |id: &str| familia(id).unwrap().gerador.clone();
        assert_eq!((g("iso10642"), g("iso7045"), g("din6923"), g("din938"), g("iso7379")), ("parafuso_escareado".into(), "parafuso_cruz".into(), "porca_flangeada".into(), "prisioneiro".into(), "parafuso_ombro".into()));
        // a porca flangeada continua porca para a montagem; o escareado e o ombro continuam Allen
        assert_eq!(familia("din6923").unwrap().tipo, TipoFixador::Porca);
        assert_eq!(familia("iso7379").unwrap().tipo, TipoFixador::ParafusoAllen);
        // ISO 10642 M8: dk 17,92 e k 4,96 (cone de 90°: (dk - d) / 2 = k)
        let t = tamanho("iso10642", "M8").unwrap();
        assert_eq!((t.valor("dk"), t.valor("k"), t.valor("s")), (Some(17.92), Some(4.96), Some(5.)));
        assert!(((17.92 - 8.) / 2. - 4.96_f64).abs() < 1e-9);
        // DIN 6923 M10: chave 15 (não 17) e aba de 21,8
        let t = tamanho("din6923", "M10").unwrap();
        assert_eq!((t.valor("s"), t.valor("dc")), (Some(15.), Some(21.8)));
        // prisioneiros: bm = d na DIN 938 e 1,25 d na 939
        for t in &familia("din938").unwrap().tamanhos {
            assert_eq!(t.valor("bm"), t.diametro_nominal(), "{}", t.designacao);
        }
        assert_eq!(tamanho("din939", "M16").unwrap().valor("bm"), Some(20.));
        // ombro: o tamanho é "M8 Ø10", com d nominal do ombro e a rosca menor
        let t = tamanho("iso7379", "M8 Ø10").unwrap();
        assert_eq!((t.diametro_nominal(), t.valor("dr")), (Some(10.), Some(8.)));
        // polegada: 3/8-16 UNC guardado em mm, achado também por "3/8 pol"
        let t = tamanho("asme_b18_3", "3/8 pol").unwrap();
        assert_eq!(t.designacao, "3/8-16 UNC");
        assert_eq!((t.diametro_nominal(), t.valor("dk"), t.valor("k")), (Some(9.525), Some(14.275), Some(9.525)));
        assert!((t.valor("P").unwrap() - 25.4 / 16.).abs() < 1e-3);
        // sem campo do gerador a família não valida
        let sem_aba = include_str!("../../../biblioteca/fixadores/din6923.json").replace("\"c\"", "\"x\"");
        assert!(Biblioteca::de_jsons(Camada::Publica, &[("t.json", &sem_aba)]).is_err());
    }

    #[test]
    fn rolamentos_das_series_60_e_62_pelas_medidas_de_contorno() {
        // B44: 6000 a 6020 e 6200 a 6220, d D B conferidos em duas fontes
        let (s60, s62) = (familia("iso15_60").unwrap(), familia("iso15_62").unwrap());
        assert_eq!((s60.tamanhos.len(), s62.tamanhos.len()), (21, 21));
        for f in [s60, s62] {
            assert!(f.tamanhos.iter().all(|t| t.confirmado_por >= 2 && t.diametro_nominal().is_some()), "{}", f.id);
            // dentro da série, furo e externo crescem juntos e o anel tem espessura
            for w in f.tamanhos.windows(2) {
                assert!(w[1].diametro_nominal() > w[0].diametro_nominal() && w[1].valor("D") > w[0].valor("D"), "{} {}", f.id, w[1].designacao);
            }
            for t in &f.tamanhos {
                assert!(t.valor("D").unwrap() > t.diametro_nominal().unwrap() + 10., "{}", t.designacao);
            }
        }
        // a série 62 é mais robusta que a 60 no mesmo furo: 6205 (25 x 52 x 15) contra 6005 (25 x 47 x 12)
        let (a, b) = (tamanho("iso15_62", "6205").unwrap(), tamanho("iso15_60", "6005").unwrap());
        assert_eq!((a.diametro_nominal(), a.valor("D"), a.valor("B")), (Some(25.), Some(52.), Some(15.)));
        assert!(a.valor("D") > b.valor("D") && a.valor("B") > b.valor("B"));
        // o furo do alojamento acha o tamanho certo em cada série (o 80 é 6010 na 60 e 6208 na 62)
        assert_eq!(s62.tamanhos_para_furo(80., 0.05).iter().map(|t| t.designacao.as_str()).collect::<Vec<_>>(), ["6208"]);
        // capacidade de carga só na nota, citando o fabricante
        assert!(tamanho("iso15_60", "6000").unwrap().nota.as_deref().is_some_and(|n| n.contains("NSK") && n.contains("C = 5050 N")));
    }

    #[test]
    fn parafusos_aeronauticos_an3_a_an6_pela_pega() {
        // B43: AN3 a AN6, pega e comprimento conferidos em duas tabelas; b = L − pega é constante por tamanho
        let f = familia("an3").unwrap();
        assert_eq!(f.tamanhos.iter().map(|t| t.designacao.as_str()).collect::<Vec<_>>(), ["AN3", "AN4", "AN5", "AN6", "AN7", "AN8"]);
        let pol = 25.4;
        // AN7 e AN8 (B43): AN7-12A com L 1 11/32" dentro de 1,328 a 1,375 e b = 21/32" (Military
        // Fasteners); chave 5/8" e 3/4" (Hi-Line) dentro da faixa entre faces
        let an7 = tamanho("an3", "7/16-20 UNF").unwrap();
        assert!((an7.valor("b").unwrap() - 21. / 32. * pol).abs() < 1e-3 && (an7.valor("P").unwrap() - pol / 20.).abs() < 1e-3);
        assert!(an7.comprimentos.iter().any(|l| (1.328..=1.375).contains(&(l / pol))));
        assert!((0.615..=0.627).contains(&(an7.valor("s").unwrap() / pol)) && (0.615..=0.627).contains(&(5. / 8.)));
        let an8 = tamanho("an3", "AN8").unwrap();
        assert!((an8.valor("b").unwrap() - 25. / 32. * pol).abs() < 1e-3);
        // AN8-12: pega de 9/16" (L 1 11/32" − 25/32")
        assert!(an8.comprimentos.iter().any(|l| (l / pol - 25. / 32. - 9. / 16.).abs() < 1e-3));
        assert!((0.740..=0.752).contains(&(an8.valor("s").unwrap() / pol)));
        // a altura da cabeça de AN7 e AN8 só tem uma fonte gratuita: fica marcada e PENDENTE
        for t in [an7, an8] {
            assert!(t.confirmado_por == 1 && t.nota.as_deref().is_some_and(|n| n.contains("PENDENTE")), "{}", t.designacao);
        }
        let an3 = tamanho("an3", "AN3").unwrap();
        assert!((an3.valor("P").unwrap() - pol / 32.).abs() < 1e-3 && (an3.valor("b").unwrap() - 13. / 32. * pol).abs() < 1e-3);
        assert!((an3.diametro_nominal().unwrap() - 0.19 * pol).abs() < 1e-3);
        // AN3-10: L = 1 1/32" com pega de 5/8" (a ficha do fabricante diz 1,031 e 0,625)
        assert!(an3.comprimentos.iter().any(|l| (l - 1.03125 * pol).abs() < 1e-2));
        // achado pela rosca também
        assert_eq!(tamanho("an3", "1/4-28 UNF").unwrap().designacao, "AN4");
        // AN6 (3/8-24): AN6-12 com L = 1 21/64" e pega 11/16" (b = 41/64"), cabeça de 0,203 a 0,235"
        let an6 = tamanho("an3", "3/8-24 UNF").unwrap();
        assert_eq!(an6.designacao, "AN6");
        assert!((an6.valor("b").unwrap() - 41. / 64. * pol).abs() < 1e-3 && (an6.valor("P").unwrap() - pol / 24.).abs() < 1e-3);
        assert!(an6.comprimentos.iter().any(|l| (l - (1. + 21. / 64.) * pol).abs() < 1e-2));
        let k = an6.valor("k").unwrap() / pol;
        assert!((0.203..=0.235).contains(&k), "{k}");
        for t in &f.tamanhos {
            let (s, k, d) = (t.valor("s").unwrap(), t.valor("k").unwrap(), t.diametro_nominal().unwrap());
            assert!(s > 1.4 * d && k > 0.3 * d && k < 0.8 * d, "{}", t.designacao);
            assert!(t.comprimentos.iter().all(|l| *l > t.valor("b").unwrap()), "{}: pega positiva em todo traço", t.designacao);
        }
    }

    #[test]
    fn porcas_aeronauticas_an310_an315_ms21042() {
        // B43: chave e altura máxima conferidas em duas fontes (o que tem uma só diz PENDENTE)
        let pol = 25.4;
        for id in ["an310", "an315", "ms21042"] {
            let f = familia(id).unwrap();
            assert_eq!((f.tipo, f.gerador.as_str(), f.nivel.as_str()), (TipoFixador::Porca, "porca", "C"));
            assert!(f.fontes.len() >= 2, "{id}");
            for t in &f.tamanhos {
                let pendente = t.nota.as_deref().is_some_and(|n| n.contains("PENDENTE"));
                assert!(t.confirmado_por >= 2 || pendente, "{id} {}", t.designacao);
                assert!(t.valor("s").unwrap() > t.diametro_nominal().unwrap(), "{id} {}", t.designacao);
            }
        }
        // AN310-6: chave 9/16" e altura 13/32"; AN310-7: 5/8" e 29/64" (Pegasus e Military Fasteners)
        let t = tamanho("an310", "AN310-6").unwrap();
        assert!((t.valor("s").unwrap() - 9. / 16. * pol).abs() < 1e-3 && (t.valor("m").unwrap() - 13. / 32. * pol).abs() < 1e-3);
        assert!((tamanho("an310", "AN310-7").unwrap().valor("m").unwrap() / pol - 29. / 64.).abs() < 1e-4);
        // a castelo é mais alta que a sextavada simples do mesmo tamanho; a MS21042 tem chave reduzida
        for n in 3..=6 {
            let (c, h, r) = (tamanho("an310", &format!("AN310-{n}")).unwrap(), tamanho("an315", &format!("AN315-{n}R")).unwrap(), tamanho("ms21042", &format!("MS21042L{n}")).unwrap());
            assert!(c.valor("m") > h.valor("m") && r.valor("s") < h.valor("s"), "-{n}");
            assert_eq!(c.diametro_nominal(), r.diametro_nominal());
        }
        // achada pela rosca
        assert_eq!(tamanho("ms21042", "1/4-28 UNF").unwrap().designacao, "MS21042L4");
        // só -6 e -7 da AN315 ficam com uma fonte
        let pend: Vec<_> = familia("an315").unwrap().tamanhos.iter().filter(|t| t.confirmado_por < 2).map(|t| t.designacao.as_str()).collect();
        assert_eq!(pend, ["AN315-6R", "AN315-7R"]);
    }

    /// B44: séries finas 68, 69 e 160, contato angular 72, rolos cilíndricos NU2 e cônicos 302,
    /// todas com duas fontes que concordam e a regra de furo da ISO 15 (código × 5 a partir de 04).
    #[test]
    fn series_finas_contato_angular_e_rolos() {
        let furo = |no: &str| -> f64 {
            let c: u32 = no[no.len() - 2..].parse().unwrap();
            match c { 0 => 10., 1 => 12., 2 => 15., 3 => 17., c => 5. * c as f64 }
        };
        for (id, n, nome) in [("iso15_68", 12, "série 68"), ("iso15_69", 12, "série 69"), ("iso15_160", 10, "série 160"), ("iso15_72", 19, "contato angular"), ("iso15_nu2", 9, "rolos cilíndricos"), ("iso355_302", 9, "rolos cônicos")] {
            let f = familia(id).unwrap_or_else(|| panic!("{id}"));
            assert_eq!((f.tamanhos.len(), f.gerador.as_str(), f.nivel.as_str()), (n, "rolamento", "C"), "{id}");
            assert!(f.nome.contains(nome) && f.fontes.len() >= 2, "{id}");
            for t in &f.tamanhos {
                assert!(t.confirmado_por >= 2 && t.nota.as_deref().unwrap().contains("concordam"), "{id} {}", t.designacao);
                let num = t.designacao.trim_start_matches("NU");
                assert_eq!(t.diametro_nominal(), Some(furo(num)), "{id} {}", t.designacao);
            }
            for w in f.tamanhos.windows(2) {
                assert!(w[1].diametro_nominal() > w[0].diametro_nominal() && w[1].valor("D") >= w[0].valor("D"), "{id}");
            }
        }
        let v = |f: &str, n: &str| { let t = tamanho(f, n).unwrap(); (t.diametro_nominal().unwrap(), t.valor("D").unwrap(), t.valor("B").unwrap()) };
        assert_eq!(v("iso15_68", "6805"), (25., 37., 7.));
        assert_eq!(v("iso15_69", "6905"), (25., 42., 9.));
        assert_eq!(v("iso15_160", "16005"), (25., 47., 8.));
        // no mesmo furo a seção cresce: 68 < 69 < 160 < 60 em D
        assert!(v("iso15_68", "6805").1 < v("iso15_69", "6905").1 && v("iso15_69", "6905").1 < v("iso15_160", "16005").1);
        assert_eq!(v("iso15_160", "16005").1, v("iso15_60", "6005").1);
        // contato angular 72 e rolos NU2 têm o contorno da série 62 de esferas
        for (a, b, c) in [("7205", "NU205", "6205"), ("7210", "NU210", "6210"), ("7212", "NU212", "6212")] {
            assert_eq!(v("iso15_72", a), v("iso15_62", c));
            assert_eq!(v("iso15_nu2", b), v("iso15_62", c));
        }
        // cônico: B guarda a largura total T, maior que a do 62 no mesmo furo
        assert_eq!(v("iso355_302", "30205"), (25., 52., 16.25));
        assert!(tamanho("iso355_302", "30205").unwrap().nota.as_deref().unwrap().contains("Koyo (30205JR): C = 31500 N, C0 = 33700 N"));
        assert!(tamanho("iso15_72", "7205").unwrap().nota.as_deref().unwrap().contains("7205B (40°, gaiola usinada): C = 14000 N"));
        assert!(tamanho("iso15_nu2", "NU205").unwrap().nota.as_deref().unwrap().contains("C = 17700 N, C0 = 15700 N"));
    }

    /// B44: toda família nova gera um corpo com o contorno da tabela.
    #[test]
    fn familias_novas_de_rolamento_geram_o_contorno() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        for (id, n) in [("iso15_68", "6805"), ("iso15_69", "6910"), ("iso15_160", "16008"), ("iso15_72", "7208"), ("iso15_nu2", "NU206"), ("iso355_302", "30207")] {
            let t = tamanho(id, n).unwrap();
            let (d, dext, b) = (t.diametro_nominal().unwrap(), t.valor("D").unwrap(), t.valor("B").unwrap());
            let doc = crate::fixadores::fixador_documento(id, n, None).unwrap();
            let rc = crate::recompute(&doc);
            assert!(doc.features.iter().all(|f| rc.status[&f.id] == crate::Status::Ok), "{id} {n}");
            assert_eq!(rc.bodies.len(), 1, "{id} {n}");
            let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
            for p in rc.bodies[0].mesh.positions.chunks_exact(3) {
                for i in 0..3 {
                    lo[i] = lo[i].min(p[i] as f64);
                    hi[i] = hi[i].max(p[i] as f64);
                }
            }
            assert!((hi[2] - lo[2] - b).abs() < 1e-3 && (hi[0] - lo[0] - dext).abs() < 0.2, "{id} {n}: caixa {lo:?} {hi:?}");
            let (d2, d1) = (dext - (dext - d) / 3., d + (dext - d) / 3.);
            let esperado = std::f64::consts::PI / 4. * ((dext * dext - d2 * d2) * b + (d1 * d1 - d * d) * b + (d2 * d2 - d1 * d1) * b / 2.);
            let vol = rc.bodies[0].mass.volume;
            assert!((vol - esperado).abs() / esperado < 0.01, "{id} {n}: volume {vol} contra {esperado}");
        }
    }

    #[test]
    fn serie_63_e_capacidade_de_carga_em_todos_os_tamanhos() {
        // B44: série 63 de 6300 a 6320 (Partmatch e TFL concordam em d, D e B)
        let s63 = familia("iso15_63").unwrap();
        assert_eq!(s63.tamanhos.len(), 21);
        assert!(s63.tamanhos.iter().all(|t| t.confirmado_por >= 2));
        for w in s63.tamanhos.windows(2) {
            assert!(w[1].diametro_nominal() > w[0].diametro_nominal() && w[1].valor("D") > w[0].valor("D") && w[1].valor("B") > w[0].valor("B"));
        }
        let t = tamanho("iso15_63", "6305").unwrap();
        assert_eq!((t.diametro_nominal(), t.valor("D"), t.valor("B")), (Some(25.), Some(62.), Some(17.)));
        // no mesmo furo: 60 < 62 < 63
        let d = |f: &str, n: &str| tamanho(f, n).unwrap().valor("D").unwrap();
        assert!(d("iso15_60", "6005") < d("iso15_62", "6205") && d("iso15_62", "6205") < d("iso15_63", "6305"));
        assert_eq!(s63.tamanhos_para_furo(62., 0.05).iter().map(|t| t.designacao.as_str()).collect::<Vec<_>>(), ["6305"]);
        // todo tamanho das três séries tem a capacidade citada com o nome do fabricante, e a
        // capacidade cresce da série leve para a pesada no mesmo furo
        let c = |f: &str, n: &str| -> f64 {
            let nota = tamanho(f, n).unwrap().nota.clone().unwrap();
            let i = nota.find("C = ").unwrap() + 4;
            nota[i..].split(' ').next().unwrap().parse().unwrap()
        };
        for f in ["iso15_60", "iso15_62", "iso15_63"] {
            for t in &familia(f).unwrap().tamanhos {
                let n = t.nota.as_deref().unwrap();
                assert!(n.contains("Capacidade de carga citada do fabricante") && n.contains("C0 = "), "{f} {}", t.designacao);
            }
        }
        assert!(c("iso15_60", "6010") < c("iso15_62", "6210") && c("iso15_62", "6210") < c("iso15_63", "6310"));
        assert!(tamanho("iso15_62", "6210").unwrap().nota.as_deref().unwrap().contains("SRV Bearing: C = 35000 N, C0 = 23200 N"));
        assert!(tamanho("iso15_63", "6310").unwrap().nota.as_deref().unwrap().contains("TFL Bearing: C = 62000 N, C0 = 38500 N"));
    }

    #[test]
    fn busca_por_tamanho_e_comprimentos() {
        let t = tamanho("iso4762", "M6").unwrap();
        assert_eq!((t.valor("dk"), t.valor("k"), t.valor("s")), (Some(10.), Some(6.), Some(5.)));
        assert_eq!(t.diametro_nominal(), Some(6.));
        assert!(tamanho("iso4762", "M7").is_none() && tamanho("nada", "M6").is_none());
        let f = familia("iso4762").unwrap();
        // M6: 12 <= L <= 60
        let l = f.comprimentos_de(t);
        assert_eq!((l.first(), l.last()), (Some(&12.), Some(&60.)));
        // M3: o mínimo de 6 mm vale (2·d = 6) e o máximo é 30
        let l = f.comprimentos_de(f.tamanho("M3").unwrap());
        assert_eq!((l.first(), l.last()), (Some(&6.), Some(&30.)));
    }

    #[test]
    fn dados_conferidos_e_notas_de_fonte_unica() {
        // DIN difere da ISO em M10 e M12
        assert_eq!(tamanho("iso4017", "M10").unwrap().valor("s"), Some(16.));
        assert_eq!(tamanho("din933", "M10").unwrap().valor("s"), Some(17.));
        assert_eq!(tamanho("din934", "M12").unwrap().valor("s"), Some(19.));
        // regra b = 2d + 12 nos Allen
        for t in &familia("iso4762").unwrap().tamanhos {
            assert_eq!(t.valor("b"), Some(2. * t.diametro_nominal().unwrap() + 12.), "{}", t.designacao);
        }
        // tamanhos de fonte única trazem nota
        for f in familias() {
            for t in f.tamanhos.iter().filter(|t| t.confirmado_por < 2) {
                assert!(t.nota.is_some(), "{} {} sem nota", f.id, t.designacao);
            }
        }
    }

    #[test]
    fn validacao_rejeita_dados_ruins() {
        let ok = include_str!("../../../biblioteca/fixadores/iso7089.json");
        let ruim = |de: &str, para: &str| {
            assert!(ok.contains(de), "{de}");
            Biblioteca::de_jsons(Camada::Publica, &[("t.json", &ok.replacen(de, para, 1))])
        };
        assert!(ruim("\"h\": 0.5", "\"h\": 0").is_err());
        assert!(ruim("\"h\": 0.5", "\"h\": -1").is_err());
        assert!(ruim("\"nivel\": \"C\"", "\"nivel\": \"Z\"").is_err());
        assert!(ruim("\"campos\": [\"d1\", \"d2\", \"h\"]", "\"campos\": [\"d1\", \"d2\"]").is_err());
        assert!(ruim("\"tipo\": \"arruela\"", "\"tipo\": \"prego\"").is_err());
        assert!(ruim("\"confirmado_por\": 2", "\"confirmado_por\": 0").is_err());
        assert!(ruim("\"designacao\": \"M4\"", "\"designacao\": \"M3\"").is_err());
        // id repetido entre arquivos
        assert!(Biblioteca::de_jsons(Camada::Publica, &[("a.json", ok), ("b.json", ok)]).is_err());
        assert!(Biblioteca::de_jsons(Camada::Publica, &[("a.json", ok)]).is_ok());
    }
    /// Família de teste (dados inventados, só para exercitar o formato): designação em polegada,
    /// série própria por tamanho, faixa de furo e de pega.
    const SINTETICA: &str = r#"{
      "id": "teste_pol", "nome": "Parafuso de teste", "norma": "TESTE 1", "revisao": "2020", "versao": 3,
      "gerador": "parafuso_sextavado", "nivel": "A", "licenca": "teste",
      "fontes": [{ "rotulo": "teste", "url": "https://exemplo.invalid/t", "pagina": "p. 1" }],
      "classe": "A2-70", "acabamento": "Passivado",
      "campos": ["P", "s", "k"], "unidades": { "P": "mm" },
      "tamanhos": [
        { "designacao": "1/4-28 UNF", "original": "1/4 pol", "d_nominal": 6.35,
          "valores": { "P": 0.907, "s": 11.1, "k": 4.3 }, "confirmado_por": 1,
          "comprimentos": [9.5, 12.7, 15.9], "grip_min": 1.6, "grip_max": 6.4, "furo_min": 6.4, "furo_max": 6.5, "pn": "T-04" },
        { "designacao": "-06", "d_nominal": 4.8, "valores": { "P": 0.8, "s": 8, "k": 3.5 }, "confirmado_por": 1,
          "comprimentos": [8, 10], "furo_min": 4.8, "furo_max": 4.95 }
      ]
    }"#;

    #[test]
    fn designacao_livre_serie_por_tamanho_e_faixa_de_furo() {
        let b = Biblioteca::de_jsons(Camada::Usuario, &[("t.json", SINTETICA)]).unwrap();
        let f = b.familia("teste_pol").unwrap();
        assert_eq!((f.camada, f.versao, f.gerador.as_str(), f.tipo), (Camada::Usuario, 3, "parafuso_sextavado", TipoFixador::ParafusoSextavado));
        // a busca ignora espaços, caixa e vírgula, e acha pelo valor original
        for q in ["1/4-28 UNF", "1/4-28unf", " 1/4 pol ", "-06"] {
            assert!(f.tamanho(q).is_some(), "{q}");
        }
        let t = f.tamanho("1/4 pol").unwrap();
        assert_eq!(t.diametro_nominal(), Some(6.35));
        // cada tamanho tem a sua série, sem regra 2d-10d
        assert_eq!(f.limites_de_comprimento(t), Some((9.5, 15.9)));
        assert_eq!(f.limites_de_comprimento(f.tamanho("-06").unwrap()), Some((8., 10.)));
        // furo de 4,9 mm cai só na faixa do -06 (antes dava "sem tamanho")
        let achados: Vec<_> = f.tamanhos_para_furo(4.9, 0.).iter().map(|t| t.designacao.as_str()).collect();
        assert_eq!(achados, ["-06"]);
        assert!(f.tamanhos_para_furo(5.5, 0.).is_empty());
        assert_eq!(f.tamanhos_para_furo(6.45, 0.)[0].designacao, "1/4-28 UNF");
    }

    #[test]
    fn metricos_herdam_a_faixa_de_furo_da_tabela_de_folgas() {
        let f = familia("iso4762").unwrap();
        // M6: folga fina 6,4 até folgada 7,0
        assert_eq!(f.faixa_furo(f.tamanho("M6").unwrap()), Some((6.4, 7.0)));
        assert_eq!(f.tamanhos_para_furo(6.6, 0.).len(), 1);
    }

    #[test]
    fn validacao_dos_campos_novos() {
        let ruim = |de: &str, para: &str| Biblioteca::de_jsons(Camada::Publica, &[("t.json", &SINTETICA.replacen(de, para, 1))]);
        assert!(ruim("\"revisao\": \"2020\"", "\"revisao\": \"\"").is_err(), "revisão obrigatória");
        assert!(ruim("\"gerador\": \"parafuso_sextavado\"", "\"gerador\": \"inexistente\"").is_err());
        assert!(ruim("\"d_nominal\": 6.35,", "").is_err(), "designação não métrica exige d_nominal");
        assert!(ruim("\"furo_max\": 6.5", "\"furo_max\": 6.0").is_err(), "faixa invertida");
        assert!(ruim("\"grip_max\": 6.4, ", "").is_err(), "grip_min sem grip_max");
        assert!(ruim("[9.5, 12.7, 15.9]", "[12.7, 9.5]").is_err(), "série fora de ordem");
        assert!(ruim("\"P\": \"mm\"", "\"P\": \"furlong\"").is_err(), "unidade desconhecida");
        assert!(ruim("\"nivel\": \"A\"", "\"nivel\": \"C\"").is_err(), "nível C com uma fonte só");
        assert!(ruim("\"designacao\": \"-06\"", "\"designacao\": \"1/4-28unf\"").is_err(), "chave repetida");
    }

    #[test]
    fn camadas_se_sobrepoem_pelo_id() {
        let publica = publica().clone();
        let usuario = Biblioteca::de_jsons(Camada::Usuario, &[("t.json", SINTETICA), ("iso.json", &include_str!("../../../biblioteca/fixadores/iso7089.json").replacen("\"versao\": 1", "\"versao\": 9", 1))]).unwrap();
        let m = Biblioteca::mesclar(&[usuario, publica]); // a ordem de entrada não importa
        assert_eq!(m.camada, Camada::Usuario);
        assert_eq!(m.familias.len(), 44);
        let a = m.familia("iso7089").unwrap();
        assert_eq!((a.camada, a.versao), (Camada::Usuario, 9));
        assert_eq!(m.familia("iso4762").unwrap().camada, Camada::Publica);
        m.validar().unwrap();
    }

    #[test]
    fn toda_familia_publica_tem_revisao_serie_e_fonte() {
        for f in familias() {
            assert!(!f.revisao.is_empty() && f.versao >= 1, "{}", f.id);
            // o gerador existe e pertence ao tipo da família
            assert!(GERADORES.iter().any(|(g, t)| *g == f.gerador && *t == f.tipo), "{}: gerador {} do tipo {:?}", f.id, f.gerador, f.tipo);
            if f.tipo.e_parafuso() {
                for t in &f.tamanhos {
                    assert!(!t.comprimentos.is_empty(), "{} {} sem série própria", f.id, t.designacao);
                }
            }
        }
    }
}
