//! Fixadores inteligentes (B29): preenche os furos de uma montagem com parafuso, arruelas e porca
//! da biblioteca pública (`fenkra_solidos::biblioteca`).
//!
//! Como funciona:
//! 1. cada peça visível dá a sua `hole_table`; furos passantes cujo diâmetro casa com a folga
//!    escolhida de um tamanho métrico (±0,05 mm) e furos roscados ("M6", "M8x1.25"...) viram
//!    candidatos;
//! 2. candidatos de peças diferentes com eixos colineares no mundo (0,05 mm / 0,5°) formam a
//!    **pilha** que um mesmo parafuso atravessa;
//! 3. a pega é a distância entre a boca de entrada e a saída da pilha ao longo do eixo; o
//!    comprimento é a pega + arruelas + porca + 2 passos, arredondado para cima na série da
//!    família (em furo roscado: a profundidade útil arredondada para baixo, sem porca);
//! 4. as peças do fixador são gravadas em `<pasta>/fixadores/` (reaproveitadas se já existirem) e
//!    entram na montagem já na posição final, com restrições de eixo (concêntrico) e de contato.
//!
//! Convenção do fixador (ver `fenkra_solidos::fixadores`): eixo Z, face de apoio em z = 0, corpo
//! para -Z. Com `u` o sentido de entrada do parafuso (da boca para a saída), o parafuso e as
//! peças do lado da porca têm Z = -u e a arruela sob a cabeça tem Z = +u (o corpo dela fica do lado
//! de fora da peça).

use crate::load::{full_path, PartCache};
use crate::*;
use fenkra_core::PartType;
use fenkra_kernel::Shape;
use fenkra_solidos::biblioteca::{self, Familia, Tamanho};
use fenkra_solidos::fixadores::fixador_documento;
use fenkra_solidos::hardware;
use std::collections::HashMap;
use std::path::Path;

/// Tolerância de diâmetro entre o furo e a folga da tabela (mm).
const TOL_DIAMETRO: f64 = 0.05;
/// Tolerância de distância entre eixos coaxiais (mm).
const TOL_EIXO: f64 = 0.05;
/// Tolerância angular entre eixos paralelos (graus).
const TOL_ANGULO: f64 = 0.5;
/// Nome do grupo da árvore que recebe os fixadores.
const GRUPO: &str = "Fixadores";

#[derive(Clone, Debug, PartialEq)]
pub struct OpcoesFixadores {
    /// família do parafuso ("iso4762", "iso4017", "din933")
    pub familia_parafuso: String,
    /// arruela sob a cabeça (e, com porca, também sob ela)
    pub arruela: bool,
    pub porca: bool,
    pub familia_porca: String,
    pub familia_arruela: String,
    /// folga do furo de passagem: 0 = fino, 1 = normal, 2 = folgado
    pub folga: u8,
    /// só os furos destes componentes (vazio = todos os visíveis)
    pub componentes: Vec<Id>,
}

