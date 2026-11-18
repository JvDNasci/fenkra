// SPDX-License-Identifier: LGPL-2.1-or-later
//! B506: barra de propriedades gráficas, aplicada à seleção. Nasce no topo e é uma barra do sistema do
//! B502 (solta, encaixa em qualquer borda, fecha no "x" e se restaura).
//!
//! Campos: cor, transparência, tipo e espessura de linha, símbolo de ponto, camada, material e
//! textura. O campo que não vale para o que está selecionado fica desabilitado, com a dica do motivo,
//! para a barra não pular de tamanho. Seleção com valores diferentes mostra "vários"; escolher um
//! valor aplica a todos.
//!
//! Só exibição: a mudança mexe em aparência, estilo de linha e transparência, que o recálculo ignora
//! (`geo_stale_from`), então nada é recalculado. Cada escolha é uma mudança só do documento, e o
//! desfazer a trata como qualquer outra.

use crate::app::{App, Sel};
use crate::i18n::tr;
use crate::picking::Pick;
use eframe::egui::{self, RichText};
use fenkra_core::{Appearance, Document, FeatureKind, Finish, Id, LineStyle, RefClass, APPEARANCE_PRESETS};

/// O que a barra edita.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Alvo {
    Corpo(Id),
    /// faces geradas por um comando de sólido
    Recurso(Id),
    Ref(Id, RefClass),
    /// curva de sketch
    CurvaSketch(Id),
    /// ponto de sketch
    PontoSketch(Id),
}

/// Valor de um campo para a seleção inteira.
#[derive(Clone, PartialEq, Debug, Default)]
pub enum Valor<T> {
    /// nada selecionado aceita o campo
    #[default]
    Nenhum,
    Um(T),
    Varios,
}

impl<T: PartialEq + Clone> Valor<T> {
    fn juntar(&mut self, v: T) {
        *self = match std::mem::take(self) {
            Valor::Nenhum => Valor::Um(v),
            Valor::Um(a) if a == v => Valor::Um(a),
            _ => Valor::Varios,
        };
    }
    pub fn aplica(&self) -> bool {
        !matches!(self, Valor::Nenhum)
    }
    pub fn um(&self) -> Option<&T> {
        if let Valor::Um(v) = self { Some(v) } else { None }
    }
}

/// O que a barra mostra.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Estado {
    pub alvos: Vec<Alvo>,
    pub cor: Valor<[u8; 3]>,
    pub transparencia: Valor<u8>,
    pub espessura: Valor<f32>,
    pub tipo_linha: Valor<u8>,
    pub simbolo: Valor<u8>,
    /// nome do material (aparência pronta); "" = personalizada
    pub material: Valor<String>,
    pub textura: Valor<Option<u8>>,
    pub texturas: Vec<String>,
}

/// Uma escolha na barra.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Mudanca {
    Cor([u8; 3]),
    Transparencia(u8),
    Espessura(f32),
    TipoLinha(u8),
    Simbolo(u8),
    /// índice em `APPEARANCE_PRESETS`
    Material(usize),
    Textura(Option<u8>),
    /// B520: acabamento sozinho (o pincel copia o do material sem trocar a cor)
    Acabamento(Finish),
}

pub const TIPOS_DE_LINHA: [&str; 4] = ["Contínua", "Tracejada", "Traço-ponto", "Pontilhada"];
pub const SIMBOLOS: [&str; 6] = ["Cruz 3D", "Asterisco", "X", "Círculo", "Quadrado", "Ponto"];
pub(crate) const ESTILO_PADRAO: LineStyle = LineStyle { width: 2., dash: 0, point: 1, size: 8. };
const COR_REF: [u8; 3] = [222, 170, 110];

