// SPDX-License-Identifier: LGPL-2.1-or-later
//! Roda de comandos (B256): menu radial de 8 setores aberto por tecla configurável (padrão Shift+W).
//!
//! - segurar a tecla, apontar e soltar executa o setor sob a direção;
//! - toque curto (soltar logo, sem mover) fixa a roda aberta; clique num setor executa, Esc cancela;
//! - gesto de 60 px ou mais executa mesmo antes de a roda aparecer (memória muscular, sem desenhar);
//! - a roda do mouse vira as páginas quando há mais de oito comandos; teclas 1 a 8 escolhem direto;
//! - o conteúdo vem da bancada, do modo e da seleção; os dois primeiros setores da primeira página
//!   são os próximos comandos mais prováveis, aprendidos do uso local (nada sai da máquina).
//!
//! Este arquivo é a lógica pura (máquina de estados, geometria, conteúdo e configuração), testável
//! sem janela. O desenho e o editor ficam em `roda/ui.rs`.

pub mod ui;

use crate::settings::Bench;
use crate::toolbar::{Act, Item};
use serde::{Deserialize, Serialize};

pub const SETORES: usize = 8;
/// Raio da zona morta no centro: ali nenhum setor está apontado.
pub const RAIO_MORTO: f32 = 44.;
pub const RAIO_EXTERNO: f32 = 150.;
/// Deslocamento a partir do qual soltar executa sem precisar de roda aberta.
pub const GESTO_PX: f32 = 60.;
/// A roda aparece até este tempo depois da tecla (aceite: abrir em menos de 80 ms).
pub const ATRASO_DESENHO_S: f64 = 0.08;
/// Soltar antes disto, sem mover, é um toque: a roda fica fixa.
pub const TOQUE_S: f64 = 0.20;

/// Setor apontado por um deslocamento (dx, dy) em pixels de tela (y para baixo). O setor 0 fica em
/// cima e a numeração segue o sentido horário. Dentro da zona morta devolve `None`.
pub fn setor_do_deslocamento(dx: f32, dy: f32) -> Option<usize> {
    if dx.hypot(dy) < RAIO_MORTO {
        return None;
    }
    // ângulo medido do norte, horário, em [0, 360)
    let graus = dx.atan2(-dy).to_degrees().rem_euclid(360.);
    Some((((graus + 22.5) / 45.) as usize) % SETORES)
}

/// Direção do centro do setor, em radianos a partir do norte (horário); serve ao desenho.
pub fn angulo_do_setor(s: usize) -> f32 {
    (s as f32 * 45.).to_radians()
}