impl Default for OpcoesFixadores {
    fn default() -> Self {
        OpcoesFixadores { familia_parafuso: "iso4762".into(), arruela: true, porca: true, familia_porca: "iso4032".into(), familia_arruela: "iso7089".into(), folga: 1, componentes: vec![] }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RelatorioFixadores {
    /// parafusos inseridos
    pub fixadores: usize,
    /// ("M6x35", quantidade), na ordem em que apareceram
    pub por_tamanho: Vec<(String, usize)>,
    pub arruelas: usize,
    pub porcas: usize,
    /// pilhas que já tinham um fixador (ou outra peça de hardware) no eixo
    pub pulados: usize,
    /// furos sem tamanho compatível e outros problemas, uma frase cada
    pub avisos: Vec<String>,
}

impl RelatorioFixadores {
    fn conta(&mut self, rotulo: String) {
        self.fixadores += 1;
        match self.por_tamanho.iter_mut().find(|(r, _)| *r == rotulo) {
            Some((_, n)) => *n += 1,
            None => self.por_tamanho.push((rotulo, 1)),
        }
    }
}

/// Furo candidato, já no mundo.
#[derive(Clone, Debug)]
struct Furo {
    comp: Id,
    nome: String,
    /// centro da boca
    boca: V3,
    /// sentido da boca para dentro do material (unitário)
    dir: V3,
    diametro: f64,
    /// comprimento útil ao longo de `dir` a partir da boca (espessura, ou profundidade do roscado cego)
    espessura: f64,
    roscado: bool,
    /// designação do tamanho ("M6"); `None` = passante sem tamanho compatível
    tamanho: Option<String>,
    /// casa com a folga escolhida (ou é roscado): serve de âncora da pilha
    ancora: bool,
}

/// Preenche os furos da montagem com fixadores. `dir` é a pasta da montagem (onde nasce
/// `fixadores/`); sem ela não há onde gravar as peças.
pub fn preencher_furos(asm: &mut Assembly, dir: Option<&Path>, cache: &mut PartCache, op: &OpcoesFixadores) -> Result<RelatorioFixadores, String> {
    let dir = dir.ok_or("salve a montagem antes: as peças dos fixadores são gravadas na pasta dela")?;
    let fam_p = biblioteca::familia(&op.familia_parafuso).filter(|f| f.tipo.e_parafuso()).ok_or_else(|| format!("família de parafuso desconhecida: {}", op.familia_parafuso))?;
    let fam_arr = biblioteca::familia(&op.familia_arruela);
    let fam_porca = biblioteca::familia(&op.familia_porca);
    let mut rel = RelatorioFixadores::default();

    // 1) furos candidatos de todos os componentes
    let mut furos: Vec<Furo> = vec![];
    let comps: Vec<(Id, String, Placement, String)> = asm
        .nodes
        .iter()
        .filter(|n| n.visible && (op.componentes.is_empty() || op.componentes.contains(&n.id)))
        .filter_map(|n| match &n.kind {
            NodeKind::Part { path } => Some((n.id, n.name.clone(), n.placement, path.clone())),
            _ => None,
        })
        .collect();
    for (id, nome, pl, path) in comps {
        let lp = match cache.get(&full_path(Some(dir), &path), true) {
            Ok(l) => l,
            Err(e) => {
                rel.avisos.push(format!("{nome}: não foi possível ler a peça ({e})"));
                continue;
            }
        };
        for r in fenkra_solidos::hole_table(&lp.doc) {
            let roscado = !r.thread.trim().is_empty();
            let (tamanho, ancora) = if roscado {
                match tamanho_da_rosca(&r.thread) {
                    Some(t) => (Some(t), true),
                    None => {
                        rel.avisos.push(format!("{nome}: rosca {} sem tamanho métrico na biblioteca", r.thread.trim()));
                        continue;
                    }
                }
            } else if r.through {
                match casa_folga(r.diameter, op.folga) {
                    Some(t) => (Some(t.to_string()), true),
                    None => (casa_outra_folga(r.diameter).map(str::to_string), false),
                }
            } else {
                // furo cego liso: não recebe fixador
                continue;
            };
            let dir_w = unit(pl.dir(r.dir));
            let boca = pl.point(r.pos);
            let espessura = if roscado && !r.through {
                Some(r.depth)
            } else {
                lp.shape.as_ref().and_then(|s| espessura_ao_longo(s, r.pos, r.dir, r.diameter))
            };
            let Some(espessura) = espessura.filter(|e| *e > 1e-6) else {
                rel.avisos.push(format!("{nome}: não foi possível medir a espessura do furo Ø{}", num(r.diameter)));
                continue;
            };
            furos.push(Furo { comp: id, nome: nome.clone(), boca, dir: dir_w, diametro: r.diameter, espessura, roscado, tamanho, ancora });
        }
    }

    // 2) pilhas: furos de eixos colineares
    let mut pilhas: Vec<Vec<Furo>> = vec![];
    for f in furos {
        match pilhas.iter_mut().find(|p| colineares(p[0].boca, p[0].dir, f.boca, f.dir)) {
            Some(p) => p.push(f),
            None => pilhas.push(vec![f]),
        }
    }

    let eixos_existentes = eixos_das_restricoes(asm);
    let mut arquivos: HashMap<String, (String, String)> = HashMap::new();
    let mut grupo: Option<Id> = asm.nodes.iter().find(|n| matches!(n.kind, NodeKind::Group) && n.name == GRUPO).map(|n| n.id);

    for mut pilha in pilhas {
        // um membro por componente (o primeiro; prefere o que tem tamanho)
        let mut vistos: Vec<Id> = vec![];
        pilha.sort_by_key(|f| f.tamanho.is_none());
        pilha.retain(|f| {
            let novo = !vistos.contains(&f.comp);
            vistos.push(f.comp);
            novo
        });
        // de volta à ordem da árvore
        let ordem = |c: Id| asm.nodes.iter().position(|n| n.id == c).unwrap_or(usize::MAX);
        pilha.sort_by_key(|f| ordem(f.comp));

        if let Some(f) = pilha.iter().find(|f| f.tamanho.is_none()) {
            rel.avisos.push(format!("{}: furo Ø{} sem tamanho métrico compatível (folga {})", f.nome, num(f.diametro), nome_folga(op.folga)));
            continue;
        }
        let Some(ancora) = pilha.iter().find(|f| f.ancora) else {
            let f = &pilha[0];
            rel.avisos.push(format!("{}: furo Ø{} não casa com a folga {} (é outra folga de {})", f.nome, num(f.diametro), nome_folga(op.folga), f.tamanho.as_deref().unwrap_or("?")));
            continue;
        };
        let desig = ancora.tamanho.clone().unwrap_or_default();
        if pilha.iter().any(|f| f.tamanho.as_deref() != Some(desig.as_str())) {
            rel.avisos.push(format!("{}: furos coaxiais de tamanhos diferentes ({})", ancora.nome, pilha.iter().map(|f| f.tamanho.clone().unwrap_or_default()).collect::<Vec<_>>().join(", ")));
            continue;
        }
        let ids: Vec<Id> = pilha.iter().map(|f| f.comp).collect();

        // já tem fixador (ou outro hardware) neste eixo?
        let (p0, u) = (pilha[0].boca, pilha[0].dir);
        let ocupado = eixos_existentes.iter().any(|(c, p, d)| !ids.contains(c) && colineares(p0, u, *p, *d) && e_hardware(asm, cache, dir, *c));
        if ocupado {
            rel.pulados += 1;
            continue;
        }

        let Some(t_p) = fam_p.tamanho(&desig) else {
            rel.avisos.push(format!("{}: {} não tem o tamanho {desig}", ancora.nome, fam_p.norma));
            continue;
        };
        let passo = t_p.valor("P").unwrap_or(0.);

        // 3) geometria da pilha ao longo de u (u = sentido do furo do primeiro componente)
        let mut ini = (f64::MAX, 0usize, V3::default());
        let mut fim = (f64::MIN, 0usize, V3::default());
        for (i, f) in pilha.iter().enumerate() {
            let s0 = dot(sub(f.boca, p0), u);
            let (a, b) = (f.boca, [0, 1, 2].map(|k| f.boca[k] + f.dir[k] * f.espessura));
            let (sa, sb) = (s0, s0 + dot(f.dir, u) * f.espessura);
            let (lo, pt_lo, hi, pt_hi) = if sa <= sb { (sa, a, sb, b) } else { (sb, b, sa, a) };
            if lo < ini.0 {
                ini = (lo, i, pt_lo);
            }
            if hi > fim.0 {
                fim = (hi, i, pt_hi);
            }
        }
        let (entrada, saida) = (ini.2, fim.2);
        let pega = fim.0 - ini.0;
        let (c_ent, c_sai) = (pilha[ini.1].comp, pilha[fim.1].comp);
        let roscada = pilha.iter().any(|f| f.roscado);
        let com_porca = op.porca && !roscada;

        let esp_arruela = |on: bool| -> Option<f64> { on.then(|| fam_arr.and_then(|f| f.tamanho(&desig)).and_then(|t| t.valor("h"))).flatten() };
        let h1 = esp_arruela(op.arruela);
        let h2 = esp_arruela(op.arruela && com_porca);
        let altura_porca = if com_porca { fam_porca.and_then(|f| f.tamanho(&desig)).and_then(|t| t.valor("m")) } else { None };
        if op.arruela && h1.is_none() {
            rel.avisos.push(format!("arruela {} não existe na família {}: {} segue sem arruela", desig, op.familia_arruela, ancora.nome));
        }
        if com_porca && altura_porca.is_none() {
            rel.avisos.push(format!("porca {} não existe na família {}: {} segue sem porca", desig, op.familia_porca, ancora.nome));
        }
        let com_porca = com_porca && altura_porca.is_some();
        let (w1, w2, m) = (h1.unwrap_or(0.), if com_porca { h2.unwrap_or(0.) } else { 0. }, if com_porca { altura_porca.unwrap_or(0.) } else { 0. });

        let serie = fam_p.comprimentos_de(t_p);
        let comprimento = if roscada {
            // profundidade útil arredondada para baixo
            match serie.iter().copied().filter(|l| *l <= pega + w1 + 1e-9).last() {
                Some(l) => l,
                None => {
                    rel.avisos.push(format!("{}: profundidade útil de {} mm é menor que o menor {} {desig} da série", ancora.nome, num(pega), fam_p.norma));
                    continue;
                }
            }
        } else {
            let precisa = pega + w1 + w2 + m + 2. * passo;
            match serie.iter().copied().find(|l| *l >= precisa - 1e-9) {
                Some(l) => l,
                None => {
                    let Some(l) = serie.last().copied() else { continue };
                    rel.avisos.push(format!("{}: a pega pede L ≥ {} mm, mas {} {desig} vai só até {} mm", ancora.nome, num((precisa * 10.).round() / 10.), fam_p.norma, num(l)));
                    l
                }
            }
        };

        // 4) peças e restrições
        let mut peca = |fam: &Familia, t: &Tamanho, l: Option<f64>| -> Result<(String, String), String> { arquivo_fixador(dir, fam, t, l, &mut arquivos) };
        let (arq_parafuso, nome_parafuso) = peca(fam_p, t_p, Some(comprimento))?;
        let pasta = *grupo.get_or_insert_with(|| asm.add_group(GRUPO, None));

        let eixo_mundo = (entrada, [-u[0], -u[1], -u[2]]);
        let (cima, baixo) = (u, [-u[0], -u[1], -u[2]]);
        let ponto = |o: V3, d: V3, k: f64| [0, 1, 2].map(|i| o[i] + d[i] * k);

        // parafuso: Z = -u, com a cabeça apoiada na arruela (ou na face da peça)
        let origem_parafuso = ponto(entrada, baixo, w1);
        let id_parafuso = asm.add_component(&nome_parafuso, NodeKind::Part { path: arq_parafuso }, orientado(origem_parafuso, baixo), Some(pasta));
        // concêntrico ao furo de entrada
        concentrico(asm, c_ent, id_parafuso, eixo_mundo);
        let face_entrada = (entrada, baixo); // normal de fora da peça
        let arruela_cabeca = fam_arr.zip(h1).and_then(|(f, _)| f.tamanho(&desig).map(|t| (f, t)));
        if let Some((fam_a, t_a)) = arruela_cabeca {
            let (arq, nome) = peca(fam_a, t_a, None)?;
            let id = asm.add_component(&nome, NodeKind::Part { path: arq }, orientado(entrada, cima), Some(pasta));
            concentrico(asm, id_parafuso, id, eixo_mundo);
            apoio(asm, c_ent, face_entrada, id, (entrada, cima));
            // cabeça sobre a face de fora da arruela
            let fora = ponto(entrada, baixo, w1);
            apoio(asm, id, (fora, baixo), id_parafuso, (fora, cima));
            rel.arruelas += 1;
        } else {
            apoio(asm, c_ent, face_entrada, id_parafuso, (entrada, cima));
        }

        if let (true, Some(fam_n)) = (com_porca, fam_porca) {
            // (componente de apoio, sua face de fora, ponto da face): a peça ou a arruela da saída
            let mut apoio_porca = (c_sai, (saida, cima), saida);
            let arruela_saida = fam_arr.zip(h2).and_then(|(f, h)| f.tamanho(&desig).map(|t| (f, t, h)));
            if let Some((fam_a, t_a, h)) = arruela_saida {
                let (arq, nome) = peca(fam_a, t_a, None)?;
                let id = asm.add_component(&nome, NodeKind::Part { path: arq }, orientado(saida, baixo), Some(pasta));
                concentrico(asm, id_parafuso, id, eixo_mundo);
                apoio(asm, c_sai, (saida, cima), id, (saida, baixo));
                let fora = ponto(saida, cima, h);
                apoio_porca = (id, (fora, cima), fora);
                rel.arruelas += 1;
            }
            if let Some(t_n) = fam_n.tamanho(&desig) {
                let (arq, nome) = peca(fam_n, t_n, None)?;
                let id = asm.add_component(&nome, NodeKind::Part { path: arq }, orientado(apoio_porca.2, baixo), Some(pasta));
                concentrico(asm, id_parafuso, id, eixo_mundo);
                apoio(asm, apoio_porca.0, apoio_porca.1, id, (apoio_porca.2, baixo));
                rel.porcas += 1;
            }
        }
        rel.conta(format!("{desig}x{}", num(comprimento)));
    }
    Ok(rel)
}

/// Placement com o eixo Z do componente apontando para `z` e a origem em `origem`.
fn orientado(origem: V3, z: V3) -> Placement {
    let z = unit(z);
    let x = any_perp(z);
    Placement { origin: origem, rot: [x, cross(z, x), z] }
}

fn eixo_local(a: &Assembly, comp: Id, p: V3, d: V3) -> Geom {
    let pl = a.node(comp).map(|n| n.placement).unwrap_or_default();
    Geom::Axis { p: pl.to_local(p), d: unit(pl.dir_to_local(d)) }
}

fn plano_local(a: &Assembly, comp: Id, p: V3, n: V3) -> Geom {
    let pl = a.node(comp).map(|n| n.placement).unwrap_or_default();
    Geom::Plane { p: pl.to_local(p), n: unit(pl.dir_to_local(n)) }
}

/// Restrição concêntrica entre `a` e `b` pelo eixo (ponto, direção) do mundo.
fn concentrico(asm: &mut Assembly, a: Id, b: Id, eixo: (V3, V3)) {
    let (ga, gb) = (eixo_local(asm, a, eixo.0, eixo.1), eixo_local(asm, b, eixo.0, eixo.1));
    asm.add_mate(MateKind::Coincident { flip: false }, MateRef { comp: a, geom: ga, pick: None }, MateRef { comp: b, geom: gb, pick: None });
}

/// Contato entre a face de `a` (ponto, normal de fora do sólido) e a de `b`, de frente uma para a outra.
fn apoio(asm: &mut Assembly, a: Id, fa: (V3, V3), b: Id, fb: (V3, V3)) {
    let (ga, gb) = (plano_local(asm, a, fa.0, fa.1), plano_local(asm, b, fb.0, fb.1));
    asm.add_mate(MateKind::Contact, MateRef { comp: a, geom: ga, pick: None }, MateRef { comp: b, geom: gb, pick: None });
}

/// Grava a peça do fixador (se ainda não existe) e devolve (caminho relativo à montagem, nome).
fn arquivo_fixador(dir: &Path, fam: &Familia, t: &Tamanho, l: Option<f64>, memo: &mut HashMap<String, (String, String)>) -> Result<(String, String), String> {
    let base = match l {
        Some(l) => format!("{}_{}x{}", fam.id, t.designacao, num(l)),
        None => format!("{}_{}", fam.id, t.designacao),
    };
    if let Some(r) = memo.get(&base) {
        return Ok(r.clone());
    }
    let doc = fixador_documento(&fam.id, &t.designacao, l)?;
    let rel = format!("fixadores/{base}.fkp");
    let destino = dir.join("fixadores").join(format!("{base}.fkp"));
    if !destino.exists() {
        std::fs::create_dir_all(dir.join("fixadores")).map_err(|e| format!("{}: {e}", dir.join("fixadores").display()))?;
        std::fs::write(&destino, doc.to_json()).map_err(|e| format!("{}: {e}", destino.display()))?;
    }
    let r = (rel, doc.props.description.clone());
    memo.insert(base, r.clone());
    Ok(r)
}

/// Eixos (componente, ponto e direção no mundo) de todas as restrições ativas.
fn eixos_das_restricoes(a: &Assembly) -> Vec<(Id, V3, V3)> {
    let mut v = vec![];
    for m in a.mates.iter().filter(|m| !m.suppressed) {
        for r in [&m.a, &m.b] {
            if let (Geom::Axis { p, d }, Some(n)) = (r.geom, a.node(r.comp)) {
                v.push((r.comp, n.placement.point(p), unit(n.placement.dir(d))));
            }
        }
    }
    v
}

/// O componente é uma peça de hardware (parafuso, pino...)?
fn e_hardware(a: &Assembly, cache: &mut PartCache, dir: &Path, comp: Id) -> bool {
    let Some(NodeKind::Part { path }) = a.node(comp).map(|n| n.kind.clone()) else { return false };
    cache.get(&full_path(Some(dir), &path), false).is_ok_and(|p| p.doc.props.part_type == PartType::Hardware)
}

/// Eixos (ponto e direção) na mesma reta: paralelos ou antiparalelos até 0,5° e a menos de 0,05 mm.
fn colineares(pa: V3, da: V3, pb: V3, db: V3) -> bool {
    if norm(cross(da, db)) > TOL_ANGULO.to_radians().sin() {
        return false;
    }
    let d = sub(pb, pa);
    let k = dot(d, da);
    norm([0, 1, 2].map(|i| d[i] - da[i] * k)) <= TOL_EIXO
}

/// Tamanho cuja folga escolhida casa com o diâmetro do furo.
fn casa_folga(diametro: f64, folga: u8) -> Option<&'static str> {
    hardware::METRIC.iter().find(|m| (m.clearance[folga.min(2) as usize] - diametro).abs() <= TOL_DIAMETRO).map(|m| m.name)
}

/// Tamanho cuja folga (qualquer uma das três) casa com o diâmetro do furo.
fn casa_outra_folga(diametro: f64) -> Option<&'static str> {
    hardware::METRIC.iter().find(|m| m.clearance.iter().any(|c| (c - diametro).abs() <= TOL_DIAMETRO)).map(|m| m.name)
}

