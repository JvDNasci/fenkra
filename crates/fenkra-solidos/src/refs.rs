//! Referências v2 (ADR 0010): face e aresta escolhidas pelo usuário viram uma consulta em camadas,
//! resolvida a cada recompute — dono (comando que gerou a face), papel (o que ela é para o dono),
//! assinatura geométrica e, só como reserva, o ponto clicado.
//!
//! Aqui ficam as funções puras sobre a topologia do corpo atual; o recompute (lib.rs) as chama
//! antes de entregar pontos ao kernel.

use fenkra_core::{EdgeKey, FaceKey, Id};
use fenkra_kernel::{Topology, NO_FACE};

/// Corpo visto pelas referências: topologia e dono/papel de cada face (mesma ordem de faces).
pub struct View<'a> {
    pub topo: &'a Topology,
    pub owners: &'a [Id],
    pub roles: &'a [i32],
}

/// Por que a consulta não resolveu para exatamente uma entidade.
#[derive(Debug, PartialEq, Eq)]
pub enum Lost {
    /// nenhuma candidata
    None,
    /// várias candidatas que a assinatura e o ponto não separam
    Ambiguous(usize),
}

/// 0 plano, 1 cilindro, 2 outro (o tipo do kernel tem mais classes).
pub fn kind_of(raw: u8) -> u8 {
    match raw {
        0 => 0,
        1 => 1,
        _ => 2,
    }
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|i| (a[i] - b[i]).powi(2)).sum::<f64>().sqrt()
}

/// Nome do papel para mensagens.
pub fn role_name(role: i32) -> String {
    match role {
        0 => "tampa inicial".into(),
        1 => "tampa final".into(),
        r if r >= 2 => format!("lateral {}", r - 1),
        _ => "sem papel".into(),
    }
}

impl View<'_> {
    fn faces(&self) -> usize {
        self.topo.faces.len().min(self.owners.len()).min(self.roles.len())
    }

    /// A face `j` é do dono e tem o papel da chave?
    fn is(&self, j: usize, key: &FaceKey) -> bool {
        j < self.faces() && self.owners[j] == key.owner && self.roles[j] == key.role
    }

    /// Mesmo tipo e, para plano, mesma normal (para eixo, o sentido não importa).
    fn same_shape(&self, j: usize, key: &FaceKey) -> bool {
        let f = &self.topo.faces[j];
        if kind_of(f.kind) != key.kind {
            return false;
        }
        let d = dot(f.dir, key.normal_or_axis);
        match key.kind {
            0 => d > 0.99,
            1 => d.abs() > 0.99,
            _ => true,
        }
    }

    fn area_diff(&self, j: usize, key: &FaceKey) -> f64 {
        (self.topo.faces[j].area - key.area).abs() / key.area.abs().max(1e-9)
    }

    /// Chave v2 da face `j`; `None` se o papel é desconhecido (não há como achá-la depois).
    pub fn face_key(&self, j: usize) -> Option<FaceKey> {
        if j >= self.faces() || self.roles[j] < 0 {
            return None;
        }
        let f = &self.topo.faces[j];
        Some(FaceKey { owner: self.owners[j], role: self.roles[j], kind: kind_of(f.kind), normal_or_axis: f.dir, area: f.area })
    }

    /// Chave v2 da aresta `e`: as duas faces vizinhas (ambas com papel conhecido).
    pub fn edge_key(&self, e: usize) -> Option<EdgeKey> {
        let [p, q] = *self.topo.edge_faces.get(e)?;
        if p == NO_FACE || q == NO_FACE {
            return None;
        }
        Some(EdgeKey { a: self.face_key(p as usize)?, b: self.face_key(q as usize)? })
    }
}

/// Fica com o melhor só se ele for claramente melhor que o segundo (nunca escolhe no empate).
fn clear_best(mut scored: Vec<(usize, f64)>) -> Option<usize> {
    scored.sort_by(|a, b| a.1.total_cmp(&b.1));
    match scored.as_slice() {
        [] => None,
        [one] => Some(one.0),
        [a, b, ..] => (b.1 - a.1 > 1e-9 && a.1 <= 0.5 * b.1).then_some(a.0),
    }
}

