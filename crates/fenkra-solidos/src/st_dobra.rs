// SPDX-License-Identifier: LGPL-2.1-or-later
//! Dobra de tubo no nó (B326, B327, B330): contas de comprimento desenvolvido, dedução de dobra,
//! alongamento e afinamento estimados, retorno elástico e a avaliação das regras cadastradas.
//!
//! Como funciona no kernel: duas linhas guia de um mesmo comando que dividem uma ponta e têm
//! `StructJoint.mode = Dobra` viram um tubo só. O recompute recua cada perna pelo comprimento de
//! tangente (`tangente`), gira a seção em torno do centro da curva por `theta` e funde perna, arco
//! e perna num sólido. Este módulo só faz as contas (sem sólido): o recompute e a interface usam
//! o mesmo resultado, então o 3D, o painel, a ficha e a lista de corte nunca discordam.
//!
//! Fontes das fórmulas: `docs/pesquisa/dobra-de-tubos.md`. O comprimento do arco no eixo e a
//! dedução de dobra têm fonte lida; **o alongamento da fibra externa, o afinamento, o deslocamento
//! da linha neutra e a geometria dos modos em V são estimativas** (derivação própria, sem
//! confirmação em ensaio): o campo `estimativa` fica verdadeiro e a interface o diz.

use fenkra_core::{BendMode, SectionKind, TubeBend, TubeRule};
use std::collections::HashMap;
use std::f64::consts::PI;

/// Aviso que aparece junto a qualquer valor que não é de engenharia confirmada.
pub const AVISO_ESTIMATIVA: &str = "estimativa (modelo simples, sem fonte confirmada): não é valor de engenharia";

/// Aviso dos modos em V e do canto com gabarito: a geometria vem de derivação sem protótipo.
pub const AVISO_DERIVADA: &str = "geometria derivada, sem protótipo: a validar em peça real";

/// Regra com os números já avaliados. Campo `None` = a regra não diz nada.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RegraAvaliada {
    pub nome: String,
    pub fonte: String,
    pub raio_min: Option<f64>,
    pub raio_min_x: Option<f64>,
    pub neutra: Option<f64>,
    pub retorno: Option<f64>,
    pub folga: Option<f64>,
    pub alongamento_adm: Option<f64>,
    pub k_placa: Option<f64>,
    pub folga_raiz: Option<f64>,
    pub raio_min_articulacao: Option<f64>,
}

/// A regra serve para esta seção e este material?
pub fn regra_serve(r: &TubeRule, kind: SectionKind, material: &str) -> bool {
    r.section.is_none_or(|s| s == kind) && (r.material.trim().is_empty() || material.trim().is_empty() || r.material.trim().eq_ignore_ascii_case(material.trim()))
}

fn num(e: &str, vars: &HashMap<String, f64>, campo: &str) -> Result<Option<f64>, String> {
    if e.trim().is_empty() {
        return Ok(None);
    }
    crate::profile::ev(e, vars, campo).map(Some)
}

/// Avalia as expressões de uma regra cadastrada.
pub fn avaliar_regra(r: &TubeRule, vars: &HashMap<String, f64>) -> Result<RegraAvaliada, String> {
    let n = |e: &str, c: &str| num(e, vars, &format!("regra '{}': {c}", r.name));
    Ok(RegraAvaliada {
        nome: r.name.clone(),
        fonte: r.source.clone(),
        raio_min: n(&r.min_radius, "raio mínimo")?,
        raio_min_x: n(&r.min_radius_x, "raio mínimo em múltiplos")?,
        neutra: n(&r.neutral_shift, "deslocamento da neutra")?,
        retorno: n(&r.springback, "retorno elástico")?,
        folga: n(&r.clearance, "folga")?,
        alongamento_adm: n(&r.max_strain, "alongamento admissível")?,
        k_placa: n(&r.plate_k, "K da placa")?,
        folga_raiz: n(&r.root_gap, "folga de raiz")?,
        raio_min_articulacao: n(&r.min_hinge_radius, "raio mínimo da articulação")?,
    })
}

/// Valor com a sua origem ("da regra X", "informado no nó", "padrão geométrico").
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Valor {
    pub v: f64,
    pub origem: String,
}

/// Entrada do cálculo de uma dobra.
#[derive(Clone, Debug)]
pub struct EntradaDobra {
    pub tipo: SectionKind,
    pub dims: Vec<f64>,
    /// ângulo de dobra (desvio de direção entre as duas pernas), em graus
    pub theta: f64,
    pub modo: BendMode,
    pub raio: f64,
    pub regra: Option<RegraAvaliada>,
    /// valores informados no nó (sobrepõem a regra)
    pub k: Option<f64>,
    pub retorno: Option<f64>,
    pub folga_raiz: Option<f64>,
    pub rasgos: usize,
    pub gomos: usize,
    /// tubo retangular: 0 = a largura fica no plano da dobra, 1 = a altura
    pub plano: u8,
}

