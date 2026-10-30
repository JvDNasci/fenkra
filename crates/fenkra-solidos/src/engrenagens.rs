//! Gerador de engrenagens (estilo Design Accelerator): cálculo dos diâmetros e documento paramétrico
//! de engrenagem reta, helicoidal, interna e cremalheira, com perfil de dente em evolvente.
//!
//! **Convenção de eixos** (a montagem conta com ela):
//! - o eixo da engrenagem é o **Z** do documento e a **face inferior** fica em **z = 0** (largura
//!   para +Z); o dente 0 é centrado no eixo +X;
//! - a **cremalheira** corre ao longo de **X** (de x = 0 a x = L), com a **linha primitiva em y = 0**,
//!   os dentes para +Y e o corpo para -Y, largura em +Z.
//!
//! Geometria: um dente é amostrado pela evolvente `x = rb(cosθ + θ·senθ)`, `y = rb(senθ − θ·cosθ)`,
//! espelhado para o outro flanco; cada flanco vira uma spline que passa pelos pontos, o topo e a raiz
//! são arcos e, abaixo do círculo base, o flanco desce em linha radial até a raiz. A folga (backlash)
//! sai da espessura do dente no círculo primitivo. Sem filete de raiz nem trocoide.
//!
//! O perfil fica no sketch como coordenadas fixas (a cada `m`, `z`, `alpha`... diferentes gera-se outro
//! documento); a largura `b` e o furo central são paramétricos no documento. A **helicoidal** é um
//! loft reto (`ruled`) entre seções do perfil giradas de `z·tan β / r`, com passo máximo de 4° por
//! trecho (o kernel não tem extrusão com torção); o ângulo de hélice fica gravado nas seções.
//! A **interna** é um anel com o vão dos dentes desenhado no sketch (sempre reta).

use crate::biblioteca::chaveta_para_eixo;
use crate::fixadores::Montador;
use fenkra_core::sketch::Curve;
use fenkra_core::*;
use std::f64::consts::PI;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum TipoEngrenagem {
    Reta,
    Helicoidal,
    Interna,
    Cremalheira,
}

impl TipoEngrenagem {
    pub const ALL: [TipoEngrenagem; 4] = [TipoEngrenagem::Reta, TipoEngrenagem::Helicoidal, TipoEngrenagem::Interna, TipoEngrenagem::Cremalheira];

    pub fn rotulo(self) -> &'static str {
        match self {
            TipoEngrenagem::Reta => "Reta",
            TipoEngrenagem::Helicoidal => "Helicoidal",
            TipoEngrenagem::Interna => "Interna",
            TipoEngrenagem::Cremalheira => "Cremalheira",
        }
    }
}

/// Entradas do gerador. Ângulos em graus, medidas em mm.
#[derive(Clone, Debug, PartialEq)]
pub struct ParamEngrenagem {
    pub tipo: TipoEngrenagem,
    /// módulo normal m
    pub modulo: f64,
    /// número de dentes z (a cremalheira ignora: quem conta os dentes é o comprimento)
    pub dentes: u32,
    /// ângulo de pressão normal
    pub angulo_pressao: f64,
    /// ângulo de hélice β no primitivo (só helicoidal); o sinal dá o sentido (+ = direita)
    pub angulo_helice: f64,
    pub largura: f64,
    /// folga entre dentes (backlash), tirada da espessura do dente no primitivo
    pub folga_dentes: f64,
    /// coeficiente de correção do perfil x (só reta e helicoidal)
    pub coef_correcao: f64,
    /// diâmetro do furo central; 0 = sem furo (a interna e a cremalheira ignoram)
    pub furo: f64,
    /// comprimento da barra (só cremalheira)
    pub comprimento_cremalheira: f64,
    /// rasgo de chaveta no cubo (DIN 6885 forma A pelo diâmetro do furo); só reta/helicoidal com furo > 0
    pub rasgo_cubo: bool,
}

impl Default for ParamEngrenagem {
    fn default() -> Self {
        ParamEngrenagem {
            tipo: TipoEngrenagem::Reta,
            modulo: 2.,
            dentes: 20,
            angulo_pressao: 20.,
            angulo_helice: 15.,
            largura: 10.,
            folga_dentes: 0.,
            coef_correcao: 0.,
            furo: 0.,
            comprimento_cremalheira: 100.,
            rasgo_cubo: false,
        }
    }
}

impl ParamEngrenagem {
    /// Descrição da peça, ex.: "Engrenagem reta m2 z24".
    pub fn descricao(&self) -> String {
        let m = self.modulo;
        match self.tipo {
            TipoEngrenagem::Reta => format!("Engrenagem reta m{m} z{}", self.dentes),
            TipoEngrenagem::Helicoidal => format!("Engrenagem helicoidal m{m} z{} β{}", self.dentes, self.angulo_helice),
            TipoEngrenagem::Interna => format!("Engrenagem interna m{m} z{}", self.dentes),
            TipoEngrenagem::Cremalheira => format!("Cremalheira m{m} L{}", self.comprimento_cremalheira),
        }
    }

