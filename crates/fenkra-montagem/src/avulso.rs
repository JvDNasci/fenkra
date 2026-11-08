// SPDX-License-Identifier: LGPL-2.1-or-later
//! Item normalizado avulso (B242): um parafuso, porca ou arruela da biblioteca que a pessoa leva do
//! catálogo até um furo da montagem.
//!
//! Como funciona:
//! 1. [`furos_do_mundo`] lista os furos das peças visíveis já no mundo (boca, sentido para dentro,
//!    diâmetro, espessura); é nesses furos que o fantasma pode saltar;
//! 2. [`casa`] diz se um tamanho serve a um furo: a faixa de folga do tamanho contém o diâmetro, ou,
//!    em furo roscado, a rosca do furo é a do tamanho;
//! 3. [`pose`] dá a boca de entrada e o sentido do item. `inverter` (Tab na viewport) entra pela outra
//!    ponta do furo passante;
//! 4. [`inserir_avulso`] grava a peça (compartilhada ou cópia) e a põe na montagem com as mesmas três
//!    restrições do preenchimento de furos: concêntrico, contato na face e trava de giro. As três
//!    tiram os 6 graus de liberdade do item.
//!
//! O modo **padrão** reaproveita a peça `fixadores/<família>_<tamanho>[x<L>].fkp` (todos os itens
//! iguais apontam para o mesmo arquivo); o **personalizado** grava uma cópia só dele, que a pessoa
//! pode editar sem mexer nos outros.

use crate::fixadores::{apoio, arquivo_fixador, concentrico, espessura_ao_longo, mais_proximo, num, orientado, tamanho_da_rosca, trava_de_giro, GRUPO};
use crate::load::{full_path, PartCache};
use crate::*;
use fenkra_solidos::biblioteca::{Biblioteca, Familia, Tamanho};
use std::collections::HashMap;
use std::path::Path;

/// Folga de diâmetro entre o furo e a faixa do tamanho (mm).
const TOL: f64 = 0.05;

/// Peça compartilhada ou cópia própria.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ModoInsercao {
    #[default]
    Padrao,
    Personalizado,
}

/// Um furo de uma peça, no mundo, que pode receber um item avulso.
#[derive(Clone, Debug, PartialEq)]
pub struct FuroAlvo {
    pub comp: Id,
    pub nome: String,
    /// centro da boca de entrada
    pub boca: V3,
    /// sentido da boca para dentro do material (unitário)
    pub dir: V3,
    pub diametro: f64,
    /// comprimento útil ao longo de `dir` (espessura, ou profundidade do roscado cego)
    pub espessura: f64,
    /// atravessa a peça: só nele se entra pela outra ponta
    pub passante: bool,
    /// rosca do furo ("M6"); vazia no furo liso
    pub rosca: String,
}

/// Furos das peças visíveis (só as de `comps`, quando não vazio), no mundo. Furo cego liso não entra:
/// um parafuso ali não teria onde prender.
pub fn furos_do_mundo(asm: &Assembly, dir: Option<&Path>, cache: &mut PartCache, comps: &[Id]) -> Vec<FuroAlvo> {
    let mut v = vec![];
    let pecas: Vec<(Id, String, Placement, String)> = asm
        .nodes
        .iter()
        .filter(|n| n.visible && (comps.is_empty() || comps.contains(&n.id)))
        .filter_map(|n| match &n.kind {
            NodeKind::Part { path } => Some((n.id, n.name.clone(), n.placement, path.clone())),
            _ => None,
        })
        .collect();
    for (id, nome, pl, path) in pecas {
        let Ok(lp) = cache.get(&full_path(dir, &path), true) else { continue };
        for r in fenkra_solidos::hole_table(&lp.doc) {
            let roscado = !r.thread.trim().is_empty();
            if !roscado && !r.through {
                continue;
            }
            let espessura = if roscado && !r.through { Some(r.depth) } else { lp.shape.as_ref().and_then(|s| espessura_ao_longo(s, r.pos, r.dir, r.diameter)) };
            let Some(espessura) = espessura.filter(|e| *e > 1e-6) else { continue };
            v.push(FuroAlvo { comp: id, nome: nome.clone(), boca: pl.point(r.pos), dir: unit(pl.dir(r.dir)), diametro: r.diameter, espessura, passante: r.through, rosca: r.thread.trim().to_string() });
        }
        // furo sem comando de furo (corte de círculo, peça importada): pela aresta circular da boca
        let Some(forma) = lp.shape.as_ref() else { continue };
        for (boca, dir, diametro, espessura) in furos_por_aresta(&lp.result, forma) {
            let (boca, dir) = (pl.point(boca), unit(pl.dir(dir)));
            if v.iter().any(|f| f.comp == id && mesmo_eixo(f, boca, dir, diametro)) {
                continue;
            }
            v.push(FuroAlvo { comp: id, nome: nome.clone(), boca, dir, diametro, espessura, passante: true, rosca: String::new() });
        }
        // ponto publicado: o item assenta nele, com o eixo na normal da face, e serve a qualquer
        // tamanho (diâmetro 0). Fixadores já inseridos publicam a ponta; esses não viram alvo.
        if lp.doc.props.part_type == fenkra_core::PartType::Hardware {
            continue;
        }
        for (publicacao, g) in fenkra_solidos::publications(&lp.doc) {
            let fenkra_core::LinkedGeo::Point(p) = g else { continue };
            let Some(din) = normal_para_dentro(forma, p) else { continue };
            let espessura = crate::fixadores::espessura_ao_longo(forma, p, din, 1.).unwrap_or(1.);
            v.push(FuroAlvo { comp: id, nome: format!("{nome} · {publicacao}"), boca: pl.point(p), dir: unit(pl.dir(din)), diametro: 0., espessura, passante: false, rosca: String::new() });
        }
    }
    v
}

