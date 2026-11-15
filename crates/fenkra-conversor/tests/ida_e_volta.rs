// SPDX-License-Identifier: LGPL-2.1-or-later
//! B211: relatório de ida e volta por formato. Cada linha grava um modelo montado em código, lê de volta
//! e compara: no 2D, o número de entidades e a extensão; no 3D, o estado dos sólidos, o volume e a caixa.
//! O que um formato não faz nesta versão também tem linha, com o motivo. O texto publicado em
//! `docs/modulos/ida-e-volta.md` sai daqui; `FENKRA_ATUALIZAR_IDA_E_VOLTA=1` regrava.

use fenkra_conversor::folha::{abrir_como_desenho, caixa};
use fenkra_conversor::{detectar, Destino, Formato, InstanciaNeutra, ModeloNeutro, OpcoesExportar, OpcoesImportar, ProdutoNeutro, Provedor, Registro};
use fenkra_core::sketch::{Curve, SketchData};
use fenkra_core::{Document, FeatureKind, Id, OriginPlane, PlaneRef, Sketch};
use fenkra_desenho::esboco::DonoEsboco;
use fenkra_desenho::layout::{sheet, Item};
use fenkra_desenho::source::Models;
use fenkra_desenho::{export, Drawing, Format, Note, NoteKind};
use fenkra_kernel::{ModeloXde, Shape, IDENTIDADE};
use std::path::{Path, PathBuf};

fn pasta(teste: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("Fenkra ida e volta açaí {} {teste}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

fn num(v: f64) -> String {
    format!("{v:.3}").replace('.', ",")
}

fn extensao(lo: [f64; 2], hi: [f64; 2]) -> String {
    format!("{} × {}", num(hi[0] - lo[0]), num(hi[1] - lo[1]))
}

/// Uma linha do relatório.
struct Linha {
    formato: &'static str,
    caminho: &'static str,
    ida: String,
    volta: String,
    estado: &'static str,
}

/// Esboço "Contorno": retângulo 80 × 40, furo de raio 6 e um arco de raio 10 por cima.
fn peca_2d() -> Document {
    let mut d = Document::new("Placa");
    let mut sk = SketchData::default();
    let mut al = || d.alloc();
    sk.add_rect(&mut al, [0., 0.], [80., 40.], false);
    let c = sk.add_point(&mut al, 20., 20.);
    sk.add_curve(&mut al, Curve::Circle { c, r: 6. }, false);
    let (o, s, e) = (sk.add_point(&mut al, 60., 40.), sk.add_point(&mut al, 70., 40.), sk.add_point(&mut al, 50., 40.));
    sk.add_curve(&mut al, Curve::Arc { c: o, s, e }, false);
    let id = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data: sk }));
    if let Some(f) = d.features.iter_mut().find(|f| f.id == id) {
        f.name = "Contorno".into();
    }
    d
}

/// Curvas e extensão dos esboços de um documento, com o arco pela caixa do traço.
fn medir_esbocos(doc: &Document) -> (usize, [f64; 2], [f64; 2]) {
    let (mut lo, mut hi, mut n) = ([f64::MAX; 2], [f64::MIN; 2], 0);
    for f in &doc.features {
        let FeatureKind::Sketch(s) = &f.kind else { continue };
        for c in s.data.curves.iter().filter(|c| !c.construction) {
            n += 1;
            let mut pts = vec![];
            match &c.geom {
                Curve::Line { a, b } => pts.extend([s.data.pos(*a), s.data.pos(*b)]),
                Curve::Circle { c, r } => {
                    let o = s.data.pos(*c);
                    pts.extend([[o[0] - r, o[1] - r], [o[0] + r, o[1] + r]]);
                }
                Curve::Arc { c, s: a, e } => {
                    let (o, p, q) = (s.data.pos(*c), s.data.pos(*a), s.data.pos(*e));
                    let r = (p[0] - o[0]).hypot(p[1] - o[1]);
                    let (t0, mut t1) = ((p[1] - o[1]).atan2(p[0] - o[0]), (q[1] - o[1]).atan2(q[0] - o[0]));
                    while t1 <= t0 {
                        t1 += std::f64::consts::TAU;
                    }
                    pts.extend((0..=64).map(|i| t0 + (t1 - t0) * f64::from(i) / 64.).map(|t| [o[0] + r * t.cos(), o[1] + r * t.sin()]));
                }
                _ => {}
            }
            for p in pts {
                for i in 0..2 {
                    lo[i] = lo[i].min(p[i]);
                    hi[i] = hi[i].max(p[i]);
                }
            }
        }
    }
    (n, lo, hi)
}

