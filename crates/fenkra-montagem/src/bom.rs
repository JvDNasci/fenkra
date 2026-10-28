//! Lista de peças por nível (só este nível, todas as peças, estruturada), com números de item
//! guardados na montagem: um item mantém o número entre revisões, mesmo que saia e volte, e os
//! novos recebem o próximo número livre.

use crate::load::full_path;
use crate::{Assembly, NodeKind};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Nível da lista.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum BomLevel {
    /// só os componentes diretos desta montagem (subconjuntos como um item)
    #[default]
    Topo,
    /// todas as peças de todos os níveis, somadas
    Pecas,
    /// árvore: subconjuntos com os itens deles logo abaixo (2, 2.1, 2.2…)
    Estruturada,
}

impl BomLevel {
    pub const ALL: [BomLevel; 3] = [BomLevel::Topo, BomLevel::Pecas, BomLevel::Estruturada];
    pub fn label(self) -> &'static str {
        match self {
            BomLevel::Topo => "Primeiro nível",
            BomLevel::Pecas => "Só peças (todos os níveis)",
            BomLevel::Estruturada => "Estruturada",
        }
    }
}

/// Número de item fixo de um componente (chave = caminho relativo à pasta da montagem).
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct ItemNo {
    pub key: String,
    pub no: u32,
}

#[derive(Clone, PartialEq, Debug)]
pub struct BomRow {
    /// "3" ou, na estruturada, "3.2"
    pub item: String,
    pub depth: usize,
    pub key: String,
    pub path: PathBuf,
    pub sub: bool,
    pub qty: u32,
    pub name: String,
}

/// Chave estável de um arquivo: relativo à pasta da montagem de cima, com "/".
fn key_of(top: Option<&Path>, p: &Path) -> String {
    let r = top.and_then(|d| p.strip_prefix(d).ok()).unwrap_or(p);
    r.to_string_lossy().replace('\\', "/")
}

fn load_sub(p: &Path) -> Option<Assembly> {
    std::fs::read_to_string(p).ok().and_then(|t| Assembly::from_json(&t).ok())
}

/// Componentes diretos: (arquivo, é subconjunto, quantidade, nome), na ordem da árvore.
fn direct(a: &Assembly, dir: Option<&Path>) -> Vec<(PathBuf, bool, u32, String)> {
    let mut out: Vec<(PathBuf, bool, u32, String)> = vec![];
    for n in &a.nodes {
        let (p, sub) = match &n.kind {
            NodeKind::Part { path } => (path, false),
            NodeKind::SubAssembly { path } => (path, true),
            NodeKind::Group => continue,
        };
        let fp = full_path(dir, p);
        match out.iter_mut().find(|r| r.0 == fp) {
            Some(r) => r.2 += 1,
            None => out.push((fp, sub, 1, n.name.clone())),
        }
    }
    out
}

/// Peças de todos os níveis, somadas: (arquivo, quantidade, nome).
fn flat(a: &Assembly, dir: Option<&Path>, mult: u32, depth: usize, out: &mut Vec<(PathBuf, u32, String)>) {
    if depth > 16 {
        return;
    }
    for (fp, sub, q, name) in direct(a, dir) {
        if sub {
            if let Some(s) = load_sub(&fp) {
                flat(&s, fp.parent(), mult * q, depth + 1, out);
            }
        } else {
            match out.iter_mut().find(|r| r.0 == fp) {
                Some(r) => r.1 += mult * q,
                None => out.push((fp, mult * q, name)),
            }
        }
    }
}

impl Assembly {
    /// Número fixo do item, se já tiver.
    pub fn item_of(&self, key: &str) -> Option<u32> {
        self.items.iter().find(|i| i.key == key).map(|i| i.no)
    }

    /// Numera os componentes que ainda não têm número (diretos primeiro, depois as peças dos
    /// subconjuntos). Os já numerados mantêm o número; números de itens que saíram não voltam
    /// a ser usados.
    pub fn assign_items(&mut self, dir: Option<&Path>) {
        let mut keys: Vec<String> = direct(self, dir).into_iter().map(|r| key_of(dir, &r.0)).collect();
        let mut f = vec![];
        flat(self, dir, 1, 0, &mut f);
        keys.extend(f.into_iter().map(|r| key_of(dir, &r.0)));
        for k in keys {
            if self.item_of(&k).is_none() {
                let no = self.items.iter().map(|i| i.no).max().unwrap_or(0) + 1;
                self.items.push(ItemNo { key: k, no });
            }
        }
    }
}