/// Resultado do cálculo de uma dobra.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ResultadoDobra {
    pub modo: BendMode,
    pub theta: f64,
    /// raio informado no nó (a leitura depende do modo)
    pub raio: f64,
    /// dimensão da seção no plano da dobra (altura H; diâmetro no redondo)
    pub h_plano: f64,
    /// dimensão fora do plano (largura b)
    pub b_fora: f64,
    pub parede: f64,
    /// distância do nó (cruzamento das linhas guia) ao início do arco, ao longo de cada perna;
    /// é o recuo que o recompute aplica às pernas no modo dobradeira
    pub tangente: f64,
    /// comprimento do arco na linha neutra (dobradeira) ou da placa dobrada (modos em V)
    pub arco: f64,
    /// dedução de dobra: quanto o comprimento de corte encurta em relação às duas pernas até o nó
    pub deducao: f64,
    /// o que somar à soma das pernas (nó a nó) para obter o comprimento de corte desta dobra
    pub ajuste_corte: f64,
    /// deslocamento da linha neutra para dentro da curva (mm)
    pub neutra: Valor,
    /// alongamento da fibra externa (fração); estimativa
    pub alongamento: Option<f64>,
    /// variação de espessura da parede externa e da interna (fração; negativo = afina); estimativa
    pub afinamento_ext: Option<f64>,
    pub espessamento_int: Option<f64>,
    pub retorno: Valor,
    /// ângulo que a máquina dobra: `theta + retorno`
    pub angulo_maquina: f64,
    /// raio de centro mínimo pelo alongamento admissível
    pub raio_min_alongamento: Option<f64>,
    /// aberturas e passos dos modos em V (B328): lado do V, abertura, passo
    pub abertura_v: Option<f64>,
    pub lado_v: Option<f64>,
    pub passo_v: Option<f64>,
    /// folga plana no alto do corte do canto com gabarito (B333) e comprimento plano da placa
    pub folga_plana: Option<f64>,
    pub placa: Option<f64>,
    /// comprimento de solda (mm), quando o modo solda
    pub solda: Option<f64>,
    /// corte simples: diferença entre o comprimento externo e o interno da perna, e quanto a ponta
    /// longa passa do nó
    pub dif_ext_int: Option<f64>,
    pub ponta_longa: Option<f64>,
    /// gomos do corte simples em tubo redondo: (peça do meio, peça da ponta)
    pub gomos: Option<(f64, f64)>,
    /// o modo gera geometria própria no 3D?
    pub geometria_pronta: bool,
    /// número de rasgos em V (modos de corte) e folga de raiz usada no canto com gabarito
    pub rasgos: usize,
    pub folga_raiz: f64,
    /// origem dos valores usados (campo, origem)
    pub origens: Vec<(String, String)>,
    /// avisos; vazios = tudo dentro das regras
    pub avisos: Vec<String>,
    /// bloqueios: o raio não cabe, o modo não vale para a seção…
    pub erros: Vec<String>,
    /// verdadeiro enquanto alongamento, afinamento e neutra forem estimativas
    pub estimativa: bool,
}

fn dims_plano(tipo: SectionKind, d: &[f64], plano: u8) -> (f64, f64, f64) {
    let g = |i: usize| d.get(i).copied().unwrap_or(0.);
    match tipo {
        SectionKind::TuboQuadrado => (g(0), g(0), g(1)),
        SectionKind::TuboRetangular => {
            if plano == 0 {
                (g(0), g(1), g(2))
            } else {
                (g(1), g(0), g(2))
            }
        }
        SectionKind::TuboRedondo => (g(0), g(0), g(1)),
        _ => (g(0), g(0), 0.),
    }
}

/// Posição X (contada do início do arco, ao longo da perna) em que a bissetriz do canto com
/// gabarito cruza a face interna do tubo. A folga de raiz desloca a bissetriz de e/2 para o lado;
/// sem folga, vale `(Ro − H)·tan(h)`. `hf` é metade do ângulo da dobra, em radianos.
pub fn ix_do_canto(ro: f64, h: f64, t: f64, hf: f64, folga: f64) -> f64 {
    let rf = ro - t;
    let a = if rf > 0. { (folga / (2. * rf)).clamp(0., 1.).asin() } else { 0. };
    let (ex, ey) = (rf * (hf - a).sin(), ro - rf * (hf - a).cos());
    ex - (h - ey) * hf.tan()
}

fn ramanujan(a: f64, b: f64) -> f64 {
    PI * (3. * (a + b) - ((3. * a + b) * (a + 3. * b)).sqrt())
}

