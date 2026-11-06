// SPDX-License-Identifier: LGPL-2.1-or-later
//! Desenvolvimento: `FENKRA_DEMO_SKETCH_GESTO=<passos>` faz no app de verdade o que só o mouse faria.
//! Os passos viram eventos crus do egui, antes de a interface os ler, e passam pelo mesmo caminho do
//! ponteiro: `Response` da viewport, escolha da entidade, janela e clique.
//!
//! Passos separados por `;`, em coordenadas do plano do sketch: `clique:x,y`, `shift:x,y`, `ctrl:x,y`
//! e `arrasto:x0,y0,x1,y1`. Ao fim de cada passo, a seleção sai na saída de erro.

use crate::sketch_ui::Ent;
use eframe::egui::{self, Event, Modifiers, PointerButton, Pos2};
use std::collections::VecDeque;

type P = [f64; 2];

/// Espera até a vista do sketch assentar, em segundos desde a abertura.
const ESPERA: f64 = 5.;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Passo {
    Clique(P, Modifiers),
    Arrasto(P, P),
}

fn ler(t: &str) -> Option<Vec<Passo>> {
    t.split(';').filter(|p| !p.trim().is_empty()).map(ler_passo).collect()
}

fn ler_passo(t: &str) -> Option<Passo> {
    let (nome, resto) = t.trim().split_once(':')?;
    let n: Vec<f64> = resto.split(',').map(|x| x.trim().parse().ok()).collect::<Option<_>>()?;
    match (nome, n.as_slice()) {
        ("clique", [x, y]) => Some(Passo::Clique([*x, *y], Modifiers::NONE)),
        ("shift", [x, y]) => Some(Passo::Clique([*x, *y], Modifiers::SHIFT)),
        ("ctrl", [x, y]) => Some(Passo::Clique([*x, *y], Modifiers::COMMAND)),
        ("arrasto", [a, b, c, d]) => Some(Passo::Arrasto([*a, *b], [*c, *d])),
        _ => None,
    }
}

/// Eventos de cada quadro do passo. Um quadro por mudança: o egui decide clique e arraste entre quadros.
fn quadros(p: Passo, tela: &dyn Fn(P) -> Pos2) -> VecDeque<Vec<Event>> {
    let botao = |pos, pressed, modifiers| Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers };
    match p {
        Passo::Clique(uv, m) => {
            let q = tela(uv);
            [vec![Event::ModifiersChanged(m), Event::PointerMoved(q)], vec![botao(q, true, m)], vec![botao(q, false, m)], vec![Event::ModifiersChanged(Modifiers::NONE)]].into()
        }
        Passo::Arrasto(de, ate) => {
            let (a, b) = (tela(de), tela(ate));
            let m = Modifiers::NONE;
            [vec![Event::PointerMoved(a)], vec![botao(a, true, m)], vec![Event::PointerMoved(a.lerp(b, 0.5))], vec![Event::PointerMoved(b)], vec![botao(b, false, m)], vec![]].into()
        }
    }
}

/// Gesto em andamento.
pub struct Gesto {
    passos: VecDeque<Passo>,
    agora: VecDeque<Vec<Event>>,
    n: usize,
}

impl Gesto {
    pub fn do_ambiente() -> Option<Gesto> {
        let t = std::env::var("FENKRA_DEMO_SKETCH_GESTO").ok()?;
        Some(Gesto { passos: ler(&t)?.into(), agora: VecDeque::new(), n: 0 })
    }

    /// Chamado pela viewport: é lá que se sabe onde cada ponto do plano cai na tela.
    pub fn quadro(&mut self, tempo: f64, tela: &dyn Fn(P) -> Pos2, sel: &[Ent]) {
        if tempo < ESPERA || !self.agora.is_empty() {
            return;
        }
        if self.n > 0 {
            let curvas = sel.iter().filter(|e| matches!(e, Ent::Cv(_))).count();
            eprintln!("gesto {}: {curvas} curva(s) na seleção", self.n);
        }
        if let Some(p) = self.passos.pop_front() {
            self.agora = quadros(p, tela);
            self.n += 1;
        } else {
            self.n = 0;
        }
    }

    /// Chamado antes de a interface ler a entrada do quadro.
    pub fn eventos(&mut self, raw: &mut egui::RawInput) {
        raw.events.extend(self.agora.pop_front().unwrap_or_default());
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn le_os_passos_e_recusa_o_que_nao_entende() {
        let v = ler("clique:1,2; shift:3,4;ctrl:5,6;arrasto:0,0,9,9").unwrap();
        assert_eq!(v[0], Passo::Clique([1., 2.], Modifiers::NONE));
        assert_eq!(v[1], Passo::Clique([3., 4.], Modifiers::SHIFT));
        assert_eq!(v[2], Passo::Clique([5., 6.], Modifiers::COMMAND));
        assert_eq!(v[3], Passo::Arrasto([0., 0.], [9., 9.]));
        assert!(ler("clique:1").is_none() && ler("pulo:1,2").is_none());
    }
}
