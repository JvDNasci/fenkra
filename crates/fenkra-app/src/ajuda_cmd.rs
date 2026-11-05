// SPDX-License-Identifier: LGPL-2.1-or-later
//! Ajuda de comandos (B60): busca de comandos com Ctrl+K e sinônimos PT/EN, mini fluxo do
//! tooltip em dois estágios e a linha de próximo passo da barra de status.
//!
//! Tudo aqui é texto e lógica pura (sem estado do documento), para ser testável sem janela: a
//! aplicação só fornece o contexto e recebe de volta a ação escolhida.

use crate::i18n::tr;
use crate::toolbar::{self, Act, Entry, Item};
use eframe::egui::{self, RichText};

/// Tempo parado sobre o botão até aparecer o segundo estágio (mini fluxo), em segundos. O primeiro
/// estágio (nome e frase) usa o atraso de dicas do egui (500 ms).
pub const ESTAGIO_2: f64 = 1.2;

/// Sinônimos por ícone (português e inglês, sem acento, separados por espaço). Quem digita
/// "extrude", "pocket" ou "round" encontra o comando mesmo com o nome em português.
const SINONIMOS: &[(&str, &str)] = &[
    ("sketch", "sketch esboco desenho 2d perfil draw"),
    ("extrusao", "extrusao extrude pad boss empurrar"),
    ("ext_simetrico", "simetrico midplane symmetric"),
    ("recorte", "recorte cut pocket furar remover subtrair"),
    ("ext_cego", "cego blind pocket profundidade"),
    ("revolucao", "revolucao revolve revolution lathe girar torno"),
    ("ranhura", "ranhura groove"),
    ("nervura", "nervura sweep rib varredura"),
    ("rasgo_varrido", "rasgo slot sweep cut"),
    ("multisecoes", "multissecoes loft transicao"),
    ("prim_caixa", "caixa box cubo bloco"),
    ("prim_cilindro", "cilindro cylinder tubo"),
    ("prim_esfera", "esfera sphere bola"),
    ("bool_unir", "unir union fuse combinar somar boolean"),
    ("bool_subtrair", "subtrair subtract cut diferenca boolean"),
    ("bool_intersecao", "intersecao intersect common boolean"),
    ("furo_simples", "furo hole drill broca"),
    ("furo_roscado", "rosca thread tap parafuso"),
    ("arestas_selecionar", "filete fillet round arredondar raio canto"),
    ("chanfro", "chanfro chamfer bevel quebrar canto"),
    ("casca", "casca shell oco hollow parede"),
    ("saida", "saida draft angulo desmoldagem"),
    ("padrao_linear", "padrao linear pattern array repetir fileira"),
    ("padrao_circular", "padrao circular polar pattern repetir"),
    ("espelho", "espelho mirror simetria refletir"),
    ("escala", "escala scale redimensionar"),
    ("xf_mover", "mover move translate transladar"),
    ("xf_girar", "girar rotate rotacionar"),
    ("salvar", "salvar save gravar"),
    ("abrir_projeto", "abrir open carregar"),
    ("asm_inserir", "inserir insert componente montagem peca"),
    ("rest_coincidencia", "coincidencia mate coincident juntar"),
    ("rest_fixar", "fixar fix ground travar"),
    ("sm_parede", "chapa sheet metal parede base"),
    ("sm_aba", "aba flange dobra"),
    ("sm_desdobrar", "desdobrar unfold flat planificar"),
    ("des_folha", "folha sheet desenho prancha drawing"),
    ("des_pdf", "pdf exportar imprimir"),
    ("des_dxf", "dxf exportar cad"),
    ("pmi_tolerancia", "tolerancia tolerance gdt"),
    ("ajustar", "ajustar fit zoom tudo enquadrar"),
    ("ortho", "ortogonal ortho perspectiva perspective"),
    ("sk_polilinha", "linha line polilinha polyline segmento"),
    ("sk_retangulo", "retangulo rectangle rect quadrado"),
    ("sk_circulo", "circulo circle raio diametro"),
    ("sk_arco3", "arco arc curva"),
    ("sk_espelhar", "espelhar mirror simetria"),
    ("sk_offset", "offset paralela deslocar equidistante"),
    ("sk_filete", "filete fillet arredondar canto raio"),
    ("sk_chanfro", "chanfro chamfer bevel"),
    ("sk_aparar", "aparar trim cortar tesoura"),
    ("sk_estender", "estender extend prolongar"),
    ("sk_spline", "spline curva bezier livre"),
    ("sk_cota", "cota dimension medida dimensionar"),
    ("sk_concluir", "concluir finish sair fechar exit sketch"),
];

