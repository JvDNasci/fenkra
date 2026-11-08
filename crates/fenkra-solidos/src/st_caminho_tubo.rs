// SPDX-License-Identifier: LGPL-2.1-or-later
//! Caminho de corte em tubo para laser ou plasma de tubo rotativo (B163), a partir da curva de
//! encaixe já calculada (`st_encaixe`, B155).
//!
//! Como funciona: a máquina de tubo prende o tubo num mandril que gira (eixo A) e avança (eixo X);
//! a tocha fica parada em cima e aponta para o eixo do tubo. Cada ponto da curva de encaixe vira um
//! ponto de máquina: `x` é a posição ao longo do tubo (a partir do início da barra), `a` é o giro do
//! mandril que traz aquele ponto para baixo da tocha e o vetor da tocha é a direção em que ela
//! aponta (radial para dentro, inclinada pelo chanfro quando há).
//!
//! O caminho de cada ponta é: entrada (lead-in) pelo lado da sobra, a volta inteira da curva com o
//! deslocamento do kerf para o lado da sobra, saída (lead-out) passando um pouco do ponto de início
//! para fechar o corte, e as microjuntas como trechos em que a tocha não corta. A ordem é: ponta 1
//! primeiro e ponta 2 por último, porque o corte da ponta 2 separa a peça da barra.
//!
//! Fica de fora (fase posterior, como diz o item): colisão da tocha com o perfil, corte em linha
//! comum entre peças e G-code de máquina; a saída é CSV com a tabela por ponto e o relatório de
//! comprimento e tempo.

use crate::geo3d::{self, V3};
use crate::st_encaixe::CorteEncaixe;
use crate::st_encaixe_doc::{MembroComEncaixe, PontaEncaixe};

/// Ajustes do caminho. O tempo vem da velocidade de corte e do tempo de perfuração, que dependem
/// da máquina, do material e da espessura: são do usuário.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OpcoesCaminho {
    /// largura do sulco de corte (mm); o caminho anda kerf/2 para o lado da sobra
    pub kerf: f64,
    /// comprimento da entrada, pelo lado da sobra (mm)
    pub entrada: f64,
    /// quanto a saída passa do ponto de início (mm)
    pub saida: f64,
    /// microjuntas por ponta (0 = nenhuma), espalhadas por igual no perímetro
    pub microjuntas: usize,
    /// comprimento de cada microjunta (mm)
    pub microjunta: f64,
    /// velocidade de corte (mm/min)
    pub velocidade: f64,
    /// tempo de cada perfuração (s)
    pub perfuracao: f64,
    /// percorre a curva no sentido de `s` decrescente
    pub inverter: bool,
}

