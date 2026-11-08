// SPDX-License-Identifier: LGPL-2.1-or-later
//! Checador de similaridade: compara o que foi montado com o alvo e diz onde diverge.
//!
//! Sem ele um agente não tem como saber que errou. O relatório é feito para orientar a próxima
//! tentativa: além de aprovado ou não, diz qual medida saiu, quais furos faltam ou sobram e em que
//! ponto do espaço as superfícies mais se afastam. Os dois corpos valem no mesmo sistema de
//! coordenadas: não há alinhamento automático, porque ele esconderia uma peça fora do lugar.

use crate::topologia::{dist, dist_reta, Furo, Resumo};
use crate::{r6, r6v, ErroApi};
use serde_json::{json, Value};

/// Tolerâncias do checador. Relativas ao alvo, salvo `furo_*`, em mm. `distancia` e `caixa` são
/// frações da diagonal do alvo.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tolerancias {
    pub volume: f64,
    pub area: f64,
    pub centroide: f64,
    pub caixa: f64,
    pub inercia: f64,
    pub distancia: f64,
    pub furo_diametro: f64,
    pub furo_posicao: f64,
}

impl Default for Tolerancias {
    fn default() -> Self {
        Tolerancias {
            volume: 0.005,
            area: 0.01,
            centroide: 0.005,
            caixa: 0.005,
            inercia: 0.01,
            distancia: 0.01,
            furo_diametro: 0.01,
            furo_posicao: 0.05,
        }
    }
}

impl Tolerancias {
    /// Tolerâncias do JSON; o que não vier fica no padrão, e campo desconhecido é erro.
    pub fn de_json(v: &Value) -> Result<Tolerancias, ErroApi> {
        let mut t = Tolerancias::default();
        let Some(o) = v.as_object() else {
            return if v.is_null() {
                Ok(t)
            } else {
                Err(ErroApi::argumento("`tolerancias` deve ser um objeto"))
            };
        };
        for (k, x) in o {
            let n = x
                .as_f64()
                .filter(|n| *n >= 0. && n.is_finite())
                .ok_or_else(|| {
                    ErroApi::argumento(format!("tolerância '{k}' deve ser um número positivo"))
                })?;
            match k.as_str() {
                "volume" => t.volume = n,
                "area" => t.area = n,
                "centroide" => t.centroide = n,
                "caixa" => t.caixa = n,
                "inercia" => t.inercia = n,
                "distancia" => t.distancia = n,
                "furo_diametro" => t.furo_diametro = n,
                "furo_posicao" => t.furo_posicao = n,
                _ => {
                    return Err(ErroApi::argumento(format!(
                        "tolerância desconhecida: '{k}'"
                    )))
                }
            }
        }
        Ok(t)
    }
}

/// Teto de contas ponto-triângulo por sentido: peça grande perde amostras, não trava o servidor.
const ORCAMENTO: usize = 150_000_000;
const AMOSTRAS: usize = 3000;

type Tri = [[f64; 3]; 3];

fn triangulos(r: &Resumo) -> Vec<Tri> {
    let p = &r.malha.positions;
    let v = |i: u32| {
        let i = i as usize * 3;
        [p[i] as f64, p[i + 1] as f64, p[i + 2] as f64]
    };
    r.malha
        .indices
        .chunks_exact(3)
        .map(|t| [v(t[0]), v(t[1]), v(t[2])])
        .collect()
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    fenkra_kernel::cross(a, b)
}

