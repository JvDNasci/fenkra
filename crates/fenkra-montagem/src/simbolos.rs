// SPDX-License-Identifier: LGPL-2.1-or-later
//! Símbolos de restrição e junta no modelo (B245). Cada símbolo sai da geometria que a restrição
//! guarda, no sistema da peça, e passa pela posição de agora: por isso acompanha a peça sem cache
//! nenhum. Aqui não há desenho: a tela pergunta onde cada coisa fica e `arranjar` decide o que cabe,
//! para uma montagem grande não virar uma mancha.

use crate::kin::{medir_grau, Faixa, Grau, Joint, JointKind};
use crate::*;
use std::collections::HashMap;

/// O que a junta deixa a peça fazer, para desenhar a seta certa.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Movimento {
    Nenhum,
    Reta,
    Curva,
    RetaCurva,
    Esfera,
    Plano,
    Trajetoria,
}

impl Movimento {
    pub fn da_junta(k: &JointKind) -> Movimento {
        match k {
            JointKind::Revolute => Movimento::Curva,
            JointKind::Prismatic => Movimento::Reta,
            JointKind::Cylindrical | JointKind::Screw { .. } => Movimento::RetaCurva,
            JointKind::Spherical => Movimento::Esfera,
            JointKind::Planar => Movimento::Plano,
            JointKind::Rigid => Movimento::Nenhum,
            JointKind::PointCurve { .. } | JointKind::Cam { .. } => Movimento::Trajetoria,
        }
    }
}

/// Cota no modelo, em coordenadas do mundo.
#[derive(Clone, PartialEq, Debug)]
pub enum Medida {
    /// de um elemento ao outro; `sentido` é para onde o valor cresce, e `sinal` diz se o valor passa
    /// para o outro lado do zero (medida ao longo de uma normal) ou para nele (módulo)
    Distancia { de: V3, ate: V3, sentido: V3, valor: f64, sinal: bool },
    /// arco em volta de `centro`, de `u` até `v`
    Angulo { centro: V3, u: V3, v: V3, valor: f64 },
}

#[derive(Clone, PartialEq, Debug)]
pub struct Simbolo {
    pub id: Id,
    /// tipo legível; é a chave do ocultar por tipo
    pub tipo: &'static str,
    pub icone: &'static str,
    /// âncoras no mundo, uma em cada elemento
    pub a: V3,
    pub b: V3,
    pub da: Option<V3>,
    pub db: Option<V3>,
    pub comps: (Id, Id),
    pub medida: Option<Medida>,
    /// junta: origem, eixo e referência no mundo
    pub eixos: Option<(V3, V3, V3)>,
    pub movimento: Movimento,
    pub falhou: bool,
    pub suprimido: bool,
    /// o valor é número puro: dá para arrastar sem quebrar a fórmula
    pub editavel: bool,
}

impl Simbolo {
    /// Ponto onde o símbolo fica: entre as duas âncoras.
    pub fn centro(&self) -> V3 {
        [0, 1, 2].map(|i| (self.a[i] + self.b[i]) / 2.)
    }
}

/// Ponto do elemento, no sistema da peça: onde foi clicado, ou o mais perto disso no eixo.
pub fn ponto_local(r: &MateRef) -> V3 {
    match r.geom {
        Geom::Plane { p, .. } => r.pick.unwrap_or(p),
        Geom::Axis { p, d } => match r.pick {
            Some(q) => {
                let d = unit(d);
                let k = dot(sub(q, p), d);
                [p[0] + d[0] * k, p[1] + d[1] * k, p[2] + d[2] * k]
            }
            None => p,
        },
        Geom::Point(p) => p,
    }
}

fn direcao_local(g: &Geom) -> Option<V3> {
    match *g {
        Geom::Plane { n, .. } => Some(n),
        Geom::Axis { d, .. } => Some(d),
        Geom::Point(_) => None,
    }
}

fn numero(e: &str) -> Option<f64> {
    e.trim().replace(',', ".").parse().ok()
}

