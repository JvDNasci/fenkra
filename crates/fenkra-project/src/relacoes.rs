// SPDX-License-Identifier: LGPL-2.1-or-later
//! Relações entre documentos: o que cada arquivo referencia, onde uma peça é usada e como reparar
//! referências quebradas (B94).
//!
//! Três ideias sustentam o módulo:
//! - **Referências vêm do JSON, sem abrir o documento** (nem recalcular nada): montagem guarda
//!   `kind: {Part: {path}}`/`{SubAssembly: {path}}` em cada nó, desenho guarda `source.path` em cada
//!   vista. Uma peça guarda os links em `external[]`, cada um com `path`, `uid` e `publication`:
//!   peça linkada, peça derivada (corpo trazido de outra) e o furo que a montagem manda (publicação
//!   com [`PREFIXO_FURO`], que aponta para a montagem). A malha importada fica em
//!   `kind: {MeshImport: {path}}` e as imagens de textura em `textures[].path`. Ler assim é barato e
//!   funciona mesmo para arquivo de formato futuro.
//! - **Cabeçalho** ([`ler_cabecalho`]) lê só `uid`, `name`, `format_version` e `props.pn/description`;
//!   o resto do arquivo é saltado pelo leitor sem montar nenhuma estrutura em memória.
//! - **Reparar é em duas etapas**: [`diagnosticar`] só olha e propõe; [`aplicar_reparos`] só troca o
//!   texto do caminho (o restante do arquivo fica byte a byte igual) e grava de forma atômica.

use crate::referencias::{caminho_relativo, resolver, Ref, Resolucao};
use crate::seguro::gravar_atomico;
use serde::Deserialize;
use serde_json::Value;
use std::path::{Path, PathBuf};

/// Publicação dos furos que a montagem manda para a peça (`fenkra_montagem::furos::PREFIXO`). O link
/// aponta para a montagem, que não é dependência da peça: quem copia ou empacota a peça não segue
/// esse caminho, só o reaponta.
pub const PREFIXO_FURO: &str = "montagem:furo:";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TipoRef {
    /// componente do tipo peça numa montagem
    Peca,
    /// componente do tipo subconjunto numa montagem
    Subconjunto,
    /// vista de desenho
    Vista,
    /// link entre peças: `external[]` com `path` + `uid` (peça linkada ou derivada)
    Link,
    /// furo mandado pela montagem: `external[]` cuja publicação começa com [`PREFIXO_FURO`]
    FuroMontagem,
    /// arquivo de malha lido pelo comando de importar malha
    Malha,
    /// imagem de textura de uma aparência
    Textura,
}

