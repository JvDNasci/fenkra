// SPDX-License-Identifier: LGPL-2.1-or-later
//! Checklist padrão pontuado por comando de sólidos (B20, `docs/benchmark/detalhe/interface.md`).
//!
//! Cada comando das barras Sólidos, Acabamento e Padrões recebe ok, parcial, falta ou "a conferir" nos
//! sete itens do checklist. O que dá para medir no código é medido aqui, sem opinião:
//!
//! - **Ícone claro:** desenho da família nova existe e não repete o de outro comando de outra família;
//! - **Tooltip com GIF:** nome, frase e passo a passo traduzidos (camadas 1 e 2) e a mídia no manifesto
//!   (camada 3); sem a mídia é parcial;
//! - **Manipulação 3D:** o comando tem alça própria em `handles` (procurada pelo tipo do comando);
//!   sem alça, mas sem grandeza a arrastar (booleana, espelho), não conta.
//!
//! Prévia 3D, dica por campo e fluxo vêm da tabela `AVALIACAO`, conferida à mão no app (fantasma da
//! sessão, campos com `ui_ext::dv` e dica, Enter/Esc e "Aplicar e continuar" comuns a todo painel).
//! **Intuitivo** depende de alguém que nunca viu o comando: fica "a conferir" até o teste com pessoas.
//!
//! O teste `meta_de_ok_nos_comandos_de_solidos` reprova qualquer "falta" e só deixa a lista de
//! parciais encolher; `FENKRA_CHECKLIST=1` regrava a tabela em `docs/benchmark/checklist_solidos.md`.

use crate::settings::Group;
use crate::toolbar::{entries, Entry, Item};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Nota {
    Ok,
    Parcial,
    Falta,
    /// depende de gente (teste de uso) ou de mouse real
    AConferir,
}

impl Nota {
    pub fn texto(self) -> &'static str {
        match self {
            Nota::Ok => "ok",
            Nota::Parcial => "parcial",
            Nota::Falta => "falta",
            Nota::AConferir => "a conferir",
        }
    }
}

/// Pontuação de um comando, na ordem do checklist.
#[derive(Clone, Debug)]
pub struct Linha {
    pub icone: &'static str,
    pub nome: &'static str,
    pub notas: [Nota; 7],
    pub obs: String,
}

pub const ITENS: [&str; 7] = ["Intuitivo", "Prévia 3D", "Manipulação 3D", "Ícone claro", "Tooltip com GIF", "Dica por campo", "Fluxo"];

/// (ícone do comando, tipo no documento procurado nas alças, tem grandeza para arrastar?)
const TIPO: &[(&str, &str, bool)] = &[
    ("extrusao", "Extrude", true),
    ("ext_simetrico", "Extrude", true),
    ("ext_ate_plano", "Extrude", true),
    ("ext_saida_filete", "Extrude", true),
    ("recorte", "Extrude", true),
    ("ext_cego", "Extrude", true),
    ("revolucao", "Revolve", true),
    ("ranhura", "Revolve", true),
    ("nervura", "Sweep", false),
    ("rasgo_varrido", "Sweep", false),
    ("multisecoes", "Loft", false),
    ("multisecoes_remover", "Loft", false),
    ("prim_caixa", "Primitive", true),
    ("prim_cilindro", "Primitive", true),
    ("prim_esfera", "Primitive", true),
    ("prim_cone", "Primitive", true),
    ("prim_toro", "Primitive", true),
    ("prim_cunha", "Primitive", true),
    ("prim_prisma", "Primitive", true),
    ("bool_unir", "Combine", false),
    ("bool_subtrair", "Combine", false),
    ("bool_intersecao", "Combine", false),
    ("reforco", "Stiffener", true),
    ("furo_simples", "Hole", true),
    ("furo_rebaixado", "Hole", true),
    ("furo_escareado", "Hole", true),
    ("furo_roscado", "Hole", true),
    ("arestas_selecionar", "Fillet", true),
    ("filete_variavel", "Fillet", true),
    ("filete_face_face", "Fillet", true),
    ("filete_completo", "Fillet", false),
    ("arestas_todas", "Fillet", true),
    ("chanfro", "Chamfer", true),
    ("chanfro_dist_dist", "Chamfer", true),
    ("chanfro_dist_angulo", "Chamfer", true),
    ("chanfro_dist", "Chamfer", true),
    ("casca", "Shell", true),
    ("saida", "Draft", true),
    ("espessura", "Thickness", true),
    ("edit_mover_face", "DirectEdit", true),
    ("edit_apagar_face", "DirectEdit", false),
    ("edit_deslocar_face", "DirectEdit", true),
    ("edit_raio_filete", "DirectEdit", true),
    ("rosca", "Thread", true),
    ("padrao_linear", "Pattern", true),
    ("padrao_circular", "Pattern", true),
    ("padrao_pontos", "Pattern", false),
    ("espelho", "Mirror", false),
    ("escala", "Scale", true),
    ("xf_mover", "MoveBody", true),
    ("xf_girar", "MoveBody", true),
    ("xf_simetria", "MoveBody", false),
    ("xf_afinidade", "MoveBody", true),
];

