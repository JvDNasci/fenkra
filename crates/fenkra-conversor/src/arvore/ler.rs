// SPDX-License-Identifier: LGPL-2.1-or-later
//! Do arquivo de árvore ao documento, com histórico. Texto de fora é hostil até prova em contrário: o
//! esquema barra a forma errada e esta leitura barra id repetido ou solto, porque o esquema não vê ligações.

use super::*;
use crate::ErroConv;
use fenkra_core::formato::{aviso_futuro, preparar, Tipo};
use fenkra_core::sketch::{Constraint, Curve, Kind, SkCurve, SkPoint, SketchPos, SplineTan};
use fenkra_core::{Appearance, AxisRef, BoolOp, CornerMode, CurveRef, Document, EdgeMode, EdgeOp, EdgeSel, Extent, Extrude, Feature, FeatureKind, Finish, GussetPos, GussetProfile, GussetRepeat, GussetSide, Hole, HoleKind, Id, MarginMode, Mirror, OriginAxis, OriginPlane, Pattern, PatternKind, PlaneRef, PlateHoles, PlateSide, PointRef, PrimKind, Primitive, RefElement, RefKind, Revolve, SectionKind, SheetParams, Shell, Sketch, SketchData, SmBend, SmFlange, SmHem, SmWall, StructGusset, StructMember, StructPlate, Unit};
use std::collections::HashMap;

/// Documento remontado e o que a reancoragem achou.
#[derive(Clone, Debug, PartialEq)]
pub struct Remontada {
    pub doc: Document,
    pub reancoragem: Reancoragem,
}

/// Lê o texto, migra pela cadeia do B64 e valida pelo esquema. Versão mais nova que a do programa é
/// recusada: o `.fkp` pode abrir só para leitura porque guarda tudo, mas aqui um comando desconhecido
/// sumiria da peça sem aviso.
pub fn ler_texto(texto: &str) -> Result<Arquivo, ErroConv> {
    let p = preparar(Tipo::Arvore, texto).map_err(|e| ErroConv::Leitura(format!("JSON inválido: {e}")))?;
    if p.futuro {
        return Err(ErroConv::Leitura(aviso_futuro(Tipo::Arvore, p.versao_original)));
    }
    validar(&p.valor).map_err(|e| ErroConv::Leitura(format!("arquivo fora do esquema em {e}")))?;
    serde_json::from_value(p.valor).map_err(|e| ErroConv::Leitura(format!("arquivo fora do esquema: {e}")))
}

fn erro(peca: &Peca, motivo: String) -> ErroConv {
    ErroConv::Leitura(format!("peça '{}': {motivo}", peca.nome))
}

/// Ids de texto do arquivo e os ids do documento.
struct Tabela<'a> {
    peca: &'a Peca,
    /// corpos, referências e itens
    global: HashMap<&'a str, Id>,
    /// pontos e curvas de cada esboço; o mesmo id pode repetir em esboços diferentes
    esbocos: HashMap<&'a str, HashMap<&'a str, Id>>,
}

impl<'a> Tabela<'a> {
    fn novo(&mut self, d: &mut Document, id: &'a str) -> Result<Id, ErroConv> {
        let n = d.alloc();
        if self.global.insert(id, n).is_some() {
            return Err(erro(self.peca, format!("id '{id}' repetido")));
        }
        Ok(n)
    }

    fn achar(&self, id: &str) -> Result<Id, ErroConv> {
        self.global.get(id).copied().ok_or_else(|| erro(self.peca, format!("id '{id}' não existe")))
    }

    fn do_esboco(&self, esboco: &str, id: &str) -> Result<Id, ErroConv> {
        self.esbocos.get(esboco).and_then(|m| m.get(id)).copied().ok_or_else(|| erro(self.peca, format!("'{id}' não existe no esboço '{esboco}'")))
    }

    fn plano(&self, p: &Plano) -> Result<PlaneRef, ErroConv> {
        Ok(match p {
            Plano::Origem(PlanoOrigem::XY) => PlaneRef::Origin(OriginPlane::XY),
            Plano::Origem(PlanoOrigem::XZ) => PlaneRef::Origin(OriginPlane::XZ),
            Plano::Origem(PlanoOrigem::YZ) => PlaneRef::Origin(OriginPlane::YZ),
            Plano::Referencia { referencia } => PlaneRef::Datum(self.achar(referencia)?),
        })
    }

