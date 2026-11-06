// SPDX-License-Identifier: LGPL-2.1-or-later
//! O visualizador dentro do PLM (B339, ADR 0066): pré-visualização de peça e de montagem pelo pacote
//! `.fkv`, sem abrir o Fenkra, e comparação de versões.
//!
//! Como funciona:
//! 1. cada versão de um documento do projeto tem, no máximo, um pacote em `.visualizacao/`, nomeado pelo
//!    hash da versão (ver `fenkra_project::visualizacao`). Com a opção do projeto ligada, ele é gerado
//!    em segundo plano logo depois de gravar uma versão nova; sem ela, é gerado quando alguém pede;
//! 2. o egui não embute página web. "Visualizar" grava uma página **autocontida** (a página do
//!    visualizador com o pacote dentro, e o da versão anterior quando é comparação) e a entrega ao
//!    navegador padrão. Não há servidor, porta nem rede: o navegador abre um arquivo local;
//! 3. a geração roda numa linha de execução própria, com uma cópia do documento da versão pedida. O
//!    resultado volta por uma fila e quem grava no projeto é o laço da interface.

use super::*;
use fenkra_pacote::{de_montagem, de_peca, pagina_html_completa, Opcoes};
use fenkra_project::visualizacao::OpcoesVisualizacao;
use std::path::Path;
use std::sync::Mutex;

/// O que vai ser feito quando o pacote da versão ficar pronto.
#[derive(Clone, Debug, PartialEq)]
struct PedidoAbrir {
    raiz: PathBuf,
    id: u64,
    /// versão nova (a que se vê) e, se for comparação, a antiga
    novo: String,
    antigo: Option<String>,
}

enum Destino {
    Versao { raiz: PathBuf, id: u64, hash: String },
    Montagem { nome: String, saida: PathBuf },
}

struct Pronto {
    destino: Destino,
    pacote: Result<Vec<u8>, String>,
}

static PRONTOS: Mutex<Vec<Pronto>> = Mutex::new(Vec::new());
static EM_ANDAMENTO: Mutex<Vec<(PathBuf, u64, String)>> = Mutex::new(Vec::new());
static ABRIR: Mutex<Vec<PedidoAbrir>> = Mutex::new(Vec::new());

fn opcoes_de(o: &OpcoesVisualizacao) -> Opcoes {
    if o.simplificada {
        Opcoes { decimar: Some(0.3), capturas: false, ..Default::default() }
    } else {
        Opcoes::default()
    }
}

/// Entrega o arquivo ao programa padrão do sistema (o navegador, para `.html`).
pub(crate) fn abrir_no_sistema(caminho: &Path) -> std::io::Result<()> {
    #[cfg(target_os = "windows")]
    let r = std::process::Command::new("rundll32").arg("url.dll,FileProtocolHandler").arg(caminho).spawn();
    #[cfg(target_os = "macos")]
    let r = std::process::Command::new("open").arg(caminho).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let r = std::process::Command::new("xdg-open").arg(caminho).spawn();
    r.map(|_| ())
}

/// Dados do PLM que a página mostra ao lado do título: estado, revisão e versão.
fn meta_do_documento(p: &Project, id: u64, hash: &str, comparando: Option<(&str, &str)>) -> String {
    let Ok(e) = p.entry(id) else { return String::new() };
    let numero = e.versions.iter().rposition(|v| v.hash == hash).map(|i| i + 1);
    let revisao = e.revisions.iter().rev().find(|r| r.hash == hash).map(|r| r.label.clone());
    let mut m = serde_json::json!({
        "estado": tr(e.state.label()),
        "revisao": revisao,
        "versao": numero.map(|n| format!("v{n}")),
        "pn": e.pn,
        "nome": e.name,
    });
    if let Some((antes, depois)) = comparando {
        m["comparando"] = serde_json::json!({"antes": antes, "depois": depois});
    }
    m.to_string()
}

