// SPDX-License-Identifier: LGPL-2.1-or-later
//! Subconjuntos, montagem geral, juntas, simulação, interferência e lista de peças.
use crate::apoio::*;
use fenkra_core::{Id, LinkedGeo};
use fenkra_montagem::bom::{bom, BomLevel};
use fenkra_montagem::interferencia::{analisar, OpcoesInterf};
use fenkra_montagem::kin::{simulate, CouplingKind, JFrame, JRef, JointKind};
use fenkra_montagem::load::PartCache;
use fenkra_montagem::solve::{dof, solve};
use fenkra_montagem::{Assembly, Geom, MateKind, MateRef, NodeKind, Placement, V3};
use std::path::Path;

fn pl(o: V3) -> Placement {
    Placement { origin: o, ..Default::default() }
}

/// Giro de 180° em Z: peça do lado direito.
fn pl180(o: V3) -> Placement {
    Placement { origin: o, rot: [[-1., 0., 0.], [0., -1., 0.], [0., 0., 1.]] }
}

fn roty(o: V3, graus: f64) -> Placement {
    let (s, c) = graus.to_radians().sin_cos();
    Placement { origin: o, rot: [[c, 0., -s], [0., 1., 0.], [s, 0., c]] }
}

fn peca(a: &mut Assembly, cod: &str, p: Placement) -> Id {
    let n = a.nodes.iter().filter(|x| x.name.starts_with(cod)).count();
    let nome = if n == 0 { cod.to_string() } else { format!("{cod}:{}", n + 1) };
    a.add_component(&nome, NodeKind::Part { path: format!("{cod}.fkp") }, p, None)
}

fn sub(a: &mut Assembly, arq: &str) -> Id {
    a.add_component(arq, NodeKind::SubAssembly { path: format!("{arq}.fka") }, Placement::default(), None)
}

fn fixa(a: &mut Assembly, id: Id) {
    a.node_mut(id).unwrap().fixed = true;
}

fn grava(a: &Assembly, n: &str) {
    let _ = std::fs::write(modelo().join(format!("{n}.fka")), a.to_json());
}

fn valor(u: &[(String, LinkedGeo)], n: &str) -> f64 {
    match u.iter().find(|(p, _)| p == n) {
        Some((_, LinkedGeo::Value(x))) => *x,
        _ => panic!("sem {n}"),
    }
}

fn eixo(comp: Id, o: V3, z: V3) -> JRef {
    JRef { comp, frame: JFrame::from_axis(o, z) }
}

fn dist(a: V3, b: V3) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

fn ang_xy(p: &Placement) -> f64 {
    // giro em torno do eixo próprio: pela direção do X local no plano do mundo
    p.rot[0][1].atan2(p.rot[0][0]).to_degrees()
}

pub fn sc_quadro(u: &[(String, LinkedGeo)]) -> Assembly {
    let ee = valor(u, "EE");
    let (lq, zq) = (valor(u, "LARG_Q"), valor(u, "Z_Q"));
    let mut a = Assembly::new("SC-QUADRO");
    let q = peca(&mut a, "QUADRO", pl([0.; 3]));
    fixa(&mut a, q);
    for y in [lq / 2., 0., -lq / 2.] {
        peca(&mut a, "CH-08", pl([0., y, zq + 30.]));
    }
    for s in [1., -1.] {
        peca(&mut a, "CH-09", pl([ee - 40., s * valor(u, "PM") / 2., valor(u, "Z_EIXO_D")]));
    }
    for x in [0., 1., 2., 3.] {
        peca(&mut a, "CH-10", pl([x * 40. - 60., 0., 0.]));
    }
    peca(&mut a, "CH-11", pl([170., -260., zq + 20.]));
    peca(&mut a, "CH-12", pl([ee - 220., 0., 130.]));
    for s in [1., -1.] {
        peca(&mut a, "CH-13", pl([ee + 100., s * 60., zq + 20.]));
    }
    peca(&mut a, "CH-14", pl([60., 190., zq + 30.]));
    a
}

