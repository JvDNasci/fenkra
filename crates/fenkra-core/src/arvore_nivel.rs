// SPDX-License-Identifier: LGPL-2.1-or-later
//! Nível de exibição na árvore (B557): o recurso gerado nasce um nível abaixo de quem o gerou.
//!
//! É só apresentação. A ordem de recálculo é a de `Document::features` e nunca muda por aqui; o que
//! este módulo decide é onde a linha aparece na árvore:
//!
//! - o **gerador** de um recurso é a primeira entrada dele que também é um comando da mesma linha do
//!   tempo (o sketch de uma extrusão, a extrusão de um furo derivado dela...), desde que venha antes
//!   na ordem e esteja na mesma pasta de comandos;
//! - por padrão o recurso aparece **dentro** do gerador (filho expansível);
//! - a pessoa pode tirá-lo dali, para o mesmo nível do gerador, ou pô-lo de volta. A escolha fica em
//!   `Document::soltos_do_gerador` (lista vazia = tudo como a regra manda). Arquivo antigo não tem
//!   o campo e abre com a regra nova: nenhum recurso muda de posição no cálculo.

use crate::*;

/// Um nó da hierarquia de exibição de um corpo: o recurso e o que aparece dentro dele.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NoExibicao {
    pub id: Id,
    pub filhos: Vec<NoExibicao>,
}

impl Document {
    fn pasta_de_comandos_de(&self, f: &Feature) -> Option<Id> {
        f.folder.filter(|p| self.folder(*p).is_some_and(|fo| fo.kind == FolderKind::Comandos))
    }

    /// Quem gerou o recurso `id`, segundo as entradas dele. `None` para sketch, para recurso sem
    /// entrada na mesma linha do tempo e para sketch livre.
    pub fn gerador_de(&self, id: Id) -> Option<Id> {
        let i = self.feature_index(id)?;
        let f = &self.features[i];
        if matches!(f.kind, FeatureKind::Sketch(_)) || self.is_free(f) {
            return None;
        }
        let pasta = self.pasta_de_comandos_de(f);
        Self::refs_of(&f.kind).into_iter().find(|x| {
            self.feature_index(*x).is_some_and(|j| {
                let g = &self.features[j];
                *x != id && j < i && !self.is_free(g) && g.target == f.target && self.pasta_de_comandos_de(g) == pasta
            })
        })
    }

    /// O recurso foi tirado de dentro do gerador pela pessoa?
    pub fn esta_fora_do_gerador(&self, id: Id) -> bool {
        self.soltos_do_gerador.contains(&id)
    }

    /// Pai do recurso na árvore: o gerador, enquanto ele não foi tirado de dentro dele.
    pub fn pai_na_arvore(&self, id: Id) -> Option<Id> {
        self.gerador_de(id).filter(|_| !self.esta_fora_do_gerador(id))
    }

    /// Filhos de `id` na árvore, na ordem do documento.
    pub fn filhos_na_arvore(&self, id: Id) -> Vec<Id> {
        self.features.iter().filter(|f| self.pai_na_arvore(f.id) == Some(id)).map(|f| f.id).collect()
    }

    /// Põe o recurso dentro do gerador (`fora = false`) ou no nível dele (`fora = true`). Não mexe na
    /// ordem de recálculo. Sem gerador não há o que gravar.
    pub fn definir_fora_do_gerador(&mut self, id: Id, fora: bool) {
        if fora && self.gerador_de(id).is_some() {
            if !self.soltos_do_gerador.contains(&id) {
                self.soltos_do_gerador.push(id);
            }
        } else {
            self.soltos_do_gerador.retain(|x| *x != id);
        }
    }

    /// Hierarquia de exibição da linha do tempo do corpo `corpo`: só as raízes, cada uma com os filhos
    /// dentro, tudo na ordem do documento. Sketch livre fica de fora (vive na geometria).
    pub fn hierarquia_exibicao(&self, corpo: Id) -> Vec<NoExibicao> {
        fn no(d: &Document, id: Id) -> NoExibicao {
            NoExibicao { id, filhos: d.filhos_na_arvore(id).into_iter().map(|c| no(d, c)).collect() }
        }
        self.features.iter().filter(|f| f.target == corpo && !self.is_free(f) && self.pai_na_arvore(f.id).is_none()).map(|f| no(self, f.id)).collect()
    }
}
