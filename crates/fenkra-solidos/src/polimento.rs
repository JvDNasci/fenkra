// SPDX-License-Identifier: LGPL-2.1-or-later
//! Malha de tela (B346, ADR 0069): a superfície que se vê não depende da malha de trabalho.
//!
//! A malha de trabalho segue a qualidade e serve à seleção, às medidas e ao desenho, então não pode
//! mudar com o zoom. A de tela segue a câmera, sai de uma cópia da forma e só entra na cena: o resultado
//! do cálculo a traz em `RecomputeResult::exibicao`, por corpo ou superfície.
//!
//! Só o pedido da vista (`Estado::agora`) tessela. Editar não paga por ela: o corpo que mudou volta à
//! malha de trabalho até a vista pedir de novo, e o que não mudou continua com a que tinha.

use crate::{forma_vinculo::FormaResolvida, hash_of, FormaViz};
use fenkra_core::subd::MalhaLisa;
use fenkra_core::{Document, FeatureKind, Id, Polimento, QualidadeMalha};
use fenkra_kernel::{Mesh, Shape};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Erro relativo da curvatura interpolada que o mapa aceita antes de dividir uma aresta: abaixo disso
/// a cor não muda de faixa na escala.
const CURVATURA_TOL: f64 = 0.03;
/// Passos a mais que a forma livre pode ganhar além dos que a gaiola pede.
const PASSOS_MAX: u32 = 4;

struct Guardada {
    sig: u64,
    alvo: Polimento,
    qualidade: QualidadeMalha,
    /// `None`: a malha de trabalho já atende (peça plana, ou o teto não deixou afinar)
    malha: Option<Arc<Mesh>>,
}

struct Lisa {
    sig: u64,
    passos: u32,
    malha: MalhaLisa,
}

/// O que o cálculo guarda da malha de tela entre um pedido e outro.
#[derive(Default)]
pub struct Estado {
    /// o que a vista quer; `None` desliga
    pub alvo: Option<Polimento>,
    /// este cálculo pode tesselar? Sem isso só reaproveita o que já tem
    pub agora: bool,
    malhas: HashMap<Id, Guardada>,
    superficies: HashMap<Id, (u64, Shape)>,
    lisas: HashMap<Id, Lisa>,
}

/// Uma forma com a malha de trabalho dela.
pub(crate) struct Alvo<'a> {
    pub id: Id,
    pub sig: u64,
    pub forma: Shape,
    pub base: &'a Mesh,
}

/// Triângulos das faces curvas: é o que cresce quando a malha afina.
fn triangulos_curvos(m: &Mesh) -> usize {
    let total = m.indices.len();
    (0..m.face_ranges.len())
        .filter(|f| m.face_info.get(*f).is_some_and(|i| i.kind != 0))
        .map(|f| m.face_ranges.get(f + 1).map_or(total, |x| *x as usize) - m.face_ranges[f] as usize)
        .sum::<usize>()
        / 3
}

/// Malha de tela de uma forma, ou `None` se ela não mudaria nada.
fn malha_de_tela(a: &Alvo, q: &QualidadeMalha, p: &Polimento, folga: f64) -> Result<Option<Mesh>, fenkra_kernel::KernelError> {
    let curvos = triangulos_curvos(a.base);
    if curvos == 0 {
        return Ok(None);
    }
    let diagonal = a.forma.bbox().map(|(x, y)| crate::geo3d::dist(x, y)).unwrap_or(100.);
    let (d0, a0) = (q.desvio(diagonal), q.angulo_graus);
    let (d, ang) = p.dentro_do_teto(curvos, d0, a0, folga);
    if d >= d0 * 0.99 && ang >= a0 * 0.99 && !p.curvatura {
        return Ok(None);
    }
    let tol = if p.curvatura { CURVATURA_TOL } else { 0. };
    a.forma.mesh_view(d, ang.to_radians(), tol, 1. / (2. * diagonal.max(1e-6))).map(Some)
}

impl Estado {
    /// Guarda a forma de uma superfície de referência: o cálculo só a vê quando ela muda.
    pub(crate) fn lembrar_superficie(&mut self, id: Id, sig: u64, forma: &Shape) {
        self.superficies.insert(id, (sig, forma.clone()));
    }

