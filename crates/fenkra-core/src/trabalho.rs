// SPDX-License-Identifier: LGPL-2.1-or-later
//! Fila de trabalho pesado fora da thread da interface, com progresso e cancelamento (B259).
//!
//! Regras:
//! - **Um pedido novo cancela o anterior.** O cálculo em andamento recebe o sinal de cancelar (que o
//!   kernel e o recálculo consultam) e o seu resultado, se chegar a terminar, é descartado: a
//!   interface nunca mostra um resultado já superado.
//! - **Pedidos acumulados se fundem.** Se vários chegam enquanto a thread está ocupada, só o último
//!   roda; `fundir` decide o que herdar dos que foram pulados (por exemplo "reconstruir do zero").
//! - **O estado pesado fica dentro da thread.** `fabrica` roda na thread de trabalho e devolve a
//!   função que processa os pedidos, então o que não atravessa threads (formas do OpenCASCADE, cache
//!   do recálculo) nunca precisa ser `Send`.
//! - **Resultado igual ao do zero.** Cancelar não deixa nada pela metade no cache de quem calcula:
//!   quem implementa `executar` deve guardar só o que terminou.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};

/// Sinal de cancelamento e contagem de progresso de um pedido. Barato de copiar.
#[derive(Clone, Debug, Default)]
pub struct Controle {
    cancelar: Arc<AtomicBool>,
    feito: Arc<AtomicU32>,
    total: Arc<AtomicU32>,
}

impl Controle {
    pub fn novo() -> Self {
        Self::default()
    }
    pub fn cancelar(&self) {
        self.cancelar.store(true, Ordering::Relaxed);
    }
    pub fn cancelado(&self) -> bool {
        self.cancelar.load(Ordering::Relaxed)
    }
    /// A bandeira em si, para quem só entende `Arc<AtomicBool>` (o recálculo e o kernel).
    pub fn bandeira(&self) -> Arc<AtomicBool> {
        self.cancelar.clone()
    }
    pub fn definir_total(&self, n: u32) {
        self.total.store(n, Ordering::Relaxed);
        self.feito.store(0, Ordering::Relaxed);
    }
    /// Acrescenta passos ao total quando o trabalho descobre mais (subconjunto aberto no caminho).
    pub fn somar_total(&self, n: u32) {
        self.total.fetch_add(n, Ordering::Relaxed);
    }
    pub fn avancar(&self) {
        self.feito.fetch_add(1, Ordering::Relaxed);
    }
    /// (feitos, total); total 0 = ainda sem contagem.
    pub fn progresso(&self) -> (u32, u32) {
        (self.feito.load(Ordering::Relaxed), self.total.load(Ordering::Relaxed))
    }
    /// De 0 a 1, ou `None` quando ainda não há contagem.
    pub fn fracao(&self) -> Option<f32> {
        let (f, t) = self.progresso();
        (t > 0).then(|| (f as f32 / t as f32).min(1.0))
    }
}

/// Função que processa um pedido. Devolve `None` quando abandonou o trabalho.
pub type Executor<J, R> = Box<dyn FnMut(J, &Controle) -> Option<R>>;

pub struct Fila<J: Send + 'static, R: Send + 'static> {
    tx: Sender<(J, Controle)>,
    rx: Receiver<R>,
    atual: Mutex<Option<Controle>>,
}

