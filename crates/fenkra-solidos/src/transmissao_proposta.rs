// SPDX-License-Identifier: LGPL-2.1-or-later
//! Adaptação de projeto (B340): a pessoa fixa o que limita (distância entre dois pontos, razão,
//! espaço, módulo) e o comando **propõe o que cabe**, da melhor para a pior proposta.
//!
//! - **Engrenagens retas**: módulo da lista de uso comum e dentes inteiros; a distância sai de
//!   `a = m (z1 + z2) / 2`, e a razão de `z2 / z1`.
//! - **Helicoidais**: com o módulo normal fixo, o ângulo de hélice fecha a distância **exata**:
//!   `cos β = m (z1 + z2) / (2 a)`, entre 8° e 30°.
//! - **Corrente**: passo da biblioteca (só linhas conferidas), dentes do pinhão de 11 a 40, elos pares
//!   e distância entre centros corrigida (a corrente não se estica: a distância final é a que fecha).
//! - **Correia dentada**: passo da biblioteca, polias com dentes inteiros e correia de dentes inteiros.
//!
//! Cada proposta traz o erro de distância e de razão contra o pedido e o maior diâmetro externo
//! (para o filtro de espaço). Nada é gravado: a interface aplica a proposta escolhida.

use crate::engrenagens::{calculo, ParamEngrenagem, TipoEngrenagem};
use crate::engrenagens_dim::MODULOS_USUAIS;
use crate::transmissao_dados::{self as dados, FamiliaCorreia};

/// O que a pessoa fixou. Campos `None` ficam livres para a proposta escolher.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Restricoes {
    /// distância entre os dois eixos (por exemplo, entre dois pontos escolhidos no 3D)
    pub distancia: Option<f64>,
    /// razão z2 / z1 (maior que 1 reduz)
    pub razao: Option<f64>,
    /// módulo (engrenagens) fixo
    pub modulo: Option<f64>,
    /// diâmetro externo máximo de qualquer roda (espaço disponível)
    pub espaco: Option<f64>,
    pub angulo_pressao: f64,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Familia {
    Retas,
    Helicoidais,
    Corrente,
    CorreiaDentada,
}

impl Familia {
    pub const ALL: [Familia; 4] = [Familia::Retas, Familia::Helicoidais, Familia::Corrente, Familia::CorreiaDentada];

    pub fn rotulo(self) -> &'static str {
        match self {
            Familia::Retas => "Engrenagens retas",
            Familia::Helicoidais => "Engrenagens helicoidais",
            Familia::Corrente => "Corrente",
            Familia::CorreiaDentada => "Correia dentada",
        }
    }
}

/// Uma proposta que cabe nos limites.
#[derive(Clone, Debug, PartialEq)]
pub struct Proposta {
    pub familia: Familia,
    /// módulo (engrenagens) ou passo (corrente e correia), em mm
    pub modulo_ou_passo: f64,
    /// designação da corrente ou da correia da biblioteca; vazio nas engrenagens
    pub designacao: String,
    pub n1: u32,
    pub n2: u32,
    /// ângulo de hélice (só helicoidais)
    pub angulo_helice: f64,
    /// distância entre centros resultante
    pub distancia: f64,
    pub razao: f64,
    /// `distancia − pedida` (0 sem pedido) e `razao − pedida`
    pub erro_distancia: f64,
    pub erro_razao: f64,
    /// maior diâmetro externo entre as duas rodas
    pub diametro_maximo: f64,
    /// elos (corrente) ou dentes da correia
    pub elos: u32,
}

impl Proposta {
    pub fn descricao(&self) -> String {
        match self.familia {
            Familia::Retas => format!("m {} · z1 {} · z2 {}", self.modulo_ou_passo, self.n1, self.n2),
            Familia::Helicoidais => format!("m {} · z1 {} · z2 {} · β {:.2}°", self.modulo_ou_passo, self.n1, self.n2, self.angulo_helice),
            Familia::Corrente => format!("{} · z1 {} · z2 {} · {} elos", self.designacao, self.n1, self.n2, self.elos),
            Familia::CorreiaDentada => format!("{} · z1 {} · z2 {} · {} dentes", self.designacao, self.n1, self.n2, self.elos),
        }
    }
}

