// SPDX-License-Identifier: LGPL-2.1-or-later
//! Barra de restrições ao selecionar e fantasma do resultado (B273).
//!
//! A prévia é um clone do sketch com a restrição aplicada e resolvido: o documento nem chega a ser
//! tocado. O solver tem Jacobiano numérico, e o custo cresce rápido com o tamanho do sketch, por isso
//! ele roda com prazo. Passou do orçamento, a prévia cai para só destacar os elementos, e o sketch
//! daquele tamanho não tenta de novo, para o mouse não engasgar a cada opção.

use crate::i18n::tr;
use crate::sketch_ui::{applicable, apply_constraint, ghost_ids, CKind, Ent, Tool};
use eframe::egui::{self, Pos2, Rect};
use fenkra_core::sketch::Kind;
use fenkra_core::{Id, SketchData};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Quanto a prévia pode levar por opção: o mouse precisa continuar leve.
pub const ORCAMENTO: Duration = Duration::from_millis(30);

/// O que aconteceria com a restrição aplicada.
#[derive(Clone, Debug, PartialEq)]
pub enum Previa {
    /// a geometria já resolvida; `conflito` se a restrição não cabe no que já existe
    Fantasma { depois: Box<SketchData>, conflito: bool },
    /// resolver passou do orçamento: só os elementos ficam em destaque
    Destaque,
}

/// Resolve o sketch com a restrição num clone. `redundantes` é o que o sketch já tem antes: restrição
/// a mais que não acrescenta nada também conflita, porque deixaria o sketch sobre-restrito.
pub fn prever(sk: &SketchData, k: CKind, sel: &[Ent], vars: &HashMap<String, f64>, redundantes: usize, orcamento: Duration) -> Option<Previa> {
    let mut g = sk.clone();
    apply_constraint(&mut g, &mut ghost_ids(), k, sel).ok()?;
    let rep = g.solve_ate(vars, &[], Some(Instant::now() + orcamento));
    Some(if rep.estourou {
        Previa::Destaque
    } else {
        Previa::Fantasma { conflito: !rep.converged || rep.redundant > redundantes, depois: Box::new(g) }
    })
}

/// Lembra a prévia aberta, para não resolver de novo a cada quadro com o mouse parado.
#[derive(Default)]
pub struct Memoria {
    chave: Option<(CKind, Vec<Ent>)>,
    base: SketchData,
    previa: Option<Previa>,
    /// menor sketch que já estourou o orçamento
    lento_com: Option<usize>,
    /// opções da barra, que só dependem da seleção e das contagens do sketch
    opcoes: Option<((Vec<Ent>, usize, usize, usize), Vec<CKind>)>,
}

impl Memoria {
    /// Prévia de `k` na seleção, resolvendo só quando algo mudou.
    pub fn previa(&mut self, sk: &SketchData, k: CKind, sel: &[Ent], vars: &HashMap<String, f64>, redundantes: usize, orcamento: Duration) -> Option<&Previa> {
        let chave = Some((k, sel.to_vec()));
        if self.chave != chave || self.base != *sk {
            self.chave = chave;
            self.base = sk.clone();
            let n = sk.points.len();
            self.previa = if self.lento_com.is_some_and(|m| n >= m) {
                Some(Previa::Destaque)
            } else {
                prever(sk, k, sel, vars, redundantes, orcamento)
            };
            if self.previa == Some(Previa::Destaque) {
                self.lento_com = Some(self.lento_com.map_or(n, |m| m.min(n)));
            }
        }
        self.previa.as_ref()
    }

    /// Restrições que a barra oferece para a seleção.
    pub fn opcoes(&mut self, sk: &SketchData, sel: &[Ent]) -> Vec<CKind> {
        let chave = (sel.to_vec(), sk.points.len(), sk.curves.len(), sk.constraints.len());
        if self.opcoes.as_ref().is_none_or(|o| o.0 != chave) {
            self.opcoes = Some((chave.clone(), opcoes(sk, sel)));
        }
        self.opcoes.as_ref().map_or(vec![], |o| o.1.clone())
    }
}

/// Mesma restrição: o mesmo tipo sobre os mesmos elementos, em qualquer ordem.
fn mesma(a: &Kind, b: &Kind) -> bool {
    let ordem = |k: &Kind| {
        let mut r = k.refs();
        r.sort_by_key(|x| x.0);
        r
    };
    std::mem::discriminant(a) == std::mem::discriminant(b) && ordem(a) == ordem(b)
}

/// O que `applicable` acha, menos o que já está no sketch: depois de aplicar, a opção sai da barra.
pub fn opcoes(sk: &SketchData, sel: &[Ent]) -> Vec<CKind> {
    let ja = |k: CKind| {
        let mut g = sk.clone();
        let antes = g.constraints.len();
        apply_constraint(&mut g, &mut ghost_ids(), k, sel).is_ok() && g.constraints.len() > antes && g.constraints[antes..].iter().all(|n| sk.constraints.iter().any(|c| mesma(&c.kind, &n.kind)))
    };
    applicable(sk, sel).into_iter().filter(|k| !ja(*k)).collect()
}

