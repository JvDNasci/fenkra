// SPDX-License-Identifier: LGPL-2.1-or-later
//! USD em texto e USDZ, só gravação. O formato declara a unidade (`metersPerUnit`) e o eixo de cima, e
//! o Fenkra grava em Z para cima. Cada malha é uma classe em `/Prototipos`, e cada ocorrência uma
//! referência a ela: a peça repetida não se repete no arquivo. O USDZ é um ZIP sem compressão, com
//! cada arquivo alinhado em 64 bytes.

use super::{zip, Cena};
use crate::ErroConv;
use std::collections::HashSet;
use std::fmt::Write as _;

/// Nome de prim: letras, números e sublinhado, sem começar por número.
fn prim(nome: &str, i: usize) -> String {
    let limpo: String = nome.chars().map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' }).collect();
    if limpo.is_empty() || limpo.starts_with(|c: char| c.is_ascii_digit()) {
        format!("N{i}_{limpo}")
    } else {
        limpo
    }
}

fn pontos(v: impl Iterator<Item = String>) -> String {
    v.collect::<Vec<_>>().join(", ")
}

fn prototipo(s: &mut String, cena: &Cena, i: usize, mm: f64) {
    let m = &cena.malhas[i];
    let _ = writeln!(s, "    def Mesh \"M{i}\"\n    {{");
    let _ = writeln!(s, "        uniform bool doubleSided = 0\n        uniform token subdivisionScheme = \"none\"\n        uniform token orientation = \"rightHanded\"");
    let _ = writeln!(s, "        point3f[] points = [{}]", pontos(m.pos.iter().map(|p| format!("({}, {}, {})", p[0] / mm, p[1] / mm, p[2] / mm))));
    let _ = writeln!(s, "        int[] faceVertexCounts = [{}]", pontos(m.tris.iter().map(|_| "3".to_string())));
    let _ = writeln!(s, "        int[] faceVertexIndices = [{}]", pontos(m.tris.iter().flatten().map(u32::to_string)));
    if m.cor_v.len() == m.pos.len() {
        let _ = writeln!(s, "        color3f[] primvars:displayColor = [{}] (\n            interpolation = \"vertex\"\n        )", pontos(m.cor_v.iter().map(|c| format!("({}, {}, {})", f32::from(c[0]) / 255., f32::from(c[1]) / 255., f32::from(c[2]) / 255.))));
    } else if let Some(c) = cena.cor_de(m) {
        let _ = writeln!(s, "        color3f[] primvars:displayColor = [({}, {}, {})]", c[0], c[1], c[2]);
        let _ = writeln!(s, "        float[] primvars:displayOpacity = [{}]", c[3]);
    }
    if let Some(k) = m.material {
        let _ = writeln!(s, "        rel material:binding = </Materiais/Mat{k}>");
    }
    let _ = writeln!(s, "    }}");
}

fn material(s: &mut String, cena: &Cena, k: usize) {
    let m = &cena.materiais[k];
    let _ = writeln!(s, "    def Material \"Mat{k}\"\n    {{\n        token outputs:surface.connect = </Materiais/Mat{k}/Superficie.outputs:surface>");
    let _ = writeln!(s, "        def Shader \"Superficie\"\n        {{\n            uniform token info:id = \"UsdPreviewSurface\"");
    let _ = writeln!(s, "            color3f inputs:diffuseColor = ({}, {}, {})\n            float inputs:opacity = {}\n            float inputs:metallic = {}\n            float inputs:roughness = {}", m.cor[0], m.cor[1], m.cor[2], m.cor[3], m.metal, m.rugosidade);
    let _ = writeln!(s, "            token outputs:surface\n        }}\n    }}");
}

/// Matriz do USD: vetor linha, com o deslocamento na última linha.
fn matriz(m: &[f64; 12], mm: f64) -> String {
    format!("( ({}, {}, {}, 0), ({}, {}, {}, 0), ({}, {}, {}, 0), ({}, {}, {}, 1) )", m[0], m[4], m[8], m[1], m[5], m[9], m[2], m[6], m[10], m[3] / mm, m[7] / mm, m[11] / mm)
}

