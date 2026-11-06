// SPDX-License-Identifier: LGPL-2.1-or-later
//! Cortes completos e vistas especiais (B106): linha de corte com vários segmentos (desvio e meio
//! corte), profundidade total ou por distância, fatia, peças que não são cortadas, acabamento da
//! borda e o contorno livre do detalhe.
//!
//! Como funciona no kernel: todo plano de corte contém a direção de observação da vista mãe, porque
//! a linha é traçada nela. Então o material a tirar é sempre um prisma reto: o polígono do lado do
//! observador, no plano da vista mãe, extrudado ao longo dessa direção. Uma linha reta dá meio espaço;
//! uma linha em degraus dá o recorte em degraus; uma linha com uma volta de 90 graus dá o quarto
//! (meio corte). A face cortada de cada segmento é a seção do plano daquele segmento recortada pela
//! faixa que o segmento ocupa; os degraus (paralelos à direção de observação) somem na projeção.

use crate::source::{add, cross, dot, scale3, sub, unit, Axes};
use crate::{Drawing, ViewKind, V2, V3};
use fenkra_core::Id;
use fenkra_kernel::{Bool, Shape};
use serde::{Deserialize, Serialize};

/// Como a borda da face cortada é desenhada.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum Borda {
    /// traço contínuo
    #[default]
    Lisa,
    /// traço de ruptura em zigue-zague
    Serrilhada,
}

impl Borda {
    pub const ALL: [Borda; 2] = [Borda::Lisa, Borda::Serrilhada];
    pub fn label(self) -> &'static str {
        match self {
            Borda::Lisa => "Lisa",
            Borda::Serrilhada => "Serrilhada",
        }
    }
}

/// Opções do corte além da reta a–b do `ViewKind::Section`.
#[derive(Clone, PartialEq, Debug, Default, Serialize, Deserialize)]
pub struct CorteOpcoes {
    /// vértices depois de `b` (coordenadas da vista mãe): vazio = corte reto
    #[serde(default)]
    pub pontos: Vec<V2>,
    /// distância visível atrás do plano (mm do modelo); `None` = profundidade total
    #[serde(default)]
    pub profundidade: Option<f64>,
    /// só a face cortada, sem o que está atrás
    #[serde(default)]
    pub fatia: bool,
    /// peças da montagem (índice na ordem de carga) que aparecem inteiras, sem corte
    #[serde(default)]
    pub sem_corte: Vec<usize>,
    #[serde(default)]
    pub borda: Borda,
}

impl CorteOpcoes {
    pub fn padrao(&self) -> bool {
        *self == CorteOpcoes::default()
    }
}

/// Contorno do detalhe em perfil livre (vértices na vista mãe). Vazio = círculo.
#[derive(Clone, PartialEq, Debug, Default, Serialize, Deserialize)]
pub struct DetalheOpcoes {
    #[serde(default)]
    pub contorno: Vec<V2>,
}

/// Linha de corte completa: `a`, `b` e os vértices extras.
pub fn caminho(a: V2, b: V2, extra: &[V2]) -> Vec<V2> {
    let mut v = vec![a, b];
    v.extend_from_slice(extra);
    v
}

/// Linha de corte da vista `id` (vazia se não é um corte).
pub fn caminho_da_vista(d: &Drawing, id: Id) -> Vec<V2> {
    match d.view(id) {
        Some(v) => match &v.kind {
            ViewKind::Section { a, b, .. } => caminho(*a, *b, &v.corte.pontos),
            _ => vec![],
        },
        None => vec![],
    }
}

/// Grava a linha de corte (no mínimo dois pontos) na vista. `false` se não é corte ou faltam pontos.
pub fn definir_caminho(d: &mut Drawing, id: Id, pts: &[V2]) -> bool {
    if pts.len() < 2 {
        return false;
    }
    let Some(v) = d.views.iter_mut().find(|v| v.id == id) else { return false };
    match &mut v.kind {
        ViewKind::Section { a, b, .. } => {
            *a = pts[0];
            *b = pts[1];
            v.corte.pontos = pts[2..].to_vec();
            true
        }
        _ => false,
    }
}

fn v2unit(a: V2, b: V2) -> Option<V2> {
    let d = [b[0] - a[0], b[1] - a[1]];
    let l = (d[0] * d[0] + d[1] * d[1]).sqrt();
    (l > 1e-9).then(|| [d[0] / l, d[1] / l])
}

/// Direção, na folha, para onde a vista do corte fica alinhada com a mãe (lado das setas): o corte
/// herda a coordenada ao longo da linha (a mesma do centro da mãe) e só desliza na perpendicular.
pub fn direcao_de_alinhamento(sa: V2, sb: V2) -> Option<V2> {
    let u = v2unit(sa, sb)?;
    Some([u[1], -u[0]])
}

