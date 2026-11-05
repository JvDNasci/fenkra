// SPDX-License-Identifier: LGPL-2.1-or-later
//! Tabela de projeto e configurações da peça (B102).
//!
//! Uma peça pode ter várias variantes (a mesma peça em três comprimentos, com e sem chanfro). Em vez
//! de copiar o arquivo, a **tabela de projeto** guarda uma linha por variante e uma coluna por coisa
//! que varia: o valor de um parâmetro, a supressão de um comando ou o número de peça (PN). Uma célula
//! vazia herda o valor base do documento.
//!
//! A **configuração ativa** é a linha que vale quando a peça é aberta ou calculada. Uma montagem
//! escolhe a configuração de cada componente pelo **nome**. Nome que não existe é erro visível
//! (`ErroConfig`), nunca "a primeira linha": a montagem errada de silêncio é pior que a que avisa.
//!
//! Como funciona: `Document::com_configuracao` copia o documento e troca, na cópia, a expressão dos
//! parâmetros, a supressão por expressão dos comandos (`suppress_if`, ADR 0028) e o PN. O recálculo
//! segue o caminho normal sobre essa cópia, então árvore, exportação e lista de peças concordam.
//!
//! CSV (sem Excel): a primeira coluna é o nome da configuração; as demais têm o cabeçalho `PN`, o
//! nome de um parâmetro ou `suprimir:<nome do comando>`. Lê `;`, `,` ou tabulação e aspas duplas;
//! escreve com `;`, que o Excel em português abre direto.

use crate::doc::{Document, Id};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

/// O que uma coluna da tabela varia.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum Coluna {
    /// expressão de um parâmetro, pelo nome
    Parametro(String),
    /// supressão de um comando: expressão (diferente de zero suprime); aceita sim/não, 1/0, verdadeiro/falso
    Supressao(Id),
    /// número de peça da variante
    Pn,
}

/// Uma variante: nome e uma célula por coluna (vazia = herda o valor base).
#[derive(Clone, PartialEq, Debug, Default, Serialize, Deserialize)]
pub struct Linha {
    pub nome: String,
    pub celulas: Vec<String>,
}

/// A tabela de projeto da peça.
#[derive(Clone, PartialEq, Debug, Default, Serialize, Deserialize)]
pub struct TabelaProjeto {
    #[serde(default)]
    pub colunas: Vec<Coluna>,
    #[serde(default)]
    pub linhas: Vec<Linha>,
    /// nome da configuração ativa; vazio = os valores base do documento
    #[serde(default)]
    pub ativa: String,
}

impl TabelaProjeto {
    /// Sem linhas, colunas nem ativa: não precisa ir para o arquivo.
    pub fn vazia(&self) -> bool {
        self.colunas.is_empty() && self.linhas.is_empty() && self.ativa.is_empty()
    }

    pub fn nomes(&self) -> Vec<String> {
        self.linhas.iter().map(|l| l.nome.clone()).collect()
    }

    pub fn linha(&self, nome: &str) -> Option<&Linha> {
        self.linhas.iter().find(|l| l.nome == nome)
    }
}

/// Por que uma configuração não pôde ser aplicada. O texto vai direto para a interface.
#[derive(Clone, PartialEq, Debug)]
pub enum ErroConfig {
    /// o nome não está na tabela (a peça pode nem ter tabela)
    Inexistente { nome: String, existentes: Vec<String> },
    /// uma coluna aponta para um parâmetro ou comando que não existe mais
    ColunaInvalida(String),
    /// o CSV não pôde ser lido
    Csv(String),
}

impl std::fmt::Display for ErroConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErroConfig::Inexistente { nome, existentes } if existentes.is_empty() => write!(f, "a configuração '{nome}' não existe: esta peça não tem tabela de projeto"),
            ErroConfig::Inexistente { nome, existentes } => write!(f, "a configuração '{nome}' não existe nesta peça (existem: {})", existentes.join(", ")),
            ErroConfig::ColunaInvalida(m) => write!(f, "tabela de projeto: {m}"),
            ErroConfig::Csv(m) => write!(f, "CSV da tabela de projeto: {m}"),
        }
    }
}

