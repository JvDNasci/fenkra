// SPDX-License-Identifier: LGPL-2.1-or-later
//! Acessibilidade básica (B321): paleta segura para daltonismo, símbolo para todo estado (a cor
//! nunca é o único sinal), escala da fonte e opção de reduzir movimento. A pesquisa está em
//! `docs/pesquisa/acessibilidade.md`.
//!
//! As cores semânticas vêm de funções puras (`cor_estado`, `cor_papel`) para o teste simular as três
//! deficiências sem depender do tema global. A simulação usa a matriz linear de Machado, Oliveira e
//! Fernandes (2009), severidade máxima, aplicada em RGB linear.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};

/// Opções do usuário; ficam em `Settings::acessibilidade`.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Acessibilidade {
    /// troca verde x vermelho por azul x laranja nos estados e nos papéis da prévia
    pub paleta_segura: bool,
    /// câmera salta em vez de deslizar e as transições da interface terminam no mesmo quadro
    pub reduzir_movimento: bool,
    /// fator aplicado ao tamanho das fontes, além da escala da interface (0,8 a 1,6)
    pub escala_fonte: f32,
}

impl Default for Acessibilidade {
    fn default() -> Self {
        Acessibilidade { paleta_segura: false, reduzir_movimento: false, escala_fonte: 1.0 }
    }
}

impl Acessibilidade {
    pub fn fator_fonte(&self) -> f32 {
        if self.escala_fonte.is_finite() { self.escala_fonte.clamp(0.8, 1.6) } else { 1.0 }
    }
}

static SEGURA: AtomicBool = AtomicBool::new(false);
/// fator de fonte em milésimos (1000 = 1,0), para os rótulos pintados à mão no 3D
static FONTE_MILESIMOS: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1000);

/// Tamanho de um rótulo pintado no 3D: acompanha a escala da fonte escolhida (a escala da interface
/// já chega pelo fator de zoom do egui).
pub fn fonte_3d(base: f32) -> f32 {
    base * FONTE_MILESIMOS.load(Ordering::Relaxed) as f32 / 1000.
}

/// Publica a escolha para as funções de cor que não recebem as configurações.
pub fn definir(a: &Acessibilidade) {
    SEGURA.store(a.paleta_segura, Ordering::Relaxed);
    FONTE_MILESIMOS.store((a.fator_fonte() * 1000.).round() as u32, Ordering::Relaxed);
}

pub fn paleta_segura() -> bool {
    SEGURA.load(Ordering::Relaxed)
}

/// Estados que a interface distingue por cor. Cada um tem um símbolo.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Estado {
    Ok,
    Aviso,
    Erro,
    Selecao,
    PreSelecao,
    Acrescenta,
    Remove,
    Referencia,
}

impl Estado {
    pub const TODOS: [Estado; 8] = [Estado::Ok, Estado::Aviso, Estado::Erro, Estado::Selecao, Estado::PreSelecao, Estado::Acrescenta, Estado::Remove, Estado::Referencia];

