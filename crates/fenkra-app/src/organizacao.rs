// SPDX-License-Identifier: LGPL-2.1-or-later
//! B436: comandos organizados por tipo de operação em cada bancada.
//!
//! Cada bancada tem uma barra por tipo de operação, com poucos botões; as variantes ficam na seta do
//! botão mais usado (a família da barra lembra a última variante escolhida). A ordem segue o estudo de
//! organização dos comandos (cofre, Conhecimento/Interface), a partir da documentação pública de
//! softwares de modelagem CAD avançados:
//!
//! - Sólidos: recursos de sketch, acabamento, superfície, transformações, booleanas e referências;
//! - Chapa: paredes (com os parâmetros da chapa primeiro), dobras, corte e estampagem, transformações;
//! - Estrutural: referências (o arame 3D de apoio), membros, ligações, solda e saídas;
//! - Montagem: estrutura do produto, recursos, restrições e a Biblioteca (fixadores, engrenagens e
//!   cálculo, que saíram da barra da montagem);
//! - Desenho: vistas, gerar cotas, cotas, anotações, folha e tabelas.
//!
//! `ordem` dá todos os grupos da bancada (os do contexto primeiro, na ordem da barra); `no_contexto` dá
//! os que ficam à vista na barra da direita. O resto vai para o botão "Mais", um clique adiante: nenhum
//! comando some. As restrições do sketch continuam no botão direito e, além disso, num botão só na
//! barra do sketch (menu pequeno), que se esconde como qualquer grupo (doca "oculta").

use crate::settings::{Bench, Group};
use Group as G;

/// Grupos à vista na barra da direita, na ordem da barra.
pub fn no_contexto(b: Bench) -> &'static [Group] {
    match b {
        Bench::Solidos => &[G::Sketch, G::Solidos, G::Acabamento, G::Hibrido, G::Padroes, G::Booleanas, G::Referencias],
        Bench::Superficies => &[G::Sketch, G::Wireframe, G::Superficies, G::Operacoes],
        Bench::Montagem => &[G::Montagem, G::AsmRecursos, G::Restricoes, G::Biblioteca],
        Bench::Chapa => &[G::Sketch, G::Chapa, G::ChDobras, G::ChCorte, G::ChTransformar],
        Bench::Estrutural => &[G::Sketch, G::Referencias, G::Estrutural, G::StLigacoes, G::StSolda, G::StSaidas],
        Bench::FormaLivre => &[G::FormaLivre],
        Bench::Desenho => &[G::DesVistas, G::DesGerar, G::DesCotas, G::DesAnotacoes, G::DesArquivo, G::DesTabelas],
    }
}

/// Grupos fora do contexto (botão "Mais"), na ordem do menu. Vista e seleção moram na barra de baixo.
fn no_mais(b: Bench) -> &'static [Group] {
    match b {
        Bench::Solidos => &[G::Primitivas, G::EdicaoDireta, G::Malha, G::Analise, G::Pmi, G::Organizacao, G::Vista, G::Selecao],
        Bench::Superficies => &[G::Hibrido, G::Malha, G::Analise, G::Pmi, G::Organizacao, G::Vista, G::Selecao],
        Bench::Montagem => &[G::Cinematica, G::Pmi, G::Analise, G::Vista, G::Selecao],
        Bench::Chapa => &[G::Referencias, G::Analise, G::Pmi, G::Organizacao, G::Vista, G::Selecao],
        Bench::Estrutural => &[G::Analise, G::Pmi, G::Organizacao, G::Vista, G::Selecao],
        Bench::FormaLivre => &[G::Referencias, G::Analise, G::Organizacao, G::Vista, G::Selecao],
        Bench::Desenho => &[],
    }
}

/// Todos os grupos da bancada: os do contexto, na ordem da barra, e depois os do "Mais".
pub fn ordem(b: Bench) -> Vec<Group> {
    no_contexto(b).iter().chain(no_mais(b)).copied().collect()
}

