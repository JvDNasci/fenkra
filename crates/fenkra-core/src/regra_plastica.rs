// SPDX-License-Identifier: LGPL-2.1-or-later
//! B198: regra plástica do documento.
//!
//! Uma peça injetada nasce de três números: a espessura nominal da parede, o ângulo de saída e o
//! raio nominal dos cantos. A [`RegraPlastica`] guarda esses três, os limites aceitos (parede mínima
//! e máxima, saída mínima, raio mínimo, fração da nervura e da base do filete) e o nome e a versão da
//! entrada da biblioteca de onde veio.
//!
//! **Como os recursos consomem a regra.** O recálculo põe os valores da regra no conjunto de
//! variáveis das fórmulas ([`RegraPlastica::vars`]): `plastico_espessura`, `plastico_saida`,
//! `plastico_raio` e `plastico_nervura`. Um comando criado com a regra no documento escreve a fórmula
//! (por exemplo a casca com espessura `plastico_espessura`), então mudar a regra muda todos os
//! recursos que a citam, sem tocar em nenhum. Escrever um número no lugar da fórmula é a sobrescrita,
//! e a tela só a permite em modo avançado ([`RegraPlastica::avancado`]).
//!
//! **Diagnóstico.** Mesmo com sobrescrita, o recálculo compara o valor de cada recurso com os
//! limites ([`RegraPlastica::confere`]) e avisa na árvore quando ele sai da regra.
//!
//! **Biblioteca.** As regras de fábrica vêm em `regras_plasticas.json`; a pessoa grava as dela numa
//! pasta (`carregar_pasta`, `guardar`) e cada gravação sobe a versão. O documento guarda uma cópia
//! com a versão: quando a biblioteca tem uma versão mais nova da mesma regra, a tela oferece trazer.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Nome das variáveis que a regra põe nas fórmulas.
pub const VAR_ESPESSURA: &str = "plastico_espessura";
pub const VAR_SAIDA: &str = "plastico_saida";
pub const VAR_RAIO: &str = "plastico_raio";
pub const VAR_NERVURA: &str = "plastico_nervura";

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct RegraPlastica {
    pub nome: String,
    /// material de referência (texto livre: ABS, PC, PP...)
    #[serde(default)]
    pub material: String,
    /// versão da entrada da biblioteca; sobe a cada gravação
    #[serde(default = "um")]
    pub versao: u32,
    /// espessura nominal da parede, mm
    pub espessura: f64,
    /// ângulo de saída nominal, graus
    pub saida: f64,
    /// raio nominal dos cantos, mm
    pub raio: f64,
    /// parede mínima e máxima aceitas, mm
    pub espessura_min: f64,
    pub espessura_max: f64,
    /// saída mínima aceita, graus
    pub saida_min: f64,
    /// raio mínimo aceito nos cantos e filetes, mm
    pub raio_min: f64,
    /// espessura da nervura como fração da parede nominal (nervura grossa chupa a face oposta)
    #[serde(default = "nervura_padrao")]
    pub nervura_fracao: f64,
    /// nervura acima disso, em fração da parede, é aviso
    #[serde(default = "nervura_max_padrao")]
    pub nervura_max: f64,
    /// modo avançado: os comandos aceitam número próprio no lugar da regra
    #[serde(default)]
    pub avancado: bool,
}

fn um() -> u32 {
    1
}
fn nervura_padrao() -> f64 {
    0.6
}
fn nervura_max_padrao() -> f64 {
    0.75
}

/// O que um recurso mede contra a regra.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Grandeza {
    /// parede de casca, espessura, parede fina de extrusão
    Parede,
    /// espessura de nervura
    Nervura,
    /// ângulo de saída
    Saida,
    /// raio de filete ou de canto
    Raio,
}

const FABRICA: &str = include_str!("../regras_plasticas.json");

impl RegraPlastica {
    /// Regras que vêm com o programa.
    pub fn de_fabrica() -> Vec<RegraPlastica> {
        serde_json::from_str::<Vec<RegraPlastica>>(FABRICA).unwrap_or_default().into_iter().filter(|r| r.valida().is_ok()).collect()
    }

    pub fn de_json(t: &str) -> Result<RegraPlastica, String> {
        let r: RegraPlastica = serde_json::from_str(t).map_err(|e| format!("regra plástica inválida: {e}"))?;
        r.valida()?;
        Ok(r)
    }

