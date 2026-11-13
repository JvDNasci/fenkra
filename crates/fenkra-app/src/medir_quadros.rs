// SPDX-License-Identifier: LGPL-2.1-or-later
//! `FENKRA_MEDIR_QUADROS=<arquivo.csv>`: tempo de cada quadro e das fases pesadas, para medir o aceite
//! de 100 ms por quadro ao carregar uma montagem (B259, B125). Sem a variável, não faz nada.
//!
//! Uma linha por evento: `quadro,<n>,<ms>,` traz o tempo de CPU do quadro anterior (interface e
//! desenho juntos, como o eframe conta); `fase,<n>,<ms>,<nome>` traz uma fase dentro dele;
//! `valor,<n>,<numero>,<nome>` traz uma grandeza (bytes na placa, triângulos).
//!
//! B259: para separar o quadro lento por código do quadro lento pelo sistema, a passada da interface
//! de cada quadro (`inicio`, que grava ao sair de `App::ui`) traz o tempo de parede (`parede`), o
//! tempo de CPU que a thread da interface de fato rodou nele (`cpu_thread`, usuário + núcleo) e a
//! fração ociosa da máquina inteira no mesmo intervalo (`ocioso_sistema`, de 0 a 1). Quadro de 1 s com
//! 5 ms de CPU da thread não foi trabalho do programa: a thread esperou (o sistema não a escalou,
//! paginação de memória ou trava). Com a máquina sem ociosidade, a espera foi por processador.

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

/// Medida da passada da interface de um quadro (B259): criada no começo de `App::ui` e gravada ao
/// sair dele. Medir só a passada separa o quadro lento da espera por evento (parada, a interface não
/// desenha quadro nenhum).
pub struct Medida(Option<(Instant, u64, u64, u64)>);

/// Começa a medida do quadro; sem `FENKRA_MEDIR_QUADROS`, não lê relógio nenhum.
pub fn inicio() -> Medida {
    if saida().is_none() {
        return Medida(None);
    }
    Medida(match (relogio::cpu_da_thread(), relogio::tempos_do_sistema()) {
        (Some(c), Some((o, s))) => Some((Instant::now(), c, o, s)),
        _ => None,
    })
}

impl Drop for Medida {
    fn drop(&mut self) {
        let Some((t0, c0, o0, s0)) = self.0 else { return };
        let (Some(c1), Some((o1, s1))) = (relogio::cpu_da_thread(), relogio::tempos_do_sistema()) else { return };
        let r = Intervalo { parede_ms: t0.elapsed().as_secs_f64() * 1000., cpu_ms: c1.saturating_sub(c0) as f64 / 1e4, ocioso: o1.saturating_sub(o0), total: s1.saturating_sub(s0) };
        linha("valor", r.parede_ms, "parede");
        linha("valor", r.cpu_ms, "cpu_thread");
        linha("valor", r.fracao_ociosa(), "ocioso_sistema");
    }
}

/// Um intervalo medido, para classificar.
#[derive(Clone, Copy, Debug)]
pub struct Intervalo {
    pub parede_ms: f64,
    pub cpu_ms: f64,
    /// tempo ocioso e total de todos os processadores, em 100 ns
    pub ocioso: u64,
    pub total: u64,
}

impl Intervalo {
    pub fn fracao_ociosa(&self) -> f64 {
        if self.total == 0 {
            return 1.;
        }
        self.ocioso as f64 / self.total as f64
    }
}

/// Por que um quadro passou do limite.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Causa {
    /// a thread trabalhou quase o tempo todo: código do programa
    Programa,
    /// a thread esperou e a máquina estava sem processador livre: outros processos
    SistemaOcupado,
    /// a thread esperou com processador livre: paginação, disco, placa de vídeo ou trava
    Espera,
}

/// Classifica um quadro lento: metade ou mais da parede em CPU da thread é trabalho do programa;
/// senão, máquina com menos de 10% ociosa é sistema ocupado, e o resto é espera.
pub fn causa(i: &Intervalo) -> Causa {
    if i.cpu_ms >= 0.5 * i.parede_ms {
        Causa::Programa
    } else if i.fracao_ociosa() < 0.1 {
        Causa::SistemaOcupado
    } else {
        Causa::Espera
    }
}

#[cfg(windows)]
mod relogio {
    #[repr(C)]
    #[derive(Default, Clone, Copy)]
    struct Filetime {
        baixo: u32,
        alto: u32,
    }
    impl Filetime {
        fn v(self) -> u64 {
            (self.alto as u64) << 32 | self.baixo as u64
        }
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GetCurrentThread() -> isize;
        fn GetThreadTimes(h: isize, criacao: *mut Filetime, saida: *mut Filetime, nucleo: *mut Filetime, usuario: *mut Filetime) -> i32;
        fn GetSystemTimes(ocioso: *mut Filetime, nucleo: *mut Filetime, usuario: *mut Filetime) -> i32;
    }

    /// CPU da thread atual (usuário + núcleo), em 100 ns.
    pub fn cpu_da_thread() -> Option<u64> {
        let (mut a, mut b, mut k, mut u) = (Filetime::default(), Filetime::default(), Filetime::default(), Filetime::default());
        // SAFETY: ponteiros para estruturas locais do tamanho que a função espera
        let ok = unsafe { GetThreadTimes(GetCurrentThread(), &mut a, &mut b, &mut k, &mut u) };
        (ok != 0).then(|| k.v() + u.v())
    }

    /// (ocioso, total) de todos os processadores, em 100 ns. O tempo de núcleo já inclui o ocioso.
    pub fn tempos_do_sistema() -> Option<(u64, u64)> {
        let (mut o, mut k, mut u) = (Filetime::default(), Filetime::default(), Filetime::default());
        // SAFETY: idem
        let ok = unsafe { GetSystemTimes(&mut o, &mut k, &mut u) };
        (ok != 0).then(|| (o.v(), k.v() + u.v()))
    }
}

#[cfg(not(windows))]
mod relogio {
    pub fn cpu_da_thread() -> Option<u64> {
        None
    }
    pub fn tempos_do_sistema() -> Option<(u64, u64)> {
        None
    }
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

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn classifica_o_quadro_lento() {
        let i = |parede_ms, cpu_ms, ocioso| Intervalo { parede_ms, cpu_ms, ocioso, total: 100 };
        assert_eq!(causa(&i(300., 280., 50)), Causa::Programa);
        assert_eq!(causa(&i(1000., 5., 3)), Causa::SistemaOcupado);
        assert_eq!(causa(&i(1000., 5., 60)), Causa::Espera);
    }

    #[cfg(windows)]
    #[test]
    fn relogios_do_sistema_andam() {
        let c0 = relogio::cpu_da_thread().unwrap();
        let (o0, t0) = relogio::tempos_do_sistema().unwrap();
        let t = Instant::now();
        let mut x = 0u64;
        while t.elapsed().as_millis() < 60 {
            x = x.wrapping_add(std::hint::black_box(1));
        }
        let c1 = relogio::cpu_da_thread().unwrap();
        let (o1, t1) = relogio::tempos_do_sistema().unwrap();
        assert!(x > 0 && c1 > c0, "a thread girou e o relógio dela andou");
        assert!(t1 > t0 && o1 >= o0 && (t1 - t0) >= (o1 - o0));
    }
}
