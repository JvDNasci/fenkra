// SPDX-License-Identifier: LGPL-2.1-or-later
//! B520: pincel de aparência da barra de propriedades gráficas (B506).
//!
//! Como funciona: o botão liga o pincel. O primeiro clique no 3D escolhe a origem e guarda uma
//! [`Amostra`] (valores, não referências); cada clique seguinte num destino aplica a amostra. Clique
//! simples no botão desliga o pincel depois de um destino; duplo clique o mantém ligado para vários
//! destinos; Esc desliga. A amostra mora no `App`, não no documento: por isso vale entre abas.
//!
//! O que se copia depende do tipo da origem (face: cor, acabamento, textura e transparência; linha:
//! cor, espessura e tipo; ponto: cor e símbolo), e no destino só entra o que ele aceita, pelas mesmas
//! funções de aplicar da barra (`propriedades_graficas::aplicar`). A textura é levada pelo nome,
//! porque o índice dela é de cada documento. Só exibição: nada recalcula, e como o quadro inteiro é
//! uma mudança só do documento, o desfazer volta a aplicação num passo.

use crate::propriedades_graficas::{self as pg, Alvo, Mudanca};
use fenkra_core::{Document, Finish};

/// Estado do botão.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Modo {
    #[default]
    Desligado,
    /// ligado até o primeiro destino
    UmaVez,
    /// ligado até o Esc ou um novo clique no botão
    Continuo,
}

/// Aparência copiada da origem. `None` = a origem não tem esse campo.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Amostra {
    pub cor: Option<[u8; 3]>,
    pub acabamento: Option<Finish>,
    /// `Some(None)` = a origem não tem textura; o nome vale entre documentos
    pub textura: Option<Option<String>>,
    pub transparencia: Option<u8>,
    pub espessura: Option<f32>,
    pub tipo_linha: Option<u8>,
    pub simbolo: Option<u8>,
}

/// O que um clique do pincel fez.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Resultado {
    /// o clique não caiu em nada que tenha aparência
    SemAlvo,
    /// primeira etapa: a origem foi copiada
    Copiou,
    /// o destino recebeu a aparência
    Aplicou,
    /// o destino não aceita nenhum campo da amostra
    NadaAplicavel,
}

#[derive(Default)]
pub struct Pincel {
    pub modo: Modo,
    pub amostra: Option<Amostra>,
    /// mensagem para o aviso da tela (texto, erro)
    pub aviso: Option<(String, bool)>,
}

impl Modo {
    pub fn ativo(self) -> bool {
        self != Modo::Desligado
    }
}

impl Amostra {
    /// Lê a aparência do alvo.
    pub fn de(d: &Document, a: Alvo) -> Amostra {
        let mut m = Amostra::default();
        let id = pg::id_de(a);
        if let Some(ap) = pg::aparencia(d, a) {
            m.cor = Some(ap.color);
            if pg::tem_face(a) {
                m.acabamento = Some(ap.finish);
                m.textura = Some(ap.texture.and_then(|i| d.textures.get(i as usize)).map(|t| t.name.clone()));
            }
        }
        if pg::tem_face(a) {
            m.transparencia = Some(d.transparencia(id).unwrap_or(0));
        }
        let st = d.line_style(id).unwrap_or(pg::ESTILO_PADRAO);
        if pg::tem_linha(a) {
            m.espessura = Some(st.width);
            m.tipo_linha = Some(st.dash);
        }
        if pg::tem_ponto(a) {
            m.simbolo = Some(st.point);
        }
        m
    }

    /// As escolhas da barra que equivalem à amostra, para o documento de destino.
    fn mudancas(&self, d: &Document) -> Vec<Mudanca> {
        let mut v = vec![];
        v.extend(self.cor.map(Mudanca::Cor));
        v.extend(self.acabamento.map(Mudanca::Acabamento));
        match &self.textura {
            Some(None) => v.push(Mudanca::Textura(None)),
            // textura que o destino não tem: fica como está
            Some(Some(nome)) => v.extend(d.textures.iter().position(|t| &t.name == nome).map(|i| Mudanca::Textura(Some(i as u8)))),
            None => {}
        }
        v.extend(self.transparencia.map(Mudanca::Transparencia));
        v.extend(self.espessura.map(Mudanca::Espessura));
        v.extend(self.tipo_linha.map(Mudanca::TipoLinha));
        v.extend(self.simbolo.map(Mudanca::Simbolo));
        v
    }

