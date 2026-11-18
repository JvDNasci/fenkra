// SPDX-License-Identifier: LGPL-2.1-or-later
//! Âncoras dos campos que só guardavam o ponto (B516, ADR 0010 fase 3).
//!
//! Os comandos de chapa (linha da dobra, do joggle e da veneziana, face da aba por superfície e da
//! conversão, centro do rasgo em arco, centros do furo flangeado) e as placas e reforços do estrutural
//! nasceram com um ponto clicado. A cada cálculo o ponto escolhia a face mais perto, e uma face que
//! andava (espessura, perfil) deixava o comando na face vizinha ou no ar sem aviso.
//!
//! Agora cada campo ganha uma âncora no comando (`Feature::ancoras`): a face por dono e papel, ou a
//! ponta de membro pelo comando, trecho e ponta. Como funciona:
//!
//! - **sem âncora** (arquivo antigo, ou ponto recém-clicado): o ponto vale como antes e, se ele cai numa
//!   face com papel conhecido, a chave dela vira âncora (promoção, gravada pelo app);
//! - **com âncora**: a face é achada pela chave. O ponto continua o mesmo enquanto estiver na face; se a
//!   face andou na direção da normal (a chapa ficou mais grossa), o ponto vai até ela. Face que sumiu,
//!   ambígua, ou que deixou o ponto fora dela é referência perdida, com as candidatas pelo nome;
//! - a âncora só vale enquanto o campo tiver o mesmo ponto: clicar outra face troca o ponto e a âncora
//!   velha deixa de valer sozinha, sem o painel precisar saber dela;
//! - na chapa, a âncora só vale no estado (dobrado ou planificado) em que o ponto foi escolhido; no
//!   outro estado as faces são outras e o ponto segue levado pelo mapa de dobras, como antes.

use super::*;
use fenkra_core::{Ancora, ChaveAncora};

/// Ponto de uma face plana mais perto de `p` (projeção no plano dela).
fn no_plano(p: V3, f: &fenkra_kernel::FaceInfo) -> V3 {
    geo3d::sub(p, geo3d::mul(f.dir, geo3d::dot(geo3d::sub(p, f.point), f.dir)))
}

