// SPDX-License-Identifier: LGPL-2.1-or-later
//! Exportar STEP e IGES (B104). Painel aberto só durante o comando: escolhe formato e opções e depois o
//! arquivo. A gravação roda no programa: o modelo é nosso, não um arquivo de fora que possa derrubar o
//! kernel. O STL segue pelo menu, que não tem opção além do preset.

use super::*;
use fenkra_conversor::{EsquemaSaida, Estrutura, Formato, ModeloNeutro, ModoIges, OpcoesExportar, Registro, UnidadeSaida};

const LARGURA: f32 = 300.;

fn nome_esquema(e: EsquemaSaida) -> &'static str {
    match e {
        EsquemaSaida::Ap203 => "AP203",
        EsquemaSaida::Ap214 => "AP214",
        EsquemaSaida::Ap242 => "AP242",
    }
}

fn nome_unidade(u: UnidadeSaida) -> &'static str {
    match u {
        UnidadeSaida::Milimetro => "Milímetro",
        UnidadeSaida::Polegada => "Polegada",
        UnidadeSaida::Metro => "Metro",
    }
}

fn dica_modo(m: ModoIges) -> &'static str {
    match m {
        ModoIges::Brep => "Leva a topologia completa: o sólido chega sólido em quem lê esse modo.",
        ModoIges::Faces => "Leva faces aparadas soltas: o que todo programa antigo lê. Chega como superfícies, para costurar.",
    }
}

