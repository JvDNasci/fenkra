// SPDX-License-Identifier: LGPL-2.1-or-later
//! Estilos de chapa metálica (B283), em cima de [`crate::regras`].
//!
//! `SheetParams` guarda os valores próprios da peça; quando há um estilo escolhido, só as chaves em
//! `overrides` valem como estão e as demais vêm do estilo. Todo leitor (recompute, tabela de
//! dobras, janela) trabalha com [`SheetParams::efetivo`], a cópia já resolvida, então trocar o
//! estilo recalcula a peça e a tabela sem mexer em nenhum comando.

use crate::doc::{Document, SheetParams};
use crate::regras::{self, EstiloRegras, Origem};
use std::collections::HashMap;

/// Chaves de um estilo de chapa.
pub const CHAVES: [&str; 9] = ["material", "espessura", "raio", "k", "metodo", "tolerancia", "alivio", "alivio_forma", "canto"];

/// Nome da variável de contexto das fórmulas da chapa (`raio = 1.15*espessura`).
pub const VAR_ESPESSURA: &str = "espessura";

impl SheetParams {
    /// Texto atual do campo (o valor próprio, sem olhar o estilo).
    pub fn campo(&self, chave: &str) -> String {
        match chave {
            "material" => self.material.clone(),
            "espessura" => self.thickness.clone(),
            "raio" => self.radius.clone(),
            "k" => self.k.clone(),
            "metodo" => self.method.to_string(),
            "tolerancia" => self.allowance.clone(),
            "alivio" => self.relief.clone(),
            "alivio_forma" => self.relief_kind.to_string(),
            "canto" => self.corner.clone(),
            _ => String::new(),
        }
    }

    fn grava(&mut self, chave: &str, v: &str) {
        match chave {
            "material" => self.material = v.to_string(),
            "espessura" => self.thickness = v.to_string(),
            "raio" => self.radius = v.to_string(),
            "k" => self.k = v.to_string(),
            "metodo" => self.method = v.trim().parse().unwrap_or(0),
            "tolerancia" => self.allowance = v.to_string(),
            "alivio" => self.relief = v.to_string(),
            "alivio_forma" => self.relief_kind = v.trim().parse().unwrap_or(0),
            "canto" => self.corner = v.to_string(),
            _ => {}
        }
    }

    /// Cópia com o estilo aplicado: cada chave não sobrescrita e definida no estilo troca o valor
    /// próprio pelo do estilo. Sem estilo (ou nome que não existe mais), é a própria peça.
    pub fn efetivo(&self, estilos: &[EstiloRegras]) -> SheetParams {
        let mut e = self.clone();
        let Some(est) = regras::acha(estilos, &self.style) else { return e };
        for c in CHAVES {
            let (f, o) = regras::resolve(Some(est), &self.overrides, c, &self.campo(c));
            if o == Origem::Estilo {
                e.grava(c, &f);
            }
        }
        e
    }

    /// De onde vem o valor de um campo (para o cadeado e a dica da janela).
    pub fn origem(&self, estilos: &[EstiloRegras], chave: &str) -> Origem {
        regras::resolve(regras::acha(estilos, &self.style), &self.overrides, chave, &self.campo(chave)).1
    }

    /// Grava na peça o que a janela mudou: `antes` e `depois` são cópias efetivas (de `efetivo`).
    /// Com estilo ativo, um campo alterado vira sobrescrita (cadeado); sem estilo, só grava.
    pub fn aplica_edicao(&mut self, antes: &SheetParams, depois: &SheetParams, estilos: &[EstiloRegras]) {
        let com_estilo = regras::acha(estilos, &self.style).is_some();
        for c in CHAVES {
            let novo = depois.campo(c);
            if novo != antes.campo(c) {
                self.grava(c, &novo);
                if com_estilo {
                    regras::marca(&mut self.overrides, c, true);
                }
            }
        }
        self.flat = depois.flat;
    }

    /// Devolve o campo ao estilo (tira o cadeado). O valor próprio fica guardado, sem efeito.
    pub fn restaura(&mut self, chave: &str) {
        regras::marca(&mut self.overrides, chave, false);
    }

    /// Escolhe o estilo; as sobrescritas antigas deixam de valer (o estilo novo vale inteiro).
    pub fn escolhe_estilo(&mut self, nome: &str) {
        self.style = nome.to_string();
        self.overrides.clear();
    }

    /// Estilo novo com os valores efetivos desta peça (o "salvar como estilo" da janela).
    pub fn como_estilo(&self, nome: &str, estilos: &[EstiloRegras]) -> EstiloRegras {
        let e = self.efetivo(estilos);
        let mut est = EstiloRegras::novo(nome);
        for c in CHAVES {
            let v = e.campo(c);
            // chaves vazias não entram: o estilo só define o que a peça definiu
            if !v.trim().is_empty() {
                est.campos.insert(c.to_string(), v);
            }
        }
        est
    }

