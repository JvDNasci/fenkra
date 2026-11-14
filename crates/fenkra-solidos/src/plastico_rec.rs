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
use fenkra_core::recurso_plastico::{Boss, Grelha, Lip, RecursoPlastico, Snap, BOSS_NERVURA_DIAMETROS};
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

/// Medidas do snap-fit já avaliadas e conferidas.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SnapMedidas {
    pub comprimento: f64,
    pub espessura: f64,
    pub gancho: f64,
    pub gancho_alt: f64,
    pub raio: f64,
    /// deformação admissível, fração (0,02 = 2%)
    pub adm: f64,
}

impl SnapMedidas {
    /// Braço da viga: da raiz até a face de retenção do gancho.
    pub fn braco(&self) -> f64 {
        self.comprimento - self.gancho_alt
    }
    /// Deformação na raiz ao defletir o gancho inteiro: 1,5 t y / l² (viga em balanço de seção constante).
    pub fn deformacao(&self) -> f64 {
        1.5 * self.espessura * self.gancho / self.braco().powi(2)
    }
}

pub fn snap_medidas(s: &Snap, vars: &Vars) -> Result<SnapMedidas, String> {
    let m = SnapMedidas {
        comprimento: ev(&s.comprimento, vars, "comprimento do snap")?,
        espessura: ev(&s.espessura, vars, "espessura do snap")?,
        gancho: ev(&s.gancho, vars, "altura do gancho")?,
        gancho_alt: ev(&s.gancho_alt, vars, "comprimento do gancho")?,
        raio: ev(&s.raio, vars, "filete da raiz")?,
        adm: ev(&s.deformacao_adm, vars, "deformação admissível")? / 100.,
    };
    if !(m.comprimento > 0. && m.espessura > 0. && m.gancho > 0. && m.gancho_alt > 0.) {
        return Err("o snap-fit precisa de comprimento, espessura e gancho positivos".into());
    }
    if m.gancho_alt >= m.comprimento {
        return Err("o gancho precisa ser mais curto que a viga".into());
    }
    if m.raio < 0. || m.raio >= m.braco() {
        return Err("o filete da raiz vai de zero até o braço da viga".into());
    }
    if !(m.adm > 0.) {
        return Err("a deformação admissível precisa ser positiva".into());
    }
    Ok(m)
}

/// Perfil do snap no plano (m, n): m sai da linha para o lado do gancho, n sobe pela viga.
pub fn snap_perfil(m: &SnapMedidas) -> Vec<Seg2> {
    let (t, l, y, h, r) = (m.espessura, m.comprimento, m.gancho, m.gancho_alt, m.raio);
    let k = r * (1. - std::f64::consts::FRAC_1_SQRT_2);
    let mut v = vec![];
    if r > 0. {
        v.push(Seg2::Line([-r, 0.], [t + r, 0.]));
        v.push(Seg2::Arc([t + r, 0.], [t + k, k], [t, r]));
        v.push(Seg2::Line([t, r], [t, l - h]));
    } else {
        v.push(Seg2::Line([0., 0.], [t, 0.]));
        v.push(Seg2::Line([t, 0.], [t, l - h]));
    }
    v.push(Seg2::Line([t, l - h], [t + y, l - h]));
    v.push(Seg2::Line([t + y, l - h], [t, l]));
    v.push(Seg2::Line([t, l], [0., l]));
    if r > 0. {
        v.push(Seg2::Line([0., l], [0., r]));
        v.push(Seg2::Arc([0., r], [-k, k], [-r, 0.]));
    } else {
        v.push(Seg2::Line([0., l], [0., 0.]));
    }
    v
}

/// Sólido de um snap com a raiz na linha `a`–`b`, viga pela normal `n` e gancho para o lado `lado`.
pub fn snap_solido(m: &SnapMedidas, a: V3, b: V3, n: V3, lado: bool) -> Result<Shape, String> {
    let k = |e: KernelError| e.0;
    let larg = geo3d::dist(a, b);
    let d = geo3d::unit(geo3d::sub(b, a)).ok_or("linha do snap sem comprimento")?;
    // m = n × d sai da linha no plano; no outro lado o perfil parte de b e anda para a
    let (o, mv, dir) = if lado { (b, geo3d::mul(cruz(n, d), -1.), geo3d::mul(d, -1.)) } else { (a, cruz(n, d), d) };
    let fr = Frame { origin: o, x: mv, normal: dir };
    let face = Shape::face(&fr, &[snap_perfil(m)]).map_err(k)?;
    face.extrude(dir, larg, 0.).map_err(k)
}

