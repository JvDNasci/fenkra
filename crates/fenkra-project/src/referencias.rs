// SPDX-License-Identifier: LGPL-2.1-or-later
//! Identidade de documento e resolução de referências entre arquivos (B66).
//!
//! Uma referência (componente de montagem, link entre peças, vista de desenho) guarda **uid +
//! caminho relativo ao documento que referencia**. O caminho pode subir pastas (`../comum/x.fkp`);
//! só é absoluto quando os dois arquivos estão em unidades diferentes (outro drive), onde não existe
//! caminho relativo. Para abrir, a ordem é fixa:
//!
//! 1. o caminho gravado: se o arquivo existe e tem o mesmo uid, é ele;
//! 2. se o arquivo existe mas tem **outro** uid, é um impostor (alguém salvou outra peça com o mesmo
//!    nome): nunca é usado em silêncio, o chamador mostra o diálogo Resolver;
//! 3. se o caminho não serve, procura nas pastas de busca (por exemplo a pasta do projeto) um arquivo
//!    com o uid gravado: foi movido ou renomeado;
//! 4. sem nada: não encontrado, diálogo Resolver.
//!
//! Referências antigas sem uid (alphas anteriores) valem pelo caminho, como sempre valeram.

use std::path::{Component, Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ref {
    pub uid: String,
    pub caminho: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Resolucao {
    /// o caminho gravado aponta para o documento certo
    Encontrado(PathBuf),
    /// o caminho gravado não serve, mas o uid foi achado em outro lugar (movido ou renomeado)
    Movido(PathBuf),
    /// o arquivo no caminho gravado tem outro uid: não use sem o usuário decidir
    UidDiferente { caminho: PathBuf, uid_achado: String, alternativa: Option<PathBuf> },
    /// mais de um arquivo tem o uid procurado (cópia do arquivo no disco): o usuário escolhe
    Varios(Vec<PathBuf>),
    NaoEncontrado,
}

/// Tira `.` e resolve `..` sem tocar o disco.
fn normalizar(p: &Path) -> Vec<Component<'_>> {
    let mut out: Vec<Component> = Vec::new();
    for c in p.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                if matches!(out.last(), Some(Component::Normal(_))) {
                    out.pop();
                } else {
                    out.push(c);
                }
            }
            _ => out.push(c),
        }
    }
    out
}