fn nota(p: &Proposta, r: &Restricoes) -> f64 {
    let ed = r.distancia.map_or(0., |a| p.erro_distancia.abs() / a);
    let er = r.razao.map_or(0., |i| p.erro_razao.abs() / i);
    // sem pedido de razão nem de distância, prefere o menor conjunto que cabe
    ed + er + 1e-6 * p.diametro_maximo
}

/// Até `max` propostas da família, da melhor para a pior. Vazio se o pedido não tem solução (ou
/// faltam dados: sem distância e sem razão não há o que adaptar).
pub fn propor(f: Familia, r: &Restricoes, max: usize) -> Vec<Proposta> {
    if r.distancia.is_none() && r.razao.is_none() {
        return vec![];
    }
    if r.distancia.is_some_and(|a| !(a.is_finite() && a > 0.)) || r.razao.is_some_and(|i| !(i.is_finite() && i >= 1.)) || r.espaco.is_some_and(|e| !(e.is_finite() && e > 0.)) {
        return vec![];
    }
    let mut v = match f {
        Familia::Retas => propor_retas(r),
        Familia::Helicoidais => propor_helicoidais(r),
        Familia::Corrente => propor_corrente(r),
        Familia::CorreiaDentada => propor_correia(r),
    };
    v.retain(|p| r.espaco.is_none_or(|e| p.diametro_maximo <= e + 1e-9));
    v.sort_by(|a, b| nota(a, r).total_cmp(&nota(b, r)).then(b.modulo_ou_passo.total_cmp(&a.modulo_ou_passo)));
    v.truncate(max);
    v
}

fn modulos(r: &Restricoes) -> Vec<f64> {
    r.modulo.map_or_else(|| MODULOS_USUAIS.to_vec(), |m| vec![m])
}

fn diametro_topo(m: f64, z: u32, beta: f64, alfa: f64, tipo: TipoEngrenagem) -> Option<f64> {
    let p = ParamEngrenagem { tipo, modulo: m, dentes: z, angulo_pressao: alfa, angulo_helice: beta, ..Default::default() };
    calculo(&p).ok().map(|c| c.diametro_externo)
}

fn propor_retas(r: &Restricoes) -> Vec<Proposta> {
    let alfa = if r.angulo_pressao > 0. { r.angulo_pressao } else { 20. };
    let mut out = vec![];
    for m in modulos(r) {
        for z1 in 12..=200u32 {
            // z2: da razão, ou da distância (z1 + z2 = 2a / m)
            let candidatos: Vec<u32> = match (r.razao, r.distancia) {
                (Some(i), _) => vec![(i * z1 as f64).floor() as u32, (i * z1 as f64).ceil() as u32],
                (None, Some(a)) => {
                    let soma = 2. * a / m;
                    vec![(soma - z1 as f64).floor() as u32, (soma - z1 as f64).ceil() as u32]
                }
                _ => vec![],
            };
            for z2 in candidatos {
                if z2 < z1 || z2 > 400 {
                    continue;
                }
                let (Some(d1), Some(d2)) = (diametro_topo(m, z1, 0., alfa, TipoEngrenagem::Reta), diametro_topo(m, z2, 0., alfa, TipoEngrenagem::Reta)) else { continue };
                let a = m * (z1 + z2) as f64 / 2.;
                let rz = z2 as f64 / z1 as f64;
                let p = Proposta {
                    familia: Familia::Retas,
                    modulo_ou_passo: m,
                    designacao: String::new(),
                    n1: z1,
                    n2: z2,
                    angulo_helice: 0.,
                    distancia: a,
                    razao: rz,
                    erro_distancia: r.distancia.map_or(0., |p| a - p),
                    erro_razao: r.razao.map_or(0., |p| rz - p),
                    diametro_maximo: d1.max(d2),
                    elos: 0,
                };
                if !out.iter().any(|q: &Proposta| q.modulo_ou_passo == m && q.n1 == z1 && q.n2 == z2) {
                    out.push(p);
                }
            }
        }
    }
    out
}