/// Medidas da grelha já avaliadas.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GrelhaMedidas {
    pub borda: f64,
    pub passo: f64,
    pub nervura: f64,
    pub angulo: f64,
    /// `None` = atravessa a parede
    pub profundidade: Option<f64>,
}

pub fn grelha_medidas(g: &Grelha, vars: &Vars) -> Result<GrelhaMedidas, String> {
    let m = GrelhaMedidas {
        borda: ev(&g.borda, vars, "borda da grelha")?,
        passo: ev(&g.passo, vars, "passo da grelha")?,
        nervura: ev(&g.nervura, vars, "barra da grelha")?,
        angulo: opcional(&g.angulo, vars, "ângulo das barras", 0.)?,
        profundidade: if g.profundidade.trim().is_empty() { None } else { Some(ev(&g.profundidade, vars, "profundidade da grelha")?) },
    };
    if m.borda < 0. {
        return Err("a borda da grelha não pode ser negativa".into());
    }
    if !(m.nervura > 0. && m.passo > m.nervura) {
        return Err("o passo da grelha precisa ser maior que a largura das barras".into());
    }
    if m.profundidade.is_some_and(|p| p <= 0.) {
        return Err("a profundidade da grelha precisa ser positiva".into());
    }
    Ok(m)
}

/// Barras retas de largura `w`, com eixo a cada `passo` a partir de `c` na direção `f`, ao longo de `e`,
/// cobrindo um quadrado de meia diagonal `meia`, empurradas por `v`.
fn barras(c: V3, e: V3, f: V3, passo: f64, w: f64, meia: f64, v: V3) -> Result<Shape, String> {
    let k = |e: KernelError| e.0;
    let n = (meia / passo).ceil() as i64 + 1;
    let mut s = Shape::empty();
    for i in -n..=n {
        let o = geo3d::add(c, geo3d::mul(f, i as f64 * passo));
        let p = |a: f64, b: f64| geo3d::add(o, geo3d::add(geo3d::mul(e, a), geo3d::mul(f, b)));
        let quad = [p(-meia, -w / 2.), p(meia, -w / 2.), p(meia, w / 2.), p(-meia, w / 2.)];
        let b = Shape::poly_prism(&quad, v).map_err(k)?;
        s = if s.is_null() { b } else { s.boolean(&b, Bool::Fuse).map_err(k)? };
    }
    Ok(s)
}

/// Medidas do lip já avaliadas: a faixa (de fora e de dentro, medidas do contorno para dentro) e a
/// profundidade, já com a folga quando é ranhura.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LipMedidas {
    pub fora: f64,
    pub dentro: f64,
    pub altura: f64,
}

pub fn lip_medidas(l: &Lip, vars: &Vars) -> Result<LipMedidas, String> {
    let recuo = ev(&l.recuo, vars, "recuo do lip")?;
    let largura = ev(&l.largura, vars, "largura do lip")?;
    let altura = ev(&l.altura, vars, "altura do lip")?;
    let folga = ev(&l.folga, vars, "folga do lip")?;
    if recuo < 0. || !(largura > 0. && altura > 0.) || folga < 0. {
        return Err("o lip precisa de largura e altura positivas, e recuo e folga não negativos".into());
    }
    Ok(if l.ranhura {
        LipMedidas { fora: recuo - folga, dentro: recuo + largura + folga, altura: altura + folga }
    } else {
        LipMedidas { fora: recuo, dentro: recuo + largura, altura }
    })
}

