// SPDX-License-Identifier: LGPL-2.1-or-later
//! B351 e B373: onde a rosca modelada de um furo começa e termina, e como o sulco é construído.
//!
//! Como funciona no kernel: a rosca modelada é um perfil em V varrido ao longo de uma hélice e
//! subtraído do corpo. O perfil tem largura (a "meia largura" de cada lado do eixo da hélice), então
//! o sulco ocupa de `início - meia` até `fim + meia` ao longo do eixo.
//!
//! - **Entrada contínua (B373).** Se a rosca começa na boca do furo, a hélice começa um passo acima
//!   dela, no ar, e atravessa a boca. Assim o sulco nasce na boca como o de um macho de verdade, sem o
//!   degrau que a ponta cortada da hélice deixava na primeira volta, e o parafuso entra liso.
//! - **Fim.** Se a hélice termina exatamente no fundo do furo, ou começa exatamente onde o escareado
//!   encontra o cilindro, a ponta do perfil encosta numa face quase tangente e a booleana do OpenCASCADE
//!   devolve o corpo sem o sulco, sem avisar (B351). Por isso o início desce uma meia largura abaixo do
//!   escareado ou do rebaixo, o fim sobe uma meia largura acima do fundo, e num furo passante a hélice
//!   termina bem fora da peça, onde só corta ar.
//! - **Ângulo de início (B373).** A hélice é girada em torno do eixo: o ângulo é onde o sulco cruza a
//!   boca (ou onde ele começa, se há folga em cima ou rebaixo), medido a partir de uma direção de
//!   referência perpendicular ao eixo.
//! - **Emendas.** A varredura é feita em trechos de poucas voltas (a booleana não aguenta mais), cada
//!   um começando no ângulo exato em que o anterior termina.
//!
//! A prévia durante a edição não constrói nada disso (`Cache::leve`): só as linhas da hélice.

use crate::furo_norma::comprimento_de_rosca;
use crate::geo3d::V3;
use crate::hardware::ThreadSpec;
use fenkra_kernel::{Bool, HoleShape, Shape, Xform};

/// Voltas de cada trecho da ferramenta. Uma varredura de muitas voltas vira uma face só, de
/// B-spline com dezenas de vãos, e a interseção dela com o cilindro do furo falha: a booleana devolve
/// o corpo intacto ou um sólido inválido, sem erro. Com até duas voltas por trecho o resultado saiu
/// correto e válido na maioria dos casos medidos (furo cego, passante, escareado, rebaixado); com três
/// já houve sólido inválido.
pub const VOLTAS_POR_TRECHO: f64 = 2.;

/// Escada de tentativas quando a booleana ainda falha (erro, ou o corpo sai sem o sulco): trechos de
/// outro tamanho, que mudam onde as emendas caem em relação às arestas do furo. Cada degrau é (voltas
/// por trecho, voltas do primeiro trecho). B373: a fase da hélice nunca muda entre os degraus, porque o
/// ângulo de início é uma escolha do usuário; o início do sulco também não anda (antes os degraus
/// deslocavam o início para dentro do furo).
pub const ESCADA: [(f64, f64); 7] = [(VOLTAS_POR_TRECHO, 2.), (1., 1.), (VOLTAS_POR_TRECHO, 1.), (1.5, 1.5), (3., 3.), (1., 0.5), (2.5, 1.25)];

/// Onde a hélice começa em volta do eixo e para que lado ela gira.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fase {
    /// direção de referência, perpendicular ao eixo e unitária (ângulo 0)
    pub referencia: V3,
    /// ângulo de início em graus, positivo no sentido da regra da mão direita em torno do eixo de entrada
    pub angulo: f64,
    /// rosca à esquerda
    pub esquerda: bool,
}

impl Fase {
    /// Fase padrão: referência automática, ângulo 0, à direita.
    pub fn padrao(dir: V3) -> Fase {
        Fase { referencia: referencia(dir, None), angulo: 0., esquerda: false }
    }
}

