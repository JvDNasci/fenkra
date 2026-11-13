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
//!
//! Também têm prévia leve os modos especiais do filete e do chanfro (variável, assimétrico, duas
//! distâncias, distância e ângulo: a linha de tangência no maior valor; face a face e completo: as
//! arestas entre as faces clicadas), a chapa (a zona que cada comando ocupa: os pontos de referência
//! inflados pela medida principal) e o estrutural (cada membro como a caixa orientada da seção ao
//! longo da linha guia; chapa de ligação, mão-francesa e solda como zona). Chapa e estrutural só ficam
//! leves quando são o último comando ativo: os comandos depois deles leem faces que ainda não existiriam.

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
    /// filete face a face e completo: pares de pontos em duas faces; a aresta entre elas é a afetada
    pub pares: Vec<[V3; 2]>,
}

/// Comandos que têm prévia leve.
pub fn cobre(k: &FeatureKind) -> bool {
    match k {
        // B356: todos os modos; os especiais desenham no maior valor ou entre as faces clicadas
        FeatureKind::Fillet(_) | FeatureKind::Chamfer(_) => true,
        FeatureKind::Shell(_) | FeatureKind::Pattern(_) | FeatureKind::Mirror(_) => true,
        // B356: varredura e multisseções varrem ou costuram perfis com o kernel e fazem a booleana com
        // o corpo; a prévia leve mostra só a caixa que os perfis e o caminho ocupam
        FeatureKind::Sweep(_) | FeatureKind::Loft(_) => true,
        k => so_no_fim(k),
    }
}

/// Chapa e estrutural: leves só quando são o último comando ativo (`cobre_no`). O desdobrar parcial
/// fica de fora: ele não cria material, só muda o estado em que as abas aparecem.
pub fn so_no_fim(k: &FeatureKind) -> bool {
    use FeatureKind as K;
    matches!(
        k,
        K::SmWall(_)
            | K::SmFlange(_)
            | K::SmHem(_)
            | K::SmJoggle(_)
            | K::SmBend(_)
            | K::SmLouver(_)
            | K::SmStamp(_)
            | K::SmFlangedHole(_)
            | K::SmProfileFlange(_)
            | K::SmCurveFlange(_)
            | K::SmFlangeLoop(_)
            | K::SmCornerRelief(_)
            | K::SmCornerClose(_)
            | K::SmLoftFlange(_)
            | K::SmConvert(_)
            | K::SmArcSlot(_)
            | K::SmSurfFlange(_)
            | K::StructMember(_)
            | K::StructPlate(_)
            | K::StructGusset(_)
            | K::StructWeld(_)
    )
}

/// Aba, bainha e dobra por linha: a alça de ângulo, raio e recuos lê a linha de dobra do próprio
/// comando, que só sai da construção dele. Esses três ficam leves por dentro do cálculo normal: a
/// ferramenta e a linha de dobra são feitas (peças pequenas), mas a colisão e a booleana com o corpo,
/// que custam por corpo inteiro, não. O recorte fica completo: a prévia dele mostra o contorno nos
/// dois estados, que só o cálculo real dá.
pub fn por_dentro(k: &FeatureKind) -> bool {
    matches!(k, FeatureKind::SmFlange(_) | FeatureKind::SmHem(_) | FeatureKind::SmBend(_))
}

