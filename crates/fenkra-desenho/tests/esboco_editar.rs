// SPDX-License-Identifier: LGPL-2.1-or-later
//! B352: a ferramenta de esboço do desenho, pelo que ela chama. Cada teste monta o esboço com os
//! cliques que a folha daria, confere a geometria resolvida e o que sai na folha.

use fenkra_core::sketch::{Curve, Kind};
use fenkra_core::Id;
use fenkra_desenho::esboco::{DonoEsboco, TipoLinha};
use fenkra_desenho::esboco_editar::{self as ed, Alvo, Clique, Cota, Forma};
use fenkra_desenho::layout::{sheet, Prim, Style};
use fenkra_desenho::source::Models;
use fenkra_desenho::*;

fn perto(a: V2, b: V2) -> bool {
    (a[0] - b[0]).abs() < 1e-6 && (a[1] - b[1]).abs() < 1e-6
}

fn novo() -> (Drawing, Id) {
    let mut d = Drawing::new("t", Format::A3);
    let f = d.sheets[0].id;
    let id = d.add_esboco(DonoEsboco::Folha(f));
    (d, id)
}

#[test]
fn polilinha_pelos_cliques_liga_os_pontos_e_alinha_sozinha() {
    let (mut d, id) = novo();
    let e = d.esboco_mut(id).unwrap();
    // primeiro trecho quase horizontal, o segundo vertical, o terceiro torto
    let l1 = ed::criar(
        e,
        Forma::Linha,
        &[Clique::solto([100., 100.]), Clique::solto([140., 100.5])],
        false,
        TipoLinha::Visivel,
    )
    .unwrap()[0];
    let fim = ed::fim_da_linha(e, l1).unwrap();
    let l2 = ed::criar(
        e,
        Forma::Linha,
        &[
            Clique {
                uv: e.dados.pos(fim),
                ponto: Some(fim),
                fixo: false,
            },
            Clique::solto([140., 130.]),
        ],
        false,
        TipoLinha::Visivel,
    )
    .unwrap()[0];
    ed::criar(
        e,
        Forma::Linha,
        &[
            Clique {
                uv: [140., 130.],
                ponto: ed::fim_da_linha(e, l2),
                fixo: false,
            },
            Clique::solto([110., 150.]),
        ],
        false,
        TipoLinha::Fina,
    )
    .unwrap();
    assert_eq!(e.dados.curves.len(), 3);
    assert_eq!(e.dados.points.len(), 4, "pontos compartilhados nas emendas");
    let ks: Vec<&Kind> = e.dados.constraints.iter().map(|c| &c.kind).collect();
    assert!(
        ks.contains(&&Kind::Horizontal(l1)) && ks.contains(&&Kind::Vertical(l2)),
        "{ks:?}"
    );
    assert_eq!(ks.len(), 2, "a linha torta não ganha restrição");
    assert!(e.resolver());
    // resolvida, a primeira fica horizontal de fato
    let Curve::Line { a, b } = e.dados.curve(l1).unwrap().geom else {
        panic!()
    };
    assert!((e.dados.pos(a)[1] - e.dados.pos(b)[1]).abs() < 1e-6);
    assert_eq!(e.tipo(e.dados.curves[2].id), TipoLinha::Fina);

    // sai na folha: três linhas, duas visíveis e uma fina
    let mut m = Models::new(None);
    let items = sheet(&d, d.sheets[0].id, &mut m);
    let estilos: Vec<Style> = items
        .iter()
        .filter(|i| i.owner == Some(id))
        .filter_map(|i| {
            if let Prim::Line { style, .. } = &i.prim {
                Some(*style)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(
        estilos.iter().filter(|s| **s == Style::Visible).count(),
        2,
        "{estilos:?}"
    );
    assert_eq!(estilos.iter().filter(|s| **s == Style::Thin).count(), 1);
}

#[test]
fn retangulo_circulo_e_arcos_nascem_com_a_forma_dos_cliques() {
    let (mut d, id) = novo();
    let e = d.esboco_mut(id).unwrap();
    let r = ed::criar(
        e,
        Forma::RetanguloCentro,
        &[Clique::solto([50., 50.]), Clique::solto([70., 60.])],
        false,
        TipoLinha::Visivel,
    )
    .unwrap();
    assert_eq!(r.len(), 5, "4 lados e a diagonal");
    assert!(
        e.dados.curve(r[4]).unwrap().construction,
        "a diagonal é de construção"
    );
    assert!(e.resolver());
    let xs: Vec<f64> = e.dados.points.iter().map(|p| p.x).collect();
    assert!(
        xs.iter().any(|x| (x - 30.).abs() < 1e-6) && xs.iter().any(|x| (x - 70.).abs() < 1e-6),
        "{xs:?}"
    );

    let c = ed::criar(
        e,
        Forma::Circulo3,
        &[
            Clique::solto([10., 0.]),
            Clique::solto([0., 10.]),
            Clique::solto([-10., 0.]),
        ],
        false,
        TipoLinha::Centro,
    )
    .unwrap()[0];
    assert!((e.dados.radius_of(c).unwrap() - 10.).abs() < 1e-9);
    assert_eq!(e.tipo(c), TipoLinha::Centro);

    // arco por três pontos: passa pelo do meio, mesmo clicado "ao contrário"
    let a = ed::criar(
        e,
        Forma::Arco3,
        &[
            Clique::solto([10., 0.]),
            Clique::solto([-10., 0.]),
            Clique::solto([0., -10.]),
        ],
        false,
        TipoLinha::Visivel,
    )
    .unwrap()[0];
    let pts = ed::curva_pts(&e.dados, e.dados.curve(a).unwrap());
    assert!(
        pts.iter()
            .any(|p| perto(*p, [0., -10.]) || (p[1] + 10.).abs() < 0.05),
        "o arco passa por baixo: {:?}",
        &pts[..3]
    );
    assert!(pts.iter().all(|p| p[1] <= 1e-6), "e não por cima");

    // arco pelo centro: o fim cai no raio do início
    let a2 = ed::criar(
        e,
        Forma::ArcoCentro,
        &[
            Clique::solto([0., 0.]),
            Clique::solto([5., 0.]),
            Clique::solto([0., 30.]),
        ],
        false,
        TipoLinha::Visivel,
    )
    .unwrap()[0];
    let Curve::Arc { e: fim, .. } = e.dados.curve(a2).unwrap().geom else {
        panic!()
    };
    assert!(perto(e.dados.pos(fim), [0., 5.]));

    assert!(
        ed::criar(
            e,
            Forma::Circulo3,
            &[
                Clique::solto([0., 0.]),
                Clique::solto([1., 1.]),
                Clique::solto([2., 2.])
            ],
            false,
            TipoLinha::Visivel
        )
        .is_err(),
        "alinhados não fazem círculo"
    );
    assert!(
        ed::criar(
            e,
            Forma::Retangulo,
            &[Clique::solto([0., 0.]), Clique::solto([5., 0.])],
            false,
            TipoLinha::Visivel
        )
        .is_err(),
        "retângulo sem área"
    );
}

#[test]
fn cota_nasce_com_a_medida_e_manda_na_geometria() {
    let (mut d, id) = novo();
    let e = d.esboco_mut(id).unwrap();
    let r = ed::criar(
        e,
        Forma::Retangulo,
        &[Clique::solto([100., 100.]), Clique::solto([137., 123.])],
        false,
        TipoLinha::Visivel,
    )
    .unwrap();
    let Curve::Line { a: p0, .. } = e.dados.curve(r[0]).unwrap().geom else {
        panic!()
    };
    assert!(ed::com_resolucao(e, |s| {
        let mut al = ed::alocador(s);
        s.add_constraint(&mut al, Kind::Fixed(p0));
        true
    }));
    // ferramenta inteligente: a linha dá o comprimento, com a medida de agora
    let k = ed::cota_inteligente(e, &[Alvo::Curva(r[0])]).unwrap();
    assert_eq!(k, Cota::Comprimento(r[0]));
    let c = ed::cotar(e, k).unwrap();
    assert_eq!(ed::valor(e, c).as_deref(), Some("37"));
    assert!(ed::definir_valor(e, c, "40"));
    assert!((ed::medida(e, c).unwrap() - 40.).abs() < 1e-6);
    // vertical pelos pontos do lado
    let (a, b) = ed::pontos_da_cota(e, &[Alvo::Curva(r[1])]).unwrap();
    let v = ed::cotar(e, Cota::Vertical(a, b)).unwrap();
    assert!(ed::definir_valor(e, v, "20,5"), "vírgula decimal vale");
    let Curve::Line { b: p2, .. } = e.dados.curve(r[1]).unwrap().geom else {
        panic!()
    };
    assert!(
        perto(e.dados.pos(p2), [140., 120.5]),
        "{:?}",
        e.dados.pos(p2)
    );
    // valor impossível volta atrás
    let antes = e.dados.clone();
    assert!(!ed::definir_valor(e, c, "abc"));
    assert_eq!(e.dados, antes);

    // as cotas saem na folha com o valor
    let mut m = Models::new(None);
    let items = sheet(&d, d.sheets[0].id, &mut m);
    let t: Vec<String> = items
        .iter()
        .filter(|i| i.owner == Some(id))
        .filter_map(|i| {
            if let Prim::Text { text, .. } = &i.prim {
                Some(text.clone())
            } else {
                None
            }
        })
        .collect();
    assert!(
        t.iter().any(|x| x == "40") && t.iter().any(|x| x.starts_with("20")),
        "{t:?}"
    );
}

#[test]
fn arrastar_respeita_as_restricoes_e_o_alvo_acha_ponto_antes_da_curva() {
    let (mut d, id) = novo();
    let e = d.esboco_mut(id).unwrap();
    let l = ed::criar(
        e,
        Forma::Linha,
        &[Clique::solto([0., 0.]), Clique::solto([10., 0.])],
        false,
        TipoLinha::Visivel,
    )
    .unwrap()[0];
    let Curve::Line { a, b } = e.dados.curve(l).unwrap().geom else {
        panic!()
    };
    assert_eq!(ed::mais_perto(e, [9.8, 0.1], 0.5), Some(Alvo::Ponto(b)));
    assert_eq!(ed::mais_perto(e, [5., 0.2], 0.5), Some(Alvo::Curva(l)));
    assert_eq!(ed::mais_perto(e, [5., 3.], 0.5), None);
    // a linha é horizontal: arrastar a ponta para cima leva a outra junto
    assert!(ed::arrastar(e, b, [12., 4.]));
    assert!((e.dados.pos(a)[1] - e.dados.pos(b)[1]).abs() < 1e-6);
    assert!(perto(e.dados.pos(b), [12., 4.]));
    // ponto fixo (vértice da vista) não se arrasta
    let p = ed::criar(
        e,
        Forma::Ponto,
        &[Clique {
            uv: [3., 3.],
            ponto: None,
            fixo: true,
        }],
        false,
        TipoLinha::Visivel,
    )
    .unwrap()[0];
    let Curve::Point { p: pp } = e.dados.curve(p).unwrap().geom else {
        panic!()
    };
    assert!(!ed::arrastar(e, pp, [9., 9.]));
    // mover o esboço todo
    ed::mover(e, [1., 2.]);
    assert!(perto(e.dados.pos(pp), [4., 5.]));
}

#[test]
fn cotas_do_esboco_saem_para_fora_do_contorno() {
    let (mut d, id) = novo();
    let e = d.esboco_mut(id).unwrap();
    let r = ed::criar(
        e,
        Forma::Retangulo,
        &[Clique::solto([100., 100.]), Clique::solto([140., 120.])],
        false,
        TipoLinha::Visivel,
    )
    .unwrap();
    // comprimento da linha de baixo, vertical do lado direito e horizontal da linha de cima
    ed::cotar(e, Cota::Comprimento(r[0])).unwrap();
    let (a, b) = ed::pontos_da_cota(e, &[Alvo::Curva(r[1])]).unwrap();
    ed::cotar(e, Cota::Vertical(a, b)).unwrap();
    let (a, b) = ed::pontos_da_cota(e, &[Alvo::Curva(r[2])]).unwrap();
    ed::cotar(e, Cota::Horizontal(a, b)).unwrap();
    let mut m = Models::new(None);
    let items = sheet(&d, d.sheets[0].id, &mut m);
    let textos: Vec<V2> = items
        .iter()
        .filter(|i| i.owner == Some(id))
        .filter_map(|i| {
            if let Prim::Text { pos, .. } = &i.prim {
                Some(*pos)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(textos.len(), 3, "{textos:?}");
    let dentro = |p: &V2| p[0] > 100. && p[0] < 140. && p[1] > 100. && p[1] < 120.;
    assert!(
        textos.iter().all(|p| !dentro(p)),
        "texto de cota dentro do retângulo: {textos:?}"
    );
    assert!(
        textos.iter().any(|p| p[1] < 100.)
            && textos.iter().any(|p| p[0] > 140.)
            && textos.iter().any(|p| p[1] > 120.),
        "{textos:?}"
    );
}
