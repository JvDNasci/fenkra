// SPDX-License-Identifier: LGPL-2.1-or-later
//! Salvar como com referências (B93).
//!
//! O documento a salvar pode referenciar outros (montagem -> peças e subconjuntos, desenho ->
//! montagem). Salvar em outro lugar sem pensar nisso quebra links ou deixa a cópia amarrada aos
//! originais sem que ninguém perceba. Por isso o plano mostra uma **tabela de referências** e cada
//! linha decide: `Manter` (a cópia continua apontando para o original; só o caminho relativo é
//! recalculado) ou `Copiar` (o arquivo referenciado também é copiado, com prefixo/sufixo).
//!
//! Os quatro modos:
//!
//! | modo | identidade (uid) | arquivo original | quem fica aberto |
//! |---|---|---|---|
//! | `RenomearMover` | **mantém** | removido depois de tudo gravado; quem o usava é atualizado | o movido |
//! | `CopiaContinuar` | nova | intacto | o original |
//! | `CopiaAbrir` | nova | intacto | a cópia |
//! | `NovoDocumento` | nova, PN limpo, nome = nome do arquivo | intacto | a cópia |
//!
//! Por que a cópia ganha uid novo: o uid é a identidade do documento (B66). Dois arquivos com o mesmo
//! uid fazem o resolvedor de referências achar "vários candidatos" e pedir decisão ao usuário.
//!
//! Garantias: o plano não grava nada; `executar` valida tudo antes, grava com escrita atômica e, se
//! qualquer gravação falhar, **desfaz** o que já tinha gravado (apaga os arquivos criados e restaura
//! o conteúdo dos substituídos). O original só é removido (modo mover) depois de tudo gravado.

use crate::referencias::{caminho_relativo, Ref};
use crate::relacoes::{onde_usado, referencias_do_json, trocar_caminhos, TipoRef};
use crate::seguro::gravar_atomico;
use crate::{PResult, ProjectError};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModoSalvarComo {
    RenomearMover,
    CopiaContinuar,
    CopiaAbrir,
    NovoDocumento,
}

impl ModoSalvarComo {
    pub const TODOS: [ModoSalvarComo; 4] = [ModoSalvarComo::RenomearMover, ModoSalvarComo::CopiaContinuar, ModoSalvarComo::CopiaAbrir, ModoSalvarComo::NovoDocumento];

