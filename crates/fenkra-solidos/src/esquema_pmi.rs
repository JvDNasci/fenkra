// SPDX-License-Identifier: LGPL-2.1-or-later
//! B192: esquema de cotagem automático (assistente de PMI).
//!
//! Lê o corpo final da peça e propõe, sem digitar nada, o sistema de referências (DRF) e as
//! anotações das features:
//!
//! 1. **Referências**: as faces planas se agrupam por plano (mesma normal e mesma altura). A
//!    primária é o plano de maior área; a secundária, o maior plano perpendicular a ela; a terciária,
//!    o maior plano perpendicular às duas. Empate de área (as duas faces de uma chapa) fica com a
//!    face de apoio: a de normal mais negativa (embaixo, atrás, à esquerda).
//! 2. **Furos** (pelo reconhecimento de features, `reconhece`): agrupados por diâmetro e direção,
//!    com "n×" quando o tamanho se repete. Uma cota de diâmetro por grupo e a localização do
//!    primeiro furo do grupo a partir das referências B e C.
//! 3. **Rasgos e degraus**: duas faces planas paralelas que se encaram (normais de fora uma contra
//!    a outra) mais perto que a peça inteira: cota da largura.
//! 4. **Chanfros**: nota "C1" (ou "1 × 2") na face do chanfro.
//!
//! Dois modos, como nos softwares de modelagem CAD avançados:
//! - **Mais/Menos**: medidas com ± (tamanho total, localização dos furos, largura dos rasgos) e o
//!   furo pela classe ISO 286 da regra;
//! - **Geométrico**: planeza na primária, perpendicularidade da secundária a A e da terciária a A e
//!   B, posição dos furos em relação a A, B e C; a localização vira cota básica (em caixa).
//!
//! Tudo nasce na pasta [`PASTA`] e com nome estável (a chave da regra). [`aplicar`] regenera sem
//! apagar o que a pessoa fez: a anotação gerada que ainda existe mantém o texto e a tolerância
//! editados e só tem a geometria (âncoras e alvos) refeita; a que perdeu a feature sai; o que está
//! fora da pasta é manual e nunca é tocado (tirar da pasta é o jeito de "congelar" uma anotação).
//! As regras por tipo de feature ([`RegrasEsquema`]) são dados: gravam e voltam em JSON para
//! reaproveitar de uma peça para outra.

use crate::reconhece::{self, Tipo};
use fenkra_core::pmi::{num, GdtChar, Pmi, PmiFrame, PmiKind, PmiStyle, PmiTarget, TargetClass};
use fenkra_core::{Document, Id};
use fenkra_kernel::{FaceInfo, Shape};
use serde::{Deserialize, Serialize};

/// Pasta da árvore do PMI onde mora tudo o que o assistente gera.
pub const PASTA: &str = "Esquema automático";

type V3 = [f64; 3];

fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn mais(a: V3, b: V3, k: f64) -> V3 {
    [a[0] + b[0] * k, a[1] + b[1] * k, a[2] + b[2] * k]
}
fn unit(a: V3) -> V3 {
    let l = dot(a, a).sqrt();
    if l < 1e-12 { a } else { [a[0] / l, a[1] / l, a[2] / l] }
}

/// Modo do esquema.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModoEsquema {
    /// medidas com ± e classes de ajuste
    #[default]
    MaisMenos,
    /// referências, planeza, perpendicularidade e posição
    Geometrico,
}

impl ModoEsquema {
    pub fn label(self) -> &'static str {
        match self {
            ModoEsquema::MaisMenos => "Mais/Menos",
            ModoEsquema::Geometrico => "Geométrico",
        }
    }
}

/// Regra de um tipo de feature: se entra no esquema e com que tolerância.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RegraFeature {
    pub gerar: bool,
    /// furo: classe ISO 286 ("H8"); rasgo: ± em mm ("0,1"); chanfro: não usa
    pub tol: String,
}

/// Regras reaproveitáveis do esquema automático, por tipo de feature.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RegrasEsquema {
    pub modo: ModoEsquema,
    /// ± das medidas lineares no modo Mais/Menos (mm)
    pub tol_linear: f64,
    /// planeza da primária (mm)
    pub planeza: f64,
    /// perpendicularidade da secundária e da terciária (mm)
    pub perpendicularidade: f64,
    /// posição dos furos (zona cilíndrica, mm)
    pub posicao: f64,
    pub furo: RegraFeature,
    pub rasgo: RegraFeature,
    pub chanfro: RegraFeature,
}

