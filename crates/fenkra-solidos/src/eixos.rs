//! Eixos escalonados e chavetas paralelas DIN 6885 forma A (B28, parte 2).
//!
//! **Convenção de eixos**: o eixo corre ao longo de **Z**, a partir de z = 0 (trechos em sequência,
//! para +Z), e o rasgo de chaveta é cortado em **+X**. O corpo do eixo é um único revolve do perfil
//! escalonado (com os chanfros); o rasgo é um corte num plano paralelo ao YZ. A chaveta fica
//! centrada em X e Y, com a base em z = 0 e o comprimento ao longo de X.
//!
//! As medidas do perfil ficam fixas no sketch (outro eixo = outro documento); os valores da tabela
//! (b, t1, ...) e as medidas de cada trecho ficam como parâmetros do documento, e os cortes do
//! rasgo e dos furos de centro usam esses parâmetros.

use crate::biblioteca::{chaveta_para_eixo, comprimentos_chaveta, Chaveta};
use crate::fixadores::Montador;
use fenkra_core::sketch::Curve;
use fenkra_core::*;

/// Rasgo de chaveta forma A num trecho: comprimento total (com as pontas arredondadas) e distância
/// do início do trecho (lado de z menor) ao início do rasgo.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RasgoChaveta {
    pub comprimento: f64,
    pub desloc_inicio: f64,
}

/// Trecho cilíndrico do eixo. O chanfro `chanfro_fim` (45°, 0 = sem) fica na aresta externa do fim
/// do trecho quando ela é convexa (última ponta ou ombro para um trecho menor); o primeiro trecho
/// também chanfra a ponta em z = 0.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrechoEixo {
    pub diametro: f64,
    pub comprimento: f64,
    pub chanfro_fim: f64,
    pub rasgo: Option<RasgoChaveta>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParamEixo {
    pub trechos: Vec<TrechoEixo>,
    /// furos de centro (DIN 332) nas duas pontas
    pub furo_centro: bool,
}

impl Default for ParamEixo {
    fn default() -> Self {
        ParamEixo {
            trechos: vec![
                TrechoEixo { diametro: 20., comprimento: 50., chanfro_fim: 1., rasgo: None },
                TrechoEixo { diametro: 25., comprimento: 70., chanfro_fim: 1., rasgo: Some(RasgoChaveta { comprimento: 40., desloc_inicio: 10. }) },
            ],
            furo_centro: false,
        }
    }
}

const MAX_TRECHOS: usize = 20;

fn finito(v: f64, nome: &str) -> Result<(), String> {
    if v.is_finite() {
        Ok(())
    } else {
        Err(format!("{nome} inválido"))
    }
}

impl ParamEixo {
    pub fn comprimento_total(&self) -> f64 {
        self.trechos.iter().map(|t| t.comprimento).sum()
    }

    /// Descrição da peça, ex.: "Eixo Ø20/Ø25 L120".
    pub fn descricao(&self) -> String {
        let ds: Vec<String> = self.trechos.iter().map(|t| format!("Ø{}", t.diametro)).collect();
        format!("Eixo {} L{}", ds.join("/"), self.comprimento_total())
    }

    /// Nome de arquivo (sem extensão) que identifica o conjunto de parâmetros.
    pub fn chave(&self) -> String {
        let ts: Vec<String> = self
            .trechos
            .iter()
            .map(|t| {
                let r = t.rasgo.map(|r| format!("r{}_{}", r.comprimento, r.desloc_inicio)).unwrap_or_default();
                format!("d{}l{}c{}{r}", t.diametro, t.comprimento, t.chanfro_fim)
            })
            .collect();
        format!("eixo-{}{}", ts.join("-"), if self.furo_centro { "-fc" } else { "" })
    }

