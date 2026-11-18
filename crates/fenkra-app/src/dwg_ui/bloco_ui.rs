// SPDX-License-Identifier: LGPL-2.1-or-later
//! B512: editor de bloco de título e modelos de folha.
//!
//! O comando abre o modelo da folha atual em edição (ou cria um, a partir do formato da folha) e mostra
//! o painel só enquanto a edição durar. A prévia é a própria folha: cada mudança grava o modelo no
//! desenho e todas as folhas que o usam se refazem. Na folha, um clique escolhe a caixa, arrastar uma
//! divisória muda a divisão e o canto de cima à esquerda do bloco muda o tamanho dele. A regra de cada
//! operação mora em `fenkra_desenho::modelo_folha`; aqui só se liga o mouse e o painel. Cores só do
//! tema.

use super::*;
use crate::theme;
use fenkra_desenho::modelo_folha::{self as mf, Alinhamento, Caminho, Conteudo, Corte, ModeloFolha};

/// O que o arraste em curso mexe.
#[derive(Clone, Debug, PartialEq)]
enum Arraste {
    Divisoria(Caminho),
    Canto,
}

/// Estado da edição do bloco.
#[derive(Default)]
pub struct Estado {
    /// `uid` do modelo em edição
    pub editando: Option<String>,
    /// desenho de antes da edição: Cancelar volta para ele
    antes: Option<Drawing>,
    /// caixa escolhida
    pub sel: Caminho,
    arraste: Option<Arraste>,
    /// aviso da última operação (arquivo gravado, imagem que não abriu…)
    aviso: Option<(String, bool)>,
}

/// Abre a edição do modelo da folha atual; sem modelo, cria um com o formato da folha e o aplica.
pub fn abrir(st: &mut DwgState) {
    super::esboco_ui::concluir(st);
    st.tool = None;
    st.clicks.clear();
    st.sel = None;
    st.focus = None;
    st.props_open = false;
    let antes = st.d.clone();
    let Some(sh) = st.d.sheet(st.sheet).cloned() else { return };
    let uid = match st.d.modelo_da_folha(&sh) {
        Some(m) => m.uid.clone(),
        None => {
            let mut m = ModeloFolha::novo(&format!("{} {}", tr("Modelo"), sh.format.label()), sh.format, sh.portrait);
            m.zonas = sh.zones;
            let uid = m.uid.clone();
            st.d.aplicar_modelo(sh.id, m);
            uid
        }
    };
    st.bloco = Estado { editando: Some(uid), antes: Some(antes), sel: vec![], arraste: None, aviso: None };
    st.bloco.sel = primeira_celula(st).unwrap_or_default();
}

fn primeira_celula(st: &DwgState) -> Option<Caminho> {
    let m = modelo(st)?;
    m.bloco.celulas([0., 0.]).into_iter().next().map(|x| x.0)
}

fn modelo(st: &DwgState) -> Option<&ModeloFolha> {
    st.d.modelo(st.bloco.editando.as_ref()?)
}

/// Fecha a edição mantendo o que foi feito.
pub fn concluir(st: &mut DwgState) {
    st.bloco = Estado::default();
}

/// Fecha a edição e desfaz tudo o que ela fez.
pub fn cancelar(st: &mut DwgState) {
    if let Some(antes) = st.bloco.antes.take() {
        st.d = antes;
    }
    concluir(st);
}

/// Esc: solta o arraste; sem arraste, conclui. Devolve se tratou.
pub fn esc(st: &mut DwgState) -> bool {
    if st.bloco.editando.is_none() {
        return false;
    }
    if st.bloco.arraste.take().is_some() {
        super::alcas::end_drag(st);
    } else {
        concluir(st);
    }
    true
}

fn dist_seg(p: V2, a: V2, b: V2) -> f64 {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let l2 = dx * dx + dy * dy;
    let t = if l2 > 0. { (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / l2).clamp(0., 1.) } else { 0. };
    let (qx, qy) = (a[0] + t * dx, a[1] + t * dy);
    ((p[0] - qx).powi(2) + (p[1] - qy).powi(2)).sqrt()
}

