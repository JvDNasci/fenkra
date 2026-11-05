// SPDX-License-Identifier: LGPL-2.1-or-later
//! Forma livre por subdivisão (B153): a gaiola de controle, a malha semiaresta e o Catmull-Clark.
//!
//! A gaiola é o dado que a pessoa edita e o que vai para o arquivo. Nós e faces têm número persistente
//! (`id`), que nunca é reaproveitado: é por ele que os vínculos e as faces do B-rep continuam
//! apontando para o mesmo lugar depois de inserir uma aresta. A topologia semiaresta ([`Topo`]) é
//! derivada e refeita sempre que preciso.
//!
//! Cada passo de refinamento faz um ponto por face, um por aresta e move os nós antigos. O vinco com
//! nitidez `s` usa a regra de aresta viva por `s` passos e depois a regra lisa; entre os dois, a média
//! ponderada pela parte fracionária. Fontes em `docs/arquitetura/procedencia.md`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type V3 = [f64; 3];

pub(crate) const NADA: u32 = u32::MAX;
/// Nitidez de um vinco cheio ou de uma borda: nunca amolece.
pub const VIVO: f64 = f64::INFINITY;
/// Peso 1 é vinco cheio; abaixo disso a nitidez cresce até este número de passos.
pub const PASSOS_DO_PESO: f64 = 3.;
/// Marca dos números do lado espelhado: a metade espelhada não é editada, nasce da outra.
pub const ESPELHO: u32 = 1 << 30;
/// Nós mais perto do plano de simetria do que isto ficam presos a ele.
pub const TOL_PLANO: f64 = 1e-6;

pub(crate) fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
pub(crate) fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
pub(crate) fn mul(a: V3, k: f64) -> V3 {
    [a[0] * k, a[1] * k, a[2] * k]
}
pub(crate) fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
pub(crate) fn cross(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
pub(crate) fn norma(a: V3) -> f64 {
    dot(a, a).sqrt()
}
pub(crate) fn unit(a: V3) -> Option<V3> {
    let n = norma(a);
    (n > 1e-12).then(|| mul(a, 1. / n))
}
pub(crate) fn lerp(a: V3, b: V3, t: f64) -> V3 {
    add(mul(a, 1. - t), mul(b, t))
}

/// Plano tangente pedido num nó: a normal, e o quanto o anel vizinho foi aberto em relação ao nó.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct Tangente {
    pub normal: V3,
    pub tensao: f64,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct NoG {
    pub id: u32,
    pub p: V3,
    /// canto: a superfície passa pelo nó e nenhuma tangência o atravessa
    #[serde(default)]
    pub canto: bool,
    #[serde(default)]
    pub tangente: Option<Tangente>,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct FaceG {
    pub id: u32,
    /// números dos nós, no sentido anti-horário visto de fora
    pub nos: Vec<u32>,
}

/// Aresta vincada, com `a < b`. Peso 0 não fica guardado.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct Vinco {
    pub a: u32,
    pub b: u32,
    pub peso: f64,
}

/// Plano de simetria: `eixo` é a normal (0 X, 1 Y, 2 Z) e `pos` a coordenada dele.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct Simetria {
    pub eixo: u8,
    pub pos: f64,
}

impl Simetria {
    pub fn distancia(&self, p: V3) -> f64 {
        p[self.eixo as usize] - self.pos
    }

    pub fn refletir(&self, p: V3) -> V3 {
        let mut q = p;
        q[self.eixo as usize] = 2. * self.pos - p[self.eixo as usize];
        q
    }
}

/// A malha de controle. Só a metade do lado positivo é guardada quando há simetria.
#[derive(Clone, PartialEq, Debug, Default, Serialize, Deserialize)]
pub struct Gaiola {
    pub nos: Vec<NoG>,
    pub faces: Vec<FaceG>,
    #[serde(default)]
    pub vincos: Vec<Vinco>,
    #[serde(default)]
    pub simetria: Option<Simetria>,
    pub prox_no: u32,
    pub prox_face: u32,
}

