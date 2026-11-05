// SPDX-License-Identifier: LGPL-2.1-or-later
//! Configurações em camadas, da aplicação ao documento (B100).
//!
//! Ordem de baixo para cima: **programa < empresa < projeto < usuário**, e o **documento** por cima
//! só nas chaves que pertencem a ele (cotagem, grade, anotações 3D, qualidade da malha).
//! Uma camada de empresa ou de projeto pode **travar** chaves: dali para cima ninguém muda o valor
//! (nem o usuário, nem o documento). Vale a trava da camada mais baixa.
//!
//! O módulo não conhece a estrutura de configuração do programa: trabalha com mapas JSON de chave
//! para valor, então uma opção nova entra sem mexer aqui. A classificação de cada opção (do
//! documento, da interface, da máquina, da sessão) fica no aplicativo.
//!
//! Arquivos: o de camada (`empresa.json`, `.fenkra/config.json` do projeto) é
//! `{"formato": "fenkra-camada", "versao": 1, "travadas": [...], "valores": {...}}`; o perfil de
//! interface exportado é `{"formato": "fenkra-perfil", "versao": 1, ...}`. Ambos são abertos:
//! campo desconhecido é ignorado, e `versao` maior que a conhecida abre com aviso.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Mapa de chave para valor.
pub type Mapa = Map<String, Value>;

pub const VERSAO_CAMADA: u32 = 1;
pub const VERSAO_PERFIL: u32 = 1;

/// De onde veio o valor de uma opção.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub enum Origem {
    Programa,
    Empresa,
    Projeto,
    Usuario,
    Documento,
}

impl Origem {
    pub fn rotulo(self) -> &'static str {
        match self {
            Origem::Programa => "programa",
            Origem::Empresa => "empresa",
            Origem::Projeto => "projeto",
            Origem::Usuario => "usuário",
            Origem::Documento => "documento",
        }
    }
}

/// Uma camada de empresa ou de projeto: valores e chaves travadas.
#[derive(Clone, PartialEq, Debug, Default, Serialize, Deserialize)]
pub struct Camada {
    #[serde(default)]
    pub formato: String,
    #[serde(default)]
    pub versao: u32,
    /// chaves cujo valor esta camada fixa para quem está acima
    #[serde(default)]
    pub travadas: Vec<String>,
    #[serde(default)]
    pub valores: Mapa,
}

impl Camada {
    pub fn nova() -> Self {
        Camada { formato: "fenkra-camada".into(), versao: VERSAO_CAMADA, travadas: vec![], valores: Mapa::new() }
    }
    /// Lê o texto de um arquivo de camada. Devolve também um aviso quando o arquivo é de uma
    /// versão mais nova ou de outro tipo.
    pub fn ler(texto: &str) -> Result<(Camada, Option<String>), String> {
        let c: Camada = serde_json::from_str(texto).map_err(|e| format!("arquivo de camada inválido: {e}"))?;
        let aviso = if !c.formato.is_empty() && c.formato != "fenkra-camada" {
            return Err(format!("não é um arquivo de camada (formato \"{}\")", c.formato));
        } else if c.versao > VERSAO_CAMADA {
            Some(format!("camada gravada em versão mais nova (v{}); campos desconhecidos foram ignorados", c.versao))
        } else {
            None
        };
        Ok((c, aviso))
    }
    pub fn para_texto(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }
}

/// Resultado de juntar as camadas.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Resolvido {
    pub valores: Mapa,
    /// camada que decidiu cada chave
    pub origem: BTreeMap<String, Origem>,
    /// chaves travadas e quem as travou
    pub travadas: BTreeMap<String, Origem>,
}

