// SPDX-License-Identifier: LGPL-2.1-or-later
//! Arquivo 2D para a folha do desenho técnico (B211). É a volta do que `fenkra_desenho::export::dxf`
//! grava: o DXF, o SVG ou o DWG (pelo LibreDWG externo, ADR 0090) vira um desenho com uma folha, e cada
//! camada vira um esboço solto nela.
//!
//! Na folha as coordenadas são milímetros sem escala, então o arquivo entra no lugar em que estava. O tipo
//! de linha de cada curva sai do nome da camada (as cinco que o Fenkra grava), do tipo de linha da camada
//! ou, por último, da espessura: o traço fino continua fino. Texto vira nota de texto. Hachura fica de fora
//! e o resumo conta quantas.
//!
//! O formato da folha é o menor que cabe o conteúdo a partir da origem. Um DXF que o Fenkra gravou de uma
//! folha A3 volta numa A3, e o que a folha nova já desenha (moldura, zonas, campos iguais da legenda) não entra
//! de novo.

use crate::dxf::{escalada, no_sketch, Pontos};
use crate::{ErroConv, Formato};
use fenkra_core::sketch::SketchData;
use fenkra_core::Id;
use fenkra_desenho::esboco::{DonoEsboco, EsbocoDesenho, TipoLinha};
use fenkra_desenho::{Drawing, Format, Note, NoteKind};
use fenkra_core::pmi::PmiStyle;
use fenkra_dxf::{Anexo, Desenho, Entidade, Item};
use std::collections::BTreeMap;
use std::path::Path;

/// O que entrou na folha.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ResumoFolha {
    /// esboços criados, um por camada com curva
    pub esbocos: usize,
    pub curvas: usize,
    /// notas de texto criadas
    pub textos: usize,
    /// o que o arquivo trazia e a folha não representa: tipo e quantidade
    pub fora: BTreeMap<String, usize>,
    /// o arquivo não declarava a unidade e valeu milímetro
    pub sem_unidade: bool,
    /// pontas a menos da tolerância que viraram o mesmo ponto
    pub juntados: usize,
    /// entidades iguais às da moldura e da legenda da folha nova, que a própria folha já desenha
    pub da_moldura: usize,
}

/// Folga que junta duas pontas no mesmo ponto, em mm da folha.
pub const TOLERANCIA_FOLHA: f64 = 1e-6;

/// Lê um arquivo 2D (DXF, SVG ou DWG) no desenho do crate DXF. O DWG passa pelo LibreDWG externo.
pub fn ler_2d(caminho: &Path) -> Result<(Desenho, Formato), ErroConv> {
    let bytes = std::fs::read(caminho).map_err(|e| ErroConv::Leitura(format!("não foi possível ler '{}': {e}", caminho.display())))?;
    let (formato, _) = crate::detectar(caminho, &bytes)?;
    let d = match formato {
        Formato::Dxf => fenkra_dxf::ler(&bytes).map_err(ErroConv::Leitura)?,
        Formato::Svg => crate::plano::ler_svg(&bytes)?,
        outro => return Err(ErroConv::Formato(format!("{} não é desenho 2D: use Importar para trazer a peça", outro.nome()))),
    };
    Ok((d, formato))
}

/// Tipo de linha pela camada e pela espessura do item.
fn tipo_de(item: &Item, camadas: &BTreeMap<&str, (&str, Option<i16>)>) -> TipoLinha {
    let nome = item.camada.to_uppercase();
    match nome.as_str() {
        "VISIVEL" | "MOLDURA" => return TipoLinha::Visivel,
        "FINA" => return TipoLinha::Fina,
        "OCULTA" => return TipoLinha::Oculta,
        "CENTRO" => return TipoLinha::Centro,
        _ => {}
    }
    let (linha, espessura_camada) = camadas.get(item.camada.as_str()).copied().unwrap_or(("", None));
    let linha = linha.to_uppercase();
    if ["HIDDEN", "DASH", "OCULT", "TRACEJ"].iter().any(|t| linha.contains(t) && !linha.contains("DOT")) {
        return TipoLinha::Oculta;
    }
    if ["CENTER", "CENTRO", "DASHDOT", "PHANTOM"].iter().any(|t| linha.contains(t)) {
        return TipoLinha::Centro;
    }
    match item.espessura.or(espessura_camada) {
        Some(e) if (0..35).contains(&e) => TipoLinha::Fina,
        _ => TipoLinha::Visivel,
    }
}