    pub(crate) fn superficie(&self, id: Id) -> Option<(u64, Shape)> {
        self.superficies.get(&id).map(|(s, f)| (*s, f.clone()))
    }

    pub(crate) fn esquecer_superficies(&mut self, doc: &Document) {
        self.superficies.retain(|id, _| doc.reference(*id).is_some());
    }

    /// Malhas de tela de `alvos`, por id. O que não pode ser feito agora fica na malha de trabalho.
    pub(crate) fn aplicar(&mut self, alvos: &[Alvo], q: &QualidadeMalha, cancelar: Option<&AtomicBool>) -> HashMap<Id, Arc<Mesh>> {
        self.malhas.retain(|id, g| alvos.iter().any(|a| a.id == *id && a.sig == g.sig));
        let Some(p) = self.alvo else {
            self.malhas.clear();
            return HashMap::new();
        };
        let total: usize = alvos.iter().map(|a| triangulos_curvos(a.base)).sum();
        let mut saida = HashMap::new();
        for a in alvos {
            let pronta = self.malhas.get(&a.id).is_some_and(|g| g.alvo == p && g.qualidade == *q);
            if self.agora && !pronta && !cancelar.is_some_and(|c| c.load(Ordering::Relaxed)) {
                let folga = triangulos_curvos(a.base) as f64 / total.max(1) as f64;
                // cancelado no meio: fica a malha anterior, e a vista pede de novo
                if let Ok(m) = malha_de_tela(a, q, &p, folga) {
                    self.malhas.insert(a.id, Guardada { sig: a.sig, alvo: p, qualidade: *q, malha: m.map(Arc::new) });
                }
            }
            if let Some(m) = self.malhas.get(&a.id).and_then(|g| g.malha.clone()) {
                saida.insert(a.id, m);
            }
        }
        saida
    }

    /// Troca a malha de cada forma livre pela de tela, quando houver. A forma ainda não tem B-rep:
    /// afinar é dar mais passos da mesma subdivisão, que converge para a mesma superfície limite.
    pub(crate) fn formas(&mut self, doc: &Document, formas: &mut [FormaViz], resolvidas: &[FormaResolvida]) {
        self.lisas.retain(|id, _| formas.iter().any(|f| f.id == *id));
        for f in formas {
            let Some(FeatureKind::Forma(fm)) = doc.feature(f.id).map(|x| &x.kind) else { continue };
            let g = resolvidas.iter().find(|x| x.id == f.id).map_or(&fm.gaiola, |x| &x.gaiola);
            let sig = hash_of(&[&serde_json::to_string(g).unwrap_or_default(), &fm.niveis.to_string()], 0);
            if self.lisas.get(&f.id).is_some_and(|l| l.sig != sig) {
                self.lisas.remove(&f.id);
            }
            let Some(p) = self.alvo else {
                self.lisas.remove(&f.id);
                continue;
            };
            if self.agora {
                let passos = passos_para(&f.malha, &p);
                if passos > 0 && self.lisas.get(&f.id).is_none_or(|l| l.passos != passos) {
                    if let Ok(malha) = g.lisa(fm.niveis as u32 + passos) {
                        self.lisas.insert(f.id, Lisa { sig, passos, malha });
                    }
                }
            }
            if let Some(l) = self.lisas.get(&f.id) {
                f.malha = l.malha.clone();
            }
            if p.curvatura {
                f.curvatura = f.malha.curvaturas();
            }
        }
    }
}

/// Passos de subdivisão que levam a malha ao pedido. Cada passo divide o ângulo entre vértices por 2 e
/// a flecha por 4, e multiplica os triângulos por 4.
fn passos_para(m: &MalhaLisa, p: &Polimento) -> u32 {
    let (mut angulo, mut flecha) = (0f64, 0f64);
    for t in &m.tris {
        for e in 0..3 {
            let (a, b) = (t[e] as usize, t[(e + 1) % 3] as usize);
            let (na, nb) = (m.nrm[a], m.nrm[b]);
            let cos = (na[0] * nb[0] + na[1] * nb[1] + na[2] * nb[2]).clamp(-1., 1.);
            let th = cos.acos();
            let l = (0..3).map(|k| (m.pos[a][k] - m.pos[b][k]).powi(2)).sum::<f64>().sqrt();
            angulo = angulo.max(th);
            flecha = flecha.max(l * th / 8.);
        }
    }
    let pelo_angulo = (angulo.to_degrees() / p.angulo_graus).max(1.).log2().ceil();
    let pela_flecha = (flecha / p.desvio_mm).max(1.).log(4.).ceil();
    let cabe = ((p.teto_triangulos as f64 / m.tris.len().max(1) as f64).max(1.)).log(4.).floor();
    (pelo_angulo.max(pela_flecha).min(cabe) as u32).min(PASSOS_MAX)
}

