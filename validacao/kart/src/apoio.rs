// SPDX-License-Identifier: LGPL-2.1-or-later
//! Ajudas do projeto-teste: montar documentos, recalcular, conferir e registrar cada peça.
use fenkra_core::sketch::Curve;
use fenkra_core::*;
use fenkra_solidos::{recompute, RecomputeResult, Status};
use std::collections::BTreeMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::PathBuf;

pub fn s(x: &str) -> String {
    x.to_string()
}

pub fn modelo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("modelo")
}

#[derive(Clone, Debug)]
pub struct Linha {
    pub cod: String,
    pub nome: String,
    pub bancada: String,
    pub ok: bool,
    pub volume: f64,
    pub caixa: [f64; 3],
    pub corpos: usize,
    pub nota: String,
}

#[derive(Default)]
pub struct Reg {
    pub linhas: BTreeMap<String, Linha>,
    /// bancada → comando → vezes que o comando entrou num documento e calculou sem erro (uso real)
    pub cmds: BTreeMap<String, BTreeMap<String, u32>>,
    /// bancada → comando → vezes que entrou num documento e falhou ou ficou suprimido
    pub sem_efeito: BTreeMap<String, BTreeMap<String, u32>>,
    /// bancada → comando → vezes que o roteiro disse usar, sem prova no documento (B431)
    pub declarados: BTreeMap<String, BTreeMap<String, u32>>,
}

impl Reg {
    /// O roteiro declara que usou estes comandos (fluxos fora do documento: montagem, desenho, PMI,
    /// PLM). Vai para a tabela de declarados: não vira uso real sem prova (B431).
    pub fn usa(&mut self, bancada: &str, cmds: &[&str]) {
        let b = self.declarados.entry(bancada.into()).or_default();
        for c in cmds {
            *b.entry((*c).into()).or_default() += 1;
        }
    }

    /// Conta os comandos do documento na matriz pelo que o cálculo mostrou: só entra como uso real o
    /// comando que calculou sem erro e não está suprimido. Antes a matriz contava os comandos que o
    /// documento tinha, mesmo os que falharam (B431).
    pub fn conta_doc(&mut self, bancada: &str, d: &Document, r: &RecomputeResult) {
        for f in &d.features {
            let t = format!("{:?}", f.kind);
            let nome = t.split(|c: char| !c.is_alphanumeric()).next().unwrap_or("").to_string();
            let efetivo = !f.suppressed && !matches!(r.status.get(&f.id), Some(Status::Error(_)) | Some(Status::Inactive));
            let alvo = if efetivo { &mut self.cmds } else { &mut self.sem_efeito };
            *alvo.entry(bancada.into()).or_default().entry(nome).or_default() += 1;
        }
    }

    /// Tabelas geradas: resultados por item e matriz de comandos.
    pub fn grava_tabelas(&self) {
        let mut t = String::from("# Resultados gerados\n\nGerado por `kart.exe`: não edite à mão. Volume em mm³, caixa em mm.\n\n| Item | Nome | Bancada | Estado | Volume | Caixa | Corpos/itens | Observação |\n|---|---|---|---|---|---|---|---|\n");
        for l in self.linhas.values() {
            t += &format!("| {} | {} | {} | {} | {:.0} | {:.1} × {:.1} × {:.1} | {} | {} |\n", l.cod, l.nome, l.bancada, if l.ok { "ok" } else { "FALHA" }, l.volume, l.caixa[0], l.caixa[1], l.caixa[2], l.corpos, l.nota.trim().replace('|', "/"));
        }
        t += "\n## Matriz de comandos exercitados\n\nUso real: o comando entrou num documento e calculou sem erro.\n\n| Bancada | Comando | Uso real | Sem efeito (erro ou suprimido) |\n|---|---|---|---|\n";
        let mut chaves: std::collections::BTreeSet<(&String, &String)> = std::collections::BTreeSet::new();
        for m in [&self.cmds, &self.sem_efeito] {
            for (b, c) in m {
                for k in c.keys() {
                    chaves.insert((b, k));
                }
            }
        }
        let n = |m: &BTreeMap<String, BTreeMap<String, u32>>, b: &String, c: &String| m.get(b).and_then(|x| x.get(c)).copied().unwrap_or(0);
        for (b, c) in chaves {
            t += &format!("| {b} | {c} | {} | {} |\n", n(&self.cmds, b, c), n(&self.sem_efeito, b, c));
        }
        t += "\n## Declarados pelo roteiro, sem prova no documento\n\nFluxos que não ficam num `.fkp` (montagem, desenho, PMI, PLM). Contam chamadas do roteiro, não uso conferido.\n\n| Bancada | Comando | Vezes |\n|---|---|---|\n";
        for (b, m) in &self.declarados {
            for (c, k) in m {
                t += &format!("| {b} | {c} | {k} |\n");
            }
        }
        let _ = std::fs::write(modelo().parent().unwrap().join("resultados.md"), t);
    }

