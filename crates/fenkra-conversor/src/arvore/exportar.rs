// SPDX-License-Identifier: LGPL-2.1-or-later
//! Do documento ao arquivo de árvore. O formato cobre os comandos comuns de quem migra entre programas;
//! o resto entra em [`Fora`], junto com o que dependia dele, e vai escrito no próprio arquivo: um
//! leitor nunca confunde uma árvore incompleta com uma inteira.

use super::*;
use fenkra_core::sketch::{Constraint, Curve, Kind};
use fenkra_core::{AxisRef, BoolOp, Document, EdgeMode, EdgeProp, EdgeSel, Extent, ExtrudeEnds, Feature, FeatureKind, HoleKind, Id, OriginAxis, OriginPlane, Parameter, PatternKind, PlaneRef, PointRef, PrimKind, RefKind, Sketch, Unit};
use std::collections::HashSet;

/// O que não coube no arquivo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fora {
    pub nome: String,
    pub motivo: String,
}

impl Fora {
    pub fn texto(&self) -> String {
        format!("{}: {}", self.nome, self.motivo)
    }
}

/// Peça traduzida e o que ficou de fora.
#[derive(Clone, Debug, PartialEq)]
pub struct Exportado {
    pub peca: Peca,
    pub fora: Vec<Fora>,
}

impl Exportado {
    /// Arquivo com só esta peça, pronto para gravar.
    pub fn arquivo(self) -> Arquivo {
        let fora = self.fora.iter().map(Fora::texto).collect();
        Arquivo { formato: NOME_FORMATO.into(), format_version: fenkra_core::formato::Tipo::Arvore.versao_atual(), gerador: gerador(), unidades: "mm".into(), pecas: vec![self.peca], montagem: None, fora }
    }
}

pub(super) fn gerador() -> String {
    format!("Fenkra {}", env!("CARGO_PKG_VERSION"))
}

pub(super) fn id(i: Id) -> String {
    format!("e{}", i.0)
}

type Traducao<T> = Result<T, String>;

/// Tradutor de uma peça: sabe o que já entrou, para recusar quem aponta para o que ficou de fora.
struct Tradutor {
    entrou: HashSet<Id>,
}

impl Tradutor {
    fn exige(&self, i: Id) -> Traducao<String> {
        if self.entrou.contains(&i) {
            Ok(id(i))
        } else {
            Err(format!("depende do elemento {} que ficou de fora", id(i)))
        }
    }

    fn plano(&self, p: &PlaneRef) -> Traducao<Plano> {
        Ok(match p {
            PlaneRef::Origin(OriginPlane::XY) => Plano::Origem(PlanoOrigem::XY),
            PlaneRef::Origin(OriginPlane::XZ) => Plano::Origem(PlanoOrigem::XZ),
            PlaneRef::Origin(OriginPlane::YZ) => Plano::Origem(PlanoOrigem::YZ),
            PlaneRef::Datum(i) => Plano::Referencia { referencia: self.exige(*i)? },
        })
    }

    fn eixo(&self, a: &AxisRef) -> Traducao<Eixo> {
        Ok(match a {
            AxisRef::Origin(OriginAxis::X) => Eixo::Origem(EixoOrigem::X),
            AxisRef::Origin(OriginAxis::Y) => Eixo::Origem(EixoOrigem::Y),
            AxisRef::Origin(OriginAxis::Z) => Eixo::Origem(EixoOrigem::Z),
            AxisRef::SketchLine { sketch, entity } => Eixo::Curva { esboco: self.exige(*sketch)?, curva: id(*entity) },
            AxisRef::Ref(i) => Eixo::Referencia { referencia: self.exige(*i)? },
            AxisRef::Fixed { origin, dir } => Eixo::Fixo { origem: *origin, direcao: *dir },
            AxisRef::Face { point, .. } => Eixo::Face { face: *point },
            AxisRef::Edge { mid, .. } => Eixo::Aresta { aresta: *mid },
            AxisRef::Planes { a, b } => Eixo::Planos { planos: Box::new([self.plano(a)?, self.plano(b)?]) },
        })
    }