impl App {
    /// Depois de gravar uma versão: com a opção do projeto ligada, gera o pacote dela em segundo plano.
    pub(super) fn visualizacao_ao_salvar(&mut self, id: u64) {
        let Some(p) = self.project.as_ref() else { return };
        if !p.opcoes_visualizacao().gerar_ao_salvar {
            return;
        }
        if let Some(hash) = p.visualizacao_pendente(id) {
            self.pedir_visualizacao(id, &hash);
        }
    }

    /// Começa a gerar o pacote da versão `hash`. Devolve se começou (já pronto ou em andamento: não).
    fn pedir_visualizacao(&mut self, id: u64, hash: &str) -> bool {
        let Some(p) = self.project.as_ref() else { return false };
        if p.visualizacao_da_versao(id, hash).is_some() {
            return false;
        }
        let chave = (p.root.clone(), id, hash.to_string());
        {
            let mut andamento = EM_ANDAMENTO.lock().unwrap_or_else(|e| e.into_inner());
            if andamento.contains(&chave) {
                return false;
            }
            andamento.push(chave.clone());
        }
        let doc = match p.load_version(hash) {
            Ok(d) => d,
            Err(e) => {
                EM_ANDAMENTO.lock().unwrap_or_else(|e| e.into_inner()).retain(|c| c != &chave);
                self.toast(e.to_string(), true);
                return false;
            }
        };
        let opc = opcoes_de(&p.opcoes_visualizacao());
        let (raiz, hash) = (p.root.clone(), hash.to_string());
        std::thread::spawn(move || {
            let pacote = de_peca(&doc, &opc).and_then(|p| p.bytes());
            EM_ANDAMENTO.lock().unwrap_or_else(|e| e.into_inner()).retain(|c| c != &(raiz.clone(), id, hash.clone()));
            PRONTOS.lock().unwrap_or_else(|e| e.into_inner()).push(Pronto { destino: Destino::Versao { raiz, id, hash }, pacote });
        });
        true
    }

    /// Roda a cada quadro (chamado por `janelas_projeto`): grava os pacotes que ficaram prontos e abre
    /// no navegador o que alguém pediu para ver.
    pub(super) fn visualizador_quadro(&mut self, ctx: &egui::Context) {
        // camadas de comparação no 3D (B214, B130): recolhe o cálculo pronto e mostra a legenda
        match crate::camada3d::receber() {
            Some(Ok(())) => {
                self.scene_dirty = true;
                ctx.request_repaint();
            }
            Some(Err(e)) => self.toast(e, true),
            None => {}
        }
        if crate::camada3d::janela(ctx) {
            self.scene_dirty = true;
        }
        let prontos: Vec<Pronto> = std::mem::take(&mut *PRONTOS.lock().unwrap_or_else(|e| e.into_inner()));
        for pronto in prontos {
            match (pronto.destino, pronto.pacote) {
                (Destino::Versao { raiz, id, hash }, Ok(bytes)) => {
                    if self.project.as_ref().is_some_and(|p| p.root == raiz) {
                        if let Err(e) = self.project.as_ref().unwrap().gravar_visualizacao(id, &hash, &bytes) {
                            self.toast(format!("{}: {e}", tr("Falha ao gravar a visualização")), true);
                        }
                    }
                }
                (Destino::Versao { hash, .. }, Err(e)) => {
                    ABRIR.lock().unwrap_or_else(|a| a.into_inner()).retain(|a| a.novo != hash && a.antigo.as_deref() != Some(hash.as_str()));
                    self.toast(format!("{}: {e}", tr("Falha ao gerar a visualização")), true);
                }
                (Destino::Montagem { nome, saida }, Ok(bytes)) => {
                    let html = pagina_html_completa(Some(&bytes), None, Some(&serde_json::json!({"nome": nome}).to_string()));
                    let arquivo = saida.join(format!("{}.html", nome_de_arquivo(&nome)));
                    match std::fs::create_dir_all(&saida).and_then(|_| std::fs::write(&arquivo, html)).and_then(|_| abrir_no_sistema(&arquivo)) {
                        Ok(()) => self.toast(format!("{}: {}", tr("Aberto no navegador"), arquivo.display()), false),
                        Err(e) => self.toast(format!("{}: {e}", tr("Falha ao abrir a visualização")), true),
                    }
                }
                (Destino::Montagem { .. }, Err(e)) => self.toast(format!("{}: {e}", tr("Falha ao gerar a visualização")), true),
            }
        }
        // pedidos de abrir: só quando todos os pacotes de que precisam existem
        let pedidos: Vec<PedidoAbrir> = ABRIR.lock().unwrap_or_else(|a| a.into_inner()).clone();
        for pedido in pedidos {
            if self.project.as_ref().map_or(true, |p| p.root != pedido.raiz) {
                ABRIR.lock().unwrap_or_else(|a| a.into_inner()).retain(|a| a != &pedido);
                continue;
            }
            let (pronto_novo, pronto_antigo) = {
                let p = self.project.as_ref().unwrap();
                (p.visualizacao_da_versao(pedido.id, &pedido.novo).is_some(), pedido.antigo.as_ref().map_or(true, |a| p.visualizacao_da_versao(pedido.id, a).is_some()))
            };
            if pronto_novo && pronto_antigo {
                ABRIR.lock().unwrap_or_else(|a| a.into_inner()).retain(|a| a != &pedido);
                self.abrir_pagina(&pedido);
            }
        }
        if !EM_ANDAMENTO.lock().unwrap_or_else(|e| e.into_inner()).is_empty() {
            ctx.request_repaint_after(std::time::Duration::from_millis(250));
        }
    }

