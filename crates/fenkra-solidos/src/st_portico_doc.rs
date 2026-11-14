// SPDX-License-Identifier: LGPL-2.1-or-later
//! B114: apoios e cargas do pórtico guardados na peça, e a análise do documento inteiro.
//!
//! O modelo de barras nasce dos membros retos (`st_portico::modelo_da_estrutura`); o que falta para
//! analisar é dizer onde a estrutura se apoia e o que pesa nela. Isso fica nas propriedades
//! personalizadas da peça, para viajar com o arquivo sem mudar o formato:
//!
//! * `Pórtico · apoios`: `x,y,z=ux uy uz rx ry rz` com 1 nos graus presos, um nó por item, `;` entre
//!   itens. O nó é achado pela posição (a menos de 0,5 mm), então sobrevive a recálculos que não
//!   mexem na geometria.
//! * `Pórtico · cargas nos`: `x,y,z=Fx,Fy,Fz,Mx,My,Mz` (N e N·mm, no global).
//! * `Pórtico · cargas barras`: `<comando>:<linha>=qx,qy,qz` (N/mm, no global), aplicada em todos os
//!   trechos daquela linha.
//! * `Pórtico · peso próprio`: `1` liga o peso da barra (área × densidade do material × g, para −Z).
//!
//! Unidades: N, mm, MPa (o aço da análise é E = 200 GPa, G = 77 GPa).

use crate::st_portico::{analisar, modelo_da_estrutura, Portico, Resultado};
use fenkra_core::{Document, Id};

const APOIOS: &str = "Pórtico · apoios";
const CARGAS_NOS: &str = "Pórtico · cargas nos";
const CARGAS_BARRAS: &str = "Pórtico · cargas barras";
const PESO: &str = "Pórtico · peso próprio";
/// aceleração da gravidade (m/s²)
pub const G: f64 = 9.80665;

/// Apoios e cargas do pórtico.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CasoPortico {
    pub apoios: Vec<([f64; 3], [bool; 6])>,
    pub cargas_nos: Vec<([f64; 3], [f64; 6])>,
    pub cargas_barras: Vec<((Id, usize), [f64; 3])>,
    pub peso_proprio: bool,
}

fn nums(t: &str) -> Vec<f64> {
    t.split(',').map(|x| x.trim().parse::<f64>().unwrap_or(0.)).map(|x| if x.is_finite() { x } else { 0. }).collect()
}

fn ponto(t: &str) -> Option<[f64; 3]> {
    let v = nums(t);
    (v.len() == 3).then(|| [v[0], v[1], v[2]])
}

fn itens(t: &str) -> impl Iterator<Item = (&str, &str)> {
    t.split(';').filter_map(|x| x.split_once('='))
}

fn fmt(v: &[f64]) -> String {
    v.iter().map(|x| format!("{x}")).collect::<Vec<_>>().join(",")
}

impl CasoPortico {
    pub fn ler(doc: &Document) -> CasoPortico {
        let prop = |k: &str| doc.props.custom.iter().find(|(c, _)| c == k).map(|(_, v)| v.as_str()).unwrap_or("");
        let mut c = CasoPortico { peso_proprio: prop(PESO).trim() == "1", ..Default::default() };
        for (p, g) in itens(prop(APOIOS)) {
            let Some(p) = ponto(p) else { continue };
            let b: Vec<bool> = g.split_whitespace().map(|x| x == "1").collect();
            if b.len() == 6 {
                c.apoios.push((p, [b[0], b[1], b[2], b[3], b[4], b[5]]));
            }
        }
        for (p, f) in itens(prop(CARGAS_NOS)) {
            let (Some(p), f) = (ponto(p), nums(f)) else { continue };
            if f.len() == 6 {
                c.cargas_nos.push((p, [f[0], f[1], f[2], f[3], f[4], f[5]]));
            }
        }
        for (b, q) in itens(prop(CARGAS_BARRAS)) {
            let Some((id, linha)) = b.split_once(':') else { continue };
            let (Ok(id), Ok(linha), q) = (id.trim().parse::<u64>(), linha.trim().parse::<usize>(), nums(q)) else { continue };
            if q.len() == 3 {
                c.cargas_barras.push(((Id(id), linha), [q[0], q[1], q[2]]));
            }
        }
        c
    }

