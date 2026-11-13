// SPDX-License-Identifier: LGPL-2.1-or-later
//! Fixadores simplificados e paramétricos (parafusos, porcas e arruelas) gerados a partir das
//! famílias da biblioteca pública ([`crate::biblioteca`]).
//!
//! **Convenção de eixos** (o furo/fixador inteligente conta com ela):
//! - o eixo do fixador é o **Z** do documento;
//! - a **face de apoio** fica em **z = 0** (voltada para +Z, contra a peça ou a cabeça vizinha);
//! - o **corpo vai para -Z**: a haste do parafuso ocupa `z ∈ [-L, 0]`, a porca `[-m, 0]` e a
//!   arruela `[-h, 0]`; só a cabeça do parafuso sobe, em `z ∈ [0, k]`;
//! - o sextavado tem um vértice em +X (as faces ficam normais a ±Y e aos giros de 60°).
//!
//! Assim uma pilha parafuso → arruela → peças → porca se monta só com translações em Z.
//!
//! Os documentos guardam as medidas como parâmetros (`d`, `L`, `dk`, `k`, `s`, `m`, `d1`, `d2`, `h`...)
//! e os comandos usam expressões deles; mudar `L` no documento recompõe a peça. Modelo simplificado:
//! sem rosca, chanfros, filetes sob a cabeça nem face de arruela.

use crate::biblioteca::{self, Familia, Tamanho, TipoFixador};
use fenkra_core::*;

/// Número de lados do sextavado.
const LADOS: &str = "6";

/// Geração dos geradores de geometria. Sobe quando a forma de uma peça gerada muda (novo chanfro,
/// novo eixo de apoio...): peças gravadas com número menor aparecem como desatualizadas.
pub const VERSAO_BIBLIOTECA: u32 = 2;

/// Nomes das propriedades personalizadas que carregam o selo da biblioteca na peça gerada.
const SELO_GERACAO: &str = "Biblioteca: geração";
const SELO_FAMILIA: &str = "Biblioteca: família";

/// Documento de um fixador da biblioteca. `comprimento` (L, mm) vale só para parafusos: `None`
/// escolhe o primeiro comprimento da série do tamanho com `L >= 4·d`; um valor explícito precisa
/// estar entre o menor e o maior da série do tamanho. Para porcas e arruelas o comprimento é ignorado.
pub fn fixador_documento(familia_id: &str, tamanho: &str, comprimento: Option<f64>) -> Result<Document, String> {
    let fam = biblioteca::familia(familia_id).ok_or_else(|| format!("família de fixador desconhecida: {familia_id}"))?;
    gerar(fam, tamanho, comprimento)
}

/// Como [`fixador_documento`], para uma família já em mãos (de qualquer camada).
pub fn gerar(fam: &Familia, tamanho: &str, comprimento: Option<f64>) -> Result<Document, String> {
    let t = fam.tamanho(tamanho).ok_or_else(|| format!("{}: tamanho {tamanho} não existe", fam.norma))?;
    let d = t.diametro_nominal().ok_or_else(|| format!("{}: designação inválida {}", fam.id, t.designacao))?;
    let curta = fam.norma.split(" (").next().unwrap_or(&fam.norma).to_string();

    let (desc, comp) = if matches!(fam.tipo, TipoFixador::CilindroPneumatico | TipoFixador::TrilhoLinear) {
        // o comprimento é o curso do cilindro ou o comprimento do trilho: livre entre o menor e o
        // maior da série; sem pedido, 100 mm ou o primeiro acima disso
        let serie = fam.comprimentos_de(t);
        let (minimo, maximo) = fam.limites_de_comprimento(t).ok_or("sem comprimentos para o tamanho")?;
        let c = comprimento.or_else(|| serie.iter().copied().find(|c| *c >= 100.)).or(serie.last().copied()).ok_or("sem comprimentos para o tamanho")?;
        let nome = if fam.tipo == TipoFixador::TrilhoLinear { "comprimento" } else { "curso" };
        if !c.is_finite() || c < minimo || c > maximo {
            return Err(format!("{} {}: {nome} {c} fora de {} a {} mm", curta, t.designacao, minimo, maximo));
        }
        (format!("{curta} {}x{c}", t.designacao), Some(c))
    } else if fam.tipo.e_parafuso() || fam.tipo == TipoFixador::Rebite {
        let (minimo, maximo) = fam.limites_de_comprimento(t).ok_or("sem comprimentos para o tamanho")?;
        let l = match comprimento {
            Some(l) => l,
            // rebite sem pedido: o L que fecha uma pilha de um diâmetro
            None if fam.tipo == TipoFixador::Rebite => biblioteca::rebite_pela_pega(fam, t, d).or(fam.comprimentos_de(t).last().copied()).ok_or("sem comprimentos para o tamanho")?,
            None => {
                let serie = fam.comprimentos_de(t);
                serie.iter().copied().find(|l| *l >= 4. * d).or(serie.last().copied()).ok_or("sem comprimentos para o tamanho")?
            }
        };
        if !l.is_finite() || l < minimo || l > maximo {
            return Err(format!("{} {}: comprimento {l} fora de {} a {} mm", curta, t.designacao, minimo, maximo));
        }
        (format!("{curta} {}x{l}", t.designacao), Some(l))
    } else {
        (format!("{curta} {}", t.designacao), None)
    };

    let mut m = Montador::novo(&desc);
    m.param("d", &d.to_string())?;
    // a forma vem do gerador; o tipo só diz como a montagem usa a peça
    let l = comp.unwrap_or(0.);
    match fam.gerador.as_str() {
        "parafuso_allen" => allen(&mut m, fam, t, l)?,
        "parafuso_sextavado" => sextavado(&mut m, fam, t, l)?,
        "parafuso_escareado" => escareado(&mut m, fam, t, l)?,
        "parafuso_cruz" => cruz(&mut m, fam, t, l)?,
        "parafuso_ombro" => ombro(&mut m, fam, t, l)?,
        "prisioneiro" => prisioneiro(&mut m, fam, t, l)?,
        "porca" => porca(&mut m, fam, t)?,
        "porca_flangeada" => porca_flangeada(&mut m, fam, t)?,
        "arruela" => arruela(&mut m, fam, t)?,
        "rolamento" => {
            m.param("D", &medida(fam, t, "D")?.to_string())?;
            m.param("B", &medida(fam, t, "B")?.to_string())?;
            crate::rolamentos::corpo(&mut m)?;
        }
        "cilindro_pneumatico" => crate::pneumatica::corpo(&mut m, fam, t, l)?,
        "trilho_linear" => crate::guias::corpo(&mut m, fam, t, l)?,
        "rebite_universal" | "rebite_escareado" => crate::rebites::corpo(&mut m, fam, t, l)?,
        g => return Err(format!("{}: gerador '{g}' sem geometria", fam.id)),
    }
    propriedades(&mut m.doc, fam, t);
    publicacoes(&mut m, fam)?;
    Ok(m.doc)
}