    fn abrir_pagina(&mut self, pedido: &PedidoAbrir) {
        let p = self.project.as_ref().unwrap();
        let ler = |h: &str| p.visualizacao_da_versao(pedido.id, h).and_then(|c| std::fs::read(c).ok());
        let (Some(novo), antigo) = (ler(&pedido.novo), pedido.antigo.as_deref().and_then(ler)) else {
            return self.toast(tr("O pacote de visualização não pôde ser lido."), true);
        };
        let rotulos = pedido.antigo.as_ref().map(|a| {
            let n = |h: &str| p.entry(pedido.id).ok().and_then(|e| e.versions.iter().rposition(|v| v.hash == h)).map(|i| format!("v{}", i + 1)).unwrap_or_default();
            (n(a), n(&pedido.novo))
        });
        let meta = meta_do_documento(p, pedido.id, &pedido.novo, rotulos.as_ref().map(|(a, b)| (a.as_str(), b.as_str())));
        let html = pagina_html_completa(Some(&novo), antigo.as_deref(), Some(&meta));
        let nome = format!("{}-{}", pedido.id, &pedido.novo[..pedido.novo.len().min(8)]);
        let arquivo = p.root.join(fenkra_project::visualizacao::PASTA).join("paginas").join(format!("{nome}.html"));
        let r = arquivo.parent().map(std::fs::create_dir_all).unwrap_or(Ok(())).and_then(|_| std::fs::write(&arquivo, html)).and_then(|_| abrir_no_sistema(&arquivo));
        match r {
            Ok(()) => self.toast(tr("Aberto no navegador"), false),
            Err(e) => self.toast(format!("{}: {e}", tr("Falha ao abrir a visualização")), true),
        }
    }

    /// Abre a versão `novo` (e, se dado, compara com `antigo`) no navegador, gerando o que faltar.
    pub(super) fn visualizar_versao(&mut self, id: u64, novo: &str, antigo: Option<&str>) {
        let Some(p) = self.project.as_ref() else { return };
        let pedido = PedidoAbrir { raiz: p.root.clone(), id, novo: novo.into(), antigo: antigo.map(str::to_string) };
        let mut faltou = false;
        for h in std::iter::once(novo).chain(antigo) {
            faltou |= self.pedir_visualizacao(id, h);
        }
        {
            let mut a = ABRIR.lock().unwrap_or_else(|a| a.into_inner());
            if !a.contains(&pedido) {
                a.push(pedido);
            }
        }
        if faltou {
            self.toast(tr("Gerando a visualização… abre no navegador quando ficar pronta."), false);
        }
    }

