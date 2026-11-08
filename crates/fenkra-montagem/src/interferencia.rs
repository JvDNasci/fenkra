// SPDX-License-Identifier: LGPL-2.1-or-later
//! Análise de interferências (B264). Sem interface: quem desenha é `fenkra-app/asm_interf.rs`.
//!
//! Como funciona no kernel:
//! 1. cada peça vira uma **unidade** com as formas já no mundo. Um subconjunto é uma unidade só (as
//!    peças dele não se comparam entre si) ou se abre em uma unidade por peça, conforme a opção;
//! 2. para cada par do escopo, as caixas envolventes filtram o que está longe; depois as malhas das
//!    peças (as mesmas da vista) descartam o par cujas caixas se cruzam mas as peças não chegam a
//!    2 × desvio da malha uma da outra, nem uma está dentro da outra (B394: a booleana é o passo
//!    caro, e num quadro a caixa de um tubo cobre meio conjunto). Só então a operação booleana
//!    Comum devolve o sólido compartilhado. O volume dele é a medida da interferência, e a malha dele
//!    é o que aparece em vermelho na vista. Os pares que sobram vão todos de uma vez para
//!    `Shape::comuns`, que faz as booleanas em várias threads, cada uma sobre cópias próprias das
//!    formas, e sem unificar as faces do comum (o volume e a malha não precisam disso);
//! 3. duas faces encostadas têm volume comum zero, então o Comum não enxerga coincidência. Para isso
//!    existe a seção entre as formas, que devolve as linhas de contato: com a opção ligada, seção não
//!    vazia sem volume vira um achado de volume zero;
//! 4. o que a pessoa decidiu ignorar fica em [`RegrasInterf`], dentro da montagem. A análise sempre
//!    acha tudo; [`separar`] só reparte a lista, e por isso ignorar não exige analisar de novo;
//! 5. prendedor por ponto ou vetor atravessa as peças de propósito: o furo será feito na hora.
//!    [`separar_na_montagem`] tira da lista só o que cabe no corpo do parafuso. O que encosta fora dele
//!    (a cabeça numa peça vizinha, por exemplo) segue sendo problema.

use crate::load::{full_path, PartCache};
use crate::view::{placed_shape, posicionado, shown};
use crate::*;
use fenkra_kernel::{Mesh, Shape};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Volume comum mínimo para contar, em mm³: abaixo disso é ruído da operação booleana.
pub const TOL_PADRAO: f64 = 0.01;

/// Par que a pessoa decidiu ignorar, com o motivo. Os lados ficam em ordem para o par valer nos dois
/// sentidos.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct ParIgnorado {
    pub a: String,
    pub b: String,
    #[serde(default)]
    pub motivo: String,
}

/// Decisões da análise guardadas na montagem.
#[derive(Clone, PartialEq, Debug, Default, Serialize, Deserialize)]
pub struct RegrasInterf {
    #[serde(default)]
    pub ignorados: Vec<ParIgnorado>,
    /// volume abaixo do qual o achado sai da lista principal (mm³); 0 desliga
    #[serde(default)]
    pub menores_que: f64,
}

fn em_ordem<'a>(a: &'a str, b: &'a str) -> (&'a str, &'a str) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}

impl RegrasInterf {
    pub fn ignorado(&self, a: &str, b: &str) -> Option<&ParIgnorado> {
        let (a, b) = em_ordem(a, b);
        self.ignorados.iter().find(|p| p.a == a && p.b == b)
    }

    pub fn ignorar(&mut self, a: &str, b: &str, motivo: &str) {
        self.restaurar(a, b);
        let (a, b) = em_ordem(a, b);
        self.ignorados.push(ParIgnorado { a: a.to_string(), b: b.to_string(), motivo: motivo.trim().to_string() });
    }

    pub fn restaurar(&mut self, a: &str, b: &str) {
        let (a, b) = em_ordem(a, b);
        self.ignorados.retain(|p| !(p.a == a && p.b == b));
    }
}

/// O que entra na análise.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Escopo {
    #[default]
    Toda,
    /// pares com pelo menos uma das peças escolhidas
    ContraAsDemais,
    /// só pares em que as duas foram escolhidas
    EntreEscolhidas,
}

#[derive(Clone, PartialEq, Debug)]
pub struct OpcoesInterf {
    pub escopo: Escopo,
    pub escolhidas: Vec<Id>,
    /// faces encostadas, sem volume comum, também contam
    pub coincidencia: bool,
    /// o subconjunto conta como uma peça: o que há dentro dele não se compara
    pub sub_unico: bool,
    /// volume comum mínimo (mm³)
    pub tol: f64,
}

impl Default for OpcoesInterf {
    fn default() -> Self {
        OpcoesInterf { escopo: Escopo::Toda, escolhidas: vec![], coincidencia: false, sub_unico: true, tol: TOL_PADRAO }
    }
}