impl TipoRef {
    pub fn rotulo(self) -> &'static str {
        match self {
            TipoRef::Peca => "Peça",
            TipoRef::Subconjunto => "Subconjunto",
            TipoRef::Vista => "Vista de desenho",
            TipoRef::Link => "Link",
            TipoRef::FuroMontagem => "Furo mandado pela montagem",
            TipoRef::Malha => "Malha",
            TipoRef::Textura => "Textura",
        }
    }

    /// O alvo é um documento do Fenkra (`.fkp`, `.fka`, `.fkd`). Malha e textura são arquivos de
    /// outro formato: copiados byte a byte, sem procurar referências dentro.
    pub fn eh_documento(self) -> bool {
        !matches!(self, TipoRef::Malha | TipoRef::Textura)
    }

    /// Quem copia ou empacota o documento leva também o alvo e segue as referências dele. O furo da
    /// montagem é o contrário de uma dependência (a montagem é que usa a peça): segui-lo traria a
    /// montagem e todas as outras peças dela.
    pub fn e_dependencia(self) -> bool {
        self != TipoRef::FuroMontagem
    }

    /// Caminho gravado como texto absoluto: o comando lê malha e textura pelo caminho como está, a
    /// partir da pasta de trabalho do programa, então um caminho relativo à peça quebraria.
    pub fn caminho_absoluto(self) -> bool {
        !self.eh_documento()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RefEncontrada {
    pub tipo: TipoRef,
    /// caminho exatamente como está gravado no arquivo
    pub caminho: String,
    /// uid gravado junto (só os links guardam)
    pub uid: String,
    /// id do nó na árvore da montagem (para a interface selecionar o componente)
    pub no: Option<u64>,
    /// nome do nó ou da vista
    pub nome: String,
}

impl RefEncontrada {
    /// Arquivo no disco a que a referência leva, visto da pasta do documento que a grava.
    pub fn arquivo(&self, dir: &Path) -> PathBuf {
        arquivo_da_ref(dir, self.tipo, &self.caminho)
    }
}

/// Regra única para achar o alvo de uma referência (usada também pela cópia para o projeto, B445):
/// - caminho absoluto vale como está;
/// - documento e link: relativo à pasta do documento que grava (mesma regra de
///   `fenkra_montagem::links::resolver`);
/// - malha e textura: ao lado do documento ou, se lá não existir, relativo à pasta de trabalho, que é
///   como o comando as lê.
pub fn arquivo_da_ref(dir: &Path, tipo: TipoRef, caminho: &str) -> PathBuf {
    let p = Path::new(caminho);
    if p.is_absolute() {
        return p.to_path_buf();
    }
    let perto = dir.join(p);
    if !tipo.eh_documento() && !perto.is_file() && p.is_file() {
        return std::path::absolute(p).unwrap_or_else(|_| p.to_path_buf());
    }
    perto
}

fn caminho_de(v: &Value) -> Option<&str> {
    v.get("path").and_then(Value::as_str).filter(|s| !s.is_empty())
}

fn varrer(v: &Value, out: &mut Vec<RefEncontrada>) {
    match v {
        Value::Object(m) => {
            // nó de montagem: o tipo mora em `kind`
            if let Some(Value::Object(k)) = m.get("kind") {
                for (chave, tipo) in [("Part", TipoRef::Peca), ("SubAssembly", TipoRef::Subconjunto), ("MeshImport", TipoRef::Malha)] {
                    if let Some(c) = k.get(chave).and_then(caminho_de) {
                        out.push(RefEncontrada {
                            tipo,
                            caminho: c.into(),
                            uid: String::new(),
                            no: if tipo == TipoRef::Malha { None } else { m.get("id").and_then(Value::as_u64) },
                            nome: m.get("name").and_then(Value::as_str).unwrap_or_default().into(),
                        });
                    }
                }
            }
            // vista de desenho: `source.path`
            if let Some(c) = m.get("source").and_then(caminho_de) {
                out.push(RefEncontrada {
                    tipo: TipoRef::Vista,
                    caminho: c.into(),
                    uid: String::new(),
                    no: None,
                    nome: m.get("name").and_then(Value::as_str).unwrap_or_default().into(),
                });
            }
            // link de peça: item de `external[]`, com `publication` + `path` + `uid`
            if let (Some(p), Some(c)) = (m.get("publication").and_then(Value::as_str), caminho_de(v)) {
                out.push(RefEncontrada {
                    tipo: if p.starts_with(PREFIXO_FURO) { TipoRef::FuroMontagem } else { TipoRef::Link },
                    caminho: c.into(),
                    uid: m.get("uid").and_then(Value::as_str).unwrap_or_default().into(),
                    no: None,
                    nome: m.get("name").and_then(Value::as_str).unwrap_or_default().into(),
                });
            }
            // imagens de textura das aparências
            if let Some(Value::Array(t)) = m.get("textures") {
                for x in t {
                    if let Some(c) = caminho_de(x) {
                        out.push(RefEncontrada { tipo: TipoRef::Textura, caminho: c.into(), uid: String::new(), no: None, nome: x.get("name").and_then(Value::as_str).unwrap_or_default().into() });
                    }
                }
            }
            // forma antiga de link: par uid + caminho
            if let (Some(u), Some(c)) = (m.get("uid").and_then(Value::as_str), m.get("caminho").and_then(Value::as_str)) {
                if !c.is_empty() {
                    out.push(RefEncontrada { tipo: TipoRef::Link, caminho: c.into(), uid: u.into(), no: None, nome: String::new() });
                }
            }
            for (k, x) in m {
                if k != "kind" {
                    varrer(x, out);
                }
            }
        }
        Value::Array(a) => a.iter().for_each(|x| varrer(x, out)),
        _ => {}
    }
}

/// Referências de um JSON já lido, na ordem do arquivo.
pub fn referencias_do_json(texto: &str) -> Vec<RefEncontrada> {
    let mut out = vec![];
    if let Ok(v) = serde_json::from_str::<Value>(texto) {
        varrer(&v, &mut out);
    }
    out
}

/// Referências de um arquivo `.fkp`, `.fka` ou `.fkd`.
pub fn referencias_de(arquivo: &Path) -> Vec<RefEncontrada> {
    std::fs::read_to_string(arquivo).map(|s| referencias_do_json(&s)).unwrap_or_default()
}

#[derive(Deserialize, Default)]
struct Props {
    #[serde(default)]
    pn: String,
    #[serde(default)]
    description: String,
}

#[derive(Deserialize, Default)]
struct Cab {
    #[serde(default)]
    uid: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    format_version: u32,
    #[serde(default)]
    props: Option<Props>,
}

/// O que se sabe de um documento sem abri-lo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cabecalho {
    pub uid: String,
    pub nome: String,
    pub pn: String,
    pub descricao: String,
    pub format_version: u32,
}

