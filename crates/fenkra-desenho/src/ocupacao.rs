// SPDX-License-Identifier: LGPL-2.1-or-later
//! B415: o que cabe na folha. Tabelas não podem cair sobre a legenda nem sair da moldura (o `pos`
//! da tabela é o canto superior esquerdo, com y contado de baixo, então um y pequeno faz a tabela
//! descer para dentro da legenda ou para fora da folha). Aqui ficam a área livre, a conferência
//! com aviso, a realocação e a escolha da escala automática em lista padronizada.

use crate::layout;
use crate::novo::ESCALAS_AUTO;
use crate::source::Models;
use crate::{Drawing, Id, Sheet, TitleStyle, V2};

/// Folga entre a tabela e o que está em volta (mm).
const FOLGA: f64 = 5.;

/// Caixa da legenda (carimbo), do canto inferior esquerdo ao superior direito, na folha.
/// No estilo ANSI inclui o quadro de tolerâncias gerais à esquerda.
pub fn legenda_caixa(estilo: TitleStyle, folha: V2) -> (V2, V2) {
    let (largura, altura) = match estilo {
        TitleStyle::Iso7200 => (180., 36.),
        TitleStyle::Abnt => (175., 50.),
        TitleStyle::Ansi => (178. + 70., 40.),
        TitleStyle::Simples => (150., 18.),
    };
    let (x1, y0) = (folha[0] - 10., 10.);
    ([x1 - largura, y0], [x1, y0 + altura])
}

/// Área dentro da moldura (margem de arquivamento de 20 mm à esquerda, 10 mm nos demais lados).
pub fn area_moldura(sh: &Sheet) -> (V2, V2) {
    let [w, h] = sh.size();
    ([20., 10.], [w - 10., h - 10.])
}

fn cruza(a: (V2, V2), b: (V2, V2)) -> bool {
    a.0[0] < b.1[0] - 1e-9 && b.0[0] < a.1[0] - 1e-9 && a.0[1] < b.1[1] - 1e-9 && b.0[1] < a.1[1] - 1e-9
}

fn dentro(a: (V2, V2), area: (V2, V2)) -> bool {
    a.0[0] >= area.0[0] - 1e-9 && a.0[1] >= area.0[1] - 1e-9 && a.1[0] <= area.1[0] + 1e-9 && a.1[1] <= area.1[1] + 1e-9
}

/// O que há de errado com a posição da tabela.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Problema {
    /// invade a legenda
    SobreLegenda,
    /// passa da moldura da folha
    ForaDaFolha,
}

/// Aviso de uma tabela mal posicionada.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct AvisoTabela {
    pub tabela: Id,
    pub problema: Problema,
}

impl AvisoTabela {
    /// Texto para a pessoa (a interface traduz pela mesma frase).
    pub fn texto(&self) -> &'static str {
        match self.problema {
            Problema::SobreLegenda => "A tabela está sobre a legenda da folha.",
            Problema::ForaDaFolha => "A tabela sai da moldura da folha.",
        }
    }
}

/// Caixa que a tabela ocupa na folha onde ela mora (com todos os blocos lado a lado).
pub fn caixa_da_tabela(m: &mut Models, d: &Drawing, id: Id) -> Option<(V2, V2)> {
    let t = d.tables.iter().find(|t| t.id == id)?;
    layout::tabela_caixa(m, d, t, t.pos)
}

/// Confere todas as tabelas do desenho.
pub fn avisos_de_tabelas(m: &mut Models, d: &Drawing) -> Vec<AvisoTabela> {
    let mut out = vec![];
    for t in &d.tables {
        let Some(sh) = d.sheet(t.sheet) else { continue };
        let Some((lo, hi)) = layout::tabela_caixa(m, d, t, t.pos) else { continue };
        let area = area_moldura(sh);
        if !dentro((lo, hi), area) {
            out.push(AvisoTabela { tabela: t.id, problema: Problema::ForaDaFolha });
        } else if cruza((lo, hi), legenda_caixa(d.title.style, sh.size())) {
            out.push(AvisoTabela { tabela: t.id, problema: Problema::SobreLegenda });
        }
    }
    out
}

