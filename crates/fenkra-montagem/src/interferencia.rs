// SPDX-License-Identifier: LGPL-2.1-or-later
//! Análise de interferências (B264). Sem interface: quem desenha é `fenkra-app/asm_interf.rs`.
//!
//! Como funciona no kernel:
//! 1. cada peça vira uma **unidade** com as formas já no mundo. Um subconjunto é uma unidade só (as
//!    peças dele não se comparam entre si) ou se abre em uma unidade por peça, conforme a opção;
//! 2. para cada par do escopo, as caixas envolventes filtram o que está longe e a operação booleana
//!    Comum devolve o sólido compartilhado. O volume dele é a medida da interferência, e a malha dele
//!    é o que aparece em vermelho na vista;
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
use fenkra_kernel::{Bool, Mesh, Shape};
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
}

/// Formas das peças de um subconjunto, no sistema de quem o contém.
fn formas_do_sub(path: &str, dir: Option<&Path>, cache: &mut PartCache, pl: Placement, prefixo: &str, fundo: u32) -> Vec<(String, String, Shape)> {
    if fundo > 16 {
        return vec![];
    }
    let full = full_path(dir, path);
    let Some(sub) = std::fs::read_to_string(&full).ok().and_then(|t| Assembly::from_json(&t).ok()) else { return vec![] };
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
                for (c, nome, s) in formas_do_sub(path, sub_dir.as_deref(), cache, n.placement, &chave, fundo + 1) {
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
                    v.push(Unidade { lado: lado(n.id.0.to_string(), n.name.clone()), formas: vec![s] });
                }
            }
            NodeKind::SubAssembly { path } => {
                let chave = n.id.0.to_string();
                let pecas = formas_do_sub(path, dir, cache, n.placement, &chave, 0);
                if sub_unico {
                    if !pecas.is_empty() {
                        v.push(Unidade { lado: lado(chave, n.name.clone()), formas: pecas.into_iter().map(|p| p.2).collect() });
                    }
                } else {
                    for (c, nome, s) in pecas {
                        v.push(Unidade { lado: lado(c, format!("{} / {nome}", n.name)), formas: vec![s] });
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

/// Compara dois lados. `folga` deixa passar caixas que só se tocam, para a coincidência.
fn comparar(a: &Unidade, b: &Unidade, op: &OpcoesInterf, q: &fenkra_core::QualidadeMalha) -> Option<Achado> {
    let folga = if op.coincidencia { 1e-6 } else { 0. };
    let (mut volume, mut malha, mut contato, mut caixa) = (0., Mesh::default(), vec![], None);
    for fa in &a.formas {
        for fb in &b.formas {
            if let (Some((la, ha)), Some((lb, hb))) = (fa.bbox(), fb.bbox()) {
                if (0..3).any(|k| ha[k] + folga < lb[k] || hb[k] + folga < la[k]) {
                    continue;
                }
            }
            let mut achou = false;
            if let Ok(c) = fa.boolean(fb, Bool::Common) {
                let v = c.mass().volume;
                if v > op.tol {
                    achou = true;
                    volume += v;
                    if let Ok(m) = fenkra_solidos::tesselar(&c, q) {
                        acrescentar(&mut malha, &m);
                    }
                    if let Some(bb) = c.bbox() {
                        unir_caixa(&mut caixa, bb);
                    }
                }
            }
            if !achou && op.coincidencia {
                for linha in fa.section(fb).unwrap_or_default() {
                    for p in &linha {
                        unir_caixa(&mut caixa, (*p, *p));
                    }
                    contato.push(linha);
                }
            }
        }
    }
    (volume > 0. || !contato.is_empty()).then(|| Achado { a: a.lado.clone(), b: b.lado.clone(), volume, malha, contato, caixa })
}

/// Todos os achados do escopo, do maior volume para o menor. Ignorados não saem daqui: ver [`separar`].
pub fn analisar(a: &Assembly, dir: Option<&Path>, cache: &mut PartCache, op: &OpcoesInterf) -> Vec<Achado> {
    let q = cache.qualidade();
    let us = unidades(a, dir, cache, op.sub_unico);
    let mut out = vec![];
    for i in 0..us.len() {
        for j in i + 1..us.len() {
            let (ua, ub) = (&us[i], &us[j]);
            // dentro do mesmo subconjunto aberto em peças, compara normalmente; mesma chave nunca
            if ua.lado.chave == ub.lado.chave || !no_escopo(op, ua.lado.comp, ub.lado.comp) {
                continue;
            }
            if let Some(c) = comparar(ua, ub, op, &q) {
                out.push(c);
            }
        }
    }
    out.sort_by(|x, y| y.volume.total_cmp(&x.volume));
    out
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
