// SPDX-License-Identifier: LGPL-2.1-or-later
//! Painel "Features encontradas" (B143): o que o reconhecimento infere num corpo importado.
//!
//! A análise (fase 1) não altera o corpo; escolher uma linha realça a feature no 3D e leva a câmera
//! até ela. Converter (fase 2) recria a feature como comando do Fenkra e só vale se volume, área e
//! centroide do corpo recriado baterem com o original em 1e-6 relativo. O texto diz sempre
//! "estrutura inferida", nunca "histórico restaurado".

use super::*;
use crate::render::{LineVertex, Scene};
use fenkra_solidos::reconhece::{self, Encontrada, Reconhecimento, Tipo};
use std::collections::HashSet;

const LARGURA: f32 = 380.;

pub(super) struct Estado {
    rec: Reconhecimento,
    escolhida: Option<usize>,
    convertidas: HashSet<usize>,
    /// quantos comandos o documento tinha ao analisar: se mudou, a análise está velha
    comandos: usize,
    relatorio: String,
}

#[derive(Clone, Debug, PartialEq)]
enum Acao {
    Nada,
    Fechar,
    Escolher(usize),
    Converter(Vec<usize>),
    Analisar,
}

fn icone_do_tipo(t: Tipo) -> &'static str {
    match t {
        Tipo::FuroPassante | Tipo::FuroCego => "furo",
        Tipo::Filete => "filete",
        Tipo::Chanfro => "chanfro",
    }
}

fn nome_do_tipo(t: Tipo) -> &'static str {
    match t {
        Tipo::FuroPassante => "Furo passante",
        Tipo::FuroCego => "Furo cego",
        Tipo::Filete => "Filete",
        Tipo::Chanfro => "Chanfro",
    }
}

fn linha_da_feature(e: &Encontrada) -> String {
    match e.tipo {
        Tipo::FuroPassante => format!("{} Ø{:.3}", tr(nome_do_tipo(e.tipo)), e.medida),
        Tipo::FuroCego => format!("{} Ø{:.3} × {:.3}", tr(nome_do_tipo(e.tipo)), e.medida, e.medida2),
        Tipo::Filete => format!("{} R{:.3}", tr(nome_do_tipo(e.tipo)), e.medida),
        Tipo::Chanfro => format!("{} {:.3} × {:.3}", tr(nome_do_tipo(e.tipo)), e.medida, e.medida2),
    }
}

/// Detalhes extras de um furo, em uma linha só.
fn detalhe(e: &Encontrada) -> Option<String> {
    let mut p = vec![];
    if let Some((d, a)) = e.escareado {
        p.push(format!("{} Ø{d:.3} {a:.0}°", tr("escareado")));
    }
    if let Some((d, h)) = e.rebaixo {
        p.push(format!("{} Ø{d:.3} × {h:.3}", tr("rebaixo")));
    }
    if let Some(a) = e.ponta {
        p.push(format!("{} {a:.0}°", tr("ponta")));
    }
    if matches!(e.tipo, Tipo::Filete) {
        p.push(tr(if e.convexo { "convexo" } else { "côncavo" }).to_string());
    }
    (!p.is_empty()).then(|| p.join(", "))
}

fn perpendiculares(d: [f32; 3]) -> ([f32; 3], [f32; 3]) {
    let n = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt().max(1e-9);
    let d = [d[0] / n, d[1] / n, d[2] / n];
    let aux = if d[2].abs() < 0.9 { [0., 0., 1.] } else { [1., 0., 0.] };
    let u = [d[1] * aux[2] - d[2] * aux[1], d[2] * aux[0] - d[0] * aux[2], d[0] * aux[1] - d[1] * aux[0]];
    let nu = (u[0] * u[0] + u[1] * u[1] + u[2] * u[2]).sqrt().max(1e-9);
    let u = [u[0] / nu, u[1] / nu, u[2] / nu];
    let v = [d[1] * u[2] - d[2] * u[1], d[2] * u[0] - d[0] * u[2], d[0] * u[1] - d[1] * u[0]];
    (u, v)
}

