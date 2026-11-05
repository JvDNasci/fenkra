// SPDX-License-Identifier: LGPL-2.1-or-later
//! B229: documento importado com selo de origem, cofre por hash e reimportação.

use fenkra_core::*;
use fenkra_project::importacao::Reimportacao;
use fenkra_project::Project;
use fenkra_solidos::importacao::importar_step;
use fenkra_solidos::{export, recompute, Status};
use std::fs;
use std::path::{Path, PathBuf};

fn pasta(teste: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("Fenkra importação {} {teste}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    p
}

/// Grava um bloco `x` por 30 por 20 como STEP.
fn step_de_bloco(caminho: &Path, x: f64) {
    let mut d = Document::new("origem");
    let mut data = SketchData::default();
    let mut al = || d.alloc();
    data.add_rect(&mut al, [0., 0.], [x, 30.], false);
    let s = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
    d.add_feature(FeatureKind::Extrude(Extrude { sketch: s, op: BoolOp::Add, extent: Extent::Blind("20".into()), reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0 }));
    export(&d, None, caminho.to_str().unwrap()).unwrap();
}

fn volume(d: &Document) -> f64 {
    let r = recompute(d);
    for f in &d.features {
        assert_eq!(r.status[&f.id], Status::Ok, "{}", f.name);
    }
    r.bodies.iter().map(|b| b.mass.volume).sum()
}

#[test]
fn importado_mantem_selo_hash_e_volume_depois_de_reabrir() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let dir = pasta("reabrir");
    let arquivo = dir.join("peça de fora.step");
    step_de_bloco(&arquivo, 40.);
    let i = importar_step(arquivo.to_str().unwrap()).unwrap();
    let selo = i.doc.origem.clone().unwrap();
    assert_eq!(selo.nome, "peça de fora.step");
    assert_eq!(selo.hash, blake3::hash(&fs::read(&arquivo).unwrap()).to_hex().to_string());
    assert!(selo.esquema.contains("AUTOMOTIVE_DESIGN"), "{}", selo.esquema);
    assert!(!selo.unidades.is_empty() && selo.data > 0);

    let raiz = dir.join("projeto");
    let id = {
        let mut p = Project::create(&raiz, "P").unwrap();
        p.adicionar_importado(&arquivo, i.doc, &i.relatorio).unwrap()
    };

    let p = Project::open(&raiz).unwrap();
    let d = p.load(id).unwrap();
    assert_eq!(d.origem.as_ref(), Some(&selo));
    assert!((volume(&d) - 24000.).abs() < 1e-3);
    assert_eq!(p.entry(id).unwrap().origem.as_ref(), Some(&selo), "o painel lê o selo do índice");
    assert_eq!(fs::read(p.original_no_cofre(&selo.hash, "step")).unwrap(), fs::read(&arquivo).unwrap());
    assert!(fs::read_to_string(p.relatorio_no_cofre(&selo.hash)).unwrap().contains("Faces: 6"));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn reimportar_detecta_mudanca_e_atualiza_mantendo_comandos() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let dir = pasta("atualizar");
    let arquivo = dir.join("placa.stp");
    step_de_bloco(&arquivo, 40.);
    let i = importar_step(arquivo.to_str().unwrap()).unwrap();
    let hash_velho = i.doc.origem.as_ref().unwrap().hash.clone();
    let mut p = Project::create(&dir.join("projeto"), "P").unwrap();
    let id = p.adicionar_importado(&arquivo, i.doc, &i.relatorio).unwrap();
    assert_eq!(p.reimportacao("placa.stp", &hash_velho), Reimportacao::Igual { id });
    assert_eq!(p.reimportacao("outra.stp", &hash_velho), Reimportacao::Nova);

    // um comando feito depois da importação tem de sobreviver à atualização
    let mut d = p.load(id).unwrap();
    d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data: SketchData::default() }));
    let comandos = d.features.len();
    p.save(id, &d, "edição").unwrap();

    step_de_bloco(&arquivo, 50.);
    let nova = importar_step(arquivo.to_str().unwrap()).unwrap();
    let hash_novo = nova.doc.origem.as_ref().unwrap().hash.clone();
    assert_ne!(hash_novo, hash_velho);
    assert_eq!(p.reimportacao("placa.stp", &hash_novo), Reimportacao::Mudou { id, hash_anterior: hash_velho.clone() });

    p.atualizar_importado(id, &arquivo, &nova.doc, &nova.relatorio).unwrap();
    let d = p.load(id).unwrap();
    assert_eq!(d.features.len(), comandos);
    assert_eq!(d.origem.as_ref().unwrap().hash, hash_novo);
    let r = recompute(&d);
    assert!((r.bodies[0].mass.volume - 30000.).abs() < 1e-3);
    assert!(p.original_no_cofre(&hash_velho, "stp").is_file(), "o original antigo fica no cofre");
    assert!(p.original_no_cofre(&hash_novo, "stp").is_file());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn derivado_aponta_para_a_base_e_nao_muda_o_documento_antigo() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let dir = pasta("derivado");
    let arquivo = dir.join("placa.step");
    step_de_bloco(&arquivo, 40.);
    let i = importar_step(arquivo.to_str().unwrap()).unwrap();
    let mut p = Project::create(&dir.join("projeto"), "P").unwrap();
    let base = p.adicionar_importado(&arquivo, i.doc, &i.relatorio).unwrap();

    step_de_bloco(&arquivo, 50.);
    let nova = importar_step(arquivo.to_str().unwrap()).unwrap();
    let derivado = p.adicionar_derivado(base, &arquivo, nova.doc, &nova.relatorio).unwrap();

    assert_ne!(base, derivado);
    assert!((volume(&p.load(base).unwrap()) - 24000.).abs() < 1e-3);
    assert!((volume(&p.load(derivado).unwrap()) - 30000.).abs() < 1e-3);
    let uid_base = p.entry(base).unwrap().uid.clone();
    assert_eq!(p.entry(derivado).unwrap().origem.as_ref().unwrap().derivado_de, Some(uid_base));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn importar_recusa_o_que_nao_e_step() {
    let dir = pasta("recusa");
    let falso = dir.join("peça.obj");
    fs::write(&falso, "o").unwrap();
    assert!(importar_step(falso.to_str().unwrap()).is_err());
    assert!(importar_step(dir.join("não existe.step").to_str().unwrap()).is_err());
    fs::remove_dir_all(dir).unwrap();
}