fn esboco(dir: &Path, f: Formato) -> Linha {
    let r = Registro::default();
    let a = dir.join(format!("placa ç.{}", f.extensoes()[0]));
    let doc = peca_2d();
    r.gravar_documento(&doc, &a, &OpcoesExportar::default()).unwrap();
    let lido = r.importar(&a, &OpcoesImportar::default()).unwrap();
    let ((n0, lo0, hi0), (n1, lo1, hi1)) = (medir_esbocos(&doc), medir_esbocos(&lido.doc));
    assert_eq!(n1, n0, "{}: entidades", f.nome());
    for i in 0..2 {
        assert!((lo1[i] - lo0[i]).abs() < 1e-6 && (hi1[i] - hi0[i]).abs() < 1e-6, "{}: {lo1:?} {hi1:?}", f.nome());
    }
    Linha { formato: f.nome(), caminho: "esboço da peça", ida: format!("{n0} curvas, {}", extensao(lo0, hi0)), volta: format!("{n1} curvas, {}", extensao(lo1, hi1)), estado: "igual" }
}

/// Folha A3 com um esboço (retângulo e furo) e uma nota de texto.
fn folha() -> Drawing {
    let mut d = Drawing::new("Prova", Format::A3);
    let id_folha = d.sheets[0].id;
    let id = d.add_esboco(DonoEsboco::Folha(id_folha));
    let mut n = 10_000;
    let mut al = move || {
        n += 1;
        Id(n)
    };
    let e = d.esboco_mut(id).unwrap();
    e.cotas = false;
    e.dados.add_rect(&mut al, [60., 50.], [180., 130.], false);
    let c = e.dados.add_point(&mut al, 120., 90.);
    e.dados.add_curve(&mut al, Curve::Circle { c, r: 15. }, false);
    let nota = d.alloc();
    d.notes.push(Note { id: nota, sheet: id_folha, kind: NoteKind::Text, pos: [70., 140.], text: "CHAPA 3 mm".into(), size: 5., ..Default::default() });
    d
}

fn do_conteudo(d: &Drawing) -> Vec<Item> {
    let donos: Vec<Id> = d.esbocos.iter().map(|e| e.id).chain(d.notes.iter().map(|n| n.id)).collect();
    let mut m = Models::new(None);
    sheet(d, d.sheets[0].id, &mut m).into_iter().filter(|i| i.owner.is_some_and(|o| donos.contains(&o))).collect()
}

fn medir_folha(d: &Drawing) -> (usize, usize, ([f64; 2], [f64; 2])) {
    let curvas = d.esbocos.iter().map(|e| e.dados.curves.iter().filter(|c| !c.construction).count()).sum();
    let des = fenkra_dxf::ler(export::dxf(&do_conteudo(d)).as_bytes()).unwrap();
    (curvas, d.notes.len(), caixa(&des).unwrap())
}

fn na_folha(dir: &Path, f: Formato) -> Linha {
    let d = folha();
    let a = dir.join(format!("folha.{}", f.extensoes()[0]));
    let texto = if f == Formato::Svg { export::svg(&do_conteudo(&d), d.sheets[0].size()) } else { export::dxf(&do_conteudo(&d)) };
    std::fs::write(&a, texto).unwrap();
    let (aberto, _) = abrir_como_desenho(&a).unwrap();
    let ((c0, t0, x0), (c1, t1, x1)) = (medir_folha(&d), medir_folha(&aberto));
    assert_eq!((c1, t1), (c0, t0), "{}: curvas e textos", f.nome());
    // o SVG da folha grava três casas
    let tol = if f == Formato::Svg { 1e-2 } else { 1e-6 };
    for i in 0..2 {
        assert!((x1.0[i] - x0.0[i]).abs() < tol && (x1.1[i] - x0.1[i]).abs() < tol, "{}: {x1:?}", f.nome());
    }
    let fmt = |c: usize, t: usize, x: ([f64; 2], [f64; 2])| format!("{c} curvas e {t} texto, {}", extensao(x.0, x.1));
    Linha { formato: f.nome(), caminho: "folha do desenho", ida: fmt(c0, t0, x0), volta: fmt(c1, t1, x1), estado: if f == Formato::Svg { "igual a 0,01 mm" } else { "igual" } }
}