    pub fn gravar(&self, doc: &mut Document) {
        doc.props.custom.retain(|(k, _)| ![APOIOS, CARGAS_NOS, CARGAS_BARRAS, PESO].contains(&k.as_str()));
        let mut put = |k: &str, v: String| {
            if !v.is_empty() {
                doc.props.custom.push((k.to_string(), v));
            }
        };
        put(APOIOS, self.apoios.iter().map(|(p, g)| format!("{}={}", fmt(p), g.iter().map(|&x| if x { "1" } else { "0" }).collect::<Vec<_>>().join(" "))).collect::<Vec<_>>().join(";"));
        put(CARGAS_NOS, self.cargas_nos.iter().map(|(p, f)| format!("{}={}", fmt(p), fmt(f))).collect::<Vec<_>>().join(";"));
        put(CARGAS_BARRAS, self.cargas_barras.iter().map(|((id, l), q)| format!("{}:{}={}", id.0, l, fmt(q))).collect::<Vec<_>>().join(";"));
        if self.peso_proprio {
            put(PESO, "1".into());
        }
    }

    /// Apoio do nó (graus presos), se houver.
    pub fn apoio_em(&self, p: [f64; 3]) -> Option<[bool; 6]> {
        self.apoios.iter().find(|(q, _)| crate::geo3d::dist(*q, p) < 0.5).map(|x| x.1)
    }

    pub fn por_apoio(&mut self, p: [f64; 3], g: Option<[bool; 6]>) {
        self.apoios.retain(|(q, _)| crate::geo3d::dist(*q, p) >= 0.5);
        if let Some(g) = g.filter(|g| g.iter().any(|&x| x)) {
            self.apoios.push((p, g));
        }
    }

    pub fn carga_em(&self, p: [f64; 3]) -> [f64; 6] {
        self.cargas_nos.iter().find(|(q, _)| crate::geo3d::dist(*q, p) < 0.5).map(|x| x.1).unwrap_or_default()
    }

    pub fn por_carga(&mut self, p: [f64; 3], f: [f64; 6]) {
        self.cargas_nos.retain(|(q, _)| crate::geo3d::dist(*q, p) >= 0.5);
        if f.iter().any(|x| x.abs() > 0.) {
            self.cargas_nos.push((p, f));
        }
    }

    pub fn q_da_linha(&self, l: (Id, usize)) -> [f64; 3] {
        self.cargas_barras.iter().find(|(k, _)| *k == l).map(|x| x.1).unwrap_or_default()
    }

    pub fn por_q(&mut self, l: (Id, usize), q: [f64; 3]) {
        self.cargas_barras.retain(|(k, _)| *k != l);
        if q.iter().any(|x| x.abs() > 0.) {
            self.cargas_barras.push((l, q));
        }
    }
}

/// O pórtico do documento com os apoios e cargas guardados, e a origem de cada barra.
pub fn portico_do_documento(doc: &Document) -> Result<(Portico, Vec<(Id, usize)>), String> {
    let (mut p, origem) = modelo_da_estrutura(doc)?;
    let caso = CasoPortico::ler(doc);
    for n in &mut p.nos {
        if let Some(g) = caso.apoio_em(n.p) {
            n.apoio = g;
        }
        n.carga = caso.carga_em(n.p);
    }
    for (b, o) in p.barras.iter_mut().zip(&origem) {
        let q = caso.q_da_linha(*o);
        let mut peso = 0.;
        if caso.peso_proprio {
            let material = match doc.feature(o.0).map(|f| &f.kind) {
                Some(fenkra_core::FeatureKind::StructMember(m)) if !m.material.trim().is_empty() => m.material.clone(),
                _ => doc.props.material.clone(),
            };
            // g/cm³ → kg/mm³ é 1e-6
            peso = b.secao.area * fenkra_core::materiais::densidade(&material) * 1e-6 * G;
        }
        b.q = [q[0], q[1], q[2] - peso];
    }
    Ok((p, origem))
}

