// SPDX-License-Identifier: LGPL-2.1-or-later
//! A única porta de arquivo da automação: nada lê nem grava fora da pasta informada, e nada
//! sobrescreve sem pedido explícito.
//!
//! A checagem tem duas etapas. A do texto recusa o que nunca é legítimo (`..`, unidade, fluxo
//! alternativo `a.step:x`, nome de dispositivo do Windows). A do disco resolve ligações simbólicas e
//! junções pelo caminho canônico, porque um nome limpo pode apontar para fora.

use crate::ErroApi;
use std::fs::{self, OpenOptions};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const DISPOSITIVOS: [&str; 22] = [
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

static SEQUENCIA: AtomicU64 = AtomicU64::new(0);

/// Raiz do projeto. `raiz` é o caminho comum, que o kernel entende; `canonica` só serve para comparar
/// (no Windows ela vem com o prefixo `\\?\`).
#[derive(Clone, Debug)]
pub struct Pasta {
    raiz: PathBuf,
    canonica: PathBuf,
}

fn fora(texto: &str) -> ErroApi {
    ErroApi::novo(
        "caminho_fora",
        format!("'{texto}' fica fora da pasta do projeto ou não é um caminho permitido"),
    )
}

impl Pasta {
    pub fn nova(raiz: impl AsRef<Path>) -> Result<Pasta, ErroApi> {
        let raiz = std::path::absolute(raiz.as_ref())
            .map_err(|e| ErroApi::argumento(format!("pasta inválida: {e}")))?;
        let canonica = fs::canonicalize(&raiz)
            .map_err(|e| ErroApi::argumento(format!("pasta {} não existe: {e}", raiz.display())))?;
        if !canonica.is_dir() {
            return Err(ErroApi::argumento(format!(
                "{} não é uma pasta",
                raiz.display()
            )));
        }
        Ok(Pasta { raiz, canonica })
    }

    pub fn raiz(&self) -> &Path {
        &self.raiz
    }

    /// Caminho relativo à raiz, validado só pelo texto.
    fn relativo(&self, texto: &str) -> Result<PathBuf, ErroApi> {
        // absoluto só vale se já estiver dentro da raiz, para quem copia o caminho do explorador
        let sem_raiz = Path::new(texto)
            .strip_prefix(&self.raiz)
            .ok()
            .and_then(Path::to_str)
            .unwrap_or(texto);
        let limpo = sem_raiz.replace('\\', "/");
        let caminho = Path::new(&limpo);
        if limpo.is_empty()
            || limpo.contains(':')
            || limpo.chars().any(char::is_control)
            || caminho.is_absolute()
            || limpo.starts_with('/')
        {
            return Err(fora(texto));
        }
        let mut saida = PathBuf::new();
        for c in caminho.components() {
            let Component::Normal(nome) = c else {
                return Err(fora(texto));
            };
            let nome = nome.to_str().ok_or_else(|| fora(texto))?;
            let base = nome
                .split('.')
                .next()
                .unwrap_or_default()
                .trim_end()
                .to_ascii_lowercase();
            if DISPOSITIVOS.contains(&base.as_str()) || nome.ends_with('.') || nome.ends_with(' ') {
                return Err(fora(texto));
            }
            saida.push(nome);
        }
        if saida.as_os_str().is_empty() {
            return Err(fora(texto));
        }
        Ok(saida)
    }

    fn dentro(&self, canonico: &Path, texto: &str) -> Result<(), ErroApi> {
        if canonico.starts_with(&self.canonica) {
            Ok(())
        } else {
            Err(fora(texto))
        }
    }

    /// Arquivo existente para leitura, com uma das extensões aceitas.
    pub fn para_ler(&self, texto: &str, extensoes: &[&str]) -> Result<PathBuf, ErroApi> {
        let rel = self.relativo(texto)?;
        confere_extensao(&rel, extensoes)?;
        let caminho = self.raiz.join(&rel);
        let canonico = fs::canonicalize(&caminho).map_err(|e| {
            ErroApi::novo("leitura", format!("não foi possível abrir '{texto}': {e}"))
        })?;
        self.dentro(&canonico, texto)?;
        if !canonico.is_file() {
            return Err(ErroApi::novo(
                "leitura",
                format!("'{texto}' não é um arquivo"),
            ));
        }
        Ok(caminho)
    }

    /// Lê um texto de dentro da pasta.
    pub fn ler_texto(&self, texto: &str, extensoes: &[&str]) -> Result<String, ErroApi> {
        let caminho = self.para_ler(texto, extensoes)?;
        fs::read_to_string(&caminho)
            .map_err(|e| ErroApi::novo("leitura", format!("não foi possível ler '{texto}': {e}")))
    }

    /// Prepara a pasta do destino e devolve o caminho absoluto, sem criar o arquivo.
    fn destino(&self, texto: &str, extensoes: &[&str]) -> Result<PathBuf, ErroApi> {
        let rel = self.relativo(texto)?;
        confere_extensao(&rel, extensoes)?;
        let caminho = self.raiz.join(&rel);
        // o ancestral mais próximo que existe decide: uma junção no meio do caminho leva para fora
        let mut ancestral = caminho.parent().unwrap_or(&self.raiz).to_path_buf();
        while !ancestral.exists() {
            if !ancestral.pop() {
                return Err(fora(texto));
            }
        }
        let canonico = fs::canonicalize(&ancestral).map_err(|_| fora(texto))?;
        self.dentro(&canonico, texto)?;
        if let Some(pai) = caminho.parent() {
            fs::create_dir_all(pai).map_err(|e| {
                ErroApi::novo("gravacao", format!("não foi possível criar a pasta: {e}"))
            })?;
        }
        // o próprio arquivo pode ser uma ligação, inclusive quebrada, que `exists` não enxerga
        if let Ok(m) = fs::symlink_metadata(&caminho) {
            if m.file_type().is_symlink() {
                return Err(fora(texto));
            }
        }
        Ok(caminho)
    }

    /// Grava em `texto`. `escrever` recebe um arquivo provisório ao lado do destino: quem falha no meio
    /// não deixa arquivo pela metade. Sem `sobrescrever`, um arquivo que já existe é recusado.
    pub fn gravar(
        &self,
        texto: &str,
        extensoes: &[&str],
        sobrescrever: bool,
        escrever: impl FnOnce(&Path) -> Result<(), ErroApi>,
    ) -> Result<PathBuf, ErroApi> {
        let caminho = self.destino(texto, extensoes)?;
        let mut reservado = false;
        if !sobrescrever {
            // criar de forma exclusiva é a checagem e a reserva num passo só, sem janela entre elas
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&caminho)
            {
                Ok(_) => reservado = true,
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    return Err(ErroApi::novo(
                        "ja_existe",
                        format!("'{texto}' já existe; peça `sobrescrever` para trocar"),
                    ));
                }
                Err(e) => {
                    return Err(ErroApi::novo(
                        "gravacao",
                        format!("não foi possível criar '{texto}': {e}"),
                    ))
                }
            }
        }
        let extensao = caminho
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("tmp");
        let n = SEQUENCIA.fetch_add(1, Ordering::Relaxed);
        let provisorio =
            caminho.with_extension(format!("fenkra-{}-{n}.{extensao}", std::process::id()));
        let feito = escrever(&provisorio).and_then(|()| {
            renomear_com_tentativas(&provisorio, &caminho).map_err(|e| {
                let motivo = if bloqueio_temporario(&e) {
                    format!("{e}; o arquivo continuou bloqueado (em uso ou sem permissão) após várias tentativas, tente de novo")
                } else {
                    e.to_string()
                };
                ErroApi::novo(
                    "gravacao",
                    format!("não foi possível gravar '{texto}': {motivo}"),
                )
            })
        });
        if let Err(e) = feito {
            let _ = fs::remove_file(&provisorio);
            if reservado {
                let _ = fs::remove_file(&caminho);
            }
            return Err(e);
        }
        Ok(caminho)
    }

    /// Cria um arquivo novo para acrescentar linhas; `None` se o nome já está tomado.
    pub fn criar_novo(&self, texto: &str, extensoes: &[&str]) -> Result<Option<fs::File>, ErroApi> {
        let caminho = self.destino(texto, extensoes)?;
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&caminho)
        {
            Ok(f) => Ok(Some(f)),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(None),
            Err(e) => Err(ErroApi::novo(
                "gravacao",
                format!("não foi possível criar '{texto}': {e}"),
            )),
        }
    }
}

