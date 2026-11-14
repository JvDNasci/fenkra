// SPDX-License-Identifier: LGPL-2.1-or-later
//! B452: Salvar como, pacote e onde-usado seguem os links reais de peça (`external[]` com `path` e
//! `uid`), a malha importada, as texturas e o furo que a montagem manda.
//!
//! Cenário montado em código, com os documentos de verdade gravados pelo formato de sempre:
//!
//! ```text
//! conj.fka ── Part ──> pecas/placa.fkp ── link (valor) ──────> pecas/mestre.fkp
//!     │                    │  └─ link (corpo derivado) ──────> pecas/mestre.fkp
//!     │                    │  └─ furo mandado pela montagem ─> conj.fka
//!     │                    └─ textura ───────────────────────> pecas/textura.png
//!     └─── Part ──> pecas/casca.fkp ── malha importada ──────> pecas/casca.stl
//! ```
//!
//! A montagem não cita o mestre: ele só é alcançado pelo link da placa, que era o que se perdia.

use crate::pacote::{empacotar, OpcoesPacote};
use crate::relacoes::{diagnosticar, onde_usado, referencias_de, TipoRef, Veredito};
use crate::salvar_como::{AcaoRef, ModoSalvarComo, PlanoSalvarComo};
use fenkra_core::{BoolOp, Document, FeatureKind, LinkedGeo, OriginPlane, PlaneRef, PrimKind, Primitive, TextureRef};
use std::path::{Path, PathBuf};

fn pasta(nome: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("fenkra-b452-{nome}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(p.join("pecas")).unwrap();
    p
}

fn caixa(nome: &str, a: &str) -> Document {
    let s = |x: &str| x.to_string();
    let mut d = Document::new(nome);
    d.add_feature(FeatureKind::Primitive(Primitive { kind: PrimKind::Caixa, plane: PlaneRef::Origin(OriginPlane::XY), a: s(a), b: s("20"), c: s("5"), d: s("0"), op: BoolOp::Add }));
    d
}

struct Cenario {
    d: PathBuf,
    conj: PathBuf,
    placa: PathBuf,
    mestre: PathBuf,
    uid_mestre: String,
}

fn montar(nome: &str) -> Cenario {
    let d = pasta(nome);
    let mestre = caixa("Mestre", "40");
    std::fs::write(d.join("pecas/mestre.fkp"), mestre.to_json()).unwrap();

    let mut placa = caixa("Placa", "60");
    placa.add_link("Mestre", "mestre.fkp", "Largura", LinkedGeo::Value(40.));
    placa.external.last_mut().unwrap().uid = mestre.uid.0.clone();
    placa.add_link("Mestre", "mestre.fkp", "Corpo", LinkedGeo::Surface(String::new()));
    placa.external.last_mut().unwrap().uid = mestre.uid.0.clone();
    placa.add_link("Conj", "../conj.fka", "montagem:furo:1:1:0", LinkedGeo::Value(0.));
    placa.textures.push(TextureRef { name: "Grão".into(), path: "textura.png".into(), scale: 40. });
    std::fs::write(d.join("pecas/textura.png"), [0x89u8, b'P', b'N', b'G', 0, 1, 2, 3]).unwrap();
    std::fs::write(d.join("pecas/placa.fkp"), placa.to_json()).unwrap();

    let mut casca = Document::new("Casca");
    casca.add_feature(FeatureKind::MeshImport(fenkra_core::MeshImport { path: "casca.stl".into(), solid: true, ..Default::default() }));
    std::fs::write(d.join("pecas/casca.stl"), b"solid x\nendsolid x\n").unwrap();
    std::fs::write(d.join("pecas/casca.fkp"), casca.to_json()).unwrap();

    // montagem no formato gravado pela bancada de montagem
    let conj = d.join("conj.fka");
    std::fs::write(
        &conj,
        r#"{"format_version": 1, "uid": "uid-conj", "name": "Conj", "nodes": [
            {"id": 1, "name": "Placa", "kind": {"Part": {"path": "pecas/placa.fkp"}}},
            {"id": 2, "name": "Casca", "kind": {"Part": {"path": "pecas/casca.fkp"}}}]}"#,
    )
    .unwrap();
    Cenario { placa: d.join("pecas/placa.fkp"), mestre: d.join("pecas/mestre.fkp"), uid_mestre: mestre.uid.0.clone(), conj, d }
}