    /// Nome de arquivo (sem extensão) que identifica o conjunto de parâmetros que mudam a peça.
    pub fn chave(&self) -> String {
        let base = format!("m{}-a{}-b{}-f{}", self.modulo, self.angulo_pressao, self.largura, self.folga_dentes);
        let rasgo = if self.rasgo_cubo && self.furo > 0. { "-k" } else { "" };
        match self.tipo {
            TipoEngrenagem::Reta => format!("engrenagem-reta-{base}-z{}-x{}-d{}{rasgo}", self.dentes, self.coef_correcao, self.furo),
            TipoEngrenagem::Helicoidal => format!("engrenagem-helicoidal-{base}-z{}-x{}-d{}-h{}{rasgo}", self.dentes, self.coef_correcao, self.furo, self.angulo_helice),
            TipoEngrenagem::Interna => format!("engrenagem-interna-{base}-z{}", self.dentes),
            TipoEngrenagem::Cremalheira => format!("cremalheira-{base}-l{}", self.comprimento_cremalheira),
        }
    }
}

/// Resultado do cálculo (medidas no plano transversal; zero onde não se aplica).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CalculoEngrenagem {
    /// m_t = m / cos β
    pub modulo_transversal: f64,
    pub angulo_pressao_transversal: f64,
    pub diametro_primitivo: f64,
    pub diametro_base: f64,
    /// diâmetro de topo (na interna, o dos dentes que apontam para dentro)
    pub diametro_externo: f64,
    pub diametro_raiz: f64,
    /// diâmetro externo do anel da engrenagem interna
    pub diametro_anel: f64,
    /// passo circular transversal
    pub passo: f64,
    /// espessura do dente no primitivo, já sem a folga
    pub espessura_dente: f64,
    pub altura_dente: f64,
    /// dentes da barra (só cremalheira)
    pub dentes_cremalheira: u32,
    pub avisos: Vec<String>,
}

/// Dimensões de um dente em evolvente (plano transversal). Na interna descreve o **vão** entre dois
/// dentes, que tem o mesmo desenho de um dente externo.
struct Dente {
    r: f64,
    rb: f64,
    ra: f64,
    rf: f64,
    /// espessura em arco no círculo primitivo
    sp: f64,
    /// ângulo de pressão transversal (rad)
    alfa: f64,
    z: u32,
}

/// Passo do contorno: o ponto final (a spline traz todos os pontos depois do início).
enum Passo {
    Linha([f64; 2]),
    Arco([f64; 2]),
    Spline(Vec<[f64; 2]>),
}

fn inv(t: f64) -> f64 {
    t - t.atan()
}

fn polar(r: f64, a: f64) -> [f64; 2] {
    [r * a.cos(), r * a.sin()]
}

/// Parâmetros do dente da reta/helicoidal.
fn dente_externo(p: &ParamEngrenagem) -> Dente {
    let beta = if p.tipo == TipoEngrenagem::Helicoidal { p.angulo_helice.to_radians() } else { 0. };
    let (m, x) = (p.modulo, p.coef_correcao);
    let alfa_n = p.angulo_pressao.to_radians();
    let cb = beta.cos();
    let alfa = (alfa_n.tan() / cb).atan();
    let r = m / cb * p.dentes as f64 / 2.;
    Dente { r, rb: r * alfa.cos(), ra: r + m * (1. + x), rf: r - m * (1.25 - x), sp: (m * (PI / 2. + 2. * x * alfa_n.tan()) - p.folga_dentes) / cb, alfa, z: p.dentes }
}

/// Vão da engrenagem interna: topo do vão na raiz da interna (r + 1,25m), fundo no topo dos dentes
/// (r − m) e largura no primitivo = passo − espessura do dente.
fn dente_interno(p: &ParamEngrenagem) -> Dente {
    let (m, alfa) = (p.modulo, p.angulo_pressao.to_radians());
    let r = m * p.dentes as f64 / 2.;
    Dente { r, rb: r * alfa.cos(), ra: r + 1.25 * m, rf: r - m, sp: PI * m / 2. + p.folga_dentes, alfa, z: p.dentes }
}

/// Limites do flanco: (θ no pé, θ no topo, meio ângulo do dente no pé, meio ângulo no topo).
struct Limites {
    th0: f64,
    th1: f64,
    psi0: f64,
    psia: f64,
}

impl Dente {
    fn theta(&self, r: f64) -> f64 {
        ((r / self.rb).powi(2) - 1.).max(0.).sqrt()
    }

    /// Meio ângulo do dente (do eixo do dente ao flanco) no ponto θ da evolvente.
    fn meio_angulo(&self, theta: f64) -> f64 {
        self.sp / (2. * self.r) + inv(self.alfa.tan()) - inv(theta)
    }