/// Grupos do modo sketch, na ordem da barra: perfil, operação, cotas, restrições (menu pequeno) e
/// ferramentas.
pub const DO_SKETCH: [Group; 5] = [G::SkCriacao, G::SkTransformar, G::SkCotagem, G::SkRestricoes, G::SkFerramentas];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{Dock, Settings};
    use crate::toolbar::{self, Entry};

    pub const TODAS: [Bench; 7] = [Bench::Solidos, Bench::Superficies, Bench::Montagem, Bench::Chapa, Bench::Estrutural, Bench::FormaLivre, Bench::Desenho];

    /// Comandos de uma entrada (o botão e as variantes da seta).
    fn acoes(e: Entry) -> Vec<String> {
        match e {
            Entry::One(i) => vec![format!("{:?}", i.act)],
            Entry::Family(_, v) => v.iter().map(|i| format!("{:?}", i.act)).collect(),
        }
    }

    /// Todos os comandos que a bancada oferece em alguma barra: dentro ou fora do contexto, com ou sem
    /// sketch aberto.
    fn acoes_da_bancada(b: Bench) -> Vec<String> {
        let mut v: Vec<String> = vec![];
        for g in Group::ALL {
            if g.in_bench(b) || g.sketch_only() {
                for e in toolbar::entries(g) {
                    for a in acoes(e) {
                        if !v.contains(&a) {
                            v.push(a);
                        }
                    }
                }
            }
        }
        v
    }

    /// Comandos de cada bancada antes do regrupamento (fotografados com `FENKRA_FOTO_COMANDOS`).
    const ANTES: &str = include_str!("organizacao_antes.tsv");

    /// B436: nenhum comando sumiu de nenhuma bancada com o regrupamento.
    #[test]
    fn nenhum_comando_sumiu() {
        let mut faltam = vec![];
        for linha in ANTES.lines().filter(|l| !l.trim().is_empty()) {
            let (banc, acao) = linha.split_once('\t').unwrap();
            let b = TODAS.into_iter().find(|b| format!("{b:?}") == banc).unwrap();
            if !acoes_da_bancada(b).iter().any(|a| a == acao) {
                faltam.push(format!("{banc}: {acao}"));
            }
        }
        assert!(faltam.is_empty(), "comandos que sumiram: {faltam:#?}");
        assert!(ANTES.lines().count() > 300, "fotografia incompleta");
    }

    /// B436: todo comando da bancada está a no máximo um clique: na barra da direita (botão ou seta) ou
    /// no "Mais", que mostra os grupos fora do contexto.
    #[test]
    fn todo_comando_da_bancada_esta_na_barra_ou_no_mais() {
        let mut s = Settings::default();
        for b in TODAS {
            s.bench = b;
            let (faixa, mais) = toolbar::contexto(&s, toolbar::groups_at(&s, Dock::Direita, false), false);
            let baixo = toolbar::grupos_de_baixo(&s, false);
            let alcance: Vec<Group> = faixa.iter().chain(&mais).chain(&baixo).copied().collect();
            for g in ordem(b) {
                assert!(alcance.contains(&g), "{b:?}: {g:?} fora de alcance");
            }
            assert_eq!(faixa, no_contexto(b).to_vec(), "{b:?}: ordem da barra");
        }
    }

    /// B436: a ordem dos grupos por bancada é a do estudo.
    #[test]
    fn ordem_dos_grupos_por_bancada() {
        let rotulos = |b: Bench| no_contexto(b).iter().map(|g| g.label()).collect::<Vec<_>>();
        assert_eq!(rotulos(Bench::Solidos), ["Sketch", "Recursos de sketch", "Acabamento", "Superfície e volume", "Transformações", "Booleanas", "Referências"]);
        assert_eq!(rotulos(Bench::Chapa), ["Sketch", "Paredes", "Dobras", "Corte e estampagem", "Transformações da chapa"]);
        assert_eq!(rotulos(Bench::Estrutural), ["Sketch", "Referências", "Membros", "Ligações", "Solda", "Saídas"]);
        assert_eq!(rotulos(Bench::Montagem), ["Estrutura do produto", "Recursos de montagem", "Restrições e movimento", "Biblioteca"]);
        assert_eq!(rotulos(Bench::Desenho), ["Vistas", "Gerar cotas", "Cotas", "Anotações e símbolos", "Folha", "Tabelas"]);
        // parâmetros da chapa é o primeiro botão da chapa
        assert_eq!(toolbar::entry_title(&toolbar::entries(Group::Chapa)[0]).0, "sm_parametros");
        // grupo nenhum repete numa bancada
        for b in TODAS {
            let o = ordem(b);
            for (i, g) in o.iter().enumerate() {
                assert!(!o[i + 1..].contains(g), "{b:?}: {g:?} repetido");
            }
        }
    }

    /// B436: fixadores, engrenagens, chaveta e cálculo saíram da estrutura do produto e estão na
    /// Biblioteca, com as variantes na seta do botão mais usado.
    #[test]
    fn fixadores_engrenagens_e_calculo_na_biblioteca() {
        let em = |g: Group| toolbar::entries(g).into_iter().flat_map(acoes).collect::<Vec<_>>();
        let bib = em(Group::Biblioteca);
        let estrutura = em(Group::Montagem);
        for icone in ["asm_fixadores", "asm_conjunto", "asm_regras_cor", "asm_catalogo", "asm_engrenagens", "asm_chaveta", "asm_calculo"] {
            let i = toolbar::all_entries().into_iter().flat_map(|e| match e {
                Entry::One(i) => vec![i],
                Entry::Family(_, v) => v,
            }).find(|i| i.icon == icone).unwrap();
            let a = format!("{:?}", i.act);
            assert!(bib.contains(&a), "{icone} fora da Biblioteca");
            assert!(!estrutura.contains(&a), "{icone} ainda na estrutura do produto");
        }
        // engrenagens, a mais usada, mostra a chaveta e o cálculo na seta
        let fam = toolbar::entries(Group::Biblioteca).into_iter().find_map(|e| match e {
            Entry::Family("asm_transmissao", v) => Some(v.iter().map(|i| i.icon).collect::<Vec<_>>()),
            _ => None,
        });
        assert_eq!(fam, Some(vec!["asm_engrenagens", "asm_chaveta", "asm_calculo"]));
    }

    /// B436: as restrições do sketch seguem no botão direito e, na barra, viram um botão só, com as
    /// restrições na seta; escondido o grupo, ele sai da barra.
    #[test]
    fn restricoes_num_menu_pequeno_opcional() {
        let mut s = Settings::default();
        let direita = toolbar::groups_at(&s, Dock::Direita, true);
        assert_eq!(direita, DO_SKETCH.to_vec());
        let menu = toolbar::entradas_na_barra(Group::SkRestricoes, &s);
        assert_eq!(menu.len(), 1, "um botão só");
        let n = acoes(menu[0].clone()).len();
        assert_eq!(n, toolbar::entries(Group::SkRestricoes).into_iter().flat_map(acoes).count());
        s.set_dock(Group::SkRestricoes, Dock::Oculta);
        assert!(!toolbar::groups_at(&s, Dock::Direita, true).contains(&Group::SkRestricoes));
    }

    /// B436: todo rótulo de grupo tem tradução.
    #[test]
    fn rotulos_dos_grupos_traduzidos() {
        for g in Group::ALL {
            assert!(crate::i18n::tem_traducao(g.label()), "{g:?} sem tradução");
        }
    }

    #[test]
    fn fotografa_comandos() {
        let Ok(dest) = std::env::var("FENKRA_FOTO_COMANDOS") else { return };
        let mut out = String::new();
        for b in TODAS {
            for a in acoes_da_bancada(b) {
                out.push_str(&format!("{b:?}\t{a}\n"));
            }
        }
        std::fs::write(dest, out).unwrap();
    }
}
