// SPDX-License-Identifier: LGPL-2.1-or-later
//! Estrutural: cordão de solda por conjuntos de faces (B285).
//!
//! O cordão nasce do encontro de dois conjuntos de faces: o kernel intersecta as faces do conjunto
//! 1 com as do conjunto 2 e devolve os caminhos (polilinhas). Cada caminho vira uma `Trilha`: os
//! pontos e, para cada segmento, as duas direções das pernas do filete. A perna A corre pela face
//! do conjunto 1, a perna B pela do conjunto 2, ambas perpendiculares ao caminho e para o lado
//! livre (o lado para onde as duas normais de saída apontam), de modo que o triângulo
//! (caminho, A, B) é a seção do metal depositado.
//!
//! O comprimento de solda é o do caminho (o mesmo conceito de `ResultadoDobra::solda`, que dá o
//! contorno de solda de um nó de tubo; aqui ele sai do encontro real das faces). O cordão contínuo
//! é um trecho por caminho; o intermitente divide o caminho por comprimento de arco em trechos de
//! comprimento `l` a cada passo `p` (= `l` + vão). Só entram trechos inteiros: o cordão nunca fica
//! mais comprido que o pedido. No alternado os caminhos de índice ímpar começam meio passo depois,
//! como a solda dos dois lados de uma chapa.
//!
//! Cosmético (padrão): só os caminhos e comprimentos, lidos por `weld_beads` (símbolo do desenho).
//! Sólido: o metal vira malha fechada por trecho, com chanfro de esquadria nas quinas do caminho
//! (sem frestas nem sobreposição), entra no corpo como peça separada e na lista de itens.

use super::*;

const EPS: f64 = 1e-9;

/// Um caminho de solda com as direções das pernas em cada segmento.
#[derive(Clone, Debug, PartialEq)]
pub struct Trilha {
    pub pts: Vec<V3>,
    /// perna sobre a face do conjunto 1, por segmento (unitária, perpendicular ao segmento)
    pub ua: Vec<V3>,
    /// perna sobre a face do conjunto 2, por segmento
    pub ub: Vec<V3>,
    pub fechada: bool,
}

impl Trilha {
    pub fn comprimento(&self) -> f64 {
        self.pts.windows(2).map(|w| geo3d::dist(w[0], w[1])).sum()
    }

    /// Pedaço entre os comprimentos de arco `s0` e `s1`, com as pontas exatas.
    pub fn trecho(&self, s0: f64, s1: f64) -> Trilha {
        let mut out = Trilha { pts: vec![], ua: vec![], ub: vec![], fechada: false };
        let mut acc = 0.;
        for (i, w) in self.pts.windows(2).enumerate() {
            let l = geo3d::dist(w[0], w[1]);
            let (a0, a1) = (acc, acc + l);
            acc = a1;
            if l < EPS || a1 <= s0 + EPS || a0 >= s1 - EPS {
                continue;
            }
            let (t0, t1) = (((s0 - a0) / l).clamp(0., 1.), ((s1 - a0) / l).clamp(0., 1.));
            if out.pts.is_empty() {
                out.pts.push(geo3d::lerp(w[0], w[1], t0));
            }
            out.pts.push(geo3d::lerp(w[0], w[1], t1));
            out.ua.push(self.ua[i]);
            out.ub.push(self.ub[i]);
        }
        out
    }
}

/// Início e fim (comprimento de arco) de cada trecho de um caminho de comprimento `total`.
/// `desloc` empurra o primeiro trecho (alternado). `passo = 0` ou `seg >= total` = um trecho só.
pub fn intervalos(total: f64, seg: f64, passo: f64, desloc: f64) -> Vec<(f64, f64)> {
    if seg <= EPS || passo <= EPS {
        return vec![(0., total)];
    }
    let mut out = vec![];
    let mut s = desloc;
    while s + seg <= total + 1e-6 {
        out.push((s, (s + seg).min(total)));
        s += passo;
    }
    out
}