    pub fn rotulo(self) -> &'static str {
        match self {
            ModoSalvarComo::RenomearMover => "Renomear ou mover",
            ModoSalvarComo::CopiaContinuar => "Salvar cópia e continuar no original",
            ModoSalvarComo::CopiaAbrir => "Salvar cópia e abrir a cópia",
            ModoSalvarComo::NovoDocumento => "Novo documento a partir deste",
        }
    }

    pub fn descricao(self) -> &'static str {
        match self {
            ModoSalvarComo::RenomearMover => "O arquivo muda de nome ou de pasta e continua sendo o mesmo documento. Quem o usa é atualizado.",
            ModoSalvarComo::CopiaContinuar => "Grava uma cópia independente e você segue editando o original.",
            ModoSalvarComo::CopiaAbrir => "Grava uma cópia independente e passa a editar a cópia.",
            ModoSalvarComo::NovoDocumento => "Como a cópia, mas começa do zero: PN em branco e nome igual ao do arquivo novo.",
        }
    }

    /// O documento aberto passa a ser o arquivo de destino.
    pub fn abre_destino(self) -> bool {
        !matches!(self, ModoSalvarComo::CopiaContinuar)
    }

    fn mantem_uid(self) -> bool {
        matches!(self, ModoSalvarComo::RenomearMover)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AcaoRef {
    /// continua apontando para o arquivo original
    Manter,
    /// copia também o arquivo referenciado
    Copiar,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Conflito {
    Nenhum,
    /// já existe um arquivo com esse nome na pasta de destino
    ExisteNoDestino,
    /// outra linha da tabela vai gravar no mesmo nome
    MesmoNomeDeOutraLinha,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResolucaoConflito {
    /// acrescenta `_2`, `_3`... ao nome (padrão: nunca destrói nada)
    Renomear,
    /// grava por cima do arquivo existente
    Substituir,
    /// não copia esta linha: a referência fica apontando para o original
    Pular,
}

#[derive(Clone, Debug)]
pub struct LinhaRef {
    /// arquivo referenciado, caminho completo
    pub original: PathBuf,
    /// como estava gravado no primeiro documento que o cita
    pub gravado: String,
    pub tipo: TipoRef,
    /// quantas referências a este arquivo existem no conjunto
    pub usos: usize,
    /// profundidade na árvore de referências (1 = citado direto pelo documento)
    pub nivel: u32,
    /// o arquivo existe: sem ele só dá para manter a referência (e avisar)
    pub existe: bool,
    pub acao: AcaoRef,
    /// nome do arquivo no destino; editável na tabela
    pub nome_destino: String,
    pub resolucao: ResolucaoConflito,
}

#[derive(Clone, Debug)]
pub struct PlanoSalvarComo {
    pub origem: PathBuf,
    /// conteúdo em memória do documento principal (com edições ainda não salvas); `None` lê o arquivo
    pub texto_principal: Option<String>,
    pub destino: PathBuf,
    pub modo: ModoSalvarComo,
    /// pasta onde os referenciados copiados são gravados (padrão: a pasta do destino)
    pub pasta_refs: PathBuf,
    pub prefixo: String,
    pub sufixo: String,
    pub linhas: Vec<LinhaRef>,
    /// pastas onde procurar quem usa o documento (modo mover atualiza esses arquivos)
    pub pastas_dependentes: Vec<PathBuf>,
    /// permite gravar por cima de um destino que já existe
    pub substituir_destino: bool,
}

#[derive(Clone, Debug, Default)]
pub struct ResultadoSalvarComo {
    pub destino: PathBuf,
    pub abrir_destino: bool,
    /// (original, novo) de cada arquivo gravado, o principal primeiro
    pub gravados: Vec<(PathBuf, PathBuf)>,
    /// linhas cujo nome mudou por conflito: (nome pedido, nome gravado)
    pub renomeados_por_conflito: Vec<(String, String)>,
    /// arquivos de outros documentos atualizados para apontar ao novo lugar (modo mover)
    pub dependentes_atualizados: Vec<PathBuf>,
    /// referências que não puderam ser copiadas porque o arquivo não existe
    pub ausentes: Vec<PathBuf>,
}

fn absoluto(p: &Path) -> PathBuf {
    std::path::absolute(p).unwrap_or_else(|_| p.to_path_buf())
}

/// Chave estável de um arquivo para comparar caminhos (resolve `..`, diferenças de maiúscula no Windows).
fn chave(p: &Path) -> String {
    let base = std::fs::canonicalize(p).unwrap_or_else(|_| {
        let mut out = PathBuf::new();
        for c in absoluto(p).components() {
            match c {
                std::path::Component::ParentDir => {
                    out.pop();
                }
                std::path::Component::CurDir => {}
                c => out.push(c.as_os_str()),
            }
        }
        out
    });
    let t = base.to_string_lossy().replace('\\', "/");
    let t = t.strip_prefix("//?/").unwrap_or(&t).to_string();
    if cfg!(windows) { t.to_lowercase() } else { t }
}

fn nome_arquivo(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

fn com_prefixo_sufixo(nome: &str, prefixo: &str, sufixo: &str) -> String {
    let p = Path::new(nome);
    let stem = p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    match p.extension() {
        Some(e) => format!("{prefixo}{stem}{sufixo}.{}", e.to_string_lossy()),
        None => format!("{prefixo}{stem}{sufixo}"),
    }
}

fn com_contador(nome: &str, n: u32) -> String {
    com_prefixo_sufixo(nome, "", &format!("_{n}"))
}

impl PlanoSalvarComo {
    /// Monta o plano: percorre as referências do documento (e as dos referenciados) e cria a tabela.
    /// Nada é gravado. `texto` é o conteúdo em memória do documento, se houver edição não salva.
    pub fn planejar(origem: &Path, texto: Option<String>, destino: &Path, modo: ModoSalvarComo) -> PResult<PlanoSalvarComo> {
        let origem = absoluto(origem);
        let destino = absoluto(destino);
        let texto_principal = match texto {
            Some(t) => t,
            None => std::fs::read_to_string(&origem)?,
        };
        let mut linhas: Vec<LinhaRef> = vec![];
        let mut indice: HashMap<String, usize> = HashMap::new();
        let mut visitados: HashSet<String> = HashSet::new();
        visitados.insert(chave(&origem));
        // (texto do documento, pasta dele, nível dos que ele cita)
        let mut fila: Vec<(String, PathBuf, u32)> = vec![(texto_principal.clone(), origem.parent().map(Path::to_path_buf).unwrap_or_default(), 1)];
        while !fila.is_empty() {
            let (txt, dir, nivel) = fila.remove(0);
            for r in referencias_do_json(&txt) {
                let completo = absoluto(&Ref { uid: r.uid.clone(), caminho: r.caminho.clone() }.caminho_completo(&dir));
                let k = chave(&completo);
                if let Some(&i) = indice.get(&k) {
                    linhas[i].usos += 1;
                    continue;
                }
                let existe = completo.is_file();
                indice.insert(k.clone(), linhas.len());
                linhas.push(LinhaRef {
                    nome_destino: nome_arquivo(&completo),
                    original: completo.clone(),
                    gravado: r.caminho,
                    tipo: r.tipo,
                    usos: 1,
                    nivel,
                    existe,
                    acao: AcaoRef::Manter,
                    resolucao: ResolucaoConflito::Renomear,
                });
                if existe && visitados.insert(k) {
                    if let Ok(t) = std::fs::read_to_string(&completo) {
                        fila.push((t, completo.parent().map(Path::to_path_buf).unwrap_or_default(), nivel + 1));
                    }
                }
            }
        }
        let pasta = destino.parent().map(Path::to_path_buf).unwrap_or_default();
        Ok(PlanoSalvarComo {
            pastas_dependentes: vec![origem.parent().map(Path::to_path_buf).unwrap_or_default()],
            origem,
            texto_principal: Some(texto_principal),
            destino,
            modo,
            pasta_refs: pasta,
            prefixo: String::new(),
            sufixo: String::new(),
            linhas,
            substituir_destino: false,
        })
    }

    /// Marca (ou desmarca) todas as linhas que existem.
    pub fn marcar_todas(&mut self, acao: AcaoRef) {
        for l in self.linhas.iter_mut().filter(|l| l.existe) {
            l.acao = acao;
        }
    }

    /// Define prefixo e sufixo e recalcula o nome de destino de todas as linhas (descarta edições
    /// manuais de nome, que o usuário refaz depois se quiser).
    pub fn definir_prefixo_sufixo(&mut self, prefixo: &str, sufixo: &str) {
        self.prefixo = prefixo.into();
        self.sufixo = sufixo.into();
        for l in &mut self.linhas {
            l.nome_destino = com_prefixo_sufixo(&nome_arquivo(&l.original), prefixo, sufixo);
        }
    }

    fn destino_da_linha(&self, l: &LinhaRef) -> PathBuf {
        self.pasta_refs.join(&l.nome_destino)
    }

    /// Conflito de cada linha a copiar, na ordem da tabela.
    pub fn conflitos(&self) -> Vec<Conflito> {
        let mut usados: HashMap<String, usize> = HashMap::new();
        usados.insert(chave(&self.destino), 1);
        for l in self.linhas.iter().filter(|l| l.acao == AcaoRef::Copiar && l.existe) {
            *usados.entry(chave(&self.destino_da_linha(l))).or_default() += 1;
        }
        self.linhas
            .iter()
            .map(|l| {
                if l.acao != AcaoRef::Copiar || !l.existe {
                    return Conflito::Nenhum;
                }
                let d = self.destino_da_linha(l);
                let k = chave(&d);
                if usados.get(&k).copied().unwrap_or(0) > 1 {
                    Conflito::MesmoNomeDeOutraLinha
                } else if d.exists() {
                    Conflito::ExisteNoDestino
                } else {
                    Conflito::Nenhum
                }
            })
            .collect()
    }

    /// Motivos que impedem executar. Lista vazia = pode confirmar.
    pub fn problemas(&self) -> Vec<String> {
        let mut v = vec![];
        if self.destino.as_os_str().is_empty() || nome_arquivo(&self.destino).is_empty() {
            v.push("Escolha o arquivo de destino".to_string());
        }
        let mesmo = chave(&self.destino) == chave(&self.origem);
        if mesmo {
            v.push("O destino é o próprio arquivo de origem".to_string());
        }
        if !mesmo && self.destino.exists() && !self.substituir_destino {
            v.push("Já existe um arquivo com o nome do destino".to_string());
        }
        let origens: HashSet<String> = self.linhas.iter().map(|l| chave(&l.original)).chain([chave(&self.origem)]).collect();
        let conflitos = self.conflitos();
        for (l, c) in self.linhas.iter().zip(conflitos) {
            if l.acao == AcaoRef::Copiar && l.existe {
                if l.nome_destino.trim().is_empty() {
                    v.push(format!("A linha {} está sem nome de destino", nome_arquivo(&l.original)));
                }
                if l.resolucao == ResolucaoConflito::Substituir && origens.contains(&chave(&self.destino_da_linha(l))) {
                    v.push(format!("Substituir {} apagaria um arquivo de origem", l.nome_destino));
                }
                let _ = c;
            }
        }
        v
    }

    /// Executa o plano. Se algo falha no meio, desfaz o que gravou e devolve o erro.
    pub fn executar(&self) -> PResult<ResultadoSalvarComo> {
        let p = self.problemas();
        if !p.is_empty() {
            return Err(ProjectError::Recusado(p.join("; ")));
        }
        let mut tx = Transacao::default();
        match self.executar_interno(&mut tx) {
            Ok(r) => Ok(r),
            Err(e) => {
                tx.desfazer();
                Err(e)
            }
        }
    }

    fn executar_interno(&self, tx: &mut Transacao) -> PResult<ResultadoSalvarComo> {
        let mut res = ResultadoSalvarComo { destino: self.destino.clone(), abrir_destino: self.modo.abre_destino(), ..Default::default() };

        // 1. decide o destino final de cada arquivo a copiar, resolvendo conflitos
        let conflitos = self.conflitos();
        let mut ocupados: HashSet<String> = HashSet::new();
        ocupados.insert(chave(&self.destino));
        let mut copias: Vec<(&LinhaRef, PathBuf)> = vec![];
        for (l, c) in self.linhas.iter().zip(&conflitos) {
            if l.acao != AcaoRef::Copiar {
                continue;
            }
            if !l.existe {
                res.ausentes.push(l.original.clone());
                continue;
            }
            let mut dest = self.destino_da_linha(l);
            // a primeira linha a pedir um nome o leva; as seguintes (ou as que esbarram num arquivo
            // existente) seguem a resolução escolhida
            let ocupado = ocupados.contains(&chave(&dest));
            let existe = dest.exists();
            let _ = c;
            if ocupado || existe {
                match l.resolucao {
                    ResolucaoConflito::Pular => continue,
                    ResolucaoConflito::Substituir if existe && !ocupado => {}
                    _ => {
                        let mut n = 2;
                        loop {
                            let tentativa = self.pasta_refs.join(com_contador(&l.nome_destino, n));
                            if !tentativa.exists() && !ocupados.contains(&chave(&tentativa)) {
                                res.renomeados_por_conflito.push((l.nome_destino.clone(), nome_arquivo(&tentativa)));
                                dest = tentativa;
                                break;
                            }
                            n += 1;
                        }
                    }
                }
            }
            ocupados.insert(chave(&dest));
            copias.push((l, dest));
        }

        // 2. mapa original -> destino (o principal e os copiados)
        let mut mapa: HashMap<String, PathBuf> = HashMap::new();
        mapa.insert(chave(&self.origem), self.destino.clone());
        for (l, d) in &copias {
            mapa.insert(chave(&l.original), d.clone());
        }

        // 3. quem usa o original precisa ser achado antes de qualquer gravação (modo mover)
        let uid_origem = crate::relacoes::ler_cabecalho(&self.origem).map(|c| c.uid).unwrap_or_default();
        let dependentes = if self.modo == ModoSalvarComo::RenomearMover { onde_usado(&self.pastas_dependentes, &self.origem, &uid_origem) } else { vec![] };

        // 4. trabalhos: (origem, texto, destino, mantém uid)
        let mut trabalhos: Vec<(PathBuf, String, PathBuf, bool)> = vec![];
        let principal = match &self.texto_principal {
            Some(t) => t.clone(),
            None => std::fs::read_to_string(&self.origem)?,
        };
        trabalhos.push((self.origem.clone(), principal, self.destino.clone(), self.modo.mantem_uid()));
        for (l, d) in &copias {
            trabalhos.push((l.original.clone(), std::fs::read_to_string(&l.original)?, d.clone(), false));
        }
        // uids novos, decididos antes para que quem referencia a cópia receba o uid dela
        let mut uids: Vec<(String, String)> = vec![];
        for (orig, texto, _, mantem) in &trabalhos {
            let antigo = fenkra_core::formato::uid_do_json(texto);
            let _ = orig;
            if !mantem && !antigo.is_empty() {
                uids.push((antigo, fenkra_core::formato::novo_uid()));
            }
        }

        for (orig, texto, dest, _) in &trabalhos {
            let dir_orig = orig.parent().map(Path::to_path_buf).unwrap_or_default();
            let dir_novo = dest.parent().map(Path::to_path_buf).unwrap_or_default();
            let mut trocas: Vec<(String, String)> = vec![];
            let mut vistos: HashSet<String> = HashSet::new();
            for r in referencias_do_json(texto) {
                if !vistos.insert(r.caminho.clone()) {
                    continue;
                }
                let completo = absoluto(&Ref { uid: r.uid.clone(), caminho: r.caminho.clone() }.caminho_completo(&dir_orig));
                let alvo = mapa.get(&chave(&completo)).cloned().unwrap_or(completo);
                let novo = caminho_relativo(&absoluto(&dir_novo), &absoluto(&alvo));
                if novo != r.caminho {
                    trocas.push((r.caminho, novo));
                }
            }
            let (mut saida, _) = trocar_caminhos(texto, &trocas);
            for (velho, novo) in &uids {
                for sep in [": ", ":"] {
                    saida = saida.replace(&format!("\"uid\"{sep}\"{velho}\""), &format!("\"uid\"{sep}\"{novo}\""));
                }
            }
            if self.modo == ModoSalvarComo::NovoDocumento && *dest == self.destino {
                saida = limpar_identidade(&saida, dest);
            }
            tx.gravar(dest, saida.as_bytes())?;
            res.gravados.push((orig.clone(), dest.clone()));
        }

        // 5. modo mover: quem usava o original passa a apontar para o destino, depois o original sai
        if self.modo == ModoSalvarComo::RenomearMover {
            let mut feitos: HashSet<String> = HashSet::new();
            for u in &dependentes {
                if mapa.contains_key(&chave(&u.arquivo)) || !feitos.insert(chave(&u.arquivo)) {
                    continue;
                }
                let texto = std::fs::read_to_string(&u.arquivo)?;
                let dir = u.arquivo.parent().map(Path::to_path_buf).unwrap_or_default();
                let (novo, n) = trocar_caminhos(&texto, &[(u.caminho_gravado.clone(), caminho_relativo(&absoluto(&dir), &self.destino))]);
                if n > 0 {
                    tx.gravar(&u.arquivo, novo.as_bytes())?;
                    res.dependentes_atualizados.push(u.arquivo.clone());
                }
            }
            tx.remover(&self.origem)?;
        }
        Ok(res)
    }
}

/// Novo documento: zera o PN e usa o nome do arquivo como nome do documento.
fn limpar_identidade(texto: &str, destino: &Path) -> String {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(texto) else { return texto.to_string() };
    let lit = |s: &str| serde_json::to_string(s).unwrap_or_default();
    let mut saida = texto.to_string();
    if let Some(nome) = v.get("name").and_then(|n| n.as_str()) {
        let novo = destino.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        for sep in [": ", ":"] {
            saida = saida.replacen(&format!("\"name\"{sep}{}", lit(nome)), &format!("\"name\"{sep}{}", lit(&novo)), 1);
        }
    }
    if let Some(pn) = v.get("props").and_then(|p| p.get("pn")).and_then(|n| n.as_str()).filter(|p| !p.is_empty()) {
        for sep in [": ", ":"] {
            saida = saida.replacen(&format!("\"pn\"{sep}{}", lit(pn)), &format!("\"pn\"{sep}\"\""), 1);
        }
    }
    saida
}

/// Registro do que foi gravado, para desfazer em caso de falha.
#[derive(Default)]
struct Transacao {
    criados: Vec<PathBuf>,
    substituidos: Vec<(PathBuf, Vec<u8>)>,
    /// arquivos removidos pela operação (só o original no modo mover)
    removidos: Vec<(PathBuf, Vec<u8>)>,
}

impl Transacao {
    fn gravar(&mut self, p: &Path, dados: &[u8]) -> std::io::Result<()> {
        if p.exists() {
            self.substituidos.push((p.to_path_buf(), std::fs::read(p)?));
        } else {
            self.criados.push(p.to_path_buf());
        }
        gravar_atomico(p, dados)
    }

    fn remover(&mut self, p: &Path) -> std::io::Result<()> {
        let bytes = std::fs::read(p)?;
        std::fs::remove_file(p)?;
        self.removidos.push((p.to_path_buf(), bytes));
        Ok(())
    }

    fn desfazer(self) {
        for (p, b) in self.removidos {
            let _ = std::fs::write(p, b);
        }
        for (p, b) in self.substituidos.into_iter().rev() {
            let _ = std::fs::write(p, b);
        }
        for p in self.criados {
            let _ = std::fs::remove_file(&p);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(nome: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("fenkra-sc-{nome}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn escrever(dir: &Path, rel: &str, json: &str) -> PathBuf {
        let p = dir.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, json).unwrap();
        p
    }

    fn peca(uid: &str, nome: &str) -> String {
        format!(r#"{{"format_version": 1, "uid": "{uid}", "name": "{nome}", "props": {{"pn": "PN-{nome}"}}}}"#)
    }

    fn montagem(uid: &str, comps: &[(u64, &str)]) -> String {
        let nos: Vec<String> = comps.iter().map(|(id, p)| format!(r#"{{"id": {id}, "name": "c{id}", "kind": {{"Part": {{"path": "{p}"}}}}}}"#)).collect();
        format!(r#"{{"format_version": 1, "uid": "{uid}", "name": "Conj", "nodes": [{}]}}"#, nos.join(","))
    }

    /// d/conj.fka -> pecas/a.fkp, pecas/b.fkp
    fn cenario(nome: &str) -> (PathBuf, PathBuf) {
        let d = tmp(nome);
        escrever(&d, "pecas/a.fkp", &peca("ua", "A"));
        escrever(&d, "pecas/b.fkp", &peca("ub", "B"));
        let c = escrever(&d, "conj.fka", &montagem("uc", &[(1, "pecas/a.fkp"), (2, "pecas/b.fkp")]));
        (d, c)
    }

    #[test]
    fn tabela_lista_as_referencias_diretas_e_as_dos_subconjuntos() {
        let d = tmp("tabela");
        escrever(&d, "pecas/p.fkp", &peca("up", "P"));
        escrever(&d, "sub.fka", &montagem("us", &[(1, "pecas/p.fkp")]));
        let c = escrever(&d, "conj.fka", &format!(r#"{{"name":"C","uid":"uc","nodes":[{{"id":1,"name":"S","kind":{{"SubAssembly":{{"path":"sub.fka"}}}}}},{{"id":2,"name":"P","kind":{{"Part":{{"path":"pecas/p.fkp"}}}}}}]}}"#));
        let plano = PlanoSalvarComo::planejar(&c, None, &d.join("novo/conj2.fka"), ModoSalvarComo::CopiaAbrir).unwrap();
        assert_eq!(plano.linhas.len(), 2, "p.fkp aparece uma vez só, mesmo citada por dois");
        let p = plano.linhas.iter().find(|l| l.nome_destino == "p.fkp").unwrap();
        assert_eq!((p.usos, p.nivel, p.existe, p.acao), (2, 1, true, AcaoRef::Manter));
        assert_eq!(plano.linhas.iter().find(|l| l.nome_destino == "sub.fka").unwrap().tipo, TipoRef::Subconjunto);
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn copia_mantendo_referencias_so_recalcula_o_caminho_relativo() {
        let (d, c) = cenario("copia-manter");
        let dest = d.join("outra/pasta/conj2.fka");
        let plano = PlanoSalvarComo::planejar(&c, None, &dest, ModoSalvarComo::CopiaContinuar).unwrap();
        assert!(plano.problemas().is_empty());
        let r = plano.executar().unwrap();
        assert!(!r.abrir_destino);
        let novo = std::fs::read_to_string(&dest).unwrap();
        assert!(novo.contains("../../pecas/a.fkp") && novo.contains("../../pecas/b.fkp"), "{novo}");
        // as referências da cópia resolvem para os originais
        let diag = crate::relacoes::diagnosticar(&dest, &[d.clone()]);
        assert!(diag.iter().all(|x| x.veredito == crate::relacoes::Veredito::Ok), "{diag:?}");
        // o original ficou intacto, e a cópia tem identidade nova
        assert!(std::fs::read_to_string(&c).unwrap().contains("\"uid\": \"uc\""));
        let uid_novo = fenkra_core::formato::uid_do_json(&novo);
        assert!(!uid_novo.is_empty() && uid_novo != "uc");
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn copiar_referenciados_com_prefixo_e_sufixo_e_uid_novo() {
        let (d, c) = cenario("copia-tudo");
        let dest = d.join("v2/conj.fka");
        let mut plano = PlanoSalvarComo::planejar(&c, None, &dest, ModoSalvarComo::CopiaAbrir).unwrap();
        plano.marcar_todas(AcaoRef::Copiar);
        plano.definir_prefixo_sufixo("v2_", "_rev");
        assert_eq!(plano.linhas[0].nome_destino, "v2_a_rev.fkp");
        let r = plano.executar().unwrap();
        assert!(r.abrir_destino);
        assert_eq!(r.gravados.len(), 3);
        let novo = std::fs::read_to_string(&dest).unwrap();
        assert!(novo.contains("v2_a_rev.fkp") && novo.contains("v2_b_rev.fkp"));
        assert!(!novo.contains("pecas/a.fkp"));
        let copia_a = d.join("v2/v2_a_rev.fkp");
        assert!(copia_a.is_file());
        assert_ne!(crate::referencias::ler_uid(&copia_a), "ua", "peça copiada tem identidade própria");
        assert!(std::fs::read_to_string(d.join("pecas/a.fkp")).unwrap().contains("\"ua\""), "o original não muda");
        let diag = crate::relacoes::diagnosticar(&dest, &[d.clone()]);
        assert!(diag.iter().all(|x| x.veredito == crate::relacoes::Veredito::Ok), "{diag:?}");
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn conflito_e_listado_e_por_padrao_renomeia_sem_destruir() {
        let (d, c) = cenario("conflito");
        escrever(&d, "v2/a.fkp", "ARQUIVO DO USUARIO");
        let mut plano = PlanoSalvarComo::planejar(&c, None, &d.join("v2/conj.fka"), ModoSalvarComo::CopiaAbrir).unwrap();
        plano.marcar_todas(AcaoRef::Copiar);
        assert_eq!(plano.conflitos(), vec![Conflito::ExisteNoDestino, Conflito::Nenhum]);
        let r = plano.executar().unwrap();
        assert_eq!(std::fs::read_to_string(d.join("v2/a.fkp")).unwrap(), "ARQUIVO DO USUARIO", "o arquivo do usuário não foi tocado");
        assert_eq!(r.renomeados_por_conflito, vec![("a.fkp".to_string(), "a_2.fkp".to_string())]);
        assert!(std::fs::read_to_string(d.join("v2/conj.fka")).unwrap().contains("a_2.fkp"));
        // duas linhas com o mesmo nome de destino também são conflito
        let mut p2 = PlanoSalvarComo::planejar(&c, None, &d.join("v3/conj.fka"), ModoSalvarComo::CopiaAbrir).unwrap();
        p2.marcar_todas(AcaoRef::Copiar);
        p2.linhas[1].nome_destino = "a.fkp".into();
        assert_eq!(p2.conflitos(), vec![Conflito::MesmoNomeDeOutraLinha, Conflito::MesmoNomeDeOutraLinha]);
        p2.executar().unwrap();
        assert!(d.join("v3/a.fkp").is_file() && d.join("v3/a_2.fkp").is_file());
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn substituir_nunca_apaga_um_arquivo_de_origem() {
        let (d, c) = cenario("subst");
        // destino de a.fkp = o próprio a.fkp original
        let mut plano = PlanoSalvarComo::planejar(&c, None, &d.join("conj_copia.fka"), ModoSalvarComo::CopiaContinuar).unwrap();
        plano.marcar_todas(AcaoRef::Copiar);
        plano.pasta_refs = d.join("pecas");
        plano.linhas[0].resolucao = ResolucaoConflito::Substituir;
        let p = plano.problemas();
        assert!(p.iter().any(|m| m.contains("apagaria um arquivo de origem")), "{p:?}");
        assert!(plano.executar().is_err());
        assert!(std::fs::read_to_string(d.join("pecas/a.fkp")).unwrap().contains("\"ua\""));
        assert!(!d.join("conj_copia.fka").exists(), "nada foi gravado");
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn mover_mantem_uid_remove_o_original_e_atualiza_quem_usa() {
        let (d, c) = cenario("mover");
        let alvo = d.join("pecas/a.fkp");
        let desenho = escrever(&d, "des.fkd", r#"{"name":"D","views":[{"id":1,"name":"F","source":{"path":"pecas/a.fkp"}}]}"#);
        let dest = d.join("comum/eixo.fkp");
        let plano = PlanoSalvarComo::planejar(&alvo, None, &dest, ModoSalvarComo::RenomearMover).unwrap();
        let mut plano = plano;
        plano.pastas_dependentes = vec![d.clone()];
        let r = plano.executar().unwrap();
        assert!(!alvo.exists(), "o original saiu");
        assert_eq!(crate::referencias::ler_uid(&dest), "ua", "continua sendo o mesmo documento");
        assert_eq!(r.dependentes_atualizados.len(), 2);
        assert!(std::fs::read_to_string(&c).unwrap().contains("comum/eixo.fkp"));
        assert!(std::fs::read_to_string(&desenho).unwrap().contains("comum/eixo.fkp"));
        assert!(std::fs::read_to_string(&c).unwrap().contains("pecas/b.fkp"), "as outras referências não mudam");
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn falha_no_meio_desfaz_tudo() {
        let (d, c) = cenario("desfaz");
        let dest = d.join("v2/conj.fka");
        let mut plano = PlanoSalvarComo::planejar(&c, None, &dest, ModoSalvarComo::CopiaAbrir).unwrap();
        plano.marcar_todas(AcaoRef::Copiar);
        // a segunda cópia não consegue ser gravada: o lugar dela é uma pasta
        std::fs::create_dir_all(d.join("v2/b.fkp")).unwrap();
        plano.linhas[1].resolucao = ResolucaoConflito::Substituir;
        assert!(plano.executar().is_err());
        assert!(!dest.exists(), "o principal gravado antes foi desfeito");
        assert!(!d.join("v2/a.fkp").exists(), "a primeira cópia também");
        assert!(d.join("pecas/a.fkp").is_file() && d.join("pecas/b.fkp").is_file());
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn novo_documento_zera_pn_e_usa_o_nome_do_arquivo() {
        let d = tmp("novo");
        let p = escrever(&d, "base.fkp", &peca("u1", "Base"));
        let dest = d.join("derivada.fkp");
        PlanoSalvarComo::planejar(&p, None, &dest, ModoSalvarComo::NovoDocumento).unwrap().executar().unwrap();
        let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&dest).unwrap()).unwrap();
        assert_eq!(v["name"], "derivada");
        assert_eq!(v["props"]["pn"], "");
        assert_ne!(v["uid"], "u1");
        // a cópia simples mantém nome e PN
        let dest2 = d.join("copia.fkp");
        PlanoSalvarComo::planejar(&p, None, &dest2, ModoSalvarComo::CopiaContinuar).unwrap().executar().unwrap();
        let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&dest2).unwrap()).unwrap();
        assert_eq!((v["name"].as_str(), v["props"]["pn"].as_str()), (Some("Base"), Some("PN-Base")));
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn conteudo_em_memoria_vale_mais_que_o_arquivo() {
        let d = tmp("memoria");
        let p = escrever(&d, "base.fkp", &peca("u1", "Base"));
        let editado = peca("u1", "Editado");
        let dest = d.join("copia.fkp");
        PlanoSalvarComo::planejar(&p, Some(editado), &dest, ModoSalvarComo::CopiaContinuar).unwrap().executar().unwrap();
        assert!(std::fs::read_to_string(&dest).unwrap().contains("Editado"));
        assert!(std::fs::read_to_string(&p).unwrap().contains("\"Base\""), "o arquivo em disco não foi tocado");
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn destino_existente_ou_igual_a_origem_e_recusado() {
        let (d, c) = cenario("destino");
        let p = PlanoSalvarComo::planejar(&c, None, &c, ModoSalvarComo::CopiaContinuar).unwrap();
        assert!(p.problemas().iter().any(|m| m.contains("próprio arquivo")));
        let mut p = PlanoSalvarComo::planejar(&c, None, &d.join("pecas/a.fkp"), ModoSalvarComo::CopiaContinuar).unwrap();
        assert!(p.problemas().iter().any(|m| m.contains("Já existe")));
        p.substituir_destino = true;
        assert!(p.problemas().is_empty());
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn novo_documento_com_documento_real_troca_nome_e_pn_do_topo() {
        let d = tmp("novo-real");
        let mut doc = fenkra_core::Document::new("Base");
        doc.props.pn = "FK-0042".into();
        let p = escrever(&d, "base.fkp", &doc.to_json());
        let dest = d.join("derivada.fkp");
        PlanoSalvarComo::planejar(&p, None, &dest, ModoSalvarComo::NovoDocumento).unwrap().executar().unwrap();
        let novo = fenkra_core::Document::from_json(&std::fs::read_to_string(&dest).unwrap()).unwrap();
        assert_eq!((novo.name.as_str(), novo.props.pn.as_str()), ("derivada", ""));
        assert_ne!(novo.uid.0, doc.uid.0, "identidade nova");
        // a cópia simples mantém tudo e só troca a identidade
        let dest2 = d.join("copia.fkp");
        PlanoSalvarComo::planejar(&p, None, &dest2, ModoSalvarComo::CopiaAbrir).unwrap().executar().unwrap();
        let copia = fenkra_core::Document::from_json(&std::fs::read_to_string(&dest2).unwrap()).unwrap();
        assert_eq!((copia.name.as_str(), copia.props.pn.as_str()), ("Base", "FK-0042"));
        assert_ne!(copia.uid.0, doc.uid.0);
        let _ = std::fs::remove_dir_all(d);
    }
}
