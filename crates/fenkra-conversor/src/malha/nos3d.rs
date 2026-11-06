// SPDX-License-Identifier: LGPL-2.1-or-later
//! Grafo de cena comum a VRML e X3D: o X3D é o VRML em XML, com os mesmos nós e campos. Cada leitor só
//! traduz a sintaxe para [`No3d`]; a montagem da cena mora aqui, uma vez.
//!
//! Os dois formatos convencionam metros e Y para cima. `DEF` entra na tabela depois que o nó termina,
//! então `USE` nunca aponta para um ancestral: o grafo não tem ciclo.

use super::{Cena, Malha, No};
use crate::modelo::{compor, PROFUNDIDADE};
use crate::ErroConv;
use fenkra_kernel::IDENTIDADE;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

/// Valor de um campo. Números ficam numa lista só, seja um escalar, um vetor ou uma tabela.
#[derive(Debug)]
pub(super) enum Valor {
    Numeros(Vec<f64>),
    Nos(Vec<Rc<No3d>>),
    Texto(String),
}

#[derive(Debug, Default)]
pub(super) struct No3d {
    pub tipo: String,
    pub nome: String,
    pub campos: HashMap<String, Valor>,
}

impl No3d {
    fn numeros(&self, campo: &str) -> &[f64] {
        match self.campos.get(campo) {
            Some(Valor::Numeros(v)) => v,
            _ => &[],
        }
    }

    fn nos(&self, campo: &str) -> &[Rc<No3d>] {
        match self.campos.get(campo) {
            Some(Valor::Nos(v)) => v,
            _ => &[],
        }
    }

    fn no(&self, campo: &str) -> Option<&Rc<No3d>> {
        self.nos(campo).first()
    }

    /// Campo lógico: VRML escreve `FALSE`, X3D `false`.
    fn logico(&self, campo: &str, padrao: bool) -> bool {
        match self.campos.get(campo) {
            Some(Valor::Texto(t)) => t.eq_ignore_ascii_case("true"),
            _ => padrao,
        }
    }
}

/// Giro em torno de um eixo, pela fórmula de Rodrigues.
fn giro(r: &[f64]) -> [f64; 12] {
    let [x, y, z, a] = r else { return IDENTIDADE };
    let n = (x * x + y * y + z * z).sqrt();
    if n < 1e-12 {
        return IDENTIDADE;
    }
    let (x, y, z) = (x / n, y / n, z / n);
    let (s, c) = a.sin_cos();
    let t = 1. - c;
    [t * x * x + c, t * x * y - s * z, t * x * z + s * y, 0., t * x * y + s * z, t * y * y + c, t * y * z - s * x, 0., t * x * z - s * y, t * y * z + s * x, t * z * z + c, 0.]
}

fn deslocamento(t: [f64; 3]) -> [f64; 12] {
    [1., 0., 0., t[0], 0., 1., 0., t[1], 0., 0., 1., t[2]]
}

fn vetor(v: &[f64], padrao: f64) -> [f64; 3] {
    match v {
        [x, y, z, ..] => [*x, *y, *z],
        _ => [padrao; 3],
    }
}

/// Matriz de um `Transform`: desloca, vai ao centro, gira, escala, volta do centro.
fn matriz(n: &No3d) -> [f64; 12] {
    let (t, c, s) = (vetor(n.numeros("translation"), 0.), vetor(n.numeros("center"), 0.), vetor(n.numeros("scale"), 1.));
    let escala = [s[0], 0., 0., 0., 0., s[1], 0., 0., 0., 0., s[2], 0.];
    let giro_escala = compor(&giro(n.numeros("rotation")), &escala);
    let volta = deslocamento(c.map(|v| -v));
    compor(&compor(&deslocamento(t), &deslocamento(c)), &compor(&giro_escala, &volta))
}

/// Eixo e ângulo de uma rotação, para gravar. Perto de 180 graus o eixo vem da diagonal.
pub(super) fn eixo_angulo(m: &[f64; 12]) -> [f64; 4] {
    let traco = m[0] + m[5] + m[10];
    let v = [m[9] - m[6], m[2] - m[8], m[4] - m[1]];
    let n = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    // o arco cosseno perde metade dos dígitos perto de 180 graus; seno e cosseno juntos não
    let angulo = (n / 2.).atan2((traco - 1.) / 2.);
    if angulo < 1e-12 {
        return [0., 0., 1., 0.];
    }
    if n > 1e-9 {
        return [v[0] / n, v[1] / n, v[2] / n, angulo];
    }
    let d = [m[0], m[5], m[10]].map(|x| ((x + 1.) / 2.).max(0.).sqrt());
    let k = (0..3).max_by(|a, b| d[*a].total_cmp(&d[*b])).unwrap_or(0);
    let mut e = d;
    for j in 0..3 {
        if j != k && m[k * 4 + j] < 0. {
            e[j] = -e[j];
        }
    }
    [e[0], e[1], e[2], angulo]
}

