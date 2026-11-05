// SPDX-License-Identifier: LGPL-2.1-or-later
//! Esquema aberto de árvore paramétrica (B266): a receita da peça, e não o corpo, em JSON documentado.
//!
//! O formato é uma tradução, não uma cópia do `.fkp`. O `.fkp` muda junto com o programa e usa ids
//! internos; este é pequeno, com nomes próprios e ids em texto, para que um script de fora consiga
//! escrevê-lo só com a especificação em `docs/formatos/arvore.md`. Referência a face e aresta vai por
//! ponto no espaço: a numeração de faces de outro programa não significa nada aqui, e a leitura
//! reancora pela geometria (ADR 0010).
//!
//! A versão segue a cadeia do B64 ([`fenkra_core::formato::Tipo::Arvore`]). O esquema publicado é o
//! mesmo que a leitura usa para validar, então os dois não têm como divergir.

mod esquema;
mod exportar;
mod importar;
mod ler;
mod montagem;
mod reancorar;

pub use esquema::{validar, ErroEsquema};
pub use exportar::{exportar_peca, Exportado, Fora};
pub use ler::{ler_peca, ler_texto, Remontada};
pub use montagem::{exportar_montagem, gravar_em_pasta, remontar, Conjunto};
pub use reancorar::{atualizar_pontos, reancorar, Reancoragem};

pub(crate) use importar::importar;

use serde::{Deserialize, Serialize};

/// Valor de `formato` no topo do arquivo.
pub const NOME_FORMATO: &str = "fenkra-arvore";
/// Esquema JSON da versão atual, o mesmo publicado na documentação.
pub const ESQUEMA: &str = include_str!("../../esquema/arvore-v1.schema.json");

fn falso(v: &bool) -> bool {
    !*v
}

fn zero() -> String {
    "0".into()
}