/// Posição do corte na folha com o cursor em `m`: sobre a reta que passa pelo centro da mãe na
/// direção `dir` (de `direcao_de_alinhamento`), a menos que `livre` (Ctrl).
pub fn alinhar_ao_corte(mae: V2, dir: V2, m: V2, livre: bool) -> V2 {
    if livre {
        return m;
    }
    let t = (m[0] - mae[0]) * dir[0] + (m[1] - mae[1]) * dir[1];
    [mae[0] + dir[0] * t, mae[1] + dir[1] * t]
}

/// Normal do lado do observador de um segmento no plano da vista mãe.
pub fn normal_obs(u: V2) -> V2 {
    [-u[1], u[0]]
}

/// Remove vértices repetidos em sequência.
fn limpar(pts: &[V2]) -> Vec<V2> {
    let mut out: Vec<V2> = vec![];
    for p in pts {
        if out.last().is_none_or(|q| (q[0] - p[0]).hypot(q[1] - p[1]) > 1e-9) {
            out.push(*p);
        }
    }
    out
}

/// Polígono (plano da vista mãe) da região do lado do observador da linha de corte.
pub fn poligono_observador(pts: &[V2], big: f64) -> Option<Vec<V2>> {
    let pts = limpar(pts);
    if pts.len() < 2 {
        return None;
    }
    let u0 = v2unit(pts[0], pts[1])?;
    let n = pts.len();
    let ul = v2unit(pts[n - 2], pts[n - 1])?;
    let (n0, nl) = (normal_obs(u0), normal_obs(ul));
    let mut poly = vec![[pts[0][0] - u0[0] * big, pts[0][1] - u0[1] * big]];
    poly.extend_from_slice(&pts);
    let fim = [pts[n - 1][0] + ul[0] * big, pts[n - 1][1] + ul[1] * big];
    poly.push(fim);
    poly.push([fim[0] + nl[0] * big, fim[1] + nl[1] * big]);
    poly.push([poly[0][0] + n0[0] * big, poly[0][1] + n0[1] * big]);
    Some(poly)
}

/// Polígono da região mais funda que `dist` atrás do plano (para a profundidade por distância). Os
/// vértices internos andam pela bissetriz, para o recuo ser `dist` em cada segmento.
pub fn poligono_fundo(pts: &[V2], dist: f64, big: f64) -> Option<Vec<V2>> {
    let pts = limpar(pts);
    if pts.len() < 2 {
        return None;
    }
    let n = pts.len();
    let us: Vec<V2> = (0..n - 1).map(|i| v2unit(pts[i], pts[i + 1])).collect::<Option<_>>()?;
    let mut rec: Vec<V2> = vec![];
    for i in 0..n {
        let nm = if i == 0 {
            normal_obs(us[0])
        } else if i == n - 1 {
            normal_obs(us[n - 2])
        } else {
            let (a, b) = (normal_obs(us[i - 1]), normal_obs(us[i]));
            let s = [a[0] + b[0], a[1] + b[1]];
            let l = (s[0] * s[0] + s[1] * s[1]).sqrt();
            if l < 1e-9 {
                a
            } else {
                // miter: comprimento dist / cos(meio ângulo)
                let m = [s[0] / l, s[1] / l];
                let c = (m[0] * a[0] + m[1] * a[1]).max(0.2);
                [m[0] / c, m[1] / c]
            }
        };
        rec.push([pts[i][0] - nm[0] * dist, pts[i][1] - nm[1] * dist]);
    }
    let (u0, ul) = (us[0], us[n - 2]);
    let (n0, nl) = (normal_obs(u0), normal_obs(ul));
    let mut poly = vec![[rec[0][0] - u0[0] * big, rec[0][1] - u0[1] * big]];
    poly.extend_from_slice(&rec);
    let fim = [rec[n - 1][0] + ul[0] * big, rec[n - 1][1] + ul[1] * big];
    poly.push(fim);
    poly.push([fim[0] - nl[0] * big, fim[1] - nl[1] * big]);
    poly.push([poly[0][0] - n0[0] * big, poly[0][1] - n0[1] * big]);
    Some(poly)
}

/// Prisma que atravessa o modelo na direção de observação da vista mãe (`pa.toward`).
fn prisma(pa: &Axes, poly: &[V2], z0: f64, big: f64) -> Result<Shape, String> {
    let pts: Vec<V3> = poly.iter().map(|p| add(pa.unproject(*p), scale3(pa.toward, z0 - big))).collect();
    Shape::poly_prism(&pts, scale3(pa.toward, 2. * big)).map_err(|e| e.0)
}

/// Recorta um polígono 3D pelo semi-espaço `dot(p, u) >= lim` (`maior`) ou `<= lim`.
fn recortar_semi(poly: &[V3], u: V3, lim: f64, maior: bool) -> Vec<V3> {
    let dentro = |p: V3| if maior { dot(p, u) >= lim - 1e-9 } else { dot(p, u) <= lim + 1e-9 };
    let n = poly.len();
    let mut out = vec![];
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        let (da, db) = (dentro(a), dentro(b));
        if da {
            out.push(a);
        }
        if da != db {
            let (ta, tb) = (dot(a, u) - lim, dot(b, u) - lim);
            let t = ta / (ta - tb);
            out.push([0, 1, 2].map(|k| a[k] + (b[k] - a[k]) * t));
        }
    }
    out
}

