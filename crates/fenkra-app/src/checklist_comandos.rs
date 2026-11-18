// SPDX-License-Identifier: LGPL-2.1-or-later
//! Levantamento do contrato de comando em todo o registro de comandos (B20).
//!
//! `checklist_solidos` pontua os comandos de sólidos nos sete itens do checklist padrão. Este módulo
//! percorre **todas** as barras (`Group::ALL`) e mede, só pelo código, o que o contrato de
//! `docs/benchmark/detalhe/interface.md` pede de cada comando:
//!
//! - **Ícone, Tooltip (camadas traduzidas) e GIF** (camada 3): para qualquer comando de qualquer barra;
//! - **Manipulação 3D** (alça própria) e **Aplicar e continuar** (`continuar::sem_escolha` conhece o tipo,
//!   quando o comando vive da escolha na viewport): para os comandos de sólidos, que têm o tipo do
//!   documento conhecido (`TIPO`);
//! - **Enter/Esc/Ctrl+Enter**: o tratamento único das teclas na sessão de comando (`app.rs`) existe;
//! - **Estados visuais** (`alca_estado`): toda alça de sólidos desenha pelo `EstadoAlca`; as que ainda
//!   não desenham ficam na lista `SEM_ESTADO`, que só pode encolher.
//!
//! Prévia 3D, dica por campo e o fluxo do painel são do componente comum (`previa.rs`, `textos_campos.rs`,
//! `editor.rs`) e não variam por comando; os testes deles estão nos módulos deles. Intuitivo depende de
//! gente. Fora de sólidos, só ícone, tooltip e GIF são medidos: o resto fica "a conferir" até alguém
//! mapear o tipo do documento de cada comando.
//!
//! `FENKRA_CHECKLIST=1 cargo test -p fenkra-app checklist_comandos` regrava
//! `docs/benchmark/detalhe/checklist_comandos.md`.

use crate::checklist_solidos::{icone_repetido, midias, tem_alca, Nota, TIPO};
use crate::settings::Group;
use crate::toolbar::{entries, Entry, Item};

/// Tipos de comando que vivem da escolha de faces, arestas ou pontos na viewport: "Aplicar e continuar"
/// vale para eles e `continuar::sem_escolha` precisa conhecê-los.
const DE_ESCOLHA: &[&str] = &["Fillet", "Chamfer", "Hole", "Shell", "Draft", "Thickness", "DirectEdit"];

const FONTE_CONTINUAR: &str = include_str!("continuar.rs");
const FONTE_APP: &str = include_str!("app.rs");

/// Fontes onde mora o desenho das alças de sólidos.
const FONTES_ALCAS: &[(&str, &str)] = &[
    ("handles.rs", include_str!("handles.rs")),
    ("handles/aresta.rs", include_str!("handles/aresta.rs")),
    ("handles/furo.rs", include_str!("handles/furo.rs")),
    ("handles/limites.rs", include_str!("handles/limites.rs")),
    ("handles/padrao.rs", include_str!("handles/padrao.rs")),
    ("handles/solidos.rs", include_str!("handles/solidos.rs")),
];

#[derive(Clone, Debug)]
pub struct Linha {
    pub grupo: String,
    pub icone: &'static str,
    pub nome: &'static str,
    pub icone_nota: Nota,
    pub tooltip: Nota,
    pub gif: Nota,
    pub manip: Nota,
    pub continuar: Nota,
    pub teclas: Nota,
    pub estados: Nota,
    pub obs: String,
}

/// O código sem o módulo de testes do fim do arquivo. Um `#[cfg(test)] mod x;` solto no topo (declaração de
/// arquivo de teste) não corta o resto: só conta o `#[cfg(test)]` seguido de `mod ... {`.
fn codigo(f: &str) -> &str {
    let mut pos = 0;
    while let Some(i) = f[pos..].find("#[cfg(test)]") {
        let ini = pos + i;
        let resto = f[ini + "#[cfg(test)]".len()..].trim_start();
        if resto.starts_with("mod ") && resto.lines().next().is_some_and(|l| l.contains('{')) {
            return &f[..ini];
        }
        pos = ini + 1;
    }
    f
}

/// Funções de alça (as que chamam `ui.interact`) que não desenham pelo `EstadoAlca`: (arquivo, função).
pub fn alcas_sem_estado() -> Vec<(String, String)> {
    let mut out = vec![];
    for (arq, fonte) in FONTES_ALCAS {
        let f = codigo(fonte);
        // uma função de nível de módulo começa numa linha sem recuo
        let mut inicios: Vec<usize> = vec![];
        let mut pos = 0;
        for l in f.split_inclusive('\n') {
            let t = l.trim_start_matches("pub(crate) ").trim_start_matches("pub(super) ").trim_start_matches("pub ");
            if !l.starts_with(' ') && t.starts_with("fn ") {
                inicios.push(pos);
            }
            pos += l.len();
        }
        inicios.push(f.len());
        for w in inicios.windows(2) {
            let corpo = &f[w[0]..w[1]];
            if corpo.contains("ui.interact(") && !corpo.contains("EstadoAlca") {
                let ini = corpo.find("fn ").unwrap_or(0) + 3;
                let nome: String = corpo[ini..].chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
                out.push((arq.to_string(), nome));
            }
        }
    }
    out
}