/// Alvos da seleção: faces e elementos clicados no 3D, mais o item (ou os itens) da árvore.
pub fn alvos(d: &Document, sel: Sel, multi: &[Id], picks: &[Pick]) -> Vec<Alvo> {
    let mut v: Vec<Alvo> = vec![];
    let mut por = |a: Option<Alvo>| {
        if let Some(a) = a.filter(|a| !v.contains(a)) {
            v.push(a);
        }
    };
    let de_id = |id: Id| -> Option<Alvo> {
        if let Some(r) = d.reference(id) {
            return Some(Alvo::Ref(id, r.kind.class()));
        }
        if d.collectors.iter().any(|c| c.id == id) {
            return Some(Alvo::Corpo(id));
        }
        d.feature(id).filter(|f| !matches!(f.kind, FeatureKind::Sketch(_))).map(|_| Alvo::Recurso(id))
    };
    for p in picks {
        por(match p {
            Pick::Face { owner, .. } => de_id(*owner),
            Pick::Geo(id) => de_id(*id),
            Pick::SkCurve { curve, .. } => Some(Alvo::CurvaSketch(*curve)),
            Pick::SkPoint { point, .. } => Some(Alvo::PontoSketch(*point)),
            _ => None,
        });
    }
    if multi.len() > 1 {
        for id in multi {
            por(de_id(*id));
        }
    } else {
        por(match sel {
            Sel::Ref(id) | Sel::Collector(id) | Sel::Feature(id) => de_id(id),
            _ => None,
        });
    }
    v
}

/// Aparência em vigor do alvo (a própria ou a herdada), se o alvo tiver cor.
pub(crate) fn aparencia(d: &Document, a: Alvo) -> Option<Appearance> {
    let do_corpo = |id: Id| d.collectors.iter().find(|c| c.id == id).map(|c| Appearance { color: c.color, finish: c.finish, texture: c.texture });
    match a {
        Alvo::Corpo(id) => do_corpo(id),
        Alvo::Recurso(id) => d.feature(id).and_then(|f| f.appearance.or_else(|| do_corpo(f.target))),
        Alvo::Ref(id, _) => d.reference(id).map(|r| r.appearance.unwrap_or(Appearance { color: COR_REF, finish: Finish::Acetinado, texture: None })),
        Alvo::CurvaSketch(_) | Alvo::PontoSketch(_) => None,
    }
}

pub(crate) fn tem_face(a: Alvo) -> bool {
    matches!(a, Alvo::Corpo(_) | Alvo::Recurso(_) | Alvo::Ref(_, RefClass::Surface))
}
pub(crate) fn tem_linha(a: Alvo) -> bool {
    matches!(a, Alvo::Ref(_, RefClass::Curve) | Alvo::CurvaSketch(_))
}
pub(crate) fn tem_ponto(a: Alvo) -> bool {
    matches!(a, Alvo::Ref(_, RefClass::Point) | Alvo::PontoSketch(_))
}
pub(crate) fn id_de(a: Alvo) -> Id {
    match a {
        Alvo::Corpo(id) | Alvo::Recurso(id) | Alvo::Ref(id, _) | Alvo::CurvaSketch(id) | Alvo::PontoSketch(id) => id,
    }
}

/// Lê os valores da seleção.
pub fn ler(d: &Document, alvos: &[Alvo]) -> Estado {
    let mut e = Estado { alvos: alvos.to_vec(), texturas: d.textures.iter().map(|t| t.name.clone()).collect(), ..Default::default() };
    for a in alvos {
        if let Some(ap) = aparencia(d, *a) {
            e.cor.juntar(ap.color);
            if tem_face(*a) {
                let nome = APPEARANCE_PRESETS.iter().find(|(_, p)| p.color == ap.color && p.finish == ap.finish).map_or(String::new(), |(n, _)| n.to_string());
                e.material.juntar(nome);
                e.textura.juntar(ap.texture);
            }
        }
        if tem_face(*a) {
            e.transparencia.juntar(d.transparencia(id_de(*a)).unwrap_or(0));
        }
        let st = d.line_style(id_de(*a)).unwrap_or(ESTILO_PADRAO);
        if tem_linha(*a) {
            e.espessura.juntar(st.width);
            e.tipo_linha.juntar(st.dash);
        }
        if tem_ponto(*a) {
            e.simbolo.juntar(st.point);
        }
    }
    e
}

