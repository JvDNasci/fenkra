// SPDX-License-Identifier: LGPL-2.1-or-later
//! B577: subelementos com nome próprio, "Dono · Face 1 · Aresta 2 · Vértice 1", nunca por coordenada.
//!
//! O nome nasce da estrutura do corpo e não do ponto: a face é a n-ésima do dono pela ordem dos papéis
//! (`FaceKey::role`: tampa inicial, tampa final, laterais pelo índice da aresta do perfil, furos, abas;
//! numa superfície, o índice da face), a aresta é a m-ésima da face dela pela ordem da face vizinha, e o
//! vértice é a ponta 1 ou 2 daquela aresta. Mudar uma cota não muda a estrutura, então o nome segue o
//! mesmo elemento; o comando que o usa guarda a chave (`FaceKey`/`EdgeKey`), que é o que o recálculo
//! resolve, e esta tabela só traduz a chave para o nome e o nome para o elemento da tela.
//!
//! A tabela é montada de cada resultado de recálculo (corpos e superfícies) e serve ao painel, à caixa de
//! seleção e à árvore de subelementos. Coordenada só entra como dica.

use crate::refs::Tradutor;
use crate::RecomputeResult;
use fenkra_core::{Document, EdgeKey, FaceKey, Id, PAPEL_SUPERFICIE};
use fenkra_kernel::{Mesh, NO_FACE};
use std::collections::HashMap;

/// Índice de "corpo" das superfícies de referência nas seleções (o mesmo `SURF_BASE` do picking).
pub const SURF_BASE: usize = 1 << 20;

/// Uma face numerada.
#[derive(Clone, Debug)]
pub struct FaceN {
    /// corpo (índice em `bodies`) ou `SURF_BASE + índice em surfaces`
    pub corpo: usize,
    /// índice na ordem de `mesh.face_ranges` (o do picking)
    pub face: usize,
    pub dono: Id,
    pub papel: i32,
    /// "Face n" dentro do dono, a partir de 1
    pub numero: u32,
    pub tipo: u8,
    pub dir: [f64; 3],
    pub area: f64,
    pub ponto: [f64; 3],
}

/// Uma aresta numerada dentro da face dela.
#[derive(Clone, Debug)]
pub struct ArestaN {
    pub corpo: usize,
    /// índice na ordem de `mesh.edge_ranges` (o do picking)
    pub aresta: usize,
    /// face de que a aresta é a de número `numero` (índice em `SubNomes::faces`)
    pub face: usize,
    /// "Aresta m" dentro da face
    pub numero: u32,
    pub a: (Id, i32),
    pub b: (Id, i32),
    /// as duas pontas: "Vértice 1" e "Vértice 2"
    pub pontas: [[f32; 3]; 2],
    pub meio: [f32; 3],
}

/// A hierarquia de nomes de um resultado de recálculo.
#[derive(Clone, Debug, Default)]
pub struct SubNomes {
    pub faces: Vec<FaceN>,
    pub arestas: Vec<ArestaN>,
    por_face: HashMap<(usize, usize), usize>,
    por_aresta: HashMap<(usize, usize), usize>,
}

/// Um corpo ou superfície visto pela tabela.
struct Parte<'a> {
    corpo: usize,
    malha: &'a Mesh,
    donos: Vec<Id>,
    papeis: Vec<i32>,
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn pontas_da_aresta(m: &Mesh, e: usize) -> Option<[[f32; 3]; 2]> {
    let nseg = m.edges.len() / 6;
    let s0 = *m.edge_ranges.get(e)? as usize;
    let s1 = m.edge_ranges.get(e + 1).map_or(nseg, |x| *x as usize);
    if s1 <= s0 || s1 > nseg {
        return None;
    }
    let p = |i: usize| [m.edges[i], m.edges[i + 1], m.edges[i + 2]];
    Some([p(s0 * 6), p((s1 - 1) * 6 + 3)])
}