/// Calcula uma dobra. Não falha: o que impede a geometria vem em `erros`.
pub fn calcular(e: &EntradaDobra) -> ResultadoDobra {
    let mut r = ResultadoDobra { modo: e.modo, theta: e.theta, raio: e.raio, estimativa: true, ..Default::default() };
    let fechado = matches!(e.tipo, SectionKind::TuboQuadrado | SectionKind::TuboRetangular | SectionKind::TuboRedondo);
    if !fechado {
        r.erros.push("a dobra de tubo vale para tubo quadrado, retangular ou redondo".into());
        return r;
    }
    let (h, b, t) = dims_plano(e.tipo, &e.dims, e.plano);
    let redondo = e.tipo == SectionKind::TuboRedondo;
    r.h_plano = h;
    r.b_fora = b;
    r.parede = t;
    if !(e.theta > 0.5 && e.theta < 179.5) {
        r.erros.push(format!("ângulo de dobra {:.1}° fora de 0,5° a 179,5°: as linhas são quase colineares ou voltam sobre si", e.theta));
        return r;
    }
    if e.raio <= 0. && e.modo != BendMode::CorteSimples {
        r.erros.push("o raio do nó precisa ser positivo".into());
        return r;
    }
    let th = e.theta.to_radians();
    let hf = th / 2.;
    let regra = e.regra.as_ref();
    let nome_regra = regra.map(|x| x.nome.clone());
    let da_regra = |campo: &str| format!("regra '{}' ({}){}", nome_regra.clone().unwrap_or_default(), campo, regra.filter(|x| !x.fonte.trim().is_empty()).map(|x| format!(", fonte: {}", x.fonte.trim())).unwrap_or_default());
    // retorno elástico: nó > regra > nenhum
    r.retorno = match (e.retorno, regra.and_then(|x| x.retorno)) {
        (Some(v), _) => Valor { v, origem: "informado no nó".into() },
        (None, Some(v)) => Valor { v, origem: da_regra("retorno elástico") },
        _ => Valor { v: 0., origem: "sem regra: retorno não compensado".into() },
    };
    r.angulo_maquina = e.theta + r.retorno.v;
    r.origens.push(("retorno elástico".into(), r.retorno.origem.clone()));
    // deslocamento da linha neutra
    r.neutra = match regra.and_then(|x| x.neutra) {
        Some(v) => Valor { v, origem: da_regra("linha neutra") },
        None => Valor { v: 0., origem: "padrão geométrico: neutra no eixo do tubo".into() },
    };
    r.origens.push(("linha neutra".into(), r.neutra.origem.clone()));
    let ep = 1e-9;

    match e.modo {
        BendMode::Dobradeira => {
            let clr = e.raio;
            if clr <= h / 2. + 0.01 {
                r.erros.push(format!("o raio de centro {clr:.2} mm não passa da metade da dimensão no plano ({:.2} mm): a curva se cruza", h / 2.));
            }
            let neutra = clr - r.neutra.v;
            r.tangente = clr * hf.tan();
            r.arco = neutra * th;
            r.deducao = 2. * r.tangente - r.arco;
            r.ajuste_corte = -r.deducao;
            if clr > h / 2. {
                r.alongamento = Some((h / 2.) / clr);
                r.afinamento_ext = Some(clr / (clr + h / 2.) - 1.);
                r.espessamento_int = Some(clr / (clr - h / 2.) - 1.);
            }
            // regras: raio mínimo e alongamento admissível
            if let Some(rr) = regra {
                if let Some(m) = rr.raio_min {
                    if clr + ep < m {
                        r.avisos.push(format!("raio de centro {clr:.2} mm abaixo do mínimo da regra ({m:.2} mm; {})", da_regra("raio mínimo")));
                    }
                }
                if let Some(x) = rr.raio_min_x {
                    if clr + ep < x * h {
                        r.avisos.push(format!("raio de centro {clr:.2} mm abaixo de {x:.2} × {h:.2} = {:.2} mm ({})", x * h, da_regra("raio mínimo em múltiplos")));
                    }
                }
                if let Some(eps) = rr.alongamento_adm.filter(|v| *v > 0.) {
                    let min = (h / 2.) / eps;
                    r.raio_min_alongamento = Some(min);
                    if clr + ep < min {
                        r.avisos.push(format!("alongamento estimado {:.0}% passa do admissível ({:.0}%): raio de centro mínimo {min:.1} mm ({})", 100. * (h / 2.) / clr, eps * 100., da_regra("alongamento admissível")));
                    }
                }
            }
            r.geometria_pronta = true;
            r.ajuste_corte = -r.deducao;
        }
        BendMode::VComum | BendMode::CorteDobradeira => {
            if redondo {
                r.erros.push("o corte em V não vale para tubo redondo: use dobradeira ou corte simples com gomos".into());
                return r;
            }
            let ri = e.raio;
            let n = e.rasgos.max(1);
            let k = match (e.k, regra.and_then(|x| x.k_placa)) {
                (Some(v), _) => Valor { v, origem: "informado no nó".into() },
                (None, Some(v)) => Valor { v, origem: da_regra("K da placa") },
                _ => Valor { v: 0.5, origem: "padrão geométrico: neutra no meio da parede, sem fonte".into() },
            };
            r.origens.push(("K da placa".into(), k.origem.clone()));
            if n == 1 {
                let ba = th * (ri + k.v * t);
                let ossb = (ri + t) * hf.tan();
                r.arco = ba;
                r.deducao = 2. * ossb - ba;
                r.tangente = (ossb - (h / 2.) * hf.tan()).max(0.);
                r.ajuste_corte = h * hf.tan() - r.deducao;
                r.abertura_v = Some(2. * (h - t) * hf.tan());
                r.lado_v = Some((h - t) / hf.cos());
                r.solda = Some(2. * (h - t) / hf.cos() + b);
                let eps = t / (2. * ri + t);
                r.alongamento = Some(eps);
                if let Some(adm) = regra.and_then(|x| x.alongamento_adm).filter(|v| *v > 0.) {
                    let min = t * (1. / adm - 1.) / 2.;
                    r.raio_min_alongamento = Some(min);
                    if ri + ep < min {
                        r.avisos.push(format!("a dobradiça estica {:.0}% (estimativa), acima do admissível {:.0}%: raio interno mínimo {min:.2} mm ({})", eps * 100., adm * 100., da_regra("alongamento admissível")));
                    }
                }
                if let Some(m) = regra.and_then(|x| x.raio_min) {
                    if ri + ep < m {
                        r.avisos.push(format!("raio da dobradiça {ri:.2} mm abaixo do mínimo da regra ({m:.2} mm)"));
                    }
                }
            } else {
                // V múltiplo: o raio é o da linha de articulações (face interna da placa)
                let phi = th / n as f64;
                let rp = e.raio;
                let passo = 2. * rp * (phi / 2.).sin();
                let w = 2. * (h - t) * (phi / 2.).tan();
                let rp_min = (h - t) / (phi / 2.).cos();
                r.passo_v = Some(passo);
                r.abertura_v = Some(w);
                r.lado_v = Some((h - t) / (phi / 2.).cos());
                if rp + ep < rp_min {
                    r.erros.push(format!("V múltiplo: o raio das articulações {rp:.2} mm é menor que o mínimo {rp_min:.2} mm (as sobras entre os V somem; canto vivo só por V comum)"));
                }
                if let Some(m) = regra.and_then(|x| x.raio_min_articulacao) {
                    if rp + ep < m {
                        r.avisos.push(format!("raio das articulações {rp:.2} mm abaixo do mínimo da regra ({m:.2} mm; {})", da_regra("raio mínimo da articulação")));
                    }
                }
                let rv = passo / (2. * (phi / 2.).sin());
                let s = rv * (((n - 1) as f64) * phi / 2.).sin() / hf.cos();
                // dedução pela face interna da placa
                r.deducao = 2. * s - (n - 1) as f64 * passo;
                r.arco = (n - 1) as f64 * passo;
                // X1i e X2i medem a face interna da placa: o nó está (h/2 - t)·tan(Θ/2) antes do canto virtual dela
                r.tangente = (s - (h / 2. - t) * hf.tan()).max(0.);
                r.ajuste_corte = (h - 2. * t) * hf.tan() - r.deducao;
                r.solda = Some(n as f64 * (2. * (h - t) / (phi / 2.).cos() + b));
            }
            if e.modo == BendMode::CorteDobradeira {
                r.origens.push(("modo".into(), "corte em V fechado por ferramenta; o retorno vem da regra".into()));
            }
            r.rasgos = n;
            r.geometria_pronta = r.erros.is_empty();
            if n > 1 {
                r.avisos.push("V múltiplo: no 3D as articulações ficam em canto vivo (sem raio de dobradiça); o raio só entra na conta".into());
            }
            r.avisos.push(AVISO_DERIVADA.into());
        }
        BendMode::VComRaio => {
            if redondo {
                r.erros.push("o canto com gabarito vale para tubo quadrado ou retangular".into());
                return r;
            }
            let ro = e.raio;
            let rf = ro - t;
            if rf <= 0. {
                r.erros.push("o raio externo precisa passar da parede".into());
                return r;
            }
            let k = match (e.k, regra.and_then(|x| x.k_placa)) {
                (Some(v), _) => Some(Valor { v, origem: "informado no nó".into() }),
                (None, Some(v)) => Some(Valor { v, origem: da_regra("K da placa") }),
                _ => None,
            };
            let rn = match &k {
                Some(k) => {
                    r.origens.push(("K da placa".into(), k.origem.clone()));
                    rf + k.v * t
                }
                None => {
                    r.origens.push(("K da placa".into(), "padrão geométrico: neutra de volume constante, sem fonte".into()));
                    (rf * ro).sqrt()
                }
            };
            let placa = rn * th;
            r.placa = Some(placa);
            r.arco = placa;
            r.deducao = 2. * ro * hf.tan() - placa;
            r.tangente = (ro * hf.tan() - (h / 2.) * hf.tan()).max(0.);
            r.ajuste_corte = h * hf.tan() - r.deducao;
            r.folga_plana = Some(placa - 2. * rf * hf.sin());
            let raiz = match (e.folga_raiz, regra.and_then(|x| x.folga_raiz)) {
                (Some(v), _) => Some((v, "informado no nó".to_string())),
                (None, Some(v)) => Some((v, da_regra("folga de raiz"))),
                _ => None,
            };
            if let Some((g, origem)) = raiz {
                r.folga_raiz = g;
                r.origens.push(("folga de raiz".into(), origem));
                let a = (g / (2. * rf)).clamp(-1., 1.).asin();
                r.folga_plana = Some(placa - 2. * rf * (hf - a).sin());
            }
            // abertura plana da parede interna: ℓ − 2·Ix, com Ix = (Ro − H)·tan(h − recuo da raiz)
            let ix = ix_do_canto(ro, h, r.parede, hf, r.folga_raiz);
            if placa - 2. * ix <= 1e-6 {
                r.erros.push(format!("raio externo {ro:.1} mm grande demais: a abertura plana da parede interna zera (limite perto de 4,6 vezes a altura a 90°)"));
            }
            if ro <= h / 2. + 0.01 {
                r.erros.push(format!("o raio externo {ro:.2} mm não passa da metade da altura ({:.2} mm): a curva se cruza", h / 2.));
            }
            // arcos de 4·rf·h nas duas laterais mais as bissetrizes
            r.solda = Some(4. * rf * hf + 2. * rf);
            // fora da faixa próxima da altura do tubo o modo não existe (4,6 H a 90°)
            if ro < h - 1e-6 || ro > 4.6 * h + 1e-6 {
                r.avisos.push(format!("raio externo {ro:.1} mm fora da faixa do canto com gabarito ({h:.1} a {:.1} mm, de 1 a 4,6 vezes a altura): o modo foi testado só aí", 4.6 * h));
            }
            if !(30. - 1e-6..=150. + 1e-6).contains(&e.theta) {
                r.avisos.push("o canto com gabarito foi derivado só de 30° a 150°".into());
            }
            r.geometria_pronta = true;
            r.avisos.push(AVISO_DERIVADA.into());
        }
        BendMode::CorteSimples => {
            r.tangente = 0.;
            r.arco = 0.;
            r.deducao = 0.;
            r.ajuste_corte = 0.;
            r.dif_ext_int = Some(h * hf.tan());
            r.ponta_longa = Some((h / 2.) * hf.tan());
            r.geometria_pronta = true;
            let m = e.gomos.max(1);
            if redondo {
                let a = (h / 2.) / hf.cos();
                r.solda = Some(ramanujan(a, h / 2.) * m as f64);
                if m > 1 {
                    let phi = th / m as f64;
                    let meio = 2. * e.raio * (phi / 2.).tan();
                    r.gomos = Some((meio, meio / 2.));
                    r.avisos.push("gomos: o 3D mostra uma esquadria só; as peças intermediárias entram na lista de corte, não no 3D".into());
                }
            } else {
                r.solda = Some(2. * h / hf.cos() + 2. * b);
            }
        }
    }
    if r.alongamento.is_some() || r.afinamento_ext.is_some() || r.neutra.v != 0. {
        r.origens.push(("alongamento e afinamento".into(), AVISO_ESTIMATIVA.into()));
    }
    r
}