fn confere_extensao(rel: &Path, extensoes: &[&str]) -> Result<(), ErroApi> {
    let achada = rel
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if extensoes.contains(&achada.as_str()) {
        Ok(())
    } else {
        Err(ErroApi::novo(
            "formato",
            format!(
                "extensão '.{achada}' não aceita aqui; use {}",
                extensoes.join(", ")
            ),
        ))
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn pasta(nome: &str) -> Pasta {
        let d = std::env::temp_dir().join(format!("fenkra-pasta-{nome}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        Pasta::nova(&d).unwrap()
    }

    fn escreve(c: &str) -> impl FnOnce(&Path) -> Result<(), ErroApi> + '_ {
        move |p| fs::write(p, c).map_err(|e| ErroApi::novo("gravacao", e.to_string()))
    }

    #[test]
    fn recusa_tudo_que_sai_da_pasta_pelo_texto() {
        let p = pasta("texto");
        for ruim in [
            "../a.fkx",
            "x/../../a.fkx",
            "C:/Windows/a.fkx",
            "C:\\Windows\\a.fkx",
            "/etc/a.fkx",
            "\\\\servidor\\c\\a.fkx",
            "a.fkx:fluxo",
            "NUL.fkx",
            "con.fkx",
            "x/aux/a.fkx",
            "a.fkx.",
            "a\nb.fkx",
            "",
        ] {
            let e = p.gravar(ruim, &["fkx"], false, escreve("x")).unwrap_err();
            assert_eq!(e.codigo, "caminho_fora", "{ruim:?}: {e}");
            assert_eq!(
                p.para_ler(ruim, &["fkx"]).unwrap_err().codigo,
                "caminho_fora",
                "{ruim:?}"
            );
        }
    }

    #[test]
    fn absoluto_dentro_da_raiz_vale() {
        let p = pasta("absoluto");
        let dentro = p.raiz().join("sub").join("a.fkx");
        let feito = p
            .gravar(dentro.to_str().unwrap(), &["fkx"], false, escreve("x"))
            .unwrap();
        assert_eq!(feito, dentro);
        assert!(p.para_ler("sub/a.fkx", &["fkx"]).is_ok());
    }

    #[test]
    fn nao_sobrescreve_sem_pedido_e_guarda_o_conteudo() {
        let p = pasta("sobrescrever");
        let c = p.gravar("a.fkx", &["fkx"], false, escreve("um")).unwrap();
        assert_eq!(
            p.gravar("a.fkx", &["fkx"], false, escreve("dois"))
                .unwrap_err()
                .codigo,
            "ja_existe"
        );
        assert_eq!(fs::read_to_string(&c).unwrap(), "um");
        p.gravar("a.fkx", &["fkx"], true, escreve("dois")).unwrap();
        assert_eq!(fs::read_to_string(&c).unwrap(), "dois");
    }

    #[test]
    fn falha_no_meio_nao_deixa_rastro() {
        let p = pasta("falha");
        let e = p
            .gravar("a.fkx", &["fkx"], false, |_| {
                Err(ErroApi::novo("kernel", "parou"))
            })
            .unwrap_err();
        assert_eq!(e.codigo, "kernel");
        assert_eq!(fs::read_dir(p.raiz()).unwrap().count(), 0);
    }

    #[test]
    fn extensao_fora_da_lista_e_recusada() {
        let p = pasta("extensao");
        assert_eq!(
            p.gravar("a.exe", &["fkx"], false, escreve("x"))
                .unwrap_err()
                .codigo,
            "formato"
        );
    }

    #[cfg(windows)]
    #[test]
    fn juncao_para_fora_nao_passa() {
        let p = pasta("juncao");
        let alvo = pasta("juncao-alvo");
        fs::write(alvo.raiz().join("segredo.fkx"), "x").unwrap();
        let elo = p.raiz().join("elo");
        let ok = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(&elo)
            .arg(alvo.raiz())
            .output()
            .unwrap()
            .status
            .success();
        assert!(ok, "mklink /J não funcionou");
        assert_eq!(
            p.para_ler("elo/segredo.fkx", &["fkx"]).unwrap_err().codigo,
            "caminho_fora"
        );
        assert_eq!(
            p.gravar("elo/novo.fkx", &["fkx"], false, escreve("x"))
                .unwrap_err()
                .codigo,
            "caminho_fora"
        );
        assert!(!alvo.raiz().join("novo.fkx").exists());
    }
}

/// Bloqueio breve do Windows: 32 (arquivo em uso), 33 (trecho travado) e 5 (acesso negado, que a
/// troca de nome devolve quando o destino está aberto por outro processo; num arquivo realmente
/// protegido o erro persiste e volta claro depois das tentativas). Antivírus, indexador e
/// outro processo abrem o arquivo recém-criado por instantes, e a troca de nome falha nesse intervalo.
fn bloqueio_temporario(e: &std::io::Error) -> bool {
    cfg!(windows) && matches!(e.raw_os_error(), Some(5) | Some(32) | Some(33))
}

/// Troca o nome tentando de novo só quando o bloqueio é temporário (até 8 vezes, ~0,6 s no total).
/// O provisório continua intacto até a troca dar certo, então nenhum dado se perde; outros erros
/// voltam na hora.
fn renomear_com_tentativas(de: &Path, para: &Path) -> std::io::Result<()> {
    let mut espera = 10u64;
    for _ in 0..8 {
        match fs::rename(de, para) {
            Err(e) if bloqueio_temporario(&e) => {
                std::thread::sleep(std::time::Duration::from_millis(espera));
                espera = (espera * 2).min(160);
            }
            r => return r,
        }
    }
    fs::rename(de, para)
}

#[cfg(all(test, windows))]
mod testes_bloqueio {
    use super::*;
    use std::os::windows::fs::OpenOptionsExt;

    #[test]
    fn troca_de_nome_vence_bloqueio_breve_do_destino() {
        let d = std::env::temp_dir().join(format!("fenkra-pasta-bloqueio-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        let (prov, alvo) = (d.join("p.tmp"), d.join("b.step"));
        fs::write(&prov, b"conteudo").unwrap();
        fs::write(&alvo, b"").unwrap();
        // outro processo (antivírus, indexador) segura o destino por instantes
        let seg = OpenOptions::new().read(true).share_mode(0).open(&alvo).unwrap();
        let t = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(150));
            drop(seg);
        });
        renomear_com_tentativas(&prov, &alvo).unwrap();
        t.join().unwrap();
        assert_eq!(fs::read(&alvo).unwrap(), b"conteudo");
        let _ = fs::remove_dir_all(&d);
    }
}