/// PN, material pela classe, acabamento, norma com revisão, massa nominal e o selo da biblioteca.
fn propriedades(doc: &mut Document, fam: &Familia, t: &Tamanho) {
    if let Some(pn) = &t.pn {
        doc.props.pn = pn.clone();
    }
    if let Some(c) = &fam.classe {
        doc.props.material = biblioteca::material_da_classe(c);
    }
    let mut extra = vec![("Norma".to_string(), format!("{}:{}", fam.norma, fam.revisao))];
    if let Some(c) = &fam.classe {
        extra.push(("Classe".into(), c.clone()));
    }
    if let Some(a) = &fam.acabamento {
        extra.push(("Acabamento".into(), a.clone()));
    }
    if let Some(g) = t.massa_g {
        extra.push(("Massa nominal (g)".into(), g.to_string()));
    }
    extra.push((SELO_GERACAO.into(), VERSAO_BIBLIOTECA.to_string()));
    extra.push((SELO_FAMILIA.into(), format!("{}@{}", fam.id, fam.versao)));
    doc.props.custom.extend(extra);
}

/// Publica o eixo, o plano de apoio e (nos parafusos) a ponta, para a montagem se prender a eles
/// por link. Todos seguem os parâmetros: mudar `L` move a ponta.
fn publicacoes(m: &mut Montador, fam: &Familia) -> Result<(), String> {
    let (inicio, fim) = match (fam.gerador.as_str(), fam.tipo) {
        // o escareado fica todo abaixo da face (cabeça rente); o ombro tem a rosca depois do ombro
        ("parafuso_escareado" | "rebite_escareado", _) => ("-L", "0"),
        (_, TipoFixador::Rebite) => ("-L", "k"),
        ("parafuso_ombro", _) => ("-L-b", "k"),
        ("prisioneiro", _) => ("-L", "bm"),
        (_, TipoFixador::ParafusoAllen | TipoFixador::ParafusoSextavado | TipoFixador::ParafusoCruz | TipoFixador::Prisioneiro) => ("-L", "k"),
        (_, TipoFixador::Porca) => ("-m", "0"),
        (_, TipoFixador::Arruela) => ("-h", "0"),
        (_, TipoFixador::Rolamento) => ("-B", "0"),
        // a face traseira apoia; a haste sai para +Z e a ponta segue o curso e a posição s
        (_, TipoFixador::CilindroPneumatico) => ("L8+WH+curso*s+AM", "0"),
        // o trilho apoia pela base; o eixo publicado é a normal da base, no meio do trilho
        (_, TipoFixador::TrilhoLinear) => ("HR", "0"),
    };
    let ponto0 = PointRef::Origin;
    m.publicar("Eixo", RefKind::LineDir { point: ponto0, dir: AxisRef::Origin(OriginAxis::Z), start: inicio.into(), end: fim.into() });
    m.publicar("Plano_apoio", RefKind::OffsetPlane { base: PlaneRef::Origin(OriginPlane::XY), offset: "0".into() });
    if fam.tipo.usa_comprimento() {
        m.publicar("Ponta", RefKind::PointCoords { x: "0".into(), y: "0".into(), z: inicio.into(), frame: None });
    }
    if fam.gerador == "parafuso_ombro" {
        // fim do ombro: o contato da peça que gira no ombro
        m.publicar("Plano_ombro", RefKind::OffsetPlane { base: PlaneRef::Origin(OriginPlane::XY), offset: "-L".into() });
    }
    Ok(())
}

/// Situação do selo de uma peça de fixador gerada pela biblioteca.
#[derive(Clone, Debug, PartialEq)]
pub enum SituacaoSelo {
    Atual,
    /// a peça é mais antiga que os geradores ou que os dados da família; o texto diz por quê
    Desatualizada(String),
    /// a peça não vem da biblioteca (sem selo)
    SemSelo,
}

/// Compara o selo gravado na peça com a biblioteca de agora.
pub fn situacao_do_selo(doc: &Document) -> SituacaoSelo {
    let get = |n: &str| doc.props.custom.iter().find(|(k, _)| k == n).map(|(_, v)| v.as_str());
    let (Some(g), Some(f)) = (get(SELO_GERACAO), get(SELO_FAMILIA)) else { return SituacaoSelo::SemSelo };
    let geracao: u32 = g.parse().unwrap_or(0);
    if geracao < VERSAO_BIBLIOTECA {
        return SituacaoSelo::Desatualizada(format!("gerada com a versão {geracao} da biblioteca; a atual é a {VERSAO_BIBLIOTECA}"));
    }
    let (id, v) = f.split_once('@').unwrap_or((f, "0"));
    match biblioteca::familia(id) {
        Some(fam) if v.parse::<u32>().unwrap_or(0) < fam.versao => SituacaoSelo::Desatualizada(format!("{id} está na versão {v} dos dados; a atual é a {}", fam.versao)),
        Some(_) => SituacaoSelo::Atual,
        None => SituacaoSelo::Desatualizada(format!("a família {id} não existe mais na biblioteca")),
    }
}

