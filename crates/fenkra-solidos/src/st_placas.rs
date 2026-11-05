// SPDX-License-Identifier: LGPL-2.1-or-later
//! Estrutural: placa de extremidade e reforço (B255). A placa escolhe a face da ponta, fica fora ou
//! dentro do membro (com recuo), tem sobra em mm ou em razão da parede e furos de fixação. O reforço
//! tem abas D1/D2 com inversão, perfil triangular, chanfrado ou poligonal, lado da espessura,
//! posição e repetição ao longo da aresta de encontro.

use super::*;
use fenkra_kernel::{Loop2, Seg2};

/// Campo opcional em mm: vazio vale 0.
fn opt(src: &str, vars: &Vars, field: &str) -> Result<f64, String> {
    if src.trim().is_empty() {
        Ok(0.)
    } else {
        ev(src, vars, field)
    }
}

/// Faixas (início, fim) da espessura do reforço ao longo da aresta de encontro, medidas a partir do
/// meio da aresta (comprimento `l`). `offset` afasta da extremidade; `side` só decide a faixa da
/// posição "meio" (início e fim encostam a face no afastamento). Repetição: `Quantidade` espalha as
/// cópias por igual entre as duas extremidades úteis; `Espacamento` repete a cada `sp` a partir da
/// posição (para os dois lados no "meio").
#[allow(clippy::too_many_arguments)]
pub fn faixas_do_reforco(pos: GussetPos, side: GussetSide, offset: f64, l: f64, t: f64, rep: GussetRepeat, n: usize, sp: f64) -> Result<Vec<(f64, f64)>, String> {
    const EPS: f64 = 1e-6;
    if t <= 0. {
        return Err("a espessura do reforço precisa ser maior que 0".into());
    }
    if offset < 0. {
        return Err("o afastamento não pode ser negativo".into());
    }
    let (lo_min, hi_max) = (-l / 2. + offset, l / 2. - offset);
    if hi_max - lo_min < t - EPS {
        return Err(format!("o reforço (espessura {t:.3}) não cabe na aresta de encontro de {l:.3} mm com afastamento {offset:.3}"));
    }
    let base = match pos {
        GussetPos::Inicio => (lo_min, lo_min + t),
        GussetPos::Fim => (hi_max - t, hi_max),
        GussetPos::Meio => match side {
            GussetSide::DoisLados => (offset - t / 2., offset + t / 2.),
            GussetSide::Interno => (offset - t, offset),
            GussetSide::Externo => (offset, offset + t),
        },
    };
    if base.0 < lo_min - EPS || base.1 > hi_max + EPS {
        return Err("o reforço sai da aresta de encontro: reduza o afastamento ou mude a posição".into());
    }
    Ok(match rep {
        GussetRepeat::Nenhuma => vec![base],
        GussetRepeat::Quantidade => {
            if n == 0 {
                return Err("a quantidade precisa ser pelo menos 1".into());
            }
            if n == 1 {
                vec![base]
            } else {
                let passo = (hi_max - lo_min - t) / (n - 1) as f64;
                if passo < t - EPS {
                    return Err(format!("{n} reforços de espessura {t:.3} não cabem na aresta: eles se sobrepõem"));
                }
                (0..n).map(|i| (lo_min + passo * i as f64, lo_min + passo * i as f64 + t)).collect()
            }
        }
        GussetRepeat::Espacamento => {
            if sp < t - EPS {
                return Err("o espaçamento precisa ser pelo menos a espessura do reforço".into());
            }
            let dentro = |q: (f64, f64)| q.0 >= lo_min - EPS && q.1 <= hi_max + EPS;
            let mut v = vec![base];
            let sentidos: &[f64] = match pos {
                GussetPos::Inicio => &[1.],
                GussetPos::Fim => &[-1.],
                GussetPos::Meio => &[1., -1.],
            };
            for s in sentidos {
                let mut k = 1.;
                loop {
                    let q = (base.0 + s * k * sp, base.1 + s * k * sp);
                    if !dentro(q) || k > 10_000. {
                        break;
                    }
                    v.push(q);
                    k += 1.;
                }
            }
            v.sort_by(|a, b| a.0.total_cmp(&b.0));
            v
        }
    })
}

