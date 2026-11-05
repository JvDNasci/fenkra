// SPDX-License-Identifier: LGPL-2.1-or-later
//! Alças diretas na folha (B250): as operações por trás das alças que aparecem na vista selecionada
//! (inverter o lado do corte, raio do detalhe, escala por passos, alinhamento com a vista mãe e
//! distância ao vizinho). Ficam aqui, sem interface, para serem testadas e para a tela só desenhar e
//! chamar. Cada operação muda o desenho de uma vez só: um passo de desfazer.

use crate::{Dir, Drawing, ViewKind, V2};
use fenkra_core::Id;

/// Escalas preferidas (ISO 5455): ampliações, 1:1 e reduções.
pub const ESCALAS: [f64; 17] = [
    50., 20., 10., 5., 2., 1., 1. / 2., 1. / 5., 1. / 10., 1. / 20., 1. / 50., 1. / 100., 1. / 200., 1. / 500., 1. / 1000., 1. / 2000., 1. / 5000.,
];

/// Menor raio que a alça de detalhe aceita (mm do modelo).
pub const RAIO_MIN: f64 = 0.5;

/// Escala vizinha na série preferida: `passo` positivo amplia, negativo reduz. Uma escala fora da
/// série (por exemplo 1:3) vai para a mais próxima na direção pedida.
pub fn escala_passo(atual: f64, passo: i32) -> f64 {
    if passo == 0 || atual <= 0. {
        return atual;
    }
    // ESCALAS é decrescente: índice menor = escala maior
    let idx = ESCALAS.iter().position(|e| (e / atual - 1.).abs() < 1e-9);
    let alvo = match idx {
        Some(i) => i as i32 - passo,
        None => {
            // primeira da série menor que a atual
            let abaixo = ESCALAS.iter().position(|e| *e < atual).map(|p| p as i32).unwrap_or(ESCALAS.len() as i32);
            if passo > 0 {
                abaixo - passo
            } else {
                abaixo - 1 - passo
            }
        }
    };
    ESCALAS[alvo.clamp(0, ESCALAS.len() as i32 - 1) as usize]
}

/// Vista cuja escala vale para `id`: projetadas e cortes herdam a da mãe (o alinhamento do
/// desenho as força a isso), então a alça de escala delas muda a da vista de origem.
pub fn vista_da_escala(d: &Drawing, id: Id) -> Option<Id> {
    let mut cur = id;
    for _ in 0..16 {
        let v = d.view(cur)?;
        match &v.kind {
            ViewKind::Projected { parent, .. } | ViewKind::Section { parent, .. } => cur = *parent,
            _ => return Some(cur),
        }
    }
    None
}

/// Dá um passo na escala da vista (ou da vista de origem, se ela herda). Devolve a escala nova.
pub fn passo_de_escala(d: &mut Drawing, id: Id, passo: i32) -> Option<f64> {
    let alvo = vista_da_escala(d, id)?;
    let v = d.views.iter_mut().find(|v| v.id == alvo)?;
    let nova = escala_passo(v.scale, passo);
    if (nova - v.scale).abs() < 1e-12 {
        return None;
    }
    v.scale = nova;
    Some(nova)
}

/// A vista está presa à mãe por alinhamento (projetada)? Devolve a mãe e o eixo preso:
/// `true` = o x é igual ao da mãe (vistas de cima e de baixo), `false` = o y (laterais).
pub fn travada_na_mae(d: &Drawing, id: Id) -> Option<(Id, bool)> {
    let v = d.view(id)?;
    match &v.kind {
        ViewKind::Projected { parent, dir } => match dir {
            Dir::Topo | Dir::Base => Some((*parent, true)),
            Dir::Esquerda | Dir::Direita | Dir::Tras => Some((*parent, false)),
            _ => None,
        },
        _ => None,
    }
}

/// Inverte o lado de observação do corte: troca as pontas da linha de corte. As setas e a vista
/// cortada passam a olhar para o lado oposto. `false` se `id` não é um corte.
pub fn inverter_corte(d: &mut Drawing, id: Id) -> bool {
    let Some(v) = d.views.iter_mut().find(|v| v.id == id) else { return false };
    match &mut v.kind {
        ViewKind::Section { a, b, .. } => {
            std::mem::swap(a, b);
            true
        }
        _ => false,
    }
}

/// Muda o raio do círculo de um detalhe (mm do modelo da vista mãe), com o mínimo de `RAIO_MIN`.
/// `false` se `id` não é um detalhe ou o raio não muda.
pub fn definir_raio_detalhe(d: &mut Drawing, id: Id, raio: f64) -> bool {
    let Some(v) = d.views.iter_mut().find(|v| v.id == id) else { return false };
    match &mut v.kind {
        ViewKind::Detail { radius, .. } => {
            if !raio.is_finite() {
                return false;
            }
            let novo = raio.max(RAIO_MIN);
            if (novo - *radius).abs() < 1e-9 {
                return false;
            }
            *radius = novo;
            true
        }
        _ => false,
    }
}

