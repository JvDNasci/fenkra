// SPDX-License-Identifier: LGPL-2.1-or-later
//! Pilha de fixadores como objeto recalculável (B98).
//!
//! O preenchimento de furos ([`crate::fixadores`]) insere peças soltas. Aqui cada conjunto de parafuso,
//! arruelas e porca ganha uma [`Pilha`], guardada na montagem, que sabe de qual furo veio e o que a
//! pessoa pediu. [`recalcular`] compara o que os furos dizem agora com o que a pilha fez da última vez:
//!
//! - o furo é achado pelo comando dele e pela posição entre os centros desse comando. Se o comando
//!   tem o mesmo número de centros, o furo mudou de lugar e o fixador vai junto; com menos centros, o
//!   ponto foi apagado e o fixador sai com ele;
//! - com "seguir o furo", o tamanho vem do diâmetro do furo; com "atualizar o comprimento", vem da pega
//!   e da série; com "trocar junto", arruela e porca acompanham o tamanho do parafuso;
//! - o que não cabe não some em silêncio: vira um [`Diagnostico`] de erro na linha da pilha.
//!
//! A troca refaz as peças da pilha. Quem chama guarda um passo de desfazer antes, então mudar o furo
//! de M6 para M8 volta inteiro com um desfazer.
//!
//! Uma pilha **manual** nasce de um ponto clicado numa face, sem furo. O automático não toca nela: só
//! mover o ponto ou editar a linha muda alguma coisa.

use crate::fixadores::{coletar_furos, colineares, inserir_pecas, medir, reconhecer, um_por_componente, Furo, Medida, OpcoesFixadores, Recusa, RelatorioFixadores, ENGAJAMENTO_PADRAO, GRUPO};
use crate::load::PartCache;
use crate::*;
use fenkra_solidos::biblioteca::Biblioteca;
use serde::{Deserialize, Serialize};
use std::path::Path;

fn sim() -> bool {
    true
}

fn engajamento_padrao() -> f64 {
    ENGAJAMENTO_PADRAO
}

fn rosca_alem_padrao() -> f64 {
    crate::fixadores::ROSCA_ALEM_PORCA_PADRAO
}

