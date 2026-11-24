// SPDX-License-Identifier: LGPL-2.1-or-later
//! B579: mapa de deformação da aba de linha de dobra curva, no 3D.
//!
//! Como funciona. O recálculo da aba (`fenkra_solidos::chapa_hidro::analisar`) mede, em cada ponto da linha
//! de dobra, a deformação circunferencial da borda livre (`ε`, negativa na contração, positiva no
//! estiramento), a espessura final (`t'/t`) e o risco (ok, atenção, falha). Aqui essa medida vira cor por
//! vértice da malha da aba:
//!
//! - cada vértice da aba procura o ponto mais perto da linha de dobra (grade espacial, sem varrer todos);
//! - a deformação cresce da dobra para a borda livre (uma fibra a distância `d` da dobra se deforma
//!   `ε·d/d_máx` em primeira ordem, e `ε` é a da borda livre, onde `d = d_máx`), então a dobra fica na cor
//!   neutra e a borda mostra o pior;
//! - o risco do vértice sai do mesmo critério do aviso (`chapa_hidro::classifica`), e a cor da faixa vem do
//!   tema, nunca fixa: azul de destaque do tema na contração sem risco, verde (estado ok) onde está bom,
//!   amarelo (aviso) na atenção e vermelho (erro) onde enruga ou trinca. Com a paleta segura (B321) as
//!   quatro cores viram a escala de luminância monótona.
//!
//! A cor vai direto em `MeshVertex::col`, como a de qualquer face; o pipeline do wgpu não muda. O resto da
//! peça mantém a cor de peça do tema (distinta do fundo). A opção "Mostrar deformação" mora no painel dos
//! comandos de aba, vive nas preferências e a legenda abre num painel livre enquanto ela está ligada.

use crate::i18n::tr;
use eframe::egui::{self, Rect, RichText};
use fenkra_core::Id;
use fenkra_solidos::chapa_hidro::{self, Analise, Regime, Risco};
use std::collections::HashMap;

/// Contração a partir da qual um ponto sem risco aparece em azul (0,2 %): abaixo disso a aba fica na cor ok.
pub const CONTRACAO_VISIVEL: f64 = -0.002;

/// As quatro faixas do mapa, na ordem das paradas de cor do tema (`sup_analise::paradas`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Faixa {
    /// a borda contrai sem risco (a chapa espessa um pouco)
    Contracao = 0,
    Ok = 1,
    Atencao = 2,
    /// enruga (compressão) ou trinca (tração)
    Falha = 3,
}

impl Faixa {
    pub const TODAS: [Faixa; 4] = [Faixa::Contracao, Faixa::Ok, Faixa::Atencao, Faixa::Falha];

    /// Faixa de uma deformação `eps` da borda livre, pelo critério do material da análise.
    pub fn de(eps: f64, a: &Analise) -> Faixa {
        let (regime, risco, _) = chapa_hidro::classifica(eps, &a.aba, &a.lim);
        match risco {
            Risco::Falha => Faixa::Falha,
            Risco::Atencao => Faixa::Atencao,
            Risco::Ok if regime == Regime::Contracao && eps <= CONTRACAO_VISIVEL => Faixa::Contracao,
            Risco::Ok => Faixa::Ok,
        }
    }

    pub fn cor(self, paradas: &[[f32; 3]; 4]) -> [f32; 3] {
        paradas[self as usize]
    }
}

/// Grade espacial dos pontos da linha de dobra: o ponto mais perto de um vértice sem varrer a linha toda.
struct Grade {
    celula: f64,
    min: [f64; 3],
    celulas: HashMap<[i32; 3], Vec<u32>>,
}

impl Grade {
    fn nova(pts: &[[f64; 3]]) -> Grade {
        let mut min = [f64::MAX; 3];
        let mut max = [f64::MIN; 3];
        for p in pts {
            for k in 0..3 {
                min[k] = min[k].min(p[k]);
                max[k] = max[k].max(p[k]);
            }
        }
        // célula com alguns pontos: a linha tem milhares, a caixa dela é grande
        let diag = (0..3).map(|k| (max[k] - min[k]).powi(2)).sum::<f64>().sqrt().max(1e-6);
        let celula = (diag / (pts.len() as f64).sqrt().max(4.)).max(1e-3);
        let mut g = Grade { celula, min, celulas: HashMap::new() };
        for (i, p) in pts.iter().enumerate() {
            g.celulas.entry(g.chave(*p)).or_default().push(i as u32);
        }
        g
    }

    fn chave(&self, p: [f64; 3]) -> [i32; 3] {
        [0, 1, 2].map(|k| ((p[k] - self.min[k]) / self.celula).floor() as i32)
    }