// ------------------------------------------------------------------------- máquina de estados

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Fase {
    Fechada,
    /// tecla segurada desde `t0`, com o ponteiro em `p0`
    Segurando { t0: f64, p0: [f32; 2] },
    /// toque curto: aberta até clicar num setor ou Esc
    Fixa { p0: [f32; 2] },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Evento {
    Tecla { t: f64, pos: [f32; 2] },
    Solta { t: f64, pos: [f32; 2] },
    /// clique do mouse com a roda fixa
    Clique { pos: [f32; 2] },
    Roda { passos: i32 },
    Numero(usize),
    Esc,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Saida {
    Nada,
    /// executar o item deste índice da página atual
    Executar(usize),
    Cancelou,
}

#[derive(Clone, Debug)]
pub struct Roda {
    pub fase: Fase,
    pub pagina: usize,
}

impl Default for Roda {
    fn default() -> Self {
        Roda { fase: Fase::Fechada, pagina: 0 }
    }
}

impl Roda {
    pub fn aberta(&self) -> bool {
        self.fase != Fase::Fechada
    }

    /// A roda está desenhada neste instante? Fixa sempre; segurada só depois do atraso, para o
    /// gesto rápido não piscar o menu.
    pub fn desenhada(&self, agora: f64) -> bool {
        match self.fase {
            Fase::Fechada => false,
            Fase::Fixa { .. } => true,
            Fase::Segurando { t0, .. } => agora - t0 >= ATRASO_DESENHO_S,
        }
    }

    pub fn centro(&self) -> Option<[f32; 2]> {
        match self.fase {
            Fase::Fechada => None,
            Fase::Segurando { p0, .. } | Fase::Fixa { p0 } => Some(p0),
        }
    }

    /// Avança a máquina. `paginas` é quantas páginas o conteúdo tem e `ocupados` quantos setores da
    /// página atual têm comando (setor vazio não executa).
    pub fn passo(&mut self, e: Evento, paginas: usize, ocupados: usize) -> Saida {
        let paginas = paginas.max(1);
        match (self.fase, e) {
            (Fase::Fechada, Evento::Tecla { t, pos }) => {
                self.fase = Fase::Segurando { t0: t, p0: pos };
                self.pagina = 0;
                Saida::Nada
            }
            (Fase::Segurando { t0, p0 }, Evento::Solta { t, pos }) => {
                let (dx, dy) = (pos[0] - p0[0], pos[1] - p0[1]);
                let moveu = dx.hypot(dy);
                let setor = setor_do_deslocamento(dx, dy).filter(|s| *s < ocupados);
                if moveu >= GESTO_PX || (t - t0 >= TOQUE_S && moveu >= RAIO_MORTO) {
                    self.fase = Fase::Fechada;
                    return setor.map_or(Saida::Cancelou, Saida::Executar);
                }
                if t - t0 < TOQUE_S {
                    self.fase = Fase::Fixa { p0 };
                    Saida::Nada
                } else {
                    // segurou e soltou no centro: desistiu
                    self.fase = Fase::Fechada;
                    Saida::Cancelou
                }
            }
            (Fase::Fixa { p0 }, Evento::Clique { pos }) => {
                let setor = setor_do_deslocamento(pos[0] - p0[0], pos[1] - p0[1]).filter(|s| *s < ocupados);
                self.fase = Fase::Fechada;
                setor.map_or(Saida::Cancelou, Saida::Executar)
            }
            (Fase::Segurando { .. } | Fase::Fixa { .. }, Evento::Roda { passos }) => {
                self.pagina = (self.pagina as i32 + passos).rem_euclid(paginas as i32) as usize;
                Saida::Nada
            }
            (Fase::Segurando { .. } | Fase::Fixa { .. }, Evento::Numero(n)) => {
                self.fase = Fase::Fechada;
                if (1..=SETORES).contains(&n) && n - 1 < ocupados {
                    Saida::Executar(n - 1)
                } else {
                    Saida::Cancelou
                }
            }
            (Fase::Segurando { .. } | Fase::Fixa { .. }, Evento::Esc) => {
                self.fase = Fase::Fechada;
                Saida::Cancelou
            }
            // a tecla repetida pelo sistema, soltar sem ter apertado etc. não mudam nada
            _ => Saida::Nada,
        }
    }
}

// ------------------------------------------------------------------------- contexto e conteúdo

/// O que a roda sabe do momento: bancada, modo e o tipo da seleção.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Contexto {
    pub bancada: Bench,
    pub modo: Modo,
    pub selecao: Selecao,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Modo {
    Peca,
    Sketch,
    Montagem,
    Desenho,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Selecao {
    Nada,
    Face,
    Aresta,
    /// vértice ou ponto
    Vertice,
    /// superfície (face de corpo de superfície ou superfície de referência)
    Superficie,
    Outra,
}

impl Contexto {
    /// Chave do conteúdo salvo, da mais específica à geral: `bancada:modo:selecao`.
    pub fn chaves(&self) -> [String; 4] {
        let b = match self.bancada {
            Bench::Solidos => "solidos",
            Bench::Superficies => "superficies",
            Bench::Montagem => "montagem",
            Bench::Chapa => "chapa",
            Bench::Estrutural => "estrutural",
            Bench::FormaLivre => "forma_livre",
            Bench::Desenho => "desenho",
        };
        let m = match self.modo {
            Modo::Peca => "peca",
            Modo::Sketch => "sketch",
            Modo::Montagem => "montagem",
            Modo::Desenho => "desenho",
        };
        let s = match self.selecao {
            Selecao::Nada => "nada",
            Selecao::Face => "face",
            Selecao::Aresta => "aresta",
            Selecao::Vertice => "vertice",
            Selecao::Superficie => "superficie",
            Selecao::Outra => "outra",
        };
        // da mais específica à geral; `*:modo:seleção` vale para qualquer bancada (montagem com uma face)
        [format!("{b}:{m}:{s}"), format!("*:{m}:{s}"), format!("{b}:{m}:*"), format!("*:{m}:*")]
    }

    pub fn nome(&self) -> String {
        self.chaves()[0].clone()
    }
}

/// Contextos que o editor oferece, com o rótulo em português. A roda de cada bancada e modo traz os
/// comandos do fluxo daquela bancada; com algo selecionado, traz os de face, aresta, vértice ou
/// superfície (B372). A pessoa continua podendo trocar qualquer um.
pub const CONTEXTOS: &[(&str, &str)] = &[
    ("*:peca:*", "Peça (qualquer bancada)"),
    ("solidos:peca:face", "Sólidos com uma face selecionada"),
    ("solidos:peca:aresta", "Sólidos com uma aresta selecionada"),
    ("solidos:peca:vertice", "Sólidos com um vértice selecionado"),
    ("superficies:peca:*", "Superfícies"),
    ("superficies:peca:superficie", "Superfícies com uma superfície selecionada"),
    ("superficies:peca:aresta", "Superfícies com uma aresta selecionada"),
    ("estrutural:peca:*", "Estrutural"),
    ("estrutural:peca:aresta", "Estrutural com uma linha ou aresta selecionada"),
    ("estrutural:peca:vertice", "Estrutural com um vértice selecionado"),
    ("forma_livre:peca:*", "Forma livre"),
    ("*:sketch:*", "Sketch"),
    ("chapa:peca:*", "Chapa metálica"),
    ("chapa:peca:face", "Chapa com uma face selecionada"),
    ("chapa:peca:aresta", "Chapa com uma aresta selecionada"),
    ("chapa:peca:vertice", "Chapa com um vértice selecionado"),
    ("*:montagem:*", "Montagem"),
    ("*:montagem:face", "Montagem com uma face selecionada"),
    ("*:montagem:aresta", "Montagem com uma aresta selecionada"),
    ("*:montagem:vertice", "Montagem com um vértice selecionado"),
    ("*:desenho:*", "Desenho técnico"),
];

/// Conteúdo de fábrica (ícones dos comandos da barra) de cada chave de `CONTEXTOS`.
pub fn padrao(chave: &str) -> &'static [&'static str] {
    match chave {
        "solidos:peca:face" => &["sketch", "recorte", "furo_simples", "casca", "espessura", "saida", "padrao_linear", "espelho"],
        "solidos:peca:aresta" => &["arestas_selecionar", "chanfro", "sketch", "extrusao", "recorte", "furo_simples", "espelho", "ajustar"],
        "solidos:peca:vertice" => &["sketch", "ponto_coord", "linha_pontos", "plano_3pontos", "plano_ponto_normal", "furo_simples", "extrair", "ajustar"],
        "superficies:peca:*" => &["sup_extrudada", "sup_revolucao", "sup_varredura_av", "sup_conectar", "sup_preenchida", "sup_deslocada", "hib_costurar", "hib_engrossar"],
        "superficies:peca:superficie" => &["sup_deslocada", "sup_filete", "hib_engrossar", "hib_costurar", "hib_dividir", "hib_fechar", "curva_extraida", "ajustar"],
        "superficies:peca:aresta" => &["sup_filete_aresta", "sup_extrudada", "curva_extraida", "sup_preenchida", "sup_conectar", "hib_costurar", "linha_pontos", "ajustar"],
        "estrutural:peca:*" => &["st_membro", "wf_repetir", "st_placa", "st_reforco", "st_solda", "st_canto_meia", "st_corte", "st_montagem"],
        "estrutural:peca:aresta" => &["st_membro", "st_solda", "st_reforco", "st_canto_meia", "chanfro", "arestas_selecionar", "st_corte", "ajustar"],
        "estrutural:peca:vertice" => &["st_placa", "st_reforco", "st_canto_meia", "st_membro", "ponto_coord", "linha_pontos", "st_solda", "ajustar"],
        "forma_livre:peca:*" => &["forma_caixa", "forma_esfera", "forma_cilindro", "forma_anel", "forma_plano", "sketch", "ajustar"],
        "*:sketch:*" => &["sk_cota", "sk_offset", "sk_aparar", "sk_espelhar", "sk_filete", "sk_construcao", "sk_concluir", "sk_estender"],
        "chapa:peca:*" => &["sm_parede", "sm_aba", "sm_desdobrar", "sketch", "extrusao", "recorte", "furo_simples", "ajustar"],
        "chapa:peca:face" => &["sm_aba", "sm_dobra_linha", "sm_recorte", "furo_simples", "sm_estampo", "sm_veneziana", "sm_desdobrar", "sketch"],
        "chapa:peca:aresta" => &["sm_aba", "arestas_selecionar", "chanfro", "sm_alivio_canto", "sm_fecha_canto", "sm_dobra_linha", "sm_desdobrar", "ajustar"],
        "chapa:peca:vertice" => &["sm_alivio_canto", "sm_fecha_canto", "ponto_coord", "sketch", "sm_desdobrar", "ajustar"],
        "*:montagem:*" => &["asm_inserir", "rest_coincidencia", "rest_fixar", "rest_paralelo", "rest_perpendicular", "rest_distancia", "rest_angulo", "ajustar"],
        "*:montagem:face" => &["rest_coincidencia", "rest_contato", "rest_paralelo", "rest_perpendicular", "rest_distancia", "rest_angulo", "rest_fixar", "ajustar"],
        "*:montagem:aresta" => &["rest_coincidencia", "rest_paralelo", "rest_distancia", "rest_angulo", "rest_alinhar", "rest_conjunta", "rest_fixar", "ajustar"],
        "*:montagem:vertice" => &["rest_coincidencia", "rest_distancia", "rest_fixar", "rest_conjunta", "rest_alinhar", "asm_inserir", "ajustar"],
        "*:desenho:*" => &["des_folha", "des_pdf", "des_dxf", "sk_cota", "ajustar", "salvar"],
        _ => &["sketch", "extrusao", "recorte", "revolucao", "furo_simples", "arestas_selecionar", "chanfro", "casca"],
    }
}

/// Conteúdo salvo e uso aprendido. Vai no `config.json`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// conteúdo trocado pelo usuário: (chave do contexto, ícones em ordem de setor)
    pub slots: Vec<(String, Vec<String>)>,
    pub uso: Uso,
}

impl Config {
    /// Ícones do contexto: o que o usuário salvou (da chave mais específica à geral) ou o de fábrica.
    pub fn icones(&self, ctx: &Contexto) -> Vec<String> {
        for k in ctx.chaves() {
            if let Some((_, v)) = self.slots.iter().find(|(c, _)| *c == k) {
                return v.clone();
            }
        }
        for k in ctx.chaves() {
            let p = padrao(&k);
            if !std::ptr::eq(p, padrao("")) {
                return p.iter().map(|s| (*s).to_owned()).collect();
            }
        }
        padrao("").iter().map(|s| (*s).to_owned()).collect()
    }

    pub fn definir(&mut self, chave: &str, icones: Vec<String>) {
        self.slots.retain(|(c, _)| c != chave);
        self.slots.push((chave.to_owned(), icones));
    }

    /// Volta o contexto ao conteúdo de fábrica.
    pub fn restaurar(&mut self, chave: &str) {
        self.slots.retain(|(c, _)| c != chave);
    }

    pub fn exportar(&self) -> String {
        serde_json::to_string_pretty(&self.slots).unwrap_or_default()
    }

    /// Lê um conteúdo exportado; só aceita ícones que existem na barra (`validos`).
    pub fn importar(&mut self, texto: &str, validos: &[&str]) -> Result<usize, String> {
        let v: Vec<(String, Vec<String>)> = serde_json::from_str(texto).map_err(|e| format!("arquivo inválido: {e}"))?;
        for (c, ics) in &v {
            if c.is_empty() || ics.is_empty() {
                return Err(format!("contexto {c:?} vazio"));
            }
            if let Some(x) = ics.iter().find(|i| !validos.contains(&i.as_str())) {
                return Err(format!("comando {x:?} não existe nesta versão"));
            }
        }
        let n = v.len();
        for (c, ics) in v {
            self.definir(&c, ics);
        }
        Ok(n)
    }
}

/// Páginas de oito: os dois primeiros setores da primeira página são os próximos mais prováveis.
pub fn paginas(icones: &[String], provaveis: &[String]) -> Vec<Vec<String>> {
    let mut lista: Vec<String> = vec![];
    for p in provaveis.iter().take(2) {
        if !lista.contains(p) {
            lista.push(p.clone());
        }
    }
    for i in icones {
        if !lista.contains(i) {
            lista.push(i.clone());
        }
    }
    lista.chunks(SETORES).map(<[String]>::to_vec).collect()
}

/// Item da barra pelo ícone.
pub fn item(icone: &str) -> Option<Item> {
    crate::ajuda_cmd::catalogo().iter().find(|i| i.icon == icone).copied()
}

/// Ícone da barra de uma ação.
pub fn icone_da_acao(a: Act) -> Option<&'static str> {
    crate::ajuda_cmd::catalogo().iter().find(|i| i.act == a).map(|i| i.icon)
}

