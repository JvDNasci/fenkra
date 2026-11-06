// SPDX-License-Identifier: LGPL-2.1-or-later
//! B87: importação pela ponte XDE até o documento recalculado.

use fenkra_conversor::{Destino, ErroConv, Formato, ModeloNeutro, ModoIges, OpcoesExportar, OpcoesImportar, Registro, Relatorio, Reparo, TipoDefeito, UnidadeArquivo, UnidadeSaida};
use fenkra_core::Document;
use fenkra_kernel::{Frame, InstanciaXde, ModeloXde, ProdutoXde, Seg2, Shape, IDENTIDADE};
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

fn uma_peca() -> OpcoesImportar {
    OpcoesImportar { destino: Destino::UmaPeca, ..Default::default() }
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

    let i = Registro::default().importar(&arquivo, &uma_peca()).unwrap();
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
    let i = Registro::default().importar(&arquivo, &uma_peca()).unwrap();
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
    let e = Registro::default().gravar(&montagem(), &dir.join("a.xyz"), &OpcoesExportar::default()).unwrap_err();
    assert!(matches!(e, ErroConv::Formato(_)));
    assert!(!dir.join("a.xyz").exists());
}

/// As seis faces de um cubo de 10 mm, sem casca; a primeira é a tampa.
fn faces_de_cubo() -> Vec<Shape> {
    let quad = |o: [f64; 3], x: [f64; 3], n: [f64; 3]| {
        let f = Frame { origin: o, x, normal: n };
        let cantos = [[0., 0.], [10., 0.], [10., 10.], [0., 10.]];
        let segs: Vec<_> = (0..4).map(|i| Seg2::Line(cantos[i], cantos[(i + 1) % 4])).collect();
        Shape::face(&f, &[segs]).unwrap()
    };
    vec![
        quad([0., 0., 10.], [1., 0., 0.], [0., 0., 1.]),
        quad([0., 0., 0.], [1., 0., 0.], [0., -1., 0.]),
        quad([0., 10., 0.], [0., 0., 1.], [0., 1., 0.]),
        quad([0., 0., 0.], [0., 0., 1.], [-1., 0., 0.]),
        quad([10., 0., 0.], [0., 1., 0.], [1., 0., 0.]),
        quad([0., 0., 0.], [0., 1., 0.], [0., 0., -1.]),
    ]
}

fn composto(formas: Vec<Shape>) -> Shape {
    let mut formas = formas.into_iter();
    let primeira = formas.next().unwrap();
    formas.fold(primeira, |a, b| a.compound(&b))
}

#[test]
fn reparo_em_niveis_mostra_antes_e_depois_e_lista_o_que_sobrou() {
    let dir = pasta("niveis");
    let arquivo = dir.join("faces.brep");
    std::fs::write(&arquivo, composto(faces_de_cubo()).to_brep().unwrap()).unwrap();

    let nada = Registro::default().importar(&arquivo, &OpcoesImportar { reparo: Reparo::Nenhum, ..Default::default() }).unwrap();
    let r = &nada.relatorio;
    assert_eq!((r.antes.soltas, r.depois.soltas, r.depois.solidos), (6, 6, 0), "nível Nenhum só analisa");
    assert_eq!(r.defeitos.len(), 6);
    assert!(r.defeitos.iter().all(|d| d.tipo == TipoDefeito::FaceSolta && !d.segmentos.is_empty()));

    let padrao = Registro::default().importar(&arquivo, &OpcoesImportar::default()).unwrap();
    let r = &padrao.relatorio;
    assert_eq!((r.antes.soltas, r.antes.solidos, r.depois.soltas, r.depois.solidos), (6, 0, 0, 1), "{r:?}");
    assert_eq!(r.faces_costuradas, 6);
    assert!(r.defeitos.is_empty() && r.nao_fecharam.is_empty());
    assert_eq!(r.reparo_pecas.len(), 1, "a peça que tinha defeito fica registrada");
    let (volume, ..) = recalcular(&padrao.doc);
    assert!(perto(volume, 1000.), "{volume}");
}

