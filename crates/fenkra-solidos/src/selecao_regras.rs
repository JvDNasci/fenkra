// SPDX-License-Identifier: LGPL-2.1-or-later
//! Seleção por intenção sobre a malha do kernel (B95): faces tangentes, arestas em cadeia e
//! círculos concêntricos. A interface usa para crescer a seleção a partir do clique; o recompute usa
//! para refazer a regra gravada no filete e no chanfro, de modo que a cadeia acompanha o modelo.

use fenkra_core::{RegraAresta, RegraSelecao};
use fenkra_kernel::{EdgeFormula, Mesh, Shape, NO_FACE};

/// Tolerância (mm) para dizer que duas pontas são o mesmo ponto.
const PONTA: f64 = 1e-3;
/// Cosseno mínimo entre as direções de duas arestas para a ponta contar como tangente (~14 graus).
const COS_TANGENTE: f64 = 0.97;
/// Distância máxima (mm) de um centro ao eixo para dois círculos serem coaxiais.
const TOL_EIXO: f64 = 1e-3;

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn norma(a: [f64; 3]) -> f64 {
    dot(a, a).sqrt()
}
fn unit(a: [f64; 3]) -> [f64; 3] {
    let n = norma(a).max(1e-12);
    [a[0] / n, a[1] / n, a[2] / n]
}

/// Polilinha de uma aresta (na ordem dos segmentos da malha).
fn polilinha(m: &Mesh, e: usize) -> Vec<[f64; 3]> {
    let Some(&s0) = m.edge_ranges.get(e) else { return vec![] };
    let s1 = m.edge_ranges.get(e + 1).map(|x| *x as usize).unwrap_or(m.edges.len() / 6);
    let mut out: Vec<[f64; 3]> = vec![];
    for sg in m.edges.chunks_exact(6).take(s1).skip(s0 as usize) {
        if out.is_empty() {
            out.push([sg[0] as f64, sg[1] as f64, sg[2] as f64]);
        }
        out.push([sg[3] as f64, sg[4] as f64, sg[5] as f64]);
    }
    out
}

/// Pontas de cada aresta e a direção de saída em cada uma (apontando para fora da aresta).
struct Ponta {
    pos: [f64; 3],
    saindo: [f64; 3],
}

fn pontas(m: &Mesh, e: usize) -> Option<[Ponta; 2]> {
    let p = polilinha(m, e);
    if p.len() < 2 {
        return None;
    }
    let n = p.len();
    // a direção de saída olha o primeiro segmento que tem comprimento
    let ini = (1..n).find(|i| norma(sub(p[*i], p[0])) > 1e-9).map(|i| unit(sub(p[i], p[0])))?;
    let fim = (0..n - 1).rev().find(|i| norma(sub(p[*i], p[n - 1])) > 1e-9).map(|i| unit(sub(p[i], p[n - 1])))?;
    Some([Ponta { pos: p[0], saindo: ini }, Ponta { pos: p[n - 1], saindo: fim }])
}

fn mesmo_ponto(a: [f64; 3], b: [f64; 3]) -> bool {
    norma(sub(a, b)) < PONTA
}

/// Faces alcançadas a partir de `face` atravessando só arestas suaves (tangentes entre as faces).
/// Inclui a própria face; a ordem é a da busca.
pub fn faces_tangentes(m: &Mesh, face: usize) -> Vec<usize> {
    let nf = m.face_ranges.len();
    if face >= nf {
        return vec![];
    }
    let mut visto = vec![false; nf];
    let mut fila = vec![face];
    visto[face] = true;
    let mut i = 0;
    while i < fila.len() {
        let f = fila[i];
        i += 1;
        for (e, fs) in m.edge_faces.iter().enumerate() {
            if m.edge_smooth.get(e).copied().unwrap_or(0) == 0 {
                continue;
            }
            let outra = if fs[0] as usize == f && fs[1] != NO_FACE {
                fs[1]
            } else if fs[1] as usize == f && fs[0] != NO_FACE {
                fs[0]
            } else {
                continue;
            };
            let o = outra as usize;
            if o < nf && !visto[o] {
                visto[o] = true;
                fila.push(o);
            }
        }
    }
    fila
}

