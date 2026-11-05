//! B201: todo fonte (.rs, .cpp, .h, .inc) abre com o identificador da licença, e arquivo novo sem ele
//! reprova. O cabeçalho é aplicado por `python scripts/spdx.py`.

use std::fs;
use std::path::{Path, PathBuf};

const CABECALHO: &str = "// SPDX-License-Identifier: LGPL-2.1-or-later";

/// Caminhos ainda sem o cabeçalho: outras frentes estavam editando quando a regra entrou. Depois de
/// rodar o script nelas, esvazie esta lista.
const PENDENTES: [&str; 6] = [
    "crates/fenkra-app/",
    "crates/fenkra-core/src/pmi.rs",
    "crates/fenkra-desenho/",
    "crates/fenkra-montagem/",
    "crates/fenkra-solidos/",
    "tools/",
];

fn raiz() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().expect("raiz do repositório")
}

fn coletar(dir: &Path, saida: &mut Vec<PathBuf>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            if !matches!(e.file_name().to_str(), Some("target" | ".git" | "__pycache__" | "node_modules")) {
                coletar(&p, saida);
            }
        } else if matches!(p.extension().and_then(|x| x.to_str()), Some("rs" | "cpp" | "h" | "inc")) {
            saida.push(p);
        }
    }
}

fn tem_cabecalho(p: &Path) -> bool {
    let dados = fs::read(p).unwrap_or_default();
    dados.strip_prefix(b"\xef\xbb\xbf").unwrap_or(&dados).starts_with(CABECALHO.as_bytes())
}

#[test]
fn todo_fonte_tem_cabecalho_spdx() {
    let raiz = raiz();
    let mut fontes = Vec::new();
    for pasta in ["crates", "tools"] {
        coletar(&raiz.join(pasta), &mut fontes);
    }
    assert!(fontes.len() > 50, "a varredura não achou os fontes: {}", fontes.len());
    let relativo = |p: &Path| p.strip_prefix(&raiz).unwrap().to_string_lossy().replace('\\', "/");
    let faltando: Vec<String> =
        fontes.iter().filter(|p| !tem_cabecalho(p)).map(|p| relativo(p)).filter(|r| !PENDENTES.iter().any(|x| r.starts_with(x))).collect();
    assert!(faltando.is_empty(), "sem cabeçalho SPDX (rode python scripts/spdx.py):\n{}", faltando.join("\n"));
}

#[test]
fn cabecalho_acompanha_a_licenca_do_workspace() {
    let manifesto = fs::read_to_string(raiz().join("Cargo.toml")).unwrap();
    let licenca = CABECALHO.rsplit(' ').next().unwrap();
    assert!(manifesto.contains(&format!("license = \"{licenca}\"")));
}
