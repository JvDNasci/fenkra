// SPDX-License-Identifier: LGPL-2.1-or-later
//! Malha compartilhada e nível de detalhe da vista da montagem (B125).
//!
//! Cem cópias da mesma peça são a mesma malha em cem lugares. O `Prototipo` guarda essa malha uma vez,
//! no sistema da peça, em até três qualidades; a `Instancia` guarda só onde a cópia está. A placa de
//! vídeo recebe o protótipo uma vez e desenha todas as cópias de uma vez só.
//!
//! O nível vem do erro na tela: o desvio da malha, em milímetros, vira pixels na distância da cópia.
//! Enquanto a malha grossa erra menos que o limiar, ninguém enxerga a diferença, e a peça usa ela.
//! Volume, massa, seleção e medidas continuam na malha fina e na forma exata: o nível só muda o desenho.

use crate::Placement;
use fenkra_core::QualidadeMalha;
use fenkra_kernel::Mesh;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Qualidade do nível mais grosso: os limites aceitos para valores digitados.
pub const LONGE: QualidadeMalha = QualidadeMalha { desvio_rel: QualidadeMalha::REL_MAX, angulo_graus: QualidadeMalha::ANG_MAX };

/// Um nível só entra se cortar ao menos esta fração dos triângulos do nível anterior.
const GANHO_MINIMO: f64 = 0.25;

/// Erro na tela, em pixels, que a pessoa aceita sem notar (com folga para o anti-serrilhamento).
pub const LIMIAR_PX: f32 = 1.5;

/// Caixa alinhada aos eixos: mínimo e máximo.
pub type Caixa = ([f32; 3], [f32; 3]);

/// Caixa de uma malha; `None` se ela não tem vértices.
pub fn caixa_de(m: &Mesh) -> Option<Caixa> {
    let mut it = m.positions.chunks_exact(3);
    let p = it.next()?;
    let (mut lo, mut hi) = ([p[0], p[1], p[2]], [p[0], p[1], p[2]]);
    for p in it {
        for k in 0..3 {
            lo[k] = lo[k].min(p[k]);
            hi[k] = hi[k].max(p[k]);
        }
    }
    Some((lo, hi))
}

/// Caixa envolvente da caixa `c` depois de posicionada.
pub fn caixa_posta(c: &Caixa, p: &Placement) -> Caixa {
    let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
    for k in 0..8 {
        let canto = [0, 1, 2].map(|e| if k >> e & 1 == 0 { c.0[e] as f64 } else { c.1[e] as f64 });
        let q = p.point(canto);
        for e in 0..3 {
            lo[e] = lo[e].min(q[e] as f32);
            hi[e] = hi[e].max(q[e] as f32);
        }
    }
    (lo, hi)
}

/// Uma qualidade da malha do protótipo.
pub struct Nivel {
    pub malha: Mesh,
    /// desvio máximo da malha em relação à forma exata, em milímetros
    pub desvio: f32,
}

/// Malha de uma peça ou subconjunto, no sistema dele, compartilhada por todas as cópias.
pub struct Prototipo {
    /// identidade para a placa de vídeo: muda sempre que a malha muda
    pub id: u64,
    /// do mais fino ao mais grosso; o primeiro existe sempre
    pub niveis: Vec<Nivel>,
    pub caixa: Caixa,
}

impl Prototipo {
    /// Monta o protótipo, do nível mais fino ao mais grosso. Nível que quase não economiza é
    /// descartado: ocupar a placa com ele não paga.
    pub fn novo(malhas: Vec<(Mesh, QualidadeMalha)>) -> Arc<Prototipo> {
        static PROXIMO: AtomicU64 = AtomicU64::new(1);
        let caixa = malhas.first().and_then(|(m, _)| caixa_de(m)).unwrap_or(([0.; 3], [0.; 3]));
        let diagonal = (0..3).map(|k| ((caixa.1[k] - caixa.0[k]) as f64).powi(2)).sum::<f64>().sqrt();
        let mut niveis: Vec<Nivel> = vec![];
        for (malha, q) in malhas {
            let vale = niveis.last().is_none_or(|n| (malha.indices.len() as f64) < n.malha.indices.len() as f64 * (1. - GANHO_MINIMO));
            if vale {
                niveis.push(Nivel { malha, desvio: q.desvio(diagonal) as f32 });
            }
        }
        if niveis.is_empty() {
            niveis.push(Nivel { malha: Mesh::default(), desvio: 0. });
        }
        Arc::new(Prototipo { id: PROXIMO.fetch_add(1, Ordering::Relaxed), niveis, caixa })
    }

