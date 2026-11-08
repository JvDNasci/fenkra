// SPDX-License-Identifier: LGPL-2.1-or-later
//! B174: acessibilidade da tocha de solda MIG/TIG. Para cada ponto do cordão, a tocha é girada em
//! torno da posição de referência (bissetriz das duas pernas do filete) pelo ângulo de trabalho (em
//! torno do eixo do cordão) e pelo de deslocamento (inclinação ao longo do cordão). Cada direção é
//! testada contra a malha da peça inteira com a BVH do `fenkra-malha`: o bico é um cilindro do raio
//! do bocal, do afastamento (stick-out) até o comprimento da tocha, representado por um raio no eixo
//! e raios na casca do cilindro. Direção livre = nenhum raio bate antes do fim da tocha.
//!
//! O resultado de cada ponto: as direções livres e as bloqueadas (o cone de direções viáveis), a melhor
//! livre (a mais perto da referência), e quem bloqueia (o comando dono da face atingida). Ponto sem
//! nenhuma direção livre é inviável; a sugestão diz para soldar antes de pôr a peça que bloqueia, ou
//! para mudar o ângulo quando a referência bate mas outra direção passa.
//!
//! Simplificações, ditas no painel: bocal cilíndrico (sem o cone do bico nem o cabo), malha da tela
//! (a tesselação de trabalho), e o cordão já depositado não conta como obstáculo.

use super::*;
use fenkra_malha::bvh::Bvh;
use fenkra_malha::cena::Malha;
use st_solda::CordaoCalculado;

/// Medidas da tocha e da amostragem.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OpcoesTocha {
    /// raio externo do bocal (mm)
    pub raio_bocal: f64,
    /// distância do ponto de solda à ponta do bocal (stick-out + recuo do bico)
    pub afastamento: f64,
    /// comprimento livre exigido ao longo do eixo da tocha, a partir do ponto
    pub comprimento: f64,
    /// faixa de ângulo de trabalho e de deslocamento em torno da referência (± graus)
    pub faixa_trabalho: f64,
    pub faixa_deslocamento: f64,
    /// passo dos ângulos (graus)
    pub passo_angulo: f64,
    /// distância entre pontos ao longo do cordão (mm)
    pub passo: f64,
}

impl Default for OpcoesTocha {
    fn default() -> Self {
        OpcoesTocha { raio_bocal: 8., afastamento: 15., comprimento: 250., faixa_trabalho: 30., faixa_deslocamento: 30., passo_angulo: 15., passo: 20. }
    }
}

/// Uma direção testada: os dois ângulos (graus) e a direção (do ponto para a tocha).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DirecaoTocha {
    pub trabalho: f64,
    pub deslocamento: f64,
    pub d: V3,
    pub livre: bool,
}

/// Resultado de um ponto do cordão.
#[derive(Clone, Debug, PartialEq)]
pub struct PontoTocha {
    pub p: V3,
    /// direção de referência (bissetriz das pernas)
    pub referencia: V3,
    pub direcoes: Vec<DirecaoTocha>,
    /// índice em `direcoes` da melhor direção livre (a de menor desvio da referência)
    pub melhor: Option<usize>,
    /// comandos donos das faces que bloquearam alguma direção
    pub bloqueios: Vec<Id>,
}

impl PontoTocha {
    pub fn viavel(&self) -> bool {
        self.melhor.is_some()
    }
    /// Fração das direções testadas que estão livres.
    pub fn fracao_livre(&self) -> f64 {
        if self.direcoes.is_empty() {
            return 0.;
        }
        self.direcoes.iter().filter(|d| d.livre).count() as f64 / self.direcoes.len() as f64
    }
}

/// Malha da peça para a busca: os triângulos e o comando dono de cada um.
pub struct Obstaculos {
    bvh: Bvh,
    dono: Vec<Option<Id>>,
}

impl Obstaculos {
    /// Junta os corpos do recálculo numa malha só.
    pub fn da_peca(r: &RecomputeResult) -> Obstaculos {
        let mut m = Malha::default();
        let mut dono = vec![];
        for b in &r.bodies {
            let base = m.pos.len() as u32;
            m.pos.extend(b.mesh.positions.chunks(3).map(|p| [p[0] as f64, p[1] as f64, p[2] as f64]));
            let tris = b.mesh.indices.len() / 3;
            for t in 0..tris {
                let i = &b.mesh.indices[t * 3..t * 3 + 3];
                m.tris.push([base + i[0], base + i[1], base + i[2]]);
                // face do triângulo: a última faixa que começa antes dele
                let k = (t * 3) as u32;
                let face = b.mesh.face_ranges.partition_point(|s| *s <= k).saturating_sub(1);
                dono.push(b.face_owner.get(face).copied());
            }
        }
        Obstaculos { bvh: Bvh::nova(&m), dono }
    }

    /// Primeiro triângulo atingido pelo raio antes de `max` (distância e dono).
    fn bate(&self, o: V3, d: V3, max: f64) -> Option<(f64, Option<Id>)> {
        let (t, tri) = self.bvh.raio(o, d)?;
        (t < max).then(|| (t, self.dono.get(tri as usize).copied().flatten()))
    }
}

