// SPDX-License-Identifier: LGPL-2.1-or-later
//! Engrenagens cônicas retas a 90° e sem-fim com coroa (B341).
//!
//! **Cônica reta.** O dente segue a aproximação clássica das retas de projeto de cônicas
//! ("Tredgold", da engrenagem virtual no cone de costas): o perfil é o de uma engrenagem reta de
//! `z' = z / cos δ` dentes e raio `r / cos δ`, com as medidas de altura do dente mantidas e os ângulos
//! dilatados por `1 / cos δ` para caber nos `z` dentes reais. `δ = atan(z / z_par)` é o ângulo do
//! cone primitivo (eixos a 90°: δ1 + δ2 = 90°). O perfil da extremidade maior é desenhado no plano
//! perpendicular ao eixo (a pequena diferença de altura entre pé e topo é projetada) e o sólido é um
//! `loft` reto até a mesma seção reduzida pelo fator `(Re − b) / Re` em direção ao vértice, então as
//! arestas do dente passam pelo vértice do cone, como numa cônica reta de verdade.
//!
//! **Convenção de eixos.** O eixo da peça é **Z**; a extremidade maior (de módulo `m`) fica em
//! `z = 0`, o vértice do cone primitivo em `z = A = r / tan δ` (para +Z, no rumo do par) e o cubo
//! em `z < 0`. O dente 0 é centrado em +X.
//!
//! **Sem-fim.** Eixo Z, de `z = 0` a `z = L`; o filete é rotacionado em `2π z / avanço` (mão
//! direita para `+`). A seção transversal de cada filete tem o flanco reto no plano axial, o que
//! dá, em polar, meia-largura angular `t(r) / (m z1)` com `t(r) = π m / 2 − 2 (r − rp) tan α`. O
//! sólido é um `loft` reto de seções giradas (até 15° entre seções), com no máximo
//! [`MAX_SECOES`] seções. A **coroa** do sem-fim é a helicoidal do gerador comum, com módulo normal
//! `m cos λ` e hélice λ, com a ponta do dente cortada em garganta pelo toro do sem-fim (B52, ver
//! [`coroa_documento`]).

use crate::engrenagens::{pontos_flanco, polar, Dente, ParamEngrenagem, Passo, TipoEngrenagem};
use crate::fixadores::Montador;
use fenkra_core::*;
use std::f64::consts::PI;

/// Máximo de seções do `loft` do sem-fim (dezenas de voltas pedem mais passo ou menos comprimento).
pub const MAX_SECOES: usize = 160;
/// Giro máximo entre duas seções do sem-fim (graus).
const PASSO_SECAO: f64 = 15.;

// ---------------------------------------------------------------- cônica

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ParamConica {
    /// módulo na extremidade maior
    pub modulo: f64,
    pub dentes: u32,
    /// dentes da peça conjugada (define o ângulo do cone)
    pub dentes_par: u32,
    pub angulo_pressao: f64,
    /// largura de face ao longo da geratriz; 0 = Re / 3
    pub largura: f64,
    /// folga entre dentes tirada da espessura (par)
    pub folga_dentes: f64,
    pub furo: f64,
}