    /// Variáveis da peça mais `espessura`, para avaliar raio e alívios como fórmula da espessura.
    /// Espessura que não calcula devolve as variáveis sem ela (o erro aparece ao avaliar o campo).
    pub fn vars_com_espessura(&self, vars: &HashMap<String, f64>) -> HashMap<String, f64> {
        let mut v = vars.clone();
        if let Ok(t) = crate::expr::eval_str(&self.thickness, vars) {
            v.insert(VAR_ESPESSURA.to_string(), t);
        }
        v
    }
}

impl Document {
    /// Parâmetros de chapa com o estilo aplicado (leitor único para recompute e tabela).
    pub fn sheet_efetivo(&self) -> Option<SheetParams> {
        self.sheet.as_ref().map(|s| s.efetivo(&self.sheet_styles))
    }

    /// Salva (ou substitui, pelo nome) um estilo com os valores efetivos da chapa e o escolhe.
    /// Devolve `false` se não há chapa ou o nome é vazio.
    pub fn salva_estilo_chapa(&mut self, nome: &str) -> bool {
        let nome = nome.trim();
        let Some(s) = &self.sheet else { return false };
        if nome.is_empty() {
            return false;
        }
        let est = s.como_estilo(nome, &self.sheet_styles);
        match self.sheet_styles.iter_mut().find(|e| e.nome == nome) {
            Some(x) => *x = est,
            None => self.sheet_styles.push(est),
        }
        if let Some(s) = self.sheet.as_mut() {
            s.escolhe_estilo(nome);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn estilos() -> Vec<EstiloRegras> {
        vec![
            EstiloRegras::novo("Inox fino").com("espessura", "1.5").com("raio", "1.15*espessura").com("k", "0.45").com("alivio", "2").com("canto", "3"),
            EstiloRegras::novo("Aço grosso").com("espessura", "3").com("raio", "2*espessura").com("metodo", "2").com("tolerancia", "4.5"),
        ]
    }

    #[test]
    fn herda_do_estilo_e_marca_sobrescrita() {
        let est = estilos();
        let mut s = SheetParams { style: "Inox fino".into(), ..Default::default() };
        let e = s.efetivo(&est);
        assert_eq!((e.thickness.as_str(), e.radius.as_str(), e.k.as_str(), e.relief.as_str(), e.corner.as_str()), ("1.5", "1.15*espessura", "0.45", "2", "3"));
        assert_eq!(s.origem(&est, "raio"), Origem::Estilo);
        // o estilo não define "metodo": sem cadeado, vale o da peça
        assert_eq!(s.origem(&est, "metodo"), Origem::Padrao);

        // editar o raio na janela vira sobrescrita
        let antes = s.efetivo(&est);
        let mut depois = antes.clone();
        depois.radius = "4".into();
        s.aplica_edicao(&antes, &depois, &est);
        assert_eq!(s.overrides, vec!["raio".to_string()]);
        assert_eq!(s.origem(&est, "raio"), Origem::Sobrescrita);
        let e = s.efetivo(&est);
        assert_eq!((e.radius.as_str(), e.thickness.as_str()), ("4", "1.5"));

        // soltar o cadeado devolve o raio ao estilo
        s.restaura("raio");
        assert_eq!(s.efetivo(&est).radius, "1.15*espessura");
    }

    #[test]
    fn trocar_de_estilo_zera_sobrescritas_e_serializa() {
        let est = estilos();
        let mut s = SheetParams { style: "Inox fino".into(), overrides: vec!["raio".into()], ..Default::default() };
        s.escolhe_estilo("Aço grosso");
        assert!(s.overrides.is_empty());
        let e = s.efetivo(&est);
        assert_eq!((e.thickness.as_str(), e.method, e.allowance.as_str()), ("3", 2, "4.5"));
        // salvar e abrir preserva o estilo e as sobrescritas
        s.overrides.push("k".into());
        let json = serde_json::to_string(&s).unwrap();
        let volta: SheetParams = serde_json::from_str(&json).unwrap();
        assert_eq!(volta, s);
        // documento antigo, sem os campos novos, ainda abre
        let antigo: SheetParams = serde_json::from_str(r#"{"thickness":"2","radius":"2","k":"0.44"}"#).unwrap();
        assert!(antigo.style.is_empty() && antigo.overrides.is_empty());
    }

    #[test]
    fn salvar_como_estilo_guarda_os_efetivos_e_escolhe() {
        let mut d = Document::new("x");
        d.sheet = Some(SheetParams { thickness: "2".into(), radius: "1.5*espessura".into(), ..Default::default() });
        assert!(d.salva_estilo_chapa("Meu estilo"));
        assert_eq!(d.sheet_styles[0].campo("raio"), Some("1.5*espessura"));
        assert_eq!(d.sheet.as_ref().unwrap().style, "Meu estilo");
        // sem nome ou sem chapa, nada acontece
        assert!(!d.salva_estilo_chapa("  "));
    }

    #[test]
    fn variavel_espessura_nas_formulas() {
        let s = SheetParams { thickness: "2.5".into(), ..Default::default() };
        let v = s.vars_com_espessura(&HashMap::new());
        assert!((crate::expr::eval_str("1.2*espessura", &v).unwrap() - 3.0).abs() < 1e-12);
    }
}
