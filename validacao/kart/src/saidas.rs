// SPDX-License-Identifier: LGPL-2.1-or-later
//! Etapas 13 e 14: desenho, PMI, PLM com a regressão do entre-eixos, pacote de visualização.
use crate::apoio::*;
use fenkra_core::pmi::{GdtChar, Pmi, PmiKind, PmiTarget, TargetClass};
use fenkra_core::*;
use fenkra_desenho::export::save;
use fenkra_desenho::layout::sheet;
use fenkra_desenho::source::Models;
use fenkra_desenho::{Dir, Drawing, Format, Source, Table, TableKind, ViewKind};
use fenkra_montagem::bom::BomLevel;
use fenkra_montagem::links::{atualizar, Gatilho};
use fenkra_montagem::load::PartCache;
use fenkra_montagem::Assembly;
use fenkra_project::estados::Externos;
use fenkra_project::{DocState, Project};
use fenkra_solidos::recompute;
use std::path::Path;

fn src(p: &str) -> Source {
    Source { path: p.into(), display_state: String::new(), hidden: vec![] }
}

fn tabela(d: &mut Drawing, sh: Id, kind: TableKind, p: &str, pos: [f64; 2], level: BomLevel) {
    let id = d.alloc();
    d.tables.push(Table { id, sheet: sh, kind, source: src(p), pos, level, cfg: Default::default() });
}

fn relata(r: &mut Reg, cod: &str, nome: &str, d: &Drawing, m: &mut Models, ext: &str) {
    let dir = modelo();
    let _ = std::fs::write(dir.join(format!("{cod}.fkd")), d.to_json());
    let sh = d.sheets[0].id;
    let t = std::time::Instant::now();
    let itens: usize = d.sheets.iter().map(|s| sheet(d, s.id, m).len()).sum();
    let vazias: Vec<String> = d
        .views
        .iter()
        .filter_map(|v| {
            let p = m.proj(d, v.id);
            (p.lines.is_empty()).then(|| format!("{:?}", v.kind).chars().take(24).collect())
        })
        .collect();
    let tabs: Vec<String> = d.tables.iter().map(|t| format!("{}:{}", t.kind.label(), m.table(t, d).rows.len())).collect();
    let saida = dir.join(format!("{cod}.{ext}"));
    let e = save(d, sh, m, saida.to_str().unwrap());
    r.usa("Desenho", &["Folha", "Vista", "Tabela", "Exportar"]);
    r.registra(Linha { cod: cod.into(), nome: nome.into(), bancada: "Desenho".into(), ok: e.is_ok() && itens > 0 && vazias.is_empty(), volume: 0., caixa: [0.; 3], corpos: d.views.len(), nota: format!("itens={itens} tabelas={tabs:?} vistas vazias={vazias:?} exportar={e:?} [{:.1}s]", t.elapsed().as_secs_f64()) });
}