/// Sentido para dentro do material num ponto da superfície da peça: o dos eixos da peça em que, um
/// milímetro antes do ponto, o raio bate na face no próprio ponto. `None` se o ponto não está numa
/// face plana alinhada a um eixo (o caso comum das publicações de montagem).
fn normal_para_dentro(forma: &fenkra_kernel::Shape, p: V3) -> Option<V3> {
    let eixos: [V3; 6] = [[0., 0., -1.], [0., 0., 1.], [-1., 0., 0.], [1., 0., 0.], [0., -1., 0.], [0., 1., 0.]];
    eixos.into_iter().find(|d| {
        let o: V3 = [0, 1, 2].map(|i| p[i] - d[i]);
        matches!(forma.ray_hit(o, *d), Ok(Some((q, _))) if norm(sub(q, p)) < 0.05)
    })
}

/// O furo `f` está no mesmo eixo de `boca`/`dir` e tem o mesmo diâmetro (as duas bocas de um passante,
/// ou o furo que o comando de furo já listou).
fn mesmo_eixo(f: &FuroAlvo, boca: V3, dir: V3, diametro: f64) -> bool {
    let paralelo = norm(cross(f.dir, dir)) < 1e-3;
    let fora = sub(boca, f.boca);
    let dist = norm(sub(fora, f.dir.map(|x| x * dot(fora, f.dir))));
    paralelo && dist < 0.05 && (f.diametro - diametro).abs() < 0.05
}

/// Furos passantes achados pelas arestas circulares inteiras da peça (coordenadas da peça): boca,
/// sentido para dentro do material, diâmetro e espessura. Uma aresta é boca de furo quando, a um passo
/// para fora do raio, a face começa no plano da aresta (há material ao redor) e, pelo centro, o raio
/// atravessa a peça sem tocar nada (é vazio por dentro, até o outro lado). Ressalto e pino não entram:
/// pelo centro deles o raio bate na face.
pub fn furos_por_aresta(res: &fenkra_solidos::RecomputeResult, forma: &fenkra_kernel::Shape) -> Vec<(V3, V3, f64, f64)> {
    let mut v: Vec<(V3, V3, f64, f64)> = vec![];
    for b in &res.bodies {
        let n_arestas = b.mesh.edge_geom.len() / fenkra_kernel::EDGE_GEOM;
        for e in 0..n_arestas {
            let Some(fenkra_kernel::EdgeFormula::Conic { c, u, v: w, t0, t1 }) = b.mesh.edge_formula(e) else { continue };
            let (ru, rw) = (norm(u), norm(w));
            if (ru - rw).abs() > 1e-6 * ru.max(1.) || (t1 - t0) < 2. * std::f64::consts::PI - 1e-6 || ru < 0.5 {
                continue;
            }
            let eixo = unit(cross(u, w));
            let radial = unit(u);
            for din in [eixo, eixo.map(|x| -x)] {
                // material começa no plano da aresta, logo fora do raio
                let anel: V3 = [0, 1, 2].map(|i| c[i] + radial[i] * (ru + 0.4) - din[i]);
                let toca = matches!(forma.ray_hit(anel, din), Ok(Some((p, _))) if (dot(sub(p, anel), din) - 1.).abs() < 0.05);
                if !toca {
                    continue;
                }
                // vazio pelo centro, de ponta a ponta
                let centro: V3 = [0, 1, 2].map(|i| c[i] - din[i]);
                if !matches!(forma.ray_hit(centro, din), Ok(None)) {
                    continue;
                }
                let Some(esp) = crate::fixadores::espessura_ao_longo(forma, c, din, 2. * ru).filter(|e| *e > 1e-6) else { continue };
                let alvo = FuroAlvo { comp: Id(0), nome: String::new(), boca: c, dir: din, diametro: 2. * ru, espessura: esp, passante: true, rosca: String::new() };
                if !v.iter().any(|(bb, dd, d, _)| mesmo_eixo(&alvo, *bb, *dd, *d)) {
                    v.push((c, din, 2. * ru, esp));
                }
            }
        }
    }
    v
}

/// O tamanho serve ao furo? Liso: o diâmetro cai na faixa do tamanho (fina a folgada, ou a faixa que a
/// família declara). Roscado: a rosca do furo é a do tamanho.
pub fn casa(fam: &Familia, t: &Tamanho, f: &FuroAlvo) -> bool {
    // ponto publicado: não tem diâmetro, qualquer tamanho assenta nele
    if f.diametro <= 0. {
        return true;
    }
    if !f.rosca.is_empty() {
        return tamanho_da_rosca(fam, &f.rosca).is_some_and(|d| d == t.designacao);
    }
    fam.faixa_furo(t).is_some_and(|(a, b)| f.diametro >= a - TOL && f.diametro <= b + TOL)
}

/// Tamanhos da família que servem ao furo, do menor para o maior.
pub fn tamanhos_que_casam<'a>(fam: &'a Familia, f: &FuroAlvo) -> Vec<&'a Tamanho> {
    let mut v: Vec<&Tamanho> = fam.tamanhos.iter().filter(|t| casa(fam, t, f)).collect();
    v.sort_by(|a, b| a.diametro_nominal().partial_cmp(&b.diametro_nominal()).unwrap_or(std::cmp::Ordering::Equal));
    v
}

/// Boca de entrada do item e sentido de entrada. Sem `inverter`: a boca do furo e o sentido dele.
/// Com `inverter`: a saída do furo e o sentido oposto. Só furo passante inverte.
pub fn pose(f: &FuroAlvo, inverter: bool) -> Result<(V3, V3), String> {
    if !inverter {
        return Ok((f.boca, f.dir));
    }
    if !f.passante {
        return Err("furo cego: o item só entra pela boca".into());
    }
    let saida = [0, 1, 2].map(|i| f.boca[i] + f.dir[i] * f.espessura);
    Ok((saida, f.dir.map(|x| -x)))
}

