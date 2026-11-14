// SPDX-License-Identifier: LGPL-2.1-or-later
//! DWG pelo LibreDWG como programa externo (B211, ADR 0090).
//!
//! O LibreDWG é GPL-3.0. Ele não entra no executável nem no repositório: o Fenkra procura os programas
//! `dwg2dxf` e `dxf2dwg` já instalados, chama cada um como processo separado e troca só arquivos com ele.
//! A leitura converte o DWG para DXF e segue pelo leitor de DXF de sempre; a gravação grava o DXF e pede ao
//! `dxf2dwg` o DWG. Sem o programa, a mensagem diz onde obtê-lo e onde apontá-lo.
//!
//! Os caminhos passados ao programa são nomes curtos numa pasta temporária: o programa de linha de comando
//! pode não entender acento no caminho, e a pasta do usuário costuma ter.

use crate::ErroConv;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

/// Variável com a pasta dos programas do LibreDWG, ou o caminho de um deles. As Configurações gravam nela.
pub const VARIAVEL: &str = "FENKRA_LIBREDWG";

/// Tempo máximo de uma conversão: um DWG grande leva segundos, não minutos.
const LIMITE: Duration = Duration::from_secs(120);

/// Como obter o programa, para a mensagem de erro e para a página das Configurações.
pub const COMO_INSTALAR: &str = "o DWG precisa do LibreDWG, programa livre (GPL) instalado à parte. Baixe o pacote para Windows em https://ftp.gnu.org/gnu/libredwg/ (ou instale com `vcpkg install libredwg[tools]`), descompacte e aponte a pasta com dwg2dxf e dxf2dwg em Configurações > Programas externos. Sem ele, peça o arquivo em DXF a quem mandou o DWG";

/// O DWG abre com a versão em texto: `AC1015` (2000) até `AC1032` (2018), ou `AC1.2` e `AC2.10` nos antigos.
pub fn e_dwg(conteudo: &[u8]) -> bool {
    conteudo.len() >= 6 && conteudo.starts_with(b"AC") && conteudo[2..6].iter().all(|b| b.is_ascii_digit() || *b == b'.') && conteudo[2].is_ascii_digit()
}

/// Versão do DWG pelo cabeçalho, em nome de lançamento.
pub fn versao(conteudo: &[u8]) -> Option<&'static str> {
    if !e_dwg(conteudo) {
        return None;
    }
    Some(match &conteudo[..6] {
        b"AC1012" => "R13",
        b"AC1014" => "R14",
        b"AC1015" => "R2000",
        b"AC1018" => "R2004",
        b"AC1021" => "R2007",
        b"AC1024" => "R2010",
        b"AC1027" => "R2013",
        b"AC1032" => "R2018",
        _ => "anterior ao R13",
    })
}

fn candidatos(pasta: &Path, nome: &str) -> impl Iterator<Item = PathBuf> {
    let pasta = pasta.to_path_buf();
    let nome = nome.to_string();
    ["exe", "cmd", "bat", ""].into_iter().map(move |ext| if ext.is_empty() { pasta.join(&nome) } else { pasta.join(format!("{nome}.{ext}")) })
}

/// Pastas onde o programa costuma estar, depois da apontada e do PATH.
fn pastas_conhecidas() -> Vec<PathBuf> {
    let mut v = Vec::new();
    for var in ["ProgramFiles", "ProgramFiles(x86)", "LOCALAPPDATA"] {
        if let Some(base) = std::env::var_os(var).map(PathBuf::from) {
            v.extend([base.join("LibreDWG"), base.join("LibreDWG").join("bin"), base.join("libredwg").join("bin")]);
        }
    }
    let vcpkg = std::env::var_os("VCPKG_ROOT").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(r"C:\dev\vcpkg"));
    for triplo in ["x64-windows", "x64-windows-release", "x64-windows-static"] {
        v.push(vcpkg.join("installed").join(triplo).join("tools").join("libredwg"));
    }
    v
}

/// Caminho do programa do LibreDWG (`dwg2dxf` ou `dxf2dwg`), se estiver instalado.
pub fn programa(nome: &str) -> Option<PathBuf> {
    let existe = |p: &PathBuf| p.is_file();
    if let Some(apontado) = std::env::var_os(VARIAVEL).map(PathBuf::from).filter(|p| !p.as_os_str().is_empty()) {
        let pasta = if apontado.is_file() { apontado.parent().map(Path::to_path_buf).unwrap_or_default() } else { apontado };
        // apontado e não achado: não cai no PATH, senão a pessoa não sabe qual programa rodou
        return candidatos(&pasta, nome).find(existe);
    }
    let caminho = std::env::var_os("PATH").map(|p| std::env::split_paths(&p).collect::<Vec<_>>()).unwrap_or_default();
    caminho.into_iter().chain(pastas_conhecidas()).find_map(|pasta| candidatos(&pasta, nome).find(existe))
}

