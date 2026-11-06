// SPDX-License-Identifier: LGPL-2.1-or-later
//! Diagnóstico de conflito (B103). Quando o sistema não fecha, o resíduo final não diz quem é o
//! culpado: o mínimos quadrados espalha o erro, e uma restrição inocente aparece como falha. A resposta
//! certa é o núcleo: o menor conjunto de restrições que, junto com a que entrou, não tem solução. Tirar
//! qualquer uma dele resolve o conflito.

use crate::solve::{self, vizinhos_de};
use crate::*;

/// Soluções de teste que o diagnóstico pode gastar. A prévia roda a cada mudança no painel, e um
/// núcleo maior que isso não ajudaria ninguém a decidir.
const MAX_TESTES: usize = 48;

/// Resultado do diagnóstico.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Nucleo {
    /// restrições e juntas que, com o item, não têm solução; sem o item
    pub envolvidas: Vec<Id>,
    /// o conjunto é mínimo. Sem isso o orçamento acabou: há um conflito ali dentro, mas pode sobrar
    /// restrição inocente
    pub minimo: bool,
    /// o item não fecha nem com todas as vizinhas tiradas: o problema é ele com o que está fixo
    pub sozinho: bool,
    pub testes: usize,
}

/// Cópia de `a` sem os itens `fora`: restrições e juntas suprimidas, acoplamentos tirados. Acoplamento
/// de uma junta que saiu também sai, porque ele ainda mediria a junta suprimida.
fn sem(a: &Assembly, fora: &[Id]) -> Assembly {
    let mut c = a.clone();
    c.mates.iter_mut().filter(|m| fora.contains(&m.id)).for_each(|m| m.suppressed = true);
    c.joints.iter_mut().filter(|j| fora.contains(&j.id)).for_each(|j| j.suppressed = true);
    let fora_juntas: Vec<Id> = c.joints.iter().filter(|j| j.suppressed).map(|j| j.id).collect();
    c.couplings.retain(|k| !fora.contains(&k.id) && !fora_juntas.contains(&k.a) && !fora_juntas.contains(&k.b));
    c
}