/// Comando da barra (pelo ícone) e a ação do mapa de atalhos que o dispara. A dica mostra a tecla
/// que está valendo agora, não a de fábrica (camada 1 do tooltip, B278).
pub const ATALHOS: &[(&str, crate::keymap::KeyAction)] = {
    use crate::keymap::KeyAction as K;
    &[
        ("sketch", K::Sketch),
        ("extrusao", K::Extrude),
        ("recorte", K::Cut),
        ("revolucao", K::Revolve),
        ("furo_simples", K::Hole),
        ("arestas_selecionar", K::Fillet),
        ("chanfro", K::Chamfer),
        ("ajustar", K::Fit),
        ("ortho", K::Ortho),
        ("salvar", K::Save),
        ("sk_selecionar", K::SkSelect),
        ("sk_polilinha", K::SkLine),
        ("sk_retangulo", K::SkRect),
        ("sk_circulo", K::SkCircle),
        ("sk_arco3", K::SkArc),
        ("sk_ponto", K::SkPoint),
        ("sk_cota", K::SkDim),
        ("sk_construcao", K::SkConstruction),
        ("sk_grade", K::SkGrid),
        ("sk_aparar", K::SkTrim),
        ("sk_offset", K::SkOffset),
        ("sk_filete", K::SkFillet),
        ("sk_espelhar", K::SkMirror),
        ("sk_spline", K::SkSpline),
    ]
};

fn chave_atalhos() -> egui::Id {
    egui::Id::new("atalhos_dos_comandos")
}

/// Texto do atalho de cada comando no mapa atual, por ícone.
pub fn atalhos_do_mapa(km: &crate::keymap::Keymap) -> std::collections::HashMap<String, String> {
    ATALHOS.iter().filter_map(|(icone, a)| km.binding(*a).map(|b| ((*icone).to_owned(), b.text()))).collect()
}

/// A aplicação publica o mapa a cada quadro; o tooltip lê sem precisar das configurações.
pub fn publicar_atalhos(ctx: &egui::Context, km: &crate::keymap::Keymap) {
    let mapa = atalhos_do_mapa(km);
    ctx.data_mut(|d| d.insert_temp(chave_atalhos(), mapa));
}

/// Atalho atual do comando (vazio se não tem).
pub fn atalho_do_comando(ctx: &egui::Context, icone: &str) -> Option<String> {
    ctx.data(|d| d.get_temp::<std::collections::HashMap<String, String>>(chave_atalhos())).and_then(|m| m.get(icone).cloned())
}

/// Minúsculas e sem acento: a busca ignora os dois.
pub fn normalizar(s: &str) -> String {
    s.chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            c => c,
        })
        .collect()
}

fn sinonimos(icone: &str) -> &'static str {
    SINONIMOS.iter().find(|(i, _)| *i == icone).map(|(_, s)| *s).unwrap_or("")
}

/// Todos os comandos da barra, uma vez cada (famílias abertas).
pub fn comandos() -> Vec<Item> {
    let mut out: Vec<Item> = vec![];
    for e in toolbar::all_entries() {
        let itens = match e {
            Entry::One(i) => vec![i],
            Entry::Family(_, v) => v,
        };
        for i in itens {
            if !out.iter().any(|x| x.icon == i.icon && x.name == i.name) {
                out.push(i);
            }
        }
    }
    out
}