fn cota_de(m: &Mate, a: V3, b: V3, da: Option<V3>, db: Option<V3>, vars: &HashMap<String, f64>) -> Option<Medida> {
    match &m.kind {
        MateKind::Distance(e) => {
            let valor = expr::eval_str(e, vars).ok()?;
            // ao longo da normal a cota tem sinal; entre pontos e eixos é módulo
            let plano = |g: &Geom| matches!(g, Geom::Plane { .. });
            if plano(&m.a.geom) || plano(&m.b.geom) {
                let n = if plano(&m.a.geom) { da? } else { db? };
                let medida = dot(sub(b, a), n);
                let ate = [0, 1, 2].map(|i| a[i] + n[i] * medida);
                // o valor cresce para o lado que a cota positiva aponta
                let s = if m.lado_invertido { -1. } else { 1. };
                Some(Medida::Distancia { de: a, ate, sentido: n.map(|x| x * s), valor, sinal: true })
            } else {
                let d = sub(b, a);
                let d = match (m.a.geom, da) {
                    (Geom::Axis { .. }, Some(u)) => {
                        let k = dot(d, u);
                        [d[0] - u[0] * k, d[1] - u[1] * k, d[2] - u[2] * k]
                    }
                    _ => d,
                };
                let sentido = if norm(d) > 1e-9 { unit(d) } else { da.map(any_perp).unwrap_or([1., 0., 0.]) };
                Some(Medida::Distancia { de: a, ate: [0, 1, 2].map(|i| a[i] + d[i]), sentido, valor, sinal: false })
            }
        }
        MateKind::Angle(e) => {
            let valor = expr::eval_str(e, vars).ok()?;
            let (u, v) = (da?, db?);
            let v = if m.sentido == Sentido::Oposto { v.map(|x| -x) } else { v };
            Some(Medida::Angulo { centro: a, u, v, valor })
        }
        _ => None,
    }
}

fn da_restricao(a: &Assembly, m: &Mate, vars: &HashMap<String, f64>) -> Option<Simbolo> {
    let (pa, pb) = (a.lugar(m.a.comp)?, a.lugar(m.b.comp)?);
    let (wa, wb) = (pa.point(ponto_local(&m.a)), pb.point(ponto_local(&m.b)));
    let (da, db) = (direcao_local(&m.a.geom).map(|d| unit(pa.dir(d))), direcao_local(&m.b.geom).map(|d| unit(pb.dir(d))));
    let editavel = match &m.kind {
        MateKind::Distance(e) | MateKind::Angle(e) => numero(e).is_some(),
        _ => false,
    };
    Some(Simbolo {
        id: m.id,
        tipo: m.kind.label(),
        icone: m.kind.icon(),
        a: wa,
        b: wb,
        da,
        db,
        comps: (m.a.comp, m.b.comp),
        medida: cota_de(m, wa, wb, da, db, vars),
        eixos: None,
        movimento: Movimento::Nenhum,
        falhou: false,
        suprimido: m.suppressed,
        editavel,
    })
}

fn da_junta(a: &Assembly, j: &Joint) -> Option<Simbolo> {
    let (pa, pb) = (a.lugar(j.a.comp)?, a.lugar(j.b.comp)?);
    let (oa, za, xa) = j.a.frame.world(&pa);
    let (ob, zb, _) = j.b.frame.world(&pb);
    Some(Simbolo {
        id: j.id,
        tipo: j.kind.label(),
        icone: "⚙",
        a: oa,
        b: ob,
        da: Some(za),
        db: Some(zb),
        comps: (j.a.comp, j.b.comp),
        medida: None,
        eixos: Some((oa, za, xa)),
        movimento: Movimento::da_junta(&j.kind),
        falhou: false,
        suprimido: j.suppressed,
        editavel: false,
    })
}

/// Faixa de um grau de liberdade no 3D: um arco em volta de `eixo`, ou um trilho ao longo dele.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct MarcaLimite {
    pub grau: Grau,
    /// centro do arco, ou o ponto do trilho onde o grau vale zero
    pub origem: V3,
    pub eixo: V3,
    /// direção em que o arco vale zero, perpendicular ao eixo
    pub zero: V3,
    pub faixa: Faixa,
    pub atual: f64,
    /// trajetória no mundo: quando existe, as marcas andam sobre ela e não em linha reta
    pub curva: Vec<V3>,
    /// comprimento andado na curva onde o grau vale zero
    pub arco0: f64,
}