/// Núcleo do conflito de `item`. `a` é a montagem com o item, nas posições de antes de resolver.
/// `ignorar` são as restrições que já falhavam sem ele: ficam de fora, porque o problema delas é outro.
///
/// Filtro por remoção: tenta tirar um pedaço das vizinhas; se o conflito continua, o pedaço era
/// inocente e sai de vez. Os pedaços vão diminuindo até uma por vez, e é essa última passada que
/// garante o mínimo. Só as do mesmo aglomerado entram: as outras não encostam no item.
pub fn nucleo(a: &Assembly, item: Id, ignorar: &[Id]) -> Nucleo {
    let base = sem(a, ignorar);
    let conflita = |fora: &[Id]| !solve::solve(&mut sem(&base, fora)).converged;
    if !conflita(&[]) {
        return Nucleo { minimo: true, testes: 1, ..Default::default() };
    }
    let mut suspeitas = vizinhos_de(&base, item);
    let todas = suspeitas.clone();
    if conflita(&todas) {
        return Nucleo { minimo: true, sozinho: true, testes: 2, ..Default::default() };
    }
    let mut gastos = 2;
    let mut fora: Vec<Id> = vec![];
    let mut pedaco = (suspeitas.len() / 2).max(1);
    let mut minimo = true;
    'filtro: loop {
        let mut i = 0;
        while i < suspeitas.len() {
            if gastos >= MAX_TESTES {
                minimo = false;
                break 'filtro;
            }
            let fim = (i + pedaco).min(suspeitas.len());
            let tentativa: Vec<Id> = fora.iter().chain(&suspeitas[i..fim]).copied().collect();
            gastos += 1;
            if conflita(&tentativa) {
                fora = tentativa;
                suspeitas.drain(i..fim);
            } else {
                i = fim;
            }
        }
        if pedaco == 1 {
            break;
        }
        pedaco = (pedaco / 2).max(1);
    }
    Nucleo { envolvidas: suspeitas, minimo, sozinho: false, testes: gastos }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn peca(a: &mut Assembly, nome: &str, z: f64) -> Id {
        a.add_component(nome, NodeKind::Part { path: "p".into() }, Placement { origin: [0., 0., z], ..Default::default() }, None)
    }

    fn plano(comp: Id, n: f64) -> MateRef {
        MateRef { comp, geom: Geom::Plane { p: [0.; 3], n: [0., 0., n] }, pick: None, viva: None }
    }

    /// Base fixa, placa presa a 10 mm dela e outras duas peças soltas ligadas à placa.
    fn base_e_placa() -> (Assembly, Id, Id, Id) {
        let mut a = Assembly::new("c");
        let base = peca(&mut a, "Base", 0.);
        a.node_mut(base).unwrap().fixed = true;
        let placa = peca(&mut a, "Placa", 10.);
        let d = a.add_mate(MateKind::Distance("10".into()), plano(base, 1.), plano(placa, 1.));
        a.mates.iter_mut().find(|m| m.id == d).unwrap().sentido = Sentido::Mesmo;
        (a, base, placa, d)
    }

    #[test]
    fn nucleo_aponta_as_duas_distancias_e_poupa_a_inocente() {
        let (mut a, base, placa, d) = base_e_placa();
        // inocente: orienta a placa em relação à base, sem tocar na altura
        let eixo = |c| MateRef { comp: c, geom: Geom::Axis { p: [0.; 3], d: [0., 0., 1.] }, pick: None, viva: None };
        let inocente = a.add_mate(MateKind::Coincident { flip: false }, eixo(base), eixo(placa));
        assert!(solve::solve(&mut a).converged);
        let nova = a.add_mate(MateKind::Distance("25".into()), plano(base, 1.), plano(placa, 1.));
        let n = nucleo(&a, nova, &[]);
        assert_eq!(n.envolvidas, vec![d], "{n:?}");
        assert!(n.minimo && !n.sozinho);
        assert!(!n.envolvidas.contains(&inocente));
    }

    #[test]
    fn restricao_de_outro_aglomerado_nem_entra_no_teste() {
        let (mut a, base, placa, d) = base_e_placa();
        let outra = peca(&mut a, "Outra", 50.);
        let longe = a.add_mate(MateKind::Distance("50".into()), plano(base, 1.), plano(outra, 1.));
        let nova = a.add_mate(MateKind::Distance("25".into()), plano(base, 1.), plano(placa, 1.));
        assert_eq!(solve::vizinhos_de(&a, nova), vec![d], "a base fixa não liga os dois lados");
        assert!(!nucleo(&a, nova, &[]).envolvidas.contains(&longe));
    }

    #[test]
    fn conflito_em_cadeia_traz_todo_o_caminho() {
        // base -10- p1 -10- p2, e a nova pede p2 a 5 da base: as três formam o conflito
        let (mut a, base, placa, d1) = base_e_placa();
        let p2 = peca(&mut a, "P2", 20.);
        let d2 = a.add_mate(MateKind::Distance("10".into()), plano(placa, 1.), plano(p2, 1.));
        a.mates.iter_mut().find(|m| m.id == d2).unwrap().sentido = Sentido::Mesmo;
        assert!(solve::solve(&mut a).converged);
        let nova = a.add_mate(MateKind::Distance("5".into()), plano(base, 1.), plano(p2, 1.));
        let mut n = nucleo(&a, nova, &[]).envolvidas;
        n.sort_by_key(|i| i.0);
        assert_eq!(n, vec![d1, d2]);
    }

    #[test]
    fn duas_pecas_fixas_nao_tem_vizinha_para_culpar() {
        let (mut a, base, placa, _) = base_e_placa();
        a.node_mut(placa).unwrap().fixed = true;
        let nova = a.add_mate(MateKind::Distance("99".into()), plano(base, 1.), plano(placa, 1.));
        let n = nucleo(&a, nova, &[]);
        assert!(n.sozinho && n.envolvidas.is_empty(), "{n:?}");
    }

    #[test]
    fn o_que_ja_falhava_nao_e_culpa_da_nova() {
        let (mut a, base, placa, d) = base_e_placa();
        let velha = a.add_mate(MateKind::Distance("30".into()), plano(base, 1.), plano(placa, 1.));
        let ja = solve::solve(&mut a.clone()).failed;
        assert!(ja.contains(&velha));
        let solta = peca(&mut a, "Solta", 70.);
        let nova = a.add_mate(MateKind::Distance("70".into()), plano(base, 1.), plano(solta, 1.));
        let n = nucleo(&a, nova, &ja);
        assert!(n.envolvidas.is_empty() && !n.sozinho, "sem conflito novo: {n:?} {d:?}");
    }

    #[test]
    fn nucleo_nao_mexe_na_montagem() {
        let (mut a, base, placa, _) = base_e_placa();
        let nova = a.add_mate(MateKind::Distance("25".into()), plano(base, 1.), plano(placa, 1.));
        let antes = a.clone();
        nucleo(&a, nova, &[]);
        assert_eq!(a, antes);
    }
}
