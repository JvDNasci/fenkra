// SPDX-License-Identifier: LGPL-2.1-or-later
//! Conversão da forma livre em B-rep (B179).
//!
//! `subd_brep` separa a superfície em remendos exatos e regiões em volta dos nós irregulares. Aqui os
//! remendos viram faces e cada região é fechada conforme a continuidade pedida:
//! - **G0**: um remendo bicúbico por quadrilátero, com as arestas compartilhadas; a tangente quebra de
//!   leve nas arestas que tocam o nó.
//! - **G1 e G2**: a região inteira vira uma só face, um preenchimento com a condição de tangência ou de
//!   curvatura nas faces vizinhas e passando pelas amostras do limite. Assim o nó irregular fica no
//!   meio de uma face lisa e não há nenhuma aresta chegando nele: é o que evita o problema clássico de
//!   fechar remendos em volta de um nó de valência diferente de 4.
//!
//! Região que o preenchimento recusa cai para G0 e o relatório diz qual foi e por quê: a conversão
//! entrega o que conseguiu, em vez de falhar inteira.

use fenkra_core::subd::{Gaiola, V3};
use fenkra_core::subd_brep::{decompor, Decomposicao, Regiao, Remendo};
use fenkra_kernel::Shape;

/// O que aconteceu com uma região.
#[derive(Clone, Debug, PartialEq)]
pub struct RegiaoRelatorio {
    pub no: Option<u32>,
    pub ponto: V3,
    pub valencia: usize,
    /// continuidade entregue (pode ser menor que a pedida)
    pub continuidade: u8,
    /// maior distância entre a face e a superfície de subdivisão (mm)
    pub desvio: f64,
    /// erro medido pelo preenchimento na junção: folga (mm) e ângulo (rad)
    pub erro_g0: f64,
    pub erro_g1: f64,
    /// diferença de curvatura na junção, medida pelo preenchimento quando G2 foi pedido
    pub erro_g2: f64,
    /// por que não saiu na continuidade pedida
    pub motivo: Option<String>,
}

/// Relatório de uma conversão, para o 3D e para o painel.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RelatorioForma {
    pub pedida: u8,
    pub remendos: usize,
    pub exatos: usize,
    pub regioes: Vec<RegiaoRelatorio>,
    pub desvio_max: f64,
    /// cada ponto medido e o desvio dele: é o mapa de desvio
    pub amostras: Vec<(V3, f64)>,
    pub solido: bool,
    pub faces: u32,
    /// números de nome das faces, para avisar quando mudam
    pub nomes: Vec<i32>,
    pub tempo_ms: f64,
}

/// Nome da face do remendo: o número da face da gaiola com o pedaço dentro dela. O reflexo da simetria
/// tem nome próprio, e o nome é sempre positivo porque negativo quer dizer "sem papel".
pub fn nome_do_remendo(r: &Remendo) -> i32 {
    let espelho = (r.face & fenkra_core::subd::ESPELHO != 0) as i32;
    (espelho << 29) | (((r.face & 0xF_FFFF) as i32) << 8) | (r.pedaco & 0xFF) as i32
}

/// Nome da face de uma região: o nó irregular que ela cobre.
fn nome_da_regiao(r: &Regiao, k: usize) -> i32 {
    let no = r.no.map_or(0xF_FFFF - k as u32, |n| n & 0xF_FFFF);
    (1 << 30) | no as i32
}

fn polos_planos(lista: &[&Remendo]) -> Vec<f64> {
    let mut v = Vec::with_capacity(lista.len() * 48);
    for r in lista {
        for linha in &r.polos {
            for p in linha {
                v.extend_from_slice(p);
            }
        }
    }
    v
}

/// No máximo `n` pontos, espalhados pela lista: passar muitos deixa o preenchimento lento e rígido.
fn poucos(pts: &[V3], n: usize) -> Vec<V3> {
    let passo = (pts.len() / n.max(1)).max(1);
    pts.iter().step_by(passo).copied().collect()
}

