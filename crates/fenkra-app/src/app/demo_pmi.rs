// SPDX-License-Identifier: LGPL-2.1-or-later
//! Desenvolvimento: casos do PMI 3D para captura sem mouse (`FENKRA_DEMO_FOTO`). Cada caso monta as
//! anotações na placa de exemplo (`FENKRA_DEMO_PMI=<caso>`) e seleciona a que mostra a alça ou o aviso:
//! `angulo` (ângulo entre planos, com a alça do arco), `chamada` e `chamada_cruza` (nota de três pontas e dois vértices, limpa e com o caminho cruzado), `perfil` (texto deitado visto de perfil) e `alcas` (raio e nota com a alça de posição).

use super::*;
use fenkra_core::pmi::*;

impl App {
    pub(super) fn demo_pmi_caso(&mut self, modo: &str) {
        if !matches!(modo, "angulo" | "chamada" | "chamada_cruza" | "perfil" | "alcas") {
            return;
        }
        let d = &mut self.open.doc;
        d.pmi.clear();
        let mut novo = |d: &mut Document, kind: PmiKind, anchors: Vec<[f64; 3]>, offset: [f64; 3], text: &str, nome: &str| {
            let id = d.alloc();
            d.pmi.push(Pmi { id, name: nome.into(), kind, anchors, offset, text: text.into(), tol_plus: String::new(), tol_minus: String::new(), visible: true, plane: None, style: Default::default(), folder: String::new(), targets: vec![] });
            id
        };
        let sel = match modo {
            "angulo" => {
                // topo (normal +Z) e frente (normal -Y) da placa: o diedro é de 90 graus
                let a = fenkra_core::pmi_infer::angle_from_planes([0., -20., 8.], [0., 0., 1.], [0., -35., 4.], [0., -1., 0.]).expect("planos concorrentes");
                let id = novo(d, PmiKind::Angle, a.to_vec(), [0., 0., 12.], "", "Ângulo entre planos");
                if let Some(p) = d.pmi.last_mut() {
                    p.style.arc_r = 22.;
                    // presa às duas faces: o vértice e as pernas se refazem a cada quadro
                    let plana = |point, dir| fenkra_core::pmi::PmiTarget { class: fenkra_core::pmi::TargetClass::FacePlana, point, dir, radius: 0., concave: false };
                    p.targets = vec![Some(plana([0., -20., 8.], [0., 0., 1.])), Some(plana([0., -35., 4.], [0., -1., 0.]))];
                }
                id
            }
            "chamada" | "chamada_cruza" => {
                let mut n = Pmi { id: d.alloc(), name: "Nota com três pontas".into(), kind: PmiKind::Note, anchors: vec![[-40., 0., 8.]], offset: [40., 0., 40.], text: "Rebarbar".into(), tol_plus: String::new(), tol_minus: String::new(), visible: true, plane: None, style: Default::default(), folder: String::new(), targets: vec![] };
                let rotulo = [0., 0., 48.];
                fenkra_core::pmi_leader::add_tip(&mut n, [40., 20., 8.], None);
                fenkra_core::pmi_leader::add_tip(&mut n, [40., -20., 8.], None);
                // limpa: os vértices ficam do lado da ponta; `chamada_cruza` leva o caminho por cima da outra ponta
                let vias = if modo == "chamada" { [[-30., 0., 30.], [-38., 0., 18.]] } else { [[30., 0., 30.], [-30., 0., 20.]] };
                fenkra_core::pmi_leader::add_vertex(&mut n, rotulo, 0, vias[0]);
                fenkra_core::pmi_leader::add_vertex(&mut n, rotulo, 0, vias[1]);
                let id = n.id;
                d.pmi.push(n);
                id
            }
            "perfil" => {
                // texto deitado num plano de normal X: visto de frente (ao longo de Y) fica de perfil
                let id = novo(d, PmiKind::Note, vec![[0., 0., 8.]], [0., 0., 30.], "Texto deitado", "Nota no plano");
                if let Some(p) = d.pmi.last_mut() {
                    p.plane = Some(([1., 0., 0.], [0., 1., 0.]));
                }
                id
            }
            _ => {
                novo(d, PmiKind::Diameter, vec![[0., 0., 8.], [10., 0., 8.]], [25., 25., 20.], "", "Diâmetro");
                novo(d, PmiKind::Note, vec![[30., 15., 8.]], [20., 20., 25.], "Face usinada", "Nota")
            }
        };
        // só o realce (sem painel lateral): as alças e o aviso aparecem sobre a anotação
        self.pmi_focus = Some(sel);
        self.started = true;
    }
}

impl App {
    /// Chamada de feature na montagem (`FENKRA_DEMO_ASM=1` com `FENKRA_DEMO_JANELA=pmi_feature_asm`): escolhe a
    /// parede de um furo no primeiro componente e passa pelo mesmo caminho do clique (`after_viewport`).
    pub(super) fn demo_feature_montagem(&mut self) -> bool {
        use crate::picking::Pick;
        // a vista só espia o cache da interface: carrega as peças dos corpos antes
        let n = self.asm.as_ref().and_then(|s| s.view.as_ref()).map_or(0, |r| r.bodies.len());
        for body in 0..n {
            crate::pmi_feat_asm::aquecer(self, body);
        }
        let Some(st) = self.asm.as_ref() else { return false };
        let Some(r) = st.view.as_ref() else { return false };
        let mut achada = None;
        'o: for (body, b) in r.bodies.iter().enumerate() {
            for face in 0..b.mesh.face_ranges.len() {
                let p = Pick::Face { body, face, owner: b.collector, point: [0.; 3], normal: [0.; 3] };
                let Some(t) = crate::pmi_ui::pick_target(r, &p) else { continue };
                if t.class != fenkra_core::pmi::TargetClass::FaceCilindrica || !t.concave {
                    continue;
                }
                let Some((comp, feat)) = crate::pmi_feat_asm::dono_da_face(st, body, face) else { continue };
                if crate::pmi_feat_asm::chamada(st, comp, feat, true).is_some_and(|c| c.hole_diameter.is_some()) {
                    // um ponto da parede do furo: do eixo, um raio para fora, perpendicular a ele
                    let d = t.dir;
                    let h = if d[0].abs() < 0.9 { [1., 0., 0.] } else { [0., 1., 0.] };
                    let mut v = [d[1] * h[2] - d[2] * h[1], d[2] * h[0] - d[0] * h[2], d[0] * h[1] - d[1] * h[0]];
                    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-9);
                    v = v.map(|x| x / l * t.radius);
                    let ponto = [0, 1, 2].map(|k| (t.point[k] + v[k]) as f32);
                    achada = Some(Pick::Face { body, face, owner: b.collector, point: ponto, normal: [0.; 3] });
                    break 'o;
                }
            }
        }
        let Some(p) = achada else { return false };
        self.picks = vec![p];
        self.pmi_pending = Some(15);
        crate::pmi_ui::after_viewport(self);
        self.picks.clear();
        self.asm.as_ref().is_some_and(|s| !s.asm.pmi.is_empty())
    }
}