/// Edição na folha: realce da caixa sob o cursor e da escolhida, divisórias e canto arrastáveis.
#[allow(clippy::too_many_arguments)]
pub(super) fn tratar(ui: &egui::Ui, painter: &egui::Painter, st: &mut DwgState, resp: &egui::Response, mouse: Option<V2>, to_scr: &dyn Fn(V2) -> Pos2, to_mm: &dyn Fn(Pos2) -> V2, z: f32, rect: Rect) {
    let Some(uid) = st.bloco.editando.clone() else { return };
    let Some(sh) = st.d.sheet(st.sheet).cloned() else { return };
    let Some(m) = st.d.modelo(&uid).cloned() else {
        concluir(st);
        return;
    };
    if sh.modelo.as_deref() != Some(uid.as_str()) {
        // trocou para uma folha que não usa este modelo: a edição continua no painel, sem alças
        painter.text(rect.center_top() + Vec2::new(0., 10.), egui::Align2::CENTER_TOP, tr("Esta folha não usa o modelo em edição"), egui::FontId::proportional(14.), ui.visuals().warn_fg_color);
        return;
    }
    let folha = sh.size();
    let origem = m.origem_do_bloco(folha);
    let acc = theme::accent(ui.visuals());
    let sel_fill = ui.visuals().selection.bg_fill.gamma_multiply(0.35);
    painter.text(
        rect.center_top() + Vec2::new(0., 10.),
        egui::Align2::CENTER_TOP,
        format!("{}   (Esc {})", tr("Clique numa caixa para editá-la; arraste as divisórias e o canto do bloco"), tr("conclui")),
        egui::FontId::proportional(14.),
        ui.visuals().warn_fg_color,
    );
    let tol = (7. / z) as f64;
    let divs = m.bloco.divisorias(origem);
    let canto: V2 = [origem[0], origem[1] + m.bloco.altura];
    let perto_canto = |p: V2| ((p[0] - canto[0]).powi(2) + (p[1] - canto[1]).powi(2)).sqrt() < tol * 1.3;
    let perto_div = |p: V2| divs.iter().filter(|d| dist_seg(p, d.de, d.ate) < tol).min_by(|a, b| dist_seg(p, a.de, a.ate).total_cmp(&dist_seg(p, b.de, b.ate))).cloned();
    // caixas: a escolhida pintada, a sob o cursor contornada
    let sob = mouse.and_then(|p| m.bloco.celula_em(origem, p));
    for (cam, (lo, hi), _) in m.bloco.celulas(origem) {
        let r = Rect::from_two_pos(to_scr(lo), to_scr(hi));
        if cam == st.bloco.sel {
            painter.rect_filled(r, 0., sel_fill);
            painter.rect_stroke(r, 0., Stroke::new(2., acc), egui::StrokeKind::Inside);
        } else if Some(&cam) == sob.as_ref() && st.bloco.arraste.is_none() {
            painter.rect_stroke(r, 0., Stroke::new(1.2, acc), egui::StrokeKind::Inside);
        }
    }
    // divisórias: alça em toda a linha; a pegada ou sob o cursor fica mais grossa, com a medida
    let ativa = match &st.bloco.arraste {
        Some(Arraste::Divisoria(c)) => divs.iter().find(|d| &d.caminho == c).cloned(),
        _ => None,
    };
    let quente = ativa.clone().or_else(|| mouse.filter(|_| st.bloco.arraste.is_none()).and_then(perto_div));
    for dv in &divs {
        let grossa = quente.as_ref().is_some_and(|q| q.caminho == dv.caminho);
        painter.line_segment([to_scr(dv.de), to_scr(dv.ate)], Stroke::new(if grossa { 3.5 } else { 1.5 }, acc.gamma_multiply(if grossa { 1. } else { 0.7 })));
    }
    if let Some(q) = &quente {
        ui.ctx().set_cursor_icon(match q.corte {
            Corte::Vertical => egui::CursorIcon::ResizeHorizontal,
            Corte::Horizontal => egui::CursorIcon::ResizeVertical,
        });
        if let (Some(pos), Some(mp)) = (m.bloco.pos_da_divisoria(&q.caminho), mouse) {
            let t = format!("{} mm", dec_txt(pos));
            painter.text(to_scr(mp) + Vec2::new(14., -14.), egui::Align2::LEFT_BOTTOM, t, egui::FontId::proportional(13.), acc);
        }
    }
    // canto do bloco: quadrado; arrastar muda largura e altura
    let c = to_scr(canto);
    let canto_quente = st.bloco.arraste == Some(Arraste::Canto) || (quente.is_none() && mouse.is_some_and(perto_canto));
    let lado = if canto_quente { 12. } else { 9. };
    painter.rect_filled(Rect::from_center_size(c, Vec2::splat(lado)), 1., ui.visuals().window_fill);
    painter.rect_stroke(Rect::from_center_size(c, Vec2::splat(lado)), 1., Stroke::new(2., acc), egui::StrokeKind::Middle);
    if canto_quente {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeNwSe);
        painter.text(c + Vec2::new(-10., -10.), egui::Align2::RIGHT_BOTTOM, format!("{} × {} mm", dec_txt(m.bloco.largura), dec_txt(m.bloco.altura)), egui::FontId::proportional(13.), acc);
    }
    // mouse
    if resp.drag_started_by(egui::PointerButton::Primary) {
        // o arraste só começa depois de o mouse andar um pouco: o alvo é onde o botão desceu
        let inicio = ui.input(|i| i.pointer.press_origin()).map(to_mm).or(mouse);
        if let Some(p) = inicio {
            let alvo = if perto_canto(p) { Some(Arraste::Canto) } else { perto_div(p).map(|d| Arraste::Divisoria(d.caminho)) };
            if let Some(a) = alvo {
                super::alcas::begin_drag(st);
                st.bloco.arraste = Some(a);
            }
        }
    }
    if let (Some(a), Some(p)) = (st.bloco.arraste.clone(), mouse) {
        if resp.dragged_by(egui::PointerButton::Primary) {
            let mg = m.margens;
            if let Some(mm) = st.d.modelo_mut(&uid) {
                match a {
                    Arraste::Divisoria(cam) => {
                        mm.bloco.arrastar_divisoria(origem, &cam, p);
                    }
                    Arraste::Canto => {
                        // o canto de baixo à direita fica preso na moldura
                        let direita = folha[0] - mg.direita;
                        let larg_max = direita - mg.esquerda;
                        let alt_max = folha[1] - mg.cima - mg.baixo;
                        mm.bloco.largura = (((direita - p[0]) * 2.).round() / 2.).clamp(20., larg_max);
                        mm.bloco.altura = (((p[1] - mg.baixo) * 2.).round() / 2.).clamp(6., alt_max);
                    }
                }
            }
        }
    }
    if resp.drag_stopped() && st.bloco.arraste.take().is_some() {
        super::alcas::end_drag(st);
    }
    if resp.clicked_by(egui::PointerButton::Primary) {
        if let Some(cam) = mouse.and_then(|p| m.bloco.celula_em(origem, p)) {
            st.bloco.sel = cam;
        }
    }
}

