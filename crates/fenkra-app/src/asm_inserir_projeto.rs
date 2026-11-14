// SPDX-License-Identifier: LGPL-2.1-or-later
//! Peça de fora do projeto na montagem (B445).
//!
//! Com um projeto aberto, a montagem usa documentos do projeto: são eles que têm PN, versões e as
//! exceções do perfil de qualidade. Antes, a peça de outra pasta era avaliada sem exceção possível e
//! ficava barrada. Agora pegar uma peça de fora não a põe na mão: abre um aviso com duas saídas,
//! "Copiar para o projeto" (o padrão, com o foco) e "Cancelar". A cópia é feita por
//! [`fenkra_project::Project::copiar_para_projeto`], que leva junto as peças de que ela depende, e é a
//! cópia que vai para a mão. O original fica onde estava.

use crate::app::App;
use crate::asm_inserir::{self, Estado};
use crate::asm_ui::AsmState;
use crate::i18n::tr;
use eframe::egui::{self, RichText};
use fenkra_project::copia_projeto::CopiaParaProjeto;
use fenkra_project::Project;
use std::path::{Path, PathBuf};

/// A peça precisa passar pelo aviso? Só com projeto aberto e só peça: o subconjunto não passa pelo
/// perfil de qualidade, e as peças dele seguem as regras de quando ele foi montado.
pub fn precisa_copiar(projeto: Option<&Project>, arquivo: &Path, sub: bool) -> bool {
    !sub && projeto.is_some_and(|p| !p.pertence(arquivo))
}

/// Pega a peça ou, se ela é de fora do projeto, deixa o aviso pendente com a mão vazia.
pub fn pegar_ou_avisar(projeto: Option<&Project>, st: &mut AsmState, est: &mut Estado, arquivo: &Path) -> Result<(), String> {
    if precisa_copiar(projeto, arquivo, est.sub) {
        est.mao = None;
        est.bloqueada = None;
        est.externa = Some(arquivo.to_path_buf());
        return Ok(());
    }
    asm_inserir::pegar(st, est, arquivo)
}

/// "Copiar para o projeto": copia, põe a cópia na mão e devolve o arquivo novo e os avisos.
pub fn copiar_e_pegar(projeto: &mut Project, st: &mut AsmState, est: &mut Estado) -> Result<(PathBuf, Vec<String>), String> {
    let arquivo = est.externa.take().ok_or_else(|| tr("Nenhuma peça esperando a cópia").to_string())?;
    let r = projeto.copiar_para_projeto(&arquivo).map_err(|e| format!("{}: {e}", tr("A cópia para o projeto falhou")))?;
    asm_inserir::pegar(st, est, &r.arquivo)?;
    Ok((r.arquivo.clone(), avisos(projeto, &r)))
}

/// "Cancelar": nada é copiado nem entra; com mais arquivos do diálogo na fila, passa ao próximo.
pub fn cancelar(projeto: Option<&Project>, st: &mut AsmState, est: &mut Estado) -> Result<(), String> {
    est.externa = None;
    if est.fila.is_empty() {
        return Ok(());
    }
    let f = est.fila.remove(0);
    pegar_ou_avisar(projeto, st, est, &f)
}

