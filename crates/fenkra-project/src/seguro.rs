// SPDX-License-Identifier: LGPL-2.1-or-later
//! Gravação segura, detecção de mudança externa, autorrecuperação e log de sessão (B65).
//!
//! Por que assim:
//! - `gravar_atomico` escreve num arquivo ao lado, força o disco (`sync_all`), guarda o conteúdo
//!   anterior em `.bak` e só então troca o nome. Uma queda de energia no meio deixa o arquivo antigo
//!   inteiro, nunca um arquivo pela metade.
//! - `Assinatura` (data de modificação + tamanho + hash) diz se outro programa mexeu no arquivo desde
//!   que o abrimos; o hash só é calculado quando data ou tamanho mudam.
//! - `Recuperacao` guarda cópias periódicas fora do arquivo do usuário, para oferecer na próxima
//!   abertura depois de uma falha. Nada aqui apaga o arquivo original.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

/// Caminho irmão com sufixo extra (`peca.fkp` -> `peca.fkp.bak`).
fn com_sufixo(p: &Path, sufixo: &str) -> PathBuf {
    let mut s = p.as_os_str().to_os_string();
    s.push(sufixo);
    PathBuf::from(s)
}

/// Caminho da cópia de segurança do arquivo (`.bak`).
pub fn caminho_bak(p: &Path) -> PathBuf {
    com_sufixo(p, ".bak")
}

