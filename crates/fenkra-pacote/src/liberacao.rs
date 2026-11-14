// SPDX-License-Identifier: LGPL-2.1-or-later
//! Pacote de liberação (B141): um ZIP com o que a fábrica e o fornecedor precisam de uma revisão
//! liberada, sem precisar do Fenkra para abrir.
//!
//! ```text
//! liberacao.zip
//! ├── manifesto.json                     projeto, data, autor, aviso de mudança e hash de cada arquivo
//! ├── step/<PN>_rev<R>.step              sólido de cada peça, na revisão liberada
//! ├── desenhos/<nome>.pdf                todas as folhas de cada desenho, em vetor
//! ├── desenhos/<nome>_f<N>.dxf           cada folha em DXF
//! ├── bom/<montagem>.csv | .json | .xml  lista estruturada, com PN, quantidade, estado e revisão
//! └── mudancas/registro.json, <ECN>.json pedidos e avisos de mudança (ECR/ECN)
//! ```
//!
//! Como funciona:
//! 1. as peças vêm do índice. Por padrão só entram as liberadas, **na versão da revisão** (o arquivo
//!    de trabalho pode já ter edição nova que ninguém liberou). Com um aviso de mudança escolhido,
//!    entram só as peças dele, na revisão "depois" de cada item;
//! 2. o STEP sai do sólido final recalculado da peça (o kernel grava num arquivo temporário e os
//!    bytes vão para o ZIP);
//! 3. desenhos e montagens são os `.fkd` e `.fka` das pastas do projeto. A lista de peças é a
//!    estruturada da montagem (`fenkra_montagem::bom`), com o estado e a revisão de cada peça do
//!    índice casados pelo arquivo;
//! 4. o que não deu (peça com erro, desenho ilegível) não derruba o pacote: vira aviso no manifesto,
//!    para quem recebe saber o que falta;
//! 5. o manifesto guarda o hash BLAKE3 de cada arquivo: quem recebe confere a integridade.

use fenkra_montagem::bom::{bom, BomLevel};
use fenkra_montagem::Assembly;
use fenkra_project::arvore_produto::{EstadoNo, TipoNo};
use fenkra_project::Project;
use serde_json::{json, Value};
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpcoesLiberacao {
    pub step: bool,
    pub pdf: bool,
    pub dxf: bool,
    pub bom_csv: bool,
    pub bom_json: bool,
    pub bom_xml: bool,
    /// só peças liberadas, na revisão; desligado, todas na versão atual (pacote de trabalho)
    pub so_liberadas: bool,
    /// aviso de mudança (ECN) cujas peças entram; `None` = pelas regras acima
    pub aviso: Option<String>,
}

impl Default for OpcoesLiberacao {
    fn default() -> Self {
        OpcoesLiberacao { step: true, pdf: true, dxf: true, bom_csv: true, bom_json: true, bom_xml: true, so_liberadas: true, aviso: None }
    }
}

#[derive(Clone, Debug, Default)]
pub struct ResumoLiberacao {
    /// nome no ZIP e tamanho
    pub arquivos: Vec<(String, usize)>,
    pub avisos: Vec<String>,
    pub bytes: usize,
}

/// Nome seguro para dentro do ZIP.
fn seguro(s: &str) -> String {
    let t: String = s.chars().map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' { c } else { '_' }).collect();
    if t.is_empty() {
        "sem_nome".into()
    } else {
        t
    }
}

