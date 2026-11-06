// SPDX-License-Identifier: LGPL-2.1-or-later
//! Reparo de malha, em passos que não dependem de quem veio antes para fazerem sentido:
//!
//! 1. solda, para que triângulos encostados virem vizinhos;
//! 2. tira degenerados e duplicados;
//! 3. desfaz não manifold: em cada aresta com três faces ou mais, as faces são pareadas pelo ângulo em
//!    volta da aresta e, em cada vértice, cada leque de faces ganha a sua cópia do vértice. Nenhum
//!    triângulo é apagado: duas peças que se tocavam por uma aresta só ficam separadas;
//! 4. orienta: cada componente é percorrido pelas arestas virando quem discorda do vizinho, e a casca
//!    fechada fica com o volume positivo, ou negativo se estiver dentro de outra (é uma cavidade);
//! 5. fecha furos pelos laços de borda, recortando orelhas no plano médio do laço;
//! 6. tira as partes soltas pequenas.

use crate::diagnostico::{arestas, componentes_de, lacos_de_borda};
use crate::{add, cross, dot, mul, norma, normal_area, sub, unit, Bvh, Malha, V3};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OpcoesReparo {
    /// distância da solda em mm; zero junta só o que é idêntico
    pub solda: f64,
    pub nao_manifold: bool,
    pub normais: bool,
    pub furos: bool,
    /// maior furo que o reparo fecha, em arestas; zero é sem limite
    pub furo_maximo: usize,
    pub soltas: bool,
    /// parte solta que fica: fração da área da maior parte
    pub solta_minima: f64,
}

