// SPDX-License-Identifier: LGPL-2.1-or-later
//! B126: empilhamento de tolerâncias 1D a partir das cotas do PMI.
//!
//! O comando pega as cotas escolhidas (uma cadeia ou uma base são cotas soltas que se escolhem
//! juntas) e monta a cadeia: o nominal é a medida da cota no modelo, os desvios vêm da tolerância
//! dela (± e +/−, ou a classe ISO 286 como "H7" pelo `fenkra-tolerancias`) e o sentido sai da
//! direção de cada cota contra a da primeira, com troca de um clique. Cotas de referência (texto
//! entre parênteses) e teóricas (valor no quadro) ficam fora, com o motivo. O painel mostra o
//! pior caso, o RSS, o Monte Carlo e a contribuição de cada cota.

use crate::app::App;
use crate::i18n::tr;
use crate::paineis::{self, Encaixe, Painel};
use eframe::egui::{self, Rect, RichText};
use fenkra_core::pmi::{Pmi, PmiFrame, PmiKind};
use fenkra_core::Id;
use fenkra_tolerancias::empilhamento::{analisar, Elo};
use fenkra_tolerancias::ClasseTol;
use std::sync::Mutex;

/// Cadeia aberta no painel.
#[derive(Clone, Debug, Default)]
pub struct Estado {
    pub ids: Vec<Id>,
    /// sentido escolhido por cota (+1 ou −1); `None` = o deduzido da direção
    pub sentido: Vec<Option<f64>>,
    pub amostras: usize,
    pub lim_min: String,
    pub lim_max: String,
}

static ESTADO: Mutex<Option<Estado>> = Mutex::new(None);

fn travar() -> std::sync::MutexGuard<'static, Option<Estado>> {
    ESTADO.lock().unwrap_or_else(|e| e.into_inner())
}

/// Por que a cota fica fora do empilhamento (`None` = entra).
pub fn motivo_fora(p: &Pmi) -> Option<&'static str> {
    if !matches!(p.kind, PmiKind::Linear | PmiKind::Ordinate { .. } | PmiKind::Gap { .. } | PmiKind::Diameter | PmiKind::Radius) {
        return Some("não é cota linear");
    }
    let t = p.text.trim();
    if t.starts_with('(') && t.ends_with(')') {
        return Some("cota de referência");
    }
    if p.style.frame == PmiFrame::Caixa {
        return Some("cota teórica (valor no quadro)");
    }
    None
}

fn numero(s: &str) -> Option<f64> {
    s.trim().replace(',', ".").replace('−', "-").parse().ok()
}

/// Desvios (superior, inferior) da tolerância da cota para o nominal `n`.
pub fn desvios(p: &Pmi, n: f64) -> (f64, f64) {
    let (a, b) = (p.tol_plus.trim(), p.tol_minus.trim());
    if let Some(c) = ClasseTol::parse(a).or_else(|| ClasseTol::parse(b)) {
        if let Some((max, min)) = c.limites_mm(n) {
            return (max - n, min - n);
        }
    }
    match (numero(a), numero(b)) {
        // o campo de baixo guarda a magnitude ("0,05" é −0,05); um valor negativo já vem com sinal
        (Some(s), Some(i)) => (s, if i > 0. { -i } else { i }),
        (Some(s), None) => (s.abs(), -s.abs()),
        (None, Some(i)) => (0., -i.abs()),
        (None, None) => (0., 0.),
    }
}

fn direcao(p: &Pmi) -> Option<[f64; 3]> {
    let (a, b) = (p.anchors.first()?, p.anchors.get(1)?);
    Some([b[0] - a[0], b[1] - a[1], b[2] - a[2]])
}

