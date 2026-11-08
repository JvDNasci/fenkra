// SPDX-License-Identifier: LGPL-2.1-or-later
//! B191: o PMI semântico atravessa o STEP AP242? Ida e volta pela ponte XDE do kernel, no OCCT 8.0.1.
//!
//! Cada teste fixa um fato conferido no arquivo gravado. Os que dizem "não atravessa" existem para o
//! dia em que o OCCT mudar: aí o teste cai e o mapa em `docs/modulos/pmi-3d.md` se atualiza.

use fenkra_kernel::*;

fn caixa() -> Shape {
    Shape::box_tool([0., 0., 0.], [1., 0., 0.], [0., 0., 1.], [20., 30., 40.]).unwrap()
}

fn kernel_ok() -> bool {
    Shape::box_tool([0., 0., 0.], [1., 0., 0.], [0., 0., 1.], [1., 1., 1.]).is_ok()
}

fn peca(anot: Vec<AnotacaoXde>) -> ModeloXde {
    let p = ProdutoXde { nome: "Bloco".into(), forma: Some(caixa()), ..Default::default() };
    ModeloXde { produtos: vec![p], raizes: vec![0], anotacoes: anot, ..Default::default() }
}

/// Caminho só deste uso: os testes rodam em paralelo na mesma pasta, e dois com o mesmo nome leriam o
/// arquivo um do outro (um AP214 "com PMI", um AP242 "sem cota").
fn caminho(nome: &str) -> std::path::PathBuf {
    static PROXIMO: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let dir = std::env::temp_dir().join(format!("fenkra_b191_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(format!("{}_{nome}", PROXIMO.fetch_add(1, std::sync::atomic::Ordering::Relaxed)))
}

fn gravar(m: &ModeloXde, nome: &str, esquema: EsquemaStep, pmi: bool) -> String {
    let c = caminho(nome);
    m.gravar(c.to_str().unwrap(), OpcoesGravarXde { esquema, pmi, ..Default::default() }).unwrap();
    c.to_str().unwrap().to_string()
}

fn ler(c: &str) -> ModeloXde {
    ModeloXde::ler(c, OpcoesLerXde::default()).unwrap()
}

fn datum(letra: &str, posicao: u32, face: u32, alvo: Option<AlvoXde>) -> AnotacaoXde {
    AnotacaoXde { apresentacao: None, produto: 0, nome: letra.into(), faces: vec![face], dado: DadoXde::Datum { letra: letra.into(), posicao, alvo } }
}

#[allow(clippy::too_many_arguments)]
fn tol(tipo: TipoToleranciaXde, v: f64, zona: ZonaXde, mat: MaterialXde, mz: ModZonaXde, mzv: f64, mods: Vec<ModToleranciaXde>, datums: Vec<usize>, face: u32) -> AnotacaoXde {
    AnotacaoXde { apresentacao: None, produto: 0, nome: "".into(), faces: vec![face], dado: DadoXde::Tolerancia { tipo, valor: v, zona, material: mat, mod_zona: mz, valor_zona: mzv, modificadores: mods, datums } }
}

fn dim(tipo: TipoDimensaoXde, v: f64, t: Option<(f64, f64)>, f1: u32, f2: Option<u32>) -> AnotacaoXde {
    AnotacaoXde { apresentacao: None, produto: 0, nome: "".into(), faces: vec![f1], dado: DadoXde::Dimensao { tipo, valor: v, tolerancia: t, faces2: f2.into_iter().collect() } }
}

fn tolerancias(m: &ModeloXde) -> Vec<&DadoXde> {
    m.anotacoes.iter().map(|a| &a.dado).filter(|d| matches!(d, DadoXde::Tolerancia { .. })).collect()
}

fn letras(m: &ModeloXde, refs: &[usize]) -> Vec<String> {
    refs.iter().map(|i| match &m.anotacoes[*i].dado { DadoXde::Datum { letra, .. } => letra.clone(), o => panic!("{o:?}") }).collect()
}

/// Nome da entidade de cada tipo de tolerância como o OCCT 8.0.1 grava. Confere o que `gdt.rs` diz.
#[test]
fn nomes_de_entidade_de_cada_tolerancia() {
    if !kernel_ok() {
        return;
    }
    let esperado = [
        (TipoToleranciaXde::Angularidade, "ANGULARITY_TOLERANCE"),
        (TipoToleranciaXde::BatimentoCircular, "CIRCULAR_RUNOUT_TOLERANCE"),
        (TipoToleranciaXde::Circularidade, "ROUNDNESS_TOLERANCE"),
        (TipoToleranciaXde::Coaxialidade, "COAXIALITY_TOLERANCE"),
        (TipoToleranciaXde::Concentricidade, "CONCENTRICITY_TOLERANCE"),
        (TipoToleranciaXde::Cilindricidade, "CYLINDRICITY_TOLERANCE"),
        (TipoToleranciaXde::Planeza, "FLATNESS_TOLERANCE"),
        (TipoToleranciaXde::Paralelismo, "PARALLELISM_TOLERANCE"),
        (TipoToleranciaXde::Perpendicularidade, "PERPENDICULARITY_TOLERANCE"),
        (TipoToleranciaXde::Posicao, "POSITION_TOLERANCE"),
        (TipoToleranciaXde::PerfilLinha, "LINE_PROFILE_TOLERANCE"),
        (TipoToleranciaXde::PerfilSuperficie, "SURFACE_PROFILE_TOLERANCE"),
        (TipoToleranciaXde::Retitude, "STRAIGHTNESS_TOLERANCE"),
        (TipoToleranciaXde::Simetria, "SYMMETRY_TOLERANCE"),
        (TipoToleranciaXde::BatimentoTotal, "TOTAL_RUNOUT_TOLERANCE"),
    ];
    for (tipo, entidade) in esperado {
        // todas com uma referência: algumas características não são escritas sem ela
        let a = tol(tipo, 0.1, ZonaXde::Largura, MaterialXde::Nenhum, ModZonaXde::Nenhum, 0., vec![], vec![0], 0);
        let c = gravar(&peca(vec![datum("A", 1, 1, None), a]), "t.step", EsquemaStep::Ap242, true);
        let txt = std::fs::read_to_string(&c).unwrap();
        assert!(txt.contains(entidade), "{tipo:?}: esperava {entidade}");
        let lido = ler(&c);
        let volta = tolerancias(&lido);
        assert!(matches!(volta.as_slice(), [DadoXde::Tolerancia { tipo: t, .. }] if *t == tipo), "{tipo:?} voltou {volta:?}");
    }
}

/// Dos 31 tipos de dimensão do XCAF, 26 fazem a ida e volta com o mesmo tipo e o mesmo valor. Os outros
/// cinco são recusados antes de chegar ao gravador (ver `TipoDimensaoXde::atravessa`).
#[test]
fn tipos_de_dimensao_que_atravessam_e_os_que_sao_recusados() {
    if !kernel_ok() {
        return;
    }
    let mut passaram = 0;
    for tipo in TipoDimensaoXde::TODOS.into_iter().filter(|t| *t != TipoDimensaoXde::Nenhum) {
        let valor = if tipo.e_angulo() { 30. } else { 12.5 };
        let m = peca(vec![dim(tipo, valor, None, 2, Some(3))]);
        let c = caminho("d.step");
        let r = m.gravar(c.to_str().unwrap(), OpcoesGravarXde { esquema: EsquemaStep::Ap242, pmi: true, ..Default::default() });
        if !tipo.atravessa() {
            assert!(r.is_err(), "{tipo:?} devia ser recusado");
            continue;
        }
        r.unwrap();
        let lido = ler(c.to_str().unwrap());
        assert_eq!(lido.anotacoes.len(), 1, "{tipo:?}");
        let DadoXde::Dimensao { tipo: t, valor: v, .. } = &lido.anotacoes[0].dado else { panic!("{tipo:?}: voltou {:?}", lido.anotacoes[0].dado) };
        assert_eq!(*t, tipo);
        assert!((v - valor).abs() < 1e-9, "{tipo:?}: {valor} voltou {v}");
        passaram += 1;
    }
    assert_eq!(passaram, 26);
}

#[test]
fn cota_com_tolerancia_faces_e_referencias_atravessam() {
    if !kernel_ok() {
        return;
    }
    let anot = vec![
        datum("A", 1, 0, None),
        datum("B", 2, 1, None),
        dim(TipoDimensaoXde::LocalDistanciaLinear, 20., Some((-0.05, 0.1)), 2, Some(3)),
        tol(TipoToleranciaXde::Paralelismo, 0.05, ZonaXde::Largura, MaterialXde::Nenhum, ModZonaXde::Nenhum, 0., vec![], vec![0, 1], 4),
    ];
    let c = gravar(&peca(anot), "a.step", EsquemaStep::Ap242, true);
    let txt = std::fs::read_to_string(&c).unwrap();
    for entidade in ["DATUM_FEATURE", "DATUM(", "SHAPE_ASPECT_RELATIONSHIP", "DIMENSIONAL_LOCATION", "PLUS_MINUS_TOLERANCE", "DATUM_SYSTEM", "GEOMETRIC_TOLERANCE_WITH_DATUM_REFERENCE", "PARALLELISM_TOLERANCE"] {
        assert!(txt.contains(entidade), "falta {entidade} no arquivo");
    }
    let lido = ler(&c);
    assert_eq!(lido.pmi, [1, 1, 2]);
    let d = lido.anotacoes.iter().find_map(|a| if let DadoXde::Dimensao { valor, tolerancia, faces2, .. } = &a.dado { Some((*valor, *tolerancia, a.faces.clone(), faces2.clone())) } else { None }).unwrap();
    assert_eq!((d.0, d.2, d.3), (20., vec![2], vec![3]));
    // o gravador escreve o módulo do desvio inferior: o sinal não atravessa
    assert_eq!(d.1, Some((0.05, 0.1)));
    let t = tolerancias(&lido);
    let [DadoXde::Tolerancia { valor, datums, .. }] = t.as_slice() else { panic!() };
    assert_eq!(*valor, 0.05);
    assert_eq!(letras(&lido, datums), vec!["A", "B"]);
    assert_eq!(lido.anotacoes.iter().find(|a| a.nome == "A").map(|a| a.faces.clone()), Some(vec![0]));
}

#[test]
fn datums_voltam_unicos_e_a_precedencia_e_do_datum_e_nao_do_quadro() {
    if !kernel_ok() {
        return;
    }
    let anot = vec![
        datum("A", 1, 0, None),
        datum("B", 2, 1, None),
        tol(TipoToleranciaXde::Perpendicularidade, 0.1, ZonaXde::Largura, MaterialXde::Nenhum, ModZonaXde::Nenhum, 0., vec![], vec![0, 1], 2),
        // o mesmo par em ordem trocada: o STEP guarda uma ordem por datum, então B|A não existe
        tol(TipoToleranciaXde::Paralelismo, 0.1, ZonaXde::Largura, MaterialXde::Nenhum, ModZonaXde::Nenhum, 0., vec![], vec![1, 0], 3),
    ];
    let lido = ler(&gravar(&peca(anot), "o.step", EsquemaStep::Ap242, true));
    let datums = lido.anotacoes.iter().filter(|a| matches!(a.dado, DadoXde::Datum { .. })).count();
    assert_eq!(datums, 2, "o leitor do OCCT cria um datum por uso; a ponte junta de volta");
    for t in tolerancias(&lido) {
        let DadoXde::Tolerancia { datums, .. } = t else { unreachable!() };
        assert_eq!(letras(&lido, datums), vec!["A", "B"], "a ordem trocada não atravessa");
    }
}

#[test]
fn zona_diametro_material_e_modificadores_atravessam_e_os_outros_nao() {
    if !kernel_ok() {
        return;
    }
    let anot = vec![
        datum("A", 1, 0, None),
        tol(TipoToleranciaXde::Posicao, 0.1, ZonaXde::Diametro, MaterialXde::Maximo, ModZonaXde::Nenhum, 0., vec![ModToleranciaXde::ZonaComum, ModToleranciaXde::EstadoLivre, ModToleranciaXde::Estatistica], vec![0], 3),
        tol(TipoToleranciaXde::Posicao, 0.1, ZonaXde::Esfera, MaterialXde::Minimo, ModZonaXde::Nenhum, 0., vec![ModToleranciaXde::PlanoTangente], vec![0], 4),
        // não atravessam: zona projetada (e a altura dela) e os modificadores de contorno
        tol(TipoToleranciaXde::Posicao, 0.1, ZonaXde::Largura, MaterialXde::Nenhum, ModZonaXde::Projetada, 12., vec![ModToleranciaXde::TodoContorno, ModToleranciaXde::TodaSuperficie], vec![0], 5),
    ];
    let c = gravar(&peca(anot), "z.step", EsquemaStep::Ap242, true);
    let txt = std::fs::read_to_string(&c).unwrap();
    assert!(!txt.contains("PROJECTED_ZONE_DEFINITION"), "se isto falhar, o OCCT passou a escrever a zona projetada");
    assert!(!txt.contains("ALL_AROUND") && !txt.contains("ALL_OVER"));
    let lido = ler(&c);
    let t = tolerancias(&lido);
    let campos = |d: &DadoXde| match d {
        DadoXde::Tolerancia { zona, material, mod_zona, valor_zona, modificadores, .. } => (*zona, *material, *mod_zona, *valor_zona, modificadores.clone()),
        _ => unreachable!(),
    };
    let achar = |z: ZonaXde, m: MaterialXde| t.iter().map(|d| campos(d)).find(|c| c.0 == z && c.1 == m);
    let a = achar(ZonaXde::Diametro, MaterialXde::Maximo).expect("zona diâmetro com máximo material");
    assert_eq!(a.4, vec![ModToleranciaXde::ZonaComum, ModToleranciaXde::EstadoLivre, ModToleranciaXde::Estatistica]);
    let b = achar(ZonaXde::Esfera, MaterialXde::Minimo).expect("zona esférica com mínimo material");
    assert_eq!(b.4, vec![ModToleranciaXde::PlanoTangente]);
    let p = achar(ZonaXde::Largura, MaterialXde::Nenhum).expect("a terceira");
    assert_eq!((p.2, p.3, p.4.len()), (ModZonaXde::Nenhum, 0., 0), "projetada, altura e contorno ficaram pelo caminho");
}

#[test]
fn alvos_de_datum_atravessam_com_tipo_numero_e_dimensoes() {
    if !kernel_ok() {
        return;
    }
    let a = |n, t, c, l| datum("A", 1, 0, Some(AlvoXde { tipo: t, numero: n, comprimento: c, largura: l }));
    let mut anot = vec![datum("A", 1, 0, None), a(1, AlvoDatumXde::Ponto, 0., 0.), a(2, AlvoDatumXde::Retangulo, 5., 3.), a(3, AlvoDatumXde::Circulo, 4., 0.)];
    // o arquivo sempre leva os alvos (ficam ligados ao datum), mas o leitor do OCCT só os devolve pelo
    // sistema de referências de uma tolerância: sem quadro que cite A, eles não voltam
    let sem_quadro = ler(&gravar(&peca(anot.clone()), "al0.step", EsquemaStep::Ap242, true));
    assert!(std::fs::read_to_string(gravar(&peca(anot.clone()), "al1.step", EsquemaStep::Ap242, true)).unwrap().contains("PLACED_DATUM_TARGET_FEATURE"));
    assert_eq!(sem_quadro.anotacoes.iter().filter(|x| matches!(x.dado, DadoXde::Datum { alvo: Some(_), .. })).count(), 0, "se isto falhar, o leitor passou a devolver alvos soltos");
    anot.push(tol(TipoToleranciaXde::Planeza, 0.1, ZonaXde::Largura, MaterialXde::Nenhum, ModZonaXde::Nenhum, 0., vec![], vec![0], 1));
    let lido = ler(&gravar(&peca(anot), "al.step", EsquemaStep::Ap242, true));
    let mut alvos: Vec<(u32, AlvoDatumXde, f64, f64)> = lido.anotacoes.iter().filter_map(|x| if let DadoXde::Datum { alvo: Some(a), .. } = &x.dado { Some((a.numero, a.tipo, a.comprimento, a.largura)) } else { None }).collect();
    alvos.sort_by_key(|a| a.0);
    assert_eq!(alvos.len(), 3, "um por alvo, sem cópias");
    assert_eq!(alvos[0].1, AlvoDatumXde::Ponto);
    assert_eq!((alvos[1].1, alvos[1].2, alvos[1].3), (AlvoDatumXde::Retangulo, 5., 3.));
    assert_eq!(alvos[2].1, AlvoDatumXde::Circulo);
    assert_eq!(lido.anotacoes.iter().filter(|x| matches!(x.dado, DadoXde::Datum { alvo: None, .. })).count(), 1);
}

#[test]
fn sem_ap242_ou_sem_a_chave_nao_sai_pmi() {
    if !kernel_ok() {
        return;
    }
    let anot = vec![datum("A", 1, 0, None), tol(TipoToleranciaXde::Planeza, 0.1, ZonaXde::Largura, MaterialXde::Nenhum, ModZonaXde::Nenhum, 0., vec![], vec![0], 1)];
    assert_eq!(ler(&gravar(&peca(anot.clone()), "c.step", EsquemaStep::Ap242, false)).pmi, [0, 0, 0]);
    assert_eq!(ler(&gravar(&peca(anot.clone()), "d.step", EsquemaStep::Ap214, true)).pmi, [0, 0, 0]);
    assert_eq!(ler(&gravar(&peca(anot), "e.step", EsquemaStep::Ap203, true)).pmi, [0, 0, 0]);
}

/// O esquema é de cada gravação: gravar AP214 e AP242 ao mesmo tempo não troca um pelo outro.
#[test]
fn gravacoes_em_paralelo_nao_trocam_o_esquema() {
    if !kernel_ok() {
        return;
    }
    let fios: Vec<_> = (0..4)
        .map(|f| {
            std::thread::spawn(move || {
                for i in 0..6 {
                    let anot = vec![datum("A", 1, 0, None), tol(TipoToleranciaXde::Planeza, 0.1, ZonaXde::Largura, MaterialXde::Nenhum, ModZonaXde::Nenhum, 0., vec![], vec![0], 1)];
                    let ap242 = (f + i) % 2 == 0;
                    let esquema = if ap242 { EsquemaStep::Ap242 } else { EsquemaStep::Ap214 };
                    let pmi = ler(&gravar(&peca(anot), "p.step", esquema, true)).pmi;
                    assert_eq!(pmi.iter().sum::<u32>() > 0, ap242, "fio {f}, rodada {i}: {pmi:?}");
                }
            })
        })
        .collect();
    for f in fios {
        f.join().unwrap();
    }
}

#[test]
fn anotacao_com_referencia_inexistente_ou_face_fora_da_peca_e_erro_e_nao_queda() {
    if !kernel_ok() {
        return;
    }
    let c = caminho("x.step");
    let o = OpcoesGravarXde { esquema: EsquemaStep::Ap242, pmi: true, ..Default::default() };
    let face_fora = peca(vec![dim(TipoDimensaoXde::TamanhoDiametro, 5., None, 99, None)]);
    assert!(face_fora.gravar(c.to_str().unwrap(), o).is_err());
    let sem_datum = peca(vec![tol(TipoToleranciaXde::Planeza, 0.1, ZonaXde::Largura, MaterialXde::Nenhum, ModZonaXde::Nenhum, 0., vec![], vec![3], 0)]);
    assert!(sem_datum.gravar(c.to_str().unwrap(), o).is_err());
}