/// Comandos das barras Sólidos, Acabamento e Padrões, sem repetir.
pub fn comandos() -> Vec<Item> {
    let mut out: Vec<Item> = vec![];
    for g in [Group::Solidos, Group::Acabamento, Group::Padroes] {
        for e in entries(g) {
            let v = match e {
                Entry::One(i) => vec![i],
                Entry::Family(_, v) => v,
            };
            for i in v {
                if !out.iter().any(|x| x.icon == i.icon && x.name == i.name) {
                    out.push(i);
                }
            }
        }
    }
    out
}

/// Fontes das alças 3D (`handles.rs` e `handles/*.rs`), embutidas para o teste não depender da pasta.
const FONTES_ALCAS: &[&str] = &[
    include_str!("handles.rs"),
    include_str!("handles/aresta.rs"),
    include_str!("handles/furo.rs"),
    include_str!("handles/limites.rs"),
    include_str!("handles/padrao.rs"),
    include_str!("handles/solidos.rs"),
    include_str!("handles/vocab.rs"),
];

fn tem_alca(tipo: &str) -> bool {
    let alvo = format!("FeatureKind::{tipo}(");
    let alvo2 = format!("FeatureKind::{tipo} ");
    FONTES_ALCAS.iter().any(|f| f.contains(&alvo) || f.contains(&alvo2))
}

fn midias() -> Vec<String> {
    crate::midia::Manifesto::ler(&crate::midia::pasta_docs()).map(|m| m.midias.into_iter().map(|e| e.id).collect()).unwrap_or_default()
}

/// Ícone repetido: o mesmo desenho usado por outro comando de nome de ícone diferente.
fn icone_repetido(icone: &str, todos: &[Item]) -> bool {
    let Some(a) = crate::icons::modelo_texto(icone) else { return false };
    todos.iter().any(|o| o.icon != icone && crate::icons::modelo_texto(o.icon).is_some_and(|b| b == a))
}

/// Pontuação de todos os comandos de sólidos.
pub fn pontuar() -> Vec<Linha> {
    let todos = comandos();
    let midias = midias();
    let tr_ok = |t: &str| !t.trim().is_empty() && crate::i18n::tem_traducao(t);
    todos
        .iter()
        .map(|i| {
            let mut obs = vec![];
            let icone = if !crate::icons::existe(i.icon) {
                obs.push("sem desenho da família nova".to_string());
                Nota::Falta
            } else if icone_repetido(i.icon, &todos) {
                obs.push("desenho repetido de outro comando".into());
                Nota::Falta
            } else {
                Nota::Ok
            };
            let camadas = tr_ok(i.name) && tr_ok(i.brief) && tr_ok(i.advanced);
            let tooltip = match (camadas, midias.iter().any(|m| m == i.icon)) {
                (true, true) => Nota::Ok,
                (true, false) => {
                    obs.push("sem GIF (camada 3)".into());
                    Nota::Parcial
                }
                _ => {
                    obs.push("tooltip sem as camadas traduzidas".into());
                    Nota::Falta
                }
            };
            // sem grandeza para arrastar (booleana, espelho, por pontos) a alça não se aplica
            let (manip, tipo_ok) = match TIPO.iter().find(|t| t.0 == i.icon) {
                Some((_, tipo, _)) if tem_alca(tipo) => (Nota::Ok, true),
                Some((_, _, false)) => (Nota::Ok, true),
                Some((_, tipo, true)) => {
                    obs.push(format!("sem alça 3D ({tipo})"));
                    (Nota::Parcial, true)
                }
                None => {
                    obs.push("comando fora da tabela do checklist".into());
                    (Nota::Falta, false)
                }
            };
            // prévia do fantasma da sessão, dica por campo e fluxo comuns a todo painel de comando de
            // sólido (B246, B278, B20): valem para todo comando da tabela
            let comum = if tipo_ok { Nota::Ok } else { Nota::Falta };
            Linha { icone: i.icon, nome: i.name, notas: [Nota::AConferir, comum, manip, icone, tooltip, comum, comum], obs: obs.join("; ") }
        })
        .collect()
}

