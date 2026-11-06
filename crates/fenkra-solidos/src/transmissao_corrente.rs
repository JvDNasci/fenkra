// SPDX-License-Identifier: LGPL-2.1-or-later
//! Geometria de corrente, coroa/pinhão, correia e polias (B342).
//!
//! - **Roda de corrente** (coroa ou pinhão, ver [`roda_documento`]): disco com um vão em arco para
//!   cada rolo (raio de assento `0,505·d1`, ângulo de assento `140° − 90°/N`, valores de referência
//!   de projeto), flancos retos até o topo `Da = p (0,6 + cot(180°/N))`. É um perfil **simplificado**,
//!   sem o raio de pé e sem a curva de entrada que as normas de roda trazem: serve ao desenho, ao
//!   engrenamento visual e à montagem, e leva o rótulo de cálculo preliminar.
//! - **Caminho** ([`Caminho`]): o laço da corrente/correia nas duas rodas (duas retas tangentes e dois
//!   arcos), com o ponto e a direção a cada comprimento de arco: é a base do desenho dos elos e do
//!   movimento (os elos andam por esse caminho).
//! - **Corrente 3D** ([`corrente_documento`]): rolos em círculos extrudados e duas fitas de placas
//!   (anéis), uma de cada lado, num corpo só.
//! - **Correia 3D** ([`correia_documento`]) e **polias** ([`polia_documento`]): anel com a espessura da
//!   correia; polia em V por revolução, polia dentada sem perfil de dente (a altura do dente da
//!   correia não tem duas fontes), lisa no diâmetro primitivo, com flanges.
//!
//! Eixo das rodas e polias: Z, face em `z = 0`. O caminho fica no plano XY, com a roda menor na
//! origem e a maior em `(C, 0)`.

use crate::engrenagens::{Passo, polar};
use crate::fixadores::Montador;
use crate::transmissao_dados::{diametro_primitivo_roda, diametro_topo_roda, Corrente};
use fenkra_core::sketch::Curve;
use fenkra_core::*;
use std::f64::consts::PI;

/// Parâmetros de uma roda de corrente (coroa ou pinhão).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ParamRoda {
    pub corrente: String,
    pub dentes: u32,
    pub furo: f64,
    /// usinagens editáveis sobre a roda padronizada (B345)
    #[serde(default)]
    pub usinagens: Vec<crate::usinagens::Usinagem>,
}

impl ParamRoda {
    pub fn nova(corrente: &str, dentes: u32, furo: f64) -> ParamRoda {
        ParamRoda { corrente: corrente.into(), dentes, furo, usinagens: vec![] }
    }

    /// Chave da roda padronizada do catálogo.
    pub fn chave(&self) -> String {
        format!("roda-{}-z{}-f{}", self.corrente.replace(' ', ""), self.dentes, self.furo)
    }