    pub fn registra(&mut self, l: Linha) {
        println!("{} {} [{}] {} vol={:.0} caixa={:.1}x{:.1}x{:.1} corpos={} {}", if l.ok { "OK   " } else { "FALHA" }, l.cod, l.bancada, l.nome, l.volume, l.caixa[0], l.caixa[1], l.caixa[2], l.corpos, l.nota);
        self.linhas.insert(l.cod.clone(), l);
    }

    /// Roda a montagem da peça, recalcula, confere, grava `<cod>.fkp`.
    pub fn peca(&mut self, cod: &str, nome: &str, bancada: &str, material: &str, f: impl FnOnce() -> Result<Document, String>) -> Option<Document> {
        let t = std::time::Instant::now();
        let falha = |nota: String| Linha { cod: cod.into(), nome: nome.into(), bancada: bancada.into(), ok: false, volume: 0., caixa: [0.; 3], corpos: 0, nota };
        let mut d = match catch_unwind(AssertUnwindSafe(f)) {
            Ok(Ok(d)) => d,
            Ok(Err(e)) => {
                self.registra(falha(e));
                return None;
            }
            Err(p) => {
                let m = p.downcast_ref::<String>().cloned().or(p.downcast_ref::<&str>().map(|x| x.to_string())).unwrap_or_default();
                self.registra(falha(format!("pânico: {m}")));
                return None;
            }
        };
        d.name = format!("{cod} {nome}");
        d.props.pn = format!("FK-{cod}");
        d.props.description = nome.into();
        if !material.is_empty() {
            d.props.material = material.into();
        }
        let r = recompute(&d);
        let (ok, mut nota) = confere(&d, &r);
        let (lo, hi) = caixa(&r);
        let tam = [hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]];
        let v: f64 = r.bodies.iter().map(|b| b.mass.volume).sum();
        nota = format!("{nota} [{:.1}s]", t.elapsed().as_secs_f64());
        let _ = std::fs::write(modelo().join(format!("{cod}.fkp")), d.to_json());
        self.conta_doc(bancada, &d, &r);
        let semgeo = cod != "MESTRE" && r.bodies.is_empty();
        self.registra(Linha { cod: cod.into(), nome: nome.into(), bancada: bancada.into(), ok: ok && !semgeo, volume: v, caixa: tam, corpos: r.bodies.len(), nota });
        Some(d)
    }
}

/// Junta os avisos e erros dos comandos; `ok` falso só se houver erro.
pub fn confere(d: &Document, r: &RecomputeResult) -> (bool, String) {
    let mut ok = r.param_error.is_none();
    let mut nota = r.param_error.clone().unwrap_or_default();
    for f in &d.features {
        match r.status.get(&f.id) {
            Some(Status::Error(e)) => {
                ok = false;
                nota += &format!(" ERRO {}: {e};", f.name);
            }
            Some(Status::Warning(e)) if !matches!(f.kind, FeatureKind::Sketch(_)) => nota += &format!(" aviso {}: {e};", f.name),
            _ => {}
        }
    }
    (ok, nota)
}