/// Pedaços da polilinha do lado `frente` do plano (origem `o`, normal `n`). Um caminho que cruza o
/// plano devolve o pedaço de cada lado que sobrou; o ponto de corte fica exatamente no plano.
pub fn recorta_no_plano(pts: &[V3], o: V3, n: V3, frente: bool) -> Vec<Vec<V3>> {
    let sinal = if frente { 1. } else { -1. };
    let d = |p: V3| sinal * geo3d::dot(geo3d::sub(p, o), n);
    let mut out: Vec<Vec<V3>> = vec![];
    let mut atual: Vec<V3> = vec![];
    for w in pts.windows(2) {
        let (da, db) = (d(w[0]), d(w[1]));
        let dentro = |x: f64| x >= -1e-9;
        if dentro(da) && atual.is_empty() {
            atual.push(w[0]);
        }
        match (dentro(da), dentro(db)) {
            (true, true) => atual.push(w[1]),
            (true, false) => {
                let t = da / (da - db);
                atual.push(geo3d::lerp(w[0], w[1], t));
                out.push(std::mem::take(&mut atual));
            }
            (false, true) => {
                let t = da / (da - db);
                atual = vec![geo3d::lerp(w[0], w[1], t), w[1]];
            }
            (false, false) => {}
        }
    }
    if atual.len() >= 2 {
        out.push(atual);
    }
    out.retain(|p| p.len() >= 2 && p.windows(2).map(|w| geo3d::dist(w[0], w[1])).sum::<f64>() > 1e-6);
    out
}

/// Pernas de um segmento de direção `t` entre as faces de normais de saída `na` e `nb`.
pub fn pernas(t: V3, na: V3, nb: V3) -> Result<(V3, V3), String> {
    let t = geo3d::unit(t).ok_or("segmento de solda sem comprimento")?;
    let mut a = geo3d::unit(geo3d::cross(na, t)).ok_or("o caminho segue a normal da face")?;
    let mut b = geo3d::unit(geo3d::cross(nb, t)).ok_or("o caminho segue a normal da face")?;
    // cada perna se afasta da outra face, para o lado livre do encontro
    if geo3d::dot(a, nb) < 0. {
        a = geo3d::mul(a, -1.);
    }
    if geo3d::dot(b, na) < 0. {
        b = geo3d::mul(b, -1.);
    }
    if geo3d::dot(a, nb).abs() < 1e-6 || geo3d::dot(b, na).abs() < 1e-6 {
        return Err("as faces dos dois conjuntos são paralelas ou tangentes: não há canto para o filete".into());
    }
    Ok((a, b))
}

/// Pontos do perfil do cordão em relação ao caminho (o primeiro é o próprio caminho), a partir das
/// pernas de comprimento `a`. Convexo e côncavo trocam a hipotenusa por um arco parabólico com
/// flecha de 15% da corda.
fn perfil(ua: V3, ub: V3, a: f64, formato: BeadShape) -> Vec<V3> {
    let (pa, pb) = (geo3d::mul(ua, a), geo3d::mul(ub, a));
    let mut v = vec![[0.; 3], pa];
    if formato != BeadShape::Plano {
        let corda = geo3d::dist(pa, pb);
        let meio = geo3d::mul(geo3d::add(pa, pb), 0.5);
        let fora = geo3d::unit(meio).unwrap_or(ua);
        let flecha = 0.15 * corda * if formato == BeadShape::Convexo { 1. } else { -1. };
        for f in [0.25, 0.5, 0.75] {
            let base = geo3d::lerp(pa, pb, f);
            v.push(geo3d::add(base, geo3d::mul(fora, flecha * 4. * f * (1. - f))));
        }
    }
    v.push(pb);
    v
}

