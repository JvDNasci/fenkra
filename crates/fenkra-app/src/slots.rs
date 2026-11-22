// SPDX-License-Identifier: LGPL-2.1-or-later
//! Campos de referência vazios: um comando novo só recebe o que estava pré-selecionado; o resto
//! nasce "a escolher" (um id sentinela por tipo) e é preenchido pela próxima seleção no 3D ou na
//! árvore, na ordem dos campos.

use crate::geo_ui::{axis_of, Seed};
use fenkra_core::*;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use std::collections::VecDeque;

pub const UNSET_POINT: Id = Id(u64::MAX - 11);
pub const UNSET_CURVE: Id = Id(u64::MAX - 12);
pub const UNSET_PLANE: Id = Id(u64::MAX - 13);
pub const UNSET_SURF: Id = Id(u64::MAX - 14);
pub const UNSET_AXIS: Id = Id(u64::MAX - 15);
pub const UNSET_SKETCH: Id = Id(u64::MAX - 16);

pub fn point() -> PointRef {
    PointRef::Ref(UNSET_POINT)
}
pub fn curve() -> CurveRef {
    CurveRef::Ref(UNSET_CURVE)
}
pub fn plane() -> PlaneRef {
    PlaneRef::Datum(UNSET_PLANE)
}
pub fn surf() -> SurfRef {
    SurfRef::Ref(UNSET_SURF)
}
pub fn axis() -> AxisRef {
    AxisRef::Ref(UNSET_AXIS)
}

pub fn is_unset(id: Id) -> bool {
    id.a_escolher()
}

/// Texto do campo vazio (pede a referência certa).
pub fn label(id: Id) -> Option<String> {
    let what = match id {
        x if x == UNSET_POINT => "um vértice, ponto, aresta circular (centro) ou face",
        x if x == UNSET_CURVE => "uma aresta ou curva",
        x if x == UNSET_PLANE => "um plano, face plana, sketch ou curva plana",
        x if x == UNSET_SURF => "uma face ou superfície",
        x if x == UNSET_AXIS => "uma aresta, linha, eixo, face plana (normal) ou cilíndrica (eixo)",
        x if x == UNSET_SKETCH => "um sketch ou curva fechada do wireframe",
        _ => return None,
    };
    Some(format!("⊙ {} {}", crate::i18n::tr("selecione"), crate::i18n::tr(what)))
}

fn walk_count(v: &Value, n: &mut usize) {
    match v {
        Value::Number(x) => {
            if x.as_u64().is_some_and(|u| is_unset(Id(u))) {
                *n += 1;
            }
        }
        Value::Object(m) => m.values().for_each(|x| walk_count(x, n)),
        Value::Array(a) => a.iter().for_each(|x| walk_count(x, n)),
        _ => {}
    }
}

/// Quantos campos ainda estão vazios.
pub fn count_unset<T: Serialize>(k: &T) -> usize {
    let mut n = 0;
    if let Ok(v) = serde_json::to_value(k) {
        walk_count(&v, &mut n);
    }
    n
}

/// Candidatos vindos da seleção (consumidos na ordem dos campos).
#[derive(Default)]
pub struct Cands {
    pub points: VecDeque<PointRef>,
    pub curves: VecDeque<CurveRef>,
    pub planes: VecDeque<PlaneRef>,
    pub surfs: VecDeque<SurfRef>,
    pub sketches: VecDeque<Id>,
    /// alternativas tiradas da geometria (quando falta o tipo exato): pontos (centro de círculo,
    /// ponto clicado na face), eixos (normal de face plana, eixo de cilindro, eixo de círculo),
    /// planos (o plano de um sketch)
    pub alt_points: VecDeque<PointRef>,
    pub alt_axes: VecDeque<AxisRef>,
    pub alt_planes: VecDeque<PlaneRef>,
}

impl Cands {
    pub fn from_seed(s: &Seed, sketch: Option<Id>) -> Self {
        let mut surfs: VecDeque<SurfRef> = s.surface.into_iter().collect();
        surfs.extend(s.planes.iter().map(|p| SurfRef::Plane(*p)));
        Cands {
            points: s.points.iter().copied().collect(),
            curves: s.curves.iter().cloned().collect(),
            planes: s.planes.iter().copied().collect(),
            surfs,
            sketches: sketch.into_iter().collect(),
            ..Default::default()
        }
    }
}

