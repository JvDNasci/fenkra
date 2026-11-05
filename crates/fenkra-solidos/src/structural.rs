// SPDX-License-Identifier: LGPL-2.1-or-later
//! Estrutural: seções de perfis padronizados (com pontos de ancoragem), biblioteca de famílias e
//! a geração dos membros ao longo do esqueleto (linhas/curvas 3D), com cantos em meia-esquadria,
//! aparados ou com recorte de encaixe (folga de solda), placas de extremidade e reforços.

use fenkra_core::SectionKind;
use fenkra_kernel::{Loop2, Seg2};

/// Família de perfis: tipo de seção, nomes das cotas e tamanhos padronizados.
#[derive(Clone, Debug, PartialEq)]
pub struct Family {
    pub name: String,
    /// grupo da cascata de escolha (norma, tipo, tamanho): as de fábrica por tipo de produto, as do
    /// usuário em "Minhas famílias"
    pub norm: String,
    pub kind: SectionKind,
    pub sizes: Vec<(String, Vec<f64>)>,
}

/// Nomes das cotas de cada tipo de seção (na ordem de `dims`).
pub fn dim_names(k: SectionKind) -> &'static [&'static str] {
    match k {
        SectionKind::TuboQuadrado => &["lado", "parede"],
        SectionKind::TuboRetangular => &["largura", "altura", "parede"],
        SectionKind::TuboRedondo => &["diâmetro", "parede"],
        SectionKind::PerfilI => &["altura", "aba", "alma", "mesa"],
        SectionKind::PerfilU => &["altura", "aba", "alma", "mesa"],
        SectionKind::Cantoneira => &["aba a", "aba b", "espessura"],
        SectionKind::BarraChata => &["largura", "espessura"],
        SectionKind::BarraRedonda => &["diâmetro"],
        SectionKind::BarraQuadrada => &["lado"],
        SectionKind::PerfilT => &["altura", "aba", "alma", "mesa"],
    }
}

pub fn kind_label(k: SectionKind) -> &'static str {
    match k {
        SectionKind::TuboQuadrado => "Tubo quadrado",
        SectionKind::TuboRetangular => "Tubo retangular",
        SectionKind::TuboRedondo => "Tubo redondo",
        SectionKind::PerfilI => "Perfil I / W",
        SectionKind::PerfilU => "Perfil U",
        SectionKind::Cantoneira => "Cantoneira L",
        SectionKind::BarraChata => "Barra chata",
        SectionKind::BarraRedonda => "Barra redonda",
        SectionKind::BarraQuadrada => "Barra quadrada",
        SectionKind::PerfilT => "Perfil T",
    }
}

/// Grupo (norma) das famílias de fábrica de cada tipo de seção. É o agrupamento por produto usado na
/// cascata de escolha; as medidas são as comerciais usuais, não uma tabela normativa completa.
pub fn norm_of(k: SectionKind) -> &'static str {
    match k {
        SectionKind::TuboQuadrado | SectionKind::TuboRetangular | SectionKind::TuboRedondo => "Tubos de seção fechada",
        SectionKind::PerfilI | SectionKind::PerfilU | SectionKind::PerfilT => "Perfis laminados",
        SectionKind::Cantoneira => "Cantoneiras",
        SectionKind::BarraChata | SectionKind::BarraRedonda | SectionKind::BarraQuadrada => "Barras",
    }
}

pub const KINDS: [SectionKind; 10] = [
    SectionKind::TuboQuadrado,
    SectionKind::TuboRetangular,
    SectionKind::TuboRedondo,
    SectionKind::PerfilI,
    SectionKind::PerfilU,
    SectionKind::Cantoneira,
    SectionKind::BarraChata,
    SectionKind::BarraRedonda,
    SectionKind::BarraQuadrada,
    SectionKind::PerfilT,
];

