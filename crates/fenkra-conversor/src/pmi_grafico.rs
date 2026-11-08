// SPDX-License-Identifier: LGPL-2.1-or-later
//! PMI gráfico para o STEP AP242 (B191): cada anotação desenhada em segmentos de reta no espaço do
//! modelo (linha de chamada, linha de cota com setas, quadro e texto em traços) e as capturas do PMI
//! como vistas salvas.
//!
//! O arquivo leva polilinhas, não fontes: por isso o texto sai numa fonte de traços própria, desenhada
//! numa grade de 4 x 6 (altura de letra maiúscula = 6). Minúsculas saem como maiúsculas e acentos caem;
//! o texto original vai também no nome da apresentação, em ASCII.

use fenkra_core::doc::NamedView;
use fenkra_core::pmi::{Pmi, PmiKind};
use fenkra_kernel::{ApresentacaoXde, VistaXde};

type P3 = [f64; 3];
type Seg = [P3; 2];

/// Altura padrão do texto, em mm (ISO 3098: 3,5 mm é a altura usual em anotação).
pub const ALTURA_TEXTO: f64 = 3.5;
/// Campo de visão vertical da câmera do Fenkra (rad), para o tamanho da janela da vista.
const FOV: f64 = 0.6;

fn add(a: P3, b: P3) -> P3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn sub(a: P3, b: P3) -> P3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn mul(a: P3, k: f64) -> P3 {
    [a[0] * k, a[1] * k, a[2] * k]
}
fn dot(a: P3, b: P3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: P3, b: P3) -> P3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn unit(a: P3) -> Option<P3> {
    let n = dot(a, a).sqrt();
    (n > 1e-12).then(|| mul(a, 1. / n))
}

/// Traços de um caractere na grade 4 x 6: polilinhas separadas por `;`, pontos "x,y".
fn glifo(c: char) -> Option<&'static str> {
    Some(match c {
        '0' | 'O' => "1,0 3,0 4,1 4,5 3,6 1,6 0,5 0,1 1,0",
        '1' => "1,5 2,6 2,0;1,0 3,0",
        '2' => "0,5 1,6 3,6 4,5 4,4 0,0 4,0",
        '3' => "0,5 1,6 3,6 4,5 4,4 3,3 1,3;3,3 4,2 4,1 3,0 1,0 0,1",
        '4' => "3,0 3,6 0,2 4,2",
        '5' => "4,6 0,6 0,3 3,3 4,2 4,1 3,0 0,0",
        '6' => "4,5 3,6 1,6 0,5 0,1 1,0 3,0 4,1 4,2 3,3 0,3",
        '7' => "0,6 4,6 1,0",
        '8' => "1,3 0,4 0,5 1,6 3,6 4,5 4,4 3,3 1,3 0,2 0,1 1,0 3,0 4,1 4,2 3,3",
        '9' => "4,3 1,3 0,4 0,5 1,6 3,6 4,5 4,1 3,0 1,0 0,1",
        'A' => "0,0 0,4 2,6 4,4 4,0;0,3 4,3",
        'B' => "0,0 0,6 3,6 4,5 4,4 3,3 0,3;3,3 4,2 4,1 3,0 0,0",
        'C' => "4,5 3,6 1,6 0,5 0,1 1,0 3,0 4,1",
        'D' => "0,0 0,6 2,6 4,4 4,2 2,0 0,0",
        'E' => "4,6 0,6 0,0 4,0;0,3 3,3",
        'F' => "4,6 0,6 0,0;0,3 3,3",
        'G' => "4,5 3,6 1,6 0,5 0,1 1,0 3,0 4,1 4,3 2,3",
        'H' => "0,0 0,6;4,0 4,6;0,3 4,3",
        'I' => "1,6 3,6;2,6 2,0;1,0 3,0",
        'J' => "4,6 4,1 3,0 1,0 0,1",
        'K' => "0,0 0,6;4,6 0,2;1,3 4,0",
        'L' => "0,6 0,0 4,0",
        'M' => "0,0 0,6 2,3 4,6 4,0",
        'N' => "0,0 0,6 4,0 4,6",
        'P' => "0,0 0,6 3,6 4,5 4,4 3,3 0,3",
        'Q' => "1,0 3,0 4,1 4,5 3,6 1,6 0,5 0,1 1,0;2,2 4,0",
        'R' => "0,0 0,6 3,6 4,5 4,4 3,3 0,3;2,3 4,0",
        'S' => "4,5 3,6 1,6 0,5 0,4 1,3 3,3 4,2 4,1 3,0 1,0 0,1",
        'T' => "0,6 4,6;2,6 2,0",
        'U' => "0,6 0,1 1,0 3,0 4,1 4,6",
        'V' => "0,6 2,0 4,6",
        'W' => "0,6 1,0 2,3 3,0 4,6",
        'X' => "0,0 4,6;0,6 4,0",
        'Y' => "0,6 2,3 4,6;2,3 2,0",
        'Z' => "0,6 4,6 0,0 4,0",
        '.' => "1.8,0 2.2,0 2.2,0.4 1.8,0.4 1.8,0",
        ',' => "2.2,0.6 2.2,0 1.6,-1",
        ':' => "2,1 2,1.4;2,4 2,4.4",
        '-' | '−' | '–' | '—' => "0.5,3 3.5,3",
        '+' => "0,3 4,3;2,1 2,5",
        '±' => "0,4 4,4;2,2 2,6;0,1 4,1",
        '=' => "0,2 4,2;0,4 4,4",
        '/' => "0,0 4,6",
        '\\' => "0,6 4,0",
        '(' => "3,6 1,4 1,2 3,0",
        ')' => "1,6 3,4 3,2 1,0",
        '[' => "3,6 1,6 1,0 3,0",
        ']' => "1,6 3,6 3,0 1,0",
        '|' => "2,-1 2,7",
        '%' => "0,0 4,6;0,5 0,6 1,6 1,5 0,5;3,0 3,1 4,1 4,0 3,0",
        '°' => "1.5,6 2.5,6 2.5,5 1.5,5 1.5,6",
        '\'' => "2,6 2,4.5",
        '"' => "1.5,6 1.5,4.5;2.5,6 2.5,4.5",
        '*' => "2,1 2,5;0.5,2 3.5,4;0.5,4 3.5,2",
        '<' => "4,5 0,3 4,1",
        '>' => "0,5 4,3 0,1",
        'Ø' | 'ø' | '⌀' => "1,1 3,1 3.8,2 3.8,4 3,5 1,5 0.2,4 0.2,2 1,1;0,0 4,6",
        '√' => "0,3 1,0 3,6 4,6",
        '⌒' => "0,4 1,5 3,5 4,4",
        '○' | '◯' => "1,1 3,1 4,2 4,4 3,5 1,5 0,4 0,2 1,1",
        '#' => "1,0 1,6;3,0 3,6;0,2 4,2;0,4 4,4",
        '?' => "0,5 1,6 3,6 4,5 4,4 2,3 2,2;2,0 2,0.4",
        _ => return None,
    })
}

