// SPDX-License-Identifier: LGPL-2.1-or-later
//! Marcas de centro associativas na interface (B294): modos simples, circular e linear, fantasma
//! enquanto se escolhem os furos, painel da marca selecionada e a opção de esconder por vista.
//! A geometria vem de `fenkra_desenho::centros`; aqui só se colhem os cliques.

use super::*;
use fenkra_desenho::centros::{self, ModoCentro};

/// Escolhas da ferramenta enquanto armada.
#[derive(Clone, Debug)]
pub struct EstadoCentro {
    pub modo: ModoCentro,
    /// quanto a cruz passa da borda do furo (mm na folha)
    pub extra: f64,
}

impl Default for EstadoCentro {
    fn default() -> Self {
        EstadoCentro { modo: ModoCentro::Simples, extra: 2. }
    }
}

/// Círculo debaixo do clique: o de borda mais perto (até 4 mm na folha) ou, dentro de um furo, o
/// menor círculo que contém o ponto. Devolve a vista e o centro no plano dela.
fn circulo_em(st: &mut DwgState, m: V2, so_vista: Option<Id>) -> Option<(Id, V2)> {
    let views: Vec<View> = st.d.views.iter().filter(|v| v.sheet == st.sheet && so_vista.is_none_or(|x| x == v.id)).cloned().collect();
    let mut melhor: Option<(f64, Id, V2)> = None;
    for v in &views {
        let p = st.models.proj(&st.d, v.id);
        let q = layout::from_sheet(v, &p, m);
        for (c, r) in centros::arcos(&p) {
            let dc = ((q[0] - c[0]).powi(2) + (q[1] - c[1]).powi(2)).sqrt();
            // distância à borda (na folha); dentro do furo pesa o raio, para o menor ganhar
            let borda = ((dc - r) * v.scale).abs();
            let peso = if dc <= r { r * v.scale * 0.01 } else { borda };
            if (borda < 4. || dc <= r) && melhor.as_ref().is_none_or(|b| peso < b.0) {
                melhor = Some((peso, v.id, c));
            }
        }
    }
    melhor.map(|b| (b.1, b.2))
}

/// Pontos já escolhidos no plano da vista (os cliques estão em mm na folha).
fn pontos(st: &mut DwgState) -> Vec<V2> {
    let Some(v) = st.tool_view.and_then(|id| st.d.view(id).cloned()) else { return vec![] };
    let p = st.models.proj(&st.d, v.id);
    st.clicks.iter().map(|c| layout::from_sheet(&v, &p, *c)).collect()
}

fn monta(st: &mut DwgState, vid: Id, pts: &[V2], modo: ModoCentro) -> Option<centros::MarcaCentro> {
    let p = st.models.proj(&st.d, vid);
    let mut m = centros::montar(Id(super::ghost::SCRATCH), vid, modo, pts, &p)?;
    m.extra = st.centro.extra;
    Some(m)
}

/// Clique na folha com a ferramenta armada: escolhe um furo; quando o modo tem furos de sobra
/// termina sozinho (simples: 1, linear: 2). Circular termina com Enter (3 ou mais).
pub fn clicar(st: &mut DwgState, m: V2) -> Result<Option<Id>, &'static str> {
    let so = st.tool_view;
    let Some((vid, _)) = circulo_em(st, m, so) else {
        return Err(if st.tool_view.is_some() { "Nenhum círculo perto do clique nessa vista" } else { "Nenhum arco ou círculo perto do clique" });
    };
    st.tool_view = Some(vid);
    // o mesmo furo duas vezes não conta
    let novo = {
        let v = st.d.view(vid).cloned().ok_or("Nenhum arco ou círculo perto do clique")?;
        let p = st.models.proj(&st.d, vid);
        let q = layout::from_sheet(&v, &p, m);
        let (_, c) = circulo_em(st, m, Some(vid)).ok_or("Nenhum arco ou círculo perto do clique")?;
        let ja = pontos(st).iter().any(|x| ((x[0] - c[0]).powi(2) + (x[1] - c[1]).powi(2)).sqrt() < 1e-6 || ((x[0] - q[0]).powi(2) + (x[1] - q[1]).powi(2)).sqrt() < 1e-9);
        !ja
    };
    if novo {
        st.clicks.push(m);
    }
    let modo = st.centro.modo;
    if modo != ModoCentro::Circular && st.clicks.len() >= modo.minimo() {
        return finalizar(st).map(Some);
    }
    Ok(None)
}

/// Cria a marca com os furos escolhidos. Erro em texto quando não dá (poucos furos, círculo de
/// furos impossível).
pub fn finalizar(st: &mut DwgState) -> Result<Id, &'static str> {
    let modo = st.centro.modo;
    let vid = st.tool_view.ok_or("Escolha os furos na vista")?;
    let pts = pontos(st);
    let mut mc = monta(st, vid, &pts, modo).ok_or(match modo {
        ModoCentro::Circular => "O círculo de furos pede três furos fora de uma reta",
        _ => "Escolha os furos na vista",
    })?;
    let id = st.d.alloc();
    mc.id = id;
    st.d.centers.push(mc);
    st.tool = None;
    st.clicks.clear();
    st.tool_view = None;
    st.sel = Some(id);
    Ok(id)
}

