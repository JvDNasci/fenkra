// SPDX-License-Identifier: LGPL-2.1-or-later
//! Substituir componente com religação (B55).
//!
//! Trocar o arquivo de um componente deixa as restrições apontando para geometria da peça velha.
//! O plano é montado antes de mudar qualquer coisa, para a pessoa ver e decidir. Cada elemento
//! procura o par na peça nova, nesta ordem:
//! 1. nome: o elemento velho era uma publicação, e a peça nova publica o mesmo nome;
//! 2. assinatura: no mesmo lugar existe elemento do mesmo tipo, com a mesma posição;
//! 3. proximidade: há elemento do mesmo tipo perto dali. É palpite, então vale só se confirmado.
//!
//! O que não casa, ou que não foi confirmado, não vai para outra geometria em silêncio: a restrição
//! fica suprimida e aparece na tabela.

use crate::load::{full_path, geom_at, PartCache};
use crate::publicacoes::com_apelidos;
use crate::*;
use fenkra_core::LinkedGeo;
use fenkra_kernel::Shape;
use std::path::Path;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Escopo {
    Esta,
    /// todas as instâncias do mesmo arquivo
    Todas,
}

#[derive(Clone, PartialEq, Debug)]
pub enum Casamento {
    Nome(String),
    Assinatura,
    Proximidade(f64),
    NaoCasou,
}

/// Um elemento de restrição e o par achado na peça nova.
#[derive(Clone, PartialEq, Debug)]
pub struct Linha {
    pub mate: Id,
    pub mate_nome: String,
    pub comp: Id,
    /// lado da restrição: `false` = a, `true` = b
    pub lado_b: bool,
    pub casamento: Casamento,
    pub geom: Option<Geom>,
    pub pick: Option<V3>,
    /// nome e assinatura já nascem aceitos; proximidade espera a pessoa
    pub aceita: bool,
}

/// O que vai acontecer, antes de acontecer.
#[derive(Clone, PartialEq, Debug)]
pub struct Plano {
    pub comps: Vec<Id>,
    pub kind: NodeKind,
    pub linhas: Vec<Linha>,
}

impl Plano {
    /// Restrições que ficariam suprimidas com as escolhas atuais.
    pub fn sem_par(&self) -> Vec<&Linha> {
        self.linhas.iter().filter(|l| !l.aceita || l.geom.is_none()).collect()
    }
}

/// Mesma geometria, dentro da tolerância?
fn iguais(a: &Geom, b: &Geom) -> bool {
    const TOL: f64 = 1e-6;
    let alinhados = |u: V3, v: V3| norm(cross(unit(u), unit(v))) < TOL;
    match (a, b) {
        (Geom::Plane { p, n }, Geom::Plane { p: q, n: m }) => alinhados(*n, *m) && dot(sub(*p, *q), unit(*n)).abs() < TOL,
        (Geom::Axis { p, d }, Geom::Axis { p: q, d: e }) => alinhados(*d, *e) && norm(cross(sub(*p, *q), unit(*d))) < TOL,
        (Geom::Point(p), Geom::Point(q)) => norm(sub(*p, *q)) < TOL,
        _ => false,
    }
}

/// Publicação como elemento de restrição.
fn geom_de(g: &LinkedGeo) -> Option<Geom> {
    match g {
        LinkedGeo::Plane { origin, normal, .. } => Some(Geom::Plane { p: *origin, n: *normal }),
        LinkedGeo::Point(p) => Some(Geom::Point(*p)),
        LinkedGeo::Curve(v) if v.len() >= 2 => Some(Geom::Axis { p: v[0], d: unit(sub(v[v.len() - 1], v[0])) }),
        _ => None,
    }
}

