// SPDX-License-Identifier: LGPL-2.1-or-later
//! Gerenciador único de salvamento e exportação (B378).
//!
//! O menu Arquivo tinha vinte itens de gravar e exportar misturados. Agora ele guarda só o que se
//! faz o tempo todo (abrir, importar, salvar, salvar como) e uma entrada para este gerenciador, que
//! reúne todos os destinos em grupos: salvar, troca de CAD, malha e visualização, fabricação (com
//! nesting) e dados. A lista é uma tabela de dados, testável; a janela só a desenha.

use super::*;
use crate::paineis::{self, Encaixe, Painel};

/// O que cada linha do gerenciador faz.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Destino {
    Salvar,
    SalvarTudo,
    SalvarComReferencias,
    Empacotar,
    ReporLinks,
    Step,
    Iges,
    Malha,
    Stl,
    Visualizacao,
    VisualizacaoSimplificada,
    Impressao,
    Nesting,
    DesenhoPdf,
    DesenhoSvg,
    DesenhoDxf,
    ListaDePecas,
    Arvore,
    Lote,
    Formatos,
    RelatorioImportacao,
    FeaturesEncontradas,
}

pub(crate) struct Entrada {
    pub grupo: &'static str,
    pub rotulo: &'static str,
    pub dica: &'static str,
    pub destino: Destino,
}

/// Todos os destinos, na ordem em que aparecem, agrupados.
pub(crate) const ENTRADAS: &[Entrada] = &[
    Entrada { grupo: "Salvar", rotulo: "Salvar", dica: "Grava a peça, a montagem ou o desenho em uso.   Ctrl+S", destino: Destino::Salvar },
    Entrada { grupo: "Salvar", rotulo: "Salvar tudo…", dica: "Mostra o que mudou em todos os documentos abertos e grava de uma vez.   Ctrl+Shift+S", destino: Destino::SalvarTudo },
    Entrada { grupo: "Salvar", rotulo: "Salvar como com referências…", dica: "Grava uma cópia e decide o que fazer com as peças que ela usa.", destino: Destino::SalvarComReferencias },
    Entrada { grupo: "Salvar", rotulo: "Empacotar…", dica: "Reúne o documento e tudo que ele usa numa pasta para enviar.", destino: Destino::Empacotar },
    Entrada { grupo: "Salvar", rotulo: "Reparar links…", dica: "Acha de novo as peças que mudaram de lugar.", destino: Destino::ReporLinks },
    Entrada { grupo: "Troca com outros CADs", rotulo: "Exportar STEP…", dica: "Sólido exato, para qualquer programa de modelagem.", destino: Destino::Step },
    Entrada { grupo: "Troca com outros CADs", rotulo: "Exportar IGES…", dica: "Superfícies e curvas exatas, para programas mais antigos.", destino: Destino::Iges },
    Entrada { grupo: "Malha e visualização", rotulo: "Exportar malha…", dica: "Formatos abertos de malha e visualização: 3MF, OBJ, PLY, glTF, USD e outros.", destino: Destino::Malha },
    Entrada { grupo: "Malha e visualização", rotulo: "Exportar STL", dica: "Malha para impressão ou fresagem, com a qualidade que você escolher.", destino: Destino::Stl },
    Entrada { grupo: "Malha e visualização", rotulo: "Exportar visualização…", dica: "Página que abre a peça no navegador do computador ou do celular, sem instalar nada.", destino: Destino::Visualizacao },
    Entrada { grupo: "Malha e visualização", rotulo: "Exportar visualização simplificada…", dica: "Igual à anterior, com a malha reduzida: protege a geometria e pesa menos.", destino: Destino::VisualizacaoSimplificada },
    Entrada { grupo: "Fabricação", rotulo: "Preparar impressão…", dica: "Põe os corpos na mesa, confere se imprimem e grava 3MF ou STL.", destino: Destino::Impressao },
    Entrada { grupo: "Fabricação", rotulo: "Nesting e dobra…", dica: "Encaixa as chapas planificadas em chapas comerciais e simula a sequência de dobra. Só para peças de chapa.", destino: Destino::Nesting },
    Entrada { grupo: "Desenho e montagem", rotulo: "Desenho em PDF…", dica: "As folhas do desenho técnico, com a cor e as espessuras de linha. Só na bancada Desenho.", destino: Destino::DesenhoPdf },
    Entrada { grupo: "Desenho e montagem", rotulo: "Desenho em SVG…", dica: "As folhas em vetor, para editar ou publicar. Só na bancada Desenho.", destino: Destino::DesenhoSvg },
    Entrada { grupo: "Desenho e montagem", rotulo: "Desenho em DXF…", dica: "As vistas e cotas em camadas, para corte a laser ou outro programa de desenho. Só na bancada Desenho.", destino: Destino::DesenhoDxf },
    Entrada { grupo: "Desenho e montagem", rotulo: "Lista de peças (CSV)…", dica: "Abre a lista de peças da montagem, com item, PN e quantidade, e grava em CSV. Só na montagem.", destino: Destino::ListaDePecas },
    Entrada { grupo: "Dados e lote", rotulo: "Exportar árvore paramétrica…", dica: "A receita da peça ou da montagem em JSON aberto, sem a geometria pronta.", destino: Destino::Arvore },
    Entrada { grupo: "Dados e lote", rotulo: "Conversão em lote…", dica: "Converte arquivos e pastas para vários formatos de uma vez, com simulação antes de gravar.", destino: Destino::Lote },
    Entrada { grupo: "Dados e lote", rotulo: "Formatos suportados…", dica: "O que o Fenkra lê e grava, e o que atravessa em cada formato.", destino: Destino::Formatos },
    Entrada { grupo: "Dados e lote", rotulo: "Relatório da importação…", dica: "O relatório guardado no documento importado. Só para peças vindas de outro programa.", destino: Destino::RelatorioImportacao },
    Entrada { grupo: "Dados e lote", rotulo: "Features encontradas…", dica: "Procura furos, filetes e chanfros no corpo importado e mostra no 3D.", destino: Destino::FeaturesEncontradas },
];

