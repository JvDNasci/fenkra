// SPDX-License-Identifier: LGPL-2.1-or-later
//! Carregamento das peças da montagem: lê o arquivo, calcula (com cache por data de modificação)
//! e entrega a malha e a forma para exibição, seleção e restrições.

use crate::lod::{self, Prototipo};
use crate::*;
use fenkra_core::{Document, QualidadeMalha};
use fenkra_kernel::Shape;
use fenkra_solidos::RecomputeResult;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::SystemTime;

/// Peça calculada.
pub struct LoadedPart {
    pub doc: Document,
    pub result: RecomputeResult,
    /// forma final (para achar faces e eixos); `None` no modo só visualização
    pub shape: Option<Shape>,
    /// malha da peça, compartilhada pelas cópias, nos níveis de detalhe gerados
    pub prototipo: Arc<Prototipo>,
    stamp: Option<SystemTime>,
}

/// Cache das peças lidas, por caminho completo.
#[derive(Default)]
pub struct PartCache {
    parts: HashMap<PathBuf, LoadedPart>,
    /// qualidade das malhas das peças (a mesma da peça aberta)
    qualidade: QualidadeMalha,
    /// peças criadas no contexto que ainda não têm arquivo (gravadas com a montagem)
    pendentes: HashMap<PathBuf, Document>,
    /// peças calculadas numa configuração da tabela de projeto, por caminho e nome (B102)
    configs: HashMap<(PathBuf, String), LoadedPart>,
    /// gera os níveis grossos e não guarda formas por conta própria: é o cache da thread de composição
    niveis: bool,
    /// bandeira do pedido em andamento: o cálculo da peça para e nada fica guardado pela metade
    pub cancelar: Option<Arc<AtomicBool>>,
    /// só precisa da forma (o desenho técnico projeta o sólido; não mostra malha): `forma` calcula
    /// os comandos sem tesselar nem montar os níveis de detalhe (B393)
    pub so_forma: bool,
    /// formas calculadas por `forma` no modo `so_forma`, com a data do arquivo de quando saíram
    formas: HashMap<PathBuf, (Option<SystemTime>, Option<Shape>)>,
    /// peças esquecidas desde a última vez que alguém perguntou
    esquecidas: Vec<PathBuf>,
    /// peças calculadas desde a última vez que alguém perguntou
    recem: Vec<PathBuf>,
    /// documentos lidos sem calcular: a árvore precisa da lista de comandos, não do sólido
    documentos: HashMap<PathBuf, (Option<SystemTime>, Document)>,
    /// protótipo de cada subconjunto e a assinatura do que entrou nele
    pub(crate) subconjuntos: HashMap<(PathBuf, String), (u64, crate::view::ComponenteVisto)>,
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
    /// Cache da thread de composição (B125): com níveis de detalhe e sem guardar formas à toa.
    pub fn com_niveis() -> Self {
        PartCache { niveis: true, ..Default::default() }
    }

    /// Este cache é o da thread de composição?
    pub fn gera_niveis(&self) -> bool {
        self.niveis
    }

    /// Calcula a peça e monta o protótipo. A troca de qualidade só re-tessela: os comandos ficam no
    /// cache do cálculo, então os níveis grossos custam só a malha.
    ///
    /// `com_forma`: devolve também a forma final, tirada do cache deste mesmo cálculo (B393). Antes
    /// a forma vinha de `final_shape`, que refazia todos os comandos: cada peça era calculada duas
    /// vezes. Sem a forma no cache (cálculo interrompido), cai em `final_shape`.
    fn calcular(&self, doc: &Document, com_forma: bool) -> Result<(RecomputeResult, Arc<Prototipo>, Option<fenkra_kernel::Shape>), String> {
        let mut c = fenkra_solidos::Cache::com_qualidade(self.qualidade);
        c.cancelar = self.cancelar.clone();
        let result = fenkra_solidos::recompute_cached(doc, &mut c);
        if result.cancelado {
            return Err("cancelado".into());
        }
        let forma = if com_forma { fenkra_solidos::forma_cache::forma_do_cache(doc, &c).or_else(|| fenkra_solidos::final_shape(doc)) } else { None };
        let mut malhas = vec![(crate::view::malha_unica(&result), self.qualidade)];
        if self.niveis {
            for q in lod::qualidades(self.qualidade).into_iter().skip(1) {
                c.qualidade = q;
                let r = fenkra_solidos::recompute_cached(doc, &mut c);
                if r.cancelado {
                    return Err("cancelado".into());
                }
                malhas.push((crate::view::malha_unica(&r), q));
            }
        }
        Ok((result, Prototipo::novo(malhas), forma))
    }