/// Lê só o cabeçalho. O leitor do `serde_json` salta os demais campos sem construir valores, então o
/// custo é percorrer o texto uma vez, sem a árvore de objetos nem a migração do documento.
pub fn ler_cabecalho(arquivo: &Path) -> Option<Cabecalho> {
    let bytes = std::fs::read(arquivo).ok()?;
    let c: Cab = serde_json::from_slice(&bytes).ok()?;
    let p = c.props.unwrap_or_default();
    Some(Cabecalho { uid: c.uid, nome: c.name, pn: p.pn, descricao: p.description, format_version: c.format_version })
}

fn eh_documento(p: &Path) -> bool {
    matches!(p.extension().and_then(|x| x.to_str()), Some("fkp" | "fka" | "fkd"))
}

/// Arquivos de documento sob `dir`, até `fundo` níveis, pulando pastas ocultas (`.versoes`, `.originais`).
pub fn listar_documentos(dir: &Path, fundo: u32, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            if fundo > 0 && !e.file_name().to_string_lossy().starts_with('.') {
                listar_documentos(&p, fundo - 1, out);
            }
        } else if eh_documento(&p) {
            out.push(p);
        }
    }
}

fn mesmo_arquivo(a: &Path, b: &Path) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(x), Ok(y)) => x == y,
        _ => false,
    }
}

/// Um documento que usa o alvo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Uso {
    pub arquivo: PathBuf,
    /// nome do documento que usa (do cabeçalho)
    pub nome: String,
    pub tipo: TipoRef,
    /// nó da montagem que referencia o alvo: a interface seleciona este componente na árvore
    pub no: Option<u64>,
    pub nome_no: String,
    pub caminho_gravado: String,
}