/// Nota que desenha o texto onde o arquivo o pôs. A nota prende a caixa pelo canto de baixo do lado do
/// anexo, e a caixa tem uma folga de 0,45 da altura dos lados e 0,4 embaixo: a posição desconta a folga, para o
/// texto cair no ponto de inserção e voltar igual na próxima gravação.
fn nota_do_texto(id: Id, folha: Id, pos: [f64; 2], altura: f64, angulo: f64, anexo: Anexo, texto: &str) -> Note {
    let (canto, justifica, dx) = match anexo {
        Anexo::Esquerda => (6, 0, -0.45),
        Anexo::Centro => (7, 1, 0.),
        Anexo::Direita => (8, 2, 0.45),
    };
    let style = PmiStyle { anchor: canto, justify: justifica, angle: angulo as f32, ..Default::default() };
    Note { id, sheet: folha, kind: NoteKind::Text, pos: [pos[0] + dx * altura, pos[1] - 0.4 * altura], text: texto.to_string(), size: altura, style, ..Default::default() }
}

/// Polilinha reta com trechos de arco amostrados volta a arco e círculo. O SVG grava curva como pontos.
fn com_arcos(des: &mut Desenho) {
    let mut saida = Vec::with_capacity(des.itens.len());
    for item in des.itens.drain(..) {
        let pts = match &item.entidade {
            Entidade::Polilinha { vertices, fechada } if vertices.len() >= 4 && vertices.iter().all(|v| v.bojo == 0.) => {
                let mut pts: Vec<[f64; 2]> = vertices.iter().map(|v| v.p).collect();
                if *fechada {
                    pts.push(pts[0]);
                }
                pts
            }
            _ => {
                saida.push(item);
                continue;
            }
        };
        let partes = fenkra_dxf::reconhecer(&pts, fenkra_dxf::TOLERANCIA_ARCO);
        if partes.iter().any(|e| matches!(e, Entidade::Arco { .. } | Entidade::Circulo { .. })) {
            saida.extend(partes.into_iter().map(|e| Item { entidade: e, ..item.clone() }));
        } else {
            saida.push(item);
        }
    }
    des.itens = saida;
}

/// Menor formato da folha (deitada, depois em pé) que cabe a caixa a partir da origem.
pub fn formato_que_cabe(hi: [f64; 2]) -> (Format, bool) {
    const ORDEM: [Format; 5] = [Format::A4, Format::A3, Format::A2, Format::A1, Format::A0];
    let folga = 1e-6;
    for f in ORDEM {
        let [l, a] = f.size();
        if hi[0] <= l + folga && hi[1] <= a + folga {
            return (f, false);
        }
        if hi[0] <= a + folga && hi[1] <= l + folga {
            return (f, true);
        }
    }
    (Format::A0, hi[1] > hi[0])
}