fn texto(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

/// Caminho de `alvo` visto a partir da pasta `base` (a pasta do documento que referencia).
/// Sobe com `..` quando preciso; devolve o caminho absoluto só se não houver relação (drives diferentes)
/// ou se algum dos dois não for absoluto.
pub fn caminho_relativo(base: &Path, alvo: &Path) -> String {
    let (b, a) = (normalizar(base), normalizar(alvo));
    let absolutos = base.is_absolute() && alvo.is_absolute();
    let raiz = |v: &[Component]| v.first().map(|c| matches!(c, Component::Prefix(_) | Component::RootDir)).unwrap_or(false);
    if !absolutos || !raiz(&b) || !raiz(&a) {
        return texto(alvo);
    }
    // a unidade (prefixo) diferente impede caminho relativo
    if let (Some(Component::Prefix(pb)), Some(Component::Prefix(pa))) = (b.first(), a.first()) {
        if !pb.as_os_str().eq_ignore_ascii_case(pa.as_os_str()) {
            return texto(alvo);
        }
    } else if matches!(b.first(), Some(Component::Prefix(_))) != matches!(a.first(), Some(Component::Prefix(_))) {
        return texto(alvo);
    }
    let igual = |x: &Component, y: &Component| match (x, y) {
        (Component::Normal(p), Component::Normal(q)) => {
            if cfg!(windows) {
                p.to_string_lossy().eq_ignore_ascii_case(&q.to_string_lossy())
            } else {
                p == q
            }
        }
        (x, y) => x == y || matches!((x, y), (Component::Prefix(_), Component::Prefix(_))),
    };
    let mut i = 0;
    while i < b.len() && i < a.len() && igual(&b[i], &a[i]) {
        i += 1;
    }
    let mut partes: Vec<String> = vec![];
    for _ in i..b.len() {
        partes.push("..".into());
    }
    for c in &a[i..] {
        partes.push(c.as_os_str().to_string_lossy().into_owned());
    }
    if partes.is_empty() {
        ".".into()
    } else {
        partes.join("/")
    }
}

impl Ref {
    /// Referência de `doc_dir` (pasta do documento que referencia) para o arquivo `alvo` com o uid dado.
    pub fn para(doc_dir: &Path, alvo: &Path, uid: &str) -> Ref {
        Ref { uid: uid.into(), caminho: caminho_relativo(doc_dir, alvo) }
    }

    /// Caminho completo no disco, a partir da pasta do documento que referencia.
    pub fn caminho_completo(&self, doc_dir: &Path) -> PathBuf {
        let p = Path::new(&self.caminho);
        if p.is_absolute() {
            p.to_path_buf()
        } else {
            doc_dir.join(p)
        }
    }
}

/// uid gravado em um arquivo `.fkp`, `.fka` ou `.fkd` (vazio se o arquivo é antigo ou ilegível).
pub fn ler_uid(arquivo: &Path) -> String {
    std::fs::read_to_string(arquivo).map(|s| fenkra_core::formato::uid_do_json(&s)).unwrap_or_default()
}

fn procurar(dir: &Path, uid: &str, fundo: u32, achados: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        let nome = e.file_name().to_string_lossy().into_owned();
        if p.is_dir() {
            // as versões imutáveis do projeto repetem o uid de propósito: não são candidatas
            if fundo > 0 && !nome.starts_with('.') {
                procurar(&p, uid, fundo - 1, achados);
            }
        } else if matches!(p.extension().and_then(|x| x.to_str()), Some("fkp" | "fka" | "fkd")) && ler_uid(&p) == uid {
            achados.push(p);
        }
    }
}

