// SPDX-License-Identifier: LGPL-2.1-or-later
//! Corte de encaixe (B155) lido do documento: acha as pontas de membros com tratamento "Encaixe",
//! monta a seção e os eixos de cada membro e chama `st_encaixe::calcular`.
//!
//! Como funciona: o recompute trata cada par de linhas que dividem uma ponta (`shared_end`) com a
//! regra do comando posterior (`joint_rule`). Quando o tratamento é Encaixe, quem continua é o
//! hospedeiro e o outro membro é cortado. Aqui o mesmo par vira dados para a máquina: a curva da
//! ponta cortada contra todos os hospedeiros que chegam ao mesmo nó (o de maior alcance segura a
//! ponta). As opções (parede, orçamento, perpendicular) valem para todas as pontas; a folga vem da
//! regra do canto, a mesma que o 3D já usa.

use crate::st_dobra_corte::Desenho;
use crate::st_encaixe::{calcular, tubo_aberto, CorteEncaixe, Membro, Opcoes, Secao};
use crate::{geo3d, shared_end, Ctx, MemberGeom};
use fenkra_core::{CornerMode, Document, FeatureKind, Id, SectionKind};
use std::cell::RefCell;
use std::collections::HashMap;

/// Uma ponta cortada: a curva e os hospedeiros que a seguram.
#[derive(Clone, Debug)]
pub struct PontaEncaixe {
    pub corte: CorteEncaixe,
    /// nó do esqueleto onde a ponta encosta
    pub no: [f64; 3],
    /// direção do eixo do membro cortado, do nó para dentro dele
    pub dir: [f64; 3],
    /// de onde cada hospedeiro vem: (comando, linha)
    pub hospedeiros: Vec<(Id, usize)>,
}

/// Um membro reto com encaixe numa ou nas duas pontas.
#[derive(Clone, Debug)]
pub struct MembroComEncaixe {
    pub feature: Id,
    pub linha: usize,
    pub designacao: String,
    /// distância entre os nós das duas pontas
    pub comprimento: f64,
    pub ponta1: Option<PontaEncaixe>,
    pub ponta2: Option<PontaEncaixe>,
}

impl MembroComEncaixe {
    /// O contorno plano do tubo aberto com os encaixes (para DXF e SVG).
    pub fn desenho(&self) -> Desenho {
        let titulo = format!("Encaixe · {} · linha {}", self.designacao, self.linha + 1);
        tubo_aberto(&titulo, self.comprimento, self.ponta1.as_ref().map(|p| &p.corte), self.ponta2.as_ref().map(|p| &p.corte))
    }
    /// Comprimento de corte dos encaixes (somando as pontas), em mm.
    pub fn comprimento_de_corte(&self) -> f64 {
        [&self.ponta1, &self.ponta2].into_iter().flatten().map(|p| p.corte.comprimento_corte()).sum()
    }
    pub fn pontas(&self) -> impl Iterator<Item = &PontaEncaixe> {
        [&self.ponta1, &self.ponta2].into_iter().flatten()
    }
}

fn secao_do_membro(kind: SectionKind, g: &MemberGeom) -> Option<Secao> {
    Some(match kind {
        SectionKind::TuboRedondo => Secao::Redonda { d: g.w, t: g.wall },
        SectionKind::BarraRedonda => Secao::Redonda { d: g.w, t: 0. },
        SectionKind::TuboQuadrado | SectionKind::TuboRetangular => Secao::Retangular { b: g.w, h: g.h, t: g.wall },
        SectionKind::BarraQuadrada | SectionKind::BarraChata => Secao::Retangular { b: g.w, h: g.h, t: 0. },
        _ => return None,
    })
}

fn membro(doc: &Document, id: Id, g: &MemberGeom, no: [f64; 3], para_dentro: [f64; 3]) -> Option<Membro> {
    let FeatureKind::StructMember(m) = &doc.feature(id)?.kind else { return None };
    Some(Membro {
        secao: secao_do_membro(m.kind, g)?,
        ponto: no,
        x: g.x,
        y: g.y,
        para_dentro,
        // a seção nasce centrada na caixa e `shift` a desloca em relação à linha guia
        centro: [-g.shift[0], -g.shift[1]],
        comprimento: geo3d::dist(g.start, g.end),
    })
}