    fn eixo(&self, e: &Eixo) -> Result<AxisRef, ErroConv> {
        Ok(match e {
            Eixo::Origem(EixoOrigem::X) => AxisRef::Origin(OriginAxis::X),
            Eixo::Origem(EixoOrigem::Y) => AxisRef::Origin(OriginAxis::Y),
            Eixo::Origem(EixoOrigem::Z) => AxisRef::Origin(OriginAxis::Z),
            Eixo::Curva { esboco, curva } => AxisRef::SketchLine { sketch: self.achar(esboco)?, entity: self.do_esboco(esboco, curva)? },
            Eixo::Referencia { referencia } => AxisRef::Ref(self.achar(referencia)?),
            Eixo::Fixo { origem, direcao } => AxisRef::Fixed { origin: *origem, dir: *direcao },
            Eixo::Face { face } => AxisRef::Face { point: *face, key: None },
            Eixo::Aresta { aresta } => AxisRef::Edge { mid: *aresta, key: None },
            Eixo::Planos { planos } => AxisRef::Planes { a: self.plano(&planos[0])?, b: self.plano(&planos[1])? },
        })
    }

    fn ponto(&self, p: &Ponto) -> Result<PointRef, ErroConv> {
        Ok(match p {
            Ponto::Coordenadas(c) => PointRef::Fixed(*c),
            Ponto::Referencia { referencia } => PointRef::Ref(self.achar(referencia)?),
            Ponto::DeEsboco { esboco, ponto } => PointRef::SketchPoint { sketch: self.achar(esboco)?, point: self.do_esboco(esboco, ponto)? },
        })
    }

    fn termino(&self, t: &Termino) -> Result<Extent, ErroConv> {
        Ok(match t {
            Termino::Cego { distancia } => Extent::Blind(distancia.clone()),
            Termino::Simetrico { distancia } => Extent::Symmetric(distancia.clone()),
            Termino::DoisLados { a, b } => Extent::TwoSides(a.clone(), b.clone()),
            Termino::Atravessar => Extent::ThroughAll,
            Termino::AtePlano { plano } => Extent::UpTo(self.plano(plano)?),
            Termino::AteProxima => Extent::UpToNext,
            Termino::AteUltima => Extent::UpToLast,
        })
    }

    fn definicao(&self, d: &Definicao) -> Result<RefKind, ErroConv> {
        Ok(match d {
            Definicao::PlanoDeslocado { base, deslocamento } => RefKind::OffsetPlane { base: self.plano(base)?, offset: deslocamento.clone() },
            Definicao::PlanoFixo { origem, x, normal } => RefKind::FacePlane { origin: *origem, x: *x, normal: *normal, owner: None, key: None },
            Definicao::Ponto { x, y, z, sistema } => RefKind::PointCoords { x: x.clone(), y: y.clone(), z: z.clone(), frame: sistema.as_ref().map(|s| self.plano(s)).transpose()? },
            Definicao::Reta { ponto, direcao, inicio, fim } => RefKind::LineDir { point: self.ponto(ponto)?, dir: self.eixo(direcao)?, start: inicio.clone(), end: fim.clone() },
            Definicao::RetaPontos { a, b, inicio, fim } => RefKind::LinePoints { a: self.ponto(a)?, b: self.ponto(b)?, start: inicio.clone(), end: fim.clone() },
        })
    }

    fn curva(&self, c: &Curva) -> Result<CurveRef, ErroConv> {
        Ok(match c {
            Curva::Origem(EixoOrigem::X) => CurveRef::Axis(OriginAxis::X),
            Curva::Origem(EixoOrigem::Y) => CurveRef::Axis(OriginAxis::Y),
            Curva::Origem(EixoOrigem::Z) => CurveRef::Axis(OriginAxis::Z),
            Curva::Referencia { referencia } => CurveRef::Ref(self.achar(referencia)?),
            Curva::DeEsboco { esboco, curva } => CurveRef::SketchCurve { sketch: self.achar(esboco)?, curve: self.do_esboco(esboco, curva)? },
            Curva::Pontos { pontos } => CurveRef::Edge(pontos.clone()),
        })
    }

    fn fontes(&self, v: &[String]) -> Result<Vec<Id>, ErroConv> {
        v.iter().map(|i| self.achar(i)).collect()
    }

