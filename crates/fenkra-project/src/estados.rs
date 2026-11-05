// SPDX-License-Identifier: LGPL-2.1-or-later
//! Ciclo de vida com tabela de transições, esquema de revisão e lista de conferência (B265).
//!
//! Antes, qualquer documento podia ir de qualquer estado para qualquer outro (`set_state`). Agora o
//! ciclo é **dados**: um JSON com os estados, as transições permitidas (`de`, `para`, `papel`,
//! `exige_motivo`) e o esquema de revisão. Uma mudança que não está na tabela é recusada com o motivo
//! dito em português; a troca livre deixou de existir.
//!
//! ```json
//! {
//!   "nome": "Três estados",
//!   "estados": ["EmTrabalho", "Liberado", "Obsoleto"],
//!   "inicial": "EmTrabalho",
//!   "esquema_revisao": "letras",
//!   "transicoes": [
//!     { "de": "EmTrabalho", "para": "Liberado", "papel": "*", "exige_motivo": false },
//!     { "de": "Liberado",   "para": "Obsoleto", "papel": "*", "exige_motivo": true }
//!   ]
//! }
//! ```
//!
//! O papel `*` vale para qualquer usuário. Dois modelos acompanham o programa: [`Ciclo::padrao`]
//! (três estados) e [`Ciclo::com_aprovacao`] (acrescenta `EmAprovacao`: quem projeta pede a aprovação
//! e só o aprovador libera ou devolve).
//!
//! Liberar uma revisão é a transição para `Liberado`, e só acontece junto com a criação da letra:
//! por isso `Project::transicionar` recusa ir para `Liberado` e `Project::liberar` roda antes a lista
//! de conferência ([`Conferencia`]): recálculo sem erro, referências resolvidas, PN definido e desenho
//! atualizado. Falhou um item que bloqueia, a letra não é criada.

use crate::DocState;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EsquemaRevisao {
    /// A, B, ..., Z, AA...
    #[default]
    Letras,
    /// 1, 2, 3...
    Numeros,
    /// 0.1, 0.2, ...: revisões de prévia, antes da primeira liberação formal
    Previa,
}

impl EsquemaRevisao {
    pub fn rotulo(self) -> &'static str {
        match self {
            EsquemaRevisao::Letras => "Letras (A, B, C...)",
            EsquemaRevisao::Numeros => "Números (1, 2, 3...)",
            EsquemaRevisao::Previa => "Prévia (0.1, 0.2...)",
        }
    }

    /// Próxima revisão depois de `anterior`. Se a anterior não é do formato do esquema (o projeto
    /// trocou de esquema no meio), a sequência recomeça no primeiro valor do esquema novo.
    pub fn proxima(self, anterior: Option<&str>) -> String {
        match self {
            EsquemaRevisao::Letras => match anterior {
                Some(p) if !p.is_empty() && p.bytes().all(|b| b.is_ascii_uppercase()) => crate::next_label(Some(p)),
                _ => "A".into(),
            },
            EsquemaRevisao::Numeros => match anterior.and_then(|p| p.parse::<u32>().ok()) {
                Some(n) => (n + 1).to_string(),
                None => "1".into(),
            },
            EsquemaRevisao::Previa => match anterior.and_then(|p| p.strip_prefix("0.")).and_then(|n| n.parse::<u32>().ok()) {
                Some(n) => format!("0.{}", n + 1),
                None => "0.1".into(),
            },
        }
    }
}

