// SPDX-License-Identifier: LGPL-2.1-or-later
//! Peça de fora do projeto copiada para ele (B445).
//!
//! Com um projeto aberto, a montagem só deve usar documentos que o projeto conhece: são eles que têm
//! PN, versões, exceções de qualidade e diário. Uma peça de outra pasta não tinha nada disso, então o
//! perfil de qualidade a barrava sem saída (a exceção mora no projeto). Agora ela entra por uma cópia:
//!
//! 1. percorre a peça e, pelos links, as peças de onde ela tira geometria, até o fim da cadeia. O que
//!    já é do projeto (mesmo uid ou mesmo arquivo do índice) não é copiado: o link passa a apontar
//!    para o documento do projeto;
//! 2. cada peça copiada vai para `pecas/` com o nome do arquivo original. Se o nome já existe no
//!    projeto, ganha `_2`, `_3`... e o chamador avisa;
//! 3. a cópia é outro documento: ganha uid novo (dois arquivos com o mesmo uid fariam o resolvedor
//!    achar "vários candidatos"), e quem a cita por link recebe o uid novo junto com o caminho novo;
//! 4. o arquivo de malha e as texturas também são copiados, e o comando passa a ler a cópia;
//! 5. cada peça entra no índice com PN e a primeira versão, cuja nota diz de onde veio. O PN do arquivo
//!    é mantido se estiver livre no projeto; se outro documento já o usa, a cópia ganha um novo.
//!
//! O histórico de comandos não muda: a cópia é o mesmo documento paramétrico, lido e gravado pelo
//! formato de sempre, e só os caminhos e os uids das referências são trocados. O original não é
//! tocado. Se qualquer gravação falha, os arquivos criados são apagados e o índice volta ao que era.

use crate::referencias::{caminho_relativo, ler_uid};
use crate::seguro::gravar_atomico;
use crate::{PResult, Project, ProjectError, PARTS};
use fenkra_core::{Document, FeatureKind};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Publicação dos furos que a montagem manda para a peça (`fenkra_montagem::furos::PREFIXO`). Esse
/// link aponta para a montagem, que não é dependência da peça: não é copiada, só reapontada.
const PREFIXO_FURO: &str = "montagem:furo:";

/// O que a cópia gravou e o que o chamador precisa avisar.
#[derive(Clone, Debug, Default)]
pub struct CopiaParaProjeto {
    /// a cópia da peça pedida, dentro do projeto
    pub arquivo: PathBuf,
    /// id da cópia da peça pedida no índice
    pub id: u64,
    /// ids de todas as peças registradas, a pedida primeiro
    pub ids: Vec<u64>,
    /// (original, cópia) de cada arquivo gravado, a peça pedida primeiro
    pub copiados: Vec<(PathBuf, PathBuf)>,
    /// (nome pedido, nome gravado) dos arquivos cujo nome já existia no projeto
    pub renomeados: Vec<(String, String)>,
    /// peças usadas por link que já eram do projeto: o link passa a apontar para elas
    pub do_projeto: Vec<PathBuf>,
    /// referências cujo arquivo não existe: ficam gravadas como estavam
    pub ausentes: Vec<String>,
    /// (PN do arquivo, PN dado) quando o PN já era de outro documento do projeto
    pub pn_trocados: Vec<(String, String)>,
}

/// Uma peça a copiar, já lida.
struct Item {
    original: PathBuf,
    doc: Document,
    destino: PathBuf,
}

fn absoluto(p: &Path) -> PathBuf {
    std::path::absolute(p).unwrap_or_else(|_| p.to_path_buf())
}

/// Chave para comparar arquivos: o caminho canônico, sem diferença de maiúscula no Windows.
fn chave(p: &Path) -> String {
    let c = std::fs::canonicalize(p).unwrap_or_else(|_| absoluto(p));
    let t = c.to_string_lossy().replace('\\', "/");
    let t = t.strip_prefix("//?/").unwrap_or(&t).to_string();
    if cfg!(windows) {
        t.to_lowercase()
    } else {
        t
    }
}

fn nome_de(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

fn minusculo(s: &str) -> String {
    if cfg!(windows) {
        s.to_lowercase()
    } else {
        s.to_string()
    }
}

/// Arquivo de origem de um link, visto da peça que o grava. Mesma regra de `fenkra_montagem::links`:
/// caminho relativo vale a partir da pasta da peça.
fn alvo_do_link(peca: &Path, caminho: &str) -> PathBuf {
    let p = Path::new(caminho);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        peca.parent().map(|d| d.join(p)).unwrap_or_else(|| p.to_path_buf())
    }
}

