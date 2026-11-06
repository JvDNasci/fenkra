// SPDX-License-Identifier: LGPL-2.1-or-later
//! Cálculo de transmissão ao vivo (B343): relação, voltas, velocidade angular, torque, potência,
//! rendimento e perda de cada estágio, e a soma de uma cadeia de vários estágios.
//!
//! **Cálculo preliminar.** As fórmulas são as de elementos de máquinas de livro-texto (cinemática do
//! trem de engrenagens, conservação de potência com rendimento, e o rendimento do sem-fim por
//! ângulo de avanço e atrito). Os rendimentos de engrenagem, corrente e correia são **valores típicos
//! editáveis** (faixas de livros de elementos de máquinas), não um dado de norma: o painel os rotula
//! "valor típico, confira" e a pessoa pode trocá-los pelo do fabricante.
//!
//! Convenções:
//! - a razão de um estágio é `i = n2 / n1` (n = dentes, ou diâmetro primitivo nas correias), maior
//!   que 1 = redução; a razão de voltas é `1 / i` (voltas da saída por volta da entrada);
//! - `ω_saída = ω_entrada / i`; `T_saída = T_entrada · i · η`; `P_saída = P_entrada · η`;
//! - o **sinal do giro** é − nos pares externos de eixos paralelos (sentido contrário), + em interna,
//!   corrente e correia, e nos pares cônicos e sem-fim o sinal depende de como os eixos foram
//!   orientados: aqui as cônicas e o sem-fim seguem a convenção "eixos saindo do ponto de contato"
//!   (cônicas −, sem-fim −), a mesma que a montagem usa nas juntas.

use std::f64::consts::PI;

/// Fórmulas e dados usados, para o rodapé do painel.
pub const REFERENCIA: &str = "Cálculo preliminar: cinemática do trem de engrenagens e rendimento por estágio (livros de elementos de máquinas). Rendimentos típicos são editáveis; confira com o fabricante.";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TipoEstagio {
    Externa,
    Helicoidal,
    Interna,
    EspinhaDePeixe,
    Conica,
    SemFim,
    Cremalheira,
    Corrente,
    CorreiaDentada,
    CorreiaV,
    /// caixa como entrada e as duas laterais como saída: sem redução, com o torque dividido entre elas
    Diferencial,
}

impl TipoEstagio {
    pub const ALL: [TipoEstagio; 10] = [
        TipoEstagio::Externa,
        TipoEstagio::Helicoidal,
        TipoEstagio::Interna,
        TipoEstagio::EspinhaDePeixe,
        TipoEstagio::Conica,
        TipoEstagio::SemFim,
        TipoEstagio::Cremalheira,
        TipoEstagio::Corrente,
        TipoEstagio::CorreiaDentada,
        TipoEstagio::CorreiaV,
    ];

    pub fn rotulo(self) -> &'static str {
        match self {
            TipoEstagio::Externa => "Engrenagem reta",
            TipoEstagio::Helicoidal => "Engrenagem helicoidal",
            TipoEstagio::Interna => "Engrenagem interna",
            TipoEstagio::EspinhaDePeixe => "Espinha de peixe",
            TipoEstagio::Conica => "Cônica a 90°",
            TipoEstagio::SemFim => "Sem-fim e coroa",
            TipoEstagio::Cremalheira => "Pinhão e cremalheira",
            TipoEstagio::Corrente => "Corrente",
            TipoEstagio::CorreiaDentada => "Correia dentada",
            TipoEstagio::CorreiaV => "Correia em V",
            TipoEstagio::Diferencial => "Diferencial de caixa solta",
        }
    }

    /// Rendimento típico de um estágio (valor preliminar, faixa de livro-texto; editável).
    pub fn rendimento_tipico(self) -> f64 {
        match self {
            TipoEstagio::Externa | TipoEstagio::Interna => 0.98,
            TipoEstagio::Helicoidal | TipoEstagio::EspinhaDePeixe => 0.98,
            TipoEstagio::Conica | TipoEstagio::Diferencial => 0.97,
            TipoEstagio::Cremalheira => 0.95,
            TipoEstagio::Corrente => 0.97,
            TipoEstagio::CorreiaDentada => 0.98,
            TipoEstagio::CorreiaV => 0.95,
            // o sem-fim sai da fórmula, não de um valor típico
            TipoEstagio::SemFim => 0.7,
        }
    }

    /// Sinal do giro da saída em relação à entrada (ver convenção no topo do módulo).
    pub fn sinal(self) -> f64 {
        match self {
            TipoEstagio::Externa | TipoEstagio::Helicoidal | TipoEstagio::EspinhaDePeixe | TipoEstagio::Conica | TipoEstagio::SemFim => -1.,
            // a entrada é a caixa, e a média das duas laterais gira com ela (ω caixa = (ω1 + ω2) / 2)
            TipoEstagio::Interna | TipoEstagio::Corrente | TipoEstagio::CorreiaDentada | TipoEstagio::CorreiaV | TipoEstagio::Cremalheira | TipoEstagio::Diferencial => 1.,
        }
    }
}