/// O comando `f` do documento roda na versão leve quando aberto? Igual a `cobre`, menos chapa e
/// estrutural que não são o último comando ativo.
pub fn cobre_no(doc: &fenkra_core::Document, f: &Feature) -> bool {
    if !cobre(&f.kind) {
        return false;
    }
    if !so_no_fim(&f.kind) {
        return true;
    }
    let Some(i) = doc.features.iter().position(|g| g.id == f.id) else { return false };
    !(i + 1..doc.features.len()).any(|k| doc.is_active(k))
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

/// Caixa alinhada aos eixos que envolve os pontos (`None` sem pontos).
pub fn caixa_dos_pontos(pts: &[V3]) -> Option<[V3; 8]> {
    let p0 = *pts.first()?;
    let (mut lo, mut hi) = (p0, p0);
    for p in pts {
        for i in 0..3 {
            lo[i] = lo[i].min(p[i]);
            hi[i] = hi[i].max(p[i]);
        }
    }
    Some(cantos_da_caixa(lo, hi))
}

/// A caixa (de `cantos_da_caixa`) crescida `r` para cada lado.
pub fn inflar(c: [V3; 8], r: f64) -> [V3; 8] {
    let (lo, hi) = (c[0], c[6]);
    cantos_da_caixa([lo[0] - r, lo[1] - r, lo[2] - r], [hi[0] + r, hi[1] + r, hi[2] + r])
}

/// Caixa orientada do membro: a caixa da seção (já deslocada pela ancoragem) nos eixos x e y do
/// membro, da ponta inicial menos o prolongamento até a final mais o prolongamento. Mesmos eixos do
/// `member_shape`, então a caixa cobre o perfil final. Membro curvo: a caixa da curva inflada pela
/// meia diagonal da seção.
pub(crate) fn caixa_do_membro(g: &crate::MemberGeom) -> [V3; 8] {
    use crate::geo3d::{add, mul, sub};
    if !g.straight {
        let r = (g.w * g.w + g.h * g.h).sqrt() / 2.;
        return inflar(caixa_dos_pontos(&g.pts).unwrap_or_else(|| cantos_da_caixa(g.start, g.end)), r);
    }
    let ([x0, y0], [x1, y1]) = caixa_da_secao(&crate::structural::shifted(&g.loops, g.shift));
    let o = sub(g.start, mul(g.z, g.ext.0));
    let l = crate::geo3d::dist(g.start, g.end) + g.ext.0 + g.ext.1;
    let p = |x: f64, y: f64, z: f64| add(add(o, mul(g.x, x)), add(mul(g.y, y), mul(g.z, z)));
    [p(x0, y0, 0.), p(x1, y0, 0.), p(x1, y1, 0.), p(x0, y1, 0.), p(x0, y0, l), p(x1, y0, l), p(x1, y1, l), p(x0, y1, l)]
}

/// Caixa 2D dos laços da seção (mínimo, máximo).
fn caixa_da_secao(loops: &[fenkra_kernel::Loop2]) -> ([f64; 2], [f64; 2]) {
    use fenkra_kernel::Seg2;
    let (mut lo, mut hi) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
    let mut mais = |p: [f64; 2], r: f64| {
        for i in 0..2 {
            lo[i] = lo[i].min(p[i] - r);
            hi[i] = hi[i].max(p[i] + r);
        }
    };
    for s in loops.iter().flatten() {
        match s {
            Seg2::Line(a, b) => [*a, *b].into_iter().for_each(|p| mais(p, 0.)),
            Seg2::Arc(a, b, c) => [*a, *b, *c].into_iter().for_each(|p| mais(p, 0.)),
            Seg2::Circle(c, r) => mais(*c, *r),
            Seg2::Spline(ps) => ps.iter().for_each(|p| mais(*p, 0.)),
            Seg2::Ellipse(c, u, b, _, _) => mais(*c, (u[0] * u[0] + u[1] * u[1]).sqrt().max(*b)),
        }
    }
    if lo[0] > hi[0] {
        return ([0.; 2], [0.; 2]);
    }
    (lo, hi)
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
        if self.leve != Some(f.id) || !cobre_no(self.doc, f) || por_dentro(&f.kind) {
            return None;
        }
        Some(self.previa_leve_de(f))
    }

    fn previa_leve_de(&mut self, f: &Feature) -> Result<Option<String>, String> {
        let mut viz = PreviaViz { feature: f.id, arestas: vec![], valor: 0., chanfro: false, todas: false, caixas: vec![], pares: vec![] };
        match &f.kind {
            FeatureKind::Fillet(o) | FeatureKind::Chamfer(o) => {
                if matches!(&o.edges, EdgeSel::Edges(m) if m.is_empty()) {
                    return Ok(Some("clique nas arestas na viewport (ou escolha 'Todas') — até lá nada é aplicado".into()));
                }
                viz.valor = crate::ev(&o.value, &self.vars, "valor")?;
                viz.chanfro = matches!(f.kind, FeatureKind::Chamfer(_));
                // modos especiais: a linha de tangência vai no maior valor, que é até onde o material muda
                let segundo = |e: &str, nome: &str| crate::ev(e, &self.vars, nome);
                match &o.mode {
                    EdgeMode::Constant => {}
                    EdgeMode::Variable(e) => viz.valor = viz.valor.max(segundo(e, "raio final")?),
                    EdgeMode::Asym(e) => viz.valor = viz.valor.max(segundo(e, "raio da 2ª face")?),
                    EdgeMode::TwoDist(e) => viz.valor = viz.valor.max(segundo(e, "segunda distância")?),
                    EdgeMode::DistAngle(e) => {
                        let a = segundo(e, "ângulo")?;
                        if a <= 0. || a >= 90. {
                            return Err(format!("ângulo {a}° fora de (0°, 90°)"));
                        }
                        viz.valor = viz.valor.max(viz.valor * a.to_radians().tan());
                    }
                    // a aresta comum às faces clicadas; a interface a acha na malha pelas duas faces
                    EdgeMode::FaceFace(a, b) => {
                        viz.pares.push([*a, *b]);
                        self.previas.push(viz);
                        return Ok(None);
                    }
                    EdgeMode::FullRound { mid, a, b } => {
                        viz.pares.extend([[*mid, *a], [*mid, *b]]);
                        self.previas.push(viz);
                        return Ok(None);
                    }
                }
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
            FeatureKind::Sweep(w) => {
                let mut pts = self.pontos_do_sketch(w.sketch).ok_or("escolha o perfil da varredura")?;
                match &w.spine {
                    fenkra_core::CurveRef::SketchLine { sketch, .. } | fenkra_core::CurveRef::SketchCurve { sketch, .. } => pts.extend(self.pontos_do_sketch(*sketch).unwrap_or_default()),
                    fenkra_core::CurveRef::Edge(p) | fenkra_core::CurveRef::BodyEdge { pts: p, .. } => pts.extend(p.iter().copied()),
                    // curva de referência: o caminho exato só sai do kernel; a caixa fica com o perfil
                    _ => {}
                }
                viz.caixas.push(caixa_dos_pontos(&pts).ok_or("perfil vazio")?);
            }
            FeatureKind::Loft(l) => {
                if l.sections.len() < 2 {
                    return Ok(Some("escolha pelo menos duas seções — até lá nada é aplicado".into()));
                }
                let mut pts = vec![];
                for s in &l.sections {
                    pts.extend(self.pontos_do_sketch(*s).ok_or_else(|| format!("a seção '{}' não está disponível", self.doc.name_of(*s)))?);
                }
                viz.caixas.push(caixa_dos_pontos(&pts).ok_or("seções vazias")?);
            }
            k if so_no_fim(k) => self.zona_chapa_estrutural(f, &mut viz)?,
            _ => return Ok(None),
        }
        self.previas.push(viz);
        Ok(None)
    }

    /// A polilinha inteira de cada aresta clicada (o clique guarda só um ponto dela). Consulta de
    /// topologia, sem construir nada; sem corpo ou sem aresta, fica o próprio ponto.
    fn arestas_inteiras(&self, alvo: Id, pts: &[V3]) -> Vec<V3> {
        let b = self.body(alvo);
        pts.iter().flat_map(|p| b.and_then(|b| b.edge_at(*p).ok()).filter(|v| !v.is_empty()).unwrap_or_else(|| vec![*p])).collect()
    }

    /// Aba, bainha ou dobra aberta para edição, na versão leve por dentro do cálculo (`por_dentro`).
    pub(crate) fn leve_dentro(&self, f: &Feature) -> bool {
        self.leve == Some(f.id) && por_dentro(&f.kind) && cobre_no(self.doc, f)
    }

    /// Prévia da aba leve: a caixa da própria ferramenta (a parede nova, com a dobra), sem a booleana.
    pub(crate) fn previa_da_ferramenta(&mut self, f: &Feature, tool: &fenkra_kernel::Shape) {
        if let Some((lo, hi)) = tool.bbox() {
            let mut viz = PreviaViz { feature: f.id, arestas: vec![], valor: 0., chanfro: false, todas: false, caixas: vec![cantos_da_caixa(lo, hi)], pares: vec![] };
            viz.valor = (0..3).map(|i| hi[i] - lo[i]).fold(0., f64::max);
            self.previas.push(viz);
        }
    }

    /// Prévia da dobra por linha leve: a zona da linha, crescida pelo raio e a espessura.
    pub(crate) fn previa_da_dobra(&mut self, f: &Feature, pts: &[V3], r: f64) {
        if let Some(c) = caixa_dos_pontos(pts) {
            self.previas.push(PreviaViz { feature: f.id, arestas: vec![], valor: r, chanfro: false, todas: false, caixas: vec![inflar(c, r)], pares: vec![] });
        }
    }

    /// Valor de um campo; vazio ou inválido conta como zero (só para o tamanho da zona).
    fn num(&self, e: &str) -> f64 {
        crate::ev(e, &self.vars, "").unwrap_or(0.).abs()
    }

    /// Chapa e estrutural: membro como caixa orientada; o resto como a zona dos pontos de referência
    /// inflada pela medida principal do comando.
    fn zona_chapa_estrutural(&mut self, f: &Feature, viz: &mut PreviaViz) -> Result<(), String> {
        use FeatureKind as K;
        let (t, rd) = self.sheet().map(|s| (s.t, s.r)).unwrap_or((1., 1.));
        // a aba sobe o comprimento mais a espessura e a dobra: a zona cobre a parede inteira
        let aba = |l: f64, r: &Option<String>| l + t + r.as_deref().map_or(rd, |e| crate::ev(e, &self.vars, "").unwrap_or(rd).abs());
        let esboco = |cx: &Self, id: Id| cx.pontos_do_sketch(id).unwrap_or_default();
        let (pts, r): (Vec<V3>, f64) = match &f.kind {
            K::StructMember(m) => {
                for g in self.members(m)? {
                    viz.caixas.push(caixa_do_membro(&g));
                }
                return Ok(());
            }
            K::SmWall(w) => (esboco(self, w.sketch), t),
            K::SmJoggle(x) => (vec![x.a, x.b], self.num(&x.depth).max(self.num(&x.run)).max(t)),
            K::SmLouver(x) => (vec![x.a, x.b], self.num(&x.width).max(self.num(&x.height)).max(t)),
            K::SmStamp(x) => (esboco(self, x.sketch), self.num(&x.depth).max(t)),
            K::SmFlangedHole(x) => {
                let mut p = x.points.clone();
                if p.is_empty() {
                    p = esboco(self, x.sketch);
                }
                (p, (self.num(&x.diameter) / 2.).max(self.num(&x.height)).max(t))
            }
            K::SmProfileFlange(x) => (esboco(self, x.sketch), aba(self.num(&x.l1).max(self.num(&x.l2)), &x.radius)),
            K::SmCurveFlange(x) => (x.edge.clone(), aba(self.num(&x.length), &x.radius)),
            K::SmFlangeLoop(x) => (self.arestas_inteiras(f.target, &x.edges), aba(self.num(&x.length), &x.radius)),
            K::SmCornerRelief(x) => (vec![x.at], self.num(&x.size).max(t)),
            K::SmCornerClose(x) => (vec![x.at], self.num(&x.gap).max(3. * t)),
            K::SmLoftFlange(x) => ([esboco(self, x.a), esboco(self, x.b)].concat(), t),
            K::SmArcSlot(x) => (vec![x.center], self.num(&x.radius) + self.num(&x.width) / 2.),
            K::SmSurfFlange(x) => (vec![x.face], aba(self.num(&x.length), &x.radius)),
            // converter em chapa refaz o corpo todo: a zona é o próprio corpo
            K::SmConvert(_) => {
                let b = self.body(f.target).ok_or("não há corpo para converter")?;
                let (lo, hi) = b.bbox().ok_or("corpo sem dimensões")?;
                (vec![lo, hi], 0.)
            }
            K::StructPlate(x) => (vec![x.at], self.num(&x.margin) + self.num(&x.thickness)),
            K::StructGusset(x) => (vec![x.at], self.num(&x.leg).max(self.num(&x.leg2))),
            K::StructWeld(x) => (x.faces.clone(), self.num(&x.leg)),
            _ => return Ok(()),
        };
        let c = caixa_dos_pontos(&pts).ok_or("escolha onde o comando age — até lá nada é aplicado")?;
        viz.caixas.push(inflar(c, r));
        viz.valor = r;
        Ok(())
    }

    /// Pontos do desenho de um sketch já avaliado, no espaço (sem kernel).
    fn pontos_do_sketch(&self, id: Id) -> Option<Vec<V3>> {
        let (fr, es) = self.sketches.get(&id)?;
        Some(es.viz.iter().filter(|(cons, _)| !cons).flat_map(|(_, l)| l.iter().map(|q| fr.point(q[0], q[1]))).collect())
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
