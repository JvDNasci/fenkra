// SPDX-License-Identifier: LGPL-2.1-or-later
//! Publicações (B55): o nome estável que uma peça ou montagem oferece às outras.
//!
//! O link guarda o nome, não a geometria de origem. Por isso as três operações que mexem numa
//! publicação sem quebrar quem a usa são todas sobre o nome:
//! - renomear deixa o nome velho como apelido (`Document::apelidos_publicacao`), e quem ainda o usa
//!   continua achando a publicação ([`com_apelidos`]) até atualizar;
//! - trocar a geometria passa o nome para outro elemento: os links seguem, sem saber da troca;
//! - remover pede antes a lista de [`dependentes`], porque ali o nome some de verdade.

use crate::links::usa_origem;
use crate::load::{full_path, PartCache};
use crate::*;
use fenkra_core::{Document, LinkedGeo};
use std::path::{Path, PathBuf};

/// Publicações pelos nomes atuais e pelos apelidos. É o que um link consulta.
pub fn com_apelidos(doc: &Document) -> Vec<(String, LinkedGeo)> {
    let mut v = fenkra_solidos::publications(doc);
    for (velho, novo) in &doc.apelidos_publicacao {
        if let Some(g) = v.iter().find(|(n, _)| n == novo).map(|(_, g)| g.clone()) {
            v.push((velho.clone(), g));
        }
    }
    v
}

/// Publica um sketch: cada curva dele vira uma cópia associativa publicada, `Nome.1`, `Nome.2`…
/// Devolve quantas. Publicar de novo o mesmo sketch não repete as curvas já publicadas.
pub fn publicar_sketch(doc: &mut Document, sketch: Id) -> usize {
    let Some(f) = doc.feature(sketch) else { return 0 };
    let fenkra_core::FeatureKind::Sketch(s) = &f.kind else { return 0 };
    let nome = f.name.clone();
    let curvas: Vec<Id> = s.data.curves.iter().filter(|c| !c.construction).map(|c| c.id).collect();
    let mut novas = 0;
    for (i, curve) in curvas.into_iter().enumerate() {
        let alvo = fenkra_core::CurveRef::SketchCurve { sketch, curve };
        if doc.references.iter().any(|r| r.published && matches!(&r.kind, fenkra_core::RefKind::CurveCopy { curve: c } if *c == alvo)) {
            continue;
        }
        let id = doc.add_geometry(fenkra_core::RefKind::CurveCopy { curve: alvo });
        if let Some(r) = doc.references.iter_mut().find(|r| r.id == id) {
            r.published = true;
            r.name = format!("{nome}.{}", i + 1);
        }
        novas += 1;
    }
    novas
}

/// Nome atual de uma publicação, seguindo o apelido.
pub fn nome_atual<'a>(doc: &'a Document, nome: &'a str) -> &'a str {
    doc.apelidos_publicacao.iter().find(|(v, _)| v == nome).map(|(_, n)| n.as_str()).unwrap_or(nome)
}

/// Renomeia uma referência publicada e guarda o nome velho como apelido.
pub fn renomear(doc: &mut Document, atual: &str, novo: &str) -> Result<(), String> {
    let novo = novo.trim();
    if novo.is_empty() {
        return Err("dê um nome à publicação".into());
    }
    if doc.name_taken(novo) {
        return Err("já existe um elemento com este nome".into());
    }
    let r = doc.references.iter_mut().find(|r| r.published && r.name == atual).ok_or("publicação não encontrada")?;
    r.name = novo.to_string();
    // cadeia a → b → c vira a → c e b → c: um apelido sempre aponta para o nome de agora
    for (_, n) in doc.apelidos_publicacao.iter_mut().filter(|(_, n)| n == atual) {
        *n = novo.to_string();
    }
    doc.apelidos_publicacao.retain(|(v, _)| v != novo);
    doc.apelidos_publicacao.push((atual.to_string(), novo.to_string()));
    Ok(())
}

