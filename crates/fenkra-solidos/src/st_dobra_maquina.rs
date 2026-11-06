// SPDX-License-Identifier: LGPL-2.1-or-later
//! Estrutural, B332: dados de dobra para a dobradeira de tubo, em formatos abertos de texto.
//!
//! A rota do tubo tem dois modelos equivalentes:
//!
//! - **LRA** (ou **YBC**, as mesmas três grandezas com outras letras): `L` é o trecho reto antes da
//!   dobra (de tangente a tangente), `R` é a rotação do tubo em torno do próprio eixo entre o plano da
//!   dobra anterior e o desta, e `A` é o ângulo da dobra. É o que a máquina consome.
//! - **XYZ**: os pontos do eixo (início, cruzamento das linhas guia em cada dobra e fim) mais o raio de
//!   centro de cada dobra. É o que o desenho consome. A conversão XYZ -> LRA é o cálculo inverso.
//!
//! Convenções (cada máquina escolhe as suas; confira no software dela antes de produzir):
//!
//! - o primeiro plano de dobra define a orientação do tubo: sua rotação só gira a peça inteira;
//! - `R` incremental é o giro, pela regra da mão direita em torno do sentido de avanço, entre a direção
//!   para a qual a dobra anterior fechou (já carregada pelo tubo) e a direção desta dobra; `R = 0` é a
//!   mesma direção (dobras no mesmo plano, mesmo lado), `R = 180` é a dobra em S;
//! - no modo absoluto, `L` é a posição, no tubo reto desenvolvido, em que a dobra começa (retos e
//!   arcos anteriores somados) e `R` é a rotação acumulada; no incremental, cada valor vale só para sua
//!   dobra;
//! - o tubo é medido no eixo; o comprimento de corte da barra usa o arco da linha neutra.
//!
//! Ver `docs/pesquisa/dobra-de-tubos.md` (seção 5) e a nota de procedência.

use crate::st_dobra::TuboDobrado;
use fenkra_core::BendMode;

type V = [f64; 3];

fn sub(a: V, b: V) -> V {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn add(a: V, b: V) -> V {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn mul(a: V, k: f64) -> V {
    [a[0] * k, a[1] * k, a[2] * k]
}
fn dot(a: V, b: V) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: V, b: V) -> V {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn norma(a: V) -> f64 {
    dot(a, a).sqrt()
}
fn unit(a: V) -> Option<V> {
    let n = norma(a);
    (n > 1e-12).then(|| mul(a, 1. / n))
}

/// Normaliza um ângulo para (-180, 180].
fn normaliza(a: f64) -> f64 {
    let mut x = a % 360.;
    if x > 180. {
        x -= 360.;
    } else if x <= -180. {
        x += 360.;
    }
    x
}

/// Uma dobra da rota, na ordem do tubo.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DobraMaq {
    /// raio de centro (CLR), mm
    pub raio: f64,
    /// ângulo geométrico da dobra, graus
    pub angulo: f64,
    /// ângulo que a máquina dobra (com o retorno elástico compensado), graus
    pub angulo_maquina: f64,
    /// rotação incremental em relação à dobra anterior, graus, em (-180, 180]
    pub rotacao: f64,
    /// comprimento do arco na linha neutra, mm
    pub arco: f64,
}

impl DobraMaq {
    /// Distância do cruzamento das linhas guia ao início do arco.
    pub fn tangente(&self) -> f64 {
        self.raio * (self.angulo.to_radians() / 2.).tan()
    }
}

/// A rota do tubo: `retos.len() == dobras.len() + 1`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RotaTubo {
    /// trechos retos de tangente a tangente: antes da primeira dobra, entre elas e depois da última
    pub retos: Vec<f64>,
    pub dobras: Vec<DobraMaq>,
}

impl RotaTubo {
    /// Comprimento de corte da barra: retos mais arcos.
    pub fn desenvolvido(&self) -> f64 {
        self.retos.iter().sum::<f64>() + self.dobras.iter().map(|d| d.arco).sum::<f64>()
    }
}

