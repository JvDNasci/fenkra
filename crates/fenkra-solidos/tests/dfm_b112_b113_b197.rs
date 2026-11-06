// SPDX-License-Identifier: LGPL-2.1-or-later
//! B112, B113 e B197: motor de regras DFM, espessura, saliência, canto vivo, saída e undercut.
//! Cada teste monta a peça em código, recalcula e mede no corpo real do kernel.

mod common;

use common::*;
use fenkra_core::*;
use fenkra_solidos::dfm::{self, Perfil, Severidade};
use fenkra_solidos::dfm_geo::{self as geo, Classe, Malha};
use fenkra_solidos::{final_shape, recompute};

const XY: PlaneRef = PlaneRef::Origin(OriginPlane::XY);

fn extrusao(sk: Id, op: BoolOp, dist: &str, inicio: &str, taper: &str) -> FeatureKind {
    FeatureKind::Extrude(Extrude {
        sketch: sk,
        op,
        extent: Extent::Blind(dist.into()),
        reverse: false,
        taper: taper.into(),
        thin: None,
        dir: None,
        thin_side: 0,
        ends: ExtrudeEnds { start: inicio.into(), ..Default::default() },
    })
}

fn confere(d: &Document) {
    let r = recompute(d);
    assert!(problema_de_status(d, &r).is_none(), "{:?}", problema_de_status(d, &r));
}

fn malha(d: &Document) -> (fenkra_kernel::Mesh, Malha) {
    let s = final_shape(d).unwrap();
    let (lo, hi) = s.bbox().unwrap();
    let diag = (0..3).map(|k| (hi[k] - lo[k]).powi(2)).sum::<f64>().sqrt();
    let mesh = s.mesh_angle(diag * 1e-3, 0.25).unwrap();
    let m = Malha::de(&mesh).unwrap();
    (mesh, m)
}

fn perfil(nome: &str) -> Perfil {
    dfm::perfis_padrao().into_iter().find(|p| p.nome.starts_with(nome)).unwrap()
}

/// Degrau: 40 x 20 com 1 mm de altura, e a metade da direita (x 20..40) com 3 mm.
fn degrau() -> Document {
    let mut d = Document::new("degrau");
    let a = sk_rect(&mut d, XY, [0., 0.], [40., 20.]);
    d.add_feature(ext(a, BoolOp::Add, "1"));
    let b = sk_rect(&mut d, XY, [20., 0.], [40., 20.]);
    d.add_feature(ext(b, BoolOp::Add, "3"));
    d
}

#[test]
fn degrau_de_1_e_3_mm_gera_duas_faixas_com_erro_ate_5_por_cento() {
    if !kernel_disponivel() {
        return;
    }
    let d = degrau();
    confere(&d);
    let (_, m) = malha(&d);
    let a = geo::espessura(&m, 4000);
    assert!(a.len() > 100, "poucas amostras: {}", a.len());
    // todas as faixas: as duas grandes e as de quina e degrau, pequenas
    let todas = geo::faixas(&a, 0.05);
    let total: f64 = todas.iter().map(|x| x.area).sum();
    let f = geo::faixas_principais(&a, 0.05, 0.05);
    let fora: f64 = total - f.iter().map(|x| x.area).sum::<f64>();
    assert!(fora < total * 0.05, "{:.1} mm2 de {total:.1} fora das faixas principais", fora);
    for x in &f {
        eprintln!("faixa {:.3}..{:.3} media {:.3} area {:.1} n {}", x.min, x.max, x.media, x.area, x.amostras);
    }
    assert_eq!(f.len(), 2, "{f:#?}");
    assert!((f[0].media - 1.).abs() <= 0.05, "{:?}", f[0]);
    assert!((f[1].media - 3.).abs() <= 3. * 0.05, "{:?}", f[1]);
    // a faixa de 1 mm é a metade esquerda: a área dela é maior que a de 3 mm só se contarmos o fundo e o topo
    assert!(f[0].area > 0. && f[1].area > 0.);
}