fn todos_os_comandos() -> Vec<(Group, Item)> {
    let mut out: Vec<(Group, Item)> = vec![];
    for g in Group::ALL {
        for e in entries(g) {
            let v = match e {
                Entry::One(i) => vec![i],
                Entry::Family(_, v) => v,
            };
            for i in v {
                if !out.iter().any(|(_, x)| x.icon == i.icon && x.name == i.name) {
                    out.push((g, i));
                }
            }
        }
    }
    out
}

pub fn levantar() -> Vec<Linha> {
    let todos = todos_os_comandos();
    let itens: Vec<Item> = todos.iter().map(|(_, i)| *i).collect();
    let midias = midias();
    let tr_ok = |t: &str| !t.trim().is_empty() && crate::i18n::tem_traducao(t);
    let teclas_ok = {
        let a = codigo(FONTE_APP);
        a.contains("i.key_pressed(egui::Key::Enter) && i.modifiers.command") && a.contains("self.conclude_session()") && a.contains("self.cancel_session()")
    };
    todos
        .iter()
        .map(|(g, i)| {
            let mut obs = vec![];
            let icone_nota = if !crate::icons::existe(i.icon) {
                obs.push("sem desenho da família nova");
                Nota::Falta
            } else if icone_repetido(i.icon, &itens) {
                obs.push("desenho repetido");
                Nota::Falta
            } else {
                Nota::Ok
            };
            let tooltip = if tr_ok(i.name) && tr_ok(i.brief) && tr_ok(i.advanced) {
                Nota::Ok
            } else {
                obs.push("tooltip sem as camadas traduzidas");
                Nota::Falta
            };
            let gif = if midias.iter().any(|m| m == i.icon) {
                Nota::Ok
            } else {
                obs.push("sem GIF");
                Nota::Parcial
            };
            let (manip, continuar, teclas, estados) = match TIPO.iter().find(|t| t.0 == i.icon) {
                None => (Nota::AConferir, Nota::AConferir, Nota::AConferir, Nota::AConferir),
                Some((_, t, arrasta)) => {
                    let manip = if tem_alca(t) || !*arrasta {
                        Nota::Ok
                    } else {
                        obs.push("sem alça 3D");
                        Nota::Parcial
                    };
                    let continuar = if !DE_ESCOLHA.contains(t) {
                        Nota::NaoSeAplica
                    } else if codigo(FONTE_CONTINUAR).contains(&format!("FeatureKind::{t}(")) {
                        Nota::Ok
                    } else {
                        obs.push("aplicar e continuar não conhece o tipo");
                        Nota::Falta
                    };
                    let teclas = if teclas_ok { Nota::Ok } else { Nota::Falta };
                    let estados = if tem_alca(t) { Nota::Ok } else { Nota::NaoSeAplica };
                    (manip, continuar, teclas, estados)
                }
            };
            Linha { grupo: format!("{g:?}"), icone: i.icon, nome: i.name, icone_nota, tooltip, gif, manip, continuar, teclas, estados, obs: obs.join("; ") }
        })
        .collect()
}

impl Linha {
    fn notas(&self) -> [Nota; 7] {
        [self.icone_nota, self.tooltip, self.gif, self.manip, self.continuar, self.teclas, self.estados]
    }

    pub fn de_solidos(&self) -> bool {
        TIPO.iter().any(|t| t.0 == self.icone)
    }
}

pub const COLUNAS: [&str; 7] = ["Ícone", "Tooltip", "GIF", "Manipulação 3D", "Aplicar e continuar", "Enter/Esc", "Estados da alça"];

