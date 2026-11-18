// SPDX-License-Identifier: LGPL-2.1-or-later
//! B512: modelo de folha com bloco de título montado por caixas.
//!
//! Em vez de desenhar o carimbo linha por linha, a pessoa parte de um retângulo e o divide: cada
//! divisão corta uma caixa em duas, na vertical ou na horizontal, a uma distância em mm. O bloco é,
//! portanto, uma árvore binária de cortes cujas folhas são as células. Isso dá de graça três coisas:
//! as linhas internas nunca ficam soltas ou sobrepostas, arrastar uma divisória mexe só nas caixas
//! daquele corte, e juntar duas caixas é trocar o corte por uma delas.
//!
//! Cada célula guarda um conteúdo: texto (fixo ou com campos `{chave}` ligados às propriedades da
//! peça e do desenho), logotipo (imagem), tabela de revisões ou o símbolo do diedro. O modelo de folha
//! junta formato, orientação, margens e o bloco; fica guardado no próprio desenho (as folhas apontam
//! para ele pelo `uid`, então editar o modelo atualiza todas as folhas que o usam) e pode ser gravado
//! num arquivo `.fkfolha` para reutilizar em outros desenhos.

use crate::{Format, V2};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Menor lado de uma caixa (mm): a divisória para antes disso.
pub const MENOR_CAIXA: f64 = 3.;
/// Extensão dos arquivos de modelo de folha.
pub const EXTENSAO: &str = "fkfolha";

/// Margens da moldura (mm). A esquerda é a de arquivamento (ISO 5457).
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct Margens {
    pub esquerda: f64,
    pub direita: f64,
    pub cima: f64,
    pub baixo: f64,
}

impl Default for Margens {
    fn default() -> Self {
        Margens { esquerda: 20., direita: 10., cima: 10., baixo: 10. }
    }
}

/// Alinhamento horizontal do texto na célula.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum Alinhamento {
    #[default]
    Esquerda,
    Centro,
    Direita,
}

impl Alinhamento {
    pub const TODOS: [Alinhamento; 3] = [Alinhamento::Esquerda, Alinhamento::Centro, Alinhamento::Direita];
    pub fn label(self) -> &'static str {
        match self {
            Alinhamento::Esquerda => "Esquerda",
            Alinhamento::Centro => "Centro",
            Alinhamento::Direita => "Direita",
        }
    }
}

/// O que a célula mostra.
#[derive(Clone, PartialEq, Debug, Default, Serialize, Deserialize)]
pub enum Conteudo {
    /// rótulo pequeno e texto (fixo ou com campos `{chave}`)
    #[default]
    Texto,
    /// logotipo: arquivo PNG ou JPEG (relativo à pasta do desenho ou absoluto)
    Imagem { caminho: String },
    /// tabela de revisões do desenho: as últimas `linhas` revisões, a mais nova embaixo
    Revisoes { linhas: u8 },
    /// símbolo do método de projeção (1º ou 3º diedro)
    Diedro,
}

impl Conteudo {
    /// Os tipos que o editor oferece (a imagem nasce sem arquivo, a tabela com três linhas).
    pub fn tipos() -> [Conteudo; 4] {
        [Conteudo::Texto, Conteudo::Imagem { caminho: String::new() }, Conteudo::Revisoes { linhas: 3 }, Conteudo::Diedro]
    }
    pub fn label(&self) -> &'static str {
        match self {
            Conteudo::Texto => "Texto",
            Conteudo::Imagem { .. } => "Logotipo (imagem)",
            Conteudo::Revisoes { .. } => "Tabela de revisões",
            Conteudo::Diedro => "Símbolo do diedro",
        }
    }
    pub fn mesmo_tipo(&self, o: &Conteudo) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(o)
    }
}

/// Célula do bloco.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Celula {
    /// texto pequeno no canto de cima (ex.: "MATERIAL"); vazio = sem rótulo
    #[serde(default)]
    pub rotulo: String,
    /// texto da célula; `{chave}` vira o valor da propriedade
    #[serde(default)]
    pub texto: String,
    /// altura do texto (mm); 0 = a do estilo do desenho
    #[serde(default)]
    pub altura: f64,
    #[serde(default)]
    pub negrito: bool,
    #[serde(default)]
    pub alinhar: Alinhamento,
    #[serde(default)]
    pub conteudo: Conteudo,
}

