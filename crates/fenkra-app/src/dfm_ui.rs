// SPDX-License-Identifier: LGPL-2.1-or-later
//! B112, B113 e B197: o painel da análise de fabricação (DFM).
//!
//! O painel é livre, encaixável e só aparece enquanto a tela está em "Análise: DFM". Escolhe o perfil
//! do processo (fresagem, chapa, injeção, impressão 3D), varre a peça e lista as violações; clicar
//! numa linha realça a face no 3D. Os limites do perfil se editam ali mesmo e se guardam em
//! `%APPDATA%\Fenkra\perfis_dfm`, sem recompilar. O mapa de espessura pinta pontos da superfície com
//! as cores de estado do tema (fino em erro, grosso no destaque).
//!
//! O estado fica aqui, não em `App`: é só do painel e se perde ao fechar o programa, menos os perfis
//! guardados em arquivo.

use crate::app::App;
use crate::i18n::tr;
use crate::render::{MeshVertex, Scene};
use crate::sup_analise::{paradas, rampa};
use eframe::egui::{self, Color32, Rect, RichText};
use fenkra_solidos::dfm::{self, Perfil, Relatorio, Severidade};
use fenkra_solidos::dfm_geo;
use fenkra_solidos::RecomputeResult;
use std::cell::RefCell;

#[derive(Default)]
struct Estado {
    perfis: Vec<Perfil>,
    carregado: bool,
    erros_perfis: Vec<String>,
    escolhido: usize,
    rel: Option<Relatorio>,
    erro: Option<String>,
    mapa_espessura: bool,
    /// índice da violação escolhida na lista
    sel: Option<usize>,
    /// nome do perfil usado na última varredura: mudar de perfil esvazia o relatório
    varrido: String,
    /// `FENKRA_DEMO_DFM` já foi lido
    demo_lido: bool,
}

thread_local! {
    static ST: RefCell<Estado> = RefCell::new(Estado::default());
}

/// Pasta dos perfis do usuário.
pub fn pasta_perfis() -> Option<std::path::PathBuf> {
    std::env::var_os("APPDATA").map(|a| std::path::PathBuf::from(a).join("Fenkra").join("perfis_dfm"))
}

fn carrega(e: &mut Estado) {
    if e.carregado {
        return;
    }
    e.carregado = true;
    let (v, erros) = match pasta_perfis() {
        Some(p) => dfm::carregar_pasta(&p),
        None => (dfm::perfis_padrao(), vec![]),
    };
    e.perfis = v;
    e.erros_perfis = erros;
}

fn cor_sev(s: Severidade) -> Color32 {
    match s {
        Severidade::Erro => crate::theme::st_err(),
        Severidade::Aviso => crate::theme::st_warn(),
        Severidade::Info => crate::theme::st_ok(),
    }
}

fn rgb(c: Color32) -> [f32; 3] {
    [c.r() as f32 / 255., c.g() as f32 / 255., c.b() as f32 / 255.]
}

/// Cor da espessura: a faixa mais fina na última cor da escala (erro), a mais grossa na primeira.
fn cor_espessura(p: &[[f32; 3]; 4], v: f64, lo: f64, hi: f64) -> [f32; 3] {
    // uma faixa só: tudo na cor de destaque, sem sugerir problema
    let t = if hi > lo { ((v - lo) / (hi - lo)).clamp(0., 1.) } else { 1. };
    rampa(p, (1. - t) as f32)
}

fn varre(app: &App, e: &mut Estado) {
    e.erro = None;
    e.rel = None;
    e.sel = None;
    let Some(p) = e.perfis.get(e.escolhido).cloned() else { return };
    let Some(s) = fenkra_solidos::final_shape(app.doc()) else {
        e.erro = Some(tr("A peça não tem forma para medir.").to_string());
        return;
    };
    match dfm::varrer(&s, &p) {
        Ok(mut r) => {
            // chave estável da face: só quando há um corpo, para a ordem das faces ser a da malha
            if let Some(b) = app.result.as_ref().filter(|r| r.bodies.len() == 1).and_then(|r| r.bodies.first()) {
                for v in &mut r.violacoes {
                    if let Some(f) = v.faces.first() {
                        v.chave = b.face_key(*f);
                    }
                }
            }
            e.varrido = p.nome.clone();
            e.rel = Some(r);
        }
        Err(m) => e.erro = Some(m),
    }
}