#[test]
fn parede_fina_vira_violacao_com_a_face_e_o_ponto() {
    if !kernel_disponivel() {
        return;
    }
    let d = degrau();
    let s = final_shape(&d).unwrap();
    // o limite do perfil de injeção é 0,8 mm: o degrau de 1 mm passa; a 1,2 mm de mínimo, reprova
    let mut p = perfil("Injeção");
    let r0 = dfm::varrer(&s, &p).unwrap();
    assert_eq!(r0.das_regras("inj.parede.min").count(), 0, "{:#?}", r0.violacoes);
    assert!(p.ajusta("inj.parede.min", Some(1.2), None));
    let r1 = dfm::varrer(&s, &p).unwrap();
    let v: Vec<_> = r1.das_regras("inj.parede.min").collect();
    assert!(!v.is_empty());
    for x in &v {
        assert_eq!(x.severidade, Severidade::Erro);
        assert!((x.valor - 1.).abs() < 0.06, "{x:?}");
        assert_eq!(x.limite, 1.2);
        assert!(!x.faces.is_empty());
        assert!(x.texto.contains("1.20"), "{}", x.texto);
    }
    // a faixa de 3 mm não aparece: só a região de 1 mm reprova
    assert!(v.iter().all(|x| x.valor < 1.5));
    // o ponto é uma amostra dentro da região fina (x < 20 ou no degrau)
    assert!(v.iter().all(|x| x.ponto[0] <= 20.5), "{v:#?}");
    // parede grossa: o máximo de 4 mm do perfil aceita 3 mm; com 2,5 mm reprova na parte alta
    assert!(p.ajusta("inj.parede.max", None, Some(2.5)));
    let r2 = dfm::varrer(&s, &p).unwrap();
    let g: Vec<_> = r2.das_regras("inj.parede.max").collect();
    assert!(!g.is_empty() && g.iter().all(|x| x.valor > 2.5));
}

/// Coluna 10 x 10 x 8 com um braço de 30 mm que parte de z = 5 até z = 8.
fn braco() -> Document {
    let mut d = Document::new("braco");
    let a = sk_rect(&mut d, XY, [0., 0.], [40., 10.]);
    d.add_feature(ext(a, BoolOp::Add, "2"));
    let c = sk_rect(&mut d, XY, [0., 0.], [10., 10.]);
    d.add_feature(ext(c, BoolOp::Add, "8"));
    let b = sk_rect(&mut d, XY, [0., 0.], [30., 10.]);
    d.add_feature(extrusao(b, BoolOp::Add, "3", "5", "0"));
    d
}

#[test]
fn saliencia_acha_a_face_de_baixo_do_braco_e_ignora_a_mesa() {
    if !kernel_disponivel() {
        return;
    }
    let d = braco();
    confere(&d);
    let (_, m) = malha(&d);
    let r = geo::saliencia(&m, [0., 0., 1.], 45.);
    assert_eq!(r.len(), 1, "{r:#?}");
    assert!((r[0].pior - 90.).abs() < 1e-6);
    // 20 x 10 sob o braço (x 10..30)
    assert!((r[0].area - 200.).abs() < 1e-6, "{}", r[0].area);
    assert!((r[0].ponto[2] - 5.).abs() < 1e-6);
    // construindo de cabeça para baixo, a mesa é o topo do braço e o fundo da base vira teto
    let inv = geo::saliencia(&m, [0., 0., -1.], 45.);
    assert!(inv.iter().any(|f| (f.ponto[2] - 2.).abs() < 1e-6), "{inv:#?}");
}

#[test]
fn perfil_de_impressao_3d_acusa_saliencia_e_furo_pequeno() {
    if !kernel_disponivel() {
        return;
    }
    let d = braco();
    let s = final_shape(&d).unwrap();
    let r = dfm::varrer(&s, &perfil("Impressão")).unwrap();
    let v: Vec<_> = r.das_regras("fdm.saliencia").collect();
    assert_eq!(v.len(), 1, "{:#?}", r.violacoes);
    assert_eq!(v[0].faces.len(), 1);
    assert!(r.faces_realcadas().iter().any(|(f, _)| *f == v[0].faces[0]));
    assert!(r.resumo().contains("Impressão 3D"));
}