/// Peso de 0 a 1 para passos de nitidez.
pub fn nitidez_do_peso(peso: f64) -> f64 {
    if peso >= 1. {
        VIVO
    } else {
        peso.max(0.) * PASSOS_DO_PESO
    }
}

pub(crate) fn par(a: u32, b: u32) -> (u32, u32) {
    if a < b {
        (a, b)
    } else {
        (b, a)
    }
}

/// Topologia semiaresta de uma malha de polígonos orientados.
#[derive(Clone, Debug)]
pub struct Topo {
    pub prox: Vec<u32>,
    pub ant: Vec<u32>,
    pub gemea: Vec<u32>,
    /// nó de onde a semiaresta sai
    pub origem: Vec<u32>,
    pub face: Vec<u32>,
    /// número da aresta (a semiaresta e a gêmea têm o mesmo)
    pub aresta: Vec<u32>,
    pub n_arestas: usize,
    /// primeira semiaresta de cada face
    pub da_face: Vec<u32>,
    /// semiaresta saindo de cada nó; no nó de borda, a primeira do leque
    pub do_no: Vec<u32>,
}

impl Topo {
    /// Monta e confere: aresta com mais de duas faces, orientação trocada e nó em que duas folhas se
    /// tocam dão erro, porque nenhuma regra de subdivisão vale neles.
    pub fn novo(n_nos: usize, faces: &[Vec<u32>]) -> Result<Topo, String> {
        let total: usize = faces.iter().map(Vec::len).sum();
        let mut t = Topo {
            prox: Vec::with_capacity(total),
            ant: Vec::with_capacity(total),
            gemea: vec![NADA; total],
            origem: Vec::with_capacity(total),
            face: Vec::with_capacity(total),
            aresta: vec![NADA; total],
            n_arestas: 0,
            da_face: Vec::with_capacity(faces.len()),
            do_no: vec![NADA; n_nos],
        };
        let mut dirigidas: HashMap<(u32, u32), u32> = HashMap::with_capacity(total);
        for (fi, f) in faces.iter().enumerate() {
            if f.len() < 3 {
                return Err(format!("face {fi} com menos de três nós"));
            }
            let base = t.prox.len() as u32;
            t.da_face.push(base);
            let n = f.len() as u32;
            for (k, &v) in f.iter().enumerate() {
                let k = k as u32;
                if v as usize >= n_nos {
                    return Err(format!("face {fi} cita um nó que não existe"));
                }
                let w = f[((k + 1) % n) as usize];
                t.prox.push(base + (k + 1) % n);
                t.ant.push(base + (k + n - 1) % n);
                t.origem.push(v);
                t.face.push(fi as u32);
                if v == w {
                    return Err(format!("face {fi} repete um nó"));
                }
                if dirigidas.insert((v, w), base + k).is_some() {
                    return Err(format!("aresta {v}-{w} repetida no mesmo sentido: faces com orientação trocada ou mais de duas faces na aresta"));
                }
            }
        }
        for (&(v, w), &h) in &dirigidas {
            if let Some(&g) = dirigidas.get(&(w, v)) {
                t.gemea[h as usize] = g;
            }
        }
        for h in 0..total {
            if t.aresta[h] == NADA {
                let e = t.n_arestas as u32;
                t.n_arestas += 1;
                t.aresta[h] = e;
                let g = t.gemea[h];
                if g != NADA {
                    t.aresta[g as usize] = e;
                }
            }
        }
        // nó de borda começa na semiaresta cuja anterior não tem gêmea
        for h in 0..total as u32 {
            let v = t.origem[h as usize] as usize;
            let borda = t.gemea[t.ant[h as usize] as usize] == NADA;
            if t.do_no[v] == NADA || borda {
                t.do_no[v] = h;
            }
        }
        let mut por_no = vec![0usize; n_nos];
        for &v in &t.origem {
            por_no[v as usize] += 1;
        }
        for (v, &n) in por_no.iter().enumerate() {
            if n > 0 && t.leque(v as u32).0.len() != n {
                return Err(format!("nó {v} une duas folhas da malha"));
            }
        }
        Ok(t)
    }

