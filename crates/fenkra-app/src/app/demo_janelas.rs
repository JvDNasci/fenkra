// SPDX-License-Identifier: LGPL-2.1-or-later
//! Desenvolvimento: `FENKRA_DEMO_JANELA=<nome>` abre, no segundo quadro, uma janela ou painel que só o
//! mouse abriria, para a captura (`FENKRA_DEMO_FOTO`). Nomes: `salvar_como`, `empacotar`, `reparar`,
//! `liberar` (combine com `FENKRA_DEMO_PROJETO=1`), `resumo` (com `FENKRA_DEMO_EST=1`), `roda`, `roda_editor`,
//! `ramos` (com `FENKRA_DEMO_PROJETO=1`: ramo em conflito e janela de mesclagem), `tabela` (parâmetros com a tabela de projeto preenchida).

use super::*;

impl App {
    pub(super) fn demo_janela(&mut self, ctx: &egui::Context) {
        static FEITO: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
        let Ok(nome) = std::env::var("FENKRA_DEMO_JANELA") else { return };
        self.demo_diagnostico_acao(ctx);
        if ctx.input(|i| i.time) < 2. || FEITO.swap(true, std::sync::atomic::Ordering::Relaxed) {
            return;
        }
        if matches!(nome.as_str(), "salvar_como" | "empacotar" | "reparar") && self.open.file.is_none() {
            self.demo_arquivo();
        }
        if nome == "ramos" {
            self.demo_ramos();
        }
        if nome == "liberar" {
            self.demo_conjunto_que_usa();
        }
        match nome.as_str() {
            "salvar_como" => self.abrir_salvar_como(ctx),
            "empacotar" => self.abrir_empacotar(ctx),
            "reparar" => self.abrir_reparar_links(ctx),
            "liberar" => {
                if let Some(id) = self.project_sel {
                    self.release_dialog = Some((id, String::new()));
                }
            }
            "ramos" => self.demo_abrir_mescla(ctx),
            "resumo" => crate::st_montagem_ui::abrir(self),
            "roda" => {
                let r = ctx.content_rect();
                self.roda.fase = crate::roda::Fase::Fixa { p0: [r.center().x, r.center().y] };
            }
            // anotação selecionada: `pmi_barra` abre a mini barra da cota; `pmi_fcf` o painel da tolerância geométrica
            "pmi_barra" | "pmi_fcf" => {
                use fenkra_core::pmi::PmiKind;
                let barra = nome == "pmi_barra";
                let alvo = self.open.doc.pmi.iter().find(|p| if barra { matches!(p.kind, PmiKind::Linear) } else { matches!(p.kind, PmiKind::Gdt { .. }) }).map(|p| p.id);
                if let Some(id) = alvo {
                    self.sel = Sel::Pmi(id);
                    if barra {
                        crate::pmi_minibar::open_for(self, id);
                    }
                }
            }
            "daltonico" => self.settings.acessibilidade.paleta_segura = true,
            "diagnostico" => self.demo_diagnostico(),
            "roda_editor" => crate::roda::ui::pedir_abrir(ctx),
            "tabela" => {
                let s = |x: &str| x.to_string();
                let d = &mut self.open.doc;
                let nome_p = d.params.first().map(|p| p.name.clone()).unwrap_or_default();
                d.tabela.colunas = vec![fenkra_core::configuracoes::Coluna::Parametro(nome_p), fenkra_core::configuracoes::Coluna::Pn];
                d.tabela.linhas = vec![
                    fenkra_core::configuracoes::Linha { nome: s("Fina"), celulas: vec![s("8"), s("PN-001")] },
                    fenkra_core::configuracoes::Linha { nome: s("Grossa"), celulas: vec![s("20"), s("PN-002")] },
                ];
                d.tabela.ativa = s("Inexistente");
                self.sel = Sel::Params;
            }
            _ => {}
        }
    }

    /// Grava a peça aberta numa pasta temporária, com dois links (um existe, um sumiu), para os diálogos de arquivo.
    fn demo_arquivo(&mut self) {
        let pasta = std::env::temp_dir().join(format!("fenkra-demo-arquivo-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&pasta);
        let alvo = pasta.join("modelo.fkp");
        let _ = std::fs::write(pasta.join("parafuso.fkp"), self.open.doc.to_json());
        let Ok(mut v) = serde_json::from_str::<serde_json::Value>(&self.open.doc.to_json()) else { return };
        v["links_demo"] = serde_json::json!([
            {"uid": "demo-uid-1", "caminho": "parafuso.fkp"},
            {"uid": "demo-uid-2", "caminho": "suporte_antigo.fkp"}
        ]);
        if std::fs::write(&alvo, v.to_string()).is_ok() {
            self.open.file = Some(alvo);
        }
    }

    /// Cria, ao lado da peça do projeto, uma montagem que a usa: a lista "quem usa" da liberação ganha linhas.
    fn demo_conjunto_que_usa(&mut self) {
        let Some(p) = self.project.as_ref() else { return };
        let Some(id) = self.project_sel else { return };
        let Ok(e) = p.entry(id) else { return };
        let arquivo = p.root.join(&e.file);
        let j = serde_json::json!({"name": "Conjunto de exemplo", "nodes": [
            {"id": 7, "name": "Placa esquerda", "kind": {"Part": {"path": arquivo.display().to_string()}}},
            {"id": 8, "name": "Placa direita", "kind": {"Part": {"path": arquivo.display().to_string()}}}
        ]});
        let _ = std::fs::write(p.root.join("conjunto.fka"), j.to_string());
    }
}
