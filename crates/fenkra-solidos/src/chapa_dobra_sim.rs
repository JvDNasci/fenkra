// SPDX-License-Identifier: LGPL-2.1-or-later
//! Simulação da sequência de dobra da chapa (B173): ferramental do usuário, colisão da aba com o
//! punção, a matriz e a máquina em cada dobra, aba mínima, sugestão de ordem por busca limitada e
//! retorno elástico por fórmula analítica calibrável por material.
//!
//! Nenhum valor de ferramenta ou de material vem de tabela embutida: punções, matrizes, máquina e
//! materiais são estilos nomeados do usuário (`fenkra_core::regras::EstiloRegras`, o mesmo
//! mecanismo dos estilos de chapa), com fórmulas que podem citar `espessura`.
//! Chaves: punção `angulo`, `raio_ponta`, `altura`, `largura_corpo`; matriz `abertura_v`,
//! `angulo_v`, `altura`, `ombro`; máquina `altura_livre`; material `escoamento` (MPa), `modulo`
//! (MPa), `calibracao` (opcional, 1 por padrão).
//!
//! Como funciona. A partir da planificada, cada dobra é uma reta no blank. Ao dobrar a reta `i`, a
//! seção perpendicular a ela é um desenho 2D no quadro da matriz: a ponta do punção na origem, a
//! matriz em V simétrica, e as duas abas giram `φ/2` cada (φ é o ângulo de dobra com sobredobra).
//! As dobras paralelas já feitas viram pernas (segmentos) nessa seção, a distância `d` da reta com
//! altura `H` medida no desenvolvido; elas giram junto com a aba e são testadas contra:
//!
//! - o punção (cunha de ângulo `angulo` até `altura`, afunilada até `largura_corpo`);
//! - a matriz (bloco com o V entalhado) e o plano da mesa, quando a perna aponta para baixo;
//! - a altura livre da máquina, medida acima do plano dos ombros da matriz.
//!
//! O ângulo do punção e o da matriz precisam caber no ângulo interno final (`180 - φ`), e a aba
//! plana precisa chegar aos ombros (`abertura_v / 2`). A posição testada é a final de cada dobra
//! (a de maior giro), que é a pior para a cunha. Dobras já feitas que não são paralelas à atual
//! não entram na colisão: o relatório as lista como "não verificadas" em vez de calar.
//!
//! O retorno elástico usa a relação de raios de dobra pura em material elastoplástico ideal:
//! `Ri/Rf = 4 K^3 - 3 K + 1`, com `K = Ri*Y/(E*t)`, e a conservação do arco da linha neutra,
//! `Ai*(Ri + t/2) = Af*(Rf + t/2)`. O fator `calibracao` escala a diferença `Ai - Af` para
//! ajustar o resultado a dobras de teste na máquina real.