/// Um lado de um achado.
#[derive(Clone, PartialEq, Debug)]
pub struct Lado {
    /// nó da montagem aberta: a peça, ou o subconjunto que a contém
    pub comp: Id,
    /// identidade estável do lado: o id do nó, mais o caminho dentro do subconjunto
    pub chave: String,
    pub nome: String,
}

/// Uma interferência encontrada.
#[derive(Clone, Debug)]
pub struct Achado {
    pub a: Lado,
    pub b: Lado,
    /// volume comum (mm³); zero na coincidência
    pub volume: f64,
    /// sólido comum, no mundo
    pub malha: Mesh,
    /// linhas de contato da coincidência, no mundo
    pub contato: Vec<Vec<V3>>,
    /// caixa do que há em comum
    pub caixa: Option<(V3, V3)>,
}

impl Achado {
    pub fn coincidente(&self) -> bool {
        self.volume <= 0.
    }
}

/// Peça (ou subconjunto) com as formas no mundo.
struct Unidade {
    lado: Lado,
    formas: Vec<Shape>,
    /// caixa de cada forma, medida uma vez (B394): medir a cada par custava N² chamadas ao kernel
    caixas: Vec<Option<(V3, V3)>>,
    /// malha de cada forma, para o filtro antes da booleana
    grossas: Vec<Option<Grossa>>,
}

impl Unidade {
    /// Caixa de tudo o que a unidade tem.
    fn caixa(&self) -> Option<(V3, V3)> {
        let mut c = None;
        for b in self.caixas.iter().flatten() {
            unir_caixa(&mut c, *b);
        }
        c
    }
}

/// Caixa e malha de cada forma de todas as unidades, em threads (B394). As formas saem de
/// `posicionado`, que copia a peça sem a malha: tesselar é o custo aqui, e cada thread tesselava uma
/// cópia própria.
fn medir(us: &mut [Unidade], q: &fenkra_core::QualidadeMalha, threads: usize) {
    let medidas = {
        let formas: Vec<&Shape> = us.iter().flat_map(|u| u.formas.iter()).collect();
        Shape::em_threads(&formas, threads, |_, s| (s.bbox(), grossa(&s, q)))
    };
    let mut it = medidas.into_iter();
    for u in us.iter_mut() {
        (u.caixas, u.grossas) = it.by_ref().take(u.formas.len()).unzip();
    }
}

/// Malha de uma forma para descartar, sem a booleana, os pares que nem chegam perto (B394). A
/// malha fica a no máximo `desvio` da superfície exata: se as malhas estão a mais que
/// 2 × desvio uma da outra e nenhuma está dentro da outra, os sólidos não se tocam.
struct Grossa {
    tris: Vec<[V3; 3]>,
    caixas: Vec<(V3, V3)>,
    caixa: (V3, V3),
    desvio: f64,
}

fn grossa(s: &Shape, q: &fenkra_core::QualidadeMalha) -> Option<Grossa> {
    let (lo, hi) = s.bbox()?;
    let desvio = q.desvio(norm(sub(hi, lo)));
    // a mesma qualidade da vista; o desvio fica na `Grossa` para a margem do filtro
    let m = fenkra_solidos::tesselar(s, q).ok()?;
    let p = |i: u32| {
        let k = i as usize * 3;
        [m.positions[k] as f64, m.positions[k + 1] as f64, m.positions[k + 2] as f64]
    };
    let tris: Vec<[V3; 3]> = m.indices.chunks_exact(3).map(|t| [p(t[0]), p(t[1]), p(t[2])]).collect();
    if tris.is_empty() {
        return None;
    }
    let caixas = tris.iter().map(|t| ([0, 1, 2].map(|i| t[0][i].min(t[1][i]).min(t[2][i])), [0, 1, 2].map(|i| t[0][i].max(t[1][i]).max(t[2][i])))).collect();
    Some(Grossa { tris, caixas, caixa: (lo, hi), desvio })
}