/// "M6x1", "M8" → "M6", "M8" (só se o tamanho existe na tabela métrica).
fn tamanho_da_rosca(rosca: &str) -> Option<String> {
    let r = rosca.trim().strip_prefix(['M', 'm'])?;
    let d: f64 = r.chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect::<String>().parse().ok()?;
    hardware::METRIC.iter().find(|m| m.name[1..].parse::<f64>().is_ok_and(|x| (x - d).abs() < 1e-6)).map(|m| m.name.to_string())
}

/// Distância da boca até a saída do furo, ao longo de `dir` (no sistema local da peça).
/// Sonda a peça com retas paralelas ao eixo, logo ao lado do furo, vindas de fora pelo lado da
/// saída; a mediana dos acertos vence peças com rebaixo ou chanfro num dos lados.
fn espessura_ao_longo(forma: &Shape, boca: V3, dir: V3, diametro: f64) -> Option<f64> {
    let dir = unit(dir);
    let (lo, hi) = forma.bbox()?;
    let mut longe = f64::MIN;
    for c in 0..8 {
        let q = [if c & 1 == 0 { lo[0] } else { hi[0] }, if c & 2 == 0 { lo[1] } else { hi[1] }, if c & 4 == 0 { lo[2] } else { hi[2] }];
        longe = longe.max(dot(sub(q, boca), dir));
    }
    if longe <= 1e-6 {
        return None;
    }
    let (e1, e2) = (any_perp(dir), cross(dir, any_perp(dir)));
    let mut s: Vec<f64> = vec![];
    for folga in [0.4, 0.9, 1.6] {
        for v in [e1, e2, e1.map(|x| -x), e2.map(|x| -x)] {
            let r = diametro / 2. + folga;
            let q: V3 = [0, 1, 2].map(|i| boca[i] + v[i] * r + dir[i] * (longe + 1.));
            if let Ok(Some((pt, _))) = forma.ray_hit(q, dir.map(|x| -x)) {
                let d = dot(sub(pt, boca), dir);
                if d > 0.05 {
                    s.push(d);
                }
            }
        }
    }
    if s.is_empty() {
        // sem acerto (peça fina demais ao redor): a caixa envolvente
        return Some(longe);
    }
    s.sort_by(|a, b| a.total_cmp(b));
    Some(s[s.len() / 2])
}