    /// Documento da peça, sem calcular nada: é o que a thread da interface pode pedir a cada quadro.
    pub fn documento(&mut self, full: &Path) -> Result<&Document, String> {
        let stamp = std::fs::metadata(full).and_then(|m| m.modified()).ok();
        if self.parts.get(full).is_some_and(|p| p.stamp == stamp) {
            return Ok(&self.parts[full].doc);
        }
        if self.pendentes.contains_key(full) {
            return Ok(&self.pendentes[full]);
        }
        if self.documentos.get(full).is_none_or(|(s, _)| *s != stamp) {
            let txt = std::fs::read_to_string(full).map_err(|e| format!("{}: {e}", full.display()))?;
            let doc = Document::from_json(&txt).map_err(|e| e.to_string())?;
            self.documentos.insert(full.to_path_buf(), (stamp, doc));
        }
        Ok(&self.documentos[full].1)
    }

    /// Documento que já está na memória, sem tocar no disco: serve à árvore, que redesenha todo quadro.
    pub fn documento_pronto(&self, full: &Path) -> Option<&Document> {
        self.parts.get(full).map(|p| &p.doc).or_else(|| self.pendentes.get(full)).or_else(|| self.documentos.get(full).map(|(_, d)| d))
    }

    /// Guarda um documento que outra thread já leu.
    pub fn semear_documento(&mut self, full: PathBuf, doc: Document) {
        self.documentos.insert(full, (None, doc));
    }

    /// Documentos das peças calculadas desde a última chamada.
    pub fn tirar_recem(&mut self) -> Vec<(PathBuf, Document)> {
        let lista = std::mem::take(&mut self.recem);
        lista.into_iter().filter_map(|f| self.parts.get(&f).map(|p| (f.clone(), p.doc.clone()))).collect()
    }

    /// Peças que só existem na memória, como a interface as vê agora.
    pub fn pendentes(&self) -> Vec<(PathBuf, Document)> {
        self.pendentes.iter().map(|(p, d)| (p.clone(), d.clone())).collect()
    }

    /// Acerta as peças da memória com a lista da interface; a que mudou é calculada de novo.
    pub fn sincronizar_pendentes(&mut self, lista: Vec<(PathBuf, Document)>) {
        let novas: HashMap<PathBuf, Document> = lista.into_iter().collect();
        for f in self.pendentes.keys().filter(|f| !novas.contains_key(*f)).cloned().collect::<Vec<_>>() {
            self.parts.remove(&f);
        }
        for (f, d) in &novas {
            if self.pendentes.get(f).is_none_or(|x| x.to_json() != d.to_json()) {
                self.parts.remove(f);
            }
        }
        self.pendentes = novas;
    }

    /// Peças esquecidas desde a última chamada: quem tem outro cache precisa esquecer também.
    pub fn tirar_esquecidas(&mut self) -> Vec<PathBuf> {
        std::mem::take(&mut self.esquecidas)
    }

    pub fn qualidade(&self) -> QualidadeMalha {
        self.qualidade
    }

    /// Troca a qualidade das malhas; as peças já lidas são descartadas para tesselar de novo.
    /// Devolve `true` se mudou.
    pub fn set_qualidade(&mut self, q: QualidadeMalha) -> bool {
        if q == self.qualidade {
            return false;
        }
        self.qualidade = q;
        self.parts.clear();
        self.configs.clear();
        true
    }

    /// Lê e calcula a peça (só de novo se o arquivo mudou). `with_shape`: guarda a forma
    /// (modo editável); no modo visualização só a malha fica na memória.
    pub fn get(&mut self, full: &Path, with_shape: bool) -> Result<&LoadedPart, String> {
        let stamp = std::fs::metadata(full).and_then(|m| m.modified()).ok();
        let fresh = match self.parts.get(full) {
            Some(p) => p.stamp != stamp || (with_shape && p.shape.is_none()),
            None => true,
        };
        if fresh {
            let doc = match self.pendentes.get(full) {
                Some(d) => d.clone(),
                None => {
                    let txt = std::fs::read_to_string(full).map_err(|e| format!("{}: {e}", full.display()))?;
                    Document::from_json(&txt).map_err(|e| e.to_string())?
                }
            };
            let (result, prototipo, shape) = self.calcular(&doc, with_shape)?;
            self.parts.insert(full.to_path_buf(), LoadedPart { doc, result, shape, prototipo, stamp });
            self.recem.push(full.to_path_buf());
        }
        Ok(self.parts.get(full).unwrap())
    }