/// B466: a forma do comando na ordem em que os campos foram declarados. O `serde_json::Value` guarda
/// os objetos num mapa ordenado por nome (sem o recurso `preserve_order`), então percorrer o `Value`
/// visita os campos em ordem alfabética: na varredura cônica `apex` vinha antes de `guide1`, e o
/// primeiro clique caía na "Guia do ápice". A forma é lida do texto JSON, que o serde escreve na
/// ordem da declaração, e diz em que ordem visitar as chaves do `Value`.
#[derive(Debug, Default)]
pub(crate) enum Forma {
    #[default]
    Folha,
    Lista(Vec<Forma>),
    Objeto(Vec<(String, Forma)>),
}

impl<'de> serde::Deserialize<'de> for Forma {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> serde::de::Visitor<'de> for V {
            type Value = Forma;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("JSON")
            }
            fn visit_bool<E>(self, _: bool) -> Result<Forma, E> {
                Ok(Forma::Folha)
            }
            fn visit_i64<E>(self, _: i64) -> Result<Forma, E> {
                Ok(Forma::Folha)
            }
            fn visit_u64<E>(self, _: u64) -> Result<Forma, E> {
                Ok(Forma::Folha)
            }
            fn visit_f64<E>(self, _: f64) -> Result<Forma, E> {
                Ok(Forma::Folha)
            }
            fn visit_str<E>(self, _: &str) -> Result<Forma, E> {
                Ok(Forma::Folha)
            }
            fn visit_unit<E>(self) -> Result<Forma, E> {
                Ok(Forma::Folha)
            }
            fn visit_none<E>(self) -> Result<Forma, E> {
                Ok(Forma::Folha)
            }
            fn visit_some<D: serde::Deserializer<'de>>(self, d: D) -> Result<Forma, D::Error> {
                serde::Deserialize::deserialize(d)
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut a: A) -> Result<Forma, A::Error> {
                let mut v = vec![];
                while let Some(x) = a.next_element()? {
                    v.push(x);
                }
                Ok(Forma::Lista(v))
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(self, mut a: A) -> Result<Forma, A::Error> {
                let mut v = vec![];
                while let Some((k, x)) = a.next_entry::<String, Forma>()? {
                    v.push((k, x));
                }
                Ok(Forma::Objeto(v))
            }
        }
        d.deserialize_any(V)
    }
}

impl Forma {
    pub(crate) fn de<T: Serialize>(k: &T) -> Forma {
        serde_json::to_string(k).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
    }
}

/// Chaves do objeto `m` na ordem da forma (as que a forma não conhece vão no fim, em ordem de nome).
fn chaves_em_ordem<'a>(m: &serde_json::Map<String, Value>, f: &'a Forma) -> Vec<(String, Option<&'a Forma>)> {
    let mut out: Vec<(String, Option<&Forma>)> = match f {
        Forma::Objeto(v) => v.iter().filter(|(k, _)| m.contains_key(k)).map(|(k, x)| (k.clone(), Some(x))).collect(),
        _ => vec![],
    };
    for k in m.keys() {
        if !out.iter().any(|(j, _)| j == k) {
            out.push((k.clone(), None));
        }
    }
    out
}

/// Sentinelas dos campos vazios na ordem da declaração (a do painel).
pub fn vazios_em_ordem<T: Serialize>(k: &T) -> Vec<u64> {
    fn walk(v: &Value, f: &Forma, out: &mut Vec<u64>) {
        match v {
            Value::Number(x) => out.extend(x.as_u64().filter(|u| is_unset(Id(*u)))),
            Value::Object(m) => {
                for (k, sub) in chaves_em_ordem(m, f) {
                    walk(&m[&k], sub.unwrap_or(&Forma::Folha), out);
                }
            }
            Value::Array(a) => {
                let fs = if let Forma::Lista(l) = f { l.as_slice() } else { &[] };
                for (i, x) in a.iter().enumerate() {
                    walk(x, fs.get(i).unwrap_or(&Forma::Folha), out);
                }
            }
            _ => {}
        }
    }
    let mut out = vec![];
    if let Ok(v) = serde_json::to_value(k) {
        walk(&v, &Forma::de(k), &mut out);
    }
    out
}

