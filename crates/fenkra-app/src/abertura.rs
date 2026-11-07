// SPDX-License-Identifier: LGPL-2.1-or-later
//! Abrir uma peça: em qual bancada ela aparece e quando a câmera a enquadra (B423).

use crate::settings::Bench;
use fenkra_core::Document;

/// Bancada em que o conteúdo do documento abre. Chapa dobrada abre na bancada de chapa; todo o
/// resto, inclusive peças feitas de superfícies e de forma livre, abre em Modelagem Sólida: é lá
/// que o corpo aparece inteiro e que a árvore tem todos os comandos. Antes a peça abria na bancada
/// que estivesse ativa (por exemplo Desenho ou Forma livre) e a pessoa via uma tela vazia.
pub fn bancada_do_conteudo(d: &Document) -> Bench {
    if d.sheet.is_some() {
        Bench::Chapa
    } else {
        Bench::Solidos
    }
}

/// Estado do enquadramento ao abrir. A câmera só pode enquadrar depois que a cena NOVA subiu à
/// placa de vídeo: enquadrar na hora usava a caixa da peça que acabou de sair, e a peça nova
/// aparecia grande, pequena ou cortada.
#[derive(Default, Debug)]
pub struct Enquadrar {
    /// aguardando o resultado do recálculo da peça recém-aberta
    pub ao_resultar: bool,
    /// o resultado chegou: enquadrar assim que a cena subir
    pub apos_cena: bool,
}

impl Enquadrar {
    /// Peça aberta: enquadra quando o primeiro cálculo dela chegar.
    pub fn peca_aberta(&mut self) {
        self.ao_resultar = true;
    }

    /// Chegou um resultado. `atual` diz se é do último pedido enviado (o da peça aberta).
    pub fn resultado(&mut self, atual: bool) {
        if atual && self.ao_resultar {
            self.ao_resultar = false;
            self.apos_cena = true;
        }
    }

    /// Pede o enquadramento para depois da próxima subida de cena.
    pub fn depois_da_cena(&mut self) {
        self.apos_cena = true;
    }

    /// A cena subiu: devolve true uma vez se é hora de enquadrar.
    pub fn cena_subiu(&mut self) -> bool {
        std::mem::take(&mut self.apos_cena)
    }
}

/// Alvo e distância com que uma vista nomeada abre. A vista guarda a câmera do dia em que foi
/// salva; se a peça cresceu, encolheu ou mudou de lugar desde então, a vista abria cortando a peça
/// ou mostrando um ponto. Nesses casos mantém os ângulos da vista e enquadra a peça; vista que já
/// mostra a peça inteira, em tamanho razoável, abre exatamente como foi salva.
pub fn vista_enquadrada(alvo: glam::Vec3, dist: f32, caixa: Option<(glam::Vec3, glam::Vec3)>) -> (glam::Vec3, f32) {
    let Some((lo, hi)) = caixa else { return (alvo, dist) };
    let centro = (lo + hi) / 2.;
    let diag = (hi - lo).length().max(1e-3);
    let ideal = (diag * 1.6).max(20.);
    let corta = dist < ideal * 0.6;
    let minuscula = dist > ideal * 3.;
    let longe = (alvo - centro).length() > diag * 0.5;
    if corta || minuscula || longe {
        (centro, ideal)
    } else {
        (alvo, dist)
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn chapa_abre_na_bancada_de_chapa_e_o_resto_em_solidos() {
        let d = Document::new("Peça");
        assert_eq!(bancada_do_conteudo(&d), Bench::Solidos);
        let mut c = Document::new("Chapa");
        c.sheet = Some(Default::default());
        assert_eq!(bancada_do_conteudo(&c), Bench::Chapa);
    }

    #[test]
    fn vista_nomeada_enquadra_quando_corta_ou_some_e_respeita_quando_serve() {
        use glam::Vec3;
        let caixa = Some((Vec3::splat(-50.), Vec3::splat(50.)));
        let ideal = (Vec3::splat(100.).length() * 1.6).max(20.);
        // já mostra a peça inteira: abre como foi salva
        assert_eq!(vista_enquadrada(Vec3::new(5., 0., 0.), ideal, caixa), (Vec3::new(5., 0., 0.), ideal));
        // salva de muito perto: corta a peça
        let (a, d) = vista_enquadrada(Vec3::ZERO, ideal * 0.2, caixa);
        assert_eq!((a, d), (Vec3::ZERO, ideal));
        // peça mudou de lugar: o alvo salvo ficou longe dela
        let longe = Some((Vec3::splat(950.), Vec3::splat(1050.)));
        let (a, _) = vista_enquadrada(Vec3::ZERO, ideal, longe);
        assert_eq!(a, Vec3::splat(1000.));
        // sem caixa (cena vazia): não mexe
        assert_eq!(vista_enquadrada(Vec3::X, 7., None), (Vec3::X, 7.));
    }

    #[test]
    fn enquadra_so_depois_do_resultado_da_peca_nova_e_da_cena() {
        let mut e = Enquadrar::default();
        e.peca_aberta();
        // resultado velho (de outro pedido) não dispara
        e.resultado(false);
        assert!(!e.cena_subiu());
        e.resultado(true);
        assert!(e.cena_subiu(), "com o resultado novo, a cena seguinte enquadra");
        assert!(!e.cena_subiu(), "enquadra uma vez só");
    }
}