#[test]
fn agressivo_fecha_a_folga_que_o_padrao_recusa() {
    let dir = pasta("folga");
    let arquivo = dir.join("folga.brep");
    // a tampa flutua 0,05 mm acima das paredes: maior que a tolerância e menor que 100 vezes ela
    let mut faces = faces_de_cubo();
    faces[0] = faces[0].com_matriz(&[1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0.05]).unwrap();
    std::fs::write(&arquivo, composto(faces).to_brep().unwrap()).unwrap();

    let o = OpcoesImportar { tolerancia: 0.01, ..Default::default() };
    let padrao = Registro::default().importar(&arquivo, &o).unwrap();
    assert_eq!(padrao.relatorio.depois.solidos, 0, "{:?}", padrao.relatorio.depois);
    assert!(padrao.relatorio.depois.tem_defeito());
    assert!(!padrao.relatorio.defeitos.is_empty(), "o que não fechou aparece na lista");

    let agressivo = Registro::default().importar(&arquivo, &OpcoesImportar { reparo: Reparo::Agressivo, ..o }).unwrap();
    assert_eq!(agressivo.relatorio.depois.solidos, 1, "{:?}", agressivo.relatorio.depois);
    assert!(agressivo.relatorio.depois.tolerancia > padrao.relatorio.depois.tolerancia, "o agressivo paga subindo a tolerância");
    let (volume, ..) = recalcular(&agressivo.doc);
    assert!((volume - 1000.).abs() < 10., "{volume}");
}

#[test]
fn relatorio_fica_embutido_no_documento_e_sobrevive_ao_arquivo() {
    let dir = pasta("embutido");
    let arquivo = dir.join("peça.step");
    gravar(&montagem(), &arquivo, OpcoesExportar::default());
    let i = Registro::default().importar(&arquivo, &uma_peca()).unwrap();
    let doc = Document::from_json(&i.doc.to_json()).unwrap();
    std::fs::remove_file(&arquivo).unwrap();
    let embutido = doc.origem.as_ref().and_then(|o| o.relatorio.as_deref()).expect("relatório no selo");
    assert_eq!(Relatorio::de_json(embutido).as_ref(), Some(&i.relatorio));
}

#[test]
fn iges_de_faces_entra_como_superficie_e_a_costura_faz_o_solido() {
    let dir = pasta("iges faces");
    let arquivo = dir.join("bloco ç.igs");
    let m = ModeloNeutro::de_forma(Formato::Step, "Bloco", caixa([10., 20., 30.]));
    gravar(&m, &arquivo, OpcoesExportar { iges: ModoIges::Faces, ..Default::default() });

    let sem = Registro::default().importar(&arquivo, &OpcoesImportar { reparo: Reparo::Nenhum, ..Default::default() }).unwrap();
    assert_eq!(sem.relatorio.formato, "IGES");
    assert!(sem.relatorio.esquema.starts_with("IGES"), "{}", sem.relatorio.esquema);
    assert_eq!(sem.relatorio.solidos, 0);
    let r = fenkra_solidos::recompute(&sem.doc);
    assert!(r.bodies.is_empty() && r.surfaces.len() == 1, "{} corpos, {} superfícies", r.bodies.len(), r.surfaces.len());

    let costurado = Registro::default().importar(&arquivo, &OpcoesImportar::default()).unwrap();
    assert_eq!(costurado.relatorio.solidos, 1, "{:?}", costurado.relatorio);
    let (volume, lo, hi) = recalcular(&costurado.doc);
    assert!(perto(volume, 6000.), "{volume}");
    assert!(perto(hi[0] - lo[0], 10.) && perto(hi[1] - lo[1], 20.) && perto(hi[2] - lo[2], 30.), "{lo:?} {hi:?}");
}

#[test]
fn iges_aberto_fica_como_referencia_e_avisa() {
    let dir = pasta("iges aberto");
    let arquivo = dir.join("casca.igs");
    let m = ModeloNeutro::de_forma(Formato::Step, "Casca", superficie_aberta());
    gravar(&m, &arquivo, OpcoesExportar { iges: ModoIges::Faces, ..Default::default() });
    let i = Registro::default().importar(&arquivo, &OpcoesImportar::default()).unwrap();
    assert_eq!(i.relatorio.solidos, 0);
    assert!(i.relatorio.texto_pt().contains("ficou como superfície de referência"), "{}", i.relatorio.texto_pt());
    assert!(!i.relatorio.defeitos.is_empty(), "as bordas abertas ficam na lista");
    let r = fenkra_solidos::recompute(&i.doc);
    assert!(r.bodies.is_empty() && r.surfaces.len() == 1);
}

#[test]
fn iges_brep_mantem_o_solido_a_unidade_e_varias_pecas() {
    let dir = pasta("iges brep");
    let arquivo = dir.join("montagem.iges");
    gravar(&montagem(), &arquivo, OpcoesExportar { unidade: UnidadeSaida::Polegada, ..Default::default() });
    let i = Registro::default().importar(&arquivo, &OpcoesImportar::default()).unwrap();
    assert!(i.relatorio.unidades.to_uppercase().starts_with("IN"), "{}", i.relatorio.unidades);
    assert_eq!(i.relatorio.solidos, 3, "{:?}", i.relatorio);
    let (volume, _, hi) = recalcular(&i.doc);
    assert!((volume - 68_000.).abs() < 1., "{volume}");
    assert!((hi[0] - 100.).abs() < 1e-3 && (hi[2] - 40.).abs() < 1e-3, "{hi:?}");
}