/// Campos que o texto pode usar: os automáticos e as propriedades personalizadas que a folha conhece.
fn campos_da_folha(st: &mut DwgState) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = FIELDS.iter().map(|(k, d)| (k.to_string(), tr(d).to_string())).collect();
    if let Some(sh) = st.d.sheet(st.sheet).cloned() {
        let valores = layout::sheet_fields(&st.d, &sh, &mut st.models);
        let mut extras: Vec<&String> = valores.keys().filter(|k| !out.iter().any(|(x, _)| x.trim_matches(['{', '}']) == k.as_str())).collect();
        extras.sort();
        for k in extras {
            out.push((format!("{{{k}}}"), tr("propriedade personalizada").to_string()));
        }
    }
    out
}

/// Painel do comando (só aberto durante a edição). Devolve se tomou o painel.
pub(super) fn painel(ui: &mut egui::Ui, st: &mut DwgState) -> bool {
    let Some(uid) = st.bloco.editando.clone() else { return false };
    let Some(original) = st.d.modelo(&uid).cloned() else {
        concluir(st);
        return false;
    };
    let mut m = original.clone();
    ui.heading(tr("Bloco de título"));
    ui.label(RichText::new(tr("Clique numa caixa na folha para editá-la; arraste as divisórias e o canto do bloco. A folha mostra o resultado na hora.")).weak().small());
    if let Some((t, erro)) = &st.bloco.aviso {
        if *erro {
            ui.colored_label(ui.visuals().error_fg_color, t);
        } else {
            ui.label(RichText::new(t).color(theme::accent(ui.visuals())));
        }
    }
    ui.separator();
    // ---------- modelo de folha
    ui.label(RichText::new(tr("Modelo de folha")).strong());
    egui::Grid::new("bloco_folha").num_columns(2).spacing([8., 6.]).show(ui, |ui| {
        ui.label(tr("Nome"));
        ui.text_edit_singleline(&mut m.nome).on_hover_text(tr("Nome do modelo na biblioteca e na lista de modelos de folha"));
        ui.end_row();
        ui.label(tr("Formato"));
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("bloco_formato").selected_text(m.formato.label()).show_ui(ui, |ui| {
                for f in Format::ALL {
                    ui.selectable_value(&mut m.formato, f, f.label());
                }
            }).response.on_hover_text(tr("Tamanho da folha das folhas que usam o modelo"));
            ui.checkbox(&mut m.retrato, tr("Retrato")).on_hover_text(tr("Folha em pé"));
            ui.checkbox(&mut m.zonas, tr("Zoneamento")).on_hover_text(tr("Números e letras na margem para localizar detalhes"));
        });
        ui.end_row();
        ui.label(tr("Margens"));
        ui.horizontal(|ui| {
            for (rot, v, dica) in [
                ("E", &mut m.margens.esquerda, "Margem esquerda (mm): a de arquivamento, costuma ser maior"),
                ("D", &mut m.margens.direita, "Margem direita (mm)"),
                ("C", &mut m.margens.cima, "Margem de cima (mm)"),
                ("B", &mut m.margens.baixo, "Margem de baixo (mm)"),
            ] {
                ui.label(rot);
                ui.add(crate::ui_ext::dv(v).range(0.0..=60.).speed(0.5).dica(tr(dica)));
            }
        });
        ui.end_row();
        ui.label(tr("Bloco"));
        ui.horizontal(|ui| {
            ui.add(crate::ui_ext::dv(&mut m.bloco.largura).range(20.0..=600.).speed(0.5).suffix(" mm").dica(tr("Largura do bloco de título; o canto de baixo à direita fica preso na moldura")));
            ui.label("×");
            ui.add(crate::ui_ext::dv(&mut m.bloco.altura).range(6.0..=300.).speed(0.5).suffix(" mm").dica(tr("Altura do bloco de título")));
        });
        ui.end_row();
    });
    ui.separator();
    // ---------- caixa escolhida
    let folha = m.tamanho();
    let origem = m.origem_do_bloco(folha);
    if m.bloco.celula(&st.bloco.sel).is_none() {
        st.bloco.sel = m.bloco.celulas(origem).into_iter().next().map(|x| x.0).unwrap_or_default();
    }
    let sel = st.bloco.sel.clone();
    ui.label(RichText::new(tr("Caixa escolhida")).strong());
    ui.horizontal_wrapped(|ui| {
        if ui.button(tr("⬌ Dividir lado a lado")).on_hover_text(tr("Corta a caixa em duas com uma linha vertical; a da direita nasce vazia")).clicked() {
            if let Some(c) = m.bloco.dividir(origem, &sel, Corte::Vertical) {
                st.bloco.sel = c;
            }
        }
        if ui.button(tr("⬍ Dividir em cima e embaixo")).on_hover_text(tr("Corta a caixa em duas com uma linha horizontal; a de baixo nasce vazia")).clicked() {
            if let Some(c) = m.bloco.dividir(origem, &sel, Corte::Horizontal) {
                st.bloco.sel = c;
            }
        }
        if ui.add_enabled(!sel.is_empty(), egui::Button::new(tr("Juntar com a vizinha"))).on_hover_text(tr("Tira a linha que separa esta caixa da vizinha; o conteúdo desta fica")).clicked() {
            if let Some(c) = m.bloco.juntar(&sel) {
                st.bloco.sel = c;
            }
        }
    });
    let sel = st.bloco.sel.clone();
    // divisória que criou esta caixa: a posição se digita aqui também
    if let Some((_, pai)) = sel.split_last() {
        if let Some(pos) = m.bloco.pos_da_divisoria(pai) {
            let mut p = pos;
            ui.horizontal(|ui| {
                ui.label(tr("Divisória"));
                ui.add(crate::ui_ext::dv(&mut p).range(0.0..=600.).speed(0.5).suffix(" mm").dica(tr("Distância da linha que separa esta caixa da vizinha até a borda esquerda (ou de cima) da caixa dividida")));
            });
            if p != pos {
                m.bloco.por_divisoria(origem, pai, p);
            }
        }
    }
    let campos = campos_da_folha(st);
    let valores = st.d.sheet(st.sheet).cloned().map(|sh| layout::sheet_fields(&st.d, &sh, &mut st.models)).unwrap_or_default();
    let mut escolher_imagem = false;
    let mut revisoes_ui = false;
    if let Some(cel) = m.bloco.celula_mut(&sel) {
        ui.horizontal(|ui| {
            ui.label(tr("Conteúdo"));
            egui::ComboBox::from_id_salt("bloco_conteudo").selected_text(tr(cel.conteudo.label())).show_ui(ui, |ui| {
                for t in Conteudo::tipos() {
                    if ui.selectable_label(cel.conteudo.mesmo_tipo(&t), tr(t.label())).clicked() && !cel.conteudo.mesmo_tipo(&t) {
                        cel.conteudo = t;
                    }
                }
            }).response.on_hover_text(tr("Texto (fixo ou com campos ligados à peça), logotipo, tabela de revisões ou símbolo do diedro"));
        });
        egui::Grid::new("bloco_celula").num_columns(2).spacing([8., 6.]).show(ui, |ui| {
            ui.label(tr("Rótulo"));
            ui.text_edit_singleline(&mut cel.rotulo).on_hover_text(tr("Texto pequeno no canto de cima da caixa, como MATERIAL ou ESCALA; vazio = sem rótulo"));
            ui.end_row();
            match &mut cel.conteudo {
                Conteudo::Texto => {
                    ui.label(tr("Texto"));
                    ui.vertical(|ui| {
                        ui.add(egui::TextEdit::multiline(&mut cel.texto).desired_rows(2).desired_width(200.)).on_hover_text(tr("Texto fixo ou campos entre chaves, como {codigo} ou {cliente}: o valor vem da peça e do desenho e se atualiza sozinho"));
                        egui::ComboBox::from_id_salt("bloco_campo").selected_text(tr("Inserir campo…")).width(200.).show_ui(ui, |ui| {
                            for (k, d) in &campos {
                                if ui.selectable_label(false, format!("{k}  ·  {d}")).clicked() {
                                    cel.texto.push_str(k);
                                }
                            }
                        }).response.on_hover_text(tr("Acrescenta um campo ligado às propriedades da peça, do desenho ou da folha"));
                    });
                    ui.end_row();
                    ui.label(tr("Na folha"));
                    let v = fenkra_desenho::resolve(&cel.texto, &valores);
                    ui.label(RichText::new(if v.trim().is_empty() { tr("(vazio)").to_string() } else { v }).weak()).on_hover_text(tr("Como o texto sai na folha com os valores de agora"));
                    ui.end_row();
                    ui.label(tr("Altura"));
                    ui.add(crate::ui_ext::dv(&mut cel.altura).range(0.0..=20.).speed(0.1).suffix(" mm").dica(tr("Altura do texto; 0 usa o estilo de texto do desenho. Texto que não cabe diminui sozinho")));
                    ui.end_row();
                    ui.label(tr("Estilo"));
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut cel.negrito, tr("Negrito")).on_hover_text(tr("Texto em negrito, como no título"));
                        egui::ComboBox::from_id_salt("bloco_alinhar").selected_text(tr(cel.alinhar.label())).show_ui(ui, |ui| {
                            for a in Alinhamento::TODOS {
                                ui.selectable_value(&mut cel.alinhar, a, tr(a.label()));
                            }
                        }).response.on_hover_text(tr("Alinhamento do texto na caixa"));
                    });
                    ui.end_row();
                }
                Conteudo::Imagem { caminho } => {
                    ui.label(tr("Arquivo"));
                    ui.horizontal(|ui| {
                        ui.add(egui::TextEdit::singleline(caminho).desired_width(150.)).on_hover_text(tr("Imagem PNG ou JPEG do logotipo; o fundo branco ou transparente não aparece"));
                        if ui.button(tr("Escolher…")).clicked() {
                            escolher_imagem = true;
                        }
                    });
                    ui.end_row();
                }
                Conteudo::Revisoes { linhas } => {
                    ui.label(tr("Linhas"));
                    ui.add(crate::ui_ext::dv(linhas).range(1..=12).dica(tr("Quantas revisões a tabela mostra; as mais novas ficam embaixo")));
                    ui.end_row();
                    revisoes_ui = true;
                }
                Conteudo::Diedro => {
                    ui.label("");
                    ui.label(RichText::new(tr("O símbolo segue o método de projeção do desenho (1º ou 3º diedro).")).weak().small());
                    ui.end_row();
                }
            }
        });
    }
    if escolher_imagem {
        if let Some(p) = rfd::FileDialog::new().add_filter(tr("Imagem"), &["png", "jpg", "jpeg"]).pick_file() {
            let caminho = st.rel(&p);
            if fenkra_desenho::logo::carregar(&p).is_none() {
                st.bloco.aviso = Some((tr("A imagem não abriu (use PNG ou JPEG)").to_string(), true));
            }
            if let Some(Conteudo::Imagem { caminho: c }) = m.bloco.celula_mut(&sel).map(|c| &mut c.conteudo) {
                *c = caminho;
            }
        }
    }
    // o modelo mudou: grava no desenho e as folhas que o usam se refazem
    if m != original {
        st.d.atualizar_modelo(m.clone());
    }
    if revisoes_ui {
        revisoes(ui, st);
    }
    ui.separator();
    propriedades(ui, st);
    ui.separator();
    // ---------- folhas e biblioteca
    let usam = st.d.sheets.iter().filter(|s| s.modelo.as_deref() == Some(uid.as_str())).count();
    ui.label(RichText::new(format!("{}: {usam} / {}", tr("Folhas que usam este modelo"), st.d.sheets.len())).weak());
    ui.horizontal_wrapped(|ui| {
        if ui.add_enabled(usam < st.d.sheets.len(), egui::Button::new(tr("Usar em todas as folhas"))).on_hover_text(tr("Todas as folhas do desenho passam a usar este modelo")).clicked() {
            for s in &mut st.d.sheets {
                s.modelo = Some(uid.clone());
            }
            st.d.sincronizar_folhas();
        }
        if ui.button(tr("Voltar à legenda pronta")).on_hover_text(tr("Esta folha deixa o modelo e volta à legenda da norma do desenho")).clicked() {
            let s = st.sheet;
            st.d.tirar_modelo(s);
            concluir(st);
        }
    });
    if st.bloco.editando.is_none() {
        return true;
    }
    ui.label(RichText::new(tr("Biblioteca")).strong());
    ui.horizontal_wrapped(|ui| {
        if ui.button(tr("Gravar na biblioteca")).on_hover_text(tr("Guarda o modelo para escolher em outros desenhos (Novo desenho e Modelo de folha)")).clicked() {
            st.bloco.aviso = Some(match mf::pasta_biblioteca() {
                Some(dir) => {
                    let p = dir.join(mf::nome_de_arquivo(&m.nome));
                    match std::fs::create_dir_all(&dir).map_err(|e| e.to_string()).and_then(|_| m.gravar(&p)) {
                        Ok(()) => (format!("{}: {}", tr("Modelo gravado"), p.display()), false),
                        Err(e) => (format!("{}: {e}", tr("Não gravou o modelo")), true),
                    }
                }
                None => (tr("Sem pasta de biblioteca neste computador").to_string(), true),
            });
        }
        if ui.button(tr("Gravar como…")).on_hover_text(tr("Grava o modelo num arquivo .fkfolha")).clicked() {
            if let Some(p) = rfd::FileDialog::new().add_filter(tr("Modelo de folha"), &[mf::EXTENSAO]).set_file_name(mf::nome_de_arquivo(&m.nome)).save_file() {
                st.bloco.aviso = Some(match m.gravar(&p) {
                    Ok(()) => (format!("{}: {}", tr("Modelo gravado"), p.display()), false),
                    Err(e) => (format!("{}: {e}", tr("Não gravou o modelo")), true),
                });
            }
        }
        if ui.button(tr("Abrir modelo…")).on_hover_text(tr("Lê um modelo .fkfolha e o aplica a esta folha")).clicked() {
            let dir = mf::pasta_biblioteca();
            let mut dlg = rfd::FileDialog::new().add_filter(tr("Modelo de folha"), &[mf::EXTENSAO]);
            if let Some(d) = dir.filter(|d| d.exists()) {
                dlg = dlg.set_directory(d);
            }
            if let Some(p) = dlg.pick_file() {
                match ModeloFolha::ler(&p) {
                    Ok(novo) => {
                        let s = st.sheet;
                        let u = novo.uid.clone();
                        st.d.aplicar_modelo(s, novo);
                        st.bloco.editando = Some(u);
                        st.bloco.sel = vec![];
                        st.bloco.aviso = Some((tr("Modelo aplicado à folha").to_string(), false));
                    }
                    Err(e) => st.bloco.aviso = Some((format!("{}: {e}", tr("Não abriu o modelo")), true)),
                }
            }
        }
    });
    ui.separator();
    ui.horizontal(|ui| {
        if ui.button(tr("✔ Concluir")).on_hover_text(tr("Fecha o editor mantendo o bloco (Esc)")).clicked() {
            concluir(st);
        }
        if ui.button(tr("✖ Cancelar")).on_hover_text(tr("Fecha o editor e desfaz tudo o que foi mudado nele")).clicked() {
            cancelar(st);
        }
    });
    true
}

