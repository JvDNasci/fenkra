// SPDX-License-Identifier: LGPL-2.1-or-later
//! Conjunto de fixadores (B375): um comando na árvore da montagem que manda em vários prendedores.
//!
//! O conjunto guarda o que a pessoa pediu: parafuso, diâmetro, comprimento (ou automático), a pilha de
//! cima, a de baixo e os alvos. Cada alvo vira uma [`Pilha`] comum, e cada peça dela segue sendo uma
//! peça normal da montagem. [`sincronizar`] é o único caminho: copia o pedido para as pilhas, cria as que
//! faltam, tira as que sobraram e deixa [`pilhas::recalcular`] refazer o que mudou. Por isso editar o
//! conjunto depois mantém os ids das peças e as restrições que a pessoa fez nelas.
//!
//! Três tipos de alvo:
//! - **furo**: a pega vem dos furos coaxiais, e a cabeça assenta no rebaixo ou no cone (ver `assento`);
//! - **ponto**: um lugar numa peça, com o sentido para dentro do material;
//! - **vetor**: um lugar e um sentido escolhido à mão.
//!
//! Ponto e vetor não têm furo: a reta atravessa as peças e o prendedor fica lá dentro de propósito,
//! porque o furo será feito na hora, sem aparecer nas subpeças. A pega é o bloco de material que a reta
//! cruza ([`pilhas::bloco_da_pega`]). A análise de interferência reconhece esse caso
//! (`interferencia::separar_na_montagem`).
//!
//! Regras por cor: [`RegraCor`] liga uma cor a um diâmetro. Furos e pontos carregam a cor da aparência do
//! comando que os criou, e [`por_cor`] acha todos os de uma cor. Com `por_cor` ligado no conjunto, cada
//! alvo tira o tamanho da regra da própria cor.

use crate::fixadores::{coletar_furos, colineares, OpcoesFixadores, RelatorioFixadores};
use crate::load::{full_path, PartCache};
use crate::pilhas::{self, Ancora, Diagnostico, ItemPilha, Nivel, Pilha, Recalculo, VAO_PADRAO};
use crate::*;
use fenkra_core::FeatureKind;
use fenkra_solidos::biblioteca::{Biblioteca, Familia};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Quanto cada canal pode variar para duas cores contarem como a mesma.
const TOL_COR: i32 = 6;
/// Dois diâmetros iguais, em mm: 1/8" é 3,175 e não pode virar M3 sem aviso.
const TOL_DIAMETRO: f64 = 0.01;

fn vao_padrao() -> f64 {
    VAO_PADRAO
}

fn familia_padrao() -> String {
    "iso4762".into()
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum Sistema {
    #[default]
    Metrico,
    Polegada,
}

/// Uma cor e o diâmetro que ela quer dizer.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct RegraCor {
    pub cor: [u8; 3],
    /// como a pessoa fala: "1/8", "M3"
    pub nome: String,
    /// diâmetro nominal em mm
    pub diametro: f64,
    #[serde(default)]
    pub sistema: Sistema,
}

/// Regras de fábrica: frações de polegada e métricos. As cores só precisam ser fáceis de separar.
pub fn regras_padrao() -> Vec<RegraCor> {
    let pol = |cor, nome: &str, n: f64, d: f64| RegraCor { cor, nome: nome.into(), diametro: n / d * 25.4, sistema: Sistema::Polegada };
    let met = |cor, d: f64| RegraCor { cor, nome: format!("M{d}"), diametro: d, sistema: Sistema::Metrico };
    vec![
        pol([192, 192, 200], "3/32", 3., 32.),
        pol([200, 120, 60], "1/8", 1., 8.),
        pol([70, 70, 80], "5/32", 5., 32.),
        pol([214, 180, 70], "3/16", 3., 16.),
        pol([120, 80, 170], "1/4", 1., 4.),
        met([230, 90, 90], 1.),
        met([240, 150, 60], 2.),
        met([235, 215, 70], 3.),
        met([90, 190, 110], 4.),
        met([70, 190, 200], 5.),
        met([80, 130, 230], 6.),
        met([170, 100, 220], 8.),
        met([225, 110, 180], 10.),
    ]
}

pub fn mesma_cor(a: [u8; 3], b: [u8; 3]) -> bool {
    (0..3).all(|k| (a[k] as i32 - b[k] as i32).abs() <= TOL_COR)
}

pub fn regra_da_cor(regras: &[RegraCor], cor: [u8; 3]) -> Option<&RegraCor> {
    regras.iter().find(|r| mesma_cor(r.cor, cor))
}