/// Enter na ferramenta: conclui o círculo de furos. `true` se a tecla foi usada.
pub fn terminar(st: &mut DwgState) -> bool {
    if st.tool != Some(c::CENTER) || st.centro.modo != ModoCentro::Circular || st.clicks.is_empty() {
        return false;
    }
    if let Err(e) = finalizar(st) {
        st.toast = Some((tr(e).to_string(), true));
    }
    true
}

/// Fantasma: a marca que sairia com os furos já escolhidos mais o que está sob o cursor. Faltando
/// furos para o modo, mostra a cruz de cada um.
pub fn fantasma(st: &mut DwgState, m: V2) -> Vec<Item> {
    let so = st.tool_view;
    let Some((vid, c)) = circulo_em(st, m, so) else {
        // sem círculo sob o cursor: só o que já foi escolhido
        let Some(vid) = st.tool_view else { return vec![] };
        let pts = pontos(st);
        return prever(st, vid, &pts);
    };
    let mut pts = if st.tool_view == Some(vid) { pontos(st) } else { vec![] };
    if !pts.iter().any(|x| ((x[0] - c[0]).powi(2) + (x[1] - c[1]).powi(2)).sqrt() < 1e-6) {
        pts.push(c);
    }
    prever(st, vid, &pts)
}

fn prever(st: &mut DwgState, vid: Id, pts: &[V2]) -> Vec<Item> {
    if pts.is_empty() {
        return vec![];
    }
    let modo = st.centro.modo;
    let mc = if pts.len() >= modo.minimo() { monta(st, vid, pts, modo) } else { None }.or_else(|| monta(st, vid, pts, ModoCentro::Simples));
    match mc {
        Some(mc) => layout::centro_items(&st.d, &mc, &mut st.models),
        None => vec![],
    }
}

/// Texto de ajuda do passo atual.
pub fn dica(st: &DwgState) -> &'static str {
    match (st.centro.modo, st.clicks.len()) {
        (ModoCentro::Simples, _) => "Clique na borda ou dentro de um furo da vista",
        (ModoCentro::Linear, 0) => "Clique o primeiro furo do eixo",
        (ModoCentro::Linear, _) => "Clique o segundo furo do eixo",
        (ModoCentro::Circular, n) if n < 3 => "Clique os furos do círculo de furos (três ou mais); Enter conclui",
        (ModoCentro::Circular, _) => "Clique mais furos ou Enter para concluir o círculo de furos",
    }
}

/// Barra de opções da ferramenta: modo e quanto a cruz passa da borda.
pub fn barra(ui: &mut egui::Ui, st: &mut DwgState) {
    for md in ModoCentro::ALL {
        let antes = st.centro.modo;
        if ui.selectable_label(st.centro.modo == md, tr(md.label())).on_hover_text(tr(match md {
            ModoCentro::Simples => "Cruz no centro de um furo",
            ModoCentro::Circular => "Escolha os furos de um círculo de furos: sai o círculo e a cruz de cada um",
            ModoCentro::Linear => "Eixo entre dois furos",
        })).clicked() {
            st.centro.modo = md;
            if antes != md {
                st.clicks.clear();
                st.tool_view = None;
            }
        }
    }
    ui.separator();
    ui.label(tr("Passa da borda (mm)")).on_hover_text(tr("quanto a linha de centro ultrapassa a borda do furo"));
    ui.add(crate::ui_ext::dv(&mut st.centro.extra).range(0.0..=10.).speed(0.1));
    if st.centro.modo == ModoCentro::Circular && !st.clicks.is_empty() && ui.button(tr("Concluir (Enter)")).clicked() {
        if let Err(e) = finalizar(st) {
            st.toast = Some((tr(e).to_string(), true));
        }
    }
}

/// Painel da marca selecionada. `true` se `id` era uma marca (e o painel foi mostrado).
pub fn painel(ui: &mut egui::Ui, st: &mut DwgState, id: Id) -> bool {
    let Some(i) = st.d.centers.iter().position(|c| c.id == id) else { return false };
    ui.heading(tr("Marca de centro"));
    let mc = &mut st.d.centers[i];
    ui.label(format!("{}: {}", tr("Modo"), tr(mc.modo.label())));
    ui.label(RichText::new(format!("{} {}", mc.anchors.len(), tr("furo(s) ligados à vista; acompanham a peça"))).weak().small());
    ui.horizontal(|ui| {
        ui.label(tr("Passa da borda (mm)")).on_hover_text(tr("quanto a linha de centro ultrapassa a borda do furo"));
        ui.add(crate::ui_ext::dv(&mut mc.extra).range(0.0..=10.).speed(0.1));
    });
    crate::editor::check(ui, &mut mc.visivel, "Visível");
    if ui.button(tr("🗑 Apagar")).clicked() {
        st.d.delete(id);
        st.sel = None;
    }
    true
}

/// Mostrar ou esconder todas as marcas de centro de uma vista.
pub fn visibilidade_da_vista(ui: &mut egui::Ui, ocultos: &mut Vec<Id>, marcas: &[centros::MarcaCentro], vid: Id) {
    if !marcas.iter().any(|c| c.view == vid) {
        return;
    }
    let mut on = !ocultos.contains(&vid);
    if crate::editor::check(ui, &mut on, "Marcas de centro visíveis").changed() {
        ocultos.retain(|x| *x != vid);
        if !on {
            ocultos.push(vid);
        }
    }
}
