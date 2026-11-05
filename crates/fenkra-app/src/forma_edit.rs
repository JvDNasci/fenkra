// SPDX-License-Identifier: LGPL-2.1-or-later
//! Regras da bancada Forma livre (B153), sem egui: o que está sob o mouse, em que eixos o manipulador
//! trabalha e o que cada gesto faz na gaiola. `forma_ui.rs` só desenha e lê o mouse, para estas regras
//! serem testadas sem janela.

use fenkra_core::subd::{Gaiola, Simetria, V3};
use fenkra_core::subd_ops::TipoNo;
use std::collections::BTreeSet;

fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn mul(a: V3, k: f64) -> V3 {
    [a[0] * k, a[1] * k, a[2] * k]
}
fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn unit(a: V3) -> Option<V3> {
    let n = dot(a, a).sqrt();
    (n > 1e-12).then(|| mul(a, 1. / n))
}

/// Pedaço da gaiola: um nó, uma aresta (com `a < b`) ou uma face.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Elem {
    No(u32),
    Aresta(u32, u32),
    Face(u32),
}

/// O que aparece: Tab alterna, e o botão do 3D faz o mesmo.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ModoVista {
    Gaiola,
    Liso,
    #[default]
    Ambos,
}

impl ModoVista {
    pub const TODOS: [ModoVista; 3] = [ModoVista::Gaiola, ModoVista::Liso, ModoVista::Ambos];

    pub fn proximo(self) -> ModoVista {
        match self {
            ModoVista::Gaiola => ModoVista::Liso,
            ModoVista::Liso => ModoVista::Ambos,
            ModoVista::Ambos => ModoVista::Gaiola,
        }
    }

    pub fn rotulo(self) -> &'static str {
        match self {
            ModoVista::Gaiola => "Gaiola",
            ModoVista::Liso => "Liso",
            ModoVista::Ambos => "Ambos",
        }
    }
}

/// Em que eixos o manipulador trabalha.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Espaco {
    #[default]
    Mundo,
    Vista,
    Normal,
    Local,
}

impl Espaco {
    pub const TODOS: [Espaco; 4] = [Espaco::Mundo, Espaco::Vista, Espaco::Normal, Espaco::Local];

    pub fn rotulo(self) -> &'static str {
        match self {
            Espaco::Mundo => "Mundo",
            Espaco::Vista => "Vista",
            Espaco::Normal => "Normal",
            Espaco::Local => "Local",
        }
    }

    pub fn dica(self) -> &'static str {
        match self {
            Espaco::Mundo => "Eixos do modelo",
            Espaco::Vista => "Eixos da tela: o terceiro aponta para você",
            Espaco::Normal => "O terceiro eixo segue a normal do que está selecionado",
            Espaco::Local => "Eixos da própria aresta ou face",
        }
    }
}

/// Ferramenta ligada na barra do 3D. Sem nenhuma, o mouse só seleciona e move.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum Ferramenta {
    #[default]
    Selecionar,
    /// passar o mouse numa aresta mostra o laço; apertar e arrastar desliza
    Laco,
    /// primeiro nó já clicado
    InserirAresta(Option<u32>),
    /// raio e segmentos da faixa
    Suavizar { raio: f64, segmentos: u32 },
    Tangencia,
}

/// O que há em cada elemento para o mouse: posições na tela já projetadas.
pub struct Tela<'a> {
    /// posição na tela de um ponto do modelo
    pub projetar: &'a dyn Fn(V3) -> Option<[f32; 2]>,
    /// a face com este centro e esta normal olha para a câmera?
    pub de_frente: &'a dyn Fn(V3, V3) -> bool,
    /// distância do ponto à câmera (para desempatar quem está na frente)
    pub profundidade: &'a dyn Fn(V3) -> f64,
}

/// Raio de captura do nó e da aresta, em pontos de tela.
pub const RAIO_NO: f32 = 9.;
pub const RAIO_ARESTA: f32 = 6.;

fn dist_seg(p: [f32; 2], a: [f32; 2], b: [f32; 2]) -> (f32, f32) {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let l2 = dx * dx + dy * dy;
    let t = if l2 < 1e-6 { 0. } else { (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / l2).clamp(0., 1.) };
    let (x, y) = (a[0] + dx * t - p[0], a[1] + dy * t - p[1]);
    ((x * x + y * y).sqrt(), t)
}