/// Resolve uma referência na ordem definida no topo do módulo. `busca` são as pastas onde procurar
/// um documento movido (normalmente a pasta do projeto); a busca desce até 6 níveis.
pub fn resolver(doc_dir: &Path, r: &Ref, busca: &[PathBuf]) -> Resolucao {
    let alvo = r.caminho_completo(doc_dir);
    let existe = alvo.is_file();
    let procurar_uid = |excluir: Option<&Path>| -> Vec<PathBuf> {
        let mut v = vec![];
        if !r.uid.is_empty() {
            for d in busca {
                procurar(d, &r.uid, 6, &mut v);
            }
            v.sort();
            v.dedup();
            v.retain(|p| Some(p.as_path()) != excluir);
        }
        v
    };
    if existe {
        if r.uid.is_empty() {
            return Resolucao::Encontrado(alvo);
        }
        let achado = ler_uid(&alvo);
        if achado == r.uid {
            return Resolucao::Encontrado(alvo);
        }
        let alt = procurar_uid(Some(&alvo)).into_iter().next();
        return Resolucao::UidDiferente { caminho: alvo, uid_achado: achado, alternativa: alt };
    }
    let mut v = procurar_uid(None);
    match v.len() {
        0 => Resolucao::NaoEncontrado,
        1 => Resolucao::Movido(v.remove(0)),
        _ => Resolucao::Varios(v),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(nome: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("fenkra-refs-{nome}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn peca(dir: &Path, arquivo: &str, uid: &str) -> PathBuf {
        let p = dir.join(arquivo);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, format!(r#"{{"format_version": 1, "uid": "{uid}", "name": "x"}}"#)).unwrap();
        p
    }

    #[test]
    fn caminho_relativo_sobe_pastas_e_so_vira_absoluto_entre_drives() {
        let r = |b: &str, a: &str| caminho_relativo(Path::new(b), Path::new(a));
        if cfg!(windows) {
            assert_eq!(r("C:/proj/montagens", "C:/proj/montagens/sub/a.fkp"), "sub/a.fkp");
            assert_eq!(r("C:/proj/montagens", "C:/proj/comum/a.fkp"), "../comum/a.fkp");
            assert_eq!(r("C:/proj/a/b", "C:/proj/x.fkp"), "../../x.fkp");
            assert_eq!(r("C:/proj", "c:/PROJ/x.fkp"), "x.fkp");
            assert_eq!(r("C:/proj", "D:/outro/x.fkp"), "D:/outro/x.fkp");
        } else {
            assert_eq!(r("/proj/montagens", "/proj/comum/a.fkp"), "../comum/a.fkp");
            assert_eq!(r("/proj/a/b", "/proj/x.fkp"), "../../x.fkp");
        }
        assert_eq!(r("relativo", "outro/x.fkp"), "outro/x.fkp", "sem base absoluta não inventa relação");
    }

    #[test]
    fn caminho_relativo_resolve_de_volta_para_o_mesmo_arquivo() {
        let d = tmp("ida-volta");
        let a = peca(&d, "comum/a.fkp", "u1");
        let base = d.join("montagens").join("m1");
        std::fs::create_dir_all(&base).unwrap();
        let rel = caminho_relativo(&base, &a);
        assert_eq!(rel, "../../comum/a.fkp");
        let r = Ref { uid: "u1".into(), caminho: rel };
        assert_eq!(resolver(&base, &r, &[]), Resolucao::Encontrado(base.join("../../comum/a.fkp")));
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn uid_igual_no_caminho_gravado_resolve() {
        let d = tmp("ok");
        peca(&d, "a.fkp", "u1");
        let r = Ref { uid: "u1".into(), caminho: "a.fkp".into() };
        assert_eq!(resolver(&d, &r, &[d.clone()]), Resolucao::Encontrado(d.join("a.fkp")));
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn outro_uid_no_mesmo_caminho_nunca_e_usado_em_silencio() {
        let d = tmp("impostor");
        peca(&d, "a.fkp", "u-impostor");
        peca(&d, "guardada/original.fkp", "u1");
        let r = Ref { uid: "u1".into(), caminho: "a.fkp".into() };
        match resolver(&d, &r, &[d.clone()]) {
            Resolucao::UidDiferente { uid_achado, alternativa, .. } => {
                assert_eq!(uid_achado, "u-impostor");
                assert_eq!(alternativa, Some(d.join("guardada").join("original.fkp")));
            }
            outro => panic!("esperava UidDiferente, veio {outro:?}"),
        }
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn arquivo_movido_ou_renomeado_e_achado_pelo_uid() {
        let d = tmp("movido");
        peca(&d, "novas/renomeada.fkp", "u1");
        let r = Ref { uid: "u1".into(), caminho: "antiga.fkp".into() };
        assert_eq!(resolver(&d, &r, &[d.clone()]), Resolucao::Movido(d.join("novas").join("renomeada.fkp")));
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn copia_no_disco_gera_varios_e_ausente_gera_nao_encontrado() {
        let d = tmp("varios");
        peca(&d, "x/a.fkp", "u1");
        peca(&d, "y/a copia.fkp", "u1");
        let r = Ref { uid: "u1".into(), caminho: "sumiu.fkp".into() };
        assert!(matches!(resolver(&d, &r, &[d.clone()]), Resolucao::Varios(v) if v.len() == 2));
        let r2 = Ref { uid: "nao-existe".into(), caminho: "sumiu.fkp".into() };
        assert_eq!(resolver(&d, &r2, &[d.clone()]), Resolucao::NaoEncontrado);
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn referencia_antiga_sem_uid_vale_pelo_caminho() {
        let d = tmp("legado");
        peca(&d, "a.fkp", "");
        let r = Ref { uid: String::new(), caminho: "a.fkp".into() };
        assert_eq!(resolver(&d, &r, &[]), Resolucao::Encontrado(d.join("a.fkp")));
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn versoes_imutaveis_do_projeto_nao_sao_candidatas() {
        let d = tmp("versoes");
        peca(&d, ".versoes/abc.fkp", "u1");
        let r = Ref { uid: "u1".into(), caminho: "sumiu.fkp".into() };
        assert_eq!(resolver(&d, &r, &[d.clone()]), Resolucao::NaoEncontrado);
        let _ = std::fs::remove_dir_all(d);
    }
}
