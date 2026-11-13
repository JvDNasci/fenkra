// SPDX-License-Identifier: LGPL-2.1-or-later
//! B427: projeção das vistas de montagem fora da thread da interface, com progresso e cancelamento.
//!
//! Projetar o conjunto do kart levava mais de dois minutos dentro de um quadro: a janela congelava,
//! sem barra e sem como desistir. Aqui a vista de montagem (`.fka`) é calculada numa thread própria,
//! que tem o seu `Models` (a forma do OpenCASCADE não atravessa threads; só a `Proj`, que são
//! listas de números, volta). A interface recebe uma projeção vazia marcada `calculando` enquanto
//! isso e troca pela pronta quando chega.
//!
//! B393: as vistas pedidas no mesmo quadro vão juntas, num lote: a montagem é carregada uma vez e
//! as linhas ocultas de cada vista rodam numa thread própria (`fenkra_kernel::hlr_varias`). Cada
//! lote passa por duas etapas: a prévia, projetada sobre a malha (segundos), e a exata, que a
//! substitui quando chega. O corte não tem prévia: a peça cortada seria refeita duas vezes.
//!
//! Regras:
//! - um lote por vez na thread; vistas pedidas enquanto ele roda esperam o próximo;
//! - cancelar esvazia a fila e solta o lote em andamento. A vista cancelada fica marcada até a
//!   pessoa pedir de novo ou editar a vista, para não ser recalculada a cada quadro;
//! - o cache da thread só guarda o que terminou: cancelado, nada fica pela metade;
//! - a chamada de projeção do OpenCASCADE (linhas ocultas) não é interrompível: depois de cancelar,
//!   a thread termina essas chamadas e descarta o resultado. A interface já está livre.

use crate::source::{Models, Proj};
use crate::{Id, View, ViewKind};
use fenkra_core::trabalho::{Controle, Fila};
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::time::SystemTime;

/// Fonte de uma vista: caminho completo e estado de exibição (a chave do cache de carga).
pub(crate) type Fonte = (PathBuf, String);
/// Arquivos de que um cálculo dependeu, com a data de cada um quando foi lido.
pub(crate) type Arquivos = Vec<(PathBuf, Option<SystemTime>)>;

#[derive(Clone)]
pub(crate) struct Pedido {
    pub id: Id,
    pub vista: View,
    pub mae: Option<Proj>,
    pub dir: Option<PathBuf>,
    /// a prévia por malha já chegou (ou a vista não tem prévia): falta só a exata
    pub sem_previa: bool,
}

/// Vistas calculadas juntas: a montagem carrega uma vez e as linhas ocultas rodam em paralelo.
pub(crate) struct Lote {
    pub pedidos: Vec<Pedido>,
    /// etapa da prévia (projeção da malha) em vez da exata
    pub previa: bool,
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
    /// vistas ainda esperando, fora as que estão sendo calculadas
    pub esperando: usize,
    /// há vistas sendo calculadas
    pub calculando: bool,
    /// as vistas em cálculo já mostram a prévia por malha; falta a projeção exata
    pub refinando: bool,
}

pub(crate) struct Fundo {
    fila: Fila<Lote, Vec<Resposta>>,
    espera: VecDeque<(Pedido, u64)>,
    andamento: Option<(Vec<(Pedido, u64)>, bool, Controle)>,
    /// vistas canceladas e a chave em que foram: mudar a vista (outra chave) as libera
    pub cancelados: HashMap<Id, u64>,
    /// arquivos e datas de cada fonte calculada em segundo plano
    pub arquivos: HashMap<Fonte, Arquivos>,
    /// volume e nomes das peças de cada fonte calculada em segundo plano
    pub extras: HashMap<Fonte, (f64, Vec<String>)>,
    /// liga a etapa da prévia por malha (desligada nos testes que conferem a projeção exata)
    pub com_previa: bool,
}

impl Fundo {
    pub fn novo(acordar: impl Fn() + Send + 'static) -> Self {
        let fila = Fila::nova(
            "projetar-vista",
            |_, novo| novo,
            || {
                fenkra_kernel::reduzir_prioridade_da_thread();
                let mut m = Models::new(None);
                Box::new(move |l: Lote, ctl: &Controle| m.calcular_lote(l, ctl))
            },
            acordar,
        );
        Fundo { fila, espera: VecDeque::new(), andamento: None, cancelados: HashMap::new(), arquivos: HashMap::new(), extras: HashMap::new(), com_previa: true }
    }

