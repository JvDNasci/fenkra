// SPDX-License-Identifier: LGPL-2.1-or-later
//! Edição da gaiola (B153): primitivas e as operações que mudam a topologia. Toda operação preserva
//! os números das faces e nós que continuam existindo; o que nasce ganha número novo, nunca repetido.

use crate::subd::*;
use crate::subd::{add, cross, dot, lerp, mul, norma, par, sub, unit};
use std::collections::{HashMap, HashSet};

/// Como a forma nasce.
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub enum PrimitivaForma {
    Caixa,
    Plano,
    Cilindro,
    Esfera,
}

fn indice_em(f: &[u32], v: u32) -> Option<usize> {
    f.iter().position(|x| *x == v)
}

impl Gaiola {
    /// Caixa centrada em `c`, com `div` faces por lado em cada eixo.
    pub fn caixa(c: V3, tam: V3, div: [u32; 3]) -> Gaiola {
        let d = div.map(|x| x.max(1));
        let ponto = |i: u32, j: u32, k: u32| [c[0] + tam[0] * (i as f64 / d[0] as f64 - 0.5), c[1] + tam[1] * (j as f64 / d[1] as f64 - 0.5), c[2] + tam[2] * (k as f64 / d[2] as f64 - 0.5)];
        Gaiola::grade_fechada(d, ponto)
    }

    /// Esfera de quadriláteros: a caixa com os nós levados à esfera (oito nós de valência 3).
    pub fn esfera(c: V3, raio: f64, div: u32) -> Gaiola {
        let d = div.max(1);
        let ponto = |i: u32, j: u32, k: u32| {
            let u = [i, j, k].map(|x| 2. * x as f64 / d as f64 - 1.);
            let n = unit(u).unwrap_or([0., 0., 1.]);
            add(c, mul(n, raio))
        };
        Gaiola::grade_fechada([d; 3], ponto)
    }

    /// Superfície de uma grade `d`: só os pontos da casca entram.
    fn grade_fechada(d: [u32; 3], ponto: impl Fn(u32, u32, u32) -> V3) -> Gaiola {
        let mut g = Gaiola::default();
        let mut nos: HashMap<[u32; 3], u32> = HashMap::new();
        let mut no = |g: &mut Gaiola, q: [u32; 3]| *nos.entry(q).or_insert_with(|| g.novo_no(ponto(q[0], q[1], q[2])));
        // cada lado: eixo fixo, valor, e os dois eixos do plano em ordem que dá a normal para fora
        for (fixo, a, b) in [(0usize, 1usize, 2usize), (1, 2, 0), (2, 0, 1)] {
            for (valor, inverte) in [(d[fixo], false), (0, true)] {
                for i in 0..d[a] {
                    for j in 0..d[b] {
                        let q = |x: u32, y: u32| {
                            let mut p = [0; 3];
                            p[fixo] = valor;
                            p[a] = x;
                            p[b] = y;
                            p
                        };
                        let mut f = vec![no(&mut g, q(i, j)), no(&mut g, q(i + 1, j)), no(&mut g, q(i + 1, j + 1)), no(&mut g, q(i, j + 1))];
                        if inverte {
                            f.reverse();
                        }
                        g.nova_face(f);
                    }
                }
            }
        }
        g
    }

    /// Plano aberto no XY, centrado em `c`.
    pub fn plano(c: V3, tam: [f64; 2], div: [u32; 2]) -> Gaiola {
        let d = div.map(|x| x.max(1));
        let mut g = Gaiola::default();
        let mut ids = vec![];
        for j in 0..=d[1] {
            for i in 0..=d[0] {
                ids.push(g.novo_no([c[0] + tam[0] * (i as f64 / d[0] as f64 - 0.5), c[1] + tam[1] * (j as f64 / d[1] as f64 - 0.5), c[2]]));
            }
        }
        let at = |i: u32, j: u32| ids[(j * (d[0] + 1) + i) as usize];
        for j in 0..d[1] {
            for i in 0..d[0] {
                g.nova_face(vec![at(i, j), at(i + 1, j), at(i + 1, j + 1), at(i, j + 1)]);
            }
        }
        g
    }

    /// Cilindro de eixo Z com tampas: `lados` em volta e `andares` na altura.
    pub fn cilindro(c: V3, raio: f64, altura: f64, lados: u32, andares: u32) -> Gaiola {
        let (n, m) = (lados.max(3), andares.max(1));
        let mut g = Gaiola::default();
        let mut anel = vec![];
        for k in 0..=m {
            let z = c[2] + altura * (k as f64 / m as f64 - 0.5);
            anel.push((0..n).map(|i| {
                let a = std::f64::consts::TAU * i as f64 / n as f64;
                g.novo_no([c[0] + raio * a.cos(), c[1] + raio * a.sin(), z])
            }).collect::<Vec<_>>());
        }
        for k in 0..m as usize {
            for i in 0..n as usize {
                let j = (i + 1) % n as usize;
                g.nova_face(vec![anel[k][i], anel[k][j], anel[k + 1][j], anel[k + 1][i]]);
            }
        }
        g.nova_face(anel[m as usize].clone());
        g.nova_face(anel[0].iter().rev().copied().collect());
        g
    }

    pub fn primitiva(tipo: PrimitivaForma, c: V3, tam: V3) -> Gaiola {
        match tipo {
            PrimitivaForma::Caixa => Gaiola::caixa(c, tam, [2, 2, 2]),
            PrimitivaForma::Plano => Gaiola::plano(c, [tam[0], tam[1]], [4, 4]),
            PrimitivaForma::Cilindro => Gaiola::cilindro(c, tam[0].min(tam[1]) / 2., tam[2], 8, 2),
            PrimitivaForma::Esfera => Gaiola::esfera(c, tam[0].min(tam[1]) / 2., 2),
        }
    }

    /// Topologia da malha guardada, com o índice de cada número de nó.
    pub fn topologia(&self) -> Result<(Topo, HashMap<u32, usize>), String> {
        let idx = self.por_nos();
        let faces: Vec<Vec<u32>> = self.faces.iter().map(|f| f.nos.iter().map(|n| idx[n] as u32).collect()).collect();
        Ok((Topo::novo(self.nos.len(), &faces)?, idx))
    }