/// Aplica a restrição e diz o que continua selecionado: a barra fica aberta para a próxima. Um ponto
/// fundido com outro deixa de existir e sai da seleção.
pub fn aplicar(sk: &mut SketchData, al: &mut impl FnMut() -> Id, k: CKind, sel: &[Ent]) -> Result<Vec<Ent>, &'static str> {
    apply_constraint(sk, al, k, sel)?;
    Ok(sel
        .iter()
        .copied()
        .filter(|e| match e {
            Ent::Pt(p) => sk.point(*p).is_some(),
            Ent::Cv(c) => sk.curve(*c).is_some(),
            Ent::Cs(c) => sk.constraints.iter().any(|x| x.id == *c),
        })
        .collect())
}

/// Onde a barra abre: logo acima da seleção, sem sair da vista.
pub fn lugar(caixa: Rect, vista: Rect, n: usize) -> Pos2 {
    let largura = n as f32 * 32. + 16.;
    let x = (caixa.center().x - largura / 2.).clamp(vista.left() + 8., (vista.right() - largura - 8.).max(vista.left() + 8.));
    let y = if caixa.top() - 52. > vista.top() + 60. { caixa.top() - 52. } else { (caixa.bottom() + 12.).min(vista.bottom() - 52.) };
    egui::pos2(x, y)
}

