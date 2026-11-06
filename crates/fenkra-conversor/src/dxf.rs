// SPDX-License-Identifier: LGPL-2.1-or-later
//! DXF para sketch (B105). O DXF é 2D e não tem forma para o kernel: as entidades viram curvas de sketch
//! no plano XY, e quem fecha um perfil é o comando que usar o sketch.
//!
//! Cada camada vira um sketch com o nome dela, para a pessoa extrudar o contorno e deixar a cota de fora.
//! Pontas a menos da tolerância viram o mesmo ponto: é o que faz o laço fechar, e arquivos reais trazem
//! contornos com folgas pequenas. Centros ficam de fora dessa junção, senão um arco pequeno perderia o
//! centro para a ponta vizinha.

use crate::importar::{Importado, Previa};
use crate::relatorio::*;
use crate::{ErroConv, Formato, OpcoesImportar, Relatorio};
use fenkra_core::sketch::{Curve, SketchData};
use fenkra_core::{Document, FeatureKind, Id, OriginPlane, PlaneRef, SeloOrigem, Sketch};
use fenkra_dxf::{arco_do_bojo, Desenho, Entidade, Vertice, P2};
use std::collections::{BTreeMap, HashMap};
use std::f64::consts::TAU;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// Pontos por trecho entre nós quando a spline só tem polígono de controle.
const PONTOS_POR_TRECHO: usize = 8;

/// Junta pontos próximos numa grade do tamanho da tolerância.
struct Pontos {
    tol: f64,
    grade: HashMap<(i64, i64), Vec<(Id, P2)>>,
    juntados: usize,
}

impl Pontos {
    fn novo(tol: f64) -> Pontos {
        Pontos { tol: tol.max(1e-9), grade: HashMap::new(), juntados: 0 }
    }

    fn celula(&self, p: P2) -> (i64, i64) {
        ((p[0] / self.tol).floor() as i64, (p[1] / self.tol).floor() as i64)
    }

    fn ponto(&mut self, sk: &mut SketchData, al: &mut impl FnMut() -> Id, p: P2) -> Id {
        let (cx, cy) = self.celula(p);
        for dx in -1..=1 {
            for dy in -1..=1 {
                let achado = self.grade.get(&(cx + dx, cy + dy)).and_then(|l| l.iter().find(|(_, q)| (q[0] - p[0]).hypot(q[1] - p[1]) <= self.tol));
                if let Some((id, q)) = achado {
                    self.juntados += usize::from(*q != p);
                    return *id;
                }
            }
        }
        let id = sk.add_point(al, p[0], p[1]);
        self.grade.entry((cx, cy)).or_default().push((id, p));
        id
    }
}

/// Curva B-spline racional no parâmetro `u`, pelo algoritmo de de Boor.
fn de_boor(grau: usize, controle: &[P2], pesos: &[f64], nos: &[f64], u: f64) -> P2 {
    let n = controle.len();
    let k = (grau..n).rev().find(|i| nos[*i] <= u).unwrap_or(grau);
    let peso = |i: usize| pesos.get(i).copied().unwrap_or(1.);
    let mut d: Vec<[f64; 3]> = (0..=grau).map(|j| k + j - grau).map(|i| [controle[i][0] * peso(i), controle[i][1] * peso(i), peso(i)]).collect();
    for r in 1..=grau {
        for j in (r..=grau).rev() {
            let i = k + j - grau;
            let vao = nos[i + grau - r + 1] - nos[i];
            let a = if vao.abs() < 1e-14 { 0. } else { (u - nos[i]) / vao };
            let (ant, atual) = (d[j - 1], d[j]);
            d[j] = [0, 1, 2].map(|c| (1. - a) * ant[c] + a * atual[c]);
        }
    }
    let q = d[grau];
    [q[0] / q[2], q[1] / q[2]]
}