    /// Tira nós que nenhuma face usa e vincos de aresta que deixou de existir.
    pub fn limpar(&mut self) {
        let usados: HashSet<u32> = self.faces.iter().flat_map(|f| f.nos.iter().copied()).collect();
        self.nos.retain(|n| usados.contains(&n.id));
        let arestas: HashSet<(u32, u32)> = self.arestas().into_iter().collect();
        self.vincos.retain(|v| arestas.contains(&(v.a, v.b)));
    }

    /// Nós de valência diferente de 4 (ou 3 na borda), os que a conversão não representa exatamente.
    pub fn nos_irregulares(&self) -> Vec<u32> {
        let Ok((t, _)) = self.topologia() else { return vec![] };
        self.nos
            .iter()
            .enumerate()
            .filter(|(i, _)| {
                let (l, fechado) = t.leque(*i as u32);
                !l.is_empty() && if fechado { l.len() != 4 } else { l.len() > 2 }
            })
            .map(|(_, n)| n.id)
            .collect()
    }

    /// Divide a aresta `a`-`b` com um nó novo em `t`, em todas as faces que a usam.
    pub fn dividir_aresta(&mut self, a: u32, b: u32, t: f64) -> Result<u32, String> {
        let (pa, pb) = (self.pos(a).ok_or("nó inexistente")?, self.pos(b).ok_or("nó inexistente")?);
        if !self.arestas().contains(&par(a, b)) {
            return Err("esses nós não formam uma aresta".into());
        }
        let m = self.novo_no(lerp(pa, pb, t));
        for f in &mut self.faces {
            let n = f.nos.len();
            for k in 0..n {
                let (x, y) = (f.nos[k], f.nos[(k + 1) % n]);
                if (x, y) == (a, b) || (x, y) == (b, a) {
                    f.nos.insert(k + 1, m);
                    break;
                }
            }
        }
        let w = self.peso(a, b);
        if w > 0. {
            self.definir_vinco(a, b, 0.);
            self.definir_vinco(a, m, w);
            self.definir_vinco(m, b, w);
        }
        Ok(m)
    }

    /// Corta a face entre dois nós dela. A parte que começa em `a` guarda o número da face.
    pub fn dividir_face(&mut self, face: u32, a: u32, b: u32) -> Result<u32, String> {
        let f = self.face(face).ok_or("face inexistente")?.nos.clone();
        let (i, j) = (indice_em(&f, a).ok_or("o nó não é dessa face")?, indice_em(&f, b).ok_or("o nó não é dessa face")?);
        let (i, j) = (i.min(j), i.max(j));
        if j - i < 2 || (i == 0 && j == f.len() - 1) {
            return Err("os nós já são vizinhos na face".into());
        }
        let primeira: Vec<u32> = f[..=i].iter().chain(f[j..].iter()).copied().collect();
        let segunda: Vec<u32> = f[i..=j].to_vec();
        self.faces.iter_mut().find(|x| x.id == face).ok_or("face inexistente")?.nos = primeira;
        Ok(self.nova_face(segunda))
    }

    /// Inserir aresta: liga dois nós de uma mesma face.
    pub fn inserir_aresta(&mut self, a: u32, b: u32) -> Result<u32, String> {
        let face = self.faces.iter().find(|f| f.nos.contains(&a) && f.nos.contains(&b)).map(|f| f.id).ok_or("os nós não estão na mesma face")?;
        self.dividir_face(face, a, b)
    }

    /// As arestas que um corte em laço atravessa a partir de `a`-`b`, todas no mesmo sentido, e as
    /// faces entre duas consecutivas. Para no primeiro polígono que não é quadrilátero.
    pub fn laco_de(&self, a: u32, b: u32) -> Result<(Vec<(u32, u32)>, Vec<u32>), String> {
        let (t, idx) = self.topologia()?;
        let (ia, ib) = (*idx.get(&a).ok_or("nó inexistente")? as u32, *idx.get(&b).ok_or("nó inexistente")? as u32);
        let saindo = |de: u32, para: u32| t.leque(de).0.into_iter().find(|h| t.destino(*h) == para);
        let (ab, ba) = (saindo(ia, ib), saindo(ib, ia));
        if ab.is_none() && ba.is_none() {
            return Err("esses nós não formam uma aresta".into());
        }
        let id_no = |i: u32| self.nos[i as usize].id;
        // num quadrilátero a,b,c,d a aresta paralela a a→b, no mesmo sentido, é d→c: a gêmea da oposta
        let andar = |h0: u32| -> (Vec<(u32, u32)>, Vec<u32>, bool) {
            let (mut es, mut fs) = (vec![], vec![]);
            let mut h = h0;
            loop {
                let face = &self.faces[t.face[h as usize] as usize];
                if face.nos.len() != 4 {
                    return (es, fs, false);
                }
                fs.push(face.id);
                let oposta = t.prox[t.prox[h as usize] as usize];
                let g = t.gemea[oposta as usize];
                if g == h0 {
                    return (es, fs, true);
                }
                es.push((id_no(t.destino(oposta)), id_no(t.origem[oposta as usize])));
                if g == u32::MAX {
                    return (es, fs, false);
                }
                h = g;
            }
        };
        let (mut arestas, mut faces) = (vec![(a, b)], vec![]);
        if let Some(h) = ab {
            let (es, fs, fechou) = andar(h);
            arestas.extend(es);
            faces = fs;
            if fechou {
                return Ok((arestas, faces));
            }
        }
        if let Some(h) = ba {
            // do outro lado o sentido vem trocado
            let (es, mut fs, _) = andar(h);
            let mut antes: Vec<(u32, u32)> = es.into_iter().map(|(x, y)| (y, x)).collect();
            antes.reverse();
            antes.extend(arestas);
            arestas = antes;
            fs.reverse();
            fs.extend(faces);
            faces = fs;
        }
        Ok((arestas, faces))
    }