    pub fn fina(&self) -> &Mesh {
        &self.niveis[0].malha
    }

    /// Índice do nível mais grosso.
    pub fn mais_grosso(&self) -> usize {
        self.niveis.len() - 1
    }

    /// Desvio de cada nível, do mais fino ao mais grosso.
    pub fn desvios(&self) -> Vec<f32> {
        self.niveis.iter().map(|n| n.desvio).collect()
    }
}

/// Uma cópia posicionada, ligada ao corpo `corpo` do resultado da vista.
#[derive(Clone)]
pub struct Instancia {
    pub corpo: usize,
    pub prototipo: Arc<Prototipo>,
    /// subconjunto com peças de aparências diferentes: desenha estas no lugar do protótipo, que
    /// segue valendo para a seleção e a caixa
    pub partes: Arc<Vec<Parte>>,
    pub posicao: Placement,
    /// a pessoa pediu a versão simplificada deste componente: fica sempre no nível mais grosso
    pub simplificada: bool,
}

/// Pedaço de um subconjunto em que todas as peças têm a mesma aparência. Uma cópia tem uma cor só,
/// então cada aparência precisa do próprio protótipo para continuar sendo desenhada como cópia.
#[derive(Clone)]
pub struct Parte {
    pub prototipo: Arc<Prototipo>,
    pub aparencia: fenkra_core::Appearance,
}

/// Qualidades geradas para uma peça: a escolhida, o rascunho do B61 e a de longe, sem repetir nem
/// passar para uma mais fina que a escolhida.
pub fn qualidades(q: QualidadeMalha) -> Vec<QualidadeMalha> {
    let mut v = vec![q];
    for g in [QualidadeMalha::RASCUNHO, LONGE] {
        if g.desvio_rel > v[v.len() - 1].desvio_rel {
            v.push(g);
        }
    }
    v
}