pub fn desenhos(r: &mut Reg) {
    let dir = modelo();
    let mut m = Models::new(Some(dir.clone()));
    // conjunto com lista de peças
    let mut d = Drawing::new("Kart, conjunto", Format::A1);
    let sh = d.sheets[0].id;
    d.add_standard_set(sh, src("KART-PLANO.fka"), [300., 300.], 0.2, 60.);
    tabela(&mut d, sh, TableKind::Bom, "KART.fka", [560., 40.], BomLevel::Estruturada);
    relata(r, "DW-KART", "Folha de conjunto", &d, &mut m, "pdf");
    // quadro: lista de corte
    let mut d = Drawing::new("Quadro", Format::A2);
    let sh = d.sheets[0].id;
    let v = d.add_view(sh, src("QUADRO.fkp"), ViewKind::Standard(Dir::Topo), [200., 180.], 0.25);
    d.add_view(sh, src("QUADRO.fkp"), ViewKind::Standard(Dir::Frente), [200., 320.], 0.25);
    d.add_view(sh, src("QUADRO.fkp"), ViewKind::Detail { parent: v, center: [100., 100.], radius: 120., label: "A".into() }, [470., 300.], 0.5);
    tabela(&mut d, sh, TableKind::CutList, "QUADRO.fkp", [400., 30.], BomLevel::default());
    relata(r, "DW-QUADRO", "Quadro com lista de corte", &d, &mut m, "pdf");
    // eixo, disco e manga: detalhe e corte
    let mut d = Drawing::new("Eixo", Format::A3);
    let sh = d.sheets[0].id;
    let v = d.add_view(sh, src("EX-01.fkp"), ViewKind::Standard(Dir::Frente), [200., 120.], 0.3);
    d.add_view(sh, src("EX-01.fkp"), ViewKind::Section { parent: v, a: [-30., 0.], b: [30., 0.], label: "B".into() }, [200., 220.], 1.);
    relata(r, "DW-EX01", "Eixo traseiro", &d, &mut m, "svg");
    let mut d = Drawing::new("Disco", Format::A3);
    let sh = d.sheets[0].id;
    d.add_view(sh, src("EX-08.fkp"), ViewKind::Standard(Dir::Frente), [150., 150.], 1.);
    tabela(&mut d, sh, TableKind::Holes, "EX-08.fkp", [290., 40.], BomLevel::default());
    relata(r, "DW-EX08", "Disco com tabela de furos", &d, &mut m, "dxf");
    let mut d = Drawing::new("Manga", Format::A3);
    let sh = d.sheets[0].id;
    let v = d.add_view(sh, src("DI-08.fkp"), ViewKind::Standard(Dir::Topo), [150., 150.], 1.);
    d.add_view(sh, src("DI-08.fkp"), ViewKind::Section { parent: v, a: [-120., 0.], b: [120., 0.], label: "C".into() }, [300., 150.], 1.);
    relata(r, "DW-DI08", "Manga de eixo", &d, &mut m, "pdf");
    // planificações
    for (c, n) in [("PO-04", "Assoalho"), ("CH-08", "Suporte do mancal")] {
        let mut d = Drawing::new(n, Format::A3);
        let sh = d.sheets[0].id;
        d.add_view(sh, src(&format!("{c}.fkp")), ViewKind::Flat, [200., 150.], 0.4);
        tabela(&mut d, sh, TableKind::Bends, &format!("{c}.fkp"), [20., 20.], BomLevel::default());
        relata(r, &format!("DW-{c}"), &format!("{n} planificado"), &d, &mut m, "dxf");
    }
    // peça filha: nota simples e legenda
    let mut d = Drawing::new("CH-09", Format::A4);
    let sh = d.sheets[0].id;
    d.add_view(sh, src("CH-09.fkp"), ViewKind::Standard(Dir::Frente), [100., 150.], 1.);
    relata(r, "DW-CH09", "Peça filha com legenda", &d, &mut m, "pdf");
    let _ = dir;
}

/// Alvo como o clique no cilindro de eixo `dir` deixaria.
fn cil(p: [f64; 3], dir: [f64; 3], raio: f64, furo: bool) -> Option<PmiTarget> {
    Some(PmiTarget { class: TargetClass::FaceCilindrica, point: p, dir, radius: raio, concave: furo })
}

fn anota(d: &mut Document, n: &str, kind: PmiKind, a: Vec<[f64; 3]>, txt: &str, alvo: Option<PmiTarget>) {
    let id = d.alloc();
    let a_n = a.len();
    let (tp, tm) = if n.contains("tol") { ("0.05".to_string(), "0".to_string()) } else { (String::new(), String::new()) };
    d.pmi.push(Pmi { id, name: n.into(), kind, anchors: a, offset: [20., 20., 20.], text: txt.into(), tol_plus: tp, tol_minus: tm, visible: true, plane: None, style: Default::default(), folder: String::new(), targets: if alvo.is_some() { vec![alvo; a_n] } else { vec![] } });
}