fn dica_estrutura(e: Estrutura) -> &'static str {
    match e {
        Estrutura::Arvore => "Grava produtos e instâncias, como na árvore. A peça repetida vai uma vez só.",
        Estrutura::Achatada => "Grava um produto só, com todas as peças no lugar. Serve a quem lê o arquivo sem árvore.",
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Acao {
    Nada,
    Gravar,
    Cancelar,
}

/// Painel aberto durante o comando.
pub struct Dialogo {
    formato: Formato,
    opcoes: OpcoesExportar,
    /// exporta a montagem aberta, não a peça
    montagem: bool,
}

impl Dialogo {
    fn novo(formato: Formato, opcoes: OpcoesExportar, montagem: bool) -> Dialogo {
        Dialogo { formato, opcoes, montagem }
    }

    fn ui(&mut self, ui: &mut egui::Ui) -> Acao {
        ui.label(tr("Formato"));
        egui::ComboBox::from_id_salt("exportar_formato").selected_text(self.formato.nome()).width(LARGURA - 40.).show_ui(ui, |ui| {
            for f in [Formato::Step, Formato::Iges] {
                ui.selectable_value(&mut self.formato, f, f.nome());
            }
        }).response.on_hover_text(tr("STEP leva a árvore de produtos e é o padrão atual. IGES serve a quem só lê superfícies."));
        let o = &mut self.opcoes;
        if self.formato == Formato::Step {
            ui.label(tr("Esquema"));
            egui::ComboBox::from_id_salt("exportar_esquema").selected_text(nome_esquema(o.esquema)).width(LARGURA - 40.).show_ui(ui, |ui| {
                for e in [EsquemaSaida::Ap203, EsquemaSaida::Ap214, EsquemaSaida::Ap242] {
                    ui.selectable_value(&mut o.esquema, e, nome_esquema(e));
                }
            }).response.on_hover_text(tr("AP214 leva cores e camadas; AP203 não; AP242 é o mais novo."));
        } else {
            ui.label(tr("Modo do IGES"));
            egui::ComboBox::from_id_salt("exportar_modo_iges").selected_text(tr(o.iges.nome())).width(LARGURA - 40.).show_ui(ui, |ui| {
                for m in ModoIges::TODOS {
                    ui.selectable_value(&mut o.iges, m, tr(m.nome())).on_hover_text(tr(dica_modo(m)));
                }
            }).response.on_hover_text(tr(dica_modo(o.iges)));
        }
        if self.montagem {
            ui.label(tr("Estrutura"));
            egui::ComboBox::from_id_salt("exportar_estrutura").selected_text(tr(o.estrutura.nome())).width(LARGURA - 40.).show_ui(ui, |ui| {
                for e in Estrutura::TODAS {
                    ui.selectable_value(&mut o.estrutura, e, tr(e.nome())).on_hover_text(tr(dica_estrutura(e)));
                }
            }).response.on_hover_text(tr(dica_estrutura(o.estrutura)));
        }
        ui.label(tr("Unidade do arquivo"));
        egui::ComboBox::from_id_salt("exportar_unidade").selected_text(tr(nome_unidade(o.unidade))).width(LARGURA - 40.).show_ui(ui, |ui| {
            for u in [UnidadeSaida::Milimetro, UnidadeSaida::Polegada, UnidadeSaida::Metro] {
                ui.selectable_value(&mut o.unidade, u, tr(nome_unidade(u)));
            }
        }).response.on_hover_text(tr("Em que unidade os números vão escritos. O tamanho da peça não muda."));
        ui.horizontal_wrapped(|ui| {
            ui.checkbox(&mut o.cores, tr("Cores")).on_hover_text(tr("Grava a cor da peça."));
            ui.checkbox(&mut o.nomes, tr("Nomes")).on_hover_text(tr("Grava os nomes das peças."));
            ui.checkbox(&mut o.camadas, tr("Camadas")).on_hover_text(tr("Grava as camadas que a peça tiver."));
        });
        ui.add_space(6.);
        let mut acao = Acao::Nada;
        ui.horizontal(|ui| {
            if ui.button(tr("Exportar…")).on_hover_text(tr("Escolhe o arquivo e grava.")).clicked() {
                acao = Acao::Gravar;
            }
            if ui.button(tr("Cancelar")).clicked() {
                acao = Acao::Cancelar;
            }
        });
        acao
    }
}

impl App {
    pub(super) fn abrir_painel_exportar(&mut self, formato: Formato) {
        self.exportar_dialogo = Some(Dialogo::novo(formato, self.settings.opcoes_exportar, self.in_asm()));
    }

    pub(super) fn painel_exportar(&mut self, ctx: &egui::Context, area: Rect) {
        use crate::paineis::{self, Encaixe, Painel};
        let Some(mut d) = self.exportar_dialogo.take() else { return };
        let padrao = paineis::abaixo_do_cubo(area, LARGURA);
        let cfg = Painel { nome: "exportar".into(), titulo: tr("Exportar arquivo").to_string(), padrao, largura: LARGURA, redimensiona: false };
        let posicao = self.settings.painel("exportar", Encaixe::Direita);
        let saida = paineis::mostrar(ctx, area, &cfg, posicao, |ui| d.ui(ui));
        if let Some(p) = saida.movido {
            self.settings.guardar_painel("exportar", p);
        }
        match saida.conteudo.unwrap_or(Acao::Nada) {
            Acao::Nada => self.exportar_dialogo = Some(d),
            Acao::Cancelar => {}
            Acao::Gravar => self.gravar_exportacao(d),
        }
    }

    fn gravar_exportacao(&mut self, d: Dialogo) {
        self.settings.opcoes_exportar = d.opcoes;
        let montagem = self.asm.as_ref().filter(|_| d.montagem);
        // as peças são lidas dos arquivos da montagem: sem pasta, os caminhos não levam a lugar nenhum
        let pasta = montagem.and_then(|s| s.file.as_ref()).and_then(|p| p.parent()).map(|p| p.to_path_buf());
        if d.montagem && pasta.is_none() {
            return self.toast(tr("Salve a montagem antes de exportar: as peças são lidas dos arquivos dela.").to_string(), true);
        }
        let nome = montagem.map(|s| s.asm.name.clone()).unwrap_or_else(|| self.open.doc.name.clone());
        let extensao = d.formato.extensoes()[0];
        let Some(caminho) = rfd::FileDialog::new().add_filter(d.formato.nome(), d.formato.extensoes()).set_file_name(format!("{nome}.{extensao}")).save_file() else { return };
        let modelo = match (self.asm.as_ref().filter(|_| d.montagem), &pasta) {
            (Some(s), Some(pasta)) => ModeloNeutro::da_montagem(&s.asm, pasta),
            _ => ModeloNeutro::de_documento(&self.open.doc),
        };
        let r = modelo.and_then(|m| Registro::default().gravar(&m, &caminho, &d.opcoes));
        match r {
            Ok(()) => self.toast(format!("Exportado: {}", caminho.display()), false),
            Err(e) => self.toast(format!("Falha ao exportar: {e}"), true),
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn textos(d: &mut Dialogo) -> Vec<String> {
        let ctx = egui::Context::default();
        let mut textos = vec![];
        for _ in 0..2 {
            let entrada = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(400., 800.))), ..Default::default() };
            let mut saida = ctx.run_ui(entrada, |ui| {
                d.ui(ui);
            });
            saida.textures_delta.clear();
            textos = saida.shapes.iter().filter_map(|s| match &s.shape {
                egui::epaint::Shape::Text(t) => Some(t.galley.text().to_string()),
                _ => None,
            }).collect();
        }
        textos
    }

    #[test]
    fn modo_do_iges_so_aparece_no_iges_e_esquema_so_no_step() {
        let mut d = Dialogo::novo(Formato::Iges, OpcoesExportar::default(), false);
        let t = textos(&mut d).join("\n");
        assert!(t.contains("Modo do IGES") && t.contains("BRep") && !t.contains("Esquema"), "{t}");
        d.formato = Formato::Step;
        let t = textos(&mut d).join("\n");
        assert!(t.contains("Esquema") && t.contains("AP214") && !t.contains("Modo do IGES"), "{t}");
    }

    #[test]
    fn estrutura_so_aparece_na_montagem() {
        let mut d = Dialogo::novo(Formato::Step, OpcoesExportar::default(), false);
        assert!(!textos(&mut d).join("\n").contains("Estrutura"));
        d.montagem = true;
        let t = textos(&mut d).join("\n");
        assert!(t.contains("Estrutura") && t.contains("Com estrutura"), "{t}");
        for e in Estrutura::TODAS {
            assert!(crate::i18n::tem_traducao(e.nome()) && crate::i18n::tem_traducao(dica_estrutura(e)));
        }
    }

    #[test]
    fn textos_do_painel_tem_traducao() {
        for s in [
            "Formato",
            "Esquema",
            "Modo do IGES",
            "Exportar arquivo",
            "Exportar…",
            "Escolhe o arquivo e grava.",
            "Grava a cor da peça.",
            "Grava os nomes das peças.",
            "Grava as camadas que a peça tiver.",
            "STEP leva a árvore de produtos e é o padrão atual. IGES serve a quem só lê superfícies.",
            "AP214 leva cores e camadas; AP203 não; AP242 é o mais novo.",
            "Em que unidade os números vão escritos. O tamanho da peça não muda.",
            dica_modo(ModoIges::Brep),
            dica_modo(ModoIges::Faces),
        ] {
            assert!(crate::i18n::tem_traducao(s), "sem tradução: {s}");
        }
    }
}