/// De onde a pilha sai, no sistema da peça da âncora.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Ancora {
    pub comp: Id,
    /// comando do furo; `Id(0)` no ponto manual
    pub feature: Id,
    pub indice: usize,
    /// centros do comando na última vez: distingue furo movido de ponto apagado
    pub irmaos: usize,
    pub pos: V3,
    /// sentido para dentro do material
    pub dir: V3,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Pilha {
    pub id: Id,
    pub nome: String,
    pub ancora: Ancora,
    /// ponto dado à mão: o automático não toca
    #[serde(default)]
    pub manual: bool,
    pub familia: String,
    pub tamanho: String,
    pub comprimento: f64,
    pub arruela: bool,
    pub porca: bool,
    pub familia_arruela: String,
    pub familia_porca: String,
    /// tamanho da arruela e da porca, quando não acompanham o parafuso
    #[serde(default)]
    pub tam_acessorios: Option<String>,
    #[serde(default)]
    pub folga: u8,
    #[serde(default = "sim")]
    pub travar_giro: bool,
    /// componentes da pilha, com o parafuso primeiro
    pub pecas: Vec<Id>,
    /// restrições que a própria pilha criou: refazer troca só estas. Vazio em arquivo antigo
    #[serde(default)]
    pub restricoes: Vec<Id>,
    #[serde(default = "sim")]
    pub seguir_furo: bool,
    #[serde(default = "sim")]
    pub atualizar_comprimento: bool,
    #[serde(default = "sim")]
    pub trocar_junto: bool,
    /// múltiplos do diâmetro de rosca engajada em furo roscado
    #[serde(default = "engajamento_padrao")]
    pub engajamento: f64,
    /// passos de rosca que sobram além da porca
    #[serde(default = "rosca_alem_padrao")]
    pub rosca_alem_porca: f64,
    /// espessura presa no ponto manual
    #[serde(default)]
    pub pega_manual: f64,
    /// o que as peças de hoje representam: mudou, refaz
    #[serde(default)]
    pub assinatura: String,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Nivel {
    Ok,
    Aviso,
    Erro,
}

/// Como está um ponto depois do recálculo.
#[derive(Clone, PartialEq, Debug)]
pub struct Diagnostico {
    pub pilha: Id,
    pub nivel: Nivel,
    pub texto: String,
}

/// O que mudou num recálculo.
#[derive(Clone, Default, PartialEq, Debug)]
pub struct Recalculo {
    pub diagnosticos: Vec<Diagnostico>,
    pub refeitas: usize,
    pub removidas: usize,
}

impl Recalculo {
    fn dizer(&mut self, pilha: Id, nivel: Nivel, texto: String) {
        self.diagnosticos.push(Diagnostico { pilha, nivel, texto });
    }

    pub fn de(&self, pilha: Id) -> Option<&Diagnostico> {
        self.diagnosticos.iter().find(|d| d.pilha == pilha)
    }
}

impl Pilha {
    fn opcoes(&self, comps: Vec<Id>) -> OpcoesFixadores {
        OpcoesFixadores {
            familia_parafuso: self.familia.clone(),
            familia_arruela: self.familia_arruela.clone(),
            familia_porca: self.familia_porca.clone(),
            arruela: self.arruela,
            porca: self.porca,
            folga: self.folga,
            componentes: comps,
            travar_giro: self.travar_giro,
            comprimento: (!self.atualizar_comprimento).then_some(self.comprimento),
            tam_acessorios: self.tam_acessorios.clone(),
            engajamento: self.engajamento,
            rosca_alem_porca: self.rosca_alem_porca,
            ..Default::default()
        }
    }

    /// Mesmo parafuso: é o que a tabela pinta da mesma cor.
    pub fn equivalente(&self, o: &Pilha) -> bool {
        self.familia == o.familia && self.tamanho == o.tamanho && (self.comprimento - o.comprimento).abs() < 1e-9
    }

    pub fn rotulo(&self) -> String {
        format!("{}x{}", self.tamanho, crate::fixadores::num(self.comprimento))
    }
}

fn assinatura(a: &Assembly, comp: Id, md: &Medida, p: &Pilha, tamanho: &str) -> String {
    let pl = a.node(comp).map(|n| n.placement).unwrap_or_default();
    let (e, u) = (pl.to_local(md.entrada), unit(pl.inverse().dir(md.u)));
    let r = |x: f64| (x * 100.).round() / 100. + 0.;
    format!("{}|{tamanho}|{}|{}|{:?}|{}|{:?}|{}|{}|{:?}|{:?}", p.familia, r(md.comprimento), md.com_porca, md.h1.map(r), p.arruela, p.tam_acessorios, p.familia_arruela, p.familia_porca, e.map(r), u.map(r))
}

/// Guarda a pilha que o preenchimento acabou de inserir.
pub(crate) fn registrar(a: &mut Assembly, furos: &[Furo], md: &Medida, tamanho: &str, op: &OpcoesFixadores, pecas: Vec<Id>, rotulo: &str) {
    let f = furos.iter().find(|f| f.comp == md.c_ent).unwrap_or(&furos[0]);
    let irmaos = furos.iter().filter(|x| x.comp == f.comp && x.feature == f.feature).count().max(f.indice + 1);
    let id = a.alloc();
    let mut p = Pilha {
        id,
        nome: rotulo.to_string(),
        ancora: Ancora { comp: f.comp, feature: f.feature, indice: f.indice, irmaos, pos: f.local.0, dir: f.local.1 },
        manual: false,
        familia: op.familia_parafuso.clone(),
        tamanho: tamanho.to_string(),
        comprimento: md.comprimento,
        arruela: op.arruela,
        porca: op.porca,
        familia_arruela: op.familia_arruela.clone(),
        familia_porca: op.familia_porca.clone(),
        tam_acessorios: op.tam_acessorios.clone(),
        folga: op.folga,
        travar_giro: op.travar_giro,
        restricoes: a.mates.iter().filter(|m| pecas.contains(&m.a.comp) || pecas.contains(&m.b.comp)).map(|m| m.id).collect(),
        pecas,
        seguir_furo: true,
        atualizar_comprimento: op.comprimento.is_none(),
        trocar_junto: op.tam_acessorios.is_none(),
        engajamento: op.engajamento,
        rosca_alem_porca: op.rosca_alem_porca,
        pega_manual: 0.,
        assinatura: String::new(),
    };
    p.assinatura = assinatura(a, f.comp, md, &p, tamanho);
    a.pilhas.push(p);
}

/// Tira as peças da pilha da montagem, com as restrições delas.
fn tirar_pecas(a: &mut Assembly, pecas: &[Id]) {
    a.nodes.retain(|n| !pecas.contains(&n.id));
    a.mates.retain(|m| !pecas.contains(&m.a.comp) && !pecas.contains(&m.b.comp));
}

/// A restrição foi feita pela pilha? Arquivo antigo não guardou a lista: ali vale que a pilha nunca
/// grava o clique, e que ela nunca se prende à origem.
fn da_pilha(p: &Pilha, m: &Mate) -> bool {
    if !p.restricoes.is_empty() {
        return p.restricoes.contains(&m.id);
    }
    m.a.pick.is_none() && m.b.pick.is_none() && m.a.comp != ORIGEM && m.b.comp != ORIGEM
}

/// Quem herda o id de cada peça antiga, na ordem: o parafuso sempre é o primeiro; depois quem usa o
/// mesmo arquivo; e, com o mesmo número de peças, a mesma posição na pilha, que é a troca de tamanho.
/// Peça sem herdeira saiu de verdade, como a arruela desligada.
fn herdeiras(velhas: &[(Id, NodeKind)], novas: &[(Id, NodeKind)]) -> Vec<(Id, Id)> {
    let mut pares: Vec<(Id, Id)> = vec![];
    let livre = |pares: &[(Id, Id)], v: Id, n: Id| !pares.iter().any(|p| p.0 == v || p.1 == n);
    if let (Some(v), Some(n)) = (velhas.first(), novas.first()) {
        pares.push((v.0, n.0));
    }
    for (v, kv) in velhas {
        if let Some((n, _)) = novas.iter().find(|(n, kn)| kn == kv && livre(&pares, *v, *n)) {
            pares.push((*v, *n));
        }
    }
    if velhas.len() == novas.len() {
        for (v, n) in velhas.iter().zip(novas) {
            if livre(&pares, v.0, n.0) {
                pares.push((v.0, n.0));
            }
        }
    }
    pares
}

/// A peça nova passa a ser a antiga: mesmo id e o que a pessoa ajustou nela.
fn herdar(a: &mut Assembly, velho: &Node, novo: Id) {
    if let Some(n) = a.node_mut(novo) {
        n.id = velho.id;
        n.visible = velho.visible;
        n.color = velho.color;
        n.number = velho.number;
        n.mode = velho.mode;
    }
    for m in &mut a.mates {
        for r in [&mut m.a, &mut m.b] {
            if r.comp == novo {
                r.comp = velho.id;
            }
        }
    }
}

/// Apaga a pilha e o que ela pôs na montagem.
pub fn apagar(a: &mut Assembly, id: Id) {
    if let Some(p) = a.pilhas.iter().find(|p| p.id == id).cloned() {
        tirar_pecas(a, &p.pecas);
        a.pilhas.retain(|x| x.id != id);
    }
}

/// Pilha à qual o componente pertence.
pub fn da_peca(a: &Assembly, comp: Id) -> Option<Id> {
    a.pilhas.iter().find(|p| p.pecas.contains(&comp)).map(|p| p.id)
}

fn pasta_dos_fixadores(a: &mut Assembly) -> Id {
    match a.nodes.iter().find(|n| matches!(n.kind, NodeKind::Group) && n.name == GRUPO) {
        Some(n) => n.id,
        None => a.add_group(GRUPO, None),
    }
}

/// O furo da âncora hoje: o mesmo lugar, ou o mesmo índice se o comando não perdeu centro. `None`
/// quando o ponto foi apagado.
fn achar_furo<'a>(an: &Ancora, furos: &'a [Furo]) -> Option<&'a Furo> {
    let irmaos: Vec<&Furo> = furos.iter().filter(|f| f.comp == an.comp && f.feature == an.feature).collect();
    let perto = |f: &&&Furo| (0..3).map(|k| (f.local.0[k] - an.pos[k]).powi(2)).sum::<f64>() < 1e-4;
    irmaos.iter().find(perto).copied().or_else(|| (irmaos.len() >= an.irmaos).then(|| irmaos.iter().find(|f| f.indice == an.indice).copied()).flatten())
}