/// Contorno deslocado para dentro por `d` (negativo vai para fora); zero devolve o próprio contorno.
fn para_dentro(face: &Shape, d: f64) -> Result<Shape, String> {
    if d.abs() < 1e-12 {
        return Ok(face.clone());
    }
    face.offset_face(-d).map_err(|e| format!("o contorno do lip não aceita o recuo de {d} mm: {}", e.0))
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
            RecursoPlastico::Snap(s) => self.plastico_snap(f, s),
            RecursoPlastico::Grelha(g) => self.plastico_grelha(f, g),
            RecursoPlastico::Lip(l) => self.plastico_lip(f, l),
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

    fn plastico_grelha(&mut self, f: &Feature, g: &Grelha) -> Result<Option<String>, String> {
        let k = |e: KernelError| e.0;
        let (frame, sk) = self.sketch(g.sketch)?;
        if sk.loops.is_empty() {
            return Err("o sketch da grelha precisa de um perfil fechado (a região vazada)".into());
        }
        let m = grelha_medidas(g, &self.vars)?;
        let regiao = Shape::face(&frame, &sk.loops).map_err(k)?;
        let abertura = if m.borda > 0. { regiao.offset_face(-m.borda).map_err(|e| format!("a borda de {} mm não cabe na região da grelha: {}", m.borda, e.0))? } else { regiao };
        let (lo, hi) = abertura.bbox().ok_or("região da grelha vazia")?;
        let c = [0, 1, 2].map(|i| (lo[i] + hi[i]) / 2.);
        let meia = geo3d::dist(lo, hi) / 2. + m.passo;
        let dir = if g.reverse { frame.normal } else { frame.normal.map(|x| -x) };
        // sem profundidade: atravessa a parede no centro da região e para no meio do vão seguinte
        let prof = match m.profundidade {
            Some(p) => p,
            None => {
                let corpo = self.body(f.target).ok_or("a grelha precisa de um corpo para vazar")?;
                let mut t: Vec<f64> = corpo.ray_hits(c, dir).map_err(k)?.into_iter().filter(|t| *t > 1e-6).collect();
                t.sort_by(|a, b| a.total_cmp(b));
                match t.as_slice() {
                    [] => return Err("a grelha não encontra a parede na direção do vazado — use 'para o lado da normal'".into()),
                    [saida] => saida + 1.,
                    [saida, volta, ..] => (saida + volta) / 2.,
                }
            }
        };
        // as barras passam um pouco das duas faces do vazado, para a booleana não ter face coincidente
        let v = geo3d::mul(dir, prof + 2.);
        let c0 = geo3d::sub(c, dir);
        let mut tool = abertura.extrude(dir, prof, 0.).map_err(k)?;
        let th = m.angulo.to_radians();
        let e = geo3d::add(geo3d::mul(frame.x, th.cos()), geo3d::mul(frame.y(), th.sin()));
        let fy = cruz(frame.normal, e);
        tool = tool.boolean(&barras(c0, e, fy, m.passo, m.nervura, meia, v)?, Bool::Cut).map_err(k)?;
        if g.cruzada {
            tool = tool.boolean(&barras(c0, fy, e, m.passo, m.nervura, meia, v)?, Bool::Cut).map_err(k)?;
        }
        self.apply(f.target, tool, BoolOp::Cut, f.id)?;
        Ok(None)
    }

    fn plastico_lip(&mut self, f: &Feature, l: &Lip) -> Result<Option<String>, String> {
        let k = |e: KernelError| e.0;
        let (frame, sk) = self.sketch(l.sketch)?;
        if sk.loops.is_empty() {
            return Err("o sketch do lip precisa do contorno externo fechado da parede".into());
        }
        let m = lip_medidas(l, &self.vars)?;
        let contorno = Shape::face(&frame, &sk.loops).map_err(k)?;
        let fora = para_dentro(&contorno, m.fora)?;
        let dentro = para_dentro(&contorno, m.dentro)?;
        // o lip sobe pela normal; a ranhura afunda contra ela
        let sobe = if l.reverse { frame.normal.map(|x| -x) } else { frame.normal };
        let dir = if l.ranhura { sobe.map(|x| -x) } else { sobe };
        let anel = fora.extrude(dir, m.altura, 0.).map_err(k)?;
        // o miolo passa 1 mm das duas pontas: a booleana não fica com face coincidente
        let miolo = dentro.extrude(dir, m.altura + 1., 1.).map_err(k)?;
        let tool = anel.boolean(&miolo, Bool::Cut).map_err(k)?;
        self.apply(f.target, tool, if l.ranhura { BoolOp::Cut } else { BoolOp::Add }, f.id)?;
        Ok(None)
    }

    fn plastico_snap(&mut self, f: &Feature, s: &Snap) -> Result<Option<String>, String> {
        let k = |e: KernelError| e.0;
        let (frame, sk) = self.sketch(s.sketch)?;
        if sk.lines.is_empty() {
            return Err("o sketch do snap-fit precisa de pelo menos uma linha (a raiz da viga)".into());
        }
        let m = snap_medidas(s, &self.vars)?;
        let n = if s.reverse { frame.normal.map(|x| -x) } else { frame.normal };
        let mut linhas: Vec<_> = sk.lines.iter().collect();
        linhas.sort_by_key(|(id, _)| **id);
        let mut tool = Shape::empty();
        for (_, (p, q)) in linhas {
            let sol = snap_solido(&m, frame.point(p[0], p[1]), frame.point(q[0], q[1]), n, s.lado)?;
            tool = if tool.is_null() { sol } else { tool.boolean(&sol, Bool::Fuse).map_err(k)? };
        }
        self.apply(f.target, tool, BoolOp::Add, f.id)?;
        let e = m.deformacao();
        Ok((e > m.adm + 1e-12).then(|| format!("snap-fit: deformação na raiz de {:.2}% passa da admissível de {:.2}% (afine ou alongue a viga, ou diminua o gancho)", e * 100., m.adm * 100.)))
    }
}
