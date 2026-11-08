// SPDX-License-Identifier: LGPL-2.1-or-later
//! Tutoriais por peça (B376): organizados pelo objetivo ("quero uma peça assim"), cada um com a peça
//! pronta para abrir e o passo a passo dos comandos que a fazem. Cada passo leva à aula do comando
//! no curso (B323), onde o comando é explicado com calma; o de fixadores leva às regras dos fixadores
//! inteligentes.
//!
//! Um tutorial é uma `Aula` do curso com mais um campo: a aula de cada passo. Assim o progresso, o
//! pular e o retomar e o reconhecimento pelo documento são os mesmos do curso, sem código novo. A
//! peça pronta cumpre todos os passos que deixam rastro no documento (um teste confere), então
//! abrir o exemplo e percorrer a árvore já mostra o tutorial inteiro como feito.

use crate::curso::{Aula, Cond, Nivel, Passo};
use crate::settings::Bench;

pub struct Tutorial {
    /// o que a pessoa quer desenhar, na voz dela
    pub objetivo: &'static str,
    /// passos (texto, ícone e reconhecimento) e a peça pronta
    pub aula: Aula,
    /// aula do curso de cada passo, na mesma ordem
    pub licoes: &'static [&'static str],
}

const fn p(texto: &'static str, icone: &'static str, cond: Cond) -> Passo {
    Passo { texto, icone, cond }
}

#[allow(clippy::too_many_arguments)]
const fn t(id: &'static str, objetivo: &'static str, titulo: &'static str, resumo: &'static str, bancada: Bench, nivel: Nivel, exemplo: &'static str, passos: &'static [Passo], licoes: &'static [&'static str]) -> Tutorial {
    Tutorial { objetivo, aula: Aula { id, titulo, resumo, bancada, nivel, exemplo: Some(exemplo), midia: None, passos }, licoes }
}

use Bench::*;
use Cond::*;
use Nivel::*;

