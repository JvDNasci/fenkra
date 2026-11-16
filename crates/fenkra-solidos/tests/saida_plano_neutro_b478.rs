// SPDX-License-Identifier: LGPL-2.1-or-later
//! B478: Saída na Placa de exemplo (placa 120 x 72 x 10 com filete de 1 mm em todas as arestas).
//! A Saída na lateral falhava sempre, com qualquer plano neutro: a face é tangente aos filetes das
//! bordas, e o OpenCASCADE não inclina uma face cuja vizinha tangente (um filete de eixo horizontal)
//! não pode ser inclinada. O aviso agora diz isso; com a Saída antes do filete ela funciona.

mod common;

use common::*;
use fenkra_core::*;
use fenkra_solidos::{example_document, recompute, Status};

fn saida(d: &mut Document, neutral: PlaneRef) -> Id {
    d.add_feature(FeatureKind::Draft(Draft { faces: vec![[0., -36., 5.]], neutral, angle: "3".into(), reverse: false, face_keys: vec![], regras: vec![] }))
}

fn topo(d: &mut Document) -> PlaneRef {
    PlaneRef::Datum(d.add_face_plane([0., 0., 10.], [1., 0., 0.], [0., 0., 1.], None))
}

#[test]
fn lateral_tangente_a_filete_avisa_a_causa_com_qualquer_plano_neutro() {
    for nome in ["XY", "YZ", "topo"] {
        let mut d = example_document();
        let n = match nome {
            "XY" => PlaneRef::Origin(OriginPlane::XY),
            "YZ" => PlaneRef::Origin(OriginPlane::YZ),
            _ => topo(&mut d),
        };
        let id = saida(&mut d, n);
        let r = recompute(&d);
        assert!(matches!(&r.status[&id], Status::Error(m) if m.contains("tangente a um filete")), "{nome}: {:?}", r.status[&id]);
    }
}

/// Sem o filete (Saída antes dele) a mesma face aceita 3 graus: o volume muda e a caixa da peça fica
/// a mesma (10 mm) e o pé da frente avança tan(3) x 10 mm para fora: com o neutro no topo, a face se abre
/// para a base e ganha material.
#[test]
fn lateral_sem_filete_aceita_saida_com_neutro_no_topo() {
    let mut base = example_document();
    base.features.pop();
    let r0 = recompute(&base);
    let v0 = r0.bodies[0].mass.volume;
    let n = topo(&mut base);
    let id = saida(&mut base, n);
    let r = recompute(&base);
    assert!(matches!(r.status[&id], Status::Ok), "{:?}", r.status[&id]);
    let v = r.bodies[0].mass.volume;
    // a face de 120 x 10 cai 3 graus: o pé da frente avança tan(3) x 10 mm, em 120 mm de comprimento (cunha: metade da base x altura)
    let esperado = 120. * 10. * 10. * 0.5 * 3f64.to_radians().tan();
    assert!((v - v0 - esperado).abs() < esperado * 0.02, "volume {v}, antes {v0}, ganha ~{esperado}");
    let (lo, hi) = caixa_da_malha(&r);
    assert!((hi[2] - lo[2] - 10.).abs() < 0.01 && (hi[0] - lo[0] - 120.).abs() < 0.01, "{lo:?} {hi:?}");
    // o pé da frente avança para y = -36 - tan(3) x 10
    let avanco = 10. * 3f64.to_radians().tan();
    assert!((hi[1] - lo[1] - (72. + avanco)).abs() < 0.01, "{lo:?} {hi:?}");
}