/// PMI em EX-01, EX-02, DI-08 e CH-09, com nota e legenda; exporta o STEP com PMI.
pub fn pmi(r: &mut Reg) {
    for (cod, f) in [
        ("EX-01", (|d: &mut Document| {
            anota(d, "tol diâmetro", PmiKind::Diameter, vec![[0., 0., 0.], [25., 0., 0.]], "", cil([0.; 3], [0., 1., 0.], 25., false));
            anota(d, "A", PmiKind::Datum { letter: "A".into() }, vec![[25., 300., 0.]], "", cil([0.; 3], [0., 1., 0.], 25., false));
            anota(d, "Batimento", PmiKind::Gdt { ch: GdtChar::BatimentoCircular, tol: "0.05".into(), datums: "A".into(), extra: Default::default() }, vec![[25., 500., 0.]], "", cil([0.; 3], [0., 1., 0.], 25., false));
        }) as fn(&mut Document)),
        ("EX-02", |d| {
            anota(d, "tol alojamento", PmiKind::Diameter, vec![[0., 0., 0.], [40., 0., 0.]], "", cil([0.; 3], [0., 1., 0.], 40., true));
            anota(d, "Rugosidade", PmiKind::Roughness { ra: "1.6".into(), process: "Tornear".into(), extra: Default::default() }, vec![[40., 0., 0.]], "", cil([0.; 3], [0., 1., 0.], 40., true));
        }),
        ("DI-08", |d| {
            anota(d, "tol ponta", PmiKind::Diameter, vec![[0., 40., 0.], [12.5, 40., 0.]], "", cil([0., 40., 0.], [0., 1., 0.], 12.5, false));
            anota(d, "B", PmiKind::Datum { letter: "B".into() }, vec![[16., 0., 0.]], "", cil([0.; 3], [0., 0., 1.], 5., true));
            anota(d, "Perpendicularidade", PmiKind::Gdt { ch: GdtChar::Perpendicularidade, tol: "0.1".into(), datums: "B".into(), extra: Default::default() }, vec![[12.5, 40., 0.]], "", cil([0., 40., 0.], [0., 1., 0.], 12.5, false));
        }),
        ("CH-09", |d| {
            anota(d, "tol furo", PmiKind::Diameter, vec![[28., 0., 35.], [33., 0., 35.]], "", cil([28., 0., 0.], [0., 0., 1.], 5., true));
            anota(d, "Posição", PmiKind::Gdt { ch: GdtChar::Posicao, tol: "0.1".into(), datums: "".into(), extra: Default::default() }, vec![[33., 0., 35.]], "", cil([28., 0., 0.], [0., 0., 1.], 5., true));
            anota(d, "Nota", PmiKind::Note, vec![[0., 0., 35.]], "Quebrar cantos vivos", None);
            anota(d, "Legenda", PmiKind::TitleBlock { template: "{codigo} {descricao} {material} {massa}".into() }, vec![[0., 0., 40.]], "", None);
        }),
    ] {
        let p = modelo().join(format!("{cod}.fkp"));
        let mut d = Document::from_json(&std::fs::read_to_string(&p).unwrap()).unwrap();
        d.pmi.clear();
        f(&mut d);
        let _ = std::fs::write(&p, d.to_json());
        let nota = match fenkra_conversor::ModeloNeutro::de_documento_pmi(&d) {
            Ok((m, fora)) => format!("anotações no STEP={} fora={} de {} PMI", m.xde.anotacoes.len(), fora.len(), d.pmi.len()),
            Err(e) => format!("STEP com PMI falhou: {e:?}"),
        };
        r.usa("PMI", &["Diâmetro", "Referência", "Quadro de tolerância", "Rugosidade", "Nota", "Legenda 3D", "STEP com PMI"]);
        r.registra(Linha { cod: format!("PMI-{cod}"), nome: "PMI".into(), bancada: "PMI".into(), ok: !nota.contains("falhou"), volume: 0., caixa: [0.; 3], corpos: d.pmi.len(), nota });
    }
}

/// Toda peça ganha nota simples e legenda; todo conjunto ganha número de item e balões.
pub fn pmi_geral(r: &mut Reg) {
    let dir = modelo();
    let (mut pecas, mut baloes) = (0, 0);
    for e in std::fs::read_dir(&dir).unwrap().flatten() {
        let p = e.path();
        let nome = p.file_stem().unwrap().to_string_lossy().to_string();
        match p.extension().and_then(|x| x.to_str()) {
            Some("fkp") if nome != "MESTRE" && !nome.starts_with("CV-") => {
                let mut d = Document::from_json(&std::fs::read_to_string(&p).unwrap()).unwrap();
                if !d.pmi.iter().any(|x| matches!(x.kind, PmiKind::TitleBlock { .. })) {
                    anota(&mut d, "Legenda", PmiKind::TitleBlock { template: "{codigo} {descricao} {material} {massa}".into() }, vec![[0.; 3]], "", None);
                    anota(&mut d, "Nota", PmiKind::Note, vec![[0.; 3]], "Tolerância geral ISO 2768-m", None);
                    let _ = std::fs::write(&p, d.to_json());
                }
                pecas += 1;
            }
            Some("fka") if nome.starts_with("SC-") => {
                let mut a = Assembly::from_json(&std::fs::read_to_string(&p).unwrap()).unwrap();
                a.assign_items(Some(&dir));
                let rows = fenkra_montagem::bom::bom(&a, Some(&dir), BomLevel::Topo);
                a.pmi.clear();
                let nos: Vec<(String, [f64; 3])> = a.nodes.iter().map(|n| (n.name.split(':').next().unwrap_or("").to_string(), n.placement.origin)).collect();
                let mut vistos = std::collections::HashSet::new();
                for (n, o) in nos {
                    if !vistos.insert(n.clone()) {
                        continue;
                    }
                    let Some(row) = rows.iter().find(|x| x.key.contains(&n) || x.name.starts_with(&n)) else { continue };
                    let id = a.alloc();
                    a.pmi.push(Pmi { id, name: format!("Balão {n}"), kind: PmiKind::Balloon { item: row.item.clone(), manual: false }, anchors: vec![o], offset: [60., 60., 80.], text: String::new(), tol_plus: String::new(), tol_minus: String::new(), visible: true, plane: None, style: Default::default(), folder: String::new(), targets: vec![] });
                    baloes += 1;
                }
                let _ = std::fs::write(&p, a.to_json());
            }
            _ => {}
        }
    }
    r.usa("PMI", &["Balão", "Legenda 3D", "Nota"]);
    r.registra(Linha { cod: "PMI-GERAL".into(), nome: "Legendas e balões".into(), bancada: "PMI".into(), ok: pecas > 0 && baloes > 0, volume: 0., caixa: [0.; 3], corpos: baloes, nota: format!("{pecas} peças com legenda e nota, {baloes} balões nos subconjuntos") });
}