    fn arestas(&self, a: &Arestas) -> Result<EdgeSel, ErroConv> {
        Ok(match a {
            Arestas::Todas(_) => EdgeSel::AllOfBody,
            Arestas::Geradas { geradas_por } => EdgeSel::CreatedBy(self.achar(geradas_por)?),
            Arestas::Pontos(p) => EdgeSel::Edges(p.clone()),
        })
    }

    fn restricao(&self, esboco: &str, r: &Restricao, id: Id) -> Result<Constraint, ErroConv> {
        use TipoRestricao as T;
        let e: Vec<Id> = r.entidades.iter().map(|x| self.do_esboco(esboco, x)).collect::<Result<_, _>>()?;
        let valor = || r.valor.clone().ok_or_else(|| erro(self.peca, format!("cota {:?} do esboço '{esboco}' sem valor", r.tipo)));
        let quantos = match r.tipo {
            T::Horizontal | T::Vertical | T::Fixa | T::Comprimento | T::Raio | T::Diametro | T::AnguloReta => 1,
            T::Simetrica => 3,
            _ => 2,
        };
        if e.len() != quantos {
            return Err(erro(self.peca, format!("restrição {:?} do esboço '{esboco}' pede {quantos} entidades e tem {}", r.tipo, e.len())));
        }
        let kind = match r.tipo {
            T::Coincidente => Kind::Coincident(e[0], e[1]),
            T::Horizontal => Kind::Horizontal(e[0]),
            T::Vertical => Kind::Vertical(e[0]),
            T::Paralela => Kind::Parallel(e[0], e[1]),
            T::Perpendicular => Kind::Perpendicular(e[0], e[1]),
            T::Igual => Kind::Equal(e[0], e[1]),
            T::Tangente => Kind::Tangent(e[0], e[1]),
            T::Concentrica => Kind::Concentric(e[0], e[1]),
            T::PontoMedio => Kind::Midpoint(e[0], e[1]),
            T::SobreCurva => Kind::PointOnCurve(e[0], e[1]),
            T::Fixa => Kind::Fixed(e[0]),
            T::Simetrica => Kind::Symmetric(e[0], e[1], e[2]),
            T::HorizontalPontos => Kind::HorizontalPts(e[0], e[1]),
            T::VerticalPontos => Kind::VerticalPts(e[0], e[1]),
            T::Distancia => Kind::Distance(e[0], e[1], valor()?),
            T::DistanciaX => Kind::DistanceX(e[0], e[1], valor()?),
            T::DistanciaY => Kind::DistanceY(e[0], e[1], valor()?),
            T::Comprimento => Kind::Length(e[0], valor()?),
            T::Raio => Kind::Radius(e[0], valor()?),
            T::Diametro => Kind::Diameter(e[0], valor()?),
            T::Angulo => Kind::Angle(e[0], e[1], valor()?),
            T::AnguloSuplementar => Kind::AngleSupp(e[0], e[1], valor()?),
            T::DistanciaPontoReta => Kind::PointLineDistance(e[0], e[1], valor()?),
            T::AnguloReta => Kind::LineAngle(e[0], valor()?),
        };
        Ok(Constraint { id, kind, label: None, reference: r.referencia, flipped: r.invertida })
    }

    fn geometria(&self, esboco: &str, g: &Geometria) -> Result<Curve, ErroConv> {
        let p = |x: &str| self.do_esboco(esboco, x);
        Ok(match g {
            Geometria::Reta { a, b } => Curve::Line { a: p(a)?, b: p(b)? },
            Geometria::Circulo { centro, raio } => Curve::Circle { c: p(centro)?, r: *raio },
            Geometria::Arco { centro, inicio, fim } => Curve::Arc { c: p(centro)?, s: p(inicio)?, e: p(fim)? },
            Geometria::Ponto { ponto } => Curve::Point { p: p(ponto)? },
            Geometria::Spline { pontos } => Curve::Spline { pts: pontos.iter().map(|x| p(x)).collect::<Result<_, _>>()? },
            Geometria::Elipse { centro, inicio, fim, maior, menor, t0, t1 } => Curve::Ellipse { c: p(centro)?, s: p(inicio)?, e: p(fim)?, major: *maior, minor: *menor, t0: *t0, t1: *t1 },
        })
    }