/// ISO 4762: cabeça cilíndrica dk × k com sextavado interno de abertura s e profundidade k/2.
fn allen(m: &mut Montador, fam: &Familia, t: &Tamanho, l: f64) -> Result<(), String> {
    let (dk, k, s) = (medida(fam, t, "dk")?, medida(fam, t, "k")?, medida(fam, t, "s")?);
    if s >= dk {
        return Err(format!("{} {}: sextavado interno maior que a cabeça", fam.norma, t.designacao));
    }
    m.param("L", &l.to_string())?;
    m.param("P", &medida(fam, t, "P")?.to_string())?;
    m.param("dk", &dk.to_string())?;
    m.param("k", &k.to_string())?;
    m.param("s", &s.to_string())?;
    // sem "b" na norma (cabeça abaulada), a rosca de referência é o corpo inteiro
    m.param("b", &t.valor("b").unwrap_or(l).to_string())?;
    m.param("prof", "k/2")?;
    let topo = PlaneRef::Origin(OriginPlane::XY);
    m.prim("Cabeca", PrimKind::Cilindro, topo, ["dk/2", "k", "0", "0"], BoolOp::Add)?;
    let p_corpo = m.plano("Plano_corpo", "-L")?;
    m.prim("Corpo", PrimKind::Cilindro, p_corpo, ["d/2", "L", "0", "0"], BoolOp::Add)?;
    // o sextavado interno sobra 1 mm acima do topo para o corte não ficar rente à face
    let p_soquete = m.plano("Plano_sextavado", "k-prof")?;
    m.prim("Sextavado_interno", PrimKind::Prisma, p_soquete, [LADOS, "s/(2*cos(30))", "prof+1", "0"], BoolOp::Cut)
}

/// ISO 4017 / DIN 933: cabeça sextavada (s entre faces, altura k) e haste.
fn sextavado(m: &mut Montador, fam: &Familia, t: &Tamanho, l: f64) -> Result<(), String> {
    m.param("L", &l.to_string())?;
    m.param("P", &medida(fam, t, "P")?.to_string())?;
    m.param("s", &medida(fam, t, "s")?.to_string())?;
    m.param("k", &medida(fam, t, "k")?.to_string())?;
    // canto a canto: não vem da tabela, é e = s / cos(30°)
    m.param("cc", "s/cos(30)")?;
    let topo = PlaneRef::Origin(OriginPlane::XY);
    m.prim("Cabeca", PrimKind::Prisma, topo, [LADOS, "cc/2", "k", "0"], BoolOp::Add)?;
    let p_corpo = m.plano("Plano_corpo", "-L")?;
    m.prim("Corpo", PrimKind::Cilindro, p_corpo, ["d/2", "L", "0", "0"], BoolOp::Add)
}

/// ISO 10642: cabeça escareada de 90° rente à face (z de -k a 0, de Ø d a Ø dk) e sextavado interno.
/// O `L` da norma conta a cabeça: a haste vai de -L até o início do cone.
fn escareado(m: &mut Montador, fam: &Familia, t: &Tamanho, l: f64) -> Result<(), String> {
    let (dk, k, s) = (medida(fam, t, "dk")?, medida(fam, t, "k")?, medida(fam, t, "s")?);
    if s >= dk || l <= k {
        return Err(format!("{} {}: sextavado maior que a cabeça ou comprimento menor que a cabeça", fam.norma, t.designacao));
    }
    m.param("L", &l.to_string())?;
    m.param("P", &medida(fam, t, "P")?.to_string())?;
    m.param("dk", &dk.to_string())?;
    m.param("k", &k.to_string())?;
    m.param("s", &s.to_string())?;
    // a profundidade t mudou entre revisões da norma; o modelo usa uma fração fixa da cabeça
    m.param("prof", "0.6*k")?;
    let p_corpo = m.plano("Plano_corpo", "-L")?;
    m.prim("Corpo", PrimKind::Cilindro, p_corpo, ["d/2", "L-k", "0", "0"], BoolOp::Add)?;
    let p_cabeca = m.plano("Plano_cabeca", "-k")?;
    m.prim("Cabeca", PrimKind::Cone, p_cabeca, ["d/2", "dk/2", "k", "0"], BoolOp::Add)?;
    let p_soquete = m.plano("Plano_sextavado", "-prof")?;
    m.prim("Sextavado_interno", PrimKind::Prisma, p_soquete, [LADOS, "s/(2*cos(30))", "prof+1", "0"], BoolOp::Cut)
}

