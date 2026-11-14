// SPDX-License-Identifier: LGPL-2.1-or-later
//! Biblioteca de correntes e correias (B342): dados normativos com **nível de conferência por linha**
//! e as contas de corrente (elos, distância entre centros corrigida, diâmetros da roda) e de correia
//! (comprimento e distância entre centros).
//!
//! Regra dos dados: só entra como [`Conferencia::DuasFontes`] a linha cujo valor duas fontes gratuitas
//! públicas trazem igual; os outros ficam na lista como `UmaFonte` ou `Conflito` e **a geração os
//! recusa** (ver [`Corrente::liberada`]). Nada foi escrito de memória: cada linha cita as fontes
//! consultadas em 2026-11-06 (também em `docs/arquitetura/procedencia.md`).
//!
//! Fontes (todas gratuitas, consultadas em 2026-11-06):
//! - F1: Wikipedia, "Roller chain" (tabela ASME/ANSI B29.1: passo e diâmetro máximo do rolo; largura nominal);
//! - F2: Texas Belting, tabela de correntes de rolos ANSI (largura interna) e "Timing Belt Pitch Chart";
//! - F3: listagens de distribuidor (RS) e tabela do fabricante China Transmissions para a série ISO 606 (B);
//! - F4: Wikipedia, "V-belt" (seções clássicas);
//! - F5: folha de dados DIN 2215 do distribuidor Maedler e tabela de seções da Texas Belting;
//! - F6: zzr-parts (tabela de passos de correias dentadas);
//! - F7: tabela de correntes de kart de revendedor (gokartsusa), ficha da corrente 219 do fabricante CZ (speedwear), tabela de tamanhos de corrente de moto (cyclechaos) e especificações de revendedor de peças (monsterscooterparts).

use std::f64::consts::PI;

/// Quantas fontes concordam com a linha.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Conferencia {
    /// duas fontes gratuitas independentes trazem o mesmo valor: liberada para gerar
    DuasFontes,
    /// só uma fonte: aguarda a segunda
    UmaFonte,
    /// as fontes divergem: aguarda a norma
    Conflito,
}

impl Conferencia {
    pub fn rotulo(self) -> &'static str {
        match self {
            Conferencia::DuasFontes => "conferido em duas fontes",
            Conferencia::UmaFonte => "uma fonte só (pendente)",
            Conferencia::Conflito => "fontes divergem (pendente)",
        }
    }
}

/// Linha da biblioteca de correntes de rolos (mm).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Corrente {
    pub designacao: &'static str,
    pub norma: &'static str,
    pub passo: f64,
    /// diâmetro do rolo (máximo, como as tabelas dão)
    pub rolo: f64,
    /// largura entre as placas internas; 0 = não conferida
    pub largura: f64,
    pub conferencia: Conferencia,
    pub fontes: &'static str,
    pub nota: &'static str,
}

impl Corrente {
    /// Só linhas conferidas em duas fontes geram peça.
    pub fn liberada(&self) -> bool {
        self.conferencia == Conferencia::DuasFontes && self.largura > 0.
    }
}

use Conferencia::*;

