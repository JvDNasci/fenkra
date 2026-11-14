// SPDX-License-Identifier: LGPL-2.1-or-later
//! Comando Nota (B448): clique simples cria a nota numa face, aresta ou vértice; com Ctrl ou Shift
//! os cliques somam alvos e a nota só nasce com Enter ou OK, com uma seta para cada alvo. O painel
//! do comando mostra os campos da nota (texto e alvos), e não as propriedades da seleção anterior.
//!
//! Causa dos defeitos do teste com mouse real: o comando Nota passava pelo caminho geral das
//! anotações, que cria assim que há elementos bastantes (`PmiKind::anchors()` vale 1 para a nota),
//! sem olhar Ctrl e Shift; e o painel de comando, com uma anotação pendente, caía no editor da
//! seleção (`editor::show`), que mostrava o último item escolhido na árvore.

use crate::app::App;
use crate::i18n::tr;
use eframe::egui::{self, RichText};
use fenkra_core::pmi::PmiKind;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

/// Índice do comando Nota em `pmi_ui::kind_of`.
pub const NOTA: u8 = 4;

/// Já houve um clique com Ctrl ou Shift: daqui em diante só Enter ou OK criam.
static ACUMULANDO: AtomicBool = AtomicBool::new(false);
/// Texto escrito no painel antes de a nota existir.
static TEXTO: Mutex<String> = Mutex::new(String::new());

/// Comando aberto: tudo começa vazio.
pub fn comecar() {
    ACUMULANDO.store(false, Ordering::Relaxed);
    TEXTO.lock().unwrap_or_else(|e| e.into_inner()).clear();
}

pub fn acumulando() -> bool {
    ACUMULANDO.load(Ordering::Relaxed)
}

/// Depois de um clique no 3D com o comando aberto: clique simples cria; com Ctrl ou Shift (ou depois
/// de um deles) o alvo só se soma aos outros.
pub fn depois_do_clique(app: &mut App) {
    if app.picks.is_empty() {
        return;
    }
    if crate::pmi_setas::modificador() || acumulando() {
        ACUMULANDO.store(true, Ordering::Relaxed);
        return;
    }
    criar(app);
}

/// Cria a nota com uma seta para cada elemento escolhido e o texto do painel.
pub fn criar(app: &mut App) {
    let pts = crate::pmi_ui::pontas_das_escolhas(app);
    if pts.is_empty() {
        app.toast(tr("Escolha ao menos uma face, aresta ou vértice").to_string(), false);
        return;
    }
    let picks = app.picks.clone();
    let (a, t): (Vec<_>, Vec<_>) = pts.into_iter().unzip();
    crate::pmi_ui::create(app, PmiKind::Note, a, t, &picks);
    let texto = std::mem::take(&mut *TEXTO.lock().unwrap_or_else(|e| e.into_inner()));
    if let Some(p) = crate::pmi_ui::list_mut(app).last_mut() {
        fenkra_core::pmi_leader::fit(p);
        if !texto.trim().is_empty() {
            p.text = texto;
        }
    }
    ACUMULANDO.store(false, Ordering::Relaxed);
}