/// Monta a entrada a partir do nó cadastrado. `vars` avalia as expressões; a regra é buscada por
/// nome entre as cadastradas e só vale se servir para a seção e o material.
pub fn entrada_do_no(b: &TubeBend, tipo: SectionKind, dims: &[f64], theta: f64, material: &str, regras: &[TubeRule], vars: &HashMap<String, f64>) -> Result<EntradaDobra, String> {
    let n = |e: &str, c: &str| num(e, vars, c);
    let regra = if b.rule.trim().is_empty() {
        None
    } else {
        let r = regras.iter().find(|r| r.name == b.rule).ok_or_else(|| format!("a regra de dobra '{}' não existe mais", b.rule))?;
        if !regra_serve(r, tipo, material) {
            return Err(format!("a regra '{}' não vale para esta seção ou este material", r.name));
        }
        Some(avaliar_regra(r, vars)?)
    };
    let raio = match n(&b.radius, "raio da dobra")? {
        Some(v) => v,
        None if b.mode == BendMode::CorteSimples => 0.,
        None => return Err("preencha o raio da dobra".into()),
    };
    Ok(EntradaDobra {
        tipo,
        dims: dims.to_vec(),
        theta,
        modo: b.mode,
        raio,
        regra,
        k: n(&b.k, "K")?,
        retorno: n(&b.springback, "retorno elástico")?,
        folga_raiz: n(&b.root_gap, "folga de raiz")?,
        rasgos: n(&b.cuts, "rasgos")?.map(|v| v.round().max(1.) as usize).unwrap_or(1),
        gomos: n(&b.segments, "gomos")?.map(|v| v.round().max(1.) as usize).unwrap_or(1),
        plano: b.plane_dim,
    })
}

