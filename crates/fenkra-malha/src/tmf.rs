// SPDX-License-Identifier: LGPL-2.1-or-later
//! 3MF: pacote ZIP com um XML de modelo. Objetos com malha ou com componentes, itens de construção e
//! unidade declarada. A especificação pede malha fechada e vértices compartilhados, por isso a saída é
//! soldada, e pede que o objeto seja definido antes de quem o usa, por isso a saída vai em pós-ordem.

use crate::{erro, mm_da_unidade, xml, zip, Cena, Malha, No};
use crate::Erro as ErroConv;
use crate::IDENTIDADE;
use std::collections::HashMap;
use std::fmt::Write as _;

const TIPOS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="model" ContentType="application/vnd.ms-package.3dmanufacturing-3dmodel+xml"/></Types>"#;
const RELACOES: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Target="/3D/3dmodel.model" Id="rel0" Type="http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel"/></Relationships>"#;

/// A matriz do 3MF multiplica vetor linha: a rotação sai transposta e o deslocamento na última linha.
fn transformacao(m: &[f64; 12], mm: f64) -> String {
    format!("{} {} {} {} {} {} {} {} {} {} {} {}", m[0], m[4], m[8], m[1], m[5], m[9], m[2], m[6], m[10], m[3] / mm, m[7] / mm, m[11] / mm)
}

fn ler_transformacao(t: Option<&str>) -> [f64; 12] {
    let n: Vec<f64> = t.unwrap_or("").split_whitespace().filter_map(|x| x.parse().ok()).collect();
    match n[..] {
        [a, b, c, d, e, f, g, h, i, x, y, z] => [a, d, g, x, b, e, h, y, c, f, i, z],
        _ => IDENTIDADE,
    }
}

fn hex(c: [f32; 4]) -> String {
    let [r, g, b, a] = c.map(|v| (v.clamp(0., 1.) * 255.).round() as u8);
    format!("#{r:02X}{g:02X}{b:02X}{a:02X}")
}

fn de_hex(t: &str) -> Option<[f32; 4]> {
    let t = t.strip_prefix('#')?;
    let canal = |i: usize| u8::from_str_radix(t.get(i..i + 2)?, 16).ok().map(|v| f32::from(v) / 255.);
    Some([canal(0)?, canal(2)?, canal(4)?, if t.len() >= 8 { canal(6)? } else { 1. }])
}

struct Saida<'a> {
    cena: &'a Cena,
    texto: String,
    /// id do objeto de cada malha e de cada nó já escrito
    malhas: HashMap<usize, usize>,
    nos: HashMap<usize, usize>,
    proximo: usize,
    grupo: usize,
    mm: f64,
}