pub fn sc_eixo(u: &[(String, LinkedGeo)]) -> Assembly {
    let (zt, bt, l) = (valor(u, "Z_EIXO_T"), valor(u, "BT"), valor(u, "L_EIXO"));
    let mut a = Assembly::new("SC-EIXO");
    let e = peca(&mut a, "EX-01", pl([0., 0., zt]));
    let mut mancais = vec![];
    for y in [300., 0., -300.] {
        mancais.push(peca(&mut a, "EX-02", pl([0., y, zt])));
        peca(&mut a, "EX-03", pl([0., y, zt]));
    }
    fixa(&mut a, mancais[0]);
    let mut grupo = vec![e];
    grupo.push(peca(&mut a, "EX-05", pl([0., l / 2. - 72., zt])));
    grupo.push(peca(&mut a, "EX-05", pl180([0., -l / 2. + 72., zt])));
    grupo.push(peca(&mut a, "EX-07", pl([0., 150., zt])));
    grupo.push(peca(&mut a, "EX-08", pl([0., 191., zt])));
    grupo.push(peca(&mut a, "EX-06", pl180([0., -100., zt])));
    grupo.push(peca(&mut a, "TR-01", pl([0., -142.5, zt])));
    for (cod, c) in [("RD-02", 1.), ("RD-04", 1.), ("RD-02", -1.), ("RD-04", -1.)] {
        let p = if c > 0. { pl([0., bt / 2., zt]) } else { pl180([0., -bt / 2., zt]) };
        grupo.push(peca(&mut a, cod, p));
    }
    a.add_rigid(grupo);
    // restrição de verdade: eixo do tubo no eixo do mancal; a junta dá o giro
    let m = a.add_mate(MateKind::Coincident { flip: false }, MateRef { comp: mancais[0], geom: Geom::Axis { p: [0.; 3], d: [0., 1., 0.] }, pick: None, viva: None }, MateRef { comp: e, geom: Geom::Axis { p: [0.; 3], d: [0., 1., 0.] }, pick: None, viva: None });
    let _ = m;
    for i in 1..3 {
        fixa(&mut a, mancais[i]);
    }
    a
}

pub struct Direcao {
    pub a: Assembly,
    pub coluna: Id,
    pub manga_e: Id,
    pub manga_d: Id,
    pub junta: Id,
    pub pos0: Placement,
}