/// Troca as peças da pilha `p` pelas de `md` e grava a pilha.
fn refazer(a: &mut Assembly, dir: &Path, lib: &Biblioteca, mut p: Pilha, md: &Medida, desig: &str, op: &OpcoesFixadores) -> Result<(), String> {
    let fam = lib.familia(&p.familia).ok_or("família desconhecida")?;
    let t = fam.tamanho(desig).ok_or("tamanho desconhecido")?;
    // o que a pessoa fez nas peças da pilha não é da pilha: fica de fora do refazer
    let velhas: Vec<Node> = p.pecas.iter().filter_map(|c| a.node(*c).cloned()).collect();
    let toca = |m: &Mate| p.pecas.contains(&m.a.comp) || p.pecas.contains(&m.b.comp);
    let manuais: Vec<Mate> = a.mates.iter().filter(|m| toca(m) && !da_pilha(&p, m)).cloned().collect();
    tirar_pecas(a, &p.pecas);
    let pasta = pasta_dos_fixadores(a);
    let mut rel = RelatorioFixadores::default();
    let mut arquivos = HashMap::new();
    let novas = inserir_pecas(a, dir, md, fam, t, desig, lib.familia(&p.familia_arruela), lib.familia(&p.familia_porca), op, pasta, &mut arquivos, &mut rel)?;
    // as restrições novas são as que tocam as peças novas, antes de qualquer troca de id
    p.restricoes = a.mates.iter().filter(|m| novas.contains(&m.a.comp) || novas.contains(&m.b.comp)).map(|m| m.id).collect();
    let tipo = |n: &Node| (n.id, n.kind.clone());
    let lista_nova: Vec<(Id, NodeKind)> = novas.iter().filter_map(|c| a.node(*c).map(tipo)).collect();
    let pares = herdeiras(&velhas.iter().map(tipo).collect::<Vec<_>>(), &lista_nova);
    for (v, n) in &pares {
        if let Some(velho) = velhas.iter().find(|x| x.id == *v) {
            herdar(a, velho, *n);
        }
    }
    p.pecas = novas.iter().map(|n| pares.iter().find(|x| x.1 == *n).map_or(*n, |x| x.0)).collect();
    // a restrição manual volta se as peças dela ainda existem; a de peça que saiu vai junto com ela
    let existe = |a: &Assembly, c: Id| c == ORIGEM || a.node(c).is_some();
    let voltam: Vec<Mate> = manuais.into_iter().filter(|m| existe(a, m.a.comp) && existe(a, m.b.comp)).collect();
    a.mates.extend(voltam);
    p.tamanho = desig.to_string();
    p.comprimento = md.comprimento;
    p.nome = p.rotulo();
    p.assinatura = assinatura(a, p.ancora.comp, md, &p, desig);
    if let Some(q) = a.pilhas.iter_mut().find(|q| q.id == p.id) {
        *q = p;
    }
    Ok(())
}

