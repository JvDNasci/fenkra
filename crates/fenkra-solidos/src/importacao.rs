//! Importação de STEP (B229). O corpo lido entra no documento como geometria isolada, seguido de um
//! comando de corpo ligado; assim a árvore continua normal e aceita comandos depois dele. O selo de
//! origem guarda de qual arquivo o documento nasceu.

use fenkra_core::{Document, FeatureKind, LinkedBody, LinkedGeo, SeloOrigem};
use fenkra_kernel::Shape;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// Documento pronto para abrir ou entrar no projeto, com o relatório do que foi lido.
#[derive(Clone, Debug)]
pub struct Importacao {
    pub doc: Document,
    pub relatorio: String,
}

/// Lê um `.step` ou `.stp` e monta o documento com o selo de origem.
pub fn importar_step(caminho: &str) -> Result<Importacao, String> {
    let p = Path::new(caminho);
    let ext = p.extension().map(|e| e.to_string_lossy().to_lowercase());
    if !matches!(ext.as_deref(), Some("step" | "stp")) {
        return Err("importe um arquivo .step ou .stp".into());
    }
    let bytes = std::fs::read(p).map_err(|e| format!("não foi possível ler '{caminho}': {e}"))?;
    let (forma, info) = Shape::import_step_info(caminho).map_err(|e| e.0)?;
    let brep = forma.to_brep().map_err(|e| e.0)?;

    let nome = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let base = p.file_stem().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "Importado".into());
    let mut doc = Document::new(&base);
    let geo = doc.add_link("importação", caminho, &base, LinkedGeo::Surface(brep));
    // a cópia vira geometria própria: o arquivo de fora não é mais lido a cada cálculo
    if let Some(ext) = doc.external.last().map(|e| e.id) {
        doc.isolate_link(ext);
    }
    doc.add_feature(FeatureKind::LinkedBody(LinkedBody { geo }));

    let selo = SeloOrigem {
        nome,
        hash: blake3::hash(&bytes).to_hex().to_string(),
        data: SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0),
        esquema: info.esquema,
        unidades: info.unidades,
        derivado_de: None,
    };
    let m = forma.mass();
    let relatorio = format!(
        "Relatório de conversão\nArquivo: {}\nHash: {}\nTamanho: {} bytes\nEsquema: {}\nUnidades lidas: {}\nFaces: {}\nVolume: {:.3} mm³\nÁrea: {:.3} mm²\n",
        selo.nome,
        selo.hash,
        bytes.len(),
        selo.esquema,
        selo.unidades,
        forma.face_count(),
        m.volume,
        m.area,
    );
    doc.origem = Some(selo);
    Ok(Importacao { doc, relatorio })
}
