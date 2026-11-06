// SPDX-License-Identifier: LGPL-2.1-or-later
//! AMF: XML de manufatura aditiva, às vezes dentro de um ZIP. Objetos com volumes, um material por
//! volume, e constelações que posicionam objetos. A rotação da instância é em graus de Euler, aplicada
//! em X, depois Y, depois Z.

use super::{erro, mm_da_unidade, xml, zip, Cena, Malha, No};
use crate::ErroConv;
use fenkra_kernel::IDENTIDADE;
use std::collections::HashMap;
use std::fmt::Write as _;

/// Ângulos (graus) de `Rz Ry Rx` a partir da matriz.
fn euler(m: &[f64; 12]) -> [f64; 3] {
    let seno = (-m[8]).clamp(-1., 1.);
    let b = seno.asin();
    let (a, c) = if seno.abs() > 1. - 1e-12 { ((-m[6]).atan2(m[5]), 0.) } else { (m[9].atan2(m[10]), m[4].atan2(m[0])) };
    [a, b, c].map(f64::to_degrees)
}

fn de_euler(g: [f64; 3], t: [f64; 3]) -> [f64; 12] {
    let [a, b, c] = g.map(f64::to_radians);
    let ((sa, ca), (sb, cb), (sc, cc)) = (a.sin_cos(), b.sin_cos(), c.sin_cos());
    [cc * cb, cc * sb * sa - sc * ca, cc * sb * ca + sc * sa, t[0], sc * cb, sc * sb * sa + cc * ca, sc * sb * ca - cc * sa, t[1], -sb, cb * sa, cb * ca, t[2]]
}

fn instancia(id: usize, m: &[f64; 12], mm: f64) -> String {
    let [rx, ry, rz] = euler(m);
    format!("<instance objectid=\"{id}\"><deltax>{}</deltax><deltay>{}</deltay><deltaz>{}</deltaz><rx>{rx}</rx><ry>{ry}</ry><rz>{rz}</rz></instance>", m[3] / mm, m[7] / mm, m[11] / mm)
}

struct Saida<'a> {
    cena: &'a Cena,
    objetos: String,
    constelacoes: String,
    por_no: HashMap<usize, usize>,
    por_malhas: HashMap<Vec<usize>, usize>,
    proximo: usize,
    mm: f64,
}

impl Saida<'_> {
    fn id(&mut self) -> usize {
        self.proximo += 1;
        self.proximo
    }

    /// Um objeto com um volume por malha: os vértices são de todo o objeto.
    fn objeto(&mut self, malhas: &[usize], nome: &str) -> usize {
        if let Some(id) = self.por_malhas.get(malhas) {
            return *id;
        }
        let id = self.id();
        let (mut vertices, mut volumes, mut base) = (String::new(), String::new(), 0u32);
        for &i in malhas {
            let Some(m) = self.cena.malhas.get(i).map(Malha::soldada) else { continue };
            for p in &m.pos {
                let _ = write!(vertices, "<vertex><coordinates><x>{}</x><y>{}</y><z>{}</z></coordinates></vertex>", p[0] / self.mm, p[1] / self.mm, p[2] / self.mm);
            }
            let material = m.material.map(|k| format!(" materialid=\"{}\"", k + 1)).unwrap_or_default();
            let _ = write!(volumes, "<volume{material}>");
            for t in &m.tris {
                let _ = write!(volumes, "<triangle><v1>{}</v1><v2>{}</v2><v3>{}</v3></triangle>", t[0] + base, t[1] + base, t[2] + base);
            }
            volumes.push_str("</volume>");
            base += m.pos.len() as u32;
        }
        let proprio = malhas.first().map(|i| self.cena.malhas[*i].nome.as_str()).filter(|n| !n.is_empty()).unwrap_or(nome);
        let _ = writeln!(self.objetos, "<object id=\"{id}\"><metadata type=\"name\">{}</metadata><mesh><vertices>{vertices}</vertices>{volumes}</mesh></object>", xml(proprio));
        self.por_malhas.insert(malhas.to_vec(), id);
        id
    }

    fn no(&mut self, i: usize, pilha: &mut Vec<usize>) -> Option<usize> {
        if let Some(id) = self.por_no.get(&i) {
            return Some(*id);
        }
        if pilha.contains(&i) {
            return None;
        }
        let no = self.cena.nos.get(i)?;
        if no.filhos.is_empty() {
            let id = self.objeto(&no.malhas, &no.nome);
            self.por_no.insert(i, id);
            return Some(id);
        }
        pilha.push(i);
        let mut corpo = String::new();
        if !no.malhas.is_empty() {
            let id = self.objeto(&no.malhas, &no.nome);
            corpo.push_str(&instancia(id, &IDENTIDADE, self.mm));
        }
        for &f in &no.filhos {
            if let Some(id) = self.no(f, pilha) {
                corpo.push_str(&instancia(id, &self.cena.nos[f].local, self.mm));
            }
        }
        pilha.pop();
        let id = self.id();
        let _ = writeln!(self.constelacoes, "<constellation id=\"{id}\"><metadata type=\"name\">{}</metadata>{corpo}</constellation>", xml(&no.nome));
        self.por_no.insert(i, id);
        Some(id)
    }
}