pub fn caixa(r: &RecomputeResult) -> ([f64; 3], [f64; 3]) {
    let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
    for b in &r.bodies {
        for p in b.mesh.positions.chunks_exact(3) {
            for i in 0..3 {
                lo[i] = lo[i].min(p[i] as f64);
                hi[i] = hi[i].max(p[i] as f64);
            }
        }
    }
    if lo[0] > hi[0] {
        return ([0.; 3], [0.; 3]);
    }
    (lo, hi)
}

// ------------------------------------------------------------------ sketch

pub fn sk_rect(d: &mut Document, plane: PlaneRef, a: [f64; 2], b: [f64; 2]) -> Id {
    let mut data = SketchData::default();
    let mut al = || d.alloc();
    data.add_rect(&mut al, a, b, false);
    d.add_feature(FeatureKind::Sketch(Sketch { plane, data }))
}

pub fn sk_circ(d: &mut Document, plane: PlaneRef, cs: &[([f64; 2], f64)]) -> Id {
    let mut data = SketchData::default();
    let mut al = || d.alloc();
    for (c, r) in cs {
        let p = data.add_point(&mut al, c[0], c[1]);
        data.add_curve(&mut al, Curve::Circle { c: p, r: *r }, false);
    }
    d.add_feature(FeatureKind::Sketch(Sketch { plane, data }))
}

pub fn sk_pts(d: &mut Document, plane: PlaneRef, pts: &[[f64; 2]]) -> Id {
    let mut data = SketchData::default();
    let mut al = || d.alloc();
    for p in pts {
        let id = data.add_point(&mut al, p[0], p[1]);
        data.add_curve(&mut al, Curve::Point { p: id }, false);
    }
    d.add_feature(FeatureKind::Sketch(Sketch { plane, data }))
}

pub fn sk_poli(d: &mut Document, plane: PlaneRef, pts: &[[f64; 2]]) -> Id {
    let mut data = SketchData::default();
    let mut al = || d.alloc();
    let ids: Vec<Id> = pts.iter().map(|p| data.add_point(&mut al, p[0], p[1])).collect();
    for i in 0..ids.len() {
        data.add_curve(&mut al, Curve::Line { a: ids[i], b: ids[(i + 1) % ids.len()] }, false);
    }
    d.add_feature(FeatureKind::Sketch(Sketch { plane, data }))
}

pub fn ext(sketch: Id, op: BoolOp, e: Extent) -> FeatureKind {
    FeatureKind::Extrude(Extrude { sketch, op, extent: e, reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() })
}

pub fn blind(x: &str) -> Extent {
    Extent::Blind(x.into())
}

pub fn sim(x: &str) -> Extent {
    Extent::Symmetric(x.into())
}

/// Revolução completa do perfil em torno de um eixo da origem.
pub fn rev(sketch: Id, eixo: OriginAxis) -> FeatureKind {
    FeatureKind::Revolve(Revolve { sketch, axis: AxisRef::Origin(eixo), angle: "360".into(), symmetric: false, op: BoolOp::Add, thin: None, thin_side: 0, angle2: None })
}

pub fn furo(sketch: Id, dia: &str) -> FeatureKind {
    FeatureKind::Hole(Hole { sketch, kind: HoleKind::Simples, diameter: dia.into(), depth: Extent::ThroughAll, thread: None, thread_modeled: false, standard: None, points: vec![], dir: None, reverse: false, tip: None, taper: "0".into(), left_hand: false, extra: Default::default() })
}

pub fn oxy() -> PlaneRef {
    PlaneRef::Origin(OriginPlane::XY)
}
pub fn oxz() -> PlaneRef {
    PlaneRef::Origin(OriginPlane::XZ)
}
pub fn oyz() -> PlaneRef {
    PlaneRef::Origin(OriginPlane::YZ)
}

// ------------------------------------------------------------------ estrutural

