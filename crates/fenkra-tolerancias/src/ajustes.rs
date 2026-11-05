//! Classes de tolerância ("H7", "g6"), ajustes ("H7/g6") e seus limites.

use crate::desvios::{limites_eixo, limites_furo, Desvios, Letra};
use crate::it::ItGrau;
use serde::{Deserialize, Serialize};

/// Zona de tolerância de um furo (letra maiúscula) ou de um eixo (minúscula).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, Serialize, Deserialize)]
pub struct ClasseTol {
    pub letra: Letra,
    pub grau: ItGrau,
    /// `true` = furo (letra maiúscula).
    pub furo: bool,
}

impl ClasseTol {
    /// Lê "H7", "g6", "JS9", "zc10". Letras maiúsculas = furo; minúsculas = eixo; mistura é erro.
    pub fn parse(s: &str) -> Option<ClasseTol> {
        let s = s.trim();
        let i = s.find(|c: char| c.is_ascii_digit())?;
        let (l, g) = s.split_at(i);
        if l.is_empty() {
            return None;
        }
        let furo = l.chars().all(|c| c.is_ascii_uppercase());
        let eixo = l.chars().all(|c| c.is_ascii_lowercase());
        if !furo && !eixo {
            return None;
        }
        Some(ClasseTol { letra: Letra::parse(l)?, grau: ItGrau::parse(g)?, furo })
    }
    pub fn texto(&self) -> String {
        let l = self.letra.texto();
        let l = if self.furo { l.to_string() } else { l.to_ascii_lowercase() };
        format!("{l}{}", self.grau.numero().max(0))
    }
    /// Desvios em µm para o diâmetro nominal `d` (mm).
    pub fn desvios(&self, d: f64) -> Option<Desvios> {
        if self.furo {
            limites_furo(self.letra, self.grau, d)
        } else {
            limites_eixo(self.letra, self.grau, d)
        }
    }
    /// Dimensão máxima e mínima (mm) para o nominal `d`.
    pub fn limites_mm(&self, d: f64) -> Option<(f64, f64)> {
        let x = self.desvios(d)?;
        Some((arred(d + x.sup_mm()), arred(d + x.inf_mm())))
    }
}

fn arred(v: f64) -> f64 {
    (v * 1e6).round() / 1e6
}

/// Tipo do ajuste pelo sinal das folgas extremas.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum TipoAjuste {
    /// folga mínima ≥ 0: sempre há folga
    Folga,
    /// pode dar folga ou interferência
    Incerto,
    /// folga máxima ≤ 0: sempre há interferência
    Interferencia,
}

impl TipoAjuste {
    pub fn rotulo(self) -> &'static str {
        match self {
            TipoAjuste::Folga => "com folga",
            TipoAjuste::Incerto => "incerto",
            TipoAjuste::Interferencia => "com interferência",
        }
    }
}

/// Furo e eixo de mesmo diâmetro nominal.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct Ajuste {
    pub furo: ClasseTol,
    pub eixo: ClasseTol,
}

/// Resultado da análise de um ajuste, em µm. Folga negativa = interferência.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct AnaliseAjuste {
    pub folga_max_um: f64,
    pub folga_min_um: f64,
    pub tipo: TipoAjuste,
}

impl Ajuste {
    /// Lê "H7/g6" (aceita também "H7-g6" e "H7 g6").
    pub fn parse(s: &str) -> Option<Ajuste> {
        let mut it = s.split(['/', '-', ' ']).filter(|p| !p.is_empty());
        let (a, b) = (it.next()?, it.next()?);
        if it.next().is_some() {
            return None;
        }
        let (furo, eixo) = (ClasseTol::parse(a)?, ClasseTol::parse(b)?);
        (furo.furo && !eixo.furo).then_some(Ajuste { furo, eixo })
    }
    pub fn texto(&self) -> String {
        format!("{}/{}", self.furo.texto(), self.eixo.texto())
    }
    pub fn analisar(&self, d: f64) -> Option<AnaliseAjuste> {
        let (f, e) = (self.furo.desvios(d)?, self.eixo.desvios(d)?);
        let folga_max_um = f.sup_um - e.inf_um;
        let folga_min_um = f.inf_um - e.sup_um;
        let tipo = if folga_min_um >= 0. {
            TipoAjuste::Folga
        } else if folga_max_um <= 0. {
            TipoAjuste::Interferencia
        } else {
            TipoAjuste::Incerto
        };
        Some(AnaliseAjuste { folga_max_um, folga_min_um, tipo })
    }
    /// Sistema de ajuste: "furo-base" (H), "eixo-base" (h) ou "misto".
    pub fn sistema(&self) -> &'static str {
        match (self.furo.letra, self.eixo.letra) {
            (Letra::H, _) => "furo-base",
            (_, Letra::H) => "eixo-base",
            _ => "misto",
        }
    }
}

/// Os campos que o `limits_and_fits` do AP242 pede para uma cota com classe ISO 286.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct LimitesAp242 {
    /// letra da variância (fundamental deviation), na caixa da classe
    pub variancia_forma: String,
    /// grau IT, como "IT7"
    pub grau: String,
    pub furo: bool,
    pub fonte: &'static str,
    pub nominal_mm: f64,
    pub desvio_sup_mm: f64,
    pub desvio_inf_mm: f64,
    pub limite_max_mm: f64,
    pub limite_min_mm: f64,
    pub versao_tabelas: u32,
}

impl LimitesAp242 {
    pub fn de(classe: &ClasseTol, nominal_mm: f64) -> Option<LimitesAp242> {
        let x = classe.desvios(nominal_mm)?;
        let (max, min) = classe.limites_mm(nominal_mm)?;
        let l = classe.letra.texto();
        Some(LimitesAp242 {
            variancia_forma: if classe.furo { l.into() } else { l.to_ascii_lowercase() },
            grau: classe.grau.nome(),
            furo: classe.furo,
            fonte: "ISO 286-1",
            nominal_mm,
            desvio_sup_mm: arred(x.sup_mm()),
            desvio_inf_mm: arred(x.inf_mm()),
            limite_max_mm: max,
            limite_min_mm: min,
            versao_tabelas: crate::VERSAO_TABELAS,
        })
    }
}

/// Um desvio em mm no texto brasileiro com sinal: "+0,021", "0", "-0,007".
pub fn formatar_desvio_mm(v_mm: f64) -> String {
    let v = arred(v_mm);
    if v == 0. {
        return "0".into();
    }
    let mut s = format!("{:.4}", v.abs());
    while s.ends_with('0') && s.matches('.').count() == 1 && s.split('.').nth(1).is_some_and(|f| f.len() > 3) {
        s.pop();
    }
    let sinal = if v > 0. { '+' } else { '-' };
    format!("{sinal}{}", s.replace('.', ","))
}

/// Par de textos (superior, inferior) de uma classe para o diâmetro `d`, pronto para a cota.
pub fn textos_desvios(classe: &ClasseTol, d: f64) -> Option<(String, String)> {
    let x = classe.desvios(d)?;
    Some((formatar_desvio_mm(x.sup_mm()), formatar_desvio_mm(x.inf_mm())))
}
