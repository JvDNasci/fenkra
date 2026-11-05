// SPDX-License-Identifier: LGPL-2.1-or-later
//! B277: Aparar e dividir por região. O corte devolve os pedaços e a pessoa escolhe por clique quais
//! ficam. A ordem dos pedaços é do kernel e muda com o modelo, então a escolha não guarda número:
//! guarda uma âncora espacial, com o ponto clicado e de que lado de cada corte ele estava.
//!
//! O lado é o que segura a escolha quando a ferramenta anda: um corte que passa por cima do ponto
//! guardado deixaria o ponto no pedaço vizinho, mas o lado continua dizendo qual era o pedaço.

use super::*;
use fenkra_core::RegionMark;

/// Pedaço depois do corte, como a tela o recebe.
#[derive(Clone, Debug)]
pub struct RegiaoNota {
    /// de qual superfície veio: 0 a principal; `i + 1` a ferramenta `i` (modo mútuo)
    pub fonte: u32,
    /// ponto dentro do pedaço: onde a âncora nasce
    pub ponto: V3,
    /// lado do pedaço em cada corte (ver `lado_do_pedaco`)
    pub lados: Vec<i8>,
    pub area: f64,
    pub fica: bool,
    /// índices das marcas que caíram neste pedaço
    pub marcas: Vec<usize>,
    pub malha: Arc<Mesh>,
}

impl PartialEq for RegiaoNota {
    fn eq(&self, o: &Self) -> bool {
        (self.fonte, self.ponto, &self.lados, self.area, self.fica, &self.marcas) == (o.fonte, o.ponto, &o.lados, o.area, o.fica, &o.marcas)
    }
}

impl RegiaoNota {
    /// Âncora que grava a escolha deste pedaço.
    pub fn marca(&self) -> RegionMark {
        RegionMark { source: self.fonte, point: self.ponto, sides: self.lados.clone() }
    }
}

/// Ponto da malha perto do centro de área, sempre sobre um triângulo: o centro de uma folha em L, por
/// exemplo, cairia fora dela.
pub fn ponto_interno(m: &Mesh) -> Option<V3> {
    let p = |i: u32| [m.positions[i as usize * 3] as f64, m.positions[i as usize * 3 + 1] as f64, m.positions[i as usize * 3 + 2] as f64];
    let tris: Vec<(V3, f64)> = m
        .indices
        .chunks_exact(3)
        .map(|t| {
            let (a, b, c) = (p(t[0]), p(t[1]), p(t[2]));
            (geo3d::mul(geo3d::add(geo3d::add(a, b), c), 1. / 3.), geo3d::norm(geo3d::cross(geo3d::sub(b, a), geo3d::sub(c, a))) / 2.)
        })
        .filter(|x| x.1 > 1e-12)
        .collect();
    let total: f64 = tris.iter().map(|x| x.1).sum();
    if total <= 0. {
        return None;
    }
    let centro = tris.iter().fold([0.; 3], |acc, x| geo3d::add(acc, geo3d::mul(x.0, x.1 / total)));
    tris.iter().map(|x| x.0).min_by(|a, b| geo3d::dist(*a, centro).total_cmp(&geo3d::dist(*b, centro)))
}

/// Área somada dos triângulos.
pub fn area_da_malha(m: &Mesh) -> f64 {
    let p = |i: u32| [m.positions[i as usize * 3] as f64, m.positions[i as usize * 3 + 1] as f64, m.positions[i as usize * 3 + 2] as f64];
    m.indices.chunks_exact(3).map(|t| geo3d::norm(geo3d::cross(geo3d::sub(p(t[1]), p(t[0])), geo3d::sub(p(t[2]), p(t[0])))) / 2.).sum()
}

/// Mesmo lado em todo corte que os dois conhecem. Zero é "sobre o corte" e não conta; corte acrescentado
/// depois da marca também não.
pub fn mesmos_lados(a: &[i8], b: &[i8]) -> bool {
    a.iter().zip(b).all(|(x, y)| *x == 0 || *y == 0 || x == y)
}

/// Pedaço da marca: entre os da mesma origem com os mesmos lados, o mais perto do ponto. `None` quer
/// dizer que o pedaço sumiu; aplicar a marca no vizinho seria escolher por conta própria.
pub fn resolver_marca(fontes_e_lados: &[(u32, &[i8])], m: &RegionMark, dist: &dyn Fn(usize) -> f64) -> Option<usize> {
    (0..fontes_e_lados.len()).filter(|i| fontes_e_lados[*i].0 == m.source && mesmos_lados(fontes_e_lados[*i].1, &m.sides)).min_by(|a, b| dist(*a).total_cmp(&dist(*b)))
}

