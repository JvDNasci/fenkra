// SPDX-License-Identifier: LGPL-2.1-or-later
//! Desenvolvimento: casos da bancada Estrutural para captura sem mouse (`FENKRA_DEMO_FOTO`).
//!
//! * `FENKRA_DEMO_GOMOS=1`: L de tubo redondo com corte simples em três gomos (B326), com o painel do nó
//!   aberto quando `FENKRA_EDITAR=Membro`.
//! * `FENKRA_DEMO_DWG_TUBO=<v|canto|dobradeira>`: desenho de um tubo dobrado com a vista do tubo reto, a tabela
//!   de dobras e a lista de corte (B331).
//! * `FENKRA_DEMO_SOLDA=<cosmetico|solido|intermitente>`: tubo contra placa com o cordão de filete; com
//!   `FENKRA_DEMO_TOCHA=1`, um tubo vizinho e o teste de acessibilidade da tocha ligado (B174)
//!   (B285); com `FENKRA_EDITAR=Cordão` abre o painel do cordão.
//! * `FENKRA_DEMO_REFORCO=<triangular|chanfrado|poligonal>`: L de tubo quadrado com placa de
//!   extremidade interna recuada e reforço na aresta de encontro (B255), três reforços repetidos ao
//!   longo da aresta; com `FENKRA_EDITAR=Reforço1` ou `Placa1` abre o painel.
//! * `FENKRA_DEMO_QUADRO=1`: quadro de tubo redondo com longarinas dobradas e travessas com canto
//!   Encaixe chegando no meio de um trecho e num nó de dobra (B387, K-09 do kart), com a lista de corte.
//! * `FENKRA_DEMO_ACESSO=<1|gerada>`: comando Escada e guarda-corpo aberto ao lado de uma plataforma (B136), com a
//!   prévia das linhas no 3D; `gerada` já põe a escada de dois lances na peça e fecha o painel.

use super::*;