/// Letra sem acento, na mesma caixa.
fn sem_acento(c: char) -> char {
    match c {
        'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
        'Á' | 'À' | 'Â' | 'Ã' | 'Ä' => 'A',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'É' | 'È' | 'Ê' | 'Ë' => 'E',
        'í' | 'ì' | 'î' | 'ï' => 'i',
        'Í' | 'Ì' | 'Î' | 'Ï' => 'I',
        'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
        'Ó' | 'Ò' | 'Ô' | 'Õ' | 'Ö' => 'O',
        'ú' | 'ù' | 'û' | 'ü' => 'u',
        'Ú' | 'Ù' | 'Û' | 'Ü' => 'U',
        'ç' => 'c',
        'Ç' => 'C',
        'ñ' => 'n',
        'Ñ' => 'N',
        outro => outro,
    }
}

/// Letra sem acento e em maiúscula: a fonte de traços só tem maiúsculas.
fn dobrar(c: char) -> char {
    let c = sem_acento(c);
    if c.is_ascii_lowercase() {
        c.to_ascii_uppercase()
    } else {
        c
    }
}

/// Letra circulada do Unicode (Ⓐ..Ⓩ, ⓐ..ⓩ): os modificadores de tolerância (Ⓜ, Ⓛ, Ⓟ...).
fn circulada(c: char) -> Option<char> {
    let n = c as u32;
    match n {
        0x24B6..=0x24CF => char::from_u32('A' as u32 + n - 0x24B6),
        0x24D0..=0x24E9 => char::from_u32('A' as u32 + n - 0x24D0),
        _ => None,
    }
}

/// Avanço de um caractere, em unidades da grade (largura 4 + 1 de espaço).
const AVANCO: f64 = 5.;

/// Largura de uma linha de texto em mm, na altura `h`.
pub fn largura_texto(s: &str, h: f64) -> f64 {
    let n = s.chars().count() as f64;
    (n * AVANCO - 1.).max(0.) * h / 6.
}

