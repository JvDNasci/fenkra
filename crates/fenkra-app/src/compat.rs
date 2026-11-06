// SPDX-License-Identifier: LGPL-2.1-or-later
//! Modo de compatibilidade "de onde você vem?" (B324).
//!
//! A pessoa escolhe um perfil neutro de fluxo, descrito pelo jeito de trabalhar e nunca por nome de
//! software (decisão de 05/11/2026). O perfil faz três coisas: ajusta atalhos e navegação com uma
//! das predefinições que já existem, mostra lado a lado o fluxo que ela conhece e o do Fenkra
//! (comando equivalente, onde fica e o que muda) e aponta a trilha do curso (B323) mais adequada.
//! Tudo pode ser trocado depois em Ajuda. Os atalhos que a pessoa mudou à mão são preservados.

use crate::ajuda_cmd;
use crate::curso::{self, Trilha};
use crate::keymap::Preset;
use crate::settings::{Bench, Group, Settings};
use crate::toolbar::{self, Entry, Item};

/// Uma linha da comparação: o que a pessoa faz no fluxo dela, o comando daqui e o que muda.
#[derive(Clone, Copy, Debug)]
pub struct Equivalente {
    pub seu: &'static str,
    /// ícone do comando equivalente no Fenkra
    pub comando: &'static str,
    pub muda: &'static str,
}

#[derive(Clone, Copy, Debug)]
pub struct Perfil {
    pub id: &'static str,
    pub nome: &'static str,
    pub para_quem: &'static str,
    /// predefinição de atalhos (a navegação do mouse acompanha, ver `Preset::nav`)
    pub preset: Preset,
    /// trilha do curso que o perfil abre
    pub trilha: &'static str,
    pub equivalentes: [Equivalente; 3],
}

const fn q(seu: &'static str, comando: &'static str, muda: &'static str) -> Equivalente {
    Equivalente { seu, comando, muda }
}

pub const PERFIS: &[Perfil] = &[
    Perfil {
        id: "iniciante",
        nome: "Estou começando",
        para_quem: "Nunca usei um programa de modelagem 3D. Quero o caminho mais curto e com explicação.",
        preset: Preset::Fenkra,
        trilha: "primeiros_passos",
        equivalentes: [
            q("Ter uma ideia de peça", "sketch", "Tudo começa num desenho 2D sobre um plano; a barra mostra só o que serve agora."),
            q("Dar volume ao desenho", "extrusao", "Empurre o desenho para criar o sólido; a árvore guarda cada passo."),
            q("Mudar uma medida", "parametro", "Mude o valor na tabela de parâmetros e a peça inteira recalcula."),
        ],
    },
    Perfil {
        id: "recursos",
        nome: "Esboço e recursos em histórico",
        para_quem: "Modelo peças desenhando um esboço e aplicando recursos em sequência, com histórico editável.",
        preset: Preset::VistasCtrl,
        trilha: "solidos",
        equivalentes: [
            q("Esboçar sobre um plano", "sketch", "O esboço fica na árvore e pode ser reeditado a qualquer hora."),
            q("Aplicar um recurso", "extrusao", "Os recursos ficam na linha do tempo; o marcador de rollback volta no tempo."),
            q("Editar uma cota", "parametro", "Toda cota pode ser um parâmetro com nome, usável em expressões."),
        ],
    },
    Perfil {
        id: "mecanico",
        nome: "Peças e montagens mecânicas",
        para_quem: "Projeto conjuntos de peças que se encaixam, com restrições e lista de peças.",
        preset: Preset::PoucosAtalhos,
        trilha: "montagem",
        equivalentes: [
            q("Montar a partir de peças", "asm_inserir", "A montagem é um documento próprio; as peças continuam arquivos separados."),
            q("Posicionar por relações", "rest_coincidencia", "As restrições ficam na bancada Montagem, no grupo Restrições e movimento."),
            q("Procurar colisões", "rest_interferencia", "A verificação lista os pares de peças que se atravessam."),
        ],
    },
    Perfil {
        id: "fabricacao",
        nome: "Chapa e estruturas soldadas",
        para_quem: "Projeto peças dobradas e estruturas de perfis que vão para o corte e a solda.",
        preset: Preset::LetraF6,
        trilha: "chapa",
        equivalentes: [
            q("Dobrar uma chapa", "sm_aba", "Espessura e fator K ficam nos parâmetros da chapa, valendo para a peça toda."),
            q("Ver a peça plana", "sm_desdobrar", "Dobrar e desdobrar são o mesmo comando, e o comprimento planificado aparece na hora."),
            q("Montar com perfis", "st_membro", "Cada linha do esqueleto vira um perfil da biblioteca, com cantos configuráveis."),
        ],
    },
    Perfil {
        id: "forma_livre",
        nome: "Superfícies e forma livre",
        para_quem: "Desenho formas orgânicas ou de design industrial, onde a suavidade da superfície importa.",
        preset: Preset::VistasShift,
        trilha: "superficies",
        equivalentes: [
            q("Desenhar a curva guia", "spline", "Curvas e pontos de referência ficam no grupo Wireframe, separados dos sólidos."),
            q("Criar a superfície", "sup_multisecoes", "A superfície nasce das curvas e continua editável; a continuidade é escolhida no comando."),
            q("Conferir a suavidade", "an_zebra", "Zebra e curvatura ficam no grupo Análise, sempre à mão."),
        ],
    },
    Perfil {
        id: "prancheta",
        nome: "Desenho técnico de prancheta",
        para_quem: "Meu trabalho é produzir folhas de desenho com vistas, cotas e tabelas.",
        preset: Preset::LetraHome,
        trilha: "desenho",
        equivalentes: [
            q("Abrir a folha", "des_novo", "O desenho é um documento próprio, com folhas, formato e carimbo."),
            q("Projetar as vistas", "des_tres_vistas", "As vistas saem do modelo 3D e acompanham as mudanças dele."),
            q("Cotar", "des_cota_h", "A cota lê a medida do modelo, então você não digita o valor."),
        ],
    },
    Perfil {
        id: "codigo_aberto",
        nome: "CAD de código aberto",
        para_quem: "Já uso CAD livre e quero um fluxo parecido, com bancadas e árvore de operações.",
        preset: Preset::FreeCad,
        trilha: "solidos",
        equivalentes: [
            q("Esboçar com restrições", "sketch", "O solver mostra os graus de liberdade que sobram em cada esboço."),
            q("Aplicar uma operação de sólido", "extrusao", "As operações ficam na árvore, na ordem de execução, e o recálculo é incremental."),
            q("Navegar com o mouse", "ajustar", "O estilo de navegação já vem no perfil; troque em Configurações se preferir outro."),
        ],
    },
    Perfil {
        id: "malha",
        nome: "Modelagem 3D por malhas",
        para_quem: "Venho da modelagem artística por malhas e quero entender o que é diferente num CAD.",
        preset: Preset::Blender,
        trilha: "superficies",
        equivalentes: [
            q("Começar de uma forma básica", "prim_caixa", "Os sólidos básicos são exatos, não malhas, e guardam as medidas como parâmetros."),
            q("Moldar com simetria", "espelho", "O espelho é um comando editável na árvore, não uma cópia fixa."),
            q("Arredondar as bordas", "arestas_selecionar", "O filete usa um raio em milímetros, não um nível de subdivisão."),
        ],
    },
];

