// SPDX-License-Identifier: LGPL-2.1-or-later
//! Verificação de resistência do par de engrenagens (aba Verificar, B52).
//!
//! **Cálculo preliminar**, com as fórmulas fechadas dos livros-texto de elementos de máquinas; não
//! substitui o cálculo por norma de quem assina o projeto (ver [`crate::calculo_projeto::AVISO_PRELIMINAR`]).
//!
//! - **Flexão no pé (Lewis, 1892).** O dente é uma viga em balanço com a carga tangencial `Ft` no
//!   topo, aplicada onde a linha de ação que passa pela ponta cruza o eixo do dente. A seção crítica é a de maior `6 l / t²` ao longo do flanco **gerado** (evolvente e
//!   trocoide do pé), em que `t` é a espessura do dente na seção e `l` a distância da seção à carga:
//!   é a parábola de igual resistência tangente ao pé. Daí sai o fator de forma
//!   `Y = t² / (6 l m)` do próprio dente, e `σF = Kv · Ft / (b · m · Y)`.
//! - **Fator dinâmico (Barth).** Para perfil cortado, `Kv = (6,1 + V) / 6,1`, com `V` a velocidade
//!   no primitivo em m/s.
//! - **Contato (Hertz, 1882).** Dois cilindros com os raios de curvatura da evolvente no ponto
//!   primitivo, `ρ = d sen α / 2`:
//!   `σH = Cp √(Kv Ft (1/ρ1 + 1/ρ2) / (b cos α))`, com `Cp = √(1 / (π ((1 − ν1²)/E1 + (1 − ν2²)/E2)))`.
//!   Na interna a curvatura da coroa é côncava (`1/ρ2` negativo) e na cremalheira é nula.
//!
//! O coeficiente de segurança à flexão é `Sy / σF` (escoamento do material, conservador por não
//! ter fadiga). O contato só é comparado se a pessoa der a pressão admissível do material.

use crate::calculo_projeto::{Material, AVISO_PRELIMINAR};
use crate::engrenagens::{dente_externo, ParamEngrenagem, TipoEngrenagem};
use std::f64::consts::PI;

/// Método de verificação. Só o de livro-texto está implementado; os de norma ficam listados para a
/// escolha aparecer, mas devolvem erro explicando que ainda não existem.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Metodo {
    #[default]
    LewisHertz,
    Iso6336,
}

impl Metodo {
    pub const TODOS: [Metodo; 2] = [Metodo::LewisHertz, Metodo::Iso6336];

    pub fn rotulo(self) -> &'static str {
        match self {
            Metodo::LewisHertz => "Lewis e Hertz (livro-texto)",
            Metodo::Iso6336 => "ISO 6336 (ainda não disponível)",
        }
    }
}

/// Entradas da verificação do par.
#[derive(Clone, Debug, PartialEq)]
pub struct EntradaVerificacao {
    pub metodo: Metodo,
    /// potência transmitida (kW)
    pub potencia_kw: f64,
    /// rotação do pinhão (rpm)
    pub rpm_pinhao: f64,
    pub material_pinhao: Material,
    pub material_coroa: Material,
    /// pressão de contato admissível do material mais fraco (MPa); 0 = não comparar
    pub contato_admissivel: f64,
    /// coeficiente de segurança exigido
    pub fs_exigido: f64,
}

impl Default for EntradaVerificacao {
    fn default() -> Self {
        let aco = crate::calculo_projeto::MATERIAIS[0];
        EntradaVerificacao { metodo: Metodo::LewisHertz, potencia_kw: 1., rpm_pinhao: 1450., material_pinhao: aco, material_coroa: aco, contato_admissivel: 0., fs_exigido: 1.5 }
    }
}

/// Resultado por engrenagem e do par.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ResultadoVerificacao {
    /// torque no pinhão (N·m)
    pub torque_pinhao: f64,
    /// força tangencial no primitivo (N)
    pub forca_tangencial: f64,
    /// força radial (N)
    pub forca_radial: f64,
    /// velocidade no primitivo (m/s)
    pub velocidade: f64,
    pub kv: f64,
    /// fator de forma de Lewis do pinhão e da coroa (0 na interna e na cremalheira, que não são verificadas à flexão)
    pub y_pinhao: f64,
    pub y_coroa: f64,
    /// tensão de flexão no pé (MPa)
    pub flexao_pinhao: f64,
    pub flexao_coroa: f64,
    /// coeficientes de segurança à flexão (Sy / σF)
    pub fs_flexao_pinhao: f64,
    pub fs_flexao_coroa: f64,
    /// pressão de contato de Hertz (MPa)
    pub contato: f64,
    /// coeficiente de segurança ao contato (admissível / σH); 0 = não comparado
    pub fs_contato: f64,
    pub avisos: Vec<String>,
}

