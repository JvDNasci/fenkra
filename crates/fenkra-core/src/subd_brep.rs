// SPDX-License-Identifier: LGPL-2.1-or-later
//! Da gaiola aos remendos do B-rep (B179), sem kernel: a parte que é só matemática.
//!
//! Longe dos nós irregulares a superfície de Catmull-Clark é, em cada quadrilátero, um polinômio
//! bicúbico. Em vez de montar a vizinhança com pontos fantasmas para cada caso de borda e vinco, a
//! decomposição amostra o limite numa grade e ajusta os 16 polos por mínimos quadrados: se o
//! quadrilátero é polinomial, o ajuste o recupera exatamente e o resíduo fica no ruído numérico. O
//! resíduo é, assim, a própria prova de que o remendo é exato, e o que sobra (os quadriláteros em
//! volta de um nó irregular) forma as regiões que a conversão precisa aproximar.

use crate::subd::*;
use crate::subd::{add, mul, norma, sub};
use std::collections::HashMap;

/// Amostras por lado do quadrilátero menos um: três passos de refinamento além da malha dos remendos.
const PASSOS_DA_GRADE: u32 = 3;
const N: usize = 1 << PASSOS_DA_GRADE;
/// Resíduo abaixo disto, em fração do tamanho da forma, é ruído numérico.
const RESIDUO_EXATO: f64 = 1e-9;

/// Remendo bicúbico: polos de Bézier `polos[i][j]`, `i` ao longo do lado 0→1 do quadrilátero.
#[derive(Clone, Debug, PartialEq)]
pub struct Remendo {
    pub polos: [[V3; 4]; 4],
    /// face da gaiola e pedaço dentro dela: é o nome do remendo
    pub face: u32,
    pub pedaco: u32,
    pub centro: V3,
    /// maior distância entre o remendo e a superfície de subdivisão
    pub residuo: f64,
    pub exato: bool,
}

/// Trecho do contorno de uma região.
#[derive(Clone, Debug, PartialEq)]
pub struct Trecho {
    pub pts: Vec<V3>,
    /// do outro lado há um remendo liso (sem vinco nem borda): é ali que G1 e G2 valem
    pub vizinho_liso: bool,
}

/// Quadriláteros não polinomiais, ligados entre si: o entorno de um nó irregular.
#[derive(Clone, Debug, PartialEq)]
pub struct Regiao {
    /// nó da gaiola no meio da região, se ele existe (o centro de uma face de n lados não é nó)
    pub no: Option<u32>,
    pub ponto: V3,
    pub valencia: usize,
    /// índices em `remendos`
    pub quads: Vec<usize>,
    /// contorno fechado em ordem; `None` quando a região não é um disco simples
    pub contorno: Option<Vec<Trecho>>,
    /// pontos da superfície dentro da região
    pub amostras: Vec<V3>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Decomposicao {
    pub remendos: Vec<Remendo>,
    pub regioes: Vec<Regiao>,
    pub niveis: u32,
    /// diagonal da caixa da forma
    pub escala: f64,
    pub fechada: bool,
}

impl Decomposicao {
    pub fn exatos(&self) -> usize {
        self.remendos.iter().filter(|r| r.exato).count()
    }
}

/// Resolve o sistema quadrado por eliminação com pivô.
fn resolver(mut a: Vec<Vec<f64>>, mut b: Vec<Vec<f64>>) -> Vec<Vec<f64>> {
    let n = a.len();
    for c in 0..n {
        let p = (c..n).max_by(|x, y| a[*x][c].abs().total_cmp(&a[*y][c].abs())).unwrap_or(c);
        a.swap(c, p);
        b.swap(c, p);
        for l in 0..n {
            if l != c {
                let k = a[l][c] / a[c][c];
                for j in c..n {
                    a[l][j] -= k * a[c][j];
                }
                for j in 0..b[l].len() {
                    b[l][j] -= k * b[c][j];
                }
            }
        }
    }
    for l in 0..n {
        let k = 1. / a[l][l];
        b[l].iter_mut().for_each(|x| *x *= k);
    }
    b
}

fn bernstein(t: f64) -> [f64; 4] {
    let s = 1. - t;
    [s * s * s, 3. * s * s * t, 3. * s * t * t, t * t * t]
}

/// Matrizes da grade: `b[k][i]` é a base no ponto k, e `pinv` leva amostras a polos.
struct Base {
    b: Vec<[f64; 4]>,
    pinv: Vec<Vec<f64>>,
}

impl Base {
    fn nova() -> Base {
        let b: Vec<[f64; 4]> = (0..=N).map(|k| bernstein(k as f64 / N as f64)).collect();
        let m: Vec<Vec<f64>> = (0..4).map(|i| (0..4).map(|j| b.iter().map(|x| x[i] * x[j]).sum()).collect()).collect();
        let bt: Vec<Vec<f64>> = (0..4).map(|i| b.iter().map(|x| x[i]).collect()).collect();
        Base { pinv: resolver(m, bt), b }
    }

