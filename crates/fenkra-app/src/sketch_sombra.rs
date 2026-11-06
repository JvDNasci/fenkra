// SPDX-License-Identifier: LGPL-2.1-or-later
//! Perfil sombreado e o que o sketch mostra (B274).
//!
//! O sombreamento sai de `SketchData::loops`, a mesma leitura de contornos que a extrusão usa: se a
//! área está pintada, o contorno está fechado. Tudo acontece sobre os dados do sketch, sem passar pelo
//! recálculo da peça, por isso aparece no mesmo quadro em que o último segmento fecha.
//!
//! Os contornos viram faixas horizontais pela regra par-ímpar. Isso resolve contornos côncavos e furos
//! de uma vez: furo é a região com número par de bordas à esquerda, igual ao que o perfil faz no sólido.

use crate::sketch_ui::Ent;
use eframe::egui::{self, epaint, Color32, Pos2};
use fenkra_core::sketch::{LinkShape, Seg};
use fenkra_core::{Id, SketchData};
use std::collections::HashSet;

type P = [f64; 2];

/// O que aparece no sketch em edição. Só muda a tela: nada disso entra no documento.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Exibir {
    pub perfil: bool,
    pub pontos: bool,
    pub cotas: bool,
    pub restricoes: bool,
    pub construcao: bool,
    pub projetadas: bool,
}

impl Default for Exibir {
    fn default() -> Self {
        Exibir { perfil: true, pontos: true, cotas: true, restricoes: true, construcao: true, projetadas: true }
    }
}

/// Curvas e pontos que a tela não mostra: também não são clicados nem atraem snap.
pub fn ocultos(sk: &SketchData, e: &Exibir) -> (HashSet<Id>, HashSet<Id>) {
    let (ext_curvas, ext_pontos) = if e.projetadas { (HashSet::new(), HashSet::new()) } else { externas(sk) };
    let curvas: HashSet<Id> = sk.curves.iter().filter(|c| (!e.construcao && c.construction) || ext_curvas.contains(&c.id)).map(|c| c.id).collect();
    let de_visivel: HashSet<Id> = sk.curves.iter().filter(|c| !curvas.contains(&c.id)).flat_map(|c| SketchData::curve_points(&c.geom)).collect();
    let de_oculta = sk.curves.iter().filter(|c| curvas.contains(&c.id)).flat_map(|c| SketchData::curve_points(&c.geom)).chain(ext_pontos).filter(|p| !de_visivel.contains(p));
    let pontos: HashSet<Id> = if e.pontos { de_oculta.collect() } else { sk.points.iter().map(|p| p.id).collect() };
    (curvas, pontos)
}

/// Curvas e pontos que vieram do 3D pelos vínculos.
fn externas(sk: &SketchData) -> (HashSet<Id>, HashSet<Id>) {
    let (mut curvas, mut pontos) = (HashSet::new(), HashSet::new());
    for l in &sk.links {
        match &l.shape {
            LinkShape::Poly { points, .. } => pontos.extend(points.iter().copied()),
            LinkShape::Circle { center, curve } => {
                curvas.insert(*curve);
                pontos.insert(*center);
            }
            LinkShape::Line { a, b, curve } => {
                curvas.insert(*curve);
                pontos.extend([*a, *b]);
            }
            LinkShape::Arc { center, s, e, curve } | LinkShape::Ellipse { center, s, e, curve } => {
                curvas.insert(*curve);
                pontos.extend([*center, *s, *e]);
            }
        }
    }
    // a polilinha não guarda as curvas: são as que só usam pontos dela
    curvas.extend(sk.curves.iter().filter(|c| SketchData::curve_points(&c.geom).iter().all(|p| pontos.contains(p))).map(|c| c.id));
    (curvas, pontos)
}

fn circulo(c: P, r: f64) -> Vec<P> {
    (0..72).map(|i| i as f64 / 72. * std::f64::consts::TAU).map(|t| [c[0] + r * t.cos(), c[1] + r * t.sin()]).collect()
}