// ------------------------------------------------------------------------- uso aprendido

/// Transições "comando A → comando B" contadas no uso local, com decaimento semanal.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Uso {
    pub pares: Vec<(String, String, f32)>,
    pub ultimo: Option<String>,
    /// semana (dias desde 1970 / 7) da última atualização do decaimento
    pub semana: u64,
}

/// Padrão inicial, do arquivo do projeto: o que costuma vir depois de quê.
const TRANSICOES_INICIAIS: &str = include_str!("../../../assets/roda_transicoes.json");

impl Uso {
    /// Cada semana que passou divide as contagens por dois; some o que ficou abaixo de 0,05.
    pub fn decair(&mut self, semana_atual: u64) {
        if self.semana == 0 {
            self.semana = semana_atual;
            return;
        }
        let dif = semana_atual.saturating_sub(self.semana);
        if dif > 0 {
            let f = 0.5f32.powi(dif.min(30) as i32);
            for p in &mut self.pares {
                p.2 *= f;
            }
            self.pares.retain(|p| p.2 >= 0.05);
            self.semana = semana_atual;
        }
    }

    /// Anota que `cmd` foi executado (e a transição a partir do anterior).
    pub fn registrar(&mut self, cmd: &str) {
        if let Some(a) = self.ultimo.clone() {
            if a != cmd {
                match self.pares.iter_mut().find(|p| p.0 == a && p.1 == cmd) {
                    Some(p) => p.2 += 1.,
                    None => self.pares.push((a, cmd.to_owned(), 1.)),
                }
            }
        }
        self.ultimo = Some(cmd.to_owned());
    }