/// Pontos por onde a curva passa, que é o que a spline do sketch interpola.
fn pontos_da_spline(grau: u32, controle: &[P2], nos: &[f64], pesos: &[f64], ajuste: &[P2]) -> Vec<P2> {
    if ajuste.len() >= 2 {
        return ajuste.to_vec();
    }
    let grau = grau as usize;
    if controle.len() <= grau || nos.len() != controle.len() + grau + 1 {
        return controle.to_vec();
    }
    let (a, b) = (nos[grau], nos[controle.len()]);
    let n = (controle.len() - grau) * PONTOS_POR_TRECHO;
    (0..=n).map(|i| a + (b - a) * i as f64 / n as f64).map(|u| de_boor(grau, controle, pesos, nos, u.min(b - 1e-12 * (b - a).abs()))).collect()
}

fn ponta_do_arco(c: P2, r: f64, graus: f64) -> P2 {
    let (s, k) = graus.to_radians().sin_cos();
    [c[0] + r * k, c[1] + r * s]
}

fn na_elipse(c: P2, maior: P2, razao: f64, t: f64) -> P2 {
    let menor = [-maior[1] * razao, maior[0] * razao];
    [c[0] + maior[0] * t.cos() + menor[0] * t.sin(), c[1] + maior[1] * t.cos() + menor[1] * t.sin()]
}

fn arco(sk: &mut SketchData, al: &mut impl FnMut() -> Id, pts: &mut Pontos, centro: P2, s: P2, e: P2) {
    let c = sk.add_point(al, centro[0], centro[1]);
    let (s, e) = (pts.ponto(sk, al, s), pts.ponto(sk, al, e));
    if s != e {
        sk.add_curve(al, Curve::Arc { c, s, e }, false);
    }
}

fn polilinha(sk: &mut SketchData, al: &mut impl FnMut() -> Id, pts: &mut Pontos, v: &[Vertice], fechada: bool) {
    let n = v.len();
    let trechos = if fechada { n } else { n.saturating_sub(1) };
    for i in 0..trechos {
        let (a, b) = (v[i], v[(i + 1) % n]);
        match arco_do_bojo(a.p, b.p, a.bojo) {
            Some((c, _, true)) => arco(sk, al, pts, c, a.p, b.p),
            Some((c, _, false)) => arco(sk, al, pts, c, b.p, a.p),
            None => {
                let (p, q) = (pts.ponto(sk, al, a.p), pts.ponto(sk, al, b.p));
                if p != q {
                    sk.add_curve(al, Curve::Line { a: p, b: q }, false);
                }
            }
        }
    }
}

/// Põe a entidade no sketch, já em mm. Devolve `false` para o que o sketch não representa.
fn no_sketch(sk: &mut SketchData, al: &mut impl FnMut() -> Id, pts: &mut Pontos, e: &Entidade) -> bool {
    match e {
        Entidade::Linha(a, b) => {
            let (p, q) = (pts.ponto(sk, al, *a), pts.ponto(sk, al, *b));
            if p != q {
                sk.add_curve(al, Curve::Line { a: p, b: q }, false);
            }
        }
        Entidade::Circulo { centro, raio } => {
            let c = sk.add_point(al, centro[0], centro[1]);
            sk.add_curve(al, Curve::Circle { c, r: *raio }, false);
        }
        Entidade::Arco { centro, raio, ini, fim } => arco(sk, al, pts, *centro, ponta_do_arco(*centro, *raio, *ini), ponta_do_arco(*centro, *raio, *fim)),
        Entidade::Polilinha { vertices, fechada } => polilinha(sk, al, pts, vertices, *fechada),
        Entidade::Ponto(p) => {
            let p = pts.ponto(sk, al, *p);
            sk.add_curve(al, Curve::Point { p }, false);
        }
        Entidade::Elipse { centro, maior, razao, t0, t1 } => {
            let (t0, t1) = (*t0, if t1 > t0 { *t1 } else { *t1 + TAU });
            let c = sk.add_point(al, centro[0], centro[1]);
            let (s, f) = if t1 - t0 >= TAU - 1e-9 { (c, c) } else { (pts.ponto(sk, al, na_elipse(*centro, *maior, *razao, t0)), pts.ponto(sk, al, na_elipse(*centro, *maior, *razao, t1))) };
            sk.add_curve(al, Curve::Ellipse { c, s, e: f, major: *maior, minor: maior[0].hypot(maior[1]) * razao, t0, t1 }, false);
        }
        Entidade::Spline { grau, controle, nos, pesos, ajuste, fechada } => {
            let lista = pontos_da_spline(*grau, controle, nos, pesos, ajuste);
            let mut ids: Vec<Id> = lista.iter().map(|p| pts.ponto(sk, al, *p)).collect();
            ids.dedup();
            if *fechada && ids.first() != ids.last() {
                ids.push(ids[0]);
            }
            if ids.len() >= 2 {
                sk.add_curve(al, Curve::Spline { pts: ids }, false);
            }
        }
        Entidade::Texto { .. } | Entidade::Hachura { .. } => return false,
    }
    true
}