fn xml(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn csv(s: &str) -> String {
    if s.contains([';', '"', '\n']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.into()
    }
}

/// STEP do sólido final da peça.
fn step_de(doc: &fenkra_core::Document) -> Result<Vec<u8>, String> {
    let forma = fenkra_solidos::final_shape(doc).ok_or("a peça não tem sólido")?;
    let tmp = std::env::temp_dir().join(format!("fenkra_lib_{}_{}.step", std::process::id(), blake3::hash(doc.to_json().as_bytes()).to_hex()));
    forma.export_step(&tmp.to_string_lossy()).map_err(|e| e.to_string())?;
    let b = std::fs::read(&tmp).map_err(|e| e.to_string());
    let _ = std::fs::remove_file(&tmp);
    b
}

/// Linhas da lista de peças de uma montagem: item, nível, PN, nome, quantidade, estado e revisão.
pub fn linhas_bom(p: &Project, montagem: &Path) -> Result<Vec<Value>, String> {
    let txt = std::fs::read_to_string(montagem).map_err(|e| format!("{}: {e}", montagem.display()))?;
    let a = Assembly::from_json(&txt)?;
    let dir = montagem.parent().unwrap_or(Path::new("."));
    let arv = p.arvore_produto();
    Ok(bom(&a, Some(dir), BomLevel::Estruturada)
        .into_iter()
        .map(|r| {
            let no = arv.no_do_arquivo(&dir.join(&r.path)).or_else(|| arv.no_do_arquivo(&r.path)).map(|i| &arv.nos[i]);
            let estado = match no.map(|n| n.estado) {
                Some(EstadoNo::Documento(e)) => e.label().to_string(),
                Some(EstadoNo::Derivado { liberadas, total }) => format!("{liberadas}/{total} liberadas"),
                _ => "fora do projeto".into(),
            };
            let pn = if r.pn.is_empty() { no.map(|n| n.pn.clone()).unwrap_or_default() } else { r.pn.clone() };
            json!({
                "item": r.item, "nivel": r.depth, "pn": pn, "nome": r.name, "quantidade": r.qty,
                "tipo": if r.sub { "montagem" } else { "peca" },
                "estado": estado, "revisao": no.and_then(|n| n.revisao.clone()).unwrap_or_default(),
                "arquivo": r.key,
            })
        })
        .collect())
}

fn bom_csv(l: &[Value]) -> String {
    let mut s = String::from("Item;Nível;PN;Nome;Quantidade;Tipo;Estado;Revisão\n");
    for r in l {
        let t = |k: &str| r[k].as_str().map(str::to_string).unwrap_or_else(|| r[k].to_string());
        s += &[t("item"), t("nivel"), t("pn"), t("nome"), t("quantidade"), t("tipo"), t("estado"), t("revisao")].iter().map(|x| csv(x)).collect::<Vec<_>>().join(";");
        s.push('\n');
    }
    s
}

fn bom_xml(nome: &str, l: &[Value]) -> String {
    let mut s = format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<lista montagem=\"{}\">\n", xml(nome));
    for r in l {
        let t = |k: &str| xml(&r[k].as_str().map(str::to_string).unwrap_or_else(|| r[k].to_string()));
        s += &format!(
            "  <item numero=\"{}\" nivel=\"{}\" pn=\"{}\" quantidade=\"{}\" tipo=\"{}\" estado=\"{}\" revisao=\"{}\">{}</item>\n",
            t("item"),
            t("nivel"),
            t("pn"),
            t("quantidade"),
            t("tipo"),
            t("estado"),
            t("revisao"),
            t("nome")
        );
    }
    s + "</lista>\n"
}

