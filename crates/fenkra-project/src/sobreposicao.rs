// SPDX-License-Identifier: LGPL-2.1-or-later
//! Sobreposição de duas versões de um modelo (B214, parte do 3D) e prévia lado a lado de uma
//! mesclagem (B130): quais regiões da malha existem só numa das versões.
//!
//! Como funciona, sem kernel e sem booleana:
//! 1. as malhas das duas versões viram listas de triângulos no mesmo espaço (as versões do mesmo
//!    documento compartilham a origem, então a sobreposição já está alinhada);
//! 2. cada triângulo de um lado é amostrado em 7 pontos (3 vértices, 3 pontos médios e o centro) e
//!    cada ponto é medido contra a malha do outro lado, por uma grade uniforme (só os triângulos
//!    vizinhos entram na conta);
//! 3. o triângulo (ou, na borda da mudança, o pedaço dele depois de dividido em 4 até a aresta caber em 1/48
//!    da diagonal) é "só deste lado" quando a maioria das amostras está a mais de `tolerancia` da
//!    superfície do outro lado. `tolerancia` cobre o erro de tesselação, que muda um pouco de uma
//!    versão para outra em superfície curva (padrão: 0,4 % da diagonal, no mínimo 0,05 mm).
//!
//! O que sai: triângulos só de `antes` (removido), só de `depois` (adicionado) e a contagem dos
//! comuns. Quem desenha é o aplicativo, com cores do tema. Limite honesto: mudança menor que a
//! tolerância não aparece, e uma face que só mudou de lugar na normal (espelhada) conta como igual.

use std::collections::HashMap;

/// Uma malha: posições (x, y, z em sequência) e índices de triângulos.
#[derive(Clone, Copy)]
pub struct MalhaRef<'a> {
    pub positions: &'a [f32],
    pub indices: &'a [u32],
}

type V = [f64; 3];
type Tri = [V; 3];

#[derive(Clone, Debug, Default)]
pub struct Sobreposicao {
    /// triângulos só de `antes`, 9 números cada (3 vértices)
    pub removido: Vec<f32>,
    /// triângulos só de `depois`
    pub adicionado: Vec<f32>,
    pub triangulos_comuns: usize,
    pub area_removida: f64,
    pub area_adicionada: f64,
    pub tolerancia: f64,
}

impl Sobreposicao {
    pub fn e_igual(&self) -> bool {
        self.removido.is_empty() && self.adicionado.is_empty()
    }
}

