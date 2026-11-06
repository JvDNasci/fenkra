// SPDX-License-Identifier: LGPL-2.1-or-later
//! X3D na codificação XML: os mesmos nós do VRML, com campos em atributos. O campo de um filho vem do
//! tipo dele, a não ser que `containerField` diga outro.

use super::nos3d::{cena_de, eixo_angulo, em_metros, nomes_def, No3d, Valor};
use super::{erro, xml, Cena};
use crate::modelo::PROFUNDIDADE;
use crate::ErroConv;
use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::rc::Rc;

fn campo_do_filho(n: roxmltree::Node) -> String {
    if let Some(c) = n.attribute("containerField") {
        return c.to_string();
    }
    match n.tag_name().name() {
        "Appearance" => "appearance",
        "Material" => "material",
        "Coordinate" | "CoordinateDouble" => "coord",
        "Color" | "ColorRGBA" => "color",
        "Normal" => "normal",
        "ImageTexture" | "PixelTexture" | "MovieTexture" => "texture",
        "TextureCoordinate" => "texCoord",
        "IndexedFaceSet" | "IndexedTriangleSet" | "TriangleSet" | "Box" | "Sphere" | "Cylinder" | "Cone" | "ElevationGrid" | "Extrusion" | "Text" | "IndexedLineSet" | "LineSet" | "PointSet" => "geometry",
        _ => "children",
    }
    .to_string()
}

/// Atributo como lista de números, ou como texto quando não for.
fn valor(t: &str) -> Valor {
    let partes: Vec<&str> = t.split(|c: char| c.is_whitespace() || c == ',').filter(|p| !p.is_empty()).collect();
    let numeros: Vec<f64> = partes.iter().filter_map(|p| p.parse().ok()).collect();
    if !partes.is_empty() && numeros.len() == partes.len() {
        Valor::Numeros(numeros)
    } else {
        Valor::Texto(t.trim().to_string())
    }
}

fn no(e: roxmltree::Node, def: &mut HashMap<String, Rc<No3d>>, nivel: usize) -> Result<Option<Rc<No3d>>, ErroConv> {
    if nivel > PROFUNDIDADE {
        return Err(erro("X3D", "árvore funda demais"));
    }
    if let Some(nome) = e.attribute("USE") {
        return Ok(def.get(nome).cloned());
    }
    let mut n = No3d { tipo: e.tag_name().name().to_string(), nome: e.attribute("DEF").unwrap_or("").to_string(), ..Default::default() };
    for a in e.attributes() {
        if !matches!(a.name(), "DEF" | "USE" | "containerField") {
            n.campos.insert(a.name().to_string(), valor(a.value()));
        }
    }
    let mut filhos: HashMap<String, Vec<Rc<No3d>>> = HashMap::new();
    for f in e.children().filter(|c| c.is_element()) {
        if let Some(q) = no(f, def, nivel + 1)? {
            filhos.entry(campo_do_filho(f)).or_default().push(q);
        }
    }
    for (campo, lista) in filhos {
        n.campos.insert(campo, Valor::Nos(lista));
    }
    let n = Rc::new(n);
    if let Some(nome) = e.attribute("DEF") {
        def.insert(nome.to_string(), n.clone());
    }
    Ok(Some(n))
}

pub(crate) fn ler(b: &[u8]) -> Result<Cena, ErroConv> {
    let texto = String::from_utf8_lossy(b);
    let opcoes = roxmltree::ParsingOptions { allow_dtd: true, ..Default::default() };
    let doc = roxmltree::Document::parse_with_options(&texto, opcoes).map_err(|e| erro("X3D", e))?;
    let raiz = doc.root_element();
    let cena = raiz.children().find(|c| c.tag_name().name() == "Scene").ok_or_else(|| erro("X3D", "sem o elemento Scene"))?;
    // a declaração de unidade é do 3.3 em diante; sem ela, metros
    let unidade = raiz.descendants().find(|n| n.tag_name().name() == "unit" && n.attribute("category") == Some("length"));
    let metros = unidade.and_then(|u| u.attribute("conversionFactor")).and_then(|v| v.parse().ok()).unwrap_or(1.);
    let mut def = HashMap::new();
    let mut raizes = Vec::new();
    for f in cena.children().filter(|c| c.is_element()) {
        if let Some(n) = no(f, &mut def, 0)? {
            raizes.push(n);
        }
    }
    cena_de(&raizes, vec![], metros, "X3D")
}

