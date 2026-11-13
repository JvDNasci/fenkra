// SPDX-License-Identifier: LGPL-2.1-or-later
//! Papel de cada face nas ferramentas que o kernel não rotula (B15, ADR 0010 fase 2): varredura,
//! loft, extrusão até plano, até superfície, até o próximo e com ângulo de saída, e furos.
//!
//! Como funciona: a extrusão e a revolução recebem o papel do próprio OCCT (`FirstShape`,
//! `LastShape` e `Generated` da aresta do perfil). As demais ferramentas passam por caminhos que
//! não guardam esse histórico (o tubo por trecho com junta de esquadria, o prisma cortado por um
//! semi-espaço, a ferramenta recortada pela superfície). Aqui o papel sai da geometria do resultado,
//! com a mesma convenção da extrusão:
//!
//! - a tampa inicial é a face plana, no plano do perfil, que encosta nas arestas do perfil (papel 0);
//! - a lateral gerada pela aresta `i` do perfil é a outra face dessa aresta (papel `2 + i`, com `i`
//!   na ordem das arestas do perfil, a mesma do kernel);
//! - a tampa final (papel 1) é, conforme a ferramenta: a que encosta nas arestas da última seção
//!   (loft), a face plana restante com a área da tampa inicial (varredura), ou tudo o que sobra e
//!   não encosta na tampa inicial (extrusões);
//! - na varredura com cantos, o trecho seguinte da lateral `i` encosta só na lateral `i` do trecho
//!   anterior (pela junta), e herda o papel dela.
//!
//! Em qualquer dúvida (aresta do perfil que não aparece no resultado, face com dois papéis) a
//! ferramenta fica sem papéis: a referência segue pelo ponto, como antes, e nunca por um papel errado.
//!
//! Furos têm papéis próprios a partir de `PAPEL_FURO`: parede, parede da cabeça, escareado, ponta,
//! boca, fundo e assento, por furo (a ordem dos centros).

use fenkra_kernel::{Shape, Topology, NO_FACE};

/// Primeiro papel dos furos: `PAPEL_FURO + 16 * furo + parte`.
pub const PAPEL_FURO: i32 = 1000;

/// Partes de um furo.
pub const PARTES_FURO: [&str; 8] = ["parede", "parede da cabeça", "escareado", "ponta", "boca", "fundo", "assento", "fundo do outro lado"];

/// Como achar a tampa final.
pub enum Fim<'a> {
    /// a face que encosta nas arestas desta seção (loft)
    Perfil(&'a Shape),
    /// faces planas restantes com a área da tampa inicial (varredura)
    MesmaArea,
    /// tudo o que sobra e não encosta na tampa inicial (extrusões)
    Resto,
}

/// Papéis da ferramenta `tool` gerada a partir do perfil `inicio`. `None` quando não dá para dizer
/// com certeza (aí a ferramenta fica como antes, sem papéis).
pub fn por_perfil(tool: &Shape, inicio: &Shape, fim: Fim) -> Option<Vec<i32>> {
    let t = tool.topology().ok()?;
    let p = inicio.topology().ok()?;
    let ultimo = match fim {
        Fim::Perfil(s) => Some(s.topology().ok()?),
        _ => None,
    };
    let modo = match fim {
        Fim::Perfil(_) => 0,
        Fim::MesmaArea => 1,
        Fim::Resto => 2,
    };
    papeis_topo(&t, &p, ultimo.as_ref(), modo)
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    dot(sub(a, b), sub(a, b)).sqrt()
}

/// Tolerância de casamento: o resultado tem as arestas do perfil exatas (a ~1e-7 mm); 1e-4 da
/// diagonal absorve a tolerância do kernel sem casar arestas vizinhas.
fn tolerancia(t: &Topology) -> f64 {
    let pts: Vec<[f64; 3]> = t.faces.iter().map(|f| f.point).chain(t.edge_mids.iter().copied()).collect();
    let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
    for q in &pts {
        for i in 0..3 {
            lo[i] = lo[i].min(q[i]);
            hi[i] = hi[i].max(q[i]);
        }
    }
    if pts.is_empty() {
        return 1e-4;
    }
    (dist(lo, hi) * 1e-4).clamp(1e-5, 1e-2)
}

