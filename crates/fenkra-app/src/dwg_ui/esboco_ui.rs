// SPDX-License-Identifier: LGPL-2.1-or-later
//! B352: ferramenta de esboço na folha do desenho.
//!
//! O comando pede um clique: dentro de uma vista, o esboço fica preso a ela (anda e escala com a
//! vista, as cotas mostram a medida do modelo); fora das vistas, fica solto na folha. Depois o
//! esboço abre em edição, com as ferramentas do sketch: formas, cotas e restrições, as mesmas do
//! sketch de peça e com os mesmos ícones. A regra de cada operação mora em
//! `fenkra_desenho::esboco_editar`; aqui só se traduzem os cliques do plano da folha para o plano do
//! esboço e se desenha a prévia.

use super::*;
use crate::sketch_ui::{apply_constraint, CKind, Ent, Tool};
use crate::theme;
use fenkra_desenho::esboco::{DonoEsboco, EsbocoDesenho, TipoLinha};
use fenkra_desenho::esboco_editar::{self as ed, Alvo, Clique, Cota, Forma};
use fenkra_desenho::source::Proj;

/// Ferramenta em uso dentro do esboço.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Ferr {
    #[default]
    Selecionar,
    Forma(Forma),
    Cota(TipoCota),
    Restricao(CKind),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TipoCota {
    Inteligente,
    Horizontal,
    Vertical,
    Raio,
    Diametro,
}

const COTAS: [TipoCota; 5] = [
    TipoCota::Inteligente,
    TipoCota::Horizontal,
    TipoCota::Vertical,
    TipoCota::Raio,
    TipoCota::Diametro,
];
const RESTRICOES: [CKind; 11] = [
    CKind::Coincident,
    CKind::Horizontal,
    CKind::Vertical,
    CKind::Parallel,
    CKind::Perpendicular,
    CKind::Tangent,
    CKind::Equal,
    CKind::Concentric,
    CKind::Midpoint,
    CKind::Fixed,
    CKind::Symmetric,
];

impl Ferr {
    /// A ferramenta do sketch de peça que esta reproduz: ícone e nome vêm dela.
    fn do_sketch(self) -> Tool {
        match self {
            Ferr::Selecionar => Tool::Select,
            Ferr::Forma(f) => match f {
                Forma::Linha => Tool::Line,
                Forma::Retangulo => Tool::Rect,
                Forma::RetanguloCentro => Tool::RectCenter,
                Forma::Circulo => Tool::Circle,
                Forma::Circulo3 => Tool::Circle3,
                Forma::Arco3 => Tool::Arc3,
                Forma::ArcoCentro => Tool::ArcCenter,
                Forma::Ponto => Tool::Point,
            },
            Ferr::Cota(c) => match c {
                TipoCota::Inteligente => Tool::Dimension,
                TipoCota::Horizontal => Tool::DimH,
                TipoCota::Vertical => Tool::DimV,
                TipoCota::Raio => Tool::DimRadius,
                TipoCota::Diametro => Tool::DimDiameter,
            },
            Ferr::Restricao(k) => Tool::Constr(k),
        }
    }

    /// Dica da ferramenta no esboço do desenho (o que o clique faz aqui).
    pub fn dica(self, n: usize) -> &'static str {
        match (self, n) {
            (Ferr::Selecionar, _) => "Clique para selecionar (Shift soma); arraste um ponto: as restrições continuam valendo",
            (Ferr::Forma(Forma::Linha), 0) => "Clique o início da linha (snap nos pontos do esboço e nos vértices da vista)",
            (Ferr::Forma(Forma::Linha), _) => "Clique o próximo ponto; botão direito ou Esc termina a polilinha",
            (Ferr::Forma(Forma::Retangulo), 0) => "Clique o primeiro canto do retângulo",
            (Ferr::Forma(Forma::Retangulo), _) => "Clique o canto oposto",
            (Ferr::Forma(Forma::RetanguloCentro), 0) => "Clique o centro do retângulo",
            (Ferr::Forma(Forma::RetanguloCentro), _) => "Clique um canto",
            (Ferr::Forma(Forma::Circulo), 0) => "Clique o centro do círculo",
            (Ferr::Forma(Forma::Circulo), _) => "Clique um ponto da borda",
            (Ferr::Forma(Forma::Circulo3), _) => "Clique três pontos da circunferência",
            (Ferr::Forma(Forma::Arco3), 0) => "Clique o início do arco",
            (Ferr::Forma(Forma::Arco3), 1) => "Clique o fim do arco",
            (Ferr::Forma(Forma::Arco3), _) => "Clique um ponto do meio do arco",
            (Ferr::Forma(Forma::ArcoCentro), 0) => "Clique o centro do arco",
            (Ferr::Forma(Forma::ArcoCentro), 1) => "Clique o início do arco",
            (Ferr::Forma(Forma::ArcoCentro), _) => "Clique o fim do arco (sentido anti-horário)",
            (Ferr::Forma(Forma::Ponto), _) => "Clique onde o ponto vai ficar",
            (Ferr::Cota(TipoCota::Inteligente), _) => "Clique uma linha, um círculo, um arco ou dois pontos; o valor se digita no painel",
            (Ferr::Cota(TipoCota::Horizontal | TipoCota::Vertical), _) => "Clique uma linha ou dois pontos",
            (Ferr::Cota(_), _) => "Clique um círculo ou um arco",
            (Ferr::Restricao(_), _) => "Clique os elementos da restrição; ela se aplica quando a escolha basta",
        }
    }
}