pub const TUTORIAIS: &[Tutorial] = &[
    t(
        "tut_flange",
        "Quero um flange com furos em círculo",
        "Flange",
        "Disco com cubo central e furação em círculo, toda guiada pelo diâmetro de furação.",
        Solidos,
        Iniciante,
        "Flange",
        &[
            p("Desenhe o disco num sketch, com os círculos da furação dentro dele.", "sketch", Feature("sketch", 1)),
            p("Extrude o disco: os círculos de dentro viram furos passantes.", "extrusao", Feature("extrusao", 1)),
            p("Desenhe o cubo central num sketch sobre a face de cima.", "sketch", Feature("sketch", 2)),
            p("Extrude o cubo por cima do disco.", "extrusao", Feature("extrusao", 2)),
        ],
        &["sol_esboco", "sol_volume", "sol_esboco", "sol_volume"],
    ),
    t(
        "tut_placa",
        "Quero uma placa com furos e rasgo que mudam por parâmetro",
        "Placa paramétrica",
        "Placa com furos, rasgo, rebaixos e filetes; mudar um parâmetro refaz tudo no lugar.",
        Solidos,
        Intermediario,
        "Placa paramétrica",
        &[
            p("Desenhe o contorno da placa e cote com nomes de parâmetro.", "sketch", CotaSketch),
            p("Extrude a placa.", "extrusao", Feature("extrusao", 1)),
            p("Fure a placa nos pontos marcados.", "furo_simples", Feature("furo", 1)),
            p("Arredonde os cantos com filete.", "arestas_selecionar", Feature("filete", 1)),
        ],
        &["sol_esboco", "sol_volume", "sol_furo", "sol_acabamento"],
    ),
    t(
        "tut_caixa",
        "Quero uma caixa oca de parede fina",
        "Caixa com casca",
        "Perfil oblongo extrudado e esvaziado: a casca deixa a parede com a mesma espessura.",
        Solidos,
        Intermediario,
        "Caixa com casca",
        &[
            p("Desenhe o perfil oblongo num sketch.", "sketch", Feature("sketch", 1)),
            p("Extrude o perfil até a altura da caixa.", "extrusao", Feature("extrusao", 1)),
            p("Esvazie a peça com a casca, escolhendo a face que fica aberta.", "casca", Feature("casca", 1)),
        ],
        &["sol_esboco", "sol_volume", "sol_casca"],
    ),
    t(
        "tut_suporte",
        "Quero um suporte de chapa dobrada",
        "Suporte de chapa",
        "Parede base, aba com alívio, bainha e veneziana; a peça dobra e planifica.",
        Chapa,
        Iniciante,
        "Suporte de chapa",
        &[
            p("Crie a parede base a partir de um perfil fechado.", "sm_parede", Feature("sm_parede", 1)),
            p("Dobre uma aba numa aresta da parede.", "sm_aba", Feature("sm_aba", 1)),
            p("Feche a borda oposta com uma bainha.", "sm_bainha", Feature("sm_bainha", 1)),
        ],
        &["ch_base", "ch_abas", "ch_abas"],
    ),
    t(
        "tut_quadro",
        "Quero um quadro de tubos soldados",
        "Quadro de mesa",
        "Esqueleto de linhas, tubos quadrados sobre ele e cantos em meia-esquadria.",
        Estrutural,
        Intermediario,
        "Quadro de mesa",
        &[
            p("Desenhe as linhas do esqueleto do quadro.", "linha_pontos", Referencias(1)),
            p("Crie os membros sobre as linhas com a seção do tubo.", "st_membro", Feature("st_membro", 1)),
            p("Escolha o canto entre os tubos: meia-esquadria, topo ou encaixe.", "st_canto_meia", Mudou("st_membro")),
        ],
        &["est_membros", "est_membros", "est_cantos"],
    ),
    t(
        "tut_fixadores",
        "Quero prender duas peças com parafusos",
        "Peças presas com parafusos",
        "Duas peças na montagem, posicionadas por restrições e presas pelos fixadores inteligentes.",
        Montagem,
        Intermediario,
        "Conjunto de exemplo",
        &[
            p("Insira as duas peças na montagem.", "asm_inserir", Componentes(2)),
            p("Encoste uma na outra com restrições.", "rest_coincidencia", Restricoes(1)),
            p("Preencha os furos com fixadores: o parafuso sai do furo e da espessura atravessada.", "asm_fixadores", Comando("asm_fixadores")),
            p("Use as regras por cor para os furos que pedem outro diâmetro.", "asm_regras_cor", Comando("asm_regras_cor")),
        ],
        &["mon_inserir", "mon_restringir", "mon_fixadores", "mon_fixadores"],
    ),
];

pub fn tutorial(id: &str) -> Option<&'static Tutorial> {
    TUTORIAIS.iter().find(|t| t.aula.id == id)
}

