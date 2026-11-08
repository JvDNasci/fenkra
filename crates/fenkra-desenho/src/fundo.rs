// SPDX-License-Identifier: LGPL-2.1-or-later
//! B427: projeção das vistas de montagem fora da thread da interface, com progresso e cancelamento.
//!
//! Projetar o conjunto do kart levava mais de dois minutos dentro de um quadro: a janela congelava,
//! sem barra e sem como desistir. Aqui a vista de montagem (`.fka`) é calculada numa thread própria,
//! que tem o seu `Models` (a forma do OpenCASCADE não atravessa threads; só a `Proj`, que são
//! listas de números, volta). A interface recebe uma projeção vazia marcada `calculando` enquanto
//! isso e troca pela pronta quando chega.
//!
//! Regras:
//! - um pedido por vez na thread, em ordem de chegada; as demais vistas esperam na fila daqui;
//! - cancelar esvazia a fila e solta o pedido em andamento. A vista cancelada fica marcada até a
//!   pessoa pedir de novo ou editar a vista, para não ser recalculada a cada quadro;
//! - o cache da thread só guarda o que terminou: cancelado, nada fica pela metade;
//! - a chamada de projeção do OpenCASCADE (linhas ocultas) é uma só e não é interrompível: depois de
//!   cancelar, a thread termina essa chamada e descarta o resultado. A interface já está livre.

use crate::source::{Models, Proj};
use crate::{Id, View};
use fenkra_core::trabalho::{Controle, Fila};
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::time::SystemTime;

/// Fonte de uma vista: caminho completo e estado de exibição (a chave do cache de carga).
pub(crate) type Fonte = (PathBuf, String);
/// Arquivos de que um cálculo dependeu, com a data de cada um quando foi lido.
pub(crate) type Arquivos = Vec<(PathBuf, Option<SystemTime>)>;

pub(crate) struct Pedido {
    pub id: Id,
    pub vista: View,
    pub mae: Option<Proj>,
    pub dir: Option<PathBuf>,
}

pub(crate) struct Resposta {
    pub id: Id,
    pub proj: Proj,
    pub fonte: Fonte,
    pub arquivos: Arquivos,
    /// volume da forma da montagem (mm³), para os campos da legenda
    pub volume: f64,
    /// nome de cada peça da montagem, para a lista de peças do corte
    pub nomes: Vec<String>,
}

/// O que se mostra da fila na interface.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Andamento {
    /// passos feitos e total do pedido em andamento (0 = ainda sem contagem)
    pub feitos: u32,
    pub total: u32,
    /// vistas ainda esperando, fora a que está sendo calculada
    pub esperando: usize,
    /// há uma vista sendo calculada
    pub calculando: bool,
}

pub(crate) struct Fundo {
    fila: Fila<Pedido, Resposta>,
    espera: VecDeque<(Pedido, u64)>,
    andamento: Option<(Pedido, u64, Controle)>,
    /// vistas canceladas e a chave em que foram: mudar a vista (outra chave) as libera
    pub cancelados: HashMap<Id, u64>,
    /// arquivos e datas de cada fonte calculada em segundo plano
    pub arquivos: HashMap<Fonte, Arquivos>,
    /// volume e nomes das peças de cada fonte calculada em segundo plano
    pub extras: HashMap<Fonte, (f64, Vec<String>)>,
}

impl Fundo {
    pub fn novo(acordar: impl Fn() + Send + 'static) -> Self {
        let fila = Fila::nova(
            "projetar-vista",
            |_, novo| novo,
            || {
                fenkra_kernel::reduzir_prioridade_da_thread();
                let mut m = Models::new(None);
                Box::new(move |p: Pedido, ctl: &Controle| m.calcular_em_fundo(p, ctl))
            },
            acordar,
        );
        Fundo { fila, espera: VecDeque::new(), andamento: None, cancelados: HashMap::new(), arquivos: HashMap::new(), extras: HashMap::new() }
    }

    /// A vista já foi pedida (esperando ou em cálculo) com esta chave?
    pub fn pedida(&self, id: Id, chave: u64) -> bool {
        self.andamento.as_ref().is_some_and(|(p, k, _)| p.id == id && *k == chave) || self.espera.iter().any(|(p, k)| p.id == id && *k == chave)
    }

    /// Enfileira a vista. Pedido antigo da mesma vista (outra chave) sai da fila ou é cancelado.
    pub fn pedir(&mut self, p: Pedido, chave: u64) {
        self.espera.retain(|(x, _)| x.id != p.id);
        if self.andamento.as_ref().is_some_and(|(x, _, _)| x.id == p.id) {
            if let Some((_, _, ctl)) = self.andamento.take() {
                ctl.cancelar();
            }
        }
        self.espera.push_back((p, chave));
        self.seguir();
    }

    /// Manda o próximo da fila se a thread está livre.
    fn seguir(&mut self) {
        if self.andamento.is_some() {
            return;
        }
        if let Some((p, k)) = self.espera.pop_front() {
            let copia = Pedido { id: p.id, vista: p.vista.clone(), mae: p.mae.clone(), dir: p.dir.clone() };
            let ctl = self.fila.enviar(copia);
            self.andamento = Some((p, k, ctl));
        }
    }

    /// Resposta pronta, com a vista que a pediu, a mãe e a chave do pedido. Põe o próximo na thread.
    pub fn receber(&mut self) -> Option<(Resposta, Pedido, u64)> {
        let r = self.fila.receber()?;
        match self.andamento.take() {
            Some((p, k, _)) if p.id == r.id => {
                self.seguir();
                Some((r, p, k))
            }
            // resposta de um pedido já cancelado ou trocado: não vale
            outro => {
                self.andamento = outro;
                self.seguir();
                None
            }
        }
    }

    /// Cancela o que está em andamento e esvazia a fila; as vistas viram canceladas.
    pub fn cancelar_tudo(&mut self) {
        if let Some((p, k, ctl)) = self.andamento.take() {
            ctl.cancelar();
            self.cancelados.insert(p.id, k);
        }
        for (p, k) in self.espera.drain(..) {
            self.cancelados.insert(p.id, k);
        }
    }

    pub fn andamento(&self) -> Andamento {
        let (feitos, total) = self.andamento.as_ref().map_or((0, 0), |(_, _, c)| c.progresso());
        Andamento { feitos, total, esperando: self.espera.len(), calculando: self.andamento.is_some() }
    }

    pub fn ocupado(&self) -> bool {
        self.andamento.is_some() || !self.espera.is_empty()
    }
}

/// A parte da chave de cache de uma vista que não depende dos arquivos lidos.
pub(crate) fn chave_da_vista(v: &View, mae: Option<&Proj>, h: &mut impl std::hash::Hasher) {
    use std::hash::Hash;
    serde_json::to_string(&(&v.source, &v.kind, v.hidden_lines, v.tangent_edges, v.render, &v.corte, &v.detalhe)).unwrap_or_default().hash(h);
    if let Some(pp) = mae {
        format!("{:?}{:?}", pp.axes, pp.center).hash(h);
        pp.lines.len().hash(h);
    }
}
