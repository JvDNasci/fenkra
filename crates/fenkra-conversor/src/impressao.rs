// SPDX-License-Identifier: LGPL-2.1-or-later
//! Preparo de impressão (B119): os corpos do documento na mesa, conferidos, e a saída em 3MF ou STL.
//!
//! O plano não mexe no documento: cada corpo ganha uma matriz com o lugar na mesa, e essa matriz vai no
//! item de construção do 3MF. A peça continua no lugar em que foi modelada. A bancada completa de
//! orientação e suportes é o B164; aqui ficam assentar numa face e o mapa de balanço mínimo.

use crate::malha::{Cena, Malha, No};
use crate::ErroConv;
use fenkra_core::{Document, Id};
use fenkra_malha::impressao::{arrumar, assentar, balanco, conferir, deslocar, medida_depois, Balanco, Conferencia};
use fenkra_malha::tmf::{self, OpcoesTmf};
use fenkra_malha::{miniatura, stl};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};

/// Lado da miniatura do pacote, em pixels.
const LADO_MINIATURA: u32 = 256;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct OpcoesImpressao {
    /// quanto a malha do corpo B-rep pode se afastar da superfície, em mm
    pub desvio: f64,
    /// ângulo máximo entre normais vizinhas, em graus
    pub angulo: f64,
    /// volume da impressora em mm
    pub mesa: [f64; 3],
    /// distância entre peças na mesa, em mm
    pub espaco: f64,
    /// maior ângulo com a vertical que imprime sem suporte, em graus
    pub balanco: f64,
    pub titulo: String,
    pub autor: String,
    pub descricao: String,
    pub miniatura: bool,
    pub cores: bool,
}

impl Default for OpcoesImpressao {
    fn default() -> Self {
        OpcoesImpressao { desvio: 0.02, angulo: 10., mesa: [220., 220., 250.], espaco: 5., balanco: 45., titulo: String::new(), autor: String::new(), descricao: String::new(), miniatura: true, cores: true }
    }
}