/// Alguma parte de `a` fica a menos de `m` de `b`? Conservador: compara as caixas dos triângulos,
/// que nunca ficam mais longe que os próprios triângulos. Grade de células só com o que cai na
/// região comum das duas peças.
fn perto(a: &Grossa, b: &Grossa, m: f64) -> bool {
    let toca = |x: &(V3, V3), y: &(V3, V3)| (0..3).all(|k| x.0[k] - m <= y.1[k] && y.0[k] - m <= x.1[k]);
    let regiao = ([0, 1, 2].map(|k| a.caixa.0[k].max(b.caixa.0[k]) - m), [0, 1, 2].map(|k| a.caixa.1[k].min(b.caixa.1[k]) + m));
    if (0..3).any(|k| regiao.0[k] > regiao.1[k]) {
        return false;
    }
    let na: Vec<&(V3, V3)> = a.caixas.iter().filter(|c| toca(c, &regiao)).collect();
    let nb: Vec<&(V3, V3)> = b.caixas.iter().filter(|c| toca(c, &regiao)).collect();
    if na.is_empty() || nb.is_empty() {
        return false;
    }
    // célula do tamanho do maior lado médio dos triângulos de `a`, nunca menor que a margem
    let lado = na.iter().map(|c| (0..3).map(|k| c.1[k] - c.0[k]).fold(0., f64::max)).sum::<f64>() / na.len() as f64;
    let cel = lado.max(2. * m).max(1e-3);
    let idx = |x: f64, k: usize| ((x - regiao.0[k]) / cel).floor() as i64;
    let mut grade: HashMap<(i64, i64, i64), Vec<usize>> = HashMap::new();
    for (i, c) in na.iter().enumerate() {
        let (l, h) = ([0, 1, 2].map(|k| idx(c.0[k] - m, k)), [0, 1, 2].map(|k| idx(c.1[k] + m, k)));
        // triângulo enorme diante da célula: vai para todas as que cobre, até um teto
        if (0..3).map(|k| (h[k] - l[k] + 1) as u64).product::<u64>() > 4096 {
            return true;
        }
        for x in l[0]..=h[0] {
            for y in l[1]..=h[1] {
                for z in l[2]..=h[2] {
                    grade.entry((x, y, z)).or_default().push(i);
                }
            }
        }
    }
    nb.iter().any(|c| {
        let (l, h) = ([0, 1, 2].map(|k| idx(c.0[k], k)), [0, 1, 2].map(|k| idx(c.1[k], k)));
        if (0..3).map(|k| (h[k] - l[k] + 1) as u64).product::<u64>() > 4096 {
            return true;
        }
        (l[0]..=h[0]).any(|x| (l[1]..=h[1]).any(|y| (l[2]..=h[2]).any(|z| grade.get(&(x, y, z)).is_some_and(|v| v.iter().any(|i| toca(na[*i], c))))))
    })
}

/// `a` está inteira dentro de `b`? Só pode estar se a caixa couber; aí conta quantas vezes um raio
/// saindo de um vértice de `a` atravessa `b` (ímpar = dentro).
fn dentro(a: &Grossa, b: &Grossa) -> bool {
    if (0..3).any(|k| a.caixa.0[k] < b.caixa.0[k] || a.caixa.1[k] > b.caixa.1[k]) {
        return false;
    }
    let o = a.tris[0][0];
    // direção torta, para não passar exatamente por arestas de peças alinhadas aos eixos
    let d = unit([0.5773, 0.3137, 0.7539]);
    let cruza = |t: &[V3; 3]| {
        let (e1, e2) = (sub(t[1], t[0]), sub(t[2], t[0]));
        let pv = cross(d, e2);
        let det = dot(e1, pv);
        if det.abs() < 1e-14 {
            return false;
        }
        let tv = sub(o, t[0]);
        let u = dot(tv, pv) / det;
        let qv = cross(tv, e1);
        let v = dot(d, qv) / det;
        u >= 0. && v >= 0. && u + v <= 1. && dot(e2, qv) / det > 1e-9
    };
    b.tris.iter().filter(|t| cruza(t)).count() % 2 == 1
}

/// As caixas ficam separadas por mais que `folga` em algum eixo?
fn longe(a: Option<(V3, V3)>, b: Option<(V3, V3)>, folga: f64) -> bool {
    let (Some((la, ha)), Some((lb, hb))) = (a, b) else { return false };
    (0..3).any(|k| ha[k] + folga < lb[k] || hb[k] + folga < la[k])
}

/// Formas das peças de um subconjunto, no sistema de quem o contém.
fn formas_do_sub(path: &str, flex: &[crate::flexivel::PosicaoFlex], dir: Option<&Path>, cache: &mut PartCache, pl: Placement, prefixo: &str, fundo: u32) -> Vec<(String, String, Shape)> {
    if fundo > 16 {
        return vec![];
    }
    let full = full_path(dir, path);
    let Some(mut sub) = std::fs::read_to_string(&full).ok().and_then(|t| Assembly::from_json(&t).ok()) else { return vec![] };
    crate::flexivel::aplicar(&mut sub, flex);
    let sub_dir = full.parent().map(|x| x.to_path_buf());
    let mut out = vec![];
    for n in sub.nodes.iter().filter(|n| shown(&sub, n.id) && n.mode == LoadMode::Editavel) {
        let chave = format!("{prefixo}/{}", n.id.0);
        match &n.kind {
            NodeKind::Part { .. } => {
                if let Some(s) = placed_shape(&sub, n.id, sub_dir.as_deref(), cache).and_then(|s| posicionado(s, &pl)) {
                    out.push((chave, n.name.clone(), s));
                }
            }
            NodeKind::SubAssembly { path } => {
                for (c, nome, s) in formas_do_sub(path, &[], sub_dir.as_deref(), cache, n.placement, &chave, fundo + 1) {
                    if let Some(s) = posicionado(s, &pl) {
                        out.push((c, format!("{} / {nome}", n.name), s));
                    }
                }
            }
            NodeKind::Group => {}
        }
    }
    out
}