/// Biblioteca de fábrica (medidas comerciais usuais, em mm).
pub fn builtin() -> Vec<Family> {
    let f = |name: &str, kind, sizes: &[(&str, &[f64])]| Family { name: name.into(), norm: norm_of(kind).into(), kind, sizes: sizes.iter().map(|(n, d)| (n.to_string(), d.to_vec())).collect() };
    vec![
        f("Tubo quadrado", SectionKind::TuboQuadrado, &[("20x2", &[20., 2.]), ("30x2", &[30., 2.]), ("40x3", &[40., 3.]), ("50x3", &[50., 3.]), ("60x4", &[60., 4.]), ("80x4", &[80., 4.]), ("100x5", &[100., 5.])]),
        f("Tubo retangular", SectionKind::TuboRetangular, &[("40x20x2", &[40., 20., 2.]), ("50x30x3", &[50., 30., 3.]), ("80x40x3", &[80., 40., 3.]), ("100x50x4", &[100., 50., 4.])]),
        f("Tubo redondo", SectionKind::TuboRedondo, &[("21,3x2", &[21.3, 2.]), ("33,7x2,6", &[33.7, 2.6]), ("48,3x3,2", &[48.3, 3.2]), ("60,3x3,6", &[60.3, 3.6]), ("88,9x4", &[88.9, 4.])]),
        f("Perfil W (laminado)", SectionKind::PerfilI, &[("W150x13", &[148., 100., 4.3, 4.9]), ("W200x15", &[200., 100., 4.3, 5.2]), ("W250x17,9", &[251., 101., 4.8, 5.3]), ("W310x21", &[303., 101., 5.1, 5.7])]),
        f("Perfil U (laminado)", SectionKind::PerfilU, &[("U 3\"", &[76.2, 35.8, 4.32, 6.93]), ("U 4\"", &[101.6, 40.2, 4.57, 7.52]), ("U 6\"", &[152.4, 48.8, 5.08, 8.71])]),
        f("Cantoneira abas iguais", SectionKind::Cantoneira, &[("25x25x3", &[25., 25., 3.]), ("38x38x4,8", &[38.1, 38.1, 4.76]), ("50x50x6,3", &[50.8, 50.8, 6.35]), ("76x76x6,3", &[76.2, 76.2, 6.35])]),
        f("Barra chata", SectionKind::BarraChata, &[("25x3", &[25.4, 3.18]), ("38x6", &[38.1, 6.35]), ("50x6", &[50.8, 6.35]), ("76x9,5", &[76.2, 9.53])]),
        f("Barra redonda", SectionKind::BarraRedonda, &[("Ø10", &[10.]), ("Ø12,5", &[12.7]), ("Ø16", &[15.88]), ("Ø20", &[19.05]), ("Ø25", &[25.4])]),
        f("Barra quadrada", SectionKind::BarraQuadrada, &[("12", &[12.7]), ("20", &[19.05]), ("25", &[25.4])]),
        f("Perfil T", SectionKind::PerfilT, &[("T 50", &[50., 50., 6., 6.]), ("T 80", &[80., 80., 8., 8.])]),
    ]
}

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Loop2 {
    vec![Seg2::Line([x0, y0], [x1, y0]), Seg2::Line([x1, y0], [x1, y1]), Seg2::Line([x1, y1], [x0, y1]), Seg2::Line([x0, y1], [x0, y0])]
}

fn poly(p: &[[f64; 2]]) -> Loop2 {
    (0..p.len()).map(|i| Seg2::Line(p[i], p[(i + 1) % p.len()])).collect()
}

