// SPDX-License-Identifier: LGPL-2.1-or-later
//! Árvore do produto como grafo (B364): montagens, subconjuntos e peças do projeto ligados pelo que
//! cada montagem usa, com PN, nome e estado de cada nó.
//!
//! Como funciona:
//! 1. as montagens (`.fka`) moram soltas nas pastas do projeto; as peças moram no índice. Para não
//!    abrir nem recalcular nada, a árvore sai só do JSON: o cabeçalho de cada arquivo (nome, PN, uid)
//!    e as referências `Part`/`SubAssembly` dos nós de cada montagem (`relacoes::referencias_de`);
//! 2. cada arquivo vira **um** nó, não uma ocorrência: a mesma peça usada em três montagens é um nó
//!    só, com três pais. As arestas guardam a quantidade (quantas vezes o pai usa o filho). Assim o
//!    grafo de um conjunto grande cresce com o número de arquivos, e não com o de instâncias;
//! 3. o nó de peça casa com o documento do índice pelo arquivo ou, se o arquivo foi movido, pelo uid
//!    gravado dentro dele. Peça do índice dá estado, revisão e o hash da versão atual, que é a chave
//!    do pacote `.fkv` de onde sai a miniatura;
//! 4. montagem não está no índice e não tem estado próprio: o estado dela é **derivado** dos
//!    descendentes. A regra de cor é uma só, [`ArvoreProduto::liberado`]: peça liberada, ou montagem
//!    cujas peças estão todas liberadas;
//! 5. a assinatura de uma montagem junta o conteúdo do arquivo e as assinaturas dos filhos. Muda
//!    quando qualquer peça de baixo muda, então o pacote de visualização nomeado por ela nunca fica
//!    velho (o mesmo princípio dos pacotes por versão de `visualizacao`);
//! 6. referência circular (A usa B que usa A) não trava nada: quem percorre guarda o caminho e para
//!    no nó que já está nele.

use crate::relacoes::{self, TipoRef};
use crate::{DocState, Project};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Quantos níveis de pasta a busca de montagens desce.
const FUNDO: u32 = 6;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TipoNo {
    Peca,
    Montagem,
}

/// Estado do nó para a regra de cor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EstadoNo {
    /// peça do índice: o estado do ciclo de vida
    Documento(DocState),
    /// montagem: quantas peças distintas de baixo estão liberadas, de quantas
    Derivado { liberadas: usize, total: usize },
    /// arquivo fora do índice do projeto (ou que não existe mais)
    Fora,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Aresta {
    pub filho: usize,
    /// quantas vezes o pai usa o filho
    pub quantidade: u32,
}

#[derive(Clone, Debug)]
pub struct NoProduto {
    pub arquivo: PathBuf,
    pub tipo: TipoNo,
    pub nome: String,
    pub pn: String,
    /// documento do índice, quando é peça do projeto
    pub id: Option<u64>,
    pub estado: EstadoNo,
    pub revisao: Option<String>,
    /// hash da versão atual (peça do índice): a chave do pacote `.fkv` dela
    pub versao: Option<String>,
    pub existe: bool,
    pub filhos: Vec<Aresta>,
    /// montagens que usam este nó
    pub pais: Vec<usize>,
    /// conteúdo do nó e do que está abaixo dele (16 hexadecimais)
    pub assinatura: String,
}

#[derive(Clone, Debug, Default)]
pub struct ArvoreProduto {
    pub nos: Vec<NoProduto>,
    /// montagens de topo (ninguém as usa) e, depois, as peças do índice que nenhuma montagem usa
    pub raizes: Vec<usize>,
}

fn chave(p: &Path) -> PathBuf {
    std::fs::canonicalize(p).unwrap_or_else(|_| std::path::absolute(p).unwrap_or_else(|_| p.to_path_buf()))
}

fn hash_arquivo(p: &Path) -> String {
    match std::fs::read(p) {
        Ok(b) => blake3::hash(&b).to_hex()[..16].to_string(),
        Err(_) => "0".repeat(16),
    }
}

impl ArvoreProduto {
    /// A regra de cor: liberado é peça liberada ou montagem com todas as peças de baixo liberadas.
    pub fn liberado(&self, i: usize) -> bool {
        match self.nos[i].estado {
            EstadoNo::Documento(e) => e == DocState::Liberado,
            EstadoNo::Derivado { liberadas, total } => total > 0 && liberadas == total,
            EstadoNo::Fora => false,
        }
    }

