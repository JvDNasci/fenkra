// SPDX-License-Identifier: LGPL-2.1-or-later
//! Núcleo do Fenkra.
//!
//! - [`expr`]: avaliador de expressões dos parâmetros (RF-PAR-002/003).
//! - [`doc`]: documento de peça com árvore fixa (Origem, Referências, Parâmetros,
//!   coletores de corpos, pastas) e linha de tempo de comandos (RF-ARV).
//!
//! Princípio de edição não destrutiva (RF-NUC-020): todo objeto tem um [`Id`] estável;
//! editar altera o objeto no lugar e as referências apontam para o `Id`, nunca para posições.

pub mod aba_unif;
pub mod arvore_soltura;
pub mod config;
pub mod configuracoes;
pub mod doc;
pub mod edicao_direta;
pub mod estilo_chapa;
pub mod expr;
pub mod formato;
pub mod macros;
pub mod malha;
pub mod materiais;
pub mod parametros;
pub mod gdt;
pub mod pmi;
pub mod pmi_acabamento;
pub mod pmi_alvos;
pub mod pmi_baloes;
pub mod pmi_cotas;
pub mod pmi_feature;
pub mod pmi_leader;
pub mod pmi_infer;
pub mod pmi_snap;
pub mod projecao;
pub mod vinculo;
pub mod qualidade;
pub mod regras;
pub mod requisitos;
pub mod sketch;
pub mod subd;
pub mod subd_ops;
pub mod subd_brep;
pub mod subd_sel;
pub mod subd_vinculo;
pub mod trabalho;

pub use doc::*;
pub use arvore_soltura::RecusaSoltura;
pub use edicao_direta::{DirectEdit, OpDireta};
pub use regras::{EstiloRegras, Origem};
pub use qualidade::{Polimento, PresetMalha, PresetStl, QualidadeMalha};
pub use sketch::SketchData;