/// Nome do modo para listas e painéis.
pub fn nome_modo(m: BendMode) -> &'static str {
    match m {
        BendMode::Dobradeira => "Dobradeira",
        BendMode::VComum => "V comum",
        BendMode::VComRaio => "V com raio",
        BendMode::CorteDobradeira => "Corte + dobradeira",
        BendMode::CorteSimples => "Corte simples",
    }
}

/// Uma dobra de um tubo dobrado, no comprimento desenvolvido.
#[derive(Clone, Debug, PartialEq)]
pub struct DobraDoTubo {
    /// nó (comando, linha anterior, linha seguinte) e ponto
    pub linhas: (usize, usize),
    pub ponto: [f64; 3],
    pub resultado: ResultadoDobra,
    /// posição, ao longo do tubo reto desenvolvido, em que a dobra começa e termina
    pub inicio: f64,
    pub fim: f64,
}

/// Um tubo dobrado: as pernas encadeadas por nós de dobra de um mesmo comando.
#[derive(Clone, Debug, PartialEq)]
pub struct TuboDobrado {
    pub feature: fenkra_core::Id,
    /// linhas do comando, na ordem do tubo
    pub linhas: Vec<usize>,
    pub designacao: String,
    pub material: String,
    /// comprimento nó a nó de cada perna
    pub pernas: Vec<f64>,
    pub dobras: Vec<DobraDoTubo>,
    /// comprimento de corte da barra reta (pernas + ajustes das dobras + prolongamentos das pontas)
    pub desenvolvido: f64,
    /// soma das pernas nó a nó
    pub soma_pernas: f64,
    pub avisos: Vec<String>,
    pub erros: Vec<String>,
    /// as pernas formam um sólido só (todos os nós são dobradeira e sem erro)
    pub fundido: bool,
}