impl Ctx<'_> {
    /// Guarda a âncora nova para o app gravar no comando. Não apaga outra promoção do mesmo comando.
    pub(super) fn promover_ancora(&self, a: Ancora) {
        let mut p = self.promoted.borrow_mut();
        match &mut *p {
            Some(Promoted::Ancoras(v)) => {
                v.retain(|b| b.campo != a.campo);
                v.push(a);
            }
            None => *p = Some(Promoted::Ancoras(vec![a])),
            Some(_) => {}
        }
    }

    /// Ponto `p` do campo `campo`, numa face do corpo de destino, pela âncora (ver o módulo).
    /// `flat_now`: o estado atual da chapa (fora da chapa, `f.flat_space`).
    pub(super) fn ponto_na_face(&self, what: &str, f: &Feature, campo: &str, p: V3, flat_now: bool) -> Result<V3, String> {
        if f.flat_space != flat_now {
            return Ok(p);
        }
        let Some((b, owners, roles)) = self.body_owned(f.target) else { return Ok(p) };
        let k = |e: KernelError| e.0;
        let topo = b.topology().map_err(k)?;
        let v = refs::View { topo: &topo, owners, roles };
        let Some(key) = f.ancora_face(campo, p) else {
            // arquivo antigo ou ponto novo: vale o ponto; a face dele vira âncora se tiver nome
            if let Some(j) = b.face_index(p, 1.0).map_err(k)? {
                if let Some(key) = refs::promote_face(&v, j) {
                    self.promover_ancora(Ancora { campo: campo.into(), ponto: p, chave: ChaveAncora::Face(key) });
                }
            }
            return Ok(p);
        };
        let nome = || refs::nome_face(self.doc, key, &|s| s.to_string());
        let j = refs::find_face(&v, key, Some(p)).map_err(|why| self.lost(what, f, why, format!("a face '{}'{}", nome(), refs::sugerir_faces(self.doc, &v, Some(key), p))))?;
        let face = &topo.faces[j];
        let fora = || self.lost(what, f, refs::Lost::None, format!("o ponto na face '{}' (a face mudou e o ponto ficou fora dela){}", nome(), refs::sugerir_faces(self.doc, &v, Some(key), p)));
        if face.kind == 0 {
            let q = no_plano(p, face);
            if geo3d::dist(p, q) < 1e-6 || b.face_index(q, 1e-3).map_err(k)? == Some(j) {
                return Ok(q);
            }
            return Err(fora());
        }
        if b.face_index(p, 1e-2).map_err(k)? == Some(j) {
            Ok(p)
        } else {
            Err(fora())
        }
    }

    /// Linha `a`–`b` desenhada numa face (dobra, joggle, veneziana): a âncora é a do ponto médio, e a
    /// linha anda junto com ele quando a face anda (ver [`Ctx::ponto_na_face`]).
    pub(super) fn linha_na_face(&self, what: &str, f: &Feature, campo: &str, a: V3, b: V3, flat_now: bool) -> Result<(V3, V3), String> {
        let m = geo3d::lerp(a, b, 0.5);
        let q = self.ponto_na_face(what, f, campo, m, flat_now)?;
        let d = geo3d::sub(q, m);
        Ok((geo3d::add(a, d), geo3d::add(b, d)))
    }

    /// Ponta de membro estrutural do campo `campo` (placa e reforço, B516). Com âncora, a ponta é a do
    /// comando e trecho guardados, onde quer que ela esteja agora (`true` na terceira posição); sem âncora,
    /// a ponta mais perto do ponto, como antes, e a identidade dela vira âncora.
    pub(super) fn membro_ancorado(&self, what: &str, f: &Feature, campo: &str, at: V3) -> Result<(MemberGeom, bool, bool), String> {
        let tr = |s: &str| s.to_string();
        if let Some(ch @ ChaveAncora::PontaMembro { membro, trecho, fim }) = f.ancora(campo, at) {
            let dono = self.doc.features.iter().take_while(|g| g.id != f.id).find(|g| g.id == *membro);
            let achado = match dono.map(|g| &g.kind) {
                Some(FeatureKind::StructMember(m)) => self.members(m)?.into_iter().nth(*trecho as usize),
                _ => None,
            };
            if let Some(g) = achado {
                return Ok((g, !*fim, true));
            }
            let perto = self
                .member_end_ref(at, f)
                .ok()
                .map(|(_, s, id, i)| format!(" (a mais perto agora: {})", refs::nome_ancora(self.doc, &ChaveAncora::PontaMembro { membro: id, trecho: i as u32, fim: !s }, &tr)))
                .unwrap_or_default();
            return Err(self.lost(what, f, refs::Lost::None, format!("a ponta '{}'{perto}", refs::nome_ancora(self.doc, ch, &tr))));
        }
        let (g, s, id, i) = self.member_end_ref(at, f)?;
        self.promover_ancora(Ancora { campo: campo.into(), ponto: at, chave: ChaveAncora::PontaMembro { membro: id, trecho: i as u32, fim: !s } });
        Ok((g, s, false))
    }

    /// Chave da face de término "até a face" que só tinha o ponto (B516): vira `Promoted::Ate` para o app
    /// gravar em `SurfRef::Face::key`. Só quando o ponto cai numa face com nome, sem outra promoção.
    pub(super) fn promover_ate(&self, e: &Extent) {
        let Extent::UpToSurface(SurfRef::Face { point, key: None, .. }) = e else { return };
        if self.promoted.borrow().is_some() {
            return;
        }
        if let Some(fenkra_core::ChaveAncora::Face(k)) = self.chave_no_ponto(*point, false) {
            *self.promoted.borrow_mut() = Some(Promoted::Ate(k));
        }
    }

    /// Chave da face (ou da aresta, com `aresta`) em que o ponto cai, em qualquer corpo, se ela tiver
    /// nome e o nome sozinho a achar de novo (promoção de referências antigas, B516).
    pub(super) fn chave_no_ponto(&self, p: V3, aresta: bool) -> Option<ChaveAncora> {
        let mut achadas = vec![];
        for (b, owners, roles) in self.bodies.values() {
            if b.is_null() {
                continue;
            }
            let Ok(topo) = b.topology() else { continue };
            let v = refs::View { topo: &topo, owners, roles };
            if aresta {
                if let Some(e) = refs::edge_near(&v, p, 0.1) {
                    achadas.push(refs::promote_edge(&v, e).map(ChaveAncora::Aresta));
                }
            } else if let Ok(Some(j)) = b.face_index(p, 1e-3) {
                achadas.push(refs::promote_face(&v, j).map(ChaveAncora::Face));
            }
        }
        // o ponto em dois corpos (faces encostadas) não escolhe
        match achadas.as_slice() {
            [one] => one.clone(),
            _ => None,
        }
    }

    /// Linhas guia de membro que eram só os pontos de uma aresta do modelo (`CurveRef::Edge`, arquivo
    /// antigo): a aresta com nome vira `CurveRef::BodyEdge` (B516), e o membro passa a seguir a aresta
    /// quando a peça muda. Devolve as linhas novas se alguma mudou.
    pub(super) fn promover_caminhos(&self, paths: &[CurveRef]) -> Option<Vec<CurveRef>> {
        let mut out = paths.to_vec();
        let mut mudou = false;
        for c in out.iter_mut() {
            let CurveRef::Edge(pts) = c else { continue };
            if pts.len() < 2 {
                continue;
            }
            // ponto sobre a aresta: o meio da corda na reta (o polígono tem só as pontas), o vértice do
            // meio na curva
            let q = if pts.len() == 2 { geo3d::lerp(pts[0], pts[1], 0.5) } else { pts[pts.len() / 2] };
            let mut achadas = vec![];
            for (b, owners, roles) in self.bodies.values() {
                if b.is_null() {
                    continue;
                }
                let Ok(topo) = b.topology() else { continue };
                let v = refs::View { topo: &topo, owners, roles };
                let e = match refs::edge_near(&v, q, 0.1) {
                    Some(e) => Some(e),
                    None => b.edges_through(q, 0.01).ok().filter(|on| on.len() == 1).map(|on| on[0]),
                };
                if let Some(e) = e {
                    // o nome que, com o ponto para desempatar, acha a mesma aresta
                    achadas.push(v.edge_key(e).filter(|k| refs::find_edge(&v, k, Some(q)) == Ok(e)));
                }
            }
            if let [Some(key)] = achadas.as_slice() {
                *c = CurveRef::BodyEdge { pts: pts.clone(), key: *key };
                mudou = true;
            }
        }
        mudou.then_some(out)
    }
}