/// Arestas reais (sem costura) de uma topologia: (índice, ponto médio, face do perfil).
fn arestas(p: &Topology) -> Vec<(usize, [f64; 3], usize)> {
    (0..p.edge_mids.len().min(p.edge_faces.len()))
        .filter_map(|i| {
            let [a, b] = p.edge_faces[i];
            let f = if a != NO_FACE { a } else { b };
            (f != NO_FACE).then_some((i, p.edge_mids[i], f as usize))
        })
        .collect()
}

/// Marca tampa e laterais a partir das arestas de uma seção. Devolve `false` na dúvida.
fn marcar(t: &Topology, sec: &Topology, tampa: i32, role: &mut [i32], laterais: bool, tol: f64) -> bool {
    let lista = arestas(sec);
    if lista.is_empty() {
        return false;
    }
    for (i, m, pf) in lista {
        let Some(info) = sec.faces.get(pf) else { return false };
        let (pn, pp) = (info.dir, info.point);
        let Some(e) = (0..t.edge_mids.len().min(t.edge_faces.len())).find(|e| t.edge_faces[*e] != [NO_FACE, NO_FACE] && dist(t.edge_mids[*e], m) < tol) else {
            return false;
        };
        let [a, b] = t.edge_faces[e];
        if a == NO_FACE || b == NO_FACE {
            return false;
        }
        let no_plano = |j: usize| {
            let f = &t.faces[j];
            f.kind == 0 && dot(f.dir, pn).abs() > 1. - 1e-6 && dot(sub(f.point, pp), pn).abs() < tol
        };
        let (cap, lat) = match (no_plano(a as usize), no_plano(b as usize)) {
            (true, false) => (a as usize, b as usize),
            (false, true) => (b as usize, a as usize),
            // seção fechada sem tampa (loft fechado) ou face ambígua
            _ => return false,
        };
        if role[cap] != -1 && role[cap] != tampa {
            return false;
        }
        role[cap] = tampa;
        let r = 2 + i as i32;
        if laterais {
            if role[lat] != -1 && role[lat] != r {
                return false;
            }
            role[lat] = r;
        } else if role[lat] == -1 {
            role[lat] = r;
        }
    }
    true
}

/// Vizinhas de cada face (pelas arestas).
fn vizinhas(t: &Topology) -> Vec<Vec<usize>> {
    let mut v = vec![vec![]; t.faces.len()];
    for [a, b] in &t.edge_faces {
        if *a != NO_FACE && *b != NO_FACE && a != b && (*a as usize) < v.len() && (*b as usize) < v.len() {
            v[*a as usize].push(*b as usize);
            v[*b as usize].push(*a as usize);
        }
    }
    v
}

/// Tampa final da varredura: as faces planas restantes com a área de uma tampa inicial, que não
/// encostam nela; vale só se forem tantas quanto as tampas iniciais.
fn tampa_por_area(t: &Topology, role: &mut [i32], viz: &[Vec<usize>]) {
    let inicio: Vec<usize> = (0..role.len()).filter(|j| role[*j] == 0).collect();
    let cand: Vec<usize> = (0..role.len())
        .filter(|j| role[*j] == -1 && t.faces[*j].kind == 0)
        .filter(|j| !viz[*j].iter().any(|n| role[*n] == 0))
        .filter(|j| inicio.iter().any(|c| (t.faces[*j].area - t.faces[*c].area).abs() <= 1e-3 * t.faces[*c].area.abs().max(1e-9)))
        .collect();
    if !cand.is_empty() && cand.len() == inicio.len() {
        for j in cand {
            role[j] = 1;
        }
    }
}