    /// Fim da semiaresta.
    pub fn destino(&self, h: u32) -> u32 {
        self.origem[self.prox[h as usize] as usize]
    }

    /// Semiarestas saindo do nó em ordem de giro, e se o leque fecha. No nó de borda, a aresta de
    /// entrada (a anterior da primeira) não aparece aqui.
    pub fn leque(&self, v: u32) -> (Vec<u32>, bool) {
        let h0 = self.do_no[v as usize];
        let mut out = vec![];
        if h0 == NADA {
            return (out, true);
        }
        let mut h = h0;
        loop {
            out.push(h);
            let g = self.gemea[h as usize];
            if g == NADA {
                return (out, false);
            }
            h = self.prox[g as usize];
            if h == h0 || out.len() > self.prox.len() {
                return (out, true);
            }
        }
    }

    /// Vizinhos do nó na ordem do leque; num nó de borda as duas pontas da borda ficam nas extremidades.
    pub fn vizinhos(&self, v: u32) -> Vec<u32> {
        let (l, fechado) = self.leque(v);
        let mut out: Vec<u32> = l.iter().map(|h| self.destino(*h)).collect();
        if !fechado {
            out.insert(0, self.origem[self.ant[l[0] as usize] as usize]);
        }
        out
    }

    pub fn borda(&self, h: u32) -> bool {
        self.gemea[h as usize] == NADA
    }
}

/// Uma malha num nível de refinamento.
#[derive(Clone, Debug)]
pub struct Nivel {
    pub pos: Vec<V3>,
    pub faces: Vec<Vec<u32>>,
    /// nitidez das arestas vincadas (a borda não aparece: é sempre viva)
    pub nitidez: HashMap<(u32, u32), f64>,
    pub canto: Vec<bool>,
    /// número do nó da gaiola de onde o vértice veio, ou `u32::MAX` se nasceu no refinamento
    pub no: Vec<u32>,
    /// de cada face: número da face da gaiola e índice do pedaço dentro dela
    pub origem: Vec<(u32, u32)>,
}

impl Nivel {
    pub fn nitidez_de(&self, t: &Topo, h: u32) -> f64 {
        if t.borda(h) {
            return VIVO;
        }
        let (a, b) = (t.origem[h as usize], t.destino(h));
        self.nitidez.get(&par(a, b)).copied().unwrap_or(0.)
    }

    /// Arestas com nitidez que chegam ao nó (a ponta oposta e a nitidez), bordas incluídas.
    pub fn vivas_em(&self, t: &Topo, v: u32, leque: &[u32], fechado: bool) -> Vec<(u32, f64)> {
        let mut vivas: Vec<(u32, f64)> = leque.iter().filter_map(|&h| {
            let s = self.nitidez_de(t, h);
            (s > 0.).then(|| (t.destino(h), s))
        }).collect();
        if !fechado {
            vivas.push((t.origem[t.ant[leque[0] as usize] as usize], VIVO));
        }
        debug_assert!(leque.iter().all(|h| t.origem[*h as usize] == v));
        vivas
    }