/// Arestas ligadas ponta a ponta a `edge`. Com `so_tangentes`, só segue quando a aresta seguinte
/// sai na mesma direção (sem quina); sem ele, segue qualquer aresta que encosta, parando nas
/// bifurcações (um vértice com mais de duas arestas não é seguido por nenhum dos ramos).
pub fn arestas_em_cadeia(m: &Mesh, edge: usize, so_tangentes: bool) -> Vec<usize> {
    let ne = m.edge_ranges.len();
    if edge >= ne {
        return vec![];
    }
    let todas: Vec<Option<[Ponta; 2]>> = (0..ne).map(|e| pontas(m, e)).collect();
    if todas[edge].is_none() {
        return vec![edge];
    }
    // quantas pontas de aresta caem em cada ponto (para reconhecer bifurcação)
    let grau = |p: [f64; 3]| todas.iter().flatten().flat_map(|a| a.iter()).filter(|q| mesmo_ponto(q.pos, p)).count();
    let mut cadeia = vec![edge];
    // cada ponta aberta da aresta inicial é seguida para um lado
    for lado in 0..2 {
        let mut atual = edge;
        let mut ponta_atual = lado;
        loop {
            let Some(pa) = todas[atual].as_ref().map(|a| (a[ponta_atual].pos, a[ponta_atual].saindo)) else { break };
            if !so_tangentes && grau(pa.0) != 2 {
                break;
            }
            let mut achada = None;
            for (e, ps) in todas.iter().enumerate() {
                if e == atual || cadeia.contains(&e) {
                    continue;
                }
                let Some(ps) = ps else { continue };
                for (k, q) in ps.iter().enumerate() {
                    if !mesmo_ponto(q.pos, pa.0) {
                        continue;
                    }
                    // na ponta em comum as duas arestas saem em sentidos opostos quando são tangentes
                    if so_tangentes && dot(pa.1, q.saindo) > -COS_TANGENTE {
                        continue;
                    }
                    achada = Some((e, 1 - k));
                    break;
                }
                if achada.is_some() {
                    break;
                }
            }
            let Some((e, outra)) = achada else { break };
            cadeia.push(e);
            atual = e;
            ponta_atual = outra;
        }
    }
    cadeia
}

/// Eixo (ponto e direção unitária) de um círculo da malha.
pub fn eixo_do_circulo(m: &Mesh, e: usize) -> Option<([f64; 3], [f64; 3])> {
    match m.edge_formula(e)? {
        EdgeFormula::Conic { c, u, v, .. } => {
            let n = cross(u, v);
            (norma(n) > 1e-12).then(|| (c, unit(n)))
        }
        EdgeFormula::Line { .. } => None,
    }
}

fn sobre_o_eixo(eixo: ([f64; 3], [f64; 3]), p: [f64; 3]) -> bool {
    let d = sub(p, eixo.0);
    norma(cross(d, eixo.1)) < TOL_EIXO
}

/// Arestas circulares coaxiais com a de `edge` (inclui ela). Aresta que não é círculo: só ela.
pub fn arestas_concentricas(m: &Mesh, edge: usize) -> Vec<usize> {
    let Some(eixo) = eixo_do_circulo(m, edge) else { return vec![edge] };
    (0..m.edge_ranges.len())
        .filter(|e| *e == edge || eixo_do_circulo(m, *e).is_some_and(|(c, n)| dot(n, eixo.1).abs() > 0.999 && sobre_o_eixo(eixo, c)))
        .collect()
}