/// Estado da edição do esboço.
#[derive(Default)]
pub struct Estado {
    /// esboço aberto em edição
    pub editando: Option<Id>,
    pub ferr: Ferr,
    /// cliques da forma em andamento, no plano do esboço
    cliques: Vec<Clique>,
    /// seleção (pontos, curvas e restrições do sketch)
    pub sel: Vec<Ent>,
    /// ponto em arraste
    arraste: Option<Id>,
    /// tipo de linha e construção das curvas novas
    pub tipo_novo: TipoLinha,
    pub construcao: bool,
    /// cota em edição no painel: restrição e texto do valor
    pub cota: Option<(Id, String)>,
    /// aviso da última operação (restrição que não coube, valor inalcançável)
    pub aviso: Option<String>,
}

impl Estado {
    pub fn limpar_passo(&mut self) {
        self.cliques.clear();
        self.arraste = None;
    }
}

/// Plano do esboço na folha: identidade quando solto, a vista (posição, escala) quando preso.
enum Plano {
    Folha,
    Vista(Box<(View, Proj)>),
}

impl Plano {
    fn de(st: &mut DwgState, e: &EsbocoDesenho) -> Option<Plano> {
        match e.dono {
            DonoEsboco::Folha(_) => Some(Plano::Folha),
            DonoEsboco::Vista(v) => {
                let view = st.d.view(v)?.clone();
                let p = st.models.proj(&st.d, v);
                (!p.calculando).then(|| Plano::Vista(Box::new((view, p))))
            }
        }
    }
    fn folha(&self, q: V2) -> V2 {
        match self {
            Plano::Folha => q,
            Plano::Vista(b) => layout::to_sheet(&b.0, &b.1, q),
        }
    }
    fn esboco(&self, s: V2) -> V2 {
        match self {
            Plano::Folha => s,
            Plano::Vista(b) => layout::from_sheet(&b.0, &b.1, s),
        }
    }
    /// mm da folha por unidade do esboço
    fn escala(&self) -> f64 {
        match self {
            Plano::Folha => 1.,
            Plano::Vista(b) => b.0.scale.max(1e-9),
        }
    }
}

fn esboco<'a>(st: &'a DwgState) -> Option<&'a EsbocoDesenho> {
    let id = st.esb.editando?;
    st.d.esbocos.iter().find(|e| e.id == id)
}

fn esboco_mut(st: &mut DwgState) -> Option<&mut EsbocoDesenho> {
    let id = st.esb.editando?;
    st.d.esboco_mut(id)
}

/// Abre o esboço `id` em edição.
pub fn editar(st: &mut DwgState, id: Id) {
    st.tool = None;
    st.clicks.clear();
    st.esb = Estado {
        editando: Some(id),
        ferr: Ferr::Selecionar,
        ..Default::default()
    };
    // sem seleção na folha: o esboço em edição mostra os tipos de linha, não o realce
    st.sel = None;
    st.focus = None;
}

/// Fecha a edição. O esboço que ficou vazio some: não há o que mostrar na folha.
pub fn concluir(st: &mut DwgState) {
    if let Some(id) = st.esb.editando.take() {
        if st
            .d
            .esbocos
            .iter()
            .any(|e| e.id == id && e.dados.curves.is_empty())
        {
            st.d.delete(id);
        }
    }
    st.esb = Estado::default();
}

