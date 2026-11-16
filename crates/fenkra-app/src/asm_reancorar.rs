// SPDX-License-Identifier: LGPL-2.1-or-later
//! B474: reancorar uma restrição de referência perdida. O fluxo é: selecionar a restrição, clicar no
//! 3D a face certa e usar «Reancorar na seleção atual». O clique no 3D escolhia a peça clicada e
//! trocava a seleção, então o painel da restrição fechava justo quando o botão ficava ativo. Enquanto
//! a restrição selecionada tem referência perdida, o clique no 3D só escolhe o elemento e a
//! restrição continua no painel; depois de reancorar (estado OK) o clique volta a escolher a peça.

use fenkra_core::Id;
use fenkra_montagem::releitura::perdida;
use fenkra_montagem::Assembly;

/// A seleção é uma restrição com referência perdida, esperando o elemento novo?
pub fn manter_restricao(asm: &Assembly, sel: Option<Id>) -> bool {
    let Some(id) = sel else { return false };
    asm.mates.iter().find(|m| m.id == id).is_some_and(|m| perdida(&m.a) || perdida(&m.b))
}

/// O botão «Reancorar na seleção atual»: o lado A (`lado_a`) ou B da restrição `id` passa a usar o
/// elemento clicado por último. A seleção não muda: o painel da restrição continua aberto para
/// conferir o resultado (B474).
pub fn aplicar(st: &mut crate::asm_ui::AsmState, id: Id, lado_a: bool, clique: crate::picking::Pick) -> Result<(), String> {
    let nova = st.mate_ref(clique)?;
    fenkra_montagem::releitura::reancorar(&mut st.asm, id, lado_a, nova)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;
    use eframe::egui;
    use fenkra_montagem::releitura::Viva;
    use fenkra_montagem::{Geom, MateKind, MateRef};

    fn ref_de(comp: u64, perdida: bool) -> MateRef {
        MateRef { comp: Id(comp), geom: Geom::Plane { p: [0.; 3], n: [0., 0., 1.] }, pick: None, viva: Some(Viva::Face { chave: None, perdida }) }
    }

    fn montagem(perdida: bool) -> (Assembly, Id) {
        let mut a = Assembly::new("t");
        let id = a.add_mate(MateKind::Coincident { flip: false }, ref_de(1, perdida), ref_de(2, false));
        (a, id)
    }

    #[test]
    fn restricao_perdida_selecionada_continua_no_painel_ate_reancorar() {
        let (mut a, id) = montagem(true);
        assert!(manter_restricao(&a, Some(id)), "perdida: o clique no 3D não troca a seleção");
        // reancorado (estado OK): o clique volta a escolher a peça
        a.mates[0].a = ref_de(1, false);
        assert!(!manter_restricao(&a, Some(id)));
    }

    /// Bloco de `l` de lado, com o canto na origem.
    fn bloco(l: f64) -> fenkra_core::Document {
        use fenkra_core::{BoolOp, Document, Extent, Extrude, FeatureKind, OriginPlane, PlaneRef, Sketch, SketchData};
        let mut d = Document::new("Pino");
        let mut n = 7_300_000u64;
        let mut al = || {
            n += 1;
            Id(n)
        };
        let mut s = SketchData::default();
        s.add_rect(&mut al, [0., 0.], [l, l], false);
        let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data: s }));
        d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind(l.to_string()), reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
        d
    }

    /// Escolha de aresta como a viewport entrega: o corpo da peça e a aresta mais perto de `q`.
    fn aresta_da_vista(st: &crate::asm_ui::AsmState, comp: Id, q: [f64; 3]) -> crate::picking::Pick {
        let v = st.view.as_ref().expect("vista");
        let body = (0..v.bodies.len()).find(|b| st.comp_of_body(*b) == Some(comp)).expect("corpo da peça");
        let m = &v.bodies[body].mesh;
        let (edge, d) = crate::entradas::aresta_perto(m, q).expect("aresta");
        assert!(d < 0.2, "a aresta passa pelo ponto: {d}");
        let mid = [m.edge_mids[edge * 3], m.edge_mids[edge * 3 + 1], m.edge_mids[edge * 3 + 2]];
        crate::picking::Pick::Edge { body, edge, mid }
    }

    #[test]
    fn furo_apagado_deixa_a_coincidencia_perdida_e_o_painel_segue_aberto_depois_do_reancorar() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        use crate::asm_ui::AsmState;
        use fenkra_montagem::{NodeKind, Placement};
        // B479 + B474, como na rodada 4: placa de exemplo e um pino com o vértice na boca circular de
        // um dos "Furos M6"; apagar os furos deixa a coincidência perdida (antes ela ia para a aresta do
        // canto, OK e sem aviso), e o Reancorar não fecha o painel da restrição
        let dir = std::env::temp_dir().join(format!("fenkra_b479_reancorar_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let placa_doc = fenkra_solidos::example_document();
        std::fs::write(dir.join("placa.fkp"), placa_doc.to_json()).unwrap();
        std::fs::write(dir.join("pino.fkp"), bloco(8.).to_json()).unwrap();
        let mut a = Assembly::new("conj4");
        let placa = a.add_component("Placa", NodeKind::Part { path: "placa.fkp".into() }, Placement::default(), None);
        a.node_mut(placa).unwrap().fixed = true;
        let pino = a.add_component("Pino", NodeKind::Part { path: "pino.fkp".into() }, Placement { origin: [40., 20., 12.], ..Default::default() }, None);
        let mut st = AsmState::new(a, Some(dir.join("conj4.fka")));
        st.refresh();
        // a boca do rebaixo de (48, 24) termina no filete de 1: raio 6,5 na face de cima
        let boca = aresta_da_vista(&st, placa, [48. + 6.5, 24., 10.]);
        let ra = st.mate_ref(boca).unwrap();
        assert!(matches!(ra.geom, Geom::Axis { p, d } if (p[0] - 48.).abs() < 0.05 && (p[1] - 24.).abs() < 0.05 && d[2].abs() > 0.999), "{:?}", ra.geom);
        let v = st.view.as_ref().unwrap();
        let corpo_pino = (0..v.bodies.len()).find(|b| st.comp_of_body(*b) == Some(pino)).unwrap();
        let rb = st.mate_ref(crate::picking::Pick::Vertex { body: corpo_pino, pos: [40., 20., 12.] }).unwrap();
        let id = st.asm.add_mate(MateKind::Coincident { flip: false }, ra, rb);
        st.solve();
        assert!(st.report.converged, "{:?}", st.report);
        let o = st.asm.node(pino).unwrap().placement.origin;
        // ponto no eixo da boca: o pino desce até o eixo, na altura em que estava
        assert!((o[0] - 48.).abs() < 0.05 && (o[1] - 24.).abs() < 0.05 && (o[2] - 12.).abs() < 0.05, "o vértice do pino no eixo da boca: {o:?}");
        assert!(!manter_restricao(&st.asm, Some(id)));
        // Editar a peça, apagar "Furos M6", Ctrl+S, Voltar
        let mut sem = placa_doc.clone();
        sem.features.retain(|f| f.name != "Furos M6");
        std::fs::write(dir.join("placa.fkp"), sem.to_json()).unwrap();
        st.cache.forget(&dir.join("placa.fkp"));
        st.solve();
        assert!(st.report.converged);
        let m = st.asm.mates.iter().find(|m| m.id == id).unwrap();
        assert!(perdida(&m.a), "referência perdida, com aviso e Reancorar: {:?}", m.a);
        assert!(matches!(m.a.geom, Geom::Axis { p, .. } if (p[0] - 48.).abs() < 0.05 && (p[1] - 24.).abs() < 0.05), "não foi para o canto da placa: {:?}", m.a.geom);
        let o2 = st.asm.node(pino).unwrap().placement.origin;
        assert!((0..3).all(|i| (o2[i] - o[i]).abs() < 1e-6), "o pino fica onde estava: {o:?} -> {o2:?}");
        st.dirty = true;
        st.refresh();
        let v = st.view.as_ref().unwrap();
        assert!(v.status.values().all(|s| !matches!(s, fenkra_solidos::Status::Error(_))), "{:?}", v.status);
        let corpo_placa = (0..v.bodies.len()).find(|b| st.comp_of_body(*b) == Some(placa)).unwrap();
        let macica = 120. * 72. * 10. - (40. * 12. + std::f64::consts::PI * 36.) * 10.;
        assert!((v.bodies[corpo_placa].mass.volume - macica).abs() < 0.02 * macica, "placa sem os furos: {} (esperado ~{macica})", v.bodies[corpo_placa].mass.volume);
        // painel da restrição aberto; clique na aresta da frente da placa: a seleção não muda
        let mut app = App::new(&eframe::CreationContext::_new_kittest(egui::Context::default()));
        app.settings.bench = crate::settings::Bench::Montagem;
        st.sel = Some(id);
        let frente = aresta_da_vista(&st, placa, [0., -35., 10.]);
        app.asm = Some(st);
        let ctx = egui::Context::default();
        app.picks = vec![frente];
        crate::asm_ui::after_viewport(&mut app, &ctx);
        assert_eq!(app.asm.as_ref().unwrap().sel, Some(id), "o clique só escolhe o elemento novo");
        // botão "Reancorar na seleção atual"
        let st = app.asm.as_mut().unwrap();
        aplicar(st, id, true, frente).unwrap();
        st.solve();
        assert!(st.report.converged);
        assert!(!manter_restricao(&st.asm, Some(id)), "reancorada: estado OK");
        let o3 = st.asm.node(pino).unwrap().placement.origin;
        assert!((o3[1] + 35.).abs() < 0.05 && (o3[2] - 10.).abs() < 0.05, "o vértice na aresta da frente: {o3:?}");
        // o quadro seguinte, com o mesmo clique: o painel da restrição continua aberto para conferir
        crate::asm_ui::after_viewport(&mut app, &ctx);
        assert_eq!(app.asm.as_ref().unwrap().sel, Some(id), "B474: o painel não fecha depois do Reancorar");
        // reler de novo não perde a aresta nova
        let st = app.asm.as_mut().unwrap();
        st.solve();
        assert!(!manter_restricao(&st.asm, Some(id)));
        // um clique novo, já com a restrição OK, volta a escolher a peça
        st.dirty = true;
        st.refresh();
        let clique_pino = aresta_da_vista(app.asm.as_ref().unwrap(), pino, [o3[0] + 4., o3[1], o3[2]]);
        app.picks = vec![clique_pino];
        crate::asm_ui::after_viewport(&mut app, &ctx);
        assert_eq!(app.asm.as_ref().unwrap().sel, Some(pino));
    }

    #[test]
    fn sem_restricao_perdida_o_clique_escolhe_a_peca() {
        let (a, id) = montagem(false);
        assert!(!manter_restricao(&a, Some(id)));
        assert!(!manter_restricao(&a, Some(Id(1))), "um componente não é restrição");
        assert!(!manter_restricao(&a, None));
    }
}