#[test]
fn iges_renomeado_e_reconhecido_pelo_conteudo() {
    let dir = pasta("iges nome");
    let certo = dir.join("a.igs");
    gravar(&ModeloNeutro::de_forma(Formato::Step, "A", caixa([1., 2., 3.])), &certo, OpcoesExportar::default());
    let errado = dir.join("a.dat");
    std::fs::rename(&certo, &errado).unwrap();
    let i = Registro::default().importar(&errado, &OpcoesImportar::default()).unwrap();
    assert_eq!(i.relatorio.formato, "IGES");
    assert!(i.relatorio.texto_pt().contains("não combina com o conteúdo"));
}

#[test]
fn documento_aberto_vai_para_iges_nos_dois_modos_e_volta_com_a_mesma_massa() {
    let dir = pasta("doc iges");
    let origem = dir.join("bloco.brep");
    std::fs::write(&origem, caixa([10., 20., 30.]).to_brep().unwrap()).unwrap();
    let doc = Registro::default().importar(&origem, &OpcoesImportar::default()).unwrap().doc;
    let modelo = ModeloNeutro::de_documento(&doc).unwrap();
    for (nome, iges) in [("a.igs", ModoIges::Brep), ("b.iges", ModoIges::Faces)] {
        let arquivo = dir.join(nome);
        gravar(&modelo, &arquivo, OpcoesExportar { iges, ..Default::default() });
        let i = Registro::default().importar(&arquivo, &OpcoesImportar::default()).unwrap();
        assert_eq!(i.relatorio.solidos, 1, "{nome}: {:?}", i.relatorio);
        assert!(perto(i.relatorio.volume, 6000.), "{nome}: {}", i.relatorio.volume);
    }
    assert!(ModeloNeutro::de_documento(&Document::new("vazio")).is_err());
}

#[test]
fn iges_com_cota_lista_a_entidade_que_ficou_de_fora() {
    let dir = pasta("iges cota");
    let arquivo = dir.join("cota.igs");
    std::fs::write(&arquivo, iges_com_nota()).unwrap();
    let i = Registro::default().importar(&arquivo, &OpcoesImportar::default()).unwrap();
    let t = i.relatorio.texto_pt();
    assert!(t.contains("Entidades que o leitor de IGES não traduziu"), "{t}");
}

/// Reta (110) e nota geral (212) à mão, em cartões de 80 colunas: nosso gravador só escreve geometria.
fn iges_com_nota() -> String {
    let cartao = |texto: &str, secao: char, n: usize| format!("{texto:<72}{secao}{n:>7}\n");
    let mut s = cartao("", 'S', 1);
    s += &cartao("1H,,1H;,1Hn,1Hf,1Hs,1Hs,32,38,6,308,15,1Hn,1.,2,2HMM,", 'G', 1);
    s += &cartao("1,1.,13H260101.000000,0.001,100.,5Hteste,5Hteste,11,0,13H260101.000000;", 'G', 2);
    let d = |tipo: &str, p: usize, seq: usize, linhas: usize, forma: usize| {
        let a = format!("{tipo:>8}{p:>8}{:>8}{:>8}{:>8}{:>8}{:>8}{:>8}{:>8}", 0, 0, 0, 0, 0, 0, "00000000");
        let b = format!("{tipo:>8}{:>8}{:>8}{linhas:>8}{forma:>8}{:>8}{:>8}{:>8}{:>8}", 0, 0, "", "", "", 0);
        cartao(&a, 'D', seq) + &cartao(&b, 'D', seq + 1)
    };
    s += &d("110", 1, 1, 1, 0);
    s += &d("212", 2, 3, 2, 0);
    s += &format!("{:<64}{:>8}P{:>7}\n", "110,0.,0.,0.,10.,0.,0.;", 1, 1);
    s += &format!("{:<64}{:>8}P{:>7}\n", "212,1,4,10.,5.,1,0.,0.,0,0,0.,0.,0.,", 3, 2);
    s += &format!("{:<64}{:>8}P{:>7}\n", "4Hnota;", 3, 3);
    s += &cartao("S      1G      2D      4P      3", 'T', 1);
    s
}
