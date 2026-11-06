// SPDX-License-Identifier: LGPL-2.1-or-later
//! Fechamento de canto entre duas abas de 90° (B293).
//!
//! Duas abas vizinhas, cada uma saindo de uma aresta da chapa, deixam no canto um vão: a parede de
//! uma termina antes de encontrar a da outra (o raio de dobra e a espessura ficam no meio). Este
//! módulo calcula, com geometria simples de plano, as peças que fecham esse vão:
//!
//! - **topo (0)**: as duas pontas avançam até uma diagonal e param separadas pela folga;
//! - **sobreposto (1)**: a primeira aba avança e cobre a ponta da segunda; a segunda para na folga;
//! - **sob (2)**: o contrário, a segunda cobre a primeira.
//!
//! A razão de sobreposição (0 a 1) diz quanto da espessura da aba coberta a de cima cobre; o canto
//! estreito limita essa cobertura à metade. Tudo é medido num referencial do canto: `x` ao longo
//! da saída da primeira aba, `y` ao longo da saída da segunda e `z` na normal da chapa, a partir
//! da face de referência das abas. Na chapa planificada as mesmas peças viram retângulos da
//! espessura da chapa, e o fechamento não sobrepõe material (verificado antes de gravar).

use super::*;
use fenkra_core::SmCornerClose;

/// Distância máxima do fim de uma aba ao canto (mm) para a aba contar como vizinha dele.
const FIM_MAX: f64 = 25.;
/// Tolerância geométrica dos alinhamentos (mm).
const TOL: f64 = 0.02;

/// Uma aba de 90° vista do canto, no espaço dobrado.
#[derive(Clone, Copy, Debug)]
pub struct AbaCanto {
    /// pontas da aresta da aba, na face de referência
    pub c0: V3,
    pub ev: V3,
    pub l: f64,
    /// normal da chapa (a aba sobe por ela) e saída da aba, a partir da aresta
    pub n: V3,
    pub u: V3,
    pub t: f64,
    pub r: f64,
    /// comprimento desenvolvido da dobra
    pub ba: f64,
    /// parte reta da aba, depois da dobra
    pub comp: f64,
}

/// O canto entre duas abas já ordenadas: `a` é a primeira, `b` a segunda.
#[derive(Clone, Copy, Debug)]
pub struct Canto {
    pub k: V3,
    pub a: AbaCanto,
    pub b: AbaCanto,
    /// vão entre o fim de cada aba e o canto (recuo)
    pub off_a: f64,
    pub off_b: f64,
}

/// Parâmetros do fechamento, já avaliados.
#[derive(Clone, Copy, Debug)]
pub struct ParamsCanto {
    pub kind: u8,
    pub gap: f64,
    pub ratio: f64,
    pub narrow: bool,
}

/// Peça que fecha o canto: polígono em (x, y) do canto, a altura `z0` e a espessura `dz` da peça.
#[derive(Clone, Debug)]
pub struct Peca {
    pub pts: Vec<[f64; 2]>,
    pub z0: f64,
    pub dz: f64,
}

#[cfg(test)]
impl Peca {
    pub fn area(&self) -> f64 {
        let n = self.pts.len();
        (0..n).map(|i| self.pts[i][0] * self.pts[(i + 1) % n][1] - self.pts[(i + 1) % n][0] * self.pts[i][1]).sum::<f64>().abs() / 2.
    }
    pub fn volume(&self) -> f64 {
        self.area() * self.dz
    }
}