/// Comprimento que a série do tamanho oferece para o furo: no passante, o menor L que atravessa a
/// espessura com dois passos de rosca sobrando (ou o maior da série); no roscado cego, o maior L que
/// cabe na profundidade.
pub fn comprimento_sugerido(fam: &Familia, t: &Tamanho, f: &FuroAlvo) -> Option<f64> {
    let serie = fam.comprimentos_de(t);
    let passo = t.valor("P").unwrap_or(0.);
    if !f.rosca.is_empty() && !f.passante {
        return serie.iter().copied().filter(|l| *l <= f.espessura + 1e-9).last().or(serie.first().copied());
    }
    serie.iter().copied().find(|l| *l >= f.espessura + 2. * passo - 1e-9).or(serie.last().copied())
}

/// Onde o item fica no mundo: origem na boca de entrada. Parafuso com Z = -u (cabeça para fora, corpo
/// para dentro); porca e arruela com Z = u (o corpo fica do lado de fora da peça). É a pose do
/// fantasma e a da peça inserida.
pub fn posicao_do_item(parafuso: bool, entrada: V3, u: V3) -> Placement {
    orientado(entrada, if parafuso { u.map(|x| -x) } else { u })
}

#[derive(Clone, Debug)]
pub struct ParamAvulso {
    pub familia: String,
    pub tamanho: String,
    /// comprimento pedido (parafusos); vale o valor da série mais próximo. `None`: o sugerido
    pub comprimento: Option<f64>,
    pub furo: FuroAlvo,
    pub inverter: bool,
    pub modo: ModoInsercao,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Inserido {
    pub comp: Id,
    /// caminho da peça relativo à montagem
    pub arquivo: String,
    pub nome: String,
    /// boca de entrada e sentido (da boca para dentro) usados
    pub entrada: V3,
    pub eixo: V3,
    pub comprimento: Option<f64>,
    /// restrições criadas (concêntrica, trava de giro, contato)
    pub restricoes: Vec<Id>,
}

/// Grava uma cópia própria da peça, com número livre na pasta `fixadores/`.
fn arquivo_copia(dir: &Path, fam: &Familia, t: &Tamanho, l: Option<f64>) -> Result<(String, String), String> {
    let d = crate::fixadores::designacao_de_arquivo(&t.designacao);
    let base = match l {
        Some(l) => format!("{}_{}x{}", fam.id, d, num(l)),
        None => format!("{}_{}", fam.id, d),
    };
    let pasta = dir.join("fixadores");
    std::fs::create_dir_all(&pasta).map_err(|e| format!("{}: {e}", pasta.display()))?;
    let n = (1..).find(|n| !pasta.join(format!("{base}_copia{n}.fkp")).exists()).unwrap_or(1);
    let mut doc = fenkra_solidos::fixadores::gerar(fam, &t.designacao, l)?;
    doc.props.description = format!("{} (cópia {n})", doc.props.description);
    let nome = doc.props.description.clone();
    let arq = format!("fixadores/{base}_copia{n}.fkp");
    std::fs::write(dir.join(&arq), doc.to_json()).map_err(|e| format!("{arq}: {e}"))?;
    Ok((arq, nome))
}

/// Comprimento de um parafuso que não está num furo: o da série mais perto de quatro diâmetros.
pub fn comprimento_livre(fam: &Familia, t: &Tamanho) -> Option<f64> {
    mais_proximo(&fam.comprimentos_de(t), 4. * t.diametro_nominal().unwrap_or(0.))
}

/// Grava (ou reaproveita) a peça do item e devolve arquivo, nome e comprimento escolhido. Parafuso:
/// o comprimento pedido vale o da série mais próximo; sem pedido, `sugerido`, e sem sugestão o da série
/// mais perto de quatro diâmetros. Porca e arruela não têm comprimento.
fn peca_do_item(dir: &Path, fam: &Familia, t: &Tamanho, pedido: Option<f64>, sugerido: Option<f64>, modo: ModoInsercao) -> Result<(String, String, Option<f64>), String> {
    let comprimento = if fam.tipo.e_parafuso() {
        let serie = fam.comprimentos_de(t);
        let alvo = pedido.or(sugerido).or_else(|| comprimento_livre(fam, t)).ok_or("tamanho sem série de comprimentos")?;
        Some(mais_proximo(&serie, alvo).ok_or("tamanho sem série de comprimentos")?)
    } else {
        None
    };
    let (arquivo, nome) = match modo {
        ModoInsercao::Padrao => arquivo_fixador(dir, fam, t, comprimento, &mut HashMap::new(), &mut vec![])?,
        ModoInsercao::Personalizado => arquivo_copia(dir, fam, t, comprimento)?,
    };
    Ok((arquivo, nome, comprimento))
}

/// Onde um item fora de furo entra (B374).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PosicaoLivre {
    /// num ponto da superfície de uma peça, com o eixo do item na normal (que aponta para fora do material)
    Ponto { ponto: V3, normal: V3 },
    /// solto no espaço, sem eixo escolhido: o item entra com os eixos do mundo
    Solto { origem: V3 },
}