impl Default for RegrasEsquema {
    fn default() -> Self {
        RegrasEsquema {
            modo: ModoEsquema::MaisMenos,
            tol_linear: 0.1,
            planeza: 0.05,
            perpendicularidade: 0.05,
            posicao: 0.1,
            furo: RegraFeature { gerar: true, tol: "H8".into() },
            rasgo: RegraFeature { gerar: true, tol: "0,1".into() },
            chanfro: RegraFeature { gerar: true, tol: String::new() },
        }
    }
}

impl RegrasEsquema {
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }
    pub fn from_json(s: &str) -> Result<Self, String> {
        serde_json::from_str(s).map_err(|e| e.to_string())
    }
}

/// Um plano de referência: face plana (ou grupo coplanar) com a área somada.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlanoRef {
    pub letra: char,
    /// ponto sobre a maior face do plano
    pub ponto: V3,
    /// normal de fora
    pub normal: V3,
    pub area: f64,
}

#[derive(Clone, Copy, Debug)]
struct Plano {
    normal: V3,
    altura: f64,
    area: f64,
    ponto: V3,
    maior: f64,
}

/// Agrupa as faces planas por plano (normal e altura iguais).
fn planos(faces: &[FaceInfo], tol: f64) -> Vec<Plano> {
    let mut out: Vec<Plano> = vec![];
    for f in faces.iter().filter(|f| f.kind == 0 && f.area > 0.) {
        let n = unit(f.dir);
        let h = dot(f.point, n);
        match out.iter_mut().find(|p| dot(p.normal, n) > 1. - 1e-6 && (p.altura - h).abs() < tol) {
            Some(p) => {
                p.area += f.area;
                if f.area > p.maior {
                    p.maior = f.area;
                    p.ponto = f.point;
                }
            }
            None => out.push(Plano { normal: n, altura: h, area: f.area, ponto: f.point, maior: f.area }),
        }
    }
    out
}

/// Maior plano que passa no filtro; empate de área (1e-6 relativo) fica com a normal mais negativa.
fn maior(ps: &[Plano], ok: impl Fn(&Plano) -> bool) -> Option<Plano> {
    let mut best: Option<Plano> = None;
    for p in ps.iter().filter(|p| ok(p)) {
        best = match best {
            None => Some(*p),
            Some(b) => {
                let rel = (p.area - b.area) / b.area.max(1e-12);
                let neg = |q: &Plano| q.normal[0] + q.normal[1] + q.normal[2];
                if rel > 1e-6 || (rel.abs() <= 1e-6 && neg(p) < neg(&b) - 1e-9) { Some(*p) } else { Some(b) }
            }
        };
    }
    best
}

/// Escolhe o DRF: primária, secundária e terciária (as que existirem).
pub fn escolher_drf(faces: &[FaceInfo], tol: f64) -> Vec<PlanoRef> {
    let ps = planos(faces, tol);
    let mut out = vec![];
    let Some(a) = maior(&ps, |_| true) else { return out };
    out.push(PlanoRef { letra: 'A', ponto: a.ponto, normal: a.normal, area: a.area });
    let perp = |p: &Plano, q: V3| dot(p.normal, q).abs() < 1e-3;
    let Some(b) = maior(&ps, |p| perp(p, a.normal)) else { return out };
    out.push(PlanoRef { letra: 'B', ponto: b.ponto, normal: b.normal, area: b.area });
    if let Some(c) = maior(&ps, |p| perp(p, a.normal) && perp(p, b.normal)) {
        out.push(PlanoRef { letra: 'C', ponto: c.ponto, normal: c.normal, area: c.area });
    }
    out
}

/// Grupo de furos do mesmo diâmetro e direção.
#[derive(Clone, Debug, PartialEq)]
pub struct GrupoFuros {
    pub diametro: f64,
    pub eixo: V3,
    /// boca de cada furo (ponto do eixo na entrada)
    pub bocas: Vec<V3>,
    pub cego: Option<f64>,
    /// rebaixo: diâmetro e profundidade
    pub rebaixo: Option<(f64, f64)>,
    /// classe escrita no próprio furo ("H7"); vazia = a da regra
    pub classe: String,
}