pub fn sc_direcao(u: &[(String, LinkedGeo)]) -> Result<Direcao, String> {
    let (ee, pm, bd, zd) = (valor(u, "EE"), valor(u, "PM"), valor(u, "BD"), valor(u, "Z_EIXO_D"));
    let caster = valor(u, "CASTER");
    let mut a = Assembly::new("SC-DIRECAO");
    let base = peca(&mut a, "QUADRO", pl([0.; 3]));
    fixa(&mut a, base);
    let pino_e: V3 = [ee, pm / 2., zd];
    let pino_d: V3 = [ee, -pm / 2., zd];
    let eixo_pino: V3 = [-caster.to_radians().sin(), 0., caster.to_radians().cos()];
    let manga_e = peca(&mut a, "DI-08", roty(pino_e, -caster));
    let manga_d = peca(&mut a, "DI-08D", roty(pino_d, -caster));
    let ponta_e = a.lugar(manga_e).unwrap().point([-90., -35., 0.]);
    let ponta_d = a.lugar(manga_d).unwrap().point([-90., 35., 0.]);
    // coluna inclinada 25°: a posição sai de o tirante caber entre a placa e o braço
    let incl = 25f64;
    let ucol: V3 = [-incl.to_radians().sin(), 0., incl.to_radians().cos()];
    let (zb, rod) = (120., pm / 2. - 60.);
    let mut xc = ee - 220.;
    // X local = Y do mundo; Y local = (-cos, 0, -sen); a placa tem o furo da coluna em (0, 10)
    let (ci, si) = (incl.to_radians().cos(), incl.to_radians().sin());
    let furo_w = |xc: f64, s: f64| -> V3 { [xc + ucol[0] * 5. + ci * 55., s * 45., zb + ucol[2] * 5. + si * 55.] };
    for _ in 0..60 {
        let e = dist(furo_w(xc, 1.), ponta_e) - rod;
        xc += e * 0.5;
    }
    println!("coluna em x={xc:.1}; folga do tirante {:.3}", dist(furo_w(xc, 1.), ponta_e) - rod);
    // X local da coluna/placa = Y do mundo; Z local = ucol; Y local = Z × X
    let (lx, lz): (V3, V3) = ([0., 1., 0.], ucol);
    let ly: V3 = [lz[1] * lx[2] - lz[2] * lx[1], lz[2] * lx[0] - lz[0] * lx[2], lz[0] * lx[1] - lz[1] * lx[0]];
    let rot = [lx, ly, lz];
    let b: V3 = [xc, 0., zb];
    let coluna = peca(&mut a, "DI-03", Placement { origin: b, rot });
    let po = [b[0] - ly[0] * 10., b[1] - ly[1] * 10., b[2] - ly[2] * 10.];
    let placa = peca(&mut a, "DI-05", Placement { origin: po, rot });
    let topo = [b[0] + ucol[0] * 560., b[1], b[2] + ucol[2] * 560.];
    let vol = peca(&mut a, "DI-01", Placement { origin: topo, rot });
    let cubo = peca(&mut a, "DI-02", Placement { origin: [topo[0] - ucol[0] * 14., topo[1], topo[2] - ucol[2] * 14.], rot });
    a.add_rigid(vec![coluna, placa, vol, cubo]);
    let junta = a.add_joint(JointKind::Revolute, eixo(base, b, ucol), eixo(coluna, [0.; 3], [0., 0., 1.]));
    a.joints.iter_mut().find(|j| j.id == junta).unwrap().drive_angle = Some("t".into());
    a.add_joint(JointKind::Revolute, eixo(base, pino_e, eixo_pino), eixo(manga_e, [0.; 3], [0., 0., 1.]));
    a.add_joint(JointKind::Revolute, eixo(base, pino_d, eixo_pino), eixo(manga_d, [0.; 3], [0., 0., 1.]));
    // tirantes: bola nas duas pontas
    for (s, ponta, manga, local) in [(1., ponta_e, manga_e, [-90., -35., 0.]), (-1., ponta_d, manga_d, [-90., 35., 0.])] {
        let f = a.lugar(placa).unwrap().point([s * 45., -45., 5.]);
        let d = [ponta[0] - f[0], ponta[1] - f[1], ponta[2] - f[2]];
        let n = dist(ponta, f);
        let y: V3 = [d[0] / n, d[1] / n, d[2] / n];
        let x0: V3 = [y[1], -y[0], 0.];
        let nx = (x0[0] * x0[0] + x0[1] * x0[1]).sqrt();
        let x: V3 = [x0[0] / nx, x0[1] / nx, 0.];
        let z: V3 = [x[1] * y[2] - x[2] * y[1], x[2] * y[0] - x[0] * y[2], x[0] * y[1] - x[1] * y[0]];
        let t = peca(&mut a, "DI-06", Placement { origin: f, rot: [x, y, z] });
        a.add_joint(JointKind::Spherical, JRef { comp: placa, frame: JFrame::from_axis([s * 45., -45., 5.], [0., 0., 1.]) }, eixo(t, [0.; 3], [0., 0., 1.]));
        a.add_joint(JointKind::Spherical, JRef { comp: manga, frame: JFrame::from_axis(local, [0., 0., 1.]) }, eixo(t, [0., n, 0.], [0., 0., 1.]));
    }
    // cubos e rodas presos às mangas
    for (s, m) in [(1., manga_e), (-1., manga_d)] {
        let ym = bd / 2. - 44.;
        let mp = a.lugar(m).unwrap();
        let y0 = [0., s * (ym - pm / 2.), 0.];
        let w0 = [0., s * (bd / 2. - pm / 2.), 0.];
        let lado = |o: V3, mp: &Placement| Placement { origin: mp.point(o), rot: if s > 0. { mp.rot } else { [[-mp.rot[0][0], -mp.rot[0][1], -mp.rot[0][2]], [-mp.rot[1][0], -mp.rot[1][1], -mp.rot[1][2]], mp.rot[2]] } };
        let c = peca(&mut a, "DI-10", lado(y0, &mp));
        let r1 = peca(&mut a, "RD-01", lado(w0, &mp));
        let r2 = peca(&mut a, "RD-03", lado(w0, &mp));
        a.add_rigid(vec![m, c, r1, r2]);
    }
    let pos0 = a.lugar(manga_e).unwrap();
    Ok(Direcao { a, coluna, manga_e, manga_d, junta, pos0 })
}

