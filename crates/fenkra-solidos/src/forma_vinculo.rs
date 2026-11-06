// SPDX-License-Identifier: LGPL-2.1-or-later
//! Resolve os vínculos de uma forma livre (B303) a cada recálculo.
//!
//! A gaiola do documento fica como está: daqui sai uma cópia com os nós onde as referências mandam, e é
//! essa cópia que vira superfície e sólido. Referência que sumiu é erro do comando, com a forma parada
//! onde estava: nenhum vínculo cai para outro lugar calado. Por isso a face é resolvida sempre pela
//! forma viva do modelo, e não pelo ponto guardado no clique.

use crate::{ev, geo3d, Ctx};
use fenkra_core::subd::{Gaiola, TOL_PLANO, V3};
use fenkra_core::subd_vinculo::*;
use fenkra_core::*;

/// Gaiola com os vínculos aplicados e a geometria deles, para o 3D desenhar e o arraste deslizar.
#[derive(Clone, Debug, PartialEq)]
pub struct FormaResolvida {
    pub id: Id,
    /// a gaiola do documento que entrou: diz se esta conta ainda vale para o que está na tela
    pub entrada: Gaiola,
    pub gaiola: Gaiola,
    pub geos: Vec<(u32, GeoNo)>,
    /// plano e distância de cada cota, na ordem do documento
    pub cotas: Vec<(u32, QuadroV, f64)>,
    pub bordas: Vec<RelatorioBorda>,
}

fn perdido(no: u32, e: String) -> String {
    if e.starts_with("referência perdida") {
        format!("vínculo do nó {no}: {e}")
    } else {
        format!("referência perdida no vínculo do nó {no}: {e}")
    }
}

/// Põe o nó em `q`. A simetria manda mais que o vínculo: o nó da costura não sai do plano, e nenhum
/// nó passa para o outro lado, porque ali a malha se cruzaria com o próprio reflexo.
fn por(g: &mut Gaiola, id: u32, mut q: V3, avisos: &mut Vec<String>) -> Result<(), String> {
    if let Some(s) = g.simetria {
        if g.no_preso_ao_plano(id).is_some() {
            if (q[s.eixo as usize] - s.pos).abs() > 1e-6 {
                avisos.push(format!("o nó {id} está na costura da simetria e só acompanha a referência dentro do plano"));
            }
            q[s.eixo as usize] = s.pos;
        } else if s.distancia(q) <= TOL_PLANO {
            return Err(format!("o vínculo leva o nó {id} para o outro lado do plano de simetria"));
        }
    }
    g.no_mut(id).ok_or_else(|| format!("referência perdida: o nó {id} não existe mais na gaiola"))?.p = q;
    Ok(())
}