fn produto_vetorial(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

fn normaliza(v: V3) -> Option<V3> {
    let n = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    (n > 1e-9).then(|| [v[0] / n, v[1] / n, v[2] / n])
}

/// Gira `v` em torno do eixo unitário `eixo` por `graus` (Rodrigues).
pub fn gira(v: V3, eixo: V3, graus: f64) -> V3 {
    let (s, c) = graus.to_radians().sin_cos();
    let k = produto_vetorial(eixo, v);
    let d = eixo[0] * v[0] + eixo[1] * v[1] + eixo[2] * v[2];
    [0, 1, 2].map(|i| v[i] * c + k[i] * s + eixo[i] * d * (1. - c))
}

/// Direção de referência do ângulo de início: a escolhida projetada no plano perpendicular ao eixo; se
/// não há escolhida (ou ela é paralela ao eixo), a direção X do mundo projetada, e a Y se o eixo for X.
/// Determinística: o mesmo furo dá sempre o mesmo início.
pub fn referencia(dir: V3, escolhida: Option<V3>) -> V3 {
    let eixo = normaliza(dir).unwrap_or([0., 0., 1.]);
    let projeta = |v: V3| {
        let d = v[0] * eixo[0] + v[1] * eixo[1] + v[2] * eixo[2];
        normaliza([0, 1, 2].map(|i| v[i] - eixo[i] * d))
    };
    escolhida.and_then(projeta).or_else(|| projeta([1., 0., 0.])).or_else(|| projeta([0., 1., 0.])).unwrap_or([1., 0., 0.])
}

/// Ferramenta do sulco, no primeiro degrau da escada.
pub fn ferramenta(origem: V3, dir: V3, rosca: &ThreadSpec, comprimento: f64, interna: bool) -> Result<Shape, String> {
    ferramenta_no_degrau(origem, dir, rosca, comprimento, interna, 0)
}

/// Ferramenta do sulco no degrau `degrau` da [`ESCADA`], com a fase padrão (comando Rosca).
pub fn ferramenta_no_degrau(origem: V3, dir: V3, rosca: &ThreadSpec, comprimento: f64, interna: bool, degrau: usize) -> Result<Shape, String> {
    ferramenta_com_fase(origem, dir, rosca, comprimento, interna, degrau, Fase::padrao(dir))
}

/// Ferramenta do sulco: trechos de poucas voltas, um depois do outro ao longo do eixo, a partir de
/// `origem` (centro do perfil no começo da hélice). Cada trecho começa no ângulo exato em que o
/// anterior termina, então o sulco é contínuo na emenda. O ângulo de início e o sentido (a esquerda
/// espelha por um plano que contém o eixo e a referência) vêm de `fase`.
pub fn ferramenta_com_fase(origem: V3, dir: V3, rosca: &ThreadSpec, comprimento: f64, interna: bool, degrau: usize, fase: Fase) -> Result<Shape, String> {
    let (voltas, primeiro) = ESCADA[degrau.min(ESCADA.len() - 1)];
    let eixo = normaliza(dir).ok_or("direção da rosca nula")?;
    let mut feito = 0.;
    let mut ferramenta = Shape::empty();
    let mut trecho = primeiro * rosca.pitch;
    while feito < comprimento - 1e-9 {
        let l = trecho.min(comprimento - feito);
        let o = [0, 1, 2].map(|i| origem[i] + eixo[i] * feito);
        // à esquerda constrói à direita no ângulo oposto e espelha: o sulco cruza o ângulo pedido
        let base = if fase.esquerda { -fase.angulo } else { fase.angulo };
        let x = gira(fase.referencia, eixo, base + 360. * feito / rosca.pitch);
        let parte = Shape::thread_tool_dir(o, eixo, x, interna, rosca.major, rosca.pitch, l, rosca.angle).map_err(|e| e.0)?;
        // os trechos se encostam numa tampa plana em comum: soltos num conjunto, cada tampa sobrava como uma
        // membrana sem espessura dentro do sulco depois do corte (faces soltas, uma a cada emenda). Fundidos
        // num sólido só, as tampas somem. Se a fusão falhar, o conjunto solto ainda corta (com as membranas).
        ferramenta = if ferramenta.is_null() {
            parte
        } else {
            match ferramenta.boolean(&parte, Bool::Fuse) {
                Ok(f) => f,
                Err(_) => ferramenta.compound(&parte),
            }
        };
        feito += l;
        trecho = voltas * rosca.pitch;
    }
    if ferramenta.is_null() {
        return Err("rosca sem comprimento".into());
    }
    if fase.esquerda {
        let n = produto_vetorial(eixo, fase.referencia);
        ferramenta = ferramenta.transformed(Xform::Mirror(origem, n)).map_err(|e| e.0)?;
    }
    Ok(ferramenta)
}

/// Sulco do furo, com a entrada contínua: quando a rosca começa na boca (`plano.inicio` = 0), a hélice
/// começa um passo acima dela, no ar, e atravessa a boca como uma rosca feita com macho. `origem` é a
/// boca do furo e `dir` o sentido de entrada no material.
pub fn ferramenta_do_furo(origem: V3, dir: V3, rosca: &ThreadSpec, plano: &PlanoDaRosca, degrau: usize, fase: Fase) -> Result<Shape, String> {
    let eixo = normaliza(dir).ok_or("direção da rosca nula")?;
    let acima = if plano.inicio < 1e-9 { rosca.pitch } else { 0. };
    let o = [0, 1, 2].map(|i| origem[i] + eixo[i] * (plano.inicio - acima));
    ferramenta_com_fase(o, eixo, rosca, plano.comprimento + acima, true, degrau, fase)
}

/// Cone do chanfro de entrada (45°): tira da boca do furo de diâmetro `diametro` um chanfro de `c` mm,
/// medido a partir da parede. A ferramenta é um escareado com a haste mais fina que o furo, para a
/// superfície cilíndrica dela não coincidir com a parede.
pub fn chanfro_de_entrada(boca: V3, dir: V3, diametro: f64, c: f64) -> Result<Shape, String> {
    let haste = (diametro - 2.).max(diametro * 0.5);
    let fundo = (diametro + 2. * c - haste) / 2.;
    Shape::hole_tool_ext(boca, dir, haste, fundo + 0.5, HoleShape::Countersink { diameter: diametro + 2. * c, angle: 90. }, 0., 0.).map_err(|e| e.0)
}

/// Estimativa do volume que o sulco tira da parede de um furo de raio `raio_furo` ao longo de
/// `comprimento`: seção do V além da parede vezes o caminho do centroide. Serve para desconfiar de uma
/// booleana que cortou só uma fração.
pub fn volume_estimado(rosca: &ThreadSpec, raio_furo: f64, comprimento: f64) -> f64 {
    let r_maj = rosca.major / 2.;
    let r_in = r_maj - profundidade_do_filete(rosca.pitch) - 0.1 * rosca.pitch;
    let meia = meia_largura(rosca);
    let fundo = (r_maj - raio_furo).max(0.);
    let area = meia * fundo * fundo / (r_maj - r_in).max(1e-9);
    let r_centro = (raio_furo + 2. * r_maj) / 3.;
    area * 2. * std::f64::consts::PI * r_centro * comprimento / rosca.pitch
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
    plano_com_topo(rosca, cabeca, limite, material, profundidade_rosca, folga, passante, 0.)
}

/// Como [`plano`], com `topo` mm sem rosca entre a boca (ou o fim do rebaixo) e o início do sulco
/// (B373, folga em cima).
pub fn plano_com_topo(rosca: &ThreadSpec, cabeca: f64, limite: f64, material: f64, profundidade_rosca: Option<f64>, folga: f64, passante: bool, topo: f64) -> Option<PlanoDaRosca> {
    let meia = meia_largura(rosca);
    let topo = topo.max(0.);
    let inicio = if cabeca > 1e-9 || topo > 1e-9 { cabeca + topo + meia } else { 0. };
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

    #[test]
    fn folga_em_cima_empurra_o_inicio_para_baixo() {
        let sem = plano(m8(), 0., 20., 30., None, 0., false).unwrap();
        let com = plano_com_topo(m8(), 0., 20., 30., None, 0., false, 3.).unwrap();
        assert_eq!(sem.inicio, 0.);
        assert!((com.inicio - (3. + meia_largura(m8()))).abs() < 1e-9);
        // o fim não anda
        assert!(((sem.inicio + sem.comprimento) - (com.inicio + com.comprimento)).abs() < 1e-9);
    }

    #[test]
    fn referencia_e_perpendicular_ao_eixo_e_estavel() {
        for eixo in [[0., 0., 1.], [0., 0., -1.], [1., 0., 0.], [0., 1., 0.], [1., 1., 1.]] {
            let r = referencia(eixo, None);
            let e = normaliza(eixo).unwrap();
            assert!((r[0] * e[0] + r[1] * e[1] + r[2] * e[2]).abs() < 1e-9);
            assert!(((r[0] * r[0] + r[1] * r[1] + r[2] * r[2]) - 1.).abs() < 1e-9);
            assert_eq!(r, referencia(eixo, None));
        }
        // a escolhida é projetada; paralela ao eixo cai na padrão
        assert_eq!(referencia([0., 0., 1.], Some([0., 3., 5.])), [0., 1., 0.]);
        assert_eq!(referencia([0., 0., 1.], Some([0., 0., 7.])), [1., 0., 0.]);
    }

    #[test]
    fn gira_segue_a_mao_direita() {
        let v = gira([1., 0., 0.], [0., 0., 1.], 90.);
        assert!((v[0]).abs() < 1e-12 && (v[1] - 1.).abs() < 1e-12);
    }
}