/// Núcleo puro (testável sem o kernel). `modo`: 0 seção final, 1 mesma área, 2 resto.
pub fn papeis_topo(t: &Topology, inicio: &Topology, ultimo: Option<&Topology>, modo: u8) -> Option<Vec<i32>> {
    let n = t.faces.len();
    if n == 0 {
        return None;
    }
    let tol = tolerancia(t);
    let mut role = vec![-1; n];
    if !marcar(t, inicio, 0, &mut role, true, tol) {
        return None;
    }
    let viz = vizinhas(t);
    match modo {
        0 => {
            if !marcar(t, ultimo?, 1, &mut role, false, tol) {
                return None;
            }
        }
        1 => tampa_por_area(t, &mut role, &viz),
        _ => {
            for j in 0..n {
                if role[j] == -1 && !viz[j].iter().any(|v| role[*v] == 0) {
                    role[j] = 1;
                }
            }
        }
    }
    // trechos seguintes das laterais (varredura com cantos, loft reto de várias seções): a face sem
    // papel que encosta em uma só lateral conhecida herda o papel dela
    // (cada passada olha só os papéis da passada anterior: a ordem das faces não muda o resultado)
    loop {
        let antes = role.clone();
        let mut mudou = false;
        for j in 0..n {
            if antes[j] != -1 {
                continue;
            }
            let mut rs: Vec<i32> = viz[j].iter().map(|v| antes[*v]).filter(|r| *r >= 2).collect();
            rs.sort_unstable();
            rs.dedup();
            if rs.len() == 1 {
                role[j] = rs[0];
                mudou = true;
            }
        }
        if !mudou {
            break;
        }
    }
    if modo == 1 && !role.contains(&1) {
        tampa_por_area(t, &mut role, &viz);
    }
    Some(role)
}

/// Papéis da ferramenta de furos: `seats` são a boca e a direção de cada furo, `d` o diâmetro e
/// `prof` a profundidade de cada um. A face vai para o furo de eixo mais perto.
pub fn furos(tool: &Shape, seats: &[([f64; 3], [f64; 3])], d: f64, prof: &[f64]) -> Option<Vec<i32>> {
    let t = tool.topology().ok()?;
    furos_topo(&t, seats, d, prof)
}

/// Núcleo puro de `furos`.
pub fn furos_topo(t: &Topology, seats: &[([f64; 3], [f64; 3])], d: f64, prof: &[f64]) -> Option<Vec<i32>> {
    if seats.is_empty() || t.faces.is_empty() {
        return None;
    }
    let eixo = |h: usize, q: [f64; 3]| {
        let (o, v) = seats[h];
        let s = dot(sub(q, o), v);
        let r = dist(q, [o[0] + v[0] * s, o[1] + v[1] * s, o[2] + v[2] * s]);
        (s, r)
    };
    let tol = (d * 1e-4).max(1e-6);
    let mut out = Vec::with_capacity(t.faces.len());
    for f in &t.faces {
        let h = (0..seats.len()).min_by(|a, b| eixo(*a, f.point).1.total_cmp(&eixo(*b, f.point).1))?;
        let (s, r) = eixo(h, f.point);
        let p = prof.get(h).copied().unwrap_or(0.).max(tol);
        let parte = match f.kind {
            1 if (r - d / 2.).abs() <= 1e-3 * d.max(1.) => 0,
            1 => 1,
            0 if s.abs() <= tol * 10. => 4,
            0 if s < 0. => 7,
            0 if s >= 0.5 * p => 5,
            0 => 6,
            _ if s < 0.5 * p => 2,
            _ => 3,
        };
        out.push(PAPEL_FURO + 16 * h as i32 + parte);
    }
    Some(out)
}