fn no(s: &mut String, cena: &Cena, i: usize, nivel: usize, mm: f64, irmaos: &mut HashSet<String>, pilha: &mut Vec<usize>) {
    if pilha.contains(&i) {
        return;
    }
    let Some(n) = cena.nos.get(i) else { return };
    let mut nome = prim(&n.nome, i);
    if !irmaos.insert(nome.clone()) {
        nome = format!("{nome}_{i}");
        irmaos.insert(nome.clone());
    }
    let r = "    ".repeat(nivel);
    let _ = writeln!(s, "{r}def Xform \"{nome}\"\n{r}{{");
    let _ = writeln!(s, "{r}    matrix4d xformOp:transform = {}\n{r}    uniform token[] xformOpOrder = [\"xformOp:transform\"]", matriz(&n.local, mm));
    for &m in &n.malhas {
        let _ = writeln!(s, "{r}    def \"Malha{m}\" (\n{r}        references = </Prototipos/M{m}>\n{r}    )\n{r}    {{\n{r}    }}");
    }
    pilha.push(i);
    let mut proprios = HashSet::new();
    for &f in &n.filhos {
        no(s, cena, f, nivel + 1, mm, &mut proprios, pilha);
    }
    pilha.pop();
    let _ = writeln!(s, "{r}}}");
}

pub(crate) fn gravar_texto(cena: &Cena, mm: f64) -> String {
    let mut c = cena.clone();
    c.assentar();
    let mut s = format!("#usda 1.0\n(\n    defaultPrim = \"Modelo\"\n    metersPerUnit = {}\n    upAxis = \"Z\"\n    doc = \"Fenkra\"\n)\n\n", mm / 1000.);
    if !c.materiais.is_empty() {
        s.push_str("def Scope \"Materiais\"\n{\n");
        (0..c.materiais.len()).for_each(|k| material(&mut s, &c, k));
        s.push_str("}\n\n");
    }
    s.push_str("class Scope \"Prototipos\"\n{\n");
    (0..c.malhas.len()).for_each(|i| prototipo(&mut s, &c, i, mm));
    s.push_str("}\n\ndef Xform \"Modelo\"\n{\n");
    let mut irmaos = HashSet::new();
    for &r in &c.raizes {
        no(&mut s, &c, r, 1, mm, &mut irmaos, &mut Vec::new());
    }
    s.push_str("}\n");
    s
}

pub(crate) fn gravar_pacote(cena: &Cena, mm: f64) -> Result<Vec<u8>, ErroConv> {
    let texto = gravar_texto(cena, mm);
    Ok(zip::gravar(&[("modelo.usda", texto.as_bytes())], true)?)
}

#[cfg(test)]
mod testes {
    use super::*;
    use fenkra_malha::cena::cubo;

    #[test]
    fn nome_de_prim_nao_comeca_por_numero_nem_leva_acento() {
        assert_eq!(prim("Pino 1", 0), "Pino_1");
        assert_eq!(prim("9º", 3), "N3_9_");
        assert_eq!(prim("", 2), "N2_");
    }

    #[test]
    fn unidade_e_peca_repetida_uma_vez_so() {
        let mut c = Cena::default();
        let peca = c.raiz_com(cubo("Pino", 2.));
        let mut outro = c.nos[peca].clone();
        outro.local[3] = 10.;
        c.nos.push(outro);
        c.raizes.push(1);
        let t = gravar_texto(&c, 1.);
        assert!(t.contains("metersPerUnit = 0.001"), "{t}");
        assert!(t.contains("upAxis = \"Z\""));
        assert_eq!(t.matches("def Mesh").count(), 1);
        assert_eq!(t.matches("references = </Prototipos/M0>").count(), 2);
        assert!(t.contains("(10, 0, 0, 1)"));
    }
}