/// Arco pelos três pontos, sem repetir o último.
fn arco(a: P, m: P, b: P) -> Vec<P> {
    let d = 2. * (a[0] * (m[1] - b[1]) + m[0] * (b[1] - a[1]) + b[0] * (a[1] - m[1]));
    if d.abs() < 1e-12 {
        return vec![a];
    }
    let q = |p: P| p[0] * p[0] + p[1] * p[1];
    let c = [
        (q(a) * (m[1] - b[1]) + q(m) * (b[1] - a[1]) + q(b) * (a[1] - m[1])) / d,
        (q(a) * (b[0] - m[0]) + q(m) * (a[0] - b[0]) + q(b) * (m[0] - a[0])) / d,
    ];
    let r = (a[0] - c[0]).hypot(a[1] - c[1]);
    let ang = |p: P| (p[1] - c[1]).atan2(p[0] - c[0]);
    let tau = std::f64::consts::TAU;
    let (a0, am, a1) = (ang(a), ang(m), ang(b));
    // o sentido é o que passa pelo ponto do meio
    let antes = (am - a0).rem_euclid(tau) < (a1 - a0).rem_euclid(tau);
    let varre = if antes { (a1 - a0).rem_euclid(tau) } else { -(a0 - a1).rem_euclid(tau) };
    (0..32).map(|i| a0 + varre * i as f64 / 32.).map(|t| [c[0] + r * t.cos(), c[1] + r * t.sin()]).collect()
}

/// Contorno fechado como polígono, sem repetir o primeiro ponto.
pub(crate) fn poligono(laco: &[Seg]) -> Vec<P> {
    let mut v: Vec<P> = vec![];
    for s in laco {
        match s {
            Seg::Line(a, _) => v.push(*a),
            Seg::Arc(a, m, b) => v.extend(arco(*a, *m, *b)),
            Seg::Circle(c, r) => v.extend(circulo(*c, *r)),
            Seg::Spline(p) => v.extend(p.iter().take(p.len().saturating_sub(1))),
            Seg::Ellipse(c, m, r, t0, t1) => {
                let p = fenkra_core::projecao::elipse_amostras(*c, *m, *r, *t0, *t1);
                v.extend(p.iter().take(p.len().saturating_sub(1)));
            }
        }
    }
    v
}

/// Regiões fechadas do sketch em faixas horizontais: cada uma é um trapézio (esq. e dir. embaixo,
/// dir. e esq. em cima). Curvas de construção não fazem perfil, como em `loops`.
pub fn faixas(sk: &SketchData) -> Vec<[P; 4]> {
    let (lacos, _) = sk.loops();
    let polis: Vec<Vec<P>> = lacos.iter().map(|l| poligono(l)).filter(|p| p.len() >= 3).collect();
    faixas_de(&polis)
}

fn faixas_de(polis: &[Vec<P>]) -> Vec<[P; 4]> {
    let mut lados: Vec<(P, P)> = vec![];
    for p in polis {
        for (i, a) in p.iter().enumerate() {
            let b = p[(i + 1) % p.len()];
            // lado horizontal não cruza nenhuma faixa
            if a[1] < b[1] {
                lados.push((*a, b));
            } else if a[1] > b[1] {
                lados.push((b, *a));
            }
        }
    }
    let mut ys: Vec<f64> = lados.iter().flat_map(|l| [l.0[1], l.1[1]]).collect();
    ys.sort_by(f64::total_cmp);
    ys.dedup();
    let x_em = |l: &(P, P), y: f64| l.0[0] + (l.1[0] - l.0[0]) * (y - l.0[1]) / (l.1[1] - l.0[1]);
    let mut v = vec![];
    for w in ys.windows(2) {
        let (y0, y1) = (w[0], w[1]);
        let meio = (y0 + y1) / 2.;
        let mut corta: Vec<&(P, P)> = lados.iter().filter(|l| l.0[1] <= y0 && l.1[1] >= y1).collect();
        corta.sort_by(|a, b| x_em(a, meio).total_cmp(&x_em(b, meio)));
        for par in corta.chunks_exact(2) {
            v.push([[x_em(par[0], y0), y0], [x_em(par[1], y0), y0], [x_em(par[1], y1), y1], [x_em(par[0], y1), y1]]);
        }
    }
    v
}

/// O ponto está dentro do polígono? Par-ímpar, a mesma regra que pinta as faixas.
fn dentro(q: &[P], p: P) -> bool {
    let mut d = false;
    for (i, a) in q.iter().enumerate() {
        let b = q[(i + 1) % q.len()];
        if (a[1] > p[1]) != (b[1] > p[1]) && p[0] < a[0] + (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) {
            d = !d;
        }
    }
    d
}

fn area_de(q: &[P]) -> f64 {
    let dobro: f64 = q.iter().enumerate().map(|(i, a)| a[0] * q[(i + 1) % q.len()][1] - q[(i + 1) % q.len()][0] * a[1]).sum();
    dobro.abs() / 2.
}

