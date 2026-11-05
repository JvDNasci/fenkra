// SPDX-License-Identifier: LGPL-2.1-or-later
//! Núcleo do Fenkra.
//!
//! - [`expr`]: avaliador de expressões dos parâmetros (RF-PAR-002/003).
//! - [`doc`]: documento de peça com árvore fixa (Origem, Referências, Parâmetros,
//!   coletores de corpos, pastas) e linha de tempo de comandos (RF-ARV).
//!
//! Princípio de edição não destrutiva (RF-NUC-020): todo objeto tem um [`Id`] estável;
//! editar altera o objeto no lugar e as referências apontam para o `Id`, nunca para posições.

pub mod config;
pub mod doc;
pub mod expr;
pub mod formato;
pub mod macros;
pub mod materiais;
pub mod parametros;
pub mod pmi;
pub mod pmi_cotas;
pub mod pmi_feature;
pub mod pmi_leader;
pub mod pmi_infer;
pub mod pmi_snap;
pub mod projecao;
pub mod qualidade;
pub mod sketch;
pub mod subd;
pub mod subd_ops;
pub mod subd_brep;
pub mod trabalho;

pub use doc::*;
pub use qualidade::{PresetMalha, PresetStl, QualidadeMalha};
pub use sketch::SketchData;
