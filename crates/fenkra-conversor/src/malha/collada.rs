// SPDX-License-Identifier: LGPL-2.1-or-later
//! Collada 1.4: XML com bibliotecas de geometria, material e nós. Declara unidade e eixo de cima, então
//! a gravação fica em Z para cima, na unidade pedida. Cada nó da cena vira um nó em `library_nodes`, e
//! usar de novo é um `instance_node`: a peça repetida não se repete no arquivo.

use super::{erro, xml, Cena, Malha, Material, No};
use crate::ErroConv;
use std::collections::HashMap;
use std::fmt::Write as _;

/// Y para cima a Z para cima, por linhas: (x, y, z) -> (x, -z, y).
const Y_PARA_Z: [f64; 12] = [1., 0., 0., 0., 0., 0., -1., 0., 0., 1., 0., 0.];
/// X para cima a Z para cima: direita é -Y e profundidade é +Z, então (x, y, z) -> (-y, -z, x).
const X_PARA_Z: [f64; 12] = [0., -1., 0., 0., 0., 0., -1., 0., 1., 0., 0., 0.];

fn numeros(v: impl Iterator<Item = String>) -> String {
    v.collect::<Vec<_>>().join(" ")
}

pub(crate) fn gravar(cena: &Cena, mm: f64, unidade: &str) -> String {
    let mut c = cena.clone();
    c.assentar();
    c.escalar(1. / mm);
    let mut s = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<COLLADA xmlns=\"http://www.collada.org/2005/11/COLLADASchema\" version=\"1.4.1\">\n");
    let _ = writeln!(s, "<asset><contributor><authoring_tool>Fenkra</authoring_tool></contributor><unit name=\"{unidade}\" meter=\"{}\"/><up_axis>Z_UP</up_axis></asset>", mm / 1000.);
    s.push_str("<library_effects>\n");
    for (i, k) in c.materiais.iter().enumerate() {
        let _ = writeln!(s, "<effect id=\"efeito{i}\"><profile_COMMON><technique sid=\"comum\"><phong><diffuse><color>{} {} {} {}</color></diffuse><transparency><float>{}</float></transparency></phong></technique></profile_COMMON></effect>", k.cor[0], k.cor[1], k.cor[2], k.cor[3], k.cor[3]);
    }
    s.push_str("</library_effects>\n<library_materials>\n");
    for (i, k) in c.materiais.iter().enumerate() {
        let _ = writeln!(s, "<material id=\"material{i}\" name=\"{}\"><instance_effect url=\"#efeito{i}\"/></material>", xml(&k.nome));
    }
    s.push_str("</library_materials>\n<library_geometries>\n");
    for (i, m) in c.malhas.iter().enumerate() {
        let _ = write!(s, "<geometry id=\"geo{i}\" name=\"{}\"><mesh>", xml(&m.nome));
        let _ = write!(s, "<source id=\"geo{i}p\"><float_array id=\"geo{i}pa\" count=\"{}\">{}</float_array><technique_common><accessor source=\"#geo{i}pa\" count=\"{}\" stride=\"3\"><param name=\"X\" type=\"float\"/><param name=\"Y\" type=\"float\"/><param name=\"Z\" type=\"float\"/></accessor></technique_common></source>", m.pos.len() * 3, numeros(m.pos.iter().flatten().map(|v| v.to_string())), m.pos.len());
        let _ = write!(s, "<vertices id=\"geo{i}v\"><input semantic=\"POSITION\" source=\"#geo{i}p\"/></vertices>");
        let material = if m.material.is_some() { " material=\"m\"" } else { "" };
        let _ = writeln!(s, "<triangles{material} count=\"{}\"><input semantic=\"VERTEX\" source=\"#geo{i}v\" offset=\"0\"/><p>{}</p></triangles></mesh></geometry>", m.tris.len(), numeros(m.tris.iter().flatten().map(|v| v.to_string())));
    }
    s.push_str("</library_geometries>\n<library_nodes>\n");
    for (i, n) in c.nos.iter().enumerate() {
        let l = &n.local;
        let _ = write!(s, "<node id=\"no{i}\" name=\"{}\"><matrix>{}</matrix>", xml(&n.nome), numeros(l.iter().map(|v| v.to_string()).chain(["0 0 0 1".to_string()])));
        for &m in &n.malhas {
            let _ = write!(s, "<instance_geometry url=\"#geo{m}\">");
            if let Some(k) = c.malhas[m].material {
                let _ = write!(s, "<bind_material><technique_common><instance_material symbol=\"m\" target=\"#material{k}\"/></technique_common></bind_material>");
            }
            s.push_str("</instance_geometry>");
        }
        for &f in &n.filhos {
            let _ = write!(s, "<instance_node url=\"#no{f}\"/>");
        }
        s.push_str("</node>\n");
    }
    let _ = writeln!(s, "</library_nodes>\n<library_visual_scenes><visual_scene id=\"cena\" name=\"{}\">", xml(&c.nome));
    for &r in &c.raizes {
        let _ = writeln!(s, "<node id=\"raiz{r}\"><instance_node url=\"#no{r}\"/></node>");
    }
    s.push_str("</visual_scene></library_visual_scenes>\n<scene><instance_visual_scene url=\"#cena\"/></scene>\n</COLLADA>\n");
    s
}

