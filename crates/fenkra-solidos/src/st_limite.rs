// SPDX-License-Identifier: LGPL-2.1-or-later
//! Estrutural: aparar e estender a ponta de um membro até um limite (B37).
//!
//! O membro tem, em cada ponta, um prolongamento (`ext1` no início, `ext2` no fim) que o recompute
//! soma ao comprimento da linha guia; valor negativo apara. Aqui só se acha o valor que leva a ponta
//! até o limite, sem sólidos: a interface chama a cada quadro para a prévia (o que sobra, o que sai,
//! o que entra) e, ao confirmar, grava o número no comando.
//!
//! Limites: um plano (ponto e normal) ou o eixo de outro membro, com um recuo (meia largura do outro
//! perfil, para a ponta parar na face dele e não no eixo).

use crate::geo3d;
use crate::st_frames::MemberFrame;
use crate::geo3d::V3;
use fenkra_core::{Document, FeatureKind, Id};

/// Ponta do membro que se mexe.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ponta {
    Inicio,
    Fim,
}

/// O que o usuário quer: só aparar (encurtar), só estender (alongar) ou o que for preciso.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Acao {
    Aparar,
    Estender,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Limite {
    Plano { ponto: V3, normal: V3 },
    /// eixo de outro membro (dois pontos) e recuo em mm antes dele
    Eixo { a: V3, b: V3, recuo: f64 },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Resultado {
    pub ponta: Ponta,
    pub acao: Acao,
    /// prolongamento hoje e o que leva a ponta ao limite (mm; negativo apara)
    pub ext_atual: f64,
    pub ext_novo: f64,
    /// o limite fica do lado que a ação pede? (aparar: dentro do membro; estender: além da ponta)
    pub alcanca: bool,
    /// o valor a gravar: o novo se alcança ou se foi pedido prolongar além; senão o atual
    pub ext_aplicado: f64,
    /// onde o eixo do membro encontra o limite
    pub ponto_limite: V3,
    /// ponta atual do eixo (com o prolongamento de hoje)
    pub ponta_atual: V3,
    /// trecho que sai (vermelho na prévia), só ao aparar
    pub removido: Option<(V3, V3)>,
    /// trecho que entra (verde), só ao estender
    pub acrescentado: Option<(V3, V3)>,
    /// direção, a partir do limite, do lado que sobra do membro (a seta da prévia)
    pub lado_que_sobra: V3,
}

/// Distância ao longo de `d` a partir de `p` até o limite; `None` se a linha não o encontra.
fn distancia_ate(p: V3, d: V3, limite: &Limite) -> Option<f64> {
    match *limite {
        Limite::Plano { ponto, normal } => {
            let n = geo3d::unit(normal)?;
            let den = geo3d::dot(n, d);
            if den.abs() < 1e-9 {
                return None;
            }
            Some(geo3d::dot(n, geo3d::sub(ponto, p)) / den)
        }
        Limite::Eixo { a, b, recuo } => {
            let u = geo3d::unit(geo3d::sub(b, a))?;
            // mínimo da distância entre as retas p + s·d e a + t·u
            let w = geo3d::sub(p, a);
            let (dd, du, uu) = (geo3d::dot(d, d), geo3d::dot(d, u), geo3d::dot(u, u));
            let (dw, uw) = (geo3d::dot(d, w), geo3d::dot(u, w));
            let den = dd * uu - du * du;
            if den.abs() < 1e-9 {
                return None;
            }
            let s = (du * uw - uu * dw) / den;
            // a ponta para na face do outro membro: recuo medido na perpendicular ao outro eixo
            let seno = (1. - du * du / (dd * uu)).max(0.).sqrt().max(0.05);
            Some(s - recuo / seno)
        }
    }
}

/// Valor de prolongamento que leva a ponta do membro ao limite.
///
/// `alem`: aceita o resultado mesmo quando o limite não fica do lado pedido (aparar que na verdade
/// precisa prolongar, estender que na verdade precisa encurtar). Sem isso, `ext_aplicado` fica no
/// valor de hoje e `alcanca` diz o que houve.
pub fn calcular(fr: &MemberFrame, ponta: Ponta, acao: Acao, limite: &Limite, alem: bool) -> Result<Resultado, String> {
    let (base, saindo, ext_atual, outro_ext) = match ponta {
        Ponta::Inicio => (fr.start, geo3d::mul(fr.z, -1.), fr.ext.0, fr.ext.1),
        Ponta::Fim => (fr.end, fr.z, fr.ext.1, fr.ext.0),
    };
    let s = distancia_ate(base, saindo, limite).ok_or("o eixo do membro não encontra o limite (paralelo)")?;
    let comprimento = geo3d::dist(fr.start, fr.end);
    if comprimento + outro_ext + s < 1e-3 {
        return Err("o limite fica além da outra ponta: o membro some".into());
    }
    let ponta_atual = geo3d::add(base, geo3d::mul(saindo, ext_atual));
    let ponto_limite = geo3d::add(base, geo3d::mul(saindo, s));
    let alcanca = match acao {
        Acao::Aparar => s <= ext_atual + 1e-9,
        Acao::Estender => s >= ext_atual - 1e-9,
    };
    let ext_aplicado = if alcanca || alem { s } else { ext_atual };
    let aplica = alcanca || alem;
    let (removido, acrescentado) = if !aplica {
        (None, None)
    } else if s < ext_atual {
        (Some((ponto_limite, ponta_atual)), None)
    } else {
        (None, Some((ponta_atual, ponto_limite)))
    };
    Ok(Resultado {
        ponta,
        acao,
        ext_atual,
        ext_novo: s,
        alcanca,
        ext_aplicado,
        ponto_limite,
        ponta_atual,
        removido,
        acrescentado,
        lado_que_sobra: geo3d::mul(saindo, -1.),
    })
}

/// Grava o prolongamento `valor` na ponta da linha guia do comando `id`. Devolve se mudou algo.
pub fn aplicar(doc: &mut Document, id: Id, ponta: Ponta, valor: f64) -> bool {
    let Some(FeatureKind::StructMember(m)) = doc.feature_mut(id).map(|f| &mut f.kind) else { return false };
    let texto = format!("{}", (valor * 1e4).round() / 1e4);
    let campo = match ponta {
        Ponta::Inicio => &mut m.ext1,
        Ponta::Fim => &mut m.ext2,
    };
    if *campo == texto {
        return false;
    }
    *campo = texto;
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::st_frames::member_frames;
    use fenkra_core::{CornerMode, CurveRef, PointRef, RefKind, SectionKind, StructMember};

    fn membro(d: &mut Document, a: [f64; 3], b: [f64; 3]) -> Id {
        let s = |x: &str| x.to_string();
        let l = CurveRef::Ref(d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed(a), b: PointRef::Fixed(b), start: "0".into(), end: "0".into() }));
        d.add_feature(FeatureKind::StructMember(StructMember {
            paths: vec![l],
            family: s("Tubo quadrado"),
            designation: s("40x3"),
            kind: SectionKind::TuboQuadrado,
            dims: vec![s("40"), s("3")],
            anchor: 0,
            angle: s("0"),
            off_x: s("0"),
            off_y: s("0"),
            ext1: s("0"),
            ext2: s("0"),
            corner: CornerMode::Nenhum,
            gap: s("0"),
            joints: vec![],
            material: String::new(),
            mirror: false,
        }))
    }

    fn plano_z(z: f64) -> Limite {
        Limite::Plano { ponto: [0., 0., z], normal: [0., 0., 1.] }
    }

    #[test]
    fn aparar_encurta_ate_o_plano_e_o_trecho_que_sai_e_o_do_outro_lado() {
        let mut d = Document::new("t");
        let id = membro(&mut d, [0., 0., 0.], [0., 0., 200.]);
        let fr = member_frames(&d, id).remove(0);
        let r = calcular(&fr, Ponta::Fim, Acao::Aparar, &plano_z(150.), false).unwrap();
        assert!(r.alcanca);
        assert!((r.ext_aplicado + 50.).abs() < 1e-9, "{r:?}");
        assert_eq!(r.removido, Some(([0., 0., 150.], [0., 0., 200.])));
        assert!(r.acrescentado.is_none());
        // a seta do lado que sobra aponta para o início
        assert!(geo3d::dist(r.lado_que_sobra, [0., 0., -1.]) < 1e-9);
        assert!(aplicar(&mut d, id, Ponta::Fim, r.ext_aplicado));
        let depois = member_frames(&d, id).remove(0);
        assert_eq!(depois.ext, (0., -50.));
    }

    #[test]
    fn estender_alonga_e_o_inicio_usa_a_direcao_oposta() {
        let mut d = Document::new("t");
        let id = membro(&mut d, [0., 0., 0.], [0., 0., 200.]);
        let fr = member_frames(&d, id).remove(0);
        let r = calcular(&fr, Ponta::Fim, Acao::Estender, &plano_z(230.), false).unwrap();
        assert!(r.alcanca && (r.ext_aplicado - 30.).abs() < 1e-9);
        assert_eq!(r.acrescentado, Some(([0., 0., 200.], [0., 0., 230.])));
        // início: o limite em z = -25 pede 25 mm de prolongamento
        let i = calcular(&fr, Ponta::Inicio, Acao::Estender, &plano_z(-25.), false).unwrap();
        assert!(i.alcanca && (i.ext_aplicado - 25.).abs() < 1e-9, "{i:?}");
    }

    #[test]
    fn aparar_que_nao_alcanca_so_prolonga_se_pedido() {
        let mut d = Document::new("t");
        let id = membro(&mut d, [0., 0., 0.], [0., 0., 200.]);
        let fr = member_frames(&d, id).remove(0);
        // o limite está além da ponta: aparar não alcança
        let r = calcular(&fr, Ponta::Fim, Acao::Aparar, &plano_z(260.), false).unwrap();
        assert!(!r.alcanca);
        assert_eq!(r.ext_aplicado, 0.);
        assert!(r.removido.is_none() && r.acrescentado.is_none());
        let p = calcular(&fr, Ponta::Fim, Acao::Aparar, &plano_z(260.), true).unwrap();
        assert!(!p.alcanca && (p.ext_aplicado - 60.).abs() < 1e-9);
        assert_eq!(p.acrescentado, Some(([0., 0., 200.], [0., 0., 260.])));
    }

    #[test]
    fn limite_pelo_eixo_de_outro_membro_para_na_face() {
        let mut d = Document::new("t");
        let id = membro(&mut d, [0., 0., 0.], [0., 0., 200.]);
        let fr = member_frames(&d, id).remove(0);
        // outro membro horizontal em z = 260, perfil de 40 mm: a ponta para 20 mm antes do eixo
        let lim = Limite::Eixo { a: [-100., 0., 260.], b: [100., 0., 260.], recuo: 20. };
        let r = calcular(&fr, Ponta::Fim, Acao::Estender, &lim, false).unwrap();
        assert!((r.ext_novo - 40.).abs() < 1e-9, "{r:?}");
    }

    #[test]
    fn paralelo_e_membro_que_some_dao_erro() {
        let mut d = Document::new("t");
        let id = membro(&mut d, [0., 0., 0.], [0., 0., 200.]);
        let fr = member_frames(&d, id).remove(0);
        let par = Limite::Plano { ponto: [0., 0., 50.], normal: [1., 0., 0.] };
        assert!(calcular(&fr, Ponta::Fim, Acao::Aparar, &par, false).is_err());
        // aparar o fim em z = -10: passaria da outra ponta
        assert!(calcular(&fr, Ponta::Fim, Acao::Aparar, &plano_z(-10.), true).is_err());
    }

    #[test]
    fn o_recompute_encurta_o_solido_pelo_valor_gravado() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let mut d = Document::new("t");
        let id = membro(&mut d, [0., 0., 0.], [0., 0., 200.]);
        let fr = member_frames(&d, id).remove(0);
        let r = calcular(&fr, Ponta::Fim, Acao::Aparar, &plano_z(150.), false).unwrap();
        aplicar(&mut d, id, Ponta::Fim, r.ext_aplicado);
        let itens = crate::structural_members(&d);
        assert_eq!(itens.len(), 1);
        // seção 40x40 com parede 3 (cantos vivos): 444 mm² × 150 mm
        let v = itens[0].shape.mass().volume;
        assert!((v - 444. * 150.).abs() < 1., "volume {v}");
    }
}