fn unidades(a: &Assembly, dir: Option<&Path>, cache: &mut PartCache, sub_unico: bool) -> Vec<Unidade> {
    let mut v = vec![];
    for n in a.nodes.iter().filter(|n| n.mode == LoadMode::Editavel && shown(a, n.id)) {
        let lado = |chave: String, nome: String| Lado { comp: n.id, chave, nome };
        match &n.kind {
            NodeKind::Group => {}
            NodeKind::Part { .. } => {
                if let Some(s) = placed_shape(a, n.id, dir, cache) {
                    v.push(Unidade { lado: lado(n.id.0.to_string(), n.name.clone()), formas: vec![s], caixas: vec![], grossas: vec![] });
                }
            }
            NodeKind::SubAssembly { path } => {
                let chave = n.id.0.to_string();
                let pecas = formas_do_sub(path, &a.flex_de(n.id), dir, cache, n.placement, &chave, 0);
                if sub_unico {
                    if !pecas.is_empty() {
                        v.push(Unidade { lado: lado(chave, n.name.clone()), formas: pecas.into_iter().map(|p| p.2).collect(), caixas: vec![], grossas: vec![] });
                    }
                } else {
                    for (c, nome, s) in pecas {
                        v.push(Unidade { lado: lado(c, format!("{} / {nome}", n.name)), formas: vec![s], caixas: vec![], grossas: vec![] });
                    }
                }
            }
        }
    }
    v
}

fn no_escopo(op: &OpcoesInterf, a: Id, b: Id) -> bool {
    let (ea, eb) = (op.escolhidas.contains(&a), op.escolhidas.contains(&b));
    match op.escopo {
        Escopo::Toda => true,
        Escopo::ContraAsDemais => ea || eb,
        Escopo::EntreEscolhidas => ea && eb,
    }
}

fn acrescentar(m: &mut Mesh, c: &Mesh) {
    let base = (m.positions.len() / 3) as u32;
    m.positions.extend_from_slice(&c.positions);
    m.normals.extend_from_slice(&c.normals);
    m.indices.extend(c.indices.iter().map(|i| i + base));
}

fn unir_caixa(c: &mut Option<(V3, V3)>, n: (V3, V3)) {
    *c = Some(match *c {
        None => n,
        Some((lo, hi)) => ([0, 1, 2].map(|i| lo[i].min(n.0[i])), [0, 1, 2].map(|i| hi[i].max(n.1[i]))),
    });
}

/// Par de formas que passou pelos filtros e vai para a booleana: unidades `i` e `j`, formas `fa` e `fb`.
struct Candidato {
    i: usize,
    j: usize,
    fa: usize,
    fb: usize,
}

/// Pares de formas de duas unidades que podem se tocar. `folga` deixa passar caixas que só se
/// tocam, para a coincidência.
fn candidatos(a: &Unidade, b: &Unidade, i: usize, j: usize, folga: f64, out: &mut Vec<Candidato>) {
    for (fa, ca) in a.caixas.iter().enumerate() {
        for (fb, cb) in b.caixas.iter().enumerate() {
            if longe(*ca, *cb, folga) {
                continue;
            }
            // a booleana é o passo caro: as malhas descartam antes o par que nem chega perto
            if let (Some(Some(ga)), Some(Some(gb))) = (a.grossas.get(fa), b.grossas.get(fb)) {
                if !perto(ga, gb, ga.desvio + gb.desvio + folga + 1e-3) && !dentro(ga, gb) && !dentro(gb, ga) {
                    continue;
                }
            }
            out.push(Candidato { i, j, fa, fb });
        }
    }
}

/// O que sai da booleana de um candidato: volume, malha e caixa do comum.
type Comum = Option<(f64, Mesh, Option<(V3, V3)>)>;

/// Todos os achados do escopo, do maior volume para o menor. Ignorados não saem daqui: ver [`separar`].
///
/// As booleanas rodam em várias threads ([`Shape::comuns`], B394), sem unificar as faces: só o volume e
/// a malha do comum interessam. A ordem e a soma dos volumes são as de antes, par por par.
pub fn analisar(a: &Assembly, dir: Option<&Path>, cache: &mut PartCache, op: &OpcoesInterf) -> Vec<Achado> {
    analisar_em(a, dir, cache, op, 0)
}