/// Posição do anel de perfil no vértice `i`: nas pontas, o perfil do único segmento; no meio, o
/// ponto onde as retas dos dois segmentos vizinhos se cruzam (esquadria), que fecha a quina sem
/// frestas. Se as retas não se cruzam (perfis incompatíveis), a média.
fn anel(tr: &Trilha, i: usize, a: f64, formato: BeadShape) -> Vec<V3> {
    let n = tr.pts.len() - 1;
    let (prev, next) = match (i, i == n) {
        (0, _) => (if tr.fechada { Some(n - 1) } else { None }, Some(0)),
        (_, true) => (Some(n - 1), if tr.fechada { Some(0) } else { None }),
        _ => (Some(i - 1), Some(i)),
    };
    let p = tr.pts[i];
    let dir = |k: usize| geo3d::unit(geo3d::sub(tr.pts[k + 1], tr.pts[k])).unwrap_or([1., 0., 0.]);
    let w = |k: usize| perfil(tr.ua[k], tr.ub[k], a, formato);
    match (prev, next) {
        (Some(k0), Some(k1)) => {
            let (w0, w1) = (w(k0), w(k1));
            let (t0, t1) = (dir(k0), dir(k1));
            w0.iter()
                .zip(&w1)
                .map(|(a0, a1)| {
                    let dw = geo3d::sub(*a1, *a0);
                    // resolve mu*t0 - nu*t1 = dw pelos mínimos quadrados
                    let (g00, g01, g11) = (geo3d::dot(t0, t0), -geo3d::dot(t0, t1), geo3d::dot(t1, t1));
                    let (b0, b1) = (geo3d::dot(t0, dw), -geo3d::dot(t1, dw));
                    let det = g00 * g11 - g01 * g01;
                    if det.abs() > 1e-9 {
                        let mu = (b0 * g11 - b1 * g01) / det;
                        let nu = (g00 * b1 - g01 * b0) / det;
                        let res = geo3d::sub(geo3d::sub(geo3d::mul(t0, mu), geo3d::mul(t1, nu)), dw);
                        if geo3d::norm(res) < 1e-6 * a.max(1.) {
                            return geo3d::add(p, geo3d::add(*a0, geo3d::mul(t0, mu)));
                        }
                    }
                    geo3d::add(p, geo3d::mul(geo3d::add(*a0, *a1), 0.5))
                })
                .collect()
        }
        (Some(k), None) | (None, Some(k)) => w(k).into_iter().map(|q| geo3d::add(p, q)).collect(),
        _ => vec![],
    }
}

/// Malha fechada (triângulos) do metal depositado ao longo da trilha.
pub fn malha_do_cordao(tr: &Trilha, perna: f64, formato: BeadShape) -> Vec<f64> {
    let rings: Vec<Vec<V3>> = (0..tr.pts.len()).map(|i| anel(tr, i, perna, formato)).collect();
    let m = rings[0].len();
    let mut t = vec![];
    let mut tri = |a: V3, b: V3, c: V3| t.extend_from_slice(&[a[0], a[1], a[2], b[0], b[1], b[2], c[0], c[1], c[2]]);
    let ultimo = if tr.fechada { rings.len() - 1 } else { rings.len() };
    for k in 0..rings.len() - 1 {
        for j in 0..m {
            let j2 = (j + 1) % m;
            let (a, b, c, d) = (rings[k][j], rings[k][j2], rings[k + 1][j2], rings[k + 1][j]);
            tri(a, b, c);
            tri(a, c, d);
        }
    }
    if !tr.fechada {
        for k in [0, ultimo - 1] {
            for j in 1..m - 1 {
                tri(rings[k][0], rings[k][j], rings[k][j + 1]);
            }
        }
    }
    t
}

/// Um trecho do cordão (contínuo: o caminho todo).
#[derive(Clone, Debug)]
pub struct TrechoCordao {
    pub pts: Vec<V3>,
    pub comprimento: f64,
}

/// Um caminho do cordão com seus trechos.
#[derive(Clone, Debug)]
pub struct CaminhoCordao {
    pub pts: Vec<V3>,
    pub comprimento: f64,
    pub fechado: bool,
    pub trechos: Vec<TrechoCordao>,
    /// B174: pernas do filete em cada segmento de `pts` (sobre a face do conjunto 1 e do 2)
    pub pernas: Vec<(V3, V3)>,
}

/// O que o cordão de um comando calculou, para a interface (lista de caminhos com realce) e para o
/// símbolo do desenho.
#[derive(Clone, Debug)]
pub struct CordaoCalculado {
    pub feature: Id,
    pub modo: BeadMode,
    pub formato: BeadShape,
    pub padrao: BeadPattern,
    pub alternado: bool,
    /// perna do filete (mm)
    pub perna: f64,
    pub comprimento_trecho: f64,
    pub vao: f64,
    pub caminhos: Vec<CaminhoCordao>,
    /// metal depositado (mm³); 0 no cosmético
    pub volume: f64,
}