/// Analisa o documento: o pórtico, a origem de cada barra e o resultado. Sem apoio é recusado com
/// motivo antes de montar a matriz.
pub fn analisar_documento(doc: &Document) -> Result<(Portico, Vec<(Id, usize)>, Resultado), String> {
    let (p, origem) = portico_do_documento(doc)?;
    if p.nos.iter().all(|n| n.apoio.iter().all(|x| !x)) {
        return Err("sem apoio: marque ao menos um nó como apoio".into());
    }
    let r = analisar(&p)?;
    Ok((p, origem, r))
}

/// Seções de aço (B115) de cada barra, pelas cotas do membro de origem, com o aço `fy` (MPa).
pub fn secoes_aco(doc: &Document, origem: &[(Id, usize)], fy: f64) -> Result<Vec<crate::st_verifica_aco::SecaoAco>, String> {
    use crate::st_verifica_aco as va;
    use fenkra_core::SectionKind::*;
    let vars = doc.eval_params().map_err(|e| e.to_string())?;
    let e = crate::st_portico::E_ACO;
    origem
        .iter()
        .map(|(id, _)| {
            let Some(fenkra_core::FeatureKind::StructMember(m)) = doc.feature(*id).map(|f| &f.kind) else { return Err("barra sem membro de origem".to_string()) };
            let d: Vec<f64> = m.dims.iter().map(|x| fenkra_core::expr::eval_str(x.trim(), &vars).map_err(|e| e.to_string())).collect::<Result<_, _>>()?;
            let g = |i: usize| d.get(i).copied().ok_or_else(|| "cota da seção faltando".to_string());
            Ok(match m.kind {
                TuboRedondo => va::tubo_redondo(g(0)?, g(1)?, e, fy),
                TuboQuadrado => va::tubo_retangular(g(0)?, g(0)?, g(1)?, e, fy),
                TuboRetangular => va::tubo_retangular(g(0)?, g(1)?, g(2)?, e, fy),
                PerfilI => va::perfil_i(g(0)?, g(1)?, g(2)?, g(3)?, e, fy),
                k => return Err(format!("seção {} ainda não entra na verificação", crate::structural::kind_label(k))),
            })
        })
        .collect()
}

#[cfg(test)]
mod testes {
    use super::*;
    use fenkra_core::*;

    fn s(x: &str) -> String {
        x.to_string()
    }

