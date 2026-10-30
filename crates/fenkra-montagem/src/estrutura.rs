//! Estrutura Pai → montagem: cada grupo de peças iguais da peça com membros estruturais vira uma
//! peça `.fkp` (o corpo no referencial local, como link para o pai) e a montagem `.fka` leva uma
//! instância fixa por membro, agrupadas por item.

use crate::*;
use fenkra_core::{Document, FeatureKind, LinkState, LinkedBody, LinkedGeo};
use fenkra_solidos::StructItem;
use std::path::{Path, PathBuf};

/// Peça gerada para um grupo de peças iguais.
#[derive(Clone, Debug, PartialEq)]
pub struct GrupoGerado {
    pub item: usize,
    /// nome do perfil ("Tubo quadrado 40x3", "Placa", "Reforço")
    pub nome: String,
    pub arquivo: PathBuf,
    /// quantidade de instâncias na montagem
    pub quantidade: usize,
}

/// Arquivos gerados por `estrutura_para_montagem`.
#[derive(Clone, Debug, PartialEq)]
pub struct Gerado {
    pub montagem: PathBuf,
    pub grupos: Vec<GrupoGerado>,
}

/// Troca o que não é letra, número ou hífen por "_" (nome de arquivo).
fn limpo(t: &str) -> String {
    t.chars().map(|c| if c.is_alphanumeric() || c == '-' { c } else { '_' }).collect()
}