fn dentro(p: [f32; 2], poli: &[[f32; 2]]) -> bool {
    let mut in_ = false;
    let n = poli.len();
    for i in 0..n {
        let (a, b) = (poli[i], poli[(i + n - 1) % n]);
        if (a[1] > p[1]) != (b[1] > p[1]) && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0] {
            in_ = !in_;
        }
    }
    in_
}

/// Faces que olham para a câmera.
pub fn faces_de_frente(g: &Gaiola, t: &Tela) -> BTreeSet<u32> {
    g.faces.iter().filter(|f| g.centro_face(f.id).zip(g.normal_face(f.id)).is_some_and(|(c, n)| (t.de_frente)(c, n))).map(|f| f.id).collect()
}

/// O que está sob o mouse, sem trocar de ferramenta: nó antes de aresta, aresta antes de face.
pub fn escolher(g: &Gaiola, t: &Tela, m: [f32; 2]) -> Option<Elem> {
    let frente = faces_de_frente(g, t);
    let visivel = |id: u32| g.faces.iter().any(|f| frente.contains(&f.id) && f.nos.contains(&id));
    let mut melhor: Option<(f32, u32)> = None;
    for n in &g.nos {
        let Some(s) = (t.projetar)(n.p) else { continue };
        let d = ((s[0] - m[0]).powi(2) + (s[1] - m[1]).powi(2)).sqrt();
        if d < RAIO_NO && melhor.is_none_or(|x| d < x.0) && visivel(n.id) {
            melhor = Some((d, n.id));
        }
    }
    if let Some((_, id)) = melhor {
        return Some(Elem::No(id));
    }
    let mut melhor: Option<(f32, u32, u32)> = None;
    for (a, b) in g.arestas() {
        let (Some(pa), Some(pb)) = (g.pos(a), g.pos(b)) else { continue };
        let (Some(sa), Some(sb)) = ((t.projetar)(pa), (t.projetar)(pb)) else { continue };
        let (d, _) = dist_seg(m, sa, sb);
        let vista = g.faces.iter().any(|f| frente.contains(&f.id) && f.nos.contains(&a) && f.nos.contains(&b));
        if d < RAIO_ARESTA && vista && melhor.is_none_or(|x| d < x.0) {
            melhor = Some((d, a, b));
        }
    }
    if let Some((_, a, b)) = melhor {
        return Some(Elem::Aresta(a, b));
    }
    let mut melhor: Option<(f64, u32)> = None;
    for f in g.faces.iter().filter(|f| frente.contains(&f.id)) {
        let poli: Option<Vec<[f32; 2]>> = f.nos.iter().map(|n| g.pos(*n).and_then(|p| (t.projetar)(p))).collect();
        let Some(poli) = poli else { continue };
        if dentro(m, &poli) {
            let z = g.centro_face(f.id).map_or(f64::MAX, |c| (t.profundidade)(c));
            if melhor.is_none_or(|x| z < x.0) {
                melhor = Some((z, f.id));
            }
        }
    }
    melhor.map(|x| Elem::Face(x.1))
}

/// Nós movidos por uma seleção.
pub fn nos_de(g: &Gaiola, sel: &[Elem]) -> Vec<u32> {
    let mut v = BTreeSet::new();
    for e in sel {
        match *e {
            Elem::No(n) => {
                v.insert(n);
            }
            Elem::Aresta(a, b) => {
                v.insert(a);
                v.insert(b);
            }
            Elem::Face(f) => {
                if let Some(f) = g.face(f) {
                    v.extend(f.nos.iter().copied());
                }
            }
        }
    }
    v.into_iter().filter(|n| g.no(*n).is_some()).collect()
}

/// Centro da seleção, onde o manipulador fica.
pub fn centro(g: &Gaiola, sel: &[Elem]) -> Option<V3> {
    let nos = nos_de(g, sel);
    if nos.is_empty() {
        return None;
    }
    let s = nos.iter().filter_map(|n| g.pos(*n)).fold([0.; 3], add);
    Some(mul(s, 1. / nos.len() as f64))
}