/// Nível mais grosso cujo erro, nesta escala de tela, fica abaixo do limiar.
pub fn nivel_para_tela(desvios: &[f32], px_por_mm: f32, limiar_px: f32) -> usize {
    desvios.iter().rposition(|d| d * px_por_mm <= limiar_px).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::load::PartCache;
    use crate::view::{compor, compose};
    use crate::{Assembly, NodeKind};
    use fenkra_core::trabalho::Controle;
    use std::path::PathBuf;

    /// Pasta temporária com a peça de exemplo e uma montagem de `n` cópias em grade.
    fn grade(nome: &str, n: usize) -> (Assembly, PathBuf) {
        let dir = std::env::temp_dir().join(format!("fenkra_lod_{nome}_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("placa.fkp"), fenkra_solidos::example_document().to_json()).unwrap();
        let mut a = Assembly::new("grade");
        for k in 0..n {
            a.add_component("Placa", NodeKind::Part { path: "placa.fkp".into() }, Placement { origin: [(k % 10) as f64 * 150., (k / 10) as f64 * 100., 0.], ..Default::default() }, None);
        }
        (a, dir)
    }

    const VERMELHO: fenkra_core::Appearance = fenkra_core::Appearance { color: [200, 30, 30], finish: fenkra_core::Finish::MetalPolido, texture: None };
    const AZUL: fenkra_core::Appearance = fenkra_core::Appearance { color: [30, 60, 200], finish: fenkra_core::Finish::Borracha, texture: None };

    fn pasta_limpa(nome: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("fenkra_lod_{nome}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn peca_pintada(dir: &std::path::Path, nome: &str, aparencia: fenkra_core::Appearance) {
        let mut d = fenkra_solidos::example_document();
        d.appearance = Some(aparencia);
        std::fs::write(dir.join(nome), d.to_json()).unwrap();
    }

    /// Subconjunto com as duas peças e a montagem de topo que o usa.
    fn topo_de(dir: &std::path::Path, b: &str) -> Assembly {
        let mut sub = Assembly::new("Sub");
        sub.add_component("A", NodeKind::Part { path: "a.fkp".into() }, Placement::default(), None);
        sub.add_component("B", NodeKind::Part { path: b.into() }, Placement { origin: [200., 0., 0.], ..Default::default() }, None);
        std::fs::write(dir.join("sub.fka"), sub.to_json()).unwrap();
        let mut topo = Assembly::new("Topo");
        topo.add_component("Sub", NodeKind::SubAssembly { path: "sub.fka".into() }, Placement::default(), None);
        topo
    }

    #[test]
    fn a_montagem_mostra_a_cor_e_o_acabamento_da_peca() {
        let dir = pasta_limpa("pintada");
        peca_pintada(&dir, "a.fkp", VERMELHO);
        let mut a = Assembly::new("A");
        let id = a.add_component("A", NodeKind::Part { path: "a.fkp".into() }, Placement::default(), None);
        let c = compor(&a, Some(&dir), &mut PartCache::default(), &mut vec![], None).unwrap();
        assert_eq!((c.doc.collectors[0].color, c.doc.collectors[0].finish), (VERMELHO.color, VERMELHO.finish));

        // a cor do nó vence, e o acabamento continua o da peça
        a.node_mut(id).unwrap().color = Some([1, 2, 3]);
        let c = compor(&a, Some(&dir), &mut PartCache::default(), &mut vec![], None).unwrap();
        assert_eq!((c.doc.collectors[0].color, c.doc.collectors[0].finish), ([1, 2, 3], VERMELHO.finish));
    }

    #[test]
    fn subconjunto_mostra_cada_peca_com_a_aparencia_dela() {
        let dir = pasta_limpa("sub_pintado");
        peca_pintada(&dir, "a.fkp", VERMELHO);
        peca_pintada(&dir, "b.fkp", AZUL);
        let mut topo = topo_de(&dir, "b.fkp");
        let c = compor(&topo, Some(&dir), &mut PartCache::default(), &mut vec![], None).unwrap();
        let i = &c.instancias[0];
        let aparencias: Vec<_> = i.partes.iter().map(|p| p.aparencia).collect();
        assert_eq!(aparencias, vec![VERMELHO, AZUL]);
        // as partes somam o conjunto, sem faltar nem repetir triângulo
        let soma: usize = i.partes.iter().map(|p| p.prototipo.fina().indices.len()).sum();
        assert_eq!(soma, i.prototipo.fina().indices.len());

        // pintar o nó do subconjunto cobre tudo
        let id = topo.nodes[0].id;
        topo.node_mut(id).unwrap().color = Some([9, 9, 9]);
        let c = compor(&topo, Some(&dir), &mut PartCache::default(), &mut vec![], None).unwrap();
        assert!(c.instancias[0].partes.is_empty());
        assert_eq!(c.doc.collectors[0].color, [9, 9, 9]);
    }

    #[test]
    fn subconjunto_de_uma_aparencia_so_nao_guarda_malha_a_mais() {
        let dir = pasta_limpa("sub_igual");
        peca_pintada(&dir, "a.fkp", VERMELHO);
        let topo = topo_de(&dir, "a.fkp");
        let c = compor(&topo, Some(&dir), &mut PartCache::default(), &mut vec![], None).unwrap();
        assert!(c.instancias[0].partes.is_empty());
        assert_eq!((c.doc.collectors[0].color, c.doc.collectors[0].finish), (VERMELHO.color, VERMELHO.finish));
    }

    #[test]
    fn pintar_um_no_dentro_do_subconjunto_refaz_as_partes() {
        let dir = pasta_limpa("sub_repinta");
        peca_pintada(&dir, "a.fkp", VERMELHO);
        peca_pintada(&dir, "b.fkp", AZUL);
        let topo = topo_de(&dir, "b.fkp");
        let mut cache = PartCache::default();
        compor(&topo, Some(&dir), &mut cache, &mut vec![], None).unwrap();
        let mut sub = Assembly::from_json(&std::fs::read_to_string(dir.join("sub.fka")).unwrap()).unwrap();
        let id = sub.nodes[1].id;
        sub.node_mut(id).unwrap().color = Some([0, 200, 0]);
        std::fs::write(dir.join("sub.fka"), sub.to_json()).unwrap();
        let c = compor(&topo, Some(&dir), &mut cache, &mut vec![], None).unwrap();
        assert_eq!(c.instancias[0].partes[1].aparencia.color, [0, 200, 0], "o cache não pode segurar a cor antiga");
    }

    #[test]
    fn copias_da_mesma_peca_dividem_a_malha() {
        let (a, dir) = grade("copias", 12);
        let mut cache = PartCache::default();
        let mut erros = vec![];
        let c = compor(&a, Some(&dir), &mut cache, &mut erros, None).unwrap();
        assert!(erros.is_empty());
        assert_eq!(c.instancias.len(), 12);
        assert!(c.instancias.iter().all(|i| Arc::ptr_eq(&i.prototipo, &c.instancias[0].prototipo)), "uma malha só");
        assert_eq!(c.caixas.len(), c.result.bodies.len());
        // a caixa da cópia posta é a da malha que a seleção usa, e o volume é o da peça
        for (b, caixa) in c.result.bodies.iter().zip(&c.caixas) {
            let exata = caixa_de(&b.mesh).unwrap();
            for k in 0..3 {
                assert!((exata.0[k] - caixa.0[k]).abs() < 1e-2 && (exata.1[k] - caixa.1[k]).abs() < 1e-2);
            }
            assert!((b.mass.volume - c.result.bodies[0].mass.volume).abs() < 1e-6 && b.mass.volume > 0.);
        }
    }

    #[test]
    fn niveis_grossos_cortam_triangulos_sem_mudar_faces_nem_caixa() {
        let (a, dir) = grade("niveis", 2);
        let mut cache = PartCache::com_niveis();
        let c = compor(&a, Some(&dir), &mut cache, &mut vec![], None).unwrap();
        let p = &c.instancias[0].prototipo;
        assert!(p.niveis.len() >= 2, "a peça de exemplo tem curvas: há o que engrossar");
        for w in p.niveis.windows(2) {
            assert!(w[1].malha.indices.len() < w[0].malha.indices.len());
            assert!(w[1].desvio > w[0].desvio);
            assert_eq!(w[1].malha.face_ranges.len(), w[0].malha.face_ranges.len(), "a topologia é a mesma");
        }
        let fina = caixa_de(p.fina()).unwrap();
        let grossa = caixa_de(&p.niveis[p.mais_grosso()].malha).unwrap();
        let tol = p.niveis[p.mais_grosso()].desvio + 1e-3;
        for k in 0..3 {
            assert!((fina.0[k] - grossa.0[k]).abs() <= tol && (fina.1[k] - grossa.1[k]).abs() <= tol);
        }
        // a peça é a mesma: o volume do corpo não depende de haver níveis
        let (_, r) = compose(&a, Some(&dir), &mut PartCache::default(), &mut vec![]);
        assert!((r.bodies[0].mass.volume - c.result.bodies[0].mass.volume).abs() < 1e-6);
    }

    #[test]
    fn protótipo_sobrevive_a_recompor_e_muda_quando_a_peca_muda() {
        let (a, dir) = grade("estavel", 2);
        let mut cache = PartCache::com_niveis();
        let id = |c: &mut PartCache| compor(&a, Some(&dir), c, &mut vec![], None).unwrap().instancias[0].prototipo.id;
        let primeiro = id(&mut cache);
        assert_eq!(id(&mut cache), primeiro, "nada mudou: a placa não sobe a malha de novo");
        cache.forget(&dir.join("placa.fkp"));
        assert_ne!(id(&mut cache), primeiro);
        assert_eq!(cache.tirar_esquecidas(), vec![dir.join("placa.fkp")]);
    }

    #[test]
    fn composicao_cancelada_nao_deixa_nada_e_a_seguinte_sai_igual_a_do_zero() {
        let (a, dir) = grade("cancelar", 3);
        let mut cache = PartCache::com_niveis();
        let ctl = Controle::novo();
        ctl.cancelar();
        assert!(compor(&a, Some(&dir), &mut cache, &mut vec![], Some(&ctl)).is_none());
        // peça abandonada no meio do cálculo também não fica no cache
        cache.cancelar = Some(ctl.bandeira());
        assert!(cache.get(&dir.join("placa.fkp"), false).is_err());
        cache.cancelar = None;
        let c = compor(&a, Some(&dir), &mut cache, &mut vec![], Some(&Controle::novo())).unwrap();
        let (_, zero) = compose(&a, Some(&dir), &mut PartCache::default(), &mut vec![]);
        assert_eq!(c.result.bodies.len(), zero.bodies.len());
        assert_eq!(c.result.bodies[2].mesh.positions, zero.bodies[2].mesh.positions);
    }

    #[test]
    fn documento_lido_na_thread_chega_a_arvore_sem_tocar_no_disco() {
        let (a, dir) = grade("recem", 2);
        let mut fundo = PartCache::com_niveis();
        compor(&a, Some(&dir), &mut fundo, &mut vec![], None).unwrap();
        let mut interface = PartCache::default();
        let f = dir.join("placa.fkp");
        assert!(interface.documento_pronto(&f).is_none());
        for (p, d) in fundo.tirar_recem() {
            interface.semear_documento(p, d);
        }
        assert!(interface.documento_pronto(&f).is_some_and(|d| !d.features.is_empty()));
        compor(&a, Some(&dir), &mut fundo, &mut vec![], None).unwrap();
        assert!(fundo.tirar_recem().is_empty(), "nada novo para mandar");
    }

    #[test]
    fn subconjunto_repetido_tambem_divide_a_malha() {
        let (sub, dir) = grade("sub", 2);
        std::fs::write(dir.join("sub.fka"), sub.to_json()).unwrap();
        let mut a = Assembly::new("topo");
        for k in 0..3 {
            a.add_component("Sub", NodeKind::SubAssembly { path: "sub.fka".into() }, Placement { origin: [0., k as f64 * 500., 0.], ..Default::default() }, None);
        }
        let mut cache = PartCache::com_niveis();
        let c = compor(&a, Some(&dir), &mut cache, &mut vec![], None).unwrap();
        assert_eq!(c.instancias.len(), 3);
        assert!(Arc::ptr_eq(&c.instancias[0].prototipo, &c.instancias[2].prototipo));
        assert!(c.instancias[0].prototipo.niveis.len() >= 2);
        let id = c.instancias[0].prototipo.id;
        assert_eq!(compor(&a, Some(&dir), &mut cache, &mut vec![], None).unwrap().instancias[0].prototipo.id, id);
    }

    fn malha(tris: usize) -> Mesh {
        Mesh { positions: vec![0., 0., 0., 100., 0., 0., 0., 100., 0.], indices: vec![0; tris * 3], ..Default::default() }
    }

    #[test]
    fn perto_usa_a_fina_e_longe_a_grossa() {
        let d = [0.1, 0.4, 2.];
        assert_eq!(nivel_para_tela(&d, 20., 1.5), 0, "0,1 mm são 2 px: nem a fina passa, fica nela");
        assert_eq!(nivel_para_tela(&d, 10., 1.5), 0);
        assert_eq!(nivel_para_tela(&d, 3., 1.5), 1);
        assert_eq!(nivel_para_tela(&d, 0.5, 1.5), 2);
    }

    #[test]
    fn navegar_com_limiar_maior_engrossa() {
        let d = [0.1, 0.4, 2.];
        assert!(nivel_para_tela(&d, 3., 1.5 * 4.) > nivel_para_tela(&d, 3., 1.5));
    }

    #[test]
    fn niveis_seguem_do_fino_ao_grosso_sem_repetir() {
        assert_eq!(qualidades(QualidadeMalha::NORMAL).len(), 3);
        assert_eq!(qualidades(QualidadeMalha::ALTA).len(), 3);
        assert_eq!(qualidades(QualidadeMalha::RASCUNHO).len(), 2);
        assert_eq!(qualidades(LONGE).len(), 1);
    }

    #[test]
    fn nivel_que_nao_economiza_nao_entra() {
        let q = QualidadeMalha::NORMAL;
        let p = Prototipo::novo(vec![(malha(1000), q), (malha(900), QualidadeMalha::RASCUNHO), (malha(200), LONGE)]);
        assert_eq!(p.niveis.len(), 2);
        assert_eq!(p.niveis[1].malha.indices.len(), 600);
    }

    #[test]
    fn caixa_posta_acompanha_a_posicao() {
        let p = Placement { origin: [10., 0., 0.], ..Default::default() };
        let c = caixa_posta(&([0., 0., 0.], [1., 2., 3.]), &p);
        assert_eq!(c, ([10., 0., 0.], [11., 2., 3.]));
    }
}