    /// Valida e devolve a chaveta da tabela de cada trecho com rasgo.
    pub fn validar(&self) -> Result<Vec<Option<Chaveta>>, String> {
        if self.trechos.is_empty() || self.trechos.len() > MAX_TRECHOS {
            return Err(format!("o eixo precisa de 1 a {MAX_TRECHOS} trechos"));
        }
        let mut chavetas = vec![];
        for (i, t) in self.trechos.iter().enumerate() {
            let rot = format!("trecho {}", i + 1);
            for (v, nome) in [(t.diametro, "diâmetro"), (t.comprimento, "comprimento"), (t.chanfro_fim, "chanfro")] {
                finito(v, &format!("{nome} do {rot}"))?;
            }
            if t.diametro <= 0. || t.diametro > 1000. || t.comprimento <= 0. || t.comprimento > 10_000. {
                return Err(format!("{rot}: diâmetro (até 1000) e comprimento (até 10000) precisam ser positivos"));
            }
            if t.chanfro_fim < 0. || t.chanfro_fim >= t.diametro / 2. || t.chanfro_fim * if i == 0 { 2. } else { 1. } > t.comprimento {
                return Err(format!("{rot}: chanfro de {} mm não cabe no trecho", t.chanfro_fim));
            }
            if t.chanfro_fim > 0. {
                if let Some(prox) = self.trechos.get(i + 1) {
                    let ressalto = (t.diametro - prox.diametro) / 2.;
                    if ressalto > 0. && t.chanfro_fim > ressalto + 1e-9 {
                        return Err(format!("{rot}: o chanfro ({} mm) é maior que o ressalto até o trecho seguinte ({ressalto} mm)", t.chanfro_fim));
                    }
                }
            }
            chavetas.push(match t.rasgo {
                None => None,
                Some(r) => {
                    finito(r.comprimento, &format!("comprimento do rasgo do {rot}"))?;
                    finito(r.desloc_inicio, &format!("posição do rasgo do {rot}"))?;
                    let c = chaveta_para_eixo(t.diametro).ok_or_else(|| format!("{rot}: Ø{} está fora da tabela DIN 6885 (Ø6 a Ø85) — não há chaveta para o rasgo", t.diametro))?;
                    if r.comprimento <= c.b {
                        return Err(format!("{rot}: o rasgo precisa ser mais longo que a largura da chaveta ({} mm)", c.b));
                    }
                    if r.desloc_inicio < 0. || r.desloc_inicio + r.comprimento > t.comprimento + 1e-9 {
                        return Err(format!("{rot}: o rasgo (início {} + comprimento {}) não cabe no trecho de {} mm", r.desloc_inicio, r.comprimento, t.comprimento));
                    }
                    Some(c)
                }
            });
        }
        Ok(chavetas)
    }
}

/// Diâmetro do furo de centro (forma A, DIN 332) para o diâmetro da ponta.
fn diametro_furo_centro(d: f64) -> f64 {
    match d {
        d if d <= 10. => 1.6,
        d if d <= 30. => 2.5,
        d if d <= 50. => 4.,
        _ => 5.,
    }
}

fn nomear(m: &mut Montador, id: Id, nome: &str) {
    if let Some(f) = m.doc.features.iter_mut().find(|f| f.id == id) {
        f.name = nome.into();
    }
}

/// Plano paralelo ao YZ deslocado por `desvio` (expressão), oculto na viewport.
pub(crate) fn plano_yz(m: &mut Montador, nome: &str, desvio: &str) -> Result<PlaneRef, String> {
    let id = m.doc.add_offset_plane(PlaneRef::Origin(OriginPlane::YZ), desvio);
    let r = m.doc.references.iter_mut().find(|r| r.id == id).ok_or("plano não criado")?;
    r.name = nome.into();
    r.visible = false;
    Ok(PlaneRef::Datum(id))
}