impl Default for ParamConica {
    fn default() -> Self {
        ParamConica { modulo: 2., dentes: 20, dentes_par: 20, angulo_pressao: 20., largura: 0., folga_dentes: 0., furo: 8. }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CalculoConica {
    /// ângulo do cone primitivo δ (graus)
    pub angulo_cone: f64,
    pub diametro_primitivo: f64,
    /// diâmetro de topo e de raiz na extremidade maior
    pub diametro_topo: f64,
    pub diametro_raiz: f64,
    /// distância do vértice ao primitivo maior (geratriz Re)
    pub geratriz: f64,
    pub largura: f64,
    /// altura do vértice acima do plano da extremidade maior (z = A)
    pub altura_vertice: f64,
    /// espessura axial dos dentes (z de 0 até aqui)
    pub profundidade_dentes: f64,
    /// dentes da engrenagem virtual z / cos δ
    pub dentes_virtuais: f64,
    pub fator_pequena: f64,
    pub avisos: Vec<String>,
}

/// Cone de dois eixos a 90°: δ1 = atan(z1 / z2).
pub fn angulo_cone(dentes: u32, dentes_par: u32) -> f64 {
    (dentes as f64 / dentes_par as f64).atan().to_degrees()
}

pub fn calculo_conica(p: &ParamConica) -> Result<CalculoConica, String> {
    for (v, n) in [(p.modulo, "módulo"), (p.angulo_pressao, "ângulo de pressão"), (p.largura, "largura"), (p.folga_dentes, "folga"), (p.furo, "furo")] {
        if !v.is_finite() {
            return Err(format!("{n} inválido"));
        }
    }
    if p.modulo <= 0. || p.modulo > 50. {
        return Err("o módulo precisa estar entre 0 e 50 mm".into());
    }
    if !(14.5..=30.).contains(&p.angulo_pressao) {
        return Err("o ângulo de pressão precisa estar entre 14,5° e 30°".into());
    }
    if p.dentes < 8 || p.dentes > 200 || p.dentes_par < 8 || p.dentes_par > 200 {
        return Err("a cônica precisa de 8 a 200 dentes (a peça e a conjugada)".into());
    }
    if p.folga_dentes < 0. || p.folga_dentes > 0.5 * p.modulo || p.furo < 0. || p.largura < 0. {
        return Err("folga, furo e largura precisam ser positivos (folga até meio módulo)".into());
    }
    let m = p.modulo;
    let delta = (p.dentes as f64 / p.dentes_par as f64).atan();
    let (cd, sd) = (delta.cos(), delta.sin());
    let r = m * p.dentes as f64 / 2.;
    let re = r / sd;
    let largura = if p.largura > 0. { p.largura } else { re / 3. };
    if largura >= re * 0.5 {
        return Err(format!("a largura de face precisa ficar abaixo de metade da geratriz ({:.1} mm)", re * 0.5));
    }
    let k = (re - largura) / re;
    let a = r / delta.tan();
    let g = dente_virtual(p);
    g.limites()?;
    let zv = p.dentes as f64 / cd;
    let mut avisos = vec![];
    if zv < 17. {
        avisos.push(format!("dentes virtuais z/cos δ = {zv:.1} < 17: risco de rebaixo no pé do dente"));
    }
    if largura > re / 3. + 1e-9 {
        avisos.push("largura acima de um terço da geratriz: o dente fica fino no extremo menor".into());
    }
    let calc = CalculoConica {
        angulo_cone: delta.to_degrees(),
        diametro_primitivo: 2. * r,
        diametro_topo: 2. * (r + m * cd),
        diametro_raiz: 2. * (r - 1.25 * m * cd),
        geratriz: re,
        largura,
        altura_vertice: a,
        profundidade_dentes: a * (1. - k),
        dentes_virtuais: zv,
        fator_pequena: k,
        avisos,
    };
    let (dc, _) = cubo_dimensoes(p, &calc)?;
    if p.furo >= dc - 1.5 {
        return Err(format!("o furo precisa ser menor que {:.1} mm (cubo e raiz do extremo menor)", dc - 1.5));
    }
    Ok(calc)
}

/// Dente da engrenagem virtual (cone de costas).
fn dente_virtual(p: &ParamConica) -> Dente {
    let m = p.modulo;
    let delta = (p.dentes as f64 / p.dentes_par as f64).atan();
    let rv = m * p.dentes as f64 / 2. / delta.cos();
    let alfa = p.angulo_pressao.to_radians();
    Dente { r: rv, rb: rv * alfa.cos(), ra: rv + m, rf: rv - 1.25 * m, sp: PI * m / 2. - p.folga_dentes, alfa, z: (p.dentes as f64 / delta.cos()).round() as u32, rho: 0. }
}

/// Diâmetro e comprimento do cubo: cabe sob a raiz do extremo menor.
fn cubo_dimensoes(p: &ParamConica, c: &CalculoConica) -> Result<(f64, f64), String> {
    let raiz_pequena = c.diametro_raiz * c.fator_pequena;
    let dc = (raiz_pequena * 0.85).min((p.furo * 1.8).max(p.furo + 6.));
    if dc < 2. {
        return Err("a cônica é pequena demais para ter cubo".into());
    }
    Ok((dc, (p.furo * 1.2).max(p.modulo * 3.)))
}

/// Contorno da cônica no plano perpendicular ao eixo, reduzido por `escala` (1 = extremidade maior).
fn passos_conica(p: &ParamConica, escala: f64) -> Result<([f64; 2], Vec<Passo>), String> {
    let g = dente_virtual(p);
    let l = g.limites()?;
    let delta = (p.dentes as f64 / p.dentes_par as f64).atan();
    let cd = delta.cos();
    let r = p.modulo * p.dentes as f64 / 2.;
    let nf = pontos_flanco(p.dentes);
    let abaixo = g.rf < g.rb;
    let r_de = |th: f64| g.rb * (1. + th * th).sqrt();
    // ponto do perfil virtual (raio rv', ângulo a') no dente que fica em `fi`
    let mp = |rv: f64, a: f64, fi: f64| polar(escala * (r + (rv - g.r) * cd), fi + a / cd);
    let ths: Vec<f64> = (0..=nf).map(|i| l.th0 + (l.th1 - l.th0) * i as f64 / nf as f64).collect();
    let passo_ang = 2. * PI / p.dentes as f64;
    let inicio = mp(g.rf, -l.psi0, 0.);
    let mut passos = vec![];
    for k in 0..p.dentes {
        let fi = passo_ang * k as f64;
        if abaixo {
            passos.push(Passo::Linha(mp(g.rb, -l.psi0, fi)));
        }
        passos.push(Passo::Spline(ths[1..].iter().map(|th| mp(r_de(*th), -g.meio_angulo(*th), fi)).collect()));
        passos.push(Passo::Arco(mp(g.ra, l.psia, fi)));
        passos.push(Passo::Spline(ths[..nf].iter().rev().map(|th| mp(r_de(*th), g.meio_angulo(*th), fi)).collect()));
        if abaixo {
            passos.push(Passo::Linha(mp(g.rf, l.psi0, fi)));
        }
        passos.push(Passo::Arco(mp(g.rf, -l.psi0, fi + passo_ang)));
    }
    Ok((inicio, passos))
}

/// Documento paramétrico da cônica reta: dentes (loft), cubo e furo.
pub fn conica_documento(p: &ParamConica) -> Result<Document, String> {
    let c = calculo_conica(p)?;
    let mut m = Montador::novo(&format!("Engrenagem cônica m{} z{}/z{}", p.modulo, p.dentes, p.dentes_par));
    m.doc.props.material = "Aço 1045".into();
    m.doc.props.part_type = PartType::Fabricada;
    for (nome, v) in [("m", p.modulo), ("furo", p.furo)] {
        m.param(nome, &v.to_string())?;
    }
    m.doc.add_param("z", &p.dentes.to_string(), Unit::Numero).map_err(|e| e.to_string())?;
    m.doc.add_param("alpha", &p.angulo_pressao.to_string(), Unit::Angulo).map_err(|e| e.to_string())?;
    m.param("zt", &c.profundidade_dentes.to_string())?;
    let (dc, hl) = cubo_dimensoes(p, &c)?;
    m.param("dcubo", &dc.to_string())?;
    m.param("hcubo", &hl.to_string())?;
    // seção da extremidade maior em z = 0 e a reduzida em z = zt
    let (ini0, p0) = passos_conica(p, 1.)?;
    let (ini1, p1) = passos_conica(p, c.fator_pequena)?;
    let xy = PlaneRef::Origin(OriginPlane::XY);
    let s0 = m.sketch_contorno("Secao_maior", xy, ini0, &p0, 0., None);
    let plano_t = m.plano("Plano_menor", "zt")?;
    let s1 = m.sketch_contorno("Secao_menor", plano_t, ini1, &p1, 0., None);
    let id = m.doc.add_feature(FeatureKind::Loft(Loft { sections: vec![s0, s1], ruled: true, closed: false, op: BoolOp::Add, torcao: vec![], inicio: Default::default(), fim: Default::default() }));
    if let Some(f) = m.doc.features.iter_mut().find(|f| f.id == id) {
        f.name = "Dentes_conicos".into();
    }
    let plano_cubo = m.plano("Plano_cubo", "-hcubo")?;
    m.prim("Cubo", PrimKind::Cilindro, plano_cubo, ["dcubo/2", "hcubo+0.01", "0", "0"], BoolOp::Add)?;
    if p.furo > 0. {
        let plano_furo = m.plano("Plano_furo", "-hcubo-1")?;
        m.prim("Furo", PrimKind::Cilindro, plano_furo, ["furo/2", "hcubo+zt+2", "0", "0"], BoolOp::Cut)?;
    }
    Ok(m.doc)
}

// ---------------------------------------------------------------- sem-fim

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ParamSemFim {
    /// módulo axial
    pub modulo: f64,
    /// entradas (filetes)
    pub entradas: u32,
    pub diametro_primitivo: f64,
    pub comprimento: f64,
    pub angulo_pressao: f64,
    pub furo: f64,
    /// dentes da coroa
    pub dentes_coroa: u32,
    /// largura da coroa; 0 = 0,75 do diâmetro primitivo do sem-fim
    pub largura_coroa: f64,
    pub furo_coroa: f64,
}

impl Default for ParamSemFim {
    fn default() -> Self {
        ParamSemFim { modulo: 2., entradas: 1, diametro_primitivo: 20., comprimento: 24., angulo_pressao: 20., furo: 8., dentes_coroa: 30, largura_coroa: 0., furo_coroa: 10. }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CalculoSemFim {
    pub avanco: f64,
    /// ângulo de avanço λ no primitivo (graus)
    pub angulo_avanco: f64,
    pub diametro_topo: f64,
    pub diametro_raiz: f64,
    pub diametro_primitivo_coroa: f64,
    pub distancia_centros: f64,
    /// i = z2 / entradas
    pub razao: f64,
    pub largura_coroa: f64,
    pub secoes: usize,
    pub avisos: Vec<String>,
}

pub fn calculo_sem_fim(p: &ParamSemFim) -> Result<CalculoSemFim, String> {
    for (v, n) in [(p.modulo, "módulo"), (p.diametro_primitivo, "diâmetro primitivo"), (p.comprimento, "comprimento"), (p.angulo_pressao, "ângulo de pressão"), (p.furo, "furo"), (p.largura_coroa, "largura da coroa"), (p.furo_coroa, "furo da coroa")] {
        if !v.is_finite() {
            return Err(format!("{n} inválido"));
        }
    }
    if p.modulo <= 0. || p.modulo > 30. {
        return Err("o módulo axial precisa estar entre 0 e 30 mm".into());
    }
    if !(1..=4).contains(&p.entradas) {
        return Err("o sem-fim tem de 1 a 4 entradas".into());
    }
    if !(14.5..=30.).contains(&p.angulo_pressao) {
        return Err("o ângulo de pressão precisa estar entre 14,5° e 30°".into());
    }
    let m = p.modulo;
    if p.diametro_primitivo < 5. * m {
        return Err(format!("o diâmetro primitivo do sem-fim precisa ser de pelo menos 5 módulos ({:.1} mm)", 5. * m));
    }
    if p.dentes_coroa < 17 || p.dentes_coroa > 200 {
        return Err("a coroa precisa de 17 a 200 dentes".into());
    }
    if p.furo < 0. || p.furo >= p.diametro_primitivo - 2.5 * m - 2. {
        return Err(format!("o furo do sem-fim precisa ser menor que {:.1} mm", p.diametro_primitivo - 2.5 * m - 2.));
    }
    let avanco = PI * m * p.entradas as f64;
    let lam = (avanco / (PI * p.diametro_primitivo)).atan();
    if p.comprimento < 2. * avanco {
        return Err(format!("o comprimento precisa cobrir ao menos 2 avanços ({:.1} mm)", 2. * avanco));
    }
    let secoes = secoes_do_sem_fim(p.comprimento, avanco);
    if secoes > MAX_SECOES {
        return Err(format!("o sem-fim pede {secoes} seções (máximo {MAX_SECOES}): reduza o comprimento ou aumente o módulo/entradas"));
    }
    let mut avisos = vec![];
    if lam.to_degrees() > 25. {
        avisos.push("avanço acima de 25°: o rendimento é bom, mas a geometria do dente da coroa foge do caso usual".into());
    }
    let rc = coroa_param(p)?;
    let cc = crate::engrenagens::calculo(&rc)?;
    let b = if p.largura_coroa > 0. { p.largura_coroa } else { 0.75 * p.diametro_primitivo };
    Ok(CalculoSemFim {
        avanco,
        angulo_avanco: lam.to_degrees(),
        diametro_topo: p.diametro_primitivo + 2. * m,
        diametro_raiz: p.diametro_primitivo - 2.5 * m,
        diametro_primitivo_coroa: m * p.dentes_coroa as f64,
        distancia_centros: (p.diametro_primitivo + m * p.dentes_coroa as f64) / 2.,
        razao: p.dentes_coroa as f64 / p.entradas as f64,
        largura_coroa: b,
        secoes,
        avisos: avisos.into_iter().chain(cc.avisos).collect(),
    })
}

fn secoes_do_sem_fim(comprimento: f64, avanco: f64) -> usize {
    (comprimento / avanco * 360. / PASSO_SECAO).ceil() as usize + 1
}

/// A coroa: helicoidal de módulo normal `m cos λ` e hélice λ (mesma mão do sem-fim); a garganta é
/// acrescentada em [`coroa_documento`].
pub fn coroa_param(p: &ParamSemFim) -> Result<ParamEngrenagem, String> {
    let lam = (PI * p.modulo * p.entradas as f64 / (PI * p.diametro_primitivo)).atan();
    let b = if p.largura_coroa > 0. { p.largura_coroa } else { 0.75 * p.diametro_primitivo };
    Ok(ParamEngrenagem {
        tipo: TipoEngrenagem::Helicoidal,
        modulo: p.modulo * lam.cos(),
        dentes: p.dentes_coroa,
        angulo_pressao: p.angulo_pressao,
        angulo_helice: lam.to_degrees(),
        largura: b,
        furo: p.furo_coroa,
        ..Default::default()
    })
}

/// Coroa com **garganta** (B52): o dente cresce até o diâmetro externo `d_e2 = d_a2 + m` e a ponta é
/// cortada por um toro em volta do eixo do sem-fim, de raio `r_g = a − d_a2/2` com centro na
/// distância entre centros `a`, no plano médio da coroa. No plano médio a ponta fica no diâmetro de
/// topo `d_a2` (garganta) e sobe em arco até `d_e2` nas faces, abraçando o sem-fim. Se o dente mais
/// alto ficaria pontiagudo, o acréscimo cai para m/4 e depois para zero (coroa sem garganta).
pub fn coroa_documento(p: &ParamSemFim) -> Result<Document, String> {
    let c = calculo_sem_fim(p)?;
    let base = coroa_param(p)?;
    let ra2 = crate::engrenagens::calculo(&base)?.diametro_externo / 2.;
    let (mut d, acrescimo) = [0.5, 0.25]
        .iter()
        .find_map(|f| crate::engrenagens::engrenagem_documento(&ParamEngrenagem { acrescimo_topo: f * p.modulo, ..base.clone() }).ok().map(|d| (d, f * p.modulo)))
        .map_or_else(|| crate::engrenagens::engrenagem_documento(&base).map(|d| (d, 0.)), Ok)?;
    if acrescimo > 0. {
        let mut m = Montador { doc: d };
        let r_g = c.distancia_centros - ra2;
        m.param("rg", &r_g.to_string())?;
        let mut data = SketchData::default();
        let mut al = || m.doc.alloc();
        // perfil no XZ (x = raio, y = z): círculo do toro na distância entre centros, no plano médio
        let centro = data.add_point(&mut al, c.distancia_centros, c.largura_coroa / 2.);
        data.add_curve(&mut al, fenkra_core::sketch::Curve::Circle { c: centro, r: r_g }, false);
        let sk = m.sketch("Perfil_garganta", PlaneRef::Origin(OriginPlane::XZ), data);
        let id = m.doc.add_feature(FeatureKind::Revolve(Revolve { sketch: sk, axis: AxisRef::Origin(OriginAxis::Z), angle: "360".into(), symmetric: false, op: BoolOp::Cut, thin: None, thin_side: 0, angle2: None }));
        if let Some(f) = m.doc.features.iter_mut().find(|f| f.id == id) {
            f.name = "Garganta".into();
        }
        d = m.doc;
    }
    d.props.description = format!("Coroa do sem-fim m{} z{}", p.modulo, p.dentes_coroa);
    Ok(d)
}

/// Meia-largura angular do filete no raio `r` (rad), para o filete de uma entrada.
fn meia_largura(p: &ParamSemFim, r: f64) -> f64 {
    let m = p.modulo;
    let rp = p.diametro_primitivo / 2.;
    let t = PI * m / 2. - 2. * (r - rp) * p.angulo_pressao.to_radians().tan();
    t / (m * p.entradas as f64)
}

/// Contorno transversal do sem-fim (todos os filetes), girado de `rot`.
fn passos_sem_fim(p: &ParamSemFim) -> Result<([f64; 2], Vec<Passo>), String> {
    let m = p.modulo;
    let (rp, rf, ra) = (p.diametro_primitivo / 2., p.diametro_primitivo / 2. - 1.25 * m, p.diametro_primitivo / 2. + m);
    let _ = rp;
    let (h_raiz, h_topo) = (meia_largura(p, rf), meia_largura(p, ra));
    let passo = 2. * PI / p.entradas as f64;
    if h_topo <= 0.01 || h_raiz >= passo / 2. - 0.01 {
        return Err("o filete fica pontiagudo ou encosta no vizinho: ajuste módulo, entradas ou ângulo de pressão".into());
    }
    let n = 7usize;
    let rs: Vec<f64> = (0..=n).map(|i| rf + (ra - rf) * i as f64 / n as f64).collect();
    let inicio = polar(rf, -h_raiz);
    let mut passos = vec![];
    for k in 0..p.entradas {
        let th = passo * k as f64;
        passos.push(Passo::Spline(rs[1..].iter().map(|r| polar(*r, th - meia_largura(p, *r))).collect()));
        passos.push(Passo::Arco(polar(ra, th + h_topo)));
        passos.push(Passo::Spline(rs[..n].iter().rev().map(|r| polar(*r, th + meia_largura(p, *r))).collect()));
        passos.push(Passo::Arco(polar(rf, th + passo - h_raiz)));
    }
    Ok((inicio, passos))
}

/// Documento do sem-fim: loft reto de seções girados do filete e furo central.
pub fn sem_fim_documento(p: &ParamSemFim) -> Result<Document, String> {
    let c = calculo_sem_fim(p)?;
    let mut m = Montador::novo(&format!("Sem-fim m{} {} entrada(s)", p.modulo, p.entradas));
    m.doc.props.material = "Aço 1045".into();
    m.doc.props.part_type = PartType::Fabricada;
    m.param("m", &p.modulo.to_string())?;
    m.param("d1", &p.diametro_primitivo.to_string())?;
    m.param("L", &p.comprimento.to_string())?;
    m.param("furo", &p.furo.to_string())?;
    m.param("avanco", &c.avanco.to_string())?;
    let (inicio, passos) = passos_sem_fim(p)?;
    let n = c.secoes - 1;
    let mut secoes = vec![];
    for i in 0..=n {
        let z = p.comprimento * i as f64 / n as f64;
        let plano = if i == 0 { PlaneRef::Origin(OriginPlane::XY) } else { m.plano(&format!("Plano_secao_{i}"), &format!("L*{i}/{n}"))? };
        secoes.push(m.sketch_contorno(&format!("Secao_{i}"), plano, inicio, &passos, 2. * PI * z / c.avanco, None));
    }
    let id = m.doc.add_feature(FeatureKind::Loft(Loft { sections: secoes, ruled: true, closed: false, op: BoolOp::Add, torcao: vec![], inicio: Default::default(), fim: Default::default() }));
    if let Some(f) = m.doc.features.iter_mut().find(|f| f.id == id) {
        f.name = "Filete".into();
    }
    if p.furo > 0. {
        let plano = m.plano("Plano_furo", "-1")?;
        m.prim("Furo", PrimKind::Cilindro, plano, ["furo/2", "L+2", "0", "0"], BoolOp::Cut)?;
    }
    Ok(m.doc)
}

/// Rotação (rad) do sem-fim em torno do próprio eixo para o filete passar no vão da coroa na linha
/// de contato, com o eixo do sem-fim ao longo de Y do mundo (ver montagem): o centro de um filete
/// precisa estar a `π m / 2` do plano médio da coroa. A posição axial do sem-fim é pelo centro.
pub fn giro_de_engrenamento(p: &ParamSemFim) -> f64 {
    let avanco = PI * p.modulo * p.entradas as f64;
    let z_t = p.comprimento / 2. + PI * p.modulo / 2.;
    (PI - 2. * PI * z_t / avanco).rem_euclid(2. * PI)
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn calculo_conica_20_20_e_20_40() {
        // z = 20/20 a 90°: δ = 45°, r = 20, Re = 28,28, A = 20, b = Re/3 = 9,43, k = 2/3
        let c = calculo_conica(&ParamConica::default()).unwrap();
        assert!((c.angulo_cone - 45.).abs() < 1e-9 && (c.geratriz - 20. * 2_f64.sqrt()).abs() < 1e-9);
        assert!((c.altura_vertice - 20.).abs() < 1e-9 && (c.largura - c.geratriz / 3.).abs() < 1e-9);
        assert!((c.fator_pequena - 2. / 3.).abs() < 1e-9 && (c.profundidade_dentes - 20. / 3.).abs() < 1e-9);
        assert!((c.dentes_virtuais - 20. * 2_f64.sqrt()).abs() < 1e-9);
        // 20/40: δ1 = atan(0,5) = 26,565°, δ2 = 63,435°; soma 90°
        let a = angulo_cone(20, 40);
        assert!((a - 26.5651).abs() < 1e-3 && (a + angulo_cone(40, 20) - 90.).abs() < 1e-9);
        let p = calculo_conica(&ParamConica { dentes: 20, dentes_par: 40, ..Default::default() }).unwrap();
        assert!((p.diametro_primitivo - 40.).abs() < 1e-9);
        // entradas inválidas
        assert!(calculo_conica(&ParamConica { dentes: 5, ..Default::default() }).is_err());
        assert!(calculo_conica(&ParamConica { largura: 50., ..Default::default() }).is_err());
        assert!(calculo_conica(&ParamConica { furo: 60., ..Default::default() }).is_err());
    }

    #[test]
    fn conica_gera_solido_com_a_caixa_esperada() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let p = ParamConica::default();
        let c = calculo_conica(&p).unwrap();
        let (v, lo, hi) = medir(&conica_documento(&p).unwrap());
        // a extremidade maior fica em z = 0 e o topo dos dentes tem raio r + m cos δ
        let r_topo = c.diametro_topo / 2.;
        assert!((hi[0].max(hi[1]) - r_topo).abs() < 0.2, "{hi:?} {r_topo}");
        assert!(lo[2] < -0.5 && (hi[2] - c.profundidade_dentes).abs() < 1e-3, "{lo:?} {hi:?}");
        // o volume é menor que o do tronco de cone dos topos e maior que o do cone das raízes
        let tronco = |r0: f64, r1: f64, h: f64| PI * h / 3. * (r0 * r0 + r0 * r1 + r1 * r1);
        let (rt, rr) = (c.diametro_topo / 2., c.diametro_raiz / 2.);
        let k = c.fator_pequena;
        let dentes = v - PI * 4. * 4. * 0.; // inclui cubo e furo
        assert!(dentes > 0.);
        let (v_topo, v_raiz) = (tronco(rt, rt * k, c.profundidade_dentes), tronco(rr, rr * k, c.profundidade_dentes));
        assert!(v < v_topo + 2000. && v > v_raiz * 0.8, "v {v} topo {v_topo} raiz {v_raiz}");
    }

    #[test]
    fn calculo_sem_fim_m2_d20() {
        // 1 entrada: avanço 2π = 6,283; tan λ = 6,283/(π 20) = 0,1; i = 30; centros (20 + 60)/2 = 40
        let c = calculo_sem_fim(&ParamSemFim::default()).unwrap();
        assert!((c.avanco - 2. * PI).abs() < 1e-9 && (c.angulo_avanco - 0.1_f64.atan().to_degrees()).abs() < 1e-9);
        assert!((c.razao - 30.).abs() < 1e-12 && (c.distancia_centros - 40.).abs() < 1e-9);
        assert!((c.diametro_topo - 24.).abs() < 1e-9 && (c.diametro_raiz - 15.).abs() < 1e-9);
        // duas entradas: avanço e razão acompanham
        let c2 = calculo_sem_fim(&ParamSemFim { entradas: 2, comprimento: 30., ..Default::default() }).unwrap();
        assert!((c2.avanco - 4. * PI).abs() < 1e-9 && (c2.razao - 15.).abs() < 1e-12);
        // a coroa é helicoidal: módulo normal m cos λ e transversal m
        let r = coroa_param(&ParamSemFim::default()).unwrap();
        assert!((r.modulo / (r.angulo_helice.to_radians().cos()) - 2.).abs() < 1e-9 && r.tipo == TipoEngrenagem::Helicoidal);
        // recusas
        assert!(calculo_sem_fim(&ParamSemFim { diametro_primitivo: 8., ..Default::default() }).is_err());
        assert!(calculo_sem_fim(&ParamSemFim { comprimento: 5., ..Default::default() }).is_err());
        assert!(calculo_sem_fim(&ParamSemFim { comprimento: 400., ..Default::default() }).is_err(), "seções demais");
    }

    #[test]
    fn sem_fim_gera_solido_e_a_coroa_tambem() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let p = ParamSemFim { comprimento: 13., ..Default::default() };
        let (v, lo, hi) = medir(&sem_fim_documento(&p).unwrap());
        assert!(lo[2].abs() < 1e-3 && (hi[2] - 13.).abs() < 1e-3, "{lo:?} {hi:?}");
        assert!(hi[0] > 11.9 && hi[0] < 12.1, "raio de topo 12: {hi:?}");
        // volume entre o do cilindro das raízes e o do cilindro dos topos, menos o furo
        let furo = PI * 16. * 13.;
        assert!(v > PI * 7.5 * 7.5 * 13. - furo && v < PI * 144. * 13. - furo, "{v}");
        let (vc, ..) = medir(&coroa_documento(&p).unwrap());
        assert!(vc > 0.);
    }

    /// Garganta da coroa: nas faces o dente chega ao diâmetro externo d_a2 + m; no plano médio a
    /// ponta fica no diâmetro de topo d_a2 (o toro do sem-fim corta o excesso).
    #[test]
    fn coroa_tem_garganta() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let p = ParamSemFim::default();
        let c = calculo_sem_fim(&p).unwrap();
        let doc = coroa_documento(&p).unwrap();
        assert!(doc.features.iter().any(|f| f.name == "Garganta"));
        let (v, lo, hi) = medir(&doc);
        let base = coroa_param(&p).unwrap();
        let ra2 = crate::engrenagens::calculo(&base).unwrap().diametro_externo / 2.;
        let re2 = ra2 + p.modulo / 2.;
        assert!(lo[2].abs() < 1e-3 && (hi[2] - c.largura_coroa).abs() < 1e-3, "{lo:?} {hi:?}");
        let raio_max = hi[0].max(hi[1]).max(-lo[0]).max(-lo[1]);
        assert!(raio_max > ra2 + 0.2 && raio_max < re2 + 0.05, "{raio_max} entre {ra2} e {re2}");
        // no plano médio nenhum ponto da malha passa do topo d_a2
        let r = recompute(&doc);
        let meio = c.largura_coroa / 2.;
        let maior_no_meio = r.bodies[0].mesh.positions.chunks_exact(3).filter(|q| (q[2] as f64 - meio).abs() < 0.3).map(|q| (q[0] as f64).hypot(q[1] as f64)).fold(0., f64::max);
        assert!(maior_no_meio < ra2 + 0.05, "{maior_no_meio} > {ra2}");
        // sem a garganta (dente alto inteiro) o volume é maior
        let (v_alto, ..) = medir(&crate::engrenagens::engrenagem_documento(&ParamEngrenagem { acrescimo_topo: p.modulo / 2., ..base }).unwrap());
        assert!(v < v_alto, "{v} {v_alto}");
    }
}
