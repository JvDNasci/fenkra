// SPDX-License-Identifier: LGPL-2.1-or-later
//! Testes da ferramenta de marca de centro (B294) e das tabelas configuráveis (B295). Montam o
//! desenho em código com a placa de exemplo (quatro furos) e conferem o que a ferramenta cria.
//! O que depende do mouse real (cursor, arrastar) não está aqui.

use super::*;
use fenkra_desenho::centros::{self, ModoCentro};

/// Estado com a placa de exemplo vista de cima (os quatro furos aparecem como círculos).
fn estado_topo(nome: &str) -> (DwgState, Id) {
    let dir = std::env::temp_dir().join(nome);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("placa.fkp"), fenkra_solidos::example_document().to_json()).unwrap();
    let mut d = Drawing::new("T", Format::A3);
    let s = d.sheets[0].id;
    let src = Source { path: "placa.fkp".into(), display_state: String::new(), hidden: vec![] };
    let v = d.add_view(s, src.clone(), ViewKind::Standard(Dir::Topo), [200., 150.], 1.);
    let mut st = DwgState::new(d, Some(dir.join("t.fkd")));
    st.source = Some(src);
    (st, v)
}

/// Centros dos furos (raio pequeno) na folha, do menor x ao maior.
fn furos_na_folha(st: &mut DwgState, v: Id) -> Vec<V2> {
    let p = st.models.proj(&st.d, v);
    let vw = st.d.view(v).unwrap().clone();
    let ax = p.axes.unwrap();
    // os quatro furos da placa de exemplo ficam em (±48, ±24) no modelo
    let arcos = centros::arcos(&p);
    [(-1., -1.), (1., -1.), (1., 1.), (-1., 1.)]
        .iter()
        .filter_map(|(sx, sy)| {
            let q = ax.project([sx * 48., sy * 24., 0.]);
            arcos.iter().find(|(c, _)| ((c[0] - q[0]).powi(2) + (c[1] - q[1]).powi(2)).sqrt() < 0.05).map(|(c, _)| layout::to_sheet(&vw, &p, *c))
        })
        .collect()
}

#[test]
fn ferramenta_cria_marca_simples_linear_e_circular_nos_furos_da_vista() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let (mut st, v) = estado_topo("fenkra_dwg_ui_centros");
    let f = furos_na_folha(&mut st, v);
    assert!(f.len() >= 4, "furos na vista de topo: {}", f.len());
    // simples: um clique dentro do furo
    st.tool = Some(c::CENTER);
    st.centro.modo = ModoCentro::Simples;
    let antes = st.d.clone();
    let g = ghost::compute(&mut st, f[0], &|_, _, _| None, &|_, _| Some(v)).expect("fantasma");
    assert!(!g.items.is_empty(), "o fantasma mostra a cruz antes do clique");
    assert_eq!(st.d, antes, "o fantasma não cria nada");
    let id = centros_ui::clicar(&mut st, f[0]).expect("clique").expect("simples termina no primeiro furo");
    assert_eq!(st.d.centers.len(), 1);
    assert_eq!(st.d.centers[0].modo, ModoCentro::Simples);
    assert_eq!(st.sel, Some(id));
    assert!(st.tool.is_none(), "a ferramenta termina com a marca criada");
    // linear: dois furos
    st.tool = Some(c::CENTER);
    st.centro.modo = ModoCentro::Linear;
    assert_eq!(centros_ui::clicar(&mut st, f[0]), Ok(None), "um furo só não basta");
    let l = centros_ui::clicar(&mut st, f[3]).unwrap().expect("o segundo furo conclui");
    let mc = st.d.centers.iter().find(|m| m.id == l).unwrap();
    assert_eq!((mc.modo, mc.anchors.len()), (ModoCentro::Linear, 2));
    // circular: os quatro furos do retângulo estão num mesmo círculo; Enter conclui
    st.tool = Some(c::CENTER);
    st.centro.modo = ModoCentro::Circular;
    for p in &f[..4] {
        assert_eq!(centros_ui::clicar(&mut st, *p), Ok(None));
    }
    assert!(centros_ui::terminar(&mut st), "Enter conclui o círculo de furos");
    let mc = st.d.centers.last().unwrap();
    assert_eq!((mc.modo, mc.anchors.len()), (ModoCentro::Circular, 4));
    assert!(mc.centro.is_some());
    // a folha desenha as três marcas e esconder a vista as tira
    let com = layout::sheet(&st.d, st.sheet, &mut st.models).len();
    st.d.centros_ocultos.push(v);
    assert!(layout::sheet(&st.d, st.sheet, &mut st.models).len() < com);
    // clique longe de qualquer círculo: erro e nada criado
    st.tool = Some(c::CENTER);
    st.clicks.clear();
    st.tool_view = None;
    let n = st.d.centers.len();
    assert!(centros_ui::clicar(&mut st, [20., 20.]).is_err());
    assert_eq!(st.d.centers.len(), n);
}

