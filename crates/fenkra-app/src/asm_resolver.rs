// SPDX-License-Identifier: LGPL-2.1-or-later
//! B66: ao abrir uma montagem, cada componente é conferido por uid + caminho. Arquivo movido com o
//! mesmo uid segue sozinho; uid diferente, vários candidatos ou arquivo sumido vão para a janela
//! Resolver: outro documento nunca entra no lugar em silêncio.

use crate::asm_ui::AsmState;
use crate::i18n::tr;
use eframe::egui;
use fenkra_core::Id;
use fenkra_montagem::{Assembly, NodeKind};
use fenkra_project::referencias::{caminho_relativo, ler_uid, resolver, Ref, Resolucao};
use std::path::{Path, PathBuf};

/// Componente que precisa de uma decisão da pessoa.
pub struct Pendencia {
    pub comp: Id,
    pub nome: String,
    pub caminho: String,
    pub resolucao: Resolucao,
}

/// Caminho do arquivo do componente, se ele aponta para um.
fn caminho_mut(k: &mut NodeKind) -> Option<&mut String> {
    match k {
        NodeKind::Part { path } | NodeKind::SubAssembly { path } => Some(path),
        NodeKind::Group => None,
    }
}

/// Confere os componentes. Devolve as pendências e quantos seguiram sozinhos para o lugar novo.
pub fn conferir(a: &mut Assembly, dir: &Path, busca: &[PathBuf]) -> (Vec<Pendencia>, usize) {
    let (mut pendencias, mut movidos) = (vec![], 0);
    for n in &mut a.nodes {
        let (comp, nome, uid) = (n.id, n.name.clone(), n.doc_uid.clone());
        let Some(caminho) = caminho_mut(&mut n.kind) else { continue };
        match resolver(dir, &Ref { uid, caminho: caminho.clone() }, busca) {
            Resolucao::Encontrado(p) => {
                if n.doc_uid.is_empty() {
                    n.doc_uid = ler_uid(&p);
                }
            }
            Resolucao::Movido(p) => {
                *caminho = caminho_relativo(dir, &p);
                movidos += 1;
            }
            resolucao => pendencias.push(Pendencia { comp, nome, caminho: caminho.clone(), resolucao }),
        }
    }
    (pendencias, movidos)
}

/// A pessoa escolheu o arquivo: o componente passa a apontar para ele e assume o uid dele.
pub fn escolher(a: &mut Assembly, dir: &Path, comp: Id, arquivo: &Path) {
    let Some(n) = a.node_mut(comp) else { return };
    n.doc_uid = ler_uid(arquivo);
    if let Some(c) = caminho_mut(&mut n.kind) {
        *c = caminho_relativo(dir, arquivo);
    }
}

/// Arquivos que a janela oferece para a pendência.
fn candidatos(r: &Resolucao) -> Vec<PathBuf> {
    match r {
        Resolucao::Varios(v) => v.clone(),
        Resolucao::UidDiferente { caminho, alternativa, .. } => alternativa.iter().cloned().chain([caminho.clone()]).collect(),
        _ => vec![],
    }
}

fn motivo(r: &Resolucao) -> &'static str {
    match r {
        Resolucao::Varios(_) => "Vários arquivos têm a identidade deste componente.",
        Resolucao::UidDiferente { .. } => "O arquivo neste caminho é outro documento.",
        _ => "O arquivo não foi encontrado.",
    }
}

/// Janela Resolver: uma linha por pendência, e nada muda sem clique.
pub fn janela(st: &mut AsmState, ctx: &egui::Context) -> bool {
    if st.referencias.is_empty() {
        return false;
    }
    let Some(dir) = st.dir() else { return false };
    let mut escolha: Option<(Id, PathBuf)> = None;
    let mut ignorar = None;
    egui::Window::new(tr("Resolver referências")).id(egui::Id::new("asm_resolver")).anchor(egui::Align2::CENTER_TOP, [0., 130.]).default_width(460.).show(ctx, |ui| {
        ui.label(tr("Estes componentes não abriram como foram salvos. Escolha o arquivo certo de cada um."));
        for p in &st.referencias {
            ui.separator();
            ui.label(egui::RichText::new(&p.nome).strong());
            ui.label(egui::RichText::new(format!("{} · {}", p.caminho, tr(motivo(&p.resolucao)))).small());
            ui.horizontal_wrapped(|ui| {
                for c in candidatos(&p.resolucao) {
                    let rotulo = caminho_relativo(&dir, &c);
                    if ui.button(format!("{} {rotulo}", tr("Usar"))).clicked() {
                        escolha = Some((p.comp, c));
                    }
                }
                if ui.button(tr("Procurar…")).clicked() {
                    if let Some(f) = rfd::FileDialog::new().add_filter("Fenkra", &["fkp", "fka"]).pick_file() {
                        escolha = Some((p.comp, f));
                    }
                }
                if ui.button(tr("Deixar como está")).clicked() {
                    ignorar = Some(p.comp);
                }
            });
        }
    });
    let mudou = escolha.is_some();
    if let Some((comp, f)) = escolha {
        st.push_undo();
        escolher(&mut st.asm, &dir, comp, &f);
        st.referencias.retain(|p| p.comp != comp);
    }
    if let Some(comp) = ignorar {
        st.referencias.retain(|p| p.comp != comp);
    }
    mudou
}

