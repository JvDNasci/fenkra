//! Carregamento das peças da montagem: lê o arquivo, calcula (com cache por data de modificação)
//! e entrega a malha e a forma para exibição, seleção e restrições.

use crate::*;
use scc_core::Document;
use scc_kernel::Shape;
use scc_solidos::RecomputeResult;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Peça calculada.
pub struct LoadedPart {
    pub doc: Document,
    pub result: RecomputeResult,
    /// forma final (para achar faces e eixos); `None` no modo só visualização
    pub shape: Option<Shape>,
    stamp: Option<SystemTime>,
}

/// Cache das peças lidas, por caminho completo.
#[derive(Default)]
pub struct PartCache {
    parts: HashMap<PathBuf, LoadedPart>,
}

/// Caminho completo de um componente (relativo à pasta da montagem ou absoluto).
pub fn full_path(asm_dir: Option<&Path>, path: &str) -> PathBuf {
    let p = Path::new(path);
    match (asm_dir, p.is_relative()) {
        (Some(d), true) => d.join(p),
        _ => p.to_path_buf(),
    }
}

impl PartCache {
    /// Lê e calcula a peça (só de novo se o arquivo mudou). `with_shape`: guarda a forma
    /// (modo editável); no modo visualização só a malha fica na memória.
    pub fn get(&mut self, full: &Path, with_shape: bool) -> Result<&LoadedPart, String> {
        let stamp = std::fs::metadata(full).and_then(|m| m.modified()).ok();
        let fresh = match self.parts.get(full) {
            Some(p) => p.stamp != stamp || (with_shape && p.shape.is_none()),
            None => true,
        };
        if fresh {
            let txt = std::fs::read_to_string(full).map_err(|e| format!("{}: {e}", full.display()))?;
            let doc = Document::from_json(&txt).map_err(|e| e.to_string())?;
            let result = scc_solidos::recompute(&doc);
            let shape = if with_shape { scc_solidos::final_shape(&doc) } else { None };
            self.parts.insert(full.to_path_buf(), LoadedPart { doc, result, shape, stamp });
        }
        Ok(self.parts.get(full).unwrap())
    }

    /// Esquece uma peça (para recarregar depois de editar).
    pub fn forget(&mut self, full: &Path) {
        self.parts.remove(full);
    }

    /// Peça já carregada (sem ler o disco).
    pub fn peek(&self, full: &Path) -> Option<&LoadedPart> {
        self.parts.get(full)
    }
}

/// Elemento geométrico no ponto local clicado de uma peça: face plana → plano; face cilíndrica ou
/// cônica → eixo; demais faces → plano tangente.
pub fn geom_at(shape: &Shape, local: V3) -> Result<Geom, String> {
    let fp = shape.face_props(local).map_err(|e| e.0)?;
    let (q, n) = shape.surface_near(local).map_err(|e| e.0)?;
    Ok(match fp.kind {
        1 | 2 => Geom::Axis { p: fp.origin, d: unit(fp.dir) },
        _ => Geom::Plane { p: q, n: unit(n) },
    })
}

/// Reencontra a geometria das restrições depois que a peça mudou (pelo ponto clicado).
pub fn refresh_refs(a: &mut Assembly, cache: &mut PartCache, asm_dir: Option<&Path>) {
    let paths: HashMap<Id, PathBuf> = a
        .nodes
        .iter()
        .filter_map(|n| match &n.kind {
            NodeKind::Part { path } => Some((n.id, full_path(asm_dir, path))),
            _ => None,
        })
        .collect();
    for m in &mut a.mates {
        for r in [&mut m.a, &mut m.b] {
            let (Some(pick), Some(path)) = (r.pick, paths.get(&r.comp)) else { continue };
            let Ok(p) = cache.get(path, true) else { continue };
            if let Some(sh) = &p.shape {
                if let Ok(g) = geom_at(sh, pick) {
                    // mantém o tipo escolhido (um ponto continua ponto)
                    if std::mem::discriminant(&g) == std::mem::discriminant(&r.geom) {
                        r.geom = g;
                    }
                }
            }
        }
    }
}
