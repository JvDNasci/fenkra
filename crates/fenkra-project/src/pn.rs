// SPDX-License-Identifier: LGPL-2.1-or-later
//! Regras de part-number por máscara e ambientes de numeração (B47, PNEngine).
//!
//! Uma máscara é um texto com campos entre chaves, por exemplo `{PREFIXO}-{ANO}-{TIPO}-{SEQ:4}`:
//!
//! | campo | vira |
//! |---|---|
//! | `{PREFIXO}` | o prefixo do ambiente (ex.: `FK`) |
//! | `{ANO}` | ano com 4 dígitos |
//! | `{TIPO}` | código do tipo de peça (`FAB`, `COM`, `HW`, `CHP`, `EST`, `FAN`) |
//! | `{AMB}` | nome curto do ambiente (ex.: `PROTO`) |
//! | `{CAMPO:nome}` | valor da propriedade personalizada `nome` |
//! | `{SEQ:N}` | contador do ambiente com `N` dígitos (1 a 9), exatamente um por máscara |
//!
//! Por que o `{SEQ}` é obrigatório: sem ele dois documentos teriam o mesmo PN, e o projeto recusa PN
//! duplicado. Por que ambientes: protótipo e produção costumam numerar separado (um `PROTO-0007` não
//! deve queimar o `FK-0007` de produção); cada ambiente tem máscara e contador próprios.
//!
//! A prévia ([`previa`]) devolve segmentos com a marca `preenchido`: o campo ainda sem valor aparece
//! como marcador (`AAAA`, `TIPO`) e a interface o desenha em cinza, enquanto o usuário preenche.

use serde::{Deserialize, Serialize};

/// Máscara usada quando o projeto não define nenhuma: equivale ao `FK-0001` histórico.
pub const MASCARA_PADRAO: &str = "{PREFIXO}{SEQ:4}";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AmbientePn {
    pub nome: String,
    pub prefixo: String,
    pub mascara: String,
    /// próximo número da sequência deste ambiente
    pub proximo: u32,
}

impl AmbientePn {
    pub fn novo(nome: &str, prefixo: &str, mascara: &str) -> Self {
        AmbientePn { nome: nome.into(), prefixo: prefixo.into(), mascara: mascara.into(), proximo: 1 }
    }
}