    /// Marca que acompanha a cor: forma, não matiz.
    pub fn simbolo(self) -> &'static str {
        match self {
            Estado::Ok => "✔",
            Estado::Aviso => "▲",
            Estado::Erro => "■",
            Estado::Selecao => "●",
            Estado::PreSelecao => "○",
            Estado::Acrescenta => "＋",
            Estado::Remove => "－",
            Estado::Referencia => "┄",
        }
    }

    /// Texto curto para leitor de tela e dica (chave de tradução).
    pub fn nome(self) -> &'static str {
        match self {
            Estado::Ok => "ok",
            Estado::Aviso => "aviso",
            Estado::Erro => "erro",
            Estado::Selecao => "selecionado",
            Estado::PreSelecao => "pré-seleção",
            Estado::Acrescenta => "acrescenta",
            Estado::Remove => "remove",
            Estado::Referencia => "referência",
        }
    }

    /// Padrão de traço para quem desenha linhas: só a referência é tracejada, a prévia que
    /// remove é hachurada; os demais são contínuos.
    pub fn traco(self) -> Traco {
        match self {
            Estado::Referencia => Traco::Tracejado,
            Estado::Remove => Traco::Hachura,
            Estado::PreSelecao => Traco::Pontilhado,
            _ => Traco::Continuo,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Traco {
    Continuo,
    Tracejado,
    Pontilhado,
    Hachura,
}

/// Cor do estado para texto sobre o painel do tema escuro ou claro. Paleta segura: azul para ok,
/// amarelo-âmbar para aviso e vermelhão para erro; separam-se por luminância e por azul x laranja.
pub fn cor_estado(e: Estado, escuro: bool, segura: bool) -> [u8; 3] {
    match (e, segura, escuro) {
        (Estado::Ok, false, true) => [110, 210, 120],
        (Estado::Ok, false, false) => [30, 130, 50],
        (Estado::Aviso, false, true) => [240, 200, 90],
        (Estado::Aviso, false, false) => [170, 110, 0],
        (Estado::Erro, false, true) => [255, 120, 100],
        (Estado::Erro, false, false) => [200, 40, 30],
        (Estado::Ok, true, true) => [86, 180, 233],
        (Estado::Ok, true, false) => [0, 90, 170],
        (Estado::Aviso, true, true) => [240, 228, 66],
        (Estado::Aviso, true, false) => [125, 90, 0],
        (Estado::Erro, true, true) => [255, 120, 100],
        (Estado::Erro, true, false) => [190, 0, 90],
        // fora do trio de estado, as demais marcas usam o mesmo par azul x laranja
        (Estado::Acrescenta, _, true) => [86, 180, 233],
        (Estado::Acrescenta, _, false) => [0, 90, 170],
        (Estado::Remove, _, true) => [255, 140, 80],
        (Estado::Remove, _, false) => [190, 50, 0],
        (Estado::Selecao, _, true) => [240, 228, 66],
        (Estado::Selecao, _, false) => [125, 90, 0],
        (Estado::PreSelecao, _, true) => [200, 205, 215],
        (Estado::PreSelecao, _, false) => [70, 75, 85],
        (Estado::Referencia, _, true) => [150, 158, 170],
        (Estado::Referencia, _, false) => [90, 96, 108],
    }
}

/// Papéis da seleção do comando e da prévia (B246, B247): volume que sai, ferramenta e neutro.
pub fn cor_papel(papel: Papel, escuro: bool, segura: bool) -> [u8; 3] {
    match (papel, segura, escuro) {
        (Papel::Retirada, false, true) => [240, 110, 170],
        (Papel::Retirada, false, false) => [185, 30, 110],
        (Papel::Ferramenta, false, true) => [175, 140, 255],
        (Papel::Ferramenta, false, false) => [100, 60, 190],
        (Papel::Neutro, false, true) => [130, 215, 195],
        (Papel::Neutro, false, false) => [20, 120, 105],
        (Papel::Retirada, true, true) => [255, 140, 80],
        (Papel::Retirada, true, false) => [190, 50, 0],
        (Papel::Ferramenta, true, true) => [160, 150, 255],
        (Papel::Ferramenta, true, false) => [70, 50, 170],
        (Papel::Neutro, true, true) => [110, 220, 190],
        (Papel::Neutro, true, false) => [0, 115, 110],
        // corpo alvo da booleana, da divisão e do mover (B300): verde-limão, longe do rosa, do roxo e do verde-água
        (Papel::Alvo, false, true) => [190, 225, 70],
        (Papel::Alvo, false, false) => [85, 115, 0],
        (Papel::Alvo, true, true) => [235, 235, 130],
        (Papel::Alvo, true, false) => [105, 90, 45],
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Papel {
    Retirada,
    Ferramenta,
    Neutro,
}

/// Deficiências simuladas.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Deficiencia {
    Protanopia,
    Deuteranopia,
    Tritanopia,
}

impl Deficiencia {
    pub const TODAS: [Deficiencia; 3] = [Deficiencia::Protanopia, Deficiencia::Deuteranopia, Deficiencia::Tritanopia];
    fn matriz(self) -> [[f32; 3]; 3] {
        match self {
            Deficiencia::Protanopia => [[0.152286, 1.052583, -0.204868], [0.114503, 0.786281, 0.099216], [-0.003882, -0.048116, 1.051998]],
            Deficiencia::Deuteranopia => [[0.367322, 0.860646, -0.227968], [0.280085, 0.672501, 0.047413], [-0.011820, 0.042940, 0.968881]],
            Deficiencia::Tritanopia => [[1.255528, -0.076749, -0.178779], [-0.078411, 0.930809, 0.147602], [0.004733, 0.691367, 0.303900]],
        }
    }
}

fn linear(v: u8) -> f32 {
    let s = v as f32 / 255.;
    if s <= 0.04045 { s / 12.92 } else { ((s + 0.055) / 1.055).powf(2.4) }
}

fn codificar(l: f32) -> u8 {
    let l = l.clamp(0., 1.);
    let s = if l <= 0.0031308 { 12.92 * l } else { 1.055 * l.powf(1. / 2.4) - 0.055 };
    (s * 255.).round().clamp(0., 255.) as u8
}

/// Como a cor aparece para quem tem a deficiência (também usada para simular capturas).
pub fn simular(c: [u8; 3], d: Deficiencia) -> [u8; 3] {
    let m = d.matriz();
    let l = [linear(c[0]), linear(c[1]), linear(c[2])];
    let o = |i: usize| codificar(m[i][0] * l[0] + m[i][1] * l[1] + m[i][2] * l[2]);
    [o(0), o(1), o(2)]
}

/// Simula o quadro inteiro (RGBA, 4 bytes por pixel), por exemplo uma captura de tela.
pub fn simular_quadro(rgba: &mut [u8], d: Deficiencia) {
    for p in rgba.chunks_exact_mut(4) {
        let s = simular([p[0], p[1], p[2]], d);
        p[..3].copy_from_slice(&s);
    }
}

fn lab(c: [u8; 3]) -> [f32; 3] {
    let (r, g, b) = (linear(c[0]), linear(c[1]), linear(c[2]));
    let x = (0.4124 * r + 0.3576 * g + 0.1805 * b) / 0.95047;
    let y = 0.2126 * r + 0.7152 * g + 0.0722 * b;
    let z = (0.0193 * r + 0.1192 * g + 0.9505 * b) / 1.08883;
    let f = |t: f32| if t > 0.008856 { t.cbrt() } else { 7.787 * t + 16. / 116. };
    let (fx, fy, fz) = (f(x), f(y), f(z));
    [116. * fy - 16., 500. * (fx - fy), 200. * (fy - fz)]
}

/// Distância de cor (CIE76 em Lab). Abaixo de 10 as cores se confundem à vista.
pub fn delta_e(a: [u8; 3], b: [u8; 3]) -> f32 {
    let (la, lb) = (lab(a), lab(b));
    ((la[0] - lb[0]).powi(2) + (la[1] - lb[1]).powi(2) + (la[2] - lb[2]).powi(2)).sqrt()
}

/// Luminância relativa (WCAG) e razão de contraste.
pub fn contraste(a: [u8; 3], b: [u8; 3]) -> f32 {
    let l = |c: [u8; 3]| 0.2126 * linear(c[0]) + 0.7152 * linear(c[1]) + 0.0722 * linear(c[2]);
    let (x, y) = (l(a), l(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

/// Pior distância entre qualquer par da lista, vista por qualquer das três deficiências e também
/// sem deficiência.
pub fn pior_distancia(cores: &[[u8; 3]]) -> f32 {
    let mut pior = f32::MAX;
    for (i, a) in cores.iter().enumerate() {
        for b in &cores[i + 1..] {
            pior = pior.min(delta_e(*a, *b));
            for d in Deficiencia::TODAS {
                pior = pior.min(delta_e(simular(*a, d), simular(*b, d)));
            }
        }
    }
    pior
}

/// Aplica o fator de fonte sobre os tamanhos padrão do egui (não acumula entre chamadas).
pub fn aplicar_fonte(ctx: &eframe::egui::Context, a: &Acessibilidade) {
    let base = eframe::egui::Style::default();
    let k = a.fator_fonte();
    ctx.global_style_mut(|s| {
        for (ts, f) in s.text_styles.iter_mut() {
            if let Some(b) = base.text_styles.get(ts) {
                f.size = b.size * k;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme;

    #[test]
    fn todo_estado_tem_simbolo_e_nome_proprios() {
        let mut simbolos = std::collections::HashSet::new();
        for e in Estado::TODOS {
            assert!(!e.simbolo().is_empty() && !e.nome().is_empty());
            assert!(simbolos.insert(e.simbolo()), "símbolo repetido em {e:?}");
        }
        // prévia que remove e que acrescenta diferem também no traço, não só na cor
        assert_ne!(Estado::Remove.traco(), Estado::Acrescenta.traco());
        assert_ne!(Estado::Referencia.traco(), Estado::Selecao.traco());
    }

    #[test]
    fn simulacao_e_identidade_no_cinza_e_muda_o_vermelho_e_o_verde() {
        for d in Deficiencia::TODAS {
            let c = simular([128, 128, 128], d);
            for k in 0..3 {
                assert!((c[k] as i32 - 128).abs() <= 2, "{d:?}: cinza virou {c:?}");
            }
        }
        // vermelho e verde puros ficam quase iguais para protanopia e deuteranopia
        let v = delta_e(simular([200, 40, 30], Deficiencia::Deuteranopia), simular([30, 130, 50], Deficiencia::Deuteranopia));
        let n = delta_e([200, 40, 30], [30, 130, 50]);
        assert!(v < n, "a simulação deveria aproximar vermelho e verde ({v} contra {n})");
    }

    #[test]
    fn paleta_segura_separa_ok_aviso_e_erro_nas_tres_deficiencias() {
        for escuro in [true, false] {
            let trio = [cor_estado(Estado::Ok, escuro, true), cor_estado(Estado::Aviso, escuro, true), cor_estado(Estado::Erro, escuro, true)];
            let d = pior_distancia(&trio);
            assert!(d >= 10., "paleta segura (escuro={escuro}) tem par a {d:.1} de distância");
        }
    }

    #[test]
    fn paleta_segura_separa_os_papeis_e_o_par_acrescenta_remove() {
        for escuro in [true, false] {
            let papeis = [cor_papel(Papel::Retirada, escuro, true), cor_papel(Papel::Ferramenta, escuro, true), cor_papel(Papel::Neutro, escuro, true), cor_papel(Papel::Alvo, escuro, true)];
            assert!(pior_distancia(&papeis) >= 10., "papéis (escuro={escuro})");
            let par = [cor_estado(Estado::Acrescenta, escuro, true), cor_estado(Estado::Remove, escuro, true)];
            assert!(pior_distancia(&par) >= 10., "acrescenta x remove (escuro={escuro})");
        }
    }

    #[test]
    fn a_paleta_antiga_falha_onde_a_segura_passa() {
        // garante que o teste enxerga o problema: verde x vermelho se confunde no tema claro
        let antiga = [cor_estado(Estado::Ok, false, false), cor_estado(Estado::Aviso, false, false), cor_estado(Estado::Erro, false, false)];
        assert!(pior_distancia(&antiga) < pior_distancia(&[cor_estado(Estado::Ok, false, true), cor_estado(Estado::Aviso, false, true), cor_estado(Estado::Erro, false, true)]));
    }

    #[test]
    fn estados_e_papeis_tem_contraste_aa_em_todos_os_temas() {
        for t in theme::builtin() {
            for fundo in [t.panel, t.window] {
                for segura in [false, true] {
                    for e in [Estado::Ok, Estado::Aviso, Estado::Erro, Estado::Acrescenta, Estado::Remove, Estado::Selecao] {
                        let r = contraste(cor_estado(e, t.dark, segura), fundo);
                        assert!(r >= 3.0, "{} (segura={segura}): {e:?} sobre o fundo tem contraste {r:.2}", t.name);
                    }
                    for p in [Papel::Retirada, Papel::Ferramenta, Papel::Neutro, Papel::Alvo] {
                        let r = contraste(cor_papel(p, t.dark, segura), fundo);
                        assert!(r >= 3.0, "{} (segura={segura}): {p:?} tem contraste {r:.2}", t.name);
                    }
                }
            }
        }
    }

    #[test]
    fn fator_de_fonte_e_limitado_e_nao_acumula() {
        let ctx = eframe::egui::Context::default();
        let corpo = |ctx: &eframe::egui::Context| ctx.global_style().text_styles[&eframe::egui::TextStyle::Body].size;
        let base = corpo(&ctx);
        let a = Acessibilidade { escala_fonte: 1.5, ..Default::default() };
        aplicar_fonte(&ctx, &a);
        aplicar_fonte(&ctx, &a);
        assert!((corpo(&ctx) - base * 1.5).abs() < 0.01);
        assert_eq!(Acessibilidade { escala_fonte: 9.0, ..Default::default() }.fator_fonte(), 1.6);
        assert_eq!(Acessibilidade { escala_fonte: f32::NAN, ..Default::default() }.fator_fonte(), 1.0);
        aplicar_fonte(&ctx, &Acessibilidade::default());
        assert!((corpo(&ctx) - base).abs() < 0.01);
    }

    #[test]
    fn json_antigo_sem_acessibilidade_carrega_com_padroes() {
        let a: Acessibilidade = serde_json::from_str("{}").unwrap();
        assert_eq!(a, Acessibilidade::default());
        let s: crate::settings::Settings = serde_json::from_str(r#"{"ui_scale": 1.2}"#).unwrap();
        assert!(!s.acessibilidade.paleta_segura && !s.acessibilidade.reduzir_movimento);
    }
}