/// Grava a aparência do alvo com `f` aplicada sobre a que está em vigor.
fn mudar_aparencia(d: &mut Document, a: Alvo, f: impl Fn(&mut Appearance)) -> bool {
    let Some(mut ap) = aparencia(d, a) else { return false };
    f(&mut ap);
    match a {
        Alvo::Corpo(id) => {
            if let Some(c) = d.collectors.iter_mut().find(|c| c.id == id) {
                (c.color, c.finish, c.texture) = (ap.color, ap.finish, ap.texture);
            }
        }
        Alvo::Recurso(id) => {
            if let Some(x) = d.feature_mut(id) {
                x.appearance = Some(ap);
            }
        }
        Alvo::Ref(id, _) => {
            if let Some(r) = d.references.iter_mut().find(|r| r.id == id) {
                r.appearance = Some(ap);
            }
        }
        _ => return false,
    }
    true
}

fn mudar_estilo(d: &mut Document, a: Alvo, f: impl Fn(&mut LineStyle)) {
    let id = id_de(a);
    let mut st = d.line_style(id).unwrap_or(ESTILO_PADRAO);
    f(&mut st);
    d.set_line_style(id, Some(st));
}

/// Aplica a escolha a todos os alvos que aceitam o campo. Devolve quantos mudaram.
pub fn aplicar(d: &mut Document, alvos: &[Alvo], m: Mudanca) -> usize {
    let mut n = 0;
    for a in alvos.iter().copied() {
        let ok = match m {
            Mudanca::Cor(c) => mudar_aparencia(d, a, |ap| ap.color = c),
            Mudanca::Material(i) if tem_face(a) => match APPEARANCE_PRESETS.get(i) {
                Some((_, p)) => mudar_aparencia(d, a, |ap| (ap.color, ap.finish) = (p.color, p.finish)),
                None => false,
            },
            Mudanca::Textura(t) if tem_face(a) => mudar_aparencia(d, a, |ap| ap.texture = t),
            Mudanca::Acabamento(f) if tem_face(a) => mudar_aparencia(d, a, |ap| ap.finish = f),
            Mudanca::Transparencia(t) if tem_face(a) => {
                d.set_transparencia(id_de(a), t);
                true
            }
            Mudanca::Espessura(w) if tem_linha(a) => {
                mudar_estilo(d, a, |s| s.width = w.clamp(0.5, 12.));
                true
            }
            Mudanca::TipoLinha(t) if tem_linha(a) => {
                mudar_estilo(d, a, |s| s.dash = t.min(3));
                true
            }
            Mudanca::Simbolo(p) if tem_ponto(a) => {
                mudar_estilo(d, a, |s| s.point = p.min(5));
                true
            }
            _ => false,
        };
        n += usize::from(ok);
    }
    n
}

/// Motivo de um campo desabilitado.
fn motivo(e: &Estado, campo: &str) -> &'static str {
    if e.alvos.is_empty() {
        return "Selecione uma face, um corpo, um comando ou um elemento de geometria.";
    }
    match campo {
        "cor" => "A cor vale para faces, corpos e elementos de geometria; curva e ponto de sketch usam espessura, tipo de linha e símbolo.",
        "linha" => "Espessura e tipo de linha valem só para linhas e curvas.",
        "ponto" => "O símbolo vale só para pontos.",
        _ => "Material, textura e transparência valem só para faces, corpos e superfícies.",
    }
}

/// Rótulo de um combo: o valor, "vários" ou nada.
fn rotulo<T: PartialEq + Clone>(v: &Valor<T>, nome: impl Fn(&T) -> String) -> String {
    match v {
        Valor::Um(x) => nome(x),
        Valor::Varios => tr("vários").to_string(),
        Valor::Nenhum => "—".into(),
    }
}

/// Campo da barra: habilitado ou com a dica do motivo; a largura não muda.
fn campo(ui: &mut egui::Ui, ativo: bool, motivo: &str, f: impl FnOnce(&mut egui::Ui)) {
    let r = ui.add_enabled_ui(ativo, f).response;
    if !ativo {
        r.on_hover_text(tr(motivo));
    }
}

