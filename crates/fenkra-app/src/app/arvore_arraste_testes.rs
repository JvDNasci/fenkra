// SPDX-License-Identifier: LGPL-2.1-or-later
//! B504: arrastar na árvore com prévia de onde o item cai.
//!
//! Soltar sobre um item põe o arrastado logo depois dele, no mesmo lugar (corpo, pasta de comandos ou
//! pasta de geometria); soltar sobre um contêiner põe dentro; o que a dependência proíbe é recusado com
//! o motivo e o documento fica como estava. Cada teste monta o documento, solta, recalcula e confere
//! status, volume e caixa envolvente.

use super::*;
use crate::arvore_soltura_ui::{aplicar, testar, Alvo};

fn id_de(d: &Document, nome: &str) -> Id {
    d.features.iter().find(|f| f.name == nome).map(|f| f.id).or_else(|| d.references.iter().find(|r| r.name == nome).map(|r| r.id)).unwrap_or_else(|| panic!("{nome} não existe"))
}

fn indice(d: &Document, id: Id) -> usize {
    d.feature_index(id).expect("comando")
}

fn pasta_geo(d: &Document) -> Id {
    d.folders.iter().find(|f| f.kind == FolderKind::Geometria).map(|f| f.id).expect("pasta de geometria")
}

/// Recalcula e confere que nenhum comando nem elemento ficou com erro.
fn recalcular_ok(d: &Document) -> RecomputeResult {
    let r = fenkra_solidos::recompute(d);
    for (id, st) in &r.status {
        assert!(!matches!(st, Status::Error(_)), "{} com erro: {st:?}", d.name_of(*id));
    }
    r
}

fn volume(r: &RecomputeResult) -> f64 {
    r.bodies.iter().map(|b| b.mass.volume).sum()
}

fn caixa(r: &RecomputeResult) -> ([f32; 3], [f32; 3]) {
    let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
    for b in &r.bodies {
        for q in b.mesh.positions.chunks_exact(3) {
            for i in 0..3 {
                lo[i] = lo[i].min(q[i]);
                hi[i] = hi[i].max(q[i]);
            }
        }
    }
    (lo, hi)
}

fn mesma_caixa(a: ([f32; 3], [f32; 3]), b: ([f32; 3], [f32; 3])) {
    for i in 0..3 {
        assert!((a.0[i] - b.0[i]).abs() < 1e-3 && (a.1[i] - b.1[i]).abs() < 1e-3, "caixa mudou: {a:?} x {b:?}");
    }
}

#[test]
fn sketch_vai_para_a_pasta_de_geometria_e_o_recalculo_continua_ok() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = fenkra_solidos::example_document();
    let r0 = recalcular_ok(&d);
    let (v0, c0) = (volume(&r0), caixa(&r0));
    let geo = pasta_geo(&d);
    let base = id_de(&d, "Sketch base");
    let centros = id_de(&d, "Centros dos furos");
    let topo = id_de(&d, "PlanoTopo");
    // na pasta (contêiner) e sobre um elemento da pasta (logo depois dele)
    aplicar(&mut d, base, Alvo::Pasta(geo)).expect("sketch na pasta de geometria");
    aplicar(&mut d, centros, Alvo::Depois(topo)).expect("sketch depois do plano");
    for s in [base, centros] {
        let f = d.feature(s).unwrap();
        assert!(d.is_free(f), "{} ficou livre", f.name);
        assert_eq!(f.folder, Some(geo));
    }
    let r = recalcular_ok(&d);
    assert!((volume(&r) - v0).abs() < 1e-6 * v0, "volume {} x {v0}", volume(&r));
    mesma_caixa(caixa(&r), c0);
    // e volta para a linha do tempo pelo nó do corpo, sem mudar a peça
    let corpo = d.feature(id_de(&d, "Placa")).unwrap().target;
    aplicar(&mut d, base, Alvo::Corpo(corpo)).expect("sketch de volta ao corpo");
    assert!(!d.is_free(d.feature(base).unwrap()));
    let r = recalcular_ok(&d);
    assert!((volume(&r) - v0).abs() < 1e-6 * v0);
}

