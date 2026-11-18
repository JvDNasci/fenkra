// SPDX-License-Identifier: LGPL-2.1-or-later
//! Converter em chapa um corpo de cantos vivos (caixa com casca aberta, por exemplo).
//!
//! O reconhecimento de `chapa_converte` só acha dobras onde o corpo já tem os dois cilindros
//! coaxiais (raio interno e externo). Uma caixa feita por extrusão e casca tem as paredes ligadas
//! por arestas vivas: sem dobra não há mapa de dobras, e a planificada ficava igual ao corpo. Os
//! softwares de modelagem CAD avançados tratam essas arestas assim, e aqui fazemos o mesmo:
//!
//! 1. Cada trecho plano da chapa é um nó (o plano médio da espessura, como no mapa de dobras). Duas
//!    paredes ligadas por aresta viva formam uma junta.
//! 2. Uma busca em largura a partir da face fixa escolhe a árvore: a junta de pai para filho vira
//!    dobra; a junta que fecharia um laço (o canto entre duas paredes laterais) vira rasgo.
//! 3. Rasgo: uma fenda fina separa as duas paredes no canto (uma fica com o canto inteiro, a outra
//!    recua a largura da fenda). Alívio: perto do pai comum, uma caixa tira a zona em que as duas
//!    dobras se cruzariam, para cada dobra terminar numa face plana e não num canto esférico.
//! 4. Dobra: a aresta viva de fora recebe filete de raio `r + t` e a de dentro, de raio `r`. As
//!    duas ficam coaxiais, a espessura continua constante e as medidas externas não mudam.
//!
//! Depois disso o corpo tem cilindros de dobra de verdade e o reconhecimento normal monta o mapa de
//! dobras e a planificada.

use super::chapa_converte::{pontos_da_face, AnaliseChapa};
use super::chapa_converte_mapa::{nos_da_chapa, No};
use super::*;
use fenkra_kernel::{Mesh, NO_FACE};
use std::collections::VecDeque;

/// O que a conversão das arestas vivas fez, para o aviso do comando.
pub(super) struct Vivas {
    pub corpo: Shape,
    pub dobras: usize,
    pub rasgos: usize,
}

/// Junta entre dois nós (índices em `nos`), por aresta viva.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Junta(usize, usize);

impl Junta {
    fn tem(&self, n: usize) -> bool {
        self.0 == n || self.1 == n
    }
    fn outro(&self, n: usize) -> usize {
        if self.0 == n {
            self.1
        } else {
            self.0
        }
    }
}

/// Nó de uma face plana pelo plano dela (a face fica numa das duas faces do nó, a `t/2` do meio).
fn no_do_plano(nos: &[No], dir: V3, ponto: V3, t: f64) -> Option<usize> {
    let d = geo3d::unit(dir)?;
    nos.iter().position(|n| geo3d::dot(n.n, d).abs() > 0.9999 && ((geo3d::dot(ponto, n.n) - n.d).abs() - t / 2.).abs() < 0.05 * t + 1e-3)
}

/// Centro dos pontos das faces de um nó.
fn centro(m: &Mesh, no: &No) -> V3 {
    let pts: Vec<V3> = no.faces.iter().flat_map(|f| pontos_da_face(m, *f)).collect();
    let n = pts.len().max(1) as f64;
    let s = pts.iter().fold([0.; 3], |a, p| geo3d::add(a, *p));
    geo3d::mul(s, 1. / n)
}

/// Juntas por aresta viva entre faces principais de nós diferentes e não paralelos.
fn juntas(m: &Mesh, a: &AnaliseChapa, nos: &[No]) -> Vec<Junta> {
    let no_da_face = |f: u32| -> Option<usize> {
        if f == NO_FACE || !a.planas.contains(&(f as usize)) {
            return None;
        }
        nos.iter().position(|n| n.faces.contains(&(f as usize)))
    };
    let mut out: Vec<Junta> = vec![];
    for (e, fs) in m.edge_faces.iter().enumerate() {
        if m.edge_smooth.get(e).copied().unwrap_or(1) != 0 {
            continue;
        }
        let (Some(i), Some(j)) = (no_da_face(fs[0]), no_da_face(fs[1])) else { continue };
        if i == j || geo3d::dot(nos[i].n, nos[j].n).abs() > 0.9999 {
            continue;
        }
        let jn = Junta(i.min(j), i.max(j));
        if !out.contains(&jn) {
            out.push(jn);
        }
    }
    out
}