fn casar(velho: &Geom, pick: Option<V3>, pubs_velhas: &[(String, LinkedGeo)], pubs_novas: &[(String, LinkedGeo)], forma: Option<&Shape>) -> (Casamento, Option<Geom>, Option<V3>) {
    let tipo = |g: &Geom| std::mem::discriminant(g);
    let nome = pubs_velhas.iter().find(|(_, g)| geom_de(g).is_some_and(|x| iguais(&x, velho))).map(|(n, _)| n);
    if let Some(n) = nome {
        if let Some(g) = pubs_novas.iter().find(|(m, _)| m == n).and_then(|(_, g)| geom_de(g)).filter(|g| tipo(g) == tipo(velho)) {
            return (Casamento::Nome(n.clone()), Some(g), None);
        }
    }
    let Some(sh) = forma else { return (Casamento::NaoCasou, None, None) };
    let ponto = pick.or(match velho {
        Geom::Plane { p, .. } | Geom::Axis { p, .. } | Geom::Point(p) => Some(*p),
    });
    let Some(ponto) = ponto else { return (Casamento::NaoCasou, None, None) };
    if let Ok(g) = geom_at(sh, ponto) {
        if iguais(&g, velho) {
            return (Casamento::Assinatura, Some(g), pick);
        }
    }
    if let Ok((q, _)) = sh.surface_near(ponto) {
        if let Some(g) = geom_at(sh, q).ok().filter(|g| tipo(g) == tipo(velho)) {
            return (Casamento::Proximidade(norm(sub(q, ponto))), Some(g), Some(q));
        }
    }
    (Casamento::NaoCasou, None, None)
}

/// Monta o plano sem mexer em nada. `arquivo_novo` já vem no formato do nó.
pub fn planejar(a: &Assembly, comp: Id, escopo: Escopo, kind: NodeKind, dir: Option<&Path>, cache: &mut PartCache) -> Result<Plano, String> {
    let velho = a.node(comp).ok_or("componente não encontrado")?.kind.clone();
    let comps: Vec<Id> = match escopo {
        Escopo::Esta => vec![comp],
        Escopo::Todas => a.nodes.iter().filter(|n| n.kind == velho).map(|n| n.id).collect(),
    };
    let pubs = |k: &NodeKind, cache: &mut PartCache| match k {
        NodeKind::Part { path } => cache.get(&full_path(dir, path), false).map(|p| com_apelidos(&p.doc)).unwrap_or_default(),
        _ => vec![],
    };
    let (pv, pn) = (pubs(&velho, cache), pubs(&kind, cache));
    let forma = match &kind {
        NodeKind::Part { path } => cache.get(&full_path(dir, path), true)?.shape.clone(),
        _ => None,
    };
    let mut linhas = vec![];
    for m in &a.mates {
        for (r, lado_b) in [(&m.a, false), (&m.b, true)] {
            if !comps.contains(&r.comp) {
                continue;
            }
            let (casamento, geom, pick) = casar(&r.geom, r.pick, &pv, &pn, forma.as_ref());
            let aceita = matches!(casamento, Casamento::Nome(_) | Casamento::Assinatura);
            linhas.push(Linha { mate: m.id, mate_nome: m.name.clone(), comp: r.comp, lado_b, casamento, geom, pick, aceita });
        }
    }
    Ok(Plano { comps, kind, linhas })
}

/// Aplica o plano. Devolve as restrições que ficaram suprimidas.
pub fn aplicar(a: &mut Assembly, plano: &Plano) -> Vec<Id> {
    for c in &plano.comps {
        a.replace(*c, plano.kind.clone());
    }
    let mut suprimidas = vec![];
    for l in &plano.linhas {
        let Some(m) = a.mates.iter_mut().find(|m| m.id == l.mate) else { continue };
        match (l.aceita, l.geom) {
            (true, Some(g)) => {
                let r = if l.lado_b { &mut m.b } else { &mut m.a };
                r.geom = g;
                r.pick = l.pick;
            }
            _ => {
                m.suppressed = true;
                if !suprimidas.contains(&l.mate) {
                    suprimidas.push(l.mate);
                }
            }
        }
    }
    suprimidas
}

#[cfg(test)]
mod tests {
    use super::*;
    use fenkra_core::{BoolOp, Document, Extent, Extrude, FeatureKind, OriginPlane, PlaneRef, Sketch, SketchData};
    use std::path::PathBuf;