/// ISO 7045: cabeça panela simplificada (cilindro dk x k) com recesso cruzado em duas fendas.
fn cruz(m: &mut Montador, fam: &Familia, t: &Tamanho, l: f64) -> Result<(), String> {
    m.param("L", &l.to_string())?;
    m.param("P", &medida(fam, t, "P")?.to_string())?;
    m.param("dk", &medida(fam, t, "dk")?.to_string())?;
    m.param("k", &medida(fam, t, "k")?.to_string())?;
    // recesso: braços de 0,5·dk por 0,12·dk, até a metade da altura da cabeça (desenho, não tabela)
    m.param("prof", "k/2")?;
    let topo = PlaneRef::Origin(OriginPlane::XY);
    m.prim("Cabeca", PrimKind::Cilindro, topo, ["dk/2", "k", "0", "0"], BoolOp::Add)?;
    let p_corpo = m.plano("Plano_corpo", "-L")?;
    m.prim("Corpo", PrimKind::Cilindro, p_corpo, ["d/2", "L", "0", "0"], BoolOp::Add)?;
    let p_recesso = m.plano("Plano_recesso", "k-prof")?;
    m.prim("Recesso_x", PrimKind::Caixa, p_recesso.clone(), ["0.5*dk", "0.12*dk", "prof+1", "0"], BoolOp::Cut)?;
    m.prim("Recesso_y", PrimKind::Caixa, p_recesso, ["0.12*dk", "0.5*dk", "prof+1", "0"], BoolOp::Cut)
}

/// ISO 7379: cabeça Allen, ombro de Ø d (= ds) e comprimento L, e a rosca menor Ø dr de comprimento b
/// depois do ombro.
fn ombro(m: &mut Montador, fam: &Familia, t: &Tamanho, l: f64) -> Result<(), String> {
    let (dk, k, s, dr) = (medida(fam, t, "dk")?, medida(fam, t, "k")?, medida(fam, t, "s")?, medida(fam, t, "dr")?);
    let d = t.diametro_nominal().unwrap_or(0.);
    if s >= dk || dr >= d || d >= dk {
        return Err(format!("{} {}: precisa de s < dk e rosca < ombro < cabeça", fam.norma, t.designacao));
    }
    m.param("L", &l.to_string())?;
    m.param("P", &medida(fam, t, "P")?.to_string())?;
    m.param("dk", &dk.to_string())?;
    m.param("k", &k.to_string())?;
    m.param("s", &s.to_string())?;
    m.param("dr", &dr.to_string())?;
    m.param("b", &medida(fam, t, "b")?.to_string())?;
    m.param("prof", "k/2")?;
    let topo = PlaneRef::Origin(OriginPlane::XY);
    m.prim("Cabeca", PrimKind::Cilindro, topo, ["dk/2", "k", "0", "0"], BoolOp::Add)?;
    let p_ombro = m.plano("Plano_ombro_base", "-L")?;
    m.prim("Ombro", PrimKind::Cilindro, p_ombro, ["d/2", "L", "0", "0"], BoolOp::Add)?;
    let p_rosca = m.plano("Plano_rosca", "-L-b")?;
    m.prim("Rosca", PrimKind::Cilindro, p_rosca, ["dr/2", "b", "0", "0"], BoolOp::Add)?;
    let p_soquete = m.plano("Plano_sextavado", "k-prof")?;
    m.prim("Sextavado_interno", PrimKind::Prisma, p_soquete, [LADOS, "s/(2*cos(30))", "prof+1", "0"], BoolOp::Cut)
}

/// DIN 938 / 939: haste Ø d de z = -L (lado da porca) a z = bm (ponta fixa, no lugar da cabeça).
/// O plano de apoio z = 0 é a face da peça roscada onde a ponta fixa entra.
fn prisioneiro(m: &mut Montador, fam: &Familia, t: &Tamanho, l: f64) -> Result<(), String> {
    m.param("L", &l.to_string())?;
    m.param("P", &medida(fam, t, "P")?.to_string())?;
    m.param("b", &medida(fam, t, "b")?.to_string())?;
    m.param("bm", &medida(fam, t, "bm")?.to_string())?;
    let p_corpo = m.plano("Plano_corpo", "-L")?;
    m.prim("Corpo", PrimKind::Cilindro, p_corpo, ["d/2", "L+bm", "0", "0"], BoolOp::Add)
}

/// DIN 6923: porca sextavada s x m com aba Ø dc x c rente à face de apoio (z = 0).
fn porca_flangeada(m: &mut Montador, fam: &Familia, t: &Tamanho) -> Result<(), String> {
    let (s, dc, c, mm) = (medida(fam, t, "s")?, medida(fam, t, "dc")?, medida(fam, t, "c")?, medida(fam, t, "m")?);
    if dc <= s / 30_f64.to_radians().cos() || c >= mm {
        return Err(format!("{} {}: a aba precisa passar dos cantos do sextavado e ser mais fina que a porca", fam.norma, t.designacao));
    }
    m.param("s", &s.to_string())?;
    m.param("m", &mm.to_string())?;
    m.param("dc", &dc.to_string())?;
    m.param("c", &c.to_string())?;
    m.param("cc", "s/cos(30)")?;
    let p_corpo = m.plano("Plano_porca", "-m")?;
    m.prim("Corpo", PrimKind::Prisma, p_corpo, [LADOS, "cc/2", "m", "0"], BoolOp::Add)?;
    let p_aba = m.plano("Plano_aba", "-c")?;
    m.prim("Aba", PrimKind::Cilindro, p_aba, ["dc/2", "c", "0", "0"], BoolOp::Add)?;
    let p_furo = m.plano("Plano_furo", "-m-1")?;
    m.prim("Furo", PrimKind::Cilindro, p_furo, ["d/2", "m+2", "0", "0"], BoolOp::Cut)
}

/// ISO 4032 / DIN 934: prisma sextavado s × m com furo de diâmetro d, de z = -m a z = 0.
fn porca(m: &mut Montador, fam: &Familia, t: &Tamanho) -> Result<(), String> {
    m.param("s", &medida(fam, t, "s")?.to_string())?;
    m.param("m", &medida(fam, t, "m")?.to_string())?;
    m.param("cc", "s/cos(30)")?;
    let p_corpo = m.plano("Plano_porca", "-m")?;
    m.prim("Corpo", PrimKind::Prisma, p_corpo, [LADOS, "cc/2", "m", "0"], BoolOp::Add)?;
    let p_furo = m.plano("Plano_furo", "-m-1")?;
    m.prim("Furo", PrimKind::Cilindro, p_furo, ["d/2", "m+2", "0", "0"], BoolOp::Cut)
}