fn filhos<'a, 'i: 'a>(n: roxmltree::Node<'a, 'i>, nome: &'a str) -> impl Iterator<Item = roxmltree::Node<'a, 'i>> + 'a {
    n.children().filter(move |c| c.tag_name().name() == nome)
}

fn lista_de(t: Option<&str>) -> Vec<f64> {
    t.unwrap_or("").split_whitespace().filter_map(|x| x.parse().ok()).collect()
}

fn alvo(t: Option<&str>) -> &str {
    t.unwrap_or("").trim_start_matches('#')
}

/// Triângulos de um grupo `triangles`, `polylist` ou `polygons`, como índices de posição.
fn triangulos_do_grupo(g: roxmltree::Node) -> Result<Vec<[u32; 3]>, ErroConv> {
    let entradas: Vec<_> = filhos(g, "input").collect();
    let passo = entradas.iter().filter_map(|i| i.attribute("offset")?.parse::<usize>().ok()).max().unwrap_or(0) + 1;
    let vertice = entradas.iter().find(|i| i.attribute("semantic") == Some("VERTEX")).and_then(|i| i.attribute("offset")?.parse::<usize>().ok()).unwrap_or(0);
    let cantos = |p: roxmltree::Node| -> Vec<u32> { lista_de(p.text()).chunks_exact(passo).map(|c| c[vertice] as u32).collect() };
    let mut saida = Vec::new();
    let leque = |f: &[u32], saida: &mut Vec<[u32; 3]>| {
        for k in 1..f.len().saturating_sub(1) {
            saida.push([f[0], f[k], f[k + 1]]);
        }
    };
    match g.tag_name().name() {
        "triangles" => {
            for p in filhos(g, "p") {
                saida.extend(cantos(p).chunks_exact(3).map(|t| [t[0], t[1], t[2]]));
            }
        }
        "polylist" => {
            let contagens = lista_de(filhos(g, "vcount").next().and_then(|v| v.text()));
            let todos = filhos(g, "p").next().map(cantos).unwrap_or_default();
            let mut i = 0;
            for n in contagens {
                let n = n as usize;
                let face = todos.get(i..i + n).ok_or_else(|| erro("Collada", "polilista cortada"))?;
                leque(face, &mut saida);
                i += n;
            }
        }
        _ => {
            for p in filhos(g, "p") {
                leque(&cantos(p), &mut saida);
            }
        }
    }
    Ok(saida)
}

/// Uma geometria: posições e, por grupo, o símbolo do material e os triângulos.
struct Geometria {
    nome: String,
    pos: Vec<[f64; 3]>,
    grupos: Vec<(String, Vec<[u32; 3]>)>,
}

