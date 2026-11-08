// SPDX-License-Identifier: LGPL-2.1-or-later
//! B356: prévia leve dos comandos pesados (ADR 0075).
//!
//! Como funciona no kernel: filete, chanfro, casca e padrão chamam operações do OpenCASCADE que
//! reconstroem faces e fazem booleanas sobre o corpo inteiro; o custo cresce com o número de faces e
//! de instâncias e é refeito a cada mudança de campo. Com o comando aberto (`Cache::leve`), em vez
//! disso, o corpo de entrada passa intacto e o comando só devolve onde e quanto ele vai mexer
//! (`PreviaViz`): as arestas escolhidas e a distância do filete, a caixa interna da casca, a caixa de
//! cada instância do padrão. A interface desenha isso em linhas, na cor do tema. Ao concluir, o app
//! zera `leve`, a assinatura do comando deixa de ter a marca "leve" e o cálculo real roda: o
//! resultado final nunca vem desta prévia.

use crate::geo3d::V3;
use crate::{Ctx, KernelError};
use fenkra_core::{EdgeMode, EdgeSel, Feature, FeatureKind, Id, PatternKind};
use fenkra_kernel::Xform;

/// O que um comando pesado vai fazer, em geometria barata.
#[derive(Clone, Debug, PartialEq)]
pub struct PreviaViz {
    pub feature: Id,
    /// pontos médios das arestas que o filete ou chanfro afeta, já resolvidos no corpo
    pub arestas: Vec<V3>,
    /// valor do filete (raio) ou do chanfro (distância), em mm
    pub valor: f64,
    /// chanfro: a distância vale igual nas duas faces; filete: é o raio
    pub chanfro: bool,
    /// comando sem arestas escolhidas ("todas"): a interface realça todas as arestas do corpo
    pub todas: bool,
    /// caixas (oito cantos, na ordem de `cantos_da_caixa`) que o comando acrescenta ou tira
    pub caixas: Vec<[V3; 8]>,
}

/// Comandos que têm prévia leve.
pub fn cobre(k: &FeatureKind) -> bool {
    match k {
        FeatureKind::Fillet(o) | FeatureKind::Chamfer(o) => matches!(o.mode, EdgeMode::Constant),
        FeatureKind::Shell(_) | FeatureKind::Pattern(_) | FeatureKind::Mirror(_) => true,
        _ => false,
    }
}

/// Oito cantos de uma caixa alinhada aos eixos: os quatro de baixo (z mínimo) em ordem de volta, e os
/// quatro de cima na mesma ordem.
pub fn cantos_da_caixa(lo: V3, hi: V3) -> [V3; 8] {
    [
        [lo[0], lo[1], lo[2]],
        [hi[0], lo[1], lo[2]],
        [hi[0], hi[1], lo[2]],
        [lo[0], hi[1], lo[2]],
        [lo[0], lo[1], hi[2]],
        [hi[0], lo[1], hi[2]],
        [hi[0], hi[1], hi[2]],
        [lo[0], hi[1], hi[2]],
    ]
}

/// As doze arestas de uma caixa, como pares de índices de `cantos_da_caixa`.
pub const ARESTAS_DA_CAIXA: [[usize; 2]; 12] = [[0, 1], [1, 2], [2, 3], [3, 0], [4, 5], [5, 6], [6, 7], [7, 4], [0, 4], [1, 5], [2, 6], [3, 7]];

/// Leva um ponto pela mesma transformação que o kernel aplicaria à ferramenta.
pub fn leva(x: &Xform, p: V3) -> V3 {
    use crate::geo3d::{add, cross, dot, mul, sub, unit};
    match *x {
        Xform::Translate(t) => add(p, t),
        Xform::Scale(c, f) => add(c, mul(sub(p, c), f)),
        Xform::Mirror(o, n) => {
            let n = unit(n).unwrap_or([0., 0., 1.]);
            sub(p, mul(n, 2. * dot(sub(p, o), n)))
        }
        // Rodrigues
        Xform::Rotate(o, d, graus) => {
            let k = unit(d).unwrap_or([0., 0., 1.]);
            let (s, c) = graus.to_radians().sin_cos();
            let v = sub(p, o);
            let r = add(add(mul(v, c), mul(cross(k, v), s)), mul(k, dot(k, v) * (1. - c)));
            add(o, r)
        }
    }
}