impl Ctx<'_> {
    /// Ponto da face mais perto de `p` e a normal, sem aceitar face que sumiu.
    fn face_viva(&self, face: &SurfRef, p: V3) -> Result<(V3, V3), String> {
        match face {
            SurfRef::Plane(_) => self.surf_at(face, p, 0),
            _ => self.surface_shape(face, 0)?.surface_near(p).map_err(|e| e.0),
        }
    }

    /// Normal da face em `p` e curvatura normal na direção que atravessa a aresta. A curvatura sai da
    /// flecha de um ponto a `h` do lado de dentro: `2·δ / h²`.
    fn apoio_da_borda(&self, face: &SurfRef, p: V3, t: V3, h: f64) -> Result<(V3, f64), String> {
        use geo3d::*;
        let (o, n) = self.face_viva(face, p)?;
        let Some(atravessa) = unit(cross(t, n)) else { return Ok((n, 0.)) };
        for lado in [1., -1.] {
            let a = add(o, mul(atravessa, lado * h));
            let (q, _) = self.face_viva(face, a)?;
            // o ponto caiu dentro da face se a projeção não voltou para a beira
            if dist(q, a) < 0.5 * h {
                return Ok((n, 2. * dot(sub(q, o), n).abs() / (h * h)));
            }
        }
        Ok((n, 0.))
    }

    /// Gaiola resolvida e o aviso do que ficou aquém do pedido.
    pub(crate) fn resolver_forma(&self, id: Id, fm: &Forma) -> Result<(FormaResolvida, Option<String>), String> {
        let mut g = fm.gaiola.clone();
        let (mut avisos, mut geos, mut cotas, mut bordas) = (vec![], vec![], vec![], vec![]);
        for v in &fm.vinculos.nos {
            let dica = g.pos(v.no).ok_or_else(|| format!("referência perdida: o nó {} não existe mais na gaiola", v.no))?;
            let e = |e: String| perdido(v.no, e);
            let (q, geo) = match &v.alvo {
                AlvoNo::Ponto(p) => {
                    let q = self.point(p, 0).map_err(e)?;
                    (q, GeoNo::Ponto(q))
                }
                AlvoNo::Curva { curva, t } => {
                    let c = self.curve(curva, 0).map_err(e)?;
                    let mut pts = c.pts.clone();
                    if c.closed {
                        pts.push(pts[0]);
                    }
                    (ponto_na_curva(&pts, t.unwrap_or_else(|| fracao_na_curva(&pts, dica))), GeoNo::Curva(pts))
                }
                AlvoNo::Plano { plano, uv } => {
                    let f = self.plane(plano, 0).map_err(e)?;
                    let quadro = QuadroV { o: f.origin, x: f.x, y: f.y(), n: f.normal };
                    let [u, w] = uv.unwrap_or_else(|| {
                        let (u, w) = quadro.coordenadas(dica);
                        [u, w]
                    });
                    (quadro.ponto(u, w), GeoNo::Plano(quadro))
                }
                AlvoNo::Face { face } => {
                    let (q, _) = self.face_viva(face, dica).map_err(e)?;
                    (q, GeoNo::Face(q))
                }
            };
            por(&mut g, v.no, q, &mut avisos)?;
            geos.push((v.no, geo));
        }
        for c in &fm.vinculos.cotas {
            let p = g.pos(c.no).ok_or_else(|| format!("referência perdida: o nó {} não existe mais na gaiola", c.no))?;
            let f = self.plane(&c.plano, 0).map_err(|e| perdido(c.no, e))?;
            let quadro = QuadroV { o: f.origin, x: f.x, y: f.y(), n: f.normal };
            let d = ev(&c.expr, &self.vars, "cota do nó")?;
            por(&mut g, c.no, geo3d::add(p, geo3d::mul(quadro.n, d - quadro.distancia(p))), &mut avisos)?;
            cotas.push((c.no, quadro, d));
        }
        for b in &fm.vinculos.bordas {
            let c = self.curve(&b.aresta, 0).map_err(|e| format!("referência perdida na borda casada: {}", e.trim_start_matches("referência perdida: ")))?;
            let mut pts = c.pts.clone();
            if c.closed {
                pts.push(pts[0]);
            }
            let tamanho: f64 = pts.windows(2).map(|w| geo3d::dist(w[0], w[1])).sum();
            let h = (tamanho * 0.01).max(1e-3);
            let apoio = |p: V3, t: V3| -> Result<(V3, f64), String> {
                let face = b.face.as_ref().ok_or("G1 e G2 precisam da face vizinha: escolha a face")?;
                self.apoio_da_borda(face, p, t, h).map_err(|e| format!("referência perdida na face da borda casada: {}", e.trim_start_matches("referência perdida: ")))
            };
            let rel = casar_borda(&mut g, &b.nos, &pts, b.continuidade, Some(&apoio))?;
            if let Some(m) = &rel.motivo {
                avisos.push(m.clone());
            }
            bordas.push(rel);
        }
        g.refazer_tangencias();
        let aviso = (!avisos.is_empty()).then(|| avisos.join("; "));
        Ok((FormaResolvida { id, entrada: fm.gaiola.clone(), gaiola: g, geos, cotas, bordas }, aviso))
    }
}

#[cfg(test)]
mod testes {
    use crate::{recompute, Status};
    use fenkra_core::subd::Gaiola;
    use fenkra_core::subd_vinculo::*;
    use fenkra_core::*;

    fn s(x: &str) -> String {
        x.to_string()
    }

    fn forma(g: Gaiola, v: Vinculos, conversao: bool) -> FeatureKind {
        let c = conversao.then(|| ConversaoForma { continuidade: 0, tol: s("0.01"), pausada: false });
        FeatureKind::Forma(Forma { gaiola: g, niveis: 3, congelada: false, conversao: c, vinculos: v })
    }

    fn no_em(g: &Gaiola, p: [f64; 3]) -> u32 {
        g.nos.iter().find(|n| (0..3).all(|k| (n.p[k] - p[k]).abs() < 1e-9)).expect("nó").id
    }

