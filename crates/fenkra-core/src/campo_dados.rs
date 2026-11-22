// SPDX-License-Identifier: LGPL-2.1-or-later
//! ADR 0107, onda 0: dados que a caixa de seleção universal guarda por campo de um comando.
//!
//! Um campo de curvas (perfil, guia, seções) pode ter **junções na hora**: itens da lista que a
//! pessoa uniu pelo botão direito "unir". A junção mora aqui, junto do comando e do campo (por
//! `dono` e `campo`), como lista de índices dos itens: não existe recurso Junção escondido na árvore,
//! e `Document::delete` leva os dados do campo junto com o comando. O campo também guarda a opção
//! "seguir tangentes" (cadeia tangente como recurso do campo).
//!
//! A lista de itens continua no próprio comando (`curves`, `sections`...), na mesma ordem de sempre:
//! aqui ficam só os agrupamentos. Arquivo antigo não tem nada disso (`serde(default)`).

use crate::doc::Id;
use serde::{Deserialize, Serialize};

/// Dados de um campo de seleção de um comando.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DadosCampo {
    /// comando (feature ou referência) dono do campo
    pub dono: Id,
    /// identificador estável do campo no descritor (`Entrada::id`)
    pub campo: String,
    /// um clique numa curva já leva as vizinhas tangentes (o duplo clique faz o mesmo uma vez)
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub tangentes: bool,
    /// junções feitas na hora: cada grupo é a lista de índices (na lista do campo) dos itens unidos,
    /// em ordem. Grupo com menos de dois itens não existe.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub juncoes: Vec<Vec<usize>>,
}

impl DadosCampo {
    pub fn novo(dono: Id, campo: &str) -> Self {
        DadosCampo { dono, campo: campo.into(), tangentes: false, juncoes: vec![] }
    }

    /// Nada que gravar: o dado some do documento.
    pub fn vazio(&self) -> bool {
        !self.tangentes && self.juncoes.is_empty()
    }

    /// O item `i` pertence a uma junção? Devolve o índice do grupo.
    pub fn grupo_de(&self, i: usize) -> Option<usize> {
        self.juncoes.iter().position(|g| g.contains(&i))
    }

    /// Une os itens `itens` num grupo novo. Itens que já estavam em outra junção levam o grupo todo
    /// junto (a união de dois grupos é um grupo só). Devolve se alguma junção mudou.
    pub fn unir(&mut self, itens: &[usize]) -> bool {
        let mut novo: Vec<usize> = vec![];
        for i in itens {
            if !novo.contains(i) {
                novo.push(*i);
            }
        }
        // junções tocadas pela união entram inteiras
        let tocados: Vec<usize> = (0..self.juncoes.len()).filter(|g| self.juncoes[*g].iter().any(|x| novo.contains(x))).collect();
        for g in tocados.iter().rev() {
            for x in self.juncoes.remove(*g) {
                if !novo.contains(&x) {
                    novo.push(x);
                }
            }
        }
        if novo.len() < 2 {
            return false;
        }
        novo.sort_unstable();
        self.juncoes.push(novo);
        self.juncoes.sort();
        true
    }

    /// Desfaz as junções que contêm algum dos itens. Devolve se algo mudou.
    pub fn separar(&mut self, itens: &[usize]) -> bool {
        let antes = self.juncoes.len();
        self.juncoes.retain(|g| !g.iter().any(|x| itens.contains(x)));
        self.juncoes.len() != antes
    }

    /// A lista do campo mudou: `novo_indice[i]` é onde o item `i` antigo foi parar (`None` = saiu).
    /// Os grupos seguem os itens; o grupo que ficar com menos de dois itens se desfaz.
    pub fn remapear(&mut self, novo_indice: &[Option<usize>]) {
        for g in &mut self.juncoes {
            *g = g.iter().filter_map(|i| novo_indice.get(*i).copied().flatten()).collect();
            g.sort_unstable();
            g.dedup();
        }
        self.juncoes.retain(|g| g.len() >= 2);
        self.juncoes.sort();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unir_e_separar() {
        let mut d = DadosCampo::novo(Id(3), "perfil");
        assert!(d.unir(&[2, 0]));
        assert_eq!(d.juncoes, vec![vec![0, 2]]);
        assert_eq!(d.grupo_de(2), Some(0));
        assert!(!d.unir(&[1]), "um item sozinho não é junção");
        assert!(d.unir(&[2, 4]), "unir com item de outro grupo leva o grupo todo");
        assert_eq!(d.juncoes, vec![vec![0, 2, 4]]);
        assert!(d.separar(&[4]));
        assert!(d.vazio());
    }

    #[test]
    fn remapear_segue_os_itens() {
        let mut d = DadosCampo::novo(Id(3), "perfil");
        d.unir(&[1, 2, 3]);
        // o item 0 saiu: 1->0, 2->1, 3->2
        d.remapear(&[None, Some(0), Some(1), Some(2)]);
        assert_eq!(d.juncoes, vec![vec![0, 1, 2]]);
        // dois dos três saíram: o grupo desfaz
        d.remapear(&[Some(0), None, None]);
        assert!(d.juncoes.is_empty());
    }

    #[test]
    fn juncao_grava_no_comando_e_reabre_igual() {
        use crate::doc::{CurveRef, Document, OriginAxis, RefKind};
        let mut d = Document::new("t");
        let sem = d.to_json();
        assert!(!sem.contains("\"campos\""), "documento sem campos não grava nada novo (arquivo antigo abre igual)");
        let id = d.add_geometry(RefKind::CurveJoin { curves: vec![CurveRef::Axis(OriginAxis::X), CurveRef::Axis(OriginAxis::Y), CurveRef::Axis(OriginAxis::Z)] });
        let mut c = DadosCampo::novo(id, "g_cj");
        c.tangentes = true;
        c.unir(&[0, 2]);
        d.set_dados_campo(c.clone());
        let aberto = Document::from_json(&d.to_json()).expect("reabre");
        assert_eq!(aberto.dados_campo(id, "g_cj"), Some(&c), "a junção volta igual");
        assert_eq!(aberto.references, d.references, "a lista de itens do comando não mudou");
        // apagar o comando leva os dados do campo junto
        d.delete(id, false);
        assert!(d.campos.is_empty());
        // dado vazio não fica no arquivo
        d.set_dados_campo(DadosCampo::novo(id, "g_cj"));
        assert!(d.campos.is_empty());
    }
}