    /// Corte em laço: um nó novo em cada aresta atravessada, na fração `t` (o deslize), ligados dentro
    /// de cada quadrilátero. Devolve os nós novos.
    pub fn corte_em_laco(&mut self, a: u32, b: u32, t: f64) -> Result<Vec<u32>, String> {
        let (arestas, faces) = self.laco_de(a, b)?;
        let t = t.clamp(0.01, 0.99);
        let mut novos: HashMap<(u32, u32), u32> = HashMap::new();
        let mut lista = vec![];
        for &(x, y) in &arestas {
            let m = self.dividir_aresta(x, y, t)?;
            novos.insert(par(x, y), m);
            lista.push(m);
        }
        for f in faces {
            let nos = self.face(f).ok_or("face inexistente")?.nos.clone();
            let meus: Vec<u32> = lista.iter().copied().filter(|m| nos.contains(m)).collect();
            if let [p, q] = meus[..] {
                self.dividir_face(f, p, q)?;
            }
        }
        Ok(lista)
    }

    /// Extruda a região de faces: os nós dela se deslocam `dist` pela normal média de cada nó e a
    /// borda ganha quadriláteros novos. As faces da região guardam o número.
    pub fn extrudar(&mut self, faces: &[u32], dist: f64) -> Result<Vec<u32>, String> {
        let regiao: HashSet<u32> = faces.iter().copied().collect();
        if regiao.is_empty() {
            return Err("escolha uma face".into());
        }
        // arestas dirigidas da região sem par dentro dela são a borda
        let mut dirigidas: HashSet<(u32, u32)> = HashSet::new();
        for f in self.faces.iter().filter(|f| regiao.contains(&f.id)) {
            for k in 0..f.nos.len() {
                dirigidas.insert((f.nos[k], f.nos[(k + 1) % f.nos.len()]));
            }
        }
        let borda: Vec<(u32, u32)> = dirigidas.iter().filter(|(a, b)| !dirigidas.contains(&(*b, *a))).copied().collect();
        let mut soma: HashMap<u32, V3> = HashMap::new();
        for &f in &regiao {
            let n = self.normal_face(f).ok_or("face inexistente")?;
            for &v in &self.face(f).ok_or("face inexistente")?.nos.clone() {
                let e = soma.entry(v).or_insert([0.; 3]);
                *e = add(*e, n);
            }
        }
        let mut novo: HashMap<u32, u32> = HashMap::new();
        let mut ordem: Vec<u32> = soma.keys().copied().collect();
        ordem.sort_unstable();
        for v in ordem {
            let d = unit(soma[&v]).unwrap_or([0., 0., 1.]);
            let p = self.pos(v).ok_or("nó inexistente")?;
            let mut q = add(p, mul(d, dist));
            // a costura de simetria continua no plano
            if let Some(e) = self.no_preso_ao_plano(v) {
                q[e as usize] = p[e as usize];
            }
            let m = self.novo_no(q);
            novo.insert(v, m);
        }
        for f in self.faces.iter_mut().filter(|f| regiao.contains(&f.id)) {
            for v in &mut f.nos {
                *v = novo[v];
            }
        }
        let mut lados = vec![];
        let mut ordenada = borda;
        ordenada.sort_unstable();
        for (a, b) in ordenada {
            // parede sobre o plano de simetria ficaria dentro da peça, com a do reflexo
            if self.no_preso_ao_plano(a).is_some() && self.no_preso_ao_plano(b).is_some() {
                continue;
            }
            lados.push(self.nova_face(vec![a, b, novo[&b], novo[&a]]));
        }
        self.limpar();
        Ok(lados)
    }

    /// Suavizar borda: troca a aresta `a`-`b` por uma faixa de `segmentos` faces, recuando `raio` pelas
    /// arestas vizinhas de cada lado. A faixa segue um arco de Bézier com o nó no meio, então o raio é
    /// o recuo. Nó de valência 3 some, porque a face do outro lado recebe o arco; nos outros o nó fica
    /// e uma tampa fecha o fim da faixa.
    pub fn suavizar_borda(&mut self, a: u32, b: u32, raio: f64, segmentos: u32) -> Result<(), String> {
        let dirigida = |g: &Gaiola, x: u32, y: u32| -> Option<u32> {
            g.faces.iter().find(|f| (0..f.nos.len()).any(|k| f.nos[k] == x && f.nos[(k + 1) % f.nos.len()] == y)).map(|f| f.id)
        };
        // a aresta aparece nos dois sentidos, um em cada face; a ordem de a e b não importa
        let (a, b) = if dirigida(self, a, b).is_some() { (a, b) } else { (b, a) };
        let (esq, dir) = (dirigida(self, a, b).ok_or("a borda precisa de duas faces")?, dirigida(self, b, a).ok_or("a borda precisa de duas faces")?);
        let segs = segmentos.max(1) as usize;
        let vizinho = |g: &Gaiola, face: u32, v: u32, depois: bool| -> Result<u32, String> {
            let f = &g.face(face).ok_or("face inexistente")?.nos;
            let k = indice_em(f, v).ok_or("nó fora da face")?;
            Ok(if depois { f[(k + 1) % f.len()] } else { f[(k + f.len() - 1) % f.len()] })
        };
        let (ve, vd) = (vizinho(self, esq, a, false)?, vizinho(self, dir, a, true)?);
        let (we, wd) = (vizinho(self, esq, b, true)?, vizinho(self, dir, b, false)?);
        // as faces do outro lado de cada aresta recuada
        let lado = |g: &Gaiola, x: u32, y: u32| dirigida(g, x, y).ok_or_else(|| "a ponta da aresta está na borda da malha".to_string());
        let (fa_e, fa_d, fb_e, fb_d) = (lado(self, a, ve)?, lado(self, vd, a)?, lado(self, we, b)?, lado(self, b, wd)?);
        let arco = |g: &mut Gaiola, v: u32, e: u32, d: u32| -> Result<Vec<u32>, String> {
            let p = g.pos(v).ok_or("nó inexistente")?;
            let recuo = |q: V3| -> Result<V3, String> {
                let l = norma(sub(q, p));
                if l <= raio * 1.0001 {
                    return Err("o raio é maior que a aresta vizinha".into());
                }
                Ok(lerp(p, q, raio / l))
            };
            let (pe, pd) = (recuo(g.pos(e).ok_or("nó inexistente")?)?, recuo(g.pos(d).ok_or("nó inexistente")?)?);
            Ok((0..=segs)
                .map(|i| {
                    let u = i as f64 / segs as f64;
                    g.novo_no(add(add(mul(pe, (1. - u) * (1. - u)), mul(p, 2. * u * (1. - u))), mul(pd, u * u)))
                })
                .collect())
        };
        let aa = arco(self, a, ve, vd)?;
        let ab = arco(self, b, we, wd)?;
        let trocar = |g: &mut Gaiola, face: u32, v: u32, por: &[u32]| -> Result<(), String> {
            let f = g.faces.iter_mut().find(|f| f.id == face).ok_or("face inexistente")?;
            let k = indice_em(&f.nos, v).ok_or("nó fora da face")?;
            f.nos.splice(k..=k, por.iter().copied());
            Ok(())
        };
        trocar(self, esq, a, &aa[..1])?;
        trocar(self, esq, b, &ab[..1])?;
        trocar(self, dir, a, &aa[segs..])?;
        trocar(self, dir, b, &ab[segs..])?;
        // ponta `a`: a face do outro lado percorre o arco de trás para a frente
        if fa_e == fa_d {
            let volta: Vec<u32> = aa.iter().rev().copied().collect();
            trocar(self, fa_e, a, &volta)?;
        } else {
            trocar(self, fa_e, a, &[a, aa[0]])?;
            trocar(self, fa_d, a, &[aa[segs], a])?;
            self.nova_face(std::iter::once(a).chain(aa.iter().rev().copied()).collect());
        }
        // ponta `b`: no sentido do arco
        if fb_e == fb_d {
            trocar(self, fb_e, b, &ab)?;
        } else {
            trocar(self, fb_e, b, &[ab[0], b])?;
            trocar(self, fb_d, b, &[b, ab[segs]])?;
            self.nova_face(std::iter::once(b).chain(ab.iter().copied()).collect());
        }
        for i in 0..segs {
            self.nova_face(vec![ab[i], aa[i], aa[i + 1], ab[i + 1]]);
        }
        self.limpar();
        self.topologia().map(|_| ())
    }

