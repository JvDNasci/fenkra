// SPDX-License-Identifier: LGPL-2.1-or-later
//! B392 (K-14 do kart): recorte novo que saiu para o lado sem material vira o sentido sozinho.
//!
//! O recorte entra contra a normal do plano do sketch e a adição sai a favor; quem cria um recorte
//! num plano com a peça do outro lado descobria o "inverter" por tentativa. O cálculo já confere o
//! outro lado quando o recorte não remove nada (`fenkra_solidos::sentido_corte`); aqui, só para o
//! comando recém-criado e só uma vez por comando, o app liga ou desliga "inverter" e avisa. Depois
//! disso a escolha é de quem usa: desfazer a virada não é desfeito de novo.

use fenkra_core::{Document, FeatureKind, Id};
use fenkra_solidos::Status;
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

static VIRADOS: Mutex<Option<HashSet<Id>>> = Mutex::new(None);

/// Texto do aviso ao virar o sentido.
pub const VIRADO: &str = "O recorte ia para o lado sem material: o sentido foi virado para entrar na peça ('inverter' mudou).";

/// `novo`: o comando aberto acabou de ser criado. `calculado`: o documento que gerou `status`.
/// Devolve `true` se virou o sentido no documento aberto (é preciso recalcular).
pub fn ajusta(doc: &mut Document, calculado: Option<&Document>, novo: Option<Id>, status: &HashMap<Id, Status>) -> bool {
    let Some(id) = novo else { return false };
    let Some(Status::Warning(m)) = status.get(&id) else { return false };
    if !fenkra_solidos::sentido_corte::virar_resolve(m) {
        return false;
    }
    // o comando aberto tem de ser o mesmo que foi calculado (senão o aviso é de outra versão dele)
    let igual = matches!((calculado.and_then(|d| d.feature(id)), doc.feature(id)), (Some(a), Some(b)) if a.kind == b.kind);
    if !igual {
        return false;
    }
    let mut g = VIRADOS.lock().unwrap_or_else(|e| e.into_inner());
    if !g.get_or_insert_with(HashSet::new).insert(id) {
        return false;
    }
    match doc.feature_mut(id).map(|f| &mut f.kind) {
        Some(FeatureKind::Extrude(e)) => {
            e.reverse = !e.reverse;
            true
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fenkra_core::*;

    fn doc() -> (Document, Id) {
        let mut d = Document::new("c");
        let mut data = SketchData::default();
        let mut al = || d.alloc();
        data.add_rect(&mut al, [0., 0.], [10., 10.], false);
        let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
        let id = d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Cut, extent: Extent::ThroughAll, reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
        (d, id)
    }

    fn rev(d: &Document, id: Id) -> bool {
        matches!(&d.feature(id).unwrap().kind, FeatureKind::Extrude(e) if e.reverse)
    }

    #[test]
    fn vira_uma_vez_so_no_comando_novo_e_so_quando_o_outro_lado_resolve() {
        let (mut d, id) = doc();
        let calc = d.clone();
        let st = HashMap::from([(id, Status::Warning(fenkra_solidos::sentido_corte::LIGUE_INVERTER.into()))]);
        // comando que não é novo: não mexe
        assert!(!ajusta(&mut d, Some(&calc), None, &st));
        // aviso que não se resolve virando: não mexe
        let longe = HashMap::from([(id, Status::Warning(fenkra_solidos::sentido_corte::NAO_ALCANCA.into()))]);
        assert!(!ajusta(&mut d, Some(&calc), Some(id), &longe));
        assert!(!rev(&d, id));
        // comando novo para o lado sem material: vira
        assert!(ajusta(&mut d, Some(&calc), Some(id), &st));
        assert!(rev(&d, id));
        // quem desvira de propósito não é desvirado de novo
        if let FeatureKind::Extrude(e) = &mut d.feature_mut(id).unwrap().kind {
            e.reverse = false;
        }
        let calc = d.clone();
        assert!(!ajusta(&mut d, Some(&calc), Some(id), &st));
        assert!(!rev(&d, id));
    }

    #[test]
    fn resultado_de_outra_versao_do_comando_nao_vira() {
        let (mut d, id) = doc();
        let mut calc = d.clone();
        if let FeatureKind::Extrude(e) = &mut calc.feature_mut(id).unwrap().kind {
            e.extent = Extent::Blind("3".into());
        }
        let st = HashMap::from([(id, Status::Warning(fenkra_solidos::sentido_corte::LIGUE_INVERTER.into()))]);
        assert!(!ajusta(&mut d, Some(&calc), Some(id), &st));
    }
}