/// Normal de uma seleção: da face, a média das duas faces da aresta, ou a do nó.
pub fn normal(g: &Gaiola, sel: &[Elem]) -> Option<V3> {
    let mut s = [0.; 3];
    for e in sel {
        let n = match *e {
            Elem::No(n) => g.normal_no(n),
            Elem::Aresta(a, b) => unit(add(g.normal_no(a).unwrap_or([0.; 3]), g.normal_no(b).unwrap_or([0.; 3]))),
            Elem::Face(f) => g.normal_face(f),
        };
        s = add(s, n?);
    }
    unit(s)
}

/// Terno direito com o terceiro eixo em `z` e o primeiro o mais perto possível de `x`.
fn terno(z: V3, x: V3) -> [V3; 3] {
    let zz = unit(z).unwrap_or([0., 0., 1.]);
    let tenta = sub(x, mul(zz, dot(x, zz)));
    let xx = unit(tenta).or_else(|| unit(cross(zz, [0., 1., 0.]))).or_else(|| unit(cross(zz, [1., 0., 0.]))).unwrap_or([1., 0., 0.]);
    [xx, cross(zz, xx), zz]
}

/// Eixos do manipulador. `vista` são a direita, o alto e o sentido para a câmera.
pub fn eixos(g: &Gaiola, sel: &[Elem], e: Espaco, vista: [V3; 3]) -> [V3; 3] {
    let mundo = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
    match e {
        Espaco::Mundo => mundo,
        Espaco::Vista => vista,
        Espaco::Normal => normal(g, sel).map_or(mundo, |n| terno(n, [1., 0., 0.])),
        Espaco::Local => {
            let Some(n) = normal(g, sel) else { return mundo };
            // primeira aresta do elemento
            let dir = match sel.first() {
                Some(Elem::Aresta(a, b)) => g.pos(*a).zip(g.pos(*b)).map(|(p, q)| sub(q, p)),
                Some(Elem::Face(f)) => g.face(*f).and_then(|x| g.pos(x.nos[0]).zip(g.pos(x.nos[1]))).map(|(p, q)| sub(q, p)),
                Some(Elem::No(id)) => g.arestas().into_iter().find(|(a, b)| a == id || b == id).and_then(|(a, b)| g.pos(a).zip(g.pos(b))).map(|(p, q)| sub(q, p)),
                None => None,
            };
            terno(n, dir.unwrap_or([1., 0., 0.]))
        }
    }
}

/// Soma dos valores do manipulador nos eixos dele.
pub fn deslocamento(ax: [V3; 3], vals: [f64; 3]) -> V3 {
    add(add(mul(ax[0], vals[0]), mul(ax[1], vals[1])), mul(ax[2], vals[2]))
}

/// Move os nós a partir de como estavam (`base`). Nó da costura de simetria só anda dentro do plano.
pub fn mover(g: &mut Gaiola, base: &Gaiola, nos: &[u32], d: V3) {
    for &id in nos {
        let Some(p) = base.pos(id) else { continue };
        let mut q = add(p, d);
        if let Some(eixo) = base.no_preso_ao_plano(id) {
            q[eixo as usize] = p[eixo as usize];
        }
        if let Some(n) = g.no_mut(id) {
            n.p = q;
        }
    }
    g.refazer_tangencias();
}

/// Gira os nós em torno do eixo por `c`, a partir de como estavam.
pub fn girar(g: &mut Gaiola, base: &Gaiola, nos: &[u32], c: V3, eixo: V3, graus: f64) {
    let Some(k) = unit(eixo) else { return };
    let (s, co) = graus.to_radians().sin_cos();
    for &id in nos {
        let Some(p) = base.pos(id) else { continue };
        let v = sub(p, c);
        let r = add(add(mul(v, co), mul(cross(k, v), s)), mul(k, dot(k, v) * (1. - co)));
        let mut q = add(c, r);
        if let Some(e) = base.no_preso_ao_plano(id) {
            q[e as usize] = p[e as usize];
        }
        if let Some(n) = g.no_mut(id) {
            n.p = q;
        }
    }
    g.refazer_tangencias();
}

/// Faces de uma seleção.
pub fn faces_de(sel: &[Elem]) -> Vec<u32> {
    sel.iter().filter_map(|e| if let Elem::Face(f) = e { Some(*f) } else { None }).collect()
}