fn f3(p: [f64; 3]) -> [f32; 3] {
    p.map(|x| x as f32)
}

fn segmento(o: &mut Vec<LineVertex>, a: [f32; 3], b: [f32; 3], col: [f32; 4]) {
    o.push(LineVertex { pos: a, col });
    o.push(LineVertex { pos: b, col });
}

fn circulo(o: &mut Vec<LineVertex>, c: [f32; 3], dir: [f32; 3], r: f32, col: [f32; 4]) {
    let (u, v) = perpendiculares(dir);
    let n = 48;
    let ponto = |i: usize| {
        let t = i as f32 / n as f32 * std::f32::consts::TAU;
        [0, 1, 2].map(|k| c[k] + r * (u[k] * t.cos() + v[k] * t.sin()))
    };
    for i in 0..n {
        segmento(o, ponto(i), ponto(i + 1), col);
    }
}

/// Marca da feature por cima das faces: furo com círculos e eixo, e uma cruz em cada face dela.
fn marcar(e: &Encontrada, o: &mut Vec<LineVertex>, col: [f32; 4]) {
    let escala = match e.tipo {
        Tipo::FuroPassante | Tipo::FuroCego => e.medida * 0.3,
        Tipo::Filete => e.medida * 0.5,
        Tipo::Chanfro => (e.medida + e.medida2) * 0.5,
    }
    .max(0.5) as f32;
    for p in &e.pontos {
        let c = f3(*p);
        for k in 0..3 {
            let mut a = c;
            let mut b = c;
            a[k] -= escala;
            b[k] += escala;
            segmento(o, a, b, col);
        }
    }
    if matches!(e.tipo, Tipo::FuroPassante | Tipo::FuroCego) {
        let (boca, dir) = (f3(e.boca), f3(e.direcao));
        let fim = [0, 1, 2].map(|k| boca[k] + dir[k] * e.medida2 as f32);
        let raio = (e.medida / 2.) as f32;
        circulo(o, boca, dir, raio, col);
        circulo(o, fim, dir, raio, col);
        segmento(o, boca, fim, col);
    }
}

/// Features na viewport enquanto o painel está aberto: todas discretas e a escolhida em destaque,
/// por cima das faces porque furo e filete costumam estar atrás de alguma coisa.
pub(super) fn na_cena(est: Option<&Estado>, v: &egui::Visuals, s: &mut Scene) {
    let Some(est) = est else { return };
    let rgba = |c: egui::Color32| [c.r() as f32 / 255., c.g() as f32 / 255., c.b() as f32 / 255., 1.];
    let (calmo, forte) = (rgba(crate::theme::st_ok()), rgba(crate::theme::accent(v)));
    for (i, e) in est.rec.encontradas.iter().enumerate() {
        if est.convertidas.contains(&i) {
            continue;
        }
        let escolhida = est.escolhida == Some(i);
        marcar(e, &mut s.overlay, if escolhida { forte } else { calmo });
    }
}