impl SubNomes {
    /// Tabela do resultado: os corpos (dono e papel de cada face) e as superfícies de referência.
    pub fn de(r: &RecomputeResult) -> SubNomes {
        let mut partes: Vec<Parte> = r.bodies.iter().enumerate().map(|(i, b)| Parte { corpo: i, malha: &b.mesh, donos: b.face_owner.clone(), papeis: b.face_role.clone() }).collect();
        for (i, s) in r.surfaces.iter().enumerate() {
            let n = s.mesh.face_info.len();
            partes.push(Parte { corpo: SURF_BASE + i, malha: &s.mesh, donos: vec![s.id; n], papeis: crate::refs::surface_roles(n) });
        }
        Self::montar(&partes)
    }

    fn montar(partes: &[Parte]) -> SubNomes {
        // todas as faces, na ordem em que os donos aparecem, e dentro do dono pelo papel (sem papel por último)
        let mut brutas: Vec<FaceN> = vec![];
        let mut ordem_dono: HashMap<Id, usize> = HashMap::new();
        for p in partes {
            for (j, info) in p.malha.face_info.iter().enumerate() {
                let (Some(dono), Some(papel)) = (p.donos.get(j).copied(), p.papeis.get(j).copied()) else { continue };
                let n = ordem_dono.len();
                ordem_dono.entry(dono).or_insert(n);
                brutas.push(FaceN { corpo: p.corpo, face: j, dono, papel, numero: 0, tipo: crate::refs::kind_of(info.kind), dir: info.dir, area: info.area, ponto: info.point });
            }
        }
        brutas.sort_by_key(|f| (ordem_dono[&f.dono], f.papel < 0, f.papel, f.corpo, f.face));
        let mut contagem: HashMap<Id, u32> = HashMap::new();
        for f in &mut brutas {
            let c = contagem.entry(f.dono).or_insert(0);
            *c += 1;
            f.numero = *c;
        }
        let por_face: HashMap<(usize, usize), usize> = brutas.iter().enumerate().map(|(i, f)| ((f.corpo, f.face), i)).collect();

        // arestas: a face canônica é a de menor ordem das duas; dentro dela, pela ordem da vizinha
        struct Crua {
            corpo: usize,
            aresta: usize,
            canon: usize,
            outra: usize,
            a: (Id, i32),
            b: (Id, i32),
            pontas: [[f32; 3]; 2],
            meio: [f32; 3],
        }
        let mut cruas: Vec<Crua> = vec![];
        for p in partes {
            let m = p.malha;
            for (e, par) in m.edge_faces.iter().enumerate() {
                let [x, y] = *par;
                let (x, y) = match (x == NO_FACE, y == NO_FACE) {
                    (true, true) => continue,
                    (false, true) => (x, x),
                    (true, false) => (y, y),
                    _ => (x, y),
                };
                let (Some(&ix), Some(&iy)) = (por_face.get(&(p.corpo, x as usize)), por_face.get(&(p.corpo, y as usize))) else { continue };
                let Some(pontas) = pontas_da_aresta(m, e) else { continue };
                let meio = m.edge_mids.get(e * 3..e * 3 + 3).map_or(pontas[0], |v| [v[0], v[1], v[2]]);
                let (canon, outra) = if ix <= iy { (ix, iy) } else { (iy, ix) };
                let (fa, fb) = (&brutas[ix], &brutas[iy]);
                cruas.push(Crua { corpo: p.corpo, aresta: e, canon, outra, a: (fa.dono, fa.papel), b: (fb.dono, fb.papel), pontas, meio });
            }
        }
        cruas.sort_by_key(|c| (c.canon, c.outra, c.corpo, c.aresta));
        let mut arestas: Vec<ArestaN> = Vec::with_capacity(cruas.len());
        let mut atual = (usize::MAX, 0u32);
        for c in cruas {
            if atual.0 == c.canon {
                atual.1 += 1;
            } else {
                atual = (c.canon, 1);
            }
            arestas.push(ArestaN { corpo: c.corpo, aresta: c.aresta, face: c.canon, numero: atual.1, a: c.a, b: c.b, pontas: c.pontas, meio: c.meio });
        }
        let por_aresta = arestas.iter().enumerate().map(|(i, a)| ((a.corpo, a.aresta), i)).collect();
        SubNomes { faces: brutas, arestas, por_face, por_aresta }
    }

