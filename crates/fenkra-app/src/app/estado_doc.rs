// SPDX-License-Identifier: LGPL-2.1-or-later
//! Estado do documento na barra e botões de gestão do painel de projeto (B279). A regra de qual estado
//! vale fica em `fenkra_project::estado_doc`; aqui ficam só os fatos do que está aberto e o desenho.

use super::*;
use fenkra_project::estado_doc::{comparar, Diferencas, EstadoDoc, Fatos};

const LADO_ICONE: f32 = 16.;

/// Botão do painel: ícone do conjunto, texto e tooltip em dois estágios.
pub(super) fn botao(ui: &mut egui::Ui, ativo: bool, icone: &str, rotulo: &str, frase: &str, passos: &str) -> egui::Response {
    let imagem = egui::Image::new(crate::icons::get(icone)).fit_to_exact_size(egui::vec2(LADO_ICONE, LADO_ICONE));
    let r = ui.add_enabled(ativo, egui::Button::image_and_text(imagem, tr(rotulo)));
    crate::icons::hover_help(ui, r, tr(rotulo), tr(frase), tr(passos))
}

/// Cores do tema: estado bom no tom de link, aviso e erro nos tons próprios, o resto apagado.
fn cor(ui: &egui::Ui, e: &EstadoDoc) -> egui::Color32 {
    let v = ui.visuals();
    match e {
        EstadoDoc::Liberada(_) => v.hyperlink_color,
        EstadoDoc::Editado(_) => v.warn_fg_color,
        EstadoDoc::EmConflito => v.error_fg_color,
        EstadoDoc::SemRevisao | EstadoDoc::SomenteLeitura => v.weak_text_color(),
    }
}

/// Texto da barra, já traduzido.
fn texto(e: &EstadoDoc) -> String {
    tr(e.modelo()).replace("{0}", e.revisao())
}

/// Fatos fixos para capturar cada estado sem preparar um projeto: `FENKRA_DEMO_ESTADO`.
fn fatos_de_demonstracao(nome: &str) -> Fatos {
    let revisao = Some("B".to_string());
    match nome {
        "liberada" => Fatos { revisao, ..Default::default() },
        "editado" => Fatos { revisao, sujo_desde_revisao: true, ..Default::default() },
        "leitura" => Fatos { revisao, somente_leitura: true, ..Default::default() },
        "conflito" => Fatos { revisao, conflito: true, ..Default::default() },
        _ => Fatos::default(),
    }
}

impl App {
    fn fatos_do_documento(&self) -> Fatos {
        if let Ok(nome) = std::env::var("FENKRA_DEMO_ESTADO") {
            return fatos_de_demonstracao(&nome);
        }
        let peca = self.documento_ativo_e_peca();
        let entrada = self.open.entry.filter(|_| peca).and_then(|id| self.project.as_ref()?.entry(id).ok());
        let mut f = entrada.map(Fatos::da_entrada).unwrap_or_default();
        f.modificado = self.ativo_modificado();
        f.somente_leitura = peca && self.open.frozen.is_some();
        f.conflito = self.ativo_em_conflito();
        f
    }

    pub(super) fn estado_na_barra(&mut self, ui: &mut egui::Ui) {
        let e = EstadoDoc::de(&self.fatos_do_documento());
        let rotulo = texto(&e);
        let imagem = egui::Image::new(crate::icons::get(e.icone())).fit_to_exact_size(egui::vec2(LADO_ICONE, LADO_ICONE));
        let r = ui.add(egui::Button::image_and_text(imagem, RichText::new(&rotulo).small().color(cor(ui, &e))).frame(false));
        if crate::icons::hover_help(ui, r, &rotulo, tr(e.dica()), tr(e.passos())).clicked() {
            if e == EstadoDoc::EmConflito {
                self.salvar_ativo();
            } else {
                self.left = LeftTab::Projeto;
            }
        }
    }

