// SPDX-License-Identifier: LGPL-2.1-or-later
//! `verificar` (B418): confere os requisitos declarados na peça e devolve um resultado por requisito,
//! com o que foi medido. Não muda a peça: a propagação de parâmetro roda numa cópia.
//!
//! Como cada conferência funciona:
//! - massa: soma do volume de cada corpo vezes a densidade do material dele;
//! - furos: o reconhecimento de features da forma final (`reconhece`), contando os furos de diâmetro
//!   dentro da tolerância;
//! - interferência: volume comum (booleana de interseção) de cada par de corpos;
//! - folga: distância dos vértices da malha fina de um corpo até a superfície exata do outro, nos dois
//!   sentidos. É amostrada: entre duas faces curvas, o valor real pode ser um pouco menor que o
//!   medido, no máximo a flecha da malha;
//! - caixa: caixa envolvente da forma final;
//! - propagação: troca a expressão do parâmetro, recalcula a cópia e compara a medida.

use crate::{final_shape, recompute, reconhece, Status};
use fenkra_core::requisitos::{Medida, Requisito, TipoRequisito};
use fenkra_core::{materiais, Document, Id};
use fenkra_kernel::{Bool, Shape};

/// Resultado de um requisito.
#[derive(Clone, Debug, PartialEq)]
pub struct Resultado {
    pub nome: String,
    pub tipo: &'static str,
    pub ok: bool,
    /// o que foi medido, em texto curto com unidade
    pub medido: String,
    /// por que falhou, ou o que foi conferido
    pub detalhe: String,
}

/// Tamanho da caixa envolvente. A malha que outra conferência (furos, folga) deixou na forma engorda
/// a caixa do OpenCASCADE em centésimos de mm; sem ela, a caixa é a exata.
fn caixa(s: &Shape) -> Option<[f64; 3]> {
    let _ = s.discard_mesh();
    s.bbox().map(|(a, b)| [b[0] - a[0], b[1] - a[1], b[2] - a[2]])
}

/// Forma de cada corpo visível, calculada com só ele visível.
fn corpos(doc: &Document) -> Vec<(Id, String, Shape)> {
    doc.collectors
        .iter()
        .filter(|c| c.visible)
        .filter_map(|c| {
            let mut d = doc.clone();
            for o in &mut d.collectors {
                o.visible = o.id == c.id;
            }
            final_shape(&d).filter(|s| !s.is_null()).map(|s| (c.id, c.name.clone(), s))
        })
        .collect()
}

fn massa(doc: &Document) -> (f64, bool) {
    let r = recompute(doc);
    let m = r.bodies.iter().map(|b| materiais::massa_kg(doc, b.collector, b.mass.volume)).sum::<f64>();
    (m.max(0.), !r.bodies.is_empty())
}

/// Menor distância entre as superfícies de `a` e `b`; negativa se um vértice de um está dentro do outro.
fn folga(a: &Shape, b: &Shape) -> Option<f64> {
    let diag = |s: &Shape| s.bbox().map(|(p, q)| ((q[0] - p[0]).powi(2) + (q[1] - p[1]).powi(2) + (q[2] - p[2]).powi(2)).sqrt()).unwrap_or(100.);
    let lado = |de: &Shape, ate: &Shape| -> Option<f64> {
        let m = de.mesh(diag(de) * 1e-3).ok()?;
        let pts: Vec<[f64; 3]> = m.positions.chunks_exact(3).map(|p| [p[0] as f64, p[1] as f64, p[2] as f64]).collect();
        let d = ate.signed_distances(&pts).ok()?;
        // vértice dentro do outro corpo: a distância com sinal fica negativa
        if d.iter().any(|x| *x < -1e-6) {
            return Some(-1.);
        }
        d.into_iter().map(f64::abs).reduce(f64::min)
    };
    match (lado(a, b), lado(b, a)) {
        (Some(x), Some(y)) => Some(x.min(y)),
        (x, y) => x.or(y),
    }
}

fn medir(doc: &Document, m: Medida) -> Result<f64, String> {
    if m == Medida::Massa {
        return Ok(massa(doc).0);
    }
    let s = final_shape(doc).filter(|s| !s.is_null()).ok_or("a peça não tem corpo")?;
    Ok(match m {
        Medida::Volume => s.mass().volume,
        Medida::CaixaX | Medida::CaixaY | Medida::CaixaZ => {
            let c = caixa(&s).ok_or("caixa vazia")?;
            c[match m {
                Medida::CaixaX => 0,
                Medida::CaixaY => 1,
                _ => 2,
            }]
        }
        Medida::Massa => unreachable!(),
    })
}

