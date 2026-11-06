// SPDX-License-Identifier: LGPL-2.1-or-later
//! DXF 2D em Rust puro (B105): lê e grava sem kernel e sem processo filho.
//!
//! O DXF é uma lista de pares código e valor em texto. O OpenCASCADE não traduz esse formato, e o
//! leitor aqui não é C++: um arquivo ruim vira `Err`, não uma falha de memória. Por isso este crate
//! não depende de nada do Fenkra e serve tanto ao conversor quanto ao desenho e à planificada, que
//! estão abaixo do conversor na ordem de dependência.
//!
//! A gravação sai em R2000 (`AC1015`), a primeira versão que tem o que o desenho precisa em entidades
//! de verdade: polilinha leve, hachura e texto com várias linhas. O leitor aceita de R12 em diante.

mod arcos;
mod gravar;
mod ler;

pub use arcos::{reconhecer, TOLERANCIA_ARCO};
pub use gravar::gravar;
pub use ler::{e_binario, ler};

use std::collections::BTreeMap;

pub type P2 = [f64; 2];

/// Vértice de polilinha. `bojo` é a tangente de um quarto do ângulo do arco até o próximo vértice:
/// 0 é reta, 1 é meia-volta para a esquerda, negativo vira para a direita.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vertice {
    pub p: P2,
    pub bojo: f64,
}

/// Canto do texto preso ao ponto de inserção, na linha de baixo.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Anexo {
    #[default]
    Esquerda,
    Centro,
    Direita,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Entidade {
    Linha(P2, P2),
    /// anti-horário de `ini` a `fim`, em graus
    Arco { centro: P2, raio: f64, ini: f64, fim: f64 },
    Circulo { centro: P2, raio: f64 },
    Polilinha { vertices: Vec<Vertice>, fechada: bool },
    /// `maior`: do centro à ponta do semieixo maior; `razao`: menor sobre maior; parâmetros em radianos
    Elipse { centro: P2, maior: P2, razao: f64, t0: f64, t1: f64 },
    /// curva por pontos de ajuste (`ajuste`) ou por polígono de controle e nós
    Spline { grau: u32, controle: Vec<P2>, nos: Vec<f64>, pesos: Vec<f64>, ajuste: Vec<P2>, fechada: bool },
    Ponto(P2),
    /// texto em Unicode, com `\n` nas quebras; ângulo em graus
    Texto { pos: P2, altura: f64, angulo: f64, anexo: Anexo, texto: String },
    /// hachura cheia: um laço por contorno
    Hachura { lacos: Vec<Vec<P2>> },
}

/// Entidade com o que vem de fora dela. `cor` é o índice da paleta de 256; `None` segue a camada.
#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub camada: String,
    pub cor: Option<u16>,
    /// espessura em centésimos de milímetro; `None` segue a camada
    pub espessura: Option<i16>,
    pub entidade: Entidade,
}

impl Item {
    pub fn novo(camada: &str, entidade: Entidade) -> Item {
        Item { camada: camada.into(), cor: None, espessura: None, entidade }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Camada {
    pub nome: String,
    pub cor: u16,
    pub tipo_linha: String,
    /// espessura em centésimos de milímetro; `None` usa a padrão
    pub espessura: Option<i16>,
}

impl Camada {
    pub fn nova(nome: &str, cor: u16, tipo_linha: &str) -> Camada {
        Camada { nome: nome.into(), cor, tipo_linha: tipo_linha.into(), espessura: None }
    }
}

/// Tipo de linha: traços positivos, vazios negativos, em mm.
#[derive(Clone, Debug, PartialEq)]
pub struct TipoLinha {
    pub nome: String,
    pub padrao: Vec<f64>,
}

/// Unidade declarada em `$INSUNITS`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Unidade {
    /// o arquivo não declara: quem lê precisa perguntar
    #[default]
    SemUnidade,
    Polegada,
    Pe,
    Milha,
    Milimetro,
    Centimetro,
    Metro,
    Quilometro,
    Micropolegada,
    Milesimo,
    Jarda,
    Angstrom,
    Nanometro,
    Micrometro,
    Decimetro,
}

impl Unidade {
    const ORDEM: [Unidade; 15] = [
        Unidade::SemUnidade,
        Unidade::Polegada,
        Unidade::Pe,
        Unidade::Milha,
        Unidade::Milimetro,
        Unidade::Centimetro,
        Unidade::Metro,
        Unidade::Quilometro,
        Unidade::Micropolegada,
        Unidade::Milesimo,
        Unidade::Jarda,
        Unidade::Angstrom,
        Unidade::Nanometro,
        Unidade::Micrometro,
        Unidade::Decimetro,
    ];

    /// Pelo valor de `$INSUNITS`; códigos fora da lista valem como sem unidade.
    pub fn do_codigo(c: i32) -> Unidade {
        usize::try_from(c).ok().and_then(|i| Self::ORDEM.get(i)).copied().unwrap_or_default()
    }

    fn codigo(self) -> i32 {
        Self::ORDEM.iter().position(|u| *u == self).unwrap_or(0) as i32
    }

