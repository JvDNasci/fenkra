// SPDX-License-Identifier: LGPL-2.1-or-later
//! Parâmetros com unidades, tipos, limites e supressão por expressão (B101).
//!
//! O valor interno de todo parâmetro de comprimento é em milímetros e o de ângulo em graus. A
//! unidade de exibição ([`UnidadeComp`]) e os sufixos digitados (`2 in`, `10cm`) só convertem o
//! que a pessoa vê ou escreve: nenhuma delas muda a geometria de um documento existente.
//!
//! Tipos:
//! - `Numero`: expressão numérica (o caso de sempre);
//! - `Booleano`: expressão avaliada como verdadeiro (diferente de zero) ou falso; entra nas outras
//!   expressões como 1 ou 0 e aceita `true`/`false` (por exemplo para `suppress_if`);
//! - `Lista`: o valor é uma das `opcoes` (cada uma uma expressão), guardada em `expr`;
//! - `Texto`: texto livre (código, observação). Não entra em contas.
//!
//! Limites e checagens nunca impedem o recálculo: viram [`AvisoParam`], mostrados na árvore.

use crate::doc::{Document, Feature, Id, Parameter, Unit};
use crate::expr::{self, DimUnidade};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum ParamTipo {
    #[default]
    Numero,
    Texto,
    Booleano,
    Lista,
}

impl ParamTipo {
    pub const TODOS: [ParamTipo; 4] = [ParamTipo::Numero, ParamTipo::Texto, ParamTipo::Booleano, ParamTipo::Lista];
    pub fn rotulo(self) -> &'static str {
        match self {
            ParamTipo::Numero => "Número",
            ParamTipo::Texto => "Texto",
            ParamTipo::Booleano => "Booleano",
            ParamTipo::Lista => "Lista",
        }
    }
    /// O parâmetro entra nas contas (tem valor numérico)?
    pub fn numerico(self) -> bool {
        self != ParamTipo::Texto
    }
}

/// Unidade em que um parâmetro de comprimento é mostrado e editado.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum UnidadeComp {
    #[default]
    Mm,
    Cm,
    M,
    Pol,
}

impl UnidadeComp {
    pub const TODAS: [UnidadeComp; 4] = [UnidadeComp::Mm, UnidadeComp::Cm, UnidadeComp::M, UnidadeComp::Pol];
    /// Quantos milímetros vale uma unidade.
    pub fn mm(self) -> f64 {
        match self {
            UnidadeComp::Mm => 1.0,
            UnidadeComp::Cm => 10.0,
            UnidadeComp::M => 1000.0,
            UnidadeComp::Pol => 25.4,
        }
    }
    pub fn simbolo(self) -> &'static str {
        match self {
            UnidadeComp::Mm => "mm",
            UnidadeComp::Cm => "cm",
            UnidadeComp::M => "m",
            UnidadeComp::Pol => "pol",
        }
    }
    /// Valor interno (mm) para o mostrado nesta unidade.
    pub fn de_mm(self, mm: f64) -> f64 {
        mm / self.mm()
    }
    pub fn para_mm(self, v: f64) -> f64 {
        v * self.mm()
    }
}

/// O que há de errado (ou duvidoso) com um parâmetro; nunca interrompe o recálculo.
#[derive(Clone, PartialEq, Debug)]
pub enum AvisoTipo {
    AbaixoDoMinimo { valor: f64, limite: f64 },
    AcimaDoMaximo { valor: f64, limite: f64 },
    /// a expressão de `verificacao` deu falso
    VerificacaoFalhou,
    /// `min`, `max` ou `verificacao` não puderam ser avaliados
    ExpressaoInvalida { campo: &'static str, motivo: String },
    /// o valor de uma lista não é uma das opções
    ForaDaLista,
    /// lista sem opções
    ListaVazia,
    /// unidade de comprimento num parâmetro de ângulo, ou o contrário
    UnidadeIncompativel,
}

#[derive(Clone, PartialEq, Debug)]
pub struct AvisoParam {
    pub param: Id,
    pub nome: String,
    pub tipo: AvisoTipo,
}

impl AvisoParam {
    /// Molde em português (cada `{}` recebe um item de [`Self::args`]); a interface o traduz.
    pub fn molde(&self) -> &'static str {
        match self.tipo {
            AvisoTipo::AbaixoDoMinimo { .. } => "{} está abaixo do mínimo ({} < {})",
            AvisoTipo::AcimaDoMaximo { .. } => "{} está acima do máximo ({} > {})",
            AvisoTipo::VerificacaoFalhou => "A checagem de {} falhou",
            AvisoTipo::ExpressaoInvalida { .. } => "Expressão inválida em {}: {}",
            AvisoTipo::ForaDaLista => "{} não é uma das opções da lista",
            AvisoTipo::ListaVazia => "A lista {} não tem opções",
            AvisoTipo::UnidadeIncompativel => "{} usa uma unidade que não combina com o tipo dele",
        }
    }
    pub fn args(&self) -> Vec<String> {
        let n = self.nome.clone();
        match &self.tipo {
            AvisoTipo::AbaixoDoMinimo { valor, limite } | AvisoTipo::AcimaDoMaximo { valor, limite } => vec![n, fmt(*valor), fmt(*limite)],
            AvisoTipo::ExpressaoInvalida { motivo, .. } => vec![n, motivo.clone()],
            _ => vec![n],
        }
    }
}