    /// Prepara a simetria: corta a gaiola no plano, tira o lado negativo e liga a simetria.
    pub fn ligar_simetria(&mut self, s: Simetria) -> Result<(), String> {
        if self.simetria.is_some() {
            return Err("a simetria já está ligada".into());
        }
        // nó em cada aresta que atravessa o plano
        for (a, b) in self.arestas() {
            let (pa, pb) = (self.pos(a).ok_or("nó inexistente")?, self.pos(b).ok_or("nó inexistente")?);
            let (da, db) = (s.distancia(pa), s.distancia(pb));
            if da.abs() > TOL_PLANO && db.abs() > TOL_PLANO && da * db < 0. {
                self.dividir_aresta(a, b, da / (da - db))?;
            }
        }
        for n in &mut self.nos {
            if s.distancia(n.p).abs() <= TOL_PLANO {
                n.p[s.eixo as usize] = s.pos;
            }
        }
        // cada face cruzada fica partida entre os seus nós do plano
        let ids: Vec<u32> = self.faces.iter().map(|f| f.id).collect();
        for id in ids {
            let nos = self.face(id).ok_or("face inexistente")?.nos.clone();
            let d: Vec<f64> = nos.iter().filter_map(|n| self.pos(*n)).map(|p| s.distancia(p)).collect();
            let cruza = d.iter().any(|x| *x > TOL_PLANO) && d.iter().any(|x| *x < -TOL_PLANO);
            if !cruza {
                continue;
            }
            let no_plano: Vec<u32> = nos.iter().zip(&d).filter(|(_, x)| x.abs() <= TOL_PLANO).map(|(n, _)| *n).collect();
            if let [p, q] = no_plano[..] {
                self.dividir_face(id, p, q)?;
            } else {
                return Err("uma face atravessa o plano mais de uma vez".into());
            }
        }
        let dist = |g: &Gaiola, f: &FaceG| f.nos.iter().filter_map(|n| g.pos(*n)).map(|p| s.distancia(p)).fold(f64::MIN, f64::max);
        let fica: Vec<bool> = self.faces.iter().map(|f| dist(self, f) > TOL_PLANO).collect();
        let mut it = fica.iter();
        self.faces.retain(|_| *it.next().unwrap_or(&false));
        if self.faces.is_empty() {
            return Err("não sobrou nada do lado positivo do plano".into());
        }
        self.limpar();
        self.simetria = Some(s);
        Ok(())
    }

    /// Desliga a simetria guardando as duas metades como gaiola comum.
    pub fn desligar_simetria(&mut self) {
        if self.simetria.is_none() {
            return;
        }
        let inteira = self.inteira();
        // a metade espelhada ganha números comuns
        let (mut mapa, mut proximo) = (HashMap::new(), self.prox_no);
        for n in &inteira.nos {
            if n.id & ESPELHO != 0 {
                mapa.insert(n.id, proximo);
                proximo += 1;
            }
        }
        let mut proxima_face = self.prox_face;
        let no = |v: u32| mapa.get(&v).copied().unwrap_or(v);
        self.nos = inteira.nos.iter().map(|n| NoG { id: no(n.id), ..n.clone() }).collect();
        self.faces = inteira
            .faces
            .iter()
            .map(|f| {
                let id = if f.id & ESPELHO != 0 {
                    proxima_face += 1;
                    proxima_face - 1
                } else {
                    f.id
                };
                FaceG { id, nos: f.nos.iter().map(|v| no(*v)).collect() }
            })
            .collect();
        self.vincos = inteira.vincos.iter().map(|v| {
            let (a, b) = par(no(v.a), no(v.b));
            Vinco { a, b, peso: v.peso }
        }).collect();
        self.prox_no = proximo;
        self.prox_face = proxima_face;
        self.simetria = None;
    }

    /// Prende ao plano os nós que estão nele: nó da costura de simetria não sai do plano.
    pub fn no_preso_ao_plano(&self, id: u32) -> Option<u8> {
        let s = self.simetria?;
        (s.distancia(self.pos(id)?).abs() < TOL_PLANO).then_some(s.eixo)
    }

