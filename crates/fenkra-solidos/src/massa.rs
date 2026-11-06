// SPDX-License-Identifier: LGPL-2.1-or-later
//! B92: massa, centro, matriz de inércia e eixos principais por corpo e do conjunto.
//!
//! Como funciona: o kernel integra o volume do sólido com densidade 1 (`GProp`) e devolve a matriz
//! de inércia em torno do centro de massa; aqui multiplicamos pela densidade do material do corpo
//! (tabela em `fenkra_core::materiais`). Para vários corpos, cada matriz é transportada para o
//! centro de massa comum pelo teorema dos eixos paralelos e diagonalizada (Jacobi).
//!
//! Unidades: comprimento em mm, densidade em g/cm³, massa em kg, inércia em kg·mm².

use super::*;
use fenkra_core::materiais;

/// 1 g/cm³ = 1e-6 kg/mm³
const KG_POR_MM3_POR_G_CM3: f64 = 1e-6;

/// Propriedades de massa de um corpo (ou do conjunto).
#[derive(Clone, Debug, PartialEq)]
pub struct MassaCorpo {
    /// coletor do corpo (`0` no total)
    pub coletor: Id,
    pub nome: String,
    pub material: String,
    /// g/cm³ usada na conta (a do material, ou a padrão se ele não está na tabela)
    pub densidade: f64,
    pub volume_mm3: f64,
    pub massa_kg: f64,
    pub centro: [f64; 3],
    /// [xx, yy, zz, xy, xz, yz] em torno do centro de massa, kg·mm²
    pub inercia: [f64; 6],
    /// momentos principais em ordem crescente, kg·mm²
    pub principais: [f64; 3],
    /// eixo principal de cada momento (unitário)
    pub eixos: [[f64; 3]; 3],
}

/// Massa por corpo e total.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RelatorioMassa {
    pub corpos: Vec<MassaCorpo>,
    /// `None` quando não há corpo sólido
    pub total: Option<MassaCorpo>,
}

impl RelatorioMassa {
    pub fn massa_total_kg(&self) -> f64 {
        self.total.as_ref().map_or(0., |t| t.massa_kg)
    }
}

type M3 = [[f64; 3]; 3];

fn sim(m: [f64; 6]) -> M3 {
    [[m[0], m[3], m[4]], [m[3], m[1], m[5]], [m[4], m[5], m[2]]]
}

fn de_sim(m: &M3) -> [f64; 6] {
    [m[0][0], m[1][1], m[2][2], m[0][1], m[0][2], m[1][2]]
}

/// Autovalores (crescentes) e autovetores unitários de uma matriz simétrica 3x3, por Jacobi.
pub fn diagonalizar(m: [f64; 6]) -> ([f64; 3], [[f64; 3]; 3]) {
    let mut a = sim(m);
    let mut v: M3 = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
    for _ in 0..64 {
        let (mut p, mut q, mut mx) = (0, 1, 0.);
        for (i, j) in [(0, 1), (0, 2), (1, 2)] {
            if a[i][j].abs() > mx {
                (p, q, mx) = (i, j, a[i][j].abs());
            }
        }
        let escala = a[0][0].abs().max(a[1][1].abs()).max(a[2][2].abs()).max(1e-300);
        if mx <= 1e-15 * escala {
            break;
        }
        let theta = (a[q][q] - a[p][p]) / (2. * a[p][q]);
        let t = theta.signum() / (theta.abs() + (theta * theta + 1.).sqrt());
        let t = if theta == 0. { 1. } else { t };
        let c = 1. / (t * t + 1.).sqrt();
        let s = t * c;
        for k in 0..3 {
            let (akp, akq) = (a[k][p], a[k][q]);
            a[k][p] = c * akp - s * akq;
            a[k][q] = s * akp + c * akq;
        }
        for k in 0..3 {
            let (apk, aqk) = (a[p][k], a[q][k]);
            a[p][k] = c * apk - s * aqk;
            a[q][k] = s * apk + c * aqk;
        }
        for k in 0..3 {
            let (vkp, vkq) = (v[k][p], v[k][q]);
            v[k][p] = c * vkp - s * vkq;
            v[k][q] = s * vkp + c * vkq;
        }
    }
    let mut idx = [0usize, 1, 2];
    idx.sort_by(|&x, &y| a[x][x].partial_cmp(&a[y][y]).unwrap_or(std::cmp::Ordering::Equal));
    let vals = [a[idx[0]][idx[0]], a[idx[1]][idx[1]], a[idx[2]][idx[2]]];
    let eixos = [0, 1, 2].map(|n| [v[0][idx[n]], v[1][idx[n]], v[2][idx[n]]]);
    (vals, eixos)
}