/// Junta as camadas. `programa` traz o valor padrão de toda chave; `empresa` e `projeto` são
/// opcionais; `usuario` é o que a pessoa mudou; `documento` só conta nas `chaves_doc`.
pub fn resolver(programa: &Mapa, empresa: Option<&Camada>, projeto: Option<&Camada>, usuario: &Mapa, documento: Option<&Mapa>, chaves_doc: &[&str]) -> Resolvido {
    let mut r = Resolvido { valores: programa.clone(), ..Default::default() };
    for k in programa.keys() {
        r.origem.insert(k.clone(), Origem::Programa);
    }
    let mut camadas: Vec<(Origem, &[String], &Mapa)> = vec![];
    let vazio: &[String] = &[];
    if let Some(c) = empresa {
        camadas.push((Origem::Empresa, &c.travadas, &c.valores));
    }
    if let Some(c) = projeto {
        camadas.push((Origem::Projeto, &c.travadas, &c.valores));
    }
    camadas.push((Origem::Usuario, vazio, usuario));
    if let Some(d) = documento {
        camadas.push((Origem::Documento, vazio, d));
    }
    for (quem, travadas, valores) in camadas {
        for (k, v) in valores {
            if r.travadas.contains_key(k) {
                continue;
            }
            if quem == Origem::Documento && !chaves_doc.contains(&k.as_str()) {
                continue;
            }
            r.valores.insert(k.clone(), v.clone());
            r.origem.insert(k.clone(), quem);
        }
        for k in travadas {
            r.travadas.entry(k.clone()).or_insert(quem);
        }
    }
    r
}

/// O que a pessoa mudou em relação ao que as camadas de baixo já resolvem: é o que o arquivo do
/// usuário guarda. Chave travada nunca entra (a trava manda, e não adianta gravá-la).
pub fn diferenca(atual: &Mapa, base: &Resolvido) -> Mapa {
    atual.iter().filter(|(k, v)| !base.travadas.contains_key(*k) && base.valores.get(*k) != Some(*v)).map(|(k, v)| (k.clone(), v.clone())).collect()
}

/// Repõe nas `atual` os valores travados. Devolve as chaves que estavam diferentes.
pub fn reforcar_travas(atual: &mut Mapa, base: &Resolvido) -> Vec<String> {
    let mut mexidas = vec![];
    for k in base.travadas.keys() {
        if let Some(v) = base.valores.get(k) {
            if atual.get(k) != Some(v) {
                atual.insert(k.clone(), v.clone());
                mexidas.push(k.clone());
            }
        }
    }
    mexidas
}

// ---------------------------------------------------------------- perfil de interface

/// Perfil de interface exportado: só as chaves de interface, com versão e quem gravou.
#[derive(Clone, PartialEq, Debug, Default, Serialize, Deserialize)]
pub struct Perfil {
    #[serde(default)]
    pub formato: String,
    #[serde(default)]
    pub versao: u32,
    #[serde(default)]
    pub app_version: String,
    #[serde(default)]
    pub valores: Mapa,
}

/// O que sobrou de um perfil importado.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct PerfilImportado {
    pub valores: Mapa,
    /// chaves do arquivo que não são de interface (ou que este programa não conhece)
    pub ignoradas: Vec<String>,
    /// chaves de interface que uma camada travou
    pub travadas: Vec<String>,
    pub aviso: Option<String>,
}

pub fn exportar_perfil(atual: &Mapa, chaves_interface: &[&str], app_version: &str) -> String {
    let valores: Mapa = atual.iter().filter(|(k, _)| chaves_interface.contains(&k.as_str())).map(|(k, v)| (k.clone(), v.clone())).collect();
    serde_json::to_string_pretty(&Perfil { formato: "fenkra-perfil".into(), versao: VERSAO_PERFIL, app_version: app_version.into(), valores }).unwrap_or_default()
}

