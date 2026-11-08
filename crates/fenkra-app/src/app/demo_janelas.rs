// SPDX-License-Identifier: LGPL-2.1-or-later
//! Desenvolvimento: `FENKRA_DEMO_JANELA=<nome>` abre, no segundo quadro, uma janela ou painel que só o
//! mouse abriria, para a captura (`FENKRA_DEMO_FOTO`). Nomes: `salvar_como`, `empacotar`, `reparar`,
//! `liberar` (combine com `FENKRA_DEMO_PROJETO=1`), `resumo` (com `FENKRA_DEMO_EST=1`), `roda`, `roda_editor`,
//! `ramos` (com `FENKRA_DEMO_PROJETO=1`: ramo em conflito e janela de mesclagem), `tabela` (parâmetros com a tabela de projeto preenchida).

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
            // B24: ponto definido no plano YZ, selecionado: as setas do referencial dele aparecem no 3D
            "ponto_referencial" => {
                let p = self.open.doc.add_geometry(RefKind::PointCoords { x: "20".into(), y: "20".into(), z: "30".into(), frame: Some(PlaneRef::Origin(OriginPlane::YZ)) });
                self.sel = Sel::Ref(p);
                self.scene_dirty = true;
            }
            "daltonico" => self.settings.acessibilidade.paleta_segura = true,
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