/// Dois pilares de 300 × 300 × 3000, a 4000 mm um do outro, com cor.
fn obra() -> ModeloNeutro {
    let forma = Shape::box_tool([0., 0., 0.], [1., 0., 0.], [0., 0., 1.], [300., 300., 3000.]).unwrap();
    let pilar = ProdutoNeutro { nome: "Pilar".into(), forma: Some(forma), cor: Some([0.6, 0.6, 0.2, 1.]), ..Default::default() };
    let em = |x: f64| [1., 0., 0., x, 0., 1., 0., 0., 0., 0., 1., 0.];
    let filhos = vec![InstanciaNeutra { produto: 0, nome: "P1".into(), local: IDENTIDADE, cor: None }, InstanciaNeutra { produto: 0, nome: "P2".into(), local: em(4000.), cor: None }];
    let topo = ProdutoNeutro { nome: "Obra".into(), filhos, ..Default::default() };
    ModeloNeutro { formato: Formato::Step, xde: ModeloXde { produtos: vec![pilar, topo], raizes: vec![1], ..Default::default() } }
}

fn solidos(sol: u32, fora: usize, v: f64, m: [f64; 3]) -> String {
    format!("{sol} sólidos fechados{}, {} mm³, {} × {} × {}", if fora > 0 { format!(" e {fora} abertos") } else { String::new() }, format!("{v:.0}"), num(m[0]), num(m[1]), num(m[2]))
}

fn ifc_obra(dir: &Path) -> Linha {
    let a = dir.join("obra.ifc");
    Registro::default().gravar(&obra(), &a, &OpcoesExportar::default()).unwrap();
    let i = Registro::default().importar(&a, &OpcoesImportar { destino: Destino::UmaPeca, ..Default::default() }).unwrap();
    let r = &i.relatorio;
    let m = r.medidas().unwrap();
    let v = 2. * 300. * 300. * 3000.;
    assert_eq!(r.solidos, 2);
    assert!(r.nao_fecharam.is_empty(), "{:?}", r.nao_fecharam);
    assert!((r.volume - v).abs() < 1. && (m[0] - 4300.).abs() < 1e-3 && (m[1] - 300.).abs() < 1e-3 && (m[2] - 3000.).abs() < 1e-3, "{} {m:?}", r.volume);
    Linha { formato: "IFC", caminho: "peças (gravar e ler)", ida: solidos(2, 0, v, [4300., 300., 3000.]), volta: solidos(r.solidos, r.nao_fecharam.len(), r.volume, m), estado: "igual" }
}

/// Viga I de 200 × 190 e tubo retangular 100 × 50 × 5, 1000 mm cada, lado a lado: o que um programa de
/// edificação grava como extrusão de perfil.
const PERFIS: &str = "ISO-10303-21;\nHEADER;\nFILE_SCHEMA(('IFC4'));\nENDSEC;\nDATA;\n#1=IFCSIUNIT(*,.LENGTHUNIT.,.MILLI.,.METRE.);\n#2=IFCDIRECTION((0.,0.,1.));\n\
#10=IFCISHAPEPROFILEDEF(.AREA.,$,$,200.,190.,6.5,10.,$,$,$);\n#11=IFCEXTRUDEDAREASOLID(#10,$,#2,1000.);\n#12=IFCSHAPEREPRESENTATION($,'Body','SweptSolid',(#11));\n\
#13=IFCPRODUCTDEFINITIONSHAPE($,$,(#12));\n#14=IFCLOCALPLACEMENT($,$);\n#15=IFCBEAM('a',$,'Viga',$,$,#14,#13,$,$);\n\
#20=IFCRECTANGLEHOLLOWPROFILEDEF(.AREA.,$,$,100.,50.,5.,$,$);\n#21=IFCEXTRUDEDAREASOLID(#20,$,#2,1000.);\n#22=IFCSHAPEREPRESENTATION($,'Body','SweptSolid',(#21));\n\
#23=IFCPRODUCTDEFINITIONSHAPE($,$,(#22));\n#24=IFCCARTESIANPOINT((300.,0.,0.));\n#25=IFCAXIS2PLACEMENT3D(#24,$,$);\n#26=IFCLOCALPLACEMENT($,#25);\n\
#27=IFCMEMBER('b',$,'Tubo',$,$,#26,#23,$,$);\nENDSEC;\nEND-ISO-10303-21;\n";