/// Pontuação de um comando para a consulta (0 = não casa). Cada palavra da consulta precisa casar;
/// nome vale mais que sinônimo, que vale mais que a descrição.
pub fn pontuar(item: &Item, consulta: &str) -> u32 {
    let palavras: Vec<String> = normalizar(consulta).split_whitespace().map(str::to_owned).collect();
    if palavras.is_empty() {
        return 0;
    }
    let nome = normalizar(item.name);
    let sin = sinonimos(item.icon);
    let desc = normalizar(&format!("{} {}", item.brief, item.advanced));
    let mut total = 0;
    for p in &palavras {
        let pts = if nome == *p {
            100
        } else if nome.starts_with(p.as_str()) {
            80
        } else if nome.contains(p.as_str()) {
            60
        } else if sin.split_whitespace().any(|s| s == p) {
            50
        } else if sin.split_whitespace().any(|s| s.starts_with(p.as_str())) {
            40
        } else if desc.contains(p.as_str()) {
            10
        } else {
            return 0;
        };
        total += pts;
    }
    total
}

/// Comandos que casam com a consulta, do melhor ao pior.
pub fn buscar(consulta: &str, limite: usize) -> Vec<Item> {
    let mut v: Vec<(u32, Item)> = comandos().into_iter().map(|i| (pontuar(&i, consulta), i)).filter(|(p, _)| *p > 0).collect();
    v.sort_by(|a, b| b.0.cmp(&a.0));
    v.into_iter().take(limite).map(|(_, i)| i).collect()
}

/// Janela de busca de comandos (Ctrl+K).
#[derive(Default)]
pub struct Busca {
    pub aberta: bool,
    texto: String,
    sel: usize,
}

impl Busca {
    pub fn alternar(&mut self) {
        self.aberta = !self.aberta;
        self.texto.clear();
        self.sel = 0;
    }

    /// Desenha a busca e devolve a ação escolhida (Enter ou clique).
    pub fn ui(&mut self, ctx: &egui::Context) -> Option<Act> {
        if !self.aberta {
            return None;
        }
        let mut escolhida = None;
        let achados = if self.texto.trim().is_empty() { vec![] } else { buscar(&self.texto, 8) };
        self.sel = self.sel.min(achados.len().saturating_sub(1));
        let (baixo, cima, enter, esc) = ctx.input(|i| (i.key_pressed(egui::Key::ArrowDown), i.key_pressed(egui::Key::ArrowUp), i.key_pressed(egui::Key::Enter), i.key_pressed(egui::Key::Escape)));
        if baixo && self.sel + 1 < achados.len() {
            self.sel += 1;
        }
        if cima {
            self.sel = self.sel.saturating_sub(1);
        }
        if esc {
            self.aberta = false;
            return None;
        }
        if enter {
            if let Some(i) = achados.get(self.sel) {
                escolhida = Some(i.act);
            }
        }
        egui::Area::new(egui::Id::new("busca_comandos")).anchor(egui::Align2::CENTER_TOP, egui::vec2(0., 80.)).order(egui::Order::Foreground).show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_width(440.);
                let r = ui.add(egui::TextEdit::singleline(&mut self.texto).hint_text(tr("Buscar comando (ex.: extrude, filete, cota)")).desired_width(f32::INFINITY));
                r.request_focus();
                if r.changed() {
                    self.sel = 0;
                }
                if achados.is_empty() && !self.texto.trim().is_empty() {
                    ui.label(RichText::new(tr("Nenhum comando encontrado.")).weak());
                }
                for (n, i) in achados.iter().enumerate() {
                    let linha = ui.horizontal(|ui| {
                        ui.add(egui::Image::new(crate::icons::get(i.icon)).fit_to_exact_size(egui::vec2(20., 20.)));
                        ui.selectable_label(n == self.sel, format!("{}  ·  {}", tr(i.name), tr(i.brief)))
                    });
                    if linha.inner.clicked() {
                        escolhida = Some(i.act);
                    }
                }
            });
        });
        if escolhida.is_some() {
            self.aberta = false;
        }
        escolhida
    }
}

