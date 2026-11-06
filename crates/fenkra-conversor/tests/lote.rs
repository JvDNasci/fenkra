// SPDX-License-Identifier: LGPL-2.1-or-later
//! B213: conversão em lote. A simulação não grava nada, e a execução roda exatamente o que ela listou.

use fenkra_conversor::lote::*;
use fenkra_conversor::processo::{Resposta, Trabalho};
use fenkra_conversor::{ErroConv, Formato, ModeloNeutro, OpcoesExportar, OpcoesImportar, Registro};
use fenkra_core::{BoolOp, Document, Extent, Extrude, FeatureKind};
use fenkra_kernel::Shape;
use std::path::{Path, PathBuf};

fn pasta(teste: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("Fenkra lote açaí {} {teste}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

fn step(caminho: &Path, lado: f64) {
    let forma = Shape::box_tool([0., 0., 0.], [1., 0., 0.], [0., 0., 1.], [lado; 3]).unwrap();
    Registro::default().gravar(&ModeloNeutro::de_forma(Formato::Step, "Bloco", forma), caminho, &OpcoesExportar::default()).unwrap();
}

fn perfil(formato: Formato) -> PerfilSaida {
    PerfilSaida { formato, opcoes: OpcoesExportar::default() }
}

fn plano(saida: &Path, formatos: &[Formato], se_existir: SeExistir) -> PlanoLote {
    PlanoLote { pasta: saida.to_path_buf(), perfis: formatos.iter().copied().map(perfil).collect(), se_existir, ..Default::default() }
}

/// Faz o papel do processo filho, no mesmo processo.
fn direto(t: Trabalho) -> Result<Resposta, ErroConv> {
    match t {
        Trabalho::Converter { origem, destino, importar, exportar } => Registro::default().converter(&origem, &destino, &importar, &exportar).map(|()| Resposta::Convertido),
        _ => Err(ErroConv::Leitura("trabalho inesperado".into())),
    }
}

fn arquivos(p: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(p).unwrap().flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect();
    v.sort();
    v
}

#[test]
fn pasta_expande_em_profundidade_e_ignora_o_que_nao_le() {
    let dir = pasta("entradas");
    std::fs::create_dir_all(dir.join("sub/fundo")).unwrap();
    for n in ["a.step", "sub/b.stl", "sub/fundo/c.fkp", "sub/leia.txt", "sub/fundo/d.png"] {
        std::fs::write(dir.join(n), "x").unwrap();
    }
    let e = listar_entradas(&[dir.clone()], &Registro::default());
    let nomes: Vec<&str> = e.iter().map(|x| x.nome.as_str()).collect();
    assert_eq!(nomes, vec!["a", "b", "c"]);
}

#[test]
fn simular_nao_grava_nada_e_diz_o_que_vai_acontecer() {
    let (dir, saida) = (pasta("simula"), pasta("simula saida"));
    step(&dir.join("a.step"), 10.);
    std::fs::write(saida.join("a.stl"), "antigo").unwrap();
    let entradas = vec![EntradaLote::de_arquivo(&dir.join("a.step")), EntradaLote::de_arquivo(&dir.join("sumiu.step"))];
    let p = plano(&saida, &[Formato::Stl, Formato::Glb], SeExistir::Pular);
    let itens = simular(&entradas, &p, &Registro::default());
    assert_eq!(itens.len(), 4);
    assert_eq!(itens[0].situacao, Situacao::Pular(MOTIVO_EXISTE.into()));
    assert_eq!(itens[1].situacao, Situacao::Converter);
    assert_eq!(itens[2].situacao, Situacao::Recusado(MOTIVO_AUSENTE.into()));
    assert_eq!(arquivos(&saida), vec!["a.stl"], "a simulação não pode gravar");
}

#[test]
fn politica_de_arquivo_existente_vale_na_execucao() {
    let (dir, saida) = (pasta("politica"), pasta("politica saida"));
    step(&dir.join("a.step"), 10.);
    let entradas = vec![EntradaLote::de_arquivo(&dir.join("a.step"))];
    std::fs::write(saida.join("a.stl"), "antigo").unwrap();

    let p = plano(&saida, &[Formato::Stl], SeExistir::Pular);
    let r = executar(&simular(&entradas, &p, &Registro::default()), &p, direto, |_, _| {});
    assert!(r.is_empty());
    assert_eq!(std::fs::read(saida.join("a.stl")).unwrap(), b"antigo");

    let p = plano(&saida, &[Formato::Stl], SeExistir::Numerar);
    let itens = simular(&entradas, &p, &Registro::default());
    assert_eq!(itens[0].situacao, Situacao::Numerado);
    let r = executar(&itens, &p, direto, |_, _| {});
    assert!(r[0].saida.is_ok(), "{:?}", r[0].saida);
    assert_eq!(arquivos(&saida), vec!["a (2).stl", "a.stl"]);
    assert_eq!(std::fs::read(saida.join("a.stl")).unwrap(), b"antigo");

    let p = plano(&saida, &[Formato::Stl], SeExistir::Substituir);
    let itens = simular(&entradas, &p, &Registro::default());
    assert_eq!(itens[0].situacao, Situacao::Substituir);
    executar(&itens, &p, direto, |_, _| {});
    assert_ne!(std::fs::read(saida.join("a.stl")).unwrap(), b"antigo");
}

#[test]
fn dois_itens_no_mesmo_destino_nao_se_atropelam() {
    let (dir, saida) = (pasta("colisao"), pasta("colisao saida"));
    std::fs::create_dir_all(dir.join("x")).unwrap();
    step(&dir.join("a.step"), 10.);
    step(&dir.join("x/a.step"), 20.);
    let entradas = vec![EntradaLote::de_arquivo(&dir.join("a.step")), EntradaLote::de_arquivo(&dir.join("x/a.step"))];
    let itens = simular(&entradas, &plano(&saida, &[Formato::Stl], SeExistir::Substituir), &Registro::default());
    assert_eq!(itens[1].situacao, Situacao::Pular(MOTIVO_COLISAO.into()));
    let itens = simular(&entradas, &plano(&saida, &[Formato::Stl], SeExistir::Numerar), &Registro::default());
    assert_eq!(itens[1].situacao, Situacao::Numerado);
    assert_ne!(itens[0].destino, itens[1].destino);
}

#[test]
fn regra_com_pn_e_revisao_nomeia_a_saida() {
    let (dir, saida) = (pasta("regra"), pasta("regra saida"));
    step(&dir.join("placa.step"), 10.);
    let e = EntradaLote { caminho: dir.join("placa.step"), nome: "Placa".into(), pn: "FK-0002".into(), rev: "B".into() };
    let p = plano(&saida, &[Formato::Tmf, Formato::Obj], SeExistir::Pular);
    let r = executar(&simular(&[e], &p, &Registro::default()), &p, direto, |_, _| {});
    assert!(r.iter().all(|x| x.saida.is_ok()), "{r:?}");
    assert_eq!(arquivos(&saida), vec!["FK-0002_B.3mf", "FK-0002_B.obj"]);
}

#[test]
fn obj_leva_o_mtl_junto_quando_a_peca_tem_cor() {
    let (dir, saida) = (pasta("mtl"), pasta("mtl saida"));
    let forma = Shape::box_tool([0., 0., 0.], [1., 0., 0.], [0., 0., 1.], [10.; 3]).unwrap();
    let mut m = ModeloNeutro::de_forma(Formato::Step, "Peça", forma);
    m.xde.produtos[0].cor = Some([0.8, 0., 0., 1.]);
    Registro::default().gravar(&m, &dir.join("a.step"), &OpcoesExportar::default()).unwrap();
    let e = EntradaLote::de_arquivo(&dir.join("a.step"));
    let p = plano(&saida, &[Formato::Obj], SeExistir::Pular);
    let r = executar(&simular(&[e], &p, &Registro::default()), &p, direto, |_, _| {});
    assert!(r[0].saida.is_ok(), "{:?}", r[0].saida);
    assert_eq!(arquivos(&saida), vec!["a.mtl", "a.obj"]);
}

#[test]
fn erro_num_item_nao_para_os_outros_nem_deixa_sobra() {
    let (dir, saida) = (pasta("erro"), pasta("erro saida"));
    step(&dir.join("bom.step"), 10.);
    std::fs::write(dir.join("ruim.step"), "ISO-10303-21;\nlixo").unwrap();
    let entradas = vec![EntradaLote::de_arquivo(&dir.join("ruim.step")), EntradaLote::de_arquivo(&dir.join("bom.step"))];
    let p = plano(&saida, &[Formato::Glb], SeExistir::Pular);
    let mut avisos = 0;
    let r = executar(&simular(&entradas, &p, &Registro::default()), &p, direto, |_, _| avisos += 1);
    assert_eq!(avisos, 2);
    assert!(r[0].saida.is_err() && r[1].saida.is_ok(), "{r:?}");
    assert_eq!(arquivos(&saida), vec!["bom.glb"], "a pasta provisória precisa sumir");
}

#[test]
fn estouro_de_tempo_nao_toca_no_arquivo_antigo() {
    let (dir, saida) = (pasta("tempo"), pasta("tempo saida"));
    step(&dir.join("a.step"), 10.);
    std::fs::write(saida.join("a.stl"), "antigo").unwrap();
    let e = EntradaLote::de_arquivo(&dir.join("a.step"));
    let p = plano(&saida, &[Formato::Stl], SeExistir::Substituir);
    let itens = simular(&[e], &p, &Registro::default());
    let r = executar(
        &itens,
        &p,
        |t| {
            // o filho chegou a gravar parte do arquivo antes de ser encerrado
            if let Trabalho::Converter { destino, .. } = t {
                std::fs::write(destino, "pela metade").unwrap();
            }
            Err(ErroConv::Tempo(1))
        },
        |_, _| {},
    );
    assert_eq!(r[0].saida, Err(ErroConv::Tempo(1)));
    assert_eq!(std::fs::read(saida.join("a.stl")).unwrap(), b"antigo");
    assert_eq!(arquivos(&saida), vec!["a.stl"]);
}

#[test]
fn cancelar_para_o_resto_do_lote() {
    let (dir, saida) = (pasta("cancela"), pasta("cancela saida"));
    let mut entradas = vec![];
    for n in ["a", "b", "c"] {
        step(&dir.join(format!("{n}.step")), 10.);
        entradas.push(EntradaLote::de_arquivo(&dir.join(format!("{n}.step"))));
    }
    let p = plano(&saida, &[Formato::Stl], SeExistir::Pular);
    let itens = simular(&entradas, &p, &Registro::default());
    let mut chamadas = 0;
    let r = executar(
        &itens,
        &p,
        |t| {
            chamadas += 1;
            if chamadas == 2 {
                Err(ErroConv::Cancelado)
            } else {
                direto(t)
            }
        },
        |_, _| {},
    );
    assert_eq!(chamadas, 2, "o terceiro nem começa");
    assert_eq!(r.len(), 3);
    assert!(r[0].saida.is_ok() && r[2].saida == Err(ErroConv::Cancelado));
}

fn peca() -> Document {
    use fenkra_core::sketch::{Curve, SketchData};
    use fenkra_core::{OriginPlane, PlaneRef, Sketch};
    let mut d = Document::new("Disco");
    let mut sk = SketchData::default();
    let mut al = || d.alloc();
    let c = sk.add_point(&mut al, 0., 0.);
    sk.add_curve(&mut al, Curve::Circle { c, r: 10. }, false);
    let id = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data: sk }));
    d.add_feature(FeatureKind::Extrude(Extrude { sketch: id, op: BoolOp::Add, extent: Extent::Blind("5".into()), reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
    d
}

#[test]
fn peca_do_fenkra_entra_no_lote_com_a_forma_recalculada() {
    let (dir, saida) = (pasta("peca"), pasta("peca saida"));
    std::fs::write(dir.join("disco.fkp"), peca().to_json()).unwrap();
    let e = EntradaLote { caminho: dir.join("disco.fkp"), nome: "Disco".into(), pn: "FK-0007".into(), rev: "A".into() };
    let p = plano(&saida, &[Formato::Step, Formato::Arvore], SeExistir::Pular);
    let itens = simular(&[e], &p, &Registro::default());
    assert!(itens.iter().all(|i| i.situacao == Situacao::Converter), "{itens:?}");
    let r = executar(&itens, &p, direto, |_, _| {});
    assert!(r.iter().all(|x| x.saida.is_ok()), "{r:?}");
    let i = Registro::default().importar(&saida.join("FK-0007_A.step"), &OpcoesImportar::default()).unwrap();
    assert!((i.relatorio.volume - std::f64::consts::PI * 500.).abs() < 1e-3, "{}", i.relatorio.volume);
    let receita = Registro::default().importar(&saida.join("FK-0007_A.fkx"), &OpcoesImportar::default()).unwrap();
    assert_eq!(receita.doc.features.len(), 2);
}

#[test]
fn arquivo_sem_receita_nao_vira_arvore() {
    let (dir, saida) = (pasta("receita"), pasta("receita saida"));
    step(&dir.join("a.step"), 10.);
    let itens = simular(&[EntradaLote::de_arquivo(&dir.join("a.step"))], &plano(&saida, &[Formato::Arvore], SeExistir::Pular), &Registro::default());
    assert_eq!(itens[0].situacao, Situacao::Recusado(MOTIVO_SEM_RECEITA.into()));
}