    fn ponto(&self, p: &PointRef) -> Traducao<Ponto> {
        Ok(match p {
            PointRef::Origin => Ponto::Coordenadas([0.; 3]),
            PointRef::Fixed(c) => Ponto::Coordenadas(*c),
            PointRef::Ref(i) => Ponto::Referencia { referencia: self.exige(*i)? },
            PointRef::SketchPoint { sketch, point } => Ponto::DeEsboco { esboco: self.exige(*sketch)?, ponto: id(*point) },
            PointRef::CurveVertex { .. } | PointRef::Vertex { .. } | PointRef::Center { .. } | PointRef::OnFace { .. } => return Err("ponto tirado de curva ou aresta: o formato guarda só coordenadas e pontos de esboço".into()),
        })
    }

    fn termino(&self, e: &Extent) -> Traducao<Termino> {
        Ok(match e {
            Extent::Blind(d) => Termino::Cego { distancia: d.clone() },
            Extent::Symmetric(d) => Termino::Simetrico { distancia: d.clone() },
            Extent::TwoSides(a, b) => Termino::DoisLados { a: a.clone(), b: b.clone() },
            Extent::ThroughAll => Termino::Atravessar,
            Extent::UpTo(p) => Termino::AtePlano { plano: self.plano(p)? },
            Extent::UpToNext => Termino::AteProxima,
            Extent::UpToLast => Termino::AteUltima,
            Extent::UpToSurface(_) => return Err("término até superfície não existe na versão 1".into()),
        })
    }

    fn definicao(&self, k: &RefKind) -> Traducao<Definicao> {
        Ok(match k {
            RefKind::OffsetPlane { base, offset } => Definicao::PlanoDeslocado { base: self.plano(base)?, deslocamento: offset.clone() },
            RefKind::FacePlane { origin, x, normal, .. } => Definicao::PlanoFixo { origem: *origin, x: *x, normal: *normal },
            RefKind::PointCoords { x, y, z, frame } => Definicao::Ponto { x: x.clone(), y: y.clone(), z: z.clone(), sistema: frame.as_ref().map(|f| self.plano(f)).transpose()? },
            RefKind::LineDir { point, dir, start, end } => Definicao::Reta { ponto: self.ponto(point)?, direcao: self.eixo(dir)?, inicio: start.clone(), fim: end.clone() },
            _ => return Err("referência sem equivalente na versão 1".into()),
        })
    }

    fn fontes(&self, v: &[Id]) -> Traducao<Vec<String>> {
        v.iter().map(|i| self.exige(*i)).collect()
    }

    fn arestas(&self, e: &EdgeSel) -> Traducao<Arestas> {
        Ok(match e {
            EdgeSel::AllOfBody => Arestas::Todas(Todas::Todas),
            EdgeSel::CreatedBy(i) => Arestas::Geradas { geradas_por: self.exige(*i)? },
            EdgeSel::Edges(p) => Arestas::Pontos(p.clone()),
        })
    }