/// Virar o volante e medir as mangas.
pub fn testa_direcao(d: &mut Direcao) -> String {
    let rep = solve(&mut d.a);
    let mut s = format!("resolvida={} dof={}; ", rep.converged, dof(&d.a));
    let quadros = simulate(&mut d.a, -60., 60., 8);
    let a0 = ang_xy(&d.pos0);
    for q in &quadros {
        let e = ang_xy(&q.placements[&d.manga_e]) - a0;
        let dd = ang_xy(&q.placements[&d.manga_d]) - a0;
        s += &format!("t={:.0} ok={} E={e:.2} D={dd:.2} | ", q.t, q.ok);
    }
    s
}

pub fn sc_motor(u: &[(String, LinkedGeo)]) -> Assembly {
    let (dc, zt) = (valor(u, "DC"), valor(u, "Z_EIXO_T"));
    let (x, z) = (dc * 0.8, zt + dc * 0.6);
    let mut a = Assembly::new("SC-MOTOR");
    let m = peca(&mut a, "MO-01", pl180([x, -262.5, z - 75.]));
    fixa(&mut a, m);
    peca(&mut a, "MO-02", pl180([x, -262.5, z - 115.]));
    peca(&mut a, "TR-02", pl([x, -142.5, z]));
    peca(&mut a, "TR-03", Placement { origin: [0., -142.5, zt], rot: [[0.8, 0., 0.6], [-0.6, 0., 0.8], [0., -1., 0.]] });
    peca(&mut a, "TR-04", pl([x / 2., -142.5, zt + 60.]));
    peca(&mut a, "MO-03", roty([x + 120., -420., z], 95.));
    a
}

/// Relação pinhão e coroa por acoplamento de juntas.
pub fn testa_transmissao(u: &[(String, LinkedGeo)]) -> String {
    let (dc, zt) = (valor(u, "DC"), valor(u, "Z_EIXO_T"));
    let (np, nc) = (valor(u, "N_PINHAO"), valor(u, "N_COROA"));
    let mut a = Assembly::new("TESTE-TRANSMISSAO");
    let b = peca(&mut a, "MO-02", pl([0.; 3]));
    fixa(&mut a, b);
    let pin = peca(&mut a, "TR-02", pl([dc * 0.8, 0., zt + dc * 0.6]));
    let cor = peca(&mut a, "TR-01", pl([0., 0., zt]));
    let y: V3 = [0., 1., 0.];
    let j1 = a.add_joint(JointKind::Revolute, eixo(b, [dc * 0.8, 0., zt + dc * 0.6], y), eixo(pin, [0.; 3], y));
    let j2 = a.add_joint(JointKind::Revolute, eixo(b, [0., 0., zt], y), eixo(cor, [0.; 3], y));
    a.add_coupling(CouplingKind::Gear, j1, j2, &format!("{}", np / nc));
    a.joints.iter_mut().find(|j| j.id == j1).unwrap().drive_angle = Some("t".into());
    let q = simulate(&mut a, 0., 360., 8);
    grava(&a, "TESTE-TRANSMISSAO");
    let g = |p: &Placement| p.rot[0][2].atan2(p.rot[0][0]).to_degrees();
    let mut s = format!("relação esperada {:.4}: ", nc / np);
    for f in &q {
        s += &format!("t={:.0} ok={} coroa={:.3} | ", f.t, f.ok, g(&f.placements[&cor]));
    }
    s
}

