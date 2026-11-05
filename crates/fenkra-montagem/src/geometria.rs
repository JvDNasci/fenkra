//! Geometria própria da montagem e peça esqueleto (B55).
//!
//! Sketch, ponto, eixo e plano no nível da montagem moram num documento embutido no `.fka`
//! (`Assembly::geometria`), no sistema da montagem. É um `Document` comum de propósito: o editor de
//! sketch, os comandos de referência, o recálculo e as publicações da peça valem aqui sem
//! nenhuma cópia, e a montagem só o mostra.
//!
//! A peça esqueleto é um arquivo de peça comum, fixo na origem, que fica fora da lista de materiais.
//! Ela reúne o traçado do produto em um lugar, e "Fazer peça" cria peças já ligadas às publicações
//! dela, na mesma origem. Quando o esqueleto muda, os links atualizam as peças.

use crate::load::PartCache;
use crate::publicacoes::com_apelidos;
use crate::*;
use fenkra_core::{Document, LinkedGeo};
use std::path::Path;

impl Assembly {
    /// Documento da geometria da montagem, criando na primeira vez.
    pub fn geometria_mut(&mut self) -> &mut Document {
        let nome = format!("{} (geometria)", self.name);
        self.geometria.get_or_insert_with(|| Document::new(&nome))
    }

    /// Pontos de um sketch da montagem, no mundo, e a normal do plano dele.
    pub fn pontos_de_sketch(&self, sketch: Id) -> Option<(Vec<V3>, V3)> {
        let g = self.geometria.as_ref()?;
        let r = fenkra_solidos::recompute(g);
        let s = r.sketches.iter().find(|s| s.feature == sketch)?;
        Some((s.points.iter().map(|p| p.map(|v| v as f64)).collect(), s.frame.normal))
    }

    /// Publicação da geometria da montagem.
    pub fn publicacao_da_montagem(&self, nome: &str) -> Option<LinkedGeo> {
        com_apelidos(self.geometria.as_ref()?).into_iter().find(|(n, _)| n == nome).map(|(_, g)| g)
    }
}

/// Peça esqueleto nova, só na memória até o Salvar da montagem. `arquivo` é o caminho completo e
/// `path` é como o nó o guarda.
pub fn criar_esqueleto(a: &mut Assembly, nome: &str, arquivo: &Path, path: &str, cache: &mut PartCache) -> Result<Id, String> {
    if a.esqueleto.is_some_and(|e| a.node(e).is_some()) {
        return Err("esta montagem já tem uma peça esqueleto".into());
    }
    cache.definir_pendente(arquivo, Document::new(nome));
    let id = a.add_component(nome, NodeKind::Part { path: path.into() }, Placement::default(), None);
    if let Some(n) = a.node_mut(id) {
        n.fixed = true;
    }
    a.esqueleto = Some(id);
    Ok(id)
}