    /// Os `n` comandos mais prováveis depois do último executado: o uso local vale mais que o padrão.
    pub fn provaveis(&self, n: usize) -> Vec<String> {
        let Some(a) = self.ultimo.as_deref() else { return vec![] };
        let mut pontos: Vec<(String, f32)> = vec![];
        let mut soma = |b: &str, p: f32| match pontos.iter_mut().find(|x| x.0 == b) {
            Some(x) => x.1 += p,
            None => pontos.push((b.to_owned(), p)),
        };
        for (x, b, c) in transicoes_iniciais() {
            if x == a {
                soma(&b, c * 0.25);
            }
        }
        for (x, b, c) in &self.pares {
            if x == a {
                soma(b, *c);
            }
        }
        pontos.sort_by(|p, q| q.1.total_cmp(&p.1).then_with(|| p.0.cmp(&q.0)));
        pontos.into_iter().take(n).map(|p| p.0).collect()
    }
}

fn transicoes_iniciais() -> Vec<(String, String, f32)> {
    serde_json::from_str(TRANSICOES_INICIAIS).unwrap_or_default()
}

#[cfg(test)]
mod testes {
    use super::*;

    fn rodar(r: &mut Roda, eventos: &[Evento], paginas: usize, ocupados: usize) -> Vec<Saida> {
        eventos.iter().map(|e| r.passo(*e, paginas, ocupados)).collect()
    }