/// Tabela de revisões do desenho (o conteúdo da caixa "Tabela de revisões").
fn revisoes(ui: &mut egui::Ui, st: &mut DwgState) {
    ui.label(RichText::new(tr("Revisões do desenho")).strong());
    let mut tirar = None;
    egui::Grid::new("bloco_revisoes").num_columns(5).spacing([4., 4.]).show(ui, |ui| {
        for t in ["Rev", "Descrição", "Data", "Autor", ""] {
            ui.label(RichText::new(tr(t)).small());
        }
        ui.end_row();
        for (i, r) in st.d.revisoes.iter_mut().enumerate() {
            ui.add(egui::TextEdit::singleline(&mut r.rev).desired_width(28.));
            ui.add(egui::TextEdit::singleline(&mut r.descricao).desired_width(110.));
            ui.add(egui::TextEdit::singleline(&mut r.data).desired_width(70.));
            ui.add(egui::TextEdit::singleline(&mut r.autor).desired_width(60.));
            if ui.small_button("🗑").on_hover_text(tr("Apaga a revisão")).clicked() {
                tirar = Some(i);
            }
            ui.end_row();
        }
    });
    if let Some(i) = tirar {
        st.d.revisoes.remove(i);
    }
    if ui.button(tr("+ Revisão")).on_hover_text(tr("Acrescenta a próxima letra com a data de hoje; {revisao} passa a valer a letra nova")).clicked() {
        let prox = st.d.revisoes.last().and_then(|r| r.rev.chars().next()).map(|c| ((c as u8).saturating_add(1) as char).to_string()).unwrap_or_else(|| "A".into());
        st.d.revisoes.push(mf::Revisao { rev: prox, descricao: String::new(), data: today(), autor: std::env::var("USERNAME").unwrap_or_default() });
    }
}