/// Lê um perfil: só entram as chaves de interface, e as travadas ficam de fora.
pub fn importar_perfil(texto: &str, chaves_interface: &[&str], base: &Resolvido) -> Result<PerfilImportado, String> {
    let p: Perfil = serde_json::from_str(texto).map_err(|e| format!("perfil inválido: {e}"))?;
    if p.formato != "fenkra-perfil" {
        return Err(format!("não é um perfil de interface do Fenkra (formato \"{}\")", p.formato));
    }
    let mut out = PerfilImportado::default();
    if p.versao > VERSAO_PERFIL {
        out.aviso = Some(format!("perfil gravado em versão mais nova (v{}); só o que este programa conhece foi lido", p.versao));
    }
    for (k, v) in p.valores {
        if !chaves_interface.contains(&k.as_str()) {
            out.ignoradas.push(k);
        } else if base.travadas.contains_key(&k) {
            out.travadas.push(k);
        } else {
            out.valores.insert(k, v);
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------- arquivo com backup

/// O que a leitura de um arquivo de configuração encontrou.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Leitura {
    pub valor: Option<Value>,
    /// o que a pessoa precisa saber (arquivo ruim, restaurado do backup, etc.)
    pub aviso: Option<String>,
    pub do_backup: bool,
}

pub fn caminho_backup(p: &Path) -> PathBuf {
    sufixo(p, ".bak")
}

fn sufixo(p: &Path, s: &str) -> PathBuf {
    let mut n = p.as_os_str().to_os_string();
    n.push(s);
    PathBuf::from(n)
}

fn ler_valor(p: &Path) -> Result<Value, String> {
    let t = std::fs::read_to_string(p).map_err(|e| e.to_string())?;
    serde_json::from_str::<Value>(&t).map_err(|e| e.to_string()).and_then(|v| if v.is_object() { Ok(v) } else { Err("não é um objeto JSON".into()) })
}

/// Lê o arquivo; se estiver corrompido tenta o backup. O arquivo ruim é guardado ao lado como
/// `.corrompido` para a pessoa poder olhar, e o aviso diz o que aconteceu. Arquivo que não existe
/// não é problema (primeira execução).
pub fn ler_com_backup(p: &Path) -> Leitura {
    if !p.exists() {
        return Leitura::default();
    }
    match ler_valor(p) {
        Ok(v) => Leitura { valor: Some(v), ..Default::default() },
        Err(e) => {
            let guardado = sufixo(p, ".corrompido");
            let _ = std::fs::copy(p, &guardado);
            let nome = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            match ler_valor(&caminho_backup(p)) {
                Ok(v) => Leitura {
                    valor: Some(v),
                    do_backup: true,
                    aviso: Some(format!("{nome} não pôde ser lido ({e}); foi restaurado o backup. O arquivo com problema ficou em {}.", guardado.display())),
                },
                Err(_) => Leitura {
                    valor: None,
                    do_backup: false,
                    aviso: Some(format!("{nome} não pôde ser lido ({e}) e não há backup válido; usando os padrões. O arquivo com problema ficou em {}.", guardado.display())),
                },
            }
        }
    }
}

/// Grava com segurança: escreve num temporário, guarda o arquivo atual como `.bak` (só se ele
/// estiver válido, para um arquivo ruim nunca apagar um backup bom) e troca. Devolve o motivo
/// quando falha, para a interface avisar em vez de perder o ajuste em silêncio.
pub fn gravar_com_backup(p: &Path, texto: &str) -> Result<(), String> {
    if let Some(dir) = p.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("não consegui criar a pasta {}: {e}", dir.display()))?;
    }
    let tmp = sufixo(p, ".tmp");
    {
        use std::io::Write;
        let mut f = std::fs::File::create(&tmp).map_err(|e| format!("não consegui gravar {}: {e}", tmp.display()))?;
        f.write_all(texto.as_bytes()).and_then(|_| f.sync_all()).map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            format!("não consegui gravar {}: {e}", tmp.display())
        })?;
    }
    if p.exists() && ler_valor(p).is_ok() {
        let _ = std::fs::copy(p, caminho_backup(p));
    }
    std::fs::rename(&tmp, p).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        format!("não consegui substituir {}: {e}", p.display())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn m(v: Value) -> Mapa {
        v.as_object().unwrap().clone()
    }

    fn pasta(nome: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("fenkra_config_{nome}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn camadas_valem_na_ordem_programa_empresa_projeto_usuario() {
        let prog = m(json!({"a": 1, "b": 1, "c": 1, "d": 1}));
        let emp = Camada { valores: m(json!({"a": 2, "b": 2, "c": 2})), ..Camada::nova() };
        let proj = Camada { valores: m(json!({"b": 3, "c": 3})), ..Camada::nova() };
        let usu = m(json!({"c": 4}));
        let r = resolver(&prog, Some(&emp), Some(&proj), &usu, None, &[]);
        assert_eq!(r.valores, m(json!({"a": 2, "b": 3, "c": 4, "d": 1})));
        assert_eq!(r.origem["a"], Origem::Empresa);
        assert_eq!(r.origem["b"], Origem::Projeto);
        assert_eq!(r.origem["c"], Origem::Usuario);
        assert_eq!(r.origem["d"], Origem::Programa);
    }

    #[test]
    fn chave_travada_nao_muda_para_quem_esta_acima() {
        let prog = m(json!({"norma": "ISO", "casas": 2, "grade": 5}));
        let emp = Camada { travadas: vec!["norma".into()], valores: m(json!({"norma": "ABNT"})), ..Camada::nova() };
        let proj = Camada { valores: m(json!({"norma": "ASME", "casas": 3})), travadas: vec!["casas".into()], ..Camada::nova() };
        let usu = m(json!({"norma": "DIN", "casas": 1, "grade": 10}));
        let r = resolver(&prog, Some(&emp), Some(&proj), &usu, Some(&m(json!({"casas": 9, "grade": 20}))), &["casas", "grade"]);
        assert_eq!(r.valores["norma"], "ABNT", "a trava da empresa vence projeto e usuário");
        assert_eq!(r.valores["casas"], 3, "o projeto travou depois de dar o valor dele");
        assert_eq!(r.valores["grade"], 20, "o documento decide o que ninguém travou");
        assert_eq!(r.travadas["norma"], Origem::Empresa);
        assert_eq!(r.travadas["casas"], Origem::Projeto);
        assert!(!r.travadas.contains_key("grade"));
    }

    #[test]
    fn trava_sem_valor_na_camada_segura_o_valor_de_baixo() {
        let prog = m(json!({"x": 7}));
        let emp = Camada { travadas: vec!["x".into()], ..Camada::nova() };
        let r = resolver(&prog, Some(&emp), None, &m(json!({"x": 99})), None, &[]);
        assert_eq!(r.valores["x"], 7);
    }

    #[test]
    fn documento_so_manda_nas_chaves_dele() {
        let prog = m(json!({"grade": 5, "tema": "claro"}));
        let doc = m(json!({"grade": 20, "tema": "escuro"}));
        let r = resolver(&prog, None, None, &Mapa::new(), Some(&doc), &["grade"]);
        assert_eq!(r.valores["grade"], 20);
        assert_eq!(r.valores["tema"], "claro", "tema não é do documento");
        assert_eq!(r.origem["grade"], Origem::Documento);
    }

    #[test]
    fn diferenca_grava_so_o_que_mudou_e_nunca_o_travado() {
        let prog = m(json!({"a": 1, "b": 1, "c": 1}));
        let emp = Camada { travadas: vec!["c".into()], valores: m(json!({"c": 5})), ..Camada::nova() };
        let base = resolver(&prog, Some(&emp), None, &Mapa::new(), None, &[]);
        let atual = m(json!({"a": 1, "b": 2, "c": 9}));
        assert_eq!(diferenca(&atual, &base), m(json!({"b": 2})));
        let mut a = atual.clone();
        assert_eq!(reforcar_travas(&mut a, &base), vec!["c".to_string()]);
        assert_eq!(a["c"], 5);
        assert!(reforcar_travas(&mut a, &base).is_empty());
    }

    #[test]
    fn perfil_exporta_so_interface_e_importa_respeitando_travas() {
        let atual = m(json!({"tema": "escuro", "escala": 1.25, "grade": 5, "recentes": ["a"]}));
        let texto = exportar_perfil(&atual, &["tema", "escala"], "0.1");
        let p: Perfil = serde_json::from_str(&texto).unwrap();
        assert_eq!(p.formato, "fenkra-perfil");
        assert_eq!(p.valores.len(), 2);
        let emp = Camada { travadas: vec!["escala".into()], valores: m(json!({"escala": 1.0})), ..Camada::nova() };
        let base = resolver(&atual, Some(&emp), None, &Mapa::new(), None, &[]);
        let i = importar_perfil(&texto, &["tema", "escala"], &base).unwrap();
        assert_eq!(i.valores, m(json!({"tema": "escuro"})));
        assert_eq!(i.travadas, vec!["escala".to_string()]);
        // chave que não é de interface é ignorada, e o aviso de versão futura aparece
        let futuro = r#"{"formato":"fenkra-perfil","versao":9,"valores":{"tema":"claro","grade":99}}"#;
        let i = importar_perfil(futuro, &["tema"], &base).unwrap();
        assert_eq!(i.ignoradas, vec!["grade".to_string()]);
        assert!(i.aviso.unwrap().contains("v9"));
        assert!(importar_perfil(r#"{"formato":"outra coisa"}"#, &["tema"], &base).is_err());
        assert!(importar_perfil("não é json", &["tema"], &base).is_err());
    }

    #[test]
    fn camada_de_versao_futura_abre_com_aviso() {
        let (c, aviso) = Camada::ler(r#"{"formato":"fenkra-camada","versao":7,"travadas":["x"],"valores":{"x":1},"campo_novo":true}"#).unwrap();
        assert_eq!(c.travadas, vec!["x".to_string()]);
        assert!(aviso.is_some());
        assert!(Camada::ler(r#"{"formato":"fenkra-perfil"}"#).is_err());
        assert_eq!(Camada::ler(&Camada::nova().para_texto()).unwrap().1, None);
    }

    #[test]
    fn arquivo_corrompido_volta_do_backup_com_aviso() {
        let d = pasta("backup");
        let p = d.join("config.json");
        gravar_com_backup(&p, r#"{"v": 1}"#).unwrap();
        assert!(!caminho_backup(&p).exists(), "primeira gravação não tem o que guardar");
        gravar_com_backup(&p, r#"{"v": 2}"#).unwrap();
        assert_eq!(ler_valor(&caminho_backup(&p)).unwrap()["v"], 1);
        // o arquivo estraga
        std::fs::write(&p, "{ isto não é json").unwrap();
        let l = ler_com_backup(&p);
        assert!(l.do_backup);
        assert_eq!(l.valor.unwrap()["v"], 1);
        assert!(l.aviso.unwrap().contains("restaurado o backup"));
        assert!(sufixo(&p, ".corrompido").exists(), "o arquivo ruim fica para a pessoa olhar");
        // gravar por cima de um arquivo ruim não destrói o backup bom
        gravar_com_backup(&p, r#"{"v": 3}"#).unwrap();
        assert_eq!(ler_valor(&caminho_backup(&p)).unwrap()["v"], 1);
        assert_eq!(ler_com_backup(&p).valor.unwrap()["v"], 3);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn sem_arquivo_nao_ha_aviso_e_sem_backup_valido_usa_padrao() {
        let d = pasta("sem");
        let p = d.join("config.json");
        let l = ler_com_backup(&p);
        assert!(l.valor.is_none() && l.aviso.is_none());
        std::fs::write(&p, "lixo").unwrap();
        let l = ler_com_backup(&p);
        assert!(l.valor.is_none());
        assert!(l.aviso.unwrap().contains("não há backup válido"));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn falha_ao_gravar_devolve_o_motivo() {
        let d = pasta("falha");
        // um arquivo no lugar da pasta impede criar o destino
        let arquivo = d.join("ocupado");
        std::fs::write(&arquivo, "x").unwrap();
        let e = gravar_com_backup(&arquivo.join("config.json"), "{}").unwrap_err();
        assert!(e.contains("não consegui"), "{e}");
        let _ = std::fs::remove_dir_all(&d);
    }
}
