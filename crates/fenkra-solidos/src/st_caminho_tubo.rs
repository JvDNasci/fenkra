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
//! O início de cada corte pode ser girado (`inicio`, em graus a partir da costura do perímetro),
//! para levar a perfuração a uma face que vai para a sucata ou a um lado sem solda. O ponto 3D de
//! cada passo acompanha o kerf e a entrada: o deslocamento em `x` vira deslocamento ao longo do eixo
//! do tubo, então a linha no 3D é a que a tocha percorre, não a curva nominal.
//!
//! Fica de fora (fase posterior, como diz o item): colisão da tocha com o perfil, corte em linha
//! comum entre peças e G-code de máquina; a saída é CSV com a tabela por ponto, o relatório de
//! comprimento e tempo e o DXF da vista desenvolvida (`desenvolvido`).

use crate::geo3d::{self, V3};
use crate::st_dobra_corte::{Camada, Desenho};
use crate::st_tubo_real::TuboReal;
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
    /// onde o corte começa, em graus no perímetro a partir da costura (0 = primeiro ponto da curva)
    pub inicio: f64,
}

impl Default for OpcoesCaminho {
    fn default() -> Self {
        OpcoesCaminho { kerf: 0.2, entrada: 3., saida: 1., microjuntas: 0, microjunta: 0.5, velocidade: 2000., perfuracao: 0.5, inverter: false, inicio: 0. }
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
    /// ponto da pele externa no mundo, já com o kerf e a entrada (onde a tocha passa)
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
    /// perímetro externo da seção (altura da vista desenvolvida)
    pub perimetro: f64,
    pub operacoes: Vec<Operacao>,
    pub opc: OpcoesCaminho,
    /// medidas reais do lote (B176); a costura é o zero da vista desenvolvida
    pub real: TuboReal,
    /// seção da curva (já com diâmetro e parede reais)
    pub secao: crate::st_encaixe::Secao,
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

    /// Vista desenvolvida para DXF ou SVG: o tubo aberto (X ao longo da barra, Y no perímetro
    /// externo, de 0 na costura até o perímetro) em `CONTORNO`; o caminho que a tocha corta, com o
    /// kerf, a entrada e a saída, em `CORTE`; as microjuntas (onde a tocha passa sem cortar) em
    /// `CONTORNO` tracejado; e em `COTA` o número de cada operação no ponto de perfuração, uma seta
    /// do sentido e o resumo. O caminho que cruza a costura é partido em dois traços.
    /// Altura na vista desenvolvida: posição no perímetro a partir da costura, entre 0 e o perímetro.
    pub fn y_de(&self, s: f64) -> f64 {
        let per = self.perimetro.max(1e-9);
        (s - self.real.costura / 360. * per).rem_euclid(per)
    }

    pub fn desenvolvido(&self) -> Desenho {
        let mut d = Desenho { titulo: self.titulo.clone(), ..Default::default() };
        let per = self.perimetro.max(1e-9);
        let (xmin, xmax) = self.operacoes.iter().flat_map(|o| o.pontos.iter()).fold((0f64, self.comprimento), |(a, b), p| (a.min(p.x), b.max(p.x)));
        d.retangulo(Camada::Contorno, xmin, 0., xmax, per);
        for (k, o) in self.operacoes.iter().enumerate() {
            // traços abertos: (cortando, pontos)
            let mut tracos: Vec<(bool, Vec<[f64; 2]>)> = vec![];
            let mut junta = |corta: bool, a: [f64; 2], b: [f64; 2]| {
                if let Some((c, pts)) = tracos.last_mut() {
                    if *c == corta && pts.last().is_some_and(|q| (q[0] - a[0]).abs() < 1e-9 && (q[1] - a[1]).abs() < 1e-9) {
                        pts.push(b);
                        return;
                    }
                }
                tracos.push((corta, vec![a, b]));
            };
            for (a, b) in o.trechos() {
                let corta = a.corta && b.corta;
                let y0 = self.y_de(a.s);
                let y1 = y0 + (b.s - a.s);
                // passa da costura: vai até a borda e continua do outro lado
                let borda = if y1 > per { Some(per) } else if y1 < 0. { Some(0.) } else { None };
                match borda {
                    Some(yb) => {
                        let t = (yb - y0) / (y1 - y0);
                        let xb = a.x + (b.x - a.x) * t;
                        junta(corta, [a.x, y0], [xb, yb]);
                        let outra = per - yb;
                        junta(corta, [xb, outra], [b.x, y1 - yb + outra]);
                    }
                    None => junta(corta, [a.x, y0], [b.x, y1]),
                }
            }
            for (corta, pts) in tracos {
                if corta {
                    d.poli(Camada::Corte, pts, false);
                } else {
                    for w in pts.windows(2) {
                        d.linha(Camada::Contorno, w[0], w[1], true);
                    }
                }
            }
            // número da operação na perfuração e seta do sentido no primeiro trecho da curva
            if let Some(p0) = o.pontos.first() {
                d.texto(Camada::Cota, [p0.x + 1., self.y_de(p0.s) + 1.], 2.5, format!("{}{}", k + 1, if o.separa { " (separa)" } else { "" }));
            }
            if let Some((a, b)) = o.trechos().find(|(a, b)| a.etapa == Etapa::Corte && b.etapa == Etapa::Corte && (b.s - a.s).hypot(b.x - a.x) > 1e-6) {
                let (dx, ds) = (b.x - a.x, b.s - a.s);
                let l = dx.hypot(ds);
                let (ux, uy) = (dx / l, ds / l);
                let ponta = [a.x, self.y_de(a.s)];
                for giro in [150f64, -150.] {
                    let (c, s) = (giro.to_radians().cos(), giro.to_radians().sin());
                    d.linha(Camada::Cota, ponta, [ponta[0] + 3. * (ux * c - uy * s), ponta[1] + 3. * (ux * s + uy * c)], false);
                }
            }
        }
        d.texto(
            Camada::Cota,
            [xmin, -5.],
            2.5,
            format!(
                "{}: tubo desenrolado {:.2} x {:.2} mm (pele externa; y = 0 na costura; x do início da barra). Corte {:.1} mm, {} perfurações, kerf {:.2} mm.",
                self.titulo,
                self.comprimento,
                per,
                self.comprimento_cortado(),
                self.perfuracoes(),
                self.opc.kerf
            ),
        );
        d
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
        if !self.real.vazio() {
            t += &format!("tubo real: costura a {:.0}° (zero da vista desenvolvida)", self.real.costura);
            if self.real.raio_canto > 0. {
                t += &format!("; raio de canto {:.1} mm: pele externa {:.2} mm (nominal {:.2})", self.real.raio_canto, self.real.perimetro(self.secao), self.perimetro);
            }
            if self.real.ovalizacao > 0. {
                t += &format!("; ovalização {:.2} mm: a ponta pode errar ±{:.2} mm na pele", self.real.ovalizacao, self.real.desvio_radial());
            }
            t += "\n";
        }
        t
    }
}

/// Monta o caminho de uma ponta. `inicio` = ponta 1 (a sobra fica antes do nó, `x` menor).
fn operacao(nome: &str, p: &PontaEncaixe, comprimento: f64, inicio: bool, separa: bool, opc: &OpcoesCaminho, costura: f64) -> Option<Operacao> {
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
    let mut ordem: Vec<usize> = if opc.inverter { (0..n).rev().collect() } else { (0..n).collect() };
    let per = c.perimetro;
    // início: o ponto mais perto da posição pedida no perímetro (distância cíclica)
    let alvo = ((opc.inicio + costura).rem_euclid(360.) / 360.) * per;
    let ciclica = |s: f64| {
        let d = (s - alvo).rem_euclid(per);
        d.min(per - d)
    };
    if let Some(k0) = (0..n).min_by(|&a, &b| ciclica(c.pontos[ordem[a]].s).total_cmp(&ciclica(c.pontos[ordem[b]].s))) {
        ordem.rotate_left(k0);
    }
    // direção, no mundo, em que `x` cresce
    let eixo_x = geo3d::mul(eixo, if inicio { 1. } else { -1. });
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
        let p3 = geo3d::add(q.p3, geo3d::mul(eixo_x, x - base[k].1));
        cam.push(PontoTocha { x, a, s, bisel: q.bisel, tocha, p3, etapa: Etapa::Corte, corta: true });
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
        let p3 = geo3d::add(a.p3, geo3d::mul(geo3d::sub(b.p3, a.p3), t));
        cam.push(PontoTocha { s: ult.s + ds * t, x: a.x + (b.x - a.x) * t, a: ult.a + da * t, p3, etapa: Etapa::Saida, ..b });
        falta -= seg * t;
        k += 1;
    }
    // entrada: reta vinda da sobra até o início
    if opc.entrada > 0. {
        let p3 = geo3d::add(primeiro.p3, geo3d::mul(eixo_x, lado_sobra * opc.entrada));
        let e = PontoTocha { x: primeiro.x + lado_sobra * opc.entrada, p3, etapa: Etapa::Entrada, ..primeiro };
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
    caminho_do_membro_real(m, opc, &TuboReal::default())
}

/// O mesmo caminho com as medidas reais do lote (B176): a costura vira o zero da vista desenvolvida
/// e a referência do início; ovalização e raio de canto entram no relatório.
pub fn caminho_do_membro_real(m: &MembroComEncaixe, opc: &OpcoesCaminho, real: &TuboReal) -> Option<CaminhoTubo> {
    let mut ops = vec![];
    let perimetro = m.pontas().next()?.corte.perimetro;
    let secao = m.pontas().next()?.corte.secao;
    if let Some(p) = &m.ponta1 {
        ops.push(operacao("ponta 1 (encaixe)", p, m.comprimento, true, m.ponta2.is_none(), opc, real.costura)?);
    }
    if let Some(p) = &m.ponta2 {
        ops.push(operacao("ponta 2 (encaixe)", p, m.comprimento, false, true, opc, real.costura)?);
    }
    if ops.is_empty() {
        return None;
    }
    Some(CaminhoTubo { titulo: format!("Caminho de corte · {} · linha {}", m.designacao, m.linha + 1), comprimento: m.comprimento, perimetro, operacoes: ops, opc: *opc, real: *real, secao })
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
        let opc = OpcoesCaminho { kerf: 0.4, entrada: 3., saida: 2., microjuntas: 2, microjunta: 1., velocidade: 1200., perfuracao: 0.8, inverter: false, inicio: 0. };
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

    #[test]
    fn inicio_girado_muda_a_perfuracao_e_nao_o_comprimento() {
        let m = travessa();
        let opc = OpcoesCaminho { kerf: 0., entrada: 0., saida: 0., ..Default::default() };
        let c0 = caminho_do_membro(&m, &opc).unwrap();
        let c90 = caminho_do_membro(&m, &OpcoesCaminho { inicio: 90., ..opc }).unwrap();
        let per = c0.perimetro;
        // perímetro do tubo 60,3: π·60,3
        assert!((per - std::f64::consts::PI * 60.3).abs() < 1e-9);
        // a perfuração vai para um quarto do perímetro (a menos de um passo da curva)
        let passo = per / m.ponta1.as_ref().unwrap().corte.pontos.len() as f64;
        let s0 = c90.operacoes[0].pontos[0].s.rem_euclid(per);
        assert!((s0 - per / 4.).abs() <= passo, "{s0} {}", per / 4.);
        // a mesma volta fechada: o mesmo comprimento cortado
        assert!((c0.comprimento_cortado() - c90.comprimento_cortado()).abs() < 1e-6, "{} {}", c0.comprimento_cortado(), c90.comprimento_cortado());
        let giro = c90.operacoes[0].pontos.last().unwrap().a - c90.operacoes[0].pontos[0].a;
        assert!((giro.abs() - 360.).abs() < 1e-6);
    }

    #[test]
    fn costura_real_e_o_zero_da_vista_e_do_inicio() {
        let m = travessa();
        let opc = OpcoesCaminho { kerf: 0., entrada: 0., saida: 0., ..Default::default() };
        let real = TuboReal { costura: 90., ovalizacao: 0.4, ..Default::default() };
        let c = caminho_do_membro_real(&m, &opc, &real).unwrap();
        let per = c.perimetro;
        let passo = per / m.ponta1.as_ref().unwrap().corte.pontos.len() as f64;
        // o corte começa na costura (a um quarto do perímetro da referência) e ali a vista vale zero
        let p0 = c.operacoes[0].pontos[0];
        assert!((p0.s.rem_euclid(per) - per / 4.).abs() <= passo, "{}", p0.s);
        let y = c.y_de(p0.s);
        assert!(y.min(per - y) <= passo, "{y}");
        // o mesmo comprimento do caminho sem medida real; o relatório fala da costura e da ovalização
        let c0 = caminho_do_membro(&m, &opc).unwrap();
        assert!((c.comprimento_cortado() - c0.comprimento_cortado()).abs() < 1e-6);
        let r = c.relatorio();
        assert!(r.contains("costura a 90°") && r.contains("±0.10 mm"), "{r}");
    }

    #[test]
    fn ponto_3d_segue_o_kerf_e_a_entrada() {
        let m = travessa();
        let opc = OpcoesCaminho { kerf: 0.4, entrada: 3., saida: 0., ..Default::default() };
        let c = caminho_do_membro(&m, &opc).unwrap();
        let o = &c.operacoes[0];
        let p1 = &m.ponta1.as_ref().unwrap().corte;
        // a entrada fica 3 mm para fora no eixo do tubo (a sobra da ponta 1 é o lado do nó)
        let (e, c0) = (&o.pontos[0], &o.pontos[1]);
        assert_eq!(e.etapa, Etapa::Entrada);
        assert!((geo3d::dist(e.p3, c0.p3) - 3.).abs() < 1e-9);
        let eixo = m.ponta1.as_ref().unwrap().dir;
        assert!((geo3d::dot(geo3d::sub(c0.p3, e.p3), eixo) - 3.).abs() < 1e-9);
        // nos pontos da curva, a tocha passa a no máximo kerf/2 da curva nominal, ao longo do eixo
        for (q, p) in o.pontos[1..].iter().zip(&p1.pontos) {
            let d = geo3d::sub(q.p3, p.p3);
            assert!(geo3d::norm(d) <= 0.2 + 1e-9);
            assert!((geo3d::norm(d) - geo3d::dot(d, eixo).abs()).abs() < 1e-9, "desvio fora do eixo");
            assert!(((q.x - p.lambda) - geo3d::dot(d, eixo)).abs() < 1e-9, "x e ponto 3D andam juntos");
        }
    }

    #[test]
    fn vista_desenvolvida_tem_o_corte_inteiro_e_vira_dxf() {
        let m = travessa();
        let opc = OpcoesCaminho { kerf: 0.2, entrada: 3., saida: 1., microjuntas: 2, microjunta: 1., inicio: 45., ..Default::default() };
        let c = caminho_do_membro(&m, &opc).unwrap();
        let d = c.desenvolvido();
        let per = c.perimetro;
        let corte: f64 = d.tracos.iter().filter(|t| t.camada == Camada::Corte).flat_map(|t| t.pts.windows(2).map(|w| (w[1][0] - w[0][0]).hypot(w[1][1] - w[0][1]))).sum();
        // o que está na camada de corte é exatamente o que a tocha corta (microjuntas fora)
        assert!((corte - c.comprimento_cortado()).abs() < 1e-6, "{corte} {}", c.comprimento_cortado());
        // tudo dentro do tubo desenrolado, sem traço atravessando a peça de uma ponta à outra
        for t in d.tracos.iter().filter(|t| t.camada == Camada::Corte) {
            assert!(t.pts.iter().all(|p| p[1] >= -1e-9 && p[1] <= per + 1e-9));
            for w in t.pts.windows(2) {
                assert!((w[1][0] - w[0][0]).abs() < 10. && (w[1][1] - w[0][1]).abs() < per / 4., "{w:?}");
            }
        }
        // microjuntas tracejadas e o número de cada operação
        assert!(d.tracos.iter().any(|t| t.camada == Camada::Contorno && t.tracejado));
        assert!(d.rotulos.iter().any(|r| r.texto == "2 (separa)"));
        let dxf = crate::st_dobra_saida::dxf(&d);
        assert!(dxf.contains("CORTE") && dxf.contains("LINE") && dxf.contains("ENTITIES"));
    }
}
