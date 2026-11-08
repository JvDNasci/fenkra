// SPDX-License-Identifier: LGPL-2.1-or-later
//! B38: etiquetas do consultor de tolerância junto da anotação afetada, no 3D.
//!
//! Cada anotação com achado ganha uma etiqueta logo abaixo do rótulo: a marca da gravidade, o
//! diagnóstico e o que fazer. Passar o cursor realça os pontos da anotação e o traço até ela;
//! clicar na etiqueta enquadra a anotação e a seleciona; quando a regra tem uma saída única, o
//! botão de correção resolve com um clique (desfazível). As regras e as correções moram em
//! `fenkra_core::gdt`; aqui só se desenha e se aplica.

use crate::app::{App, Sel};
use crate::i18n::tr;
use crate::render::Camera;
use eframe::egui::{self, Pos2, Rect, RichText, Stroke};
use fenkra_core::gdt::{consultar, Achado, Gravidade, Norma};
use fenkra_core::pmi::{Pmi, PmiKind};
use fenkra_core::Id;
use glam::Vec3;

/// Etiquetas ligadas (a mesma chave do painel do consultor guarda a norma).
fn norma(ctx: &egui::Context) -> Norma {
    ctx.data(|d| d.get_temp::<Norma>(egui::Id::new("pmi_consultor_norma_estado"))).unwrap_or_default()
}

/// Ponto do rótulo da anotação no modelo (primeira âncora mais o deslocamento do texto).
pub fn ponto_do_rotulo(p: &Pmi) -> Option<[f64; 3]> {
    let a = p.anchors.first()?;
    Some([a[0] + p.offset[0], a[1] + p.offset[1], a[2] + p.offset[2]])
}

/// Caixa que enquadra a anotação: as âncoras e o rótulo, com folga.
pub fn caixa(p: &Pmi) -> Option<(Vec3, Vec3)> {
    let pts: Vec<[f64; 3]> = p.anchors.iter().copied().chain(ponto_do_rotulo(p)).collect();
    let mut lo = Vec3::splat(f32::MAX);
    let mut hi = Vec3::splat(f32::MIN);
    for q in &pts {
        let v = Vec3::new(q[0] as f32, q[1] as f32, q[2] as f32);
        lo = lo.min(v);
        hi = hi.max(v);
    }
    if pts.is_empty() {
        return None;
    }
    let folga = ((hi - lo).length() * 0.5).max(10.);
    Some((lo - Vec3::splat(folga), hi + Vec3::splat(folga)))
}

/// O achado mais grave de cada anotação (a lista do consultor já vem do mais grave ao mais leve).
pub fn por_anotacao(achados: &[Achado]) -> Vec<(Id, &Achado, usize)> {
    let mut out: Vec<(Id, &Achado, usize)> = vec![];
    for a in achados {
        match out.iter_mut().find(|x| x.0 == a.id) {
            Some(x) => x.2 += 1,
            None => out.push((a.id, a, 1)),
        }
    }
    out
}

fn v3(p: [f64; 3]) -> Vec3 {
    Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32)
}