    pub fn vazia(&self) -> bool {
        self.faces.is_empty()
    }

    // ---------------------------------------------------------------- do elemento da tela ao nome

    pub fn face_de(&self, corpo: usize, face: usize) -> Option<&FaceN> {
        self.por_face.get(&(corpo, face)).map(|i| &self.faces[*i])
    }

    pub fn aresta_de(&self, corpo: usize, aresta: usize) -> Option<&ArestaN> {
        self.por_aresta.get(&(corpo, aresta)).map(|i| &self.arestas[*i])
    }

    /// A aresta do corpo que tem esta ponta (a de menor face e número, para o nome ser único), com o
    /// número da ponta (1 ou 2).
    pub fn aresta_com_ponta(&self, corpo: usize, pos: [f32; 3]) -> Option<(&ArestaN, u32)> {
        let perto = |q: [f32; 3]| (0..3).all(|i| (q[i] - pos[i]).abs() < 1e-3);
        self.arestas.iter().filter(|a| a.corpo == corpo).filter_map(|a| (0..2).find(|k| perto(a.pontas[*k])).map(|k| (a, k as u32 + 1))).min_by_key(|(a, _)| (a.face, a.numero))
    }

    /// O vértice (aresta e ponta) mais perto de `p`, em qualquer corpo, se estiver a `tol` mm ou menos.
    /// Serve a comandos que guardam o canto como ponto: o painel mostra o nome do vértice que está ali.
    pub fn vertice_perto(&self, p: [f64; 3], tol: f64) -> Option<(&ArestaN, u32)> {
        let d = |q: [f32; 3]| (0..3).map(|i| (q[i] as f64 - p[i]).powi(2)).sum::<f64>().sqrt();
        self.arestas
            .iter()
            .flat_map(|a| (0..2).map(move |k| (a, k)))
            .map(|(a, k)| (a, k as u32 + 1, d(a.pontas[k])))
            .filter(|x| x.2 <= tol)
            .min_by(|x, y| x.2.total_cmp(&y.2).then((x.0.face, x.0.numero).cmp(&(y.0.face, y.0.numero))))
            .map(|(a, k, _)| (a, k))
    }

    /// A aresta de corpo ou superfície cujo meio está mais perto de `p`, se a `tol` mm ou menos.
    pub fn aresta_perto(&self, p: [f64; 3], tol: f64) -> Option<&ArestaN> {
        let d = |a: &ArestaN| (0..3).map(|i| (a.meio[i] as f64 - p[i]).powi(2)).sum::<f64>().sqrt();
        self.arestas.iter().filter(|a| d(a) <= tol).min_by(|x, y| d(x).total_cmp(&d(y)))
    }

    /// A face plana que passa por `p` (a mais perto do ponto de referência dela), se houver.
    pub fn face_plana_em(&self, p: [f64; 3], tol: f64) -> Option<&FaceN> {
        let dist_plano = |f: &FaceN| (0..3).map(|i| (p[i] - f.ponto[i]) * f.dir[i]).sum::<f64>().abs();
        let dist_ponto = |f: &FaceN| (0..3).map(|i| (p[i] - f.ponto[i]).powi(2)).sum::<f64>().sqrt();
        self.faces.iter().filter(|f| f.tipo == 0 && dist_plano(f) <= tol).min_by(|a, b| dist_ponto(a).total_cmp(&dist_ponto(b)))
    }

    // ---------------------------------------------------------------- da chave ao elemento

    /// A face da chave: mesmo dono e papel; se houver mais de uma, a de tipo, eixo e área mais parecidos.
    pub fn face_da_chave(&self, k: &FaceKey) -> Option<&FaceN> {
        let tipo = crate::refs::kind_of(k.kind);
        self.faces.iter().filter(|f| f.dono == k.owner && f.papel == k.role).min_by(|a, b| {
            let nota = |f: &FaceN| (if f.tipo == tipo { 0. } else { 10. }) + (1. - dot(f.dir, k.normal_or_axis)).abs() + (f.area - k.area).abs() / k.area.abs().max(1e-9);
            nota(a).total_cmp(&nota(b))
        })
    }

