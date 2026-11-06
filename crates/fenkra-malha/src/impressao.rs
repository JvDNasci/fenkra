// SPDX-License-Identifier: LGPL-2.1-or-later
//! Conta do preparo de impressão (B119): conferência da malha, assentar numa face, balanço e mesa.
//!
//! O balanço é o ângulo que a face faz com a horizontal olhando para baixo. Uma camada fundida só se
//! apoia na de baixo: a face que olha para baixo mais deitada que o limite (45 graus é a regra comum)
//! fica sem apoio. Quem encosta na mesa não conta, porque a mesa é o apoio.

use crate::{dot, mul, norma, normal_area, sub, unit, Bvh, Diagnostico, Malha, V3};

/// Margem para considerar que uma face encosta na mesa, em mm.
const FOLGA_MESA: f64 = 0.01;

/// Um item da conferência antes de imprimir.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Achado {
    /// a malha não fecha: o fatiador inventa ou perde parede
    NaoEstanque,
    NaoManifold,
    /// normais que discordam ou casca do avesso
    Normais,
    /// mais de uma parte: pode ser de propósito, e cada uma imprime solta
    PartesSoltas,
    /// triângulos que se cruzam
    Cruzamentos,
    /// a peça não cabe no volume da impressora
    ForaDaMesa,
}