    /// Plano paralelo a XY na altura `expr`.
    fn plano_em(d: &mut Document, expr: &str) -> PlaneRef {
        PlaneRef::Datum(d.add_geometry(RefKind::OffsetPlane { base: PlaneRef::Origin(OriginPlane::XY), offset: s(expr) }))
    }

    /// Bloco de 40 por 40 com a base em z = 0, por extrusão: é ela que dá papel às faces e arestas.
    fn caixa_com_altura(d: &mut Document, h: &str) -> Id {
        let mut data = SketchData::default();
        let mut n = 5_000_000u64;
        let mut al = || {
            n += 1;
            Id(n)
        };
        data.add_rect(&mut al, [-20., -20.], [20., 20.], false);
        let sketch = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
        d.add_feature(FeatureKind::Extrude(Extrude { sketch, op: BoolOp::Add, extent: Extent::Blind(s(h)), reverse: false, taper: s("0"), thin: None, dir: None, thin_side: 0, ends: Default::default() }))
    }

    /// Chave da face cujo primeiro triângulo está todo acima de `z`.
    fn chave_de_cima(b: &crate::BodyResult, z: f32) -> Option<FaceKey> {
        (0..b.mesh.face_ranges.len()).find(|f| {
            let i = b.mesh.face_ranges[*f] as usize;
            (i..i + 3).all(|k| b.mesh.positions[b.mesh.indices[k] as usize * 3 + 2] > z)
        }).and_then(|f| b.face_key(f))
    }

    fn resolvida(r: &crate::RecomputeResult, id: Id) -> &super::FormaResolvida {
        r.formas_resolvidas.iter().find(|x| x.id == id).expect("forma resolvida")
    }