    /// Viga de tubo quadrado 100x100x5 de 4 m, horizontal, sobre dois nós.
    fn viga() -> (Document, Id) {
        let mut d = Document::new("viga");
        let l = CurveRef::Ref(d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed([0., 0., 0.]), b: PointRef::Fixed([4000., 0., 0.]), start: s("0"), end: s("0") }));
        let id = d.add_feature(FeatureKind::StructMember(StructMember {
            paths: vec![l],
            family: s("Tubo quadrado"),
            designation: s("100x5"),
            kind: SectionKind::TuboQuadrado,
            dims: vec![s("100"), s("5")],
            anchor: 0,
            angle: s("0"),
            off_x: s("0"),
            off_y: s("0"),
            ext1: s("0"),
            ext2: s("0"),
            corner: CornerMode::Nenhum,
            gap: s("0"),
            joints: vec![],
            material: String::new(),
            mirror: false,
        }));
        (d, id)
    }

    #[test]
    fn caso_vai_e_volta_pelas_propriedades() {
        let (mut d, id) = viga();
        let mut c = CasoPortico::default();
        c.por_apoio([0., 0., 0.], Some([true, true, true, false, false, false]));
        c.por_carga([2000., 0., 0.], [0., 0., -1000., 0., 0., 0.]);
        c.por_q((id, 0), [0., 0., -2.5]);
        c.peso_proprio = true;
        c.gravar(&mut d);
        assert_eq!(CasoPortico::ler(&d), c);
        // tirar tudo limpa as propriedades
        CasoPortico::default().gravar(&mut d);
        assert!(d.props.custom.is_empty());
    }

    #[test]
    fn viga_biapoiada_do_documento_bate_com_a_conta_a_mao() {
        let (mut d, id) = viga();
        let r = crate::recompute(&d);
        assert!(r.status.values().all(|s| *s == crate::Status::Ok));
        // o sólido da viga: 4 m de 100x100x5
        assert_eq!(r.bodies.len(), 1);
        if fenkra_kernel::AVAILABLE {
            let g = crate::structural_groups(&d);
            let corpo = &g[0].shape;
            let area = 100. * 100. - 90. * 90.;
            assert!((corpo.mass().volume - area * 4000.).abs() / (area * 4000.) < 1e-6);
            // referencial local: Z ao longo da barra
            let (lo, hi) = corpo.bbox().unwrap();
            assert!((hi[2] - lo[2] - 4000.).abs() < 0.01 && (hi[1] - lo[1] - 100.).abs() < 0.01 && (hi[0] - lo[0] - 100.).abs() < 0.01, "{lo:?} {hi:?}");
        }
        // sem apoio: recusado com motivo
        assert!(analisar_documento(&d).unwrap_err().contains("sem apoio"));
        let mut c = CasoPortico::default();
        c.por_apoio([0., 0., 0.], Some([true, true, true, true, false, false]));
        c.por_apoio([4000., 0., 0.], Some([false, true, true, false, false, false]));
        c.por_q((id, 0), [0., 0., -2.]);
        c.gravar(&mut d);
        let (p, origem, res) = analisar_documento(&d).unwrap();
        assert_eq!(origem, vec![(id, 0)]);
        assert_eq!(p.nos.len(), 2);
        // q = 2 N/mm, L = 4000 mm: M = qL²/8 = 4 000 000 N·mm; V = qL/2 = 4000 N
        let b = &res.barras[0];
        let m = b.maximo(|e| e.my).max(b.maximo(|e| e.mz));
        assert!((m - 4.0e6).abs() / 4.0e6 < 1e-6, "{m}");
        assert!((b.maximo(|e| e.vy).max(b.maximo(|e| e.vz)) - 4000.).abs() < 1e-6);
        // flecha 5qL⁴/(384EI), I = (100⁴ − 90⁴)/12 = 2 865 833 mm⁴ → 11,63 mm
        let i = (100f64.powi(4) - 90f64.powi(4)) / 12.;
        let f = 5. * 2. * 4000f64.powi(4) / (384. * 200_000. * i);
        let flecha = b.maximo(|e| e.v).max(b.maximo(|e| e.w));
        assert!((flecha - f).abs() / f < 1e-3, "{flecha} {f}");
        // reações: metade da carga em cada apoio, para cima
        let rz: f64 = res.reacoes.iter().map(|r| r[2]).sum();
        assert!((rz - 8000.).abs() < 1e-6, "{rz}");
    }

    #[test]
    fn peso_proprio_usa_a_densidade_do_material() {
        let (mut d, _) = viga();
        d.props.material = s("Aço 1020");
        let mut c = CasoPortico { peso_proprio: true, ..Default::default() };
        c.por_apoio([0., 0., 0.], Some([true; 6]));
        c.gravar(&mut d);
        let (p, _, res) = analisar_documento(&d).unwrap();
        // balanço de 4 m com o próprio peso: q = A·ρ·g; reação = q·L
        let rho = fenkra_core::materiais::densidade("Aço 1020");
        let q = 1900. * rho * 1e-6 * G;
        assert!((p.barras[0].q[2] + q).abs() < 1e-12);
        assert!((res.reacoes[0][2] - q * 4000.).abs() < 1e-6, "{:?}", res.reacoes[0]);
        // momento no engaste qL²/2
        let m = res.barras[0].maximo(|e| e.my).max(res.barras[0].maximo(|e| e.mz));
        assert!((m - q * 4000f64.powi(2) / 2.).abs() / m < 1e-6);
    }
}
