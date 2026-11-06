// SPDX-License-Identifier: LGPL-2.1-or-later
//! Balões automáticos com prévia (B299) e canto de nascimento da legenda 3D.
//!
//! O comando não mexe no documento ao ser aberto: primeiro monta um `Plano` (quantos balões nascem,
//! quantos antigos saem, quantos feitos à mão ficam) e só depois de confirmado o aplica de uma vez,
//! o que deixa o desfazer em um passo. Balão feito à mão tem `manual = true`; os automáticos, que
//! nascem aqui, têm `false` (documentos antigos, sem o campo, contam como automáticos, como era).

use crate::doc::Id;
use crate::pmi::{Pmi, PmiKind};

/// Um balão que o comando pretende criar: número de item e ponto preso na peça.
#[derive(Clone, Debug, PartialEq)]
pub struct Candidato {
    pub item: String,
    pub ancora: [f64; 3],
    /// diagonal da caixa do componente (mm): afasta o balão proporcionalmente
    pub tamanho: f64,
}

/// O que a confirmação vai fazer.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Plano {
    /// índices (em `candidatos`) dos balões que serão criados
    pub criar: Vec<usize>,
    /// balões atuais que saem
    pub trocar: Vec<Id>,
    /// balões feitos à mão que ficam
    pub preservados: usize,
    /// candidatos que não nascem porque o número já tem um balão preservado
    pub cobertos: usize,
}

impl Plano {
    /// "N criados, M trocados", o resumo do diálogo (valores; o texto é montado na interface).
    pub fn contagem(&self) -> (usize, usize) {
        (self.criar.len(), self.trocar.len())
    }
}

fn eh_balao(p: &Pmi) -> Option<(&str, bool)> {
    match &p.kind {
        PmiKind::Balloon { item, manual } => Some((item.as_str(), *manual)),
        _ => None,
    }
}

/// Conta o que acontece, sem alterar nada. Com `preservar_manuais`, os feitos à mão ficam e o número
/// que eles já cobrem não ganha um segundo balão; sem ela, saem todos e nascem todos.
pub fn planejar(existentes: &[Pmi], candidatos: &[Candidato], preservar_manuais: bool) -> Plano {
    let mut plano = Plano::default();
    let mut itens_manuais: Vec<&str> = vec![];
    for p in existentes {
        let Some((item, manual)) = eh_balao(p) else { continue };
        if manual && preservar_manuais {
            plano.preservados += 1;
            itens_manuais.push(item);
        } else {
            plano.trocar.push(p.id);
        }
    }
    for (i, c) in candidatos.iter().enumerate() {
        if itens_manuais.contains(&c.item.as_str()) {
            plano.cobertos += 1;
        } else {
            plano.criar.push(i);
        }
    }
    plano
}

/// Centro das âncoras: de lá os balões se afastam para fora do conjunto.
pub fn centro(candidatos: &[Candidato]) -> [f64; 3] {
    let n = candidatos.len().max(1) as f64;
    [0, 1, 2].map(|k| candidatos.iter().map(|c| c.ancora[k]).sum::<f64>() / n)
}

/// Posição do texto do balão em relação à âncora, afastada do centro do conjunto.
pub fn afastamento(c: &Candidato, centro: [f64; 3]) -> [f64; 3] {
    let d = [c.ancora[0] - centro[0], c.ancora[1] - centro[1]];
    let l = (d[0] * d[0] + d[1] * d[1]).sqrt();
    let u = if l > 1e-6 { [d[0] / l, d[1] / l] } else { [1., 0.] };
    let k = c.tamanho * 0.35 + 25.;
    [u[0] * k, u[1] * k, 20.]
}

/// Balão pronto para entrar na lista, afastado do centro do conjunto.
pub fn montar(c: &Candidato, centro: [f64; 3], id: Id, pasta: &str, nome: &str) -> Pmi {
    Pmi {
        id,
        name: format!("{nome} {}", c.item),
        kind: PmiKind::Balloon { item: c.item.clone(), manual: false },
        anchors: vec![c.ancora],
        offset: afastamento(c, centro),
        text: String::new(),
        tol_plus: String::new(),
        tol_minus: String::new(),
        visible: true,
        plane: None,
        style: Default::default(),
        folder: pasta.to_string(),
        targets: vec![],
    }
}

/// Tira os balões que o plano troca. Quem chama inclui depois os novos (precisa dos ids do documento).
pub fn remover_trocados(lista: &mut Vec<Pmi>, plano: &Plano) {
    lista.retain(|p| !plano.trocar.contains(&p.id));
}

/// Próximo número de item livre para um balão feito à mão.
pub fn proximo_item(existentes: &[Pmi]) -> String {
    let maior = existentes.iter().filter_map(eh_balao).filter_map(|(i, _)| i.trim().parse::<u32>().ok()).max().unwrap_or(0);
    (maior + 1).to_string()
}

/// Os oito cantos da caixa do modelo, na ordem: 0..4 em cima (z máximo) e 4..8 embaixo.
/// Em cada grupo: esquerda-frente, direita-frente, direita-fundo, esquerda-fundo (frente = y mínimo).
pub fn canto_da_caixa(lo: [f64; 3], hi: [f64; 3], i: usize) -> [f64; 3] {
    let i = i % 8;
    let (x, y) = match i % 4 {
        0 => (lo[0], lo[1]),
        1 => (hi[0], lo[1]),
        2 => (hi[0], hi[1]),
        _ => (lo[0], hi[1]),
    };
    [x, y, if i < 4 { hi[2] } else { lo[2] }]
}