impl Default for OpcoesReparo {
    fn default() -> Self {
        OpcoesReparo { solda: 0., nao_manifold: true, normais: true, furos: true, furo_maximo: 0, soltas: false, solta_minima: 0.001 }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RelatorioReparo {
    pub vertices_soldados: usize,
    pub triangulos_removidos: usize,
    pub vertices_separados: usize,
    pub triangulos_virados: usize,
    pub furos_fechados: usize,
    pub furos_abertos: usize,
    pub partes_removidas: usize,
}

/// Tira triângulos sem área ou repetidos.
fn limpar(m: &mut Malha) -> usize {
    let antes = m.tris.len();
    let escala = m.diagonal().max(1e-9);
    let mut vistos = HashSet::with_capacity(m.tris.len());
    let pos = &m.pos;
    m.tris.retain(|t| {
        let mut k = *t;
        k.sort_unstable();
        k[0] != k[1] && k[1] != k[2] && vistos.insert(k) && norma(normal_area(pos[t[0] as usize], pos[t[1] as usize], pos[t[2] as usize])) > 1e-14 * escala * escala
    });
    antes - m.tris.len()
}

fn raiz(pai: &mut [u32], mut x: u32) -> u32 {
    while pai[x as usize] != x {
        pai[x as usize] = pai[pai[x as usize] as usize];
        x = pai[x as usize];
    }
    x
}

/// Ângulo da face em volta da aresta `a`-`b`, medido por um terceiro vértice qualquer.
fn angulo_na_aresta(m: &Malha, f: u32, a: u32, b: u32, base: (V3, V3, V3)) -> f64 {
    let t = m.tris[f as usize];
    let o = t.iter().copied().find(|v| *v != a && *v != b).unwrap_or(t[0]);
    let d = sub(m.pos[o as usize], m.pos[a as usize]);
    dot(d, base.2).atan2(dot(d, base.1))
}

/// Separa o que está ligado por aresta com mais de duas faces ou por um vértice só.
fn desfazer_nao_manifold(m: &mut Malha) -> usize {
    let mapa = arestas(m);
    // canto = face * 3 + posição do vértice; cantos que ficam juntos são o mesmo vértice final
    let mut pai: Vec<u32> = (0..m.tris.len() as u32 * 3).collect();
    let canto = |m: &Malha, f: u32, v: u32| f * 3 + m.tris[f as usize].iter().position(|x| *x == v).unwrap_or(0) as u32;
    let ligar = |m: &Malha, pai: &mut Vec<u32>, f: u32, g: u32, a: u32, b: u32| {
        for v in [a, b] {
            let (x, y) = (raiz(pai, canto(m, f, v)), raiz(pai, canto(m, g, v)));
            pai[x as usize] = y;
        }
    };
    for (&(a, b), faces) in &mapa {
        match faces[..] {
            [_] => {}
            [(f, _), (g, _)] => ligar(m, &mut pai, f, g, a, b),
            _ => {
                // em volta da aresta, cada face casa com a vizinha do lado de dentro do volume
                let eixo = unit(sub(m.pos[b as usize], m.pos[a as usize]));
                let u = unit(cross(eixo, if eixo[0].abs() < 0.9 { [1., 0., 0.] } else { [0., 1., 0.] }));
                let base = (eixo, u, cross(eixo, u));
                let mut lista: Vec<(f64, u32, bool)> = faces.iter().map(|(f, d)| (angulo_na_aresta(m, *f, a, b, base), *f, *d)).collect();
                lista.sort_by(|x, y| x.0.total_cmp(&y.0));
                let n = lista.len();
                // começa numa face cuja seguinte corre em sentido contrário: é o par que fecha uma peça
                let inicio = (0..n).find(|i| lista[*i].2 != lista[(i + 1) % n].2).unwrap_or(0);
                let mut i = 0;
                while i + 1 < n {
                    let (p, q) = (lista[(inicio + i) % n], lista[(inicio + i + 1) % n]);
                    ligar(m, &mut pai, p.1, q.1, a, b);
                    i += 2;
                }
            }
        }
    }
    let mut novos: HashMap<(u32, u32), u32> = HashMap::new();
    let mut pos = Vec::with_capacity(m.pos.len());
    let mut usados = vec![false; m.pos.len()];
    let mut separados = 0;
    let mut tris = m.tris.clone();
    for f in 0..m.tris.len() {
        for k in 0..3 {
            let v = m.tris[f][k];
            let g = raiz(&mut pai, f as u32 * 3 + k as u32);
            tris[f][k] = *novos.entry((v, g)).or_insert_with(|| {
                separados += usize::from(usados[v as usize]);
                usados[v as usize] = true;
                pos.push(m.pos[v as usize]);
                (pos.len() - 1) as u32
            });
        }
    }
    m.pos = pos;
    m.tris = tris;
    m.cor_v.clear();
    m.nrm.clear();
    separados
}

/// Vira os triângulos que discordam do vizinho, componente por componente.
fn orientar(m: &mut Malha) -> usize {
    let mapa = arestas(m);
    let mut vizinhos: Vec<Vec<(u32, bool)>> = vec![Vec::new(); m.tris.len()];
    for faces in mapa.values() {
        if let [(f, a), (g, b)] = faces[..] {
            // `a == b`: as duas percorrem a aresta no mesmo sentido, então uma tem de virar
            vizinhos[f as usize].push((g, a == b));
            vizinhos[g as usize].push((f, a == b));
        }
    }
    let mut estado: Vec<Option<bool>> = vec![None; m.tris.len()];
    for inicio in 0..m.tris.len() {
        if estado[inicio].is_some() {
            continue;
        }
        estado[inicio] = Some(false);
        let mut pilha = vec![inicio as u32];
        while let Some(f) = pilha.pop() {
            let vira = estado[f as usize].unwrap_or(false);
            for &(g, discorda) in &vizinhos[f as usize] {
                if estado[g as usize].is_none() {
                    estado[g as usize] = Some(vira ^ discorda);
                    pilha.push(g);
                }
            }
        }
    }
    let mut virados = 0;
    for (t, e) in m.tris.iter_mut().zip(&estado) {
        if *e == Some(true) {
            t.swap(1, 2);
            virados += 1;
        }
    }
    virados
}

/// Casca fechada fica com volume positivo; a que está dentro de outra, negativo. Sem isto a peça sai
/// do avesso, e uma cavidade viraria um sólido dentro do sólido.
fn orientar_cascas(m: &mut Malha) -> usize {
    const MAXIMO: usize = 256;
    let (ids, n) = componentes_de(m);
    if n == 0 || n > MAXIMO {
        return 0;
    }
    let partes: Vec<Vec<usize>> = {
        let mut p = vec![Vec::new(); n];
        ids.iter().enumerate().for_each(|(f, c)| p[*c as usize].push(f));
        p
    };
    let mapa = arestas(m);
    let mut livre = vec![false; n];
    for faces in mapa.values() {
        if faces.len() != 2 {
            livre[ids[faces[0].0 as usize] as usize] = true;
        }
    }
    let sub_malha = |lista: &[usize]| Malha { pos: m.pos.clone(), tris: lista.iter().map(|f| m.tris[*f]).collect(), ..Default::default() };
    let cascas: Vec<(Bvh, f64, V3)> = partes
        .iter()
        .map(|l| {
            let s = sub_malha(l);
            (Bvh::nova(&s), s.volume(), m.pos[m.tris[l[0]][0] as usize])
        })
        .collect();
    let mut virados = 0;
    for c in 0..n {
        if livre[c] {
            continue;
        }
        let profundidade = (0..n).filter(|o| *o != c && !livre[*o] && cascas[*o].1.abs() > cascas[c].1.abs() && cascas[*o].0.contem(cascas[c].2)).count();
        let quer_positivo = profundidade % 2 == 0;
        if (cascas[c].1 > 0.) != quer_positivo {
            for f in &partes[c] {
                m.tris[*f].swap(1, 2);
            }
            virados += partes[c].len();
        }
    }
    virados
}

/// Polígono do laço no plano médio dele.
fn plano_do_laco(m: &Malha, laco: &[u32]) -> (V3, V3, V3) {
    let p: Vec<V3> = laco.iter().map(|v| m.pos[*v as usize]).collect();
    let mut n = [0.; 3];
    for i in 0..p.len() {
        n = add(n, cross(p[i], p[(i + 1) % p.len()]));
    }
    let n = unit(n);
    let u = unit(cross(n, if n[0].abs() < 0.9 { [1., 0., 0.] } else { [0., 1., 0.] }));
    (n, u, cross(n, u))
}

/// Triângulos que fecham o laço, no sentido contrário ao das faces em volta. Recorta orelhas no plano
/// médio; laço torcido demais para isso ganha um vértice no centro e um leque.
fn fechar_laco(m: &mut Malha, laco: &[u32]) -> Vec<[u32; 3]> {
    let (_, u, v) = plano_do_laco(m, laco);
    // o laço corre no sentido das faces: o furo visto de fora gira ao contrário
    let mut resto: Vec<u32> = laco.iter().rev().copied().collect();
    let plano = |m: &Malha, i: u32| {
        let p = m.pos[i as usize];
        [dot(p, u), dot(p, v)]
    };
    let area2 = |a: [f64; 2], b: [f64; 2], c: [f64; 2]| (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
    let soma: f64 = (0..resto.len()).map(|i| {
        let (a, b) = (plano(m, resto[i]), plano(m, resto[(i + 1) % resto.len()]));
        a[0] * b[1] - a[1] * b[0]
    }).sum();
    let sinal = if soma >= 0. { 1. } else { -1. };
    let mut tris = Vec::with_capacity(resto.len());
    while resto.len() > 3 {
        let n = resto.len();
        let mut melhor: Option<(usize, f64)> = None;
        for i in 0..n {
            let (ia, ib, ic) = (resto[(i + n - 1) % n], resto[i], resto[(i + 1) % n]);
            let (a, b, c) = (plano(m, ia), plano(m, ib), plano(m, ic));
            if area2(a, b, c) * sinal <= 0. {
                continue;
            }
            let dentro = resto.iter().any(|&q| {
                if q == ia || q == ib || q == ic {
                    return false;
                }
                let p = plano(m, q);
                area2(a, b, p) * sinal >= 0. && area2(b, c, p) * sinal >= 0. && area2(c, a, p) * sinal >= 0.
            });
            if dentro {
                continue;
            }
            // orelha de lados parecidos: evita lascas compridas no remendo
            let lado = |x: [f64; 2], y: [f64; 2]| ((x[0] - y[0]).powi(2) + (x[1] - y[1]).powi(2)).sqrt();
            let qualidade = area2(a, b, c).abs() / (lado(a, b) + lado(b, c) + lado(c, a)).powi(2).max(1e-300);
            if melhor.is_none_or(|(_, q)| qualidade > q) {
                melhor = Some((i, qualidade));
            }
        }
        let Some((i, _)) = melhor else {
            let centro = mul(resto.iter().fold([0.; 3], |s, q| add(s, m.pos[*q as usize])), 1. / resto.len() as f64);
            m.pos.push(centro);
            let c = (m.pos.len() - 1) as u32;
            for k in 0..resto.len() {
                tris.push([resto[k], resto[(k + 1) % resto.len()], c]);
            }
            return tris;
        };
        tris.push([resto[(i + n - 1) % n], resto[i], resto[(i + 1) % n]]);
        resto.remove(i);
    }
    tris.push([resto[0], resto[1], resto[2]]);
    tris
}

fn fechar_furos(m: &mut Malha, maximo: usize, r: &mut RelatorioReparo) {
    for laco in lacos_de_borda(m) {
        if maximo > 0 && laco.len() > maximo {
            r.furos_abertos += 1;
            continue;
        }
        let novos = fechar_laco(m, &laco);
        m.tris.extend(novos);
        r.furos_fechados += 1;
    }
}

/// Tira partes menores que `fracao` da área da maior.
fn tirar_soltas(m: &mut Malha, fracao: f64) -> usize {
    let (ids, n) = componentes_de(m);
    let mut areas = vec![0.; n];
    for (t, c) in m.tris.iter().zip(&ids) {
        let [a, b, d] = t.map(|i| m.pos[i as usize]);
        areas[*c as usize] += norma(normal_area(a, b, d)) / 2.;
    }
    let maior = areas.iter().copied().fold(0., f64::max);
    let fica: Vec<bool> = areas.iter().map(|a| *a >= maior * fracao).collect();
    let mut i = 0;
    m.tris.retain(|_| {
        i += 1;
        fica[ids[i - 1] as usize]
    });
    fica.iter().filter(|f| !**f).count()
}

/// Tira vértices que nenhum triângulo usa.
fn sem_orfaos(m: &mut Malha) {
    let mut novo = vec![u32::MAX; m.pos.len()];
    let mut pos = Vec::with_capacity(m.pos.len());
    for t in &mut m.tris {
        for v in t.iter_mut() {
            if novo[*v as usize] == u32::MAX {
                novo[*v as usize] = pos.len() as u32;
                pos.push(m.pos[*v as usize]);
            }
            *v = novo[*v as usize];
        }
    }
    m.pos = pos;
}

impl Malha {
    /// Malha reparada, com o que foi feito. A entrada não muda.
    pub fn reparada(&self, o: &OpcoesReparo) -> (Malha, RelatorioReparo) {
        let mut r = RelatorioReparo::default();
        let mut m = self.soldada_com(o.solda);
        r.vertices_soldados = self.pos.len().saturating_sub(m.pos.len());
        r.triangulos_removidos = self.tris.len().saturating_sub(m.tris.len()) + limpar(&mut m);
        if o.nao_manifold {
            r.vertices_separados = desfazer_nao_manifold(&mut m);
        }
        if o.normais || o.furos {
            r.triangulos_virados = orientar(&mut m);
        }
        if o.furos {
            fechar_furos(&mut m, o.furo_maximo, &mut r);
        }
        if o.normais {
            r.triangulos_virados += orientar_cascas(&mut m);
        }
        if o.soltas {
            r.partes_removidas = tirar_soltas(&mut m, o.solta_minima);
        }
        sem_orfaos(&mut m);
        (m, r)
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::teste::*;

    fn tudo() -> OpcoesReparo {
        OpcoesReparo { soltas: true, ..Default::default() }
    }

    #[test]
    fn malha_sa_nao_muda() {
        let c = caixa([10., 10., 10.]);
        let (m, r) = c.reparada(&tudo());
        assert_eq!(r, RelatorioReparo::default());
        assert_eq!(m.tris.len(), 12);
        assert!((m.volume() - 1000.).abs() < 1e-9);
    }

    #[test]
    fn furo_fecha_e_o_volume_volta() {
        let mut e = esfera(10., 24);
        let inteira = e.volume();
        // tira a calota de cima: um furo com 48 arestas
        e.tris.retain(|t| !t.contains(&0));
        assert_eq!(e.diagnostico().furos, 1);
        let (m, r) = e.reparada(&tudo());
        assert_eq!(r.furos_fechados, 1);
        assert!(m.diagnostico().estanque(), "{:?}", m.diagnostico());
        assert!((m.volume() - inteira).abs() / inteira < 0.01, "{} de {inteira}", m.volume());
    }

    #[test]
    fn furo_maior_que_o_limite_fica_aberto() {
        let mut e = esfera(10., 24);
        e.tris.retain(|t| !t.contains(&0));
        let (m, r) = e.reparada(&OpcoesReparo { furo_maximo: 8, ..Default::default() });
        assert_eq!((r.furos_fechados, r.furos_abertos), (0, 1));
        assert_eq!(m.diagnostico().furos, 1);
    }

    #[test]
    fn furo_nao_convexo_fecha_sem_cruzar() {
        // tampa em L: a orelha de um polígono côncavo não pode passar por fora dele
        let contorno: [[f64; 2]; 6] = [[0., 0.], [20., 0.], [20., 10.], [10., 10.], [10., 20.], [0., 20.]];
        let mut m = Malha::default();
        let n = contorno.len() as u32;
        for p in contorno {
            m.pos.push([p[0], p[1], 0.]);
            m.pos.push([p[0], p[1], 5.]);
        }
        let (b, t) = (|i: u32| 2 * (i % n), |i: u32| 2 * (i % n) + 1);
        for i in 0..n {
            m.tris.push([b(i), b(i + 1), t(i + 1)]);
            m.tris.push([b(i), t(i + 1), t(i)]);
        }
        let (r, rel) = m.reparada(&tudo());
        assert_eq!(rel.furos_fechados, 2);
        assert!(r.diagnostico().estanque(), "{:?}", r.diagnostico());
        assert!((r.volume() - 300. * 5.).abs() < 1e-6, "{}", r.volume());
    }

    #[test]
    fn triangulos_virados_e_avesso_voltam() {
        let mut c = caixa([10., 10., 10.]);
        c.tris[3].swap(1, 2);
        c.tris[7].swap(1, 2);
        let (m, r) = c.reparada(&tudo());
        assert!(r.triangulos_virados >= 2);
        assert!(m.diagnostico().estanque() && (m.volume() - 1000.).abs() < 1e-9, "{:?}", m.diagnostico());

        let mut avesso = caixa([10., 10., 10.]);
        avesso.tris.iter_mut().for_each(|t| t.swap(1, 2));
        assert!(avesso.diagnostico().virada_para_dentro());
        assert!((avesso.reparada(&tudo()).0.volume() - 1000.).abs() < 1e-9);
    }

    #[test]
    fn cavidade_fica_com_o_sinal_de_cavidade() {
        let externa = caixa([30., 30., 30.]);
        let mut interna = caixa([10., 10., 10.]);
        interna.pos.iter_mut().for_each(|p| *p = p.map(|v| v + 10.));
        let (m, _) = crate::juntar([&externa, &interna]).reparada(&OpcoesReparo::default());
        assert!((m.volume() - (27000. - 1000.)).abs() < 1e-6, "{}", m.volume());
    }

    #[test]
    fn peca_que_toca_em_vertice_e_aresta_separa_sem_perder_triangulo() {
        let a = caixa([10., 10., 10.]);
        let mut b = caixa([10., 10., 10.]);
        b.pos.iter_mut().for_each(|p| *p = p.map(|v| v + 10.));
        let j = crate::juntar([&a, &b]).soldada();
        assert_eq!(j.diagnostico().vertices_nao_manifold, 1);
        let (m, r) = j.reparada(&OpcoesReparo::default());
        assert!(r.vertices_separados >= 1);
        assert_eq!(m.tris.len(), 24);
        let d = m.diagnostico();
        assert!(d.estanque() && d.componentes == 2, "{d:?}");

        // duas caixas pela aresta: quatro faces na aresta comum, duas de cada peça
        let mut b = caixa([10., 10., 10.]);
        b.pos.iter_mut().for_each(|p| *p = [p[0] + 10., p[1] + 10., p[2]]);
        let j = crate::juntar([&a, &b]).soldada();
        assert_eq!(j.diagnostico().arestas_nao_manifold, 1);
        let (m, _) = j.reparada(&OpcoesReparo::default());
        let d = m.diagnostico();
        assert!(d.estanque() && d.componentes == 2, "{d:?}");
        assert!((m.volume() - 2000.).abs() < 1e-9);
    }

    #[test]
    fn parte_solta_pequena_sai_e_a_grande_fica() {
        let a = caixa([100., 100., 100.]);
        let mut b = caixa([1., 1., 1.]);
        b.pos.iter_mut().for_each(|p| p[0] += 200.);
        let (m, r) = crate::juntar([&a, &b]).reparada(&tudo());
        assert_eq!(r.partes_removidas, 1);
        assert_eq!(m.tris.len(), 12);
        assert_eq!(m.pos.len(), 8);
    }

    #[test]
    fn triangulos_soltos_com_folga_viram_casca() {
        let c = caixa([10., 10., 10.]);
        let mut sopa = Malha::default();
        for t in &c.tris {
            let b = sopa.pos.len() as u32;
            sopa.pos.extend(t.map(|i| c.pos[i as usize].map(|v| v + 1e-5 * f64::from(b % 7))));
            sopa.tris.push([b, b + 1, b + 2]);
        }
        let (m, _) = sopa.reparada(&OpcoesReparo { solda: 1e-3, ..Default::default() });
        assert!(m.diagnostico().estanque());
        assert!((m.volume() - 1000.).abs() < 0.5);
    }
}