    fn esboco(&self, d: &mut Document, id: &str, e: &Esboco) -> Result<Sketch, ErroConv> {
        let mut data = SketchData::default();
        for q in &e.pontos {
            data.points.push(SkPoint { id: self.do_esboco(id, &q.id)?, x: q.x, y: q.y, fixed: q.fixo });
        }
        for c in &e.curvas {
            data.curves.push(SkCurve { id: self.do_esboco(id, &c.id)?, construction: c.construcao, geom: self.geometria(id, &c.geometria)? });
        }
        for r in &e.restricoes {
            let n = d.alloc();
            data.constraints.push(self.restricao(id, r, n)?);
        }
        for t in &e.tangentes {
            data.spline_tan.push(SplineTan { curve: self.do_esboco(id, &t.curva)?, point: self.do_esboco(id, &t.ponto)?, dir: t.direcao });
        }
        if let Some(p) = &e.posicao {
            data.pos = SketchPos { origem: p.origem, trocar: p.trocar, inverter_h: p.inverter_h, inverter_v: p.inverter_v };
        }
        Ok(Sketch { plane: self.plano(&e.plano)?, data })
    }

    fn comando(&self, d: &mut Document, id: &str, c: &Comando) -> Result<FeatureKind, ErroConv> {
        Ok(match c {
            Comando::Esboco(e) => FeatureKind::Sketch(self.esboco(d, id, e)?),
            Comando::Extrusao { esboco, operacao: op, termino, inverter, saida, fina, direcao } => FeatureKind::Extrude(Extrude {
                sketch: self.achar(esboco)?,
                op: operacao(*op),
                extent: self.termino(termino)?,
                reverse: *inverter,
                taper: saida.clone(),
                thin: fina.as_ref().map(|f| f.espessura.clone()),
                thin_side: fina.as_ref().map_or(0, |f| f.lado),
                dir: direcao.as_ref().map(|x| self.eixo(x)).transpose()?,
                ends: Default::default(),
            }),
            Comando::Revolucao { esboco, eixo, angulo, angulo2, simetrico, operacao: op, fina } => FeatureKind::Revolve(Revolve {
                sketch: self.achar(esboco)?,
                axis: self.eixo(eixo)?,
                angle: angulo.clone(),
                symmetric: *simetrico,
                op: operacao(*op),
                thin: fina.as_ref().map(|f| f.espessura.clone()),
                thin_side: fina.as_ref().map_or(0, |f| f.lado),
                angle2: angulo2.clone(),
            }),
            Comando::Furo { esboco, forma, diametro, termino, inverter, rosca, ponta } => FeatureKind::Hole(Hole {
                sketch: self.achar(esboco)?,
                kind: match forma {
                    FormaFuro::Simples => HoleKind::Simples,
                    FormaFuro::Rebaixado { diametro, profundidade } => HoleKind::Rebaixado { diameter: diametro.clone(), depth: profundidade.clone() },
                    FormaFuro::Escareado { diametro, angulo } => HoleKind::Escareado { diameter: diametro.clone(), angle: angulo.clone() },
                    FormaFuro::Contrafurado { diametro, profundidade, angulo } => HoleKind::Contrafurado { diameter: diametro.clone(), depth: profundidade.clone(), angle: angulo.clone() },
                },
                diameter: diametro.clone(),
                depth: self.termino(termino)?,
                thread: rosca.clone(),
                thread_modeled: false,
                standard: None,
                points: vec![],
                dir: None,
                reverse: *inverter,
                tip: ponta.clone(),
                taper: "0".into(),
                left_hand: false,
                extra: Default::default(),
            }),
            Comando::Filete { raio, arestas } => FeatureKind::Fillet(EdgeOp { extra: Default::default(), value: raio.clone(), edges: self.arestas(arestas)?, mode: EdgeMode::Constant, prop: Default::default(), keys: vec![] }),
            Comando::Chanfro { distancia, arestas } => FeatureKind::Chamfer(EdgeOp { extra: Default::default(), value: distancia.clone(), edges: self.arestas(arestas)?, mode: EdgeMode::Constant, prop: Default::default(), keys: vec![] }),
            Comando::Casca { faces, espessura, para_fora, fechada } => FeatureKind::Shell(Shell { espessuras: vec![], faces: faces.clone(), thickness: espessura.clone(), outward: *para_fora, face_keys: vec![], closed: *fechada, regras: vec![] }),
            Comando::Primitiva { forma, plano, a, b, c, d: dd, operacao: op } => {
                FeatureKind::Primitive(Primitive { kind: primitiva(*forma), plane: self.plano(plano)?, a: a.clone(), b: b.clone(), c: c.clone(), d: dd.clone(), op: operacao(*op) })
            }
            Comando::Padrao(p) => {
                let kind = match &p.forma {
                    FormaPadrao::Linear { direcao, quantidade, espacamento, direcao2, quantidade2, espacamento2, total } => PatternKind::Linear {
                        dir: self.eixo(direcao)?,
                        count: quantidade.clone(),
                        spacing: espacamento.clone(),
                        dir2: direcao2.as_ref().map(|x| self.eixo(x)).transpose()?,
                        count2: quantidade2.clone(),
                        spacing2: espacamento2.clone(),
                        total: *total,
                    },
                    FormaPadrao::Circular { eixo, quantidade, angulo } => PatternKind::Circular { axis: self.eixo(eixo)?, count: quantidade.clone(), angle: angulo.clone() },
                };
                FeatureKind::Pattern(Pattern { kind, sources: self.fontes(&p.fontes)?, skip: p.pular.iter().map(|q| (q[0], q[1])).collect() })
            }
            Comando::Espelho { plano, fontes } => FeatureKind::Mirror(Mirror { plane: self.plano(plano)?, sources: self.fontes(fontes)? }),
            Comando::ParedeChapa { esboco, inverter, deslocamento, meio } => {
                FeatureKind::SmWall(SmWall { sketch: self.achar(esboco)?, reverse: *inverter, offset: sem_zero(deslocamento), mid: *meio, surface: None })
            }
            Comando::Aba { aresta, comprimento, angulo, raio, recuo1, recuo2, inverter, alivio, alivio_forma, lado, meia_esquadria } => FeatureKind::SmFlange(SmFlange {
                edge: *aresta,
                length: comprimento.clone(),
                angle: angulo.clone(),
                radius: raio.clone(),
                off1: recuo1.clone(),
                off2: recuo2.clone(),
                flip: *inverter,
                relief: alivio.clone(),
                relief_kind: *alivio_forma as u8,
                side: *lado as u8,
                miter: *meia_esquadria,
                min_gap: true,
                edge_key: None,
            }),
            Comando::Bainha { aresta, comprimento, folga, inverter, forma } => FeatureKind::SmHem(SmHem { edge: *aresta, length: comprimento.clone(), gap: folga.clone(), flip: *inverter, kind: *forma as u8, edge_key: None }),
            Comando::Dobra { a, b, angulo, raio, inverter, para_baixo, esboco, posicao } => FeatureKind::SmBend(SmBend {
                a: *a,
                b: *b,
                angle: angulo.clone(),
                radius: raio.clone(),
                flip: *inverter,
                down: *para_baixo,
                sketch: esboco.as_deref().map(|e| self.achar(e)).transpose()?,
                position: *posicao as u8,
            }),
            Comando::Membro(m) => FeatureKind::StructMember(StructMember {
                paths: m.caminhos.iter().map(|c| self.curva(c)).collect::<Result<_, _>>()?,
                family: m.familia.clone(),
                designation: m.designacao.clone(),
                kind: secao(m.secao),
                dims: m.cotas.clone(),
                anchor: m.ancoragem,
                angle: m.angulo.clone(),
                off_x: m.deslocamento_x.clone(),
                off_y: m.deslocamento_y.clone(),
                ext1: m.prolongamento1.clone(),
                ext2: m.prolongamento2.clone(),
                corner: match m.canto {
                    Canto::Nenhum => CornerMode::Nenhum,
                    Canto::MeiaEsquadria => CornerMode::MeiaEsquadria,
                    Canto::Aparar => CornerMode::Aparar,
                    Canto::Encaixe => CornerMode::Encaixe,
                },
                gap: m.folga.clone(),
                joints: vec![],
                material: m.material.clone(),
                mirror: m.espelhar,
            }),
            Comando::PlacaExtremidade { em, espessura, sobra, lado, recuo, sobra_em, furos } => FeatureKind::StructPlate(StructPlate {
                at: *em,
                thickness: espessura.clone(),
                margin: sobra.clone(),
                side: match lado {
                    LadoPlaca::Externo => PlateSide::Externo,
                    LadoPlaca::Interno => PlateSide::Interno,
                },
                recess: sem_zero(recuo),
                margin_mode: match sobra_em {
                    SobraEm::Mm => MarginMode::Valor,
                    SobraEm::Parede => MarginMode::RazaoParede,
                },
                holes: furos.as_ref().map(|f| PlateHoles { diameter: f.diametro.clone(), edge: f.borda.clone() }),
            }),
            Comando::Reforco { em, aba, espessura, chanfro, aba2, trocar, lado, posicao, afastamento, repeticao, quantidade, espacamento } => FeatureKind::StructGusset(StructGusset {
                at: *em,
                leg: aba.clone(),
                thickness: espessura.clone(),
                chamfer: chanfro.clone(),
                leg2: aba2.clone(),
                swap: *trocar,
                profile: GussetProfile::Chanfrado,
                poly: vec![],
                side: match lado {
                    LadoReforco::DoisLados => GussetSide::DoisLados,
                    LadoReforco::Interno => GussetSide::Interno,
                    LadoReforco::Externo => GussetSide::Externo,
                },
                position: match posicao {
                    PosicaoReforco::Inicio => GussetPos::Inicio,
                    PosicaoReforco::Meio => GussetPos::Meio,
                    PosicaoReforco::Fim => GussetPos::Fim,
                },
                offset: sem_zero(afastamento),
                repeat: match repeticao {
                    RepeticaoReforco::Nenhuma => GussetRepeat::Nenhuma,
                    RepeticaoReforco::Quantidade => GussetRepeat::Quantidade,
                    RepeticaoReforco::Espacamento => GussetRepeat::Espacamento,
                },
                count: quantidade.clone(),
                spacing: espacamento.clone(),
            }),
        })
    }
}