    /// Peças distintas abaixo de `i` (o próprio `i`, se for peça), sem repetir nem entrar em ciclo.
    pub fn pecas_abaixo(&self, i: usize) -> Vec<usize> {
        let mut vistos = vec![false; self.nos.len()];
        let mut pilha = vec![i];
        let mut out = vec![];
        while let Some(n) = pilha.pop() {
            if std::mem::replace(&mut vistos[n], true) {
                continue;
            }
            match self.nos[n].tipo {
                TipoNo::Peca => out.push(n),
                TipoNo::Montagem => pilha.extend(self.nos[n].filhos.iter().map(|a| a.filho)),
            }
        }
        out.sort_unstable();
        out
    }

    /// Instâncias de `i` dentro da raiz `r`: produto das quantidades ao longo de cada caminho.
    pub fn quantidade_em(&self, r: usize, i: usize) -> u64 {
        fn rec(a: &ArvoreProduto, n: usize, alvo: usize, caminho: &mut Vec<usize>) -> u64 {
            if n == alvo {
                return 1;
            }
            if caminho.contains(&n) {
                return 0;
            }
            caminho.push(n);
            let q = a.nos[n].filhos.iter().map(|x| u64::from(x.quantidade) * rec(a, x.filho, alvo, caminho)).sum();
            caminho.pop();
            q
        }
        rec(self, r, i, &mut vec![])
    }

    /// Nós até `fundo` níveis abaixo de `r` (contando com ele), seguindo cada caminho só uma vez.
    pub fn contar_ate(&self, r: usize, fundo: usize) -> usize {
        fn rec(a: &ArvoreProduto, n: usize, fundo: usize, caminho: &mut Vec<usize>) -> usize {
            if fundo == 0 || caminho.contains(&n) {
                return 0;
            }
            caminho.push(n);
            let c = 1 + a.nos[n].filhos.iter().map(|x| rec(a, x.filho, fundo - 1, caminho)).sum::<usize>();
            caminho.pop();
            c
        }
        rec(self, r, fundo, &mut vec![])
    }

    /// Maior número de níveis abaixo de `r` (uma peça tem 1).
    pub fn niveis(&self, r: usize) -> usize {
        fn rec(a: &ArvoreProduto, n: usize, caminho: &mut Vec<usize>) -> usize {
            if caminho.contains(&n) {
                return 0;
            }
            caminho.push(n);
            let c = 1 + a.nos[n].filhos.iter().map(|x| rec(a, x.filho, caminho)).max().unwrap_or(0);
            caminho.pop();
            c
        }
        rec(self, r, &mut vec![])
    }

    /// `de` chega a `ate` descendo pelas arestas?
    pub fn alcanca(&self, de: usize, ate: usize) -> bool {
        let mut vistos = vec![false; self.nos.len()];
        let mut pilha = vec![de];
        while let Some(n) = pilha.pop() {
            if std::mem::replace(&mut vistos[n], true) {
                continue;
            }
            for a in &self.nos[n].filhos {
                if a.filho == ate {
                    return true;
                }
                pilha.push(a.filho);
            }
        }
        false
    }

    /// Nó do arquivo `p`, se ele está na árvore.
    pub fn no_do_arquivo(&self, p: &Path) -> Option<usize> {
        let k = chave(p);
        self.nos.iter().position(|n| chave(&n.arquivo) == k)
    }

    /// Nó da peça `id` do índice.
    pub fn no_do_documento(&self, id: u64) -> Option<usize> {
        self.nos.iter().position(|n| n.id == Some(id))
    }
}

struct Montador<'a> {
    p: &'a Project,
    nos: Vec<NoProduto>,
    por_chave: HashMap<PathBuf, usize>,
    por_uid: HashMap<String, u64>,
    entrada_por_chave: HashMap<PathBuf, u64>,
}