impl MarcaLimite {
    /// Ponto do mundo onde o grau vale `v`. O `raio` só conta no arco.
    pub fn ponto(&self, v: f64, raio: f64) -> V3 {
        if let Some(q) = crate::kin::ponto_no_arco(&self.curva, v + self.arco0) {
            return q;
        }
        if !self.grau.angular() {
            return [0, 1, 2].map(|i| self.origem[i] + self.eixo[i] * v);
        }
        if raio == 0. {
            return self.origem;
        }
        let d = rotvec(self.zero, self.eixo.map(|e| e * v.to_radians()));
        [0, 1, 2].map(|i| self.origem[i] + d[i] * raio)
    }
}

/// Marcas dos graus limitados da junta, nas posições de agora. A trajetória não entra: ela já é a
/// própria curva, e não um arco nem um trilho.
pub fn marcas_limite(a: &Assembly, j: &Joint) -> Vec<MarcaLimite> {
    let (Some(pa), Some(pb)) = (a.lugar(j.a.comp), a.lugar(j.b.comp)) else { return vec![] };
    let (oa, za, xa) = j.a.frame.world(&pa);
    let ya = cross(za, xa);
    let em = |base: V3, d: V3, k: f64| [0, 1, 2].map(|i| base[i] + d[i] * k);
    let lugar = |g: Grau| match g {
        Grau::Giro => Some((oa, za, rotvec(xa, za.map(|e| e * j.angle0.to_radians())))),
        Grau::Curso => Some((em(oa, za, j.len0), za, xa)),
        Grau::CursoX => Some((em(em(oa, xa, j.zero[0]), ya, j.zero[1]), xa, za)),
        Grau::CursoY => Some((em(em(oa, xa, j.zero[0]), ya, j.zero[1]), ya, za)),
        // tomba do eixo z em direção ao x
        Grau::Inclinacao => Some((oa, ya, za)),
        Grau::Trajeto => {
            let JointKind::PointCurve { curve } = &j.kind else { return None };
            let (origem, adiante) = (crate::kin::ponto_no_arco(curve, j.zero[2])?, crate::kin::ponto_no_arco(curve, j.zero[2] + 1.)?);
            Some((pa.point(origem), unit(pa.dir(sub(adiante, origem))), xa))
        }
    };
    let curva = |g: Grau| match &j.kind {
        JointKind::PointCurve { curve } if g == Grau::Trajeto => curve.iter().map(|q| pa.point(*q)).collect(),
        _ => vec![],
    };
    let graus = j.kind.graus().iter().filter(|g| j.limites.faixa(**g).ligada());
    graus.filter_map(|g| lugar(*g).map(|(origem, eixo, zero)| MarcaLimite { grau: *g, origem, eixo, zero, faixa: *j.limites.faixa(*g), atual: medir_grau(j, *g, &pa, &pb), curva: curva(*g), arco0: if *g == Grau::Trajeto { j.zero[2] } else { 0. } })).collect()
}

/// Símbolos da montagem nas posições de agora. Peça oculta leva junto o que só ela segura.
pub fn simbolos(a: &Assembly, falhas: &[Id]) -> Vec<Simbolo> {
    let vars = a.vars();
    let vista = |c: Id| a.node(c).is_none_or(|n| n.visible);
    let mut out: Vec<Simbolo> = a.mates.iter().filter(|m| vista(m.a.comp) || vista(m.b.comp)).filter_map(|m| da_restricao(a, m, &vars)).collect();
    out.extend(a.joints.iter().filter(|j| vista(j.a.comp) || vista(j.b.comp)).filter_map(|j| da_junta(a, j)));
    for s in &mut out {
        s.falhou = falhas.contains(&s.id);
    }
    out
}

/// O que aparece, depois de arrumar os símbolos na tela.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Arranjo {
    /// índices dos símbolos desenhados inteiros
    pub mostrar: Vec<usize>,
    /// (índice do símbolo visível, quantos ficaram atrás dele)
    pub grupos: Vec<(usize, usize)>,
}

