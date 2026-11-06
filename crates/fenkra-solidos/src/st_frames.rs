// SPDX-License-Identifier: LGPL-2.1-or-later
//! Estrutural: referenciais dos membros para as alças do 3D (B253). Só geometria, sem sólidos:
//! a interface chama a cada quadro enquanto o comando Membro está aberto.

use super::*;

/// Referencial de uma linha de membro: onde ela começa e termina, os eixos da seção e os valores
/// que as alças mexem (já avaliados).
#[derive(Clone, Debug, PartialEq)]
pub struct MemberFrame {
    /// índice da linha dentro do comando (a mesma numeração da lista de corte)
    pub line: usize,
    pub start: V3,
    pub end: V3,
    /// eixo do membro
    pub z: V3,
    /// eixo X da seção, já com a rotação do comando
    pub x: V3,
    /// eixo X da seção com rotação 0 (de onde o arco de rotação mede o ângulo)
    pub x0: V3,
    pub y: V3,
    pub w: f64,
    pub h: f64,
    /// prolongamento pedido no início e no fim
    pub ext: (f64, f64),
    /// deslocamento X/Y da seção em relação ao ponto de ancoragem
    pub off: [f64; 2],
    /// rotação da seção em graus
    pub angle: f64,
    pub straight: bool,
}

/// Referenciais das linhas do membro `id` (vazio se o comando não é um membro ou não resolve).
pub fn member_frames(doc: &Document, id: Id) -> Vec<MemberFrame> {
    let Some(FeatureKind::StructMember(m)) = doc.feature(id).map(|f| &f.kind) else { return vec![] };
    let Ok(vars) = doc.eval_params() else { return vec![] };
    let cx = Ctx {
        doc,
        vars,
        qualidade: Default::default(),
        bodies: HashMap::new(),
        sketches: HashMap::new(),
        status: HashMap::new(),
        tools: HashMap::new(),
        memo: RefCell::new(HashMap::new()),
        notes: Default::default(),
        body_sig: HashMap::new(),
        last_sheet: None,
        sheet_maps: vec![],
        cut_outlines: vec![],
        malhas: HashMap::new(),
        roscas: vec![],
        relatorios_forma: vec![],
        formas_resolvidas: vec![],
        steps: HashMap::new(),
        miter_next: false,
        promoted: RefCell::new(None),
    };
    let Ok(geo) = cx.members(m) else { return vec![] };
    let num = |e: &str| ev(e, &cx.vars, "").unwrap_or(0.);
    let off = [num(&m.off_x), num(&m.off_y)];
    let angle = num(&m.angle);
    geo.into_iter()
        .enumerate()
        .map(|(line, g)| MemberFrame { line, start: g.start, end: g.end, z: g.z, x: g.x, x0: g.x0, y: g.y, w: g.w, h: g.h, ext: g.ext, off, angle, straight: g.straight })
        .collect()
}