/// Peça nova a partir do esqueleto: mesma origem, fixa, com um link para cada publicação escolhida.
pub fn fazer_peca(a: &mut Assembly, nome: &str, arquivo: &Path, path: &str, publicacoes: &[String], dir: Option<&Path>, cache: &mut PartCache) -> Result<Id, String> {
    let esq = a.esqueleto.and_then(|e| a.node(e)).cloned().ok_or("a montagem não tem peça esqueleto")?;
    let NodeKind::Part { path: pe } = &esq.kind else { return Err("o esqueleto não é uma peça".into()) };
    let origem = crate::load::full_path(dir, pe);
    let pubs = com_apelidos(&cache.get(&origem, false)?.doc);
    let mut doc = Document::new(nome);
    for p in publicacoes {
        let geo = pubs.iter().find(|(n, _)| n == p).map(|(_, g)| g.clone()).ok_or_else(|| format!("o esqueleto não publica {p}"))?;
        doc.add_link(&esq.name, &crate::furos::caminho_de(arquivo, &origem), p, geo);
    }
    cache.definir_pendente(arquivo, doc);
    let id = a.add_component(nome, NodeKind::Part { path: path.into() }, esq.placement, None);
    if let Some(n) = a.node_mut(id) {
        n.fixed = true;
    }
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::links::{self, Gatilho};
    use fenkra_core::{sketch::Curve, FeatureKind, OriginPlane, PlaneRef, Sketch, SketchData};
    use std::path::PathBuf;

    fn pasta(nome: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("fenkra_geometria_{nome}"));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn publica_plano(d: &mut Document, z: &str) {
        let id = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), z);
        let r = d.references.iter_mut().find(|r| r.id == id).unwrap();
        r.name = "Assento".into();
        r.published = true;
    }

    #[test]
    fn geometria_da_montagem_vai_e_volta_e_aparece_na_vista() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let mut a = Assembly::new("A");
        publica_plano(a.geometria_mut(), "15");
        let b = Assembly::from_json(&a.to_json()).unwrap();
        assert_eq!(b.geometria, a.geometria);
        assert!(matches!(b.publicacao_da_montagem("Assento"), Some(LinkedGeo::Plane { origin, .. }) if (origin[2] - 15.).abs() < 1e-9));
        let (doc, r) = crate::view::compose(&a, None, &mut PartCache::default(), &mut vec![]);
        assert!(r.planes.iter().any(|p| (p.frame.origin[2] - 15.).abs() < 1e-9), "o plano da montagem aparece");
        assert!(doc.collectors.is_empty(), "e não vira componente");
    }

    #[test]
    fn furo_pelos_pontos_de_um_sketch_da_montagem() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let dir = pasta("sketch");
        let mut placa = Document::new("Placa");
        let mut n = 9_000_000u64;
        let mut al = || {
            n += 1;
            Id(n)
        };
        let mut base = SketchData::default();
        base.add_rect(&mut al, [-30., -30.], [30., 30.], false);
        let sb = placa.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data: base }));
        placa.add_feature(FeatureKind::Extrude(fenkra_core::Extrude { sketch: sb, op: fenkra_core::BoolOp::Add, extent: fenkra_core::Extent::Blind("10".into()), reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0 }));
        std::fs::write(dir.join("p.fkp"), placa.to_json()).unwrap();
        let mut a = Assembly::new("A");
        let c = a.add_component("Placa", NodeKind::Part { path: "p.fkp".into() }, Placement::default(), None);
        let topo = a.geometria_mut().add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "10");
        let mut s = SketchData::default();
        for (x, y) in [(10., 0.), (-10., 0.)] {
            let p = s.add_point(&mut al, x, y);
            s.add_curve(&mut al, Curve::Point { p }, false);
        }
        let sk = a.geometria_mut().add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Datum(topo), data: s }));
        let mut cache = PartCache::default();
        let serie = SerieFuros::nova(EixoFuro::Sketch { sketch: sk, inverter: false }, "6");
        a.add_serie_furos(serie, vec![], Some(&dir), &mut cache).unwrap();
        let arq = dir.join("a.fka");
        let r = crate::furos::sincronizar(&mut a, Some(&dir), Some(&arq), &mut cache, None);
        assert_eq!(r.gravados, 2, "{r:?}");
        assert_eq!(fenkra_solidos::hole_table(&cache.get(&dir.join("p.fkp"), false).unwrap().doc).len(), 2);
        let _ = c;
    }

    #[test]
    fn esqueleto_fora_da_lista_e_peca_ligada_a_ele() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let dir = pasta("esqueleto");
        let mut a = Assembly::new("A");
        let mut cache = PartCache::default();
        let f = dir.join("esq.fkp");
        let e = criar_esqueleto(&mut a, "Esqueleto", &f, "esq.fkp", &mut cache).unwrap();
        assert!(criar_esqueleto(&mut a, "Outro", &dir.join("o.fkp"), "o.fkp", &mut cache).is_err(), "um só");
        assert!(a.node(e).unwrap().fixed && !f.exists());
        let mut d = cache.get(&f, false).unwrap().doc.clone();
        publica_plano(&mut d, "20");
        cache.definir_pendente(&f, d);

        let g = dir.join("tampa.fkp");
        let t = fazer_peca(&mut a, "Tampa", &g, "tampa.fkp", &["Assento".to_string()], Some(&dir), &mut cache).unwrap();
        assert!(fazer_peca(&mut a, "X", &dir.join("x.fkp"), "x.fkp", &["Nada".to_string()], Some(&dir), &mut cache).is_err());
        assert_eq!(a.node(t).unwrap().placement, a.node(e).unwrap().placement);
        assert_eq!(cache.get(&g, false).unwrap().doc.external.len(), 1);

        // tudo vira arquivo no Salvar, e o esqueleto fica fora da lista
        cache.gravar_pendentes(&a, Some(&dir)).unwrap();
        assert!(f.exists() && g.exists());
        let bom = crate::bom::bom(&a, Some(&dir), crate::bom::BomLevel::Topo);
        assert_eq!(bom.len(), 1, "{bom:?}");

        // mudar o esqueleto chega à peça pelo link
        let mut d = cache.get(&f, false).unwrap().doc.clone();
        let r = d.references.iter().find(|r| r.published).unwrap().id;
        let id = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "35");
        crate::publicacoes::trocar_geometria(&mut d, "Assento", id).unwrap();
        let _ = r;
        std::fs::write(&f, d.to_json()).unwrap();
        cache.forget(&f);
        let rel = links::atualizar(&a, Some(&dir), &mut cache, Gatilho::Manual);
        assert_eq!(rel.atualizados, 1, "{rel:?}");
        let s = cache.get(&g, false).unwrap().doc.external[0].snapshot.clone().unwrap();
        assert!(matches!(s, LinkedGeo::Plane { origin, .. } if (origin[2] - 35.).abs() < 1e-9));

        a.delete(e);
        assert_eq!(a.esqueleto, None);
    }
}
