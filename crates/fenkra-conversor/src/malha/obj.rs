// SPDX-License-Identifier: LGPL-2.1-or-later
//! OBJ com MTL: grupos e materiais em texto, sem unidade nem árvore. Montagem vai achatada, um `o` por
//! peça. O MTL é um arquivo ao lado, com o mesmo nome.

use super::{erro, escrever, Cena, Malha, Material, No};
use crate::ErroConv;
use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::{Component, Path, PathBuf};

/// Sem assinatura, o OBJ se reconhece pelas palavras do começo de cada linha.
pub(crate) fn parece(b: &[u8]) -> bool {
    let fatia = &b[..b.len().min(4096)];
    let t = String::from_utf8_lossy(fatia);
    let mut linhas: Vec<&str> = t.lines().collect();
    if fatia.len() < b.len() {
        linhas.pop();
    }
    let chave = |l: &str| l.split_whitespace().next().unwrap_or("").to_string();
    let conhecida = |c: &str| c.is_empty() || c.starts_with('#') || matches!(c, "v" | "vt" | "vn" | "vp" | "f" | "l" | "p" | "o" | "g" | "s" | "mtllib" | "usemtl" | "usemap");
    linhas.iter().all(|l| conhecida(&chave(l))) && linhas.iter().any(|l| chave(l) == "v")
}

/// Materiais de um MTL pelo nome. `Kd` é a cor difusa, `d` a opacidade e `Tr` o contrário dela.
fn ler_mtl(t: &str, cena: &mut Cena) -> HashMap<String, usize> {
    let mut nomes = HashMap::new();
    let mut atual: Option<usize> = None;
    for linha in t.lines() {
        let mut w = linha.split_whitespace();
        let Some(chave) = w.next() else { continue };
        let numeros: Vec<f32> = w.clone().filter_map(|x| x.parse().ok()).collect();
        match (chave, atual) {
            ("newmtl", _) => {
                let nome = w.collect::<Vec<_>>().join(" ");
                cena.materiais.push(Material::de_cor(&nome, [0.8, 0.8, 0.8, 1.]));
                atual = Some(cena.materiais.len() - 1);
                nomes.insert(nome, cena.materiais.len() - 1);
            }
            ("Kd", Some(i)) if numeros.len() >= 3 => cena.materiais[i].cor[..3].copy_from_slice(&numeros[..3]),
            ("d", Some(i)) if !numeros.is_empty() => cena.materiais[i].cor[3] = numeros[0],
            ("Tr", Some(i)) if !numeros.is_empty() => cena.materiais[i].cor[3] = 1. - numeros[0],
            ("Pm", Some(i)) if !numeros.is_empty() => cena.materiais[i].metal = numeros[0],
            ("Pr", Some(i)) if !numeros.is_empty() => cena.materiais[i].rugosidade = numeros[0],
            (c, Some(_)) if c.starts_with("map_") => cena.ignorar("textura", 1),
            _ => {}
        }
    }
    nomes
}

/// Só arquivo ao lado: um arquivo hostil não pode apontar para fora da pasta e virar leitura de
/// qualquer arquivo da máquina.
pub(crate) fn ao_lado(caminho: &Path, nome: &str) -> Option<PathBuf> {
    let p = Path::new(nome);
    p.components().all(|c| matches!(c, Component::Normal(_))).then(|| caminho.parent().unwrap_or(Path::new("")).join(p))
}

#[derive(Default)]
struct Parte {
    m: Malha,
    /// par (vértice, normal) do arquivo para índice nesta malha
    locais: HashMap<(usize, Option<usize>), u32>,
}

fn indice(texto: &str, total: usize) -> Option<usize> {
    let i: i64 = texto.parse().ok()?;
    let r = if i < 0 { total as i64 + i } else { i - 1 };
    usize::try_from(r).ok().filter(|r| *r < total)
}