fn qualquer() -> String {
    "*".into()
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transicao {
    pub de: DocState,
    pub para: DocState,
    /// papel que pode fazer esta mudança; `*` = qualquer
    #[serde(default = "qualquer")]
    pub papel: String,
    #[serde(default)]
    pub exige_motivo: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ciclo {
    pub nome: String,
    pub estados: Vec<DocState>,
    pub inicial: DocState,
    pub transicoes: Vec<Transicao>,
    #[serde(default)]
    pub esquema_revisao: EsquemaRevisao,
}

fn t(de: DocState, para: DocState, papel: &str, exige_motivo: bool) -> Transicao {
    Transicao { de, para, papel: papel.into(), exige_motivo }
}

impl Default for Ciclo {
    fn default() -> Self {
        Ciclo::padrao()
    }
}

impl Ciclo {
    /// Modelo de três estados: quem edita também libera.
    pub fn padrao() -> Ciclo {
        use DocState::*;
        Ciclo {
            nome: "Três estados".into(),
            estados: vec![EmTrabalho, Liberado, Obsoleto],
            inicial: EmTrabalho,
            esquema_revisao: EsquemaRevisao::Letras,
            transicoes: vec![
                t(EmTrabalho, Liberado, "*", false),
                t(Liberado, EmTrabalho, "*", true),
                t(EmTrabalho, Obsoleto, "*", true),
                t(Liberado, Obsoleto, "*", true),
                t(Obsoleto, EmTrabalho, "*", true),
            ],
        }
    }

    /// Modelo com aprovação opcional: o projetista pode pedir aprovação; o aprovador libera ou devolve.
    /// A transição direta `EmTrabalho -> Liberado` continua existindo, só para o aprovador.
    pub fn com_aprovacao() -> Ciclo {
        use DocState::*;
        Ciclo {
            nome: "Com aprovação".into(),
            estados: vec![EmTrabalho, EmAprovacao, Liberado, Obsoleto],
            inicial: EmTrabalho,
            esquema_revisao: EsquemaRevisao::Letras,
            transicoes: vec![
                t(EmTrabalho, EmAprovacao, "projetista", false),
                t(EmAprovacao, EmTrabalho, "aprovador", true),
                t(EmAprovacao, EmTrabalho, "projetista", true),
                t(EmAprovacao, Liberado, "aprovador", false),
                t(EmTrabalho, Liberado, "aprovador", false),
                t(Liberado, EmTrabalho, "projetista", true),
                t(EmTrabalho, Obsoleto, "projetista", true),
                t(Liberado, Obsoleto, "aprovador", true),
                t(Obsoleto, EmTrabalho, "aprovador", true),
            ],
        }
    }

    /// Lê um ciclo de JSON e o valida.
    pub fn de_json(texto: &str) -> Result<Ciclo, String> {
        let c: Ciclo = serde_json::from_str(texto).map_err(|e| format!("ciclo de vida inválido: {e}"))?;
        c.validar()?;
        Ok(c)
    }

    pub fn para_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// Coerência da tabela: estados sem repetição, transições entre estados declarados, e a liberação
    /// precisa ser alcançável a partir do estado inicial (senão nenhum documento poderia ser liberado).
    pub fn validar(&self) -> Result<(), String> {
        if self.estados.is_empty() {
            return Err("o ciclo não tem estados".into());
        }
        for (i, e) in self.estados.iter().enumerate() {
            if self.estados[..i].contains(e) {
                return Err(format!("estado repetido: {}", e.label()));
            }
        }
        if !self.estados.contains(&self.inicial) {
            return Err(format!("o estado inicial {} não está na lista de estados", self.inicial.label()));
        }
        for x in &self.transicoes {
            for e in [x.de, x.para] {
                if !self.estados.contains(&e) {
                    return Err(format!("a transição {} -> {} usa o estado {}, que não está na lista", x.de.label(), x.para.label(), e.label()));
                }
            }
            if x.de == x.para {
                return Err(format!("a transição {0} -> {0} não muda nada", x.de.label()));
            }
            if x.papel.trim().is_empty() {
                return Err(format!("a transição {} -> {} está sem papel (use * para qualquer)", x.de.label(), x.para.label()));
            }
        }
        // alcança Liberado partindo do inicial
        let mut alcancados = vec![self.inicial];
        let mut i = 0;
        while i < alcancados.len() {
            let de = alcancados[i];
            for x in self.transicoes.iter().filter(|x| x.de == de) {
                if !alcancados.contains(&x.para) {
                    alcancados.push(x.para);
                }
            }
            i += 1;
        }
        if self.estados.contains(&DocState::Liberado) && !alcancados.contains(&DocState::Liberado) {
            return Err("nenhuma sequência de transições leva do estado inicial a Liberado".into());
        }
        Ok(())
    }

    /// A mudança `de -> para` é permitida para quem tem `papeis`? Devolve a regra aplicada ou o motivo
    /// da recusa. `motivo` vazio é recusado quando a regra exige motivo.
    pub fn verificar(&self, de: DocState, para: DocState, papeis: &[String], motivo: &str) -> Result<&Transicao, String> {
        if de == para {
            return Err(format!("o documento já está em {}", de.label()));
        }
        let candidatas: Vec<&Transicao> = self.transicoes.iter().filter(|x| x.de == de && x.para == para).collect();
        if candidatas.is_empty() {
            let destinos: Vec<&str> = self.transicoes.iter().filter(|x| x.de == de).map(|x| x.para.label()).collect();
            let lista = if destinos.is_empty() { "nenhuma".to_string() } else { destinos.join(", ") };
            return Err(format!("a mudança de {} para {} não existe no ciclo \"{}\"; a partir de {} é possível: {}", de.label(), para.label(), self.nome, de.label(), lista));
        }
        let Some(regra) = candidatas.iter().find(|x| x.papel == "*" || papeis.iter().any(|p| p == &x.papel)).copied() else {
            let exigidos: Vec<&str> = candidatas.iter().map(|x| x.papel.as_str()).collect();
            return Err(format!("a mudança de {} para {} exige o papel {}", de.label(), para.label(), exigidos.join(" ou ")));
        };
        if regra.exige_motivo && motivo.trim().is_empty() {
            return Err(format!("a mudança de {} para {} exige um motivo", de.label(), para.label()));
        }
        Ok(regra)
    }

    /// Transições que `papeis` pode fazer a partir de `de` (para montar os botões da interface).
    pub fn disponiveis(&self, de: DocState, papeis: &[String]) -> Vec<&Transicao> {
        self.transicoes.iter().filter(|x| x.de == de && (x.papel == "*" || papeis.iter().any(|p| p == &x.papel))).collect()
    }
}

// ------------------------------------------------------------------------- lista de conferência

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EstadoItem {
    Ok,
    /// bloqueia a liberação
    Falha,
    /// o programa não teve como conferir agora (ex.: o documento não está aberto para recalcular)
    NaoVerificado,
    /// não se aplica (ex.: nenhum desenho usa a peça)
    NaoAplicavel,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ItemConferencia {
    pub chave: &'static str,
    pub rotulo: &'static str,
    pub estado: EstadoItem,
    /// o que falhou, em uma frase (vazio quando está Ok)
    pub detalhe: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Conferencia {
    pub itens: Vec<ItemConferencia>,
}

impl Conferencia {
    /// Algum item falhou: a letra não pode ser criada.
    pub fn bloqueia(&self) -> bool {
        self.itens.iter().any(|i| i.estado == EstadoItem::Falha)
    }

    pub fn falhas(&self) -> impl Iterator<Item = &ItemConferencia> {
        self.itens.iter().filter(|i| i.estado == EstadoItem::Falha)
    }

    pub fn resumo(&self) -> String {
        self.falhas().map(|i| format!("{}: {}", i.rotulo, i.detalhe)).collect::<Vec<_>>().join("; ")
    }
}

/// O que só quem tem o documento aberto sabe dizer. `None` = não conferido.
#[derive(Clone, Debug, Default)]
pub struct Externos {
    /// resultado do recálculo do documento: `Err` traz a mensagem do primeiro erro
    pub recalculo: Option<Result<(), String>>,
    /// substitui a conferência automática do desenho (por data de gravação)
    pub desenho_atualizado: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use DocState::*;

    fn papeis(p: &[&str]) -> Vec<String> {
        p.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn esquemas_de_revisao() {
        assert_eq!(EsquemaRevisao::Letras.proxima(None), "A");
        assert_eq!(EsquemaRevisao::Letras.proxima(Some("Z")), "AA");
        assert_eq!(EsquemaRevisao::Numeros.proxima(None), "1");
        assert_eq!(EsquemaRevisao::Numeros.proxima(Some("9")), "10");
        assert_eq!(EsquemaRevisao::Previa.proxima(None), "0.1");
        assert_eq!(EsquemaRevisao::Previa.proxima(Some("0.9")), "0.10");
        // troca de esquema no meio do projeto: recomeça no esquema novo
        assert_eq!(EsquemaRevisao::Numeros.proxima(Some("C")), "1");
        assert_eq!(EsquemaRevisao::Letras.proxima(Some("0.3")), "A");
        assert_eq!(EsquemaRevisao::Letras.proxima(Some("7")), "A");
    }

    #[test]
    fn modelos_que_acompanham_o_programa_sao_validos() {
        Ciclo::padrao().validar().unwrap();
        Ciclo::com_aprovacao().validar().unwrap();
        for c in [Ciclo::padrao(), Ciclo::com_aprovacao()] {
            assert_eq!(Ciclo::de_json(&c.para_json()).unwrap(), c, "ida e volta pelo JSON");
        }
    }

    #[test]
    fn json_do_usuario_e_lido_com_padroes() {
        let c = Ciclo::de_json(
            r#"{"nome":"Mínimo","estados":["EmTrabalho","Liberado"],"inicial":"EmTrabalho",
                "transicoes":[{"de":"EmTrabalho","para":"Liberado"}]}"#,
        )
        .unwrap();
        assert_eq!(c.transicoes[0].papel, "*");
        assert!(!c.transicoes[0].exige_motivo);
        assert_eq!(c.esquema_revisao, EsquemaRevisao::Letras);
        let n = Ciclo::de_json(r#"{"nome":"N","estados":["EmTrabalho","Liberado"],"inicial":"EmTrabalho","esquema_revisao":"numeros","transicoes":[{"de":"EmTrabalho","para":"Liberado"}]}"#).unwrap();
        assert_eq!(n.esquema_revisao, EsquemaRevisao::Numeros);
    }

    #[test]
    fn tabelas_incoerentes_sao_recusadas_com_motivo() {
        let erro = |j: &str| Ciclo::de_json(j).unwrap_err();
        assert!(erro("{").contains("inválido"));
        assert!(erro(r#"{"nome":"x","estados":[],"inicial":"EmTrabalho","transicoes":[]}"#).contains("não tem estados"));
        assert!(erro(r#"{"nome":"x","estados":["EmTrabalho","EmTrabalho"],"inicial":"EmTrabalho","transicoes":[]}"#).contains("repetido"));
        assert!(erro(r#"{"nome":"x","estados":["Liberado"],"inicial":"EmTrabalho","transicoes":[]}"#).contains("inicial"));
        assert!(erro(r#"{"nome":"x","estados":["EmTrabalho","Liberado"],"inicial":"EmTrabalho","transicoes":[{"de":"EmTrabalho","para":"Obsoleto"}]}"#).contains("não está na lista"));
        assert!(erro(r#"{"nome":"x","estados":["EmTrabalho","Liberado"],"inicial":"EmTrabalho","transicoes":[]}"#).contains("leva do estado inicial a Liberado"));
        assert!(erro(r#"{"nome":"x","estados":["EmTrabalho","Liberado"],"inicial":"EmTrabalho","transicoes":[{"de":"EmTrabalho","para":"EmTrabalho"}]}"#).contains("não muda nada"));
    }

    #[test]
    fn transicao_fora_da_tabela_e_recusada_com_motivo() {
        let c = Ciclo::padrao();
        let todos = papeis(&["projetista"]);
        // Obsoleto -> Liberado não existe: precisa voltar a trabalho antes
        let e = c.verificar(Obsoleto, Liberado, &todos, "x").unwrap_err();
        assert!(e.contains("não existe no ciclo") && e.contains("Em trabalho"), "{e}");
        assert!(c.verificar(EmTrabalho, Liberado, &todos, "").is_ok());
        // exige motivo
        assert!(c.verificar(Liberado, Obsoleto, &todos, "  ").unwrap_err().contains("exige um motivo"));
        assert!(c.verificar(Liberado, Obsoleto, &todos, "substituída").is_ok());
        assert!(c.verificar(Liberado, Liberado, &todos, "x").unwrap_err().contains("já está"));
    }

    #[test]
    fn papel_errado_e_recusado_e_o_certo_passa() {
        let c = Ciclo::com_aprovacao();
        let proj = papeis(&["projetista"]);
        let aprov = papeis(&["projetista", "aprovador"]);
        assert!(c.verificar(EmTrabalho, EmAprovacao, &proj, "").is_ok());
        let e = c.verificar(EmAprovacao, Liberado, &proj, "").unwrap_err();
        assert!(e.contains("exige o papel aprovador"), "{e}");
        assert!(c.verificar(EmAprovacao, Liberado, &aprov, "").is_ok());
        // devolver pede motivo
        assert!(c.verificar(EmAprovacao, EmTrabalho, &aprov, "").unwrap_err().contains("motivo"));
        let botoes: Vec<DocState> = c.disponiveis(EmTrabalho, &proj).iter().map(|x| x.para).collect();
        assert_eq!(botoes, vec![EmAprovacao, Obsoleto]);
    }
}