fn ui(ui: &mut egui::Ui, est: &Estado, velha: bool) -> Acao {
    let mut acao = Acao::Nada;
    ui.label(egui::RichText::new(tr("Estrutura inferida da geometria; não é o histórico original. Analisar não altera o corpo.")).weak().small());
    if let Some(p) = &est.rec.pulada {
        ui.colored_label(crate::theme::st_warn(), format!("{} {p}", tr("Análise não feita:")));
    }
    if velha {
        ui.colored_label(crate::theme::st_warn(), tr("O modelo mudou depois da análise. Analise de novo antes de converter."));
    }
    ui.horizontal(|ui| {
        let r = ui.button(tr("Analisar de novo"));
        if r.on_hover_text(tr("Lê o corpo atual outra vez e refaz a lista. Não altera o corpo.")).clicked() {
            acao = Acao::Analisar;
        }
        let pendentes: Vec<usize> = (0..est.rec.encontradas.len()).filter(|i| est.rec.encontradas[*i].convertivel && !est.convertidas.contains(i)).collect();
        let r = ui.add_enabled(!pendentes.is_empty() && !velha, egui::Button::new(tr("Converter todas")));
        if r.on_hover_text(tr("Recria cada feature como comando do Fenkra. Só vale se volume, área e centroide do corpo recriado baterem com o original em 1e-6; senão o corpo fica como estava.")).clicked() {
            acao = Acao::Converter(pendentes);
        }
    });
    ui.separator();
    if est.rec.encontradas.is_empty() && est.rec.pulada.is_none() {
        ui.label(tr("Nenhum furo, filete ou chanfro reconhecido neste corpo."));
    }
    egui::ScrollArea::vertical().max_height(260.).show(ui, |ui| {
        for (i, e) in est.rec.encontradas.iter().enumerate() {
            let feita = est.convertidas.contains(&i);
            ui.horizontal(|ui| {
                ui.add(egui::Image::new(crate::icons::get(icone_do_tipo(e.tipo))).fit_to_exact_size(egui::vec2(16., 16.)));
                let mut texto = linha_da_feature(e);
                if feita {
                    texto += &format!(" — {}", tr("convertida"));
                }
                let r = ui.selectable_label(est.escolhida == Some(i), texto);
                let dica = match detalhe(e) {
                    Some(d) => format!("{}\n{d}", tr("Clique para ver a feature no 3D.")),
                    None => tr("Clique para ver a feature no 3D.").to_string(),
                };
                if r.on_hover_text(dica).clicked() {
                    acao = Acao::Escolher(i);
                }
                if e.convertivel && !feita {
                    let r = ui.add_enabled(!velha, egui::Button::new(tr("Converter")).small());
                    if r.on_hover_text(tr("Recria só esta feature como comando, com a mesma verificação de volume, área e centroide.")).clicked() {
                        acao = Acao::Converter(vec![i]);
                    }
                }
            });
        }
    });
    if est.rec.toros + est.rec.livres > 0 {
        ui.label(egui::RichText::new(format!("{} {} / {} {}", est.rec.toros, tr("faces tóricas"), est.rec.livres, tr("superfícies livres: ficam como corpo."))).weak().small());
    }
    for d in &est.rec.duvidosas {
        ui.label(egui::RichText::new(d).weak().small());
    }
    if !est.relatorio.is_empty() {
        ui.separator();
        ui.label(egui::RichText::new(&est.relatorio).small());
    }
    ui.separator();
    if ui.button(tr("Fechar")).clicked() {
        acao = Acao::Fechar;
    }
    acao
}

impl App {
    fn analisar_features(&self) -> Option<Estado> {
        let rec = reconhece::analisar_documento(&self.open.doc)?;
        Some(Estado { rec, escolhida: None, convertidas: HashSet::new(), comandos: self.open.doc.active_features().count(), relatorio: String::new() })
    }

    /// Abre o painel com a análise do corpo atual.
    pub(super) fn abrir_features(&mut self) {
        self.features = self.analisar_features();
        if self.features.is_none() {
            self.toast(tr("Este documento ainda não tem corpo para analisar."), true);
        }
        self.scene_dirty = true;
    }

