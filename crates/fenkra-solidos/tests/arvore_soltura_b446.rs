// SPDX-License-Identifier: LGPL-2.1-or-later
//! B446: soltar um item arrastado em cada tipo de alvo da árvore, no exemplo "Placa paramétrica"
//! (o mesmo do teste com mouse real). Confere a ordem da árvore depois de cada soltura e recalcula:
//! status ok, volume e caixa envolvente iguais aos de antes (organizar não muda a peça), e recusa
//! sem mexer em nada.

mod common;

use common::*;
use fenkra_core::*;
use fenkra_solidos::{example_document, recompute};

fn id_de(d: &Document, nome: &str) -> Id {
    d.features.iter().find(|f| f.name == nome).map(|f| f.id).or_else(|| d.references.iter().find(|r| r.name == nome).map(|r| r.id)).or_else(|| d.params.iter().find(|p| p.name == nome).map(|p| p.id)).or_else(|| d.folders.iter().find(|f| f.name == nome).map(|f| f.id)).unwrap_or_else(|| panic!("sem {nome}"))
}

fn nomes(d: &Document) -> Vec<String> {
    d.features.iter().map(|f| f.name.clone()).collect()
}

/// Recalcula e devolve (volume, caixa); falha com o primeiro comando sem status ok.
fn medir(d: &Document) -> (f64, ([f64; 3], [f64; 3])) {
    let r = recompute(d);
    assert!(problema_de_status(d, &r).is_none(), "{:?}", problema_de_status(d, &r));
    (volume_total(&r), caixa_da_malha(&r))
}

fn igual(a: (f64, ([f64; 3], [f64; 3])), b: (f64, ([f64; 3], [f64; 3])), passo: &str) {
    assert!((a.0 - b.0).abs() <= 1e-6 * a.0.abs().max(1.), "{passo}: volume {} → {}", a.0, b.0);
    for i in 0..3 {
        assert!((a.1 .0[i] - b.1 .0[i]).abs() < 1e-3 && (a.1 .1[i] - b.1 .1[i]).abs() < 1e-3, "{passo}: caixa mudou");
    }
}

#[test]
fn cada_soltura_da_arvore_mantem_a_peca_e_a_ordem_valida() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = example_document();
    let base = medir(&d);
    assert!(base.0 > 0.);
    let ordem0 = nomes(&d);

    // 1. recurso na pasta de comandos do mesmo corpo: entra, encostado no Filete, sem mudar a ordem
    let furos = id_de(&d, "Furos M6");
    let acab = id_de(&d, "Acabamentos");
    d.soltar_em_pasta(furos, acab).unwrap();
    assert_eq!(d.feature(furos).unwrap().folder, Some(acab));
    assert_eq!(nomes(&d), ordem0, "Furos M6 já vinha logo antes do primeiro membro");
    igual(base, medir(&d), "pasta de comandos");

    // 2. Placa na mesma pasta: o sketch Centros dos furos fica antes da pasta (Placa encosta em Furos)
    let placa = id_de(&d, "Placa");
    d.soltar_em_pasta(placa, acab).unwrap();
    let n = nomes(&d);
    let pos = |s: &str| n.iter().position(|x| x == s).unwrap();
    assert!(pos("Centros dos furos") < pos("Placa") && pos("Placa") + 1 == pos("Furos M6"), "{n:?}");
    igual(base, medir(&d), "pasta de comandos, de mais longe");

    // 3. o sketch base, que a Placa usa, também pode entrar: fica logo antes da Placa
    let sk = id_de(&d, "Sketch base");
    d.soltar_em_pasta(sk, acab).unwrap();
    igual(base, medir(&d), "sketch na pasta de comandos");

    // 4. parâmetro sobre parâmetro: L desce para depois de margem
    let l = id_de(&d, "L");
    let margem = id_de(&d, "margem");
    d.mover_parametro(l, margem).unwrap();
    assert_eq!(d.params.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(), vec!["W", "esp", "margem", "L"]);
    igual(base, medir(&d), "parâmetros reordenados");

    // 5. plano sobre o corpo: recusa e nada muda
    let antes = d.clone();
    let corpo = d.collectors[0].id;
    let topo = id_de(&d, "PlanoTopo");
    assert_eq!(d.soltar_no_corpo(topo, corpo), Err(RecusaSoltura::Referencia));
    assert_eq!(d.soltar_no_corpo(furos, corpo), Err(RecusaSoltura::JaNoCorpo(d.collectors[0].name.clone())));
    // a base não sai do corpo deixando os furos e o filete sem sólido
    let corpo2 = d.add_collector("Corpo2");
    assert!(matches!(d.soltar_no_corpo(placa, corpo2), Err(RecusaSoltura::UsadoNoCorpo(_))));
    // os furos não vão para um corpo sem sólido
    assert!(matches!(d.soltar_no_corpo(furos, corpo2), Err(RecusaSoltura::SemSolidoNoCorpo(_))));
    d.collectors.retain(|c| c.id != corpo2);
    assert!(d.features == antes.features && d.references == antes.references && d.params == antes.params && d.folders == antes.folders, "recusa não mexe no documento");

    // 6. recurso sobre corpo que aceita: um segundo corpo com a sua caixa recebe o sketch dos furos
    let corpo2 = d.add_collector("Corpo2");
    let sk2 = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [200., 0.], [220., 20.]);
    let cx = d.add_feature(ext(sk2, BoolOp::Add, "5"));
    d.feature_mut(sk2).unwrap().target = corpo2;
    d.feature_mut(cx).unwrap().target = corpo2;
    let centros = id_de(&d, "Centros dos furos");
    d.soltar_no_corpo(centros, corpo2).unwrap();
    assert_eq!(d.feature(centros).unwrap().target, corpo2);
    let r = recompute(&d);
    assert!(problema_de_status(&d, &r).is_none(), "{:?}", problema_de_status(&d, &r));
    assert!((volume_total(&r) - (base.0 + 20. * 20. * 5.)).abs() < 1e-3 * base.0, "os furos continuam na placa");

    // 7. pasta de comandos vazia aceita; pasta de geometria recusa comando sólido
    let vazia = d.add_folder("Vazia");
    d.soltar_em_pasta(cx, vazia).unwrap();
    assert_eq!(d.corpo_da_pasta(vazia), Some(corpo2));
    let geo = d.default_geometry_folder();
    assert_eq!(d.soltar_em_pasta(furos, geo), Err(RecusaSoltura::SoGeometriaLivre));
    // Furos M6 não entra na pasta do outro corpo
    assert!(matches!(d.soltar_em_pasta(furos, vazia), Err(RecusaSoltura::PastaDeOutroCorpo(_))));
    let r = recompute(&d);
    assert!(problema_de_status(&d, &r).is_none());
}