/// ISO 7089: anel de diâmetro interno d1, externo d2 e espessura h, de z = -h a z = 0.
fn arruela(m: &mut Montador, fam: &Familia, t: &Tamanho) -> Result<(), String> {
    m.param("d1", &medida(fam, t, "d1")?.to_string())?;
    m.param("d2", &medida(fam, t, "d2")?.to_string())?;
    m.param("h", &medida(fam, t, "h")?.to_string())?;
    let p_corpo = m.plano("Plano_arruela", "-h")?;
    m.prim("Corpo", PrimKind::Cilindro, p_corpo, ["d2/2", "h", "0", "0"], BoolOp::Add)?;
    let p_furo = m.plano("Plano_furo", "-h-1")?;
    m.prim("Furo", PrimKind::Cilindro, p_furo, ["d1/2", "h+2", "0", "0"], BoolOp::Cut)
}

fn medida(fam: &Familia, t: &Tamanho, campo: &str) -> Result<f64, String> {
    t.valor(campo).ok_or_else(|| format!("{} {}: falta '{campo}'", fam.norma, t.designacao))
}

/// Monta o documento: propriedades da peça, parâmetros, planos de apoio ocultos e primitivas.
pub(crate) struct Montador {
    pub(crate) doc: Document,
}

impl Montador {
    pub(crate) fn novo(descricao: &str) -> Self {
        let mut doc = Document::new(descricao);
        doc.props.pn = String::new();
        doc.props.description = descricao.into();
        doc.props.material = "Aço 1020".into();
        doc.props.part_type = PartType::Hardware;
        Montador { doc }
    }

    pub(crate) fn param(&mut self, nome: &str, expr: &str) -> Result<(), String> {
        self.doc.add_param(nome, expr, Unit::Comprimento).map(|_| ()).map_err(|e| e.to_string())
    }

