// SPDX-License-Identifier: LGPL-2.1-or-later
//! B371: importar as anotações do 3D (PMI) para a vista do desenho. `reconhecer` lista o que há no
//! modelo e diz, anotação por anotação, como chega à folha (cota, nota, solda, rugosidade, quadro
//! geométrico, referência, balão) ou por que fica de fora; a pessoa confere a lista e `importar`
//! cria só as escolhidas. Cada item criado guarda o id da anotação de origem (`Dim::pmi`,
//! `Note::pmi`) e as âncoras seguem em coordenadas do modelo, então `atualizar` refaz o que mudou
//! no 3D e acusa o que foi apagado de lá.

use crate::layout::{from_sheet, to_sheet};
use crate::source::{Model, Models};
use crate::{Dim, DimKind, Drawing, DwgGdt, DwgWeld, Id, Note, NoteKind, V2, V3};
use fenkra_core::pmi::{Pmi, PmiKind};

/// Texto com mais linhas ou caracteres que isto é bloco de notas: não cabe como nota de folha.
const LINHAS_BLOCO: usize = 4;
const CARACTERES_BLOCO: usize = 160;

/// Como a anotação chega ao desenho.
#[derive(Clone, PartialEq, Debug)]
pub enum Como {
    Cota(DimKind),
    Nota(NoteKind),
}

/// Por que uma anotação não entra.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Fora {
    /// legenda 3D: a folha já tem a sua legenda
    Legenda,
    /// bloco de notas: não cabe na folha
    BlocoDeNotas,
    /// sem equivalente de cota na folha (ordenada, entre furos, comprimento de arco)
    SemEquivalente,
    /// alvo de referência: só existe no 3D
    AlvoDeReferencia,
    /// a medida não aparece nesta vista (a anotação aponta ao longo da direção de visão)
    NaoApareceNaVista,
    /// a anotação está oculta no modelo
    Oculta,
    /// já importada para esta vista
    JaImportada,
}

impl Fora {
    /// Texto para a lista de conferência (a interface traduz pela mesma frase).
    pub fn texto(self) -> &'static str {
        match self {
            Fora::Legenda => "A folha já tem legenda",
            Fora::BlocoDeNotas => "Bloco de notas: não cabe na folha",
            Fora::SemEquivalente => "Sem equivalente de cota na folha",
            Fora::AlvoDeReferencia => "Alvo de referência: só existe no 3D",
            Fora::NaoApareceNaVista => "Não aparece nesta vista",
            Fora::Oculta => "Oculta no modelo",
            Fora::JaImportada => "Já importada nesta vista",
        }
    }
}

/// Uma linha da lista de conferência.
#[derive(Clone, Debug, PartialEq)]
pub struct Candidato {
    pub pmi: Id,
    pub nome: String,
    pub destino: Result<Como, Fora>,
}

impl Candidato {
    pub fn entra(&self) -> bool {
        self.destino.is_ok()
    }
}

fn medida_fora_da_vista(a: V3, b: V3, ax: &crate::source::Axes) -> bool {
    let (p, q) = (ax.project(a), ax.project(b));
    ((q[0] - p[0]).powi(2) + (q[1] - p[1]).powi(2)).sqrt() < 1e-6
}

fn bloco(texto: &str) -> bool {
    texto.lines().count() > LINHAS_BLOCO || texto.chars().count() > CARACTERES_BLOCO
}

/// Anotações 3D do modelo: as da peça ou as da própria montagem. Na montagem elas já estão no
/// sistema do conjunto, o mesmo em que a vista projeta; as anotações das peças de dentro ficam
/// de fora (cada peça tem o seu sistema e a sua numeração).
fn pmi_do_modelo(model: &Model) -> &[Pmi] {
    match model {
        Model::Part(doc) => &doc.pmi,
        Model::Asm(a) => &a.pmi,
    }
}

/// Lê o PMI do modelo da vista (peça ou montagem) e classifica cada anotação para a vista `vista`.
pub fn reconhecer(d: &Drawing, m: &mut Models, vista: Id) -> Vec<Candidato> {
    let Some(v) = d.view(vista) else { return vec![] };
    let axes = m.proj(d, vista).axes;
    let loaded = m.load(&v.source);
    pmi_do_modelo(&loaded.model)
        .iter()
        .map(|p| Candidato { pmi: p.id, nome: p.name.clone(), destino: classificar(d, vista, p, axes.as_ref()) })
        .collect()
}

