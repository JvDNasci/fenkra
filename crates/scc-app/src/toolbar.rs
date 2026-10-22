//! Barras de comandos em grupos (RF-UI-001): cada grupo pode ficar no topo, na lateral,
//! flutuar pela tela ou ser ocultado. Clique direito no grupo muda a posição.
//! Cada botão tem ícone + nome; a dica mostra a descrição breve e, após 5 s, a avançada.

use crate::app::Cmd;
use crate::icons::tool_button;
use crate::render::DisplayMode;
use crate::settings::{Dock, Group, Settings};
use crate::sketch_ui::Tool;
use eframe::egui::{self, RichText};

/// Ação pedida por um botão de ferramenta.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Act {
    Cmd(Cmd),
    Fit,
    View(f32, f32),
    Normal,
    Ortho,
    HiddenEnv,
    Settings,
    Dock(Group, Dock),
    SkTool(Tool),
    SkConstruction,
    SkGrid,
    SkSnap,
    SkAutoConstraints,
    SkAutoDims,
    SkFinish,
}

/// Item de barra: ícone, nome, descrição breve, descrição avançada, ação.
pub struct Item {
    pub icon: &'static str,
    pub name: &'static str,
    pub brief: &'static str,
    pub advanced: &'static str,
    pub act: Act,
}

fn it(icon: &'static str, name: &'static str, brief: &'static str, advanced: &'static str, act: Act) -> Item {
    Item { icon, name, brief, advanced, act }
}

pub fn items(g: Group) -> Vec<Item> {
    use crate::app::Cmd as C;
    use Act::*;
    match g {
        Group::Sketch => vec![it("sketch", "Sketch", "Novo sketch no plano selecionado", "Cria um sketch no plano ou referência selecionada (ou XY) e entra no modo de desenho: a vista se alinha ao plano e aparecem as ferramentas de desenho, cotagem e restrições.", Cmd(C::Sketch))],
        Group::SketchDesenho => {
            let mut v: Vec<Item> = Tool::DRAW
                .iter()
                .map(|t| {
                    let (i, n, b, a) = t.info();
                    it(i, n, b, a, SkTool(*t))
                })
                .collect();
            v.push(it("sk_construcao", "Construção", "Geometria de construção  (X)", "Liga/desliga: as próximas curvas serão de construção (tracejadas), usadas como guias e eixos e ignoradas nos perfis.", SkConstruction));
            v.push(it("sk_grade", "Grade", "Grade  (G)", "Mostra/oculta a grade do plano. O passo é ajustado ao zoom a partir do passo das configurações.", SkGrid));
            v.push(it("sk_snap", "Snaps", "Snaps", "Atrai o cursor para pontos existentes, para a origem e para a grade.", SkSnap));
            v.push(it("sk_auto_restricao", "Auto restr.", "Restrições automáticas", "Linhas quase horizontais/verticais recebem a restrição; clicar em um ponto existente reaproveita o ponto (coincidência).", SkAutoConstraints));
            v.push(it("sk_auto_cota", "Auto cotas", "Cotas automáticas", "Cada forma criada já recebe suas cotas (comprimento, diâmetro, raio, largura e altura), editáveis depois.", SkAutoDims));
            v.push(it("sk_concluir", "Concluir", "Concluir sketch  (Esc)", "Sai do modo sketch. O sketch continua editável: clique duas vezes nele na árvore.", SkFinish));
            v
        }
        Group::SketchRestricoes => Tool::CONSTRAINTS
            .iter()
            .map(|t| {
                let (i, n, b, a) = t.info();
                it(i, n, b, a, SkTool(*t))
            })
            .collect(),
        Group::Basicos => vec![
            it("extrusao", "Extrusão", "Extrudar o sketch", "Cria material empurrando o perfil fechado do sketch ao longo da normal do plano: cego, simétrico, dois sentidos ou passante.", Cmd(C::Extrude)),
            it("recorte", "Recorte", "Remover material pelo sketch", "Como a extrusão, mas remove material. Por padrão entra no material (contra a normal) e atravessa tudo.", Cmd(C::Cut)),
            it("revolucao", "Revolução", "Girar o perfil em torno de um eixo", "Gira o perfil fechado em torno de um eixo da Origem ou de uma linha (de construção) do sketch.", Cmd(C::Revolve)),
            it("furo", "Furo", "Furo inteligente", "Furos nos pontos de um sketch, escolhendo a rosca/tamanho (M3–M20) e o tipo: passagem, roscado, rebaixado ou escareado.", Cmd(C::Hole)),
        ],
        Group::Acabamento => vec![
            it("filete", "Filete", "Arredondar arestas", "Arredonda as arestas do corpo com o raio informado (aceita expressão).", Cmd(C::Fillet)),
            it("chanfro", "Chanfro", "Chanfrar arestas", "Chanfra as arestas do corpo com a distância informada.", Cmd(C::Chamfer)),
        ],
        Group::Referencias => vec![it("plano", "Plano", "Plano de referência", "Plano paralelo ao plano selecionado, deslocado por uma distância (aceita expressão). Vira uma variável nomeada na pasta Referências.", Cmd(C::Plane))],
        Group::Organizacao => vec![
            it("parametro", "Parâmetro", "Novo parâmetro", "Cria um parâmetro global da peça (nome = expressão), usável em qualquer cota ou comando.", Cmd(C::Param)),
            it("pasta", "Pasta", "Nova pasta", "Pasta para organizar comandos na árvore; não muda a ordem de execução.", Cmd(C::Folder)),
            it("coletor", "Coletor", "Novo coletor de corpos", "Grupo de corpos independente; vira o coletor ativo para os próximos comandos.", Cmd(C::Collector)),
        ],
        Group::Vista => vec![
            it("ajustar", "Ajustar", "Ajustar à tela  (F)", "Enquadra todo o modelo na viewport. Duplo clique no fundo também ajusta.", Fit),
            it("vista_iso", "Iso", "Vista isométrica  (Ctrl+7)", "", View(-0.785, 0.6)),
            it("vista_frente", "Frente", "Vista frontal  (Ctrl+1)", "", View(-1.5708, 0.0)),
            it("vista_topo", "Topo", "Vista superior  (Ctrl+5)", "", View(-1.5708, 1.5706)),
            it("vista_direita", "Direita", "Vista lateral direita  (Ctrl+3)", "", View(0.0, 0.0)),
            it("ortho", "Ortogonal", "Projeção ortogonal  (O)", "Alterna entre perspectiva e projeção ortogonal (sem distorção, ideal para sketches).", Ortho),
        ],
    }
}