    pub(super) fn painel_features(&mut self, ctx: &egui::Context, area: Rect) {
        use crate::paineis::{self, Encaixe, Painel};
        let Some(est) = self.features.take() else { return };
        let velha = est.comandos != self.open.doc.active_features().count() && est.convertidas.is_empty();
        let padrao = paineis::ao_lado_da_arvore(area);
        let cfg = Painel { nome: "features".into(), titulo: tr("Features encontradas").to_string(), padrao, largura: LARGURA, redimensiona: true };
        let posicao = self.settings.painel("features", Encaixe::Livre);
        let saida = paineis::mostrar(ctx, area, &cfg, posicao, |u| ui(u, &est, velha));
        if let Some(p) = saida.movido {
            self.settings.guardar_painel("features", p);
        }
        let mut est = est;
        match saida.conteudo.unwrap_or(Acao::Nada) {
            Acao::Nada => {}
            Acao::Fechar => {
                self.scene_dirty = true;
                return;
            }
            Acao::Escolher(i) => {
                est.escolhida = Some(i);
                self.scene_dirty = true;
                if let (Some(e), Some(v)) = (est.rec.encontradas.get(i), self.viewport.as_mut()) {
                    v.camera.target = glam::Vec3::new(e.centro[0] as f32, e.centro[1] as f32, e.centro[2] as f32);
                }
            }
            Acao::Analisar => {
                self.features = self.analisar_features();
                self.scene_dirty = true;
                return;
            }
            Acao::Converter(quais) => self.converter_features(&mut est, &quais),
        }
        self.features = Some(est);
    }

    fn converter_features(&mut self, est: &mut Estado, quais: &[usize]) {
        let antes = self.open.doc.clone();
        match reconhece::converter(&antes, &est.rec, Some(quais)) {
            Ok(c) => {
                est.relatorio = c.relatorio.clone();
                if !c.convertidas.is_empty() {
                    self.open.doc = c.doc;
                    self.push_undo_pub(antes);
                    est.convertidas.extend(c.convertidas.iter().copied());
                    est.comandos = self.open.doc.active_features().count();
                }
                let msg = format!("{} {} / {}", tr("Features convertidas:"), c.convertidas.len(), quais.len());
                self.toast(msg, c.convertidas.is_empty());
            }
            Err(e) => {
                est.relatorio = e.clone();
                self.toast(e, true);
            }
        }
        self.scene_dirty = true;
    }