impl Default for Celula {
    fn default() -> Self {
        Celula { rotulo: String::new(), texto: String::new(), altura: 0., negrito: false, alinhar: Alinhamento::Esquerda, conteudo: Conteudo::Texto }
    }
}

impl Celula {
    pub fn texto(rotulo: &str, texto: &str) -> Self {
        Celula { rotulo: rotulo.into(), texto: texto.into(), ..Default::default() }
    }
}

/// Direção da linha que divide a caixa.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Corte {
    /// linha vertical: `a` fica à esquerda, `pos` conta da borda esquerda
    Vertical,
    /// linha horizontal: `a` fica em cima, `pos` conta da borda de cima
    Horizontal,
}

/// Nó da árvore do bloco.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum No {
    Celula(Celula),
    Divisao { corte: Corte, pos: f64, a: Box<No>, b: Box<No> },
}

/// Caminho de um nó a partir da raiz: 0 = `a`, 1 = `b`.
pub type Caminho = Vec<u8>;

/// Divisória (linha interna) do bloco, na folha.
#[derive(Clone, PartialEq, Debug)]
pub struct Divisoria {
    /// caminho do nó `Divisao`
    pub caminho: Caminho,
    pub corte: Corte,
    pub de: V2,
    pub ate: V2,
}

/// Retângulo (canto inferior esquerdo, canto superior direito).
pub type Caixa = (V2, V2);

fn dividir_caixa(c: Caixa, corte: Corte, pos: f64) -> (Caixa, Caixa) {
    let (lo, hi) = c;
    match corte {
        Corte::Vertical => {
            let p = (lo[0] + pos.clamp(0., hi[0] - lo[0])).min(hi[0]);
            ((lo, [p, hi[1]]), ([p, lo[1]], hi))
        }
        Corte::Horizontal => {
            let p = (hi[1] - pos.clamp(0., hi[1] - lo[1])).max(lo[1]);
            (([lo[0], p], hi), (lo, [hi[0], p]))
        }
    }
}

/// Bloco de título: tamanho e árvore de caixas. Fica no canto inferior direito da moldura.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct BlocoTitulo {
    pub largura: f64,
    pub altura: f64,
    pub raiz: No,
}

impl Default for BlocoTitulo {
    /// Bloco de partida: logotipo e título em cima; código, material, escala e revisão no meio;
    /// autor, data e folha embaixo. Mostra o que dá para fazer e já sai preenchido pela peça.
    fn default() -> Self {
        let c = |r: &str, t: &str| Box::new(No::Celula(Celula::texto(r, t)));
        let v = |pos: f64, a: Box<No>, b: Box<No>| Box::new(No::Divisao { corte: Corte::Vertical, pos, a, b });
        let h = |pos: f64, a: Box<No>, b: Box<No>| Box::new(No::Divisao { corte: Corte::Horizontal, pos, a, b });
        let titulo = Box::new(No::Celula(Celula { altura: 4.2, negrito: true, ..Celula::texto("TÍTULO", "{descricao}") }));
        let empresa = Box::new(No::Celula(Celula { altura: 4.2, negrito: true, alinhar: Alinhamento::Centro, ..Celula::texto("", "{empresa}") }));
        let topo = v(60., empresa, titulo);
        let meio = v(60., c("CÓDIGO", "{codigo}"), v(60., c("MATERIAL", "{material}"), v(30., c("ESCALA", "{escala}"), c("REVISÃO", "{revisao}"))));
        let baixo = v(60., c("DESENHO", "{desenhista}"), v(60., c("DATA", "{data}"), v(30., c("FORMATO", "{formato}"), c("FOLHA", "{folha}/{folhas}"))));
        BlocoTitulo { largura: 180., altura: 36., raiz: *h(12., topo, h(12., meio, baixo)) }
    }
}