fn propor_helicoidais(r: &Restricoes) -> Vec<Proposta> {
    let Some(a) = r.distancia else { return vec![] };
    let alfa = if r.angulo_pressao > 0. { r.angulo_pressao } else { 20. };
    let mut out = vec![];
    for m in modulos(r) {
        for z1 in 12..=120u32 {
            let candidatos: Vec<u32> = match r.razao {
                Some(i) => vec![(i * z1 as f64).floor() as u32, (i * z1 as f64).ceil() as u32],
                None => (z1..=(4 * z1).min(250)).collect(),
            };
            for z2 in candidatos {
                if z2 < z1 || z2 > 250 {
                    continue;
                }
                let cb = m * (z1 + z2) as f64 / (2. * a);
                if !(0.0..=1.).contains(&cb) {
                    continue;
                }
                let beta = cb.acos().to_degrees();
                if !(8.0..=30.).contains(&beta) {
                    continue;
                }
                let (Some(d1), Some(d2)) = (diametro_topo(m, z1, beta, alfa, TipoEngrenagem::Helicoidal), diametro_topo(m, z2, beta, alfa, TipoEngrenagem::Helicoidal)) else { continue };
                let rz = z2 as f64 / z1 as f64;
                out.push(Proposta {
                    familia: Familia::Helicoidais,
                    modulo_ou_passo: m,
                    designacao: String::new(),
                    n1: z1,
                    n2: z2,
                    angulo_helice: beta,
                    distancia: a,
                    razao: rz,
                    erro_distancia: 0.,
                    erro_razao: r.razao.map_or(0., |p| rz - p),
                    diametro_maximo: d1.max(d2),
                    elos: 0,
                });
            }
        }
    }
    out
}

fn propor_corrente(r: &Restricoes) -> Vec<Proposta> {
    let mut out = vec![];
    for c in dados::CORRENTES.iter().filter(|c| c.liberada()) {
        for n1 in 11..=40u32 {
            let candidatos: Vec<u32> = match r.razao {
                Some(i) => vec![(i * n1 as f64).floor() as u32, (i * n1 as f64).ceil() as u32],
                None => (n1..=(4 * n1).min(100)).collect(),
            };
            for n2 in candidatos {
                if n2 < n1 || n2 > 100 {
                    continue;
                }
                let pedido = match r.distancia {
                    Some(a) => a,
                    None => 40. * c.passo,
                };
                let elos = dados::elos_da_corrente(c.passo, n1, n2, pedido);
                let Some(cc) = dados::distancia_corrigida_corrente(c.passo, n1, n2, elos) else { continue };
                // a corrente precisa abraçar a roda menor em pelo menos 120°: centros ≥ ~ (d2 − d1) / 2 · 1,2 já garante
                let (d1, d2) = (dados::diametro_primitivo_roda(c.passo, n1), dados::diametro_primitivo_roda(c.passo, n2));
                if cc < (d1 + d2) / 2. + c.passo {
                    continue;
                }
                let rz = n2 as f64 / n1 as f64;
                out.push(Proposta {
                    familia: Familia::Corrente,
                    modulo_ou_passo: c.passo,
                    designacao: c.designacao.into(),
                    n1,
                    n2,
                    angulo_helice: 0.,
                    distancia: cc,
                    razao: rz,
                    erro_distancia: r.distancia.map_or(0., |p| cc - p),
                    erro_razao: r.razao.map_or(0., |p| rz - p),
                    diametro_maximo: dados::diametro_topo_roda(c.passo, n1).max(dados::diametro_topo_roda(c.passo, n2)),
                    elos,
                });
            }
        }
    }
    out
}