fn nome_folga(folga: u8) -> &'static str {
    ["fina", "normal", "folgada"][folga.min(2) as usize]
}

/// 35.0 → "35", 1.6 → "1.6"
fn num(x: f64) -> String {
    if (x - x.round()).abs() < 1e-9 {
        format!("{}", x.round() as i64)
    } else {
        format!("{x}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fenkra_core::{sketch::Curve, BoolOp, Document, Extent, Extrude, FeatureKind, Hole, HoleKind, OriginPlane, PlaneRef, Sketch, SketchData};

    /// Placa de 60 × 60 com `esp` mm de espessura (z de 0 a esp) e um furo de Ø`diametro` no centro,
    /// aberto pelo topo. `roscado`: furo cego com rosca "M6" de `prof` mm; sem ele, passante.
    fn placa(esp: f64, diametro: f64, roscado: Option<f64>) -> Document {
        let mut d = Document::new("Placa");
        let mut n = 7_000_000u64;
        let mut al = || {
            n += 1;
            Id(n)
        };
        let mut base = SketchData::default();
        base.add_rect(&mut al, [-30., -30.], [30., 30.], false);
        let sb = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data: base }));
        d.add_feature(FeatureKind::Extrude(Extrude { sketch: sb, op: BoolOp::Add, extent: Extent::Blind(num(esp)), reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0 }));
        let topo = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), &num(esp));
        let mut centro = SketchData::default();
        let p = centro.add_point(&mut al, 0., 0.);
        centro.add_curve(&mut al, Curve::Point { p }, false);
        let sc = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Datum(topo), data: centro }));
        let (depth, thread) = match roscado {
            Some(prof) => (Extent::Blind(num(prof)), Some("M6".to_string())),
            None => (Extent::ThroughAll, None),
        };
        d.add_feature(FeatureKind::Hole(Hole { sketch: sc, kind: HoleKind::Simples, diameter: num(diametro), depth, thread, thread_modeled: false, standard: None, points: vec![], dir: None, reverse: false, tip: None, taper: "0".into(), left_hand: false }));
        d
    }

    fn pasta(nome: &str) -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!("fenkra_fixadores_{nome}"));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    /// Duas placas iguais empilhadas (a de baixo é a fixa; a de cima começa em z = 10).
    fn pilha_de_placas(dir: &Path, doc: &Document) -> (Assembly, Id, Id) {
        std::fs::write(dir.join("placa.fkp"), doc.to_json()).unwrap();
        let mut a = Assembly::new("Pilha");
        let baixo = a.add_component("Placa 1", NodeKind::Part { path: "placa.fkp".into() }, Placement::default(), None);
        let cima = a.add_component("Placa 2", NodeKind::Part { path: "placa.fkp".into() }, Placement { origin: [0., 0., 10.], ..Default::default() }, None);
        (a, baixo, cima)
    }

    fn pos(a: &Assembly, sufixo: &str) -> Placement {
        a.nodes.iter().find(|n| matches!(&n.kind, NodeKind::Part { path } if path.ends_with(sufixo))).unwrap_or_else(|| panic!("sem {sufixo}")).placement
    }

    #[test]
    fn duas_placas_um_parafuso_com_arruelas_e_porca() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let dir = pasta("pilha");
        let (mut a, _, _) = pilha_de_placas(&dir, &placa(10., 6.4, None));
        let mut cache = PartCache::default();
        let op = OpcoesFixadores { folga: 0, ..Default::default() };
        let rel = preencher_furos(&mut a, Some(&dir), &mut cache, &op).unwrap();
        // pega 20 + 1,6 + 1,6 + 5,2 + 2 = 30,4 → próximo L padrão: 35
        assert_eq!(rel.fixadores, 1, "{rel:?}");
        assert_eq!(rel.por_tamanho, vec![("M6x35".to_string(), 1)]);
        assert_eq!((rel.arruelas, rel.porcas, rel.pulados), (2, 1, 0));
        assert!(rel.avisos.is_empty(), "{:?}", rel.avisos);
        for f in ["iso4762_M6x35.fkp", "iso7089_M6.fkp", "iso4032_M6.fkp"] {
            assert!(dir.join("fixadores").join(f).exists(), "{f}");
        }
        assert_eq!(a.nodes.iter().filter(|n| matches!(&n.kind, NodeKind::Part { path } if path.starts_with("fixadores/"))).count(), 4);

        let rep = solve::solve(&mut a);
        assert!(rep.converged, "{rep:?}");
        // parafuso no eixo do furo (x = y = 0), cabeça para cima (o Z local aponta para a cabeça e o
        // corpo desce), apoiada na arruela que está sobre a face (z = 20)
        let p = pos(&a, "iso4762_M6x35.fkp");
        assert!(p.origin[0].hypot(p.origin[1]) < 0.01, "{:?}", p.origin);
        assert!(norm(sub(p.dir([0., 0., 1.]), [0., 0., 1.])) < 1e-6);
        assert!((p.origin[2] - 21.6).abs() < 0.01, "cabeça: {:?}", p.origin);
        // a porca fica sob a placa 1 (z = 0), depois da arruela de 1,6
        let porca = pos(&a, "iso4032_M6.fkp");
        assert!(porca.origin[0].hypot(porca.origin[1]) < 0.01);
        assert!((porca.origin[2] + 1.6).abs() < 0.01, "porca: {:?}", porca.origin);

        // chamar de novo não duplica
        let antes = a.nodes.len();
        let de_novo = preencher_furos(&mut a, Some(&dir), &mut cache, &op).unwrap();
        assert_eq!((de_novo.fixadores, de_novo.pulados), (0, 1), "{de_novo:?}");
        assert_eq!(a.nodes.len(), antes);
    }

    #[test]
    fn placa_de_cima_virada_ainda_forma_a_pilha() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let dir = pasta("virada");
        let (mut a, _, cima) = pilha_de_placas(&dir, &placa(10., 6.6, None));
        // placa 2 de cabeça para baixo (ainda ocupa z de 10 a 20, mas o furo agora entra por baixo)
        a.node_mut(cima).unwrap().placement = Placement::default().moved([0., 0., 20.], [std::f64::consts::PI, 0., 0.], [0.; 3]);
        let mut cache = PartCache::default();
        let rel = preencher_furos(&mut a, Some(&dir), &mut cache, &OpcoesFixadores::default()).unwrap();
        assert_eq!(rel.por_tamanho, vec![("M6x35".to_string(), 1)], "{rel:?}");
        assert!(solve::solve(&mut a).converged);
        // a entrada é a face de cima da placa 2 (z = 20) + arruela de 1,6; a porca sai por baixo da placa 1
        let p = pos(&a, "iso4762_M6x35.fkp");
        assert!(p.origin[0].hypot(p.origin[1]) < 0.01 && (p.origin[2] - 21.6).abs() < 0.01, "{:?}", p.origin);
        assert!((pos(&a, "iso4032_M6.fkp").origin[2] + 1.6).abs() < 0.01);
    }

    #[test]
    fn sem_arruela_nem_porca_e_so_nos_escolhidos() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let dir = pasta("simples");
        let (mut a, baixo, _) = pilha_de_placas(&dir, &placa(10., 6.6, None));
        let mut cache = PartCache::default();
        // só a placa de baixo: pilha de uma peça, pega 10 + 2 passos = 12
        let op = OpcoesFixadores { arruela: false, porca: false, componentes: vec![baixo], ..Default::default() };
        let rel = preencher_furos(&mut a, Some(&dir), &mut cache, &op).unwrap();
        assert_eq!((rel.fixadores, rel.arruelas, rel.porcas), (1, 0, 0), "{rel:?}");
        assert_eq!(rel.por_tamanho, vec![("M6x12".to_string(), 1)]);
        assert!(solve::solve(&mut a).converged);
        // cabeça direto na face de cima da placa 1 (z = 10)
        let p = pos(&a, "M6x12.fkp");
        assert!(p.origin[0].hypot(p.origin[1]) < 0.01 && (p.origin[2] - 10.).abs() < 0.01, "{:?}", p.origin);
    }

    #[test]
    fn furo_roscado_cego_leva_parafuso_sem_porca() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let dir = pasta("roscado");
        let mut a = Assembly::new("Rosca");
        std::fs::write(dir.join("bloco.fkp"), placa(30., 5., Some(18.)).to_json()).unwrap();
        a.add_component("Bloco", NodeKind::Part { path: "bloco.fkp".into() }, Placement::default(), None);
        let mut cache = PartCache::default();
        let rel = preencher_furos(&mut a, Some(&dir), &mut cache, &OpcoesFixadores::default()).unwrap();
        // profundidade 18 + arruela 1,6 = 19,6 → arredonda para baixo: L16
        assert_eq!((rel.fixadores, rel.arruelas, rel.porcas), (1, 1, 0), "{rel:?}");
        assert_eq!(rel.por_tamanho, vec![("M6x16".to_string(), 1)]);
    }

    #[test]
    fn furo_sem_tamanho_compativel_avisa() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let dir = pasta("aviso");
        let (mut a, _, _) = pilha_de_placas(&dir, &placa(10., 7.3, None));
        let mut cache = PartCache::default();
        let rel = preencher_furos(&mut a, Some(&dir), &mut cache, &OpcoesFixadores::default()).unwrap();
        assert_eq!(rel.fixadores, 0);
        assert_eq!(rel.avisos.len(), 1, "{:?}", rel.avisos);
        assert!(rel.avisos[0].contains("7.3"), "{:?}", rel.avisos);
        assert!(preencher_furos(&mut a, None, &mut cache, &OpcoesFixadores::default()).is_err(), "sem pasta não grava");
    }

    #[test]
    fn auxiliares_de_tamanho_e_eixo() {
        assert_eq!(casa_folga(6.6, 1), Some("M6"));
        assert_eq!(casa_folga(6.6, 0), None);
        assert_eq!(casa_folga(6.64, 1), Some("M6"));
        assert_eq!(casa_outra_folga(7.0), Some("M6"));
        assert_eq!(casa_outra_folga(7.3), None);
        assert_eq!(tamanho_da_rosca("M8x1.25").as_deref(), Some("M8"));
        assert_eq!(tamanho_da_rosca("M6").as_deref(), Some("M6"));
        assert_eq!(tamanho_da_rosca("M7"), None);
        assert_eq!(tamanho_da_rosca("G1/4"), None);
        // colinear: sentido contrário também vale; 0,1 mm de desvio ou 1° não
        assert!(colineares([0., 0., 0.], [0., 0., 1.], [0., 0., 30.], [0., 0., -1.]));
        assert!(!colineares([0., 0., 0.], [0., 0., 1.], [0.1, 0., 5.], [0., 0., 1.]));
        let g = 1_f64.to_radians();
        assert!(!colineares([0., 0., 0.], [0., 0., 1.], [0., 0., 5.], [g.sin(), 0., g.cos()]));
        assert_eq!(num(35.), "35");
        assert_eq!(num(1.6), "1.6");
        // o eixo Z da orientação aponta para onde se pediu e os eixos são ortonormais
        let p = orientado([1., 2., 3.], [0., 1., 0.]);
        assert!(norm(sub(p.dir([0., 0., 1.]), [0., 1., 0.])) < 1e-12);
        assert!(norm(sub(cross(p.rot[0], p.rot[1]), p.rot[2])) < 1e-12);
    }
}
