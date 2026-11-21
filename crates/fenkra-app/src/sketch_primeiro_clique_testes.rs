// SPDX-License-Identifier: LGPL-2.1-or-later
//! B572: primeiro clique sobre geometria que já existe, pelo `overlay` de verdade.
//!
//! Cada teste monta o sketch em código, abre com `begin` como a interface abre e manda os eventos crus
//! do mouse (mover, descer, subir) quadro a quadro, numa vista de cima ortogonal. O ponto da tela de
//! cada clique sai da mesma projeção que o `overlay` usa, então o clique cai em cima da curva.

use super::*;
use crate::preso::Alvo;
use fenkra_core::sketch::Constraint;
use fenkra_core::{Document, OriginPlane, PlaneRef, Sketch};

/// Ids do sketch montado no teste: longe dos do documento, que o `overlay` aloca a partir do início.
fn ids() -> impl FnMut() -> Id {
    let mut n = 10_000u64;
    move || {
        n += 1;
        Id(n)
    }
}

struct Bancada {
    app: App,
    ctx: egui::Context,
    cam: Camera,
    tela: Rect,
    t: f64,
    _serie: std::sync::MutexGuard<'static, ()>,
}

impl Bancada {
    fn nova(sk: SketchData) -> Self {
        let serie = crate::pmi_nota_cmd::tests::SERIE.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("FENKRA_DEMO_TESTE", "1");
        let ctx = egui::Context::default();
        let mut app = App::new(&eframe::CreationContext::_new_kittest(ctx.clone()));
        app.started = true;
        let mut d = Document::new("B572");
        let id = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data: sk }));
        app.load_doc_pub(d, None);
        let s = &mut app.settings;
        (s.grid, s.snaps, s.auto_constraints, s.snap_restringe, s.auto_dims, s.dim_on_type) = (false, true, true, true, false, false);
        begin(&mut app, id);
        // vista de cima, ortogonal: o plano XY do sketch de frente para a tela
        let cam = Camera { target: Vec3::ZERO, yaw: -std::f32::consts::FRAC_PI_2, pitch: std::f32::consts::FRAC_PI_2 - 1e-3, dist: 300., ortho: true };
        let mut b = Bancada { app, ctx, cam, tela: Rect::from_min_size(Pos2::ZERO, egui::vec2(1000., 700.)), t: 0., _serie: serie };
        b.quadro(vec![]);
        b
    }

    fn quadro(&mut self, evs: Vec<egui::Event>) {
        // 0,1 s por quadro: dois cliques seguidos ficam longe da janela do duplo clique
        self.t += 0.1;
        let raw = egui::RawInput { screen_rect: Some(self.tela), time: Some(self.t), events: evs, ..Default::default() };
        let (app, cam, tela) = (&mut self.app, self.cam, self.tela);
        let mut saida = self.ctx.run_ui(raw, |ui| {
            let resp = ui.allocate_rect(tela, egui::Sense::click_and_drag());
            overlay(app, ui, tela, &resp, cam);
        });
        saida.textures_delta.clear();
    }

    fn tela_de(&self, uv: [f64; 2]) -> Pos2 {
        View { cam: self.cam, rect: self.tela, f: frame(&self.app).expect("plano do sketch") }.to_screen(uv)
    }

    fn botao(&mut self, uv: [f64; 2], button: egui::PointerButton) {
        let pos = self.tela_de(uv);
        let ev = |pressed| egui::Event::PointerButton { pos, button, pressed, modifiers: egui::Modifiers::NONE };
        self.quadro(vec![egui::Event::PointerMoved(pos)]);
        self.quadro(vec![ev(true)]);
        self.quadro(vec![ev(false)]);
        self.quadro(vec![]);
        // o cursor sai de cima do ponto: o próximo clique, mesmo perto, não vira duplo
        self.t += 0.5;
    }

    fn clique(&mut self, uv: [f64; 2]) {
        self.botao(uv, egui::PointerButton::Primary);
    }

    fn sk(&self) -> &SketchData {
        data(&self.app).expect("sketch aberto")
    }

    fn edit(&mut self) -> &mut SketchEdit {
        self.app.sketch.as_mut().expect("modo sketch")
    }

    fn ferramenta(&mut self, t: Tool, presa: bool) {
        let e = self.edit();
        e.set_tool(t);
        if presa {
            e.preso.prender(Alvo::Sk(t));
        }
    }

    /// Curvas que não são eixos nem pontos soltos.
    fn formas(&self) -> Vec<(Id, Curve)> {
        let sk = self.sk();
        sk.curves.iter().filter(|c| !sk.e_eixo(c.id) && !matches!(c.geom, Curve::Point { .. })).map(|c| (c.id, c.geom.clone())).collect()
    }

    fn tem(&self, k: &Kind) -> bool {
        self.sk().constraints.iter().any(|c: &Constraint| c.kind == *k)
    }
}