/// A barra. Devolve a escolha feita neste quadro.
pub fn barra(ui: &mut egui::Ui, e: &Estado) -> Option<Mudanca> {
    let mut m = None;
    let fraco = |ui: &mut egui::Ui, t: &str| {
        ui.label(RichText::new(tr(t)).small().weak());
    };
    // cor
    campo(ui, e.cor.aplica(), motivo(e, "cor"), |ui| {
        fraco(ui, "Cor");
        let mut c = e.cor.um().copied().unwrap_or([200, 200, 200]);
        let r = ui.color_edit_button_srgb(&mut c).on_hover_text(tr("Cor da seleção (faces, corpos, curvas e pontos de referência)"));
        if r.changed() {
            m = Some(Mudanca::Cor(c));
        }
        if e.cor == Valor::Varios {
            fraco(ui, "vários");
        }
    });
    ui.separator();
    // transparência
    campo(ui, e.transparencia.aplica(), motivo(e, "face"), |ui| {
        fraco(ui, "Transparência");
        let mut t = e.transparencia.um().copied().unwrap_or(0);
        let s = crate::ui_ext::slider(&mut t, 0..=85).suffix(" %").text(if e.transparencia == Valor::Varios { tr("vários") } else { "" });
        if ui.add_sized([120., 18.], s).on_hover_text(tr("Até 85 %: a peça não some no fundo e as arestas continuam à vista.")).changed() {
            m = Some(Mudanca::Transparencia(t));
        }
    });
    ui.separator();
    // linha: tipo e espessura
    campo(ui, e.tipo_linha.aplica(), motivo(e, "linha"), |ui| {
        fraco(ui, "Linha");
        egui::ComboBox::from_id_salt("pg_tipo_linha").width(96.).selected_text(rotulo(&e.tipo_linha, |t| tr(TIPOS_DE_LINHA[(*t as usize).min(3)]).to_string())).show_ui(ui, |ui| {
            for (i, n) in TIPOS_DE_LINHA.iter().enumerate() {
                if ui.selectable_label(e.tipo_linha.um() == Some(&(i as u8)), tr(n)).clicked() {
                    m = Some(Mudanca::TipoLinha(i as u8));
                }
            }
        });
        let mut w = e.espessura.um().copied().unwrap_or(2.);
        let r = ui.add(crate::ui_ext::dv(&mut w).range(0.5..=12.).speed(0.1).suffix(" px")).on_hover_text(tr("Espessura da linha"));
        if r.changed() {
            m = Some(Mudanca::Espessura(w));
        }
        if e.espessura == Valor::Varios {
            fraco(ui, "vários");
        }
    });
    ui.separator();
    // símbolo de ponto
    campo(ui, e.simbolo.aplica(), motivo(e, "ponto"), |ui| {
        fraco(ui, "Ponto");
        egui::ComboBox::from_id_salt("pg_simbolo").width(84.).selected_text(rotulo(&e.simbolo, |p| tr(SIMBOLOS[(*p as usize).min(5)]).to_string())).show_ui(ui, |ui| {
            for (i, n) in SIMBOLOS.iter().enumerate() {
                if ui.selectable_label(e.simbolo.um() == Some(&(i as u8)), tr(n)).clicked() {
                    m = Some(Mudanca::Simbolo(i as u8));
                }
            }
        });
    });
    ui.separator();
    // camada: ainda não existe no documento; o campo fica para a barra não mudar quando existir
    campo(ui, false, "Camadas ainda não existem nesta versão: tudo fica sem camada.", |ui| {
        fraco(ui, "Camada");
        egui::ComboBox::from_id_salt("pg_camada").width(84.).selected_text(tr("sem camada")).show_ui(ui, |_| {});
    });
    ui.separator();
    // material e textura
    campo(ui, e.material.aplica(), motivo(e, "face"), |ui| {
        fraco(ui, "Material");
        let texto = rotulo(&e.material, |n| if n.is_empty() { tr("personalizado").to_string() } else { tr(n).to_string() });
        egui::ComboBox::from_id_salt("pg_material").width(110.).selected_text(texto).show_ui(ui, |ui| {
            for (i, (n, _)) in APPEARANCE_PRESETS.iter().enumerate() {
                if ui.selectable_label(e.material.um().is_some_and(|x| x == n), tr(n)).clicked() {
                    m = Some(Mudanca::Material(i));
                }
            }
        });
        fraco(ui, "Textura");
        let nome_tex = |t: &Option<u8>| t.and_then(|i| e.texturas.get(i as usize).cloned()).unwrap_or_else(|| tr("sem textura").to_string());
        egui::ComboBox::from_id_salt("pg_textura").width(96.).selected_text(rotulo(&e.textura, nome_tex)).show_ui(ui, |ui| {
            if ui.selectable_label(e.textura.um() == Some(&None), tr("sem textura")).clicked() {
                m = Some(Mudanca::Textura(None));
            }
            for (i, n) in e.texturas.iter().enumerate() {
                if ui.selectable_label(e.textura.um() == Some(&Some(i as u8)), n).clicked() {
                    m = Some(Mudanca::Textura(Some(i as u8)));
                }
            }
            if e.texturas.is_empty() {
                ui.label(RichText::new(tr("Carregue imagens nas propriedades da peça.")).weak().small());
            }
        });
    });
    m
}