/// Alt ao arrastar: a extrusão nasce com altura zero e o arraste a abre. A seleção continua a mesma
/// porque as faces extrudadas guardam o número.
pub fn comecar_extrusao(g: &mut Gaiola, sel: &[Elem]) -> Result<(), String> {
    g.extrudar(&faces_de(sel), 0.).map(|_| ())
}

/// Chip Suave, Vinco ou Canto sobre a seleção.
pub fn aplicar_tipo(g: &mut Gaiola, sel: &[Elem], tipo: TipoNo, peso: f64) -> Result<(), String> {
    for e in sel {
        match *e {
            Elem::No(n) => g.definir_tipo_no(n, tipo, peso)?,
            Elem::Aresta(a, b) => {
                g.definir_vinco(a, b, if tipo == TipoNo::Suave { 0. } else if tipo == TipoNo::Canto { 1. } else { peso });
                for n in [a, b] {
                    if let Some(x) = g.no_mut(n) {
                        x.canto = tipo == TipoNo::Canto;
                    }
                }
            }
            Elem::Face(_) => {}
        }
    }
    Ok(())
}

/// Tipo que o chip mostra: o do primeiro elemento que tem um.
pub fn tipo_da_selecao(g: &Gaiola, sel: &[Elem]) -> Option<(TipoNo, f64)> {
    sel.iter().find_map(|e| match *e {
        Elem::No(n) => {
            let peso = g.vincos.iter().find(|v| v.a == n || v.b == n).map_or(0., |v| v.peso);
            Some((g.tipo_no(n), peso))
        }
        Elem::Aresta(a, b) => {
            let w = g.peso(a, b);
            let canto = g.no(a).is_some_and(|x| x.canto) && g.no(b).is_some_and(|x| x.canto);
            Some((if canto { TipoNo::Canto } else if w > 0. { TipoNo::Vinco } else { TipoNo::Suave }, w))
        }
        Elem::Face(_) => None,
    })
}

/// Chip Simetria: liga no plano que passa pelo centro da forma, ou desliga.
pub fn alternar_simetria(g: &mut Gaiola, eixo: u8) -> Result<(), String> {
    if g.simetria.is_some() {
        g.desligar_simetria();
        return Ok(());
    }
    let n = g.nos.len().max(1) as f64;
    let c = g.nos.iter().fold(0., |a, x| a + x.p[eixo as usize]) / n;
    g.ligar_simetria(Simetria { eixo, pos: c })
}

/// Acima disto a conversão merece aviso antes de começar.
pub const SEGUNDOS_DE_ALERTA: f64 = 10.;

/// Remendos, regiões e o tempo esperado da conversão, medidos numa caixa de 384 remendos e 8 regiões:
/// cada remendo custa perto de 1,5 ms e cada região a preencher, 200 ms em G1 e 1,2 s em G2.
pub fn custo_da_conversao(g: &Gaiola, continuidade: u8) -> (usize, usize, f64) {
    let inteira = g.inteira();
    let remendos = fenkra_core::subd_brep::remendos_estimados(&inteira);
    let regioes = inteira.nos_irregulares().len() + inteira.faces.iter().filter(|f| f.nos.len() != 4).count();
    let por_regiao = [0.002, 0.2, 1.2][continuidade.min(2) as usize];
    (remendos, regioes, remendos as f64 * 0.0015 + regioes as f64 * por_regiao)
}

/// Nomes de face que existiam e sumiram: quem apontava para eles perdeu a referência.
pub fn nomes_perdidos(antes: &[i32], agora: &[i32]) -> usize {
    antes.iter().filter(|n| agora.binary_search(n).is_err()).count()
}

/// Fração da aresta sob o mouse: é o deslize do corte em laço.
pub fn fracao_na_aresta(sa: [f32; 2], sb: [f32; 2], m: [f32; 2]) -> f64 {
    dist_seg(m, sa, sb).1 as f64
}