pub fn tabela(linhas: &[Linha]) -> String {
    let mut s = String::from(
        "# Levantamento do contrato de comando (B20)\n\n\
         Gerado por `FENKRA_CHECKLIST=1 cargo test -p fenkra-app checklist_comandos`; não edite à mão.\n\
         Mede só o que o código mostra, em todas as barras. Prévia 3D, dica por campo e fluxo do painel são do\n\
         componente comum e não variam por comando (ver `docs/benchmark/checklist_solidos.md`); Intuitivo\n\
         depende de gente. \"n/a\" = o critério não vale para o comando; \"a conferir\" = o tipo do documento\n\
         do comando ainda não foi mapeado (só os comandos de sólidos têm).\n\n",
    );
    s.push_str("| Barra | Comando | ");
    s.push_str(&COLUNAS.join(" | "));
    s.push_str(" | Observação |\n|---|---|");
    s.push_str(&"---|".repeat(COLUNAS.len() + 1));
    s.push('\n');
    for l in linhas {
        s.push_str(&format!("| {} | {} | {} | {} |\n", l.grupo, l.nome, l.notas().iter().map(|n| n.texto()).collect::<Vec<_>>().join(" | "), l.obs));
    }
    let sol: Vec<&Linha> = linhas.iter().filter(|l| l.de_solidos()).collect();
    let ok_sol = sol.iter().filter(|l| l.notas().iter().all(|n| matches!(n, Nota::Ok | Nota::NaoSeAplica))).count();
    s.push_str(&format!("\n- Comandos de sólidos com ok em tudo o que se mede: {ok_sol} de {}.\n", sol.len()));
    for (i, c) in COLUNAS.iter().enumerate() {
        let n = |nota: Nota| linhas.iter().filter(|l| l.notas()[i] == nota).count();
        s.push_str(&format!("- {c}: {} ok, {} parcial, {} falta, {} n/a, {} a conferir.\n", n(Nota::Ok), n(Nota::Parcial), n(Nota::Falta), n(Nota::NaoSeAplica), n(Nota::AConferir)));
    }
    let sem = alcas_sem_estado();
    s.push_str(&format!(
        "\nAlças de sólidos que ainda não usam `EstadoAlca` (só podem encolher): {}.\n",
        if sem.is_empty() { "nenhuma".into() } else { sem.iter().map(|(a, f)| format!("`{a}::{f}`")).collect::<Vec<_>>().join(", ") }
    ));
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Alças que desenham botão ou ponto clicável, não uma grandeza arrastável, e ainda sem os cinco
    /// estados. Só pode encolher.
    const SEM_ESTADO: &[(&str, &str)] = &[
        ("handles.rs", "tag"),
        ("handles.rs", "contour_overlay"),
        ("handles.rs", "inverter_plano"),
        ("handles/aresta.rs", "inverter"),
        ("handles/padrao.rs", "pontos"),
    ];

    /// Tetos de pendências fora dos sólidos (comando sem GIF, ícone ou tooltip). Só podem descer.
    /// Temporário: B506 (+9 botões da barra de vistas) e B509 (Medir) entraram sem GIF, porque a
    /// mídia pede a captura da janela. Volta a 297 quando os 10 GIFs forem gerados.
    const TETO_FORA_SEM_GIF: usize = 307;
    const TETO_FORA_SEM_ICONE: usize = 0;
    const TETO_FORA_SEM_TOOLTIP: usize = 10;

    #[test]
    fn contrato_de_comando_em_todo_o_registro() {
        let linhas = levantar();
        assert!(linhas.len() > 150, "esperava o registro inteiro, achei {}", linhas.len());
        if std::env::var("FENKRA_CHECKLIST").is_ok() {
            let alvo = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/benchmark/detalhe/checklist_comandos.md");
            std::fs::write(alvo, tabela(&linhas)).unwrap();
        }
        let sol: Vec<&Linha> = linhas.iter().filter(|l| l.de_solidos()).collect();
        assert!(sol.len() >= 40, "{}", sol.len());
        let faltas: Vec<String> = sol.iter().filter(|l| l.notas().contains(&Nota::Falta)).map(|l| format!("{} ({})", l.nome, l.obs)).collect();
        assert!(faltas.is_empty(), "comando de sólidos com falta: {faltas:?}");
        let sem_manip: Vec<&str> = sol.iter().filter(|l| l.manip == Nota::Parcial).map(|l| l.icone).collect();
        assert!(sem_manip.is_empty(), "comando de sólidos sem alça 3D: {sem_manip:?}");
        let fora: Vec<&Linha> = linhas.iter().filter(|l| !l.de_solidos()).collect();
        let c = |f: &dyn Fn(&Linha) -> bool| fora.iter().filter(|l| f(l)).count();
        assert!(c(&|l| l.gif != Nota::Ok) <= TETO_FORA_SEM_GIF, "mais comandos sem GIF que o teto");
        assert!(c(&|l| l.icone_nota != Nota::Ok) <= TETO_FORA_SEM_ICONE, "mais comandos sem ícone que o teto");
        assert!(c(&|l| l.tooltip != Nota::Ok) <= TETO_FORA_SEM_TOOLTIP, "mais comandos sem tooltip que o teto");
    }

    #[test]
    fn toda_alca_de_solidos_desenha_pelos_estados_comuns() {
        let sem = alcas_sem_estado();
        let novas: Vec<&(String, String)> = sem.iter().filter(|(a, f)| !SEM_ESTADO.iter().any(|(x, y)| x == a && y == f)).collect();
        assert!(novas.is_empty(), "alça nova sem EstadoAlca (repouso, pairando, ativa, travada, erro): {novas:?}");
        let sobrou: Vec<&(&str, &str)> = SEM_ESTADO.iter().filter(|(x, y)| !sem.iter().any(|(a, f)| a == x && f == y)).collect();
        assert!(sobrou.is_empty(), "já usam EstadoAlca: tire da lista SEM_ESTADO: {sobrou:?}");
    }

    #[test]
    fn tabela_tem_uma_linha_por_comando() {
        let linhas = levantar();
        let t = tabela(&linhas);
        assert_eq!(t.lines().filter(|l| l.starts_with("| ") && !l.starts_with("| Barra")).count(), linhas.len());
    }
}
