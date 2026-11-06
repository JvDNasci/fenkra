// SPDX-License-Identifier: LGPL-2.1-or-later
//! B91: padrões com re-solução. Cada instância tem uma posição (i, j); a pessoa pula instâncias por
//! posição e a lista de puladas sobrevive a mudar quantidade, espaçamento ou ângulo.
//!
//! Como funciona no kernel: o padrão gera uma transformação por instância (translação ou rotação) e
//! o recompute copia a forma de cada comando de origem em cada transformação, unindo ao corpo.
//! Pular uma instância é só não gerar a cópia dela. Como a chave é a posição, ao aumentar a
//! quantidade as puladas continuam nos mesmos lugares; ao diminuir, as que ficaram fora são
//! ignoradas e o comando avisa, sem falhar.

use super::*;

/// Uma instância do padrão, para a prévia desenhar um ponto clicável por instância.
#[derive(Clone, Debug, PartialEq)]
pub struct InstanciaPadrao {
    /// posição (i, j); a original é (0, 0) e não se pula
    pub chave: (u32, u32),
    /// onde a instância cai (o centro da caixa envolvente da origem, levado pela transformação)
    pub centro: V3,
    pub pulada: bool,
}

/// Tira as instâncias puladas. Devolve as transformações restantes e o aviso das posições puladas
/// que não existem mais.
pub(crate) fn sem_puladas(xs: Vec<Xform>, keys: &[(u32, u32)], skip: &[(u32, u32)]) -> (Vec<Xform>, Option<String>) {
    let fantasmas: Vec<String> = skip.iter().filter(|s| !keys.contains(s) && **s != (0, 0)).map(|(i, j)| if *j == 0 { format!("{}", i + 1) } else { format!("({},{})", i + 1, j + 1) }).collect();
    let mantidas: Vec<Xform> = xs.into_iter().zip(keys).filter(|(_, k)| !skip.contains(k)).map(|(x, _)| x).collect();
    let aviso = (!fantasmas.is_empty()).then(|| format!("instâncias puladas que não existem mais (quantidade menor): {}", fantasmas.join(", ")));
    (mantidas, aviso)
}

fn aplica(x: &Xform, p: V3) -> V3 {
    match x {
        Xform::Translate(t) => [p[0] + t[0], p[1] + t[1], p[2] + t[2]],
        Xform::Rotate(o, d, ang) => {
            let (s, c) = ang.to_radians().sin_cos();
            let d = geo3d::unit(*d).unwrap_or([0., 0., 1.]);
            let v = geo3d::sub(p, *o);
            let kv = geo3d::cross(d, v);
            let kd = geo3d::dot(d, v);
            [0, 1, 2].map(|i| o[i] + v[i] * c + kv[i] * s + d[i] * kd * (1. - c))
        }
        _ => p,
    }
}

/// Instâncias do padrão `id` (a original primeiro), com a posição de cada uma e se está pulada.
/// A prévia desenha um ponto clicável em cada uma; clicar acrescenta a chave a `Pattern::skip`.
pub fn instancias_do_padrao(doc: &Document, id: Id) -> Vec<InstanciaPadrao> {
    let Some(Feature { kind: FeatureKind::Pattern(pt), .. }) = doc.feature(id) else { return vec![] };
    // sem o próprio padrão no cálculo, o corpo é só a origem das cópias: o centro dele é o das instâncias
    let mut d = doc.clone();
    if let Some(f) = d.feature_mut(id) {
        f.suppressed = true;
    }
    let centro0 = origem_do_padrao(&d).unwrap_or([0.; 3]);
    let Some(chaves_xs) = transformacoes(doc, pt) else { return vec![] };
    let mut v = vec![InstanciaPadrao { chave: (0, 0), centro: centro0, pulada: false }];
    for (k, x) in chaves_xs {
        v.push(InstanciaPadrao { chave: k, centro: aplica(&x, centro0), pulada: pt.skip.contains(&k) });
    }
    v
}

/// Bocas dos furos que o padrão `id` repete: as dos furos de origem e as de cada instância, com a
/// marca de pulada. A tabela de furos só conhece os furos de origem, porque o padrão copia a forma da
/// ferramenta sem rodar o comando de novo. Padrão que não repete furo devolve lista vazia.
pub fn furos_do_padrao(doc: &Document, id: Id) -> Vec<InstanciaPadrao> {
    let Some(Feature { kind: FeatureKind::Pattern(pt), .. }) = doc.feature(id) else { return vec![] };
    let bocas: Vec<V3> = hole_table(doc).into_iter().filter(|r| pt.sources.contains(&r.feature)).map(|r| r.pos).collect();
    let Some(chaves_xs) = transformacoes(doc, pt).filter(|_| !bocas.is_empty()) else { return vec![] };
    let mut v: Vec<InstanciaPadrao> = bocas.iter().map(|p| InstanciaPadrao { chave: (0, 0), centro: *p, pulada: false }).collect();
    for (k, x) in chaves_xs {
        v.extend(bocas.iter().map(|p| InstanciaPadrao { chave: k, centro: aplica(&x, *p), pulada: pt.skip.contains(&k) }));
    }
    v
}