    /// Visualiza no navegador a montagem aberta, sem precisar do Fenkra para ver (B339).
    fn visualizar_montagem_aberta(&mut self) {
        let Some(s) = self.asm.as_ref() else { return };
        let Some(pasta) = s.file.as_ref().and_then(|f| f.parent()).map(Path::to_path_buf) else {
            return self.toast(tr("Salve a montagem antes de visualizar: as peças são lidas dos arquivos dela."), true);
        };
        let opc = self.project.as_ref().map(|p| opcoes_de(&p.opcoes_visualizacao())).unwrap_or_default();
        let saida = self.project.as_ref().map(|p| p.root.join(fenkra_project::visualizacao::PASTA).join("montagens")).unwrap_or_else(|| std::env::temp_dir().join("fenkra_visualizacao"));
        let (asm, nome) = (s.asm.clone(), s.asm.name.clone());
        self.toast(tr("Gerando a visualização… abre no navegador quando ficar pronta."), false);
        std::thread::spawn(move || {
            let pacote = de_montagem(&asm, &pasta, &opc).and_then(|p| p.bytes());
            PRONTOS.lock().unwrap_or_else(|e| e.into_inner()).push(Pronto { destino: Destino::Montagem { nome, saida }, pacote });
        });
    }

    /// Seção "Visualizador" do painel do projeto: a opção do projeto e os botões de ver (B339).
    pub(super) fn secao_visualizador(&mut self, ui: &mut egui::Ui, sel: u64) {
        let Some(p) = self.project.as_ref() else { return };
        let mut opc = p.opcoes_visualizacao();
        let Ok(e) = p.entry(sel) else { return };
        let atual = e.versions.last().map(|v| v.hash.clone());
        let pronto = atual.as_ref().and_then(|h| p.visualizacao_da_versao(sel, h)).and_then(|c| std::fs::metadata(c).ok()).map(|m| m.len());
        let ha_versoes = e.versions.len();
        let penultima = (ha_versoes >= 2).then(|| e.versions[ha_versoes - 2].hash.clone());
        egui::CollapsingHeader::new(tr("Visualizador (navegador)")).id_salt(("visualizador", sel)).show(ui, |ui| {
            let antes = opc.clone();
            ui.checkbox(&mut opc.gerar_ao_salvar, tr("Gerar a visualização ao salvar")).on_hover_text(tr("Opção do projeto. Depois de cada versão salva, o pacote .fkv é gerado em segundo plano para abrir rápido no navegador, sem abrir o Fenkra."));
            ui.checkbox(&mut opc.simplificada, tr("Versão simplificada")).on_hover_text(tr("Reduz a malha e tira os pontos de medição exatos: protege a geometria e o pacote fica menor."));
            if opc != antes {
                if let Err(err) = self.project.as_mut().unwrap().definir_opcoes_visualizacao(&opc) {
                    self.toast(err.to_string(), true);
                }
            }
            match (pronto, atual.is_some()) {
                (Some(b), _) => {
                    ui.label(RichText::new(format!("{} ({} KB)", tr("Pacote da versão atual pronto"), b / 1024)).weak().small());
                }
                (None, true) => {
                    ui.label(RichText::new(tr("A versão atual ainda não tem pacote; ele é gerado ao pedir.")).weak().small());
                }
                _ => {}
            }
            ui.horizontal_wrapped(|ui| {
                let r = ui.add_enabled(atual.is_some(), egui::Button::new(tr("Ver no navegador"))).on_hover_text(tr("Abre a versão atual no visualizador, no navegador padrão, com atributos, estado, zonas e análise."));
                if r.clicked() {
                    self.visualizar_versao(sel, atual.as_deref().unwrap(), None);
                }
                let r = ui.add_enabled(penultima.is_some(), egui::Button::new(tr("Comparar com a anterior"))).on_hover_text(tr("Abre no navegador a versão atual e a anterior, com o que foi adicionado, removido, movido e alterado."));
                if r.clicked() {
                    self.visualizar_versao(sel, atual.as_deref().unwrap(), penultima.as_deref());
                }
                let r = ui.add_enabled(self.asm.is_some(), egui::Button::new(tr("Ver montagem aberta"))).on_hover_text(tr("Abre no navegador a montagem aberta, com interferências, folgas e peças por zona já calculadas."));
                if r.clicked() {
                    self.visualizar_montagem_aberta();
                }
            });
        });
    }
}

