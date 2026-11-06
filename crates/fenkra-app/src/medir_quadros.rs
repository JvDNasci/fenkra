// SPDX-License-Identifier: LGPL-2.1-or-later
//! `FENKRA_MEDIR_QUADROS=<arquivo.csv>`: tempo de cada quadro e das fases pesadas, para medir o aceite
//! de 100 ms por quadro ao carregar uma montagem (B259, B125). Sem a variável, não faz nada.
//!
//! Uma linha por evento: `quadro,<n>,<ms>,` traz o tempo de CPU do quadro anterior (interface e
//! desenho juntos, como o eframe conta); `fase,<n>,<ms>,<nome>` traz uma fase dentro dele;
//! `valor,<n>,<numero>,<nome>` traz uma grandeza (bytes na placa, triângulos).

use std::io::Write;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

fn saida() -> &'static Option<Mutex<std::fs::File>> {
    static S: OnceLock<Option<Mutex<std::fs::File>>> = OnceLock::new();
    S.get_or_init(|| std::env::var_os("FENKRA_MEDIR_QUADROS").and_then(|p| std::fs::File::create(p).ok()).map(Mutex::new))
}

fn linha(tipo: &str, valor: f64, nome: &str) {
    if let Some(f) = saida() {
        if let Ok(mut f) = f.lock() {
            let _ = writeln!(f, "{tipo},{},{valor:.3},{nome}", crate::partida::quadros());
        }
    }
}

/// A medição está ligada?
pub fn ativo() -> bool {
    saida().is_some()
}

/// Chamado no começo de cada quadro com o tempo de CPU do anterior, em segundos.
pub fn quadro(cpu: Option<f32>) {
    if let Some(s) = cpu.filter(|_| saida().is_some()) {
        linha("quadro", s as f64 * 1000., "");
    }
}

/// Mede uma fase pesada.
pub fn fase<T>(nome: &str, f: impl FnOnce() -> T) -> T {
    if saida().is_none() {
        return f();
    }
    let t = Instant::now();
    let r = f();
    linha("fase", t.elapsed().as_secs_f64() * 1000., nome);
    r
}

/// Registra uma grandeza.
pub fn valor(nome: &str, v: f64) {
    if saida().is_some() {
        linha("valor", v, nome);
    }
}