#[cfg(test)]
mod tests {
    use crate::{recompute_cached, Cache};
    use fenkra_core::*;

    fn s(x: &str) -> String {
        x.to_string()
    }

    fn esfera(raio: &str) -> Document {
        let mut d = Document::new("polimento");
        d.add_feature(FeatureKind::Primitive(Primitive { kind: PrimKind::Esfera, plane: PlaneRef::Origin(OriginPlane::XY), a: s(raio), b: String::new(), c: String::new(), d: String::new(), op: BoolOp::Add }));
        d
    }

    fn alvo() -> Polimento {
        Polimento { desvio_mm: 0.02, angulo_graus: 5., curvatura: false, teto_triangulos: Polimento::TETO }
    }

    fn cache() -> Cache {
        Cache::com_qualidade(QualidadeMalha { desvio_rel: 0.02, angulo_graus: 45. })
    }

    #[test]
    fn malha_de_tela_afina_sem_tocar_no_resultado_de_trabalho() {
        let d = esfera("40");
        let mut c = cache();
        let antes = recompute_cached(&d, &mut c);
        assert!(antes.exibicao.is_empty(), "sem pedido da vista não há malha de tela");
        let b0 = &antes.bodies[0];
        c.polir.alvo = Some(alvo());
        c.polir.agora = true;
        let r = recompute_cached(&d, &mut c);
        let b = &r.bodies[0];
        assert_eq!(b.mesh.indices, b0.mesh.indices, "a malha de trabalho não muda com a vista");
        assert!((b.mass.volume - 4. / 3. * std::f64::consts::PI * 40f64.powi(3)).abs() < 1.);
        let tela = &r.exibicao[&b.collector];
        assert!(tela.indices.len() > b.mesh.indices.len() * 8, "{} vs {}", tela.indices.len(), b.mesh.indices.len());
        assert_eq!(tela.face_ranges.len(), b.mesh.face_ranges.len());
        for i in 0..tela.positions.len() / 3 {
            let p = [0, 1, 2].map(|k| tela.positions[i * 3 + k] as f64);
            assert!(((p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt() - 40.).abs() < 1e-3);
        }
    }

    #[test]
    fn editar_nao_tessela_e_a_vista_pede_de_novo() {
        let mut d = esfera("40");
        let mut c = cache();
        c.polir.alvo = Some(alvo());
        c.polir.agora = true;
        let r = recompute_cached(&d, &mut c);
        let id = r.bodies[0].collector;
        assert!(r.exibicao.contains_key(&id));
        // o mesmo documento sem o pedido continua com a malha pronta
        c.polir.agora = false;
        let r = recompute_cached(&d, &mut c);
        assert!(r.exibicao.contains_key(&id), "corpo que não mudou segue polido");
        // o corpo que mudou volta à malha de trabalho, e a nova forma aparece nela
        if let FeatureKind::Primitive(p) = &mut d.features.last_mut().unwrap().kind {
            p.a = s("50");
        }
        let r = recompute_cached(&d, &mut c);
        assert!(!r.exibicao.contains_key(&id), "malha velha de outra forma não entra na cena");
        c.polir.agora = true;
        let r = recompute_cached(&d, &mut c);
        let tela = &r.exibicao[&id];
        let p = [tela.positions[0] as f64, tela.positions[1] as f64, tela.positions[2] as f64];
        assert!(((p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt() - 50.).abs() < 1e-3);
    }

    #[test]
    fn peca_plana_nao_ganha_malha_de_tela() {
        let mut d = Document::new("caixa");
        d.add_feature(FeatureKind::Primitive(Primitive { kind: PrimKind::Caixa, plane: PlaneRef::Origin(OriginPlane::XY), a: s("40"), b: s("30"), c: s("20"), d: String::new(), op: BoolOp::Add }));
        let mut c = cache();
        c.polir.alvo = Some(alvo());
        c.polir.agora = true;
        assert!(recompute_cached(&d, &mut c).exibicao.is_empty());
    }

    #[test]
    fn teto_segura_a_malha_de_tela() {
        let d = esfera("40");
        let mut c = cache();
        c.polir.alvo = Some(Polimento { desvio_mm: 0.002, angulo_graus: 5., curvatura: false, teto_triangulos: 3000 });
        c.polir.agora = true;
        let r = recompute_cached(&d, &mut c);
        let tela = &r.exibicao[&r.bodies[0].collector];
        assert!(tela.indices.len() / 3 < 3000 * 2, "{} triângulos", tela.indices.len() / 3);
        assert!(tela.indices.len() > r.bodies[0].mesh.indices.len());
    }

    #[test]
    fn superficie_de_referencia_ganha_malha_de_tela_e_curvatura_refinada() {
        let mut d = Document::new("painel");
        let mut secoes = vec![];
        for i in 0..4 {
            let y = i as f64 * 40.;
            let f = [0., 14., -10., 8.][i];
            let pts = [[-70., y, f], [-30., y, 22. - f], [15., y, -8. + f], [70., y, 12. - f]];
            let c = d.add_geometry(RefKind::Spline { points: pts.iter().map(|p| PointRef::Fixed(*p)).collect(), closed: false, tangents: vec![] });
            secoes.push(CurveRef::Ref(c));
        }
        let sup = d.add_geometry(RefKind::SurfLoft { sections: secoes, guide: None, ruled: false, closed: false, opts: Default::default() });
        let mut c = cache();
        c.polir.alvo = Some(alvo());
        c.polir.agora = true;
        let sem = recompute_cached(&d, &mut c).exibicao[&sup].indices.len();
        c.polir.alvo = Some(Polimento { curvatura: true, ..alvo() });
        let r = recompute_cached(&d, &mut c);
        let m = &r.exibicao[&sup];
        assert!(m.indices.len() > sem, "{} vs {sem}", m.indices.len());
        assert!(m.curvature.iter().any(|k| k.abs() > 1e-4), "a curvatura do painel existe");
    }

    #[test]
    fn superficie_interrompida_nao_fica_quebrada_no_cache() {
        let mut d = Document::new("cancelado");
        let sup = d.add_geometry(RefKind::SurfSphere { center: PointRef::Fixed([0., 0., 0.]), radius: s("15") });
        let mut c = cache();
        // a bandeira ligada faz a malha falhar, como um pedido novo faria no meio do cálculo
        c.cancelar = Some(std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true)));
        let cancelado = recompute_cached(&d, &mut c);
        assert!(cancelado.surfaces.iter().all(|x| x.id != sup), "sem resultado neste cálculo");
        c.cancelar = None;
        let r = recompute_cached(&d, &mut c);
        assert!(r.surfaces.iter().any(|x| x.id == sup), "o cálculo seguinte calcula de novo: {:?}", r.status.get(&sup));
    }

    #[test]
    fn forma_livre_ganha_passos_ate_o_pedido() {
        let mut d = Document::new("forma");
        let id = d.add_feature(FeatureKind::Forma(Forma { gaiola: fenkra_core::subd::Gaiola::caixa([0., 0., 15.], [60., 60., 30.], [2, 2, 2]), niveis: 3, congelada: false, conversao: None, vinculos: Default::default() }));
        let mut c = cache();
        let base = recompute_cached(&d, &mut c).formas.iter().find(|f| f.id == id).unwrap().malha.tris.len();
        c.polir.alvo = Some(Polimento { desvio_mm: 0.01, angulo_graus: 5., curvatura: false, teto_triangulos: 500_000 });
        c.polir.agora = true;
        let fina = recompute_cached(&d, &mut c).formas.iter().find(|f| f.id == id).unwrap().malha.tris.len();
        assert!(fina >= base * 4 && fina <= 500_000 * 4, "{base} virou {fina}");
    }
}