/// Nome de um papel de furo (para mensagens), ou `None` se o papel não é de furo.
pub fn nome_furo(role: i32) -> Option<String> {
    if role < PAPEL_FURO {
        return None;
    }
    let r = role - PAPEL_FURO;
    let parte = PARTES_FURO.get((r % 16) as usize).copied().unwrap_or("parte");
    Some(format!("furo {}, {parte}", r / 16 + 1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use fenkra_kernel::FaceInfo;

    fn face(kind: u8, dir: [f64; 3], area: f64, point: [f64; 3]) -> FaceInfo {
        FaceInfo { kind, dir, area, point }
    }

    /// Perfil quadrado 10 x 10 no plano z = 0 (uma face, quatro arestas).
    fn perfil() -> Topology {
        Topology {
            faces: vec![face(0, [0., 0., 1.], 100., [5., 5., 0.])],
            edge_mids: vec![[5., 0., 0.], [10., 5., 0.], [5., 10., 0.], [0., 5., 0.]],
            edge_faces: vec![[0, NO_FACE]; 4],
        }
    }

    /// Cubo varrido de z = 0 a z = 10: faces 0 base, 1 topo, 2..5 laterais (fora de ordem de propósito).
    fn cubo() -> Topology {
        let faces = vec![
            face(0, [0., 0., -1.], 100., [5., 5., 0.]),
            face(0, [0., 0., 1.], 100., [5., 5., 10.]),
            face(0, [-1., 0., 0.], 100., [0., 5., 5.]),
            face(0, [0., -1., 0.], 100., [5., 0., 5.]),
            face(0, [1., 0., 0.], 100., [10., 5., 5.]),
            face(0, [0., 1., 0.], 100., [5., 10., 5.]),
        ];
        let edge_mids = vec![
            [5., 0., 0.], [10., 5., 0.], [5., 10., 0.], [0., 5., 0.],
            [5., 0., 10.], [10., 5., 10.], [5., 10., 10.], [0., 5., 10.],
            [0., 0., 5.], [10., 0., 5.], [10., 10., 5.], [0., 10., 5.],
        ];
        let edge_faces = vec![[0, 3], [0, 4], [0, 5], [0, 2], [1, 3], [1, 4], [1, 5], [1, 2], [2, 3], [3, 4], [4, 5], [5, 2]];
        Topology { faces, edge_mids, edge_faces }
    }

    #[test]
    fn varredura_reta_acha_tampas_e_laterais_pela_aresta_do_perfil() {
        // o cubo é o caso difícil: as seis faces têm a mesma área e o mesmo número de arestas
        let r = papeis_topo(&cubo(), &perfil(), None, 1).unwrap();
        assert_eq!(r, vec![0, 1, 5, 2, 3, 4]);
        let r = papeis_topo(&cubo(), &perfil(), None, 2).unwrap();
        assert_eq!(r, vec![0, 1, 5, 2, 3, 4]);
    }

    #[test]
    fn aresta_do_perfil_que_sumiu_deixa_a_ferramenta_sem_papeis() {
        let mut t = cubo();
        t.edge_mids[0] = [5., 0., 0.5];
        assert_eq!(papeis_topo(&t, &perfil(), None, 2), None);
    }

    #[test]
    fn trecho_seguinte_da_varredura_herda_a_lateral_pela_junta() {
        // dois trechos: laterais 2..5 do primeiro; 6 é o trecho seguinte da lateral da aresta 0 (encosta só
        // na face 3 do primeiro trecho e nas vizinhas ainda sem papel)
        let mut t = cubo();
        t.faces.push(face(0, [0., -1., 0.], 80., [5., 0., 15.]));
        t.faces.push(face(0, [1., 0., 0.], 80., [10., 5., 15.]));
        t.edge_mids.push([5., 0., 20.]);
        t.edge_faces[4] = [6, 3]; // a junta: o topo do primeiro trecho vira encontro com o seguinte
        t.edge_faces[5] = [7, 4];
        t.edge_faces.push([6, 7]);
        t.faces[1].point = [5., 5., 99.]; // a antiga tampa agora é outra face, sem papel
        let r = papeis_topo(&t, &perfil(), None, 1).unwrap();
        assert_eq!(r[6], 2, "segue a lateral da aresta 0 do perfil (face 3)");
        assert_eq!(r[7], 3);
        assert_eq!(r[1], 1, "a tampa final sai pela área");
    }

    #[test]
    fn furo_rebaixado_tem_parede_cabeca_assento_e_fundo() {
        let seats = [([0., 0., 10.], [0., 0., -1.])];
        let t = Topology {
            faces: vec![
                face(1, [0., 0., 1.], 1., [3., 0., 5.]),   // parede Ø6
                face(1, [0., 0., 1.], 1., [5., 0., 9.]),   // parede do rebaixo Ø10
                face(0, [0., 0., 1.], 1., [4., 0., 8.]),   // assento a 2 mm
                face(0, [0., 0., -1.], 1., [1., 0., 0.]),  // fundo a 10 mm
                face(0, [0., 0., 1.], 1., [4., 0., 10.]),  // boca
            ],
            edge_mids: vec![],
            edge_faces: vec![],
        };
        let r = furos_topo(&t, &seats, 6., &[10.]).unwrap();
        assert_eq!(r, vec![1000, 1001, 1006, 1005, 1004]);
        assert_eq!(nome_furo(1006).as_deref(), Some("furo 1, assento"));
        assert_eq!(nome_furo(2), None);
    }
}