/// Põe o conteúdo do arquivo na folha: um esboço por camada, notas para os textos.
pub fn importar_na_folha(d: &mut Drawing, folha: Id, des: &Desenho) -> ResumoFolha {
    let escala = des.unidade.em_mm().unwrap_or(1.);
    let mut r = ResumoFolha { sem_unidade: des.unidade.em_mm().is_none(), fora: des.ignorados.clone(), ..Default::default() };
    let camadas: BTreeMap<&str, (&str, Option<i16>)> = des.camadas.iter().map(|c| (c.nome.as_str(), (c.tipo_linha.as_str(), c.espessura))).collect();
    let mut ordem: Vec<&str> = Vec::new();
    let mut por_camada: BTreeMap<&str, Vec<&Item>> = BTreeMap::new();
    for item in &des.itens {
        if !por_camada.contains_key(item.camada.as_str()) {
            ordem.push(item.camada.as_str());
        }
        por_camada.entry(item.camada.as_str()).or_default().push(item);
    }
    for camada in ordem {
        let mut sk = SketchData::default();
        let mut tipos: Vec<(Id, TipoLinha)> = Vec::new();
        let mut pts = Pontos::novo(TOLERANCIA_FOLHA);
        for item in &por_camada[camada] {
            let e = escalada(&item.entidade, escala);
            if let Entidade::Texto { pos, altura, angulo, anexo, texto } = &e {
                let id = d.alloc();
                d.notes.push(nota_do_texto(id, folha, *pos, *altura, *angulo, *anexo, texto));
                r.textos += 1;
                continue;
            }
            let antes = sk.curves.len();
            let mut al = || d.alloc();
            if no_sketch(&mut sk, &mut al, &mut pts, &e) {
                let tipo = tipo_de(item, &camadas);
                if tipo != TipoLinha::Visivel {
                    tipos.extend(sk.curves[antes..].iter().map(|c| (c.id, tipo)));
                }
            } else {
                *r.fora.entry("HATCH".into()).or_insert(0) += 1;
            }
        }
        r.juntados += pts.juntados;
        if sk.curves.is_empty() {
            continue;
        }
        r.curvas += sk.curves.len();
        r.esbocos += 1;
        let id = d.alloc();
        let nome = if camada.is_empty() { format!("Esboço {}", d.esbocos.len() + 1) } else { camada.to_string() };
        // a cota do arquivo já veio como linhas e texto: o esboço não desenha outra
        d.esbocos.push(EsbocoDesenho { id, nome, dono: DonoEsboco::Folha(folha), dados: sk, tipos, cotas: false });
    }
    r
}

/// Caixa do conteúdo em mm: das curvas e das posições dos textos.
pub fn caixa(des: &Desenho) -> Option<([f64; 2], [f64; 2])> {
    let k = des.unidade.em_mm().unwrap_or(1.);
    let mut pts: Vec<[f64; 2]> = Vec::new();
    for item in &des.itens {
        match &item.entidade {
            Entidade::Linha(a, b) => pts.extend([*a, *b]),
            Entidade::Circulo { centro, raio } | Entidade::Arco { centro, raio, .. } => pts.extend([[centro[0] - raio, centro[1] - raio], [centro[0] + raio, centro[1] + raio]]),
            Entidade::Polilinha { vertices, .. } => pts.extend(vertices.iter().map(|v| v.p)),
            Entidade::Elipse { centro, maior, .. } => {
                let r = maior[0].hypot(maior[1]);
                pts.extend([[centro[0] - r, centro[1] - r], [centro[0] + r, centro[1] + r]]);
            }
            Entidade::Spline { controle, ajuste, .. } => pts.extend(controle.iter().chain(ajuste).copied()),
            Entidade::Ponto(p) => pts.push(*p),
            Entidade::Texto { pos, .. } => pts.push(*pos),
            Entidade::Hachura { lacos } => pts.extend(lacos.iter().flatten().copied()),
        }
    }
    let lo = pts.iter().fold(None, |a: Option<[f64; 2]>, p| Some(a.map_or(*p, |a| [a[0].min(p[0]), a[1].min(p[1])])))?;
    let hi = pts.iter().fold(lo, |a, p| [a[0].max(p[0]), a[1].max(p[1])]);
    Some(([lo[0] * k, lo[1] * k], [hi[0] * k, hi[1] * k]))
}

/// Chave de comparação de uma entidade, com as medidas a 0,001 mm.
fn chave(i: &Item) -> String {
    let q = |v: f64| format!("{:.3}", v + 0.);
    let p = |a: &[f64; 2]| format!("{},{}", q(a[0]), q(a[1]));
    let e = match &i.entidade {
        Entidade::Linha(a, b) => format!("L{}{}", p(a), p(b)),
        Entidade::Circulo { centro, raio } => format!("C{}{}", p(centro), q(*raio)),
        Entidade::Arco { centro, raio, ini, fim } => format!("A{}{}{}{}", p(centro), q(*raio), q(*ini), q(*fim)),
        Entidade::Polilinha { vertices, fechada } => format!("P{fechada}{}", vertices.iter().map(|v| format!("{}{}", p(&v.p), q(v.bojo))).collect::<String>()),
        Entidade::Texto { pos, altura, texto, .. } => format!("T{}{}{texto}", p(pos), q(*altura)),
        outra => format!("{outra:?}"),
    };
    format!("{}|{e}", i.camada)
}

