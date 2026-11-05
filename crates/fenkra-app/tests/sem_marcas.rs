//! Regra do projeto (B310): nenhum software proprietário é citado em código, interface ou
//! documentação, nem como compatibilidade. As fontes consultadas ficam só nos arquivos de fontes.
//! Software livre confiável pode ser citado e não está na lista.

use std::fs;
use std::path::{Path, PathBuf};

/// Únicos lugares onde nomes de terceiros podem aparecer (fontes consultadas e sua geração).
const ARQUIVOS_DE_FONTES: [&str; 3] = ["docs/arquitetura/procedencia.md", "docs/benchmark/fontes.md", "scripts/benchmark/fontes.py"];

/// Exceção temporária: arquivo gerado, em edição por outra bancada (PMI 3D). Sai da lista quando
/// `scripts/benchmark/gerar.py` for rodado de novo, pois o gerador já escreve sem marcas.
const EM_EDICAO: [&str; 1] = ["docs/benchmark/pmi-3d.md"];

/// Termos sem distinção de maiúsculas (nomes que não são palavras comuns).
const TERMOS: [&str; 22] = [
    "solidworks", "solid edge", "solidedge", "catia", "autodesk", "autocad", "onshape", "dassault", "siemens", "parasolid", "3dexperience",
    "enovia", "windchill", "teamcenter", "keyshot", "ansys", "abaqus", "bricscad", "mastercam", "sketchup", "fusion 360", "hole wizard",
];

/// Termos que só contam com a grafia de nome próprio (também são palavras comuns).
const NOMES_PROPRIOS: [&str; 4] = ["Inventor", "Rhino", "Creo", "Toolbox"];

fn raiz() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().expect("raiz do repositório")
}

fn coletar(dir: &Path, saida: &mut Vec<PathBuf>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        let nome = e.file_name().to_string_lossy().to_string();
        if p.is_dir() {
            if matches!(nome.as_str(), "target" | ".git" | "__pycache__" | "node_modules") {
                continue;
            }
            coletar(&p, saida);
        } else if matches!(p.extension().and_then(|x| x.to_str()), Some("rs" | "inc" | "h" | "cpp" | "md" | "html" | "json" | "py" | "ps1" | "toml" | "txt" | "svg")) {
            saida.push(p);
        }
    }
}

fn letra(c: Option<char>) -> bool {
    c.is_some_and(|c| c.is_alphanumeric() || c == '_')
}

/// Acha `termo` como palavra inteira em `texto`.
fn tem_palavra(texto: &str, termo: &str) -> bool {
    texto.match_indices(termo).any(|(i, _)| {
        let antes = texto[..i].chars().next_back();
        let depois = texto[i + termo.len()..].chars().next();
        !letra(antes) && !letra(depois)
    })
}

fn marcas_em(texto: &str) -> Vec<String> {
    let min = texto.to_lowercase();
    let mut v: Vec<String> = TERMOS.iter().filter(|t| tem_palavra(&min, t)).map(|t| t.to_string()).collect();
    v.extend(NOMES_PROPRIOS.iter().filter(|t| tem_palavra(texto, t)).map(|t| t.to_string()));
    v
}

#[test]
fn nenhuma_marca_fora_dos_arquivos_de_fontes() {
    let raiz = raiz();
    let mut arquivos = Vec::new();
    for d in ["crates", "docs", "scripts"] {
        coletar(&raiz.join(d), &mut arquivos);
    }
    arquivos.push(raiz.join("README.md"));
    assert!(arquivos.len() > 100, "a varredura achou poucos arquivos: {}", arquivos.len());
    let este = Path::new(file!()).file_name().unwrap().to_owned();
    let mut achados = Vec::new();
    for p in arquivos {
        let rel = p.strip_prefix(&raiz).unwrap().to_string_lossy().replace(char::from(92), "/");
        if ARQUIVOS_DE_FONTES.contains(&rel.as_str()) || EM_EDICAO.contains(&rel.as_str()) || p.file_name() == Some(&este) {
            continue;
        }
        let Ok(texto) = fs::read_to_string(&p) else { continue };
        let m = marcas_em(&texto);
        if !m.is_empty() {
            achados.push(format!("{rel}: {}", m.join(", ")));
        }
    }
    assert!(achados.is_empty(), "marcas fora dos arquivos de fontes ({}):\n{}", achados.len(), achados.join("\n"));
}

#[test]
fn detector_acha_marca_e_ignora_palavra_comum() {
    assert!(!marcas_em(&["o ", "CAT", "IA", " V5"].concat()).is_empty());
    assert!(marcas_em("o inventor da roda; fusão de sólidos; fusion de malhas").is_empty());
}