impl BlocoTitulo {
    /// Bloco com uma caixa só.
    pub fn vazio(largura: f64, altura: f64) -> Self {
        BlocoTitulo { largura, altura, raiz: No::Celula(Celula::default()) }
    }

    pub fn no(&self, caminho: &[u8]) -> Option<&No> {
        let mut n = &self.raiz;
        for &k in caminho {
            n = match n {
                No::Divisao { a, b, .. } => {
                    if k == 0 {
                        a
                    } else {
                        b
                    }
                }
                No::Celula(_) => return None,
            };
        }
        Some(n)
    }

    pub fn no_mut(&mut self, caminho: &[u8]) -> Option<&mut No> {
        let mut n = &mut self.raiz;
        for &k in caminho {
            n = match n {
                No::Divisao { a, b, .. } => {
                    if k == 0 {
                        a
                    } else {
                        b
                    }
                }
                No::Celula(_) => return None,
            };
        }
        Some(n)
    }

    pub fn celula(&self, caminho: &[u8]) -> Option<&Celula> {
        match self.no(caminho)? {
            No::Celula(c) => Some(c),
            _ => None,
        }
    }

    pub fn celula_mut(&mut self, caminho: &[u8]) -> Option<&mut Celula> {
        match self.no_mut(caminho)? {
            No::Celula(c) => Some(c),
            _ => None,
        }
    }

    /// Caixa do bloco na folha, com o canto inferior esquerdo em `origem`.
    pub fn caixa(&self, origem: V2) -> Caixa {
        (origem, [origem[0] + self.largura, origem[1] + self.altura])
    }

    /// Caixa de um nó qualquer (célula ou divisão) na folha.
    pub fn caixa_do_no(&self, origem: V2, caminho: &[u8]) -> Option<Caixa> {
        let mut c = self.caixa(origem);
        let mut n = &self.raiz;
        for &k in caminho {
            match n {
                No::Divisao { corte, pos, a, b } => {
                    let (ca, cb) = dividir_caixa(c, *corte, *pos);
                    if k == 0 {
                        c = ca;
                        n = a;
                    } else {
                        c = cb;
                        n = b;
                    }
                }
                No::Celula(_) => return None,
            }
        }
        Some(c)
    }

    /// Todas as células, com o caminho e a caixa na folha, na ordem de leitura da árvore.
    pub fn celulas(&self, origem: V2) -> Vec<(Caminho, Caixa, &Celula)> {
        fn rec<'a>(n: &'a No, c: Caixa, cam: &mut Caminho, out: &mut Vec<(Caminho, Caixa, &'a Celula)>) {
            match n {
                No::Celula(cel) => out.push((cam.clone(), c, cel)),
                No::Divisao { corte, pos, a, b } => {
                    let (ca, cb) = dividir_caixa(c, *corte, *pos);
                    cam.push(0);
                    rec(a, ca, cam, out);
                    cam.pop();
                    cam.push(1);
                    rec(b, cb, cam, out);
                    cam.pop();
                }
            }
        }
        let mut out = vec![];
        rec(&self.raiz, self.caixa(origem), &mut vec![], &mut out);
        out
    }

    /// Linhas internas (uma por divisão), na folha.
    pub fn divisorias(&self, origem: V2) -> Vec<Divisoria> {
        fn rec(n: &No, c: Caixa, cam: &mut Caminho, out: &mut Vec<Divisoria>) {
            if let No::Divisao { corte, pos, a, b } = n {
                let (ca, cb) = dividir_caixa(c, *corte, *pos);
                let (de, ate) = match corte {
                    Corte::Vertical => ([ca.1[0], c.0[1]], [ca.1[0], c.1[1]]),
                    Corte::Horizontal => ([c.0[0], ca.0[1]], [c.1[0], ca.0[1]]),
                };
                out.push(Divisoria { caminho: cam.clone(), corte: *corte, de, ate });
                cam.push(0);
                rec(a, ca, cam, out);
                cam.pop();
                cam.push(1);
                rec(b, cb, cam, out);
                cam.pop();
            }
        }
        let mut out = vec![];
        rec(&self.raiz, self.caixa(origem), &mut vec![], &mut out);
        out
    }