struct Saida<'a> {
    c: &'a Cena,
    nomes: Vec<String>,
    nos: HashSet<usize>,
    malhas: HashSet<usize>,
    s: String,
}

impl Saida<'_> {
    fn forma(&mut self, i: usize) {
        let Some(m) = self.c.malhas.get(i) else { return };
        self.s.push_str("<Shape>");
        if let Some(k) = self.c.cor_de(m) {
            let _ = write!(self.s, r#"<Appearance><Material diffuseColor="{} {} {}" transparency="{}"/></Appearance>"#, k[0], k[1], k[2], 1. - k[3]);
        }
        if self.malhas.insert(i) {
            let faces: Vec<String> = m.tris.iter().map(|t| format!("{} {} {} -1", t[0], t[1], t[2])).collect();
            let pontos: Vec<String> = m.pos.iter().map(|p| format!("{} {} {}", p[0], p[1], p[2])).collect();
            let _ = write!(self.s, r#"<IndexedFaceSet DEF="M_{i}" solid="true" coordIndex="{}"><Coordinate point="{}"/></IndexedFaceSet>"#, faces.join(" "), pontos.join(" "));
        } else {
            let _ = write!(self.s, r#"<IndexedFaceSet USE="M_{i}"/>"#);
        }
        self.s.push_str("</Shape>\n");
    }

    fn no(&mut self, i: usize, pilha: &mut Vec<usize>) {
        if self.nos.contains(&i) {
            let _ = writeln!(self.s, r#"<Transform USE="{}"/>"#, xml(&self.nomes[i]));
            return;
        }
        if pilha.contains(&i) {
            return;
        }
        let c = self.c;
        let Some(n) = c.nos.get(i) else { return };
        let (e, l) = (eixo_angulo(&n.local), &n.local);
        let _ = writeln!(self.s, r#"<Transform DEF="{}" translation="{} {} {}" rotation="{} {} {} {}">"#, xml(&self.nomes[i]), l[3], l[7], l[11], e[0], e[1], e[2], e[3]);
        pilha.push(i);
        for &m in &n.malhas {
            self.forma(m);
        }
        for &f in &n.filhos {
            self.no(f, pilha);
        }
        pilha.pop();
        self.s.push_str("</Transform>\n");
        self.nos.insert(i);
    }
}

pub(crate) fn gravar(cena: &Cena) -> String {
    let c = em_metros(cena);
    let mut s = Saida { nomes: nomes_def(&c), c: &c, nos: HashSet::new(), malhas: HashSet::new(), s: String::new() };
    s.s.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<X3D profile=\"Interchange\" version=\"3.3\">\n<head><meta name=\"generator\" content=\"Fenkra\"/></head>\n<Scene>\n<Transform rotation=\"1 0 0 -1.5707963267948966\">\n");
    for &r in &c.raizes {
        s.no(r, &mut Vec::new());
    }
    s.s.push_str("</Transform>\n</Scene>\n</X3D>\n");
    s.s
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn triangulo_indexado_com_unidade_declarada() {
        let t = r#"<X3D><head><unit category="length" name="mm" conversionFactor="0.001"/></head><Scene>
            <Shape><IndexedTriangleSet index="0 1 2"><Coordinate point="0 0 0, 10 0 0, 0 10 0"/></IndexedTriangleSet></Shape>
        </Scene></X3D>"#;
        let c = ler(t.as_bytes()).unwrap();
        assert_eq!(c.malhas[0].tris.len(), 1);
        assert_eq!(c.malhas[0].pos[1], [10., 0., 0.], "a unidade do arquivo era milímetro");
    }

    #[test]
    fn sem_scene_e_erro() {
        assert!(ler(b"<X3D/>").is_err());
    }
}
