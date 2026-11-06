// SPDX-License-Identifier: LGPL-2.1-or-later
//! Quadro de tolerância geométrica (FCF) tipado: zona, modificadores com compatibilidade por
//! característica e posição, quadro composto, referências (datum/DRF) e o consultor por norma.
//!
//! As regras aqui são nossa leitura geral das normas de GD&T (a de produto geométrico ISO e a
//! americana de dimensionamento), escritas com palavras próprias e sem tabela copiada. Servem de
//! consultor: o erro bloqueia o que a norma não admite, o aviso aponta o que depende da norma
//! escolhida ou do contrato. Confira sempre na edição que o projeto adota.
//!
//! Os símbolos saem como traços vetoriais (`GdtChar::tracado`, `Modif::tracado`), não como glifos
//! de fonte: o desenho não depende de a fonte ter os caracteres.

use crate::pmi::{GdtChar, GdtExtra, Pmi, PmiKind};
use crate::Id;
use serde::{Deserialize, Serialize};

/// Norma de referência do quadro.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum Norma {
    /// produto geométrico (ISO 1101 e a de referências ISO 5459)
    #[default]
    Iso,
    /// dimensionamento e tolerância americano (edição de 2018)
    Asme,
}

impl Norma {
    pub const ALL: [Norma; 2] = [Norma::Iso, Norma::Asme];
    pub fn label(self) -> &'static str {
        match self {
            Norma::Iso => "ISO 1101",
            Norma::Asme => "ASME Y14.5",
        }
    }
}

/// Forma da zona de tolerância.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum Zona {
    /// entre dois planos ou duas linhas paralelas: o valor é a largura
    #[default]
    Largura,
    /// cilindro (valor é diâmetro): Ø
    Diametro,
    /// esfera (valor é diâmetro): SØ
    Esfera,
    /// largura repartida de modo desigual em torno do perfil nominal (precisa do deslocamento)
    NaoUniforme,
}

impl Zona {
    pub const ALL: [Zona; 4] = [Zona::Largura, Zona::Diametro, Zona::Esfera, Zona::NaoUniforme];
    pub fn label(self) -> &'static str {
        match self {
            Zona::Largura => "largura",
            Zona::Diametro => "cilíndrica Ø",
            Zona::Esfera => "esférica SØ",
            Zona::NaoUniforme => "não uniforme",
        }
    }
    /// Prefixo escrito antes do valor.
    pub fn prefixo(self) -> &'static str {
        match self {
            Zona::Diametro => "Ø",
            Zona::Esfera => "SØ",
            _ => "",
        }
    }
    /// Nome da forma da zona na troca de dados STEP (tolerance_zone_form).
    pub fn step_forma(self) -> &'static str {
        match self {
            Zona::Largura => "within a cylinder or between two parallel lines/planes",
            Zona::Diametro => "cylindrical or circular",
            Zona::Esfera => "spherical",
            Zona::NaoUniforme => "non uniform",
        }
    }
}

/// O elemento controlado é a superfície (ou linha) ou o eixo/plano mediano de um elemento de tamanho.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum Elemento {
    #[default]
    Superficie,
    Mediano,
}

impl Elemento {
    pub fn label(self) -> &'static str {
        match self {
            Elemento::Superficie => "superfície",
            Elemento::Mediano => "eixo ou plano mediano",
        }
    }
}

/// Onde o modificador aparece no quadro.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PosMod {
    /// na casa do valor, depois dele
    Valor,
    /// na casa de uma referência, depois da letra
    Datum,
    /// junto do quadro (acima, ao lado ou na chamada)
    Chamada,
}

/// Modificadores do quadro de tolerância geométrica. Não existe modificador de "independente do
/// tamanho": essa marca saiu das normas atuais (regra é independente por padrão) e foi retirada.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Modif {
    /// máximo material
    Mmc,
    /// mínimo material
    Lmc,
    /// zona de tolerância projetada (com altura)
    Projetada,
    /// estado livre
    EstadoLivre,
    /// plano tangente
    PlanoTangente,
    /// tolerância estatística
    Estatistica,
    /// zona comum (CZ)
    ZonaComum,
    /// qualquer seção transversal (ACS)
    SecaoQualquer,
    /// elemento de linha (LE)
    ElementoLinha,
    /// não convexa (NC)
    NaoConvexa,
    /// feições unidas (UF)
    FeicoesUnidas,
    /// perfil dinâmico
    PerfilDinamico,
    /// entre dois pontos (letras)
    Entre,
    /// todo o contorno
    TodoContorno,
    /// toda a superfície
    TodaSuperficie,
    /// feição contínua (CF)
    Continua,
}