/// Um estágio: o par motor/movido. `n1` e `n2` são os dentes (ou diâmetros nas correias); no sem-fim,
/// `n1` são as **entradas** (filetes) do sem-fim e `n2` os dentes da coroa.
#[derive(Clone, Debug, PartialEq)]
pub struct Estagio {
    pub tipo: TipoEstagio,
    pub n1: f64,
    pub n2: f64,
    /// rendimento escolhido (0 a 1); `None` usa o típico (ou a fórmula, no sem-fim)
    pub rendimento: Option<f64>,
    /// sem-fim: ângulo de avanço λ no cilindro primitivo (graus)
    pub angulo_avanco: f64,
    /// sem-fim: ângulo de pressão normal (graus)
    pub angulo_pressao: f64,
    /// sem-fim: coeficiente de atrito entre os flancos
    pub atrito: f64,
}

impl Estagio {
    pub fn novo(tipo: TipoEstagio, n1: f64, n2: f64) -> Estagio {
        Estagio { tipo, n1, n2, rendimento: None, angulo_avanco: 5.71, angulo_pressao: 20., atrito: 0.05 }
    }
}

/// Rendimento do sem-fim girando a partir do sem-fim: `η = (cos φn − f tan λ) / (cos φn + f cot λ)`
/// (fórmula clássica de elementos de máquinas). Ângulos em graus.
pub fn rendimento_sem_fim(angulo_pressao_normal: f64, angulo_avanco: f64, atrito: f64) -> f64 {
    let (phi, lam) = (angulo_pressao_normal.to_radians(), angulo_avanco.to_radians());
    let r = (phi.cos() - atrito * lam.tan()) / (phi.cos() + atrito / lam.tan());
    r.clamp(0., 1.)
}

/// Sem-fim autotravante (a coroa não move o sem-fim): vale quando `tan λ <= f / cos φn`, ou seja,
/// o rendimento no sentido inverso é nulo.
pub fn sem_fim_autotravante(angulo_pressao_normal: f64, angulo_avanco: f64, atrito: f64) -> bool {
    angulo_avanco.to_radians().tan() <= atrito / angulo_pressao_normal.to_radians().cos()
}

/// Entrada da cadeia: velocidade e (torque ou potência).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Entrada {
    /// rotações por minuto (com sinal)
    pub rpm: f64,
    /// torque na entrada, N·m
    pub torque: f64,
}

impl Entrada {
    pub fn potencia_w(&self) -> f64 {
        self.torque * self.rpm.abs() * PI / 30.
    }

    /// Entrada a partir de potência (W) e rpm.
    pub fn de_potencia(potencia_w: f64, rpm: f64) -> Entrada {
        let w = rpm.abs() * PI / 30.;
        Entrada { rpm, torque: if w > 0. { potencia_w / w } else { 0. } }
    }
}

