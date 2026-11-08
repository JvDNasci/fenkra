// SPDX-License-Identifier: LGPL-2.1-or-later
//! B393: linhas ocultas exatas de várias vistas ao mesmo tempo, um processo por vista.
//!
//! O algoritmo exato de linhas ocultas do OpenCASCADE guarda estado global: duas threads ao mesmo
//! tempo devolvem vistas vazias ou com linhas a menos, sem erro. Processos não dividem nada. A
//! thread de projeção grava a forma em binário numa pasta temporária, chama o próprio executável
//! com `--fenkra-hlr` uma vez por vista e lê as linhas de volta. De quebra, cancelar agora para de
//! verdade: os processos são encerrados.
//!
//! Sem executável conhecido (`definir_executavel` ou `FENKRA_HLR_EXE`), ou com uma vista só, quem
//! chama projeta na própria thread. Processo que falha cai para a própria thread também.

use fenkra_kernel::{KResult, KernelError, Shape, VistaHlr};
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::OnceLock;
use std::time::Duration;

/// Argumento que põe o executável no modo de projetar uma vista e sair.
pub const ARG: &str = "--fenkra-hlr";

type Linhas = Vec<(Vec<[f64; 2]>, u8)>;

static EXE: OnceLock<PathBuf> = OnceLock::new();

/// Executável que atende `--fenkra-hlr` (o próprio Fenkra; nos testes, o `fenkra-hlr`).
pub fn definir_executavel(p: PathBuf) {
    let _ = EXE.set(p);
}

fn executavel() -> Option<PathBuf> {
    EXE.get().cloned().or_else(|| std::env::var_os("FENKRA_HLR_EXE").map(PathBuf::from)).filter(|p| p.is_file())
}

/// Processos ao mesmo tempo: cada um carrega a forma inteira, então não passa de quatro.
fn simultaneos() -> usize {
    std::thread::available_parallelism().map_or(2, |n| (n.get() / 2).clamp(1, 4))
}

/// Modo filho: `--fenkra-hlr <forma> <saida> dx dy dz xx xy xz ocultas`. `None` se os argumentos
/// não são deste modo; senão o código de saída do processo.
pub fn principal(args: &[String]) -> Option<i32> {
    if args.first().map(String::as_str) != Some(ARG) {
        return None;
    }
    let saida = PathBuf::from(args.get(2)?);
    // quem chamou mantém a entrada aberta; se ele fechar (saiu, caiu, foi encerrado), este sai também
    std::thread::spawn(|| {
        use std::io::Read;
        let mut b = [0u8; 64];
        while matches!(std::io::stdin().read(&mut b), Ok(n) if n > 0) {}
        std::process::exit(4);
    });
    let r = (|| -> Result<Linhas, String> {
        let num = |i: usize| args.get(i).and_then(|a| a.parse::<f64>().ok()).ok_or_else(|| format!("argumento {i} inválido"));
        let dados = std::fs::read(args.get(1).ok_or("sem forma")?).map_err(|e| e.to_string())?;
        let forma = Shape::de_bin(&dados).map_err(|e| e.0)?;
        let para = [num(3)?, num(4)?, num(5)?];
        let x = [num(6)?, num(7)?, num(8)?];
        let ocultas = args.get(9).is_some_and(|a| a == "1");
        forma.hlr(para, x, ocultas).map_err(|e| e.0)
    })();
    Some(match r {
        Ok(l) => match std::fs::write(&saida, codificar(&l)) {
            Ok(()) => 0,
            Err(_) => 3,
        },
        Err(e) => {
            let _ = std::fs::write(saida.with_extension("erro"), e);
            2
        }
    })
}

fn codificar(l: &Linhas) -> Vec<u8> {
    let mut o = Vec::with_capacity(l.iter().map(|(p, _)| 5 + p.len() * 16).sum::<usize>() + 4);
    o.extend_from_slice(&(l.len() as u32).to_le_bytes());
    for (pts, k) in l {
        o.push(*k);
        o.extend_from_slice(&(pts.len() as u32).to_le_bytes());
        for p in pts {
            o.extend_from_slice(&p[0].to_le_bytes());
            o.extend_from_slice(&p[1].to_le_bytes());
        }
    }
    o
}

fn decodificar(b: &[u8]) -> Option<Linhas> {
    let mut i = 0usize;
    let u32_ = |i: &mut usize| -> Option<u32> {
        let v = u32::from_le_bytes(b.get(*i..*i + 4)?.try_into().ok()?);
        *i += 4;
        Some(v)
    };
    let n = u32_(&mut i)? as usize;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        let k = *b.get(i)?;
        i += 1;
        let m = u32_(&mut i)? as usize;
        let mut pts = Vec::with_capacity(m);
        for _ in 0..m {
            let f = |j: usize| -> Option<f64> { Some(f64::from_le_bytes(b.get(j..j + 8)?.try_into().ok()?)) };
            pts.push([f(i)?, f(i + 8)?]);
            i += 16;
        }
        out.push((pts, k));
    }
    (i == b.len()).then_some(out)
}