/// Desenha as etiquetas e trata clique, realce e correção.
pub fn etiquetas(app: &mut App, ui: &mut egui::Ui, rect: Rect, cam: &Camera) {
    let lista: Vec<Pmi> = crate::pmi_ui::list(app).iter().filter(|p| p.visible).cloned().collect();
    if !lista.iter().any(|p| matches!(p.kind, PmiKind::Gdt { .. } | PmiKind::Datum { .. })) {
        return;
    }
    let achados = consultar(&lista, norma(ui.ctx()));
    let mut enquadrar: Option<Id> = None;
    let mut corrigir: Option<(Id, fenkra_core::gdt::Correcao)> = None;
    let painter = ui.painter_at(rect);
    for (id, a, n) in por_anotacao(&achados) {
        // a referência não usada é só informação: não polui a cena
        if a.aviso.gravidade == Gravidade::Info {
            continue;
        }
        let Some(p) = lista.iter().find(|p| p.id == id) else { continue };
        let Some(base) = ponto_do_rotulo(p).and_then(|q| cam.project(rect, v3(q))) else { continue };
        let pos = base + egui::vec2(-8., 16.);
        if !rect.contains(pos) {
            continue;
        }
        let (cor, marca) = match a.aviso.gravidade {
            Gravidade::Erro => (crate::theme::st_err(), "✖"),
            _ => (crate::theme::st_warn(), "⚠"),
        };
        let mais = if n > 1 { format!("  (+{})", n - 1) } else { String::new() };
        let area = egui::Area::new(egui::Id::new(("pmi_consultor_etiqueta", id.0))).order(egui::Order::Middle).fixed_pos(pos).constrain_to(rect);
        let r = area.show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style()).inner_margin(egui::Margin::symmetric(6, 3)).stroke(Stroke::new(1.2, cor)).show(ui, |ui| {
                ui.set_max_width(260.);
                let topo = ui.add(egui::Label::new(RichText::new(format!("{marca} {}{mais}", tr(&a.aviso.msg))).color(cor).small()).sense(egui::Sense::click()));
                let acao = (!a.aviso.acao.is_empty()).then(|| ui.add(egui::Label::new(RichText::new(format!("→ {}", tr(&a.aviso.acao))).small()).sense(egui::Sense::click())));
                let mut clicou = topo.clicked() || acao.as_ref().is_some_and(|x| x.clicked());
                if let Some(c) = &a.aviso.correcao {
                    if ui.small_button(format!("✔ {}", tr(&c.label()))).on_hover_text(tr("corrige o quadro com um clique (Ctrl+Z desfaz)")).clicked() {
                        corrigir = Some((id, c.clone()));
                        clicou = false;
                    }
                }
                clicou
            })
            .inner
        });
        if r.inner {
            enquadrar = Some(id);
        }
        let quente = r.response.hovered() || r.response.contains_pointer();
        if quente {
            // realce do alvo: os pontos da anotação e o traço da etiqueta até o rótulo
            let acc = crate::theme::accent(ui.visuals());
            painter.line_segment([pos, base], Stroke::new(1.5, acc));
            for q in &p.anchors {
                if let Some(s) = cam.project(rect, v3(*q)) {
                    painter.circle_stroke(s, 7., Stroke::new(2.5, acc));
                }
            }
            r.response.on_hover_text(tr("clique para enquadrar a anotação"));
        }
    }
    if let Some((id, c)) = corrigir {
        aplicar(app, id, &c);
    }
    if let Some(id) = enquadrar {
        if let Some(b) = lista.iter().find(|p| p.id == id).and_then(caixa) {
            app.enquadrar_caixa(b);
        }
        app.pmi_focus = Some(id);
        if app.in_asm() {
            if let Some(st) = app.asm.as_mut() {
                st.sel = Some(id);
            }
        } else {
            app.sel = Sel::Pmi(id);
        }
    }
}

/// Aplica a correção ao quadro da anotação, com desfazer.
pub fn aplicar(app: &mut App, id: Id, c: &fenkra_core::gdt::Correcao) -> bool {
    let antes = (!app.in_asm()).then(|| app.open.doc.clone());
    let mudou = crate::pmi_ui::list_mut(app).iter_mut().find(|p| p.id == id).is_some_and(|p| corrigir_kind(&mut p.kind, c));
    if mudou {
        if let Some(a) = antes {
            app.push_undo_pub(a);
        }
        app.toast(format!("{}: {}", tr("Corrigido"), tr(&c.label())), false);
    }
    mudou
}

/// A correção no tipo da anotação (só quadros de tolerância têm o que corrigir).
pub fn corrigir_kind(k: &mut PmiKind, c: &fenkra_core::gdt::Correcao) -> bool {
    match k {
        PmiKind::Gdt { datums, extra, .. } => c.aplicar(datums, extra),
        _ => false,
    }
}

/// O ponto da tela de uma anotação, para testes e para quem precisa apontar para ela.
#[allow(dead_code)]
pub fn na_tela(p: &Pmi, rect: Rect, cam: &Camera) -> Option<Pos2> {
    ponto_do_rotulo(p).and_then(|q| cam.project(rect, v3(q)))
}

#[cfg(test)]
mod testes {
    use super::*;
    use fenkra_core::gdt::{Correcao, Modif};
    use fenkra_core::pmi::{GdtChar, GdtExtra};