/// Nome seguro para arquivo: o que não é letra, número, hífen ou sublinhado vira sublinhado.
fn nome_de_arquivo(n: &str) -> String {
    let s: String = n.chars().map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect();
    if s.is_empty() {
        "montagem".into()
    } else {
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nome_de_arquivo_nao_deixa_passar_separadores() {
        assert_eq!(nome_de_arquivo("Conjunto A/B:1"), "Conjunto_A_B_1");
        assert_eq!(nome_de_arquivo("..\\x"), "___x");
        assert_eq!(nome_de_arquivo(""), "montagem");
    }

    #[test]
    fn opcao_simplificada_decima_e_tira_os_pontos_exatos() {
        let o = opcoes_de(&OpcoesVisualizacao { gerar_ao_salvar: true, simplificada: true });
        assert_eq!(o.decimar, Some(0.3));
        assert!(!o.capturas);
        let c = opcoes_de(&OpcoesVisualizacao::default());
        assert_eq!(c.decimar, None);
        assert!(c.capturas);
    }

    #[test]
    fn meta_do_plm_leva_estado_revisao_e_versao() {
        let d = std::env::temp_dir().join(format!("fenkra_viz_meta_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let mut p = Project::create(&d, "P").unwrap();
        let id = p.add_document(Document::new("Placa")).unwrap();
        p.release(id, "primeira").unwrap();
        let hash = p.entry(id).unwrap().versions.last().unwrap().hash.clone();
        let m: serde_json::Value = serde_json::from_str(&meta_do_documento(&p, id, &hash, Some(("v1", "v2")))).unwrap();
        assert_eq!(m["revisao"], "A");
        assert_eq!(m["versao"], "v1");
        assert_eq!(m["comparando"]["antes"], "v1");
        assert!(m["estado"].as_str().is_some_and(|s| !s.is_empty()));
        let _ = std::fs::remove_dir_all(&d);
    }

    /// O pacote gerado de uma versão e a página com ele dentro: o que "Ver no navegador" entrega.
    #[test]
    fn pagina_do_plm_leva_o_pacote_e_o_estado() {
        use fenkra_core::{BoolOp, FeatureKind, OriginPlane, PlaneRef, PrimKind, Primitive};
        let s = |x: &str| x.to_string();
        let mut doc = Document::new("Placa");
        doc.add_feature(FeatureKind::Primitive(Primitive { kind: PrimKind::Caixa, plane: PlaneRef::Origin(OriginPlane::XY), a: s("40"), b: s("20"), c: s("5"), d: s("0"), op: BoolOp::Add }));
        let pacote = de_peca(&doc, &Opcoes::default()).unwrap().bytes().unwrap();
        let html = pagina_html_completa(Some(&pacote), None, Some(r#"{"estado":"Em trabalho","nome":"</script><b>"}"#));
        assert!(html.contains("Em trabalho"));
        assert!(!html.contains("</script><b>"), "o JSON do PLM não pode fechar a marca da página");
        assert!(html.contains("\\u003c/script>"));
    }
}
