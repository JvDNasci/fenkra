// SPDX-License-Identifier: LGPL-2.1-or-later
//! B191: PMI gráfico e vistas salvas da flange anotada no STEP AP242, nos três modos do diálogo
//! (nenhum, gráfico, gráfico + semântico), relidos do próprio arquivo.

#[path = "../../fenkra-solidos/tests/common/mod.rs"]
mod common;

#[path = "pmi_corpus/mod.rs"]
mod pmi_corpus;

use common::*;
use fenkra_conversor::{anotacoes_com_modo, EsquemaSaida, ModoPmi, OpcoesExportar, Registro, NOTAS_PMI};
use fenkra_core::doc::NamedView;
use fenkra_core::pmi::PmiCapture;
use fenkra_core::*;
use fenkra_kernel::{DadoXde, ModeloXde, OpcoesLerXde};
use fenkra_solidos::final_shape;
use pmi_corpus::flange_anotada;

fn doc_com_capturas() -> Document {
    let mut d = flange_anotada();
    // a nota do corpus é vazia; aqui ela tem o texto de uma nota de fabricação
    let nota = d.pmi.iter_mut().find(|p| p.id == Id(9)).unwrap();
    nota.text = "Quebrar cantos vivos".into();
    nota.anchors = vec![[25., 0., 8.]];
    nota.offset = [10., 0., 10.];
    let frente = NamedView { name: String::new(), target: [0., 0., 4.], yaw: -std::f64::consts::FRAC_PI_2, pitch: 0., dist: 150., ortho: true };
    d.pmi_captures = vec![
        PmiCapture { id: Id(100), name: "Frente".into(), view: frente, shown: vec![Id(3), Id(8), Id(9)], hidden: vec![], with_model: false },
        PmiCapture { id: Id(101), name: "Topo".into(), view: NamedView { name: String::new(), target: [0.; 3], yaw: 0.3, pitch: 1.5, dist: 200., ortho: false }, shown: vec![Id(5), Id(6)], hidden: vec![], with_model: false },
    ];
    d
}