impl CordaoCalculado {
    pub fn comprimento_total(&self) -> f64 {
        self.caminhos.iter().map(|c| c.comprimento).sum()
    }
    /// Comprimento realmente soldado (a soma dos trechos).
    pub fn comprimento_soldado(&self) -> f64 {
        self.caminhos.iter().flat_map(|c| &c.trechos).map(|t| t.comprimento).sum()
    }
    pub fn trechos(&self) -> usize {
        self.caminhos.iter().map(|c| c.trechos.len()).sum()
    }
    pub fn intermitente(&self) -> bool {
        self.padrao != BeadPattern::Continuo
    }
}

thread_local! {
    /// coleta dos cordões durante `weld_beads` (fora dela fica `None`)
    static WELD_LOG: RefCell<Option<Vec<CordaoCalculado>>> = const { RefCell::new(None) };
}

/// Cordões de solda do documento, com caminhos, trechos, comprimentos e volume. Recalcula o
/// documento; chame sob demanda (a interface guarda o resultado).
pub fn weld_beads(doc: &Document) -> Vec<CordaoCalculado> {
    if !doc.active_features().any(|(_, f)| matches!(f.kind, FeatureKind::StructWeld(_))) {
        return vec![];
    }
    WELD_LOG.with(|l| *l.borrow_mut() = Some(vec![]));
    let _ = final_shape(doc);
    WELD_LOG.with(|l| l.borrow_mut().take()).unwrap_or_default()
}

fn opcional(src: &str, vars: &Vars, campo: &str) -> Result<f64, String> {
    if src.trim().is_empty() {
        Ok(0.)
    } else {
        ev(src, vars, campo)
    }
}