/// Texto em traços: `origem` é o canto inferior esquerdo da primeira linha; `ex` e `ey` são as direções
/// do texto no plano (unitárias); linhas seguintes descem 1,6 h. Caractere sem glifo vira uma caixa.
pub fn tracos_texto(s: &str, origem: P3, ex: P3, ey: P3, h: f64, out: &mut Vec<Seg>) {
    let k = h / 6.;
    let em = |x: f64, y: f64, base: P3| add(base, add(mul(ex, x * k), mul(ey, y * k)));
    for (li, linha) in s.lines().enumerate() {
        let base = add(origem, mul(ey, -1.6 * h * li as f64));
        for (ci, c) in linha.chars().enumerate() {
            let x0 = ci as f64 * AVANCO;
            let (traco, anel) = match circulada(c) {
                Some(l) => (glifo(l), true),
                None if c.is_whitespace() => continue,
                None => (glifo(dobrar(c)), false),
            };
            let traco = traco.unwrap_or("0,0 4,0 4,6 0,6 0,0");
            // letra circulada: a letra a 60% dentro de um anel de 12 lados
            let (esc, dx, dy) = if anel { (0.6, 0.8, 1.2) } else { (1., 0., 0.) };
            for poli in traco.split(';') {
                let pts: Vec<(f64, f64)> = poli
                    .split_whitespace()
                    .filter_map(|p| {
                        let (a, b) = p.split_once(',')?;
                        Some((a.parse::<f64>().ok()? * esc + dx, b.parse::<f64>().ok()? * esc + dy))
                    })
                    .collect();
                for w in pts.windows(2) {
                    out.push([em(x0 + w[0].0, w[0].1, base), em(x0 + w[1].0, w[1].1, base)]);
                }
            }
            if anel {
                let r = 3.;
                let pts: Vec<(f64, f64)> = (0..=12).map(|i| {
                    let t = i as f64 / 12. * std::f64::consts::TAU;
                    (2. + r * t.cos(), 3. + r * t.sin())
                }).collect();
                for w in pts.windows(2) {
                    out.push([em(x0 + w[0].0, w[0].1, base), em(x0 + w[1].0, w[1].1, base)]);
                }
            }
        }
    }
}