#[test]
fn grupo_geometria_sem_pasta_recebe_o_sketch() {
    // documento antigo: elementos sem pasta e nenhuma pasta de geometria. O grupo "Geometria" que a
    // árvore desenha não era alvo de soltura e o sketch caía sem efeito (o bug relatado).
    let mut d = fenkra_solidos::example_document();
    d.folders.retain(|f| f.kind != FolderKind::Geometria);
    for r in &mut d.references {
        r.folder = None;
    }
    let base = id_de(&d, "Sketch base");
    assert!(testar(&d, base, Alvo::GeometriaPadrao).is_ok());
    aplicar(&mut d, base, Alvo::GeometriaPadrao).unwrap();
    let f = d.feature(base).unwrap();
    assert!(d.is_free(f));
    // sobre um elemento sem pasta também: vai para a pasta padrão, já criada
    let centros = id_de(&d, "Centros dos furos");
    let topo = id_de(&d, "PlanoTopo");
    aplicar(&mut d, centros, Alvo::Depois(topo)).unwrap();
    assert_eq!(d.feature(centros).unwrap().folder, Some(pasta_geo(&d)));
    assert_eq!(d.folders.iter().filter(|f| f.kind == FolderKind::Geometria).count(), 1, "uma pasta só");
    if fenkra_kernel::AVAILABLE {
        recalcular_ok(&d);
    }
}

#[test]
fn recurso_entre_pastas_cai_logo_depois_do_alvo() {
    let mut d = fenkra_solidos::example_document();
    let furo = id_de(&d, "Furos M6");
    let placa = id_de(&d, "Placa");
    let filete = d.features.iter().find(|f| matches!(f.kind, FeatureKind::Fillet(_))).map(|f| f.id).unwrap();
    let acab = d.feature(filete).unwrap().folder.expect("filete na pasta Acabamentos");
    let r0 = fenkra_kernel::AVAILABLE.then(|| recalcular_ok(&d));
    // uma pasta de comandos nova com o furo dentro
    let usinagem = d.add_folder("Usinagem");
    aplicar(&mut d, furo, Alvo::Pasta(usinagem)).expect("furo na pasta nova");
    assert_eq!(d.feature(furo).unwrap().folder, Some(usinagem));
    // o filete solto sobre o furo: logo depois dele e dentro da pasta dele
    aplicar(&mut d, filete, Alvo::Depois(furo)).expect("filete depois do furo");
    assert_eq!(d.feature(filete).unwrap().folder, Some(usinagem), "entra na pasta do vizinho");
    assert_eq!(indice(&d, filete), indice(&d, furo) + 1);
    assert!(!d.features.iter().any(|f| f.folder == Some(acab)), "Acabamentos ficou vazia");
    if let Some(r0) = &r0 {
        let r = recalcular_ok(&d);
        assert!((volume(&r) - volume(r0)).abs() < 1e-6 * volume(r0), "mesma ordem relativa, mesma peça");
        mesma_caixa(caixa(&r), caixa(r0));
    }
    // o filete solto sobre a placa sobe para logo depois dela e sai da pasta (a placa não está em pasta)
    aplicar(&mut d, filete, Alvo::Depois(placa)).expect("filete depois da placa");
    assert_eq!(indice(&d, filete), indice(&d, placa) + 1);
    assert_eq!(d.feature(filete).unwrap().folder, None);
    assert_eq!(d.feature(furo).unwrap().folder, Some(usinagem), "o furo continua na pasta dele");
    if fenkra_kernel::AVAILABLE {
        let r = recalcular_ok(&d);
        assert!(volume(&r) > 0.);
    }
}