    #[test]
    fn setores_seguem_a_direcao_do_norte_no_sentido_horario() {
        assert_eq!(setor_do_deslocamento(0., -70.), Some(0));
        assert_eq!(setor_do_deslocamento(70., 0.), Some(2));
        assert_eq!(setor_do_deslocamento(0., 70.), Some(4));
        assert_eq!(setor_do_deslocamento(-70., 0.), Some(6));
        // a diagonal cai no setor 1; o limite de 22,5 graus separa os vizinhos
        assert_eq!(setor_do_deslocamento(50., -50.), Some(1));
        assert_eq!(setor_do_deslocamento(10., -70.), Some(0));
        assert_eq!(setor_do_deslocamento(40., -70.), Some(1));
        // zona morta
        assert_eq!(setor_do_deslocamento(3., 3.), None);
        // os oito setores aparecem todos ao dar a volta
        let vistos: std::collections::BTreeSet<usize> = (0..360).filter_map(|g| setor_do_deslocamento((g as f32).to_radians().sin() * 80., -(g as f32).to_radians().cos() * 80.)).collect();
        assert_eq!(vistos.len(), SETORES);
    }

    #[test]
    fn gesto_de_60_px_executa_sem_desenhar_a_roda() {
        let mut r = Roda::default();
        assert_eq!(r.passo(Evento::Tecla { t: 10.0, pos: [100., 100.] }, 1, 8), Saida::Nada);
        // a roda ainda não foi desenhada aos 50 ms, e o gesto já termina
        assert!(!r.desenhada(10.05));
        let s = r.passo(Evento::Solta { t: 10.05, pos: [100., 40.] }, 1, 8);
        assert_eq!(s, Saida::Executar(0));
        assert!(!r.aberta());
    }

