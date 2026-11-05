// SPDX-License-Identifier: LGPL-2.1-or-later
//! A saída padrão pertence ao protocolo, mas o OpenCASCADE escreve estatística de leitura e gravação
//! direto nela, por `std::cout`. Uma linha dessas no meio do JSON derruba o cliente, então o programa
//! guarda a saída real para si e aponta o descritor 1 para a saída de erro.

use std::ffi::c_int;
use std::fs::File;

#[cfg(windows)]
mod crt {
    use std::ffi::{c_int, c_void};
    extern "C" {
        fn _dup(fd: c_int) -> c_int;
        fn _dup2(de: c_int, para: c_int) -> c_int;
        fn _get_osfhandle(fd: c_int) -> isize;
    }
    pub fn arquivo(fd: c_int) -> Option<std::fs::File> {
        use std::os::windows::io::FromRawHandle;
        // SAFETY: `fd` acabou de sair de `_dup`, é só nosso e fica aberto até o fim do processo
        let h = unsafe { _get_osfhandle(fd) };
        (h != -1).then(|| unsafe { std::fs::File::from_raw_handle(h as *mut c_void) })
    }
    pub fn duplicar(fd: c_int) -> c_int {
        // SAFETY: chamada da biblioteca C com descritor numérico; erro volta como -1
        unsafe { _dup(fd) }
    }
    pub fn apontar(de: c_int, para: c_int) -> c_int {
        // SAFETY: idem
        unsafe { _dup2(de, para) }
    }
}

#[cfg(unix)]
mod crt {
    use std::ffi::c_int;
    extern "C" {
        fn dup(fd: c_int) -> c_int;
        fn dup2(de: c_int, para: c_int) -> c_int;
    }
    pub fn arquivo(fd: c_int) -> Option<std::fs::File> {
        use std::os::fd::FromRawFd;
        // SAFETY: `fd` acabou de sair de `dup` e passa a ser só do `File`
        Some(unsafe { std::fs::File::from_raw_fd(fd) })
    }
    pub fn duplicar(fd: c_int) -> c_int {
        // SAFETY: chamada da biblioteca C com descritor numérico; erro volta como -1
        unsafe { dup(fd) }
    }
    pub fn apontar(de: c_int, para: c_int) -> c_int {
        // SAFETY: idem
        unsafe { dup2(de, para) }
    }
}

const STDOUT: c_int = 1;
const STDERR: c_int = 2;

/// Toma a saída padrão para o protocolo. Depois disso, `print!` e `std::cout` vão para a saída de erro.
pub fn reservar_stdout() -> std::io::Result<File> {
    let copia = crt::duplicar(STDOUT);
    if copia < 0 || crt::apontar(STDERR, STDOUT) < 0 {
        return Err(std::io::Error::last_os_error());
    }
    crt::arquivo(copia).ok_or_else(std::io::Error::last_os_error)
}
