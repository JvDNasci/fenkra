// SPDX-License-Identifier: LGPL-2.1-or-later
//! Composição da montagem fora da thread da interface (B125, B259).
//!
//! Ler as peças, calcular cada uma e tesselar levava segundos dentro de um quadro: a janela congelava
//! ao abrir uma montagem. A thread de composição tem o próprio `PartCache`, porque a forma do
//! OpenCASCADE não atravessa threads; o cache da interface continua servindo seleção e restrições.
//! O que a interface sabe e a thread não (peças só na memória, peças editadas) vai em cada pedido.

use fenkra_core::trabalho::{Controle, Fila};
use fenkra_core::{Document, Id, QualidadeMalha};
use fenkra_montagem::load::PartCache;
use fenkra_montagem::view::{self, Composta};
use fenkra_montagem::Assembly;
use std::path::PathBuf;

/// Um pedido de composição.
pub struct Pedido {
    /// número do pedido: resposta de número antigo é descartada
    pub geracao: u64,
    pub asm: Assembly,
    pub dir: Option<PathBuf>,
    pub qualidade: QualidadeMalha,
    /// peças que só existem na memória
    pub pendentes: Vec<(PathBuf, Document)>,
    /// peças que mudaram desde o pedido anterior
    pub esquecer: Vec<PathBuf>,
}

pub struct Resposta {
    pub geracao: u64,
    pub composta: Composta,
    pub erros: Vec<(Id, String)>,
    /// documentos das peças que esta composição leu: a árvore da interface usa sem tocar no disco
    pub documentos: Vec<(PathBuf, Document)>,
}

/// Junta dois pedidos: vale o mais novo, sem perder o que o antigo mandava esquecer.
fn fundir(antes: Pedido, mut novo: Pedido) -> Pedido {
    novo.esquecer.extend(antes.esquecer);
    novo
}

fn executar(cache: &mut PartCache, p: Pedido, ctl: &Controle) -> Option<Resposta> {
    cache.set_qualidade(p.qualidade);
    for f in &p.esquecer {
        cache.forget(f);
    }
    cache.sincronizar_pendentes(p.pendentes);
    cache.cancelar = Some(ctl.bandeira());
    let mut erros = vec![];
    let composta = crate::medir_quadros::fase("compor_fundo", || view::compor(&p.asm, p.dir.as_deref(), cache, &mut erros, Some(ctl)));
    cache.cancelar = None;
    let composta = composta?;
    Some(Resposta { geracao: p.geracao, composta, erros, documentos: cache.tirar_recem() })
}

/// Fila da composição: uma thread por montagem aberta.
pub struct ComporFundo {
    fila: Fila<Pedido, Resposta>,
}

impl ComporFundo {
    pub fn nova(acordar: impl Fn() + Send + 'static) -> Self {
        let fila = Fila::nova(
            "compor-montagem",
            fundir,
            || {
                fenkra_kernel::reduzir_prioridade_da_thread();
                let mut cache = PartCache::com_niveis();
                Box::new(move |p: Pedido, ctl: &Controle| executar(&mut cache, p, ctl))
            },
            acordar,
        );
        ComporFundo { fila }
    }

    pub fn enviar(&self, p: Pedido) {
        self.fila.enviar(p);
    }

    pub fn receber(&self) -> Option<Resposta> {
        self.fila.receber()
    }

    pub fn progresso(&self) -> Option<(u32, u32)> {
        self.fila.progresso()
    }
}
