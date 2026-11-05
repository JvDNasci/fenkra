// SPDX-License-Identifier: LGPL-2.1-or-later
//! Ajudas comuns aos testes de integração do crate: montagem de documentos em código, conferência de
//! status, volume e caixa envolvente, medida de memória do processo e gerador pseudoaleatório com
//! semente fixa. Só usa a API pública; nada aqui toca `src/`.
#![allow(dead_code)]

use fenkra_core::sketch::Curve;
use fenkra_core::*;
use fenkra_solidos::{final_shape, recompute, BodyResult, RecomputeResult, Status};
use std::f64::consts::PI;

pub fn kernel_disponivel() -> bool {
    fenkra_kernel::AVAILABLE
}

// ------------------------------------------------------------------ montagem

/// Sketch retangular `a`..`b` no plano dado.
pub fn sk_rect(d: &mut Document, plane: PlaneRef, a: [f64; 2], b: [f64; 2]) -> Id {
    let mut data = SketchData::default();
    let mut al = || d.alloc();
    data.add_rect(&mut al, a, b, false);
    d.add_feature(FeatureKind::Sketch(Sketch { plane, data }))
}

/// Substitui o retângulo de um sketch (muda as cotas do perfil sem trocar o id do sketch).
pub fn set_rect(d: &mut Document, sk: Id, a: [f64; 2], b: [f64; 2]) {
    let mut data = SketchData::default();
    let mut al = || d.alloc();
    data.add_rect(&mut al, a, b, false);
    if let Some(FeatureKind::Sketch(s)) = d.feature_mut(sk).map(|f| &mut f.kind) {
        s.data = data;
    }
}

fn dados_circulos(d: &mut Document, circulos: &[([f64; 2], f64)]) -> SketchData {
    let mut data = SketchData::default();
    let mut al = || d.alloc();
    for (c, r) in circulos {
        let p = data.add_point(&mut al, c[0], c[1]);
        data.add_curve(&mut al, Curve::Circle { c: p, r: *r }, false);
    }
    data
}

/// Sketch com círculos `(centro, raio)`.
pub fn sk_circulos(d: &mut Document, plane: PlaneRef, circulos: &[([f64; 2], f64)]) -> Id {
    let data = dados_circulos(d, circulos);
    d.add_feature(FeatureKind::Sketch(Sketch { plane, data }))
}

/// Troca o raio do círculo único de um sketch.
pub fn set_circulo(d: &mut Document, sk: Id, c: [f64; 2], r: f64) {
    let data = dados_circulos(d, &[(c, r)]);
    if let Some(FeatureKind::Sketch(s)) = d.feature_mut(sk).map(|f| &mut f.kind) {
        s.data = data;
    }
}

/// Sketch com pontos isolados (centros de furo).
pub fn sk_pontos(d: &mut Document, plane: PlaneRef, pts: &[[f64; 2]]) -> Id {
    let mut data = SketchData::default();
    let mut al = || d.alloc();
    for p in pts {
        let id = data.add_point(&mut al, p[0], p[1]);
        data.add_curve(&mut al, Curve::Point { p: id }, false);
    }
    d.add_feature(FeatureKind::Sketch(Sketch { plane, data }))
}

/// Sketch com um polígono fechado de linhas que compartilham pontos.
pub fn sk_poligono(d: &mut Document, plane: PlaneRef, pts: &[[f64; 2]]) -> Id {
    let mut data = SketchData::default();
    let mut al = || d.alloc();
    let ids: Vec<Id> = pts.iter().map(|p| data.add_point(&mut al, p[0], p[1])).collect();
    for i in 0..ids.len() {
        data.add_curve(&mut al, Curve::Line { a: ids[i], b: ids[(i + 1) % ids.len()] }, false);
    }
    d.add_feature(FeatureKind::Sketch(Sketch { plane, data }))
}

pub fn ext(sketch: Id, op: BoolOp, dist: &str) -> FeatureKind {
    FeatureKind::Extrude(Extrude { sketch, op, extent: Extent::Blind(dist.into()), reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0 })
}

pub fn furo_passante(sketch: Id, diametro: &str) -> FeatureKind {
    FeatureKind::Hole(Hole {
        sketch,
        kind: HoleKind::Simples,
        diameter: diametro.into(),
        depth: Extent::ThroughAll,
        thread: None,
        thread_modeled: false,
        standard: None,
        points: vec![],
        dir: None,
        reverse: false,
        tip: None,
        taper: "0".into(),
        left_hand: false,
    })
}