pub fn perfil(id: &str) -> Option<&'static Perfil> {
    PERFIS.iter().find(|p| p.id == id)
}

impl Perfil {
    pub fn trilha(&self) -> &'static Trilha {
        curso::trilha(self.trilha).expect("perfil aponta para trilha existente")
    }

    /// Bancada em que a trilha do perfil começa.
    pub fn bancada(&self) -> Bench {
        self.trilha().aulas.first().and_then(|a| curso::aula(a)).map_or(Bench::Solidos, |a| a.bancada)
    }
}

/// Aplica o perfil: atalhos e navegação da predefinição, perfil lembrado, curso na trilha dele.
/// Os atalhos que a pessoa trocou à mão continuam valendo. Não muda a bancada nem abre nada.
pub fn aplicar(s: &mut Settings, p: &Perfil) {
    s.keys.preset = p.preset;
    s.nav = p.preset.nav();
    s.curso.perfil = Some(p.id.to_string());
    s.curso.perfil_visto = true;
    s.curso.atual = s.curso.proxima_da_trilha(p.trilha()).map(|a| a.id.to_string());
}

/// A pessoa dispensou a pergunta: nada muda e ela não é perguntada de novo.
pub fn dispensar(s: &mut Settings) {
    s.curso.perfil_visto = true;
}

/// Perfil escolhido.
pub fn escolhido(s: &Settings) -> Option<&'static Perfil> {
    s.curso.perfil.as_deref().and_then(perfil)
}

/// Comando da barra pelo ícone.
pub fn comando(icone: &str) -> Option<Item> {
    ajuda_cmd::catalogo().iter().find(|i| i.icon == icone).copied()
}

/// Grupo da barra em que o comando mora (primeiro que o contém).
pub fn grupo_do_comando(icone: &str) -> Option<Group> {
    Group::ALL.into_iter().find(|g| {
        toolbar::entries(*g).iter().any(|e| match e {
            Entry::One(i) => i.icon == icone,
            Entry::Family(_, v) => v.iter().any(|i| i.icon == icone),
        })
    })
}