    /// Plano paralelo ao XY deslocado por `desvio` (expressão), oculto na viewport.
    pub(crate) fn plano(&mut self, nome: &str, desvio: &str) -> Result<PlaneRef, String> {
        let id = self.doc.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), desvio);
        let r = self.doc.references.iter_mut().find(|r| r.id == id).ok_or("plano não criado")?;
        r.name = nome.into();
        r.visible = false;
        Ok(PlaneRef::Datum(id))
    }

    /// Elemento de referência publicado (outras peças o usam por link), oculto na viewport.
    pub(crate) fn publicar(&mut self, nome: &str, kind: RefKind) {
        let id = self.doc.add_geometry(kind);
        if let Some(r) = self.doc.references.iter_mut().find(|r| r.id == id) {
            r.name = nome.into();
            r.visible = false;
            r.published = true;
        }
    }

    pub(crate) fn prim(&mut self, nome: &str, kind: PrimKind, plane: PlaneRef, v: [&str; 4], op: BoolOp) -> Result<(), String> {
        let [a, b, c, d] = v.map(String::from);
        let id = self.doc.add_feature(FeatureKind::Primitive(Primitive { kind, plane, a, b, c, d, op }));
        let f = self.doc.features.iter_mut().find(|f| f.id == id).ok_or("comando não criado")?;
        f.name = nome.into();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{recompute, Status};
    use std::f64::consts::PI;

    /// Recompute com todos os comandos em Ok; devolve (volume, mín, máx) do corpo único.
    fn medir(doc: &Document) -> (f64, [f64; 3], [f64; 3]) {
        let r = recompute(doc);
        assert!(r.param_error.is_none(), "{:?}", r.param_error);
        for f in &doc.features {
            assert_eq!(r.status[&f.id], Status::Ok, "{} / {}", doc.name, f.name);
        }
        assert_eq!(r.bodies.len(), 1, "{}", doc.name);
        let b = &r.bodies[0];
        let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
        for p in b.mesh.positions.chunks_exact(3) {
            for i in 0..3 {
                lo[i] = lo[i].min(p[i] as f64);
                hi[i] = hi[i].max(p[i] as f64);
            }
        }
        (b.mass.volume, lo, hi)
    }

    fn param(doc: &mut Document, nome: &str, expr: &str) {
        doc.params.iter_mut().find(|p| p.name == nome).unwrap_or_else(|| panic!("sem parâmetro {nome}")).expr = expr.into();
    }

    #[test]
    fn propriedades_e_parametros() {
        let d = fixador_documento("iso4762", "M6", Some(20.)).unwrap();
        assert_eq!(d.props.description, "ISO 4762 M6x20");
        assert!(d.props.pn.is_empty());
        assert!(d.props.material.starts_with("Aço"));
        for n in ["d", "L", "dk", "k", "s"] {
            assert!(d.params.iter().any(|p| p.name == n), "{n}");
        }
        assert_eq!(fixador_documento("iso4032", "M8", None).unwrap().props.description, "ISO 4032 M8");
        // comprimento padrão: primeiro da série com L >= 4·d (M6 → 25)
        assert_eq!(fixador_documento("iso4017", "M6", None).unwrap().props.description, "ISO 4017 M6x25");
    }

    #[test]
    fn propriedades_da_biblioteca_e_selo() {
        let d = fixador_documento("iso4017", "M6", Some(20.)).unwrap();
        assert_eq!(d.props.material, "Aço classe 8.8");
        let custom = |n: &str| d.props.custom.iter().find(|(k, _)| k == n).map(|(_, v)| v.clone());
        assert_eq!(custom("Norma").as_deref(), Some("ISO 4017:2014"));
        assert_eq!(custom("Classe").as_deref(), Some("8.8"));
        assert_eq!(situacao_do_selo(&d), SituacaoSelo::Atual);
        // geração antiga e versão antiga dos dados aparecem como desatualizadas
        let mut velho = d.clone();
        velho.props.custom.iter_mut().find(|(k, _)| k == SELO_GERACAO).unwrap().1 = "1".into();
        assert!(matches!(situacao_do_selo(&velho), SituacaoSelo::Desatualizada(m) if m.contains("versão 1")));
        let mut sem_familia = d.clone();
        sem_familia.props.custom.iter_mut().find(|(k, _)| k == SELO_FAMILIA).unwrap().1 = "nada@1".into();
        assert!(matches!(situacao_do_selo(&sem_familia), SituacaoSelo::Desatualizada(_)));
        let mut dados = d.clone();
        dados.props.custom.iter_mut().find(|(k, _)| k == SELO_FAMILIA).unwrap().1 = "iso4017@0".into();
        assert!(matches!(situacao_do_selo(&dados), SituacaoSelo::Desatualizada(_)));
        assert_eq!(situacao_do_selo(&Document::new("avulsa")), SituacaoSelo::SemSelo);
        // material pela classe também nas porcas e arruelas
        assert_eq!(fixador_documento("iso4032", "M8", None).unwrap().props.material, "Aço classe 8");
    }

    #[test]
    fn publicacoes_do_fixador() {
        let d = fixador_documento("iso4762", "M6", Some(20.)).unwrap();
        let pub_ = |n: &str| d.references.iter().find(|r| r.name == n).unwrap_or_else(|| panic!("sem publicação {n}"));
        for n in ["Eixo", "Plano_apoio", "Ponta"] {
            let r = pub_(n);
            assert!(r.published && !r.visible, "{n}");
        }
        assert!(matches!(&pub_("Ponta").kind, RefKind::PointCoords { z, .. } if z == "-L"));
        // porca e arruela não têm ponta
        assert!(!fixador_documento("iso4032", "M8", None).unwrap().references.iter().any(|r| r.name == "Ponta"));
    }

    #[test]
    fn serie_de_comprimentos_por_tamanho() {
        // M3 vai de 6 a 30, M20 de 40 a 120 (a série publicada para o tamanho, sem regra geral)
        assert!(fixador_documento("iso4762", "M3", Some(6.)).is_ok());
        assert!(fixador_documento("iso4762", "M3", Some(31.)).is_err());
        assert!(fixador_documento("iso4762", "M20", Some(30.)).is_err());
        assert!(fixador_documento("iso4762", "M20", Some(120.)).is_ok());
        // designação livre
        assert!(fixador_documento("iso4762", "m 6", Some(20.)).is_ok());
    }

    #[test]
    fn massa_nominal_confere_com_o_volume() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        // só vale para itens sem comprimento que trazem massa_g na biblioteca
        for f in biblioteca::familias().iter().filter(|f| !f.tipo.e_parafuso()) {
            for t in f.tamanhos.iter().filter(|t| t.massa_g.is_some()) {
                let (v, _, _) = medir(&gerar(f, &t.designacao, None).unwrap());
                let g = v * biblioteca::DENSIDADE_ACO_G_MM3;
                let nominal = t.massa_g.unwrap();
                assert!((g - nominal).abs() / nominal < 0.15, "{} {}: {g:.2} g contra {nominal} g", f.id, t.designacao);
            }
        }
    }

    #[test]
    fn entradas_invalidas() {
        assert!(fixador_documento("nada", "M6", None).is_err());
        assert!(fixador_documento("iso4762", "M7", None).is_err());
        assert!(fixador_documento("iso4762", "M6", Some(10.)).is_err(), "abaixo da série");
        assert!(fixador_documento("iso4762", "M6", Some(61.)).is_err(), "acima da série");
        assert!(fixador_documento("iso4762", "M3", Some(5.)).is_err(), "mínimo de 6 mm");
        assert!(fixador_documento("iso4762", "M6", Some(f64::NAN)).is_err());
        // porca e arruela ignoram o comprimento
        assert!(fixador_documento("iso7089", "M6", Some(1000.)).is_ok());
    }

    #[test]
    fn allen_m6x20() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let mut d = fixador_documento("iso4762", "M6", Some(20.)).unwrap();
        let (v, lo, hi) = medir(&d);
        // cabeça 6 acima de z = 0 e corpo 20 abaixo; Ø10 (tolerância da malha)
        assert!((lo[2] + 20.).abs() < 1e-3 && (hi[2] - 6.).abs() < 1e-3, "z {lo:?} {hi:?}");
        assert!((hi[0] - lo[0] - 10.).abs() < 0.2 && (hi[1] - lo[1] - 10.).abs() < 0.2, "xy {lo:?} {hi:?}");
        // cabeça + haste − sextavado interno (área de hexágono de abertura 5, profundidade 3)
        let esperado = PI / 4. * 100. * 6. + PI / 4. * 36. * 20. - 3_f64.sqrt() / 2. * 25. * 3.;
        assert!((v - esperado).abs() / esperado < 0.01, "volume {v} vs {esperado}");
        // paramétrico: L = 30 alonga a haste
        param(&mut d, "L", "30");
        let (_, lo, hi) = medir(&d);
        assert!((lo[2] + 30.).abs() < 1e-3 && (hi[2] - 6.).abs() < 1e-3);
    }

    #[test]
    fn porca_m8_iso() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let d = fixador_documento("iso4032", "M8", None).unwrap();
        let (v, lo, hi) = medir(&d);
        assert!((hi[2] - lo[2] - 6.8).abs() < 1e-3 && hi[2].abs() < 1e-3, "z {lo:?} {hi:?}");
        // vértice em +X: entre faces é a extensão em Y; canto a canto, em X
        assert!((hi[1] - lo[1] - 13.).abs() < 1e-3, "entre faces {lo:?} {hi:?}");
        assert!((hi[0] - lo[0] - 13. / (30_f64.to_radians().cos())).abs() < 1e-3, "canto a canto {lo:?} {hi:?}");
        let esperado = (3_f64.sqrt() / 2. * 169. - PI / 4. * 64.) * 6.8;
        assert!((v - esperado).abs() / esperado < 0.01, "volume {v} vs {esperado}");
    }

    #[test]
    fn arruela_m10() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let d = fixador_documento("iso7089", "M10", None).unwrap();
        let (v, lo, hi) = medir(&d);
        let esperado = PI / 4. * (20. * 20. - 10.5 * 10.5) * 2.;
        assert!((v - esperado).abs() / esperado < 0.01, "volume {v} vs {esperado}");
        assert!((lo[2] + 2.).abs() < 1e-3 && hi[2].abs() < 1e-3);
    }

    #[test]
    fn parafuso_sextavado_din_e_iso() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        // M10: a cabeça DIN tem s = 17 e a ISO, 16 (extensão em Y)
        let (_, lo, hi) = medir(&fixador_documento("din933", "M10", Some(30.)).unwrap());
        assert!((hi[1] - lo[1] - 17.).abs() < 1e-3 && (hi[2] - 6.4).abs() < 1e-3 && (lo[2] + 30.).abs() < 1e-3);
        let (_, lo, hi) = medir(&fixador_documento("iso4017", "M10", Some(30.)).unwrap());
        assert!((hi[1] - lo[1] - 16.).abs() < 1e-3);
    }

    #[test]
    fn familias_do_b51_geram_com_as_cotas_da_norma() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        // ISO 4014 M10x40: cabeça s = 16 e k = 6,4, haste 40 abaixo da face de apoio
        let (_, lo, hi) = medir(&fixador_documento("iso4014", "M10", Some(40.)).unwrap());
        assert!((hi[1] - lo[1] - 16.).abs() < 1e-3 && (hi[2] - 6.4).abs() < 1e-3 && (lo[2] + 40.).abs() < 1e-3, "{lo:?} {hi:?}");
        // comprimento igual ou menor que b não é rosca parcial: fora da série
        assert!(fixador_documento("iso4014", "M10", Some(20.)).is_err());
        // ISO 4035 M12: baixa, 6 mm de altura contra 10,8 da ISO 4032, mesma abertura s = 18
        let (v, lo, hi) = medir(&fixador_documento("iso4035", "M12", None).unwrap());
        assert!((hi[2] - lo[2] - 6.).abs() < 1e-3 && (hi[1] - lo[1] - 18.).abs() < 1e-3, "{lo:?} {hi:?}");
        let esperado = (3_f64.sqrt() / 2. * 324. - PI / 4. * 144.) * 6.;
        assert!((v - esperado).abs() / esperado < 0.01, "volume {v} vs {esperado}");
        // ISO 7380-1 M8x20 (sem "b"): cabeça Ø14 x 4,4 e haste 20
        let d = fixador_documento("iso7380_1", "M8", Some(20.)).unwrap();
        let (_, lo, hi) = medir(&d);
        assert!((hi[2] - 4.4).abs() < 1e-3 && (lo[2] + 20.).abs() < 1e-3 && (hi[0] - lo[0] - 14.).abs() < 0.2, "{lo:?} {hi:?}");
        assert!(fixador_documento("iso7380_1", "M8", Some(8.)).is_err(), "abaixo da série de M8 (10)");
    }

    #[test]
    fn porca_autotravante_din985_gera_na_altura_total() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        // M8: chave 13 e altura total 8; volume do sextavado menos o furo de Ø8 (a rosca não entra)
        let (v, lo, hi) = medir(&fixador_documento("din985", "M8", None).unwrap());
        assert!((hi[2] - lo[2] - 8.).abs() < 1e-3 && (hi[1] - lo[1] - 13.).abs() < 1e-3, "{lo:?} {hi:?}");
        let esperado = (3_f64.sqrt() / 2. * 169. - PI / 4. * 64.) * 8.;
        assert!((v - esperado).abs() / esperado < 0.03, "volume {v} vs {esperado}");
    }

    #[test]
    fn escareado_iso10642_fica_rente_e_tem_o_cone_da_norma() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        // M6x20: L conta a cabeça; nada acima da face; Ø13,44 no topo
        let (v, lo, hi) = medir(&fixador_documento("iso10642", "M6", Some(20.)).unwrap());
        assert!(hi[2].abs() < 1e-3 && (lo[2] + 20.).abs() < 1e-3, "{lo:?} {hi:?}");
        assert!((hi[1] - lo[1] - 13.44).abs() < 0.2, "{lo:?} {hi:?}");
        let (r1, r2, k): (f64, f64, f64) = (3., 6.72, 3.72);
        let prof = 0.6 * k;
        let esperado = PI * 9. * (20. - k) + PI * k / 3. * (r1 * r1 + r1 * r2 + r2 * r2) - 3_f64.sqrt() / 2. * 16. * prof;
        assert!((v - esperado).abs() / esperado < 0.01, "volume {v} vs {esperado}");
    }

    #[test]
    fn panela_iso7045_tira_o_recesso_da_cabeca() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (v, lo, hi) = medir(&fixador_documento("iso7045", "M6", Some(20.)).unwrap());
        assert!((hi[2] - 4.6).abs() < 1e-3 && (lo[2] + 20.).abs() < 1e-3 && (hi[0] - lo[0] - 12.).abs() < 0.2, "{lo:?} {hi:?}");
        // cabeça e haste menos a cruz (dois braços 6 x 1,44 cruzados, 2,3 de fundo)
        let (a, b, p) = (6., 1.44, 2.3);
        let esperado = PI * 36. * 4.6 + PI * 9. * 20. - (2. * a * b - b * b) * p;
        assert!((v - esperado).abs() / esperado < 0.01, "volume {v} vs {esperado}");
    }

    #[test]
    fn ombro_iso7379_tem_ombro_rosca_e_plano_publicado() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let d = fixador_documento("iso7379", "M8 Ø10", Some(30.)).unwrap();
        assert_eq!(d.props.description, "ISO 7379 M8 Ø10x30");
        assert!(d.references.iter().any(|r| r.name == "Plano_ombro" && r.published));
        let (v, lo, hi) = medir(&d);
        // cabeça 7 acima, ombro 30 e rosca 13,25 abaixo
        assert!((hi[2] - 7.).abs() < 1e-3 && (lo[2] + 43.25).abs() < 1e-3 && (hi[0] - lo[0] - 16.).abs() < 0.2, "{lo:?} {hi:?}");
        let esperado = PI * 64. * 7. + PI * 25. * 30. + PI * 16. * 13.25 - 3_f64.sqrt() / 2. * 25. * 3.5;
        assert!((v - esperado).abs() / esperado < 0.01, "volume {v} vs {esperado}");
    }

    #[test]
    fn prisioneiro_din938_e_939_trazem_a_ponta_fixa() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        // DIN 938 M10x40: ponta fixa de 10 acima da face; DIN 939: 12
        let (v, lo, hi) = medir(&fixador_documento("din938", "M10", Some(40.)).unwrap());
        assert!((hi[2] - 10.).abs() < 1e-3 && (lo[2] + 40.).abs() < 1e-3, "{lo:?} {hi:?}");
        assert!((v - PI * 25. * 50.).abs() / v < 0.01);
        let (_, _, hi) = medir(&fixador_documento("din939", "M10", Some(40.)).unwrap());
        assert!((hi[2] - 12.).abs() < 1e-3);
    }

    #[test]
    fn porca_flangeada_din6923_tem_a_aba_na_face_de_apoio() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (v, lo, hi) = medir(&fixador_documento("din6923", "M8", None).unwrap());
        // aba Ø17,9 manda na caixa; altura 8 abaixo da face
        assert!((hi[2] - lo[2] - 8.).abs() < 1e-3 && hi[2].abs() < 1e-3 && (hi[1] - lo[1] - 17.9).abs() < 0.2, "{lo:?} {hi:?}");
        let hex = 3_f64.sqrt() / 2. * 169.;
        let aba = PI / 4. * 17.9 * 17.9;
        let esperado = hex * (8. - 1.2) + aba * 1.2 - PI / 4. * 64. * 8.;
        assert!((v - esperado).abs() / esperado < 0.01, "volume {v} vs {esperado}");
    }

    #[test]
    fn arruelas_do_b51_e_allen_em_polegada() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        // DIN 6798 A M8: anel 8,4 x 15 x 0,8; par de cunha DIN 25201 M10: 10,7 x 16,6 x 2,5
        let (v, _, _) = medir(&fixador_documento("din6798a", "M8", None).unwrap());
        let e = PI / 4. * (225. - 8.4 * 8.4) * 0.8;
        assert!((v - e).abs() / e < 0.01, "{v} {e}");
        let (_, lo, hi) = medir(&fixador_documento("din25201", "M10", None).unwrap());
        assert!((hi[2] - lo[2] - 2.5).abs() < 1e-3 && (hi[0] - lo[0] - 16.6).abs() < 0.2);
        // 1/4-20 UNC: cabeça 0,375" (9,525 mm) por 0,25" (6,35 mm), haste 1" abaixo
        let d = fixador_documento("asme_b18_3", "1/4-20 UNC", Some(25.4)).unwrap();
        assert_eq!(d.props.description, "ASME B18.3 1/4-20 UNCx25.4");
        let (_, lo, hi) = medir(&d);
        assert!((hi[2] - 6.35).abs() < 1e-3 && (lo[2] + 25.4).abs() < 1e-3 && (hi[0] - lo[0] - 9.525).abs() < 0.2, "{lo:?} {hi:?}");
        // pelo valor original também
        assert!(fixador_documento("asme_b18_3", "1/4 pol", Some(25.4)).is_ok());
    }

    #[test]
    fn todos_os_tamanhos_de_todas_as_familias_geram() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        for f in biblioteca::familias() {
            for t in &f.tamanhos {
                let doc = fixador_documento(&f.id, &t.designacao, None).unwrap_or_else(|e| panic!("{} {}: {e}", f.id, t.designacao));
                let (v, _, hi) = medir(&doc);
                assert!(v > 0. && hi[2] >= -1e-6, "{} {}", f.id, t.designacao);
                // e todo comprimento da série do tamanho
                for l in f.comprimentos_de(t).into_iter().filter(|_| f.tipo.atravessa_furo()) {
                    let doc = fixador_documento(&f.id, &t.designacao, Some(l)).unwrap_or_else(|e| panic!("{} {} L{l}: {e}", f.id, t.designacao));
                    let (v, lo, _) = medir(&doc);
                    // o parafuso de ombro tem a rosca b depois do ombro de comprimento L
                    let fundo = l + if f.gerador == "parafuso_ombro" { t.valor("b").unwrap() } else { 0. };
                    assert!(v > 0. && (lo[2] + fundo).abs() < 1e-3, "{} {} L{l}", f.id, t.designacao);
                }
            }
        }
    }
}