pub fn sc_freio(u: &[(String, LinkedGeo)]) -> Assembly {
    let zt = valor(u, "Z_EIXO_T");
    let mut a = Assembly::new("SC-FREIO");
    let p = peca(&mut a, "EX-09", pl([90., 191., zt - 20.]));
    fixa(&mut a, p);
    peca(&mut a, "EX-10", pl([0., 182., zt]));
    peca(&mut a, "EX-10", pl([0., 200., zt]));
    peca(&mut a, "EX-11", pl([560., 170., 80.]));
    peca(&mut a, "EX-12", pl([0.; 3]));
    let ee = valor(u, "EE");
    peca(&mut a, "PO-05", pl([ee + 60., 90., 60.]));
    peca(&mut a, "PO-06", pl([ee + 60., -90., 60.]));
    peca(&mut a, "PO-07", pl([ee - 460., 90., 70.]));
    peca(&mut a, "PO-07", pl([ee - 460., -90., 70.]));
    a
}

pub fn sc_posto(u: &[(String, LinkedGeo)]) -> Assembly {
    let (ee, zq) = (valor(u, "EE"), valor(u, "Z_Q"));
    let mut a = Assembly::new("SC-POSTO");
    let b = peca(&mut a, "PO-01", pl([ee * 0.33, 0., zq + 60.]));
    fixa(&mut a, b);
    for (x, y) in [(300., 190.), (300., -190.), (520., 190.), (520., -190.)] {
        peca(&mut a, "PO-02", pl([x, y, 200.]));
    }
    peca(&mut a, "PO-03", pl([0.; 3]));
    peca(&mut a, "PO-03", pl180([600., 0., 0.]));
    peca(&mut a, "PO-04", pl([300., 0., zq - 16.]));
    peca(&mut a, "PO-08", pl([ee * 0.68, 0., zq + 20.]));
    a
}

pub fn sc_carroceria(u: &[(String, LinkedGeo)]) -> Assembly {
    let (ee, bal) = (valor(u, "EE"), valor(u, "BAL_D"));
    let mut a = Assembly::new("SC-CARROCERIA");
    let c = peca(&mut a, "PR-02", pl([0.; 3]));
    fixa(&mut a, c);
    peca(&mut a, "PR-01", pl([0.; 3]));
    peca(&mut a, "PR-03", pl([0.; 3]));
    for cod in ["PR-04", "PR-05", "PR-10"] {
        peca(&mut a, cod, pl([0.; 3]));
        peca(&mut a, cod, Placement { origin: [if cod == "PR-10" { 0. } else { ee * 0.5 + 280. }, 0., 0.], rot: [[-1., 0., 0.], [0., -1., 0.], [0., 0., 1.]] });
    }
    peca(&mut a, "PR-06", pl([ee + 200., 0., 0.]));
    peca(&mut a, "PR-07", pl([ee - 100., 0., 0.]));
    peca(&mut a, "PR-08", pl([ee * 0.5, valor(u, "Y_LAT_SUP") - 30., 0.]));
    peca(&mut a, "PR-08", pl180([ee * 0.5, -valor(u, "Y_LAT_SUP") + 30., 0.]));
    peca(&mut a, "PR-09", pl([-valor(u, "BAL_T") + 60., 0., 0.]));
    let _ = bal;
    a
}

