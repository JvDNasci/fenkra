// SPDX-License-Identifier: LGPL-2.1-or-later
//! Referências por geometria (ADR 0010). O arquivo diz "a aresta que passa por este ponto", e o ponto
//! pode ser qualquer um sobre ela: o programa de origem não sabe qual é o nosso ponto médio. O recálculo
//! só aceita o ponto médio a até 0,1 mm, então a leitura acha a aresta e troca o ponto pelo médio. Dali
//! em diante a referência é igual à de um clique, e o primeiro recálculo dá a ela dono e papel.
//!
//! Vale a regra da ADR: sem uma aresta claramente mais perto, nada é escolhido. O ponto fica como veio
//! e o comando sai em "referência perdida".

use fenkra_core::{Document, EdgeKey, EdgeSel, FaceKey, FeatureKind, Id};
use fenkra_kernel::NO_FACE;
use fenkra_solidos::{recompute, recompute_cached, BodyResult, Cache, Promoted, Status};

/// Distância máxima do ponto à aresta. A mesma tolerância do recálculo, medida agora na curva inteira.
const TOLERANCIA: f64 = 0.1;
/// Abaixo disso o ponto já é o médio; trocar só traria o arredondamento da malha.
const JA_NO_MEIO: f64 = 1e-3;

/// O que aconteceu com as referências de uma peça lida.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Reancoragem {
    /// pontos trocados pelo médio da aresta achada
    pub reancoradas: usize,
    /// comandos em "referência perdida", com a mensagem
    pub perdidas: Vec<(String, String)>,
}

use serde::{Deserialize, Serialize};

fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|i| (a[i] - b[i]).powi(2)).sum::<f64>().sqrt()
}

fn dist_ao_segmento(p: [f64; 3], a: [f64; 3], b: [f64; 3]) -> f64 {
    let ab = [0, 1, 2].map(|i| b[i] - a[i]);
    let quadrado: f64 = ab.iter().map(|x| x * x).sum();
    if quadrado < 1e-18 {
        return dist(p, a);
    }
    let t = ((0..3).map(|i| (p[i] - a[i]) * ab[i]).sum::<f64>() / quadrado).clamp(0., 1.);
    dist(p, [0, 1, 2].map(|i| a[i] + t * ab[i]))
}

fn meio(b: &BodyResult, e: usize) -> [f64; 3] {
    [0, 1, 2].map(|i| b.mesh.edge_mids[e * 3 + i] as f64)
}

/// Distância de `p` à polilinha da aresta `e`.
fn dist_a_aresta(b: &BodyResult, e: usize, p: [f64; 3]) -> f64 {
    let m = &b.mesh;
    let inicio = m.edge_ranges[e] as usize;
    let fim = m.edge_ranges.get(e + 1).map_or(m.edges.len() / 6, |x| *x as usize);
    m.edges[inicio * 6..fim * 6]
        .chunks_exact(6)
        .map(|s| dist_ao_segmento(p, [s[0] as f64, s[1] as f64, s[2] as f64], [s[3] as f64, s[4] as f64, s[5] as f64]))
        .fold(f64::INFINITY, f64::min)
}

/// Aresta que passa por `p`. Um ponto num vértice fica perto de duas, e aí não há o que decidir.
fn aresta_por_ponto(b: &BodyResult, p: [f64; 3]) -> Option<usize> {
    let mut perto: Vec<(usize, f64)> = (0..b.mesh.edge_ranges.len())
        .filter(|e| b.mesh.edge_faces.get(*e).is_some_and(|f| f[0] != NO_FACE && f[1] != NO_FACE))
        .map(|e| (e, dist_a_aresta(b, e, p)))
        .filter(|(_, d)| *d <= TOLERANCIA)
        .collect();
    perto.sort_by(|x, y| x.1.total_cmp(&y.1));
    match perto.as_slice() {
        [] => None,
        [um] => Some(um.0),
        [a, c, ..] => (c.1 - a.1 > 1e-6 && a.1 <= 0.5 * c.1).then_some(a.0),
    }
}

/// Corpo do coletor como estava antes do comando `i`.
fn corpo_antes(doc: &Document, i: usize, cache: &mut Cache) -> Option<BodyResult> {
    let alvo = doc.features[i].target;
    let mut d = doc.clone();
    d.rollback = i;
    recompute_cached(&d, cache).bodies.into_iter().find(|b| b.collector == alvo)
}

fn mesmo_papel(a: &FaceKey, b: &FaceKey) -> bool {
    a.owner == b.owner && a.role == b.role
}

fn mesma_aresta(a: &EdgeKey, b: &EdgeKey) -> bool {
    (mesmo_papel(&a.a, &b.a) && mesmo_papel(&a.b, &b.b)) || (mesmo_papel(&a.a, &b.b) && mesmo_papel(&a.b, &b.a))
}