fn ler_geometria(g: roxmltree::Node) -> Result<Option<Geometria>, ErroConv> {
    let Some(malha) = filhos(g, "mesh").next() else { return Ok(None) };
    let vertices = filhos(malha, "vertices").next().ok_or_else(|| erro("Collada", "malha sem vértices"))?;
    let fonte = filhos(vertices, "input").find(|i| i.attribute("semantic") == Some("POSITION")).map(|i| alvo(i.attribute("source")).to_string()).unwrap_or_default();
    let origem = filhos(malha, "source").find(|s| s.attribute("id") == Some(fonte.as_str())).ok_or_else(|| erro("Collada", "posições não encontradas"))?;
    let numeros = lista_de(filhos(origem, "float_array").next().and_then(|a| a.text()));
    let passo = origem.descendants().find(|n| n.tag_name().name() == "accessor").and_then(|a| a.attribute("stride")?.parse().ok()).unwrap_or(3usize).max(3);
    let pos = numeros.chunks_exact(passo).map(|p| [p[0], p[1], p[2]]).collect();
    let mut grupos = Vec::new();
    for e in malha.children().filter(|n| matches!(n.tag_name().name(), "triangles" | "polylist" | "polygons")) {
        grupos.push((e.attribute("material").unwrap_or("").to_string(), triangulos_do_grupo(e)?));
    }
    Ok(Some(Geometria { nome: g.attribute("name").unwrap_or("").to_string(), pos, grupos }))
}

/// Matriz local: a composição, em ordem, de `matrix`, `translate`, `rotate` e `scale`.
fn matriz_do_no(n: roxmltree::Node) -> [f64; 12] {
    use crate::modelo::compor;
    let mut m = fenkra_kernel::IDENTIDADE;
    for e in n.children().filter(|c| c.is_element()) {
        let v = lista_de(e.text());
        let t = match (e.tag_name().name(), v.as_slice()) {
            ("matrix", x) if x.len() == 16 => [x[0], x[1], x[2], x[3], x[4], x[5], x[6], x[7], x[8], x[9], x[10], x[11]],
            ("translate", [x, y, z]) => [1., 0., 0., *x, 0., 1., 0., *y, 0., 0., 1., *z],
            ("scale", [x, y, z]) => [*x, 0., 0., 0., 0., *y, 0., 0., 0., 0., *z, 0.],
            ("rotate", [x, y, z, a]) => {
                let n = (x * x + y * y + z * z).sqrt().max(1e-300);
                let (x, y, z) = (x / n, y / n, z / n);
                let (s, c) = a.to_radians().sin_cos();
                let k = 1. - c;
                [k * x * x + c, k * x * y - s * z, k * x * z + s * y, 0., k * x * y + s * z, k * y * y + c, k * y * z - s * x, 0., k * x * z - s * y, k * y * z + s * x, k * z * z + c, 0.]
            }
            _ => continue,
        };
        m = compor(&m, &t);
    }
    m
}

struct Leitura<'a, 'i> {
    cena: Cena,
    geometrias: HashMap<String, Geometria>,
    /// material do arquivo pelo id, já na cena
    materiais: HashMap<String, usize>,
    /// nós por id, para `instance_node`
    por_id: HashMap<String, roxmltree::Node<'a, 'i>>,
    feitos: HashMap<roxmltree::NodeId, usize>,
    malhas: HashMap<(String, String, Option<usize>), usize>,
}