    /// Desenvolvimento: `FENKRA_DEMO_FEATURES=1` abre uma placa importada de STEP com o painel;
    /// um número escolhe a feature; `converter` converte todas.
    pub(super) fn demo_features(&mut self, modo: &str) {
        use fenkra_core::sketch::Curve;
        let s = |x: &str| x.to_string();
        let mut d = Document::new("Placa importada");
        let xy = PlaneRef::Origin(OriginPlane::XY);
        let mut data = SketchData::default();
        let mut n = 9_800_000u64;
        let mut al = || {
            n += 1;
            Id(n)
        };
        data.add_rect(&mut &mut al, [-40., -25.], [40., 25.], false);
        let _ = Curve::Circle { c: Id(0), r: 0. };
        let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: xy, data }));
        d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind(s("20")), reverse: false, taper: s("0"), thin: None, thin_side: 0, dir: None, ends: Default::default() }));
        let furo = |p: [f64; 3], diam: &str, kind: HoleKind, depth: Extent| {
            FeatureKind::Hole(Hole {
                sketch: Id(0),
                kind,
                diameter: s(diam),
                depth,
                thread: None,
                thread_modeled: false,
                standard: None,
                points: vec![PointRef::Fixed(p)],
                dir: Some(AxisRef::Fixed { origin: p, dir: [0., 0., -1.] }),
                reverse: false,
                tip: None,
                taper: s("0"),
                left_hand: false,
                extra: Default::default(),
            })
        };
        d.add_feature(furo([-25., 10., 20.], "8", HoleKind::Simples, Extent::ThroughAll));
        d.add_feature(furo([-10., 10., 20.], "6", HoleKind::Simples, Extent::Blind(s("8"))));
        d.add_feature(furo([10., 10., 20.], "5", HoleKind::Rebaixado { diameter: s("10"), depth: s("4") }, Extent::ThroughAll));
        d.add_feature(furo([25., -10., 20.], "5", HoleKind::Escareado { diameter: s("10"), angle: s("90") }, Extent::ThroughAll));
        let aresta = |v: &str, p: [f64; 3]| EdgeOp { value: s(v), edges: EdgeSel::Edges(vec![p]), mode: EdgeMode::Constant, prop: Default::default(), keys: vec![], extra: Default::default() };
        d.add_feature(FeatureKind::Fillet(aresta("3", [40., -25., 10.])));
        d.add_feature(FeatureKind::Fillet(aresta("4", [40., 25., 10.])));
        d.add_feature(FeatureKind::Chamfer(aresta("2", [-40., 0., 0.])));
        // o corpo chega por STEP: sem histórico, como qualquer arquivo de fora
        let pasta = std::env::temp_dir().join("fenkra_demo_features");
        let _ = std::fs::create_dir_all(&pasta);
        let alvo = pasta.join("placa.step");
        let importado = fenkra_solidos::final_shape(&d)
            .and_then(|f| f.export_step(alvo.to_str()?).ok())
            .and_then(|_| fenkra_solidos::importacao::importar_step(alvo.to_str()?).ok());
        let Some(imp) = importado else { return };
        self.load_doc(imp.doc, None, None);
        self.abrir_features();
        if modo == "converter" {
            if let Some(mut est) = self.features.take() {
                let todas: Vec<usize> = (0..est.rec.encontradas.len()).collect();
                self.converter_features(&mut est, &todas);
                self.features = Some(est);
            }
        } else if let Ok(i) = modo.parse::<usize>() {
            if let Some(est) = self.features.as_mut() {
                est.escolhida = Some(i);
            }
        }
        self.started = true;
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn toda_frase_do_painel_tem_traducao() {
        for t in [
            "Estrutura inferida da geometria; não é o histórico original. Analisar não altera o corpo.",
            "Análise não feita:",
            "O modelo mudou depois da análise. Analise de novo antes de converter.",
            "Analisar de novo",
            "Lê o corpo atual outra vez e refaz a lista. Não altera o corpo.",
            "Converter todas",
            "Recria cada feature como comando do Fenkra. Só vale se volume, área e centroide do corpo recriado baterem com o original em 1e-6; senão o corpo fica como estava.",
            "Nenhum furo, filete ou chanfro reconhecido neste corpo.",
            "Clique para ver a feature no 3D.",
            "Converter",
            "Recria só esta feature como comando, com a mesma verificação de volume, área e centroide.",
            "convertida",
            "faces tóricas",
            "superfícies livres: ficam como corpo.",
            "Fechar",
            "Features encontradas",
            "Features convertidas:",
            "Este documento ainda não tem corpo para analisar.",
            "Furo passante",
            "Furo cego",
            "Filete",
            "Chanfro",
            "escareado",
            "rebaixo",
            "ponta",
            "convexo",
            "côncavo",
            "Features encontradas…",
            "Procura furos, filetes e chanfros no corpo importado e mostra no 3D. Estrutura inferida, não o histórico original.",
        ] {
            assert!(crate::i18n::tem_traducao(t), "sem tradução: {t}");
        }
    }

    #[test]
    fn marca_de_furo_tem_circulos_eixo_e_cruz_por_face() {
        let e = Encontrada {
            tipo: Tipo::FuroCego,
            medida: 6.,
            medida2: 8.,
            convexo: false,
            boca: [0., 0., 20.],
            direcao: [0., 0., -1.],
            escareado: None,
            rebaixo: None,
            ponta: None,
            faces: vec![1, 2],
            pontos: vec![[3., 0., 16.], [0., 0., 12.]],
            centro: [0., 0., 16.],
            convertivel: true,
            aviso: None,
        };
        let mut o = vec![];
        marcar(&e, &mut o, [1.; 4]);
        // 2 faces x 3 eixos + 2 círculos de 48 + eixo, dois vértices por segmento
        assert_eq!(o.len(), 2 * (2 * 3 + 2 * 48 + 1));
        assert!(detalhe(&e).is_none());
    }
}