    /// Célula sob o ponto da folha.
    pub fn celula_em(&self, origem: V2, p: V2) -> Option<Caminho> {
        self.celulas(origem).into_iter().find(|(_, (lo, hi), _)| p[0] >= lo[0] && p[0] <= hi[0] && p[1] >= lo[1] && p[1] <= hi[1]).map(|x| x.0)
    }

    /// Divide a célula em duas, ao meio. A metade `a` (esquerda ou de cima) fica com o conteúdo; a
    /// outra nasce vazia com o mesmo estilo de texto. Devolve o caminho da célula nova.
    pub fn dividir(&mut self, origem: V2, caminho: &[u8], corte: Corte) -> Option<Caminho> {
        let (lo, hi) = self.caixa_do_no(origem, caminho)?;
        let lado = match corte {
            Corte::Vertical => hi[0] - lo[0],
            Corte::Horizontal => hi[1] - lo[1],
        };
        if lado < 2. * MENOR_CAIXA {
            return None;
        }
        let n = self.no_mut(caminho)?;
        let No::Celula(c) = n else { return None };
        let nova = Celula { altura: c.altura, negrito: c.negrito, alinhar: c.alinhar, ..Default::default() };
        let velha = std::mem::take(c);
        *n = No::Divisao { corte, pos: (lado / 2.).round(), a: Box::new(No::Celula(velha)), b: Box::new(No::Celula(nova)) };
        let mut out = caminho.to_vec();
        out.push(1);
        Some(out)
    }

    /// Junta a célula com a vizinha do mesmo corte: o corte some e a caixa toda fica com esta célula.
    /// Devolve o caminho da célula que ficou.
    pub fn juntar(&mut self, caminho: &[u8]) -> Option<Caminho> {
        let (&lado, pai) = caminho.split_last()?;
        let n = self.no_mut(pai)?;
        let No::Divisao { a, b, .. } = n else { return None };
        let fica = if lado == 0 { std::mem::replace(a.as_mut(), No::Celula(Celula::default())) } else { std::mem::replace(b.as_mut(), No::Celula(Celula::default())) };
        *n = fica;
        Some(pai.to_vec())
    }

    /// Posição da divisória (mm a partir da borda esquerda ou de cima da caixa dividida).
    pub fn pos_da_divisoria(&self, caminho: &[u8]) -> Option<f64> {
        match self.no(caminho)? {
            No::Divisao { pos, .. } => Some(*pos),
            _ => None,
        }
    }

    /// Põe a divisória a `pos` mm da borda (esquerda ou de cima), sem deixar caixa menor que
    /// [`MENOR_CAIXA`] dos dois lados. Devolve a posição que ficou.
    pub fn por_divisoria(&mut self, origem: V2, caminho: &[u8], pos: f64) -> Option<f64> {
        let (lo, hi) = self.caixa_do_no(origem, caminho)?;
        let No::Divisao { corte, pos: p, .. } = self.no_mut(caminho)? else { return None };
        let lado = match corte {
            Corte::Vertical => hi[0] - lo[0],
            Corte::Horizontal => hi[1] - lo[1],
        };
        let max = (lado - MENOR_CAIXA).max(MENOR_CAIXA);
        *p = pos.clamp(MENOR_CAIXA, max);
        Some(*p)
    }

    /// Arraste da divisória até o ponto `ponto` da folha (com passo de 0,5 mm).
    pub fn arrastar_divisoria(&mut self, origem: V2, caminho: &[u8], ponto: V2) -> Option<f64> {
        let (lo, hi) = self.caixa_do_no(origem, caminho)?;
        let No::Divisao { corte, .. } = self.no(caminho)? else { return None };
        let pos = match corte {
            Corte::Vertical => ponto[0] - lo[0],
            Corte::Horizontal => hi[1] - ponto[1],
        };
        self.por_divisoria(origem, caminho, (pos * 2.).round() / 2.)
    }

