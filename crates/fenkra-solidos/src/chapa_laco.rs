// SPDX-License-Identifier: LGPL-2.1-or-later
//! Aba com extensão lateral e laço de arestas (B284).
//!
//! - `Ctx::run_flange`: a aba de aresta (antes dentro do `match` do recompute), agora com a
//!   extensão lateral escolhida. `SmFlange` usa `ExtensaoAba::Inteira` e os recuos `off1`/`off2`.
//! - `recuos_extensao`: converte largura centrada, deslocada ou entre duas referências nos dois
//!   recuos que o kernel entende (a partir de cada ponta da aresta).
//! - `laco_de_malha`: arestas retas que formam o contorno de uma face, a partir de uma delas.
//! - `Ctx::run_flange_loop`: uma aba por aresta do laço, em um comando só, com a mesma extensão.

use super::*;
use fenkra_core::{ExtensaoAba, SmFlangeLoop};
use fenkra_kernel::EdgeFormula;

/// Ordem canônica das pontas de uma aresta: a de menor (x, y, z), com folga de 1e-6. O kernel pode
/// entregar a aresta em qualquer sentido; a medida "a partir do início" precisa de um começo que
/// o usuário consiga prever e inverter.
fn menor(a: V3, b: V3) -> bool {
    for i in 0..3 {
        if (a[i] - b[i]).abs() > 1e-6 {
            return a[i] < b[i];
        }
    }
    true
}

/// Recuos (a partir da ponta 0 e da ponta 1 do kernel) que deixam a aba só no trecho pedido.
///
/// `comp` é o comprimento da aresta e `ends` as pontas dela, como o kernel as entrega. `inverter`
/// troca a ponta de referência da medida "a partir do início". As medidas são fórmulas já
/// avaliadas por `ev` (o chamador passa o avaliador).
pub fn recuos_extensao(ext: &ExtensaoAba, comp: f64, ends: [V3; 2], inverter: bool, vars: &Vars) -> Result<(f64, f64), String> {
    if matches!(ext, ExtensaoAba::Inteira) {
        return Ok((0., 0.));
    }
    // começo canônico: a ponta que a ordem escolhe (invertida se pedido)
    let kernel_comeca_no_canonico = menor(ends[0], ends[1]) != inverter;
    let (c0, c1) = if kernel_comeca_no_canonico { (ends[0], ends[1]) } else { (ends[1], ends[0]) };
    let dir = geo3d::unit(geo3d::sub(c1, c0)).ok_or("aresta nula")?;
    let (s0, s1) = match ext {
        ExtensaoAba::Inteira => unreachable!(),
        ExtensaoAba::Centrada { largura } => {
            let w = ev(largura, vars, "largura da aba")?;
            if w <= 1e-6 {
                return Err("a largura da aba precisa ser maior que zero".into());
            }
            if w > comp + 1e-6 {
                return Err(format!("a largura da aba ({w:.2} mm) é maior que a aresta ({comp:.2} mm)"));
            }
            ((comp - w) / 2., (comp + w) / 2.)
        }
        ExtensaoAba::Deslocada { inicio, largura } => {
            let d = ev(inicio, vars, "início da aba")?;
            let w = ev(largura, vars, "largura da aba")?;
            if w <= 1e-6 {
                return Err("a largura da aba precisa ser maior que zero".into());
            }
            if d < -1e-6 || d + w > comp + 1e-6 {
                return Err(format!("a aba de {w:.2} mm a partir de {d:.2} mm passa do fim da aresta ({comp:.2} mm)"));
            }
            (d, d + w)
        }
        ExtensaoAba::Entre { a, b } => {
            // as duas referências são projetadas sobre a aresta; fora dela, ficam na ponta
            let pa = geo3d::dot(geo3d::sub(*a, c0), dir).clamp(0., comp);
            let pb = geo3d::dot(geo3d::sub(*b, c0), dir).clamp(0., comp);
            (pa.min(pb), pa.max(pb))
        }
    };
    if s1 - s0 <= 1e-6 {
        return Err("as duas referências caem no mesmo ponto da aresta: a aba ficaria sem largura".into());
    }
    // volta para as pontas do kernel
    Ok(if kernel_comeca_no_canonico { (s0, comp - s1) } else { (comp - s1, s0) })
}