/// Resultado de cortar o modelo.
pub struct Cortado {
    /// o que sobra do modelo (peças cortadas e peças inteiras)
    pub alvo: Shape,
    /// contornos da face cortada na vista (coordenadas da vista do corte) e a peça de cada um
    pub hachura: Vec<Vec<V2>>,
    pub grupo: Vec<usize>,
    /// x (na vista do corte) das junções entre segmentos: o contorno não é traçado ali
    pub costuras: Vec<f64>,
}

/// Corta `forma` (ou cada peça de `pecas`) pela linha `pts`. `axes` são os eixos da vista do corte
/// e `pa` os da vista mãe; `c3` é o centro do modelo, que dá a profundidade do plano.
pub fn cortar(forma: &Shape, pecas: &[Shape], pa: &Axes, axes: &Axes, pts: &[V2], op: &CorteOpcoes, c3: V3, big: f64) -> Result<Cortado, String> {
    let pts = limpar(pts);
    let z0 = dot(c3, pa.toward);
    let poly = poligono_observador(&pts, big).ok_or("linha de corte sem comprimento")?;
    let mut ferramentas = vec![prisma(pa, &poly, z0, big)?];
    if let Some(d) = op.profundidade.filter(|d| *d > 1e-6) {
        let fundo = poligono_fundo(&pts, d, big).ok_or("linha de corte sem comprimento")?;
        ferramentas.push(prisma(pa, &fundo, z0, big)?);
    }
    let cortar_peca = |s: &Shape| -> Result<Shape, String> {
        let mut r = s.clone();
        for t in &ferramentas {
            r = r.boolean(t, Bool::Cut).map_err(|e| e.0)?;
        }
        Ok(r)
    };
    // segmentos: origem 3D, direção, normal e faixa [s0, s1] ao longo da direção
    struct Seg {
        o: V3,
        n: V3,
        u: V3,
        s0: f64,
        s1: f64,
    }
    let mut segs = vec![];
    for w in pts.windows(2) {
        let (p, q) = (add(pa.unproject(w[0]), scale3(pa.toward, z0)), add(pa.unproject(w[1]), scale3(pa.toward, z0)));
        let Some(u) = unit(sub(q, p)) else { continue };
        segs.push(Seg { o: p, n: cross(pa.toward, u), u, s0: dot(p, u), s1: dot(q, u) });
    }
    let reto = segs.len() == 1;
    let mut hachura = vec![];
    let mut grupo = vec![];
    let mut corte_laco = |s: &Shape, g: usize| -> bool {
        let mut achou = false;
        for sg in &segs {
            let Ok(loops) = s.section_loops(sg.o, sg.n, big) else { continue };
            for lp in loops {
                let lp = if reto { lp } else { recortar_semi(&recortar_semi(&lp, sg.u, sg.s0, true), sg.u, sg.s1, false) };
                if lp.len() < 3 {
                    continue;
                }
                let pl: Vec<V2> = lp.iter().map(|p| axes.project(*p)).collect();
                // faixa sem área na projeção (degrau paralelo à direção de observação)
                let area: f64 = (0..pl.len()).map(|i| pl[i][0] * pl[(i + 1) % pl.len()][1] - pl[(i + 1) % pl.len()][0] * pl[i][1]).sum::<f64>().abs() / 2.;
                if area < 1e-6 {
                    continue;
                }
                hachura.push(pl);
                grupo.push(g);
                achou = true;
            }
        }
        achou
    };
    let multiplas = pecas.len() > 1 && !op.sem_corte.is_empty();
    let alvo = if multiplas {
        // montagem com peças que não são cortadas: cada uma por si, depois o conjunto
        let mut acc: Option<Shape> = None;
        let mut g = 0;
        for (i, p) in pecas.iter().enumerate() {
            let s = if op.sem_corte.contains(&i) {
                p.clone()
            } else {
                let c = cortar_peca(p)?;
                if corte_laco(p, g) {
                    g += 1;
                }
                c
            };
            acc = Some(match acc {
                None => s,
                Some(a) => a.compound(&s),
            });
        }
        acc.ok_or("montagem sem peças")?
    } else {
        let alvo = cortar_peca(forma)?;
        let mut g = 0;
        let mut feito = false;
        for p in pecas {
            if corte_laco(p, g) {
                g += 1;
                feito = true;
            }
        }
        if !feito {
            corte_laco(forma, 0);
        }
        alvo
    };
    let costuras = if reto { vec![] } else { segs.windows(2).map(|w| dot(w[1].o, axes.x)).collect() };
    Ok(Cortado { alvo, hachura, grupo, costuras })
}