/// Laços da seção com a caixa envolvente centrada na origem; devolve também (largura, altura) e
/// o centróide (para a ancoragem).
pub fn section(kind: SectionKind, d: &[f64]) -> Result<(Vec<Loop2>, f64, f64, [f64; 2]), String> {
    let need = dim_names(kind).len();
    if d.len() < need || d.iter().take(need).any(|v| *v <= 0.) {
        return Err(format!("{}: preencha {} cotas positivas", kind_label(kind), need));
    }
    let wall_ok = |t: f64, s: f64| if 2. * t >= s { Err("a parede é grossa demais para o tamanho".to_string()) } else { Ok(()) };
    Ok(match kind {
        SectionKind::TuboQuadrado | SectionKind::TuboRetangular => {
            let (b, h, t) = if kind == SectionKind::TuboQuadrado { (d[0], d[0], d[1]) } else { (d[0], d[1], d[2]) };
            wall_ok(t, b.min(h))?;
            (vec![rect(-b / 2., -h / 2., b / 2., h / 2.), rect(-b / 2. + t, -h / 2. + t, b / 2. - t, h / 2. - t)], b, h, [0., 0.])
        }
        SectionKind::TuboRedondo => {
            wall_ok(d[1], d[0])?;
            (vec![vec![Seg2::Circle([0., 0.], d[0] / 2.)], vec![Seg2::Circle([0., 0.], d[0] / 2. - d[1])]], d[0], d[0], [0., 0.])
        }
        SectionKind::BarraRedonda => (vec![vec![Seg2::Circle([0., 0.], d[0] / 2.)]], d[0], d[0], [0., 0.]),
        SectionKind::BarraQuadrada => (vec![rect(-d[0] / 2., -d[0] / 2., d[0] / 2., d[0] / 2.)], d[0], d[0], [0., 0.]),
        SectionKind::BarraChata => (vec![rect(-d[0] / 2., -d[1] / 2., d[0] / 2., d[1] / 2.)], d[0], d[1], [0., 0.]),
        SectionKind::PerfilI => {
            let (h, b, tw, tf) = (d[0], d[1], d[2], d[3]);
            let (x, y) = (b / 2., h / 2.);
            let p = [[-x, -y], [x, -y], [x, -y + tf], [tw / 2., -y + tf], [tw / 2., y - tf], [x, y - tf], [x, y], [-x, y], [-x, y - tf], [-tw / 2., y - tf], [-tw / 2., -y + tf], [-x, -y + tf]];
            (vec![poly(&p)], b, h, [0., 0.])
        }
        SectionKind::PerfilU => {
            let (h, b, tw, tf) = (d[0], d[1], d[2], d[3]);
            let (x, y) = (b / 2., h / 2.);
            let p = [[-x, -y], [x, -y], [x, -y + tf], [-x + tw, -y + tf], [-x + tw, y - tf], [x, y - tf], [x, y], [-x, y]];
            // centróide em x (alma à esquerda)
            let a_w = tw * h;
            let a_f = 2. * (b - tw) * tf;
            let cx = (a_w * (-x + tw / 2.) + a_f * (-x + tw + (b - tw) / 2.)) / (a_w + a_f);
            (vec![poly(&p)], b, h, [cx, 0.])
        }
        SectionKind::PerfilT => {
            let (h, b, tw, tf) = (d[0], d[1], d[2], d[3]);
            let (x, y) = (b / 2., h / 2.);
            let p = [[-tw / 2., -y], [tw / 2., -y], [tw / 2., y - tf], [x, y - tf], [x, y], [-x, y], [-x, y - tf], [-tw / 2., y - tf]];
            let a_w = tw * (h - tf);
            let a_f = b * tf;
            let cy = (a_w * (-y + (h - tf) / 2.) + a_f * (y - tf / 2.)) / (a_w + a_f);
            (vec![poly(&p)], b, h, [0., cy])
        }
        SectionKind::Cantoneira => {
            let (a, b, t) = (d[0], d[1], d[2]);
            let (x, y) = (b / 2., a / 2.);
            let p = [[-x, -y], [x, -y], [x, -y + t], [-x + t, -y + t], [-x + t, y], [-x, y]];
            let a1 = b * t;
            let a2 = (a - t) * t;
            let cx = (a1 * 0. + a2 * (-x + t / 2.)) / (a1 + a2);
            let cy = (a1 * (-y + t / 2.) + a2 * (-y + t + (a - t) / 2.)) / (a1 + a2);
            (vec![poly(&p)], b, a, [cx, cy])
        }
    })
}