/// Divide uma descrição em passos: uma frase por passo (ponto seguido de espaço e maiúscula).
pub fn passos_do_texto(texto: &str) -> Vec<String> {
    let cs: Vec<char> = texto.chars().collect();
    let mut out = vec![];
    let mut ini = 0;
    for i in 0..cs.len() {
        let fim_de_frase = cs[i] == '.' && cs.get(i + 1) == Some(&' ') && cs.get(i + 2).is_some_and(|c| c.is_uppercase());
        if fim_de_frase {
            out.push(cs[ini..i].iter().collect::<String>().trim().to_owned());
            ini = i + 2;
        }
    }
    let resto: String = cs[ini..].iter().collect();
    let resto = resto.trim().trim_end_matches('.').trim().to_owned();
    if !resto.is_empty() {
        out.push(resto);
    }
    out.into_iter().filter(|s| !s.is_empty()).collect()
}

/// Mini fluxo do tooltip: passos numerados com o que fazer, na ordem.
pub fn mini_fluxo(ui: &mut egui::Ui, advanced: &str) {
    let passos = passos_do_texto(advanced);
    if passos.len() < 2 {
        ui.label(RichText::new(advanced).small());
        return;
    }
    let cor = crate::theme::accent(ui.visuals());
    for (n, p) in passos.iter().enumerate() {
        ui.horizontal_top(|ui| {
            let (r, _) = ui.allocate_exact_size(egui::vec2(16., 16.), egui::Sense::hover());
            ui.painter().circle_filled(r.center(), 8., cor);
            ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, (n + 1).to_string(), egui::FontId::proportional(10.), egui::Color32::WHITE);
            ui.label(RichText::new(p).small());
        });
    }
}

/// O que a barra de status precisa saber para sugerir o próximo passo.
pub struct Contexto<'a> {
    /// texto do passo atual da ferramenta de sketch em edição
    pub sketch: Option<&'a str>,
    pub n_comandos: usize,
    pub ultimo_e_sketch: bool,
    pub n_selecionados: usize,
}

/// Linha de próximo passo da barra de status (já traduzida).
pub fn proximo_passo(c: &Contexto) -> Option<String> {
    let t = if let Some(passo) = c.sketch {
        passo.to_owned()
    } else if c.n_comandos == 0 {
        tr("crie um Sketch num plano da Origem").to_owned()
    } else if c.ultimo_e_sketch {
        tr("transforme o sketch em sólido: Extrusão ou Revolução").to_owned()
    } else if c.n_selecionados > 0 {
        tr("escolha um comando para a seleção (Ctrl+K busca)").to_owned()
    } else {
        tr("selecione uma face ou aresta, ou use Ctrl+K para buscar um comando").to_owned()
    };
    Some(format!("{}: {t}", tr("Próximo passo")))
}

#[cfg(test)]
mod testes {
    use super::*;