/// Separa os símbolos em tela: dois nunca ficam a menos de `celula` pixels um do outro. Quem sobra
/// vira um contador no vizinho. Os destacados (escolhido ou sob o mouse) sempre aparecem. Usa uma
/// grade, então custa o mesmo por símbolo mesmo em milhares deles.
pub fn arranjar(pontos: &[[f32; 2]], destaque: impl Fn(usize) -> bool, celula: f32) -> Arranjo {
    let chave = |p: [f32; 2]| ((p[0] / celula).floor() as i32, (p[1] / celula).floor() as i32);
    let mut grade: HashMap<(i32, i32), Vec<usize>> = HashMap::new();
    let mut out = Arranjo::default();
    let mut atras: HashMap<usize, usize> = HashMap::new();
    let ordem = (0..pontos.len()).filter(|i| destaque(*i)).chain((0..pontos.len()).filter(|i| !destaque(*i)));
    for i in ordem {
        let p = pontos[i];
        let (cx, cy) = chave(p);
        let vizinho = (-1..=1).flat_map(|dx| (-1..=1).map(move |dy| (cx + dx, cy + dy))).filter_map(|c| grade.get(&c)).flatten().copied().find(|j| {
            let q = pontos[*j];
            (p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2) < celula * celula
        });
        match vizinho {
            Some(j) if !destaque(i) => *atras.entry(j).or_insert(0) += 1,
            _ => {
                grade.entry((cx, cy)).or_default().push(i);
                out.mostrar.push(i);
            }
        }
    }
    out.grupos = atras.into_iter().collect();
    out.grupos.sort();
    out
}