/// Ponto de ancoragem: 0 centro, 1–8 cantos e meios da caixa (sentido horário a partir do
/// canto superior esquerdo: 1 sup. esq., 2 sup. meio, 3 sup. dir., 4 dir. meio, 5 inf. dir.,
/// 6 inf. meio, 7 inf. esq., 8 esq. meio), 9 centróide.
pub fn anchor_point(anchor: u8, w: f64, h: f64, c: [f64; 2]) -> [f64; 2] {
    let (x, y) = (w / 2., h / 2.);
    match anchor {
        1 => [-x, y],
        2 => [0., y],
        3 => [x, y],
        4 => [x, 0.],
        5 => [x, -y],
        6 => [0., -y],
        7 => [-x, -y],
        8 => [-x, 0.],
        9 => c,
        _ => [0., 0.],
    }
}

/// Desloca os laços para que o ponto de ancoragem fique na origem (linha guia).
pub fn shifted(loops: &[Loop2], by: [f64; 2]) -> Vec<Loop2> {
    let m = |p: [f64; 2]| [p[0] - by[0], p[1] - by[1]];
    loops
        .iter()
        .map(|l| {
            l.iter()
                .map(|s| match s {
                    Seg2::Line(a, b) => Seg2::Line(m(*a), m(*b)),
                    Seg2::Arc(a, b, c) => Seg2::Arc(m(*a), m(*b), m(*c)),
                    Seg2::Circle(c, r) => Seg2::Circle(m(*c), *r),
                    Seg2::Spline(p) => Seg2::Spline(p.iter().map(|q| m(*q)).collect()),
                    // espelhar inverte o sentido do parâmetro
                    Seg2::Ellipse(c, v, r, t0, t1) => Seg2::Ellipse(m(*c), m(*v), *r, -*t1, -*t0),
                    Seg2::Ellipse(c, a, r, t0, t1) => Seg2::Ellipse(m(*c), *a, *r, *t0, *t1),
                })
                .collect()
        })
        .collect()
}

/// Cota desenhada sobre a seção (B254): de `a` a `b`, em coordenadas da seção centrada na caixa.
#[derive(Clone, Debug, PartialEq)]
pub struct Cota {
    /// nome da dimensão (o mesmo de `dim_names`)
    pub name: &'static str,
    pub a: [f64; 2],
    pub b: [f64; 2],
    pub value: f64,
    /// medida total da caixa (largura ou altura); as demais são espessuras sobre a peça
    pub overall: bool,
}

/// Onde cada cota da seção fica no desenho, na ordem de `dim_names`. Uma cota total vai de lado a lado
/// da caixa; uma espessura cruza a parede ou a aba que ela mede. Devolve vazio se a seção é inválida.
pub fn dimension_marks(kind: SectionKind, d: &[f64]) -> Vec<Cota> {
    let Ok((_, w, h, _)) = section(kind, d) else { return vec![] };
    let names = dim_names(kind);
    let (x, y) = (w / 2., h / 2.);
    let horiz = |i: usize, v: f64| Cota { name: names[i], a: [-v / 2., y], b: [v / 2., y], value: v, overall: true };
    let vert = |i: usize, v: f64| Cota { name: names[i], a: [-x, -v / 2.], b: [-x, v / 2.], value: v, overall: true };
    let seg = |i: usize, a: [f64; 2], b: [f64; 2]| Cota { name: names[i], a, b, value: ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt(), overall: false };
    match kind {
        SectionKind::TuboQuadrado => vec![horiz(0, d[0]), seg(1, [x - d[1], 0.], [x, 0.])],
        SectionKind::TuboRetangular => vec![horiz(0, d[0]), vert(1, d[1]), seg(2, [x - d[2], 0.], [x, 0.])],
        SectionKind::TuboRedondo => vec![horiz(0, d[0]), seg(1, [x - d[1], 0.], [x, 0.])],
        SectionKind::BarraRedonda | SectionKind::BarraQuadrada => vec![horiz(0, d[0])],
        SectionKind::BarraChata => vec![horiz(0, d[0]), vert(1, d[1])],
        SectionKind::PerfilI => vec![vert(0, d[0]), horiz(1, d[1]), seg(2, [-d[2] / 2., 0.], [d[2] / 2., 0.]), seg(3, [x, -y], [x, -y + d[3]])],
        SectionKind::PerfilU => vec![vert(0, d[0]), horiz(1, d[1]), seg(2, [-x, 0.], [-x + d[2], 0.]), seg(3, [x, -y], [x, -y + d[3]])],
        SectionKind::PerfilT => vec![vert(0, d[0]), horiz(1, d[1]), seg(2, [-d[2] / 2., 0.], [d[2] / 2., 0.]), seg(3, [x, y - d[3]], [x, y])],
        SectionKind::Cantoneira => vec![vert(0, d[0]), horiz(1, d[1]), seg(2, [x, -y], [x, -y + d[2]])],
    }
}