/// Tutoriais da bancada, na ordem da lista.
pub fn da_bancada(b: Bench) -> impl Iterator<Item = &'static Tutorial> {
    TUTORIAIS.iter().filter(move |t| t.aula.bancada == b)
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::curso::{aula, Contexto, EstadoCurso, AULAS};

    #[test]
    fn cada_passo_leva_a_uma_aula_que_ensina_o_mesmo_comando() {
        let icones: Vec<&str> = crate::ajuda_cmd::comandos().iter().map(|i| i.icon).collect();
        let mut ids = vec![];
        for tu in TUTORIAIS {
            assert!(!ids.contains(&tu.aula.id) && !AULAS.iter().any(|a| a.id == tu.aula.id), "{}: id repetido", tu.aula.id);
            ids.push(tu.aula.id);
            assert_eq!(tu.licoes.len(), tu.aula.passos.len(), "{}: uma aula por passo", tu.aula.id);
            assert!(crate::curso_exemplos::existe(tu.aula.exemplo.unwrap()), "{}: peça pronta não existe", tu.aula.id);
            for (p, l) in tu.aula.passos.iter().zip(tu.licoes) {
                let a = aula(l).unwrap_or_else(|| panic!("{}: aula {l} não existe", tu.aula.id));
                assert_eq!(a.bancada, tu.aula.bancada, "{}: a aula {l} é de outra bancada", tu.aula.id);
                assert!(icones.contains(&p.icone), "{}: ícone {} não é de nenhum comando", tu.aula.id, p.icone);
                // a aula ensina o comando do passo: algum passo dela usa o mesmo ícone
                assert!(a.passos.iter().any(|q| q.icone == p.icone || (q.cond == p.cond && p.cond != Cond::Manual)), "{}: a aula {l} não ensina {}", tu.aula.id, p.icone);
            }
        }
        assert!(TUTORIAIS.len() >= 6);
    }

    #[test]
    fn todo_texto_dos_tutoriais_tem_traducao() {
        let mut faltam = vec![];
        for tu in TUTORIAIS {
            for s in [tu.objetivo, tu.aula.titulo, tu.aula.resumo].into_iter().chain(tu.aula.passos.iter().map(|p| p.texto)) {
                if !crate::i18n::tem_traducao(s) {
                    faltam.push(s);
                }
            }
        }
        assert!(faltam.is_empty(), "sem tradução: {faltam:#?}");
    }

    /// A peça pronta cumpre todos os passos que deixam rastro na peça: abrir o exemplo mostra o
    /// tutorial feito até onde o documento sabe contar.
    #[test]
    fn a_peca_pronta_cumpre_os_passos_do_tutorial() {
        for tu in TUTORIAIS.iter().filter(|t| t.aula.bancada != Bench::Montagem) {
            let (doc, _) = crate::curso_exemplos::peca(tu.aula.exemplo.unwrap()).unwrap();
            let mut e = EstadoCurso::default();
            e.sincronizar(&tu.aula, &Contexto::so_documento(&doc));
            for (i, p) in tu.aula.passos.iter().enumerate() {
                // mudar um valor é gesto da pessoa: a peça pronta não tem como cumprir
                if !matches!(p.cond, Cond::Manual | Cond::Mudou(_)) {
                    assert!(e.prog(tu.aula.id).passo_feito(i), "{}: a peça pronta não cumpre o passo {i} ({})", tu.aula.id, p.texto);
                }
            }
        }
        // o de fixadores conclui com as peças, a restrição e os dois comandos usados
        let tu = tutorial("tut_fixadores").unwrap();
        let doc = fenkra_core::Document::new("M");
        let mut e = EstadoCurso::default();
        let c = Contexto { componentes: 2, restricoes: 1, usados: &["asm_fixadores", "asm_regras_cor"], ..Contexto::so_documento(&doc) };
        e.sincronizar(&tu.aula, &c);
        assert!(e.concluida(&tu.aula));
    }

    #[test]
    fn pular_e_retomar_um_tutorial_guarda_o_progresso() {
        let tu = tutorial("tut_caixa").unwrap();
        let mut e = EstadoCurso::default();
        e.confirmar(&tu.aula, 0);
        e.pular_passo(&tu.aula, 1);
        e.pular_aula(&tu.aula);
        let volta: EstadoCurso = serde_json::from_str(&serde_json::to_string(&e).unwrap()).unwrap();
        assert!(volta.prog(tu.aula.id).aula_pulada && volta.prog(tu.aula.id).passo_pulado(1));
        let mut volta = volta;
        volta.retomar(&tu.aula);
        let pr = volta.prog(tu.aula.id);
        assert!(!pr.aula_pulada && pr.passo_feito(0), "retomar volta aos pulados e guarda o feito");
        assert_eq!(volta.proximo_passo(&tu.aula), Some(1));
    }
}