/// Pontos médios das arestas retas que contornam, junto com a aresta `semente`, a face maior que
/// ela toca (a face de topo da chapa, não a lateral da espessura). Só arestas encadeadas pelas
/// pontas entram: o laço da face, e não o contorno de um furo ou de outra volta. Arestas curvas
/// ficam de fora (têm a aba curva). A ordem é a do encadeamento, começando pela semente.
pub fn laco_de_malha(mesh: &Mesh, semente: usize) -> Vec<V3> {
    let faces = |e: usize| mesh.edge_faces.get(e).copied().unwrap_or([fenkra_kernel::NO_FACE; 2]);
    let area = |f: u32| mesh.face_info.get(f as usize).map(|i| i.area).unwrap_or(0.);
    let [f0, f1] = faces(semente);
    let face = match (f0 == fenkra_kernel::NO_FACE, f1 == fenkra_kernel::NO_FACE) {
        (true, true) => return vec![],
        (false, true) => f0,
        (true, false) => f1,
        (false, false) => {
            if area(f0) >= area(f1) {
                f0
            } else {
                f1
            }
        }
    };
    // arestas retas da face, com as pontas
    let mut retas: Vec<(usize, V3, V3)> = vec![];
    for e in 0..mesh.edge_faces.len() {
        let [a, b] = faces(e);
        if a != face && b != face {
            continue;
        }
        if let Some(EdgeFormula::Line { a, b }) = mesh.edge_formula(e) {
            retas.push((e, a, b));
        }
    }
    let Some(ini) = retas.iter().position(|r| r.0 == semente) else { return vec![] };
    let perto = |p: V3, q: V3| geo3d::dist(p, q) < 1e-3;
    let mut fila = vec![ini];
    let mut usadas = vec![false; retas.len()];
    usadas[ini] = true;
    let mut i = 0;
    while i < fila.len() {
        let (_, a, b) = retas[fila[i]];
        for (j, r) in retas.iter().enumerate() {
            if !usadas[j] && (perto(r.1, a) || perto(r.1, b) || perto(r.2, a) || perto(r.2, b)) {
                usadas[j] = true;
                fila.push(j);
            }
        }
        i += 1;
    }
    fila.into_iter()
        .map(|j| {
            let (e, a, b) = retas[j];
            // o ponto médio exato da reta (a malha guarda em f32)
            let m = geo3d::lerp(a, b, 0.5);
            let _ = e;
            m
        })
        .collect()
}

