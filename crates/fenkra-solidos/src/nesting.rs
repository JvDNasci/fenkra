// SPDX-License-Identifier: LGPL-2.1-or-later
//! Nesting de chapa (B116): arranja as planificadas em chapas comerciais, minimizando sobra, e
//! grava um DXF por chapa mais o relatório de aproveitamento.
//!
//! Como funciona. Cada peça vem da planificada (`planificada::Planificada`, B86): o contorno exato
//! vira um polígono (arcos amostrados a cada 3 graus) e os furos ficam só para o desenho, porque a
//! colisão trata a peça como preenchida (o mesmo caminho dos motores de nesting abertos, que
//! recusam furo interno). Para cada cópia, da maior para a menor, o motor testa as rotações
//! permitidas e posições candidatas nos cantos das peças já colocadas (heurística canto inferior
//! esquerdo); a melhor posição ainda desliza para baixo e para a esquerda em passos que encolhem
//! pela metade, o que encaixa a peça em reentrâncias da vizinha. Duas peças ficam separadas pelo
//! `espaco` (distância mínima entre contornos, a folga de corte). Se a cópia não cabe em nenhuma
//! chapa aberta, abre outra; se não cabe nem na chapa vazia, vai para `nao_coube`.
//!
//! O "melhor esforço" repete a heurística com ordens diferentes das peças (área, maior lado e
//! embaralhamentos de semente fixa) e guarda o melhor layout; pode ser cancelado por uma bandeira
//! atômica e devolve o melhor até ali. A colisão é exata para polígonos (distância entre segmentos),
//! mas a posição é gulosa: não há garantia de ótimo.
//!
//! Limites conhecidos: sem encaixe de peças pequenas dentro dos furos das grandes, sem sentido de
//! fio por material (só a flag `girar` por peça) e sem corte comum entre peças.

use crate::planificada::{self, Entidade, Planificada};
use std::sync::atomic::{AtomicBool, Ordering};

pub type P = [f64; 2];

/// Chapa comercial em que as peças são dispostas.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChapaComercial {
    pub largura: f64,
    pub altura: f64,
    /// faixa da borda que fica sem peça (pinça, sobra de refilo)
    pub margem: f64,
}

impl Default for ChapaComercial {
    fn default() -> Self {
        ChapaComercial { largura: 3000., altura: 1250., margem: 5. }
    }
}

/// Parâmetros do nesting.
#[derive(Clone, Debug, PartialEq)]
pub struct Opcoes {
    pub chapa: ChapaComercial,
    /// distância mínima entre contornos de duas peças (mm)
    pub espaco: f64,
    /// ângulos permitidos (graus); vazio vale `[0]`
    pub rotacoes: Vec<f64>,
    /// tentativas do melhor esforço (1 = só a ordem por área)
    pub esforco: usize,
}

impl Default for Opcoes {
    fn default() -> Self {
        Opcoes { chapa: ChapaComercial::default(), espaco: 3., rotacoes: vec![0., 90., 180., 270.], esforco: 1 }
    }
}

/// Peça a dispor: a planificada de um documento mais a quantidade pedida.
#[derive(Clone, Debug, PartialEq)]
pub struct PecaNesting {
    pub nome: String,
    pub quantidade: usize,
    /// `false` fixa a peça na orientação original (sentido do fio)
    pub girar: bool,
    pub plana: Planificada,
    /// contorno externo (maior laço) e furos, em polígono; o primeiro serve à colisão
    pub(crate) contorno: Vec<P>,
    area_contorno: f64,
    area_liquida: f64,
}

impl PecaNesting {
    pub fn da_planificada(nome: &str, plana: &Planificada, quantidade: usize) -> Result<PecaNesting, String> {
        let lacos = encadeia(&plana.contorno);
        let contorno = lacos.into_iter().filter(|l| l.len() >= 3).max_by(|a, b| area(a).abs().total_cmp(&area(b).abs())).ok_or_else(|| format!("'{nome}' não tem contorno fechado na planificada"))?;
        let a_contorno = area(&contorno).abs();
        let a_furos: f64 = encadeia(&plana.furos).iter().filter(|l| l.len() >= 3).map(|l| area(l).abs()).sum();
        Ok(PecaNesting { nome: nome.to_string(), quantidade, girar: true, plana: plana.clone(), contorno, area_contorno: a_contorno, area_liquida: (a_contorno - a_furos).max(0.) })
    }

    /// Contorno externo da cópia já na chapa (polígono), para desenhar a prévia.
    pub fn poligono_na_chapa(&self, k: &Colocacao) -> Vec<P> {
        self.contorno.iter().map(|q| {
            let r = rota(*q, k.rot);
            [r[0] + k.t[0], r[1] + k.t[1]]
        }).collect()
    }