/// Caixa dada por faixas ao longo de três direções ortogonais (`faixas[i]` ao longo de `eixos[i]`).
fn caixa(eixos: [V3; 3], faixas: [[f64; 2]; 3]) -> Result<Shape, String> {
    let k = |e: KernelError| e.0;
    let (x, z) = (eixos[0], eixos[2]);
    let y = fenkra_kernel::cross(z, x);
    // a faixa ao longo de y: a do segundo eixo, com o sinal de y em relação a ele
    let s = if geo3d::dot(y, eixos[1]) >= 0. { 1. } else { -1. };
    let fy = if s > 0. { faixas[1] } else { [-faixas[1][1], -faixas[1][0]] };
    let lo = |f: [f64; 2]| f[0].min(f[1]);
    let len = |f: [f64; 2]| (f[1] - f[0]).abs();
    let o = geo3d::add(geo3d::add(geo3d::mul(x, lo(faixas[0])), geo3d::mul(y, lo(fy))), geo3d::mul(z, lo(faixas[2])));
    Shape::box_tool(o, x, z, [len(faixas[0]), len(fy), len(faixas[2])]).map_err(k)
}

/// Dobra as arestas vivas do corpo (rasgos e alívios nos cantos). `None` quando o corpo não tem
/// aresta viva entre trechos de chapa: nada a fazer.
pub(super) fn dobrar_arestas_vivas(corpo: &Shape, m: &Mesh, a: &AnaliseChapa, face_fixa: V3, r: f64, q: &QualidadeMalha) -> Result<Option<Vivas>, String> {
    let k = |e: KernelError| e.0;
    let t = a.espessura;
    let nos = nos_da_chapa(m, a);
    let js = juntas(m, a, &nos);
    if js.is_empty() {
        return Ok(None);
    }
    // raiz: o nó da face fixa ou o da maior face principal
    let ref_face = if face_fixa != [0.; 3] {
        a.planas.iter().copied().find(|f| {
            let i = &m.face_info[*f];
            geo3d::dot(geo3d::sub(face_fixa, i.point), i.dir).abs() < 1e-3 * t.max(1.)
        })
    } else {
        a.planas.iter().copied().max_by(|x, y| m.face_info[*x].area.total_cmp(&m.face_info[*y].area))
    };
    let ref_face = ref_face.ok_or("a face fixa não é uma face plana da chapa")?;
    let raiz = nos.iter().position(|n| n.faces.contains(&ref_face)).ok_or("a face fixa não está num trecho de chapa")?;
    // árvore: juntas de pai para filho viram dobras; as que fecham laço viram rasgos
    let mut pai: Vec<Option<usize>> = vec![None; nos.len()];
    let mut visto = vec![false; nos.len()];
    let mut usada = vec![false; js.len()];
    let mut dobras: Vec<(usize, usize)> = vec![]; // (pai, filho)
    let mut rasgos: Vec<Junta> = vec![];
    let mut fila = VecDeque::from([raiz]);
    visto[raiz] = true;
    while let Some(p) = fila.pop_front() {
        for (ji, j) in js.iter().enumerate() {
            if usada[ji] || !j.tem(p) {
                continue;
            }
            usada[ji] = true;
            let c = j.outro(p);
            if visto[c] {
                rasgos.push(*j);
                continue;
            }
            visto[c] = true;
            pai[c] = Some(p);
            dobras.push((p, c));
            fila.push_back(c);
        }
    }
    if usada.iter().any(|u| !u) {
        return Err("há paredes de cantos vivos que não se ligam à face fixa: o corpo tem partes soltas".into());
    }
    let centros: Vec<V3> = nos.iter().map(|n| centro(m, n)).collect();
    // lado de fora do nó `n` visto do nó `de`: +1 se `n` fica do lado da normal em relação a `de`
    let lado = |n: usize, de: usize| if nos[n].d - geo3d::dot(centros[de], nos[n].n) >= 0. { 1. } else { -1. };
    let g = (t / 10.).max(0.1);
    let (lo, hi) = corpo.bbox().ok_or("corpo sem dimensões")?;
    let grande = geo3d::dist(lo, hi) + 10. * (t + r);
    let mut novo = corpo.clone();
    // rasgos e alívios
    for j in &rasgos {
        let (ia, ib) = (j.0, j.1);
        let (na, nb) = (nos[ia].n, nos[ib].n);
        let p = match (pai[ia], pai[ib]) {
            (Some(x), Some(y)) if x == y => x,
            _ => return Err("canto vivo entre paredes que não saem da mesma parede: ainda não sei abrir esse canto (arredonde as arestas ou separe as paredes)".into()),
        };
        let np = nos[p].n;
        if geo3d::dot(na, nb).abs() > 1e-3 || geo3d::dot(na, np).abs() > 1e-3 || geo3d::dot(nb, np).abs() > 1e-3 {
            return Err("canto vivo entre paredes que não são perpendiculares: ainda não sei abrir esse canto (arredonde as arestas da dobra)".into());
        }
        // lados de fora: de cada parede em relação ao pai e à outra parede; do pai em relação às paredes
        let (sa, sb, sp) = (lado(ia, p), lado(ib, p), lado(p, ia));
        let (da, db, dp) = (nos[ia].d, nos[ib].d, nos[p].d);
        // faixa ao longo de um nó: de `fora` (além da face externa) até `dentro` (passando da interna)
        let faixa = |d: f64, s: f64, fora: f64, dentro: f64| [d + s * (t / 2. + fora), d - s * (t / 2. + dentro)];
        // alívio: a zona em que as duas dobras se cruzariam, do lado de fora até passar da dobra
        let alivio = caixa([na, nb, np], [faixa(da, sa, 1., r + g), faixa(db, sb, 1., r + g), faixa(dp, sp, 1., r + g)])?;
        // rasgo: a parede B recua `g` a partir da face interna de A, em toda a altura
        let rasgo = caixa([na, nb, np], [[da - sa * t / 2., da - sa * (t / 2. + g)], faixa(db, sb, 1., g), [dp + sp * (t / 2. + 1.), dp - sp * grande]])?;
        novo = novo.boolean(&alivio, Bool::Cut).map_err(k)?.boolean(&rasgo, Bool::Cut).map_err(k)?;
    }
    // dobras: arestas vivas de fora (raio r + t) e de dentro (raio r) entre pai e filho
    let malha = tesselar(&novo, q).map_err(k)?;
    let (mut fora, mut dentro) = (vec![], vec![]);
    for (e, fs) in malha.edge_faces.iter().enumerate() {
        if malha.edge_smooth.get(e).copied().unwrap_or(1) != 0 || fs[0] == NO_FACE || fs[1] == NO_FACE {
            continue;
        }
        let (f0, f1) = (&malha.face_info[fs[0] as usize], &malha.face_info[fs[1] as usize]);
        if f0.kind != 0 || f1.kind != 0 {
            continue;
        }
        let (Some(i), Some(j)) = (no_do_plano(&nos, f0.dir, f0.point, t), no_do_plano(&nos, f1.dir, f1.point, t)) else { continue };
        let Some(&(p, c)) = dobras.iter().find(|(p, c)| (*p == i && *c == j) || (*p == j && *c == i)) else { continue };
        let Some(mid) = malha.edge_mids.get(e * 3..e * 3 + 3).map(|v| [v[0] as f64, v[1] as f64, v[2] as f64]) else { continue };
        // a aresta de fora fica na face externa do pai (do lado oposto ao filho)
        let externa_pai = nos[p].d + lado(p, c) * t / 2.;
        if (geo3d::dot(mid, nos[p].n) - externa_pai).abs() < 0.05 * t + 1e-3 {
            fora.push(mid);
        } else {
            dentro.push(mid);
        }
    }
    if fora.is_empty() || dentro.is_empty() {
        return Err("não achei as arestas vivas das dobras depois de abrir os cantos".into());
    }
    let tol = 0.01 * t.max(1.);
    let (novo, _) = novo.edges_tagged(r + t, &fora, tol, false, &[]).map_err(|e| format!("não consegui fazer as dobras de fora (raio {:.2}): {}", r + t, e.0))?;
    let (novo, _) = novo.edges_tagged(r, &dentro, tol, false, &[]).map_err(|e| format!("não consegui fazer as dobras de dentro (raio {r:.2}): {}", e.0))?;
    Ok(Some(Vivas { corpo: novo, dobras: dobras.len(), rasgos: rasgos.len() }))
}