    /// Índice e distância do ponto da linha mais perto de `q`: anéis de células até o anel passar do melhor.
    fn mais_perto(&self, pts: &[[f64; 3]], q: [f64; 3]) -> Option<(usize, f64)> {
        let c = self.chave(q);
        let mut melhor: Option<(usize, f64)> = None;
        for anel in 0..4096i32 {
            // o anel `anel` está a pelo menos `(anel - 1)·celula` do ponto
            if let Some((_, d)) = melhor {
                if (anel - 1).max(0) as f64 * self.celula > d {
                    break;
                }
            }
            for dx in -anel..=anel {
                for dy in -anel..=anel {
                    for dz in -anel..=anel {
                        if dx.abs().max(dy.abs()).max(dz.abs()) != anel {
                            continue;
                        }
                        let Some(v) = self.celulas.get(&[c[0] + dx, c[1] + dy, c[2] + dz]) else { continue };
                        for &i in v {
                            let p = pts[i as usize];
                            let d = ((p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2) + (p[2] - q[2]).powi(2)).sqrt();
                            if melhor.is_none_or(|(_, b)| d < b) {
                                melhor = Some((i as usize, d));
                            }
                        }
                    }
                }
            }
            if self.celulas.is_empty() {
                return None;
            }
        }
        melhor
    }
}

/// O mapa de uma aba: a análise, a grade dos pontos e a distância da borda livre à linha de dobra.
pub struct Campo<'a> {
    analise: &'a Analise,
    pts: Vec<[f64; 3]>,
    grade: Grade,
    /// distância (mm) da borda livre à linha de dobra; é onde a deformação do ponto vale por inteiro
    pub dmax: f64,
}

impl<'a> Campo<'a> {
    /// `dmax`: a maior distância de um vértice da aba à linha de dobra (a borda livre). `None` sem pontos.
    pub fn novo(analise: &'a Analise, dmax: f64) -> Option<Campo<'a>> {
        let pts: Vec<[f64; 3]> = analise.pontos.iter().map(|p| p.p).collect();
        if pts.is_empty() {
            return None;
        }
        let grade = Grade::nova(&pts);
        Some(Campo { analise, pts, grade, dmax: dmax.max(1e-6) })
    }

    /// Distância do ponto à linha de dobra (mm).
    pub fn distancia(&self, p: [f32; 3]) -> f64 {
        self.grade.mais_perto(&self.pts, p.map(f64::from)).map_or(0., |(_, d)| d)
    }

    /// Deformação (ε) e espessura final relativa (t'/t) na posição `p` do 3D.
    pub fn medida(&self, p: [f32; 3]) -> (f64, f64) {
        let q = p.map(f64::from);
        let Some((i, d)) = self.grade.mais_perto(&self.pts, q) else { return (0., 1.) };
        let eps = self.analise.pontos[i].eps * (d / self.dmax).clamp(0., 1.);
        (eps, 1. / (1. + eps))
    }

    pub fn faixa(&self, p: [f32; 3]) -> Faixa {
        Faixa::de(self.medida(p).0, self.analise)
    }
}

/// Cor por vértice de todas as abas com análise: o que `scene::build` consulta em cada face da aba.
pub struct Mapa<'a> {
    pub paradas: [[f32; 3]; 4],
    abas: HashMap<Id, &'a Analise>,
}

impl<'a> Mapa<'a> {
    pub fn novo(conformacoes: &'a [(Id, Analise)], paradas: [[f32; 3]; 4]) -> Option<Mapa<'a>> {
        let abas: HashMap<Id, &Analise> = conformacoes.iter().filter(|(_, a)| !a.pontos.is_empty()).map(|(id, a)| (*id, a)).collect();
        (!abas.is_empty()).then_some(Mapa { paradas, abas })
    }

    pub fn analise(&self, comando: Id) -> Option<&'a Analise> {
        self.abas.get(&comando).copied()
    }

    /// O campo do comando para os vértices `pos` (posições da malha) das faces dele.
    pub fn campo(&self, comando: Id, pos: &[[f32; 3]]) -> Option<Campo<'a>> {
        let a = self.analise(comando)?;
        let base = Campo::novo(a, 1.)?;
        let dmax = pos.iter().map(|p| base.distancia(*p)).fold(0., f64::max);
        Campo::novo(a, dmax)
    }

    pub fn cor(&self, campo: &Campo, p: [f32; 3]) -> [f32; 3] {
        campo.faixa(p).cor(&self.paradas)
    }
}