/// O laço fecha? Cada segmento começa onde o anterior termina e o último volta ao primeiro; um
/// círculo sozinho já é fechado. Laço vazio não é fechado.
pub fn loop_closed(l: &Loop2) -> bool {
    let ends = |s: &Seg2| -> Option<([f64; 2], [f64; 2])> {
        match s {
            Seg2::Line(a, b) => Some((*a, *b)),
            Seg2::Arc(a, _, c) => Some((*a, *c)),
            Seg2::Circle(..) => None,
            Seg2::Spline(p) => Some((*p.first()?, *p.last()?)),
            Seg2::Ellipse(c, m, r, t0, t1) => {
                if (t1 - t0).abs() >= std::f64::consts::TAU - 1e-9 {
                    return None;
                }
                let g = m[0].hypot(m[1]).max(1e-12);
                let (ux, uy) = (m[0] / g, m[1] / g);
                let em = |t: f64| [c[0] + g * t.cos() * ux - r * t.sin() * uy, c[1] + g * t.cos() * uy + r * t.sin() * ux];
                Some((em(*t0), em(*t1)))
            }
        }
    };
    if l.is_empty() {
        return false;
    }
    let near = |p: [f64; 2], q: [f64; 2]| (p[0] - q[0]).abs() < 1e-9 && (p[1] - q[1]).abs() < 1e-9;
    if l.len() == 1 {
        return match ends(&l[0]) {
            None => true,
            Some((a, b)) => near(a, b),
        };
    }
    (0..l.len()).all(|i| match (ends(&l[i]), ends(&l[(i + 1) % l.len()])) {
        (Some((_, fim)), Some((ini, _))) => near(fim, ini),
        _ => false,
    })
}

/// Espelha os laços em torno do eixo Y da seção (x vira -x). A ordem dos pontos de cada laço fica
/// como está: o sólido não depende do sentido do contorno.
pub fn mirrored(loops: &[Loop2]) -> Vec<Loop2> {
    let m = |p: [f64; 2]| [-p[0], p[1]];
    loops
        .iter()
        .map(|l| {
            l.iter()
                .map(|s| match s {
                    Seg2::Line(a, b) => Seg2::Line(m(*a), m(*b)),
                    Seg2::Arc(a, b, c) => Seg2::Arc(m(*a), m(*b), m(*c)),
                    Seg2::Circle(c, r) => Seg2::Circle(m(*c), *r),
                    Seg2::Spline(p) => Seg2::Spline(p.iter().map(|q| m(*q)).collect()),
                    // espelhar inverte o sentido do parâmetro
                    Seg2::Ellipse(c, v, r, t0, t1) => Seg2::Ellipse(m(*c), m(*v), *r, -*t1, -*t0),
                })
                .collect()
        })
        .collect()
}