    #[test]
    fn no_segue_ponto_plano_e_curva_de_referencia() {
        let mut d = Document::new("v");
        d.add_param("H", "30", Unit::Comprimento).unwrap();
        let g = Gaiola::plano([0., 0., 0.], [40., 40.], [2, 2]);
        let plano = plano_em(&mut d, "H");
        let ponto = d.add_geometry(RefKind::PointCoords { x: s("20"), y: s("20"), z: s("H+5"), frame: None });
        let linha = d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed([-20., -20., 0.]), b: PointRef::Fixed([20., -20., 0.]), start: s("0"), end: s("H") });
        let (a, b, c) = (no_em(&g, [0., 0., 0.]), no_em(&g, [20., 20., 0.]), no_em(&g, [0., -20., 0.]));
        let v = Vinculos {
            nos: vec![
                VinculoNo { no: a, alvo: AlvoNo::Plano { plano, uv: None } },
                VinculoNo { no: b, alvo: AlvoNo::Ponto(PointRef::Ref(ponto)) },
                VinculoNo { no: c, alvo: AlvoNo::Curva { curva: CurveRef::Ref(linha), t: Some(0.5) } },
            ],
            ..Default::default()
        };
        let id = d.add_feature(forma(g.clone(), v, false));
        let r = recompute(&d);
        assert!(matches!(r.status.get(&id), Some(Status::Ok)), "{:?}", r.status.get(&id));
        let x = resolvida(&r, id);
        assert_eq!(x.gaiola.pos(a).unwrap(), [0., 0., 30.]);
        assert_eq!(x.gaiola.pos(b).unwrap(), [20., 20., 35.]);
        assert!((x.gaiola.pos(c).unwrap()[0] - 15.).abs() < 1e-9, "linha de -20 a 50, no meio: {:?}", x.gaiola.pos(c));
        assert!(r.formas[0].malha.caixa().unwrap().1[2] > 10., "a superfície acompanha");
        // o parâmetro leva todos juntos; a gaiola do arquivo não muda
        d.params.iter_mut().find(|p| p.name == "H").unwrap().expr = s("50");
        let r = recompute(&d);
        let x = resolvida(&r, id);
        assert_eq!(x.gaiola.pos(a).unwrap(), [0., 0., 50.]);
        assert_eq!(x.gaiola.pos(b).unwrap(), [20., 20., 55.]);
        assert!((x.gaiola.pos(c).unwrap()[0] - 25.).abs() < 1e-9);
        assert_eq!(x.entrada, g);
    }

    /// Aceite do B303: mudar um parâmetro da peça move os nós vinculados e o sólido convertido.
    #[test]
    fn parametro_da_peca_move_os_nos_e_o_solido() {
        let mut d = Document::new("v");
        d.add_param("H", "40", Unit::Comprimento).unwrap();
        let g = Gaiola::caixa([0., 0., 50.], [100.; 3], [2, 2, 2]);
        let topo = no_em(&g, [0., 0., 100.]);
        let v = Vinculos { cotas: vec![CotaNo { no: topo, plano: PlaneRef::Origin(OriginPlane::XY), expr: s("H*3") }], ..Default::default() };
        let id = d.add_feature(forma(g, v, true));
        let r = recompute(&d);
        assert!(matches!(r.status.get(&id), Some(Status::Ok | Status::Warning(_))), "{:?}", r.status.get(&id));
        let (v0, z0) = (r.bodies[0].mass.volume, r.bodies[0].mesh.positions.chunks_exact(3).fold(f32::MIN, |a, q| a.max(q[2])));
        assert_eq!(resolvida(&r, id).cotas[0].2, 120.);
        d.params.iter_mut().find(|p| p.name == "H").unwrap().expr = s("60");
        let r = recompute(&d);
        let (v1, z1) = (r.bodies[0].mass.volume, r.bodies[0].mesh.positions.chunks_exact(3).fold(f32::MIN, |a, q| a.max(q[2])));
        assert!(v1 > v0 * 1.05, "{v0} para {v1}");
        assert!(z1 > z0 + 20., "{z0} para {z1}");
        assert_eq!(resolvida(&r, id).gaiola.pos(topo).unwrap()[2], 180.);
    }

    /// Aceite do B303: apagar o pai gera estado de erro, e a forma fica onde estava.
    #[test]
    fn apagar_o_pai_vira_erro_e_nao_mexe_na_forma() {
        let mut d = Document::new("v");
        let g = Gaiola::plano([0., 0., 0.], [40., 40.], [2, 2]);
        let plano = plano_em(&mut d, "30");
        let PlaneRef::Datum(pai) = plano else { unreachable!() };
        let a = no_em(&g, [0., 0., 0.]);
        let v = Vinculos { nos: vec![VinculoNo { no: a, alvo: AlvoNo::Plano { plano, uv: None } }], ..Default::default() };
        let id = d.add_feature(forma(g, v, false));
        assert!(d.dependents(pai).contains(&id), "a forma aparece como dependente do pai");
        let r = recompute(&d);
        assert!(matches!(r.status.get(&id), Some(Status::Ok)));
        d.delete(pai, false);
        let r = recompute(&d);
        let Some(Status::Error(m)) = r.status.get(&id) else { panic!("{:?}", r.status.get(&id)) };
        assert!(m.contains("nó"), "{m}");
        assert_eq!(r.formas.len(), 1, "a superfície continua na tela");
        assert_eq!(r.formas[0].malha.caixa().unwrap().1[2], 0., "e parada onde a gaiola guardada mandava");
        assert!(r.formas_resolvidas.is_empty());
    }

    #[test]
    fn face_que_sumiu_tambem_vira_erro() {
        let mut d = Document::new("v");
        let furo = caixa_com_altura(&mut d, "20");
        let r = recompute(&d);
        let b = &r.bodies[0];
        let chave = chave_de_cima(b, 19.);
        assert!(chave.is_some());
        let g = Gaiola::plano([0., 0., 50.], [20., 20.], [2, 2]);
        let n = no_em(&g, [0., 0., 50.]);
        let face = SurfRef::Face { point: [0., 0., 20.], normal: [0., 0., 1.], owner: Some(furo), key: chave };
        let id = d.add_feature(forma(g, Vinculos { nos: vec![VinculoNo { no: n, alvo: AlvoNo::Face { face } }], ..Default::default() }, false));
        let r = recompute(&d);
        assert!(matches!(r.status.get(&id), Some(Status::Ok)), "{:?}", r.status.get(&id));
        assert_eq!(resolvida(&r, id).gaiola.pos(n).unwrap()[2], 20., "o nó cai na face");
        d.delete(furo, false);
        let r = recompute(&d);
        assert!(matches!(r.status.get(&id), Some(Status::Error(_))), "{:?}", r.status.get(&id));
    }

    #[test]
    fn no_que_sumiu_da_gaiola_e_erro() {
        let mut d = Document::new("v");
        let g = Gaiola::plano([0., 0., 0.], [40., 40.], [2, 2]);
        let v = Vinculos { nos: vec![VinculoNo { no: 9999, alvo: AlvoNo::Ponto(PointRef::Origin) }], ..Default::default() };
        let id = d.add_feature(forma(g, v, false));
        let Some(Status::Error(m)) = recompute(&d).status.get(&id).cloned() else { panic!() };
        assert!(m.contains("9999"), "{m}");
    }

    /// Lado de cima de uma caixa: o caminho da forma casa com a aresta e a face continua na forma.
    #[test]
    fn borda_casa_com_aresta_do_modelo_e_segue_a_altura() {
        let mut d = Document::new("v");
        d.add_param("H", "20", Unit::Comprimento).unwrap();
        let caixa = caixa_com_altura(&mut d, "H");
        let r = recompute(&d);
        let b = &r.bodies[0];
        // aresta de cima na beira x = 20
        let topo = b.mesh.edge_ranges.len();
        let (mut chave, mut pts) = (None, vec![]);
        for e in 0..topo {
            let l = fenkra_solidos_edge(&r, e);
            if l.iter().all(|p| p[2] > 19.9 && p[0] > 19.9) {
                chave = b.edge_key(e);
                pts = l;
            }
        }
        let key = chave.expect("aresta");
        let g = Gaiola::plano([30., 0., 20.], [20., 40.], [2, 4]);
        let mut caminho: Vec<(f64, u32)> = g.nos.iter().filter(|n| (n.p[0] - 20.).abs() < 1e-9).map(|n| (n.p[1], n.id)).collect();
        caminho.sort_by(|a, b| a.0.total_cmp(&b.0));
        let nos: Vec<u32> = caminho.iter().map(|x| x.1).collect();
        let face = SurfRef::Face { point: [0., 0., 20.], normal: [0., 0., 1.], owner: Some(caixa), key: chave_de_cima(b, 19.) };
        let v = Vinculos { bordas: vec![VinculoBorda { nos: nos.clone(), aresta: CurveRef::BodyEdge { pts, key }, face: Some(face), continuidade: 2 }], ..Default::default() };
        let id = d.add_feature(forma(g, v, false));
        let r = recompute(&d);
        assert!(matches!(r.status.get(&id), Some(Status::Ok)), "{:?}", r.status.get(&id));
        let x = resolvida(&r, id);
        assert_eq!(x.bordas[0].obtida, 2, "{:?}", x.bordas[0]);
        assert!(x.bordas[0].desvio < 1e-6 && x.bordas[0].angulo < 1e-6, "{:?}", x.bordas[0]);
        d.params.iter_mut().find(|p| p.name == "H").unwrap().expr = s("35");
        let r = recompute(&d);
        assert!(matches!(r.status.get(&id), Some(Status::Ok)), "{:?}", r.status.get(&id));
        let x = resolvida(&r, id);
        assert!(nos.iter().all(|n| (x.gaiola.pos(*n).unwrap()[2] - 35.).abs() < 1e-6), "a borda subiu com a aresta");
        // a fileira vizinha continua no plano da face de cima: tangente
        assert!(x.gaiola.nos.iter().filter(|n| n.p[0] < 31.).all(|n| (n.p[2] - 35.).abs() < 1e-6), "{:?}", x.gaiola.nos);
    }

    fn fenkra_solidos_edge(r: &crate::RecomputeResult, e: usize) -> Vec<[f64; 3]> {
        let m = &r.bodies[0].mesh;
        let s0 = m.edge_ranges[e] as usize;
        let s1 = m.edge_ranges.get(e + 1).map_or(m.edges.len() / 6, |x| *x as usize);
        let mut out = vec![];
        for k in s0..s1 {
            let q = &m.edges[k * 6..k * 6 + 6];
            if k == s0 {
                out.push([q[0] as f64, q[1] as f64, q[2] as f64]);
            }
            out.push([q[3] as f64, q[4] as f64, q[5] as f64]);
        }
        out
    }

    #[test]
    fn g2_em_face_curva_avisa_e_nao_finge() {
        let mut d = Document::new("v");
        // cilindro em pé de raio 50: a reta de referência corre na geratriz
        let cil = d.add_feature(FeatureKind::Primitive(Primitive { kind: PrimKind::Cilindro, plane: PlaneRef::Origin(OriginPlane::XY), a: s("50"), b: s("80"), c: s("0"), d: s("0"), op: BoolOp::Add }));
        let lateral = SurfRef::Face { point: [50., 0., 40.], normal: [1., 0., 0.], owner: Some(cil), key: None };
        let linha = d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed([50., 0., 0.]), b: PointRef::Fixed([50., 0., 80.]), start: s("0"), end: s("0") });
        let mut g = Gaiola::plano([20., 0., 0.], [40., 80.], [2, 2]);
        // a folha fica em pé, com a borda x = 40 na geratriz
        for n in &mut g.nos {
            n.p = [50., n.p[0] - 40., n.p[1] + 40.];
        }
        let mut caminho: Vec<(f64, u32)> = g.nos.iter().filter(|n| n.p[1].abs() < 1e-9).map(|n| (n.p[2], n.id)).collect();
        caminho.sort_by(|a, b| a.0.total_cmp(&b.0));
        let nos: Vec<u32> = caminho.iter().map(|x| x.1).collect();
        let v = Vinculos { bordas: vec![VinculoBorda { nos, aresta: CurveRef::Ref(linha), face: Some(lateral), continuidade: 2 }], ..Default::default() };
        let id = d.add_feature(forma(g, v, false));
        let r = recompute(&d);
        let Some(Status::Warning(m)) = r.status.get(&id) else { panic!("{:?}", r.status.get(&id)) };
        assert!(m.contains("G1"), "{m}");
        let rel = &resolvida(&r, id).bordas[0];
        assert_eq!(rel.obtida, 1);
        assert!(rel.motivo.as_ref().unwrap().contains("50."), "{rel:?}");
    }

    #[test]
    fn no_da_costura_acompanha_so_dentro_do_plano() {
        let mut d = Document::new("v");
        let mut g = Gaiola::caixa([0.; 3], [40.; 3], [2, 2, 2]);
        g.ligar_simetria(subd::Simetria { eixo: 0, pos: 0. }).unwrap();
        let a = no_em(&g, [0., 0., 20.]);
        let v = Vinculos { nos: vec![VinculoNo { no: a, alvo: AlvoNo::Ponto(PointRef::Fixed([5., 0., 30.])) }], ..Default::default() };
        let id = d.add_feature(forma(g, v, false));
        let r = recompute(&d);
        assert!(matches!(r.status.get(&id), Some(Status::Warning(_))), "{:?}", r.status.get(&id));
        assert_eq!(resolvida(&r, id).gaiola.pos(a).unwrap(), [0., 0., 30.]);
    }

    #[test]
    fn vinculo_nao_atravessa_a_simetria() {
        let mut d = Document::new("v");
        let mut g = Gaiola::caixa([0.; 3], [40.; 3], [2, 2, 2]);
        g.ligar_simetria(subd::Simetria { eixo: 0, pos: 0. }).unwrap();
        let a = no_em(&g, [20., 0., 20.]);
        let v = Vinculos { nos: vec![VinculoNo { no: a, alvo: AlvoNo::Ponto(PointRef::Fixed([-5., 0., 30.])) }], ..Default::default() };
        let id = d.add_feature(forma(g, v, false));
        let Some(Status::Error(m)) = recompute(&d).status.get(&id).cloned() else { panic!() };
        assert!(m.contains("simetria"), "{m}");
    }

    #[test]
    fn salvar_e_abrir_preserva_os_vinculos() {
        let v = Vinculos {
            nos: vec![VinculoNo { no: 3, alvo: AlvoNo::Curva { curva: CurveRef::Axis(OriginAxis::X), t: Some(0.25) } }],
            cotas: vec![CotaNo { no: 4, plano: PlaneRef::Origin(OriginPlane::XY), expr: s("H/2") }],
            bordas: vec![VinculoBorda { nos: vec![1, 2], aresta: CurveRef::Axis(OriginAxis::Y), face: None, continuidade: 0 }],
        };
        let js = serde_json::to_string(&v).unwrap();
        assert_eq!(serde_json::from_str::<Vinculos>(&js).unwrap(), v);
        // arquivo antigo, sem o campo, abre com a lista vazia
        let velho = r#"{"gaiola":{"nos":[],"faces":[],"prox_no":0,"prox_face":0}}"#;
        assert!(serde_json::from_str::<Forma>(velho).unwrap().vinculos.vazio());
    }
}
