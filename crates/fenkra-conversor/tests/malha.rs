// SPDX-License-Identifier: LGPL-2.1-or-later
//! B210: ida e volta de cada formato de malha, do B-rep ao arquivo e de volta ao documento recalculado.

use fenkra_conversor::malha::{Cena, Malha, No};
use fenkra_conversor::{Destino, Formato, Importado, ModeloNeutro, OpcoesExportar, OpcoesImportar, OpcoesMalha, Registro, UnidadeArquivo, UnidadeSaida};
use fenkra_core::Document;
use fenkra_kernel::{InstanciaXde, ModeloXde, ProdutoXde, Shape, IDENTIDADE};
use fenkra_solidos::eixos::{chaveta_documento, eixo_documento, ParamEixo, TrechoEixo};
use fenkra_solidos::fixadores::fixador_documento;
use fenkra_solidos::{recompute, Status};
use std::path::{Path, PathBuf};

const VERMELHO: [f32; 4] = [0.8, 0.2, 0.2, 1.];
const AZUL: [f32; 4] = [0.2, 0.2, 0.8, 1.];
/// Formatos que leem e gravam.
const IDA_E_VOLTA: [Formato; 10] = [Formato::Stl, Formato::Obj, Formato::Ply, Formato::Tmf, Formato::Amf, Formato::Gltf, Formato::Glb, Formato::Vrml, Formato::X3d, Formato::Collada];
/// Formatos com árvore e instâncias.
const COM_ARVORE: [Formato; 7] = [Formato::Tmf, Formato::Amf, Formato::Gltf, Formato::Glb, Formato::Vrml, Formato::X3d, Formato::Collada];