pub(crate) fn gravar(cena: &Cena, mm: f64, unidade: &str) -> String {
    // materiais ocupam os ids de 1 a N, porque o volume aponta para eles
    let mut s = Saida { cena, objetos: String::new(), constelacoes: String::new(), por_no: HashMap::new(), por_malhas: HashMap::new(), proximo: cena.materiais.len(), mm };
    let mut topo = String::new();
    let simples = cena.raizes.len() == 1 && cena.nos.get(cena.raizes[0]).is_some_and(|n| n.local == IDENTIDADE);
    for &r in &cena.raizes {
        if let Some(id) = s.no(r, &mut Vec::new()) {
            topo.push_str(&instancia(id, &cena.nos[r].local, mm));
        }
    }
    if !simples {
        let id = s.id();
        let _ = writeln!(s.constelacoes, "<constellation id=\"{id}\"><metadata type=\"name\">{}</metadata>{topo}</constellation>", xml(&cena.nome));
    }
    let mut t = format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<amf unit=\"{unidade}\" version=\"1.1\">\n<metadata type=\"cad\">Fenkra</metadata>\n");
    for (i, k) in cena.materiais.iter().enumerate() {
        let _ = writeln!(t, "<material id=\"{}\"><metadata type=\"name\">{}</metadata><color><r>{}</r><g>{}</g><b>{}</b><a>{}</a></color></material>", i + 1, xml(&k.nome), k.cor[0], k.cor[1], k.cor[2], k.cor[3]);
    }
    t.push_str(&s.objetos);
    t.push_str(&s.constelacoes);
    t.push_str("</amf>\n");
    t
}

fn filho<'a, 'i>(n: roxmltree::Node<'a, 'i>, nome: &str) -> Option<roxmltree::Node<'a, 'i>> {
    n.children().find(|c| c.tag_name().name() == nome)
}

fn valor(n: roxmltree::Node, nome: &str) -> f64 {
    filho(n, nome).and_then(|c| c.text()).and_then(|t| t.trim().parse().ok()).unwrap_or(0.)
}

fn cor(n: roxmltree::Node) -> Option<[f32; 4]> {
    let c = filho(n, "color")?;
    let a = filho(c, "a").map(|_| valor(c, "a") as f32).unwrap_or(1.);
    Some([valor(c, "r") as f32, valor(c, "g") as f32, valor(c, "b") as f32, a])
}

fn nome_de(n: roxmltree::Node) -> String {
    n.children().find(|c| c.tag_name().name() == "metadata" && c.attribute("type") == Some("name")).and_then(|c| c.text()).unwrap_or("").to_string()
}

/// Volume com só os vértices que usa, para a malha de cada material fechar sozinha.
fn volume(pos: &[[f64; 3]], tris: Vec<[u32; 3]>, nome: &str, material: Option<usize>) -> Result<Malha, ErroConv> {
    let mut m = Malha { nome: nome.into(), material, ..Default::default() };
    let mut novo: HashMap<u32, u32> = HashMap::new();
    for t in tris {
        let mut r = [0; 3];
        for (k, v) in t.into_iter().enumerate() {
            let p = *pos.get(v as usize).ok_or_else(|| erro("AMF", "triângulo com vértice inexistente"))?;
            r[k] = *novo.entry(v).or_insert_with(|| {
                m.pos.push(p);
                (m.pos.len() - 1) as u32
            });
        }
        m.tris.push(r);
    }
    m.conferir("AMF")?;
    Ok(m)
}