    /// Aplica ao alvo só o que ele aceita. Devolve se algum campo valeu.
    pub fn aplicar(&self, d: &mut Document, a: Alvo) -> bool {
        let mut ok = false;
        for m in self.mudancas(d) {
            ok |= pg::aplicar(d, &[a], m) > 0;
        }
        ok
    }
}

impl Pincel {
    pub fn ativo(&self) -> bool {
        self.modo != Modo::Desligado
    }

    /// Clique simples no botão: liga, ou desliga se já estava ligado.
    pub fn alternar(&mut self) {
        if self.ativo() {
            self.desligar();
        } else {
            self.modo = Modo::UmaVez;
        }
    }

    /// Duplo clique no botão: fica ligado para vários destinos.
    pub fn continuo(&mut self) {
        self.modo = Modo::Continuo;
    }

    pub fn desligar(&mut self) {
        self.modo = Modo::Desligado;
        self.amostra = None;
    }

    /// Um clique do 3D sobre `alvo` no documento `d` (o aberto agora, que pode não ser o da origem).
    pub fn clique(&mut self, d: &mut Document, alvo: Option<Alvo>) -> Resultado {
        let Some(alvo) = alvo else { return Resultado::SemAlvo };
        let Some(amostra) = self.amostra.as_ref() else {
            self.amostra = Some(Amostra::de(d, alvo));
            return Resultado::Copiou;
        };
        if !amostra.aplicar(d, alvo) {
            return Resultado::NadaAplicavel;
        }
        if self.modo == Modo::UmaVez {
            self.desligar();
        }
        Resultado::Aplicou
    }

    /// Texto do que fazer agora, para o cursor.
    pub fn dica(&self) -> &'static str {
        match (self.modo, self.amostra.is_some()) {
            (Modo::Desligado, _) => "",
            (_, false) => "Pincel: clique no elemento de origem",
            (Modo::Continuo, true) => "Pincel ligado: clique nos destinos (Esc desliga)",
            (Modo::UmaVez, true) => "Pincel: clique no destino (Esc desliga)",
        }
    }

    /// Clique do 3D que deixa o aviso correspondente para a tela.
    pub fn clique_com_aviso(&mut self, d: &mut Document, alvo: Option<Alvo>) -> Resultado {
        let r = self.clique(d, alvo);
        let (txt, erro) = match r {
            Resultado::SemAlvo => ("Esse elemento não tem aparência para copiar ou receber.", true),
            Resultado::Copiou => ("Aparência copiada. Clique no destino.", false),
            Resultado::Aplicou => ("Aparência aplicada.", false),
            Resultado::NadaAplicavel => ("O destino não aceita nenhum dos campos copiados.", true),
        };
        self.aviso = Some((crate::i18n::tr(txt).to_string(), erro));
        r
    }
}

impl crate::app::App {
    /// O pincel só age com o 3D livre: sem comando aberto e sem sketch (lá o clique é do comando).
    pub(crate) fn pincel_vale(&self) -> bool {
        self.pincel.ativo() && self.session.is_none() && self.sketch.is_none()
    }

    /// Clique (`false`) ou duplo clique (`true`) no botão da barra.
    pub(crate) fn pedir_pincel(&mut self, duplo: bool) {
        if duplo {
            self.pincel.continuo();
        } else {
            self.pincel.alternar();
        }
        let msg = if self.pincel.ativo() { self.pincel.dica() } else { "Pincel desligado" };
        self.toast(crate::i18n::tr(msg).to_string(), false);
    }

    /// Começo do quadro: entrega o aviso da última ação e trata o Esc (que só desliga o pincel).
    pub(crate) fn pincel_teclas(&mut self, ctx: &eframe::egui::Context, digitando: bool) {
        use eframe::egui;
        if let Some((m, erro)) = self.pincel.aviso.take() {
            self.toast(m, erro);
        }
        if self.pincel.ativo() && !digitando && !egui::Popup::is_any_open(ctx) && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            self.pincel.desligar();
            self.toast(crate::i18n::tr("Pincel desligado").to_string(), false);
        }
    }
}

