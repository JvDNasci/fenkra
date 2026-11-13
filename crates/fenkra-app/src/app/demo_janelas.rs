// SPDX-License-Identifier: LGPL-2.1-or-later
//! Desenvolvimento: `FENKRA_DEMO_JANELA=<nome>` abre, no segundo quadro, uma janela ou painel que só o
//! mouse abriria, para a captura (`FENKRA_DEMO_FOTO`). Nomes: `salvar_como`, `empacotar`, `reparar`,
//! `liberar` (combine com `FENKRA_DEMO_PROJETO=1`), `resumo` (com `FENKRA_DEMO_EST=1`), `roda`, `roda_editor`,
//! `ramos` (com `FENKRA_DEMO_PROJETO=1`: ramo em conflito e janela de mesclagem), `tabela` (parâmetros com a tabela de projeto preenchida), `reparo_candidatas` (assistente de
//! reparo com as candidatas da face perdida numeradas no 3D).

use super::*;

impl App {
    pub(super) fn demo_janela(&mut self, ctx: &egui::Context) {
        static FEITO: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
        let Ok(nome) = std::env::var("FENKRA_DEMO_JANELA") else { return };
        self.demo_diagnostico_acao(ctx);
        // a vista da montagem compõe fora da thread da interface: tenta a cada quadro até a chamada nascer
        if nome == "pmi_feature_asm" {
            static CRIADA: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
            if ctx.input(|i| i.time) > 3. && !CRIADA.load(std::sync::atomic::Ordering::Relaxed) && self.demo_feature_montagem() {
                CRIADA.store(true, std::sync::atomic::Ordering::Relaxed);
            }
            return;
        }
        if ctx.input(|i| i.time) < 2. || FEITO.swap(true, std::sync::atomic::Ordering::Relaxed) {
            return;
        }
        if matches!(nome.as_str(), "salvar_como" | "empacotar" | "reparar") && self.open.file.is_none() {
            self.demo_arquivo();
        }
        if nome == "ramos" {
            self.demo_ramos();
        }
        if nome == "liberar" {
            self.demo_conjunto_que_usa();
        }
        match nome.as_str() {
            "salvar_como" => self.abrir_salvar_como(ctx),
            "empacotar" => self.abrir_empacotar(ctx),
            "reparar" => self.abrir_reparar_links(ctx),
            "liberar" => {
                if let Some(id) = self.project_sel {
                    self.release_dialog = Some((id, String::new()));
                }
            }
            "ramos" => self.demo_abrir_mescla(ctx),
            // B15: casca com a face perdida e o assistente de reparo aberto, candidatas numeradas no 3D
            "reparo_candidatas" => {
                let mut d = Document::new("Reparo");
                let mut data = fenkra_core::SketchData::default();
                let mut n = 9_700_000u64;
                let mut al = || {
                    n += 1;
                    Id(n)
                };
                data.add_rect(&mut &mut al, [-20., -15.], [20., 15.], false);
                let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
                d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind("20".into()), reverse: false, taper: "0".into(), thin: None, thin_side: 0, dir: None, ends: Default::default() }));
                // o ponto ficou 3 mm acima do topo (o topo baixou depois do clique)
                d.add_feature(FeatureKind::Shell(Shell { espessuras: vec![], faces: vec![[6., 4., 23.]], thickness: "1.5".into(), outward: false, face_keys: vec![None], closed: false, regras: vec![] }));
                self.open.doc = d;
                self.sent_doc = None;
                self.request_recompute();
                self.wizard_open = true;
            }
            "resumo" => crate::st_montagem_ui::abrir(self),
            "roda" => {
                let r = ctx.content_rect();
                self.roda.fase = crate::roda::Fase::Fixa { p0: [r.center().x, r.center().y] };
            }
            // anotação selecionada: `pmi_barra` abre a mini barra da cota; `pmi_fcf` o painel da tolerância geométrica
            "pmi_barra" | "pmi_fcf" => {
                use fenkra_core::pmi::PmiKind;
                let barra = nome == "pmi_barra";
                let alvo = self.open.doc.pmi.iter().find(|p| if barra { matches!(p.kind, PmiKind::Linear) } else { matches!(p.kind, PmiKind::Gdt { .. }) }).map(|p| p.id);
                if let Some(id) = alvo {
                    self.sel = Sel::Pmi(id);
                    if barra {
                        crate::pmi_minibar::open_for(self, id);
                    }
                }
            }
            // balões automáticos e legenda 3D: o diálogo de confirmação, com `FENKRA_DEMO_ASM=1` (balões)
            "pmi_baloes" => crate::pmi_ui::command(self, 10),
            "pmi_legenda" => crate::pmi_ui::command(self, 9),
            // reporte do sketch: retângulo, linha e ponto publicados pelo menu do sketch
            "sketch_publicar" => {
                let (d, ultimo) = crate::sketch_pub::documento_demo();
                self.open.doc = d;
                self.sent_doc = None;
                self.sel = Sel::Ref(ultimo);
            }
            // B407: três curvas ligadas lado a lado e uma usada por outro elemento; a do meio selecionada
            "curvas_ligadas" => {
                let mut d = Document::new("Peça ligada");
                let ids: Vec<Id> = (0..3)
                    .map(|i| {
                        let y = i as f64 * 6.;
                        d.add_link("mestre", "mestre.fkp", &format!("LONG_E_{i}"), fenkra_core::LinkedGeo::Curve(vec![[0., y, 0.], [120., y, 0.]]))
                    })
                    .collect();
                d.add_geometry(RefKind::PointOnCurve { curve: CurveRef::Ref(ids[0]), at: OnCurve::Ratio("0.5".into()), reverse: false });
                self.open.doc = d;
                self.sent_doc = None;
                self.sel = Sel::Ref(ids[1]);
            }
            // B16: `ocultar_pasta` põe os sketches do primeiro corpo numa pasta de comandos oculta;
            // `ocultar_corpo` oculta o primeiro corpo (os sketches dele somem junto, cada um com o seu estado)
            "ocultar_pasta" | "ocultar_corpo" => {
                let d = &mut self.open.doc;
                if let Some(c) = d.collectors.first().map(|c| c.id) {
                    if nome == "ocultar_corpo" {
                        d.collectors[0].visible = false;
                    } else {
                        let fo = d.add_folder("Esboços");
                        let sks: Vec<Id> = d.features.iter().filter(|f| f.target == c && matches!(f.kind, FeatureKind::Sketch(_)) && !d.is_free(f)).map(|f| f.id).collect();
                        for s in sks {
                            if let Some(f) = d.feature_mut(s) {
                                f.folder = Some(fo);
                                f.hidden = false;
                            }
                        }
                        if let Some(f) = d.folders.iter_mut().find(|f| f.id == fo) {
                            f.hidden = true;
                        }
                    }
                    self.scene_dirty = true;
                }
            }
            // B95: caixa com as arestas verticais arredondadas e um filete com a regra "tangentes"
            // gravada a partir de uma aresta de cima; o painel do filete aberto mostra a regra
            "filete_regra" => {
                let mut d = Document::new("Regra gravada");
                let mut sk = fenkra_core::SketchData::default();
                let mut al = || d.alloc();
                sk.add_rect(&mut al, [-20., -15.], [20., 15.], false);
                let s = d.add_feature(FeatureKind::Sketch(fenkra_core::Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data: sk }));
                d.add_feature(FeatureKind::Extrude(fenkra_core::Extrude { sketch: s, op: fenkra_core::BoolOp::Add, extent: fenkra_core::Extent::Blind("20".into()), reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
                let canto = |m: Vec<[f64; 3]>, v: &str, regras: Vec<fenkra_core::RegraAresta>| {
                    FeatureKind::Fillet(fenkra_core::EdgeOp { value: v.into(), edges: EdgeSel::Edges(m), mode: EdgeMode::Constant, prop: Default::default(), keys: vec![], extra: EdgeExtra { regras, ..Default::default() } })
                };
                d.add_feature(canto(vec![[20., 15., 10.], [-20., 15., 10.], [-20., -15., 10.], [20., -15., 10.]], "4", vec![]));
                let f = d.add_feature(canto(vec![[0., -15., 20.]], "1.5", vec![fenkra_core::RegraAresta { semente: 0, regra: fenkra_core::RegraSelecao::Tangentes }]));
                self.open.doc = d;
                self.sent_doc = None;
                self.start_session(f, false, None);
            }
            // B95: saída de 3° gravada pela regra "tangentes" a partir de uma parede: os cantos
            // arredondados entram sozinhos, com o painel aberto mostrando a regra
            "saida_regra" => {
                let mut d = Document::new("Regra de faces");
                let mut sk = fenkra_core::SketchData::default();
                let mut al = || d.alloc();
                sk.add_rect(&mut al, [-20., -15.], [20., 15.], false);
                let s = d.add_feature(FeatureKind::Sketch(fenkra_core::Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data: sk }));
                d.add_feature(FeatureKind::Extrude(fenkra_core::Extrude { sketch: s, op: fenkra_core::BoolOp::Add, extent: fenkra_core::Extent::Blind("20".into()), reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
                d.add_feature(FeatureKind::Fillet(fenkra_core::EdgeOp { value: "6".into(), edges: EdgeSel::Edges(vec![[20., 15., 10.], [-20., 15., 10.], [-20., -15., 10.], [20., -15., 10.]]), mode: EdgeMode::Constant, prop: Default::default(), keys: vec![], extra: EdgeExtra::default() }));
                let f = d.add_feature(FeatureKind::Draft(fenkra_core::Draft { faces: vec![[0., -15., 10.]], neutral: PlaneRef::Origin(OriginPlane::XY), angle: "8".into(), reverse: false, face_keys: vec![], regras: vec![fenkra_core::RegraAresta { semente: 0, regra: fenkra_core::RegraSelecao::Tangentes }] }));
                self.open.doc = d;
                self.sent_doc = None;
                self.start_session(f, false, None);
            }
            // B20: furo novo aberto, com o botão Aplicar e continuar na barra do comando
            "aplicar_continuar" => {
                let mut d = Document::new("Aplicar e continuar");
                let mut sk = fenkra_core::SketchData::default();
                let mut al = || d.alloc();
                sk.add_rect(&mut al, [-20., -15.], [20., 15.], false);
                let s = d.add_feature(FeatureKind::Sketch(fenkra_core::Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data: sk }));
                d.add_feature(FeatureKind::Extrude(fenkra_core::Extrude { sketch: s, op: fenkra_core::BoolOp::Add, extent: fenkra_core::Extent::Blind("20".into()), reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
                let snap = d.clone();
                let f = d.add_feature(FeatureKind::Fillet(fenkra_core::EdgeOp { value: "3".into(), edges: EdgeSel::Edges(vec![[0., -15., 20.]]), mode: EdgeMode::Constant, prop: Default::default(), keys: vec![], extra: EdgeExtra::default() }));
                self.open.doc = d;
                self.sent_doc = None;
                self.start_session(f, true, Some(snap));
            }
            // B24: "Selecionados → nova pasta" aplicado a dois pontos, escolhendo como pai a pasta
            // de geometria que já existe: a pasta nova aparece dentro dela, com os dois
            "nova_pasta" => {
                let d = &mut self.open.doc;
                let p1 = d.add_geometry(RefKind::PointCoords { x: "10".into(), y: "0".into(), z: "20".into(), frame: None });
                let p2 = d.add_geometry(RefKind::PointCoords { x: "30".into(), y: "0".into(), z: "20".into(), frame: None });
                let pai = d.folders.iter().find(|f| f.kind == fenkra_core::FolderKind::Geometria).map(|f| f.id);
                if let Ok(n) = d.agrupar_em_pasta(&[p1, p2], pai, "Selecionados") {
                    self.sel = Sel::Folder(n);
                }
                self.scene_dirty = true;
            }
            // B24: ponto definido no plano YZ, selecionado: as setas do referencial dele aparecem no 3D
            "ponto_referencial" => {
                let p = self.open.doc.add_geometry(RefKind::PointCoords { x: "20".into(), y: "20".into(), z: "30".into(), frame: Some(PlaneRef::Origin(OriginPlane::YZ)) });
                self.sel = Sel::Ref(p);
                self.scene_dirty = true;
            }
            "daltonico" => self.settings.acessibilidade.paleta_segura = true,
            // B322: configurações na página de navegação, onde ficam o mouse 3D e os botões dele
            "config_navegacao" => {
                self.settings_open = true;
                self.settings_page = 3;
                let filete = crate::ajuda_cmd::comandos().into_iter().find(|i| i.name == "Filete").map(|i| i.icon.to_string()).unwrap_or_default();
                self.settings.mouse3d.comandos = vec![String::new(), filete];
            }
            // B378: o gerenciador único de salvamento e exportação
            "gerenciador" => self.abrir_gerenciador(),
            // B408: o painel da peça aberto pelo clique na raiz da árvore; `FENKRA_DEMO_MATERIAL=<nome>`
            // escolhe o material (um nome fora da tabela mostra o aviso)
            "peca" => {
                if let Ok(m) = std::env::var("FENKRA_DEMO_MATERIAL") {
                    self.open.doc.props.material = m;
                }
                self.sel = Sel::Part;
            }
            "diagnostico" => self.demo_diagnostico(),
            // B361: a face de cima do primeiro corpo já escolhida (contorno externo, furos de fora)
            // B372: `roda_face` escolhe a face de cima e abre a roda, para ver o conteúdo por seleção
            "face" | "roda_face" => {
                if nome == "roda_face" {
                    let r = ctx.content_rect();
                    self.roda.fase = crate::roda::Fase::Fixa { p0: [r.center().x, r.center().y] };
                }
                let alvo = self.result.as_ref().and_then(|r| r.bodies.first()).and_then(|b| {
                    let m = &b.mesh;
                    (0..m.face_ranges.len()).filter_map(|f| {
                        let v = m.indices[m.face_ranges[f] as usize] as usize;
                        (m.normals[v * 3 + 2] > 0.99).then(|| (f, m.positions[v * 3 + 2], [m.positions[v * 3], m.positions[v * 3 + 1], m.positions[v * 3 + 2]]))
                    }).max_by(|a, b| a.1.total_cmp(&b.1)).map(|(f, _, p)| (f, p, b.face_owner.get(f).copied().unwrap_or(b.collector)))
                });
                if let Some((face, point, owner)) = alvo {
                    self.picks = vec![crate::picking::Pick::Face { body: 0, face, owner, point, normal: [0., 0., 1.] }];
                }
            }
            // B355: abre o último comando e conclui: a peça não pode ficar laranja
            "concluir" => {
                if let Some(id) = self.open.doc.features.last().map(|f| f.id) {
                    self.start_session(id, false, None);
                    self.conclude_session();
                }
            }
            "roda_editor" => crate::roda::ui::pedir_abrir(ctx),
            // B60: a ajuda do comando que o F1 abre, para a primeira entrada com passo a passo
            "ajuda_f1" => {
                if let Some(i) = crate::ajuda_cmd::catalogo().iter().find(|i| i.name.contains("Filete") && !i.advanced.is_empty()) {
                    crate::ajuda_f1::abrir(ctx, i.icon);
                }
            }
            // B59: modo Scan ligado olhando o primeiro comando sólido da peça
            "scan" => {
                crate::scan::alternar(self);
                if let Some(f) = self.open.doc.features.iter().find(|f| !self.open.doc.is_free(f) && crate::feat_ui::is_shape(&f.kind)) {
                    self.sel = Sel::Feature(f.id);
                }
            }
            "tabela" => {
                let s = |x: &str| x.to_string();
                let d = &mut self.open.doc;
                let nome_p = d.params.first().map(|p| p.name.clone()).unwrap_or_default();
                d.tabela.colunas = vec![fenkra_core::configuracoes::Coluna::Parametro(nome_p), fenkra_core::configuracoes::Coluna::Pn];
                d.tabela.linhas = vec![
                    fenkra_core::configuracoes::Linha { nome: s("Fina"), celulas: vec![s("8"), s("PN-001")] },
                    fenkra_core::configuracoes::Linha { nome: s("Grossa"), celulas: vec![s("20"), s("PN-002")] },
                ];
                d.tabela.ativa = s("Inexistente");
                self.sel = Sel::Params;
            }
            _ => {}
        }
    }

    /// Grava a peça aberta numa pasta temporária, com dois links (um existe, um sumiu), para os diálogos de arquivo.
    fn demo_arquivo(&mut self) {
        let pasta = std::env::temp_dir().join(format!("fenkra-demo-arquivo-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&pasta);
        let alvo = pasta.join("modelo.fkp");
        let _ = std::fs::write(pasta.join("parafuso.fkp"), self.open.doc.to_json());
        let Ok(mut v) = serde_json::from_str::<serde_json::Value>(&self.open.doc.to_json()) else { return };
        v["links_demo"] = serde_json::json!([
            {"uid": "demo-uid-1", "caminho": "parafuso.fkp"},
            {"uid": "demo-uid-2", "caminho": "suporte_antigo.fkp"}
        ]);
        if std::fs::write(&alvo, v.to_string()).is_ok() {
            self.open.file = Some(alvo);
        }
    }

    /// Cria, ao lado da peça do projeto, uma montagem que a usa: a lista "quem usa" da liberação ganha linhas.
    fn demo_conjunto_que_usa(&mut self) {
        let Some(p) = self.project.as_ref() else { return };
        let Some(id) = self.project_sel else { return };
        let Ok(e) = p.entry(id) else { return };
        let arquivo = p.root.join(&e.file);
        let j = serde_json::json!({"name": "Conjunto de exemplo", "nodes": [
            {"id": 7, "name": "Placa esquerda", "kind": {"Part": {"path": arquivo.display().to_string()}}},
            {"id": 8, "name": "Placa direita", "kind": {"Part": {"path": arquivo.display().to_string()}}}
        ]});
        let _ = std::fs::write(p.root.join("conjunto.fka"), j.to_string());
    }
}