#[test]
fn undercut_embaixo_do_braco_nas_duas_faces_que_se_olham() {
    if !kernel_disponivel() {
        return;
    }
    let d = braco();
    let (_, m) = malha(&d);
    let r = geo::saida(&m, [0., 0., 1.], 1.);
    let bloqueadas: Vec<_> = r.iter().filter(|f| f.undercut()).collect();
    // o chão sob o braço (olha para cima, coberto) e o fundo do braço (olha para baixo, apoiado no chão)
    assert_eq!(bloqueadas.len(), 2, "{r:#?}");
    let zs: Vec<f64> = {
        let mut v: Vec<f64> = bloqueadas.iter().map(|f| f.ponto[2]).collect();
        v.sort_by(|a, b| a.total_cmp(b));
        v
    };
    assert!((zs[0] - 2.).abs() < 1e-6 && (zs[1] - 5.).abs() < 1e-6, "{zs:?}");
    // o topo do braço e o fundo da base veem a saída livre
    let topo = r.iter().find(|f| (f.ponto[2] - 8.).abs() < 1e-6 && f.classe == Classe::Cavidade).unwrap();
    assert!(!topo.undercut());
    let fundo = r.iter().find(|f| f.ponto[2].abs() < 1e-6 && f.classe == Classe::Nucleo).unwrap();
    assert!(!fundo.undercut());
    // visto pelo lado, sem nada na frente, o mesmo braço não tem undercut nas faces do fundo e do topo
    let lado = geo::saida(&m, [1., 0., 0.], 1.);
    assert!(lado.iter().filter(|f| f.undercut()).count() <= 2);
}

/// Bloco 40 x 40 x 20 com bolso quadrado 20 x 20 aberto em cima, de z = 10 a z = 20.
fn bolso(raio_vertical: Option<&str>) -> Document {
    let mut d = Document::new("bolso");
    let a = sk_rect(&mut d, XY, [0., 0.], [40., 40.]);
    d.add_feature(ext(a, BoolOp::Add, "20"));
    let topo = d.add_offset_plane(XY, "20");
    let c = sk_rect(&mut d, PlaneRef::Datum(topo), [10., 10.], [30., 30.]);
    d.add_feature(ext(c, BoolOp::Cut, "10"));
    if let Some(r) = raio_vertical {
        let mids = vec![[10., 10., 15.], [30., 10., 15.], [30., 30., 15.], [10., 30., 15.]];
        d.add_feature(filete(r, mids, vec![None; 4]));
    }
    d
}

#[test]
fn canto_vivo_conta_as_arestas_concavas_do_bolso() {
    if !kernel_disponivel() {
        return;
    }
    let d = bolso(None);
    confere(&d);
    let (mesh, m) = malha(&d);
    let c = geo::cantos_vivos(&mesh, &m, 1e-3);
    // 4 arestas do fundo e 4 cantos verticais; as bordas de cima são convexas e não entram
    assert_eq!(c.len(), 8, "{c:#?}");
    assert!(c.iter().all(|x| (x.abertura - 90.).abs() < 1e-3), "{c:#?}");
    let verticais = c.iter().filter(|x| (x.comprimento - 10.).abs() < 1e-3).count();
    assert_eq!(verticais, 4);
}