fn propor_correia(r: &Restricoes) -> Vec<Proposta> {
    let mut out = vec![];
    for c in dados::CORREIAS.iter().filter(|c| c.liberada() && c.familia == FamiliaCorreia::Dentada) {
        for n1 in 14..=60u32 {
            let candidatos: Vec<u32> = match r.razao {
                Some(i) => vec![(i * n1 as f64).floor() as u32, (i * n1 as f64).ceil() as u32],
                None => (n1..=(4 * n1).min(150)).collect(),
            };
            for n2 in candidatos {
                if n2 < n1 || n2 > 150 {
                    continue;
                }
                let (d1, d2) = (dados::diametro_polia_dentada(c.passo, n1), dados::diametro_polia_dentada(c.passo, n2));
                let pedido = r.distancia.unwrap_or(3. * (d1 + d2));
                let l = dados::comprimento_correia(d1, d2, pedido);
                let nt = dados::dentes_da_correia(c.passo, l);
                let Some(cc) = dados::distancia_da_correia(d1, d2, nt as f64 * c.passo) else { continue };
                if cc < (d1 + d2) / 2. {
                    continue;
                }
                let rz = n2 as f64 / n1 as f64;
                out.push(Proposta {
                    familia: Familia::CorreiaDentada,
                    modulo_ou_passo: c.passo,
                    designacao: c.designacao.into(),
                    n1,
                    n2,
                    angulo_helice: 0.,
                    distancia: cc,
                    razao: rz,
                    erro_distancia: r.distancia.map_or(0., |p| cc - p),
                    erro_razao: r.razao.map_or(0., |p| rz - p),
                    diametro_maximo: d1.max(d2),
                    elos: nt,
                });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retas_por_distancia_e_razao_acham_o_par_exato() {
        // 90 mm e razão 2: m2 z30/z60, m3 z20/z40... fecham sem erro
        let r = Restricoes { distancia: Some(90.), razao: Some(2.), ..Default::default() };
        let v = propor(Familia::Retas, &r, 6);
        assert!(v.len() >= 4);
        for p in v.iter().take(4) {
            assert!(p.erro_distancia.abs() < 1e-9 && p.erro_razao.abs() < 1e-9, "{p:?}");
            assert!((p.modulo_ou_passo * (p.n1 + p.n2) as f64 / 2. - 90.).abs() < 1e-9);
        }
        // só a distância: os pares fecham a distância com qualquer razão
        let so_a = propor(Familia::Retas, &Restricoes { distancia: Some(90.), ..Default::default() }, 6);
        assert!(so_a.iter().all(|p| p.erro_distancia.abs() < 1e-9));
        // sem pedido, nada a propor
        assert!(propor(Familia::Retas, &Restricoes::default(), 6).is_empty());
        assert!(propor(Familia::Retas, &Restricoes { distancia: Some(-1.), ..Default::default() }, 6).is_empty());
    }

    #[test]
    fn limite_de_espaco_e_modulo_fixo_filtram() {
        let base = Restricoes { distancia: Some(90.), razao: Some(2.), ..Default::default() };
        let todas = propor(Familia::Retas, &base, 50);
        // o diâmetro externo da maior roda: a coroa de 90·2/3·... limita o espaço
        let menor_maior = todas.iter().map(|p| p.diametro_maximo).fold(f64::MAX, f64::min);
        let apertado = propor(Familia::Retas, &Restricoes { espaco: Some(menor_maior + 0.01), ..base.clone() }, 50);
        assert!(!apertado.is_empty() && apertado.iter().all(|p| p.diametro_maximo <= menor_maior + 0.01));
        assert!(todas.iter().any(|p| p.diametro_maximo > menor_maior + 0.01), "o filtro tirou algo");
        assert!(propor(Familia::Retas, &Restricoes { espaco: Some(10.), ..base.clone() }, 50).is_empty());
        // módulo fixo em 3: só m3 (z20/z40)
        let m3 = propor(Familia::Retas, &Restricoes { modulo: Some(3.), ..base }, 50);
        assert!(m3.iter().all(|p| p.modulo_ou_passo == 3.) && m3[0].n1 == 20 && m3[0].n2 == 40, "{m3:?}");
    }

    #[test]
    fn helicoidal_fecha_a_distancia_exata_com_o_angulo() {
        // m2, 20/40 dá a = 60 reto; pedir 62 mm pede cos β = 60/62: β = 14,6°
        let r = Restricoes { distancia: Some(62.), razao: Some(2.), modulo: Some(2.), ..Default::default() };
        let v = propor(Familia::Helicoidais, &r, 6);
        assert!(!v.is_empty());
        let p = v.iter().find(|p| p.n1 == 20 && p.n2 == 40).expect("20/40");
        assert!((p.angulo_helice - (60_f64 / 62.).acos().to_degrees()).abs() < 1e-9, "{p:?}");
        assert!(p.erro_distancia == 0.);
        // o par resultante, conferido pelo cálculo de pares, tem mesmo a = 62
        let pn = ParamEngrenagem { tipo: TipoEngrenagem::Helicoidal, modulo: 2., dentes: 20, angulo_helice: p.angulo_helice, ..Default::default() };
        let d1 = calculo(&pn).unwrap().diametro_primitivo;
        let d2 = calculo(&ParamEngrenagem { dentes: 40, angulo_helice: -p.angulo_helice, ..pn }).unwrap().diametro_primitivo;
        assert!(((d1 + d2) / 2. - 62.).abs() < 1e-9);
        // fora de 8°–30° não propõe: 60 mm pede β = 0
        assert!(propor(Familia::Helicoidais, &Restricoes { distancia: Some(60.), razao: Some(2.), modulo: Some(2.), ..Default::default() }, 6).iter().all(|p| p.angulo_helice >= 8.));
    }

    #[test]
    fn corrente_propoe_elos_pares_e_a_distancia_corrigida() {
        let r = Restricoes { distancia: Some(400.), razao: Some(2.), ..Default::default() };
        let v = propor(Familia::Corrente, &r, 6);
        assert!(!v.is_empty());
        for p in &v {
            assert_eq!(p.elos % 2, 0, "{p:?}");
            assert!(dados::corrente(&p.designacao).unwrap().liberada(), "só linha conferida: {p:?}");
            // a distância corrigida é a que a fórmula devolve para o número de elos
            let c = dados::distancia_corrigida_corrente(p.modulo_ou_passo, p.n1, p.n2, p.elos).unwrap();
            assert!((c - p.distancia).abs() < 1e-9);
            // o erro contra o pedido é menos que um passo mais a folga do arredondamento de elos
            assert!(p.erro_distancia.abs() < 2. * p.modulo_ou_passo, "{p:?}");
            assert!((p.razao - 2.).abs() < 0.1);
        }
        assert!(propor(Familia::Corrente, &Restricoes { distancia: Some(400.), razao: Some(2.), espaco: Some(20.), ..Default::default() }, 6).is_empty());
    }

    #[test]
    fn correia_dentada_fecha_em_dentes_inteiros() {
        let r = Restricoes { distancia: Some(120.), razao: Some(2.), ..Default::default() };
        let v = propor(Familia::CorreiaDentada, &r, 6);
        assert!(!v.is_empty());
        for p in &v {
            let (d1, d2) = (dados::diametro_polia_dentada(p.modulo_ou_passo, p.n1), dados::diametro_polia_dentada(p.modulo_ou_passo, p.n2));
            // a correia fecha em dentes inteiros e o comprimento é o da fórmula na distância devolvida
            let l = dados::comprimento_correia(d1, d2, p.distancia);
            assert!((l - p.elos as f64 * p.modulo_ou_passo).abs() < 0.05, "{p:?} {l}");
            assert!(p.erro_distancia.abs() < p.modulo_ou_passo, "{p:?}");
        }
    }
}