    fn limites(&self) -> Result<Limites, String> {
        if self.ra <= self.rb * 1.0001 {
            return Err("o círculo de topo não passa do círculo base — aumente o número de dentes".into());
        }
        if self.rf <= 0. {
            return Err("o círculo de raiz é nulo — aumente o número de dentes ou reduza a correção".into());
        }
        let th0 = if self.rf > self.rb { self.theta(self.rf) } else { 0. };
        let th1 = self.theta(self.ra);
        let (psi0, psia) = (self.meio_angulo(th0), self.meio_angulo(th1));
        if psia <= 0.002 {
            return Err("dente pontiagudo: o flanco se cruza antes do topo — reduza a correção x ou aumente z".into());
        }
        if 2. * PI / self.z as f64 - 2. * psi0 <= 0.002 {
            return Err("os dentes se tocam na raiz — reduza a correção x ou a folga negativa".into());
        }
        Ok(Limites { th0, th1, psi0, psia })
    }

    /// Contorno completo com os `z` dentes, no sentido anti-horário: ponto inicial e passos (o último
    /// termina no inicial).
    fn contorno(&self, flanco: usize) -> Result<([f64; 2], Vec<Passo>), String> {
        let l = self.limites()?;
        let abaixo = self.rf < self.rb;
        let r_de = |th: f64| self.rb * (1. + th * th).sqrt();
        let ths: Vec<f64> = (0..=flanco).map(|i| l.th0 + (l.th1 - l.th0) * i as f64 / flanco as f64).collect();
        let passo_ang = 2. * PI / self.z as f64;
        let inicio = polar(self.rf, -l.psi0);
        let mut passos = vec![];
        for k in 0..self.z {
            let fi = passo_ang * k as f64;
            if abaixo {
                passos.push(Passo::Linha(polar(self.rb, fi - l.psi0)));
            }
            passos.push(Passo::Spline(ths[1..].iter().map(|th| polar(r_de(*th), fi - self.meio_angulo(*th))).collect()));
            passos.push(Passo::Arco(polar(self.ra, fi + l.psia)));
            passos.push(Passo::Spline(ths[..flanco].iter().rev().map(|th| polar(r_de(*th), fi + self.meio_angulo(*th))).collect()));
            if abaixo {
                passos.push(Passo::Linha(polar(self.rf, fi + l.psi0)));
            }
            passos.push(Passo::Arco(polar(self.rf, fi + passo_ang - l.psi0)));
        }
        Ok((inicio, passos))
    }
}

fn num(v: f64, nome: &str) -> Result<(), String> {
    if v.is_finite() {
        Ok(())
    } else {
        Err(format!("{nome} inválido"))
    }
}