    fn comando(&self, k: &FeatureKind) -> Traducao<Comando> {
        Ok(match k {
            FeatureKind::Sketch(s) => Comando::Esboco(esboco(self, s)?),
            FeatureKind::Extrude(e) => {
                if e.ends != ExtrudeEnds::default() {
                    return Err("início deslocado ou segundo término não existem na versão 1".into());
                }
                Comando::Extrusao {
                    esboco: self.exige(e.sketch)?,
                    operacao: operacao(e.op),
                    termino: self.termino(&e.extent)?,
                    inverter: e.reverse,
                    saida: e.taper.clone(),
                    fina: fina(&e.thin, e.thin_side),
                    direcao: e.dir.as_ref().map(|d| self.eixo(d)).transpose()?,
                }
            }
            FeatureKind::Revolve(r) => Comando::Revolucao {
                esboco: self.exige(r.sketch)?,
                eixo: self.eixo(&r.axis)?,
                angulo: r.angle.clone(),
                angulo2: r.angle2.clone(),
                simetrico: r.symmetric,
                operacao: operacao(r.op),
                fina: fina(&r.thin, r.thin_side),
            },
            FeatureKind::Hole(h) => {
                let cilindrico = h.taper.trim().is_empty() || h.taper.trim() == "0";
                if !h.points.is_empty() || h.dir.is_some() || !cilindrico || h.left_hand || h.thread_modeled || h.extra != Default::default() {
                    return Err("furo com pontos 3D, direção própria, conicidade ou rosca modelada não existe na versão 1".into());
                }
                let forma = match &h.kind {
                    HoleKind::Simples => FormaFuro::Simples,
                    HoleKind::Rebaixado { diameter, depth } => FormaFuro::Rebaixado { diametro: diameter.clone(), profundidade: depth.clone() },
                    HoleKind::Escareado { diameter, angle } => FormaFuro::Escareado { diametro: diameter.clone(), angulo: angle.clone() },
                    HoleKind::Contrafurado { diameter, depth, angle } => FormaFuro::Contrafurado { diametro: diameter.clone(), profundidade: depth.clone(), angulo: angle.clone() },
                };
                Comando::Furo { esboco: self.exige(h.sketch)?, forma, diametro: h.diameter.clone(), termino: self.termino(&h.depth)?, inverter: h.reverse, rosca: h.thread.clone(), ponta: h.tip.clone() }
            }
            FeatureKind::Fillet(o) | FeatureKind::Chamfer(o) => {
                if o.mode != EdgeMode::Constant || o.prop != EdgeProp::Tangente {
                    return Err("só filete e chanfro constantes, com propagação tangente, existem na versão 1".into());
                }
                let arestas = self.arestas(&o.edges)?;
                if matches!(k, FeatureKind::Fillet(_)) {
                    Comando::Filete { raio: o.value.clone(), arestas }
                } else {
                    Comando::Chanfro { distancia: o.value.clone(), arestas }
                }
            }
            FeatureKind::Shell(s) => Comando::Casca { faces: s.faces.clone(), espessura: s.thickness.clone(), para_fora: s.outward, fechada: s.closed },
            FeatureKind::Primitive(p) => Comando::Primitiva { forma: forma(p.kind), plano: self.plano(&p.plane)?, a: p.a.clone(), b: p.b.clone(), c: p.c.clone(), d: p.d.clone(), operacao: operacao(p.op) },
            FeatureKind::Pattern(p) => {
                let forma = match &p.kind {
                    PatternKind::Linear { dir, count, spacing, dir2, count2, spacing2, total } => FormaPadrao::Linear {
                        direcao: self.eixo(dir)?,
                        quantidade: count.clone(),
                        espacamento: spacing.clone(),
                        direcao2: dir2.as_ref().map(|d| self.eixo(d)).transpose()?,
                        quantidade2: count2.clone(),
                        espacamento2: spacing2.clone(),
                        total: *total,
                    },
                    PatternKind::Circular { axis, count, angle } => FormaPadrao::Circular { eixo: self.eixo(axis)?, quantidade: count.clone(), angulo: angle.clone() },
                    PatternKind::Points { .. } => return Err("padrão por pontos não existe na versão 1".into()),
                };
                Comando::Padrao(Padrao { forma, fontes: self.fontes(&p.sources)?, pular: p.skip.iter().map(|(i, j)| [*i, *j]).collect() })
            }
            FeatureKind::Mirror(m) => Comando::Espelho { plano: self.plano(&m.plane)?, fontes: self.fontes(&m.sources)? },
            _ => return Err("comando sem equivalente na versão 1".into()),
        })
    }
}

fn operacao(o: BoolOp) -> Operacao {
    match o {
        BoolOp::Add => Operacao::Unir,
        BoolOp::Cut => Operacao::Cortar,
        BoolOp::Intersect => Operacao::Intersectar,
    }
}