    /// Chave do arquivo, com o sufixo das usinagens.
    pub fn chave_arquivo(&self) -> String {
        format!("{}{}", self.chave(), crate::usinagens::sufixo_de_chave(&self.usinagens))
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CalculoRoda {
    pub diametro_primitivo: f64,
    pub diametro_topo: f64,
    pub diametro_raiz: f64,
    pub largura_dente: f64,
    pub diametro_cubo: f64,
}

fn dados(corr: &str) -> Result<&'static Corrente, String> {
    let c = crate::transmissao_dados::corrente(corr).ok_or_else(|| format!("corrente '{corr}' fora da biblioteca"))?;
    if !c.liberada() {
        return Err(format!("{}: dado {}: a geração só usa linhas conferidas em duas fontes", c.designacao, c.conferencia.rotulo()));
    }
    Ok(c)
}

pub fn calculo_roda(p: &ParamRoda) -> Result<CalculoRoda, String> {
    let c = dados(&p.corrente)?;
    if p.dentes < 9 || p.dentes > 120 {
        return Err("a roda precisa de 9 a 120 dentes".into());
    }
    if !p.furo.is_finite() || p.furo < 0. {
        return Err("furo inválido".into());
    }
    let d = diametro_primitivo_roda(c.passo, p.dentes);
    let raiz = d - c.rolo * 1.01;
    let cubo = (p.furo * 1.8).max(p.furo + 6.).min(raiz * 0.8);
    if p.furo > 0. && p.furo >= cubo - 2. {
        return Err(format!("o furo precisa ser menor que {:.1} mm nesta roda", cubo - 2.));
    }
    let hub = (p.furo * 0.8).max(4.);
    crate::usinagens::validar(&crate::usinagens::GeoBase { largura: 0.9 * c.largura, raio_raiz: raiz / 2., furo: p.furo, cubo_proprio: Some((hub, cubo)) }, &p.usinagens)?;
    Ok(CalculoRoda { diametro_primitivo: d, diametro_topo: diametro_topo_roda(c.passo, p.dentes), diametro_raiz: raiz, largura_dente: 0.9 * c.largura, diametro_cubo: cubo })
}

/// Contorno fechado da roda (CCW), dente a dente, centrado na origem; o vão 0 em +X.
fn passos_roda(c: &Corrente, n: u32) -> Result<([f64; 2], Vec<Passo>), String> {
    let nn = n as f64;
    let rp = diametro_primitivo_roda(c.passo, n) / 2.;
    let ra = diametro_topo_roda(c.passo, n) / 2.;
    let rs = 0.505 * c.rolo;
    let u0 = (140. - 90. / nn).to_radians() / 2.;
    let passo_ang = 2. * PI / nn;
    // ponto do arco de assento do vão em `fi`, parâmetro u (−u0..u0 de lado a lado)
    let assento = |fi: f64, u: f64| {
        let c0 = polar(rp, fi);
        let a = fi + PI + u;
        [c0[0] + rs * a.cos(), c0[1] + rs * a.sin()]
    };
    let ang = |q: [f64; 2]| q[1].atan2(q[0]);
    // qual extremo do arco fica do lado CCW do vão
    let (lado_ccw, lado_cw) = if ang(assento(0., u0)) > ang(assento(0., -u0)) { (u0, -u0) } else { (-u0, u0) };
    let sinal = if lado_ccw > 0. { 1. } else { -1. };
    let fim_ccw = assento(0., lado_ccw);
    let meia_ponta = 0.06 * c.passo / ra;
    if ang(fim_ccw) >= passo_ang / 2. - meia_ponta || fim_ccw[0].hypot(fim_ccw[1]) >= ra - 0.05 * c.passo {
        return Err("o perfil simplificado não fecha para esta combinação de corrente e dentes".into());
    }
    let inicio = assento(0., lado_cw);
    let mut passos = vec![];
    let n_arco = 6usize;
    for k in 0..n {
        let fi = passo_ang * k as f64;
        // arco de assento do vão k, do lado CW ao CCW
        let pts: Vec<[f64; 2]> = (1..=n_arco).map(|i| assento(fi, sinal * (-u0 + 2. * u0 * i as f64 / n_arco as f64))).collect();
        passos.push(Passo::Spline(pts));
        // flanco reto até a ponta do dente seguinte, ponta em arco, flanco reto até o vão k+1
        let centro_dente = fi + passo_ang / 2.;
        passos.push(Passo::Linha(polar(ra, centro_dente - meia_ponta)));
        passos.push(Passo::Arco(polar(ra, centro_dente + meia_ponta)));
        passos.push(Passo::Linha(assento(fi + passo_ang, lado_cw)));
    }
    Ok((inicio, passos))
}

/// Documento da roda de corrente: dentes extrudados pela largura do dente, cubo e furo.
pub fn roda_documento(p: &ParamRoda) -> Result<Document, String> {
    let c = dados(&p.corrente)?;
    let k = calculo_roda(p)?;
    let mut m = Montador::novo(&format!("Roda {} z{}", c.designacao, p.dentes));
    m.doc.props.material = "Aço 1045".into();
    m.doc.props.part_type = PartType::Fabricada;
    m.param("p", &c.passo.to_string())?;
    m.param("d1", &c.rolo.to_string())?;
    m.param("w", &k.largura_dente.to_string())?;
    m.param("furo", &p.furo.to_string())?;
    m.param("dcubo", &k.diametro_cubo.to_string())?;
    m.doc.add_param("N", &p.dentes.to_string(), Unit::Numero).map_err(|e| e.to_string())?;
    let (inicio, passos) = passos_roda(c, p.dentes)?;
    let sk = m.sketch_contorno("Perfil_dentes", PlaneRef::Origin(OriginPlane::XY), inicio, &passos, 0., None);
    let id = m.doc.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind("w".into()), reverse: false, taper: "0".into(), thin: None, thin_side: 0, dir: None, ends: Default::default() }));
    if let Some(f) = m.doc.features.iter_mut().find(|f| f.id == id) {
        f.name = "Dentes".into();
    }
    let hub = (p.furo * 0.8).max(4.);
    let plano_w = m.plano("Plano_cubo", "w")?;
    m.prim("Cubo", PrimKind::Cilindro, plano_w, ["dcubo/2", &hub.to_string(), "0", "0"], BoolOp::Add)?;
    // o cubo precisa tocar o disco: um pequeno trecho dentro
    let plano_in = m.plano("Plano_cubo_base", "w-0.5")?;
    m.prim("Cubo_base", PrimKind::Cilindro, plano_in, ["dcubo/2", "0.5", "0", "0"], BoolOp::Add)?;
    if p.furo > 0. {
        let plano = m.plano("Plano_furo", "-1")?;
        m.prim("Furo", PrimKind::Cilindro, plano, ["furo/2", &(k.largura_dente + hub + 2.).to_string(), "0", "0"], BoolOp::Cut)?;
    }
    if !p.usinagens.is_empty() {
        crate::usinagens::aplicar(&mut m, &crate::usinagens::GeoBase { largura: k.largura_dente, raio_raiz: k.diametro_raiz / 2., furo: p.furo, cubo_proprio: Some((hub, k.diametro_cubo)) }, &p.usinagens)?;
    }
    crate::usinagens::registrar_vinculo(&mut m.doc, &p.chave(), &p.usinagens);
    Ok(m.doc)
}