    /// Como `get`, na configuração `cfg` da tabela de projeto da peça. `cfg` vazio = a ativa da própria
    /// peça (o caminho normal). Nome que a peça não tem é erro com o motivo e os nomes que existem: a
    /// montagem nunca cai em "a primeira" configuração.
    pub fn get_config(&mut self, full: &Path, with_shape: bool, cfg: &str) -> Result<&LoadedPart, String> {
        if cfg.is_empty() {
            return self.get(full, with_shape);
        }
        let stamp = std::fs::metadata(full).and_then(|m| m.modified()).ok();
        let chave = (full.to_path_buf(), cfg.to_string());
        let fresh = match self.configs.get(&chave) {
            Some(p) => p.stamp != stamp || (with_shape && p.shape.is_none()),
            None => true,
        };
        if fresh {
            let base = self.get(full, false)?.doc.clone();
            let doc = base.com_configuracao(cfg).map_err(|e| e.to_string())?;
            let (result, prototipo, shape) = self.calcular(&doc, with_shape)?;
            self.configs.insert(chave.clone(), LoadedPart { doc, result, shape, prototipo, stamp });
        }
        Ok(self.configs.get(&chave).unwrap())
    }

    /// Peça nova só na memória: o cache a entrega como se o arquivo existisse.
    pub fn definir_pendente(&mut self, full: &Path, doc: Document) {
        self.formas.remove(full);
        self.pendentes.insert(full.to_path_buf(), doc);
        self.parts.remove(full);
    }

    /// Devolve à leitura guardada o documento dado, sem recalcular: serve de "antes" na comparação.
    pub fn lembrar(&mut self, full: &Path, doc: Document) {
        let _ = self.get(full, false);
        if let Some(p) = self.parts.get_mut(full) {
            p.doc = doc;
        }
    }

    pub fn e_pendente(&self, full: &Path) -> bool {
        self.pendentes.contains_key(full)
    }

    /// Desiste da peça nova.
    pub fn descartar_pendente(&mut self, full: &Path) {
        self.pendentes.remove(full);
        self.parts.remove(full);
    }

    /// Grava as peças novas que a montagem usa, e esquece as outras: quem apagou o componente antes
    /// de salvar desistiu da peça, e ela não pode virar arquivo órfão. Devolve as gravadas. Se uma
    /// falhar, as que faltam continuam pendentes.
    pub fn gravar_pendentes(&mut self, a: &Assembly, dir: Option<&Path>) -> Result<Vec<PathBuf>, String> {
        let usados: Vec<PathBuf> = a.nodes.iter().filter_map(|n| match &n.kind {
            NodeKind::Part { path } => Some(full_path(dir, path)),
            _ => None,
        }).collect();
        self.pendentes.retain(|f, _| usados.contains(f));
        let mut feitos = vec![];
        for f in self.pendentes.keys().cloned().collect::<Vec<_>>() {
            let txt = self.pendentes[&f].to_json();
            std::fs::write(&f, txt).map_err(|e| format!("{}: {e}", f.display()))?;
            self.pendentes.remove(&f);
            self.parts.remove(&f);
            feitos.push(f);
        }
        Ok(feitos)
    }

    /// Forma final da peça. No modo `so_forma` ela sai só dos comandos, sem a malha que o `get`
    /// também calcula (num conjunto de 84 peças a tesselação era a maior parte da carga). Nos
    /// demais modos é a forma que o `get` já guarda.
    pub fn forma(&mut self, full: &Path) -> Result<Option<Shape>, String> {
        if !self.so_forma {
            return Ok(self.get(full, true)?.shape.clone());
        }
        let stamp = std::fs::metadata(full).and_then(|m| m.modified()).ok();
        if let Some((s, sh)) = self.formas.get(full) {
            if *s == stamp && !self.pendentes.contains_key(full) {
                return Ok(sh.clone());
            }
        }
        let doc = match self.pendentes.get(full) {
            Some(d) => d.clone(),
            None => {
                let txt = std::fs::read_to_string(full).map_err(|e| format!("{}: {e}", full.display()))?;
                Document::from_json(&txt).map_err(|e| e.to_string())?
            }
        };
        if self.cancelar.as_ref().is_some_and(|c| c.load(std::sync::atomic::Ordering::Relaxed)) {
            return Err("cancelado".into());
        }
        let sh = fenkra_solidos::final_shape(&doc);
        self.formas.insert(full.to_path_buf(), (stamp, sh.clone()));
        Ok(sh)
    }

    /// Esquece uma peça (para recarregar depois de editar).
    pub fn forget(&mut self, full: &Path) {
        self.formas.remove(full);
        self.parts.remove(full);
        self.configs.retain(|(p, _), _| p != full);
        self.esquecidas.push(full.to_path_buf());
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

/// Reencontra a geometria das restrições e juntas depois que a peça mudou.
pub fn refresh_refs(a: &mut Assembly, cache: &mut PartCache, asm_dir: Option<&Path>) {
    crate::releitura::reler_montagem(a, cache, asm_dir);
    crate::repeticao::sincronizar(a, cache, asm_dir);
}