fn operacao(o: Operacao) -> BoolOp {
    match o {
        Operacao::Unir => BoolOp::Add,
        Operacao::Cortar => BoolOp::Cut,
        Operacao::Intersectar => BoolOp::Intersect,
    }
}

/// "0" de quem grava é o vazio do documento: os campos opcionais de afastamento nascem vazios.
fn sem_zero(s: &str) -> String {
    if s.trim() == "0" {
        String::new()
    } else {
        s.to_string()
    }
}

pub(super) fn secao(s: Secao) -> SectionKind {
    match s {
        Secao::TuboQuadrado => SectionKind::TuboQuadrado,
        Secao::TuboRetangular => SectionKind::TuboRetangular,
        Secao::TuboRedondo => SectionKind::TuboRedondo,
        Secao::PerfilI => SectionKind::PerfilI,
        Secao::PerfilU => SectionKind::PerfilU,
        Secao::Cantoneira => SectionKind::Cantoneira,
        Secao::BarraChata => SectionKind::BarraChata,
        Secao::BarraRedonda => SectionKind::BarraRedonda,
        Secao::BarraQuadrada => SectionKind::BarraQuadrada,
        Secao::PerfilT => SectionKind::PerfilT,
    }
}

fn chapa(c: &Chapa) -> SheetParams {
    SheetParams {
        thickness: c.espessura.clone(),
        radius: c.raio.clone(),
        k: c.fator_k.clone(),
        flat: c.planificada,
        material: c.material.clone(),
        method: c.metodo as u8,
        allowance: c.compensacao.clone(),
        relief: c.alivio.clone(),
        relief_kind: c.alivio_forma as u8,
        corner: c.canto.clone(),
        ..SheetParams::default()
    }
}

