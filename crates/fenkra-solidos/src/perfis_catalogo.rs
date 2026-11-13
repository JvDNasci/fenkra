// SPDX-License-Identifier: LGPL-2.1-or-later
//! Catálogo de perfis com dimensões e massa linear (B27).
//!
//! Cada linha traz as cotas da seção (as mesmas de `structural::dim_names`), a área e a massa linear
//! publicadas, a norma, a edição e o nível de confiança. A seção que o Fenkra desenha tem cantos vivos
//! e abas paralelas (sem raios de concordância), então a área calculada difere pouco da publicada; o
//! teste confere as duas contra a massa e contra a geometria.
//!
//! Nível de confiança: `Coerente` quer dizer que as cotas, a área e a massa são consistentes entre si
//! (massa = área × 7,85 g/cm³ dentro de 1 %, e a seção desenhada dentro de 6 % da área publicada, 8 % nos tubos), mas
//! a linha ainda não foi cotejada, uma a uma, com a tabela impressa do fabricante. Só `DuasFontes`
//! significa conferida em duas tabelas independentes; as linhas de tubo EN 10210 presentes nas duas tabelas colhidas (Eurocode Applied e AMESweb) têm esse nível.

use crate::perfis_catalogo_tubos as tubos;
use crate::structural::{self, Family};
use fenkra_core::SectionKind;