/// Recalcula e confere status, volume e caixa envolvente da placa (60 x 20 x 5).
fn conferir_placa(arquivo: &Path) -> Document {
    let doc = Document::from_json(&std::fs::read_to_string(arquivo).unwrap()).unwrap();
    let r = fenkra_solidos::recompute(&doc);
    let f = &doc.features[0];
    assert_eq!(r.status[&f.id], fenkra_solidos::Status::Ok, "{:?}", r.status);
    assert_eq!(r.bodies.len(), 1);
    assert!((r.bodies[0].mass.volume - 6000.).abs() < 1e-3, "volume {}", r.bodies[0].mass.volume);
    let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
    for p in r.bodies[0].mesh.positions.chunks_exact(3) {
        for k in 0..3 {
            lo[k] = lo[k].min(p[k]);
            hi[k] = hi[k].max(p[k]);
        }
    }
    let mut ext: Vec<f32> = (0..3).map(|k| hi[k] - lo[k]).collect();
    ext.sort_by(|a, b| a.partial_cmp(b).unwrap());
    for (e, esperado) in ext.iter().zip([5f32, 20., 60.]) {
        assert!((e - esperado).abs() < 1e-3, "caixa {ext:?}");
    }
    doc
}

fn chave(p: &Path) -> PathBuf {
    std::fs::canonicalize(p).unwrap()
}

#[test]
fn leitura_acha_todos_os_tipos_de_link() {
    let c = montar("leitura");
    let refs = referencias_de(&c.placa);
    let tipos: Vec<(TipoRef, &str)> = refs.iter().map(|r| (r.tipo, r.caminho.as_str())).collect();
    assert_eq!(tipos.iter().filter(|t| **t == (TipoRef::Link, "mestre.fkp")).count(), 2, "link de valor e corpo derivado: {tipos:?}");
    assert!(refs.iter().filter(|r| r.tipo == TipoRef::Link).all(|r| r.uid == c.uid_mestre), "o uid vem junto");
    assert!(tipos.contains(&(TipoRef::FuroMontagem, "../conj.fka")));
    assert!(tipos.contains(&(TipoRef::Textura, "textura.png")));
    let casca = referencias_de(&c.d.join("pecas/casca.fkp"));
    assert_eq!((casca.len(), casca[0].tipo, casca[0].caminho.as_str(), casca[0].no), (1, TipoRef::Malha, "casca.stl", None));
    // o uid do próprio documento não vira referência
    assert!(referencias_de(&c.mestre).is_empty());
    let _ = std::fs::remove_dir_all(&c.d);
}

#[test]
fn onde_usado_acha_quem_usa_por_link_e_a_montagem() {
    let c = montar("usado");
    let usos = onde_usado(&[c.d.clone()], &c.mestre, &c.uid_mestre);
    assert!(!usos.is_empty() && usos.iter().all(|u| u.tipo == TipoRef::Link && u.arquivo.file_name().unwrap() == "placa.fkp"), "{usos:?}");
    // arquivo movido: o uid do link ainda acha a placa
    let movido = c.d.join("outro/mestre.fkp");
    std::fs::create_dir_all(movido.parent().unwrap()).unwrap();
    std::fs::rename(&c.mestre, &movido).unwrap();
    assert!(onde_usado(&[c.d.clone()], &movido, &c.uid_mestre).iter().any(|u| u.arquivo.file_name().unwrap() == "placa.fkp"));
    std::fs::rename(&movido, &c.mestre).unwrap();
    // a placa é usada pela montagem, e a montagem é citada pelo furo da placa (tipo próprio)
    let usos = onde_usado(&[c.d.clone()], &c.placa, "");
    assert_eq!(usos.len(), 1);
    assert_eq!((usos[0].tipo, usos[0].no, usos[0].arquivo.file_name().unwrap().to_str()), (TipoRef::Peca, Some(1), Some("conj.fka")));
    let furo = onde_usado(&[c.d.clone()], &c.conj, "");
    assert!(furo.iter().any(|u| u.tipo == TipoRef::FuroMontagem && u.arquivo.file_name().unwrap() == "placa.fkp"));
    // malha e textura também têm onde-usado
    assert_eq!(onde_usado(&[c.d.clone()], &c.d.join("pecas/casca.stl"), "")[0].tipo, TipoRef::Malha);
    assert_eq!(onde_usado(&[c.d.clone()], &c.d.join("pecas/textura.png"), "")[0].tipo, TipoRef::Textura);
    let _ = std::fs::remove_dir_all(&c.d);
}