/// Posições dos vértices das faces que `dono` (o comando) criou na malha `m`.
pub fn posicoes_do_dono(m: &fenkra_kernel::Mesh, face_owner: &[Id], dono: Id) -> Vec<[f32; 3]> {
    let mut v = vec![];
    for fi in 0..m.face_ranges.len() {
        if face_owner.get(fi) != Some(&dono) {
            continue;
        }
        let (a, b) = (m.face_ranges[fi] as usize, m.face_ranges.get(fi + 1).map_or(m.indices.len(), |x| *x as usize));
        v.extend(m.indices[a..b].iter().map(|i| {
            let k = *i as usize * 3;
            [m.positions[k], m.positions[k + 1], m.positions[k + 2]]
        }));
    }
    v
}

/// Resumo para a legenda: pior contração e pior estiramento, espessura final e comprimento por faixa.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Resumo {
    pub eps_min: f64,
    pub eps_max: f64,
    /// espessura final mínima e máxima, em mm
    pub esp_min: f64,
    pub esp_max: f64,
    /// comprimento da linha de dobra em cada faixa de risco da análise: ok, atenção, falha
    pub comprimentos: [f64; 3],
}

pub fn resumo(conformacoes: &[(Id, Analise)]) -> Option<Resumo> {
    let mut r: Option<Resumo> = None;
    for (_, a) in conformacoes.iter().filter(|(_, a)| !a.pontos.is_empty()) {
        let (e0, e1) = a.pontos.iter().fold((0f64, 0f64), |(lo, hi), p| (lo.min(p.eps), hi.max(p.eps)));
        let t = a.aba.t;
        let c = a.comprimentos();
        let x = r.get_or_insert(Resumo { eps_min: e0, eps_max: e1, esp_min: t / (1. + e1), esp_max: t / (1. + e0), comprimentos: [0.; 3] });
        x.eps_min = x.eps_min.min(e0);
        x.eps_max = x.eps_max.max(e1);
        x.esp_min = x.esp_min.min(t / (1. + e1));
        x.esp_max = x.esp_max.max(t / (1. + e0));
        for k in 0..3 {
            x.comprimentos[k] += c[k];
        }
    }
    r
}

/// Id do dado de tela que liga o checkbox do painel dos comandos ao app.
fn chave() -> egui::Id {
    egui::Id::new("fenkra_mostrar_deformacao")
}

/// Opção "Mostrar deformação" do painel de comando de aba. Vale para a peça inteira.
pub fn opcao(ui: &mut egui::Ui) {
    let mut v = ui.ctx().data(|d| d.get_temp::<bool>(chave())).unwrap_or(false);
    let r = crate::editor::check(ui, &mut v, "Mostrar deformação").on_hover_text(tr("Pinta a aba no 3D pela deformação da borda livre na conformação: azul contrai sem risco, verde está bom, amarelo pede atenção e vermelho enruga ou trinca. A cor sai do tema; a legenda mostra a escala e a espessura final."));
    if r.changed() {
        ui.ctx().data_mut(|d| d.insert_temp(chave(), v));
    }
}

/// Liga o dado de tela às preferências e diz se a cena precisa ser refeita. Chamada uma vez por quadro.
pub fn sincroniza(ctx: &egui::Context, ligado: &mut bool) -> bool {
    match ctx.data(|d| d.get_temp::<bool>(chave())) {
        Some(v) if v != *ligado => {
            *ligado = v;
            true
        }
        Some(_) => false,
        None => {
            ctx.data_mut(|d| d.insert_temp(chave(), *ligado));
            false
        }
    }
}

/// Desliga a opção (o ✕ da legenda).
pub fn desliga(ctx: &egui::Context, ligado: &mut bool) {
    *ligado = false;
    ctx.data_mut(|d| d.insert_temp(chave(), false));
}

/// Texto de uma deformação em %, com sinal.
pub fn pct(eps: f64) -> String {
    format!("{:+.1} %", eps * 100.)
}