/// Menor distância entre dois polígonos no plano (zero se um toca ou entra no outro).
#[cfg(test)]
pub fn distancia_pecas(a: &Peca, b: &Peca) -> f64 {
    let seg = |p: &Peca| -> Vec<([f64; 2], [f64; 2])> { (0..p.pts.len()).map(|i| (p.pts[i], p.pts[(i + 1) % p.pts.len()])).collect() };
    let ponto_seg = |p: [f64; 2], (s, e): ([f64; 2], [f64; 2])| -> f64 {
        let d = [e[0] - s[0], e[1] - s[1]];
        let q = d[0] * d[0] + d[1] * d[1];
        let t = if q < 1e-18 { 0. } else { (((p[0] - s[0]) * d[0] + (p[1] - s[1]) * d[1]) / q).clamp(0., 1.) };
        ((s[0] + t * d[0] - p[0]).powi(2) + (s[1] + t * d[1] - p[1]).powi(2)).sqrt()
    };
    let mut m = f64::MAX;
    for sa in seg(a) {
        for sb in seg(b) {
            m = m.min(ponto_seg(sa.0, sb)).min(ponto_seg(sa.1, sb)).min(ponto_seg(sb.0, sa)).min(ponto_seg(sb.1, sa));
        }
    }
    m
}

fn aba_de_mapa(m: &SheetMap) -> Option<AbaCanto> {
    // só abas de 90° que sobem pela normal, do lado de fora (não a dobra para baixo)
    if m.down || (m.a.abs() - std::f64::consts::FRAC_PI_2).abs() > 1e-3 {
        return None;
    }
    Some(AbaCanto { c0: m.c0, ev: m.ev, l: m.l, n: m.n, u: m.u, t: m.t, r: geo3d::dist(m.o, m.c0), ba: m.ba, comp: m.length })
}

/// O canto entre duas abas, sem ordem: onde as retas das arestas se cruzam e quanto falta para
/// cada aba chegar lá. Erra com a razão em palavras quando as abas não formam um canto de 90°.
pub fn canto_entre(a: AbaCanto, b: AbaCanto) -> Result<Canto, String> {
    if geo3d::dot(a.n, b.n) < 0.999 {
        return Err("as duas abas precisam sair do mesmo lado da chapa".into());
    }
    if geo3d::dot(geo3d::sub(b.c0, a.c0), a.n).abs() > TOL {
        return Err("as duas abas precisam sair do mesmo plano da chapa".into());
    }
    if geo3d::dot(a.u, b.u).abs() > 1e-3 {
        return Err("as abas não formam um canto de 90°".into());
    }
    let den = geo3d::dot(a.ev, b.u);
    if den.abs() < 0.999 || geo3d::dot(b.ev, a.u).abs() < 0.999 {
        return Err("as abas não formam um canto de 90°".into());
    }
    let sk = geo3d::dot(geo3d::sub(b.c0, a.c0), b.u) / den;
    let k = geo3d::add(a.c0, geo3d::mul(a.ev, sk));
    // fim da aba A em direção ao canto e vão que sobra
    let fim = |ab: &AbaCanto, sk: f64, para: V3| -> Result<f64, String> {
        let (off, dir) = if sk >= ab.l - 1e-6 {
            (sk - ab.l, ab.ev)
        } else if sk <= 1e-6 {
            (-sk, geo3d::mul(ab.ev, -1.))
        } else {
            return Err("o canto cai no meio de uma aba, não no fim dela".into());
        };
        if geo3d::dot(dir, para) < 0.999 {
            return Err("a aba fica do lado de fora do canto".into());
        }
        Ok(off.max(0.))
    };
    let off_a = fim(&a, sk, b.u)?;
    let sk_b = geo3d::dot(geo3d::sub(a.c0, b.c0), a.u) / geo3d::dot(b.ev, a.u);
    let off_b = fim(&b, sk_b, a.u)?;
    if off_a > FIM_MAX || off_b > FIM_MAX {
        return Err(format!("as abas terminam longe do canto (mais de {FIM_MAX:.0} mm)"));
    }
    Ok(Canto { k, a, b, off_a, off_b })
}

/// Coloca as duas abas na ordem do comando: a primeira é a que chega ao canto andando com o
/// interior da chapa à esquerda (anti-horário visto de cima). `invert` troca.
pub fn ordenar(c: Canto, invert: bool) -> Canto {
    let a_primeiro = geo3d::dot(fenkra_kernel::cross(c.a.u, c.b.u), c.a.n) > 0.;
    if a_primeiro != invert {
        c
    } else {
        Canto { k: c.k, a: c.b, b: c.a, off_a: c.off_b, off_b: c.off_a }
    }
}