/// Número de cada chave: o guardado, ou (para quem ainda não tem) o próximo livre, na ordem.
fn numbering(a: &Assembly, keys: &[String]) -> Vec<u32> {
    let mut next = a.items.iter().map(|i| i.no).max().unwrap_or(0);
    let mut extra: Vec<(String, u32)> = vec![];
    keys.iter()
        .map(|k| match a.item_of(k).or_else(|| extra.iter().find(|e| e.0 == *k).map(|e| e.1)) {
            Some(n) => n,
            None => {
                next += 1;
                extra.push((k.clone(), next));
                next
            }
        })
        .collect()
}

/// Lista de peças no nível pedido, ordenada pelo número do item.
pub fn bom(a: &Assembly, dir: Option<&Path>, level: BomLevel) -> Vec<BomRow> {
    match level {
        BomLevel::Topo => {
            let d = direct(a, dir);
            let keys: Vec<String> = d.iter().map(|r| key_of(dir, &r.0)).collect();
            let nos = numbering(a, &keys);
            let mut rows: Vec<BomRow> = d
                .into_iter()
                .zip(keys)
                .zip(nos)
                .map(|(((path, sub, qty, name), key), no)| BomRow { item: no.to_string(), depth: 0, key, path, sub, qty, name })
                .collect();
            rows.sort_by_key(|r| r.item.parse::<u32>().unwrap_or(u32::MAX));
            rows
        }
        BomLevel::Pecas => {
            let mut f = vec![];
            flat(a, dir, 1, 0, &mut f);
            let keys: Vec<String> = f.iter().map(|r| key_of(dir, &r.0)).collect();
            let nos = numbering(a, &keys);
            let mut rows: Vec<BomRow> = f
                .into_iter()
                .zip(keys)
                .zip(nos)
                .map(|(((path, qty, name), key), no)| BomRow { item: no.to_string(), depth: 0, key, path, sub: false, qty, name })
                .collect();
            rows.sort_by_key(|r| r.item.parse::<u32>().unwrap_or(u32::MAX));
            rows
        }
        BomLevel::Estruturada => {
            let mut out = vec![];
            structured(a, dir, dir, "", 0, &mut out);
            out
        }
    }
}

fn structured(a: &Assembly, dir: Option<&Path>, top: Option<&Path>, prefix: &str, depth: usize, out: &mut Vec<BomRow>) {
    if depth > 16 {
        return;
    }
    // cada nível usa a numeração da própria montagem (o subconjunto numera os itens dele)
    let mut rows = bom(a, dir, BomLevel::Topo);
    for r in rows.iter_mut() {
        r.item = format!("{prefix}{}", r.item);
        r.depth = depth;
        r.key = key_of(top, &r.path);
    }
    for r in rows {
        let (sub, path, item) = (r.sub, r.path.clone(), r.item.clone());
        out.push(r);
        if sub {
            if let Some(s) = load_sub(&path) {
                structured(&s, path.parent(), top, &format!("{item}."), depth + 1, out);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Node;

    fn part(a: &mut Assembly, path: &str) {
        let id = a.alloc();
        a.nodes.push(Node { id, name: path.into(), number: 0, kind: NodeKind::Part { path: path.into() }, parent: None, placement: Default::default(), fixed: false, visible: true, mode: Default::default(), color: None });
    }

    #[test]
    fn numeros_de_item_ficam_entre_revisoes() {
        let mut a = Assembly::new("m");
        part(&mut a, "a.fkp");
        part(&mut a, "b.fkp");
        part(&mut a, "a.fkp");
        a.assign_items(None);
        let r = bom(&a, None, BomLevel::Topo);
        assert_eq!(r.iter().map(|x| (x.item.as_str(), x.qty)).collect::<Vec<_>>(), vec![("1", 2), ("2", 1)]);
        // revisão: a peça "a" sai, entra "c" — "b" continua 2 e "c" recebe 3 (o 1 não é reaproveitado)
        a.nodes.retain(|n| !matches!(&n.kind, NodeKind::Part { path } if path == "a.fkp"));
        part(&mut a, "c.fkp");
        let r = bom(&a, None, BomLevel::Topo);
        assert_eq!(r.iter().map(|x| (x.key.as_str(), x.item.as_str())).collect::<Vec<_>>(), vec![("b.fkp", "2"), ("c.fkp", "3")]);
        // a peça antiga volta com o número dela
        part(&mut a, "a.fkp");
        a.assign_items(None);
        let r = bom(&a, None, BomLevel::Topo);
        assert_eq!(r.first().map(|x| (x.key.as_str(), x.item.as_str())), Some(("a.fkp", "1")));
    }
}