/// A entidade em mm. Arcos e bojos não mudam com escala uniforme.
fn escalada(e: &Entidade, k: f64) -> Entidade {
    let p = |q: &P2| [q[0] * k, q[1] * k];
    match e {
        Entidade::Linha(a, b) => Entidade::Linha(p(a), p(b)),
        Entidade::Circulo { centro, raio } => Entidade::Circulo { centro: p(centro), raio: raio * k },
        Entidade::Arco { centro, raio, ini, fim } => Entidade::Arco { centro: p(centro), raio: raio * k, ini: *ini, fim: *fim },
        Entidade::Polilinha { vertices, fechada } => Entidade::Polilinha { vertices: vertices.iter().map(|v| Vertice { p: p(&v.p), bojo: v.bojo }).collect(), fechada: *fechada },
        Entidade::Elipse { centro, maior, razao, t0, t1 } => Entidade::Elipse { centro: p(centro), maior: p(maior), razao: *razao, t0: *t0, t1: *t1 },
        Entidade::Spline { grau, controle, nos, pesos, ajuste, fechada } => {
            Entidade::Spline { grau: *grau, controle: controle.iter().map(p).collect(), nos: nos.clone(), pesos: pesos.clone(), ajuste: ajuste.iter().map(p).collect(), fechada: *fechada }
        }
        Entidade::Ponto(q) => Entidade::Ponto(p(q)),
        outra => outra.clone(),
    }
}

/// Segmentos de uma curva do sketch, para a miniatura e a caixa.
fn tracos(sk: &SketchData, saida: &mut Vec<[P2; 2]>) {
    for c in &sk.curves {
        let a = |i: Id| sk.pos(i);
        let mut linha = |pts: Vec<P2>| saida.extend(pts.windows(2).map(|w| [w[0], w[1]]));
        match &c.geom {
            Curve::Line { a: p, b: q } => linha(vec![a(*p), a(*q)]),
            Curve::Circle { c, r } => {
                let o = a(*c);
                linha((0..=48).map(|i| i as f64 / 48. * TAU).map(|t| [o[0] + r * t.cos(), o[1] + r * t.sin()]).collect());
            }
            Curve::Arc { c, s, e } => {
                let (o, s, e) = (a(*c), a(*s), a(*e));
                let r = (s[0] - o[0]).hypot(s[1] - o[1]);
                let t0 = (s[1] - o[1]).atan2(s[0] - o[0]);
                let varredura = ((e[1] - o[1]).atan2(e[0] - o[0]) - t0).rem_euclid(TAU);
                linha((0..=24).map(|i| t0 + varredura * i as f64 / 24.).map(|t| [o[0] + r * t.cos(), o[1] + r * t.sin()]).collect());
            }
            Curve::Spline { pts } => linha(pts.iter().map(|p| a(*p)).collect()),
            Curve::Ellipse { c, major, minor, t0, t1, .. } => {
                let o = a(*c);
                let razao = minor / major[0].hypot(major[1]).max(1e-12);
                linha((0..=48).map(|i| t0 + (t1 - t0) * i as f64 / 48.).map(|t| na_elipse(o, *major, razao, t)).collect());
            }
            Curve::Point { .. } => {}
        }
    }
}