    #[test]
    fn a_roda_aparece_em_ate_80_ms_e_segurar_e_soltar_executa() {
        let mut r = Roda::default();
        r.passo(Evento::Tecla { t: 5.0, pos: [200., 200.] }, 1, 8);
        assert!(!r.desenhada(5.079));
        assert!(r.desenhada(5.080));
        // soltou depois de 1 s apontando para a direita (setor 2)
        assert_eq!(r.passo(Evento::Solta { t: 6.0, pos: [260., 202.] }, 1, 8), Saida::Executar(2));
    }

    #[test]
    fn toque_curto_fixa_e_o_clique_executa_ou_cancela() {
        let mut r = Roda::default();
        r.passo(Evento::Tecla { t: 1.0, pos: [50., 50.] }, 1, 8);
        assert_eq!(r.passo(Evento::Solta { t: 1.1, pos: [51., 50.] }, 1, 8), Saida::Nada);
        assert_eq!(r.fase, Fase::Fixa { p0: [50., 50.] });
        assert!(r.desenhada(1.1));
        assert_eq!(r.passo(Evento::Clique { pos: [50., 120.] }, 1, 8), Saida::Executar(4));
        // fixa de novo e clica na zona morta: cancela
        r.passo(Evento::Tecla { t: 2.0, pos: [50., 50.] }, 1, 8);
        r.passo(Evento::Solta { t: 2.05, pos: [50., 50.] }, 1, 8);
        assert_eq!(r.passo(Evento::Clique { pos: [52., 51.] }, 1, 8), Saida::Cancelou);
        // Esc cancela a fixa
        r.passo(Evento::Tecla { t: 3.0, pos: [50., 50.] }, 1, 8);
        r.passo(Evento::Solta { t: 3.05, pos: [50., 50.] }, 1, 8);
        assert_eq!(r.passo(Evento::Esc, 1, 8), Saida::Cancelou);
        assert!(!r.aberta());
    }

    #[test]
    fn segurar_e_soltar_no_centro_desiste() {
        let mut r = Roda::default();
        r.passo(Evento::Tecla { t: 1.0, pos: [0., 0.] }, 1, 8);
        assert_eq!(r.passo(Evento::Solta { t: 2.0, pos: [3., 2.] }, 1, 8), Saida::Cancelou);
    }

    #[test]
    fn setor_vazio_nao_executa_e_numeros_escolhem_direto() {
        let mut r = Roda::default();
        r.passo(Evento::Tecla { t: 1.0, pos: [0., 0.] }, 1, 3);
        // setor 4 (embaixo) está vazio: só 3 ocupados
        assert_eq!(r.passo(Evento::Solta { t: 1.5, pos: [0., 90.] }, 1, 3), Saida::Cancelou);
        r.passo(Evento::Tecla { t: 2.0, pos: [0., 0.] }, 1, 3);
        assert_eq!(r.passo(Evento::Numero(3), 1, 3), Saida::Executar(2));
        r.passo(Evento::Tecla { t: 3.0, pos: [0., 0.] }, 1, 3);
        assert_eq!(r.passo(Evento::Numero(5), 1, 3), Saida::Cancelou);
    }

    #[test]
    fn a_roda_do_mouse_gira_as_paginas_em_laco() {
        let mut r = Roda::default();
        r.passo(Evento::Tecla { t: 1.0, pos: [0., 0.] }, 3, 8);
        let saidas = rodar(&mut r, &[Evento::Roda { passos: 1 }, Evento::Roda { passos: 1 }, Evento::Roda { passos: 1 }, Evento::Roda { passos: -1 }], 3, 8);
        assert!(saidas.iter().all(|s| *s == Saida::Nada));
        assert_eq!(r.pagina, 2);
        // a página volta a zero ao abrir de novo
        r.passo(Evento::Esc, 3, 8);
        r.passo(Evento::Tecla { t: 2.0, pos: [0., 0.] }, 3, 8);
        assert_eq!(r.pagina, 0);
    }

