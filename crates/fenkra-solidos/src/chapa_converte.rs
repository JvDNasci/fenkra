// SPDX-License-Identifier: LGPL-2.1-or-later
//! Converter sólido em chapa (B35): reconhece a espessura constante e as dobras de um corpo.
//!
//! Um corpo de chapa é uma casca de espessura `t` constante. O reconhecimento usa só a topologia
//! da malha (tipo, normal, área e ponto de cada face):
//!
//! 1. a espessura é a menor distância entre dois planos paralelos "de costas" um para o outro (o
//!    material fica entre eles);
//! 2. as faces principais são as planas que têm esse par e os cilindros coaxiais cujos raios
//!    diferem de `t` (o raio interno e o externo de uma dobra);
//! 3. a prova de espessura constante é o volume: com `t` constante, o volume do corpo é a metade
//!    da área das faces principais vezes `t`. Um bloco, por exemplo, não passa.
//!
//! O ângulo de cada dobra sai da área da face cilíndrica interna: `área = raio × largura × ângulo`.

use super::*;
use fenkra_kernel::Mesh;

/// Uma dobra reconhecida: raio interno, ângulo (graus) e largura ao longo do eixo.
#[derive(Clone, Debug, PartialEq)]
pub struct DobraReconhecida {
    pub raio: f64,
    pub angulo: f64,
    pub largura: f64,
}

/// O que se reconheceu do corpo.
#[derive(Clone, Debug)]
pub struct AnaliseChapa {
    pub espessura: f64,
    pub dobras: Vec<DobraReconhecida>,
    /// índices (na ordem das faces da malha) das faces planas principais
    pub planas: Vec<usize>,
}

impl AnaliseChapa {
    /// O menor raio interno das dobras (o que a chapa precisa suportar); `None` sem dobras.
    pub fn raio_minimo(&self) -> Option<f64> {
        self.dobras.iter().map(|d| d.raio).fold(None, |m, r| Some(m.map_or(r, |x: f64| x.min(r))))
    }

    /// Texto curto do que foi reconhecido, para o aviso do comando.
    pub fn resumo(&self) -> String {
        let n = self.dobras.len();
        if n == 0 {
            return format!("chapa reconhecida: espessura {:.2} mm, sem dobras (chapa plana)", self.espessura);
        }
        let lista: Vec<String> = self.dobras.iter().map(|d| format!("{:.0}° r{:.2}", d.angulo, d.raio)).collect();
        format!("chapa reconhecida: espessura {:.2} mm, {} dobra(s) ({})", self.espessura, n, lista.join(", "))
    }
}

fn pontos_da_face(m: &Mesh, f: usize) -> Vec<V3> {
    let ini = m.face_ranges.get(f).copied().unwrap_or(0) as usize;
    let fim = m.face_ranges.get(f + 1).copied().map(|x| x as usize).unwrap_or(m.indices.len());
    let mut v: Vec<V3> = vec![];
    for &i in &m.indices[ini.min(m.indices.len())..fim.min(m.indices.len())] {
        let i = i as usize * 3;
        if i + 2 < m.positions.len() {
            v.push([m.positions[i] as f64, m.positions[i + 1] as f64, m.positions[i + 2] as f64]);
        }
    }
    v
}

/// Centro (no plano da seção, na altura do primeiro ponto) e raio da circunferência que passa
/// por três pontos da face cilíndrica, projetados no plano perpendicular ao eixo.
fn circulo(pts: &[V3], eixo: V3) -> Option<(V3, f64)> {
    let p0 = *pts.first()?;
    let u = geo3d::unit(geo3d::any_perp(eixo))?;
    let w = geo3d::unit(fenkra_kernel::cross(eixo, u))?;
    let plano = |p: V3| -> [f64; 2] {
        let d = geo3d::sub(p, p0);
        [geo3d::dot(d, u), geo3d::dot(d, w)]
    };
    let q: Vec<[f64; 2]> = pts.iter().map(|p| plano(*p)).collect();
    let dist = |a: [f64; 2], b: [f64; 2]| (a[0] - b[0]).hypot(a[1] - b[1]);
    let a = q[0];
    let b = *q.iter().max_by(|x, y| dist(a, **x).total_cmp(&dist(a, **y)))?;
    // terceiro ponto: o mais longe da corda a-b
    let ab = [b[0] - a[0], b[1] - a[1]];
    let n = ab[0].hypot(ab[1]);
    if n < 1e-9 {
        return None;
    }
    let c = *q.iter().max_by(|x, y| {
        let f = |p: &[f64; 2]| ((p[0] - a[0]) * ab[1] - (p[1] - a[1]) * ab[0]).abs() / n;
        f(x).total_cmp(&f(y))
    })?;
    let d = 2. * (a[0] * (b[1] - c[1]) + b[0] * (c[1] - a[1]) + c[0] * (a[1] - b[1]));
    if d.abs() < 1e-9 {
        return None;
    }
    let (a2, b2, c2) = (a[0] * a[0] + a[1] * a[1], b[0] * b[0] + b[1] * b[1], c[0] * c[0] + c[1] * c[1]);
    let cx = (a2 * (b[1] - c[1]) + b2 * (c[1] - a[1]) + c2 * (a[1] - b[1])) / d;
    let cy = (a2 * (c[0] - b[0]) + b2 * (a[0] - c[0]) + c2 * (b[0] - a[0])) / d;
    let r = (a[0] - cx).hypot(a[1] - cy);
    let centro = geo3d::add(p0, geo3d::add(geo3d::mul(u, cx), geo3d::mul(w, cy)));
    Some((centro, r))
}