fn lado_do_ponto(corte: &Shape, p: V3) -> i8 {
    corte.surface_near(p).map_or(0, |(q, n)| {
        let h = geo3d::dot(geo3d::sub(p, q), n);
        if h.abs() < 1e-7 {
            0
        } else {
            h.signum() as i8
        }
    })
}

/// O pedaço passa dos dois lados do corte: a ferramenta não o separou.
pub const DOS_DOIS_LADOS: i8 = 2;

/// Lado do pedaço no corte. O ponto de dentro só não basta: uma folha mais alta que a ferramenta fica
/// com um rasgo e continua inteira, e o ponto dela diria "deste lado". Por isso os vértices votam; com
/// um de cada lado, o pedaço não é mais o que a marca guardou.
fn lado_do_pedaco(corte: &Shape, ponto: V3, m: &Mesh) -> i8 {
    let lado = lado_do_ponto(corte, ponto);
    let n = m.positions.len() / 3;
    let passo = (n / 24).max(1);
    let vertice = |i: usize| [m.positions[i * 3] as f64, m.positions[i * 3 + 1] as f64, m.positions[i * 3 + 2] as f64];
    let mut visto = lado;
    for i in (0..n).step_by(passo) {
        let l = lado_do_ponto(corte, vertice(i));
        if l != 0 && visto != 0 && l != visto {
            return DOS_DOIS_LADOS;
        }
        if visto == 0 {
            visto = l;
        }
    }
    lado
}

/// Entradas do comando.
pub(crate) struct Corte<'a> {
    pub surface: &'a SurfRef,
    pub tools: &'a [SurfRef],
    pub curve: Option<&'a CurveRef>,
    pub mutual: bool,
    pub remove: bool,
    pub extend: bool,
    pub marks: &'a [RegionMark],
}