fn nome_arquivo(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

/// O que a pessoa precisa saber depois da cópia: onde ela ficou, com que PN, e o que mudou de nome.
fn avisos(p: &Project, r: &CopiaParaProjeto) -> Vec<String> {
    let pn = p.entry(r.id).map(|e| e.pn.clone()).unwrap_or_default();
    let mut v = vec![format!("{} pecas/{} (PN {pn})", tr("Peça copiada para o projeto:"), nome_arquivo(&r.arquivo))];
    if r.ids.len() > 1 {
        v.push(format!("{} {}", r.ids.len() - 1, tr("peça(s) usada(s) por link também copiada(s)")));
    }
    for (de, para) in &r.renomeados {
        v.push(format!("{} {de}: {} {para}", tr("Já havia no projeto um arquivo"), tr("a cópia se chama")));
    }
    for (de, para) in &r.pn_trocados {
        v.push(format!("{} {de} {} {para}", tr("O PN"), tr("já era de outro documento; a cópia ficou com")));
    }
    for a in &r.ausentes {
        v.push(format!("{} {a}", tr("Referência sem arquivo, mantida como estava:")));
    }
    v
}

/// Janela do aviso, enquanto houver uma peça de fora esperando a decisão.
pub fn aviso(app: &mut App, ctx: &egui::Context) {
    let Some(arquivo) = app.asm.as_ref().and_then(|s| s.inserir.as_ref()).and_then(|e| e.externa.clone()) else { return };
    let Some(projeto) = app.project.as_ref() else {
        // o projeto fechou com o aviso aberto: sem projeto a peça entra como antes
        let st = app.asm.as_mut().expect("montagem aberta");
        let mut est = st.inserir.take().expect("comando aberto");
        est.externa = None;
        let erro = asm_inserir::pegar(st, &mut est, &arquivo).err();
        st.inserir = Some(est);
        if let Some(e) = erro {
            app.toast(e, true);
        }
        return;
    };
    let destino = projeto.destino_da_copia(&arquivo);
    let renomeia = nome_arquivo(&destino) != nome_arquivo(&arquivo);
    let nome_projeto = projeto.index.name.clone();
    let (mut copiar, mut desistir) = (false, false);
    egui::Modal::new(egui::Id::new("fenkra_peca_fora_do_projeto")).show(ctx, |ui| {
        ui.set_max_width(460.);
        ui.heading(tr("Peça de fora do projeto"));
        ui.label(format!("{} {} {nome_projeto}.", nome_arquivo(&arquivo), tr("não pertence ao projeto")));
        ui.label(RichText::new(arquivo.display().to_string()).small().weak());
        ui.add_space(4.);
        ui.label(tr("Para entrar na montagem, a peça é copiada para a pasta do projeto junto com as peças de que ela depende. A cópia ganha PN e versão e mantém todo o histórico de comandos; o arquivo original não muda."));
        if renomeia {
            ui.add_space(4.);
            ui.label(RichText::new(format!("{} {}", tr("Já existe um arquivo com este nome no projeto: a cópia vai se chamar"), nome_arquivo(&destino))).color(crate::theme::st_warn()));
        }
        ui.add_space(8.);
        ui.horizontal(|ui| {
            let b = ui.button(RichText::new(tr("Copiar para o projeto")).strong()).on_hover_text(tr("Copia a peça e as dependências para a pasta do projeto, registra cada uma com PN e versão e põe a cópia na mão."));
            // o padrão: Enter confirma
            if ui.memory(|m| m.focused().is_none()) {
                b.request_focus();
            }
            copiar = b.clicked();
            desistir = ui.button(tr("Cancelar")).on_hover_text(tr("A peça não entra na montagem e nada é copiado.")).clicked();
        });
    });
    if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
        desistir = true;
    }
    if !copiar && !desistir {
        return;
    }
    let (Some(st), Some(projeto)) = (app.asm.as_mut(), app.project.as_mut()) else { return };
    let Some(mut est) = st.inserir.take() else { return };
    let mut mensagens: Vec<(String, bool)> = vec![];
    let mut recente = None;
    if copiar {
        match copiar_e_pegar(projeto, st, &mut est) {
            Ok((novo, avisos)) => {
                recente = Some(novo);
                mensagens.extend(avisos.into_iter().map(|a| (a, false)));
            }
            Err(e) => mensagens.push((e, true)),
        }
    } else if let Err(e) = cancelar(Some(&*projeto), st, &mut est) {
        mensagens.push((e, true));
    }
    st.inserir = Some(est);
    if let Some(f) = recente {
        app.settings.add_recent_file(&f);
    }
    if !mensagens.is_empty() {
        let erro = mensagens.iter().any(|(_, e)| *e);
        let texto: Vec<String> = mensagens.into_iter().map(|(m, _)| m).collect();
        app.toast(texto.join("\n"), erro);
    }
}