/// As peças que fecham o canto. `solape` entra na aba para a união não depender de a face coincidir
/// ao milésimo (0 para medir ou conferir colisão).
pub fn pecas(c: &Canto, p: &ParamsCanto, flat: bool, solape: f64) -> Result<Vec<Peca>, String> {
    if p.gap < 0. {
        return Err("a folga do canto não pode ser negativa".into());
    }
    if !(0. ..=1.).contains(&p.ratio) {
        return Err("a razão de sobreposição fica entre 0 e 1".into());
    }
    let (a, b) = (c.a, c.b);
    let g = p.gap;
    let (xa0, xa1) = (a.r, a.r + a.t);
    let (yb0, yb1) = (b.r, b.r + b.t);
    let ratio = if p.narrow { p.ratio.min(0.5) } else { p.ratio };
    // fim da peça de A (em y) e da peça de B (em x), na base e no topo da espessura
    let (ya, xb): ([f64; 2], [f64; 2]) = match p.kind {
        1 => {
            let y = yb0 - g + ratio * (yb1 - yb0 + g);
            ([y, y], [xa0 - g, xa0 - g])
        }
        2 => {
            let x = xa0 - g + ratio * (xa1 - xa0 + g);
            ([yb0 - g, yb0 - g], [x, x])
        }
        _ => {
            // diagonal entre (xa0, yb0) e (xa1, yb1), afastada g/2 para cada lado
            let (dx, dy) = (xa1 - xa0, yb1 - yb0);
            let h = dx.hypot(dy);
            if h < 1e-9 {
                return Err("espessura nula".into());
            }
            let (nx, ny) = (dy / h, -dx / h);
            let ya = [yb0 + (g / 2.) / ny, yb0 + (g / 2. - nx * dx) / ny];
            let xb = [xa0 + (-g / 2.) / nx, xa0 + (-g / 2. - ny * dy) / nx];
            (ya, xb)
        }
    };
    let mut out = vec![];
    if flat {
        // planificada: retângulos da espessura da chapa, a partir do fim da dobra; o fim do corte
        // inclinado é o da meia espessura
        let ym = (ya[0] + ya[1]) / 2.;
        let xm = (xb[0] + xb[1]) / 2.;
        if ym > -c.off_a + 1e-6 {
            let (x0, x1) = (a.ba, a.ba + a.comp);
            out.push(Peca { pts: vec![[x0, -c.off_a - solape], [x1, -c.off_a - solape], [x1, ym], [x0, ym]], z0: -a.t, dz: a.t });
        }
        if xm > -c.off_b + 1e-6 {
            let (y0, y1) = (b.ba, b.ba + b.comp);
            out.push(Peca { pts: vec![[-c.off_b - solape, y0], [xm, y0], [xm, y1], [-c.off_b - solape, y1]], z0: -b.t, dz: b.t });
        }
    } else {
        if ya[0].min(ya[1]) > -c.off_a + 1e-6 {
            out.push(Peca { pts: vec![[xa0, -c.off_a - solape], [xa1, -c.off_a - solape], [xa1, ya[1]], [xa0, ya[0]]], z0: a.r, dz: a.comp });
        }
        if xb[0].min(xb[1]) > -c.off_b + 1e-6 {
            out.push(Peca { pts: vec![[-c.off_b - solape, yb0], [xb[0], yb0], [xb[1], yb1], [-c.off_b - solape, yb1]], z0: b.r, dz: b.comp });
        }
    }
    if out.is_empty() {
        return Err("nada a fechar: a folga e a razão deixam as duas abas sem avançar sobre o canto".into());
    }
    Ok(out)
}

fn ponto(c: &Canto, x: f64, y: f64, z: f64) -> V3 {
    geo3d::add(geo3d::add(geo3d::add(c.k, geo3d::mul(c.a.u, x)), geo3d::mul(c.b.u, y)), geo3d::mul(c.a.n, z))
}

