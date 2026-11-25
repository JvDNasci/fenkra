// SPDX-License-Identifier: LGPL-2.1-or-later
//! Painel da aba por laço e extensão lateral (B284): quais arestas, que trecho de cada uma recebe
//! a aba e os valores comuns (comprimento, ângulo, raio, lado, mitra, alívio).

use crate::editor::{check, expr, section};
use crate::i18n::tr;
use eframe::egui::{self, RichText};
use fenkra_core::{Document, ExtensaoAba, FeatureKind, Id, SmFlangeLoop};
use fenkra_solidos::LacoDesc;
use std::cell::RefCell;
use std::collections::HashMap;

type Vars = HashMap<String, f64>;

thread_local! {
    /// Qual laço cada aba por laço pegou, do último cálculo (B579): a face de topo da chapa guarda os mesmos laços
    /// depois que a aba nasce, então o resultado final basta para dizer "contorno externo" ou "furo N".
    static LACOS: RefCell<HashMap<Id, LacoDesc>> = RefCell::new(HashMap::new());
}

/// Renova o laço de cada aba por laço do documento com o resultado do cálculo.
pub fn atualiza_lacos(doc: &Document, r: &fenkra_solidos::RecomputeResult) {
    let mut m = HashMap::new();
    for f in &doc.features {
        let FeatureKind::SmFlangeLoop(lp) = &f.kind else { continue };
        let Some(p) = lp.edges.first().filter(|p| **p != [0.; 3]) else { continue };
        if let Some(d) = r.bodies.iter().find_map(|b| fenkra_solidos::descreve_laco(&b.mesh, *p)) {
            m.insert(f.id, d);
        }
    }
    LACOS.with(|c| *c.borrow_mut() = m);
}

/// O texto do laço pego (contorno externo ou furo N), para o painel. `None` sem cálculo ou fora do modo laço.
pub fn texto_do_laco(d: &LacoDesc) -> String {
    let onde = if d.externo {
        tr("contorno externo").to_string()
    } else {
        format!("{} {} {} {}", tr("furo"), d.furo, tr("de"), d.furos)
    };
    format!("{}: {} ({} {}, {:.1} mm)", tr("Laço pego"), onde, d.arestas, tr(if d.arestas == 1 { "aresta" } else { "arestas" }), d.comprimento)
}

/// Linha do painel que diz qual laço a aresta escolhida pegou. Só com "usar o laço da face inteira" ligado.
pub fn quem(ui: &mut egui::Ui, me: Id, kind: &FeatureKind) {
    let FeatureKind::SmFlangeLoop(lp) = kind else { return };
    if !lp.loop_all {
        return;
    }
    let Some(d) = LACOS.with(|c| c.borrow().get(&me).cloned()) else { return };
    ui.label(RichText::new(texto_do_laco(&d)).small()).on_hover_text(tr("O laço segue a aresta clicada: clicar numa aresta do contorno externo leva a aba ao contorno todo; clicar na aresta de um furo leva a aba para dentro do furo. Para trocar, clique outra aresta."));
}

/// Modos da extensão lateral, na ordem do seletor.
const MODOS: [(&str, &str); 4] = [("Aresta inteira", "inteira"), ("Largura centrada", "centrada"), ("Largura deslocada", "deslocada"), ("Entre duas referências", "entre")];

fn modo(e: &ExtensaoAba) -> usize {
    match e {
        ExtensaoAba::Inteira => 0,
        ExtensaoAba::Centrada { .. } => 1,
        ExtensaoAba::Deslocada { .. } => 2,
        ExtensaoAba::Entre { .. } => 3,
    }
}

/// Extensão nova do modo `i`, aproveitando o que já havia (a largura passa de um modo a outro).
fn troca_modo(atual: &ExtensaoAba, i: usize) -> ExtensaoAba {
    let largura = match atual {
        ExtensaoAba::Centrada { largura } | ExtensaoAba::Deslocada { largura, .. } => largura.clone(),
        _ => "20".to_string(),
    };
    match i {
        1 => ExtensaoAba::Centrada { largura },
        2 => ExtensaoAba::Deslocada { inicio: "0".into(), largura },
        3 => ExtensaoAba::Entre { a: [0.; 3], b: [0.; 3] },
        _ => ExtensaoAba::Inteira,
    }
}

