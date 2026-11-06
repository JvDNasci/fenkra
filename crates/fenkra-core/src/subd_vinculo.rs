// SPDX-License-Identifier: LGPL-2.1-or-later
//! Vínculos da gaiola com a peça (B303): o que cada um guarda e a matemática que não precisa de kernel.
//!
//! A gaiola guardada continua sendo o dado da pessoa. Quem resolve os vínculos é o cálculo, que produz
//! uma gaiola resolvida a cada recálculo; assim mudar um parâmetro move os nós sem tocar no arquivo, e
//! vínculo perdido vira erro sem mexer na forma.
//!
//! **Borda casada.** Na borda aberta a superfície de Catmull-Clark é, ao longo da borda, a B-spline
//! cúbica uniforme dos nós da borda, e através dela equivale a uma fileira fantasma `2·P0 − P1`. Daí
//! saem as três continuidades:
//! - **G0:** o limite no nó vale `(P[i-1] + 4·P[i] + P[i+1]) / 6`. Casar é resolver esse sistema
//!   tridiagonal para o limite passar pelos pontos da aresta, e não só pôr os nós em cima dela.
//! - **G1:** a derivada através da borda é `P1 − P0`. Basta a fileira vizinha ficar no plano tangente.
//! - **G2:** a derivada segunda através da borda é `P_-1 − 2·P0 + P1 = 0`. A forma tem curvatura normal
//!   nula através de uma borda livre, seja qual for a gaiola, e casa em G2 só com face que também não
//!   curva nessa direção. Onde a face curva, o relatório diz que ficou em G1, em vez de fingir.

use crate::doc::{CurveRef, Id, PlaneRef, PointRef, SurfRef};
use crate::subd::{add, dot, mul, norma, par, sub, unit, Gaiola, V3};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// A que o nó está preso, com a posição dele na referência.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum AlvoNo {
    Ponto(PointRef),
    /// `t` é a fração do comprimento da curva: a curva pode crescer e o nó fica na mesma fração. Sem
    /// `t`, vale o ponto da curva mais perto do nó, até a interface ancorar
    Curva { curva: CurveRef, t: Option<f64> },
    /// `uv` são coordenadas no plano: o plano leva o nó junto. Sem elas, vale a projeção do nó
    Plano { plano: PlaneRef, uv: Option<[f64; 2]> },
    /// a face não tem coordenada estável entre recálculos: vale o ponto da face mais perto do nó guardado
    Face { face: SurfRef },
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct VinculoNo {
    pub no: u32,
    pub alvo: AlvoNo,
}

/// Distância do nó a um plano, medida na normal, como expressão: é a cota ligada a parâmetro.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct CotaNo {
    pub no: u32,
    pub plano: PlaneRef,
    pub expr: String,
}

/// Caminho de arestas da forma casado com uma aresta do modelo.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct VinculoBorda {
    /// nós do caminho, em ordem
    pub nos: Vec<u32>,
    pub aresta: CurveRef,
    /// face vizinha: G1 e G2 precisam dela
    #[serde(default)]
    pub face: Option<SurfRef>,
    /// 0 G0, 1 G1, 2 G2
    pub continuidade: u8,
}

#[derive(Clone, PartialEq, Debug, Default, Serialize, Deserialize)]
pub struct Vinculos {
    #[serde(default)]
    pub nos: Vec<VinculoNo>,
    #[serde(default)]
    pub cotas: Vec<CotaNo>,
    #[serde(default)]
    pub bordas: Vec<VinculoBorda>,
}

impl Vinculos {
    pub fn vazio(&self) -> bool {
        self.nos.is_empty() && self.cotas.is_empty() && self.bordas.is_empty()
    }

    pub fn walk(&mut self, ids: &mut dyn FnMut(&mut Id), exprs: &mut dyn FnMut(&mut String)) {
        for v in &mut self.nos {
            match &mut v.alvo {
                AlvoNo::Ponto(p) => p.walk(ids),
                AlvoNo::Curva { curva, .. } => curva.walk(ids),
                AlvoNo::Plano { plano, .. } => plano.walk(ids),
                AlvoNo::Face { face } => face.walk(ids),
            }
        }
        for c in &mut self.cotas {
            c.plano.walk(ids);
            exprs(&mut c.expr);
        }
        for b in &mut self.bordas {
            b.aresta.walk(ids);
            if let Some(f) = &mut b.face {
                f.walk(ids);
            }
        }
    }

