// SPDX-License-Identifier: LGPL-2.1-or-later
//! B03: dobra por linha em spline. A parede acompanha a curva com um raio contínuo (os raios
//! emendam, em vez de uma dobra única pela corda), a dobra entra no mapa e a linha de dobra curva
//! sai na planificada e no DXF, na camada da dobra.

mod common;

use common::*;
use fenkra_core::*;
use fenkra_solidos::planificada::{dxf, planificar, CAMADA_DOBRA_CIMA};
use fenkra_solidos::{recompute, tabela_dobras, Status};

fn s(x: &str) -> String {
    x.to_string()
}

fn sk_spline(d: &mut Document, pts: &[[f64; 2]]) -> Id {
    let mut data = SketchData::default();
    let mut n = 7_000_000u64 + d.features.len() as u64 * 1000;
    let mut al = || {
        n += 1;
        Id(n)
    };
    let ids: Vec<Id> = pts.iter().map(|p| data.add_point(&mut al, p[0], p[1])).collect();
    data.add_curve(&mut al, fenkra_core::sketch::Curve::Spline { pts: ids }, false);
    d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }))
}

fn chapa_com_spline() -> (Document, Id) {
    let mut d = Document::new("chapa");
    d.sheet = Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), flat: false, ..Default::default() });
    let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [100., 50.]);
    d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }));
    let sp = sk_spline(&mut d, &[[50., 0.], [65., 25.], [50., 50.]]);
    let id = d.add_feature(FeatureKind::SmBend(SmBend { a: [0.; 3], b: [0.; 3], angle: s("90"), radius: None, flip: false, down: false, sketch: Some(sp), position: 0 }));
    (d, id)
}

#[test]
fn spline_dobra_continua_e_vai_para_planificada_e_dxf() {
    if !kernel_disponivel() {
        return;
    }
    let (d, id) = chapa_com_spline();
    let r = recompute(&d);
    assert!(matches!(&r.status[&id], Status::Warning(m) if m.contains("curva")), "{:?}", r.status[&id]);
    // volume quase conservado (100 x 50 x 2): a parede segue a curva com largura constante, então o
    // lado de fora da curva estica um pouco, como na peça real (aba curva tem repuxo); a planificada
    // abaixo é que guarda a área exata do blank
    let vol = volume_total(&r);
    assert!(((vol - 10000.) / 10000.).abs() < 0.05, "volume {vol}");
    // a parede levantou: a caixa ganha altura e encolhe em x (a parte além da curva subiu)
    let (lo, hi) = caixa_da_malha(&r);
    assert!(hi[2] - lo[2] > 20., "a parede não subiu: caixa {lo:?} {hi:?}");
    assert!(hi[0] < 90., "a parte além da curva continuou plana: {hi:?}");
    // raios emendados: a linha de dobra acompanha a curva (muitos pontos), não a corda
    assert_eq!(r.bend_lines.len(), 1);
    let curva = &r.bend_lines[0].curva;
    assert!(curva.len() > 5, "linha de dobra pela corda: {curva:?}");
    let xmax = curva.iter().map(|q| q[0]).fold(f64::MIN, f64::max);
    assert!(xmax > 55., "a linha não segue a barriga da spline: {xmax}");
    // registro no mapa de dobras: entra na tabela
    assert_eq!(tabela_dobras(&d, &r.bend_lines).len(), 1);

    // planificada: válida, com a linha curva e o blank do tamanho da chapa
    let p = planificar(&d).expect("planificação");
    assert!(p.valida(), "{:?}", p.problemas());
    assert_eq!(p.propriedades.dobras, 1);
    assert!((p.propriedades.area - 5000.).abs() < 50., "área {}", p.propriedades.area);
    assert!(p.dobras[0].curva.len() > 5, "{:?}", p.dobras[0]);
    // DXF: a linha curva é uma polilinha só, na camada da dobra, e o rótulo fica sobre a curva
    let texto = dxf(&p);
    let lido = fenkra_dxf::ler(texto.as_bytes()).unwrap();
    let curvas: Vec<_> = lido.itens.iter().filter(|i| i.camada == CAMADA_DOBRA_CIMA).collect();
    assert_eq!(curvas.len(), 1, "a linha curva saiu em pedaços: {}", curvas.len());
    let fenkra_dxf::Entidade::Polilinha { vertices, fechada } = &curvas[0].entidade else { panic!("{:?}", curvas[0].entidade) };
    assert!(!fechada && vertices.len() == p.dobras[0].curva.len());
    let rotulo = lido.itens.iter().find_map(|i| if let fenkra_dxf::Entidade::Texto { pos, .. } = &i.entidade { Some(*pos) } else { None }).unwrap();
    let meio = p.dobras[0].curva[p.dobras[0].curva.len() / 2];
    assert!((rotulo[0] - meio[0]).hypot(rotulo[1] - meio[1]) < 3., "rótulo {rotulo:?} longe da curva {meio:?}");

    // nesting: a peça girada 90° e levada leva a linha curva junto (antes ficava no lugar)
    use fenkra_solidos::nesting::{peca_na_chapa, Colocacao, PecaNesting};
    let peca = PecaNesting::da_planificada("chapa", &p, 1).unwrap();
    let k = Colocacao { peca: 0, copia: 0, rot: 90., t: [200., 10.] };
    let m = peca_na_chapa(&peca, &k);
    let (q0, q1) = (peca.plana.dobras[0].curva[3], m.dobras[0].curva[3]);
    let esperado = [-q0[1] + 200., q0[0] + 10.];
    assert!((q1[0] - esperado[0]).hypot(q1[1] - esperado[1]) < 1e-6, "{q1:?} esperado {esperado:?}");
}