/// Esc: desfaz o passo em andamento, depois volta para Selecionar, depois conclui. Devolve se tratou.
pub fn esc(st: &mut DwgState) -> bool {
    if st.esb.editando.is_none() {
        return false;
    }
    if !st.esb.cliques.is_empty() {
        st.esb.cliques.clear();
    } else if !st.esb.sel.is_empty() {
        st.esb.sel.clear();
    } else if st.esb.ferr != Ferr::Selecionar {
        st.esb.ferr = Ferr::Selecionar;
    } else {
        concluir(st);
        st.sel = None;
    }
    true
}

/// Delete: apaga a seleção do esboço. Devolve se tratou.
pub fn apagar(st: &mut DwgState) -> bool {
    if st.esb.editando.is_none() {
        return false;
    }
    let sel = std::mem::take(&mut st.esb.sel);
    if let Some(e) = esboco_mut(st) {
        crate::sketch_ui::delete_selection(&mut e.dados, &sel);
        e.tipos.retain(|(c, _)| e.dados.curve(*c).is_some());
        e.resolver();
    }
    true
}

/// Antes do esboço existir: o clique numa vista o prende a ela, fora das vistas fica na folha.
pub(super) fn colocar(
    ui: &egui::Ui,
    painter: &egui::Painter,
    st: &mut DwgState,
    resp: &egui::Response,
    mouse: Option<V2>,
    to_scr: &dyn Fn(V2) -> Pos2,
    view_at: &ViewAt,
    rect: Rect,
) {
    let acc = theme::accent(ui.visuals());
    let alvo = mouse.and_then(|m| view_at(st, m));
    let texto = match alvo.and_then(|v| st.d.view(v)) {
        Some(v) => format!("{} {}   (Esc {})", tr("Clique: esboço preso à vista"), v.name, tr("cancela")),
        None => format!("{}   (Esc {})", tr("Clique numa vista para prender o esboço a ela, ou fora das vistas para deixá-lo solto na folha"), tr("cancela")),
    };
    painter.text(
        rect.center_top() + Vec2::new(0., 10.),
        egui::Align2::CENTER_TOP,
        texto,
        egui::FontId::proportional(14.),
        ui.visuals().warn_fg_color,
    );
    // prévia: o contorno de quem vai receber o esboço
    match alvo.and_then(|v| st.boxes.iter().find(|b| b.0 == v)) {
        Some((_, lo, hi)) => {
            let r = Rect::from_two_pos(to_scr(*lo), to_scr(*hi));
            painter.rect_stroke(r, 2., Stroke::new(2., acc), egui::StrokeKind::Outside);
        }
        None => {
            if let Some(m) = mouse {
                let c = to_scr(m);
                painter.line_segment(
                    [c + Vec2::new(-8., 0.), c + Vec2::new(8., 0.)],
                    Stroke::new(1.5, acc),
                );
                painter.line_segment(
                    [c + Vec2::new(0., -8.), c + Vec2::new(0., 8.)],
                    Stroke::new(1.5, acc),
                );
            }
        }
    }
    if resp.clicked_by(egui::PointerButton::Primary) {
        if mouse.is_some() {
            let dono = match alvo {
                Some(v) => DonoEsboco::Vista(v),
                None => DonoEsboco::Folha(st.sheet),
            };
            let id = st.d.add_esboco(dono);
            editar(st, id);
            st.esb.ferr = Ferr::Forma(Forma::Linha);
        }
    }
    if resp.secondary_clicked() {
        st.tool = None;
    }
}

/// Cor de realce do que o cursor pega e do que está selecionado.
fn cor_sel(ui: &egui::Ui) -> Color32 {
    ui.visuals().selection.stroke.color
}