impl Ctx<'_> {
    /// A curva sobre a superfície vira uma faixa na normal, que corta dos dois lados.
    fn faixa_da_curva(&self, c: &CurveRef, sh: &Shape, d: u32) -> Result<Shape, String> {
        let c = self.curve(c, d)?;
        let (_, n) = sh.surface_near(c.pts[c.pts.len() / 2]).map_err(|x| x.0)?;
        let l = self.model_size();
        let pts: Vec<V3> = c.pts.iter().map(|p| geo3d::sub(*p, geo3d::mul(n, l))).collect();
        Shape::extrude_curve(&pts, n, 2. * l).map_err(|x| x.0)
    }

    /// Corta, publica os pedaços na nota e devolve os que ficam, costurados.
    pub(crate) fn regioes(&self, id: Id, k: Corte, d: u32) -> Result<Shape, String> {
        let base = self.cut_shape(k.surface, d)?;
        // todo corte, na ordem que as marcas guardam: a superfície, as ferramentas e a faixa da curva
        let mut cortes = vec![base.clone()];
        for t in k.tools {
            cortes.push(self.cut_shape(t, d)?);
        }
        if let Some(c) = k.curve {
            let faixa = self.faixa_da_curva(c, &base, d)?;
            cortes.push(faixa);
        }
        if cortes.len() < 2 {
            return Err("escolha com o que cortar".into());
        }
        // plano de referência só corta: não tem borda, não é um pedaço para escolher
        let mut fontes = vec![0usize];
        if k.mutual {
            fontes.extend(k.tools.iter().enumerate().filter(|(_, t)| !matches!(t, SurfRef::Plane(_))).map(|(i, _)| i + 1));
        }
        let mut pecas: Vec<(Shape, RegiaoNota)> = vec![];
        for f in fontes {
            let mut acc = Shape::empty();
            for (_, c) in cortes.iter().enumerate().filter(|(i, _)| *i != f) {
                acc = if acc.is_null() { c.clone() } else { acc.compound(c) };
            }
            for parte in cortes[f].regions(&acc, k.extend).map_err(|x| x.0)? {
                let malha = tesselar(&parte, &self.qualidade).map_err(|x| x.0)?;
                let Some(ponto) = ponto_interno(&malha) else { continue };
                let lados = cortes.iter().enumerate().map(|(i, c)| if i == f { 0 } else { lado_do_pedaco(c, ponto, &malha) }).collect();
                let area = parte.mass().area;
                pecas.push((parte, RegiaoNota { fonte: f as u32, ponto, lados, area, fica: true, marcas: vec![], malha: Arc::new(malha) }));
            }
        }
        if pecas.len() < 2 {
            return Err("o elemento de corte não atravessa (nada a dividir)".into());
        }
        let chaves: Vec<(u32, &[i8])> = pecas.iter().map(|p| (p.1.fonte, p.1.lados.as_slice())).collect();
        let mut achadas: Vec<(usize, usize)> = vec![];
        let mut perdidas = 0;
        for (i, m) in k.marks.iter().enumerate() {
            let gap = |j: usize| pecas[j].0.surface_near(m.point).map_or(f64::MAX, |x| geo3d::dist(x.0, m.point));
            match resolver_marca(&chaves, m, &gap) {
                Some(j) => achadas.push((j, i)),
                None => perdidas += 1,
            }
        }
        for (j, i) in achadas {
            pecas[j].1.marcas.push(i);
        }
        for p in &mut pecas {
            p.1.fica = p.1.marcas.is_empty() == k.remove;
        }
        let mut nota = RefNote { regioes: pecas.iter().map(|p| p.1.clone()).collect(), ..Default::default() };
        let mut avisos = vec![];
        if perdidas > 0 {
            avisos.push(format!("{perdidas} escolha(s) não acharam mais o pedaço: confira as regiões"));
        }
        if !k.mutual {
            let soltos: Vec<String> = k.tools.iter().enumerate().filter(|(i, _)| base.section(&cortes[i + 1]).is_ok_and(|c| c.is_empty())).map(|(i, _)| (i + 1).to_string()).collect();
            if !soltos.is_empty() {
                avisos.push(format!("o elemento de corte {} não cruza a superfície e não cortou nada", soltos.join(", ")));
            }
        }
        nota.warn = !avisos.is_empty();
        nota.text = avisos.join("; ");
        self.notes.borrow_mut().insert(id, nota);
        let mut acc = Shape::empty();
        for (s, _) in pecas.iter().filter(|p| p.1.fica) {
            acc = if acc.is_null() { s.clone() } else { acc.compound(s) };
        }
        if acc.is_null() {
            return Err(if k.remove { "todos os pedaços saíram: clique em um para ele ficar".into() } else { "clique nos pedaços que ficam".into() });
        }
        acc.sewn(1e-3).map_err(|x| x.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn marca(source: u32, point: V3, sides: &[i8]) -> RegionMark {
        RegionMark { source, point, sides: sides.to_vec() }
    }

    /// O lado vence a distância: a ferramenta andou e o ponto ficou no pedaço vizinho.
    #[test]
    fn lado_vence_a_distancia() {
        let (a, b): (&[i8], &[i8]) = (&[0, 1], &[0, -1]);
        let pecas = [(0, a), (0, b)];
        let dist = |i: usize| [5., 0.][i];
        assert_eq!(resolver_marca(&pecas, &marca(0, [0.; 3], &[0, 1]), &dist), Some(0));
        assert_eq!(resolver_marca(&pecas, &marca(0, [0.; 3], &[0, -1]), &dist), Some(1));
    }

    /// Dois pedaços do mesmo lado: fica o mais perto do ponto.
    #[test]
    fn mesmo_lado_desempata_pela_distancia() {
        let a: &[i8] = &[0, 1];
        let pecas = [(0, a), (0, a)];
        assert_eq!(resolver_marca(&pecas, &marca(0, [0.; 3], &[0, 1]), &|i| [9., 2.][i]), Some(1));
    }

    /// Pedaço de outra superfície não serve, e o que sumiu não é trocado por outro.
    #[test]
    fn pedaco_que_sumiu_nao_vira_outro() {
        let a: &[i8] = &[0, 1];
        assert_eq!(resolver_marca(&[(1, a)], &marca(0, [0.; 3], &[0, 1]), &|_| 0.), None);
        assert_eq!(resolver_marca(&[(0, a)], &marca(0, [0.; 3], &[0, -1]), &|_| 0.), None, "mesmo com o ponto em cima do vizinho");
    }

    /// Corte novo acrescentado depois da marca não invalida a marca.
    #[test]
    fn corte_novo_nao_invalida_a_marca() {
        assert!(mesmos_lados(&[0, 1, -1], &[0, 1]));
        assert!(!mesmos_lados(&[0, -1, -1], &[0, 1]));
    }
}