/// "Bancada › Grupo" onde o comando fica, já traduzido.
pub fn onde_fica(icone: &str) -> Option<String> {
    use crate::i18n::tr;
    let g = grupo_do_comando(icone)?;
    let b = crate::mapa_macro::BANCADAS.into_iter().find(|b| g.in_bench(*b)).unwrap_or(Bench::Solidos);
    Some(format!("{} › {}", tr(crate::mapa_macro::nome_da_bancada(b)), g.label()))
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn perfis_neutros_sem_nome_de_software() {
        // lista de nomes proibidos montada em pedaços, para o próprio teste não citar marcas
        let proibidos = [["Solid", "Works"].concat(), ["Cat", "ia"].concat(), ["Fus", "ion"].concat(), ["Inven", "tor"].concat(), ["Onsh", "ape"].concat(), ["Rhi", "no"].concat(), ["Auto", "CAD"].concat(), ["Cr", "eo"].concat(), ["Sie", "mens"].concat(), ["Free", "CAD"].concat(), ["Blen", "der"].concat()];
        for p in PERFIS {
            for t in [p.nome, p.para_quem].into_iter().chain(p.equivalentes.iter().flat_map(|e| [e.seu, e.muda])) {
                for x in &proibidos {
                    assert!(!t.to_lowercase().contains(&x.to_lowercase()), "{}: nome de software em «{t}»", p.id);
                }
            }
        }
    }

    #[test]
    fn cada_perfil_aplica_atalhos_e_navegacao_e_abre_a_trilha_certa() {
        let mut vistos = std::collections::HashSet::new();
        assert!(PERFIS.iter().all(|p| vistos.insert(p.id)));
        assert!(PERFIS.iter().all(|p| vistos.insert(p.nome)));
        let mut presets = std::collections::HashSet::new();
        for p in PERFIS {
            assert!(presets.insert(p.preset.nome()), "{}: dois perfis com a mesma predefinição", p.id);
            let mut s = Settings::default();
            s.keys.preset = Preset::Fenkra;
            aplicar(&mut s, p);
            assert_eq!(s.keys.preset, p.preset, "{}", p.id);
            assert_eq!(s.nav, p.preset.nav(), "{}", p.id);
            assert_eq!(escolhido(&s).map(|x| x.id), Some(p.id));
            assert!(s.curso.perfil_visto);
            // a trilha certa: a aula em andamento é a primeira da trilha do perfil
            let t = curso::trilha(p.trilha).unwrap_or_else(|| panic!("{}: trilha", p.id));
            assert_eq!(s.curso.atual.as_deref(), Some(t.aulas[0]), "{}", p.id);
            assert_eq!(p.bancada(), curso::aula(t.aulas[0]).unwrap().bancada);
        }
    }

    #[test]
    fn trocar_de_perfil_depois_funciona_e_preserva_atalhos_do_usuario() {
        let mut s = Settings::default();
        let a = perfil("recursos").unwrap();
        let b = perfil("prancheta").unwrap();
        aplicar(&mut s, a);
        s.keys.set(crate::keymap::KeyAction::Extrude, crate::keymap::Binding::parse("Ctrl+Shift+E"));
        aplicar(&mut s, b);
        assert_eq!(escolhido(&s).map(|x| x.id), Some("prancheta"));
        assert_eq!(s.keys.preset, Preset::LetraHome);
        assert_eq!(s.keys.binding(crate::keymap::KeyAction::Extrude).map(|x| x.text()).as_deref(), Some("Ctrl+Shift+E"), "o atalho que a pessoa trocou fica");
        // o progresso de uma aula já feita sobrevive à troca
        let au = curso::aula("sol_esboco").unwrap();
        s.curso.confirmar(au, 1);
        aplicar(&mut s, a);
        assert!(s.curso.prog("sol_esboco").passo_feito(1));
    }

    #[test]
    fn dispensar_nao_muda_atalhos_nem_navegacao() {
        let mut s = Settings::default();
        let antes = (s.keys.clone(), s.nav);
        dispensar(&mut s);
        assert!(s.curso.perfil_visto && s.curso.perfil.is_none());
        assert_eq!((s.keys.clone(), s.nav), antes);
    }

    #[test]
    fn comando_equivalente_existe_e_diz_onde_fica() {
        for p in PERFIS {
            for e in &p.equivalentes {
                let c = comando(e.comando).unwrap_or_else(|| panic!("{}: comando {} não existe", p.id, e.comando));
                assert!(!c.name.is_empty());
                let onde = onde_fica(e.comando).unwrap_or_else(|| panic!("{}: {} não está em nenhum grupo", p.id, e.comando));
                assert!(onde.contains('›'));
            }
        }
        let onde = onde_fica("sm_aba").unwrap();
        assert!(onde.contains("Chapa"), "{onde}");
    }

    #[test]
    fn textos_dos_perfis_tem_traducao() {
        let mut faltam = vec![];
        for p in PERFIS {
            for t in [p.nome, p.para_quem].into_iter().chain(p.equivalentes.iter().flat_map(|e| [e.seu, e.muda])) {
                if !crate::i18n::tem_traducao(t) {
                    faltam.push(t);
                }
            }
        }
        assert!(faltam.is_empty(), "{faltam:#?}");
    }
}