fn medidas(p: &Path) -> (f64, [f64; 3]) {
    let d = Document::from_json(&std::fs::read_to_string(p).unwrap()).unwrap();
    let r = recompute(&d);
    let (lo, hi) = caixa(&r);
    (r.bodies.iter().map(|b| b.mass.volume).sum(), [hi[0] - lo[0], hi[0], hi[2] - lo[2]])
}

/// Copia o modelo, libera, muda o entre-eixos, propaga e compara.
pub fn plm(r: &mut Reg) -> Result<(), String> {
    let raiz = std::env::temp_dir().join("kart14_plm");
    let _ = std::fs::remove_dir_all(&raiz);
    let copia = raiz.join("modelo");
    std::fs::create_dir_all(&copia).map_err(|e| e.to_string())?;
    for e in std::fs::read_dir(modelo()).map_err(|e| e.to_string())? {
        let p = e.map_err(|e| e.to_string())?.path();
        if p.is_file() {
            std::fs::copy(&p, copia.join(p.file_name().unwrap())).map_err(|e| e.to_string())?;
        }
    }
    let mut pr = Project::create(&raiz.join("proj"), "Kart").map_err(|e| format!("{e:?}"))?;
    let mut ids = vec![];
    let mut falhas = vec![];
    let t = std::time::Instant::now();
    for e in std::fs::read_dir(&copia).map_err(|e| e.to_string())? {
        let p = e.map_err(|e| e.to_string())?.path();
        if p.extension().is_some_and(|x| x == "fkp") {
            let d = Document::from_json(&std::fs::read_to_string(&p).unwrap()).unwrap();
            let nome = d.name.clone();
            match pr.add_document(d) {
                Ok(id) => ids.push((id, nome)),
                Err(e) => falhas.push(format!("{nome}: {e:?}")),
            }
        }
    }
    let mut liberadas = 0;
    for (id, n) in &ids {
        let ext = Externos { recalculo: Some(Ok(())), desenho_atualizado: Some(true), qualidade: None };
        match pr.liberar(*id, "Revisão A do projeto-teste", &ext) {
            Ok(l) => {
                liberadas += 1;
                if liberadas == 1 {
                    println!("primeira liberação: {n} rev {l}");
                }
            }
            Err(e) => falhas.push(format!("liberar {n}: {e:?}")),
        }
    }
    r.usa("PLM", &["Criar projeto", "Adicionar documento", "PN", "Liberar", "Estado", "Revisão", "Onde usado"]);
    r.registra(Linha { cod: "PLM-A".into(), nome: "Liberar revisão A".into(), bancada: "PLM".into(), ok: liberadas == ids.len() && !ids.is_empty(), volume: 0., caixa: [0.; 3], corpos: liberadas, nota: format!("{liberadas}/{} liberados em {:.1}s; falhas: {:?}", ids.len(), t.elapsed().as_secs_f64(), falhas.iter().take(4).collect::<Vec<_>>()) });
    // mudar o entre-eixos: reabrir e revisar o mestre
    let (id_m, _) = ids.iter().find(|(_, n)| n.starts_with("MESTRE")).cloned().ok_or("sem mestre")?;
    let mut ms = pr.load(id_m).map_err(|e| format!("{e:?}"))?;
    let tr = pr.transicionar(id_m, DocState::EmTrabalho, "Mudança do entre-eixos para 1060");
    ms.params.iter_mut().find(|p| p.name == "EE").unwrap().expr = "1060".into();
    let sv = pr.save(id_m, &ms, "EE 1060");
    let rb = pr.liberar(id_m, "Revisão B: entre-eixos 1060", &Externos { recalculo: Some(Ok(())), desenho_atualizado: Some(true), qualidade: None });
    println!("transição={tr:?} gravar={sv:?} rev B={rb:?}");
    let (v0, c0) = medidas(&copia.join("QUADRO.fkp"));
    let (v1, c1) = medidas(&copia.join("PR-01.fkp"));
    std::fs::write(copia.join("MESTRE.fkp"), ms.to_json()).map_err(|e| e.to_string())?;
    let k = Assembly::from_json(&std::fs::read_to_string(copia.join("KART.fka")).unwrap())?;
    let mut cache = PartCache::default();
    let t = std::time::Instant::now();
    let rel = atualizar(&k, Some(&copia), &mut cache, Gatilho::Manual);
    println!("atualizar KART: {} peças em {:.1}s", rel.pecas.len(), t.elapsed().as_secs_f64());
    // sem efeito nos subconjuntos? tenta em cada um, como faria quem os abre
    let mut soma = rel.pecas.len();
    for s in ["SC-QUADRO", "SC-EIXO", "SC-DIRECAO", "SC-MOTOR", "SC-FREIO", "SC-POSTO", "SC-CARROCERIA"] {
        let a = Assembly::from_json(&std::fs::read_to_string(copia.join(format!("{s}.fka"))).unwrap())?;
        let mut c = PartCache::default();
        let x = atualizar(&a, Some(&copia), &mut c, Gatilho::Manual);
        println!("atualizar {s}: {} peças", x.pecas.len());
        soma += x.pecas.len();
    }
    let (v2, c2) = medidas(&copia.join("QUADRO.fkp"));
    let (v3, c3) = medidas(&copia.join("PR-01.fkp"));
    let dx = c2[0] - c0[0];
    r.registra(Linha { cod: "PLM-B".into(), nome: "Entre-eixos 1040 para 1060".into(), bancada: "PLM".into(), ok: (dx - 20.).abs() < 1. && (c3[1] - c1[1] - 20.).abs() < 1., volume: v2, caixa: c2, corpos: soma, nota: format!("QUADRO vol {v0:.0}→{v2:.0} caixa {c0:.1?}→{c2:.1?}; PR-01 vol {v1:.0}→{v3:.0} caixa {c1:.1?}→{c3:.1?}") });
    Ok(())
}

