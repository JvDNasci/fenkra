// SPDX-License-Identifier: LGPL-2.1-or-later
//! Janela "Mudanças e liberação" (B141): pedidos e avisos de mudança (ECR/ECN) gravados no projeto e o
//! pacote de liberação com STEP, PDF, DXF e lista de peças em CSV, JSON e XML.
//!
//! O registro mora em `fenkra_project::mudancas` e o pacote em `fenkra_pacote::liberacao`; aqui fica só
//! a tela. A geração do pacote recalcula as peças, então roda numa linha de execução própria com uma
//! cópia do projeto aberta do disco.

use super::*;
use fenkra_pacote::liberacao::{pacote_liberacao, OpcoesLiberacao, ResumoLiberacao};
use fenkra_project::mudancas::EstadoPedido;
use std::sync::Mutex;

#[derive(Default)]
pub(super) struct LiberacaoUi {
    aba: usize,
    titulo: String,
    motivo: String,
    escolhidos: Vec<u64>,
    justificativa: String,
    descricao: String,
    opcoes: Option<OpcoesLiberacao>,
    destino: String,
    ultimo: Option<Result<ResumoLiberacao, String>>,
}

static ESTADO: Mutex<Option<LiberacaoUi>> = Mutex::new(None);
static PRONTO: Mutex<Option<Result<ResumoLiberacao, String>>> = Mutex::new(None);
static GERANDO: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn data(t: u64) -> String {
    let dias = (t / 86_400) as i64;
    // dia civil a partir da época (algoritmo de Howard Hinnant)
    let z = dias + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{d:02}/{m:02}/{y}")
}

impl App {
    pub(super) fn abrir_liberacao(&mut self, aba: usize) {
        let mut g = ESTADO.lock().unwrap_or_else(|e| e.into_inner());
        let st = g.get_or_insert_with(LiberacaoUi::default);
        st.aba = aba;
        if st.destino.is_empty() {
            if let Some(p) = &self.project {
                st.destino = p.root.join("liberacao").join(format!("{}-liberacao.zip", p.index.name.replace([' ', '/', '\\'], "_"))).display().to_string();
            }
        }
    }