#[test]
fn clique_no_cabecalho_ordena_e_na_etiqueta_destaca_a_linha() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let (mut st, v) = estado_topo("fenkra_dwg_ui_tabelas");
    let sheet = st.sheet;
    let src = st.source.clone().unwrap();
    let id = st.d.alloc();
    st.d.tables.push(Table { id, sheet, kind: TableKind::Holes, source: src, pos: [25., 120.], level: Default::default(), cfg: Default::default() });
    // sem tabela selecionada o cabeçalho não reage (clique solto não ordena sem querer)
    let s = st.d.text_height * 0.8;
    let rh = s * 1.9;
    let cabecalho = [25. + 1., 120. - rh * 1.5];
    assert!(!tabelas_ui::clique(&mut st, cabecalho));
    st.sel = Some(id);
    assert!(tabelas_ui::clique(&mut st, cabecalho), "clique no cabeçalho ordena");
    assert_eq!(st.d.tables[0].cfg.ordenar, Some(("Furo".to_string(), true)));
    assert!(tabelas_ui::clique(&mut st, cabecalho));
    assert_eq!(st.d.tables[0].cfg.ordenar, Some(("Furo".to_string(), false)));
    // etiquetas ligadas: o clique na etiqueta do segundo furo seleciona a tabela e destaca a linha
    st.d.tables[0].cfg.ordenar = None;
    st.d.tables[0].cfg.etiquetas = true;
    st.sel = None;
    let t = st.d.tables[0].clone();
    let fin = fenkra_desenho::tabela::dados(&mut st.models, &t, &st.d);
    let ets = fenkra_desenho::tabela::etiquetas(&mut st.models, &st.d, &t, &fin);
    let e = ets.iter().find(|e| e.view == v && e.linha == 1).expect("etiqueta do furo 2").clone();
    assert_eq!(e.texto, "A2");
    assert!(tabelas_ui::clique(&mut st, [e.pos[0] + 0.5, e.pos[1] + 0.5]));
    assert_eq!(st.sel, Some(id));
    assert_eq!(st.d.tables[0].cfg.linha_sel, Some(1));
    // e a linha em destaque entra na folha como contorno de alerta
    let items = layout::sheet(&st.d, sheet, &mut st.models);
    assert!(items.iter().any(|i| i.alert && i.owner == Some(id)), "linha da tabela em destaque");
    // clicar em outro lugar limpa o destaque
    assert!(!tabelas_ui::clique(&mut st, [5., 5.]));
    assert_eq!(st.d.tables[0].cfg.linha_sel, None);
}

#[test]
fn cota_inteligente_pega_o_centro_da_marca_e_mede_entre_furos() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    use fenkra_desenho::cotas::Alvo;
    let (mut st, v) = estado_topo("fenkra_dwg_ui_cota_centro");
    let f = furos_na_folha(&mut st, v);
    assert!(f.len() >= 4);
    st.tool = Some(c::CENTER);
    st.centro.modo = ModoCentro::Linear;
    assert_eq!(centros_ui::clicar(&mut st, f[0]), Ok(None));
    centros_ui::clicar(&mut st, f[1]).unwrap().expect("marca linear entre dois furos");
    st.tool = None;
    st.zoom = 3.78;
    let snap = |_: &mut DwgState, _: V2, _: Option<Id>| None;
    let vista = |_: &DwgState, _: V2| Some(v);
    let (vid, a) = cota_inteligente::alvo_sob(&mut st, f[0], &snap, &vista).expect("âncora");
    assert_eq!(vid, v);
    let Alvo::Ponto(a) = a else { panic!("esperava ponto, veio {a:?}") };
    st.ci.view = Some(v);
    let (_, b) = cota_inteligente::alvo_sob(&mut st, f[1], &snap, &vista).expect("segundo centro");
    let Alvo::Ponto(b) = b else { panic!("esperava ponto") };
    // os furos vizinhos ficam a 96 mm no eixo x (±48)
    assert!(((a[0] - b[0]).hypot(a[1] - b[1]) - 96.).abs() < 0.1, "distância {a:?} {b:?}");
    // furo sem marca continua dando o arco
    let (_, c3) = cota_inteligente::alvo_sob(&mut st, f[2], &snap, &vista).expect("arco");
    assert!(matches!(c3, Alvo::Arco { .. }), "{c3:?}");
}
