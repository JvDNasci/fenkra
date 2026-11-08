// SPDX-License-Identifier: LGPL-2.1-or-later
//! PMI semântico no documento XCAF (B191): o que o OCCT 8.0.1 sabe gravar e ler num STEP AP242.
//!
//! Os discriminantes seguem os enums `XCAFDimTolObjects_*` do OCCT instalado, na ordem exata dos
//! cabeçalhos: a ponte passa o número direto, então mudar a ordem aqui muda o tipo gravado. O teste
//! `discriminantes_seguem_o_occt` fixa os valores conferidos.

/// Os 31 tipos de dimensão do XCAF (mais `Nenhum`: 32 valores), na ordem de `XCAFDimTolObjects_DimensionType`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum TipoDimensaoXde {
    Nenhum = 0,
    LocalDistanciaCurva,
    LocalDistanciaLinear,
    LocalLinearCentroExterno,
    LocalLinearCentroInterno,
    LocalLinearExternoCentro,
    LocalLinearExternoExterno,
    LocalLinearExternoInterno,
    LocalLinearInternoCentro,
    LocalLinearInternoExterno,
    LocalLinearInternoInterno,
    LocalAngular,
    LocalOrientada,
    LocalComCaminho,
    TamanhoComprimentoCurva,
    TamanhoDiametro,
    TamanhoDiametroEsferico,
    TamanhoRaio,
    TamanhoRaioEsferico,
    TamanhoToroDiametroMenor,
    TamanhoToroDiametroMaior,
    TamanhoToroRaioMenor,
    TamanhoToroRaioMaior,
    TamanhoToroDiametroMaiorAlto,
    TamanhoToroDiametroMaiorBaixo,
    TamanhoToroRaioMaiorAlto,
    TamanhoToroRaioMaiorBaixo,
    TamanhoEspessura,
    TamanhoAngular,
    TamanhoComCaminho,
    RotuloComum,
    Apresentacao,
}

impl TipoDimensaoXde {
    pub const TODOS: [TipoDimensaoXde; 32] = {
        use TipoDimensaoXde::*;
        [
            LocalDistanciaCurva, LocalDistanciaLinear, LocalLinearCentroExterno, LocalLinearCentroInterno, LocalLinearExternoCentro,
            LocalLinearExternoExterno, LocalLinearExternoInterno, LocalLinearInternoCentro, LocalLinearInternoExterno, LocalLinearInternoInterno,
            LocalAngular, LocalOrientada, LocalComCaminho, TamanhoComprimentoCurva, TamanhoDiametro, TamanhoDiametroEsferico, TamanhoRaio,
            TamanhoRaioEsferico, TamanhoToroDiametroMenor, TamanhoToroDiametroMaior, TamanhoToroRaioMenor, TamanhoToroRaioMaior,
            TamanhoToroDiametroMaiorAlto, TamanhoToroDiametroMaiorBaixo, TamanhoToroRaioMaiorAlto, TamanhoToroRaioMaiorBaixo, TamanhoEspessura,
            TamanhoAngular, TamanhoComCaminho, RotuloComum, Apresentacao, Nenhum,
        ]
    };

    pub fn de_numero(n: u32) -> Option<Self> {
        Self::TODOS.iter().copied().find(|t| *t as u32 == n)
    }

    /// Se o gravador do OCCT 8.0.1 escreve e o leitor devolve o tipo. Conferido um a um: `Orientada` volta
    /// sem tipo, `Apresentacao` não escreve nada, e os dois "com caminho" e o rótulo comum derrubam o
    /// processo (falha de segmento dentro do gravador) quando a anotação não traz a aresta de caminho.
    pub fn atravessa(self) -> bool {
        use TipoDimensaoXde::*;
        !matches!(self, Nenhum | LocalOrientada | LocalComCaminho | TamanhoComCaminho | RotuloComum | Apresentacao)
    }

    /// Ângulo: o gravador escreve o número cru numa unidade em radianos e o leitor devolve graus.
    pub fn e_angulo(self) -> bool {
        matches!(self, TipoDimensaoXde::LocalAngular | TipoDimensaoXde::TamanhoAngular)
    }

