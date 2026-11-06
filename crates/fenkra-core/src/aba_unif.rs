// SPDX-License-Identifier: LGPL-2.1-or-later
//! Comando de aba unificado (B31): os cinco jeitos de fazer uma aba (aresta, laço, aresta curva,
//! perfil e superfície) são modos de um comando só. Trocar de modo reconstrói a feature no modo
//! novo e leva junto o que vale para todos: comprimento, ângulo, raio e direção. O que é próprio de
//! um modo (recuos, extensão lateral, sketch do perfil, suporte) recomeça do padrão dele, e o que
//! o suporte já tinha escolhido (a aresta) é aproveitado quando o modo novo também usa aresta.

use crate::doc::*;

/// Os modos do comando de aba, na ordem das miniaturas.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ModoAba {
    Aresta,
    Laco,
    Curva,
    Perfil,
    Superficie,
}

pub const MODOS_ABA: [ModoAba; 5] = [ModoAba::Aresta, ModoAba::Laco, ModoAba::Curva, ModoAba::Perfil, ModoAba::Superficie];

impl ModoAba {
    pub fn indice(self) -> usize {
        MODOS_ABA.iter().position(|m| *m == self).unwrap_or(0)
    }
}

/// O modo de uma feature de aba; `None` para as outras.
pub fn modo_de(k: &FeatureKind) -> Option<ModoAba> {
    Some(match k {
        FeatureKind::SmFlange(_) => ModoAba::Aresta,
        FeatureKind::SmFlangeLoop(_) => ModoAba::Laco,
        FeatureKind::SmCurveFlange(_) => ModoAba::Curva,
        FeatureKind::SmProfileFlange(_) => ModoAba::Perfil,
        FeatureKind::SmSurfFlange(_) => ModoAba::Superficie,
        _ => return None,
    })
}

/// O que todos os modos compartilham.
struct Comum {
    comprimento: String,
    angulo: String,
    raio: Option<String>,
    inverter: bool,
    /// aresta que o suporte já tinha (um ponto), quando o modo antigo usava aresta
    aresta: Option<[f64; 3]>,
}

fn comum_de(k: &FeatureKind) -> Option<Comum> {
    let angulo = |a: &str| if a.trim().is_empty() { "90".to_string() } else { a.to_string() };
    let ponto = |p: [f64; 3]| (p != [0.; 3]).then_some(p);
    Some(match k {
        FeatureKind::SmFlange(f) => Comum { comprimento: f.length.clone(), angulo: angulo(&f.angle), raio: f.radius.clone(), inverter: f.flip, aresta: ponto(f.edge) },
        FeatureKind::SmFlangeLoop(l) => Comum { comprimento: l.length.clone(), angulo: angulo(&l.angle), raio: l.radius.clone(), inverter: l.flip, aresta: l.edges.first().copied().and_then(ponto) },
        FeatureKind::SmCurveFlange(c) => Comum { comprimento: c.length.clone(), angulo: angulo(&c.angle), raio: c.radius.clone(), inverter: c.flip, aresta: c.edge.get(c.edge.len() / 2).copied().and_then(ponto) },
        // o perfil puxa para os dois lados do plano: o comprimento do lado 2 é o que o usuário vê como "o da aba"
        FeatureKind::SmProfileFlange(p) => Comum { comprimento: p.l2.clone(), angulo: "90".into(), raio: p.radius.clone(), inverter: p.flip, aresta: None },
        FeatureKind::SmSurfFlange(s) => Comum { comprimento: s.length.clone(), angulo: angulo(&s.angle), raio: s.radius.clone(), inverter: s.flip, aresta: ponto(s.face) },
        _ => return None,
    })
}