impl<J: Send + 'static, R: Send + 'static> Fila<J, R> {
    /// Cria a fila e a thread. `fundir(pulado, mais_novo)` junta dois pedidos acumulados;
    /// `fabrica` roda dentro da thread; `ao_terminar` acorda a interface (ex.: `request_repaint`).
    pub fn nova<F>(nome: &str, fundir: fn(J, J) -> J, fabrica: F, ao_terminar: impl Fn() + Send + 'static) -> Self
    where
        F: FnOnce() -> Executor<J, R> + Send + 'static,
    {
        let (tx, jobs) = channel::<(J, Controle)>();
        let (out, rx) = channel::<R>();
        std::thread::Builder::new()
            .name(nome.into())
            .spawn(move || {
                let mut executar = fabrica();
                // pedido cancelado antes de rodar: o que ele pedia (ex.: reconstruir) passa ao próximo
                let mut pulado: Option<J> = None;
                while let Ok((mut job, mut ctl)) = jobs.recv() {
                    if let Some(p) = pulado.take() {
                        job = fundir(p, job);
                    }
                    // pedidos acumulados: só o último roda, herdando o que `fundir` mandar
                    while let Ok((novo, c)) = jobs.try_recv() {
                        ctl.cancelar();
                        job = fundir(job, novo);
                        ctl = c;
                    }
                    if ctl.cancelado() {
                        pulado = Some(job);
                        continue;
                    }
                    let r = executar(job, &ctl);
                    // superado enquanto calculava: o resultado não vale mais
                    if let Some(r) = r.filter(|_| !ctl.cancelado()) {
                        if out.send(r).is_err() {
                            break;
                        }
                        ao_terminar();
                    }
                }
            })
            .expect("thread de trabalho");
        Fila { tx, rx, atual: Mutex::new(None) }
    }

    /// Enfileira um pedido e cancela o que estava em andamento. Devolve o controle do novo.
    pub fn enviar(&self, job: J) -> Controle {
        let ctl = Controle::novo();
        if let Ok(mut a) = self.atual.lock() {
            if let Some(antigo) = a.replace(ctl.clone()) {
                antigo.cancelar();
            }
        }
        let _ = self.tx.send((job, ctl.clone()));
        ctl
    }

    /// Próximo resultado pronto, sem esperar.
    pub fn receber(&self) -> Option<R> {
        self.rx.try_recv().ok()
    }

    /// Progresso do pedido mais recente.
    pub fn progresso(&self) -> Option<(u32, u32)> {
        self.atual.lock().ok().and_then(|a| a.as_ref().map(|c| c.progresso())).filter(|(_, t)| *t > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;
    use std::time::{Duration, Instant};

    fn esperar<R: Send + 'static>(f: &Fila<u64, R>, ate: Duration) -> Option<R> {
        let t = Instant::now();
        while t.elapsed() < ate {
            if let Some(r) = f.receber() {
                return Some(r);
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        None
    }

    /// Cálculo "longo" que consulta o cancelamento: devolve (valor, passos dados).
    fn longo(n: u64, ctl: &Controle) -> Option<(u64, u64)> {
        ctl.definir_total(100);
        let mut feitos = 0;
        for _ in 0..100 {
            if ctl.cancelado() {
                return None;
            }
            std::thread::sleep(Duration::from_millis(3));
            ctl.avancar();
            feitos += 1;
        }
        Some((n * 2, feitos))
    }

    #[test]
    fn pedido_novo_cancela_o_em_andamento_e_o_resultado_e_o_do_zero() {
        let passos_perdidos = Arc::new(AtomicUsize::new(0));
        let p = passos_perdidos.clone();
        let f: Fila<u64, (u64, u64)> = Fila::nova(
            "teste",
            |_, novo| novo,
            move || {
                Box::new(move |n, ctl| {
                    let r = longo(n, ctl);
                    if r.is_none() {
                        p.fetch_add(1, Ordering::Relaxed);
                    }
                    r
                })
            },
            || {},
        );
        let primeiro = f.enviar(1);
        // espera o cálculo 1 realmente começar (a thread pode demorar a subir numa máquina ocupada)
        let t0 = Instant::now();
        while primeiro.progresso().0 < 3 && t0.elapsed() < Duration::from_secs(20) {
            std::thread::sleep(Duration::from_millis(2));
        }
        assert!(primeiro.progresso().0 >= 3, "o primeiro cálculo não começou");
        assert!(!primeiro.cancelado());
        f.enviar(2);
        assert!(primeiro.cancelado(), "o pedido em andamento recebeu o sinal");
        let (v, passos) = esperar(&f, Duration::from_secs(5)).expect("resultado");
        assert_eq!(v, 4, "só o resultado do pedido mais novo chega");
        assert_eq!(passos, 100, "o novo cálculo foi completo, como se fosse do zero");
        assert!(esperar(&f, Duration::from_millis(100)).is_none(), "o cancelado não deixa resultado");
        assert!(passos_perdidos.load(Ordering::Relaxed) >= 1, "o cálculo antigo foi abandonado no meio");
    }

    #[test]
    fn pedidos_acumulados_se_fundem_e_so_o_ultimo_roda() {
        let rodou = Arc::new(Mutex::new(vec![]));
        let r2 = rodou.clone();
        // bit 0x100 = "reconstruir do zero", herdado dos pulados
        let f: Fila<u64, u64> = Fila::nova(
            "funde",
            |a, b| b | (a & 0x100),
            move || {
                Box::new(move |n, _| {
                    std::thread::sleep(Duration::from_millis(60));
                    r2.lock().unwrap().push(n);
                    Some(n)
                })
            },
            || {},
        );
        f.enviar(1); // ocupa a thread
        std::thread::sleep(Duration::from_millis(15));
        f.enviar(0x100 | 2);
        f.enviar(3);
        f.enviar(4);
        let mut vistos = vec![];
        while let Some(r) = esperar(&f, Duration::from_millis(400)) {
            vistos.push(r);
        }
        assert_eq!(vistos, vec![0x100 | 4], "o primeiro foi cancelado; entre os acumulados roda o último, com a marca herdada");
        assert!(!rodou.lock().unwrap().contains(&3));
    }

    #[test]
    fn progresso_aparece_e_a_fracao_sobe() {
        let f: Fila<u64, u64> = Fila::nova("prog", |_, n| n, || Box::new(|n, ctl| longo(n, ctl).map(|r| r.0)), || {});
        assert!(f.progresso().is_none());
        let c = f.enviar(5);
        std::thread::sleep(Duration::from_millis(60));
        let (feitos, total) = f.progresso().expect("progresso");
        assert_eq!(total, 100);
        assert!(feitos > 0 && feitos < 100, "{feitos}");
        assert!(c.fracao().unwrap() > 0.0);
        assert_eq!(esperar(&f, Duration::from_secs(5)), Some(10));
    }

    #[test]
    fn o_estado_da_thread_nao_precisa_ser_send() {
        use std::rc::Rc;
        let f: Fila<u64, u64> = Fila::nova(
            "rc",
            |_, n| n,
            || {
                let acumulado = Rc::new(std::cell::Cell::new(0u64));
                Box::new(move |n, _| {
                    acumulado.set(acumulado.get() + n);
                    Some(acumulado.get())
                })
            },
            || {},
        );
        f.enviar(2);
        assert_eq!(esperar(&f, Duration::from_secs(2)), Some(2));
        f.enviar(5);
        assert_eq!(esperar(&f, Duration::from_secs(2)), Some(7), "o estado fica entre os pedidos");
    }
}