/// Monta os elos: o sentido sai do produto da direção de cada cota com a da primeira.
pub fn elos(lista: &[Pmi], est: &Estado) -> (Vec<(Id, Elo)>, Vec<(Id, &'static str)>) {
    let mut dentro = vec![];
    let mut fora = vec![];
    let mut ref_dir: Option<[f64; 3]> = None;
    for (k, id) in est.ids.iter().enumerate() {
        let Some(p) = lista.iter().find(|p| p.id == *id) else { continue };
        if let Some(m) = motivo_fora(p) {
            fora.push((*id, m));
            continue;
        }
        let Some(n) = p.measure(&p.anchors) else {
            fora.push((*id, "sem medida no modelo"));
            continue;
        };
        let d = direcao(p);
        if ref_dir.is_none() {
            ref_dir = d;
        }
        let dedu = match (d, ref_dir) {
            (Some(d), Some(r)) if d[0] * r[0] + d[1] * r[1] + d[2] * r[2] < 0. => -1.,
            _ => 1.,
        };
        let sentido = est.sentido.get(k).copied().flatten().unwrap_or(dedu);
        let (sup, inf) = desvios(p, n.abs());
        let nome = if p.name.trim().is_empty() { p.display(Some(n)) } else { p.name.clone() };
        dentro.push((*id, Elo { nome, nominal: n.abs(), sup, inf, sentido }));
    }
    (dentro, fora)
}

/// Abre o painel com as cotas escolhidas (a seleção múltipla da árvore ou do 3D).
pub fn abrir(app: &mut App) {
    let mut ids: Vec<Id> = app.multi.borrow().clone();
    if let crate::app::Sel::Pmi(i) = app.sel {
        if !ids.contains(&i) {
            ids.push(i);
        }
    }
    let lista = crate::pmi_ui::list(app);
    ids.retain(|id| lista.iter().any(|p| p.id == *id && p.kind.is_dimension()));
    if ids.len() < 2 {
        app.toast(tr("Escolha ao menos duas cotas (Ctrl+clique na árvore ou no 3D) e chame o empilhamento").to_string(), true);
        return;
    }
    let n = ids.len();
    *travar() = Some(Estado { ids, sentido: vec![None; n], amostras: 10_000, lim_min: String::new(), lim_max: String::new() });
}

/// Abre o painel com as cotas dadas (exemplo de captura).
pub fn abrir_ids(ids: Vec<Id>, lim: (&str, &str)) {
    let n = ids.len();
    *travar() = Some(Estado { ids, sentido: vec![None; n], amostras: 10_000, lim_min: lim.0.into(), lim_max: lim.1.into() });
}

fn fmt(v: f64) -> String {
    let s = format!("{v:.4}");
    s.trim_end_matches('0').trim_end_matches('.').replace('.', ",")
}

/// Painel do empilhamento; chamado a cada quadro.
pub fn janela(app: &mut App, ctx: &egui::Context, area: Rect) {
    let Some(mut est) = travar().take() else { return };
    let lista: Vec<Pmi> = crate::pmi_ui::list(app).clone();
    let cfg = Painel { nome: "pmi_empilhamento".into(), titulo: tr("Empilhamento de tolerâncias").to_string(), padrao: paineis::ao_lado_da_arvore(area), largura: 340., redimensiona: true };
    let posicao = app.settings.painel("pmi_empilhamento", Encaixe::Livre);
    let mut fechar = false;
    let saida = paineis::mostrar(ctx, area, &cfg, posicao, |ui| {
        let (dentro, fora) = elos(&lista, &est);
        let lim = match (numero(&est.lim_min), numero(&est.lim_max)) {
            (Some(a), Some(b)) => Some((a.min(b), a.max(b))),
            _ => None,
        };
        let r = analisar(&dentro.iter().map(|x| x.1.clone()).collect::<Vec<_>>(), est.amostras, 1, lim);
        ui.label(RichText::new(tr("Cadeia")).strong());
        egui::Grid::new("empilhamento_elos").striped(true).show(ui, |ui| {
            for (i, (id, e)) in dentro.iter().enumerate() {
                let k = est.ids.iter().position(|x| x == id).unwrap_or(0);
                let rot = if e.sentido > 0. { "+" } else { "−" };
                if ui.button(rot).on_hover_text(tr("troca o sentido: a cota soma ou subtrai na cadeia")).clicked() {
                    est.sentido[k] = Some(-e.sentido);
                }
                ui.label(&e.nome);
                ui.label(format!("{} +{}/{}", fmt(e.nominal), fmt(e.sup), fmt(e.inf)));
                let c = r.contrib_rss.get(i).copied().unwrap_or(0.);
                ui.add(egui::ProgressBar::new(c as f32).desired_width(70.).text(format!("{:.0}%", c * 100.))).on_hover_text(tr("contribuição na variação do resultado (RSS)"));
                ui.end_row();
            }
        });
        for (id, m) in &fora {
            let nome = lista.iter().find(|p| p.id == *id).map(|p| p.name.clone()).unwrap_or_default();
            ui.label(RichText::new(format!("{nome}: {} ({})", tr("fica fora"), tr(m))).weak().small());
        }
        ui.separator();
        ui.label(RichText::new(format!("{} {}", tr("Nominal do resultado"), fmt(r.nominal))).strong());
        ui.label(format!("{}: {} … {}", tr("Pior caso"), fmt(r.pior.0), fmt(r.pior.1))).on_hover_text(tr("soma dos extremos: vale para 100% das peças"));
        ui.label(format!("{}: {} ± {}", tr("RSS"), fmt(r.rss_meio), fmt(r.rss_meia))).on_hover_text(tr("raiz da soma dos quadrados: cada cota com a meia-largura da zona como 3σ"));
        ui.horizontal(|ui| {
            ui.label(tr("Limites do resultado"));
            ui.add(egui::TextEdit::singleline(&mut est.lim_min).desired_width(55.).hint_text(tr("mín.")));
            ui.add(egui::TextEdit::singleline(&mut est.lim_max).desired_width(55.).hint_text(tr("máx.")));
        })
        .response
        .on_hover_text(tr("o que o projeto aceita para o resultado: o Monte Carlo conta as peças fora"));
        ui.horizontal(|ui| {
            ui.label(tr("Amostras do Monte Carlo"));
            let r = ui.add(crate::ui_ext::dv(&mut est.amostras).range(0..=200_000).speed(100.));
            crate::ui_ext::wheel(ui, &r, &mut est.amostras);
        });
        if let Some(mc) = &r.mc {
            ui.label(format!("{}: {} ± {} (σ)  ·  {} … {}", tr("Monte Carlo"), fmt(mc.media), fmt(mc.desvio), fmt(mc.min), fmt(mc.max)));
            if lim.is_some() {
                let cor = if mc.fora > 0.0027 { crate::theme::st_err() } else { crate::theme::st_ok() };
                ui.colored_label(cor, format!("{}: {:.2}%", tr("fora dos limites"), mc.fora * 100.));
            }
        }
        if let Some((a, b)) = lim {
            let ok = r.pior.0 >= a && r.pior.1 <= b;
            ui.colored_label(if ok { crate::theme::st_ok() } else { crate::theme::st_warn() }, tr(if ok { "O pior caso cabe nos limites" } else { "O pior caso passa dos limites: aperte as cotas de maior contribuição" }));
        }
        ui.separator();
        fechar = ui.button(tr("Fechar")).clicked();
    });
    if let Some(mut p) = saida.movido {
        fechar |= std::mem::take(&mut p.fechado);
        app.settings.guardar_painel("pmi_empilhamento", p);
    }
    if !fechar {
        *travar() = Some(est);
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn cota(id: u64, a: [f64; 3], b: [f64; 3], mais: &str, menos: &str) -> Pmi {
        Pmi { id: Id(id), name: format!("c{id}"), kind: PmiKind::Linear, anchors: vec![a, b], offset: [0.; 3], text: String::new(), tol_plus: mais.into(), tol_minus: menos.into(), visible: true, plane: None, style: Default::default(), folder: String::new(), targets: vec![] }
    }

    /// Caixa de 60 com três peças de 20 em cadeia (cotadas no sentido contrário): folga 0, com a
    /// classe H7 na caixa, ± nas peças, e a cota teórica e a de referência fora.
    #[test]
    fn cadeia_do_pmi_vira_elos_com_sentido_e_desvios() {
        let mut lista = vec![
            cota(1, [0., 0., 0.], [60., 0., 0.], "H7", ""),
            cota(2, [60., 0., 0.], [40., 0., 0.], "0,05", "0,05"),
            cota(3, [40., 0., 0.], [20., 0., 0.], "0,05", ""),
            cota(4, [20., 0., 0.], [0., 0., 0.], "+0,02", "0,03"),
            cota(5, [0., 0., 0.], [10., 0., 0.], "", ""),
            cota(6, [0., 0., 0.], [5., 0., 0.], "", ""),
        ];
        lista[4].style.frame = PmiFrame::Caixa;
        lista[5].text = "(5)".into();
        let est = Estado { ids: (1..=6).map(Id).collect(), sentido: vec![None; 6], amostras: 0, ..Default::default() };
        let (dentro, fora) = elos(&lista, &est);
        assert_eq!(dentro.len(), 4);
        assert_eq!(fora.iter().map(|f| f.0).collect::<Vec<_>>(), vec![Id(5), Id(6)]);
        assert_eq!(dentro.iter().map(|d| d.1.sentido).collect::<Vec<_>>(), vec![1., -1., -1., -1.]);
        // H7 em 60: 0 / +0,030
        assert!((dentro[0].1.sup - 0.030).abs() < 1e-9 && dentro[0].1.inf.abs() < 1e-9, "{:?}", dentro[0].1);
        assert_eq!((dentro[1].1.sup, dentro[1].1.inf), (0.05, -0.05));
        assert_eq!((dentro[2].1.sup, dentro[2].1.inf), (0.05, -0.05), "um só campo é simétrico");
        assert_eq!((dentro[3].1.sup, dentro[3].1.inf), (0.02, -0.03));
        let r = analisar(&dentro.iter().map(|d| d.1.clone()).collect::<Vec<_>>(), 0, 1, None);
        assert!(r.nominal.abs() < 1e-9);
        // pior caso: 0 + 0,030 + 0,05 + 0,05 + 0,03 = 0,16 de folga máxima; −(0,05+0,05+0,02) = −0,12
        assert!((r.pior.1 - 0.16).abs() < 1e-9 && (r.pior.0 + 0.12).abs() < 1e-9, "{:?}", r.pior);
        // trocar o sentido de uma cota à mão
        let mut est2 = est.clone();
        est2.sentido[1] = Some(1.);
        assert_eq!(elos(&lista, &est2).0[1].1.sentido, 1.);
    }
}