fn walk_fill(v: &mut Value, f: &Forma, c: &mut Cands, changed: &mut bool) {
    let rep: Option<Value> = match v {
        // perfil: sketch; ou a curva do wireframe; ou o sketch da curva clicada
        Value::Number(x) if x.as_u64() == Some(UNSET_SKETCH.0) => c
            .sketches
            .pop_front()
            .or_else(|| {
                let i = c.curves.iter().position(|k| matches!(k, CurveRef::Ref(_) | CurveRef::SketchCurve { .. } | CurveRef::SketchLine { .. }))?;
                match c.curves.remove(i)? {
                    CurveRef::Ref(id) => Some(id),
                    CurveRef::SketchCurve { sketch, .. } | CurveRef::SketchLine { sketch, .. } => Some(sketch),
                    _ => None,
                }
            })
            .or_else(|| {
                let i = c.points.iter().position(|k| matches!(k, PointRef::SketchPoint { .. }))?;
                match c.points.remove(i)? {
                    PointRef::SketchPoint { sketch, .. } => Some(sketch),
                    _ => None,
                }
            })
            .map(|s| Value::from(s.0)),
        Value::Object(m) if m.len() == 1 => match m.values().next().and_then(|x| x.as_u64()) {
            Some(u) if u == UNSET_POINT.0 => c.points.pop_front().or_else(|| c.alt_points.pop_front()).and_then(|p| serde_json::to_value(p).ok()),
            Some(u) if u == UNSET_CURVE.0 => c.curves.pop_front().and_then(|p| serde_json::to_value(p).ok()),
            Some(u) if u == UNSET_PLANE.0 => c.planes.pop_front().or_else(|| c.alt_planes.pop_front()).and_then(|p| serde_json::to_value(p).ok()),
            Some(u) if u == UNSET_SURF.0 => c.surfs.pop_front().and_then(|p| serde_json::to_value(p).ok()),
            // eixo: aresta/linha (círculo = seu eixo); senão a normal da face plana ou o eixo do cilindro
            Some(u) if u == UNSET_AXIS.0 => c
                .curves
                .pop_front()
                .map(|p| circle_axis(&p).unwrap_or_else(|| axis_of(&p)))
                .or_else(|| c.alt_axes.pop_front())
                .and_then(|p| serde_json::to_value(p).ok()),
            _ => None,
        },
        _ => None,
    };
    if let Some(r) = rep {
        *v = r;
        *changed = true;
        return;
    }
    match v {
        Value::Object(m) => {
            for (k, sub) in chaves_em_ordem(m, f) {
                if let Some(x) = m.get_mut(&k) {
                    walk_fill(x, sub.unwrap_or(&Forma::Folha), c, changed);
                }
            }
        }
        Value::Array(a) => {
            let fs = if let Forma::Lista(l) = f { l.as_slice() } else { &[] };
            for (i, x) in a.iter_mut().enumerate() {
                walk_fill(x, fs.get(i).unwrap_or(&Forma::Folha), c, changed);
            }
        }
        _ => {}
    }
}

/// Aresta fechada (círculo) usada como eixo: o eixo do círculo (centro e normal), não a corda.
fn circle_axis(c: &CurveRef) -> Option<AxisRef> {
    let p = c.edge_pts()?;
    let (ctr, n, _) = crate::geom_fit::circle(p)?;
    Some(match c {
        CurveRef::BodyEdge { key, .. } => AxisRef::Edge { mid: p[p.len() / 2], key: Some(*key) },
        _ => AxisRef::Fixed { origin: ctr, dir: n },
    })
}

/// Preenche os campos vazios com os candidatos. Devolve se mudou algo.
pub fn fill<T: Serialize + DeserializeOwned>(k: &mut T, c: &mut Cands) -> bool {
    let Ok(mut v) = serde_json::to_value(&*k) else { return false };
    let mut changed = false;
    walk_fill(&mut v, &Forma::de(&*k), c, &mut changed);
    if changed {
        if let Ok(n) = serde_json::from_value(v) {
            *k = n;
            return true;
        }
    }
    false
}