fn ifc_perfis(dir: &Path) -> Linha {
    let a = dir.join("perfis.ifc");
    std::fs::write(&a, PERFIS).unwrap();
    let i = Registro::default().importar(&a, &OpcoesImportar { destino: Destino::UmaPeca, ..Default::default() }).unwrap();
    let r = &i.relatorio;
    let m = r.medidas().unwrap();
    let v = (2. * 200. * 10. + 170. * 6.5 + 100. * 50. - 90. * 40.) * 1000.;
    // a viga vai de -100 a 100 em X e o tubo de 250 a 350; em Y a viga tem 190
    let caixa = [450., 190., 1000.];
    assert_eq!(r.solidos, 2);
    assert!((r.volume - v).abs() < 1e-3, "{} contra {v}", r.volume);
    assert!((0..3).all(|k| (m[k] - caixa[k]).abs() < 1e-6), "{m:?}");
    Linha { formato: "IFC", caminho: "extrusão de perfil de aço (ler)", ida: solidos(2, 0, v, caixa), volta: solidos(r.solidos, r.nao_fecharam.len(), r.volume, m), estado: "igual" }
}

/// Medida de um modelo 3D com camadas e cores: sólidos, abertos, volume preciso dos sólidos, caixa de tudo,
/// caminhos das camadas e número de cores.
type Medida3d = (u32, usize, f64, [f64; 3], Vec<String>, usize);

fn medir_3d(m: &ModeloNeutro) -> Medida3d {
    let (mut sol, mut fora, mut v) = (0, 0, 0.);
    let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
    let (mut camadas, mut cores) = (vec![], vec![]);
    for p in &m.xde.produtos {
        let Some(f) = p.forma.as_ref() else { continue };
        if f.is_solid() {
            sol += 1;
            v += f.volume_preciso();
        } else {
            fora += 1;
        }
        let (a, b) = f.bbox().unwrap();
        for k in 0..3 {
            lo[k] = lo[k].min(a[k]);
            hi[k] = hi[k].max(b[k]);
        }
        camadas.extend(p.camadas.iter().cloned());
        if let Some(c) = p.cor {
            let c = [0, 1, 2].map(|k| (c[k] * 255.).round() as u8);
            if !cores.contains(&c) {
                cores.push(c);
            }
        }
    }
    camadas.sort();
    (sol, fora, v, [0, 1, 2].map(|k| hi[k] - lo[k]), camadas, cores.len())
}

/// Placa 80 × 40 × 10 com furo de 12 mm na camada "Chapas > Corte" e superfície ondulada na camada
/// "Superfícies", com cores diferentes: grava em 3DM pelo openNURBS e lê de volta.
fn tresdm_peca(dir: &Path) -> Option<Linha> {
    use fenkra_kernel::{Bool, HoleShape};
    if !fenkra_3dm::disponivel() {
        return None;
    }
    let placa = Shape::box_tool([0., 0., 0.], [1., 0., 0.], [0., 0., 1.], [80., 40., 10.]).unwrap();
    let furo = Shape::hole_tool([20., 20., 10.], [0., 0., -1.], 12., 10., HoleShape::Simple).unwrap();
    let linhas: Vec<Vec<[f64; 3]>> = (0..5).map(|i| (0..4).map(|j| [100. + 10. * i as f64, 10. * j as f64, if (i + j) % 2 == 0 { 0. } else { 4. }]).collect()).collect();
    let chapa = ProdutoNeutro { nome: "Chapa".into(), forma: Some(placa.boolean(&furo, Bool::Cut).unwrap()), cor: Some([0.8, 0.2, 0.2, 1.]), camadas: vec!["Chapas > Corte".into()], ..Default::default() };
    let onda = ProdutoNeutro { nome: "Onda".into(), forma: Some(Shape::grid_surface(&linhas).unwrap()), cor: Some([0.2, 0.4, 1., 1.]), camadas: vec!["Superfícies".into()], ..Default::default() };
    let filhos = vec![InstanciaNeutra { produto: 0, nome: "Chapa".into(), local: IDENTIDADE, cor: None }, InstanciaNeutra { produto: 1, nome: "Onda".into(), local: IDENTIDADE, cor: None }];
    let topo = ProdutoNeutro { nome: "Prova".into(), filhos, ..Default::default() };
    let modelo = ModeloNeutro { formato: Formato::Step, xde: ModeloXde { produtos: vec![chapa, onda, topo], raizes: vec![2], ..Default::default() } };
    let a = dir.join("prova ç.3dm");
    Registro::default().gravar(&modelo, &a, &OpcoesExportar::default()).unwrap();
    let lido = fenkra_conversor::Provedor3dm.ler(Formato::Tresdm, &a, &OpcoesImportar::default()).unwrap();
    let (ida, volta) = (medir_3d(&modelo), medir_3d(&lido));
    assert_eq!((volta.0, volta.1), (ida.0, ida.1), "3DM: sólidos e abertos");
    assert!((volta.2 - ida.2).abs() < 1e-6 * ida.2, "3DM: volume {} contra {}", volta.2, ida.2);
    assert!((0..3).all(|k| (volta.3[k] - ida.3[k]).abs() < 1e-4), "3DM: caixa {:?} contra {:?}", volta.3, ida.3);
    assert_eq!((&volta.4, volta.5), (&ida.4, ida.5), "3DM: camadas e cores");
    let fmt = |m: &Medida3d| format!("{}; camadas {}; {} cores", solidos(m.0, m.1, m.2, m.3), m.4.join(" e "), m.5);
    Some(Linha { formato: "3DM", caminho: "peça com camadas (gravar e ler)", ida: fmt(&ida), volta: fmt(&volta), estado: "igual" })
}