/// Caixa de cada vista na folha (para a tabela não cair sobre o desenho).
fn caixas_das_vistas(m: &mut Models, d: &Drawing, folha: Id) -> Vec<(V2, V2)> {
    let ids: Vec<Id> = d.views.iter().filter(|v| v.sheet == folha).map(|v| v.id).collect();
    ids.into_iter()
        .filter_map(|id| {
            let v = d.view(id)?;
            let p = m.proj(d, id);
            if p.lines.is_empty() {
                return None;
            }
            let (a, b) = (layout::to_sheet(v, &p, p.lo), layout::to_sheet(v, &p, p.hi));
            Some(([a[0].min(b[0]), a[1].min(b[1])], [a[0].max(b[0]), a[1].max(b[1])]))
        })
        .collect()
}

/// Leva a tabela para um canto livre da folha: fora da legenda, dentro da moldura e, se houver
/// lugar, sem cobrir vistas nem outras tabelas. Devolve a posição nova (canto superior esquerdo),
/// ou `None` quando nem a folha vazia comporta a tabela (aí só resta avisar).
pub fn posicao_livre(m: &mut Models, d: &Drawing, id: Id) -> Option<V2> {
    let t = d.tables.iter().find(|t| t.id == id)?;
    let sh = d.sheet(t.sheet)?;
    let (lo, hi) = layout::tabela_caixa(m, d, t, t.pos)?;
    let (w, h) = (hi[0] - lo[0], hi[1] - lo[1]);
    let area = area_moldura(sh);
    let legenda = legenda_caixa(d.title.style, sh.size());
    let vistas = caixas_das_vistas(m, d, t.sheet);
    let outras: Vec<(V2, V2)> = d.tables.iter().filter(|o| o.id != id && o.sheet == t.sheet).filter_map(|o| layout::tabela_caixa(m, d, o, o.pos)).collect();
    // cantos candidatos (canto superior esquerdo da tabela): alto à direita, alto à esquerda,
    // acima da legenda à direita, acima da legenda à esquerda
    let (x_esq, x_dir) = (area.0[0] + FOLGA, area.1[0] - FOLGA - w);
    let (y_alto, y_baixo) = (area.1[1] - FOLGA, legenda.1[1] + FOLGA + h);
    let candidatos = [[x_dir, y_alto], [x_esq, y_alto], [x_dir, y_baixo], [x_esq, y_baixo]];
    let caixa = |p: V2| ([p[0], p[1] - h], [p[0] + w, p[1]]);
    let valido = |p: V2, estrito: bool| {
        let c = caixa(p);
        dentro(c, area) && !cruza(c, legenda) && (!estrito || (!vistas.iter().any(|v| cruza(c, *v)) && !outras.iter().any(|o| cruza(c, *o))))
    };
    candidatos.iter().copied().find(|p| valido(*p, true)).or_else(|| candidatos.iter().copied().find(|p| valido(*p, false)))
}

/// Realoca as tabelas que caem sobre a legenda ou fora da folha. Devolve as que mudaram de lugar.
/// As que não cabem em lugar nenhum ficam como estão (o aviso continua valendo).
pub fn realocar_tabelas(m: &mut Models, d: &mut Drawing) -> Vec<Id> {
    let mut mudou = vec![];
    for a in avisos_de_tabelas(m, d) {
        if let Some(p) = posicao_livre(m, d, a.tabela) {
            if let Some(t) = d.tables.iter_mut().find(|t| t.id == a.tabela) {
                t.pos = p;
                mudou.push(a.tabela);
            }
        }
    }
    mudou
}

/// Maior escala da lista padronizada em que um conjunto de `largura` x `altura` (mm do modelo, em
/// escala 1) cabe na área `area_w` x `area_h` (mm da folha). Considera os dois lados; se nem a
/// menor da lista serve, devolve a menor e `false`.
pub fn escala_que_cabe(largura: f64, altura: f64, area_w: f64, area_h: f64) -> (f64, bool) {
    if largura <= 0. || altura <= 0. {
        return (1., true);
    }
    match ESCALAS_AUTO.iter().copied().find(|s| largura * s <= area_w && altura * s <= area_h) {
        Some(s) => (s, true),
        None => (ESCALAS_AUTO[ESCALAS_AUTO.len() - 1], false),
    }
}