fn dist(a: V3, b: V3) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// Distância do ponto ao triângulo (ponto mais perto por regiões de Voronoi).
fn dist_tri(p: V3, a: V3, b: V3, c: V3) -> f64 {
    let s = |x: V3, y: V3| [x[0] - y[0], x[1] - y[1], x[2] - y[2]];
    let d = |x: V3, y: V3| x[0] * y[0] + x[1] * y[1] + x[2] * y[2];
    let (ab, ac, ap) = (s(b, a), s(c, a), s(p, a));
    let (d1, d2) = (d(ab, ap), d(ac, ap));
    if d1 <= 0. && d2 <= 0. {
        return dist(p, a);
    }
    let bp = s(p, b);
    let (d3, d4) = (d(ab, bp), d(ac, bp));
    if d3 >= 0. && d4 <= d3 {
        return dist(p, b);
    }
    let em = |o: V3, v: V3, t: f64| [o[0] + v[0] * t, o[1] + v[1] * t, o[2] + v[2] * t];
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0. && d1 >= 0. && d3 <= 0. {
        return dist(p, em(a, ab, d1 / (d1 - d3)));
    }
    let cp = s(p, c);
    let (d5, d6) = (d(ab, cp), d(ac, cp));
    if d6 >= 0. && d5 <= d6 {
        return dist(p, c);
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0. && d2 >= 0. && d6 <= 0. {
        return dist(p, em(a, ac, d2 / (d2 - d6)));
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0. && d4 - d3 >= 0. && d5 - d6 >= 0. {
        return dist(p, em(b, s(c, b), (d4 - d3) / (d4 - d3 + d5 - d6)));
    }
    let den = 1. / (va + vb + vc);
    dist(p, em(em(a, ab, vb * den), ac, vc * den))
}

fn dist_malha(m: &fenkra_kernel::Mesh, p: V3) -> f64 {
    let v = |i: u32| [m.positions[3 * i as usize] as f64, m.positions[3 * i as usize + 1] as f64, m.positions[3 * i as usize + 2] as f64];
    m.indices.chunks_exact(3).map(|t| dist_tri(p, v(t[0]), v(t[1]), v(t[2]))).fold(f64::MAX, f64::min)
}

/// Preenche uma região e mede. `apoio` é o que já está costurado em volta.
fn preencher(r: &Regiao, apoio: &Shape, ordem: u8, flecha: f64) -> Result<(Shape, [f64; 3], f64, Vec<(V3, f64)>), String> {
    let contorno = r.contorno.as_ref().ok_or("a região não é um disco: um vinco termina dentro dela ou ela tem mais de um contorno")?;
    let curvas: Vec<Vec<V3>> = contorno.iter().map(|t| t.pts.clone()).collect();
    let ordens: Vec<i32> = contorno.iter().map(|t| if t.vizinho_liso { ordem as i32 } else { 0 }).collect();
    let pesos = vec![1.; curvas.len()];
    let mut pontos = poucos(&r.amostras, 24);
    pontos.push(r.ponto);
    let (face, g0, g1, g2) = Shape::fill_surface_x(&curvas, &ordens, &pesos, apoio, &pontos).map_err(|e| e.0)?;
    // mede contra a malha fina da face: a projeção exata do kernel falha justamente quando o ponto já
    // está sobre a superfície e devolve a distância até a borda
    let malha = face.mesh(flecha).map_err(|e| e.0)?;
    if malha.indices.is_empty() {
        return Err("o preenchimento saiu sem interior".into());
    }
    let mut medidas = Vec::with_capacity(r.amostras.len());
    let mut desvio: f64 = 0.;
    for p in r.amostras.iter().chain(std::iter::once(&r.ponto)) {
        let d = dist_malha(&malha, *p);
        desvio = desvio.max(d);
        medidas.push((*p, d));
    }
    Ok((face, [g0, g1, g2], desvio, medidas))
}

/// Faces de cada coisa, na ordem do corpo: para cada face do resultado, o nome do remendo mais perto.
fn nomes_das_faces(s: &Shape, d: &Decomposicao, centros: &[(V3, i32)]) -> Result<Vec<i32>, String> {
    let m = s.mesh(d.escala * 0.02).map_err(|e| e.0)?;
    let mut out = Vec::with_capacity(m.face_ranges.len());
    for (i, ini) in m.face_ranges.iter().enumerate() {
        let fim = m.face_ranges.get(i + 1).copied().unwrap_or(m.indices.len() as u32) as usize;
        let tris = &m.indices[*ini as usize..fim];
        let mut c = [0.; 3];
        for &v in tris {
            for k in 0..3 {
                c[k] += m.positions[3 * v as usize + k] as f64;
            }
        }
        let n = tris.len().max(1) as f64;
        let c = c.map(|x| x / n);
        let nome = centros.iter().min_by(|a, b| dist(a.0, c).total_cmp(&dist(b.0, c))).map_or(-1, |x| x.1);
        out.push(nome);
    }
    Ok(out)
}

/// Converte a gaiola. Devolve a forma, o nome de cada face e o relatório.
pub fn converter(g: &Gaiola, continuidade: u8, tol: f64) -> Result<(Shape, Vec<i32>, RelatorioForma), String> {
    let t0 = std::time::Instant::now();
    let d = decompor(g)?;
    let tol = tol.max(1e-5);
    let mut rel = RelatorioForma { pedida: continuidade, remendos: d.remendos.len(), exatos: d.exatos(), ..Default::default() };
    // quem vai por preenchimento sai da lista de remendos
    let preenche: Vec<bool> = d.regioes.iter().map(|r| continuidade > 0 && r.contorno.is_some()).collect();
    let mut fora = vec![false; d.remendos.len()];
    for (r, p) in d.regioes.iter().zip(&preenche) {
        if *p {
            r.quads.iter().for_each(|q| fora[*q] = true);
        }
    }
    let base: Vec<&Remendo> = d.remendos.iter().zip(&fora).filter(|(_, f)| !**f).map(|(r, _)| r).collect();
    let apoio = Shape::bezier_faces(&polos_planos(&base)).map_err(|e| e.0)?;
    let apoio = apoio.sewn(tol).map_err(|e| format!("não deu para costurar os remendos: {}", e.0))?;
    let mut tudo = apoio.clone();
    let mut centros: Vec<(V3, i32)> = base.iter().map(|r| (r.centro, nome_do_remendo(r))).collect();
    for (k, r) in d.regioes.iter().enumerate() {
        let mut item = RegiaoRelatorio { no: r.no, ponto: r.ponto, valencia: r.valencia, continuidade: 0, desvio: 0., erro_g0: 0., erro_g1: 0., erro_g2: 0., motivo: None };
        let mut cai_para_g0 = !preenche[k];
        if continuidade > 0 && r.contorno.is_none() {
            item.motivo = Some("a região não é um disco: um vinco termina dentro dela".into());
        }
        if preenche[k] {
            match preencher(r, &apoio, continuidade, d.escala * 1e-4) {
                Ok((face, [g0, g1, g2], desvio, medidas)) => {
                    tudo = tudo.compound(&face);
                    centros.push((r.ponto, nome_da_regiao(r, k)));
                    item.continuidade = continuidade;
                    item.desvio = desvio;
                    item.erro_g0 = g0;
                    item.erro_g1 = g1;
                    item.erro_g2 = g2;
                    rel.amostras.extend(medidas);
                }
                Err(e) => {
                    item.motivo = Some(e);
                    cai_para_g0 = true;
                }
            }
        }
        if cai_para_g0 {
            let lista: Vec<&Remendo> = r.quads.iter().map(|q| &d.remendos[*q]).collect();
            if preenche[k] {
                tudo = tudo.compound(&Shape::bezier_faces(&polos_planos(&lista)).map_err(|e| e.0)?);
                centros.extend(lista.iter().map(|x| (x.centro, nome_do_remendo(x))));
            }
            item.desvio = lista.iter().map(|x| x.residuo).fold(0., f64::max);
            rel.amostras.extend(r.amostras.iter().map(|p| (*p, item.desvio)));
        }
        rel.desvio_max = rel.desvio_max.max(item.desvio);
        rel.regioes.push(item);
    }
    let casca = tudo.sewn(tol.max(1e-3)).map_err(|e| format!("não deu para costurar: {}", e.0))?;
    let mut s = casca.clone();
    if d.fechada {
        s = casca.closed_solid(tol.max(1e-3)).map_err(|e| {
            let onde = casca.sew_report().map(|r| format!("{} arestas livres, maior folga {:.4} mm", r.free_edges, r.open_gap.max(r.closed_gap))).unwrap_or_default();
            let pior = rel.regioes.iter().max_by(|a, b| a.erro_g0.total_cmp(&b.erro_g0)).map(|r| format!("; olhe o nó perto de ({:.1}, {:.1}, {:.1})", r.ponto[0], r.ponto[1], r.ponto[2])).unwrap_or_default();
            format!("a casca não fechou em sólido ({}): {onde}{pior}", e.0)
        })?;
    }
    rel.solido = s.is_solid();
    rel.faces = s.face_count();
    let nomes = nomes_das_faces(&s, &d, &centros)?;
    rel.nomes = nomes.clone();
    rel.nomes.sort_unstable();
    rel.nomes.dedup();
    rel.tempo_ms = t0.elapsed().as_secs_f64() * 1000.;
    Ok((s, nomes, rel))
}

/// Texto curto do resultado, para o status do comando.
pub fn resumo(r: &RelatorioForma) -> Option<String> {
    let caiu: Vec<&RegiaoRelatorio> = r.regioes.iter().filter(|x| x.continuidade < r.pedida).collect();
    if caiu.is_empty() {
        return None;
    }
    let exemplo = caiu[0].motivo.clone().unwrap_or_default();
    Some(format!("{} de {} regiões ficaram em G0: {exemplo}", caiu.len(), r.regioes.len()))
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::{recompute, Status};
    use fenkra_core::subd::Gaiola;
    use fenkra_core::*;

    fn doc_com(g: Gaiola, continuidade: u8) -> (Document, Id) {
        let mut d = Document::new("forma");
        let id = d.add_feature(FeatureKind::Forma(Forma { gaiola: g, niveis: 3, congelada: false, conversao: Some(ConversaoForma { continuidade, tol: "0.01".into(), pausada: false }) }));
        (d, id)
    }

    fn caixa_lisa() -> Gaiola {
        Gaiola::caixa([0.; 3], [100.; 3], [2, 2, 2])
    }

    fn confere(d: &Document, id: Id, g: &Gaiola, tol_desvio: f64) -> crate::RecomputeResult {
        let r = recompute(d);
        let st = r.status.get(&id).expect("status");
        assert!(matches!(st, Status::Ok | Status::Warning(_)), "{st:?}");
        assert_eq!(r.bodies.len(), 1, "corpo: {st:?}");
        let rel = &r.relatorios_forma[0].1;
        assert!(rel.solido, "não fechou");
        assert!(rel.desvio_max <= tol_desvio, "desvio {} regioes {:?}", rel.desvio_max, rel.regioes.iter().map(|x| (x.valencia, x.continuidade, x.desvio, x.erro_g0, x.erro_g1)).collect::<Vec<_>>());
        // volume e caixa batem com a superfície de subdivisão
        let m = g.lisa(5).unwrap();
        let b = &r.bodies[0];
        let (v, vm) = (b.mass.volume, m.volume());
        assert!((v - vm).abs() / vm < 0.01, "volume {v} contra {vm}");
        let (lo, hi) = m.caixa().unwrap();
        let p = &b.mesh.positions;
        let maior = p.chunks_exact(3).fold(f64::MIN, |a, q| a.max(q[2] as f64));
        let menor = p.chunks_exact(3).fold(f64::MAX, |a, q| a.min(q[2] as f64));
        assert!((maior - hi[2]).abs() < 1. && (menor - lo[2]).abs() < 1., "z {menor}..{maior} contra {}..{}", lo[2], hi[2]);
        eprintln!("G{}: {:.0} ms, {} faces, desvio {:.4}, regioes {:?}", rel.pedida, rel.tempo_ms, rel.faces, rel.desvio_max, rel.regioes.iter().map(|x| (x.continuidade, (x.erro_g0 * 1e4).round() / 1e4, (x.erro_g1 * 1e4).round() / 1e4)).take(3).collect::<Vec<_>>());
        r
    }

    #[test]
    fn caixa_em_g0_vira_solido() {
        let g = caixa_lisa();
        let (d, id) = doc_com(g.clone(), 0);
        let r = confere(&d, id, &g, 0.5);
        assert_eq!(r.relatorios_forma[0].1.faces, 384);
    }

    #[test]
    fn caixa_em_g1_vira_solido_com_uma_face_por_no_irregular() {
        let g = caixa_lisa();
        let (d, id) = doc_com(g.clone(), 1);
        let r = confere(&d, id, &g, 0.05);
        let rel = &r.relatorios_forma[0].1;
        assert_eq!(rel.regioes.len(), 8);
        assert!(rel.regioes.iter().all(|x| x.continuidade == 1), "{:?}", rel.regioes);
        assert_eq!(rel.faces, 384 - 24 + 8);
        assert!(rel.regioes.iter().all(|x| x.erro_g1 < 0.02), "{:?}", rel.regioes);
    }

    #[test]
    fn caixa_em_g2_vira_solido() {
        let g = caixa_lisa();
        let (d, id) = doc_com(g.clone(), 2);
        let r = confere(&d, id, &g, 0.1);
        assert!(r.relatorios_forma[0].1.regioes.iter().all(|x| x.continuidade == 2), "{:?}", r.relatorios_forma[0].1.regioes[0]);
    }

    /// Maior ângulo (graus) entre as normais de duas faces num mesmo ponto das arestas lisas do corpo.
    fn pior_quebra(m: &fenkra_kernel::Mesh) -> f64 {
        let mut por_ponto: std::collections::HashMap<[i64; 3], Vec<(usize, [f32; 3])>> = Default::default();
        for f in 0..m.face_ranges.len() {
            let (ini, fim) = (m.face_ranges[f] as usize, m.face_ranges.get(f + 1).map_or(m.indices.len(), |x| *x as usize));
            for &i in &m.indices[ini..fim] {
                let i = i as usize;
                let chave = [0, 1, 2].map(|k| (m.positions[3 * i + k] as f64 * 200.).round() as i64);
                por_ponto.entry(chave).or_default().push((f, [m.normals[3 * i], m.normals[3 * i + 1], m.normals[3 * i + 2]]));
            }
        }
        let mut pior: f64 = 0.;
        for v in por_ponto.values() {
            for a in v {
                for b in v.iter().filter(|b| b.0 != a.0) {
                    let c = (a.1[0] * b.1[0] + a.1[1] * b.1[1] + a.1[2] * b.1[2]) as f64;
                    pior = pior.max(c.clamp(-1., 1.).acos().to_degrees());
                }
            }
        }
        pior
    }

    #[test]
    fn g1_nao_quebra_a_tangente_nas_juntas_e_g0_quebra_um_pouco() {
        let g = caixa_lisa();
        let angulo = |k: u8| {
            let (d, _) = doc_com(g.clone(), k);
            pior_quebra(&recompute(&d).bodies[0].mesh)
        };
        let (g0, g1) = (angulo(0), angulo(1));
        eprintln!("quebra G0 {g0:.3} graus, G1 {g1:.3} graus");
        assert!(g1 < 0.5, "G1 quebrou {g1} graus");
        assert!(g0 > g1, "G0 {g0} contra G1 {g1}");
    }

    #[test]
    fn esfera_de_quadrilateros_fica_dentro_da_tolerancia() {
        let g = Gaiola::esfera([0.; 3], 50., 2);
        let (d, id) = doc_com(g.clone(), 1);
        confere(&d, id, &g, 0.05);
    }

    #[test]
    fn malha_regular_nao_tem_desvio() {
        let mut g = Gaiola::default();
        let (n, m) = (8u32, 6u32);
        let mut ids = vec![];
        for j in 0..m {
            for i in 0..n {
                let (a, b) = (std::f64::consts::TAU * i as f64 / n as f64, std::f64::consts::TAU * j as f64 / m as f64);
                let r = 40. + 10. * b.cos();
                ids.push(g.novo_para_teste([r * a.cos(), r * a.sin(), 10. * b.sin()]));
            }
        }
        let at = |i: u32, j: u32| ids[((j % m) * n + i % n) as usize];
        for j in 0..m {
            for i in 0..n {
                g.face_para_teste(vec![at(i, j), at(i + 1, j), at(i + 1, j + 1), at(i, j + 1)]);
            }
        }
        let (d, id) = doc_com(g.clone(), 1);
        let r = confere(&d, id, &g, 1e-9);
        assert_eq!(r.relatorios_forma[0].1.exatos, r.relatorios_forma[0].1.remendos);
    }

    #[test]
    fn valencias_tres_cinco_e_oito() {
        for lados in [5u32, 8] {
            let g = Gaiola::cilindro([0.; 3], 40., 60., lados, 2);
            let (d, id) = doc_com(g.clone(), 1);
            let r = confere(&d, id, &g, 0.1);
            assert!(r.relatorios_forma[0].1.regioes.iter().all(|x| x.continuidade == 1), "{:?}", r.relatorios_forma[0].1.regioes);
            assert!(r.relatorios_forma[0].1.regioes.iter().any(|x| x.valencia == lados as usize));
        }
    }

    #[test]
    fn folha_aberta_vira_superficie() {
        let mut g = Gaiola::plano([0.; 3], [100., 100.], [4, 4]);
        for n in &mut g.nos {
            n.p[2] = (n.p[0] * 0.05).sin() * 8.;
        }
        let (d, id) = doc_com(g, 1);
        let r = recompute(&d);
        assert!(matches!(r.status.get(&id), Some(Status::Ok)), "{:?}", r.status.get(&id));
        assert!(!r.relatorios_forma[0].1.solido);
        assert_eq!(r.bodies[0].mesh.face_ranges.len(), 64 * 4);
    }

    #[test]
    fn vinco_cheio_da_aresta_viva_no_solido() {
        let mut g = caixa_lisa();
        for (a, b) in g.arestas() {
            if g.pos(a).unwrap()[2] > 49. && g.pos(b).unwrap()[2] > 49. {
                g.definir_vinco(a, b, 1.);
            }
        }
        let (d, id) = doc_com(g.clone(), 1);
        confere(&d, id, &g, 0.1);
    }

    #[test]
    fn solido_aceita_filete_casca_e_booleana() {
        let g = caixa_lisa();
        for continuidade in [0u8, 1] {
            // booleana: corta com uma caixa
            let (mut d, id) = doc_com(g.clone(), continuidade);
            let alvo = d.feature(id).unwrap().target;
            let antes = recompute(&d).bodies[0].mass.volume;
            let s = |x: &str| x.to_string();
            let sh = d.add_feature(FeatureKind::Shell(serde_json::from_value(serde_json::json!({"faces": [], "thickness": "2", "outward": false, "closed": true})).expect("casca")));
            let r = recompute(&d);
            assert!(matches!(r.status.get(&sh), Some(Status::Ok)), "casca G{continuidade}: {:?}", r.status.get(&sh));
            assert!(r.bodies[0].mass.volume < antes * 0.5, "casca não esvaziou");
            d.features.retain(|f| f.id != sh);
            let c = d.add_feature(FeatureKind::Primitive(Primitive { kind: PrimKind::Caixa, plane: PlaneRef::Origin(OriginPlane::XY), a: s("40"), b: s("40"), c: s("200"), d: s("0"), op: BoolOp::Cut }));
            let r = recompute(&d);
            assert!(matches!(r.status.get(&c), Some(Status::Ok)), "booleana G{continuidade}: {:?}", r.status.get(&c));
            assert!(r.bodies[0].mass.volume < antes, "o corte não tirou nada");
            d.features.retain(|f| f.id != c);
            // filete numa aresta da saída que seja viva
            let mut gv = g.clone();
            for (a, b) in gv.arestas() {
                if gv.pos(a).unwrap()[2] > 49. && gv.pos(b).unwrap()[2] > 49. {
                    gv.definir_vinco(a, b, 1.);
                }
            }
            let (mut d, _) = doc_com(gv, continuidade);
            let r0 = recompute(&d);
            let b = &r0.bodies[0];
            let viva = (0..b.mesh.edge_smooth.len()).find(|e| b.mesh.edge_smooth[*e] == 0).expect("aresta viva");
            let m = [b.mesh.edge_mids[3 * viva] as f64, b.mesh.edge_mids[3 * viva + 1] as f64, b.mesh.edge_mids[3 * viva + 2] as f64];
            let fl = d.add_feature(FeatureKind::Fillet(EdgeOp { value: s("2"), edges: EdgeSel::Edges(vec![m]), mode: Default::default(), prop: Default::default(), keys: vec![] }));
            let _ = alvo;
            let r = recompute(&d);
            assert!(matches!(r.status.get(&fl), Some(Status::Ok | Status::Warning(_))), "filete G{continuidade}: {:?}", r.status.get(&fl));
        }
    }

    #[test]
    fn editar_um_no_refaz_a_conversao_e_os_filhos() {
        let g = caixa_lisa();
        let (mut d, id) = doc_com(g, 1);
        let s = |x: &str| x.to_string();
        let filho = d.add_feature(FeatureKind::Primitive(Primitive { kind: PrimKind::Caixa, plane: PlaneRef::Origin(OriginPlane::XY), a: s("20"), b: s("20"), c: s("200"), d: s("0"), op: BoolOp::Cut }));
        let v0 = recompute(&d).bodies[0].mass.volume;
        if let Some(FeatureKind::Forma(f)) = d.feature_mut(id).map(|f| &mut f.kind) {
            let n = f.gaiola.nos.iter().find(|n| n.p == [0., 0., 50.]).unwrap().id;
            f.gaiola.no_mut(n).unwrap().p[2] += 30.;
        }
        let r = recompute(&d);
        assert!(matches!(r.status.get(&filho), Some(Status::Ok)), "{:?}", r.status.get(&filho));
        assert!(r.bodies[0].mass.volume > v0);
    }

    #[test]
    fn gaiola_aberta_pausa_a_conversao_e_os_filhos_esperam() {
        let (mut d, id) = doc_com(caixa_lisa(), 0);
        let s = |x: &str| x.to_string();
        let filho = d.add_feature(FeatureKind::Primitive(Primitive { kind: PrimKind::Caixa, plane: PlaneRef::Origin(OriginPlane::XY), a: s("20"), b: s("20"), c: s("200"), d: s("0"), op: BoolOp::Cut }));
        if let Some(FeatureKind::Forma(f)) = d.feature_mut(id).map(|f| &mut f.kind) {
            f.conversao.as_mut().unwrap().pausada = true;
        }
        let r = recompute(&d);
        assert!(r.bodies.is_empty());
        assert_eq!(r.formas.len(), 1, "a superfície continua aparecendo");
        assert!(r.desatualizados.contains(&filho));
    }

    #[test]
    fn faces_tem_o_nome_da_face_da_gaiola() {
        let g = caixa_lisa();
        let (d, _) = doc_com(g.clone(), 1);
        let r = recompute(&d);
        let b = &r.bodies[0];
        assert!(b.face_role.iter().all(|x| *x >= 0), "{:?}", b.face_role);
        let mut unicos = b.face_role.clone();
        unicos.sort_unstable();
        unicos.dedup();
        assert_eq!(unicos.len(), b.face_role.len(), "nomes repetidos");
        // mover um nó não troca os nomes
        let mut g2 = g.clone();
        let n = g2.nos.iter().find(|n| n.p == [0., 0., 50.]).unwrap().id;
        g2.no_mut(n).unwrap().p[2] += 10.;
        let (d2, _) = doc_com(g2, 1);
        let mut outros = recompute(&d2).bodies[0].face_role.clone();
        outros.sort_unstable();
        assert_eq!(unicos, outros);
    }
}
