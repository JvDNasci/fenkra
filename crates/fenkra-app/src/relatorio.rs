// SPDX-License-Identifier: LGPL-2.1-or-later
//! Relatório de falhas com consentimento (B309).
//!
//! Quando o programa tem um pânico, o gancho de `partida.rs` grava um relatório de texto, só no disco
//! local. Na próxima abertura o Fenkra mostra o conteúdo inteiro e pergunta o que fazer: copiar,
//! abrir a pasta ou descartar. Não existe envio automático: este módulo não abre conexão nenhuma, e o
//! relatório é montado só com versão, sistema, hardware, mensagem do erro e o final do registro de
//! partida. O modelo (documento, sketches, nomes de arquivo) não entra: `montar` nem recebe o
//! documento. Caminhos com o nome da pessoa são trocados antes de gravar.

use crate::perfil::Hardware;
use std::path::PathBuf;

/// Quantas linhas do final do registro de partida acompanham o relatório.
const LINHAS_DO_REGISTRO: usize = 40;

fn pasta() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join("Fenkra").join("relatorios"))
}

/// Caminho do relatório que espera a decisão da pessoa.
pub fn caminho_pendente() -> Option<PathBuf> {
    pasta().map(|p| p.join("pendente.txt"))
}

pub fn pasta_dos_relatorios() -> Option<PathBuf> {
    pasta()
}

/// Troca os pedaços que identificam a pessoa (pasta do usuário e nome da conta) por marcadores.
pub fn sanear(texto: &str) -> String {
    let mut t = texto.to_string();
    let mut nomes: Vec<String> = vec![];
    for var in ["USERPROFILE", "HOME", "APPDATA", "LOCALAPPDATA"] {
        if let Ok(v) = std::env::var(var) {
            if v.len() > 3 {
                nomes.push(v);
            }
        }
    }
    // o mais longo primeiro, para APPDATA não ficar com sobra do USERPROFILE
    nomes.sort_by_key(|n| std::cmp::Reverse(n.len()));
    for n in &nomes {
        t = t.replace(n.as_str(), "<usuario>");
        t = t.replace(&n.replace('\\', "/"), "<usuario>");
    }
    if let Ok(u) = std::env::var("USERNAME").or_else(|_| std::env::var("USER")) {
        if u.len() > 2 {
            t = t.replace(&u, "<conta>");
        }
    }
    t
}

/// Tira do registro o que pode citar arquivos da pessoa (argumentos da linha de comando).
fn limpar_linha_do_registro(l: &str) -> String {
    match (l.find(" | args "), l.find(" | WGPU_BACKEND")) {
        (Some(a), Some(b)) if a < b => format!("{}{}", &l[..a], &l[b..]),
        _ => l.to_string(),
    }
}

/// Monta o texto do relatório. Recebe só dados técnicos; nunca o documento.
pub fn montar(mensagem: &str, onde: &str, fio: &str, pilha: &str, hw: Option<&Hardware>, registro: &str) -> String {
    let mut t = String::new();
    t.push_str("Relatório de falha do Fenkra\n");
    t.push_str("(gerado no seu computador; nada foi enviado)\n\n");
    t.push_str(&format!("Versão: {}\n", env!("CARGO_PKG_VERSION")));
    t.push_str(&format!("Sistema: {} {}\n", std::env::consts::OS, std::env::consts::ARCH));
    match hw {
        Some(h) => t.push_str(&format!(
            "Hardware: processadores lógicos {}, memória {}, adaptador por software {}, OpenGL {}\n",
            h.logicos,
            h.ram_gb.map_or("desconhecida".to_string(), |g| format!("{g:.1} GB")),
            if h.software { "sim" } else { "não" },
            if h.opengl { "sim" } else { "não" },
        )),
        None => t.push_str("Hardware: ainda não medido (falha antes da janela)\n"),
    }
    t.push_str(&format!("\nErro: {mensagem}\nLocal: {onde} (fio {fio})\n\nPilha:\n{pilha}\n"));
    let linhas: Vec<&str> = registro.lines().collect();
    let ini = linhas.len().saturating_sub(LINHAS_DO_REGISTRO);
    t.push_str("\nFinal do registro de partida:\n");
    for l in &linhas[ini..] {
        t.push_str(&limpar_linha_do_registro(l));
        t.push('\n');
    }
    t.push_str("\nO modelo (peças, sketches e arquivos) não faz parte deste relatório.\n");
    sanear(&t)
}

/// Grava o relatório que espera decisão. Nunca falha para quem chama (roda dentro do gancho de pânico).
pub fn gravar_pendente(texto: &str) {
    let Some(p) = caminho_pendente() else { return };
    let _ = std::fs::create_dir_all(p.parent().unwrap_or(&p));
    let _ = std::fs::write(p, texto);
}

pub fn ler_pendente() -> Option<String> {
    std::fs::read_to_string(caminho_pendente()?).ok().filter(|t| !t.trim().is_empty())
}

/// Guarda o relatório com a data (para a pessoa poder anexá-lo depois) e limpa o pendente.
pub fn arquivar_pendente() {
    let Some(p) = caminho_pendente() else { return };
    let seg = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let _ = std::fs::rename(&p, p.with_file_name(format!("falha-{seg}.txt")));
}

/// Apaga o relatório pendente (a pessoa descartou).
pub fn descartar_pendente() {
    if let Some(p) = caminho_pendente() {
        let _ = std::fs::remove_file(p);
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn relatorio_traz_o_necessario_e_avisa_que_nada_foi_enviado() {
        let t = montar("índice fora do limite", "src/x.rs:10", "main", "pilha de exemplo", None, "[1] inicio: Fenkra\n[2] adaptador: GPU");
        assert!(t.contains("índice fora do limite") && t.contains("src/x.rs:10") && t.contains("pilha de exemplo"));
        assert!(t.contains("nada foi enviado"));
        assert!(t.contains("[2] adaptador: GPU"));
        assert!(t.contains("não faz parte deste relatório"));
    }

    #[test]
    fn hardware_medido_entra_no_texto() {
        let h = Hardware { software: true, opengl: false, ram_gb: Some(6.0), logicos: 4 };
        let t = montar("x", "y", "main", "", Some(&h), "");
        assert!(t.contains("memória 6.0 GB") && t.contains("por software sim"));
    }

    #[test]
    fn argumentos_da_linha_de_comando_nao_vao_no_relatorio() {
        let l = r#"[1] inicio: Fenkra 0.1 | args ["C:/Pessoal/cliente-secreto.fenkra"] | WGPU_BACKEND None"#;
        let t = montar("x", "y", "main", "", None, l);
        assert!(!t.contains("cliente-secreto"), "{t}");
        assert!(t.contains("WGPU_BACKEND None"));
    }

    #[test]
    fn pasta_do_usuario_vira_marcador() {
        let home = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")).unwrap_or_default();
        if home.len() > 3 {
            let t = sanear(&format!("falha em {home}/projetos/x.rs"));
            assert!(!t.contains(&home) && t.contains("<usuario>"), "{t}");
        }
    }

    #[test]
    fn montar_nao_recebe_documento() {
        // a assinatura só aceita texto técnico: o modelo não tem como entrar no relatório
        let _: fn(&str, &str, &str, &str, Option<&Hardware>, &str) -> String = montar;
    }
}
