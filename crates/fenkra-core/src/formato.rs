//! Versão de formato e cadeia de migração dos arquivos do Fenkra (B64).
//!
//! Cada tipo de arquivo (`.fkp`, `.fka`, `.fkd` e o índice do projeto) grava seu próprio
//! `format_version`. Ao abrir, o JSON vira um [`serde_json::Value`] e passa pela cadeia
//! `migrate(vN -> vN+1)` até a versão atual, antes de virar a estrutura tipada. Assim um arquivo
//! antigo nunca deixa de abrir: quem muda o formato escreve um passo novo em [`Tipo::passos`] e uma
//! fixture do formato anterior.
//!
//! Regras:
//! - sem `format_version` no arquivo = versão 1 (o primeiro alpha não gravava o campo em todos);
//! - versão **maior** que a do programa: abre tolerante (campos desconhecidos são ignorados) e o
//!   chamador deve tratar como somente leitura, com aviso, para nunca regravar por cima e perder dados;
//! - `app_version` registra qual versão do programa gravou o arquivo (só informativo).

use serde_json::Value;

/// Um passo da cadeia: reescreve o JSON da versão `N` para a `N + 1`.
pub type Passo = fn(&mut Value);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tipo {
    Peca,
    Montagem,
    Desenho,
    Indice,
}

/// Passos de migração por tipo. O passo `i` leva da versão `i + 1` para a `i + 2`.
/// Hoje todos os tipos estão na versão 1, então as cadeias são vazias.
const PASSOS_PECA: &[Passo] = &[];
const PASSOS_MONTAGEM: &[Passo] = &[];
const PASSOS_DESENHO: &[Passo] = &[];
const PASSOS_INDICE: &[Passo] = &[];

impl Tipo {
    pub fn passos(self) -> &'static [Passo] {
        match self {
            Tipo::Peca => PASSOS_PECA,
            Tipo::Montagem => PASSOS_MONTAGEM,
            Tipo::Desenho => PASSOS_DESENHO,
            Tipo::Indice => PASSOS_INDICE,
        }
    }
    /// Versão de formato que este programa grava.
    pub fn versao_atual(self) -> u32 {
        1 + self.passos().len() as u32
    }
    pub fn nome(self) -> &'static str {
        match self {
            Tipo::Peca => "peça",
            Tipo::Montagem => "montagem",
            Tipo::Desenho => "desenho",
            Tipo::Indice => "índice do projeto",
        }
    }
}

/// Versão do programa que grava o arquivo (`app_version`).
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Campo `app_version` dos documentos: informativo, por isso **não conta** na igualdade
/// (abrir um arquivo gravado por outra versão não o torna "diferente").
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct VersaoApp(pub String);

impl PartialEq for VersaoApp {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl VersaoApp {
    pub fn atual() -> Self {
        VersaoApp(APP_VERSION.into())
    }
}

/// Resultado de preparar um JSON para virar a estrutura tipada.
#[derive(Debug)]
pub struct Preparado {
    pub valor: Value,
    /// versão que estava gravada no arquivo
    pub versao_original: u32,
    /// o arquivo veio de um programa mais novo: abrir só para leitura
    pub futuro: bool,
}

impl Preparado {
    pub fn aviso(&self, tipo: Tipo) -> Option<String> {
        self.futuro.then(|| aviso_futuro(tipo, self.versao_original))
    }
}

pub fn aviso_futuro(tipo: Tipo, versao: u32) -> String {
    format!(
        "Este arquivo ({}) foi gravado em um formato mais novo (v{versao}) do que este Fenkra entende (v{}). Ele abre somente para leitura; atualize o programa para editar.",
        tipo.nome(),
        tipo.versao_atual()
    )
}

/// Lê `format_version` do objeto (ausente = 1).
pub fn versao_de(v: &Value) -> u32 {
    v.get("format_version").and_then(Value::as_u64).map(|n| n.min(u32::MAX as u64) as u32).unwrap_or(1)
}

/// Aplica `passos` de `de` até `para`, gravando `format_version` a cada passo.
pub fn migrar(v: &mut Value, de: u32, para: u32, passos: &[Passo]) {
    let mut atual = de.max(1);
    while atual < para {
        if let Some(p) = passos.get((atual - 1) as usize) {
            p(v);
        }
        atual += 1;
        if let Some(o) = v.as_object_mut() {
            o.insert("format_version".into(), Value::from(atual));
        }
    }
}

/// Lê o JSON, migra até a versão atual do `tipo` e informa se é de um formato futuro.
pub fn preparar(tipo: Tipo, json: &str) -> Result<Preparado, serde_json::Error> {
    let mut valor: Value = serde_json::from_str(json)?;
    let versao_original = versao_de(&valor);
    let atual = tipo.versao_atual();
    let futuro = versao_original > atual;
    if !futuro {
        migrar(&mut valor, versao_original, atual, tipo.passos());
    }
    Ok(Preparado { valor, versao_original, futuro })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn p1(v: &mut Value) {
        // v1 -> v2: "nome" vira "name"
        if let Some(n) = v.as_object_mut().and_then(|o| o.remove("nome")) {
            v["name"] = n;
        }
    }
    fn p2(v: &mut Value) {
        // v2 -> v3: "name" ganha prefixo
        let n = v["name"].as_str().unwrap_or("").to_string();
        v["name"] = Value::from(format!("P-{n}"));
    }

    #[test]
    fn cadeia_aplica_todos_os_passos_em_ordem() {
        let mut v = json!({"format_version": 1, "nome": "x"});
        migrar(&mut v, 1, 3, &[p1, p2]);
        assert_eq!(v["name"], "P-x");
        assert_eq!(v["format_version"], 3);
    }

    #[test]
    fn cadeia_parte_da_versao_gravada() {
        let mut v = json!({"format_version": 2, "name": "x"});
        migrar(&mut v, 2, 3, &[p1, p2]);
        assert_eq!(v["name"], "P-x");
    }

    #[test]
    fn sem_versao_vale_um_e_futuro_e_marcado() {
        assert_eq!(versao_de(&json!({})), 1);
        let p = preparar(Tipo::Peca, r#"{"format_version": 99}"#).unwrap();
        assert!(p.futuro);
        assert_eq!(p.versao_original, 99);
        assert!(p.aviso(Tipo::Peca).unwrap().contains("v99"));
        let q = preparar(Tipo::Peca, "{}").unwrap();
        assert!(!q.futuro);
    }

    #[test]
    fn versao_app_nao_conta_na_igualdade() {
        assert_eq!(VersaoApp("0.0.1".into()), VersaoApp("9.9.9".into()));
    }
}