#[test]
fn dependencia_recusa_com_motivo_e_nao_muda_nada() {
    let mut d = fenkra_solidos::example_document();
    let base = id_de(&d, "Sketch base");
    let furo = id_de(&d, "Furos M6");
    let centros = id_de(&d, "Centros dos furos");
    let antes = d.features.iter().map(|f| (f.id, f.folder, f.target)).collect::<Vec<_>>();
    // o sketch base depois do furo deixaria a placa antes do sketch dela
    let e = testar(&d, base, Alvo::Depois(furo)).unwrap_err();
    assert!(matches!(e, RecusaSoltura::Ordem(_)), "{e:?}");
    assert!(crate::arvore_soltura_ui::motivo(&e).contains("Não dá para soltar aqui"));
    // o furo antes dos centros que ele usa
    let primeiro = d.features[0].id;
    let e = aplicar(&mut d, furo, Alvo::Depois(primeiro)).unwrap_err();
    assert!(matches!(e, RecusaSoltura::Ordem(_)), "{e:?}");
    assert!(indice(&d, centros) < indice(&d, furo));
    assert_eq!(d.features.iter().map(|f| (f.id, f.folder, f.target)).collect::<Vec<_>>(), antes, "recusada, nada muda");
    // soltar algo sobre si mesmo nunca vale
    assert!(testar(&d, furo, Alvo::Depois(furo)).is_err());
}

#[test]
fn superficie_muda_de_pasta_e_fica_depois_do_vizinho() {
    let s = |v: &str| v.to_string();
    let mut d = fenkra_solidos::example_document();
    let la = d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed([0., 0., 0.]), b: PointRef::Fixed([0., 30., 0.]), start: s("0"), end: s("0") });
    let sup = d.add_geometry(RefKind::SurfExtrude { curve: CurveRef::Ref(la), dir: AxisRef::Origin(OriginAxis::X), length: s("10"), length2: s("0"), until: None, until2: None });
    let geo = pasta_geo(&d);
    let sub = d.add_subfolder("Superfícies", geo);
    aplicar(&mut d, sup, Alvo::Pasta(sub)).expect("superfície na subpasta");
    assert_eq!(d.reference(sup).unwrap().folder, Some(sub));
    // a linha solta sobre a superfície: mesma pasta, logo depois dela na lista
    aplicar(&mut d, la, Alvo::Depois(sup)).expect("linha depois da superfície");
    assert_eq!(d.reference(la).unwrap().folder, Some(sub));
    let pos = |id: Id| d.references.iter().position(|r| r.id == id).unwrap();
    assert_eq!(pos(la), pos(sup) + 1);
    // superfície não entra na linha do tempo de um corpo; comando sólido não vai para a geometria
    assert_eq!(testar(&d, sup, Alvo::Depois(id_de(&d, "Placa"))).unwrap_err(), RecusaSoltura::Referencia);
    assert_eq!(testar(&d, id_de(&d, "Placa"), Alvo::Depois(sup)).unwrap_err(), RecusaSoltura::SoGeometriaLivre);
    if fenkra_kernel::AVAILABLE {
        let r = recalcular_ok(&d);
        assert_eq!(r.status.get(&sup), Some(&Status::Ok));
    }
}

// ---------------------------------------------------------------------------------------------
// O caminho da interface: arrastar a linha do sketch com o mouse até a pasta de geometria.

pub(super) struct Cena {
    pub(super) app: App,
    pub(super) ctx: egui::Context,
    frame: eframe::Frame,
    t: f64,
}

impl Cena {
    pub(super) fn nova() -> Cena {
        let ctx = egui::Context::default();
        let mut app = App::new(&eframe::CreationContext::_new_kittest(ctx.clone()));
        app.load_doc_pub(fenkra_solidos::example_document(), None);
        app.started = true;
        app.result = Some(fenkra_solidos::recompute(&app.open.doc));
        let mut c = Cena { app, ctx, frame: eframe::Frame::_new_kittest(), t: 0. };
        c.passo(vec![]);
        c.t = 5.;
        c.esperar_calculo();
        c
    }