/// Distância do ponto à borda do polígono.
fn ate_a_borda(q: &[P], p: P) -> f64 {
    let lado = |a: P, b: P| {
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let t = (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / (dx * dx + dy * dy).max(1e-30)).clamp(0., 1.);
        (a[0] + t * dx - p[0]).hypot(a[1] + t * dy - p[1])
    };
    q.iter().enumerate().map(|(i, a)| lado(*a, q[(i + 1) % q.len()])).fold(f64::MAX, f64::min)
}

/// `b` entra em `a`? Basta um vértice bem dentro: laço que só encosta num canto fica de fora, e laço
/// que cruza a borda de `a` conta, porque também limita a região.
fn entra(a: &[P], b: &[P]) -> bool {
    b.iter().any(|p| dentro(a, *p) && ate_a_borda(a, *p) > 1e-7)
}

/// Laços que limitam a região sob `p`, pelos índices em `polis`. Fora da sombra não há nenhum.
///
/// A sombra é par-ímpar: `p` está nela quando um número ímpar de laços o cerca. O menor deles é a borda
/// de fora da região, e os laços logo abaixo dele são os furos. O que está dentro de um furo é outra
/// região: três quadrados encaixados dão duas regiões, a de fora com os dois maiores e a do meio só
/// com o menor.
fn regiao_de(polis: &[Vec<P>], p: P) -> Vec<usize> {
    let cercam: Vec<usize> = (0..polis.len()).filter(|i| dentro(&polis[*i], p)).collect();
    if cercam.len() % 2 == 0 {
        return vec![];
    }
    let Some(borda) = cercam.iter().copied().min_by(|a, b| area_de(&polis[*a]).total_cmp(&area_de(&polis[*b]))) else { return vec![] };
    let abaixo: Vec<usize> = (0..polis.len()).filter(|i| !cercam.contains(i) && entra(&polis[borda], &polis[*i])).collect();
    let furo = |i: &usize| !abaixo.iter().any(|j| j != i && area_de(&polis[*j]) > area_de(&polis[*i]) && entra(&polis[*j], &polis[*i]));
    let mut v = vec![borda];
    v.extend(abaixo.iter().copied().filter(|i| furo(i)));
    v
}

/// Curvas que formam a região sombreada sob `uv`, com os laços internos. Sai dos mesmos laços que
/// `faixas` pinta, então só existe região onde existe sombra.
pub fn regiao_em(sk: &SketchData, uv: P) -> Vec<Id> {
    let lacos: Vec<(Vec<P>, Vec<Id>)> = sk.loops_com_curvas().into_iter().map(|(l, c)| (poligono(&l), c)).filter(|l| l.0.len() >= 3).collect();
    let polis: Vec<Vec<P>> = lacos.iter().map(|l| l.0.clone()).collect();
    regiao_de(&polis, uv).into_iter().flat_map(|i| lacos[i].1.iter().copied()).collect()
}

/// Clique fora de qualquer curva: a região sob o cursor troca a seleção, Shift soma e Ctrl tira. Sem
/// região, o clique limpo esvazia a seleção e o com tecla não muda nada.
pub fn clique_na_regiao(sel: &mut Vec<Ent>, regiao: &[Id], shift: bool, ctrl: bool) {
    if ctrl {
        sel.retain(|e| !matches!(e, Ent::Cv(c) if regiao.contains(c)));
    } else if shift {
        for e in regiao.iter().copied().map(Ent::Cv) {
            if !sel.contains(&e) {
                sel.push(e);
            }
        }
    } else {
        *sel = regiao.iter().copied().map(Ent::Cv).collect();
    }
}

/// Pinta as faixas num desenho só, sem emendas translúcidas sobrepostas.
pub fn desenha(painter: &egui::Painter, tela: &dyn Fn(P) -> Pos2, f: &[[P; 4]], cor: Color32) {
    let mut m = epaint::Mesh::default();
    for q in f {
        let i = m.vertices.len() as u32;
        for p in q {
            m.colored_vertex(tela(*p), cor);
        }
        m.add_triangle(i, i + 1, i + 2);
        m.add_triangle(i, i + 2, i + 3);
    }
    painter.add(egui::Shape::mesh(m));
}

