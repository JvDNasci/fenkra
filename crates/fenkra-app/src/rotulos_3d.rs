// SPDX-License-Identifier: LGPL-2.1-or-later
//! Balões e notas do 3D que não se cobrem (B430). Duas anotações presas perto uma da outra (os balões
//! de peças vizinhas, a legenda e a nota da mesma âncora) caíam no mesmo ponto da tela e ficavam
//! ilegíveis. A cada passada de desenho, o que já foi posto fica numa lista; o que chega depois desce
//! até achar lugar livre. A lista recomeça quando a passada muda.

use eframe::egui::{self, Pos2, Rect};

const FOLGA: f32 = 3.;

/// Desce `r` até não cobrir nenhum retângulo de `usados`; se a coluna de baixo acabar (passar de
/// `limite`), sobe a partir do original. Sem lugar livre, fica onde estava.
pub fn afasta(r: Rect, usados: &[Rect], limite: Rect) -> Rect {
    let cobre = |q: Rect| usados.iter().any(|u| u.expand(FOLGA / 2.).intersects(q));
    let mut q = r;
    for _ in 0..=usados.len() {
        match usados.iter().find(|u| u.expand(FOLGA / 2.).intersects(q)) {
            Some(u) => q = q.translate(egui::vec2(0., u.bottom() + FOLGA - q.top())),
            None => break,
        }
    }
    if q.bottom() > limite.bottom() || cobre(q) {
        q = r;
        for _ in 0..=usados.len() {
            match usados.iter().find(|u| u.expand(FOLGA / 2.).intersects(q)) {
                Some(u) => q = q.translate(egui::vec2(0., u.top() - FOLGA - q.bottom())),
                None => break,
            }
        }
        if cobre(q) || q.top() < limite.top() {
            return r;
        }
    }
    q
}

/// Leva o ponto `p` (origem de um rótulo) para baixo dos retângulos que o contêm.
pub fn afasta_ponto(p: Pos2, usados: &[Rect]) -> Pos2 {
    let mut q = p;
    for _ in 0..=usados.len() {
        match usados.iter().find(|u| u.expand(FOLGA / 2.).contains(q)) {
            Some(u) => q.y = u.bottom() + FOLGA,
            None => break,
        }
    }
    q
}

fn lista(ui: &egui::Ui) -> Vec<Rect> {
    let passada = ui.ctx().cumulative_pass_nr();
    let (nr, v): (u64, Vec<Rect>) = ui.ctx().data(|d| d.get_temp(egui::Id::new("rotulos_3d"))).unwrap_or_default();
    if nr == passada {
        v
    } else {
        Vec::new()
    }
}

fn guardar(ui: &egui::Ui, v: Vec<Rect>) {
    let passada = ui.ctx().cumulative_pass_nr();
    ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("rotulos_3d"), (passada, v)));
}

/// Reserva o lugar de um balão (círculo de raio `rad` em `centro`) e devolve o centro livre.
pub fn reservar_balao(ui: &egui::Ui, centro: Pos2, rad: f32, viewport: Rect) -> Pos2 {
    let mut usados = lista(ui);
    let r = afasta(Rect::from_center_size(centro, egui::vec2(rad * 2., rad * 2.)), &usados, viewport);
    usados.push(r);
    guardar(ui, usados);
    r.center()
}

/// Antes de desenhar uma nota: onde pôr a origem dela para não cair em cima de outra já desenhada.
pub fn origem_livre(ui: &egui::Ui, p: Pos2) -> Pos2 {
    afasta_ponto(p, &lista(ui))
}

/// Depois de desenhar uma nota: o retângulo dela passa a valer para as seguintes.
pub fn registrar(ui: &egui::Ui, r: Rect) {
    let mut usados = lista(ui);
    usados.push(r);
    guardar(ui, usados);
}

#[cfg(test)]
mod testes {
    use super::*;

    fn r(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect::from_min_size(Pos2::new(x, y), egui::vec2(w, h))
    }

    #[test]
    fn balao_em_cima_de_outro_desce_e_livre_fica_onde_esta() {
        let vp = r(0., 0., 800., 600.);
        let usados = [r(100., 100., 30., 30.)];
        let livre = afasta(r(105., 105., 30., 30.), &usados, vp);
        assert!(!livre.expand(1.).intersects(usados[0]), "{livre:?}");
        assert_eq!(livre.left(), 105., "só desce");
        let ok = r(300., 300., 30., 30.);
        assert_eq!(afasta(ok, &usados, vp), ok);
    }

    #[test]
    fn varios_baloes_no_mesmo_ponto_ficam_todos_separados() {
        let vp = r(0., 0., 800., 600.);
        let mut usados: Vec<Rect> = vec![];
        for _ in 0..5 {
            let q = afasta(r(200., 200., 30., 30.), &usados, vp);
            assert!(usados.iter().all(|u| !u.intersects(q)), "{q:?} cobre algum");
            usados.push(q);
        }
    }

    #[test]
    fn sem_espaco_embaixo_sobe_e_dentro_da_viewport() {
        let vp = r(0., 0., 400., 100.);
        let usados = [r(0., 60., 400., 40.)];
        let q = afasta(r(10., 70., 30., 30.), &usados, vp);
        assert!(q.bottom() <= usados[0].top() && q.top() >= 0., "{q:?}");
    }

    #[test]
    fn nota_sobre_outra_vai_para_baixo_dela() {
        let usados = [r(50., 50., 120., 40.)];
        let p = afasta_ponto(Pos2::new(60., 55.), &usados);
        assert!(p.y >= 90., "{p:?}");
        assert_eq!(afasta_ponto(Pos2::new(300., 300.), &usados), Pos2::new(300., 300.));
    }
}
