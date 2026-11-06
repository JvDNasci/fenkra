// SPDX-License-Identifier: LGPL-2.1-or-later
//! B14: a seta de comprimento de uma superfície passa sobre um elemento e o propõe como limite. Ao
//! soltar em cima dele, o comando deixa o comprimento e vai "até" esse elemento.

use super::tag;
use crate::app::App;
use crate::i18n::tr;
use crate::picking::{Pick, SURF_BASE};
use eframe::egui::{self, Rect};
use fenkra_core::*;

/// O campo `slot` deste elemento aceita um limite no lugar do comprimento?
pub(super) fn aceita(k: &RefKind, slot: u8) -> bool {
    matches!((k, slot), (RefKind::SurfExtrude { .. }, 0 | 1) | (RefKind::SurfExtrapolate { .. }, 0))
}

/// O elemento sob o cursor como limite; o próprio comando não para em si mesmo.
pub(super) fn candidato(pick: &Pick, comando: Id, superficie: Option<Id>, chave: Option<FaceKey>) -> Option<SurfRef> {
    match pick {
        Pick::Plane(p) => Some(SurfRef::Plane(*p)),
        Pick::Face { body, .. } if *body >= SURF_BASE => superficie.filter(|s| *s != comando).map(SurfRef::Ref),
        Pick::Face { owner, point, normal, .. } if *owner != comando => Some(SurfRef::Face { point: point.map(|v| v as f64), normal: normal.map(|v| v as f64), owner: Some(*owner), key: chave }),
        _ => None,
    }
}

/// Grava o limite no campo da seta.
pub(super) fn aplicar(k: &mut RefKind, slot: u8, s: SurfRef) {
    match (k, slot) {
        (RefKind::SurfExtrude { until, .. }, 0) | (RefKind::SurfExtrapolate { until, .. }, 0) => *until = Some(s),
        (RefKind::SurfExtrude { until2, .. }, 1) => *until2 = Some(s),
        _ => {}
    }
}

/// Mostra a proposta junto da seta enquanto ela passa sobre o elemento; soltar aceita.
pub(super) fn propor(app: &mut App, ui: &mut egui::Ui, rect: Rect, id: Id, slot: u8, ponta: egui::Pos2, soltou: bool) {
    if !app.open.doc.reference(id).is_some_and(|r| aceita(&r.kind, slot)) {
        return;
    }
    let Some(p) = app.hover_pick.clone() else { return };
    let superficie = match p {
        Pick::Face { body, .. } if body >= SURF_BASE => app.result.as_ref().and_then(|r| crate::picking::surface_of(r, body)),
        _ => None,
    };
    let Some(novo) = candidato(&p, id, superficie, app.face_key_of(Some(&p))) else { return };
    let cor = crate::theme::accent(ui.visuals());
    tag(ui, rect, ponta + egui::vec2(0., -24.), tr("Soltar: até este elemento").to_string(), cor, ("limite_seta", id.0));
    if soltou {
        if let Some(r) = app.open.doc.references.iter_mut().find(|r| r.id == id) {
            aplicar(&mut r.kind, slot, novo);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn extrusao() -> RefKind {
        RefKind::SurfExtrude { curve: CurveRef::Axis(OriginAxis::X), dir: AxisRef::Origin(OriginAxis::Z), length: "10".into(), length2: "0".into(), until: None, until2: None }
    }

    #[test]
    fn seta_sobre_plano_e_face_propoe_limite_e_soltar_grava_no_lado_certo() {
        let plano = SurfRef::Plane(PlaneRef::Origin(OriginPlane::XY));
        assert_eq!(candidato(&Pick::Plane(PlaneRef::Origin(OriginPlane::XY)), Id(1), None, None), Some(plano));
        let mut k = extrusao();
        aplicar(&mut k, 1, plano);
        assert!(matches!(k, RefKind::SurfExtrude { until: None, until2: Some(_), .. }), "a seta 2 grava o limite 2");
        aplicar(&mut k, 0, plano);
        assert!(matches!(k, RefKind::SurfExtrude { until: Some(_), .. }));
    }

    #[test]
    fn seta_nao_para_no_proprio_comando_nem_em_comando_sem_limite() {
        let f = |body, owner| Pick::Face { body, face: 0, owner, point: [0.; 3], normal: [0., 0., 1.] };
        assert!(candidato(&f(0, Id(5)), Id(5), None, None).is_none());
        assert!(candidato(&f(SURF_BASE, Id(9)), Id(5), Some(Id(5)), None).is_none());
        assert_eq!(candidato(&f(SURF_BASE, Id(9)), Id(5), Some(Id(9)), None), Some(SurfRef::Ref(Id(9))));
        assert!(matches!(candidato(&f(0, Id(9)), Id(5), None, None), Some(SurfRef::Face { .. })));
        assert!(aceita(&extrusao(), 1) && !aceita(&extrusao(), 2));
        assert!(!aceita(&RefKind::SurfSphere { center: PointRef::Origin, radius: "1".into() }, 0));
    }
}