/// Um furo, venha do comando Furo da árvore ou do reconhecimento do corpo.
struct UmFuro {
    diametro: f64,
    eixo: V3,
    boca: V3,
    cego: Option<f64>,
    rebaixo: Option<(f64, f64)>,
    classe: String,
}

/// Furos dos comandos Furo da peça (medidas de projeto, com a classe escrita no furo).
fn furos_do_documento(doc: &Document) -> Vec<UmFuro> {
    crate::hole_table(doc)
        .into_iter()
        .map(|h| UmFuro {
            diametro: h.diameter,
            eixo: unit(h.dir),
            boca: h.pos,
            cego: (!h.through).then_some(h.depth),
            rebaixo: (h.d_boca > 0. && h.angulo == 0.).then_some((h.d_boca, h.prof_boca)),
            classe: h.tolerancia,
        })
        .collect()
}

/// Furos achados na geometria (corpo importado, sem comando Furo).
fn furos_reconhecidos(rec: &reconhece::Reconhecimento) -> Vec<UmFuro> {
    rec.encontradas
        .iter()
        .filter(|e| matches!(e.tipo, Tipo::FuroPassante | Tipo::FuroCego))
        .map(|e| UmFuro { diametro: e.medida, eixo: unit(e.direcao), boca: e.boca, cego: (e.tipo == Tipo::FuroCego).then_some(e.medida2), rebaixo: e.rebaixo, classe: String::new() })
        .collect()
}

fn grupos_de_furos(furos: Vec<UmFuro>) -> Vec<GrupoFuros> {
    let mut out: Vec<GrupoFuros> = vec![];
    for e in furos {
        let mesmo_fim = |a: Option<f64>, b: Option<f64>| match (a, b) {
            (None, None) => true,
            (Some(x), Some(y)) => (x - y).abs() < 1e-4,
            _ => false,
        };
        match out.iter_mut().find(|g| (g.diametro - e.diametro).abs() < 1e-4 && dot(g.eixo, e.eixo).abs() > 1. - 1e-6 && mesmo_fim(g.cego, e.cego) && g.rebaixo == e.rebaixo && g.classe == e.classe) {
            Some(g) => g.bocas.push(e.boca),
            None => out.push(GrupoFuros { diametro: e.diametro, eixo: e.eixo, bocas: vec![e.boca], cego: e.cego, rebaixo: e.rebaixo, classe: e.classe }),
        }
    }
    // ordem estável: maior diâmetro primeiro, depois pela posição da primeira boca
    for g in &mut out {
        g.bocas.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    }
    out.sort_by(|a, b| b.diametro.partial_cmp(&a.diametro).unwrap_or(std::cmp::Ordering::Equal).then(a.bocas[0].partial_cmp(&b.bocas[0]).unwrap_or(std::cmp::Ordering::Equal)));
    out
}

/// Rasgo ou degrau: duas faces planas paralelas que se encaram.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rasgo {
    pub a: V3,
    pub b: V3,
    pub normal: V3,
    pub largura: f64,
}

fn rasgos(faces: &[FaceInfo], tol: f64, extensao: impl Fn(V3) -> f64) -> Vec<Rasgo> {
    let ps = planos(faces, tol);
    let mut out = vec![];
    for (i, p) in ps.iter().enumerate() {
        for q in &ps[i + 1..] {
            if dot(p.normal, q.normal) > -1. + 1e-6 {
                continue;
            }
            // as normais de fora apontam uma para a outra: o vão está entre elas
            let d = dot(sub(q.ponto, p.ponto), p.normal);
            if d <= tol || d >= extensao(p.normal) - tol {
                continue;
            }
            // o vão fica de fato entre as faces: o ponto da segunda está à frente da primeira
            out.push(Rasgo { a: p.ponto, b: mais(p.ponto, p.normal, d), normal: p.normal, largura: d });
        }
    }
    out.sort_by(|a, b| a.largura.partial_cmp(&b.largura).unwrap_or(std::cmp::Ordering::Equal));
    out
}