fn primitiva(f: Forma) -> PrimKind {
    match f {
        Forma::Caixa => PrimKind::Caixa,
        Forma::Cilindro => PrimKind::Cilindro,
        Forma::Esfera => PrimKind::Esfera,
        Forma::Cone => PrimKind::Cone,
        Forma::Toro => PrimKind::Toro,
        Forma::Cunha => PrimKind::Cunha,
        Forma::Prisma => PrimKind::Prisma,
    }
}

/// Nome que ainda não existe no documento. Outro programa pode repetir nomes; aqui eles são únicos.
fn nome_livre(d: &Document, nome: &str) -> String {
    if !nome.is_empty() && !d.name_taken(nome) {
        return nome.to_string();
    }
    let base = if nome.is_empty() { "Item" } else { nome };
    (2..).map(|i| format!("{base} {i}")).find(|n| !d.name_taken(n)).unwrap_or_default()
}

fn corpos<'a>(d: &mut Document, t: &mut Tabela<'a>, peca: &'a Peca) -> Result<(), ErroConv> {
    for (i, c) in peca.corpos.iter().enumerate() {
        let n = if i == 0 { d.active_collector } else { d.add_collector(&c.nome) };
        if t.global.insert(&c.id, n).is_some() {
            return Err(erro(peca, format!("id '{}' repetido", c.id)));
        }
        if let Some(k) = d.collectors.iter_mut().find(|k| k.id == n) {
            k.name = c.nome.clone();
            if let Some(cor) = c.cor {
                k.color = cor;
            }
        }
    }
    Ok(())
}