/// Eixo (pino) cilíndrico de diâmetro `furo` e comprimento `comprimento`, de `z = 0` a `z = comprimento`:
/// o apoio fixo de cada peça girante da transmissão.
pub fn pino_documento(nome: &str, furo: f64, comprimento: f64) -> Result<Document, String> {
    if !(furo > 0. && comprimento > 0. && furo.is_finite() && comprimento.is_finite()) {
        return Err("eixo: diâmetro e comprimento precisam ser positivos".into());
    }
    let mut m = Montador::novo(nome);
    m.doc.props.part_type = PartType::Fabricada;
    m.doc.props.material = "Aço 1045".into();
    m.param("d", &furo.to_string())?;
    m.param("L", &comprimento.to_string())?;
    m.prim("Eixo", PrimKind::Cilindro, PlaneRef::Origin(OriginPlane::XY), ["d/2", "L", "0", "0"], BoolOp::Add)?;
    Ok(m.doc)
}

// ---------------------------------------------------------------- caminho

/// Laço de duas rodas: a menor na origem (raio `r1`), a maior em `(c, 0)` (raio `r2 >= r1`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Caminho {
    pub r1: f64,
    pub r2: f64,
    pub c: f64,
}

/// Posição e direção de um ponto do caminho.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PontoCaminho {
    pub x: f64,
    pub y: f64,
    /// direção da tangente (rad), no sentido do percurso (anti-horário ao redor das rodas)
    pub direcao: f64,
}

impl Caminho {
    pub fn novo(r1: f64, r2: f64, c: f64) -> Result<Caminho, String> {
        let (r1, r2) = (r1.min(r2), r1.max(r2));
        if !(r1 > 0. && c.is_finite()) || c <= r2 - r1 + 1e-6 {
            return Err("a distância entre centros precisa passar da diferença dos raios".into());
        }
        Ok(Caminho { r1, r2, c })
    }

    /// Ângulo de inclinação das tangentes externas em relação ao eixo dos centros.
    fn gama(&self) -> f64 {
        ((self.r2 - self.r1) / self.c).asin()
    }

    fn reta(&self) -> f64 {
        (self.c * self.c - (self.r2 - self.r1).powi(2)).sqrt()
    }

    pub fn comprimento(&self) -> f64 {
        let g = self.gama();
        2. * self.reta() + self.r1 * (PI - 2. * g) + self.r2 * (PI + 2. * g)
    }