/// Grava `dados` em `destino` sem nunca deixá-lo pela metade: temporário + `sync_all` + `.bak` do
/// conteúdo anterior + troca de nome. Cria a pasta se faltar.
pub fn gravar_atomico(destino: &Path, dados: &[u8]) -> std::io::Result<()> {
    if let Some(dir) = destino.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = com_sufixo(destino, ".tmp");
    let escrever = || -> std::io::Result<()> {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(dados)?;
        f.sync_all()
    };
    if let Err(e) = escrever() {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    if destino.exists() {
        // a cópia de segurança é melhor esforço: falhar nela não impede salvar
        let _ = std::fs::copy(destino, caminho_bak(destino));
    }
    if let Err(e) = renomear_com_tentativas(&tmp, destino) {
        let _ = std::fs::remove_file(&tmp);
        return Err(if bloqueio_temporario(&e) {
            std::io::Error::new(
                e.kind(),
                format!("o arquivo continuou bloqueado (em uso ou sem permissão) após várias tentativas ({e}); o conteúdo anterior foi preservado"),
            )
        } else {
            e
        });
    }
    Ok(())
}

/// Bloqueio breve do Windows: 32 (arquivo em uso), 33 (trecho travado) e 5 (acesso negado, que a
/// troca de nome devolve quando o destino está aberto por outro processo; num arquivo realmente
/// protegido o erro persiste e volta claro depois das tentativas). Antivírus, indexador e
/// outro processo abrem o arquivo por instantes e a troca de nome falha nesse intervalo.
pub(crate) fn bloqueio_temporario(e: &std::io::Error) -> bool {
    cfg!(windows) && matches!(e.raw_os_error(), Some(5) | Some(32) | Some(33))
}

/// Troca o nome tentando de novo só em bloqueio temporário (até 8 vezes, ~0,6 s). O temporário
/// segue intacto até a troca dar certo, então nenhum dado se perde; outros erros voltam na hora.
pub(crate) fn renomear_com_tentativas(de: &Path, para: &Path) -> std::io::Result<()> {
    let mut espera = 10u64;
    for _ in 0..8 {
        match std::fs::rename(de, para) {
            Err(e) if bloqueio_temporario(&e) => {
                std::thread::sleep(std::time::Duration::from_millis(espera));
                espera = (espera * 2).min(160);
            }
            r => return r,
        }
    }
    std::fs::rename(de, para)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Assinatura {
    pub mtime_ns: u128,
    pub tamanho: u64,
    pub hash: String,
}

/// Assina o arquivo como está agora (guardar ao abrir e ao salvar).
pub fn assinar(p: &Path) -> std::io::Result<Assinatura> {
    let meta = std::fs::metadata(p)?;
    let bytes = std::fs::read(p)?;
    Ok(Assinatura { mtime_ns: mtime(&meta), tamanho: meta.len(), hash: blake3::hash(&bytes).to_hex()[..16].to_string() })
}

fn mtime(m: &std::fs::Metadata) -> u128 {
    m.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_nanos()).unwrap_or(0)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MudancaExterna {
    /// o conteúdo é o mesmo que a assinatura guardada
    Igual,
    /// outro programa gravou conteúdo diferente: Sobrescrever, Salvar como ou Recarregar
    Mudou,
    /// o arquivo foi apagado ou movido
    Sumiu,
}

/// Compara o arquivo atual com a assinatura guardada. Só relê o conteúdo se data ou tamanho mudaram.
pub fn verificar(p: &Path, antes: &Assinatura) -> MudancaExterna {
    let Ok(meta) = std::fs::metadata(p) else { return MudancaExterna::Sumiu };
    if mtime(&meta) == antes.mtime_ns && meta.len() == antes.tamanho {
        return MudancaExterna::Igual;
    }
    match std::fs::read(p) {
        Ok(b) if blake3::hash(&b).to_hex()[..16] == antes.hash => MudancaExterna::Igual,
        Ok(_) => MudancaExterna::Mudou,
        Err(_) => MudancaExterna::Sumiu,
    }
}

/// Pasta de cópias de recuperação (autosave). Um arquivo `<nome>.rec` por documento aberto.
pub struct Recuperacao {
    pub dir: PathBuf,
}

#[derive(Clone, Debug)]
pub struct Pendente {
    pub nome: String,
    pub caminho: PathBuf,
    pub tamanho: u64,
}

impl Recuperacao {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Recuperacao { dir: dir.into() }
    }

    /// Nome de arquivo seguro: só letras, números, `-`, `_` e `.`.
    fn arquivo(&self, nome: &str) -> PathBuf {
        let limpo: String = nome.chars().map(|c| if c.is_alphanumeric() || matches!(c, '-' | '_' | '.') { c } else { '_' }).collect();
        self.dir.join(format!("{limpo}.rec"))
    }

    /// Grava (de forma atômica) a cópia de recuperação do documento `nome`.
    pub fn gravar(&self, nome: &str, dados: &[u8]) -> std::io::Result<()> {
        gravar_atomico(&self.arquivo(nome), dados)
    }

    pub fn ler(&self, nome: &str) -> std::io::Result<Vec<u8>> {
        std::fs::read(self.arquivo(nome))
    }

    /// Remove a cópia depois de um salvamento ou descarte confirmado.
    pub fn descartar(&self, nome: &str) {
        let a = self.arquivo(nome);
        let _ = std::fs::remove_file(com_sufixo(&a, ".bak"));
        let _ = std::fs::remove_file(a);
    }

    /// Cópias que sobraram de uma sessão que não fechou direito.
    pub fn pendentes(&self) -> Vec<Pendente> {
        let Ok(rd) = std::fs::read_dir(&self.dir) else { return vec![] };
        let mut v: Vec<Pendente> = rd
            .flatten()
            .filter(|e| e.path().extension().map(|x| x == "rec").unwrap_or(false))
            .map(|e| Pendente {
                nome: e.path().file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default(),
                tamanho: e.metadata().map(|m| m.len()).unwrap_or(0),
                caminho: e.path(),
            })
            .collect();
        v.sort_by(|a, b| a.nome.cmp(&b.nome));
        v
    }
}

/// Acrescenta uma linha com data (segundos desde 1970) ao log de sessão local.
pub fn log_sessao(arquivo: &Path, msg: &str) {
    if let Some(d) = arquivo.parent().filter(|d| !d.as_os_str().is_empty()) {
        let _ = std::fs::create_dir_all(d);
    }
    let t = std::time::SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(arquivo) {
        let _ = writeln!(f, "[{t}] {msg}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(nome: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("fenkra-seguro-{nome}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn gravacao_atomica_guarda_bak_e_nao_deixa_temporario() {
        let d = tmp("atomico");
        let f = d.join("peca.fkp");
        gravar_atomico(&f, b"um").unwrap();
        assert!(!caminho_bak(&f).exists(), "primeira gravação não tem anterior");
        gravar_atomico(&f, b"dois").unwrap();
        assert_eq!(std::fs::read(&f).unwrap(), b"dois");
        assert_eq!(std::fs::read(caminho_bak(&f)).unwrap(), b"um");
        assert!(!com_sufixo(&f, ".tmp").exists());
        let _ = std::fs::remove_dir_all(d);
    }

    #[cfg(windows)]
    #[test]
    fn bloqueio_breve_do_windows_e_vencido_com_tentativas() {
        let d = tmp("bloqueio");
        let f = d.join("peca.fkp");
        gravar_atomico(&f, b"original").unwrap();
        // outro processo segura o destino aberto sem compartilhar por ~150 ms
        use std::os::windows::fs::OpenOptionsExt;
        let seg = std::fs::OpenOptions::new().read(true).share_mode(0).open(&f).unwrap();
        let t = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(150));
            drop(seg);
        });
        gravar_atomico(&f, b"novo").unwrap();
        t.join().unwrap();
        assert_eq!(std::fs::read(&f).unwrap(), b"novo");
        let _ = std::fs::remove_dir_all(d);
    }

    #[cfg(windows)]
    #[test]
    fn bloqueio_persistente_da_erro_claro_e_preserva_o_original() {
        let d = tmp("persiste");
        let f = d.join("peca.fkp");
        gravar_atomico(&f, b"original").unwrap();
        use std::os::windows::fs::OpenOptionsExt;
        let _seg = std::fs::OpenOptions::new().read(true).share_mode(0).open(&f).unwrap();
        let e = gravar_atomico(&f, b"novo").unwrap_err();
        assert!(e.to_string().contains("continuou bloqueado"), "{e}");
        assert!(!com_sufixo(&f, ".tmp").exists());
        drop(_seg);
        assert_eq!(std::fs::read(&f).unwrap(), b"original");
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn falha_na_gravacao_preserva_o_arquivo_original() {
        let d = tmp("falha");
        let f = d.join("peca.fkp");
        gravar_atomico(&f, b"original").unwrap();
        // um diretório no lugar do temporário faz a escrita falhar antes da troca
        std::fs::create_dir(com_sufixo(&f, ".tmp")).unwrap();
        assert!(gravar_atomico(&f, b"novo").is_err());
        assert_eq!(std::fs::read(&f).unwrap(), b"original");
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn mudanca_externa_por_hash_ignora_so_a_data() {
        let d = tmp("externa");
        let f = d.join("a.fkp");
        std::fs::write(&f, b"abc").unwrap();
        let a = assinar(&f).unwrap();
        assert_eq!(verificar(&f, &a), MudancaExterna::Igual);
        // mesmo conteúdo regravado por fora: data muda, hash não
        std::fs::write(&f, b"abc").unwrap();
        assert_eq!(verificar(&f, &a), MudancaExterna::Igual);
        std::fs::write(&f, b"abcd").unwrap();
        assert_eq!(verificar(&f, &a), MudancaExterna::Mudou);
        std::fs::remove_file(&f).unwrap();
        assert_eq!(verificar(&f, &a), MudancaExterna::Sumiu);
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn recuperacao_lista_le_e_descarta() {
        let d = tmp("rec");
        let r = Recuperacao::new(d.join("recuperacao"));
        assert!(r.pendentes().is_empty());
        r.gravar("Minha peça/1", b"{}").unwrap();
        r.gravar("outra", b"[]").unwrap();
        let p = r.pendentes();
        assert_eq!(p.len(), 2);
        assert_eq!(r.ler("outra").unwrap(), b"[]");
        r.descartar("outra");
        assert_eq!(r.pendentes().len(), 1);
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn log_de_sessao_acrescenta_linhas() {
        let d = tmp("log");
        let l = d.join("logs").join("sessao.log");
        log_sessao(&l, "abriu");
        log_sessao(&l, "salvou");
        let t = std::fs::read_to_string(&l).unwrap();
        assert_eq!(t.lines().count(), 2);
        assert!(t.contains("abriu") && t.contains("salvou"));
        let _ = std::fs::remove_dir_all(d);
    }
}