    #[test]
    fn contexto_procura_da_chave_especifica_para_a_geral() {
        let mut cfg = Config::default();
        let c = Contexto { bancada: Bench::Solidos, modo: Modo::Peca, selecao: Selecao::Face };
        assert_eq!(cfg.icones(&c)[0], "sketch");
        assert_eq!(cfg.icones(&c)[3], "casca");
        let sk = Contexto { bancada: Bench::Solidos, modo: Modo::Sketch, selecao: Selecao::Nada };
        assert_eq!(cfg.icones(&sk)[0], "sk_cota");
        cfg.definir("*:sketch:*", vec!["sk_aparar".into()]);
        assert_eq!(cfg.icones(&sk), ["sk_aparar"]);
        // a chave específica ganha da geral
        cfg.definir("solidos:sketch:nada", vec!["sk_offset".into()]);
        assert_eq!(cfg.icones(&sk), ["sk_offset"]);
        cfg.restaurar("solidos:sketch:nada");
        cfg.restaurar("*:sketch:*");
        assert_eq!(cfg.icones(&sk)[0], "sk_cota");
        // sem nada específico, qualquer peça cai no conjunto geral
        let outra = Contexto { bancada: Bench::Solidos, modo: Modo::Peca, selecao: Selecao::Nada };
        assert_eq!(cfg.icones(&outra)[0], "sketch");
    }

    #[test]
    fn todo_conteudo_de_fabrica_existe_na_barra() {
        let barra: Vec<&str> = crate::ajuda_cmd::comandos().iter().map(|i| i.icon).collect();
        let mut chaves: Vec<&str> = CONTEXTOS.iter().map(|c| c.0).collect();
        chaves.push("");
        for k in chaves {
            let ics = padrao(k);
            assert!(!ics.is_empty() && ics.len() <= 2 * SETORES, "{k}: tamanho");
            for i in ics {
                assert!(barra.contains(i), "{k}: {i} não está na barra de comandos");
            }
            let mut u = ics.to_vec();
            u.sort();
            u.dedup();
            assert_eq!(u.len(), ics.len(), "{k}: ícone repetido");
        }
        for (_, rotulo) in CONTEXTOS {
            assert!(crate::i18n::tem_traducao(rotulo), "{rotulo}");
        }
    }

    /// B372: cada bancada tem o conteúdo do próprio fluxo e a seleção troca o conteúdo.
    #[test]
    fn roda_segue_o_fluxo_da_bancada_e_a_selecao() {
        let cfg = Config::default();
        let ic = |b, m, s| cfg.icones(&Contexto { bancada: b, modo: m, selecao: s });
        // cada bancada abre com os comandos dela
        assert_eq!(ic(Bench::Superficies, Modo::Peca, Selecao::Nada)[0], "sup_extrudada");
        assert_eq!(ic(Bench::Estrutural, Modo::Peca, Selecao::Nada)[0], "st_membro");
        assert_eq!(ic(Bench::FormaLivre, Modo::Peca, Selecao::Nada)[0], "forma_caixa");
        assert_eq!(ic(Bench::Chapa, Modo::Peca, Selecao::Nada)[0], "sm_parede");
        // a seleção troca o conteúdo dentro da mesma bancada
        for b in [Bench::Solidos, Bench::Chapa] {
            let (f, a, v) = (ic(b, Modo::Peca, Selecao::Face), ic(b, Modo::Peca, Selecao::Aresta), ic(b, Modo::Peca, Selecao::Vertice));
            assert!(f != a && a != v && f != v, "{b:?}: face, aresta e vértice devem ter conteúdos próprios");
        }
        assert_ne!(ic(Bench::Superficies, Modo::Peca, Selecao::Superficie), ic(Bench::Superficies, Modo::Peca, Selecao::Nada));
        assert!(ic(Bench::Estrutural, Modo::Peca, Selecao::Vertice).contains(&"st_placa".to_string()));
        // na montagem, face e aresta oferecem restrições diferentes
        let (mf, ma) = (ic(Bench::Montagem, Modo::Montagem, Selecao::Face), ic(Bench::Montagem, Modo::Montagem, Selecao::Aresta));
        assert!(mf.contains(&"rest_contato".to_string()) && !ma.contains(&"rest_contato".to_string()));
        // sem conteúdo específico, a roda cai no geral, nunca vazia nem maior que uma página
        for b in [Bench::Solidos, Bench::Superficies, Bench::Montagem, Bench::Chapa, Bench::Estrutural, Bench::Desenho, Bench::FormaLivre] {
            for m in [Modo::Peca, Modo::Sketch, Modo::Montagem, Modo::Desenho] {
                for s in [Selecao::Nada, Selecao::Face, Selecao::Aresta, Selecao::Vertice, Selecao::Superficie, Selecao::Outra] {
                    let v = ic(b, m, s);
                    assert!(!v.is_empty() && v.len() <= SETORES, "{b:?}/{m:?}/{s:?}: {v:?}");
                }
            }
        }
    }