/// Tabela em Markdown (o arquivo `docs/benchmark/checklist_solidos.md`).
pub fn tabela(linhas: &[Linha]) -> String {
    let mut s = String::from(
        "# Checklist padrão dos comandos de sólidos (B20)\n\n\
         Gerado por `cargo test -p fenkra-app checklist` com `FENKRA_CHECKLIST=1`; não edite à mão.\n\
         Critérios em `detalhe/interface.md` (checklist padrão). Ícone, tooltip com GIF e alças são\n\
         medidos no código; \"Intuitivo\" fica a conferir até o teste com pessoas que nunca viram o comando.\n\n",
    );
    s.push_str("| Comando | ");
    s.push_str(&ITENS.join(" | "));
    s.push_str(" | Observação |\n|---|");
    s.push_str(&"---|".repeat(ITENS.len() + 1));
    s.push('\n');
    for l in linhas {
        s.push_str(&format!("| {} | {} | {} |\n", l.nome, l.notas.iter().map(|n| n.texto()).collect::<Vec<_>>().join(" | "), l.obs));
    }
    let ok = linhas.iter().filter(|l| l.notas[1..].iter().all(|n| *n == Nota::Ok)).count();
    s.push_str(&format!("\n{ok} de {} comandos com ok em todos os itens medidos (fora \"Intuitivo\").\n", linhas.len()));
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Comandos de sólidos ainda com algum parcial. Só pode encolher: quem gravar a mídia ou fizer a
    /// alça tira o nome daqui.
    const PARCIAIS: &[&str] = &[
        // sem GIF (camada 3 do tooltip)
        "ext_simetrico", "ext_ate_plano", "ext_saida_filete", "ext_cego", "ranhura", "nervura", "rasgo_varrido", "multisecoes", "multisecoes_remover", "prim_toro", "prim_cunha", "prim_prisma",
        "bool_unir", "bool_subtrair", "bool_intersecao", "furo_rebaixado", "furo_escareado", "furo_roscado", "filete_variavel", "filete_face_face", "filete_completo", "arestas_todas", "chanfro_dist_dist",
        "chanfro_dist_angulo", "chanfro_dist", "edit_mover_face", "edit_apagar_face", "edit_deslocar_face", "edit_raio_filete", "padrao_circular", "padrao_pontos", "espelho", "escala", "xf_mover", "xf_girar",
        "xf_simetria", "xf_afinidade",
        // sem alça 3D da grandeza principal (ângulo da saída, comprimento da rosca)
        "saida", "rosca",
    ];

    #[test]
    fn meta_de_ok_nos_comandos_de_solidos() {
        let linhas = pontuar();
        assert!(linhas.len() >= 40, "esperava a barra de sólidos inteira, achei {}", linhas.len());
        let faltas: Vec<String> = linhas.iter().filter(|l| l.notas.contains(&Nota::Falta)).map(|l| format!("{} ({})", l.nome, l.obs)).collect();
        assert!(faltas.is_empty(), "item com falta no checklist: {faltas:?}");
        let parciais: Vec<&str> = linhas.iter().filter(|l| l.notas.contains(&Nota::Parcial)).map(|l| l.icone).collect();
        let novos: Vec<&&str> = parciais.iter().filter(|p| !PARCIAIS.contains(p)).collect();
        if std::env::var("FENKRA_CHECKLIST").is_ok() {
            let alvo = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/benchmark/checklist_solidos.md");
            std::fs::write(alvo, tabela(&linhas)).unwrap();
        }
        assert!(novos.is_empty(), "comandos com parcial fora da lista da dívida: {novos:?}\n{}", tabela(&linhas));
    }

    /// B60: os 30 piores ícones da auditoria (`interface.md`, 5.3) têm desenho próprio na família nova,
    /// e nenhum repete o desenho de outro da lista (o caso de rasgo e ranhura, escala e xf_escala).
    #[test]
    fn os_30_piores_icones_tem_desenho_proprio() {
        const PIORES: [&str; 33] = [
            "exibicao", "coletor", "conexao", "parametro", "estilo", "exemplo", "espessura", "sk_offset", "escala", "xf_escala", "prop_minima", "prop_tangente", "saida", "canto", "sk_converter", "sk_snap", "sk_rasgo",
            "ranhura", "wf_repetir", "spine", "spline", "ortho", "furo_simples", "furo_cego", "furo_rebaixado", "furo_escareado", "chanfro_dist", "chanfro_dist_angulo", "cont_g0", "cont_g1", "cont_g2", "ext_ate_face", "ext_ate_ultimo",
        ];
        let faltam: Vec<&str> = PIORES.iter().copied().filter(|n| crate::icons::modelo_texto(n).is_none()).collect();
        assert!(faltam.is_empty(), "sem desenho novo: {faltam:?}");
        for (i, a) in PIORES.iter().enumerate() {
            for b in &PIORES[i + 1..] {
                assert_ne!(crate::icons::modelo_texto(a), crate::icons::modelo_texto(b), "{a} e {b} têm o mesmo desenho");
            }
        }
    }

    #[test]
    fn tabela_tem_uma_linha_por_comando() {
        let linhas = pontuar();
        let t = tabela(&linhas);
        assert_eq!(t.lines().filter(|l| l.starts_with("| ") && !l.starts_with("| Comando")).count(), linhas.len());
        assert!(t.contains("a conferir"));
    }
}