/// Corpo do painel de comando com uma anotação pendente. Para a nota: texto, alvos e OK; para as
/// outras, o passo do comando. Nunca as propriedades da seleção anterior.
pub fn painel(app: &mut App, ui: &mut egui::Ui) {
    let Some(i) = app.pmi_pending else { return };
    let kind = crate::pmi_ui::kind_of(i);
    if i != NOTA {
        ui.label(RichText::new(tr(kind.label())).strong());
        ui.label(format!("{}: {}", tr("Elementos escolhidos"), app.picks.len())).on_hover_text(tr("O texto junto ao cursor mostra o passo do comando e o que serve para esta anotação."));
        return;
    }
    let mut texto = TEXTO.lock().unwrap_or_else(|e| e.into_inner()).clone();
    ui.label(tr("Texto da nota")).on_hover_text(tr("O que a nota diz. Pode ficar vazio e ser escrito depois, sobre a própria nota."));
    ui.add(egui::TextEdit::multiline(&mut texto).desired_rows(2).desired_width(f32::INFINITY).hint_text(tr("ex.: Quebrar cantos")))
        .on_hover_text(tr("O que a nota diz. Pode ficar vazio e ser escrito depois, sobre a própria nota."));
    *TEXTO.lock().unwrap_or_else(|e| e.into_inner()) = texto;
    ui.add_space(4.);
    let n = app.picks.len();
    ui.label(RichText::new(format!("{} ({n})", tr("Alvos"))).strong()).on_hover_text(tr("Cada alvo vira uma seta da nota."));
    if n == 0 {
        ui.label(RichText::new(tr("clique uma face, aresta ou vértice no 3D")).weak());
    }
    let nomes: Vec<String> = app
        .picks
        .iter()
        .map(|p| app.result.as_ref().and_then(|r| crate::pmi_ui::pick_target(r, p)).map_or_else(|| p.label(&app.open.doc), |t| crate::pmi_ui::target_label(&t)))
        .collect();
    for (k, nome) in nomes.iter().enumerate() {
        ui.label(format!("{}. {nome}", k + 1));
    }
    ui.label(RichText::new(tr("Clique simples cria a nota com uma seta. Com Ctrl ou Shift os cliques somam alvos, uma seta por alvo, e a nota nasce com Enter ou OK.")).small().weak());
    if ui.add_enabled(n > 0, egui::Button::new(tr("✔ OK"))).on_hover_text(tr("Cria a nota com uma seta para cada alvo (Enter)")).clicked() {
        criar(app);
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::picking::Pick;
    use fenkra_solidos::{RecomputeResult, Status};

    /// Trava dos testes que usam o estado global do comando e das setas.
    pub(crate) static SERIE: Mutex<()> = Mutex::new(());

    /// Placa paramétrica recalculada dentro de um App de teste, sem janela.
    pub(crate) fn app_com_placa() -> App {
        let mut app = App::new(&eframe::CreationContext::_new_kittest(egui::Context::default()));
        let e = crate::examples::all().into_iter().find(|e| e.name == "Placa paramétrica").expect("exemplo");
        app.load_doc_pub(e.doc, None);
        app.result = Some(fenkra_solidos::recompute(&app.open.doc));
        app.picks.clear();
        app
    }

    /// Status sem erro, volume e caixa envolvente do resultado.
    pub(crate) fn conferir(r: &RecomputeResult) -> (f64, [f32; 3], [f32; 3]) {
        assert!(r.status.values().all(|s| !matches!(s, Status::Error(_))), "a peça recalcula sem erro");
        let vol: f64 = r.bodies.iter().map(|b| b.mass.volume).sum();
        let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
        for b in &r.bodies {
            for v in b.mesh.positions.chunks_exact(3) {
                for k in 0..3 {
                    lo[k] = lo[k].min(v[k]);
                    hi[k] = hi[k].max(v[k]);
                }
            }
        }
        (vol, lo, hi)
    }

    /// Faces distintas da placa, como o clique do 3D as entrega (ponto no meio de um triângulo).
    pub(crate) fn faces(app: &App, n: usize) -> Vec<Pick> {
        let r = app.result.as_ref().unwrap();
        let b = &r.bodies[0];
        (0..b.mesh.face_ranges.len())
            .take(n)
            .map(|face| {
                let ini = b.mesh.face_ranges[face] as usize;
                let t = &b.mesh.indices[ini..ini + 3];
                let v = |i: u32| [0, 1, 2].map(|k| b.mesh.positions[i as usize * 3 + k]);
                let (p0, p1, p2) = (v(t[0]), v(t[1]), v(t[2]));
                let point = [0, 1, 2].map(|k| (p0[k] + p1[k] + p2[k]) / 3.);
                Pick::Face { body: 0, face, owner: b.collector, point, normal: [0.; 3] }
            })
            .collect()
    }

    #[test]
    fn ctrl_acumula_alvos_e_enter_cria_uma_seta_por_alvo() {
        let _g = SERIE.lock().unwrap_or_else(|e| e.into_inner());
        let mut app = app_com_placa();
        let (vol, lo, hi) = conferir(app.result.as_ref().unwrap());
        assert!(vol > 0.);
        assert!((0..3).all(|k| hi[k] > lo[k]), "a caixa da placa tem as três medidas");
        let antes = app.open.doc.pmi.len();
        crate::pmi_ui::command(&mut app, NOTA);
        assert_eq!(app.pmi_pending, Some(NOTA), "sem seleção o comando espera os alvos");
        let alvos = faces(&app, 3);
        assert_eq!(alvos.len(), 3);
        crate::pmi_setas::fingir_modificador(true);
        for (k, p) in alvos.iter().enumerate() {
            app.picks.push(*p);
            crate::pmi_ui::after_viewport(&mut app);
            assert_eq!(app.open.doc.pmi.len(), antes, "com Ctrl o clique {} só soma o alvo", k + 1);
            assert_eq!(app.pmi_pending, Some(NOTA));
        }
        // soltou o Ctrl: ainda espera o Enter (um quadro sem clique não cria nada)
        crate::pmi_setas::fingir_modificador(false);
        crate::pmi_ui::after_viewport(&mut app);
        assert_eq!(app.open.doc.pmi.len(), antes, "só Enter ou OK criam depois de somar alvos");
        *TEXTO.lock().unwrap() = "Quebrar cantos".into();
        // Enter (pending_preview → finish_multi) e o OK do painel chegam aqui
        criar(&mut app);
        assert_eq!(app.open.doc.pmi.len(), antes + 1, "uma nota só");
        let nota = app.open.doc.pmi.last().unwrap();
        assert_eq!(nota.kind, PmiKind::Note);
        assert_eq!(nota.anchors.len(), 3, "uma seta por alvo");
        assert_eq!(nota.targets.iter().filter(|t| t.is_some()).count(), 3, "cada seta guarda a sua face");
        assert_eq!(nota.style.leader_via.len(), 3);
        assert_eq!(nota.text, "Quebrar cantos");
        assert_eq!(app.pmi_pending, None, "o comando fecha");
        assert!(app.picks.is_empty());
        assert!(!acumulando());
        // a anotação não mexe na geometria
        let (vol2, lo2, hi2) = conferir(&fenkra_solidos::recompute(&app.open.doc));
        assert!((vol2 - vol).abs() < 1e-6);
        assert_eq!((lo2, hi2), (lo, hi));
    }

    #[test]
    fn clique_simples_cria_a_nota_na_hora_e_o_comando_comeca_limpo() {
        let _g = SERIE.lock().unwrap_or_else(|e| e.into_inner());
        let mut app = app_com_placa();
        let antes = app.open.doc.pmi.len();
        *TEXTO.lock().unwrap() = "sobra de outro comando".into();
        crate::pmi_ui::command(&mut app, NOTA);
        assert!(TEXTO.lock().unwrap().is_empty(), "o texto começa vazio");
        app.picks.push(faces(&app, 1)[0]);
        crate::pmi_setas::fingir_modificador(false);
        crate::pmi_ui::after_viewport(&mut app);
        assert_eq!(app.open.doc.pmi.len(), antes + 1);
        assert_eq!(app.open.doc.pmi.last().unwrap().anchors.len(), 1);
        assert_eq!(app.pmi_pending, None);
    }
}