fn alvo_plano(p: V3, n: V3) -> Option<PmiTarget> {
    Some(PmiTarget { class: TargetClass::FacePlana, point: p, dir: n, radius: 0., concave: false })
}

/// Furo: âncora solta por enquanto. A boca do furo com rebaixo ou escareado não tem aresta circular
/// do diâmetro do furo, e o alvo ficava "quebrado"; a âncora segue a peça ao regenerar.
fn alvo_furo(_c: V3, _eixo: V3, _r: f64) -> Option<PmiTarget> {
    None
}

fn nova(nome: String, kind: PmiKind, anchors: Vec<V3>, offset: V3, targets: Vec<Option<PmiTarget>>) -> Pmi {
    Pmi { id: Id(0), name: nome, kind, anchors, offset, text: String::new(), tol_plus: String::new(), tol_minus: String::new(), visible: true, plane: None, style: PmiStyle::default(), folder: PASTA.into(), targets }
}

fn fmt(v: f64) -> String {
    num(v)
}

/// Pé de `p` no plano (ponto `o`, normal `n`).
fn pe(p: V3, o: V3, n: V3) -> V3 {
    mais(p, n, -dot(sub(p, o), n))
}

/// O que o assistente propõe para a forma (anotações com `Id(0)`, prontas para [`aplicar`]).
///
/// `furos`: os furos de projeto da peça; `None` lê os furos da geometria pelo reconhecimento.
pub fn propor_forma(s: &Shape, regras: &RegrasEsquema, furos: Option<Vec<GrupoFuros>>) -> Result<(Vec<PlanoRef>, Vec<Pmi>), String> {
    let caixa = s.bbox().ok_or("a peça está vazia")?;
    let diag = dot(sub(caixa.1, caixa.0), sub(caixa.1, caixa.0)).sqrt().max(1e-6);
    let tol = diag * 1e-6;
    let topo = s.topology().map_err(|e| e.0)?;
    let drf = escolher_drf(&topo.faces, tol);
    if drf.is_empty() {
        return Err("a peça não tem face plana para servir de referência".into());
    }
    let extensao = |n: V3| {
        let cs = [caixa.0, caixa.1];
        let mut lo = f64::MAX;
        let mut hi = f64::MIN;
        for x in 0..2 {
            for y in 0..2 {
                for z in 0..2 {
                    let v = dot([cs[x][0], cs[y][1], cs[z][2]], n);
                    lo = lo.min(v);
                    hi = hi.max(v);
                }
            }
        }
        hi - lo
    };
    let afasta = diag * 0.08;
    let geo = regras.modo == ModoEsquema::Geometrico;
    let tl = fmt(regras.tol_linear);
    let mut out = vec![];
    let letras: Vec<String> = drf.iter().map(|r| r.letra.to_string()).collect();

    // referências (no modo Mais/Menos também: são as bases de onde as medidas partem)
    for r in &drf {
        let mut p = nova(format!("Referência {}", r.letra), PmiKind::Datum { letter: r.letra.to_string() }, vec![r.ponto], mais([0.; 3], r.normal, afasta), vec![alvo_plano(r.ponto, r.normal)]);
        p.style.frame = PmiFrame::Padrao;
        out.push(p);
    }
    if geo {
        let a = &drf[0];
        out.push(nova("Planeza A".into(), PmiKind::Gdt { ch: GdtChar::Planeza, tol: fmt(regras.planeza), datums: String::new(), extra: Default::default() }, vec![a.ponto], mais([0.; 3], a.normal, afasta * 1.6), vec![alvo_plano(a.ponto, a.normal)]));
        for (i, r) in drf.iter().enumerate().skip(1) {
            out.push(nova(
                format!("Perpendicularidade {}", r.letra),
                PmiKind::Gdt { ch: GdtChar::Perpendicularidade, tol: fmt(regras.perpendicularidade), datums: letras[..i].join(" "), extra: Default::default() },
                vec![r.ponto],
                mais([0.; 3], r.normal, afasta * 1.6),
                vec![alvo_plano(r.ponto, r.normal)],
            ));
        }
    } else {
        // tamanho total ao longo de cada referência, de face a face oposta
        for r in &drf {
            let ext = extensao(r.normal);
            let b = mais(r.ponto, r.normal, -ext);
            let mut p = nova(format!("Total {}", r.letra), PmiKind::Linear, vec![r.ponto, b], mais([0.; 3], r.normal, -ext * 0.5), vec![alvo_plano(r.ponto, r.normal), None]);
            p.tol_plus = tl.clone();
            p.tol_minus = tl.clone();
            out.push(p);
        }
    }

    // furos
    if regras.furo.gerar {
        let grupos = match furos {
            Some(g) => g,
            None => grupos_de_furos(furos_reconhecidos(&reconhece::analisar(s))),
        };
        for g in grupos {
            let n = g.bocas.len();
            let chave = format!("Ø{}{}", fmt(g.diametro), if n > 1 { format!(" ({n}×)") } else { String::new() });
            let c = g.bocas[0];
            let r = g.diametro / 2.;
            // um ponto da borda, perpendicular ao eixo
            let lado = if dot(g.eixo, [1., 0., 0.]).abs() < 0.9 { [1., 0., 0.] } else { [0., 1., 0.] };
            let u = unit(mais(lado, g.eixo, -dot(lado, g.eixo)));
            let borda = mais(c, u, r);
            let mut d = nova(format!("Furo {chave}"), PmiKind::Diameter, vec![c, borda], mais(u, g.eixo, -1.4).map(|x| x * afasta), vec![alvo_furo(c, g.eixo, r), alvo_furo(c, g.eixo, r)]);
            if n > 1 {
                d.style.prefix = format!("{n}× ");
            }
            let classe = if g.classe.trim().is_empty() { regras.furo.tol.trim() } else { g.classe.trim() };
            if !classe.is_empty() {
                d.style.suffix = format!(" {classe}");
            }
            if let Some(p) = g.cego {
                d.style.suffix += &format!(" ↧{}", fmt(p));
            }
            if let Some((dr, pr)) = g.rebaixo {
                d.style.suffix += &format!(" ⌴Ø{} ↧{}", fmt(dr), fmt(pr));
            }
            out.push(d);
            if geo {
                let datums = letras.join(" ");
                out.push(nova(format!("Posição {chave}"), PmiKind::Gdt { ch: GdtChar::Posicao, tol: format!("Ø{}", fmt(regras.posicao)), datums, extra: Default::default() }, vec![c], mais(u.map(|x| x * 2.4), g.eixo, -1.).map(|x| x * afasta), vec![alvo_furo(c, g.eixo, r)]));
            }
            // localização do primeiro furo a partir de B e C (as referências que o eixo do furo cruza)
            for rf in drf.iter().filter(|rf| dot(rf.normal, g.eixo).abs() < 1e-3) {
                let p0 = pe(c, rf.ponto, rf.normal);
                if dot(sub(c, p0), sub(c, p0)).sqrt() < tol {
                    continue;
                }
                let nome = if geo { format!("Básica {chave} até {}", rf.letra) } else { format!("Local {chave} até {}", rf.letra) };
                let mut l = nova(nome, PmiKind::Linear, vec![p0, c], mais([0.; 3], g.eixo, -afasta * 0.5), vec![alvo_plano(rf.ponto, rf.normal), alvo_furo(c, g.eixo, r)]);
                if geo {
                    // cota básica: o valor exato, a tolerância está no quadro de posição
                    l.style.frame = PmiFrame::Caixa;
                } else {
                    l.tol_plus = tl.clone();
                    l.tol_minus = tl.clone();
                }
                out.push(l);
            }
        }
    }

    // rasgos e degraus
    if regras.rasgo.gerar {
        for (i, r) in rasgos(&topo.faces, tol, extensao).iter().enumerate() {
            let mut l = nova(format!("Rasgo {} ({})", fmt(r.largura), i + 1), PmiKind::Linear, vec![r.a, r.b], [0., 0., 0.], vec![alvo_plano(r.a, r.normal), alvo_plano(r.b, [-r.normal[0], -r.normal[1], -r.normal[2]])]);
            let t = regras.rasgo.tol.trim().trim_start_matches('±');
            if !t.is_empty() {
                l.tol_plus = t.to_string();
                l.tol_minus = t.to_string();
            }
            out.push(l);
        }
    }

    // chanfros
    if regras.chanfro.gerar {
        let rec = reconhece::analisar(s);
        for (i, e) in rec.encontradas.iter().filter(|e| e.tipo == Tipo::Chanfro).enumerate() {
            let txt = if (e.medida - e.medida2).abs() < 1e-4 { format!("C{}", fmt(e.medida)) } else { format!("{} × {}", fmt(e.medida), fmt(e.medida2)) };
            let ponto = e.pontos.first().copied().unwrap_or(e.centro);
            let mut p = nova(format!("Chanfro {txt} ({})", i + 1), PmiKind::Note, vec![ponto], [afasta, afasta, afasta], vec![None]);
            p.text = txt;
            out.push(p);
        }
    }
    Ok((drf, out))
}

