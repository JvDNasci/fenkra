// SPDX-License-Identifier: LGPL-2.1-or-later
//! O que a tela mostra na hora da captura, em texto (B431). A imagem sozinha não diz se o painel
//! pedido abriu: o programa grava `<captura>.png.estado.json` junto e a validação confere o que
//! esperava contra o que ficou aberto.

use crate::app::App;
use serde_json::{json, Value};

impl App {
    pub fn estado_demo(&self) -> Value {
        let doc = &self.open.doc;
        let comando = self.session.as_ref().and_then(|s| doc.features.iter().find(|f| f.id == s.id)).map(|f| f.name.clone());
        let tela_inicial = !self.started;
        json!({
            "bancada": format!("{:?}", self.settings.bench),
            "tela_inicial": tela_inicial,
            "peca": doc.name,
            "arquivo": self.open.file.as_ref().map(|p| p.display().to_string()),
            "comando_aberto": comando,
            "painel_de_comando": self.session.is_some(),
            "sketch_em_edicao": self.sketch.is_some(),
            "montagem_aberta": self.asm.is_some(),
            "desenho_aberto": self.dwg.is_some(),
            "comandos": doc.features.len(),
            // painéis da montagem que só o mouse abre
            "biblioteca_aberta": self.asm.as_ref().is_some_and(|a| a.catalogo.is_some()),
            "simulacao_aberta": self.asm.as_ref().is_some_and(|a| a.sim_open),
            "transmissao_aberta": self.asm.as_ref().is_some_and(|a| a.trans_open.is_some()),
        })
    }
}