/// Arquivo inteiro: uma ou mais peças e, se houver, a estrutura de montagem.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Arquivo {
    pub formato: String,
    pub format_version: u32,
    /// quem gravou (texto livre)
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub gerador: String,
    pub unidades: String,
    pub pecas: Vec<Peca>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub montagem: Option<Montagem>,
    /// o que o gravador não conseguiu levar: a árvore do arquivo é incompleta
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fora: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Peca {
    pub id: String,
    pub nome: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parametros: Vec<Parametro>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub corpos: Vec<Corpo>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub referencias: Vec<Referencia>,
    pub arvore: Vec<Item>,
    /// quantos itens da árvore estão ativos (marcador da linha do tempo); ausente = todos
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ativos: Option<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Grandeza {
    Comprimento,
    Angulo,
    Numero,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Parametro {
    pub nome: String,
    pub expressao: String,
    pub grandeza: Grandeza,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub comentario: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Corpo {
    pub id: String,
    pub nome: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cor: Option<[u8; 3]>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanoOrigem {
    XY,
    XZ,
    YZ,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EixoOrigem {
    X,
    Y,
    Z,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Plano {
    Origem(PlanoOrigem),
    Referencia { referencia: String },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Eixo {
    Origem(EixoOrigem),
    Curva { esboco: String, curva: String },
    Referencia { referencia: String },
    Fixo { origem: [f64; 3], direcao: [f64; 3] },
    /// eixo da face cilíndrica ou cônica que passa por este ponto
    Face { face: [f64; 3] },
    /// aresta reta que passa por este ponto
    Aresta { aresta: [f64; 3] },
    Planos { planos: Box<[Plano; 2]> },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Ponto {
    Coordenadas([f64; 3]),
    Referencia { referencia: String },
    DeEsboco { esboco: String, ponto: String },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Referencia {
    pub id: String,
    pub nome: String,
    pub definicao: Definicao,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "tipo", rename_all = "snake_case")]
pub enum Definicao {
    PlanoDeslocado {
        base: Plano,
        deslocamento: String,
    },
    PlanoFixo {
        origem: [f64; 3],
        x: [f64; 3],
        normal: [f64; 3],
    },
    Ponto {
        x: String,
        y: String,
        z: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        sistema: Option<Plano>,
    },
    Reta {
        ponto: Ponto,
        direcao: Eixo,
        inicio: String,
        fim: String,
    },
}

/// Um passo da árvore, com o que todo comando tem em comum.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    pub nome: String,
    /// corpo de destino; ausente = o primeiro
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub corpo: Option<String>,
    #[serde(default, skip_serializing_if = "falso")]
    pub suprimido: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub suprimir_se: String,
    #[serde(default, skip_serializing_if = "falso")]
    pub oculto: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cor: Option<[u8; 3]>,
    pub comando: Comando,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Operacao {
    Unir,
    Cortar,
    Intersectar,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "tipo", rename_all = "snake_case")]
pub enum Termino {
    Cego { distancia: String },
    Simetrico { distancia: String },
    DoisLados { a: String, b: String },
    Atravessar,
    AtePlano { plano: Plano },
    AteProxima,
    AteUltima,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ParedeFina {
    pub espessura: String,
    /// 0 um lado, 1 o outro, 2 simétrica
    #[serde(default)]
    pub lado: u8,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "tipo", rename_all = "snake_case")]
pub enum FormaFuro {
    Simples,
    Rebaixado { diametro: String, profundidade: String },
    Escareado { diametro: String, angulo: String },
    Contrafurado { diametro: String, profundidade: String, angulo: String },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Forma {
    Caixa,
    Cilindro,
    Esfera,
    Cone,
    Toro,
    Cunha,
    Prisma,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Arestas {
    /// `"todas"` ou o id do comando que gerou as arestas, em [`Arestas::Geradas`]
    Todas(Todas),
    Geradas { geradas_por: String },
    /// um ponto sobre cada aresta; a leitura reancora em cada uma pela geometria
    Pontos(Vec<[f64; 3]>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Todas {
    Todas,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Padrao {
    pub forma: FormaPadrao,
    /// ids dos comandos repetidos; vazio = o corpo inteiro
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fontes: Vec<String>,
    /// posições puladas (i, j)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pular: Vec<[u32; 2]>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "tipo", rename_all = "snake_case")]
pub enum FormaPadrao {
    Linear {
        direcao: Eixo,
        quantidade: String,
        espacamento: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        direcao2: Option<Eixo>,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        quantidade2: String,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        espacamento2: String,
        /// o espaçamento é o comprimento total
        #[serde(default, skip_serializing_if = "falso")]
        total: bool,
    },
    Circular {
        eixo: Eixo,
        quantidade: String,
        angulo: String,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "tipo", rename_all = "snake_case")]
pub enum Comando {
    Esboco(Esboco),
    Extrusao {
        esboco: String,
        operacao: Operacao,
        termino: Termino,
        #[serde(default, skip_serializing_if = "falso")]
        inverter: bool,
        #[serde(default = "zero")]
        saida: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        fina: Option<ParedeFina>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        direcao: Option<Eixo>,
    },
    Revolucao {
        esboco: String,
        eixo: Eixo,
        angulo: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        angulo2: Option<String>,
        #[serde(default, skip_serializing_if = "falso")]
        simetrico: bool,
        operacao: Operacao,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        fina: Option<ParedeFina>,
    },
    Furo {
        esboco: String,
        forma: FormaFuro,
        diametro: String,
        termino: Termino,
        #[serde(default, skip_serializing_if = "falso")]
        inverter: bool,
        /// rosca cosmética, ex.: `M8x1.25`
        #[serde(default, skip_serializing_if = "Option::is_none")]
        rosca: Option<String>,
        /// ângulo da ponta da broca; ausente = fundo plano
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ponta: Option<String>,
    },
    Filete {
        raio: String,
        arestas: Arestas,
    },
    Chanfro {
        distancia: String,
        arestas: Arestas,
    },
    Casca {
        /// um ponto sobre cada face aberta
        faces: Vec<[f64; 3]>,
        espessura: String,
        #[serde(default, skip_serializing_if = "falso")]
        para_fora: bool,
        #[serde(default, skip_serializing_if = "falso")]
        fechada: bool,
    },
    Primitiva {
        forma: Forma,
        plano: Plano,
        a: String,
        b: String,
        c: String,
        d: String,
        operacao: Operacao,
    },
    Padrao(Padrao),
    Espelho {
        plano: Plano,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        fontes: Vec<String>,
    },
}

/// Esboço: plano, geometria e restrições. Pontos são compartilhados por id entre as curvas.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Esboco {
    pub plano: Plano,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub posicao: Option<Posicao>,
    pub pontos: Vec<PontoEsboco>,
    pub curvas: Vec<CurvaEsboco>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub restricoes: Vec<Restricao>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tangentes: Vec<Tangente>,
}

/// Origem e orientação próprias do esboço dentro do plano.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Posicao {
    pub origem: [f64; 2],
    #[serde(default, skip_serializing_if = "falso")]
    pub trocar: bool,
    #[serde(default, skip_serializing_if = "falso")]
    pub inverter_h: bool,
    #[serde(default, skip_serializing_if = "falso")]
    pub inverter_v: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PontoEsboco {
    pub id: String,
    pub x: f64,
    pub y: f64,
    #[serde(default, skip_serializing_if = "falso")]
    pub fixo: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CurvaEsboco {
    pub id: String,
    #[serde(default, skip_serializing_if = "falso")]
    pub construcao: bool,
    pub geometria: Geometria,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "tipo", rename_all = "snake_case")]
pub enum Geometria {
    Reta { a: String, b: String },
    Circulo { centro: String, raio: f64 },
    /// anti-horário, de `inicio` a `fim`, em torno de `centro`
    Arco { centro: String, inicio: String, fim: String },
    Ponto { ponto: String },
    Spline { pontos: Vec<String> },
    Elipse { centro: String, inicio: String, fim: String, maior: [f64; 2], menor: f64, t0: f64, t1: f64 },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Tangente {
    pub curva: String,
    pub ponto: String,
    pub direcao: [f64; 2],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TipoRestricao {
    Coincidente,
    Horizontal,
    Vertical,
    Paralela,
    Perpendicular,
    Igual,
    Tangente,
    Concentrica,
    PontoMedio,
    SobreCurva,
    Fixa,
    Simetrica,
    HorizontalPontos,
    VerticalPontos,
    Distancia,
    DistanciaX,
    DistanciaY,
    Comprimento,
    Raio,
    Diametro,
    Angulo,
    AnguloSuplementar,
    DistanciaPontoReta,
    AnguloReta,
}

/// Restrição geométrica ou cota. `entidades` traz os ids na ordem que a especificação dá para o tipo;
/// só as cotas têm `valor`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Restricao {
    pub tipo: TipoRestricao,
    pub entidades: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valor: Option<String>,
    /// cota de referência: mede, não comanda
    #[serde(default, skip_serializing_if = "falso")]
    pub referencia: bool,
    /// a expressão vale menos que zero e a geometria já está do outro lado
    #[serde(default, skip_serializing_if = "falso")]
    pub invertida: bool,
}

/// Estrutura de montagem: só a árvore e as posições. Restrições entre peças não entram na versão 1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Montagem {
    pub nome: String,
    pub componentes: Vec<Componente>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Componente {
    pub id: String,
    pub nome: String,
    /// peça deste arquivo; ausente = grupo
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub peca: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pai: Option<String>,
    pub origem: [f64; 3],
    /// matriz de rotação, linha a linha
    pub rotacao: [[f64; 3]; 3],
    #[serde(default, skip_serializing_if = "falso")]
    pub fixo: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cor: Option<[u8; 3]>,
}
