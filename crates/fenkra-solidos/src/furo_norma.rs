// SPDX-License-Identifier: LGPL-2.1-or-later
//! B90: furo guiado por norma. Nome da feature com os valores, profundidade de rosca separada da
//! profundidade do furo, folga no fundo em passos de rosca e deslocamento da ponta da broca.
//!
//! Como funciona no kernel: o furo é um cilindro (ou cone, se for cônico) mais o fundo em V da broca,
//! subtraído do corpo. A broca para na profundidade pedida e a ponta em V avança além dela por
//! `raio / tan(ponta/2)`; o deslocamento da ponta soma (ou tira) esse tanto no cilindro, para que o
//! corte não invada outra parede. A rosca modelada é outro corte, helicoidal, que termina no
//! comprimento de rosca: o que vier de "profundidade de rosca", ou a profundidade do furo menos a
//! folga no fundo (em passos), para o macho não bater no fundo.

use super::*;

/// Profundidade do cilindro da broca depois do deslocamento da ponta.
pub fn profundidade_com_ponta(profundidade: f64, deslocamento: f64) -> f64 {
    profundidade + deslocamento
}

/// Deslocamento que faz a ponta em V terminar exatamente na profundidade pedida (a ponta conta na
/// profundidade): negativo, igual ao comprimento do cone.
pub fn deslocamento_ponta_dentro(diametro: f64, angulo_ponta: f64) -> f64 {
    if angulo_ponta <= 1e-6 || angulo_ponta >= 179.9 {
        return 0.;
    }
    -(diametro / 2.) / (angulo_ponta.to_radians() / 2.).tan()
}

/// Comprimento da rosca: profundidade de rosca pedida, ou a do furo menos `folga` passos.
/// Nunca passa da profundidade do furo.
pub fn comprimento_de_rosca(profundidade_furo: f64, profundidade_rosca: Option<f64>, folga_passos: f64, passo: f64) -> f64 {
    let c = match profundidade_rosca {
        Some(p) => p,
        None => profundidade_furo - folga_passos * passo,
    };
    c.min(profundidade_furo)
}

fn num(v: f64) -> String {
    let r = (v * 100.).round() / 100.;
    if (r - r.round()).abs() < 1e-9 {
        format!("{}", r.round() as i64)
    } else {
        format!("{r}").replace('.', ",")
    }
}

/// Tamanho que dá nome ao furo: designação da rosca sem o passo ("M8x1.25" vira "M8"), a norma
/// ("M8 normal" vira "M8") ou o diâmetro.
fn tamanho(h: &Hole, diametro: Option<f64>) -> String {
    if let Some(t) = &h.thread {
        return t.split(['x', 'X']).next().unwrap_or(t).trim().to_string();
    }
    if let Some(s) = h.standard.as_deref() {
        if let Some(p) = s.split_whitespace().next() {
            if p.starts_with(|c: char| c.is_ascii_uppercase()) && p.chars().skip(1).all(|c| c.is_ascii_digit() || c == '.' || c == ',') {
                return p.to_string();
            }
        }
    }
    diametro.map_or_else(|| "Ø?".to_string(), |d| format!("Ø{}", num(d)))
}

/// Nome da feature de furo com os valores: "Furo M8 roscado x 20", "Furo Ø6,6 escareado 90° x passante".
/// Expressões que não avaliam ficam como estão escritas.
pub fn nome_do_furo(h: &Hole, vars: &Vars) -> String {
    let v = |e: &str, n: &str| ev(e, vars, n).ok();
    let diam = v(&h.diameter, "diâmetro");
    let tam = tamanho(h, diam);
    let forma = match &h.kind {
        HoleKind::Simples => String::new(),
        HoleKind::Rebaixado { .. } => " rebaixado".to_string(),
        HoleKind::Escareado { angle, .. } => match v(angle, "ângulo") {
            Some(a) => format!(" escareado {}°", num(a)),
            None => " escareado".to_string(),
        },
        HoleKind::Contrafurado { .. } => " contrafurado".to_string(),
    };
    let rosca = if h.thread.is_some() { " roscado" } else { "" };
    let conico = if v(&h.taper, "conicidade").is_some_and(|t| t.abs() > 1e-9) { " cônico" } else { "" };
    let prof = match &h.depth {
        Extent::Blind(d) | Extent::Symmetric(d) | Extent::TwoSides(d, _) => v(d, "profundidade").map_or_else(|| d.clone(), num),
        Extent::ThroughAll => "passante".to_string(),
        Extent::UpToNext => "até o próximo".to_string(),
        Extent::UpToLast => "até o último".to_string(),
        Extent::UpTo(_) => "até plano".to_string(),
        Extent::UpToSurface(_) => "até superfície".to_string(),
    };
    format!("Furo {tam}{rosca}{conico}{forma} x {prof}")
}

/// Marca de nome automático: "Furo" puro ou o que `nome_do_furo` produz ("Furo ... x ...").
pub fn nome_e_automatico(nome: &str) -> bool {
    let n = nome.trim();
    n == "Furo" || (n.starts_with("Furo ") && n.contains(" x "))
}

/// Atualiza o nome de cada furo cujo nome é automático para os valores atuais. Nomes escolhidos à
/// mão ficam como estão. Devolve quantos nomes mudaram.
pub fn renomear_furos(doc: &mut Document) -> usize {
    let Ok(vars) = doc.eval_params() else { return 0 };
    let mut mudou = 0;
    for f in doc.features.iter_mut() {
        let FeatureKind::Hole(h) = &f.kind else { continue };
        if !nome_e_automatico(&f.name) {
            continue;
        }
        let novo = nome_do_furo(h, &vars);
        if novo != f.name {
            f.name = novo;
            mudou += 1;
        }
    }
    mudou
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rosca_separada_e_folga_em_passos() {
        // furo de 20 com folga de 2 passos de 1,25: rosca de 17,5
        assert!((comprimento_de_rosca(20., None, 2., 1.25) - 17.5).abs() < 1e-12);
        // profundidade de rosca explícita vale mais que a folga
        assert_eq!(comprimento_de_rosca(20., Some(12.), 2., 1.25), 12.);
        // nunca passa do furo
        assert_eq!(comprimento_de_rosca(20., Some(30.), 0., 1.25), 20.);
    }

    #[test]
    fn ponta_dentro_da_profundidade_e_o_cone_inteiro() {
        // Ø8 e 118°: cone de 4 / tan(59°) = 2,40 mm
        let d = deslocamento_ponta_dentro(8., 118.);
        assert!((d + 4. / 59f64.to_radians().tan()).abs() < 1e-12);
        assert_eq!(deslocamento_ponta_dentro(8., 0.), 0.);
    }

    #[test]
    fn nome_automatico_so_vale_para_nomes_automaticos() {
        assert!(nome_e_automatico("Furo"));
        assert!(nome_e_automatico("Furo M8 roscado x 20"));
        assert!(!nome_e_automatico("Furo do eixo"));
    }
}
