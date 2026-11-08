// SPDX-License-Identifier: LGPL-2.1-or-later
//! Aba entre dois perfis, lofted (B35): a chapa passa de um perfil fechado ao outro, como num funil
//! ou adaptador de duto.
//!
//! Os dois perfis são as superfícies de referência da chapa. O kernel faz o loft dos dois (facetado
//! se `ruled`, liso se não) e esvazia o sólido tirando as duas tampas, com a espessura da chapa. O
//! resultado é uma casca aberta nas duas pontas. No estado planificado a transição é aberta pela
//! triangulação (`chapa_perfis_plana`): a chapa plana fica no plano do primeiro perfil, com a
//! costura na geratriz do primeiro canto e uma linha de dobra em cada canto dos perfis.

use super::*;
use fenkra_core::SmLoftFlange;

impl Ctx<'_> {
    pub(super) fn run_loft_flange(&mut self, f: &Feature, lf: &SmLoftFlange) -> Result<Option<String>, String> {
        let k = |e: KernelError| e.0;
        let sp = self.sheet()?;
        let mut faces: Vec<Shape> = vec![];
        let mut centros: Vec<V3> = vec![];
        let mut avisos: Vec<String> = vec![];
        let mut perfis = vec![];
        for id in [lf.a, lf.b] {
            let (frame, sk) = &self.sketch(id)?;
            perfis.push((*frame, sk.loops.first().cloned()));
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
        if sp.flat {
            let aviso = self.loft_planificado(f, lf, &perfis, &sp)?;
            avisos.push(aviso);
        } else {
            self.apply(f.target, casca, BoolOp::Add, f.id)?;
        }
        Ok((!avisos.is_empty()).then(|| avisos.join("; ")))
    }

    /// A aba entre perfis planificada: o contorno desenvolvido na fibra neutra, extrudado na
    /// espessura no plano do primeiro perfil, e as linhas de dobra dos cantos (raio zero).
    fn loft_planificado(&mut self, f: &Feature, lf: &SmLoftFlange, perfis: &[(Frame, Option<fenkra_kernel::Loop2>)], sp: &SheetValues) -> Result<String, String> {
        use super::chapa_perfis_plana as pl;
        let k = |e: KernelError| e.0;
        let [(fa, Some(la)), (fb, Some(lb))] = perfis else {
            return Err("a aba entre perfis precisa de dois contornos fechados".into());
        };
        let (fa2, fb2) = (*fa, *fb);
        let (a, b) = pl::amostrar(la, lb, &|p| fa2.point(p[0], p[1]), &|p| fb2.point(p[0], p[1]))?;
        // os perfis são a face de fora (a chapa cresce para dentro); invertido, a de dentro. A fibra
        // neutra fica a K·t da face de dentro.
        let kf = sp.k_for(sp.r);
        let delta = if lf.flip { kf * sp.t } else { -(1. - kf) * sp.t };
        let d = pl::desenvolver(&a, &b, fa.normal, fb.normal, delta)?;
        // a chapa plana deitada no plano do primeiro perfil: a costura começa no primeiro ponto, e a
        // geratriz dele aponta para o lado de dentro do perfil
        let o = a.pontos[0];
        let n = geo3d::unit(fa.normal).ok_or("plano do perfil sem normal")?;
        let centro = a.pontos.iter().fold([0.; 3], |s, p| geo3d::add(s, geo3d::mul(*p, 1. / a.pontos.len() as f64)));
        let dentro = geo3d::sub(centro, o);
        let ey = geo3d::unit(geo3d::sub(dentro, geo3d::mul(n, geo3d::dot(dentro, n)))).ok_or("perfil degenerado")?;
        let ex = fenkra_kernel::cross(ey, n);
        let em3d = |p: [f64; 2], z: f64| geo3d::add(geo3d::add(o, geo3d::add(geo3d::mul(ex, p[0]), geo3d::mul(ey, p[1]))), geo3d::mul(n, z));
        let pts: Vec<V3> = d.contorno.iter().map(|p| em3d(*p, 0.)).collect();
        let placa = Shape::poly_prism(&pts, geo3d::mul(n, sp.t)).map_err(k)?;
        if placa.is_null() || placa.mass().volume <= 1e-6 {
            return Err("não consegui montar a chapa planificada da aba entre perfis".into());
        }
        self.apply(f.target, placa, BoolOp::Add, f.id)?;
        for x in &d.dobras {
            let linha = [em3d(x.a, sp.t / 2.), em3d(x.b, sp.t / 2.)];
            let ang = x.angulo;
            let bl = BendLine { start: linha, center: linha, end: linha, angle: ang, radius: 0., up: !x.para_fora, feature: f.id, allowance: 0., deduction: 2. * (ang.to_radians() / 2.).tan() * sp.t, eixo: None, curva: vec![] };
            self.cut_outlines.push(CutOutline { feature: f.id, folded: vec![], flat: vec![], dobra: Some(bl) });
        }
        Ok(format!("planificada por triangulação: {:.1} × {:.1} mm de borda, {} dobra(s) nos cantos, costura no primeiro canto", d.borda_a, d.borda_b, d.dobras.len()))
    }
}