pub(crate) fn ler(b: &[u8], caminho: &Path) -> Result<Cena, ErroConv> {
    let t = String::from_utf8_lossy(b);
    let mut c = Cena::default();
    let (mut v, mut cores, mut n): (Vec<[f64; 3]>, Vec<[u8; 4]>, Vec<[f32; 3]>) = (vec![], vec![], vec![]);
    let mut materiais: HashMap<String, usize> = HashMap::new();
    // objetos em ordem; cada um com uma parte por material
    let mut objetos: Vec<(String, Vec<(Option<usize>, Parte)>)> = vec![(String::new(), vec![])];
    let mut material = None;
    let mut soltos = 0;
    for linha in t.lines() {
        let linha = linha.trim();
        let (chave, resto) = linha.split_once(char::is_whitespace).unwrap_or((linha, ""));
        let resto = resto.trim();
        match chave {
            "v" => {
                let q: Vec<f64> = resto.split_whitespace().filter_map(|x| x.parse().ok()).collect();
                if q.len() < 3 {
                    return Err(erro("OBJ", format!("vértice inválido: {linha}")));
                }
                v.push([q[0], q[1], q[2]]);
                if q.len() >= 6 {
                    cores.push([(q[3] * 255.).round() as u8, (q[4] * 255.).round() as u8, (q[5] * 255.).round() as u8, 255]);
                }
            }
            "vn" => {
                let q: Vec<f32> = resto.split_whitespace().filter_map(|x| x.parse().ok()).collect();
                n.push(if q.len() >= 3 { [q[0], q[1], q[2]] } else { [0., 0., 1.] });
            }
            "mtllib" => match ao_lado(caminho, resto).map(std::fs::read) {
                Some(Ok(bytes)) => materiais.extend(ler_mtl(&String::from_utf8_lossy(&bytes), &mut c)),
                _ => c.ignorar("biblioteca de materiais não encontrada", 1),
            },
            "usemtl" => {
                material = materiais.get(resto).copied();
                if material.is_none() {
                    c.ignorar("material sem definição", 1);
                }
            }
            "o" | "g" => {
                if objetos.last().is_some_and(|(_, p)| p.is_empty()) {
                    objetos.pop();
                }
                objetos.push((resto.to_string(), vec![]));
            }
            "f" => {
                let Some((_, partes)) = objetos.last_mut() else { continue };
                let k = match partes.iter().position(|(m, _)| *m == material) {
                    Some(k) => k,
                    None => {
                        partes.push((material, Parte::default()));
                        partes.len() - 1
                    }
                };
                let parte = &mut partes[k].1;
                parte.m.material = material;
                let mut face = Vec::new();
                for canto in resto.split_whitespace() {
                    let mut pedacos = canto.split('/');
                    let i = pedacos.next().and_then(|x| indice(x, v.len())).ok_or_else(|| erro("OBJ", format!("face com vértice inexistente: {linha}")))?;
                    let j = pedacos.nth(1).and_then(|x| indice(x, n.len()));
                    let m = &mut parte.m;
                    let local = *parte.locais.entry((i, j)).or_insert_with(|| {
                        m.pos.push(v[i]);
                        if let Some(q) = j {
                            m.nrm.push(n[q]);
                        }
                        if let Some(cor) = cores.get(i) {
                            m.cor_v.push(*cor);
                        }
                        (m.pos.len() - 1) as u32
                    });
                    face.push(local);
                }
                for k in 1..face.len().saturating_sub(1) {
                    parte.m.tris.push([face[0], face[k], face[k + 1]]);
                }
            }
            "l" | "p" => soltos += 1,
            _ => {}
        }
    }
    c.ignorar("linha ou ponto solto", soltos);
    for (nome, partes) in objetos {
        if partes.is_empty() {
            continue;
        }
        let mut no = No::novo(&nome);
        for (_, mut p) in partes {
            // parte com normal em só alguns cantos não tem normal
            if p.m.nrm.len() != p.m.pos.len() {
                p.m.nrm.clear();
            }
            if p.m.cor_v.len() != p.m.pos.len() {
                p.m.cor_v.clear();
            }
            p.m.nome.clone_from(&nome);
            p.m.conferir("OBJ")?;
            no.malhas.push(c.malhas.len());
            c.malhas.push(p.m);
        }
        c.nos.push(no);
        c.raizes.push(c.nos.len() - 1);
    }
    c.cor_uniforme_vira_material();
    Ok(c)
}

pub(crate) fn gravar(cena: &Cena, caminho: &Path, mm: f64, normais: bool) -> Result<(), ErroConv> {
    let mtl = caminho.with_extension("mtl");
    let nome_mtl = mtl.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let mut s = String::from("# Fenkra\n");
    let nome_material = |i: usize| format!("Material_{}", i + 1);
    if !cena.materiais.is_empty() {
        let _ = writeln!(s, "mtllib {nome_mtl}");
    }
    let (mut base, mut base_n) = (1usize, 1usize);
    for m in cena.no_lugar() {
        let _ = writeln!(s, "o {}", m.nome.replace(['\n', '\r'], " "));
        if let Some(k) = m.material {
            let _ = writeln!(s, "usemtl {}", nome_material(k));
        }
        for (i, p) in m.pos.iter().enumerate() {
            let q = p.map(|v| v / mm);
            match m.cor_v.get(i) {
                Some(c) => {
                    let _ = writeln!(s, "v {} {} {} {} {} {}", q[0], q[1], q[2], f32::from(c[0]) / 255., f32::from(c[1]) / 255., f32::from(c[2]) / 255.);
                }
                None => {
                    let _ = writeln!(s, "v {} {} {}", q[0], q[1], q[2]);
                }
            }
        }
        let com_normal = normais && m.nrm.len() == m.pos.len();
        if com_normal {
            for n in &m.nrm {
                let _ = writeln!(s, "vn {} {} {}", n[0], n[1], n[2]);
            }
        }
        for t in &m.tris {
            let [a, b, c] = t.map(|i| i as usize + base);
            if com_normal {
                let [x, y, z] = t.map(|i| i as usize + base_n);
                let _ = writeln!(s, "f {a}//{x} {b}//{y} {c}//{z}");
            } else {
                let _ = writeln!(s, "f {a} {b} {c}");
            }
        }
        base += m.pos.len();
        if com_normal {
            base_n += m.nrm.len();
        }
    }
    escrever(caminho, s.as_bytes())?;
    if cena.materiais.is_empty() {
        return Ok(());
    }
    let mut t = String::from("# Fenkra\n");
    for (i, k) in cena.materiais.iter().enumerate() {
        let _ = writeln!(t, "newmtl {}\nKd {} {} {}\nd {}\nPm {}\nPr {}\n", nome_material(i), k.cor[0], k.cor[1], k.cor[2], k.cor[3], k.metal, k.rugosidade);
    }
    escrever(&mtl, t.as_bytes())
}