impl Modif {
    pub const ALL: [Modif; 16] = [
        Modif::Mmc,
        Modif::Lmc,
        Modif::Projetada,
        Modif::EstadoLivre,
        Modif::PlanoTangente,
        Modif::Estatistica,
        Modif::ZonaComum,
        Modif::SecaoQualquer,
        Modif::ElementoLinha,
        Modif::NaoConvexa,
        Modif::FeicoesUnidas,
        Modif::PerfilDinamico,
        Modif::Entre,
        Modif::TodoContorno,
        Modif::TodaSuperficie,
        Modif::Continua,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Modif::Mmc => "máximo material",
            Modif::Lmc => "mínimo material",
            Modif::Projetada => "zona projetada",
            Modif::EstadoLivre => "estado livre",
            Modif::PlanoTangente => "plano tangente",
            Modif::Estatistica => "tolerância estatística",
            Modif::ZonaComum => "zona comum",
            Modif::SecaoQualquer => "qualquer seção transversal",
            Modif::ElementoLinha => "elemento de linha",
            Modif::NaoConvexa => "não convexa",
            Modif::FeicoesUnidas => "feições unidas",
            Modif::PerfilDinamico => "perfil dinâmico",
            Modif::Entre => "entre dois pontos",
            Modif::TodoContorno => "todo o contorno",
            Modif::TodaSuperficie => "toda a superfície",
            Modif::Continua => "feição contínua",
        }
    }
    /// Letra do símbolo circulado, se o modificador tem um (desenhado como traço).
    pub fn circulado(self) -> Option<char> {
        Some(match self {
            Modif::Mmc => 'M',
            Modif::Lmc => 'L',
            Modif::Projetada => 'P',
            Modif::EstadoLivre => 'F',
            Modif::PlanoTangente => 'T',
            Modif::PerfilDinamico => 'D',
            _ => return None,
        })
    }
    /// Sigla escrita (modificadores sem símbolo circulado).
    pub fn sigla(self) -> &'static str {
        match self {
            Modif::Estatistica => "ST",
            Modif::ZonaComum => "CZ",
            Modif::SecaoQualquer => "ACS",
            Modif::ElementoLinha => "LE",
            Modif::NaoConvexa => "NC",
            Modif::FeicoesUnidas => "UF",
            Modif::Continua => "CF",
            _ => "",
        }
    }
    /// Texto de uma linha, para listas e para a prévia em texto (o glifo circulado do Unicode).
    pub fn texto(self) -> String {
        match self.circulado() {
            Some('M') => "Ⓜ".into(),
            Some('L') => "Ⓛ".into(),
            Some('P') => "Ⓟ".into(),
            Some('F') => "Ⓕ".into(),
            Some('T') => "Ⓣ".into(),
            Some('D') => "Ⓓ".into(),
            _ => match self {
                Modif::Entre => "↔".into(),
                Modif::TodoContorno => "○".into(),
                Modif::TodaSuperficie => "◎".into(),
                m => m.sigla().into(),
            },
        }
    }
    /// Posição natural. Máximo e mínimo material valem na casa do valor e na da referência.
    pub fn posicao(self) -> PosMod {
        match self {
            Modif::Entre | Modif::TodoContorno | Modif::TodaSuperficie | Modif::Continua => PosMod::Chamada,
            _ => PosMod::Valor,
        }
    }
    /// Nome do modificador na troca de dados (geometric_tolerance_modifier / limit_condition).
    /// Vazio quando a troca não tem valor próprio (vai por outra entidade, como a zona projetada).
    pub fn step_nome(self) -> &'static str {
        match self {
            // na tolerância o nome é *_requirement (o *_condition é do limite de uma cota); é o que o
            // gravador do OpenCASCADE escreve no arquivo (conferido em B191)
            Modif::Mmc => "maximum_material_requirement",
            Modif::Lmc => "least_material_requirement",
            Modif::Projetada => "",
            Modif::EstadoLivre => "free_state",
            Modif::PlanoTangente => "tangent_plane",
            Modif::Estatistica => "statistical_tolerance",
            Modif::ZonaComum => "common_zone",
            Modif::SecaoQualquer => "any_cross_section",
            Modif::ElementoLinha => "line_element",
            Modif::NaoConvexa => "not_convex",
            Modif::FeicoesUnidas => "united_feature",
            Modif::PerfilDinamico => "dynamic_profile",
            Modif::Entre => "",
            Modif::TodoContorno => "all_around",
            Modif::TodaSuperficie => "all_over",
            Modif::Continua => "continuous_feature",
        }
    }
    /// Traços do símbolo circulado no quadro unitário (-0,5..0,5, y para cima); vazio se o
    /// modificador é uma sigla escrita.
    pub fn tracado(self) -> Vec<Traco> {
        let Some(l) = self.circulado() else { return vec![] };
        let mut v = vec![Traco::circulo([0., 0.], 0.5)];
        let aberto = |p: &[[f64; 2]]| Traco { pts: p.to_vec(), fechado: false, cheio: false };
        match l {
            'M' => v.push(aberto(&[[-0.25, -0.27], [-0.25, 0.27], [0., -0.05], [0.25, 0.27], [0.25, -0.27]])),
            'L' => v.push(aberto(&[[-0.18, 0.27], [-0.18, -0.27], [0.22, -0.27]])),
            'P' => v.push(aberto(&[[-0.2, -0.27], [-0.2, 0.27], [0.08, 0.27], [0.2, 0.18], [0.2, 0.06], [0.08, -0.03], [-0.2, -0.03]])),
            'F' => {
                v.push(aberto(&[[-0.18, -0.27], [-0.18, 0.27], [0.2, 0.27]]));
                v.push(aberto(&[[-0.18, 0.], [0.1, 0.]]));
            }
            'T' => {
                v.push(aberto(&[[-0.22, 0.27], [0.22, 0.27]]));
                v.push(aberto(&[[0., 0.27], [0., -0.27]]));
            }
            _ => v.push(Traco { pts: vec![[-0.2, -0.27], [-0.2, 0.27], [0.05, 0.27], [0.22, 0.12], [0.22, -0.12], [0.05, -0.27]], fechado: true, cheio: false }),
        }
        v
    }
}

/// Um traço vetorial: polilinha (aberta ou fechada) em coordenadas do quadro unitário.
#[derive(Clone, PartialEq, Debug)]
pub struct Traco {
    pub pts: Vec<[f64; 2]>,
    pub fechado: bool,
    pub cheio: bool,
}

impl Traco {
    fn linha(p: &[[f64; 2]]) -> Traco {
        Traco { pts: p.to_vec(), fechado: false, cheio: false }
    }
    fn circulo(c: [f64; 2], r: f64) -> Traco {
        Traco { pts: (0..32).map(|i| { let t = i as f64 / 32. * std::f64::consts::TAU; [c[0] + r * t.cos(), c[1] + r * t.sin()] }).collect(), fechado: true, cheio: false }
    }
    fn arco(c: [f64; 2], r: f64, a0: f64, a1: f64) -> Vec<[f64; 2]> {
        (0..=16).map(|i| { let t = (a0 + (a1 - a0) * i as f64 / 16.).to_radians(); [c[0] + r * t.cos(), c[1] + r * t.sin()] }).collect()
    }
    fn seta(de: [f64; 2], ate: [f64; 2]) -> Vec<Traco> {
        let (dx, dy) = (ate[0] - de[0], ate[1] - de[1]);
        let n = (dx * dx + dy * dy).sqrt().max(1e-9);
        let (ux, uy) = (dx / n, dy / n);
        let (b, w) = (0.22, 0.09);
        let base = [ate[0] - ux * b, ate[1] - uy * b];
        vec![Traco::linha(&[de, base]), Traco { pts: vec![ate, [base[0] - uy * w, base[1] + ux * w], [base[0] + uy * w, base[1] - ux * w]], fechado: true, cheio: true }]
    }
}

