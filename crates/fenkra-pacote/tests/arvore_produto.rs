// SPDX-License-Identifier: LGPL-2.1-or-later
//! Árvore do produto com miniatura, PN e estado (B364), montada de ponta a ponta: projeto com peças
//! recalculadas, montagem com subconjunto gravada em arquivo, grafo, pacotes `.fkv` por nó e a
//! miniatura tirada de cada pacote.
//!
//! `FENKRA_FKV_SAIDA=<pasta>` grava as miniaturas em PGM com alfa achatado, para ver.

use fenkra_core::{BoolOp, Document, FeatureKind, OriginPlane, PlaneRef, PrimKind, Primitive};
use fenkra_montagem::{Assembly, NodeKind, Placement};
use fenkra_pacote::miniatura::{de_pacote, Miniatura};
use fenkra_pacote::{de_arquivo, de_peca, Opcoes};
use fenkra_project::arvore_produto::{EstadoNo, TipoNo};
use fenkra_project::{DocState, Project};
use fenkra_solidos::Status;
use std::path::PathBuf;

fn pasta(nome: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("fenkra_arvore_produto_{nome}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn caixa(nome: &str, a: &str, b: &str, c: &str) -> Document {
    let s = |x: &str| x.to_string();
    let mut d = Document::new(nome);
    d.add_feature(FeatureKind::Primitive(Primitive { kind: PrimKind::Caixa, plane: PlaneRef::Origin(OriginPlane::XY), a: s(a), b: s(b), c: s(c), d: s("0"), op: BoolOp::Add }));
    d
}

/// Recalcula e confere status, volume e caixa envolvente da peça.
fn conferir(d: &Document, volume: f64, dims: [f64; 3]) {
    let r = fenkra_solidos::recompute(d);
    assert!(r.status.values().all(|s| matches!(s, Status::Ok)), "{}: {:?}", d.name, r.status);
    let v: f64 = r.bodies.iter().map(|b| b.mass.volume).sum();
    assert!((v - volume).abs() < 1e-3, "{}: volume {v}", d.name);
    let (lo, hi) = fenkra_solidos::final_shape(d).unwrap().bbox().unwrap();
    for k in 0..3 {
        assert!((hi[k] - lo[k] - dims[k]).abs() < 1e-3, "{}: caixa {lo:?} {hi:?}", d.name);
    }
}

fn mostrar(nome: &str, m: &Miniatura) {
    let Ok(dir) = std::env::var("FENKRA_FKV_SAIDA") else { return };
    let dir = PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut b = format!("P5\n{} {}\n255\n", m.lado, m.lado).into_bytes();
    b.extend(m.rgba.chunks_exact(4).map(|p| if p[3] > 127 { p[0] } else { 255 }));
    std::fs::write(dir.join(format!("{nome}.pgm")), b).unwrap();
}

#[test]
fn arvore_do_kart_com_miniaturas_pn_e_estado() {
    let d = pasta("kart");
    let mut p = Project::create(&d, "Kart").unwrap();
    // peças: placa larga e baixa, pino alto e fino
    let placa = caixa("Placa", "80", "20", "5");
    let pino = caixa("Pino", "10", "10", "60");
    conferir(&placa, 80. * 20. * 5., [80., 20., 5.]);
    conferir(&pino, 10. * 10. * 60., [10., 10., 60.]);
    let id_placa = p.add_document(placa).unwrap();
    let id_pino = p.add_document(pino).unwrap();
    p.release(id_placa, "liberada").unwrap();
    let rel = |id: u64| p.entry(id).unwrap().file.replace('\\', "/");
    // subconjunto Roda: dois pinos; topo Kart: duas rodas e a placa
    let em = |x: f64| Placement { origin: [x, 0., 0.], ..Placement::default() };
    let mut roda = Assembly::new("Roda");
    roda.add_component("Pino 1", NodeKind::Part { path: rel(id_pino) }, em(0.), None);
    roda.add_component("Pino 2", NodeKind::Part { path: rel(id_pino) }, em(30.), None);
    std::fs::write(d.join("roda.fka"), roda.to_json()).unwrap();
    let mut kart = Assembly::new("Kart");
    kart.add_component("Roda dianteira", NodeKind::SubAssembly { path: "roda.fka".into() }, em(0.), None);
    kart.add_component("Roda traseira", NodeKind::SubAssembly { path: "roda.fka".into() }, em(100.), None);
    kart.add_component("Placa", NodeKind::Part { path: rel(id_placa) }, Placement::default(), None);
    std::fs::write(d.join("kart.fka"), kart.to_json()).unwrap();

    // o grafo: um nó por arquivo, quantidades nas arestas, estado derivado na montagem
    let a = p.arvore_produto();
    let nk = a.no_do_arquivo(&d.join("kart.fka")).unwrap();
    let nr = a.no_do_arquivo(&d.join("roda.fka")).unwrap();
    let (npl, npi) = (a.no_do_documento(id_placa).unwrap(), a.no_do_documento(id_pino).unwrap());
    assert_eq!(a.raizes, vec![nk]);
    assert_eq!(a.nos.len(), 4);
    assert_eq!((a.nos[nk].tipo, a.nos[nr].tipo, a.nos[npi].tipo), (TipoNo::Montagem, TipoNo::Montagem, TipoNo::Peca));
    assert_eq!(a.quantidade_em(nk, npi), 4);
    assert_eq!(a.niveis(nk), 3);
    assert_eq!(a.nos[npl].pn, p.entry(id_placa).unwrap().pn);
    assert!(!a.nos[npl].pn.is_empty());
    assert_eq!(a.nos[npl].estado, EstadoNo::Documento(DocState::Liberado));
    assert_eq!(a.nos[nk].estado, EstadoNo::Derivado { liberadas: 1, total: 2 });
    assert!(a.liberado(npl) && !a.liberado(npi) && !a.liberado(nk));

    // pacote por nó (peça pela versão, montagem pela assinatura) e miniatura de cada pacote
    let mut minis = vec![];
    for (i, n) in a.nos.iter().enumerate() {
        assert!(p.visualizacao_do_no(n).is_none());
        let pacote = match n.id {
            Some(id) => de_peca(&p.load(id).unwrap(), &Opcoes::default()),
            None => de_arquivo(&n.arquivo, &Opcoes { analise: false, ..Opcoes::default() }),
        }
        .unwrap();
        assert!(pacote.avisos().is_empty(), "{}: {:?}", n.nome, pacote.avisos());
        let bytes = pacote.bytes().unwrap();
        let c = p.gravar_visualizacao_no(n, &bytes).unwrap();
        assert_eq!(p.visualizacao_do_no(n), Some(c.clone()));
        let m = de_pacote(&std::fs::read(&c).unwrap(), 64).unwrap();
        assert_eq!(m.rgba.len(), 64 * 64 * 4);
        assert!(m.cobertura() > 0.04, "{}: miniatura quase vazia ({})", n.nome, m.cobertura());
        mostrar(&format!("miniatura_{i}_{}", n.nome), &m);
        minis.push(m);
    }
    // a miniatura tem a forma da peça: placa mais larga que alta, pino mais alto que largo
    let forma = |m: &Miniatura| {
        let (x0, y0, x1, y1) = m.caixa().unwrap();
        (x1 - x0 + 1, y1 - y0 + 1)
    };
    let (lp, ap) = forma(&minis[npl]);
    assert!(lp > ap * 3 / 2, "placa {lp}x{ap}");
    let (li, ai) = forma(&minis[npi]);
    assert!(ai > li * 3 / 2, "pino {li}x{ai}");
    // a peça cabe com margem e fica no meio
    for m in &minis {
        let (x0, y0, x1, y1) = m.caixa().unwrap();
        assert!(x0 >= 2 && y0 >= 2 && x1 <= 61 && y1 <= 61, "{:?}", m.caixa());
    }
    // o tom é só luz: o cinza varia entre faces e não há cor
    let tons: std::collections::BTreeSet<u8> = minis[npl].rgba.chunks_exact(4).filter(|p| p[3] == 255).map(|p| p[0]).collect();
    assert!(tons.len() >= 2, "uma face só iluminada: {tons:?}");
    assert!(minis[npl].rgba.chunks_exact(4).all(|p| p[0] == p[1] && p[1] == p[2]));

    // editar o pino muda a assinatura das montagens de cima: os pacotes delas ficam pendentes
    p.save(id_pino, &caixa("Pino", "10", "10", "70"), "mais alto").unwrap();
    let b = p.arvore_produto();
    assert!(p.visualizacao_do_no(&b.nos[b.no_do_arquivo(&d.join("kart.fka")).unwrap()]).is_none());
    assert!(p.visualizacao_do_no(&b.nos[b.no_do_documento(id_placa).unwrap()]).is_some(), "a placa não mudou");
    let _ = std::fs::remove_dir_all(&d);
}