/// `FENKRA_DEMO_ASM=resolver`: um componente com dois candidatos e um arquivo sumido.
pub fn demo(st: &mut AsmState) {
    let ids: Vec<(Id, String)> = st.asm.nodes.iter().map(|n| (n.id, n.name.clone())).collect();
    let Some(dir) = st.dir() else { return };
    if let Some((comp, nome)) = ids.first().cloned() {
        st.referencias.push(Pendencia { comp, nome, caminho: "placa.fkp".into(), resolucao: Resolucao::Varios(vec![dir.join("velhas/placa.fkp"), dir.join("novas/placa.fkp")]) });
    }
    if let Some((comp, nome)) = ids.get(1).cloned() {
        st.referencias.push(Pendencia { comp, nome, caminho: "../base/tampa.fkp".into(), resolucao: Resolucao::NaoEncontrado });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fenkra_core::Document;
    use fenkra_montagem::Placement;

    fn pasta(nome: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("fenkra-resolver-{nome}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn peca(arquivo: &Path) -> String {
        let d = Document::new("p");
        std::fs::write(arquivo, d.to_json()).unwrap();
        d.uid.0.clone()
    }

    fn montagem(caminho: &str) -> (Assembly, Id) {
        let mut a = Assembly::new("m");
        let id = a.add_component("p", NodeKind::Part { path: caminho.into() }, Placement::default(), None);
        (a, id)
    }

    #[test]
    fn primeiro_encontro_grava_o_uid_e_arquivo_movido_segue_sozinho() {
        let d = pasta("movido");
        let uid = peca(&d.join("p.fkp"));
        let (mut a, id) = montagem("p.fkp");
        assert!(conferir(&mut a, &d, &[d.clone()]).0.is_empty());
        assert_eq!(a.node(id).unwrap().doc_uid, uid);
        std::fs::create_dir_all(d.join("sub")).unwrap();
        std::fs::rename(d.join("p.fkp"), d.join("sub/p.fkp")).unwrap();
        let (pend, movidos) = conferir(&mut a, &d, &[d.clone()]);
        assert!(pend.is_empty() && movidos == 1);
        assert!(matches!(&a.node(id).unwrap().kind, NodeKind::Part { path } if path == "sub/p.fkp"));
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn outro_documento_no_caminho_vira_pendencia_e_nao_troca_em_silencio() {
        let d = pasta("outro");
        let uid = peca(&d.join("p.fkp"));
        let (mut a, id) = montagem("p.fkp");
        conferir(&mut a, &d, &[d.clone()]);
        let novo = peca(&d.join("p.fkp"));
        assert_ne!(uid, novo);
        let (pend, _) = conferir(&mut a, &d, &[d.clone()]);
        assert!(matches!(pend[0].resolucao, Resolucao::UidDiferente { .. }));
        assert_eq!(a.node(id).unwrap().doc_uid, uid, "sem decisão, o uid não muda");
        escolher(&mut a, &d, id, &d.join("p.fkp"));
        assert_eq!(a.node(id).unwrap().doc_uid, novo);
        assert!(conferir(&mut a, &d, &[d.clone()]).0.is_empty());
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn montagem_guarda_uid_e_arquivo_antigo_ganha_um() {
        let a = Assembly::new("m");
        assert!(!a.uid.esta_vazio());
        let b = Assembly::from_json(&a.to_json()).unwrap();
        assert_eq!(a.uid.0, b.uid.0);
        let velho = a.to_json().replace(&format!("\"uid\":\"{}\"", a.uid.0), "\"uid\":\"\"");
        assert!(!Assembly::from_json(&velho).unwrap().uid.esta_vazio());
    }
}