/// Espessura da parede do perfil (referência da sobra da placa em "razão da parede"): a parede dos
/// tubos, a alma dos perfis I/U/T, a espessura da cantoneira e da barra chata; 0 nas barras maciças.
pub fn wall(k: SectionKind, d: &[f64]) -> f64 {
    let at = |i: usize| d.get(i).copied().unwrap_or(0.);
    match k {
        SectionKind::TuboQuadrado | SectionKind::TuboRedondo => at(1),
        SectionKind::TuboRetangular => at(2),
        SectionKind::PerfilI | SectionKind::PerfilU | SectionKind::PerfilT => at(2),
        SectionKind::Cantoneira => at(2),
        SectionKind::BarraChata => at(1),
        SectionKind::BarraRedonda | SectionKind::BarraQuadrada => 0.,
    }
}

/// Agrupa as linhas do esqueleto por contiguidade: duas linhas que dividem uma ponta (dentro de
/// `tol`) caem no mesmo grupo, direta ou indiretamente. Cada grupo guarda os índices das linhas na
/// ordem em que foram escolhidas; os grupos saem pela primeira linha de cada um (B226).
pub fn contiguous_groups(ends: &[([f64; 3], [f64; 3])], tol: f64) -> Vec<Vec<usize>> {
    let n = ends.len();
    let mut root: Vec<usize> = (0..n).collect();
    fn find(r: &mut [usize], mut i: usize) -> usize {
        while r[i] != i {
            r[i] = r[r[i]];
            i = r[i];
        }
        i
    }
    let near = |p: [f64; 3], q: [f64; 3]| (0..3).map(|k| (p[k] - q[k]) * (p[k] - q[k])).sum::<f64>().sqrt() <= tol;
    for i in 0..n {
        for j in i + 1..n {
            let (a, b) = (ends[i], ends[j]);
            if near(a.0, b.0) || near(a.0, b.1) || near(a.1, b.0) || near(a.1, b.1) {
                let (ri, rj) = (find(&mut root, i), find(&mut root, j));
                if ri != rj {
                    root[rj.max(ri)] = ri.min(rj);
                }
            }
        }
    }
    let mut groups: Vec<Vec<usize>> = vec![];
    let mut slot: std::collections::HashMap<usize, usize> = Default::default();
    for i in 0..n {
        let r = find(&mut root, i);
        let g = *slot.entry(r).or_insert_with(|| {
            groups.push(vec![]);
            groups.len() - 1
        });
        groups[g].push(i);
    }
    groups
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secoes_validas_e_ancoragens() {
        for fam in builtin() {
            for (n, d) in &fam.sizes {
                assert!(section(fam.kind, d).is_ok(), "{} {n}", fam.name);
            }
        }
        let (_, w, h, c) = section(SectionKind::Cantoneira, &[50., 50., 5.]).unwrap();
        assert_eq!((w, h), (50., 50.));
        assert!(c[0] < 0. && c[1] < 0., "centróide da cantoneira puxado para o canto: {c:?}");
        assert_eq!(anchor_point(5, 40., 20., c), [20., -10.]);
        assert!(section(SectionKind::TuboQuadrado, &[20., 12.]).is_err());
    }

    #[test]
    fn espelhar_inverte_o_lado_do_perfil_sem_mudar_a_caixa() {
        let (l, w, h, c) = section(SectionKind::PerfilU, &[100., 50., 5., 8.]).unwrap();
        let xs = |ls: &[Loop2]| {
            let mut v = vec![];
            for lp in ls {
                for sg in lp {
                    if let Seg2::Line(a, _) = sg {
                        v.push(a[0]);
                    }
                }
            }
            (v.iter().cloned().fold(f64::MAX, f64::min), v.iter().cloned().fold(f64::MIN, f64::max))
        };
        let m = mirrored(&l);
        assert_eq!(xs(&l), (-w / 2., w / 2.));
        assert_eq!(xs(&m), (-w / 2., w / 2.));
        // o centróide do U fica do lado da alma: espelhar troca de lado
        assert!(c[0] < 0. && h == 100.);
        let primeiro = |ls: &[Loop2]| if let Seg2::Line(a, _) = ls[0][0] { a } else { unreachable!() };
        assert_eq!(primeiro(&m)[0], -primeiro(&l)[0]);
    }

    #[test]
    fn toda_medida_valida_de_fabrica_gera_lacos_fechados_e_cotas_certas() {
        for fam in builtin() {
            for (n, d) in &fam.sizes {
                let (loops, w, h, _) = section(fam.kind, d).unwrap();
                assert!(!loops.is_empty());
                for l in &loops {
                    assert!(loop_closed(l), "{} {n}: laço aberto {l:?}", fam.name);
                }
                // as cotas desenhadas medem exatamente o valor da dimensão que nomeiam
                let marcas = dimension_marks(fam.kind, d);
                assert_eq!(marcas.len(), dim_names(fam.kind).len(), "{} {n}", fam.name);
                for (i, c) in marcas.iter().enumerate() {
                    assert_eq!(c.name, dim_names(fam.kind)[i]);
                    let comp = ((c.b[0] - c.a[0]).powi(2) + (c.b[1] - c.a[1]).powi(2)).sqrt();
                    assert!((comp - d[i]).abs() < 1e-9, "{} {n}: cota {} mede {comp}, esperado {}", fam.name, c.name, d[i]);
                    // e as pontas ficam dentro da caixa da seção
                    for p in [c.a, c.b] {
                        assert!(p[0].abs() <= w / 2. + 1e-9 && p[1].abs() <= h / 2. + 1e-9, "{} {n}: {}", fam.name, c.name);
                    }
                }
            }
        }
        // medida inválida: sem cotas
        assert!(dimension_marks(SectionKind::TuboQuadrado, &[20., 12.]).is_empty());
    }

    #[test]
    fn laco_aberto_e_detectado() {
        let aberto = vec![Seg2::Line([0., 0.], [1., 0.]), Seg2::Line([1., 0.], [1., 1.]), Seg2::Line([1., 1.], [0., 1.])];
        assert!(!loop_closed(&aberto));
        assert!(!loop_closed(&vec![]));
        assert!(loop_closed(&vec![Seg2::Circle([0., 0.], 3.)]));
        assert!(loop_closed(&rect(0., 0., 2., 1.)));
    }

    #[test]
    fn toda_familia_de_fabrica_tem_grupo() {
        for f in builtin() {
            assert_eq!(f.norm, norm_of(f.kind));
            assert!(!f.norm.is_empty());
        }
    }

    #[test]
    fn parede_de_referencia_por_tipo() {
        assert_eq!(wall(SectionKind::TuboQuadrado, &[40., 3.]), 3.);
        assert_eq!(wall(SectionKind::TuboRetangular, &[40., 20., 2.]), 2.);
        assert_eq!(wall(SectionKind::PerfilU, &[100., 50., 5., 8.]), 5.);
        assert_eq!(wall(SectionKind::BarraRedonda, &[10.]), 0.);
    }

    #[test]
    fn linhas_contiguas_formam_um_grupo() {
        let l = |a: [f64; 3], b: [f64; 3]| (a, b);
        // L (duas linhas que se tocam), uma barra solta e uma terceira perna ligada ao L pela outra ponta
        let ends = [l([0., 0., 0.], [100., 0., 0.]), l([500., 0., 0.], [500., 90., 0.]), l([100., 0., 0.], [100., 80., 0.]), l([0., 0., 0.], [0., -60., 0.])];
        assert_eq!(contiguous_groups(&ends, 1e-4), vec![vec![0, 2, 3], vec![1]]);
        assert!(contiguous_groups(&[], 1e-4).is_empty());
    }
}