    /// Ponto a `s` mm do início (o início é o ponto de tangência inferior da roda menor, a
    /// percorrer a reta de baixo para a roda maior), em volta cíclica.
    pub fn ponto(&self, s: f64) -> PontoCaminho {
        let g = self.gama();
        let (rt, l) = (self.reta(), self.comprimento());
        let s = s.rem_euclid(l);
        // reta de baixo: da menor (ângulo −π/2 + g no círculo 1) à maior; direção g
        let o1 = [0., 0.];
        let o2 = [self.c, 0.];
        // normal externa da tangente de baixo: n·(o2 − o1) = r1 − r2, ou seja, cos φ = −ΔR / c
        let n_baixo = -PI / 2. - g;
        let a1 = [o1[0] + self.r1 * n_baixo.cos(), o1[1] + self.r1 * n_baixo.sin()];
        if s < rt {
            // a reta de baixo desce de leve da roda menor para a maior: direção −g
            return PontoCaminho { x: a1[0] + s * (-g).cos(), y: a1[1] + s * (-g).sin(), direcao: -g };
        }
        let s = s - rt;
        // arco na roda maior, CCW, da normal de baixo à de cima: π + 2g
        let arco2 = self.r2 * (PI + 2. * g);
        if s < arco2 {
            let a = n_baixo + s / self.r2;
            return PontoCaminho { x: o2[0] + self.r2 * a.cos(), y: o2[1] + self.r2 * a.sin(), direcao: a + PI / 2. };
        }
        let s = s - arco2;
        // reta de cima, de volta para a menor: direção π + g
        let n_cima = PI / 2. + g;
        let b2 = [o2[0] + self.r2 * n_cima.cos(), o2[1] + self.r2 * n_cima.sin()];
        let d = PI + g;
        if s < rt {
            return PontoCaminho { x: b2[0] + s * d.cos(), y: b2[1] + s * d.sin(), direcao: d };
        }
        let s = s - rt;
        let a = n_cima + s / self.r1;
        PontoCaminho { x: o1[0] + self.r1 * a.cos(), y: o1[1] + self.r1 * a.sin(), direcao: a + PI / 2. }
    }

    /// Centros dos `n` elos (rolos), a espaçamento igual no comprimento do laço e deslocados de
    /// `fase` mm: é o desenho do movimento (a fase cresce com a rotação da roda motora).
    pub fn elos(&self, n: usize, fase: f64) -> Vec<PontoCaminho> {
        let passo = self.comprimento() / n as f64;
        (0..n).map(|i| self.ponto(i as f64 * passo + fase)).collect()
    }
}

/// Avanço do laço (mm) para um giro de `graus` da roda de raio primitivo `r`: a corrente anda o
/// arco do primitivo.
pub fn avanco_da_corrente(r: f64, graus: f64) -> f64 {
    r * graus.to_radians()
}

// ---------------------------------------------------------------- sólidos do laço

/// Laço (estádio) deslocado de `h` para fora, como contorno fechado no sketch (linhas e arcos).
fn laco(m: &mut Montador, data: &mut SketchData, cam: &Caminho, h: f64) {
    let g = cam.gama();
    let (ra, rb) = (cam.r1 + h, cam.r2 + h);
    let mut al = || m.doc.alloc();
    let c1 = data.add_point(&mut al, 0., 0.);
    let c2 = data.add_point(&mut al, cam.c, 0.);
    let nb = -PI / 2. - g;
    let nc = PI / 2. + g;
    let p1b = data.add_point(&mut al, ra * nb.cos(), ra * nb.sin());
    let p2b = data.add_point(&mut al, cam.c + rb * nb.cos(), rb * nb.sin());
    let p2c = data.add_point(&mut al, cam.c + rb * nc.cos(), rb * nc.sin());
    let p1c = data.add_point(&mut al, ra * nc.cos(), ra * nc.sin());
    data.add_curve(&mut al, Curve::Line { a: p1b, b: p2b }, false);
    data.add_curve(&mut al, Curve::Arc { c: c2, s: p2b, e: p2c }, false);
    data.add_curve(&mut al, Curve::Line { a: p2c, b: p1c }, false);
    data.add_curve(&mut al, Curve::Arc { c: c1, s: p1c, e: p1b }, false);
}

fn nomear(m: &mut Montador, id: Id, nome: &str) {
    if let Some(f) = m.doc.features.iter_mut().find(|f| f.id == id) {
        f.name = nome.into();
    }
}