impl Assembly {
    /// Troca o valor de uma Distância ou Ângulo. Fórmula não é trocada por arrasto: a cota tem dono.
    pub fn definir_medida(&mut self, id: Id, valor: f64) -> bool {
        let Some(m) = self.mates.iter_mut().find(|m| m.id == id) else { return false };
        match &mut m.kind {
            MateKind::Distance(e) | MateKind::Angle(e) if numero(e).is_some() => {
                *e = valor_cota(valor);
                true
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kin::{JFrame, JRef};

    fn par() -> (Assembly, Id, Id) {
        let mut a = Assembly::new("par");
        let f = a.add_component("base", NodeKind::Part { path: "a.fkp".into() }, Placement::default(), None);
        a.node_mut(f).unwrap().fixed = true;
        let m = a.add_component("tampa", NodeKind::Part { path: "b.fkp".into() }, Placement { origin: [0., 0., 30.], ..Default::default() }, None);
        (a, f, m)
    }

    fn plano(comp: Id, z: f64, n: f64) -> MateRef {
        MateRef { comp, geom: Geom::Plane { p: [0., 0., z], n: [0., 0., n] }, pick: Some([1., 2., z]) }
    }

    #[test]
    fn simbolo_acompanha_a_peca() {
        let (mut a, f, m) = par();
        a.add_mate(MateKind::Coincident { flip: false }, plano(f, 10., 1.), plano(m, 0., 1.));
        let s0 = simbolos(&a, &[]);
        assert_eq!(s0[0].b, [1., 2., 30.]);
        a.node_mut(m).unwrap().placement.origin = [5., 0., 30.];
        let s1 = simbolos(&a, &[]);
        assert_eq!(s1[0].b, [6., 2., 30.], "âncora presa à peça");
        assert_eq!(s1[0].a, [1., 2., 10.], "a outra âncora fica");
    }

    #[test]
    fn distancia_vira_cota_com_lado() {
        let (mut a, f, m) = par();
        let id = a.add_mate(MateKind::Distance("20".into()), plano(f, 10., 1.), plano(m, 0., -1.));
        let s = simbolos(&a, &[]);
        let Some(Medida::Distancia { de, ate, sentido, valor, sinal }) = s[0].medida.clone() else { panic!() };
        assert_eq!((de, ate, valor, sinal), ([1., 2., 10.], [1., 2., 30.], 20., true));
        assert_eq!(sentido, [0., 0., 1.]);
        assert!(s[0].editavel);
        a.inverter_lado(id);
        let Some(Medida::Distancia { sentido, .. }) = simbolos(&a, &[])[0].medida.clone() else { panic!() };
        assert_eq!(sentido, [0., 0., -1.], "lado invertido: o valor cresce para baixo");
    }

    #[test]
    fn distancia_entre_pontos_e_modulo() {
        let (mut a, f, m) = par();
        a.add_mate(MateKind::Distance("5".into()), MateRef { comp: f, geom: Geom::Point([0.; 3]), pick: None }, MateRef { comp: m, geom: Geom::Point([3., 0., -26.]), pick: None });
        let Some(Medida::Distancia { sinal, sentido, ate, .. }) = simbolos(&a, &[])[0].medida.clone() else { panic!() };
        assert!(!sinal);
        assert_eq!(ate, [3., 0., 4.]);
        assert!((sentido[0] - 0.6).abs() < 1e-9);
    }

    #[test]
    fn angulo_vira_arco_entre_as_direcoes() {
        let (mut a, f, m) = par();
        let (ra, rb) = (MateRef { comp: f, geom: Geom::Plane { p: [0.; 3], n: [1., 0., 0.] }, pick: None }, MateRef { comp: m, geom: Geom::Plane { p: [0.; 3], n: [0., 1., 0.] }, pick: None });
        a.add_mate(MateKind::Angle("90".into()), ra, rb);
        let Some(Medida::Angulo { u, v, valor, .. }) = simbolos(&a, &[])[0].medida.clone() else { panic!() };
        assert_eq!((u, v, valor), ([1., 0., 0.], [0., 1., 0.], 90.));
    }

    #[test]
    fn junta_mostra_eixos_e_movimento() {
        let (mut a, f, m) = par();
        let fr = JFrame::from_axis([1., 0., 0.], [0., 0., 1.]);
        a.add_joint(JointKind::Revolute, JRef { comp: f, frame: fr }, JRef { comp: m, frame: fr });
        a.add_joint(JointKind::Prismatic, JRef { comp: f, frame: fr }, JRef { comp: m, frame: fr });
        let s = simbolos(&a, &[]);
        assert_eq!(s[0].movimento, Movimento::Curva);
        assert_eq!(s[1].movimento, Movimento::Reta);
        let (o, z, _) = s[0].eixos.unwrap();
        assert_eq!((o, z), ([1., 0., 0.], [0., 0., 1.]));
    }

    #[test]
    fn falha_e_suprimida_aparecem_no_simbolo() {
        let (mut a, f, m) = par();
        let id = a.add_mate(MateKind::Contact, plano(f, 10., 1.), plano(m, 0., -1.));
        a.mates[0].suppressed = true;
        let s = simbolos(&a, &[id]);
        assert!(s[0].falhou && s[0].suprimido);
    }

    #[test]
    fn peca_oculta_leva_o_simbolo_so_com_as_duas() {
        let (mut a, f, m) = par();
        a.add_mate(MateKind::Contact, plano(f, 10., 1.), plano(m, 0., -1.));
        a.node_mut(m).unwrap().visible = false;
        assert_eq!(simbolos(&a, &[]).len(), 1);
        a.node_mut(f).unwrap().visible = false;
        assert!(simbolos(&a, &[]).is_empty());
    }

    #[test]
    fn duzentas_restricoes_ficam_legiveis() {
        // 200 símbolos num canto de 300 x 300 px: nada sobreposto, ninguém perdido
        let pontos: Vec<[f32; 2]> = (0..200).map(|i| [(i * 37 % 300) as f32, (i * 91 % 300) as f32]).collect();
        let r = arranjar(&pontos, |_| false, 30.);
        for (k, i) in r.mostrar.iter().enumerate() {
            for j in &r.mostrar[k + 1..] {
                let (p, q) = (pontos[*i], pontos[*j]);
                assert!((p[0] - q[0]).hypot(p[1] - q[1]) >= 30., "sobrepostos");
            }
        }
        assert!(r.mostrar.len() < 120, "{} símbolos numa área de 100 células", r.mostrar.len());
        let contados: usize = r.mostrar.len() + r.grupos.iter().map(|g| g.1).sum::<usize>();
        assert_eq!(contados, 200, "cada símbolo aparece ou conta no vizinho");
        assert!(r.grupos.iter().all(|g| r.mostrar.contains(&g.0)));
    }

    #[test]
    fn destacado_sempre_aparece() {
        let pontos = vec![[10., 10.], [11., 10.], [12., 10.]];
        assert_eq!(arranjar(&pontos, |_| false, 30.).mostrar, vec![0]);
        let r = arranjar(&pontos, |i| i == 2, 30.);
        assert!(r.mostrar.contains(&2));
        assert_eq!(r.grupos, vec![(2, 2)]);
    }

    #[test]
    fn arrasto_troca_numero_e_respeita_formula() {
        let (mut a, f, m) = par();
        let id = a.add_mate(MateKind::Distance("20".into()), plano(f, 10., 1.), plano(m, 0., -1.));
        assert!(a.definir_medida(id, 12.5));
        assert_eq!(a.mates[0].kind, MateKind::Distance("12.5".into()));
        a.mates[0].kind = MateKind::Distance("2*L".into());
        assert!(!a.definir_medida(id, 3.));
        assert!(!simbolos(&a, &[]).is_empty());
    }
}