/// Correntes de rolos de uma carreira. Largura 0 = ainda sem valor conferido (não libera a geração).
pub const CORRENTES: &[Corrente] = &[
    Corrente { designacao: "ANSI 25", norma: "ASME B29.1", passo: 6.35, rolo: 3.30, largura: 0., conferencia: UmaFonte, fontes: "F1", nota: "passo e rolo só na F1; largura sem valor conferido" },
    Corrente { designacao: "ANSI 35", norma: "ASME B29.1", passo: 9.525, rolo: 5.08, largura: 0., conferencia: UmaFonte, fontes: "F1", nota: "passo e rolo só na F1; largura sem valor conferido" },
    Corrente { designacao: "ANSI 40", norma: "ASME B29.1", passo: 12.7, rolo: 7.92, largura: 0., conferencia: Conflito, fontes: "F1, F2", nota: "passo e rolo concordam; a largura interna ficou sem valor único (7,85 mm não apareceu em nenhuma fonte)" },
    Corrente { designacao: "ANSI 50", norma: "ASME B29.1", passo: 15.875, rolo: 10.16, largura: 9.525, conferencia: DuasFontes, fontes: "F1, F2", nota: "largura nominal 3/8 pol" },
    Corrente { designacao: "ANSI 60", norma: "ASME B29.1", passo: 19.05, rolo: 11.91, largura: 12.7, conferencia: DuasFontes, fontes: "F1, F2", nota: "largura nominal 1/2 pol" },
    Corrente { designacao: "ANSI 80", norma: "ASME B29.1", passo: 25.4, rolo: 15.88, largura: 15.875, conferencia: DuasFontes, fontes: "F1, F2", nota: "largura nominal 5/8 pol" },
    Corrente { designacao: "ISO 06B-1", norma: "ISO 606", passo: 9.525, rolo: 6.35, largura: 5.72, conferencia: Conflito, fontes: "F3", nota: "o rolo aparece como 6,35 e como 5 mm conforme a fonte" },
    Corrente { designacao: "ISO 08B-1", norma: "ISO 606", passo: 12.7, rolo: 8.51, largura: 7.75, conferencia: DuasFontes, fontes: "F3 (distribuidor RS e fabricante)", nota: "" },
    Corrente { designacao: "ISO 10B-1", norma: "ISO 606", passo: 15.875, rolo: 10.16, largura: 9.65, conferencia: UmaFonte, fontes: "F3 (fabricante)", nota: "só uma tabela traz as três medidas" },
    Corrente { designacao: "428", norma: "corrente de moto e kart", passo: 12.7, rolo: 8.51, largura: 7.75, conferencia: DuasFontes, fontes: "F7 (tabela de kart e listagem de peças)", nota: "passo e rolo concordam em tabelas de kart e de moto; largura 7,75 em duas fontes, uma terceira dá 0,313 pol (7,95 mm)" },
    // B354/B395: passo e rolo concordam nas duas fontes; a largura interna diverge (4,6 x 5,00 mm) e
    // o usuário decidiu, em 2026-11-13, usar a menor das duas, 4,6 mm: o dente feito para a corrente
    // mais estreita ainda entra na mais larga. Deixa de ser provisória e libera a geração.
    Corrente { designacao: "219", norma: "corrente de kart", passo: 7.774, rolo: 4.59, largura: 4.6, conferencia: DuasFontes, fontes: "F7 (tabela de kart e ficha de fabricante)", nota: "passo e rolo concordam; largura interna 4,6 mm (decisão do usuário: a menor das fontes, que dão 4,6 e 5,00 mm)" },
];

pub fn corrente(designacao: &str) -> Option<&'static Corrente> {
    CORRENTES.iter().find(|c| c.designacao == designacao)
}

/// Linhas em que passo e rolo concordam e só a largura interna diverge entre as fontes. A geração
/// usaria a **menor** largura das fontes como valor provisório, porque o dente feito para a corrente
/// mais estreita ainda entra na mais larga, e a peça levaria o aviso. Está vazia: a 219, que era a
/// única (B395), teve a largura decidida pelo usuário (4,6 mm) e saiu daqui como linha liberada.
pub const LARGURAS_PROVISORIAS: &[(&str, f64, &str)] = &[];

/// Corrente pronta para gerar e o aviso, quando a largura é provisória. Recusa as linhas sem duas
/// fontes que não estão em [`LARGURAS_PROVISORIAS`].
pub fn corrente_para_gerar(designacao: &str) -> Result<(Corrente, Option<String>), String> {
    let c = corrente(designacao).ok_or_else(|| format!("corrente '{designacao}' fora da biblioteca"))?;
    if c.liberada() {
        return Ok((*c, None));
    }
    match LARGURAS_PROVISORIAS.iter().find(|(d, _, _)| *d == c.designacao) {
        Some((_, w, aviso)) if c.conferencia == Conferencia::Conflito && c.largura == 0. => Ok((Corrente { largura: *w, ..*c }, Some((*aviso).to_string()))),
        _ => Err(format!("{}: dado {}: a geração só usa linhas conferidas em duas fontes", c.designacao, c.conferencia.rotulo())),
    }
}

/// Tipo de correia da biblioteca.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FamiliaCorreia {
    Dentada,
    EmV,
}

/// Linha da biblioteca de correias. Dentada: `passo` é o passo dos dentes; em V: `passo` fica 0 e
/// `largura_topo`/`altura` descrevem a seção. Medidas em mm; 0 = sem valor conferido.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Correia {
    pub designacao: &'static str,
    pub familia: FamiliaCorreia,
    pub passo: f64,
    pub largura_topo: f64,
    pub altura: f64,
    pub conferencia: Conferencia,
    pub fontes: &'static str,
    pub nota: &'static str,
}