pub fn kart(u: &[(String, LinkedGeo)], r: &mut Reg) {
    let dir = modelo();
    let t = std::time::Instant::now();
    let mut linha = |r: &mut Reg, n: &str, a: &mut Assembly| {
        let rep = solve(a);
        grava(a, n);
        let rows = bom(a, Some(&dir), BomLevel::Pecas);
        let qt: u32 = rows.iter().map(|x| x.qty).sum();
        r.usa("Montagem", &["Componente", "Restrição", "Rígido", "Junta", "Lista de peças"]);
        r.registra(Linha { cod: n.into(), nome: "Conjunto".into(), bancada: "Montagem".into(), ok: rep.converged, volume: 0., caixa: [0.; 3], corpos: qt as usize, nota: format!("nós={} juntas={} dof={} resolvida={}", a.nodes.len(), a.joints.len(), dof(a), rep.converged) });
    };
    let mut q = sc_quadro(u);
    linha(r, "SC-QUADRO", &mut q);
    let mut e = sc_eixo(u);
    linha(r, "SC-EIXO", &mut e);
    match sc_direcao(u) {
        Ok(mut d) => {
            let txt = testa_direcao(&mut d);
            println!("direção: {txt}");
            linha(r, "SC-DIRECAO", &mut d.a);
        }
        Err(x) => println!("direção falhou: {x}"),
    }
    let mut m = sc_motor(u);
    linha(r, "SC-MOTOR", &mut m);
    println!("transmissão: {}", testa_transmissao(u));
    let mut f = sc_freio(u);
    linha(r, "SC-FREIO", &mut f);
    let mut p = sc_posto(u);
    linha(r, "SC-POSTO", &mut p);
    let mut c = sc_carroceria(u);
    linha(r, "SC-CARROCERIA", &mut c);
    let mut k = Assembly::new("KART");
    let mestre = k.add_component("MESTRE", NodeKind::Part { path: "MESTRE.fkp".into() }, Placement::default(), None);
    k.esqueleto = Some(mestre);
    k.node_mut(mestre).unwrap().fixed = true;
    for s in ["SC-QUADRO", "SC-EIXO", "SC-DIRECAO", "SC-MOTOR", "SC-FREIO", "SC-POSTO", "SC-CARROCERIA"] {
        let n = sub(&mut k, s);
        fixa(&mut k, n);
    }
    linha(r, "KART", &mut k);
    println!("montagens em {:.1}s", t.elapsed().as_secs_f64());
    // contorno K-03: desenho e pacote não descem em subconjunto ou param no esqueleto; montagem plana
    let mut plano = Assembly::new("KART-PLANO");
    for s in ["SC-QUADRO", "SC-EIXO", "SC-DIRECAO", "SC-MOTOR", "SC-FREIO", "SC-POSTO", "SC-CARROCERIA"] {
        let a = Assembly::from_json(&std::fs::read_to_string(dir.join(format!("{s}.fka"))).unwrap()).unwrap();
        for n in a.nodes.iter() {
            if let NodeKind::Part { path } = &n.kind {
                if s == "SC-DIRECAO" && path == "QUADRO.fkp" {
                    continue;
                }
                let id = plano.add_component(&n.name, n.kind.clone(), n.placement, None);
                plano.node_mut(id).unwrap().fixed = true;
            }
        }
    }
    grava(&plano, "KART-PLANO");
    for row in bom(&k, Some(&dir), BomLevel::Estruturada).iter().take(12) {
        println!("BOM {} {} x{} PN={}", row.item, row.name, row.qty, row.pn);
    }
    let t = std::time::Instant::now();
    let mut cache = PartCache::default();
    let ach = analisar(&k, Some(&dir), &mut cache, &OpcoesInterf { sub_unico: false, ..Default::default() });
    println!("interferência: {} achados em {:.1}s", ach.len(), t.elapsed().as_secs_f64());
    for x in ach.iter().take(30) {
        println!("  {} x {}: {:.0} mm³", x.a.nome, x.b.nome, x.volume);
    }
}

pub fn _caminho(p: &Path) -> &Path {
    p
}