/// Segmentos do laço que o corte faria em `t`, para a prévia.
pub fn previa_do_laco(g: &Gaiola, a: u32, b: u32, t: f64) -> Vec<[V3; 2]> {
    let Ok((arestas, faces)) = g.laco_de(a, b) else { return vec![] };
    let ponto = |x: u32, y: u32| g.pos(x).zip(g.pos(y)).map(|(p, q)| add(mul(p, 1. - t), mul(q, t)));
    let mut out = vec![];
    for w in arestas.windows(2) {
        if let (Some(p), Some(q)) = (ponto(w[0].0, w[0].1), ponto(w[1].0, w[1].1)) {
            out.push([p, q]);
        }
    }
    // laço fechado: a última aresta volta na primeira
    if faces.len() == arestas.len() {
        if let (Some(&l), Some(&f)) = (arestas.last(), arestas.first()) {
            if let (Some(p), Some(q)) = (ponto(l.0, l.1), ponto(f.0, f.1)) {
                out.push([p, q]);
            }
        }
    }
    out
}

/// Normal girada pelo arraste no disco de tangência: `dx` e `dy` são radianos em volta do alto e da
/// direita da tela.
pub fn inclinar(n: V3, direita: V3, alto: V3, dx: f64, dy: f64) -> V3 {
    let gira = |v: V3, k: V3, a: f64| {
        let (s, c) = a.sin_cos();
        add(add(mul(v, c), mul(cross(k, v), s)), mul(k, dot(k, v) * (1. - c)))
    };
    unit(gira(gira(n, alto, dx), direita, dy)).unwrap_or(n)
}

#[cfg(test)]
mod testes {
    use super::*;

    /// Vista de cima, em que x e y do modelo são a tela e o mouse fala em milímetros.
    fn de_cima(g: &Gaiola, m: [f32; 2]) -> Option<Elem> {
        let projetar = |p: V3| Some([p[0] as f32, p[1] as f32]);
        let de_frente = |_: V3, n: V3| n[2] > 0.01;
        let profundidade = |p: V3| 100. - p[2];
        escolher(g, &Tela { projetar: &projetar, de_frente: &de_frente, profundidade: &profundidade }, m)
    }

    fn no_em(g: &Gaiola, p: V3) -> u32 {
        g.nos.iter().find(|n| n.p == p).expect("nó").id
    }

    #[test]
    fn mouse_acende_no_aresta_e_face_sem_trocar_de_ferramenta() {
        let g = Gaiola::caixa([0.; 3], [100., 100., 100.], [2, 2, 2]);
        assert_eq!(de_cima(&g, [1., 1.]), Some(Elem::No(no_em(&g, [0., 0., 50.]))));
        let e = de_cima(&g, [25., 2.]).unwrap();
        assert!(matches!(e, Elem::Aresta(..)), "{e:?}");
        assert!(matches!(de_cima(&g, [25., 25.]), Some(Elem::Face(_))));
        assert_eq!(de_cima(&g, [90., 90.]), None);
    }

    #[test]
    fn no_atras_da_peca_nao_acende() {
        let g = Gaiola::caixa([0.; 3], [100., 100., 100.], [2, 2, 2]);
        // o nó do fundo está no mesmo lugar da tela; quem ganha é o da frente
        assert_eq!(g.pos(no_em(&g, [0., 0., 50.])).unwrap()[2], 50.);
        let Some(Elem::No(n)) = de_cima(&g, [0., 0.]) else { panic!() };
        assert!(g.pos(n).unwrap()[2] > 0.);
    }

    #[test]
    fn ciclo_do_tab_e_dos_espacos() {
        let mut m = ModoVista::Ambos;
        let mut vistos = vec![];
        for _ in 0..3 {
            m = m.proximo();
            vistos.push(m);
        }
        assert_eq!(vistos, [ModoVista::Gaiola, ModoVista::Liso, ModoVista::Ambos]);
    }

    #[test]
    fn espacos_dao_ternos_direitos_e_a_normal_segue_a_face() {
        let g = Gaiola::caixa([0.; 3], [10., 10., 10.], [1, 1, 1]);
        let f = g.faces.iter().find(|f| g.normal_face(f.id).unwrap()[0] > 0.9).unwrap().id;
        let vista = [[1., 0., 0.], [0., 0., 1.], [0., -1., 0.]];
        for e in Espaco::TODOS {
            let ax = eixos(&g, &[Elem::Face(f)], e, vista);
            let z = cross(ax[0], ax[1]);
            assert!(dot(z, ax[2]) > 0.999, "{e:?}");
            if matches!(e, Espaco::Normal | Espaco::Local) {
                assert!(ax[2][0] > 0.999, "{e:?}: {ax:?}");
            }
        }
    }