impl Montador<'_> {
    fn no(&mut self, arquivo: &Path, tipo: TipoNo) -> usize {
        let k = chave(arquivo);
        if let Some(&i) = self.por_chave.get(&k) {
            return i;
        }
        let existe = arquivo.is_file();
        let cab = relacoes::ler_cabecalho(arquivo);
        let id = self.entrada_por_chave.get(&k).copied().or_else(|| cab.as_ref().filter(|c| !c.uid.is_empty()).and_then(|c| self.por_uid.get(&c.uid).copied()));
        let entrada = id.and_then(|i| self.p.entry(i).ok());
        let nome = entrada.map(|e| e.name.clone()).or_else(|| cab.as_ref().map(|c| c.nome.clone()).filter(|n| !n.is_empty())).unwrap_or_else(|| arquivo.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default());
        let pn = entrada.map(|e| e.pn.clone()).or_else(|| cab.as_ref().map(|c| c.pn.clone())).unwrap_or_default();
        let estado = match (entrada, tipo) {
            (Some(e), _) => EstadoNo::Documento(e.state),
            (None, TipoNo::Montagem) => EstadoNo::Derivado { liberadas: 0, total: 0 },
            (None, TipoNo::Peca) => EstadoNo::Fora,
        };
        let i = self.nos.len();
        self.nos.push(NoProduto {
            arquivo: arquivo.to_path_buf(),
            tipo,
            nome,
            pn,
            id,
            estado,
            revisao: entrada.and_then(|e| e.current_revision()).map(|r| r.label.clone()),
            versao: entrada.and_then(|e| e.versions.last()).map(|v| v.hash.clone()),
            existe,
            filhos: vec![],
            pais: vec![],
            assinatura: String::new(),
        });
        self.por_chave.insert(k, i);
        if tipo == TipoNo::Montagem && existe {
            let dir = arquivo.parent().map(Path::to_path_buf).unwrap_or_default();
            let mut filhos: Vec<Aresta> = vec![];
            for r in relacoes::referencias_de(arquivo) {
                let t = match r.tipo {
                    TipoRef::Peca => TipoNo::Peca,
                    TipoRef::Subconjunto => TipoNo::Montagem,
                    _ => continue,
                };
                let f = self.no(&r.arquivo(&dir), t);
                match filhos.iter_mut().find(|a| a.filho == f) {
                    Some(a) => a.quantidade += 1,
                    None => filhos.push(Aresta { filho: f, quantidade: 1 }),
                }
            }
            for a in &filhos {
                if !self.nos[a.filho].pais.contains(&i) {
                    self.nos[a.filho].pais.push(i);
                }
            }
            self.nos[i].filhos = filhos;
        }
        i
    }
}

impl Project {
    /// Árvore do produto do projeto (B364): montagens das pastas de busca e peças do índice.
    pub fn arvore_produto(&self) -> ArvoreProduto {
        let mut m = Montador { p: self, nos: vec![], por_chave: HashMap::new(), por_uid: HashMap::new(), entrada_por_chave: HashMap::new() };
        for d in &self.index.documents {
            m.entrada_por_chave.insert(chave(&self.root.join(&d.file)), d.id);
            if !d.uid.is_empty() {
                m.por_uid.insert(d.uid.clone(), d.id);
            }
        }
        let mut montagens = vec![];
        for pasta in self.pastas_de_busca() {
            relacoes::listar_documentos(&pasta, FUNDO, &mut montagens);
        }
        montagens.retain(|a| a.extension().is_some_and(|x| x == "fka"));
        montagens.sort();
        montagens.dedup_by(|a, b| chave(a) == chave(b));
        let mut topo = vec![];
        for a in &montagens {
            topo.push(m.no(a, TipoNo::Montagem));
        }
        let mut soltas = vec![];
        for d in &self.index.documents {
            soltas.push(m.no(&self.root.join(&d.file), TipoNo::Peca));
        }
        let mut arv = ArvoreProduto { nos: m.nos, raizes: vec![] };
        // raízes: montagens que ninguém usa, pela ordem do nome do arquivo; depois as peças soltas
        topo.dedup();
        let topo_todas = topo.clone();
        let mut raizes: Vec<usize> = topo.into_iter().filter(|&i| arv.nos[i].pais.is_empty()).collect();
        raizes.extend(soltas.into_iter().filter(|&i| arv.nos[i].pais.is_empty()));
        raizes.dedup();
        // montagem só alcançável por um ciclo (A usa B que usa A) não teria raiz: entra como raiz
        for &t in &topo_todas {
            let alcancada = raizes.iter().any(|&r| r == t || arv.alcanca(r, t));
            if !alcancada {
                raizes.push(t);
            }
        }
        arv.raizes = raizes;
        // estado derivado das montagens
        for i in 0..arv.nos.len() {
            if matches!(arv.nos[i].estado, EstadoNo::Derivado { .. }) {
                let pecas = arv.pecas_abaixo(i);
                let liberadas = pecas.iter().filter(|&&n| arv.liberado(n)).count();
                arv.nos[i].estado = EstadoNo::Derivado { liberadas, total: pecas.len() };
            }
        }
        // assinaturas: peça do índice pela versão, o resto pelo conteúdo do arquivo e dos filhos
        let mut cache: Vec<Option<String>> = vec![None; arv.nos.len()];
        for i in 0..arv.nos.len() {
            let s = assinatura(&arv, i, &mut cache, &mut vec![]);
            arv.nos[i].assinatura = s;
        }
        arv
    }
}

