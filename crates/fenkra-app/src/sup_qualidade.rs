// SPDX-License-Identifier: LGPL-2.1-or-later
//! B180: ajustes das análises de qualidade de superfície e o plano de corte dinâmico.
//!
//! - Zebra: o shader reflete a direção do olho na normal e conta faixas ao longo de um eixo do mundo.
//!   O número de faixas e o eixo são ajustáveis; o modo "reflexo" troca as faixas duras por um
//!   ambiente de tubos de luz suaves, que mostra o brilho correndo pela peça.
//! - Isofotas: linhas de mesmo `n·l`; o número de linhas e a direção da luz são ajustáveis.
//! - Plano de corte: corta a vista em qualquer análise, ao longo de X, Y ou Z, na posição do controle.
//!   No plano, a seção exata do kernel (`section_loops`) ganha o porco-espinho de curvatura: cada laço
//!   é partido nos cantos vivos e medido como curva, com a mesma conta do pente das curvas (B34).
//!
//! Nada aqui muda o documento: tudo é preferência de exibição.

use crate::i18n::tr;
use crate::render::{DisplayMode, MapaGpu, Scene};
use eframe::egui::{self, RichText};
use fenkra_solidos::analise::Dente;
use fenkra_solidos::geo3d::{add, cross, dot, mul, norm, sub, V3};
use glam::Vec3;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

/// Preferências das análises de superfície que não são o mapa de curvatura nem o de inclinação.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct AjustesAnalise {
    /// faixas da zebra por volta do reflexo
    pub zebra_faixas: f32,
    /// eixo do ambiente de listras (0 X, 1 Y, 2 Z)
    pub zebra_eixo: u8,
    /// tubos de luz suaves no lugar das faixas duras
    pub zebra_reflexo: bool,
    /// linhas das isofotas
    pub iso_linhas: f32,
    /// eixo da luz das isofotas (0 X, 1 Y, 2 Z, 3 oblíqua)
    pub iso_luz: u8,
    /// plano de corte dinâmico ligado
    pub corte: bool,
    /// normal do plano (0 X, 1 Y, 2 Z)
    pub corte_eixo: u8,
    /// posição do plano entre os limites da peça (0 a 1)
    pub corte_pos: f32,
    /// esconde o outro lado
    pub corte_inverte: bool,
    /// porco-espinho de curvatura na seção
    pub corte_pente: bool,
    /// altura do maior dente, em fração da diagonal da peça
    pub corte_altura: f32,
}

impl Default for AjustesAnalise {
    fn default() -> Self {
        AjustesAnalise { zebra_faixas: 9., zebra_eixo: 2, zebra_reflexo: false, iso_linhas: 14., iso_luz: 3, corte: false, corte_eixo: 0, corte_pos: 0.5, corte_inverte: false, corte_pente: true, corte_altura: 0.12 }
    }
}

fn eixo(i: u8) -> [f32; 3] {
    let mut d = [0.; 3];
    d[(i as usize).min(2)] = 1.;
    d
}

/// Luz das isofotas: um eixo do mundo ou a oblíqua de antes.
pub fn luz_das_isofotas(i: u8) -> [f32; 3] {
    if i >= 3 {
        let v = Vec3::new(0.35, -0.45, 0.82).normalize();
        return v.to_array();
    }
    eixo(i)
}

/// Parâmetros da zebra e das isofotas para o shader (`mapa.z` faixas, `mapa.w` estilo, `mapa_dir` eixo).
pub fn mapa_das_listras(s: &crate::settings::Settings) -> Option<MapaGpu> {
    let a = &s.analise_sup;
    let (n, estilo, dir) = match s.display {
        DisplayMode::Zebra => (a.zebra_faixas, if a.zebra_reflexo { 1. } else { 0. }, eixo(a.zebra_eixo)),
        DisplayMode::Isofotas => (a.iso_linhas, 0., luz_das_isofotas(a.iso_luz)),
        _ => return None,
    };
    Some(MapaGpu { tipo: 0., raio: 1., minimo: n.clamp(2., 60.), tampa: estilo, dir, paradas: [[0.; 3]; 4] })
}