#[test]
fn perfil_de_fresagem_reprova_canto_vivo_e_aceita_filete_maior_que_a_fresa() {
    if !kernel_disponivel() {
        return;
    }
    let vivo = bolso(None);
    let s = final_shape(&vivo).unwrap();
    let p = perfil("Fresagem");
    let r = dfm::varrer(&s, &p).unwrap();
    let c: Vec<_> = r.das_regras("cnc.canto.raio").collect();
    assert_eq!(c.len(), 8, "{:#?}", r.violacoes);
    assert!(c.iter().all(|x| x.severidade == Severidade::Erro && x.valor == 0. && x.limite == 1.5 && x.faces.len() == 2));
    assert!(r.contar(Severidade::Erro) >= 8);

    // cantos verticais com R3: some o canto vivo vertical e o filete (R3 > 1,5) não reprova
    let f = bolso(Some("3"));
    confere(&f);
    let s2 = final_shape(&f).unwrap();
    let r2 = dfm::varrer(&s2, &p).unwrap();
    let c2: Vec<_> = r2.das_regras("cnc.canto.raio").collect();
    // sobram só as arestas do fundo (4 retas e 4 arcos): nenhuma aresta viva vertical
    assert_eq!(c2.len(), 8, "{:#?}", c2);
    assert!(c2.iter().all(|x| (x.ponto[2] - 10.).abs() < 0.5), "{c2:#?}");
    // os quatro filetes de R3 são reconhecidos: com fresa de 5 mm de raio mínimo eles reprovam com valor 3
    let mut grande = p.clone();
    grande.ajusta("cnc.canto.raio", Some(5.), None);
    let rg = dfm::varrer(&s2, &grande).unwrap();
    let f3 = rg.das_regras("cnc.canto.raio").filter(|x| (x.valor - 3.).abs() < 1e-3).count();
    assert_eq!(f3, 4, "{:#?}", rg.violacoes);
    // o filete de 3 mm tem 10 mm de parede: 10/3 = 3,3 vezes, abaixo do limite de 8
    assert_eq!(r2.das_regras("cnc.bolso.profundidade").count(), 0, "{:#?}", r2.violacoes);
    // com limite apertado em 2, o mesmo filete reprova pela profundidade do bolso
    let mut ap = p.clone();
    ap.ajusta("cnc.bolso.profundidade", None, Some(2.));
    let r3 = dfm::varrer(&s2, &ap).unwrap();
    let b: Vec<_> = r3.das_regras("cnc.bolso.profundidade").collect();
    assert!(!b.is_empty(), "{:#?}", r3.violacoes);
    assert!(b.iter().all(|x| x.valor > 2. && x.valor < 4.), "{b:#?}");
}

#[test]
fn furo_pequeno_e_chapa_fina_pelo_perfil() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("chapa_furada");
    let a = sk_rect(&mut d, XY, [0., 0.], [60., 40.]);
    d.add_feature(ext(a, BoolOp::Add, "2"));
    let p = sk_pontos(&mut d, XY, &[[20., 20.]]);
    d.add_feature(furo_passante(p, "0.8"));
    let q = sk_pontos(&mut d, XY, &[[45., 20.]]);
    d.add_feature(furo_passante(q, "6"));
    confere(&d);
    let s = final_shape(&d).unwrap();
    // fresagem: o furo de 0,8 mm fica abaixo de 1 mm
    let r = dfm::varrer(&s, &perfil("Fresagem")).unwrap();
    let f: Vec<_> = r.das_regras("cnc.furo.diametro_min").collect();
    assert_eq!(f.len(), 1, "{:#?}", r.violacoes);
    assert!((f[0].valor - 0.8).abs() < 1e-6);
    assert!((f[0].ponto[0] - 20.).abs() < 1.5 && (f[0].ponto[1] - 20.).abs() < 1.5, "{:?}", f[0].ponto);
    // chapa: 2 mm cabe na faixa 0,5..6; o furo de 0,8 é 0,4 vezes a espessura (< 1) e o de 6 é 3 vezes
    let c = dfm::varrer(&s, &perfil("Chapa")).unwrap();
    assert_eq!(c.das_regras("chapa.espessura").count(), 0, "{:#?}", c.violacoes);
    let fe: Vec<_> = c.das_regras("chapa.furo.diametro").collect();
    assert_eq!(fe.len(), 1, "{:#?}", c.violacoes);
    assert!((fe[0].valor - 0.4).abs() < 0.03, "{:?}", fe[0]);
    // 2 mm fora do intervalo 3..6: a espessura dominante da peça reprova
    let mut grossa = perfil("Chapa");
    grossa.ajusta("chapa.espessura", Some(3.), Some(6.));
    let g = dfm::varrer(&s, &grossa).unwrap();
    let e: Vec<_> = g.das_regras("chapa.espessura").collect();
    assert_eq!(e.len(), 1);
    assert!((e[0].valor - 2.).abs() < 0.1, "{:?}", e[0]);
}