/// Nomes dos grupos, na ordem de aparição.
pub(crate) fn grupos() -> Vec<&'static str> {
    let mut v: Vec<&'static str> = vec![];
    for e in ENTRADAS {
        if !v.contains(&e.grupo) {
            v.push(e.grupo);
        }
    }
    v
}

impl App {
    pub(crate) fn abrir_gerenciador(&mut self) {
        self.gerenciador_aberto = true;
    }

    /// A linha pode ser usada com o documento aberto?
    fn destino_ativo(&self, d: Destino) -> bool {
        match d {
            Destino::Nesting => self.open.doc.sheet.is_some(),
            Destino::DesenhoPdf | Destino::DesenhoSvg | Destino::DesenhoDxf => self.settings.bench == crate::settings::Bench::Desenho && self.dwg.is_some(),
            Destino::ListaDePecas => self.in_asm(),
            Destino::RelatorioImportacao => self.open.doc.origem.as_ref().is_some_and(|o| o.relatorio.is_some()),
            Destino::FeaturesEncontradas => self.features.is_none(),
            _ => true,
        }
    }

    fn executar_destino(&mut self, ctx: &egui::Context, d: Destino) {
        match d {
            Destino::Salvar => self.salvar_ativo(),
            Destino::SalvarTudo => self.abrir_plano(),
            Destino::SalvarComReferencias => self.abrir_salvar_como(ctx),
            Destino::Empacotar => self.abrir_empacotar(ctx),
            Destino::ReporLinks => self.abrir_reparar_links(ctx),
            Destino::Step => self.abrir_painel_exportar(fenkra_conversor::Formato::Step),
            Destino::Iges => self.abrir_painel_exportar(fenkra_conversor::Formato::Iges),
            Destino::Malha => self.abrir_painel_exportar(fenkra_conversor::Formato::Glb),
            Destino::Stl => {}
            Destino::Visualizacao => self.exportar_visualizacao(false),
            Destino::VisualizacaoSimplificada => self.exportar_visualizacao(true),
            Destino::Impressao => self.abrir_impressao(),
            Destino::Nesting => crate::chapa_fab_ui::abrir(ctx),
            Destino::DesenhoPdf => crate::dwg_ui::command(self, crate::dwg_ui::c::PDF),
            Destino::DesenhoSvg => crate::dwg_ui::command(self, crate::dwg_ui::c::SVG),
            Destino::DesenhoDxf => crate::dwg_ui::command(self, crate::dwg_ui::c::DXF),
            Destino::ListaDePecas => {
                if let Some(st) = self.asm.as_mut() {
                    st.bom_open = Some(st.bom_open.unwrap_or_default());
                }
            }
            Destino::Arvore => self.exportar_arvore(),
            Destino::Lote => self.abrir_painel_lote(),
            Destino::Formatos => self.abrir_formatos(),
            Destino::RelatorioImportacao => self.abrir_relatorio_embutido(),
            Destino::FeaturesEncontradas => self.abrir_features(),
        }
    }