/// Edição: prévia, snap, cliques e arraste. Chamado pela folha a cada quadro com o esboço aberto.
#[allow(clippy::too_many_arguments)]
pub(super) fn tratar(
    ui: &egui::Ui,
    painter: &egui::Painter,
    st: &mut DwgState,
    resp: &egui::Response,
    mouse: Option<V2>,
    to_scr: &dyn Fn(V2) -> Pos2,
    z: f32,
    rect: Rect,
    snap: &SnapFn,
) {
    let Some(e) = esboco(st).cloned() else {
        st.esb = Estado::default();
        return;
    };
    let Some(pl) = Plano::de(st, &e) else {
        painter.text(
            rect.center_top() + Vec2::new(0., 10.),
            egui::Align2::CENTER_TOP,
            tr("A vista ainda está sendo calculada"),
            egui::FontId::proportional(14.),
            ui.visuals().warn_fg_color,
        );
        return;
    };
    let acc = theme::accent(ui.visuals());
    let sel_cor = cor_sel(ui);
    let fraco = ui.visuals().weak_text_color();
    let px = |q: V2| to_scr(pl.folha(q));
    // tolerância do snap: 9 px na tela, no plano do esboço
    let tol = 9. / z as f64 / pl.escala();
    let uv = mouse.map(|m| pl.esboco(m));
    let ferr = st.esb.ferr;
    let n = st.esb.cliques.len();
    painter.text(
        rect.center_top() + Vec2::new(0., 10.),
        egui::Align2::CENTER_TOP,
        format!("{} — {}   (Esc)", e.nome, tr(ferr.dica(n))),
        egui::FontId::proportional(14.),
        ui.visuals().warn_fg_color,
    );

    // curvas de construção (não saem na folha) e pontos do esboço
    let s = &e.dados;
    let marcada = |x: Ent| st.esb.sel.contains(&x);
    let sob = uv.and_then(|q| ed::mais_perto(&e, q, tol));
    for c in &s.curves {
        let pts: Vec<Pos2> = ed::curva_pts(s, c).into_iter().map(px).collect();
        let quente = sob == Some(Alvo::Curva(c.id));
        let cor = if marcada(Ent::Cv(c.id)) {
            sel_cor
        } else if quente {
            acc
        } else {
            fraco
        };
        if pts.len() == 1 {
            painter.circle_stroke(pts[0], 4., Stroke::new(1.5, cor));
        } else if c.construction {
            painter.extend(egui::Shape::dashed_line(
                &pts,
                Stroke::new(1.2, cor),
                6.,
                4.,
            ));
        } else if quente || marcada(Ent::Cv(c.id)) {
            painter.add(egui::Shape::line(pts, Stroke::new(2.5, cor)));
        }
    }
    for p in &s.points {
        let cor = if marcada(Ent::Pt(p.id)) {
            sel_cor
        } else if sob == Some(Alvo::Ponto(p.id)) {
            acc
        } else {
            fraco
        };
        if p.fixed {
            painter.rect_filled(
                Rect::from_center_size(px([p.x, p.y]), Vec2::splat(6.)),
                0.,
                cor,
            );
        } else {
            painter.circle_filled(px([p.x, p.y]), 3., cor);
        }
    }

    // snap do clique: ponto do esboço, depois vértice da vista (fica preso ali)
    let clique_em = |st: &mut DwgState, m: V2| -> Clique {
        let q = pl.esboco(m);
        if let Some(Alvo::Ponto(p)) = ed::mais_perto(&e, q, tol) {
            return Clique {
                uv: e.dados.pos(p),
                ponto: Some(p),
                fixo: false,
            };
        }
        let so = match e.dono {
            DonoEsboco::Vista(v) => Some(v),
            DonoEsboco::Folha(_) => None,
        };
        if let Some((_, s, q3)) = snap(st, m, so) {
            let uv = if so.is_some() { q3 } else { s };
            return Clique {
                uv,
                ponto: None,
                fixo: true,
            };
        }
        Clique::solto(q)
    };
    let alvo_snap = mouse.map(|m| clique_em(st, m));
    if let (Some(c), Ferr::Forma(_)) = (alvo_snap, ferr) {
        if c.ponto.is_some() || c.fixo {
            painter.circle_stroke(px(c.uv), 6., Stroke::new(2., acc));
        }
    }

    // prévia da forma em andamento (nada é criado antes do último clique)
    if let (Ferr::Forma(f), Some(c)) = (ferr, alvo_snap) {
        let mut cl = st.esb.cliques.clone();
        if !cl.is_empty() || f == Forma::Ponto {
            cl.push(c);
            if f == Forma::Circulo3 || f == Forma::Arco3 || f == Forma::ArcoCentro {
                while cl.len() < 3 {
                    cl.push(c);
                }
            }
            let mut g = e.clone();
            if ed::criar(&mut g, f, &cl, st.esb.construcao, st.esb.tipo_novo).is_ok() {
                for cv in g
                    .dados
                    .curves
                    .iter()
                    .filter(|x| e.dados.curve(x.id).is_none())
                {
                    let pts: Vec<Pos2> = ed::curva_pts(&g.dados, cv).into_iter().map(px).collect();
                    if pts.len() > 1 {
                        painter.extend(egui::Shape::dashed_line(
                            &pts,
                            Stroke::new(1.6, acc),
                            7.,
                            4.,
                        ));
                    }
                }
            } else if let (Some(a), Some(m)) = (st.esb.cliques.first(), mouse) {
                painter.line_segment([px(a.uv), to_scr(m)], Stroke::new(1., acc));
            }
        }
        for c in &st.esb.cliques {
            painter.circle_filled(px(c.uv), 3.5, acc);
        }
    }

    // ---------- cliques
    if resp.clicked_by(egui::PointerButton::Primary) {
        if let Some(m) = mouse {
            let shift = ui.input(|i| i.modifiers.shift);
            clicar(st, m, &clique_em, sob, shift);
        }
    }
    if resp.secondary_clicked() {
        // botão direito termina a polilinha (ou a forma em andamento)
        st.esb.cliques.clear();
    }
    // ---------- arraste de pontos (Selecionar)
    if ferr == Ferr::Selecionar {
        if resp.drag_started_by(egui::PointerButton::Primary) {
            if let Some(Alvo::Ponto(p)) = sob {
                st.esb.arraste = Some(p);
                super::alcas::begin_drag(st);
            }
        } else if resp.dragged_by(egui::PointerButton::Primary) {
            if let (Some(p), Some(q)) = (st.esb.arraste, uv) {
                if let Some(e) = esboco_mut(st) {
                    ed::arrastar(e, p, q);
                }
            }
        }
        if resp.drag_stopped() {
            st.esb.arraste = None;
        }
    }
}

