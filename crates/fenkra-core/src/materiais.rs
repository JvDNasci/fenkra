// SPDX-License-Identifier: LGPL-2.1-or-later
//! B92: tabela de materiais de engenharia (densidade) e o material efetivo de cada corpo.
//!
//! Densidades em g/cm³ (= t/m³), valores típicos de catálogo à temperatura ambiente. A tabela é
//! a fonte única: massa, inércia, carimbo do desenho e lista de materiais leem daqui.

use crate::{Document, Id};

/// Um material da tabela.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Material {
    pub nome: &'static str,
    /// massa específica em g/cm³
    pub densidade: f64,
    pub familia: &'static str,
}

/// Densidade usada quando o material não está na tabela (aço carbono).
pub const DENSIDADE_PADRAO: f64 = 7.85;

/// Os oito primeiros nomes e valores coincidem com a lista histórica da interface.
pub const MATERIAIS: &[Material] = &[
    Material { nome: "Aço 1020", densidade: 7.87, familia: "Aço" },
    Material { nome: "Aço 1045", densidade: 7.85, familia: "Aço" },
    Material { nome: "Aço inox 304", densidade: 8.0, familia: "Aço" },
    Material { nome: "Alumínio 6061", densidade: 2.7, familia: "Alumínio" },
    Material { nome: "Latão", densidade: 8.5, familia: "Cobre" },
    Material { nome: "Nylon", densidade: 1.14, familia: "Polímero" },
    Material { nome: "ABS", densidade: 1.05, familia: "Polímero" },
    Material { nome: "PLA", densidade: 1.24, familia: "Polímero" },
    Material { nome: "Aço inox 316", densidade: 8.0, familia: "Aço" },
    Material { nome: "Aço ferramenta D2", densidade: 7.7, familia: "Aço" },
    Material { nome: "Ferro fundido cinzento", densidade: 7.2, familia: "Ferro" },
    Material { nome: "Alumínio 7075", densidade: 2.81, familia: "Alumínio" },
    Material { nome: "Alumínio 5052", densidade: 2.68, familia: "Alumínio" },
    Material { nome: "Cobre", densidade: 8.96, familia: "Cobre" },
    Material { nome: "Bronze", densidade: 8.8, familia: "Cobre" },
    Material { nome: "Titânio Ti-6Al-4V", densidade: 4.43, familia: "Titânio" },
    Material { nome: "Magnésio AZ31", densidade: 1.77, familia: "Magnésio" },
    Material { nome: "Zinco", densidade: 7.14, familia: "Zinco" },
    Material { nome: "PETG", densidade: 1.27, familia: "Polímero" },
    Material { nome: "Policarbonato", densidade: 1.2, familia: "Polímero" },
    Material { nome: "Acetal (POM)", densidade: 1.41, familia: "Polímero" },
    Material { nome: "Polipropileno", densidade: 0.9, familia: "Polímero" },
    Material { nome: "Madeira (pinus)", densidade: 0.5, familia: "Madeira" },
    Material { nome: "Concreto", densidade: 2.4, familia: "Construção" },
    Material { nome: "Vidro", densidade: 2.5, familia: "Cerâmica" },
    // B408: o kart pediu cromo-molibdênio e borracha, e os dois caíam na densidade padrão
    Material { nome: "Aço AISI 4130", densidade: 7.85, familia: "Aço" },
    Material { nome: "Aço AISI 4140", densidade: 7.85, familia: "Aço" },
    Material { nome: "Borracha natural", densidade: 0.93, familia: "Elastômero" },
    Material { nome: "Borracha nitrílica (NBR)", densidade: 1.0, familia: "Elastômero" },
    Material { nome: "Policloropreno (CR)", densidade: 1.23, familia: "Elastômero" },
    Material { nome: "Poliuretano", densidade: 1.2, familia: "Elastômero" },
];

/// Nomes curtos que as pessoas escrevem, apontando para a linha da tabela.
const APELIDOS: &[(&str, &str)] = &[
    ("borracha", "Borracha natural"),
    ("aço 4130", "Aço AISI 4130"),
    ("4130", "Aço AISI 4130"),
    ("cromo-molibdênio", "Aço AISI 4130"),
    ("aço 4140", "Aço AISI 4140"),
    ("nbr", "Borracha nitrílica (NBR)"),
];