impl Ctx<'_> {
    /// Aba de aresta. `ext` escolhe o trecho da aresta que recebe a aba; com `Inteira` valem os
    /// recuos `off1`/`off2` do comando.
    pub(super) fn run_flange(&mut self, f: &Feature, fl: &SmFlange, ext: &ExtensaoAba, inverter: bool) -> Result<Option<String>, String> {
        let k = |e: KernelError| e.0;
        let sp = self.sheet()?;
        let (fl_edge, from) = self.chapa_aresta("aba", f, fl.edge, fl.edge_key.as_ref(), sp.flat)?;
        let r = match &fl.radius {
            Some(e) => ev(e, &self.vars, "raio")?,
            None => sp.r,
        };
        let mut p = fenkra_kernel::SheetFlange {
            t: sp.t,
            r,
            angle: ev(&fl.angle, &self.vars, "ângulo")?,
            length: ev(&fl.length, &self.vars, "comprimento")?,
            off1: ev(&fl.off1, &self.vars, "recuo 1")?.max(if fl.min_gap { 1e-4 } else { 0. }),
            off2: ev(&fl.off2, &self.vars, "recuo 2")?.max(if fl.min_gap { 1e-4 } else { 0. }),
            flip: fl.flip,
            flat: sp.flat,
            k: sp.k_for(r),
        };
        // extensão lateral: mede a aresta inteira e converte o trecho pedido nos dois recuos
        if !matches!(ext, ExtensaoAba::Inteira) {
            let b = self.body(f.target).ok_or("a aba precisa de uma parede de chapa antes")?.clone();
            let e0 = self.map_point(fl_edge, from, p.flat);
            let (_, inteira) = b.sm_flange(e0, fenkra_kernel::SheetFlange { off1: 0., off2: 0., ..p }).map_err(k)?;
            let (o1, o2) = recuos_extensao(ext, inteira.length, inteira.ends, inverter, &self.vars)?;
            let gap = if fl.min_gap { 1e-4 } else { 0. };
            p.off1 = o1.max(gap);
            p.off2 = o2.max(gap);
        }
        // o alívio próprio da aba (cadeado) vale; sem ele, herda o do estilo da chapa
        let relief = match &fl.relief {
            Some(w) => Some(ev(w, &self.vars, "largura do alívio")?),
            None => sp.relief,
        };
        let relief_kind = if fl.relief.is_some() { fl.relief_kind } else { sp.relief_kind };
        // lado do material: recua a parede e a aba começa mais para dentro
        let trim = match fl.side {
            1 => p.t,
            2 => p.t + p.r,
            _ => 0.,
        };
        let mut edge = fl_edge;
        if trim > 1e-9 {
            let b = self.body(f.target).ok_or("a aba precisa de uma parede de chapa antes")?.clone();
            let e0 = self.map_point(fl_edge, from, p.flat);
            let (_, info) = b.sm_flange(e0, p).map_err(k)?;
            let e = geo3d::unit(geo3d::sub(info.ends[1], info.ends[0])).ok_or("aresta nula")?;
            let z = geo3d::mul(info.normal, -1.);
            let x = e;
            let y = fenkra_kernel::cross(z, x);
            // faixa ao longo da aresta, da borda para dentro (só o trecho da aba)
            let len = geo3d::dist(info.ends[0], info.ends[1]);
            let start = if geo3d::dot(y, info.out) > 0. { geo3d::sub(info.ends[0], geo3d::mul(info.out, trim)) } else { info.ends[0] };
            let strip = Shape::box_tool(geo3d::add(start, geo3d::mul(info.normal, 0.01)), x, z, [len, trim, p.t + 0.02]).map_err(k)?;
            self.apply(f.target, strip, BoolOp::Cut, f.id)?;
            edge = geo3d::sub(e0, geo3d::mul(info.out, trim));
            if p.flat != from {
                edge = self.map_point(edge, p.flat, from);
            }
        }
        self.miter_next = fl.miter;
        let out = self.sheet_tool(f, edge, from, p, true)?;
        if let (Some(w), Some(info)) = (relief, self.last_sheet) {
            // alívio de canto: corte retangular atrás da dobra em cada ponta recuada
            let depth = p.r + p.t;
            let e = geo3d::unit(geo3d::sub(info.ends[1], info.ends[0])).ok_or("aresta nula")?;
            let mut cut = Shape::empty();
            for (end, x, off) in [(info.ends[0], geo3d::mul(e, -1.), p.off1), (info.ends[1], e, p.off2)] {
                if off <= 1e-9 {
                    continue;
                }
                let z = geo3d::mul(info.normal, -1.);
                let y = fenkra_kernel::cross(z, x);
                let o = if geo3d::dot(y, info.out) > 0. { geo3d::sub(end, geo3d::mul(info.out, depth)) } else { end };
                let wr = w.min(off);
                let bx = match relief_kind {
                    // rasgo: corte fino
                    2 => Shape::box_tool(o, x, z, [wr.min(0.5), depth, p.t]).map_err(k)?,
                    // redondo: retângulo com a ponta em meio círculo (oblongo)
                    1 => {
                        let far = if geo3d::dot(y, info.out) > 0. { o } else { geo3d::add(o, geo3d::mul(info.out, -depth)) };
                        let rect = Shape::box_tool(o, x, z, [wr, depth, p.t]).map_err(k)?;
                        let top = geo3d::add(geo3d::add(far, geo3d::mul(x, wr / 2.)), geo3d::mul(info.normal, 0.5));
                        let cyl = Shape::hole_tool(top, z, wr, p.t + 1., HoleShape::Simple).map_err(k)?;
                        rect.compound(&cyl)
                    }
                    _ => Shape::box_tool(o, x, z, [wr, depth, p.t]).map_err(k)?,
                };
                cut = if cut.is_null() { bx } else { cut.compound(&bx) };
            }
            if !cut.is_null() {
                self.apply(f.target, cut, BoolOp::Cut, f.id)?;
            }
        }
        Ok(out)
    }

    /// Várias abas em um comando: uma por aresta escolhida (ou por aresta do laço da face, com
    /// `loop_all`), todas com os mesmos comprimento, ângulo, raio e extensão lateral.
    pub(super) fn run_flange_loop(&mut self, f: &Feature, lp: &SmFlangeLoop) -> Result<Option<String>, String> {
        let k = |e: KernelError| e.0;
        let sp = self.sheet()?;
        if lp.edges.is_empty() {
            return Err("escolha pelo menos uma aresta da chapa".into());
        }
        // arestas pela chave (dono + papel das faces) quando o comando está no estado em que foram
        // escolhidas; a chave de cada uma é gravada de volta pelo app (promoção)
        let mut pontos = lp.edges.clone();
        if f.flat_space == sp.flat {
            if let Some((b, owners, roles)) = self.body_owned(f.target) {
                let keys: Vec<Option<EdgeKey>> = (0..pontos.len()).map(|i| lp.edge_keys.get(i).cloned().flatten()).collect();
                pontos = self.resolve_edges("aba por laço", f, b, owners, roles, &lp.edges, &keys)?;
            }
        }
        let promovido = self.promoted.borrow().clone();
        if lp.loop_all {
            let b = self.body(f.target).ok_or("a aba precisa de uma parede de chapa antes")?.clone();
            let m = tesselar(&b, &self.qualidade).map_err(k)?;
            let semente = (0..m.edge_faces.len())
                .filter(|e| m.edge_formula(*e).is_some())
                .min_by(|a, c| {
                    let d = |e: usize| geo3d::dist([m.edge_mids[e * 3] as f64, m.edge_mids[e * 3 + 1] as f64, m.edge_mids[e * 3 + 2] as f64], pontos[0]);
                    d(*a).total_cmp(&d(*c))
                })
                .ok_or("a chapa não tem arestas")?;
            let laco = laco_de_malha(&m, semente);
            if laco.is_empty() {
                return Err("a aresta escolhida não tem laço reto de arestas para criar abas".into());
            }
            pontos = laco;
        }
        for (i, p) in pontos.iter().enumerate() {
            let fl = SmFlange {
                edge: *p,
                length: lp.length.clone(),
                angle: lp.angle.clone(),
                radius: lp.radius.clone(),
                off1: "0".into(),
                off2: "0".into(),
                flip: lp.flip,
                relief: lp.relief.clone(),
                relief_kind: lp.relief_kind,
                side: lp.side,
                miter: lp.miter,
                min_gap: true,
                edge_key: None,
            };
            self.run_flange(f, &fl, &lp.extent, lp.invert).map_err(|e| if pontos.len() > 1 { format!("aba {} de {}: {e}", i + 1, pontos.len()) } else { e })?;
        }
        // cada aba resolve a própria aresta e sobrescreve a promoção; vale a do conjunto
        *self.promoted.borrow_mut() = promovido;
        Ok(None)
    }
}