/// Contorno da face cortada como traços de ruptura (zigue-zague) de amplitude `amp`, com `passo`
/// entre os dentes. Os dentes ficam para o lado de fora de cada aresta.
pub fn serrilhar(laco: &[V2], passo: f64, amp: f64) -> Vec<V2> {
    let n = laco.len();
    let mut out = vec![];
    for i in 0..n {
        let (a, b) = (laco[i], laco[(i + 1) % n]);
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let l = dx.hypot(dy);
        if l < 1e-9 {
            continue;
        }
        let (ux, uy) = (dx / l, dy / l);
        let k = ((l / passo).round() as usize).max(1);
        out.push(a);
        for j in 0..k {
            let t = (j as f64 + 0.5) / k as f64 * l;
            let s = if j % 2 == 0 { amp } else { -amp };
            out.push([a[0] + ux * t - uy * s, a[1] + uy * t + ux * s]);
        }
    }
    if let Some(p) = out.first().copied() {
        out.push(p);
    }
    out
}

/// Partes das polilinhas dentro do polígono `poly` (regra par/ímpar), cortando os segmentos nas
/// arestas: o recorte do detalhe em perfil livre.
pub fn recortar_poligono(pl: &[V2], poly: &[V2]) -> Vec<Vec<V2>> {
    let n = poly.len();
    if n < 3 {
        return vec![];
    }
    let dentro = |p: V2| {
        let mut c = false;
        let mut j = n - 1;
        for i in 0..n {
            let (a, b) = (poly[i], poly[j]);
            if (a[1] > p[1]) != (b[1] > p[1]) && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0] {
                c = !c;
            }
            j = i;
        }
        c
    };
    let lerp = |p: V2, q: V2, t: f64| [p[0] + (q[0] - p[0]) * t, p[1] + (q[1] - p[1]) * t];
    // parâmetros t em (0,1) onde o segmento p→q cruza uma aresta do polígono
    let cruza = |p: V2, q: V2| -> Vec<f64> {
        let r = [q[0] - p[0], q[1] - p[1]];
        let mut ts = vec![];
        for i in 0..n {
            let (a, b) = (poly[i], poly[(i + 1) % n]);
            let s = [b[0] - a[0], b[1] - a[1]];
            let den = r[0] * s[1] - r[1] * s[0];
            if den.abs() < 1e-12 {
                continue;
            }
            let ap = [a[0] - p[0], a[1] - p[1]];
            let t = (ap[0] * s[1] - ap[1] * s[0]) / den;
            let u = (ap[0] * r[1] - ap[1] * r[0]) / den;
            if t > 1e-9 && t < 1. - 1e-9 && (0. ..=1.).contains(&u) {
                ts.push(t);
            }
        }
        ts.sort_by(f64::total_cmp);
        ts
    };
    let mut out = vec![];
    let mut cur: Vec<V2> = vec![];
    for w in pl.windows(2) {
        let (p, q) = (w[0], w[1]);
        let mut ts = vec![0.];
        ts.extend(cruza(p, q));
        ts.push(1.);
        for s in ts.windows(2) {
            let (a, b) = (lerp(p, q, s[0]), lerp(p, q, s[1]));
            if dentro(lerp(p, q, (s[0] + s[1]) / 2.)) {
                if cur.is_empty() {
                    cur.push(a);
                }
                cur.push(b);
            } else if cur.len() >= 2 {
                out.push(std::mem::take(&mut cur));
            } else {
                cur.clear();
            }
        }
    }
    if cur.len() >= 2 {
        out.push(cur);
    }
    out
}