    pub fn para_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// Os números fazem sentido juntos?
    pub fn valida(&self) -> Result<(), String> {
        if self.nome.trim().is_empty() {
            return Err("regra plástica sem nome".into());
        }
        let pos = [("espessura", self.espessura), ("espessura mínima", self.espessura_min), ("espessura máxima", self.espessura_max), ("fração da nervura", self.nervura_fracao)];
        if let Some((n, _)) = pos.iter().find(|(_, v)| !(v.is_finite() && *v > 0.)) {
            return Err(format!("{}: {n} precisa ser positiva", self.nome));
        }
        if self.espessura_min > self.espessura_max {
            return Err(format!("{}: parede mínima maior que a máxima", self.nome));
        }
        if self.espessura < self.espessura_min - 1e-9 || self.espessura > self.espessura_max + 1e-9 {
            return Err(format!("{}: a espessura nominal sai dos limites da própria regra", self.nome));
        }
        if !(0. ..=45.).contains(&self.saida) || self.saida < self.saida_min - 1e-9 {
            return Err(format!("{}: a saída nominal precisa ficar entre a mínima e 45 graus", self.nome));
        }
        if self.raio < 0. || self.raio < self.raio_min - 1e-9 {
            return Err(format!("{}: o raio nominal fica abaixo do mínimo", self.nome));
        }
        Ok(())
    }

    /// Espessura que a nervura deve ter, mm.
    pub fn nervura(&self) -> f64 {
        self.espessura * self.nervura_fracao
    }

    /// Variáveis das fórmulas, com os valores da regra (valem mesmo que exista parâmetro de mesmo nome).
    pub fn vars(&self, base: &HashMap<String, f64>) -> HashMap<String, f64> {
        let mut v = base.clone();
        v.insert(VAR_ESPESSURA.into(), self.espessura);
        v.insert(VAR_SAIDA.into(), self.saida);
        v.insert(VAR_RAIO.into(), self.raio);
        v.insert(VAR_NERVURA.into(), self.nervura());
        v
    }

    /// Fórmula padrão de um campo quando a regra existe.
    pub fn formula(g: Grandeza) -> &'static str {
        match g {
            Grandeza::Parede => VAR_ESPESSURA,
            Grandeza::Nervura => VAR_NERVURA,
            Grandeza::Saida => VAR_SAIDA,
            Grandeza::Raio => VAR_RAIO,
        }
    }

    /// Confere um valor contra a regra. Devolve o motivo quando sai dela.
    pub fn confere(&self, g: Grandeza, v: f64) -> Option<String> {
        let f = |x: f64| {
            let s = format!("{x:.3}");
            s.trim_end_matches('0').trim_end_matches('.').to_string()
        };
        match g {
            Grandeza::Parede if v < self.espessura_min - 1e-9 => Some(format!("parede de {} mm abaixo da mínima de {} mm", f(v), f(self.espessura_min))),
            Grandeza::Parede if v > self.espessura_max + 1e-9 => Some(format!("parede de {} mm acima da máxima de {} mm", f(v), f(self.espessura_max))),
            Grandeza::Nervura if v > self.espessura * self.nervura_max + 1e-9 => {
                Some(format!("nervura de {} mm passa de {}% da parede ({} mm): chupa a face oposta", f(v), f(self.nervura_max * 100.), f(self.espessura * self.nervura_max)))
            }
            Grandeza::Nervura if v < self.espessura_min - 1e-9 => Some(format!("nervura de {} mm abaixo da parede mínima de {} mm", f(v), f(self.espessura_min))),
            Grandeza::Saida if v.abs() < self.saida_min - 1e-9 => Some(format!("saída de {}° abaixo da mínima de {}°", f(v.abs()), f(self.saida_min))),
            Grandeza::Raio if v < self.raio_min - 1e-9 => Some(format!("raio de {} mm abaixo do mínimo de {} mm", f(v), f(self.raio_min))),
            _ => None,
        }
    }
}

/// Regras de fábrica trocadas pelas da pasta do usuário: um JSON com o mesmo nome e versão maior ou
/// igual substitui a de fábrica; nome novo entra na lista. Arquivo que não lê vai para o segundo valor.
pub fn carregar_pasta(pasta: &std::path::Path) -> (Vec<RegraPlastica>, Vec<String>) {
    let mut v = RegraPlastica::de_fabrica();
    let mut erros = vec![];
    let Ok(rd) = std::fs::read_dir(pasta) else { return (v, erros) };
    let mut arqs: Vec<_> = rd.filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "json")).collect();
    arqs.sort();
    for a in arqs {
        match std::fs::read_to_string(&a).map_err(|e| e.to_string()).and_then(|t| RegraPlastica::de_json(&t)) {
            Ok(r) => match v.iter_mut().find(|q| q.nome == r.nome) {
                Some(q) if r.versao >= q.versao => *q = r,
                Some(_) => {}
                None => v.push(r),
            },
            Err(e) => erros.push(format!("{}: {e}", a.display())),
        }
    }
    (v, erros)
}