/// `FENKRA_DEMO_ASM=inserir_fora`: projeto com uma `placa.fkp` e a mesma peça de uma pasta externa
/// esperando a decisão, para a captura do aviso com o nome que muda.
pub fn demo(app: &mut App) {
    let base = std::env::temp_dir().join(format!("fenkra-demo-fora-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let fora = base.join("biblioteca externa");
    if std::fs::create_dir_all(&fora).is_err() {
        return;
    }
    let mut placa = fenkra_solidos::example_document();
    placa.name = "Placa".into();
    let _ = std::fs::write(fora.join("placa.fkp"), placa.to_json());
    if app.project.is_none() {
        app.project = Project::create(&base.join("Projeto da demonstração"), "Projeto da demonstração").ok();
    }
    let Some(p) = app.project.as_mut() else { return };
    // o projeto já tem uma placa.fkp: a peça de fora vai entrar como placa_2.fkp
    let _ = p.copiar_para_projeto(&fora.join("placa.fkp"));
    let Some(st) = app.asm.as_mut() else { return };
    let Some(mut est) = st.inserir.take() else { return };
    let _ = pegar_ou_avisar(app.project.as_ref(), st, &mut est, &fora.join("placa.fkp"));
    st.inserir = Some(est);
}

#[cfg(test)]
mod tests {
    use super::*;
    use fenkra_montagem::{Assembly, NodeKind};

    fn pasta(nome: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("fenkra_fora_{nome}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).expect("pasta");
        d
    }

    /// Caixa envolvente dos corpos: (mínimo, máximo).
    fn caixa(r: &fenkra_solidos::RecomputeResult) -> ([f32; 3], [f32; 3]) {
        let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
        for b in &r.bodies {
            for p in b.mesh.positions.chunks_exact(3) {
                for k in 0..3 {
                    lo[k] = lo[k].min(p[k]);
                    hi[k] = hi[k].max(p[k]);
                }
            }
        }
        (lo, hi)
    }

    fn volume(r: &fenkra_solidos::RecomputeResult) -> f64 {
        r.bodies.iter().map(|b| b.mass.volume).sum()
    }

    fn pecas(st: &AsmState) -> usize {
        st.asm.nodes.iter().filter(|n| !matches!(n.kind, NodeKind::Group)).count()
    }

    /// Projeto montado em código, montagem numa pasta, peça numa pasta externa.
    fn cenario(nome: &str) -> (Project, AsmState, PathBuf, PathBuf) {
        let d = pasta(nome);
        let p = Project::create(&d.join("projeto"), "P").expect("projeto");
        let fora = d.join("fora");
        std::fs::create_dir_all(&fora).expect("fora");
        let mut placa = fenkra_solidos::example_document();
        placa.name = "Placa".into();
        std::fs::write(fora.join("placa.fkp"), placa.to_json()).expect("peça");
        let conj = d.join("montagem");
        std::fs::create_dir_all(&conj).expect("montagem");
        let st = AsmState::new(Assembly::new("t"), Some(conj.join("conjunto.fka")));
        (p, st, fora.join("placa.fkp"), d)
    }

    #[test]
    fn peca_de_fora_abre_o_aviso_e_a_copia_entra_na_montagem_igual_a_original() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (mut p, mut st, original, d) = cenario("copia");
        let mut est = Estado::novo(false, None);
        // pegar não põe nada na mão: o aviso fica pendente
        pegar_ou_avisar(Some(&p), &mut st, &mut est, &original).unwrap();
        assert!(est.mao.is_none());
        assert_eq!(est.externa.as_deref(), Some(original.as_path()));
        // sem projeto, a mesma peça entra direto, como antes
        let mut sem = Estado::novo(false, None);
        pegar_ou_avisar(None, &mut st, &mut sem, &original).unwrap();
        assert!(sem.mao.is_some() && sem.externa.is_none());
        // Copiar para o projeto: a cópia está na pasta do projeto, com PN e versão, e vai para a mão
        let (copia, avisos) = copiar_e_pegar(&mut p, &mut st, &mut est).unwrap();
        assert!(est.externa.is_none());
        assert_eq!(copia, std::path::absolute(p.root.join("pecas").join("placa.fkp")).unwrap());
        assert!(copia.is_file() && p.pertence(&copia));
        let id = p.id_do_arquivo(&copia).unwrap();
        assert!(!p.entry(id).unwrap().pn.is_empty() && p.entry(id).unwrap().versions.len() == 1);
        assert!(avisos[0].contains("placa.fkp"), "{avisos:?}");
        // a peça da mão agora é do projeto: o checador acha as exceções dela pelo uid
        assert_eq!(est.mao.as_ref().unwrap().uid, p.entry(id).unwrap().uid);
        assert!(p.index.documents.iter().any(|e| e.uid == est.mao.as_ref().unwrap().uid));
        // coloca e recalcula a montagem
        let comp = asm_inserir::colocar_copia(&mut st, &mut est, [0.; 3]).expect("entra");
        let NodeKind::Part { path } = &st.asm.node(comp).unwrap().kind else { panic!("peça") };
        let gravado = fenkra_montagem::load::full_path(st.dir().as_deref(), path);
        assert_eq!(std::fs::canonicalize(gravado).unwrap(), std::fs::canonicalize(&copia).unwrap(), "a montagem usa a cópia");
        st.refresh();
        assert!(st.errors.is_empty(), "{:?}", st.errors);
        assert_eq!(pecas(&st), 1);
        // volume e caixa da montagem batem com os da peça original
        let ok = |r: &fenkra_solidos::RecomputeResult| r.status.values().all(|s| matches!(s, fenkra_solidos::Status::Ok | fenkra_solidos::Status::Inactive));
        let (vol_o, caixa_o, n_o) = {
            let r = &st.cache.get(&original, false).unwrap().result;
            assert!(ok(r));
            (volume(r), caixa(r), r.status.len())
        };
        {
            let r = &st.cache.get(&copia, false).unwrap().result;
            assert!(ok(r), "a cópia recalcula ok");
            assert_eq!(r.status.len(), n_o, "o histórico de comandos vem inteiro");
        }
        let vista = st.view.as_ref().unwrap();
        assert!(vol_o > 0.);
        assert!((volume(vista) - vol_o).abs() < 1e-6 * vol_o, "{} x {vol_o}", volume(vista));
        let a = caixa(vista);
        for k in 0..3 {
            assert!((a.0[k] - caixa_o.0[k]).abs() < 1e-3 && (a.1[k] - caixa_o.1[k]).abs() < 1e-3, "caixa {a:?} x {caixa_o:?}");
        }
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn nome_repetido_no_projeto_vira_placa_2_e_o_aviso_diz_antes_e_depois() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (mut p, mut st, original, d) = cenario("colisao");
        let primeira = p.copiar_para_projeto(&original).unwrap().arquivo;
        let conteudo = std::fs::read_to_string(&primeira).unwrap();
        // a mesma peça de fora outra vez: ainda não é do projeto (a cópia tem outro uid)
        assert!(precisa_copiar(Some(&p), &original, false));
        assert_eq!(nome_arquivo(&p.destino_da_copia(&original)), "placa_2.fkp", "o aviso já mostra o nome novo");
        let mut est = Estado::novo(false, None);
        pegar_ou_avisar(Some(&p), &mut st, &mut est, &original).unwrap();
        let (copia, avisos) = copiar_e_pegar(&mut p, &mut st, &mut est).unwrap();
        assert_eq!(nome_arquivo(&copia), "placa_2.fkp");
        assert!(avisos.iter().any(|a| a.contains("placa.fkp") && a.contains("placa_2.fkp")), "{avisos:?}");
        assert_eq!(std::fs::read_to_string(&primeira).unwrap(), conteudo, "a placa que já estava no projeto fica intacta");
        asm_inserir::colocar_copia(&mut st, &mut est, [0.; 3]).expect("entra");
        st.refresh();
        assert!(st.errors.is_empty(), "{:?}", st.errors);
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn cancelar_nao_copia_nada_e_passa_ao_proximo_da_fila() {
        let (p, mut st, original, d) = cenario("cancelar");
        let mut est = Estado::novo(false, None);
        est.fila = vec![original.clone()];
        pegar_ou_avisar(Some(&p), &mut st, &mut est, &original).unwrap();
        cancelar(Some(&p), &mut st, &mut est).unwrap();
        assert!(est.mao.is_none() && est.fila.is_empty());
        assert_eq!(est.externa.as_deref(), Some(original.as_path()), "o próximo da fila também é de fora: novo aviso");
        cancelar(Some(&p), &mut st, &mut est).unwrap();
        assert!(est.externa.is_none() && est.mao.is_none());
        assert!(p.index.documents.is_empty(), "nada entrou no projeto");
        assert_eq!(std::fs::read_dir(p.root.join("pecas")).unwrap().count(), 0);
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn peca_do_projeto_entra_sem_aviso() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (mut p, mut st, _, d) = cenario("do_projeto");
        let id = p.add_document(fenkra_solidos::example_document()).unwrap();
        let arq = p.root.join(&p.entry(id).unwrap().file);
        let mut est = Estado::novo(false, None);
        pegar_ou_avisar(Some(&p), &mut st, &mut est, &arq).unwrap();
        assert!(est.externa.is_none() && est.mao.is_some());
        let _ = std::fs::remove_dir_all(d);
    }
}