/// Faces curvas que giram em torno do mesmo eixo da face `face` (inclui ela). O eixo vem dos
/// círculos de borda da face; face sem círculo de borda (plano, por exemplo): só ela.
pub fn faces_concentricas(m: &Mesh, face: usize) -> Vec<usize> {
    let nf = m.face_ranges.len();
    if face >= nf {
        return vec![];
    }
    let bordas_de = |f: usize| (0..m.edge_faces.len()).filter(move |e| m.edge_faces[*e].contains(&(f as u32)));
    let Some(eixo) = bordas_de(face).find_map(|e| eixo_do_circulo(m, e)) else { return vec![face] };
    let mut out = vec![face];
    for e in 0..m.edge_faces.len() {
        let Some((c, n)) = eixo_do_circulo(m, e) else { continue };
        if dot(n, eixo.1).abs() < 0.999 || !sobre_o_eixo(eixo, c) {
            continue;
        }
        for f in m.edge_faces[e] {
            let f = f as usize;
            // só superfícies curvas: o plano que fecha um furo não gira em torno do eixo
            if f < nf && !out.contains(&f) && m.face_info.get(f).is_some_and(|i| i.kind != 0) {
                out.push(f);
            }
        }
    }
    out
}

/// Arestas da borda da face plana `face` (o contorno inteiro, laço externo e furos).
fn bordas_da_face(m: &Mesh, face: usize) -> Vec<usize> {
    (0..m.edge_faces.len()).filter(|e| m.edge_faces[*e].contains(&(face as u32))).collect()
}

/// B543: arestas "do mesmo plano" que `edge`: todo o contorno de uma face plana que ela limita.
/// Uma aresta limita até duas faces; escolhe-se a plana que mais olha para o `olho` (a que a pessoa
/// está vendo), ou a primeira plana sem `olho`. Aresta que não toca face plana: só ela. O resultado
/// traz `edge` na frente.
pub fn arestas_do_mesmo_plano(m: &Mesh, edge: usize, olho: Option<[f64; 3]>) -> Vec<usize> {
    let Some(fs) = m.edge_faces.get(edge) else { return vec![edge] };
    let nf = m.face_ranges.len();
    let planas: Vec<usize> = fs
        .iter()
        .map(|f| *f as usize)
        .filter(|f| *f < nf && m.face_info.get(*f).is_some_and(|i| i.kind == 0))
        .collect();
    let voltada = |f: usize| {
        let Some(i) = m.face_info.get(f) else { return f64::MIN };
        // quanto o plano olha para o olho: normal externa contra a direção até o olho
        olho.map(|o| dot(unit(i.dir), unit(sub(o, i.point)))).unwrap_or(0.)
    };
    let Some(face) = planas.iter().copied().max_by(|a, b| voltada(*a).partial_cmp(&voltada(*b)).unwrap_or(std::cmp::Ordering::Equal).then(b.cmp(a))) else { return vec![edge] };
    let mut out = vec![edge];
    out.extend(bordas_da_face(m, face).into_iter().filter(|e| *e != edge));
    out
}

/// Pontos de uma face (vértices dos triângulos dela), para sentir de que lado do plano ela fica.
fn vertices_da_face(m: &Mesh, face: usize) -> Vec<[f64; 3]> {
    let Some(&ini) = m.face_ranges.get(face) else { return vec![] };
    let fim = m.face_ranges.get(face + 1).map(|x| *x as usize).unwrap_or(m.indices.len());
    m.indices[ini as usize..fim.min(m.indices.len())]
        .iter()
        .filter_map(|i| m.positions.get(*i as usize * 3..*i as usize * 3 + 3))
        .map(|p| [p[0] as f64, p[1] as f64, p[2] as f64])
        .collect()
}

