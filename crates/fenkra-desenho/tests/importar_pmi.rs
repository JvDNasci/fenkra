// SPDX-License-Identifier: LGPL-2.1-or-later
//! B371: importar as anotações do 3D para a vista. Reconhece o que vem e como, deixa de fora o que
//! não cabe, cria só o que foi escolhido e atualiza com o modelo.

use fenkra_core::pmi::{Pmi, PmiKind, WeldExtra, WeldKind};
use fenkra_core::Id;
use fenkra_desenho::importar::{self, Como, Fora};
use fenkra_desenho::source::Models;
use fenkra_desenho::*;

fn pmi(id: u64, nome: &str, kind: PmiKind, anchors: Vec<[f64; 3]>, text: &str) -> Pmi {
    Pmi { id: Id(id), name: nome.into(), kind, anchors, offset: [0.; 3], text: text.into(), tol_plus: String::new(), tol_minus: String::new(), visible: true, plane: None, style: Default::default(), folder: String::new(), targets: vec![] }
}

fn doc_com_pmi(comprimento: f64) -> fenkra_core::Document {
    let mut d = fenkra_solidos::example_document();
    d.pmi.push(pmi(9001, "Comprimento", PmiKind::Linear, vec![[0., 0., 0.], [comprimento, 0., 0.]], ""));
    // ao longo da direção de visão da frente: não aparece nela
    d.pmi.push(pmi(9002, "Profundidade", PmiKind::Linear, vec![[0., 0., 0.], [0., 50., 0.]], ""));
    d.pmi.push(pmi(9003, "Nota curta", PmiKind::Note, vec![[10., 0., 5.]], "Rebarbar"));
    d.pmi.push(pmi(9004, "Bloco", PmiKind::Note, vec![[10., 0., 5.]], &"linha\n".repeat(8)));
    d.pmi.push(pmi(9005, "Legenda", PmiKind::TitleBlock { template: "{nome}".into() }, vec![[0.; 3]], ""));
    d.pmi.push(pmi(9006, "Solda", PmiKind::Weld { kind: WeldKind::Filete, size: "a5".into(), all_around: false, extra: WeldExtra::default() }, vec![[20., 0., 0.]], ""));
    d.pmi.push(pmi(9007, "Ordenada", PmiKind::Ordinate { axis: 0 }, vec![[0.; 3], [30., 0., 0.]], ""));
    d
}

fn montar(nome: &str, doc: &fenkra_core::Document) -> (Drawing, Models, Id, std::path::PathBuf) {
    let dir = std::env::temp_dir().join(format!("{nome}_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("p.fkp"), doc.to_json()).unwrap();
    let src = Source { path: "p.fkp".into(), display_state: String::new(), hidden: vec![] };
    let mut d = Drawing::new("t", Format::A3);
    let s = d.sheets[0].id;
    let v = d.add_view(s, src, ViewKind::Standard(Dir::Frente), [150., 150.], 1.);
    (d, Models::new(Some(dir.clone())), v, dir)
}

#[test]
fn reconhece_o_que_vem_e_o_que_fica_de_fora() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let (d, mut m, v, dir) = montar("fenkra_importar_reconhece", &doc_com_pmi(120.));
    let l = importar::reconhecer(&d, &mut m, v);
    let de = |id: u64| l.iter().find(|c| c.pmi == Id(id)).unwrap().destino.clone();
    assert_eq!(de(9001), Ok(Como::Cota(DimKind::Horizontal)));
    assert_eq!(de(9002), Err(Fora::NaoApareceNaVista));
    assert_eq!(de(9003), Ok(Como::Nota(NoteKind::Text)));
    assert_eq!(de(9004), Err(Fora::BlocoDeNotas));
    assert_eq!(de(9005), Err(Fora::Legenda));
    assert_eq!(de(9006), Ok(Como::Nota(NoteKind::Weld)));
    assert_eq!(de(9007), Err(Fora::SemEquivalente));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn importa_so_as_escolhidas_ligadas_ao_modelo_e_atualiza_com_ele() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let (mut d, mut m, v, dir) = montar("fenkra_importar_atualiza", &doc_com_pmi(120.));
    // a pessoa desmarca a nota curta e tenta marcar o bloco (recusado)
    let rel = importar::importar(&mut d, &mut m, v, &[Id(9001), Id(9006), Id(9004)]);
    assert_eq!((rel.cotas.len(), rel.notas.len()), (1, 1));
    assert_eq!(rel.recusadas, vec![(Id(9004), Fora::BlocoDeNotas)]);
    assert_eq!(d.notes.len(), 1);
    assert!(d.notes[0].weld.is_some() && d.notes[0].anchor3.is_some(), "solda ligada ao modelo");
    let p0 = m.proj(&d, v);
    let ax = p0.axes.unwrap();
    let medida = |d: &Drawing, m: &mut Models| {
        let p = m.proj(d, v);
        layout::dim_value(&d.dims[0], d.view(v).unwrap(), &p).map(|x| x.0).unwrap_or_else(|| ax.project([0.; 3])[0])
    };
    assert!((medida(&d, &mut m) - 120.).abs() < 1e-6);
    // importar de novo não duplica
    let l = importar::reconhecer(&d, &mut m, v);
    assert_eq!(l.iter().find(|c| c.pmi == Id(9001)).unwrap().destino, Err(Fora::JaImportada));

    // o modelo muda: a cota passa a 150 mm e a solda anda
    std::thread::sleep(std::time::Duration::from_millis(30));
    let mut novo = doc_com_pmi(150.);
    novo.pmi.iter_mut().find(|p| p.id == Id(9006)).unwrap().anchors = vec![[40., 0., 0.]];
    std::fs::write(dir.join("p.fkp"), novo.to_json()).unwrap();
    assert!(importar::atualizar(&mut d, &mut m).is_empty());
    assert!((medida(&d, &mut m) - 150.).abs() < 1e-6, "a cota acompanha o 3D");
    assert_eq!(d.notes[0].anchor3, Some([40., 0., 0.]));

    // a anotação some do modelo: a cota importada é acusada como órfã
    std::thread::sleep(std::time::Duration::from_millis(30));
    let mut sem = doc_com_pmi(150.);
    sem.pmi.retain(|p| p.id != Id(9001));
    std::fs::write(dir.join("p.fkp"), sem.to_json()).unwrap();
    assert_eq!(importar::atualizar(&mut d, &mut m), vec![d.dims[0].id]);
    let _ = std::fs::remove_dir_all(dir);
}