    pub fn area_contorno(&self) -> f64 {
        self.area_contorno
    }
    pub fn area_liquida(&self) -> f64 {
        self.area_liquida
    }
}

/// Uma cópia colocada: `p' = rot(p) + t` leva a planificada à chapa.
#[derive(Clone, Debug, PartialEq)]
pub struct Colocacao {
    pub peca: usize,
    pub copia: usize,
    /// graus, anti-horário, em torno da origem da planificada
    pub rot: f64,
    pub t: P,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ChapaLayout {
    pub colocacoes: Vec<Colocacao>,
    /// soma das áreas dos contornos colocados (mm²)
    pub area_contornos: f64,
    /// a mesma soma com os furos descontados
    pub area_liquida: f64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Layout {
    pub chapas: Vec<ChapaLayout>,
    /// (peça, cópia) que não coube nem numa chapa vazia
    pub nao_coube: Vec<(usize, usize)>,
}

impl Layout {
    /// Aproveitamento da chapa `i`: área dos contornos sobre a área da chapa.
    pub fn aproveitamento(&self, i: usize, op: &Opcoes) -> f64 {
        self.chapas[i].area_contornos / (op.chapa.largura * op.chapa.altura)
    }
    pub fn aproveitamento_total(&self, op: &Opcoes) -> f64 {
        let n = self.chapas.len();
        if n == 0 {
            return 0.;
        }
        self.chapas.iter().map(|c| c.area_contornos).sum::<f64>() / (n as f64 * op.chapa.largura * op.chapa.altura)
    }
}

// ---------- geometria ----------

pub(crate) fn area(p: &[P]) -> f64 {
    let n = p.len();
    (0..n).map(|i| p[i][0] * p[(i + 1) % n][1] - p[(i + 1) % n][0] * p[i][1]).sum::<f64>() / 2.
}

fn rota(p: P, graus: f64) -> P {
    let (s, c) = graus.to_radians().sin_cos();
    [p[0] * c - p[1] * s, p[0] * s + p[1] * c]
}

pub(crate) fn caixa(p: &[P]) -> (P, P) {
    let (mut a, mut b) = ([f64::MAX; 2], [f64::MIN; 2]);
    for q in p {
        for k in 0..2 {
            a[k] = a[k].min(q[k]);
            b[k] = b[k].max(q[k]);
        }
    }
    (a, b)
}

fn pontos_entidade(e: &Entidade) -> Vec<P> {
    match e {
        Entidade::Linha(a, b) => vec![*a, *b],
        Entidade::Circulo { centro, raio } => (0..120).map(|i| {
            let a = (i as f64 * 3.).to_radians();
            [centro[0] + raio * a.cos(), centro[1] + raio * a.sin()]
        }).collect(),
        Entidade::Arco { centro, raio, ini, fim } => {
            let mut v = *fim;
            while v <= *ini {
                v += 360.;
            }
            let n = (((v - ini) / 3.).ceil() as usize).max(2);
            (0..=n).map(|i| {
                let a = (ini + (v - ini) * i as f64 / n as f64).to_radians();
                [centro[0] + raio * a.cos(), centro[1] + raio * a.sin()]
            }).collect()
        }
        Entidade::Polilinha(p, _) => p.clone(),
    }
}

/// Junta as entidades em laços pelos pontos finais iguais (tolerância 1e-3 mm). Um círculo é um
/// laço sozinho; entidades que não fecham laço são descartadas.
pub(crate) fn encadeia(ents: &[Entidade]) -> Vec<Vec<P>> {
    const TOL: f64 = 1e-3;
    let perto = |a: P, b: P| (a[0] - b[0]).hypot(a[1] - b[1]) < TOL;
    let mut lacos = vec![];
    let mut restantes: Vec<Vec<P>> = vec![];
    for e in ents {
        let p = pontos_entidade(e);
        let fechada = matches!(e, Entidade::Circulo { .. }) || matches!(e, Entidade::Polilinha(_, true));
        if fechada {
            lacos.push(p);
        } else if p.len() >= 2 {
            restantes.push(p);
        }
    }
    while let Some(mut atual) = restantes.pop() {
        loop {
            if atual.len() > 2 && perto(atual[0], *atual.last().unwrap()) {
                atual.pop();
                lacos.push(atual);
                break;
            }
            let fim = *atual.last().unwrap();
            let Some(i) = restantes.iter().position(|r| perto(r[0], fim) || perto(*r.last().unwrap(), fim)) else { break };
            let mut r = restantes.swap_remove(i);
            if !perto(r[0], fim) {
                r.reverse();
            }
            atual.extend(r.into_iter().skip(1));
        }
    }
    lacos
}

fn dist_ponto_seg(p: P, a: P, b: P) -> f64 {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let l2 = dx * dx + dy * dy;
    let t = if l2 < 1e-18 { 0. } else { (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / l2).clamp(0., 1.) };
    (p[0] - a[0] - t * dx).hypot(p[1] - a[1] - t * dy)
}

fn cruza(a: P, b: P, c: P, d: P) -> bool {
    let o = |p: P, q: P, r: P| (q[0] - p[0]) * (r[1] - p[1]) - (q[1] - p[1]) * (r[0] - p[0]);
    let (d1, d2, d3, d4) = (o(c, d, a), o(c, d, b), o(a, b, c), o(a, b, d));
    ((d1 > 1e-12 && d2 < -1e-12) || (d1 < -1e-12 && d2 > 1e-12)) && ((d3 > 1e-12 && d4 < -1e-12) || (d3 < -1e-12 && d4 > 1e-12))
}

fn dist_seg_seg(a: P, b: P, c: P, d: P) -> f64 {
    if cruza(a, b, c, d) {
        return 0.;
    }
    dist_ponto_seg(a, c, d).min(dist_ponto_seg(b, c, d)).min(dist_ponto_seg(c, a, b)).min(dist_ponto_seg(d, a, b))
}

fn dentro(poly: &[P], p: P) -> bool {
    let n = poly.len();
    let mut d = false;
    let mut j = n - 1;
    for i in 0..n {
        if (poly[i][1] > p[1]) != (poly[j][1] > p[1]) && p[0] < (poly[j][0] - poly[i][0]) * (p[1] - poly[i][1]) / (poly[j][1] - poly[i][1]) + poly[i][0] {
            d = !d;
        }
        j = i;
    }
    d
}

/// Polígono já posicionado na chapa, com a caixa.
struct Posto {
    pts: Vec<P>,
    min: P,
    max: P,
}

fn colide(a: &Posto, b: &Posto, folga: f64) -> bool {
    if a.max[0] + folga <= b.min[0] || b.max[0] + folga <= a.min[0] || a.max[1] + folga <= b.min[1] || b.max[1] + folga <= a.min[1] {
        return false;
    }
    let (na, nb) = (a.pts.len(), b.pts.len());
    let lim = folga - 1e-9;
    for i in 0..na {
        let (p, q) = (a.pts[i], a.pts[(i + 1) % na]);
        for j in 0..nb {
            let (r, s) = (b.pts[j], b.pts[(j + 1) % nb]);
            if dist_seg_seg(p, q, r, s) < lim {
                return true;
            }
        }
    }
    dentro(&b.pts, a.pts[0]) || dentro(&a.pts, b.pts[0])
}

/// Folga efetiva: o `espaco` pedido, com piso de 1 micrômetro para que peças que apenas se
/// encostam (ou se sobrepõem borda sobre borda) não passem como livres.
pub const FOLGA_MINIMA: f64 = 1e-3;

fn folga(op: &Opcoes) -> f64 {
    op.espaco.max(FOLGA_MINIMA)
}

fn desloca(pts: &[P], t: P) -> Posto {
    let pts: Vec<P> = pts.iter().map(|p| [p[0] + t[0], p[1] + t[1]]).collect();
    let (min, max) = caixa(&pts);
    Posto { pts, min, max }
}

/// Peça girada com a caixa na origem.
struct Girada {
    rot: f64,
    pts: Vec<P>,
    /// translação que leva a planificada à caixa na origem
    base: P,
    w: f64,
    h: f64,
}

fn gira_peca(p: &PecaNesting, rot: f64) -> Girada {
    let pts: Vec<P> = p.contorno.iter().map(|q| rota(*q, rot)).collect();
    let (mn, mx) = caixa(&pts);
    Girada { rot, pts: pts.iter().map(|q| [q[0] - mn[0], q[1] - mn[1]]).collect(), base: [-mn[0], -mn[1]], w: mx[0] - mn[0], h: mx[1] - mn[1] }
}

struct Chapa {
    postos: Vec<Posto>,
    lay: ChapaLayout,
}

fn livre(g: &Girada, pos: P, chapa: &Chapa, op: &Opcoes) -> bool {
    let c = &op.chapa;
    if pos[0] < c.margem - 1e-9 || pos[1] < c.margem - 1e-9 || pos[0] + g.w > c.largura - c.margem + 1e-9 || pos[1] + g.h > c.altura - c.margem + 1e-9 {
        return false;
    }
    let novo = desloca(&g.pts, pos);
    !chapa.postos.iter().any(|o| colide(&novo, o, folga(op)))
}

/// Desliza a peça para baixo e para a esquerda em passos que caem pela metade.
fn compacta(g: &Girada, mut pos: P, chapa: &Chapa, op: &Opcoes) -> P {
    for _ in 0..2 {
        for eixo in [1usize, 0] {
            let mut passo = (op.chapa.largura.max(op.chapa.altura)) / 4.;
            while passo > 0.2 {
                loop {
                    let mut q = pos;
                    q[eixo] -= passo;
                    if livre(g, q, chapa, op) {
                        pos = q;
                    } else {
                        break;
                    }
                }
                passo /= 2.;
            }
        }
    }
    pos
}

/// Melhor posição da peça numa chapa: (pos, rotação escolhida, nota). Nota menor é melhor.
fn melhor_posicao(p: &PecaNesting, chapa: &Chapa, op: &Opcoes) -> Option<(P, usize, f64, Girada)> {
    let c = &op.chapa;
    let mut melhor: Option<(P, usize, f64, Girada)> = None;
    let rots: Vec<f64> = if p.girar && !op.rotacoes.is_empty() { op.rotacoes.clone() } else { vec![op.rotacoes.first().copied().filter(|_| !p.girar).unwrap_or(0.)] };
    for (ri, rot) in rots.iter().enumerate() {
        let g = gira_peca(p, *rot);
        let mut xs = vec![c.margem];
        let mut ys = vec![c.margem];
        for o in &chapa.postos {
            xs.push(o.max[0] + folga(op));
            ys.push(o.max[1] + folga(op));
            xs.push(o.min[0] - folga(op) - g.w);
            ys.push(o.min[1] - folga(op) - g.h);
        }
        let mut cand: Vec<P> = vec![];
        // vértices das peças já colocadas: a caixa da nova encosta neles pelos quatro cantos, o que
        // abre as reentrâncias (L, U) para a vizinha
        for o in &chapa.postos {
            let passo = (o.pts.len() / 48).max(1);
            for v in o.pts.iter().step_by(passo) {
                let f = folga(op);
                for (dx, dy) in [(0., 0.), (f, f), (f, 0.), (0., f)] {
                    cand.push([v[0] + dx, v[1] + dy]);
                    cand.push([v[0] - f - g.w + dx, v[1] + dy]);
                    cand.push([v[0] + dx, v[1] - f - g.h + dy]);
                    cand.push([v[0] - f - g.w + dx, v[1] - f - g.h + dy]);
                }
            }
        }
        for y in &ys {
            for x in &xs {
                if *x >= c.margem - 1e-9 && *y >= c.margem - 1e-9 {
                    cand.push([*x, *y]);
                }
            }
        }
        cand.sort_by(|a, b| a[1].total_cmp(&b[1]).then(a[0].total_cmp(&b[0])));
        cand.dedup();
        let Some(pos) = cand.into_iter().find(|q| livre(&g, *q, chapa, op)) else { continue };
        let pos = compacta(&g, pos, chapa, op);
        // nota: a quina superior direita da caixa, ponderando o eixo longo da chapa
        let nota = if c.largura >= c.altura { (pos[0] + g.w) * 1.0 + (pos[1] + g.h) * 0.01 } else { (pos[1] + g.h) + (pos[0] + g.w) * 0.01 };
        if melhor.as_ref().is_none_or(|m| nota < m.2 - 1e-9) {
            melhor = Some((pos, ri, nota, g));
        }
    }
    melhor
}

fn roda_ordem(pecas: &[PecaNesting], ordem: &[(usize, usize)], op: &Opcoes) -> Layout {
    let mut chapas: Vec<Chapa> = vec![];
    let mut nao_coube = vec![];
    for &(i, copia) in ordem {
        let p = &pecas[i];
        let mut colocou = false;
        for ch in chapas.iter_mut() {
            if let Some((pos, _, _, g)) = melhor_posicao(p, ch, op) {
                poe(ch, p, i, copia, &g, pos);
                colocou = true;
                break;
            }
        }
        if colocou {
            continue;
        }
        let mut nova = Chapa { postos: vec![], lay: ChapaLayout::default() };
        if let Some((pos, _, _, g)) = melhor_posicao(p, &nova, op) {
            poe(&mut nova, p, i, copia, &g, pos);
            chapas.push(nova);
        } else {
            nao_coube.push((i, copia));
        }
    }
    Layout { chapas: chapas.into_iter().map(|c| c.lay).collect(), nao_coube }
}

fn poe(ch: &mut Chapa, p: &PecaNesting, i: usize, copia: usize, g: &Girada, pos: P) {
    ch.postos.push(desloca(&g.pts, pos));
    ch.lay.area_contornos += p.area_contorno;
    ch.lay.area_liquida += p.area_liquida;
    ch.lay.colocacoes.push(Colocacao { peca: i, copia, rot: g.rot, t: [g.base[0] + pos[0], g.base[1] + pos[1]] });
}

fn nota_layout(l: &Layout, pecas: &[PecaNesting]) -> (usize, usize, f64) {
    // menos peças fora, menos chapas e, no empate, a última chapa o mais vazia possível
    let ultima = l.chapas.last().map(|c| c.colocacoes.iter().map(|k| pecas[k.peca].area_contorno).sum::<f64>()).unwrap_or(0.);
    (l.nao_coube.len(), l.chapas.len(), -ultima)
}

/// Roda o nesting. `cancelar` interrompe o melhor esforço entre tentativas e devolve o melhor
/// layout até ali (a primeira tentativa sempre roda).
pub fn dispor(pecas: &[PecaNesting], op: &Opcoes, cancelar: Option<&AtomicBool>) -> Layout {
    let mut copias: Vec<(usize, usize)> = pecas.iter().enumerate().flat_map(|(i, p)| (0..p.quantidade).map(move |c| (i, c))).collect();
    copias.sort_by(|a, b| pecas[b.0].area_contorno.total_cmp(&pecas[a.0].area_contorno).then(a.cmp(b)));
    let mut melhor = roda_ordem(pecas, &copias, op);
    let mut semente = 0x9e37_79b9_7f4a_7c15u64;
    for k in 1..op.esforco.max(1) {
        if cancelar.is_some_and(|c| c.load(Ordering::Relaxed)) {
            break;
        }
        let mut ordem = copias.clone();
        if k == 1 {
            let lado = |i: usize| {
                let (a, b) = caixa(&pecas[i].contorno);
                (b[0] - a[0]).max(b[1] - a[1])
            };
            ordem.sort_by(|a, b| lado(b.0).total_cmp(&lado(a.0)).then(a.cmp(b)));
        } else {
            for i in (1..ordem.len()).rev() {
                semente = semente.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                ordem.swap(i, (semente >> 33) as usize % (i + 1));
            }
        }
        let l = roda_ordem(pecas, &ordem, op);
        if nota_layout(&l, pecas) < nota_layout(&melhor, pecas) {
            melhor = l;
        }
    }
    melhor
}

// ---------- saída ----------

fn gira_ent(e: &Entidade, rot: f64, t: P) -> Entidade {
    let f = |p: P| {
        let q = rota(p, rot);
        [q[0] + t[0], q[1] + t[1]]
    };
    match e {
        Entidade::Linha(a, b) => Entidade::Linha(f(*a), f(*b)),
        Entidade::Circulo { centro, raio } => Entidade::Circulo { centro: f(*centro), raio: *raio },
        Entidade::Arco { centro, raio, ini, fim } => Entidade::Arco { centro: f(*centro), raio: *raio, ini: ini + rot, fim: fim + rot },
        Entidade::Polilinha(p, c) => Entidade::Polilinha(p.iter().map(|q| f(*q)).collect(), *c),
    }
}

/// Planificada da cópia já na posição da chapa (contorno, furos, ferramenta e dobras).
pub fn peca_na_chapa(p: &PecaNesting, k: &Colocacao) -> Planificada {
    let mut s = p.plana.clone();
    let ent = |l: &[Entidade]| l.iter().map(|e| gira_ent(e, k.rot, k.t)).collect::<Vec<_>>();
    s.contorno = ent(&s.contorno);
    s.furos = ent(&s.furos);
    s.ferramenta = ent(&s.ferramenta);
    for d in s.dobras.iter_mut() {
        let f = |q: P| {
            let r = rota(q, k.rot);
            [r[0] + k.t[0], r[1] + k.t[1]]
        };
        d.a = f(d.a);
        d.b = f(d.b);
    }
    s
}

/// DXF R2000 de uma chapa do layout: o retângulo da chapa em `CHAPA`, cada peça nas camadas fixas
/// da planificada (`CONTORNO`, `FUROS`, `DOBRA_*`, `FERRAMENTA`) e o nome em `ROTULOS`.
pub fn dxf_chapa(pecas: &[PecaNesting], layout: &Layout, indice: usize, op: &Opcoes) -> String {
    use fenkra_dxf::{Anexo, Camada, Desenho, Entidade as D, Item, TipoLinha, Unidade, Vertice};
    let conv = |e: &Entidade| match e {
        Entidade::Linha(a, b) => D::Linha(*a, *b),
        Entidade::Circulo { centro, raio } => D::Circulo { centro: *centro, raio: *raio },
        Entidade::Arco { centro, raio, ini, fim } => D::Arco { centro: *centro, raio: *raio, ini: *ini, fim: *fim },
        Entidade::Polilinha(p, f) => D::Polilinha { vertices: p.iter().map(|q| Vertice { p: *q, bojo: 0. }).collect(), fechada: *f },
    };
    let mut d = Desenho {
        unidade: Unidade::Milimetro,
        tipos_linha: vec![TipoLinha { nome: "DASHED".into(), padrao: vec![6., -3.] }],
        camadas: vec![
            Camada::nova(CAMADA_CHAPA, 8, "Continuous"),
            Camada::nova(planificada::CAMADA_CONTORNO, 7, "Continuous"),
            Camada::nova(planificada::CAMADA_FUROS, 3, "Continuous"),
            Camada::nova(planificada::CAMADA_DOBRA_CIMA, 1, "DASHED"),
            Camada::nova(planificada::CAMADA_DOBRA_BAIXO, 5, "DASHED"),
            Camada::nova(planificada::CAMADA_FERRAMENTA, 6, "Continuous"),
            Camada::nova(planificada::CAMADA_ROTULOS, 2, "Continuous"),
        ],
        ..Default::default()
    };
    let (w, h) = (op.chapa.largura, op.chapa.altura);
    d.itens.push(Item::novo(CAMADA_CHAPA, D::Polilinha { vertices: [[0., 0.], [w, 0.], [w, h], [0., h]].iter().map(|p| Vertice { p: *p, bojo: 0. }).collect(), fechada: true }));
    let Some(ch) = layout.chapas.get(indice) else { return fenkra_dxf::gravar(&d) };
    for k in &ch.colocacoes {
        let p = &pecas[k.peca];
        let s = peca_na_chapa(p, k);
        for (camada, lista) in [(planificada::CAMADA_CONTORNO, &s.contorno), (planificada::CAMADA_FUROS, &s.furos), (planificada::CAMADA_FERRAMENTA, &s.ferramenta)] {
            d.itens.extend(lista.iter().map(|e| Item::novo(camada, conv(e))));
        }
        for b in &s.dobras {
            let camada = if b.cima { planificada::CAMADA_DOBRA_CIMA } else { planificada::CAMADA_DOBRA_BAIXO };
            d.itens.push(Item::novo(camada, D::Linha(b.a, b.b)));
        }
        let pts: Vec<P> = s.contorno.iter().flat_map(pontos_entidade).collect();
        let (mn, mx) = caixa(&pts);
        let texto = format!("{} #{}", p.nome, k.copia + 1);
        d.itens.push(Item::novo(planificada::CAMADA_ROTULOS, D::Texto { pos: [(mn[0] + mx[0]) / 2., (mn[1] + mx[1]) / 2.], altura: 5., angulo: 0., anexo: Anexo::Centro, texto }));
    }
    fenkra_dxf::gravar(&d)
}

/// Camada do retângulo da chapa comercial nos DXF do nesting.
pub const CAMADA_CHAPA: &str = "CHAPA";

/// Relatório CSV: uma linha por chapa (peças, áreas e aproveitamento em %) e o total.
pub fn relatorio_csv(pecas: &[PecaNesting], layout: &Layout, op: &Opcoes) -> String {
    let mut s = String::from("chapa;pecas;area_chapa_mm2;area_contornos_mm2;area_liquida_mm2;aproveitamento_pct;aproveitamento_liquido_pct\n");
    let a_chapa = op.chapa.largura * op.chapa.altura;
    for (i, c) in layout.chapas.iter().enumerate() {
        s += &format!("{};{};{:.1};{:.1};{:.1};{:.2};{:.2}\n", i + 1, c.colocacoes.len(), a_chapa, c.area_contornos, c.area_liquida, 100. * c.area_contornos / a_chapa, 100. * c.area_liquida / a_chapa);
    }
    let n = layout.chapas.len() as f64;
    if n > 0. {
        let (ac, al): (f64, f64) = (layout.chapas.iter().map(|c| c.area_contornos).sum(), layout.chapas.iter().map(|c| c.area_liquida).sum());
        s += &format!("total;{};{:.1};{:.1};{:.1};{:.2};{:.2}\n", layout.chapas.iter().map(|c| c.colocacoes.len()).sum::<usize>(), n * a_chapa, ac, al, 100. * ac / (n * a_chapa), 100. * al / (n * a_chapa));
    }
    for (i, c) in &layout.nao_coube {
        s += &format!("nao_coube;{} #{}\n", pecas[*i].nome, c + 1);
    }
    s
}

/// Grava um DXF por chapa (`<prefixo>_chapa<N>.dxf`) e o relatório (`<prefixo>_relatorio.csv`) em
/// `pasta`. Devolve os caminhos gravados.
pub fn exportar(pecas: &[PecaNesting], layout: &Layout, op: &Opcoes, pasta: &std::path::Path, prefixo: &str) -> Result<Vec<std::path::PathBuf>, String> {
    std::fs::create_dir_all(pasta).map_err(|e| format!("não foi possível criar {}: {e}", pasta.display()))?;
    let mut out = vec![];
    for i in 0..layout.chapas.len() {
        let c = pasta.join(format!("{prefixo}_chapa{}.dxf", i + 1));
        std::fs::write(&c, dxf_chapa(pecas, layout, i, op)).map_err(|e| format!("não foi possível gravar {}: {e}", c.display()))?;
        out.push(c);
    }
    let r = pasta.join(format!("{prefixo}_relatorio.csv"));
    std::fs::write(&r, relatorio_csv(pecas, layout, op)).map_err(|e| format!("não foi possível gravar {}: {e}", r.display()))?;
    out.push(r);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planificada::{Propriedades};

    pub fn retangulo(w: f64, h: f64, furo: Option<f64>) -> Planificada {
        let linhas = vec![Entidade::Linha([0., 0.], [w, 0.]), Entidade::Linha([w, 0.], [w, h]), Entidade::Linha([w, h], [0., h]), Entidade::Linha([0., h], [0., 0.])];
        Planificada {
            espessura: 2.,
            material: String::new(),
            contorno: linhas,
            furos: furo.map(|r| vec![Entidade::Circulo { centro: [w / 2., h / 2.], raio: r }]).unwrap_or_default(),
            ferramenta: vec![],
            dobras: vec![],
            propriedades: Propriedades { largura: w, altura: h, area: w * h, volume: 0., massa: None, dobras: 0 },
            sobreposicoes: vec![],
            ausentes: vec![],
        }
    }

    fn op(l: f64, a: f64) -> Opcoes {
        Opcoes { chapa: ChapaComercial { largura: l, altura: a, margem: 0. }, espaco: 0., rotacoes: vec![0., 90.], esforco: 1 }
    }

    #[test]
    fn quatro_quadrados_enchem_uma_chapa_sem_se_tocar_por_dentro() {
        let p = PecaNesting::da_planificada("q", &retangulo(50., 50., None), 4).unwrap();
        let o = op(100.003, 100.003);
        let l = dispor(&[p.clone()], &o, None);
        assert_eq!(l.chapas.len(), 1);
        assert!(l.nao_coube.is_empty());
        assert!(l.aproveitamento(0, &o) > 0.999);
        // nenhum par colide (folga 0 permite encostar)
        let postos: Vec<Posto> = l.chapas[0].colocacoes.iter().map(|k| {
            let g = gira_peca(&p, k.rot);
            desloca(&g.pts, [k.t[0] - g.base[0], k.t[1] - g.base[1]])
        }).collect();
        for i in 0..4 {
            for j in i + 1..4 {
                assert!(!colide(&postos[i], &postos[j], FOLGA_MINIMA), "{i} x {j}");
            }
        }
    }

    #[test]
    fn peca_que_nao_cabe_gira_e_a_quinta_abre_outra_chapa() {
        // 90x40 só cabe em chapa 50x100 girada
        let p = PecaNesting::da_planificada("r", &retangulo(90., 40., None), 1).unwrap();
        let l = dispor(&[p.clone()], &op(50., 100.), None);
        assert_eq!(l.chapas.len(), 1);
        assert!((l.chapas[0].colocacoes[0].rot - 90.).abs() < 1e-9 || (l.chapas[0].colocacoes[0].rot - 270.).abs() < 1e-9);
        // fixa a orientação, não cabe
        let mut f = p.clone();
        f.girar = false;
        let l = dispor(&[f], &op(50., 100.), None);
        assert_eq!(l.nao_coube.len(), 1);
        // cinco quadrados de 50 em chapa 100x100: quatro mais um na segunda
        let q = PecaNesting::da_planificada("q", &retangulo(50., 50., None), 5).unwrap();
        let l = dispor(&[q], &op(100.003, 100.003), None);
        assert_eq!(l.chapas.len(), 2);
        assert_eq!(l.chapas[1].colocacoes.len(), 1);
    }

    #[test]
    fn espaco_entre_pecas_e_respeitado() {
        let q = PecaNesting::da_planificada("q", &retangulo(40., 40., None), 2).unwrap();
        let mut o = op(100., 40.);
        o.espaco = 5.;
        let l = dispor(&[q.clone()], &o, None);
        assert_eq!(l.chapas.len(), 1);
        let t: Vec<P> = l.chapas[0].colocacoes.iter().map(|k| k.t).collect();
        assert!((t[0][0] - t[1][0]).abs() >= 45. - 1e-6);
        // em 80 de largura, com 5 de espaço, só uma por chapa
        o.chapa.largura = 80.;
        let l = dispor(&[q], &o, None);
        assert_eq!(l.chapas.len(), 2);
    }

    #[test]
    fn peca_em_l_aceita_vizinha_na_reentrancia() {
        // L: 100x100 com recorte 60x60 no canto superior direito; o quadrado 50x50 entra no recorte
        let l_pts = [[0., 0.], [100., 0.], [100., 40.], [40., 40.], [40., 100.], [0., 100.]];
        let ents: Vec<Entidade> = (0..6).map(|i| Entidade::Linha(l_pts[i], l_pts[(i + 1) % 6])).collect();
        let mut pl = retangulo(100., 100., None);
        pl.contorno = ents;
        let ele = PecaNesting::da_planificada("L", &pl, 1).unwrap();
        let q = PecaNesting::da_planificada("q", &retangulo(55., 55., None), 1).unwrap();
        let mut o = op(110., 110.);
        o.rotacoes = vec![0.];
        // a chapa de 110x110 só comporta as duas peças se o quadrado ocupar o recorte do L
        let lay = dispor(&[ele, q], &o, None);
        assert_eq!(lay.chapas.len(), 1, "o quadrado deveria entrar na reentrância do L: {lay:?}");
    }

    #[test]
    fn contorno_com_arcos_vira_laco_e_aproveitamento_desconta_furos() {
        let plana = retangulo(60., 60., Some(10.));
        let p = PecaNesting::da_planificada("f", &plana, 1).unwrap();
        assert!((p.area_contorno() - 3600.).abs() < 1e-6);
        let furo = std::f64::consts::PI * 100.;
        assert!((p.area_liquida() - (3600. - furo)).abs() < 1.0, "{}", p.area_liquida());
        let sem_contorno = Planificada { contorno: vec![Entidade::Linha([0., 0.], [1., 0.])], ..plana };
        assert!(PecaNesting::da_planificada("x", &sem_contorno, 1).is_err());
    }

    #[test]
    fn melhor_esforco_nunca_piora_e_cancelar_devolve_o_primeiro() {
        let a = PecaNesting::da_planificada("a", &retangulo(70., 30., None), 3).unwrap();
        let b = PecaNesting::da_planificada("b", &retangulo(40., 40., None), 3).unwrap();
        let mut o = op(120., 80.);
        let base = dispor(&[a.clone(), b.clone()], &o, None);
        o.esforco = 12;
        let melhor = dispor(&[a.clone(), b.clone()], &o, None);
        assert!(nota_layout(&melhor, &[a.clone(), b.clone()]) <= nota_layout(&base, &[a.clone(), b.clone()]));
        let cancela = AtomicBool::new(true);
        let c = dispor(&[a.clone(), b.clone()], &o, Some(&cancela));
        assert_eq!(c, base);
    }

    #[test]
    fn dxf_por_chapa_tem_a_chapa_as_pecas_e_o_furo_girado() {
        let p = PecaNesting::da_planificada("f", &retangulo(80., 30., Some(5.)), 1).unwrap();
        // chapa 40x100: a peça só cabe girada, então o furo tem de acompanhar a rotação
        let o = op(40., 100.);
        let lay = dispor(&[p.clone()], &o, None);
        let txt = dxf_chapa(&[p.clone()], &lay, 0, &o);
        let d = fenkra_dxf::ler(txt.as_bytes()).expect("o DXF gerado precisa ler de volta");
        let na = |c: &str| d.itens.iter().filter(|i| i.camada == c).count();
        assert_eq!(na(CAMADA_CHAPA), 1);
        assert_eq!(na(planificada::CAMADA_CONTORNO), 4);
        assert_eq!(na(planificada::CAMADA_FUROS), 1);
        let furo = d.itens.iter().find(|i| i.camada == planificada::CAMADA_FUROS).unwrap();
        let fenkra_dxf::Entidade::Circulo { centro, .. } = &furo.entidade else { panic!("furo não é círculo") };
        // centro do retângulo girado 90 e encostado na origem: (15, 40)
        assert!((centro[0] - 15.).abs() < 1e-6 && (centro[1] - 40.).abs() < 1e-6, "{centro:?}");
        let csv = relatorio_csv(&[p], &lay, &o);
        assert!(csv.contains("total;1;4000.0;2400.0;"), "{csv}");
    }
}