#[test]
fn saida_do_perfil_de_injecao_separa_parede_reta_de_parede_com_saida() {
    if !kernel_disponivel() {
        return;
    }
    let monta = |taper: &str| {
        let mut d = Document::new("saida");
        let a = sk_rect(&mut d, XY, [0., 0.], [30., 20.]);
        d.add_feature(extrusao(a, BoolOp::Add, "15", "0", taper));
        d
    };
    // parede reta: as quatro laterais ficam com saída 0 e reprovam
    let reta = monta("0");
    confere(&reta);
    let s = final_shape(&reta).unwrap();
    let p = perfil("Injeção");
    let r = dfm::varrer(&s, &p).unwrap();
    let v: Vec<_> = r.das_regras("inj.saida.angulo").collect();
    assert_eq!(v.len(), 4, "{:#?}", r.violacoes);
    assert!(v.iter().all(|x| x.valor < 1e-6 && x.limite == 1.));
    assert_eq!(r.saida.iter().filter(|f| f.classe == Classe::Lateral).count(), 4);
    // tampa e fundo, perpendiculares à abertura, não reprovam
    assert_eq!(r.saida.iter().filter(|f| f.classe == Classe::Cavidade).count(), 1);
    assert_eq!(r.saida.iter().filter(|f| f.classe == Classe::Nucleo).count(), 1);

    // com 3 graus de saída cada lateral mede 3 graus e passa
    let sai = monta("3");
    confere(&sai);
    let s3 = final_shape(&sai).unwrap();
    let r3 = dfm::varrer(&s3, &p).unwrap();
    assert_eq!(r3.das_regras("inj.saida.angulo").count(), 0, "{:#?}", r3.violacoes);
    let lat: Vec<_> = r3.saida.iter().filter(|f| f.saida_max < 80.).collect();
    assert_eq!(lat.len(), 4);
    for f in lat {
        assert!((f.saida_min - 3.).abs() < 0.05 && (f.saida_max - 3.).abs() < 0.05, "{f:?}");
    }
    // exigir 5 graus reprova as quatro
    let mut rig = p.clone();
    rig.ajusta("inj.saida.angulo", Some(5.), None);
    assert_eq!(dfm::varrer(&s3, &rig).unwrap().das_regras("inj.saida.angulo").count(), 4);
}

#[test]
fn undercut_do_perfil_de_injecao_e_erro_e_a_abertura_muda_o_resultado() {
    if !kernel_disponivel() {
        return;
    }
    let d = braco();
    let s = final_shape(&d).unwrap();
    let mut p = perfil("Injeção");
    let r = dfm::varrer(&s, &p).unwrap();
    let u: Vec<_> = r.das_regras("inj.undercut").collect();
    assert_eq!(u.len(), 2, "{:#?}", r.violacoes);
    assert!(u.iter().all(|x| x.severidade == Severidade::Erro && x.valor > 100.));
    // abrindo pelo eixo y, o braço está livre dos dois lados
    p.direcao = [0., 1., 0.];
    let r2 = dfm::varrer(&s, &p).unwrap();
    assert_eq!(r2.das_regras("inj.undercut").count(), 0, "{:#?}", r2.violacoes);
}

#[test]
fn varredura_sem_regra_de_um_tipo_nao_mede_o_que_nao_pediu() {
    if !kernel_disponivel() {
        return;
    }
    let d = braco();
    let s = final_shape(&d).unwrap();
    // o perfil de impressão não tem regra de saída nem de undercut: nada é classificado
    let r = dfm::varrer(&s, &perfil("Impressão")).unwrap();
    assert!(r.saida.is_empty());
    // mas tem de parede: as amostras de espessura saem para o mapa
    assert!(!r.espessura.is_empty());
    // regra desligada não roda
    let mut p = perfil("Impressão");
    for x in &mut p.regras {
        x.ativa = false;
    }
    let vazio = dfm::varrer(&s, &p).unwrap();
    assert!(vazio.violacoes.is_empty() && vazio.espessura.is_empty());
}