/// Grava no documento as chaves que o recálculo calculou, como a tela faz depois de cada cálculo.
fn gravar_chaves(doc: &mut Document, chaves: &std::collections::HashMap<Id, Promoted>) {
    for f in &mut doc.features {
        match (&mut f.kind, chaves.get(&f.id)) {
            (FeatureKind::Fillet(o) | FeatureKind::Chamfer(o), Some(Promoted::Edges(k))) => o.keys = k.clone(),
            (FeatureKind::Shell(s), Some(Promoted::Faces(k))) if s.faces.len() == k.len() => s.face_keys = k.clone(),
            _ => {}
        }
    }
}

fn perdidas(doc: &Document) -> (Vec<(String, String)>, std::collections::HashMap<Id, Promoted>) {
    let r = recompute(doc);
    let lista = doc
        .features
        .iter()
        .filter_map(|f| match r.status.get(&f.id) {
            Some(Status::Error(m)) if m.starts_with("referência perdida") => Some((f.name.clone(), m.clone())),
            _ => None,
        })
        .collect();
    (lista, r.promoted)
}

/// Leva os pontos de aresta para o médio da aresta achada e grava as chaves.
pub fn reancorar(doc: &mut Document) -> Reancoragem {
    let mut cache = Cache::default();
    let mut reancoradas = 0;
    for i in 0..doc.features.len() {
        let (FeatureKind::Fillet(o) | FeatureKind::Chamfer(o)) = &doc.features[i].kind else { continue };
        let EdgeSel::Edges(pontos) = &o.edges else { continue };
        let mut novos = pontos.clone();
        let Some(b) = corpo_antes(doc, i, &mut cache) else { continue };
        for p in &mut novos {
            let Some(e) = aresta_por_ponto(&b, *p) else { continue };
            let m = meio(&b, e);
            if dist(m, *p) > JA_NO_MEIO {
                *p = m;
                reancoradas += 1;
            }
        }
        if let FeatureKind::Fillet(o) | FeatureKind::Chamfer(o) = &mut doc.features[i].kind {
            o.edges = EdgeSel::Edges(novos);
        }
    }
    let (lista, chaves) = perdidas(doc);
    gravar_chaves(doc, &chaves);
    Reancoragem { reancoradas, perdidas: lista }
}

/// O ponto guardado é o do clique: depois de mudar uma cota ele fica para trás e só a chave segue a
/// aresta. O arquivo não leva chaves, então antes de exportar o ponto volta para onde a aresta está.
pub fn atualizar_pontos(doc: &mut Document) {
    let mut cache = Cache::default();
    for i in 0..doc.features.len() {
        match &doc.features[i].kind {
            FeatureKind::Fillet(o) | FeatureKind::Chamfer(o) if o.keys.iter().any(Option::is_some) => {
                let (EdgeSel::Edges(pontos), chaves) = (&o.edges, o.keys.clone()) else { continue };
                let mut novos = pontos.clone();
                let Some(b) = corpo_antes(doc, i, &mut cache) else { continue };
                for (p, k) in novos.iter_mut().zip(&chaves) {
                    let Some(k) = k else { continue };
                    let candidatas = (0..b.mesh.edge_ranges.len()).filter(|e| b.edge_key(*e).is_some_and(|c| mesma_aresta(&c, k)));
                    if let Some(e) = candidatas.min_by(|x, y| dist(meio(&b, *x), *p).total_cmp(&dist(meio(&b, *y), *p))) {
                        *p = meio(&b, e);
                    }
                }
                if let FeatureKind::Fillet(o) | FeatureKind::Chamfer(o) = &mut doc.features[i].kind {
                    o.edges = EdgeSel::Edges(novos);
                }
            }
            FeatureKind::Shell(s) if s.face_keys.iter().any(Option::is_some) => {
                let (mut novos, chaves) = (s.faces.clone(), s.face_keys.clone());
                let Some(b) = corpo_antes(doc, i, &mut cache) else { continue };
                for (p, k) in novos.iter_mut().zip(&chaves) {
                    let Some(k) = k else { continue };
                    let candidatas = (0..b.mesh.face_info.len()).filter(|j| b.face_key(*j).is_some_and(|c| mesmo_papel(&c, k)));
                    if let Some(j) = candidatas.min_by(|x, y| dist(b.mesh.face_info[*x].point, *p).total_cmp(&dist(b.mesh.face_info[*y].point, *p))) {
                        *p = b.mesh.face_info[j].point;
                    }
                }
                if let FeatureKind::Shell(s) = &mut doc.features[i].kind {
                    s.faces = novos;
                }
            }
            _ => {}
        }
    }
}
