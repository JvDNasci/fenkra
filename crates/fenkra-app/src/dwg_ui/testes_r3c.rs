// SPDX-License-Identifier: LGPL-2.1-or-later
//! Rodada 3 do teste com mouse real, bancada Desenho: B472 (Balões automáticos sem montagem) e B473
//! (o conjunto de vistas automático cabe na folha).

use super::*;

fn estado_com_placa(nome: &str) -> DwgState {
    let dir = std::env::temp_dir().join(format!("{nome}_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("placa.fkp"), fenkra_solidos::example_document().to_json()).unwrap();
    let mut d = Drawing::new("T", Format::A3);
    let s = d.sheets[0].id;
    let src = Source { path: "placa.fkp".into(), display_state: String::new(), hidden: vec![] };
    d.add_view(s, src.clone(), ViewKind::Standard(Dir::Frente), [150., 150.], 1.);
    let mut st = DwgState::new(d, Some(dir.join("t.fkd")));
    st.source = Some(src);
    st
}

#[test]
fn baloes_com_peca_como_origem_avisam_e_nao_criam_nada() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut st = estado_com_placa("fenkra_dwg_r3c_baloes");
    st.tool = Some(c::AUTO_BALLOONS);
    let (msg, erro) = baloes_ui::terminar(&mut st);
    assert!(erro, "sem montagem é erro, não sucesso");
    assert_eq!(msg, tr(baloes_ui::SEM_MONTAGEM));
    assert!(msg.to_lowercase().contains("montagem") || msg.to_lowercase().contains("assembly"), "{msg}");
    assert!(st.d.notes.is_empty(), "nenhum balão criado");
    assert!(crate::i18n::tem_traducao(baloes_ui::SEM_MONTAGEM));
    // o painel também mostra o motivo (a barra lê o mesmo erro)
    assert_eq!(baloes_ui::candidatos(&mut st).err(), Some(baloes_ui::SEM_MONTAGEM));
}

#[test]
fn janela_de_dois_cliques_nao_passa_de_um_uso_para_o_outro() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut st = estado_com_placa("fenkra_dwg_r3c_janela");
    st.tool = Some(c::AUTO_BALLOONS);
    baloes_ui::clique(&mut st, [100., 100.]);
    baloes_ui::clique(&mut st, [200., 200.]);
    st.baloes.fora.push("item 1".into());
    assert_eq!(st.baloes.janela.len(), 2);
    // armar o comando de novo (ou cancelar) começa sem janela nem peças tiradas
    baloes_ui::rearmar(&mut st);
    assert!(st.baloes.janela.is_empty() && st.baloes.fora.is_empty());
}

#[test]
fn conjunto_automatico_fica_dentro_da_area_util_mesmo_clicando_no_canto() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut st = estado_com_placa("fenkra_dwg_r3c_conjunto");
    st.d.views.clear();
    st.scale_auto = true;
    let sheet = st.sheet;
    let area = fenkra_desenho::conjunto::area_util(&st.d, sheet);
    // clique colado na quina da moldura: o conjunto inteiro é trazido para dentro
    let (t, ids) = ghost::new_views(&mut st, c::SET, [21., 11.]).expect("conjunto");
    assert_eq!(ids.len(), 4);
    for i in ids {
        let v = t.view(i).unwrap().clone();
        let p = st.models.proj(&t, i);
        let (a, b) = (layout::to_sheet(&v, &p, p.lo), layout::to_sheet(&v, &p, p.hi));
        let (lo, hi) = ([a[0].min(b[0]), a[1].min(b[1])], [a[0].max(b[0]), a[1].max(b[1])]);
        assert!(lo[0] >= area.0[0] - 0.01 && lo[1] >= area.0[1] - 0.01 && hi[0] <= area.1[0] + 0.01 && hi[1] <= area.1[1] + 0.01, "vista fora da área útil {area:?}: {lo:?} {hi:?}");
    }
}
