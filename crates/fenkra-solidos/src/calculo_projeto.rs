// SPDX-License-Identifier: LGPL-2.1-or-later
//! Calculadoras de projeto (B110): parafuso, ajuste forçado, mola helicoidal e viga.
//!
//! **Cálculo preliminar.** São as fórmulas fechadas dos livros-texto de elementos de máquinas e de
//! resistência dos materiais, para dimensionar e comparar alternativas no começo do projeto. Não
//! substituem o cálculo de quem assina o projeto: não há fadiga, concentração de tensão nem efeitos
//! dinâmicos, e os materiais são valores típicos que a pessoa pode trocar. Toda tela que mostra
//! estes resultados traz o rótulo [`AVISO_PRELIMINAR`].
//!
//! Fontes (fórmulas de domínio público ou de norma aberta; nenhum texto foi copiado):
//!
//! - **Parafuso**: área resistente à tração `As = π/4 · ((d2 + d3)/2)²`, com `d2 = d − 0,6495·P` e
//!   `d3 = d − 1,2269·P` (perfil básico ISO 68-1, a mesma geometria usada na rosca modelada); tensão
//!   de prova `Sp` por classe de resistência (ISO 898-1); pré-carga como fração de `Sp·As` e torque
//!   de aperto `T = K·Fi·d` (equação do fator de porca, de uso corrente no projeto de juntas
//!   parafusadas); cisalhamento pelo critério da energia de distorção (von Mises, 1913),
//!   `τ_adm = 0,577·Sp`, com `Sp` no lugar do escoamento (conservador).
//! - **Ajuste forçado**: cilindros de parede espessa de Lamé (1833). Pressão de contato
//!   `p = δ / ( d · [ (1/Ec)·((Dc² + d²)/(Dc² − d²) + νc) + (1/Ee)·((d² + di²)/(d² − di²) − νe) ] )`
//!   para interferência diametral `δ`; capacidade de torque `T = μ·p·π·d²·L/2` e de força axial
//!   `F = μ·p·π·d·L`. A interferência mínima e a máxima vêm do ajuste ISO 286 (`fenkra-tolerancias`).
//! - **Mola helicoidal de compressão**: constante `k = G·d⁴ / (8·D³·Na)`; tensão de cisalhamento
//!   `τ = Kw · 8·F·D / (π·d³)` com o fator de Wahl `Kw = (4C − 1)/(4C − 4) + 0,615/C` (Wahl, 1935),
//!   `C = D/d`; altura sólida pelo tipo de extremidade.
//! - **Viga**: Euler-Bernoulli. Flecha e momento máximos dos quatro casos clássicos (biapoiada ou
//!   engastada, carga pontual ou distribuída), `σ = M·c / I`.

use crate::hardware::metric;
use std::f64::consts::PI;

/// Rótulo que acompanha todo resultado das calculadoras.
pub const AVISO_PRELIMINAR: &str = "Cálculo preliminar: fórmulas fechadas com valores típicos, sem fadiga nem concentração de tensão. Confirme com quem assina o projeto antes de fabricar.";

fn positivo(v: f64, nome: &str) -> Result<f64, String> {
    if v.is_finite() && v > 0. {
        Ok(v)
    } else {
        Err(format!("{nome} precisa ser um número maior que zero"))
    }
}

// ------------------------------------------------------------------------------------ parafuso

/// Classe de resistência de parafuso de aço (ISO 898-1): tensão de prova `Sp` em MPa. A 8.8 muda de
/// 580 para 600 MPa acima de M16.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClasseParafuso {
    pub nome: &'static str,
    pub sp_ate_m16: f64,
    pub sp_acima_m16: f64,
}

pub const CLASSES_PARAFUSO: &[ClasseParafuso] = &[
    ClasseParafuso { nome: "4.6", sp_ate_m16: 225., sp_acima_m16: 225. },
    ClasseParafuso { nome: "5.8", sp_ate_m16: 380., sp_acima_m16: 380. },
    ClasseParafuso { nome: "8.8", sp_ate_m16: 580., sp_acima_m16: 600. },
    ClasseParafuso { nome: "10.9", sp_ate_m16: 830., sp_acima_m16: 830. },
    ClasseParafuso { nome: "12.9", sp_ate_m16: 970., sp_acima_m16: 970. },
];

pub fn classe_parafuso(nome: &str) -> Option<&'static ClasseParafuso> {
    CLASSES_PARAFUSO.iter().find(|c| c.nome == nome)
}

/// Área resistente à tração (mm²) da rosca métrica de diâmetro `d` e passo `p`.
pub fn area_tracao(d: f64, p: f64) -> f64 {
    let (d2, d3) = (d - 0.649_519 * p, d - 1.226_869 * p);
    PI / 4. * ((d2 + d3) / 2.).powi(2)
}

/// Diâmetro nominal (mm) de "M10".
fn diametro_de(tamanho: &str) -> Option<f64> {
    tamanho.strip_prefix('M')?.parse().ok()
}

#[derive(Clone, Debug, PartialEq)]
pub struct EntradaParafuso {
    /// "M10"
    pub tamanho: String,
    /// "8.8"
    pub classe: String,
    /// carga de tração por parafuso (N)
    pub tracao: f64,
    /// carga cortante por parafuso (N); 0 = sem corte
    pub corte: f64,
    /// planos de corte que o parafuso atravessa (1 ou 2)
    pub planos: u32,
    /// fator de segurança exigido
    pub fs_exigido: f64,
    /// pré-carga como fração de `Sp·As`: 0,75 para junta desmontável, 0,9 para permanente
    pub fracao_pre_carga: f64,
    /// fator de porca K (0,2 para aço seco, sem lubrificante)
    pub fator_porca: f64,
}