struct Cil {
    eixo: V3,
    centro: V3,
    raio: f64,
    area: f64,
    largura: f64,
}

/// Reconhece a chapa na malha de um corpo de `volume` mm³. Erra com o motivo quando o corpo não
/// tem espessura constante.
pub fn analisar_malha(m: &Mesh, volume: f64) -> Result<AnaliseChapa, String> {
    let info = &m.face_info;
    if info.is_empty() || volume <= 0. {
        return Err("o corpo está vazio".into());
    }
    let planas: Vec<usize> = (0..info.len()).filter(|i| info[*i].kind == 0).collect();
    // dois planos paralelos só formam espessura se as faces se enfrentam: as projeções delas no
    // plano se sobrepõem (a lateral da peça não enfrenta a face da aba vizinha)
    let pts: Vec<Vec<V3>> = planas.iter().map(|i| pontos_da_face(m, *i)).collect();
    let extensao = |k: usize, u: V3, w: V3| -> Option<([f64; 2], [f64; 2])> {
        let p = &pts[k];
        if p.is_empty() {
            return None;
        }
        let f = |e: V3| p.iter().fold((f64::MAX, f64::MIN), |(lo, hi), q| (lo.min(geo3d::dot(*q, e)), hi.max(geo3d::dot(*q, e))));
        let (a, b) = (f(u), f(w));
        Some(([a.0, a.1], [b.0, b.1]))
    };
    let se_enfrentam = |a: usize, b: usize| -> bool {
        let n = info[planas[a]].dir;
        let Some(u) = geo3d::unit(geo3d::any_perp(n)) else { return false };
        let Some(w) = geo3d::unit(fenkra_kernel::cross(n, u)) else { return false };
        let (Some(ea), Some(eb)) = (extensao(a, u, w), extensao(b, u, w)) else { return false };
        let sobra = |x: [f64; 2], y: [f64; 2]| x[1].min(y[1]) - x[0].max(y[0]);
        sobra(ea.0, eb.0) > 1e-6 && sobra(ea.1, eb.1) > 1e-6
    };
    // distância entre os planos de duas faces planas paralelas que se enfrentam
    let espaco = |a: usize, b: usize| -> Option<f64> {
        if geo3d::dot(info[planas[a]].dir, info[planas[b]].dir).abs() < 0.9999 || !se_enfrentam(a, b) {
            return None;
        }
        let d = geo3d::dot(geo3d::sub(info[planas[b]].point, info[planas[a]].point), info[planas[a]].dir).abs();
        (d > 1e-6).then_some(d)
    };
    // espessura: a menor distância entre faces planas paralelas que se enfrentam
    let mut t = f64::MAX;
    for a in 0..planas.len() {
        for b in (a + 1)..planas.len() {
            if let Some(d) = espaco(a, b) {
                t = t.min(d);
            }
        }
    }
    if t == f64::MAX {
        return Err("o corpo não tem faces planas paralelas que se enfrentam: não é uma chapa".into());
    }
    let perto = |a: f64, b: f64| (a - b).abs() <= 0.02 * b.max(1e-6);
    // faces planas principais: têm uma face na frente à distância da espessura
    let mut principais: Vec<usize> = vec![];
    for a in 0..planas.len() {
        if (0..planas.len()).any(|b| b != a && espaco(a, b).is_some_and(|d| perto(d, t))) {
            principais.push(planas[a]);
        }
    }
    // cilindros ajustados (raio e eixo)
    let mut cils: Vec<(usize, Cil)> = vec![];
    for i in (0..info.len()).filter(|i| info[*i].kind == 1) {
        let pts = pontos_da_face(m, i);
        let Some(eixo) = geo3d::unit(info[i].dir) else { continue };
        let Some((centro, raio)) = circulo(&pts, eixo) else { continue };
        let (mut lo, mut hi) = (f64::MAX, f64::MIN);
        for p in &pts {
            let s = geo3d::dot(*p, eixo);
            lo = lo.min(s);
            hi = hi.max(s);
        }
        cils.push((i, Cil { eixo, centro, raio, area: info[i].area, largura: hi - lo }));
    }
    // dobras: pares coaxiais com raios que diferem de t
    let mut dobras: Vec<DobraReconhecida> = vec![];
    let mut area_cil = 0.;
    let mut usados = vec![false; cils.len()];
    for a in 0..cils.len() {
        for b in 0..cils.len() {
            if a == b || usados[a] || usados[b] || cils[a].1.raio >= cils[b].1.raio {
                continue;
            }
            let (ci, ce) = (&cils[a].1, &cils[b].1);
            let d = geo3d::sub(ce.centro, ci.centro);
            let fora_do_eixo = geo3d::sub(d, geo3d::mul(ci.eixo, geo3d::dot(d, ci.eixo)));
            let coaxial = geo3d::dot(ci.eixo, ce.eixo).abs() > 0.9999 && geo3d::dot(fora_do_eixo, fora_do_eixo).sqrt() < 0.01 * ce.raio.max(1.);
            if coaxial && perto(ce.raio - ci.raio, t) && perto(ce.largura, ci.largura) {
                usados[a] = true;
                usados[b] = true;
                area_cil += ci.area + ce.area;
                dobras.push(DobraReconhecida { raio: ci.raio, angulo: (ci.area / (ci.raio * ci.largura.max(1e-9))).to_degrees(), largura: ci.largura });
            }
        }
    }
    // prova da espessura constante: volume = metade da área das faces principais × t
    let area_planas: f64 = principais.iter().map(|i| info[*i].area).sum();
    let esperado = (area_planas + area_cil) / 2. * t;
    if (esperado - volume).abs() > 0.1 * volume {
        return Err(format!("a espessura não é constante: com {t:.2} mm de espessura o corpo teria {esperado:.0} mm³, e ele tem {volume:.0} mm³"));
    }
    dobras.sort_by(|a, b| a.raio.total_cmp(&b.raio).then(a.angulo.total_cmp(&b.angulo)));
    Ok(AnaliseChapa { espessura: t, dobras, planas: principais })
}

