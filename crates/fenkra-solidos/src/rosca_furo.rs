// SPDX-License-Identifier: LGPL-2.1-or-later
//! B351: onde a rosca modelada de um furo começa e termina.
//!
//! Como funciona no kernel: a rosca modelada é um perfil em V varrido ao longo de uma hélice e
//! subtraído do corpo. O perfil tem largura (a "meia largura" de cada lado do eixo da hélice), então
//! o sulco ocupa de `início - meia` até `fim + meia` ao longo do eixo. Se a hélice termina exatamente
//! no fundo do furo, ou começa exatamente onde o escareado encontra o cilindro, a ponta do perfil
//! encosta numa face quase tangente e a booleana do OpenCASCADE devolve o corpo sem o sulco, sem
//! avisar (foi o defeito do furo escareado e do furo cego). A solução é geométrica: o sulco inteiro
//! cabe na parte cilíndrica. O início desce uma meia largura abaixo do escareado ou do rebaixo; o fim
//! sobe uma meia largura acima do fundo; e num furo passante a hélice termina bem fora da peça, onde
//! só corta ar.
//!
//! A prévia durante a edição não constrói nada disso (`Cache::leve`): só as linhas da hélice.

use crate::furo_norma::comprimento_de_rosca;
use crate::geo3d::V3;
use crate::hardware::ThreadSpec;
use fenkra_kernel::Shape;

/// Voltas de cada trecho da ferramenta. Uma varredura de muitas voltas vira uma face só, de
/// B-spline com dezenas de vãos, e a interseção dela com o cilindro do furo falha: a booleana devolve
/// o corpo intacto ou um sólido inválido, sem erro. Com até duas voltas por trecho o resultado saiu
/// correto e válido na maioria dos casos medidos (furo cego, passante, escareado, rebaixado); com três
/// já houve sólido inválido.
pub const VOLTAS_POR_TRECHO: f64 = 2.;

/// Escada de tentativas quando a booleana ainda falha (erro, ou o corpo sai sem o sulco): trechos de
/// outro tamanho e o início deslocado uma fração do passo, que muda onde a ponta do perfil toca as
/// arestas do furo. Cada degrau é (voltas por trecho, deslocamento do início em passos).
pub const ESCADA: [(f64, f64); 5] = [(VOLTAS_POR_TRECHO, 0.), (1., 0.), (VOLTAS_POR_TRECHO, 0.23), (1., 0.41), (3., 0.67)];

/// Ferramenta do sulco, no primeiro degrau da escada.
pub fn ferramenta(origem: V3, dir: V3, rosca: &ThreadSpec, comprimento: f64, interna: bool) -> Result<Shape, String> {
    ferramenta_no_degrau(origem, dir, rosca, comprimento, interna, 0)
}

/// Ferramenta do sulco no degrau `degrau` da [`ESCADA`]: trechos de `voltas` voltas, um depois do outro
/// ao longo do eixo. O comprimento de cada trecho é múltiplo do passo, então o perfil de um começa onde
/// o anterior termina, na mesma fase da hélice. O fim do sulco não muda com o degrau.
pub fn ferramenta_no_degrau(origem: V3, dir: V3, rosca: &ThreadSpec, comprimento: f64, interna: bool, degrau: usize) -> Result<Shape, String> {
    let (voltas, desloca) = ESCADA[degrau.min(ESCADA.len() - 1)];
    let trecho = voltas * rosca.pitch;
    let mut feito = desloca * rosca.pitch;
    let mut ferramenta = Shape::empty();
    while feito < comprimento - 1e-9 {
        let l = trecho.min(comprimento - feito);
        let o = [0, 1, 2].map(|i| origem[i] + dir[i] * feito);
        let parte = if interna { Shape::thread_tool(o, dir, rosca.major, rosca.pitch, l, rosca.angle) } else { Shape::thread_tool_ext(o, dir, rosca.major, rosca.pitch, l, rosca.angle) }.map_err(|e| e.0)?;
        ferramenta = if ferramenta.is_null() { parte } else { ferramenta.compound(&parte) };
        feito += l;
    }
    if ferramenta.is_null() {
        return Err("rosca sem comprimento".into());
    }
    Ok(ferramenta)
}