impl<'a, 'i> Leitura<'a, 'i> {
    fn malhas_da_instancia(&mut self, i: roxmltree::Node) -> Result<Vec<usize>, ErroConv> {
        let id = alvo(i.attribute("url")).to_string();
        let mut ligacoes: HashMap<String, usize> = HashMap::new();
        for b in i.descendants().filter(|n| n.tag_name().name() == "instance_material") {
            if let (Some(s), Some(k)) = (b.attribute("symbol"), self.materiais.get(alvo(b.attribute("target")))) {
                ligacoes.insert(s.to_string(), *k);
            }
        }
        let Some(g) = self.geometrias.get(&id) else { return Ok(vec![]) };
        let mut saida = Vec::new();
        for (simbolo, tris) in &g.grupos {
            let material = ligacoes.get(simbolo).copied();
            let chave = (id.clone(), simbolo.clone(), material);
            if let Some(k) = self.malhas.get(&chave) {
                saida.push(*k);
                continue;
            }
            let mut novo: HashMap<u32, u32> = HashMap::new();
            let mut m = Malha { nome: g.nome.clone(), material, ..Default::default() };
            for t in tris {
                let mut r = [0; 3];
                for (k, v) in t.iter().enumerate() {
                    let p = *g.pos.get(*v as usize).ok_or_else(|| erro("Collada", "triângulo com vértice inexistente"))?;
                    r[k] = *novo.entry(*v).or_insert_with(|| {
                        m.pos.push(p);
                        (m.pos.len() - 1) as u32
                    });
                }
                m.tris.push(r);
            }
            m.conferir("Collada")?;
            self.cena.malhas.push(m);
            self.malhas.insert(chave, self.cena.malhas.len() - 1);
            saida.push(self.cena.malhas.len() - 1);
        }
        Ok(saida)
    }

    fn no(&mut self, n: roxmltree::Node<'a, 'i>, pilha: &mut Vec<roxmltree::NodeId>) -> Result<Option<usize>, ErroConv> {
        if let Some(i) = self.feitos.get(&n.id()) {
            return Ok(Some(*i));
        }
        if pilha.contains(&n.id()) || pilha.len() > crate::modelo::PROFUNDIDADE {
            return Err(erro("Collada", "árvore com ciclo ou funda demais"));
        }
        pilha.push(n.id());
        let mut no = No::novo(n.attribute("name").unwrap_or(""));
        no.local = matriz_do_no(n);
        for e in n.children().filter(|c| c.is_element()) {
            match e.tag_name().name() {
                "instance_geometry" => no.malhas.extend(self.malhas_da_instancia(e)?),
                "instance_node" => {
                    if let Some(alvo) = self.por_id.get(alvo(e.attribute("url"))).copied() {
                        if let Some(i) = self.no(alvo, pilha)? {
                            no.filhos.push(i);
                        }
                    }
                }
                "node" => {
                    if let Some(i) = self.no(e, pilha)? {
                        no.filhos.push(i);
                    }
                }
                "instance_camera" => self.cena.ignorar("câmera", 1),
                "instance_light" => self.cena.ignorar("luz", 1),
                "instance_controller" => self.cena.ignorar("esqueleto", 1),
                _ => {}
            }
        }
        pilha.pop();
        self.cena.nos.push(no);
        self.feitos.insert(n.id(), self.cena.nos.len() - 1);
        Ok(Some(self.cena.nos.len() - 1))
    }
}