impl Correia {
    pub fn liberada(&self) -> bool {
        self.conferencia == Conferencia::DuasFontes
    }
}

pub const CORREIAS: &[Correia] = &[
    Correia { designacao: "MXL", familia: FamiliaCorreia::Dentada, passo: 2.032, largura_topo: 0., altura: 0., conferencia: DuasFontes, fontes: "F2, F6", nota: "passo 0,080 pol" },
    Correia { designacao: "XL", familia: FamiliaCorreia::Dentada, passo: 5.08, largura_topo: 0., altura: 0., conferencia: DuasFontes, fontes: "F2, F6", nota: "passo 1/5 pol" },
    Correia { designacao: "L", familia: FamiliaCorreia::Dentada, passo: 9.525, largura_topo: 0., altura: 0., conferencia: DuasFontes, fontes: "F2, F6", nota: "passo 3/8 pol" },
    Correia { designacao: "H", familia: FamiliaCorreia::Dentada, passo: 12.7, largura_topo: 0., altura: 0., conferencia: DuasFontes, fontes: "F2, F6", nota: "passo 1/2 pol" },
    Correia { designacao: "HTD 3M", familia: FamiliaCorreia::Dentada, passo: 3., largura_topo: 0., altura: 0., conferencia: DuasFontes, fontes: "F2, F6", nota: "perfil curvilíneo" },
    Correia { designacao: "HTD 5M", familia: FamiliaCorreia::Dentada, passo: 5., largura_topo: 0., altura: 0., conferencia: DuasFontes, fontes: "F2, F6", nota: "perfil curvilíneo" },
    Correia { designacao: "HTD 8M", familia: FamiliaCorreia::Dentada, passo: 8., largura_topo: 0., altura: 0., conferencia: DuasFontes, fontes: "F2, F6", nota: "perfil curvilíneo" },
    Correia { designacao: "GT2", familia: FamiliaCorreia::Dentada, passo: 2., largura_topo: 0., altura: 0., conferencia: DuasFontes, fontes: "F2, listagens de distribuidor", nota: "perfil curvilíneo, não intercambiável com MXL" },
    Correia { designacao: "T5", familia: FamiliaCorreia::Dentada, passo: 5., largura_topo: 0., altura: 0., conferencia: UmaFonte, fontes: "F2", nota: "" },
    Correia { designacao: "V Z", familia: FamiliaCorreia::EmV, passo: 0., largura_topo: 10., altura: 6., conferencia: DuasFontes, fontes: "F4, F5", nota: "largura e altura concordam" },
    Correia { designacao: "V A", familia: FamiliaCorreia::EmV, passo: 0., largura_topo: 13., altura: 8., conferencia: Conflito, fontes: "F4, F5", nota: "largura 13 concorda; altura 9 (F4) × 8 (F5)" },
    Correia { designacao: "V B", familia: FamiliaCorreia::EmV, passo: 0., largura_topo: 17., altura: 11., conferencia: Conflito, fontes: "F4, F5", nota: "largura 17 concorda; altura 11 (F4 e DIN) × 10,5 (tabela em polegadas)" },
    Correia { designacao: "V C", familia: FamiliaCorreia::EmV, passo: 0., largura_topo: 22., altura: 14., conferencia: Conflito, fontes: "F4, F5", nota: "largura 22 concorda; altura 14 (F4) × 13,5 (F5)" },
    Correia { designacao: "V D", familia: FamiliaCorreia::EmV, passo: 0., largura_topo: 32., altura: 19., conferencia: DuasFontes, fontes: "F4, F5", nota: "largura e altura concordam" },
];

pub fn correia(designacao: &str) -> Option<&'static Correia> {
    CORREIAS.iter().find(|c| c.designacao == designacao)
}

// ---------------------------------------------------------------- corrente de rolos

/// Diâmetro primitivo da roda de corrente: `D = p / sen(180° / N)` (geometria do polígono de passo).
pub fn diametro_primitivo_roda(passo: f64, dentes: u32) -> f64 {
    passo / (PI / dentes as f64).sin()
}

/// Diâmetro de topo da roda: `Da = p (0,6 + cot(180° / N))`, a fórmula de projeto de rodas de rolos
/// de uso corrente nos catálogos (valor de referência; o limite vem do fabricante).
pub fn diametro_topo_roda(passo: f64, dentes: u32) -> f64 {
    passo * (0.6 + 1. / (PI / dentes as f64).tan())
}