    #[test]
    fn paginas_de_oito_com_os_provaveis_na_frente_sem_repetir() {
        let ics: Vec<String> = (0..10).map(|i| format!("c{i}")).collect();
        let p = paginas(&ics, &["c7".into(), "x".into(), "c9".into()]);
        assert_eq!(p.len(), 2);
        assert_eq!(&p[0][..3], ["c7", "x", "c0"]);
        assert_eq!(p[0].len(), 8);
        // só os dois primeiros prováveis entram; c9 aparece na ordem normal
        assert_eq!(p.iter().flatten().filter(|s| *s == "c9").count(), 1);
        assert_eq!(p.iter().flatten().filter(|s| *s == "c7").count(), 1);
        assert_eq!(p.iter().map(Vec::len).sum::<usize>(), 11);
    }

    #[test]
    fn uso_aprende_decai_e_o_padrao_inicial_so_desempata() {
        let mut u = Uso::default();
        assert!(u.provaveis(2).is_empty());
        u.registrar("sketch");
        // sem uso próprio, vale o padrão inicial do projeto
        assert_eq!(u.provaveis(1), ["extrusao"]);
        for _ in 0..3 {
            u.registrar("sketch");
            u.registrar("revolucao");
            u.registrar("sketch");
        }
        // o uso local passa o padrão
        u.ultimo = Some("sketch".into());
        assert_eq!(u.provaveis(1), ["revolucao"]);
        // o decaimento semanal divide ao meio e depois apaga o que sumiu
        u.semana = 100;
        let antes = u.pares.iter().find(|p| p.1 == "revolucao").unwrap().2;
        u.decair(101);
        let depois = u.pares.iter().find(|p| p.1 == "revolucao").unwrap().2;
        assert!((depois - antes / 2.).abs() < 1e-6);
        u.decair(140);
        assert!(u.pares.is_empty());
    }

    #[test]
    fn exportar_e_importar_viram_ida_e_volta_e_recusam_comando_desconhecido() {
        let mut a = Config::default();
        a.definir("*:sketch:*", vec!["sk_cota".into(), "sk_offset".into()]);
        let texto = a.exportar();
        let barra: Vec<&str> = crate::ajuda_cmd::comandos().iter().map(|i| i.icon).collect();
        let mut b = Config::default();
        assert_eq!(b.importar(&texto, &barra), Ok(1));
        assert_eq!(b.slots, a.slots);
        assert!(b.importar("[[\"x\",[\"nao_existe\"]]]", &barra).unwrap_err().contains("não existe"));
        assert!(b.importar("lixo", &barra).is_err());
        assert!(b.importar("[[\"\",[\"sk_cota\"]]]", &barra).is_err());
    }

    #[test]
    fn tecla_da_roda_esta_no_mapa_sem_conflito_em_todas_as_predefinicoes() {
        use crate::keymap::{KeyAction, Keymap, Preset};
        for p in Preset::ALL {
            let km = Keymap { preset: p, custom: vec![] };
            assert_eq!(km.binding(KeyAction::RodaComandos).map(|b| b.text()).as_deref(), Some("Shift+W"), "{p:?}");
            assert!(km.conflicts(KeyAction::RodaComandos).is_empty(), "{p:?}: {:?}", km.conflicts(KeyAction::RodaComandos));
        }
    }
}