impl Default for OpcoesCaminho {
    fn default() -> Self {
        OpcoesCaminho { kerf: 0.2, entrada: 3., saida: 1., microjuntas: 0, microjunta: 0.5, velocidade: 2000., perfuracao: 0.5, inverter: false }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Etapa {
    Entrada,
    Corte,
    Saida,
}

/// Um ponto de máquina.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PontoTocha {
    /// posição ao longo do tubo, a partir do nó da ponta 1 (mm)
    pub x: f64,
    /// giro do mandril (graus, contínuo: passa de 360 sem voltar a zero)
    pub a: f64,
    /// posição no perímetro externo desenrolado (mm)
    pub s: f64,
    /// chanfro (graus; 0 = tocha perpendicular à parede)
    pub bisel: f64,
    /// direção em que a tocha aponta, no mundo
    pub tocha: V3,
    /// ponto da pele externa no mundo
    pub p3: V3,
    pub etapa: Etapa,
    /// falso nas microjuntas (a tocha passa sem cortar)
    pub corta: bool,
}

/// O corte de uma ponta.
#[derive(Clone, Debug, PartialEq)]
pub struct Operacao {
    pub nome: String,
    /// esta operação separa a peça da barra
    pub separa: bool,
    pub pontos: Vec<PontoTocha>,
}

impl Operacao {
    fn trechos(&self) -> impl Iterator<Item = (&PontoTocha, &PontoTocha)> {
        self.pontos.windows(2).map(|w| (&w[0], &w[1]))
    }
    /// Comprimento com a tocha cortando (entrada, curva e saída, sem as microjuntas), em mm, medido
    /// na pele externa desenrolada.
    pub fn comprimento_cortado(&self) -> f64 {
        self.trechos().filter(|(a, b)| a.corta && b.corta).map(|(a, b)| (b.s - a.s).hypot(b.x - a.x)).sum()
    }
    /// Perfurações: uma no início e uma depois de cada microjunta.
    pub fn perfuracoes(&self) -> usize {
        1 + self.trechos().filter(|(a, b)| !a.corta && b.corta).count()
    }
}

/// O caminho de uma peça: operações na ordem da máquina.
#[derive(Clone, Debug, PartialEq)]
pub struct CaminhoTubo {
    pub titulo: String,
    pub comprimento: f64,
    pub operacoes: Vec<Operacao>,
    pub opc: OpcoesCaminho,
}

impl CaminhoTubo {
    pub fn comprimento_cortado(&self) -> f64 {
        self.operacoes.iter().map(|o| o.comprimento_cortado()).sum()
    }
    pub fn perfuracoes(&self) -> usize {
        self.operacoes.iter().map(|o| o.perfuracoes()).sum()
    }
    /// Tempo estimado (s): corte à velocidade dada mais as perfurações. Não conta giro, avanço nem
    /// carga e descarga.
    pub fn tempo(&self) -> f64 {
        self.comprimento_cortado() / self.opc.velocidade.max(1e-9) * 60. + self.perfuracoes() as f64 * self.opc.perfuracao
    }

    /// Tabela por ponto em CSV (separador `;`, ponto decimal).
    pub fn csv(&self) -> String {
        let mut t = String::from("operacao;ordem;etapa;x_mm;a_graus;s_mm;bisel_graus;tocha_x;tocha_y;tocha_z;corta\n");
        for (k, o) in self.operacoes.iter().enumerate() {
            for p in &o.pontos {
                let etapa = match p.etapa {
                    Etapa::Entrada => "entrada",
                    Etapa::Corte => "corte",
                    Etapa::Saida => "saida",
                };
                t += &format!("{};{};{etapa};{:.3};{:.3};{:.3};{:.2};{:.5};{:.5};{:.5};{}\n", o.nome, k + 1, p.x, p.a, p.s, p.bisel, p.tocha[0], p.tocha[1], p.tocha[2], if p.corta { 1 } else { 0 });
            }
        }
        t
    }