fn fina(espessura: &Option<String>, lado: u8) -> Option<ParedeFina> {
    espessura.as_ref().map(|e| ParedeFina { espessura: e.clone(), lado })
}

fn forma(k: PrimKind) -> Forma {
    match k {
        PrimKind::Caixa => Forma::Caixa,
        PrimKind::Cilindro => Forma::Cilindro,
        PrimKind::Esfera => Forma::Esfera,
        PrimKind::Cone => Forma::Cone,
        PrimKind::Toro => Forma::Toro,
        PrimKind::Cunha => Forma::Cunha,
        PrimKind::Prisma => Forma::Prisma,
    }
}

fn restricao(c: &Constraint) -> Restricao {
    use TipoRestricao as T;
    let tipo = match &c.kind {
        Kind::Coincident(..) => T::Coincidente,
        Kind::Horizontal(_) => T::Horizontal,
        Kind::Vertical(_) => T::Vertical,
        Kind::Parallel(..) => T::Paralela,
        Kind::Perpendicular(..) => T::Perpendicular,
        Kind::Equal(..) => T::Igual,
        Kind::Tangent(..) => T::Tangente,
        Kind::Concentric(..) => T::Concentrica,
        Kind::Midpoint(..) => T::PontoMedio,
        Kind::PointOnCurve(..) => T::SobreCurva,
        Kind::Fixed(_) => T::Fixa,
        Kind::Symmetric(..) => T::Simetrica,
        Kind::HorizontalPts(..) => T::HorizontalPontos,
        Kind::VerticalPts(..) => T::VerticalPontos,
        Kind::Distance(..) => T::Distancia,
        Kind::DistanceX(..) => T::DistanciaX,
        Kind::DistanceY(..) => T::DistanciaY,
        Kind::Length(..) => T::Comprimento,
        Kind::Radius(..) => T::Raio,
        Kind::Diameter(..) => T::Diametro,
        Kind::Angle(..) => T::Angulo,
        Kind::AngleSupp(..) => T::AnguloSuplementar,
        Kind::PointLineDistance(..) => T::DistanciaPontoReta,
        Kind::LineAngle(..) => T::AnguloReta,
    };
    Restricao { tipo, entidades: c.kind.refs().into_iter().map(id).collect(), valor: c.kind.clone().value_mut().map(|v| v.clone()), referencia: c.reference, invertida: c.flipped }
}

fn geometria(g: &Curve) -> Geometria {
    match g {
        Curve::Line { a, b } => Geometria::Reta { a: id(*a), b: id(*b) },
        Curve::Circle { c, r } => Geometria::Circulo { centro: id(*c), raio: *r },
        Curve::Arc { c, s, e } => Geometria::Arco { centro: id(*c), inicio: id(*s), fim: id(*e) },
        Curve::Point { p } => Geometria::Ponto { ponto: id(*p) },
        Curve::Spline { pts } => Geometria::Spline { pontos: pts.iter().map(|p| id(*p)).collect() },
        Curve::Ellipse { c, s, e, major, minor, t0, t1 } => Geometria::Elipse { centro: id(*c), inicio: id(*s), fim: id(*e), maior: *major, menor: *minor, t0: *t0, t1: *t1 },
    }
}

fn esboco(t: &Tradutor, s: &Sketch) -> Traducao<Esboco> {
    // geometria trazida de fora muda com o modelo, e essa ligação não existe no arquivo
    if !s.data.links.is_empty() {
        return Err("esboço com geometria externa não existe na versão 1".into());
    }
    let p = &s.data.pos;
    let posicao = (*p != Default::default()).then(|| Posicao { origem: p.origem, trocar: p.trocar, inverter_h: p.inverter_h, inverter_v: p.inverter_v });
    Ok(Esboco {
        plano: t.plano(&s.plane)?,
        posicao,
        pontos: s.data.points.iter().map(|q| PontoEsboco { id: id(q.id), x: q.x, y: q.y, fixo: q.fixed }).collect(),
        curvas: s.data.curves.iter().map(|c| CurvaEsboco { id: id(c.id), construcao: c.construction, geometria: geometria(&c.geom) }).collect(),
        restricoes: s.data.constraints.iter().map(restricao).collect(),
        tangentes: s.data.spline_tan.iter().map(|k| Tangente { curva: id(k.curve), ponto: id(k.point), direcao: k.dir }).collect(),
    })
}

