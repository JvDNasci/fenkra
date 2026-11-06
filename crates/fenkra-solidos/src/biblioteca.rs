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
}

/// Geradores de geometria nomeados. Uma família escolhe o gerador pelo nome (`"gerador"` no JSON);
/// nova família que usa um gerador existente é só um arquivo de dados, sem código.
pub const GERADORES: &[(&str, TipoFixador)] = &[
    ("parafuso_allen", TipoFixador::ParafusoAllen),
    ("parafuso_sextavado", TipoFixador::ParafusoSextavado),
    ("porca", TipoFixador::Porca),
    ("arruela", TipoFixador::Arruela),
];

impl TipoFixador {
    pub fn rotulo(self) -> &'static str {
        match self {
            TipoFixador::ParafusoAllen => "Parafuso Allen",
            TipoFixador::ParafusoSextavado => "Parafuso sextavado",
            TipoFixador::Porca => "Porca",
            TipoFixador::Arruela => "Arruela",
        }
    }

    /// Nome do gerador deste tipo.
    pub fn gerador(self) -> &'static str {
        GERADORES.iter().find(|(_, t)| *t == self).map(|(n, _)| *n).unwrap_or("")
    }

    pub fn e_parafuso(self) -> bool {
        matches!(self, TipoFixador::ParafusoAllen | TipoFixador::ParafusoSextavado)
    }

    /// Campos que a geometria do tipo precisa em todo tamanho.
    pub fn campos_obrigatorios(self) -> &'static [&'static str] {
        match self {
            // "b" é opcional: a cabeça abaulada (ISO 7380-1) não publica comprimento roscado de referência
            TipoFixador::ParafusoAllen => &["P", "dk", "k", "s"],
            TipoFixador::ParafusoSextavado => &["P", "s", "k"],
            TipoFixador::Porca => &["s", "m"],
            TipoFixador::Arruela => &["d1", "d2", "h"],
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
    /// nome do gerador de geometria (ver [`GERADORES`])
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
            gerador: tipo.gerador().to_string(),
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
        if self.campos.is_empty() {
            return er("sem campos");
        }
        for c in self.tipo.campos_obrigatorios() {
            if !self.campos.iter().any(|x| x == c) {
                return er(&format!("falta o campo '{c}' exigido pelo tipo {}", self.tipo.rotulo()));
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
            if t.confirmado_por == 0 {
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
        if self.tipo.e_parafuso() {
            for t in &self.tamanhos {
                if self.comprimentos_de(t).is_empty() {
                    return er(&format!("{}: parafuso sem série de comprimentos", t.designacao));
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
        assert_eq!(b.familias.len(), 9);
        b.validar().unwrap();
        for id in ["iso4762", "iso4017", "din933", "iso4032", "din934", "iso7089"] {
            let f = familia(id).unwrap_or_else(|| panic!("{id} ausente"));
            assert_eq!(f.tamanhos.len(), 9, "{id}");
            assert_eq!(f.nivel, "C");
            assert!(f.licenca.starts_with("CC BY 4.0"));
        }
        assert_eq!(familias().iter().filter(|f| f.tipo.e_parafuso()).count(), 5);
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
        assert_eq!(m.familias.len(), 10);
        let a = m.familia("iso7089").unwrap();
        assert_eq!((a.camada, a.versao), (Camada::Usuario, 9));
        assert_eq!(m.familia("iso4762").unwrap().camada, Camada::Publica);
        m.validar().unwrap();
    }

    #[test]
    fn toda_familia_publica_tem_revisao_serie_e_fonte() {
        for f in familias() {
            assert!(!f.revisao.is_empty() && f.versao >= 1, "{}", f.id);
            assert_eq!(f.gerador, f.tipo.gerador());
            if f.tipo.e_parafuso() {
                for t in &f.tamanhos {
                    assert!(!t.comprimentos.is_empty(), "{} {} sem série própria", f.id, t.designacao);
                }
            }
        }
    }
}