    /// Milímetros por unidade; `None` quando o arquivo não declara.
    pub fn em_mm(self) -> Option<f64> {
        Some(match self {
            Unidade::SemUnidade => return None,
            Unidade::Polegada => 25.4,
            Unidade::Pe => 304.8,
            Unidade::Milha => 1_609_344.,
            Unidade::Milimetro => 1.,
            Unidade::Centimetro => 10.,
            Unidade::Metro => 1000.,
            Unidade::Quilometro => 1e6,
            Unidade::Micropolegada => 2.54e-5,
            Unidade::Milesimo => 0.0254,
            Unidade::Jarda => 914.4,
            Unidade::Angstrom => 1e-7,
            Unidade::Nanometro => 1e-6,
            Unidade::Micrometro => 1e-3,
            Unidade::Decimetro => 100.,
        })
    }

    pub fn nome(self) -> &'static str {
        match self {
            Unidade::SemUnidade => "",
            Unidade::Polegada => "polegada",
            Unidade::Pe => "pé",
            Unidade::Milha => "milha",
            Unidade::Milimetro => "mm",
            Unidade::Centimetro => "cm",
            Unidade::Metro => "m",
            Unidade::Quilometro => "km",
            Unidade::Micropolegada => "micropolegada",
            Unidade::Milesimo => "milésimo de polegada",
            Unidade::Jarda => "jarda",
            Unidade::Angstrom => "angstrom",
            Unidade::Nanometro => "nm",
            Unidade::Micrometro => "µm",
            Unidade::Decimetro => "dm",
        }
    }
}

/// O conteúdo de um arquivo, com os blocos já explodidos.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Desenho {
    /// valor de `$ACADVER`; vazio em arquivo que não declara
    pub versao: String,
    pub unidade: Unidade,
    pub tipos_linha: Vec<TipoLinha>,
    pub camadas: Vec<Camada>,
    pub itens: Vec<Item>,
    /// o que o arquivo trazia e a leitura não aproveitou: tipo e quantidade
    pub ignorados: BTreeMap<String, usize>,
}

/// Índice da paleta de 256 mais perto da cor, entre os nove fixos. O R2000 não guarda cor verdadeira.
pub fn cor_indexada(rgb: [u8; 3]) -> u16 {
    const BASE: [(u16, [i32; 3]); 9] = [
        (1, [255, 0, 0]),
        (2, [255, 255, 0]),
        (3, [0, 255, 0]),
        (4, [0, 255, 255]),
        (5, [0, 0, 255]),
        (6, [255, 0, 255]),
        (7, [255, 255, 255]),
        (8, [128, 128, 128]),
        (9, [192, 192, 192]),
    ];
    let distancia = |c: &[i32; 3]| (0..3).map(|i| (c[i] - rgb[i] as i32).pow(2)).sum::<i32>();
    BASE.iter().min_by_key(|(_, c)| distancia(c)).map(|(i, _)| *i).unwrap_or(7)
}

/// Centro, raio e sentido do arco de `a` a `b` com o bojo dado. `None` em bojo nulo ou pontas iguais.
pub fn arco_do_bojo(a: P2, b: P2, bojo: f64) -> Option<(P2, f64, bool)> {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let corda = dx.hypot(dy);
    if bojo.abs() < 1e-12 || corda < 1e-12 {
        return None;
    }
    // o centro fica na mediatriz, à esquerda da corda quando o arco anda para a esquerda, a uma distância
    // de (1/bojo - bojo)/2 meias cordas do meio dela
    let k = (1. / bojo - bojo) / 2.;
    let centro = [(a[0] + b[0]) / 2. - k * dy / 2., (a[1] + b[1]) / 2. + k * dx / 2.];
    let raio = corda * (1. + bojo * bojo) / (4. * bojo.abs());
    Some((centro, raio, bojo > 0.))
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn unidade_pelo_codigo_e_em_mm() {
        assert_eq!(Unidade::do_codigo(1), Unidade::Polegada);
        assert_eq!(Unidade::do_codigo(4).em_mm(), Some(1.));
        assert_eq!(Unidade::do_codigo(6).em_mm(), Some(1000.));
        assert_eq!(Unidade::do_codigo(0).em_mm(), None);
        assert_eq!(Unidade::do_codigo(99), Unidade::SemUnidade);
        assert_eq!(Unidade::Polegada.codigo(), 1);
        assert_eq!(Unidade::Milimetro.codigo(), 4);
    }

    #[test]
    fn bojo_um_e_meia_volta_para_a_esquerda() {
        // de (0,0) a (10,0), meia-volta anti-horária passa por baixo: o centro é o meio da corda
        let (c, r, esquerda) = arco_do_bojo([0., 0.], [10., 0.], 1.).unwrap();
        assert!((c[0] - 5.).abs() < 1e-12 && c[1].abs() < 1e-12 && (r - 5.).abs() < 1e-12 && esquerda);
        // um quarto de círculo: bojo tan(22,5°) de (1,0) a (0,1), centro na origem
        let (c, r, _) = arco_do_bojo([1., 0.], [0., 1.], (std::f64::consts::PI / 8.).tan()).unwrap();
        assert!(c[0].abs() < 1e-12 && c[1].abs() < 1e-12 && (r - 1.).abs() < 1e-12, "{c:?} {r}");
        assert!(arco_do_bojo([1., 0.], [0., 1.], 0.).is_none());
    }

    #[test]
    fn cor_vai_para_o_indice_mais_perto() {
        assert_eq!(cor_indexada([250, 10, 10]), 1);
        assert_eq!(cor_indexada([10, 10, 240]), 5);
        assert_eq!(cor_indexada([130, 125, 128]), 8);
    }
}
