// SPDX-License-Identifier: LGPL-2.1-or-later
//! Abrir o `.fkv` com duplo clique (B365), sem instalador.
//!
//! O `.fkv` é um pacote zip: nenhum navegador sabe abri-lo sozinho. Quem o abre é a página do
//! visualizador, e a página só lê o pacote que está dentro dela ou que a pessoa escolhe. Por isso a
//! associação não aponta para o navegador: aponta para este programa com `--abrir-fkv "%1"`, que lê o
//! pacote, grava uma página autocontida numa pasta temporária e a entrega ao navegador padrão. Não
//! sobe janela do Fenkra, servidor nem porta.
//!
//! O registro vai para `HKCU\Software\Classes` (só do usuário atual, sem administrador) e só quando a
//! pessoa clica. Desfazer apaga exatamente as duas chaves que criamos, e só se ainda forem nossas.

use fenkra_pacote::{pagina_html, Pacote};
use std::path::{Path, PathBuf};

/// Nome do tipo de arquivo no registro.
const CLASSE: &str = "Fenkra.Visualizacao";
const BASE: &str = r"HKCU\Software\Classes";

/// Os argumentos de `reg.exe` que registram o `.fkv` para o executável `exe`, na ordem.
pub fn comandos_de_registro(exe: &Path) -> Vec<Vec<String>> {
    let s = |x: &str| x.to_string();
    let comando = format!("\"{}\" --abrir-fkv \"%1\"", exe.display());
    vec![
        vec![s("add"), format!(r"{BASE}\.fkv"), s("/ve"), s("/d"), s(CLASSE), s("/f")],
        vec![s("add"), format!(r"{BASE}\{CLASSE}"), s("/ve"), s("/d"), s("Pacote de visualização do Fenkra"), s("/f")],
        vec![s("add"), format!(r"{BASE}\{CLASSE}\shell\open\command"), s("/ve"), s("/d"), comando, s("/f")],
    ]
}

/// Os que desfazem: as duas chaves que criamos.
pub fn comandos_de_remocao() -> Vec<Vec<String>> {
    [format!(r"{BASE}\.fkv"), format!(r"{BASE}\{CLASSE}")].into_iter().map(|k| vec!["delete".to_string(), k, "/f".to_string()]).collect()
}

#[cfg(windows)]
fn reg(args: &[String]) -> Result<String, String> {
    use std::os::windows::process::CommandExt;
    // CREATE_NO_WINDOW: sem piscar um console
    let o = std::process::Command::new("reg").args(args).creation_flags(0x0800_0000).output().map_err(|e| e.to_string())?;
    if o.status.success() {
        Ok(String::from_utf8_lossy(&o.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&o.stderr).trim().to_string())
    }
}

/// O `.fkv` já abre por este programa (a chave existe e aponta para a nossa classe).
#[cfg(windows)]
pub fn registrado() -> bool {
    reg(&["query".into(), format!(r"{BASE}\.fkv"), "/ve".into()]).is_ok_and(|s| s.contains(CLASSE))
}

#[cfg(not(windows))]
pub fn registrado() -> bool {
    false
}

/// Registra o `.fkv` para este executável.
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

/// Desfaz o registro. Só apaga o `.fkv` se ele ainda aponta para a nossa classe.
#[cfg(windows)]
pub fn desfazer() -> Result<(), String> {
    let mut erro = None;
    if registrado() {
        if let Err(e) = reg(&comandos_de_remocao()[0]) {
            erro = Some(e);
        }
    }
    // a classe é só nossa: se não existir, o reg avisa e não é erro
    let _ = reg(&comandos_de_remocao()[1]);
    erro.map_or(Ok(()), Err)
}

#[cfg(not(windows))]
pub fn desfazer() -> Result<(), String> {
    Err("A associação de arquivos sem instalador existe só no Windows.".into())
}

/// Lê o `.fkv`, grava a página autocontida na pasta temporária e devolve o caminho dela.
pub fn pagina_do_fkv(arquivo: &Path) -> Result<PathBuf, String> {
    let bytes = std::fs::read(arquivo).map_err(|e| format!("{}: {e}", arquivo.display()))?;
    Pacote::ler(&bytes).and_then(|p| p.validar()).map_err(|e| format!("pacote inválido: {e}"))?;
    let pasta = std::env::temp_dir().join("fenkra_visualizacao");
    std::fs::create_dir_all(&pasta).map_err(|e| e.to_string())?;
    let nome = arquivo.file_stem().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "pacote".into());
    let seguro: String = nome.chars().map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect();
    let destino = pasta.join(format!("{seguro}.html"));
    std::fs::write(&destino, pagina_html(Some(&bytes))).map_err(|e| e.to_string())?;
    Ok(destino)
}