pub(crate) fn ler(b: &[u8]) -> Result<Cena, ErroConv> {
    let texto = String::from_utf8_lossy(b);
    let doc = roxmltree::Document::parse(&texto).map_err(|e| erro("Collada", e))?;
    let raiz = doc.root_element();
    let elementos = |nome: &'static str| raiz.descendants().filter(move |n| n.tag_name().name() == nome);
    let mut l = Leitura { cena: Cena::default(), geometrias: HashMap::new(), materiais: HashMap::new(), por_id: HashMap::new(), feitos: HashMap::new(), malhas: HashMap::new() };

    let mut cores: HashMap<String, [f32; 4]> = HashMap::new();
    for e in elementos("effect") {
        let Some(id) = e.attribute("id") else { continue };
        let difusa = e.descendants().find(|n| n.tag_name().name() == "diffuse");
        l.cena.ignorar("textura", usize::from(difusa.is_some_and(|d| d.descendants().any(|n| n.tag_name().name() == "texture"))));
        let cor = difusa.and_then(|d| d.descendants().find(|n| n.tag_name().name() == "color")).map(|c| lista_de(c.text()));
        if let Some([r, g, b, resto @ ..]) = cor.as_deref() {
            cores.insert(id.to_string(), [*r as f32, *g as f32, *b as f32, resto.first().copied().unwrap_or(1.) as f32]);
        }
    }
    for m in elementos("material") {
        let Some(id) = m.attribute("id") else { continue };
        let efeito = filhos(m, "instance_effect").next().map(|e| alvo(e.attribute("url")).to_string()).unwrap_or_default();
        let cor = cores.get(&efeito).copied().unwrap_or([0.8, 0.8, 0.8, 1.]);
        l.cena.materiais.push(Material::de_cor(m.attribute("name").unwrap_or(id), cor));
        l.materiais.insert(id.to_string(), l.cena.materiais.len() - 1);
    }
    for g in elementos("geometry") {
        let Some(id) = g.attribute("id") else { continue };
        if let Some(q) = ler_geometria(g)? {
            l.geometrias.insert(id.to_string(), q);
        }
    }
    l.cena.ignorar("animação", elementos("animation").count());
    for n in elementos("node") {
        if let Some(id) = n.attribute("id") {
            l.por_id.insert(id.to_string(), n);
        }
    }
    let ativa = elementos("instance_visual_scene").next().map(|e| alvo(e.attribute("url")).to_string());
    let cena = elementos("visual_scene").find(|c| ativa.as_deref().is_none_or(|a| c.attribute("id") == Some(a))).ok_or_else(|| erro("Collada", "sem cena visual"))?;
    l.cena.nome = cena.attribute("name").unwrap_or("").to_string();
    for n in filhos(cena, "node") {
        if let Some(i) = l.no(n, &mut Vec::new())? {
            l.cena.raizes.push(i);
        }
    }
    let mut c = l.cena;
    let unidade = elementos("unit").next();
    let metros: f64 = unidade.and_then(|u| u.attribute("meter")?.parse().ok()).unwrap_or(1.);
    c.unidade = unidade.and_then(|u| u.attribute("name")).unwrap_or("METRE").to_string();
    match elementos("up_axis").next().and_then(|e| e.text()).map(str::trim) {
        Some("Z_UP") => {}
        Some("X_UP") => c.embrulhar(X_PARA_Z),
        _ => c.embrulhar(Y_PARA_Z),
    }
    c.escalar(metros * 1000.);
    Ok(c)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn polilista_com_quadrado_vira_dois_triangulos_e_y_para_cima_gira() {
        let t = r##"<COLLADA><asset><unit meter="1"/><up_axis>Y_UP</up_axis></asset>
        <library_geometries><geometry id="g"><mesh>
          <source id="p"><float_array id="a" count="12">0 0 0 1 0 0 1 1 0 0 1 0</float_array></source>
          <vertices id="v"><input semantic="POSITION" source="#p"/></vertices>
          <polylist count="1"><input semantic="VERTEX" source="#v" offset="0"/><input semantic="NORMAL" source="#n" offset="1"/><vcount>4</vcount><p>0 0 1 0 2 0 3 0</p></polylist>
        </mesh></geometry></library_geometries>
        <library_visual_scenes><visual_scene id="c"><node><translate>0 2 0</translate><instance_geometry url="#g"/></node></visual_scene></library_visual_scenes>
        <scene><instance_visual_scene url="#c"/></scene></COLLADA>"##;
        let mut c = ler(t.as_bytes()).unwrap();
        c.enxugar();
        let m = &c.no_lugar()[0];
        assert_eq!(m.tris.len(), 2);
        // Y do arquivo vira Z, em milímetros
        assert_eq!(m.pos[0], [0., 0., 2000.]);
    }

    #[test]
    fn no_que_se_instancia_e_erro_e_nao_trava() {
        let t = r##"<COLLADA><library_visual_scenes><visual_scene id="c"><node id="a"><instance_node url="#a"/></node></visual_scene></library_visual_scenes></COLLADA>"##;
        assert!(ler(t.as_bytes()).is_err());
    }
}