/// O comando tem rosca modelada (furo com rosca modelada ou o comando Rosca modelado)?
pub fn modelada(k: &fenkra_core::FeatureKind) -> bool {
    match k {
        fenkra_core::FeatureKind::Hole(h) => h.thread.is_some() && h.thread_modeled,
        fenkra_core::FeatureKind::Thread(t) => t.modeled,
        _ => false,
    }
}

/// Profundidade do filete (ISO), a mesma do kernel.
pub fn profundidade_do_filete(passo: f64) -> f64 {
    0.6134 * passo
}

/// Metade da largura axial do perfil na raiz: o que o sulco avança para cada lado da hélice.
pub fn meia_largura(rosca: &ThreadSpec) -> f64 {
    (rosca.angle / 2.).to_radians().tan() * (profundidade_do_filete(rosca.pitch) + 0.1 * rosca.pitch)
}

/// Início e comprimento da hélice, medidos a partir da boca do furo, ao longo do eixo.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlanoDaRosca {
    /// quanto a hélice começa abaixo da boca (0 = na boca; o perfil sai para o ar)
    pub inicio: f64,
    /// comprimento da hélice
    pub comprimento: f64,
}

/// Plano da hélice de um furo.
///
/// - `cabeca`: profundidade do escareado/rebaixo (0 num furo simples);
/// - `limite`: profundidade do furo, ou do material se for menor;
/// - `material`: extensão do material a partir do ponto, na direção do furo;
/// - `profundidade_rosca` e `folga`: ajustes de norma (B90);
/// - `passante`: o furo atravessa a peça (sem fundo).
///
/// Devolve `None` quando a rosca não cabe (menos de um passo).
pub fn plano(rosca: &ThreadSpec, cabeca: f64, limite: f64, material: f64, profundidade_rosca: Option<f64>, folga: f64, passante: bool) -> Option<PlanoDaRosca> {
    let meia = meia_largura(rosca);
    let inicio = if cabeca > 1e-9 { cabeca + meia } else { 0. };
    // até onde o sulco vai, medido da boca
    let ate = comprimento_de_rosca(limite, profundidade_rosca, folga, rosca.pitch);
    let sai_pelo_ar = passante && profundidade_rosca.is_none() && folga <= 1e-12;
    let comprimento = if sai_pelo_ar { material - inicio + 2. * meia } else { ate - inicio - meia };
    (comprimento >= rosca.pitch).then_some(PlanoDaRosca { inicio, comprimento })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hardware::thread;

    fn m8() -> &'static ThreadSpec {
        thread("M8x1.25").expect("M8x1.25 na tabela")
    }

    #[test]
    fn meia_largura_de_m8_e_meio_milimetro() {
        // tan(30°) * (0,6134 * 1,25 + 0,125)
        assert!((meia_largura(m8()) - 0.5149).abs() < 1e-3, "{}", meia_largura(m8()));
    }

    #[test]
    fn furo_cego_termina_uma_meia_largura_acima_do_fundo() {
        let p = plano(m8(), 0., 20., 30., None, 0., false).unwrap();
        assert_eq!(p.inicio, 0.);
        assert!((p.inicio + p.comprimento + meia_largura(m8()) - 20.).abs() < 1e-9);
    }

    #[test]
    fn escareado_comeca_uma_meia_largura_abaixo_do_cone() {
        let p = plano(m8(), 4.6, 20., 30., None, 0., false).unwrap();
        assert!((p.inicio - (4.6 + meia_largura(m8()))).abs() < 1e-9);
        assert!(p.inicio + p.comprimento + meia_largura(m8()) <= 20. + 1e-9);
    }

    #[test]
    fn furo_passante_termina_fora_da_peca() {
        let p = plano(m8(), 4.6, 30., 30., None, 0., true).unwrap();
        assert!(p.inicio + p.comprimento > 30. + meia_largura(m8()));
    }

    #[test]
    fn passante_com_folga_ou_profundidade_para_antes() {
        let p = plano(m8(), 0., 30., 30., Some(12.), 0., true).unwrap();
        assert!((p.inicio + p.comprimento + meia_largura(m8()) - 12.).abs() < 1e-9);
        let q = plano(m8(), 0., 30., 30., None, 2., true).unwrap();
        assert!(q.inicio + q.comprimento < 30.);
    }

    #[test]
    fn rosca_menor_que_um_passo_nao_cabe() {
        assert!(plano(m8(), 4.6, 6., 30., None, 0., false).is_none());
    }
}