/// [`analisar`] com o número de threads das booleanas (0 = uma por núcleo, 1 = tudo nesta thread).
pub fn analisar_em(a: &Assembly, dir: Option<&Path>, cache: &mut PartCache, op: &OpcoesInterf, threads: usize) -> Vec<Achado> {
    let q = cache.qualidade();
    // só a forma: a malha de vista que o `get` faria de cada peça não serve aqui (a análise tessela a
    // forma posicionada), e no kart ela era boa parte da primeira análise
    let so_forma = std::mem::replace(&mut cache.so_forma, true);
    let mut us = unidades(a, dir, cache, op.sub_unico);
    cache.so_forma = so_forma;
    medir(&mut us, &q, threads);
    let caixas: Vec<_> = us.iter().map(Unidade::caixa).collect();
    let folga = if op.coincidencia { 1e-6 } else { 0. };
    let mut cands = vec![];
    for i in 0..us.len() {
        for j in i + 1..us.len() {
            let (ua, ub) = (&us[i], &us[j]);
            // dentro do mesmo subconjunto aberto em peças, compara normalmente; mesma chave nunca
            if ua.lado.chave == ub.lado.chave || !no_escopo(op, ua.lado.comp, ub.lado.comp) || longe(caixas[i], caixas[j], folga) {
                continue;
            }
            candidatos(ua, ub, i, j, folga, &mut cands);
        }
    }
    // os pares mais caros primeiro: no kart, três pares levam 2 s cada, e se ficassem para o fim
    // as outras threads esperariam paradas. O custo cresce com os triângulos das duas peças.
    let custo = |c: &Candidato| [(c.i, c.fa), (c.j, c.fb)].iter().map(|(u, f)| us[*u].grossas[*f].as_ref().map_or(0, |g| g.tris.len())).sum::<usize>();
    let mut ordem: Vec<usize> = (0..cands.len()).collect();
    ordem.sort_by_key(|k| std::cmp::Reverse(custo(&cands[*k])));
    let pares: Vec<(&Shape, &Shape)> = ordem.iter().map(|k| (&us[cands[*k].i].formas[cands[*k].fa], &us[cands[*k].j].formas[cands[*k].fb])).collect();
    let tol = op.tol;
    let feitos: Vec<Comum> = Shape::comuns(&pares, threads, |_, r| {
        let c = r.ok()?;
        let v = c.mass().volume;
        (v > tol).then(|| (v, fenkra_solidos::tesselar(&c, &q).unwrap_or_default(), c.bbox()))
    });
    let mut comuns: Vec<Comum> = (0..cands.len()).map(|_| None).collect();
    for (k, c) in ordem.into_iter().zip(feitos) {
        comuns[k] = c;
    }
    let mut out: Vec<Achado> = vec![];
    let mut par_atual = None;
    for (c, comum) in cands.iter().zip(comuns) {
        // os candidatos de um par de unidades vêm juntos: um achado por par
        if par_atual != Some((c.i, c.j)) {
            par_atual = Some((c.i, c.j));
            out.push(Achado { a: us[c.i].lado.clone(), b: us[c.j].lado.clone(), volume: 0., malha: Mesh::default(), contato: vec![], caixa: None });
        }
        let x = out.last_mut().expect("achado do par");
        match comum {
            Some((v, m, bb)) => {
                x.volume += v;
                acrescentar(&mut x.malha, &m);
                if let Some(bb) = bb {
                    unir_caixa(&mut x.caixa, bb);
                }
            }
            None if op.coincidencia => {
                for linha in pares_secao(&us[c.i].formas[c.fa], &us[c.j].formas[c.fb]) {
                    for p in &linha {
                        unir_caixa(&mut x.caixa, (*p, *p));
                    }
                    x.contato.push(linha);
                }
            }
            None => {}
        }
    }
    out.retain(|x| x.volume > 0. || !x.contato.is_empty());
    out.sort_by(|x, y| y.volume.total_cmp(&x.volume));
    out
}

/// Linhas de contato de duas formas sem volume comum.
fn pares_secao(a: &Shape, b: &Shape) -> Vec<Vec<V3>> {
    a.section(b).unwrap_or_default()
}

/// Índices dos achados, repartidos pelas regras da montagem.
#[derive(Clone, Default, PartialEq, Debug)]
pub struct Reparticao {
    pub ativos: Vec<usize>,
    /// ignorados um a um, com o motivo
    pub ignorados: Vec<(usize, String)>,
    /// abaixo do limite "ignorar menores que"
    pub menores: Vec<usize>,
}