    fn quadro(id: u64, ch: GdtChar, tol: &str, datums: &str, extra: GdtExtra) -> Pmi {
        Pmi { id: Id(id), name: format!("q{id}"), kind: PmiKind::Gdt { ch, tol: tol.into(), datums: datums.into(), extra }, anchors: vec![[0., 0., 0.]], offset: [10., 0., 10.], text: String::new(), tol_plus: String::new(), tol_minus: String::new(), visible: true, plane: None, style: Default::default(), folder: String::new(), targets: vec![] }
    }

    /// B38: toda regra que dispara diz o que fazer, e a correção de um clique deixa o quadro válido.
    #[test]
    fn todo_achado_tem_acao_e_a_correcao_resolve() {
        let com = |f: &dyn Fn(&mut GdtExtra)| {
            let mut e = GdtExtra::default();
            f(&mut e);
            e
        };
        let casos = vec![
            quadro(1, GdtChar::Planeza, "0,05", "A", GdtExtra::default()),
            quadro(2, GdtChar::Planeza, "0,05", "", com(&|e| e.mods = vec![Modif::Mmc])),
            quadro(3, GdtChar::Planeza, "0,05", "", com(&|e| e.zone_dia = true)),
            quadro(4, GdtChar::Perpendicularidade, "0,05", "", GdtExtra::default()),
            quadro(5, GdtChar::Posicao, "0,1", "A A", com(&|e| e.elemento = fenkra_core::gdt::Elemento::Mediano)),
            quadro(6, GdtChar::Posicao, "x", "Z", GdtExtra::default()),
            quadro(7, GdtChar::Retitude, "0,05", "", com(&|e| e.max_valor = "0,1".into())),
        ];
        let mut ref_a = quadro(8, GdtChar::Planeza, "0,01", "", GdtExtra::default());
        ref_a.kind = PmiKind::Datum { letter: "A".into() };
        let mut lista = casos.clone();
        lista.push(ref_a);
        let achados = consultar(&lista, Norma::Iso);
        assert!(!achados.is_empty());
        for a in &achados {
            assert!(!a.aviso.acao.trim().is_empty(), "sem ação: {}", a.aviso.msg);
        }
        // as correções de um clique deixam o quadro sem o erro que as pediu
        let mut corrigidos = 0;
        for a in achados.iter().filter(|a| a.aviso.correcao.is_some()) {
            let mut p = lista.iter().find(|p| p.id == a.id).unwrap().clone();
            assert!(corrigir_kind(&mut p.kind, a.aviso.correcao.as_ref().unwrap()), "{:?} não mudou nada", a.aviso.correcao);
            let mut l2 = lista.clone();
            let pid = p.id;
            *l2.iter_mut().find(|x| x.id == pid).unwrap() = p;
            let depois = consultar(&l2, Norma::Iso);
            assert!(!depois.iter().any(|d| d.id == a.id && d.aviso.msg == a.aviso.msg), "a correção {:?} não resolveu: {}", a.aviso.correcao, a.aviso.msg);
            corrigidos += 1;
        }
        assert!(corrigidos >= 4, "{corrigidos}");
        // exemplos do item: modificador inválido sai com um clique; forma perde as referências
        assert!(achados.iter().any(|a| a.id == Id(2) && a.aviso.correcao == Some(Correcao::TirarModificador(Modif::Mmc))));
        assert!(achados.iter().any(|a| a.id == Id(1) && a.aviso.correcao == Some(Correcao::TirarReferencias)));
    }

    #[test]
    fn caixa_do_enquadrar_contem_a_ancora_e_o_rotulo() {
        let p = quadro(1, GdtChar::Planeza, "0,05", "", GdtExtra::default());
        let (lo, hi) = caixa(&p).unwrap();
        for q in [Vec3::ZERO, Vec3::new(10., 0., 10.)] {
            assert!(q.cmpge(lo).all() && q.cmple(hi).all());
        }
        let achados = vec![
            Achado { id: Id(1), aviso: fenkra_core::gdt::Aviso { gravidade: Gravidade::Erro, msg: "a".into(), acao: "b".into(), correcao: None } },
            Achado { id: Id(1), aviso: fenkra_core::gdt::Aviso { gravidade: Gravidade::Aviso, msg: "c".into(), acao: "d".into(), correcao: None } },
        ];
        let g = por_anotacao(&achados);
        assert_eq!(g.len(), 1);
        assert_eq!(g[0].2, 2, "conta os outros achados da mesma anotação");
    }
}
