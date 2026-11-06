// SPDX-License-Identifier: LGPL-2.1-or-later
//! Análise de montagem que vai no manifesto (B339): interferências e folgas **já calculadas**.
//!
//! O visualizador não tem kernel, então não pode medir sólidos. Quem calcula é o Fenkra, na hora de
//! gerar o pacote, e o resultado vai em `analise` para a página só listar, filtrar e destacar.
//!
//! Como funciona:
//! 1. interferências: as mesmas do comando de interferência da montagem (`interferencia::analisar`,
//!    volume comum pela operação booleana, caixas envolventes como filtro);
//! 2. folgas: para cada par de peças que **não** interfere e cujas caixas estão a menos de
//!    [`LIMITE_FOLGA_MM`] uma da outra, a menor distância com sinal entre uma amostra de vértices de
//!    cada peça e o sólido da outra. É um limite superior da folga real: exato quando as peças se
//!    olham por faces planas ou cilindros, e pode passar um pouco da real entre arestas. O campo
//!    `metodo` avisa isso ao leitor;
//! 3. montagem grande não é analisada ([`MAX_PECAS`]): o par cresce com o quadrado do número de
//!    peças, e o pacote diz que não calculou em vez de travar quem salva.
//!
//! Os IDs são os da árvore do pacote (`n<id>` do componente de topo).

use fenkra_kernel::Shape;
use fenkra_montagem::interferencia::{analisar, OpcoesInterf};
use fenkra_montagem::load::PartCache;
use fenkra_montagem::view::placed_shape;
use fenkra_montagem::{Assembly, LoadMode, NodeKind};
use serde_json::{json, Value};
use std::path::Path;

/// Acima disso a análise não roda.
pub const MAX_PECAS: usize = 150;
/// Pares mais afastados que isso não entram na lista de folgas.
pub const LIMITE_FOLGA_MM: f64 = 25.;
/// Vértices por peça usados para medir a folga.
const AMOSTRA: usize = 256;
/// Volume comum mínimo (mm³) para contar como interferência.
const TOL_MM3: f64 = 1e-3;

fn r6(x: f64) -> f64 {
    let v = (x * 1e6).round() / 1e6;
    if v == 0. {
        0.
    } else {
        v
    }
}

/// ID da árvore do pacote para a chave de um achado: só componentes de topo têm id curto.
fn id_do_lado(chave: &str) -> Option<String> {
    (!chave.contains('/')).then(|| format!("n{chave}"))
}

fn distancia_caixas(a: &([f64; 3], [f64; 3]), b: &([f64; 3], [f64; 3])) -> f64 {
    let mut q = 0.;
    for k in 0..3 {
        let d = (a.0[k] - b.1[k]).max(b.0[k] - a.1[k]).max(0.);
        q += d * d;
    }
    q.sqrt()
}

/// Amostra espalhada de vértices da forma (a malha grossa basta: a distância é medida no sólido).
fn amostra(s: &Shape) -> Vec<[f64; 3]> {
    let Ok(m) = s.mesh(1.0) else { return vec![] };
    let n = m.positions.len() / 3;
    if n == 0 {
        return vec![];
    }
    let passo = n.div_ceil(AMOSTRA).max(1);
    (0..n).step_by(passo).map(|i| [f64::from(m.positions[i * 3]), f64::from(m.positions[i * 3 + 1]), f64::from(m.positions[i * 3 + 2])]).collect()
}

/// Menor distância (fora do sólido) dos pontos até a forma; `None` se algum ponto cai dentro dela
/// ou se o kernel não responde.
fn menor_distancia(forma: &Shape, pontos: &[[f64; 3]]) -> Option<f64> {
    let d = forma.signed_distances(pontos).ok()?;
    d.into_iter().filter(|x| x.is_finite()).map(f64::abs).reduce(f64::min)
}

/// `analise` do manifesto da montagem. Nunca falha: o que não deu para calcular vai em `avisos`.
pub fn analisar_montagem(asm: &Assembly, pasta: &Path) -> Value {
    let pecas: Vec<_> = asm.nodes.iter().filter(|n| matches!(n.kind, NodeKind::Part { .. } | NodeKind::SubAssembly { .. }) && n.mode == LoadMode::Editavel && n.visible).collect();
    if pecas.len() > MAX_PECAS {
        return json!({"calculada": false, "motivo": format!("montagem com {} componentes de topo; a análise cobre até {MAX_PECAS}", pecas.len()), "interferencias": [], "folgas": []});
    }
    if !fenkra_kernel::AVAILABLE {
        return json!({"calculada": false, "motivo": "sem kernel", "interferencias": [], "folgas": []});
    }
    let mut cache = PartCache::default();
    let achados = analisar(asm, Some(pasta), &mut cache, &OpcoesInterf { tol: TOL_MM3, ..Default::default() });
    let mut interf = vec![];
    let mut com_interferencia = std::collections::HashSet::new();
    for x in &achados {
        let (Some(a), Some(b)) = (id_do_lado(&x.a.chave), id_do_lado(&x.b.chave)) else { continue };
        com_interferencia.insert((a.clone(), b.clone()));
        com_interferencia.insert((b.clone(), a.clone()));
        interf.push(json!({"a": a, "b": b, "nome_a": x.a.nome, "nome_b": x.b.nome, "volume_mm3": r6(x.volume)}));
    }

    // folgas entre componentes de peça de topo
    let formas: Vec<(String, String, Shape)> = pecas
        .iter()
        .filter(|n| matches!(n.kind, NodeKind::Part { .. }))
        .filter_map(|n| placed_shape(asm, n.id, Some(pasta), &mut cache).map(|s| (format!("n{}", n.id.0), n.name.clone(), s)))
        .collect();
    let caixas: Vec<_> = formas.iter().map(|f| f.2.bbox()).collect();
    let mut folgas = vec![];
    for i in 0..formas.len() {
        for j in i + 1..formas.len() {
            let (Some(ca), Some(cb)) = (&caixas[i], &caixas[j]) else { continue };
            if com_interferencia.contains(&(formas[i].0.clone(), formas[j].0.clone())) || distancia_caixas(ca, cb) > LIMITE_FOLGA_MM {
                continue;
            }
            let (pa, pb) = (amostra(&formas[i].2), amostra(&formas[j].2));
            let d = [menor_distancia(&formas[i].2, &pb), menor_distancia(&formas[j].2, &pa)].into_iter().flatten().reduce(f64::min);
            if let Some(d) = d.filter(|d| *d <= LIMITE_FOLGA_MM) {
                folgas.push(json!({"a": formas[i].0, "b": formas[j].0, "nome_a": formas[i].1, "nome_b": formas[j].1, "distancia_mm": r6(d), "metodo": "vertices"}));
            }
        }
    }
    folgas.sort_by(|x, y| x["distancia_mm"].as_f64().partial_cmp(&y["distancia_mm"].as_f64()).unwrap_or(std::cmp::Ordering::Equal));
    json!({"calculada": true, "tolerancia_mm3": TOL_MM3, "limite_folga_mm": LIMITE_FOLGA_MM, "interferencias": interf, "folgas": folgas})
}