pub fn primitiva(kind: PrimKind, a: &str, b: &str, c: &str, d: &str) -> FeatureKind {
    FeatureKind::Primitive(Primitive { kind, plane: PlaneRef::Origin(OriginPlane::XY), a: a.into(), b: b.into(), c: c.into(), d: d.into(), op: BoolOp::Add })
}

pub fn filete(valor: &str, mids: Vec<[f64; 3]>, keys: Vec<Option<EdgeKey>>) -> FeatureKind {
    FeatureKind::Fillet(EdgeOp { value: valor.into(), edges: EdgeSel::Edges(mids), mode: EdgeMode::Constant, prop: Default::default(), keys })
}

pub fn chanfro(valor: &str, mids: Vec<[f64; 3]>, keys: Vec<Option<EdgeKey>>) -> FeatureKind {
    FeatureKind::Chamfer(EdgeOp { value: valor.into(), edges: EdgeSel::Edges(mids), mode: EdgeMode::Constant, prop: Default::default(), keys })
}

pub fn definir_param(d: &mut Document, nome: &str, expr: &str) {
    d.params.iter_mut().find(|p| p.name == nome).unwrap_or_else(|| panic!("parâmetro {nome} não existe")).expr = expr.into();
}

/// Índice da aresta da malha cujo ponto médio está mais perto de `p` (o que o picking faz).
pub fn aresta_perto(b: &BodyResult, p: [f64; 3]) -> usize {
    let dist = |e: usize| (0..3).map(|i| (b.mesh.edge_mids[e * 3 + i] as f64 - p[i]).powi(2)).sum::<f64>();
    (0..b.mesh.edge_mids.len() / 3).min_by(|a, c| dist(*a).total_cmp(&dist(*c))).expect("corpo sem arestas")
}

pub fn ponto_medio(b: &BodyResult, e: usize) -> [f64; 3] {
    [0, 1, 2].map(|i| b.mesh.edge_mids[e * 3 + i] as f64)
}

// ------------------------------------------------------------------ conferência

/// O que se espera de uma peça: volume (tolerância relativa) e dimensões da caixa envolvente.
pub struct Esperado {
    pub vol: f64,
    pub tol_vol: f64,
    pub tam: [f64; 3],
    pub tol_tam: f64,
    /// canto mínimo da caixa, quando a posição também faz parte do contrato
    pub minimo: Option<[f64; 3]>,
    /// mede a caixa pelos vértices da malha em vez de pela caixa do kernel (que engorda superfícies curvas)
    pub via_malha: bool,
}

impl Esperado {
    pub fn prisma(vol: f64, tam: [f64; 3], minimo: [f64; 3]) -> Self {
        Esperado { vol, tol_vol: 1e-4, tam, tol_tam: 1e-3, minimo: Some(minimo), via_malha: false }
    }
    pub fn curva(vol: f64, tam: [f64; 3]) -> Self {
        Esperado { vol, tol_vol: 2e-3, tam, tol_tam: 0.2, minimo: None, via_malha: true }
    }
}

/// Status de todos os comandos: Ok, ou aviso em sketch (perfil aberto/pontos). Devolve o primeiro problema.
pub fn problema_de_status(d: &Document, r: &RecomputeResult) -> Option<String> {
    for f in &d.features {
        let Some(st) = r.status.get(&f.id) else { return Some(format!("{} (#{}): sem status", f.name, f.id.0)) };
        let aviso_de_sketch = matches!(f.kind, FeatureKind::Sketch(_)) && matches!(st, Status::Warning(_));
        if *st != Status::Ok && !aviso_de_sketch {
            return Some(format!("{} (#{}): {:?}", f.name, f.id.0, st));
        }
    }
    None
}

pub fn caixa_envolvente(d: &Document) -> ([f64; 3], [f64; 3]) {
    final_shape(d).expect("sem forma final").bbox().expect("sem caixa envolvente")
}

/// Caixa dos vértices da malha de todos os corpos (nunca passa da caixa real; erra pela flecha da tesselação).
pub fn caixa_da_malha(r: &RecomputeResult) -> ([f64; 3], [f64; 3]) {
    let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
    for b in &r.bodies {
        for p in b.mesh.positions.chunks_exact(3) {
            for i in 0..3 {
                lo[i] = lo[i].min(p[i] as f64);
                hi[i] = hi[i].max(p[i] as f64);
            }
        }
    }
    (lo, hi)
}

pub fn volume_total(r: &RecomputeResult) -> f64 {
    r.bodies.iter().map(|b| b.mass.volume).sum()
}