/// Resultado de um estágio.
#[derive(Clone, Debug, PartialEq)]
pub struct ResultadoEstagio {
    pub tipo: TipoEstagio,
    /// razão de transmissão i = n2/n1 (sem-fim: dentes da coroa / entradas)
    pub razao: f64,
    /// voltas da saída por volta da entrada (1/i, com o sinal do giro)
    pub voltas: f64,
    pub rendimento: f64,
    pub rpm_entrada: f64,
    pub rpm_saida: f64,
    /// velocidade angular na saída, rad/s (com sinal)
    pub omega_saida: f64,
    pub torque_entrada: f64,
    pub torque_saida: f64,
    pub potencia_entrada: f64,
    pub potencia_saida: f64,
    pub perda: f64,
    pub avisos: Vec<String>,
}

/// Total da cadeia.
#[derive(Clone, Debug, PartialEq)]
pub struct ResultadoCadeia {
    pub estagios: Vec<ResultadoEstagio>,
    pub razao_total: f64,
    /// voltas da saída por volta da entrada, com sinal
    pub voltas_total: f64,
    pub rendimento_total: f64,
    pub rpm_saida: f64,
    pub omega_saida: f64,
    pub torque_saida: f64,
    pub potencia_entrada: f64,
    pub potencia_saida: f64,
    pub perda_total: f64,
}

impl ResultadoCadeia {
    /// Razão que a junta de acoplamento da montagem usa entre o eixo de entrada e o de saída:
    /// giro da saída por giro da entrada, com sinal.
    pub fn razao_de_junta(&self) -> f64 {
        self.voltas_total
    }
}

/// Calcula um estágio dada a entrada.
pub fn calcular_estagio(e: &Estagio, entrada: Entrada) -> Result<ResultadoEstagio, String> {
    if !(e.n1.is_finite() && e.n2.is_finite()) || e.n1 <= 0. || e.n2 <= 0. {
        return Err("o estágio precisa de dentes (ou diâmetros) positivos".into());
    }
    if !(entrada.rpm.is_finite() && entrada.torque.is_finite()) {
        return Err("a entrada precisa de rpm e torque finitos".into());
    }
    let mut avisos = vec![];
    let razao = e.n2 / e.n1;
    let rend = match (e.rendimento, e.tipo) {
        (Some(r), _) => r,
        (None, TipoEstagio::SemFim) => {
            if sem_fim_autotravante(e.angulo_pressao, e.angulo_avanco, e.atrito) {
                avisos.push("sem-fim autotravante: a coroa não aciona o sem-fim, e o rendimento direto fica abaixo de 50%".into());
            }
            rendimento_sem_fim(e.angulo_pressao, e.angulo_avanco, e.atrito)
        }
        (None, t) => t.rendimento_tipico(),
    };
    if !(0. < rend && rend <= 1.) {
        return Err("o rendimento precisa estar entre 0 e 1".into());
    }
    let sinal = e.tipo.sinal();
    let rpm_saida = sinal * entrada.rpm / razao;
    let p_in = entrada.potencia_w();
    let p_out = p_in * rend;
    let torque_saida = entrada.torque * razao * rend;
    Ok(ResultadoEstagio {
        tipo: e.tipo,
        razao,
        voltas: sinal / razao,
        rendimento: rend,
        rpm_entrada: entrada.rpm,
        rpm_saida,
        omega_saida: rpm_saida * PI / 30.,
        torque_entrada: entrada.torque,
        torque_saida,
        potencia_entrada: p_in,
        potencia_saida: p_out,
        perda: p_in - p_out,
        avisos,
    })
}