impl std::error::Error for ErroConfig {}

/// Texto da supressão para o campo `suppress_if`: sim/não viram 1/0; expressão passa como veio.
fn supressao(cel: &str) -> String {
    match cel.trim().to_lowercase().as_str() {
        "sim" | "s" | "verdadeiro" | "true" | "yes" => "1".into(),
        "não" | "nao" | "n" | "falso" | "false" | "no" => "0".into(),
        _ => cel.trim().to_string(),
    }
}

impl Document {
    /// Cópia do documento com a configuração `nome` aplicada. A cópia sai sem a tabela, para o
    /// recálculo não aplicar duas vezes. Nome inexistente é erro, nunca a primeira linha.
    pub fn com_configuracao(&self, nome: &str) -> Result<Document, ErroConfig> {
        let tab = &self.tabela;
        let Some(linha) = tab.linha(nome) else { return Err(ErroConfig::Inexistente { nome: nome.to_string(), existentes: tab.nomes() }) };
        let mut d = self.clone();
        d.tabela = TabelaProjeto::default();
        for (col, cel) in tab.colunas.iter().zip(&linha.celulas) {
            if cel.trim().is_empty() {
                continue;
            }
            match col {
                Coluna::Parametro(n) => match d.params.iter_mut().find(|p| &p.name == n) {
                    Some(p) => p.expr = cel.trim().to_string(),
                    None => return Err(ErroConfig::ColunaInvalida(format!("o parâmetro '{n}' da coluna não existe mais"))),
                },
                Coluna::Supressao(id) => match d.feature_mut(*id) {
                    Some(f) => f.suppress_if = supressao(cel),
                    None => return Err(ErroConfig::ColunaInvalida("um comando da coluna de supressão não existe mais".into())),
                },
                Coluna::Pn => d.props.pn = cel.trim().to_string(),
            }
        }
        Ok(d)
    }