    /// Um passo de Catmull-Clark.
    pub fn refinar(&self) -> Result<Nivel, String> {
        let t = Topo::novo(self.pos.len(), &self.faces)?;
        let (nv, nf, ne) = (self.pos.len(), self.faces.len(), t.n_arestas);
        let centro: Vec<V3> = self.faces.iter().map(|f| mul(f.iter().fold([0.; 3], |a, v| add(a, self.pos[*v as usize])), 1. / f.len() as f64)).collect();
        // `meio` entra na regra do nó; `e_pos` é o ponto novo da aresta, misturado pela nitidez
        let mut meio = vec![[0.; 3]; ne];
        let mut e_pos = vec![[0.; 3]; ne];
        let mut visto = vec![false; ne];
        for h in 0..t.prox.len() as u32 {
            let e = t.aresta[h as usize] as usize;
            if visto[e] {
                continue;
            }
            visto[e] = true;
            let (a, b) = (self.pos[t.origem[h as usize] as usize], self.pos[t.destino(h) as usize]);
            meio[e] = mul(add(a, b), 0.5);
            let s = self.nitidez_de(&t, h);
            e_pos[e] = if s >= 1. {
                meio[e]
            } else {
                let g = t.gemea[h as usize];
                let liso = mul(add(add(a, b), add(centro[t.face[h as usize] as usize], centro[t.face[g as usize] as usize])), 0.25);
                lerp(liso, meio[e], s)
            };
        }
        let mut pos = Vec::with_capacity(nv + nf + ne);
        let mut canto = vec![false; nv + nf + ne];
        let mut no = vec![NADA; nv + nf + ne];
        for v in 0..nv as u32 {
            let (l, fechado) = t.leque(v);
            let iv = v as usize;
            no[iv] = self.no[iv];
            canto[iv] = self.canto[iv];
            let p = self.pos[iv];
            if l.is_empty() {
                pos.push(p);
                continue;
            }
            let vivas = self.vivas_em(&t, v, &l, fechado);
            let vinco = |vv: &[(u32, f64)]| add(mul(p, 0.75), mul(add(self.pos[vv[0].0 as usize], self.pos[vv[1].0 as usize]), 0.125));
            let novo = if self.canto[iv] || (!fechado && l.len() == 1) {
                p
            } else if !fechado {
                if vivas.len() == 2 {
                    vinco(&vivas)
                } else {
                    p
                }
            } else {
                let n = l.len() as f64;
                let (mut f, mut e) = ([0.; 3], [0.; 3]);
                for &h in &l {
                    f = add(f, centro[t.face[h as usize] as usize]);
                    e = add(e, meio[t.aresta[h as usize] as usize]);
                }
                let liso = add(add(mul(p, (n - 3.) / n), mul(f, 1. / (n * n))), mul(e, 2. / (n * n)));
                match vivas.len() {
                    0 | 1 => liso,
                    2 => mistura(vinco(&vivas), liso, &vivas),
                    _ => mistura(p, liso, &vivas),
                }
            };
            pos.push(novo);
        }
        pos.extend(centro.iter().copied());
        pos.extend(e_pos.iter().copied());
        let mut faces = Vec::with_capacity(t.prox.len());
        let mut origem = Vec::with_capacity(t.prox.len());
        for (fi, f) in self.faces.iter().enumerate() {
            let h0 = t.da_face[fi];
            let n = f.len() as u32;
            for k in 0..n {
                let h = h0 + k;
                let c = (nv + fi) as u32;
                let e1 = (nv + nf) as u32 + t.aresta[h as usize];
                let e0 = (nv + nf) as u32 + t.aresta[t.ant[h as usize] as usize];
                faces.push(vec![t.origem[h as usize], e1, c, e0]);
                let (id, sub) = self.origem[fi];
                origem.push((id, sub * n.max(4) + k));
            }
        }
        // cada vinco vira duas metades com um passo a menos
        let mut nitidez = HashMap::new();
        for h in 0..t.prox.len() as u32 {
            if t.borda(h) {
                continue;
            }
            let (a, b) = (t.origem[h as usize], t.destino(h));
            if a > b {
                continue;
            }
            let Some(&s) = self.nitidez.get(&(a, b)) else { continue };
            let filha = if s.is_infinite() { VIVO } else { s - 1. };
            if filha > 0. {
                let m = (nv + nf) as u32 + t.aresta[h as usize];
                nitidez.insert(par(a, m), filha);
                nitidez.insert(par(m, b), filha);
            }
        }
        Ok(Nivel { pos, faces, nitidez, canto, no, origem })
    }