/// Valida as entradas e calcula as dimensões da engrenagem.
pub fn calculo(p: &ParamEngrenagem) -> Result<CalculoEngrenagem, String> {
    for (v, n) in [(p.modulo, "módulo"), (p.angulo_pressao, "ângulo de pressão"), (p.angulo_helice, "ângulo de hélice"), (p.largura, "largura"), (p.folga_dentes, "folga"), (p.coef_correcao, "coeficiente de correção"), (p.furo, "furo"), (p.comprimento_cremalheira, "comprimento")] {
        num(v, n)?;
    }
    if p.modulo <= 0. || p.modulo > 100. {
        return Err("o módulo precisa estar entre 0 e 100 mm".into());
    }
    if !(14.5..=30.).contains(&p.angulo_pressao) {
        return Err("o ângulo de pressão precisa estar entre 14,5° e 30°".into());
    }
    if p.largura <= 0. {
        return Err("a largura precisa ser positiva".into());
    }
    if p.folga_dentes < 0. || p.folga_dentes > 0.5 * p.modulo {
        return Err("a folga precisa estar entre 0 e metade do módulo".into());
    }
    if p.furo < 0. {
        return Err("o furo não pode ser negativo".into());
    }
    let m = p.modulo;
    let mut c = CalculoEngrenagem { modulo_transversal: m, angulo_pressao_transversal: p.angulo_pressao, ..Default::default() };
    match p.tipo {
        TipoEngrenagem::Cremalheira => {
            let passo = PI * m;
            if p.comprimento_cremalheira < 2. * passo || p.comprimento_cremalheira > 10_000. {
                return Err(format!("o comprimento da cremalheira precisa estar entre {:.1} (2 dentes) e 10000 mm", 2. * passo));
            }
            c.passo = passo;
            c.espessura_dente = passo / 2. - p.folga_dentes;
            c.altura_dente = 2.25 * m;
            c.dentes_cremalheira = (p.comprimento_cremalheira / passo).floor() as u32;
            if c.espessura_dente / 2. - m * p.angulo_pressao.to_radians().tan() <= 0.01 * m {
                return Err("dente pontiagudo: reduza a folga".into());
            }
            if p.coef_correcao != 0. {
                c.avisos.push("o coeficiente de correção não se aplica à cremalheira".into());
            }
        }
        TipoEngrenagem::Interna => {
            if p.dentes < 12 {
                return Err("a engrenagem interna precisa de pelo menos 12 dentes".into());
            }
            if p.dentes > 400 {
                return Err("no máximo 400 dentes".into());
            }
            let g = dente_interno(p);
            g.limites()?;
            c.diametro_primitivo = 2. * g.r;
            c.diametro_base = 2. * g.rb;
            c.diametro_externo = c.diametro_primitivo - 2. * m;
            c.diametro_raiz = c.diametro_primitivo + 2.5 * m;
            c.diametro_anel = c.diametro_raiz + 5. * m;
            c.passo = PI * m;
            c.espessura_dente = PI * m / 2. - p.folga_dentes;
            c.altura_dente = 2.25 * m;
            if p.coef_correcao != 0. {
                c.avisos.push("o coeficiente de correção é ignorado na engrenagem interna".into());
            }
            if p.furo != 0. {
                c.avisos.push("o furo central não se aplica à engrenagem interna".into());
            }
        }
        TipoEngrenagem::Reta | TipoEngrenagem::Helicoidal => {
            if p.dentes < 6 {
                return Err("são precisos pelo menos 6 dentes".into());
            }
            if p.dentes > 400 {
                return Err("no máximo 400 dentes".into());
            }
            if p.coef_correcao.abs() > 1. {
                return Err("o coeficiente de correção precisa estar entre -1 e 1".into());
            }
            let helicoidal = p.tipo == TipoEngrenagem::Helicoidal;
            if helicoidal && p.angulo_helice.abs() > 45. {
                return Err("o ângulo de hélice precisa estar entre -45° e 45°".into());
            }
            let g = dente_externo(p);
            let l = g.limites()?;
            let beta = if helicoidal { p.angulo_helice.to_radians() } else { 0. };
            c.modulo_transversal = m / beta.cos();
            c.angulo_pressao_transversal = g.alfa.to_degrees();
            c.diametro_primitivo = 2. * g.r;
            c.diametro_base = 2. * g.rb;
            c.diametro_externo = 2. * g.ra;
            c.diametro_raiz = 2. * g.rf;
            c.passo = PI * c.modulo_transversal;
            c.espessura_dente = g.sp;
            c.altura_dente = g.ra - g.rf;
            if p.furo >= c.diametro_raiz - 2. * m {
                return Err(format!("o furo precisa ser menor que {:.2} mm (deixa 1 módulo de aro sob a raiz)", c.diametro_raiz - 2. * m));
            }
            if p.rasgo_cubo && p.furo > 0. {
                let k = chaveta_para_eixo(p.furo).ok_or_else(|| format!("furo Ø{} fora da tabela DIN 6885 (Ø6 a Ø85): sem rasgo de chaveta", p.furo))?;
                if p.furo + 2. * k.t2 >= c.diametro_raiz - 2. * m {
                    return Err(format!("o rasgo do cubo (furo + {:.1} mm) passa da raiz: aumente o módulo ou os dentes", k.t2));
                }
            }
            let z_min = 2. * (1. - p.coef_correcao) / g.alfa.sin().powi(2);
            if (p.dentes as f64) < z_min.floor() {
                c.avisos.push(format!(
                    "z = {} < {}: risco de rebaixo (interferência) no pé do dente — use correção x ≥ {:.2}",
                    p.dentes,
                    z_min.floor(),
                    (z_min.floor() - p.dentes as f64) / z_min.floor()
                ));
            }
            if 2. * g.ra * l.psia < 0.25 * m {
                c.avisos.push(format!("dente fino no topo ({:.2} mm)", 2. * g.ra * l.psia));
            }
            if helicoidal && p.angulo_helice.abs() < 1e-6 {
                c.avisos.push("ângulo de hélice 0°: a peça sai como engrenagem reta".into());
            }
        }
    }
    Ok(c)
}

/// Documento paramétrico da engrenagem (peça `.fkp`).
pub fn engrenagem_documento(p: &ParamEngrenagem) -> Result<Document, String> {
    calculo(p)?;
    let mut m = Montador::novo(&p.descricao());
    m.doc.props.material = "Aço 1045".into();
    m.doc.props.part_type = PartType::Fabricada;
    m.parametros(p)?;
    match p.tipo {
        TipoEngrenagem::Reta => m.reta(&dente_externo(p))?,
        TipoEngrenagem::Helicoidal if p.angulo_helice.abs() < 1e-6 => m.reta(&dente_externo(p))?,
        TipoEngrenagem::Helicoidal => m.helicoidal(&dente_externo(p), p.angulo_helice.to_radians(), p.largura)?,
        TipoEngrenagem::Interna => m.interna(&dente_interno(p), calculo(p)?.diametro_anel)?,
        TipoEngrenagem::Cremalheira => m.cremalheira(p)?,
    }
    if p.furo > 0. && matches!(p.tipo, TipoEngrenagem::Reta | TipoEngrenagem::Helicoidal) {
        let plano = m.plano("Plano_furo", "-1")?;
        m.prim("Furo", PrimKind::Cilindro, plano, ["furo/2", "b+2", "0", "0"], BoolOp::Cut)?;
        if p.rasgo_cubo {
            // rasgo do cubo: largura b e profundidade t2 a partir do furo, em +X, ao longo da largura
            let k = chaveta_para_eixo(p.furo).ok_or("furo fora da tabela DIN 6885")?;
            m.param("kb", &k.b.to_string())?;
            m.param("kt2", &k.t2.to_string())?;
            let plano = crate::eixos::plano_yz(&mut m, "Plano_rasgo_cubo", "furo/2-1")?;
            let mut data = SketchData::default();
            let mut al = || m.doc.alloc();
            data.add_rect(&mut al, [-k.b / 2., -0.5], [k.b / 2., p.largura + 0.5], false);
            let sk = m.sketch("Rasgo_cubo", plano, data);
            crate::eixos::extrusao_cega(&mut m, "Rasgo_chaveta", sk, "kt2+1", BoolOp::Cut, true);
        }
    }
    Ok(m.doc)
}