/// Caminho de `para` visto da pasta `de`, com "/" (outro disco: o absoluto).
fn caminho_relativo(de: &Path, para: &Path) -> String {
    let de = std::fs::canonicalize(de).unwrap_or_else(|_| de.to_path_buf());
    let para = std::fs::canonicalize(para).unwrap_or_else(|_| para.to_path_buf());
    let (a, b): (Vec<_>, Vec<_>) = (de.components().collect(), para.components().collect());
    let n = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    if n == 0 {
        return para.to_string_lossy().trim_start_matches(r"\\?\").replace('\\', "/");
    }
    let mut partes: Vec<String> = vec!["..".into(); a.len() - n];
    partes.extend(b[n..].iter().map(|c| c.as_os_str().to_string_lossy().into_owned()));
    partes.join("/")
}

/// Gera, em `saida`, uma peça por grupo de peças iguais da peça `pai` e a montagem com uma
/// instância fixa por membro (posicionada pelo referencial do membro).
pub fn estrutura_para_montagem(pai: &Path, saida: &Path) -> Result<Gerado, String> {
    let txt = std::fs::read_to_string(pai).map_err(|e| format!("{}: {e}", pai.display()))?;
    let doc = Document::from_json(&txt).map_err(|e| format!("{}: {e}", pai.display()))?;
    let grupos = fenkra_solidos::structural_groups(&doc);
    if grupos.is_empty() {
        return Err("a peça não tem membros estruturais calculados".into());
    }
    std::fs::create_dir_all(saida).map_err(|e| format!("{}: {e}", saida.display()))?;
    let rel = caminho_relativo(saida, pai);
    // nome da montagem = nome da peça (no projeto o arquivo se chama só pelo número)
    let stem = limpo(&doc.name);
    let stem = if stem.is_empty() { "estrutura".to_string() } else { stem };
    let mut asm = Assembly::new(&doc.name);
    asm.props.description = doc.props.description.clone();
    let mut gerado = Gerado { montagem: saida.join(format!("{stem}.fka")), grupos: vec![] };

    for g in &grupos {
        let comp = if g.signature.kind == StructItem::Member { format!(" L={} mm", g.signature.length_mm()) } else { String::new() };
        let desc = format!("{}{comp}", g.name);
        let nome = format!("item{:02}_{}{}", g.item, limpo(&g.name), if comp.is_empty() { String::new() } else { format!("_L{}", g.signature.length_mm()) });
        let arquivo = format!("{nome}.fkp");

        // peça: o corpo do membro no referencial local, como cópia ligada ao pai
        let brep = g.shape.to_brep().map_err(|e| e.0)?;
        let mut p = Document::new(&nome);
        p.props.description = desc.clone();
        p.props.material = doc.props.material.clone();
        if !doc.props.pn.is_empty() {
            p.props.pn = format!("{}-{:02}", doc.props.pn, g.item);
        }
        p.props.custom.push(("origem".into(), "estrutura_pai".into()));
        p.props.custom.push(("estrutura_grupo".into(), g.signature.key()));
        p.props.custom.push(("estrutura_item".into(), g.item.to_string()));
        let geo = p.add_link(&doc.name, &rel, &format!("item {:02}", g.item), LinkedGeo::Surface(brep));
        // a publicação "item NN" não existe no pai: 'atualizar links' a marcaria como quebrada.
        // O link nasce congelado e a atualização é gerar a montagem de novo.
        if let Some(e) = p.external.last_mut() {
            e.state = LinkState::Congelado;
        }
        p.add_feature(FeatureKind::LinkedBody(LinkedBody { geo }));
        let destino = saida.join(&arquivo);
        std::fs::write(&destino, p.to_json()).map_err(|e| format!("{}: {e}", destino.display()))?;

        // montagem: grupo do item e uma instância fixa por membro
        let gid = asm.add_group(&format!("Item {:02} · {desc}", g.item), None);
        for m in &g.members {
            let placement = Placement { origin: m.frame.origin, rot: [m.frame.x, m.frame.y(), m.frame.normal] };
            let id = asm.add_component(&format!("{}.{}", doc.name_of(m.feature), m.index + 1), NodeKind::Part { path: arquivo.clone() }, placement, Some(gid));
            if let Some(n) = asm.node_mut(id) {
                n.fixed = true;
            }
        }
        gerado.grupos.push(GrupoGerado { item: g.item, nome: g.name.clone(), arquivo: destino, quantidade: g.members.len() });
    }
    asm.assign_items(Some(saida));
    std::fs::write(&gerado.montagem, asm.to_json()).map_err(|e| format!("{}: {e}", gerado.montagem.display()))?;
    Ok(gerado)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::load::PartCache;
    use fenkra_core::{CornerMode, CurveRef, PointRef, RefKind, SectionKind, StructMember};

    fn quadro() -> Document {
        let s = |x: &str| x.to_string();
        let mut d = Document::new("Quadro");
        let pts = [[0., 0., 0.], [400., 0., 0.], [400., 300., 0.], [0., 300., 0.]];
        let mut paths = vec![];
        for i in 0..4 {
            let l = d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed(pts[i]), b: PointRef::Fixed(pts[(i + 1) % 4]), start: s("0"), end: s("0") });
            paths.push(CurveRef::Ref(l));
        }
        d.add_feature(FeatureKind::StructMember(StructMember {
            paths,
            family: s("Tubo quadrado"),
            designation: s("40x3"),
            kind: SectionKind::TuboQuadrado,
            dims: vec![s("40"), s("3")],
            anchor: 0,
            angle: s("0"),
            off_x: s("0"),
            off_y: s("0"),
            ext1: s("0"),
            ext2: s("0"),
            corner: CornerMode::MeiaEsquadria,
            gap: s("0"),
            joints: vec![],
        }));
        d
    }

    #[test]
    fn estrutura_vira_montagem_com_pecas_agrupadas() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let base = std::env::temp_dir().join("fenkra_estrutura_teste");
        let _ = std::fs::remove_dir_all(&base);
        let saida = base.join("saida");
        std::fs::create_dir_all(&base).unwrap();
        let pai = base.join("quadro.fkp");
        let doc = quadro();
        std::fs::write(&pai, doc.to_json()).unwrap();

        let g = estrutura_para_montagem(&pai, &saida).unwrap();
        assert_eq!(g.grupos.len(), 2);
        assert_eq!(g.grupos.iter().map(|x| x.quantidade).collect::<Vec<_>>(), vec![2, 2]);
        assert!(g.grupos.iter().all(|x| x.arquivo.exists()));

        // as peças apontam para o pai por caminho relativo e guardam a origem
        let p0 = Document::from_json(&std::fs::read_to_string(&g.grupos[0].arquivo).unwrap()).unwrap();
        let ext = &p0.external[0];
        assert!(Path::new(&ext.path).is_relative(), "{}", ext.path);
        assert_eq!(std::fs::canonicalize(saida.join(&ext.path)).unwrap(), std::fs::canonicalize(&pai).unwrap());
        assert!(p0.props.custom.iter().any(|(k, v)| k == "origem" && v == "estrutura_pai"));
        assert!(p0.props.description.contains("L=300"), "{}", p0.props.description);
        // o link nasce congelado: 'atualizar links' não o marca como quebrado
        assert!(g.grupos.iter().all(|x| {
            let p = Document::from_json(&std::fs::read_to_string(&x.arquivo).unwrap()).unwrap();
            p.external.len() == 1 && p.external[0].state == fenkra_core::LinkState::Congelado
        }));

        // gerar de novo sobrescreve com os mesmos nomes de arquivo
        let de_novo = estrutura_para_montagem(&pai, &saida).unwrap();
        assert_eq!(de_novo, g);

        // montagem: um componente por membro, em dois grupos de item
        let a = Assembly::from_json(&std::fs::read_to_string(&g.montagem).unwrap()).unwrap();
        let comps: Vec<&Node> = a.nodes.iter().filter(|n| !matches!(n.kind, NodeKind::Group)).collect();
        assert_eq!(comps.len(), 4);
        assert_eq!(a.nodes.iter().filter(|n| matches!(n.kind, NodeKind::Group)).count(), 2);
        assert!(comps.iter().all(|n| n.fixed && n.parent.is_some()));
        assert_eq!(a.items.len(), 2);

        // compor: o volume da montagem fecha com o da peça pai
        let mut cache = PartCache::default();
        let mut erros = vec![];
        let (_, r) = view::compose(&a, Some(&saida), &mut cache, &mut erros);
        assert!(erros.is_empty(), "{erros:?}");
        assert_eq!(r.bodies.len(), 4);
        let v_asm: f64 = r.bodies.iter().map(|b| b.mass.volume).sum();
        let v_pai: f64 = fenkra_solidos::recompute(&doc).bodies.iter().map(|b| b.mass.volume).sum();
        assert!((v_asm - v_pai).abs() < 0.01 * v_pai, "montagem {v_asm} × pai {v_pai}");

        // e as instâncias caem onde os membros estão: a caixa do conjunto é a do pai
        let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
        for n in &comps {
            let sh = view::placed_shape(&a, n.id, Some(&saida), &mut cache).unwrap();
            let (l, h) = sh.bbox().unwrap();
            for k in 0..3 {
                lo[k] = lo[k].min(l[k]);
                hi[k] = hi[k].max(h[k]);
            }
        }
        let (plo, phi) = fenkra_solidos::final_shape(&doc).unwrap().bbox().unwrap();
        for k in 0..3 {
            assert!((lo[k] - plo[k]).abs() < 0.5 && (hi[k] - phi[k]).abs() < 0.5, "eixo {k}: {lo:?} {hi:?} × {plo:?} {phi:?}");
        }
    }
}