/// Define o contorno livre do detalhe (mínimo de 3 vértices). Vazio volta ao círculo. `false` se
/// não é um detalhe.
pub fn definir_contorno_detalhe(d: &mut Drawing, id: Id, contorno: Vec<V2>) -> bool {
    if !contorno.is_empty() && contorno.len() < 3 {
        return false;
    }
    let Some(v) = d.views.iter_mut().find(|v| v.id == id) else { return false };
    if !matches!(v.kind, ViewKind::Detail { .. }) || v.detalhe.contorno == contorno {
        return false;
    }
    v.detalhe.contorno = contorno;
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn poligono_observador_cobre_o_lado_do_observador() {
        // linha horizontal da esquerda para a direita: o observador fica em +y
        let p = poligono_observador(&[[0., 0.], [10., 0.]], 100.).unwrap();
        assert!(p.iter().any(|q| q[1] > 50.), "{p:?}");
        assert!(p.iter().all(|q| q[1] >= -1e-9), "nada do lado oposto: {p:?}");
        // invertida, o lado muda
        let q = poligono_observador(&[[10., 0.], [0., 0.]], 100.).unwrap();
        assert!(q.iter().all(|q| q[1] <= 1e-9));
    }

    #[test]
    fn poligono_do_fundo_recua_a_distancia_pedida() {
        let p = poligono_fundo(&[[0., 0.], [10., 0.]], 3., 100.).unwrap();
        assert!(p.iter().all(|q| q[1] <= -3. + 1e-9), "{p:?}");
        // com uma volta de 90 graus o recuo de cada segmento continua sendo 3 (bissetriz): o lado
        // do observador do segundo trecho é -x, então o fundo dele fica em x = 10 + 3
        let p = poligono_fundo(&[[0., 0.], [10., 0.], [10., 10.]], 3., 100.).unwrap();
        assert!(p.iter().any(|q| (q[0] - 13.).abs() < 1e-9 && (q[1] + 3.).abs() < 1e-9), "{p:?}");
    }

    #[test]
    fn recorte_do_poligono_corta_a_linha_nas_bordas() {
        let quad = [[0., 0.], [10., 0.], [10., 10.], [0., 10.]];
        let r = recortar_poligono(&[[-5., 5.], [15., 5.]], &quad);
        assert_eq!(r.len(), 1);
        assert!((r[0][0][0]).abs() < 1e-9 && (r[0].last().unwrap()[0] - 10.).abs() < 1e-9);
        assert!(recortar_poligono(&[[-5., 20.], [15., 20.]], &quad).is_empty());
    }

    #[test]
    fn serrilhado_alterna_os_dentes_dos_dois_lados() {
        let z = serrilhar(&[[0., 0.], [10., 0.], [10., 5.], [0., 5.]], 2., 0.4);
        assert!(z.len() > 12);
        assert_eq!(z.first(), z.last());
        assert!(z.iter().any(|p| p[1] > 0.3 && p[1] < 1.) && z.iter().any(|p| p[1] < -0.3));
    }

    #[test]
    fn semi_espaco_corta_o_laco_na_faixa() {
        let q = vec![[0., 0., 0.], [10., 0., 0.], [10., 5., 0.], [0., 5., 0.]];
        let r = recortar_semi(&recortar_semi(&q, [1., 0., 0.], 2., true), [1., 0., 0.], 6., false);
        let xs: Vec<f64> = r.iter().map(|p| p[0]).collect();
        assert!(xs.iter().all(|x| *x >= 2. - 1e-9 && *x <= 6. + 1e-9), "{xs:?}");
        assert_eq!(r.len(), 4);
    }

    // ---- com o kernel: volume e caixa do que sobra depois do corte

    fn placa() -> Option<(Shape, Axes, V3, f64, V2, V2)> {
        if !fenkra_kernel::AVAILABLE {
            return None;
        }
        let sh = fenkra_solidos::final_shape(&fenkra_solidos::example_document())?;
        let (lo, hi) = sh.bbox()?;
        let c3 = [0, 1, 2].map(|k| (lo[k] + hi[k]) / 2.);
        let big = (0..3).map(|k| hi[k] - lo[k]).fold(0., f64::max) * 2. + 10.;
        // vista mãe: frente (x para a direita, z para cima)
        let pa = Axes::of(crate::Dir::Frente);
        let (plo, phi) = ([pa.project(lo)[0], pa.project(lo)[1]], [pa.project(hi)[0], pa.project(hi)[1]]);
        let _ = (plo, phi);
        let _ = c3;
        Some((sh, pa, c3, big, [lo[0], lo[2]], [hi[0], hi[2]]))
    }

    /// Eixos da vista do corte para a linha que começa em `p0` e termina em `p1`.
    fn eixos_do_corte(pa: &Axes, p0: V2, p1: V2) -> Axes {
        let u = unit(sub(pa.unproject(p1), pa.unproject(p0))).unwrap();
        Axes::new(cross(pa.toward, u), u)
    }

    fn volume(s: &Shape) -> f64 {
        s.mass().volume
    }

    #[test]
    fn corte_reto_remove_o_lado_do_observador() {
        let Some((sh, pa, c3, big, lo, hi)) = placa() else { return };
        let ym = (lo[1] + hi[1]) / 2.;
        let pts = [[lo[0] - 5., ym], [hi[0] + 5., ym]];
        let ax = eixos_do_corte(&pa, pts[0], pts[1]);
        let c = cortar(&sh, &[], &pa, &ax, &pts, &CorteOpcoes::default(), c3, big).unwrap();
        let (v, vt) = (volume(&c.alvo), volume(&sh));
        assert!(v > 0. && v < vt * 0.8, "sobra a metade do lado oposto: {v} de {vt}");
        assert!(!c.hachura.is_empty() && c.costuras.is_empty());
        // a caixa em z (vertical da frente) termina no plano do corte
        let (blo, bhi) = c.alvo.bbox().unwrap();
        assert!((bhi[2] - ym).abs() < 1e-3 && (blo[2] - lo[1]).abs() < 1e-3, "{blo:?} {bhi:?} ym={ym}");
    }

    #[test]
    fn corte_em_degraus_tira_menos_de_um_lado_e_mais_do_outro() {
        let Some((sh, pa, c3, big, lo, hi)) = placa() else { return };
        let (y1, y2) = (lo[1] + (hi[1] - lo[1]) * 0.35, lo[1] + (hi[1] - lo[1]) * 0.7);
        let xm = (lo[0] + hi[0]) / 2.;
        let reto = |y: f64| {
            let pts = [[lo[0] - 5., y], [hi[0] + 5., y]];
            let ax = eixos_do_corte(&pa, pts[0], pts[1]);
            volume(&cortar(&sh, &[], &pa, &ax, &pts, &CorteOpcoes::default(), c3, big).unwrap().alvo)
        };
        let (v1, v2) = (reto(y1), reto(y2));
        let pts = [[lo[0] - 5., y1], [xm, y1], [xm, y2], [hi[0] + 5., y2]];
        let ax = eixos_do_corte(&pa, pts[0], pts[1]);
        let c = cortar(&sh, &[], &pa, &ax, &pts, &CorteOpcoes::default(), c3, big).unwrap();
        let v = volume(&c.alvo);
        assert!(v > v1 + 1e-6 && v < v2 - 1e-6, "o desvio fica entre os dois cortes retos: {v1} < {v} < {v2}");
        assert_eq!(c.costuras.len(), 2, "uma junção por vértice interno");
        assert!(!c.hachura.is_empty());
        let (_, bhi) = c.alvo.bbox().unwrap();
        assert!((bhi[2] - y2).abs() < 1e-3, "do lado direito o plano é o mais alto");
    }

    #[test]
    fn meio_corte_tira_so_um_quarto() {
        let Some((sh, pa, c3, big, lo, hi)) = placa() else { return };
        let ym = (lo[1] + hi[1]) / 2.;
        let xm = (lo[0] + hi[0]) / 2.;
        let reto = {
            let pts = [[lo[0] - 5., ym], [hi[0] + 5., ym]];
            let ax = eixos_do_corte(&pa, pts[0], pts[1]);
            volume(&cortar(&sh, &[], &pa, &ax, &pts, &CorteOpcoes::default(), c3, big).unwrap().alvo)
        };
        let pts = [[lo[0] - 5., ym], [xm, ym], [xm, hi[1] + 5.]];
        let ax = eixos_do_corte(&pa, pts[0], pts[1]);
        let c = cortar(&sh, &[], &pa, &ax, &pts, &CorteOpcoes::default(), c3, big).unwrap();
        let (v, vt) = (volume(&c.alvo), volume(&sh));
        assert!(v > reto + 1e-6 && v < vt - 1e-6, "{reto} < {v} < {vt}");
        let (_, bhi) = c.alvo.bbox().unwrap();
        assert!((bhi[2] - hi[1]).abs() < 1e-3, "o outro lado continua inteiro até o topo");
    }

    #[test]
    fn profundidade_por_distancia_limita_o_que_aparece_atras() {
        let Some((sh, pa, c3, big, lo, hi)) = placa() else { return };
        let ym = (lo[1] + hi[1]) / 2.;
        let pts = [[lo[0] - 5., ym], [hi[0] + 5., ym]];
        let ax = eixos_do_corte(&pa, pts[0], pts[1]);
        let total = cortar(&sh, &[], &pa, &ax, &pts, &CorteOpcoes::default(), c3, big).unwrap();
        let d = (hi[1] - lo[1]) * 0.1;
        let rasa = cortar(&sh, &[], &pa, &ax, &pts, &CorteOpcoes { profundidade: Some(d), ..Default::default() }, c3, big).unwrap();
        assert!(volume(&rasa.alvo) < volume(&total.alvo) - 1e-6);
        let (blo, bhi) = rasa.alvo.bbox().unwrap();
        // a caixa do kernel é folgada nas superfícies curvas (furos): tolerância de 0,2 mm
        assert!((bhi[2] - ym).abs() < 1e-3 && blo[2] > ym - d - 0.2, "{blo:?} {bhi:?}");
        assert!(!rasa.hachura.is_empty(), "a face do plano continua hachurada");
    }

    #[test]
    fn caminho_e_inversao_com_varios_segmentos() {
        let mut d = Drawing::new("t", crate::Format::A3);
        let s = d.sheets[0].id;
        let src = crate::Source { path: "x.fkp".into(), display_state: String::new(), hidden: vec![] };
        let m = d.add_view(s, src.clone(), ViewKind::Standard(crate::Dir::Frente), [0., 0.], 1.);
        let c = d.add_view(s, src, ViewKind::Section { parent: m, a: [0., 0.], b: [5., 0.], label: "A".into() }, [50., 0.], 1.);
        assert!(definir_caminho(&mut d, c, &[[0., 0.], [5., 0.], [5., 4.], [9., 4.]]));
        assert_eq!(caminho_da_vista(&d, c).len(), 4);
        assert!(crate::alcas::inverter_corte(&mut d, c));
        assert_eq!(caminho_da_vista(&d, c), vec![[9., 4.], [5., 4.], [5., 0.], [0., 0.]]);
        assert!(!definir_caminho(&mut d, c, &[[0., 0.]]));
        // o desenho guarda e relê as opções
        d.views.iter_mut().find(|v| v.id == c).unwrap().corte.fatia = true;
        let r = Drawing::from_json(&d.to_json()).unwrap();
        assert!(r.view(c).unwrap().corte.fatia);
    }

    #[test]
    fn peca_marcada_como_sem_corte_aparece_inteira() {
        let Some((sh, pa, c3, big, lo, hi)) = placa() else { return };
        let ym = (lo[1] + hi[1]) / 2.;
        let pts = [[lo[0] - 5., ym], [hi[0] + 5., ym]];
        let ax = eixos_do_corte(&pa, pts[0], pts[1]);
        // duas peças lado a lado (a segunda deslocada em profundidade)
        let pecas = [sh.clone(), sh.transformed(fenkra_kernel::Xform::Translate([0., 100., 0.])).unwrap()];
        let conjunto = pecas[0].compound(&pecas[1]);
        let ambas = cortar(&conjunto, &pecas, &pa, &ax, &pts, &CorteOpcoes::default(), c3, big).unwrap();
        let uma = cortar(&conjunto, &pecas, &pa, &ax, &pts, &CorteOpcoes { sem_corte: vec![1], ..Default::default() }, c3, big).unwrap();
        let inteira = volume(&sh);
        let cortada = volume(&ambas.alvo) / 2.;
        let v = volume(&uma.alvo);
        assert!(cortada < inteira * 0.8 && (v - (cortada + inteira)).abs() < 1e-3 * inteira, "{v} ~ {cortada} + {inteira}");
        assert_eq!(ambas.grupo.iter().max(), Some(&1), "cada peça cortada tem o seu grupo de hachura");
        assert_eq!(uma.grupo.iter().max(), Some(&0), "a peça inteira não é hachurada");
    }

    fn montar(nome: &str) -> (crate::source::Models, Drawing, crate::Id, crate::Source, V2, V2, crate::Id) {
        let dir = std::env::temp_dir().join(nome);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("placa.fkp"), fenkra_solidos::example_document().to_json()).unwrap();
        let fonte = crate::Source { path: "placa.fkp".into(), display_state: String::new(), hidden: vec![] };
        let mut d = Drawing::new("t", crate::Format::A3);
        let s = d.sheets[0].id;
        let mut m = crate::source::Models::new(Some(dir));
        let f = d.add_view(s, fonte.clone(), ViewKind::Standard(crate::Dir::Frente), [100., 100.], 1.);
        let pf = m.proj(&d, f);
        (m, d, s, fonte, pf.lo, pf.hi, f)
    }

    #[test]
    fn linhas_de_explosao_do_estado_seguem_para_a_vista() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        use fenkra_montagem::display::{Explosao, PassoExplosao, ModoPasso};
        use fenkra_montagem::{Assembly, NodeKind, Placement};
        let dir = std::env::temp_dir().join(format!("fenkra_desenho_explosao_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("placa.fkp"), fenkra_solidos::example_document().to_json()).unwrap();
        let mut a = Assembly::new("m");
        let c = a.add_component("P", NodeKind::Part { path: "placa.fkp".into() }, Placement::default(), None);
        let mut e = Explosao::nova("E");
        e.passos.push(PassoExplosao { nome: "p".into(), comps: vec![c], modo: ModoPasso::Linear { dir: [1., 0., 0.], dist: 80. }, suprimido: false });
        let centros = [(c, [0., 0., 0.])].into();
        let mut estado = e.no_estado(&a.snapshot_state("Explodida"), &centros);
        estado.name = "Explodida".into();
        a.display_states.push(estado);
        a.explosoes.push(e);
        std::fs::write(dir.join("m.fka"), a.to_json()).unwrap();
        let mut d = Drawing::new("t", crate::Format::A3);
        let s = d.sheets[0].id;
        let mut m = crate::source::Models::new(Some(dir.clone()));
        let src = |estado: &str| crate::Source { path: "m.fka".into(), display_state: estado.into(), hidden: vec![] };
        let com = d.add_view(s, src("Explodida"), ViewKind::Standard(crate::Dir::Topo), [100., 100.], 1.);
        let sem = d.add_view(s, src(""), ViewKind::Standard(crate::Dir::Topo), [300., 100.], 1.);
        let p = m.proj(&d, com);
        assert!(p.error.is_none(), "{:?}", p.error);
        let l: Vec<_> = p.lines.iter().filter(|x| x.1 == 7).collect();
        assert_eq!(l.len(), 1, "uma linha para a peça que saiu do lugar");
        let (q0, q1) = (l[0].0[0], l[0].0[1]);
        assert!(((q1[0] - q0[0]).hypot(q1[1] - q0[1]) - 80.).abs() < 1e-6, "{q0:?} {q1:?}");
        assert!(!m.proj(&d, sem).lines.iter().any(|x| x.1 == 7), "sem estado, sem linha");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn fatia_so_tem_a_face_cortada_e_o_corte_em_degraus_refaz_a_projecao() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (mut m, mut d, s, fonte, lo, hi, f) = montar("fenkra_desenho_corte_fatia");
        let ym = (lo[1] + hi[1]) / 2.;
        let c = d.add_view(s, fonte, ViewKind::Section { parent: f, a: [lo[0] - 5., ym], b: [hi[0] + 5., ym], label: "A".into() }, [300., 100.], 1.);
        let cheio = m.proj(&d, c);
        assert!(cheio.error.is_none() && cheio.lines.len() > 4);
        d.views.iter_mut().find(|v| v.id == c).unwrap().corte.fatia = true;
        let fatia = m.proj(&d, c);
        assert!(fatia.error.is_none(), "{:?}", fatia.error);
        assert_eq!(fatia.lines.len(), fatia.hatch.len(), "só o contorno de cada face cortada");
        assert!(fatia.lines.iter().all(|(_, k)| *k == 0));
        // mudar o caminho invalida o cache da projeção
        d.views.iter_mut().find(|v| v.id == c).unwrap().corte.fatia = false;
        let xm = (lo[0] + hi[0]) / 2.;
        assert!(definir_caminho(&mut d, c, &[[lo[0] - 5., ym], [xm, ym], [xm, ym + 3.], [hi[0] + 5., ym + 3.]]));
        let degraus = m.proj(&d, c);
        assert!(degraus.error.is_none(), "{:?}", degraus.error);
        assert_eq!(degraus.seams.len(), 2);
        assert_ne!(degraus.hatch.len() + degraus.lines.len(), cheio.hatch.len() + cheio.lines.len());
    }

    #[test]
    fn detalhe_em_perfil_livre_recorta_pelo_poligono() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (mut m, mut d, s, fonte, lo, _hi, f) = montar("fenkra_desenho_corte_detalhe");
        let det = d.add_view(s, fonte, ViewKind::Detail { parent: f, center: lo, radius: 10., label: "B".into() }, [300., 200.], 2.);
        let circ = m.proj(&d, det);
        assert!(circ.lines.iter().any(|(_, k)| *k == 4));
        // perfil triangular pequeno no canto
        let tri = vec![lo, [lo[0] + 12., lo[1]], [lo[0], lo[1] + 6.]];
        assert!(definir_contorno_detalhe(&mut d, det, tri.clone()));
        assert!(!definir_contorno_detalhe(&mut d, det, tri.clone()), "igual não muda");
        assert!(!definir_contorno_detalhe(&mut d, det, vec![lo, lo]), "menos de 3 vértices");
        let p = m.proj(&d, det);
        assert!(p.error.is_none());
        let borda = p.lines.iter().find(|(_, k)| *k == 4).unwrap();
        assert_eq!(borda.0.len(), 4, "o triângulo fecha em 4 pontos");
        for (pl, _) in &p.lines {
            for q in pl {
                assert!(q[0] >= lo[0] - 1e-6 && q[1] >= lo[1] - 1e-6 && q[0] / 12. + q[1] / 6. - (lo[0] / 12. + lo[1] / 6.) <= 1. + 1e-6, "{q:?} fora do perfil");
            }
        }
        // sem contorno volta ao círculo
        assert!(definir_contorno_detalhe(&mut d, det, vec![]));
        assert!(m.proj(&d, det).lines.iter().any(|(pl, k)| *k == 4 && pl.len() > 8));
    }

    #[test]
    fn corte_desliza_so_na_perpendicular_e_ctrl_solta() {
        // linha horizontal na folha: a vista do corte fica alinhada na vertical com a mãe
        let dir = direcao_de_alinhamento([10., 50.], [90., 50.]).unwrap();
        assert!((dir[0]).abs() < 1e-9 && (dir[1] + 1.).abs() < 1e-9, "setas para baixo: {dir:?}");
        let mae = [50., 60.];
        assert_eq!(alinhar_ao_corte(mae, dir, [200., 20.], false), [50., 20.]);
        assert_eq!(alinhar_ao_corte(mae, dir, [200., 20.], true), [200., 20.]);
        // linha inclinada 45 graus
        let dir = direcao_de_alinhamento([0., 0.], [10., 10.]).unwrap();
        let p = alinhar_ao_corte([0., 0.], dir, [30., 0.], false);
        assert!((p[0] - 15.).abs() < 1e-9 && (p[1] + 15.).abs() < 1e-9, "{p:?}");
        assert!(direcao_de_alinhamento([1., 1.], [1., 1.]).is_none());
    }
}