/// Passa o nome publicado para outro elemento da mesma classe. Os links mantêm o nome e leem a
/// geometria nova na próxima atualização.
pub fn trocar_geometria(doc: &mut Document, nome: &str, novo: Id) -> Result<(), String> {
    let i = doc.references.iter().position(|r| r.published && r.name == nome).ok_or("publicação não encontrada")?;
    let j = doc.references.iter().position(|r| r.id == novo).ok_or("elemento não encontrado")?;
    if i == j {
        return Ok(());
    }
    if doc.references[i].kind.class() != doc.references[j].kind.class() {
        return Err("a geometria nova precisa ser do mesmo tipo da publicada".into());
    }
    let velho = doc.references[j].name.clone();
    doc.references[j].name = nome.to_string();
    doc.references[j].published = true;
    doc.references[i].name = velho;
    doc.references[i].published = false;
    Ok(())
}

/// Tira a publicação e os apelidos dela. Quem usa fica pendurado: consulte [`dependentes`] antes.
pub fn remover(doc: &mut Document, nome: &str) -> Result<(), String> {
    let r = doc.references.iter_mut().find(|r| r.published && r.name == nome).ok_or("publicação não encontrada")?;
    r.published = false;
    doc.apelidos_publicacao.retain(|(_, n)| n != nome);
    Ok(())
}

/// Quem usa uma publicação.
#[derive(Clone, Debug, PartialEq)]
pub struct Dependente {
    pub comp: Id,
    pub peca: String,
    pub link: String,
}