/// Nome de arquivo seguro para a regra.
pub fn arquivo(nome: &str) -> String {
    let s: String = nome.chars().map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect();
    format!("{s}.json")
}

/// Grava a regra na pasta da biblioteca subindo a versão acima da que estiver lá. Devolve a versão nova.
pub fn guardar(pasta: &std::path::Path, regra: &RegraPlastica) -> Result<u32, String> {
    regra.valida()?;
    std::fs::create_dir_all(pasta).map_err(|e| e.to_string())?;
    let (atuais, _) = carregar_pasta(pasta);
    let antes = atuais.iter().find(|r| r.nome == regra.nome).map_or(0, |r| r.versao);
    let mut r = regra.clone();
    r.versao = antes.max(regra.versao) + 1;
    r.avancado = false;
    std::fs::write(pasta.join(arquivo(&r.nome)), r.para_json()).map_err(|e| e.to_string())?;
    Ok(r.versao)
}

/// A biblioteca tem uma versão mais nova da regra do documento?
pub fn mais_nova<'a>(doc: &RegraPlastica, biblioteca: &'a [RegraPlastica]) -> Option<&'a RegraPlastica> {
    biblioteca.iter().find(|r| r.nome == doc.nome && r.versao > doc.versao)
}

impl crate::doc::Document {
    /// Variáveis das fórmulas com a regra plástica, quando o documento tem uma.
    pub fn vars_com_plastico(&self, base: HashMap<String, f64>) -> HashMap<String, f64> {
        match &self.plastico {
            Some(r) => r.vars(&base),
            None => base,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn abs() -> RegraPlastica {
        RegraPlastica::de_fabrica().into_iter().find(|r| r.material == "ABS").unwrap()
    }

    #[test]
    fn regras_de_fabrica_leem_e_sao_validas() {
        let v = RegraPlastica::de_fabrica();
        assert!(v.len() >= 4, "{v:?}");
        let mut nomes: Vec<_> = v.iter().map(|r| r.nome.clone()).collect();
        nomes.sort();
        nomes.dedup();
        assert_eq!(nomes.len(), v.len());
    }

    #[test]
    fn variaveis_seguem_a_regra() {
        let r = abs();
        let v = r.vars(&HashMap::from([(VAR_ESPESSURA.to_string(), 99.)]));
        assert_eq!(v[VAR_ESPESSURA], r.espessura, "a regra vence o parâmetro de mesmo nome");
        assert!((v[VAR_NERVURA] - r.espessura * r.nervura_fracao).abs() < 1e-12);
        assert_eq!(crate::expr::eval_str("plastico_espessura*2", &v).unwrap(), 2. * r.espessura);
    }

    #[test]
    fn confere_cada_grandeza() {
        let r = abs();
        assert!(r.confere(Grandeza::Parede, r.espessura).is_none());
        assert!(r.confere(Grandeza::Parede, r.espessura_min / 2.).unwrap().contains("abaixo"));
        assert!(r.confere(Grandeza::Parede, r.espessura_max * 2.).unwrap().contains("acima"));
        assert!(r.confere(Grandeza::Nervura, r.nervura()).is_none());
        assert!(r.confere(Grandeza::Nervura, r.espessura).unwrap().contains("chupa"));
        assert!(r.confere(Grandeza::Saida, r.saida_min / 2.).is_some());
        assert!(r.confere(Grandeza::Saida, -r.saida).is_none(), "o sinal da saída é só o sentido");
        assert!(r.confere(Grandeza::Raio, r.raio_min / 2.).is_some());
    }

    #[test]
    fn valida_recusa_regra_incoerente() {
        let mut r = abs();
        r.espessura = r.espessura_max + 1.;
        assert!(r.valida().is_err());
        let mut r = abs();
        r.saida = 0.;
        assert!(r.valida().is_err());
        assert!(RegraPlastica::de_json("{").is_err());
    }

    #[test]
    fn biblioteca_versiona_e_avisa_quando_ha_mais_nova() {
        let dir = std::env::temp_dir().join(format!("regra_plastica_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut r = abs();
        let doc = r.clone();
        r.espessura = r.espessura_min + 0.1;
        let v1 = guardar(&dir, &r).unwrap();
        assert_eq!(v1, doc.versao + 1);
        let v2 = guardar(&dir, &r).unwrap();
        assert_eq!(v2, v1 + 1, "cada gravação sobe a versão");
        std::fs::write(dir.join("ruim.json"), "{ nao").unwrap();
        let (lib, erros) = carregar_pasta(&dir);
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(erros.len(), 1);
        let nova = mais_nova(&doc, &lib).unwrap();
        assert_eq!(nova.versao, v2);
        assert!((nova.espessura - r.espessura).abs() < 1e-12);
        assert!(mais_nova(nova, &lib).is_none());
    }
}