impl Ctx<'_> {
    /// Faces de um conjunto como uma forma só (com a cadeia tangente quando pedido).
    fn faces_do_conjunto(&self, corpo: &Shape, pts: &[V3], tangente: bool) -> Result<Shape, String> {
        let k = |e: KernelError| e.0;
        let mut all = Shape::empty();
        for p in pts {
            let s = if tangente { corpo.face_propagate(*p, 2, false).map_err(k)?.0 } else { corpo.face_copy(*p).map_err(k)? };
            all = if all.is_null() { s } else { all.compound(&s) };
        }
        Ok(all)
    }

    pub(super) fn struct_weld(&mut self, f: &Feature, w: &StructWeld) -> Result<Option<String>, String> {
        let k = |e: KernelError| e.0;
        let (p1, p2) = w.conjuntos();
        if p1.is_empty() || p2.is_empty() {
            return Ok(Some("escolha as faces dos dois conjuntos na viewport — até lá o cordão não é aplicado".into()));
        }
        let perna = ev(&w.leg, &self.vars, "perna do filete")?;
        if perna <= 0. {
            return Err("a perna do filete precisa ser maior que 0".into());
        }
        let (seg, vao) = match w.pattern {
            BeadPattern::Continuo => (0., 0.),
            _ => {
                let s = ev(&w.seg_len, &self.vars, "comprimento do trecho")?;
                let e = opcional(&w.spacing, &self.vars, if w.pattern == BeadPattern::ComprimentoEVao { "vão" } else { "passo" })?;
                if s <= 0. {
                    return Err("o comprimento do trecho precisa ser maior que 0".into());
                }
                let vao = if w.pattern == BeadPattern::ComprimentoEVao { e } else { e - s };
                if vao < 0. {
                    return Err("o passo não pode ser menor que o comprimento do trecho".into());
                }
                if w.pattern == BeadPattern::ComprimentoEVao && e <= 0. {
                    return Err("o vão precisa ser maior que 0 (para um cordão sem vão, use contínuo)".into());
                }
                (s, vao)
            }
        };
        let (corpo, owners, roles) = self.body_owned(f.target).ok_or("não há corpo para soldar")?;
        let corpo = corpo.clone();
        let (owners, roles) = (owners.to_vec(), roles.to_vec());
        let pts = self.resolve_faces("cordão de solda", f, &corpo, &owners, &roles, &w.faces, &w.face_keys)?;
        let (q1, q2) = pts.split_at(p1.len());
        let (s1, s2) = (self.faces_do_conjunto(&corpo, q1, w.tangent)?, self.faces_do_conjunto(&corpo, q2, w.tangent)?);
        let caminhos = s1.section(&s2).map_err(|_| match distancia_aproximada(&s1, &s2) {
            // kart CV-EST: dizer quão longe um conjunto está do outro ajuda a achar a face errada
            Some(d) if d > 0.05 => format!("os dois conjuntos de faces não se encontram: o conjunto 1 fica a cerca de {d:.1} mm do conjunto 2 (as faces precisam se tocar); confira as faces escolhidas"),
            _ => "os dois conjuntos de faces não se encontram: não há caminho para o cordão".to_string(),
        })?;
        // término em plano
        let mut caminhos = caminhos;
        if let Some(pl) = &w.end_plane {
            let fr = self.plane(pl, 0)?;
            caminhos = caminhos.iter().flat_map(|c| recorta_no_plano(c, fr.origin, fr.normal, w.end_keep_front)).collect();
            if caminhos.is_empty() {
                return Err("o plano de término deixou o cordão sem nenhum caminho".into());
            }
        }
        let passo = seg + vao;
        let mut calc = CordaoCalculado {
            feature: f.id,
            modo: w.mode,
            formato: w.shape,
            padrao: w.pattern,
            alternado: w.staggered,
            perna,
            comprimento_trecho: seg,
            vao,
            caminhos: vec![],
            volume: 0.,
        };
        let mut solidos = Shape::empty();
        let mut n_item = 0usize;
        for (idx, c) in caminhos.iter().enumerate() {
            if c.len() < 2 {
                continue;
            }
            let mut tr = Trilha { pts: c.clone(), ua: vec![], ub: vec![], fechada: false };
            tr.fechada = tr.pts.len() > 3 && geo3d::dist(tr.pts[0], *tr.pts.last().unwrap()) < 1e-4;
            for g in tr.pts.windows(2) {
                let meio = geo3d::lerp(g[0], g[1], 0.5);
                let na = s1.surface_near(meio).map_err(k)?.1;
                let nb = s2.surface_near(meio).map_err(k)?.1;
                let (a, b) = pernas(geo3d::sub(g[1], g[0]), na, nb)?;
                tr.ua.push(a);
                tr.ub.push(b);
            }
            let total = tr.comprimento();
            let desloc = if w.staggered && idx % 2 == 1 { passo / 2. } else { 0. };
            let ivs = match w.pattern {
                BeadPattern::Continuo => vec![(0., total)],
                _ => intervalos(total, seg, passo, desloc),
            };
            let pernas = tr.ua.iter().zip(&tr.ub).map(|(a, b)| (*a, *b)).collect();
            let mut caminho = CaminhoCordao { pts: tr.pts.clone(), comprimento: total, fechado: tr.fechada, trechos: vec![], pernas };
            for (s0, s1_) in ivs {
                // o cordão contínuo de um caminho fechado mantém a esquadria da quina de fechamento
                let todo = w.pattern == BeadPattern::Continuo;
                let sub = if todo { tr.clone() } else { tr.trecho(s0, s1_) };
                if sub.pts.len() < 2 {
                    continue;
                }
                caminho.trechos.push(TrechoCordao { pts: sub.pts.clone(), comprimento: sub.comprimento() });
                if w.mode == BeadMode::Solido {
                    let malha = malha_do_cordao(&sub, perna, w.shape);
                    let s = Shape::from_mesh(&malha, true).map_err(k)?;
                    let (lo, hi) = s.bbox().ok_or("cordão sem volume")?;
                    if geo3d::dist(lo, hi) < 1e-9 || s.mass().volume.abs() < 1e-9 {
                        return Err("o cordão de solda saiu sem volume: confira a perna e as faces".into());
                    }
                    calc.volume += s.mass().volume;
                    let t = geo3d::unit(geo3d::sub(sub.pts[1], sub.pts[0])).unwrap_or([1., 0., 0.]);
                    let n = geo3d::unit(geo3d::add(sub.ua[0], sub.ub[0])).unwrap_or(geo3d::any_perp(t));
                    let fr = Frame { origin: sub.pts[0], x: t, normal: n };
                    let nome = format!("Cordão z{}", num_curto(perna));
                    log_struct(f, n_item, StructItem::Weld, &nome, format!("Filete z{}", num_curto(perna)), sub.comprimento(), fr, &s, &w.material);
                    n_item += 1;
                    solidos = if solidos.is_null() { s } else { solidos.compound(&s) };
                }
            }
            calc.caminhos.push(caminho);
        }
        if calc.trechos() == 0 {
            return Err("nenhum trecho cabe no caminho: reduza o comprimento do trecho".into());
        }
        if w.mode == BeadMode::Solido {
            self.add_separate(f, solidos);
        }
        WELD_LOG.with(|l| {
            if let Some(v) = l.borrow_mut().as_mut() {
                v.push(calc);
            }
        });
        Ok(None)
    }
}