/// Propriedades do próprio desenho: completam os campos que a peça não tem.
fn propriedades(ui: &mut egui::Ui, st: &mut DwgState) {
    egui::CollapsingHeader::new(tr("Propriedades do desenho")).default_open(!st.d.propriedades.is_empty()).show(ui, |ui| {
        ui.label(RichText::new(tr("Valem como campos {chave}; quando a peça tem a mesma propriedade, vale a da peça.")).weak().small());
        let mut tirar = None;
        egui::Grid::new("bloco_props").num_columns(3).spacing([4., 4.]).show(ui, |ui| {
            for (i, (k, v)) in st.d.propriedades.iter_mut().enumerate() {
                ui.add(egui::TextEdit::singleline(k).desired_width(90.)).on_hover_text(tr("Nome da propriedade; no texto use entre chaves, em minúsculas"));
                ui.add(egui::TextEdit::singleline(v).desired_width(130.));
                if ui.small_button("🗑").clicked() {
                    tirar = Some(i);
                }
                ui.end_row();
            }
        });
        if let Some(i) = tirar {
            st.d.propriedades.remove(i);
        }
        if ui.button(tr("+ Propriedade")).on_hover_text(tr("Acrescenta uma propriedade do desenho para usar como campo")).clicked() {
            st.d.propriedades.push((tr("cliente").to_string(), String::new()));
        }
    });
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn abrir_cria_o_modelo_da_folha_concluir_mantem_e_cancelar_desfaz() {
        let mut st = DwgState::new(Drawing::new("D", Format::A3), None);
        abrir(&mut st);
        let uid = st.bloco.editando.clone().expect("editor aberto");
        assert_eq!(st.d.modelos.len(), 1);
        assert_eq!(st.d.sheets[0].modelo.as_deref(), Some(uid.as_str()));
        assert!(!st.bloco.sel.is_empty(), "a primeira caixa já vem escolhida");
        assert!(st.em_comando() && st.active(), "o painel fica aberto só durante a edição");
        cancelar(&mut st);
        assert!(st.d.modelos.is_empty() && st.d.sheets[0].modelo.is_none(), "cancelar volta à legenda pronta");
        assert!(!st.active());
        abrir(&mut st);
        concluir(&mut st);
        assert_eq!(st.d.modelos.len(), 1);
        let uid = st.d.modelos[0].uid.clone();
        // abrir de novo edita o mesmo modelo, não cria outro
        abrir(&mut st);
        assert_eq!(st.bloco.editando.as_deref(), Some(uid.as_str()));
        assert_eq!(st.d.modelos.len(), 1);
        assert!(esc(&mut st) && st.bloco.editando.is_none());
    }

    /// Um quadro da folha com o editor aberto: zoom 2 px/mm e a folha no canto da tela.
    fn quadro(ctx: &egui::Context, st: &mut DwgState, eventos: Vec<egui::Event>) {
        let tela = Rect::from_min_size(Pos2::ZERO, Vec2::new(1000., 700.));
        let raw = egui::RawInput { screen_rect: Some(tela), events: eventos, ..Default::default() };
        let h = st.d.sheet(st.sheet).unwrap().size()[1] as f32;
        let mut saida = ctx.run_ui(raw, |ui| {
            let rect = ui.max_rect();
            let resp = ui.allocate_rect(rect, egui::Sense::click_and_drag());
            let painter = ui.painter_at(rect);
            let z = 2.;
            let to_scr = |p: V2| Pos2::new(p[0] as f32 * z, (h - p[1] as f32) * z);
            let to_mm = |s: Pos2| [(s.x / z) as f64, (h - s.y / z) as f64];
            let mouse = resp.hover_pos().map(to_mm);
            tratar(ui, &painter, st, &resp, mouse, &to_scr, &to_mm, z, rect);
        });
        saida.textures_delta.clear();
    }

    fn botao(pos: Pos2, pressed: bool) -> egui::Event {
        egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() }
    }

    #[test]
    fn arrastar_a_divisoria_na_folha_muda_a_caixa_e_vira_um_passo_de_desfazer() {
        let mut st = DwgState::new(Drawing::new("D", Format::A3), None);
        abrir(&mut st);
        let h = 297.;
        let px = |p: V2| Pos2::new(p[0] as f32 * 2., (h - p[1]) as f32 * 2.);
        // bloco de partida: 180 × 36 no canto (230, 10); a primeira divisória horizontal fica a 12 mm
        // do topo, em y = 34
        let uid = st.bloco.editando.clone().unwrap();
        assert_eq!(st.d.modelo(&uid).unwrap().bloco.pos_da_divisoria(&[]), Some(12.));
        let ctx = egui::Context::default();
        let a = px([300., 34.]);
        let b = px([300., 28.]);
        quadro(&ctx, &mut st, vec![egui::Event::PointerMoved(a)]);
        quadro(&ctx, &mut st, vec![botao(a, true)]);
        quadro(&ctx, &mut st, vec![egui::Event::PointerMoved(px([300., 31.]))]);
        quadro(&ctx, &mut st, vec![egui::Event::PointerMoved(b)]);
        quadro(&ctx, &mut st, vec![botao(b, false)]);
        quadro(&ctx, &mut st, vec![]);
        assert_eq!(st.d.modelo(&uid).unwrap().bloco.pos_da_divisoria(&[]), Some(18.), "a divisória foi para y = 28 (18 mm do topo)");
        assert_eq!(st.undo.len(), 1, "o arraste inteiro é um passo de desfazer");
        assert!(st.bloco.arraste.is_none());
        // um clique numa caixa a escolhe
        let c = px([400., 15.]);
        quadro(&ctx, &mut st, vec![egui::Event::PointerMoved(c)]);
        quadro(&ctx, &mut st, vec![botao(c, true)]);
        quadro(&ctx, &mut st, vec![botao(c, false)]);
        quadro(&ctx, &mut st, vec![]);
        let m = st.d.modelo(&uid).unwrap();
        let esperada = m.bloco.celula_em(m.origem_do_bloco([420., 297.]), [400., 15.]).unwrap();
        assert_eq!(st.bloco.sel, esperada);
        assert_eq!(m.bloco.celula(&esperada).unwrap().texto, "{folha}/{folhas}");
    }

    #[test]
    fn painel_do_editor_desenha_sem_travar_e_textos_tem_traducao() {
        let mut st = DwgState::new(Drawing::new("D", Format::A3), None);
        abrir(&mut st);
        let ctx = egui::Context::default();
        for _ in 0..3 {
            let raw = egui::RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800., 900.))), ..Default::default() };
            let mut saida = ctx.run_ui(raw, |ui| {
                assert!(painel(ui, &mut st));
            });
            saida.textures_delta.clear();
        }
        // todo texto do editor passado a `tr("…")` tem tradução
        let fonte = include_str!("bloco_ui.rs");
        let fonte = &fonte[..fonte.find("#[cfg(test)]").unwrap()];
        crate::i18n::set_english(true);
        let mut faltam = vec![];
        let mut resto = fonte;
        while let Some(i) = resto.find("tr(\"") {
            let depois = &resto[i + 4..];
            let fim = depois.find('"').unwrap();
            let t = &depois[..fim];
            if !crate::i18n::tem_traducao(t) {
                faltam.push(t.to_string());
            }
            resto = &depois[fim..];
        }
        let dicas = ["Margem esquerda (mm): a de arquivamento, costuma ser maior", "Margem direita (mm)", "Margem de cima (mm)", "Margem de baixo (mm)"];
        for t in Conteudo::tipos().iter().map(|c| c.label()).chain(Alinhamento::TODOS.iter().map(|a| a.label())).chain(FIELDS.iter().map(|f| f.1)).chain(dicas) {
            if !crate::i18n::tem_traducao(t) {
                faltam.push(t.to_string());
            }
        }
        crate::i18n::set_english(false);
        assert!(faltam.is_empty(), "sem tradução: {faltam:?}");
    }
}