fn medida_manual(a: &Assembly, p: &Pilha, lib: &Biblioteca) -> Result<Medida, String> {
    let pl = a.node(p.ancora.comp).ok_or("a peça do ponto saiu da montagem")?.placement;
    let (e, u) = (pl.point(p.ancora.pos), unit(pl.dir(p.ancora.dir)));
    let acess = p.tam_acessorios.as_deref().unwrap_or(&p.tamanho);
    let h = |on: bool| on.then(|| lib.familia(&p.familia_arruela).and_then(|f| f.tamanho(acess)).and_then(|t| t.valor("h"))).flatten();
    let altura_porca = lib.familia(&p.familia_porca).and_then(|f| f.tamanho(acess)).and_then(|t| t.valor("m")).filter(|_| p.porca);
    let (h1, h2) = (h(p.arruela), h(p.arruela && altura_porca.is_some()));
    let saida = [0, 1, 2].map(|i| e[i] + u[i] * p.pega_manual);
    let mut falhas = vec![];
    let precisa = p.pega_manual + h1.unwrap_or(0.) + h2.unwrap_or(0.) + altura_porca.unwrap_or(0.);
    if altura_porca.is_some() && p.comprimento < precisa {
        falhas.push(format!("L{} não alcança a porca (pega + arruelas + porca = {} mm)", crate::fixadores::num(p.comprimento), crate::fixadores::num((precisa * 10.).round() / 10.)));
    }
    Ok(Medida { entrada: e, saida, u, c_ent: p.ancora.comp, c_sai: p.ancora.comp, roscada: false, com_porca: altura_porca.is_some(), h1, h2, w1: h1.unwrap_or(0.), comprimento: p.comprimento, falhas })
}

/// Cria uma pilha num ponto clicado, fora dos furos. `pos` e `dir_local` no sistema da peça, com
/// `dir_local` para dentro do material.
#[allow(clippy::too_many_arguments)]
pub fn adicionar_manual(a: &mut Assembly, dir: &Path, lib: &Biblioteca, comp: Id, pos: V3, dir_local: V3, familia: &str, tamanho: &str, comprimento: f64, op: &OpcoesFixadores, pega: f64) -> Result<Id, String> {
    let id = a.alloc();
    let mut p = Pilha {
        id,
        nome: String::new(),
        ancora: Ancora { comp, feature: Id(0), indice: 0, irmaos: 0, pos, dir: unit(dir_local) },
        manual: true,
        familia: familia.to_string(),
        tamanho: tamanho.to_string(),
        comprimento,
        arruela: op.arruela,
        porca: op.porca,
        familia_arruela: op.familia_arruela.clone(),
        familia_porca: op.familia_porca.clone(),
        tam_acessorios: None,
        folga: op.folga,
        travar_giro: op.travar_giro,
        pecas: vec![],
        restricoes: vec![],
        seguir_furo: false,
        atualizar_comprimento: false,
        trocar_junto: true,
        engajamento: op.engajamento,
        rosca_alem_porca: op.rosca_alem_porca,
        pega_manual: pega,
        assinatura: String::new(),
    };
    p.nome = p.rotulo();
    a.pilhas.push(p.clone());
    let md = medida_manual(a, &p, lib)?;
    let o = p.opcoes(vec![]);
    refazer(a, dir, lib, p, &md, tamanho, &o)?;
    Ok(id)
}

/// Move o ponto de uma pilha manual.
pub fn mover_ponto(a: &mut Assembly, dir: &Path, lib: &Biblioteca, id: Id, pos: V3) -> Result<(), String> {
    let Some(p) = a.pilhas.iter_mut().find(|p| p.id == id && p.manual) else { return Err("só o ponto manual se move à mão".into()) };
    p.ancora.pos = pos;
    let p = p.clone();
    let md = medida_manual(a, &p, lib)?;
    let o = p.opcoes(vec![]);
    let t = p.tamanho.clone();
    refazer(a, dir, lib, p, &md, &t, &o)
}

/// Recalcula uma pilha manual depois de editar a linha dela. O recálculo geral não passa por aqui.
fn refazer_manual(a: &mut Assembly, dir: &Path, lib: &Biblioteca, p: Pilha, r: &mut Recalculo) {
    let (id, t) = (p.id, p.tamanho.clone());
    let md = match medida_manual(a, &p, lib) {
        Ok(m) => m,
        Err(e) => return r.dizer(id, Nivel::Erro, e),
    };
    let o = p.opcoes(vec![]);
    if assinatura(a, p.ancora.comp, &md, &p, &t) != p.assinatura {
        match refazer(a, dir, lib, p, &md, &t, &o) {
            Ok(()) => r.refeitas += 1,
            Err(e) => return r.dizer(id, Nivel::Erro, e),
        }
    }
    match md.falhas.first() {
        Some(f) => r.dizer(id, Nivel::Erro, f.clone()),
        None => r.dizer(id, Nivel::Ok, String::new()),
    }
}

