// SPDX-License-Identifier: LGPL-2.1-or-later
//! Mapa de dobras do corpo convertido em chapa (B35).
//!
//! `chapa_converte` reconhece espessura e dobras. Aqui cada dobra reconhecida vira um `SheetMap`,
//! o mesmo registro que as abas e dobras por linha deixam, e por isso a tabela de dobras, as linhas
//! de dobra, a planificada e os recortes sobre o corpo passam a funcionar como numa chapa feita
//! pelos comandos de chapa.
//!
//! Como se monta, só com a malha (tipo, normal, ponto e área de cada face):
//!
//! 1. Cada trecho plano da chapa é um *nó*: o plano médio da espessura o identifica (a face de
//!    cima e a de baixo caem no mesmo plano médio).
//! 2. Cada dobra liga os dois nós que tocam o cilindro interno: são os planos principais
//!    perpendiculares ao eixo e a um raio interno de distância dele.
//! 3. A base é o nó da face fixa do comando (ou o da maior face plana). Uma busca em largura leva
//!    as dobras da base para fora: a dobra de pai para filho vira um `SheetMap` com a face do pai
//!    como referência, `u` saindo do pai e ângulo positivo para o lado do centro de curvatura.
//!    Uma dobra que fecharia um laço (caixa fechada) não se planifica e vira erro com o motivo.
//! 4. Planificar o corpo: remove-se a cunha de cada dobra, as pernas giram de volta ao plano do
//!    pai (da folha para a raiz, como `map_tool`) e a tira do comprimento desenvolvido entra no
//!    lugar do arco. O volume é conferido contra a conta (corpo menos arcos mais tiras).

use super::chapa_converte::AnaliseChapa;
use super::*;
use fenkra_kernel::Mesh;
use std::collections::VecDeque;

/// Um trecho plano da chapa: o plano médio (normal canônica e posição) e as faces principais.
struct No {
    n: V3,
    d: f64,
    faces: Vec<usize>,
}

fn canonica(n: V3) -> V3 {
    let k = (0..3).find(|i| n[*i].abs() > 1e-6).unwrap_or(0);
    if n[k] < 0. {
        geo3d::mul(n, -1.)
    } else {
        n
    }
}

fn nos_da_chapa(m: &Mesh, a: &AnaliseChapa) -> Vec<No> {
    let t = a.espessura;
    let mut nos: Vec<No> = vec![];
    for &f in &a.planas {
        let i = &m.face_info[f];
        let Some(dir) = geo3d::unit(i.dir) else { continue };
        let meio = geo3d::sub(i.point, geo3d::mul(dir, t / 2.));
        let n = canonica(dir);
        let d = geo3d::dot(meio, n);
        match nos.iter_mut().find(|x| geo3d::dot(x.n, n) > 0.9999 && (x.d - d).abs() < 0.05 * t + 1e-3) {
            Some(x) => x.faces.push(f),
            None => nos.push(No { n, d, faces: vec![f] }),
        }
    }
    nos
}

/// Uma dobra com seus dois nós e as faces tangentes ao cilindro interno em cada um.
struct Ligacao {
    dobra: usize,
    nos: [usize; 2],
    face: [usize; 2],
}

fn ligacoes(m: &Mesh, a: &AnaliseChapa, nos: &[No]) -> Result<Vec<Ligacao>, String> {
    let no_da_face = |f: usize| nos.iter().position(|n| n.faces.contains(&f));
    let mut out = vec![];
    for (di, d) in a.dobras.iter().enumerate() {
        let o = geo3d::add(d.origem, geo3d::mul(d.eixo, (d.faixa[0] + d.faixa[1]) / 2.));
        let mut achados: Vec<(usize, usize)> = vec![];
        for &f in &a.planas {
            let i = &m.face_info[f];
            let Some(dir) = geo3d::unit(i.dir) else { continue };
            // plano paralelo ao eixo, tangente ao cilindro interno, com a normal apontando para ele
            if geo3d::dot(dir, d.eixo).abs() > 1e-3 {
                continue;
            }
            let dist = geo3d::dot(geo3d::sub(i.point, o), dir);
            if (dist + d.raio).abs() > 0.02 * d.raio + 0.01 {
                continue;
            }
            // a face precisa cobrir a faixa axial da dobra
            let pts = super::chapa_converte::pontos_da_face(m, f);
            let (lo, hi) = pts.iter().fold((f64::MAX, f64::MIN), |(l, h), p| (l.min(geo3d::dot(*p, d.eixo)), h.max(geo3d::dot(*p, d.eixo))));
            if hi < d.faixa[0] + 1e-3 || lo > d.faixa[1] - 1e-3 {
                continue;
            }
            if let Some(n) = no_da_face(f) {
                if !achados.iter().any(|(x, _)| *x == n) {
                    achados.push((n, f));
                }
            }
        }
        if achados.len() != 2 {
            return Err(format!("a dobra {} (r{:.2}, {:.0}°) não liga dois trechos planos: achei {}", di + 1, d.raio, d.angulo, achados.len()));
        }
        out.push(Ligacao { dobra: di, nos: [achados[0].0, achados[1].0], face: [achados[0].1, achados[1].1] });
    }
    Ok(out)
}