fn assinatura(a: &ArvoreProduto, i: usize, cache: &mut Vec<Option<String>>, caminho: &mut Vec<usize>) -> String {
    if let Some(s) = &cache[i] {
        return s.clone();
    }
    let n = &a.nos[i];
    let proprio = n.versao.clone().unwrap_or_else(|| hash_arquivo(&n.arquivo));
    if n.filhos.is_empty() || caminho.contains(&i) {
        // no ciclo, a assinatura parcial não vai para o cache: o nó ainda não terminou
        if !caminho.contains(&i) {
            cache[i] = Some(proprio.clone());
        }
        return proprio;
    }
    caminho.push(i);
    let mut h = blake3::Hasher::new();
    h.update(proprio.as_bytes());
    for f in &n.filhos {
        h.update(assinatura(a, f.filho, cache, caminho).as_bytes());
        h.update(&f.quantidade.to_le_bytes());
    }
    caminho.pop();
    let s = h.finalize().to_hex()[..16].to_string();
    cache[i] = Some(s.clone());
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use fenkra_core::{BoolOp, Document, FeatureKind, OriginPlane, PlaneRef, PrimKind, Primitive};

    fn pasta(nome: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("fenkra_arvore_{nome}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    fn peca(nome: &str, lado: &str) -> Document {
        let s = |x: &str| x.to_string();
        let mut d = Document::new(nome);
        d.add_feature(FeatureKind::Primitive(Primitive { kind: PrimKind::Caixa, plane: PlaneRef::Origin(OriginPlane::XY), a: s(lado), b: s("20"), c: s("5"), d: s("0"), op: BoolOp::Add }));
        d
    }

    /// Montagem mínima no formato gravado: cada componente é `(tipo, nome, caminho)`.
    fn montagem(nome: &str, comp: &[(&str, &str, &str)]) -> String {
        let nos: Vec<String> = comp
            .iter()
            .enumerate()
            .map(|(i, (t, n, c))| format!(r#"{{"id": {}, "name": "{n}", "kind": {{"{t}": {{"path": {}}}}}}}"#, i + 1, serde_json::to_string(c).unwrap()))
            .collect();
        format!(r#"{{"format_version": 1, "name": "{nome}", "nodes": [{}]}}"#, nos.join(","))
    }

    fn arquivo(p: &Project, id: u64) -> String {
        p.entry(id).unwrap().file.replace('\\', "/")
    }

    #[test]
    fn grafo_com_niveis_quantidades_e_estado_derivado() {
        let d = pasta("niveis");
        let mut p = Project::create(&d, "Kart").unwrap();
        let base = p.add_document(peca("Base", "40")).unwrap();
        let eixo = p.add_document(peca("Eixo", "10")).unwrap();
        let solta = p.add_document(peca("Calço", "5")).unwrap();
        p.release(base, "ok").unwrap();
        let (fb, fe) = (arquivo(&p, base), arquivo(&p, eixo));
        std::fs::write(d.join("roda.fka"), montagem("Roda", &[("Part", "Eixo", &fe), ("Part", "Eixo", &fe), ("Part", "Base", &fb)])).unwrap();
        std::fs::write(d.join("kart.fka"), montagem("Kart", &[("SubAssembly", "Roda", "roda.fka"), ("SubAssembly", "Roda", "roda.fka"), ("Part", "Base", &fb)])).unwrap();
        let a = p.arvore_produto();
        let kart = a.no_do_arquivo(&d.join("kart.fka")).unwrap();
        let roda = a.no_do_arquivo(&d.join("roda.fka")).unwrap();
        let (nb, ne, ns) = (a.no_do_documento(base).unwrap(), a.no_do_documento(eixo).unwrap(), a.no_do_documento(solta).unwrap());
        // um nó por arquivo; raízes: a montagem de topo e a peça que ninguém usa
        assert_eq!(a.nos.len(), 5);
        assert_eq!(a.raizes, vec![kart, ns]);
        assert_eq!(a.nos[kart].filhos, vec![Aresta { filho: roda, quantidade: 2 }, Aresta { filho: nb, quantidade: 1 }]);
        assert_eq!(a.nos[nb].pais.len(), 2, "a base é usada pelo kart e pela roda");
        // instâncias: 2 rodas × 2 eixos; base: 1 direta + 2 rodas × 1
        assert_eq!(a.quantidade_em(kart, ne), 4);
        assert_eq!(a.quantidade_em(kart, nb), 3);
        assert_eq!(a.niveis(kart), 3);
        // PN, nome e estado vêm do índice
        assert_eq!(a.nos[nb].pn, p.entry(base).unwrap().pn);
        assert_eq!(a.nos[nb].revisao.as_deref(), Some("A"));
        assert_eq!(a.nos[nb].estado, EstadoNo::Documento(DocState::Liberado));
        assert!(a.liberado(nb) && !a.liberado(ne));
        assert_eq!(a.nos[kart].nome, "Kart");
        assert_eq!(a.nos[kart].estado, EstadoNo::Derivado { liberadas: 1, total: 2 });
        assert!(!a.liberado(kart), "o eixo ainda está em trabalho");
        // liberar o eixo libera a montagem pela regra derivada
        p.release(eixo, "ok").unwrap();
        let a = p.arvore_produto();
        assert!(a.liberado(a.no_do_arquivo(&d.join("kart.fka")).unwrap()));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn assinatura_da_montagem_muda_quando_a_peca_de_baixo_muda() {
        let d = pasta("assinatura");
        let mut p = Project::create(&d, "P").unwrap();
        let id = p.add_document(peca("Base", "40")).unwrap();
        std::fs::write(d.join("conj.fka"), montagem("Conj", &[("Part", "Base", &arquivo(&p, id))])).unwrap();
        let a1 = p.arvore_produto();
        let s1 = a1.nos[a1.no_do_arquivo(&d.join("conj.fka")).unwrap()].assinatura.clone();
        assert_eq!(s1.len(), 16);
        assert_eq!(p.arvore_produto().nos[a1.no_do_arquivo(&d.join("conj.fka")).unwrap()].assinatura, s1, "sem mudança, a mesma assinatura");
        p.save(id, &peca("Base", "55"), "maior").unwrap();
        let a2 = p.arvore_produto();
        let nb = a2.no_do_documento(id).unwrap();
        assert_eq!(a2.nos[nb].versao.as_deref(), Some(p.entry(id).unwrap().versions.last().unwrap().hash.as_str()));
        assert_ne!(a2.nos[a2.no_do_arquivo(&d.join("conj.fka")).unwrap()].assinatura, s1);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn ciclo_arquivo_ausente_e_peca_movida_pelo_uid() {
        let d = pasta("ciclo");
        let mut p = Project::create(&d, "P").unwrap();
        let id = p.add_document(peca("Base", "40")).unwrap();
        // a peça foi copiada para outra pasta: o caminho não bate, o uid gravado dentro sim
        std::fs::create_dir_all(d.join("copia")).unwrap();
        std::fs::copy(d.join(&p.entry(id).unwrap().file), d.join("copia/base.fkp")).unwrap();
        std::fs::write(d.join("a.fka"), montagem("A", &[("SubAssembly", "B", "b.fka"), ("Part", "Base", "copia/base.fkp"), ("Part", "Sumiu", "nao/existe.fkp")])).unwrap();
        std::fs::write(d.join("b.fka"), montagem("B", &[("SubAssembly", "A", "a.fka")])).unwrap();
        let a = p.arvore_produto();
        let na = a.no_do_arquivo(&d.join("a.fka")).unwrap();
        let movida = a.no_do_arquivo(&d.join("copia/base.fkp")).unwrap();
        assert_eq!(a.nos[movida].id, Some(id));
        let sumiu = a.no_do_arquivo(&d.join("nao/existe.fkp")).unwrap();
        assert!(!a.nos[sumiu].existe);
        assert_eq!(a.nos[sumiu].estado, EstadoNo::Fora);
        // o ciclo A → B → A termina: a volta para A não conta nível, e as contagens são finitas
        assert_eq!(a.niveis(na), 2);
        assert!(a.contar_ate(na, 50) < 10);
        assert_eq!(a.pecas_abaixo(na).len(), 2);
        // num ciclo puro as duas têm pai, mas a primeira entra como raiz para o conjunto não sumir
        assert!(a.raizes.contains(&na));
        assert!(!a.raizes.contains(&a.no_do_arquivo(&d.join("b.fka")).unwrap()), "a B já aparece abaixo da A");
        let _ = std::fs::remove_dir_all(&d);
    }
}
