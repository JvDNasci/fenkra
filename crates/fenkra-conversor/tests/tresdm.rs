// SPDX-License-Identifier: LGPL-2.1-or-later
//! B211: 3DM pelo openNURBS. Grava um modelo montado em código (placa com furo e uma superfície livre, em
//! camadas e cores diferentes), lê de volta e confere estado, volume, caixa, camadas e cores. Sem o
//! openNURBS na compilação, os testes dizem que ficaram de fora e conferem só a resposta de "como obter".

use fenkra_conversor::tresdm::{arquivo_do_modelo, gravar_com_avisos, modelo_do_arquivo};
use fenkra_conversor::{detectar, Destino, ErroConv, Formato, InstanciaNeutra, ModeloNeutro, OpcoesImportar, ProdutoNeutro, Provedor, Provedor3dm, Registro};
use fenkra_kernel::{Bool, HoleShape, ModeloXde, Shape, IDENTIDADE};
use std::path::{Path, PathBuf};

fn pasta(teste: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("Fenkra 3dm açaí {} {teste}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

const VERMELHO: [f32; 4] = [0.8, 0.2, 0.2, 1.];
const AZUL: [f32; 4] = [0.2, 0.4, 1.0, 1.];

/// Placa 80 × 40 × 10 com furo passante de 12 mm na camada "Chapas > Corte" e uma superfície ondulada
/// (casca aberta) na camada "Superfícies", a 100 mm da placa.
pub fn modelo() -> ModeloNeutro {
    let placa = Shape::box_tool([0., 0., 0.], [1., 0., 0.], [0., 0., 1.], [80., 40., 10.]).unwrap();
    let furo = Shape::hole_tool([20., 20., 10.], [0., 0., -1.], 12., 10., HoleShape::Simple).unwrap();
    let placa = placa.boolean(&furo, Bool::Cut).unwrap();
    let linhas: Vec<Vec<[f64; 3]>> = (0..5).map(|i| (0..4).map(|j| [100. + 10. * i as f64, 10. * j as f64, if (i + j) % 2 == 0 { 0. } else { 4. }]).collect()).collect();
    let casca = Shape::grid_surface(&linhas).unwrap();
    let chapa = ProdutoNeutro { nome: "Chapa".into(), forma: Some(placa), cor: Some(VERMELHO), camadas: vec!["Chapas > Corte".into()], ..Default::default() };
    let onda = ProdutoNeutro { nome: "Onda".into(), forma: Some(casca), cor: Some(AZUL), camadas: vec!["Superfícies".into()], ..Default::default() };
    let filhos = vec![InstanciaNeutra { produto: 0, nome: "Chapa".into(), local: IDENTIDADE, cor: None }, InstanciaNeutra { produto: 1, nome: "Onda".into(), local: IDENTIDADE, cor: None }];
    let topo = ProdutoNeutro { nome: "Prova".into(), filhos, ..Default::default() };
    ModeloNeutro { formato: Formato::Step, xde: ModeloXde { produtos: vec![chapa, onda, topo], raizes: vec![2], ..Default::default() } }
}

fn perto(a: f32, b: f32) -> bool {
    (a - b).abs() < 1. / 255.
}

#[test]
fn placa_com_furo_e_superficie_livre_vao_e_voltam_com_camadas_e_cores() {
    if !fenkra_3dm::disponivel() {
        eprintln!("3DM IGNORADO: compilado sem o openNURBS (scripts/baixar_opennurbs.py)");
        return;
    }
    let dir = pasta("ida e volta");
    let a = dir.join("prova ç.3dm");
    let avisos = gravar_com_avisos(&modelo(), &a).unwrap();
    assert!(avisos.is_empty(), "o openNURBS recusou: {avisos:?}");
    let bytes = std::fs::read(&a).unwrap();
    assert_eq!(detectar(&a, &bytes).unwrap(), (Formato::Tresdm, false));

    let lido = Provedor3dm.ler(Formato::Tresdm, &a, &OpcoesImportar::default()).unwrap();
    assert!(lido.xde.ignoradas.is_empty(), "{:?}", lido.xde.ignoradas);
    let pecas: Vec<&ProdutoNeutro> = lido.xde.produtos.iter().filter(|p| p.forma.is_some()).collect();
    assert_eq!(pecas.len(), 2);
    let chapa = pecas.iter().find(|p| p.nome == "Chapa").expect("Chapa");
    let onda = pecas.iter().find(|p| p.nome == "Onda").expect("Onda");
    // camadas com a hierarquia e cores de volta
    assert_eq!(chapa.camadas, vec!["Chapas > Corte".to_string()]);
    assert_eq!(onda.camadas, vec!["Superfícies".to_string()]);
    let (c, o) = (chapa.cor.unwrap(), onda.cor.unwrap());
    assert!((0..3).all(|k| perto(c[k], VERMELHO[k]) && perto(o[k], AZUL[k])), "{c:?} {o:?}");
    // a camada aparece na árvore: Prova > Chapas > Corte > Chapa
    let caminhos: Vec<Vec<String>> = lido.ocorrencias().into_iter().map(|c| c.caminho).collect();
    assert!(caminhos.contains(&vec!["prova ç".to_string(), "Chapas".into(), "Corte".into(), "Chapa".into()]), "{caminhos:?}");

    // geometria: a placa volta sólida e válida, com o volume pela fórmula e a mesma caixa
    let f = chapa.forma.as_ref().unwrap();
    assert!(f.is_solid() && f.is_valid());
    let exato = 80. * 40. * 10. - std::f64::consts::PI * 36. * 10.;
    assert!((f.volume_preciso() - exato).abs() < 1e-6 * exato, "{} contra {exato}", f.volume_preciso());
    let (lo, hi) = f.bbox().unwrap();
    assert!((0..3).all(|k| lo[k].abs() < 1e-4) && (hi[0] - 80.).abs() < 1e-4 && (hi[1] - 40.).abs() < 1e-4 && (hi[2] - 10.).abs() < 1e-4, "{lo:?} {hi:?}");
    // a superfície livre volta aberta, com a mesma área e passando pelos mesmos pontos
    let g = onda.forma.as_ref().unwrap();
    assert!(!g.is_solid() && g.is_valid());
    let original = modelo().xde.produtos[1].forma.clone().unwrap();
    assert!((g.mass().area - original.mass().area).abs() < 1e-6 * original.mass().area, "{} contra {}", g.mass().area, original.mass().area);
    let pts: Vec<[f64; 3]> = [[105., 5.], [115., 12.], [127., 23.]].iter().map(|&[x, y]| [x, y, 0.]).collect();
    let (d0, d1) = (original.signed_distances(&pts).unwrap(), g.signed_distances(&pts).unwrap());
    assert!(d0.iter().zip(&d1).all(|(a, b)| (a.abs() - b.abs()).abs() < 1e-7), "{d0:?} {d1:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn importar_3dm_da_um_corpo_por_objeto_com_a_camada_no_nome_e_a_cor() {
    if !fenkra_3dm::disponivel() {
        eprintln!("3DM IGNORADO: compilado sem o openNURBS");
        return;
    }
    let dir = pasta("importar");
    let a = dir.join("prova.3dm");
    Registro::default().gravar(&modelo(), &a, &Default::default()).unwrap();
    let i = Registro::default().importar(&a, &OpcoesImportar { destino: Destino::UmaPeca, ..Default::default() }).unwrap();
    let r = &i.relatorio;
    assert_eq!((r.formato.as_str(), r.provedor.as_str(), r.unidades.as_str()), ("3DM", "openNURBS", "mm"));
    assert_eq!(r.solidos, 1, "{:?}", r.nao_fecharam);
    assert_eq!(r.nao_fecharam.len(), 1, "a superfície livre fica aberta");
    let nomes: Vec<&str> = i.doc.collectors.iter().map(|k| k.name.as_str()).collect();
    assert!(nomes.contains(&"Chapas / Corte / Chapa"), "{nomes:?}");
    let k = i.doc.collectors.iter().find(|k| k.name == "Chapas / Corte / Chapa").unwrap();
    assert_eq!(k.color, [204, 51, 51]);
    // como montagem, cada camada é um grupo
    let m = Registro::default().importar(&a, &OpcoesImportar { destino: Destino::Montagem, ..Default::default() }).unwrap();
    assert!(m.conjunto.is_some(), "o 3DM com dois objetos em camadas pede conjunto");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn sem_o_opennurbs_a_resposta_diz_como_obter() {
    if fenkra_3dm::disponivel() {
        // com o openNURBS, o arquivo que não é 3DM de verdade é recusado pelo próprio openNURBS, com erro
        let dir = pasta("falso");
        let a = dir.join("falso.3dm");
        std::fs::write(&a, b"3D Geometry File Format        8\0\0lixo").unwrap();
        let e = Provedor3dm.ler(Formato::Tresdm, &a, &OpcoesImportar::default()).err().unwrap();
        assert!(matches!(e, ErroConv::Leitura(_)), "{e:?}");
        let _ = std::fs::remove_dir_all(&dir);
        return;
    }
    let e = Provedor3dm.ler(Formato::Tresdm, Path::new("x.3dm"), &OpcoesImportar::default()).err().unwrap();
    assert!(matches!(&e, ErroConv::Ausente(t) if t.contains("baixar_opennurbs") && t.contains("STEP")), "{e:?}");
}

#[test]
fn modelo_sem_forma_nao_vira_3dm() {
    let vazio = ModeloNeutro { formato: Formato::Step, xde: ModeloXde::default() };
    assert!(arquivo_do_modelo(&vazio).is_err());
}

/// Lê os 3DM de uma pasta (por exemplo os arquivos de exemplo do repositório do openNURBS) e confere que
/// cada objeto vira forma válida. Só roda com `FENKRA_3DM_AMOSTRAS=<pasta>`.
#[test]
fn amostras_de_fora_viram_formas_validas() {
    let Ok(raiz) = std::env::var("FENKRA_3DM_AMOSTRAS") else { return };
    if !fenkra_3dm::disponivel() {
        return;
    }
    let mut arquivos = vec![];
    let mut pilha = vec![PathBuf::from(raiz)];
    while let Some(p) = pilha.pop() {
        for e in std::fs::read_dir(&p).into_iter().flatten().flatten() {
            let q = e.path();
            if q.is_dir() {
                pilha.push(q);
            } else if q.extension().is_some_and(|x| x.eq_ignore_ascii_case("3dm")) {
                arquivos.push(q);
            }
        }
    }
    arquivos.sort();
    let mut ruins = vec![];
    for a in &arquivos {
        let Ok(arq) = fenkra_3dm::ler(a) else {
            eprintln!("{}: o openNURBS não leu", a.display());
            continue;
        };
        let m = modelo_do_arquivo(&arq, "amostra");
        let formas: Vec<&Shape> = m.xde.produtos.iter().filter_map(|p| p.forma.as_ref()).collect();
        let invalidas = formas.iter().filter(|f| !f.is_valid()).count();
        let solidos = formas.iter().filter(|f| f.is_solid()).count();
        eprintln!(
            "{}: {} objetos, {} formas ({solidos} sólidos, {invalidas} inválidas), {} camadas, aproximados {}, fora: {:?}",
            Path::new(a).file_name().unwrap().to_string_lossy(),
            arq.objetos.len(),
            formas.len(),
            arq.camadas.len(),
            arq.objetos.iter().filter(|o| o.aproximado).count(),
            m.xde.ignoradas
        );
        if invalidas > 0 {
            ruins.push(a.display().to_string());
        }
    }
    assert!(ruins.is_empty(), "formas inválidas em {ruins:?}");
}