/// Preenche os `{}` de um molde, em ordem.
pub fn preencher(molde: &str, args: &[String]) -> String {
    let mut out = String::new();
    let mut it = args.iter();
    let mut resto = molde;
    while let Some(i) = resto.find("{}") {
        out.push_str(&resto[..i]);
        if let Some(a) = it.next() {
            out.push_str(a)
        }
        resto = &resto[i + 2..];
    }
    out.push_str(resto);
    out
}

fn fmt(v: f64) -> String {
    let t = format!("{v:.4}");
    t.trim_end_matches('0').trim_end_matches('.').to_string()
}

impl Parameter {
    /// Valor mostrado ao editar, na unidade de exibição (só comprimento converte).
    pub fn valor_exibido(&self, mm: f64) -> f64 {
        if self.unit == Unit::Comprimento { self.exibir.de_mm(mm) } else { mm }
    }
    /// Texto da expressão que representa `v` digitado na unidade de exibição.
    pub fn expr_de_exibido(&self, v: f64) -> String {
        if self.unit == Unit::Comprimento && self.exibir != UnidadeComp::Mm {
            format!("{} {}", fmt(v), self.exibir.simbolo())
        } else {
            fmt(v)
        }
    }
}

impl Document {
    /// A supressão por expressão de `f` está valendo agora? Expressão que não avalia não suprime
    /// (o erro aparece como aviso do comando, nunca some com ele em silêncio).
    pub fn suprimido_por_expressao(&self, f: &Feature) -> bool {
        if f.suppress_if.trim().is_empty() {
            return false;
        }
        self.eval_params().ok().and_then(|v| expr::eval_str(&f.suppress_if, &v).ok()).is_some_and(|r| r != 0.0)
    }

    /// Suprimido de qualquer forma: marcado à mão ou por expressão.
    pub fn suprimido_efetivo(&self, f: &Feature) -> bool {
        f.suppressed || self.suprimido_por_expressao(f)
    }

    /// Erro de sintaxe ou de nome na expressão de supressão de um comando.
    pub fn erro_suppress_if(&self, f: &Feature) -> Option<String> {
        if f.suppress_if.trim().is_empty() {
            return None;
        }
        let vars = self.eval_params().ok()?;
        expr::eval_str(&f.suppress_if, &vars).err().map(|e| e.to_string())
    }

    /// Todos os avisos de limites, checagens, listas e unidades, na ordem dos parâmetros.
    pub fn avisos_param(&self) -> Vec<AvisoParam> {
        let Ok(vars) = self.eval_params() else { return vec![] };
        let mut out = vec![];
        for p in &self.params {
            let mut aviso = |tipo| out.push(AvisoParam { param: p.id, nome: p.name.clone(), tipo });
            if p.tipo == ParamTipo::Texto {
                continue;
            }
            let Some(&v) = vars.get(&p.name) else { continue };
            for (campo, src, abaixo) in [("min", &p.min, true), ("max", &p.max, false)] {
                if src.trim().is_empty() {
                    continue;
                }
                match expr::eval_str(src, &vars) {
                    Ok(l) if abaixo && v < l - 1e-9 => aviso(AvisoTipo::AbaixoDoMinimo { valor: v, limite: l }),
                    Ok(l) if !abaixo && v > l + 1e-9 => aviso(AvisoTipo::AcimaDoMaximo { valor: v, limite: l }),
                    Ok(_) => {}
                    Err(e) => aviso(AvisoTipo::ExpressaoInvalida { campo, motivo: e.to_string() }),
                }
            }
            if !p.verificacao.trim().is_empty() {
                match expr::eval_str(&p.verificacao, &vars) {
                    Ok(r) if r == 0.0 => aviso(AvisoTipo::VerificacaoFalhou),
                    Ok(_) => {}
                    Err(e) => aviso(AvisoTipo::ExpressaoInvalida { campo: "verificacao", motivo: e.to_string() }),
                }
            }
            if p.tipo == ParamTipo::Lista {
                if p.opcoes.is_empty() {
                    aviso(AvisoTipo::ListaVazia);
                } else if !p.opcoes.iter().any(|o| o.trim() == p.expr.trim() || expr::eval_str(o, &vars).ok() == Some(v)) {
                    aviso(AvisoTipo::ForaDaLista);
                }
            }
            let dims = expr::dimensoes(&p.expr);
            let ruim = match p.unit {
                Unit::Angulo => dims.contains(&DimUnidade::Comprimento),
                Unit::Comprimento => dims.contains(&DimUnidade::Angulo),
                Unit::Numero => !dims.is_empty(),
            };
            if ruim {
                aviso(AvisoTipo::UnidadeIncompativel);
            }
        }
        out
    }