    /// Janela do gerenciador: painel livre e encaixável, aberto só quando pedido.
    pub(crate) fn gerenciador_ui(&mut self, ctx: &egui::Context, area: Rect) {
        if !self.gerenciador_aberto {
            return;
        }
        let posicao = paineis::Posicao { fechado: false, ..self.settings.painel("gerenciador", Encaixe::Esquerda) };
        let cfg = Painel { nome: "gerenciador".into(), titulo: tr("Salvar e exportar").to_string(), padrao: paineis::ao_lado_da_arvore(area), largura: 360., redimensiona: true };
        let mut pedido: Option<Destino> = None;
        let mut stl: Option<fenkra_core::PresetStl> = None;
        let saida = paineis::mostrar(ctx, area, &cfg, posicao, |ui| {
            ui.label(RichText::new(tr("Tudo que grava ou exporta, num lugar só. Passe o mouse sobre uma linha para ver o que ela faz.")).weak().small());
            ui.add_space(4.);
            egui::ScrollArea::vertical().show(ui, |ui| {
                for g in grupos() {
                    egui::CollapsingHeader::new(RichText::new(tr(g)).strong()).default_open(true).show(ui, |ui| {
                        for e in ENTRADAS.iter().filter(|e| e.grupo == g) {
                            let ativo = self.destino_ativo(e.destino);
                            if e.destino == Destino::Stl {
                                ui.menu_button(tr(e.rotulo), |ui| {
                                    for p in fenkra_core::PresetStl::TODOS {
                                        if ui.button(tr(p.nome())).clicked() {
                                            stl = Some(p);
                                            ui.close();
                                        }
                                    }
                                })
                                .response
                                .on_hover_text(tr(e.dica));
                                continue;
                            }
                            let r = ui.add_enabled(ativo, egui::Button::new(tr(e.rotulo)).min_size(egui::vec2(ui.available_width(), 0.)));
                            if r.on_hover_text(tr(e.dica)).clicked() {
                                pedido = Some(e.destino);
                            }
                        }
                    });
                }
            });
        });
        if let Some(mut p) = saida.movido {
            let fechou = std::mem::take(&mut p.fechado);
            self.settings.guardar_painel("gerenciador", p);
            if fechou {
                self.gerenciador_aberto = false;
            }
        }
        if let Some(p) = stl {
            self.export("stl", p);
        }
        if let Some(d) = pedido {
            self.executar_destino(ctx, d);
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn todos_os_destinos_do_menu_antigo_estao_no_gerenciador() {
        let tem = |d: Destino| ENTRADAS.iter().any(|e| e.destino == d);
        for d in [
            Destino::Salvar,
            Destino::SalvarTudo,
            Destino::SalvarComReferencias,
            Destino::Empacotar,
            Destino::ReporLinks,
            Destino::Step,
            Destino::Iges,
            Destino::Malha,
            Destino::Stl,
            Destino::Visualizacao,
            Destino::VisualizacaoSimplificada,
            Destino::Impressao,
            Destino::Nesting,
            Destino::DesenhoPdf,
            Destino::DesenhoSvg,
            Destino::DesenhoDxf,
            Destino::ListaDePecas,
            Destino::Arvore,
            Destino::Lote,
            Destino::Formatos,
            Destino::RelatorioImportacao,
            Destino::FeaturesEncontradas,
        ] {
            assert!(tem(d), "{d:?} sumiu do gerenciador");
        }
    }

    #[test]
    fn nesting_fica_agrupado_com_a_fabricacao_e_o_destino_nao_se_repete() {
        let n = ENTRADAS.iter().find(|e| e.destino == Destino::Nesting).expect("nesting");
        assert_eq!(n.grupo, "Fabricação");
        for (i, a) in ENTRADAS.iter().enumerate() {
            assert!(ENTRADAS.iter().skip(i + 1).all(|b| b.destino != a.destino), "{:?} repetido", a.destino);
        }
        assert_eq!(grupos(), ["Salvar", "Troca com outros CADs", "Malha e visualização", "Fabricação", "Desenho e montagem", "Dados e lote"]);
    }

    #[test]
    fn todo_texto_do_gerenciador_tem_traducao() {
        for e in ENTRADAS {
            for t in [e.grupo, e.rotulo, e.dica] {
                assert!(crate::i18n::tem_traducao(t), "sem tradução: {t}");
            }
        }
    }
}