fn classificar(d: &Drawing, vista: Id, p: &Pmi, axes: Option<&crate::source::Axes>) -> Result<Como, Fora> {
    if d.dims.iter().any(|x| x.view == vista && x.pmi == Some(p.id)) || d.notes.iter().any(|n| n.view == Some(vista) && n.pmi == Some(p.id)) {
        return Err(Fora::JaImportada);
    }
    if !p.visible {
        return Err(Fora::Oculta);
    }
    let dois = |ax: Option<&crate::source::Axes>| match (p.anchors.first(), p.anchors.get(1), ax) {
        (Some(a), Some(b), Some(ax)) => !medida_fora_da_vista(*a, *b, ax),
        _ => false,
    };
    match &p.kind {
        PmiKind::Linear => {
            if !dois(axes) {
                return Err(Fora::NaoApareceNaVista);
            }
            Ok(Como::Cota(escolher_linear(p, axes)))
        }
        PmiKind::Radius => dois(axes).then_some(Como::Cota(DimKind::Radius)).ok_or(Fora::NaoApareceNaVista),
        PmiKind::Diameter => dois(axes).then_some(Como::Cota(DimKind::Diameter)).ok_or(Fora::NaoApareceNaVista),
        PmiKind::Angle if p.anchors.len() >= 3 => Ok(Como::Cota(DimKind::Angle)),
        PmiKind::Angle => Err(Fora::SemEquivalente),
        PmiKind::Ordinate { .. } | PmiKind::Gap { .. } | PmiKind::ArcLength => Err(Fora::SemEquivalente),
        PmiKind::Note | PmiKind::FeatureNote { .. } => {
            if bloco(&p.text) {
                Err(Fora::BlocoDeNotas)
            } else {
                Ok(Como::Nota(NoteKind::Text))
            }
        }
        PmiKind::Roughness { .. } => Ok(Como::Nota(NoteKind::Roughness)),
        PmiKind::Weld { .. } => Ok(Como::Nota(NoteKind::Weld)),
        PmiKind::Datum { .. } => Ok(Como::Nota(NoteKind::Datum)),
        PmiKind::Gdt { .. } => Ok(Como::Nota(NoteKind::Gdt)),
        PmiKind::Balloon { .. } => Ok(Como::Nota(NoteKind::Balloon)),
        PmiKind::TitleBlock { .. } => Err(Fora::Legenda),
        PmiKind::DatumTarget { .. } => Err(Fora::AlvoDeReferencia),
    }
}

/// Cota linear: horizontal ou vertical quando a medida está alinhada com um eixo da folha; senão alinhada.
fn escolher_linear(p: &Pmi, axes: Option<&crate::source::Axes>) -> DimKind {
    let (Some(a), Some(b), Some(ax)) = (p.anchors.first(), p.anchors.get(1), axes) else { return DimKind::Aligned };
    let (q0, q1) = (ax.project(*a), ax.project(*b));
    let (dx, dy) = ((q1[0] - q0[0]).abs(), (q1[1] - q0[1]).abs());
    if dy < 1e-6 {
        DimKind::Horizontal
    } else if dx < 1e-6 {
        DimKind::Vertical
    } else {
        DimKind::Aligned
    }
}

/// O que `importar` fez.
#[derive(Debug, Default, PartialEq)]
pub struct Relatorio {
    pub cotas: Vec<Id>,
    pub notas: Vec<Id>,
    /// escolhidas que não entraram (anotação inexistente ou classificada como "fora")
    pub recusadas: Vec<(Id, Fora)>,
}

fn tolerancia(dm: &mut Dim, p: &Pmi) {
    let (mais, menos) = (p.tol_plus.trim(), p.tol_minus.trim());
    if !mais.is_empty() || !menos.is_empty() {
        dm.tol = format!("+{}/-{}", if mais.is_empty() { "0" } else { mais }, if menos.is_empty() { "0" } else { menos });
    }
}