/// A feature de aba reconstruída no modo `novo`. Devolve `None` se `k` não é uma aba. Se o modo é
/// o mesmo, devolve a própria feature sem mexer.
pub fn converter_aba(k: &FeatureKind, novo: ModoAba) -> Option<FeatureKind> {
    let atual = modo_de(k)?;
    if atual == novo {
        return Some(k.clone());
    }
    let c = comum_de(k)?;
    let aresta = c.aresta.unwrap_or([0.; 3]);
    Some(match novo {
        ModoAba::Aresta => FeatureKind::SmFlange(SmFlange {
            edge: aresta,
            length: c.comprimento,
            angle: c.angulo,
            radius: c.raio,
            off1: "0".into(),
            off2: "0".into(),
            flip: c.inverter,
            relief: None,
            relief_kind: 0,
            side: 0,
            miter: false,
            min_gap: true,
            edge_key: None,
        }),
        ModoAba::Laco => FeatureKind::SmFlangeLoop(SmFlangeLoop {
            // com uma aresta só, o laço da face inteira é o padrão do modo
            loop_all: true,
            edges: vec![aresta],
            edge_keys: vec![],
            extent: ExtensaoAba::Inteira,
            invert: false,
            length: c.comprimento,
            angle: c.angulo,
            radius: c.raio,
            flip: c.inverter,
            side: 0,
            miter: true,
            relief: None,
            relief_kind: 0,
        }),
        ModoAba::Curva => FeatureKind::SmCurveFlange(SmCurveFlange { edge: if aresta == [0.; 3] { vec![] } else { vec![aresta] }, length: c.comprimento, flip: c.inverter, angle: c.angulo, radius: c.raio, side: 0, edge_key: None }),
        ModoAba::Perfil => FeatureKind::SmProfileFlange(SmProfileFlange { sketch: Id(0), l1: "0".into(), l2: c.comprimento, radius: c.raio, flip: c.inverter }),
        ModoAba::Superficie => FeatureKind::SmSurfFlange(SmSurfFlange {
            support: SurfRef::Plane(PlaneRef::Origin(OriginPlane::YZ)),
            face: aresta,
            length: c.comprimento,
            radius: c.raio,
            flip: c.inverter,
            keep_base: false,
            mode: 0,
            angle: c.angulo,
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn aba() -> FeatureKind {
        FeatureKind::SmFlange(SmFlange { edge: [100., 25., 2.], length: "30".into(), angle: "75".into(), radius: Some("1.5*espessura".into()), off1: "5".into(), off2: "6".into(), flip: true, relief: None, relief_kind: 0, side: 0, miter: false, min_gap: true, edge_key: None })
    }

    #[test]
    fn cada_aba_tem_um_modo_e_o_resto_nao() {
        assert_eq!(modo_de(&aba()), Some(ModoAba::Aresta));
        assert_eq!(modo_de(&FeatureKind::SmCornerRelief(SmCornerRelief { at: [0.; 3], size: "4".into(), kind: 0 })), None);
        assert!(converter_aba(&FeatureKind::SmCornerRelief(SmCornerRelief { at: [0.; 3], size: "4".into(), kind: 0 }), ModoAba::Laco).is_none());
    }

    #[test]
    fn trocar_de_modo_leva_comprimento_angulo_raio_e_direcao() {
        for m in MODOS_ABA {
            let k = converter_aba(&aba(), m).unwrap();
            assert_eq!(modo_de(&k), Some(m));
            let c = comum_de(&k).unwrap();
            assert_eq!(c.comprimento, "30", "{m:?}");
            assert_eq!(c.raio.as_deref(), Some("1.5*espessura"), "{m:?}: o raio da regra não pode virar número fixo");
            assert!(c.inverter, "{m:?}: a direção invertida se perde");
            if m != ModoAba::Perfil {
                assert_eq!(c.angulo, "75", "{m:?}");
            }
        }
    }

    #[test]
    fn a_aresta_do_suporte_passa_entre_os_modos_que_usam_aresta() {
        let laco = converter_aba(&aba(), ModoAba::Laco).unwrap();
        let FeatureKind::SmFlangeLoop(l) = &laco else { panic!() };
        assert_eq!(l.edges, vec![[100., 25., 2.]]);
        assert!(l.loop_all);
        let volta = converter_aba(&laco, ModoAba::Aresta).unwrap();
        let FeatureKind::SmFlange(f) = &volta else { panic!() };
        assert_eq!(f.edge, [100., 25., 2.]);
        // o que é só do modo antigo recomeça do padrão: os recuos da aba comum não vão para o laço
        let FeatureKind::SmFlange(orig) = aba() else { panic!() };
        assert_eq!(orig.off1, "5");
        assert_eq!(f.off1, "0");
    }

    #[test]
    fn mesmo_modo_nao_mexe() {
        assert_eq!(converter_aba(&aba(), ModoAba::Aresta), Some(aba()));
    }

    #[test]
    fn modo_sem_aresta_comeca_vazio_e_nao_inventa_ponto() {
        let perfil = converter_aba(&aba(), ModoAba::Perfil).unwrap();
        let FeatureKind::SmProfileFlange(p) = &perfil else { panic!() };
        assert_eq!(p.l2, "30");
        let de_volta = converter_aba(&perfil, ModoAba::Aresta).unwrap();
        let FeatureKind::SmFlange(f) = &de_volta else { panic!() };
        assert_eq!(f.edge, [0.; 3], "o perfil não tem aresta: o suporte fica por escolher");
        assert_eq!(f.angle, "90");
    }
}