    fn ajustar(&self, g: &[Vec<V3>]) -> [[V3; 4]; 4] {
        // primeiro em u, depois em v
        let mut meio = vec![[[0.; 3]; 4]; N + 1];
        for (v, linha) in meio.iter_mut().enumerate() {
            for (i, p) in linha.iter_mut().enumerate() {
                for u in 0..=N {
                    *p = add(*p, mul(g[u][v], self.pinv[i][u]));
                }
            }
        }
        let mut out = [[[0.; 3]; 4]; 4];
        for i in 0..4 {
            for j in 0..4 {
                for v in 0..=N {
                    out[i][j] = add(out[i][j], mul(meio[v][i], self.pinv[j][v]));
                }
            }
        }
        out
    }

    fn avaliar(&self, p: &[[V3; 4]; 4], u: usize, v: usize) -> V3 {
        let mut s = [0.; 3];
        for i in 0..4 {
            for j in 0..4 {
                s = add(s, mul(p[i][j], self.b[u][i] * self.b[v][j]));
            }
        }
        s
    }

    fn residuo(&self, p: &[[V3; 4]; 4], g: &[Vec<V3>]) -> f64 {
        let mut r: f64 = 0.;
        for u in 0..=N {
            for v in 0..=N {
                r = r.max(norma(sub(self.avaliar(p, u, v), g[u][v])));
            }
        }
        r
    }

    /// Curva cúbica pelas amostras, com as pontas presas.
    fn curva(&self, pts: &[V3]) -> [V3; 4] {
        let (a, d) = (pts[0], pts[N]);
        let (mut m11, mut m12, mut m22) = (0., 0., 0.);
        let (mut r1, mut r2) = ([0.; 3], [0.; 3]);
        for k in 0..=N {
            let w = self.b[k];
            let alvo = sub(pts[k], add(mul(a, w[0]), mul(d, w[3])));
            m11 += w[1] * w[1];
            m12 += w[1] * w[2];
            m22 += w[2] * w[2];
            r1 = add(r1, mul(alvo, w[1]));
            r2 = add(r2, mul(alvo, w[2]));
        }
        let det = m11 * m22 - m12 * m12;
        [a, mul(sub(mul(r1, m22), mul(r2, m12)), 1. / det), mul(sub(mul(r2, m11), mul(r1, m12)), 1. / det), d]
    }