/// Material da tabela pelo nome ou apelido (sem diferenciar maiúsculas).
pub fn material(nome: &str) -> Option<&'static Material> {
    let n = nome.trim().to_lowercase();
    let n = APELIDOS.iter().find(|(a, _)| *a == n).map_or(n, |(_, alvo)| alvo.to_lowercase());
    MATERIAIS.iter().find(|m| m.nome.to_lowercase() == n)
}

/// Aviso para um material que não está na tabela: a massa sai com a densidade padrão, e isso
/// precisa aparecer em vez de passar calado. `None` para nome vazio ou conhecido.
pub fn aviso_fora_da_tabela(nome: &str) -> Option<String> {
    (!nome.trim().is_empty() && material(nome).is_none()).then(|| format!("'{}' não está na tabela de materiais: a massa usa {DENSIDADE_PADRAO} g/cm³", nome.trim()))
}

/// Densidade em g/cm³; fora da tabela vale `DENSIDADE_PADRAO`.
pub fn densidade(nome: &str) -> f64 {
    material(nome).map_or(DENSIDADE_PADRAO, |m| m.densidade)
}

/// Material efetivo do corpo: o do coletor, ou o da peça quando o coletor não define o seu.
pub fn material_do_corpo(doc: &Document, coletor: Id) -> String {
    doc.collectors
        .iter()
        .find(|c| c.id == coletor)
        .map(|c| c.material.trim())
        .filter(|m| !m.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| doc.props.material.clone())
}

/// Massa em kg de um volume em mm³ do corpo `coletor` (B414): usa o material efetivo do corpo,
/// o mesmo de `fenkra-solidos::massa`, para a interface não divergir do cálculo.
pub fn massa_kg(doc: &Document, coletor: Id, volume_mm3: f64) -> f64 {
    volume_mm3 * densidade(&material_do_corpo(doc, coletor)) * 1e-6
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn massa_kg_usa_a_tabela_inteira_e_nao_a_lista_antiga() {
        let mut doc = Document::new("m");
        doc.props.material = "Alumínio 7075".into();
        // 1 dm³ de alumínio 7075 pesa 2,81 kg; a lista antiga de 8 itens cairia em 7,85
        let m = massa_kg(&doc, crate::doc::Id(0), 1e6);
        assert!((m - 2.81).abs() < 1e-9, "{m}");
    }

    #[test]
    fn nomes_sao_unicos_e_densidades_plausiveis() {
        for (i, m) in MATERIAIS.iter().enumerate() {
            assert!(m.densidade > 0.1 && m.densidade < 25., "{}", m.nome);
            assert!(MATERIAIS.iter().skip(i + 1).all(|o| o.nome != m.nome), "nome repetido: {}", m.nome);
        }
    }

    #[test]
    fn busca_ignora_caixa_e_cai_no_padrao() {
        assert_eq!(densidade("alumínio 6061"), 2.7);
        assert_eq!(densidade("material inventado"), DENSIDADE_PADRAO);
        assert!(material("Cobre").is_some());
    }

    #[test]
    fn cromo_molibdenio_e_borracha_estao_na_tabela() {
        assert_eq!(densidade("Aço AISI 4130"), 7.85);
        assert_eq!(densidade("borracha"), 0.93, "o apelido leva à borracha natural");
        assert_eq!(material("aço 4130").map(|m| m.nome), Some("Aço AISI 4130"));
        let mut doc = Document::new("m");
        doc.props.material = "Borracha".into();
        // 1 dm³ de borracha: 0,93 kg, e não 7,85 kg do aço padrão
        assert!((massa_kg(&doc, crate::doc::Id(0), 1e6) - 0.93).abs() < 1e-9);
        assert!(aviso_fora_da_tabela("Borracha").is_none());
        assert!(aviso_fora_da_tabela("").is_none());
        assert!(aviso_fora_da_tabela("Unobtânio").unwrap().contains("7.85"));
    }

    #[test]
    fn corpo_herda_material_da_peca_e_pode_sobrescrever() {
        let mut d = Document::new("p");
        let c = d.add_collector("Corpo");
        assert_eq!(material_do_corpo(&d, c), d.props.material);
        d.collectors.iter_mut().find(|x| x.id == c).unwrap().material = "Cobre".into();
        assert_eq!(material_do_corpo(&d, c), "Cobre");
    }
}