    /// Relatório curto de comprimento e tempo.
    pub fn relatorio(&self) -> String {
        let mut t = format!("{}\nbarra {:.1} mm, {} operação(ões), corte final: {}\n", self.titulo, self.comprimento, self.operacoes.len(), self.operacoes.iter().find(|o| o.separa).map(|o| o.nome.as_str()).unwrap_or("-"));
        for (k, o) in self.operacoes.iter().enumerate() {
            t += &format!("{}. {}: {:.1} mm cortados, {} perfuração(ões)\n", k + 1, o.nome, o.comprimento_cortado(), o.perfuracoes());
        }
        t += &format!(
            "total: {:.1} mm, {} perfurações, {:.1} s (a {:.0} mm/min e {:.2} s por perfuração; kerf {:.2} mm)\n",
            self.comprimento_cortado(),
            self.perfuracoes(),
            self.tempo(),
            self.opc.velocidade,
            self.opc.perfuracao,
            self.opc.kerf
        );
        t
    }
}

/// Monta o caminho de uma ponta. `inicio` = ponta 1 (a sobra fica antes do nó, `x` menor).
fn operacao(nome: &str, p: &PontaEncaixe, comprimento: f64, inicio: bool, separa: bool, opc: &OpcoesCaminho) -> Option<Operacao> {
    let c: &CorteEncaixe = &p.corte;
    let n = c.pontos.len();
    if n < 3 {
        return None;
    }
    let eixo = geo3d::unit(p.dir)?;
    // x ao longo da barra: a ponta 1 mede do nó para dentro; a ponta 2, do nó da ponta 1
    let x_de = |lambda: f64| if inicio { lambda } else { comprimento - lambda };
    // referencial do giro: o radial do primeiro ponto (s = 0) e o perpendicular no sentido de s
    let radial = |q: V3| geo3d::unit(geo3d::sub(geo3d::sub(q, p.no), geo3d::mul(eixo, geo3d::dot(geo3d::sub(q, p.no), eixo))));
    let u0 = radial(c.pontos[0].p3)?;
    let mut v0 = geo3d::unit(fenkra_kernel::cross(eixo, u0))?;
    if let Some(r1) = radial(c.pontos[1].p3) {
        if geo3d::dot(r1, v0) < 0. {
            v0 = geo3d::mul(v0, -1.);
        }
    }
    // sobra: do lado do nó (x menor na ponta 1, maior na ponta 2)
    let lado_sobra = if inicio { -1. } else { 1. };
    let ordem: Vec<usize> = if opc.inverter { (0..n).rev().collect() } else { (0..n).collect() };
    let per = c.perimetro;
    // curva desenrolada com o kerf: deslocamento na normal do desenho (s, x), para o lado da sobra
    let base: Vec<(f64, f64)> = ordem.iter().map(|&i| (c.pontos[i].s, x_de(c.pontos[i].lambda))).collect();
    let mut cam: Vec<PontoTocha> = vec![];
    let mut a_ant: Option<f64> = None;
    let mut s_acum = 0.;
    for (k, &i) in ordem.iter().enumerate() {
        let q = &c.pontos[i];
        let (sp, xp) = base[(k + n - 1) % n];
        let (sn, xn) = base[(k + 1) % n];
        // derivada no perímetro, tratando a costura (s volta a zero)
        let mut ds = sn - sp;
        if ds.abs() > per / 2. {
            ds -= per * ds.signum();
        }
        let dx = xn - xp;
        let l = ds.hypot(dx).max(1e-12);
        // normal do desenho apontando para o lado da sobra (componente x com o sinal da sobra)
        let mut nx = ds / l;
        if nx * lado_sobra < 0. {
            nx = -nx;
        }
        let x = base[k].1 + nx * opc.kerf / 2.;
        let s = if k == 0 { base[0].0 } else { s_acum };
        let r = radial(q.p3).unwrap_or(u0);
        let mut a = geo3d::dot(r, v0).atan2(geo3d::dot(r, u0)).to_degrees();
        if let Some(ap) = a_ant {
            while a - ap > 180. {
                a -= 360.;
            }
            while a - ap < -180. {
                a += 360.;
            }
        }
        a_ant = Some(a);
        // tocha para dentro; o chanfro a inclina ao longo do eixo, para o lado da peça
        let b = q.bisel.to_radians();
        let para_peca = geo3d::mul(eixo, if inicio { 1. } else { -1. });
        let tocha = geo3d::unit(geo3d::add(geo3d::mul(r, -b.cos()), geo3d::mul(para_peca, b.sin()))).unwrap_or(geo3d::mul(r, -1.));
        cam.push(PontoTocha { x, a, s, bisel: q.bisel, tocha, p3: q.p3, etapa: Etapa::Corte, corta: true });
        if k + 1 < n {
            let mut d = base[k + 1].0 - base[k].0;
            if d.abs() > per / 2. {
                d -= per * d.signum();
            }
            s_acum = s + d;
        }
    }
    // fecha a volta e passa do início pela saída
    let primeiro = cam[0];
    let ult = *cam.last()?;
    let mut d_fecha = primeiro.s + per * if cam.len() > 1 && cam[1].s < primeiro.s { -1. } else { 1. } - ult.s;
    if d_fecha.abs() > per / 2. {
        d_fecha -= per * d_fecha.signum();
    }
    let volta = if ult.a >= cam[0].a { 360. } else { -360. };
    cam.push(PontoTocha { s: ult.s + d_fecha, a: primeiro.a + volta, ..primeiro });
    // saída: continua sobre a curva por `saida` mm
    let mut falta = opc.saida;
    let mut k = 1;
    while falta > 1e-9 && k < cam.len() - 1 {
        let (a, b) = (cam[k - 1], cam[k]);
        let seg = (b.s - a.s).hypot(b.x - a.x).max(1e-12);
        let t = (falta / seg).min(1.);
        let ult = *cam.last().unwrap();
        let ds = b.s - a.s;
        let da = b.a - a.a;
        cam.push(PontoTocha { s: ult.s + ds * t, x: a.x + (b.x - a.x) * t, a: ult.a + da * t, etapa: Etapa::Saida, ..b });
        falta -= seg * t;
        k += 1;
    }
    // entrada: reta vinda da sobra até o início
    if opc.entrada > 0. {
        let e = PontoTocha { x: primeiro.x + lado_sobra * opc.entrada, etapa: Etapa::Entrada, ..primeiro };
        cam.insert(0, e);
    }
    // microjuntas: trechos sem corte espalhados no perímetro (fora da entrada e da saída)
    if opc.microjuntas > 0 && opc.microjunta > 0. {
        let s0 = cam.iter().find(|p| p.etapa == Etapa::Corte).map(|p| p.s).unwrap_or(0.);
        let passo = per / opc.microjuntas as f64;
        for p in cam.iter_mut().filter(|p| p.etapa == Etapa::Corte) {
            let u = (p.s - s0).abs();
            if (0..opc.microjuntas).any(|j| (u - passo * (j as f64 + 0.5)).abs() < opc.microjunta / 2.) {
                p.corta = false;
            }
        }
    }
    Some(Operacao { nome: nome.into(), separa, pontos: cam })
}

/// Caminho de corte de um membro com encaixes: ponta 1, depois ponta 2 (o corte que separa).
pub fn caminho_do_membro(m: &MembroComEncaixe, opc: &OpcoesCaminho) -> Option<CaminhoTubo> {
    let mut ops = vec![];
    if let Some(p) = &m.ponta1 {
        ops.push(operacao("ponta 1 (encaixe)", p, m.comprimento, true, m.ponta2.is_none(), opc)?);
    }
    if let Some(p) = &m.ponta2 {
        ops.push(operacao("ponta 2 (encaixe)", p, m.comprimento, false, true, opc)?);
    }
    if ops.is_empty() {
        return None;
    }
    Some(CaminhoTubo { titulo: format!("Caminho de corte · {} · linha {}", m.designacao, m.linha + 1), comprimento: m.comprimento, operacoes: ops, opc: *opc })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::st_encaixe::Opcoes;
    use crate::st_encaixe_doc::encaixes_da_estrutura;
    use fenkra_core::*;

    fn s(x: &str) -> String {
        x.to_string()
    }

    /// Travessa redonda 60,3 entre dois montantes iguais: encaixe nas duas pontas.
    fn travessa() -> MembroComEncaixe {
        let mut d = Document::new("t");
        let mut l = |a: V3, b: V3| CurveRef::Ref(d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed(a), b: PointRef::Fixed(b), start: s("0"), end: s("0") }));
        let paths = vec![l([0., 0., 0.], [0., 0., 600.]), l([800., 0., 0.], [800., 0., 600.]), l([0., 0., 300.], [800., 0., 300.])];
        d.add_feature(FeatureKind::StructMember(StructMember {
            paths,
            family: s("Tubo redondo"),
            designation: s("60,3x3,6"),
            kind: SectionKind::TuboRedondo,
            dims: vec![s("60.3"), s("3.6")],
            anchor: 0,
            angle: s("0"),
            off_x: s("0"),
            off_y: s("0"),
            ext1: s("0"),
            ext2: s("0"),
            corner: CornerMode::Encaixe,
            gap: s("0"),
            joints: vec![],
            material: String::new(),
            mirror: false,
        }));
        let r = encaixes_da_estrutura(&d, &Opcoes::default());
        r.into_iter().find(|m| m.ponta1.is_some() && m.ponta2.is_some()).expect("travessa com duas pontas de encaixe")
    }

    #[test]
    fn sem_kerf_o_caminho_segue_a_curva_e_da_uma_volta() {
        let m = travessa();
        let opc = OpcoesCaminho { kerf: 0., entrada: 0., saida: 0., ..Default::default() };
        let c = caminho_do_membro(&m, &opc).unwrap();
        assert_eq!(c.operacoes.len(), 2);
        assert!(!c.operacoes[0].separa && c.operacoes[1].separa, "a ponta 2 separa a peça");
        let o = &c.operacoes[0];
        let p1 = &m.ponta1.as_ref().unwrap().corte;
        // x = λ na ponta 1, e a volta completa dá o comprimento de corte da curva
        for (q, p) in o.pontos.iter().zip(&p1.pontos) {
            assert!((q.x - p.lambda).abs() < 1e-9);
        }
        let fecho = p1.comprimento_corte();
        assert!(o.comprimento_cortado() >= fecho && o.comprimento_cortado() - fecho < 5., "{} {}", o.comprimento_cortado(), fecho);
        // o mandril gira uma volta
        let a = o.pontos.last().unwrap().a - o.pontos[0].a;
        assert!((a.abs() - 360.).abs() < 1e-6, "giro {a}");
        // ponta 2: x medido do início da barra
        let o2 = &c.operacoes[1];
        assert!(o2.pontos.iter().all(|p| p.x > m.comprimento - 40. && p.x <= m.comprimento + 1e-9));
        // a tocha é unitária, para dentro e perpendicular ao eixo (sem chanfro)
        for p in &o.pontos {
            let t = p.tocha;
            assert!((geo3d::norm(t) - 1.).abs() < 1e-9);
            assert!(geo3d::dot(t, [1., 0., 0.]).abs() < 1e-6, "{t:?}");
        }
    }

    #[test]
    fn kerf_entrada_saida_microjuntas_e_tempo() {
        let m = travessa();
        let base = caminho_do_membro(&m, &OpcoesCaminho { kerf: 0., entrada: 0., saida: 0., ..Default::default() }).unwrap();
        let opc = OpcoesCaminho { kerf: 0.4, entrada: 3., saida: 2., microjuntas: 2, microjunta: 1., velocidade: 1200., perfuracao: 0.8, inverter: false };
        let c = caminho_do_membro(&m, &opc).unwrap();
        let (o, o0) = (&c.operacoes[0], &base.operacoes[0]);
        // kerf: a ponta 1 anda para o lado da sobra (x menor), no máximo kerf/2
        let cortes: Vec<&PontoTocha> = o.pontos.iter().filter(|p| p.etapa == Etapa::Corte).collect();
        for (q, p) in cortes.iter().zip(&o0.pontos) {
            assert!(q.x <= p.x + 1e-9 && p.x - q.x <= 0.2 + 1e-9, "{} {}", q.x, p.x);
        }
        // ponta 2: a sobra fica depois
        let c2: Vec<&PontoTocha> = c.operacoes[1].pontos.iter().filter(|p| p.etapa == Etapa::Corte).collect();
        for (q, p) in c2.iter().zip(&base.operacoes[1].pontos) {
            assert!(q.x >= p.x - 1e-9);
        }
        // entrada vem da sobra e a saída existe
        assert_eq!(o.pontos[0].etapa, Etapa::Entrada);
        assert!((o.pontos[1].x - o.pontos[0].x - 3.).abs() < 1e-9);
        assert!(o.pontos.iter().any(|p| p.etapa == Etapa::Saida));
        // duas microjuntas: três perfurações por ponta (início e depois de cada uma)
        assert_eq!(o.perfuracoes(), 3, "{}", o.perfuracoes());
        assert!(o.pontos.iter().any(|p| !p.corta));
        // tempo = comprimento / velocidade + perfurações
        let t = c.comprimento_cortado() / 1200. * 60. + c.perfuracoes() as f64 * 0.8;
        assert!((c.tempo() - t).abs() < 1e-9);
        // o CSV tem uma linha por ponto e o relatório fala do corte final
        let n: usize = c.operacoes.iter().map(|o| o.pontos.len()).sum();
        assert_eq!(c.csv().lines().count(), n + 1);
        assert!(c.relatorio().contains("corte final: ponta 2"));
    }
}