pub fn pacote(r: &mut Reg) {
    let dir = modelo();
    let a0 = Assembly::from_json(&std::fs::read_to_string(dir.join("KART.fka")).unwrap()).unwrap();
    let o = fenkra_pacote::Opcoes { decimar: Some(0.3), capturas: false, ..Default::default() };
    let t = std::time::Instant::now();
    println!("pacote do KART com esqueleto: {:?}", fenkra_pacote::de_montagem(&a0, &dir, &o).map(|_| "ok"));
    println!("[{:.1}s]", t.elapsed().as_secs_f64());
    let a = Assembly::from_json(&std::fs::read_to_string(dir.join("KART-PLANO.fka")).unwrap()).unwrap();
    let t = std::time::Instant::now();
    let n = match fenkra_pacote::de_montagem(&a, &dir, &o) {
        Ok(p) => {
            let v = p.validar();
            let b = p.bytes().unwrap_or_default();
            let g = p.gravar(&dir.join("KART.fkv"));
            let pagina = fenkra_pacote::pagina_html(Some(&b));
            let h = std::fs::write(dir.join("KART.html"), &pagina);
            format!("validar={v:?} gravar={g:?} html={} bytes {h:?} [{:.1}s]", pagina.len(), t.elapsed().as_secs_f64())
        }
        Err(e) => format!("falhou: {e}"),
    };
    r.usa("PLM", &["Pacote .fkv", "Página HTML"]);
    r.registra(Linha { cod: "FKV".into(), nome: "Pacote de visualização".into(), bancada: "PLM".into(), ok: !n.contains("falhou") && !n.contains("Err"), volume: 0., caixa: [0.; 3], corpos: 0, nota: n });
}