impl Ctx<'_> {
    /// Confere que o corpo é chapa (espessura constante, face fixa plana e principal) e avisa o que
    /// reconheceu. O corpo não muda: os comandos de chapa seguintes passam a valer sobre ele.
    pub(super) fn run_convert(&mut self, f: &Feature, c: &fenkra_core::SmConvert) -> Result<Option<String>, String> {
        let k = |e: KernelError| e.0;
        let corpo = self.body(f.target).ok_or("o corpo a converter precisa existir antes deste comando")?.clone();
        let malha = tesselar(&corpo, &self.qualidade).map_err(k)?;
        let a = analisar_malha(&malha, corpo.mass().volume)?;
        if !c.thickness.trim().is_empty() {
            let pedida = ev(&c.thickness, &self.vars, "espessura")?;
            if (pedida - a.espessura).abs() > 0.02 * pedida.max(1e-6) {
                return Err(format!("o corpo tem {:.2} mm de espessura, e não os {pedida:.2} mm pedidos", a.espessura));
            }
        }
        if c.face != [0.; 3] {
            let n = a.planas.iter().any(|i| {
                let fi = &malha.face_info[*i];
                geo3d::dot(geo3d::sub(c.face, fi.point), fi.dir).abs() < 1e-3 * a.espessura.max(1.)
            });
            if !n {
                return Err("a face fixa precisa ser uma face plana da chapa (não a lateral da espessura)".into());
            }
        }
        let aviso = a.resumo();
        // o parâmetro de espessura da chapa deve combinar com o corpo
        if let Some(sp) = self.doc.sheet_efetivo() {
            let t = ev(&sp.thickness, &self.vars, "espessura da chapa")?;
            if (t - a.espessura).abs() > 0.02 * a.espessura {
                return Ok(Some(format!("{aviso}; a chapa dos parâmetros tem {t:.2} mm: use a espessura medida em Chapa › Parâmetros")));
            }
        }
        Ok(Some(aviso))
    }
}

/// Analisa o corpo de uma peça até antes do comando `ate` (que pode ser o próprio converter). Serve
/// ao painel, que mostra a espessura e as dobras e aplica à chapa do documento.
pub fn analisar_corpo(doc: &Document, ate: Id) -> Result<AnaliseChapa, String> {
    let mut d = doc.clone();
    let pos = d.features.iter().position(|f| f.id == ate).ok_or("comando não encontrado")?;
    d.features.truncate(pos);
    let r = recompute(&d);
    let b = r.bodies.first().ok_or("não há corpo para converter antes deste comando")?;
    analisar_malha(&b.mesh, b.mass.volume)
}