    /// Campos `{chave}` usados nas células de texto, sem repetição, na ordem em que aparecem.
    pub fn campos(&self) -> Vec<String> {
        let mut out: Vec<String> = vec![];
        for (_, _, c) in self.celulas([0., 0.]) {
            for k in chaves(&c.texto) {
                if !out.contains(&k) {
                    out.push(k);
                }
            }
        }
        out
    }
}

/// Chaves `{x}` de um texto.
pub fn chaves(texto: &str) -> Vec<String> {
    let mut out = vec![];
    let mut resto = texto;
    while let Some(i) = resto.find('{') {
        let depois = &resto[i + 1..];
        let Some(j) = depois.find('}') else { break };
        let k = &depois[..j];
        if !k.is_empty() && !k.contains('{') {
            out.push(k.to_string());
        }
        resto = &depois[j + 1..];
    }
    out
}

/// Modelo de folha: formato, orientação, zoneamento, margens e bloco de título.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct ModeloFolha {
    /// identidade: as folhas apontam para o modelo por ela (o nome pode mudar)
    pub uid: String,
    pub nome: String,
    pub formato: Format,
    #[serde(default)]
    pub retrato: bool,
    #[serde(default = "sim")]
    pub zonas: bool,
    #[serde(default)]
    pub margens: Margens,
    pub bloco: BlocoTitulo,
}

fn sim() -> bool {
    true
}

/// Arquivo `.fkfolha`: o modelo e a versão do formato.
#[derive(Serialize, Deserialize)]
struct Arquivo {
    fenkra_modelo_folha: u32,
    modelo: ModeloFolha,
}

impl ModeloFolha {
    pub fn novo(nome: &str, formato: Format, retrato: bool) -> Self {
        ModeloFolha { uid: fenkra_core::formato::novo_uid(), nome: nome.into(), formato, retrato, zonas: true, margens: Margens::default(), bloco: BlocoTitulo::default() }
    }

    /// (largura, altura) da folha, mm.
    pub fn tamanho(&self) -> V2 {
        let s = self.formato.size();
        if self.retrato {
            [s[1], s[0]]
        } else {
            s
        }
    }

    /// Canto inferior esquerdo do bloco numa folha de tamanho `folha` (encostado na moldura, embaixo
    /// à direita).
    pub fn origem_do_bloco(&self, folha: V2) -> V2 {
        [folha[0] - self.margens.direita - self.bloco.largura, self.margens.baixo]
    }

    pub fn caixa_do_bloco(&self, folha: V2) -> Caixa {
        self.bloco.caixa(self.origem_do_bloco(folha))
    }

    /// Área dentro da moldura numa folha de tamanho `folha`.
    pub fn moldura(&self, folha: V2) -> Caixa {
        let m = self.margens;
        ([m.esquerda, m.baixo], [folha[0] - m.direita, folha[1] - m.cima])
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(&Arquivo { fenkra_modelo_folha: 1, modelo: self.clone() }).unwrap_or_default()
    }

    pub fn from_json(s: &str) -> Result<Self, String> {
        let a: Arquivo = serde_json::from_str(s).map_err(|e| e.to_string())?;
        if a.fenkra_modelo_folha > 1 {
            return Err(format!("modelo de folha gravado por uma versão mais nova ({})", a.fenkra_modelo_folha));
        }
        Ok(a.modelo)
    }

    pub fn gravar(&self, caminho: &Path) -> Result<(), String> {
        std::fs::write(caminho, self.to_json()).map_err(|e| format!("{}: {e}", caminho.display()))
    }

    pub fn ler(caminho: &Path) -> Result<Self, String> {
        let t = std::fs::read_to_string(caminho).map_err(|e| format!("{}: {e}", caminho.display()))?;
        Self::from_json(&t)
    }
}

/// Pasta da biblioteca de modelos de folha do usuário (`%APPDATA%/Fenkra/modelos_folha`, ou
/// `~/.config/fenkra/modelos_folha`).
pub fn pasta_biblioteca() -> Option<PathBuf> {
    if let Ok(a) = std::env::var("APPDATA") {
        return Some(PathBuf::from(a).join("Fenkra").join("modelos_folha"));
    }
    std::env::var("HOME").ok().map(|h| PathBuf::from(h).join(".config").join("fenkra").join("modelos_folha"))
}