/// Cria na vista as anotações `escolhidas` (ids de PMI). O que foi classificado como "fora" é
/// recusado e vai para o relatório.
pub fn importar(d: &mut Drawing, m: &mut Models, vista: Id, escolhidas: &[Id]) -> Relatorio {
    let mut rel = Relatorio::default();
    let Some(v) = d.view(vista).cloned() else { return rel };
    let proj = m.proj(d, vista);
    let Some(ax) = proj.axes else { return rel };
    let lista = reconhecer(d, m, vista);
    let pmis: Vec<Pmi> = pmi_do_modelo(&m.load(&v.source).model).to_vec();
    let th = d.text_height;
    for id in escolhidas {
        let (Some(c), Some(p)) = (lista.iter().find(|c| c.pmi == *id), pmis.iter().find(|p| p.id == *id)) else { continue };
        match c.destino.clone() {
            Err(f) => rel.recusadas.push((*id, f)),
            Ok(Como::Cota(kind)) => {
                let nid = d.alloc();
                let mut dm = Dim { id: nid, view: vista, kind, anchors: p.anchors.clone(), offset: if kind == DimKind::Angle { 0. } else { 8. }, text: p.text.clone(), pmi: Some(*id), ..Default::default() };
                tolerancia(&mut dm, p);
                d.dims.push(dm);
                rel.cotas.push(nid);
            }
            Ok(Como::Nota(kind)) => {
                let nid = d.alloc();
                let a0 = p.anchors.first().copied().unwrap_or([0.; 3]);
                let tip = to_sheet(&v, &proj, ax.project(a0));
                let desl = ax.project([a0[0] + p.offset[0], a0[1] + p.offset[1], a0[2] + p.offset[2]]);
                let base = ax.project(a0);
                let (dx, dy) = ((desl[0] - base[0]) * v.scale, (desl[1] - base[1]) * v.scale);
                let pos = if dx.abs() + dy.abs() > 1e-6 { [tip[0] + dx, tip[1] + dy] } else { [tip[0] + 12., tip[1] + 10.] };
                let mut n = Note { id: nid, sheet: v.sheet, kind: kind.clone(), pos, leader: Some(tip), text: p.text.clone(), size: th, view: Some(vista), anchor3: Some(a0), snapped: true, pmi: Some(*id), style: p.style.clone(), ..Default::default() };
                preencher_nota(&mut n, p);
                pontas_extras(&mut n, p, &v, &proj);
                d.notes.push(n);
                rel.notas.push(nid);
            }
        }
    }
    rel
}

/// Texto e dados específicos de cada tipo de nota, tomados da anotação 3D.
fn preencher_nota(n: &mut Note, p: &Pmi) {
    match &p.kind {
        PmiKind::Roughness { ra, process, extra } => {
            n.text = ra.clone();
            n.rough = Some(crate::simbolos::DwgRough::de_pmi(extra, process));
        }
        PmiKind::Weld { kind, size, all_around, extra } => {
            n.text = size.clone();
            n.weld = Some(DwgWeld { kind: *kind, all_around: *all_around, extra: extra.clone() });
        }
        PmiKind::Datum { letter } => n.text = letter.clone(),
        PmiKind::Gdt { ch, tol, datums, extra } => {
            n.text = String::new();
            n.gdt = Some(DwgGdt { ch: *ch, tol: tol.clone(), datums: datums.clone(), extra: extra.clone() });
        }
        PmiKind::Balloon { item, .. } => n.text = item.clone(),
        _ => {}
    }
}

/// Refaz o que veio do 3D: âncoras, texto e tolerância das cotas e notas importadas. Devolve os
/// ids das anotações do desenho cuja origem sumiu do modelo.
pub fn atualizar(d: &mut Drawing, m: &mut Models) -> Vec<Id> {
    m.refresh();
    let mut orfas = vec![];
    // PMI de cada fonte de vista, lido uma vez por vista
    let vistas: Vec<Id> = d.views.iter().map(|v| v.id).collect();
    for vid in vistas {
        let Some(v) = d.view(vid).cloned() else { continue };
        let pmis: Vec<Pmi> = pmi_do_modelo(&m.load(&v.source).model).to_vec();
        let proj = m.proj(d, vid);
        for dm in d.dims.iter_mut().filter(|x| x.view == vid && x.pmi.is_some()) {
            match pmis.iter().find(|p| Some(p.id) == dm.pmi) {
                Some(p) => {
                    dm.anchors = p.anchors.clone();
                    dm.text = p.text.clone();
                    tolerancia(dm, p);
                }
                None => orfas.push(dm.id),
            }
        }
        for n in d.notes.iter_mut().filter(|n| n.view == Some(vid) && n.pmi.is_some()) {
            match pmis.iter().find(|p| Some(p.id) == n.pmi) {
                Some(p) => {
                    let a0 = p.anchors.first().copied().unwrap_or([0.; 3]);
                    n.anchor3 = Some(a0);
                    n.text = p.text.clone();
                    preencher_nota(n, p);
                    if let Some(ax) = proj.axes {
                        n.leader = Some(to_sheet(&v, &proj, ax.project(a0)));
                    }
                    pontas_extras(n, p, &v, &proj);
                }
                None => orfas.push(n.id),
            }
        }
    }
    let _: fn(&crate::View, &crate::source::Proj, V2) -> V2 = from_sheet;
    orfas
}

/// B370: as âncoras do PMI além da primeira viram pontas extras da nota, presas ao modelo; o lado e
/// a seta de cada uma já vêm no estilo copiado do PMI.
fn pontas_extras(n: &mut crate::Note, p: &Pmi, v: &crate::View, proj: &crate::source::Proj) {
    n.leaders_extra.clear();
    n.anchors_extra.clear();
    let Some(ax) = proj.axes else { return };
    for a in p.anchors.iter().skip(1) {
        n.leaders_extra.push(to_sheet(v, proj, ax.project(*a)));
        n.anchors_extra.push(Some(*a));
    }
}