/// Plano de corte em uso: normal e distância, no formato de `ViewStyle::secao` (o lado positivo some).
/// Só vale numa análise de superfície e com a peça na tela.
pub fn plano(s: &crate::settings::Settings, limites: Option<(Vec3, Vec3)>) -> Option<[f32; 4]> {
    let a = &s.analise_sup;
    if !a.corte || !s.display.e_analise() {
        return None;
    }
    let (lo, hi) = limites?;
    let k = (a.corte_eixo as usize).min(2);
    let d = lo[k] + (hi[k] - lo[k]) * a.corte_pos.clamp(0., 1.);
    // a seção fica um fio para dentro do lado visível: o laço desenhado no plano não some no corte
    let folga = (hi - lo).length() * 1e-4;
    let mut n = [0f32; 3];
    n[k] = if a.corte_inverte { -1. } else { 1. };
    let dist = if a.corte_inverte { -d } else { d } + folga;
    Some([n[0], n[1], n[2], dist])
}

/// Parte o laço onde a direção vira mais que `graus`: canto vivo não é curvatura. Cada trecho sai
/// com a marca de fechado (laço liso, sem canto).
pub fn partir_nos_cantos(laco: &[V3], graus: f64) -> Vec<(Vec<V3>, bool)> {
    let mut pts: Vec<V3> = vec![];
    for p in laco {
        if pts.last().is_none_or(|q| norm(sub(*p, *q)) > 1e-9) {
            pts.push(*p);
        }
    }
    let fechado = pts.len() > 2 && norm(sub(pts[0], *pts.last().unwrap())) < 1e-6;
    if fechado {
        pts.pop();
    }
    let n = pts.len();
    if n < 3 {
        return vec![];
    }
    let lim = graus.to_radians().cos();
    let canto = |i: usize| {
        let (a, b, c) = (pts[(i + n - 1) % n], pts[i], pts[(i + 1) % n]);
        let (u, v) = (sub(b, a), sub(c, b));
        let (nu, nv) = (norm(u), norm(v));
        nu > 0. && nv > 0. && dot(u, v) / (nu * nv) < lim
    };
    let cantos: Vec<usize> = (0..n).filter(|i| (fechado || (*i > 0 && *i < n - 1)) && canto(*i)).collect();
    if cantos.is_empty() {
        return vec![(pts, fechado)];
    }
    let mut out = vec![];
    if fechado {
        for (j, &c) in cantos.iter().enumerate() {
            let fim = cantos[(j + 1) % cantos.len()];
            let mut t = vec![pts[c]];
            let mut i = c;
            loop {
                i = (i + 1) % n;
                t.push(pts[i]);
                if i == fim {
                    break;
                }
            }
            out.push((t, false));
        }
    } else {
        let mut ini = 0;
        for &c in cantos.iter().chain(std::iter::once(&(n - 1))) {
            out.push((pts[ini..=c].to_vec(), false));
            ini = c;
        }
    }
    out
}

/// Curvatura no vértice `b`: o inverso do raio do círculo pelos vizinhos, e o lado de dentro.
/// Com os vizinhos de verdade (não uma janela fixa) a conta é exata para pontos sobre um arco,
/// qualquer que seja o espaçamento da discretização.
fn curvatura_no_vertice(a: V3, b: V3, c: V3) -> (f64, V3) {
    let (ab, cb) = (sub(a, b), sub(c, b));
    let n = norm(cross(ab, cb));
    let den = norm(ab) * norm(cb) * norm(sub(a, c));
    if den < 1e-18 || n < 1e-9 * den {
        return (0., [0.; 3]);
    }
    let meio = add(mul(ab, 1. / norm(ab)), mul(cb, 1. / norm(cb)));
    (2. * n / den, mul(meio, 1. / norm(meio).max(1e-18)))
}

/// Pente de um trecho poligonal: um dente por vértice; as pontas de um trecho aberto repetem o vizinho.
fn pente_poligonal(pts: &[V3], fechado: bool, esc: f64) -> Vec<Dente> {
    let n = pts.len();
    let mut ks: Vec<(f64, V3)> = (0..n)
        .map(|i| {
            if !fechado && (i == 0 || i == n - 1) {
                (0., [0.; 3])
            } else {
                curvatura_no_vertice(pts[(i + n - 1) % n], pts[i], pts[(i + 1) % n])
            }
        })
        .collect();
    if !fechado && n >= 3 {
        ks[0] = ks[1];
        ks[n - 1] = ks[n - 2];
    }
    let mut d: Vec<Dente> = pts.iter().zip(&ks).map(|(p, (k, dentro))| Dente { base: *p, ponta: sub(*p, mul(*dentro, k * esc)), k: *k }).collect();
    if fechado {
        d.push(d[0]);
    }
    d
}