/// Estado necessário para marcar botões selecionados.
pub struct ToolState {
    pub hidden_env: bool,
    pub sketch_tool: Option<Tool>,
    pub construction: bool,
}

/// Mostra os botões de um grupo; devolve as ações clicadas.
pub fn show_group(ui: &mut egui::Ui, g: Group, s: &mut Settings, st: &ToolState, vertical: bool) -> Vec<Act> {
    let mut acts = vec![];
    let size = s.icon_size;
    let labels = s.show_labels && !vertical;
    let mut body = |ui: &mut egui::Ui| {
        for i in items(g) {
            let selected = match i.act {
                Act::Ortho => s.ortho,
                Act::SkTool(t) => st.sketch_tool == Some(t),
                Act::SkConstruction => st.construction,
                Act::SkGrid => s.grid,
                Act::SkSnap => s.snaps,
                Act::SkAutoConstraints => s.auto_constraints,
                Act::SkAutoDims => s.auto_dims,
                _ => false,
            };
            if tool_button(ui, i.icon, i.name, size, labels, selected, i.brief, i.advanced).clicked() {
                acts.push(i.act);
            }
        }
        if g == Group::Vista {
            let r = tool_button(ui, "estilo", "Estilo", size, labels, false, "Estilo de exibição", "Sombreado com/sem arestas, linhas ocultas, arame, raio-X ou semi-renderizado.");
            egui::Popup::menu(&r).show(|ui| {
                for m in DisplayMode::ALL {
                    ui.radio_value(&mut s.display, m, m.label());
                }
                ui.separator();
                ui.checkbox(&mut s.edges_in_render, "arestas no semi-renderizado");
            });
            let r = tool_button(ui, "filtro", "Filtro", size, labels, false, "Filtro de exibição", "Escolha quais tipos de elemento aparecem na viewport.");
            egui::Popup::menu(&r).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside).show(|ui| {
                let f = &mut s.filter;
                ui.label(RichText::new("Mostrar na viewport").strong());
                ui.checkbox(&mut f.corpos, "Corpos");
                ui.checkbox(&mut f.sketches, "Sketches");
                ui.checkbox(&mut f.pontos, "Pontos de sketch");
                ui.checkbox(&mut f.planos_origem, "Planos da Origem");
                ui.checkbox(&mut f.planos_ref, "Planos de referência");
                ui.checkbox(&mut f.eixos, "Eixos da Origem");
                ui.checkbox(&mut f.rotulos, "Rótulos dos planos");
            });
            if tool_button(ui, "ocultos", "Ocultos", size, labels, st.hidden_env, "Ambiente de ocultos  (Shift+H)", "Mostra só os itens ocultos, para reexibir o que precisar. Clique de novo para voltar.").clicked() {
                acts.push(Act::HiddenEnv);
            }
            if tool_button(ui, "config", "Ajustes", size, labels, false, "Configurações", "Tema, fundo, estilos, barras de ferramentas, cotagem e sketch.").clicked() {
                acts.push(Act::Settings);
            }
        }
    };
    let inner = if vertical {
        ui.vertical_centered(|ui| body(ui)).response
    } else {
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| body(ui)).response
    };
    // clique direito no grupo: escolher onde ele fica
    inner.context_menu(|ui| {
        ui.label(RichText::new(g.label()).strong());
        for d in Dock::ALL {
            if ui.radio(s.dock(g) == d, d.label()).clicked() {
                acts.push(Act::Dock(g, d));
                ui.close();
            }
        }
    });
    acts
}

/// Grupos visíveis com determinada posição: no modo sketch, só os de sketch e a Vista.
pub fn groups_at(s: &Settings, d: Dock, sketching: bool) -> Vec<Group> {
    Group::ALL
        .iter()
        .copied()
        .filter(|g| s.dock(*g) == d)
        .filter(|g| if sketching { g.sketch_only() || *g == Group::Vista } else { !g.sketch_only() })
        .collect()
}