    /// Dimensão de tamanho (a característica do elemento) e não de localização.
    pub fn e_tamanho(self) -> bool {
        (self as u32) >= TipoDimensaoXde::TamanhoComprimentoCurva as u32 && (self as u32) <= TipoDimensaoXde::TamanhoComCaminho as u32
    }
}

/// Tipos de tolerância geométrica do XCAF, na ordem de `XCAFDimTolObjects_GeomToleranceType`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum TipoToleranciaXde {
    Nenhum = 0,
    Angularidade,
    BatimentoCircular,
    Circularidade,
    Coaxialidade,
    Concentricidade,
    Cilindricidade,
    Planeza,
    Paralelismo,
    Perpendicularidade,
    Posicao,
    PerfilLinha,
    PerfilSuperficie,
    Retitude,
    Simetria,
    BatimentoTotal,
}

impl TipoToleranciaXde {
    pub fn de_numero(n: u32) -> Option<Self> {
        use TipoToleranciaXde::*;
        [Nenhum, Angularidade, BatimentoCircular, Circularidade, Coaxialidade, Concentricidade, Cilindricidade, Planeza, Paralelismo, Perpendicularidade, Posicao, PerfilLinha, PerfilSuperficie, Retitude, Simetria, BatimentoTotal]
            .into_iter()
            .find(|t| *t as u32 == n)
    }
}

/// Forma da zona (`XCAFDimTolObjects_GeomToleranceTypeValue`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum ZonaXde {
    #[default]
    Largura = 0,
    Diametro = 1,
    Esfera = 2,
}

/// Condição de material (`XCAFDimTolObjects_GeomToleranceMatReqModif`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum MaterialXde {
    #[default]
    Nenhum = 0,
    Maximo = 1,
    Minimo = 2,
}

/// Modificador da zona (`XCAFDimTolObjects_GeomToleranceZoneModif`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum ModZonaXde {
    #[default]
    Nenhum = 0,
    Projetada = 1,
    Batimento = 2,
    NaoUniforme = 3,
}

/// Modificadores de tolerância que o XCAF tem (`XCAFDimTolObjects_GeomToleranceModif`). O XCAF não tem
/// perfil dinâmico, feição contínua nem feições unidas: esses ficam de fora e viram aviso na saída.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum ModToleranciaXde {
    SecaoQualquer = 0,
    ZonaComum,
    ElementoRadial,
    EstadoLivre,
    MaterialMinimo,
    ElementoLinha,
    DiametroMaior,
    MaterialMaximo,
    DiametroMenor,
    NaoConvexa,
    DiametroPasso,
    Reciprocidade,
    Separado,
    Estatistica,
    PlanoTangente,
    TodoContorno,
    TodaSuperficie,
}

impl ModToleranciaXde {
    pub fn de_numero(n: u32) -> Option<Self> {
        use ModToleranciaXde::*;
        [SecaoQualquer, ZonaComum, ElementoRadial, EstadoLivre, MaterialMinimo, ElementoLinha, DiametroMaior, MaterialMaximo, DiametroMenor, NaoConvexa, DiametroPasso, Reciprocidade, Separado, Estatistica, PlanoTangente, TodoContorno, TodaSuperficie]
            .into_iter()
            .find(|m| *m as u32 == n)
    }
}

/// Tipo de alvo de datum (`XCAFDimTolObjects_DatumTargetType`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum AlvoDatumXde {
    Ponto = 0,
    Linha,
    Retangulo,
    Circulo,
    Area,
}

impl AlvoDatumXde {
    pub fn de_numero(n: u32) -> Option<Self> {
        [AlvoDatumXde::Ponto, AlvoDatumXde::Linha, AlvoDatumXde::Retangulo, AlvoDatumXde::Circulo, AlvoDatumXde::Area].into_iter().find(|a| *a as u32 == n)
    }
}

/// Alvo de referência de um datum.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AlvoXde {
    pub tipo: AlvoDatumXde,
    pub numero: u32,
    pub comprimento: f64,
    pub largura: f64,
}