fn extrusao_de(m: &mut Montador, nome: &str, sk: Id, prof: &str) {
    let id = m.doc.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind(prof.into()), reverse: false, taper: "0".into(), thin: None, thin_side: 0, dir: None, ends: Default::default() }));
    nomear(m, id, nome);
}

/// Parâmetros do desenho da corrente.
#[derive(Clone, Debug, PartialEq)]
pub struct ParamLacoCorrente {
    pub corrente: String,
    pub dentes1: u32,
    pub dentes2: u32,
    pub elos: u32,
    /// distância entre centros (a corrigida para `elos`)
    pub centros: f64,
}

/// Corrente 3D: `elos` rolos (círculos extrudados entre as placas) e duas fitas de placas ao longo do
/// laço, `z = 0` a `largura` entre elas. Num corpo só.
pub fn corrente_documento(p: &ParamLacoCorrente) -> Result<Document, String> {
    let c = dados(&p.corrente)?;
    let (d1, d2) = (diametro_primitivo_roda(c.passo, p.dentes1), diametro_primitivo_roda(c.passo, p.dentes2));
    let cam = Caminho::novo(d1 / 2., d2 / 2., p.centros)?;
    if p.elos < 20 || p.elos > 400 {
        return Err("a corrente precisa de 20 a 400 elos".into());
    }
    let mut m = Montador::novo(&format!("Corrente {} {} elos", c.designacao, p.elos));
    m.doc.props.material = "Aço 1045".into();
    m.doc.props.part_type = PartType::Comprada;
    m.param("p", &c.passo.to_string())?;
    m.param("b1", &c.largura.to_string())?;
    let h = 0.42 * c.rolo; // meia altura da placa
    let tp = (0.12 * c.passo).max(0.6);
    m.param("tp", &tp.to_string())?;
    // rolos: círculos nos centros dos elos
    let pontos = cam.elos(p.elos as usize, 0.);
    let mut data = SketchData::default();
    {
        let mut al = || m.doc.alloc();
        for q in &pontos {
            let ctr = data.add_point(&mut al, q.x, q.y);
            data.add_curve(&mut al, Curve::Circle { c: ctr, r: c.rolo / 2. }, false);
        }
    }
    let sk = m.sketch("Rolos", PlaneRef::Origin(OriginPlane::XY), data);
    extrusao_de(&mut m, "Rolos", sk, "b1");
    // as duas fitas de placas: anel entre h dentro e h fora do laço
    for (nome, desvio) in [("Placas_a", "-tp"), ("Placas_b", "b1")] {
        let plano = m.plano(&format!("Plano_{nome}"), desvio)?;
        let mut data = SketchData::default();
        laco(&mut m, &mut data, &cam, h);
        laco(&mut m, &mut data, &cam, -h);
        let sk = m.sketch(nome, plano, data);
        extrusao_de(&mut m, nome, sk, "tp");
    }
    Ok(m.doc)
}

/// Parâmetros do desenho da correia (laço fechado de espessura `espessura` e largura `largura`).
#[derive(Clone, Debug, PartialEq)]
pub struct ParamLacoCorreia {
    pub d1: f64,
    pub d2: f64,
    pub centros: f64,
    pub espessura: f64,
    pub largura: f64,
}

/// Correia 3D: anel com a face interna sobre as polias (raios externos das polias) e a espessura
/// dada, extrudado pela largura. Perfil liso: não desenha os dentes nem a seção em V.
pub fn correia_documento(p: &ParamLacoCorreia) -> Result<Document, String> {
    if !(p.espessura > 0. && p.largura > 0.) {
        return Err("espessura e largura da correia precisam ser positivas".into());
    }
    let cam = Caminho::novo(p.d1 / 2., p.d2 / 2., p.centros)?;
    let mut m = Montador::novo("Correia");
    m.doc.props.material = "Borracha".into();
    m.doc.props.part_type = PartType::Comprada;
    m.param("esp", &p.espessura.to_string())?;
    m.param("larg", &p.largura.to_string())?;
    let mut data = SketchData::default();
    laco(&mut m, &mut data, &cam, p.espessura);
    laco(&mut m, &mut data, &cam, 0.);
    let sk = m.sketch("Laco", PlaneRef::Origin(OriginPlane::XY), data);
    extrusao_de(&mut m, "Correia", sk, "larg");
    Ok(m.doc)
}

