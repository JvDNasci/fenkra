// SPDX-License-Identifier: LGPL-2.1-or-later
//! Testes do corte completo, do detalhe em perfil (B106) e da cota inteligente (B107). Montam o
//! desenho em código, com a placa de exemplo gravada numa pasta própria, e conferem o que a
//! ferramenta cria. O que depende do mouse real (arrastar alças, ler o cursor) não está aqui.

use super::*;
use fenkra_desenho::cotas::{Alvo, Motivo};

/// Estado com a placa de exemplo (vista da frente em [150, 150]).
fn estado_com_placa(nome: &str) -> (DwgState, Id) {
    let dir = std::env::temp_dir().join(nome);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("placa.fkp"), fenkra_solidos::example_document().to_json()).unwrap();
    let mut d = Drawing::new("T", Format::A3);
    let s = d.sheets[0].id;
    let src = Source { path: "placa.fkp".into(), display_state: String::new(), hidden: vec![] };
    let v = d.add_view(s, src.clone(), ViewKind::Standard(Dir::Frente), [150., 150.], 1.);
    let mut st = DwgState::new(d, Some(dir.join("t.fkd")));
    st.source = Some(src);
    (st, v)
}

fn xs_do_fantasma(g: &ghost::Ghost) -> (f64, f64) {
    let xs: Vec<f64> = g
        .items
        .iter()
        .skip(1)
        .flat_map(|i| match &i.prim {
            Prim::Line { pts, .. } => pts.iter().map(|p| p[0]).collect::<Vec<_>>(),
            _ => vec![],
        })
        .collect();
    xs.iter().fold((f64::MAX, f64::MIN), |a, x| (a.0.min(*x), a.1.max(*x)))
}

#[test]
fn linha_de_corte_em_degraus_conclui_com_enter_e_a_vista_nasce_alinhada() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let (mut st, v) = estado_com_placa("fenkra_dwg_ui_corte_degraus");
    st.tool = Some(c::SECTION);
    st.sel = Some(v);
    for p in [[80., 150.], [150., 150.], [150., 158.], [220., 158.]] {
        assert!(corte_ui::clique(&mut st, p).is_none(), "ainda traçando");
    }
    assert_eq!(st.clicks.len(), 4);
    assert!(!corte_ui::pronta(&st, c::SECTION));
    assert!(corte_ui::terminar(&mut st), "Enter conclui a linha");
    assert!(!corte_ui::terminar(&mut st), "já concluída");
    // a vista da linha horizontal desliza só na vertical: o x fica o da mãe, mesmo com o clique longe
    st.ctrl_livre = false;
    let id = corte_ui::clique(&mut st, [300., 40.]).expect("o clique de posição cria a vista");
    let vw = st.d.view(id).unwrap().clone();
    assert!((vw.pos[0] - 150.).abs() < 1e-9 && (vw.pos[1] - 40.).abs() < 1e-9, "{:?}", vw.pos);
    assert_eq!(vw.corte.pontos.len(), 2, "quatro vértices: a, b e mais dois");
    assert!(st.tool.is_none() && st.clicks.is_empty() && !st.corte_pronto);
    let p = st.models.proj(&st.d, id);
    assert!(p.error.is_none(), "{:?}", p.error);
    assert_eq!(p.seams.len(), 2);
    assert!(!p.hatch.is_empty());
}

#[test]
fn duplo_clique_conclui_a_linha_e_ctrl_solta_o_alinhamento() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let (mut st, v) = estado_com_placa("fenkra_dwg_ui_corte_duplo");
    st.tool = Some(c::SECTION);
    st.sel = Some(v);
    corte_ui::clique(&mut st, [80., 150.]);
    corte_ui::clique(&mut st, [220., 150.]);
    // o segundo clique do duplo clique cai no mesmo ponto: conclui sem repetir o vértice
    corte_ui::clique(&mut st, [220., 150.]);
    assert!(st.corte_pronto && st.clicks.len() == 2);
    st.ctrl_livre = true;
    let id = corte_ui::clique(&mut st, [300., 40.]).unwrap();
    assert_eq!(st.d.view(id).unwrap().pos, [300., 40.], "Ctrl solta o alinhamento");
    assert!(st.d.view(id).unwrap().corte.pontos.is_empty(), "corte reto");
}