    /// Onde cada vértice acaba depois de infinitos passos (máscaras do limite).
    pub fn limite(&self) -> Result<Vec<V3>, String> {
        let t = Topo::novo(self.pos.len(), &self.faces)?;
        let mut out = self.pos.clone();
        for v in 0..self.pos.len() as u32 {
            let (l, fechado) = t.leque(v);
            if l.is_empty() {
                continue;
            }
            let vivas = self.vivas_em(&t, v, &l, fechado);
            let p = self.pos[v as usize];
            if self.canto[v as usize] || vivas.len() > 2 || (!fechado && l.len() == 1) {
                continue;
            }
            if vivas.len() == 2 {
                let (a, b) = (self.pos[vivas[0].0 as usize], self.pos[vivas[1].0 as usize]);
                out[v as usize] = add(mul(p, 4. / 6.), mul(add(a, b), 1. / 6.));
                continue;
            }
            if l.iter().all(|h| self.faces[t.face[*h as usize] as usize].len() == 4) {
                let n = l.len() as f64;
                let (mut soma_e, mut soma_f) = ([0.; 3], [0.; 3]);
                for &h in &l {
                    soma_e = add(soma_e, self.pos[t.destino(h) as usize]);
                    // nó diagonal do quadrilátero
                    soma_f = add(soma_f, self.pos[t.origem[t.prox[t.prox[h as usize] as usize] as usize] as usize]);
                }
                out[v as usize] = mul(add(add(mul(p, n * n), mul(soma_e, 4.)), soma_f), 1. / (n * n + 5. * n));
            }
        }
        Ok(out)
    }
}

/// A regra dura do nó enquanto as arestas ainda têm nitidez; a parte fracionária leva à regra lisa.
fn mistura(duro: V3, liso: V3, vivas: &[(u32, f64)]) -> V3 {
    let s = vivas.iter().map(|x| x.1.min(1.)).sum::<f64>() / vivas.len() as f64;
    lerp(liso, duro, s.clamp(0., 1.))
}

/// Superfície pronta para desenhar: quatro cantos por quadrilátero, com a normal já quebrada nos vincos.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MalhaLisa {
    pub pos: Vec<V3>,
    pub nrm: Vec<V3>,
    pub tris: Vec<[u32; 3]>,
    /// face da gaiola de cada triângulo
    pub face: Vec<u32>,
}

impl MalhaLisa {
    /// Volume pela soma dos tetraedros (só faz sentido em malha fechada).
    pub fn volume(&self) -> f64 {
        self.tris.iter().map(|t| dot(self.pos[t[0] as usize], cross(self.pos[t[1] as usize], self.pos[t[2] as usize])) / 6.).sum()
    }

    pub fn caixa(&self) -> Option<(V3, V3)> {
        let mut it = self.pos.iter();
        let p0 = *it.next()?;
        Some(it.fold((p0, p0), |(a, b), p| ([a[0].min(p[0]), a[1].min(p[1]), a[2].min(p[2])], [b[0].max(p[0]), b[1].max(p[1]), b[2].max(p[2])])))
    }
}

impl Gaiola {
    pub fn por_nos(&self) -> HashMap<u32, usize> {
        self.nos.iter().enumerate().map(|(i, n)| (n.id, i)).collect()
    }

    pub fn no(&self, id: u32) -> Option<&NoG> {
        self.nos.iter().find(|n| n.id == id)
    }

    pub fn no_mut(&mut self, id: u32) -> Option<&mut NoG> {
        self.nos.iter_mut().find(|n| n.id == id)
    }