/// Linha base fora dos eixos e da origem: de (-50, 0) a (50, 10). (20, 7) está sobre ela, longe do meio.
fn com_linha() -> (SketchData, Id, [Id; 2]) {
    let mut al = ids();
    let mut sk = SketchData::default();
    let (a, b) = (sk.add_point(&mut al, -50., 0.), sk.add_point(&mut al, 50., 10.));
    let l = sk.add_curve(&mut al, Curve::Line { a, b }, false);
    (sk, l, [a, b])
}

fn com_circulo() -> (SketchData, Id) {
    let mut al = ids();
    let mut sk = SketchData::default();
    let c = sk.add_point(&mut al, 5., -5.);
    let k = sk.add_curve(&mut al, Curve::Circle { c, r: 30. }, false);
    (sk, k)
}

/// Arco de 0° a 120° em torno de (5, -5), raio 30.
fn com_arco() -> (SketchData, Id) {
    let mut al = ids();
    let mut sk = SketchData::default();
    let c = sk.add_point(&mut al, 5., -5.);
    let s = sk.add_point(&mut al, 35., -5.);
    let e = sk.add_point(&mut al, 5. + 30. * 120f64.to_radians().cos(), -5. + 30. * 120f64.to_radians().sin());
    let k = sk.add_curve(&mut al, Curve::Arc { c, s, e }, false);
    (sk, k)
}

/// A 45° no círculo ou no arco de (5, -5) e raio 30: longe dos quadrantes e das pontas.
const A_45: [f64; 2] = [5. + 21.213203, -5. + 21.213203];

/// Ponto do sketch mais perto de `uv`.
fn ponto_em(sk: &SketchData, uv: [f64; 2]) -> Id {
    sk.points.iter().min_by(|a, b| d2([a.x, a.y], uv).total_cmp(&d2([b.x, b.y], uv))).map(|p| p.id).unwrap()
}

/// O que se espera do ponto de partida da linha.
enum Partida {
    /// ponto novo preso à curva por "ponto na curva"
    NaCurva(Id),
    /// o ponto que já existia
    Ponto(Id),
}

/// Linha começada em `inicio` (sobre geometria) e terminada em `fim` (no vazio): o 2º clique cria o
/// segmento, ligado à geometria no 1º ponto. Solta, a ferramenta volta à seleção depois do segmento;
/// presa, a cadeia segue e o 3º clique faz o segundo segmento.
fn linha_partindo_de(sk: SketchData, inicio: [f64; 2], partida: Partida, presa: bool) {
    let caso = format!("{inicio:?}, presa={presa}");
    let mut b = Bancada::nova(sk);
    b.ferramenta(Tool::Line, presa);
    let antes = b.formas().len();
    b.clique(inicio);
    assert_eq!(b.edit().tool, Tool::Line, "{caso}: o 1º clique não pode encerrar a ferramenta");
    assert!(b.edit().chain_last.is_some(), "{caso}: o 1º clique fica à espera do segundo");

    let fim = [-20., 45.];
    b.clique(fim);
    let novas: Vec<(Id, Curve)> = b.formas().into_iter().skip(antes).collect();
    let [(nova, Curve::Line { a, b: pb })] = novas.as_slice() else { panic!("{caso}: o 2º clique cria um segmento, veio {novas:?}") };
    let (nova, a, pb) = (*nova, *a, *pb);
    // a inferência pode puxar o cursor alguns pixels para um alinhamento
    assert!(d2(b.sk().pos(pb), fim) < 3., "{caso}: a ponta segue o 2º clique, veio {:?}", b.sk().pos(pb));
    match partida {
        Partida::NaCurva(c) => {
            assert!(b.tem(&Kind::PointOnCurve(a, c)), "{caso}: o 1º ponto fica preso à curva clicada");
        }
        Partida::Ponto(p) => assert_eq!(a, p, "{caso}: o 1º ponto é o ponto que já existia"),
    }
    assert!(b.edit().report.converged, "{caso}: o sketch resolve");
    if presa {
        assert_eq!(b.edit().tool, Tool::Line, "{caso}: presa, segue na linha");
        assert_eq!(b.edit().chain_last, Some(pb), "{caso}: presa, a cadeia continua da ponta");
        b.clique([30., 60.]);
        let segunda = b.formas().into_iter().skip(antes + 1).collect::<Vec<_>>();
        assert!(matches!(segunda.as_slice(), [(_, Curve::Line { a, .. })] if *a == pb), "{caso}: o 3º clique faz o 2º segmento");
    } else {
        assert_eq!(b.edit().tool, Tool::Select, "{caso}: solta, um segmento e volta à seleção");
    }
    let _ = nova;
}