/// Soma ao documento importado um segundo corpo com a mesma geometria, como um STEP de duas peças.
fn com_segundo_corpo(mut d: Document) -> Document {
    let geo = d.references.iter().find_map(|r| match &r.kind {
        RefKind::Isolated { geo } => Some(geo.clone()),
        _ => None,
    });
    let coletor = d.add_collector("Outro");
    d.active_collector = coletor;
    let alvo = d.add_link("importação", "x", "outro", geo.unwrap());
    let ext = d.external.last().map(|e| e.id).unwrap();
    d.isolate_link(ext);
    d.add_feature(FeatureKind::LinkedBody(LinkedBody { geo: alvo }));
    d
}

#[test]
fn atualizar_troca_todos_os_corpos_e_recusa_contagem_diferente() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let dir = pasta("corpos");
    let arquivo = dir.join("par.step");
    step_de_bloco(&arquivo, 40.);
    let i = importar_step(arquivo.to_str().unwrap()).unwrap();
    let mut p = Project::create(&dir.join("projeto"), "P").unwrap();
    let id = p.adicionar_importado(&arquivo, com_segundo_corpo(i.doc), &i.relatorio).unwrap();
    assert!((volume(&p.load(id).unwrap()) - 48000.).abs() < 1e-3);

    step_de_bloco(&arquivo, 50.);
    let nova = importar_step(arquivo.to_str().unwrap()).unwrap();
    p.atualizar_importado(id, &arquivo, &com_segundo_corpo(nova.doc.clone()), &nova.relatorio).unwrap();
    assert!((volume(&p.load(id).unwrap()) - 60000.).abs() < 1e-3, "os dois corpos foram trocados");

    let e = p.atualizar_importado(id, &arquivo, &nova.doc, &nova.relatorio).unwrap_err();
    assert!(e.to_string().contains("crie um derivado"), "{e}");
    assert!((volume(&p.load(id).unwrap()) - 60000.).abs() < 1e-3, "recusar não muda o documento");
    fs::remove_dir_all(dir).unwrap();
}
