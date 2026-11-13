// SPDX-License-Identifier: LGPL-2.1-or-later
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
        r if r >= crate::papeis::PAPEL_FURO => crate::papeis::nome_furo(r).unwrap_or_default(),
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

/// A aresta de ponto médio mais perto de `p` e a distância, sem limite. Só para sugerir no erro (K-13):
/// quem escolhe a aresta é o ponto exato ou a chave, nunca a vizinha.
pub fn edge_nearest(v: &View, p: [f64; 3]) -> Option<(usize, f64)> {
    (0..v.topo.edge_mids.len().min(v.topo.edge_faces.len()))
        .filter(|e| v.topo.edge_faces[*e] != [NO_FACE, NO_FACE])
        .map(|e| (e, dist(v.topo.edge_mids[e], p)))
        .min_by(|a, b| a.1.total_cmp(&b.1))
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

/// Arestas criadas pelo comando `id` (`EdgeSel::CreatedBy`, ADR 0010 fase 2): as que ficam entre duas
/// faces dele. Quando o comando deu papéis às faces, as duas faces precisam ter papel conhecido: uma
/// face dele sem papel veio de um caminho que perdeu o histórico (a booleana que não devolveu a origem
/// marca a face como do comando) e a aresta dela não é garantidamente obra dele. Comando sem papel
/// nenhum (ferramenta que o kernel não rotula) segue só pelo dono, como antes.
pub fn arestas_criadas(v: &View, id: Id) -> Vec<usize> {
    let n = v.faces();
    let mine = |f: u32| f != NO_FACE && (f as usize) < n && v.owners[f as usize] == id;
    let com_papel = (0..n).any(|j| v.owners[j] == id && v.roles[j] >= 0);
    let conta = |f: u32| !com_papel || v.roles[f as usize] >= 0;
    (0..v.topo.edge_faces.len().min(v.topo.edge_mids.len()))
        .filter(|e| {
            let [a, b] = v.topo.edge_faces[*e];
            mine(a) && mine(b) && conta(a) && conta(b)
        })
        .collect()
}

/// Candidatas de uma face perdida, para o assistente de reparo mostrar no 3D: as do mesmo dono e
/// papel (ambígua) ou, se nenhuma, as de mesmo tipo mais perto do último ponto conhecido.
pub fn candidatas_face(v: &View, key: Option<&FaceKey>, hint: [f64; 3], max: usize) -> Vec<usize> {
    let n = v.faces();
    let mut c: Vec<usize> = match key {
        Some(k) => (0..n).filter(|j| v.is(*j, k)).collect(),
        None => vec![],
    };
    if c.is_empty() {
        c = (0..n).filter(|j| key.is_none_or(|k| kind_of(v.topo.faces[*j].kind) == k.kind)).collect();
    }
    c.sort_by(|a, b| dist(v.topo.faces[*a].point, hint).total_cmp(&dist(v.topo.faces[*b].point, hint)));
    c.truncate(max);
    c
}

/// Candidatas de uma aresta perdida: as entre as faces da chave (ambígua) ou as mais perto do ponto.
pub fn candidatas_aresta(v: &View, key: Option<&EdgeKey>, hint: [f64; 3], max: usize) -> Vec<usize> {
    let real = |e: usize| v.topo.edge_faces[e] != [NO_FACE, NO_FACE];
    let total = v.topo.edge_faces.len().min(v.topo.edge_mids.len());
    let mut c: Vec<usize> = match key {
        Some(k) => (0..total)
            .filter(|e| {
                let [p, q] = v.topo.edge_faces[*e];
                p != NO_FACE && q != NO_FACE && ((v.is(p as usize, &k.a) && v.is(q as usize, &k.b)) || (v.is(p as usize, &k.b) && v.is(q as usize, &k.a)))
            })
            .collect(),
        None => vec![],
    };
    if c.is_empty() {
        c = (0..total).filter(|e| real(*e)).collect();
    }
    c.sort_by(|a, b| dist(v.topo.edge_mids[*a], hint).total_cmp(&dist(v.topo.edge_mids[*b], hint)));
    c.truncate(max);
    c
}

#[cfg(test)]
mod tests {
    use super::*;
    use fenkra_kernel::FaceInfo;

    fn topo(n: usize, arestas: &[[u32; 2]]) -> Topology {
        let f = FaceInfo { kind: 0, dir: [0., 0., 1.], area: 1., point: [0.; 3] };
        Topology { faces: vec![f; n], edge_mids: (0..arestas.len()).map(|i| [i as f64, 0., 0.]).collect(), edge_faces: arestas.to_vec() }
    }

    #[test]
    fn criadas_pelo_comando_seguem_os_papeis() {
        // faces 0..2 do comando 7 (tampa e duas laterais), 3 da base, 4 do comando 7 sem papel
        let t = topo(5, &[[0, 1], [0, 2], [1, 2], [1, 3], [2, 4], [4, 0]]);
        let owners = [Id(7), Id(7), Id(7), Id(1), Id(7)];
        let roles = [1, 2, 3, 0, -1];
        let v = View { topo: &t, owners: &owners, roles: &roles };
        // a aresta com a base fica de fora (já existia); as da face sem papel também
        assert_eq!(arestas_criadas(&v, Id(7)), vec![0, 1, 2]);
        // sem papel nenhum: só o dono decide, como antes
        let sem = [-1; 5];
        let v = View { topo: &t, owners: &owners, roles: &sem };
        assert_eq!(arestas_criadas(&v, Id(7)), vec![0, 1, 2, 4, 5]);
    }

    #[test]
    fn candidatas_da_aresta_ambigua_sao_as_do_par_de_faces() {
        let t = topo(4, &[[0, 1], [0, 2], [1, 2], [0, 1]]);
        let owners = [Id(7); 4];
        let roles = [0, 2, 3, 1];
        let v = View { topo: &t, owners: &owners, roles: &roles };
        let k = EdgeKey { a: v.face_key(0).unwrap(), b: v.face_key(1).unwrap() };
        // duas arestas entre a tampa e a lateral 1: as duas são candidatas, a mais perto primeiro
        assert_eq!(candidatas_aresta(&v, Some(&k), [3., 0., 0.], 5), vec![3, 0]);
        assert_eq!(candidatas_aresta(&v, None, [2.1, 0., 0.], 2), vec![2, 3]);
    }
}
