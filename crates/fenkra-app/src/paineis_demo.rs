// SPDX-License-Identifier: LGPL-2.1-or-later
//! Desenvolvimento: `FENKRA_DEMO_PAINEL=<passos>` faz no app de verdade o que só o mouse faria com os
//! painéis: arrastar, encaixar, minimizar, fechar e redimensionar. Os passos viram eventos crus de ponteiro
//! do egui, antes de a interface os ler, e passam pelo mesmo caminho do mouse.
//!
//! Passos separados por `;`, em pontos da janela: `clique:x,y`, `arrasto:x0,y0,x1,y1` (aperta, move em
//! passos e solta) e `segura:x0,y0,x1,y1` (igual, mas deixa o botão apertado, para capturar a sombra do
//! encaixe ou um redimensionamento em andamento). Começam 4 s depois da abertura.

use eframe::egui::{self, Event, PointerButton, Pos2};
use std::collections::VecDeque;
use std::sync::Mutex;

/// Espera até a interface assentar, em segundos desde a abertura.
const ESPERA: f64 = 4.;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Passo {
    Clique(Pos2),
    Arrasto(Pos2, Pos2, bool),
}

fn ler(t: &str) -> Option<Vec<Passo>> {
    t.split(';').filter(|p| !p.trim().is_empty()).map(ler_passo).collect()
}

fn ler_passo(t: &str) -> Option<Passo> {
    let (nome, resto) = t.trim().split_once(':')?;
    let n: Vec<f32> = resto.split(',').map(|x| x.trim().parse().ok()).collect::<Option<_>>()?;
    match (nome, n.as_slice()) {
        ("clique", [x, y]) => Some(Passo::Clique(egui::pos2(*x, *y))),
        ("arrasto", [a, b, c, d]) => Some(Passo::Arrasto(egui::pos2(*a, *b), egui::pos2(*c, *d), true)),
        ("segura", [a, b, c, d]) => Some(Passo::Arrasto(egui::pos2(*a, *b), egui::pos2(*c, *d), false)),
        _ => None,
    }
}

fn quadros(p: Passo) -> VecDeque<Vec<Event>> {
    let botao = |pos, pressed| Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Default::default() };
    match p {
        Passo::Clique(q) => [vec![Event::PointerMoved(q)], vec![botao(q, true)], vec![botao(q, false)], vec![]].into(),
        Passo::Arrasto(a, b, solta) => {
            let mut v: Vec<Vec<Event>> = vec![vec![Event::PointerMoved(a)], vec![botao(a, true)]];
            for k in 1..=6 {
                v.push(vec![Event::PointerMoved(a.lerp(b, k as f32 / 6.))]);
            }
            if solta {
                v.push(vec![botao(b, false)]);
                v.push(vec![]);
            }
            v.into()
        }
    }
}

struct Estado {
    passos: VecDeque<Passo>,
    agora: VecDeque<Vec<Event>>,
}

static ESTADO: Mutex<Option<Estado>> = Mutex::new(None);

/// Chamado antes de a interface ler a entrada do quadro.
pub fn eventos(raw: &mut egui::RawInput) {
    let Ok(t) = std::env::var("FENKRA_DEMO_PAINEL") else { return };
    if raw.time.unwrap_or(0.) < ESPERA {
        return;
    }
    let Ok(mut g) = ESTADO.lock() else { return };
    let e = g.get_or_insert_with(|| Estado { passos: ler(&t).unwrap_or_default().into(), agora: VecDeque::new() });
    if e.agora.is_empty() {
        if let Some(p) = e.passos.pop_front() {
            e.agora = quadros(p);
        }
    }
    raw.events.extend(e.agora.pop_front().unwrap_or_default());
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn le_os_passos_e_recusa_o_que_nao_entende() {
        let v = ler("clique:1,2; arrasto:0,0,9,9;segura:1,1,2,2").unwrap();
        assert_eq!(v[0], Passo::Clique(egui::pos2(1., 2.)));
        assert_eq!(v[1], Passo::Arrasto(egui::pos2(0., 0.), egui::pos2(9., 9.), true));
        assert_eq!(v[2], Passo::Arrasto(egui::pos2(1., 1.), egui::pos2(2., 2.), false));
        assert!(ler("clique:1").is_none() && ler("pulo:1,2").is_none());
    }

    #[test]
    fn segurar_nao_solta_o_botao() {
        let solta = |q: &VecDeque<Vec<Event>>| q.iter().flatten().any(|e| matches!(e, Event::PointerButton { pressed: false, .. }));
        assert!(!solta(&quadros(Passo::Arrasto(egui::pos2(0., 0.), egui::pos2(9., 9.), false))));
        assert!(solta(&quadros(Passo::Arrasto(egui::pos2(0., 0.), egui::pos2(9., 9.), true))));
    }
}