/// Extrusão cega (`prof` é uma expressão) com a operação dada. Nos planos paralelos ao YZ a
/// extrusão normal vai para −X; `inverter` leva o corte para +X.
pub(crate) fn extrusao_cega(m: &mut Montador, nome: &str, sketch: Id, prof: &str, op: BoolOp, inverter: bool) {
    let id = m.doc.add_feature(FeatureKind::Extrude(Extrude {
        sketch,
        op,
        extent: Extent::Blind(prof.into()),
        reverse: inverter,
        taper: "0".into(),
        thin: None,
        thin_side: 0,
        dir: None,
    }));
    nomear(m, id, nome);
}

/// Contorno de "estádio" (duas retas e dois semicírculos de raio `r`) entre os centros `a` e `b`.
fn estadio(m: &mut Montador, nome: &str, plano: PlaneRef, a: [f64; 2], b: [f64; 2], r: f64) -> Id {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let n = dx.hypot(dy);
    let nv = [-dy / n * r, dx / n * r];
    let mut data = SketchData::default();
    let mut al = || m.doc.alloc();
    let (ca, cb) = (data.add_point(&mut al, a[0], a[1]), data.add_point(&mut al, b[0], b[1]));
    let pb_baixo = data.add_point(&mut al, b[0] - nv[0], b[1] - nv[1]);
    let pb_cima = data.add_point(&mut al, b[0] + nv[0], b[1] + nv[1]);
    let pa_cima = data.add_point(&mut al, a[0] + nv[0], a[1] + nv[1]);
    let pa_baixo = data.add_point(&mut al, a[0] - nv[0], a[1] - nv[1]);
    data.add_curve(&mut al, Curve::Arc { c: cb, s: pb_baixo, e: pb_cima }, false);
    data.add_curve(&mut al, Curve::Line { a: pb_cima, b: pa_cima }, false);
    data.add_curve(&mut al, Curve::Arc { c: ca, s: pa_cima, e: pa_baixo }, false);
    data.add_curve(&mut al, Curve::Line { a: pa_baixo, b: pb_baixo }, false);
    m.sketch(nome, plano, data)
}