/// Perfil do reforço no plano das abas: (u ao longo de D1, v ao longo de D2), a partir do canto.
pub fn perfil_do_reforco(gs: &StructGusset, vars: &Vars, la: f64, lb: f64) -> Result<Vec<[f64; 2]>, String> {
    match gs.profile {
        GussetProfile::Chanfrado => {
            let c = opt(&gs.chamfer, vars, "chanfro")?;
            if c < 0. {
                return Err("o chanfro não pode ser negativo".into());
            }
            if c >= la.min(lb) - 1e-9 && c > 1e-9 {
                return Err("o chanfro de alívio precisa ser menor que as abas".into());
            }
            Ok(if c > 1e-9 { vec![[c, 0.], [la, 0.], [0., lb], [0., c]] } else { vec![[0., 0.], [la, 0.], [0., lb]] })
        }
        GussetProfile::Poligonal => {
            if gs.poly.len() < 3 {
                return Err("o perfil poligonal precisa de pelo menos 3 vértices".into());
            }
            let pts: Vec<[f64; 2]> = gs
                .poly
                .iter()
                .enumerate()
                .map(|(i, q)| Ok([ev(&q[0], vars, &format!("vértice {} (u)", i + 1))?, ev(&q[1], vars, &format!("vértice {} (v)", i + 1))?]))
                .collect::<Result<_, String>>()?;
            let area2: f64 = (0..pts.len()).map(|i| pts[i][0] * pts[(i + 1) % pts.len()][1] - pts[(i + 1) % pts.len()][0] * pts[i][1]).sum();
            if area2.abs() < 1e-9 {
                return Err("o perfil poligonal não tem área".into());
            }
            Ok(pts)
        }
    }
}