/// Cursor de mira e a dica ao lado dele, nas cores do tema, enquanto o pincel espera um clique.
pub fn cursor(ctx: &eframe::egui::Context, pos: Option<eframe::egui::Pos2>, p: &Pincel) {
    use eframe::egui;
    ctx.set_cursor_icon(egui::CursorIcon::Crosshair);
    let Some(pos) = pos else { return };
    let v = ctx.global_style().visuals.clone();
    let pintor = ctx.layer_painter(egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("pincel_dica")));
    let g = pintor.layout_no_wrap(crate::i18n::tr(p.dica()).to_string(), egui::FontId::proportional(12.), v.text_color());
    let r = egui::Rect::from_min_size(pos + egui::vec2(16., 18.), g.size()).expand2(egui::vec2(6., 3.));
    pintor.rect(r, 4., v.window_fill, egui::Stroke::new(1.5, crate::theme::accent(&v)), egui::StrokeKind::Inside);
    pintor.galley(r.min + egui::vec2(6., 3.), g, v.text_color());
}

/// Textos novos do pincel (nos dois idiomas).
pub const TEXTOS: &[(&str, &str)] = &[
    ("Pincel", "Brush"),
    ("Pincel de aparência", "Appearance brush"),
    (
        "Copia a aparência de um elemento para outros: clique no pincel, no elemento de origem e depois nos destinos. Duplo clique no pincel o mantém ligado; Esc desliga.",
        "Copies the look of one element to others: click the brush, the source element and then the targets. Double-click the brush to keep it on; Esc turns it off.",
    ),
    (
        "Cor, transparência, tipo e espessura de linha, símbolo de ponto, material e textura. Entre tipos diferentes (face, linha, ponto) só vai o que o destino aceita. Vale também entre os documentos abertos, e o desfazer volta tudo num passo.",
        "Color, transparency, line type and width, point symbol, material and texture. Between different kinds (face, line, point) only what the target accepts is copied. It also works across open documents, and undo reverts it in one step.",
    ),
    ("Pincel: clique no elemento de origem", "Brush: click the source element"),
    ("Pincel: clique no destino (Esc desliga)", "Brush: click the target (Esc turns it off)"),
    ("Pincel ligado: clique nos destinos (Esc desliga)", "Brush on: click the targets (Esc turns it off)"),
    ("Pincel desligado", "Brush off"),
    ("Esse elemento não tem aparência para copiar ou receber.", "This element has no look to copy or receive."),
    ("Aparência copiada. Clique no destino.", "Look copied. Click the target."),
    ("Aparência aplicada.", "Look applied."),
    ("O destino não aceita nenhum dos campos copiados.", "The target accepts none of the copied fields."),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::picking::Pick;
    use fenkra_core::{BoolOp, FeatureKind, Id, OriginPlane, PlaneRef, PointRef, PrimKind, Primitive, RefClass, RefKind, TextureRef};

    /// Duas caixas (A e B) e uma linha de referência.
    fn modelo() -> (Document, Id, Id, Id) {
        let mut d = Document::new("pincel");
        let s = |x: &str| x.to_string();
        let caixa = |d: &mut Document| d.add_feature(FeatureKind::Primitive(Primitive { kind: PrimKind::Caixa, plane: PlaneRef::Origin(OriginPlane::XY), a: s("40"), b: s("20"), c: s("10"), d: s("0"), op: BoolOp::Add }));
        let a = caixa(&mut d);
        let b = caixa(&mut d);
        let l = d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed([0.; 3]), b: PointRef::Fixed([50., 0., 0.]), start: s("0"), end: s("0") });
        (d, a, b, l)
    }

    fn face(d: &Document, f: Id) -> Option<Alvo> {
        let p = Pick::Face { body: 0, face: 1, owner: f, point: [0.; 3], normal: [0., 0., 1.] };
        pg::alvos(d, crate::app::Sel::None, &[], &[p]).first().copied()
    }

    /// Aceite: cor, linha e material de uma face vão para outra face e para uma linha; o documento
    /// guarda; o desfazer (uma entrada do histórico) volta tudo.
    #[test]
    fn copia_cor_linha_e_material_de_uma_face_para_face_e_linha() {
        let (mut d, a, b, l) = modelo();
        let origem = face(&d, a).unwrap();
        pg::aplicar(&mut d, &[origem], Mudanca::Material(1));
        pg::aplicar(&mut d, &[origem], Mudanca::Cor([10, 120, 200]));
        pg::aplicar(&mut d, &[origem], Mudanca::Transparencia(30));
        let acab = d.feature(a).unwrap().appearance.unwrap().finish;
        let linha = Alvo::Ref(l, RefClass::Curve);
        pg::aplicar(&mut d, &[linha], Mudanca::TipoLinha(2));
        pg::aplicar(&mut d, &[linha], Mudanca::Espessura(5.));

        let mut p = Pincel::default();
        p.alternar();
        assert_eq!({ let alvo = face(&d, a); p.clique(&mut d, alvo) }, Resultado::Copiou);
        assert!(p.ativo());
        let antes = d.clone();
        let mut h = crate::historico::Historico::new(64);

        // destino 1: outra face (cor, material e transparência)
        assert_eq!({ let alvo = face(&d, b); p.clique(&mut d, alvo) }, Resultado::Aplicou);
        let fb = d.feature(b).unwrap().appearance.unwrap();
        assert_eq!((fb.color, fb.finish), ([10, 120, 200], acab));
        assert_eq!(d.transparencia(b), Some(30));
        assert!(!p.ativo(), "modo simples desliga depois do destino");
        h.registrar(antes.clone(), &d);
        let (volta, _) = h.desfazer(d.clone()).unwrap();
        assert_eq!(volta, antes, "o desfazer volta tudo num passo");

        // destino 2: uma linha (face -> linha: só a cor se aplica)
        let mut p = Pincel::default();
        p.alternar();
        { let alvo = face(&d, a); p.clique(&mut d, alvo) };
        let st_antes = d.line_style(l);
        assert_eq!(p.clique(&mut d, Some(linha)), Resultado::Aplicou);
        assert_eq!(d.reference(l).unwrap().appearance.map(|x| x.color), Some([10, 120, 200]));
        assert_eq!(d.line_style(l), st_antes, "a face não tem linha: o estilo da linha não muda");
        assert_eq!(d.transparencia(l), None, "linha não ganha transparência");
    }

    /// Linha -> linha copia tipo e espessura; linha -> face, só a cor; ponto e linha não trocam campos.
    #[test]
    fn entre_tipos_diferentes_copia_so_o_que_se_aplica() {
        let (mut d, a, _, l) = modelo();
        let l2 = d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed([0., 5., 0.]), b: PointRef::Fixed([50., 5., 0.]), start: "0".into(), end: "0".into() });
        let pt = d.add_geometry(RefKind::PointCoords { x: "1".into(), y: "2".into(), z: "3".into(), frame: None });
        let (linha, linha2, ponto) = (Alvo::Ref(l, RefClass::Curve), Alvo::Ref(l2, RefClass::Curve), Alvo::Ref(pt, RefClass::Point));
        pg::aplicar(&mut d, &[linha], Mudanca::Cor([1, 2, 3]));
        pg::aplicar(&mut d, &[linha], Mudanca::TipoLinha(3));
        pg::aplicar(&mut d, &[linha], Mudanca::Espessura(6.));
        let am = Amostra::de(&d, linha);
        assert!(am.acabamento.is_none() && am.transparencia.is_none() && am.simbolo.is_none());
        assert!(am.aplicar(&mut d, linha2));
        assert_eq!(d.line_style(l2).map(|s| (s.dash, s.width)), Some((3, 6.)));
        // linha -> face: só a cor; acabamento e transparência ficam
        let fa = face(&d, a).unwrap();
        let acab = pg::aparencia(&d, fa).unwrap().finish;
        assert!(am.aplicar(&mut d, fa));
        assert_eq!(d.feature(a).unwrap().appearance.map(|x| x.color), Some([1, 2, 3]));
        assert_eq!(pg::aparencia(&d, fa).unwrap().finish, acab);
        assert_eq!(d.transparencia(a), None);
        // linha -> ponto: só a cor; o ponto não ganha estilo de linha
        assert!(am.aplicar(&mut d, ponto));
        assert_eq!(d.line_style(pt), None);
        // ponto: copia cor e símbolo, não espessura
        pg::aplicar(&mut d, &[ponto], Mudanca::Simbolo(3));
        let ap = Amostra::de(&d, ponto);
        assert_eq!((ap.simbolo, ap.espessura, ap.tipo_linha), (Some(3), None, None));
        // elemento de sketch não tem cor: amostra só de cor não vale ali
        let so_cor = Amostra { cor: Some([9, 9, 9]), ..Default::default() };
        let sk = Alvo::CurvaSketch(d.alloc());
        assert!(!so_cor.aplicar(&mut d, sk));
    }

    /// Modo contínuo: vários destinos sem religar; Esc (desligar) encerra e esquece a amostra.
    #[test]
    fn modo_continuo_e_esc() {
        let (mut d, a, b, l) = modelo();
        let origem = face(&d, a).unwrap();
        pg::aplicar(&mut d, &[origem], Mudanca::Cor([200, 10, 10]));
        let mut p = Pincel::default();
        p.alternar();
        p.continuo(); // duplo clique
        assert_eq!(p.modo, Modo::Continuo);
        assert_eq!({ let alvo = face(&d, a); p.clique(&mut d, alvo) }, Resultado::Copiou);
        assert_eq!({ let alvo = face(&d, b); p.clique(&mut d, alvo) }, Resultado::Aplicou);
        assert!(p.ativo(), "continua ligado");
        assert_eq!(p.clique(&mut d, Some(Alvo::Ref(l, RefClass::Curve))), Resultado::Aplicou);
        assert!(p.ativo());
        assert_eq!(p.clique(&mut d, None), Resultado::SemAlvo);
        assert!(p.ativo(), "clique no vazio não desliga");
        p.desligar(); // Esc
        assert!(!p.ativo() && p.amostra.is_none());
        // clique simples no botão ligado desliga
        p.alternar();
        p.alternar();
        assert!(!p.ativo());
    }

    /// Entre documentos: a amostra vai por valor e a textura pelo nome no destino.
    #[test]
    fn vale_entre_documentos_abertos() {
        let (mut d1, a, _, _) = modelo();
        let tex = |n: &str| TextureRef { name: n.into(), path: String::new(), scale: 10. };
        d1.textures = vec![tex("madeira"), tex("tijolo")];
        let fa = face(&d1, a).unwrap();
        pg::aplicar(&mut d1, &[fa], Mudanca::Textura(Some(1)));
        pg::aplicar(&mut d1, &[fa], Mudanca::Cor([5, 6, 7]));
        let mut p = Pincel::default();
        p.alternar();
        p.clique(&mut d1, Some(fa));
        // outro documento, com as texturas em outra ordem
        let (mut d2, a2, _, _) = modelo();
        d2.textures = vec![tex("tijolo")];
        let alvo2 = face(&d2, a2);
        assert_eq!(p.clique(&mut d2, alvo2), Resultado::Aplicou);
        let ap = d2.feature(a2).unwrap().appearance.unwrap();
        assert_eq!((ap.color, ap.texture), ([5, 6, 7], Some(0)), "textura achada pelo nome");
        // documento sem a textura: a cor vai e a textura fica como estava
        let mut p = Pincel::default();
        p.alternar();
        p.clique(&mut d1, Some(fa));
        let (mut d3, a3, _, _) = modelo();
        let alvo3 = face(&d3, a3);
        p.clique(&mut d3, alvo3);
        assert_eq!(d3.feature(a3).unwrap().appearance.map(|x| (x.color, x.texture)), Some(([5, 6, 7], None)));
    }

    /// O botão (clique e duplo clique) e o Esc do app, pelo quadro inteiro.
    #[test]
    fn botao_e_esc_no_app() {
        use crate::app::App;
        use eframe::egui;
        let _g = crate::pmi_nota_cmd::tests::SERIE.lock().unwrap_or_else(|e| e.into_inner());
        let ctx = egui::Context::default();
        let mut app = App::new(&eframe::CreationContext::_new_kittest(ctx.clone()));
        app.started = true;
        let mut frame = eframe::Frame::_new_kittest();
        let mut t = 0.;
        let mut quadro = |app: &mut App, ev: Vec<egui::Event>| {
            t += 0.1;
            let raw = egui::RawInput { time: Some(t), screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280., 800.))), events: ev, ..Default::default() };
            let mut s = ctx.run_ui(raw, |ui| eframe::App::ui(app, ui, &mut frame));
            s.textures_delta.clear();
        };
        quadro(&mut app, vec![]);
        app.pedir_pincel(false);
        assert_eq!(app.pincel.modo, Modo::UmaVez);
        app.pedir_pincel(true);
        assert_eq!(app.pincel.modo, Modo::Continuo);
        quadro(&mut app, vec![]);
        assert!(app.pincel.ativo());
        let esc = egui::Event::Key { key: egui::Key::Escape, physical_key: None, pressed: true, repeat: false, modifiers: Default::default() };
        quadro(&mut app, vec![esc]);
        assert!(!app.pincel.ativo(), "Esc desliga");
    }

    #[test]
    fn textos_do_pincel_traduzidos() {
        for (pt, en) in TEXTOS {
            assert!(crate::i18n::tem_traducao(pt), "{pt}");
            assert!(!en.is_empty());
        }
    }
}