/// Acerta as pilhas com os furos de agora. Quem chama guarda o passo de desfazer antes.
pub fn recalcular(a: &mut Assembly, dir: &Path, cache: &mut PartCache, lib: &Biblioteca) -> Recalculo {
    let mut r = Recalculo::default();
    // peça apagada na árvore: a pilha sai junto, e o resto das peças dela também
    for p in a.pilhas.clone() {
        if p.pecas.iter().any(|c| a.node(*c).is_none()) || a.node(p.ancora.comp).is_none() {
            apagar(a, p.id);
            r.removidas += 1;
        }
    }
    let hardware: Vec<Id> = a.pilhas.iter().flat_map(|p| p.pecas.iter().copied()).collect();
    let comps: Vec<Id> = a.nodes.iter().filter(|n| matches!(n.kind, NodeKind::Part { .. }) && !hardware.contains(&n.id)).map(|n| n.id).collect();
    // ler os furos custa um recálculo da peça: uma vez por família e folga
    let mut lidos: HashMap<(String, u8), Vec<Furo>> = HashMap::new();
    for p in a.pilhas.clone() {
        if p.manual {
            refazer_manual(a, dir, lib, p, &mut r);
            continue;
        }
        let id = p.id;
        let Some(fam) = lib.familia(&p.familia) else {
            r.dizer(id, Nivel::Erro, format!("família desconhecida: {}", p.familia));
            continue;
        };
        let furos = lidos.entry((p.familia.clone(), p.folga)).or_insert_with(|| coletar_furos(a, dir, cache, &p.opcoes(comps.clone()), fam, &mut RelatorioFixadores::default()));
        let Some(f) = achar_furo(&p.ancora, furos).cloned() else {
            apagar(a, id);
            r.removidas += 1;
            continue;
        };
        let irmaos = furos.iter().filter(|x| x.comp == f.comp && x.feature == f.feature).count();
        let mut grupo: Vec<Furo> = furos.iter().filter(|x| colineares(f.boca, f.dir, x.boca, x.dir)).cloned().collect();
        um_por_componente(a, &mut grupo);
        let lido = f.tamanho.clone().or_else(|| reconhecer(fam, f.diametro, p.folga).0);
        let desig = match (p.seguir_furo, lido) {
            (true, Some(t)) => t,
            (true, None) => {
                r.dizer(id, Nivel::Erro, format!("furo Ø{} sem tamanho compatível na família {}", crate::fixadores::num(f.diametro), fam.norma));
                continue;
            }
            (false, _) => p.tamanho.clone(),
        };
        let Some(t) = fam.tamanho(&desig) else {
            r.dizer(id, Nivel::Erro, format!("{} não tem o tamanho {desig}", fam.norma));
            continue;
        };
        // com "trocar junto" desligado, a arruela e a porca ficam no tamanho que tinham
        let mut p = p;
        if !p.trocar_junto && desig != p.tamanho && p.tam_acessorios.is_none() {
            p.tam_acessorios = Some(p.tamanho.clone());
        }
        if p.trocar_junto {
            p.tam_acessorios = None;
        }
        let op = p.opcoes(comps.clone());
        let mut avisos = vec![];
        let md = match medir(&grupo, fam, t, &desig, lib.familia(&p.familia_arruela), lib.familia(&p.familia_porca), &op, &mut avisos) {
            Ok(m) => m,
            Err(Recusa::Mostrar(m)) => {
                r.dizer(id, Nivel::Erro, m);
                continue;
            }
            Err(Recusa::Calar) => {
                r.dizer(id, Nivel::Erro, format!("{} sem série de comprimentos", fam.norma));
                continue;
            }
        };
        // a âncora segue o furo, mesmo quando as peças não precisam mudar
        let ancora = Ancora { comp: f.comp, feature: f.feature, indice: f.indice, irmaos, pos: f.local.0, dir: f.local.1 };
        let falhas = md.falhas.clone();
        let nova = assinatura(a, p.ancora.comp, &md, &p, &desig);
        p.ancora = ancora;
        if nova != p.assinatura {
            match refazer(a, dir, lib, p.clone(), &md, &desig, &op) {
                Ok(()) => r.refeitas += 1,
                Err(e) => {
                    r.dizer(id, Nivel::Erro, e);
                    continue;
                }
            }
        } else if let Some(q) = a.pilhas.iter_mut().find(|q| q.id == id) {
            q.ancora = p.ancora.clone();
        }
        if let Some(m) = falhas.first() {
            r.dizer(id, Nivel::Erro, m.clone());
        } else if let Some(m) = avisos.first() {
            r.dizer(id, Nivel::Aviso, m.clone());
        } else if !p.seguir_furo && f.tamanho.as_deref().is_some_and(|x| x != desig) {
            r.dizer(id, Nivel::Aviso, format!("o furo pede {}, a pilha segue em {desig}", f.tamanho.clone().unwrap_or_default()));
        } else if p.tam_acessorios.as_deref().is_some_and(|x| x != desig) {
            r.dizer(id, Nivel::Aviso, format!("arruela e porca seguem em {}", p.tam_acessorios.clone().unwrap_or_default()));
        } else {
            r.dizer(id, Nivel::Ok, String::new());
        }
    }
    r
}