use crate::nesting::{caixa, encadeia, P};
use crate::planificada::Planificada;
use fenkra_core::regras::{self, EstiloRegras};
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub struct Puncao {
    pub nome: String,
    /// ângulo incluído da cunha (graus)
    pub angulo: f64,
    pub raio_ponta: f64,
    pub altura: f64,
    pub largura_corpo: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Matriz {
    pub nome: String,
    pub abertura_v: f64,
    pub angulo_v: f64,
    pub altura: f64,
    /// largura do ombro além da borda do V, de cada lado
    pub ombro: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Material {
    pub nome: String,
    pub escoamento: f64,
    pub modulo: f64,
    pub calibracao: f64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Ferramental {
    pub puncoes: Vec<Puncao>,
    pub matrizes: Vec<Matriz>,
    /// altura livre da máquina acima do plano dos ombros, com o punção embaixo (mm)
    pub altura_livre: f64,
}

fn chave(e: &EstiloRegras, c: &str, esp: f64) -> Result<f64, String> {
    let f = e.campo(c).ok_or_else(|| format!("'{}': falta o campo '{c}' (o valor vem do usuário)", e.nome))?;
    regras::avalia(f, &HashMap::new(), (fenkra_core::estilo_chapa::VAR_ESPESSURA, esp))
}

pub fn puncao_de(e: &EstiloRegras, esp: f64) -> Result<Puncao, String> {
    let p = Puncao { nome: e.nome.clone(), angulo: chave(e, "angulo", esp)?, raio_ponta: chave(e, "raio_ponta", esp)?, altura: chave(e, "altura", esp)?, largura_corpo: chave(e, "largura_corpo", esp)? };
    if !(p.angulo > 0. && p.angulo < 180. && p.altura > 0. && p.largura_corpo > 0. && p.raio_ponta >= 0.) {
        return Err(format!("punção '{}' com medidas inválidas", p.nome));
    }
    Ok(p)
}

pub fn matriz_de(e: &EstiloRegras, esp: f64) -> Result<Matriz, String> {
    let m = Matriz { nome: e.nome.clone(), abertura_v: chave(e, "abertura_v", esp)?, angulo_v: chave(e, "angulo_v", esp)?, altura: chave(e, "altura", esp)?, ombro: chave(e, "ombro", esp)? };
    if !(m.abertura_v > 0. && m.angulo_v > 0. && m.angulo_v < 180. && m.altura > 0. && m.ombro >= 0.) {
        return Err(format!("matriz '{}' com medidas inválidas", m.nome));
    }
    Ok(m)
}

pub fn material_de(e: &EstiloRegras, esp: f64) -> Result<Material, String> {
    let calibracao = match e.campo("calibracao") {
        Some(_) => chave(e, "calibracao", esp)?,
        None => 1.,
    };
    let m = Material { nome: e.nome.clone(), escoamento: chave(e, "escoamento", esp)?, modulo: chave(e, "modulo", esp)?, calibracao };
    if !(m.escoamento > 0. && m.modulo > 0. && m.calibracao >= 0.) {
        return Err(format!("material '{}' com medidas inválidas", m.nome));
    }
    Ok(m)
}

/// Retorno elástico estimado de uma dobra.
#[derive(Clone, Debug, PartialEq)]
pub struct Retorno {
    /// ângulo a dobrar para o ângulo final cair no alvo (graus)
    pub sobredobra: f64,
    /// `sobredobra - alvo`
    pub delta: f64,
    /// raio interno depois de soltar
    pub raio_final: f64,
}

/// Retorno elástico: ver a documentação do módulo.
pub fn retorno_elastico(raio: f64, esp: f64, alvo: f64, m: &Material) -> Result<Retorno, String> {
    if raio <= 0. || esp <= 0. {
        return Err("raio e espessura precisam ser positivos".into());
    }
    let k = raio * m.escoamento / (m.modulo * esp);
    let razao = 4. * k.powi(3) - 3. * k + 1.;
    // a cúbica só descreve a descarga para K < 0,5 (em 0,5 o raio final é infinito)
    if k >= 0.5 || razao <= 1e-3 {
        return Err(format!("raio interno grande demais para o material '{}' (K = {k:.2}): quase todo o arco é elástico e a fórmula não vale", m.nome));
    }
    let rf = raio / razao;
    let ai = alvo * (rf + esp / 2.) / (raio + esp / 2.);
    // `ai` é o ângulo sob carga que dá o alvo na fórmula do arco neutro; a diferença é calibrável
    let delta = (alvo - ai).abs() * m.calibracao;
    Ok(Retorno { sobredobra: alvo + delta, delta, raio_final: rf })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tipo {
    SemFerramenta,
    AbaCurta,
    AnguloDaFerramenta,
    RaioDaPonta,
    PernaNoPuncao,
    PernaNaMatriz,
    AlturaLivre,
    Retorno,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Problema {
    pub tipo: Tipo,
    pub texto: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PassoSim {
    /// índice em `Planificada::dobras`
    pub dobra: usize,
    /// `DobraPlana::ordem`: a mesma numeração da vista e da tabela, para marcar no 3D
    pub ordem_dobra: usize,
    pub puncao: String,
    pub matriz: String,
    pub alvo: f64,
    pub sobredobra: f64,
    pub retorno: f64,
    /// comprimento da aba que gira, do eixo da dobra à borda livre
    pub aba: f64,
    pub aba_minima: f64,
    /// verifica se a chapa precisa ser virada em relação ao passo anterior
    pub vira: bool,
    pub problemas: Vec<Problema>,
    pub nao_verificado: Vec<String>,
}

impl PassoSim {
    pub fn viavel(&self) -> bool {
        self.problemas.is_empty()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Simulacao {
    pub passos: Vec<PassoSim>,
    /// a sequência simulada é viável de ponta a ponta
    pub viavel: bool,
    pub troca_ferramenta: usize,
    pub viradas: usize,
}

impl Simulacao {
    /// Numeração (`DobraPlana::ordem`) das dobras inviáveis, para a marcação no 3D.
    pub fn inviaveis(&self) -> Vec<usize> {
        self.passos.iter().filter(|p| !p.viavel()).map(|p| p.ordem_dobra).collect()
    }

    pub fn relatorio(&self) -> String {
        let mut s = String::new();
        for (n, p) in self.passos.iter().enumerate() {
            s += &format!(
                "Passo {} - dobra {}: {:.1} graus (sobredobra {:.2}, retorno {:.2}), aba {:.1} mm (mínima {:.1}), punção '{}', matriz '{}'{}\n",
                n + 1, p.ordem_dobra, p.alvo, p.sobredobra, p.retorno, p.aba, p.aba_minima, p.puncao, p.matriz, if p.vira { ", virar a chapa" } else { "" }
            );
            for q in &p.problemas {
                s += &format!("  INVIÁVEL: {}\n", q.texto);
            }
            for q in &p.nao_verificado {
                s += &format!("  não verificado: {q}\n");
            }
        }
        s += &format!("Resultado: {} ({} trocas de ferramenta, {} viradas)\n", if self.viavel { "viável" } else { "inviável" }, self.troca_ferramenta, self.viradas);
        s
    }
}

// ---------- geometria da seção ----------

fn unit(a: P, b: P) -> Option<P> {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let n = dx.hypot(dy);
    (n > 1e-9).then(|| [dx / n, dy / n])
}

struct Perna {
    lado: f64,
    d: f64,
    /// altura com sinal: positiva aponta para o punção
    h: f64,
}

struct Secao {
    /// distância do eixo ao fim do blank em cada lado (+ e -)
    ext: [f64; 2],
    /// distância do eixo à dobra paralela mais próxima, por lado (ou à borda)
    plano: [f64; 2],
    pernas: Vec<Perna>,
    nao_verificado: Vec<String>,
}

fn secao(plana: &Planificada, blank: &[P], i: usize, feitas: &[bool]) -> Option<Secao> {
    let bi = &plana.dobras[i];
    let u = unit(bi.a, bi.b)?;
    let n = [-u[1], u[0]];
    let s = |p: P| (p[0] - bi.a[0]) * n[0] + (p[1] - bi.a[1]) * n[1];
    let (mut smin, mut smax) = (f64::MAX, f64::MIN);
    for p in blank {
        smin = smin.min(s(*p));
        smax = smax.max(s(*p));
    }
    let mut sec = Secao { ext: [smax.max(0.), (-smin).max(0.)], plano: [smax.max(0.), (-smin).max(0.)], pernas: vec![], nao_verificado: vec![] };
    for (j, bj) in plana.dobras.iter().enumerate() {
        if j == i {
            continue;
        }
        let Some(uj) = unit(bj.a, bj.b) else { continue };
        let sen = (u[0] * uj[1] - u[1] * uj[0]).abs();
        let d = (s(bj.a) + s(bj.b)) / 2.;
        if sen < 0.02 {
            let lado = if d >= 0. { 0 } else { 1 };
            sec.plano[lado] = sec.plano[lado].min(d.abs());
            if feitas[j] {
                let alem = if d >= 0. { smax - d } else { d - smin };
                if alem > 1e-6 {
                    let h = if bj.cima == bi.cima { alem } else { -alem };
                    sec.pernas.push(Perna { lado: if d >= 0. { 1. } else { -1. }, d: d.abs(), h });
                }
            }
        } else if feitas[j] {
            sec.nao_verificado.push(format!("a dobra {} já feita não é paralela: a colisão 3D com a máquina não é simulada", bj.ordem));
        }
    }
    Some(sec)
}

fn gira(lado: f64, d: f64, h: f64, th: f64) -> P {
    let (s, c) = th.sin_cos();
    [lado * (d * c - h * s), d * s + h * c]
}

fn dentro_puncao(p: &Puncao, x: f64, y: f64) -> bool {
    if y < 0. || y > p.altura {
        return false;
    }
    let hw = (y * (p.angulo.to_radians() / 2.).tan()).min(p.largura_corpo / 2.);
    x.abs() < hw - 1e-9
}

fn dentro_matriz(m: &Matriz, x: f64, y: f64) -> bool {
    // bloco abaixo do plano dos ombros, largura V/2 + ombro de cada lado, com o V entalhado
    let meia = m.abertura_v / 2. + m.ombro;
    if y >= 0. || y < -m.altura || x.abs() >= meia {
        return false;
    }
    let prof = -y;
    let notch = prof * (m.angulo_v.to_radians() / 2.).tan();
    x.abs() > notch
}

/// Verifica uma dobra com um par de ferramentas. `retorno` é o valor já calculado do material.
fn verifica(plana: &Planificada, sec: &Secao, i: usize, pu: &Puncao, ma: &Matriz, sobredobra: f64, altura_livre: f64) -> Vec<Problema> {
    let b = &plana.dobras[i];
    let mut v = vec![];
    let interno = 180. - sobredobra;
    if interno < 1. {
        // bainha (180 graus): o ângulo interno some e nenhum punção em V fecha a dobra de uma vez
        v.push(Problema { tipo: Tipo::AnguloDaFerramenta, texto: format!("a dobra {} fecha em {:.0} graus (bainha): precisa de pré-dobra aguda e ferramenta de achatar, fora desta simulação", b.ordem, sobredobra.min(180.)) });
        return v;
    }
    if pu.angulo > interno + 1e-9 {
        v.push(Problema { tipo: Tipo::AnguloDaFerramenta, texto: format!("o punção '{}' ({:.1} graus) não cabe no ângulo interno {:.1} graus da dobra {}", pu.nome, pu.angulo, interno, b.ordem) });
    }
    if ma.angulo_v > interno + 1e-9 {
        v.push(Problema { tipo: Tipo::AnguloDaFerramenta, texto: format!("a matriz '{}' (V de {:.1} graus) não cabe no ângulo interno {:.1} graus da dobra {}", ma.nome, ma.angulo_v, interno, b.ordem) });
    }
    if pu.raio_ponta > b.raio + 1e-9 {
        v.push(Problema { tipo: Tipo::RaioDaPonta, texto: format!("a ponta do punção '{}' (R{:.2}) é maior que o raio da dobra {} (R{:.2})", pu.nome, pu.raio_ponta, b.ordem, b.raio) });
    }
    let minima = ma.abertura_v / 2.;
    for (lado, nome) in [(0usize, "positivo"), (1, "negativo")] {
        let livre = sec.plano[lado];
        if sec.ext[lado] > 1e-6 && livre < minima - 1e-9 {
            v.push(Problema { tipo: Tipo::AbaCurta, texto: format!("a aba plana do lado {nome} da dobra {} tem {:.1} mm e não chega ao ombro da matriz '{}' ({:.1} mm)", b.ordem, livre, ma.nome, minima) });
        }
    }
    let th_final = sobredobra.to_radians() / 2.;
    for pe in &sec.pernas {
        let (mut bate_pu, mut bate_ma, mut acima) = (false, false, f64::MIN);
        let mut th = 0.;
        while th <= th_final + 1e-9 {
            for k in 0..=12 {
                let h = pe.h * k as f64 / 12.;
                let q = gira(pe.lado, pe.d, h, th);
                if (th - th_final).abs() < 0.02 {
                    bate_pu |= dentro_puncao(pu, q[0], q[1]);
                    acima = acima.max(q[1]);
                }
                bate_ma |= dentro_matriz(ma, q[0], q[1]);
            }
            th += 1f64.to_radians();
        }
        if bate_pu {
            v.push(Problema { tipo: Tipo::PernaNoPuncao, texto: format!("a perna já dobrada a {:.1} mm do eixo (altura {:.1} mm) bate no punção '{}'", pe.d, pe.h.abs(), pu.nome) });
        }
        if bate_ma {
            v.push(Problema { tipo: Tipo::PernaNaMatriz, texto: format!("a perna já dobrada a {:.1} mm do eixo (altura {:.1} mm) bate na matriz '{}'", pe.d, pe.h.abs(), ma.nome) });
        }
        if altura_livre > 0. && acima > altura_livre - pu.altura {
            v.push(Problema { tipo: Tipo::AlturaLivre, texto: format!("a perna sobe {:.1} mm acima da matriz e passa da altura livre da máquina ({:.1} mm com o punção '{}')", acima, altura_livre - pu.altura, pu.nome) });
        }
    }
    v
}

fn passo(plana: &Planificada, blank: &[P], i: usize, feitas: &[bool], fer: &Ferramental, mat: &Material, par: Option<(usize, usize)>) -> PassoSim {
    let b = &plana.dobras[i];
    let base = PassoSim { dobra: i, ordem_dobra: b.ordem, puncao: String::new(), matriz: String::new(), alvo: b.angulo.abs(), sobredobra: b.angulo.abs(), retorno: 0., aba: 0., aba_minima: 0., vira: false, problemas: vec![], nao_verificado: vec![] };
    let Some(sec) = secao(plana, blank, i, feitas) else {
        return PassoSim { problemas: vec![Problema { tipo: Tipo::SemFerramenta, texto: format!("a dobra {} tem linha degenerada", b.ordem) }], ..base };
    };
    let aba = sec.ext[0].min(if sec.ext[1] > 1e-6 { sec.ext[1] } else { f64::MAX });
    let mut p = PassoSim { aba, nao_verificado: sec.nao_verificado.clone(), ..base };
    let mut problemas = vec![];
    match retorno_elastico(b.raio, plana.espessura, p.alvo, mat) {
        Ok(r) => {
            p.sobredobra = r.sobredobra;
            p.retorno = r.delta;
        }
        Err(e) => problemas.push(Problema { tipo: Tipo::Retorno, texto: e }),
    }
    if fer.puncoes.is_empty() || fer.matrizes.is_empty() {
        problemas.push(Problema { tipo: Tipo::SemFerramenta, texto: "a biblioteca não tem punção ou matriz: cadastre as do seu ferramental".into() });
        p.problemas = problemas;
        return p;
    }
    let pares: Vec<(usize, usize)> = match par {
        Some(x) => vec![x],
        None => (0..fer.puncoes.len()).flat_map(|a| (0..fer.matrizes.len()).map(move |m| (a, m))).collect(),
    };
    let mut pior: Option<(Vec<Problema>, usize, usize)> = None;
    let mut ok = None;
    for (a, m) in pares {
        let v = verifica(plana, &sec, i, &fer.puncoes[a], &fer.matrizes[m], p.sobredobra, fer.altura_livre);
        if v.is_empty() {
            ok = Some((a, m));
            break;
        }
        if pior.as_ref().is_none_or(|x| v.len() < x.0.len()) {
            pior = Some((v, a, m));
        }
    }
    let (a, m) = match (ok, pior) {
        (Some(x), _) => x,
        (None, Some((v, a, m))) => {
            problemas.extend(v);
            (a, m)
        }
        (None, None) => (0, 0),
    };
    p.puncao = fer.puncoes[a].nome.clone();
    p.matriz = fer.matrizes[m].nome.clone();
    p.aba_minima = fer.matrizes[m].abertura_v / 2.;
    p.problemas = problemas;
    p
}

/// Simula a sequência dada (índices de `Planificada::dobras`, na ordem de dobra). Com `par`, usa só
/// aquele par (punção, matriz); sem, escolhe por dobra o primeiro par que cabe.
pub fn simular(plana: &Planificada, sequencia: &[usize], fer: &Ferramental, mat: &Material, par: Option<(usize, usize)>) -> Result<Simulacao, String> {
    let blank = blank_poligono(plana)?;
    let mut feitas = vec![false; plana.dobras.len()];
    let mut passos: Vec<PassoSim> = vec![];
    for &i in sequencia {
        if i >= plana.dobras.len() || feitas[i] {
            return Err(format!("sequência inválida: dobra {i} fora da lista ou repetida"));
        }
        let mut p = passo(plana, &blank, i, &feitas, fer, mat, par);
        if let Some(ant) = passos.last() {
            p.vira = plana.dobras[ant.dobra].cima != plana.dobras[i].cima;
        }
        feitas[i] = true;
        passos.push(p);
    }
    Ok(resume(plana, passos))
}

fn resume(plana: &Planificada, passos: Vec<PassoSim>) -> Simulacao {
    let troca = passos.windows(2).filter(|w| (&w[0].puncao, &w[0].matriz) != (&w[1].puncao, &w[1].matriz)).count();
    let viradas = passos.iter().filter(|p| p.vira).count();
    let _ = plana;
    Simulacao { viavel: passos.iter().all(PassoSim::viavel), passos, troca_ferramenta: troca, viradas }
}

fn blank_poligono(plana: &Planificada) -> Result<Vec<P>, String> {
    let l = encadeia(&plana.contorno);
    let mut pts: Vec<P> = vec![];
    for laco in &l {
        pts.extend(laco.iter().copied());
    }
    if pts.len() < 3 {
        return Err("a planificada não tem contorno".into());
    }
    let (a, b) = caixa(&pts);
    if b[0] - a[0] <= 0. || b[1] - a[1] <= 0. {
        return Err("a planificada tem contorno degenerado".into());
    }
    Ok(pts)
}

/// Resultado da busca de ordem.
#[derive(Clone, Debug, PartialEq)]
pub struct Sugestao {
    pub sequencia: Vec<usize>,
    pub simulacao: Simulacao,
    /// nós visitados na busca
    pub nos: usize,
    /// a busca parou no limite antes de esgotar as ordens
    pub truncada: bool,
}

/// Sugere uma ordem de dobra por busca em profundidade com poda: só avança por passos viáveis e,
/// entre as ordens completas viáveis, guarda a de menor custo (trocas de ferramenta, depois viradas
/// da chapa). Para no `limite` de nós. Sem nenhuma ordem viável, devolve `Ok(None)`.
pub fn sugerir_ordem(plana: &Planificada, fer: &Ferramental, mat: &Material, par: Option<(usize, usize)>, limite: usize) -> Result<Option<Sugestao>, String> {
    let blank = blank_poligono(plana)?;
    let n = plana.dobras.len();
    struct Busca<'a> {
        plana: &'a Planificada,
        blank: &'a [P],
        fer: &'a Ferramental,
        mat: &'a Material,
        par: Option<(usize, usize)>,
        limite: usize,
        nos: usize,
        melhor: Option<(Vec<PassoSim>, (usize, usize))>,
        truncada: bool,
    }
    fn custo(p: &[PassoSim]) -> (usize, usize) {
        (p.windows(2).filter(|w| (&w[0].puncao, &w[0].matriz) != (&w[1].puncao, &w[1].matriz)).count(), p.iter().filter(|x| x.vira).count())
    }
    fn dfs(b: &mut Busca, feitas: &mut Vec<bool>, passos: &mut Vec<PassoSim>) {
        if b.truncada {
            return;
        }
        if passos.len() == feitas.len() {
            let c = custo(passos);
            if b.melhor.as_ref().is_none_or(|m| c < m.1) {
                b.melhor = Some((passos.clone(), c));
            }
            return;
        }
        // poda: já há solução sem troca nem virada
        if b.melhor.as_ref().is_some_and(|m| m.1 == (0, 0)) {
            return;
        }
        for i in 0..feitas.len() {
            if feitas[i] {
                continue;
            }
            b.nos += 1;
            if b.nos > b.limite {
                b.truncada = true;
                return;
            }
            let mut p = passo(b.plana, b.blank, i, feitas, b.fer, b.mat, b.par);
            if !p.viavel() {
                continue;
            }
            if let Some(ant) = passos.last() {
                p.vira = b.plana.dobras[ant.dobra].cima != b.plana.dobras[i].cima;
            }
            feitas[i] = true;
            passos.push(p);
            if b.melhor.as_ref().is_none_or(|m| custo(passos) < m.1) {
                dfs(b, feitas, passos);
            }
            passos.pop();
            feitas[i] = false;
        }
    }
    let mut b = Busca { plana, blank: &blank, fer, mat, par, limite, nos: 0, melhor: None, truncada: false };
    dfs(&mut b, &mut vec![false; n], &mut vec![]);
    Ok(b.melhor.map(|(p, _)| Sugestao { sequencia: p.iter().map(|x| x.dobra).collect(), simulacao: resume(plana, p), nos: b.nos, truncada: b.truncada }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planificada::{DobraPlana, Entidade, Propriedades};

    fn mat() -> Material {
        material_de(&EstiloRegras::novo("teste").com("escoamento", "250").com("modulo", "200000"), 2.).unwrap()
    }

    fn fer() -> Ferramental {
        let p = EstiloRegras::novo("P88").com("angulo", "88").com("raio_ponta", "1").com("altura", "100").com("largura_corpo", "30");
        let m = EstiloRegras::novo("V16").com("abertura_v", "8*espessura").com("angulo_v", "88").com("altura", "60").com("ombro", "20");
        Ferramental { puncoes: vec![puncao_de(&p, 2.).unwrap()], matrizes: vec![matriz_de(&m, 2.).unwrap()], altura_livre: 300. }
    }

    /// Canal em U: blank 200 x 100 (x de 0 a 200), duas dobras paralelas a y em x = 40 e x = 160.
    fn canal(altura_perna: f64) -> Planificada {
        let (a, b) = (altura_perna, 200. - altura_perna);
        let ls = vec![Entidade::Linha([0., 0.], [200., 0.]), Entidade::Linha([200., 0.], [200., 100.]), Entidade::Linha([200., 100.], [0., 100.]), Entidade::Linha([0., 100.], [0., 0.])];
        let dobra = |ordem, x| DobraPlana { curva: vec![], ordem, rotulo: String::new(), a: [x, 0.], b: [x, 100.], cima: true, angulo: 90., raio: 2., desenvolvido: 3. };
        Planificada {
            espessura: 2.,
            material: String::new(),
            contorno: ls,
            furos: vec![],
            ferramenta: vec![],
            dobras: vec![dobra(1, a), dobra(2, b)],
            propriedades: Propriedades { largura: 200., altura: 100., area: 20000., volume: 40000., massa: None, dobras: 2 },
            sobreposicoes: vec![],
            ausentes: vec![],
        }
    }

    #[test]
    fn retorno_elastico_cresce_com_o_escoamento_e_a_calibracao_escala() {
        let m = mat();
        let r = retorno_elastico(2., 2., 90., &m).unwrap();
        assert!(r.delta > 0. && r.delta < 10., "{r:?}");
        assert!(r.raio_final > 2.);
        let duro = Material { escoamento: 500., ..m.clone() };
        assert!(retorno_elastico(2., 2., 90., &duro).unwrap().delta > r.delta);
        let calibrado = Material { calibracao: 2., ..m.clone() };
        assert!((retorno_elastico(2., 2., 90., &calibrado).unwrap().delta - 2. * r.delta).abs() < 1e-9);
        // raio enorme: regime elástico, a fórmula não vale
        assert!(retorno_elastico(2000., 2., 90., &m).is_err());
    }

    #[test]
    fn ferramenta_sem_valor_do_usuario_e_recusada_nao_inventada() {
        let e = EstiloRegras::novo("P").com("angulo", "88");
        assert!(puncao_de(&e, 2.).unwrap_err().contains("raio_ponta"));
        let e = EstiloRegras::novo("M").com("escoamento", "250");
        assert!(material_de(&e, 2.).unwrap_err().contains("modulo"));
    }

    #[test]
    fn canal_com_pernas_curtas_e_viavel_e_com_pernas_altas_bate_no_punção() {
        // pernas de 40 mm: a segunda dobra, com a primeira perna de pé, cabe ao lado do punção de 30 mm
        let sim = simular(&canal(40.), &[0, 1], &fer(), &mat(), None).unwrap();
        assert!(sim.viavel, "{}", sim.relatorio());
        // a mesma peça com perna enorme: a aba de 16 mm é menor que V/2 = 8? 16 > 8, então testa a perna alta
        let mut alta = canal(40.);
        alta.contorno = vec![Entidade::Linha([0., 0.], [200., 0.]), Entidade::Linha([200., 0.], [200., 100.]), Entidade::Linha([200., 100.], [0., 100.]), Entidade::Linha([0., 100.], [0., 0.])];
        // pernas de 6 mm (aba menor que V/2 = 8 mm) não se apoiam na matriz
        let curta = canal(6.);
        let sim = simular(&curta, &[0, 1], &fer(), &mat(), None).unwrap();
        assert!(!sim.viavel);
        assert!(sim.passos.iter().flat_map(|p| &p.problemas).any(|q| q.tipo == Tipo::AbaCurta), "{}", sim.relatorio());
        assert!(!sim.inviaveis().is_empty());
    }

    #[test]
    fn perna_alta_perto_do_eixo_bate_no_puncao_e_a_ordem_do_usuario_importa() {
        // dobras em x = 15 e x = 25 (aba central de 10 mm, perto do eixo): com a primeira feita e a
        // perna de 15 mm de pé a 10 mm do eixo, a cunha (corpo de 30 mm) pega na perna
        let mut p = canal(15.);
        p.dobras[1] = DobraPlana { a: [25., 0.], b: [25., 100.], ..p.dobras[1].clone() };
        let f = fer();
        let sim = simular(&p, &[0, 1], &f, &mat(), None).unwrap();
        assert!(sim.passos[1].problemas.iter().any(|q| matches!(q.tipo, Tipo::PernaNoPuncao | Tipo::PernaNaMatriz)), "{}", sim.relatorio());
    }

    #[test]
    fn angulo_do_puncao_maior_que_o_interno_impede_dobra_aguda() {
        let mut p = canal(40.);
        for d in p.dobras.iter_mut() {
            d.angulo = 120.; // interno 60 graus: punção de 88 não cabe
        }
        let sim = simular(&p, &[0, 1], &fer(), &mat(), None).unwrap();
        assert!(sim.passos[0].problemas.iter().any(|q| q.tipo == Tipo::AnguloDaFerramenta));
    }

    #[test]
    fn sugestao_acha_ordem_viavel_e_respeita_o_limite_de_nos() {
        let s = sugerir_ordem(&canal(40.), &fer(), &mat(), None, 10_000).unwrap().expect("há ordem viável");
        assert_eq!(s.sequencia.len(), 2);
        assert!(s.simulacao.viavel);
        assert_eq!(s.simulacao.troca_ferramenta, 0);
        // limite de um nó: trunca
        let t = sugerir_ordem(&canal(40.), &fer(), &mat(), None, 1).unwrap();
        assert!(t.is_none() || t.unwrap().truncada);
        // sem ferramental: nenhuma ordem
        assert!(sugerir_ordem(&canal(40.), &Ferramental::default(), &mat(), None, 1000).unwrap().is_none());
    }

    #[test]
    fn bainha_de_180_graus_e_apontada_e_nao_derruba_a_simulacao() {
        let mut p = canal(40.);
        p.dobras[0].angulo = 180.;
        let sim = simular(&p, &[0, 1], &fer(), &mat(), None).unwrap();
        assert!(!sim.passos[0].viavel());
        assert!(sim.passos[0].problemas[0].texto.contains("bainha"), "{}", sim.relatorio());
        assert!(sim.passos[1].viavel(), "a segunda dobra segue verificada: {}", sim.relatorio());
    }

    #[test]
    fn sequencia_invalida_e_recusada() {
        assert!(simular(&canal(40.), &[0, 0], &fer(), &mat(), None).is_err());
        assert!(simular(&canal(40.), &[5], &fer(), &mat(), None).is_err());
    }
}