fn clicar(
    st: &mut DwgState,
    m: V2,
    clique_em: &dyn Fn(&mut DwgState, V2) -> Clique,
    sob: Option<Alvo>,
    shift: bool,
) {
    let ferr = st.esb.ferr;
    st.esb.aviso = None;
    let ent = |a: Alvo| match a {
        Alvo::Ponto(p) => Ent::Pt(p),
        Alvo::Curva(c) => Ent::Cv(c),
    };
    match ferr {
        Ferr::Selecionar => {
            match sob.map(ent) {
                Some(x) if shift => {
                    if let Some(i) = st.esb.sel.iter().position(|y| *y == x) {
                        st.esb.sel.remove(i);
                    } else {
                        st.esb.sel.push(x);
                    }
                }
                Some(x) => st.esb.sel = vec![x],
                None => st.esb.sel.clear(),
            }
            abrir_cota_da_selecao(st);
        }
        Ferr::Forma(f) => {
            let c = clique_em(st, m);
            st.esb.cliques.push(c);
            if st.esb.cliques.len() < f.cliques() {
                return;
            }
            let cl = std::mem::take(&mut st.esb.cliques);
            let (cons, tipo) = (st.esb.construcao, st.esb.tipo_novo);
            let Some(e) = esboco_mut(st) else { return };
            match ed::criar(e, f, &cl, cons, tipo) {
                Ok(novas) => {
                    // a polilinha continua do fim da última linha, até fechar num ponto que já existia
                    if f == Forma::Linha {
                        let fechou = cl[1].ponto.is_some();
                        if let Some(fim) = novas
                            .first()
                            .and_then(|l| ed::fim_da_linha(e, *l))
                            .filter(|_| !fechou)
                        {
                            let uv = e.dados.pos(fim);
                            st.esb.cliques = vec![Clique {
                                uv,
                                ponto: Some(fim),
                                fixo: false,
                            }];
                        }
                    }
                }
                Err(motivo) => st.esb.aviso = Some(motivo.to_string()),
            }
        }
        Ferr::Cota(t) => {
            let Some(a) = sob else { return };
            if !st.esb.sel.contains(&ent(a)) {
                st.esb.sel.push(ent(a));
            }
            let alvos: Vec<Alvo> = st
                .esb
                .sel
                .iter()
                .filter_map(|x| match x {
                    Ent::Pt(p) => Some(Alvo::Ponto(*p)),
                    Ent::Cv(c) => Some(Alvo::Curva(*c)),
                    Ent::Cs(_) => None,
                })
                .collect();
            let Some(e) = esboco(st) else { return };
            let cota = match t {
                TipoCota::Inteligente => ed::cota_inteligente(e, &alvos),
                TipoCota::Horizontal => {
                    ed::pontos_da_cota(e, &alvos).map(|(a, b)| Cota::Horizontal(a, b))
                }
                TipoCota::Vertical => {
                    ed::pontos_da_cota(e, &alvos).map(|(a, b)| Cota::Vertical(a, b))
                }
                TipoCota::Raio | TipoCota::Diametro => match alvos.as_slice() {
                    [Alvo::Curva(c)] if e.dados.radius_of(*c).is_some() => {
                        Some(if t == TipoCota::Raio {
                            Cota::Raio(*c)
                        } else {
                            Cota::Diametro(*c)
                        })
                    }
                    _ => None,
                },
            };
            let Some(cota) = cota else {
                // dois cliques sem cota possível: recomeça pelo último
                if alvos.len() >= 2 {
                    st.esb.sel = vec![ent(a)];
                }
                return;
            };
            st.esb.sel.clear();
            let Some(e) = esboco_mut(st) else { return };
            if let Some(id) = ed::cotar(e, cota) {
                let v = ed::valor(e, id).unwrap_or_default();
                st.esb.sel = vec![Ent::Cs(id)];
                st.esb.cota = Some((id, v));
            }
        }
        Ferr::Restricao(k) => {
            let Some(a) = sob else { return };
            if !st.esb.sel.contains(&ent(a)) {
                st.esb.sel.push(ent(a));
            }
            let sel = st.esb.sel.clone();
            let pronta = sel.len() >= k.needs()
                || (matches!(k, CKind::Horizontal | CKind::Vertical)
                    && matches!(a, Alvo::Curva(_)));
            if !pronta {
                return;
            }
            st.esb.sel.clear();
            let Some(e) = esboco_mut(st) else { return };
            let mut erro = None;
            let ok = ed::com_resolucao(e, |s| {
                let mut al = ed::alocador(s);
                match apply_constraint(s, &mut al, k, &sel) {
                    Ok(()) => true,
                    Err(m) => {
                        erro = Some(m);
                        false
                    }
                }
            });
            if !ok {
                st.esb.aviso = Some(
                    erro.unwrap_or("a restrição entra em conflito com as que já existem")
                        .to_string(),
                );
            }
        }
    }
}