/// Passo máximo de torção entre seções do loft da helicoidal (graus).
const PASSO_TORCAO: f64 = 4.;
/// Máximo de trechos do loft.
const MAX_TRECHOS: usize = 24;
/// Pontos por flanco: mais dentes, menos pontos.
fn pontos_flanco(z: u32) -> usize {
    if z > 60 {
        6
    } else {
        8
    }
}

impl Montador {
    fn parametros(&mut self, p: &ParamEngrenagem) -> Result<(), String> {
        let (n, a) = (Unit::Numero, Unit::Angulo);
        let def = |m: &mut Self, nome: &str, expr: String, u: Unit| m.doc.add_param(nome, &expr, u).map(|_| ()).map_err(|e| e.to_string());
        def(self, "m", p.modulo.to_string(), Unit::Comprimento)?;
        if p.tipo != TipoEngrenagem::Cremalheira {
            def(self, "z", p.dentes.to_string(), n)?;
        }
        def(self, "alpha", p.angulo_pressao.to_string(), a)?;
        if p.tipo == TipoEngrenagem::Helicoidal {
            def(self, "beta", p.angulo_helice.to_string(), a)?;
        }
        def(self, "b", p.largura.to_string(), Unit::Comprimento)?;
        def(self, "folga", p.folga_dentes.to_string(), Unit::Comprimento)?;
        match p.tipo {
            TipoEngrenagem::Reta | TipoEngrenagem::Helicoidal => {
                def(self, "x", p.coef_correcao.to_string(), n)?;
                def(self, "furo", p.furo.to_string(), Unit::Comprimento)?;
                let helicoidal = p.tipo == TipoEngrenagem::Helicoidal;
                def(self, "d", if helicoidal { "m*z/cos(beta)".into() } else { "m*z".into() }, Unit::Comprimento)?;
                let alfa_t = if helicoidal { "atan(tan(alpha)/cos(beta))" } else { "alpha" };
                def(self, "db", format!("d*cos({alfa_t})"), Unit::Comprimento)?;
                def(self, "da", "d+2*m*(1+x)".into(), Unit::Comprimento)?;
                def(self, "df", "d-2*m*(1.25-x)".into(), Unit::Comprimento)?;
            }
            TipoEngrenagem::Interna => {
                def(self, "d", "m*z".into(), Unit::Comprimento)?;
                def(self, "db", "d*cos(alpha)".into(), Unit::Comprimento)?;
                def(self, "da", "d-2*m".into(), Unit::Comprimento)?;
                def(self, "df", "d+2.5*m".into(), Unit::Comprimento)?;
                def(self, "Dext", "df+5*m".into(), Unit::Comprimento)?;
            }
            TipoEngrenagem::Cremalheira => {
                def(self, "L", p.comprimento_cremalheira.to_string(), Unit::Comprimento)?;
                def(self, "passo", "pi*m".into(), Unit::Comprimento)?;
            }
        }
        Ok(())
    }

    /// Sketch com o contorno (e o círculo externo, se `anel`): `rot` gira o desenho em torno de Z.
    fn sketch_contorno(&mut self, nome: &str, plano: PlaneRef, inicio: [f64; 2], passos: &[Passo], rot: f64, anel: Option<f64>) -> Id {
        let (c, s) = (rot.cos(), rot.sin());
        let rotar = |p: [f64; 2]| [p[0] * c - p[1] * s, p[0] * s + p[1] * c];
        let mut data = SketchData::default();
        let mut al = || self.doc.alloc();
        let centro = data.add_point(&mut al, 0., 0.);
        if let Some(d) = anel {
            data.add_curve(&mut al, Curve::Circle { c: centro, r: d / 2. }, false);
        }
        let q = rotar(inicio);
        let primeiro = data.add_point(&mut al, q[0], q[1]);
        let mut atual = primeiro;
        for (i, passo) in passos.iter().enumerate() {
            // o último passo fecha o contorno no ponto inicial
            let ultimo = i + 1 == passos.len();
            match passo {
                Passo::Linha(f) | Passo::Arco(f) => {
                    let fim = if ultimo {
                        primeiro
                    } else {
                        let q = rotar(*f);
                        data.add_point(&mut al, q[0], q[1])
                    };
                    let geom = if matches!(passo, Passo::Linha(_)) { Curve::Line { a: atual, b: fim } } else { Curve::Arc { c: centro, s: atual, e: fim } };
                    data.add_curve(&mut al, geom, false);
                    atual = fim;
                }
                Passo::Spline(v) => {
                    let mut pts = vec![atual];
                    for (j, f) in v.iter().enumerate() {
                        pts.push(if ultimo && j + 1 == v.len() {
                            primeiro
                        } else {
                            let q = rotar(*f);
                            data.add_point(&mut al, q[0], q[1])
                        });
                    }
                    atual = *pts.last().unwrap_or(&atual);
                    data.add_curve(&mut al, Curve::Spline { pts }, false);
                }
            }
        }
        self.sketch(nome, plano, data)
    }

