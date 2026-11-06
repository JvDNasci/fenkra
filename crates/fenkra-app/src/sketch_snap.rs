// SPDX-License-Identifier: LGPL-2.1-or-later
//! Snaps com tipo e linhas de inferência do sketch (B96).
//!
//! O snap diz em que a ponta vai se apoiar antes do clique, e um glifo mostra o tipo. O que o glifo
//! promete vira restrição: ponto médio, sobre a curva, interseção. A inferência é mais fraca: alinha
//! o cursor com o ponto de partida ou com outros pontos, mostra a linha tracejada e, na linha que
//! continua outra, lembra a perpendicular ou a paralela escolhida.

use crate::i18n::tr;
use crate::sketch_relimita::{cruzamentos, traco, Traco};
use eframe::egui::{self, Color32, Pos2, Stroke};
use fenkra_core::sketch::{Curve, Kind};
use fenkra_core::{Id, SketchData};

type P = [f64; 2];

/// Dica da opção que guarda o apoio do snap como restrição.
pub const DICA_RESTRINGE: &str = "O ponto que nasce ou é solto sobre o meio, o quadrante ou uma curva fica preso ali. Desligada, o snap só posiciona.";
/// Dica da opção que junta dois pontos ao fim do arraste.
pub const DICA_JUNTAR: &str = "Soltar um ponto sobre outro faz os dois virarem um só. Desligada, ou com Ctrl durante o arraste, ele só fica ali em cima.";
/// Aviso de quando a junção foi desfeita.
pub const NAO_JUNTOU: &str = "Juntar esses pontos deixaria o sketch em conflito: o ponto ficou onde estava";

/// Distância na tela em que um ponto notável atrai o cursor.
pub const RAIO_PONTO: f32 = 10.;
/// Distância na tela em que uma curva atrai o cursor: menor que a dos pontos, que têm preferência.
pub const RAIO_CURVA: f32 = 7.;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum TipoSnap {
    #[default]
    Livre,
    Grade,
    Origem,
    Ponta,
    Centro,
    Meio,
    Quadrante,
    Intersecao,
    SobreCurva,
}

impl TipoSnap {
    pub fn nome(self) -> &'static str {
        match self {
            TipoSnap::Livre => "",
            TipoSnap::Grade => "grade",
            TipoSnap::Origem => "origem",
            TipoSnap::Ponta => "ponta",
            TipoSnap::Centro => "centro",
            TipoSnap::Meio => "ponto médio",
            TipoSnap::Quadrante => "quadrante",
            TipoSnap::Intersecao => "interseção",
            TipoSnap::SobreCurva => "sobre a curva",
        }
    }
}

/// Em que o cursor se apoia.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Achado {
    pub tipo: TipoSnap,
    pub uv: P,
    /// ponto do sketch sob o cursor, se houver
    pub ponto: Option<Id>,
    /// curvas em que o ponto novo se apoia
    pub curvas: [Option<Id>; 2],
}

fn longe(a: Pos2, b: Pos2) -> f32 {
    (a - b).length()
}

fn perto_da_curva(t: &Traco, uv: P, tela: &dyn Fn(P) -> Pos2, cursor: Pos2) -> Option<(f32, P)> {
    let u = t.param(uv);
    // arco: fora do trecho o parâmetro passa de 1, e ali não há curva
    if matches!(t, Traco::Arco { .. }) && u > 1. {
        return None;
    }
    let q = t.em(u.clamp(0., 1.));
    Some((longe(tela(q), cursor), q))
}