/// Cota selecionada: o valor dela abre no painel.
fn abrir_cota_da_selecao(st: &mut DwgState) {
    let cs = match st.esb.sel.as_slice() {
        [Ent::Cs(c)] => Some(*c),
        _ => None,
    };
    st.esb.cota = cs.and_then(|c| esboco(st).and_then(|e| ed::valor(e, c)).map(|v| (c, v)));
}

fn botao(ui: &mut egui::Ui, st: &mut DwgState, f: Ferr) {
    let (icone, nome, breve, _) = f.do_sketch().info();
    let r = crate::icons::tool_button(
        ui,
        icone,
        nome,
        26.,
        false,
        st.esb.ferr == f,
        breve,
        f.dica(0),
    );
    if r.clicked() {
        st.esb.ferr = f;
        st.esb.limpar_passo();
        if !matches!(f, Ferr::Selecionar) {
            st.esb.sel.clear();
        }
    }
}

fn tipo_combo(ui: &mut egui::Ui, salt: &str, t: &mut TipoLinha) -> bool {
    let antes = *t;
    egui::ComboBox::from_id_salt(salt)
        .selected_text(tr(t.label()))
        .show_ui(ui, |ui| {
            for x in TipoLinha::TODOS {
                ui.selectable_value(t, x, tr(x.label()));
            }
        });
    *t != antes
}