/// Campos de lista que não são referências (centros do furo, caminhos do membro, seções do loft,
/// fontes de padrão e espelho). Os campos de coordenada (aresta da aba, face da rosca, linha da
/// dobra) saem do descritor: `contrato::preencher`.
pub fn fill_special(k: &mut FeatureKind, s: &Seed, sketch: Option<Id>, shape: Option<Id>) -> bool {
    match k {
        FeatureKind::Hole(h) if h.sketch == UNSET_SKETCH && h.points.is_empty() && !s.points.is_empty() => {
            h.points = s.points.clone();
            h.sketch = Id(0);
            true
        }
        FeatureKind::StructMember(m) if m.paths.is_empty() && !s.curves.is_empty() => {
            m.paths = s.curves.clone();
            true
        }
        FeatureKind::Loft(l) => match sketch {
            Some(sk) if !l.sections.contains(&sk) => {
                l.sections.push(sk);
                true
            }
            _ => false,
        },
        FeatureKind::Pattern(p) => match shape {
            Some(f) if !p.sources.contains(&f) => {
                p.sources.push(f);
                true
            }
            _ => false,
        },
        FeatureKind::Mirror(m) => match shape {
            Some(f) if !m.sources.contains(&f) => {
                m.sources.push(f);
                true
            }
            _ => false,
        },
        _ => false,
    }
}

/// Comandos de chapa que pegam a aresta direto do clique no 3D, com o comando aberto.
pub fn recebe_aresta_clicada(k: &FeatureKind) -> bool {
    matches!(k, FeatureKind::SmFlange(_) | FeatureKind::SmHem(_) | FeatureKind::SmFlangeLoop(_) | FeatureKind::SmCurveFlange(_))
}

/// B514: clique numa aresta com um comando de chapa aberto. Aba e bainha trocam a aresta; o laço de
/// abas soma a aresta clicada (clicar de novo numa escolhida a tira, como nas outras listas); a aba
/// curva troca a cadeia tangente (`cadeia` só é calculada para ela). A chave (dono e papel das
/// faces) acompanha o ponto. Devolve se mudou algo.
///
/// Teste da alpha.38: a aba aceita aresta que não é reta. Com a aba de aresta aberta, clicar numa
/// aresta curva (`reta` falso) troca o recurso pela aba curva, com os mesmos comprimento, ângulo,
/// raio, lado e sentido, e a aresta clicada só (sem as tangentes).
pub fn aresta_clicada(k: &mut FeatureKind, mid: [f64; 3], key: Option<EdgeKey>, reta: bool, cadeia: impl FnOnce(bool) -> Vec<[f64; 3]>) -> bool {
    let z = crate::contrato::PONTO_VAZIO;
    let perto = |a: &[f64; 3]| (0..3).map(|i| (a[i] - mid[i]).powi(2)).sum::<f64>().sqrt() < 0.01;
    match k {
        FeatureKind::SmFlange(f) if !reta => {
            let pts = cadeia(true);
            if pts.len() < 2 {
                return false;
            }
            *k = FeatureKind::SmCurveFlange(aba_curva_de(f, pts, key));
            true
        }
        FeatureKind::SmFlange(f) => {
            f.edge = mid;
            f.edge_key = key;
            true
        }
        FeatureKind::SmHem(h) => {
            h.edge = mid;
            h.edge_key = key;
            true
        }
        FeatureKind::SmFlangeLoop(l) => {
            // as chaves andam junto com as arestas (uma por aresta; faltando, sem chave)
            let mut pares: Vec<([f64; 3], Option<EdgeKey>)> = l.edges.iter().enumerate().filter(|(_, e)| !crate::contrato::ponto_vazio(e)).map(|(i, e)| (*e, l.edge_keys.get(i).copied().flatten())).collect();
            let antes = pares.len();
            match pares.iter().position(|(e, _)| perto(e)) {
                Some(i) => {
                    pares.remove(i);
                }
                None => pares.push((mid, key)),
            }
            // como na criação: uma aresta só vale pelo laço da face; várias valem como estão
            if (antes <= 1) != (pares.len() <= 1) {
                l.loop_all = pares.len() <= 1;
            }
            l.edges = if pares.is_empty() { vec![z] } else { pares.iter().map(|p| p.0).collect() };
            l.edge_keys = if pares.iter().all(|p| p.1.is_none()) { vec![] } else { pares.iter().map(|p| p.1).collect() };
            true
        }
        FeatureKind::SmCurveFlange(c) => {
            let pts = cadeia(c.so_aresta);
            if pts.len() < 2 {
                return false;
            }
            // B531: seletor inteligente. Uma aresta reta sozinha (sem curva emendada) volta ao modo
            // reta, devolvendo os recuos e o alívio que a aba tinha antes de virar curva
            if reta && polilinha_reta(&pts) {
                if let Some(FeatureKind::SmFlange(mut f)) = fenkra_core::aba_unif::converter_aba(&FeatureKind::SmCurveFlange(c.clone()), fenkra_core::aba_unif::ModoAba::Aresta) {
                    f.edge = mid;
                    f.edge_key = key;
                    *k = FeatureKind::SmFlange(f);
                    return true;
                }
            }
            c.edge = pts;
            c.edge_key = key;
            true
        }
        _ => false,
    }
}