    pub(crate) fn sketch(&mut self, nome: &str, plano: PlaneRef, data: SketchData) -> Id {
        let id = self.doc.add_feature(FeatureKind::Sketch(Sketch { plane: plano, data }));
        if let Some(f) = self.doc.features.iter_mut().find(|f| f.id == id) {
            f.name = nome.into();
        }
        id
    }

    fn extrusao(&mut self, nome: &str, sketch: Id) {
        let id = self.doc.add_feature(FeatureKind::Extrude(Extrude {
            sketch,
            op: BoolOp::Add,
            extent: Extent::Blind("b".into()),
            reverse: false,
            taper: "0".into(),
            thin: None,
            thin_side: 0,
            dir: None,
        }));
        if let Some(f) = self.doc.features.iter_mut().find(|f| f.id == id) {
            f.name = nome.into();
        }
    }

    fn reta(&mut self, g: &Dente) -> Result<(), String> {
        let (inicio, passos) = g.contorno(pontos_flanco(g.z))?;
        let xy = PlaneRef::Origin(OriginPlane::XY);
        let sk = self.sketch_contorno("Perfil_dentes", xy, inicio, &passos, 0., None);
        self.extrusao("Dentes", sk);
        Ok(())
    }

    fn interna(&mut self, g: &Dente, anel: f64) -> Result<(), String> {
        let (inicio, passos) = g.contorno(pontos_flanco(g.z))?;
        let xy = PlaneRef::Origin(OriginPlane::XY);
        let sk = self.sketch_contorno("Perfil_anel", xy, inicio, &passos, 0., Some(anel));
        self.extrusao("Anel", sk);
        Ok(())
    }

    /// Loft reto entre seções giradas de `z·tan β / r`.
    fn helicoidal(&mut self, g: &Dente, beta: f64, largura: f64) -> Result<(), String> {
        let (inicio, passos) = g.contorno(pontos_flanco(g.z))?;
        let torcao = (largura * beta.tan() / g.r).abs().to_degrees();
        let trechos = ((torcao / PASSO_TORCAO).ceil() as usize).clamp(1, MAX_TRECHOS);
        let mut secoes = vec![];
        for i in 0..=trechos {
            let plano = if i == 0 { PlaneRef::Origin(OriginPlane::XY) } else { self.plano(&format!("Plano_secao_{i}"), &format!("b*{i}/{trechos}"))? };
            let rot = largura * (i as f64 / trechos as f64) * beta.tan() / g.r;
            secoes.push(self.sketch_contorno(&format!("Secao_{i}"), plano, inicio, &passos, rot, None));
        }
        let id = self.doc.add_feature(FeatureKind::Loft(Loft { sections: secoes, ruled: true, closed: false, op: BoolOp::Add }));
        if let Some(f) = self.doc.features.iter_mut().find(|f| f.id == id) {
            f.name = "Dentes_helicoidais".into();
        }
        Ok(())
    }

