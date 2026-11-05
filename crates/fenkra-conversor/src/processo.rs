// SPDX-License-Identifier: LGPL-2.1-or-later
//! Processo filho `fenkra-conv`: o kernel lê arquivo de fora longe do programa.
//!
//! O OpenCASCADE é C++ e um arquivo hostil pode derrubá-lo com falha de memória, que nenhum tratamento
//! de erro alcança; ou travar num laço. Dentro do filho isso custa só o filho. O pedido vai em JSON
//! pela entrada padrão e a resposta volta num arquivo temporário: ela carrega o documento inteiro, e
//! um pipe cheio travaria os dois lados.

use crate::{ErroConv, Importado, OpcoesExportar, OpcoesImportar, Registro};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// O que o filho deve fazer.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Trabalho {
    Importar { caminho: PathBuf, opcoes: OpcoesImportar },
    Converter { origem: PathBuf, destino: PathBuf, importar: OpcoesImportar, exportar: OpcoesExportar },
    /// Só para testar o isolamento: encerra como uma falha de memória faria.
    Queda,
    /// Só para testar o limite de tempo.
    Trava,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Pedido {
    trabalho: Trabalho,
    resposta: PathBuf,
}

/// Resultado de um trabalho.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Resposta {
    Importado(Box<Importado>),
    Convertido,
}

fn executar(t: Trabalho) -> Result<Resposta, ErroConv> {
    let registro = Registro::default();
    match t {
        Trabalho::Importar { caminho, opcoes } => registro.importar(&caminho, &opcoes).map(|i| Resposta::Importado(Box::new(i))),
        Trabalho::Converter { origem, destino, importar, exportar } => registro.converter(&origem, &destino, &importar, &exportar).map(|()| Resposta::Convertido),
        Trabalho::Queda => std::process::abort(),
        Trabalho::Trava => loop {
            std::thread::sleep(Duration::from_secs(60));
        },
    }
}

/// Ponto de entrada do binário `fenkra-conv`. Devolve o código de saída.
pub fn principal() -> i32 {
    let mut entrada = String::new();
    if std::io::stdin().read_to_string(&mut entrada).is_err() {
        return 2;
    }
    let Ok(pedido) = serde_json::from_str::<Pedido>(&entrada) else { return 2 };
    let resultado = executar(pedido.trabalho);
    let Ok(json) = serde_json::to_vec(&resultado) else { return 3 };
    // nome provisório: o programa nunca lê uma resposta pela metade
    let provisorio = pedido.resposta.with_extension("parcial");
    if std::fs::write(&provisorio, json).is_err() || std::fs::rename(&provisorio, &pedido.resposta).is_err() {
        return 3;
    }
    0
}

/// Quem chama o filho, com o limite de tempo.
#[derive(Clone, Debug)]
pub struct Filho {
    pub programa: PathBuf,
    pub limite: Duration,
}

fn nome_do_programa() -> String {
    format!("fenkra-conv{}", std::env::consts::EXE_SUFFIX)
}

static SEQUENCIA: AtomicU64 = AtomicU64::new(0);

impl Filho {
    /// Limite padrão: um STEP grande leva minutos; dez já indicam laço.
    pub const LIMITE: Duration = Duration::from_secs(600);

    /// O filho mora na pasta do executável; nos testes o executável está em `deps/`.
    pub fn ao_lado_do_programa() -> Result<Filho, ErroConv> {
        let exe = std::env::current_exe().map_err(|e| ErroConv::Ausente(e.to_string()))?;
        let pasta = exe.parent().ok_or_else(|| ErroConv::Ausente(exe.display().to_string()))?;
        let nome = nome_do_programa();
        let candidatos = [pasta.join(&nome), pasta.parent().map(|p| p.join(&nome)).unwrap_or_default()];
        let achado = candidatos.iter().find(|c| c.is_file()).ok_or_else(|| ErroConv::Ausente(candidatos[0].display().to_string()))?;
        Ok(Filho { programa: achado.clone(), limite: Self::LIMITE })
    }

