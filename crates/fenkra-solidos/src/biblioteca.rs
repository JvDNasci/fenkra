//! Biblioteca pública (ADR 0008): famílias de fixadores lidas de arquivos JSON abertos.
//!
//! Os arquivos ficam em `biblioteca/fixadores/` na raiz do repositório, um por família, e são
//! embutidos no binário com `include_str!` (continuam editáveis no repositório). Cada família guarda
//! a norma, as fontes das medidas, o nível de uso (A/B/C/X, ver `docs/biblioteca/fontes-de-dados.md`)
//! e a licença dos dados. As medidas são fatos; a geometria nasce em [`crate::fixadores`].
//!
//! Camadas: hoje só existe a [`Camada::Publica`]. A estrutura já separa "camada" de "família" para
//! que as camadas empresa e usuário/projeto entrem depois, sobrepondo a pública pelo `id`.

use serde::Deserialize;
use std::collections::{BTreeMap, HashSet};
use std::sync::OnceLock;

/// Origem de uma biblioteca. As de cima sobrepõem as de baixo (pública → empresa → usuário).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Camada {
    Publica,
    // Empresa e Usuario entram quando as camadas forem implementadas.
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TipoFixador {
    ParafusoAllen,
    ParafusoSextavado,
    Porca,
    Arruela,
}

impl TipoFixador {
    pub fn rotulo(self) -> &'static str {
        match self {
            TipoFixador::ParafusoAllen => "Parafuso Allen",
            TipoFixador::ParafusoSextavado => "Parafuso sextavado",
            TipoFixador::Porca => "Porca",
            TipoFixador::Arruela => "Arruela",
        }
    }

    pub fn e_parafuso(self) -> bool {
        matches!(self, TipoFixador::ParafusoAllen | TipoFixador::ParafusoSextavado)
    }

    /// Campos que a geometria do tipo precisa em todo tamanho.
    pub fn campos_obrigatorios(self) -> &'static [&'static str] {
        match self {
            TipoFixador::ParafusoAllen => &["P", "dk", "k", "s", "b"],
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
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Tamanho {
    /// "M6", "M8"...
    pub designacao: String,
    /// medidas em mm, uma por nome de `Familia::campos`
    pub valores: BTreeMap<String, f64>,
    /// quantas fontes independentes trazem estes valores
    pub confirmado_por: u32,
    #[serde(default)]
    pub nota: Option<String>,
}

impl Tamanho {
    pub fn valor(&self, campo: &str) -> Option<f64> {
        self.valores.get(campo).copied()
    }

    /// Diâmetro nominal da rosca pela designação ("M6" → 6).
    pub fn diametro_nominal(&self) -> Option<f64> {
        self.designacao.strip_prefix('M')?.parse::<f64>().ok().filter(|d| *d > 0.)
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Familia {
    pub id: String,
    pub nome: String,
    pub norma: String,
    pub tipo: TipoFixador,
    /// nível de uso da fonte: A, B, C ou X
    pub nivel: String,
    pub licenca: String,
    pub fontes: Vec<Fonte>,
    /// nomes das medidas guardadas em cada tamanho
    pub campos: Vec<String>,
    pub tamanhos: Vec<Tamanho>,
    /// série de comprimentos L padrão (só parafusos)
    #[serde(default)]
    pub comprimentos: Vec<f64>,
}

impl Familia {
    pub fn tamanho(&self, designacao: &str) -> Option<&Tamanho> {
        let d = designacao.trim();
        self.tamanhos.iter().find(|t| t.designacao.eq_ignore_ascii_case(d))
    }

    /// Comprimentos L da série que servem ao tamanho: `2·d <= L <= 10·d`, com mínimo de 6 mm.
    pub fn comprimentos_de(&self, t: &Tamanho) -> Vec<f64> {
        let Some(d) = t.diametro_nominal() else { return vec![] };
        self.comprimentos.iter().copied().filter(|l| *l >= (2. * d).max(6.) && *l <= 10. * d).collect()
    }

    pub fn validar(&self) -> Result<(), String> {
        let er = |m: &str| Err(format!("{}: {m}", self.id));
        if self.id.trim().is_empty() || self.nome.trim().is_empty() || self.norma.trim().is_empty() || self.licenca.trim().is_empty() {
            return er("id, nome, norma e licença são obrigatórios");
        }
        if !["A", "B", "C", "X"].contains(&self.nivel.as_str()) {
            return er("nível deve ser A, B, C ou X");
        }
        if self.fontes.is_empty() || self.fontes.iter().any(|f| f.rotulo.trim().is_empty() || !f.url.starts_with("http")) {
            return er("cada família cita ao menos uma fonte, com rótulo e URL");
        }
        if self.campos.is_empty() {
            return er("sem campos");
        }
        for c in self.tipo.campos_obrigatorios() {
            if !self.campos.iter().any(|x| x == c) {
                return er(&format!("falta o campo '{c}' exigido pelo tipo {}", self.tipo.rotulo()));
            }
        }
        if self.tamanhos.is_empty() {
            return er("sem tamanhos");
        }
        let mut vistos = HashSet::new();
        for t in &self.tamanhos {
            if t.diametro_nominal().is_none() {
                return er(&format!("designação '{}' não é do tipo M<diâmetro>", t.designacao));
            }
            if !vistos.insert(t.designacao.as_str()) {
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
        }
        if self.tipo.e_parafuso() {
            if self.comprimentos.is_empty() {
                return er("parafuso sem série de comprimentos");
            }
            if self.comprimentos.iter().any(|l| !(*l > 0.)) || self.comprimentos.windows(2).any(|w| w[0] >= w[1]) {
                return er("comprimentos devem ser positivos e crescentes");
            }
            for t in &self.tamanhos {
                if self.comprimentos_de(t).is_empty() {
                    return er(&format!("{}: nenhum comprimento da série serve ao tamanho", t.designacao));
                }
            }
        }
        Ok(())
    }
}

/// Conjunto de famílias de uma camada.
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
            let f: Familia = serde_json::from_str(texto).map_err(|e| format!("{arquivo}: {e}"))?;
            f.validar().map_err(|e| format!("{arquivo}: {e}"))?;
            if familias.iter().any(|o| o.id == f.id) {
                return Err(format!("{arquivo}: id '{}' repetido", f.id));
            }
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
}

/// Arquivos da camada pública, embutidos em tempo de compilação.
const PUBLICA: &[(&str, &str)] = &[
    ("iso4762.json", include_str!("../../../biblioteca/fixadores/iso4762.json")),
    ("iso4017.json", include_str!("../../../biblioteca/fixadores/iso4017.json")),
    ("din933.json", include_str!("../../../biblioteca/fixadores/din933.json")),
    ("iso4032.json", include_str!("../../../biblioteca/fixadores/iso4032.json")),
    ("din934.json", include_str!("../../../biblioteca/fixadores/din934.json")),
    ("iso7089.json", include_str!("../../../biblioteca/fixadores/iso7089.json")),
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
        assert_eq!(b.familias.len(), 6);
        b.validar().unwrap();
        for id in ["iso4762", "iso4017", "din933", "iso4032", "din934", "iso7089"] {
            let f = familia(id).unwrap_or_else(|| panic!("{id} ausente"));
            assert_eq!(f.tamanhos.len(), 9, "{id}");
            assert_eq!(f.nivel, "C");
            assert!(f.licenca.starts_with("CC BY 4.0"));
        }
        assert_eq!(familias().iter().filter(|f| f.tipo.e_parafuso()).count(), 3);
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
}