impl TuboDobrado {
    /// O modo é um só para todo o tubo? Devolve o primeiro.
    pub fn modo(&self) -> Option<BendMode> {
        self.dobras.first().map(|d| d.resultado.modo)
    }
    /// Todas as dobras têm o corte simples (as peças são separadas)?
    pub fn so_corte_simples(&self) -> bool {
        !self.dobras.is_empty() && self.dobras.iter().all(|d| d.resultado.modo == BendMode::CorteSimples)
    }
    /// Texto de uma linha para listas: "R50,8 x 90,0°, R50,8 x 45,0°".
    pub fn resumo(&self) -> String {
        self.dobras.iter().map(|d| format!("R{:.1}×{:.1}°", d.resultado.raio, d.resultado.theta)).collect::<Vec<_>>().join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entrada(tipo: SectionKind, dims: &[f64], theta: f64, modo: BendMode, raio: f64) -> EntradaDobra {
        EntradaDobra { tipo, dims: dims.to_vec(), theta, modo, raio, regra: None, k: None, retorno: None, folga_raiz: None, rasgos: 1, gomos: 1, plano: 0 }
    }

    #[test]
    fn dobradeira_redondo_confere_com_a_pesquisa() {
        // D 25,4 x 1,5, 90 graus, CLR = 2 D = 50,8: L = 79,80; BD = 21,80; alongamento 25%
        let r = calcular(&entrada(SectionKind::TuboRedondo, &[25.4, 1.5], 90., BendMode::Dobradeira, 50.8));
        assert!(r.erros.is_empty(), "{:?}", r.erros);
        assert!((r.arco - 79.80).abs() < 0.01, "{}", r.arco);
        assert!((r.deducao - 21.80).abs() < 0.01, "{}", r.deducao);
        assert!((r.alongamento.unwrap() - 0.25).abs() < 1e-9);
        assert!((r.afinamento_ext.unwrap() + 0.20).abs() < 1e-9);
        assert!((r.espessamento_int.unwrap() - 1. / 3.).abs() < 1e-9);
        assert!(r.estimativa);
        assert!(r.origens.iter().any(|(_, o)| o.contains("estimativa")));
        // D 50, CLR 100, 90 graus: arco 157,08
        let r = calcular(&entrada(SectionKind::TuboRedondo, &[50., 3.], 90., BendMode::Dobradeira, 100.));
        assert!((r.arco - 157.08).abs() < 0.01);
        assert!((r.alongamento.unwrap() - 0.25).abs() < 1e-9);
    }

    #[test]
    fn dobradeira_quadrado_e_retorno_da_regra() {
        let mut e = entrada(SectionKind::TuboQuadrado, &[30., 1.2], 90., BendMode::Dobradeira, 60.);
        e.regra = Some(RegraAvaliada { nome: "interna".into(), fonte: "ensaio".into(), retorno: Some(3.), alongamento_adm: Some(0.15), ..Default::default() });
        let r = calcular(&e);
        assert!((r.arco - 94.25).abs() < 0.01);
        assert!((r.deducao - 25.75).abs() < 0.01);
        assert!((r.angulo_maquina - 93.).abs() < 1e-9);
        assert!(r.retorno.origem.contains("interna") && r.retorno.origem.contains("ensaio"), "{}", r.retorno.origem);
        // alongamento 25% passa do admissível 15%: raio mínimo 100
        assert!((r.raio_min_alongamento.unwrap() - 100.).abs() < 1e-9);
        assert_eq!(r.avisos.len(), 1, "{:?}", r.avisos);
        // com o raio no mínimo, o aviso some
        e.raio = 100.;
        assert!(calcular(&e).avisos.is_empty());
    }

    #[test]
    fn sem_regra_nada_vem_de_tabela() {
        let r = calcular(&entrada(SectionKind::TuboQuadrado, &[30., 1.2], 90., BendMode::Dobradeira, 60.));
        assert_eq!(r.retorno.v, 0.);
        assert!(r.retorno.origem.contains("sem regra"));
        assert!(r.avisos.is_empty());
    }

    #[test]
    fn neutra_da_regra_encurta_o_desenvolvido() {
        let mut e = entrada(SectionKind::TuboRedondo, &[50., 3.], 90., BendMode::Dobradeira, 100.);
        let base = calcular(&e).arco;
        e.regra = Some(RegraAvaliada { nome: "n".into(), neutra: Some(1.), ..Default::default() });
        let r = calcular(&e);
        // erro de desenvolvido = delta * theta (pesquisa)
        assert!((base - r.arco - 1. * PI / 2.).abs() < 1e-9);
    }

    #[test]
    fn raio_menor_que_a_meia_altura_bloqueia() {
        let r = calcular(&entrada(SectionKind::TuboQuadrado, &[30., 1.2], 90., BendMode::Dobradeira, 14.));
        assert_eq!(r.erros.len(), 1);
        let r = calcular(&entrada(SectionKind::TuboQuadrado, &[30., 1.2], 180., BendMode::Dobradeira, 60.));
        assert!(!r.erros.is_empty());
        let r = calcular(&entrada(SectionKind::PerfilU, &[100., 50., 5., 8.], 90., BendMode::Dobradeira, 60.));
        assert!(!r.erros.is_empty());
    }

    #[test]
    fn v_comum_confere_com_a_pesquisa() {
        // 30x30x1,2, 90 graus, ri = 1,8, K = 0,33: BA 3,449; OSSB 3,000; BD 2,551; w 57,6; lado 40,73; solda 111,5
        let mut e = entrada(SectionKind::TuboQuadrado, &[30., 1.2], 90., BendMode::VComum, 1.8);
        e.k = Some(0.33);
        let r = calcular(&e);
        assert!((r.arco - 3.449).abs() < 0.001, "{}", r.arco);
        assert!((r.deducao - 2.551).abs() < 0.001, "{}", r.deducao);
        assert!((r.abertura_v.unwrap() - 57.6).abs() < 0.01);
        assert!((r.lado_v.unwrap() - 40.73).abs() < 0.01);
        assert!((r.solda.unwrap() - 111.5).abs() < 0.1);
        assert!(r.geometria_pronta && r.avisos.iter().any(|a| a.contains("a validar em peça real")));
        // redondo não serve
        assert!(!calcular(&entrada(SectionKind::TuboRedondo, &[30., 1.2], 90., BendMode::VComum, 1.8)).erros.is_empty());
    }

    #[test]
    fn v_multiplo_confere_e_barra_raio_pequeno() {
        // Rp = 45, n = 6: phi 15, p 11,75, w 7,58, solda ~529
        let mut e = entrada(SectionKind::TuboQuadrado, &[30., 1.2], 90., BendMode::VComum, 45.);
        e.rasgos = 6;
        let r = calcular(&e);
        assert!(r.erros.is_empty(), "{:?}", r.erros);
        assert!((r.passo_v.unwrap() - 11.75).abs() < 0.01);
        assert!((r.abertura_v.unwrap() - 7.58).abs() < 0.01);
        assert!((r.solda.unwrap() - 529.).abs() < 2.);
        // L = X1i + X2i - 18,75 : a dedução pela face interna é 18,75
        assert!((r.deducao - 18.75).abs() < 0.05, "{}", r.deducao);
        // Rp mínimo para n = 6 é 29,05
        e.raio = 29.0;
        assert!(!calcular(&e).erros.is_empty());
        e.raio = 29.1;
        assert!(calcular(&e).erros.is_empty());
    }

    #[test]
    fn canto_com_gabarito_confere_com_a_pesquisa() {
        // 30x30x1,2 a 90: placa 46,17, BD 13,83, folga 5,44; com e = 1 a folga vai a 6,16
        let mut e = entrada(SectionKind::TuboQuadrado, &[30., 1.2], 90., BendMode::VComRaio, 30.);
        let r = calcular(&e);
        assert!((r.placa.unwrap() - 46.17).abs() < 0.01, "{:?}", r.placa);
        assert!((r.deducao - 13.83).abs() < 0.01);
        assert!((r.folga_plana.unwrap() - 5.44).abs() < 0.01);
        assert!((r.solda.unwrap() - 148.1).abs() < 0.1, "{:?}", r.solda);
        e.folga_raiz = Some(1.);
        assert!((calcular(&e).folga_plana.unwrap() - 6.16).abs() < 0.01);
        e.folga_raiz = Some(2.);
        assert!((calcular(&e).folga_plana.unwrap() - 6.88).abs() < 0.01);
        // fora da faixa: avisa
        e.raio = 200.;
        assert!(calcular(&e).avisos.iter().any(|a| a.contains("faixa")));
    }

    #[test]
    fn corte_simples_confere_a_solda_da_pesquisa() {
        // quadrado 30x30, 90: 2*30*raiz(2) + 2*30 = 144,85
        let r = calcular(&entrada(SectionKind::TuboQuadrado, &[30., 1.2], 90., BendMode::CorteSimples, 0.));
        assert!((r.solda.unwrap() - 144.85).abs() < 0.01);
        assert!((r.dif_ext_int.unwrap() - 30.).abs() < 1e-9);
        // redondo 25,4: elipse de perímetro 97,03
        let r = calcular(&entrada(SectionKind::TuboRedondo, &[25.4, 1.5], 90., BendMode::CorteSimples, 0.));
        assert!((r.solda.unwrap() - 97.03).abs() < 0.05, "{:?}", r.solda);
        // gomos m = 3, raio 50,8: peça do meio 27,22, ponta 13,61
        let mut e = entrada(SectionKind::TuboRedondo, &[25.4, 1.5], 90., BendMode::CorteSimples, 50.8);
        e.gomos = 3;
        let (meio, ponta) = calcular(&e).gomos.unwrap();
        assert!((meio - 27.22).abs() < 0.01 && (ponta - 13.61).abs() < 0.01);
    }

    #[test]
    fn retangular_usa_a_dimensao_do_plano() {
        let a = calcular(&entrada(SectionKind::TuboRetangular, &[40., 20., 2.], 90., BendMode::Dobradeira, 80.));
        let mut e = entrada(SectionKind::TuboRetangular, &[40., 20., 2.], 90., BendMode::Dobradeira, 80.);
        e.plano = 1;
        let b = calcular(&e);
        assert_eq!((a.h_plano, a.b_fora), (40., 20.));
        assert_eq!((b.h_plano, b.b_fora), (20., 40.));
        assert!(a.alongamento.unwrap() > b.alongamento.unwrap());
        // o comprimento do arco no eixo não depende do lado
        assert!((a.arco - b.arco).abs() < 1e-9);
    }

    #[test]
    fn regra_filtra_por_secao_e_material() {
        let r = TubeRule { name: "x".into(), material: "Aço".into(), section: Some(SectionKind::TuboRedondo), ..Default::default() };
        assert!(regra_serve(&r, SectionKind::TuboRedondo, "aço"));
        assert!(regra_serve(&r, SectionKind::TuboRedondo, ""));
        assert!(!regra_serve(&r, SectionKind::TuboQuadrado, "aço"));
        assert!(!regra_serve(&r, SectionKind::TuboRedondo, "Alumínio"));
    }
}