/// Painel livre da análise de fabricação.
pub fn painel(app: &mut App, ctx: &egui::Context, area: Rect) {
    use crate::paineis::{self, Encaixe, Painel, Posicao};
    if app.settings.display != crate::render::DisplayMode::Dfm {
        return;
    }
    let cfg = Painel { nome: "analise_dfm".into(), titulo: tr("Análise de fabricação").to_string(), padrao: paineis::abaixo_do_cubo(area, 330.), largura: 330., redimensiona: false };
    let posicao = app.settings.painel(&cfg.nome, Encaixe::Livre);
    let pal = paradas(&ctx.global_style().visuals);
    let mut pedir = false;
    let saida = ST.with(|st| {
        let mut e = st.borrow_mut();
        carrega(&mut e);
        // captura sem mouse: FENKRA_DEMO_DFM=<parte do nome do perfil> escolhe o perfil, liga o mapa e varre
        if !e.demo_lido {
            e.demo_lido = true;
            if let Ok(n) = std::env::var("FENKRA_DEMO_DFM") {
                let n = n.to_lowercase();
                if let Some(k) = e.perfis.iter().position(|p| p.nome.to_lowercase().contains(&n)) {
                    e.escolhido = k;
                }
                e.mapa_espessura = true;
                pedir = true;
            }
        }
        paineis::mostrar(ctx, area, &cfg, posicao, |ui| {
            if e.perfis.is_empty() {
                ui.label(tr("Nenhum perfil de processo."));
                return;
            }
            let antes = e.escolhido;
            egui::ComboBox::from_label(tr("Processo")).selected_text(e.perfis[e.escolhido.min(e.perfis.len() - 1)].nome.clone()).show_ui(ui, |ui| {
                for i in 0..e.perfis.len() {
                    let n = e.perfis[i].nome.clone();
                    ui.selectable_value(&mut e.escolhido, i, n);
                }
            }).response.on_hover_text(tr("Cada processo traz as regras e os limites dele. Os limites se editam abaixo."));
            if e.escolhido != antes {
                e.rel = None;
                e.sel = None;
            }
            let i = e.escolhido.min(e.perfis.len() - 1);
            ui.horizontal(|ui| {
                ui.label(RichText::new(tr("Abertura ou construção")).small());
                let d = &mut e.perfis[i].direcao;
                let (mut eixo, mut inv) = (d.iter().position(|x| x.abs() > 0.5).unwrap_or(2), d.iter().any(|x| *x < -0.5));
                for (k, n) in ["X", "Y", "Z"].iter().enumerate() {
                    ui.selectable_value(&mut eixo, k, *n);
                }
                ui.checkbox(&mut inv, tr("Inverter")).on_hover_text(tr("Direção em que o molde abre, a ferramenta entra ou a impressão sobe. Vale para saída, undercut e saliência."));
                *d = [0.; 3];
                d[eixo] = if inv { -1. } else { 1. };
            });
            ui.horizontal(|ui| {
                if ui.button(tr("Varrer a peça")).on_hover_text(tr("Mede a peça contra as regras do processo e lista o que passa do limite.")).clicked() {
                    pedir = true;
                }
                ui.checkbox(&mut e.mapa_espessura, tr("Mapa de espessura")).on_hover_text(tr("Pinta pontos da superfície pela espessura local: fino em vermelho, grosso na cor de destaque."));
            });
            if let Some(m) = &e.erro {
                ui.colored_label(crate::theme::st_err(), m);
            }
            let sel_atual = e.sel;
            let mut nova_sel = sel_atual;
            if let Some(r) = &e.rel {
                if e.varrido != e.perfis[i].nome {
                    ui.label(RichText::new(tr("O perfil mudou: varra de novo.")).small().weak());
                }
                ui.label(RichText::new(format!("{} {}  ·  {} {}  ·  {} {}", r.contar(Severidade::Erro), tr("erros"), r.contar(Severidade::Aviso), tr("avisos"), r.contar(Severidade::Info), tr("informações"))).strong());
                for a in &r.avisos {
                    ui.label(RichText::new(a).small().weak());
                }
                if !r.espessura.is_empty() && e.mapa_espessura {
                    let f = dfm_geo::faixas_principais(&r.espessura, dfm::TOL_FAIXA, 0.02);
                    let (lo, hi) = (f.first().map_or(0., |x| x.media), f.last().map_or(0., |x| x.media));
                    for x in &f {
                        ui.horizontal(|ui| {
                            let (rc, _) = ui.allocate_exact_size(egui::vec2(14., 10.), egui::Sense::hover());
                            ui.painter().rect_filled(rc, 2., crate::theme::cor_da_escala(cor_espessura(&pal, x.media, lo, hi)));
                            ui.label(RichText::new(format!("{} mm", crate::app::fmt(x.media))).small().strong());
                            ui.label(RichText::new(format!("{} mm²", crate::app::fmt(x.area))).small().weak());
                        });
                    }
                }
                egui::ScrollArea::vertical().max_height(180.).show(ui, |ui| {
                    for (k, v) in r.violacoes.iter().enumerate() {
                        let ativa = nova_sel == Some(k);
                        let rot = format!("{}  {}", v.severidade.rotulo().chars().next().unwrap_or(' '), v.texto);
                        let resp = ui.selectable_label(ativa, RichText::new(rot).small().color(cor_sev(v.severidade)));
                        let ref_estavel = if v.chave.is_some() { tr("Referência estável à face: sobrevive ao recálculo.") } else { tr("Face sem referência estável (só posição).") };
                        let resp = resp.on_hover_text(format!("{}: {}\n{}", v.regra, v.faces.iter().map(|f| f.to_string()).collect::<Vec<_>>().join(", "), ref_estavel));
                        if resp.clicked() {
                            nova_sel = Some(k);
                        }
                    }
                });
            }
            e.sel = nova_sel;
            ui.separator();
            ui.collapsing(tr("Limites do perfil"), |ui| {
                for r in &mut e.perfis[i].regras {
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut r.ativa, "").on_hover_text(tr("Liga ou desliga a regra."));
                        ui.label(RichText::new(&r.id).small());
                        for (lim, nome) in [(&mut r.min, "mín"), (&mut r.max, "máx")] {
                            if let Some(v) = lim {
                                ui.add(crate::ui_ext::dv(v).speed(0.05).prefix(format!("{} ", tr(nome))).dica(tr("Limite da regra. Valor fora dele vira violação.")));
                            }
                        }
                    });
                }
                ui.horizontal(|ui| {
                    if ui.button(tr("Guardar perfil")).on_hover_text(tr("Grava o perfil na pasta do usuário; no próximo uso ele substitui o de fábrica de mesmo nome.")).clicked() {
                        if let Some(dir) = pasta_perfis() {
                            let _ = std::fs::create_dir_all(&dir);
                            let arq = dir.join(format!("{}.json", e.perfis[i].nome.replace(['/', '\\', ':', ' '], "_")));
                            if let Err(x) = std::fs::write(&arq, e.perfis[i].para_json()) {
                                e.erro = Some(format!("{}: {x}", arq.display()));
                            }
                        }
                    }
                    if ui.button(tr("Restaurar de fábrica")).on_hover_text(tr("Descarta as edições e volta aos perfis que vêm com o programa.")).clicked() {
                        e.perfis = dfm::perfis_padrao();
                        e.rel = None;
                    }
                });
            });
            for x in &e.erros_perfis {
                ui.label(RichText::new(x).small().color(crate::theme::st_warn()));
            }
        })
    });
    if pedir {
        ST.with(|st| {
            let mut e = st.borrow_mut();
            varre(app, &mut e);
        });
    }
    if let Some(m) = saida.movido {
        app.settings.guardar_painel(&cfg.nome, Posicao { ..m });
    }
}

