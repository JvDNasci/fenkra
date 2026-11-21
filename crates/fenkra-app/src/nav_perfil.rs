// SPDX-License-Identifier: LGPL-2.1-or-later
//! Perfis de controle do mouse (B440): qual botão, com qual modificador, gira, desloca ou dá zoom.
//!
//! O perfil só escolhe a ação. A velocidade do giro, do deslocamento e do zoom, o pivô sob o cursor e
//! as prévias continuam os mesmos em todos os perfis, porque a vista é calculada depois, num lugar só
//! (`App::viewport`). Por isso trocar de perfil não muda a sensação da movimentação 3D, só os botões.
//!
//! Como o egui informa o arrasto: `dragged_by(b)` vale para todo botão que está apertado durante o
//! arrasto. Com o meio e o esquerdo juntos, os dois aparecem arrastando; `esq_preso`/`dir_preso`
//! dizem se o outro botão está apertado. O gesto aqui guarda exatamente essas leituras.
//!
//! A precedência é a do código antigo: deslocar vence o zoom, que vence o giro. Ctrl + arrastar com
//! o esquerdo, fora do sketch, é seleção em caixa e não chega à câmera.

use crate::i18n::tr;
use crate::settings::NavStyle;
use eframe::egui;

/// O que um arrasto faz com a vista.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Acao {
    Nada,
    Girar,
    Mover,
    Zoom,
    /// Ctrl + esquerdo: seleção em caixa (só na tabela; a câmera não recebe)
    Caixa,
}

impl Acao {
    pub fn nome(self) -> &'static str {
        match self {
            Acao::Nada => "—",
            Acao::Girar => tr("girar"),
            Acao::Mover => tr("deslocar"),
            Acao::Zoom => tr("zoom"),
            Acao::Caixa => tr("seleção em caixa"),
        }
    }
}

/// Leitura do mouse num quadro.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct Gesto {
    /// arrastando com o esquerdo (já sem a seleção em caixa)
    pub esq: bool,
    pub meio: bool,
    pub dir: bool,
    /// botão apertado agora, mesmo sem ser o que começou o arrasto
    pub esq_preso: bool,
    pub dir_preso: bool,
    pub shift: bool,
    pub ctrl: bool,
    /// editando um sketch: no perfil padrão o esquerdo fica com a ferramenta
    pub sketch: bool,
}

/// (girar, deslocar, zoom arrastando) do perfil, antes da precedência.
fn bruto(n: NavStyle, g: Gesto) -> (bool, bool, bool) {
    let Gesto { esq: left, meio: mid, dir: right, esq_preso: left_down, dir_preso: right_down, shift, ctrl, sketch } = g;
    use NavStyle as N;
    match n {
        // 21/11: o direito fica só com o menu; deslocar é Shift + arrastar (esquerdo ou meio)
        N::Fenkra => ((left && !shift && !sketch) || (mid && !shift), shift && (left || mid), false),
        // Shift+meio dá zoom, como no perfil de botão do meio mais comum entre os CADs paramétricos
        N::MeioGira => (mid && !ctrl, mid && ctrl, mid && shift && !ctrl),
        N::ShiftMeioGira => (mid && shift, mid && !shift, false),
        N::MeioMove => (mid && right_down, mid && !right_down, false),
        N::Blender => (mid && !shift && !ctrl, mid && shift, mid && ctrl),
        N::DireitoGira => (right && !ctrl, (right && ctrl) || mid, false),
        N::FreeCad => (mid && (left_down || right_down), mid && !left_down && !right_down, false),
        N::DireitoGiraShift => (right && !shift && !ctrl, right && shift, right && ctrl),
    }
}

/// Ação da câmera para o gesto, no perfil escolhido.
pub fn acao(n: NavStyle, g: Gesto) -> Acao {
    match bruto(n, g) {
        (_, true, _) => Acao::Mover,
        (_, _, true) => Acao::Zoom,
        (true, _, _) => Acao::Girar,
        _ => Acao::Nada,
    }
}

/// A roda gira ao contrário? O perfil de botão do meio com Ctrl nasce invertido; a opção da pessoa
/// troca o sentido em qualquer perfil.
pub fn roda_invertida(n: NavStyle, opcao: bool) -> bool {
    opcao != (n == NavStyle::MeioGira)
}

/// Combinações mostradas na tabela das Opções, na ordem: rótulo e gesto (fora do sketch).
pub fn combinacoes() -> Vec<(&'static str, Gesto)> {
    let e = Gesto { esq: true, esq_preso: true, ..Default::default() };
    let m = Gesto { meio: true, ..Default::default() };
    let d = Gesto { dir: true, dir_preso: true, ..Default::default() };
    let s = |g: Gesto| Gesto { shift: true, ..g };
    let c = |g: Gesto| Gesto { ctrl: true, ..g };
    vec![
        ("Esquerdo", e),
        ("Shift + esquerdo", s(e)),
        ("Ctrl + esquerdo", c(e)),
        ("Meio", m),
        ("Shift + meio", s(m)),
        ("Ctrl + meio", c(m)),
        ("Direito", d),
        ("Shift + direito", s(d)),
        ("Ctrl + direito", c(d)),
        // com dois botões apertados o egui dá os dois como arrastando
        ("Meio + esquerdo", Gesto { esq: true, esq_preso: true, meio: true, ..Default::default() }),
        ("Meio + direito", Gesto { dir: true, dir_preso: true, meio: true, ..Default::default() }),
    ]
}

