//! Contraste entre a peça, as arestas dela e o fundo (B425).
//!
//! A cor das arestas vem do tema e é uma só para a cena: escura no tema claro, clara no escuro. Peça
//! pintada na mesma luminância dessa cor (preta no tema claro, branca no escuro) perdia o contorno e
//! virava uma mancha. Aqui a aresta se adapta à cor do corpo: se a diferença de luminância é pequena,
//! a aresta passa para o lado oposto. O sombreamento (luz de preenchimento e contorno de silhueta
//! para cor de baixa luminância) mora no sombreador, em `render.rs`, com a mesma regra de luminância.

/// Luminância percebida (pesos de Rec. 709 sobre valores de tela 0..1).
pub fn luminancia(c: [f32; 3]) -> f32 {
    0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
}

/// Diferença mínima de luminância entre aresta e face para o contorno se ler.
pub const DIFERENCA_MIN: f32 = 0.30;

/// Cor da aresta para um corpo da cor `face`: a do tema, quando já contrasta; senão, do lado oposto
/// da luminância da face (clareada para face escura, escurecida para face clara).
pub fn aresta_para(aresta: [f32; 4], face: [f32; 3]) -> [f32; 4] {
    let lf = luminancia(face);
    if (luminancia([aresta[0], aresta[1], aresta[2]]) - lf).abs() >= DIFERENCA_MIN {
        return aresta;
    }
    let alvo = if lf < 0.5 { [0.88, 0.90, 0.94] } else { [0.05, 0.06, 0.08] };
    [alvo[0], alvo[1], alvo[2], aresta[3]]
}

#[cfg(test)]
mod testes {
    use super::*;

    const PRETO: [f32; 3] = [0.03, 0.03, 0.03];
    const BRANCO: [f32; 3] = [0.97, 0.97, 0.97];

    fn dif(a: [f32; 4], f: [f32; 3]) -> f32 {
        (luminancia([a[0], a[1], a[2]]) - luminancia(f)).abs()
    }

    #[test]
    fn peca_preta_e_branca_ganham_aresta_legivel_com_aresta_escura_ou_clara() {
        for aresta in [[0.06, 0.07, 0.09, 1.0], [0.85, 0.88, 0.92, 1.0]] {
            for face in [PRETO, BRANCO] {
                let a = aresta_para(aresta, face);
                assert!(dif(a, face) >= DIFERENCA_MIN, "aresta {aresta:?} sobre {face:?} deu {a:?}");
            }
        }
    }

    #[test]
    fn aresta_do_tema_que_ja_contrasta_nao_muda() {
        let escura = [0.06, 0.07, 0.09, 1.0];
        assert_eq!(aresta_para(escura, [0.55, 0.60, 0.70]), escura);
    }

    #[test]
    fn cada_tema_com_peca_preta_e_branca_mantem_contorno() {
        for t in crate::theme::builtin() {
            let aresta = t.aresta.map(|a| [a[0] as f32 / 255., a[1] as f32 / 255., a[2] as f32 / 255., 1.0]).unwrap_or([0.06, 0.07, 0.09, 1.0]);
            for face in [PRETO, BRANCO] {
                assert!(dif(aresta_para(aresta, face), face) >= DIFERENCA_MIN, "tema {} face {face:?}", t.name);
            }
        }
    }
}