/// Medida curta para nomes: "5" ou "5,5".
pub fn num_curto(v: f64) -> String {
    if (v - v.round()).abs() < 1e-9 {
        format!("{}", v.round() as i64)
    } else {
        format!("{v:.1}").replace('.', ",")
    }
}

/// Menor distância entre os vértices das malhas de dois conjuntos de faces (aproximada pela
/// malha; serve só para a mensagem de erro).
fn distancia_aproximada(a: &Shape, b: &Shape) -> Option<f64> {
    let pts = |s: &Shape| -> Option<Vec<[f64; 3]>> {
        let m = s.mesh_angle(0.5, 0.35).ok()?;
        Some(m.positions.chunks(3).map(|v| [v[0] as f64, v[1] as f64, v[2] as f64]).collect())
    };
    let (pa, pb) = (pts(a)?, pts(b)?);
    if pa.is_empty() || pb.is_empty() || pa.len().saturating_mul(pb.len()) > 4_000_000 {
        return None;
    }
    pa.iter().flat_map(|p| pb.iter().map(move |q| geo3d::dist(*p, *q))).min_by(f64::total_cmp)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reta(n: usize, l: f64) -> Trilha {
        Trilha { pts: (0..=n).map(|i| [l * i as f64 / n as f64, 0., 0.]).collect(), ua: vec![[0., 0., 1.]; n], ub: vec![[0., 1., 0.]; n], fechada: false }
    }

    #[test]
    fn intervalos_contam_so_trechos_inteiros() {
        // 100 mm, trecho 10, passo 20: inícios em 0, 20, 40, 60, 80
        assert_eq!(intervalos(100., 10., 20., 0.).len(), 5);
        // 105 mm: o trecho que começaria em 100 não cabe inteiro
        assert_eq!(intervalos(105., 10., 20., 0.).len(), 5);
        // 110 mm: cabe o de 100
        assert_eq!(intervalos(110., 10., 20., 0.).len(), 6);
        // alternado: começa meio passo depois e perde um trecho no fim
        let alt = intervalos(100., 10., 20., 10.);
        assert_eq!(alt.first().copied(), Some((10., 20.)));
        assert_eq!(alt.len(), 5);
        // contínuo e trecho maior que o caminho
        assert_eq!(intervalos(100., 0., 0., 0.), vec![(0., 100.)]);
        assert!(intervalos(5., 10., 20., 0.).is_empty());
    }

    #[test]
    fn trecho_tem_as_pontas_exatas_e_o_comprimento_pedido() {
        let tr = reta(4, 100.);
        let t = tr.trecho(30., 70.);
        assert!((t.comprimento() - 40.).abs() < 1e-9);
        assert!((t.pts[0][0] - 30.).abs() < 1e-9 && (t.pts.last().unwrap()[0] - 70.).abs() < 1e-9);
        assert_eq!(t.ua.len(), t.pts.len() - 1);
    }

    #[test]
    fn plano_de_termino_corta_e_mantem_o_lado() {
        let pts = vec![[0., 0., 0.], [100., 0., 0.]];
        let a = recorta_no_plano(&pts, [60., 0., 0.], [1., 0., 0.], false);
        assert_eq!(a.len(), 1);
        assert!((a[0][1][0] - 60.).abs() < 1e-9);
        let b = recorta_no_plano(&pts, [60., 0., 0.], [1., 0., 0.], true);
        assert!((b[0][0][0] - 60.).abs() < 1e-9 && (b[0][1][0] - 100.).abs() < 1e-9);
        assert!(recorta_no_plano(&pts, [200., 0., 0.], [1., 0., 0.], true).is_empty());
    }

    #[test]
    fn pernas_ficam_no_lado_livre_do_canto() {
        // tubo em pé (face lateral, normal +x) sobre uma chapa (normal +z), caminho ao longo de y
        let (a, b) = pernas([0., 10., 0.], [1., 0., 0.], [0., 0., 1.]).unwrap();
        assert!(geo3d::dot(a, [0., 0., 1.]) > 0.99, "a perna da face lateral sobe: {a:?}");
        assert!(geo3d::dot(b, [1., 0., 0.]) > 0.99, "a perna da chapa se afasta do tubo: {b:?}");
        assert!(pernas([0., 10., 0.], [0., 0., 1.], [0., 0., 1.]).is_err(), "faces paralelas não têm canto");
    }

    #[test]
    fn malha_do_filete_reto_tem_a_area_do_triangulo() {
        // sem kernel: volume da malha por divergência = a^2/2 * L
        let tr = reta(1, 100.);
        let m = malha_do_cordao(&tr, 6., BeadShape::Plano);
        let mut v = 0.;
        for t in m.chunks(9) {
            let (a, b, c) = ([t[0], t[1], t[2]], [t[3], t[4], t[5]], [t[6], t[7], t[8]]);
            v += geo3d::dot(a, geo3d::cross(b, c)) / 6.;
        }
        assert!((v.abs() - 18. * 100.).abs() < 1e-6, "{v}");
    }

    #[test]
    fn convexo_pesa_mais_e_concavo_menos_que_o_plano() {
        let vol = |f| {
            let m = malha_do_cordao(&reta(1, 50.), 6., f);
            m.chunks(9).map(|t| geo3d::dot([t[0], t[1], t[2]], geo3d::cross([t[3], t[4], t[5]], [t[6], t[7], t[8]])) / 6.).sum::<f64>().abs()
        };
        let (p, cx, cv) = (vol(BeadShape::Plano), vol(BeadShape::Convexo), vol(BeadShape::Concavo));
        assert!(cx > p && cv < p, "{cx} {p} {cv}");
        // a parábola de flecha 15% da corda é amostrada em 3 pontos (alturas 0,75, 1 e 0,75 da
        // flecha): a área da poligonal é 0,625 x corda x flecha, um pouco menos que os 2/3 da curva
        let corda = 6. * 2f64.sqrt();
        assert!((cx - p - 50. * 0.625 * corda * 0.15 * corda).abs() < 1e-6);
        assert!((p - cv - 50. * 0.625 * corda * 0.15 * corda).abs() < 1e-6);
    }

    #[test]
    fn quina_fechada_tem_esquadria_sem_perder_nem_somar_volume() {
        // quadrado de 40 sobre o plano z = 0 com o cordão por fora: perna A sobe, perna B sai para fora
        let q = [[0., 0., 0.], [40., 0., 0.], [40., 40., 0.], [0., 40., 0.], [0., 0., 0.]];
        let mut tr = Trilha { pts: q.to_vec(), ua: vec![], ub: vec![], fechada: true };
        for w in q.windows(2) {
            let t = geo3d::sub(w[1], w[0]);
            let fora = geo3d::unit([t[1], -t[0], 0.]).unwrap();
            tr.ua.push([0., 0., 1.]);
            tr.ub.push(fora);
        }
        let m = malha_do_cordao(&tr, 5., BeadShape::Plano);
        let v: f64 = m.chunks(9).map(|t| geo3d::dot([t[0], t[1], t[2]], geo3d::cross([t[3], t[4], t[5]], [t[6], t[7], t[8]])) / 6.).sum::<f64>().abs();
        // Pappus: área do triângulo x perímetro da linha do centroide (a/3 para fora): 160 + 8 a/3
        let esperado = 12.5 * (160. + 8. * 5. / 3.);
        assert!((v - esperado).abs() < 1e-6, "{v} {esperado}");
    }
}