fn num(x: f64) -> String {
    let s = format!("{x:.3}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" { "0".into() } else { s.into() }
}

/// Confere `requisitos` na peça `doc`, na ordem dada.
pub fn verificar(doc: &Document, requisitos: &[Requisito]) -> Vec<Resultado> {
    let forma = std::cell::OnceCell::new();
    let forma = || forma.get_or_init(|| final_shape(doc).filter(|s| !s.is_null()));
    let lista_corpos = std::cell::OnceCell::new();
    let lista_corpos = || lista_corpos.get_or_init(|| corpos(doc));
    requisitos
        .iter()
        .map(|r| {
            let (ok, medido, detalhe) = match &r.tipo {
                TipoRequisito::Massa { min, max } => {
                    let (m, tem) = massa(doc);
                    if !tem {
                        (false, "—".into(), "a peça não tem corpo".into())
                    } else {
                        let ok = min.is_none_or(|a| m >= a) && max.is_none_or(|b| m <= b);
                        let faixa = format!("{} a {} kg", min.map_or("—".into(), num), max.map_or("—".into(), num));
                        (ok, format!("{} kg", num(m)), if ok { format!("dentro de {faixa}") } else { format!("fora de {faixa}") })
                    }
                }
                TipoRequisito::Caixa { max } => match forma().as_ref().and_then(caixa) {
                    None => (false, "—".into(), "a peça não tem corpo".into()),
                    Some(c) => {
                        let fora: Vec<&str> = ["X", "Y", "Z"].into_iter().zip(c.iter().zip(max)).filter(|(_, (v, m))| **v > **m + 1e-6).map(|(e, _)| e).collect();
                        let medido = format!("{} × {} × {} mm", num(c[0]), num(c[1]), num(c[2]));
                        (fora.is_empty(), medido, if fora.is_empty() { "cabe na caixa".into() } else { format!("passa da caixa em {}", fora.join(", ")) })
                    }
                },
                TipoRequisito::Furos { diametro, quantidade, tol } => match forma() {
                    None => (false, "—".into(), "a peça não tem corpo".into()),
                    Some(s) => {
                        let rec = reconhece::analisar(s);
                        let n = rec
                            .encontradas
                            .iter()
                            .filter(|e| matches!(e.tipo, reconhece::Tipo::FuroPassante | reconhece::Tipo::FuroCego) && (e.medida - diametro).abs() <= *tol)
                            .count();
                        let ok = n as u32 >= *quantidade;
                        (ok, format!("{n} furo(s) Ø{}", num(*diametro)), if ok { format!("pede {quantidade}") } else { format!("pede {quantidade}, faltam {}", *quantidade as usize - n) })
                    }
                },
                TipoRequisito::SemInterferencia { tol } => {
                    let c = lista_corpos();
                    let mut pior: Option<(f64, String)> = None;
                    for i in 0..c.len() {
                        for j in i + 1..c.len() {
                            let v = c[i].2.boolean(&c[j].2, Bool::Common).map(|s| if s.is_null() { 0. } else { s.mass().volume.abs() }).unwrap_or(0.);
                            if pior.as_ref().is_none_or(|(p, _)| v > *p) {
                                pior = Some((v, format!("{} e {}", c[i].1, c[j].1)));
                            }
                        }
                    }
                    match pior {
                        None => (true, "—".into(), "um corpo só: nada a conferir".into()),
                        Some((v, par)) => (v <= *tol, format!("{} mm³", num(v)), if v <= *tol { "nenhum par se sobrepõe".into() } else { format!("{par} se sobrepõem") }),
                    }
                }
                TipoRequisito::Folga { min } => {
                    let c = lista_corpos();
                    let mut pior: Option<(f64, String)> = None;
                    for i in 0..c.len() {
                        for j in i + 1..c.len() {
                            if let Some(d) = folga(&c[i].2, &c[j].2) {
                                if pior.as_ref().is_none_or(|(p, _)| d < *p) {
                                    pior = Some((d, format!("{} e {}", c[i].1, c[j].1)));
                                }
                            }
                        }
                    }
                    match pior {
                        None => (true, "—".into(), "um corpo só: nada a conferir".into()),
                        Some((d, par)) if d < 0. => (false, "0 mm".into(), format!("{par} se tocam ou se cruzam")),
                        Some((d, par)) => (d + 1e-6 >= *min, format!("{} mm", num(d)), format!("menor folga: {par}")),
                    }
                }
                TipoRequisito::Propagacao { parametro, valor, medida, esperado, tol } => propagacao(doc, parametro, valor, *medida, *esperado, *tol),
            };
            Resultado { nome: r.nome.clone(), tipo: r.tipo.rotulo(), ok, medido, detalhe }
        })
        .collect()
}

fn propagacao(doc: &Document, parametro: &str, valor: &str, medida: Medida, esperado: Option<f64>, tol: f64) -> (bool, String, String) {
    let Some(i) = doc.params.iter().position(|p| p.name == parametro) else {
        return (false, "—".into(), format!("o parâmetro '{parametro}' não existe"));
    };
    let antes = match medir(doc, medida) {
        Ok(v) => v,
        Err(e) => return (false, "—".into(), e),
    };
    let mut copia = doc.clone();
    copia.params[i].expr = valor.to_string();
    let r = recompute(&copia);
    if let Some(e) = r.param_error {
        return (false, "—".into(), format!("o parâmetro não aceitou '{valor}': {e}"));
    }
    let falhas: Vec<String> = copia.features.iter().filter(|f| matches!(r.status.get(&f.id), Some(Status::Error(_)))).map(|f| f.name.clone()).collect();
    if !falhas.is_empty() {
        return (false, "—".into(), format!("com {parametro} = {valor} falham: {}", falhas.join(", ")));
    }
    let depois = match medir(&copia, medida) {
        Ok(v) => v,
        Err(e) => return (false, "—".into(), e),
    };
    let medido = format!("{} → {}", num(antes), num(depois));
    match esperado {
        Some(x) => {
            let ok = (depois - x).abs() <= tol;
            (ok, medido, if ok { format!("{} seguiu o parâmetro", medida.rotulo()) } else { format!("{} devia ir a {}", medida.rotulo(), num(x)) })
        }
        None => {
            let ok = (depois - antes).abs() > tol;
            (ok, medido, if ok { format!("{} mudou com o parâmetro", medida.rotulo()) } else { format!("{} não mudou: o parâmetro não chega na forma", medida.rotulo()) })
        }
    }
}