/// Mostra a barra e devolve (opção sob o mouse, opção clicada).
pub fn mostrar(ctx: &egui::Context, pos: Pos2, opcoes: &[CKind]) -> (Option<CKind>, Option<CKind>) {
    let (mut sobre, mut clicada) = (None, None);
    egui::Area::new(egui::Id::unique("sk_barra_restricoes")).fixed_pos(pos).order(egui::Order::Foreground).show(ctx, |ui| {
        egui::Frame::popup(ui.style()).show(ui, |ui| {
            ui.horizontal(|ui| {
                for k in opcoes {
                    let (icone, nome, _, dica) = Tool::Constr(*k).info();
                    let img = egui::Image::new(crate::icons::get(icone)).fit_to_exact_size(egui::vec2(20., 20.));
                    let r = ui.add(egui::Button::image(img).frame(false)).on_hover_text(format!("{}\n{}", tr(nome), tr(dica)));
                    if r.hovered() {
                        sobre = Some(*k);
                    }
                    if r.clicked() {
                        clicada = Some(*k);
                    }
                }
            });
        });
    });
    (sobre, clicada)
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

    fn duas_linhas() -> (SketchData, Id, Id) {
        let mut al = ids();
        let mut s = SketchData::default();
        let a = s.add_point(&mut al, 0., 0.);
        let b = s.add_point(&mut al, 30., 6.);
        let c = s.add_point(&mut al, 0., 20.);
        let d = s.add_point(&mut al, 30., 30.);
        let l1 = s.add_curve(&mut al, Curve::Line { a, b }, false);
        let l2 = s.add_curve(&mut al, Curve::Line { a: c, b: d }, false);
        (s, l1, l2)
    }

    fn dir(s: &SketchData, l: Id) -> [f64; 2] {
        let Some(Curve::Line { a, b }) = s.curve(l).map(|c| c.geom.clone()) else { panic!() };
        let (p, q) = (s.pos(a), s.pos(b));
        [q[0] - p[0], q[1] - p[1]]
    }

    #[test]
    fn previa_mostra_o_resolvido_e_nao_toca_no_sketch() {
        let (s, l1, l2) = duas_linhas();
        let antes = s.clone();
        let vars = HashMap::new();
        let sel = [Ent::Cv(l1), Ent::Cv(l2)];
        let Some(Previa::Fantasma { depois, conflito }) = prever(&s, CKind::Parallel, &sel, &vars, 0, Duration::from_secs(5)) else { panic!("sem fantasma") };
        assert!(!conflito);
        let (u, w) = (dir(&depois, l1), dir(&depois, l2));
        assert!((u[0] * w[1] - u[1] * w[0]).abs() < 1e-6, "o fantasma não está paralelo: {u:?} {w:?}");
        assert_eq!(s, antes, "a prévia não pode alterar o sketch");
        assert!(s.constraints.is_empty());
    }

    #[test]
    fn conflito_e_restricao_de_sobra_ficam_vermelhas() {
        let (mut s, l1, l2) = duas_linhas();
        let mut al = ids();
        for _ in 0..50 {
            al();
        }
        s.add_constraint(&mut al, Kind::Horizontal(l1));
        s.add_constraint(&mut al, Kind::Vertical(l2));
        let vars = HashMap::new();
        let rep = s.solve(&vars, &[]);
        assert!(rep.converged);
        let sel = [Ent::Cv(l1), Ent::Cv(l2)];
        let tempo = Duration::from_secs(5);
        let conflita = |p: Option<Previa>| matches!(p, Some(Previa::Fantasma { conflito: true, .. }));
        assert!(conflita(prever(&s, CKind::Parallel, &sel, &vars, rep.redundant, tempo)), "horizontal e vertical não são paralelas");
        assert!(!conflita(prever(&s, CKind::Perpendicular, &sel, &vars, rep.redundant, tempo)) || rep.redundant == 0);

        // a linha já é horizontal: pedir de novo por dois pontos sobra
        let Some(Curve::Line { a, b }) = s.curve(l1).map(|c| c.geom.clone()) else { panic!() };
        assert!(conflita(prever(&s, CKind::Horizontal, &[Ent::Pt(a), Ent::Pt(b)], &vars, rep.redundant, tempo)));
    }

    /// Em sketch grande a prévia cai para o destaque, e o seguinte nem tenta.
    #[test]
    fn sketch_grande_passa_do_orcamento_e_so_destaca() {
        let mut al = ids();
        let mut s = SketchData::default();
        let mut ant = s.add_point(&mut al, 0., 0.);
        let mut linhas = vec![];
        for i in 1..1200 {
            let p = s.add_point(&mut al, i as f64 * 3., (i % 7) as f64);
            linhas.push(s.add_curve(&mut al, Curve::Line { a: ant, b: p }, false));
            ant = p;
        }
        for l in &linhas[..600] {
            s.add_constraint(&mut al, Kind::Horizontal(*l));
        }
        let antes = s.clone();
        let vars = HashMap::new();
        let sel = [Ent::Cv(linhas[900]), Ent::Cv(linhas[1000])];
        let mut m = Memoria::default();
        let inicio = Instant::now();
        assert_eq!(m.previa(&s, CKind::Parallel, &sel, &vars, 0, ORCAMENTO), Some(&Previa::Destaque));
        assert!(inicio.elapsed() < Duration::from_secs(2), "o prazo tem que cortar o solver: {:?}", inicio.elapsed());
        assert_eq!(s, antes);
        assert_eq!(m.lento_com, Some(s.points.len()));
        // a próxima opção não paga o prazo de novo
        assert_eq!(m.previa(&s, CKind::Perpendicular, &sel, &vars, 0, Duration::from_secs(60)), Some(&Previa::Destaque));
    }

    #[test]
    fn memoria_nao_resolve_de_novo_com_o_mouse_parado() {
        let (s, l1, l2) = duas_linhas();
        let vars = HashMap::new();
        let sel = [Ent::Cv(l1), Ent::Cv(l2)];
        let mut m = Memoria::default();
        assert!(m.previa(&s, CKind::Equal, &sel, &vars, 0, Duration::from_secs(5)).is_some());
        // sem prazo nenhum, só a memória pode responder
        assert!(matches!(m.previa(&s, CKind::Equal, &sel, &vars, 0, Duration::ZERO), Some(Previa::Fantasma { .. })));
    }

    /// A barra continua aberta depois de aplicar, sem repetir o que já foi aplicado.
    #[test]
    fn barra_fica_aberta_e_tira_o_que_ja_foi_aplicado() {
        let (mut s, l1, l2) = duas_linhas();
        let sel = vec![Ent::Cv(l1), Ent::Cv(l2)];
        let antes = opcoes(&s, &sel);
        assert!(antes.contains(&CKind::Parallel) && antes.contains(&CKind::Equal));
        let mut al = ids();
        for _ in 0..50 {
            al();
        }
        let resta = aplicar(&mut s, &mut al, CKind::Parallel, &sel).unwrap();
        assert_eq!(resta, sel, "a seleção continua");
        let depois = opcoes(&s, &resta);
        assert!(!depois.contains(&CKind::Parallel), "{depois:?}");
        assert!(depois.contains(&CKind::Equal), "as outras continuam: {depois:?}");
    }

    #[test]
    fn ponto_fundido_sai_da_selecao() {
        let (mut s, l1, l2) = duas_linhas();
        let (Some(Curve::Line { b, .. }), Some(Curve::Line { a: c, .. })) = (s.curve(l1).map(|x| x.geom.clone()), s.curve(l2).map(|x| x.geom.clone())) else { panic!() };
        let mut al = ids();
        let resta = aplicar(&mut s, &mut al, CKind::Coincident, &[Ent::Pt(b), Ent::Pt(c)]).unwrap();
        assert_eq!(resta.len(), 1);
    }

    #[test]
    fn barra_abre_acima_da_selecao_e_dentro_da_vista() {
        let vista = Rect::from_min_size(egui::pos2(0., 0.), egui::vec2(800., 600.));
        let caixa = Rect::from_min_size(egui::pos2(300., 300.), egui::vec2(100., 50.));
        let p = lugar(caixa, vista, 4);
        assert!(p.y < 300.);
        // selecionado junto ao topo: abre embaixo
        let p = lugar(Rect::from_min_size(egui::pos2(780., 10.), egui::vec2(10., 10.)), vista, 4);
        assert!(p.y > 20. && p.x + 4. * 32. + 16. <= 800.);
    }
}
