// SPDX-License-Identifier: LGPL-2.1-or-later
//! B405: duas anotações presas à mesma âncora (a legenda e a nota, por exemplo) caem uma sobre a
//! outra na tela e nenhuma se lê. Aqui se calcula, em pixels, quanto afastar cada rótulo para que
//! os retângulos deixem de se cobrir: o rótulo que vem depois desce até o fim do anterior.
//! O deslocamento é só de tela (o dado da anotação não muda) e é recalculado a cada quadro a partir
//! dos retângulos do quadro anterior sem o deslocamento, para não oscilar.

use eframe::egui::{Rect, Vec2};
use std::collections::HashMap;

/// Vão entre dois rótulos afastados, em pixels.
const VAO: f32 = 3.;

/// Rótulo medido: id da anotação, retângulo na tela já com o deslocamento aplicado, e se pode
/// ser afastado (texto de frente para a tela; o deitado num plano e as cotas ficam onde estão).
#[derive(Clone, Copy, Debug)]
pub struct Rotulo {
    pub id: u64,
    pub rect: Rect,
    pub movel: bool,
}

/// Deslocamentos guardados de um quadro para o outro.
#[derive(Clone, Default)]
pub struct Memoria {
    pub desloc: HashMap<u64, Vec2>,
}

fn cobre(a: Rect, b: Rect) -> bool {
    a.shrink(0.5).intersects(b.shrink(0.5))
}

/// Deslocamento vertical de cada rótulo móvel para que nenhum cubra outro. Os fixos entram
/// primeiro e não se mexem; os móveis são tratados na ordem do id (o mais antigo fica no lugar).
/// `rotulos` traz os retângulos SEM deslocamento. Só aparecem no resultado os que mudaram de lugar.
pub fn afastar(rotulos: &[Rotulo]) -> HashMap<u64, Vec2> {
    let mut colocados: Vec<Rect> = rotulos.iter().filter(|r| !r.movel).map(|r| r.rect).collect();
    let mut moveis: Vec<&Rotulo> = rotulos.iter().filter(|r| r.movel).collect();
    moveis.sort_by_key(|r| r.id);
    let mut out = HashMap::new();
    for r in moveis {
        let mut dy = 0.;
        // desce até não cobrir ninguém; cada passo salta o fim do rótulo que atrapalha
        for _ in 0..rotulos.len() + 1 {
            let atual = r.rect.translate(Vec2::new(0., dy));
            match colocados.iter().filter(|c| cobre(**c, atual)).map(|c| c.max.y).reduce(f32::max) {
                Some(fim) => dy += fim + VAO - atual.min.y,
                None => break,
            }
        }
        if dy > 0. {
            out.insert(r.id, Vec2::new(0., dy));
        }
        colocados.push(r.rect.translate(Vec2::new(0., dy)));
    }
    out
}

/// Atualiza a memória com os rótulos do quadro (retângulos já deslocados pelo que estava na
/// memória). Devolve quantos estão afastados agora.
pub fn atualizar(mem: &mut Memoria, quadro: &[Rotulo]) -> usize {
    let base: Vec<Rotulo> = quadro
        .iter()
        .map(|r| {
            let d = mem.desloc.get(&r.id).copied().unwrap_or(Vec2::ZERO);
            Rotulo { rect: r.rect.translate(-d), ..*r }
        })
        .collect();
    mem.desloc = afastar(&base);
    mem.desloc.len()
}

/// Rótulos que continuam cobrindo outro depois do afastamento (texto deitado num plano, cota):
/// a interface avisa neles, em vez de deixar a pessoa descobrir. Devolve o id do que vem depois.
pub fn ainda_cobertos(quadro: &[Rotulo]) -> Vec<(u64, Rect)> {
    let mut ordem: Vec<&Rotulo> = quadro.iter().collect();
    ordem.sort_by_key(|r| r.id);
    let mut out = vec![];
    for (i, b) in ordem.iter().enumerate() {
        if !b.movel && ordem[..i].iter().any(|a| !a.movel && cobre(a.rect, b.rect)) {
            out.push((b.id, b.rect));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::pos2;

    fn r(id: u64, x: f32, y: f32, movel: bool) -> Rotulo {
        Rotulo { id, rect: Rect::from_min_size(pos2(x, y), Vec2::new(80., 14.)), movel }
    }

    #[test]
    fn dois_rotulos_na_mesma_ancora_deixam_de_se_cobrir() {
        let rs = [r(1, 100., 100., true), r(2, 100., 100., true)];
        let d = afastar(&rs);
        assert!(!d.contains_key(&1), "o mais antigo fica");
        let novo = rs[1].rect.translate(d[&2]);
        assert!(!cobre(rs[0].rect, novo));
        assert!(novo.min.y >= rs[0].rect.max.y);
    }

    #[test]
    fn tres_empilham_e_o_que_ja_estava_livre_nao_se_mexe() {
        let rs = [r(1, 100., 100., true), r(2, 100., 105., true), r(3, 100., 108., true), r(4, 300., 300., true)];
        let d = afastar(&rs);
        let fim: Vec<Rect> = rs.iter().map(|x| x.rect.translate(d.get(&x.id).copied().unwrap_or(Vec2::ZERO))).collect();
        for i in 0..4 {
            for j in i + 1..4 {
                assert!(!cobre(fim[i], fim[j]), "{i} e {j} ainda se cobrem");
            }
        }
        assert!(!d.contains_key(&4));
    }

    #[test]
    fn rotulo_fixo_nao_sai_do_lugar_e_o_movel_desvia_dele() {
        let rs = [r(1, 100., 100., false), r(2, 100., 100., true)];
        let d = afastar(&rs);
        assert!(!d.contains_key(&1) && d.contains_key(&2));
        let rs = [r(1, 100., 100., false), r(2, 100., 100., false)];
        assert!(afastar(&rs).is_empty(), "dois fixos: nada a fazer");
    }

    #[test]
    fn fixos_que_se_cobrem_sao_avisados_e_os_afastados_nao() {
        let fixos = [r(1, 100., 100., false), r(2, 110., 102., false), r(3, 400., 400., false)];
        let av = ainda_cobertos(&fixos);
        assert_eq!(av.iter().map(|x| x.0).collect::<Vec<_>>(), vec![2]);
        // móveis afastados: depois do deslocamento ninguém se cobre
        let moveis = [r(1, 100., 100., true), r(2, 100., 100., true)];
        let d = afastar(&moveis);
        let depois: Vec<Rotulo> = moveis.iter().map(|m| Rotulo { rect: m.rect.translate(d.get(&m.id).copied().unwrap_or(Vec2::ZERO)), ..*m }).collect();
        assert!(ainda_cobertos(&depois).is_empty());
    }

    #[test]
    fn a_memoria_e_estavel_quadro_a_quadro() {
        let mut mem = Memoria::default();
        let q1 = [r(1, 100., 100., true), r(2, 100., 100., true)];
        assert_eq!(atualizar(&mut mem, &q1), 1);
        // o quadro seguinte já vem com o deslocamento aplicado: o resultado não muda
        let q2 = [q1[0], Rotulo { rect: q1[1].rect.translate(mem.desloc[&2]), ..q1[1] }];
        let antes = mem.desloc[&2];
        assert_eq!(atualizar(&mut mem, &q2), 1);
        assert_eq!(mem.desloc[&2], antes);
        // separadas de novo (a pessoa afastou a âncora): o deslocamento some
        let q3 = [q1[0], r(2, 100., 300., true)];
        assert_eq!(atualizar(&mut mem, &q3), 0);
    }
}