impl Achado {
    /// Texto para a interface, com a tradução em `textos_malha`.
    pub fn texto(self) -> &'static str {
        match self {
            Achado::NaoEstanque => "A malha não fecha: há furos ou arestas livres.",
            Achado::NaoManifold => "Há arestas ou vértices não manifold.",
            Achado::Normais => "Há faces viradas ou casca do avesso.",
            Achado::PartesSoltas => "A peça tem partes soltas.",
            Achado::Cruzamentos => "Há triângulos que se cruzam.",
            Achado::ForaDaMesa => "A peça não cabe na mesa.",
        }
    }

    /// Impede imprimir, ou só merece atenção?
    pub fn grave(self) -> bool {
        !matches!(self, Achado::PartesSoltas | Achado::Cruzamentos)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Conferencia {
    pub diagnostico: Diagnostico,
    pub achados: Vec<Achado>,
}

impl Conferencia {
    pub fn pronta(&self) -> bool {
        !self.achados.iter().any(|a| a.grave())
    }
}

/// Confere a malha para impressão. `mesa` é o volume da impressora em mm.
pub fn conferir(m: &Malha, mesa: Option<V3>) -> Conferencia {
    let solda = m.soldada();
    let d = solda.diagnostico_completo();
    let mut achados = Vec::new();
    if d.arestas_livres > 0 {
        achados.push(Achado::NaoEstanque);
    }
    if d.arestas_nao_manifold + d.vertices_nao_manifold > 0 {
        achados.push(Achado::NaoManifold);
    }
    if d.arestas_invertidas > 0 || d.virada_para_dentro() {
        achados.push(Achado::Normais);
    }
    if d.componentes > 1 {
        achados.push(Achado::PartesSoltas);
    }
    if d.autointersecoes.unwrap_or(0) > 0 {
        achados.push(Achado::Cruzamentos);
    }
    if let (Some(v), Some((a, b))) = (mesa, m.caixa()) {
        if (0..3).any(|i| b[i] - a[i] > v[i] + 1e-9) {
            achados.push(Achado::ForaDaMesa);
        }
    }
    Conferencia { diagnostico: d, achados }
}

/// Giro que leva `normal` para baixo (-Z), como matriz 3x4 sem deslocamento. Rodrigues pelo eixo
/// comum; normal já para baixo não gira, e para cima gira meia volta em X.
pub fn giro_para_baixo(normal: V3) -> [f64; 12] {
    let n = unit(normal);
    let alvo = [0., 0., -1.];
    let c = dot(n, alvo);
    if c > 1. - 1e-12 {
        return crate::IDENTIDADE;
    }
    if c < -1. + 1e-12 {
        return [1., 0., 0., 0., 0., -1., 0., 0., 0., 0., -1., 0.];
    }
    let v = crate::cross(n, alvo);
    let k = 1. / (1. + c);
    [
        1. - k * (v[1] * v[1] + v[2] * v[2]),
        k * v[0] * v[1] - v[2],
        k * v[0] * v[2] + v[1],
        0.,
        k * v[0] * v[1] + v[2],
        1. - k * (v[0] * v[0] + v[2] * v[2]),
        k * v[1] * v[2] - v[0],
        0.,
        k * v[0] * v[2] - v[1],
        k * v[1] * v[2] + v[0],
        1. - k * (v[0] * v[0] + v[1] * v[1]),
        0.,
    ]
}

/// Matriz que deita a peça na face de normal `normal` e encosta o ponto mais baixo em z = 0.
pub fn assentar(m: &Malha, normal: V3) -> [f64; 12] {
    let mut r = giro_para_baixo(normal);
    let baixo = m.pos.iter().map(|p| crate::cena::aplicar(&r, *p)[2]).fold(f64::INFINITY, f64::min);
    if baixo.is_finite() {
        r[11] = -baixo;
    }
    r
}

/// Normal do triângulo mais perto de um ponto clicado: é a face em que o corpo de malha assenta.
pub fn normal_no_ponto(m: &Malha, p: V3) -> Option<V3> {
    let r = Bvh::nova(m).mais_proximo(p)?;
    let [a, b, c] = m.tris[r.triangulo as usize].map(|i| m.pos[i as usize]);
    Some(unit(normal_area(a, b, c)))
}

/// Maior face plana, pela soma das áreas por normal: o palpite de qual lado vai para a mesa.
pub fn melhor_base(m: &Malha) -> Option<V3> {
    let mut grupos: std::collections::HashMap<[i32; 3], (f64, V3)> = std::collections::HashMap::new();
    for t in &m.tris {
        let [a, b, c] = t.map(|i| m.pos[i as usize]);
        let n = normal_area(a, b, c);
        let area = norma(n) / 2.;
        if area <= 0. {
            continue;
        }
        let u = unit(n);
        let g = grupos.entry(u.map(|x| (x * 50.).round() as i32)).or_insert((0., [0.; 3]));
        g.0 += area;
        g.1 = crate::add(g.1, mul(u, area));
    }
    let mut lista: Vec<_> = grupos.into_iter().collect();
    lista.sort_by_key(|(k, _)| *k);
    lista.into_iter().max_by(|a, b| a.1 .0.total_cmp(&b.1 .0)).map(|(_, (_, n))| unit(n))
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Balanco {
    /// triângulos sem apoio
    pub triangulos: Vec<u32>,
    pub area: f64,
    /// fração da área total
    pub fracao: f64,
}

/// Faces sem apoio. A malha já deve estar na posição de impressão, com z = 0 na mesa. `limite` é o
/// maior ângulo com a vertical que a impressora vence sem suporte, em graus.
pub fn balanco(m: &Malha, limite: f64) -> Balanco {
    let piso = m.caixa().map(|(a, _)| a[2]).unwrap_or(0.);
    let corte = -limite.clamp(0., 90.).to_radians().sin();
    let mut b = Balanco::default();
    let mut total = 0.;
    for (i, t) in m.tris.iter().enumerate() {
        let [p, q, r] = t.map(|v| m.pos[v as usize]);
        let n = normal_area(p, q, r);
        let area = norma(n) / 2.;
        total += area;
        if area <= 0. {
            continue;
        }
        let encosta = [p, q, r].iter().all(|v| v[2] <= piso + FOLGA_MESA);
        // normal unitária apontando mais para baixo que o seno do limite
        if !encosta && unit(n)[2] < corte {
            b.triangulos.push(i as u32);
            b.area += area;
        }
    }
    b.fracao = if total > 0. { b.area / total } else { 0. };
    b
}

/// Lugar de cada caixa na mesa, em fileiras a partir do canto, com `espaco` entre as peças. Devolve o
/// canto de cada uma, e `None` em quem não coube. As maiores entram primeiro.
pub fn arrumar(caixas: &[[f64; 2]], mesa: [f64; 2], espaco: f64) -> Vec<Option<[f64; 2]>> {
    let mut ordem: Vec<usize> = (0..caixas.len()).collect();
    ordem.sort_by(|a, b| caixas[*b][1].total_cmp(&caixas[*a][1]).then(a.cmp(b)));
    let mut saida = vec![None; caixas.len()];
    let (mut x, mut y, mut fileira) = (0f64, 0f64, 0f64);
    for i in ordem {
        let [l, a] = caixas[i];
        if x > 0. && x + l > mesa[0] {
            x = 0.;
            y += fileira + espaco;
            fileira = 0.;
        }
        if x + l <= mesa[0] && y + a <= mesa[1] {
            saida[i] = Some([x, y]);
            x += l + espaco;
            fileira = fileira.max(a);
        }
    }
    saida
}

/// Desvio da peça até o canto da mesa: traz o canto da caixa em `(0, 0)` para o `lugar`.
pub fn deslocar(m: &[f64; 12], malha: &Malha, lugar: [f64; 2]) -> [f64; 12] {
    let mut r = *m;
    let (lo, _) = malha.pos.iter().map(|p| crate::cena::aplicar(m, *p)).fold(([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]), |(a, b), q| ([0, 1, 2].map(|i| a[i].min(q[i])), [0, 1, 2].map(|i| b[i].max(q[i]))));
    r[3] += lugar[0] - lo[0];
    r[7] += lugar[1] - lo[1];
    r
}

/// Tamanho da caixa depois da matriz.
pub fn medida_depois(m: &[f64; 12], malha: &Malha) -> V3 {
    let (lo, hi) = malha.pos.iter().map(|p| crate::cena::aplicar(m, *p)).fold(([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]), |(a, b), q| ([0, 1, 2].map(|i| a[i].min(q[i])), [0, 1, 2].map(|i| b[i].max(q[i]))));
    sub(hi, lo)
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::teste::*;

    fn posta(m: &Malha, r: &[f64; 12]) -> Malha {
        m.transformada(r)
    }

    #[test]
    fn caixa_sa_passa_e_cada_defeito_aparece() {
        let c = caixa([10., 10., 10.]);
        let r = conferir(&c, Some([20., 20., 20.]));
        assert!(r.pronta() && r.achados.is_empty(), "{r:?}");

        let mut aberta = caixa([10., 10., 10.]);
        aberta.tris.pop();
        let r = conferir(&aberta, None);
        assert!(r.achados.contains(&Achado::NaoEstanque) && !r.pronta());

        let mut avesso = caixa([10., 10., 10.]);
        avesso.tris.iter_mut().for_each(|t| t.swap(1, 2));
        assert!(conferir(&avesso, None).achados.contains(&Achado::Normais));

        let mut duas = caixa([10., 10., 10.]);
        let mut b = caixa([1., 1., 1.]);
        b.pos.iter_mut().for_each(|p| p[0] += 20.);
        duas = crate::juntar([&duas, &b]);
        let r = conferir(&duas, None);
        assert!(r.achados.contains(&Achado::PartesSoltas) && r.pronta(), "parte solta só avisa: {r:?}");

        let r = conferir(&c, Some([5., 20., 20.]));
        assert!(r.achados.contains(&Achado::ForaDaMesa) && !r.pronta());
    }

    #[test]
    fn giro_leva_qualquer_normal_para_baixo_sem_escala() {
        for n in [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.], [0., 0., -1.], [1., 2., 3.], [-3., 0.5, -2.]] {
            let r = giro_para_baixo(n);
            let v = crate::cena::aplicar(&r, unit(n));
            assert!(norma(sub(v, [0., 0., -1.])) < 1e-9, "{n:?}: {v:?}");
            assert!(crate::cena::e_rigida(&r), "{n:?}");
        }
    }

    #[test]
    fn assentar_encosta_a_face_em_z_zero() {
        let c = caixa([10., 20., 30.]);
        // deita sobre a face +X: a altura vira o lado de 10
        let r = assentar(&c, [1., 0., 0.]);
        let p = posta(&c, &r);
        let (a, b) = p.caixa().unwrap();
        assert!(a[2].abs() < 1e-9 && (b[2] - 10.).abs() < 1e-9, "{a:?} {b:?}");
        assert!((p.volume() - 6000.).abs() < 1e-6);
    }

    #[test]
    fn normal_no_ponto_e_maior_base() {
        let c = caixa([10., 20., 30.]);
        let n = normal_no_ponto(&c, [5., 10., 31.]).unwrap();
        assert!(norma(sub(n, [0., 0., 1.])) < 1e-9);
        // a maior face da caixa é a de 20 x 30, e as duas empatam: fica uma delas
        let b = melhor_base(&c).unwrap();
        assert!(b[0].abs() > 0.99, "{b:?}");
    }

    #[test]
    fn balanco_acha_a_marquise_e_ignora_a_mesa() {
        // T: pilar 10x10x20 e laje 30x10x5 por cima
        let mut pilar = caixa([10., 10., 20.]);
        pilar.pos.iter_mut().for_each(|p| p[0] += 10.);
        let mut laje = caixa([30., 10., 5.]);
        laje.pos.iter_mut().for_each(|p| p[2] += 20.);
        let t = crate::booleana::booleana(&pilar, &laje, crate::booleana::OpMalha::Unir).unwrap();
        let b = balanco(&t, 45.);
        // a parte de baixo da laje que sobra para os dois lados: 2 x 10 x 10
        assert!((b.area - 200.).abs() < 1e-6, "{}", b.area);
        assert!(b.fracao > 0. && b.fracao < 1.);

        let simples = balanco(&caixa([10., 10., 10.]), 45.);
        assert!(simples.triangulos.is_empty(), "o fundo encosta na mesa");
    }

    #[test]
    fn limite_decide_o_chanfro() {
        // cunha: a face inclinada olha para baixo a 45 graus exatos da vertical
        let m = Malha { pos: vec![[0., 0., 0.], [10., 0., 10.], [0., 10., 0.], [10., 10., 10.], [0., 0., 10.], [0., 10., 10.]], tris: vec![[0, 3, 1], [0, 2, 3], [4, 3, 5], [4, 1, 3], [0, 5, 2], [0, 4, 5], [0, 1, 4], [2, 5, 3]], ..Default::default() };
        assert!(m.diagnostico().estanque() && m.volume() > 0., "{:?}", m.diagnostico());
        assert!(balanco(&m, 40.).area > 100., "mais deitada que 40 graus");
        assert_eq!(balanco(&m, 50.).area, 0., "dentro dos 50 graus");
    }

    #[test]
    fn arrumar_nao_sobrepoe_e_avisa_quem_nao_cabe() {
        let caixas = [[50., 50.], [40., 30.], [40., 30.], [200., 10.]];
        let l = arrumar(&caixas, [100., 100.], 5.);
        assert!(l[3].is_none(), "maior que a mesa");
        let postos: Vec<([f64; 2], [f64; 2])> = l.iter().zip(&caixas).filter_map(|(p, c)| p.map(|p| (p, *c))).collect();
        assert_eq!(postos.len(), 3);
        for (i, (p, c)) in postos.iter().enumerate() {
            assert!(p[0] + c[0] <= 100. && p[1] + c[1] <= 100.);
            for (q, d) in &postos[i + 1..] {
                let separadas = p[0] + c[0] <= q[0] || q[0] + d[0] <= p[0] || p[1] + c[1] <= q[1] || q[1] + d[1] <= p[1];
                assert!(separadas, "{p:?} e {q:?}");
            }
        }
    }

    #[test]
    fn deslocar_poe_o_canto_no_lugar() {
        let mut c = caixa([10., 10., 10.]);
        c.pos.iter_mut().for_each(|p| p[0] -= 33.);
        let r = deslocar(&crate::IDENTIDADE, &c, [7., 9.]);
        let (a, _) = posta(&c, &r).caixa().unwrap();
        assert!((a[0] - 7.).abs() < 1e-12 && (a[1] - 9.).abs() < 1e-12);
    }
}