/// Monta o pacote de liberação do projeto e grava em `destino`.
pub fn pacote_liberacao(p: &Project, destino: &Path, o: &OpcoesLiberacao) -> Result<ResumoLiberacao, String> {
    let mut arquivos: Vec<(String, Vec<u8>)> = vec![];
    let mut avisos: Vec<String> = vec![];
    let registro = p.mudancas().map_err(|e| e.to_string())?;
    let aviso = match &o.aviso {
        Some(n) => Some(registro.aviso(n).ok_or_else(|| format!("aviso de mudança {n} não existe"))?.clone()),
        None => None,
    };
    // peças: (id, rótulo da revisão, hash da versão)
    let mut pecas: Vec<(u64, String, String)> = vec![];
    for e in &p.index.documents {
        let escolha = match &aviso {
            Some(a) => a.itens.iter().find(|i| i.id == e.id).and_then(|i| e.revisions.iter().find(|r| r.label == i.revisao_depois)).map(|r| (r.label.clone(), r.hash.clone())),
            // a última revisão liberada vale mesmo com edição nova por cima (o estado volta a "em
            // trabalho" ao salvar, mas a revisão A continua sendo a que a fábrica usa); obsoleta não sai
            None if o.so_liberadas => (e.state != fenkra_project::DocState::Obsoleto).then(|| e.current_revision()).flatten().map(|r| (r.label.clone(), r.hash.clone())),
            None => e.versions.last().map(|v| (e.current_revision().filter(|r| r.hash == v.hash).map(|r| r.label.clone()).unwrap_or_else(|| format!("v{}", e.versions.len())), v.hash.clone())),
        };
        if let Some((rev, hash)) = escolha {
            pecas.push((e.id, rev, hash));
        }
    }
    if pecas.is_empty() {
        return Err(if o.so_liberadas && aviso.is_none() { "nenhuma peça liberada no projeto".into() } else { "nenhuma peça para o pacote".into() });
    }
    let mut lista_pecas = vec![];
    for (id, rev, hash) in &pecas {
        let e = p.entry(*id).map_err(|x| x.to_string())?;
        let mut item = json!({"id": id, "pn": e.pn, "nome": e.name, "revisao": rev, "versao": hash, "estado": e.state.label()});
        if o.step {
            match p.load_version(hash).map_err(|x| x.to_string()).and_then(|d| step_de(&d)) {
                Ok(b) => {
                    let nome = format!("step/{}_rev{}.step", seguro(if e.pn.is_empty() { &e.name } else { &e.pn }), seguro(rev));
                    item["step"] = json!(nome);
                    arquivos.push((nome, b));
                }
                Err(x) => avisos.push(format!("STEP de {} ({}): {x}", e.pn, e.name)),
            }
        }
        lista_pecas.push(item);
    }
    // desenhos e montagens das pastas do projeto
    let mut docs = vec![];
    for pasta in p.pastas_de_busca() {
        fenkra_project::relacoes::listar_documentos(&pasta, 6, &mut docs);
    }
    docs.sort();
    docs.dedup();
    if o.pdf || o.dxf {
        for f in docs.iter().filter(|f| f.extension().is_some_and(|x| x == "fkd")) {
            let stem = seguro(&f.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default());
            let d = match std::fs::read_to_string(f).map_err(|e| e.to_string()).and_then(|t| fenkra_desenho::Drawing::from_json(&t)) {
                Ok(d) => d,
                Err(e) => {
                    avisos.push(format!("desenho {}: {e}", f.display()));
                    continue;
                }
            };
            let mut modelos = fenkra_desenho::source::Models::new(f.parent().map(Path::to_path_buf));
            let paginas: Vec<_> = d.sheets.iter().map(|s| (s.size(), fenkra_desenho::layout::sheet(&d, s.id, &mut modelos))).collect();
            if paginas.is_empty() {
                avisos.push(format!("desenho {stem}: sem folhas"));
                continue;
            }
            if o.pdf {
                arquivos.push((format!("desenhos/{stem}.pdf"), fenkra_desenho::export::pdf(&paginas)));
            }
            if o.dxf {
                for (k, (_, itens)) in paginas.iter().enumerate() {
                    arquivos.push((format!("desenhos/{stem}_f{}.dxf", k + 1), fenkra_desenho::export::dxf(itens).into_bytes()));
                }
            }
        }
    }
    if o.bom_csv || o.bom_json || o.bom_xml {
        let arv = p.arvore_produto();
        for &r in arv.raizes.iter().filter(|&&r| arv.nos[r].tipo == TipoNo::Montagem) {
            let f = &arv.nos[r].arquivo;
            let stem = seguro(&f.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default());
            match linhas_bom(p, f) {
                Ok(l) => {
                    if o.bom_csv {
                        arquivos.push((format!("bom/{stem}.csv"), bom_csv(&l).into_bytes()));
                    }
                    if o.bom_json {
                        let j = json!({"montagem": arv.nos[r].nome, "pn": arv.nos[r].pn, "itens": l});
                        arquivos.push((format!("bom/{stem}.json"), serde_json::to_vec_pretty(&j).map_err(|e| e.to_string())?));
                    }
                    if o.bom_xml {
                        arquivos.push((format!("bom/{stem}.xml"), bom_xml(&arv.nos[r].nome, &l).into_bytes()));
                    }
                }
                Err(e) => avisos.push(format!("lista de {stem}: {e}")),
            }
        }
    }
    // registro de mudanças
    if !registro.pedidos.is_empty() || !registro.avisos.is_empty() {
        arquivos.push(("mudancas/registro.json".into(), serde_json::to_vec_pretty(&registro).map_err(|e| e.to_string())?));
    }
    if let Some(a) = &aviso {
        let mut j = serde_json::to_value(a).map_err(|e| e.to_string())?;
        if let Some(ped) = registro.pedido(&a.pedido) {
            j["pedido_detalhe"] = serde_json::to_value(ped).map_err(|e| e.to_string())?;
        }
        arquivos.push((format!("mudancas/{}.json", seguro(&a.numero)), serde_json::to_vec_pretty(&j).map_err(|e| e.to_string())?));
    }
    let lista: Vec<Value> = arquivos.iter().map(|(n, b)| json!({"nome": n, "bytes": b.len(), "blake3": blake3::hash(b).to_hex().to_string()})).collect();
    let manifesto = json!({
        "formato": "fenkra-liberacao", "versao": 1,
        "projeto": p.index.name, "autor": p.autor,
        "data": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0),
        "aviso": o.aviso, "so_liberadas": o.so_liberadas,
        "pecas": lista_pecas, "arquivos": lista, "avisos": avisos,
    });
    let mut tudo = vec![("manifesto.json".to_string(), serde_json::to_vec_pretty(&manifesto).map_err(|e| e.to_string())?)];
    tudo.extend(arquivos);
    let zip = fenkra_project::pacote::escrever_zip(&tudo)?;
    if let Some(d) = destino.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    std::fs::write(destino, &zip).map_err(|e| format!("{}: {e}", destino.display()))?;
    Ok(ResumoLiberacao { arquivos: tudo.iter().map(|(n, b)| (n.clone(), b.len())).collect(), avisos, bytes: zip.len() })
}
