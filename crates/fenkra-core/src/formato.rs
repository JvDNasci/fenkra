// SPDX-License-Identifier: LGPL-2.1-or-later
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
/// A peça está na versão 2 (B101); os demais tipos seguem na 1 e têm cadeia vazia.
const PASSOS_PECA: &[Passo] = &[peca_v1_para_v2];

/// v1 -> v2 (B101): parâmetros ganham tipo, unidade de exibição, limites e opções, e comandos ganham
/// supressão por expressão. Todos os campos novos têm padrão, então o passo só declara os padrões
/// e `format_version` sobe (um Fenkra v1 abre o arquivo v2 somente leitura).
fn peca_v1_para_v2(v: &mut Value) {
    if let Some(ps) = v.get_mut("params").and_then(Value::as_array_mut) {
        for p in ps {
            if let Some(o) = p.as_object_mut() {
                o.entry("tipo").or_insert(Value::from("Numero"));
                o.entry("exibir").or_insert(Value::from("Mm"));
            }
        }
    }
}
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

/// Identidade estável do documento (B66): gravada no arquivo, sobrevive a renomear e mover.
/// Como o `app_version`, **não conta na igualdade**: duas peças montadas do mesmo jeito continuam
/// iguais (testes e assinatura de cache); a identidade vive no arquivo, não na geometria.
/// Vazio = arquivo antigo, ainda sem uid; o primeiro salvamento atribui um com [`UidDoc::garantir`].
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct UidDoc(pub String);

impl PartialEq for UidDoc {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl UidDoc {
    pub fn novo() -> Self {
        UidDoc(novo_uid())
    }
    pub fn garantir(&mut self) {
        if self.0.is_empty() {
            self.0 = novo_uid();
        }
    }
    pub fn esta_vazio(&self) -> bool {
        self.0.is_empty()
    }
}

/// uid de 128 bits em hexadecimal (32 caracteres). Sem dependência de aleatório: mistura o relógio,
/// o processo, um contador e um endereço de pilha com splitmix64; basta para não colidir entre
/// documentos de um mesmo usuário ou equipe.
pub fn novo_uid() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static CONT: AtomicU64 = AtomicU64::new(0);
    fn mix(mut z: u64) -> u64 {
        z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let c = CONT.fetch_add(1, Ordering::Relaxed);
    let pilha = &c as *const u64 as u64;
    let a = mix((t as u64) ^ mix(c) ^ mix(std::process::id() as u64));
    let b = mix(((t >> 64) as u64) ^ a ^ mix(pilha) ^ c.rotate_left(32));
    format!("{a:016x}{b:016x}")
}

/// Lê o `uid` gravado no topo do JSON de qualquer tipo de arquivo (vazio se não houver).
pub fn uid_do_json(json: &str) -> String {
    serde_json::from_str::<Value>(json).ok().and_then(|v| v.get("uid").and_then(Value::as_str).map(str::to_string)).unwrap_or_default()
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
    fn uids_sao_unicos_e_nao_contam_na_igualdade() {
        let v: std::collections::HashSet<String> = (0..2000).map(|_| novo_uid()).collect();
        assert_eq!(v.len(), 2000);
        assert!(v.iter().all(|u| u.len() == 32));
        assert_eq!(UidDoc::novo(), UidDoc::novo());
        let mut u = UidDoc::default();
        assert!(u.esta_vazio());
        u.garantir();
        let antes = u.0.clone();
        u.garantir();
        assert_eq!(u.0, antes);
        assert_eq!(uid_do_json(&format!(r#"{{"uid": "{antes}"}}"#)), antes);
        assert_eq!(uid_do_json("{}"), "");
    }

    #[test]
    fn versao_app_nao_conta_na_igualdade() {
        assert_eq!(VersaoApp("0.0.1".into()), VersaoApp("9.9.9".into()));
    }
}
