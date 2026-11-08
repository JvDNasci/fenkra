// SPDX-License-Identifier: LGPL-2.1-or-later
//! B191: PMI gráfico (apresentação em polilinhas) e vistas salvas no STEP AP242, ida e volta pela
//! ponte XDE no OCCT 8.0.1.

use fenkra_kernel::*;

fn kernel_ok() -> bool {
    Shape::box_tool([0., 0., 0.], [1., 0., 0.], [0., 0., 1.], [1., 1., 1.]).is_ok()
}

fn caminho(nome: &str) -> String {
    static PROXIMO: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let dir = std::env::temp_dir().join(format!("fenkra_b191g_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(format!("{}_{nome}", PROXIMO.fetch_add(1, std::sync::atomic::Ordering::Relaxed))).to_str().unwrap().to_string()
}

fn desenho(nome: &str, x0: f64) -> ApresentacaoXde {
    // um "L" e uma linha de chamada inclinada: três segmentos com pontos não redondos
    ApresentacaoXde {
        nome: nome.into(),
        segmentos: vec![[[x0, 0., 40.], [x0 + 12.5, 0., 47.25]], [[x0 + 12.5, 0., 47.25], [x0 + 12.5, 0., 51.]], [[x0 + 12.5, 0., 47.25], [x0 + 20.75, 0., 47.25]]],
        plano: Some(([x0 + 12.5, 0., 47.25], [0., -1., 0.], [1., 0., 0.])),
        texto_em: Some([x0 + 12.5, 0., 47.25]),
    }
}

fn modelo() -> ModeloXde {
    let caixa = Shape::box_tool([0., 0., 0.], [1., 0., 0.], [0., 0., 1.], [20., 30., 40.]).unwrap();
    let p = ProdutoXde { nome: "Bloco".into(), forma: Some(caixa), ..Default::default() };
    let faces = |a: &[u32]| a.to_vec();
    let anotacoes = vec![
        AnotacaoXde { apresentacao: Some(desenho("A", -30.)), produto: 0, nome: "A".into(), faces: faces(&[4]), dado: DadoXde::Datum { letra: "A".into(), posicao: 1, alvo: None } },
        // um datum só volta do arquivo se um quadro o cita (leitor do OCCT)
        AnotacaoXde { apresentacao: Some(desenho("PERP 0,05 A", 90.)), produto: 0, nome: "P1".into(), faces: faces(&[0]), dado: DadoXde::Tolerancia { tipo: TipoToleranciaXde::Perpendicularidade, valor: 0.05, zona: ZonaXde::Largura, material: MaterialXde::Nenhum, mod_zona: ModZonaXde::Nenhum, valor_zona: 0., modificadores: vec![], datums: vec![0] } },
        AnotacaoXde { apresentacao: Some(desenho("20 +/-0,1", 0.)), produto: 0, nome: "L1".into(), faces: faces(&[0]), dado: DadoXde::Dimensao { tipo: TipoDimensaoXde::LocalDistanciaLinear, valor: 20., tolerancia: Some((-0.1, 0.1)), faces2: faces(&[1]) } },
        AnotacaoXde { apresentacao: Some(desenho("RUG Ra 1,6", 30.)), produto: 0, nome: "Rug".into(), faces: vec![], dado: DadoXde::Grafico },
        AnotacaoXde { apresentacao: Some(desenho("NOTA: QUEBRAR CANTOS", 60.)), produto: 0, nome: "Nota".into(), faces: vec![], dado: DadoXde::Grafico },
    ];
    let vistas = vec![
        VistaXde { nome: "Frente".into(), olho: [10., -300., 20.], direcao: [0., 1., 0.], cima: [0., 0., 1.], ortogonal: true, zoom: 300., largura: 330., altura: 185.6, anotacoes: vec![2, 3] },
        VistaXde { nome: "Iso".into(), olho: [200., -200., 200.], direcao: [-1., 1., -1.], cima: [-1., 1., 2.], ortogonal: false, zoom: 346.4, largura: 200., altura: 112.5, anotacoes: vec![4, 1] },
    ];
    ModeloXde { produtos: vec![p], raizes: vec![0], anotacoes, vistas, ..Default::default() }
}

fn perto(a: [f64; 3], b: [f64; 3], tol: f64) -> bool {
    (0..3).all(|i| (a[i] - b[i]).abs() < tol)
}

fn norm(a: [f64; 3]) -> [f64; 3] {
    let n = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
    [a[0] / n, a[1] / n, a[2] / n]
}

fn ida_e_volta(unidade: UnidadeStep) {
    let m = modelo();
    let c = caminho("grafico.step");
    m.gravar(&c, OpcoesGravarXde { esquema: EsquemaStep::Ap242, pmi: true, unidade, ..Default::default() }).unwrap();
    let texto = std::fs::read_to_string(&c).unwrap();
    assert!(texto.contains("TESSELLATED_ANNOTATION_OCCURRENCE"), "a apresentação sai em polilinhas");
    assert!(texto.contains("CAMERA_MODEL_D3"), "a vista sai como câmera");
    let l = ModeloXde::ler(&c, OpcoesLerXde::default()).unwrap();

    // as duas gráficas voltam como gráficas, com nome e desenho; a cota e o datum guardam o desenho
    let graficas: Vec<&AnotacaoXde> = l.anotacoes.iter().filter(|a| a.dado == DadoXde::Grafico).collect();
    assert_eq!(graficas.len(), 2, "{:?}", l.anotacoes.iter().map(|a| &a.dado).collect::<Vec<_>>());
    for orig in &m.anotacoes {
        let po = orig.apresentacao.as_ref().unwrap();
        let lida = l.anotacoes.iter().find(|a| a.apresentacao.as_ref().is_some_and(|p| p.nome == po.nome)).unwrap_or_else(|| panic!("sem a apresentação {}: {:?}", po.nome, l.anotacoes.iter().map(|a| (&a.dado, a.apresentacao.as_ref().map(|p| (p.nome.clone(), p.segmentos.len())))).collect::<Vec<_>>()));
        assert_eq!(std::mem::discriminant(&lida.dado), std::mem::discriminant(&orig.dado), "{}", po.nome);
        let pl = lida.apresentacao.as_ref().unwrap();
        assert_eq!(pl.segmentos.len(), po.segmentos.len(), "{}", po.nome);
        for s in &po.segmentos {
            // cada segmento gravado volta (em qualquer sentido), em mm, qualquer que seja a unidade do arquivo
            assert!(pl.segmentos.iter().any(|q| (perto(q[0], s[0], 1e-6) && perto(q[1], s[1], 1e-6)) || (perto(q[0], s[1], 1e-6) && perto(q[1], s[0], 1e-6))), "{}: {s:?} não voltou em {:?}", po.nome, pl.segmentos);
        }
        // rótulo comum: o gravador do OCCT 8.0.1 não escreve o plano (só o desenho), e o leitor não o inventa
        if orig.dado == DadoXde::Grafico {
            assert_eq!(pl.plano, None, "{}", po.nome);
            continue;
        }
        let (o, n, x) = pl.plano.expect("plano da anotação");
        let (o0, n0, x0) = po.plano.unwrap();
        assert!(perto(o, o0, 1e-6) && perto(n, n0, 1e-9) && perto(x, x0, 1e-9), "{}: plano {:?}", po.nome, pl.plano);
    }
    // a cota e o quadro continuam semânticos, com os valores em mm: o gravador do OCCT escreve valor,
    // desvios e zona crus na unidade do arquivo (em polegada voltavam 25,4 vezes maiores)
    let dados = l.anotacoes.iter().map(|a| &a.dado).collect::<Vec<_>>();
    assert!(dados.iter().any(|d| matches!(d, DadoXde::Dimensao { valor, tolerancia: Some((_, sup)), .. } if (valor - 20.).abs() < 1e-9 && (sup - 0.1).abs() < 1e-9)), "{dados:?}");
    assert!(dados.iter().any(|d| matches!(d, DadoXde::Tolerancia { valor, .. } if (valor - 0.05).abs() < 1e-9)), "{dados:?}");

    // vistas: câmera e as anotações de cada uma
    assert_eq!(l.vistas.len(), 2);
    for vo in &m.vistas {
        let v = l.vistas.iter().find(|v| v.nome == vo.nome).unwrap_or_else(|| panic!("sem a vista {}", vo.nome));
        assert_eq!(v.ortogonal, vo.ortogonal, "{}", vo.nome);
        assert!(perto(v.olho, vo.olho, 1e-6), "{}: olho {:?}", vo.nome, v.olho);
        assert!(perto(v.direcao, norm(vo.direcao), 1e-9), "{}: direção {:?}", vo.nome, v.direcao);
        // o "para cima" volta perpendicular à visada, no mesmo semiplano
        let d = norm(vo.direcao);
        let c = vo.cima;
        let k = c[0] * d[0] + c[1] * d[1] + c[2] * d[2];
        let cima = norm([c[0] - k * d[0], c[1] - k * d[1], c[2] - k * d[2]]);
        assert!(perto(v.cima, cima, 1e-9), "{}: cima {:?} esperado {cima:?}", vo.nome, v.cima);
        assert!((v.zoom - vo.zoom).abs() < 1e-6 && (v.largura - vo.largura).abs() < 1e-6 && (v.altura - vo.altura).abs() < 1e-6, "{}: janela", vo.nome);
        let mut nomes: Vec<String> = v.anotacoes.iter().map(|i| l.anotacoes[*i].apresentacao.as_ref().unwrap().nome.clone()).collect();
        let mut esperado: Vec<String> = vo.anotacoes.iter().map(|i| m.anotacoes[*i].apresentacao.as_ref().unwrap().nome.clone()).collect();
        nomes.sort();
        esperado.sort();
        assert_eq!(nomes, esperado, "{}", vo.nome);
    }
}

#[test]
fn pmi_grafico_e_vistas_atravessam_em_milimetro() {
    if !kernel_ok() {
        return;
    }
    ida_e_volta(UnidadeStep::Milimetro);
}

/// O gravador do OCCT converte o plano e a geometria para a unidade do arquivo, mas copia os pontos da
/// apresentação crus, e o leitor os multiplica pela unidade: sem a correção da ponte, em polegada o
/// desenho voltava 25,4 vezes maior.
#[test]
fn pmi_grafico_e_vistas_atravessam_em_polegada_e_metro() {
    if !kernel_ok() {
        return;
    }
    ida_e_volta(UnidadeStep::Polegada);
    ida_e_volta(UnidadeStep::Metro);
}

#[test]
fn grafico_sem_desenho_e_vista_sem_para_cima_sao_recusados() {
    if !kernel_ok() {
        return;
    }
    let mut m = modelo();
    m.anotacoes[3].apresentacao = None;
    assert!(m.gravar(&caminho("sem.step"), OpcoesGravarXde { esquema: EsquemaStep::Ap242, pmi: true, ..Default::default() }).is_err());
    let mut m = modelo();
    m.vistas[0].cima = m.vistas[0].direcao;
    assert!(m.gravar(&caminho("cima.step"), OpcoesGravarXde { esquema: EsquemaStep::Ap242, pmi: true, ..Default::default() }).is_err());
}

/// Só rótulos comuns (modo gráfico): o gravador deixava o modelo de apresentação sem contexto, porque o
/// tira do primeiro elemento semântico, e a gravação falhava.
#[test]
fn so_desenho_sem_nenhuma_semantica_grava_e_volta() {
    if !kernel_ok() {
        return;
    }
    let mut m = modelo();
    m.anotacoes.retain(|a| a.dado == DadoXde::Grafico);
    m.vistas.truncate(1);
    m.vistas[0].anotacoes = vec![0, 1];
    let c = caminho("so_desenho.step");
    m.gravar(&c, OpcoesGravarXde { esquema: EsquemaStep::Ap242, pmi: true, ..Default::default() }).unwrap();
    let l = ModeloXde::ler(&c, OpcoesLerXde::default()).unwrap();
    assert_eq!(l.anotacoes.len(), 2);
    assert!(l.anotacoes.iter().all(|a| a.dado == DadoXde::Grafico));
    assert_eq!(l.vistas.len(), 1);
    assert_eq!(l.vistas[0].anotacoes.len(), 2);
}