pub fn painel(ui: &mut egui::Ui, doc: &Document, lp: &mut SmFlangeLoop, vars: &Vars) {
    // o suporte (arestas escolhidas e laço da face) está no painel de modos: `chapa_unif_ui::suporte`
    section(ui, "Parâmetros");
    ui.label(RichText::new(tr("Extensão lateral")).weak());
    let mut m = modo(&lp.extent);
    ui.horizontal_wrapped(|ui| {
        for (i, (nome, _)) in MODOS.iter().enumerate() {
            ui.selectable_value(&mut m, i, tr(nome));
        }
    });
    if m != modo(&lp.extent) {
        lp.extent = troca_modo(&lp.extent, m);
    }
    match &mut lp.extent {
        ExtensaoAba::Inteira => {
            ui.label(RichText::new(tr("A aba cobre a aresta toda.")).weak().small());
        }
        ExtensaoAba::Centrada { largura } => {
            expr(ui, "Largura da aba", largura, vars, "mm");
        }
        ExtensaoAba::Deslocada { inicio, largura } => {
            expr(ui, "Início da aba", inicio, vars, "mm");
            expr(ui, "Largura da aba", largura, vars, "mm");
        }
        ExtensaoAba::Entre { a, b } => {
            ui.label(RichText::new(tr("Escolha dois vértices ou pontos no 3D: a aba cobre o trecho entre as projeções deles na aresta.")).weak().small());
            for (nome, p) in [("Referência 1", a), ("Referência 2", b)] {
                ui.horizontal(|ui| {
                    ui.label(tr(nome));
                    for v in p.iter_mut() {
                        ui.add(crate::ui_ext::dv(v).speed(0.1).dica("Coordenada do ponto de referência no 3D"));
                    }
                });
            }
        }
    }
    if !matches!(lp.extent, ExtensaoAba::Inteira | ExtensaoAba::Entre { .. }) {
        check(ui, &mut lp.invert, "medir a partir da outra ponta").on_hover_text(tr("A medida começa na ponta de menor X (depois Y, Z) de cada aresta; ligado, começa na outra."));
    }
    crate::editor::expr_lim(ui, "Comprimento reto", &mut lp.length, vars, "mm", Some((0., f64::INFINITY)));
    crate::editor::expr_lim(ui, "Ângulo", &mut lp.angle, vars, "°", Some((0.01, 180.)));
    let mut own = lp.radius.is_some();
    ui.horizontal(|ui| {
        if check(ui, &mut own, "raio próprio (senão o da chapa)").changed() {
            lp.radius = own.then(|| crate::chapa_unif_ui::raio_da_regra(doc));
        }
        if crate::chapa_estilo_ui::cadeado_comando(ui, doc, lp.radius.is_some()) {
            lp.radius = None;
        }
    });
    if let Some(r) = &mut lp.radius {
        crate::editor::expr_lim(ui, "Raio interno", r, vars, "mm", Some((0.01, f64::INFINITY)));
    }
    ui.horizontal_wrapped(|ui| {
        ui.label(tr("Lado do material"));
        for (k, n) in [(0u8, "dobra por fora"), (1, "material por fora"), (2, "material por dentro")] {
            ui.selectable_value(&mut lp.side, k, tr(n));
        }
    });
    crate::inverter::botao(ui, &mut lp.flip);
    // B579: o laço com trecho curvo vira aba curva e tem o mapa de conformação, como a aba curva e a por superfície
    crate::chapa_deformacao::opcao(ui);
    section(ui, "Avançado");
    check(ui, &mut lp.miter, "pontas em meia-esquadria (45°) para a aba vizinha");
    let mut rel = lp.relief.is_some();
    ui.horizontal(|ui| {
        if check(ui, &mut rel, "alívio de canto nas pontas recuadas").changed() {
            lp.relief = rel.then(|| "2".to_string());
        }
        if crate::chapa_estilo_ui::cadeado_comando(ui, doc, lp.relief.is_some()) {
            lp.relief = None;
        }
    });
    if let Some(w) = &mut lp.relief {
        expr(ui, "Largura do alívio", w, vars, "mm");
        ui.horizontal_wrapped(|ui| {
            ui.label(tr("Tipo"));
            for (k, n) in [(0u8, "retangular"), (1, "redondo"), (2, "rasgo")] {
                ui.selectable_value(&mut lp.relief_kind, k, tr(n));
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn o_painel_diz_qual_laco_a_aresta_pegou() {
        let ext = LacoDesc { externo: true, furo: 0, furos: 2, arestas: 3, comprimento: 3646.2 };
        assert_eq!(texto_do_laco(&ext), "Laço pego: contorno externo (3 arestas, 3646.2 mm)");
        let furo = LacoDesc { externo: false, furo: 2, furos: 2, arestas: 1, comprimento: 369.5 };
        assert_eq!(texto_do_laco(&furo), "Laço pego: furo 2 de 2 (1 aresta, 369.5 mm)");
    }

    #[test]
    fn troca_de_modo_aproveita_a_largura() {
        let c = ExtensaoAba::Centrada { largura: "35".into() };
        assert_eq!(troca_modo(&c, 2), ExtensaoAba::Deslocada { inicio: "0".into(), largura: "35".into() });
        assert_eq!(troca_modo(&c, 0), ExtensaoAba::Inteira);
        assert_eq!(modo(&troca_modo(&ExtensaoAba::Inteira, 3)), 3);
    }
}