fn previa_de(segmentos: &[[P2; 2]]) -> (Previa, Option<([f64; 3], [f64; 3])>) {
    let mut v = Previa::default();
    let pontos = || segmentos.iter().flatten();
    if segmentos.is_empty() {
        return (v, None);
    }
    let lo = pontos().fold([f64::MAX; 2], |a, p| [a[0].min(p[0]), a[1].min(p[1])]);
    let hi = pontos().fold([f64::MIN; 2], |a, p| [a[0].max(p[0]), a[1].max(p[1])]);
    let centro = [(lo[0] + hi[0]) / 2., (lo[1] + hi[1]) / 2.];
    let ponto = |p: &P2| [(p[0] - centro[0]) as f32, (p[1] - centro[1]) as f32, 0.];
    v.arestas = segmentos.iter().map(|s| [ponto(&s[0]), ponto(&s[1])]).collect();
    (v, Some(([lo[0], lo[1], 0.], [hi[0], hi[1], 0.])))
}

/// Lê o DXF e monta um documento com os sketches, com selo, relatório e miniatura.
pub(crate) fn importar(caminho: &Path, bytes: &[u8], o: &OpcoesImportar, extensao_diverge: bool) -> Result<Importado, ErroConv> {
    let d: Desenho = fenkra_dxf::ler(bytes).map_err(ErroConv::Leitura)?;
    let nome_arquivo = caminho.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let nome_doc = caminho.file_stem().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "Importado".into());
    let hash = blake3::hash(bytes).to_hex().to_string();
    let esquema = if d.versao.is_empty() { "DXF".to_string() } else { format!("DXF {}", d.versao) };
    let unidades = d.unidade.nome().to_string();
    let escala = o.unidade.em_mm().or(d.unidade.em_mm()).unwrap_or(1.);
    let mut r = Relatorio {
        arquivo: nome_arquivo.clone(),
        hash: hash.clone(),
        tamanho: bytes.len() as u64,
        formato: Formato::Dxf.nome().into(),
        provedor: "Fenkra".into(),
        versao: format!("Fenkra {}", env!("CARGO_PKG_VERSION")),
        esquema: esquema.clone(),
        unidades: unidades.clone(),
        escala,
        reparo: crate::Reparo::Nenhum.nome().into(),
        ..Default::default()
    };
    if extensao_diverge {
        r.avisos.push(Nota::nova(AV_EXTENSAO, &[Formato::Dxf.nome().to_string()]));
    }
    if d.unidade.em_mm().is_none() && o.unidade.em_mm().is_none() {
        r.avisos.push(Nota::nova(AV_DXF_SEM_UNIDADE, &[]));
    }

    // por camada, na ordem em que aparecem; sem separar, tudo na primeira
    let mut ordem: Vec<&str> = Vec::new();
    let mut por_camada: BTreeMap<&str, Vec<&Entidade>> = BTreeMap::new();
    for item in &d.itens {
        let chave = if o.camadas { item.camada.as_str() } else { "" };
        if !por_camada.contains_key(chave) {
            ordem.push(chave);
        }
        por_camada.entry(chave).or_default().push(&item.entidade);
    }
    let mut doc = Document::new(&nome_doc);
    let mut fora: BTreeMap<String, usize> = d.ignorados.clone();
    let (mut segmentos, mut juntados) = (Vec::new(), 0);
    for camada in ordem {
        let mut sk = SketchData::default();
        let mut pts = Pontos::novo(o.tolerancia);
        let mut curvas = 0;
        {
            let mut al = || doc.alloc();
            for e in &por_camada[camada] {
                if no_sketch(&mut sk, &mut al, &mut pts, &escalada(e, escala)) {
                    curvas += 1;
                } else {
                    let tipo = if matches!(e, Entidade::Texto { .. }) { "TEXT/MTEXT" } else { "HATCH" };
                    *fora.entry(tipo.to_string()).or_insert(0) += 1;
                }
            }
        }
        juntados += pts.juntados;
        if sk.curves.is_empty() {
            continue;
        }
        tracos(&sk, &mut segmentos);
        let id = doc.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data: sk }));
        if !camada.is_empty() {
            if let Some(f) = doc.features.iter_mut().find(|f| f.id == id) {
                f.name = camada.to_string();
            }
        }
        r.camadas.push((if camada.is_empty() { "*".to_string() } else { camada.to_string() }, curvas));
    }
    if r.camadas.is_empty() {
        return Err(ErroConv::Leitura(format!("o DXF '{nome_arquivo}' não tem curvas para um sketch")));
    }
    if juntados > 0 {
        r.avisos.push(Nota::nova(AV_DXF_JUNTADOS, &[juntados.to_string(), format!("{}", o.tolerancia)]));
    }
    if !fora.is_empty() {
        let lista: Vec<String> = fora.iter().map(|(tipo, n)| format!("{tipo} ({n})")).collect();
        r.nao_carrega.push(Nota::nova(FORA_DXF, &[lista.join(", ")]));
    }
    let (previa, caixa) = previa_de(&segmentos);
    r.caixa = caixa;
    let selo = SeloOrigem {
        nome: nome_arquivo,
        hash,
        data: SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0),
        esquema,
        unidades,
        derivado_de: None,
        relatorio: serde_json::to_string(&r).ok(),
    };
    doc.origem = Some(selo);
    Ok(Importado { doc, relatorio: r, previa, conjunto: None })
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn de_boor_passa_pelas_pontas_e_acerta_a_curva_de_bezier() {
        // grau 2 com nós fechados é uma curva de Bézier: em u = 0,5 vale (P0 + 2 P1 + P2) / 4
        let c = [[0., 0.], [1., 2.], [2., 0.]];
        let nos = [0., 0., 0., 1., 1., 1.];
        let p = pontos_da_spline(2, &c, &nos, &[], &[]);
        assert_eq!(p[0], [0., 0.]);
        assert!((p[p.len() - 1][0] - 2.).abs() < 1e-6);
        let meio = p[p.len() / 2];
        assert!((meio[0] - 1.).abs() < 1e-9 && (meio[1] - 1.).abs() < 1e-9, "{meio:?}");
    }

    #[test]
    fn arco_racional_com_peso_cai_no_circulo() {
        // quarto de círculo de raio 1 como Bézier racional de grau 2
        let c = [[1., 0.], [1., 1.], [0., 1.]];
        let w = [1., std::f64::consts::FRAC_1_SQRT_2, 1.];
        let p = pontos_da_spline(2, &c, &[0., 0., 0., 1., 1., 1.], &w, &[]);
        assert!(p.iter().all(|q| (q[0].hypot(q[1]) - 1.).abs() < 1e-6), "{p:?}");
    }

    #[test]
    fn pontos_a_menos_da_tolerancia_viram_o_mesmo() {
        let mut d = Document::new("x");
        let mut sk = SketchData::default();
        let mut al = || d.alloc();
        let mut p = Pontos::novo(0.01);
        let a = p.ponto(&mut sk, &mut al, [0., 0.]);
        assert_eq!(p.ponto(&mut sk, &mut al, [0.005, 0.]), a);
        assert_ne!(p.ponto(&mut sk, &mut al, [0.02, 0.]), a);
        assert_eq!((p.juntados, sk.points.len()), (1, 2));
    }
}