/// Ponto notável do sketch sob o cursor. `sem_pontos` e `sem_curvas` ficam de fora (o que está sendo
/// arrastado não pode se apoiar em si mesmo). `extras`: liga os snaps além dos pontos que já existem.
pub fn achar(sk: &SketchData, cursor: Pos2, uv: P, tela: &dyn Fn(P) -> Pos2, sem_pontos: &[Id], sem_curvas: &[Id], extras: bool) -> Option<Achado> {
    // ponto existente: ponta, ou centro quando é o centro de um círculo ou arco
    let mut melhor: Option<(f32, Id)> = None;
    for p in sk.points.iter().filter(|p| !sem_pontos.contains(&p.id)) {
        let d = longe(tela([p.x, p.y]), cursor);
        if d < RAIO_PONTO && melhor.is_none_or(|(b, _)| d < b) {
            melhor = Some((d, p.id));
        }
    }
    if let Some((_, p)) = melhor {
        let centro = sk.curves.iter().any(|c| matches!(c.geom, Curve::Circle { c: k, .. } | Curve::Arc { c: k, .. } if k == p));
        return Some(Achado { tipo: if centro { TipoSnap::Centro } else { TipoSnap::Ponta }, uv: sk.pos(p), ponto: Some(p), curvas: [None; 2] });
    }
    if !extras {
        return None;
    }
    let vizinhas: Vec<(Id, Traco, f32, P)> = sk
        .curves
        .iter()
        .filter(|c| !sem_curvas.contains(&c.id))
        .filter_map(|c| {
            let t = traco(sk, c.id)?;
            let (d, q) = perto_da_curva(&t, uv, tela, cursor)?;
            (d < 3. * RAIO_PONTO).then_some((c.id, t, d, q))
        })
        .collect();
    // interseção: só entre as curvas perto do cursor, senão seria um cruzamento de todas com todas
    let mut cruz: Option<(f32, P, Id, Id)> = None;
    for (i, a) in vizinhas.iter().enumerate() {
        for b in &vizinhas[i + 1..] {
            for q in cruzamentos(&a.1, &b.1) {
                let d = longe(tela(q), cursor);
                if d < RAIO_PONTO && cruz.is_none_or(|x| d < x.0) {
                    cruz = Some((d, q, a.0, b.0));
                }
            }
        }
    }
    if let Some((_, q, a, b)) = cruz {
        return Some(Achado { tipo: TipoSnap::Intersecao, uv: q, ponto: None, curvas: [Some(a), Some(b)] });
    }
    // meio de linha e de arco; quadrantes de círculo e arco
    let mut nota: Option<(f32, P, TipoSnap, Id)> = None;
    for (id, t, ..) in &vizinhas {
        let mut olha = |q: P, tipo: TipoSnap| {
            let d = longe(tela(q), cursor);
            if d < RAIO_PONTO && nota.is_none_or(|x| d < x.0) {
                nota = Some((d, q, tipo, *id));
            }
        };
        match t {
            Traco::Reta { .. } => olha(t.em(0.5), TipoSnap::Meio),
            Traco::Arco { c, r, varre, .. } => {
                if !t.fechado() {
                    olha(t.em(0.5), TipoSnap::Meio);
                }
                for q in [[c[0] + r, c[1]], [c[0], c[1] + r], [c[0] - r, c[1]], [c[0], c[1] - r]] {
                    if t.fechado() || (*varre > 0. && t.param(q) <= 1.) {
                        olha(q, TipoSnap::Quadrante);
                    }
                }
            }
            Traco::Poli { .. } => {}
        }
    }
    if let Some((_, q, tipo, c)) = nota {
        return Some(Achado { tipo, uv: q, ponto: None, curvas: [Some(c), None] });
    }
    vizinhas.into_iter().filter(|v| v.2 < RAIO_CURVA).min_by(|a, b| a.2.total_cmp(&b.2)).map(|v| Achado { tipo: TipoSnap::SobreCurva, uv: v.3, ponto: None, curvas: [Some(v.0), None] })
}

/// Soltar duas vezes no mesmo apoio não repete a restrição: repetida, ela só atrapalha o solver.
fn sem_repetir(sk: &mut SketchData, al: &mut impl FnMut() -> Id, k: Kind) {
    if !sk.constraints.iter().any(|c| c.kind == k) {
        sk.add_constraint(al, k);
    }
}

/// O ponto guarda o apoio do snap como restrição. Spline e elipse não têm equação de ponto sobre a curva.
pub fn vincula(sk: &mut SketchData, al: &mut impl FnMut() -> Id, p: Id, tipo: TipoSnap, curvas: [Option<Id>; 2]) {
    let sobre = |sk: &SketchData, c: Id| !matches!(sk.curve(c).map(|x| &x.geom), Some(Curve::Spline { .. } | Curve::Ellipse { .. }) | None);
    match (tipo, curvas) {
        (TipoSnap::Meio, [Some(c), _]) if matches!(sk.curve(c).map(|x| &x.geom), Some(Curve::Line { .. })) => {
            sem_repetir(sk, al, Kind::Midpoint(p, c));
        }
        (TipoSnap::Meio | TipoSnap::SobreCurva, [Some(c), _]) if sobre(sk, c) => {
            sem_repetir(sk, al, Kind::PointOnCurve(p, c));
        }
        (TipoSnap::Quadrante, [Some(c), _]) => {
            sem_repetir(sk, al, Kind::PointOnCurve(p, c));
            let centro = match sk.curve(c).map(|x| &x.geom) {
                Some(Curve::Circle { c: k, .. } | Curve::Arc { c: k, .. }) => Some(*k),
                _ => None,
            };
            if let Some(k) = centro {
                let (a, b) = (sk.pos(p), sk.pos(k));
                let kind = if (a[0] - b[0]).abs() < (a[1] - b[1]).abs() { Kind::VerticalPts(p, k) } else { Kind::HorizontalPts(p, k) };
                sem_repetir(sk, al, kind);
            }
        }
        (TipoSnap::Intersecao, [a, b]) => {
            for c in [a, b].into_iter().flatten() {
                if sobre(sk, c) {
                    sem_repetir(sk, al, Kind::PointOnCurve(p, c));
                }
            }
        }
        _ => {}
    }
}

