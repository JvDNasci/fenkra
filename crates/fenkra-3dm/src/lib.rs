// SPDX-License-Identifier: LGPL-2.1-or-later
//! Leitura e gravação de 3DM pelo openNURBS (B211, ADR 0090).
//!
//! Este crate não conhece o OpenCASCADE. A B-rep atravessa como um fluxo de números (`Vec<f64>`) que o
//! `fenkra-kernel` monta e desmonta (`Shape::de_fluxo_nurbs`, `Shape::fluxo_nurbs`). Assim, cada lado só
//! inclui os cabeçalhos da sua biblioteca, e o formato fica escrito num lugar só, aqui.
//!
//! # Fluxo da B-rep, versão 1
//!
//! Tudo em `f64`; contagens e índices são inteiros guardados como `f64`.
//!
//! ```text
//! 3141, 1                                  marca e versão
//! nv, nv × (x, y, z, tol)                  vértices
//! ne, ne × (v0, v1, tol, curva3)           arestas; v0 no começo da curva
//! nf, nf × face
//! face   = invertida, superficie, nl, nl × laço
//! laço   = externo (1/0), nt, nt × aparo
//! aparo  = aresta (-1 = singular), invertido (a aresta corre contra o aparo), vertice (só no singular), curva2
//! superficie: tipo 0 NURBS    = grau_u, grau_v, nu, nv, nós_u (nu+grau_u+1), nós_v (nv+grau_v+1),
//!                                nu × nv × (x, y, z, w), com u por fora
//!             tipo 1 revolução = ox, oy, oz, dx, dy, dz, curva3 (meridiano); u = ângulo em radianos,
//!                                v = parâmetro do meridiano
//!             tipo 2 extrusão  = dx, dy, dz (unitária), curva3 (base); u = parâmetro da base, v = distância
//! curva3: tipo 0 NURBS = grau, n, nós (n+grau+1), n × (x, y, z, w)
//!         tipo 1 reta  = ox, oy, oz, dx, dy, dz (unitária); parâmetro = distância
//!         tipo 2 círculo = cx, cy, cz, nx, ny, nz, xx, xy, xz, raio; parâmetro = ângulo
//!         tipo -1 nenhuma (aresta degenerada)
//! curva2: grau, n, nós (n+grau+1), n × (u, v, w)
//! ```
//!
//! Os nós vêm completos, com as multiplicidades das pontas (o openNURBS guarda dois a menos). Os polos
//! vêm em coordenadas cartesianas com o peso à parte. Os aparos ficam no espaço de parâmetros da
//! superfície descrita; quem lê do openNURBS já leva para lá os aparos de superfícies de revolução e de
//! extrusão, cujo parâmetro difere do da forma NURBS.

#[cfg(opennurbs)]
mod ffi;

use std::path::Path;

/// Como obter o openNURBS quando o crate foi compilado sem ele.
pub const COMO_OBTER: &str = "este Fenkra foi compilado sem o openNURBS, a biblioteca aberta do 3DM. Quem compila: rode `python -I scripts/baixar_opennurbs.py` (baixa a versão fixada do repositório oficial) e compile de novo. Enquanto isso, exporte STEP AP214 ou AP242 no programa de origem, que leva as superfícies exatas, as camadas e as cores";