    /// Chip do nó: Suave tira canto e vincos; Vinco marca as duas arestas mais opostas; Canto deixa a
    /// superfície passar no nó.
    pub fn definir_tipo_no(&mut self, id: u32, tipo: TipoNo, peso: f64) -> Result<(), String> {
        let (t, idx) = self.topologia()?;
        let i = *idx.get(&id).ok_or("nó inexistente")? as u32;
        let viz: Vec<u32> = t.vizinhos(i).iter().map(|v| self.nos[*v as usize].id).collect();
        let p = self.pos(id).ok_or("nó inexistente")?;
        match tipo {
            TipoNo::Suave => {
                for &v in &viz {
                    self.definir_vinco(id, v, 0.);
                }
                self.no_mut(id).ok_or("nó inexistente")?.canto = false;
            }
            TipoNo::Canto => {
                let n = self.no_mut(id).ok_or("nó inexistente")?;
                n.canto = true;
                n.tangente = None;
            }
            TipoNo::Vinco => {
                let dir: Vec<V3> = viz.iter().filter_map(|v| self.pos(*v)).map(|q| unit(sub(q, p)).unwrap_or([0.; 3])).collect();
                let mut melhor = (f64::MAX, 0, 0);
                for i in 0..dir.len() {
                    for j in i + 1..dir.len() {
                        let c = dot(dir[i], dir[j]);
                        if c < melhor.0 {
                            melhor = (c, i, j);
                        }
                    }
                }
                self.definir_vinco(id, viz[melhor.1], peso);
                self.definir_vinco(id, viz[melhor.2], peso);
                self.no_mut(id).ok_or("nó inexistente")?.canto = false;
            }
        }
        Ok(())
    }

    /// Tipo mostrado no chip do nó.
    pub fn tipo_no(&self, id: u32) -> TipoNo {
        if self.no(id).is_some_and(|n| n.canto) {
            return TipoNo::Canto;
        }
        if self.vincos.iter().any(|v| v.a == id || v.b == id) {
            TipoNo::Vinco
        } else {
            TipoNo::Suave
        }
    }

    /// Vetores tangentes do limite num nó interno liso, como pesos sobre os nós do anel (Halstead):
    /// devolve, para cada nó vizinho e diagonal, os dois pesos.
    fn pesos_tangentes(&self, t: &Topo, i: u32) -> Result<Vec<(u32, f64, f64)>, String> {
        let (l, fechado) = t.leque(i);
        if !fechado {
            return Err("tangência só num nó interno".into());
        }
        let n = l.len();
        if l.iter().any(|h| self.faces[t.face[*h as usize] as usize].nos.len() != 4) {
            return Err("tangência só entre quadriláteros".into());
        }
        let tau = std::f64::consts::TAU;
        let an = 1. + (tau / n as f64).cos() + (tau / (2. * n as f64)).cos() * (2. * (9. + (tau / n as f64).cos())).sqrt();
        let mut out = vec![];
        for (k, &h) in l.iter().enumerate() {
            // a face de `h` fica entre esta aresta e a anterior do leque
            let (a0, a1) = (tau * k as f64 / n as f64, tau * (k as f64 - 1.) / n as f64);
            out.push((t.destino(h), an * a0.cos(), an * a0.sin()));
            let diag = t.origem[t.prox[t.prox[h as usize] as usize] as usize];
            out.push((diag, a0.cos() + a1.cos(), a0.sin() + a1.sin()));
        }
        Ok(out)
    }

    /// Colocar tangência: o anel do nó é ajustado por mínimos quadrados para o plano tangente do limite
    /// ter a normal pedida. O deslocamento é o menor possível, ao longo da normal.
    pub fn colocar_tangencia(&mut self, id: u32, normal: V3) -> Result<(), String> {
        let n = unit(normal).ok_or("normal nula")?;
        let (t, idx) = self.topologia()?;
        let i = *idx.get(&id).ok_or("nó inexistente")? as u32;
        if self.no(id).is_some_and(|x| x.canto) {
            return Err("um canto não tem plano tangente".into());
        }
        let pesos = self.pesos_tangentes(&t, i)?;
        // cada nó do anel pode repetir nos pesos: junta por nó
        let mut por_no: HashMap<u32, (f64, f64)> = HashMap::new();
        for (v, a, b) in pesos {
            let e = por_no.entry(v).or_insert((0., 0.));
            e.0 += a;
            e.1 += b;
        }
        let mut ordem: Vec<(u32, f64, f64)> = por_no.into_iter().map(|(v, (a, b))| (v, a, b)).collect();
        ordem.sort_unstable_by_key(|x| x.0);
        let altura = |v: u32| dot(self.nos[v as usize].p, n);
        // restrições: soma(a_i (h_i + d_i)) = 0 e soma(b_i (h_i + d_i)) = 0
        let (mut aa, mut ab, mut bb, mut ra, mut rb) = (0., 0., 0., 0., 0.);
        for &(v, a, b) in &ordem {
            let h = altura(v);
            aa += a * a;
            ab += a * b;
            bb += b * b;
            ra += a * h;
            rb += b * h;
        }
        let det = aa * bb - ab * ab;
        if det.abs() < 1e-12 {
            return Err("o anel do nó não define um plano".into());
        }
        let (la, lb) = ((ra * bb - rb * ab) / det, (rb * aa - ra * ab) / det);
        for &(v, a, b) in &ordem {
            let d = -(a * la + b * lb);
            let p = &mut self.nos[v as usize].p;
            *p = add(*p, mul(n, d));
        }
        let no = self.no_mut(id).ok_or("nó inexistente")?;
        let tensao = no.tangente.map_or(1., |x| x.tensao);
        no.tangente = Some(Tangente { normal: n, tensao });
        Ok(())
    }

    /// Tensão: abre ou fecha o anel em torno do nó dentro do plano tangente, e refaz o ajuste.
    pub fn definir_tensao(&mut self, id: u32, tensao: f64) -> Result<(), String> {
        let tg = self.no(id).and_then(|n| n.tangente).ok_or("o nó não tem tangência")?;
        let tensao = tensao.clamp(0.1, 10.);
        let (t, idx) = self.topologia()?;
        let i = *idx.get(&id).ok_or("nó inexistente")? as u32;
        let c = self.nos[i as usize].p;
        let k = tensao / tg.tensao;
        for v in t.vizinhos(i) {
            let p = self.nos[v as usize].p;
            let d = sub(p, c);
            let alto = mul(tg.normal, dot(d, tg.normal));
            let plano = sub(d, alto);
            self.nos[v as usize].p = add(add(c, alto), mul(plano, k));
        }
        self.no_mut(id).ok_or("nó inexistente")?.tangente = Some(Tangente { normal: tg.normal, tensao });
        self.colocar_tangencia(id, tg.normal)
    }