/// Tipos que só agrupam. `Switch` e `LOD` guardam alternativas; vão todas, e o relatório avisa.
fn agrupa(tipo: &str) -> bool {
    matches!(tipo, "Transform" | "Group" | "Collision" | "Anchor" | "Billboard" | "StaticGroup" | "Scene" | "Switch" | "LOD")
}

fn fora(tipo: &str) -> Option<&'static str> {
    Some(match tipo {
        "Box" | "Sphere" | "Cylinder" | "Cone" | "ElevationGrid" | "Extrusion" | "Text" => "primitiva sem malha",
        "IndexedLineSet" | "PointSet" | "LineSet" => "ponto ou linha",
        "Inline" => "arquivo externo",
        "Viewpoint" | "NavigationInfo" | "Background" | "DirectionalLight" | "PointLight" | "SpotLight" | "Fog" | "WorldInfo" => return None,
        "ImageTexture" | "PixelTexture" | "MovieTexture" => "textura",
        _ => return None,
    })
}

#[derive(Default)]
struct Montador {
    cena: Cena,
    nos: HashMap<*const No3d, usize>,
    malhas: HashMap<(*const No3d, Option<[u32; 4]>), usize>,
    vistos: HashSet<*const No3d>,
}

impl Montador {
    fn malha(&mut self, g: &Rc<No3d>, cor: Option<[f32; 4]>, nome: &str, formato: &str) -> Result<Option<usize>, ErroConv> {
        let chave = (Rc::as_ptr(g), cor.map(|c| c.map(f32::to_bits)));
        if let Some(i) = self.malhas.get(&chave) {
            return Ok(Some(*i));
        }
        let pontos = g.no("coord").map(|c| c.numeros("point")).unwrap_or_default();
        let pos: Vec<[f64; 3]> = pontos.chunks_exact(3).map(|p| [p[0], p[1], p[2]]).collect();
        let mut tris = Vec::new();
        match g.tipo.as_str() {
            "IndexedFaceSet" => {
                for face in g.numeros("coordIndex").split(|i| *i < 0.) {
                    for k in 1..face.len().saturating_sub(1) {
                        tris.push([face[0] as u32, face[k] as u32, face[k + 1] as u32]);
                    }
                }
            }
            "IndexedTriangleSet" => tris.extend(g.numeros("index").chunks_exact(3).map(|t| [t[0] as u32, t[1] as u32, t[2] as u32])),
            "TriangleSet" => tris.extend((0..pos.len() as u32 / 3).map(|i| [i * 3, i * 3 + 1, i * 3 + 2])),
            tipo => {
                if let Some(oque) = fora(tipo) {
                    self.cena.ignorar(oque, 1);
                }
                return Ok(None);
            }
        }
        if !g.logico("ccw", true) {
            tris.iter_mut().for_each(|t| t.swap(1, 2));
        }
        let mut m = Malha { nome: nome.into(), pos, tris, ..Default::default() };
        if let Some(c) = g.no("color") {
            let largura = if c.tipo == "ColorRGBA" { 4 } else { 3 };
            let canal = |x: f64| (x.clamp(0., 1.) * 255.).round() as u8;
            let cores: Vec<[u8; 4]> = c.numeros("color").chunks_exact(largura).map(|k| [canal(k[0]), canal(k[1]), canal(k[2]), k.get(3).map(|a| canal(*a)).unwrap_or(255)]).collect();
            if cores.len() == m.pos.len() && g.numeros("colorIndex").is_empty() && g.logico("colorPerVertex", true) {
                m.cor_v = cores;
            } else {
                self.cena.ignorar("cor por face", 1);
            }
        }
        m.conferir(formato)?;
        if let Some(c) = cor {
            let nome_cor = format!("Cor {}", self.cena.materiais.len() + 1);
            m.material = Some(self.cena.material(&nome_cor, c));
        }
        self.cena.malhas.push(m);
        self.malhas.insert(chave, self.cena.malhas.len() - 1);
        Ok(Some(self.cena.malhas.len() - 1))
    }