/// Nome do canto para a interface (a tradução é feita por quem mostra).
pub fn nome_do_canto(i: usize) -> &'static str {
    const N: [&str; 8] = [
        "Em cima, esquerda-frente",
        "Em cima, direita-frente",
        "Em cima, direita-fundo",
        "Em cima, esquerda-fundo",
        "Embaixo, esquerda-frente",
        "Embaixo, direita-frente",
        "Embaixo, direita-fundo",
        "Embaixo, esquerda-fundo",
    ];
    N[i % 8]
}

/// Deslocamento da legenda para fora do canto escolhido (mm), para ela não cobrir a peça.
pub fn deslocamento_do_canto(i: usize) -> [f64; 3] {
    let i = i % 8;
    let sx = if matches!(i % 4, 1 | 2) { 1. } else { -1. };
    let sy = if matches!(i % 4, 2 | 3) { 1. } else { -1. };
    let sz = if i < 4 { 1. } else { -1. };
    [sx * 20., sy * 20., sz * 20.]
}

#[cfg(test)]
mod testes {
    use super::*;

    fn cand(item: &str, x: f64) -> Candidato {
        Candidato { item: item.into(), ancora: [x, 0., 10.], tamanho: 40. }
    }

    fn balao(id: u64, item: &str, manual: bool) -> Pmi {
        let mut p = montar(&cand(item, id as f64), [0.; 3], Id(id), "Balões", "Balão");
        p.kind = PmiKind::Balloon { item: item.into(), manual };
        p
    }

    fn nota(id: u64) -> Pmi {
        Pmi { kind: PmiKind::Note, ..balao(id, "x", false) }
    }

    #[test]
    fn conta_criados_e_trocados_e_ignora_o_que_nao_e_balao() {
        let ex = vec![balao(1, "1", false), balao(2, "2", false), nota(3)];
        let c = vec![cand("1", 0.), cand("2", 10.), cand("3", 20.)];
        let p = planejar(&ex, &c, true);
        assert_eq!(p.contagem(), (3, 2));
        assert_eq!(p.trocar, vec![Id(1), Id(2)]);
        assert_eq!(p.preservados, 0);
    }

    #[test]
    fn preservar_mantem_os_manuais_e_nao_duplica_o_numero() {
        let ex = vec![balao(1, "1", false), balao(2, "2", true)];
        let c = vec![cand("1", 0.), cand("2", 10.), cand("3", 20.)];
        let p = planejar(&ex, &c, true);
        assert_eq!(p.contagem(), (2, 1), "o 2 manual cobre o candidato 2");
        assert_eq!((p.preservados, p.cobertos), (1, 1));
        let mut lista = ex.clone();
        remover_trocados(&mut lista, &p);
        assert_eq!(lista.len(), 1);
        assert!(matches!(&lista[0].kind, PmiKind::Balloon { manual: true, .. }));
    }

    #[test]
    fn sem_preservar_troca_tudo() {
        let ex = vec![balao(1, "1", false), balao(2, "2", true)];
        let c = vec![cand("1", 0.), cand("2", 10.)];
        let p = planejar(&ex, &c, false);
        assert_eq!(p.contagem(), (2, 2));
        let mut lista = ex;
        remover_trocados(&mut lista, &p);
        assert!(lista.is_empty());
    }

    #[test]
    fn balao_se_afasta_do_centro_e_o_documento_antigo_conta_como_automatico() {
        let cs = vec![cand("1", -50.), cand("2", 50.)];
        let m = montar(&cs[1], centro(&cs), Id(9), "", "B");
        assert!(m.offset[0] > 25. && m.offset[1].abs() < 1e-9, "para a direita, longe do centro");
        let antigo: PmiKind = serde_json::from_str(r#"{"Balloon":{"item":"4"}}"#).unwrap();
        assert_eq!(antigo, PmiKind::Balloon { item: "4".into(), manual: false });
    }

    #[test]
    fn proximo_item_segue_o_maior_numero() {
        assert_eq!(proximo_item(&[]), "1");
        assert_eq!(proximo_item(&[balao(1, "3", true), balao(2, "7", false), nota(3)]), "8");
    }

    #[test]
    fn cantos_da_caixa_e_deslocamento_apontam_para_fora() {
        let (lo, hi) = ([0., 0., 0.], [10., 20., 30.]);
        assert_eq!(canto_da_caixa(lo, hi, 0), [0., 0., 30.]);
        assert_eq!(canto_da_caixa(lo, hi, 2), [10., 20., 30.]);
        assert_eq!(canto_da_caixa(lo, hi, 4), [0., 0., 0.]);
        for i in 0..8 {
            let c = canto_da_caixa(lo, hi, i);
            let d = deslocamento_do_canto(i);
            let meio = [5., 10., 15.];
            for k in 0..3 {
                assert!((c[k] - meio[k]) * d[k] > 0., "canto {i} eixo {k}");
            }
        }
    }
}
