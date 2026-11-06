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

    let (desc, comp) = if fam.tipo.e_parafuso() {
        let (minimo, maximo) = fam.limites_de_comprimento(t).ok_or("sem comprimentos para o tamanho")?;
        let l = match comprimento {
            Some(l) => l,
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
    match fam.tipo {
        TipoFixador::ParafusoAllen => allen(&mut m, fam, t, comp.unwrap_or(0.))?,
        TipoFixador::ParafusoSextavado => sextavado(&mut m, fam, t, comp.unwrap_or(0.))?,
        TipoFixador::Porca => porca(&mut m, fam, t)?,
        TipoFixador::Arruela => arruela(&mut m, fam, t)?,
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
    let (inicio, fim) = match fam.tipo {
        TipoFixador::ParafusoAllen | TipoFixador::ParafusoSextavado => ("-L", "k"),
        TipoFixador::Porca => ("-m", "0"),
        TipoFixador::Arruela => ("-h", "0"),
    };
    let ponto0 = PointRef::Origin;
    m.publicar("Eixo", RefKind::LineDir { point: ponto0, dir: AxisRef::Origin(OriginAxis::Z), start: inicio.into(), end: fim.into() });
    m.publicar("Plano_apoio", RefKind::OffsetPlane { base: PlaneRef::Origin(OriginPlane::XY), offset: "0".into() });
    if fam.tipo.e_parafuso() {
        m.publicar("Ponta", RefKind::PointCoords { x: "0".into(), y: "0".into(), z: "-L".into(), frame: None });
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
                for l in f.comprimentos_de(t).into_iter().filter(|_| f.tipo.e_parafuso()) {
                    let doc = fixador_documento(&f.id, &t.designacao, Some(l)).unwrap_or_else(|e| panic!("{} {} L{l}: {e}", f.id, t.designacao));
                    let (v, lo, _) = medir(&doc);
                    assert!(v > 0. && (lo[2] + l).abs() < 1e-3, "{} {} L{l}", f.id, t.designacao);
                }
            }
        }
    }
}