/// Fator de forma de Lewis do dente gerado, com a carga no topo sobre o eixo do dente.
pub fn fator_lewis(p: &ParamEngrenagem) -> Result<f64, String> {
    if !matches!(p.tipo, TipoEngrenagem::Reta | TipoEngrenagem::Helicoidal) {
        return Err("o fator de Lewis vale para engrenagem externa".into());
    }
    let g = dente_externo(p);
    let l = g.limites()?;
    // flanco do pé ao topo, como (raio, meio ângulo): filete e evolvente
    let mut flanco: Vec<[f64; 2]> = l.filete.clone();
    let n = 60;
    for i in 0..=n {
        let th = l.th0 + (l.th1 - l.th0) * i as f64 / n as f64;
        let r = g.rb * (1. + th * th).sqrt();
        flanco.push([r, g.meio_angulo(th)]);
    }
    // a força normal no topo segue a linha de ação tangente ao círculo base; ela cruza o eixo do
    // dente no raio rb / cos(αa − ψa), que é onde Lewis aplica a componente tangencial
    let alfa_a = (g.rb / g.ra).acos();
    let r_carga = g.rb / (alfa_a - l.psia).cos();
    let pior = flanco
        .iter()
        .map(|q| {
            let (x, y) = (q[0] * q[1].cos(), q[0] * q[1].sin());
            let (t, braco) = (2. * y, r_carga - x);
            if t <= 0. || braco <= 0. {
                0.
            } else {
                6. * braco / (t * t)
            }
        })
        .fold(0., f64::max);
    if pior <= 0. {
        return Err("não achei a seção crítica do dente".into());
    }
    Ok(1. / (pior * p.modulo))
}