/// Calcula a cadeia: a saída de cada estágio é a entrada do seguinte. Os totais são o produto das
/// razões e dos rendimentos e a soma das perdas.
pub fn calcular_cadeia(estagios: &[Estagio], entrada: Entrada) -> Result<ResultadoCadeia, String> {
    if estagios.is_empty() {
        return Err("a cadeia precisa de ao menos um estágio".into());
    }
    let mut atual = entrada;
    let mut res = vec![];
    for (k, e) in estagios.iter().enumerate() {
        let r = calcular_estagio(e, atual).map_err(|m| format!("estágio {}: {m}", k + 1))?;
        atual = Entrada { rpm: r.rpm_saida, torque: r.torque_saida };
        res.push(r);
    }
    let razao_total: f64 = res.iter().map(|r| r.razao).product();
    let voltas_total: f64 = res.iter().map(|r| r.voltas).product();
    let rendimento_total: f64 = res.iter().map(|r| r.rendimento).product();
    let ult = res.last().ok_or("cadeia vazia")?;
    let perda_total = res.iter().map(|r| r.perda).sum();
    Ok(ResultadoCadeia {
        razao_total,
        voltas_total,
        rendimento_total,
        rpm_saida: ult.rpm_saida,
        omega_saida: ult.omega_saida,
        torque_saida: ult.torque_saida,
        potencia_entrada: res[0].potencia_entrada,
        potencia_saida: ult.potencia_saida,
        perda_total,
        estagios: res,
    })
}

/// Diferencial de engrenagens cônicas: a caixa (porta-satélites) gira na média das duas saídas,
/// `ω_caixa = (ω1 + ω2) / 2` (equação de Willis com razão −1), e o torque se divide em partes
/// iguais entre as saídas. Devolve (rpm da caixa, torque de cada saída para um torque `t_caixa`).
pub fn diferencial(rpm1: f64, rpm2: f64, t_caixa: f64) -> (f64, f64) {
    ((rpm1 + rpm2) / 2., t_caixa / 2.)
}