    fn visitar(&mut self, n: &Rc<No3d>, nivel: usize, formato: &str) -> Result<Option<usize>, ErroConv> {
        let chave = Rc::as_ptr(n);
        if let Some(i) = self.nos.get(&chave) {
            return Ok(Some(*i));
        }
        if nivel > PROFUNDIDADE {
            return Err(super::erro(formato, "árvore funda demais"));
        }
        let mut no = No::novo(&n.nome);
        if n.tipo == "Shape" {
            let material = n.no("appearance").and_then(|a| {
                if a.no("texture").is_some() {
                    self.cena.ignorar("textura", 1);
                }
                a.no("material")
            });
            let cor = material.map(|k| {
                let c = vetor(k.numeros("diffuseColor"), 0.8);
                let t = k.numeros("transparency").first().copied().unwrap_or(0.);
                [c[0] as f32, c[1] as f32, c[2] as f32, (1. - t) as f32]
            });
            let Some(g) = n.no("geometry") else { return Ok(None) };
            let Some(m) = self.malha(g, cor, &n.nome, formato)? else { return Ok(None) };
            no.malhas.push(m);
        } else if agrupa(&n.tipo) {
            if matches!(n.tipo.as_str(), "Switch" | "LOD") {
                self.cena.ignorar("alternativa de Switch ou LOD", 1);
            }
            if n.tipo == "Transform" {
                no.local = matriz(n);
                if n.numeros("scaleOrientation").get(3).is_some_and(|a| a.abs() > 1e-9) {
                    self.cena.ignorar("orientação de escala", 1);
                }
            }
            for campo in ["children", "choice", "level"] {
                for f in n.nos(campo) {
                    if let Some(i) = self.visitar(f, nivel + 1, formato)? {
                        no.filhos.push(i);
                    }
                }
            }
            if no.filhos.is_empty() {
                return Ok(None);
            }
        } else {
            if self.vistos.insert(chave) {
                if let Some(oque) = fora(&n.tipo) {
                    self.cena.ignorar(oque, 1);
                }
            }
            return Ok(None);
        }
        self.cena.nos.push(no);
        self.nos.insert(chave, self.cena.nos.len() - 1);
        Ok(Some(self.cena.nos.len() - 1))
    }
}

/// Cena dos nós de topo. `metros` é quantos metros vale cada unidade do arquivo.
pub(super) fn cena_de(raizes: &[Rc<No3d>], ignorados: Vec<(String, usize)>, metros: f64, formato: &str) -> Result<Cena, ErroConv> {
    let mut m = Montador::default();
    for r in raizes {
        if let Some(i) = m.visitar(r, 0, formato)? {
            m.cena.raizes.push(i);
        }
    }
    let mut c = m.cena;
    for (oque, n) in ignorados {
        c.ignorar(&oque, n);
    }
    c.unidade = "METRE".into();
    c.embrulhar([1., 0., 0., 0., 0., 0., -1., 0., 0., 1., 0., 0.]);
    c.escalar(metros * 1000.);
    Ok(c)
}

/// Cena pronta para gravar: rígida e em metros.
pub(super) fn em_metros(cena: &Cena) -> Cena {
    let mut c = cena.clone();
    c.assentar();
    c.escalar(0.001);
    c
}

/// Nomes de `DEF`: sem espaço nem símbolo, e sem repetir.
pub(super) fn nomes_def(cena: &Cena) -> Vec<String> {
    let mut usados = HashSet::new();
    cena.nos
        .iter()
        .enumerate()
        .map(|(i, n)| {
            let limpo: String = n.nome.chars().map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' }).collect();
            let base = if limpo.is_empty() || limpo.starts_with(|c: char| c.is_ascii_digit()) { format!("N_{limpo}") } else { limpo };
            let nome = if usados.contains(&base) { format!("{base}_{i}") } else { base };
            usados.insert(nome.clone());
            nome
        })
        .collect()
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn eixo_e_angulo_voltam_a_mesma_matriz() {
        for r in [[0., 0., 1., 1.], [1., 0., 0., 2.5], [1., 1., 0., std::f64::consts::PI], [0., 1., 0., std::f64::consts::PI], [1., -1., 1., std::f64::consts::PI]] {
            let m = giro(&r);
            let e = eixo_angulo(&m);
            let v = giro(&e);
            assert!(m.iter().zip(v).all(|(a, b)| (a - b).abs() < 1e-9), "{r:?} {e:?}");
        }
    }

    #[test]
    fn transform_com_centro_gira_em_torno_dele() {
        let mut n = No3d { tipo: "Transform".into(), ..Default::default() };
        n.campos.insert("rotation".into(), Valor::Numeros(vec![0., 0., 1., std::f64::consts::FRAC_PI_2]));
        n.campos.insert("center".into(), Valor::Numeros(vec![1., 0., 0.]));
        let m = matriz(&n);
        // o centro fica parado
        assert!((m[0] + m[3] - 1.).abs() < 1e-12 && (m[4] + m[7]).abs() < 1e-12, "{m:?}");
    }
}