/// Tamanho da família para o diâmetro da regra e se ele é exato. Sem o exato, vale o mais próximo: o
/// prendedor aparece, e o aviso diz que não é a medida pedida.
pub fn tamanho_da_regra(fam: &Familia, regra: &RegraCor) -> Option<(String, bool)> {
    let chave = fenkra_solidos::biblioteca::Tamanho::chave_de(&regra.nome);
    if let Some(t) = fam.tamanhos.iter().find(|t| t.chave() == chave) {
        return Some((t.designacao.clone(), true));
    }
    let perto = fam.tamanhos.iter().filter_map(|t| t.diametro_nominal().map(|d| (t, (d - regra.diametro).abs()))).min_by(|a, b| a.1.total_cmp(&b.1))?;
    Some((perto.0.designacao.clone(), perto.1 <= TOL_DIAMETRO))
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum TipoAlvo {
    /// comando do furo e posição dele entre os centros desse comando
    Furo { comp: Id, feature: Id, indice: usize, pos: V3, dir: V3 },
    /// lugar no sistema da peça, com o sentido para dentro do material
    Ponto { comp: Id, pos: V3, dir: V3 },
    /// lugar e sentido escolhidos à mão; `comp` pode ser [`ORIGEM`]
    Vetor { comp: Id, pos: V3, dir: V3 },
}

impl TipoAlvo {
    pub fn comp(&self) -> Id {
        match self {
            TipoAlvo::Furo { comp, .. } | TipoAlvo::Ponto { comp, .. } | TipoAlvo::Vetor { comp, .. } => *comp,
        }
    }

    /// Lugar e sentido no sistema de `comp`.
    pub fn local(&self) -> (V3, V3) {
        match self {
            TipoAlvo::Furo { pos, dir, .. } | TipoAlvo::Ponto { pos, dir, .. } | TipoAlvo::Vetor { pos, dir, .. } => (*pos, *dir),
        }
    }

    pub fn mesmo_lugar(&self, o: &TipoAlvo) -> bool {
        match (self, o) {
            (TipoAlvo::Furo { comp: a, feature: f, indice: i, .. }, TipoAlvo::Furo { comp: b, feature: g, indice: j, .. }) => a == b && f == g && i == j,
            (TipoAlvo::Furo { .. }, _) | (_, TipoAlvo::Furo { .. }) => false,
            _ => self.comp() == o.comp() && norm(sub(self.local().0, o.local().0)) < 1e-3 && norm(sub(self.local().1, o.local().1)) < 1e-6,
        }
    }

    pub fn sem_furo(&self) -> bool {
        !matches!(self, TipoAlvo::Furo { .. })
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Alvo {
    pub id: Id,
    pub tipo: TipoAlvo,
    /// cor do furo ou do ponto, quando ele tem uma
    #[serde(default)]
    pub cor: Option<[u8; 3]>,
    /// pega à mão, em mm; `None` mede pela reta. Não vale para furo
    #[serde(default)]
    pub pega: Option<f64>,
    /// pilha que este alvo gerou
    #[serde(default)]
    pub pilha: Option<Id>,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct ConjuntoFixadores {
    pub id: Id,
    pub nome: String,
    #[serde(default = "familia_padrao")]
    pub familia: String,
    pub tamanho: String,
    /// comprimento fixo; `None` sai da pega e das pilhas
    #[serde(default)]
    pub comprimento: Option<f64>,
    /// da cabeça para a peça
    #[serde(default)]
    pub superior: Vec<ItemPilha>,
    /// da peça para a ponta
    #[serde(default)]
    pub inferior: Vec<ItemPilha>,
    #[serde(default)]
    pub alvos: Vec<Alvo>,
    /// cada alvo tira o tamanho da regra da própria cor
    #[serde(default)]
    pub por_cor: bool,
    /// folga do furo de passagem: 0 = fino, 1 = normal, 2 = folgado
    #[serde(default)]
    pub folga: u8,
    /// maior vão de ar dentro da pega, em mm
    #[serde(default = "vao_padrao")]
    pub vao: f64,
}

/// Qual das duas pilhas.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lado {
    Superior,
    Inferior,
}

impl ConjuntoFixadores {
    pub fn novo(id: Id, nome: &str) -> Self {
        let arruela = ItemPilha { familia: "iso7089".into(), tamanho: None };
        let porca = ItemPilha { familia: "iso4032".into(), tamanho: None };
        ConjuntoFixadores { id, nome: nome.into(), familia: familia_padrao(), tamanho: "M6".into(), comprimento: None, superior: vec![arruela.clone()], inferior: vec![arruela, porca], alvos: vec![], por_cor: false, folga: 1, vao: VAO_PADRAO }
    }

    pub fn lista(&self, l: Lado) -> &Vec<ItemPilha> {
        match l {
            Lado::Superior => &self.superior,
            Lado::Inferior => &self.inferior,
        }
    }

    pub fn lista_mut(&mut self, l: Lado) -> &mut Vec<ItemPilha> {
        match l {
            Lado::Superior => &mut self.superior,
            Lado::Inferior => &mut self.inferior,
        }
    }

    /// Leva o item `i` de `de` para a posição `j` de `para`, contada na lista antes da retirada: é o que
    /// a linha de destino mostra na hora de soltar. O arraste entre as duas pilhas é o mesmo gesto.
    pub fn mover_item(&mut self, de: Lado, i: usize, para: Lado, j: usize) {
        if i >= self.lista(de).len() {
            return;
        }
        let item = self.lista_mut(de).remove(i);
        let j = if de == para && j > i { j - 1 } else { j };
        let destino = self.lista_mut(para);
        let j = j.min(destino.len());
        destino.insert(j, item);
    }

    /// Acrescenta os alvos que ainda não estão no conjunto. Devolve quantos entraram.
    pub fn acrescentar(&mut self, a: &mut Assembly, novos: impl IntoIterator<Item = (TipoAlvo, Option<[u8; 3]>)>) -> usize {
        let mut n = 0;
        for (tipo, cor) in novos {
            if self.alvos.iter().any(|x| x.tipo.mesmo_lugar(&tipo)) {
                continue;
            }
            self.alvos.push(Alvo { id: a.alloc(), tipo, cor, pega: None, pilha: None });
            n += 1;
        }
        n
    }

    /// Tamanho do alvo e o aviso, se ele não é o pedido.
    fn tamanho_do_alvo(&self, regras: &[RegraCor], fam: &Familia, alvo: &Alvo) -> (String, Option<String>) {
        let padrao = (self.tamanho.clone(), None);
        if !self.por_cor {
            return padrao;
        }
        let Some(r) = alvo.cor.and_then(|c| regra_da_cor(regras, c)) else { return padrao };
        match tamanho_da_regra(fam, r) {
            Some((t, true)) => (t, None),
            Some((t, false)) => {
                let m = format!("a regra pede {} (Ø{:.2}), que {} não tem: ficou {t}", r.nome, r.diametro, fam.norma);
                (t, Some(m))
            }
            None => padrao,
        }
    }

    fn opcoes(&self) -> OpcoesFixadores {
        OpcoesFixadores { familia_parafuso: self.familia.clone(), superior: Some(self.superior.clone()), inferior: Some(self.inferior.clone()), comprimento: self.comprimento, folga: self.folga, ..Default::default() }
    }

    /// Passa o pedido para a pilha. Quem refaz as peças é o recálculo, se a assinatura mudar.
    fn copiar_para(&self, p: &mut Pilha, alvo: &Alvo, tamanho: &str) {
        p.conjunto = Some(self.id);
        p.familia = self.familia.clone();
        p.tamanho = tamanho.to_string();
        p.superior = Some(self.superior.clone());
        p.inferior = Some(self.inferior.clone());
        p.atualizar_comprimento = self.comprimento.is_none();
        if let Some(l) = self.comprimento {
            p.comprimento = l;
        }
        p.folga = self.folga;
        p.vao = self.vao;
        p.seguir_furo = false;
        p.trocar_junto = true;
        p.tam_acessorios = None;
        if alvo.tipo.sem_furo() {
            let (pos, dir) = alvo.tipo.local();
            p.ancora.comp = alvo.tipo.comp();
            p.ancora.pos = pos;
            p.ancora.dir = unit(dir);
            p.pega_auto = alvo.pega.is_none();
            p.pega_manual = alvo.pega.unwrap_or(0.);
        }
    }

    /// Pilha nova de um ponto ou vetor, ainda sem peças.
    fn pilha_livre(&self, id: Id, alvo: &Alvo, tamanho: &str) -> Pilha {
        let op = OpcoesFixadores::default();
        let (pos, dir) = alvo.tipo.local();
        let mut p = Pilha {
            id,
            nome: format!("{tamanho} · {}", self.nome),
            ancora: Ancora { comp: alvo.tipo.comp(), feature: Id(0), indice: 0, irmaos: 0, pos, dir: unit(dir) },
            manual: true,
            familia: String::new(),
            tamanho: String::new(),
            comprimento: 0.,
            arruela: false,
            porca: false,
            familia_arruela: op.familia_arruela,
            familia_porca: op.familia_porca,
            tam_acessorios: None,
            folga: 1,
            travar_giro: true,
            pecas: vec![],
            restricoes: vec![],
            seguir_furo: false,
            atualizar_comprimento: true,
            trocar_junto: true,
            engajamento: op.engajamento,
            rosca_alem_porca: op.rosca_alem_porca,
            pega_manual: 0.,
            assinatura: String::new(),
            conjunto: None,
            superior: None,
            inferior: None,
            pega_auto: true,
            vao: VAO_PADRAO,
        };
        self.copiar_para(&mut p, alvo, tamanho);
        p
    }
}

/// O que dá para escolher na montagem: um furo ou um ponto, já no mundo.
#[derive(Clone, PartialEq, Debug)]
pub struct Disponivel {
    pub tipo: TipoAlvo,
    pub cor: Option<[u8; 3]>,
    pub mundo: V3,
    pub sentido: V3,
    pub nome: String,
    /// diâmetro, no furo
    pub diametro: Option<f64>,
}

/// Furos e pontos das peças visíveis, sem os fixadores. Cor vem da aparência do comando: do furo, ou
/// do sketch e do elemento de referência que trazem o ponto. Os pontos que já são centros de furo saem:
/// quem está ali é o furo. Furos coaxiais de peças diferentes são um lugar só, porque um parafuso
/// atravessa todos: sem isso, a cor ou a cerca poria dois prendedores no mesmo eixo.
pub fn disponiveis(a: &Assembly, dir: &Path, cache: &mut PartCache) -> Vec<Disponivel> {
    let hardware: Vec<Id> = a.pilhas.iter().flat_map(|p| p.pecas.iter().copied()).collect();
    let comps: Vec<Id> = a.nodes.iter().filter(|n| matches!(n.kind, NodeKind::Part { .. }) && !hardware.contains(&n.id) && crate::view::shown(a, n.id)).map(|n| n.id).collect();
    let mut v = vec![];
    if let Some(fam) = fenkra_solidos::biblioteca::familia("iso4762") {
        let op = OpcoesFixadores { componentes: comps.clone(), ..Default::default() };
        for f in coletar_furos(a, dir, cache, &op, fam, &mut RelatorioFixadores::default()) {
            let cor = a.node(f.comp).and_then(|n| match &n.kind {
                NodeKind::Part { path } => cache.get(&full_path(Some(dir), path), false).ok().and_then(|lp| lp.doc.feature(f.feature).and_then(|x| x.appearance).map(|ap| ap.color)),
                _ => None,
            });
            if v.iter().any(|d: &Disponivel| colineares(d.mundo, d.sentido, f.boca, f.dir)) {
                continue;
            }
            let (pos, sentido) = f.local;
            v.push(Disponivel { tipo: TipoAlvo::Furo { comp: f.comp, feature: f.feature, indice: f.indice, pos, dir: sentido }, cor, mundo: f.boca, sentido: f.dir, nome: f.nome.clone(), diametro: Some(f.diametro) });
        }
    }
    for id in comps {
        let Some(n) = a.node(id) else { continue };
        let NodeKind::Part { path } = &n.kind else { continue };
        let Ok(lp) = cache.get(&full_path(Some(dir), path), true) else { continue };
        let furos: Vec<Id> = lp.doc.active_features().filter_map(|(_, f)| if let FeatureKind::Hole(h) = &f.kind { Some(h.sketch) } else { None }).collect();
        for s in lp.result.sketches.iter().filter(|s| !furos.contains(&s.feature)) {
            let cor = lp.doc.feature(s.feature).and_then(|f| f.appearance).map(|ap| ap.color);
            let dentro = s.frame.normal.map(|x| -x);
            for q in &s.points {
                let pos = q.map(|x| x as f64);
                v.push(Disponivel { tipo: TipoAlvo::Ponto { comp: id, pos, dir: dentro }, cor, mundo: n.placement.point(pos), sentido: unit(n.placement.dir(dentro)), nome: n.name.clone(), diametro: None });
            }
        }
        for q in &lp.result.points {
            let cor = lp.doc.reference(q.id).and_then(|r| r.appearance).map(|ap| ap.color);
            let dentro = [0., 0., -1.];
            v.push(Disponivel { tipo: TipoAlvo::Ponto { comp: id, pos: q.p, dir: dentro }, cor, mundo: n.placement.point(q.p), sentido: unit(n.placement.dir(dentro)), nome: format!("{} · {}", n.name, q.name), diametro: None });
        }
    }
    v
}

/// Índices dos furos e pontos da cor.
pub fn por_cor(lista: &[Disponivel], cor: [u8; 3]) -> Vec<usize> {
    lista.iter().enumerate().filter(|(_, d)| d.cor.is_some_and(|c| mesma_cor(c, cor))).map(|(i, _)| i).collect()
}

/// Cria um conjunto vazio e devolve o id dele.
pub fn criar(a: &mut Assembly, nome: &str) -> Id {
    let id = a.alloc();
    let nome = (1..).map(|i| format!("{nome} {i}")).find(|n| !a.conjuntos.iter().any(|c| c.nome == *n)).unwrap_or_default();
    a.conjuntos.push(ConjuntoFixadores::novo(id, &nome));
    id
}

/// Conjunto a que o componente pertence.
pub fn da_peca(a: &Assembly, comp: Id) -> Option<Id> {
    let p = pilhas::da_peca(a, comp)?;
    a.pilhas.iter().find(|x| x.id == p)?.conjunto
}

/// Apaga o conjunto e tudo o que ele pôs na montagem.
pub fn apagar(a: &mut Assembly, id: Id) {
    let ids: Vec<Id> = a.pilhas.iter().filter(|p| p.conjunto == Some(id)).map(|p| p.id).collect();
    for p in ids {
        pilhas::apagar(a, p);
    }
    a.conjuntos.retain(|c| c.id != id);
}

/// Alvo cuja pilha sumiu (peça apagada na árvore, furo apagado) sai do conjunto. O alvo novo, que ainda
/// não gerou pilha, fica.
pub(crate) fn soltar_alvos_sem_pilha(a: &mut Assembly) {
    let vivas: Vec<Id> = a.pilhas.iter().map(|p| p.id).collect();
    for c in &mut a.conjuntos {
        c.alvos.retain(|x| x.pilha.is_none_or(|p| vivas.contains(&p)));
    }
}

/// Leva o conjunto para a montagem: cria o que falta, tira o que sobrou e recalcula. Quem chama guarda o
/// passo de desfazer antes. O diagnóstico vem por pilha, ou pelo id do alvo quando a pilha nem nasceu.
pub fn sincronizar(a: &mut Assembly, dir: &Path, cache: &mut PartCache, lib: &Biblioteca, id: Id) -> Recalculo {
    let mut extra: Vec<Diagnostico> = vec![];
    let Some(mut c) = a.conjuntos.iter().find(|c| c.id == id).cloned() else { return Recalculo::default() };
    let Some(fam) = lib.familia(&c.familia) else {
        let texto = format!("família desconhecida: {}", c.familia);
        let mut r = Recalculo::default();
        r.diagnosticos.extend(c.alvos.iter().map(|x| Diagnostico { pilha: x.pilha.unwrap_or(x.id), nivel: Nivel::Erro, texto: texto.clone() }));
        return r;
    };
    let regras = regras_da(a);

    // alvo tirado: a pilha vai com ele
    let donas: Vec<Id> = c.alvos.iter().filter_map(|x| x.pilha).collect();
    let sobras: Vec<Id> = a.pilhas.iter().filter(|p| p.conjunto == Some(id) && !donas.contains(&p.id)).map(|p| p.id).collect();
    for p in sobras {
        pilhas::apagar(a, p);
    }

    let op = c.opcoes();
    let quantos = c.alvos.len();
    for i in 0..quantos {
        let alvo = c.alvos[i].clone();
        let (tamanho, aviso) = c.tamanho_do_alvo(&regras, fam, &alvo);
        let existente = alvo.pilha.filter(|p| a.pilhas.iter().any(|x| x.id == *p));
        let criada = match (existente, &alvo.tipo) {
            (Some(p), _) => {
                if let Some(q) = a.pilhas.iter_mut().find(|x| x.id == p) {
                    c.copiar_para(q, &alvo, &tamanho);
                }
                Ok(p)
            }
            (None, TipoAlvo::Furo { comp, feature, indice, .. }) => {
                let feita = pilhas::criar_no_furo(a, dir, cache, lib, &op, &tamanho, *comp, *feature, *indice);
                if let (Ok(p), true) = (&feita, true) {
                    if let Some(q) = a.pilhas.iter_mut().find(|x| x.id == *p) {
                        c.copiar_para(q, &alvo, &tamanho);
                    }
                }
                feita
            }
            (None, _) => {
                let pid = a.alloc();
                let p = c.pilha_livre(pid, &alvo, &tamanho);
                pilhas::criar_livre(a, dir, cache, lib, p)
            }
        };
        match criada {
            Ok(p) => {
                c.alvos[i].pilha = Some(p);
                if let Some(texto) = aviso {
                    extra.push(Diagnostico { pilha: p, nivel: Nivel::Aviso, texto });
                }
            }
            Err(texto) => {
                c.alvos[i].pilha = None;
                extra.push(Diagnostico { pilha: alvo.id, nivel: Nivel::Erro, texto });
            }
        }
    }
    if let Some(x) = a.conjuntos.iter_mut().find(|x| x.id == id) {
        *x = c;
    }
    let mut r = pilhas::recalcular(a, dir, cache, lib);
    // o aviso da regra só aparece se o recálculo não tem nada mais grave a dizer
    for d in extra {
        match r.diagnosticos.iter_mut().find(|x| x.pilha == d.pilha) {
            Some(x) if x.nivel == Nivel::Ok => *x = d,
            Some(_) => {}
            None => r.diagnosticos.push(d),
        }
    }
    r
}

/// As regras da montagem; sem nenhuma, as de fábrica.
pub fn regras_da(a: &Assembly) -> Vec<RegraCor> {
    if a.regras_cor.is_empty() {
        regras_padrao()
    } else {
        a.regras_cor.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixadores::tests::{pasta, placa};
    use crate::interferencia::{self, OpcoesInterf};
    use crate::solve;
    use fenkra_core::{sketch::Curve, Appearance, BoolOp, Document, Extent, Extrude, FeatureKind, Finish, Hole, HoleKind, OriginPlane, PlaneRef, Sketch, SketchData};
    use fenkra_solidos::biblioteca;
    use std::path::PathBuf;

    const AZUL: [u8; 3] = [80, 130, 230];
    const LARANJA: [u8; 3] = [240, 150, 60];

    fn lib() -> &'static Biblioteca {
        biblioteca::publica()
    }

    fn arruela() -> ItemPilha {
        ItemPilha { familia: "iso7089".into(), tamanho: None }
    }

    fn porca() -> ItemPilha {
        ItemPilha { familia: "iso4032".into(), tamanho: None }
    }

    fn altura(item: &ItemPilha, acess: &str) -> f64 {
        let f = lib().familia(&item.familia).unwrap();
        let t = f.tamanho(item.tamanho.as_deref().unwrap_or(acess)).unwrap();
        t.valor("h").or(t.valor("m")).unwrap()
    }

    /// Placa lisa de 60 × 60 × `esp`, sem furo nenhum.
    fn lisa(esp: f64) -> Document {
        let mut d = Document::new("Lisa");
        let mut n = 7_500_000u64;
        let mut al = || {
            n += 1;
            Id(n)
        };
        let mut s = SketchData::default();
        s.add_rect(&mut al, [-30., -30.], [30., 30.], false);
        let sb = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data: s }));
        d.add_feature(FeatureKind::Extrude(Extrude { sketch: sb, op: BoolOp::Add, extent: Extent::Blind(esp.to_string()), reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
        d
    }

    /// Placa com um furo Ø6,6 de cada cor e um sketch de pontos soltos, também colorido.
    fn colorida() -> Document {
        let mut d = lisa(8.);
        let mut n = 7_600_000u64;
        let mut al = || {
            n += 1;
            Id(n)
        };
        let topo = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "8");
        let tinta = |cor| Some(Appearance { color: cor, finish: Finish::default(), texture: None });
        for (x, cor) in [(-15., AZUL), (15., LARANJA)] {
            let mut c = SketchData::default();
            let p = c.add_point(&mut al, x, 0.);
            c.add_curve(&mut al, Curve::Point { p }, false);
            let sc = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Datum(topo), data: c }));
            let h = d.add_feature(FeatureKind::Hole(Hole { sketch: sc, kind: HoleKind::Simples, diameter: "6.6".into(), depth: Extent::ThroughAll, thread: None, thread_modeled: false, standard: None, points: vec![], dir: None, reverse: false, tip: None, taper: "0".into(), left_hand: false, extra: Default::default() }));
            d.feature_mut(h).unwrap().appearance = tinta(cor);
        }
        let mut c = SketchData::default();
        for q in [[0., 15.], [0., -15.]] {
            let p = c.add_point(&mut al, q[0], q[1]);
            c.add_curve(&mut al, Curve::Point { p }, false);
        }
        let sc = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Datum(topo), data: c }));
        d.feature_mut(sc).unwrap().appearance = tinta(AZUL);
        d
    }

    fn montagem(nome: &str, pecas: &[(&str, &Document, f64)]) -> (Assembly, PartCache, PathBuf, Vec<Id>) {
        let d = pasta(&format!("conjunto_{nome}"));
        let mut a = Assembly::new("t");
        let mut ids = vec![];
        for (n, doc, z) in pecas {
            let arq = format!("{n}.fkp");
            std::fs::write(d.join(&arq), doc.to_json()).unwrap();
            ids.push(a.add_component(n, NodeKind::Part { path: arq }, Placement { origin: [0., 0., *z], ..Default::default() }, None));
        }
        (a, PartCache::default(), d, ids)
    }

    fn origem(a: &Assembly, c: Id) -> V3 {
        a.node(c).unwrap().placement.origin
    }

    fn pilha_de<'a>(a: &'a Assembly, c: Id, i: usize) -> &'a Pilha {
        let p = a.conjuntos.iter().find(|x| x.id == c).unwrap().alvos[i].pilha.expect("alvo com pilha");
        a.pilhas.iter().find(|x| x.id == p).unwrap()
    }

    fn sem_erro(r: &Recalculo) {
        assert!(r.diagnosticos.iter().all(|d| d.nivel != Nivel::Erro), "{:?}", r.diagnosticos);
    }

    /// Conjunto com cinco itens de tipos diferentes: três em cima e dois embaixo.
    fn cinco(c: &mut ConjuntoFixadores) {
        let grande = ItemPilha { familia: "iso7089".into(), tamanho: Some("M8".into()) };
        let fina = ItemPilha { familia: "iso4035".into(), tamanho: None };
        c.superior = vec![arruela(), grande, fina.clone()];
        c.inferior = vec![arruela(), porca()];
    }

    #[test]
    fn pilha_de_cinco_itens_de_tipos_diferentes_fica_cada_um_no_seu_lugar() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (mut a, mut cache, d, _) = montagem("cinco", &[("A", &placa(10., 6.6, None), 10.), ("B", &placa(10., 6.6, None), 0.)]);
        let furos = disponiveis(&a, &d, &mut cache);
        let id = criar(&mut a, "Conjunto");
        let mut c = a.conjuntos[0].clone();
        cinco(&mut c);
        let furo = furos.iter().find(|f| f.tipo.comp() == a.nodes[0].id).unwrap();
        c.acrescentar(&mut a, [(furo.tipo.clone(), None)]);
        a.conjuntos[0] = c.clone();
        let r = sincronizar(&mut a, &d, &mut cache, lib(), id);
        sem_erro(&r);
        let p = pilha_de(&a, id, 0).clone();
        assert_eq!(p.pecas.len(), 6, "parafuso e cinco itens: {:?}", p.pecas.iter().map(|x| a.name_of(*x)).collect::<Vec<_>>());

        // furo de z = 20 a z = 0: a pilha de cima sobe da peça, a de baixo desce
        let (h0, h1, h2) = (altura(&c.superior[0], "M6"), altura(&c.superior[1], "M6"), altura(&c.superior[2], "M6"));
        let (h3, h4) = (altura(&c.inferior[0], "M6"), altura(&c.inferior[1], "M6"));
        let z: Vec<f64> = p.pecas.iter().map(|x| origem(&a, *x)[2]).collect();
        let certo = [20. + h0 + h1 + h2, 20. + h1 + h2, 20. + h2, 20., 0., -h3];
        for (k, (v, e)) in z.iter().zip(certo).enumerate() {
            assert!((v - e).abs() < 1e-6, "peça {k}: z = {v}, esperado {e}; {z:?}");
        }
        assert!(p.pecas.iter().all(|x| origem(&a, *x)[0].hypot(origem(&a, *x)[1]) < 1e-6), "todos no eixo");
        // a face de apoio de cada item olha para a peça
        let eixo = |x: Id| a.node(x).unwrap().placement.dir([0., 0., 1.])[2];
        assert!(eixo(p.pecas[0]) > 0.9, "cabeça com o corpo para baixo");
        assert!(p.pecas[1..4].iter().all(|x| eixo(*x) < -0.9 || eixo(*x) > 0.9));
        assert!(eixo(p.pecas[1]) < -0.9 && eixo(p.pecas[3]) < -0.9, "de cima: Z para a peça");
        assert!(eixo(p.pecas[4]) > 0.9 && eixo(p.pecas[5]) > 0.9, "de baixo: Z para a peça");

        // comprimento da pilha e da pega: 20 + cinco alturas + 2 passos, no próximo da série
        let precisa = 20. + h0 + h1 + h2 + h3 + h4 + 2.;
        assert!(p.comprimento >= precisa - 1e-9, "L{} < {precisa}", p.comprimento);
        let fam = lib().familia("iso4762").unwrap();
        let serie = fam.comprimentos_de(fam.tamanho("M6").unwrap());
        assert_eq!(Some(p.comprimento), serie.iter().copied().find(|l| *l >= precisa));

        let rep = solve::solve(&mut a);
        assert!(rep.converged, "{rep:?}");
        let depois: Vec<f64> = p.pecas.iter().map(|x| origem(&a, *x)[2]).collect();
        for (v, e) in depois.iter().zip(certo) {
            assert!((v - e).abs() < 0.01, "depois de resolver: {depois:?}");
        }
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn reordenar_troca_as_posicoes_e_guarda_os_ids_das_pecas() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (mut a, mut cache, d, _) = montagem("ordem", &[("A", &placa(10., 6.6, None), 0.)]);
        let furos = disponiveis(&a, &d, &mut cache);
        let id = criar(&mut a, "Conjunto");
        let mut c = a.conjuntos[0].clone();
        cinco(&mut c);
        c.acrescentar(&mut a, [(furos[0].tipo.clone(), None)]);
        a.conjuntos[0] = c;
        sincronizar(&mut a, &d, &mut cache, lib(), id);
        let antes = pilha_de(&a, id, 0).clone();
        let nomes: Vec<String> = antes.pecas.iter().map(|x| a.name_of(*x)).collect();
        // a arruela grande (linha 1) desce para o fim da pilha de cima: encosta na peça
        a.conjuntos[0].mover_item(Lado::Superior, 1, Lado::Superior, 3);
        let r = sincronizar(&mut a, &d, &mut cache, lib(), id);
        sem_erro(&r);
        let depois = pilha_de(&a, id, 0).clone();
        assert_eq!(a.conjuntos[0].superior[2].tamanho.as_deref(), Some("M8"));
        let mut ids = antes.pecas.clone();
        let mut novos = depois.pecas.clone();
        ids.sort_by_key(|i| i.0);
        novos.sort_by_key(|i| i.0);
        assert_eq!(ids, novos, "reordenar não cria peça nova");
        assert_eq!(depois.pecas[3], antes.pecas[2], "a arruela grande é a mesma peça, agora junto da placa");
        assert_eq!(a.name_of(depois.pecas[3]), nomes[2]);
        assert!((origem(&a, depois.pecas[3])[2] - 10.).abs() < 1e-6, "encostada na face de cima");
        // de uma pilha para a outra é o mesmo gesto
        a.conjuntos[0].mover_item(Lado::Superior, 0, Lado::Inferior, 0);
        assert_eq!((a.conjuntos[0].superior.len(), a.conjuntos[0].inferior.len()), (2, 3));
        sem_erro(&sincronizar(&mut a, &d, &mut cache, lib(), id));
        assert_eq!(pilha_de(&a, id, 0).pecas.len(), 6);
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn mover_item_conta_a_posicao_da_linha_de_destino() {
        let mut c = ConjuntoFixadores::novo(Id(1), "x");
        let item = |n: &str| ItemPilha { familia: n.into(), tamanho: None };
        c.superior = vec![item("a"), item("b"), item("c")];
        c.mover_item(Lado::Superior, 0, Lado::Superior, 2);
        assert_eq!(c.superior.iter().map(|i| i.familia.as_str()).collect::<Vec<_>>(), ["b", "a", "c"]);
        c.mover_item(Lado::Superior, 2, Lado::Superior, 0);
        assert_eq!(c.superior.iter().map(|i| i.familia.as_str()).collect::<Vec<_>>(), ["c", "b", "a"]);
        c.mover_item(Lado::Superior, 9, Lado::Superior, 0);
        assert_eq!(c.superior.len(), 3, "índice fora da lista não faz nada");
    }

    #[test]
    fn alvo_por_ponto_atravessa_duas_placas_sem_furo_e_nao_vira_erro_de_interferencia() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (mut a, mut cache, d, ids) = montagem("ponto", &[("A", &lisa(8.), 8.), ("B", &lisa(8.), 0.)]);
        let id = criar(&mut a, "Conjunto");
        let mut c = a.conjuntos[0].clone();
        c.superior = vec![arruela()];
        c.inferior = vec![arruela(), porca()];
        // ponto na face de cima da placa A, no sistema dela: z = 8 local é z = 16 no mundo
        c.acrescentar(&mut a, [(TipoAlvo::Ponto { comp: ids[0], pos: [10., 10., 8.], dir: [0., 0., -1.] }, None)]);
        a.conjuntos[0] = c;
        let r = sincronizar(&mut a, &d, &mut cache, lib(), id);
        sem_erro(&r);
        let p = pilha_de(&a, id, 0).clone();
        assert!(p.manual && p.pega_auto);
        assert_eq!(p.pecas.len(), 4);
        let h = 1.6;
        let (parafuso, a1, a2, n) = (origem(&a, p.pecas[0]), origem(&a, p.pecas[1]), origem(&a, p.pecas[2]), origem(&a, p.pecas[3]));
        assert!((parafuso[2] - (16. + h)).abs() < 1e-6, "cabeça sobre a arruela: {parafuso:?}");
        assert!((parafuso[0] - 10.).abs() < 1e-6 && (parafuso[1] - 10.).abs() < 1e-6);
        assert!((a1[2] - 16.).abs() < 1e-6, "{a1:?}");
        assert!((a2[2] - 0.).abs() < 1e-6, "a pega atravessou as duas placas: {a2:?}");
        assert!((n[2] + h).abs() < 1e-6, "{n:?}");
        // 16 + 1,6 + 1,6 + 5,2 + 2 = 26,4 → 30
        assert_eq!(p.comprimento, 30.);

        // nenhuma peça ganhou furo: o volume delas segue cheio
        let v = |cache: &mut PartCache, nome: &str| cache.get(&d.join(nome), true).unwrap().shape.as_ref().unwrap().mass().volume;
        assert!((v(&mut cache, "A.fkp") - 60. * 60. * 8.).abs() < 1e-3);
        assert!((v(&mut cache, "B.fkp") - 60. * 60. * 8.).abs() < 1e-3);

        // o corpo atravessa as placas: a análise acha, mas não conta como problema
        let achados = interferencia::analisar(&a, Some(&d), &mut cache, &OpcoesInterf::default());
        assert!(achados.iter().any(|x| x.volume > 1.), "o parafuso está mesmo dentro das placas");
        let rep = interferencia::separar_na_montagem(&a, &achados, lib());
        assert!(rep.ativos.is_empty(), "{:?}", rep.ativos.iter().map(|i| (&achados[*i].a.nome, &achados[*i].b.nome, achados[*i].volume)).collect::<Vec<_>>());
        assert!(!rep.ignorados.is_empty());
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn peca_que_invade_de_verdade_continua_aparecendo_com_alvo_por_ponto() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        // a tampa fica 2 mm acima da placa, além do vão da pega: a cabeça de 6 mm entra nela, e isso
        // não é a passagem do corpo, é colisão de verdade
        let (mut a, mut cache, d, ids) = montagem("colisao", &[("A", &lisa(8.), 0.), ("Tampa", &lisa(8.), 10.)]);
        a.node_mut(ids[1]).unwrap().placement.origin = [40., 0., 10.];
        let id = criar(&mut a, "Conjunto");
        let mut c = a.conjuntos[0].clone();
        c.superior = vec![];
        c.inferior = vec![];
        c.acrescentar(&mut a, [(TipoAlvo::Ponto { comp: ids[0], pos: [11., 0., 8.], dir: [0., 0., -1.] }, None)]);
        a.conjuntos[0] = c;
        sem_erro(&sincronizar(&mut a, &d, &mut cache, lib(), id));
        let achados = interferencia::analisar(&a, Some(&d), &mut cache, &OpcoesInterf::default());
        let rep = interferencia::separar_na_montagem(&a, &achados, lib());
        let tampa = |i: &usize| achados[*i].a.comp == ids[1] || achados[*i].b.comp == ids[1];
        assert!(achados.iter().any(|x| x.a.comp == ids[1] || x.b.comp == ids[1]), "a cabeça entra na tampa");
        assert!(rep.ativos.iter().any(tampa), "a colisão da cabeça segue na lista");
        assert!(!rep.ignorados.iter().any(|(i, _)| tampa(i)));
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn alvo_por_vetor_inclinado_segue_o_sentido_pedido_e_inverter_troca_o_lado() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (mut a, mut cache, d, ids) = montagem("vetor", &[("A", &lisa(10.), 0.)]);
        let id = criar(&mut a, "Conjunto");
        let mut c = a.conjuntos[0].clone();
        c.superior = vec![];
        c.inferior = vec![];
        let s = 45_f64.to_radians().sin();
        let u = [s, 0., -s];
        c.acrescentar(&mut a, [(TipoAlvo::Vetor { comp: ids[0], pos: [0., 0., 10.], dir: u }, None)]);
        a.conjuntos[0] = c;
        sem_erro(&sincronizar(&mut a, &d, &mut cache, lib(), id));
        let p = pilha_de(&a, id, 0).clone();
        let pl = a.node(p.pecas[0]).unwrap().placement;
        assert!(norm(sub(pl.origin, [0., 0., 10.])) < 1e-6, "cabeça no início do vetor: {:?}", pl.origin);
        assert!(norm(sub(pl.dir([0., 0., 1.]), u.map(|x| -x))) < 1e-6, "corpo no sentido do vetor");
        // a reta cruza 10 / sen 45 = 14,14 de material; + 2 passos = 16,14 → 20
        assert_eq!(p.comprimento, 20.);
        // o mesmo vetor invertido, saindo da mesma origem, entra pela outra ponta do material
        if let TipoAlvo::Vetor { dir, .. } = &mut a.conjuntos[0].alvos[0].tipo {
            *dir = dir.map(|x| -x);
        }
        sem_erro(&sincronizar(&mut a, &d, &mut cache, lib(), id));
        let q = pilha_de(&a, id, 0);
        assert_eq!(q.pecas[0], p.pecas[0], "mesma peça, refeita no lugar novo");
        let o = origem(&a, q.pecas[0]);
        assert!((o[2] - 0.).abs() < 1e-6 && (o[0] - 10.).abs() < 1e-6, "entrou por baixo: {o:?}");
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn acrescentar_alvo_depois_e_trocar_a_pilha_atualiza_todos() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (mut a, mut cache, d, ids) = montagem("depois", &[("A", &lisa(8.), 0.)]);
        let id = criar(&mut a, "Conjunto");
        let ponto = |x: f64| (TipoAlvo::Ponto { comp: ids[0], pos: [x, 0., 8.], dir: [0., 0., -1.] }, None);
        let mut c = a.conjuntos[0].clone();
        c.superior = vec![arruela()];
        c.inferior = vec![porca()];
        assert_eq!(c.acrescentar(&mut a, [ponto(-20.), ponto(0.)]), 2);
        a.conjuntos[0] = c;
        sem_erro(&sincronizar(&mut a, &d, &mut cache, lib(), id));
        let primeiros: Vec<Id> = pilha_de(&a, id, 0).pecas.clone();
        let total = a.nodes.len();

        // mais um ponto; repetir um que já está não duplica
        let mut c = a.conjuntos[0].clone();
        assert_eq!(c.acrescentar(&mut a, [ponto(20.), ponto(0.)]), 1);
        a.conjuntos[0] = c;
        let r = sincronizar(&mut a, &d, &mut cache, lib(), id);
        sem_erro(&r);
        assert_eq!(r.refeitas, 0, "os que já estavam não mudam");
        assert_eq!(a.nodes.len(), total + 3);
        assert_eq!(pilha_de(&a, id, 0).pecas, primeiros);

        // trocar a pilha e o diâmetro vale para os três
        a.conjuntos[0].tamanho = "M8".into();
        a.conjuntos[0].superior = vec![arruela(), arruela()];
        let r = sincronizar(&mut a, &d, &mut cache, lib(), id);
        sem_erro(&r);
        assert_eq!(r.refeitas, 3);
        for i in 0..3 {
            let p = pilha_de(&a, id, i);
            assert_eq!(p.pecas.len(), 4);
            assert!(p.pecas.iter().all(|x| a.name_of(*x).contains("M8")), "{:?}", p.pecas.iter().map(|x| a.name_of(*x)).collect::<Vec<_>>());
            let h = lib().familia("iso7089").unwrap().tamanho("M8").unwrap().valor("h").unwrap();
            assert!((origem(&a, p.pecas[0])[2] - (8. + 2. * h)).abs() < 1e-6);
        }
        assert_eq!(pilha_de(&a, id, 0).pecas[0], primeiros[0], "o parafuso segue sendo a mesma peça");

        // tirar um alvo leva as peças dele; as outras ficam
        let n = a.nodes.len();
        a.conjuntos[0].alvos.remove(1);
        sem_erro(&sincronizar(&mut a, &d, &mut cache, lib(), id));
        assert_eq!(a.nodes.len(), n - 4);
        assert_eq!(a.pilhas.len(), 2);

        // apagar uma peça na árvore tira o alvo do conjunto
        let parafuso = pilha_de(&a, id, 0).pecas[0];
        a.delete(parafuso);
        pilhas::recalcular(&mut a, &d, &mut cache, lib());
        assert_eq!(a.conjuntos[0].alvos.len(), 1);

        // apagar o conjunto leva tudo, e a peça do usuário fica
        apagar(&mut a, id);
        assert!(a.conjuntos.is_empty() && a.pilhas.is_empty());
        assert_eq!(a.nodes.iter().filter(|n| matches!(n.kind, NodeKind::Part { .. })).count(), 1);
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn prendedor_do_conjunto_e_peca_normal_e_entra_na_lista_de_pecas() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (mut a, mut cache, d, ids) = montagem("peca", &[("A", &lisa(8.), 0.)]);
        let id = criar(&mut a, "Conjunto");
        let mut c = a.conjuntos[0].clone();
        c.acrescentar(&mut a, [(TipoAlvo::Ponto { comp: ids[0], pos: [0., 0., 8.], dir: [0., 0., -1.] }, None)]);
        a.conjuntos[0] = c;
        sem_erro(&sincronizar(&mut a, &d, &mut cache, lib(), id));
        let p = pilha_de(&a, id, 0).clone();
        assert!(p.pecas.iter().all(|x| matches!(a.node(*x).unwrap().kind, NodeKind::Part { .. })));
        assert_eq!(da_peca(&a, p.pecas[0]), Some(id));
        assert!(p.pecas.iter().all(|x| a.node(*x).unwrap().parent.is_some()), "dentro do grupo de fixadores");
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn a_pega_acompanha_a_peca_que_engrossa() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (mut a, mut cache, d, ids) = montagem("engrossa", &[("A", &lisa(8.), 0.)]);
        let id = criar(&mut a, "Conjunto");
        let mut c = a.conjuntos[0].clone();
        c.superior = vec![];
        c.inferior = vec![porca()];
        c.acrescentar(&mut a, [(TipoAlvo::Ponto { comp: ids[0], pos: [0., 0., 0.], dir: [0., 0., 1.] }, None)]);
        a.conjuntos[0] = c;
        sem_erro(&sincronizar(&mut a, &d, &mut cache, lib(), id));
        let l0 = pilha_de(&a, id, 0).comprimento;
        std::fs::write(d.join("A.fkp"), lisa(30.).to_json()).unwrap();
        cache.forget(&d.join("A.fkp"));
        let r = pilhas::recalcular(&mut a, &d, &mut cache, lib());
        assert_eq!(r.refeitas, 1, "{:?}", r.diagnosticos);
        assert!(pilha_de(&a, id, 0).comprimento > l0 + 15.);
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn furo_com_rebaixo_mantem_o_assento_da_cabeca_no_conjunto() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let doc = crate::fixadores::tests::placa_com_boca(12., 6.6, None, HoleKind::Rebaixado { diameter: "11".into(), depth: "6".into() });
        let (mut a, mut cache, d, _) = montagem("assento", &[("A", &doc, 0.)]);
        let furos = disponiveis(&a, &d, &mut cache);
        let id = criar(&mut a, "Conjunto");
        let mut c = a.conjuntos[0].clone();
        c.superior = vec![];
        c.inferior = vec![porca()];
        c.acrescentar(&mut a, [(furos[0].tipo.clone(), None)]);
        a.conjuntos[0] = c;
        sem_erro(&sincronizar(&mut a, &d, &mut cache, lib(), id));
        let p = pilha_de(&a, id, 0);
        assert!((origem(&a, p.pecas[0])[2] - 6.).abs() < 1e-6, "cabeça no fundo do rebaixo: {:?}", origem(&a, p.pecas[0]));
        // pega 12 - 6 + porca 5,2 + 2 passos = 13,2 → a série do M6 tem 12 e 16
        assert_eq!(p.comprimento, 16.);
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn selecao_por_cor_acha_furos_e_pontos_da_cor_e_a_regra_da_o_tamanho() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let doc = colorida();
        let (mut a, mut cache, d, _) = montagem("cor", &[("A", &doc, 0.)]);
        let lista = disponiveis(&a, &d, &mut cache);
        assert_eq!(lista.len(), 4, "dois furos e dois pontos: {lista:?}");
        let azuis = por_cor(&lista, AZUL);
        assert_eq!(azuis.len(), 3, "um furo e dois pontos azuis");
        assert_eq!(azuis.iter().filter(|i| lista[**i].tipo.sem_furo()).count(), 2);
        let laranjas = por_cor(&lista, LARANJA);
        assert_eq!(laranjas.len(), 1);
        assert!(por_cor(&lista, [1, 2, 3]).is_empty());

        // regras: azul é M6, laranja é M5
        a.regras_cor = vec![RegraCor { cor: AZUL, nome: "M6".into(), diametro: 6., sistema: Sistema::Metrico }, RegraCor { cor: LARANJA, nome: "M5".into(), diametro: 5., sistema: Sistema::Metrico }];
        let id = criar(&mut a, "Conjunto");
        let mut c = a.conjuntos[0].clone();
        c.por_cor = true;
        c.superior = vec![arruela()];
        c.inferior = vec![];
        c.tamanho = "M4".into();
        c.acrescentar(&mut a, azuis.iter().chain(&laranjas).map(|i| (lista[*i].tipo.clone(), lista[*i].cor)));
        a.conjuntos[0] = c;
        let r = sincronizar(&mut a, &d, &mut cache, lib(), id);
        assert_eq!(a.conjuntos[0].alvos.len(), 4);
        let tamanhos: Vec<String> = (0..4).map(|i| pilha_de(&a, id, i).tamanho.clone()).collect();
        assert_eq!(tamanhos, ["M6", "M6", "M6", "M5"], "{:?}", r.diagnosticos);
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn furos_coaxiais_de_duas_pecas_sao_um_alvo_so() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (mut a, mut cache, d, _) = montagem("coaxial", &[("A", &colorida(), 8.), ("B", &colorida(), 0.)]);
        let lista = disponiveis(&a, &d, &mut cache);
        assert_eq!(lista.iter().filter(|x| !x.tipo.sem_furo()).count(), 2, "{lista:?}");
        let azuis = por_cor(&lista, AZUL);
        let id = criar(&mut a, "Conjunto");
        let mut c = a.conjuntos[0].clone();
        c.acrescentar(&mut a, azuis.iter().filter(|i| !lista[**i].tipo.sem_furo()).map(|i| (lista[*i].tipo.clone(), lista[*i].cor)));
        a.conjuntos[0] = c;
        sem_erro(&sincronizar(&mut a, &d, &mut cache, lib(), id));
        assert_eq!(a.pilhas.len(), 1, "um parafuso só no eixo");
        let p = pilha_de(&a, id, 0);
        // a pega são as duas placas: 16 + arruelas e porca
        assert!((origem(&a, p.pecas[0])[2] - 17.6).abs() < 1e-6, "{:?}", origem(&a, p.pecas[0]));
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn regras_de_fabrica_em_polegada_e_metrico_e_tamanho_que_falta_avisa() {
        let r = regras_padrao();
        let nomes: Vec<&str> = r.iter().map(|x| x.nome.as_str()).collect();
        for n in ["3/32", "1/8", "5/32", "M1", "M2", "M3"] {
            assert!(nomes.contains(&n), "{n}");
        }
        let oitavo = r.iter().find(|x| x.nome == "1/8").unwrap();
        assert!((oitavo.diametro - 3.175).abs() < 1e-9);
        assert_eq!(oitavo.sistema, Sistema::Polegada);
        let cores: Vec<[u8; 3]> = r.iter().map(|x| x.cor).collect();
        for (i, c) in cores.iter().enumerate() {
            assert!(!cores[i + 1..].iter().any(|o| mesma_cor(*c, *o)), "cor repetida: {c:?}");
        }
        let fam = lib().familia("iso4762").unwrap();
        let m3 = r.iter().find(|x| x.nome == "M3").unwrap();
        assert_eq!(tamanho_da_regra(fam, m3), Some(("M3".to_string(), true)));
        // 1/8" não existe na família: fica o mais próximo, marcado como não exato
        assert_eq!(tamanho_da_regra(fam, oitavo), Some(("M3".to_string(), false)));
    }

    #[test]
    fn regra_sem_o_tamanho_exato_avisa_na_linha_do_alvo() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (mut a, mut cache, d, ids) = montagem("aviso", &[("A", &lisa(8.), 0.)]);
        let id = criar(&mut a, "Conjunto");
        let oitavo = regras_padrao().into_iter().find(|x| x.nome == "1/8").unwrap();
        let mut c = a.conjuntos[0].clone();
        c.por_cor = true;
        c.acrescentar(&mut a, [(TipoAlvo::Ponto { comp: ids[0], pos: [0., 0., 8.], dir: [0., 0., -1.] }, Some(oitavo.cor))]);
        a.conjuntos[0] = c;
        let r = sincronizar(&mut a, &d, &mut cache, lib(), id);
        let p = pilha_de(&a, id, 0);
        assert_eq!(p.tamanho, "M3");
        let g = r.de(p.id).unwrap();
        assert_eq!(g.nivel, Nivel::Aviso);
        assert!(g.texto.contains("1/8"), "{}", g.texto);
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn salvar_e_reabrir_guarda_o_conjunto_e_segue_editavel() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (mut a, mut cache, d, ids) = montagem("salvar", &[("A", &placa(10., 6.6, None), 0.), ("L", &lisa(8.), 40.)]);
        let furos = disponiveis(&a, &d, &mut cache);
        a.regras_cor = regras_padrao();
        a.regras_cor[0].nome = "minha".into();
        let id = criar(&mut a, "Conjunto");
        let mut c = a.conjuntos[0].clone();
        cinco(&mut c);
        c.acrescentar(&mut a, [(furos[0].tipo.clone(), Some(AZUL))]);
        c.acrescentar(&mut a, [(TipoAlvo::Ponto { comp: ids[1], pos: [5., 5., 8.], dir: [0., 0., -1.] }, None)]);
        c.acrescentar(&mut a, [(TipoAlvo::Vetor { comp: ids[1], pos: [-5., 5., 8.], dir: [0., 0., -1.] }, None)]);
        a.conjuntos[0] = c;
        sem_erro(&sincronizar(&mut a, &d, &mut cache, lib(), id));

        let json = a.to_json();
        let mut b = Assembly::from_json(&json).unwrap();
        assert_eq!(b.conjuntos, a.conjuntos);
        assert_eq!(b.pilhas, a.pilhas);
        assert_eq!(b.regras_cor[0].nome, "minha");

        // reaberto, nada muda sozinho
        let mut cache2 = PartCache::default();
        let nos = b.nodes.clone();
        let r = pilhas::recalcular(&mut b, &d, &mut cache2, lib());
        assert_eq!(r.refeitas, 0, "{:?}", r.diagnosticos);
        assert_eq!(b.nodes, nos);

        // e continua mandando: trocar a pilha depois de reabrir atualiza os três
        b.conjuntos[0].superior.remove(0);
        let r = sincronizar(&mut b, &d, &mut cache2, lib(), id);
        sem_erro(&r);
        assert_eq!(r.refeitas, 3);
        assert!((0..3).all(|i| pilha_de(&b, id, i).pecas.len() == 5));
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn arquivo_sem_conjuntos_abre_como_antes() {
        let a = Assembly::new("velha");
        let mut v: serde_json::Value = serde_json::from_str(&a.to_json()).unwrap();
        let o = v.as_object_mut().unwrap();
        o.remove("conjuntos");
        o.remove("regras_cor");
        let b = Assembly::from_json(&v.to_string()).unwrap();
        assert!(b.conjuntos.is_empty() && b.regras_cor.is_empty());
        assert_eq!(regras_da(&b).len(), regras_padrao().len());
    }

    #[test]
    fn bloco_da_pega_junta_o_que_encosta_e_para_no_vao() {
        let t = |c: u64, ini: f64, fim: f64| pilhas::Trecho { comp: Id(c), ini, fim };
        let v = [t(1, 0., 8.), t(2, 8.5, 16.), t(3, 40., 50.)];
        let (i, f) = pilhas::bloco_da_pega(&v, 1.).unwrap();
        assert_eq!((i.comp, f.comp, f.fim), (Id(1), Id(2), 16.), "o vão de 0,5 entra, a peça distante não");
        // o ponto no meio do segundo bloco fica com ele
        let w = [t(1, -60., -50.), t(2, -4., 4.), t(3, 30., 40.)];
        assert_eq!(pilhas::bloco_da_pega(&w, 1.).unwrap().0.comp, Id(2));
        // no ar: o primeiro à frente, e só sem nada à frente o de trás
        let x = [t(1, -20., -10.), t(2, 5., 9.)];
        assert_eq!(pilhas::bloco_da_pega(&x, 1.).unwrap().0.comp, Id(2));
        assert_eq!(pilhas::bloco_da_pega(&x[..1], 1.).unwrap().0.comp, Id(1));
        assert!(pilhas::bloco_da_pega(&[], 1.).is_none());
    }

    #[test]
    fn peca_oca_tem_dois_trechos_e_a_pega_para_na_primeira_parede() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        // duas placas com um vão de 20 mm: a reta vê dois trechos, e a pega fica só na de cima
        let (mut a, mut cache, d, ids) = montagem("oca", &[("A", &lisa(8.), 28.), ("B", &lisa(8.), 0.)]);
        let tr = pilhas::trechos_na_reta(&a, &d, &mut cache, [0., 0., 36.], [0., 0., -1.]);
        assert_eq!(tr.len(), 2, "{tr:?}");
        assert!((tr[0].ini - 0.).abs() < 1e-6 && (tr[0].fim - 8.).abs() < 1e-6);
        let id = criar(&mut a, "Conjunto");
        let mut c = a.conjuntos[0].clone();
        c.superior = vec![];
        c.inferior = vec![];
        c.acrescentar(&mut a, [(TipoAlvo::Ponto { comp: ids[0], pos: [0., 0., 8.], dir: [0., 0., -1.] }, None)]);
        a.conjuntos[0] = c;
        sem_erro(&sincronizar(&mut a, &d, &mut cache, lib(), id));
        // 8 + 2 passos = 10 → a série do M6 começa em 12; com as duas placas seria 40
        assert_eq!(pilha_de(&a, id, 0).comprimento, 12.);
        // com o vão maior, a pega vai até a segunda placa
        a.conjuntos[0].vao = 25.;
        sem_erro(&sincronizar(&mut a, &d, &mut cache, lib(), id));
        assert_eq!(pilha_de(&a, id, 0).comprimento, 40., "8 + 20 + 8 + 2 = 38");
        let _ = std::fs::remove_dir_all(d);
    }
}