pub(crate) fn ler(b: &[u8]) -> Result<Cena, ErroConv> {
    let aberto;
    let bytes: &[u8] = if zip::e_zip(b) {
        aberto = zip::abrir(b, |n| n.to_lowercase().ends_with(".amf"))?.ok_or_else(|| erro("AMF", "pacote sem .amf"))?;
        &aberto
    } else {
        b
    };
    let texto = String::from_utf8_lossy(bytes);
    let doc = roxmltree::Document::parse(&texto).map_err(|e| erro("AMF", e))?;
    let raiz = doc.root_element();
    let mut c = Cena::default();
    let unidade = raiz.attribute("unit").unwrap_or("millimeter");
    let mm = mm_da_unidade(unidade).ok_or_else(|| erro("AMF", format!("unidade desconhecida: {unidade}")))?;
    c.unidade = unidade.to_string();
    let elementos = |nome: &'static str| raiz.children().filter(move |n| n.tag_name().name() == nome);

    let mut materiais: HashMap<String, usize> = HashMap::new();
    for m in elementos("material") {
        let Some(id) = m.attribute("id") else { continue };
        if let Some(k) = cor(m) {
            let nome = nome_de(m);
            materiais.insert(id.to_string(), c.material(&nome, k));
        }
    }
    c.ignorar("textura", elementos("texture").count());

    let mut por_id: HashMap<String, usize> = HashMap::new();
    for o in elementos("object") {
        let Some(id) = o.attribute("id") else { continue };
        let nome = nome_de(o);
        let mut no = No::novo(&nome);
        let cor_objeto = cor(o);
        let Some(malha) = filho(o, "mesh") else { continue };
        let mut pos = Vec::new();
        for v in malha.descendants().filter(|n| n.tag_name().name() == "vertex") {
            let Some(q) = filho(v, "coordinates") else { return Err(erro("AMF", "vértice sem coordenadas")) };
            pos.push([valor(q, "x"), valor(q, "y"), valor(q, "z")]);
            c.ignorar("cor por vértice", usize::from(filho(v, "color").is_some()));
        }
        c.ignorar("aresta curva", malha.descendants().filter(|n| n.tag_name().name() == "edge").count());
        for v in malha.children().filter(|n| n.tag_name().name() == "volume") {
            let mut tris = Vec::new();
            for t in v.children().filter(|n| n.tag_name().name() == "triangle") {
                tris.push(["v1", "v2", "v3"].map(|k| valor(t, k) as u32));
            }
            let material = v.attribute("materialid").and_then(|m| materiais.get(m).copied());
            let propria = cor(v).or(cor_objeto).filter(|_| material.is_none()).map(|k| c.material("Cor", k));
            no.malhas.push(c.malhas.len());
            let m = volume(&pos, tris, &nome, material.or(propria))?;
            c.malhas.push(m);
        }
        c.nos.push(no);
        por_id.insert(id.to_string(), c.nos.len() - 1);
    }
    for k in elementos("constellation") {
        let Some(id) = k.attribute("id") else { continue };
        c.nos.push(No::novo(&nome_de(k)));
        por_id.insert(id.to_string(), c.nos.len() - 1);
    }
    let mut usados = vec![false; c.nos.len()];
    for k in elementos("constellation") {
        let Some(pai) = k.attribute("id").and_then(|id| por_id.get(id).copied()) else { continue };
        for i in k.children().filter(|n| n.tag_name().name() == "instance") {
            let Some(alvo) = i.attribute("objectid").and_then(|id| por_id.get(id).copied()) else { continue };
            usados[alvo] = true;
            let mut no = No::novo("");
            no.local = de_euler(["rx", "ry", "rz"].map(|e| valor(i, e)), ["deltax", "deltay", "deltaz"].map(|e| valor(i, e)));
            no.filhos = vec![alvo];
            c.nos.push(no);
            let novo = c.nos.len() - 1;
            c.nos[pai].filhos.push(novo);
        }
    }
    let mut tops: Vec<usize> = por_id.values().copied().filter(|i| !usados[*i]).collect();
    tops.sort_unstable();
    c.raizes = tops;
    if c.raizes.is_empty() {
        return Err(erro("AMF", "nenhum objeto de topo"));
    }
    c.escalar(mm);
    Ok(c)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn euler_vai_e_volta_inclusive_na_trava() {
        for g in [[10., 20., 30.], [0., 0., 90.], [-45., 60., 170.], [15., 90., 0.], [0., -90., 0.]] {
            let m = de_euler(g, [1., 2., 3.]);
            let r = de_euler(euler(&m), [1., 2., 3.]);
            assert!(m.iter().zip(r).all(|(a, b)| (a - b).abs() < 1e-9), "{g:?}");
        }
    }

    #[test]
    fn gira_x_antes_de_z() {
        // X 90 graus leva Y para Z; o Z 90 seguinte não mexe em Z
        let m = de_euler([90., 0., 90.], [0.; 3]);
        let y = [m[1], m[5], m[9]];
        assert!((y[2] - 1.).abs() < 1e-12, "{y:?}");
    }
}