/// Os dois programas estão ao alcance.
pub fn disponivel() -> bool {
    programa("dwg2dxf").is_some() && programa("dxf2dwg").is_some()
}

/// Pasta temporária só desta conversão, apagada ao sair.
struct Temporaria(PathBuf);

impl Temporaria {
    fn nova() -> Result<Temporaria, ErroConv> {
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(0);
        let p = std::env::temp_dir().join(format!("fenkra_dwg_{}_{}", std::process::id(), N.fetch_add(1, Ordering::Relaxed)));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).map_err(|e| ErroConv::Leitura(format!("pasta temporária do DWG: {e}")))?;
        Ok(Temporaria(p))
    }
}

impl Drop for Temporaria {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Roda o programa na pasta, com `-y -o saida entrada`, e espera até o limite.
fn rodar(nome: &str, pasta: &Path, saida: &str, entrada: &str) -> Result<(), ErroConv> {
    let exe = programa(nome).ok_or_else(|| ErroConv::Ausente(format!("{nome}: {COMO_INSTALAR}")))?;
    let mut cmd = Command::new(&exe);
    cmd.current_dir(pasta).args(["-y", "-o", saida, entrada]).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW: sem piscar um console
        cmd.creation_flags(0x0800_0000);
    }
    let mut filho = cmd.spawn().map_err(|e| ErroConv::Ausente(format!("{} não abriu ({e}): {COMO_INSTALAR}", exe.display())))?;
    let inicio = Instant::now();
    let estado = loop {
        if let Some(s) = filho.try_wait().map_err(|e| ErroConv::Leitura(e.to_string()))? {
            break s;
        }
        if inicio.elapsed() > LIMITE {
            let _ = filho.kill();
            let _ = filho.wait();
            return Err(ErroConv::Tempo(LIMITE.as_secs()));
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let mut erro = String::new();
    if let Some(mut e) = filho.stderr.take() {
        use std::io::Read;
        let _ = e.read_to_string(&mut erro);
    }
    // o LibreDWG devolve código diferente de zero até em aviso; quem manda é o arquivo de saída
    if !pasta.join(saida).is_file() {
        let detalhe = erro.lines().filter(|l| !l.trim().is_empty()).last().unwrap_or("sem mensagem").trim().to_string();
        return Err(ErroConv::Leitura(format!("{nome} não gerou o arquivo (código {}): {detalhe}", estado.code().unwrap_or(-1))));
    }
    Ok(())
}

/// Converte o DWG em DXF pelo `dwg2dxf` e devolve o texto do DXF.
pub fn para_dxf(dwg: &Path) -> Result<Vec<u8>, ErroConv> {
    let t = Temporaria::nova()?;
    std::fs::copy(dwg, t.0.join("entrada.dwg")).map_err(|e| ErroConv::Leitura(format!("não foi possível ler '{}': {e}", dwg.display())))?;
    rodar("dwg2dxf", &t.0, "saida.dxf", "entrada.dwg")?;
    std::fs::read(t.0.join("saida.dxf")).map_err(|e| ErroConv::Leitura(format!("DXF do dwg2dxf: {e}")))
}

/// Grava o DWG a partir do texto de um DXF, pelo `dxf2dwg`.
pub fn de_dxf(dxf: &str, destino: &Path) -> Result<(), ErroConv> {
    let t = Temporaria::nova()?;
    std::fs::write(t.0.join("entrada.dxf"), dxf).map_err(|e| ErroConv::Gravacao(format!("DXF temporário: {e}")))?;
    rodar("dxf2dwg", &t.0, "saida.dwg", "entrada.dxf").map_err(|e| match e {
        ErroConv::Leitura(m) => ErroConv::Gravacao(m),
        outro => outro,
    })?;
    std::fs::copy(t.0.join("saida.dwg"), destino).map(|_| ()).map_err(|e| ErroConv::Gravacao(format!("não foi possível gravar '{}': {e}", destino.display())))
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn cabecalho_do_dwg_diz_a_versao() {
        assert!(e_dwg(b"AC1032\0\0\0\0\0"));
        assert_eq!(versao(b"AC1015\0\0"), Some("R2000"));
        assert_eq!(versao(b"AC1.50\0\0"), Some("anterior ao R13"));
        assert!(!e_dwg(b"ACME 12"));
        assert!(!e_dwg(b"AC10"));
        assert!(!e_dwg(b"  0\nSECTION"));
    }
}