/// Pontos do eixo e raio de cada dobra, o modelo XYZ.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RotaXyz {
    /// início, cruzamento de cada dobra e fim
    pub pontos: Vec<V>,
    /// raio de centro das dobras (um por ponto de dobra, `pontos.len() - 2`)
    pub raios: Vec<f64>,
}

/// LRA -> XYZ: refaz o eixo do tubo. O primeiro ponto fica na origem, o primeiro reto ao longo de +X
/// e a primeira dobra para o lado +Y (girada pela rotação da primeira dobra, se houver).
pub fn para_xyz(r: &RotaTubo) -> Result<RotaXyz, String> {
    if r.retos.len() != r.dobras.len() + 1 {
        return Err("a rota precisa de um trecho reto a mais que as dobras".into());
    }
    if let Some(l) = r.retos.iter().find(|l| **l < -1e-9) {
        return Err(format!("trecho reto negativo ({l:.3} mm)"));
    }
    let mut p: V = [0.; 3];
    let mut d: V = [1., 0., 0.];
    let mut w: V = [0., 1., 0.];
    let mut pontos = vec![p];
    let mut raios = vec![];
    for (i, b) in r.dobras.iter().enumerate() {
        if !(b.angulo > 0. && b.angulo < 180.) {
            return Err(format!("dobra {}: o ângulo {:.3}° precisa estar entre 0° e 180°", i + 1, b.angulo));
        }
        if b.raio <= 0. {
            return Err(format!("dobra {}: raio inválido", i + 1));
        }
        let rot = b.rotacao.to_radians();
        // gira a direção da dobra em torno do avanço
        w = add(mul(w, rot.cos()), mul(cross(d, w), rot.sin()));
        p = add(p, mul(d, r.retos[i]));
        let t = b.tangente();
        let no = add(p, mul(d, t));
        let a = b.angulo.to_radians();
        let d2 = add(mul(d, a.cos()), mul(w, a.sin()));
        let w2 = add(mul(d, -a.sin()), mul(w, a.cos()));
        pontos.push(no);
        raios.push(b.raio);
        p = add(no, mul(d2, t));
        d = d2;
        w = w2;
    }
    p = add(p, mul(d, r.retos[r.dobras.len()]));
    pontos.push(p);
    Ok(RotaXyz { pontos, raios })
}