impl App {
    /// Alvos da seleção atual.
    pub(crate) fn alvos_graficos(&self) -> Vec<Alvo> {
        let multi = self.multi.borrow().clone();
        alvos(&self.open.doc, self.sel, &multi, &self.picks)
    }

    /// Estado da barra para a seleção atual.
    pub(crate) fn estado_grafico(&self) -> Estado {
        ler(&self.open.doc, &self.alvos_graficos())
    }

    /// Aplica a escolha da barra. Só exibição: nada recalcula.
    pub(crate) fn aplicar_grafico(&mut self, m: Mudanca) {
        let alvos = self.alvos_graficos();
        if aplicar(&mut self.open.doc, &alvos, m) > 0 {
            self.scene_dirty = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fenkra_core::{BoolOp, CurveRef, OriginPlane, PlaneRef, PointRef, PrimKind, Primitive, RefKind};

    /// Volume e caixa envolvente do modelo recalculado (tudo com status ok).
    fn medida(d: &Document) -> (f64, [[f32; 3]; 2]) {
        let r = fenkra_solidos::recompute(d);
        assert!(r.status.values().all(|s| matches!(s, fenkra_solidos::Status::Ok)), "{:?}", r.status);
        let mut c = [[f32::MAX; 3], [f32::MIN; 3]];
        for b in &r.bodies {
            for p in b.mesh.positions.chunks(3) {
                for i in 0..3 {
                    c[0][i] = c[0][i].min(p[i]);
                    c[1][i] = c[1][i].max(p[i]);
                }
            }
        }
        (r.bodies.iter().map(|b| b.mass.volume).sum(), c)
    }

    /// Caixa 40 x 20 x 10 e uma linha de referência.
    fn caixa_e_linha() -> (Document, Id, Id) {
        let mut d = Document::new("grafico");
        let s = |x: &str| x.to_string();
        let f = d.add_feature(FeatureKind::Primitive(Primitive { kind: PrimKind::Caixa, plane: PlaneRef::Origin(OriginPlane::XY), a: s("40"), b: s("20"), c: s("10"), d: s("0"), op: BoolOp::Add }));
        let l = d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed([0.; 3]), b: PointRef::Fixed([50., 0., 0.]), start: s("0"), end: s("0") });
        (d, f, l)
    }