/// Plano que corta os corpos enquanto se desenha, para o renderizador: tira a metade do lado do olho
/// e deixa à mostra a seção no plano do sketch. É só um uniforme do desenho: o modelo não muda.
pub fn plano_de_corte(f: &fenkra_kernel::Frame, olho: [f64; 3]) -> [f32; 4] {
    let d = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let lado = if d(f.normal, olho) >= d(f.normal, f.origin) { 1. } else { -1. };
    let n = f.normal.map(|x| x * lado);
    [n[0] as f32, n[1] as f32, n[2] as f32, d(n, f.origin) as f32]
}

/// Caixas do que mostrar e a opção de fatiar. Vale só enquanto o sketch está aberto.
pub fn painel(ui: &mut egui::Ui, e: &mut Exibir, fatiar: &mut bool) {
    use crate::i18n::tr;
    egui::CollapsingHeader::new(tr("Mostrar")).id_salt("sk_mostrar").default_open(false).show(ui, |ui| {
        for (v, nome, dica) in [
            (&mut e.perfil, "Perfil", "Sombreia os contornos fechados, os mesmos que a extrusão usa."),
            (&mut e.pontos, "Pontos", "Mostra os pontos do sketch. Oculto, ponto não recebe clique."),
            (&mut e.cotas, "Cotas", "Mostra as cotas. Elas continuam valendo quando ocultas."),
            (&mut e.restricoes, "Restrições", "Mostra os glifos das restrições. Elas continuam valendo quando ocultas."),
            (&mut e.construcao, "Construção", "Mostra as curvas de construção. Oculta, curva não recebe clique nem atrai snap."),
            (&mut e.projetadas, "Projetadas", "Mostra a geometria que veio do 3D. Oculta, curva não recebe clique nem atrai snap."),
        ] {
            ui.checkbox(v, tr(nome)).on_hover_text(tr(dica));
        }
        ui.separator();
        ui.checkbox(fatiar, tr("Fatiar no plano")).on_hover_text(tr("Corta os corpos no plano do sketch enquanto ele está aberto. O modelo e o arquivo não mudam."));
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use fenkra_core::sketch::Curve;

    fn ids() -> impl FnMut() -> Id {
        let mut n = 100;
        move || {
            n += 1;
            Id(n)
        }
    }

    fn area(f: &[[P; 4]]) -> f64 {
        f.iter().map(|q| ((q[1][0] - q[0][0]) + (q[2][0] - q[3][0])) / 2. * (q[3][1] - q[0][1])).sum()
    }

    /// Fechar o último segmento sombreia na hora, só com os dados do sketch.
    #[test]
    fn fechar_o_ultimo_segmento_sombreia() {
        let mut al = ids();
        let mut s = SketchData::default();
        let p: Vec<Id> = [[0., 0.], [30., 0.], [30., 20.], [0., 20.]].iter().map(|q| s.add_point(&mut al, q[0], q[1])).collect();
        for i in 0..3 {
            s.add_curve(&mut al, Curve::Line { a: p[i], b: p[i + 1] }, false);
        }
        assert!(faixas(&s).is_empty(), "cadeia aberta não tem o que pintar");
        s.add_curve(&mut al, Curve::Line { a: p[3], b: p[0] }, false);
        assert!((area(&faixas(&s)) - 600.).abs() < 1e-9);
    }

    #[test]
    fn furo_fica_vazio_e_concavo_nao_transborda() {
        let mut al = ids();
        let mut s = SketchData::default();
        s.add_rect(&mut al, [0., 0.], [40., 40.], false);
        let c = s.add_point(&mut al, 20., 20.);
        s.add_curve(&mut al, Curve::Circle { c, r: 10. }, false);
        let pi = std::f64::consts::PI;
        assert!((area(&faixas(&s)) - (1600. - pi * 100.)).abs() < 2., "{}", area(&faixas(&s)));

        // perfil em L: 30x30 sem o canto de 15x15
        let l: Vec<P> = vec![[0., 0.], [30., 0.], [30., 15.], [15., 15.], [15., 30.], [0., 30.]];
        assert!((area(&faixas_de(&[l])) - 675.).abs() < 1e-9);
    }

    #[test]
    fn arco_do_contorno_vai_pelo_ponto_do_meio() {
        // meio círculo de raio 10 fechado por uma linha, nos dois sentidos
        for (a, b, m) in [([10., 0.], [-10., 0.], [0., 10.]), ([-10., 0.], [10., 0.], [0., 10.]), ([10., 0.], [-10., 0.], [0., -10.])] {
            // `arco` não repete o último ponto: no contorno, ele vem do segmento seguinte
            let mut p = arco(a, m, b);
            p.push(b);
            let f = faixas_de(&[p]);
            let meio_circulo = std::f64::consts::PI * 50.;
            assert!((area(&f) - meio_circulo).abs() < 1., "{a:?} {m:?}: {}", area(&f));
        }
    }

    /// O corte tira o lado do olho, dos dois lados do plano: o que fica é o que está atrás do sketch.
    #[test]
    fn fatia_tira_a_metade_do_lado_do_olho() {
        let f = fenkra_kernel::Frame { origin: [0., 0., 10.], x: [1., 0., 0.], normal: [0., 0., 1.] };
        let cortado = |p: [f32; 4], z: f32| p[2] * z > p[3];
        let de_cima = plano_de_corte(&f, [0., 0., 100.]);
        assert!(cortado(de_cima, 50.) && !cortado(de_cima, 0.));
        let de_baixo = plano_de_corte(&f, [0., 0., -100.]);
        assert!(cortado(de_baixo, 0.) && !cortado(de_baixo, 50.));
    }

    /// Mostrar e fatiar são estado da tela: não existem no sketch que vai para o arquivo.
    #[test]
    fn ver_de_outro_jeito_nao_muda_o_arquivo() {
        let mut al = ids();
        let mut s = SketchData::default();
        s.add_rect(&mut al, [0., 0.], [10., 10.], false);
        let antes = serde_json::to_string(&s).unwrap();
        let e = Exibir { perfil: false, pontos: false, cotas: false, restricoes: false, construcao: false, projetadas: false };
        let _ = ocultos(&s, &e);
        let _ = faixas(&s);
        assert_eq!(serde_json::to_string(&s).unwrap(), antes);
    }

    /// Quadrado de lado 2m centrado em (c, c), como linhas ligadas.
    fn quadrado(s: &mut SketchData, al: &mut impl FnMut() -> Id, c: f64, m: f64) -> Vec<Id> {
        s.add_rect(al, [c - m, c - m], [c + m, c + m], false).1.to_vec()
    }

    fn mesmas(mut a: Vec<Id>, b: &[&[Id]]) -> bool {
        let mut e: Vec<Id> = b.concat();
        a.sort();
        e.sort();
        a == e
    }

    #[test]
    fn quadrado_dentro_de_quadrado_seleciona_os_dois() {
        let mut al = ids();
        let mut s = SketchData::default();
        let fora = quadrado(&mut s, &mut al, 0., 40.);
        let dentro = quadrado(&mut s, &mut al, 0., 15.);
        assert!(mesmas(regiao_em(&s, [30., 0.]), &[&fora, &dentro]));
        assert!(regiao_em(&s, [0., 0.]).is_empty(), "o furo não tem sombra");
        assert!(regiao_em(&s, [90., 0.]).is_empty(), "fora do perfil não há região");
    }

    #[test]
    fn encaixe_triplo_separa_as_regioes() {
        let mut al = ids();
        let mut s = SketchData::default();
        let a = quadrado(&mut s, &mut al, 0., 40.);
        let b = quadrado(&mut s, &mut al, 0., 25.);
        let c = quadrado(&mut s, &mut al, 0., 10.);
        assert!(mesmas(regiao_em(&s, [30., 0.]), &[&a, &b]), "o menor pertence à região de dentro");
        assert!(regiao_em(&s, [18., 0.]).is_empty(), "entre o segundo e o terceiro é furo");
        assert!(mesmas(regiao_em(&s, [0., 0.]), &[&c]));
    }

    #[test]
    fn dois_furos_entram_e_vizinho_fica_de_fora() {
        let mut al = ids();
        let mut s = SketchData::default();
        let fora = quadrado(&mut s, &mut al, 0., 40.);
        let furo = quadrado(&mut s, &mut al, -20., 8.);
        let c = s.add_point(&mut al, 20., 20.);
        let circ = s.add_curve(&mut al, Curve::Circle { c, r: 8. }, false);
        let vizinho = quadrado(&mut s, &mut al, 100., 10.);
        assert!(mesmas(regiao_em(&s, [0., 0.]), &[&fora, &furo, &[circ]]));
        assert!(mesmas(regiao_em(&s, [100., 100.]), &[&vizinho]));
    }

    #[test]
    fn laco_que_so_encosta_no_canto_nao_e_furo() {
        let polis = vec![vec![[0., 0.], [10., 0.], [10., 10.], [0., 10.]], vec![[10., 10.], [20., 10.], [20., 20.], [10., 20.]]];
        assert_eq!(regiao_de(&polis, [5., 5.]), vec![0]);
    }

    #[test]
    fn construcao_e_cadeia_aberta_nao_fazem_regiao() {
        let mut al = ids();
        let mut s = SketchData::default();
        s.add_rect(&mut al, [0., 0.], [10., 10.], true);
        let p: Vec<Id> = [[20., 0.], [30., 0.], [30., 10.]].iter().map(|q| s.add_point(&mut al, q[0], q[1])).collect();
        s.add_curve(&mut al, Curve::Line { a: p[0], b: p[1] }, false);
        s.add_curve(&mut al, Curve::Line { a: p[1], b: p[2] }, false);
        assert!(regiao_em(&s, [5., 5.]).is_empty());
        assert!(regiao_em(&s, [28., 2.]).is_empty());
    }

    #[test]
    fn regiao_acompanha_o_arco() {
        let mut al = ids();
        let mut s = SketchData::default();
        // meio disco: arco de cima fechado por uma linha
        let (c, a, b) = (s.add_point(&mut al, 0., 0.), s.add_point(&mut al, 10., 0.), s.add_point(&mut al, -10., 0.));
        let arc = s.add_curve(&mut al, Curve::Arc { c, s: a, e: b }, false);
        let lin = s.add_curve(&mut al, Curve::Line { a: b, b: a }, false);
        assert!(mesmas(regiao_em(&s, [0., 5.]), &[&[arc, lin]]));
        assert!(regiao_em(&s, [0., -5.]).is_empty(), "abaixo da linha não há sombra");
        assert!(regiao_em(&s, [9., 9.]).is_empty(), "no canto, fora do arco");
    }

    #[test]
    fn clique_troca_shift_soma_e_ctrl_tira() {
        let (a, b, c) = (Id(1), Id(2), Id(3));
        let mut sel = vec![Ent::Cv(c), Ent::Pt(Id(9))];
        clique_na_regiao(&mut sel, &[a, b], false, false);
        assert_eq!(sel, vec![Ent::Cv(a), Ent::Cv(b)], "clique simples troca a seleção");

        clique_na_regiao(&mut sel, &[b, c], true, false);
        assert_eq!(sel, vec![Ent::Cv(a), Ent::Cv(b), Ent::Cv(c)], "Shift não repete o que já está");

        sel.push(Ent::Pt(Id(9)));
        clique_na_regiao(&mut sel, &[a, b], false, true);
        assert_eq!(sel, vec![Ent::Cv(c), Ent::Pt(Id(9))], "Ctrl tira só as curvas da região");

        clique_na_regiao(&mut sel, &[], true, false);
        clique_na_regiao(&mut sel, &[], false, true);
        assert_eq!(sel.len(), 2, "com tecla, clicar fora da sombra não perde a seleção");
        clique_na_regiao(&mut sel, &[], false, false);
        assert!(sel.is_empty(), "clique limpo fora da sombra esvazia");
    }

    #[test]
    fn construcao_nao_faz_perfil() {
        let mut al = ids();
        let mut s = SketchData::default();
        s.add_rect(&mut al, [0., 0.], [10., 10.], true);
        assert!(faixas(&s).is_empty());
    }

    #[test]
    fn ocultar_tira_de_clique_o_que_nao_aparece() {
        let mut al = ids();
        let mut s = SketchData::default();
        let a = s.add_point(&mut al, 0., 0.);
        let b = s.add_point(&mut al, 10., 0.);
        let c = s.add_point(&mut al, 10., 10.);
        let normal = s.add_curve(&mut al, Curve::Line { a, b }, false);
        let aux = s.add_curve(&mut al, Curve::Line { a: b, b: c }, true);
        assert_eq!(ocultos(&s, &Exibir::default()), (HashSet::new(), HashSet::new()));

        let (curvas, pontos) = ocultos(&s, &Exibir { construcao: false, ..Default::default() });
        assert!(curvas.contains(&aux) && !curvas.contains(&normal));
        assert!(pontos.contains(&c) && !pontos.contains(&b), "ponta compartilhada continua");

        let (_, pontos) = ocultos(&s, &Exibir { pontos: false, ..Default::default() });
        assert_eq!(pontos.len(), 3);
    }
}