/// Desenha o glifo do snap. Cada tipo tem uma forma própria, e o nome vai ao lado.
pub fn desenha_glifo(painter: &egui::Painter, s: Pos2, tipo: TipoSnap, cor: Color32) {
    let st = Stroke::new(1.6, cor);
    let v = egui::vec2;
    let fecha = |p: Vec<Pos2>| painter.add(egui::Shape::closed_line(p, st));
    match tipo {
        TipoSnap::Livre => {
            painter.circle_filled(s, 2.5, cor);
            return;
        }
        TipoSnap::Grade => {
            painter.line_segment([s - v(3., 0.), s + v(3., 0.)], Stroke::new(1., cor));
            painter.line_segment([s - v(0., 3.), s + v(0., 3.)], Stroke::new(1., cor));
            return;
        }
        TipoSnap::Origem => {
            painter.circle_stroke(s, 7., st);
            painter.circle_filled(s, 2., cor);
        }
        TipoSnap::Ponta => {
            fecha(vec![s + v(-6., -6.), s + v(6., -6.), s + v(6., 6.), s + v(-6., 6.)]);
        }
        TipoSnap::Centro => {
            painter.circle_stroke(s, 6., st);
            painter.line_segment([s - v(9., 0.), s + v(9., 0.)], Stroke::new(1., cor));
            painter.line_segment([s - v(0., 9.), s + v(0., 9.)], Stroke::new(1., cor));
        }
        TipoSnap::Meio => {
            fecha(vec![s + v(0., -7.), s + v(6.5, 5.), s + v(-6.5, 5.)]);
        }
        TipoSnap::Quadrante => {
            fecha(vec![s + v(0., -7.), s + v(7., 0.), s + v(0., 7.), s + v(-7., 0.)]);
        }
        TipoSnap::Intersecao => {
            painter.line_segment([s + v(-6., -6.), s + v(6., 6.)], st);
            painter.line_segment([s + v(-6., 6.), s + v(6., -6.)], st);
        }
        TipoSnap::SobreCurva => {
            // ampulheta: o ponto corre pela curva
            painter.add(egui::Shape::closed_line(vec![s + v(-5., -6.), s + v(5., -6.), s + v(-5., 6.), s + v(5., 6.)], st));
        }
    }
    // o nome passa por cima da sombra do perfil: precisa de fundo e da cor de texto do tema
    let visual = painter.ctx().global_style().visuals.clone();
    let g = painter.layout_no_wrap(tr(tipo.nome()).to_string(), egui::FontId::proportional(crate::acessibilidade::fonte_3d(10.)), visual.strong_text_color());
    let r = egui::Rect::from_min_size(s + v(10., -10. - g.size().y), g.size() + v(4., 0.));
    painter.rect_filled(r, 2., visual.extreme_bg_color.gamma_multiply(0.85));
    painter.galley(r.min + v(2., 0.), g, visual.strong_text_color());
}

/// Motivo de uma linha de inferência.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Motivo {
    Horizontal,
    Vertical,
    Perpendicular,
    Paralela,
    /// alinhado com outro ponto do sketch
    Alinhado,
}