pub fn tubo(paths: Vec<CurveRef>, dia: &str, parede: &str, desig: &str) -> StructMember {
    StructMember { paths, family: s("Tubo redondo"), designation: desig.into(), kind: SectionKind::TuboRedondo, dims: vec![dia.into(), parede.into()], anchor: 0, angle: s("0"), off_x: s("0"), off_y: s("0"), ext1: s("0"), ext2: s("0"), corner: CornerMode::Nenhum, gap: s("0"), joints: vec![], material: String::new(), mirror: false }
}

pub fn ponto(d: &mut Document, nome: &str, x: &str, y: &str, z: &str) -> Id {
    let id = d.add_geometry(RefKind::PointCoords { x: x.into(), y: y.into(), z: z.into(), frame: None });
    renomeia_ref(d, id, nome);
    id
}

pub fn renomeia_ref(d: &mut Document, id: Id, nome: &str) {
    if let Some(e) = d.references.iter_mut().find(|e| e.id == id) {
        e.name = nome.into();
    }
}

pub fn publica_ref(d: &mut Document, id: Id) {
    if let Some(e) = d.references.iter_mut().find(|e| e.id == id) {
        e.published = true;
    }
}

/// Polilinha publicada pelos pontos.
pub fn polilinha(d: &mut Document, nome: &str, pts: &[Id]) -> Id {
    let id = d.add_geometry(RefKind::Polyline { points: pts.iter().map(|p| PointRef::Ref(*p)).collect(), closed: false });
    renomeia_ref(d, id, nome);
    publica_ref(d, id);
    // contorno K-02: `Segment` de curva ligada não acha os vértices; cada trecho sai publicado à parte
    if pts.len() > 2 {
        for i in 0..pts.len() - 1 {
            let t = d.add_geometry(RefKind::Polyline { points: vec![PointRef::Ref(pts[i]), PointRef::Ref(pts[i + 1])], closed: false });
            renomeia_ref(d, t, &format!("{nome}_{i}"));
            publica_ref(d, t);
        }
    }
    id
}

/// Trechos de uma polilinha como caminhos.
pub fn trechos(curva: Id, n: usize) -> Vec<CurveRef> {
    (0..n).map(|i| CurveRef::Segment { curve: curva, index: i }).collect()
}

/// Dobra de tubo em todos os nós internos do comando.
pub fn dobra_nos(d: &mut Document, f: Id, n: usize, raio: &str) {
    if let Some(FeatureKind::StructMember(m)) = d.feature_mut(f).map(|f| &mut f.kind) {
        for i in 1..n {
            m.joints.push(StructJoint { mine: i, other: f, other_path: i - 1, mode: CornerMode::Dobra, first: false, gap: None, bend: Some(TubeBend { mode: BendMode::Dobradeira, radius: raio.into(), ..Default::default() }) });
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn matriz_so_conta_como_uso_real_o_que_calculou() {
        let mut d = Document::new("t");
        let sk = sk_rect(&mut d, oxy(), [0., 0.], [10., 10.]);
        d.add_feature(ext(sk, BoolOp::Add, blind("5")));
        // extrusão que falha: o esboço citado não existe
        d.add_feature(ext(Id(9999), BoolOp::Add, blind("5")));
        let r = recompute(&d);
        let mut reg = Reg::default();
        reg.conta_doc("Sólidos", &d, &r);
        reg.usa("PMI", &["Nota"]);
        // o que o roteiro declara não vira uso real
        assert!(reg.cmds.get("PMI").is_none());
        assert_eq!(reg.declarados["PMI"]["Nota"], 1);
        // a extrusão que calculou conta como uso real; a que falhou, como sem efeito
        assert_eq!(reg.cmds["Sólidos"]["Extrude"], 1);
        assert_eq!(reg.sem_efeito["Sólidos"]["Extrude"], 1);
        let total: u32 = reg.cmds.values().chain(reg.sem_efeito.values()).flat_map(|m| m.values()).sum();
        assert_eq!(total as usize, d.features.len());
    }
}