fn sub(a: V, b: V) -> V {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn dot(a: V, b: V) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn add_esc(a: V, b: V, k: f64) -> V {
    [a[0] + b[0] * k, a[1] + b[1] * k, a[2] + b[2] * k]
}
fn cruz(a: V, b: V) -> V {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

fn triangulos(m: &[MalhaRef]) -> Vec<Tri> {
    let mut out = vec![];
    for malha in m {
        let p = |i: u32| {
            let k = i as usize * 3;
            [f64::from(malha.positions[k]), f64::from(malha.positions[k + 1]), f64::from(malha.positions[k + 2])]
        };
        for t in malha.indices.chunks_exact(3) {
            if t.iter().all(|&i| (i as usize) * 3 + 2 < malha.positions.len()) {
                out.push([p(t[0]), p(t[1]), p(t[2])]);
            }
        }
    }
    out
}

fn area(t: &Tri) -> f64 {
    let c = cruz(sub(t[1], t[0]), sub(t[2], t[0]));
    0.5 * dot(c, c).sqrt()
}

/// Ponto do triângulo mais próximo de `p` (Ericson, *Real-Time Collision Detection*, 5.1.5).
fn mais_proximo(p: V, t: &Tri) -> V {
    let (a, b, c) = (t[0], t[1], t[2]);
    let (ab, ac, ap) = (sub(b, a), sub(c, a), sub(p, a));
    let (d1, d2) = (dot(ab, ap), dot(ac, ap));
    if d1 <= 0. && d2 <= 0. {
        return a;
    }
    let bp = sub(p, b);
    let (d3, d4) = (dot(ab, bp), dot(ac, bp));
    if d3 >= 0. && d4 <= d3 {
        return b;
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0. && d1 >= 0. && d3 <= 0. {
        return add_esc(a, ab, d1 / (d1 - d3));
    }
    let cp = sub(p, c);
    let (d5, d6) = (dot(ab, cp), dot(ac, cp));
    if d6 >= 0. && d5 <= d6 {
        return c;
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0. && d2 >= 0. && d6 <= 0. {
        return add_esc(a, ac, d2 / (d2 - d6));
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0. && (d4 - d3) >= 0. && (d5 - d6) >= 0. {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return add_esc(b, sub(c, b), w);
    }
    let den = 1. / (va + vb + vc);
    add_esc(add_esc(a, ab, vb * den), ac, vc * den)
}

/// Grade uniforme: cada célula guarda os triângulos cuja caixa, alargada pela tolerância, a toca.
struct Grade {
    tris: Vec<Tri>,
    celula: f64,
    origem: V,
    mapa: HashMap<(i32, i32, i32), Vec<u32>>,
}

impl Grade {
    fn nova(tris: Vec<Tri>, origem: V, celula: f64, folga: f64) -> Grade {
        let mut g = Grade { tris: vec![], celula, origem, mapa: HashMap::new() };
        for (i, t) in tris.iter().enumerate() {
            let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
            for v in t {
                for k in 0..3 {
                    lo[k] = lo[k].min(v[k] - folga);
                    hi[k] = hi[k].max(v[k] + folga);
                }
            }
            let (a, b) = (g.cel(lo), g.cel(hi));
            for x in a.0..=b.0 {
                for y in a.1..=b.1 {
                    for z in a.2..=b.2 {
                        g.mapa.entry((x, y, z)).or_default().push(i as u32);
                    }
                }
            }
        }
        g.tris = tris;
        g
    }

    fn cel(&self, p: V) -> (i32, i32, i32) {
        let f = |k: usize| ((p[k] - self.origem[k]) / self.celula).floor() as i32;
        (f(0), f(1), f(2))
    }

    /// O ponto está a no máximo `tol` de algum triângulo da grade?
    fn perto(&self, p: V, tol: f64) -> bool {
        let Some(l) = self.mapa.get(&self.cel(p)) else { return false };
        let q = tol * tol;
        l.iter().any(|&i| {
            let c = mais_proximo(p, &self.tris[i as usize]);
            let d = sub(p, c);
            dot(d, d) <= q
        })
    }
}

fn amostras(t: &Tri) -> [V; 7] {
    let m = |a: V, b: V| [(a[0] + b[0]) / 2., (a[1] + b[1]) / 2., (a[2] + b[2]) / 2.];
    let c = [(t[0][0] + t[1][0] + t[2][0]) / 3., (t[0][1] + t[1][1] + t[2][1]) / 3., (t[0][2] + t[1][2] + t[2][2]) / 3.];
    [t[0], t[1], t[2], m(t[0], t[1]), m(t[1], t[2]), m(t[2], t[0]), c]
}

fn maior_aresta(t: &Tri) -> f64 {
    let d = |a: V, b: V| {
        let v = sub(a, b);
        dot(v, v).sqrt()
    };
    d(t[0], t[1]).max(d(t[1], t[2])).max(d(t[2], t[0]))
}

fn meio(a: V, b: V) -> V {
    [(a[0] + b[0]) / 2., (a[1] + b[1]) / 2., (a[2] + b[2]) / 2.]
}

/// Separa o que está além de `tol` da malha `contra`. Triângulo todo perto ou todo longe é decidido
/// pelas 7 amostras; o que fica no meio (a borda do que mudou) é dividido em 4 até a aresta caber em
/// `lim`, e só então decide pela maioria das amostras. Assim a região mostrada acompanha a borda da
/// mudança em vez de pegar o triângulo grande inteiro. Devolve se algum pedaço ficou longe.
fn classificar(t: &Tri, contra: &Grade, tol: f64, lim: f64, prof: u8, longe: &mut Vec<Tri>) -> bool {
    let n = amostras(t).iter().filter(|p| !contra.perto(**p, tol)).count();
    if n == 0 {
        return false;
    }
    if n == 7 {
        longe.push(*t);
        return true;
    }
    if prof == 0 || maior_aresta(t) <= lim {
        if n * 2 > 7 {
            longe.push(*t);
            return true;
        }
        return false;
    }
    let (ab, bc, ca) = (meio(t[0], t[1]), meio(t[1], t[2]), meio(t[2], t[0]));
    let mut algum = false;
    for sub in [[t[0], ab, ca], [ab, t[1], bc], [ca, bc, t[2]], [ab, bc, ca]] {
        algum |= classificar(&sub, contra, tol, lim, prof - 1, longe);
    }
    algum
}

/// Triângulos de `de` (ou pedaços deles) que ficam a mais de `tol` da malha `contra`, e quantos dos
/// triângulos de `de` ficaram inteiros perto dela.
fn separar(de: &[Tri], contra: &Grade, tol: f64, lim: f64) -> (Vec<Tri>, usize) {
    let (mut longe, mut comuns) = (vec![], 0);
    for t in de {
        if !classificar(t, contra, tol, lim, 7, &mut longe) {
            comuns += 1;
        }
    }
    (longe, comuns)
}

fn plano(ts: &[Tri]) -> Vec<f32> {
    ts.iter().flat_map(|t| t.iter().flat_map(|v| v.iter().map(|x| *x as f32))).collect()
}

/// Compara `antes` e `depois`. `tolerancia` em mm; `None` usa 0,4 % da diagonal (mínimo 0,05 mm).
pub fn sobrepor(antes: &[MalhaRef], depois: &[MalhaRef], tolerancia: Option<f64>) -> Sobreposicao {
    let (a, d) = (triangulos(antes), triangulos(depois));
    let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
    for v in a.iter().chain(d.iter()).flatten() {
        for k in 0..3 {
            lo[k] = lo[k].min(v[k]);
            hi[k] = hi[k].max(v[k]);
        }
    }
    if a.is_empty() && d.is_empty() {
        return Sobreposicao::default();
    }
    let diag = (0..3).map(|k| (hi[k] - lo[k]).powi(2)).sum::<f64>().sqrt();
    let tol = tolerancia.unwrap_or_else(|| (diag * 0.004).max(0.05));
    // células de ~8 tolerâncias: poucas células por triângulo, poucos triângulos por célula
    let celula = (tol * 8.).max(diag / 96.).max(1e-6);
    let g_antes = Grade::nova(a.clone(), lo, celula, tol);
    let g_depois = Grade::nova(d.clone(), lo, celula, tol);
    let lim = (diag / 48.).max(tol * 8.);
    let (removido, comuns_a) = separar(&a, &g_depois, tol, lim);
    let (adicionado, comuns_d) = separar(&d, &g_antes, tol, lim);
    Sobreposicao {
        area_removida: removido.iter().map(area).sum(),
        area_adicionada: adicionado.iter().map(area).sum(),
        removido: plano(&removido),
        adicionado: plano(&adicionado),
        triangulos_comuns: comuns_a.min(comuns_d).max(comuns_a.max(comuns_d) / 2),
        tolerancia: tol,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fenkra_core::{BoolOp, Document, FeatureKind, OriginPlane, PlaneRef, PrimKind, Primitive};

    fn caixa(lado: &str) -> Document {
        let s = |x: &str| x.to_string();
        let mut d = Document::new("Placa");
        d.add_feature(FeatureKind::Primitive(Primitive { kind: PrimKind::Caixa, plane: PlaneRef::Origin(OriginPlane::XY), a: s(lado), b: s("20"), c: s("5"), d: s("0"), op: BoolOp::Add }));
        d
    }

    fn malhas(d: &Document) -> Vec<(Vec<f32>, Vec<u32>)> {
        let r = fenkra_solidos::recompute(d);
        assert!(!r.bodies.is_empty(), "o documento tem que gerar corpo");
        r.bodies.iter().map(|b| (b.mesh.positions.clone(), b.mesh.indices.clone())).collect()
    }

    fn refs(m: &[(Vec<f32>, Vec<u32>)]) -> Vec<MalhaRef<'_>> {
        m.iter().map(|(p, i)| MalhaRef { positions: p, indices: i }).collect()
    }

    fn caixa_dos(v: &[f32]) -> ([f32; 3], [f32; 3]) {
        let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
        for p in v.chunks_exact(3) {
            for k in 0..3 {
                lo[k] = lo[k].min(p[k]);
                hi[k] = hi[k].max(p[k]);
            }
        }
        (lo, hi)
    }

    #[test]
    fn versoes_iguais_nao_tem_diferenca() {
        let m = malhas(&caixa("40"));
        let s = sobrepor(&refs(&m), &refs(&m), None);
        assert!(s.e_igual(), "{} removidos, {} adicionados", s.removido.len() / 9, s.adicionado.len() / 9);
        assert_eq!(s.triangulos_comuns, 12, "a caixa tem 12 triângulos");
    }

    /// A caixa é centrada em X, então 40 → 60 cresce 10 mm em cada ponta (seção 20 x 5): as duas tampas
    /// antigas ficam por dentro e somem (2 x 100 mm²); entram as duas tampas novas (2 x 100) e o
    /// prolongamento das quatro laterais em cada ponta (2 x 10 x (2 x 20 + 2 x 5) = 1000 mm²).
    #[test]
    fn caixa_mais_comprida_remove_as_tampas_velhas_e_acrescenta_as_pontas() {
        let (antes, depois) = (malhas(&caixa("40")), malhas(&caixa("60")));
        let s = sobrepor(&refs(&antes), &refs(&depois), None);
        let perto = |v: f64, alvo: f64| (v - alvo).abs() / alvo < 0.3;
        assert!(perto(s.area_removida, 200.), "removido {} mm²", s.area_removida);
        assert!(perto(s.area_adicionada, 1200.), "adicionado {} mm²", s.area_adicionada);
        let (lo, hi) = caixa_dos(&s.adicionado);
        let (blo, bhi) = caixa_dos(&depois[0].0);
        for k in 0..3 {
            assert!(lo[k] >= blo[k] - 1e-3 && hi[k] <= bhi[k] + 1e-3, "o acrescentado fica dentro da caixa nova (eixo {k})");
        }
        // as pontas novas chegam até as tampas novas, nas duas pontas
        assert!(lo[0] <= blo[0] + 1e-3 && hi[0] >= bhi[0] - 1e-3, "de uma tampa nova à outra");
        // o removido são as tampas antigas: ficam dentro da caixa antiga, em planos de X constante
        let (rlo, rhi) = caixa_dos(&s.removido);
        let (alo, ahi) = caixa_dos(&antes[0].0);
        assert!((rlo[0] - alo[0]).abs() < 1e-3 && (rhi[0] - ahi[0]).abs() < 1e-3);
    }

    #[test]
    fn caixa_mais_curta_e_o_inverso() {
        let (antes, depois) = (malhas(&caixa("60")), malhas(&caixa("40")));
        let s = sobrepor(&refs(&antes), &refs(&depois), None);
        let perto = |v: f64, alvo: f64| (v - alvo).abs() / alvo < 0.3;
        assert!(perto(s.area_removida, 1200.), "removido {} mm²", s.area_removida);
        assert!(perto(s.area_adicionada, 200.), "adicionado {} mm²", s.area_adicionada);
        let (lo, hi) = caixa_dos(&s.removido);
        let (alo, ahi) = caixa_dos(&antes[0].0);
        for k in 0..3 {
            assert!(lo[k] >= alo[k] - 1e-3 && hi[k] <= ahi[k] + 1e-3, "o removido fica dentro da caixa antiga (eixo {k})");
        }
    }

    #[test]
    fn mesma_caixa_deslocada_aparece_nos_dois_lados() {
        let m = malhas(&caixa("40"));
        let deslocada: Vec<(Vec<f32>, Vec<u32>)> = m.iter().map(|(p, i)| (p.chunks_exact(3).flat_map(|v| [v[0] + 100., v[1], v[2]]).collect(), i.clone())).collect();
        let s = sobrepor(&refs(&m), &refs(&deslocada), None);
        assert_eq!(s.removido.len() / 9, 12);
        assert_eq!(s.adicionado.len() / 9, 12);
        assert_eq!(s.triangulos_comuns, 0);
    }

    #[test]
    fn mudanca_menor_que_a_tolerancia_nao_conta() {
        let m = malhas(&caixa("40"));
        let quase: Vec<(Vec<f32>, Vec<u32>)> = m.iter().map(|(p, i)| (p.iter().map(|x| x + 0.001).collect(), i.clone())).collect();
        assert!(sobrepor(&refs(&m), &refs(&quase), None).e_igual());
        // com tolerância apertada, a mesma diferença aparece
        assert!(!sobrepor(&refs(&m), &refs(&quase), Some(0.0001)).e_igual());
    }

    #[test]
    fn sem_malha_nao_quebra() {
        let vazio = sobrepor(&[], &[], None);
        assert!(vazio.e_igual() && vazio.triangulos_comuns == 0);
        let m = malhas(&caixa("40"));
        let s = sobrepor(&[], &refs(&m), None);
        assert_eq!(s.adicionado.len() / 9, 12);
    }
}
