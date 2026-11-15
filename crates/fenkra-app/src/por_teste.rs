// SPDX-License-Identifier: LGPL-2.1-or-later
//! Estado de sessão guardado em `static` que, nos testes, vale por thread (B453).
//!
//! O executor de testes roda cada teste na sua thread. Um `static` comum (idioma, variante do
//! comando de PMI, contador de quadros, camadas de configuração) é visto por todos ao mesmo tempo,
//! e um teste muda o valor no meio do outro: o teste passa sozinho e falha com a suíte em
//! paralelo. Declarado com `global_por_teste!`, o valor é o `static` de sempre no programa e uma
//! cópia própria de cada thread nos testes, acessada do mesmo jeito (`X.lock()`, `X.load(..)`).

/// Acesso à cópia da thread atual. Cada thread cria a sua na primeira vez e a guarda até o fim do
/// processo (a cópia vaza de propósito: são poucos bytes por thread de teste).
#[cfg_attr(not(test), allow(dead_code))]
pub struct PorThread<T: 'static>(pub fn() -> &'static T);

impl<T: 'static> std::ops::Deref for PorThread<T> {
    type Target = T;
    fn deref(&self) -> &T {
        (self.0)()
    }
}

/// `global_por_teste! { static NOME: Tipo = valor_inicial; }`
macro_rules! global_por_teste {
    ($(#[$m:meta])* $vis:vis static $nome:ident: $t:ty = $init:expr;) => {
        $(#[$m])*
        #[cfg(not(test))]
        $vis static $nome: $t = $init;
        $(#[$m])*
        #[cfg(test)]
        $vis static $nome: $crate::por_teste::PorThread<$t> = {
            fn desta_thread() -> &'static $t {
                ::std::thread_local! {
                    static COPIA: &'static $t = ::std::boxed::Box::leak(::std::boxed::Box::new($init));
                }
                COPIA.with(|c| *c)
            }
            $crate::por_teste::PorThread(desta_thread)
        };
    };
}

#[cfg(test)]
mod testes {
    use std::sync::atomic::{AtomicU32, Ordering};

    global_por_teste! { static N: AtomicU32 = AtomicU32::new(0); }

    #[test]
    fn cada_thread_tem_a_sua_copia() {
        N.store(7, Ordering::Relaxed);
        let outra = std::thread::spawn(|| {
            let antes = N.load(Ordering::Relaxed);
            N.store(3, Ordering::Relaxed);
            antes
        })
        .join()
        .unwrap();
        assert_eq!(outra, 0, "a outra thread começa do valor inicial");
        assert_eq!(N.load(Ordering::Relaxed), 7, "e não mexe na cópia desta");
    }
}