    /// A aresta da chave: as duas faces pelo dono e papel (em qualquer ordem). Em superfície, o índice
    /// da aresta que a chave guarda desempata; sem desempate, a primeira.
    pub fn aresta_da_chave(&self, k: &EdgeKey) -> Option<&ArestaN> {
        let (ka, kb) = ((k.a.owner, k.a.role), (k.b.owner, k.b.role));
        let mut c = self.arestas.iter().filter(|a| (a.a == ka && a.b == kb) || (a.a == kb && a.b == ka));
        match k.edge {
            Some(i) => {
                let todas: Vec<&ArestaN> = c.collect();
                todas.iter().find(|a| a.aresta == i as usize).copied().or_else(|| todas.first().copied())
            }
            None => c.next(),
        }
    }

    // ---------------------------------------------------------------- nomes

    fn dono_nome(doc: &Document, dono: Id) -> String {
        doc.name_of(dono)
    }

    pub fn nome_da_face(&self, doc: &Document, f: &FaceN, tr: Tradutor) -> String {
        format!("{} · {} {}", Self::dono_nome(doc, f.dono), tr("Face"), f.numero)
    }

    pub fn nome_da_aresta(&self, doc: &Document, a: &ArestaN, tr: Tradutor) -> String {
        let f = &self.faces[a.face];
        format!("{} · {} {} · {} {}", Self::dono_nome(doc, f.dono), tr("Face"), f.numero, tr("Aresta"), a.numero)
    }

    pub fn nome_do_vertice(&self, doc: &Document, a: &ArestaN, ponta: u32, tr: Tradutor) -> String {
        format!("{} · {} {}", self.nome_da_aresta(doc, a, tr), tr("Vértice"), ponta)
    }

    /// Nome da face pela chave; `None` se a tabela não a conhece (peça ainda não calculada).
    pub fn nome_face_chave(&self, doc: &Document, k: &FaceKey, tr: Tradutor) -> Option<String> {
        self.face_da_chave(k).map(|f| self.nome_da_face(doc, f, tr))
    }

    /// Nome da aresta pela chave; `None` se a tabela não a conhece.
    pub fn nome_aresta_chave(&self, doc: &Document, k: &EdgeKey, tr: Tradutor) -> Option<String> {
        self.aresta_da_chave(k).map(|a| self.nome_da_aresta(doc, a, tr))
    }

    /// Descrição do papel da face, para a dica ("tampa final", "lateral 3", "furo 1, boca").
    pub fn papel_da_face(f: &FaceN, tr: Tradutor) -> String {
        crate::refs::nome_papel(f.papel, tr)
    }

    // ---------------------------------------------------------------- hierarquia para a árvore

    /// Os donos na ordem em que aparecem, com as faces (e as arestas de cada face) deles.
    pub fn donos(&self) -> Vec<Id> {
        let mut v: Vec<Id> = vec![];
        for f in &self.faces {
            if !v.contains(&f.dono) {
                v.push(f.dono);
            }
        }
        v
    }

    /// Índices (em `faces`) das faces de um dono, na ordem do número.
    pub fn faces_do_dono(&self, dono: Id) -> Vec<usize> {
        (0..self.faces.len()).filter(|i| self.faces[*i].dono == dono).collect()
    }

    /// Índices (em `arestas`) das arestas de uma face, na ordem do número.
    pub fn arestas_da_face(&self, face: usize) -> Vec<usize> {
        (0..self.arestas.len()).filter(|i| self.arestas[*i].face == face).collect()
    }

    /// `true` se o dono é uma superfície de referência (as faces dela têm papel de superfície).
    pub fn e_superficie(&self, dono: Id) -> bool {
        self.faces.iter().any(|f| f.dono == dono && f.papel >= PAPEL_SUPERFICIE)
    }
}