/// Legenda: as quatro faixas, a escala de ε e a espessura final. Painel livre, aberto só com a opção ligada.
pub fn legenda(app: &mut crate::app::App, ctx: &egui::Context, area: Rect) {
    use crate::paineis::{self, Painel};
    if !app.settings.mostrar_deformacao {
        return;
    }
    let Some(r) = app.result.as_ref() else { return };
    let Some(res) = resumo(&r.conformacoes) else { return };
    let p = crate::sup_analise::paradas(&ctx.global_style().visuals);
    let cfg = Painel { nome: "escala_deformacao".into(), titulo: tr("Mapa de deformação").to_string(), padrao: paineis::abaixo_do_cubo(area, 250.), largura: 250., redimensiona: false };
    let mut fechar = false;
    let posicao = app.settings.painel(&cfg.nome, paineis::Encaixe::Livre);
    let saida = paineis::mostrar(ctx, area, &cfg, posicao, |ui| {
        ui.label(RichText::new(tr("Deformação da borda livre da aba na conformação (prensa com borracha, hidroconformação).")).small());
        let limite = |f: Faixa| match f {
            Faixa::Contracao => format!("ε ≤ {}", pct(CONTRACAO_VISIVEL)),
            Faixa::Ok => tr("dentro do limite").to_string(),
            Faixa::Atencao => tr("perto do limite").to_string(),
            Faixa::Falha => tr("passa do limite").to_string(),
        };
        let nome = |f: Faixa| match f {
            Faixa::Contracao => tr("Contração"),
            Faixa::Ok => tr("Ok"),
            Faixa::Atencao => tr("Atenção"),
            Faixa::Falha => tr("Enruga ou trinca"),
        };
        for f in Faixa::TODAS {
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(egui::vec2(14., 10.), egui::Sense::hover());
                ui.painter().rect_filled(r, 2., crate::theme::cor_da_escala(f.cor(&p)));
                ui.label(RichText::new(nome(f)).small().strong());
                ui.label(RichText::new(limite(f)).small().weak());
            });
        }
        ui.separator();
        ui.label(RichText::new(format!("{} {} … {}", tr("ε na borda"), pct(res.eps_min), pct(res.eps_max))).small());
        ui.label(RichText::new(format!("{} {} … {} mm", tr("Espessura final"), crate::app::fmt(res.esp_min), crate::app::fmt(res.esp_max))).small()).on_hover_text(tr("Constância de volume com o raio da aba fixo: t' = t / (1 + ε). Contração engrossa a borda, estiramento afina."));
        ui.label(RichText::new(tr("A cor cresce da dobra para a borda livre, onde vale o ε medido.")).small().weak());
    });
    if let Some(m) = saida.movido {
        app.settings.guardar_painel(&cfg.nome, m);
    }
    if saida.rect.is_some_and(|r| crate::analise_janela::botao_fechar(ctx, r, &cfg.nome)) {
        fechar = true;
    }
    if fechar {
        desliga(ctx, &mut app.settings.mostrar_deformacao);
        app.scene_dirty = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chapa_hidro::{analisar, Aba, Limites};

    /// Oval fechada (a, b) em z = 0 como polilinha de `n` pontos, com o sentido de fora em cada ponto.
    fn oval(a: f64, b: f64, n: usize) -> (Vec<[f64; 3]>, Vec<[f64; 3]>) {
        let pts: Vec<[f64; 3]> = (0..=n).map(|i| {
            let t = std::f64::consts::TAU * (i % n) as f64 / n as f64;
            [a * t.cos(), b * t.sin(), 0.]
        }).collect();
        let outs = pts.iter().map(|p| {
            let g = [p[0] / (a * a), p[1] / (b * b), 0.];
            let m = (g[0] * g[0] + g[1] * g[1]).sqrt();
            [g[0] / m, g[1] / m, 0.]
        }).collect();
        (pts, outs)
    }

    fn aba(len: f64) -> Aba {
        Aba { t: 1.2, r: 1.2, k: 0.33, ang: 90., len }
    }

    #[test]
    fn faixa_pela_deformacao_e_pelo_regime() {
        let (pts, outs) = oval(600., 450., 200);
        let a = analisar(&pts, &outs, [0., 0., 1.], aba(12.), Limites::default());
        // contração pequena: azul; quase nada: verde; tração pequena: verde
        assert_eq!(Faixa::de(-0.005, &a), Faixa::Contracao);
        assert_eq!(Faixa::de(-0.0005, &a), Faixa::Ok);
        assert_eq!(Faixa::de(0.0, &a), Faixa::Ok);
        assert_eq!(Faixa::de(0.01, &a), Faixa::Ok);
        // limites do material: tração perto de metade e além do alongamento limite
        assert_eq!(Faixa::de(0.6 * a.lim.trinca, &a), Faixa::Atencao);
        assert_eq!(Faixa::de(1.2 * a.lim.trinca, &a), Faixa::Falha);
        // compressão: enruga muito antes da tração trincar
        assert_eq!(Faixa::de(-0.5, &a), Faixa::Falha);
    }

    #[test]
    fn cada_faixa_leva_a_cor_do_tema() {
        let p = [[0., 0., 1.], [0., 1., 0.], [1., 1., 0.], [1., 0., 0.]];
        assert_eq!(Faixa::Contracao.cor(&p), p[0]);
        assert_eq!(Faixa::Ok.cor(&p), p[1]);
        assert_eq!(Faixa::Atencao.cor(&p), p[2]);
        assert_eq!(Faixa::Falha.cor(&p), p[3]);
        // as quatro cores do tema (claro, escuro, paleta segura) diferem entre si
        let v = eframe::egui::Visuals::dark();
        let t = crate::sup_analise::paradas(&v);
        for i in 0..4 {
            for j in i + 1..4 {
                assert_ne!(t[i], t[j], "faixas {i} e {j} com a mesma cor");
            }
        }
    }

    /// Aba oval cuja contração toca as três faixas de risco: o comprimento é escolhido na varredura.
    #[test]
    fn oval_tem_vertices_nas_tres_faixas() {
        let (pts, outs) = oval(600., 450., 360);
        let mut achou = None;
        for len in (5..=80).map(f64::from) {
            let a = analisar(&pts, &outs, [0., 0., 1.], aba(len), Limites::default());
            let c = a.comprimentos();
            if c.iter().all(|x| *x > 1.) {
                achou = Some((len, a));
                break;
            }
        }
        let (len, a) = achou.expect("alguma aba da oval tem trechos ok, atenção e falha");
        // vértices da aba: da linha de dobra (z = 0) à borda livre, para cima, em cada ponto da linha
        let mut por_faixa = [0usize; 4];
        let campo = Campo::novo(&a, len + a.aba.r + a.aba.t).unwrap();
        for (p, o) in pts.iter().zip(&outs).take(pts.len() - 1) {
            for k in 0..=4 {
                let h = (a.aba.r + a.aba.t + len) * k as f64 / 4.;
                let q = [(p[0] + o[0] * 0.) as f32, (p[1]) as f32, h as f32];
                por_faixa[campo.faixa(q) as usize] += 1;
            }
        }
        // na dobra (altura 0) a cor é a neutra; na borda livre aparecem as faixas de risco
        assert!(por_faixa[Faixa::Ok as usize] > 0, "sem vértices ok: {por_faixa:?}");
        assert!(por_faixa[Faixa::Atencao as usize] > 0, "sem vértices em atenção: {por_faixa:?}");
        assert!(por_faixa[Faixa::Falha as usize] > 0, "sem vértices que enrugam: {por_faixa:?}");
    }

    #[test]
    fn deformacao_cresce_da_dobra_para_a_borda() {
        let (pts, outs) = oval(600., 450., 200);
        let a = analisar(&pts, &outs, [0., 0., 1.], aba(20.), Limites::default());
        let campo = Campo::novo(&a, 22.).unwrap();
        let p = pts[0];
        let na_dobra = campo.medida([p[0] as f32, p[1] as f32, 0.]).0;
        let no_meio = campo.medida([p[0] as f32, p[1] as f32, 11.]).0;
        let na_borda = campo.medida([p[0] as f32, p[1] as f32, 22.]).0;
        assert!(na_dobra.abs() < 1e-9);
        assert!(no_meio < 0. && na_borda < no_meio, "contração cresce: {na_dobra} {no_meio} {na_borda}");
        // espessura final: contração engrossa
        assert!(campo.medida([p[0] as f32, p[1] as f32, 22.]).1 > 1.);
    }

    #[test]
    fn grade_acha_o_mesmo_ponto_que_a_varredura() {
        let (pts, _) = oval(600., 450., 500);
        let g = Grade::nova(&pts);
        for q in [[10., 20., 5.], [700., 0., 30.], [-300., 440., 12.], [0., 0., 0.]] {
            let (i, d) = g.mais_perto(&pts, q).unwrap();
            let (j, dj) = pts.iter().enumerate().map(|(j, p)| (j, ((p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2) + (p[2] - q[2]).powi(2)).sqrt())).min_by(|a, b| a.1.total_cmp(&b.1)).unwrap();
            assert!((d - dj).abs() < 1e-9, "{q:?}: {i} {d} contra {j} {dj}");
        }
    }

    #[test]
    fn resumo_junta_as_abas() {
        let (pts, outs) = oval(600., 450., 200);
        let a = analisar(&pts, &outs, [0., 0., 1.], aba(12.), Limites::default());
        let r = resumo(&[(Id(1), a.clone()), (Id(2), a)]).unwrap();
        assert!(r.eps_min < 0. && r.eps_max >= 0.);
        assert!(r.esp_max > 1.2, "contração engrossa a borda: {}", r.esp_max);
        assert!(resumo(&[]).is_none());
    }
}