#[derive(Debug, thiserror::Error, Clone, PartialEq)]
pub enum Erro {
    #[error("{0}")]
    Ausente(String),
    #[error("{0}")]
    Leitura(String),
    #[error("{0}")]
    Gravacao(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Camada {
    pub nome: String,
    pub cor: [u8; 3],
    pub pai: Option<usize>,
    pub visivel: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Geometria {
    /// fluxo da B-rep (veja o topo do módulo)
    Brep(Vec<f64>),
    /// xyz por vértice e três índices por triângulo
    Malha { vertices: Vec<f64>, triangulos: Vec<u32> },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Objeto {
    pub nome: String,
    pub camada: Option<usize>,
    /// cor própria do objeto; `None` = a da camada
    pub cor: Option<[u8; 3]>,
    pub geometria: Geometria,
    /// aparos levados ponto a ponto, não exatos (superfície sem forma NURBS de mesmo parâmetro)
    pub aproximado: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Arquivo {
    pub camadas: Vec<Camada>,
    pub objetos: Vec<Objeto>,
    /// tipo de cada objeto do arquivo que a leitura não traduziu
    pub ignorados: Vec<String>,
    /// versão do 3DM lido (na gravação, 0 = a atual do openNURBS)
    pub versao: i32,
    /// tolerância absoluta do modelo, em mm
    pub tolerancia: f64,
}

impl Arquivo {
    /// Cor com que o objeto aparece: a própria ou a da camada.
    pub fn cor_de(&self, o: &Objeto) -> Option<[u8; 3]> {
        o.cor.or_else(|| o.camada.and_then(|c| self.camadas.get(c)).map(|c| c.cor))
    }

    /// Nome da camada com as mães, separado por " > ", como a árvore mostra.
    pub fn caminho_da_camada(&self, i: usize) -> String {
        let mut nomes = vec![];
        let mut atual = Some(i);
        while let Some(c) = atual.and_then(|k| self.camadas.get(k)) {
            nomes.push(c.nome.clone());
            if nomes.len() > 64 {
                break;
            }
            atual = c.pai;
        }
        nomes.reverse();
        nomes.join(" > ")
    }
}

/// O crate foi compilado com o openNURBS.
pub const fn disponivel() -> bool {
    cfg!(opennurbs)
}

/// Versão do openNURBS compilado, se houver.
pub fn versao_opennurbs() -> Option<String> {
    #[cfg(opennurbs)]
    {
        Some(ffi::bridge::versao_opennurbs())
    }
    #[cfg(not(opennurbs))]
    {
        None
    }
}

fn texto(caminho: &Path) -> Result<&str, String> {
    caminho.to_str().ok_or_else(|| format!("caminho com caracteres inválidos: {}", caminho.display()))
}

#[cfg(opennurbs)]
fn rgb(c: u32) -> [u8; 3] {
    [(c >> 16) as u8, (c >> 8) as u8, c as u8]
}

#[cfg(opennurbs)]
fn de_rgb(c: [u8; 3]) -> u32 {
    (u32::from(c[0]) << 16) | (u32::from(c[1]) << 8) | u32::from(c[2])
}

/// Lê um 3DM, com a geometria em milímetros.
pub fn ler(caminho: &Path) -> Result<Arquivo, Erro> {
    let c = texto(caminho).map_err(Erro::Leitura)?;
    #[cfg(opennurbs)]
    {
        let b = ffi::bridge::ler_3dm(c).map_err(|e| Erro::Leitura(e.what().to_string()))?;
        let indice = |i: i32| usize::try_from(i).ok();
        let camadas = b.camadas.into_iter().map(|c| Camada { nome: c.nome, cor: rgb(c.cor), pai: indice(c.pai), visivel: c.visivel }).collect();
        let objetos = b
            .objetos
            .into_iter()
            .map(|o| Objeto {
                nome: o.nome,
                camada: indice(o.camada),
                cor: o.cor_propria.then(|| rgb(o.cor)),
                geometria: if o.tipo == 0 { Geometria::Brep(o.brep) } else { Geometria::Malha { vertices: o.vertices, triangulos: o.triangulos } },
                aproximado: o.aproximado,
            })
            .collect();
        Ok(Arquivo { camadas, objetos, ignorados: b.ignorados, versao: b.versao, tolerancia: b.tolerancia })
    }
    #[cfg(not(opennurbs))]
    {
        let _ = c;
        Err(Erro::Ausente(COMO_OBTER.into()))
    }
}

/// Grava um 3DM em milímetros. Devolve os avisos do openNURBS sobre B-reps que ele não considera válidas.
pub fn gravar(caminho: &Path, a: &Arquivo) -> Result<Vec<String>, Erro> {
    let c = texto(caminho).map_err(Erro::Gravacao)?;
    #[cfg(opennurbs)]
    {
        use ffi::bridge::{ArquivoBruto, CamadaBruta, ObjetoBruto};
        let indice = |i: Option<usize>| i.map_or(-1, |k| k as i32);
        let b = ArquivoBruto {
            camadas: a.camadas.iter().map(|c| CamadaBruta { nome: c.nome.clone(), cor: de_rgb(c.cor), pai: indice(c.pai), visivel: c.visivel }).collect(),
            objetos: a
                .objetos
                .iter()
                .map(|o| {
                    let (tipo, brep, vertices, triangulos) = match &o.geometria {
                        Geometria::Brep(f) => (0, f.clone(), vec![], vec![]),
                        Geometria::Malha { vertices, triangulos } => (1, vec![], vertices.clone(), triangulos.clone()),
                    };
                    ObjetoBruto { nome: o.nome.clone(), camada: indice(o.camada), cor: o.cor.map_or(0, de_rgb), cor_propria: o.cor.is_some(), tipo, aproximado: false, brep, vertices, triangulos }
                })
                .collect(),
            ignorados: vec![],
            versao: a.versao,
            tolerancia: a.tolerancia,
        };
        ffi::bridge::gravar_3dm(c, &b).map_err(|e| Erro::Gravacao(e.what().to_string()))
    }
    #[cfg(not(opennurbs))]
    {
        let _ = (c, a);
        Err(Erro::Ausente(COMO_OBTER.into()))
    }
}

#[cfg(all(test, opennurbs))]
mod testes {
    use super::*;

    /// Quadrado plano 10 × 10 como uma face NURBS de grau 1, sem aparo além do contorno.
    fn quadrado() -> Vec<f64> {
        let mut f = vec![3141., 1.];
        let p = [[0., 0.], [10., 0.], [10., 10.], [0., 10.]];
        f.push(4.);
        for q in p {
            f.extend([q[0], q[1], 0., 1e-6]);
        }
        f.push(4.);
        for i in 0..4 {
            let (a, b) = (p[i], p[(i + 1) % 4]);
            f.extend([i as f64, ((i + 1) % 4) as f64, 1e-6, 0., 1., 2., 0., 0., 1., 1., a[0], a[1], 0., 1., b[0], b[1], 0., 1.]);
        }
        f.push(1.);
        // face: não invertida, NURBS 1 × 1 com 2 × 2 polos
        f.extend([0., 0., 1., 1., 2., 2., 0., 0., 10., 10., 0., 0., 10., 10.]);
        for (x, y) in [(0., 0.), (0., 10.), (10., 0.), (10., 10.)] {
            f.extend([x, y, 0., 1.]);
        }
        f.extend([1., 1., 4.]);
        let uv = [[0., 0.], [10., 0.], [10., 10.], [0., 10.]];
        for i in 0..4 {
            let (a, b) = (uv[i], uv[(i + 1) % 4]);
            f.extend([i as f64, 0., -1., 1., 2., 0., 0., 1., 1., a[0], a[1], 1., b[0], b[1], 1.]);
        }
        f
    }

    #[test]
    fn grava_e_le_quadrado_com_camada_e_cor_em_caminho_com_acento() {
        let dir = std::env::temp_dir().join(format!("fenkra 3dm ação {}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let a = dir.join("quadrado ç.3dm");
        let arq = Arquivo {
            camadas: vec![Camada { nome: "Chapas".into(), cor: [10, 20, 30], pai: None, visivel: true }, Camada { nome: "Corte".into(), cor: [200, 0, 0], pai: Some(0), visivel: true }],
            objetos: vec![Objeto { nome: "Tampa".into(), camada: Some(1), cor: Some([1, 2, 3]), geometria: Geometria::Brep(quadrado()), aproximado: false }],
            tolerancia: 0.001,
            ..Default::default()
        };
        let avisos = gravar(&a, &arq).unwrap();
        assert!(avisos.is_empty(), "{avisos:?}");
        let lido = ler(&a).unwrap();
        assert_eq!(lido.camadas.len(), 2, "{:?}", lido.camadas);
        assert_eq!(lido.caminho_da_camada(1), "Chapas > Corte");
        assert_eq!(lido.objetos.len(), 1);
        let o = &lido.objetos[0];
        assert_eq!((o.nome.as_str(), o.camada, o.cor), ("Tampa", Some(1), Some([1, 2, 3])));
        let Geometria::Brep(f) = &o.geometria else { panic!("não veio B-rep") };
        assert_eq!(&f[..3], &[3141., 1., 4.]);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
