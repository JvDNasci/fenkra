//! Estrutural: seções de perfis padronizados (com pontos de ancoragem), biblioteca de famílias e
//! a geração dos membros ao longo do esqueleto (linhas/curvas 3D), com cantos em meia-esquadria,
//! aparados ou com recorte de encaixe (folga de solda), placas de extremidade e reforços.

use scc_core::SectionKind;
use scc_kernel::{Loop2, Seg2};

/// Família de perfis: tipo de seção, nomes das cotas e tamanhos padronizados.
#[derive(Clone, Debug, PartialEq)]
pub struct Family {
    pub name: String,
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
    let f = |name: &str, kind, sizes: &[(&str, &[f64])]| Family { name: name.into(), kind, sizes: sizes.iter().map(|(n, d)| (n.to_string(), d.to_vec())).collect() };
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
                })
                .collect()
        })
        .collect()
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
}