/// Reparte a lista sem refazer a análise. A coincidência (volume zero) não cai em "menores que": ela
/// só aparece quando foi pedida, e o limite existe para o ruído de volume.
pub fn separar(achados: &[Achado], r: &RegrasInterf) -> Reparticao {
    let mut s = Reparticao::default();
    for (i, x) in achados.iter().enumerate() {
        if let Some(p) = r.ignorado(&x.a.chave, &x.b.chave) {
            s.ignorados.push((i, p.motivo.clone()));
        } else if !x.coincidente() && r.menores_que > 0. && x.volume < r.menores_que {
            s.menores.push(i);
        } else {
            s.ativos.push(i);
        }
    }
    s
}

/// Motivo que a lista mostra para o prendedor que atravessa de propósito.
pub const MOTIVO_SEM_FURO: &str = "Prendedor por ponto ou vetor: o furo será feito na hora";

/// Folga em volta do corpo do parafuso, em mm: a malha do comum encosta na superfície dele.
const TOL_CORPO: f64 = 0.05;

/// Corpo do parafuso de uma pilha sem furo: ponto da cabeça, sentido da ponta, comprimento e raio.
fn corpos_sem_furo(a: &Assembly, lib: &fenkra_solidos::biblioteca::Biblioteca) -> Vec<(Id, V3, V3, f64, f64)> {
    let mut v = vec![];
    for p in a.pilhas.iter().filter(|p| p.manual) {
        let Some(&parafuso) = p.pecas.first() else { continue };
        let Some(n) = a.node(parafuso) else { continue };
        let Some(d) = lib.familia(&p.familia).and_then(|f| f.tamanho(&p.tamanho)).and_then(|t| t.diametro_nominal()) else { continue };
        v.push((parafuso, n.placement.origin, unit(n.placement.dir([0., 0., -1.])), p.comprimento, d / 2.));
    }
    v
}

/// Todo o comum cabe no corpo? Sem malha, a análise não tem como provar, e o achado fica.
fn cabe_no_corpo(x: &Achado, cabeca: V3, u: V3, l: f64, raio: f64) -> bool {
    let dentro = |q: V3| {
        let d = sub(q, cabeca);
        let k = dot(d, u);
        let radial = norm([0, 1, 2].map(|i| d[i] - u[i] * k));
        k >= -TOL_CORPO && k <= l + TOL_CORPO && radial <= raio + TOL_CORPO
    };
    !x.malha.positions.is_empty() && x.malha.positions.chunks(3).all(|c| dentro([c[0] as f64, c[1] as f64, c[2] as f64]))
}