/// Primeiro ponto em que o raio (`origem`, `dir`, no mundo) toca as malhas, com a normal da face virada
/// para quem olha. Cada malha vem no sistema local dela, com a posição que a leva ao mundo: as cópias
/// de uma peça dividem a mesma malha e só mudam de posição (corpo já no mundo entra com a posição
/// padrão). É onde o item cai quando a pessoa aponta uma superfície.
pub fn ponto_na_cena<'a>(malhas: impl IntoIterator<Item = (&'a fenkra_kernel::Mesh, Placement)>, origem: V3, dir: V3) -> Option<(V3, V3)> {
    let d = unit(dir);
    let mut melhor: Option<(f64, V3)> = None;
    for (m, pl) in malhas {
        // o raio no sistema da malha: a posição é rígida, então a distância ao longo dele não muda
        let (o, d) = (pl.to_local(origem), unit(pl.dir_to_local(d)));
        let p = |i: u32| -> V3 { [0, 1, 2].map(|k| m.positions[3 * i as usize + k] as f64) };
        for t in m.indices.chunks_exact(3) {
            let (a, b, c) = (p(t[0]), p(t[1]), p(t[2]));
            let (e1, e2) = (sub(b, a), sub(c, a));
            let h = cross(d, e2);
            let det = dot(e1, h);
            if det.abs() < 1e-12 {
                continue;
            }
            let s = sub(o, a);
            let u = dot(s, h) / det;
            if !(-1e-9..=1. + 1e-9).contains(&u) {
                continue;
            }
            let q = cross(s, e1);
            let v = dot(d, q) / det;
            if v < -1e-9 || u + v > 1. + 1e-9 {
                continue;
            }
            let dist = dot(e2, q) / det;
            if dist > 1e-6 && melhor.is_none_or(|(m0, _)| dist < m0) {
                melhor = Some((dist, pl.dir(cross(e1, e2))));
            }
        }
    }
    let (dist, n) = melhor?;
    let mut n = unit(n);
    if dot(n, d) > 0. {
        n = n.map(|x| -x);
    }
    Some(([0, 1, 2].map(|i| origem[i] + d[i] * dist), n))
}

/// Põe o item na montagem como peça comum, sem restrição nenhuma: a pessoa o posiciona depois (6 graus
/// de liberdade, a não ser que seja a primeira peça, que ancora o conjunto). `pai` é o grupo de
/// destino; sem ele a peça entra na raiz. Erro com a frase do problema.
#[allow(clippy::too_many_arguments)]
pub fn inserir_livre(asm: &mut Assembly, dir: &Path, lib: &Biblioteca, familia: &str, tamanho: &str, comprimento: Option<f64>, modo: ModoInsercao, onde: PosicaoLivre, pai: Option<Id>) -> Result<Inserido, String> {
    let fam = lib.familia(familia).ok_or_else(|| format!("família desconhecida: {familia}"))?;
    let t = fam.tamanho(tamanho).ok_or_else(|| format!("{}: não existe o tamanho {tamanho}", fam.norma))?;
    let (arquivo, nome, comprimento) = peca_do_item(dir, fam, t, comprimento, None, modo)?;
    let primeira = crate::insercao::fixa_na_origem(asm);
    let (pl, eixo) = match onde {
        PosicaoLivre::Ponto { ponto, normal } => {
            let u = unit(normal).map(|x| -x);
            (posicao_do_item(fam.tipo.e_parafuso(), ponto, u), u)
        }
        PosicaoLivre::Solto { origem } => (Placement { origin: if primeira { [0.; 3] } else { origem }, ..Default::default() }, [0., 0., 1.]),
    };
    let comp = asm.add_component(&nome, NodeKind::Part { path: arquivo.clone() }, pl, pai);
    if asm.relational {
        if let Some(n) = asm.node_mut(comp) {
            n.fixed = true;
        }
    }
    Ok(Inserido { comp, arquivo, nome, entrada: pl.origin, eixo, comprimento, restricoes: vec![] })
}