/// Soma de corpos: centro ponderado pela massa e inércias transportadas (eixos paralelos).
pub fn somar(corpos: &[MassaCorpo]) -> Option<MassaCorpo> {
    let massa: f64 = corpos.iter().map(|c| c.massa_kg).sum();
    if corpos.is_empty() || massa <= 0. {
        return None;
    }
    let mut centro = [0.; 3];
    for c in corpos {
        for i in 0..3 {
            centro[i] += c.centro[i] * c.massa_kg / massa;
        }
    }
    let mut j: M3 = [[0.; 3]; 3];
    for c in corpos {
        let ic = sim(c.inercia);
        let d = [c.centro[0] - centro[0], c.centro[1] - centro[1], c.centro[2] - centro[2]];
        let d2 = d[0] * d[0] + d[1] * d[1] + d[2] * d[2];
        for r in 0..3 {
            for s in 0..3 {
                let delta = if r == s { 1. } else { 0. };
                j[r][s] += ic[r][s] + c.massa_kg * (d2 * delta - d[r] * d[s]);
            }
        }
    }
    let inercia = de_sim(&j);
    let (principais, eixos) = diagonalizar(inercia);
    let volume: f64 = corpos.iter().map(|c| c.volume_mm3).sum();
    Some(MassaCorpo {
        coletor: Id(0),
        nome: "Total".into(),
        material: String::new(),
        densidade: if volume > 0. { massa / (volume * KG_POR_MM3_POR_G_CM3) } else { 0. },
        volume_mm3: volume,
        massa_kg: massa,
        centro,
        inercia,
        principais,
        eixos,
    })
}

/// Massa e inércia de um sólido com a densidade dada (g/cm³).
pub fn massa_do_shape(sh: &Shape, densidade: f64) -> Option<MassaCorpo> {
    let mp = sh.mass();
    if mp.volume <= 0. {
        return None;
    }
    let k = densidade * KG_POR_MM3_POR_G_CM3;
    let (inercia, principais, eixos) = match sh.inertia() {
        Some(i) => (i.matrix.map(|x| x * k), i.principal.map(|x| x * k), i.axes),
        None => ([0.; 6], [0.; 3], [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]]),
    };
    Some(MassaCorpo {
        coletor: Id(0),
        nome: String::new(),
        material: String::new(),
        densidade,
        volume_mm3: mp.volume,
        massa_kg: mp.volume * k,
        centro: mp.center,
        inercia,
        principais,
        eixos,
    })
}

/// Recalcula o documento e mede cada corpo visível com o material dele.
pub fn relatorio_massa(doc: &Document) -> RelatorioMassa {
    let Ok(vars) = doc.eval_params() else {
        return RelatorioMassa::default();
    };
    let mut cx = Ctx {
        doc,
        vars,
        qualidade: Default::default(),
        bodies: HashMap::new(),
        sketches: HashMap::new(),
        status: HashMap::new(),
        tools: HashMap::new(),
        memo: RefCell::new(HashMap::new()),
        notes: Default::default(),
        body_sig: HashMap::new(),
        last_sheet: None,
        sheet_maps: vec![],
        cut_outlines: vec![],
        malhas: HashMap::new(),
        roscas: vec![],
        leve: None,
        relatorios_forma: vec![],
        formas_resolvidas: vec![],
        steps: HashMap::new(),
        miter_next: false,
        promoted: RefCell::new(None),
    };
    for f in doc.active_features().map(|(_, f)| f) {
        let _ = cx.run(f);
    }
    let mut corpos = vec![];
    for c in doc.collectors.iter().filter(|c| c.visible) {
        let Some(b) = cx.body(c.id) else { continue };
        let mat = materiais::material_do_corpo(doc, c.id);
        if let Some(mut m) = massa_do_shape(b, materiais::densidade(&mat)) {
            m.coletor = c.id;
            m.nome = c.name.clone();
            m.material = mat;
            corpos.push(m);
        }
    }
    let total = somar(&corpos);
    RelatorioMassa { corpos, total }
}

/// Massa total do modelo em kg (0 se não há sólido): é o que o carimbo do desenho e a lista de
/// materiais da montagem leem, em vez de um valor fixo de aço.
pub fn massa_do_modelo_kg(doc: &Document) -> f64 {
    relatorio_massa(doc).massa_total_kg()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jacobi_acha_autovalores_de_matriz_diagonal_e_rotacionada() {
        let (v, e) = diagonalizar([3., 1., 2., 0., 0., 0.]);
        assert_eq!(v, [1., 2., 3.]);
        assert!(e[0][1].abs() > 0.999);
        // matriz [[2,1,0],[1,2,0],[0,0,5]]: autovalores 1, 3, 5
        let (v, _) = diagonalizar([2., 2., 5., 1., 0., 0.]);
        assert!((v[0] - 1.).abs() < 1e-9 && (v[1] - 3.).abs() < 1e-9 && (v[2] - 5.).abs() < 1e-9);
    }

    #[test]
    fn eixos_paralelos_de_duas_massas_iguais() {
        // duas massas pontuais de 1 kg em x=±10: Izz = Iyy = 2·100, Ixx = 0
        let pt = |x: f64| MassaCorpo {
            coletor: Id(1),
            nome: "p".into(),
            material: String::new(),
            densidade: 1.,
            volume_mm3: 1e6,
            massa_kg: 1.,
            centro: [x, 0., 0.],
            inercia: [0.; 6],
            principais: [0.; 3],
            eixos: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        };
        let t = somar(&[pt(-10.), pt(10.)]).unwrap();
        assert!((t.massa_kg - 2.).abs() < 1e-12 && t.centro[0].abs() < 1e-12);
        assert!(t.inercia[0].abs() < 1e-9 && (t.inercia[1] - 200.).abs() < 1e-9 && (t.inercia[2] - 200.).abs() < 1e-9);
        assert!(t.principais[0].abs() < 1e-9);
        assert!(t.eixos[0][0].abs() > 0.999, "o menor momento fica no eixo que passa pelas massas");
    }
}