#[test]
fn salvar_como_copia_a_peca_linkada_e_os_recursos() {
    let c = montar("sc-copia");
    let dest = c.d.join("v2/conj.fka");
    let mut plano = PlanoSalvarComo::planejar(&c.conj, None, &dest, ModoSalvarComo::CopiaAbrir).unwrap();
    let linhas: Vec<(String, TipoRef, u32)> = plano.linhas.iter().map(|l| (l.nome_destino.clone(), l.tipo, l.nivel)).collect();
    assert!(linhas.contains(&("mestre.fkp".into(), TipoRef::Link, 2)), "a peça linkada aparece na tabela: {linhas:?}");
    assert!(linhas.contains(&("textura.png".into(), TipoRef::Textura, 2)));
    assert!(linhas.contains(&("casca.stl".into(), TipoRef::Malha, 2)));
    assert!(!linhas.iter().any(|l| l.1 == TipoRef::FuroMontagem), "a montagem citada pelo furo não é dependência");
    assert_eq!(plano.linhas.iter().find(|l| l.nome_destino == "mestre.fkp").unwrap().usos, 2, "os dois links contam");
    plano.marcar_todas(AcaoRef::Copiar);
    let r = plano.executar().unwrap();
    assert_eq!(r.gravados.len(), 6, "{:?}", r.gravados);
    let v2 = c.d.join("v2");
    for f in ["placa.fkp", "mestre.fkp", "casca.fkp", "casca.stl", "textura.png"] {
        assert!(v2.join(f).is_file(), "{f}");
    }
    assert_eq!(std::fs::read(v2.join("casca.stl")).unwrap(), b"solid x\nendsolid x\n", "a malha vai byte a byte");

    let placa = conferir_placa(&v2.join("placa.fkp"));
    let mestre = Document::from_json(&std::fs::read_to_string(v2.join("mestre.fkp")).unwrap()).unwrap();
    assert_ne!(mestre.uid.0, c.uid_mestre, "a cópia do mestre é outro documento");
    for l in placa.external.iter().filter(|e| e.source == "Mestre") {
        assert_eq!((l.path.as_str(), l.uid.as_str()), ("mestre.fkp", mestre.uid.0.as_str()), "o link aponta para a cópia, com o uid dela");
    }
    let furo = placa.external.iter().find(|e| e.publication.starts_with("montagem:furo:")).unwrap();
    assert_eq!(chave(&v2.join(&furo.path)), chave(&dest), "o furo segue a cópia da montagem");
    assert_eq!(chave(Path::new(&placa.textures[0].path)), chave(&v2.join("textura.png")), "textura com caminho absoluto da cópia");
    let casca = Document::from_json(&std::fs::read_to_string(v2.join("casca.fkp")).unwrap()).unwrap();
    let FeatureKind::MeshImport(m) = &casca.features[0].kind else { panic!() };
    assert_eq!(chave(Path::new(&m.path)), chave(&v2.join("casca.stl")));
    let diag = diagnosticar(&v2.join("placa.fkp"), &[c.d.clone()]);
    assert!(diag.iter().all(|x| x.veredito == Veredito::Ok), "{diag:?}");
    // os originais ficaram como estavam
    conferir_placa(&c.placa);
    assert!(std::fs::read_to_string(&c.placa).unwrap().contains(&c.uid_mestre));
    let _ = std::fs::remove_dir_all(&c.d);
}