impl Motivo {
    pub fn glifo(self) -> &'static str {
        match self {
            Motivo::Horizontal => "H",
            Motivo::Vertical => "V",
            Motivo::Perpendicular => "⊥",
            Motivo::Paralela => "∥",
            Motivo::Alinhado => "",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Linha {
    pub de: P,
    pub ate: P,
    pub motivo: Motivo,
}

/// Resultado da inferência: o cursor ajustado e as linhas que explicam o ajuste.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Inferencia {
    pub uv: P,
    pub linhas: Vec<Linha>,
    /// a linha nova sai perpendicular ou paralela a esta
    pub direcao: Option<(Id, Motivo)>,
}

/// Procura alinhamentos do cursor. `ancora`: de onde a forma sai. `anterior`: a linha que chega nela.
/// `tol`: a folga, no plano. `sem`: pontos que não servem de referência.
pub fn inferir(sk: &SketchData, uv: P, ancora: Option<P>, anterior: Option<Id>, tol: f64, sem: &[Id]) -> Inferencia {
    let mut q = uv;
    let (mut trava_x, mut trava_y) = (false, false);
    let mut direcao = None;
    let mut motivo_ancora = None;
    if let Some(a) = ancora {
        let (dx, dy) = ((uv[0] - a[0]).abs(), (uv[1] - a[1]).abs());
        if dy < tol && dx > tol {
            q[1] = a[1];
            trava_y = true;
            motivo_ancora = Some(Motivo::Horizontal);
        } else if dx < tol && dy > tol {
            q[0] = a[0];
            trava_x = true;
            motivo_ancora = Some(Motivo::Vertical);
        } else if let Some((l, Traco::Reta { a: p0, b: p1 })) = anterior.and_then(|l| Some((l, traco(sk, l)?))) {
            let len = (p1[0] - p0[0]).hypot(p1[1] - p0[1]).max(1e-12);
            let u = [(p1[0] - p0[0]) / len, (p1[1] - p0[1]) / len];
            let d = [uv[0] - a[0], uv[1] - a[1]];
            let (ao_longo, ao_lado) = (d[0] * u[0] + d[1] * u[1], d[0] * -u[1] + d[1] * u[0]);
            if ao_lado.abs() < tol && ao_longo.abs() > tol {
                q = [a[0] + u[0] * ao_longo, a[1] + u[1] * ao_longo];
                (trava_x, trava_y) = (true, true);
                motivo_ancora = Some(Motivo::Paralela);
                direcao = Some((l, Motivo::Paralela));
            } else if ao_longo.abs() < tol && ao_lado.abs() > tol {
                q = [a[0] - u[1] * ao_lado, a[1] + u[0] * ao_lado];
                (trava_x, trava_y) = (true, true);
                motivo_ancora = Some(Motivo::Perpendicular);
                direcao = Some((l, Motivo::Perpendicular));
            }
        }
    }
    // alinhamento com outros pontos: o mais perto em cada eixo, sem contar a própria âncora
    let mut por_x: Option<(f64, P)> = None;
    let mut por_y: Option<(f64, P)> = None;
    for p in sk.points.iter().filter(|p| !sem.contains(&p.id)) {
        let r = [p.x, p.y];
        if ancora == Some(r) {
            continue;
        }
        let (ex, ey) = ((uv[0] - r[0]).abs(), (uv[1] - r[1]).abs());
        if !trava_x && ex < tol && ey > tol && por_x.is_none_or(|b| ey < b.0) {
            por_x = Some((ey, r));
        }
        if !trava_y && ey < tol && ex > tol && por_y.is_none_or(|b| ex < b.0) {
            por_y = Some((ex, r));
        }
    }
    if let Some((_, r)) = por_x {
        q[0] = r[0];
    }
    if let Some((_, r)) = por_y {
        q[1] = r[1];
    }
    let mut linhas = vec![];
    if let (Some(a), Some(m)) = (ancora, motivo_ancora) {
        linhas.push(Linha { de: a, ate: q, motivo: m });
    }
    linhas.extend(por_x.into_iter().chain(por_y).map(|(_, r)| Linha { de: r, ate: q, motivo: Motivo::Alinhado }));
    Inferencia { uv: q, linhas, direcao }
}

/// Quanto a linha de inferência continua depois do cursor, em pixels.
const ALEM_DO_CURSOR: f32 = 60.;

/// Desenha as linhas tracejadas com o glifo do motivo.
pub fn desenha_inferencia(painter: &egui::Painter, tela: &dyn Fn(P) -> Pos2, inf: &Inferencia, cor: Color32) {
    for l in &inf.linhas {
        let (a, b) = (tela(l.de), tela(l.ate));
        // passa do cursor: até ele, a prévia da forma cobre a linha que explica o ajuste
        let alem = b + (b - a).normalized() * ALEM_DO_CURSOR;
        painter.extend(egui::Shape::dashed_line(&[a, alem], Stroke::new(1., cor), 5., 4.));
        let m = a + (b - a) * 0.5;
        painter.text(m + egui::vec2(6., -6.), egui::Align2::LEFT_BOTTOM, l.motivo.glifo(), egui::FontId::proportional(crate::acessibilidade::fonte_3d(11.)), cor);
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use eframe::egui::pos2;

    fn ids() -> impl FnMut() -> Id {
        let mut n = 5000u64;
        move || {
            n += 1;
            Id(n)
        }
    }

    fn tela(q: P) -> Pos2 {
        pos2(q[0] as f32, q[1] as f32)
    }

    fn acha(sk: &SketchData, q: P) -> Option<Achado> {
        achar(sk, tela(q), q, &tela, &[], &[], true)
    }

    fn linha(sk: &mut SketchData, al: &mut impl FnMut() -> Id, a: P, b: P) -> Id {
        let (p, q) = (sk.add_point(al, a[0], a[1]), sk.add_point(al, b[0], b[1]));
        sk.add_curve(al, Curve::Line { a: p, b: q }, false)
    }

    #[test]
    fn cada_lugar_tem_o_seu_tipo() {
        let (mut sk, mut al) = (SketchData::default(), ids());
        let l = linha(&mut sk, &mut al, [0., 0.], [100., 0.]);
        let c = sk.add_point(&mut al, 300., 0.);
        sk.add_curve(&mut al, Curve::Circle { c, r: 50. }, false);
        linha(&mut sk, &mut al, [20., -40.], [20., 40.]);
        assert_eq!(acha(&sk, [2., 1.]).unwrap().tipo, TipoSnap::Ponta);
        let meio = acha(&sk, [52., 3.]).unwrap();
        assert_eq!((meio.tipo, meio.uv, meio.curvas[0]), (TipoSnap::Meio, [50., 0.], Some(l)));
        assert_eq!(acha(&sk, [301., 1.]).unwrap().tipo, TipoSnap::Centro);
        let quad = acha(&sk, [352., 2.]).unwrap();
        assert_eq!((quad.tipo, quad.uv), (TipoSnap::Quadrante, [350., 0.]));
        let x = acha(&sk, [21., 2.]).unwrap();
        assert_eq!((x.tipo, x.uv), (TipoSnap::Intersecao, [20., 0.]));
        // longe dos pontos notáveis: corre pela curva
        let s = acha(&sk, [80., 3.]).unwrap();
        assert_eq!((s.tipo, s.uv), (TipoSnap::SobreCurva, [80., 0.]));
        assert_eq!(acha(&sk, [80., 30.]), None);
        // só os pontos existentes, com os snaps desligados
        assert_eq!(achar(&sk, tela([52., 3.]), [52., 3.], &tela, &[], &[], false), None);
    }

    #[test]
    fn o_que_se_arrasta_nao_se_apoia_em_si() {
        let (mut sk, mut al) = (SketchData::default(), ids());
        let l = linha(&mut sk, &mut al, [0., 0.], [100., 0.]);
        let Curve::Line { b, .. } = sk.curve(l).unwrap().geom else { panic!() };
        assert_eq!(achar(&sk, tela([100., 2.]), [100., 2.], &tela, &[b], &[l], true), None);
    }

    #[test]
    fn arco_so_tem_snap_no_trecho_que_existe() {
        let (mut sk, mut al) = (SketchData::default(), ids());
        let (c, s, e) = (sk.add_point(&mut al, 0., 0.), sk.add_point(&mut al, 100., 0.), sk.add_point(&mut al, 0., 100.));
        sk.add_curve(&mut al, Curve::Arc { c, s, e }, false);
        let meio = 100. * std::f64::consts::FRAC_1_SQRT_2;
        assert_eq!(acha(&sk, [meio, meio]).unwrap().tipo, TipoSnap::Meio);
        // quadrantes de baixo e da esquerda são vazio
        assert_eq!(acha(&sk, [-100., 0.]), None);
        assert_eq!(acha(&sk, [-meio, -meio]), None);
    }

    #[test]
    fn apoio_vira_restricao_que_o_solver_mantem() {
        let (mut sk, mut al) = (SketchData::default(), ids());
        let l = linha(&mut sk, &mut al, [0., 0.], [100., 0.]);
        let c = sk.add_point(&mut al, 300., 0.);
        let circ = sk.add_curve(&mut al, Curve::Circle { c, r: 50. }, false);
        let m = sk.add_point(&mut al, 50., 0.);
        vincula(&mut sk, &mut al, m, TipoSnap::Meio, [Some(l), None]);
        assert!(sk.constraints.iter().any(|k| k.kind == Kind::Midpoint(m, l)));
        let q = sk.add_point(&mut al, 300., 50.);
        vincula(&mut sk, &mut al, q, TipoSnap::Quadrante, [Some(circ), None]);
        assert!(sk.constraints.iter().any(|k| k.kind == Kind::VerticalPts(q, c)));
        assert!(sk.constraints.iter().any(|k| k.kind == Kind::PointOnCurve(q, circ)));
        let x = sk.add_point(&mut al, 0., 0.);
        vincula(&mut sk, &mut al, x, TipoSnap::Intersecao, [Some(l), Some(circ)]);
        assert_eq!(sk.constraints.iter().filter(|k| matches!(k.kind, Kind::PointOnCurve(p, _) if p == x)).count(), 2);
    }

    #[test]
    fn soltar_duas_vezes_no_mesmo_apoio_nao_repete_a_restricao() {
        let (mut sk, mut al) = (SketchData::default(), ids());
        let l = linha(&mut sk, &mut al, [0., 0.], [100., 0.]);
        let p = sk.add_point(&mut al, 30., 0.);
        vincula(&mut sk, &mut al, p, TipoSnap::SobreCurva, [Some(l), None]);
        vincula(&mut sk, &mut al, p, TipoSnap::SobreCurva, [Some(l), None]);
        assert_eq!(sk.constraints.len(), 1);
    }

    #[test]
    fn opcoes_do_snap_tem_traducao() {
        for t in [DICA_RESTRINGE, DICA_JUNTAR, NAO_JUNTOU, "Snap vira restrição", "Juntar pontos ao soltar"] {
            assert!(crate::i18n::tem_traducao(t), "{t}");
        }
    }

    #[test]
    fn inferencia_alinha_com_a_partida() {
        let sk = SketchData::default();
        let i = inferir(&sk, [50., 1.], Some([0., 0.]), None, 2., &[]);
        assert_eq!(i.uv, [50., 0.]);
        assert_eq!(i.linhas[0].motivo, Motivo::Horizontal);
        let i = inferir(&sk, [-1., 40.], Some([0., 0.]), None, 2., &[]);
        assert_eq!(i.uv, [0., 40.]);
        assert_eq!(i.linhas[0].motivo, Motivo::Vertical);
        let i = inferir(&sk, [30., 40.], Some([0., 0.]), None, 2., &[]);
        assert_eq!(i.uv, [30., 40.]);
        assert!(i.linhas.is_empty());
    }

    #[test]
    fn inferencia_acha_perpendicular_e_paralela_da_linha_anterior() {
        let (mut sk, mut al) = (SketchData::default(), ids());
        // inclinada 3-4-5: nem horizontal nem vertical
        let l = linha(&mut sk, &mut al, [0., 0.], [30., 40.]);
        let a = [30., 40.];
        // ⊥: direção (-4, 3)
        let i = inferir(&sk, [30. - 40. + 0.5, 40. + 30.], Some(a), Some(l), 2., &[]);
        assert_eq!(i.direcao, Some((l, Motivo::Perpendicular)));
        let u = [i.uv[0] - a[0], i.uv[1] - a[1]];
        assert!((u[0] * 30. + u[1] * 40.).abs() < 1e-9);
        // ∥: continua a linha
        let i = inferir(&sk, [60.5, 80.], Some(a), Some(l), 2., &[]);
        assert_eq!(i.direcao, Some((l, Motivo::Paralela)));
        let u = [i.uv[0] - a[0], i.uv[1] - a[1]];
        assert!((u[0] * 40. - u[1] * 30.).abs() < 1e-9);
    }

    #[test]
    fn inferencia_alinha_com_outros_pontos_nos_dois_eixos() {
        let (mut sk, mut al) = (SketchData::default(), ids());
        sk.add_point(&mut al, 10., 100.);
        sk.add_point(&mut al, 100., 20.);
        let i = inferir(&sk, [11., 19.], None, None, 2., &[]);
        assert_eq!(i.uv, [10., 20.]);
        assert_eq!(i.linhas.len(), 2);
        assert!(i.linhas.iter().all(|l| l.motivo == Motivo::Alinhado && l.ate == [10., 20.]));
    }
}