/// `fenkra --abrir-fkv ARQUIVO`: abre o pacote no navegador e termina sem subir a interface.
/// Devolve `true` quando o pedido era esse (e já foi tratado).
pub fn tratar_argumentos(args: &[String]) -> bool {
    if let Some(i) = args.iter().position(|a| a == "--gerar-visualizador") {
        // usado por scripts/release.ps1: a página solta, sem pacote, que abre um .fkv por arrastar ou escolher
        let r = match args.get(i + 1) {
            Some(a) => std::fs::write(a, pagina_html(None)).map_err(|e| e.to_string()),
            None => Err("faltou o arquivo de saída".into()),
        };
        if let Err(e) = r {
            rfd::MessageDialog::new().set_level(rfd::MessageLevel::Error).set_title("Fenkra").set_description(format!("Não consegui gravar o visualizador.

{e}")).show();
            std::process::exit(1);
        }
        return true;
    }
    let Some(i) = args.iter().position(|a| a == "--abrir-fkv") else { return false };
    let r = match args.get(i + 1) {
        Some(a) => pagina_do_fkv(Path::new(a)).and_then(|p| crate::app::abrir_no_navegador(&p).map_err(|e| e.to_string())),
        None => Err("faltou o arquivo .fkv".into()),
    };
    if let Err(e) = r {
        // sem console no executável de release: a caixa de mensagem é o único jeito de a pessoa saber
        rfd::MessageDialog::new().set_level(rfd::MessageLevel::Error).set_title("Fenkra").set_description(format!("Não consegui abrir o pacote de visualização.\n\n{e}")).show();
    }
    true
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn registro_aponta_o_comando_para_o_executavel_com_o_argumento() {
        let c = comandos_de_registro(Path::new(r"C:\Fenkra\fenkra.exe"));
        let ultimo = c.last().unwrap();
        assert!(ultimo[1].ends_with(r"shell\open\command"));
        assert_eq!(ultimo[4], r#""C:\Fenkra\fenkra.exe" --abrir-fkv "%1""#);
        // tudo em HKCU: nada que peça administrador
        assert!(c.iter().all(|a| a[1].starts_with(r"HKCU\Software\Classes")));
        assert_eq!(c.len(), 3);
    }

    #[test]
    fn remocao_apaga_so_as_duas_chaves_nossas() {
        let r = comandos_de_remocao();
        assert_eq!(r.len(), 2);
        assert!(r.iter().all(|a| a[1].starts_with(r"HKCU\Software\Classes") && a[2] == "/f"));
        assert!(r[0][1].ends_with(r".fkv") && r[1][1].ends_with(CLASSE));
    }

    #[test]
    fn pagina_do_fkv_embute_o_pacote_e_recusa_lixo() {
        use fenkra_core::{BoolOp, Document, FeatureKind, OriginPlane, PlaneRef, PrimKind, Primitive};
        let s = |x: &str| x.to_string();
        let mut doc = Document::new("Placa b365");
        doc.add_feature(FeatureKind::Primitive(Primitive { kind: PrimKind::Caixa, plane: PlaneRef::Origin(OriginPlane::XY), a: s("40"), b: s("20"), c: s("5"), d: s("0"), op: BoolOp::Add }));
        let bytes = fenkra_pacote::de_peca(&doc, &Default::default()).unwrap().bytes().unwrap();
        let dir = std::env::temp_dir().join(format!("fenkra_assoc_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let fkv = dir.join("minha peça.fkv");
        std::fs::write(&fkv, &bytes).unwrap();
        let html = pagina_do_fkv(&fkv).unwrap();
        assert!(html.file_name().unwrap().to_string_lossy().ends_with(".html") && !html.file_name().unwrap().to_string_lossy().contains(' '));
        let texto = std::fs::read_to_string(&html).unwrap();
        assert!(texto.contains("fkv-dados") && !texto.contains("/*@PACOTE@*/"));
        let ruim = dir.join("lixo.fkv");
        std::fs::write(&ruim, b"nao e zip").unwrap();
        assert!(pagina_do_fkv(&ruim).is_err());
        assert!(pagina_do_fkv(&dir.join("nao_existe.fkv")).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