#[test]
fn salvar_como_mantendo_aponta_para_a_peca_linkada_original() {
    let c = montar("sc-manter");
    let dest = c.d.join("outra/pasta/placa.fkp");
    let plano = PlanoSalvarComo::planejar(&c.placa, None, &dest, ModoSalvarComo::CopiaContinuar).unwrap();
    assert!(plano.linhas.iter().any(|l| l.tipo == TipoRef::Link && l.acao == AcaoRef::Manter));
    plano.executar().unwrap();
    let placa = conferir_placa(&dest);
    for l in placa.external.iter().filter(|e| e.source == "Mestre") {
        assert_eq!(l.path, "../../pecas/mestre.fkp");
        assert_eq!(l.uid, c.uid_mestre, "mantido: continua o mesmo documento");
    }
    assert_eq!(chave(Path::new(&placa.textures[0].path)), chave(&c.d.join("pecas/textura.png")));
    let diag = diagnosticar(&dest, &[c.d.clone()]);
    assert!(diag.iter().all(|x| x.veredito == Veredito::Ok), "{diag:?}");
    // mover o mestre atualiza o link da placa (achada pelo onde-usado)
    let mut mover = PlanoSalvarComo::planejar(&c.mestre, None, &c.d.join("comum/mestre.fkp"), ModoSalvarComo::RenomearMover).unwrap();
    mover.pastas_dependentes = vec![c.d.clone()];
    let r = mover.executar().unwrap();
    assert!(r.dependentes_atualizados.iter().any(|p| p.file_name().unwrap() == "placa.fkp"), "{:?}", r.dependentes_atualizados);
    let placa = conferir_placa(&c.placa);
    assert!(placa.external.iter().filter(|e| e.source == "Mestre").all(|e| e.path == "../comum/mestre.fkp"));
    let _ = std::fs::remove_dir_all(&c.d);
}

#[test]
fn pacote_inclui_a_peca_linkada_e_os_recursos() {
    let c = montar("pacote");
    let destino = c.d.join("_pacote");
    let res = empacotar(&c.conj, &destino, &OpcoesPacote { incluir_desenhos: false, ..Default::default() }).unwrap();
    assert!(res.verificacao.ok(), "{:?}", res.verificacao);
    let papeis: Vec<(&str, &str)> = res.manifesto.entradas.iter().map(|e| (e.pacote.as_str(), e.papel.as_str())).collect();
    assert!(papeis.contains(&("pecas/mestre.fkp", "linkado")), "{papeis:?}");
    assert!(papeis.contains(&("pecas/textura.png", "recurso")));
    assert!(papeis.contains(&("pecas/casca.stl", "recurso")));
    assert!(res.manifesto.externas.is_empty() && res.manifesto.ausentes.is_empty(), "{:?}", res.manifesto.externas);
    let placa = conferir_placa(&destino.join("pecas/placa.fkp"));
    assert!(placa.external.iter().filter(|e| e.source == "Mestre").all(|e| e.path == "mestre.fkp"));
    assert_eq!(placa.external.iter().find(|e| e.publication.starts_with("montagem:furo:")).unwrap().path, "../conj.fka");
    assert_eq!(placa.textures[0].path, "textura.png");
    // sem a opção, o linkado fica de fora e o link aponta para o original
    let destino2 = c.d.join("_pacote2");
    let res = empacotar(&c.conj, &destino2, &OpcoesPacote { incluir_desenhos: false, incluir_linkados: false, ..Default::default() }).unwrap();
    assert!(res.verificacao.ok(), "{:?}", res.verificacao);
    assert_eq!(res.manifesto.excluidas.len(), 1);
    assert!(!destino2.join("pecas/mestre.fkp").exists());
    // empacotar só a placa não traz a montagem do furo: ela vira referência externa
    let destino3 = c.d.join("_pacote3");
    let res = empacotar(&c.placa, &destino3, &OpcoesPacote { incluir_desenhos: false, ..Default::default() }).unwrap();
    assert!(res.verificacao.ok(), "{:?}", res.verificacao);
    assert!(!res.manifesto.entradas.iter().any(|e| e.pacote.ends_with("conj.fka")));
    assert_eq!(res.manifesto.externas.len(), 1);
    let _ = std::fs::remove_dir_all(&c.d);
}
