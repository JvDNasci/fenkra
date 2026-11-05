// SPDX-License-Identifier: LGPL-2.1-or-later
//! B87: importação pela ponte XDE até o documento recalculado.

use fenkra_conversor::{ErroConv, Formato, ModeloNeutro, OpcoesExportar, OpcoesImportar, Registro, Reparo, UnidadeArquivo, UnidadeSaida};
use fenkra_core::Document;
use fenkra_kernel::{InstanciaXde, ModeloXde, ProdutoXde, Shape, IDENTIDADE};
use std::path::{Path, PathBuf};

fn pasta(teste: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("Fenkra conversor açaí {} {teste}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

fn caixa(l: [f64; 3]) -> Shape {
    Shape::box_tool([0., 0., 0.], [1., 0., 0.], [0., 0., 1.], l).unwrap()
}

fn peca(nome: &str, forma: Shape, cor: [f32; 4]) -> ProdutoXde {
    ProdutoXde { nome: nome.into(), forma: Some(forma), cor: Some(cor), ..Default::default() }
}

fn em(x: f64, y: f64) -> [f64; 12] {
    [1., 0., 0., x, 0., 1., 0., y, 0., 0., 1., 0.]
}

/// Base e dois pinos iguais, com a mesma definição nas duas posições.
fn montagem() -> ModeloNeutro {
    let mut base = peca("Base", caixa([100., 60., 10.]), [0.8, 0.2, 0.2, 1.]);
    base.camadas = vec!["Estrutura".into()];
    let pino = peca("Pino", caixa([10., 10., 40.]), [0.2, 0.2, 0.8, 1.]);
    let topo = ProdutoXde {
        nome: "Conjunto".into(),
        filhos: vec![
            InstanciaXde { produto: 0, nome: "Base 1".into(), local: IDENTIDADE, cor: None },
            InstanciaXde { produto: 1, nome: "Pino 1".into(), local: em(10., 10.), cor: None },
            InstanciaXde { produto: 1, nome: "Pino 2".into(), local: em(80., 40.), cor: None },
        ],
        ..Default::default()
    };
    ModeloNeutro { formato: Formato::Step, xde: ModeloXde { produtos: vec![base, pino, topo], raizes: vec![2], ..Default::default() } }
}

fn gravar(modelo: &ModeloNeutro, caminho: &Path, o: OpcoesExportar) {
    Registro::default().gravar(modelo, caminho, &o).unwrap();
}

/// Recalcula e devolve volume somado e a caixa do documento.
fn recalcular(doc: &Document) -> (f64, [f64; 3], [f64; 3]) {
    let r = fenkra_solidos::recompute(doc);
    assert!(!r.status.values().any(|s| matches!(s, fenkra_solidos::Status::Error(_))), "{:?}", r.status);
    let (mut lo, mut hi, mut volume) = ([f64::MAX; 3], [f64::MIN; 3], 0.);
    for b in r.bodies.iter() {
        volume += b.mass.volume;
        for q in b.mesh.positions.chunks_exact(3) {
            for i in 0..3 {
                lo[i] = lo[i].min(q[i] as f64);
                hi[i] = hi[i].max(q[i] as f64);
            }
        }
    }
    (volume, lo, hi)
}

fn perto(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-6 * a.abs().max(b.abs()).max(1.)
}

#[test]
fn montagem_vira_corpos_nomeados_e_coloridos_no_lugar_certo() {
    let dir = pasta("montagem");
    let arquivo = dir.join("conjunto de teste.step");
    gravar(&montagem(), &arquivo, OpcoesExportar::default());

    let i = Registro::default().importar(&arquivo, &OpcoesImportar::default()).unwrap();
    let r = &i.relatorio;
    assert_eq!((r.produtos, r.instancias, r.corpos, r.solidos, r.faces), (3, 3, 3, 3, 18), "{r:?}");
    assert!(perto(r.volume, 60_000. + 2. * 4_000.), "{}", r.volume);
    let (lo, hi) = r.caixa.unwrap();
    assert!(perto(hi[0], 100.) && perto(hi[1], 60.) && perto(hi[2], 40.) && perto(lo[0], 0.), "{lo:?} {hi:?}");
    assert!(r.nao_fecharam.is_empty());
    assert_eq!(r.faces_reparadas, 0, "forma sã não é tocada");

    let nomes: Vec<&str> = i.doc.collectors.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(nomes, ["Base 1", "Pino 1", "Pino 2"]);
    assert!(i.doc.collectors[1].color[2] > 150 && i.doc.collectors[1].color[0] < 100, "{:?}", i.doc.collectors[1].color);

    let texto = r.texto_pt();
    assert!(texto.contains("Camadas (Estrutura)") && texto.contains("Estrutura de montagem"), "{texto}");

    let (volume, lo, hi) = recalcular(&i.doc);
    assert!(perto(volume, 68_000.), "{volume}");
    assert!(perto(hi[0], 100.) && perto(hi[2], 40.) && perto(lo[1], 0.), "{lo:?} {hi:?}");
    let selo = i.doc.origem.as_ref().unwrap();
    assert_eq!(selo.nome, "conjunto de teste.step");
    assert_eq!(selo.hash, r.hash);
    assert!(selo.esquema.contains("AUTOMOTIVE_DESIGN"), "{}", selo.esquema);

    assert!(!i.previa.tris.is_empty() && !i.previa.incompleta);
    assert_eq!(i.previa.pos.len(), i.previa.cor.len());
    assert!(i.previa.cor.iter().all(|c| c.is_some()));
}

#[test]
fn documento_importado_sobrevive_ao_json_e_abre_sem_o_arquivo() {
    let dir = pasta("json");
    let arquivo = dir.join("peça.step");
    gravar(&montagem(), &arquivo, OpcoesExportar::default());
    let i = Registro::default().importar(&arquivo, &OpcoesImportar::default()).unwrap();
    let doc = Document::from_json(&i.doc.to_json()).unwrap();
    std::fs::remove_file(&arquivo).unwrap();
    let (volume, ..) = recalcular(&doc);
    assert!(perto(volume, 68_000.), "{volume}");
}

#[test]
fn brep_entra_com_o_que_falta_no_relatorio() {
    let dir = pasta("brep");
    let arquivo = dir.join("bloco com espaço.brep");
    std::fs::write(&arquivo, caixa([10., 20., 30.]).to_brep().unwrap()).unwrap();
    let i = Registro::default().importar(&arquivo, &OpcoesImportar::default()).unwrap();
    assert_eq!(i.relatorio.formato, "BREP");
    assert_eq!(i.relatorio.corpos, 1);
    let texto = i.relatorio.texto_pt();
    assert!(texto.contains("o BREP guarda só a forma"), "{texto}");
    let (volume, ..) = recalcular(&i.doc);
    assert!(perto(volume, 6_000.), "{volume}");
    assert_eq!(i.doc.origem.as_ref().unwrap().esquema, "BREP");
}

#[test]
fn step_renomeado_entra_e_avisa_e_lixo_e_recusado() {
    let dir = pasta("renomeado");
    let certo = dir.join("a.step");
    gravar(&montagem(), &certo, OpcoesExportar::default());
    let errado = dir.join("a.dat");
    std::fs::rename(&certo, &errado).unwrap();
    let i = Registro::default().importar(&errado, &OpcoesImportar::default()).unwrap();
    assert!(i.relatorio.texto_pt().contains("não combina com o conteúdo"));

    let lixo = dir.join("lixo.step");
    std::fs::write(&lixo, "nada a ver").unwrap();
    assert!(matches!(Registro::default().importar(&lixo, &OpcoesImportar::default()), Err(ErroConv::Formato(_))));
    assert!(matches!(Registro::default().importar(&dir.join("sumiu.step"), &OpcoesImportar::default()), Err(ErroConv::Leitura(_))));
}

#[test]
fn unidade_suspeita_avisa_e_trocar_a_unidade_corrige_a_escala() {
    let dir = pasta("unidade");
    let arquivo = dir.join("metros.step");
    // peça de 2 m modelada como 2 "mm": o caso clássico de arquivo com unidade trocada
    let m = ModeloNeutro::de_forma(Formato::Step, "Chapa", caixa([0.05, 0.02, 0.01]));
    gravar(&m, &arquivo, OpcoesExportar::default());
    let i = Registro::default().importar(&arquivo, &OpcoesImportar::default()).unwrap();
    assert!(i.relatorio.texto_pt().contains("a unidade do arquivo pode estar errada"));

    let o = OpcoesImportar { unidade: UnidadeArquivo::Metro, ..Default::default() };
    let i = Registro::default().importar(&arquivo, &o).unwrap();
    assert!(perto(i.relatorio.escala, 1000.));
    // a escala aumenta junto a tolerância das arestas, que entra na caixa do kernel
    assert!((i.relatorio.medidas().unwrap()[0] - 50.).abs() < 1e-3, "{:?}", i.relatorio.medidas());
    assert!(!i.relatorio.texto_pt().contains("pode estar errada"));
    let (volume, ..) = recalcular(&i.doc);
    assert!(perto(volume, 50. * 20. * 10.), "{volume}");
}

#[test]
fn step_em_polegadas_mantem_a_medida_e_declara_a_unidade() {
    let dir = pasta("pol");
    let arquivo = dir.join("pol.step");
    let m = ModeloNeutro::de_forma(Formato::Step, "Bloco", caixa([25.4, 25.4, 25.4]));
    gravar(&m, &arquivo, OpcoesExportar { unidade: UnidadeSaida::Polegada, ..Default::default() });
    let i = Registro::default().importar(&arquivo, &OpcoesImportar::default()).unwrap();
    assert!(i.relatorio.unidades.to_uppercase().contains("INCH"), "{}", i.relatorio.unidades);
    assert!(perto(i.relatorio.medidas().unwrap()[0], 25.4));
}

/// Faixa cilíndrica sem tampas: casca que não fecha em nenhum nível de reparo.
fn superficie_aberta() -> Shape {
    Shape::cylinder_surface([0., 0., 0.], [0., 0., 1.], 10., 0., 20.).unwrap()
}

#[test]
fn casca_aberta_aparece_no_relatorio_e_o_documento_recalcula() {
    let dir = pasta("aberta");
    let arquivo = dir.join("casca.brep");
    std::fs::write(&arquivo, superficie_aberta().to_brep().unwrap()).unwrap();
    for reparo in Reparo::TODOS {
        let i = Registro::default().importar(&arquivo, &OpcoesImportar { reparo, ..Default::default() }).unwrap();
        assert_eq!(i.relatorio.solidos, 0);
        assert_eq!(i.relatorio.nao_fecharam, vec!["casca".to_string()]);
        assert!(i.relatorio.texto_pt().contains("Sólidos que não fecharam:\n- casca"));
        // o documento mostra a superfície como veio, sem tampar as bordas
        let (volume, ..) = recalcular(&i.doc);
        assert_eq!(volume, 0.);
        let r = fenkra_solidos::recompute(&i.doc);
        assert!(r.bodies.is_empty() && r.surfaces.len() == 1, "{} corpos, {} superfícies", r.bodies.len(), r.surfaces.len());
    }
}

#[test]
fn peca_multicorpo_vira_um_corpo_por_solido() {
    let dir = pasta("multi");
    let arquivo = dir.join("multi.step");
    let duas = caixa([1., 1., 1.]).compound(&caixa([2., 2., 2.]).com_matriz(&em(10., 0.)).unwrap());
    gravar(&ModeloNeutro::de_forma(Formato::Step, "Multi", duas), &arquivo, OpcoesExportar::default());
    let i = Registro::default().importar(&arquivo, &OpcoesImportar::default()).unwrap();
    assert_eq!(i.relatorio.corpos, 2, "{:?}", i.relatorio);
    let (volume, ..) = recalcular(&i.doc);
    assert!(perto(volume, 9.), "{volume}");
}

#[test]
fn gravar_recusa_extensao_desconhecida() {
    let dir = pasta("ext");
    let e = Registro::default().gravar(&montagem(), &dir.join("a.igs"), &OpcoesExportar::default()).unwrap_err();
    assert!(matches!(e, ErroConv::Formato(_)));
    assert!(!dir.join("a.igs").exists());
}