/// Faces com problema, a violação escolhida e o mapa de espessura, na cena.
pub fn decorar(settings: &crate::settings::Settings, r: &RecomputeResult, v: &egui::Visuals, s: &mut Scene) {
    if settings.display != crate::render::DisplayMode::Dfm {
        return;
    }
    let Some(b) = r.bodies.first().filter(|_| r.bodies.len() == 1) else { return };
    let pal = paradas(v);
    ST.with(|st| {
        let e = st.borrow();
        let Some(rel) = &e.rel else { return };
        let m = &b.mesh;
        let escolhida: Vec<usize> = e.sel.and_then(|k| rel.violacoes.get(k)).map(|x| x.faces.clone()).unwrap_or_default();
        for (face, sev) in rel.faces_realcadas() {
            let ini = m.face_ranges.get(face).copied().unwrap_or(0) as usize;
            let fim = m.face_ranges.get(face + 1).map_or(m.indices.len(), |x| *x as usize);
            let c = rgb(cor_sev(sev));
            let alfa = if escolhida.contains(&face) { 0.95 } else { 0.55 };
            for &vi in m.indices.get(ini..fim).unwrap_or(&[]) {
                let k = vi as usize * 3;
                s.ghost.push(MeshVertex { pos: [m.positions[k], m.positions[k + 1], m.positions[k + 2]], nrm: [m.normals[k], m.normals[k + 1], m.normals[k + 2]], col: [c[0], c[1], c[2], alfa], mat: [1., 0., 1., 0.] });
            }
        }
        // marca de cada violação: uma cruz pequena no ponto
        let tam = rel_tamanho(m) * 0.012;
        for x in &rel.violacoes {
            let c = rgb(cor_sev(x.severidade));
            let p = [x.ponto[0] as f32, x.ponto[1] as f32, x.ponto[2] as f32];
            for eixo in 0..3 {
                let (mut a, mut z) = (p, p);
                a[eixo] -= tam;
                z[eixo] += tam;
                crate::render::push_wide(&mut s.wide, &[a, z], [c[0], c[1], c[2], 1.], 2.5, 0);
            }
        }
        if e.mapa_espessura && !rel.espessura.is_empty() {
            let f = dfm_geo::faixas_principais(&rel.espessura, dfm::TOL_FAIXA, 0.02);
            let (lo, hi) = (f.first().map_or(0., |x| x.media), f.last().map_or(0., |x| x.media));
            let tiny = rel_tamanho(m) * 0.002;
            for a in &rel.espessura {
                let c = cor_espessura(&pal, a.valor, lo, hi);
                let p = [a.ponto[0] as f32, a.ponto[1] as f32, a.ponto[2] as f32];
                crate::render::push_wide(&mut s.wide, &[p, [p[0], p[1], p[2] + tiny]], [c[0], c[1], c[2], 1.], 4., 0);
            }
        }
    });
}