impl Saida<'_> {
    fn id(&mut self) -> usize {
        self.proximo += 1;
        self.proximo
    }

    fn malha(&mut self, i: usize) -> Option<usize> {
        if let Some(id) = self.malhas.get(&i) {
            return Some(*id);
        }
        let m = self.cena.malhas.get(i)?.soldada();
        if m.tris.is_empty() {
            return None;
        }
        let id = self.id();
        let cor = m.material.map(|k| format!(r#" pid="{}" pindex="{k}""#, self.grupo)).unwrap_or_default();
        let _ = write!(self.texto, r#"<object id="{id}" type="model" name="{}"{cor}><mesh><vertices>"#, xml(&m.nome));
        for p in &m.pos {
            let _ = write!(self.texto, r#"<vertex x="{}" y="{}" z="{}"/>"#, p[0] / self.mm, p[1] / self.mm, p[2] / self.mm);
        }
        self.texto.push_str("</vertices><triangles>");
        for t in &m.tris {
            let _ = write!(self.texto, r#"<triangle v1="{}" v2="{}" v3="{}"/>"#, t[0], t[1], t[2]);
        }
        self.texto.push_str("</triangles></mesh></object>\n");
        self.malhas.insert(i, id);
        Some(id)
    }

    fn no(&mut self, i: usize, pilha: &mut Vec<usize>) -> Option<usize> {
        if let Some(id) = self.nos.get(&i) {
            return Some(*id);
        }
        if pilha.contains(&i) {
            return None;
        }
        let no = self.cena.nos.get(i)?;
        if no.filhos.is_empty() && no.malhas.len() == 1 {
            let id = self.malha(no.malhas[0])?;
            self.nos.insert(i, id);
            return Some(id);
        }
        pilha.push(i);
        let mut componentes = String::new();
        for &m in &no.malhas {
            if let Some(id) = self.malha(m) {
                let _ = write!(componentes, r#"<component objectid="{id}"/>"#);
            }
        }
        for &f in &no.filhos {
            if let Some(id) = self.no(f, pilha) {
                let _ = write!(componentes, r#"<component objectid="{id}" transform="{}"/>"#, transformacao(&self.cena.nos[f].local, self.mm));
            }
        }
        pilha.pop();
        if componentes.is_empty() {
            return None;
        }
        let id = self.id();
        let _ = writeln!(self.texto, r#"<object id="{id}" type="model" name="{}"><components>{componentes}</components></object>"#, xml(&no.nome));
        self.nos.insert(i, id);
        Some(id)
    }
}

pub fn gravar(cena: &Cena, mm: f64, unidade: &str) -> Result<Vec<u8>, ErroConv> {
    let mut s = Saida { cena, texto: String::new(), malhas: HashMap::new(), nos: HashMap::new(), proximo: 1, grupo: 1, mm };
    if !cena.materiais.is_empty() {
        s.texto.push_str(r#"<basematerials id="1">"#);
        for k in &cena.materiais {
            let _ = write!(s.texto, r#"<base name="{}" displaycolor="{}"/>"#, xml(&k.nome), hex(k.cor));
        }
        s.texto.push_str("</basematerials>\n");
    }
    let mut itens = String::new();
    for &r in &cena.raizes {
        if let Some(id) = s.no(r, &mut Vec::new()) {
            let _ = write!(itens, r#"<item objectid="{id}" transform="{}"/>"#, transformacao(&cena.nos[r].local, mm));
        }
    }
    let modelo = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<model unit=\"{unidade}\" xml:lang=\"pt-BR\" xmlns=\"http://schemas.microsoft.com/3dmanufacturing/core/2015/02\"><metadata name=\"Title\">{}</metadata><metadata name=\"Application\">Fenkra</metadata>\n<resources>\n{}</resources><build>{itens}</build></model>\n",
        xml(&cena.nome),
        s.texto
    );
    zip::gravar(&[("[Content_Types].xml", TIPOS.as_bytes()), ("_rels/.rels", RELACOES.as_bytes()), ("3D/3dmodel.model", modelo.as_bytes())], false)
}

fn numero(n: roxmltree::Node, nome: &str) -> Result<f64, ErroConv> {
    n.attribute(nome).and_then(|v| v.parse().ok()).ok_or_else(|| erro("3MF", format!("atributo {nome} inválido")))
}

fn inteiro(n: roxmltree::Node, nome: &str) -> Option<usize> {
    n.attribute(nome).and_then(|v| v.parse().ok())
}

/// Malhas de um objeto, uma por material de triângulo.
fn malhas_do_objeto(objeto: roxmltree::Node, malha: roxmltree::Node, cores: &HashMap<(usize, usize), usize>) -> Result<Vec<Malha>, ErroConv> {
    let nome = objeto.attribute("name").unwrap_or("").to_string();
    let mut pos = Vec::new();
    let mut grupos: Vec<(Option<usize>, Vec<[u32; 3]>)> = Vec::new();
    let padrao = inteiro(objeto, "pid").map(|p| (p, inteiro(objeto, "pindex").unwrap_or(0)));
    for filho in malha.descendants().filter(|n| n.is_element()) {
        match filho.tag_name().name() {
            "vertex" => pos.push([numero(filho, "x")?, numero(filho, "y")?, numero(filho, "z")?]),
            "triangle" => {
                let v = [inteiro(filho, "v1"), inteiro(filho, "v2"), inteiro(filho, "v3")];
                let [Some(a), Some(b), Some(c)] = v else { return Err(erro("3MF", "triângulo sem vértices")) };
                let propria = inteiro(filho, "p1").map(|p| (inteiro(filho, "pid").or(padrao.map(|d| d.0)).unwrap_or(0), p));
                let material = propria.or(padrao).and_then(|k| cores.get(&k).copied());
                let i = match grupos.iter().position(|(m, _)| *m == material) {
                    Some(i) => i,
                    None => {
                        grupos.push((material, vec![]));
                        grupos.len() - 1
                    }
                };
                grupos[i].1.push([a as u32, b as u32, c as u32]);
            }
            _ => {}
        }
    }
    let mut saida = Vec::new();
    for (material, tris) in grupos {
        let m = Malha { nome: nome.clone(), pos: pos.clone(), tris, material, ..Default::default() };
        m.conferir("3MF")?;
        saida.push(m);
    }
    Ok(saida)
}

pub fn ler(b: &[u8]) -> Result<Cena, ErroConv> {
    let relacoes = zip::abrir(b, |n| n == "_rels/.rels")?.unwrap_or_default();
    let relacoes = String::from_utf8_lossy(&relacoes).to_string();
    let alvo = relacoes.split("Target=\"").skip(1).filter_map(|t| t.split('"').next()).find(|t| t.ends_with(".model")).map(|t| t.trim_start_matches('/').to_string());
    let bytes = zip::abrir(b, |n| alvo.as_deref().map(|a| a == n).unwrap_or(n.ends_with(".model")))?.ok_or_else(|| erro("3MF", "pacote sem modelo"))?;
    let texto = String::from_utf8_lossy(&bytes);
    let doc = roxmltree::Document::parse(&texto).map_err(|e| erro("3MF", e))?;
    let raiz = doc.root_element();
    let mut c = Cena::default();
    let unidade = raiz.attribute("unit").unwrap_or("millimeter");
    let mm = mm_da_unidade(unidade).ok_or_else(|| erro("3MF", format!("unidade desconhecida: {unidade}")))?;
    c.unidade = unidade.to_string();
    let elementos = || raiz.descendants().filter(|n| n.is_element());

    let mut cores: HashMap<(usize, usize), usize> = HashMap::new();
    for g in elementos().filter(|n| matches!(n.tag_name().name(), "basematerials" | "colorgroup")) {
        let Some(id) = inteiro(g, "id") else { continue };
        for (i, e) in g.children().filter(|n| n.is_element()).enumerate() {
            let Some(cor) = e.attribute("displaycolor").or(e.attribute("color")).and_then(de_hex) else { continue };
            let nome = e.attribute("name").unwrap_or("Cor");
            cores.insert((id, i), c.material(nome, cor));
        }
    }
    c.ignorar("textura", elementos().filter(|n| n.tag_name().name() == "texture2dgroup").count());
    c.ignorar("treliça de vigas", elementos().filter(|n| n.tag_name().name() == "beamlattice").count());
    c.ignorar("pilha de fatias", elementos().filter(|n| n.tag_name().name() == "slicestack").count());

    let mut por_id: HashMap<usize, usize> = HashMap::new();
    for o in elementos().filter(|n| n.tag_name().name() == "object") {
        let Some(id) = inteiro(o, "id") else { continue };
        c.nos.push(No::novo(o.attribute("name").unwrap_or("")));
        por_id.insert(id, c.nos.len() - 1);
    }
    for o in elementos().filter(|n| n.tag_name().name() == "object") {
        let Some(no) = inteiro(o, "id").and_then(|id| por_id.get(&id).copied()) else { continue };
        for e in o.children().filter(|n| n.is_element()) {
            match e.tag_name().name() {
                "mesh" => {
                    for m in malhas_do_objeto(o, e, &cores)? {
                        c.nos[no].malhas.push(c.malhas.len());
                        c.malhas.push(m);
                    }
                }
                "components" => {
                    for k in e.children().filter(|n| n.is_element()) {
                        let Some(alvo) = inteiro(k, "objectid").and_then(|id| por_id.get(&id).copied()) else {
                            c.ignorar("componente em outro arquivo", 1);
                            continue;
                        };
                        let mut instancia = No::novo("");
                        instancia.local = ler_transformacao(k.attribute("transform"));
                        instancia.filhos = vec![alvo];
                        c.nos.push(instancia);
                        let i = c.nos.len() - 1;
                        c.nos[no].filhos.push(i);
                    }
                }
                _ => {}
            }
        }
    }
    for item in elementos().filter(|n| n.tag_name().name() == "item") {
        let Some(alvo) = inteiro(item, "objectid").and_then(|id| por_id.get(&id).copied()) else { continue };
        let mut no = No::novo("");
        no.local = ler_transformacao(item.attribute("transform"));
        no.filhos = vec![alvo];
        c.nos.push(no);
        c.raizes.push(c.nos.len() - 1);
    }
    c.escalar(mm);
    Ok(c)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn matriz_do_3mf_vai_e_volta() {
        let m = [0., -1., 0., 5., 1., 0., 0., 7., 0., 0., 1., 9.];
        assert_eq!(ler_transformacao(Some(&transformacao(&m, 1.))), m);
        // ponto (1, 0, 0) como vetor linha, pela definição do formato, vai para (0, 1, 0) + deslocamento
        let t: Vec<f64> = transformacao(&m, 1.).split(' ').map(|x| x.parse().unwrap()).collect();
        assert_eq!([t[0] + t[9], t[1] + t[10], t[2] + t[11]], [5., 8., 9.]);
    }

    #[test]
    fn cor_com_e_sem_opacidade() {
        assert_eq!(de_hex("#FF0000"), Some([1., 0., 0., 1.]));
        assert_eq!(de_hex("#00FF0080").map(|c| (c[3] * 255.).round()), Some(128.));
        assert_eq!(de_hex("azul"), None);
    }
}