    fn comando(programa: &Path) -> Command {
        let mut c = Command::new(programa);
        c.stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const SEM_JANELA: u32 = 0x0800_0000;
            c.creation_flags(SEM_JANELA);
        }
        c
    }

    /// Roda o trabalho e espera. `cancelar` encerra o filho na hora.
    pub fn executar(&self, trabalho: Trabalho, cancelar: &AtomicBool) -> Result<Resposta, ErroConv> {
        let n = SEQUENCIA.fetch_add(1, Ordering::Relaxed);
        let resposta = std::env::temp_dir().join(format!("fenkra-conv-{}-{n}.json", std::process::id()));
        let resultado = self.rodar(trabalho, &resposta, cancelar);
        let _ = std::fs::remove_file(&resposta);
        let _ = std::fs::remove_file(resposta.with_extension("parcial"));
        resultado
    }

    fn rodar(&self, trabalho: Trabalho, resposta: &Path, cancelar: &AtomicBool) -> Result<Resposta, ErroConv> {
        let pedido = serde_json::to_vec(&Pedido { trabalho, resposta: resposta.to_path_buf() }).map_err(|e| ErroConv::Leitura(e.to_string()))?;
        let mut filho = Self::comando(&self.programa).spawn().map_err(|e| ErroConv::Ausente(format!("{}: {e}", self.programa.display())))?;
        if let Some(mut entrada) = filho.stdin.take() {
            // filho que morreu antes de ler é tratado pelo código de saída logo abaixo
            let _ = entrada.write_all(&pedido);
        }
        let inicio = Instant::now();
        let estado = loop {
            if cancelar.load(Ordering::Relaxed) {
                let _ = filho.kill();
                let _ = filho.wait();
                return Err(ErroConv::Cancelado);
            }
            if inicio.elapsed() > self.limite {
                let _ = filho.kill();
                let _ = filho.wait();
                return Err(ErroConv::Tempo(self.limite.as_secs()));
            }
            match filho.try_wait() {
                Ok(Some(s)) => break s,
                Ok(None) => std::thread::sleep(Duration::from_millis(20)),
                Err(e) => return Err(ErroConv::Leitura(e.to_string())),
            }
        };
        if !estado.success() {
            return Err(ErroConv::Queda(estado.code().unwrap_or(-1)));
        }
        let texto = std::fs::read(resposta).map_err(|e| ErroConv::Leitura(format!("resposta do conversor: {e}")))?;
        serde_json::from_slice::<Result<Resposta, ErroConv>>(&texto).map_err(|e| ErroConv::Leitura(format!("resposta do conversor: {e}")))?
    }

    /// Importa em isolamento.
    pub fn importar(&self, caminho: &Path, opcoes: OpcoesImportar, cancelar: &AtomicBool) -> Result<Importado, ErroConv> {
        match self.executar(Trabalho::Importar { caminho: caminho.to_path_buf(), opcoes }, cancelar)? {
            Resposta::Importado(i) => Ok(*i),
            Resposta::Convertido => Err(ErroConv::Leitura("resposta inesperada do conversor".into())),
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn programa_ausente_devolve_erro_com_o_caminho() {
        let f = Filho { programa: PathBuf::from("nao-existe-fenkra-conv.exe"), limite: Duration::from_secs(1) };
        let e = f.executar(Trabalho::Queda, &AtomicBool::new(false)).unwrap_err();
        assert!(matches!(&e, ErroConv::Ausente(m) if m.contains("nao-existe")), "{e:?}");
    }

    #[test]
    fn pedido_volta_igual_do_json() {
        let t = Trabalho::Importar { caminho: "C:/Usuários/João/peça com espaço.step".into(), opcoes: OpcoesImportar::default() };
        let j = serde_json::to_string(&Pedido { trabalho: t, resposta: "r.json".into() }).unwrap();
        let p: Pedido = serde_json::from_str(&j).unwrap();
        assert!(matches!(p.trabalho, Trabalho::Importar { caminho, .. } if caminho.to_string_lossy().contains("João")));
    }
}