fn parametros(d: &mut Document, peca: &Peca) -> Result<(), ErroConv> {
    for p in &peca.parametros {
        let unidade = match p.grandeza {
            Grandeza::Comprimento => Unit::Comprimento,
            Grandeza::Angulo => Unit::Angulo,
            Grandeza::Numero => Unit::Numero,
        };
        let id = d.add_param(&p.nome, &p.expressao, unidade).map_err(|e| erro(peca, format!("parâmetro '{}': {e}", p.nome)))?;
        if let Some(q) = d.params.iter_mut().find(|q| q.id == id) {
            q.comment = p.comentario.clone();
        }
    }
    Ok(())
}

/// Remonta a peça, sem reancorar: serve a quem só quer a árvore.
fn montar(peca: &Peca) -> Result<Document, ErroConv> {
    let mut d = Document::new(&peca.nome);
    d.sheet = peca.chapa.as_ref().map(chapa);
    let mut t = Tabela { peca, global: HashMap::new(), esbocos: HashMap::new() };
    corpos(&mut d, &mut t, peca)?;
    parametros(&mut d, peca)?;
    // ids antes do conteúdo: referência pode apontar para esboço e esboço para referência
    let ids_refs: Vec<Id> = peca.referencias.iter().map(|r| t.novo(&mut d, &r.id)).collect::<Result<_, _>>()?;
    let ids_itens: Vec<Id> = peca.arvore.iter().map(|i| t.novo(&mut d, &i.id)).collect::<Result<_, _>>()?;
    for i in &peca.arvore {
        let Comando::Esboco(e) = &i.comando else { continue };
        let mut m = HashMap::new();
        for id in e.pontos.iter().map(|p| &p.id).chain(e.curvas.iter().map(|c| &c.id)) {
            if m.insert(id.as_str(), d.alloc()).is_some() {
                return Err(erro(peca, format!("id '{id}' repetido no esboço '{}'", i.id)));
            }
        }
        t.esbocos.insert(&i.id, m);
    }
    for (r, id) in peca.referencias.iter().zip(ids_refs) {
        let kind = t.definicao(&r.definicao)?;
        let name = nome_livre(&d, &r.nome);
        d.references.push(RefElement { id, name, kind, visible: true, folder: None, published: false, appearance: None });
    }
    let primeiro = d.active_collector;
    for (i, id) in peca.arvore.iter().zip(ids_itens) {
        let kind = t.comando(&mut d, &i.id, &i.comando)?;
        let target = i.corpo.as_deref().map(|c| t.achar(c)).transpose()?.unwrap_or(primeiro);
        if !d.collectors.iter().any(|k| k.id == target) {
            return Err(erro(peca, format!("'{}' não é um corpo", i.corpo.as_deref().unwrap_or_default())));
        }
        let appearance = i.cor.map(|color| Appearance { color, finish: Finish::default(), texture: None });
        let name = nome_livre(&d, &i.nome);
        d.features.push(Feature { id, name, kind, suppressed: i.suprimido, suppress_if: i.suprimir_se.clone(), folder: None, target, hidden: i.oculto, appearance, flat_space: false });
    }
    d.rollback = peca.ativos.unwrap_or(d.features.len()).min(d.features.len());
    Ok(d)
}

/// Remonta a peça e reancora as referências pela geometria.
pub fn ler_peca(peca: &Peca) -> Result<Remontada, ErroConv> {
    let mut doc = montar(peca)?;
    let reancoragem = reancorar(&mut doc);
    Ok(Remontada { doc, reancoragem })
}