    /// Desenvolvimento (`FENKRA_DEMO_PROJETO=liberacao`): projeto com um pedido aprovado e um aviso,
    /// a janela aberta; `FENKRA_DEMO_LIBERACAO_ABA=1` mostra a aba do pacote já gerado.
    pub(super) fn demo_liberacao(&mut self) {
        self.demo_projeto(false);
        let Some(p) = self.project.as_mut() else { return };
        let Some(id) = p.index.documents.first().map(|d| d.id) else { return };
        let r = (|| -> fenkra_project::PResult<()> {
            let n = p.abrir_pedido_mudanca("Placa mais espessa", "a placa empena na solda", &[id])?;
            p.decidir_pedido_mudanca(&n, true, "aprovado pela engenharia")?;
            p.release(id, "espessura 14")?;
            p.emitir_aviso_mudanca(&n, "espessura de 10 para 14 mm")?;
            p.abrir_pedido_mudanca("Furo oblongo", "ajuste na montagem", &[id])?;
            Ok(())
        })();
        if let Err(e) = r {
            return self.toast(e.to_string(), true);
        }
        let aba = std::env::var("FENKRA_DEMO_LIBERACAO_ABA").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
        self.abrir_liberacao(aba);
        if aba == 1 {
            let p = self.project.as_ref().unwrap();
            let destino = std::env::temp_dir().join(format!("fenkra-demo-liberacao-{}.zip", std::process::id()));
            let o = OpcoesLiberacao { aviso: Some("ECN-0001".into()), ..Default::default() };
            let r = pacote_liberacao(p, &destino, &o);
            if let Some(st) = ESTADO.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
                st.opcoes = Some(o);
                st.destino = destino.display().to_string();
                st.ultimo = Some(r);
            }
        }
    }

    pub(super) fn janela_liberacao(&mut self, ctx: &egui::Context) {
        let mut g = ESTADO.lock().unwrap_or_else(|e| e.into_inner());
        let Some(st) = g.as_mut() else { return };
        let Some(p) = self.project.as_mut() else {
            *g = None;
            return;
        };
        if let Some(r) = PRONTO.lock().unwrap_or_else(|e| e.into_inner()).take() {
            st.ultimo = Some(r);
        }
        let gerando = GERANDO.load(std::sync::atomic::Ordering::SeqCst);
        if gerando {
            ctx.request_repaint_after(std::time::Duration::from_millis(250));
        }
        let registro = p.mudancas();
        let mut aberta = true;
        let mut erro: Option<String> = None;
        let mut gerar = false;
        egui::Window::new(tr("Mudanças e liberação")).id(egui::Id::new("liberacao_b141")).open(&mut aberta).default_size([620., 520.]).resizable(true).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut st.aba, 0, tr("Pedidos e avisos de mudança"));
                ui.selectable_value(&mut st.aba, 1, tr("Pacote de liberação"));
            });
            ui.separator();
            let reg = match &registro {
                Ok(r) => r,
                Err(e) => {
                    ui.label(RichText::new(format!("{}: {e}", tr("O registro de mudanças não pôde ser lido"))).color(ui.visuals().error_fg_color));
                    return;
                }
            };
            if st.aba == 0 {
                egui::ScrollArea::vertical().max_height(260.).id_salt("lib_pedidos").show(ui, |ui| {
                    if reg.pedidos.is_empty() {
                        ui.label(RichText::new(tr("Nenhum pedido de mudança.")).weak());
                    }
                    for ped in reg.pedidos.iter().rev() {
                        let cor = match ped.estado {
                            EstadoPedido::Aberto => ui.visuals().warn_fg_color,
                            EstadoPedido::Aprovado => ui.visuals().selection.bg_fill,
                            EstadoPedido::Rejeitado => ui.visuals().weak_text_color(),
                            EstadoPedido::Implementado => ui.visuals().hyperlink_color,
                        };
                        egui::CollapsingHeader::new(RichText::new(format!("{} · {} · {}", ped.numero, ped.titulo, tr(ped.estado.rotulo()))).color(cor)).id_salt(&ped.numero).show(ui, |ui| {
                            ui.label(format!("{}: {}", tr("Motivo"), ped.motivo));
                            ui.label(RichText::new(format!("{} · {}", ped.autor, data(ped.data))).small().weak());
                            for d in &ped.documentos {
                                let rev = if d.revisao_antes.is_empty() { "-".to_string() } else { d.revisao_antes.clone() };
                                ui.label(RichText::new(format!("{} {} ({} {rev})", d.pn, d.nome, tr("Rev"))).monospace().small());
                            }
                            if !ped.decisao.is_empty() {
                                ui.label(RichText::new(format!("{} {}: {}", tr("Decidido por"), ped.decidido_por, ped.decisao)).small());
                            }
                            if let Some(a) = ped.aviso.as_ref().and_then(|n| reg.aviso(n)) {
                                ui.label(RichText::new(format!("{} · {}", a.numero, a.descricao)).strong());
                                for i in &a.itens {
                                    ui.label(RichText::new(format!("{}: {} → {}", i.pn, if i.revisao_antes.is_empty() { "-" } else { &i.revisao_antes }, i.revisao_depois)).monospace().small());
                                }
                            }
                            match ped.estado {
                                EstadoPedido::Aberto => {
                                    ui.add(egui::TextEdit::singleline(&mut st.justificativa).hint_text(tr("Justificativa da decisão")).desired_width(f32::INFINITY));
                                    ui.horizontal(|ui| {
                                        if ui.button(tr("Aprovar")).clicked() {
                                            if let Err(e) = p.decidir_pedido_mudanca(&ped.numero, true, &st.justificativa) {
                                                erro = Some(e.to_string());
                                            } else {
                                                st.justificativa.clear();
                                            }
                                        }
                                        if ui.button(tr("Rejeitar")).clicked() {
                                            if let Err(e) = p.decidir_pedido_mudanca(&ped.numero, false, &st.justificativa) {
                                                erro = Some(e.to_string());
                                            } else {
                                                st.justificativa.clear();
                                            }
                                        }
                                    });
                                }
                                EstadoPedido::Aprovado => {
                                    ui.add(egui::TextEdit::singleline(&mut st.descricao).hint_text(tr("O que mudou (vai no aviso)")).desired_width(f32::INFINITY));
                                    if ui.button(tr("Emitir aviso de mudança")).on_hover_text(tr("Registra as revisões de antes e depois de cada documento. Só sai quando todos já ganharam revisão nova.")).clicked() {
                                        match p.emitir_aviso_mudanca(&ped.numero, &st.descricao) {
                                            Ok(_) => st.descricao.clear(),
                                            Err(e) => erro = Some(e.to_string()),
                                        }
                                    }
                                }
                                _ => {}
                            }
                        });
                    }
                });
                ui.separator();
                ui.label(RichText::new(tr("Novo pedido de mudança")).strong());
                ui.add(egui::TextEdit::singleline(&mut st.titulo).hint_text(tr("Título")).desired_width(f32::INFINITY));
                ui.add(egui::TextEdit::multiline(&mut st.motivo).hint_text(tr("Motivo: o problema e o que se propõe")).desired_rows(2).desired_width(f32::INFINITY));
                ui.label(RichText::new(tr("Documentos afetados")).small());
                egui::ScrollArea::vertical().max_height(110.).id_salt("lib_docs").show(ui, |ui| {
                    for d in &p.index.documents {
                        let mut marcado = st.escolhidos.contains(&d.id);
                        if ui.checkbox(&mut marcado, format!("{} {}", d.pn, d.name)).changed() {
                            if marcado {
                                st.escolhidos.push(d.id);
                            } else {
                                st.escolhidos.retain(|x| *x != d.id);
                            }
                        }
                    }
                });
                if ui.button(tr("Abrir pedido")).clicked() {
                    match p.abrir_pedido_mudanca(&st.titulo, &st.motivo, &st.escolhidos) {
                        Ok(_) => {
                            st.titulo.clear();
                            st.motivo.clear();
                            st.escolhidos.clear();
                        }
                        Err(e) => erro = Some(e.to_string()),
                    }
                }
            } else {
                let o = st.opcoes.get_or_insert_with(OpcoesLiberacao::default);
                ui.horizontal_wrapped(|ui| {
                    ui.checkbox(&mut o.step, "STEP");
                    ui.checkbox(&mut o.pdf, "PDF");
                    ui.checkbox(&mut o.dxf, "DXF");
                    ui.separator();
                    ui.label(tr("Lista de peças"));
                    ui.checkbox(&mut o.bom_csv, "CSV");
                    ui.checkbox(&mut o.bom_json, "JSON");
                    ui.checkbox(&mut o.bom_xml, "XML");
                });
                ui.checkbox(&mut o.so_liberadas, tr("Só peças liberadas, na revisão")).on_hover_text(tr("Desligado, entram todas as peças na versão de trabalho (pacote para conferência, não para fabricar)."));
                let rotulo = o.aviso.clone().unwrap_or_else(|| tr("Nenhum: pelas regras acima").to_string());
                egui::ComboBox::from_label(tr("Aviso de mudança")).selected_text(rotulo).show_ui(ui, |ui| {
                    ui.selectable_value(&mut o.aviso, None, tr("Nenhum: pelas regras acima"));
                    for a in &reg.avisos {
                        ui.selectable_value(&mut o.aviso, Some(a.numero.clone()), format!("{} · {}", a.numero, a.descricao));
                    }
                });
                ui.horizontal(|ui| {
                    ui.label(tr("Destino"));
                    ui.add(egui::TextEdit::singleline(&mut st.destino).desired_width(380.));
                    if ui.button(tr("Escolher…")).clicked() {
                        if let Some(c) = rfd::FileDialog::new().add_filter("ZIP", &["zip"]).set_file_name("liberacao.zip").save_file() {
                            st.destino = c.display().to_string();
                        }
                    }
                });
                if gerando {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(tr("Gerando o pacote… as peças são recalculadas."));
                    });
                } else if ui.button(tr("Gerar pacote de liberação")).clicked() {
                    gerar = true;
                }
                match &st.ultimo {
                    Some(Ok(r)) => {
                        ui.separator();
                        ui.label(RichText::new(format!("{} · {} {} · {} KB", tr("Pacote gravado"), r.arquivos.len(), tr("arquivos"), r.bytes / 1024)).strong());
                        egui::ScrollArea::vertical().max_height(200.).id_salt("lib_lista").show(ui, |ui| {
                            for (n, b) in &r.arquivos {
                                ui.label(RichText::new(format!("{n}  ({} KB)", b.div_ceil(1024))).monospace().small());
                            }
                        });
                        for a in &r.avisos {
                            ui.label(RichText::new(format!("⚠ {a}")).color(ui.visuals().warn_fg_color).small());
                        }
                    }
                    Some(Err(e)) => {
                        ui.label(RichText::new(e).color(ui.visuals().error_fg_color));
                    }
                    None => {}
                }
            }
        });
        if gerar && !st.destino.trim().is_empty() {
            let (raiz, destino, o) = (p.root.clone(), PathBuf::from(st.destino.trim()), st.opcoes.clone().unwrap_or_default());
            GERANDO.store(true, std::sync::atomic::Ordering::SeqCst);
            std::thread::spawn(move || {
                let r = Project::open(&raiz).map_err(|e| e.to_string()).and_then(|p| pacote_liberacao(&p, &destino, &o));
                *PRONTO.lock().unwrap_or_else(|e| e.into_inner()) = Some(r);
                GERANDO.store(false, std::sync::atomic::Ordering::SeqCst);
            });
        }
        if !aberta {
            *g = None;
        }
        drop(g);
        if let Some(e) = erro {
            self.toast(e, true);
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn data_civil_da_epoca() {
        assert_eq!(super::data(0), "01/01/1970");
        assert_eq!(super::data(1_794_355_200), "11/11/2026");
    }
}