/// Modelos gravados numa pasta, em ordem de nome. Arquivos que não abrem ficam de fora.
pub fn listar(pasta: &Path) -> Vec<(PathBuf, ModeloFolha)> {
    let Ok(rd) = std::fs::read_dir(pasta) else { return vec![] };
    let mut out: Vec<(PathBuf, ModeloFolha)> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case(EXTENSAO)))
        .filter_map(|p| ModeloFolha::ler(&p).ok().map(|m| (p, m)))
        .collect();
    out.sort_by(|a, b| a.1.nome.to_lowercase().cmp(&b.1.nome.to_lowercase()));
    out
}

/// Nome de arquivo seguro para o modelo.
pub fn nome_de_arquivo(nome: &str) -> String {
    let s: String = nome.chars().map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' { c } else { '_' }).collect();
    let s = s.trim();
    format!("{}.{EXTENSAO}", if s.is_empty() { "modelo" } else { s })
}

/// Linha da tabela de revisões do desenho.
#[derive(Clone, PartialEq, Debug, Default, Serialize, Deserialize)]
pub struct Revisao {
    pub rev: String,
    #[serde(default)]
    pub descricao: String,
    #[serde(default)]
    pub data: String,
    #[serde(default)]
    pub autor: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dividir_juntar_e_arrastar() {
        let mut b = BlocoTitulo::vazio(100., 20.);
        let o = [10., 10.];
        let nova = b.dividir(o, &[], Corte::Vertical).unwrap();
        assert_eq!(nova, vec![1]);
        assert_eq!(b.celulas(o).len(), 2);
        let (lo, hi) = b.caixa_do_no(o, &[1]).unwrap();
        assert_eq!((lo, hi), ([60., 10.], [110., 30.]));
        // a de cima da célula da direita
        b.dividir(o, &[1], Corte::Horizontal).unwrap();
        assert_eq!(b.celulas(o).len(), 3);
        assert_eq!(b.caixa_do_no(o, &[1, 0]).unwrap(), ([60., 20.], [110., 30.]));
        assert_eq!(b.divisorias(o).len(), 2);
        // arrastar a divisória vertical até x = 40 (30 mm da borda)
        assert_eq!(b.arrastar_divisoria(o, &[], [40.2, 0.]), Some(30.));
        assert_eq!(b.caixa_do_no(o, &[1, 1]).unwrap().0, [40., 10.]);
        // não passa do limite
        assert_eq!(b.por_divisoria(o, &[], 500.), Some(100. - MENOR_CAIXA));
        // a divisória foi para x = 107: à esquerda dela é a caixa 0, à direita a de baixo da direita
        assert_eq!(b.celula_em(o, [100., 12.]), Some(vec![0]));
        assert_eq!(b.celula_em(o, [109., 12.]), Some(vec![1, 1]));
        // juntar a de baixo com a de cima: volta a duas células
        assert_eq!(b.juntar(&[1, 1]), Some(vec![1]));
        assert_eq!(b.celulas(o).len(), 2);
        // caixa pequena não divide
        let mut p = BlocoTitulo::vazio(5., 5.);
        assert!(p.dividir(o, &[], Corte::Vertical).is_none());
    }

    #[test]
    fn campos_e_arquivo() {
        assert_eq!(chaves("{a} x {b}{a} {"), vec!["a", "b", "a"]);
        let m = ModeloFolha::novo("Empresa", Format::A3, false);
        let campos = m.bloco.campos();
        assert!(campos.contains(&"codigo".to_string()) && campos.contains(&"material".to_string()), "{campos:?}");
        let volta = ModeloFolha::from_json(&m.to_json()).unwrap();
        assert_eq!(volta, m);
        assert_eq!(nome_de_arquivo("A3/Cliente"), "A3_Cliente.fkfolha");
    }
}