/// O que o assistente propõe para a peça do documento.
pub fn propor(doc: &Document, regras: &RegrasEsquema) -> Result<(Vec<PlanoRef>, Vec<Pmi>), String> {
    let s = crate::final_shape(doc).ok_or("a peça não tem corpo")?;
    // furos de projeto quando a peça tem comando Furo; senão, os que a geometria mostra
    let furos = furos_do_documento(doc);
    propor_forma(&s, regras, (!furos.is_empty()).then(|| grupos_de_furos(furos)))
}

/// O que [`aplicar`] fez.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Relatorio {
    pub novas: usize,
    /// geradas que continuam (texto e tolerância como a pessoa deixou; geometria refeita)
    pub mantidas: usize,
    /// geradas cuja feature sumiu
    pub removidas: usize,
}

/// A anotação foi gerada pelo assistente (e ainda está na pasta dele)?
pub fn gerada(p: &Pmi) -> bool {
    p.folder == PASTA
}

/// Aplica a proposta ao PMI do documento: cria o que falta, refaz a geometria do que já existe
/// (sem mexer no texto nem na tolerância que a pessoa editou), tira o que perdeu a feature e não
/// toca nas anotações manuais (fora da pasta).
///
/// Com `refazer`, as geradas voltam ao que a regra manda (texto, tolerância, estilo): é o caminho
/// para aplicar uma regra nova. As manuais continuam intocadas.
pub fn aplicar(doc: &mut Document, proposta: Vec<Pmi>, refazer: bool) -> Relatorio {
    let mut rel = Relatorio::default();
    let chaves: std::collections::HashSet<String> = proposta.iter().map(|p| p.name.clone()).collect();
    let antes = doc.pmi.len();
    doc.pmi.retain(|p| !gerada(p) || chaves.contains(&p.name));
    rel.removidas = antes - doc.pmi.len();
    for nova in proposta {
        match doc.pmi.iter_mut().find(|p| gerada(p) && p.name == nova.name) {
            Some(p) if refazer => {
                let id = p.id;
                *p = Pmi { id, ..nova };
                rel.mantidas += 1;
            }
            Some(p) => {
                p.anchors = nova.anchors;
                p.targets = nova.targets;
                rel.mantidas += 1;
            }
            None => {
                let mut p = nova;
                p.id = doc.alloc();
                doc.pmi.push(p);
                rel.novas += 1;
            }
        }
    }
    rel
}