/// B560: o que o app sabe da aresta clicada e da malha, para o clique somar na lista de arestas do
/// modo "Arestas" da aba (retas e curvas misturadas).
#[derive(Clone, Debug, Default)]
pub struct CliqueAresta {
    pub mid: [f64; 3],
    pub key: Option<EdgeKey>,
    pub reta: bool,
    /// polilinha da aresta clicada e da cadeia tangente a ela
    pub so_pts: Vec<[f64; 3]>,
    pub cadeia_pts: Vec<[f64; 3]>,
    /// as arestas da cadeia tangente do clique, com a clicada primeiro (meio e chave de cada uma)
    pub cadeia: Vec<([f64; 3], Option<EdgeKey>)>,
    /// as arestas da cadeia da aba curva já aberta (vazio = o app não soube achar)
    pub existente: Vec<([f64; 3], Option<EdgeKey>)>,
}

/// B560: um clique numa aresta com a aba de arestas aberta, no modo único "Arestas". A lista aceita
/// retas e curvas misturadas: cada clique soma (a curva soma a cadeia tangente quando "seguir
/// tangentes" está ligado) e clicar de novo numa escolhida tira. Com uma aresta só o recurso é o
/// que sempre foi (aba reta ou aba curva); a segunda aresta o promove a aba de arestas (o laço), que
/// emenda os trechos curvos numa aba curva contínua e deixa as retas como abas retas. Os parâmetros
/// (comprimento, ângulo, raio, lado, alívio, direção) vão junto. Devolve se mudou algo.
pub fn aresta_clicada_lista(k: &mut FeatureKind, c: &CliqueAresta) -> bool {
    let z = crate::contrato::PONTO_VAZIO;
    let perto = |a: &[f64; 3], b: &[f64; 3]| (0..3).map(|i| (a[i] - b[i]).powi(2)).sum::<f64>().sqrt() < 0.01;
    // o que o clique acrescenta: a cadeia tangente (curva, com "seguir") ou só a aresta
    let entram = |seguir: bool| -> Vec<([f64; 3], Option<EdgeKey>)> {
        if !c.reta && seguir && !c.cadeia.is_empty() {
            c.cadeia.clone()
        } else {
            vec![(c.mid, c.key)]
        }
    };
    let para_laco = |k: &FeatureKind, itens: Vec<([f64; 3], Option<EdgeKey>)>, seguir: bool| -> Option<FeatureKind> {
        let Some(FeatureKind::SmFlangeLoop(mut l)) = fenkra_core::aba_unif::converter_aba(k, fenkra_core::aba_unif::ModoAba::Laco) else { return None };
        l.edges = itens.iter().map(|x| x.0).collect();
        l.edge_keys = if itens.iter().all(|x| x.1.is_none()) { vec![] } else { itens.iter().map(|x| x.1).collect() };
        l.loop_all = false;
        l.seguir = seguir;
        Some(FeatureKind::SmFlangeLoop(l))
    };
    match k {
        FeatureKind::SmFlange(f) if !crate::contrato::ponto_vazio(&f.edge) => {
            if perto(&f.edge, &c.mid) {
                // clicar de novo na escolhida tira (o campo volta a esperar a aresta)
                f.edge = z;
                f.edge_key = None;
                return true;
            }
            let mut itens = vec![(f.edge, f.edge_key)];
            for e in entram(false) {
                if !itens.iter().any(|(m, _)| perto(m, &e.0)) {
                    itens.push(e);
                }
            }
            match para_laco(k, itens, false) {
                Some(novo) => {
                    *k = novo;
                    true
                }
                None => false,
            }
        }
        FeatureKind::SmCurveFlange(cf) if !cf.edge.is_empty() && !c.existente.is_empty() => {
            if c.existente.iter().any(|(m, _)| perto(m, &c.mid)) {
                cf.edge.clear();
                cf.edge_key = None;
                return true;
            }
            let seguir = !cf.so_aresta;
            let mut itens = c.existente.clone();
            for e in entram(seguir) {
                if !itens.iter().any(|(m, _)| perto(m, &e.0)) {
                    itens.push(e);
                }
            }
            match para_laco(k, itens, seguir) {
                Some(novo) => {
                    *k = novo;
                    true
                }
                None => false,
            }
        }
        FeatureKind::SmFlangeLoop(l) => {
            let mut pares: Vec<([f64; 3], Option<EdgeKey>)> = l.edges.iter().enumerate().filter(|(_, e)| !crate::contrato::ponto_vazio(e)).map(|(i, e)| (*e, l.edge_keys.get(i).copied().flatten())).collect();
            let antes = pares.len();
            let novos = entram(l.seguir);
            if pares.iter().any(|(m, _)| perto(m, &c.mid)) {
                // a escolhida (e a cadeia tangente dela, com "seguir") sai
                pares.retain(|(m, _)| !novos.iter().any(|(n, _)| perto(m, n)));
            } else {
                for e in novos {
                    if !pares.iter().any(|(m, _)| perto(m, &e.0)) {
                        pares.push(e);
                    }
                }
            }
            if (antes <= 1) != (pares.len() <= 1) {
                l.loop_all = pares.len() <= 1;
            }
            l.edges = if pares.is_empty() { vec![z] } else { pares.iter().map(|p| p.0).collect() };
            l.edge_keys = if pares.iter().all(|p| p.1.is_none()) { vec![] } else { pares.iter().map(|p| p.1).collect() };
            true
        }
        _ => aresta_clicada(k, c.mid, c.key, c.reta, |so| if so { c.so_pts.clone() } else { c.cadeia_pts.clone() }),
    }
}