fn relatorio() -> String {
    let dir = pasta("relatorio");
    let mut linhas = vec![esboco(&dir, Formato::Dxf), esboco(&dir, Formato::Svg), na_folha(&dir, Formato::Dxf), na_folha(&dir, Formato::Svg), ifc_obra(&dir), ifc_perfis(&dir)];

    // 3DM: reconhecido pelo cabeçalho, mesmo com outra extensão
    assert_eq!(detectar(Path::new("casco.bin"), b"3D Geometry File Format        8\0\0").unwrap().0, Formato::Tresdm);
    let fora = |formato, caminho, motivo: &str| Linha { formato, caminho, ida: "–".into(), volta: motivo.into(), estado: "fora" };
    linhas.push(Linha { formato: "DWG", caminho: "esboço e folha", ida: "DXF R2000 pelo dxf2dwg".into(), volta: "DXF pelo dwg2dxf".into(), estado: "com o LibreDWG instalado" });
    linhas.push(tresdm_peca(&dir).unwrap_or_else(|| fora("3DM", "peça com camadas (gravar e ler)", "compilado sem o openNURBS (scripts/baixar_opennurbs.py)")));
    linhas.push(fora("PDF 3D", "peça", "sem biblioteca aberta de PRC com licença compatível (ADR 0055)"));
    linhas.push(fora("STEP em XML", "peça", "o OpenCASCADE 8.0.1 não traz a parte 28 (ADR 0055)"));

    let mut t = String::from(
        "# Ida e volta por formato (B211)\n\n> Gerado de `crates/fenkra-conversor/tests/ida_e_volta.rs`; não edite à mão. Cada linha grava um modelo\n\
> montado em código, lê de volta e compara. No 2D: curvas, textos e extensão em milímetros. No 3D: sólidos,\n\
> volume e caixa. Para regravar: `FENKRA_ATUALIZAR_IDA_E_VOLTA=1 cargo test -p fenkra-conversor --test ida_e_volta`.\n\
> O DWG não entra na conta porque depende do LibreDWG instalado à parte; o encanamento é conferido em\n\
> `tests/dwg.rs` com um programa falso, e com o de verdade quando ele existe. O 3DM pede o openNURBS baixado por\n\
> `scripts/baixar_opennurbs.py`; o volume dele é o da integração adaptativa (`Shape::volume_preciso`).\n\n\
| Formato | Caminho | Ida | Volta | Resultado |\n|---|---|---|---|---|\n",
    );
    for l in &linhas {
        t.push_str(&format!("| {} | {} | {} | {} | {} |\n", l.formato, l.caminho, l.ida, l.volta, l.estado));
    }
    t
}

/// Sem esse teste a página ficaria velha. `FENKRA_ATUALIZAR_IDA_E_VOLTA=1` regrava.
#[test]
fn cada_formato_vai_e_volta_e_o_relatorio_publicado_sai_daqui() {
    let gerado = relatorio();
    // sem o openNURBS a linha do 3DM sai diferente: as outras foram conferidas, a página publicada não
    if !fenkra_3dm::disponivel() {
        eprintln!("3DM IGNORADO: compilado sem o openNURBS; a página publicada não foi comparada");
        return;
    }
    let caminho = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/modulos/ida-e-volta.md");
    if std::env::var("FENKRA_ATUALIZAR_IDA_E_VOLTA").is_ok() {
        std::fs::write(&caminho, &gerado).unwrap();
    }
    let lido = std::fs::read_to_string(&caminho).unwrap_or_default().replace("\r\n", "\n");
    assert_eq!(lido, gerado, "rode com FENKRA_ATUALIZAR_IDA_E_VOLTA=1 para regravar docs/modulos/ida-e-volta.md");
}