/// Como [`separar`], sabendo da montagem: o corpo do prendedor sem furo não conta como problema.
pub fn separar_na_montagem(a: &Assembly, achados: &[Achado], lib: &fenkra_solidos::biblioteca::Biblioteca) -> Reparticao {
    let mut s = separar(achados, &a.interf);
    let corpos = corpos_sem_furo(a, lib);
    let de_proposito = |x: &Achado| corpos.iter().any(|(c, cabeca, u, l, r)| (x.a.comp == *c || x.b.comp == *c) && cabe_no_corpo(x, *cabeca, *u, *l, *r));
    let (fica, sai): (Vec<usize>, Vec<usize>) = s.ativos.iter().copied().partition(|i| !de_proposito(&achados[*i]));
    s.ativos = fica;
    s.ignorados.extend(sai.into_iter().map(|i| (i, MOTIVO_SEM_FURO.to_string())));
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use fenkra_core::{BoolOp, Document, Extent, Extrude, FeatureKind, OriginPlane, PlaneRef, Sketch, SketchData};
    use std::path::PathBuf;

    /// Bloco de `l` × `l` × `l` com um canto na origem.
    fn bloco(l: f64) -> Document {
        let mut d = Document::new("Bloco");
        let mut n = 7_100_000u64;
        let mut al = || {
            n += 1;
            Id(n)
        };
        let mut s = SketchData::default();
        s.add_rect(&mut al, [0., 0.], [l, l], false);
        let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data: s }));
        d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind(l.to_string()), reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
        d
    }

    fn pasta(nome: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("fenkra_interf_{nome}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("b.fkp"), bloco(10.).to_json()).unwrap();
        d
    }

    fn peca(a: &mut Assembly, nome: &str, x: f64) -> Id {
        a.add_component(nome, NodeKind::Part { path: "b.fkp".into() }, Placement { origin: [x, 0., 0.], ..Default::default() }, None)
    }

    #[test]
    fn lista_vem_do_maior_para_o_menor_com_a_malha_do_comum() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let d = pasta("ordem");
        let mut a = Assembly::new("t");
        let p = peca(&mut a, "A", 0.);
        let q = peca(&mut a, "B", 9.); // 1 × 10 × 10 com A
        let r = peca(&mut a, "C", 13.); // 6 × 10 × 10 com B
        let achados = analisar(&a, Some(&d), &mut PartCache::default(), &OpcoesInterf::default());
        assert_eq!(achados.len(), 2);
        assert!((achados[0].volume - 600.).abs() < 1e-6, "{}", achados[0].volume);
        assert!((achados[1].volume - 100.).abs() < 1e-6);
        assert_eq!((achados[0].a.comp, achados[0].b.comp), (q, r));
        assert_eq!((achados[1].a.comp, achados[1].b.comp), (p, q));
        assert!(!achados[0].malha.indices.is_empty(), "o comum tem malha para o realce");
        let (lo, hi) = achados[1].caixa.expect("caixa");
        assert!((lo[0] - 9.).abs() < 1e-6 && (hi[0] - 10.).abs() < 1e-6, "a caixa é a do comum");
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn filtro_das_malhas_nao_perde_peca_dentro_e_descarta_a_vizinha_longe() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        // B394: caixas que se cruzam sem as peças se tocarem são o caso comum que o filtro corta
        let d = pasta("filtro");
        std::fs::write(d.join("g.fkp"), bloco(30.).to_json()).unwrap();
        let mut a = Assembly::new("t");
        let grande = a.add_component("G", NodeKind::Part { path: "g.fkp".into() }, Placement::default(), None);
        // inteira dentro da grande: nenhuma face se cruza, só o raio acha
        let miolo = a.add_component("M", NodeKind::Part { path: "b.fkp".into() }, Placement { origin: [10., 10., 10.], ..Default::default() }, None);
        let achados = analisar(&a, Some(&d), &mut PartCache::default(), &OpcoesInterf::default());
        assert_eq!(achados.len(), 1, "{:?}", achados.iter().map(|x| (&x.a.nome, &x.b.nome, x.volume)).collect::<Vec<_>>());
        assert_eq!((achados[0].a.comp, achados[0].b.comp), (grande, miolo));
        assert!((achados[0].volume - 1000.).abs() < 1e-6);
        // o filtro em si: perto quando se cruzam ou quando o vão cabe na margem
        let q = fenkra_core::QualidadeMalha::NORMAL;
        let cubo = |o: V3| {
            let mut c = PartCache::default();
            let mut b = Assembly::new("x");
            let id = b.add_component("c", NodeKind::Part { path: "b.fkp".into() }, Placement { origin: o, ..Default::default() }, None);
            grossa(&placed_shape(&b, id, Some(&d), &mut c).unwrap(), &q).unwrap()
        };
        let (c0, c5, c20) = (cubo([0.; 3]), cubo([5., 5., 5.]), cubo([20., 0., 0.]));
        assert!(perto(&c0, &c5, 0.01), "cubos que se cruzam");
        assert!(!perto(&c0, &c20, 1.), "10 mm de vão");
        assert!(perto(&c0, &c20, 10.5), "o vão cabe na margem");
        assert!(dentro(&cubo([0.; 3]), &grossa(&{
            let mut c = PartCache::default();
            let mut b = Assembly::new("x");
            let id = b.add_component("g", NodeKind::Part { path: "g.fkp".into() }, Placement { origin: [-10.; 3], ..Default::default() }, None);
            placed_shape(&b, id, Some(&d), &mut c).unwrap()
        }, &q).unwrap()));
        assert!(!dentro(&c0, &c20));
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn threads_acham_o_mesmo_que_uma_so() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        // B394: a ordem, os pares e os volumes não dependem de quantas threads fazem as booleanas
        let d = pasta("threads");
        let mut a = Assembly::new("t");
        for k in 0..12 {
            peca(&mut a, &format!("P{k}"), k as f64 * 4.5);
        }
        let mut c = PartCache::default();
        let um = analisar_em(&a, Some(&d), &mut c, &OpcoesInterf::default(), 1);
        let varios = analisar_em(&a, Some(&d), &mut c, &OpcoesInterf::default(), 6);
        // cada peça cruza as duas seguintes: 5,5 mm com a vizinha e 1 mm com a outra
        assert_eq!(um.len(), 11 + 10);
        assert_eq!(um.len(), varios.len());
        for (x, y) in um.iter().zip(&varios) {
            assert_eq!((&x.a.chave, &x.b.chave), (&y.a.chave, &y.b.chave));
            assert!((x.volume - y.volume).abs() < 1e-9, "{} x {}: {} e {}", x.a.nome, x.b.nome, x.volume, y.volume);
            assert!(!y.malha.indices.is_empty());
        }
        assert!((um[0].volume - 550.).abs() < 1e-6 && (um[20].volume - 100.).abs() < 1e-6, "{} {}", um[0].volume, um[20].volume);
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn coincidencia_so_conta_com_a_opcao() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let d = pasta("coinc");
        let mut a = Assembly::new("t");
        peca(&mut a, "A", 0.);
        peca(&mut a, "B", 10.);
        let mut c = PartCache::default();
        assert!(analisar(&a, Some(&d), &mut c, &OpcoesInterf::default()).is_empty(), "encostar não é interferência por padrão");
        let achados = analisar(&a, Some(&d), &mut c, &OpcoesInterf { coincidencia: true, ..Default::default() });
        assert_eq!(achados.len(), 1);
        assert!(achados[0].coincidente());
        assert!(!achados[0].contato.is_empty(), "linhas de contato para o realce");
        let (lo, hi) = achados[0].caixa.expect("caixa do contato");
        assert!((lo[0] - 10.).abs() < 1e-6 && (hi[0] - 10.).abs() < 1e-6);
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn escopo_limita_os_pares() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let d = pasta("escopo");
        let mut a = Assembly::new("t");
        let p = peca(&mut a, "A", 0.);
        let q = peca(&mut a, "B", 5.);
        let r = peca(&mut a, "C", 7.);
        let mut c = PartCache::default();
        assert_eq!(analisar(&a, Some(&d), &mut c, &OpcoesInterf::default()).len(), 3);
        let contra = OpcoesInterf { escopo: Escopo::ContraAsDemais, escolhidas: vec![r], ..Default::default() };
        assert_eq!(analisar(&a, Some(&d), &mut c, &contra).len(), 2);
        let entre = OpcoesInterf { escopo: Escopo::EntreEscolhidas, escolhidas: vec![p, q], ..Default::default() };
        let so = analisar(&a, Some(&d), &mut c, &entre);
        assert_eq!(so.len(), 1);
        assert_eq!((so[0].a.comp, so[0].b.comp), (p, q));
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn subconjunto_como_peca_unica_ou_aberto() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let d = pasta("sub");
        // dentro do subconjunto as duas peças já se cruzam
        let mut s = Assembly::new("sub");
        peca(&mut s, "S1", 0.);
        peca(&mut s, "S2", 5.);
        std::fs::write(d.join("sub.fka"), s.to_json()).unwrap();
        let mut a = Assembly::new("t");
        let sub = a.add_component("Sub", NodeKind::SubAssembly { path: "sub.fka".into() }, Placement { origin: [100., 0., 0.], ..Default::default() }, None);
        let p = peca(&mut a, "A", 112.); // cruza só S2 (x de 105 a 115)
        let mut c = PartCache::default();
        let unico = analisar(&a, Some(&d), &mut c, &OpcoesInterf::default());
        assert_eq!(unico.len(), 1, "o que há dentro do subconjunto não conta");
        assert_eq!((unico[0].a.comp, unico[0].b.comp), (sub, p));
        assert!((unico[0].volume - 300.).abs() < 1e-6, "{}", unico[0].volume);
        let aberto = analisar(&a, Some(&d), &mut c, &OpcoesInterf { sub_unico: false, ..Default::default() });
        assert_eq!(aberto.len(), 2, "S1 × S2 e S2 × A");
        assert!(aberto.iter().any(|x| x.a.nome.contains("S1") && x.b.nome.contains("S2")));
        assert_ne!(aberto[0].a.chave, aberto[0].b.chave);
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn ignorar_fica_salvo_na_montagem_e_vale_nos_dois_sentidos() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let d = pasta("ignorar");
        let mut a = Assembly::new("t");
        peca(&mut a, "A", 0.);
        peca(&mut a, "B", 9.);
        peca(&mut a, "C", 13.);
        let achados = analisar(&a, Some(&d), &mut PartCache::default(), &OpcoesInterf::default());
        let grande = &achados[0];
        a.interf.ignorar(&grande.b.chave, &grande.a.chave, "ajuste prensado");
        a.interf.menores_que = 200.;
        // salva e abre de novo
        let lida = Assembly::from_json(&a.to_json()).unwrap();
        let s = separar(&achados, &lida.interf);
        assert_eq!(s.ignorados, vec![(0, "ajuste prensado".to_string())]);
        assert_eq!(s.menores, vec![1], "100 mm³ fica abaixo do limite de 200");
        assert!(s.ativos.is_empty());
        let mut r = lida.interf.clone();
        r.restaurar(&grande.a.chave, &grande.b.chave);
        r.menores_que = 0.;
        assert_eq!(separar(&achados, &r).ativos, vec![0, 1]);
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn montagem_antiga_abre_sem_regras() {
        let mut v: serde_json::Value = serde_json::from_str(&Assembly::new("t").to_json()).unwrap();
        v.as_object_mut().unwrap().remove("interf");
        let a = Assembly::from_json(&v.to_string()).unwrap();
        assert_eq!(a.interf, RegrasInterf::default());
    }
}