/// Quem usa `alvo` dentro de `pastas`. Uma referência conta se o caminho gravado leva ao alvo ou, nas
/// que guardam uid, se o uid é o do alvo (arquivo movido ou renomeado ainda é o mesmo documento).
pub fn onde_usado(pastas: &[PathBuf], alvo: &Path, uid_alvo: &str) -> Vec<Uso> {
    let mut arquivos = vec![];
    for p in pastas {
        listar_documentos(p, 6, &mut arquivos);
    }
    arquivos.sort();
    arquivos.dedup();
    let mut usos = vec![];
    for arq in arquivos.into_iter().filter(|a| !mesmo_arquivo(a, alvo)) {
        let refs = referencias_de(&arq);
        if refs.is_empty() {
            continue;
        }
        let dir = arq.parent().map(Path::to_path_buf).unwrap_or_default();
        let nome = ler_cabecalho(&arq).map(|c| c.nome).unwrap_or_default();
        for r in refs {
            let completo = r.arquivo(&dir);
            let por_caminho = mesmo_arquivo(&completo, alvo);
            let por_uid = !uid_alvo.is_empty() && r.uid == uid_alvo;
            if por_caminho || por_uid {
                usos.push(Uso { arquivo: arq.clone(), nome: nome.clone(), tipo: r.tipo, no: r.no, nome_no: r.nome.clone(), caminho_gravado: r.caminho });
            }
        }
    }
    usos
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Veredito {
    /// o caminho leva ao documento certo
    Ok,
    /// o arquivo sumiu do caminho, mas existe um só candidato: reparo sugerido
    Movido(PathBuf),
    /// vários candidatos: o usuário escolhe
    Varios(Vec<PathBuf>),
    /// o arquivo no caminho tem outro uid (outra peça com o mesmo nome)
    UidDiferente { alternativa: Option<PathBuf> },
    /// nada encontrado
    Perdido,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostico {
    pub referencia: RefEncontrada,
    pub veredito: Veredito,
}

fn procurar_nome(dir: &Path, nome: &str, fundo: u32, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        let n = e.file_name().to_string_lossy().into_owned();
        if p.is_dir() {
            if fundo > 0 && !n.starts_with('.') {
                procurar_nome(&p, nome, fundo - 1, out);
            }
        } else if eh_documento(&p) && n.eq_ignore_ascii_case(nome) {
            out.push(p);
        }
    }
}

/// Confere cada referência do arquivo. Quando o arquivo some do caminho, procura pelo uid (links) ou,
/// nas referências antigas que só guardam caminho, por um documento de mesmo nome de arquivo.
pub fn diagnosticar(arquivo: &Path, busca: &[PathBuf]) -> Vec<Diagnostico> {
    let dir = arquivo.parent().map(Path::to_path_buf).unwrap_or_default();
    referencias_de(arquivo)
        .into_iter()
        .map(|r| {
            let veredito = if r.uid.is_empty() {
                let alvo = r.arquivo(&dir);
                if alvo.is_file() {
                    Veredito::Ok
                } else {
                    let nome = Path::new(&r.caminho.replace('\\', "/")).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                    let mut v = vec![];
                    for b in busca {
                        procurar_nome(b, &nome, 6, &mut v);
                    }
                    v.sort();
                    v.dedup();
                    match v.len() {
                        0 => Veredito::Perdido,
                        1 => Veredito::Movido(v.remove(0)),
                        _ => Veredito::Varios(v),
                    }
                }
            } else {
                match resolver(&dir, &Ref { uid: r.uid.clone(), caminho: r.caminho.clone() }, busca) {
                    Resolucao::Encontrado(_) => Veredito::Ok,
                    Resolucao::Movido(p) => Veredito::Movido(p),
                    Resolucao::Varios(v) => Veredito::Varios(v),
                    Resolucao::UidDiferente { alternativa, .. } => Veredito::UidDiferente { alternativa },
                    Resolucao::NaoEncontrado => Veredito::Perdido,
                }
            };
            Diagnostico { referencia: r, veredito }
        })
        .collect()
}

/// Troca, no texto de um JSON, cada caminho antigo pelo novo, em uma passada só (por isso `a -> b` e
/// `b -> c` na mesma chamada não se encadeiam). Só mexe em valores das chaves `path` e `caminho`.
/// Devolve o texto novo e quantas ocorrências trocou.
pub fn trocar_caminhos(json: &str, trocas: &[(String, String)]) -> (String, usize) {
    let mut texto = json.to_string();
    let mut total = 0;
    // passo 1: cada literal antigo vira uma marca única; passo 2: a marca vira o literal novo
    let literal = |s: &str| serde_json::to_string(s).unwrap_or_default();
    for (i, (velho, _)) in trocas.iter().enumerate() {
        for chave in ["path", "caminho"] {
            for sep in [": ", ":"] {
                let padrao = format!("\"{chave}\"{sep}{}", literal(velho));
                let n = texto.matches(&padrao).count();
                if n > 0 {
                    texto = texto.replace(&padrao, &format!("\"{chave}\"{sep}\u{1}{i}\u{2}"));
                    total += n;
                }
            }
        }
    }
    for (i, (_, novo)) in trocas.iter().enumerate() {
        texto = texto.replace(&format!("\u{1}{i}\u{2}"), &literal(novo));
    }
    (texto, total)
}

/// Aplica reparos num arquivo: pares (caminho gravado, arquivo novo). O caminho novo é gravado
/// relativo à pasta do arquivo. Devolve quantas referências mudaram. Nada é gravado se nada mudou.
pub fn aplicar_reparos(arquivo: &Path, reparos: &[(String, PathBuf)]) -> std::io::Result<usize> {
    let texto = std::fs::read_to_string(arquivo)?;
    let dir = arquivo.parent().map(Path::to_path_buf).unwrap_or_default();
    let trocas: Vec<(String, String)> = reparos.iter().map(|(v, novo)| (v.clone(), caminho_relativo(&dir, novo))).collect();
    let (novo_texto, n) = trocar_caminhos(&texto, &trocas);
    if n > 0 {
        gravar_atomico(arquivo, novo_texto.as_bytes())?;
    }
    Ok(n)
}

/// Repara de uma vez todas as referências com candidato único (`Movido`). Devolve os pares aplicados.
pub fn reparar_automatico(arquivo: &Path, busca: &[PathBuf]) -> std::io::Result<Vec<(String, PathBuf)>> {
    let reparos: Vec<(String, PathBuf)> = diagnosticar(arquivo, busca)
        .into_iter()
        .filter_map(|d| match d.veredito {
            Veredito::Movido(p) => Some((d.referencia.caminho, p)),
            _ => None,
        })
        .collect();
    aplicar_reparos(arquivo, &reparos)?;
    Ok(reparos)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(nome: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("fenkra-rel-{nome}-{}", std::process::id()));
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
        format!(r#"{{"format_version": 1, "uid": "{uid}", "name": "{nome}", "props": {{"pn": "PN-{nome}", "description": "d"}}, "features": [1,2,3]}}"#)
    }

    fn montagem(componentes: &[(u64, &str, &str)]) -> String {
        let nos: Vec<String> = componentes
            .iter()
            .map(|(id, nome, path)| format!(r#"{{"id": {id}, "name": "{nome}", "kind": {{"Part": {{"path": {}}}}}}}"#, serde_json::to_string(path).unwrap()))
            .collect();
        format!(r#"{{"format_version": 1, "name": "Conj", "nodes": [{}]}}"#, nos.join(","))
    }

    #[test]
    fn referencias_de_montagem_desenho_e_link() {
        let m = montagem(&[(1, "Base", "pecas/base.fkp"), (2, "Tampa", "../comum/tampa.fkp")]);
        let r = referencias_do_json(&m);
        assert_eq!(r.len(), 2);
        assert_eq!((r[0].tipo, r[0].no, r[0].nome.as_str(), r[0].caminho.as_str()), (TipoRef::Peca, Some(1), "Base", "pecas/base.fkp"));
        assert_eq!(r[1].no, Some(2));
        let d = r#"{"name":"Des","views":[{"id":3,"name":"Frente","source":{"path":"conj.fka","hidden":[]}}]}"#;
        let r = referencias_do_json(d);
        assert_eq!((r.len(), r[0].tipo, r[0].caminho.as_str()), (1, TipoRef::Vista, "conj.fka"));
        let l = r#"{"name":"P","links":[{"uid":"abc","caminho":"outra.fkp"}]}"#;
        let r = referencias_do_json(l);
        assert_eq!((r[0].tipo, r[0].uid.as_str()), (TipoRef::Link, "abc"));
        assert!(referencias_do_json("não é json").is_empty());
    }

    #[test]
    fn cabecalho_le_sem_montar_o_documento() {
        let d = tmp("cab");
        let p = escrever(&d, "a.fkp", &peca("u1", "Eixo"));
        let c = ler_cabecalho(&p).unwrap();
        assert_eq!((c.uid.as_str(), c.nome.as_str(), c.pn.as_str(), c.format_version), ("u1", "Eixo", "PN-Eixo", 1));
        // arquivo antigo sem uid nem props continua tendo cabeçalho
        let q = escrever(&d, "b.fkp", r#"{"name": "Velha"}"#);
        let c = ler_cabecalho(&q).unwrap();
        assert_eq!((c.uid.as_str(), c.nome.as_str(), c.pn.as_str()), ("", "Velha", ""));
        assert!(ler_cabecalho(&d.join("nao-existe.fkp")).is_none());
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn onde_usado_acha_montagem_e_desenho_e_diz_o_no() {
        let d = tmp("usado");
        let alvo = escrever(&d, "pecas/base.fkp", &peca("u-base", "Base"));
        escrever(&d, "pecas/outra.fkp", &peca("u-outra", "Outra"));
        escrever(&d, "conj1.fka", &montagem(&[(1, "Tampa", "pecas/outra.fkp"), (7, "Base", "pecas/base.fkp")]));
        escrever(&d, "sub/conj2.fka", &montagem(&[(3, "Base B", "../pecas/base.fkp")]));
        escrever(&d, "desenho.fkd", r#"{"name":"Des","views":[{"id":1,"name":"Frente","source":{"path":"pecas/base.fkp"}}]}"#);
        escrever(&d, "nao-usa.fka", &montagem(&[(1, "X", "pecas/outra.fkp")]));
        let usos = onde_usado(&[d.clone()], &alvo, "u-base");
        let nomes: Vec<(String, TipoRef, Option<u64>)> = usos.iter().map(|u| (u.arquivo.file_name().unwrap().to_string_lossy().into_owned(), u.tipo, u.no)).collect();
        assert_eq!(usos.len(), 3, "{nomes:?}");
        assert!(nomes.contains(&("conj1.fka".into(), TipoRef::Peca, Some(7))));
        assert!(nomes.contains(&("conj2.fka".into(), TipoRef::Peca, Some(3))));
        assert!(nomes.contains(&("desenho.fkd".into(), TipoRef::Vista, None)));
        assert_eq!(usos.iter().find(|u| u.no == Some(7)).unwrap().nome_no, "Base");
        assert_eq!(usos.iter().find(|u| u.no == Some(7)).unwrap().nome, "Conj", "nome vem do cabeçalho da montagem");
        // peça sem uso
        assert!(onde_usado(&[d.clone()], &d.join("pecas/outra.fkp"), "x").iter().all(|u| u.arquivo.file_name().unwrap() != "desenho.fkd"));
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn onde_usado_por_uid_acha_link_de_arquivo_movido() {
        let d = tmp("usado-uid");
        let alvo = escrever(&d, "novo-lugar/eixo.fkp", &peca("u-eixo", "Eixo"));
        escrever(&d, "p.fkp", r#"{"name":"P","links":[{"uid":"u-eixo","caminho":"eixo-antigo.fkp"}]}"#);
        let usos = onde_usado(&[d.clone()], &alvo, "u-eixo");
        assert_eq!(usos.len(), 1);
        assert_eq!(usos[0].tipo, TipoRef::Link);
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn diagnostico_e_reparo_de_arquivo_movido() {
        let d = tmp("reparo");
        escrever(&d, "novas/base.fkp", &peca("u-base", "Base"));
        escrever(&d, "ok.fkp", &peca("u-ok", "Ok"));
        let conj = escrever(&d, "conj.fka", &montagem(&[(1, "Base", "pecas/base.fkp"), (2, "Ok", "ok.fkp"), (3, "Fantasma", "pecas/fantasma.fkp")]));
        let diag = diagnosticar(&conj, &[d.clone()]);
        assert_eq!(diag.len(), 3);
        assert_eq!(diag[0].veredito, Veredito::Movido(d.join("novas").join("base.fkp")));
        assert_eq!(diag[1].veredito, Veredito::Ok);
        assert_eq!(diag[2].veredito, Veredito::Perdido);

        let antes = std::fs::read_to_string(&conj).unwrap();
        let aplicados = reparar_automatico(&conj, &[d.clone()]).unwrap();
        assert_eq!(aplicados.len(), 1);
        let depois = std::fs::read_to_string(&conj).unwrap();
        assert!(depois.contains("novas/base.fkp") && !depois.contains("pecas/base.fkp"));
        // só o caminho mudou: o resto do arquivo está byte a byte igual
        assert_eq!(antes.replace("pecas/base.fkp", "novas/base.fkp"), depois);
        let diag = diagnosticar(&conj, &[d.clone()]);
        assert_eq!(diag[0].veredito, Veredito::Ok);
        assert_eq!(diag[2].veredito, Veredito::Perdido, "o que não tem candidato continua pendente");
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn reparo_com_varios_candidatos_nao_escolhe_sozinho() {
        let d = tmp("varios");
        escrever(&d, "a/base.fkp", &peca("1", "A"));
        escrever(&d, "b/base.fkp", &peca("2", "B"));
        let conj = escrever(&d, "conj.fka", &montagem(&[(1, "Base", "sumiu/base.fkp")]));
        match &diagnosticar(&conj, &[d.clone()])[0].veredito {
            Veredito::Varios(v) => assert_eq!(v.len(), 2),
            outro => panic!("{outro:?}"),
        }
        let antes = std::fs::read_to_string(&conj).unwrap();
        assert!(reparar_automatico(&conj, &[d.clone()]).unwrap().is_empty());
        assert_eq!(std::fs::read_to_string(&conj).unwrap(), antes);
        // o usuário escolhe um e aplica
        assert_eq!(aplicar_reparos(&conj, &[("sumiu/base.fkp".into(), d.join("b/base.fkp"))]).unwrap(), 1);
        assert!(std::fs::read_to_string(&conj).unwrap().contains("b/base.fkp"));
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn troca_de_caminhos_nao_encadeia_e_ignora_outros_campos() {
        let j = r#"{"nodes":[{"path": "a.fkp"},{"path": "b.fkp"}],"name": "a.fkp","caminho":"a.fkp"}"#;
        let (t, n) = trocar_caminhos(j, &[("a.fkp".into(), "b.fkp".into()), ("b.fkp".into(), "c.fkp".into())]);
        assert_eq!(n, 3);
        assert_eq!(t, r#"{"nodes":[{"path": "b.fkp"},{"path": "c.fkp"}],"name": "a.fkp","caminho":"b.fkp"}"#);
        // caminho do Windows com barra invertida (escapada no JSON) também casa
        let w = r#"{"path": "C:\\pecas\\a.fkp"}"#;
        let (t, n) = trocar_caminhos(w, &[("C:\\pecas\\a.fkp".into(), "novo/a.fkp".into())]);
        assert_eq!((t.as_str(), n), (r#"{"path": "novo/a.fkp"}"#, 1));
    }
}
