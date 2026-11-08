// SPDX-License-Identifier: LGPL-2.1-or-later
//! B426: a conversão da forma livre fica guardada. O segundo cálculo da mesma gaiola vem da memória, o
//! cálculo depois de esquecer a memória vem do disco, e os três dão o mesmo corpo: status, volume,
//! caixa e nomes de face iguais.

mod common;

use common::*;
use fenkra_core::subd::Gaiola;
use fenkra_core::*;
use fenkra_solidos::forma_brep_cache::{self as cache, Origem};
use fenkra_solidos::{recompute, Status};

fn doc_com(g: Gaiola) -> (Document, Id) {
    let mut d = Document::new("forma");
    let id = d.add_feature(FeatureKind::Forma(Forma { gaiola: g, niveis: 3, congelada: false, conversao: Some(ConversaoForma { continuidade: 1, tol: "0.01".into(), pausada: false }), vinculos: Default::default() }));
    (d, id)
}

#[test]
fn segunda_conversao_vem_da_memoria_e_do_disco_com_o_mesmo_corpo() {
    if !kernel_disponivel() {
        return;
    }
    let pasta = std::env::temp_dir().join(format!("fenkra-cache-forma-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&pasta);
    // SAFETY: único teste deste binário; nenhuma outra thread lê o ambiente agora
    unsafe { std::env::set_var("FENKRA_CACHE_FORMA", &pasta) };
    let (d, id) = doc_com(Gaiola::caixa([0.; 3], [90., 70., 50.], [2, 2, 2]));

    let mede = |o: Origem| {
        let t = std::time::Instant::now();
        let r = recompute(&d);
        let ms = t.elapsed().as_secs_f64() * 1000.;
        assert_eq!(cache::ultima_origem(), Some(o));
        assert!(matches!(r.status.get(&id), Some(Status::Ok | Status::Warning(_))), "{:?}", r.status.get(&id));
        assert_eq!(r.bodies.len(), 1);
        let b = &r.bodies[0];
        // nome de cada face junto com o centro dela: pega troca de ordem das faces na leitura
        let m = &b.mesh;
        let mut nomes: Vec<(i32, [i64; 3])> = (0..m.face_ranges.len())
            .map(|i| {
                let ini = m.face_ranges[i] as usize;
                let fim = m.face_ranges.get(i + 1).map_or(m.indices.len(), |x| *x as usize);
                let mut c = [0f64; 3];
                for &v in &m.indices[ini..fim] {
                    for k in 0..3 {
                        c[k] += m.positions[v as usize * 3 + k] as f64;
                    }
                }
                let n = (fim - ini).max(1) as f64;
                (b.face_role[i], c.map(|x| (x / n * 10.).round() as i64))
            })
            .collect();
        nomes.sort_unstable();
        (b.mass.volume, caixa_da_malha(&r), nomes, b.faces, ms)
    };
    let (v0, c0, n0, f0, t0) = mede(Origem::Convertida);
    assert!(std::fs::read_dir(&pasta).unwrap().count() >= 1, "nada no disco");
    let (v1, c1, n1, f1, t1) = mede(Origem::Memoria);
    cache::esquecer_memoria();
    let (v2, c2, n2, f2, _) = mede(Origem::Disco);
    for (v, c, n, f) in [(v1, c1, &n1, f1), (v2, c2, &n2, f2)] {
        assert!((v - v0).abs() < 1e-6 * v0, "volume {v} contra {v0}");
        assert_eq!(f, f0);
        assert_eq!(n, &n0, "os nomes das faces mudaram");
        for k in 0..3 {
            assert!((c.0[k] - c0.0[k]).abs() < 1e-6 && (c.1[k] - c0.1[k]).abs() < 1e-6, "caixa {c:?} contra {c0:?}");
        }
    }
    // a caixa é a da gaiola suavizada, centrada na origem: não passa da gaiola de 90 × 70 × 50
    for (k, l) in [90., 70., 50.].into_iter().enumerate() {
        let t = c0.1[k] - c0.0[k];
        assert!(t <= l + 1e-3 && t > l / 2. && (c0.1[k] + c0.0[k]).abs() < 1e-3, "{c0:?}");
    }
    eprintln!("convertida {t0:.0} ms, da memória {t1:.0} ms");
    assert!(t1 < t0, "a memória não poupou tempo: {t1} contra {t0}");
    assert!(cache::andamento::atual().is_none(), "o progresso ficou preso depois da conversão");
    let _ = std::fs::remove_dir_all(&pasta);
}