/// B530: aresta do contorno externo da face `face` da malha, para o laço de abas começar por ela.
/// O contorno externo é o que fica mais longe do centro das arestas da face (um furo fica dentro).
/// Devolve o índice da aresta na malha.
pub fn aresta_externa_da_face(mesh: &fenkra_kernel::Mesh, face: usize) -> Option<usize> {
    let da_face: Vec<usize> = (0..mesh.edge_faces.len()).filter(|e| mesh.edge_faces[*e].contains(&(face as u32)) && mesh.edge_formula(*e).is_some()).collect();
    if da_face.is_empty() {
        return None;
    }
    let mid = |e: usize| [mesh.edge_mids[e * 3] as f64, mesh.edge_mids[e * 3 + 1] as f64, mesh.edge_mids[e * 3 + 2] as f64];
    let n = da_face.len() as f64;
    let c = da_face.iter().fold([0.; 3], |a, e| {
        let m = mid(*e);
        [a[0] + m[0] / n, a[1] + m[1] / n, a[2] + m[2] / n]
    });
    let dist = |e: usize| {
        let m = mid(e);
        (0..3).map(|i| (m[i] - c[i]).powi(2)).sum::<f64>()
    };
    da_face.into_iter().max_by(|a, b| dist(*a).total_cmp(&dist(*b)))
}

/// B530: clique numa FACE com o laço de abas aberto. A face vale pelo contorno inteiro: grava uma
/// aresta do contorno dela (`mid`, `key`) e liga o laço da face, o mesmo estado de uma aresta só.
/// Substitui a escolha anterior. Devolve se o comando era um laço.
pub fn face_clicada_no_laco(k: &mut FeatureKind, mid: [f64; 3], key: Option<EdgeKey>) -> bool {
    let FeatureKind::SmFlangeLoop(l) = k else { return false };
    l.edges = vec![mid];
    l.edge_keys = key.into_iter().map(Some).collect();
    l.loop_all = true;
    true
}

/// B531: clique numa FACE com a aba de aresta ou de aresta curva aberta: o seletor escolhe o modo
/// sozinho e vira o laço da face (o contorno inteiro), levando comprimento, ângulo, raio, lado e
/// alívio. No laço age como `face_clicada_no_laco`. Devolve se o comando era uma aba desses modos.
pub fn face_clicada_na_aba(k: &mut FeatureKind, mid: [f64; 3], key: Option<EdgeKey>) -> bool {
    if matches!(k, FeatureKind::SmFlange(_) | FeatureKind::SmCurveFlange(_)) {
        if let Some(novo) = fenkra_core::aba_unif::converter_aba(k, fenkra_core::aba_unif::ModoAba::Laco) {
            *k = novo;
        }
    }
    face_clicada_no_laco(k, mid, key)
}