    pub(super) fn esperar_calculo(&mut self) {
        for _ in 0..500 {
            self.passo(vec![]);
            if self.app.gen_done >= self.app.gen_sent {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        self.t += 3.;
        self.passo(vec![]);
        self.passo(vec![]);
    }

    pub(super) fn passo(&mut self, eventos: Vec<egui::Event>) {
        self.t += 0.05;
        let raw = egui::RawInput { time: Some(self.t), screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280., 800.))), events: eventos, ..Default::default() };
        let (app, frame) = (&mut self.app, &mut self.frame);
        let mut saida = self.ctx.run_ui(raw, |ui| eframe::App::ui(app, ui, frame));
        saida.textures_delta.clear();
    }

    /// Retângulo da linha da árvore de um item, como desenhada no último quadro.
    fn linha(&self, id: Id) -> egui::Rect {
        let w = crate::teclado::linhas_da_arvore(&self.ctx).into_iter().find(|(i, _)| *i == id).map(|(_, w)| w).unwrap_or_else(|| panic!("{} fora da árvore", self.app.open.doc.name_of(id)));
        self.ctx.read_response(w).expect("resposta da linha").rect
    }

    fn arrastar(&mut self, de: egui::Pos2, ate: egui::Pos2) {
        let botao = |pos, pressed| egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() };
        self.passo(vec![egui::Event::PointerMoved(de)]);
        self.passo(vec![botao(de, true)]);
        for k in 1..=8 {
            let p = de + (ate - de) * (k as f32 / 8.);
            self.passo(vec![egui::Event::PointerMoved(p)]);
        }
        self.passo(vec![egui::Event::PointerMoved(ate)]);
        self.passo(vec![botao(ate, false)]);
        self.passo(vec![]);
        self.passo(vec![]);
    }
}

#[test]
fn arrastar_o_sketch_ate_a_pasta_de_geometria_pela_interface() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut c = Cena::nova();
    let d = &c.app.open.doc;
    let (base, geo) = (id_de(d, "Sketch base"), pasta_geo(d));
    let v0 = volume(c.app.result.as_ref().unwrap());
    let (de, ate) = (c.linha(base).center(), c.linha(geo).center());
    c.arrastar(de, ate);
    let d = &c.app.open.doc;
    assert!(d.is_free(d.feature(base).unwrap()), "o sketch foi para a geometria");
    assert_eq!(d.feature(base).unwrap().folder, Some(geo));
    c.esperar_calculo();
    let r = c.app.result.as_ref().unwrap();
    assert!(!r.status.values().any(|s| matches!(s, Status::Error(_))), "recálculo ok");
    assert!((volume(r) - v0).abs() < 1e-6 * v0);
    // soltar o furo sobre a placa pediria o furo antes dos centros que ele usa: recusa, nada muda
    let (furo, placa) = (id_de(&c.app.open.doc, "Furos M6"), id_de(&c.app.open.doc, "Placa"));
    let ordem: Vec<Id> = c.app.open.doc.features.iter().map(|f| f.id).collect();
    let (de, ate) = (c.linha(furo).center(), c.linha(placa).center());
    c.arrastar(de, ate);
    assert_eq!(c.app.open.doc.features.iter().map(|f| f.id).collect::<Vec<_>>(), ordem, "recusada, fica onde estava");
    // o filete solto sobre a placa vai para logo depois dela
    let filete = c.app.open.doc.features.iter().find(|f| matches!(f.kind, FeatureKind::Fillet(_))).map(|f| f.id).unwrap();
    let (de, ate) = (c.linha(filete).center(), c.linha(placa).center());
    c.arrastar(de, ate);
    let d = &c.app.open.doc;
    assert_eq!(indice(d, filete), indice(d, placa) + 1);
}