/// XYZ -> LRA: o cálculo inverso. Pontos alinhados (sem dobra) são descartados. O ângulo de máquina
/// copia o geométrico (o retorno elástico é aplicado por quem conhece a regra); o arco usa a linha
/// de centro. Falha se um reto sai negativo (raio grande demais para as pernas).
pub fn de_xyz(x: &RotaXyz) -> Result<RotaTubo, String> {
    if x.pontos.len() < 2 {
        return Err("a rota precisa de ao menos dois pontos".into());
    }
    if x.raios.len() + 2 != x.pontos.len() {
        return Err(format!("{} pontos pedem {} raios, mas vieram {}", x.pontos.len(), x.pontos.len() - 2, x.raios.len()));
    }
    // descarta pontos alinhados
    let mut pts = vec![x.pontos[0]];
    let mut raios: Vec<f64> = vec![];
    for k in 1..x.pontos.len() - 1 {
        let a = unit(sub(x.pontos[k], *pts.last().unwrap())).ok_or_else(|| format!("pontos {} e {} coincidem", k, k + 1))?;
        let b = unit(sub(x.pontos[k + 1], x.pontos[k])).ok_or_else(|| format!("pontos {} e {} coincidem", k + 1, k + 2))?;
        if dot(a, b) < 1. - 1e-12 {
            pts.push(x.pontos[k]);
            raios.push(x.raios[k - 1]);
        }
    }
    pts.push(*x.pontos.last().unwrap());
    let n = raios.len();
    let mut dobras: Vec<DobraMaq> = vec![];
    let mut tang = vec![];
    let mut w_prev: Option<(V, f64, V)> = None; // (w da dobra anterior, ângulo, direção antes dela)
    for i in 1..=n {
        let ent = unit(sub(pts[i], pts[i - 1])).ok_or("trecho de comprimento zero")?;
        let sai = unit(sub(pts[i + 1], pts[i])).ok_or("trecho de comprimento zero")?;
        let c = dot(ent, sai).clamp(-1., 1.);
        let ang = c.acos();
        if ang > 179.9_f64.to_radians() {
            return Err(format!("dobra {i}: o retorno de {:.2}° não é uma dobra", ang.to_degrees()));
        }
        let r = raios[i - 1];
        if r <= 0. {
            return Err(format!("dobra {i}: raio inválido"));
        }
        let w = unit(sub(sai, mul(ent, c))).ok_or("dobra sem plano")?;
        let rotacao = match w_prev {
            None => 0.,
            Some((wp, ap, dp)) => {
                // a direção da dobra anterior, já carregada pelo tubo até este trecho
                let carregada = add(mul(dp, -ap.sin()), mul(wp, ap.cos()));
                let s = dot(ent, cross(carregada, w));
                let co = dot(carregada, w);
                normaliza(s.atan2(co).to_degrees())
            }
        };
        let t = r * (ang / 2.).tan();
        tang.push(t);
        dobras.push(DobraMaq { raio: r, angulo: ang.to_degrees(), angulo_maquina: ang.to_degrees(), rotacao, arco: r * ang });
        w_prev = Some((w, ang, ent));
    }
    let mut retos = vec![];
    for i in 0..=n {
        let comp = norma(sub(pts[i + 1], pts[i]));
        let antes = if i > 0 { tang[i - 1] } else { 0. };
        let depois = if i < n { tang[i] } else { 0. };
        let l = comp - antes - depois;
        if l < -1e-6 {
            return Err(format!("o trecho {} mede {:.3} mm, menos que as tangentes das dobras ({:.3} mm): reduza o raio ou alongue o trecho", i + 1, comp, antes + depois));
        }
        retos.push(l.max(0.));
    }
    Ok(RotaTubo { retos, dobras })
}

/// A rota de um tubo dobrado do documento. Só vale se todas as dobras forem de dobradeira.
pub fn rota_do_tubo(t: &TuboDobrado) -> Result<RotaTubo, String> {
    if t.dobras.is_empty() {
        return Err("o tubo não tem dobras".into());
    }
    if let Some(d) = t.dobras.iter().find(|d| d.resultado.modo != BendMode::Dobradeira) {
        return Err(format!("os dados de máquina valem para dobradeira; esta dobra está em «{}»", crate::st_dobra::nome_modo(d.resultado.modo)));
    }
    if let Some(e) = t.erros.first() {
        return Err(e.clone());
    }
    if t.pontos.len() != t.dobras.len() + 2 {
        return Err("o tubo não tem os pontos do eixo".into());
    }
    let raios: Vec<f64> = t.dobras.iter().map(|d| d.resultado.raio).collect();
    let mut rota = de_xyz(&RotaXyz { pontos: t.pontos.clone(), raios })?;
    if rota.dobras.len() != t.dobras.len() {
        return Err("há dobras de 0° no tubo".into());
    }
    for (b, d) in rota.dobras.iter_mut().zip(&t.dobras) {
        b.angulo_maquina = d.resultado.angulo_maquina;
        b.arco = d.resultado.arco;
    }
    Ok(rota)
}

/// Maior diferença (mm ou graus) entre duas rotas; `None` se não têm o mesmo número de dobras.
pub fn diferenca(a: &RotaTubo, b: &RotaTubo) -> Option<f64> {
    if a.dobras.len() != b.dobras.len() {
        return None;
    }
    let mut m: f64 = 0.;
    for (x, y) in a.retos.iter().zip(&b.retos) {
        m = m.max((x - y).abs());
    }
    for (x, y) in a.dobras.iter().zip(&b.dobras) {
        m = m.max((x.raio - y.raio).abs()).max((x.angulo - y.angulo).abs()).max(normaliza(x.rotacao - y.rotacao).abs());
    }
    Some(m)
}