    pub fn face(&self, id: u32) -> Option<&FaceG> {
        self.faces.iter().find(|f| f.id == id)
    }

    pub(crate) fn novo_no(&mut self, p: V3) -> u32 {
        let id = self.prox_no;
        self.prox_no += 1;
        self.nos.push(NoG { id, p, canto: false, tangente: None });
        id
    }

    pub(crate) fn nova_face(&mut self, nos: Vec<u32>) -> u32 {
        let id = self.prox_face;
        self.prox_face += 1;
        self.faces.push(FaceG { id, nos });
        id
    }

    /// Para montar gaiola fora deste crate (testes e importação).
    pub fn novo_para_teste(&mut self, p: V3) -> u32 {
        self.novo_no(p)
    }

    pub fn face_para_teste(&mut self, nos: Vec<u32>) -> u32 {
        self.nova_face(nos)
    }

    pub fn pos(&self, id: u32) -> Option<V3> {
        self.no(id).map(|n| n.p)
    }

    /// Arestas sem repetição, com `a < b`.
    pub fn arestas(&self) -> Vec<(u32, u32)> {
        let mut v: Vec<(u32, u32)> = self.faces.iter().flat_map(|f| (0..f.nos.len()).map(|k| par(f.nos[k], f.nos[(k + 1) % f.nos.len()]))).collect();
        v.sort_unstable();
        v.dedup();
        v
    }

    pub fn peso(&self, a: u32, b: u32) -> f64 {
        let (a, b) = par(a, b);
        self.vincos.iter().find(|v| v.a == a && v.b == b).map_or(0., |v| v.peso)
    }

    pub fn definir_vinco(&mut self, a: u32, b: u32, peso: f64) {
        let (a, b) = par(a, b);
        self.vincos.retain(|v| !(v.a == a && v.b == b));
        if peso > 0. {
            self.vincos.push(Vinco { a, b, peso: peso.min(1.) });
        }
    }

    pub fn centro_face(&self, id: u32) -> Option<V3> {
        let f = self.face(id)?;
        let s = f.nos.iter().filter_map(|n| self.pos(*n)).fold([0.; 3], add);
        Some(mul(s, 1. / f.nos.len() as f64))
    }

    /// Normal da face pela soma dos produtos vetoriais (vale para polígono torto).
    pub fn normal_face(&self, id: u32) -> Option<V3> {
        let f = self.face(id)?;
        let p: Vec<V3> = f.nos.iter().filter_map(|n| self.pos(*n)).collect();
        let mut s = [0.; 3];
        for k in 0..p.len() {
            s = add(s, cross(p[k], p[(k + 1) % p.len()]));
        }
        unit(s)
    }

    pub fn normal_no(&self, id: u32) -> Option<V3> {
        unit(self.faces.iter().filter(|f| f.nos.contains(&id)).filter_map(|f| self.normal_face(f.id)).fold([0.; 3], add))
    }

    /// A malha inteira: com simetria, a metade guardada mais o reflexo, fundidos no plano. Os números
    /// do reflexo levam a marca [`ESPELHO`]; os nós do plano ficam nele.
    pub fn inteira(&self) -> Gaiola {
        let Some(s) = self.simetria else { return self.clone() };
        let mut g = self.clone();
        g.simetria = None;
        let no_plano = |p: V3| s.distancia(p).abs() < TOL_PLANO;
        let mut par_de: HashMap<u32, u32> = HashMap::new();
        for n in &mut g.nos {
            if no_plano(n.p) {
                n.p[s.eixo as usize] = s.pos;
            }
        }
        for n in &self.nos {
            if no_plano(n.p) {
                par_de.insert(n.id, n.id);
            } else {
                par_de.insert(n.id, n.id | ESPELHO);
                g.nos.push(NoG { id: n.id | ESPELHO, p: s.refletir(n.p), canto: n.canto, tangente: None });
            }
        }
        for f in &self.faces {
            let mut nos: Vec<u32> = f.nos.iter().map(|n| par_de[n]).collect();
            nos.reverse();
            g.faces.push(FaceG { id: f.id | ESPELHO, nos });
        }
        for v in &self.vincos {
            let (a, b) = par(par_de[&v.a], par_de[&v.b]);
            if (a, b) != (v.a, v.b) {
                g.vincos.push(Vinco { a, b, peso: v.peso });
            }
        }
        g
    }