/// Convexidade de uma aresta viva entre duas faces: `Some(true)` convexa (o material fica "atrás" do
/// canto, como a quina de fora de um bloco), `Some(false)` côncava (quina de dentro), `None` se a
/// aresta é suave, de borda livre ou se nenhuma das faces é plana.
///
/// Como: com a face plana `P` (normal externa `n`, aresta sobre o plano dela) e a outra face `Q`, os
/// pontos de `Q` perto da aresta ficam abaixo do plano de `P` num canto convexo e acima num côncavo.
/// Olha-se o ponto de `Q` mais próximo do meio da aresta que saia do plano.
pub fn convexidade(m: &Mesh, edge: usize) -> Option<bool> {
    let fs = m.edge_faces.get(edge)?;
    if fs[0] == NO_FACE || fs[1] == NO_FACE || m.edge_smooth.get(edge).copied().unwrap_or(0) != 0 {
        return None;
    }
    let (a, b) = (fs[0] as usize, fs[1] as usize);
    let (plana, outra) = match (m.face_info.get(a)?, m.face_info.get(b)?) {
        (ia, _) if ia.kind == 0 => (a, b),
        (_, ib) if ib.kind == 0 => (b, a),
        _ => return None,
    };
    let info = m.face_info.get(plana)?;
    let n = unit(info.dir);
    // o meio da aresta; nas costuras o kernel deixa zeros e vale o ponto do meio da polilinha
    let mid = m.edge_mids.get(edge * 3..edge * 3 + 3).map(|x| [x[0] as f64, x[1] as f64, x[2] as f64]).filter(|p| norma(*p) > 0.);
    let linha = polilinha(m, edge);
    let meio = mid.or_else(|| linha.get(linha.len() / 2).copied())?;
    // aresta e plano: a aresta tem de estar sobre o plano da face plana, senão a conta não vale
    if dot(n, sub(meio, info.point)).abs() > 1e-3 {
        return None;
    }
    // as duas faces planas: a direção para dentro de Q (perpendicular à aresta, no plano de Q) dá a
    // resposta exata, mesmo que Q dê a volta por trás do plano de P (como o topo em L de um bloco)
    if let (Some(iq), [p0, .., pn]) = (m.face_info.get(outra).filter(|i| i.kind == 0), linha.as_slice()) {
        let t = unit(sub(*pn, *p0));
        let w = unit(cross(unit(iq.dir), t));
        for sinal in [1., -1.] {
            let ponto = [meio[0] + sinal * w[0] * 1e-2, meio[1] + sinal * w[1] * 1e-2, meio[2] + sinal * w[2] * 1e-2];
            if ponto_na_face(m, outra, ponto) {
                return Some(dot(n, w) * sinal < 0.);
            }
        }
    }
    // Q curva: o ponto de Q mais perto do meio da aresta que saia do plano decide
    let mut candidatos: Vec<(f64, f64)> = vertices_da_face(m, outra)
        .into_iter()
        .map(|v| (norma(sub(v, meio)), dot(n, sub(v, meio))))
        .filter(|(_, h)| h.abs() > 1e-4)
        .collect();
    candidatos.sort_by(|x, y| x.0.partial_cmp(&y.0).unwrap_or(std::cmp::Ordering::Equal));
    candidatos.first().map(|(_, h)| *h < 0.)
}

/// O ponto (já no plano da face) cai dentro de algum triângulo da face?
fn ponto_na_face(m: &Mesh, face: usize, p: [f64; 3]) -> bool {
    let Some(&ini) = m.face_ranges.get(face) else { return false };
    let fim = m.face_ranges.get(face + 1).map(|x| *x as usize).unwrap_or(m.indices.len());
    let v = |i: u32| m.positions.get(i as usize * 3..i as usize * 3 + 3).map(|x| [x[0] as f64, x[1] as f64, x[2] as f64]);
    m.indices[ini as usize..fim.min(m.indices.len())].chunks_exact(3).any(|t| {
        let (Some(a), Some(b), Some(c)) = (v(t[0]), v(t[1]), v(t[2])) else { return false };
        // coordenadas baricêntricas no plano do triângulo
        let (e0, e1, e2) = (sub(b, a), sub(c, a), sub(p, a));
        let (d00, d01, d11, d20, d21) = (dot(e0, e0), dot(e0, e1), dot(e1, e1), dot(e2, e0), dot(e2, e1));
        let den = d00 * d11 - d01 * d01;
        if den.abs() < 1e-18 {
            return false;
        }
        let (u, w) = ((d11 * d20 - d01 * d21) / den, (d00 * d21 - d01 * d20) / den);
        let eps = -1e-6;
        u >= eps && w >= eps && u + w <= 1. - eps
    })
}