/// Número de elos (par) para a distância entre centros `c` (mm): `L = 2c/p + (N1+N2)/2 + ((N2−N1)/2π)² p/c`,
/// arredondado para cima ao próximo par, porque a corrente fecha com elos de rolo e de pino pares.
pub fn elos_da_corrente(passo: f64, n1: u32, n2: u32, c: f64) -> u32 {
    let (a, b) = (n1 as f64, n2 as f64);
    let l = 2. * c / passo + (a + b) / 2. + ((b - a) / (2. * PI)).powi(2) * passo / c;
    let teto = l.ceil() as u32;
    teto + teto % 2
}

/// Distância entre centros corrigida para `elos` elos:
/// `c = p/4 [ (L − (N1+N2)/2) + sqrt((L − (N1+N2)/2)² − 8 ((N2−N1)/2π)²) ]`. `None` se a corrente
/// é curta demais para as duas rodas.
pub fn distancia_corrigida_corrente(passo: f64, n1: u32, n2: u32, elos: u32) -> Option<f64> {
    let (a, b) = (n1 as f64, n2 as f64);
    let m = elos as f64 - (a + b) / 2.;
    let disc = m * m - 8. * ((b - a) / (2. * PI)).powi(2);
    (disc >= 0. && m > 0.).then(|| passo / 4. * (m + disc.sqrt()))
}

// ---------------------------------------------------------------- correia

/// Comprimento da correia aberta com polias de diâmetros `d1`, `d2` e centros `c`:
/// `L = 2c + π(d1+d2)/2 + (d2−d1)²/(4c)` (aproximação usual, boa para ângulos de abraçamento folgados).
pub fn comprimento_correia(d1: f64, d2: f64, c: f64) -> f64 {
    2. * c + PI * (d1 + d2) / 2. + (d2 - d1).powi(2) / (4. * c)
}

/// Distância entre centros para o comprimento `l`: com `B = l − π(d1+d2)/2`,
/// `c = (B + sqrt(B² − 2(d2−d1)²)) / 4`.
pub fn distancia_da_correia(d1: f64, d2: f64, l: f64) -> Option<f64> {
    let b = l - PI * (d1 + d2) / 2.;
    let disc = b * b - 2. * (d2 - d1).powi(2);
    (disc >= 0. && b > 0.).then(|| (b + disc.sqrt()) / 4.)
}

/// Ângulo de abraçamento da polia menor (graus): `180° − 2 asen((d2 − d1) / 2c)`.
pub fn abracamento_menor(d1: f64, d2: f64, c: f64) -> f64 {
    180. - 2. * ((d2 - d1).abs() / (2. * c)).clamp(0., 1.).asin().to_degrees()
}

/// Dentes de uma correia dentada: o comprimento dividido pelo passo, arredondado ao inteiro mais
/// próximo (a correia só existe em número inteiro de dentes).
pub fn dentes_da_correia(passo: f64, comprimento: f64) -> u32 {
    (comprimento / passo).round().max(1.) as u32
}