/// Valores que alimentam os campos da máscara. Campo `None` ou vazio fica como marcador na prévia.
#[derive(Clone, Debug, Default)]
pub struct CamposPn {
    pub prefixo: String,
    pub ambiente: String,
    pub ano: Option<u32>,
    pub tipo: String,
    pub personalizados: Vec<(String, String)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ErroMascara {
    Vazia,
    ChaveAberta,
    CampoDesconhecido(String),
    LarguraInvalida(String),
    SemSequencia,
    SequenciaRepetida,
}

impl std::fmt::Display for ErroMascara {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErroMascara::Vazia => write!(f, "a máscara está vazia"),
            ErroMascara::ChaveAberta => write!(f, "falta fechar uma chave }}"),
            ErroMascara::CampoDesconhecido(c) => write!(f, "campo desconhecido {{{c}}}"),
            ErroMascara::LarguraInvalida(c) => write!(f, "largura inválida em {{{c}}}: use de 1 a 9 dígitos"),
            ErroMascara::SemSequencia => write!(f, "a máscara precisa de um {{SEQ:N}}, senão dois documentos teriam o mesmo PN"),
            ErroMascara::SequenciaRepetida => write!(f, "a máscara tem mais de um {{SEQ}}"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Peca {
    Literal(String),
    Prefixo,
    Ambiente,
    Ano,
    Tipo,
    Campo(String),
    Seq(usize),
}

fn analisar(mascara: &str) -> Result<Vec<Peca>, ErroMascara> {
    if mascara.trim().is_empty() {
        return Err(ErroMascara::Vazia);
    }
    let mut pecas = vec![];
    let mut lit = String::new();
    let mut it = mascara.chars();
    while let Some(c) = it.next() {
        if c != '{' {
            lit.push(c);
            continue;
        }
        let mut nome = String::new();
        let mut fechou = false;
        for d in it.by_ref() {
            if d == '}' {
                fechou = true;
                break;
            }
            nome.push(d);
        }
        if !fechou {
            return Err(ErroMascara::ChaveAberta);
        }
        if !lit.is_empty() {
            pecas.push(Peca::Literal(std::mem::take(&mut lit)));
        }
        let (chave, arg) = match nome.split_once(':') {
            Some((k, a)) => (k.trim().to_uppercase(), Some(a.trim().to_string())),
            None => (nome.trim().to_uppercase(), None),
        };
        pecas.push(match (chave.as_str(), arg) {
            ("PREFIXO", None) => Peca::Prefixo,
            ("AMB", None) => Peca::Ambiente,
            ("ANO", None) => Peca::Ano,
            ("TIPO", None) => Peca::Tipo,
            ("CAMPO", Some(a)) if !a.is_empty() => Peca::Campo(a),
            ("SEQ", a) => {
                let n = match a {
                    None => 4,
                    Some(a) => a.parse::<usize>().ok().filter(|n| (1..=9).contains(n)).ok_or_else(|| ErroMascara::LarguraInvalida(nome.clone()))?,
                };
                Peca::Seq(n)
            }
            _ => return Err(ErroMascara::CampoDesconhecido(nome)),
        });
    }
    if !lit.is_empty() {
        pecas.push(Peca::Literal(lit));
    }
    match pecas.iter().filter(|p| matches!(p, Peca::Seq(_))).count() {
        0 => Err(ErroMascara::SemSequencia),
        1 => Ok(pecas),
        _ => Err(ErroMascara::SequenciaRepetida),
    }
}

/// Confere a máscara. A interface mostra o erro ao lado do campo, antes de salvar.
pub fn validar(mascara: &str) -> Result<(), ErroMascara> {
    analisar(mascara).map(|_| ())
}

/// Um pedaço da prévia. `preenchido == false` é marcador de campo ainda sem valor (cinza na tela).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Segmento {
    pub texto: String,
    pub preenchido: bool,
}

/// Prévia ao vivo do PN: a máscara aplicada aos campos já preenchidos, com `seq` no lugar do contador.
pub fn previa(mascara: &str, campos: &CamposPn, seq: u32) -> Result<Vec<Segmento>, ErroMascara> {
    let valor = |v: &str, marcador: &str| -> Segmento {
        if v.trim().is_empty() {
            Segmento { texto: marcador.into(), preenchido: false }
        } else {
            Segmento { texto: v.trim().into(), preenchido: true }
        }
    };
    Ok(analisar(mascara)?
        .into_iter()
        .map(|p| match p {
            Peca::Literal(t) => Segmento { texto: t, preenchido: true },
            Peca::Prefixo => valor(&campos.prefixo, "PREFIXO"),
            Peca::Ambiente => valor(&campos.ambiente, "AMB"),
            Peca::Ano => match campos.ano {
                Some(a) => Segmento { texto: format!("{a:04}"), preenchido: true },
                None => Segmento { texto: "AAAA".into(), preenchido: false },
            },
            Peca::Tipo => valor(&campos.tipo, "TIPO"),
            Peca::Campo(n) => {
                let v = campos.personalizados.iter().find(|(k, _)| k.eq_ignore_ascii_case(&n)).map(|(_, v)| v.as_str()).unwrap_or("");
                valor(v, &n.to_uppercase())
            }
            Peca::Seq(w) => Segmento { texto: format!("{seq:0w$}"), preenchido: true },
        })
        .collect())
}

/// Texto da prévia inteira.
pub fn texto(segmentos: &[Segmento]) -> String {
    segmentos.iter().map(|s| s.texto.as_str()).collect()
}

/// Todos os campos têm valor: o PN da prévia é o definitivo.
pub fn completo(segmentos: &[Segmento]) -> bool {
    segmentos.iter().all(|s| s.preenchido)
}

/// Exemplo para o tooltip da máscara, com valores de mostra: `{PREFIXO}-{ANO}-{SEQ:4}` -> `FK-2026-0001`.
pub fn exemplo(mascara: &str) -> Result<String, ErroMascara> {
    let c = CamposPn {
        prefixo: "FK".into(),
        ambiente: "PROD".into(),
        ano: Some(ano_atual()),
        tipo: "FAB".into(),
        personalizados: vec![],
    };
    // campos personalizados não têm valor de mostra: aparecem pelo nome, em maiúsculas
    Ok(texto(&previa(mascara, &c, 1)?))
}

/// Ano civil atual (UTC), sem depender de biblioteca de datas.
pub fn ano_atual() -> u32 {
    let dias = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() / 86_400).unwrap_or(0) as i64;
    ano_do_dia(dias)
}

/// Ano de um dia contado desde 1970-01-01 (algoritmo civil de Howard Hinnant, domínio público).
fn ano_do_dia(dias: i64) -> u32 {
    let z = dias + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn campos() -> CamposPn {
        CamposPn { prefixo: "FK".into(), ambiente: "PROTO".into(), ano: Some(2026), tipo: "FAB".into(), personalizados: vec![("Cliente".into(), "ACME".into())] }
    }

    #[test]
    fn mascara_aplicada_aos_campos() {
        let s = previa("{PREFIXO}-{ANO}-{TIPO}-{SEQ:4}", &campos(), 7).unwrap();
        assert_eq!(texto(&s), "FK-2026-FAB-0007");
        assert!(completo(&s));
        assert_eq!(texto(&previa("{AMB}/{CAMPO:cliente}/{SEQ:3}", &campos(), 12).unwrap()), "PROTO/ACME/012");
        assert_eq!(texto(&previa(MASCARA_PADRAO, &campos(), 1).unwrap()), "FK0001", "máscara padrão = formato histórico");
    }

    #[test]
    fn previa_marca_em_cinza_o_que_ainda_falta_preencher() {
        let mut c = campos();
        c.tipo.clear();
        c.ano = None;
        let s = previa("{PREFIXO}-{ANO}-{TIPO}-{SEQ:4}", &c, 3).unwrap();
        assert_eq!(texto(&s), "FK-AAAA-TIPO-0003");
        assert!(!completo(&s));
        let pend: Vec<&str> = s.iter().filter(|x| !x.preenchido).map(|x| x.texto.as_str()).collect();
        assert_eq!(pend, vec!["AAAA", "TIPO"]);
        // ao preencher, o marcador some e a prévia fica completa
        c.tipo = "COM".into();
        c.ano = Some(2027);
        assert!(completo(&previa("{PREFIXO}-{ANO}-{TIPO}-{SEQ:4}", &c, 3).unwrap()));
    }

    #[test]
    fn mascaras_invalidas_sao_recusadas_com_motivo() {
        assert_eq!(validar(""), Err(ErroMascara::Vazia));
        assert_eq!(validar("FK-{ANO}"), Err(ErroMascara::SemSequencia));
        assert_eq!(validar("{SEQ:4}{SEQ:2}"), Err(ErroMascara::SequenciaRepetida));
        assert_eq!(validar("{SEQ:0}"), Err(ErroMascara::LarguraInvalida("SEQ:0".into())));
        assert_eq!(validar("{SEQ:10}"), Err(ErroMascara::LarguraInvalida("SEQ:10".into())));
        assert_eq!(validar("{XYZ}{SEQ}"), Err(ErroMascara::CampoDesconhecido("XYZ".into())));
        assert_eq!(validar("{SEQ:4"), Err(ErroMascara::ChaveAberta));
        assert_eq!(validar("{CAMPO:}{SEQ}"), Err(ErroMascara::CampoDesconhecido("CAMPO:".into())));
        assert!(validar("{prefixo}-{seq:5}").is_ok(), "campos não diferenciam maiúscula");
    }

    #[test]
    fn exemplo_do_tooltip() {
        let e = exemplo("{PREFIXO}-{TIPO}-{SEQ:4}").unwrap();
        assert_eq!(e, "FK-FAB-0001");
        assert!(exemplo("{ANO}-{SEQ}").unwrap().ends_with("-0001"));
        assert!(exemplo("sem sequência").is_err());
    }

    #[test]
    fn ano_civil_do_dia() {
        assert_eq!(ano_do_dia(0), 1970);
        assert_eq!(ano_do_dia(365), 1971);
        assert_eq!(ano_do_dia(10_957), 2000); // 2000-01-01
        assert_eq!(ano_do_dia(10_956), 1999);
        assert_eq!(ano_do_dia(20_453), 2025); // 2025-12-31 (dia 20453)
        assert_eq!(ano_do_dia(20_454), 2026); // 2026-01-01
    }
}