/// Uma anotação semântica ligada a faces de um produto. `faces` indexa na ordem de `TopExp::MapShapes`
/// do produto (a mesma de `Shape::face_index`); vazio = o corpo inteiro.
#[derive(Clone, Debug, PartialEq)]
pub struct AnotacaoXde {
    /// PMI gráfico: o desenho da anotação (linhas de chamada, quadro e texto em traços)
    pub apresentacao: Option<ApresentacaoXde>,
    pub produto: usize,
    pub nome: String,
    pub faces: Vec<u32>,
    pub dado: DadoXde,
}

/// PMI gráfico (B191): a anotação desenhada em segmentos de reta, no espaço do modelo, em mm. O
/// gravador a escreve como polilinhas (`tessellated_annotation_occurrence`) num `draughting_callout`
/// com `nome`; o texto vai em traços, e o mesmo texto vai no nome, para quem lê só os nomes.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ApresentacaoXde {
    pub nome: String,
    pub segmentos: Vec<[[f64; 3]; 2]>,
    /// plano da anotação: origem, normal e direção x
    pub plano: Option<([f64; 3], [f64; 3], [f64; 3])>,
    /// ponto de ancoragem do texto
    pub texto_em: Option<[f64; 3]>,
}

/// Vista salva (B191): câmera e as anotações que aparecem nela (índices em `ModeloXde::anotacoes`).
#[derive(Clone, Debug, PartialEq)]
pub struct VistaXde {
    pub nome: String,
    /// ponto de projeção (olho), direção de visada e direção para cima
    pub olho: [f64; 3],
    pub direcao: [f64; 3],
    pub cima: [f64; 3],
    pub ortogonal: bool,
    /// distância do olho ao plano de vista e tamanho da janela, em mm
    pub zoom: f64,
    pub largura: f64,
    pub altura: f64,
    pub anotacoes: Vec<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DadoXde {
    Dimensao {
        tipo: TipoDimensaoXde,
        valor: f64,
        /// (inferior, superior), em desvios com sinal em relação ao valor nominal
        tolerancia: Option<(f64, f64)>,
        /// segundo grupo de faces de uma dimensão entre dois elementos
        faces2: Vec<u32>,
    },
    Tolerancia {
        tipo: TipoToleranciaXde,
        valor: f64,
        zona: ZonaXde,
        material: MaterialXde,
        mod_zona: ModZonaXde,
        /// altura da zona projetada
        valor_zona: f64,
        modificadores: Vec<ModToleranciaXde>,
        /// índices em `ModeloXde::anotacoes` das anotações que são datum, na ordem de precedência
        datums: Vec<usize>,
    },
    Datum {
        letra: String,
        /// precedência: 1 primário, 2 secundário, 3 terciário; 0 sem ordem
        posicao: u32,
        alvo: Option<AlvoXde>,
    },
    /// só gráfico (rótulo comum sem faces): rugosidade, solda, nota, balão, legenda ou o que não tem
    /// semântica no STEP; exige `apresentacao`
    Grafico,
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn discriminantes_seguem_o_occt() {
        // valores contados nos cabeçalhos XCAFDimTolObjects_* do OCCT 8.0.1 instalado
        assert_eq!(TipoDimensaoXde::LocalDistanciaLinear as u32, 2);
        assert_eq!(TipoDimensaoXde::LocalAngular as u32, 11);
        assert_eq!(TipoDimensaoXde::TamanhoDiametro as u32, 15);
        assert_eq!(TipoDimensaoXde::TamanhoRaio as u32, 17);
        assert_eq!(TipoDimensaoXde::TamanhoEspessura as u32, 27);
        assert_eq!(TipoDimensaoXde::Apresentacao as u32, 31);
        assert_eq!(TipoDimensaoXde::TODOS.len(), 32);
        assert_eq!(TipoToleranciaXde::Planeza as u32, 7);
        assert_eq!(TipoToleranciaXde::Posicao as u32, 10);
        assert_eq!(TipoToleranciaXde::BatimentoTotal as u32, 15);
        assert_eq!(ModToleranciaXde::MaterialMaximo as u32, 7);
        assert_eq!(ModToleranciaXde::TodaSuperficie as u32, 16);
        for t in TipoDimensaoXde::TODOS {
            assert_eq!(TipoDimensaoXde::de_numero(t as u32), Some(t));
        }
    }
}