/// Ponto do triângulo mais perto de `p` (regiões de Voronoi do triângulo).
fn mais_perto(p: [f64; 3], t: &Tri) -> [f64; 3] {
    let [a, b, c] = *t;
    let (ab, ac, ap) = (sub(b, a), sub(c, a), sub(p, a));
    let em = |o: [f64; 3], d: [f64; 3], k: f64| [o[0] + k * d[0], o[1] + k * d[1], o[2] + k * d[2]];
    let (d1, d2) = (dot(ab, ap), dot(ac, ap));
    if d1 <= 0. && d2 <= 0. {
        return a;
    }
    let bp = sub(p, b);
    let (d3, d4) = (dot(ab, bp), dot(ac, bp));
    if d3 >= 0. && d4 <= d3 {
        return b;
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0. && d1 >= 0. && d3 <= 0. {
        return em(a, ab, d1 / (d1 - d3));
    }
    let cp = sub(p, c);
    let (d5, d6) = (dot(ab, cp), dot(ac, cp));
    if d6 >= 0. && d5 <= d6 {
        return c;
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0. && d2 >= 0. && d6 <= 0. {
        return em(a, ac, d2 / (d2 - d6));
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0. && d4 - d3 >= 0. && d5 - d6 >= 0. {
        return em(b, sub(c, b), (d4 - d3) / ((d4 - d3) + (d5 - d6)));
    }
    let k = 1. / (va + vb + vc);
    let q = em(a, ab, vb * k);
    em(q, ac, vc * k)
}

/// Pontos sobre a malha, proporcionais à área, sempre os mesmos para a mesma malha.
fn amostrar(tris: &[Tri], n: usize) -> Vec<[f64; 3]> {
    let areas: Vec<f64> = tris
        .iter()
        .map(|t| {
            let c = cross(sub(t[1], t[0]), sub(t[2], t[0]));
            dot(c, c).sqrt() / 2.
        })
        .collect();
    let total: f64 = areas.iter().sum();
    if total <= 0. {
        return vec![];
    }
    let mut semente: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut sorteio = move || {
        semente = semente
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (semente >> 11) as f64 / (1u64 << 53) as f64
    };
    let mut saida = Vec::with_capacity(n + tris.len());
    let mut resto = 0.;
    for (t, a) in tris.iter().zip(&areas) {
        resto += a / total * n as f64;
        while resto >= 1. {
            resto -= 1.;
            let (mut u, mut v) = (sorteio(), sorteio());
            if u + v > 1. {
                (u, v) = (1. - u, 1. - v);
            }
            saida.push(
                [0, 1, 2].map(|i| t[0][i] + u * (t[1][i] - t[0][i]) + v * (t[2][i] - t[0][i])),
            );
        }
    }
    saida
}

struct Afastamento {
    maximo: f64,
    media: f64,
    onde: [f64; 3],
}

/// Maior distância dos pontos de `de` até a superfície `ate`. Cada triângulo guarda uma esfera
/// envolvente: quem está mais longe que a melhor resposta nem entra na conta.
fn afastamento(de: &[[f64; 3]], ate: &[Tri]) -> Afastamento {
    let esferas: Vec<([f64; 3], f64)> = ate
        .iter()
        .map(|t| {
            let c = [0, 1, 2].map(|i| (t[0][i] + t[1][i] + t[2][i]) / 3.);
            (c, t.iter().map(|v| dist(c, *v)).fold(0., f64::max))
        })
        .collect();
    let (mut maximo, mut soma, mut onde) = (0., 0., [0.; 3]);
    let mut ultimo = 0;
    for p in de {
        let mut melhor = ate
            .get(ultimo)
            .map_or(f64::MAX, |t| dist(*p, mais_perto(*p, t)));
        for (i, (t, (c, r))) in ate.iter().zip(&esferas).enumerate() {
            if dist(*p, *c) - r < melhor {
                let d = dist(*p, mais_perto(*p, t));
                if d < melhor {
                    melhor = d;
                    ultimo = i;
                }
            }
        }
        soma += melhor;
        if melhor > maximo {
            maximo = melhor;
            onde = *p;
        }
    }
    Afastamento {
        maximo,
        media: soma / de.len().max(1) as f64,
        onde,
    }
}

/// Mesmo furo: mesmo diâmetro e eixos juntos, comparando os dois pontos do eixo no meio do furo.
fn mesmo_lugar(a: &Furo, b: &Furo, tol: f64) -> bool {
    let meio = [0, 1, 2].map(|i| (a.inicio[i] + a.fim[i]) / 2.);
    let paralelo = dot(a.eixo, b.eixo).abs() > 1. - 1e-6;
    paralelo && dist_reta(meio, b.inicio, b.eixo) <= tol
}

fn furo_json(f: &Furo) -> Value {
    json!({ "diametro": r6(f.diametro), "inicio": r6v(f.inicio), "fim": r6v(f.fim) })
}

/// Casa furos do alvo e do modelo. Ordem de preferência: mesmo diâmetro e lugar; depois diâmetro certo
/// em lugar errado (`deslocados`) e lugar certo com diâmetro errado (`diametro_errado`).
fn comparar_furos(alvo: &Resumo, modelo: &Resumo, t: &Tolerancias) -> Value {
    let mut livres: Vec<&Furo> = modelo.furos.iter().collect();
    let igual_d = |a: &Furo, b: &Furo| (a.diametro - b.diametro).abs() <= t.furo_diametro;
    let tirar = |l: &mut Vec<&'_ Furo>, teste: &dyn Fn(&Furo) -> bool| {
        l.iter().position(|f| teste(f)).map(|i| l.remove(i).clone())
    };
    let mut pendentes = vec![];
    for a in &alvo.furos {
        if tirar(&mut livres, &|f| {
            igual_d(a, f) && mesmo_lugar(a, f, t.furo_posicao)
        })
        .is_none()
        {
            pendentes.push(a);
        }
    }
    let (mut errados, mut deslocados, mut faltam) = (vec![], vec![], vec![]);
    for a in pendentes {
        if let Some(f) = tirar(&mut livres, &|f| mesmo_lugar(a, f, t.furo_posicao)) {
            errados.push(json!({ "alvo": furo_json(a), "modelo": furo_json(&f) }));
        } else if let Some(f) = tirar(&mut livres, &|f| igual_d(a, f)) {
            let meio = [0, 1, 2].map(|i| (a.inicio[i] + a.fim[i]) / 2.);
            deslocados.push(json!({ "alvo": furo_json(a), "modelo": furo_json(&f), "desvio": r6(dist_reta(meio, f.inicio, f.eixo)) }));
        } else {
            faltam.push(furo_json(a));
        }
    }
    let sobram: Vec<Value> = livres.iter().map(|f| furo_json(f)).collect();
    let ok = errados.is_empty() && deslocados.is_empty() && faltam.is_empty() && sobram.is_empty();
    json!({ "ok": ok, "alvo": alvo.furos.len(), "modelo": modelo.furos.len(), "por_diametro": por_diametro(alvo, modelo, t.furo_diametro), "faltam": faltam, "sobram": sobram, "deslocados": deslocados, "diametro_errado": errados })
}

/// Contagem de furos por diâmetro nos dois corpos (B237): diâmetros dentro de `tol` contam juntos.
/// Cada linha diz quantos furos daquele diâmetro o alvo e o modelo têm, do menor para o maior.
fn por_diametro(alvo: &Resumo, modelo: &Resumo, tol: f64) -> Vec<Value> {
    let mut grupos: Vec<(f64, usize, usize)> = vec![];
    let mut contar = |d: f64, qual: usize| {
        let i = match grupos.iter().position(|g| (g.0 - d).abs() <= tol) {
            Some(i) => i,
            None => {
                grupos.push((d, 0, 0));
                grupos.len() - 1
            }
        };
        if qual == 0 {
            grupos[i].1 += 1;
        } else {
            grupos[i].2 += 1;
        }
    };
    alvo.furos.iter().for_each(|f| contar(f.diametro, 0));
    modelo.furos.iter().for_each(|f| contar(f.diametro, 1));
    grupos.sort_by(|a, b| a.0.total_cmp(&b.0));
    grupos
        .into_iter()
        .map(|(d, a, m)| json!({ "diametro": r6(d), "alvo": a, "modelo": m, "ok": a == m }))
        .collect()
}

fn medida(nome: &str, alvo: f64, modelo: f64, desvio: f64, tolerancia: f64) -> Value {
    json!({ "nome": nome, "alvo": r6(alvo), "modelo": r6(modelo), "desvio": (desvio * 1e9).round() / 1e9, "tolerancia": tolerancia, "ok": desvio <= tolerancia })
}

fn relativo(a: f64, b: f64) -> f64 {
    (a - b).abs() / a.abs().max(1e-12)
}

/// Compara `modelo` com `alvo`. `aprovado` só sai verdadeiro com todas as medidas e os furos dentro.
pub fn comparar(alvo: &Resumo, modelo: &Resumo, t: &Tolerancias) -> Value {
    let diagonal = dist(alvo.caixa.0, alvo.caixa.1).max(1e-9);
    let mut medidas = vec![
        medida(
            "volume",
            alvo.volume,
            modelo.volume,
            relativo(alvo.volume, modelo.volume),
            t.volume,
        ),
        medida(
            "area",
            alvo.area,
            modelo.area,
            relativo(alvo.area, modelo.area),
            t.area,
        ),
    ];
    let desvio_c = dist(alvo.centroide, modelo.centroide);
    medidas.push(json!({ "nome": "centroide", "alvo": r6v(alvo.centroide), "modelo": r6v(modelo.centroide), "desvio": (desvio_c / diagonal * 1e9).round() / 1e9, "tolerancia": t.centroide, "ok": desvio_c / diagonal <= t.centroide }));
    let desvio_x = dist(alvo.caixa.0, modelo.caixa.0).max(dist(alvo.caixa.1, modelo.caixa.1));
    medidas.push(json!({ "nome": "caixa", "alvo": [r6v(alvo.caixa.0), r6v(alvo.caixa.1)], "modelo": [r6v(modelo.caixa.0), r6v(modelo.caixa.1)], "desvio": (desvio_x / diagonal * 1e9).round() / 1e9, "tolerancia": t.caixa, "ok": desvio_x / diagonal <= t.caixa }));
    let desvio_i = (0..3)
        .map(|i| relativo(alvo.inercia[i], modelo.inercia[i]))
        .fold(0., f64::max);
    medidas.push(json!({ "nome": "inercia", "alvo": r6v(alvo.inercia), "modelo": r6v(modelo.inercia), "desvio": (desvio_i * 1e9).round() / 1e9, "tolerancia": t.inercia, "ok": desvio_i <= t.inercia }));

    let (ta, tm) = (triangulos(alvo), triangulos(modelo));
    let n = AMOSTRAS
        .min(ORCAMENTO / ta.len().max(tm.len()).max(1))
        .max(100);
    let (ida, volta) = (
        afastamento(&amostrar(&ta, n), &tm),
        afastamento(&amostrar(&tm, n), &ta),
    );
    let pior = if ida.maximo >= volta.maximo {
        &ida
    } else {
        &volta
    };
    let distancia = json!({ "maxima": r6(pior.maximo), "media": r6((ida.media + volta.media) / 2.), "onde": r6v(pior.onde), "fracao": (pior.maximo / diagonal * 1e9).round() / 1e9, "tolerancia": t.distancia, "ok": pior.maximo / diagonal <= t.distancia });

    let furos = comparar_furos(alvo, modelo, t);
    let aprovado =
        medidas.iter().all(|m| m["ok"] == true) && furos["ok"] == true && distancia["ok"] == true;
    json!({ "aprovado": aprovado, "medidas": medidas, "furos": furos, "distancia": distancia })
}