#[test]
fn fantasma_do_corte_aparece_durante_o_tracado_ao_lado_da_mae_e_depois_alinhado() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let (mut st, v) = estado_com_placa("fenkra_dwg_ui_corte_fantasma");
    let snap = |_: &mut DwgState, _: V2, _: Option<Id>| None;
    let sempre_v = move |_: &DwgState, _: V2| Some(v);
    st.tool = Some(c::SECTION);
    st.sel = Some(v);
    st.clicks = vec![[80., 150.]];
    // um ponto só e o cursor como segundo já desenham a vista em corte, com hachura
    let g = ghost::compute(&mut st, [220., 150.], &snap, &sempre_v).unwrap();
    assert!(g.items.len() > 10, "a vista resultante está no fantasma: {}", g.items.len());
    assert!(!g.guides.is_empty(), "linha de alinhamento com a mãe");
    // nada foi criado
    assert_eq!(st.d.views.len(), 1);
    // linha concluída: o fantasma segue o alinhamento (x da mãe), mesmo com o cursor longe
    st.clicks = vec![[80., 150.], [220., 150.]];
    st.corte_pronto = true;
    st.ctrl_livre = false;
    let longe = ghost::compute(&mut st, [330., 60.], &snap, &sempre_v).unwrap();
    let (lo, hi) = xs_do_fantasma(&longe);
    assert!(((lo + hi) / 2. - 150.).abs() < 8., "centro em x ~ 150, achou {}", (lo + hi) / 2.);
    st.ctrl_livre = true;
    let solto = ghost::compute(&mut st, [330., 60.], &snap, &sempre_v).unwrap();
    let (lo, hi) = xs_do_fantasma(&solto);
    assert!(((lo + hi) / 2. - 330.).abs() < 8., "com Ctrl segue o cursor, achou {}", (lo + hi) / 2.);
}

#[test]
fn detalhe_em_perfil_livre_nasce_com_o_contorno_clicado_e_a_escala_da_barra() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let (mut st, v) = estado_com_placa("fenkra_dwg_ui_detalhe_perfil");
    st.tool = Some(c::DETAIL);
    st.detalhe_perfil = true;
    st.sel = Some(v);
    st.scale_auto = false;
    st.new_scale = 5.;
    for p in [[100., 140.], [120., 140.], [120., 158.], [100., 150.]] {
        assert!(corte_ui::clique(&mut st, p).is_none());
    }
    assert!(corte_ui::terminar(&mut st), "Enter conclui com 3 ou mais pontos");
    let id = corte_ui::clique(&mut st, [260., 100.]).expect("cria o detalhe");
    let vw = st.d.view(id).unwrap();
    assert_eq!(vw.detalhe.contorno.len(), 4);
    assert_eq!(vw.scale, 5., "escala escolhida na criação");
    // detalhe em círculo: dois cliques e o terceiro posiciona, sem Enter
    let (mut st, v) = estado_com_placa("fenkra_dwg_ui_detalhe_circulo");
    st.tool = Some(c::DETAIL);
    st.sel = Some(v);
    corte_ui::clique(&mut st, [150., 150.]);
    corte_ui::clique(&mut st, [160., 150.]);
    assert!(corte_ui::pronta(&st, c::DETAIL));
    let id = corte_ui::clique(&mut st, [260., 100.]).unwrap();
    assert!(matches!(st.d.view(id).unwrap().kind, ViewKind::Detail { .. }) && st.d.view(id).unwrap().detalhe.contorno.is_empty());
}

#[test]
fn perfil_do_detalhe_precisa_de_tres_pontos() {
    let (mut st, v) = estado_com_placa("fenkra_dwg_ui_detalhe_dois");
    st.tool = Some(c::DETAIL);
    st.detalhe_perfil = true;
    st.sel = Some(v);
    corte_ui::clique(&mut st, [100., 140.]);
    corte_ui::clique(&mut st, [120., 140.]);
    assert!(!corte_ui::terminar(&mut st), "com 2 pontos não há contorno");
    assert!(!st.corte_pronto);
}

// ---------- B107: cota inteligente

fn estado_com_furo(nome: &str) -> (DwgState, Id, V2) {
    let (mut st, _v) = estado_com_placa(nome);
    let src = st.source.clone().unwrap();
    let s = st.sheet;
    let t = st.d.add_view(s, src, ViewKind::Standard(Dir::Topo), [150., 150.], 1.);
    let p = st.models.proj(&st.d, t);
    let vw = st.d.view(t).unwrap().clone();
    // um círculo da vista superior (o furo): ponto da circunferência na folha
    // o círculo que é um furo da peça (tem chamada gerada pelas features)
    let src = st.source.clone().unwrap();
    let (c, r) = p.lines.iter().filter_map(|(pl, _)| layout::arc_center(pl)).find(|(_, r)| st.models.chamada_furo(&src, 2. * r).is_some()).expect("a vista superior tem um furo da peça");
    let na_folha = layout::to_sheet(&vw, &p, [c[0] + r, c[1]]);
    (st, t, na_folha)
}