    /// Barra com dentes trapezoidais: topo em y = m, raiz em y = −1,25m, fundo em y = −3,75m.
    fn cremalheira(&mut self, p: &ParamEngrenagem) -> Result<(), String> {
        let (m, l) = (p.modulo, p.comprimento_cremalheira);
        let passo = PI * m;
        let n = (l / passo).floor() as usize;
        let t = p.angulo_pressao.to_radians().tan();
        let s = passo / 2. - p.folga_dentes;
        let (yt, yr, yb) = (m, -1.25 * m, -3.75 * m);
        let (wt, wr) = (s / 2. - m * t, s / 2. + 1.25 * m * t);
        let c0 = (l - (n as f64 - 1.) * passo) / 2.;
        let mut passos = vec![Passo::Linha([l, yb]), Passo::Linha([l, yr])];
        for k in (0..n).rev() {
            let cx = c0 + k as f64 * passo;
            passos.extend([[cx + wr, yr], [cx + wt, yt], [cx - wt, yt], [cx - wr, yr]].map(Passo::Linha));
        }
        passos.extend([[0., yr], [0., yb]].map(Passo::Linha));
        let xy = PlaneRef::Origin(OriginPlane::XY);
        let sk = self.sketch_contorno("Perfil_dentes", xy, [0., yb], &passos, 0., None);
        self.extrusao("Barra", sk);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{recompute, Status};

    /// Recompute com todos os comandos em Ok; devolve (volume, mín, máx) do corpo único.
    fn medir(doc: &Document) -> (f64, [f64; 3], [f64; 3]) {
        let r = recompute(doc);
        assert!(r.param_error.is_none(), "{:?}", r.param_error);
        for f in &doc.features {
            assert!(matches!(r.status[&f.id], Status::Ok | Status::Warning(_)), "{} / {}: {:?}", doc.name, f.name, r.status[&f.id]);
        }
        assert_eq!(r.bodies.len(), 1, "{}", doc.name);
        let b = &r.bodies[0];
        let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
        for q in b.mesh.positions.chunks_exact(3) {
            for i in 0..3 {
                lo[i] = lo[i].min(q[i] as f64);
                hi[i] = hi[i].max(q[i] as f64);
            }
        }
        (b.mass.volume, lo, hi)
    }

    fn perto(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    #[test]
    fn calculo_m2_z20() {
        let c = calculo(&ParamEngrenagem::default()).unwrap();
        assert!(perto(c.diametro_primitivo, 40., 1e-9) && perto(c.diametro_externo, 44., 1e-9) && perto(c.diametro_raiz, 35., 1e-9));
        assert!(perto(c.diametro_base, 40. * 20_f64.to_radians().cos(), 1e-9) && perto(c.diametro_base, 37.59, 0.01));
        assert!(perto(c.passo, 2. * PI, 1e-9) && perto(c.espessura_dente, PI, 1e-9));
        assert!(c.avisos.is_empty(), "{:?}", c.avisos);
    }

    #[test]
    fn calculo_helicoidal_correcao_e_avisos() {
        let h = ParamEngrenagem { tipo: TipoEngrenagem::Helicoidal, angulo_helice: 20., ..Default::default() };
        let c = calculo(&h).unwrap();
        assert!(perto(c.diametro_primitivo, 40. / 20_f64.to_radians().cos(), 1e-9) && perto(c.diametro_externo - c.diametro_primitivo, 4., 1e-9));
        assert!(c.angulo_pressao_transversal > 20.);
        // z pequeno sem correção avisa; com correção suficiente não
        let pouco = ParamEngrenagem { dentes: 10, ..Default::default() };
        assert!(!calculo(&pouco).unwrap().avisos.is_empty());
        assert!(calculo(&ParamEngrenagem { coef_correcao: 0.5, ..pouco }).unwrap().avisos.iter().all(|a| !a.contains("rebaixo")));
        // x desloca topo e raiz
        let c = calculo(&ParamEngrenagem { coef_correcao: 0.5, ..Default::default() }).unwrap();
        assert!(perto(c.diametro_externo, 40. + 2. * 2. * 1.5, 1e-9) && perto(c.diametro_raiz, 40. - 2. * 2. * 0.75, 1e-9));
        // interna e cremalheira
        let i = calculo(&ParamEngrenagem { tipo: TipoEngrenagem::Interna, dentes: 60, ..Default::default() }).unwrap();
        assert!(perto(i.diametro_primitivo, 120., 1e-9) && perto(i.diametro_externo, 116., 1e-9) && perto(i.diametro_raiz, 125., 1e-9) && perto(i.diametro_anel, 135., 1e-9));
        let r = calculo(&ParamEngrenagem { tipo: TipoEngrenagem::Cremalheira, ..Default::default() }).unwrap();
        assert_eq!(r.dentes_cremalheira, 15);
    }

    #[test]
    fn entradas_invalidas() {
        let base = ParamEngrenagem::default();
        assert!(calculo(&ParamEngrenagem { dentes: 5, ..base.clone() }).is_err());
        assert!(calculo(&ParamEngrenagem { tipo: TipoEngrenagem::Interna, dentes: 11, ..base.clone() }).is_err());
        assert!(calculo(&ParamEngrenagem { modulo: 0., ..base.clone() }).is_err());
        assert!(calculo(&ParamEngrenagem { modulo: f64::NAN, ..base.clone() }).is_err());
        assert!(calculo(&ParamEngrenagem { angulo_pressao: 10., ..base.clone() }).is_err());
        assert!(calculo(&ParamEngrenagem { angulo_pressao: 35., ..base.clone() }).is_err());
        assert!(calculo(&ParamEngrenagem { largura: 0., ..base.clone() }).is_err());
        assert!(calculo(&ParamEngrenagem { furo: 40., ..base.clone() }).is_err());
        assert!(calculo(&ParamEngrenagem { tipo: TipoEngrenagem::Cremalheira, comprimento_cremalheira: 5., ..base.clone() }).is_err());
        assert!(engrenagem_documento(&ParamEngrenagem { dentes: 3, ..base }).is_err());
    }

    #[test]
    fn descricao_e_parametros() {
        let d = engrenagem_documento(&ParamEngrenagem { dentes: 24, ..Default::default() }).unwrap();
        assert_eq!(d.props.description, "Engrenagem reta m2 z24");
        assert!(d.props.pn.is_empty());
        for n in ["m", "z", "alpha", "b", "x", "d", "da", "df"] {
            assert!(d.params.iter().any(|p| p.name == n), "{n}");
        }
        let c = engrenagem_documento(&ParamEngrenagem { tipo: TipoEngrenagem::Cremalheira, ..Default::default() }).unwrap();
        assert_eq!(c.props.description, "Cremalheira m2 L100");
        assert_ne!(ParamEngrenagem::default().chave(), ParamEngrenagem { furo: 8., ..Default::default() }.chave());
    }

    #[test]
    fn reta_recalcula_e_o_volume_fica_entre_raiz_e_topo() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let d = engrenagem_documento(&ParamEngrenagem::default()).unwrap();
        let (v, lo, hi) = medir(&d);
        let (v_raiz, v_topo) = (PI * 17.5 * 17.5 * 10., PI * 22. * 22. * 10.);
        assert!(v > v_raiz && v < v_topo, "volume {v} fora de {v_raiz}..{v_topo}");
        assert!(perto(lo[2], 0., 1e-3) && perto(hi[2], 10., 1e-3), "z {lo:?} {hi:?}");
        assert!(hi[0] > 21.9 && hi[0] < 22.01 && hi[1] < 22.01, "extensão {lo:?} {hi:?}");
        // paramétrico: a largura b
        let mut d = d;
        d.params.iter_mut().find(|p| p.name == "b").unwrap().expr = "20".into();
        let (v2, _, hi) = medir(&d);
        assert!(perto(hi[2], 20., 1e-3) && perto(v2, 2. * v, v * 1e-3));
        // furo central
        let (vf, _, _) = medir(&engrenagem_documento(&ParamEngrenagem { furo: 10., ..Default::default() }).unwrap());
        assert!(perto(vf, v - PI * 25. * 10., v * 0.01), "furo {vf} vs {}", v - PI * 250.);
    }

    #[test]
    fn rasgo_do_cubo_recalcula_e_valida() {
        let base = ParamEngrenagem { furo: 20., rasgo_cubo: true, ..Default::default() };
        assert_ne!(base.chave(), ParamEngrenagem { rasgo_cubo: false, ..base.clone() }.chave());
        // furo fora da tabela (Ø5) e rasgo que passa da raiz
        assert!(calculo(&ParamEngrenagem { furo: 5., ..base.clone() }).is_err());
        assert!(calculo(&ParamEngrenagem { furo: 28., ..base.clone() }).is_err());
        // sem furo o rasgo é ignorado
        assert!(calculo(&ParamEngrenagem { furo: 0., ..base.clone() }).is_ok());
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (v_furo, _, _) = medir(&engrenagem_documento(&ParamEngrenagem { rasgo_cubo: false, ..base.clone() }).unwrap());
        let (v, _, hi) = medir(&engrenagem_documento(&base).unwrap());
        // rasgo de 6 mm de largura, do arco do furo (x ≈ 9,54) até x = 12,8, ao longo dos 10 mm de largura
        let cortado = v_furo - v;        assert!(cortado > 6. * 2.8 * 10. && cortado < 6. * 3.5 * 10., "cortou {cortado}");
        assert!(hi[2] > 9.99);
        let h = ParamEngrenagem { tipo: TipoEngrenagem::Helicoidal, angulo_helice: 15., ..base };
        medir(&engrenagem_documento(&h).unwrap());
    }

    #[test]
    fn helicoidal_interna_e_cremalheira_recalculam() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        // helicoidal: mesmo volume da reta de mesma seção transversal é próximo e a altura é a largura
        let h = ParamEngrenagem { tipo: TipoEngrenagem::Helicoidal, angulo_helice: 20., largura: 20., ..Default::default() };
        let (v, lo, hi) = medir(&engrenagem_documento(&h).unwrap());
        let c = calculo(&h).unwrap();
        let (v_raiz, v_topo) = (PI * (c.diametro_raiz / 2.).powi(2) * 20., PI * (c.diametro_externo / 2.).powi(2) * 20.);
        assert!(v > v_raiz && v < v_topo, "helicoidal {v}");
        assert!(perto(lo[2], 0., 1e-3) && perto(hi[2], 20., 1e-3));
        // sentido contrário também gera
        medir(&engrenagem_documento(&ParamEngrenagem { angulo_helice: -20., ..h }).unwrap());

        // interna z60: anel de Ø135 com vão de Ø116 a Ø125
        let i = ParamEngrenagem { tipo: TipoEngrenagem::Interna, dentes: 60, ..Default::default() };
        let (v, _, hi) = medir(&engrenagem_documento(&i).unwrap());
        let (v_min, v_max) = (PI * (67.5_f64.powi(2) - 62.5 * 62.5) * 10., PI * (67.5_f64.powi(2) - 58. * 58.) * 10.);
        assert!(v > v_min && v < v_max, "interna {v} fora de {v_min}..{v_max}");
        assert!(perto(hi[0], 67.5, 0.05));

        // cremalheira de 100 mm: base de 5·m = 10 abaixo da raiz e 15 dentes
        let r = ParamEngrenagem { tipo: TipoEngrenagem::Cremalheira, ..Default::default() };
        let (v, lo, hi) = medir(&engrenagem_documento(&r).unwrap());
        assert!(perto(lo[0], 0., 1e-3) && perto(hi[0], 100., 1e-3) && perto(lo[1], -7.5, 1e-3) && perto(hi[1], 2., 1e-3), "{lo:?} {hi:?}");
        assert!(v > 100. * 5. * 10. && v < 100. * 9.5 * 10., "cremalheira {v}");
    }
}