/// Densidade do aço dos perfis (g/cm³), a mesma do material padrão.
pub const DENSIDADE_ACO: f64 = 7.85;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Nivel {
    /// cotas, área e massa coerentes entre si; falta cotejar com a tabela do fabricante
    Coerente,
    /// conferida em duas tabelas independentes
    DuasFontes,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Linha {
    /// grupo da cascata de escolha (norma / fabricante)
    pub grupo: &'static str,
    /// nome da família ("Perfil W (AISC)")
    pub familia: &'static str,
    pub kind: SectionKind,
    pub designacao: String,
    /// cotas da seção em mm, na ordem de `structural::dim_names`
    pub dims: Vec<f64>,
    /// área publicada (cm²)
    pub area_cm2: f64,
    /// massa linear publicada (kg/m)
    pub massa_kg_m: f64,
    /// documento de onde vêm as cotas
    pub fonte: &'static str,
    pub edicao: &'static str,
    pub nivel: Nivel,
    /// segunda tabela independente que confere a linha (só em `DuasFontes`)
    pub fonte2: Option<&'static str>,
}

const POL: f64 = 25.4;
const POL2: f64 = 6.4516;
const LBFT: f64 = 1.488_164;

fn r2(v: f64) -> f64 {
    (v * 100.).round() / 100.
}

const AISC: (&str, &str) = ("AISC Steel Construction Manual, tabelas de dimensões e propriedades (W e C)", "15ª ed.");
const NBR: (&str, &str) = ("Catálogo de perfis laminados W e U (medidas da ASTM A6), fabricantes nacionais", "tabela vigente");
const EN365: (&str, &str) = ("EN 10365 (IPE, HEA, HEB, UPN)", "2017");
const EN56: (&str, &str) = ("EN 10056-1 (cantoneiras de abas iguais)", "2017");
const EN219: (&str, &str) = ("EN 10219-2 (tubos quadrados de seção fechada)", "2019");
const EN210: (&str, &str) = ("EN 10210-2, tabela de propriedades do Eurocode Applied (EN 1993)", "2019");
const EN210_2A: &str = "EN 10210-2, tabela do AMESweb";
const EN219_ROY: (&str, &str) = ("EN 10219-2, tabela do RoyMech (conformado a frio)", "1997");
const A500: (&str, &str) = ("ASTM A500, tabela de HSS quadrado do manual de aço americano (AMESweb)", "tabela vigente, colhida em 2026-11-13");
const A500_2: &str = "ASTM A500, tabela de HSS quadrado do Engineering ToolBox";
/// Família do HSS ASTM A500: a área publicada usa a parede de projeto (0,93 da nominal) e a massa, a
/// nominal; a conferência da massa leva isso em conta.
pub const FAMILIA_A500: &str = "Tubo quadrado HSS (ASTM A500)";

/// Linha em polegadas e libras por pé (tabelas americanas).
#[allow(clippy::too_many_arguments)]
fn pol(grupo: &'static str, familia: &'static str, kind: SectionKind, des: &str, d: &[f64], a_pol2: f64, lb_ft: f64, f: (&'static str, &'static str)) -> Linha {
    Linha { grupo, familia, kind, designacao: des.into(), dims: d.iter().map(|v| r2(v * POL)).collect(), area_cm2: r2(a_pol2 * POL2), massa_kg_m: r2(lb_ft * LBFT), fonte: f.0, edicao: f.1, nivel: Nivel::Coerente, fonte2: None }
}

/// Linha em milímetros, cm² e kg/m.
#[allow(clippy::too_many_arguments)]
fn mm(grupo: &'static str, familia: &'static str, kind: SectionKind, des: &str, d: &[f64], a: f64, m: f64, f: (&'static str, &'static str)) -> Linha {
    Linha { grupo, familia, kind, designacao: des.into(), dims: d.to_vec(), area_cm2: a, massa_kg_m: m, fonte: f.0, edicao: f.1, nivel: Nivel::Coerente, fonte2: None }
}

/// Todas as linhas do catálogo.
pub fn linhas() -> Vec<Linha> {
    use SectionKind::*;
    let mut v = vec![];
    // AISC: W (d, bf, tw, tf) em polegadas
    let w = |des: &str, d: [f64; 4], a: f64, p: f64| pol("AISC", "Perfil W (AISC)", PerfilI, des, &d, a, p, AISC);
    v.extend([
        w("W4x13", [4.16, 4.06, 0.280, 0.345], 3.83, 13.),
        w("W6x15", [5.99, 5.99, 0.230, 0.260], 4.43, 15.),
        w("W6x25", [6.38, 6.08, 0.320, 0.456], 7.34, 25.),
        w("W8x18", [8.14, 5.25, 0.230, 0.330], 5.26, 18.),
        w("W8x31", [8.00, 8.00, 0.285, 0.435], 9.13, 31.),
        w("W10x30", [10.5, 5.81, 0.300, 0.510], 8.84, 30.),
        w("W10x49", [10.0, 10.0, 0.340, 0.560], 14.4, 49.),
        w("W12x26", [12.2, 6.49, 0.230, 0.380], 7.65, 26.),
        w("W12x50", [12.2, 8.08, 0.370, 0.640], 14.6, 50.),
        w("W14x30", [13.8, 6.73, 0.270, 0.385], 8.85, 30.),
        w("W14x48", [13.8, 8.03, 0.340, 0.595], 14.1, 48.),
        w("W16x40", [16.0, 7.00, 0.305, 0.505], 11.8, 40.),
        w("W18x50", [18.0, 7.50, 0.355, 0.570], 14.7, 50.),
        w("W21x62", [21.0, 8.24, 0.400, 0.615], 18.3, 62.),
        w("W24x76", [23.9, 8.99, 0.440, 0.680], 22.4, 76.),
    ]);
    // AISC: C (d, bf, tw, tf)
    let c = |des: &str, d: [f64; 4], a: f64, p: f64| pol("AISC", "Perfil C (AISC)", PerfilU, des, &d, a, p, AISC);
    v.extend([
        c("C6x8.2", [6.0, 1.92, 0.200, 0.343], 2.40, 8.2),
        c("C8x11.5", [8.0, 2.26, 0.220, 0.390], 3.38, 11.5),
        c("C10x15.3", [10.0, 2.60, 0.240, 0.436], 4.48, 15.3),
        c("C12x20.7", [12.0, 2.94, 0.282, 0.501], 6.09, 20.7),
    ]);
    // Laminados W nacionais (h, b, tw, tf em mm; A em cm²; kg/m)
    let g = |des: &str, d: [f64; 4], a: f64, m: f64| mm("Gerdau / Aço Brasil", "Perfil W (nacional)", PerfilI, des, &d, a, m, NBR);
    v.extend([
        g("W150x13", [148., 100., 4.3, 4.9], 16.6, 13.0),
        g("W200x26,6", [207., 133., 5.8, 8.4], 34.2, 26.6),
        g("W360x32,9", [349., 127., 5.8, 8.5], 42.1, 32.9),
        g("W360x51", [355., 171., 7.2, 11.6], 64.8, 51.0),
        g("W460x52", [450., 152., 7.6, 10.8], 66.6, 52.0),
        g("W530x66", [525., 165., 8.9, 11.4], 83.6, 66.0),
    ]);
    let u = |des: &str, d: [f64; 4], a: f64, m: f64| mm("Gerdau / Aço Brasil", "Perfil U (nacional)", PerfilU, des, &d, a, m, NBR);
    v.extend([u("U 76,2x6,1", [76.2, 35.8, 4.32, 6.93], 7.78, 6.1), u("U 101,6x7,95", [101.6, 40.2, 4.57, 7.52], 10.1, 7.95), u("U 152,4x12,2", [152.4, 48.8, 5.08, 8.71], 15.5, 12.2)]);
    // EN 10365: IPE (h, b, tw, tf)
    let ipe = |h: f64, b: f64, tw: f64, tf: f64, a: f64, m: f64| mm("EN 10365", "IPE", PerfilI, &format!("IPE {h}"), &[h, b, tw, tf], a, m, EN365);
    v.extend([
        ipe(100., 55., 4.1, 5.7, 10.3, 8.1),
        ipe(120., 64., 4.4, 6.3, 13.2, 10.4),
        ipe(140., 73., 4.7, 6.9, 16.4, 12.9),
        ipe(160., 82., 5.0, 7.4, 20.1, 15.8),
        ipe(180., 91., 5.3, 8.0, 23.9, 18.8),
        ipe(200., 100., 5.6, 8.5, 28.5, 22.4),
        ipe(220., 110., 5.9, 9.2, 33.4, 26.2),
        ipe(240., 120., 6.2, 9.8, 39.1, 30.7),
        ipe(270., 135., 6.6, 10.2, 45.9, 36.1),
        ipe(300., 150., 7.1, 10.7, 53.8, 42.2),
        ipe(330., 160., 7.5, 11.5, 62.6, 49.1),
        ipe(360., 170., 8.0, 12.7, 72.7, 57.1),
        ipe(400., 180., 8.6, 13.5, 84.5, 66.3),
        ipe(450., 190., 9.4, 14.6, 98.8, 77.6),
        ipe(500., 200., 10.2, 16.0, 116., 90.7),
        ipe(600., 220., 12.0, 19.0, 156., 122.),
    ]);
    let hea = |n: f64, h: f64, tw: f64, tf: f64, a: f64, m: f64| mm("EN 10365", "HEA", PerfilI, &format!("HEA {n}"), &[h, n, tw, tf], a, m, EN365);
    v.extend([
        hea(100., 96., 5.0, 8.0, 21.2, 16.7),
        hea(120., 114., 5.0, 8.0, 25.3, 19.9),
        hea(140., 133., 5.5, 8.5, 31.4, 24.7),
        hea(160., 152., 6.0, 9.0, 38.8, 30.4),
        hea(180., 171., 6.0, 9.5, 45.3, 35.5),
        hea(200., 190., 6.5, 10.0, 53.8, 42.3),
        hea(220., 210., 7.0, 11.0, 64.3, 50.5),
        hea(240., 230., 7.5, 12.0, 76.8, 60.3),
        hea(260., 250., 7.5, 12.5, 86.8, 68.2),
        hea(280., 270., 8.0, 13.0, 97.3, 76.4),
        hea(300., 290., 8.5, 14.0, 112.5, 88.3),
    ]);
    let heb = |n: f64, tw: f64, tf: f64, a: f64, m: f64| mm("EN 10365", "HEB", PerfilI, &format!("HEB {n}"), &[n, n, tw, tf], a, m, EN365);
    v.extend([
        heb(100., 6.0, 10.0, 26.0, 20.4),
        heb(120., 6.5, 11.0, 34.0, 26.7),
        heb(140., 7.0, 12.0, 43.0, 33.7),
        heb(160., 8.0, 13.0, 54.3, 42.6),
        heb(180., 8.5, 14.0, 65.3, 51.2),
        heb(200., 9.0, 15.0, 78.1, 61.3),
        heb(220., 9.5, 16.0, 91.0, 71.5),
        heb(240., 10.0, 17.0, 106., 83.2),
        heb(260., 10.0, 17.5, 118.4, 93.0),
        heb(280., 10.5, 18.0, 131.4, 103.1),
        heb(300., 11.0, 19.0, 149.1, 117.0),
    ]);
    let upn = |h: f64, b: f64, tw: f64, tf: f64, a: f64, m: f64| mm("EN 10365", "UPN", PerfilU, &format!("UPN {h}"), &[h, b, tw, tf], a, m, EN365);
    v.extend([
        upn(80., 45., 6.0, 8.0, 11.0, 8.64),
        upn(100., 50., 6.0, 8.5, 13.5, 10.6),
        upn(120., 55., 7.0, 9.0, 17.0, 13.4),
        upn(140., 60., 7.0, 10.0, 20.4, 16.0),
        upn(160., 65., 7.5, 10.5, 24.0, 18.8),
        upn(180., 70., 8.0, 11.0, 28.0, 22.0),
        upn(200., 75., 8.5, 11.5, 32.2, 25.3),
        upn(220., 80., 9.0, 12.5, 37.4, 29.4),
        upn(240., 85., 9.5, 13.0, 42.3, 33.2),
        upn(260., 90., 10.0, 14.0, 48.3, 37.9),
        upn(300., 100., 10.0, 16.0, 58.8, 46.2),
    ]);
    // EN 10056-1: cantoneiras de abas iguais (a, a, t)
    let l = |a: f64, t: f64, area: f64, m: f64| mm("EN 10056", "Cantoneira L (EN)", Cantoneira, &format!("L {a}x{a}x{t}"), &[a, a, t], area, m, EN56);
    v.extend([
        l(30., 3., 1.74, 1.36),
        l(40., 4., 3.08, 2.42),
        l(50., 5., 4.80, 3.77),
        l(60., 6., 6.91, 5.42),
        l(70., 7., 9.40, 7.38),
        l(80., 8., 12.3, 9.66),
        l(90., 9., 15.5, 12.2),
        l(100., 10., 19.2, 15.1),
        l(120., 12., 27.5, 21.6),
        l(150., 15., 43.0, 33.8),
    ]);
    // EN 10219: tubos quadrados (lado, parede)
    let s = |b: f64, t: f64, a: f64, m: f64| mm("EN 10219", "Tubo quadrado (EN 10219)", TuboQuadrado, &format!("{b}x{b}x{t}"), &[b, t], a, m, EN219);
    v.extend([
        s(40., 3., 4.34, 3.41),
        s(50., 3., 5.59, 4.39),
        s(50., 4., 6.94, 5.45),
        s(60., 3., 6.95, 5.45),
        s(60., 4., 8.79, 6.90),
        s(80., 4., 12.0, 9.41),
        s(80., 5., 14.9, 11.7),
        s(100., 4., 15.2, 11.9),
        s(100., 5., 18.7, 14.7),
        s(100., 6., 22.2, 17.4),
        s(120., 5., 22.7, 17.8),
        s(120., 8., 35.2, 27.6),
    ]);
    v.extend(tubos_en());
    v
}

/// Tubos redondos, quadrados e retangulares das tabelas colhidas (dados em `perfis_catalogo_tubos`).
fn tubos_en() -> Vec<Linha> {
    use SectionKind::*;
    let mut v = vec![];
    let mut tab = |dados: &[tubos::Dado], grupo: &'static str, familia: &'static str, kind: SectionKind, f: (&'static str, &'static str), des: &dyn Fn(&[f64]) -> String| {
        for (d, a, m, duas) in dados {
            let (nivel, fonte2) = if *duas { (Nivel::DuasFontes, Some(EN210_2A)) } else { (Nivel::Coerente, None) };
            v.push(Linha { grupo, familia, kind, designacao: des(d), dims: d.to_vec(), area_cm2: *a, massa_kg_m: *m, fonte: f.0, edicao: f.1, nivel, fonte2 });
        }
    };
    tab(tubos::CHS_10210, "EN 10210", "Tubo redondo (EN 10210)", TuboRedondo, EN210, &|d| format!("CHS {}x{}", d[0], d[1]));
    tab(tubos::SHS_10210, "EN 10210", "Tubo quadrado (EN 10210)", TuboQuadrado, EN210, &|d| format!("SHS {}x{}x{}", d[0], d[0], d[1]));
    tab(tubos::RHS_10210, "EN 10210", "Tubo retangular (EN 10210)", TuboRetangular, EN210, &|d| format!("RHS {}x{}x{}", d[1], d[0], d[2]));
    tab(tubos::RHS_10219, "EN 10219", "Tubo retangular (EN 10219)", TuboRetangular, EN219_ROY, &|d| format!("RHS {}x{}x{}", d[1], d[0], d[2]));
    // HSS quadrado A500: cotas com a parede de projeto, que dá a área publicada
    for (b, t_nom, t_proj, a, wt) in tubos::HSS_A500_QUADRADO {
        let lado = if b.fract() == 0. { format!("{b:.0}") } else { format!("{b}") };
        v.push(Linha {
            grupo: "ASTM A500 (AISC)",
            familia: FAMILIA_A500,
            kind: TuboQuadrado,
            designacao: format!("HSS {lado}x{lado}x{t_nom}"),
            dims: vec![r2(b * POL), r2(t_proj * POL)],
            area_cm2: r2(a * POL2),
            massa_kg_m: r2(wt * LBFT),
            fonte: A500.0,
            edicao: A500.1,
            nivel: Nivel::DuasFontes,
            fonte2: Some(A500_2),
        });
    }
    v
}

/// Famílias do catálogo no formato da cascata de escolha (uma por `familia`, no grupo da norma).
pub fn familias() -> Vec<Family> {
    let mut out: Vec<Family> = vec![];
    for l in linhas() {
        match out.iter_mut().find(|f| f.name == l.familia) {
            Some(f) => f.sizes.push((l.designacao.clone(), l.dims.clone())),
            None => out.push(Family { name: l.familia.into(), norm: l.grupo.into(), kind: l.kind, sizes: vec![(l.designacao.clone(), l.dims.clone())] }),
        }
    }
    out
}

/// Linha do catálogo para uma família e designação.
pub fn buscar(familia: &str, designacao: &str) -> Option<Linha> {
    linhas().into_iter().find(|l| l.familia == familia && l.designacao == designacao)
}

/// Massa linear (kg/m) de uma seção da família e designação do catálogo; `None` se não consta.
pub fn massa_linear(familia: &str, designacao: &str) -> Option<f64> {
    buscar(familia, designacao).map(|l| l.massa_kg_m)
}

/// Área (cm²) da seção que o Fenkra desenha (laços de segmentos retos e círculos; furos subtraem).
pub fn area_da_secao_cm2(kind: SectionKind, d: &[f64]) -> Option<f64> {
    use fenkra_kernel::Seg2;
    let (loops, ..) = structural::section(kind, d).ok()?;
    let mut total = 0.;
    for (i, l) in loops.iter().enumerate() {
        let mut a = 0.;
        for s in l {
            match s {
                Seg2::Line(p, q) => a += (p[0] * q[1] - q[0] * p[1]) / 2.,
                Seg2::Circle(_, r) => a += std::f64::consts::PI * r * r,
                _ => return None,
            }
        }
        // o primeiro laço é o contorno, os demais são furos
        total += if i == 0 { a.abs() } else { -a.abs() };
    }
    Some(total / 100.)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn massa_publicada_confere_com_a_area_em_1_por_cento() {
        let mut falhas = vec![];
        for l in linhas() {
            // kg/m = cm² × 100 cm/m × 7,85 g/cm³ ÷ 1000 g/kg
            let mut calc = l.area_cm2 * DENSIDADE_ACO / 10.;
            if l.familia == FAMILIA_A500 {
                // a área do HSS usa a parede de projeto (0,93 t) e a massa, a nominal: volta à nominal.
                // Paredes grossas de lado pequeno passam de 1 % pelos raios dos cantos; o teste próprio
                // do A500 confere as duas fontes entre si e o desvio aqui fica em 2,5 %
                calc /= 0.93;
                let erro = (calc - l.massa_kg_m).abs() / l.massa_kg_m;
                if erro > 0.025 {
                    falhas.push(format!("{} {}: área nominal dá {calc:.2} kg/m, publicado {} ({:.2} %)", l.familia, l.designacao, l.massa_kg_m, erro * 100.));
                }
                continue;
            }
            let erro = (calc - l.massa_kg_m).abs() / l.massa_kg_m;
            if erro > 0.01 {
                falhas.push(format!("{} {}: área {} cm² dá {calc:.2} kg/m, publicado {} ({:.2} %)", l.familia, l.designacao, l.area_cm2, l.massa_kg_m, erro * 100.));
            }
        }
        assert!(falhas.is_empty(), "{}", falhas.join("
"));
    }

    #[test]
    fn secao_desenhada_fica_perto_da_area_publicada() {
        // a seção do Fenkra não tem raios de concordância nem abas inclinadas: até 6 % de diferença
        // nos perfis abertos e 9 % nos tubos de perfil fechado (os cantos externos vivos somam área, mais nos tubos pequenos de parede grossa)
        let mut falhas = vec![];
        for l in linhas() {
            let Some(a) = area_da_secao_cm2(l.kind, &l.dims) else {
                falhas.push(format!("{} {}: seção inválida", l.familia, l.designacao));
                continue;
            };
            let erro = (a - l.area_cm2).abs() / l.area_cm2;
            let lim = if matches!(l.kind, SectionKind::TuboQuadrado | SectionKind::TuboRetangular) { 0.09 } else { 0.06 };
            if erro > lim {
                falhas.push(format!("{} {}: seção desenhada {a:.2} cm², publicada {} ({:.1} %)", l.familia, l.designacao, l.area_cm2, erro * 100.));
            }
        }
        assert!(falhas.is_empty(), "{}", falhas.join("
"));
    }

    /// B27: o HSS quadrado A500 vem das duas tabelas; a massa e a área de cada linha estão nas duas
    /// a 1 % (conferido no script de colheita) e a seção desenhada com a parede de projeto bate com a
    /// área publicada.
    #[test]
    fn hss_a500_quadrado_com_duas_fontes() {
        let ls: Vec<Linha> = linhas().into_iter().filter(|l| l.familia == FAMILIA_A500).collect();
        assert!(ls.len() >= 50, "{}", ls.len());
        assert!(ls.iter().all(|l| l.nivel == Nivel::DuasFontes && l.fonte2.is_some()));
        // HSS 4x4x1/4: 12,18 lb/ft e 3,37 pol²
        let l = buscar(FAMILIA_A500, "HSS 4x4x1/4").expect("HSS 4x4x1/4");
        assert!((l.dims[0] - 101.6).abs() < 1e-9 && (l.dims[1] - 5.92).abs() < 0.01, "{:?}", l.dims);
        assert!((l.massa_kg_m - 18.13).abs() < 0.02, "{}", l.massa_kg_m);
        assert!((l.area_cm2 - 21.74).abs() < 0.05, "{}", l.area_cm2);
    }

    #[test]
    fn designacoes_unicas_e_fonte_preenchida() {
        let ls = linhas();
        for (i, a) in ls.iter().enumerate() {
            assert!(!a.fonte.is_empty() && !a.edicao.is_empty() && !a.grupo.is_empty());
            assert_eq!(a.dims.len(), structural::dim_names(a.kind).len(), "{} {}", a.familia, a.designacao);
            for b in &ls[i + 1..] {
                assert!(!(a.familia == b.familia && a.designacao == b.designacao), "repetida: {} {}", a.familia, a.designacao);
            }
        }
        // só se declara conferida em duas fontes a linha que nomeia a segunda
        assert!(ls.iter().all(|l| (l.nivel == Nivel::DuasFontes) == l.fonte2.is_some()));
        assert!(ls.iter().filter(|l| l.nivel == Nivel::DuasFontes).count() >= 40);
    }

    #[test]
    fn familias_do_catalogo_geram_secoes_e_consultam_massa() {
        let fams = familias();
        assert!(fams.len() >= 9);
        for f in &fams {
            for (n, d) in &f.sizes {
                assert!(structural::section(f.kind, d).is_ok(), "{} {n}", f.name);
            }
        }
        let m = massa_linear("IPE", "IPE 200").unwrap();
        assert!((m - 22.4).abs() < 1e-9);
        // lb/ft → kg/m: W8x31 = 46,13 kg/m
        assert!((massa_linear("Perfil W (AISC)", "W8x31").unwrap() - 46.13).abs() < 0.01);
        assert!(massa_linear("IPE", "IPE 999").is_none());
    }
}