/// Recalcula e confere status, volume e caixa envolvente. Devolve o volume medido.
pub fn confere(nome: &str, d: &Document, e: &Esperado) -> f64 {
    let r = recompute(d);
    if let Some(p) = problema_de_status(d, &r) {
        panic!("{nome}: comando com problema: {p}");
    }
    assert_eq!(r.bodies.len(), 1, "{nome}: esperava um corpo");
    let v = volume_total(&r);
    assert!((v - e.vol).abs() <= e.tol_vol * e.vol.abs().max(1.), "{nome}: volume {v:.4} esperado {:.4} (tol {:.1e})", e.vol, e.tol_vol);
    let (lo, hi) = if e.via_malha { caixa_da_malha(&r) } else { caixa_envolvente(d) };
    for i in 0..3 {
        let t = hi[i] - lo[i];
        assert!((t - e.tam[i]).abs() <= e.tol_tam, "{nome}: eixo {i} mede {t:.4} esperado {:.4} (tol {})", e.tam[i], e.tol_tam);
        if let Some(m) = e.minimo {
            assert!((lo[i] - m[i]).abs() <= e.tol_tam, "{nome}: canto mínimo do eixo {i} em {:.4} esperado {:.4}", lo[i], m[i]);
        }
    }
    v
}

// ------------------------------------------------------------------ fórmulas

/// Volume de uma caixa a×b×c com as 12 arestas arredondadas por r.
pub fn vol_caixa_arredondada(a: f64, b: f64, c: f64, r: f64) -> f64 {
    let (x, y, z) = (a - 2. * r, b - 2. * r, c - 2. * r);
    x * y * z + 2. * r * (x * y + x * z + y * z) + PI * r * r * (x + y + z) + 4. / 3. * PI * r.powi(3)
}

/// Volume tirado por um filete de raio r na borda circular de cima de um cilindro de raio R (Pappus).
pub fn vol_filete_borda_circular(raio: f64, r: f64) -> f64 {
    2. * PI * (r * r * (raio - r / 2.) - PI * r * r / 4. * ((raio - r) + 4. * r / (3. * PI)))
}

// ------------------------------------------------------------------ memória do processo

/// Memória privada do processo em bytes (inclui o que o C++ do kernel aloca, que um alocador Rust não vê).
#[cfg(windows)]
pub fn memoria_privada() -> Option<usize> {
    #[repr(C)]
    struct Contadores {
        cb: u32,
        faltas: u32,
        pico_ws: usize,
        ws: usize,
        pico_paged: usize,
        paged: usize,
        pico_nonpaged: usize,
        nonpaged: usize,
        pagefile: usize,
        pico_pagefile: usize,
    }
    extern "system" {
        fn GetCurrentProcess() -> isize;
        fn K32GetProcessMemoryInfo(h: isize, c: *mut Contadores, cb: u32) -> i32;
    }
    let mut c = Contadores { cb: 0, faltas: 0, pico_ws: 0, ws: 0, pico_paged: 0, paged: 0, pico_nonpaged: 0, nonpaged: 0, pagefile: 0, pico_pagefile: 0 };
    c.cb = std::mem::size_of::<Contadores>() as u32;
    // SAFETY: estrutura com o layout de PROCESS_MEMORY_COUNTERS e `cb` correto; o handle é o do próprio processo.
    let ok = unsafe { K32GetProcessMemoryInfo(GetCurrentProcess(), &mut c, c.cb) };
    (ok != 0).then_some(c.pagefile)
}

#[cfg(all(unix, not(windows)))]
pub fn memoria_privada() -> Option<usize> {
    let s = std::fs::read_to_string("/proc/self/statm").ok()?;
    let paginas: usize = s.split_whitespace().nth(1)?.parse().ok()?;
    Some(paginas * 4096)
}

#[cfg(not(any(windows, unix)))]
pub fn memoria_privada() -> Option<usize> {
    None
}

// ------------------------------------------------------------------ aleatório com semente

/// SplitMix64: pequeno, determinístico e sem dependências.
pub struct Sorteio(pub u64);

impl Sorteio {
    pub fn proximo(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// Número em [lo, hi), arredondado a 0,1 mm (cotas de gente, não de máquina).
    pub fn cota(&mut self, lo: f64, hi: f64) -> f64 {
        let u = (self.proximo() >> 11) as f64 / (1u64 << 53) as f64;
        ((lo + (hi - lo) * u) * 10.).round() / 10.
    }
}