/// Propõe e aplica de uma vez (o botão do assistente).
pub fn gerar(doc: &mut Document, regras: &RegrasEsquema, refazer: bool) -> Result<Relatorio, String> {
    let (_, proposta) = propor(doc, regras)?;
    Ok(aplicar(doc, proposta, refazer))
}

#[cfg(test)]
mod testes {
    use super::*;

    fn face(kind: u8, dir: V3, area: f64, point: V3) -> FaceInfo {
        FaceInfo { kind, dir, area, point }
    }

    #[test]
    fn drf_pela_area_e_perpendicularidade_com_desempate_na_face_de_apoio() {
        // caixa 100 × 60 × 10: as faces de cima e de baixo empatam; fica a de baixo
        let faces = [
            face(0, [0., 0., 1.], 6000., [50., 30., 10.]),
            face(0, [0., 0., -1.], 6000., [50., 30., 0.]),
            face(0, [1., 0., 0.], 600., [100., 30., 5.]),
            face(0, [-1., 0., 0.], 600., [0., 30., 5.]),
            face(0, [0., 1., 0.], 1000., [50., 60., 5.]),
            face(0, [0., -1., 0.], 1000., [50., 0., 5.]),
            face(1, [0., 0., 1.], 300., [20., 20., 5.]),
        ];
        let d = escolher_drf(&faces, 1e-6);
        assert_eq!(d.len(), 3);
        assert_eq!((d[0].letra, d[0].normal), ('A', [0., 0., -1.]));
        assert_eq!((d[1].letra, d[1].normal), ('B', [0., -1., 0.]));
        assert_eq!((d[2].letra, d[2].normal), ('C', [-1., 0., 0.]));
    }