    /// Avisos de um parâmetro só.
    pub fn avisos_do_param(&self, id: Id) -> Vec<AvisoParam> {
        self.avisos_param().into_iter().filter(|a| a.param == id).collect()
    }

    /// Escolhe uma opção da lista (e só dela).
    pub fn escolher_opcao(&mut self, id: Id, opcao: usize) -> bool {
        let Some(p) = self.params.iter_mut().find(|p| p.id == id) else { return false };
        match p.opcoes.get(opcao) {
            Some(o) if p.tipo == ParamTipo::Lista => {
                p.expr = o.clone();
                true
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc() -> Document {
        Document::new("t")
    }

    #[test]
    fn unidade_de_exibicao_nao_muda_o_valor_interno() {
        let mut d = doc();
        let a = d.add_param("L", "50.8", Unit::Comprimento).unwrap();
        let antes = d.eval_params().unwrap()["L"];
        d.params.iter_mut().find(|p| p.id == a).unwrap().exibir = UnidadeComp::Pol;
        assert_eq!(d.eval_params().unwrap()["L"], antes);
        let p = d.params.iter().find(|p| p.id == a).unwrap();
        assert!((p.valor_exibido(antes) - 2.0).abs() < 1e-12);
        assert_eq!(p.expr_de_exibido(2.0), "2 pol");
    }

    #[test]
    fn sufixo_de_unidade_em_parametro() {
        let mut d = doc();
        d.add_param("A", "2 in", Unit::Comprimento).unwrap();
        d.add_param("B", "A + 1cm", Unit::Comprimento).unwrap();
        let v = d.eval_params().unwrap();
        assert!((v["A"] - 50.8).abs() < 1e-12);
        assert!((v["B"] - 60.8).abs() < 1e-12);
        assert!(d.avisos_param().is_empty());
    }

    #[test]
    fn unidade_incompativel_avisa() {
        let mut d = doc();
        d.add_param("Ang", "5 mm", Unit::Angulo).unwrap();
        d.add_param("N", "3 in", Unit::Numero).unwrap();
        let av = d.avisos_param();
        assert_eq!(av.len(), 2);
        assert!(av.iter().all(|a| a.tipo == AvisoTipo::UnidadeIncompativel));
    }

    #[test]
    fn limites_e_checagem_viram_aviso_sem_travar_o_calculo() {
        let mut d = doc();
        let w = d.add_param("W", "3", Unit::Comprimento).unwrap();
        d.add_param("T", "2", Unit::Comprimento).unwrap();
        let p = d.params.iter_mut().find(|p| p.id == w).unwrap();
        p.min = "5".into();
        p.max = "10 mm".into();
        p.verificacao = "W > T*2".into();
        let av = d.avisos_param();
        assert_eq!(av.len(), 2, "{av:?}");
        assert_eq!(av[0].tipo, AvisoTipo::AbaixoDoMinimo { valor: 3.0, limite: 5.0 });
        assert_eq!(av[1].tipo, AvisoTipo::VerificacaoFalhou);
        assert_eq!(d.eval_params().unwrap()["W"], 3.0, "o valor segue valendo");
        d.params.iter_mut().find(|p| p.id == w).unwrap().expr = "7".into();
        assert!(d.avisos_param().is_empty());
        d.params.iter_mut().find(|p| p.id == w).unwrap().expr = "12".into();
        assert!(matches!(d.avisos_param()[0].tipo, AvisoTipo::AcimaDoMaximo { .. }));
    }

    #[test]
    fn limite_com_expressao_invalida_avisa() {
        let mut d = doc();
        let w = d.add_param("W", "3", Unit::Comprimento).unwrap();
        d.params[0].min = "naoexiste".into();
        let av = d.avisos_do_param(w);
        assert!(matches!(av[0].tipo, AvisoTipo::ExpressaoInvalida { campo: "min", .. }));
    }

    #[test]
    fn booleano_e_lista_entram_nas_contas_e_texto_nao() {
        let mut d = doc();
        let b = d.add_param("Chanfro", "true", Unit::Numero).unwrap();
        d.params.iter_mut().find(|p| p.id == b).unwrap().tipo = ParamTipo::Booleano;
        let l = d.add_param("Diam", "8", Unit::Comprimento).unwrap();
        {
            let p = d.params.iter_mut().find(|p| p.id == l).unwrap();
            p.tipo = ParamTipo::Lista;
            p.opcoes = vec!["6".into(), "8".into(), "10".into()];
        }
        let t = d.add_param("Codigo", "AB-12 qualquer coisa", Unit::Numero).unwrap();
        d.params.iter_mut().find(|p| p.id == t).unwrap().tipo = ParamTipo::Texto;
        d.add_param("Dobro", "Diam*2 + Chanfro", Unit::Comprimento).unwrap();
        let v = d.eval_params().unwrap();
        assert_eq!(v["Chanfro"], 1.0);
        assert_eq!(v["Dobro"], 17.0);
        assert!(!v.contains_key("Codigo"), "texto não entra em contas");
        assert!(d.avisos_param().is_empty());
        assert!(d.escolher_opcao(l, 2));
        assert_eq!(d.eval_params().unwrap()["Diam"], 10.0);
        assert!(!d.escolher_opcao(l, 9));
        assert!(!d.escolher_opcao(b, 0), "só lista escolhe opção");
        d.params.iter_mut().find(|p| p.id == l).unwrap().expr = "7".into();
        assert_eq!(d.avisos_do_param(l)[0].tipo, AvisoTipo::ForaDaLista);
    }

    #[test]
    fn booleano_normaliza_para_zero_ou_um() {
        let mut d = doc();
        let b = d.add_param("Ligado", "7", Unit::Numero).unwrap();
        d.params.iter_mut().find(|p| p.id == b).unwrap().tipo = ParamTipo::Booleano;
        assert_eq!(d.eval_params().unwrap()["Ligado"], 1.0, "qualquer valor diferente de zero é verdadeiro");
        d.params[0].expr = "0".into();
        assert_eq!(d.eval_params().unwrap()["Ligado"], 0.0);
    }

    #[test]
    fn json_antigo_abre_com_padroes_e_o_novo_grava_tipos() {
        let mut d = doc();
        let l = d.add_param("Diam", "8", Unit::Comprimento).unwrap();
        {
            let p = d.params.iter_mut().find(|p| p.id == l).unwrap();
            p.tipo = ParamTipo::Lista;
            p.opcoes = vec!["6".into(), "8".into()];
            p.exibir = UnidadeComp::Cm;
        }
        let v2 = Document::from_json(&d.to_json()).unwrap();
        assert_eq!(v2, d);
        assert_eq!(v2.format_version, crate::formato::Tipo::Peca.versao_atual());
        let mut j: serde_json::Value = serde_json::from_str(&d.to_json()).unwrap();
        j["format_version"] = 1.into();
        for k in ["tipo", "exibir", "opcoes"] {
            j["params"][0].as_object_mut().unwrap().remove(k);
        }
        let v1 = Document::from_json(&j.to_string()).unwrap();
        assert_eq!(v1.format_version, crate::formato::Tipo::Peca.versao_atual(), "a cadeia leva o arquivo à versão atual");
        assert_eq!(v1.params[0].tipo, ParamTipo::Numero);
        assert_eq!(v1.params[0].exibir, UnidadeComp::Mm);
        assert!(!v1.formato_futuro());
    }

    #[test]
    fn supressao_por_expressao_segue_o_parametro() {
        use crate::doc::{FeatureKind, PlaneRef, Sketch};
        let mut d = doc();
        d.add_param("N", "1", Unit::Numero).unwrap();
        let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::default(), data: Default::default() }));
        let idx = d.features.iter().position(|f| f.id == sk).unwrap();
        assert!(d.is_active(idx));
        d.feature_mut(sk).unwrap().suppress_if = "N < 2".into();
        assert!(!d.is_active(idx), "N=1 satisfaz a expressão e suprime");
        assert!(d.suprimido_efetivo(&d.features[idx]));
        assert!(!d.features[idx].suppressed, "a marca manual não muda");
        d.params[0].expr = "3".into();
        assert!(d.is_active(idx), "a expressão deixou de valer e o comando volta");
        d.feature_mut(sk).unwrap().suppress_if = "naoexiste > 1".into();
        assert!(d.is_active(idx), "expressão inválida não suprime");
        assert!(d.erro_suppress_if(&d.features[idx]).is_some());
        d.feature_mut(sk).unwrap().suppress_if = String::new();
        d.feature_mut(sk).unwrap().suppressed = true;
        assert!(!d.is_active(idx), "a supressão manual segue valendo");
    }

    #[test]
    fn preencher_molde() {
        assert_eq!(preencher("{} < {}", &["a".into(), "b".into()]), "a < b");
        assert_eq!(preencher("x", &[]), "x");
    }
}