/// B543: arestas do corpo com a mesma característica de `edge`: todas as convexas ou todas as
/// côncavas (quinas vivas). Aresta suave ou sem convexidade definida: só ela. `edge` vai na frente.
pub fn arestas_mesma_convexidade(m: &Mesh, edge: usize) -> Vec<usize> {
    let Some(alvo) = convexidade(m, edge) else { return vec![edge] };
    let mut out = vec![edge];
    out.extend((0..m.edge_faces.len()).filter(|e| *e != edge && convexidade(m, *e) == Some(alvo)));
    out
}

/// Refaz as regras gravadas (B95): cada semente em `escolhidas` cresce pela regra sobre a malha atual
/// de `corpo`, e as arestas novas entram depois das escolhidas, sem repetir. Semente fora da lista é
/// ignorada.
pub fn expandir_arestas(corpo: &Shape, escolhidas: &[[f64; 3]], regras: &[RegraAresta]) -> Result<Vec<[f64; 3]>, String> {
    if regras.is_empty() {
        return Ok(escolhidas.to_vec());
    }
    let diag = corpo.bbox().map(|(a, b)| norma(sub(b, a))).unwrap_or(100.).max(1e-3);
    let m = corpo.mesh_angle(diag * 1e-3, 0.3).map_err(|e| e.0)?;
    let mid = |e: usize| m.edge_mids.get(e * 3..e * 3 + 3).map(|x| [x[0] as f64, x[1] as f64, x[2] as f64]);
    let mut saida = escolhidas.to_vec();
    let ja = |v: &[[f64; 3]], p: [f64; 3]| v.iter().any(|q| norma(sub(*q, p)) < 1e-4 * diag.max(1.));
    for r in regras {
        let Some(&semente) = escolhidas.get(r.semente) else { continue };
        let Some(e0) = (0..m.edge_ranges.len()).filter_map(|e| mid(e).map(|p| (e, norma(sub(p, semente))))).min_by(|a, b| a.1.total_cmp(&b.1)).map(|x| x.0) else { continue };
        let novas = match r.regra {
            RegraSelecao::Tangentes => arestas_em_cadeia(&m, e0, true),
            RegraSelecao::Cadeia => arestas_em_cadeia(&m, e0, false),
            RegraSelecao::Concentricas => arestas_concentricas(&m, e0),
        };
        for e in novas {
            if let Some(p) = mid(e) {
                if !ja(&saida, p) {
                    saida.push(p);
                }
            }
        }
    }
    Ok(saida)
}

/// Refaz as regras gravadas em comandos de face (B95: casca, saída e edição direta). Cada semente
/// em `escolhidas` (pontos já resolvidos sobre `corpo`) cresce pela regra sobre a malha atual: faces
/// tangentes ou concêntricas. As faces novas entram depois das escolhidas, sem repetir, pelo ponto
/// sobre a face que o kernel dá. A regra de cadeia não vale para faces: fica só a semente.
pub fn expandir_faces(corpo: &Shape, escolhidas: &[[f64; 3]], regras: &[RegraAresta]) -> Result<Vec<[f64; 3]>, String> {
    if regras.is_empty() {
        return Ok(escolhidas.to_vec());
    }
    let diag = corpo.bbox().map(|(a, b)| norma(sub(b, a))).unwrap_or(100.).max(1e-3);
    let m = corpo.mesh_angle(diag * 1e-3, 0.3).map_err(|e| e.0)?;
    let mut saida = escolhidas.to_vec();
    // índice da face de cada ponto já na lista, para não repetir a mesma face por outro ponto
    let mut ja: Vec<usize> = escolhidas.iter().filter_map(|p| corpo.face_index(*p, 1.0).ok().flatten()).collect();
    for r in regras {
        let Some(&semente) = escolhidas.get(r.semente) else { continue };
        let Some(f0) = corpo.face_index(semente, 1.0).map_err(|e| e.0)? else { continue };
        let novas = match r.regra {
            RegraSelecao::Tangentes => faces_tangentes(&m, f0),
            RegraSelecao::Concentricas => faces_concentricas(&m, f0),
            RegraSelecao::Cadeia => vec![],
        };
        for f in novas {
            if ja.contains(&f) {
                continue;
            }
            let Some(info) = m.face_info.get(f) else { continue };
            ja.push(f);
            saida.push(info.point);
        }
    }
    Ok(saida)
}
