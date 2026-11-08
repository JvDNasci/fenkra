// SPDX-License-Identifier: LGPL-2.1-or-later
//! Abrir `.fkp`, `.fka` e `.fkd` com duplo clique (B389), sem instalador.
//!
//! Como o `.fkv` (`assoc_fkv.rs`): o registro vai para `HKCU\Software\Classes` (só do usuário atual,
//! sem administrador), só quando a pessoa clica em Configurações, e desfazer apaga exatamente as chaves
//! que criamos, e só as que ainda apontam para as nossas classes. O programa recebe o arquivo como
//! primeiro argumento (`main.rs` o abre).

use std::path::Path;

const BASE: &str = r"HKCU\Software\Classes";

/// Extensão, nome da classe e descrição de cada documento.
pub const TIPOS: [(&str, &str, &str); 3] = [
    (".fkp", "Fenkra.Peca", "Peça do Fenkra"),
    (".fka", "Fenkra.Montagem", "Montagem do Fenkra"),
    (".fkd", "Fenkra.Desenho", "Desenho do Fenkra"),
];

/// Os argumentos de `reg.exe` que registram os três tipos para o executável `exe`, na ordem.
pub fn comandos_de_registro(exe: &Path) -> Vec<Vec<String>> {
    let s = |x: &str| x.to_string();
    let comando = format!("\"{}\" \"%1\"", exe.display());
    let mut v = vec![];
    for (ext, classe, descricao) in TIPOS {
        v.push(vec![s("add"), format!(r"{BASE}\{ext}"), s("/ve"), s("/d"), s(classe), s("/f")]);
        v.push(vec![s("add"), format!(r"{BASE}\{classe}"), s("/ve"), s("/d"), s(descricao), s("/f")]);
        v.push(vec![s("add"), format!(r"{BASE}\{classe}\shell\open\command"), s("/ve"), s("/d"), comando.clone(), s("/f")]);
    }
    v
}

/// Os que desfazem: a extensão e a classe de cada tipo.
pub fn comandos_de_remocao() -> Vec<Vec<String>> {
    TIPOS.iter().flat_map(|(ext, classe, _)| [format!(r"{BASE}\{ext}"), format!(r"{BASE}\{classe}")]).map(|k| vec!["delete".to_string(), k, "/f".to_string()]).collect()
}

#[cfg(windows)]
fn reg(args: &[String]) -> Result<String, String> {
    use std::os::windows::process::CommandExt;
    let o = std::process::Command::new("reg").args(args).creation_flags(0x0800_0000).output().map_err(|e| e.to_string())?;
    if o.status.success() {
        Ok(String::from_utf8_lossy(&o.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&o.stderr).trim().to_string())
    }
}

/// Os três tipos já abrem por este programa (a extensão aponta para a nossa classe).
#[cfg(windows)]
pub fn registrado() -> bool {
    TIPOS.iter().all(|(ext, classe, _)| reg(&["query".into(), format!(r"{BASE}\{ext}"), "/ve".into()]).is_ok_and(|s| s.contains(classe)))
}

#[cfg(not(windows))]
pub fn registrado() -> bool {
    false
}

#[cfg(windows)]
pub fn registrar() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    for c in comandos_de_registro(&exe) {
        if let Err(e) = reg(&c) {
            let _ = desfazer();
            return Err(e);
        }
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn registrar() -> Result<(), String> {
    Err("A associação de arquivos sem instalador existe só no Windows.".into())
}

/// Desfaz o registro. Só apaga a extensão se ela ainda aponta para a nossa classe.
#[cfg(windows)]
pub fn desfazer() -> Result<(), String> {
    let mut erro = None;
    for (ext, classe, _) in TIPOS {
        let nossa = reg(&["query".into(), format!(r"{BASE}\{ext}"), "/ve".into()]).is_ok_and(|s| s.contains(classe));
        if nossa {
            if let Err(e) = reg(&["delete".into(), format!(r"{BASE}\{ext}"), "/f".into()]) {
                erro = Some(e);
            }
        }
        // a classe é só nossa: se não existir, o reg avisa e não é erro
        let _ = reg(&["delete".into(), format!(r"{BASE}\{classe}"), "/f".into()]);
    }
    erro.map_or(Ok(()), Err)
}

#[cfg(not(windows))]
pub fn desfazer() -> Result<(), String> {
    Err("A associação de arquivos sem instalador existe só no Windows.".into())
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn registra_tres_tipos_so_em_hkcu_e_abre_pelo_programa_com_o_arquivo() {
        let r = comandos_de_registro(Path::new(r"C:\Fenkra\fenkra.exe"));
        assert_eq!(r.len(), 9);
        assert!(r.iter().all(|c| c[0] == "add" && c[1].starts_with(r"HKCU\Software\Classes\")), "nada fora de HKCU");
        // o comando de abrir passa o arquivo como primeiro argumento, sem opção na frente
        let abrir: Vec<&Vec<String>> = r.iter().filter(|c| c[1].ends_with(r"shell\open\command")).collect();
        assert_eq!(abrir.len(), 3);
        assert!(abrir.iter().all(|c| c[4] == r#""C:\Fenkra\fenkra.exe" "%1""#));
        for ext in [".fkp", ".fka", ".fkd"] {
            assert!(r.iter().any(|c| c[1].ends_with(&format!(r"Classes\{ext}"))), "{ext}");
        }
    }

    #[test]
    fn remocao_apaga_so_as_chaves_que_criamos() {
        let r = comandos_de_remocao();
        assert_eq!(r.len(), 6);
        assert!(r.iter().all(|c| c[0] == "delete" && c[1].starts_with(r"HKCU\Software\Classes\") && c[2] == "/f"));
        // nunca uma chave mais alta que a extensão ou a classe
        assert!(r.iter().all(|c| c[1].matches('\\').count() == 3));
    }
}