/// Distância entre as caixas de duas vistas e o par de pontos que a mede (um em cada caixa).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vizinho {
    pub dist: f64,
    pub de: V2,
    pub ate: V2,
    pub id: Id,
}

/// Vizinho mais perto da caixa `(lo, hi)` entre as caixas dadas (menos a própria `id`). A distância
/// é a folga entre os retângulos; caixas que se tocam ou se sobrepõem não contam.
pub fn vizinho_mais_perto(id: Id, lo: V2, hi: V2, outras: &[(Id, V2, V2)]) -> Option<Vizinho> {
    let mut melhor: Option<Vizinho> = None;
    for (oid, olo, ohi) in outras.iter().filter(|o| o.0 != id) {
        let dx = (olo[0] - hi[0]).max(lo[0] - ohi[0]).max(0.);
        let dy = (olo[1] - hi[1]).max(lo[1] - ohi[1]).max(0.);
        let dist = (dx * dx + dy * dy).sqrt();
        if dist <= 0. {
            continue;
        }
        // ponto de `a` mais perto de `b`: a borda voltada para `b`, ou o centro de `b` limitado à
        // faixa em comum quando os intervalos se sobrepõem
        let eixo = |alo: f64, ahi: f64, blo: f64, bhi: f64| -> f64 {
            if ahi < blo {
                ahi
            } else if alo > bhi {
                alo
            } else {
                ((blo + bhi) / 2.).clamp(alo.max(blo), ahi.min(bhi))
            }
        };
        let de = [eixo(lo[0], hi[0], olo[0], ohi[0]), eixo(lo[1], hi[1], olo[1], ohi[1])];
        let ate = [eixo(olo[0], ohi[0], lo[0], hi[0]), eixo(olo[1], ohi[1], lo[1], hi[1])];
        let cand = Vizinho { dist, de, ate, id: *oid };
        if melhor.is_none_or(|b| cand.dist < b.dist) {
            melhor = Some(cand);
        }
    }
    melhor
}