    #[test]
    fn mover_parte_da_base_e_nao_acumula() {
        let mut g = Gaiola::caixa([0.; 3], [10.; 3], [2, 2, 2]);
        let base = g.clone();
        let id = no_em(&g, [0., 0., 5.]);
        for z in [1., 2., 3.] {
            mover(&mut g, &base, &[id], [0., 0., z]);
        }
        assert_eq!(g.pos(id).unwrap()[2], 8.);
    }

    #[test]
    fn no_da_costura_nao_sai_do_plano() {
        let mut g = Gaiola::caixa([0.; 3], [10.; 3], [2, 2, 2]);
        alternar_simetria(&mut g, 0).unwrap();
        let id = no_em(&g, [0., 0., 5.]);
        let base = g.clone();
        mover(&mut g, &base, &[id], [3., 0., 2.]);
        assert_eq!(g.pos(id).unwrap(), [0., 0., 7.]);
        girar(&mut g, &base, &[id], [0.; 3], [0., 1., 0.], 30.);
        assert_eq!(g.pos(id).unwrap()[0], 0.);
    }

    #[test]
    fn extrudar_com_alt_abre_a_face_pelo_arraste() {
        let mut g = Gaiola::caixa([0.; 3], [10.; 3], [1, 1, 1]);
        let f = g.faces.iter().find(|f| g.normal_face(f.id).unwrap()[2] > 0.9).unwrap().id;
        let sel = [Elem::Face(f)];
        let antes = g.faces.len();
        comecar_extrusao(&mut g, &sel).unwrap();
        assert_eq!(g.faces.len(), antes + 4);
        let base = g.clone();
        let nos = nos_de(&g, &sel);
        mover(&mut g, &base, &nos, [0., 0., 6.]);
        assert_eq!(g.centro_face(f).unwrap()[2], 11.);
        assert!(g.lisa(2).unwrap().volume() > 0.);
    }

    #[test]
    fn extrudar_na_costura_nao_faz_parede_no_plano() {
        let mut g = Gaiola::caixa([0.; 3], [10.; 3], [2, 2, 2]);
        alternar_simetria(&mut g, 0).unwrap();
        // face de cima que encosta no plano x = 0
        let f = g.faces.iter().find(|f| g.normal_face(f.id).unwrap()[2] > 0.9).unwrap().id;
        let sel = [Elem::Face(f)];
        comecar_extrusao(&mut g, &sel).unwrap();
        let base = g.clone();
        let nos = nos_de(&g, &sel);
        mover(&mut g, &base, &nos, [0., 0., 6.]);
        let inteira = g.inteira();
        let (t, _) = inteira.topologia().expect("a metade espelhada não pode criar aresta com três faces");
        assert!(t.gemea.iter().all(|x| *x != u32::MAX), "ficou buraco na costura");
    }

    #[test]
    fn chip_em_aresta_e_no() {
        let mut g = Gaiola::caixa([0.; 3], [10.; 3], [2, 2, 2]);
        let (a, b) = g.arestas()[0];
        aplicar_tipo(&mut g, &[Elem::Aresta(a, b)], TipoNo::Vinco, 0.6).unwrap();
        assert_eq!(tipo_da_selecao(&g, &[Elem::Aresta(a, b)]), Some((TipoNo::Vinco, 0.6)));
        aplicar_tipo(&mut g, &[Elem::Aresta(a, b)], TipoNo::Canto, 0.6).unwrap();
        assert_eq!(tipo_da_selecao(&g, &[Elem::Aresta(a, b)]).unwrap().0, TipoNo::Canto);
        aplicar_tipo(&mut g, &[Elem::Aresta(a, b)], TipoNo::Suave, 0.6).unwrap();
        assert_eq!(tipo_da_selecao(&g, &[Elem::Aresta(a, b)]).unwrap().0, TipoNo::Suave);
        assert!(tipo_da_selecao(&g, &[Elem::Face(g.faces[0].id)]).is_none());
    }

