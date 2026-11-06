// SPDX-License-Identifier: LGPL-2.1-or-later
//! Exportar arquivo (B104, B210). Painel aberto só durante o comando, na ordem de um diálogo só: suporte
//! (o que o formato leva, lido da matriz do conversor), modo, parâmetros e avançado. Depois escolhe o
//! arquivo. A gravação roda no programa: o modelo é nosso, não um arquivo de fora que possa derrubar o
//! kernel. O STL com presets segue pelo menu.

use super::*;
use crate::ui_ext::dv;
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

/// Formatos que têm versão em texto e em binário.
fn tem_codificacao(f: Formato) -> bool {
    matches!(f, Formato::Stl | Formato::Ply)
}

/// Formatos cuja norma fixa metros: a unidade do painel não vale para eles.
fn unidade_fixa(f: Formato) -> bool {
    matches!(f, Formato::Gltf | Formato::Glb | Formato::Vrml | Formato::X3d)
}

/// Lista de nomes traduzidos, para as linhas de suporte.
fn lista(nomes: &[&'static str]) -> String {
    nomes.iter().map(|n| tr(n)).collect::<Vec<_>>().join(", ")
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Acao {
    Nada,
    Gravar,
    Cancelar,
}

/// Suporte, modo, parâmetros e avançado de um formato. O lote usa o mesmo painel em cada perfil.
pub(super) fn opcoes_ui(ui: &mut egui::Ui, formato: Formato, o: &mut OpcoesExportar, montagem: bool) {
        let suporte = Registro::default().capacidades(formato);
        ui.small(format!("{}: {}", tr("Leva"), lista(&suporte.leva()))).on_hover_text(tr("O que este formato carrega, conferido na matriz de formatos."));
        let falta = suporte.nao_leva();
        if !falta.is_empty() {
            ui.small(format!("{}: {}", tr("Não leva"), lista(&falta))).on_hover_text(tr("O que fica para trás ao gravar neste formato."));
        }
        ui.add_space(4.);
                if formato.e_malha() {
            if tem_codificacao(formato) {
                ui.checkbox(&mut o.malha.binario, tr("Binário")).on_hover_text(tr("Arquivo menor e sem perda de leitura. Desligado grava em texto, que se abre em qualquer editor."));
            }
        } else if formato == Formato::Step {
            ui.label(tr("Esquema"));
            egui::ComboBox::from_id_salt("exportar_esquema").selected_text(nome_esquema(o.esquema)).width(LARGURA - 40.).show_ui(ui, |ui| {
                for e in [EsquemaSaida::Ap203, EsquemaSaida::Ap214, EsquemaSaida::Ap242] {
                    ui.selectable_value(&mut o.esquema, e, nome_esquema(e));
                }
            }).response.on_hover_text(tr("AP214 leva cores e camadas; AP203 não; AP242 é o mais novo."));
            if o.esquema == EsquemaSaida::Ap242 {
                ui.checkbox(&mut o.pmi, tr("Anotações semânticas (PMI)")).on_hover_text(tr("Grava as cotas com tolerância, os quadros de tolerância geométrica, as referências e os alvos ligados às faces. Rugosidade, solda e notas ainda ficam de fora."));
            }
        } else if formato == Formato::Iges {
            ui.label(tr("Modo do IGES"));
            egui::ComboBox::from_id_salt("exportar_modo_iges").selected_text(tr(o.iges.nome())).width(LARGURA - 40.).show_ui(ui, |ui| {
                for m in ModoIges::TODOS {
                    ui.selectable_value(&mut o.iges, m, tr(m.nome())).on_hover_text(tr(dica_modo(m)));
                }
            }).response.on_hover_text(tr(dica_modo(o.iges)));
        }
        if montagem {
            ui.label(tr("Estrutura"));
            egui::ComboBox::from_id_salt("exportar_estrutura").selected_text(tr(o.estrutura.nome())).width(LARGURA - 40.).show_ui(ui, |ui| {
                for e in Estrutura::TODAS {
                    ui.selectable_value(&mut o.estrutura, e, tr(e.nome())).on_hover_text(tr(dica_estrutura(e)));
                }
            }).response.on_hover_text(tr(dica_estrutura(o.estrutura)));
        }
        if formato.e_malha() {
            ui.label(tr("Desvio da malha"));
            ui.add(dv(&mut o.malha.desvio).speed(0.005).range(0.001..=10.).suffix(" mm").dica(tr("Quanto a malha pode se afastar da superfície exata. Menor é mais fiel e mais pesado.")));
            ui.label(tr("Ângulo da malha"));
            ui.add(dv(&mut o.malha.angulo).speed(0.5).range(1.0..=60.).suffix(" °").dica(tr("Ângulo máximo entre triângulos vizinhos numa superfície curva. Menor deixa a curva mais lisa.")));
        }
        if unidade_fixa(formato) {
            ui.small(tr("Este formato é sempre gravado em metros, como a norma dele manda.")).on_hover_text(tr("O tamanho da peça não muda: quem abre o arquivo vê a medida certa."));
        } else if formato.e_2d() {
            ui.small(tr("Grava os esboços da peça em milímetros, uma camada por esboço.")).on_hover_text(tr("Linha de construção fica de fora. Cada esboço sai nas coordenadas dele."));
        } else {
            ui.label(tr("Unidade do arquivo"));
            egui::ComboBox::from_id_salt("exportar_unidade").selected_text(tr(nome_unidade(o.unidade))).width(LARGURA - 40.).show_ui(ui, |ui| {
                for u in [UnidadeSaida::Milimetro, UnidadeSaida::Polegada, UnidadeSaida::Metro] {
                    ui.selectable_value(&mut o.unidade, u, tr(nome_unidade(u)));
                }
            }).response.on_hover_text(tr("Em que unidade os números vão escritos. O tamanho da peça não muda."));
            if !suporte.unidade {
                ui.small(tr("O arquivo não diz a unidade: quem abrir precisa saber qual é.")).on_hover_text(tr("Este formato guarda só os números. Milímetro é o que a maioria dos programas supõe."));
            }
        }
        let malha = formato.e_malha();
        egui::CollapsingHeader::new(tr("Avançado")).id_salt("exportar_avancado").default_open(!malha).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.add_enabled(suporte.cor, egui::Checkbox::new(&mut o.cores, tr("Cores"))).on_hover_text(tr("Grava a cor da peça."));
                ui.checkbox(&mut o.nomes, tr("Nomes")).on_hover_text(tr("Grava os nomes das peças."));
                ui.add_enabled(suporte.camadas, egui::Checkbox::new(&mut o.camadas, tr("Camadas"))).on_hover_text(tr("Grava as camadas que a peça tiver."));
                if malha {
                    ui.checkbox(&mut o.malha.normais, tr("Normais")).on_hover_text(tr("Grava a normal de cada vértice nos formatos que têm esse campo: a superfície curva aparece lisa."));
                }
            });
        });
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
        let registro = Registro::default();
        ui.label(tr("Formato"));
        egui::ComboBox::from_id_salt("exportar_formato").selected_text(self.formato.nome()).width(LARGURA - 40.).show_ui(ui, |ui| {
            for f in registro.formatos_de_gravacao().into_iter().filter(|f| *f != Formato::Arvore) {
                ui.selectable_value(&mut self.formato, f, f.nome());
            }
        }).response.on_hover_text(tr("STEP leva a árvore de produtos e é o padrão atual. IGES serve a quem só lê superfícies."));
        opcoes_ui(ui, self.formato, &mut self.opcoes, self.montagem);
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
        // desenvolvimento: FENKRA_DEMO_EXPORTAR_ARQUIVO=<arquivo> grava sem perguntar, para conferir a saída
        let demo = std::env::var("FENKRA_DEMO_EXPORTAR_ARQUIVO").ok().map(std::path::PathBuf::from);
        let Some(caminho) = demo.or_else(|| rfd::FileDialog::new().add_filter(d.formato.nome(), d.formato.extensoes()).set_file_name(format!("{nome}.{extensao}")).save_file()) else { return };
        // PMI semântico: só na peça aberta, em STEP AP242 e com a opção ligada; o que não vai vira aviso
        let mut fora_do_step = vec![];
        let r = if d.formato.e_2d() || (!d.montagem && fenkra_solidos::corpo_malha::tem_corpo_de_malha(&self.open.doc)) {
            Registro::default().gravar_documento(&self.open.doc, &caminho, &d.opcoes)
        } else {
            let com_pmi = d.formato == Formato::Step && d.opcoes.pmi && d.opcoes.esquema == EsquemaSaida::Ap242 && !d.montagem;
            let modelo = match (self.asm.as_ref().filter(|_| d.montagem), &pasta) {
                (Some(s), Some(pasta)) => ModeloNeutro::da_montagem(&s.asm, pasta),
                _ if com_pmi => ModeloNeutro::de_documento_pmi(&self.open.doc).map(|(m, fora)| {
                    fora_do_step = fora;
                    m
                }),
                _ => ModeloNeutro::de_documento(&self.open.doc),
            };
            modelo.and_then(|m| Registro::default().gravar(&m, &caminho, &d.opcoes))
        };
        match r {
            Ok(()) if !fora_do_step.is_empty() => {
                let itens: Vec<String> = fora_do_step.iter().map(|n| n.texto(&|s| tr(s))).collect();
                self.toast(format!("{} {}: {}", tr("Exportado"), caminho.display(), itens.join(" ")), false)
            }
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
    fn malha_mostra_suporte_desvio_e_codificacao_so_onde_existe() {
        let mut d = Dialogo::novo(Formato::Stl, OpcoesExportar::default(), false);
        let t = textos(&mut d).join("\n");
        assert!(t.contains("Binário") && t.contains("Desvio da malha") && !t.contains("Esquema"), "{t}");
        assert!(t.contains("Leva: Malha"), "{t}");
        assert!(t.contains("Não leva") && t.contains("não diz a unidade"), "{t}");
        d.formato = Formato::Glb;
        let t = textos(&mut d).join("\n");
        assert!(!t.contains("Binário") && t.contains("sempre gravado em metros") && !t.contains("Unidade do arquivo"), "{t}");
        assert!(t.contains("Montagem"), "{t}");
    }

    #[test]
    fn termos_do_relatorio_de_malha_tem_traducao() {
        for t in fenkra_conversor::malha::ITENS_FORA {
            assert!(crate::i18n::tem_traducao(t), "sem tradução: {t}");
        }
    }

    #[test]
    fn formato_2d_nao_mostra_opcao_de_solido() {
        let mut d = Dialogo::novo(Formato::Svg, OpcoesExportar::default(), false);
        let t = textos(&mut d).join("\n");
        assert!(t.contains("esboços") && !t.contains("Modo do IGES") && !t.contains("Desvio") && !t.contains("Unidade do arquivo"), "{t}");
    }

    #[test]
    fn combo_de_formato_lista_o_que_o_conversor_grava() {
        let gravaveis = Registro::default().formatos_de_gravacao();
        assert!(gravaveis.contains(&Formato::Tmf) && gravaveis.contains(&Formato::Usdz));
        for l in Registro::default().matriz() {
            for (nome, _) in l.capacidades.colunas() {
                assert!(crate::i18n::tem_traducao(nome), "sem tradução: {nome}");
            }
        }
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
            "Leva",
            "Não leva",
            "Avançado",
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