struct Pasta(PathBuf);
impl Drop for Pasta {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn lancar(exe: &Path, forma: &Path, saida: &Path, v: &VistaHlr) -> std::io::Result<Child> {
    let mut c = Command::new(exe);
    c.arg(ARG).arg(forma).arg(saida);
    for n in v.para.iter().chain(&v.x) {
        c.arg(format!("{n:e}"));
    }
    c.arg(if v.ocultas { "1" } else { "0" });
    // a entrada fica aberta enquanto este processo vive: o filho percebe quando ela fecha
    c.stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // sem janela de console e abaixo da prioridade normal: a interface continua leve
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        const BELOW_NORMAL_PRIORITY_CLASS: u32 = 0x0000_4000;
        c.creation_flags(CREATE_NO_WINDOW | BELOW_NORMAL_PRIORITY_CLASS);
    }
    c.spawn()
}

/// Linhas ocultas exatas das vistas, um processo por vista. `None` quando não compensa ou não há
/// executável: quem chama projeta na própria thread. Cancelado, encerra os processos e devolve
/// erro em todas.
pub fn exatas(vistas: &[VistaHlr], cancelar: Option<&AtomicBool>) -> Option<Vec<KResult<Linhas>>> {
    if vistas.len() < 2 {
        return None;
    }
    let exe = executavel()?;
    let medir = std::env::var_os("FENKRA_MEDIR_DESENHO").is_some();
    let t0 = std::time::Instant::now();
    static N: AtomicU32 = AtomicU32::new(0);
    let pasta = Pasta(std::env::temp_dir().join(format!("fenkra-hlr-{}-{}", std::process::id(), N.fetch_add(1, Ordering::Relaxed))));
    std::fs::create_dir_all(&pasta.0).ok()?;
    // uma forma gravada por topologia: as vistas sem corte dividem a mesma
    let mut formas: Vec<PathBuf> = vec![];
    for (i, v) in vistas.iter().enumerate() {
        if let Some(j) = (0..i).find(|&j| vistas[j].forma.mesma(v.forma)) {
            let f = formas[j].clone();
            formas.push(f);
            continue;
        }
        let f = pasta.0.join(format!("forma{i}.bin"));
        std::fs::write(&f, v.forma.para_bin().ok()?).ok()?;
        formas.push(f);
    }
    if medir {
        eprintln!("[desenho] formas gravadas para os processos: {:?}", t0.elapsed());
    }
    let saida = |i: usize| pasta.0.join(format!("linhas{i}.bin"));
    let mut res: Vec<Option<KResult<Linhas>>> = (0..vistas.len()).map(|_| None).collect();
    let mut rodando: Vec<(usize, Child)> = vec![];
    let mut proxima = 0usize;
    let max = simultaneos();
    loop {
        if cancelar.is_some_and(|c| c.load(Ordering::Relaxed)) {
            for (_, mut c) in rodando {
                let _ = c.kill();
                let _ = c.wait();
            }
            return Some(vistas.iter().map(|_| Err(KernelError("cancelado".into()))).collect());
        }
        while rodando.len() < max && proxima < vistas.len() {
            let i = proxima;
            proxima += 1;
            match lancar(&exe, &formas[i], &saida(i), &vistas[i]) {
                Ok(c) => rodando.push((i, c)),
                // não lançou: projeta aqui mesmo
                Err(_) => res[i] = Some(vistas[i].forma.hlr(vistas[i].para, vistas[i].x, vistas[i].ocultas)),
            }
        }
        let mut k = 0;
        while k < rodando.len() {
            match rodando[k].1.try_wait() {
                Ok(None) => k += 1,
                fim => {
                    let (i, _) = rodando.swap_remove(k);
                    if medir {
                        eprintln!("[desenho] processo da vista {i} terminou em {:?}", t0.elapsed());
                    }
                    let ok = matches!(fim, Ok(Some(s)) if s.success());
                    res[i] = Some(match std::fs::read(saida(i)).ok().filter(|_| ok).and_then(|b| decodificar(&b)) {
                        Some(l) => Ok(l),
                        None => match std::fs::read_to_string(saida(i).with_extension("erro")) {
                            Ok(e) => Err(KernelError(e)),
                            // o processo caiu sem dizer por quê: projeta aqui mesmo
                            Err(_) => vistas[i].forma.hlr(vistas[i].para, vistas[i].x, vistas[i].ocultas),
                        },
                    });
                }
            }
        }
        if rodando.is_empty() && proxima >= vistas.len() {
            break;
        }
        std::thread::sleep(Duration::from_millis(30));
    }
    Some(res.into_iter().map(|r| r.unwrap_or_else(|| Err(KernelError("sem resultado".into())))).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linhas_vao_e_voltam_iguais_pelo_arquivo() {
        let l: Linhas = vec![(vec![[1.5, -2.], [3., 4.25]], 0), (vec![], 3), (vec![[0., 0.]; 3], 2)];
        assert_eq!(decodificar(&codificar(&l)), Some(l.clone()));
        let mut b = codificar(&l);
        b.pop();
        assert_eq!(decodificar(&b), None, "arquivo cortado não vale");
    }

    #[test]
    fn sem_o_argumento_nao_e_o_modo_filho() {
        assert_eq!(principal(&["--seguro".into()]), None);
        assert_eq!(principal(&[]), None);
    }
}