    #[test]
    fn previa_do_laco_fecha_a_volta() {
        let g = Gaiola::caixa([0.; 3], [10.; 3], [1, 1, 1]);
        let f = &g.faces[0];
        let s = previa_do_laco(&g, f.nos[0], f.nos[1], 0.5);
        assert_eq!(s.len(), 4);
        // e cada ponta bate com o começo do próximo
        for i in 0..4 {
            assert_eq!(s[i][1], s[(i + 1) % 4][0]);
        }
    }

    #[test]
    fn malha_densa_avisa_e_caixa_pequena_nao() {
        let pequena = Gaiola::caixa([0.; 3], [10.; 3], [2, 2, 2]);
        assert!(custo_da_conversao(&pequena, 1).2 < SEGUNDOS_DE_ALERTA);
        let densa = Gaiola::caixa([0.; 3], [10.; 3], [20, 20, 20]);
        let (remendos, regioes, tempo) = custo_da_conversao(&densa, 1);
        assert_eq!(regioes, 8);
        assert_eq!(remendos, 2400 * 16);
        assert!(tempo > SEGUNDOS_DE_ALERTA, "{tempo}");
    }

    #[test]
    fn nomes_perdidos_conta_so_o_que_sumiu() {
        assert_eq!(nomes_perdidos(&[1, 2, 3], &[1, 3, 4]), 1);
        assert_eq!(nomes_perdidos(&[], &[1]), 0);
    }

    #[test]
    fn deslize_segue_o_mouse_pela_aresta() {
        assert!((fracao_na_aresta([0., 0.], [100., 0.], [30., 12.]) - 0.3).abs() < 1e-6);
    }

    #[test]
    fn tangencia_pelo_disco_inclina_a_normal() {
        let n = inclinar([0., 0., 1.], [1., 0., 0.], [0., 1., 0.], 0.2, 0.);
        assert!(n[0] > 0.1 && n[2] < 1.);
        assert!((dot(n, n) - 1.).abs() < 1e-9);
    }

    /// Aceite do B153: da caixa até a cúpula com vinco e simetria em menos de 10 ações, sem painel.
    /// Cada linha numerada é um gesto: um arraste, um clique ou um chip.
    #[test]
    fn da_caixa_a_cupula_em_menos_de_dez_acoes() {
        // 1: arrasta a caixa no plano
        let mut g = Gaiola::caixa([0.; 3], [60., 60., 30.], [2, 2, 2]);
        let volume_caixa = g.lisa(3).unwrap().volume();
        // 2: chip Simetria
        alternar_simetria(&mut g, 0).unwrap();
        // 3: clica no nó do topo (mouse sobre ele)
        let Some(Elem::No(topo)) = de_cima(&g, [0., 0.]) else { panic!("o topo não acendeu") };
        let sel = [Elem::No(topo)];
        // 4: arrasta a seta Z
        let base = g.clone();
        let (nos, d) = (nos_de(&g, &sel), deslocamento(eixos(&g, &sel, Espaco::Mundo, [[0.; 3]; 3]), [0., 0., 40.]));
        mover(&mut g, &base, &nos, d);
        // 5: clica numa aresta da base
        let (a, b) = g.arestas().into_iter().find(|(a, b)| g.pos(*a).unwrap()[2] < -14. && g.pos(*b).unwrap()[2] < -14. && g.pos(*a).unwrap()[1].abs() > 29. && g.pos(*b).unwrap()[1].abs() > 29.).unwrap();
        let aresta = [Elem::Aresta(a, b)];
        // 6: chip Vinco
        aplicar_tipo(&mut g, &aresta, TipoNo::Vinco, 1.).unwrap();

        let m = g.lisa(3).unwrap();
        let (lo, hi) = m.caixa().unwrap();
        assert!((lo[0] + hi[0]).abs() < 1e-9, "simétrica em X");
        assert!(hi[2] > 20., "o topo subiu: {hi:?}");
        assert!(m.volume() > volume_caixa, "a cúpula tem mais volume que a caixa lisa");
        assert_eq!(g.vincos.len(), 1);
        assert!(g.inteira().vincos.len() == 2, "o vinco vale nas duas metades");
    }
}