/// A árvore de dobras a partir da base e o que se precisa dela: os mapas na ordem pai→filho.
pub(super) struct Mapa {
    pub maps: Vec<SheetMap>,
    /// volume (mm³) dos arcos que o planificado troca por tiras e das tiras
    pub arcos: f64,
    pub tiras: f64,
}

pub(super) fn mapear(m: &Mesh, a: &AnaliseChapa, face_fixa: V3, kf: &dyn Fn(f64) -> f64, grande: f64) -> Result<Mapa, String> {
    let nos = nos_da_chapa(m, a);
    if nos.is_empty() {
        return Err("o corpo não tem trecho plano de chapa".into());
    }
    let lig = ligacoes(m, a, &nos)?;
    // base e a face de referência: a face fixa do comando, ou a maior face plana principal
    let ref_face = if face_fixa != [0.; 3] {
        a.planas
            .iter()
            .copied()
            .find(|f| {
                let i = &m.face_info[*f];
                geo3d::dot(geo3d::sub(face_fixa, i.point), i.dir).abs() < 1e-3 * a.espessura.max(1.)
            })
            .ok_or("a face fixa não é uma face plana principal")?
    } else {
        a.planas.iter().copied().max_by(|x, y| m.face_info[*x].area.total_cmp(&m.face_info[*y].area)).ok_or("sem face plana")?
    };
    let ref_n = geo3d::unit(m.face_info[ref_face].dir).ok_or("face de referência sem normal")?;
    let raiz = nos.iter().position(|n| n.faces.contains(&ref_face)).ok_or("a face de referência não está num trecho de chapa")?;

    // busca em largura: pai, dobra e rotação acumulada (folha → raiz) de cada nó
    let mut pai: Vec<Option<(usize, usize)>> = vec![None; nos.len()];
    let mut visto = vec![false; nos.len()];
    let mut usada = vec![false; lig.len()];
    let mut ordem: Vec<(usize, usize, usize)> = vec![]; // (dobra, pai, filho)
    let mut fila = VecDeque::from([raiz]);
    visto[raiz] = true;
    while let Some(p) = fila.pop_front() {
        for (li, l) in lig.iter().enumerate() {
            if usada[li] || !l.nos.contains(&p) {
                continue;
            }
            let c = if l.nos[0] == p { l.nos[1] } else { l.nos[0] };
            usada[li] = true;
            if visto[c] {
                return Err("as dobras fecham um laço (caixa fechada): a chapa não se planifica sem um corte de alívio".into());
            }
            visto[c] = true;
            pai[c] = Some((p, li));
            ordem.push((li, p, c));
            fila.push_back(c);
        }
    }
    if usada.iter().any(|u| !u) {
        return Err("há dobras que não se ligam à face fixa: o corpo tem partes soltas".into());
    }
    let t = a.espessura;
    let mut maps: Vec<SheetMap> = vec![];
    // eixo e ângulo de cada nó filho, para levar uma direção do dobrado ao plano (folha → raiz)
    let mut giro: Vec<Option<(V3, f64)>> = vec![None; nos.len()];
    let (mut arcos, mut tiras) = (0., 0.);
    for (li, p, c) in ordem {
        let l = &lig[li];
        let d = &a.dobras[l.dobra];
        let (fp, fc) = if l.nos[0] == p { (l.face[0], l.face[1]) } else { (l.face[1], l.face[0]) };
        let n = geo3d::unit(m.face_info[fp].dir).ok_or("normal nula")?;
        let nc = geo3d::unit(m.face_info[fc].dir).ok_or("normal nula")?;
        let ang = geo3d::dot(n, nc).clamp(-1., 1.).acos().to_degrees();
        if ang > 179. {
            return Err(format!("a dobra {} tem {ang:.0}°: dobras acima de 179° não se planificam", l.dobra + 1));
        }
        if (ang - d.angulo).abs() > 5. && (360. - d.angulo - ang).abs() > 5. {
            return Err(format!("a dobra {}: o ângulo pelas faces ({ang:.1}°) não confere com o pela área ({:.1}°)", l.dobra + 1, d.angulo));
        }
        let ev = d.eixo;
        let lo = geo3d::add(d.origem, geo3d::mul(ev, d.faixa[0]));
        let c0 = geo3d::sub(lo, geo3d::mul(n, d.raio));
        let filho_tg = geo3d::sub(lo, geo3d::mul(nc, d.raio));
        let mut u = geo3d::unit(fenkra_kernel::cross(ev, n)).ok_or("eixo paralelo à normal")?;
        if geo3d::dot(u, geo3d::sub(filho_tg, c0)) < 0. {
            u = geo3d::mul(u, -1.);
        }
        let axis = fenkra_kernel::cross(u, n);
        let k = kf(d.raio);
        let ba = chapa::compensacao(t, d.raio, ang, k).ba;
        // a direção do centro de curvatura levada ao plano da base diz se a dobra sobe ou desce
        let mut v = n;
        let mut no = p;
        while let Some((eixo, g)) = giro[no] {
            v = geo3d::rotate(v, eixo, -g);
            no = pai[no].map_or(raiz, |x| x.0);
        }
        let down = geo3d::dot(v, ref_n) < 0.;
        let a_rad = ang.to_radians();
        giro[c] = Some((axis, a_rad));
        let larg = d.faixa[1] - d.faixa[0];
        maps.push(SheetMap { c0, ev, l: larg, n, u, t, o: lo, axis, a: a_rad, ba, length: grande, down, feature: Id(0) });
        // volumes para a conferência: o arco (setor de coroa) e a tira da planificada
        let re = d.raio + t;
        arcos += a_rad / 2. * (re * re - d.raio * d.raio) * larg;
        tiras += ba * t * larg;
    }
    Ok(Mapa { maps, arcos, tiras })
}