/// Nome da apresentação em ASCII (o nome vai como texto simples no arquivo).
pub fn nome_ascii(s: &str) -> String {
    let mut o = String::new();
    for c in s.chars() {
        match sem_acento(c) {
            c if c.is_ascii() && !c.is_ascii_control() => o.push(c),
            '\n' => o.push_str(" / "),
            'Ø' | 'ø' | '⌀' => o.push_str("DIA "),
            '±' => o.push_str("+/-"),
            '−' | '–' | '—' => o.push('-'),
            '°' => o.push_str(" deg"),
            '√' => o.push_str("RUG "),
            '⌒' => o.push_str("ARC "),
            '○' | '◯' => o.push_str("(O) "),
            c => {
                if let Some(l) = circulada(c) {
                    o.push('(');
                    o.push(l);
                    o.push(')');
                }
            }
        }
    }
    o.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Plano de leitura de uma câmera do Fenkra: normal para o olho e x para a direita da tela.
pub fn plano_da_camera(v: &NamedView) -> (P3, P3) {
    let (cp, sp) = (v.pitch.cos(), v.pitch.sin());
    let n = [cp * v.yaw.cos(), cp * v.yaw.sin(), sp];
    let cima = if v.pitch.abs() > 1.55 { mul([-v.yaw.cos(), -v.yaw.sin(), 0.], v.pitch.signum()) } else { [0., 0., 1.] };
    let x = unit(cross(cima, n)).unwrap_or([1., 0., 0.]);
    (n, x)
}

/// Câmera padrão do Fenkra (isométrica): o plano de leitura de quem não tem plano nem captura.
pub fn camera_padrao() -> NamedView {
    NamedView { name: String::new(), target: [0.; 3], yaw: -0.785, pitch: 0.6, dist: 300., ortho: false }
}

/// A captura do PMI como vista salva do STEP: olho, visada, cima e janela, como a câmera do Fenkra.
pub fn vista_da_captura(v: &NamedView, anotacoes: Vec<usize>) -> VistaXde {
    let (n, x) = plano_da_camera(v);
    let olho = add(v.target, mul(n, v.dist));
    let cima = cross(n, x);
    let altura = 2. * v.dist * (FOV / 2.).tan();
    VistaXde { nome: v.name.clone(), olho, direcao: mul(n, -1.), cima, ortogonal: v.ortho, zoom: v.dist, largura: altura * 16. / 9., altura, anotacoes }
}

/// Seta aberta na ponta `p`, apontando no sentido `d` (unitário), no plano de `ey`.
fn seta(p: P3, d: P3, lado: P3, h: f64, out: &mut Vec<Seg>) {
    let c = h * 0.9;
    let base = sub(p, mul(d, c));
    out.push([p, add(base, mul(lado, c * 0.3))]);
    out.push([p, add(base, mul(lado, -c * 0.3))]);
}

fn caixa(lo: P3, ex: P3, ey: P3, w: f64, hh: f64, out: &mut Vec<Seg>) {
    let a = lo;
    let b = add(lo, mul(ex, w));
    let c = add(b, mul(ey, hh));
    let d = add(lo, mul(ey, hh));
    out.extend([[a, b], [b, c], [c, d], [d, a]]);
}

/// Desenho da anotação em segmentos. `valor` é a medida (cotas); `plano_leitura` (normal, x) vale
/// quando a anotação não tem plano próprio. Sem âncora, ou sem texto nem linha (nota vazia), não há o que
/// desenhar.
pub fn apresentacao(p: &Pmi, valor: Option<f64>, plano_leitura: (P3, P3)) -> Option<ApresentacaoXde> {
    let a0 = *p.anchors.first()?;
    let (n, x) = p.plane.unwrap_or(plano_leitura);
    let n = unit(n)?;
    // x dentro do plano
    let ex = unit(sub(x, mul(n, dot(x, n)))).or_else(|| unit(cross([0., 0., 1.], n))).unwrap_or([1., 0., 0.]);
    let ey = cross(n, ex);
    let h = if p.plane.is_some() && p.style.size > 0. { p.style.size as f64 } else { ALTURA_TEXTO };
    let texto = p.display(valor);
    let t = add(a0, p.offset);
    let mut s: Vec<Seg> = vec![];
    let n_linhas = texto.lines().count().max(1) as f64;
    let largura = texto.lines().map(|l| largura_texto(l, h)).fold(0., f64::max);
    // o texto fica com a base em `t` (a posição do rótulo no Fenkra é o centro da primeira linha)
    let origem = sub(t, mul(ex, largura / 2.));
    let linha_de_chamada = |de: P3, s: &mut Vec<Seg>| {
        let ate = t;
        if let Some(d) = unit(sub(de, ate)) {
            s.push([ate, de]);
            seta(de, d, cross(n, d), h, s);
        }
    };
    match &p.kind {
        PmiKind::Linear | PmiKind::Gap { .. } | PmiKind::Ordinate { .. } if p.anchors.len() >= 2 => {
            let a1 = p.anchors[1];
            let a1 = if let PmiKind::Ordinate { axis } = p.kind {
                // ordenada: só a componente no eixo
                let mut q = a0;
                q[axis.min(2) as usize] = a1[axis.min(2) as usize];
                q
            } else {
                a1
            };
            // linha de cota paralela à medida, passando pelo texto; linhas de chamada até ela
            let d = unit(sub(a1, a0))?;
            let desloc = sub(sub(t, a0), mul(d, dot(sub(t, a0), d)));
            let (c0, c1) = (add(a0, desloc), add(a1, desloc));
            s.extend([[a0, c0], [a1, c1], [c0, c1]]);
            seta(c0, mul(d, -1.), cross(n, d), h, &mut s);
            seta(c1, d, cross(n, d), h, &mut s);
        }
        PmiKind::Angle if p.anchors.len() >= 3 => {
            s.push([p.anchors[0], p.anchors[1]]);
            s.push([p.anchors[0], p.anchors[2]]);
        }
        PmiKind::Radius | PmiKind::Diameter => {
            let de = p.anchors.get(1).copied().unwrap_or(a0);
            linha_de_chamada(de, &mut s);
        }
        PmiKind::Datum { .. } => {
            // triângulo de referência cheio (contorno) no pé da chamada
            if let Some(d) = unit(sub(a0, t)) {
                let l = cross(n, d);
                let base = sub(a0, mul(d, h));
                s.extend([[a0, add(base, mul(l, h * 0.6))], [add(base, mul(l, h * 0.6)), add(base, mul(l, -h * 0.6))], [add(base, mul(l, -h * 0.6)), a0], [base, t]]);
            }
        }
        _ => linha_de_chamada(a0, &mut s),
    }
    let pad = h * 0.4;
    match &p.kind {
        PmiKind::Gdt { ch, tol, datums, extra } => {
            // quadro de controle: símbolo numa célula quadrada e as células de texto ao lado
            let linhas = extra.linhas(*ch, tol, datums);
            let alt = h * 2.;
            let mut topo = add(t, mul(ey, alt));
            for l in &linhas {
                let lo = sub(topo, mul(ey, alt));
                caixa(lo, ex, ey, alt, alt, &mut s);
                let c = add(lo, add(mul(ex, alt / 2.), mul(ey, alt / 2.)));
                for tr in l.sym.unwrap_or(*ch).tracado() {
                    let pts: Vec<P3> = tr.pts.iter().map(|q| add(c, add(mul(ex, q[0] * alt * 0.62), mul(ey, q[1] * alt * 0.62)))).collect();
                    for w in pts.windows(2) {
                        s.push([w[0], w[1]]);
                    }
                    if tr.fechado && pts.len() > 2 {
                        s.push([pts[pts.len() - 1], pts[0]]);
                    }
                }
                let mut xa = alt;
                for cel in &l.celulas {
                    let txt = fenkra_core::gdt::celula_texto(cel);
                    let w = largura_texto(&txt, h) + 2. * pad;
                    caixa(add(lo, mul(ex, xa)), ex, ey, w, alt, &mut s);
                    tracos_texto(&txt, add(lo, add(mul(ex, xa + pad), mul(ey, (alt - h) / 2.))), ex, ey, h, &mut s);
                    xa += w;
                }
                topo = lo;
            }
            let chamada = extra.chamada();
            if !chamada.is_empty() {
                tracos_texto(&chamada, add(t, mul(ey, alt + h * 0.5)), ex, ey, h, &mut s);
            }
        }
        _ => {
            tracos_texto(&texto, origem, ex, ey, h, &mut s);
            let lo = sub(origem, add(mul(ex, pad), mul(ey, pad + 1.6 * h * (n_linhas - 1.))));
            let (w, hh) = (largura + 2. * pad, h + 2. * pad + 1.6 * h * (n_linhas - 1.));
            match &p.kind {
                PmiKind::Datum { .. } => caixa(lo, ex, ey, w, hh, &mut s),
                PmiKind::Balloon { .. } => {
                    let c = add(lo, add(mul(ex, w / 2.), mul(ey, hh / 2.)));
                    let r = (w.max(hh)) / 2. + pad;
                    let pts: Vec<P3> = (0..=24).map(|i| {
                        let a = i as f64 / 24. * std::f64::consts::TAU;
                        add(c, add(mul(ex, r * a.cos()), mul(ey, r * a.sin())))
                    }).collect();
                    for w in pts.windows(2) {
                        s.push([w[0], w[1]]);
                    }
                }
                _ => {}
            }
        }
    }
    if s.is_empty() {
        return None;
    }
    Some(ApresentacaoXde { nome: nome_ascii(&texto), segmentos: s, plano: Some((t, n, ex)), texto_em: Some(t) })
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn todo_caractere_das_cotas_tem_glifo() {
        // números, sinais e as letras de uso em cotas, rugosidade e notas
        for c in "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ.,:-+±=/()[]|%°Ø√⌒○−".chars() {
            assert!(glifo(dobrar(c)).is_some(), "sem glifo: {c}");
        }
        assert_eq!(dobrar('ã'), 'A');
        assert_eq!(circulada('Ⓜ'), Some('M'));
    }

    #[test]
    fn texto_em_tracos_fica_na_caixa_da_altura() {
        let mut s = vec![];
        tracos_texto("R12,5", [0.; 3], [1., 0., 0.], [0., 1., 0.], 6., &mut s);
        assert!(!s.is_empty());
        let w = largura_texto("R12,5", 6.);
        for seg in &s {
            for q in seg {
                assert!(q[0] >= -1e-9 && q[0] <= w + 1e-9, "{q:?} fora de 0..{w}");
                assert!(q[1] >= -1.01 && q[1] <= 6. + 1e-9, "{q:?}");
                assert!(q[2].abs() < 1e-12);
            }
        }
    }

    #[test]
    fn nome_vira_ascii() {
        assert_eq!(nome_ascii("Ø12 ±0,1"), "DIA 12 +/-0,1");
        assert_eq!(nome_ascii("√ Ra 1,6 (retífica)"), "RUG Ra 1,6 (retifica)");
    }

    #[test]
    fn camera_padrao_tem_x_horizontal() {
        let (n, x) = plano_da_camera(&camera_padrao());
        assert!(dot(n, x).abs() < 1e-9);
        assert!(x[2].abs() < 1e-9, "x para a direita da tela fica no plano horizontal");
        let v = vista_da_captura(&camera_padrao(), vec![]);
        assert!((dot(v.direcao, v.cima)).abs() < 1e-9);
        assert!(v.cima[2] > 0.);
    }
}