/// Linhas de alinhamento (entre os centros) da vista `id` com as outras da folha cujo centro cai
/// no mesmo x ou y, dentro de `tol` mm. Aparecem enquanto a vista é arrastada.
pub fn alinhamentos(d: &Drawing, id: Id, tol: f64) -> Vec<(V2, V2)> {
    let Some(v) = d.view(id) else { return vec![] };
    d.views
        .iter()
        .filter(|o| o.id != id && o.sheet == v.sheet)
        .filter(|o| (o.pos[0] - v.pos[0]).abs() <= tol || (o.pos[1] - v.pos[1]).abs() <= tol)
        .map(|o| (v.pos, o.pos))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Format, Source};

    fn src() -> Source {
        Source { path: "p.fkp".into(), display_state: String::new(), hidden: vec![] }
    }

    #[test]
    fn escala_anda_pela_serie_preferida() {
        assert_eq!(escala_passo(1., -1), 0.5);
        assert_eq!(escala_passo(1., 1), 2.);
        assert_eq!(escala_passo(0.5, -1), 0.2);
        assert_eq!(escala_passo(2., 2), 10.);
        // fora da série: 1:3 reduz para 1:5 e amplia para 1:2
        assert_eq!(escala_passo(1. / 3., -1), 0.2);
        assert_eq!(escala_passo(1. / 3., 1), 0.5);
        // nos limites não passa
        assert_eq!(escala_passo(50., 1), 50.);
        assert_eq!(escala_passo(1. / 5000., -1), 1. / 5000.);
    }

    #[test]
    fn escala_da_projetada_muda_a_da_vista_de_origem() {
        let mut d = Drawing::new("t", Format::A3);
        let s = d.sheets[0].id;
        let ids = d.add_standard_set(s, src(), [100., 200.], 1., 90.);
        // a superior (ids[1]) é projetada da frente (ids[0])
        assert_eq!(passo_de_escala(&mut d, ids[1], -1), Some(0.5));
        assert_eq!(d.view(ids[0]).unwrap().scale, 0.5);
        crate::layout::align_views(&mut d);
        assert_eq!(d.view(ids[1]).unwrap().scale, 0.5, "a projetada acompanha a mãe");
    }

    #[test]
    fn cadeado_so_nas_projetadas_alinhadas() {
        let mut d = Drawing::new("t", Format::A3);
        let s = d.sheets[0].id;
        let ids = d.add_standard_set(s, src(), [100., 200.], 1., 90.);
        assert_eq!(travada_na_mae(&d, ids[0]), None);
        assert_eq!(travada_na_mae(&d, ids[1]), Some((ids[0], true)));
        assert_eq!(travada_na_mae(&d, ids[2]), Some((ids[0], false)));
    }

    #[test]
    fn inverter_corte_troca_as_pontas_e_volta_ao_inverter_de_novo() {
        let mut d = Drawing::new("t", Format::A3);
        let s = d.sheets[0].id;
        let m = d.add_view(s, src(), ViewKind::Standard(Dir::Frente), [100., 100.], 1.);
        let c = d.add_view(s, src(), ViewKind::Section { parent: m, a: [0., 5.], b: [40., 5.], label: "A".into() }, [200., 100.], 1.);
        let antes = d.clone();
        assert!(inverter_corte(&mut d, c));
        match &d.view(c).unwrap().kind {
            ViewKind::Section { a, b, .. } => assert_eq!((*a, *b), ([40., 5.], [0., 5.])),
            _ => unreachable!(),
        }
        assert!(inverter_corte(&mut d, c));
        assert_eq!(d, antes);
        assert!(!inverter_corte(&mut d, m), "vista comum não é corte");
    }

    #[test]
    fn raio_do_detalhe_respeita_o_minimo_e_recusa_o_que_nao_muda() {
        let mut d = Drawing::new("t", Format::A3);
        let s = d.sheets[0].id;
        let m = d.add_view(s, src(), ViewKind::Standard(Dir::Frente), [100., 100.], 1.);
        let det = d.add_view(s, src(), ViewKind::Detail { parent: m, center: [1., 2.], radius: 10., label: "B".into() }, [200., 100.], 2.);
        assert!(definir_raio_detalhe(&mut d, det, 14.));
        assert!(matches!(&d.view(det).unwrap().kind, ViewKind::Detail { radius, .. } if (*radius - 14.).abs() < 1e-12));
        assert!(definir_raio_detalhe(&mut d, det, 0.01));
        assert!(matches!(&d.view(det).unwrap().kind, ViewKind::Detail { radius, .. } if (*radius - RAIO_MIN).abs() < 1e-12));
        assert!(!definir_raio_detalhe(&mut d, det, RAIO_MIN), "sem mudança");
        assert!(!definir_raio_detalhe(&mut d, m, 5.), "não é detalhe");
    }

    #[test]
    fn vizinho_mede_a_folga_entre_as_caixas() {
        let a = (Id(1), [0., 0.], [10., 10.]);
        let b = (Id(2), [25., 2.], [40., 8.]);
        let c = (Id(3), [0., 50.], [10., 60.]);
        let v = vizinho_mais_perto(Id(1), a.1, a.2, &[a, b, c]).unwrap();
        assert_eq!(v.id, Id(2));
        assert!((v.dist - 15.).abs() < 1e-9);
        assert!((v.de[0] - 10.).abs() < 1e-9 && (v.ate[0] - 25.).abs() < 1e-9);
        // diagonal: folga euclidiana
        let e = (Id(4), [13., 14.], [20., 20.]);
        let v = vizinho_mais_perto(Id(1), a.1, a.2, &[a, e]).unwrap();
        assert!((v.dist - 5.).abs() < 1e-9);
        // caixas que se tocam não contam
        assert!(vizinho_mais_perto(Id(1), a.1, a.2, &[a, (Id(5), [5., 5.], [20., 20.])]).is_none());
    }

    #[test]
    fn alinhamento_lista_as_vistas_com_o_mesmo_centro() {
        let mut d = Drawing::new("t", Format::A3);
        let s = d.sheets[0].id;
        let ids = d.add_standard_set(s, src(), [100., 200.], 1., 90.);
        let l = alinhamentos(&d, ids[1], 0.01);
        assert!(l.iter().any(|(_, o)| (o[0] - d.view(ids[0]).unwrap().pos[0]).abs() < 1e-9), "{l:?}");
    }

    #[test]
    fn inverter_corte_vira_a_vista_cortada_para_o_lado_oposto() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let dir = std::env::temp_dir().join("fenkra_desenho_alcas");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("placa.fkp"), fenkra_solidos::example_document().to_json()).unwrap();
        let fonte = Source { path: "placa.fkp".into(), display_state: String::new(), hidden: vec![] };
        let mut d = Drawing::new("t", Format::A3);
        let s = d.sheets[0].id;
        let mut m = crate::source::Models::new(Some(dir));
        let f = d.add_view(s, fonte.clone(), ViewKind::Standard(Dir::Frente), [100., 100.], 1.);
        let pf = m.proj(&d, f);
        let (lo, hi) = (pf.lo, pf.hi);
        let ym = (lo[1] + hi[1]) / 2.;
        let c = d.add_view(s, fonte, ViewKind::Section { parent: f, a: [lo[0] - 5., ym], b: [hi[0] + 5., ym], label: "A".into() }, [300., 100.], 1.);
        let antes = m.proj(&d, c);
        assert!(antes.error.is_none() && !antes.hatch.is_empty());
        let normal_antes = antes.axes.unwrap().toward;
        assert!(inverter_corte(&mut d, c));
        let depois = m.proj(&d, c);
        assert!(depois.error.is_none(), "{:?}", depois.error);
        assert!(!depois.hatch.is_empty(), "o corte invertido continua cortando a peça");
        let normal_depois = depois.axes.unwrap().toward;
        for k in 0..3 {
            assert!((normal_antes[k] + normal_depois[k]).abs() < 1e-9, "a vista passa a olhar para o lado oposto: {normal_antes:?} {normal_depois:?}");
        }
    }
}