/// Arquivo de malha ou de textura: absoluto, ao lado da peça ou relativo à pasta de trabalho.
fn arquivo_de_recurso(peca: &Path, caminho: &str) -> Option<PathBuf> {
    if caminho.is_empty() {
        return None;
    }
    let p = Path::new(caminho);
    if p.is_absolute() {
        return p.is_file().then(|| p.to_path_buf());
    }
    [peca.parent().map(|d| d.join(p)), Some(p.to_path_buf())].into_iter().flatten().find(|c| c.is_file()).map(|c| absoluto(&c))
}

fn texto_do_caminho(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

impl Project {
    /// Documento do índice que o arquivo é: o próprio arquivo de trabalho ou, pelo uid, o mesmo
    /// documento guardado em outra pasta.
    pub fn id_do_arquivo(&self, arquivo: &Path) -> Option<u64> {
        let k = chave(arquivo);
        if let Some(e) = self.index.documents.iter().find(|e| chave(&self.root.join(&e.file)) == k) {
            return Some(e.id);
        }
        let uid = ler_uid(arquivo);
        if uid.is_empty() {
            return None;
        }
        self.index.documents.iter().find(|e| e.uid == uid).map(|e| e.id)
    }

    /// O arquivo já é um documento deste projeto?
    pub fn pertence(&self, arquivo: &Path) -> bool {
        self.id_do_arquivo(arquivo).is_some()
    }

    /// Nomes de arquivo que o projeto já usa, em qualquer pasta dele.
    fn nomes_ocupados(&self) -> HashSet<String> {
        self.index.documents.iter().map(|e| minusculo(&nome_de(Path::new(&e.file)))).collect()
    }

    /// Onde a cópia de `arquivo` ficaria agora: `pecas/<nome>` ou, se o nome já existe no projeto,
    /// `pecas/<nome>_2`... O aviso mostra isso antes de a pessoa decidir.
    pub fn destino_da_copia(&self, arquivo: &Path) -> PathBuf {
        let mut ocupados = self.nomes_ocupados();
        self.nome_livre(&absoluto(&self.root.join(PARTS)), &nome_de(arquivo), &mut ocupados, &mut vec![])
    }

    /// Nome livre em `pasta` para `nome`: o próprio, ou com `_2`, `_3`... quando o nome já existe na
    /// pasta, em outro documento do índice ou em outra cópia desta mesma operação.
    fn nome_livre(&self, pasta: &Path, nome: &str, ocupados: &mut HashSet<String>, renomeados: &mut Vec<(String, String)>) -> PathBuf {
        let livre = |n: &str, ocupados: &HashSet<String>| !pasta.join(n).exists() && !ocupados.contains(&minusculo(n));
        let mut escolhido = nome.to_string();
        if !livre(nome, ocupados) {
            let p = Path::new(nome);
            let tronco = p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            let ext = p.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
            let mut n = 2;
            loop {
                let tentativa = format!("{tronco}_{n}{ext}");
                if livre(&tentativa, ocupados) {
                    renomeados.push((nome.to_string(), tentativa.clone()));
                    escolhido = tentativa;
                    break;
                }
                n += 1;
            }
        }
        ocupados.insert(minusculo(&escolhido));
        pasta.join(escolhido)
    }

    /// Copia a peça `arquivo` (de fora do projeto) e as peças de que ela depende para `pecas/`, registra
    /// cada uma com PN e versão e devolve onde ficou a cópia. Ver o topo do módulo.
    pub fn copiar_para_projeto(&mut self, arquivo: &Path) -> PResult<CopiaParaProjeto> {
        self.pode_gravar()?;
        if self.pertence(arquivo) {
            return Err(ProjectError::Recusado("a peça já é deste projeto".into()));
        }
        let pasta = absoluto(&self.root.join(PARTS));
        let mut res = CopiaParaProjeto::default();
        let mut ocupados = self.nomes_ocupados();
        // chave do arquivo original -> onde ele fica no projeto (cópia nova ou documento que já existia)
        let mut no_projeto: HashMap<String, PathBuf> = HashMap::new();

        // 1. a peça e a cadeia de links dela
        let mut itens: Vec<Item> = vec![];
        let mut fila = vec![absoluto(arquivo)];
        while !fila.is_empty() {
            let f = fila.remove(0);
            if no_projeto.contains_key(&chave(&f)) {
                continue;
            }
            let texto = std::fs::read_to_string(&f)?;
            let doc = Document::from_json(&texto)?;
            let destino = self.nome_livre(&pasta, &nome_de(&f), &mut ocupados, &mut res.renomeados);
            no_projeto.insert(chave(&f), destino.clone());
            for e in doc.external.iter().filter(|e| !e.path.is_empty() && !e.publication.starts_with(PREFIXO_FURO)) {
                let alvo = absoluto(&alvo_do_link(&f, &e.path));
                if no_projeto.contains_key(&chave(&alvo)) {
                    continue;
                }
                if !alvo.is_file() {
                    if !res.ausentes.contains(&e.path) {
                        res.ausentes.push(e.path.clone());
                    }
                    continue;
                }
                match self.id_do_arquivo(&alvo).and_then(|id| self.entry(id).ok()) {
                    Some(en) => {
                        let doc_projeto = absoluto(&self.root.join(&en.file));
                        no_projeto.insert(chave(&alvo), doc_projeto.clone());
                        res.do_projeto.push(doc_projeto);
                    }
                    None => fila.push(alvo),
                }
            }
            itens.push(Item { original: f, doc, destino });
        }

        // 2. identidade nova para cada cópia, decidida antes para quem a cita receber o uid certo
        let mut uids: HashMap<String, String> = HashMap::new();
        for it in &mut itens {
            let novo = fenkra_core::formato::novo_uid();
            if !it.doc.uid.esta_vazio() {
                uids.insert(it.doc.uid.0.clone(), novo.clone());
            }
            it.doc.uid.0 = novo;
        }

        // 3. caminhos das referências, vistos da pasta nova; malhas e texturas copiadas junto
        let mut recursos: Vec<(PathBuf, PathBuf)> = vec![];
        let mut recurso_de: HashMap<String, PathBuf> = HashMap::new();
        for it in &mut itens {
            let dir_novo = absoluto(it.destino.parent().unwrap_or(&pasta));
            for e in &mut it.doc.external {
                if e.path.is_empty() {
                    continue;
                }
                let alvo = absoluto(&alvo_do_link(&it.original, &e.path));
                if e.publication.starts_with(PREFIXO_FURO) {
                    if alvo.is_file() {
                        e.path = caminho_relativo(&dir_novo, &alvo);
                    }
                    continue;
                }
                if let Some(d) = no_projeto.get(&chave(&alvo)) {
                    e.path = caminho_relativo(&dir_novo, d);
                    if let Some(u) = uids.get(&e.uid) {
                        e.uid = u.clone();
                    }
                }
            }
            let mut caminhos: Vec<&mut String> = vec![];
            for f in &mut it.doc.features {
                if let FeatureKind::MeshImport(m) = &mut f.kind {
                    caminhos.push(&mut m.path);
                }
            }
            caminhos.extend(it.doc.textures.iter_mut().map(|t| &mut t.path));
            for c in caminhos {
                let Some(orig) = arquivo_de_recurso(&it.original, c) else {
                    if !c.is_empty() && !res.ausentes.contains(c) {
                        res.ausentes.push(c.clone());
                    }
                    continue;
                };
                let k = chave(&orig);
                let destino = match recurso_de.get(&k) {
                    Some(d) => d.clone(),
                    None => {
                        let d = self.nome_livre(&pasta, &nome_de(&orig), &mut ocupados, &mut res.renomeados);
                        recurso_de.insert(k, d.clone());
                        recursos.push((orig, d.clone()));
                        d
                    }
                };
                // o comando lê a malha pelo caminho como está gravado: absoluto, para não depender da
                // pasta de trabalho do programa
                *c = texto_do_caminho(&destino);
            }
        }

        // 4. PN: o do arquivo vale se estiver livre no projeto
        let mut pns: HashSet<String> = self.index.documents.iter().map(|e| e.pn.clone()).collect();
        let mut trocar: Vec<usize> = vec![];
        for (i, it) in itens.iter_mut().enumerate() {
            let pn = it.doc.props.pn.clone();
            if pn.is_empty() {
                continue;
            }
            if pns.contains(&pn) {
                res.pn_trocados.push((pn, String::new()));
                trocar.push(i);
                it.doc.props.pn.clear();
            } else {
                pns.insert(pn);
            }
        }

        // 5. grava: recursos, depois as peças pelo registro do projeto (arquivo de trabalho + versão)
        let antes = self.index.clone();
        let mut criados: Vec<PathBuf> = vec![];
        let gravacao = (|| -> PResult<()> {
            for (o, d) in &recursos {
                gravar_atomico(d, &std::fs::read(o)?)?;
                criados.push(d.clone());
            }
            for it in &itens {
                let rel = texto_do_caminho(&Path::new(PARTS).join(nome_de(&it.destino)));
                let nota = format!("copiada de {}", texto_do_caminho(&it.original));
                criados.push(it.destino.clone());
                let id = self.adicionar(it.doc.clone(), Some(rel), &nota)?;
                res.ids.push(id);
            }
            Ok(())
        })();
        if let Err(e) = gravacao {
            for c in &criados {
                let _ = std::fs::remove_file(c);
            }
            self.index = antes;
            let _ = self.save_index();
            return Err(e);
        }
        for (n, i) in trocar.iter().enumerate() {
            if let Some(en) = res.ids.get(*i).and_then(|id| self.entry(*id).ok()) {
                res.pn_trocados[n].1 = en.pn.clone();
            }
        }
        res.id = res.ids[0];
        res.arquivo = itens[0].destino.clone();
        res.copiados = itens.iter().map(|it| (it.original.clone(), it.destino.clone())).chain(recursos).collect();
        Ok(res)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fenkra_core::LinkedGeo;

    fn pasta(nome: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("fenkra-copia-{nome}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn gravar(p: &Path, d: &Document) {
        std::fs::write(p, d.to_json()).unwrap();
    }

    #[test]
    fn peca_de_fora_entra_com_pn_versao_e_os_links_apontando_para_as_copias() {
        let fora = pasta("links-fora");
        let root = pasta("links-proj").join("P");
        let mut p = Project::create(&root, "P").unwrap();
        // mestre <- placa (link de valor); a placa também tem um furo mandado por uma montagem
        let mut mestre = Document::new("Mestre");
        mestre.props.pn = "EXT-1".into();
        gravar(&fora.join("mestre.fkp"), &mestre);
        let mut placa = fenkra_solidos::example_document();
        placa.name = "Placa".into();
        let comandos = placa.features.len();
        placa.add_link("Mestre", "mestre.fkp", "EE", LinkedGeo::Value(40.));
        placa.external.last_mut().unwrap().uid = mestre.uid.0.clone();
        std::fs::write(fora.join("conj.fka"), "{}").unwrap();
        placa.add_link("conj", "conj.fka", "montagem:furo:1", LinkedGeo::Value(0.));
        gravar(&fora.join("placa.fkp"), &placa);
        let original = std::fs::read_to_string(fora.join("placa.fkp")).unwrap();

        assert!(!p.pertence(&fora.join("placa.fkp")));
        let r = p.copiar_para_projeto(&fora.join("placa.fkp")).unwrap();
        assert_eq!(r.arquivo, absoluto(&root.join("pecas").join("placa.fkp")));
        assert!(r.renomeados.is_empty());
        assert_eq!(r.ids.len(), 2, "a placa e o mestre de onde ela tira geometria");
        assert!(p.pertence(&r.arquivo));
        // identidade: PN, versão com a origem na nota e uid novo
        let e = p.entry(r.id).unwrap().clone();
        assert_eq!(e.file, "pecas/placa.fkp");
        assert!(!e.pn.is_empty());
        assert_eq!(e.versions.len(), 1);
        assert!(e.versions[0].note.starts_with("copiada de ") && e.versions[0].note.ends_with("placa.fkp"));
        let copia = p.load(r.id).unwrap();
        assert_ne!(copia.uid.0, placa.uid.0, "a cópia é outro documento");
        assert_eq!(copia.features.len(), comandos, "o histórico de comandos vem inteiro");
        // o mestre entrou com o PN que já tinha (livre no projeto)
        let m = p.entry(r.ids[1]).unwrap();
        assert_eq!((m.pn.as_str(), m.file.as_str()), ("EXT-1", "pecas/mestre.fkp"));
        // o link aponta para a cópia do mestre, com o uid dela
        let link = copia.external.iter().find(|x| x.publication == "EE").unwrap();
        assert_eq!(link.path, "mestre.fkp");
        assert_eq!(link.uid, m.uid);
        // o furo continua apontando para a montagem original, que não é copiada
        let furo = copia.external.iter().find(|x| x.publication.starts_with(PREFIXO_FURO)).unwrap();
        assert_eq!(chave(&alvo_do_link(&r.arquivo, &furo.path)), chave(&fora.join("conj.fka")));
        assert!(!root.join("pecas").join("conj.fka").exists());
        // o original não mudou
        assert_eq!(std::fs::read_to_string(fora.join("placa.fkp")).unwrap(), original);
        // copiar de novo o que já é do projeto é recusado
        assert!(p.copiar_para_projeto(&r.arquivo).is_err());
    }

    #[test]
    fn nome_que_ja_existe_no_projeto_ganha_sufixo_e_pn_repetido_ganha_outro() {
        let fora = pasta("nome-fora");
        let root = pasta("nome-proj").join("P");
        let mut p = Project::create(&root, "P").unwrap();
        let ja = p.add_document(Document::new("Placa do projeto")).unwrap();
        let pn_ja = p.entry(ja).unwrap().pn.clone();
        std::fs::write(root.join("pecas").join("placa.fkp"), "{}").unwrap();
        let mut placa = Document::new("Placa");
        placa.props.pn = pn_ja.clone();
        gravar(&fora.join("placa.fkp"), &placa);
        assert_eq!(nome_de(&p.destino_da_copia(&fora.join("placa.fkp"))), "placa_2.fkp", "o aviso já mostra o nome novo");
        let r = p.copiar_para_projeto(&fora.join("placa.fkp")).unwrap();
        assert_eq!(r.renomeados, vec![("placa.fkp".to_string(), "placa_2.fkp".to_string())]);
        assert_eq!(nome_de(&r.arquivo), "placa_2.fkp");
        assert_eq!(std::fs::read_to_string(root.join("pecas").join("placa.fkp")).unwrap(), "{}", "o arquivo que já existia fica intacto");
        assert_eq!(r.pn_trocados.len(), 1);
        assert_eq!(r.pn_trocados[0].0, pn_ja);
        assert_ne!(r.pn_trocados[0].1, pn_ja);
        assert_eq!(p.entry(r.id).unwrap().pn, r.pn_trocados[0].1);
        // de novo: placa_3, sem tocar nas outras
        let r2 = p.copiar_para_projeto(&fora.join("placa.fkp")).unwrap();
        assert_eq!(nome_de(&r2.arquivo), "placa_3.fkp");
    }

    #[test]
    fn link_para_peca_que_ja_e_do_projeto_nao_copia_e_aponta_para_ela() {
        let fora = pasta("dep-fora");
        let root = pasta("dep-proj").join("P");
        let mut p = Project::create(&root, "P").unwrap();
        let id_mestre = p.add_document(Document::new("Mestre")).unwrap();
        let arq_mestre = root.join(&p.entry(id_mestre).unwrap().file);
        let mut placa = Document::new("Placa");
        placa.add_link("Mestre", &texto_do_caminho(&arq_mestre), "EE", LinkedGeo::Value(1.));
        placa.external.last_mut().unwrap().uid = p.entry(id_mestre).unwrap().uid.clone();
        placa.add_link("Sumida", "sumida.fkp", "X", LinkedGeo::Value(1.));
        gravar(&fora.join("placa.fkp"), &placa);
        let r = p.copiar_para_projeto(&fora.join("placa.fkp")).unwrap();
        assert_eq!(r.ids.len(), 1, "o mestre já era do projeto");
        assert_eq!(r.do_projeto.len(), 1);
        assert_eq!(r.ausentes, vec!["sumida.fkp".to_string()]);
        let copia = p.load(r.id).unwrap();
        let l = copia.external.iter().find(|x| x.publication == "EE").unwrap();
        assert_eq!(chave(&alvo_do_link(&r.arquivo, &l.path)), chave(&arq_mestre));
        assert_eq!(l.uid, p.entry(id_mestre).unwrap().uid);
        assert_eq!(copia.external.iter().find(|x| x.publication == "X").unwrap().path, "sumida.fkp", "a ausente fica como estava");
    }

    #[test]
    fn malha_importada_vai_junto_e_o_comando_le_a_copia() {
        let fora = pasta("malha-fora");
        let root = pasta("malha-proj").join("P");
        let mut p = Project::create(&root, "P").unwrap();
        std::fs::write(fora.join("casca.stl"), b"solid x\nendsolid x\n").unwrap();
        let mut d = Document::new("Casca");
        d.add_feature(FeatureKind::MeshImport(fenkra_core::MeshImport { path: texto_do_caminho(&fora.join("casca.stl")), solid: true, ..Default::default() }));
        gravar(&fora.join("casca.fkp"), &d);
        let r = p.copiar_para_projeto(&fora.join("casca.fkp")).unwrap();
        let copia = p.load(r.id).unwrap();
        let m = copia.features.iter().find_map(|f| if let FeatureKind::MeshImport(m) = &f.kind { Some(m) } else { None }).expect("comando de malha");
        assert_eq!(chave(Path::new(&m.path)), chave(&root.join("pecas").join("casca.stl")));
        assert_eq!(std::fs::read(&m.path).unwrap(), b"solid x\nendsolid x\n");
        assert_eq!(r.copiados.len(), 2);
    }
}