    /// O documento que o recálculo usa: o próprio, ou a cópia com a configuração ativa aplicada.
    pub fn efetivo(&self) -> Result<Cow<'_, Document>, ErroConfig> {
        if self.tabela.ativa.is_empty() {
            return Ok(Cow::Borrowed(self));
        }
        self.com_configuracao(&self.tabela.ativa).map(Cow::Owned)
    }

    /// Coloca uma linha nova na tabela: parte dos valores atuais do documento (parâmetros e PN) para
    /// a pessoa editar só o que muda.
    pub fn nova_configuracao(&mut self, nome: &str) -> bool {
        if nome.trim().is_empty() || self.tabela.linha(nome).is_some() {
            return false;
        }
        let celulas = self
            .tabela
            .colunas
            .iter()
            .map(|c| match c {
                Coluna::Parametro(n) => self.params.iter().find(|p| &p.name == n).map(|p| p.expr.clone()).unwrap_or_default(),
                Coluna::Supressao(id) => self.features.iter().find(|f| f.id == *id).map(|f| f.suppress_if.clone()).unwrap_or_default(),
                Coluna::Pn => self.props.pn.clone(),
            })
            .collect();
        self.tabela.linhas.push(Linha { nome: nome.to_string(), celulas });
        true
    }

    /// Acrescenta a coluna (com célula vazia em todas as linhas); repetida não entra.
    pub fn nova_coluna_config(&mut self, col: Coluna) -> bool {
        if self.tabela.colunas.contains(&col) {
            return false;
        }
        self.tabela.colunas.push(col);
        for l in &mut self.tabela.linhas {
            l.celulas.push(String::new());
        }
        true
    }

    /// Cabeçalho CSV de uma coluna.
    fn cabecalho(&self, c: &Coluna) -> String {
        match c {
            Coluna::Pn => "PN".into(),
            Coluna::Parametro(n) => n.clone(),
            Coluna::Supressao(id) => format!("suprimir:{}", self.name_of(*id)),
        }
    }

    /// A tabela em CSV (`;`), pronta para abrir em qualquer planilha.
    pub fn tabela_para_csv(&self) -> String {
        let campo = |s: &str| {
            if s.contains([';', '"', '\n', '\r']) {
                format!("\"{}\"", s.replace('"', "\"\""))
            } else {
                s.to_string()
            }
        };
        let mut out = String::new();
        let mut cab = vec!["Configuração".to_string()];
        cab.extend(self.tabela.colunas.iter().map(|c| self.cabecalho(c)));
        out += &cab.iter().map(|s| campo(s)).collect::<Vec<_>>().join(";");
        out += "\n";
        for l in &self.tabela.linhas {
            let mut v = vec![l.nome.clone()];
            v.extend((0..self.tabela.colunas.len()).map(|i| l.celulas.get(i).cloned().unwrap_or_default()));
            out += &v.iter().map(|s| campo(s)).collect::<Vec<_>>().join(";");
            out += "\n";
        }
        out
    }

    /// Troca a tabela pelo conteúdo do CSV. Cabeçalho que não casa com parâmetro nem comando do
    /// documento é erro (a tabela antiga fica como estava).
    pub fn importar_tabela_csv(&mut self, texto: &str) -> Result<(), ErroConfig> {
        let linhas = ler_csv(texto)?;
        let Some((cab, resto)) = linhas.split_first() else { return Err(ErroConfig::Csv("arquivo vazio".into())) };
        if cab.len() < 2 && resto.iter().all(|l| l.len() < 2) && cab.first().is_none_or(|c| c.trim().is_empty()) {
            return Err(ErroConfig::Csv("sem cabeçalho".into()));
        }
        let mut colunas = vec![];
        for h in &cab[1..] {
            let h = h.trim();
            let col = if h.eq_ignore_ascii_case("pn") {
                Coluna::Pn
            } else if let Some(n) = h.strip_prefix("suprimir:") {
                let n = n.trim();
                let Some(f) = self.features.iter().find(|f| f.name == n) else { return Err(ErroConfig::ColunaInvalida(format!("não há comando '{n}' para a coluna '{h}'"))) };
                Coluna::Supressao(f.id)
            } else if self.params.iter().any(|p| p.name == h) {
                Coluna::Parametro(h.to_string())
            } else {
                return Err(ErroConfig::ColunaInvalida(format!("a coluna '{h}' não é PN, suprimir:<comando> nem um parâmetro desta peça")));
            };
            if colunas.contains(&col) {
                return Err(ErroConfig::ColunaInvalida(format!("a coluna '{h}' aparece duas vezes")));
            }
            colunas.push(col);
        }
        let mut novas: Vec<Linha> = vec![];
        for l in resto {
            let nome = l.first().map(|s| s.trim().to_string()).unwrap_or_default();
            if nome.is_empty() {
                continue;
            }
            if novas.iter().any(|x| x.nome == nome) {
                return Err(ErroConfig::Csv(format!("a configuração '{nome}' aparece duas vezes")));
            }
            let celulas = (0..colunas.len()).map(|i| l.get(i + 1).map(|s| s.trim().to_string()).unwrap_or_default()).collect();
            novas.push(Linha { nome, celulas });
        }
        let ativa = if novas.iter().any(|l| l.nome == self.tabela.ativa) { self.tabela.ativa.clone() } else { String::new() };
        self.tabela = TabelaProjeto { colunas, linhas: novas, ativa };
        Ok(())
    }
}