/// Centro de onde sai o padrão: o centro da caixa envolvente do corpo sem o padrão.
fn origem_do_padrao(doc: &Document) -> Option<V3> {
    let mut lo = [f64::MAX; 3];
    let mut hi = [f64::MIN; 3];
    let mut algum = false;
    let r = recompute(doc);
    for b in &r.bodies {
        for p in b.mesh.positions.chunks_exact(3) {
            for i in 0..3 {
                lo[i] = lo[i].min(p[i] as f64);
                hi[i] = hi[i].max(p[i] as f64);
            }
            algum = true;
        }
    }
    algum.then(|| geo3d::lerp(lo, hi, 0.5))
}

/// Transformações do padrão por posição, fora a original, sem avaliar o corpo.
fn transformacoes(doc: &Document, pt: &Pattern) -> Option<Vec<((u32, u32), Xform)>> {
    let vars = doc.eval_params().ok()?;
    let cx = contexto_vazio(doc, vars);
    let mut out = vec![];
    match &pt.kind {
        PatternKind::Linear { dir, count, spacing, dir2, count2, spacing2, total } => {
            let n1 = ev(count, &cx.vars, "quantidade").ok()?.round().max(1.) as usize;
            let mut d1 = ev(spacing, &cx.vars, "espaçamento").ok()?;
            if *total && n1 > 1 {
                d1 /= (n1 - 1) as f64;
            }
            let (_, u1) = cx.axis(dir).ok()?;
            let (n2, d2, u2) = match dir2 {
                Some(a) => (ev(count2, &cx.vars, "quantidade 2").ok()?.round().max(1.) as usize, ev(spacing2, &cx.vars, "espaçamento 2").ok()?, cx.axis(a).ok()?.1),
                None => (1, 0., [0.; 3]),
            };
            for i in 0..n1 {
                for j in 0..n2 {
                    if i + j > 0 {
                        out.push(((i as u32, j as u32), Xform::Translate([0, 1, 2].map(|c| u1[c] * d1 * i as f64 + u2[c] * d2 * j as f64))));
                    }
                }
            }
        }
        PatternKind::Circular { axis, count, angle } => {
            let n = ev(count, &cx.vars, "quantidade").ok()?.round().max(1.) as usize;
            let total = ev(angle, &cx.vars, "ângulo").ok()?;
            let (o, d) = cx.axis(axis).ok()?;
            let step = if (total.abs() - 360.).abs() < 1e-6 { total / n as f64 } else { total / (n.max(2) - 1) as f64 };
            for i in 1..n {
                out.push(((i as u32, 0), Xform::Rotate(o, d, step * i as f64)));
            }
        }
        PatternKind::Points { points, base } => {
            let b0 = cx.point(base, 0).ok()?;
            for (i, q) in points.iter().enumerate() {
                let p = cx.point(q, 0).ok()?;
                out.push(((i as u32, 0), Xform::Translate([p[0] - b0[0], p[1] - b0[1], p[2] - b0[2]])));
            }
        }
    }
    Some(out)
}

fn contexto_vazio<'a>(doc: &'a Document, vars: Vars) -> Ctx<'a> {
    Ctx {
        doc,
        vars,
        qualidade: Default::default(),
        bodies: HashMap::new(),
        sketches: HashMap::new(),
        status: HashMap::new(),
        tools: HashMap::new(),
        memo: RefCell::new(HashMap::new()),
        notes: Default::default(),
        body_sig: HashMap::new(),
        last_sheet: None,
        sheet_maps: vec![],
        cut_outlines: vec![],
        malhas: HashMap::new(),
        roscas: vec![],
        relatorios_forma: vec![],
        formas_resolvidas: vec![],
        steps: HashMap::new(),
        miter_next: false,
        promoted: RefCell::new(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pular_tira_so_a_posicao_pedida_e_avisa_a_que_sumiu() {
        let xs = vec![Xform::Translate([10., 0., 0.]), Xform::Translate([20., 0., 0.]), Xform::Translate([30., 0., 0.])];
        let keys = [(1, 0), (2, 0), (3, 0)];
        let (m, a) = sem_puladas(xs.clone(), &keys, &[(2, 0)]);
        assert_eq!(m.len(), 2);
        assert!(a.is_none());
        let (m, a) = sem_puladas(xs, &keys, &[(7, 0)]);
        assert_eq!(m.len(), 3);
        assert!(a.unwrap().contains('8'));
    }
}