impl Ctx<'_> {
    /// Placa de extremidade (B255).
    pub(super) fn struct_plate(&mut self, f: &Feature, pl: &StructPlate) -> Result<Option<String>, String> {
        let k = |e: KernelError| e.0;
        let (g, end_at_start) = self.member_end(pl.at, f)?;
        let t = ev(&pl.thickness, &self.vars, "espessura")?;
        if t <= 0. {
            return Err("a espessura da placa precisa ser maior que 0".into());
        }
        let raw = ev(&pl.margin, &self.vars, "sobra")?;
        let mg = match pl.margin_mode {
            MarginMode::Valor => raw,
            MarginMode::RazaoParede => {
                if g.wall <= 0. {
                    return Err("o perfil é maciço e não tem parede: dê a sobra em mm".into());
                }
                raw * g.wall
            }
        };
        let recess = opt(&pl.recess, &self.vars, "recuo")?;
        if recess < 0. {
            return Err("o recuo não pode ser negativo".into());
        }
        let (e, out) = if end_at_start { (geo3d::sub(g.start, geo3d::mul(g.z, g.ext.0)), geo3d::mul(g.z, -1.)) } else { (geo3d::add(g.end, geo3d::mul(g.z, g.ext.1)), g.z) };
        let (w, h) = (g.w + 2. * mg, g.h + 2. * mg);
        if w <= 1e-9 || h <= 1e-9 {
            return Err("a sobra negativa deixa a placa sem largura".into());
        }
        // plano da base da placa: além da ponta (externa) ou recuado para dentro do membro (interna)
        let base = match pl.side {
            PlateSide::Externo => e,
            PlateSide::Interno => geo3d::sub(e, geo3d::mul(out, recess + t)),
        };
        // centro da seção (a linha guia pode estar num canto, pela ancoragem)
        let c = geo3d::add(geo3d::add(base, geo3d::mul(g.x, -g.shift[0])), geo3d::mul(g.y, -g.shift[1]));
        let fr = Frame { origin: c, x: g.x, normal: out };
        let (hw, hh) = (w / 2., h / 2.);
        let mut loops: Vec<Loop2> = vec![vec![Seg2::Line([-hw, -hh], [hw, -hh]), Seg2::Line([hw, -hh], [hw, hh]), Seg2::Line([hw, hh], [-hw, hh]), Seg2::Line([-hw, hh], [-hw, -hh])]];
        if let Some(ho) = &pl.holes {
            let d = ev(&ho.diameter, &self.vars, "diâmetro do furo")?;
            let edge = ev(&ho.edge, &self.vars, "distância do furo à borda")?;
            if d <= 0. {
                return Err("o diâmetro do furo precisa ser maior que 0".into());
            }
            if edge < d / 2. - 1e-9 {
                return Err("o furo sai da placa: a distância à borda precisa ser pelo menos o raio".into());
            }
            let (px, py) = (hw - edge, hh - edge);
            if px <= d / 2. || py <= d / 2. {
                return Err("os furos se tocam ou ficam fora: aumente a placa ou reduza o furo".into());
            }
            for (sx, sy) in [(-1., -1.), (1., -1.), (1., 1.), (-1., 1.)] {
                loops.push(vec![Seg2::Circle([sx * px, sy * py], d / 2.)]);
            }
        }
        let face = Shape::face(&fr, &loops).map_err(k)?;
        let plate = face.extrude(out, t, 0.).map_err(k)?;
        log_struct(f, 0, StructItem::Plate, "Placa", "Placa".into(), 0., fr, &plate, "");
        self.add_separate(f, plate);
        Ok(None)
    }

    /// Reforço entre dois membros (B255).
    pub(super) fn struct_gusset(&mut self, f: &Feature, gs: &StructGusset) -> Result<Option<String>, String> {
        let k = |e: KernelError| e.0;
        let (a, b, p) = self.member_pair(gs.at, f)?;
        let d1 = ev(&gs.leg, &self.vars, "aba D1")?;
        let d2 = if gs.leg2.trim().is_empty() { d1 } else { ev(&gs.leg2, &self.vars, "aba D2")? };
        let t = ev(&gs.thickness, &self.vars, "espessura")?;
        if d1 <= 0. || d2 <= 0. {
            return Err("as abas D1 e D2 precisam ser maiores que 0".into());
        }
        // com a inversão, D1 corre pelo segundo membro e D2 pelo primeiro
        let (la, lb) = if gs.swap { (d2, d1) } else { (d1, d2) };
        let (da, db) = (a.1, b.1);
        let n = geo3d::unit(fenkra_kernel::cross(da, db)).ok_or("membros colineares não levam reforço")?;
        // canto do reforço junto às faces dos membros, no plano das linhas guia
        let ha = a.0.w.max(a.0.h) / 2.;
        let hb = b.0.w.max(b.0.h) / 2.;
        let q0 = geo3d::add(geo3d::add(p, geo3d::mul(da, hb)), geo3d::mul(db, ha));
        let perfil = perfil_do_reforco(gs, &self.vars, la, lb)?;
        // aresta de encontro: a menor extensão da seção dos dois membros ao longo de n
        let ext = |g: &MemberGeom| geo3d::dot(n, g.x).abs() * g.w + geo3d::dot(n, g.y).abs() * g.h;
        let l = ext(&a.0).min(ext(&b.0));
        let off = opt(&gs.offset, &self.vars, "afastamento")?;
        let count = match gs.repeat {
            GussetRepeat::Quantidade => {
                let c = ev(&gs.count, &self.vars, "quantidade")?;
                if c < 1. || (c - c.round()).abs() > 1e-9 {
                    return Err("a quantidade precisa ser um inteiro a partir de 1".into());
                }
                c.round() as usize
            }
            _ => 1,
        };
        let sp = if gs.repeat == GussetRepeat::Espacamento { ev(&gs.spacing, &self.vars, "espaçamento")? } else { 0. };
        let faixas = faixas_do_reforco(gs.position, gs.side, off, l, t, gs.repeat, count, sp)?;
        let mut all = Shape::empty();
        for (i, (lo, _)) in faixas.iter().enumerate() {
            let q = geo3d::add(q0, geo3d::mul(n, *lo));
            let pts: Vec<V3> = perfil.iter().map(|uv| geo3d::add(geo3d::add(q, geo3d::mul(da, uv[0])), geo3d::mul(db, uv[1]))).collect();
            let plate = Shape::poly_prism(&pts, geo3d::mul(n, t)).map_err(k)?;
            log_struct(f, i, StructItem::Gusset, "Reforço", "Reforço".into(), 0., Frame { origin: q, x: da, normal: n }, &plate, "");
            all = if all.is_null() { plate } else { all.compound(&plate) };
        }
        self.add_separate(f, all);
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quase(a: &[(f64, f64)], b: &[(f64, f64)]) -> bool {
        a.len() == b.len() && a.iter().zip(b).all(|(x, y)| (x.0 - y.0).abs() < 1e-9 && (x.1 - y.1).abs() < 1e-9)
    }

    #[test]
    fn posicao_inicio_meio_fim_com_afastamento() {
        use GussetPos::*;
        use GussetRepeat::Nenhuma;
        use GussetSide::*;
        let f = |p, s, o| faixas_do_reforco(p, s, o, 40., 6., Nenhuma, 1, 0.).unwrap();
        assert!(quase(&f(Inicio, DoisLados, 0.), &[(-20., -14.)]));
        assert!(quase(&f(Inicio, DoisLados, 4.), &[(-16., -10.)]));
        assert!(quase(&f(Fim, DoisLados, 4.), &[(10., 16.)]));
        assert!(quase(&f(Meio, DoisLados, 0.), &[(-3., 3.)]));
        assert!(quase(&f(Meio, Interno, 0.), &[(-6., 0.)]));
        assert!(quase(&f(Meio, Externo, 2.), &[(2., 8.)]));
        // não cabe / sai da aresta
        assert!(faixas_do_reforco(Meio, DoisLados, 18., 40., 6., Nenhuma, 1, 0.).is_err());
        assert!(faixas_do_reforco(Meio, DoisLados, 0., 4., 6., Nenhuma, 1, 0.).is_err());
    }

    #[test]
    fn repeticao_por_quantidade_e_por_espacamento() {
        use GussetPos::*;
        use GussetRepeat::*;
        use GussetSide::DoisLados;
        // 3 cópias espalhadas por igual: ficam nas duas pontas e no meio
        let q = faixas_do_reforco(Meio, DoisLados, 0., 40., 6., Quantidade, 3, 0.).unwrap();
        assert!(quase(&q, &[(-20., -14.), (-3., 3.), (14., 20.)]), "{q:?}");
        // espaçamento a partir do início, até acabar a aresta
        let e = faixas_do_reforco(Inicio, DoisLados, 0., 40., 6., Espacamento, 1, 12.).unwrap();
        assert!(quase(&e, &[(-20., -14.), (-8., -2.), (4., 10.)]), "{e:?}");
        // do fim para o início
        let e = faixas_do_reforco(Fim, DoisLados, 0., 40., 6., Espacamento, 1, 15.).unwrap();
        assert!(quase(&e, &[(-16., -10.), (-1., 5.), (14., 20.)]), "{e:?}");
        // no meio, o grupo cresce para os dois lados
        let m = faixas_do_reforco(Meio, DoisLados, 0., 40., 6., Espacamento, 1, 10.).unwrap();
        assert_eq!(m.len(), 3);
        assert!(quase(&m, &[(-13., -7.), (-3., 3.), (7., 13.)]), "{m:?}");
        // sobreposição e quantidade zero são recusadas
        assert!(faixas_do_reforco(Meio, DoisLados, 0., 40., 6., Quantidade, 8, 0.).is_err());
        assert!(faixas_do_reforco(Meio, DoisLados, 0., 40., 6., Quantidade, 0, 0.).is_err());
        assert!(faixas_do_reforco(Meio, DoisLados, 0., 40., 6., Espacamento, 1, 3.).is_err());
    }

    #[test]
    fn perfil_triangular_chanfrado_e_poligonal() {
        let vars = Vars::new();
        let mut g = StructGusset::novo([0.; 3], "60", "6", "0");
        let tri = perfil_do_reforco(&g, &vars, 60., 40.).unwrap();
        assert_eq!(tri, vec![[0., 0.], [60., 0.], [0., 40.]]);
        g.chamfer = "10".into();
        assert_eq!(perfil_do_reforco(&g, &vars, 60., 40.).unwrap().len(), 4);
        g.chamfer = "40".into();
        assert!(perfil_do_reforco(&g, &vars, 60., 40.).is_err());
        g.profile = GussetProfile::Poligonal;
        assert!(perfil_do_reforco(&g, &vars, 60., 40.).is_err(), "poligonal sem vértices");
        g.poly = vec![["0".into(), "0".into()], ["30".into(), "0".into()], ["30".into(), "10".into()], ["0".into(), "40".into()]];
        assert_eq!(perfil_do_reforco(&g, &vars, 60., 40.).unwrap().len(), 4);
        g.poly = vec![["0".into(), "0".into()], ["10".into(), "0".into()], ["20".into(), "0".into()]];
        assert!(perfil_do_reforco(&g, &vars, 60., 40.).is_err(), "colinear não tem área");
    }
}