/// Tira do arquivo o que a folha nova já desenha (moldura, zonas e campos da legenda iguais). Um DXF que o
/// Fenkra gravou de uma folha traz tudo isso, e sem tirar cada número de zona viraria uma nota.
fn sem_a_moldura(des: &mut Desenho, d: &Drawing, folha: Id) -> usize {
    let mut m = fenkra_desenho::source::Models::new(None);
    let base = fenkra_desenho::export::dxf(&fenkra_desenho::layout::sheet(d, folha, &mut m));
    let Ok(base) = fenkra_dxf::ler(base.as_bytes()) else { return 0 };
    if des.unidade.em_mm().unwrap_or(1.) != 1. {
        return 0;
    }
    let mut conta: BTreeMap<String, usize> = BTreeMap::new();
    for i in &base.itens {
        *conta.entry(chave(i)).or_insert(0) += 1;
    }
    let antes = des.itens.len();
    des.itens.retain(|i| match conta.get_mut(&chave(i)) {
        Some(n) if *n > 0 => {
            *n -= 1;
            false
        }
        _ => true,
    });
    antes - des.itens.len()
}

/// Abre um arquivo 2D como desenho novo, numa folha do tamanho do conteúdo.
pub fn abrir_como_desenho(caminho: &Path) -> Result<(Drawing, ResumoFolha), ErroConv> {
    let (mut des, formato) = ler_2d(caminho)?;
    if formato == Formato::Svg {
        com_arcos(&mut des);
    }
    let nome = caminho.file_stem().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "Desenho".into());
    let (fmt, em_pe) = caixa(&des).map_or((Format::A3, false), |(_, hi)| formato_que_cabe(hi));
    let mut d = Drawing::new(&nome, fmt);
    if let Some(s) = d.sheets.first_mut() {
        s.portrait = em_pe;
    }
    let folha = d.sheets[0].id;
    let da_moldura = sem_a_moldura(&mut des, &d, folha);
    let mut r = importar_na_folha(&mut d, folha, &des);
    r.da_moldura = da_moldura;
    if r.esbocos == 0 && r.textos == 0 {
        return Err(ErroConv::Leitura(format!("o {} '{}' não tem curvas nem textos para a folha", formato.nome(), caminho.display())));
    }
    Ok((d, r))
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn formato_e_o_menor_que_cabe_e_deita_ou_levanta() {
        assert_eq!(formato_que_cabe([420., 297.]), (Format::A3, false));
        assert_eq!(formato_que_cabe([200., 280.]), (Format::A4, true));
        assert_eq!(formato_que_cabe([500., 100.]), (Format::A2, false));
        assert_eq!(formato_que_cabe([5000., 100.]), (Format::A0, false));
    }

    #[test]
    fn tipo_vem_da_camada_do_fenkra_do_tipo_de_linha_ou_da_espessura() {
        let camadas: BTreeMap<&str, (&str, Option<i16>)> = [("EIXOS", ("CENTER2", None)), ("ESCONDIDA", ("HIDDEN", None)), ("COTAS", ("Continuous", Some(18)))].into_iter().collect();
        let item = |c: &str| Item::novo(c, Entidade::Ponto([0., 0.]));
        assert_eq!(tipo_de(&item("oculta"), &camadas), TipoLinha::Oculta);
        assert_eq!(tipo_de(&item("EIXOS"), &camadas), TipoLinha::Centro);
        assert_eq!(tipo_de(&item("ESCONDIDA"), &camadas), TipoLinha::Oculta);
        assert_eq!(tipo_de(&item("COTAS"), &camadas), TipoLinha::Fina);
        assert_eq!(tipo_de(&item("0"), &camadas), TipoLinha::Visivel);
    }
}