/// Todas as pontas de membros retos com encaixe, calculadas com `opc` (a folga vem de cada canto).
/// Pontas que não dão curva (seção não suportada, eixos paralelos) ficam de fora.
pub fn encaixes_da_estrutura(doc: &Document, opc: &Opcoes) -> Vec<MembroComEncaixe> {
    let Ok(vars) = doc.eval_params() else { return vec![] };
    let cx = Ctx {
        doc,
        vars,
        qualidade: Default::default(),
        bodies: HashMap::new(),
        sketches: HashMap::new(),
        status: HashMap::new(),
        tools: HashMap::new(),
        memo: RefCell::new(HashMap::new()),
        notes: Default::default(),
        body_sig: HashMap::new(),
        last_sheet: None,
        sheet_maps: vec![],
        cut_outlines: vec![],
        roscas: vec![],
        leve: None,
        relatorios_forma: vec![],
        formas_resolvidas: vec![],
        malhas: HashMap::new(),
        steps: HashMap::new(),
        miter_next: false,
        promoted: RefCell::new(None),
    };
    let lines = cx.all_member_lines();
    // (linha cortada, índice da ponta) -> hospedeiros e a folga do canto
    type Chave = (usize, bool);
    let mut grupos: Vec<(Chave, [f64; 3], [f64; 3], Vec<usize>, f64)> = vec![];
    for a in 0..lines.len() {
        for b in a + 1..lines.len() {
            let Some((p, da, db)) = shared_end(&lines[a].3, &lines[b].3) else { continue };
            let (la, lb) = (&lines[a], &lines[b]);
            let Ok((modo, a_continua, folga)) = cx.joint_rule((la.0, la.1, la.2), (lb.0, lb.1, lb.2)) else { continue };
            if modo != CornerMode::Encaixe {
                continue;
            }
            let (corte, host, dir_corte) = if a_continua { (b, a, db) } else { (a, b, da) };
            if !lines[corte].3.straight || !lines[host].3.straight {
                continue;
            }
            let g = &lines[corte].3;
            let fim = geo3d::dist(p, g.start) >= 1e-4;
            match grupos.iter_mut().find(|x| x.0 == (corte, fim)) {
                Some(x) => {
                    x.3.push(host);
                    x.4 = x.4.max(folga);
                }
                None => grupos.push(((corte, fim), p, dir_corte, vec![host], folga)),
            }
        }
    }
    let mut out: Vec<MembroComEncaixe> = vec![];
    for ((corte, fim), p, dir, hosts, folga) in grupos {
        let lc = &lines[corte];
        let Some(m) = membro(doc, lc.0, &lc.3, p, dir) else { continue };
        let mut hs = vec![];
        let mut origem = vec![];
        for h in hosts {
            let lh = &lines[h];
            // o hospedeiro se estende do nó para dentro dele
            let para_dentro = if geo3d::dist(p, lh.3.start) < 1e-4 { lh.3.z } else { geo3d::mul(lh.3.z, -1.) };
            if let Some(mh) = membro(doc, lh.0, &lh.3, p, para_dentro) {
                hs.push(mh);
                origem.push((lh.0, lh.1));
            }
        }
        if hs.is_empty() {
            continue;
        }
        let Ok(c) = calcular(&m, &hs, Opcoes { folga, ..*opc }) else { continue };
        let ponta = PontaEncaixe { corte: c, no: p, dir, hospedeiros: origem };
        let desig = match doc.feature(lc.0).map(|f| &f.kind) {
            Some(FeatureKind::StructMember(sm)) => format!("{} {}", sm.family, sm.designation),
            _ => String::new(),
        };
        let slot = match out.iter_mut().find(|x| x.feature == lc.0 && x.linha == lc.1) {
            Some(x) => x,
            None => {
                out.push(MembroComEncaixe { feature: lc.0, linha: lc.1, designacao: desig, comprimento: m.comprimento, ponta1: None, ponta2: None });
                out.last_mut().unwrap()
            }
        };
        if fim {
            slot.ponta2 = Some(ponta);
        } else {
            slot.ponta1 = Some(ponta);
        }
    }
    out
}