/// Porco-espinho da seção: um pente por trecho liso de cada laço. A altura do maior dente é
/// `altura` vezes a diagonal; trecho reto não ganha dente.
pub fn pentes_da_secao(lacos: &[Vec<V3>], diagonal: f64, altura: f64) -> Vec<Vec<Dente>> {
    let trechos: Vec<(Vec<V3>, bool)> = lacos.iter().flat_map(|l| partir_nos_cantos(l, 25.)).filter(|t| t.0.len() >= 3).collect();
    let crus: Vec<Vec<Dente>> = trechos.iter().map(|(p, f)| pente_poligonal(p, *f, 1.)).collect();
    let kmax = crus.iter().flatten().fold(0f64, |m, d| m.max(d.k));
    if kmax < 1e-9 {
        return vec![];
    }
    let esc = altura * diagonal / kmax;
    trechos.iter().zip(&crus).filter(|(_, c)| c.iter().any(|d| d.k > kmax * 1e-3)).map(|((p, f), _)| pente_poligonal(p, *f, esc)).collect()
}

/// Laços da seção da peça, guardados enquanto o recálculo e o plano não mudam.
struct Guardado {
    geracao: u64,
    forma: Option<fenkra_kernel::Shape>,
    plano: [f32; 4],
    lacos: Vec<Vec<V3>>,
}

thread_local! {
    static GUARDADO: RefCell<Option<Guardado>> = const { RefCell::new(None) };
}

/// Laços da seção: fechados quando a peça é sólida; senão, as curvas onde o plano cruza as faces.
fn secao(s: &fenkra_kernel::Shape, o: V3, n: V3, meia: f64) -> Vec<Vec<V3>> {
    match s.section_loops(o, n, meia) {
        Ok(l) if !l.is_empty() => l,
        _ => fenkra_kernel::Shape::plane_face(o, n, meia).and_then(|p| s.section(&p)).unwrap_or_default(),
    }
}

fn lacos_da_secao(doc: &fenkra_core::Document, geracao: u64, p: [f32; 4], meia: f64) -> Vec<Vec<V3>> {
    GUARDADO.with(|g| {
        let mut g = g.borrow_mut();
        if g.as_ref().is_none_or(|x| x.geracao != geracao) {
            *g = Some(Guardado { geracao, forma: fenkra_solidos::final_shape(doc), plano: [f32::NAN; 4], lacos: vec![] });
        }
        let x = g.as_mut().expect("guardado");
        if x.plano != p {
            let n = [p[0] as f64, p[1] as f64, p[2] as f64];
            let o = [n[0] * p[3] as f64, n[1] * p[3] as f64, n[2] * p[3] as f64];
            x.lacos = x.forma.as_ref().map(|s| secao(s, o, n, meia)).unwrap_or_default();
            x.plano = p;
        }
        x.lacos.clone()
    })
}

/// Contorno da seção e porco-espinho na cena. `geracao` muda a cada recálculo.
pub fn decorar(s: &crate::settings::Settings, doc: &fenkra_core::Document, geracao: u64, v: &egui::Visuals, cena: &mut Scene) {
    let Some(p) = plano(s, cena.bounds) else { return };
    let Some((lo, hi)) = cena.bounds else { return };
    let diag = (hi - lo).length() as f64;
    let lacos = lacos_da_secao(doc, geracao, p, diag.max(1.) * 2.);
    let cor = crate::theme::accent(v);
    let c = [cor.r() as f32 / 255., cor.g() as f32 / 255., cor.b() as f32 / 255., 1.];
    for l in &lacos {
        let pts: Vec<[f32; 3]> = l.iter().map(|q| [q[0] as f32, q[1] as f32, q[2] as f32]).collect();
        crate::render::push_wide(&mut cena.wide, &pts, c, 2., 0);
    }
    if s.analise_sup.corte_pente {
        for d in pentes_da_secao(&lacos, diag, s.analise_sup.corte_altura as f64) {
            crate::sup_analise::pente_na_cena(&d, v, cena);
        }
    }
}