/// Documento paramétrico do eixo (peça `.fkp`).
pub fn eixo_documento(p: &ParamEixo) -> Result<Document, String> {
    let chavetas = p.validar()?;
    let mut m = Montador::novo(&p.descricao());
    m.doc.props.material = "Aço 1045".into();
    m.doc.props.part_type = PartType::Fabricada;
    let total = p.comprimento_total();
    m.param("Ltotal", &total.to_string())?;

    // parâmetros por trecho (informativos; o perfil do revolve é fixo)
    let mut z0 = 0.;
    let mut inicios = vec![];
    for (i, t) in p.trechos.iter().enumerate() {
        let k = i + 1;
        m.param(&format!("d{k}"), &t.diametro.to_string())?;
        m.param(&format!("l{k}"), &t.comprimento.to_string())?;
        m.param(&format!("c{k}"), &t.chanfro_fim.to_string())?;
        if let (Some(r), Some(c)) = (t.rasgo, chavetas[i]) {
            m.param(&format!("kb{k}"), &c.b.to_string())?;
            m.param(&format!("kt{k}"), &c.t1.to_string())?;
            m.param(&format!("rl{k}"), &r.comprimento.to_string())?;
            m.param(&format!("ro{k}"), &r.desloc_inicio.to_string())?;
        }
        inicios.push(z0);
        z0 += t.comprimento;
    }

    // perfil (raio, z) do eixo, com os chanfros convexos
    let mut pts: Vec<[f64; 2]> = vec![[0., 0.]];
    fn poe(pts: &mut Vec<[f64; 2]>, q: [f64; 2]) {
        if pts.last().map_or(true, |u| (u[0] - q[0]).hypot(u[1] - q[1]) > 1e-9) {
            pts.push(q);
        }
    }
    for (i, t) in p.trechos.iter().enumerate() {
        let (r, z1, c) = (t.diametro / 2., inicios[i] + t.comprimento, t.chanfro_fim);
        if i == 0 {
            if c > 0. {
                poe(&mut pts, [r - c, 0.]);
                poe(&mut pts, [r, c]);
            } else {
                poe(&mut pts, [r, 0.]);
            }
        }
        let prox = p.trechos.get(i + 1).map(|n| n.diametro / 2.);
        if c > 0. && prox.map_or(true, |rn| rn < r) {
            poe(&mut pts, [r, z1 - c]);
            poe(&mut pts, [r - c, z1]);
        } else {
            poe(&mut pts, [r, z1]);
        }
        poe(&mut pts, [prox.unwrap_or(0.), z1]);
    }
    let mut data = SketchData::default();
    let mut al = || m.doc.alloc();
    let ids: Vec<Id> = pts.iter().map(|q| data.add_point(&mut al, q[0], q[1])).collect();
    for i in 0..ids.len() {
        data.add_curve(&mut al, Curve::Line { a: ids[i], b: ids[(i + 1) % ids.len()] }, false);
    }
    // perfil no XZ: x = raio, y = z
    let sk = m.sketch("Perfil_eixo", PlaneRef::Origin(OriginPlane::XZ), data);
    let rev = m.doc.add_feature(FeatureKind::Revolve(Revolve {
        sketch: sk,
        axis: AxisRef::Origin(OriginAxis::Z),
        angle: "360".into(),
        symmetric: false,
        op: BoolOp::Add,
        thin: None,
        thin_side: 0,
        angle2: None,
    }));
    nomear(&mut m, rev, "Eixo");

    // rasgos de chaveta em +X
    for (i, t) in p.trechos.iter().enumerate() {
        let (Some(r), Some(c)) = (t.rasgo, chavetas[i]) else { continue };
        let k = i + 1;
        let plano = plano_yz(&mut m, &format!("Plano_rasgo_{k}"), &format!("d{k}/2-kt{k}"))?;
        let za = inicios[i] + r.desloc_inicio;
        let sk = estadio(&mut m, &format!("Rasgo_{k}"), plano, [0., za + c.b / 2.], [0., za + r.comprimento - c.b / 2.], c.b / 2.);
        extrusao_cega(&mut m, &format!("Rasgo_chaveta_{k}"), sk, &format!("kt{k}+1"), BoolOp::Cut, true);
    }

    // furos de centro
    if p.furo_centro {
        let fim = p.trechos.len() - 1;
        for (nome, d, no_fim) in [("inicio", p.trechos[0].diametro, false), ("fim", p.trechos[fim].diametro, true)] {
            let dc = diametro_furo_centro(d);
            let prof = (1.5 * dc).min(total / 3.);
            let desvio = if no_fim { format!("Ltotal-{prof}") } else { "-1".to_string() };
            let pl = m.plano(&format!("Plano_centro_{nome}"), &desvio)?;
            m.prim(&format!("Furo_centro_{nome}"), PrimKind::Cilindro, pl, [&(dc / 2.).to_string(), &(prof + 1.).to_string(), "0", "0"], BoolOp::Cut)?;
        }
    }
    Ok(m.doc)
}

/// Comprimento da série padrão mais próximo de `l`; `None` fora da faixa da série.
pub fn comprimento_chaveta_da_serie(l: f64) -> Option<f64> {
    let s = comprimentos_chaveta();
    if !l.is_finite() || l < s[0] - 1e-9 || l > s[s.len() - 1] + 1e-9 {
        return None;
    }
    s.iter().copied().min_by(|a, b| (a - l).abs().total_cmp(&(b - l).abs()))
}

/// Maior comprimento da série que não passa de `l` (para a chaveta caber num rasgo de `l` mm).
pub fn comprimento_chaveta_ate(l: f64) -> Option<f64> {
    comprimentos_chaveta().iter().copied().filter(|x| *x <= l + 1e-9).last()
}