impl Ctx<'_> {
    /// Roda a versão leve do comando, se ele tem uma. `None` = segue o cálculo normal.
    pub(crate) fn previa_leve(&mut self, f: &Feature) -> Option<Result<Option<String>, String>> {
        if self.leve != Some(f.id) || !cobre(&f.kind) {
            return None;
        }
        Some(self.previa_leve_de(f))
    }

    fn previa_leve_de(&mut self, f: &Feature) -> Result<Option<String>, String> {
        let mut viz = PreviaViz { feature: f.id, arestas: vec![], valor: 0., chanfro: false, todas: false, caixas: vec![] };
        match &f.kind {
            FeatureKind::Fillet(o) | FeatureKind::Chamfer(o) => {
                if matches!(&o.edges, EdgeSel::Edges(m) if m.is_empty()) {
                    return Ok(Some("clique nas arestas na viewport (ou escolha 'Todas') — até lá nada é aplicado".into()));
                }
                viz.valor = crate::ev(&o.value, &self.vars, "valor")?;
                viz.chanfro = matches!(f.kind, FeatureKind::Chamfer(_));
                let (b, owners, roles) = self.body_owned(f.target).ok_or("não há corpo para aplicar")?;
                match &o.edges {
                    EdgeSel::Edges(m) => {
                        let escolhidas = self.resolve_edges(if viz.chanfro { "chanfro" } else { "filete" }, f, b, owners, roles, m, &o.keys)?;
                        // B95: a regra gravada também vale na prévia leve
                        viz.arestas = crate::selecao_regras::expandir_arestas(b, &escolhidas, &o.extra.regras)?;
                    }
                    _ => viz.todas = true,
                }
            }
            FeatureKind::Shell(sh) => {
                if sh.faces.is_empty() && !sh.closed {
                    return Ok(Some("escolha na viewport as faces a remover — até lá a casca não é aplicada".into()));
                }
                let t = crate::ev(&sh.thickness, &self.vars, "espessura")?;
                let b = self.body(f.target).ok_or("não há corpo para esvaziar")?;
                let (lo, hi) = b.bbox().ok_or("corpo sem dimensões")?;
                // a parede para dentro não cabe se for tão grossa quanto a menor dimensão do corpo
                let menor = (0..3).map(|i| hi[i] - lo[i]).fold(f64::INFINITY, f64::min);
                if !sh.outward && t >= menor - 1e-9 {
                    return Err(format!("espessura {t} grande demais para a peça: a menor dimensão do corpo é {menor}"));
                }
                // a caixa interna da cavidade (grosseira: a forma exata só sai ao concluir). Nos lados onde
                // a face escolhida é a da caixa do corpo, a cavidade fica aberta e não recua.
                let d = if sh.outward { -t } else { t };
                let (mut c0, mut c1) = ([d; 3], [-d; 3]);
                for i in 0..3 {
                    c0[i] += lo[i];
                    c1[i] += hi[i];
                }
                for p in &sh.faces {
                    // a face escolhida é a de uma lateral da caixa só se o ponto cai em exatamente uma delas
                    let lados: Vec<(usize, bool)> = (0..3).flat_map(|i| [(i, false), (i, true)]).filter(|(i, alto)| (p[*i] - if *alto { hi[*i] } else { lo[*i] }).abs() < 1e-6).collect();
                    if let [(i, alto)] = lados[..] {
                        if alto {
                            c1[i] = hi[i];
                        } else {
                            c0[i] = lo[i];
                        }
                    }
                }
                viz.valor = t;
                viz.caixas.push(cantos_da_caixa(c0, c1));
            }
            FeatureKind::Pattern(pt) => {
                let xs = self.instancias(&pt.kind)?;
                let xs = crate::padrao::sem_puladas(xs.0, &xs.1, &pt.skip).0;
                self.caixas_replicadas(&pt.sources, &xs, &mut viz, f.target)?;
            }
            FeatureKind::Mirror(m) => {
                let pl = self.plane(&m.plane, 0)?;
                self.caixas_replicadas(&m.sources, &[Xform::Mirror(pl.origin, pl.normal)], &mut viz, f.target)?;
            }
            _ => return Ok(None),
        }
        self.previas.push(viz);
        Ok(None)
    }

    /// Caixa da ferramenta de cada comando de origem, levada por cada transformação.
    fn caixas_replicadas(&mut self, sources: &[Id], xs: &[Xform], viz: &mut PreviaViz, alvo: Id) -> Result<(), String> {
        let k = |e: KernelError| e.0;
        if sources.is_empty() {
            let b = self.body(alvo).ok_or("não há corpo para repetir")?;
            let (lo, hi) = b.bbox().ok_or("corpo sem dimensões")?;
            let c = cantos_da_caixa(lo, hi);
            for x in xs {
                viz.caixas.push(c.map(|p| leva(x, p)));
            }
            return Ok(());
        }
        for src in sources {
            let tool = match self.steps.get(src) {
                Some(v) if v.len() > 1 => v[0].0.clone(),
                _ => self.tools.get(src).map(|t| t.0.clone()).ok_or_else(|| format!("'{}' não gera forma para repetir (ou vem depois)", self.doc.name_of(*src)))?,
            };
            let (lo, hi) = tool.bbox().ok_or_else(|| k(KernelError("ferramenta sem dimensões".into())))?;
            let c = cantos_da_caixa(lo, hi);
            for x in xs {
                viz.caixas.push(c.map(|p| leva(x, p)));
            }
        }
        Ok(())
    }

    /// As transformações de cada instância do padrão (sem a original) e a posição (i, j) de cada uma.
    pub(crate) fn instancias(&self, kind: &PatternKind) -> Result<(Vec<Xform>, Vec<(u32, u32)>), String> {
        let mut xs: Vec<Xform> = vec![];
        let mut keys: Vec<(u32, u32)> = vec![];
        match kind {
            PatternKind::Linear { dir, count, spacing, dir2, count2, spacing2, total } => {
                let n1 = crate::ev(count, &self.vars, "quantidade")?.round().max(1.) as usize;
                let mut d1 = crate::ev(spacing, &self.vars, "espaçamento")?;
                if *total && n1 > 1 {
                    d1 /= (n1 - 1) as f64;
                }
                let (_, u1) = self.axis(dir)?;
                let (n2, d2, u2) = match dir2 {
                    Some(a) => (crate::ev(count2, &self.vars, "quantidade 2")?.round().max(1.) as usize, crate::ev(spacing2, &self.vars, "espaçamento 2")?, self.axis(a)?.1),
                    None => (1, 0., [0.; 3]),
                };
                for i in 0..n1 {
                    for j in 0..n2 {
                        if i + j == 0 {
                            continue;
                        }
                        xs.push(Xform::Translate([0, 1, 2].map(|c| u1[c] * d1 * i as f64 + u2[c] * d2 * j as f64)));
                        keys.push((i as u32, j as u32));
                    }
                }
            }
            PatternKind::Circular { axis, count, angle } => {
                let n = crate::ev(count, &self.vars, "quantidade")?.round().max(1.) as usize;
                let total = crate::ev(angle, &self.vars, "ângulo")?;
                let (o, d) = self.axis(axis)?;
                let step = if (total.abs() - 360.).abs() < 1e-6 { total / n as f64 } else { total / (n.max(2) - 1) as f64 };
                for i in 1..n {
                    xs.push(Xform::Rotate(o, d, step * i as f64));
                    keys.push((i as u32, 0));
                }
            }
            PatternKind::Points { points, base } => {
                let b0 = self.point(base, 0)?;
                for (i, q) in points.iter().enumerate() {
                    let p = self.point(q, 0)?;
                    xs.push(Xform::Translate([p[0] - b0[0], p[1] - b0[1], p[2] - b0[2]]));
                    keys.push((i as u32, 0));
                }
            }
        }
        Ok((xs, keys))
    }
}