/// Põe o item no furo, restrito. Erro com a frase do problema (tamanho que não serve, família
/// desconhecida, furo cego invertido).
pub fn inserir_avulso(asm: &mut Assembly, dir: &Path, lib: &Biblioteca, p: &ParamAvulso) -> Result<Inserido, String> {
    let fam = lib.familia(&p.familia).ok_or_else(|| format!("família desconhecida: {}", p.familia))?;
    let t = fam.tamanho(&p.tamanho).ok_or_else(|| format!("{}: não existe o tamanho {}", fam.norma, p.tamanho))?;
    if !casa(fam, t, &p.furo) {
        return Err(format!("{} {} não serve ao furo Ø{} de {}", fam.norma, t.designacao, num(p.furo.diametro), p.furo.nome));
    }
    if asm.node(p.furo.comp).is_none() {
        return Err("a peça do furo não está mais na montagem".into());
    }
    let (entrada, u) = pose(&p.furo, p.inverter)?;
    let sugerido = comprimento_sugerido(fam, t, &p.furo);
    let (arquivo, nome, comprimento) = peca_do_item(dir, fam, t, p.comprimento, sugerido, p.modo)?;
    let grupo = match asm.nodes.iter().find(|n| matches!(n.kind, NodeKind::Group) && n.name == GRUPO).map(|n| n.id) {
        Some(g) => g,
        None => asm.add_group(GRUPO, None),
    };
    let baixo = u.map(|x| -x);
    // parafuso: Z = -u (cabeça para fora, corpo para dentro); porca e arruela: Z = u (corpo para fora da peça)
    let pl = posicao_do_item(fam.tipo.e_parafuso(), entrada, u);
    let comp = asm.add_component(&nome, NodeKind::Part { path: arquivo.clone() }, pl, Some(grupo));
    let antes: Vec<Id> = asm.mates.iter().map(|m| m.id).collect();
    let eixo = (entrada, baixo);
    concentrico(asm, p.furo.comp, comp, eixo);
    trava_de_giro(asm, p.furo.comp, comp, eixo);
    apoio(asm, p.furo.comp, (entrada, baixo), comp, (entrada, u));
    let restricoes = asm.mates.iter().map(|m| m.id).filter(|i| !antes.contains(i)).collect();
    Ok(Inserido { comp, arquivo, nome, entrada, eixo: u, comprimento, restricoes })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixadores::tests::{pasta, placa};
    use fenkra_solidos::biblioteca;

    fn montagem(dir: &Path, esp: f64, furo: f64) -> (Assembly, Id) {
        std::fs::write(dir.join("placa.fkp"), placa(esp, furo, None).to_json()).unwrap();
        let mut a = Assembly::new("Avulso");
        let c = a.add_component("Placa", NodeKind::Part { path: "placa.fkp".into() }, Placement::default(), None);
        (a, c)
    }

    fn param(furo: FuroAlvo, familia: &str, tamanho: &str) -> ParamAvulso {
        ParamAvulso { familia: familia.into(), tamanho: tamanho.into(), comprimento: None, furo, inverter: false, modo: ModoInsercao::Padrao }
    }

    #[test]
    fn furo_da_placa_aparece_no_mundo_e_so_o_m6_serve() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let dir = pasta("avulso_furos");
        let (a, c) = montagem(&dir, 10., 6.6);
        let furos = furos_do_mundo(&a, Some(&dir), &mut PartCache::default(), &[]);
        let [f] = &furos[..] else { panic!("{furos:?}") };
        assert_eq!((f.comp, f.diametro, f.passante), (c, 6.6, true));
        assert!(f.boca[0].hypot(f.boca[1]) < 1e-6 && (f.boca[2] - 10.).abs() < 1e-6 && f.dir[2] < -0.99, "{f:?}");
        assert!((f.espessura - 10.).abs() < 0.01, "{}", f.espessura);
        let fam = biblioteca::familia("iso4762").unwrap();
        let ok: Vec<&str> = tamanhos_que_casam(fam, f).iter().map(|t| t.designacao.as_str()).collect();
        assert_eq!(ok, ["M6"], "só o M6 tem 6,6 na faixa de folga");
        // comprimento sugerido: 10 + 2 passos = 12 → L12 ou o próximo da série
        let l = comprimento_sugerido(fam, fam.tamanho("M6").unwrap(), f).unwrap();
        assert!(l >= 12. && fam.comprimentos_de(fam.tamanho("M6").unwrap()).contains(&l), "{l}");
    }

    /// Placa 40 x 40 x 10 com um furo de Ø6,6 feito por corte de cilindro (sem comando de furo) e um
    /// pino de Ø6,6 em cima, que não é furo.
    fn placa_cortada() -> fenkra_core::Document {
        use fenkra_core::*;
        let mut d = Document::new("placa_cortada");
        let s = |x: &str| x.to_string();
        d.add_feature(FeatureKind::Primitive(Primitive { kind: PrimKind::Caixa, plane: PlaneRef::Origin(OriginPlane::XY), a: s("40"), b: s("40"), c: s("10"), d: s("0"), op: BoolOp::Add }));
        let baixo = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "-1");
        d.add_feature(FeatureKind::Primitive(Primitive { kind: PrimKind::Cilindro, plane: PlaneRef::Datum(baixo), a: s("3.3"), b: s("12"), c: s("0"), d: s("0"), op: BoolOp::Cut }));
        let topo = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "10");
        let _ = topo;
        d
    }

    #[test]
    fn furo_feito_por_corte_aparece_pela_aresta_circular_e_recebe_o_parafuso() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let dir = pasta("avulso_aresta");
        std::fs::write(dir.join("cortada.fkp"), placa_cortada().to_json()).unwrap();
        let mut a = Assembly::new("Aresta");
        let c = a.add_component("Cortada", NodeKind::Part { path: "cortada.fkp".into() }, Placement::default(), None);
        let furos = furos_do_mundo(&a, Some(&dir), &mut PartCache::default(), &[]);
        // um alvo só para as duas bocas do mesmo furo
        let [f] = &furos[..] else { panic!("{furos:?}") };
        assert_eq!(f.comp, c);
        assert!((f.diametro - 6.6).abs() < 1e-6 && f.passante && f.rosca.is_empty(), "{f:?}");
        assert!(f.boca[0].hypot(f.boca[1]) < 1e-6 && (f.espessura - 10.).abs() < 0.05, "{f:?}");
        assert!(f.dir[2].abs() > 0.99 && ((f.boca[2] - 10.).abs() < 1e-6 || f.boca[2].abs() < 1e-6), "{f:?}");
        let fam = biblioteca::familia("iso4762").unwrap();
        assert_eq!(tamanhos_que_casam(fam, f).iter().map(|t| t.designacao.as_str()).collect::<Vec<_>>(), ["M6"]);
        let r = inserir_avulso(&mut a, &dir, biblioteca::publica(), &param(f.clone(), "iso4762", "M6")).unwrap();
        let rep = solve::solve(&mut a);
        assert!(rep.converged && rep.dof == 0, "{rep:?}");
        let p = a.node(r.comp).unwrap().placement;
        assert!(p.origin[0].hypot(p.origin[1]) < 0.01, "no eixo do furo: {:?}", p.origin);
    }

    #[test]
    fn ponto_publicado_vira_alvo_e_o_item_entra_restrito_nele() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        use fenkra_core::*;
        // placa maciça 40 x 40 x 10 com um ponto publicado na face de cima, fora do centro
        let mut d = Document::new("placa_ponto");
        d.add_feature(FeatureKind::Primitive(Primitive { kind: PrimKind::Caixa, plane: PlaneRef::Origin(OriginPlane::XY), a: "40".into(), b: "40".into(), c: "10".into(), d: "0".into(), op: BoolOp::Add }));
        let pid = d.add_geometry(RefKind::PointCoords { x: "10".into(), y: "5".into(), z: "10".into(), frame: None });
        if let Some(r) = d.references.iter_mut().find(|r| r.id == pid) {
            r.name = "Pino".into();
            r.published = true;
        }
        let dir = pasta("avulso_ponto_pub");
        std::fs::write(dir.join("placa_ponto.fkp"), d.to_json()).unwrap();
        let mut a = Assembly::new("Ponto");
        a.add_component("Placa", NodeKind::Part { path: "placa_ponto.fkp".into() }, Placement::default(), None);
        let furos = furos_do_mundo(&a, Some(&dir), &mut PartCache::default(), &[]);
        let [f] = &furos[..] else { panic!("{furos:?}") };
        assert!(norm(sub(f.boca, [10., 5., 10.])) < 1e-6 && f.dir[2] < -0.99 && f.diametro == 0. && f.nome.ends_with("Pino"), "{f:?}");
        // qualquer tamanho serve ao ponto
        let fam = biblioteca::familia("iso4762").unwrap();
        assert_eq!(tamanhos_que_casam(fam, f).len(), fam.tamanhos.len());
        let r = inserir_avulso(&mut a, &dir, biblioteca::publica(), &param(f.clone(), "iso4762", "M8")).unwrap();
        let rep = solve::solve(&mut a);
        assert!(rep.converged && rep.dof == 0, "{rep:?}");
        let p = a.node(r.comp).unwrap().placement;
        assert!(norm(sub(p.origin, [10., 5., 10.])) < 0.01, "{:?}", p.origin);
        // o parafuso inserido publica a ponta, mas peça de fixador não vira alvo
        let depois = furos_do_mundo(&a, Some(&dir), &mut PartCache::default(), &[]);
        assert!(depois.iter().all(|x| x.comp != r.comp), "{depois:?}");
    }

    #[test]
    fn tamanho_em_polegada_com_barra_grava_a_peca() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        // "3/8-16 UNC": a barra virava pasta e a gravação falhava (B51)
        let dir = pasta("avulso_polegada");
        let mut a = Assembly::new("Polegada");
        let lib = biblioteca::publica();
        for modo in [ModoInsercao::Padrao, ModoInsercao::Personalizado] {
            let r = inserir_livre(&mut a, &dir, lib, "asme_b18_3", "3/8-16 UNC", None, modo, PosicaoLivre::Solto { origem: [0., 0., 0.] }, None).unwrap();
            assert!(!r.arquivo.contains('/') || r.arquivo.starts_with("fixadores/") && !r.arquivo["fixadores/".len()..].contains('/'), "{}", r.arquivo);
            assert!(dir.join(&r.arquivo).exists(), "{}", r.arquivo);
        }
        assert_eq!(crate::fixadores::designacao_de_arquivo("M6"), "M6");
        assert_eq!(crate::fixadores::designacao_de_arquivo("#10-24 UNC"), "_10-24_UNC");
    }

    #[test]
    fn ressalto_circular_nao_vira_furo() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        use fenkra_core::*;
        // cilindro maciço: as duas arestas circulares são de pino, não de furo
        let mut d = Document::new("pino");
        d.add_feature(FeatureKind::Primitive(Primitive { kind: PrimKind::Cilindro, plane: PlaneRef::Origin(OriginPlane::XY), a: "3.3".into(), b: "10".into(), c: "0".into(), d: "0".into(), op: BoolOp::Add }));
        let dir = pasta("avulso_pino");
        std::fs::write(dir.join("pino.fkp"), d.to_json()).unwrap();
        let mut a = Assembly::new("Pino");
        a.add_component("Pino", NodeKind::Part { path: "pino.fkp".into() }, Placement::default(), None);
        assert!(furos_do_mundo(&a, Some(&dir), &mut PartCache::default(), &[]).is_empty());
    }

    #[test]
    fn parafuso_avulso_entra_restrito_com_zero_gdl_na_posicao_e_no_eixo_do_furo() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let dir = pasta("avulso_gdl");
        let (mut a, c) = montagem(&dir, 10., 6.6);
        let f = furos_do_mundo(&a, Some(&dir), &mut PartCache::default(), &[]).remove(0);
        let antes = a.clone();
        let r = inserir_avulso(&mut a, &dir, biblioteca::publica(), &param(f, "iso4762", "M6")).unwrap();
        assert_eq!(r.restricoes.len(), 3, "concêntrico, trava de giro e contato");
        assert_eq!(a.nodes.len(), antes.nodes.len() + 2, "grupo Fixadores e a peça");
        let rep = solve::solve(&mut a);
        assert!(rep.converged && rep.dof == 0, "{rep:?}");
        assert_eq!(solve::dof_per_comp(&a).get(&r.comp).copied().unwrap_or(0), 0, "o parafuso fica sem graus de liberdade");
        // posição e eixo: no eixo do furo, cabeça apoiada na face de cima (z = 10), corpo para baixo
        let p = a.node(r.comp).unwrap().placement;
        assert!(p.origin[0].hypot(p.origin[1]) < 0.01 && (p.origin[2] - 10.).abs() < 0.01, "{:?}", p.origin);
        assert!(norm(sub(p.dir([0., 0., 1.]), [0., 0., 1.])) < 1e-6, "Z local aponta para a cabeça (para cima)");
        assert!(a.node(c).unwrap().placement == antes.node(c).unwrap().placement, "a placa não se mexe");
        // o arquivo é o da biblioteca, compartilhado
        assert_eq!(r.arquivo, format!("fixadores/iso4762_M6x{}.fkp", num(r.comprimento.unwrap())));
        assert!(dir.join(&r.arquivo).exists());
        // desfazer = voltar ao estado de antes (a janela guarda a cópia): sem a peça e sem restrições
        assert_eq!((antes.nodes.len(), antes.mates.len()), (1, 0));
        assert!(a.mates.len() == 3 && antes != a);
    }

    #[test]
    fn tab_inverte_o_sentido_e_o_item_entra_pela_saida_do_furo() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let dir = pasta("avulso_inverter");
        let (mut a, _) = montagem(&dir, 10., 6.6);
        let f = furos_do_mundo(&a, Some(&dir), &mut PartCache::default(), &[]).remove(0);
        let (entrada, u) = pose(&f, true).unwrap();
        assert!(entrada[2].abs() < 0.01 && u[2] > 0.99, "{entrada:?} {u:?}");
        let mut p = param(f, "iso4762", "M6");
        p.inverter = true;
        let r = inserir_avulso(&mut a, &dir, biblioteca::publica(), &p).unwrap();
        let rep = solve::solve(&mut a);
        assert!(rep.converged && rep.dof == 0, "{rep:?}");
        let pl = a.node(r.comp).unwrap().placement;
        // cabeça embaixo (z = 0), Z local para baixo, corpo para cima
        assert!(pl.origin[2].abs() < 0.01, "{:?}", pl.origin);
        assert!(norm(sub(pl.dir([0., 0., 1.]), [0., 0., -1.])) < 1e-6);
    }

    #[test]
    fn tamanho_errado_e_furo_cego_invertido_dao_erro_sem_mexer_na_montagem() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let dir = pasta("avulso_erros");
        let (mut a, _) = montagem(&dir, 10., 6.6);
        let f = furos_do_mundo(&a, Some(&dir), &mut PartCache::default(), &[]).remove(0);
        let antes = a.clone();
        let e = inserir_avulso(&mut a, &dir, biblioteca::publica(), &param(f.clone(), "iso4762", "M8")).unwrap_err();
        assert!(e.contains("não serve"), "{e}");
        let e = inserir_avulso(&mut a, &dir, biblioteca::publica(), &param(f.clone(), "naoexiste", "M6")).unwrap_err();
        assert!(e.contains("desconhecida"), "{e}");
        assert_eq!(a, antes);
        let cego = FuroAlvo { passante: false, ..f };
        assert!(pose(&cego, true).is_err());
    }

    #[test]
    fn modo_padrao_compartilha_a_peca_e_o_personalizado_grava_copia() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let dir = pasta("avulso_modos");
        let (mut a, _) = montagem(&dir, 10., 6.6);
        let f = furos_do_mundo(&a, Some(&dir), &mut PartCache::default(), &[]).remove(0);
        let lib = biblioteca::publica();
        let r1 = inserir_avulso(&mut a, &dir, lib, &param(f.clone(), "iso4762", "M6")).unwrap();
        let r2 = inserir_avulso(&mut a, &dir, lib, &param(f.clone(), "iso4762", "M6")).unwrap();
        assert_eq!(r1.arquivo, r2.arquivo, "padrão: mesma peça compartilhada");
        let mut p = param(f.clone(), "iso4762", "M6");
        p.modo = ModoInsercao::Personalizado;
        let c1 = inserir_avulso(&mut a, &dir, lib, &p).unwrap();
        let c2 = inserir_avulso(&mut a, &dir, lib, &p).unwrap();
        assert_ne!(c1.arquivo, r1.arquivo);
        assert_ne!(c1.arquivo, c2.arquivo, "cada cópia tem o seu arquivo");
        assert!(c1.arquivo.contains("_copia1") && c2.arquivo.contains("_copia2"), "{} {}", c1.arquivo, c2.arquivo);
        assert!(dir.join(&c1.arquivo).exists() && dir.join(&c2.arquivo).exists());
    }

    #[test]
    fn arruela_avulsa_assenta_na_boca_do_furo() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let dir = pasta("avulso_arruela");
        let (mut a, _) = montagem(&dir, 10., 6.6);
        let f = furos_do_mundo(&a, Some(&dir), &mut PartCache::default(), &[]).remove(0);
        let ar = inserir_avulso(&mut a, &dir, biblioteca::publica(), &param(f, "iso7089", "M6")).unwrap();
        let rep = solve::solve(&mut a);
        assert!(rep.converged && rep.dof == 0, "{rep:?}");
        assert_eq!(ar.comprimento, None);
        let pl = a.node(ar.comp).unwrap().placement;
        // arruela fora da peça: face de apoio em z = 10, Z local para dentro do furo (para baixo), de
        // modo que o corpo (de -h a 0 no eixo dela) fique acima da face
        assert!((pl.origin[2] - 10.).abs() < 0.01 && pl.origin[0].hypot(pl.origin[1]) < 0.01, "{:?}", pl.origin);
        assert!(norm(sub(pl.dir([0., 0., 1.]), [0., 0., -1.])) < 1e-6);
    }

    // ---------------------------------------------------------------- B374: em ponto e solto

    #[test]
    fn em_ponto_o_item_pousa_na_face_apontada_com_o_eixo_na_normal_e_sem_restricao() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let dir = pasta("avulso_ponto");
        let (mut a, c) = montagem(&dir, 10., 6.6);
        let mut erros = vec![];
        let (_, vista) = crate::view::compose(&a, Some(&dir), &mut PartCache::default(), &mut erros);
        assert!(erros.is_empty(), "{erros:?}");
        // raio de cima para baixo em (20, 10): acerta a face de cima da placa (z = 10), fora do furo
        let (ponto, normal) = ponto_na_cena(vista.bodies.iter().map(|b| (&b.mesh, Placement::default())), [20., 10., 80.], [0., 0., -1.]).expect("o raio acerta a placa");
        assert!((ponto[0] - 20.).abs() < 1e-6 && (ponto[1] - 10.).abs() < 1e-6 && (ponto[2] - 10.).abs() < 1e-6, "{ponto:?}");
        assert!(normal[2] > 0.999, "a normal aponta para fora do material: {normal:?}");
        // de baixo para cima a normal vira para quem olha (para baixo, na face z = 0)
        let (pb, nb) = ponto_na_cena(vista.bodies.iter().map(|b| (&b.mesh, Placement::default())), [20., 10., -80.], [0., 0., 1.]).unwrap();
        assert!(pb[2].abs() < 1e-6 && nb[2] < -0.999, "{pb:?} {nb:?}");
        // raio que passa ao lado da placa não acerta nada
        assert!(ponto_na_cena(vista.bodies.iter().map(|b| (&b.mesh, Placement::default())), [200., 0., 80.], [0., 0., -1.]).is_none());

        // a mesma malha em outra posição (cópia): o raio acha a face no lugar da cópia, não no da malha
        let longe = Placement { origin: [500., 0., 0.], ..Default::default() };
        let (pc, nc) = ponto_na_cena(vista.bodies.iter().map(|b| (&b.mesh, longe)), [520., 10., 80.], [0., 0., -1.]).expect("a cópia a 500 mm");
        assert!((pc[0] - 520.).abs() < 1e-6 && (pc[2] - 10.).abs() < 1e-6 && nc[2] > 0.999, "{pc:?} {nc:?}");
        assert!(ponto_na_cena(vista.bodies.iter().map(|b| (&b.mesh, longe)), [20., 10., 80.], [0., 0., -1.]).is_none(), "onde a malha estaria sem a posição não há nada");
        // cópia girada: o eixo local Z da peça vira o X do mundo, e a normal da face de cima vai junto
        let girada = orientado([0., 0., 0.], [1., 0., 0.]);
        let (pg, ng) = ponto_na_cena(vista.bodies.iter().map(|b| (&b.mesh, girada)), [80., 10., 5.], [-1., 0., 0.]).expect("a cópia girada");
        assert!((pg[0] - 10.).abs() < 1e-6 && norm(sub(ng, [1., 0., 0.])) < 1e-6, "{pg:?} {ng:?}");

        let antes = a.mates.len();
        let r = inserir_livre(&mut a, &dir, biblioteca::publica(), "iso4762", "M6", Some(20.), ModoInsercao::Padrao, PosicaoLivre::Ponto { ponto, normal }, None).unwrap();
        assert_eq!(a.mates.len(), antes, "em ponto não nasce restrição: a pessoa posiciona depois");
        assert!(r.restricoes.is_empty() && r.comprimento == Some(20.));
        assert!(dir.join(&r.arquivo).exists());
        let pl = a.node(r.comp).unwrap().placement;
        assert!(norm(sub(pl.origin, ponto)) < 1e-9, "origem no ponto apontado: {:?}", pl.origin);
        assert!(norm(sub(pl.dir([0., 0., 1.]), [0., 0., 1.])) < 1e-9, "parafuso: Z local para fora da face (cabeça), corpo para dentro");
        // sem restrição, o item é um corpo livre: 6 graus de liberdade
        let rep = solve::solve(&mut a);
        assert!(rep.converged, "{rep:?}");
        assert_eq!(solve::dof_per_comp(&a).get(&r.comp).copied().unwrap_or(6), 6, "livre para posicionar depois");
        assert!(a.node(c).unwrap().placement == Placement::default(), "a placa não se mexe");
        // arruela no mesmo ponto: o corpo fica acima da face, como no furo
        let ar = inserir_livre(&mut a, &dir, biblioteca::publica(), "iso7089", "M6", None, ModoInsercao::Padrao, PosicaoLivre::Ponto { ponto, normal }, None).unwrap();
        assert_eq!(ar.comprimento, None);
        assert!(norm(sub(a.node(ar.comp).unwrap().placement.dir([0., 0., 1.]), [0., 0., -1.])) < 1e-9);
    }

    #[test]
    fn solto_no_espaco_entra_com_os_eixos_do_mundo_e_dentro_do_grupo_escolhido() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let dir = pasta("avulso_solto");
        let (mut a, _) = montagem(&dir, 10., 6.6);
        let grupo = a.add_group("Ferragens", None);
        let r = inserir_livre(&mut a, &dir, biblioteca::publica(), "iso4762", "M8", None, ModoInsercao::Padrao, PosicaoLivre::Solto { origem: [100., -40., 25.] }, Some(grupo)).unwrap();
        let n = a.node(r.comp).unwrap();
        assert_eq!(n.placement.origin, [100., -40., 25.]);
        assert!(norm(sub(n.placement.dir([0., 0., 1.]), [0., 0., 1.])) < 1e-12, "sem eixo escolhido");
        assert_eq!(n.parent, Some(grupo), "entra no grupo de destino");
        assert!(a.mates.is_empty());
        // sem comprimento pedido: o da série mais perto de 4 diâmetros (M8 → 32)
        let fam = biblioteca::familia("iso4762").unwrap();
        assert!(fam.comprimentos_de(fam.tamanho("M8").unwrap()).contains(&r.comprimento.unwrap()));
        assert!((r.comprimento.unwrap() - 32.).abs() <= 4., "{:?}", r.comprimento);
        // o modo personalizado grava cópia própria também fora do furo
        let c = inserir_livre(&mut a, &dir, biblioteca::publica(), "iso4762", "M8", Some(30.), ModoInsercao::Personalizado, PosicaoLivre::Solto { origem: [0., 0., 50.] }, None).unwrap();
        assert!(c.arquivo.contains("_copia1") && dir.join(&c.arquivo).exists(), "{}", c.arquivo);
        // erros: família e tamanho inexistentes não mexem na montagem
        let antes = a.clone();
        assert!(inserir_livre(&mut a, &dir, biblioteca::publica(), "naoexiste", "M8", None, ModoInsercao::Padrao, PosicaoLivre::Solto { origem: [0.; 3] }, None).is_err());
        assert!(inserir_livre(&mut a, &dir, biblioteca::publica(), "iso4762", "M99", None, ModoInsercao::Padrao, PosicaoLivre::Solto { origem: [0.; 3] }, None).is_err());
        assert_eq!(a, antes);
    }

    #[test]
    fn solto_na_montagem_vazia_ancora_o_conjunto_na_origem() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let dir = pasta("avulso_vazia");
        let mut a = Assembly::new("Vazia");
        let r = inserir_livre(&mut a, &dir, biblioteca::publica(), "iso4762", "M6", Some(16.), ModoInsercao::Padrao, PosicaoLivre::Solto { origem: [50., 50., 50.] }, None).unwrap();
        let n = a.node(r.comp).unwrap();
        assert_eq!(n.placement.origin, [0.; 3], "a primeira peça ancora o conjunto na origem");
        assert!(n.fixed, "e fica fixa");
    }
}