    /// Projeto de exemplo, com uma revisão e uma edição depois dela, para capturar o painel.
    pub(super) fn demo_projeto(&mut self, comparar: bool) {
        let pasta = std::env::temp_dir().join(format!("fenkra-demo-projeto-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&pasta);
        let montar = || -> fenkra_project::PResult<(Project, u64)> {
            std::fs::create_dir_all(&pasta)?;
            let mut p = Project::create(&pasta, "Demonstração")?;
            let id = p.add_document(fenkra_solidos::example_document())?;
            p.release(id, "primeira")?;
            let mut d = p.load(id)?;
            if let Some(q) = d.params.iter_mut().find(|q| q.name == "esp") {
                q.expr = "14".into();
            }
            p.save(id, &d, "mais espessa")?;
            Ok((p, id))
        };
        let Ok((p, id)) = montar() else { return };
        self.project = Some(p);
        self.project_sel = Some(id);
        self.left = LeftTab::Projeto;
        self.started = true;
        self.open_entry(id, None);
        if comparar {
            self.comparar_com_revisao(id);
        }
    }

    /// Calcula o que mudou desde a última revisão de `id`.
    pub(super) fn comparar_com_revisao(&mut self, id: u64) {
        let Some(p) = self.project.as_ref() else { return };
        let Some(e) = p.entry(id).ok().filter(|e| !e.revisions.is_empty()) else { return };
        let (Some(hash), Some(rotulo)) = (e.current_revision().map(|r| r.hash.clone()), e.current_revision().map(|r| r.label.clone())) else { return };
        let agora = if self.open.entry == Some(id) { Ok(self.open.doc.clone()) } else { p.load(id) };
        match (p.load_version(&hash), agora) {
            (Ok(a), Ok(b)) => self.comparacao = Some((id, rotulo, comparar(&a, &b))),
            (Err(x), _) | (_, Err(x)) => self.toast(x.to_string(), true),
        }
    }

    /// Lista do que mudou, logo abaixo dos botões.
    pub(super) fn comparacao_ui(&mut self, ui: &mut egui::Ui, id: u64) {
        let Some((dono, rotulo, d)) = &self.comparacao else { return };
        if *dono != id {
            return;
        }
        ui.label(RichText::new(tr("Mudanças desde a Rev {0}").replace("{0}", rotulo)).strong());
        lista(ui, d);
        if ui.small_button(tr("Fechar")).clicked() {
            self.comparacao = None;
        }
    }
}

fn lista(ui: &mut egui::Ui, d: &Diferencas) {
    if d.vazia() {
        ui.label(RichText::new(tr("Nada mudou nos comandos e parâmetros.")).weak());
        return;
    }
    for (rotulo, nomes) in [("Novos", &d.novos), ("Removidos", &d.removidos), ("Alterados", &d.alterados)] {
        if !nomes.is_empty() {
            ui.label(format!("{}: {}", tr(rotulo), nomes.join(", ")));
        }
    }
    for (nome, antes, agora) in &d.parametros {
        ui.label(format!("{nome}: {antes} → {agora}"));
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use fenkra_project::estado_doc::FRASES;

    fn ingles(t: &str) -> Option<&'static str> {
        crate::textos_projeto::TEXTOS.iter().find(|(pt, _)| *pt == t).map(|(_, en)| *en)
    }

    /// Confere a tabela direto: trocar o idioma global aqui mexeria nos testes que rodam ao lado.
    #[test]
    fn estados_e_botoes_tem_portugues_e_ingles() {
        for t in FRASES.iter().chain(TEXTOS_DOS_BOTOES.iter().filter(|t| !JA_TRADUZIDOS.contains(t))) {
            assert!(ingles(t).is_some_and(|en| en != *t && !en.is_empty()), "sem tradução: {t}");
        }
        assert_eq!(ingles("Rev {0} liberada").unwrap().replace("{0}", "B"), "Rev B released");
        assert_eq!(ingles("Editado desde a Rev {0}").unwrap().replace("{0}", "B"), "Edited since Rev B");
        assert_eq!(texto(&EstadoDoc::Liberada("B".into())), "Rev B liberada");
        assert_eq!(texto(&EstadoDoc::Editado("B".into())), "Editado desde a Rev B");
    }

    #[test]
    fn demonstracao_cobre_os_cinco_estados() {
        let nomes = ["sem", "liberada", "editado", "leitura", "conflito"];
        let estados: Vec<EstadoDoc> = nomes.iter().map(|n| EstadoDoc::de(&fatos_de_demonstracao(n))).collect();
        for (i, e) in estados.iter().enumerate() {
            assert!(!estados[..i].contains(e), "{e:?} repetido");
        }
    }

    #[test]
    fn icones_dos_estados_vem_do_conjunto_novo() {
        for e in [EstadoDoc::SemRevisao, EstadoDoc::Liberada("A".into()), EstadoDoc::Editado("A".into()), EstadoDoc::SomenteLeitura, EstadoDoc::EmConflito] {
            assert!(crate::icons::tem_modelo(e.icone()), "{}", e.icone());
        }
        for i in ICONES_DOS_BOTOES {
            assert!(crate::icons::tem_modelo(i), "{i}");
        }
    }
}

/// Ícones dos botões do painel.
pub(super) const ICONES_DOS_BOTOES: [&str; 6] = ["salvar", "liberar_revisao", "importar_arquivo", "exportar_arquivo", "onde_usado", "comparar"];

/// Rótulos e dicas dos botões do painel: `(rótulo, frase, passos)`.
pub(super) const SALVAR_VERSAO: (&str, &str, &str) = ("Salvar versão", "Grava o estado atual no histórico do documento.", "Cada versão fica guardada e pode ser restaurada. Não muda a revisão liberada.");
pub(super) const LIBERAR: (&str, &str, &str) = ("Liberar revisão…", "Fecha uma revisão nova com a versão atual.", "Confira a lista antes de liberar. A revisão fica congelada e pode ser reaberta só para leitura.");
pub(super) const IMPORTAR: (&str, &str, &str) = ("Importar", "Traz um arquivo STEP, BREP ou árvore paramétrica para o projeto.", "Confira a miniatura e os avisos no painel e confirme. O original fica guardado no cofre do projeto.");
pub(super) const EXPORTAR: (&str, &str, &str) = ("Exportar", "Grava a peça aberta em STEP.", "Escolha o destino. Para levar o histórico, use Exportar árvore paramétrica no menu Arquivo.");
pub(super) const ONDE_USADO: (&str, &str, &str) = ("Onde é usado", "Mostra as montagens e os desenhos que usam este documento.", "Confira antes de mudar ou liberar: quem usa o documento recebe a mudança.");
pub(super) const COMPARAR: (&str, &str, &str) = ("Comparar", "Lista o que mudou desde a última revisão.", "Mostra comandos novos, removidos e alterados, e os parâmetros com valor diferente.");

/// Rótulos que a tabela geral já traduz.
#[cfg(test)]
const JA_TRADUZIDOS: [&str; 2] = ["Importar", "Onde é usado"];

#[cfg(test)]
const TEXTOS_DOS_BOTOES: [&str; 18] = [
    SALVAR_VERSAO.0, SALVAR_VERSAO.1, SALVAR_VERSAO.2, LIBERAR.0, LIBERAR.1, LIBERAR.2, IMPORTAR.0, IMPORTAR.1, IMPORTAR.2,
    EXPORTAR.0, EXPORTAR.1, EXPORTAR.2, ONDE_USADO.0, ONDE_USADO.1, ONDE_USADO.2, COMPARAR.0, COMPARAR.1, COMPARAR.2,
];