/// Painel do esboço: o que está em edição ou o selecionado na folha. Devolve se mostrou.
pub(super) fn painel(ui: &mut egui::Ui, st: &mut DwgState) -> bool {
    if st.tool == Some(c::ESBOCO) {
        ui.heading(tr("Esboço"));
        ui.label(tr("Clique numa vista para prender o esboço a ela, ou fora das vistas para deixá-lo solto na folha"));
        ui.label(RichText::new(tr("preso à vista: anda e escala com ela, e as cotas mostram a medida do modelo; solto: mm da folha")).weak().small());
        return true;
    }
    if st.esb.editando.is_none() {
        let Some(id) = st.sel.filter(|s| st.d.esbocos.iter().any(|e| e.id == *s)) else {
            return false;
        };
        ui.heading(tr("Esboço"));
        dono_e_nome(ui, st, id);
        if ui
            .button(tr("✎ Editar o esboço"))
            .on_hover_text(tr(
                "abre as ferramentas do sketch: formas, cotas e restrições",
            ))
            .clicked()
        {
            editar(st, id);
        }
        return true;
    }
    let Some(id) = st.esb.editando else {
        return false;
    };
    ui.heading(tr("Esboço"));
    dono_e_nome(ui, st, id);
    ui.separator();
    ui.label(RichText::new(tr("Desenhar")).strong());
    ui.horizontal_wrapped(|ui| {
        botao(ui, st, Ferr::Selecionar);
        for f in Forma::TODAS {
            botao(ui, st, Ferr::Forma(f));
        }
    });
    ui.horizontal(|ui| {
        ui.label(tr("Linha nova"));
        tipo_combo(ui, "esb_tipo_novo", &mut st.esb.tipo_novo);
    })
    .response
    .on_hover_text(tr(
        "tipo de linha das curvas novas na folha: visível, fina, oculta ou de centro",
    ));
    ui.checkbox(&mut st.esb.construcao, tr("Construção"))
        .on_hover_text(tr(
        "curva de apoio: ajuda a desenhar e a restringir, mas não sai na folha nem na exportação",
    ));
    ui.label(RichText::new(tr("Cotas")).strong());
    ui.horizontal_wrapped(|ui| {
        for c in COTAS {
            botao(ui, st, Ferr::Cota(c));
        }
    });
    ui.label(RichText::new(tr("Restrições")).strong());
    ui.horizontal_wrapped(|ui| {
        for k in RESTRICOES {
            botao(ui, st, Ferr::Restricao(k));
        }
    });
    // cota em edição: valor do sketch (aceita expressões)
    if let Some((cid, mut texto)) = st.esb.cota.clone() {
        ui.separator();
        let atual = esboco(st).and_then(|e| ed::medida(e, cid));
        let r = ui
            .horizontal(|ui| {
                ui.label(tr("Valor da cota"));
                ui.add(egui::TextEdit::singleline(&mut texto).desired_width(80.))
            })
            .inner
            .on_hover_text(tr("medida no plano do esboço (mm do modelo quando preso à vista); Enter aplica e resolve"));
        if r.changed() {
            st.esb.cota = Some((cid, texto.clone()));
        }
        if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            let ok = esboco_mut(st).is_some_and(|e| ed::definir_valor(e, cid, &texto));
            if !ok {
                st.esb.aviso = Some(
                    tr("o esboço não chega nesse valor: a cota voltou ao anterior").to_string(),
                );
                st.esb.cota = esboco(st).and_then(|e| ed::valor(e, cid)).map(|v| (cid, v));
            }
        }
        if let Some(v) = atual {
            ui.label(
                RichText::new(format!("{} {}", tr("medida atual"), dec_txt(v)))
                    .weak()
                    .small(),
            );
        }
        if let Some(e) = esboco_mut(st) {
            if let Some(k) = e.dados.constraints.iter_mut().find(|c| c.id == cid) {
                ui.checkbox(&mut k.reference, tr("Cota de referência"))
                    .on_hover_text(tr("mostra a medida e não comanda a geometria"));
            }
        }
    }
    // seleção: tipo de linha e construção das curvas escolhidas
    let curvas: Vec<Id> = st
        .esb
        .sel
        .iter()
        .filter_map(|x| if let Ent::Cv(c) = x { Some(*c) } else { None })
        .collect();
    if !curvas.is_empty() {
        ui.separator();
        ui.label(
            RichText::new(format!("{} ({})", tr("Curvas selecionadas"), curvas.len())).strong(),
        );
        if let Some(e) = esboco_mut(st) {
            let mut t = e.tipo(curvas[0]);
            let r = ui.horizontal(|ui| {
                ui.label(tr("Tipo de linha"));
                tipo_combo(ui, "esb_tipo_sel", &mut t)
            });
            r.response
                .on_hover_text(tr("como a curva sai na folha e na exportação"));
            if r.inner {
                for c in &curvas {
                    e.definir_tipo(*c, t);
                }
            }
            let mut cons = e.dados.curve(curvas[0]).is_some_and(|c| c.construction);
            if ui.checkbox(&mut cons, tr("Construção")).on_hover_text(tr("curva de apoio: ajuda a desenhar e a restringir, mas não sai na folha nem na exportação")).changed() {
                for c in e.dados.curves.iter_mut().filter(|c| curvas.contains(&c.id)) {
                    c.construction = cons;
                }
            }
        }
    }
    if !st.esb.sel.is_empty()
        && ui
            .button(tr("🗑 Apagar a seleção"))
            .on_hover_text(tr("Delete: apaga os pontos, curvas e cotas escolhidos"))
            .clicked()
    {
        apagar(st);
    }
    ui.separator();
    if let Some(e) = esboco_mut(st) {
        ui.checkbox(&mut e.cotas, tr("Mostrar as cotas na folha"))
            .on_hover_text(tr(
                "as cotas do esboço saem como cotas do desenho, com a medida do modelo",
            ));
        let ok = e.dados.clone().solve(&Default::default(), &[]).converged;
        let (cor, txt) = if ok {
            (theme::st_ok(), "Resolvido")
        } else {
            (theme::st_err(), "Conflito entre restrições")
        };
        ui.colored_label(cor, tr(txt));
        ui.label(
            RichText::new(format!(
                "{} {} · {} {} · {} {}",
                e.dados.curves.len(),
                tr("curvas"),
                e.dados.points.len(),
                tr("pontos"),
                e.dados.constraints.len(),
                tr("restrições")
            ))
            .weak()
            .small(),
        );
    }
    if let Some(a) = &st.esb.aviso {
        ui.colored_label(theme::st_warn(), tr(a));
    }
    true
}