/// A aba curva com os parâmetros da aba de aresta `f`, na cadeia `pts`. A aba reta inteira fica
/// guardada em `reta` (B531): recuos, alívio e mitra voltam se a pessoa clicar numa reta de novo.
pub fn aba_curva_de(f: &fenkra_core::SmFlange, pts: Vec<[f64; 3]>, key: Option<EdgeKey>) -> fenkra_core::SmCurveFlange {
    fenkra_core::SmCurveFlange { edge: pts, length: f.length.clone(), flip: f.flip, angle: f.angle.clone(), radius: f.radius.clone(), side: f.side, edge_key: key, so_aresta: true, reta: Some(Box::new(f.clone())) }
}

/// A polilinha de uma aresta é reta? (todos os pontos sobre a corda, a menos de 1 µm por mm)
pub fn polilinha_reta(p: &[[f64; 3]]) -> bool {
    let (Some(a), Some(b)) = (p.first(), p.last()) else { return true };
    let d = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let l = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
    if l < 1e-9 {
        // aresta fechada (círculo): não é reta se tiver pontos fora da ponta
        return p.iter().all(|q| (0..3).map(|i| (q[i] - a[i]).powi(2)).sum::<f64>().sqrt() < 1e-6);
    }
    let u = [d[0] / l, d[1] / l, d[2] / l];
    p.iter().all(|q| {
        let v = [q[0] - a[0], q[1] - a[1], q[2] - a[2]];
        let t = v[0] * u[0] + v[1] * u[1] + v[2] * u[2];
        let w = [v[0] - u[0] * t, v[1] - u[1] * t, v[2] - u[2] * t];
        (w[0] * w[0] + w[1] * w[1] + w[2] * w[2]).sqrt() < 1e-3 * l.max(1.)
    })
}

/// Algum campo ainda vazio? Quem responde é o descritor do comando (`contrato::FeatureSpec`): as
/// entradas declaradas (aresta da aba, face da rosca, linha da dobra, faces da casca…) e os
/// marcadores "a escolher" que o adaptador lê. Aqui não sobra comparação com `[0.; 3]`.
pub fn feature_incomplete(k: &FeatureKind) -> bool {
    crate::contrato::incompleto(crate::contrato::Alvo::Feature(k))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn campos_vazios_preenchidos_na_ordem() {
        let mut k = RefKind::LinePoints { a: point(), b: point(), start: "0".into(), end: "0".into() };
        assert_eq!(count_unset(&k), 2);
        let seed = Seed { points: vec![PointRef::Fixed([1., 2., 3.])], ..Default::default() };
        assert!(fill(&mut k, &mut Cands::from_seed(&seed, None)));
        assert_eq!(count_unset(&k), 1, "um clique preenche só o primeiro campo");
        let seed = Seed { points: vec![PointRef::Ref(Id(7))], ..Default::default() };
        assert!(fill(&mut k, &mut Cands::from_seed(&seed, None)));
        assert_eq!(k, RefKind::LinePoints { a: PointRef::Fixed([1., 2., 3.]), b: PointRef::Ref(Id(7)), start: "0".into(), end: "0".into() });
        // tipos não se misturam: uma curva não entra num campo de ponto
        let mut k = RefKind::PointOnCurve { curve: curve(), at: OnCurve::Ratio("0.5".into()), reverse: false };
        let seed = Seed { points: vec![PointRef::Ref(Id(9))], ..Default::default() };
        assert!(!fill(&mut k, &mut Cands::from_seed(&seed, None)));
        // sketch do comando vem da árvore
        let mut f = FeatureKind::Extrude(Extrude { thin_side: 0, sketch: UNSET_SKETCH, op: BoolOp::Add, extent: Extent::Blind("10".into()), reverse: false, taper: "0".into(), thin: None, dir: None, ends: Default::default() });
        assert!(feature_incomplete(&f));
        assert!(fill(&mut f, &mut Cands::from_seed(&Seed::default(), Some(Id(42)))));
        assert!(!feature_incomplete(&f));
    }
}