/// Diagonal da caixa envolvente da malha, para o tamanho das marcas.
fn rel_tamanho(m: &fenkra_kernel::Mesh) -> f32 {
    let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
    for p in m.positions.chunks_exact(3) {
        for k in 0..3 {
            lo[k] = lo[k].min(p[k]);
            hi[k] = hi[k].max(p[k]);
        }
    }
    if hi[0] < lo[0] {
        return 1.;
    }
    ((hi[0] - lo[0]).powi(2) + (hi[1] - lo[1]).powi(2) + (hi[2] - lo[2]).powi(2)).sqrt().max(1e-3)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn espessura_fina_vai_para_o_erro_e_grossa_para_o_destaque() {
        let p = [[0., 0., 0.], [0.25, 0., 0.], [0.5, 0., 0.], [1., 0., 0.]];
        assert_eq!(cor_espessura(&p, 1., 1., 3.), p[3]);
        assert_eq!(cor_espessura(&p, 3., 1., 3.), p[0]);
        assert_eq!(cor_espessura(&p, 5., 5., 5.), p[0], "uma faixa só não quebra");
    }

    #[test]
    fn perfis_carregam_sem_pasta_do_usuario() {
        let mut e = Estado::default();
        carrega(&mut e);
        assert!(e.perfis.len() >= 4);
        assert!(e.perfis.iter().all(|p| !p.regras.is_empty()));
    }
}
