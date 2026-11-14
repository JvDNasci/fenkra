// SPDX-License-Identifier: LGPL-2.1-or-later
//! B148: geometria dos recursos de plástico.
//!
//! **Boss.** Cada ponto do sketch vira um pino que cresce pela normal do plano. O corpo do pino é a
//! revolução de um perfil em (r, z): a parede externa é um tronco de cone (raio `D/2` no topo, mais
//! largo na base pela saída), a base tem o filete côncavo que encosta no plano e na parede (o centro
//! do arco fica a `raio` dos dois), e o topo tem o rebaixo do inserto e o furo. Fazer o filete no
//! perfil, e não depois com o comando de filete, dá a medida exata da regra e não depende de achar a
//! aresta da base depois da booleana. As nervuras de apoio são prismas finos radiais, de seção
//! trapezoidal (encostam na parede, descem até a base e se alargam até `nervura_larg` além dela);
//! entram sozinhas quando a altura passa de 3 diâmetros. Tudo é unido ao corpo do coletor.

use super::*;
use fenkra_core::recurso_plastico::{Boss, RecursoPlastico, BOSS_NERVURA_DIAMETROS};
use fenkra_kernel::Seg2;

/// Medidas do boss já avaliadas e conferidas.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BossMedidas {
    /// raio externo no topo e na base (a base é mais larga pela saída)
    pub r_topo: f64,
    pub r_base: f64,
    pub altura: f64,
    /// saída em graus
    pub saida: f64,
    pub raio: f64,
    pub r_furo: f64,
    pub furo_prof: f64,
    pub r_inserto: f64,
    pub inserto_prof: f64,
    pub nervuras: u32,
    pub nervura_esp: f64,
    pub nervura_larg: f64,
    pub nervura_alt: f64,
}

fn opcional(s: &str, vars: &Vars, nome: &str, padrao: f64) -> Result<f64, String> {
    if s.trim().is_empty() {
        Ok(padrao)
    } else {
        ev(s, vars, nome)
    }
}

/// Avalia e confere as medidas do boss.
pub fn boss_medidas(b: &Boss, vars: &Vars) -> Result<BossMedidas, String> {
    let d = ev(&b.diametro, vars, "diâmetro do boss")?;
    let h = ev(&b.altura, vars, "altura do boss")?;
    let saida = ev(&b.saida, vars, "saída do boss")?;
    let raio = ev(&b.raio, vars, "filete da base")?;
    let furo = ev(&b.furo, vars, "diâmetro do furo")?;
    if !(d > 0. && h > 0.) {
        return Err("o boss precisa de diâmetro e altura positivos".into());
    }
    if !(0. ..45.).contains(&saida) {
        return Err("a saída do boss fica entre 0 e 45 graus".into());
    }
    if raio < 0. {
        return Err("o filete da base não pode ser negativo".into());
    }
    if furo < 0. || furo >= d {
        return Err("o furo do boss precisa ser menor que o diâmetro externo".into());
    }
    let furo_prof = if furo > 0. { ev(&b.furo_prof, vars, "profundidade do furo")? } else { 0. };
    if furo > 0. && !(furo_prof > 0. && furo_prof <= h + 1e-9) {
        return Err("a profundidade do furo vai de zero até a altura do boss".into());
    }
    let inserto = opcional(&b.inserto, vars, "diâmetro do inserto", 0.)?;
    let inserto_prof = if inserto > 0. { opcional(&b.inserto_prof, vars, "profundidade do inserto", 0.)? } else { 0. };
    if inserto > 0. && (inserto <= furo || inserto >= d) {
        return Err("o rebaixo do inserto fica entre o furo e o diâmetro externo".into());
    }
    if inserto > 0. && !(inserto_prof > 0. && (inserto_prof < furo_prof || furo == 0. && inserto_prof <= h)) {
        return Err("a profundidade do inserto precisa ser positiva e menor que a do furo".into());
    }
    let auto = if h > BOSS_NERVURA_DIAMETROS * d + 1e-9 { 4. } else { 0. };
    let n = opcional(&b.nervuras, vars, "número de nervuras", auto)?;
    if !(0. ..=16.).contains(&n) || n.fract().abs() > 1e-9 {
        return Err("o número de nervuras é um inteiro de 0 a 16".into());
    }
    let nervura_esp = if n > 0. { ev(&b.nervura_esp, vars, "espessura da nervura")? } else { 0. };
    let nervura_larg = opcional(&b.nervura_larg, vars, "largura da nervura", d / 2.)?;
    let nervura_alt = opcional(&b.nervura_alt, vars, "altura da nervura", 0.7 * h)?;
    let r_topo = d / 2.;
    if n > 0. {
        if nervura_esp <= 0. || nervura_larg <= 0. || nervura_alt <= 0. {
            return Err("as nervuras do boss precisam de espessura, largura e altura positivas".into());
        }
        if nervura_alt > h + 1e-9 {
            return Err("a nervura não passa da altura do boss".into());
        }
        // a face interna da nervura entra na parede (ver `boss_solido`); ela precisa caber no topo
        if nervura_esp >= 2. * r_topo {
            return Err("a nervura é mais grossa que o diâmetro do boss".into());
        }
    }
    let r_base = r_topo + h * saida.to_radians().tan();
    Ok(BossMedidas {
        r_topo,
        r_base,
        altura: h,
        saida,
        raio,
        r_furo: furo / 2.,
        furo_prof,
        r_inserto: inserto / 2.,
        inserto_prof,
        nervuras: n as u32,
        nervura_esp,
        nervura_larg,
        nervura_alt,
    })
}