/// Lê CSV com `;`, `,` ou tabulação (a do cabeçalho), aspas duplas e quebras de linha dentro de aspas.
fn ler_csv(texto: &str) -> Result<Vec<Vec<String>>, ErroConfig> {
    let texto = texto.trim_start_matches('\u{feff}');
    let primeira = texto.lines().next().unwrap_or("");
    let sep = [';', '\t', ','].into_iter().max_by_key(|c| primeira.matches(*c).count()).unwrap_or(';');
    let (mut linhas, mut linha, mut campo) = (vec![], vec![], String::new());
    let (mut aspas, mut tem_campo) = (false, false);
    let mut it = texto.chars().peekable();
    while let Some(c) = it.next() {
        match c {
            '"' if aspas && it.peek() == Some(&'"') => {
                campo.push('"');
                it.next();
            }
            '"' => {
                aspas = !aspas;
                tem_campo = true;
            }
            c if c == sep && !aspas => {
                linha.push(std::mem::take(&mut campo));
                tem_campo = true;
            }
            '\r' if !aspas => {}
            '\n' if !aspas => {
                if tem_campo || !campo.is_empty() {
                    linha.push(std::mem::take(&mut campo));
                    linhas.push(std::mem::take(&mut linha));
                }
                tem_campo = false;
            }
            c => {
                campo.push(c);
                tem_campo = true;
            }
        }
    }
    if aspas {
        return Err(ErroConfig::Csv("aspas abertas sem fechar".into()));
    }
    if tem_campo || !campo.is_empty() {
        linha.push(campo);
        linhas.push(linha);
    }
    Ok(linhas)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doc::Unit;

    fn peca() -> (Document, Id) {
        let mut d = Document::new("p");
        d.add_param("Comprimento", "100", Unit::Comprimento);
        d.add_param("Largura", "20", Unit::Comprimento);
        let sk = d.add_feature(crate::doc::FeatureKind::Sketch(crate::doc::Sketch { plane: crate::doc::PlaneRef::Origin(crate::doc::OriginPlane::XY), data: Default::default() }));
        (d, sk)
    }

    fn com_tabela() -> (Document, Id) {
        let (mut d, sk) = peca();
        d.nova_coluna_config(Coluna::Parametro("Comprimento".into()));
        d.nova_coluna_config(Coluna::Pn);
        d.nova_coluna_config(Coluna::Supressao(sk));
        d.tabela.linhas.push(Linha { nome: "Curta".into(), celulas: vec!["50".into(), "P-050".into(), "".into()] });
        d.tabela.linhas.push(Linha { nome: "Longa".into(), celulas: vec!["200".into(), "P-200".into(), "sim".into()] });
        (d, sk)
    }

    #[test]
    fn aplica_parametro_pn_e_supressao_na_copia() {
        let (d, sk) = com_tabela();
        let c = d.com_configuracao("Longa").unwrap();
        assert_eq!(c.params.iter().find(|p| p.name == "Comprimento").unwrap().expr, "200");
        assert_eq!(c.params.iter().find(|p| p.name == "Largura").unwrap().expr, "20", "o que não está na tabela não muda");
        assert_eq!(c.props.pn, "P-200");
        assert_eq!(c.feature(sk).unwrap().suppress_if, "1");
        assert!(c.tabela.vazia(), "a cópia não leva a tabela: o recálculo não aplica duas vezes");
        // célula vazia herda o base
        let k = d.com_configuracao("Curta").unwrap();
        assert_eq!(k.feature(sk).unwrap().suppress_if, "");
        // o original não foi tocado
        assert_eq!(d.params[0].expr, "100");
    }

    #[test]
    fn nome_inexistente_e_erro_nunca_a_primeira() {
        let (d, _) = com_tabela();
        let e = d.com_configuracao("Media").unwrap_err();
        assert!(matches!(&e, ErroConfig::Inexistente { existentes, .. } if existentes == &["Curta", "Longa"]));
        let t = e.to_string();
        assert!(t.contains("'Media'") && t.contains("Curta, Longa"), "{t}");
        // peça sem tabela
        let (sem, _) = peca();
        assert!(sem.com_configuracao("X").unwrap_err().to_string().contains("não tem tabela"));
    }

    #[test]
    fn ativa_decide_o_documento_efetivo() {
        let (mut d, _) = com_tabela();
        assert!(matches!(d.efetivo().unwrap(), Cow::Borrowed(_)));
        d.tabela.ativa = "Curta".into();
        assert_eq!(d.efetivo().unwrap().props.pn, "P-050");
        d.tabela.ativa = "Sumiu".into();
        assert!(d.efetivo().is_err());
    }

    #[test]
    fn csv_ida_e_volta_com_aspas_e_separadores() {
        let (mut d, _) = com_tabela();
        d.tabela.linhas[0].nome = "Curta; \"especial\"".into();
        let csv = d.tabela_para_csv();
        assert!(csv.starts_with("Configuração;Comprimento;PN;suprimir:"), "{csv}");
        let mut outra = d.clone();
        outra.tabela = TabelaProjeto::default();
        outra.importar_tabela_csv(&csv).unwrap();
        assert_eq!(outra.tabela.colunas, d.tabela.colunas);
        assert_eq!(outra.tabela.linhas, d.tabela.linhas);
        // vírgula e CRLF também são lidos
        let mut c = d.clone();
        c.importar_tabela_csv("Configuração,Comprimento,PN\r\nA,10,X-1\r\nB,20,X-2\r\n").unwrap();
        assert_eq!(c.tabela.nomes(), ["A", "B"]);
        assert_eq!(c.com_configuracao("B").unwrap().params[0].expr, "20");
    }

    #[test]
    fn csv_com_coluna_desconhecida_ou_repetida_nao_mexe_na_tabela() {
        let (mut d, _) = com_tabela();
        let antes = d.tabela.clone();
        let e = d.importar_tabela_csv("Configuração;Inexistente\nA;1\n").unwrap_err();
        assert!(e.to_string().contains("'Inexistente'"), "{e}");
        assert!(d.importar_tabela_csv("Configuração;PN;PN\nA;1;2\n").is_err());
        assert!(d.importar_tabela_csv("Configuração;Comprimento\nA;1\nA;2\n").is_err(), "nome repetido");
        assert!(d.importar_tabela_csv("").is_err());
        assert!(d.importar_tabela_csv("Configuração;PN\n\"A;1\n").is_err(), "aspas abertas");
        assert_eq!(d.tabela, antes);
    }

    #[test]
    fn importar_mantem_a_ativa_so_se_ela_continua_existindo() {
        let (mut d, _) = com_tabela();
        d.tabela.ativa = "Longa".into();
        d.importar_tabela_csv("Configuração;PN\nLonga;L\nOutra;O\n").unwrap();
        assert_eq!(d.tabela.ativa, "Longa");
        d.importar_tabela_csv("Configuração;PN\nOutra;O\n").unwrap();
        assert_eq!(d.tabela.ativa, "", "ativa que sumiu volta ao base, com a tabela à vista");
    }

    #[test]
    fn coluna_que_perdeu_o_alvo_da_erro_ao_aplicar() {
        let (mut d, _) = com_tabela();
        d.params.retain(|p| p.name != "Comprimento");
        let e = d.com_configuracao("Curta").unwrap_err();
        assert!(e.to_string().contains("'Comprimento'"), "{e}");
    }

    #[test]
    fn nova_configuracao_parte_dos_valores_atuais() {
        let (mut d, _) = com_tabela();
        d.props.pn = "BASE".into();
        assert!(d.nova_configuracao("Copia"));
        assert!(!d.nova_configuracao("Copia"), "nome repetido");
        let l = d.tabela.linha("Copia").unwrap();
        assert_eq!(l.celulas[0], "100");
        assert_eq!(l.celulas[1], "BASE");
    }
}