/// Muda as pilhas `ids` com `f` e acerta o resultado. Quem chama guarda o passo de desfazer antes.
pub fn editar(a: &mut Assembly, dir: &Path, cache: &mut PartCache, lib: &Biblioteca, ids: &[Id], f: impl Fn(&mut Pilha)) -> Recalculo {
    for p in a.pilhas.iter_mut().filter(|p| ids.contains(&p.id)) {
        f(p);
    }
    recalcular(a, dir, cache, lib)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixadores::preencher_furos;
    use fenkra_core::{sketch::Curve, BoolOp, Document, Extent, Extrude, FeatureKind, Hole, HoleKind, OriginPlane, PlaneRef, Sketch, SketchData};
    use fenkra_solidos::biblioteca;
    use std::path::PathBuf;

    /// Placa 60 × 60 × `esp` com furos passantes de Ø`d` nos pontos dados.
    fn placa(esp: f64, d: f64, pontos: &[[f64; 2]], rosca: Option<f64>) -> Document {
        let mut doc = Document::new("Placa");
        let mut n = 7_300_000u64;
        let mut al = || {
            n += 1;
            Id(n)
        };
        let mut base = SketchData::default();
        base.add_rect(&mut al, [-30., -30.], [30., 30.], false);
        let sb = doc.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data: base }));
        doc.add_feature(FeatureKind::Extrude(Extrude { sketch: sb, op: BoolOp::Add, extent: Extent::Blind(esp.to_string()), reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
        let topo = doc.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), &esp.to_string());
        let mut c = SketchData::default();
        for q in pontos {
            let p = c.add_point(&mut al, q[0], q[1]);
            c.add_curve(&mut al, Curve::Point { p }, false);
        }
        let sc = doc.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Datum(topo), data: c }));
        let (depth, thread) = match rosca {
            Some(prof) => (Extent::Blind(prof.to_string()), Some("M6".to_string())),
            None => (Extent::ThroughAll, None),
        };
        doc.add_feature(FeatureKind::Hole(Hole { sketch: sc, kind: HoleKind::Simples, diameter: d.to_string(), depth, thread, thread_modeled: false, standard: None, points: vec![], dir: None, reverse: false, tip: None, taper: "0".into(), left_hand: false, extra: Default::default() }));
        doc
    }

    fn pasta(nome: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("fenkra_pilha_{nome}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn regravar(d: &Path, nome: &str, doc: &Document, c: &mut PartCache) {
        std::fs::write(d.join(nome), doc.to_json()).unwrap();
        c.forget(&d.join(nome));
    }

    /// Duas placas de `esp` empilhadas, com furos Ø6,6, e o preenchimento rodado.
    fn conjunto(d: &Path, esp: f64, pontos: &[[f64; 2]]) -> (Assembly, PartCache) {
        std::fs::write(d.join("a.fkp"), placa(esp, 6.6, pontos, None).to_json()).unwrap();
        std::fs::write(d.join("b.fkp"), placa(esp, 6.6, pontos, None).to_json()).unwrap();
        let mut a = Assembly::new("t");
        a.add_component("A", NodeKind::Part { path: "a.fkp".into() }, Placement { origin: [0., 0., esp], ..Default::default() }, None);
        a.add_component("B", NodeKind::Part { path: "b.fkp".into() }, Placement::default(), None);
        let mut c = PartCache::default();
        let r = preencher_furos(&mut a, Some(d), &mut c, &OpcoesFixadores::default()).unwrap();
        assert_eq!(r.fixadores, pontos.len(), "{:?}", r.avisos);
        (a, c)
    }

    fn nomes(a: &Assembly, p: &Pilha) -> Vec<String> {
        p.pecas.iter().map(|c| a.name_of(*c)).collect()
    }

    /// Restrição manual, com o clique gravado, entre o parafuso e a placa A.
    fn manual(a: &mut Assembly, parafuso: Id) -> Id {
        let comp = a.nodes.iter().find(|n| n.name == "A").unwrap().id;
        let g = Geom::Plane { p: [0.; 3], n: [1., 0., 0.] };
        a.add_mate(MateKind::Parallel, MateRef { comp, geom: g, pick: Some([1., 0., 0.]) }, MateRef { comp: parafuso, geom: g, pick: Some([1., 0., 0.]) })
    }

    #[test]
    fn refazer_guarda_os_ids_e_a_restricao_manual() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let d = pasta("ids");
        let (mut a, mut c) = conjunto(&d, 8., &[[0., 0.]]);
        let p = a.pilhas[0].clone();
        let m = manual(&mut a, p.pecas[0]);
        let j = {
            let f = crate::kin::JFrame::from_axis([0.; 3], [0., 0., 1.]);
            a.add_joint(crate::kin::JointKind::Revolute, crate::kin::JRef { comp: p.pecas[3], frame: f }, crate::kin::JRef { comp: p.pecas[0], frame: f })
        };
        a.node_mut(p.pecas[0]).unwrap().color = Some([1, 2, 3]);
        let auto = a.mates.iter().filter(|x| p.restricoes.contains(&x.id)).count();
        let r = editar(&mut a, &d, &mut c, biblioteca::publica(), &[p.id], |x| {
            x.seguir_furo = false;
            x.tamanho = "M5".into();
        });
        assert_eq!(r.refeitas, 1, "{r:?}");
        let q = a.pilhas[0].clone();
        assert_eq!(q.pecas, p.pecas, "cada peça continua com o id dela");
        assert_eq!(a.node(q.pecas[0]).unwrap().color, Some([1, 2, 3]));
        assert!(a.mates.iter().any(|x| x.id == m), "a restrição manual ficou");
        assert_eq!(a.mates.iter().filter(|x| q.restricoes.contains(&x.id)).count(), auto, "as da pilha foram trocadas, sem acumular");
        assert!(!q.restricoes.contains(&m));
        let junta = a.joints.iter().find(|x| x.id == j).unwrap();
        assert!(a.node(junta.a.comp).is_some() && a.node(junta.b.comp).is_some(), "a junta não fica apontando para o vazio");
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn peca_que_sai_leva_so_a_restricao_dela() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let d = pasta("arruela");
        let (mut a, mut c) = conjunto(&d, 8., &[[0., 0.]]);
        let p = a.pilhas[0].clone();
        let no_parafuso = manual(&mut a, p.pecas[0]);
        let na_arruela = manual(&mut a, p.pecas[1]);
        let r = editar(&mut a, &d, &mut c, biblioteca::publica(), &[p.id], |x| x.arruela = false);
        assert_eq!(r.refeitas, 1, "{r:?}");
        let q = a.pilhas[0].clone();
        assert_eq!(q.pecas.len(), 2, "parafuso e porca");
        assert_eq!(q.pecas[0], p.pecas[0]);
        assert_eq!(q.pecas[1], p.pecas[3], "a porca usa o mesmo arquivo e fica com o id dela");
        assert!(a.mates.iter().any(|x| x.id == no_parafuso));
        assert!(!a.mates.iter().any(|x| x.id == na_arruela), "a arruela saiu de verdade");
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn arquivo_antigo_sem_a_lista_reconhece_a_restricao_pelo_clique() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let d = pasta("antigo");
        let (mut a, mut c) = conjunto(&d, 8., &[[0., 0.]]);
        let p = a.pilhas[0].clone();
        let m = manual(&mut a, p.pecas[0]);
        a.pilhas[0].restricoes.clear();
        editar(&mut a, &d, &mut c, biblioteca::publica(), &[p.id], |x| {
            x.seguir_furo = false;
            x.tamanho = "M5".into();
        });
        assert!(a.mates.iter().any(|x| x.id == m));
        assert!(!a.pilhas[0].restricoes.is_empty(), "o refazer já deixa a lista gravada");
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn preencher_registra_a_pilha_e_recalcular_sem_mudanca_nao_mexe_em_nada() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let d = pasta("parada");
        let (mut a, mut c) = conjunto(&d, 8., &[[0., 0.]]);
        assert_eq!(a.pilhas.len(), 1);
        let p = a.pilhas[0].clone();
        assert_eq!(p.tamanho, "M6");
        assert_eq!(p.pecas.len(), 4, "parafuso, duas arruelas e porca");
        let antes = a.clone();
        let r = recalcular(&mut a, &d, &mut c, biblioteca::publica());
        assert_eq!(r.refeitas, 0);
        assert_eq!(a.nodes, antes.nodes, "as peças continuam as mesmas");
        assert_eq!(r.de(p.id).unwrap().nivel, Nivel::Ok);
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn furo_de_m6_para_m8_troca_parafuso_arruela_e_porca_com_um_desfazer() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let d = pasta("m8");
        let (mut a, mut c) = conjunto(&d, 8., &[[0., 0.]]);
        let antes = a.clone();
        assert!(nomes(&a, &a.pilhas[0]).iter().all(|n| n.contains("M6")), "{:?}", nomes(&a, &a.pilhas[0]));
        regravar(&d, "a.fkp", &placa(8., 9., &[[0., 0.]], None), &mut c);
        regravar(&d, "b.fkp", &placa(8., 9., &[[0., 0.]], None), &mut c);
        let r = recalcular(&mut a, &d, &mut c, biblioteca::publica());
        assert_eq!(r.refeitas, 1, "{:?}", r.diagnosticos);
        let p = &a.pilhas[0];
        assert_eq!(p.tamanho, "M8");
        assert_eq!(p.pecas.len(), 4);
        assert!(nomes(&a, p).iter().all(|n| n.contains("M8")), "{:?}", nomes(&a, p));
        assert_eq!(a.nodes.iter().filter(|n| n.name.contains("M6")).count(), 0, "nada de M6 sobrou");
        // o desfazer da montagem é uma cópia do estado: volta tudo de uma vez
        a = antes;
        assert!(nomes(&a, &a.pilhas[0]).iter().all(|n| n.contains("M6")));
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn com_trocar_junto_desligado_arruela_e_porca_ficam_e_o_diagnostico_avisa() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let d = pasta("junto");
        let (mut a, mut c) = conjunto(&d, 8., &[[0., 0.]]);
        a.pilhas[0].trocar_junto = false;
        regravar(&d, "a.fkp", &placa(8., 9., &[[0., 0.]], None), &mut c);
        regravar(&d, "b.fkp", &placa(8., 9., &[[0., 0.]], None), &mut c);
        let r = recalcular(&mut a, &d, &mut c, biblioteca::publica());
        let p = &a.pilhas[0];
        let n = nomes(&a, p);
        assert!(n[0].contains("M8") && n[1..].iter().all(|x| x.contains("M6")), "{n:?}");
        assert_eq!(r.de(p.id).unwrap().nivel, Nivel::Aviso);
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn sem_seguir_o_furo_o_tamanho_fica_e_o_comprimento_acompanha_a_pilha() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let d = pasta("pega");
        let (mut a, mut c) = conjunto(&d, 8., &[[0., 0.]]);
        let l0 = a.pilhas[0].comprimento;
        regravar(&d, "a.fkp", &placa(30., 6.6, &[[0., 0.]], None), &mut c);
        let id = a.nodes.iter().find(|n| n.name == "A").unwrap().id;
        a.node_mut(id).unwrap().placement.origin = [0., 0., 8.];
        let r = recalcular(&mut a, &d, &mut c, biblioteca::publica());
        assert_eq!(r.refeitas, 1, "{:?}", r.diagnosticos);
        assert!(a.pilhas[0].comprimento > l0 + 15., "a pega cresceu e o parafuso também");
        // com o comprimento travado, não acompanha, e o erro aparece
        a.pilhas[0].atualizar_comprimento = false;
        a.pilhas[0].comprimento = l0;
        a.pilhas[0].assinatura.clear();
        let r = recalcular(&mut a, &d, &mut c, biblioteca::publica());
        assert_eq!(a.pilhas[0].comprimento, l0);
        assert_eq!(r.de(a.pilhas[0].id).unwrap().nivel, Nivel::Erro, "a pega não cabe e isso é visível");
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn mover_o_ponto_do_furo_move_o_fixador_e_apagar_remove() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let d = pasta("ponto");
        let (mut a, mut c) = conjunto(&d, 8., &[[-15., 0.], [15., 0.]]);
        assert_eq!(a.pilhas.len(), 2);
        let segundo = a.pilhas.iter().find(|p| p.ancora.pos[0] > 0.).unwrap().id;
        let movidos = [[-15., 0.], [15., 12.]];
        regravar(&d, "a.fkp", &placa(8., 6.6, &movidos, None), &mut c);
        regravar(&d, "b.fkp", &placa(8., 6.6, &movidos, None), &mut c);
        let r = recalcular(&mut a, &d, &mut c, biblioteca::publica());
        assert_eq!((r.refeitas, r.removidas), (1, 0), "{:?}", r.diagnosticos);
        let p = a.pilhas.iter().find(|p| p.id == segundo).unwrap();
        let o = a.node(p.pecas[0]).unwrap().placement.origin;
        assert!((o[1] - 12.).abs() < 1e-6, "o parafuso foi com o furo: {o:?}");
        // apaga o ponto: sobra um furo, e o fixador do ponto apagado sai
        let n = a.nodes.len();
        regravar(&d, "a.fkp", &placa(8., 6.6, &[[-15., 0.]], None), &mut c);
        regravar(&d, "b.fkp", &placa(8., 6.6, &[[-15., 0.]], None), &mut c);
        let r = recalcular(&mut a, &d, &mut c, biblioteca::publica());
        assert_eq!(r.removidas, 1);
        assert_eq!(a.pilhas.len(), 1);
        assert_eq!(a.nodes.len(), n - 4);
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn apagar_uma_peca_da_pilha_na_arvore_leva_a_pilha_inteira() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let d = pasta("arvore");
        let (mut a, mut c) = conjunto(&d, 8., &[[0., 0.]]);
        let parafuso = a.pilhas[0].pecas[0];
        assert_eq!(da_peca(&a, parafuso), Some(a.pilhas[0].id));
        a.delete(parafuso);
        recalcular(&mut a, &d, &mut c, biblioteca::publica());
        assert!(a.pilhas.is_empty());
        assert_eq!(a.nodes.iter().filter(|n| matches!(n.kind, NodeKind::Part { .. })).count(), 2, "só as placas");
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn furo_roscado_abaixo_da_profundidade_e_engajamento_curto_vira_erro() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let d = pasta("rosca");
        std::fs::write(d.join("a.fkp"), placa(20., 5., &[[0., 0.]], Some(12.)).to_json()).unwrap();
        let mut a = Assembly::new("t");
        a.add_component("A", NodeKind::Part { path: "a.fkp".into() }, Placement::default(), None);
        let mut c = PartCache::default();
        let op = OpcoesFixadores { arruela: false, ..Default::default() };
        preencher_furos(&mut a, Some(&d), &mut c, &op).unwrap();
        let p = a.pilhas[0].clone();
        assert!(p.comprimento <= 12., "abaixo da profundidade: {}", p.comprimento);
        let r = recalcular(&mut a, &d, &mut c, biblioteca::publica());
        assert_eq!(r.de(p.id).unwrap().nivel, Nivel::Ok);
        // exigindo 3 × d, os 12 mm não bastam
        a.pilhas[0].engajamento = 3.;
        let r = recalcular(&mut a, &d, &mut c, biblioteca::publica());
        let g = r.de(p.id).unwrap();
        assert_eq!(g.nivel, Nivel::Erro);
        assert!(g.texto.contains("engaja"), "{}", g.texto);
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn ponto_manual_nao_e_tocado_pelo_automatico_e_mover_move_o_fixador() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let d = pasta("manual");
        std::fs::write(d.join("a.fkp"), placa(8., 6.6, &[[0., 0.]], None).to_json()).unwrap();
        let mut a = Assembly::new("t");
        let comp = a.add_component("A", NodeKind::Part { path: "a.fkp".into() }, Placement::default(), None);
        let mut c = PartCache::default();
        let lib = biblioteca::publica();
        let op = OpcoesFixadores { arruela: true, porca: false, ..Default::default() };
        let id = adicionar_manual(&mut a, &d, lib, comp, [20., 20., 8.], [0., 0., -1.], "iso4762", "M5", 16., &op, 8.).unwrap();
        let p = a.pilhas.iter().find(|p| p.id == id).unwrap().clone();
        assert!(p.manual);
        assert_eq!(p.pecas.len(), 2);
        let n = a.nodes.clone();
        let r = recalcular(&mut a, &d, &mut c, lib);
        assert_eq!(r.refeitas, 0);
        assert_eq!(a.nodes, n, "o automático não toca no ponto manual");
        mover_ponto(&mut a, &d, lib, id, [-20., 20., 8.]).unwrap();
        let q = a.pilhas.iter().find(|p| p.id == id).unwrap();
        assert!((a.node(q.pecas[0]).unwrap().placement.origin[0] + 20.).abs() < 1e-6);
        apagar(&mut a, id);
        assert_eq!(a.nodes.iter().filter(|n| matches!(n.kind, NodeKind::Part { .. })).count(), 1);
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn equivalentes_sao_o_mesmo_parafuso() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let d = pasta("equi");
        let (mut a, mut c) = conjunto(&d, 8., &[[-15., 0.], [15., 0.]]);
        assert!(a.pilhas[0].equivalente(&a.pilhas[1]));
        let id = a.pilhas[1].id;
        let r = editar(&mut a, &d, &mut c, biblioteca::publica(), &[id], |p| {
            p.atualizar_comprimento = false;
            p.comprimento = 40.;
        });
        assert_eq!(r.refeitas, 1, "{:?}", r.diagnosticos);
        assert!(!a.pilhas[0].equivalente(&a.pilhas[1]));
        let _ = std::fs::remove_dir_all(d);
    }
}