/// Testa uma direção: raio no eixo e oito na casca do bocal, do afastamento até o comprimento.
fn testar(obs: &Obstaculos, p: V3, d: V3, o: &OpcoesTocha) -> (bool, Option<Id>) {
    let u = geo3d::any_perp(d);
    let v = fenkra_kernel::cross(d, u);
    let inicio = geo3d::add(p, geo3d::mul(d, o.afastamento));
    let max = (o.comprimento - o.afastamento).max(0.);
    let mut origens = vec![inicio];
    for k in 0..8 {
        let a = k as f64 * std::f64::consts::PI / 4.;
        origens.push(geo3d::add(inicio, geo3d::add(geo3d::mul(u, o.raio_bocal * a.cos()), geo3d::mul(v, o.raio_bocal * a.sin()))));
    }
    for q in origens {
        if let Some((_, dono)) = obs.bate(q, d, max) {
            return (false, dono);
        }
    }
    (true, None)
}

/// Pontos ao longo de um caminho a cada `passo`, com o índice do segmento de cada um.
fn amostrar(pts: &[V3], passo: f64) -> Vec<(V3, usize)> {
    let mut out = vec![];
    for (i, w) in pts.windows(2).enumerate() {
        let l = geo3d::dist(w[0], w[1]);
        let n = (l / passo.max(1.)).ceil().max(1.) as usize;
        for k in 0..n {
            // pontos no meio de cada pedaço: as pontas do cordão encostam nas duas faces
            let t = (k as f64 + 0.5) / n as f64;
            out.push((geo3d::lerp(w[0], w[1], t), i));
        }
    }
    out
}

/// Acessibilidade de todos os caminhos de um cordão.
pub fn acessibilidade(cordao: &CordaoCalculado, obs: &Obstaculos, o: &OpcoesTocha) -> Vec<PontoTocha> {
    let mut out = vec![];
    let passo = o.passo_angulo.max(1.);
    let faixa = |f: f64| -> Vec<f64> {
        let n = (f / passo).floor() as i32;
        (-n..=n).map(|k| k as f64 * passo).collect()
    };
    let (trab, desl) = (faixa(o.faixa_trabalho), faixa(o.faixa_deslocamento));
    for c in &cordao.caminhos {
        for (p, i) in amostrar(&c.pts, o.passo) {
            let Some(&(ua, ub)) = c.pernas.get(i) else { continue };
            let Some(t) = geo3d::unit(geo3d::sub(c.pts[i + 1], c.pts[i])) else { continue };
            let Some(r) = geo3d::unit(geo3d::add(ua, ub)) else { continue };
            // eixo do deslocamento: perpendicular à referência e ao cordão
            let w = fenkra_kernel::cross(t, r);
            let mut direcoes = vec![];
            let mut bloqueios: Vec<Id> = vec![];
            for a in &trab {
                for b in &desl {
                    let d = geo3d::rotate(geo3d::rotate(r, t, a.to_radians()), w, b.to_radians());
                    let (livre, dono) = testar(obs, p, d, o);
                    if let Some(id) = dono {
                        if !bloqueios.contains(&id) {
                            bloqueios.push(id);
                        }
                    }
                    direcoes.push(DirecaoTocha { trabalho: *a, deslocamento: *b, d, livre });
                }
            }
            let melhor = direcoes.iter().enumerate().filter(|(_, d)| d.livre).min_by(|(_, x), (_, y)| (x.trabalho.hypot(x.deslocamento)).total_cmp(&y.trabalho.hypot(y.deslocamento))).map(|(k, _)| k);
            out.push(PontoTocha { p, referencia: r, direcoes, melhor, bloqueios });
        }
    }
    out
}

/// Resumo e sugestões de um cordão (texto para o painel).
pub fn sugestoes(doc: &Document, pts: &[PontoTocha]) -> Vec<String> {
    let mut out = vec![];
    let inviaveis = pts.iter().filter(|p| !p.viavel()).count();
    if inviaveis > 0 {
        let mut quem: Vec<Id> = pts.iter().filter(|p| !p.viavel()).flat_map(|p| p.bloqueios.iter().copied()).collect();
        quem.sort();
        quem.dedup();
        let nomes: Vec<String> = quem.iter().map(|id| doc.name_of(*id)).collect();
        out.push(format!("{inviaveis} de {} pontos sem direção livre: solde antes de montar {} ou mude a ordem de soldagem", pts.len(), if nomes.is_empty() { "a peça que bloqueia".to_string() } else { nomes.join(", ") }));
    }
    let desviar: Vec<&PontoTocha> = pts.iter().filter(|p| p.melhor.is_some_and(|k| p.direcoes[k].trabalho != 0. || p.direcoes[k].deslocamento != 0.)).collect();
    if let Some(p) = desviar.first() {
        let d = p.direcoes[p.melhor.unwrap_or(0)];
        out.push(format!("{} pontos com a posição de referência bloqueada: incline a tocha (por exemplo {:.0}° de trabalho e {:.0}° de deslocamento)", desviar.len(), d.trabalho, d.deslocamento));
    }
    out
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn amostra_no_meio_de_cada_pedaco() {
        let a = amostrar(&[[0., 0., 0.], [100., 0., 0.]], 20.);
        assert_eq!(a.len(), 5);
        assert!((a[0].0[0] - 10.).abs() < 1e-9 && (a[4].0[0] - 90.).abs() < 1e-9);
    }
}
