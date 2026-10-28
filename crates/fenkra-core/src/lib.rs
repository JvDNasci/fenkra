//! Núcleo do Fenkra.
//!
//! - [`expr`]: avaliador de expressões dos parâmetros (RF-PAR-002/003).
//! - [`doc`]: documento de peça com árvore fixa (Origem, Referências, Parâmetros,
//!   coletores de corpos, pastas) e linha de tempo de comandos (RF-ARV).
//!
//! Princípio de edição não destrutiva (RF-NUC-020): todo objeto tem um [`Id`] estável;
//! editar altera o objeto no lugar e as referências apontam para o `Id`, nunca para posições.

pub mod doc;
pub mod expr;
pub mod macros;
pub mod pmi;
pub mod sketch;

pub use doc::*;
pub use sketch::SketchData;