fn dono_e_nome(ui: &mut egui::Ui, st: &mut DwgState, id: Id) {
    let dono = st.d.esbocos.iter().find(|e| e.id == id).map(|e| e.dono);
    let onde = match dono {
        Some(DonoEsboco::Vista(v)) => format!(
            "{} {}",
            tr("Preso à vista"),
            st.d.view(v).map(|v| v.name.clone()).unwrap_or_default()
        ),
        _ => tr("Solto na folha").to_string(),
    };
    if let Some(e) = st.d.esboco_mut(id) {
        ui.horizontal(|ui| {
            ui.label(tr("Nome"));
            ui.text_edit_singleline(&mut e.nome);
        });
    }
    ui.label(RichText::new(onde).weak()).on_hover_text(tr("preso à vista: anda e escala com ela, e as cotas mostram a medida do modelo; solto: mm da folha"));
}

/// Desloca o esboço arrastado na folha (`dm` em mm da folha).
pub(super) fn mover(st: &mut DwgState, id: Id, dm: V2) -> bool {
    let Some(e) = st.d.esbocos.iter().find(|e| e.id == id) else {
        return false;
    };
    let k = fenkra_desenho::esboco::escala(&st.d, e);
    if let Some(e) = st.d.esboco_mut(id) {
        ed::mover(e, [dm[0] / k, dm[1] / k]);
    }
    true
}

/// Desenvolvimento (`FENKRA_DEMO_DWG_ESBOCO`): `colocar` arma o comando; `vista` ou `folha` cria um
/// esboço de exemplo (retângulo cotado, furo, eixo de centro e uma linha de construção) e o abre em
/// edição, com a cota do furo selecionada para o painel mostrar o valor. Nada disso grava no disco.
pub(super) fn demo(st: &mut DwgState) {
    let modo = std::env::var("FENKRA_DEMO_DWG_ESBOCO").unwrap_or_default();
    if modo == "colocar" {
        st.tool = Some(c::ESBOCO);
        return;
    }
    // a vista superior fica longe do painel do comando, que se abre à esquerda
    let na_folha = st.d.views.iter().filter(|v| v.sheet == st.sheet);
    let vista = na_folha
        .clone()
        .find(|v| matches!(v.kind, ViewKind::Projected { dir: Dir::Topo, .. }))
        .or(na_folha.clone().next())
        .map(|v| v.id);
    let (dono, o) = match (modo.as_str(), vista) {
        ("vista", Some(v)) => {
            let p = st.models.proj(&st.d, v);
            let c = if p.calculando { [0., 0.] } else { p.center };
            (DonoEsboco::Vista(v), [c[0] - 20., c[1] - 30.])
        }
        ("folha", _) => (DonoEsboco::Folha(st.sheet), [215., 50.]),
        _ => return,
    };
    let id = st.d.add_esboco(dono);
    let Some(e) = st.d.esboco_mut(id) else { return };
    let q = |x: f64, y: f64| Clique::solto([o[0] + x, o[1] + y]);
    let r = ed::criar(
        e,
        Forma::Retangulo,
        &[q(0., 0.), q(40., 18.)],
        false,
        TipoLinha::Visivel,
    )
    .unwrap_or_default();
    let furo = ed::criar(
        e,
        Forma::Circulo,
        &[q(20., 9.), q(25., 9.)],
        false,
        TipoLinha::Visivel,
    )
    .unwrap_or_default();
    ed::criar(
        e,
        Forma::Linha,
        &[q(-5., 9.), q(45., 9.)],
        false,
        TipoLinha::Centro,
    )
    .ok();
    ed::criar(
        e,
        Forma::Linha,
        &[q(0., 0.), q(40., 18.)],
        true,
        TipoLinha::Visivel,
    )
    .ok();
    if let Some(l) = r.first() {
        ed::cotar(e, Cota::Comprimento(*l));
    }
    let cota_furo = furo.first().and_then(|c| ed::cotar(e, Cota::Diametro(*c)));
    editar(st, id);
    st.esb.ferr = Ferr::Cota(TipoCota::Inteligente);
    if let Some(cid) = cota_furo {
        st.esb.sel = vec![Ent::Cs(cid)];
        abrir_cota_da_selecao(st);
    }
    if let Some(l) = r.get(1) {
        st.esb.sel.push(Ent::Cv(*l));
    }
}