/// O corpo planificado: tira o arco de cada dobra, gira cada perna de volta ao plano do pai (da
/// última dobra para a primeira) e põe a tira do comprimento desenvolvido no lugar do arco.
pub(super) fn planificar(corpo: &Shape, maps: &[SheetMap]) -> Result<Shape, String> {
    let k = |e: KernelError| e.0;
    let mut corpo = corpo.clone();
    let grande = maps.iter().map(|m| m.length).fold(0., f64::max) + 10.;
    // 1. os arcos saem: cunha entre o plano do pai e o da perna, dentro do raio externo
    for m in maps {
        let r = geo3d::dist(m.o, m.c0);
        let u2 = geo3d::rotate(m.u, m.axis, m.a);
        let cunha = half_space(m.o, m.u, grande)?.boolean(&half_space(m.o, geo3d::mul(u2, -1.), grande)?, Bool::Common).map_err(k)?;
        let margem = 0.05;
        let cil = Shape::hole_tool(geo3d::sub(m.o, geo3d::mul(m.ev, margem + 0.5)), m.ev, 2. * (r + m.t + margem), m.l + 2. * margem + 1., HoleShape::Simple).map_err(k)?;
        let arco = cunha.boolean(&cil, Bool::Common).map_err(k)?;
        corpo = corpo.boolean(&arco, Bool::Cut).map_err(k)?;
    }
    // 2. da última dobra à primeira: a perna gira ao plano do pai e a tira fecha o vão
    for m in maps.iter().rev() {
        let reg = m.region(false)?;
        let perna = corpo.boolean(&reg, Bool::Common).map_err(k)?;
        let resto = corpo.boolean(&reg, Bool::Cut).map_err(k)?;
        let movida = perna.transformed(Xform::Rotate(m.o, m.axis, -m.a.to_degrees())).map_err(k)?.transformed(Xform::Translate(geo3d::mul(m.u, m.ba))).map_err(k)?;
        let c = |du: f64, de: f64| geo3d::sub(geo3d::add(geo3d::add(m.c0, geo3d::mul(m.u, du)), geo3d::mul(m.ev, de)), geo3d::mul(m.n, m.t));
        let tira = Shape::poly_prism(&[c(0., 0.), c(m.ba, 0.), c(m.ba, m.l), c(0., m.l)], geo3d::mul(m.n, m.t)).map_err(k)?;
        let vivo = |s: &Shape| !s.is_null() && s.face_count() > 0;
        let mut novo = if vivo(&resto) { resto } else { Shape::empty() };
        for peca in [movida, tira] {
            if !vivo(&peca) {
                continue;
            }
            novo = if novo.is_null() { peca } else { novo.boolean(&peca, Bool::Fuse).map_err(k)? };
        }
        corpo = novo;
    }
    Ok(corpo)
}