/// Documento da chaveta forma A para o diâmetro de eixo `d_eixo`; o comprimento é arredondado
/// para a série padrão.
pub fn chaveta_documento(d_eixo: f64, comprimento: f64) -> Result<Document, String> {
    let c = chaveta_para_eixo(d_eixo).ok_or_else(|| format!("Ø{d_eixo} está fora da tabela DIN 6885 (Ø6 a Ø85)"))?;
    let l = comprimento_chaveta_da_serie(comprimento).ok_or_else(|| {
        let s = comprimentos_chaveta();
        format!("comprimento {comprimento} fora da série da chaveta ({} a {} mm)", s[0], s[s.len() - 1])
    })?;
    if l <= c.b {
        return Err(format!("a chaveta {}x{} precisa de comprimento maior que {} mm", c.b, c.h, c.b));
    }
    let descricao = format!("Chaveta DIN 6885 A {}x{}x{}", c.b, c.h, l);
    let mut m = Montador::novo(&descricao);
    m.doc.props.material = "Aço 1045".into();
    m.param("b", &c.b.to_string())?;
    m.param("h", &c.h.to_string())?;
    m.param("L", &l.to_string())?;
    m.param("t1", &c.t1.to_string())?;
    m.param("t2", &c.t2.to_string())?;
    m.param("deixo", &d_eixo.to_string())?;
    let e = (l - c.b) / 2.;
    let sk = estadio(&mut m, "Perfil_chaveta", PlaneRef::Origin(OriginPlane::XY), [-e, 0.], [e, 0.], c.b / 2.);
    extrusao_cega(&mut m, "Chaveta", sk, "h", BoolOp::Add, false);
    Ok(m.doc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{recompute, Status};
    use std::f64::consts::PI;

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

    fn escalonado() -> ParamEixo {
        ParamEixo {
            trechos: vec![
                TrechoEixo { diametro: 20., comprimento: 50., chanfro_fim: 0., rasgo: None },
                TrechoEixo { diametro: 25., comprimento: 70., chanfro_fim: 0., rasgo: Some(RasgoChaveta { comprimento: 40., desloc_inicio: 10. }) },
            ],
            furo_centro: false,
        }
    }

    #[test]
    fn descricoes_e_parametros() {
        let d = eixo_documento(&escalonado()).unwrap();
        assert_eq!(d.props.description, "Eixo Ø20/Ø25 L120");
        for n in ["d1", "l2", "kb2", "kt2", "Ltotal"] {
            assert!(d.params.iter().any(|p| p.name == n), "{n}");
        }
        let c = chaveta_documento(20., 20.).unwrap();
        assert_eq!(c.props.description, "Chaveta DIN 6885 A 6x6x20");
        // comprimento fora da série é arredondado para a série
        assert_eq!(chaveta_documento(20., 21.).unwrap().props.description, "Chaveta DIN 6885 A 6x6x20");
        assert_ne!(ParamEixo::default().chave(), escalonado().chave());
    }

    #[test]
    fn entradas_invalidas() {
        let mut p = escalonado();
        p.trechos[1].diametro = 100.; // fora da tabela, com rasgo
        assert!(eixo_documento(&p).unwrap_err().contains("tabela"));
        let mut p = escalonado();
        p.trechos[1].rasgo = Some(RasgoChaveta { comprimento: 40., desloc_inicio: 40. }); // passa do trecho
        assert!(eixo_documento(&p).unwrap_err().contains("cabe"));
        let mut p = escalonado();
        p.trechos[0].chanfro_fim = 11.; // maior que o raio
        assert!(eixo_documento(&p).is_err());
        assert!(eixo_documento(&ParamEixo { trechos: vec![], furo_centro: false }).is_err());
        // sem rasgo, qualquer diâmetro serve
        let grande = ParamEixo { trechos: vec![TrechoEixo { diametro: 120., comprimento: 100., chanfro_fim: 2., rasgo: None }], furo_centro: false };
        assert!(eixo_documento(&grande).is_ok());
        assert!(chaveta_documento(100., 40.).is_err());
        assert!(chaveta_documento(20., 500.).is_err());
        assert!(chaveta_documento(20., f64::NAN).is_err());
        assert!(chaveta_documento(85., 6.).is_err()); // 22 mm de largura não cabe em 6
    }

    #[test]
    fn eixo_escalonado_com_rasgo_recalcula() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (v, lo, hi) = medir(&eixo_documento(&escalonado()).unwrap());
        let cheio = PI * (10_f64.powi(2) * 50. + 12.5_f64.powi(2) * 70.);
        assert!(v < cheio && v > 0.95 * cheio, "volume {v} vs cilindros {cheio}");
        assert!(lo[2].abs() < 1e-3 && (hi[2] - 120.).abs() < 1e-3, "z {lo:?} {hi:?}");
        assert!((hi[0] - 12.5).abs() < 0.01 && (lo[0] + 12.5).abs() < 0.01, "x {lo:?} {hi:?}");
        // o rasgo tira material do lado +X: no trecho dele só sobram vértices do piso (x = 8,5) e das paredes
        let r = recompute(&eixo_documento(&escalonado()).unwrap());
        let (mut xmax, mut xmin) = (f64::MIN, f64::MAX);
        for q in r.bodies[0].mesh.positions.chunks_exact(3).filter(|q| q[2] > 62. && q[2] < 98.) {
            xmax = xmax.max(q[0] as f64);
            xmin = xmin.min(q[0] as f64);
        }
        assert!(xmin > 8.4, "rasgo fora de +X: x entre {xmin} e {xmax} no trecho do rasgo");
        // sem o rasgo o volume é o dos cilindros
        let (v_sem, _, _) = medir(&eixo_documento(&ParamEixo { trechos: vec![escalonado().trechos[0], TrechoEixo { rasgo: None, ..escalonado().trechos[1] }], furo_centro: false }).unwrap());
        assert!((v_sem - cheio).abs() < cheio * 1e-3, "{v_sem} vs {cheio}");
    }

    #[test]
    fn chanfros_e_furos_de_centro_recalculam() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let base = ParamEixo { trechos: vec![TrechoEixo { diametro: 20., comprimento: 50., chanfro_fim: 0., rasgo: None }, TrechoEixo { diametro: 16., comprimento: 30., chanfro_fim: 0., rasgo: None }], furo_centro: false };
        let (v0, _, _) = medir(&eixo_documento(&base).unwrap());
        assert!((v0 - PI * (100. * 50. + 64. * 30.)).abs() < 1.);
        let mut ch = base.clone();
        ch.trechos[0].chanfro_fim = 1.5;
        ch.trechos[1].chanfro_fim = 1.;
        let (v1, _, hi) = medir(&eixo_documento(&ch).unwrap());
        assert!(v1 < v0 && v1 > 0.97 * v0 && (hi[2] - 80.).abs() < 1e-3, "{v1} {v0}");
        let (v2, _, _) = medir(&eixo_documento(&ParamEixo { furo_centro: true, ..base }).unwrap());
        assert!(v2 < v0 && v2 > 0.99 * v0, "{v2} {v0}");
    }

    #[test]
    fn chaveta_tem_o_volume_do_estadio() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (v, lo, hi) = medir(&chaveta_documento(25., 25.).unwrap());
        let esperado = (25. - 8.) * 8. * 7. + PI * 4. * 4. * 7.;
        assert!((v - esperado).abs() < esperado * 0.01, "volume {v} vs {esperado}");
        assert!((hi[0] - 12.5).abs() < 0.01 && (lo[0] + 12.5).abs() < 0.01 && (hi[1] - 4.).abs() < 0.01, "{lo:?} {hi:?}");
        assert!(lo[2].abs() < 1e-3 && (hi[2] - 7.).abs() < 1e-3);
    }
}