fn pasta(teste: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("Fenkra malha açaí {} {teste}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

fn arquivo(dir: &Path, f: Formato) -> PathBuf {
    dir.join(format!("peça {}.{}", f.nome(), f.extensoes()[0]))
}

fn caixa(l: [f64; 3]) -> Shape {
    Shape::box_tool([0., 0., 0.], [1., 0., 0.], [0., 0., 1.], l).unwrap()
}

/// Base e dois pinos da mesma definição; o segundo deitado, girado 90 graus em X.
fn montagem() -> ModeloNeutro {
    let base = ProdutoXde { nome: "Base".into(), forma: Some(caixa([100., 60., 10.])), cor: Some(VERMELHO), ..Default::default() };
    let pino = ProdutoXde { nome: "Pino".into(), forma: Some(caixa([10., 10., 40.])), cor: Some(AZUL), ..Default::default() };
    let em_pe = [1., 0., 0., 10., 0., 1., 0., 10., 0., 0., 1., 10.];
    let deitado = [1., 0., 0., 70., 0., 0., -1., 50., 0., 1., 0., 10.];
    let topo = ProdutoXde {
        nome: "Conjunto".into(),
        filhos: vec![
            InstanciaXde { produto: 0, nome: "Base 1".into(), local: IDENTIDADE, cor: None },
            InstanciaXde { produto: 1, nome: "Pino 1".into(), local: em_pe, cor: None },
            InstanciaXde { produto: 1, nome: "Pino 2".into(), local: deitado, cor: None },
        ],
        ..Default::default()
    };
    ModeloNeutro { formato: Formato::Step, xde: ModeloXde { produtos: vec![base, pino, topo], raizes: vec![2], ..Default::default() } }
}

const VOLUME: f64 = 100. * 60. * 10. + 2. * 10. * 10. * 40.;

fn uma_peca() -> OpcoesImportar {
    OpcoesImportar { destino: Destino::UmaPeca, ..Default::default() }
}

fn gravar_e_ler(modelo: &ModeloNeutro, f: Formato, dir: &Path, saida: OpcoesExportar, entrada: OpcoesImportar) -> Importado {
    let a = arquivo(dir, f);
    Registro::default().gravar(modelo, &a, &saida).unwrap_or_else(|e| panic!("gravar {}: {e}", f.nome()));
    Registro::default().importar(&a, &entrada).unwrap_or_else(|e| panic!("ler {}: {e}", f.nome()))
}

fn perto(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

fn medida_igual(i: &Importado, m: [f64; 3], tol: f64, oque: &str) {
    let q = i.relatorio.medidas().unwrap();
    assert!((0..3).all(|k| perto(q[k], m[k], tol)), "{oque}: {q:?}");
}

fn recalcular(doc: &Document) -> f64 {
    let r = recompute(doc);
    assert!(!r.status.values().any(|s| matches!(s, Status::Error(_))), "{:?}", r.status);
    r.bodies.iter().map(|b| b.mass.volume).sum()
}

fn tem_cor(doc: &Document, c: [f32; 4]) -> bool {
    let alvo = [c[0], c[1], c[2]].map(|v| (v * 255.).round() as i32);
    doc.collectors.iter().any(|k| (0..3).all(|i| (i32::from(k.color[i]) - alvo[i]).abs() <= 1))
}

#[test]
fn cada_formato_volta_com_volume_caixa_e_solidos_fechados() {
    let dir = pasta("volume");
    let m = montagem();
    for f in IDA_E_VOLTA {
        let i = gravar_e_ler(&m, f, &dir, OpcoesExportar::default(), uma_peca());
        let r = &i.relatorio;
        assert_eq!(r.formato, f.nome());
        assert_eq!(r.solidos, 3, "{}", f.nome());
        assert!(r.nao_fecharam.is_empty(), "{}: {:?}", f.nome(), r.nao_fecharam);
        assert!(perto(r.volume, VOLUME, 1e-2), "{}: {}", f.nome(), r.volume);
        // o pino de pé sobe a 50 e o deitado encosta em y = 10
        medida_igual(&i, [100., 60., 50.], 1e-3, f.nome());
        assert_eq!(i.doc.features.len() > 0, true);
        assert!(perto(recalcular(&i.doc), VOLUME, 1e-2), "{} recalculado", f.nome());
    }
}

#[test]
fn cor_chega_ao_corpo_em_quem_tem_cor() {
    let dir = pasta("cor");
    let m = montagem();
    for f in IDA_E_VOLTA.into_iter().filter(|f| *f != Formato::Stl) {
        let i = gravar_e_ler(&m, f, &dir, OpcoesExportar::default(), uma_peca());
        assert!(tem_cor(&i.doc, VERMELHO) && tem_cor(&i.doc, AZUL), "{}: {:?}", f.nome(), i.doc.collectors.iter().map(|c| c.color).collect::<Vec<_>>());
    }
}

#[test]
fn opcao_sem_cores_tira_a_cor_da_gravacao() {
    let dir = pasta("sem cor");
    let saida = OpcoesExportar { cores: false, ..Default::default() };
    for f in [Formato::Tmf, Formato::Gltf, Formato::Collada] {
        let i = gravar_e_ler(&montagem(), f, &dir, saida, uma_peca());
        assert!(!tem_cor(&i.doc, VERMELHO), "{}", f.nome());
    }
}

#[test]
fn estrutura_de_montagem_atravessa_quem_tem_arvore_e_a_peca_repetida_fica_uma_so() {
    let dir = pasta("arvore");
    for f in COM_ARVORE {
        let i = gravar_e_ler(&montagem(), f, &dir, OpcoesExportar::default(), OpcoesImportar::default());
        let r = &i.relatorio;
        assert_eq!((r.produtos, r.instancias), (3, 3), "{}", f.nome());
        let c = i.conjunto.as_ref().unwrap_or_else(|| panic!("{}: sem conjunto", f.nome()));
        assert_eq!(c.pecas.len(), 2, "{}: o pino repetido é uma peça", f.nome());
    }
}

#[test]
fn formato_sem_arvore_chega_com_as_pecas_no_lugar() {
    let dir = pasta("achatado");
    let i = gravar_e_ler(&montagem(), Formato::Obj, &dir, OpcoesExportar::default(), uma_peca());
    let nomes: Vec<&str> = i.doc.collectors.iter().map(|c| c.name.as_str()).collect();
    assert!(nomes.iter().any(|n| n.contains("Pino 2")), "{nomes:?}");
    let i = gravar_e_ler(&montagem(), Formato::Stl, &dir, OpcoesExportar::default(), uma_peca());
    assert_eq!(i.relatorio.corpos, 3, "partes soltas viram corpos");
}

#[test]
fn unidade_declarada_mantem_a_medida_e_a_nao_declarada_avisa() {
    let dir = pasta("unidade");
    let polegada = OpcoesExportar { unidade: UnidadeSaida::Polegada, ..Default::default() };
    for f in [Formato::Tmf, Formato::Amf, Formato::Collada] {
        let i = gravar_e_ler(&montagem(), f, &dir, polegada, uma_peca());
        medida_igual(&i, [100., 60., 50.], 1e-3, f.nome());
    }
    let metro = OpcoesExportar { unidade: UnidadeSaida::Metro, ..Default::default() };
    let i = gravar_e_ler(&montagem(), Formato::Tmf, &dir, metro, uma_peca());
    medida_igual(&i, [100., 60., 50.], 1e-3, "3MF em metros");

    // STL em polegadas não diz que é polegada: sai 25,4 vezes menor e avisa
    let i = gravar_e_ler(&montagem(), Formato::Stl, &dir, polegada, uma_peca());
    medida_igual(&i, [100. / 25.4, 60. / 25.4, 50. / 25.4], 1e-3, "STL sem unidade");
    assert!(i.relatorio.avisos.iter().any(|a| a.modelo.contains("não declara a unidade")), "{:?}", i.relatorio.avisos);
    // e quem sabe a unidade corrige na leitura
    let certo = OpcoesImportar { unidade: UnidadeArquivo::Polegada, ..uma_peca() };
    let i = gravar_e_ler(&montagem(), Formato::Stl, &dir, polegada, certo);
    medida_igual(&i, [100., 60., 50.], 1e-3, "STL corrigido");
}

#[test]
fn relatorio_diz_o_que_a_malha_nao_traz() {
    let dir = pasta("relatorio");
    let i = gravar_e_ler(&montagem(), Formato::Obj, &dir, OpcoesExportar::default(), uma_peca());
    assert!(i.relatorio.nao_carrega.iter().any(|n| n.modelo.contains("Geometria exata")), "{:?}", i.relatorio.nao_carrega);
    for n in i.relatorio.nao_carrega.iter().chain(&i.relatorio.avisos) {
        assert!(fenkra_conversor::NOTAS.contains(&n.modelo.as_str()), "{}", n.modelo);
    }
}

#[test]
fn texto_e_binario_dao_o_mesmo_volume() {
    let dir = pasta("texto");
    for binario in [false, true] {
        let saida = OpcoesExportar { malha: OpcoesMalha { binario, ..Default::default() }, ..Default::default() };
        for f in [Formato::Stl, Formato::Ply] {
            let i = gravar_e_ler(&montagem(), f, &dir, saida, uma_peca());
            assert!(perto(i.relatorio.volume, VOLUME, 1e-2), "{} {binario}", f.nome());
        }
    }
}

fn eixo() -> Document {
    let trechos = vec![TrechoEixo { diametro: 20., comprimento: 50., chanfro_fim: 0., rasgo: None }, TrechoEixo { diametro: 25., comprimento: 70., chanfro_fim: 0., rasgo: None }];
    eixo_documento(&ParamEixo { trechos, furo_centro: false }).unwrap()
}

/// O aceite do corpus: a peça curva vira malha pelo desvio pedido e volta sólida, com o volume dentro
/// do que o desvio permite.
#[test]
fn corpus_atravessa_cada_formato_dentro_do_desvio() {
    let dir = pasta("corpus");
    let saida = OpcoesExportar { malha: OpcoesMalha { desvio: 0.02, angulo: 10., ..Default::default() }, ..Default::default() };
    for doc in [eixo(), chaveta_documento(25., 40.).unwrap(), fixador_documento("iso4032", "M8", None).unwrap()] {
        let forma = fenkra_solidos::final_shape(&doc).unwrap();
        let exato = forma.mass().volume;
        let (a, b) = forma.bbox().unwrap();
        let modelo = ModeloNeutro::de_documento(&doc).unwrap();
        for f in IDA_E_VOLTA {
            let i = gravar_e_ler(&modelo, f, &dir, saida, uma_peca());
            let r = &i.relatorio;
            assert!(r.solidos >= 1 && r.nao_fecharam.is_empty(), "{} {}: {:?}", doc.name, f.nome(), r.nao_fecharam);
            assert!((r.volume - exato).abs() / exato < 0.01, "{} {}: {} x {exato}", doc.name, f.nome(), r.volume);
            medida_igual(&i, [b[0] - a[0], b[1] - a[1], b[2] - a[2]], 0.1, &format!("{} {}", doc.name, f.nome()));
        }
    }
}

/// Cena sem kernel: duas malhas, uma delas repetida, com cor por vértice na outra.
fn cena_colorida() -> Cena {
    let mut c = Cena { nome: "Cena".into(), ..Default::default() };
    let pos: Vec<[f64; 3]> = vec![[0., 0., 0.], [10., 0., 0.], [0., 10., 0.], [0., 0., 10.]];
    let tris = vec![[0, 2, 1], [0, 1, 3], [1, 2, 3], [0, 3, 2]];
    let cor_v = vec![[255, 0, 0, 255], [0, 255, 0, 255], [0, 0, 255, 255], [255, 255, 0, 255]];
    c.malhas.push(Malha { nome: "Tetra".into(), pos, tris, cor_v, ..Default::default() });
    let mut n = No::novo("Tetra");
    n.malhas = vec![0];
    c.nos.push(n);
    c.raizes.push(0);
    c
}

#[test]
fn malha_para_malha_nao_passa_pelo_solido_e_mantem_a_cor_por_vertice() {
    let dir = pasta("direto");
    let r = Registro::default();
    let (a, b, d) = (dir.join("a.ply"), dir.join("b.glb"), dir.join("c.ply"));
    let cena = cena_colorida();
    fenkra_conversor::malha::gravar_para_teste(Formato::Ply, &cena, &a).unwrap();
    r.converter(&a, &b, &OpcoesImportar::default(), &OpcoesExportar::default()).unwrap();
    r.converter(&b, &d, &OpcoesImportar::default(), &OpcoesExportar::default()).unwrap();
    let volta = fenkra_conversor::malha::ler_para_teste(Formato::Ply, &d).unwrap();
    let m = &volta.no_lugar()[0];
    assert_eq!(m.tris.len(), 4);
    assert!((m.volume() - 1000. / 6.).abs() < 1e-3, "{}", m.volume());
    let vermelho = m.pos.iter().position(|p| p.iter().all(|v| v.abs() < 1e-6)).unwrap();
    assert_eq!(m.cor_v[vermelho], [255, 0, 0, 255]);
}

#[test]
fn step_vira_malha_pela_conversao_e_malha_vira_step() {
    let dir = pasta("cruzado");
    let r = Registro::default();
    let (step, glb, volta) = (dir.join("a.step"), dir.join("a.glb"), dir.join("b.step"));
    r.gravar(&montagem(), &step, &OpcoesExportar::default()).unwrap();
    r.converter(&step, &glb, &OpcoesImportar::default(), &OpcoesExportar::default()).unwrap();
    r.converter(&glb, &volta, &OpcoesImportar::default(), &OpcoesExportar::default()).unwrap();
    let i = r.importar(&volta, &uma_peca()).unwrap();
    assert!(perto(i.relatorio.volume, VOLUME, 1e-2), "{}", i.relatorio.volume);
    assert_eq!(i.relatorio.solidos, 3);
}

#[test]
fn usd_e_usdz_gravam_e_o_pacote_tem_a_cena_alinhada() {
    let dir = pasta("usd");
    let r = Registro::default();
    let (usda, usdz) = (arquivo(&dir, Formato::Usda), arquivo(&dir, Formato::Usdz));
    r.gravar(&montagem(), &usda, &OpcoesExportar::default()).unwrap();
    r.gravar(&montagem(), &usdz, &OpcoesExportar::default()).unwrap();
    let t = std::fs::read_to_string(&usda).unwrap();
    assert!(t.starts_with("#usda 1.0"));
    assert!(t.contains("metersPerUnit = 0.001"));
    assert_eq!(t.matches("def Mesh").count(), 2, "o pino repetido é uma malha só");
    assert_eq!(t.matches("references = ").count(), 3);
    assert_eq!(t.matches("def Material").count(), 2);
    let z = std::fs::read(&usdz).unwrap();
    let p = z.windows(5).position(|w| w == b"#usda").unwrap();
    assert_eq!(p % 64, 0);
    assert_eq!(fenkra_conversor::detectar(&usdz, &z).unwrap().0, Formato::Usdz);
    let e = r.importar(&usda, &OpcoesImportar::default()).err().unwrap();
    assert!(e.to_string().contains("não lê"), "{e}");
}

#[test]
fn obj_com_mtl_ao_lado_e_material_que_falta_vai_para_o_relatorio() {
    let dir = pasta("mtl");
    let a = arquivo(&dir, Formato::Obj);
    Registro::default().gravar(&montagem(), &a, &OpcoesExportar::default()).unwrap();
    assert!(a.with_extension("mtl").is_file());
    std::fs::remove_file(a.with_extension("mtl")).unwrap();
    let i = Registro::default().importar(&a, &uma_peca()).unwrap();
    let texto: Vec<String> = i.relatorio.nao_carrega.iter().flat_map(|n| n.args.clone()).collect();
    assert!(texto.iter().any(|t| t.contains("biblioteca de materiais")), "{texto:?}");
}

#[test]
fn caminho_para_fora_da_pasta_nao_e_lido() {
    let dir = pasta("fora");
    let segredo = dir.join("segredo.bin");
    std::fs::write(&segredo, [0u8; 36]).unwrap();
    let sub = dir.join("sub");
    std::fs::create_dir_all(&sub).unwrap();
    let a = sub.join("a.gltf");
    let json = r#"{"asset":{"version":"2.0"},"buffers":[{"uri":"../segredo.bin","byteLength":36}]}"#;
    std::fs::write(&a, json).unwrap();
    let e = Registro::default().importar(&a, &OpcoesImportar::default()).err().unwrap();
    assert!(e.to_string().contains("fora da pasta"), "{e}");
}

#[test]
fn arquivo_cortado_ou_trocado_devolve_erro_sem_derrubar() {
    let dir = pasta("cortado");
    let saida = OpcoesExportar::default();
    for f in IDA_E_VOLTA {
        let a = arquivo(&dir, f);
        Registro::default().gravar(&montagem(), &a, &saida).unwrap();
        let bytes = std::fs::read(&a).unwrap();
        for corte in [bytes.len() / 2, bytes.len() * 9 / 10, 90] {
            std::fs::write(&a, &bytes[..corte.min(bytes.len())]).unwrap();
            // o que importa é não cair: pode dar erro, ou pode ler o que sobrou
            let _ = Registro::default().importar(&a, &uma_peca());
        }
        let mut trocado = bytes.clone();
        for i in (0..trocado.len()).step_by(97) {
            trocado[i] ^= 0x55;
        }
        std::fs::write(&a, &trocado).unwrap();
        let _ = Registro::default().importar(&a, &uma_peca());
    }
}

#[test]
fn malha_acima_do_teto_recusa_o_solido_mas_converte_para_malha() {
    let dir = pasta("teto");
    let mut c = Cena::default();
    let n = 460usize;
    let mut m = Malha { nome: "Grade".into(), ..Default::default() };
    for j in 0..=n {
        for i in 0..=n {
            m.pos.push([i as f64, j as f64, 0.]);
        }
    }
    let k = |i: usize, j: usize| (j * (n + 1) + i) as u32;
    for j in 0..n {
        for i in 0..n {
            m.tris.push([k(i, j), k(i + 1, j), k(i + 1, j + 1)]);
            m.tris.push([k(i, j), k(i + 1, j + 1), k(i, j + 1)]);
        }
    }
    let mut no = No::novo("Grade");
    no.malhas = vec![0];
    c.malhas.push(m);
    c.nos.push(no);
    c.raizes.push(0);
    let (a, b) = (dir.join("g.stl"), dir.join("g.3mf"));
    fenkra_conversor::malha::gravar_para_teste(Formato::Stl, &c, &a).unwrap();
    let e = Registro::default().importar(&a, &OpcoesImportar::default()).err().unwrap();
    assert!(e.to_string().contains("grande demais"), "{e}");
    Registro::default().converter(&a, &b, &OpcoesImportar::default(), &OpcoesExportar::default()).unwrap();
    let volta = fenkra_conversor::malha::ler_para_teste(Formato::Tmf, &b).unwrap();
    assert_eq!(volta.no_lugar()[0].tris.len(), 2 * n * n);
}
