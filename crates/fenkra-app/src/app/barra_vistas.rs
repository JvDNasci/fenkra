// SPDX-License-Identifier: LGPL-2.1-or-later
//! B506: ações novas da barra de vistas (a ordem dos botões está em `toolbar::ORDEM_VISTAS`): zoom na
//! seleção e a isométrica cíclica. Enquadrar tudo, vista normal, estilos, ocultar e mostrar, trocar o
//! espaço visível e o material reaproveitam o que já existia (ajustar, `align_to_selection`, o
//! controle visual e a barra de propriedades gráficas).

use super::*;
use crate::picking::Pick;
use std::sync::atomic::{AtomicU8, Ordering};

/// As oito direções isométricas (giro e inclinação da câmera), começando pela isométrica de sempre.
pub const ISOMETRICAS: [(f32, f32); 8] = [(-0.785, 0.6), (0.785, 0.6), (2.356, 0.6), (-2.356, 0.6), (-0.785, -0.6), (0.785, -0.6), (2.356, -0.6), (-2.356, -0.6)];

/// Última isométrica usada.
static ISO: AtomicU8 = AtomicU8::new(0);

/// Próxima isométrica: com a câmera ainda na isométrica `atual`, a seguinte; senão, a primeira.
pub fn proxima_iso(camera: (f32, f32), atual: usize) -> usize {
    let perto = |a: f32, b: f32| {
        let d = (a - b).rem_euclid(std::f32::consts::TAU);
        d.min(std::f32::consts::TAU - d) < 0.05
    };
    let (y, p) = ISOMETRICAS[atual % 8];
    if perto(camera.0, y) && (camera.1 - p).abs() < 0.05 { (atual + 1) % 8 } else { 0 }
}

/// Caixa que contém os pontos (com um tamanho mínimo, para um ponto só não virar zoom infinito).
pub fn caixa_dos_pontos(pts: &[Vec3]) -> Option<(Vec3, Vec3)> {
    let first = *pts.first()?;
    let (mut lo, mut hi) = (first, first);
    for p in pts {
        lo = lo.min(*p);
        hi = hi.max(*p);
    }
    let falta = (Vec3::splat(10.) - (hi - lo)).max(Vec3::ZERO) / 2.;
    Some((lo - falta, hi + falta))
}

impl App {
    /// Pontos do que está selecionado: faces inteiras, arestas, vértices e os corpos da árvore.
    fn pontos_da_selecao(&self) -> Vec<Vec3> {
        let mut pts: Vec<Vec3> = vec![];
        let bodies = self.result.as_ref().map(|r| &r.bodies[..]).unwrap_or(&[]);
        let do_corpo = |pts: &mut Vec<Vec3>, col: fenkra_core::Id| {
            for b in bodies.iter().filter(|b| b.collector == col) {
                pts.extend(b.mesh.positions.chunks(3).map(|p| Vec3::new(p[0], p[1], p[2])));
            }
        };
        for p in &self.picks {
            match p {
                Pick::Face { body, face, point, .. } => match bodies.get(*body) {
                    Some(b) => {
                        let m = &b.mesh;
                        let ini = m.face_ranges.get(*face).copied().unwrap_or(0) as usize;
                        let fim = m.face_ranges.get(face + 1).map(|x| *x as usize).unwrap_or(m.indices.len());
                        for &i in m.indices.get(ini..fim).unwrap_or(&[]) {
                            let k = i as usize * 3;
                            pts.push(Vec3::new(m.positions[k], m.positions[k + 1], m.positions[k + 2]));
                        }
                    }
                    None => pts.push(Vec3::from(*point)),
                },
                Pick::Edge { mid, .. } => pts.push(Vec3::from(*mid)),
                Pick::Vertex { pos, .. } | Pick::SkPoint { pos, .. } | Pick::CurveVtx { pos, .. } => pts.push(Vec3::from(*pos)),
                Pick::SkCurve { at, .. } => pts.push(Vec3::from(*at)),
                _ => {}
            }
        }
        let d = &self.open.doc;
        match self.sel {
            Sel::Collector(id) => do_corpo(&mut pts, id),
            Sel::Feature(id) => {
                if let Some(f) = d.feature(id) {
                    do_corpo(&mut pts, f.target)
                }
            }
            _ => {}
        }
        pts
    }

    /// Zoom na seleção; sem seleção, enquadra tudo e avisa.
    pub(crate) fn zoom_na_selecao(&mut self) {
        let Some(caixa) = caixa_dos_pontos(&self.pontos_da_selecao()) else {
            self.fit_pending = true;
            self.toast(tr("Nada selecionado: enquadrei o modelo inteiro."), false);
            return;
        };
        if let Some(vp) = self.viewport.as_mut() {
            let antes = vp.camera;
            vp.camera.fit(caixa);
            vp.animate_from(antes);
        }
    }

    /// Isométrica; clique repetido gira entre as oito direções e o aviso diz qual está ativa.
    pub(crate) fn iso_ciclica(&mut self) {
        let Some(vp) = self.viewport.as_mut() else { return };
        let k = proxima_iso((vp.camera.yaw, vp.camera.pitch), ISO.load(Ordering::Relaxed) as usize);
        ISO.store(k as u8, Ordering::Relaxed);
        let antes = vp.camera;
        let (y, p) = ISOMETRICAS[k];
        vp.camera.set_view(y, p);
        vp.animate_from(antes);
        self.toast(format!("{} {} {} 8", tr("Isométrica"), k + 1, tr("de")), false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Group;
    use crate::toolbar::Entry;

    /// B506 (R4.2): a barra de vistas segue a ordem da referência.
    #[test]
    fn barra_de_vistas_na_ordem() {
        let ids: Vec<&str> = toolbar::entries(Group::Vista)
            .into_iter()
            .map(|e| match e {
                Entry::One(i) => i.icon,
                Entry::Family(k, _) => k,
            })
            .take(toolbar::ORDEM_VISTAS.len())
            .collect();
        assert_eq!(ids, toolbar::ORDEM_VISTAS);
        // as sete vistas padrão estão no menu, e a isométrica é a cíclica
        let fam = toolbar::entries(Group::Vista).into_iter().find_map(|e| if let Entry::Family("vistas", v) = e { Some(v) } else { None }).unwrap();
        assert_eq!(fam.len(), 7);
        assert_eq!(fam[0].act, Act::IsoCiclica);
        for icone in toolbar::ORDEM_VISTAS.iter().chain(&["vista_tras", "vista_esquerda", "vista_base"]) {
            if *icone != "vistas" {
                assert!(crate::icons::existe(icone), "{icone} sem ícone novo");
            }
        }
    }

    /// B506: a isométrica gira pelas oito direções e volta à primeira; fora dela, começa da primeira.
    #[test]
    fn isometrica_ciclica() {
        let mut k = 0;
        let mut vistas = vec![k];
        for _ in 0..8 {
            k = proxima_iso(ISOMETRICAS[k], k);
            vistas.push(k);
        }
        assert_eq!(vistas, [0, 1, 2, 3, 4, 5, 6, 7, 0]);
        assert_eq!(proxima_iso((0., 0.), 3), 0, "câmera fora da isométrica volta à primeira");
    }

    #[test]
    fn caixa_minima_para_um_ponto() {
        let (lo, hi) = caixa_dos_pontos(&[Vec3::new(5., 5., 5.)]).unwrap();
        assert!((hi - lo).min_element() >= 10. - 1e-4);
        assert!(caixa_dos_pontos(&[]).is_none());
    }
}