fn botoes_de_eixo(ui: &mut egui::Ui, valor: &mut u8, nomes: &[&'static str]) {
    for (i, n) in nomes.iter().enumerate() {
        // a letra do eixo não se traduz
        ui.selectable_value(valor, i as u8, if n.chars().count() > 1 { tr(n) } else { n });
    }
}

/// Controles do plano de corte, no fim de cada painel de análise.
pub fn controles_do_corte(ui: &mut egui::Ui, a: &mut AjustesAnalise) {
    ui.separator();
    ui.checkbox(&mut a.corte, tr("Plano de corte")).on_hover_text(tr("Corta a vista num plano que você move: mostra a seção da peça e o porco-espinho de curvatura no corte."));
    if !a.corte {
        return;
    }
    ui.horizontal(|ui| {
        botoes_de_eixo(ui, &mut a.corte_eixo, &["X", "Y", "Z"]);
        ui.checkbox(&mut a.corte_inverte, tr("Inverter")).on_hover_text(tr("Esconde o outro lado do plano."));
    });
    ui.add(crate::ui_ext::slider(&mut a.corte_pos, 0.0..=1.0).text(tr("Posição"))).on_hover_text(tr("Onde o plano corta, do começo ao fim da peça nesse eixo."));
    ui.checkbox(&mut a.corte_pente, tr("Porco-espinho na seção")).on_hover_text(tr("Dentes perpendiculares à seção, do tamanho da curvatura: dente que salta é curvatura que muda de repente."));
    if a.corte_pente {
        ui.add(crate::ui_ext::slider(&mut a.corte_altura, 0.02..=0.5).text(tr("Altura do pente"))).on_hover_text(tr("Altura do maior dente, em fração do tamanho da peça."));
    }
}

/// Conteúdo do painel da zebra.
pub fn controles_da_zebra(ui: &mut egui::Ui, a: &mut AjustesAnalise) {
    ui.add(crate::ui_ext::slider(&mut a.zebra_faixas, 2.0..=40.0).text(tr("Faixas"))).on_hover_text(tr("Quantas listras o ambiente tem: mais faixas mostram defeitos menores."));
    ui.horizontal(|ui| {
        ui.label(RichText::new(tr("Direção")).small());
        botoes_de_eixo(ui, &mut a.zebra_eixo, &["X", "Y", "Z"]);
    });
    ui.checkbox(&mut a.zebra_reflexo, tr("Reflexo suave")).on_hover_text(tr("Tubos de luz suaves no lugar das faixas: o brilho corre pela peça como num acabamento polido."));
}

/// Conteúdo do painel das isofotas.
pub fn controles_das_isofotas(ui: &mut egui::Ui, a: &mut AjustesAnalise) {
    ui.add(crate::ui_ext::slider(&mut a.iso_linhas, 2.0..=40.0).text(tr("Linhas"))).on_hover_text(tr("Quantas linhas de mesmo ângulo com a luz."));
    ui.horizontal(|ui| {
        ui.label(RichText::new(tr("Luz")).small());
        botoes_de_eixo(ui, &mut a.iso_luz, &["X", "Y", "Z", "Oblíqua"]);
    });
}

/// `FENKRA_DEMO_ANALISE=corte:<eixo>:<posição>,faixas:<n>,eixo:<n>,reflexo,luz:<n>,curv:<max|media|gauss|min>,sem_pente`
/// (desenvolvimento, para capturas sem mouse).
pub fn demo(spec: &str, s: &mut crate::settings::Settings) {
    let a = &mut s.analise_sup;
    for parte in spec.split(',') {
        let mut it = parte.split(':');
        let num = |x: Option<&str>| x.and_then(|v| v.parse::<f32>().ok());
        match it.next() {
            Some("corte") => {
                a.corte = true;
                a.corte_eixo = num(it.next()).unwrap_or(0.) as u8;
                a.corte_pos = num(it.next()).unwrap_or(0.5);
            }
            Some("faixas") => a.zebra_faixas = num(it.next()).unwrap_or(9.),
            Some("eixo") => a.zebra_eixo = num(it.next()).unwrap_or(2.) as u8,
            Some("reflexo") => a.zebra_reflexo = true,
            Some("luz") => a.iso_luz = num(it.next()).unwrap_or(3.) as u8,
            Some("linhas") => a.iso_linhas = num(it.next()).unwrap_or(14.),
            Some("inverte") => a.corte_inverte = true,
            Some("sem_pente") => a.corte_pente = false,
            Some("curv") => {
                s.curv_tipo = match it.next() {
                    Some("media") => crate::sup_analise::TipoCurvatura::Media,
                    Some("gauss") => crate::sup_analise::TipoCurvatura::Gaussiana,
                    Some("min") => crate::sup_analise::TipoCurvatura::Minima,
                    _ => crate::sup_analise::TipoCurvatura::Maxima,
                };
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quadrado_com_arco() -> Vec<V3> {
        // 40 × 20 com o lado direito trocado por meia-volta de raio 10
        let mut l = vec![[0., 0., 0.], [40., 0., 0.]];
        for i in 1..32 {
            let a = -std::f64::consts::FRAC_PI_2 + std::f64::consts::PI * i as f64 / 32.;
            l.push([40. + 10. * a.cos(), 10. + 10. * a.sin(), 0.]);
        }
        l.push([40., 20., 0.]);
        l.push([0., 20., 0.]);
        l.push([0., 0., 0.]);
        l
    }

    #[test]
    fn cantos_vivos_partem_o_laco_e_o_arco_fica_inteiro() {
        let t = partir_nos_cantos(&quadrado_com_arco(), 25.);
        // cantos em (0,0) e (0,20); a junção reta-arco é tangente e não parte
        assert_eq!(t.len(), 2, "{t:?}");
        let maior = t.iter().max_by_key(|x| x.0.len()).unwrap();
        assert!(maior.0.len() > 30, "o arco e as duas retas que o tocam ficam juntos");
        assert!(t.iter().all(|x| !x.1));
        // círculo sem canto: um trecho fechado
        let c: Vec<V3> = (0..=48).map(|i| i as f64 * std::f64::consts::TAU / 48.).map(|a| [5. * a.cos(), 5. * a.sin(), 0.]).collect();
        let t = partir_nos_cantos(&c, 25.);
        assert_eq!(t.len(), 1);
        assert!(t[0].1 && t[0].0.len() == 48);
        let p = pentes_da_secao(&[c], 10., 0.2);
        assert!(p[0].iter().all(|d| (d.k - 0.2).abs() < 1e-9), "raio 5, exato em qualquer vértice");
    }

    #[test]
    fn pente_da_secao_mede_o_raio_e_deixa_reta_sem_dente() {
        let p = pentes_da_secao(&[quadrado_com_arco()], 45., 0.1);
        assert_eq!(p.len(), 1, "só o trecho com o arco tem curvatura");
        let pico = fenkra_solidos::analise::pico(&p[0]).unwrap();
        assert!((pico.k - 0.1).abs() < 1e-9, "raio 10: k = {}", pico.k);
        // no meio do arco a curvatura é a do raio; nas retas, zero
        assert!(p[0].iter().filter(|d| d.base[0] > 45.).all(|d| (d.k - 0.1).abs() < 1e-9));
        assert!(p[0].iter().filter(|d| d.base[0] < 39. && d.base[0] > 1.).all(|d| d.k == 0.));
        // o maior dente tem a altura pedida: 10% da diagonal
        let alto = p[0].iter().map(|d| norm(sub(d.ponta, d.base))).fold(0., f64::max);
        assert!((alto - 4.5).abs() < 1e-6, "{alto}");
        // o dente aponta para fora da curva, longe do centro (40, 10)
        let c = [40., 10., 0.];
        assert!(norm(sub(pico.ponta, c)) > norm(sub(pico.base, c)));
    }

    #[test]
    fn plano_so_vale_em_analise_e_segue_a_posicao() {
        let mut s = crate::settings::Settings::default();
        let lim = Some((Vec3::ZERO, Vec3::new(10., 20., 30.)));
        s.analise_sup.corte = true;
        s.display = DisplayMode::Sombreado;
        assert!(plano(&s, lim).is_none(), "fora da análise o corte não aparece");
        s.display = DisplayMode::Zebra;
        s.analise_sup.corte_eixo = 1;
        s.analise_sup.corte_pos = 0.25;
        let p = plano(&s, lim).unwrap();
        assert_eq!(&p[..3], &[0., 1., 0.]);
        assert!((p[3] - 5.).abs() < 0.01);
        s.analise_sup.corte_inverte = true;
        let q = plano(&s, lim).unwrap();
        assert_eq!(&q[..3], &[0., -1., 0.]);
        assert!((q[3] + 5.).abs() < 0.01);
        assert!(plano(&s, None).is_none());
    }

    #[test]
    fn listras_vao_ao_shader_so_na_zebra_e_nas_isofotas() {
        let mut s = crate::settings::Settings::default();
        s.display = DisplayMode::Curvatura;
        assert!(mapa_das_listras(&s).is_none());
        s.display = DisplayMode::Zebra;
        s.analise_sup.zebra_faixas = 20.;
        s.analise_sup.zebra_eixo = 0;
        s.analise_sup.zebra_reflexo = true;
        let m = mapa_das_listras(&s).unwrap();
        assert_eq!((m.minimo, m.tampa, m.dir), (20., 1., [1., 0., 0.]));
        s.display = DisplayMode::Isofotas;
        s.analise_sup.iso_luz = 2;
        assert_eq!(mapa_das_listras(&s).unwrap().dir, [0., 0., 1.]);
    }

    #[test]
    fn ajustes_antigos_carregam_com_o_padrao() {
        let a: AjustesAnalise = serde_json::from_str("{\"zebra_faixas\": 12}").unwrap();
        assert_eq!(a.zebra_faixas, 12.);
        assert_eq!(a.corte_pos, 0.5);
    }
}