    /// Um nó tem no máximo uma regra: a nova tira a antiga, seja vínculo ou cota.
    pub fn definir_no(&mut self, v: VinculoNo) {
        self.soltar_no(v.no);
        self.nos.push(v);
    }

    pub fn definir_cota(&mut self, c: CotaNo) {
        self.soltar_no(c.no);
        self.cotas.push(c);
    }

    pub fn soltar_no(&mut self, no: u32) {
        self.nos.retain(|v| v.no != no);
        self.cotas.retain(|c| c.no != no);
    }

    /// Nó que algum vínculo prende, para o desenho marcar com a corrente.
    pub fn prende(&self, no: u32) -> bool {
        self.nos.iter().any(|v| v.no == no) || self.cotas.iter().any(|c| c.no == no) || self.bordas.iter().any(|b| b.nos.contains(&no))
    }
}

/// Quadro de um plano resolvido.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct QuadroV {
    pub o: V3,
    pub x: V3,
    pub y: V3,
    pub n: V3,
}

impl QuadroV {
    pub fn ponto(&self, u: f64, v: f64) -> V3 {
        add(self.o, add(mul(self.x, u), mul(self.y, v)))
    }

    pub fn coordenadas(&self, p: V3) -> (f64, f64) {
        let d = sub(p, self.o);
        (dot(d, self.x), dot(d, self.y))
    }

    pub fn distancia(&self, p: V3) -> f64 {
        dot(sub(p, self.o), self.n)
    }
}

/// Referência já resolvida, para o desenho e para o arraste deslizar nela sem o kernel.
#[derive(Clone, PartialEq, Debug)]
pub enum GeoNo {
    Ponto(V3),
    Curva(Vec<V3>),
    Plano(QuadroV),
    /// o deslize volta do cálculo: fica o ponto da face onde o nó estava
    Face(V3),
}

fn comprimento(pts: &[V3]) -> f64 {
    pts.windows(2).map(|w| norma(sub(w[1], w[0]))).sum()
}

/// Ponto na fração `t` do comprimento da polilinha.
pub fn ponto_na_curva(pts: &[V3], t: f64) -> V3 {
    let mut falta = comprimento(pts) * t.clamp(0., 1.);
    for w in pts.windows(2) {
        let l = norma(sub(w[1], w[0]));
        if falta <= l && l > 0. {
            return add(w[0], mul(sub(w[1], w[0]), falta / l));
        }
        falta -= l;
    }
    pts.last().copied().unwrap_or([0.; 3])
}

/// Fração da polilinha no ponto mais perto de `p`.
pub fn fracao_na_curva(pts: &[V3], p: V3) -> f64 {
    let (total, mut andado) = (comprimento(pts), 0.);
    let mut melhor = (f64::MAX, 0.);
    for w in pts.windows(2) {
        let d = sub(w[1], w[0]);
        let l2 = dot(d, d);
        let s = if l2 > 0. { (dot(sub(p, w[0]), d) / l2).clamp(0., 1.) } else { 0. };
        let e = norma(sub(p, add(w[0], mul(d, s))));
        if e < melhor.0 {
            melhor = (e, andado + s * l2.sqrt());
        }
        andado += l2.sqrt();
    }
    if total > 0. {
        melhor.1 / total
    } else {
        0.
    }
}

/// Distância do ponto à polilinha.
fn distancia_a_curva(pts: &[V3], p: V3) -> f64 {
    norma(sub(p, ponto_na_curva(pts, fracao_na_curva(pts, p))))
}

/// Onde o nó fica se o arraste o largou em `p`: o deslize restrito à referência.
pub fn restringir(geo: &GeoNo, p: V3) -> V3 {
    match geo {
        GeoNo::Ponto(q) => *q,
        // a projeção precisa do kernel: o cálculo a faz a partir daqui
        GeoNo::Face(_) => p,
        GeoNo::Curva(c) => ponto_na_curva(c, fracao_na_curva(c, p)),
        GeoNo::Plano(f) => {
            let (u, v) = f.coordenadas(p);
            f.ponto(u, v)
        }
    }
}

impl AlvoNo {
    /// Falta a coordenada na referência?
    pub fn sem_ancora(&self) -> bool {
        matches!(self, AlvoNo::Curva { t: None, .. } | AlvoNo::Plano { uv: None, .. })
    }