// ---- texto -------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Formato {
    Lra,
    Ybc,
    Xyz,
}

impl Formato {
    pub fn nome(self) -> &'static str {
        match self {
            Formato::Lra => "LRA",
            Formato::Ybc => "YBC",
            Formato::Xyz => "XYZ",
        }
    }
    /// Extensão sugerida do arquivo.
    pub fn extensao(self) -> &'static str {
        match self {
            Formato::Lra => "lra.csv",
            Formato::Ybc => "ybc.csv",
            Formato::Xyz => "xyz.csv",
        }
    }
    fn letras(self) -> [&'static str; 3] {
        match self {
            Formato::Lra => ["L", "R", "A"],
            Formato::Ybc => ["Y", "B", "C"],
            Formato::Xyz => ["X", "Y", "Z"],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Modo {
    Incremental,
    Absoluto,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Opcoes {
    pub modo: Modo,
    /// escreve o ângulo que a máquina dobra (com o retorno) no lugar do geométrico
    pub angulo_maquina: bool,
    /// inverte o sentido da rotação (máquinas que giram ao contrário)
    pub inverter_rotacao: bool,
}

impl Default for Opcoes {
    fn default() -> Self {
        Opcoes { modo: Modo::Incremental, angulo_maquina: false, inverter_rotacao: false }
    }
}

/// O que um arquivo lido traz.
#[derive(Clone, Debug, PartialEq)]
pub struct Importado {
    pub formato: Formato,
    pub opcoes: Opcoes,
    pub tubo: String,
    pub rota: RotaTubo,
    pub xyz: RotaXyz,
}

fn n6(v: f64) -> String {
    let s = format!("{v:.6}");
    if s == "-0.000000" {
        "0.000000".into()
    } else {
        s
    }
}

/// Escreve a rota em LRA ou YBC (`formato` Xyz cai em `exportar_xyz`).
pub fn exportar(r: &RotaTubo, formato: Formato, opc: Opcoes, tubo: &str) -> Result<String, String> {
    if formato == Formato::Xyz {
        return exportar_xyz(r, tubo);
    }
    if r.retos.len() != r.dobras.len() + 1 {
        return Err("a rota precisa de um trecho reto a mais que as dobras".into());
    }
    let [l, rt, a] = formato.letras();
    let mut t = String::new();
    t += &format!("formato;{}\nversao;1\nunidade;mm;graus\n", formato.nome());
    t += &format!("modo;{}\n", if opc.modo == Modo::Absoluto { "absoluto" } else { "incremental" });
    t += &format!("angulo;{}\n", if opc.angulo_maquina { "maquina" } else { "nominal" });
    t += &format!("rotacao;{}\n", if opc.inverter_rotacao { "invertida" } else { "normal" });
    t += &format!("tubo;{}\n", tubo.replace(';', ","));
    t += &format!("desenvolvido;{}\n", n6(r.desenvolvido()));
    t += &format!("n;{l};{rt};{a};raio;arco\n");
    let sinal = if opc.inverter_rotacao { -1. } else { 1. };
    let (mut pos, mut rot_acc) = (0., 0.);
    for (i, b) in r.dobras.iter().enumerate() {
        let ang = if opc.angulo_maquina { b.angulo_maquina } else { b.angulo };
        let (lv, rv) = match opc.modo {
            Modo::Incremental => (r.retos[i], b.rotacao),
            Modo::Absoluto => {
                pos += r.retos[i];
                rot_acc += b.rotacao;
                (pos, rot_acc)
            }
        };
        t += &format!("{};{};{};{};{};{}\n", i + 1, n6(lv), n6(sinal * rv), n6(ang), n6(b.raio), n6(b.arco));
        pos += b.arco;
    }
    let fim = match opc.modo {
        Modo::Incremental => r.retos[r.dobras.len()],
        Modo::Absoluto => pos + r.retos[r.dobras.len()],
    };
    t += &format!("fim;{}\n", n6(fim));
    Ok(t)
}

/// Escreve os pontos XYZ do eixo.
pub fn exportar_xyz(r: &RotaTubo, tubo: &str) -> Result<String, String> {
    let x = para_xyz(r)?;
    let mut t = String::new();
    t += "formato;XYZ\nversao;1\nunidade;mm;graus\n";
    t += &format!("tubo;{}\n", tubo.replace(';', ","));
    t += &format!("desenvolvido;{}\n", n6(r.desenvolvido()));
    t += "n;X;Y;Z;raio\n";
    for (i, p) in x.pontos.iter().enumerate() {
        let raio = if i >= 1 && i <= x.raios.len() { n6(x.raios[i - 1]) } else { String::new() };
        t += &format!("{};{};{};{};{}\n", i, n6(p[0]), n6(p[1]), n6(p[2]), raio);
    }
    Ok(t)
}

fn num(s: &str, onde: &str) -> Result<f64, String> {
    s.trim().replace(',', ".").parse::<f64>().map_err(|_| format!("{onde}: «{}» não é número", s.trim()))
}

/// Lê um arquivo escrito por `exportar` ou `exportar_xyz` (ou à mão, no mesmo desenho).
pub fn importar(texto: &str) -> Result<Importado, String> {
    let mut formato: Option<Formato> = None;
    let mut opc = Opcoes::default();
    let mut tubo = String::new();
    let mut linhas_dado: Vec<(usize, Vec<String>)> = vec![];
    let mut fim: Option<f64> = None;
    let mut cabecalho_visto = false;
    for (k, bruta) in texto.lines().enumerate() {
        let lin = bruta.trim().trim_start_matches('\u{feff}');
        if lin.is_empty() || lin.starts_with('#') {
            continue;
        }
        let c: Vec<String> = lin.split(';').map(|s| s.trim().to_string()).collect();
        match c[0].to_lowercase().as_str() {
            "formato" => {
                formato = Some(match c.get(1).map(|s| s.to_uppercase()).as_deref() {
                    Some("LRA") => Formato::Lra,
                    Some("YBC") => Formato::Ybc,
                    Some("XYZ") => Formato::Xyz,
                    other => return Err(format!("linha {}: formato «{}» desconhecido", k + 1, other.unwrap_or(""))),
                })
            }
            "versao" | "unidade" | "desenvolvido" => {}
            "modo" => opc.modo = if c.get(1).map(|s| s.to_lowercase()).as_deref() == Some("absoluto") { Modo::Absoluto } else { Modo::Incremental },
            "angulo" => opc.angulo_maquina = c.get(1).map(|s| s.to_lowercase()).as_deref() == Some("maquina"),
            "rotacao" => opc.inverter_rotacao = c.get(1).map(|s| s.to_lowercase()).as_deref() == Some("invertida"),
            "tubo" => tubo = c.get(1).cloned().unwrap_or_default(),
            "n" => cabecalho_visto = true,
            "fim" => fim = Some(num(c.get(1).map(String::as_str).unwrap_or(""), &format!("linha {}", k + 1))?),
            _ => {
                if !cabecalho_visto {
                    return Err(format!("linha {}: dado antes do cabeçalho «n;…»", k + 1));
                }
                linhas_dado.push((k + 1, c));
            }
        }
    }
    let formato = formato.ok_or("o arquivo não diz o formato (linha «formato;LRA», «YBC» ou «XYZ»)")?;
    if formato == Formato::Xyz {
        let mut pontos = vec![];
        let mut raios_nos: Vec<Option<f64>> = vec![];
        for (k, c) in &linhas_dado {
            if c.len() < 4 {
                return Err(format!("linha {k}: faltam colunas (n;X;Y;Z;raio)"));
            }
            let onde = format!("linha {k}");
            pontos.push([num(&c[1], &onde)?, num(&c[2], &onde)?, num(&c[3], &onde)?]);
            raios_nos.push(match c.get(4).map(String::as_str) {
                Some(s) if !s.is_empty() => Some(num(s, &onde)?),
                _ => None,
            });
        }
        if pontos.len() < 2 {
            return Err("a rota precisa de ao menos dois pontos".into());
        }
        let m = pontos.len();
        let mut raios = vec![];
        for (i, r) in raios_nos.iter().enumerate().take(m - 1).skip(1) {
            raios.push(r.ok_or_else(|| format!("o ponto {i} é de dobra e precisa do raio"))?);
        }
        let xyz = RotaXyz { pontos, raios };
        let rota = de_xyz(&xyz)?;
        return Ok(Importado { formato, opcoes: opc, tubo, rota, xyz });
    }
    let sinal = if opc.inverter_rotacao { -1. } else { 1. };
    let mut dobras = vec![];
    let mut ls = vec![];
    for (k, c) in &linhas_dado {
        if c.len() < 5 {
            return Err(format!("linha {k}: faltam colunas (n;L;R;A;raio)"));
        }
        let onde = format!("linha {k}");
        let (lv, rv, av, raio) = (num(&c[1], &onde)?, sinal * num(&c[2], &onde)?, num(&c[3], &onde)?, num(&c[4], &onde)?);
        let arco = match c.get(5).map(String::as_str) {
            Some(s) if !s.is_empty() => num(s, &onde)?,
            _ => raio * av.to_radians(),
        };
        ls.push(lv);
        dobras.push(DobraMaq { raio, angulo: av, angulo_maquina: av, rotacao: rv, arco });
    }
    let fim = fim.ok_or("falta a linha «fim;L» com o último trecho reto")?;
    ls.push(fim);
    let mut retos = vec![];
    if opc.modo == Modo::Absoluto {
        // L é posição no desenvolvido, R é acumulada: volta aos valores por dobra
        let mut anterior = 0.;
        let mut rot_ant = 0.;
        for (i, b) in dobras.iter_mut().enumerate() {
            retos.push(ls[i] - anterior);
            anterior = ls[i] + b.arco;
            let r_abs = b.rotacao;
            b.rotacao = normaliza(r_abs - rot_ant);
            rot_ant = r_abs;
        }
        retos.push(fim - anterior);
    } else {
        retos = ls;
        for b in dobras.iter_mut() {
            b.rotacao = normaliza(b.rotacao);
        }
    }
    if let Some(l) = retos.iter().find(|l| **l < -1e-6) {
        return Err(format!("trecho reto negativo ({l:.3} mm): o arquivo está no modo certo?"));
    }
    for l in retos.iter_mut() {
        *l = l.max(0.);
    }
    let rota = RotaTubo { retos, dobras };
    let xyz = para_xyz(&rota)?;
    Ok(Importado { formato, opcoes: opc, tubo, rota, xyz })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rota3d() -> RotaTubo {
        let b = |raio: f64, angulo: f64, rotacao: f64| DobraMaq { raio, angulo, angulo_maquina: angulo + 2., rotacao, arco: raio * angulo.to_radians() * 0.97 };
        RotaTubo { retos: vec![120., 80., 95.5, 60.], dobras: vec![b(50.8, 90., 0.), b(40., 60., 90.), b(63.5, 45., -120.)] }
    }

    fn dist_pares(p: &[V]) -> Vec<f64> {
        let mut v = vec![];
        for i in 0..p.len() {
            for j in i + 1..p.len() {
                v.push(norma(sub(p[i], p[j])));
            }
        }
        v
    }

    fn quase(a: &RotaTubo, b: &RotaTubo, tol: f64) {
        assert_eq!(a.retos.len(), b.retos.len());
        for (x, y) in a.retos.iter().zip(&b.retos) {
            assert!((x - y).abs() < tol, "reto {x} x {y}");
        }
        for (x, y) in a.dobras.iter().zip(&b.dobras) {
            assert!((x.raio - y.raio).abs() < tol && (x.angulo - y.angulo).abs() < tol, "{x:?} x {y:?}");
            assert!(normaliza(x.rotacao - y.rotacao).abs() < tol, "rotação {} x {}", x.rotacao, y.rotacao);
        }
    }

    #[test]
    fn planar_de_duas_dobras_da_os_pontos_conhecidos() {
        // U: 100 reto, dobra 90 (R 50), 200 reto, dobra 90 no mesmo lado (R 50), 100 reto
        let r = RotaTubo {
            retos: vec![100., 200., 100.],
            dobras: vec![DobraMaq { raio: 50., angulo: 90., angulo_maquina: 90., rotacao: 0., arco: 78.5398 }, DobraMaq { raio: 50., angulo: 90., angulo_maquina: 90., rotacao: 0., arco: 78.5398 }],
        };
        let x = para_xyz(&r).unwrap();
        // nós: (150,0,0), (150,300,0); fim: 150 - 50 (tangente) - 100 (reto) = 0
        let esperado = [[0., 0., 0.], [150., 0., 0.], [150., 300., 0.], [0., 300., 0.]];
        for (p, e) in x.pontos.iter().zip(esperado) {
            assert!(norma(sub(*p, e)) < 1e-9, "{p:?} x {e:?}");
        }
    }

    #[test]
    fn lra_xyz_lra_reproduz_a_rota_3d() {
        let r = rota3d();
        let x = para_xyz(&r).unwrap();
        assert_eq!(x.pontos.len(), 5);
        let volta = de_xyz(&x).unwrap();
        quase(&r, &volta, 1e-6);
    }

    #[test]
    fn xyz_lra_xyz_preserva_as_distancias_de_uma_rota_qualquer() {
        // pontos soltos no espaço, em posição e orientação quaisquer
        let x = RotaXyz { pontos: vec![[10., -5., 3.], [210., 20., 40.], [260., 180., 90.], [120., 260., -30.]], raios: vec![60., 45.] };
        let r = de_xyz(&x).unwrap();
        let y = para_xyz(&r).unwrap();
        let (a, b) = (dist_pares(&x.pontos), dist_pares(&y.pontos));
        for (u, v) in a.iter().zip(&b) {
            assert!((u - v).abs() < 1e-6, "{u} x {v}");
        }
        // e o sentido (lateralidade) também: o determinante dos três vetores tem o mesmo sinal
        let det = |p: &[V]| dot(sub(p[1], p[0]), cross(sub(p[2], p[1]), sub(p[3], p[2])));
        assert!(det(&x.pontos) * det(&y.pontos) > 0.);
    }

    #[test]
    fn texto_lra_ybc_xyz_ida_e_volta() {
        let r = rota3d();
        for formato in [Formato::Lra, Formato::Ybc, Formato::Xyz] {
            for modo in [Modo::Incremental, Modo::Absoluto] {
                for inverter in [false, true] {
                    let opc = Opcoes { modo, angulo_maquina: false, inverter_rotacao: inverter };
                    let t = exportar(&r, formato, opc, "Tubo 25,4 x 1,5").unwrap();
                    let lido = importar(&t).unwrap_or_else(|e| panic!("{formato:?} {modo:?}: {e}\n{t}"));
                    assert_eq!(lido.formato, formato);
                    quase(&r, &lido.rota, 1e-5);
                    // o eixo reconstruído tem as mesmas distâncias
                    let (a, b) = (dist_pares(&para_xyz(&r).unwrap().pontos), dist_pares(&lido.xyz.pontos));
                    for (u, v) in a.iter().zip(&b) {
                        assert!((u - v).abs() < 1e-4, "{formato:?}: {u} x {v}");
                    }
                }
            }
        }
    }

    #[test]
    fn modo_absoluto_acumula_posicao_e_rotacao() {
        let r = rota3d();
        let t = exportar(&r, Formato::Lra, Opcoes { modo: Modo::Absoluto, ..Default::default() }, "t").unwrap();
        let linhas: Vec<&str> = t.lines().filter(|l| l.chars().next().is_some_and(|c| c.is_ascii_digit())).collect();
        let col = |l: &str, i: usize| l.split(';').nth(i).unwrap().parse::<f64>().unwrap();
        // posição da 2ª dobra = reto1 + arco1 + reto2; rotação acumulada = 0 + 90
        let esperado = 120. + r.dobras[0].arco + 80.;
        assert!((col(linhas[1], 1) - esperado).abs() < 1e-5);
        assert!((col(linhas[1], 2) - 90.).abs() < 1e-5);
        assert!((col(linhas[2], 2) - (90. - 120.)).abs() < 1e-5);
        // o fim é o total desenvolvido
        let fim: f64 = t.lines().find(|l| l.starts_with("fim;")).unwrap()[4..].parse().unwrap();
        assert!((fim - r.desenvolvido()).abs() < 1e-5);
    }

    #[test]
    fn angulo_de_maquina_e_rotacao_invertida_no_arquivo() {
        let r = rota3d();
        let t = exportar(&r, Formato::Ybc, Opcoes { modo: Modo::Incremental, angulo_maquina: true, inverter_rotacao: true }, "t").unwrap();
        assert!(t.contains("formato;YBC") && t.contains("n;Y;B;C;raio;arco") && t.contains("angulo;maquina"));
        let l2 = t.lines().find(|l| l.starts_with("2;")).unwrap();
        let c: Vec<f64> = l2.split(';').skip(1).map(|s| s.parse().unwrap()).collect();
        assert!((c[1] + 90.).abs() < 1e-5, "rotação invertida: {c:?}");
        assert!((c[2] - 62.).abs() < 1e-5, "ângulo da máquina 60 + 2: {c:?}");
        // lido de volta, a rotação volta ao sentido da rota
        let lido = importar(&t).unwrap();
        assert!((lido.rota.dobras[1].rotacao - 90.).abs() < 1e-5);
        assert!(lido.opcoes.angulo_maquina && lido.opcoes.inverter_rotacao);
    }

    #[test]
    fn erros_claros() {
        // raio grande demais para a perna
        let x = RotaXyz { pontos: vec![[0., 0., 0.], [100., 0., 0.], [100., 100., 0.]], raios: vec![200.] };
        assert!(de_xyz(&x).unwrap_err().contains("tangentes"));
        // ponto de dobra sem raio no arquivo
        let t = "formato;XYZ\nn;X;Y;Z;raio\n0;0;0;0;\n1;100;0;0;\n2;100;100;0;\n";
        assert!(importar(t).unwrap_err().contains("raio"));
        // sem formato
        assert!(importar("n;L;R;A;raio\n").is_err());
        // número ruim
        let t = "formato;LRA\nn;L;R;A;raio\n1;abc;0;90;50\nfim;10\n";
        assert!(importar(t).unwrap_err().contains("abc"));
        // ângulo fora da faixa
        let r = RotaTubo { retos: vec![10., 10.], dobras: vec![DobraMaq { raio: 10., angulo: 190., ..Default::default() }] };
        assert!(para_xyz(&r).is_err());
    }

    #[test]
    fn pontos_alinhados_nao_viram_dobra() {
        let x = RotaXyz { pontos: vec![[0., 0., 0.], [50., 0., 0.], [100., 0., 0.]], raios: vec![30.] };
        let r = de_xyz(&x).unwrap();
        assert!(r.dobras.is_empty());
        assert!((r.retos[0] - 100.).abs() < 1e-9);
    }

    #[test]
    fn virgula_decimal_e_comentarios_sao_aceitos() {
        let t = "# gerado à mão\nformato;LRA\nmodo;incremental\nn;L;R;A;raio\n1;100,5;0;90;50,8\nfim;40,25\n";
        let l = importar(t).unwrap();
        assert!((l.rota.retos[0] - 100.5).abs() < 1e-9 && (l.rota.retos[1] - 40.25).abs() < 1e-9);
        assert!((l.rota.dobras[0].raio - 50.8).abs() < 1e-9);
    }
}