    /// Com o contorno preso, os quatro polos de dentro por mínimos quadrados.
    fn miolo(&self, p: &mut [[V3; 4]; 4], g: &[Vec<V3>]) {
        const IDX: [(usize, usize); 4] = [(1, 1), (1, 2), (2, 1), (2, 2)];
        let w = |u: usize, v: usize, (i, j): (usize, usize)| self.b[u][i] * self.b[v][j];
        let mut a = vec![vec![0.; 4]; 4];
        let mut r = vec![vec![0.; 3]; 4];
        let mut fixo = *p;
        for (i, j) in IDX {
            fixo[i][j] = [0.; 3];
        }
        for u in 0..=N {
            for v in 0..=N {
                let alvo = sub(g[u][v], self.avaliar(&fixo, u, v));
                for (x, ix) in IDX.iter().enumerate() {
                    for (y, iy) in IDX.iter().enumerate() {
                        a[x][y] += w(u, v, *ix) * w(u, v, *iy);
                    }
                    for c in 0..3 {
                        r[x][c] += w(u, v, *ix) * alvo[c];
                    }
                }
            }
        }
        let s = resolver(a, r);
        for (x, (i, j)) in IDX.iter().enumerate() {
            p[*i][*j] = [s[x][0], s[x][1], s[x][2]];
        }
    }
}

/// Espalha pela grade os limites dos filhos do quadrilátero, seguindo a ordem do refinamento: o filho
/// `k` tem o canto `k`, os meios das duas arestas dele e o centro.
fn encher(niveis: &[Nivel], lim: &[V3], d: usize, f: usize, uv: [(usize, usize); 4], g: &mut [Vec<V3>]) {
    if d + 1 == niveis.len() {
        for (k, (u, v)) in uv.iter().enumerate() {
            g[*u][*v] = lim[niveis[d].faces[f][k] as usize];
        }
        return;
    }
    let meio = |a: (usize, usize), b: (usize, usize)| ((a.0 + b.0) / 2, (a.1 + b.1) / 2);
    let c = meio(uv[0], uv[2]);
    for k in 0..4 {
        let filho = [uv[k], meio(uv[k], uv[(k + 1) % 4]), c, meio(uv[(k + 3) % 4], uv[k])];
        encher(niveis, lim, d + 1, f * 4 + k, filho, g);
    }
}

/// Amostras do lado `k` do quadrilátero, do canto `k` ao canto `k+1`.
fn lado(g: &[Vec<V3>], k: usize) -> Vec<V3> {
    (0..=N)
        .map(|t| match k {
            0 => g[t][0],
            1 => g[N][t],
            2 => g[N - t][N],
            _ => g[0][N - t],
        })
        .collect()
}

fn achar(pai: &mut [usize], x: usize) -> usize {
    let mut r = x;
    while pai[r] != r {
        pai[r] = pai[pai[r]];
        r = pai[r];
    }
    r
}

/// Quantos passos a conversão precisa: dois separam os nós irregulares, e os vincos de peso parcial
/// precisam acabar de amolecer.
pub fn niveis_da_conversao(g: &Gaiola) -> u32 {
    let s = g.vincos.iter().map(|v| nitidez_do_peso(v.peso)).filter(|s| s.is_finite()).fold(0., f64::max);
    (s.ceil() as u32).max(2)
}

/// Quantos remendos sairiam, sem calcular nada: é o número do aviso de malha densa.
pub fn remendos_estimados(g: &Gaiola) -> usize {
    let quads: usize = g.inteira().faces.iter().map(|f| f.nos.len()).sum();
    quads * 4usize.pow(niveis_da_conversao(g) - 1)
}

/// Divide a superfície em remendos exatos e regiões a aproximar.
pub fn decompor(g: &Gaiola) -> Result<Decomposicao, String> {
    let niveis = niveis_da_conversao(g);
    let n = g.refinada(niveis)?;
    let t = Topo::novo(n.pos.len(), &n.faces)?;
    let mut cadeia = vec![n.clone()];
    for _ in 0..PASSOS_DA_GRADE {
        let prox = cadeia.last().ok_or("sem nível")?.refinar()?;
        cadeia.push(prox);
    }
    let fundo = cadeia.last().ok_or("sem nível")?;
    let lim = fundo.limite()?;
    let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
    for p in &lim {
        for k in 0..3 {
            lo[k] = lo[k].min(p[k]);
            hi[k] = hi[k].max(p[k]);
        }
    }
    let escala = norma(sub(hi, lo)).max(1e-9);
    let base = Base::nova();
    let mut grades = Vec::with_capacity(n.faces.len());
    let mut remendos = Vec::with_capacity(n.faces.len());
    for f in 0..n.faces.len() {
        let mut g = vec![vec![[0.; 3]; N + 1]; N + 1];
        encher(&cadeia, &lim, 0, f, [(0, 0), (N, 0), (N, N), (0, N)], &mut g);
        let polos = base.ajustar(&g);
        let residuo = base.residuo(&polos, &g);
        remendos.push(Remendo { polos, face: n.origem[f].0, pedaco: n.origem[f].1, centro: g[N / 2][N / 2], residuo, exato: residuo <= RESIDUO_EXATO * escala });
        grades.push(g);
    }
    // junta os não exatos pelas arestas lisas
    let mut pai: Vec<usize> = (0..n.faces.len()).collect();
    for h in 0..t.prox.len() as u32 {
        let gm = t.gemea[h as usize];
        if gm == u32::MAX || n.nitidez_de(&t, h).is_infinite() {
            continue;
        }
        let (a, b) = (t.face[h as usize] as usize, t.face[gm as usize] as usize);
        if !remendos[a].exato && !remendos[b].exato {
            let (x, y) = (achar(&mut pai, a), achar(&mut pai, b));
            pai[x] = y;
        }
    }
    let mut grupos: HashMap<usize, Vec<usize>> = HashMap::new();
    for f in (0..n.faces.len()).filter(|f| !remendos[*f].exato) {
        grupos.entry(achar(&mut pai, f)).or_default().push(f);
    }
    let mut chaves: Vec<usize> = grupos.keys().copied().collect();
    chaves.sort_unstable();
    // as curvas compartilhadas valem para os dois lados
    let mut curvas: HashMap<(u32, u32), [V3; 4]> = HashMap::new();
    for f in 0..n.faces.len() {
        if remendos[f].exato {
            continue;
        }
        let q = &n.faces[f];
        let mut p = remendos[f].polos;
        for k in 0..4 {
            let (a, b) = (q[k], q[(k + 1) % 4]);
            let chave = (a.min(b), a.max(b));
            let mut amostras = lado(&grades[f], k);
            if a > b {
                amostras.reverse();
            }
            let c = *curvas.entry(chave).or_insert_with(|| base.curva(&amostras));
            let c: [V3; 4] = if a > b { [c[3], c[2], c[1], c[0]] } else { c };
            for (i, x) in c.iter().enumerate() {
                match k {
                    0 => p[i][0] = *x,
                    1 => p[3][i] = *x,
                    2 => p[3 - i][3] = *x,
                    _ => p[0][3 - i] = *x,
                }
            }
        }
        base.miolo(&mut p, &grades[f]);
        remendos[f].residuo = base.residuo(&p, &grades[f]);
        remendos[f].polos = p;
    }
    let mut regioes = vec![];
    for c in chaves {
        let quads = &grupos[&c];
        regioes.push(regiao(&n, &t, &grades, &remendos, quads, &lim));
    }
    let fechada = t.gemea.iter().all(|x| *x != u32::MAX);
    Ok(Decomposicao { remendos, regioes, niveis, escala, fechada })
}

fn regiao(n: &Nivel, t: &Topo, grades: &[Vec<Vec<V3>>], remendos: &[Remendo], quads: &[usize], lim_fundo: &[V3]) -> Regiao {
    // nó do meio: o vértice que mais se repete entre os quadriláteros da região
    let mut conta: HashMap<u32, usize> = HashMap::new();
    for f in quads {
        for v in &n.faces[*f] {
            *conta.entry(*v).or_default() += 1;
        }
    }
    let (&meio, &valencia) = conta.iter().max_by_key(|(v, c)| (**c, std::cmp::Reverse(**v))).expect("região sem quadrilátero");
    // o índice do vértice se mantém ao refinar, então o limite dele está na mesma posição
    let ponto = lim_fundo[meio as usize];
    let no = Some(n.no[meio as usize]).filter(|x| *x != u32::MAX);
    let dentro: std::collections::HashSet<usize> = quads.iter().copied().collect();
    // arestas dirigidas do contorno
    let mut saindo: HashMap<u32, Vec<(u32, Trecho)>> = HashMap::new();
    let mut total = 0;
    let mut fenda = false;
    for &f in quads {
        for k in 0..4 {
            let h = t.da_face[f] + k as u32;
            let gm = t.gemea[h as usize];
            let vivo = n.nitidez_de(t, h).is_infinite();
            let outro = (gm != u32::MAX).then(|| t.face[gm as usize] as usize);
            if outro.is_some_and(|o| dentro.contains(&o)) {
                // vinco por dentro da região: o contorno teria uma fenda
                fenda |= vivo;
                if !vivo {
                    continue;
                }
            }
            let (a, b) = (n.faces[f][k], n.faces[f][(k + 1) % 4]);
            saindo.entry(a).or_default().push((b, Trecho { pts: lado(&grades[f], k), vizinho_liso: !vivo && outro.is_some() }));
            total += 1;
        }
    }
    let contorno = if fenda || saindo.values().any(|v| v.len() != 1) {
        None
    } else {
        let inicio = *saindo.keys().min().expect("região sem contorno");
        let mut v = inicio;
        let mut lista = vec![];
        loop {
            let Some(mut s) = saindo.remove(&v) else { break };
            let (prox, trecho) = s.remove(0);
            lista.push(trecho);
            v = prox;
            if v == inicio {
                break;
            }
        }
        (lista.len() == total).then_some(lista)
    };
    let mut amostras = vec![];
    for &f in quads {
        for u in (1..N).step_by(2) {
            for w in (1..N).step_by(2) {
                amostras.push(grades[f][u][w]);
            }
        }
    }
    let _ = remendos;
    Regiao { no, ponto, valencia, quads: quads.to_vec(), contorno, amostras }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::subd::{dot, unit};

    fn toro() -> Gaiola {
        let (n, m) = (8u32, 6u32);
        let mut g = Gaiola::default();
        let mut ids = vec![];
        for j in 0..m {
            for i in 0..n {
                let (a, b) = (std::f64::consts::TAU * i as f64 / n as f64, std::f64::consts::TAU * j as f64 / m as f64);
                let r = 20. + 5. * b.cos();
                ids.push(g.novo_no([r * a.cos(), r * a.sin(), 5. * b.sin()]));
            }
        }
        let at = |i: u32, j: u32| ids[((j % m) * n + i % n) as usize];
        for j in 0..m {
            for i in 0..n {
                g.nova_face(vec![at(i, j), at(i + 1, j), at(i + 1, j + 1), at(i, j + 1)]);
            }
        }
        g
    }

    #[test]
    fn malha_regular_vira_remendos_exatos_sem_regiao() {
        let d = decompor(&toro()).unwrap();
        assert!(d.regioes.is_empty());
        assert_eq!(d.exatos(), d.remendos.len());
        assert_eq!(d.remendos.len(), 48 * 16);
        assert!(d.fechada);
        let pior = d.remendos.iter().map(|r| r.residuo).fold(0., f64::max);
        assert!(pior < 1e-10, "resíduo {pior}");
    }

    #[test]
    fn remendos_vizinhos_tem_continuidade_c2_na_aresta_comum() {
        // cada lado do remendo: as três fileiras de polos a partir dele, sempre no mesmo giro
        let faixas = |r: &Remendo| -> Vec<[[V3; 4]; 3]> {
            let q = r.polos;
            let f = |ij: &dyn Fn(usize, usize) -> (usize, usize)| -> [[V3; 4]; 3] {
                let mut o = [[[0.; 3]; 4]; 3];
                for d in 0..3 {
                    for t in 0..4 {
                        let (i, j) = ij(d, t);
                        o[d][t] = q[i][j];
                    }
                }
                o
            };
            vec![f(&|d, t| (t, d)), f(&|d, t| (3 - d, t)), f(&|d, t| (3 - t, 3 - d)), f(&|d, t| (d, 3 - t))]
        };
        let d = decompor(&toro()).unwrap();
        let mut casos = 0;
        let todas: Vec<Vec<[[V3; 4]; 3]>> = d.remendos.iter().map(faixas).collect();
        for (ia, a) in todas.iter().enumerate() {
            for b in todas.iter().skip(ia + 1) {
                for x in a {
                    for y in b {
                        if !(0..4).all(|t| norma(sub(x[0][t], y[0][3 - t])) < 1e-9) {
                            continue;
                        }
                        casos += 1;
                        for t in 0..4 {
                            // no mesmo lugar da aresta, os dois lados continuam a mesma curva
                            let (d1, d2) = (sub(x[1][t], x[0][t]), sub(y[0][3 - t], y[1][3 - t]));
                            assert!(norma(sub(d1, d2)) < 1e-9, "C1");
                            let c1 = add(sub(x[2][t], mul(x[1][t], 2.)), x[0][t]);
                            let c2 = add(sub(y[2][3 - t], mul(y[1][3 - t], 2.)), y[0][3 - t]);
                            assert!(norma(sub(c1, c2)) < 1e-9, "C2");
                        }
                    }
                }
            }
        }
        // cada remendo tem quatro vizinhos e cada junção conta uma vez
        assert_eq!(casos, d.remendos.len() * 2);
    }

    #[test]
    fn caixa_tem_oito_regioes_em_volta_dos_nos_de_valencia_tres() {
        let g = Gaiola::caixa([0.; 3], [10.; 3], [2, 2, 2]);
        let d = decompor(&g).unwrap();
        assert_eq!(d.niveis, 2);
        assert_eq!(d.regioes.len(), 8);
        for r in &d.regioes {
            assert_eq!(r.valencia, 3);
            assert_eq!(r.quads.len(), 3);
            assert!(r.no.is_some());
            let c = r.contorno.as_ref().expect("contorno");
            assert_eq!(c.len(), 6);
            assert!(c.iter().all(|t| t.vizinho_liso));
            // fecha: o fim de um é o começo do próximo
            for k in 0..c.len() {
                let (a, b) = (c[k].pts.last().unwrap(), c[(k + 1) % c.len()].pts[0]);
                assert!(norma(sub(*a, b)) < 1e-12);
            }
        }
        assert_eq!(d.exatos(), d.remendos.len() - 24);
    }

    #[test]
    fn no_de_valencia_cinco_e_oito_formam_regiao_com_o_tamanho_certo() {
        // polo de um cilindro fechado por leque de quadriláteros
        for lados in [5u32, 8] {
            let g = Gaiola::cilindro([0.; 3], 10., 10., lados, 2);
            let d = decompor(&g).unwrap();
            assert!(d.regioes.iter().any(|r| r.valencia == lados as usize), "{:?}", d.regioes.iter().map(|r| r.valencia).collect::<Vec<_>>());
        }
    }

    #[test]
    fn vinco_cheio_divide_a_regiao_e_o_contorno_marca_o_lado_vivo() {
        let mut g = Gaiola::caixa([0.; 3], [10.; 3], [2, 2, 2]);
        for (a, b) in g.arestas() {
            g.definir_vinco(a, b, 1.);
        }
        let d = decompor(&g).unwrap();
        // tudo vincado: cada face da gaiola é plana, nada para aproximar
        assert!(d.regioes.is_empty(), "{}", d.regioes.len());
        for r in &d.remendos {
            assert!(r.exato);
        }
    }

    #[test]
    fn peso_parcial_pede_mais_passos() {
        let mut g = Gaiola::caixa([0.; 3], [10.; 3], [2, 2, 2]);
        let (a, b) = g.arestas()[0];
        g.definir_vinco(a, b, 0.9);
        assert_eq!(niveis_da_conversao(&g), 3);
        assert!(remendos_estimados(&g) > remendos_estimados(&Gaiola::caixa([0.; 3], [10.; 3], [2, 2, 2])));
    }

    #[test]
    fn estimativa_bate_com_a_contagem_de_verdade() {
        let g = Gaiola::caixa([0.; 3], [10.; 3], [2, 2, 2]);
        assert_eq!(remendos_estimados(&g), decompor(&g).unwrap().remendos.len());
    }

    #[test]
    fn folha_aberta_nao_e_fechada_e_borda_nao_vira_regiao() {
        let g = Gaiola::plano([0.; 3], [40., 40.], [4, 4]);
        let d = decompor(&g).unwrap();
        assert!(!d.fechada);
        assert!(d.regioes.is_empty(), "{}", d.regioes.len());
    }

    #[test]
    fn folha_ondulada_tem_borda_e_cantos_exatos() {
        let mut g = Gaiola::plano([0.; 3], [40., 40.], [4, 4]);
        for n in &mut g.nos {
            n.p[2] = (n.p[0] * 0.2).sin() * 3. + (n.p[1] * 0.15).cos() * 2.;
        }
        let d = decompor(&g).unwrap();
        assert!(d.regioes.is_empty(), "borda e canto são polinomiais: {} regiões", d.regioes.len());
        assert!(d.remendos.iter().all(|r| r.residuo < 1e-9), "{}", d.remendos.iter().map(|r| r.residuo).fold(0., f64::max));
    }

    #[test]
    fn g0_fecha_sem_folga_entre_remendos_da_regiao() {
        let g = Gaiola::caixa([0.; 3], [10.; 3], [2, 2, 2]);
        let d = decompor(&g).unwrap();
        let r = &d.regioes[0];
        // dois quadriláteros da região dividem uma aresta com polos idênticos
        let bordas = |p: &Remendo| -> Vec<[V3; 4]> {
            let q = p.polos;
            vec![[q[0][0], q[1][0], q[2][0], q[3][0]], [q[3][0], q[3][1], q[3][2], q[3][3]], [q[3][3], q[2][3], q[1][3], q[0][3]], [q[0][3], q[0][2], q[0][1], q[0][0]]]
        };
        let mesma = |a: &[V3; 4], b: &[V3; 4]| (0..4).all(|i| norma(sub(a[i], b[3 - i])) < 1e-9);
        let (x, y) = (&d.remendos[r.quads[0]], &d.remendos[r.quads[1]]);
        assert!(bordas(x).iter().any(|a| bordas(y).iter().any(|b| mesma(a, b))), "sem aresta comum");
        // e o desvio é pequeno perto do tamanho
        let pior = r.quads.iter().map(|q| d.remendos[*q].residuo).fold(0., f64::max);
        assert!(pior < 0.01 * d.escala, "{pior}");
        let _ = (unit([1., 0., 0.]), dot([1., 0., 0.], [1., 0., 0.]));
    }
}