    /// B506 (aceite): a barra muda a cor e a linha de uma face e do elemento selecionados, e o
    /// documento guarda; a geometria não muda (volume e caixa iguais, nada a recalcular).
    #[test]
    fn barra_muda_cor_e_linha_e_o_documento_guarda() {
        let (mut d, f, l) = caixa_e_linha();
        let antes = fenkra_kernel::AVAILABLE.then(|| medida(&d));
        let face = Pick::Face { body: 0, face: 2, owner: f, point: [20., 10., 10.], normal: [0., 0., 1.] };
        let alvos = alvos(&d, Sel::None, &[], &[face, Pick::Geo(l)]);
        assert_eq!(alvos, [Alvo::Recurso(f), Alvo::Ref(l, RefClass::Curve)]);
        // a face aceita cor, transparência e material; a linha aceita cor, tipo e espessura
        let e = ler(&d, &alvos);
        assert!(e.cor.aplica() && e.transparencia.aplica() && e.tipo_linha.aplica() && e.espessura.aplica());
        assert!(!e.simbolo.aplica(), "símbolo só vale para pontos");
        assert_eq!(e.cor, Valor::Varios, "cores diferentes mostram vários");
        assert_eq!(aplicar(&mut d, &alvos, Mudanca::Cor([200, 30, 40])), 2);
        assert_eq!(aplicar(&mut d, &alvos, Mudanca::TipoLinha(1)), 1, "só a linha tem tipo de linha");
        assert_eq!(aplicar(&mut d, &alvos, Mudanca::Espessura(4.)), 1);
        assert_eq!(aplicar(&mut d, &alvos, Mudanca::Transparencia(40)), 1, "só a face tem transparência");
        assert_eq!(d.feature(f).unwrap().appearance.map(|a| a.color), Some([200, 30, 40]));
        assert_eq!(d.reference(l).unwrap().appearance.map(|a| a.color), Some([200, 30, 40]));
        assert_eq!(d.line_style(l).map(|s| (s.dash, s.width)), Some((1, 4.)));
        assert_eq!(d.transparencia(f), Some(40));
        let e = ler(&d, &alvos);
        assert_eq!(e.cor, Valor::Um([200, 30, 40]));
        // material pronto aplica cor e acabamento; vira o nome dele na barra
        aplicar(&mut d, &alvos, Mudanca::Material(1));
        assert_eq!(ler(&d, &[Alvo::Recurso(f)]).material, Valor::Um("Alumínio".into()));
        // gravar e ler de novo mantém tudo
        let de_novo: Document = serde_json::from_str(&serde_json::to_string(&d).unwrap()).unwrap();
        assert_eq!(de_novo.transparencias, d.transparencias);
        assert_eq!(de_novo.line_styles, d.line_styles);
        // só exibição: o recálculo dá o mesmo volume e a mesma caixa
        if let Some((va, ca)) = antes {
            let (vd, cd) = medida(&d);
            assert!((va - 8000.).abs() < 1. && (vd - va).abs() < 1e-6, "{va} {vd}");
            assert_eq!(ca, cd);
            assert!((ca[1][0] - ca[0][0] - 40.).abs() < 1e-3 && (ca[1][2] - ca[0][2] - 10.).abs() < 1e-3, "{ca:?}");
        }
    }

    /// B506: sem seleção, tudo desabilitado; ponto aceita só o símbolo e a cor.
    #[test]
    fn campos_inaplicaveis_ficam_desabilitados() {
        let (mut d, _, _) = caixa_e_linha();
        let e = ler(&d, &[]);
        assert!(!e.cor.aplica() && !e.transparencia.aplica() && !e.simbolo.aplica() && !e.material.aplica());
        let p = d.add_geometry(RefKind::PointCoords { x: "1".into(), y: "2".into(), z: "3".into(), frame: None });
        let e = ler(&d, &[Alvo::Ref(p, RefClass::Point)]);
        assert!(e.cor.aplica() && e.simbolo.aplica());
        assert!(!e.tipo_linha.aplica() && !e.transparencia.aplica() && !e.material.aplica());
        aplicar(&mut d, &[Alvo::Ref(p, RefClass::Point)], Mudanca::Simbolo(3));
        assert_eq!(d.line_style(p).map(|s| s.point), Some(3));
        let _ = CurveRef::Ref(p);
    }

    /// B506: a barra é do sistema do B502: nasce no topo, vai para outra borda, fecha e se restaura.
    #[test]
    fn barra_nasce_no_topo_e_e_do_sistema_de_barras() {
        use crate::barras_layout::Barra;
        use crate::settings::{Bench, Dock, Group, Settings};
        let mut s = Settings::default();
        let b = Barra::Grupo(Group::Propriedades);
        for bancada in [Bench::Solidos, Bench::Superficies, Bench::Chapa] {
            s.bench = bancada;
            assert!(crate::toolbar::groups_at(&s, Dock::Topo, false).contains(&Group::Propriedades), "{bancada:?}");
            assert!(!crate::toolbar::groups_at(&s, Dock::Topo, true).contains(&Group::Propriedades), "no sketch não");
        }
        s.bench = Bench::Montagem;
        assert!(!s.barras_da_bancada(false).contains(&b));
        s.bench = Bench::Solidos;
        s.mover_barra(b, Dock::Baixo);
        assert!(crate::toolbar::groups_at(&s, Dock::Baixo, false).contains(&Group::Propriedades));
        s.mover_barra(b, Dock::Oculta);
        assert_eq!(s.doca_da_barra(b), Dock::Oculta);
        s.restaurar_barras();
        assert_eq!(s.doca_da_barra(b), Dock::Topo);
    }
}