    /// Normal do limite no nó, pelos vetores tangentes.
    pub fn normal_limite(&self, id: u32) -> Result<V3, String> {
        let (t, idx) = self.topologia()?;
        let i = *idx.get(&id).ok_or("nó inexistente")? as u32;
        let (mut t1, mut t2) = ([0.; 3], [0.; 3]);
        for (v, a, b) in self.pesos_tangentes(&t, i)? {
            let p = self.nos[v as usize].p;
            t1 = add(t1, mul(p, a));
            t2 = add(t2, mul(p, b));
        }
        // o leque gira no sentido horário visto de fora
        unit(cross(t2, t1)).ok_or("anel degenerado".into())
    }

    /// Depois de mover nós: refaz o ajuste de quem tem tangência.
    pub fn refazer_tangencias(&mut self) {
        let lista: Vec<(u32, V3)> = self.nos.iter().filter_map(|n| n.tangente.map(|t| (n.id, t.normal))).collect();
        for (id, n) in lista {
            self.colocar_tangencia(id, n).ok();
        }
    }
}

/// Estado do nó no chip do 3D.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TipoNo {
    Suave,
    Vinco,
    Canto,
}

#[cfg(test)]
mod testes {
    use super::*;

    fn euler(g: &Gaiola) -> i64 {
        g.nos.len() as i64 - g.arestas().len() as i64 + g.faces.len() as i64
    }

    fn fecha(g: &Gaiola) {
        let (t, _) = g.topologia().expect("topologia");
        assert!(t.gemea.iter().all(|x| *x != u32::MAX), "malha aberta");
        assert_eq!(euler(g), 2, "esfera topológica");
    }

    #[test]
    fn caixa_e_esfera_sao_fechadas_e_com_a_normal_para_fora() {
        let c = Gaiola::caixa([0.; 3], [10., 20., 30.], [2, 3, 4]);
        fecha(&c);
        let m = c.lisa(1).unwrap();
        assert!(m.volume() > 0., "normal para dentro");
        fecha(&Gaiola::esfera([0.; 3], 10., 3));
        fecha(&Gaiola::cilindro([0.; 3], 5., 10., 6, 2));
        assert_eq!(Gaiola::caixa([0.; 3], [1.; 3], [2, 2, 2]).nos_irregulares().len(), 8);
    }

    #[test]
    fn vinco_cheio_em_tudo_devolve_a_caixa_exata() {
        let mut g = Gaiola::caixa([0.; 3], [10., 20., 30.], [1, 1, 1]);
        for (a, b) in g.arestas() {
            g.definir_vinco(a, b, 1.);
        }
        let m = g.lisa(3).unwrap();
        assert!((m.volume() - 6000.).abs() < 1e-6, "volume {}", m.volume());
        let (a, b) = m.caixa().unwrap();
        assert!((a[0] + 5.).abs() < 1e-9 && (b[2] - 15.).abs() < 1e-9);
    }

    #[test]
    fn caixa_lisa_encolhe_para_dentro_da_gaiola() {
        let g = Gaiola::caixa([0.; 3], [10.; 3], [1, 1, 1]);
        let m = g.lisa(4).unwrap();
        let v = m.volume();
        assert!(v > 300. && v < 350., "volume {v}");
        let (a, b) = m.caixa().unwrap();
        assert!(a[0] > -5. && b[0] < 5.);
        // canto de valência 3: (9 v + 4 Σe + Σf) / 24 leva (5, 5, 5) a (2,5; 2,5; 2,5)
        let lim = g.nivel0().limite().unwrap();
        assert!(lim.iter().all(|p| p.iter().all(|x| (x.abs() - 2.5).abs() < 1e-9)), "{lim:?}");
        // limite simétrico
        assert!((a[0] + b[0]).abs() < 1e-9);
    }

    #[test]
    fn peso_do_vinco_fica_entre_o_liso_e_o_vivo() {
        let vol = |peso: f64| {
            let mut g = Gaiola::caixa([0.; 3], [10.; 3], [1, 1, 1]);
            for (a, b) in g.arestas() {
                g.definir_vinco(a, b, peso);
            }
            g.lisa(4).unwrap().volume()
        };
        let (v0, v5, v1) = (vol(0.), vol(0.5), vol(1.));
        assert!(v0 < v5 && v5 < v1, "{v0} {v5} {v1}");
    }

    #[test]
    fn grade_regular_e_b_spline_uniforme() {
        // anel fechado em duas direções (toro): todos os nós regulares
        let (n, m) = (8u32, 6u32);
        let mut g = Gaiola::default();
        let mut ids = vec![];
        for j in 0..m {
            for i in 0..n {
                let (a, b) = (std::f64::consts::TAU * i as f64 / n as f64, std::f64::consts::TAU * j as f64 / m as f64);
                let r = 20. + 5. * b.cos();
                ids.push(g.novo_no([r * a.cos(), r * a.sin(), 5. * b.sin()]));
            }
        }
        let at = |i: u32, j: u32| ids[((j % m) * n + i % n) as usize];
        for j in 0..m {
            for i in 0..n {
                g.nova_face(vec![at(i, j), at(i + 1, j), at(i + 1, j + 1), at(i, j + 1)]);
            }
        }
        assert!(g.nos_irregulares().is_empty());
        // limite do nó regular: máscara 1/36 (16 4 1)
        let lim = g.nivel0().limite().unwrap();
        let (t, _) = g.topologia().unwrap();
        let p = |i: u32, j: u32| g.pos(at(i, j)).unwrap();
        let mut esperado = [0.; 3];
        for (di, wi) in [(n - 1, 1.), (0, 4.), (1, 1.)] {
            for (dj, wj) in [(m - 1, 1.), (0, 4.), (1, 1.)] {
                esperado = add(esperado, mul(p(2 + di, 2 + dj), wi * wj / 36.));
            }
        }
        let i0 = g.por_nos()[&at(2, 2)];
        assert!(norma(sub(lim[i0], esperado)) < 1e-9);
        // e o refinamento converge para esse ponto
        let mut nivel = g.nivel0();
        for _ in 0..6 {
            nivel = nivel.refinar().unwrap();
        }
        assert!(norma(sub(nivel.pos[i0], esperado)) < 1e-2);
        assert!(!t.prox.is_empty());
    }