fn parametro(p: &Parameter) -> Option<Parametro> {
    let grandeza = match p.unit {
        Unit::Comprimento => Grandeza::Comprimento,
        Unit::Angulo => Grandeza::Angulo,
        Unit::Numero => Grandeza::Numero,
    };
    p.tipo.numerico().then(|| Parametro { nome: p.name.clone(), expressao: p.expr.clone(), grandeza, comentario: p.comment.clone() })
}

fn item(t: &Tradutor, f: &Feature) -> Traducao<Item> {
    Ok(Item {
        id: id(f.id),
        nome: f.name.clone(),
        corpo: Some(id(f.target)),
        suprimido: f.suppressed,
        suprimir_se: f.suppress_if.clone(),
        oculto: f.hidden,
        cor: f.appearance.as_ref().map(|a| a.color),
        comando: t.comando(&f.kind)?,
    })
}

/// Traduz a peça. Os pontos de aresta e face valem como estão no documento: quem exporta um documento
/// editado passa antes por [`super::atualizar_pontos`].
pub fn exportar_peca(doc: &Document) -> Exportado {
    let mut t = Tradutor { entrou: HashSet::new() };
    let (mut referencias, mut arvore) = (vec![], vec![]);
    let mut refs_pendentes: Vec<_> = doc.references.iter().collect();
    let mut itens_pendentes: Vec<_> = doc.features.iter().enumerate().collect();
    // esboço pode estar num plano de referência e referência pode usar um esboço: repete até parar
    loop {
        let antes = t.entrou.len();
        refs_pendentes.retain(|r| match t.definicao(&r.kind) {
            Ok(definicao) => {
                t.entrou.insert(r.id);
                referencias.push(Referencia { id: id(r.id), nome: r.name.clone(), definicao });
                false
            }
            Err(_) => true,
        });
        itens_pendentes.retain(|(i, f)| match item(&t, f) {
            Ok(it) => {
                t.entrou.insert(f.id);
                arvore.push((*i, it));
                false
            }
            Err(_) => true,
        });
        if t.entrou.len() == antes {
            break;
        }
    }
    let mut fora: Vec<Fora> = refs_pendentes.iter().map(|r| Fora { nome: r.name.clone(), motivo: t.definicao(&r.kind).err().unwrap_or_default() }).collect();
    fora.extend(itens_pendentes.iter().map(|(_, f)| Fora { nome: f.name.clone(), motivo: item(&t, f).err().unwrap_or_default() }));
    fora.extend(doc.params.iter().filter(|p| !p.tipo.numerico()).map(|p| Fora { nome: p.name.clone(), motivo: "parâmetro que não é número não existe na versão 1".into() }));
    arvore.sort_by_key(|(i, _)| *i);
    let ativos = (doc.rollback < doc.features.len()).then(|| arvore.iter().filter(|(i, _)| *i < doc.rollback).count());
    let corpos = doc.collectors.iter().map(|c| Corpo { id: id(c.id), nome: c.name.clone(), cor: Some(c.color) }).collect();
    let peca = Peca { id: doc_id(doc), nome: doc.name.clone(), parametros: doc.params.iter().filter_map(parametro).collect(), corpos, referencias, arvore: arvore.into_iter().map(|(_, it)| it).collect(), ativos };
    Exportado { peca, fora }
}

fn doc_id(doc: &Document) -> String {
    if doc.uid.esta_vazio() {
        "peca".into()
    } else {
        doc.uid.0.clone()
    }
}