    /// A vista já foi pedida (esperando ou em cálculo) com esta chave?
    pub fn pedida(&self, id: Id, chave: u64) -> bool {
        self.andamento.as_ref().is_some_and(|(l, _, _)| l.iter().any(|(p, k)| p.id == id && *k == chave)) || self.espera.iter().any(|(p, k)| p.id == id && *k == chave)
    }

    /// A vista está na fila ou em cálculo, com qualquer chave?
    pub fn tem(&self, id: Id) -> bool {
        self.andamento.as_ref().is_some_and(|(l, _, _)| l.iter().any(|(p, _)| p.id == id)) || self.espera.iter().any(|(p, _)| p.id == id)
    }

    /// Enfileira a vista; vai à thread no próximo `seguir` (uma vez por quadro), junto com as
    /// outras pedidas no mesmo quadro. Pedido antigo da mesma vista sai da fila; um lote em
    /// andamento só com ela é cancelado.
    pub fn pedir(&mut self, mut p: Pedido, chave: u64) {
        // corte: a peça cortada seria refeita na prévia e de novo na exata
        if !self.com_previa || matches!(p.vista.kind, ViewKind::Section { .. }) {
            p.sem_previa = true;
        }
        self.espera.retain(|(x, _)| x.id != p.id);
        if self.andamento.as_ref().is_some_and(|(l, _, _)| l.iter().all(|(x, _)| x.id == p.id)) {
            if let Some((_, _, ctl)) = self.andamento.take() {
                ctl.cancelar();
            }
        }
        self.espera.push_back((p, chave));
    }

    /// Manda o próximo lote se a thread está livre: as vistas da fila que estão na mesma etapa
    /// da primeira (prévia ou exata).
    pub fn seguir(&mut self) {
        if self.andamento.is_some() || self.espera.is_empty() {
            return;
        }
        let previa = !self.espera[0].0.sem_previa;
        let (lote, resto): (Vec<_>, Vec<_>) = self.espera.drain(..).partition(|(p, _)| p.sem_previa != previa);
        self.espera = resto.into();
        let ctl = self.fila.enviar(Lote { pedidos: lote.iter().map(|(p, _)| p.clone()).collect(), previa });
        self.andamento = Some((lote, previa, ctl));
    }

    /// Respostas prontas, com a vista que as pediu, a chave do pedido e se é a prévia. A prévia
    /// devolve as vistas à frente da fila para a etapa exata.
    pub fn receber(&mut self) -> Vec<(Resposta, Pedido, u64, bool)> {
        let mut out = vec![];
        while let Some(rs) = self.fila.receber() {
            // resposta de um lote já cancelado ou trocado: não vale
            let vale = self.andamento.as_ref().is_some_and(|(l, _, _)| rs.len() == l.len() && rs.iter().zip(l).all(|(r, (p, _))| r.id == p.id));
            let Some((lote, previa, _)) = self.andamento.take_if(|_| vale) else { continue };
            for (r, (p, k)) in rs.into_iter().zip(lote) {
                // a prévia que já veio exata (do disco, B393) não volta à fila
                if previa && (r.proj.previa || r.proj.error.is_some()) {
                    let mut exata = p.clone();
                    exata.sem_previa = true;
                    // pedida de novo enquanto a prévia rodava: vale o pedido novo
                    if !self.espera.iter().any(|(x, _)| x.id == p.id) {
                        self.espera.push_front((exata, k));
                    }
                }
                out.push((r, p, k, previa));
            }
        }
        out
    }

    /// Cancela o que está em andamento e esvazia a fila; as vistas viram canceladas.
    pub fn cancelar_tudo(&mut self) {
        if let Some((l, _, ctl)) = self.andamento.take() {
            ctl.cancelar();
            for (p, k) in l {
                self.cancelados.insert(p.id, k);
            }
        }
        for (p, k) in self.espera.drain(..) {
            self.cancelados.insert(p.id, k);
        }
    }

    pub fn andamento(&self) -> Andamento {
        let (feitos, total) = self.andamento.as_ref().map_or((0, 0), |(_, _, c)| c.progresso());
        let refinando = self.andamento.as_ref().is_some_and(|(_, previa, _)| !previa) && self.espera.is_empty();
        Andamento { feitos, total, esperando: self.espera.len(), calculando: self.andamento.is_some(), refinando }
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