/// Um corpo no lugar de impressão.
#[derive(Clone, Debug, PartialEq)]
pub struct CorpoNaMesa {
    pub coletor: Id,
    pub nome: String,
    /// malha por cor, no lugar em que o corpo foi modelado
    pub partes: Vec<(Malha, [u8; 3])>,
    /// todas as partes numa malha só, soldada
    pub unida: Malha,
    /// matriz do modelo para a mesa
    pub posicao: [f64; 12],
    /// `false` quando a mesa não teve espaço para ele
    pub coube: bool,
    pub conferencia: Conferencia,
    pub balanco: Balanco,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PlanoImpressao {
    pub corpos: Vec<CorpoNaMesa>,
}

impl PlanoImpressao {
    pub fn pronto(&self) -> bool {
        !self.corpos.is_empty() && self.corpos.iter().all(|c| c.coube && c.conferencia.pronta())
    }
}

/// Monta o plano. `assentos` traz, por coletor, a normal da face que vai para a mesa.
pub fn planejar(doc: &Document, o: &OpcoesImpressao, assentos: &HashMap<Id, [f64; 3]>) -> Result<PlanoImpressao, ErroConv> {
    let corpos = fenkra_solidos::corpo_malha::corpos_coloridos(doc, o.desvio.max(1e-4), o.angulo.clamp(1., 60.).to_radians()).map_err(ErroConv::Gravacao)?;
    if corpos.is_empty() {
        return Err(ErroConv::Gravacao("o documento não tem corpo para imprimir".into()));
    }
    let mut plano = PlanoImpressao::default();
    for c in corpos {
        let unida = fenkra_malha::juntar(c.partes.iter().map(|(m, _)| m)).soldada();
        // sem face escolhida a peça fica de pé como foi modelada, com o fundo na mesa
        let posicao = assentar(&unida, assentos.get(&c.coletor).copied().unwrap_or([0., 0., -1.]));
        plano.corpos.push(CorpoNaMesa { coletor: c.coletor, nome: c.nome, partes: c.partes, unida, posicao, coube: false, conferencia: Conferencia::default(), balanco: Balanco::default() });
    }
    let medidas: Vec<[f64; 3]> = plano.corpos.iter().map(|c| medida_depois(&c.posicao, &c.unida)).collect();
    let caixas: Vec<[f64; 2]> = medidas.iter().map(|m| [m[0], m[1]]).collect();
    let lugares = arrumar(&caixas, [o.mesa[0], o.mesa[1]], o.espaco);
    // o grupo fica no meio da mesa, que é onde os fatiadores esperam a peça
    let fim = lugares.iter().zip(&caixas).filter_map(|(l, c)| l.map(|l| [l[0] + c[0], l[1] + c[1]])).fold([0f64; 2], |a, b| [a[0].max(b[0]), a[1].max(b[1])]);
    let sobra = [(o.mesa[0] - fim[0]) / 2., (o.mesa[1] - fim[1]) / 2.];
    for ((c, l), medida) in plano.corpos.iter_mut().zip(&lugares).zip(&medidas) {
        c.coube = l.is_some() && medida[2] <= o.mesa[2] + 1e-9;
        let lugar = l.map(|l| [l[0] + sobra[0], l[1] + sobra[1]]).unwrap_or([0., 0.]);
        c.posicao = deslocar(&c.posicao, &c.unida, lugar);
        let posta = c.unida.transformada(&c.posicao);
        c.conferencia = conferir(&posta, Some(o.mesa));
        c.balanco = balanco(&posta, o.balanco);
    }
    Ok(plano)
}

/// Cena do plano: um nó por corpo, com a matriz da mesa e uma malha por cor.
fn cena(plano: &PlanoImpressao, nome: &str, cores: bool) -> Cena {
    let mut s = Cena { nome: nome.into(), ..Default::default() };
    for c in plano.corpos.iter().filter(|c| c.coube) {
        let mut no = No::novo(&c.nome);
        no.local = c.posicao;
        for (parte, cor) in &c.partes {
            let mut m = parte.clone();
            m.nome.clone_from(&c.nome);
            if cores {
                let k = s.materiais.len() + 1;
                m.material = Some(s.material(&format!("Cor {k}"), [cor[0], cor[1], cor[2]].map(|v| f32::from(v) / 255.).into_iter().chain([1.]).collect::<Vec<_>>().try_into().unwrap_or([1.; 4])));
            }
            no.malhas.push(s.malhas.len());
            s.malhas.push(m);
        }
        s.nos.push(no);
        s.raizes.push(s.nos.len() - 1);
    }
    s
}

/// Grava o 3MF do plano, sempre em milímetros.
pub fn gravar_3mf(plano: &PlanoImpressao, o: &OpcoesImpressao, nome: &str, caminho: &Path) -> Result<(), ErroConv> {
    let titulo = if o.titulo.trim().is_empty() { nome } else { o.titulo.as_str() };
    let c = cena(plano, titulo, o.cores);
    if c.malhas.is_empty() {
        return Err(ErroConv::Gravacao("nenhum corpo coube na mesa".into()));
    }
    let mut metadados = vec![("Title".to_string(), titulo.to_string()), ("Application".to_string(), format!("Fenkra {}", env!("CARGO_PKG_VERSION")))];
    if !o.autor.trim().is_empty() {
        metadados.push(("Designer".into(), o.autor.clone()));
    }
    if !o.descricao.trim().is_empty() {
        metadados.push(("Description".into(), o.descricao.clone()));
    }
    let imagem = o.miniatura.then(|| miniatura::png(&miniatura::desenhar(&c, LADO_MINIATURA), LADO_MINIATURA));
    let bytes = tmf::gravar_com(&c, 1., "millimeter", &OpcoesTmf { metadados, miniatura: imagem })?;
    Ok(fenkra_malha::escrever(caminho, &bytes)?)
}

/// Grava o STL binário do plano: o formato não tem peças, então todos saem juntos, já na mesa.
pub fn gravar_stl(plano: &PlanoImpressao, nome: &str, caminho: &Path) -> Result<(), ErroConv> {
    let c = cena(plano, nome, false);
    if c.malhas.is_empty() {
        return Err(ErroConv::Gravacao("nenhum corpo coube na mesa".into()));
    }
    Ok(fenkra_malha::escrever(caminho, &stl::gravar(&c, true, 1.))?)
}

/// Pasta temporária do arquivo que vai para o fatiador.
pub fn arquivo_para_fatiador(nome: &str) -> PathBuf {
    let limpo: String = nome.chars().map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect();
    std::env::temp_dir().join("fenkra_impressao").join(format!("{limpo}.3mf"))
}

/// Abre o arquivo no fatiador externo. É outro processo, sem shell no meio e sem esperar por ele: o
/// Fenkra segue aberto, e nome de arquivo com espaço ou aspas vai como um argumento só.
pub fn abrir_no_fatiador(programa: &str, arquivo: &Path) -> Result<Child, ErroConv> {
    if programa.trim().is_empty() {
        return Err(ErroConv::Gravacao("escolha o programa do fatiador".into()));
    }
    Command::new(programa.trim()).arg(arquivo).spawn().map_err(|e| ErroConv::Gravacao(format!("não foi possível abrir '{}': {e}", programa.trim())))
}