impl App {
    pub(super) fn demo_estrutural(&mut self) {
        let s = |x: &str| x.to_string();
        if std::env::var("FENKRA_DEMO_GOMOS").is_ok() {
            let mut d = Document::new("Tubo com gomos");
            let mut paths = vec![];
            for (a, b) in [([0., 0., 0.], [400., 0., 0.]), ([400., 0., 0.], [400., 300., 0.])] {
                let l = d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed(a), b: PointRef::Fixed(b), start: s("0"), end: s("0") });
                paths.push(CurveRef::Ref(l));
            }
            let f = d.add_feature(FeatureKind::StructMember(StructMember {
                paths,
                family: s("Tubo redondo"),
                designation: s("60,3x3,6"),
                kind: SectionKind::TuboRedondo,
                dims: vec![s("60.3"), s("3.6")],
                anchor: 0,
                angle: s("0"),
                off_x: s("0"),
                off_y: s("0"),
                ext1: s("0"),
                ext2: s("0"),
                corner: CornerMode::MeiaEsquadria,
                gap: s("0"),
                joints: vec![],
                material: String::new(),
                mirror: false,
            }));
            if let Some(FeatureKind::StructMember(m)) = d.feature_mut(f).map(|x| &mut x.kind) {
                let b = TubeBend { mode: BendMode::CorteSimples, radius: s("120"), segments: s("4"), ..Default::default() };
                m.joints.push(StructJoint { mine: 1, other: f, other_path: 0, mode: CornerMode::Dobra, first: false, gap: None, bend: Some(b) });
            }
            for r in &mut d.references {
                r.visible = false;
            }
            self.load_doc(d, None, None);
            self.settings.bench = crate::settings::Bench::Estrutural;
            self.st_cut_open = std::env::var("FENKRA_EDITAR").is_err();
            self.started = true;
        }
        if let Ok(modo) = std::env::var("FENKRA_DEMO_DWG_TUBO") {
            use fenkra_desenho::*;
            let mut d = Document::new("Tubo dobrado");
            let mut paths = vec![];
            for (a, b) in [([0., 0., 0.], [400., 0., 0.]), ([400., 0., 0.], [400., 300., 0.])] {
                let l = d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed(a), b: PointRef::Fixed(b), start: s("0"), end: s("0") });
                paths.push(CurveRef::Ref(l));
            }
            let f = d.add_feature(FeatureKind::StructMember(StructMember {
                paths,
                family: s("Tubo quadrado"),
                designation: s("30x1.2"),
                kind: SectionKind::TuboQuadrado,
                dims: vec![s("30"), s("1.2")],
                anchor: 0,
                angle: s("0"),
                off_x: s("0"),
                off_y: s("0"),
                ext1: s("0"),
                ext2: s("0"),
                corner: CornerMode::MeiaEsquadria,
                gap: s("0"),
                joints: vec![],
                material: String::new(),
                mirror: false,
            }));
            if let Some(FeatureKind::StructMember(m)) = d.feature_mut(f).map(|x| &mut x.kind) {
                let b = match modo.as_str() {
                    "canto" => TubeBend { mode: BendMode::VComRaio, radius: s("40"), ..Default::default() },
                    "dobradeira" => TubeBend { mode: BendMode::Dobradeira, radius: s("100"), ..Default::default() },
                    _ => TubeBend { mode: BendMode::VComum, radius: s("40"), cuts: s("3"), ..Default::default() },
                };
                m.joints.push(StructJoint { mine: 1, other: f, other_path: 0, mode: CornerMode::Dobra, first: false, gap: None, bend: Some(b) });
            }
            let dir = std::env::temp_dir().join("fenkra_demo_desenho_tubo");
            let _ = std::fs::create_dir_all(&dir);
            let _ = std::fs::write(dir.join("tubo.fkp"), d.to_json());
            let src = Source { path: "tubo.fkp".into(), display_state: String::new(), hidden: vec![] };
            let mut dw = Drawing::new("Tubo dobrado", Format::A3);
            let sh = dw.sheets[0].id;
            dw.add_view(sh, src.clone(), ViewKind::Standard(Dir::Topo), [110., 215.], 0.35);
            dw.add_view(sh, src.clone(), ViewKind::Flat, [250., 130.], 0.3);
            let id = dw.alloc();
            dw.tables.push(Table { id, sheet: sh, kind: TableKind::Bends, source: src.clone(), pos: [25., 120.], level: Default::default(), cfg: Default::default() });
            let id = dw.alloc();
            dw.tables.push(Table { id, sheet: sh, kind: TableKind::CutList, source: src, pos: [25., 60.], level: Default::default(), cfg: Default::default() });
            let arq = dir.join("tubo.fkd");
            let _ = std::fs::write(&arq, dw.to_json());
            let mut dst = crate::dwg_ui::DwgState::new(dw, Some(arq));
            crate::dwg_ui::demo_ajustar(&mut dst);
            self.dwg = Some(dst);
            self.settings.bench = crate::settings::Bench::Desenho;
            self.started = true;
        }
        if let Ok(modo) = std::env::var("FENKRA_DEMO_SOLDA") {
            let mut d = Document::new("Cordão de solda");
            let l = CurveRef::Ref(d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed([0., 0., 0.]), b: PointRef::Fixed([0., 0., 80.]), start: s("0"), end: s("0") }));
            d.add_feature(FeatureKind::StructMember(StructMember {
                paths: vec![l],
                family: s("Tubo quadrado"),
                designation: s("40x3"),
                kind: SectionKind::TuboQuadrado,
                dims: vec![s("40"), s("3")],
                anchor: 0,
                angle: s("0"),
                off_x: s("0"),
                off_y: s("0"),
                ext1: s("0"),
                ext2: s("0"),
                corner: CornerMode::Nenhum,
                gap: s("0"),
                joints: vec![],
                material: String::new(),
                mirror: false,
            }));
            d.add_feature(FeatureKind::StructPlate(StructPlate::nova([0., 0., 80.], "8", "15")));
            // B174: com FENKRA_DEMO_TOCHA, um tubo vizinho fecha o acesso da tocha ao lado +X do cordão
            if std::env::var("FENKRA_DEMO_TOCHA").is_ok() {
                let l = CurveRef::Ref(d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed([75., 0., 0.]), b: PointRef::Fixed([75., 0., 70.]), start: s("0"), end: s("0") }));
                if let Some(FeatureKind::StructMember(base)) = d.features.first().map(|f| f.kind.clone()) {
                    let m = StructMember { paths: vec![l], family: s("Tubo retangular"), designation: s("200x40x3"), kind: SectionKind::TuboRetangular, dims: vec![s("200"), s("40"), s("3")], ..base };
                    let id = d.add_feature(FeatureKind::StructMember(m));
                    if let Some(f) = d.feature_mut(id) {
                        f.name = s("Tubo vizinho");
                    }
                }
            }
            let mut w = StructWeld::novo("5");
            for p in [[20., 0., 40.], [-20., 0., 40.], [0., 20., 40.], [0., -20., 40.]] {
                w.adicionar(p, None);
            }
            w.picking_b = true;
            w.adicionar([22., 0., 80.], None);
            w.picking_b = false;
            if modo != "cosmetico" {
                w.mode = BeadMode::Solido;
            }
            if modo == "intermitente" {
                w.pattern = BeadPattern::ComprimentoEVao;
                w.seg_len = s("30");
                w.spacing = s("20");
            }
            d.add_feature(FeatureKind::StructWeld(w));
            for r in &mut d.references {
                r.visible = false;
            }
            self.load_doc(d, None, None);
            self.settings.bench = crate::settings::Bench::Estrutural;
            self.started = true;
        }
        if let Ok(v) = std::env::var("FENKRA_DEMO_ACESSO") {
            let mut d = Document::new("Escada");
            if v == "gerada" {
                let e = fenkra_solidos::st_acesso::Escada { altura: 4400., ..Default::default() };
                if let Ok((_, p)) = fenkra_solidos::st_acesso::pecas_escada(&e) {
                    fenkra_solidos::st_acesso::inserir(&mut d, &p, [0.; 3], 0.);
                }
                for r in &mut d.references {
                    r.visible = false;
                }
                self.st_acesso_open = false;
            } else {
                // contexto da prévia: o guarda-corpo da plataforma onde a escada padrão (2 m) chega
                let gc = fenkra_solidos::st_acesso::GuardaCorpo::default();
                let borda = [[-500., 0., 2000.], [-500., 1500., 2000.], [500., 1500., 2000.], [500., 0., 2000.]];
                let p = fenkra_solidos::st_acesso::guarda_corpo(&borda, false, &gc, "Plataforma: ");
                fenkra_solidos::st_acesso::inserir(&mut d, &p, [0., 2070., 0.], 0.);
                for r in &mut d.references {
                    r.visible = false;
                }
            }
            self.load_doc(d, None, None);
            self.settings.bench = crate::settings::Bench::Estrutural;
            self.started = true;
        }
        if std::env::var("FENKRA_DEMO_QUADRO").is_ok() {
            let mut d = Document::new("Quadro K-09");
            let z = 40.;
            let tubo = |paths: Vec<CurveRef>, corner: CornerMode| StructMember {
                paths,
                family: s("Tubo redondo"),
                designation: s("Ø30x2,4"),
                kind: SectionKind::TuboRedondo,
                dims: vec![s("30"), s("2.4")],
                anchor: 0,
                angle: s("0"),
                off_x: s("0"),
                off_y: s("0"),
                ext1: s("0"),
                ext2: s("0"),
                corner,
                gap: s("0"),
                joints: vec![],
                material: String::new(),
                mirror: false,
            };
            let linha = |d: &mut Document, a: [f64; 3], b: [f64; 3]| CurveRef::Ref(d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed(a), b: PointRef::Fixed(b), start: s("0"), end: s("0") }));
            for (y, yd) in [(350., 280.), (-350., -280.)] {
                let p = [[-100., y, z + 35.], [120., y, z], [624., y, z], [870., yd, z], [1100., yd, z]];
                let c: Vec<CurveRef> = p.windows(2).map(|w| linha(&mut d, w[0], w[1])).collect();
                let f = d.add_feature(FeatureKind::StructMember(tubo(c, CornerMode::Nenhum)));
                if let Some(FeatureKind::StructMember(m)) = d.feature_mut(f).map(|x| &mut x.kind) {
                    for i in 1..4 {
                        m.joints.push(StructJoint { mine: i, other: f, other_path: i - 1, mode: CornerMode::Dobra, first: false, gap: None, bend: Some(TubeBend { mode: BendMode::Dobradeira, radius: s("90"), ..Default::default() }) });
                    }
                }
            }
            for (a, b) in [
                ([120., -350., z], [120., 350., z]),
                ([436.8, -350., z], [436.8, 350., z]),
                ([1000., -280., z], [1000., 280., z]),
                ([300., 350., z], [300., 190., z + 170.]),
                ([300., -350., z], [300., -190., z + 170.]),
            ] {
                let l = linha(&mut d, a, b);
                d.add_feature(FeatureKind::StructMember(tubo(vec![l], CornerMode::Encaixe)));
            }
            for r in &mut d.references {
                r.visible = false;
            }
            self.load_doc(d, None, None);
            self.settings.bench = crate::settings::Bench::Estrutural;
            self.st_cut_open = true;
            self.started = true;
        }
        if let Ok(modo) = std::env::var("FENKRA_DEMO_REFORCO") {
            let mut d = Document::new("Placa e reforço");
            let tubo = |paths: Vec<CurveRef>| StructMember {
                paths,
                family: s("Tubo quadrado"),
                designation: s("40x3"),
                kind: SectionKind::TuboQuadrado,
                dims: vec![s("40"), s("3")],
                anchor: 0,
                angle: s("0"),
                off_x: s("0"),
                off_y: s("0"),
                ext1: s("0"),
                ext2: s("0"),
                corner: CornerMode::Nenhum,
                gap: s("0"),
                joints: vec![],
                material: String::new(),
                mirror: false,
            };
            for (a, b) in [([0., 0., 0.], [0., 0., 500.]), ([0., 0., 0.], [400., 0., 0.])] {
                let l = CurveRef::Ref(d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed(a), b: PointRef::Fixed(b), start: s("0"), end: s("0") }));
                d.add_feature(FeatureKind::StructMember(tubo(vec![l])));
            }
            let mut pl = StructPlate::nova([0., 0., 500.], "8", "15");
            pl.holes = None;
            d.add_feature(FeatureKind::StructPlate(pl));
            let mut g = StructGusset::novo([0., 0., 0.], "90", "6", if modo == "chanfrado" { "15" } else { "0" });
            g.leg2 = s("120");
            if modo == "poligonal" {
                g.profile = GussetProfile::Poligonal;
                g.poly = [["0", "0"], ["90", "0"], ["90", "20"], ["20", "120"], ["0", "120"]].iter().map(|q| [s(q[0]), s(q[1])]).collect();
            }
            d.add_feature(FeatureKind::StructGusset(g));
            for r in &mut d.references {
                r.visible = false;
            }
            self.load_doc(d, None, None);
            self.settings.bench = crate::settings::Bench::Estrutural;
            self.started = true;
        }
    }
}