/// Filtra `c` por `keep`, mas só se sobrar alguma (o filtro desempata, não elimina tudo).
fn narrow(c: &mut Vec<usize>, keep: impl Fn(usize) -> bool) {
    if c.len() > 1 {
        let kept: Vec<usize> = c.iter().copied().filter(|j| keep(*j)).collect();
        if !kept.is_empty() {
            *c = kept;
        }
    }
}

/// Acha a face da chave: mesmo dono e papel; se sobrar mais de uma, desempata por tipo,
/// normal/eixo, área e, por último, pelo ponto clicado.
pub fn find_face(v: &View, key: &FaceKey, hint: Option<[f64; 3]>) -> Result<usize, Lost> {
    let mut c: Vec<usize> = (0..v.faces()).filter(|j| v.is(*j, key)).collect();
    narrow(&mut c, |j| v.same_shape(j, key));
    match c.len() {
        0 => return Err(Lost::None),
        1 => return Ok(c[0]),
        _ => {}
    }
    if let Some(j) = clear_best(c.iter().map(|j| (*j, v.area_diff(*j, key))).collect()) {
        return Ok(j);
    }
    if let Some(h) = hint {
        if let Some(j) = clear_best(c.iter().map(|j| (*j, dist(v.topo.faces[*j].point, h))).collect()) {
            return Ok(j);
        }
    }
    Err(Lost::Ambiguous(c.len()))
}

/// Acha a aresta entre as duas faces da chave (mesmas regras de desempate de `find_face`).
pub fn find_edge(v: &View, key: &EdgeKey, hint: Option<[f64; 3]>) -> Result<usize, Lost> {
    let pair = |e: usize| -> Option<(usize, usize)> {
        let [p, q] = v.topo.edge_faces[e];
        if p == NO_FACE || q == NO_FACE {
            return None;
        }
        let (p, q) = (p as usize, q as usize);
        if v.is(p, &key.a) && v.is(q, &key.b) {
            Some((p, q))
        } else if v.is(p, &key.b) && v.is(q, &key.a) {
            Some((q, p))
        } else {
            None
        }
    };
    // (aresta, face que faz o papel de `a`, face que faz o papel de `b`)
    let mut c: Vec<(usize, usize, usize)> = (0..v.topo.edge_faces.len()).filter_map(|e| pair(e).map(|(p, q)| (e, p, q))).collect();
    if c.len() > 1 {
        let kept: Vec<_> = c.iter().copied().filter(|(_, p, q)| v.same_shape(*p, &key.a) && v.same_shape(*q, &key.b)).collect();
        if !kept.is_empty() {
            c = kept;
        }
    }
    match c.len() {
        0 => return Err(Lost::None),
        1 => return Ok(c[0].0),
        _ => {}
    }
    if let Some(e) = clear_best(c.iter().map(|(e, p, q)| (*e, v.area_diff(*p, &key.a) + v.area_diff(*q, &key.b))).collect()) {
        return Ok(e);
    }
    if let Some(h) = hint {
        if let Some(e) = clear_best(c.iter().map(|(e, _, _)| (*e, dist(v.topo.edge_mids[*e], h))).collect()) {
            return Ok(e);
        }
    }
    Err(Lost::Ambiguous(c.len()))
}

/// Chave de uma face que só tinha o ponto, se ela resolver sozinha (sem o ponto) para a mesma face.
pub fn promote_face(v: &View, j: usize) -> Option<FaceKey> {
    let key = v.face_key(j)?;
    (find_face(v, &key, None) == Ok(j)).then_some(key)
}

/// Chave de uma aresta que só tinha o ponto, se ela resolver sozinha para a mesma aresta.
pub fn promote_edge(v: &View, e: usize) -> Option<EdgeKey> {
    let key = v.edge_key(e)?;
    (find_edge(v, &key, None) == Ok(e)).then_some(key)
}

/// Aresta com ponto médio mais perto de `p`, dentro de `tol` (a mesma escolha que o kernel faz).
pub fn edge_near(v: &View, p: [f64; 3], tol: f64) -> Option<usize> {
    // costuras e arestas degeneradas vêm com ponto médio zerado e sem faces vizinhas: ficam de fora
    let (e, d) = (0..v.topo.edge_mids.len().min(v.topo.edge_faces.len()))
        .filter(|e| v.topo.edge_faces[*e] != [NO_FACE, NO_FACE])
        .map(|e| (e, dist(v.topo.edge_mids[e], p)))
        .min_by(|a, b| a.1.total_cmp(&b.1))?;
    (d <= tol).then_some(e)
}