#[test]
fn sketch_linha_comeca_em_cima_de_outra_linha() {
    for presa in [false, true] {
        let (sk, l, _) = com_linha();
        linha_partindo_de(sk, [20., 7.], Partida::NaCurva(l), presa);
    }
}

#[test]
fn sketch_linha_comeca_na_ponta_de_outra_linha() {
    for presa in [false, true] {
        let (sk, _, [_, pb]) = com_linha();
        linha_partindo_de(sk, [50., 10.], Partida::Ponto(pb), presa);
    }
}

#[test]
fn sketch_linha_comeca_em_cima_de_circulo_e_de_arco() {
    for presa in [false, true] {
        let (sk, c) = com_circulo();
        linha_partindo_de(sk, A_45, Partida::NaCurva(c), presa);
        let (sk, c) = com_arco();
        linha_partindo_de(sk, A_45, Partida::NaCurva(c), presa);
    }
}

/// Todas as ferramentas de forma: o 1º clique em cima da linha base e os demais no vazio terminam a
/// forma, solta (volta à seleção) ou presa (fica na ferramenta, pronta para a próxima).
#[test]
fn sketch_toda_ferramenta_de_forma_aceita_o_primeiro_clique_sobre_uma_linha() {
    let p0 = [20., 7.];
    let casos: Vec<(Tool, Vec<[f64; 2]>)> = vec![
        (Tool::Line, vec![p0, [-20., 45.]]),
        (Tool::Rect, vec![p0, [60., 50.]]),
        (Tool::RectCenter, vec![p0, [45., 35.]]),
        (Tool::Rect3, vec![p0, [60., 22.], [55., 50.]]),
        (Tool::Parallelogram, vec![p0, [60., 22.], [55., 50.]]),
        (Tool::Circle, vec![p0, [40., 30.]]),
        (Tool::Circle3, vec![p0, [40., 40.], [-5., 45.]]),
        (Tool::Arc3, vec![p0, [-20., 47.], [30., 40.]]),
        (Tool::ArcCenter, vec![p0, [45., 22.], [-5., 35.]]),
        (Tool::Slot, vec![p0, [60., 40.], [65., 30.]]),
        (Tool::Polygon, vec![p0, [40., 30.]]),
        (Tool::Point, vec![p0]),
        (Tool::Spline, vec![p0, [40., 40.], [-5., 50.]]),
    ];
    for (t, cliques) in casos {
        for presa in [false, true] {
            let (sk, l, _) = com_linha();
            let mut b = Bancada::nova(sk);
            b.ferramenta(t, presa);
            let (formas, curvas) = (b.formas().len(), b.sk().curves.len());
            for (i, q) in cliques.iter().enumerate() {
                b.clique(*q);
                if i + 1 < cliques.len() {
                    assert_eq!(b.edit().tool, t, "{t:?} presa={presa}: o clique {} não pode encerrar a ferramenta", i + 1);
                }
            }
            if t == Tool::Spline {
                // a spline termina pelo botão direito
                b.botao([-30., 60.], egui::PointerButton::Secondary);
            }
            if t == Tool::Point {
                assert!(b.sk().curves.len() > curvas, "Ponto presa={presa}: o ponto nasceu");
                let p = ponto_em(b.sk(), p0);
                assert!(b.tem(&Kind::PointOnCurve(p, l)), "Ponto presa={presa}: preso à linha");
            } else {
                assert!(b.formas().len() > formas, "{t:?} presa={presa}: a forma nasceu");
            }
            let e = b.edit();
            assert!(e.clicks.is_empty(), "{t:?} presa={presa}: os cliques recomeçam");
            assert_eq!(e.tool, if presa { t } else { Tool::Select }, "{t:?} presa={presa}: solta volta à seleção, presa repete");
        }
    }
}

/// Círculo tangente a 3: os três cliques são em cima de linhas (o lado de dentro de um quadrado).
#[test]
fn sketch_circulo_tangente_com_todos_os_cliques_sobre_linhas() {
    for presa in [false, true] {
        let mut al = ids();
        let mut sk = SketchData::default();
        sk.add_rect(&mut al, [-40., -40.], [40., 40.], false);
        let mut b = Bancada::nova(sk);
        b.ferramenta(Tool::CircleTan3, presa);
        for q in [[10., -40.], [-40., 10.], [-10., 40.]] {
            b.clique(q);
        }
        let circulos: Vec<f64> = b.formas().iter().filter_map(|(_, c)| if let Curve::Circle { r, .. } = c { Some(*r) } else { None }).collect();
        assert!(matches!(circulos[..], [r] if (r - 40.).abs() < 1e-3), "presa={presa}: o círculo inscrito, veio {circulos:?}");
        assert_eq!(b.edit().tool, if presa { Tool::CircleTan3 } else { Tool::Select });
    }
}