fn gravar_e_ler(doc: &Document, pmi: bool, grafico: bool, nome: &str) -> ModeloXde {
    let dir = std::env::temp_dir().join(format!("Fenkra pmi grafico {} {nome}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let arquivo = dir.join("flange.step");
    let o = OpcoesExportar { esquema: EsquemaSaida::Ap242, pmi, pmi_grafico: grafico, ..Default::default() };
    Registro::default().gravar_documento(doc, &arquivo, &o).unwrap();
    let lido = ModeloXde::ler(arquivo.to_str().unwrap(), OpcoesLerXde::default()).unwrap();
    // a peça não muda com o PMI
    let v = lido.produtos[0].forma.as_ref().unwrap().mass().volume;
    assert!((v - final_shape(doc).unwrap().mass().volume).abs() < 1e-6, "{nome}: volume");
    lido
}

#[test]
fn modo_grafico_desenha_tudo_e_nao_leva_semantica() {
    if !kernel_disponivel() {
        return;
    }
    let doc = doc_com_capturas();
    let lido = gravar_e_ler(&doc, false, true, "grafico");
    // as 9 anotações vão como desenho, nenhuma como cota ou quadro
    assert_eq!(lido.anotacoes.len(), doc.pmi.len(), "{:?}", lido.anotacoes.iter().map(|a| &a.dado).collect::<Vec<_>>());
    assert!(lido.anotacoes.iter().all(|a| a.dado == DadoXde::Grafico && a.apresentacao.as_ref().is_some_and(|p| p.segmentos.len() > 3)));
    let nomes: Vec<String> = lido.anotacoes.iter().map(|a| a.apresentacao.as_ref().unwrap().nome.clone()).collect();
    // o texto exibido vai no nome, em ASCII: a rugosidade e o diâmetro com tolerância
    assert!(nomes.iter().any(|n| n.contains("RUG Ra 1,6")), "{nomes:?}");
    assert!(nomes.iter().any(|n| n.starts_with("DIA 20")), "{nomes:?}");
    assert!(nomes.iter().all(|n| n.is_ascii()));
    // duas capturas, duas vistas, com as anotações certas
    assert_eq!(lido.vistas.len(), 2);
    let frente = lido.vistas.iter().find(|v| v.nome == "Frente").unwrap();
    assert!(frente.ortogonal);
    assert!((frente.direcao[1] - 1.).abs() < 1e-9, "a frente olha ao longo de +Y: {:?}", frente.direcao);
    assert!((frente.olho[1] + 150.).abs() < 1e-6 && (frente.olho[2] - 4.).abs() < 1e-6, "{:?}", frente.olho);
    assert_eq!(frente.anotacoes.len(), 3);
    let topo = lido.vistas.iter().find(|v| v.nome == "Topo").unwrap();
    assert!(!topo.ortogonal);
    assert_eq!(topo.anotacoes.len(), 2);
}

#[test]
fn modo_grafico_e_semantico_leva_os_dois_e_o_resto_como_desenho() {
    if !kernel_disponivel() {
        return;
    }
    let doc = doc_com_capturas();
    let lido = gravar_e_ler(&doc, true, true, "ambos");
    let dims = lido.anotacoes.iter().filter(|a| matches!(a.dado, DadoXde::Dimensao { .. })).count();
    let tols = lido.anotacoes.iter().filter(|a| matches!(a.dado, DadoXde::Tolerancia { .. })).count();
    let graficas = lido.anotacoes.iter().filter(|a| a.dado == DadoXde::Grafico).count();
    // como no modo semântico: 2 cotas, 2 quadros; rugosidade e nota vão só como desenho
    assert_eq!((dims, tols, graficas), (2, 2, 2), "{:?}", lido.anotacoes.iter().map(|a| &a.dado).collect::<Vec<_>>());
    // toda cota e todo quadro levam o desenho junto
    for a in lido.anotacoes.iter().filter(|a| matches!(a.dado, DadoXde::Dimensao { .. } | DadoXde::Tolerancia { .. })) {
        assert!(a.apresentacao.as_ref().is_some_and(|p| !p.segmentos.is_empty() && p.plano.is_some()), "{:?}", a.dado);
    }
    // a vista de topo mostra os dois quadros (posição e planeza), agora semânticos
    let topo = lido.vistas.iter().find(|v| v.nome == "Topo").unwrap();
    assert_eq!(topo.anotacoes.len(), 2);
    assert!(topo.anotacoes.iter().all(|i| matches!(lido.anotacoes[*i].dado, DadoXde::Tolerancia { .. })));
    // a frente mostra a espessura (semântica), a rugosidade e a nota (desenho)
    let frente = lido.vistas.iter().find(|v| v.nome == "Frente").unwrap();
    let mut tipos: Vec<bool> = frente.anotacoes.iter().map(|i| lido.anotacoes[*i].dado == DadoXde::Grafico).collect();
    tipos.sort();
    assert_eq!(tipos, vec![false, true, true]);
}

#[test]
fn modo_nenhum_e_semantico_nao_levam_desenho_nem_vista() {
    if !kernel_disponivel() {
        return;
    }
    let doc = doc_com_capturas();
    let nada = gravar_e_ler(&doc, false, false, "nada");
    assert_eq!(nada.pmi, [0, 0, 0]);
    assert!(nada.vistas.is_empty());
    let sem = gravar_e_ler(&doc, true, false, "semantico");
    assert!(sem.anotacoes.iter().all(|a| a.apresentacao.is_none() && a.dado != DadoXde::Grafico));
    assert!(sem.vistas.is_empty());
}

#[test]
fn notas_dos_modos_com_desenho_tem_traducao_conhecida_e_dizem_o_que_virou_desenho() {
    if !kernel_disponivel() {
        return;
    }
    let mut doc = doc_com_capturas();
    // um ângulo não tem tipo no STEP gravado: no modo com desenho, vai como desenho e a nota diz isso
    let mut ang = doc.pmi[2].clone();
    ang.id = Id(50);
    ang.name = "chanfro".into();
    ang.kind = fenkra_core::pmi::PmiKind::Angle;
    ang.anchors = vec![[0., 0., 8.], [10., 0., 8.], [0., 10., 8.]];
    doc.pmi.push(ang);
    // nota vazia, sem deslocamento: nada a desenhar
    let mut vazia = doc.pmi.iter().find(|p| p.id == Id(9)).unwrap().clone();
    vazia.id = Id(51);
    vazia.name = "vazia".into();
    vazia.text.clear();
    vazia.offset = [0.; 3];
    doc.pmi.push(vazia);
    let forma = final_shape(&doc).unwrap();
    let s = anotacoes_com_modo(&doc, &forma, ModoPmi::GraficoSemantico);
    for n in &s.fora {
        assert!(NOTAS_PMI.contains(&n.modelo.as_str()), "{}", n.modelo);
    }
    assert!(s.fora.iter().any(|n| n.modelo.starts_with("A anotação {0} vai só como desenho") && n.args[0] == "chanfro"), "{:?}", s.fora);
    assert!(!s.fora.iter().any(|n| n.modelo.contains("não tem tipo equivalente") && n.args[0] == "chanfro"));
    assert!(!s.fora.iter().any(|n| n.modelo.starts_with("PMI gráfico (")), "no modo com desenho a rugosidade e a nota vão");
    assert!(s.fora.iter().any(|n| n.modelo.contains("nem texto") && n.args[0] == "vazia"), "{:?}", s.fora);
    assert!(!s.origem.contains(&Id(51)));
    assert_eq!(s.anotacoes.len(), s.origem.len());
}