/// Rpm da segunda saída do diferencial, dadas a caixa e a primeira saída: `ω2 = 2 ω_caixa − ω1`.
pub fn diferencial_segunda_saida(rpm_caixa: f64, rpm1: f64) -> f64 {
    2. * rpm_caixa - rpm1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn perto(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    #[test]
    fn par_reto_20_40_exemplo_numerico() {
        // 1500 rpm, 10 N·m: P = 10 · 157,08 = 1570,8 W. i = 2, η = 0,98:
        // 750 rpm (sentido contrário), 19,6 N·m, 1539,4 W, perda 31,4 W
        let e = Entrada { rpm: 1500., torque: 10. };
        let r = calcular_estagio(&Estagio::novo(TipoEstagio::Externa, 20., 40.), e).unwrap();
        assert!(perto(r.razao, 2., 1e-12) && perto(r.voltas, -0.5, 1e-12));
        assert!(perto(r.rpm_saida, -750., 1e-9));
        assert!(perto(r.torque_saida, 19.6, 1e-9));
        assert!(perto(r.potencia_entrada, 1570.796, 1e-2) && perto(r.potencia_saida, 1539.380, 1e-2) && perto(r.perda, 31.416, 1e-2));
        assert!(perto(r.omega_saida, -750. * PI / 30., 1e-9));
    }

    #[test]
    fn potencia_se_conserva_menos_a_perda() {
        let r = calcular_estagio(&Estagio::novo(TipoEstagio::Helicoidal, 17., 51.), Entrada::de_potencia(2000., 1000.)).unwrap();
        assert!(perto(r.potencia_entrada, 2000., 1e-9));
        // P = T ω nos dois lados
        let p_saida = r.torque_saida * r.omega_saida.abs();
        assert!(perto(p_saida, r.potencia_saida, 1e-6), "{p_saida} {}", r.potencia_saida);
        assert!(perto(r.potencia_entrada - r.potencia_saida, r.perda, 1e-9));
    }

    #[test]
    fn sem_fim_rendimento_da_formula() {
        // λ = atan(0,1) = 5,7106°, f = 0,05, φn = 20°: η = (cos20 − 0,005) / (cos20 + 0,5) = 0,6492
        let lam = 0.1_f64.atan().to_degrees();
        let eta = rendimento_sem_fim(20., lam, 0.05);
        let esperado = (20_f64.to_radians().cos() - 0.005) / (20_f64.to_radians().cos() + 0.5);
        assert!(perto(eta, esperado, 1e-9) && perto(eta, 0.6492, 1e-3), "{eta}");
        // avanço maior rende mais; atrito maior rende menos
        assert!(rendimento_sem_fim(20., 15., 0.05) > eta);
        assert!(rendimento_sem_fim(20., lam, 0.1) < eta);
        // autotravante: avanço pequeno (tan λ = 0,02 < 0,05/cos20)
        assert!(sem_fim_autotravante(20., 0.02_f64.atan().to_degrees(), 0.05));
        assert!(!sem_fim_autotravante(20., 15., 0.05));
    }

    #[test]
    fn sem_fim_usa_entradas_e_a_formula() {
        let mut e = Estagio::novo(TipoEstagio::SemFim, 1., 30.);
        e.angulo_avanco = 0.1_f64.atan().to_degrees();
        let r = calcular_estagio(&e, Entrada { rpm: 1500., torque: 1. }).unwrap();
        assert!(perto(r.razao, 30., 1e-12) && perto(r.rpm_saida.abs(), 50., 1e-9));
        assert!(perto(r.rendimento, 0.6492, 1e-3));
        assert!(perto(r.torque_saida, 30. * r.rendimento, 1e-9));
        // sem-fim de 2 entradas, 30 dentes: i = 15
        e.n1 = 2.;
        assert!(perto(calcular_estagio(&e, Entrada { rpm: 1500., torque: 1. }).unwrap().razao, 15., 1e-12));
    }

    #[test]
    fn cadeia_de_dois_estagios_soma_o_resultado() {
        // 20/40 reta e 15/60 por corrente: i = 2 · 4 = 8, η = 0,98 · 0,97
        let est = [Estagio::novo(TipoEstagio::Externa, 20., 40.), Estagio::novo(TipoEstagio::Corrente, 15., 60.)];
        let c = calcular_cadeia(&est, Entrada { rpm: 1600., torque: 5. }).unwrap();
        assert!(perto(c.razao_total, 8., 1e-12));
        assert!(perto(c.rendimento_total, 0.98 * 0.97, 1e-12));
        // reta inverte, corrente mantém: saída gira a −1600/8
        assert!(perto(c.rpm_saida, -200., 1e-9) && perto(c.voltas_total, -0.125, 1e-12));
        assert!(perto(c.torque_saida, 5. * 8. * 0.98 * 0.97, 1e-9));
        let p_in = 5. * 1600. * PI / 30.;
        assert!(perto(c.potencia_entrada, p_in, 1e-9) && perto(c.potencia_saida, p_in * 0.98 * 0.97, 1e-9));
        // a perda total é a soma das perdas dos estágios e fecha o balanço
        assert!(perto(c.perda_total, c.potencia_entrada - c.potencia_saida, 1e-9));
        assert!(perto(c.estagios[0].perda + c.estagios[1].perda, c.perda_total, 1e-12));
        assert!(perto(c.razao_de_junta(), -0.125, 1e-12));
    }

    #[test]
    fn rendimento_escolhido_vale_e_invalido_e_recusado() {
        let mut e = Estagio::novo(TipoEstagio::CorreiaV, 100., 200.);
        e.rendimento = Some(0.9);
        let r = calcular_estagio(&e, Entrada { rpm: 1000., torque: 2. }).unwrap();
        assert!(perto(r.rendimento, 0.9, 1e-12) && perto(r.torque_saida, 2. * 2. * 0.9, 1e-12));
        e.rendimento = Some(1.2);
        assert!(calcular_estagio(&e, Entrada { rpm: 1000., torque: 2. }).is_err());
        assert!(calcular_estagio(&Estagio::novo(TipoEstagio::Externa, 0., 10.), Entrada { rpm: 1., torque: 1. }).is_err());
        assert!(calcular_cadeia(&[], Entrada { rpm: 1., torque: 1. }).is_err());
    }

    #[test]
    fn diferencial_media_das_saidas() {
        let (caixa, t) = diferencial(100., 60., 40.);
        assert!(perto(caixa, 80., 1e-12) && perto(t, 20., 1e-12));
        // em curva uma roda gira mais, a outra menos, e a média é a da caixa; preso um lado, o outro dobra
        assert!(perto(diferencial_segunda_saida(80., 100.), 60., 1e-12));
        assert!(perto(diferencial_segunda_saida(50., 0.), 100., 1e-12));
    }
}