    #[test]
    fn faces_coplanares_somam_area() {
        // a face de cima partida em duas por um rasgo soma mais que a de baixo inteira? não: empata
        // com ela; mas duas faces laterais coplanares somam e passam a lateral maior
        let faces = [
            face(0, [0., 0., -1.], 5000., [0., 0., 0.]),
            face(0, [1., 0., 0.], 300., [50., 0., 5.]),
            face(0, [0., 1., 0.], 200., [0., 30., 5.]),
            face(0, [0., 1., 0.], 200., [40., 30., 5.]),
        ];
        let d = escolher_drf(&faces, 1e-6);
        assert_eq!(d[1].normal, [0., 1., 0.]);
        assert!((d[1].area - 400.).abs() < 1e-9);
    }

    #[test]
    fn aplicar_regenera_sem_apagar_edicao_manual() {
        let mut doc = Document::new("p");
        let manual = doc.alloc();
        doc.pmi.push(Pmi { id: manual, name: "Furo Ø6,5".into(), folder: String::new(), ..nova("x".into(), PmiKind::Linear, vec![[0.; 3]; 2], [0.; 3], vec![None, None]) });
        let proposta = vec![
            nova("Referência A".into(), PmiKind::Datum { letter: "A".into() }, vec![[0.; 3]], [0.; 3], vec![None]),
            nova("Furo Ø6,5".into(), PmiKind::Diameter, vec![[0.; 3], [3.25, 0., 0.]], [0.; 3], vec![None, None]),
            nova("Rasgo 8 (1)".into(), PmiKind::Linear, vec![[0.; 3], [8., 0., 0.]], [0.; 3], vec![None, None]),
        ];
        let r = aplicar(&mut doc, proposta.clone(), false);
        assert_eq!(r, Relatorio { novas: 3, mantidas: 0, removidas: 0 });
        assert_eq!(doc.pmi.len(), 4);
        // a pessoa edita a tolerância do furo gerado; a peça muda: o furo anda e o rasgo some
        let furo = doc.pmi.iter_mut().find(|p| gerada(p) && p.name == "Furo Ø6,5").unwrap();
        furo.tol_plus = "0,02".into();
        let id_furo = furo.id;
        let mut p2 = proposta;
        p2.pop();
        p2[1].anchors = vec![[10., 0., 0.], [13.25, 0., 0.]];
        let r = aplicar(&mut doc, p2.clone(), false);
        assert_eq!(r, Relatorio { novas: 0, mantidas: 2, removidas: 1 });
        let furo = doc.pmi.iter().find(|p| p.id == id_furo).unwrap();
        assert_eq!(furo.tol_plus, "0,02", "a edição da pessoa fica");
        assert_eq!(furo.anchors[0], [10., 0., 0.], "a geometria acompanha a peça");
        assert!(doc.pmi.iter().any(|p| p.id == manual), "a manual com o mesmo nome fora da pasta não é tocada");
        assert!(!doc.pmi.iter().any(|p| p.name.starts_with("Rasgo")));
        // refazer: a regra manda de novo na gerada, a manual continua
        aplicar(&mut doc, p2, true);
        assert_eq!(doc.pmi.iter().find(|p| p.id == id_furo).unwrap().tol_plus, "");
        assert!(doc.pmi.iter().any(|p| p.id == manual));
    }

    #[test]
    fn regras_voltam_do_json() {
        let mut r = RegrasEsquema { modo: ModoEsquema::Geometrico, ..Default::default() };
        r.furo.tol = "H7".into();
        r.chanfro.gerar = false;
        assert_eq!(RegrasEsquema::from_json(&r.to_json()).unwrap(), r);
    }
}