/// Perfil do boss em (r, z), fechado, para girar em torno do eixo r = 0.
pub fn boss_perfil(m: &BossMedidas) -> Vec<Seg2> {
    let a = m.saida.to_radians();
    let mut pts: Vec<[f64; 2]> = vec![[0., 0.]];
    let mut segs = vec![];
    let linha = |pts: &mut Vec<[f64; 2]>, segs: &mut Vec<Seg2>, p: [f64; 2]| {
        let q = *pts.last().unwrap();
        if (p[0] - q[0]).hypot(p[1] - q[1]) > 1e-12 {
            segs.push(Seg2::Line(q, p));
            pts.push(p);
        }
    };
    if m.raio > 0. {
        // filete côncavo da base: tangente ao plano (z = 0) e à parede inclinada
        let dt = m.raio * (1. - a.sin()) / a.cos();
        let c = [m.r_base + dt, m.raio];
        let t1 = [c[0], 0.];
        let t2 = [c[0] - m.raio * a.cos(), m.raio - m.raio * a.sin()];
        let bis = [-(a.cos()), -(1. + a.sin())];
        let nb = bis[0].hypot(bis[1]);
        let meio = [c[0] + m.raio * bis[0] / nb, c[1] + m.raio * bis[1] / nb];
        linha(&mut pts, &mut segs, t1);
        segs.push(Seg2::Arc(t1, meio, t2));
        pts.push(t2);
    } else {
        linha(&mut pts, &mut segs, [m.r_base, 0.]);
    }
    let h = m.altura;
    linha(&mut pts, &mut segs, [m.r_topo, h]);
    if m.r_inserto > 0. {
        linha(&mut pts, &mut segs, [m.r_inserto, h]);
        linha(&mut pts, &mut segs, [m.r_inserto, h - m.inserto_prof]);
    }
    if m.r_furo > 0. {
        let z = if m.r_inserto > 0. { h - m.inserto_prof } else { h };
        linha(&mut pts, &mut segs, [m.r_furo, z]);
        linha(&mut pts, &mut segs, [m.r_furo, h - m.furo_prof]);
        linha(&mut pts, &mut segs, [0., h - m.furo_prof]);
    } else if m.r_inserto > 0. {
        linha(&mut pts, &mut segs, [0., h - m.inserto_prof]);
    } else {
        linha(&mut pts, &mut segs, [0., h]);
    }
    linha(&mut pts, &mut segs, [0., 0.]);
    segs
}

fn cruz(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

/// Sólido de um boss com a base em `o`, eixo `n` e as nervuras partindo da direção `x`.
pub fn boss_solido(m: &BossMedidas, o: V3, n: V3, x: V3) -> Result<Shape, String> {
    let k = |e: KernelError| e.0;
    let x = geo3d::unit(geo3d::sub(x, geo3d::mul(n, geo3d::dot(x, n)))).unwrap_or_else(|| geo3d::any_perp(n));
    let fr = Frame { origin: o, x, normal: cruz(x, n) };
    let face = Shape::face(&fr, &[boss_perfil(m)]).map_err(k)?;
    let mut s = face.revolve(o, n, 360., 0.).map_err(k)?;
    let y = cruz(n, x);
    let a = m.saida.to_radians();
    for i in 0..m.nervuras {
        let th = std::f64::consts::TAU * i as f64 / m.nervuras as f64;
        let e = geo3d::add(geo3d::mul(x, th.cos()), geo3d::mul(y, th.sin()));
        let tg = cruz(n, e);
        // a face interna fica dentro da parede, para a união não deixar fresta
        let r_vazio = m.r_furo.max(m.r_inserto);
        let dentro = m.nervura_esp.min((m.r_topo - r_vazio) / 2.);
        let ri = m.r_topo - dentro;
        let r_alto = m.r_base - m.nervura_alt * a.tan();
        let base = geo3d::sub(o, geo3d::mul(tg, m.nervura_esp / 2.));
        let p = |r: f64, z: f64| geo3d::add(base, geo3d::add(geo3d::mul(e, r), geo3d::mul(n, z)));
        let quad = [p(ri, 0.), p(m.r_base + m.nervura_larg, 0.), p(r_alto, m.nervura_alt), p(ri, m.nervura_alt)];
        let nerv = Shape::poly_prism(&quad, geo3d::mul(tg, m.nervura_esp)).map_err(k)?;
        s = s.boolean(&nerv, Bool::Fuse).map_err(k)?;
    }
    Ok(s)
}

impl Ctx<'_> {
    pub(super) fn plastico_rec(&mut self, f: &Feature, p: &RecursoPlastico) -> Result<Option<String>, String> {
        match p {
            RecursoPlastico::Boss(b) => self.plastico_boss(f, b),
        }
    }

    fn plastico_boss(&mut self, f: &Feature, b: &Boss) -> Result<Option<String>, String> {
        let k = |e: KernelError| e.0;
        let (frame, sk) = self.sketch(b.sketch)?;
        if sk.points.is_empty() {
            return Err("o sketch do boss precisa de pelo menos um ponto (centro)".into());
        }
        let m = boss_medidas(b, &self.vars)?;
        let n = if b.reverse { frame.normal.map(|x| -x) } else { frame.normal };
        let mut tool = Shape::empty();
        for q in &sk.points {
            let s = boss_solido(&m, frame.point(q[0], q[1]), n, frame.x)?;
            tool = if tool.is_null() { s } else { tool.boolean(&s, Bool::Fuse).map_err(k)? };
        }
        self.apply(f.target, tool, BoolOp::Add, f.id)?;
        let aviso = (m.nervuras == 0 && m.altura > BOSS_NERVURA_DIAMETROS * 2. * m.r_topo + 1e-9)
            .then(|| format!("boss com altura de {:.3} mm passa de {} diâmetros sem nervuras de apoio", m.altura, BOSS_NERVURA_DIAMETROS));
        Ok(aviso)
    }
}