    /// Guarda em `self` a posição `p`, para o nó ficar ali na próxima conta.
    pub fn ancorar(&mut self, geo: &GeoNo, p: V3) {
        match (self, geo) {
            (AlvoNo::Curva { t, .. }, GeoNo::Curva(c)) => *t = Some(fracao_na_curva(c, p)),
            (AlvoNo::Plano { uv, .. }, GeoNo::Plano(f)) => {
                let (u, v) = f.coordenadas(p);
                *uv = Some([u, v]);
            }
            _ => {}
        }
    }
}

/// Normal da face e curvatura através da aresta, em `(ponto, tangente da aresta)`.
pub type Apoio<'a> = &'a dyn Fn(V3, V3) -> Result<(V3, f64), String>;

/// O que uma borda casada conseguiu.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct RelatorioBorda {
    pub pedida: u8,
    pub obtida: u8,
    /// maior distância entre a borda lisa e a aresta
    pub desvio: f64,
    /// maior ângulo entre a superfície e a face na borda, em graus
    pub angulo: f64,
    pub motivo: Option<String>,
}

/// Move os nós do caminho, e a fileira vizinha em G1 e G2, para a superfície casar com a aresta.
pub fn casar_borda(g: &mut Gaiola, nos: &[u32], curva: &[V3], continuidade: u8, apoio: Option<Apoio>) -> Result<RelatorioBorda, String> {
    if nos.len() < 2 || curva.len() < 2 {
        return Err("a borda casada precisa de uma aresta da forma e de uma aresta do modelo".into());
    }
    let arestas: HashSet<(u32, u32)> = g.arestas().into_iter().collect();
    if let Some(w) = nos.windows(2).find(|w| !arestas.contains(&par(w[0], w[1]))) {
        return Err(format!("os nós {} e {} não formam mais uma aresta", w[0], w[1]));
    }
    let n = nos.len();
    let fechada = n > 2 && arestas.contains(&par(nos[0], nos[n - 1])) && norma(sub(curva[0], curva[curva.len() - 1])) < 1e-6;
    let ps: Vec<V3> = nos.iter().map(|id| g.pos(*id).ok_or_else(|| format!("o nó {id} não existe mais"))).collect::<Result<_, _>>()?;
    // os nós mantêm a distribuição que a pessoa deu
    let lados: Vec<f64> = (0..if fechada { n } else { n - 1 }).map(|i| norma(sub(ps[(i + 1) % n], ps[i])).max(1e-9)).collect();
    let total: f64 = lados.iter().sum();
    let mut acumulado = vec![0.; n];
    for i in 1..n {
        acumulado[i] = acumulado[i - 1] + lados[i - 1] / total;
    }
    let mut c = curva.to_vec();
    if fechada {
        // a volta começa perto do primeiro nó
        c.pop();
        let k = (0..c.len()).min_by(|a, b| norma(sub(c[*a], ps[0])).total_cmp(&norma(sub(c[*b], ps[0])))).unwrap_or(0);
        c.rotate_left(k);
        c.push(c[0]);
    }
    let alvos = |c: &[V3]| -> Vec<V3> { acumulado.iter().map(|s| ponto_na_curva(c, *s)).collect() };
    let erro = |a: &[V3]| -> f64 { a.iter().zip(&ps).map(|(x, y)| norma(sub(*x, *y))).sum() };
    let mut alvo = alvos(&c);
    let mut virada = c.clone();
    virada.reverse();
    let outro = alvos(&virada);
    if erro(&outro) < erro(&alvo) {
        alvo = outro;
        c = virada;
    }
    // caminho por dentro da forma: a aresta vira vinco cheio, para valer a regra da borda
    let liga = |i: usize| (nos[i], nos[(i + 1) % n]);
    let abertas: Vec<bool> = (0..lados.len()).map(|i| {
        let (a, b) = liga(i);
        let e = par(a, b);
        g.faces.iter().filter(|f| (0..f.nos.len()).any(|k| par(f.nos[k], f.nos[(k + 1) % f.nos.len()]) == e)).count() < 2
    }).collect();
    if continuidade > 0 && abertas.iter().any(|x| !x) {
        return Err("G1 e G2 só valem em borda aberta da forma; por dentro dela use G0".into());
    }
    for (i, aberta) in abertas.iter().enumerate() {
        if !aberta {
            let (a, b) = liga(i);
            g.definir_vinco(a, b, 1.);
        }
    }
    // pontas do caminho aberto e cantos passam exatamente pelo alvo
    let fixo: Vec<bool> = (0..n).map(|i| (!fechada && (i == 0 || i == n - 1)) || g.no(nos[i]).is_some_and(|x| x.canto)).collect();
    if !fechada {
        for id in [nos[0], nos[n - 1]] {
            if let Some(x) = g.no_mut(id) {
                x.canto = true;
            }
        }
    }
    // Gauss-Seidel: a diagonal domina com razão 1/2, então 80 voltas chegam ao ruído numérico
    let mut p = alvo.clone();
    for _ in 0..80 {
        for i in 0..n {
            if fixo[i] {
                continue;
            }
            let (a, b) = (p[(i + n - 1) % n], p[(i + 1) % n]);
            p[i] = mul(sub(mul(alvo[i], 6.), add(a, b)), 0.25);
        }
    }
    let velhos = ps;
    for (id, q) in nos.iter().zip(&p) {
        if let Some(x) = g.no_mut(*id) {
            x.p = *q;
        }
    }
    let mut rel = RelatorioBorda { pedida: continuidade, ..Default::default() };
    // meio de cada vão da B-spline
    let fantasma = |i: isize| -> V3 {
        if fechada {
            p[i.rem_euclid(n as isize) as usize]
        } else if i < 0 {
            sub(mul(p[0], 2.), p[1])
        } else if i as usize >= n {
            sub(mul(p[n - 1], 2.), p[n - 2])
        } else {
            p[i as usize]
        }
    };
    for i in 0..lados.len() as isize {
        let m = mul(add(add(fantasma(i - 1), fantasma(i + 2)), mul(add(fantasma(i), fantasma(i + 1)), 23.)), 1. / 48.);
        rel.desvio = rel.desvio.max(distancia_a_curva(&c, m));
    }
    if continuidade == 0 {
        return Ok(rel);
    }
    let Some(apoio) = apoio else { return Err("G1 e G2 precisam da face vizinha: escolha a face".into()) };
    let no_caminho: HashSet<u32> = nos.iter().copied().collect();
    let mut normais = vec![];
    let mut curvatura: f64 = 0.;
    let mut filas: Vec<Vec<u32>> = vec![];
    for i in 0..n {
        let (a, b) = if fechada { (alvo[(i + n - 1) % n], alvo[(i + 1) % n]) } else { (alvo[i.saturating_sub(1)], alvo[(i + 1).min(n - 1)]) };
        let t = unit(sub(b, a)).ok_or("a aresta do modelo tem pontos repetidos")?;
        let (nr, k) = apoio(p[i], t)?;
        let nr = unit(nr).ok_or("a face vizinha não tem normal aqui")?;
        curvatura = curvatura.max(k.abs());
        let fila: Vec<u32> = g.vizinhos_de(nos[i]).into_iter().filter(|x| !no_caminho.contains(x)).collect();
        for id in &fila {
            let r = g.pos(*id).ok_or("nó inexistente")?;
            // a fileira acompanha o quanto a borda andou, e aí cai no plano tangente
            let v = sub(add(r, sub(p[i], velhos[i])), p[i]);
            let l = norma(v);
            let plana = sub(v, mul(nr, dot(v, nr)));
            let lp = norma(plana);
            if lp < 1e-6 * l.max(1e-9) {
                return Err("a forma sai perpendicular à face: G1 pede uma face que continue a forma".into());
            }
            if let Some(x) = g.no_mut(*id) {
                x.p = add(p[i], mul(plana, l / lp));
            }
        }
        normais.push(nr);
        filas.push(fila);
    }
    // ângulo da derivada através da borda, nos nós comuns
    for i in 0..n {
        let viz = [(i + n - 1) % n, i, (i + 1) % n];
        if !fechada && (i == 0 || i == n - 1) {
            continue;
        }
        if viz.iter().any(|k| filas[*k].len() != 1) {
            continue;
        }
        let linha = |f: &dyn Fn(usize) -> V3| mul(add(add(f(viz[0]), f(viz[2])), mul(f(viz[1]), 4.)), 1. / 6.);
        let r = linha(&|k| g.pos(filas[k][0]).unwrap_or(p[k]));
        let d = sub(r, linha(&|k| p[k]));
        if let Some(u) = unit(d) {
            rel.angulo = rel.angulo.max(dot(u, normais[i]).abs().min(1.).asin().to_degrees());
        }
    }
    rel.obtida = 1;
    if continuidade >= 2 {
        // curvatura pequena perto do tamanho da borda é zero para quem olha
        if curvatura * comprimento(&c).max(1e-9) < 1e-3 {
            rel.obtida = 2;
        } else {
            rel.motivo = Some(format!("a face curva através da aresta (raio {:.1} mm) e a borda livre da forma não curva: ficou em G1", 1. / curvatura));
        }
    }
    Ok(rel)
}