    /// Bloco 40 × 40 de altura `h`, com o topo publicado como `Topo`.
    fn bloco(h: f64, publica: bool) -> Document {
        let mut d = Document::new("Bloco");
        let mut n = 8_000_000u64;
        let mut al = || {
            n += 1;
            Id(n)
        };
        let mut s = SketchData::default();
        s.add_rect(&mut al, [-20., -20.], [20., 20.], false);
        let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data: s }));
        d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind(valor_cota(h)), reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
        if publica {
            let id = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), &valor_cota(h));
            let r = d.references.iter_mut().find(|r| r.id == id).unwrap();
            r.name = "Topo".into();
            r.published = true;
        }
        d
    }

    fn caso(nome: &str, novo: Document) -> (PathBuf, Assembly, PartCache, Id, Id) {
        let dir = std::env::temp_dir().join(format!("fenkra_subst_{nome}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("v.fkp"), bloco(10., true).to_json()).unwrap();
        std::fs::write(dir.join("n.fkp"), novo.to_json()).unwrap();
        std::fs::write(dir.join("base.fkp"), bloco(5., false).to_json()).unwrap();
        let mut a = Assembly::new("A");
        let base = a.add_component("Base", NodeKind::Part { path: "base.fkp".into() }, Placement::default(), None);
        let c = a.add_component("Bloco", NodeKind::Part { path: "v.fkp".into() }, Placement::default(), None);
        let topo = [0., 0., 10.];
        a.add_mate(MateKind::Contact, MateRef { comp: base, geom: Geom::Plane { p: [0., 0., 0.], n: [0., 0., -1.] }, pick: Some([0., 0., 0.]) }, MateRef { comp: c, geom: Geom::Plane { p: topo, n: [0., 0., 1.] }, pick: Some(topo) });
        (dir, a, PartCache::default(), base, c)
    }

    fn novo(a: &str) -> NodeKind {
        NodeKind::Part { path: a.into() }
    }

    #[test]
    fn nome_publicado_casa_mesmo_com_a_face_em_outro_lugar() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (dir, mut a, mut cache, _, c) = caso("nome", bloco(30., true));
        let p = planejar(&a, c, Escopo::Esta, novo("n.fkp"), Some(&dir), &mut cache).unwrap();
        assert_eq!(p.linhas.len(), 1);
        assert_eq!(p.linhas[0].casamento, Casamento::Nome("Topo".into()));
        assert!(p.sem_par().is_empty());
        assert!(aplicar(&mut a, &p).is_empty());
        let Geom::Plane { p: q, .. } = a.mates[0].b.geom else { panic!() };
        assert!((q[2] - 30.).abs() < 1e-9, "{q:?}");
        assert_eq!(a.node(c).unwrap().kind, novo("n.fkp"));
    }

    #[test]
    fn sem_nome_casa_pela_assinatura() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (dir, a, mut cache, _, c) = caso("assinatura", bloco(10., false));
        let p = planejar(&a, c, Escopo::Esta, novo("n.fkp"), Some(&dir), &mut cache).unwrap();
        assert_eq!(p.linhas[0].casamento, Casamento::Assinatura);
        assert!(p.linhas[0].aceita);
    }

    #[test]
    fn proximidade_so_vale_confirmada_e_o_resto_fica_suprimido() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (dir, mut a, mut cache, _, c) = caso("proximidade", bloco(12., false));
        let mut p = planejar(&a, c, Escopo::Esta, novo("n.fkp"), Some(&dir), &mut cache).unwrap();
        assert!(matches!(p.linhas[0].casamento, Casamento::Proximidade(d) if (d - 2.).abs() < 1e-6), "{:?}", p.linhas[0].casamento);
        assert!(!p.linhas[0].aceita, "palpite não entra sozinho");
        assert_eq!(p.sem_par().len(), 1);
        let mut b = a.clone();
        assert_eq!(aplicar(&mut b, &p), vec![b.mates[0].id]);
        assert!(b.mates[0].suppressed, "sem confirmar, a restrição fica de fora");
        p.linhas[0].aceita = true;
        assert!(aplicar(&mut a, &p).is_empty());
        assert!(!a.mates[0].suppressed);
    }

    #[test]
    fn todas_as_instancias_entram_no_mesmo_plano() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (dir, mut a, mut cache, _, c) = caso("todas", bloco(30., true));
        let outra = a.add_component("Bloco", NodeKind::Part { path: "v.fkp".into() }, Placement { origin: [100., 0., 0.], ..Default::default() }, None);
        let p = planejar(&a, c, Escopo::Todas, novo("n.fkp"), Some(&dir), &mut cache).unwrap();
        assert_eq!(p.comps, vec![c, outra]);
        let so = planejar(&a, c, Escopo::Esta, novo("n.fkp"), Some(&dir), &mut cache).unwrap();
        assert_eq!(so.comps, vec![c]);
        aplicar(&mut a, &p);
        assert_eq!(a.node(outra).unwrap().kind, novo("n.fkp"));
    }

    #[test]
    fn planejar_nao_muda_a_montagem() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (dir, a, mut cache, _, c) = caso("intacta", bloco(30., true));
        let antes = a.clone();
        planejar(&a, c, Escopo::Todas, novo("n.fkp"), Some(&dir), &mut cache).unwrap();
        assert_eq!(a, antes);
    }
}