/// Rotação (graus, em (-90, 90]) que deixa o eixo X da seção paralelo à aresta de direção `edge`,
/// vista de frente para o eixo do membro `z`. `x0` é o eixo X com rotação 0. A aresta paralela ao
/// membro não define lado (devolve `None`). O sentido da aresta não importa: a seção é alinhada
/// pela menor volta.
pub fn align_angle(z: V3, x0: V3, edge: V3) -> Option<f64> {
    let proj = geo3d::sub(edge, geo3d::mul(z, geo3d::dot(edge, z)));
    let u = geo3d::unit(proj)?;
    // rotate(x0, z, a) gira a seção no sentido que o comando usa: mede o ângulo no mesmo sentido
    let c = geo3d::dot(x0, u);
    let s = geo3d::dot(fenkra_kernel::cross(x0, u), z);
    let mut a = s.atan2(c).to_degrees();
    if a > 90. {
        a -= 180.;
    } else if a <= -90. {
        a += 180.;
    }
    Some(a)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn linha(d: &mut Document, a: [f64; 3], b: [f64; 3]) -> CurveRef {
        CurveRef::Ref(d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed(a), b: PointRef::Fixed(b), start: "0".into(), end: "0".into() }))
    }

    fn membro(paths: Vec<CurveRef>, angle: &str, off: [&str; 2], ext: [&str; 2]) -> StructMember {
        let s = |x: &str| x.to_string();
        StructMember {
            paths,
            family: s("Tubo quadrado"),
            designation: s("40x3"),
            kind: SectionKind::TuboQuadrado,
            dims: vec![s("40"), s("3")],
            anchor: 0,
            angle: s(angle),
            off_x: s(off[0]),
            off_y: s(off[1]),
            ext1: s(ext[0]),
            ext2: s(ext[1]),
            corner: CornerMode::Nenhum,
            gap: s("0"),
            joints: vec![],
            material: String::new(),
            mirror: false,
        }
    }

    #[test]
    fn referencial_traz_eixos_e_valores_avaliados() {
        let mut d = Document::new("t");
        d.add_param("L", "30", Unit::Comprimento).unwrap();
        let l = linha(&mut d, [0., 0., 0.], [0., 0., 200.]);
        let id = d.add_feature(FeatureKind::StructMember(membro(vec![l], "90", ["L/2", "0"], ["5", "0"])));
        let fr = member_frames(&d, id);
        assert_eq!(fr.len(), 1);
        let f = &fr[0];
        assert_eq!((f.off, f.angle, f.ext), ([15., 0.], 90., (5., 0.)));
        assert!(geo3d::dist(f.z, [0., 0., 1.]) < 1e-9);
        // com rotação de 90° o eixo X gira um quarto de volta em relação ao de rotação 0
        assert!(geo3d::dot(f.x, f.x0).abs() < 1e-9, "x {:?} x0 {:?}", f.x, f.x0);
        assert!(member_frames(&d, Id(u64::MAX)).is_empty());
    }

    #[test]
    fn alinhar_a_aresta_acha_a_menor_volta() {
        let z = [0., 0., 1.];
        let x0 = [1., 0., 0.];
        // aresta a 30° no plano da seção, com componente ao longo do membro que não conta
        let e = [30f64.to_radians().cos(), 30f64.to_radians().sin(), 7.];
        let a = align_angle(z, x0, e).unwrap();
        assert!((a - 30.).abs() < 1e-9, "{a}");
        // sentido oposto da mesma aresta dá o mesmo ângulo
        let b = align_angle(z, x0, geo3d::mul(e, -1.)).unwrap();
        assert!((a - b).abs() < 1e-9);
        // aresta a 120° equivale a -60° (menor volta, em (-90, 90])
        let c = align_angle(z, x0, [120f64.to_radians().cos(), 120f64.to_radians().sin(), 0.]).unwrap();
        assert!((c + 60.).abs() < 1e-9, "{c}");
        // paralela ao membro: sem lado
        assert!(align_angle(z, x0, [0., 0., 5.]).is_none());
    }

    #[test]
    fn rotacao_alinhada_bate_com_o_giro_do_comando() {
        // aplicar o ângulo devolvido ao comando deixa o X da seção paralelo à aresta
        let mut d = Document::new("t");
        let l = linha(&mut d, [0., 0., 0.], [0., 0., 200.]);
        let id = d.add_feature(FeatureKind::StructMember(membro(vec![l], "0", ["0", "0"], ["0", "0"])));
        let f0 = member_frames(&d, id).remove(0);
        let aresta = [1., 1., 0.];
        let a = align_angle(f0.z, f0.x0, aresta).unwrap();
        if let Some(FeatureKind::StructMember(m)) = d.feature_mut(id).map(|f| &mut f.kind) {
            m.angle = format!("{a}");
        }
        let f1 = member_frames(&d, id).remove(0);
        let u = geo3d::unit(aresta).unwrap();
        assert!(geo3d::dot(f1.x, u).abs() > 1. - 1e-9, "x {:?} aresta {:?}", f1.x, u);
    }
}