    pub fn nivel0(&self) -> Nivel {
        let idx = self.por_nos();
        Nivel {
            pos: self.nos.iter().map(|n| n.p).collect(),
            faces: self.faces.iter().map(|f| f.nos.iter().map(|n| idx[n] as u32).collect()).collect(),
            nitidez: self
                .vincos
                .iter()
                .filter_map(|v| {
                    let (a, b) = (*idx.get(&v.a)? as u32, *idx.get(&v.b)? as u32);
                    Some((par(a, b), nitidez_do_peso(v.peso)))
                })
                .collect(),
            canto: self.nos.iter().map(|n| n.canto).collect(),
            no: self.nos.iter().map(|n| n.id).collect(),
            origem: self.faces.iter().map(|f| (f.id, 0)).collect(),
        }
    }

    /// `niveis` passos sobre a malha inteira.
    pub fn refinada(&self, niveis: u32) -> Result<Nivel, String> {
        let mut n = self.inteira().nivel0();
        for _ in 0..niveis {
            n = n.refinar()?;
        }
        Ok(n)
    }

    /// Superfície de desenho: `niveis` passos, vértices levados ao limite e normais por setor.
    pub fn lisa(&self, niveis: u32) -> Result<MalhaLisa, String> {
        if self.faces.is_empty() {
            return Err("a gaiola não tem faces".into());
        }
        let n = self.refinada(niveis.max(1))?;
        let lim = n.limite()?;
        let t = Topo::novo(n.pos.len(), &n.faces)?;
        let normais: Vec<V3> = n
            .faces
            .iter()
            .map(|v| {
                let mut s = [0.; 3];
                for k in 0..v.len() {
                    s = add(s, cross(lim[v[k] as usize], lim[v[(k + 1) % v.len()] as usize]));
                }
                s
            })
            .collect();
        let mut m = MalhaLisa::default();
        for (fi, f) in n.faces.iter().enumerate() {
            let base = m.pos.len() as u32;
            for k in 0..f.len() {
                let h = t.da_face[fi] + k as u32;
                m.pos.push(lim[f[k] as usize]);
                m.nrm.push(unit(normal_do_setor(&n, &t, h, &normais)).unwrap_or([0., 0., 1.]));
            }
            for k in 1..f.len() as u32 - 1 {
                m.tris.push([base, base + k, base + k + 1]);
                m.face.push(n.origem[fi].0);
            }
        }
        Ok(m)
    }
}

/// Soma das normais das faces ao redor do canto `h`, parando nas arestas vivas.
fn normal_do_setor(n: &Nivel, t: &Topo, h: u32, normais: &[V3]) -> V3 {
    let mut s = normais[t.face[h as usize] as usize];
    // gira por um lado (saindo pela aresta `h`)
    let mut a = h;
    for _ in 0..64 {
        if n.nitidez_de(t, a) >= 1. {
            break;
        }
        a = t.prox[t.gemea[a as usize] as usize];
        if a == h {
            return s;
        }
        s = add(s, normais[t.face[a as usize] as usize]);
    }
    // e pelo outro (entrando pela aresta anterior)
    let mut b = h;
    for _ in 0..64 {
        let e = t.ant[b as usize];
        if n.nitidez_de(t, e) >= 1. {
            break;
        }
        b = t.gemea[e as usize];
        if b == a {
            break;
        }
        s = add(s, normais[t.face[b as usize] as usize]);
    }
    s
}