/// Diâmetro primitivo da polia dentada: `d = N p / π`.
pub fn diametro_polia_dentada(passo: f64, dentes: u32) -> f64 {
    dentes as f64 * passo / PI
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn so_linhas_conferidas_em_duas_fontes_geram() {
        let lib: Vec<_> = CORRENTES.iter().filter(|c| c.liberada()).map(|c| c.designacao).collect();
        assert_eq!(lib, ["ANSI 50", "ANSI 60", "ANSI 80", "ISO 08B-1", "428", "219"]);
        // B354/B395: a 219 sai liberada com a largura decidida pelo usuário, 4,6 mm, sem aviso
        let c219 = corrente("219").unwrap();
        assert_eq!((c219.passo, c219.rolo, c219.largura), (7.774, 4.59, 4.6));
        assert!(c219.liberada() && c219.conferencia == Conferencia::DuasFontes);
        let (c, aviso) = corrente_para_gerar("219").unwrap();
        assert_eq!((c.passo, c.rolo, c.largura), (7.774, 4.59, 4.6));
        assert_eq!(aviso, None);
        assert!(LARGURAS_PROVISORIAS.is_empty());
        assert!(corrente_para_gerar("ANSI 40").is_err() && corrente_para_gerar("ISO 06B-1").is_err());
        assert_eq!(corrente_para_gerar("428").unwrap().1, None);
        // linhas com uma fonte ou conflito ficam na lista, rotuladas, mas não liberadas
        assert!(!corrente("ANSI 40").unwrap().liberada());
        assert!(!corrente("ISO 06B-1").unwrap().liberada());
        assert_eq!(corrente("ANSI 40").unwrap().conferencia, Conferencia::Conflito);
        assert!(CORRENTES.iter().all(|c| !c.fontes.is_empty()), "toda linha cita a fonte");
        assert!(CORREIAS.iter().all(|c| !c.fontes.is_empty()));
        // a correia em V com altura divergente não libera
        assert!(correia("V Z").unwrap().liberada() && correia("V D").unwrap().liberada());
        assert!(!correia("V A").unwrap().liberada() && !correia("T5").unwrap().liberada());
        // dados da tabela da corrente 08B-1: passo 12,7, rolo 8,51, largura 7,75
        let c = corrente("ISO 08B-1").unwrap();
        assert_eq!((c.passo, c.rolo, c.largura), (12.7, 8.51, 7.75));
        assert!(CORREIAS.iter().filter(|c| c.familia == FamiliaCorreia::Dentada).all(|c| c.passo > 0.));
    }

    #[test]
    fn diametros_da_roda_exemplo_numerico() {
        // passo 12,7, 19 dentes: D = 12,7 / sen(9,4737°) = 77,16 mm; Da = 12,7 (0,6 + 5,993) = 83,7 mm
        let d = diametro_primitivo_roda(12.7, 19);
        assert!((d - 77.16).abs() < 0.01, "{d}");
        let da = diametro_topo_roda(12.7, 19);
        assert!((da - 83.73).abs() < 0.05, "{da}");
        assert!(da > d);
        // 2 dentes de roda: o polígono degenera em 2 pontos a p/sen(90°) = p
        assert!((diametro_primitivo_roda(10., 2) - 10.).abs() < 1e-12);
    }

    #[test]
    fn elos_e_distancia_corrigida() {
        // passo 12,7; 17 e 34 dentes; c pedido 400 mm: L = 62,992 + 25,5 + 0,3329 ... ≈ 88,8 -> 90 elos (par)
        let l = elos_da_corrente(12.7, 17, 34, 400.);
        assert_eq!(l, 90);
        let c = distancia_corrigida_corrente(12.7, 17, 34, l).unwrap();
        // distância corrigida perto de 400 (a sobra de 1,2 elo vira ~7,6 mm) e é fechada pela fórmula
        assert!(c > 400. && c < 415., "{c}");
        // fechamento: com a distância corrigida, a fórmula de L devolve o número de elos
        let l_volta = 2. * c / 12.7 + 25.5 + ((34. - 17.) / (2. * PI)).powi(2) * 12.7 / c;
        assert!((l_volta - 90.).abs() < 0.02, "{l_volta}");
        // corrente curta demais não fecha
        assert!(distancia_corrigida_corrente(12.7, 17, 34, 30).is_none());
        // sempre par
        for c in [150., 233., 401., 777.] {
            assert_eq!(elos_da_corrente(12.7, 15, 45, c) % 2, 0);
        }
    }

    #[test]
    fn correia_comprimento_e_distancia_ida_e_volta() {
        // d1 = 50, d2 = 100, c = 200: L = 400 + 235,62 + 3,125 = 638,74
        let l = comprimento_correia(50., 100., 200.);
        assert!((l - 638.74).abs() < 0.01, "{l}");
        let c = distancia_da_correia(50., 100., l).unwrap();
        assert!((c - 200.).abs() < 0.05, "{c}");
        assert!(distancia_da_correia(50., 100., 100.).is_none());
        // abraçamento: 180° − 2·asen(25/200) = 165,6°
        assert!((abracamento_menor(50., 100., 200.) - 165.64).abs() < 0.01);
        assert!((abracamento_menor(80., 80., 200.) - 180.).abs() < 1e-9);
    }

    #[test]
    fn polia_dentada_pelo_passo() {
        // 20 dentes GT2 (2 mm): d = 40/π = 12,73
        assert!((diametro_polia_dentada(2., 20) - 12.732).abs() < 1e-3);
        assert_eq!(dentes_da_correia(2., 300.), 150);
        assert_eq!(dentes_da_correia(2.032, 300.), 148);
    }
}