impl Gaiola {
    /// Fusão por distância: os nós da beira a menos de `dist` do plano de simetria vão para ele e soldam
    /// com o reflexo. Nó de beira só, sem o vizinho, deixaria duas folhas num nó: tudo volta e dá erro.
    pub fn fundir_no_plano(&mut self, dist: f64) -> Result<usize, String> {
        let s = self.simetria.ok_or("ligue a simetria antes de fundir")?;
        let (t, _) = self.topologia()?;
        let mut beira: HashSet<usize> = HashSet::new();
        for h in 0..t.prox.len() as u32 {
            if t.borda(h) {
                beira.insert(t.origem[h as usize] as usize);
                beira.insert(t.destino(h) as usize);
            }
        }
        let antes = self.clone();
        let mut n = 0;
        for (i, no) in self.nos.iter_mut().enumerate() {
            let d = s.distancia(no.p).abs();
            if d > crate::subd::TOL_PLANO && d <= dist && beira.contains(&i) {
                no.p[s.eixo as usize] = s.pos;
                n += 1;
            }
        }
        if let Err(e) = self.inteira().topologia() {
            *self = antes;
            return Err(format!("a fusão deixaria a malha inválida: {e}"));
        }
        Ok(n)
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::subd::Simetria;

    /// Folha de 4 por 4 quadrados de lado 10, com a borda y = 0.
    fn folha() -> Gaiola {
        Gaiola::plano([20., 20., 0.], [40., 40.], [4, 4])
    }

    fn fileira(g: &Gaiola, y: f64) -> Vec<u32> {
        let mut v: Vec<(f64, u32)> = g.nos.iter().filter(|n| (n.p[1] - y).abs() < 1e-9).map(|n| (n.p[0], n.id)).collect();
        v.sort_by(|a, b| a.0.total_cmp(&b.0));
        v.into_iter().map(|x| x.1).collect()
    }

    /// Pontos do limite que caem na borda y = 0 da folha original, pelo número da face.
    fn borda_do_limite(g: &Gaiola, niveis: u32) -> Vec<V3> {
        let m = g.refinada(niveis).unwrap();
        let t = crate::subd::Topo::novo(m.pos.len(), &m.faces).unwrap();
        let lim = m.limite().unwrap();
        let mut out = vec![];
        for h in 0..t.prox.len() as u32 {
            if t.borda(h) {
                out.push(lim[t.origem[h as usize] as usize]);
            }
        }
        out
    }

    #[test]
    fn curva_ida_e_volta_pela_fracao() {
        let c = [[0., 0., 0.], [10., 0., 0.], [10., 10., 0.]];
        assert_eq!(ponto_na_curva(&c, 0.25), [5., 0., 0.]);
        assert_eq!(ponto_na_curva(&c, 0.75), [10., 5., 0.]);
        assert!((fracao_na_curva(&c, [12., 5., 3.]) - 0.75).abs() < 1e-12);
        assert_eq!(restringir(&GeoNo::Curva(c.to_vec()), [5., 7., 0.])[1], 7.);
    }

    #[test]
    fn plano_guarda_coordenadas_e_leva_o_no() {
        let q = QuadroV { o: [0., 0., 5.], x: [1., 0., 0.], y: [0., 1., 0.], n: [0., 0., 1.] };
        let mut a = AlvoNo::Plano { plano: PlaneRef::default(), uv: None };
        assert!(a.sem_ancora());
        a.ancorar(&GeoNo::Plano(q), [3., 4., 99.]);
        assert_eq!(a, AlvoNo::Plano { plano: PlaneRef::default(), uv: Some([3., 4.]) });
        assert_eq!(restringir(&GeoNo::Plano(q), [3., 4., 99.]), [3., 4., 5.]);
    }

    #[test]
    fn regra_nova_do_no_tira_a_antiga() {
        let mut v = Vinculos::default();
        v.definir_cota(CotaNo { no: 1, plano: PlaneRef::default(), expr: "10".into() });
        v.definir_no(VinculoNo { no: 1, alvo: AlvoNo::Ponto(PointRef::Origin) });
        assert!(v.cotas.is_empty());
        assert_eq!(v.nos.len(), 1);
        v.definir_cota(CotaNo { no: 1, plano: PlaneRef::default(), expr: "10".into() });
        assert!(v.nos.is_empty() && v.prende(1));
        v.soltar_no(1);
        assert!(v.vazio());
    }

    #[test]
    fn g0_faz_o_limite_passar_pela_reta() {
        let mut g = folha();
        let nos = fileira(&g, 0.);
        // a aresta do modelo está 3 mm acima e 2 mm para o lado
        let reta = [[-5., 2., 3.], [50., 2., 3.]];
        let r = casar_borda(&mut g, &nos, &reta, 0, None).unwrap();
        assert!(r.desvio < 1e-9, "{r:?}");
        for p in borda_do_limite(&g, 3).into_iter().filter(|p| p[1] < 15. && p[2] > 2.9) {
            assert!((p[1] - 2.).abs() < 1e-6 && (p[2] - 3.).abs() < 1e-6, "{p:?}");
        }
    }

    #[test]
    fn g0_em_arco_passa_pelos_nos_e_informa_o_desvio() {
        let mut g = folha();
        let nos = fileira(&g, 0.);
        let arco: Vec<V3> = (0..=64).map(|i| {
            let a = std::f64::consts::PI * i as f64 / 64.;
            [20. - 20. * a.cos(), -20. * a.sin(), 0.]
        }).collect();
        let r = casar_borda(&mut g, &nos, &arco, 0, None).unwrap();
        // o limite em cada nó cai no arco: o nó fica fora dele, de propósito
        let p: Vec<V3> = nos.iter().map(|n| g.pos(*n).unwrap()).collect();
        for i in 1..4 {
            let l = mul(add(add(p[i - 1], p[i + 1]), mul(p[i], 4.)), 1. / 6.);
            assert!(distancia_a_curva(&arco, l) < 0.02, "{i}: {l:?}");
        }
        // quatro vãos em meia volta: a ponta presa tem curvatura zero, e é no vão da ponta que mais desvia
        assert!(r.desvio < 1. && r.desvio > 0., "{r:?}");
        // com o dobro de nós o desvio cai
        let mut fina = Gaiola::plano([20., 20., 0.], [40., 40.], [8, 4]);
        let nos = fileira(&fina, 0.);
        assert!(casar_borda(&mut fina, &nos, &arco, 0, None).unwrap().desvio < r.desvio / 2.);
    }

    #[test]
    fn caminho_invertido_acha_o_sentido_da_aresta() {
        let mut g = folha();
        let mut nos = fileira(&g, 0.);
        nos.reverse();
        let r = casar_borda(&mut g, &nos, &[[0., 0., 7.], [40., 0., 7.]], 0, None).unwrap();
        assert!(r.desvio < 1e-9);
        let p = g.pos(nos[0]).unwrap();
        assert_eq!((p[0].round(), p[2]), (40., 7.), "cada ponta foi para o lado dela");
    }

    #[test]
    fn g1_poe_a_fileira_vizinha_no_plano_tangente() {
        let mut g = folha();
        // puxa a folha para não nascer já tangente
        for n in &mut g.nos {
            n.p[2] = n.p[1] * 0.3;
        }
        let nos = fileira(&g, 0.);
        let inclinada = unit([0., -1., 1.]).unwrap();
        let apoio = move |_: V3, _: V3| Ok((inclinada, 0.));
        let r = casar_borda(&mut g, &nos, &[[0., 0., 0.], [40., 0., 0.]], 1, Some(&apoio)).unwrap();
        assert_eq!(r.obtida, 1);
        assert!(r.angulo < 1e-6, "{r:?}");
        // medida na superfície: a derivada através da borda fica no plano tangente
        let m = g.refinada(4).unwrap();
        let lim = m.limite().unwrap();
        let t = crate::subd::Topo::novo(m.pos.len(), &m.faces).unwrap();
        let mut pior: f64 = 0.;
        for h in 0..t.prox.len() as u32 {
            let (a, b) = (lim[t.origem[h as usize] as usize], lim[t.destino(h) as usize]);
            if !t.borda(h) || a[1].abs() > 1e-9 || b[1].abs() > 1e-9 || a[0] < 8. || a[0] > 32. {
                continue;
            }
            // nó da fileira de dentro do mesmo quadrado
            let f = &m.faces[t.face[h as usize] as usize];
            let dentro: Vec<V3> = f.iter().map(|v| lim[*v as usize]).filter(|p| p[1].abs() > 1e-9).collect();
            for q in dentro {
                pior = pior.max(dot(unit(sub(q, a)).unwrap(), inclinada).abs());
            }
        }
        assert!(pior < 0.02, "{pior}");
    }

    #[test]
    fn g2_so_vale_onde_a_face_nao_curva_atraves() {
        let plano = |_: V3, _: V3| Ok(([0., 0., 1.], 0.));
        let mut g = folha();
        let nos = fileira(&g, 0.);
        let r = casar_borda(&mut g, &nos, &[[0., 0., 0.], [40., 0., 0.]], 2, Some(&plano)).unwrap();
        assert_eq!((r.obtida, r.motivo.is_none()), (2, true));
        let curva = |_: V3, _: V3| Ok(([0., 0., 1.], 1. / 50.));
        let mut g = folha();
        let r = casar_borda(&mut g, &nos, &[[0., 0., 0.], [40., 0., 0.]], 2, Some(&curva)).unwrap();
        assert_eq!(r.obtida, 1);
        assert!(r.motivo.unwrap().contains("50.0"));
    }

    #[test]
    fn g1_sem_face_ou_por_dentro_recusa() {
        let mut g = folha();
        let nos = fileira(&g, 0.);
        assert!(casar_borda(&mut g, &nos, &[[0.; 3], [40., 0., 0.]], 1, None).is_err());
        let plano = |_: V3, _: V3| Ok(([0., 0., 1.], 0.));
        let meio = fileira(&g, 20.);
        assert!(casar_borda(&mut g, &meio, &[[0., 20., 0.], [40., 20., 0.]], 1, Some(&plano)).is_err());
        // em G0 o caminho por dentro vira vinco
        let r = casar_borda(&mut g, &meio, &[[0., 20., 4.], [40., 20., 4.]], 0, None).unwrap();
        assert!(r.desvio < 1e-9);
        assert_eq!(g.peso(meio[1].min(meio[2]), meio[1].max(meio[2])), 1.);
    }

    #[test]
    fn no_que_sumiu_e_aresta_desfeita_viram_erro() {
        let mut g = folha();
        let nos = fileira(&g, 0.);
        assert!(casar_borda(&mut g, &[nos[0], nos[2]], &[[0.; 3], [1., 0., 0.]], 0, None).is_err());
    }

    #[test]
    fn fusao_leva_a_beira_ao_plano_e_solda_as_metades() {
        // meia folha com a beira a 0,3 mm do plano x = 0
        let mut g = Gaiola::plano([20.3, 20., 0.], [40., 40.], [4, 4]);
        g.simetria = Some(Simetria { eixo: 0, pos: 0. });
        let antes = g.inteira();
        assert!(antes.topologia().is_ok());
        assert_eq!(g.fundir_no_plano(0.1).unwrap(), 0, "longe demais");
        assert_eq!(g.fundir_no_plano(0.5).unwrap(), 5);
        let i = g.inteira();
        assert!(i.nos.len() < antes.nos.len(), "os cinco nós soldaram");
        assert_eq!(i.nos.len(), 2 * 25 - 5);
        assert!(i.topologia().is_ok());
    }

    #[test]
    fn fusao_que_quebraria_a_malha_nao_muda_nada() {
        let mut g = Gaiola::plano([20.3, 20., 0.], [40., 40.], [4, 4]);
        g.simetria = Some(Simetria { eixo: 0, pos: 0. });
        // só um nó da beira chega perto: ele soldaria duas folhas num ponto
        for n in &mut g.nos {
            if n.p[0] < 1. && n.p[1] != 20. {
                n.p[0] = 2.;
            }
        }
        let antes = g.clone();
        assert!(g.fundir_no_plano(0.5).is_err());
        assert_eq!(g, antes);
    }

    #[test]
    fn fusao_sem_simetria_recusa() {
        assert!(folha().fundir_no_plano(1.).is_err());
    }
}