/// Verifica o par: `pinhao` e `coroa` como saem do gerador do par (a coroa pode ser interna ou cremalheira).
pub fn verificar_par(pinhao: &ParamEngrenagem, coroa: &ParamEngrenagem, e: &EntradaVerificacao) -> Result<ResultadoVerificacao, String> {
    if e.metodo != Metodo::LewisHertz {
        return Err("a verificação pela ISO 6336 ainda não está disponível: use Lewis e Hertz".into());
    }
    for (v, n) in [(e.potencia_kw, "a potência"), (e.rpm_pinhao, "a rotação"), (e.fs_exigido, "o coeficiente de segurança exigido")] {
        if !(v.is_finite() && v > 0.) {
            return Err(format!("{n} precisa ser maior que zero"));
        }
    }
    if !(e.contato_admissivel.is_finite() && e.contato_admissivel >= 0.) {
        return Err("a pressão admissível não pode ser negativa".into());
    }
    let c1 = crate::engrenagens::calculo(pinhao)?;
    let c2 = crate::engrenagens::calculo(coroa)?;
    let mut r = ResultadoVerificacao::default();
    let d1 = c1.diametro_primitivo;
    // torque em N·mm a partir de P (W) e ω (rad/s)
    let omega = e.rpm_pinhao * 2. * PI / 60.;
    let t_nmm = e.potencia_kw * 1e3 / omega * 1e3;
    r.torque_pinhao = t_nmm / 1e3;
    r.forca_tangencial = 2. * t_nmm / d1;
    let alfa_t = c1.angulo_pressao_transversal.to_radians();
    r.forca_radial = r.forca_tangencial * alfa_t.tan();
    r.velocidade = omega * d1 / 2. / 1e3;
    r.kv = (6.1 + r.velocidade) / 6.1;
    let b = pinhao.largura.min(coroa.largura);
    let m = pinhao.modulo;
    let flexao = |y: f64| r.kv * r.forca_tangencial / (b * m * y);
    r.y_pinhao = fator_lewis(pinhao)?;
    r.flexao_pinhao = flexao(r.y_pinhao);
    r.fs_flexao_pinhao = e.material_pinhao.sy / r.flexao_pinhao;
    if matches!(coroa.tipo, TipoEngrenagem::Reta | TipoEngrenagem::Helicoidal) {
        r.y_coroa = fator_lewis(coroa)?;
        r.flexao_coroa = flexao(r.y_coroa);
        r.fs_flexao_coroa = e.material_coroa.sy / r.flexao_coroa;
    } else {
        r.avisos.push("a flexão da coroa interna e da cremalheira não é verificada (o dente delas é mais largo no pé que o do pinhão)".into());
    }
    let curv1 = 2. / (d1 * alfa_t.sin());
    let curv2 = match coroa.tipo {
        TipoEngrenagem::Cremalheira => 0.,
        TipoEngrenagem::Interna => -2. / (c2.diametro_primitivo * alfa_t.sin()),
        _ => 2. / (c2.diametro_primitivo * alfa_t.sin()),
    };
    let (m1, m2) = (e.material_pinhao, e.material_coroa);
    let cp = (1. / (PI * ((1. - m1.nu * m1.nu) / m1.e + (1. - m2.nu * m2.nu) / m2.e))).sqrt();
    r.contato = cp * (r.kv * r.forca_tangencial * (curv1 + curv2) / (b * alfa_t.cos())).sqrt();
    if e.contato_admissivel > 0. {
        r.fs_contato = e.contato_admissivel / r.contato;
    }
    for (nome, fs) in [("pinhão", r.fs_flexao_pinhao), ("coroa", r.fs_flexao_coroa)] {
        if fs > 0. && fs < e.fs_exigido {
            r.avisos.push(format!("flexão no pé do(a) {nome}: segurança {fs:.2} abaixo da exigida ({:.2}) — aumente o módulo, a largura ou o material", e.fs_exigido));
        }
    }
    if r.fs_contato > 0. && r.fs_contato < e.fs_exigido {
        r.avisos.push(format!("contato: segurança {:.2} abaixo da exigida ({:.2}) — aumente a largura ou os diâmetros", r.fs_contato, e.fs_exigido));
    }
    if pinhao.tipo == TipoEngrenagem::Helicoidal {
        r.avisos.push("helicoidal verificada como reta de mesmo plano transversal (conservador: sem o ganho da razão de contato da hélice)".into());
    }
    Ok(r)
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// Relatório da verificação em HTML simples, para salvar e anexar ao projeto. `tr` traduz os textos.
pub fn relatorio_html(pinhao: &ParamEngrenagem, coroa: &ParamEngrenagem, e: &EntradaVerificacao, r: &ResultadoVerificacao, tr: &dyn Fn(&str) -> String) -> String {
    let f = |v: f64| format!("{v:.3}");
    let linha = |rotulo: &str, a: String, b: String| format!("<tr><td>{}</td><td>{}</td><td>{}</td></tr>\n", esc(&tr(rotulo)), esc(&a), esc(&b));
    let mut h = String::new();
    h.push_str("<!doctype html>\n<html><head><meta charset=\"utf-8\"><title>");
    h.push_str(&esc(&tr("Verificação do par de engrenagens")));
    h.push_str("</title><style>body{font-family:sans-serif;max-width:52em;margin:2em auto;padding:0 1em}table{border-collapse:collapse}td,th{border:1px solid #999;padding:.2em .6em;text-align:left}</style></head><body>\n");
    h.push_str(&format!("<h1>{}</h1>\n", esc(&tr("Verificação do par de engrenagens"))));
    h.push_str(&format!("<p>{}: {}</p>\n", esc(&tr("Método")), esc(&tr(e.metodo.rotulo()))));
    h.push_str(&format!("<p><em>{}</em></p>\n", esc(&tr(AVISO_PRELIMINAR))));
    h.push_str(&format!("<h2>{}</h2>\n<table>\n<tr><th></th><th>{}</th><th>{}</th></tr>\n", esc(&tr("Entradas")), esc(&tr("Pinhão")), esc(&tr("Coroa"))));
    h.push_str(&linha("Peça", pinhao.descricao(), coroa.descricao()));
    h.push_str(&linha("Material", tr(e.material_pinhao.nome), tr(e.material_coroa.nome)));
    h.push_str(&linha("Escoamento Sy (MPa)", f(e.material_pinhao.sy), f(e.material_coroa.sy)));
    h.push_str(&linha("Largura (mm)", f(pinhao.largura), f(coroa.largura)));
    h.push_str("</table>\n");
    h.push_str(&format!("<p>{}: {} kW · {} rpm · {}: {}</p>\n", esc(&tr("Potência e rotação do pinhão")), f(e.potencia_kw), f(e.rpm_pinhao), esc(&tr("segurança exigida")), f(e.fs_exigido)));
    h.push_str(&format!("<h2>{}</h2>\n<table>\n<tr><th></th><th>{}</th><th>{}</th></tr>\n", esc(&tr("Resultados")), esc(&tr("Pinhão")), esc(&tr("Coroa"))));
    let ou = |v: f64| if v == 0. { "—".to_string() } else { f(v) };
    h.push_str(&linha("Torque no pinhão (N·m)", f(r.torque_pinhao), String::new()));
    h.push_str(&linha("Força tangencial Ft (N)", f(r.forca_tangencial), String::new()));
    h.push_str(&linha("Força radial Fr (N)", f(r.forca_radial), String::new()));
    h.push_str(&linha("Velocidade no primitivo (m/s)", f(r.velocidade), String::new()));
    h.push_str(&linha("Fator dinâmico Kv", f(r.kv), String::new()));
    h.push_str(&linha("Fator de forma de Lewis Y", ou(r.y_pinhao), ou(r.y_coroa)));
    h.push_str(&linha("Tensão de flexão no pé (MPa)", ou(r.flexao_pinhao), ou(r.flexao_coroa)));
    h.push_str(&linha("Segurança à flexão", ou(r.fs_flexao_pinhao), ou(r.fs_flexao_coroa)));
    h.push_str(&linha("Pressão de contato de Hertz (MPa)", f(r.contato), String::new()));
    h.push_str(&linha("Segurança ao contato", ou(r.fs_contato), String::new()));
    h.push_str("</table>\n");
    if !r.avisos.is_empty() {
        h.push_str("<ul>\n");
        for a in &r.avisos {
            h.push_str(&format!("<li>{}</li>\n", esc(&tr(a))));
        }
        h.push_str("</ul>\n");
    }
    h.push_str("</body></html>\n");
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    fn par(z1: u32, z2: u32) -> (ParamEngrenagem, ParamEngrenagem) {
        (ParamEngrenagem { dentes: z1, ..Default::default() }, ParamEngrenagem { dentes: z2, ..Default::default() })
    }

    #[test]
    fn fator_de_lewis_do_dente_gerado_fica_na_faixa_dos_livros_e_cresce_com_z() {
        // tabelas de livro-texto para 20° de altura plena, carga no topo: Y ≈ 0,32 em z = 20 e ≈ 0,41 em z = 50
        let y20 = fator_lewis(&ParamEngrenagem { dentes: 20, ..Default::default() }).unwrap();
        let y50 = fator_lewis(&ParamEngrenagem { dentes: 50, ..Default::default() }).unwrap();
        assert!((0.29..0.36).contains(&y20), "Y(20) = {y20}");
        assert!((0.37..0.45).contains(&y50), "Y(50) = {y50}");
        let y12 = fator_lewis(&ParamEngrenagem { dentes: 12, ..Default::default() }).unwrap();
        assert!(y12 < y20 && y20 < y50);
        // o módulo não muda Y (fator adimensional)
        let y20m5 = fator_lewis(&ParamEngrenagem { dentes: 20, modulo: 5., ..Default::default() }).unwrap();
        assert!((y20m5 - y20).abs() < 1e-9);
        assert!(fator_lewis(&ParamEngrenagem { tipo: TipoEngrenagem::Interna, dentes: 60, ..Default::default() }).is_err());
    }

    #[test]
    fn verificacao_do_par_segue_as_formulas_fechadas() {
        let (p, c) = par(20, 40);
        let e = EntradaVerificacao { potencia_kw: 2., rpm_pinhao: 1000., ..Default::default() };
        let r = verificar_par(&p, &c, &e).unwrap();
        // T = P / ω; Ft = 2T / d1 com d1 = 40 mm
        let t = 2000. / (1000. * 2. * PI / 60.);
        assert!((r.torque_pinhao - t).abs() < 1e-9);
        assert!((r.forca_tangencial - 2. * t * 1e3 / 40.).abs() < 1e-6);
        assert!((r.forca_radial - r.forca_tangencial * 20_f64.to_radians().tan()).abs() < 1e-6);
        let v = 1000. * 2. * PI / 60. * 0.02;
        assert!((r.velocidade - v).abs() < 1e-9 && (r.kv - (6.1 + v) / 6.1).abs() < 1e-12);
        assert!((r.flexao_pinhao - r.kv * r.forca_tangencial / (10. * 2. * r.y_pinhao)).abs() < 1e-9);
        // a coroa (mais dentes) tem Y maior e tensão menor
        assert!(r.flexao_coroa < r.flexao_pinhao && r.fs_flexao_coroa > r.fs_flexao_pinhao);
        // Hertz com aço/aço: Cp ≈ 189,8 √MPa · √(205/200) ≈ 192
        let a = 20_f64.to_radians();
        let cp = (1. / (PI * 2. * (1. - 0.29 * 0.29) / 205_000.)).sqrt();
        let esperado = cp * (r.kv * r.forca_tangencial * (2. / (40. * a.sin()) + 2. / (80. * a.sin())) / (10. * a.cos())).sqrt();
        assert!((r.contato - esperado).abs() < 1e-9 && (185. ..200.).contains(&cp), "{cp}");
        assert_eq!(r.fs_contato, 0.);
        // dobrar a largura reduz a flexão à metade e o contato por √2
        let (p2, c2) = (ParamEngrenagem { largura: 20., ..p.clone() }, ParamEngrenagem { largura: 20., ..c.clone() });
        let r2 = verificar_par(&p2, &c2, &e).unwrap();
        assert!((r2.flexao_pinhao * 2. - r.flexao_pinhao).abs() < 1e-9 && (r2.contato * 2_f64.sqrt() - r.contato).abs() < 1e-9);
        // admissível de contato dado: compara e avisa se abaixo do exigido
        let r3 = verificar_par(&p, &c, &EntradaVerificacao { contato_admissivel: r.contato * 1.2, ..e.clone() }).unwrap();
        assert!((r3.fs_contato - 1.2).abs() < 1e-9 && r3.avisos.iter().any(|a| a.starts_with("contato")));
        // carga grande demais avisa na flexão
        let r4 = verificar_par(&p, &c, &EntradaVerificacao { potencia_kw: 50., ..e.clone() }).unwrap();
        assert!(r4.avisos.iter().any(|a| a.contains("flexão no pé do(a) pinhão")), "{:?}", r4.avisos);
    }

    #[test]
    fn interna_cremalheira_metodo_e_entradas() {
        let p = ParamEngrenagem { dentes: 20, ..Default::default() };
        let e = EntradaVerificacao::default();
        let ext = verificar_par(&p, &ParamEngrenagem { dentes: 60, ..Default::default() }, &e).unwrap();
        let int = verificar_par(&p, &ParamEngrenagem { tipo: TipoEngrenagem::Interna, dentes: 60, ..Default::default() }, &e).unwrap();
        let rack = verificar_par(&p, &ParamEngrenagem { tipo: TipoEngrenagem::Cremalheira, ..Default::default() }, &e).unwrap();
        // a curvatura côncava da interna alivia o contato; a cremalheira fica entre as duas
        assert!(int.contato < rack.contato && rack.contato < ext.contato);
        assert_eq!(int.y_coroa, 0.);
        assert!(verificar_par(&p, &p, &EntradaVerificacao { metodo: Metodo::Iso6336, ..e.clone() }).is_err());
        assert!(verificar_par(&p, &p, &EntradaVerificacao { potencia_kw: 0., ..e.clone() }).is_err());
        assert!(verificar_par(&p, &p, &EntradaVerificacao { contato_admissivel: -1., ..e }).is_err());
    }

    #[test]
    fn relatorio_traz_entradas_resultados_e_o_aviso() {
        let (p, c) = par(20, 40);
        let e = EntradaVerificacao::default();
        let r = verificar_par(&p, &c, &e).unwrap();
        let h = relatorio_html(&p, &c, &e, &r, &|s| s.to_string());
        assert!(h.starts_with("<!doctype html>") && h.contains("Engrenagem reta m2 z20") && h.contains("Engrenagem reta m2 z40"));
        assert!(h.contains(&format!("{:.3}", r.flexao_pinhao)) && h.contains(&format!("{:.3}", r.contato)));
        assert!(h.contains("Cálculo preliminar") && h.contains("Lewis e Hertz"));
        // o tradutor é aplicado
        let en = relatorio_html(&p, &c, &e, &r, &|s| if s == "Resultados" { "Results".into() } else { s.into() });
        assert!(en.contains("<h2>Results</h2>"));
    }
}