/// Tipo de polia.
#[derive(Clone, Debug, PartialEq)]
pub enum TipoPolia {
    /// dentada lisa no diâmetro primitivo, com flanges
    Dentada { passo: f64, dentes: u32, largura: f64 },
    /// em V: seção da correia (largura do topo e altura) e diâmetro primitivo
    EmV { topo: f64, altura: f64, diametro: f64 },
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParamPolia {
    pub tipo: TipoPolia,
    pub furo: f64,
}

impl ParamPolia {
    /// Diâmetro externo da face de contato da correia (o primitivo) e largura útil da polia.
    pub fn diametro(&self) -> f64 {
        match self.tipo {
            TipoPolia::Dentada { passo, dentes, .. } => crate::transmissao_dados::diametro_polia_dentada(passo, dentes),
            TipoPolia::EmV { diametro, .. } => diametro,
        }
    }
}

/// Polia por revolução: perfil (raio, z) no plano XZ em volta de Z.
pub fn polia_documento(p: &ParamPolia) -> Result<Document, String> {
    let d = p.diametro();
    if !(d.is_finite() && d >= 10.) || !(p.furo >= 0. && p.furo.is_finite()) {
        return Err("diâmetro da polia (mínimo 10 mm) ou furo inválido".into());
    }
    let r = d / 2.;
    let (rhub, pts): (f64, Vec<[f64; 2]>) = match p.tipo {
        TipoPolia::Dentada { largura, .. } => {
            if largura <= 0. {
                return Err("largura da polia inválida".into());
            }
            let (fl, e) = (1.2, 1.5); // flange: sobra radial e espessura
            let rh = (p.furo * 0.9).max(3.);
            (rh, vec![[0., 0.], [r + fl, 0.], [r + fl, e], [r, e], [r, e + largura], [r + fl, e + largura], [r + fl, 2. * e + largura], [0., 2. * e + largura]])
        }
        TipoPolia::EmV { topo, altura, .. } => {
            if !(topo > 0. && altura > 0.) || altura >= r {
                return Err("seção da correia inválida para esta polia".into());
            }
            let fundo = (topo - 2. * altura * 19_f64.to_radians().tan()).max(1.);
            let e = 3.;
            let zc = e + topo / 2.;
            let h = 2. * e + topo;
            let rh = (p.furo * 0.9).max(3.);
            // diâmetro dado é o externo; sulco trapezoidal da boca (topo) ao fundo (altura)
            (rh, vec![[0., 0.], [r, 0.], [r, e], [r - altura, zc - fundo / 2.], [r - altura, zc + fundo / 2.], [r, e + topo], [r, h], [0., h]])
        }
    };
    let _ = rhub;
    let mut m = Montador::novo(&match p.tipo {
        TipoPolia::Dentada { dentes, .. } => format!("Polia dentada z{dentes}"),
        TipoPolia::EmV { .. } => format!("Polia em V Ø{d:.0}"),
    });
    m.doc.props.material = "Alumínio 6061".into();
    m.doc.props.part_type = PartType::Fabricada;
    let altura_total = pts.iter().map(|q| q[1]).fold(0., f64::max);
    m.param("furo", &p.furo.to_string())?;
    m.param("dpri", &d.to_string())?;
    let mut data = SketchData::default();
    let mut al = || m.doc.alloc();
    let ids: Vec<Id> = pts.iter().map(|q| data.add_point(&mut al, q[0], q[1])).collect();
    for i in 0..ids.len() {
        data.add_curve(&mut al, Curve::Line { a: ids[i], b: ids[(i + 1) % ids.len()] }, false);
    }
    let sk = m.sketch("Perfil_polia", PlaneRef::Origin(OriginPlane::XZ), data);
    let rev = m.doc.add_feature(FeatureKind::Revolve(Revolve { sketch: sk, axis: AxisRef::Origin(OriginAxis::Z), angle: "360".into(), symmetric: false, op: BoolOp::Add, thin: None, thin_side: 0, angle2: None }));
    nomear(&mut m, rev, "Polia");
    if p.furo > 0. {
        let plano = m.plano("Plano_furo", "-1")?;
        m.prim("Furo", PrimKind::Cilindro, plano, ["furo/2", &(altura_total + 2.).to_string(), "0", "0"], BoolOp::Cut)?;
    }
    Ok(m.doc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transmissao_dados::{comprimento_correia, distancia_corrigida_corrente, elos_da_corrente};
    use crate::{recompute, Status};

    fn medir(doc: &Document) -> (f64, [f64; 3], [f64; 3]) {
        let r = recompute(doc);
        assert!(r.param_error.is_none(), "{:?}", r.param_error);
        for f in &doc.features {
            assert_eq!(r.status[&f.id], Status::Ok, "{} / {}", doc.name, f.name);
        }
        assert_eq!(r.bodies.len(), 1, "{}", doc.name);
        let b = &r.bodies[0];
        let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
        for p in b.mesh.positions.chunks_exact(3) {
            for i in 0..3 {
                lo[i] = lo[i].min(p[i] as f64);
                hi[i] = hi[i].max(p[i] as f64);
            }
        }
        (b.mass.volume, lo, hi)
    }

    #[test]
    fn so_corrente_conferida_gera_roda() {
        assert!(calculo_roda(&ParamRoda::nova("ANSI 40", 17, 8.)).is_err(), "largura em conflito");
        assert!(calculo_roda(&ParamRoda::nova("ISO 06B-1", 17, 8.)).is_err());
        assert!(calculo_roda(&ParamRoda::nova("inexistente", 17, 8.)).is_err());
        assert!(calculo_roda(&ParamRoda::nova("ISO 08B-1", 5, 8.)).is_err());
        let c = calculo_roda(&ParamRoda::nova("ISO 08B-1", 19, 8.)).unwrap();
        assert!(c.diametro_topo > c.diametro_primitivo && c.diametro_primitivo > c.diametro_raiz);
        assert!((c.largura_dente - 0.9 * 7.75).abs() < 1e-9);
    }

    #[test]
    fn perfil_da_roda_fecha_para_varios_dentes() {
        for n in [11, 15, 19, 25, 40, 60] {
            let c = crate::transmissao_dados::corrente("ISO 08B-1").unwrap();
            let r = passos_roda(c, n);
            assert!(r.is_ok(), "z{n}: {r:?}");
        }
    }

    #[test]
    fn roda_gera_solido_com_largura_e_diametro_de_topo() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let p = ParamRoda::nova("ISO 08B-1", 19, 8.);
        let c = calculo_roda(&p).unwrap();
        let (v, lo, hi) = medir(&roda_documento(&p).unwrap());
        assert!(lo[2].abs() < 1e-6, "{lo:?}");
        let raio = hi[0].max(hi[1]);
        assert!((raio - c.diametro_topo / 2.).abs() < 0.1, "{raio} contra {}", c.diametro_topo / 2.);
        // o volume dos dentes fica entre o disco de raiz e o de topo, menos o furo
        let w = c.largura_dente;
        let (rr, rt) = (c.diametro_raiz / 2., c.diametro_topo / 2.);
        assert!(v > PI * rr * rr * w * 0.8 - PI * 16. * w && v < PI * rt * rt * w + PI * 100. * 8., "{v}");
    }

    #[test]
    fn caminho_fecha_e_elos_ficam_sobre_o_laco() {
        let cam = Caminho::novo(40., 80., 300.).unwrap();
        // comprimento do laço: 2 sqrt(c² − ΔR²) + r1(π − 2g) + r2(π + 2g)
        let g = (40_f64 / 300.).asin();
        let esperado = 2. * (300_f64.powi(2) - 40_f64.powi(2)).sqrt() + 40. * (PI - 2. * g) + 80. * (PI + 2. * g);
        assert!((cam.comprimento() - esperado).abs() < 1e-9);
        // contínuo: o ponto em s = 0 e s = L coincidem, e o caminho passa pelas tangentes
        let (a, b) = (cam.ponto(0.), cam.ponto(cam.comprimento()));
        assert!((a.x - b.x).abs() < 1e-9 && (a.y - b.y).abs() < 1e-9);
        // cada ponto está sobre uma das retas tangentes ou a um raio de um dos centros
        let l = cam.comprimento();
        for i in 0..400 {
            let q = cam.ponto(l * i as f64 / 400.);
            let d1 = q.x.hypot(q.y);
            let d2 = (q.x - 300.).hypot(q.y);
            assert!((d1 - 40.).abs() < 1e-6 || (d2 - 80.).abs() < 1e-6 || ((q.y.abs() > 1e-9) && d1 > 40. - 1e-6 && d2 > 80. - 1e-6), "{q:?}");
        }
        // passos pequenos: a distância entre pontos vizinhos é o passo (corda <= arco)
        let el = cam.elos(100, 0.);
        let corda = ((el[1].x - el[0].x).powi(2) + (el[1].y - el[0].y).powi(2)).sqrt();
        assert!(corda <= l / 100. + 1e-9 && corda > 0.9 * l / 100.);
        // fase: andar um elo inteiro leva cada elo ao lugar do vizinho
        let el2 = cam.elos(100, l / 100.);
        assert!((el2[0].x - el[1].x).abs() < 1e-9 && (el2[0].y - el[1].y).abs() < 1e-9);
        assert!(Caminho::novo(40., 80., 30.).is_err());
        assert!((avanco_da_corrente(30., 90.) - 30. * PI / 2.).abs() < 1e-12);
    }

    #[test]
    fn corrente_e_correia_geram_um_corpo_so() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (n1, n2) = (17u32, 34u32);
        let c = crate::transmissao_dados::corrente("ISO 08B-1").unwrap();
        let elos = elos_da_corrente(c.passo, n1, n2, 400.);
        let cc = distancia_corrigida_corrente(c.passo, n1, n2, elos).unwrap();
        let p = ParamLacoCorrente { corrente: "ISO 08B-1".into(), dentes1: n1, dentes2: n2, elos, centros: cc };
        let (v, lo, hi) = medir(&corrente_documento(&p).unwrap());
        assert!(v > 0.);
        // largura: das placas externas, b1 + 2 tp
        assert!(lo[2] < -0.5 && hi[2] > c.largura + 0.5, "{lo:?} {hi:?}");
        // alcance em x: do raio da roda menor (para trás) até o centro + raio da maior
        let (d1, d2) = (diametro_primitivo_roda(c.passo, n1), diametro_primitivo_roda(c.passo, n2));
        assert!(lo[0] < -d1 / 2. + 1. && hi[0] > cc + d2 / 2. - 1., "{lo:?} {hi:?}");

        // correia: anel de espessura e e largura w; volume = área do anel × w
        let (d1, d2, cb) = (50., 100., 200.);
        let pb = ParamLacoCorreia { d1, d2, centros: cb, espessura: 3., largura: 10. };
        let (vb, lob, hib) = medir(&correia_documento(&pb).unwrap());
        let area = Caminho::novo(d1 / 2., d2 / 2., cb).unwrap().comprimento() * 3.;
        assert!((vb - area * 10.).abs() / (area * 10.) < 0.03, "volume {vb} contra ~{}", area * 10.);
        assert!(lob[2].abs() < 1e-6 && (hib[2] - 10.).abs() < 1e-6);
        // o comprimento do laço da correia é o da fórmula de correia (próximo)
        let l_formula = comprimento_correia(d1, d2, cb);
        assert!((Caminho::novo(25., 50., cb).unwrap().comprimento() - l_formula).abs() < 1.5);
    }

    #[test]
    fn polias_geram_solido_com_o_diametro_certo() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let p = ParamPolia { tipo: TipoPolia::Dentada { passo: 2., dentes: 20, largura: 8. }, furo: 5. };
        let (v, lo, hi) = medir(&polia_documento(&p).unwrap());
        assert!(v > 0. && lo[2].abs() < 1e-6);
        let r = p.diametro() / 2. + 1.2;
        assert!((hi[0].max(hi[1]) - r).abs() < 0.1, "{hi:?} {r}");
        let pv = ParamPolia { tipo: TipoPolia::EmV { topo: 10., altura: 6., diametro: 80. }, furo: 12. };
        let (vv, _, hv) = medir(&polia_documento(&pv).unwrap());
        assert!(vv > 0. && (hv[0].max(hv[1]) - 40.).abs() < 0.1, "{hv:?}");
        assert!(polia_documento(&ParamPolia { tipo: TipoPolia::EmV { topo: 10., altura: 50., diametro: 40. }, furo: 5. }).is_err());
    }
}