    #[test]
    fn extrudar_guarda_o_numero_da_face_e_cria_as_paredes() {
        let mut g = Gaiola::caixa([0.; 3], [10.; 3], [1, 1, 1]);
        let topo = g.faces.iter().find(|f| g.normal_face(f.id).unwrap()[2] > 0.9).unwrap().id;
        let lados = g.extrudar(&[topo], 5.).unwrap();
        assert_eq!(lados.len(), 4);
        fecha(&g);
        assert!((g.centro_face(topo).unwrap()[2] - 10.).abs() < 1e-9);
    }

    #[test]
    fn extrudar_regiao_nao_deixa_parede_por_dentro() {
        let mut g = Gaiola::caixa([0.; 3], [10.; 3], [2, 2, 2]);
        let topo: Vec<u32> = g.faces.iter().filter(|f| g.normal_face(f.id).unwrap()[2] > 0.9).map(|f| f.id).collect();
        assert_eq!(topo.len(), 4);
        let lados = g.extrudar(&topo, 5.).unwrap();
        assert_eq!(lados.len(), 8);
        fecha(&g);
    }

    #[test]
    fn corte_em_laco_da_a_volta_e_divide_as_faces() {
        let mut g = Gaiola::caixa([0.; 3], [10.; 3], [1, 1, 1]);
        let f0 = g.faces[0].clone();
        let (a, b) = (f0.nos[0], f0.nos[1]);
        let antes = g.faces.len();
        let novos = g.corte_em_laco(a, b, 0.25).unwrap();
        assert_eq!(novos.len(), 4, "volta na caixa");
        assert_eq!(g.faces.len(), antes + 4);
        fecha(&g);
        // todos na mesma fração da aresta original
        let pa = g.pos(a).unwrap();
        let pb = g.pos(b).unwrap();
        let m = g.pos(novos[0]).unwrap();
        assert!((norma(sub(m, pa)) / norma(sub(pb, pa)) - 0.25).abs() < 1e-9);
        let eixo = unit(sub(pb, pa)).unwrap();
        let nivel: Vec<f64> = novos.iter().map(|n| dot(g.pos(*n).unwrap(), eixo)).collect();
        assert!(nivel.iter().all(|x| (x - nivel[0]).abs() < 1e-9), "o laço ficou torto: {nivel:?}");
    }

    #[test]
    fn corte_em_laco_em_plano_para_nas_bordas() {
        let mut g = Gaiola::plano([0.; 3], [10., 10.], [3, 3]);
        let f = g.faces[4].clone();
        let n = g.corte_em_laco(f.nos[0], f.nos[1], 0.5).unwrap();
        assert_eq!(n.len(), 4);
        assert_eq!(g.faces.len(), 12);
        g.topologia().unwrap();
    }

    #[test]
    fn inserir_aresta_divide_a_face_e_vinco_acompanha_a_divisao() {
        let mut g = Gaiola::caixa([0.; 3], [10.; 3], [1, 1, 1]);
        let f = g.faces[0].clone();
        g.definir_vinco(f.nos[0], f.nos[1], 0.7);
        let m = g.dividir_aresta(f.nos[0], f.nos[1], 0.5).unwrap();
        assert_eq!(g.peso(f.nos[0], m), 0.7);
        assert_eq!(g.peso(m, f.nos[1]), 0.7);
        assert_eq!(g.peso(f.nos[0], f.nos[1]), 0.);
        g.inserir_aresta(m, f.nos[3]).unwrap();
        fecha(&g);
    }

    #[test]
    fn suavizar_borda_gera_faixa_valida() {
        for segs in [1, 2, 3] {
            let mut g = Gaiola::caixa([0.; 3], [10.; 3], [1, 1, 1]);
            let f = g.faces[0].clone();
            let antes = g.faces.len();
            g.suavizar_borda(f.nos[0], f.nos[1], 2., segs).unwrap();
            assert_eq!(g.faces.len(), antes + segs as usize);
            fecha(&g);
            assert!(g.lisa(2).unwrap().volume() > 0.);
        }
    }

    #[test]
    fn suavizar_borda_em_no_de_valencia_quatro_fecha_com_tampa() {
        let mut g = Gaiola::caixa([0.; 3], [10.; 3], [2, 2, 2]);
        // aresta entre dois nós de valência 4: do meio de uma face ao meio de uma aresta da caixa
        let centro = g.nos.iter().find(|n| n.p == [0., 0., 5.]).unwrap().id;
        let meio = g.nos.iter().find(|n| n.p == [5., 0., 5.]).unwrap().id;
        let antes = g.faces.len();
        g.suavizar_borda(centro, meio, 1., 2).unwrap();
        fecha(&g);
        assert_eq!(g.faces.len(), antes + 2 + 2, "faixa e duas tampas");
        assert!(g.lisa(2).unwrap().volume() > 0.);
    }

    #[test]
    fn suavizar_borda_com_raio_maior_que_a_aresta_recusa() {
        let mut g = Gaiola::caixa([0.; 3], [10.; 3], [1, 1, 1]);
        let f = g.faces[0].clone();
        assert!(g.suavizar_borda(f.nos[0], f.nos[1], 20., 1).is_err());
    }