/// Ação mostrada na tabela: Ctrl + esquerdo é a seleção em caixa, como em `App::viewport`.
pub fn acao_na_tabela(n: NavStyle, g: Gesto) -> Acao {
    if g.esq && g.ctrl && !g.meio && !g.sketch {
        return Acao::Caixa;
    }
    acao(n, g)
}

/// Tabela do perfil nas Opções: cada combinação e o que ela faz, tirada da mesma função que move a
/// câmera, para a tabela nunca mentir.
pub fn tabela_ui(ui: &mut egui::Ui, n: NavStyle, invert_zoom: bool) {
    egui::Grid::new("nav_perfil_tabela").num_columns(2).spacing([16., 2.]).striped(true).show(ui, |ui| {
        ui.label(egui::RichText::new(tr("Botão")).small().strong());
        ui.label(egui::RichText::new(tr("Ação")).small().strong());
        ui.end_row();
        for (rotulo, g) in combinacoes() {
            let a = acao_na_tabela(n, g);
            if a == Acao::Nada {
                continue;
            }
            ui.label(egui::RichText::new(tr(rotulo)).small());
            ui.label(egui::RichText::new(a.nome()).small());
            ui.end_row();
        }
        ui.label(egui::RichText::new(tr("Roda")).small());
        let sentido = if roda_invertida(n, invert_zoom) { tr("zoom (invertido)") } else { tr("zoom") };
        ui.label(egui::RichText::new(sentido).small());
        ui.end_row();
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use NavStyle as N;

    /// Tabela escrita à mão, uma letra por combinação de `combinacoes()`:
    /// G girar, M deslocar, Z zoom, C caixa, - nada.
    const TABELA: &[(NavStyle, &str)] = &[
        (N::Fenkra, "GMCGMG---GG"),
        (N::MeioGira, "--CGZM---GG"),
        (N::ShiftMeioGira, "--CMGM---MM"),
        (N::MeioMove, "--CMMM---MG"),
        (N::Blender, "--CGMZ---GG"),
        (N::DireitoGira, "--CMMMGGMMM"),
        (N::FreeCad, "--CMMM---GG"),
        (N::DireitoGiraShift, "--C---GMZ-G"),
    ];

    fn letra(a: Acao) -> char {
        match a {
            Acao::Nada => '-',
            Acao::Girar => 'G',
            Acao::Mover => 'M',
            Acao::Zoom => 'Z',
            Acao::Caixa => 'C',
        }
    }

    #[test]
    fn cada_perfil_leva_botao_e_modificador_para_a_acao_certa() {
        assert_eq!(TABELA.len(), NavStyle::ALL.len(), "perfil sem tabela");
        for n in NavStyle::ALL {
            let esperado = TABELA.iter().find(|(x, _)| *x == n).map(|(_, t)| *t).expect("perfil sem tabela");
            let obtido: String = combinacoes().into_iter().map(|(_, g)| letra(acao_na_tabela(n, g))).collect();
            assert_eq!(obtido, esperado, "{n:?}");
        }
    }

    #[test]
    fn todo_perfil_gira_e_desloca_com_algum_botao() {
        for n in NavStyle::ALL {
            let acoes: Vec<Acao> = combinacoes().into_iter().map(|(_, g)| acao(n, g)).collect();
            assert!(acoes.contains(&Acao::Girar) && acoes.contains(&Acao::Mover), "{n:?}");
        }
    }

    /// O padrão continua o perfil Fenkra, e nele o esquerdo não gira a vista dentro do sketch (fica
    /// com a ferramenta), o meio gira, Shift + arrastar desloca e o direito não move a vista (é o menu).
    #[test]
    fn padrao_nao_muda() {
        assert_eq!(crate::settings::Settings::default().nav, N::Fenkra);
        let e = Gesto { esq: true, esq_preso: true, sketch: true, ..Default::default() };
        assert_eq!(acao(N::Fenkra, e), Acao::Nada);
        assert_eq!(acao(N::Fenkra, Gesto { shift: true, ..e }), Acao::Mover);
        assert_eq!(acao(N::Fenkra, Gesto { meio: true, sketch: true, ..Default::default() }), Acao::Girar);
        assert_eq!(acao(N::Fenkra, Gesto { dir: true, dir_preso: true, sketch: true, ..Default::default() }), Acao::Nada);
        assert_eq!(acao(N::Fenkra, Gesto { dir: true, dir_preso: true, ..Default::default() }), Acao::Nada);
        assert_eq!(acao(N::Fenkra, Gesto { meio: true, shift: true, ..Default::default() }), Acao::Mover);
        assert!(!roda_invertida(N::Fenkra, false));
        assert!(roda_invertida(N::Fenkra, true));
        assert!(roda_invertida(N::MeioGira, false));
        // sem botão, nada se move em nenhum perfil
        for n in NavStyle::ALL {
            assert_eq!(acao(n, Gesto { shift: true, ctrl: true, ..Default::default() }), Acao::Nada);
        }
    }

    #[test]
    fn textos_da_tabela_tem_traducao() {
        for (r, _) in combinacoes() {
            assert!(crate::i18n::tem_traducao(r), "{r}");
        }
        for t in ["Botão", "Ação", "Roda", "girar", "deslocar", "zoom", "zoom (invertido)", "seleção em caixa"] {
            assert!(crate::i18n::tem_traducao(t), "{t}");
        }
    }
}