    fn acha(q: &str) -> Vec<&'static str> {
        buscar(q, 5).into_iter().map(|i| i.name).collect()
    }

    #[test]
    fn todo_atalho_de_comando_aponta_para_botao_da_barra_e_aparece_no_mapa() {
        let icones: Vec<&str> = comandos().iter().map(|i| i.icon).collect();
        let sem: Vec<&&str> = ATALHOS.iter().map(|(i, _)| i).filter(|i| !icones.contains(i)).collect();
        assert!(sem.is_empty(), "atalho para ícone que não existe na barra: {sem:?}");
        let km = crate::keymap::Keymap::default();
        let m = atalhos_do_mapa(&km);
        assert_eq!(m.get("extrusao").map(String::as_str), Some("E"));
        // trocar a tecla no mapa muda o que a dica mostra
        let mut km = km;
        km.set(crate::keymap::KeyAction::Extrude, crate::keymap::Binding::parse("Ctrl+Shift+E"));
        assert_eq!(atalhos_do_mapa(&km).get("extrusao").map(String::as_str), Some("Ctrl+Shift+E"));
        km.set(crate::keymap::KeyAction::Extrude, None);
        assert!(!atalhos_do_mapa(&km).contains_key("extrusao"));
    }

    /// B278: o tooltip mostra nome, frase e passo a passo; os três precisam de tradução, não só os dois
    /// primeiros que `padrao.rs` confere. Comando novo sem passo a passo traduzido reprova aqui.
    #[test]
    fn todo_comando_tem_dica_em_camadas_traduzida() {
        let faltam: Vec<&str> = comandos().iter().filter(|i| i.advanced.trim().is_empty() || !crate::i18n::tem_traducao(i.advanced)).map(|i| i.name).collect();
        assert!(faltam.len() <= PASSO_A_PASSO_SEM_TRADUCAO, "{} comandos com passo a passo sem tradução (limite {PASSO_A_PASSO_SEM_TRADUCAO}): {faltam:?}", faltam.len());
    }

    /// Dívida de quando a regra nasceu; só pode encolher.
    const PASSO_A_PASSO_SEM_TRADUCAO: usize = 10;

    #[test]
    fn busca_ignora_acento_e_caixa() {
        assert_eq!(normalizar("Extrusão"), "extrusao");
        assert!(acha("EXTRUSAO").contains(&"Extrusão"));
        assert!(acha("revolucao").contains(&"Revolução"));
    }

    #[test]
    fn sinonimos_em_ingles_levam_ao_comando() {
        assert_eq!(acha("extrude").first(), Some(&"Extrusão"));
        assert!(acha("fillet").iter().any(|n| n.contains("Filete") || *n == "Raio"));
        assert!(acha("pocket").contains(&"Recorte"));
        assert!(acha("mirror").iter().any(|n| n.contains("Espelh")));
        assert!(acha("chamfer").contains(&"Chanfro"));
        assert!(acha("loft").contains(&"Multi-seções"));
        assert!(acha("zzzzqq").is_empty());
    }

    #[test]
    fn nome_vale_mais_que_descricao() {
        let v = buscar("chanfro", 3);
        assert_eq!(v.first().map(|i| i.name), Some("Chanfro"));
    }

    #[test]
    fn todo_sinonimo_aponta_para_icone_da_barra() {
        let icones: Vec<&str> = comandos().iter().map(|i| i.icon).collect();
        let sem: Vec<&&str> = SINONIMOS.iter().map(|(i, _)| i).filter(|i| !icones.contains(i)).collect();
        assert!(sem.is_empty(), "sinônimo para ícone que não existe na barra: {sem:?}");
    }

    #[test]
    fn mini_fluxo_divide_em_frases() {
        let p = passos_do_texto("Clique no centro e depois num vértice. Lados iguais inscritos (ex.: L/2) num círculo. Termina com Esc.");
        assert_eq!(p.len(), 3, "{p:?}");
        assert!(p[1].contains("ex.: L/2"));
        assert_eq!(passos_do_texto("Uma frase só.").len(), 1);
    }

    #[test]
    fn proximo_passo_segue_o_estado() {
        let base = Contexto { sketch: None, n_comandos: 0, ultimo_e_sketch: false, n_selecionados: 0 };
        assert!(proximo_passo(&base).unwrap().contains("Sketch"));
        let c = Contexto { n_comandos: 1, ultimo_e_sketch: true, ..base };
        assert!(proximo_passo(&c).unwrap().contains("Extrusão"));
        let c = Contexto { sketch: Some("clique no 2º ponto"), ..c };
        assert!(proximo_passo(&c).unwrap().ends_with("clique no 2º ponto"));
    }
}