/// A peça como sólido.
pub fn solido(c: &Canto, p: &Peca) -> Result<Shape, String> {
    let pts: Vec<V3> = p.pts.iter().map(|q| ponto(c, q[0], q[1], p.z0)).collect();
    Shape::poly_prism(&pts, geo3d::mul(c.a.n, p.dz)).map_err(|e| e.0)
}

/// Todos os cantos de 90° entre abas, com o ponto do canto, na ordem das abas.
pub fn cantos_das_abas(abas: &[AbaCanto]) -> Vec<Canto> {
    let mut v = vec![];
    for i in 0..abas.len() {
        for j in (i + 1)..abas.len() {
            if let Ok(c) = canto_entre(abas[i], abas[j]) {
                v.push(c);
            }
        }
    }
    v
}

impl Ctx<'_> {
    /// Fecha o canto escolhido (ou todos, com propagação) entre abas de 90°.
    pub(super) fn run_corner_close(&mut self, f: &Feature, cc: &SmCornerClose) -> Result<Option<String>, String> {
        let k = |e: KernelError| e.0;
        let sp = self.sheet()?;
        let p = ParamsCanto { kind: cc.kind, gap: ev(&cc.gap, &self.vars, "folga")?, ratio: ev(&cc.ratio, &self.vars, "razão de sobreposição")?, narrow: cc.narrow };
        let abas: Vec<AbaCanto> = self.sheet_maps.iter().filter_map(aba_de_mapa).collect();
        if abas.len() < 2 {
            return Err("o fechamento de canto precisa de duas abas de 90° antes dele".into());
        }
        let cantos = cantos_das_abas(&abas);
        if cantos.is_empty() {
            return Err("nenhum par de abas de 90° forma um canto: confira se estão na mesma chapa e terminam perto do vértice".into());
        }
        // o ponto escolhido está num estado (dobrado ou planificado); os cantos ficam no dobrado
        let at = self.map_point(cc.at, f.flat_space, false);
        let perto = |c: &Canto| {
            let d = geo3d::sub(at, c.k);
            let no_plano = geo3d::sub(d, geo3d::mul(c.a.n, geo3d::dot(d, c.a.n)));
            geo3d::dot(no_plano, no_plano).sqrt()
        };
        let escolhido = cantos.iter().min_by(|a, b| perto(a).total_cmp(&perto(b))).copied().ok_or("sem cantos")?;
        let tol = 3. * escolhido.a.t.max(escolhido.b.t) + 1.;
        if perto(&escolhido) > tol && !cc.propagate {
            return Err(format!("nenhum canto entre abas perto do ponto escolhido (o mais perto fica a {:.1} mm)", perto(&escolhido)));
        }
        let alvo: Vec<Canto> = if cc.propagate { cantos } else { vec![escolhido] };
        let corpo = self.body(f.target).ok_or("o fechamento de canto precisa de uma chapa")?.clone();
        for (i, c) in alvo.iter().enumerate() {
            let c = ordenar(*c, cc.invert);
            let onde = |e: String| if alvo.len() > 1 { format!("canto {} de {}: {e}", i + 1, alvo.len()) } else { e };
            // a conferência usa as peças sem solape: tocar a parede é normal, entrar nela não
            let exatas = pecas(&c, &p, sp.flat, 0.).map_err(onde)?;
            let mut tool = Shape::empty();
            for (peca, exata) in pecas(&c, &p, sp.flat, 0.01).map_err(onde)?.iter().zip(exatas.iter()) {
                let sol = solido(&c, exata).map_err(onde)?;
                let comum = sol.boolean(&corpo, Bool::Common).map_err(k)?;
                if !comum.is_null() && comum.mass().volume > 0.05 {
                    return Err(onde(if sp.flat {
                        "o fechamento sobreporia material na chapa planificada: aumente a folga ou reduza a razão".into()
                    } else {
                        "o fechamento bate em outra parte da chapa: aumente a folga ou reduza a razão".into()
                    }));
                }
                let sol = solido(&c, peca).map_err(onde)?;
                tool = if tool.is_null() { sol } else { tool.compound(&sol) };
            }
            self.apply(f.target, tool, BoolOp::Add, f.id)?;
        }
        Ok(None)
    }
}