    #[test]
    fn simetria_corta_no_plano_e_a_malha_inteira_fecha() {
        let mut g = Gaiola::caixa([0.; 3], [10., 10., 10.], [1, 1, 1]);
        let vol = g.lisa(3).unwrap();
        g.ligar_simetria(Simetria { eixo: 0, pos: 0. }).unwrap();
        assert!(g.nos.iter().all(|n| n.p[0] >= -1e-9));
        g.topologia().unwrap();
        fecha(&g.inteira());
        let m = g.lisa(3).unwrap();
        let (a, b) = m.caixa().unwrap();
        assert!((a[0] + b[0]).abs() < 1e-9, "simétrica");
        // a caixa de uma divisão ganha um laço no meio: mais cheia que a original
        assert!(m.volume() > vol.volume());
        // desligar conserva a forma
        let antes = m.volume();
        g.desligar_simetria();
        fecha(&g);
        assert!((g.lisa(3).unwrap().volume() - antes).abs() < 1e-6);
        let mut ids: Vec<u32> = g.nos.iter().map(|n| n.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), g.nos.len(), "números repetidos");
    }

    #[test]
    fn simetria_mantem_o_movimento_no_reflexo() {
        let mut g = Gaiola::caixa([0.; 3], [10.; 3], [2, 2, 2]);
        g.ligar_simetria(Simetria { eixo: 0, pos: 0. }).unwrap();
        let id = g.nos.iter().find(|n| n.p[0] > 1. && n.p[2] > 1.).unwrap().id;
        g.no_mut(id).unwrap().p[2] += 3.;
        let inteira = g.inteira();
        let q = inteira.no(id | ESPELHO).unwrap().p;
        assert!((q[2] - g.pos(id).unwrap()[2]).abs() < 1e-12 && (q[0] + g.pos(id).unwrap()[0]).abs() < 1e-12);
    }

    #[test]
    fn tipo_do_no_canto_prende_a_superficie() {
        let mut g = Gaiola::caixa([0.; 3], [10.; 3], [2, 2, 2]);
        let id = g.nos.iter().find(|n| n.p == [5., 5., 5.]).unwrap().id;
        g.definir_tipo_no(id, TipoNo::Canto, 1.).unwrap();
        assert_eq!(g.tipo_no(id), TipoNo::Canto);
        let m = g.lisa(3).unwrap();
        assert!(m.pos.iter().any(|p| norma(sub(*p, [5., 5., 5.])) < 1e-9), "a superfície passa no canto");
        g.definir_tipo_no(id, TipoNo::Suave, 0.).unwrap();
        assert_eq!(g.tipo_no(id), TipoNo::Suave);
    }

    #[test]
    fn no_em_vinco_marca_as_duas_arestas_opostas() {
        let mut g = Gaiola::caixa([0.; 3], [10.; 3], [2, 2, 2]);
        let id = g.nos.iter().find(|n| n.p == [0., 0., 5.]).unwrap().id;
        g.definir_tipo_no(id, TipoNo::Vinco, 1.).unwrap();
        assert_eq!(g.vincos.len(), 2);
        assert_eq!(g.tipo_no(id), TipoNo::Vinco);
        let [v, w] = g.vincos[..] else { panic!() };
        let outra = |x: Vinco| if x.a == id { x.b } else { x.a };
        let (p, q) = (g.pos(outra(v)).unwrap(), g.pos(outra(w)).unwrap());
        assert!(norma(add(p, q)) < 10.1, "arestas opostas");
    }

    #[test]
    fn tangencia_faz_a_normal_do_limite_seguir_o_pedido() {
        for div in [2u32, 3] {
            let mut g = Gaiola::esfera([0.; 3], 20., div);
            // nó de valência 4 ou 3
            let ids: Vec<u32> = g.nos.iter().map(|n| n.id).collect();
            for id in [ids[0], ids[ids.len() / 2]] {
                let n0 = g.normal_limite(id).unwrap();
                let pedido = unit(add(n0, [0.3, -0.2, 0.1])).unwrap();
                g.colocar_tangencia(id, pedido).unwrap();
                let n1 = g.normal_limite(id).unwrap();
                assert!(dot(n1, pedido) > 1. - 1e-9, "normal {n1:?} pedido {pedido:?}");
                // e a superfície de verdade concorda: normal da malha fina junto do nó
                let m = g.lisa(5).unwrap();
                let alvo = g.nivel0().limite().unwrap()[g.por_nos()[&id]];
                let perto = (0..m.pos.len()).min_by(|a, b| norma(sub(m.pos[*a], alvo)).total_cmp(&norma(sub(m.pos[*b], alvo)))).unwrap();
                assert!(dot(m.nrm[perto], pedido) > 0.999, "malha {:?} pedido {pedido:?}", m.nrm[perto]);
            }
        }
    }

    #[test]
    fn tensao_abre_o_anel_sem_mudar_o_plano() {
        let mut g = Gaiola::caixa([0.; 3], [20.; 3], [4, 4, 4]);
        let id = g.nos.iter().find(|n| n.p == [0., 0., 10.]).unwrap().id;
        g.colocar_tangencia(id, [0., 0., 1.]).unwrap();
        let (t, idx) = g.topologia().unwrap();
        let viz = t.vizinhos(idx[&id] as u32);
        let raio = |g: &Gaiola| norma(sub(g.nos[viz[0] as usize].p, g.pos(id).unwrap()));
        let r0 = raio(&g);
        g.definir_tensao(id, 2.).unwrap();
        assert!((raio(&g) / r0 - 2.).abs() < 1e-6);
        let n = g.normal_limite(id).unwrap();
        assert!(dot(n, [0., 0., 1.]) > 1. - 1e-9);
        g.definir_tensao(id, 1.).unwrap();
        assert!((raio(&g) - r0).abs() < 1e-6, "volta sem acumular");
    }

    #[test]
    fn numeros_nao_se_repetem_depois_de_apagar() {
        let mut g = Gaiola::caixa([0.; 3], [10.; 3], [1, 1, 1]);
        let maior = g.prox_face;
        let f = g.faces[0].id;
        g.extrudar(&[f], 2.).unwrap();
        assert!(g.faces.iter().filter(|x| x.id != f).any(|x| x.id >= maior));
        assert!(g.prox_face > maior);
    }

    #[test]
    fn malha_nao_variedade_vira_erro_e_nao_panico() {
        let mut g = Gaiola::caixa([0.; 3], [10.; 3], [1, 1, 1]);
        let f = g.faces[0].clone();
        g.nova_face(f.nos.clone());
        assert!(g.lisa(1).is_err());
        assert!(g.topologia().is_err());
    }
}