impl GdtChar {
    /// Símbolo da característica como traços vetoriais no quadro unitário (-0,5..0,5, y para cima).
    pub fn tracado(self) -> Vec<Traco> {
        let l = Traco::linha;
        match self {
            GdtChar::Retitude => vec![l(&[[-0.5, 0.], [0.5, 0.]])],
            GdtChar::Planeza => vec![Traco { pts: vec![[-0.5, -0.3], [0.3, -0.3], [0.5, 0.3], [-0.3, 0.3]], fechado: true, cheio: false }],
            GdtChar::Circularidade => vec![Traco::circulo([0., 0.], 0.4)],
            GdtChar::Cilindricidade => vec![Traco::circulo([0., 0.], 0.2), l(&[[-0.5, -0.4], [-0.1, 0.4]]), l(&[[0.1, -0.4], [0.5, 0.4]])],
            GdtChar::Paralelismo => vec![l(&[[-0.4, -0.4], [0., 0.4]]), l(&[[-0.1, -0.4], [0.3, 0.4]])],
            GdtChar::Perpendicularidade => vec![l(&[[-0.45, -0.4], [0.45, -0.4]]), l(&[[0., -0.4], [0., 0.45]])],
            GdtChar::Angularidade => vec![l(&[[0.2, 0.4], [-0.4, -0.4], [0.45, -0.4]])],
            GdtChar::Posicao => vec![Traco::circulo([0., 0.], 0.3), l(&[[-0.5, 0.], [0.5, 0.]]), l(&[[0., -0.5], [0., 0.5]])],
            GdtChar::Concentricidade => vec![Traco::circulo([0., 0.], 0.42), Traco::circulo([0., 0.], 0.2)],
            GdtChar::Simetria => vec![l(&[[-0.5, 0.], [0.5, 0.]]), l(&[[-0.3, 0.22], [0.3, 0.22]]), l(&[[-0.3, -0.22], [0.3, -0.22]])],
            GdtChar::PerfilLinha => vec![l(&Traco::arco([0., -0.15], 0.45, 180., 0.))],
            GdtChar::PerfilSuperficie => {
                let mut a = Traco::arco([0., -0.15], 0.45, 180., 0.);
                a.push([-0.45, -0.15]);
                vec![Traco { pts: a, fechado: true, cheio: false }]
            }
            GdtChar::BatimentoCircular => Traco::seta([-0.4, -0.4], [0.4, 0.4]),
            GdtChar::BatimentoTotal => {
                let mut v = Traco::seta([-0.45, -0.4], [0.05, 0.4]);
                v.extend(Traco::seta([-0.05, -0.4], [0.45, 0.4]));
                v.push(l(&[[-0.25, 0.], [0.25, 0.]]));
                v
            }
        }
    }
    /// Classe da característica.
    pub fn classe(self) -> ClasseGdt {
        use GdtChar::*;
        match self {
            Retitude | Planeza | Circularidade | Cilindricidade => ClasseGdt::Forma,
            Paralelismo | Perpendicularidade | Angularidade => ClasseGdt::Orientacao,
            Posicao | Concentricidade | Simetria => ClasseGdt::Posicao,
            PerfilLinha | PerfilSuperficie => ClasseGdt::Perfil,
            BatimentoCircular | BatimentoTotal => ClasseGdt::Batimento,
        }
    }
    /// Nome da característica na troca de dados (subtipos de geometric_tolerance).
    pub fn step_nome(self) -> &'static str {
        match self {
            GdtChar::Retitude => "straightness_tolerance",
            GdtChar::Planeza => "flatness_tolerance",
            GdtChar::Circularidade => "roundness_tolerance",
            GdtChar::Cilindricidade => "cylindricity_tolerance",
            GdtChar::Paralelismo => "parallelism_tolerance",
            GdtChar::Perpendicularidade => "perpendicularity_tolerance",
            GdtChar::Angularidade => "angularity_tolerance",
            GdtChar::Posicao => "position_tolerance",
            GdtChar::Concentricidade => "coaxiality_tolerance",
            GdtChar::Simetria => "symmetry_tolerance",
            GdtChar::PerfilLinha => "line_profile_tolerance",
            GdtChar::PerfilSuperficie => "surface_profile_tolerance",
            GdtChar::BatimentoCircular => "circular_runout_tolerance",
            GdtChar::BatimentoTotal => "total_runout_tolerance",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ClasseGdt {
    Forma,
    Orientacao,
    Posicao,
    Perfil,
    Batimento,
}

/// Resposta da matriz de compatibilidade.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Compat {
    Ok,
    /// vale, mas depende da norma ou pede atenção
    Aviso(&'static str),
    /// a norma não admite
    Nao(&'static str),
}

impl Compat {
    pub fn admite(self) -> bool {
        !matches!(self, Compat::Nao(_))
    }
}

/// Matriz de validade: o modificador `m` na posição `pos` de uma tolerância `ch` sobre `el`, na
/// norma `n`. Modificadores que só existem numa das normas devolvem `Aviso` na outra.
pub fn compat(m: Modif, ch: GdtChar, el: Elemento, pos: PosMod, n: Norma) -> Compat {
    use GdtChar::*;
    let c = ch.classe();
    let orient = c == ClasseGdt::Orientacao;
    let perfil = c == ClasseGdt::Perfil;
    let med = el == Elemento::Mediano;
    // só ISO ou só ASME
    let so_iso = |ok: bool, motivo: &'static str| if !ok { Compat::Nao(motivo) } else if n == Norma::Asme { Compat::Aviso("não faz parte da norma ASME: use a regra equivalente dela") } else { Compat::Ok };
    match m {
        Modif::Mmc | Modif::Lmc => match pos {
            PosMod::Datum => {
                if matches!(ch, Paralelismo | Perpendicularidade | Angularidade | Posicao | PerfilLinha | PerfilSuperficie | Retitude | Planeza) {
                    Compat::Ok
                } else {
                    Compat::Nao("referência com material só vale em orientação, posição e perfil")
                }
            }
            _ => {
                if !matches!(ch, Retitude | Planeza | Paralelismo | Perpendicularidade | Angularidade | Posicao) {
                    Compat::Nao("material máximo/mínimo não vale para esta característica")
                } else if !med {
                    Compat::Nao("material máximo/mínimo pede eixo ou plano mediano de um elemento de tamanho")
                } else {
                    Compat::Ok
                }
            }
        },
        Modif::Projetada => {
            if matches!(ch, Paralelismo | Perpendicularidade | Angularidade | Posicao) && med {
                Compat::Ok
            } else if !med {
                Compat::Nao("zona projetada pede eixo ou plano mediano")
            } else {
                Compat::Nao("zona projetada só vale em orientação e posição")
            }
        }
        Modif::EstadoLivre => {
            if c == ClasseGdt::Batimento { Compat::Nao("estado livre não vale em batimento") } else { Compat::Ok }
        }
        Modif::PlanoTangente => {
            if orient && !med { Compat::Ok } else if orient { Compat::Nao("plano tangente é da superfície, não do eixo") } else { Compat::Nao("plano tangente só vale em orientação") }
        }
        Modif::Estatistica => {
            if c == ClasseGdt::Batimento { Compat::Nao("tolerância estatística não vale em batimento") } else { Compat::Ok }
        }
        Modif::ZonaComum => {
            if matches!(ch, Circularidade | Cilindricidade | BatimentoCircular | BatimentoTotal | Concentricidade | Simetria) {
                Compat::Nao("zona comum não vale para esta característica")
            } else if n == Norma::Asme {
                Compat::Aviso("na ASME o equivalente é o requisito simultâneo (SIM REQT)")
            } else {
                Compat::Ok
            }
        }
        Modif::SecaoQualquer | Modif::ElementoLinha => so_iso(matches!(ch, Retitude | Paralelismo | Perpendicularidade | Angularidade | PerfilLinha | PerfilSuperficie), "só vale em retitude, orientação e perfil"),
        Modif::NaoConvexa => so_iso(matches!(ch, Retitude | Planeza | Circularidade | Cilindricidade), "não convexa só vale em tolerâncias de forma"),
        Modif::FeicoesUnidas => so_iso(!matches!(ch, Circularidade | Cilindricidade | BatimentoCircular | BatimentoTotal | Retitude | Planeza), "feições unidas só valem em posição, orientação e perfil"),
        Modif::PerfilDinamico => {
            if !perfil {
                Compat::Nao("perfil dinâmico só vale em perfil")
            } else if n == Norma::Iso {
                Compat::Aviso("perfil dinâmico é da ASME 2018; na ISO use a tolerância de forma e perfil separadas")
            } else {
                Compat::Ok
            }
        }
        Modif::Entre => {
            if matches!(ch, Circularidade | Cilindricidade) { Compat::Nao("'entre' não vale em circularidade e cilindricidade") } else { Compat::Ok }
        }
        Modif::TodoContorno | Modif::TodaSuperficie => {
            if perfil || orient || ch == Posicao { Compat::Ok } else { Compat::Nao("só vale em perfil, orientação e posição") }
        }
        Modif::Continua => {
            if matches!(ch, Circularidade | Cilindricidade | BatimentoCircular | BatimentoTotal) {
                Compat::Nao("feição contínua não vale para esta característica")
            } else if n == Norma::Iso {
                Compat::Aviso("feição contínua é da ASME; na ISO use 'zona comum' ou 'feições unidas'")
            } else {
                Compat::Ok
            }
        }
    }
}

/// Gravidade de um achado do consultor.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Gravidade {
    Info,
    Aviso,
    Erro,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Aviso {
    pub gravidade: Gravidade,
    /// mensagem curta, em português; a interface traduz
    pub msg: String,
}

fn av(g: Gravidade, m: impl Into<String>) -> Aviso {
    Aviso { gravidade: g, msg: m.into() }
}

/// Letras de referência: nunca I, O nem Q (parecem 1, 0 e O/zero); depois de Z vêm AA, BB…
pub const LETRAS: &str = "ABCDEFGHJKLMNPRSTUVWXYZ";

/// A letra pode ser nome de referência? Devolve o motivo da recusa.
pub fn letra_valida(s: &str) -> Result<(), &'static str> {
    let s = s.trim();
    if s.is_empty() {
        return Err("a referência precisa de uma letra");
    }
    if s.chars().count() > 2 {
        return Err("use uma letra, ou duas iguais depois de Z (AA, BB…)");
    }
    if !s.chars().all(|c| c.is_ascii_uppercase()) {
        return Err("use letras maiúsculas de A a Z");
    }
    if s.chars().any(|c| matches!(c, 'I' | 'O' | 'Q')) {
        return Err("I, O e Q não servem de referência: se confundem com 1, 0 e o círculo");
    }
    if s.chars().count() == 2 && s.chars().nth(0) != s.chars().nth(1) {
        return Err("depois de Z use letra dobrada: AA, BB, CC…");
    }
    Ok(())
}

/// Próxima letra livre, na ordem A, B, C… (pulando I, O, Q), depois AA, BB…
pub fn proxima_letra(usadas: &[String]) -> String {
    let usa = |l: &str| usadas.iter().any(|u| u.trim() == l);
    for c in LETRAS.chars() {
        let l = c.to_string();
        if !usa(&l) {
            return l;
        }
    }
    for c in LETRAS.chars() {
        let l = format!("{c}{c}");
        if !usa(&l) {
            return l;
        }
    }
    "A".into()
}

/// Separa a cadeia de referências do quadro ("A B C", "A|B", "A,B") em letras.
pub fn separar_datums(s: &str) -> Vec<String> {
    s.split(|c: char| c == '|' || c == ',' || c.is_whitespace()).filter(|x| !x.is_empty()).map(String::from).collect()
}

/// Valor numérico de uma tolerância escrita ("0,05" ou "0.05").
pub fn valor_num(s: &str) -> Option<f64> {
    s.trim().replace(',', ".").parse::<f64>().ok().filter(|v| v.is_finite())
}

/// Parte do texto de uma casa do quadro: texto simples ou um modificador (símbolo vetorial).
#[derive(Clone, PartialEq, Debug)]
pub enum Parte {
    Texto(String),
    Mod(Modif),
}

pub type Celula = Vec<Parte>;

/// Uma linha do quadro. `sym` é a característica da linha: `None` quando a linha continua o
/// quadro composto (a casa do símbolo vale para todas).
#[derive(Clone, PartialEq, Debug)]
pub struct FcfLinha {
    pub sym: Option<GdtChar>,
    pub celulas: Vec<Celula>,
}

/// Texto simples de uma célula (modificadores com o glifo circulado do Unicode).
pub fn celula_texto(c: &Celula) -> String {
    c.iter().map(|p| match p {
        Parte::Texto(t) => t.clone(),
        Parte::Mod(m) => m.texto(),
    }).collect::<Vec<_>>().join(" ").replace(" /", "/").replace("/ ", "/")
}

/// Visão resolvida do quadro: mistura os campos novos e o que os arquivos antigos guardavam
/// (`zone_dia`, `modifier`). Ⓢ não existe mais nas normas e fica de fora, listado em `ignorados`.
#[derive(Clone, PartialEq, Debug)]
pub struct Resolvido {
    pub zona: Zona,
    pub mods: Vec<Modif>,
    pub ignorados: Vec<String>,
}

impl GdtExtra {
    pub fn resolvido(&self) -> Resolvido {
        let mut zona = self.zona.unwrap_or(if self.zone_dia { Zona::Diametro } else { Zona::Largura });
        let mut mods = self.mods.clone();
        let mut ignorados = vec![];
        for c in self.modifier.chars().filter(|c| !c.is_whitespace()) {
            let m = match c {
                'Ⓜ' => Some(Modif::Mmc),
                'Ⓛ' => Some(Modif::Lmc),
                'Ⓟ' => Some(Modif::Projetada),
                'Ⓕ' => Some(Modif::EstadoLivre),
                'Ⓣ' => Some(Modif::PlanoTangente),
                'Ⓤ' => {
                    zona = Zona::NaoUniforme;
                    None
                }
                'Ⓢ' => {
                    ignorados.push("Ⓢ".to_string());
                    None
                }
                o => {
                    ignorados.push(o.to_string());
                    None
                }
            };
            if let Some(m) = m {
                if !mods.contains(&m) {
                    mods.push(m);
                }
            }
        }
        Resolvido { zona, mods, ignorados }
    }

    /// Passa o que veio de arquivos antigos para os campos tipados e limpa os antigos.
    pub fn migrar(&mut self) {
        if self.modifier.is_empty() && !self.zone_dia {
            return;
        }
        let r = self.resolvido();
        self.zona = Some(r.zona);
        self.mods = r.mods;
        self.modifier.clear();
        self.zone_dia = false;
    }

    /// Casas do quadro prontas para desenhar: valor (zona, número, por unidade, modificadores,
    /// máximo) e uma casa por referência, mais a linha do quadro composto ou de segmento único.
    pub fn linhas(&self, ch: GdtChar, tol: &str, datums: &str) -> Vec<FcfLinha> {
        let r = self.resolvido();
        let t = |s: &str| Parte::Texto(s.to_string());
        let mut cel_valor: Celula = vec![];
        let mut num = format!("{}{}", r.zona.prefixo(), tol.trim());
        if !self.por_unidade.trim().is_empty() {
            num += &format!("/{}", self.por_unidade.trim());
        }
        cel_valor.push(t(&num));
        for m in r.mods.iter().filter(|m| m.posicao() == PosMod::Valor) {
            cel_valor.push(Parte::Mod(*m));
            if *m == Modif::Projetada && !self.proj.trim().is_empty() {
                cel_valor.push(t(self.proj.trim()));
            }
        }
        if r.zona == Zona::NaoUniforme {
            cel_valor.push(t(&format!("UZ {}", self.uz.trim()).trim().to_string()));
        }
        if !self.max_valor.trim().is_empty() {
            cel_valor.push(t(&format!("MAX {}", self.max_valor.trim())));
        }
        let refs = |d: &str, mods: &[Vec<Modif>]| -> Vec<Celula> {
            separar_datums(d).iter().enumerate().map(|(i, l)| {
                let mut c: Celula = vec![t(l)];
                for m in mods.get(i).into_iter().flatten() {
                    c.push(Parte::Mod(*m));
                }
                c
            }).collect()
        };
        let mut v = vec![];
        let mut c1 = vec![cel_valor];
        c1.extend(refs(datums, &self.datum_mods));
        v.push(FcfLinha { sym: Some(ch), celulas: c1 });
        if let Some((t2, d2)) = &self.second {
            let mut c2: Vec<Celula> = vec![vec![t(&format!("{}{}", r.zona.prefixo(), t2.trim()))]];
            c2.extend(refs(d2, &[]));
            v.push(FcfLinha { sym: self.second_ch, celulas: c2 });
        }
        v
    }

    /// Texto que fica junto do quadro: entre dois pontos, todo o contorno, toda a superfície, CF.
    pub fn chamada(&self) -> String {
        let r = self.resolvido();
        let mut p: Vec<String> = vec![];
        for m in r.mods.iter().filter(|m| m.posicao() == PosMod::Chamada) {
            p.push(match m {
                Modif::Entre => format!("{} ↔ {}", self.entre.0.trim(), self.entre.1.trim()),
                Modif::TodoContorno => "○".to_string(),
                Modif::TodaSuperficie => "◎".to_string(),
                m => m.sigla().to_string(),
            });
        }
        p.join("  ")
    }

    /// Validação do quadro: o que a norma não admite (erro) e o que depende dela (aviso).
    /// `existentes`: letras de referência que o modelo define; `None` não confere a existência.
    pub fn validar(&self, ch: GdtChar, tol: &str, datums: &str, existentes: Option<&[String]>) -> Vec<Aviso> {
        let n = self.norma;
        let r = self.resolvido();
        let mut v: Vec<Aviso> = vec![];
        for i in &r.ignorados {
            v.push(av(Gravidade::Aviso, format!("o modificador {i} não existe nas normas atuais e foi ignorado")));
        }
        // valor
        match valor_num(tol) {
            None => v.push(av(Gravidade::Erro, "informe o valor da tolerância como número")),
            Some(x) if x <= 0. => v.push(av(Gravidade::Erro, "o valor da tolerância precisa ser maior que zero")),
            _ => {}
        }
        // características que a norma escolhida já não traz
        if n == Norma::Asme && matches!(ch, GdtChar::Concentricidade | GdtChar::Simetria) {
            v.push(av(Gravidade::Erro, "concentricidade e simetria saíram da ASME 2018: use posição ou perfil"));
        }
        // zona
        let med = self.elemento == Elemento::Mediano;
        match r.zona {
            Zona::Diametro | Zona::Esfera => {
                if matches!(ch.classe(), ClasseGdt::Forma | ClasseGdt::Batimento | ClasseGdt::Perfil) && ch != GdtChar::Retitude {
                    v.push(av(Gravidade::Erro, "zona Ø só vale em retitude, orientação e posição"));
                } else if !med && ch != GdtChar::Concentricidade {
                    v.push(av(Gravidade::Erro, "zona Ø pede eixo ou ponto: marque 'eixo ou plano mediano'"));
                }
                if r.zona == Zona::Esfera && ch != GdtChar::Posicao {
                    v.push(av(Gravidade::Erro, "zona esférica SØ só vale em posição"));
                }
            }
            Zona::NaoUniforme => {
                if !matches!(ch.classe(), ClasseGdt::Perfil) {
                    v.push(av(Gravidade::Erro, "zona não uniforme só vale em perfil"));
                } else if valor_num(&self.uz).is_none() {
                    v.push(av(Gravidade::Erro, "informe o deslocamento da zona não uniforme"));
                }
            }
            Zona::Largura => {
                if ch == GdtChar::Concentricidade && n == Norma::Iso {
                    v.push(av(Gravidade::Aviso, "concentricidade usa zona Ø"));
                }
            }
        }
        // referências
        let ds = separar_datums(datums);
        for (i, d) in ds.iter().enumerate() {
            if let Err(m) = letra_valida(d) {
                v.push(av(Gravidade::Erro, format!("referência {d}: {m}")));
            } else if ds[..i].contains(d) {
                v.push(av(Gravidade::Erro, format!("a referência {d} aparece duas vezes")));
            } else if let Some(ex) = existentes {
                if !ex.iter().any(|e| e == d) {
                    v.push(av(Gravidade::Erro, format!("a referência {d} não existe no modelo: crie-a primeiro")));
                }
            }
        }
        if ds.len() > 3 {
            v.push(av(Gravidade::Erro, "o quadro aceita no máximo três referências"));
        }
        match (ch.classe(), ds.len()) {
            (ClasseGdt::Forma, k) if k > 0 => v.push(av(Gravidade::Erro, "tolerância de forma não usa referência")),
            (ClasseGdt::Orientacao | ClasseGdt::Batimento, 0) => v.push(av(Gravidade::Erro, "esta característica precisa de ao menos uma referência")),
            (ClasseGdt::Posicao, 0) if ch != GdtChar::Posicao => v.push(av(Gravidade::Erro, "esta característica precisa de ao menos uma referência")),
            (ClasseGdt::Posicao, 0) => v.push(av(Gravidade::Aviso, "posição sem referência só localiza o grupo entre si")),
            _ => {}
        }
        // modificadores do valor e da chamada
        let mut vistos: Vec<Modif> = vec![];
        for m in &r.mods {
            if vistos.contains(m) {
                v.push(av(Gravidade::Erro, format!("modificador {} repetido", m.texto())));
                continue;
            }
            vistos.push(*m);
            match compat(*m, ch, self.elemento, m.posicao(), n) {
                Compat::Nao(msg) => v.push(av(Gravidade::Erro, format!("{} {}: {msg}", m.texto(), m.label()))),
                Compat::Aviso(msg) => v.push(av(Gravidade::Aviso, format!("{} {}: {msg}", m.texto(), m.label()))),
                Compat::Ok => {}
            }
            if *m == Modif::Projetada && valor_num(&self.proj).is_none_or(|x| x <= 0.) {
                v.push(av(Gravidade::Erro, "zona projetada precisa da altura (número maior que zero)"));
            }
            if *m == Modif::Entre && (letra_valida(&self.entre.0).is_err() || letra_valida(&self.entre.1).is_err() || self.entre.0.trim() == self.entre.1.trim()) {
                v.push(av(Gravidade::Erro, "'entre' precisa de duas letras de ponto diferentes"));
            }
        }
        if r.mods.contains(&Modif::Mmc) && r.mods.contains(&Modif::Lmc) {
            v.push(av(Gravidade::Erro, "máximo e mínimo material não coexistem no mesmo valor"));
        }
        for (i, ms) in self.datum_mods.iter().enumerate() {
            let Some(letra) = ds.get(i) else { continue };
            for m in ms {
                if !matches!(m, Modif::Mmc | Modif::Lmc) {
                    v.push(av(Gravidade::Erro, format!("referência {letra}: só máximo ou mínimo material")));
                } else if let Compat::Nao(msg) = compat(*m, ch, self.elemento, PosMod::Datum, n) {
                    v.push(av(Gravidade::Erro, format!("referência {letra}: {msg}")));
                }
            }
        }
        // por unidade e valor máximo
        if !self.por_unidade.trim().is_empty() && valor_num(&self.por_unidade).is_none_or(|x| x <= 0.) {
            v.push(av(Gravidade::Erro, "o comprimento da tolerância por unidade precisa ser um número maior que zero"));
        }
        if !self.max_valor.trim().is_empty() {
            match (valor_num(&self.max_valor), valor_num(tol)) {
                (None, _) => v.push(av(Gravidade::Erro, "o valor máximo precisa ser um número")),
                (Some(mx), Some(t)) if self.por_unidade.trim().is_empty() || mx <= t => {
                    if self.por_unidade.trim().is_empty() {
                        v.push(av(Gravidade::Aviso, "valor máximo só faz sentido com tolerância por unidade"));
                    } else {
                        v.push(av(Gravidade::Erro, "o valor máximo precisa ser maior que o valor por unidade"));
                    }
                }
                _ => {}
            }
        }
        // quadro composto (linha de baixo refina) ou de segmento único (cada linha com o seu símbolo)
        if let Some((t2, d2)) = &self.second {
            match (valor_num(t2), valor_num(tol)) {
                (None, _) => v.push(av(Gravidade::Erro, "a segunda linha precisa de um valor numérico")),
                (Some(b), Some(a)) if b >= a && self.second_ch.is_none() => v.push(av(Gravidade::Erro, "no quadro composto a linha de baixo é mais rígida: valor menor que o de cima")),
                _ => {}
            }
            if let Some(c2) = self.second_ch {
                if c2 == ch {
                    v.push(av(Gravidade::Info, "segmento único com a mesma característica: considere o quadro composto"));
                }
            } else if !matches!(ch, GdtChar::Posicao | GdtChar::PerfilSuperficie | GdtChar::PerfilLinha) {
                v.push(av(Gravidade::Aviso, "quadro composto é usado em posição e perfil"));
            }
            let d2s = separar_datums(d2);
            if self.second_ch.is_none() {
                // a linha de baixo só repete referências da de cima, na mesma ordem
                let mut it = ds.iter();
                if !d2s.iter().all(|d| it.any(|x| x == d)) {
                    v.push(av(Gravidade::Erro, "no quadro composto a linha de baixo só repete referências da de cima, na mesma ordem"));
                }
            }
        }
        v
    }
}

/// Achado do consultor sobre uma anotação do modelo.
#[derive(Clone, PartialEq, Debug)]
pub struct Achado {
    pub id: Id,
    pub aviso: Aviso,
}

/// Letras de referência definidas no modelo, na ordem da lista.
pub fn datums_do_modelo(pmi: &[Pmi]) -> Vec<String> {
    pmi.iter().filter_map(|p| if let PmiKind::Datum { letter } = &p.kind { Some(letter.trim().to_string()) } else { None }).collect()
}

/// Consultor por norma: confere todas as referências e quadros do modelo contra a norma escolhida
/// e devolve os achados, do mais grave ao mais leve. O quadro usa a norma que ele mesmo guarda;
/// `norma` vale como padrão para o que ficou no valor inicial do projeto.
pub fn consultar(pmi: &[Pmi], norma: Norma) -> Vec<Achado> {
    let letras = datums_do_modelo(pmi);
    let mut out: Vec<Achado> = vec![];
    let mut definidas: Vec<&str> = vec![];
    for p in pmi.iter() {
        match &p.kind {
            PmiKind::Datum { letter } => {
                if let Err(m) = letra_valida(letter) {
                    out.push(Achado { id: p.id, aviso: av(Gravidade::Erro, m) });
                } else if definidas.contains(&letter.trim()) {
                    out.push(Achado { id: p.id, aviso: av(Gravidade::Erro, format!("a referência {} está definida mais de uma vez", letter.trim())) });
                }
                definidas.push(letter.trim());
            }
            PmiKind::Gdt { ch, tol, datums, extra } => {
                let mut e = extra.clone();
                if e.norma == Norma::Iso && norma == Norma::Asme {
                    e.norma = norma;
                }
                for a in e.validar(*ch, tol, datums, Some(&letras)) {
                    out.push(Achado { id: p.id, aviso: a });
                }
            }
            _ => {}
        }
    }
    // referência definida e nunca usada
    let usadas: Vec<String> = pmi.iter().filter_map(|p| if let PmiKind::Gdt { datums, extra, .. } = &p.kind {
        let mut u = separar_datums(datums);
        if let Some((_, d2)) = &extra.second { u.extend(separar_datums(d2)); }
        Some(u)
    } else { None }).flatten().collect();
    for p in pmi {
        if let PmiKind::Datum { letter } = &p.kind {
            if !usadas.iter().any(|u| u == letter.trim()) {
                out.push(Achado { id: p.id, aviso: av(Gravidade::Info, format!("a referência {} não é usada por nenhum quadro", letter.trim())) });
            }
        }
    }
    out.sort_by(|a, b| b.aviso.gravidade.cmp(&a.aviso.gravidade));
    out
}

/// Ajusta o quadro ao trocar a característica: tira as referências de uma de forma e põe a
/// primeira existente numa que as exige, para o quadro nascer válido sem abrir painel.
pub fn ao_trocar_caracteristica(ch: GdtChar, datums: &mut String, existentes: &[String]) {
    match ch.classe() {
        ClasseGdt::Forma => datums.clear(),
        ClasseGdt::Orientacao | ClasseGdt::Batimento | ClasseGdt::Posicao if ch != GdtChar::Posicao => {
            if separar_datums(datums).is_empty() {
                if let Some(a) = existentes.first() {
                    *datums = a.clone();
                }
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pmi::PmiStyle;

    fn pmi(id: u64, kind: PmiKind) -> Pmi {
        Pmi { id: Id(id), name: "p".into(), kind, anchors: vec![], offset: [0.; 3], text: String::new(), tol_plus: String::new(), tol_minus: String::new(), visible: true, plane: None, style: PmiStyle::default(), folder: String::new(), targets: vec![] }
    }
    fn gdt(ch: GdtChar, tol: &str, datums: &str, extra: GdtExtra) -> PmiKind {
        PmiKind::Gdt { ch, tol: tol.into(), datums: datums.into(), extra }
    }
    fn erros(v: &[Aviso]) -> Vec<&str> {
        v.iter().filter(|a| a.gravidade == Gravidade::Erro).map(|a| a.msg.as_str()).collect()
    }

    #[test]
    fn letras_nunca_i_o_q() {
        let mut usadas: Vec<String> = vec![];
        let mut seq = String::new();
        for _ in 0..30 {
            let l = proxima_letra(&usadas);
            seq += &l;
            seq.push(' ');
            usadas.push(l);
        }
        assert!(seq.starts_with("A B C D E F G H J K L M N P R S T U V W X Y Z AA BB"), "{seq}");
        for l in ["I", "O", "Q", "AI", "a", "AB", "ABC", ""] {
            assert!(letra_valida(l).is_err(), "{l:?} devia ser recusada");
        }
        for l in ["A", "H", "J", "Z", "AA", "ZZ"] {
            assert!(letra_valida(l).is_ok(), "{l}");
        }
        // a recusa explica
        assert!(letra_valida("I").unwrap_err().contains("1, 0"));
        // A usada, a próxima é B; buraco em A é reaproveitado
        assert_eq!(proxima_letra(&["A".into()]), "B");
        assert_eq!(proxima_letra(&["B".into(), "C".into()]), "A");
        assert_eq!(proxima_letra(&["H".into(), "A".into(), "B".into(), "C".into(), "D".into(), "E".into(), "F".into(), "G".into()]), "J");
    }

    #[test]
    fn criar_planeza_e_perpendicularidade_com_a_sem_painel() {
        // modelo com a referência A; a perpendicularidade nasce com A ao trocar a característica
        let existentes = vec!["A".to_string()];
        let mut d = String::new();
        ao_trocar_caracteristica(GdtChar::Perpendicularidade, &mut d, &existentes);
        assert_eq!(d, "A");
        let plan = GdtExtra::default();
        assert!(erros(&plan.validar(GdtChar::Planeza, "0,05", "", Some(&existentes))).is_empty());
        let mut perp = GdtExtra::default();
        perp.elemento = Elemento::Superficie;
        assert!(erros(&perp.validar(GdtChar::Perpendicularidade, "0,1", &d, Some(&existentes))).is_empty());
        // planeza não leva referência: a troca de volta limpa
        ao_trocar_caracteristica(GdtChar::Planeza, &mut d, &existentes);
        assert!(d.is_empty());
        // a letra I é recusada com explicação
        let e = perp.validar(GdtChar::Perpendicularidade, "0,1", "I", None);
        assert!(erros(&e).iter().any(|m| m.contains("I, O e Q")), "{e:?}");
    }

    #[test]
    fn forma_nao_leva_referencia_e_orientacao_exige() {
        let x = GdtExtra::default();
        assert!(erros(&x.validar(GdtChar::Planeza, "0,05", "A", None)).iter().any(|m| m.contains("forma não usa referência")));
        assert!(erros(&x.validar(GdtChar::Paralelismo, "0,05", "", None)).iter().any(|m| m.contains("ao menos uma referência")));
        assert!(erros(&x.validar(GdtChar::Paralelismo, "0,05", "A", Some(&["B".to_string()]))).iter().any(|m| m.contains("não existe no modelo")));
        assert!(erros(&x.validar(GdtChar::Posicao, "0,1", "A A", None)).iter().any(|m| m.contains("duas vezes")));
        assert!(erros(&x.validar(GdtChar::Posicao, "0,1", "A B C D", None)).iter().any(|m| m.contains("três")));
        assert!(!erros(&x.validar(GdtChar::Planeza, "abc", "", None)).is_empty());
        assert!(!erros(&x.validar(GdtChar::Planeza, "0", "", None)).is_empty());
    }

    #[test]
    fn zona_diametro_pede_eixo() {
        let mut x = GdtExtra { zona: Some(Zona::Diametro), ..Default::default() };
        assert!(erros(&x.validar(GdtChar::Posicao, "0,1", "A", None)).iter().any(|m| m.contains("eixo")));
        x.elemento = Elemento::Mediano;
        assert!(erros(&x.validar(GdtChar::Posicao, "0,1", "A", None)).is_empty());
        assert!(!erros(&x.validar(GdtChar::Planeza, "0,1", "", None)).is_empty(), "planeza não tem zona Ø");
        x.zona = Some(Zona::Esfera);
        assert!(erros(&x.validar(GdtChar::Posicao, "0,1", "A", None)).is_empty());
        assert!(!erros(&x.validar(GdtChar::Paralelismo, "0,1", "A", None)).is_empty(), "SØ só em posição");
    }

    #[test]
    fn matriz_de_modificadores() {
        use GdtChar::*;
        // material máximo: só elemento de tamanho
        assert!(compat(Modif::Mmc, Posicao, Elemento::Mediano, PosMod::Valor, Norma::Iso).admite());
        assert!(!compat(Modif::Mmc, Posicao, Elemento::Superficie, PosMod::Valor, Norma::Iso).admite());
        assert!(!compat(Modif::Mmc, Circularidade, Elemento::Mediano, PosMod::Valor, Norma::Iso).admite());
        assert!(!compat(Modif::Mmc, BatimentoTotal, Elemento::Mediano, PosMod::Datum, Norma::Iso).admite());
        assert!(compat(Modif::Mmc, Posicao, Elemento::Mediano, PosMod::Datum, Norma::Asme).admite());
        // projetada: orientação e posição, mediano
        assert!(compat(Modif::Projetada, Perpendicularidade, Elemento::Mediano, PosMod::Valor, Norma::Iso).admite());
        assert!(!compat(Modif::Projetada, Planeza, Elemento::Mediano, PosMod::Valor, Norma::Iso).admite());
        // tangente: orientação sobre superfície
        assert!(compat(Modif::PlanoTangente, Paralelismo, Elemento::Superficie, PosMod::Valor, Norma::Iso).admite());
        assert!(!compat(Modif::PlanoTangente, Posicao, Elemento::Superficie, PosMod::Valor, Norma::Iso).admite());
        // dependentes da norma
        assert!(matches!(compat(Modif::SecaoQualquer, PerfilLinha, Elemento::Superficie, PosMod::Valor, Norma::Asme), Compat::Aviso(_)));
        assert_eq!(compat(Modif::SecaoQualquer, PerfilLinha, Elemento::Superficie, PosMod::Valor, Norma::Iso), Compat::Ok);
        assert!(matches!(compat(Modif::PerfilDinamico, PerfilSuperficie, Elemento::Superficie, PosMod::Valor, Norma::Iso), Compat::Aviso(_)));
        assert_eq!(compat(Modif::PerfilDinamico, PerfilSuperficie, Elemento::Superficie, PosMod::Valor, Norma::Asme), Compat::Ok);
        assert!(!compat(Modif::PerfilDinamico, Posicao, Elemento::Superficie, PosMod::Valor, Norma::Asme).admite());
        assert!(matches!(compat(Modif::Continua, PerfilSuperficie, Elemento::Superficie, PosMod::Chamada, Norma::Iso), Compat::Aviso(_)));
        // todo o contorno só em perfil/orientação/posição
        assert!(!compat(Modif::TodoContorno, Planeza, Elemento::Superficie, PosMod::Chamada, Norma::Iso).admite());
        assert!(compat(Modif::TodoContorno, PerfilSuperficie, Elemento::Superficie, PosMod::Chamada, Norma::Iso).admite());
        // todo modificador listado tem posição, nome STEP (quando existe) e rótulo
        for m in Modif::ALL {
            assert!(!m.label().is_empty());
            assert!(m.circulado().is_some() || !m.sigla().is_empty() || matches!(m, Modif::Entre | Modif::TodoContorno | Modif::TodaSuperficie));
        }
    }

    #[test]
    fn modificador_s_legado_sai_e_o_u_vira_zona() {
        let x = GdtExtra { modifier: "Ⓜ Ⓢ".into(), zone_dia: true, ..Default::default() };
        let r = x.resolvido();
        assert_eq!(r.mods, vec![Modif::Mmc]);
        assert_eq!(r.ignorados, vec!["Ⓢ".to_string()]);
        assert_eq!(r.zona, Zona::Diametro);
        let aviso = x.validar(GdtChar::Posicao, "0,1", "A", None);
        assert!(aviso.iter().any(|a| a.gravidade == Gravidade::Aviso && a.msg.contains("Ⓢ")));
        let mut y = x.clone();
        y.migrar();
        assert!(y.modifier.is_empty() && !y.zone_dia);
        assert_eq!(y.resolvido().zona, Zona::Diametro);
        assert_eq!(y.resolvido().mods, vec![Modif::Mmc]);
        let u = GdtExtra { modifier: "Ⓤ".into(), ..Default::default() };
        assert_eq!(u.resolvido().zona, Zona::NaoUniforme);
    }

    #[test]
    fn arquivo_antigo_continua_com_o_mesmo_texto() {
        // o quadro de antes (Ø0,1 Ⓜ | A | B | C) sai igual com os campos novos
        let x = GdtExtra { zone_dia: true, modifier: "Ⓜ".into(), ..Default::default() };
        let ls = x.linhas(GdtChar::Posicao, "0,1", "A B C");
        assert_eq!(ls.len(), 1);
        let t: Vec<String> = ls[0].celulas.iter().map(celula_texto).collect();
        assert_eq!(t, vec!["Ø0,1 Ⓜ", "A", "B", "C"]);
    }

    #[test]
    fn celulas_com_modificadores_e_extras() {
        let x = GdtExtra {
            zona: Some(Zona::Diametro),
            mods: vec![Modif::Projetada, Modif::EstadoLivre],
            proj: "12".into(),
            datum_mods: vec![vec![Modif::Mmc]],
            elemento: Elemento::Mediano,
            ..Default::default()
        };
        let ls = x.linhas(GdtChar::Perpendicularidade, "0,2", "A B");
        let t: Vec<String> = ls[0].celulas.iter().map(celula_texto).collect();
        assert_eq!(t, vec!["Ø0,2 Ⓟ 12 Ⓕ", "A Ⓜ", "B"]);
        assert!(erros(&x.validar(GdtChar::Perpendicularidade, "0,2", "A B", None)).is_empty());
        // projetada sem altura é recusada
        let sem = GdtExtra { proj: String::new(), ..x.clone() };
        assert!(erros(&sem.validar(GdtChar::Perpendicularidade, "0,2", "A B", None)).iter().any(|m| m.contains("altura")));
    }

    #[test]
    fn por_unidade_maximo_e_chamada() {
        let x = GdtExtra { por_unidade: "100".into(), max_valor: "0,3".into(), mods: vec![Modif::TodoContorno], ..Default::default() };
        let ls = x.linhas(GdtChar::PerfilSuperficie, "0,05", "A");
        assert_eq!(celula_texto(&ls[0].celulas[0]), "0,05/100 MAX 0,3");
        assert_eq!(x.chamada(), "○");
        assert!(erros(&x.validar(GdtChar::PerfilSuperficie, "0,05", "A", None)).is_empty());
        let ruim = GdtExtra { por_unidade: "100".into(), max_valor: "0,01".into(), ..Default::default() };
        assert!(erros(&ruim.validar(GdtChar::PerfilSuperficie, "0,05", "A", None)).iter().any(|m| m.contains("maior")));
        let entre = GdtExtra { mods: vec![Modif::Entre], entre: ("C".into(), "D".into()), ..Default::default() };
        assert_eq!(entre.chamada(), "C ↔ D");
        let igual = GdtExtra { mods: vec![Modif::Entre], entre: ("C".into(), "C".into()), ..Default::default() };
        assert!(!erros(&igual.validar(GdtChar::PerfilSuperficie, "0,05", "A", None)).is_empty());
    }

    #[test]
    fn quadro_composto_e_segmento_unico() {
        // composto: linha de cima 0,3, de baixo 0,1 repetindo só a primeira referência
        let c = GdtExtra { second: Some(("0,1".into(), "A".into())), ..Default::default() };
        assert!(erros(&c.validar(GdtChar::Posicao, "0,3", "A B", None)).is_empty());
        let ls = c.linhas(GdtChar::Posicao, "0,3", "A B");
        assert_eq!(ls.len(), 2);
        assert!(ls[1].sym.is_none(), "composto: uma casa de símbolo para as duas linhas");
        // linha de baixo mais folgada que a de cima: recusada
        let folgado = GdtExtra { second: Some(("0,5".into(), "A".into())), ..Default::default() };
        assert!(erros(&folgado.validar(GdtChar::Posicao, "0,3", "A B", None)).iter().any(|m| m.contains("mais rígida")));
        // referência fora de ordem ou nova na linha de baixo
        let fora = GdtExtra { second: Some(("0,1".into(), "B A".into())), ..Default::default() };
        assert!(!erros(&fora.validar(GdtChar::Posicao, "0,3", "A B", None)).is_empty());
        // segmento único: cada linha com o seu símbolo
        let s = GdtExtra { second: Some(("0,05".into(), "A".into())), second_ch: Some(GdtChar::Perpendicularidade), ..Default::default() };
        let ls = s.linhas(GdtChar::Posicao, "0,3", "A B");
        assert_eq!(ls[1].sym, Some(GdtChar::Perpendicularidade));
        assert!(erros(&s.validar(GdtChar::Posicao, "0,3", "A B", None)).is_empty());
    }

    #[test]
    fn norma_asme_sem_concentricidade_e_simetria() {
        let asme = GdtExtra { norma: Norma::Asme, zona: Some(Zona::Diametro), ..Default::default() };
        assert!(erros(&asme.validar(GdtChar::Concentricidade, "0,1", "A", None)).iter().any(|m| m.contains("ASME 2018")));
        let iso = GdtExtra { norma: Norma::Iso, zona: Some(Zona::Diametro), ..Default::default() };
        assert!(erros(&iso.validar(GdtChar::Concentricidade, "0,1", "A", None)).is_empty());
        assert!(erros(&GdtExtra { norma: Norma::Asme, ..Default::default() }.validar(GdtChar::Simetria, "0,1", "A", None)).iter().any(|m| m.contains("ASME 2018")));
    }

    #[test]
    fn consultor_do_modelo() {
        let pmis = vec![
            pmi(1, PmiKind::Datum { letter: "A".into() }),
            pmi(2, PmiKind::Datum { letter: "A".into() }),
            pmi(3, PmiKind::Datum { letter: "C".into() }),
            pmi(4, gdt(GdtChar::Perpendicularidade, "0,1", "A", GdtExtra::default())),
            pmi(5, gdt(GdtChar::Paralelismo, "0,1", "B", GdtExtra::default())),
            pmi(6, gdt(GdtChar::Planeza, "0,05", "", GdtExtra::default())),
        ];
        let a = consultar(&pmis, Norma::Iso);
        let de = |id: u64| a.iter().filter(|x| x.id == Id(id)).collect::<Vec<_>>();
        assert!(de(2).iter().any(|x| x.aviso.msg.contains("mais de uma vez")), "A repetida");
        assert!(de(1).iter().all(|x| x.aviso.gravidade != Gravidade::Erro), "a primeira definição está certa");
        assert!(de(3).iter().any(|x| x.aviso.gravidade == Gravidade::Info), "C nunca usada");
        assert!(de(4).is_empty(), "perpendicularidade com A existente está limpa");
        assert!(de(5).iter().any(|x| x.aviso.msg.contains("B não existe")));
        assert!(de(6).is_empty());
        // erros primeiro
        assert!(a.windows(2).all(|w| w[0].aviso.gravidade >= w[1].aviso.gravidade));
        // a norma do projeto vale para quadros que ficaram no padrão
        let conc = vec![pmi(1, PmiKind::Datum { letter: "A".into() }), pmi(2, gdt(GdtChar::Concentricidade, "0,1", "A", GdtExtra { zona: Some(Zona::Diametro), ..Default::default() }))];
        assert!(consultar(&conc, Norma::Iso).is_empty() || consultar(&conc, Norma::Iso).iter().all(|x| x.aviso.gravidade != Gravidade::Erro));
        assert!(consultar(&conc, Norma::Asme).iter().any(|x| x.aviso.gravidade == Gravidade::Erro));
    }

    #[test]
    fn simbolos_sao_tracos_dentro_do_quadro() {
        for c in GdtChar::ALL {
            let t = c.tracado();
            assert!(!t.is_empty(), "{c:?}");
            for tr in &t {
                assert!(tr.pts.len() >= 2);
                for p in &tr.pts {
                    assert!(p[0].abs() <= 0.5001 && p[1].abs() <= 0.5001, "{c:?} sai do quadro: {p:?}");
                }
            }
            assert!(!c.step_nome().is_empty());
        }
        for m in Modif::ALL {
            let t = m.tracado();
            assert_eq!(!t.is_empty(), m.circulado().is_some(), "{m:?}");
            for tr in &t {
                for p in &tr.pts {
                    assert!(p[0].abs() <= 0.5001 && p[1].abs() <= 0.5001, "{m:?}: {p:?}");
                }
            }
        }
        // posição tem círculo e cruz; planeza é um paralelogramo fechado
        assert_eq!(GdtChar::Posicao.tracado().len(), 3);
        assert!(GdtChar::Planeza.tracado()[0].fechado);
    }

    #[test]
    fn mapeamento_para_a_troca_de_dados() {
        assert_eq!(Zona::Diametro.step_forma(), "cylindrical or circular");
        assert_eq!(Zona::Esfera.step_forma(), "spherical");
        assert_eq!(Modif::Mmc.step_nome(), "maximum_material_requirement");
        assert_eq!(GdtChar::Posicao.step_nome(), "position_tolerance");
    }
}