#[test]
fn cota_inteligente_pega_o_furo_propoe_raio_e_tab_troca_para_diametro() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let (mut st, t, m) = estado_com_furo("fenkra_dwg_ui_cota_inteligente");
    st.tool = Some(c::DIM_SMART);
    let snap = |_: &mut DwgState, _: V2, _: Option<Id>| None;
    let sobre = move |_: &DwgState, _: V2| Some(t);
    let (vid, alvo) = cota_inteligente::alvo_sob(&mut st, m, &snap, &sobre).expect("achou o furo");
    assert_eq!(vid, t);
    assert!(matches!(alvo, Alvo::Arco { .. }));
    st.ci.sel = vec![alvo];
    st.ci.view = Some(t);
    assert_eq!(cota_inteligente::tipo_atual(&st.ci), Some(DimKind::Radius), "o arco propõe o raio");
    st.ci.tipo = Some(DimKind::Diameter);
    assert_eq!(cota_inteligente::tipo_atual(&st.ci), Some(DimKind::Diameter), "Tab trocou");
    let sel = st.ci.sel.clone();
    let dm = cota_inteligente::montar(&mut st, t, &sel, DimKind::Diameter, [m[0] + 12., m[1]], Id(77)).unwrap();
    assert_eq!((dm.kind, dm.anchors.len(), dm.id), (DimKind::Diameter, 2, Id(77)));
    let vw = st.d.view(t).unwrap().clone();
    let p = st.models.proj(&st.d, t);
    let (valor, _) = layout::dim_value(&dm, &vw, &p).unwrap();
    assert!(valor > 0.5, "diâmetro medido: {valor}");
    // a chamada do furo substitui o texto pela gerada das features (mesmo gerador do 3D)
    let texto = cota_inteligente::chamada_do_furo(&mut st, t, &sel).expect("o furo da peça de exemplo tem chamada");
    assert!(texto.contains('Ø'), "{texto}");
    st.ci.chamada = true;
    let com = cota_inteligente::montar(&mut st, t, &sel, DimKind::Diameter, m, Id(78)).unwrap();
    assert_eq!(com.text, texto);
    let sem = {
        st.ci.chamada = false;
        cota_inteligente::montar(&mut st, t, &sel, DimKind::Diameter, m, Id(79)).unwrap()
    };
    assert!(sem.text.is_empty(), "sem a chamada a cota mostra o valor medido");
}

#[test]
fn cota_inteligente_diz_o_motivo_quando_nao_ha_ancora_sob_o_cursor() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let (mut st, t, _m) = estado_com_furo("fenkra_dwg_ui_cota_motivo");
    st.tool = Some(c::DIM_SMART);
    let snap = |_: &mut DwgState, _: V2, _: Option<Id>| None;
    let fora = |_: &DwgState, _: V2| None;
    let sobre = move |_: &DwgState, _: V2| Some(t);
    assert_eq!(cota_inteligente::alvo_sob(&mut st, [10., 10.], &snap, &fora).unwrap_err(), Motivo::ForaDeVista);
    assert_eq!(cota_inteligente::alvo_sob(&mut st, [10., 10.], &snap, &sobre).unwrap_err(), Motivo::SemAresta);
    // com uma seleção em andamento numa vista, o cursor sobre outra é recusado com o motivo certo
    st.ci.view = Some(Id(999));
    assert_eq!(cota_inteligente::alvo_sob(&mut st, [10., 10.], &snap, &sobre).unwrap_err(), Motivo::OutraVista);
}

#[test]
fn dois_vertices_em_sequencia_medem_a_distancia_com_afastamento_do_mouse() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let (mut st, t, _m) = estado_com_furo("fenkra_dwg_ui_cota_pontos");
    let p = st.models.proj(&st.d, t);
    let (a, b) = ([p.lo[0], p.lo[1]], [p.hi[0], p.lo[1]]);
    let sel = [Alvo::Ponto(a), Alvo::Ponto(b)];
    assert_eq!(fenkra_desenho::cotas::tipos(&sel)[0], DimKind::Horizontal);
    let dm = cota_inteligente::montar(&mut st, t, &sel, DimKind::Horizontal, [150., 100.], Id(5)).unwrap();
    let vw = st.d.view(t).unwrap().clone();
    let (valor, _) = layout::dim_value(&dm, &vw, &p).unwrap();
    assert!((valor - (b[0] - a[0])).abs() < 1e-6, "{valor} x {}", b[0] - a[0]);
    assert!(dm.offset.abs() >= 1., "afastamento vem do mouse");
}
