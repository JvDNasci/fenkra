// SPDX-License-Identifier: LGPL-2.1-or-later
//! Aba entre dois perfis, lofted (B35): a chapa passa de um perfil fechado ao outro, como num funil
//! ou adaptador de duto.
//!
//! Os dois perfis são as superfícies de referência da chapa. O kernel faz o loft dos dois (facetado
//! se `ruled`, liso se não) e esvazia o sólido tirando as duas tampas, com a espessura da chapa. O
//! resultado é uma casca aberta nas duas pontas. A planificação desse tipo de aba não é exata
//! (a transição facetada ou lisa precisa de um desenvolvimento próprio): no estado planificado o
//! comando avisa e mostra a casca dobrada.

use super::*;
use fenkra_core::SmLoftFlange;

impl Ctx<'_> {
    pub(super) fn run_loft_flange(&mut self, f: &Feature, lf: &SmLoftFlange) -> Result<Option<String>, String> {
        let k = |e: KernelError| e.0;
        let sp = self.sheet()?;
        let mut faces: Vec<Shape> = vec![];
        let mut centros: Vec<V3> = vec![];
        let mut avisos: Vec<String> = vec![];
        for id in [lf.a, lf.b] {
            let (frame, sk) = &self.sketch(id)?;
            let Some(laco) = sk.loops.first() else {
                return Err(format!("o perfil '{}' precisa ser um contorno fechado: a aba entre perfis liga dois contornos", self.doc.name_of(id)));
            };
            if sk.loops.len() > 1 {
                avisos.push(format!("o perfil '{}' tem {} contornos e só o primeiro entra", self.doc.name_of(id), sk.loops.len()));
            }
            let face = Shape::face(frame, std::slice::from_ref(laco)).map_err(k)?;
            centros.push(face.mass().center);
            faces.push(face);
        }
        if geo3d::dist(centros[0], centros[1]) < 1e-6 {
            return Err("os dois perfis estão no mesmo lugar: afaste um do outro para haver transição".into());
        }
        let solido = faces[0].compound(&faces[1]).loft(lf.ruled, false, true).map_err(k)?;
        if solido.is_null() {
            return Err("o kernel não conseguiu ligar os dois perfis: confira se têm o mesmo número de vértices (aba facetada) e não se cruzam".into());
        }
        let tags = vec![0; solido.face_count() as usize];
        // a chapa cresce para dentro dos perfis (eles são a face de fora); invertido, para fora
        let (casca, _) = solido.shell_tagged(&centros, sp.t, lf.flip, &tags).map_err(k)?;
        let vol = casca.mass().volume;
        if casca.is_null() || vol <= 1e-6 {
            return Err(format!("a espessura de {:.2} mm não cabe na transição entre os perfis", sp.t));
        }
        self.apply(f.target, casca, BoolOp::Add, f.id)?;
        if sp.flat {
            avisos.push("a aba entre perfis não tem planificação exata: aparece dobrada".into());
        }
        Ok((!avisos.is_empty()).then(|| avisos.join("; ")))
    }
}