impl Default for EntradaParafuso {
    fn default() -> Self {
        EntradaParafuso { tamanho: "M10".into(), classe: "8.8".into(), tracao: 10_000., corte: 0., planos: 1, fs_exigido: 2., fracao_pre_carga: 0.75, fator_porca: 0.2 }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResultadoParafuso {
    pub passo: f64,
    pub area: f64,
    pub tensao_prova: f64,
    /// carga de prova `Sp·As` (N)
    pub carga_prova: f64,
    pub tensao_tracao: f64,
    /// `None` sem carga de tração
    pub fs_tracao: Option<f64>,
    pub tensao_corte: f64,
    pub fs_corte: Option<f64>,
    pub pre_carga: f64,
    /// torque de aperto (N·m)
    pub torque: f64,
    /// menor tamanho da tabela que atende o fator de segurança exigido nas duas verificações
    pub recomendado: Option<String>,
    pub avisos: Vec<String>,
}

fn sp_do(c: &ClasseParafuso, d: f64) -> f64 {
    if d <= 16. {
        c.sp_ate_m16
    } else {
        c.sp_acima_m16
    }
}

fn verificar_parafuso(d: f64, p: f64, sp: f64, e: &EntradaParafuso) -> (f64, Option<f64>, f64, Option<f64>) {
    let area = area_tracao(d, p);
    let sigma = e.tracao / area;
    let tau = e.corte / (area * e.planos.max(1) as f64);
    let fs_t = (e.tracao > 0.).then(|| sp / sigma);
    let fs_c = (e.corte > 0.).then(|| 0.577 * sp / tau);
    (sigma, fs_t, tau, fs_c)
}

pub fn calcular_parafuso(e: &EntradaParafuso) -> Result<ResultadoParafuso, String> {
    let m = metric(&e.tamanho).ok_or_else(|| format!("tamanho {} fora da tabela métrica", e.tamanho))?;
    let d = diametro_de(m.name).ok_or("tamanho inválido")?;
    let classe = classe_parafuso(&e.classe).ok_or_else(|| format!("classe {} desconhecida", e.classe))?;
    if !e.tracao.is_finite() || e.tracao < 0. || !e.corte.is_finite() || e.corte < 0. {
        return Err("as cargas não podem ser negativas".into());
    }
    if e.tracao == 0. && e.corte == 0. {
        return Err("informe a carga de tração ou a de corte".into());
    }
    positivo(e.fs_exigido, "o fator de segurança")?;
    positivo(e.fator_porca, "o fator de porca K")?;
    if !(e.fracao_pre_carga > 0. && e.fracao_pre_carga <= 1.) {
        return Err("a fração da pré-carga fica entre 0 e 1".into());
    }
    let sp = sp_do(classe, d);
    let area = area_tracao(d, m.pitch);
    let (sigma, fs_t, tau, fs_c) = verificar_parafuso(d, m.pitch, sp, e);
    let pre = e.fracao_pre_carga * sp * area;
    let torque = e.fator_porca * pre * d / 1000.;
    let atende = |fs: Option<f64>| fs.is_none_or(|f| f >= e.fs_exigido);
    let recomendado = crate::hardware::METRIC.iter().find_map(|t| {
        let (dd, sp) = (diametro_de(t.name)?, sp_do(classe, diametro_de(t.name)?));
        let (_, ft, _, fc) = verificar_parafuso(dd, t.pitch, sp, e);
        (atende(ft) && atende(fc)).then(|| t.name.to_string())
    });
    let mut avisos = vec![];
    if !(atende(fs_t) && atende(fs_c)) {
        avisos.push(match &recomendado {
            Some(r) => format!("o fator de segurança fica abaixo do exigido ({}): o menor tamanho que atende é {r}", e.fs_exigido),
            None => format!("nenhum tamanho da tabela atende o fator de segurança {} com a classe {}", e.fs_exigido, e.classe),
        });
    }
    if pre < e.tracao {
        avisos.push("a carga de tração passa da pré-carga: a junta pode abrir; confira a rigidez da junta".into());
    }
    Ok(ResultadoParafuso { passo: m.pitch, area, tensao_prova: sp, carga_prova: sp * area, tensao_tracao: sigma, fs_tracao: fs_t, tensao_corte: tau, fs_corte: fs_c, pre_carga: pre, torque, recomendado, avisos })
}

// -------------------------------------------------------------------------------- ajuste forçado

/// Material com os valores típicos usados nas calculadoras (editáveis pela pessoa).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Material {
    pub nome: &'static str,
    /// módulo de elasticidade (MPa)
    pub e: f64,
    pub nu: f64,
    /// tensão de escoamento (MPa)
    pub sy: f64,
}

pub const MATERIAIS: &[Material] = &[
    Material { nome: "Aço carbono (típico)", e: 205_000., nu: 0.29, sy: 350. },
    Material { nome: "Aço liga temperado (típico)", e: 205_000., nu: 0.29, sy: 650. },
    Material { nome: "Aço inoxidável austenítico", e: 193_000., nu: 0.29, sy: 215. },
    Material { nome: "Alumínio liga estrutural", e: 69_000., nu: 0.33, sy: 240. },
    Material { nome: "Ferro fundido cinzento", e: 100_000., nu: 0.26, sy: 150. },
    Material { nome: "Bronze (típico)", e: 105_000., nu: 0.34, sy: 150. },
];

#[derive(Clone, Debug, PartialEq)]
pub struct EntradaAjuste {
    /// diâmetro nominal da união (mm)
    pub d: f64,
    /// diâmetro externo do cubo (mm)
    pub d_cubo: f64,
    /// diâmetro do furo do eixo, para eixo vazado (mm); 0 = maciço
    pub d_furo_eixo: f64,
    /// comprimento da união (mm)
    pub comprimento: f64,
    /// ajuste ISO 286, como "H7/p6"
    pub ajuste: String,
    /// coeficiente de atrito da união (0,12 a 0,15 para aço seco)
    pub mu: f64,
    pub cubo: Material,
    pub eixo: Material,
    /// torque que a união precisa transmitir (N·m); 0 = não verificar
    pub torque: f64,
    pub fs_exigido: f64,
}

impl Default for EntradaAjuste {
    fn default() -> Self {
        EntradaAjuste { d: 40., d_cubo: 80., d_furo_eixo: 0., comprimento: 50., ajuste: "H7/p6".into(), mu: 0.12, cubo: MATERIAIS[0], eixo: MATERIAIS[0], torque: 0., fs_exigido: 1.5 }
    }
}

/// Pressão de contato (MPa) para a interferência diametral `delta` (mm), pelas equações de Lamé.
pub fn pressao_de_contato(d: f64, d_cubo: f64, d_furo_eixo: f64, cubo: &Material, eixo: &Material, delta: f64) -> f64 {
    let (do2, d2, di2) = (d_cubo * d_cubo, d * d, d_furo_eixo * d_furo_eixo);
    let flex_cubo = ((do2 + d2) / (do2 - d2) + cubo.nu) / cubo.e;
    let flex_eixo = ((d2 + di2) / (d2 - di2) - eixo.nu) / eixo.e;
    delta / (d * (flex_cubo + flex_eixo))
}

/// Tensão equivalente de von Mises num estado plano com tensões principais `st` e `sr`.
fn von_mises(st: f64, sr: f64) -> f64 {
    (st * st - st * sr + sr * sr).sqrt()
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResultadoAjuste {
    /// interferência diametral mínima e máxima do ajuste (µm); negativa = folga
    pub interf_min_um: f64,
    pub interf_max_um: f64,
    pub p_min: f64,
    pub p_max: f64,
    /// capacidade com a pressão mínima
    pub forca_axial_min: f64,
    pub torque_min: f64,
    /// tensão de von Mises no furo do cubo e na superfície do eixo, com a pressão máxima
    pub tensao_cubo: f64,
    pub tensao_eixo: f64,
    pub fs_cubo: f64,
    pub fs_eixo: f64,
    /// capacidade de torque dividida pelo torque exigido (com a pressão mínima)
    pub fs_torque: Option<f64>,
    pub avisos: Vec<String>,
}

pub fn calcular_ajuste(e: &EntradaAjuste) -> Result<ResultadoAjuste, String> {
    positivo(e.d, "o diâmetro da união")?;
    positivo(e.comprimento, "o comprimento da união")?;
    positivo(e.mu, "o coeficiente de atrito")?;
    positivo(e.fs_exigido, "o fator de segurança")?;
    if !(e.d_cubo.is_finite() && e.d_cubo > e.d) {
        return Err("o diâmetro externo do cubo precisa ser maior que o da união".into());
    }
    if !(e.d_furo_eixo.is_finite() && e.d_furo_eixo >= 0. && e.d_furo_eixo < e.d) {
        return Err("o furo do eixo precisa ser menor que o diâmetro da união (0 = eixo maciço)".into());
    }
    if !e.torque.is_finite() || e.torque < 0. {
        return Err("o torque não pode ser negativo".into());
    }
    let ajuste = fenkra_tolerancias::ajustes::Ajuste::parse(&e.ajuste).ok_or_else(|| format!("ajuste \"{}\" não reconhecido: use o formato furo/eixo, como H7/p6", e.ajuste))?;
    let a = ajuste.analisar(e.d).ok_or_else(|| format!("o ajuste {} não tem tabela para Ø{}", e.ajuste, e.d))?;
    let (i_min, i_max) = (-a.folga_max_um, -a.folga_min_um);
    if i_max <= 0. {
        return Err(format!("o ajuste {} tem sempre folga em Ø{}: não há pressão de contato, escolha um ajuste com interferência", e.ajuste, e.d));
    }
    let p_de = |um: f64| if um <= 0. { 0. } else { pressao_de_contato(e.d, e.d_cubo, e.d_furo_eixo, &e.cubo, &e.eixo, um / 1000.) };
    let (p_min, p_max) = (p_de(i_min), p_de(i_max));
    let (do2, d2, di2) = (e.d_cubo.powi(2), e.d.powi(2), e.d_furo_eixo.powi(2));
    let st_cubo = p_max * (do2 + d2) / (do2 - d2);
    let st_eixo = -p_max * (d2 + di2) / (d2 - di2);
    let (tensao_cubo, tensao_eixo) = (von_mises(st_cubo, -p_max), von_mises(st_eixo, -p_max));
    let forca_axial_min = e.mu * p_min * PI * e.d * e.comprimento;
    let torque_min = e.mu * p_min * PI * e.d * e.d * e.comprimento / 2. / 1000.;
    let fs_torque = (e.torque > 0.).then(|| torque_min / e.torque);
    let mut avisos = vec![];
    if i_min <= 0. {
        avisos.push("o ajuste é incerto: na menor interferência pode haver folga, e a união não transmitiria carga".into());
    }
    let (fs_cubo, fs_eixo) = (e.cubo.sy / tensao_cubo, e.eixo.sy / tensao_eixo.max(1e-9));
    if fs_cubo < e.fs_exigido {
        avisos.push(format!("o cubo escoa ou fica abaixo do fator {} com a interferência máxima (FS {:.2})", e.fs_exigido, fs_cubo));
    }
    if fs_eixo < e.fs_exigido {
        avisos.push(format!("o eixo escoa ou fica abaixo do fator {} com a interferência máxima (FS {:.2})", e.fs_exigido, fs_eixo));
    }
    if fs_torque.is_some_and(|f| f < e.fs_exigido) {
        avisos.push("a capacidade de torque com a interferência mínima fica abaixo do fator exigido".into());
    }
    if e.d_cubo / e.d < 1.3 {
        avisos.push("parede do cubo fina (diâmetro externo menor que 1,3 do diâmetro da união): a conta de parede espessa é otimista".into());
    }
    Ok(ResultadoAjuste { interf_min_um: i_min, interf_max_um: i_max, p_min, p_max, forca_axial_min, torque_min, tensao_cubo, tensao_eixo, fs_cubo, fs_eixo, fs_torque, avisos })
}

// ------------------------------------------------------------------------------------------ mola

/// Extremidades da mola de compressão: decidem as espiras totais e a altura sólida.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Extremidade {
    /// espiras simples: Nt = Na, altura sólida d·(Nt + 1)
    Simples,
    /// esquadradas: Nt = Na + 2, altura sólida d·(Nt + 1)
    Esquadrada,
    /// esquadradas e retificadas: Nt = Na + 2, altura sólida d·Nt
    EsquadradaRetificada,
}

impl Extremidade {
    pub const TODAS: [Extremidade; 3] = [Extremidade::Simples, Extremidade::Esquadrada, Extremidade::EsquadradaRetificada];
    pub fn rotulo(self) -> &'static str {
        match self {
            Extremidade::Simples => "Simples",
            Extremidade::Esquadrada => "Esquadrada",
            Extremidade::EsquadradaRetificada => "Esquadrada e retificada",
        }
    }
    fn espiras_totais(self, na: f64) -> f64 {
        match self {
            Extremidade::Simples => na,
            _ => na + 2.,
        }
    }
    fn altura_solida(self, d: f64, na: f64) -> f64 {
        let nt = self.espiras_totais(na);
        match self {
            Extremidade::EsquadradaRetificada => d * nt,
            _ => d * (nt + 1.),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct EntradaMola {
    /// diâmetro do fio (mm)
    pub d_fio: f64,
    /// diâmetro médio da espira (mm)
    pub d_medio: f64,
    pub espiras_ativas: f64,
    /// módulo de cisalhamento (MPa); 79300 para aço, 69000 para aço inox
    pub g: f64,
    pub extremidade: Extremidade,
    pub comprimento_livre: f64,
    /// força de trabalho (N)
    pub forca: f64,
    /// tensão de cisalhamento admissível do fio (MPa)
    pub tau_adm: f64,
    pub fs_exigido: f64,
}

impl Default for EntradaMola {
    fn default() -> Self {
        EntradaMola { d_fio: 2., d_medio: 16., espiras_ativas: 8., g: 79_300., extremidade: Extremidade::EsquadradaRetificada, comprimento_livre: 60., forca: 50., tau_adm: 600., fs_exigido: 1.2 }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResultadoMola {
    /// índice da mola C = D/d
    pub indice: f64,
    /// constante da mola (N/mm)
    pub constante: f64,
    pub fator_wahl: f64,
    pub flecha_trabalho: f64,
    pub tensao_trabalho: f64,
    pub fs_trabalho: f64,
    pub altura_solida: f64,
    pub forca_solida: f64,
    pub tensao_solida: f64,
    pub fs_solida: f64,
    pub avisos: Vec<String>,
}

pub fn calcular_mola(e: &EntradaMola) -> Result<ResultadoMola, String> {
    for (v, n) in [(e.d_fio, "o diâmetro do fio"), (e.d_medio, "o diâmetro médio"), (e.espiras_ativas, "o número de espiras ativas"), (e.g, "o módulo G"), (e.comprimento_livre, "o comprimento livre"), (e.tau_adm, "a tensão admissível"), (e.fs_exigido, "o fator de segurança")] {
        positivo(v, n)?;
    }
    if !e.forca.is_finite() || e.forca < 0. {
        return Err("a força de trabalho não pode ser negativa".into());
    }
    if e.d_medio <= e.d_fio {
        return Err("o diâmetro médio precisa ser maior que o diâmetro do fio".into());
    }
    let c = e.d_medio / e.d_fio;
    let k = e.g * e.d_fio.powi(4) / (8. * e.d_medio.powi(3) * e.espiras_ativas);
    let kw = (4. * c - 1.) / (4. * c - 4.) + 0.615 / c;
    let tensao = |f: f64| kw * 8. * f * e.d_medio / (PI * e.d_fio.powi(3));
    let ls = e.extremidade.altura_solida(e.d_fio, e.espiras_ativas);
    let flecha_solida = (e.comprimento_livre - ls).max(0.);
    let f_solida = k * flecha_solida;
    let (t_trab, t_sol) = (tensao(e.forca), tensao(f_solida));
    let mut avisos = vec![];
    if e.comprimento_livre <= ls {
        avisos.push(format!("o comprimento livre ({}) não passa da altura sólida ({:.2}): a mola não tem curso", e.comprimento_livre, ls));
    }
    let flecha = e.forca / k;
    if flecha > flecha_solida + 1e-9 {
        avisos.push(format!("a força de trabalho comprime {flecha:.2} mm, mais que o curso até o sólido ({flecha_solida:.2} mm)"));
    }
    if !(4. ..=12.).contains(&c) {
        avisos.push(format!("índice de mola C = {c:.1} fora da faixa usual de 4 a 12: difícil de enrolar ou de baixo aproveitamento"));
    }
    if e.comprimento_livre / e.d_medio > 4. {
        avisos.push("comprimento livre maior que 4 vezes o diâmetro médio: verifique a flambagem (guia ou haste)".into());
    }
    let fs_trabalho = e.tau_adm / t_trab.max(1e-9);
    if fs_trabalho < e.fs_exigido {
        avisos.push(format!("a tensão de trabalho dá fator {fs_trabalho:.2}, abaixo do exigido ({})", e.fs_exigido));
    }
    Ok(ResultadoMola { indice: c, constante: k, fator_wahl: kw, flecha_trabalho: flecha, tensao_trabalho: t_trab, fs_trabalho, altura_solida: ls, forca_solida: f_solida, tensao_solida: t_sol, fs_solida: e.tau_adm / t_sol.max(1e-9), avisos })
}

// ------------------------------------------------------------------------------------------ viga

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Secao {
    Retangular { b: f64, h: f64 },
    Circular { d: f64 },
    Tubo { d_ext: f64, d_int: f64 },
    /// momento de inércia (mm⁴) e distância da fibra mais afastada (mm), para perfis prontos
    Generica { i: f64, c: f64 },
}

impl Secao {
    /// (momento de inércia em mm⁴, distância da fibra extrema em mm)
    pub fn propriedades(&self) -> Result<(f64, f64), String> {
        match *self {
            Secao::Retangular { b, h } => Ok((positivo(b, "a base")? * positivo(h, "a altura")?.powi(3) / 12., h / 2.)),
            Secao::Circular { d } => Ok((PI * positivo(d, "o diâmetro")?.powi(4) / 64., d / 2.)),
            Secao::Tubo { d_ext, d_int } => {
                positivo(d_ext, "o diâmetro externo")?;
                if !(d_int.is_finite() && d_int >= 0. && d_int < d_ext) {
                    return Err("o diâmetro interno do tubo precisa ser menor que o externo".into());
                }
                Ok((PI * (d_ext.powi(4) - d_int.powi(4)) / 64., d_ext / 2.))
            }
            Secao::Generica { i, c } => Ok((positivo(i, "o momento de inércia")?, positivo(c, "a distância da fibra extrema")?)),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Apoio {
    Biapoiada,
    Engastada,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Carga {
    /// força no meio do vão (biapoiada) ou na ponta (engastada)
    Pontual,
    /// carga uniformemente distribuída em todo o vão
    Distribuida,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EntradaViga {
    pub secao: Secao,
    pub apoio: Apoio,
    pub carga: Carga,
    /// N (pontual) ou N/mm (distribuída)
    pub valor: f64,
    pub vao: f64,
    pub e: f64,
    pub sy: f64,
    pub fs_exigido: f64,
}

impl Default for EntradaViga {
    fn default() -> Self {
        EntradaViga { secao: Secao::Retangular { b: 20., h: 40. }, apoio: Apoio::Biapoiada, carga: Carga::Pontual, valor: 1000., vao: 1000., e: 210_000., sy: 250., fs_exigido: 1.5 }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResultadoViga {
    pub inercia: f64,
    /// módulo resistente I/c (mm³)
    pub modulo_resistente: f64,
    /// momento fletor máximo (N·mm)
    pub momento_max: f64,
    pub cortante_max: f64,
    /// flecha máxima (mm)
    pub flecha_max: f64,
    pub tensao_max: f64,
    pub fs: f64,
    pub avisos: Vec<String>,
}

pub fn calcular_viga(e: &EntradaViga) -> Result<ResultadoViga, String> {
    let (i, c) = e.secao.propriedades()?;
    positivo(e.valor, "a carga")?;
    positivo(e.vao, "o vão")?;
    positivo(e.e, "o módulo de elasticidade")?;
    positivo(e.sy, "a tensão de escoamento")?;
    positivo(e.fs_exigido, "o fator de segurança")?;
    let (p, l, ei) = (e.valor, e.vao, e.e * i);
    // carga total W e os quatro casos clássicos
    let (m, v, f) = match (e.apoio, e.carga) {
        (Apoio::Biapoiada, Carga::Pontual) => (p * l / 4., p / 2., p * l.powi(3) / (48. * ei)),
        (Apoio::Biapoiada, Carga::Distribuida) => (p * l * l / 8., p * l / 2., 5. * p * l.powi(4) / (384. * ei)),
        (Apoio::Engastada, Carga::Pontual) => (p * l, p, p * l.powi(3) / (3. * ei)),
        (Apoio::Engastada, Carga::Distribuida) => (p * l * l / 2., p * l, p * l.powi(4) / (8. * ei)),
    };
    let sigma = m * c / i;
    let fs = e.sy / sigma;
    let mut avisos = vec![];
    if fs < e.fs_exigido {
        avisos.push(format!("a tensão de flexão dá fator {fs:.2}, abaixo do exigido ({})", e.fs_exigido));
    }
    if f > l / 250. {
        avisos.push(format!("a flecha ({f:.2} mm) passa de L/250 ({:.2} mm), limite usual de serviço", l / 250.));
    }
    if l / (2. * c) < 5. {
        avisos.push("vão curto em relação à altura da seção: a teoria de viga esbelta perde precisão (cisalhamento)".into());
    }
    Ok(ResultadoViga { inercia: i, modulo_resistente: i / c, momento_max: m, cortante_max: v, flecha_max: f, tensao_max: sigma, fs, avisos })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn perto(a: f64, b: f64, rel: f64) -> bool {
        (a - b).abs() <= rel * b.abs().max(1e-12)
    }

    // ---- parafuso

    #[test]
    fn area_de_tracao_confere_com_a_tabela_da_norma() {
        // valores de As publicados na ISO 898-1 / ISO 724: M6 20,1; M8 36,6; M10 58,0; M12 84,3; M16 157
        for (d, p, esperado) in [(6., 1., 20.1), (8., 1.25, 36.6), (10., 1.5, 58.0), (12., 1.75, 84.3), (16., 2., 157.)] {
            let a = area_tracao(d, p);
            assert!((a - esperado).abs() < 0.4, "M{d}: {a} contra {esperado}");
        }
    }

    #[test]
    fn m10_classe_88_exemplo_conferido_a_mao() {
        // As = 57,99 mm²; Sp = 580 MPa; carga de prova 33,6 kN; pré-carga 75 %: 25,2 kN
        // torque T = 0,2 · 25 227 N · 10 mm = 50,5 N·m; tração 10 kN: σ = 172,4 MPa, FS = 580/172,4 = 3,36
        let r = calcular_parafuso(&EntradaParafuso::default()).unwrap();
        assert!(perto(r.area, 57.99, 2e-3), "{}", r.area);
        assert!(perto(r.carga_prova, 580. * r.area, 1e-12) && (r.carga_prova - 33_634.).abs() < 80.);
        assert!(perto(r.pre_carga, 0.75 * r.carga_prova, 1e-12) && (r.pre_carga - 25_226.).abs() < 60., "{}", r.pre_carga);
        assert!((r.torque - 50.45).abs() < 0.2, "{}", r.torque);
        assert!((r.tensao_tracao - 172.4).abs() < 0.5 && (r.fs_tracao.unwrap() - 3.364).abs() < 0.02, "{r:?}");
        assert!(r.fs_corte.is_none() && r.avisos.is_empty(), "{:?}", r.avisos);
        assert_eq!(r.recomendado.as_deref(), Some("M8"), "o menor tamanho que dá FS 2 com 10 kN");
    }

    #[test]
    fn recomenda_o_menor_tamanho_que_atende_e_avisa_quando_o_escolhido_nao_serve() {
        // 20 kN, FS 2: precisa As ≥ 2 · 20 000 / 580 = 69 mm²; M10 (58) não serve, M12 (84,3) serve
        let e = EntradaParafuso { tracao: 20_000., ..Default::default() };
        let r = calcular_parafuso(&e).unwrap();
        assert_eq!(r.recomendado.as_deref(), Some("M12"));
        assert!(r.avisos.iter().any(|a| a.contains("M12")), "{:?}", r.avisos);
        // cisalhamento em dois planos: τ = 8 kN / (2 · 57,99) = 69 MPa; FS = 0,577 · 580 / 69 = 4,85
        let c = calcular_parafuso(&EntradaParafuso { tracao: 0., corte: 8_000., planos: 2, ..Default::default() }).unwrap();
        assert!((c.fs_corte.unwrap() - 4.85).abs() < 0.03, "{:?}", c.fs_corte);
        assert!(c.fs_tracao.is_none());
        // classe maior atende com tamanho menor
        let alta = calcular_parafuso(&EntradaParafuso { tracao: 20_000., classe: "12.9".into(), ..Default::default() }).unwrap();
        assert_eq!(alta.recomendado.as_deref(), Some("M10"));
        // entradas inválidas
        assert!(calcular_parafuso(&EntradaParafuso { tamanho: "M7".into(), ..Default::default() }).is_err());
        assert!(calcular_parafuso(&EntradaParafuso { classe: "99.9".into(), ..Default::default() }).is_err());
        assert!(calcular_parafuso(&EntradaParafuso { tracao: 0., corte: 0., ..Default::default() }).is_err());
        assert!(calcular_parafuso(&EntradaParafuso { tracao: f64::NAN, ..Default::default() }).is_err());
    }

    // ---- ajuste forçado

    #[test]
    fn pressao_de_lame_confere_com_a_forma_fechada_do_mesmo_material() {
        // mesmo material, eixo maciço: p = E · δ · (Dc² − d²) / (2 · d · Dc²); E = 205 GPa, δ = 0,04 mm
        let aco = MATERIAIS[0];
        let p = pressao_de_contato(40., 80., 0., &aco, &aco, 0.04);
        let esperado = 205_000. * 0.04 * (6400. - 1600.) / (2. * 40. * 6400.);
        assert!(perto(p, esperado, 1e-9), "{p} contra {esperado}");
        assert!((p - 76.875).abs() < 1e-6);
        // o coeficiente de Poisson do eixo e do cubo se cancela só no mesmo material: com cubo de
        // alumínio a pressão cai
        let al = MATERIAIS[3];
        assert!(pressao_de_contato(40., 80., 0., &al, &aco, 0.04) < p / 2.);
    }

    #[test]
    fn ajuste_h7_p6_usa_a_interferencia_da_iso_286() {
        // Ø40 H7/p6: H7 = +25/0 µm, p6 = +42/+26 µm → interferência de 1 a 42 µm
        let r = calcular_ajuste(&EntradaAjuste::default()).unwrap();
        assert_eq!((r.interf_min_um, r.interf_max_um), (1., 42.));
        let aco = MATERIAIS[0];
        assert!(perto(r.p_max, pressao_de_contato(40., 80., 0., &aco, &aco, 0.042), 1e-12));
        assert!((r.p_max - 80.719).abs() < 0.01, "{}", r.p_max);
        // tensão no furo do cubo: σt = p · (6400 + 1600) / (6400 − 1600) = 1,667 p; σr = −p
        let st = r.p_max * 8000. / 4800.;
        assert!(perto(r.tensao_cubo, (st * st + st * r.p_max + r.p_max * r.p_max).sqrt(), 1e-12));
        // eixo maciço: estado hidrostático, von Mises igual à pressão
        assert!(perto(r.tensao_eixo, r.p_max, 1e-12));
        // capacidade com a pressão mínima (1 µm): p = 1,92 MPa; T = 0,12 · p · π · 40² · 50 / 2 / 1000
        let p_min = 205_000. * 0.001 * 4800. / (2. * 40. * 6400.);
        assert!(perto(r.p_min, p_min, 1e-9));
        assert!(perto(r.torque_min, 0.12 * p_min * PI * 1600. * 50. / 2. / 1000., 1e-9), "{}", r.torque_min);
        assert!(perto(r.forca_axial_min, 0.12 * p_min * PI * 40. * 50., 1e-9));
    }

    #[test]
    fn ajuste_com_folga_ou_incerto_ou_desconhecido() {
        // H7/g6 sempre tem folga: erro claro
        let e = calcular_ajuste(&EntradaAjuste { ajuste: "H7/g6".into(), ..Default::default() }).unwrap_err();
        assert!(e.contains("folga"), "{e}");
        // H7/k6 é incerto: calcula e avisa
        let r = calcular_ajuste(&EntradaAjuste { ajuste: "H7/k6".into(), ..Default::default() }).unwrap();
        assert!(r.interf_min_um < 0. && r.interf_max_um > 0. && r.p_min == 0. && r.torque_min == 0.);
        assert!(r.avisos.iter().any(|a| a.contains("incerto")), "{:?}", r.avisos);
        // texto que não é ajuste, cubo mais fino que o eixo, torque exigido sem capacidade
        assert!(calcular_ajuste(&EntradaAjuste { ajuste: "xyz".into(), ..Default::default() }).is_err());
        assert!(calcular_ajuste(&EntradaAjuste { d_cubo: 30., ..Default::default() }).is_err());
        let t = calcular_ajuste(&EntradaAjuste { torque: 500., ..Default::default() }).unwrap();
        assert!(t.fs_torque.unwrap() < 1. && t.avisos.iter().any(|a| a.contains("torque")), "{:?}", t.avisos);
        // cubo de ferro fundido de parede fina com interferência alta fica sem folga de segurança
        let al = calcular_ajuste(&EntradaAjuste { ajuste: "H7/s6".into(), cubo: MATERIAIS[4], d_cubo: 50., ..Default::default() }).unwrap();
        assert!(al.fs_cubo < 1.5, "{}", al.fs_cubo);
    }

    // ---- mola

    #[test]
    fn mola_exemplo_conferido_a_mao() {
        // d = 2, D = 16 (C = 8), Na = 8, G = 79 300 MPa: k = 79300 · 16 / (8 · 4096 · 8) = 4,840 N/mm
        // Kw = 31/28 + 0,615/8 = 1,18402; F = 50 N: τ = 1,18402 · 8 · 50 · 16 / (π · 8) = 301,5 MPa
        let r = calcular_mola(&EntradaMola::default()).unwrap();
        assert!(perto(r.indice, 8., 1e-12) && perto(r.constante, 4.8401, 1e-4), "{r:?}");
        assert!(perto(r.fator_wahl, 1.184018, 1e-6));
        assert!((r.tensao_trabalho - 301.5).abs() < 0.1, "{}", r.tensao_trabalho);
        assert!(perto(r.flecha_trabalho, 50. / r.constante, 1e-12));
        // esquadrada e retificada: Nt = 10, altura sólida 20 mm; curso 40 mm; F sólida = 4,84 · 40 = 193,6 N
        assert!(perto(r.altura_solida, 20., 1e-12) && perto(r.forca_solida, 193.6, 1e-3), "{r:?}");
        assert!(perto(r.tensao_solida, 301.5 * 193.6 / 50., 2e-3));
        assert!(perto(r.fs_trabalho, 600. / r.tensao_trabalho, 1e-12));
        // a 193,6 N a tensão passa de 600 MPa: FS no sólido abaixo de 1
        assert!(r.fs_solida < 1. && r.fs_trabalho > 1.9);
        assert!(r.avisos.iter().all(|a| !a.contains("curso")), "{:?}", r.avisos);
    }

    #[test]
    fn mola_extremidades_avisos_e_entradas_invalidas() {
        let base = EntradaMola::default();
        let simples = calcular_mola(&EntradaMola { extremidade: Extremidade::Simples, ..base.clone() }).unwrap();
        assert!(perto(simples.altura_solida, 2. * 9., 1e-12), "Nt = 8: d·(Nt + 1)");
        let esq = calcular_mola(&EntradaMola { extremidade: Extremidade::Esquadrada, ..base.clone() }).unwrap();
        assert!(perto(esq.altura_solida, 2. * 11., 1e-12));
        // comprimento livre menor que a altura sólida
        let curta = calcular_mola(&EntradaMola { comprimento_livre: 15., ..base.clone() }).unwrap();
        assert!(curta.avisos.iter().any(|a| a.contains("não tem curso")), "{:?}", curta.avisos);
        // força que passa do curso, índice fora da faixa, esbeltez
        let forte = calcular_mola(&EntradaMola { forca: 400., ..base.clone() }).unwrap();
        assert!(forte.avisos.iter().any(|a| a.contains("mais que o curso")), "{:?}", forte.avisos);
        let c = calcular_mola(&EntradaMola { d_fio: 1., d_medio: 20., ..base.clone() }).unwrap();
        assert!(c.avisos.iter().any(|a| a.contains("índice")), "{:?}", c.avisos);
        let alta = calcular_mola(&EntradaMola { comprimento_livre: 90., ..base.clone() }).unwrap();
        assert!(alta.avisos.iter().any(|a| a.contains("flambagem")));
        assert!(calcular_mola(&EntradaMola { d_medio: 2., ..base.clone() }).is_err());
        assert!(calcular_mola(&EntradaMola { espiras_ativas: 0., ..base.clone() }).is_err());
        assert!(calcular_mola(&EntradaMola { forca: -1., ..base }).is_err());
    }

    // ---- viga

    #[test]
    fn viga_biapoiada_com_carga_no_meio_exemplo_conferido_a_mao() {
        // 20 × 40 mm, vão 1000 mm, P = 1000 N, E = 210 GPa
        // I = 20 · 40³ / 12 = 106 666,7 mm⁴; M = P·L/4 = 250 000 N·mm; σ = M · 20 / I = 46,875 MPa
        // flecha = P·L³ / (48·E·I) = 1e12 / (48 · 210 000 · 106 666,7) = 0,930 mm
        let r = calcular_viga(&EntradaViga::default()).unwrap();
        assert!(perto(r.inercia, 106_666.67, 1e-6) && perto(r.modulo_resistente, 5333.33, 1e-6));
        assert!(perto(r.momento_max, 250_000., 1e-12) && perto(r.cortante_max, 500., 1e-12));
        assert!(perto(r.tensao_max, 46.875, 1e-6), "{}", r.tensao_max);
        assert!(perto(r.flecha_max, 0.9301, 1e-3), "{}", r.flecha_max);
        assert!(perto(r.fs, 250. / 46.875, 1e-9));
        assert!(r.avisos.is_empty(), "{:?}", r.avisos);
    }

    #[test]
    fn viga_os_quatro_casos_classicos() {
        let base = EntradaViga::default();
        let (ei, l): (f64, f64) = (210_000. * 106_666.666_667, 1000.);
        let caso = |apoio, carga, valor| calcular_viga(&EntradaViga { apoio, carga, valor, ..base.clone() }).unwrap();
        // biapoiada, distribuída w = 1 N/mm: M = wL²/8, δ = 5wL⁴/(384EI)
        let r = caso(Apoio::Biapoiada, Carga::Distribuida, 1.);
        assert!(perto(r.momento_max, 125_000., 1e-12) && perto(r.flecha_max, 5. * l.powi(4) / (384. * ei), 1e-9));
        // engastada, ponta P = 100 N: M = PL, δ = PL³/(3EI)
        let r = caso(Apoio::Engastada, Carga::Pontual, 100.);
        assert!(perto(r.momento_max, 100_000., 1e-12) && perto(r.flecha_max, 100. * l.powi(3) / (3. * ei), 1e-9) && perto(r.cortante_max, 100., 1e-12));
        // engastada, distribuída w = 0,1 N/mm: M = wL²/2, δ = wL⁴/(8EI)
        let r = caso(Apoio::Engastada, Carga::Distribuida, 0.1);
        assert!(perto(r.momento_max, 50_000., 1e-12) && perto(r.flecha_max, 0.1 * l.powi(4) / (8. * ei), 1e-9));
    }

    #[test]
    fn viga_secoes_e_avisos() {
        assert!(perto(Secao::Circular { d: 20. }.propriedades().unwrap().0, PI * 160_000. / 64., 1e-12));
        let (i, c) = Secao::Tubo { d_ext: 30., d_int: 26. }.propriedades().unwrap();
        assert!(perto(i, PI * (810_000. - 456_976.) / 64., 1e-12) && c == 15.);
        assert!(Secao::Tubo { d_ext: 30., d_int: 30. }.propriedades().is_err());
        assert!(Secao::Retangular { b: 0., h: 10. }.propriedades().is_err());
        // carga alta: o fator cai e a flecha passa de L/250
        let r = calcular_viga(&EntradaViga { valor: 6000., ..Default::default() }).unwrap();
        assert!(r.fs < 1.5 && r.avisos.iter().any(|a| a.contains("fator")) && r.avisos.iter().any(|a| a.contains("L/250")), "{:?}", r.avisos);
        assert!(calcular_viga(&EntradaViga { valor: 0., ..Default::default() }).is_err());
        assert!(calcular_viga(&EntradaViga { vao: f64::NAN, ..Default::default() }).is_err());
    }

    #[test]
    fn aviso_de_calculo_preliminar_existe() {
        assert!(AVISO_PRELIMINAR.starts_with("Cálculo preliminar"));
    }
}