/// Peças desta montagem com link para a publicação `nome` do arquivo `origem`, por qualquer nome dela.
pub fn dependentes(a: &Assembly, dir: Option<&Path>, cache: &mut PartCache, origem: &Path, nome: &str) -> Vec<Dependente> {
    let apelidos: Vec<String> = cache.get(origem, false).map(|p| p.doc.apelidos_publicacao.iter().filter(|(_, n)| n == nome).map(|(v, _)| v.clone()).collect()).unwrap_or_default();
    let mut vistos: Vec<PathBuf> = vec![];
    let mut out = vec![];
    for n in &a.nodes {
        let NodeKind::Part { path } = &n.kind else { continue };
        let full = full_path(dir, path);
        if vistos.contains(&full) || full == origem {
            continue;
        }
        vistos.push(full.clone());
        let Ok(p) = cache.get(&full, false) else { continue };
        for e in p.doc.external.iter().filter(|e| usa_origem(&full, e, origem, dir) && (e.publication == nome || apelidos.contains(&e.publication))) {
            out.push(Dependente { comp: n.id, peca: n.name.clone(), link: p.doc.name_of(e.target.unwrap_or(e.id)) });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::links::{self, EstadoLink, Gatilho};
    use fenkra_core::{OriginPlane, PlaneRef};

    /// Peça com um plano a `z` da origem, publicado como `Base`.
    fn mestre(z: &str) -> Document {
        let mut d = Document::new("Mestre");
        let id = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), z);
        let r = d.references.iter_mut().find(|r| r.id == id).unwrap();
        r.name = "Base".into();
        r.published = true;
        d
    }

    fn pasta(nome: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("fenkra_publicacoes_{nome}"));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn origem_z(g: &LinkedGeo) -> f64 {
        match g {
            LinkedGeo::Plane { origin, .. } => origin[2],
            _ => panic!(),
        }
    }

    /// Montagem com o mestre e uma peça ligada à publicação `Base`.
    fn par(nome: &str) -> (PathBuf, Assembly, PartCache, Id) {
        let dir = pasta(nome);
        let m = mestre("10");
        std::fs::write(dir.join("m.fkp"), m.to_json()).unwrap();
        let geo = fenkra_solidos::publications(&m).remove(0).1;
        let mut p = Document::new("Filha");
        p.add_link("Mestre", "m.fkp", "Base", geo);
        std::fs::write(dir.join("f.fkp"), p.to_json()).unwrap();
        let mut a = Assembly::new("A");
        a.add_component("Mestre", NodeKind::Part { path: "m.fkp".into() }, Placement::default(), None);
        let f = a.add_component("Filha", NodeKind::Part { path: "f.fkp".into() }, Placement::default(), None);
        (dir, a, PartCache::default(), f)
    }

    fn regravar(dir: &Path, cache: &mut PartCache, d: &Document) {
        std::fs::write(dir.join("m.fkp"), d.to_json()).unwrap();
        cache.forget(&dir.join("m.fkp"));
    }

    #[test]
    fn renomear_mantem_quem_usa_o_nome_velho() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (dir, a, mut cache, f) = par("renomear");
        let mut m = cache.get(&dir.join("m.fkp"), false).unwrap().doc.clone();
        renomear(&mut m, "Base", "Apoio").unwrap();
        renomear(&mut m, "Apoio", "Assento").unwrap();
        assert_eq!(nome_atual(&m, "Base"), "Assento", "a cadeia aponta para o nome de agora");
        regravar(&dir, &mut cache, &m);
        let rel = links::atualizar(&a, Some(&dir), &mut cache, Gatilho::Manual);
        assert_eq!(rel.pendurados, 0, "{rel:?}");
        let l = links::links_da_peca(&a, Some(&dir), &mut cache, f);
        assert_eq!(l[0].estado, EstadoLink::EmContexto);
        assert_eq!(l[0].publicacao, "Assento", "o link passa ao nome novo");
        assert_eq!(dependentes(&a, Some(&dir), &mut cache, &dir.join("m.fkp"), "Assento").len(), 1);
    }

    #[test]
    fn trocar_a_geometria_mantem_o_link_e_muda_a_copia() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (dir, a, mut cache, _) = par("trocar");
        let mut m = cache.get(&dir.join("m.fkp"), false).unwrap().doc.clone();
        let outro = m.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "25");
        trocar_geometria(&mut m, "Base", outro).unwrap();
        assert_eq!(fenkra_solidos::publications(&m).len(), 1);
        regravar(&dir, &mut cache, &m);
        let rel = links::atualizar(&a, Some(&dir), &mut cache, Gatilho::Manual);
        assert_eq!((rel.pendurados, rel.atualizados), (0, 1), "{rel:?}");
        let d = cache.get(&dir.join("f.fkp"), false).unwrap().doc.clone();
        assert!((origem_z(d.external[0].snapshot.as_ref().unwrap()) - 25.).abs() < 1e-9);
    }

    #[test]
    fn trocar_por_outro_tipo_nao_vale() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let mut m = mestre("10");
        let id = m.add_geometry(fenkra_core::RefKind::PointCoords { x: "0".into(), y: "0".into(), z: "0".into(), frame: None });
        assert!(trocar_geometria(&mut m, "Base", id).is_err());
    }

    #[test]
    fn remover_lista_quem_depende_e_deixa_pendurado() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (dir, a, mut cache, f) = par("remover");
        let dep = dependentes(&a, Some(&dir), &mut cache, &dir.join("m.fkp"), "Base");
        assert_eq!(dep.len(), 1);
        assert_eq!((dep[0].comp, dep[0].peca.as_str()), (f, "Filha"));
        let mut m = cache.get(&dir.join("m.fkp"), false).unwrap().doc.clone();
        remover(&mut m, "Base").unwrap();
        regravar(&dir, &mut cache, &m);
        let rel = links::atualizar(&a, Some(&dir), &mut cache, Gatilho::Manual);
        assert_eq!(rel.pendurados, 1, "{rel:?}");
    }

    #[test]
    fn nome_em_uso_ou_vazio_nao_renomeia() {
        let mut m = mestre("10");
        assert!(renomear(&mut m, "Base", " ").is_err());
        assert!(renomear(&mut m, "Nada", "Outro").is_err());
        let id = m.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "1");
        let nome = m.reference(id).unwrap().name.clone();
        assert!(renomear(&mut m, "Base", &nome).is_err());
    }
}
