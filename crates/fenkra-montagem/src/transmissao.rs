// SPDX-License-Identifier: LGPL-2.1-or-later
//! Elemento de transmissão (B340): um comando só para engrenagens (retas, helicoidais, espinha de
//! peixe, internas), cônicas a 90°, diferencial, sem-fim e coroa, corrente e correia.
//!
//! O elemento é descrito por um [`Elemento`] (parâmetros), que fica **gravado na montagem** num
//! [`RegistroTransmissao`]: dá para reabrir e editar depois ([`editar_transmissao`]). A inserção
//! monta o conjunto pronto para animar:
//!
//! - um **eixo** fixo (pino) por peça girante, com o diâmetro do furo;
//! - uma **junta de revolução** entre cada eixo e sua peça (a junta já traz a retenção axial);
//! - **acoplamentos** de engrenagem entre as juntas, com a relação certa e o sinal do giro;
//! - o **comando de giro** (`rpm · 6 · t` graus) na junta motora, de modo que a simulação da montagem
//!   já anda com a velocidade do cálculo (B343).
//!
//! Convenção das peças: o eixo de cada peça é o Z local, e a junta usa esse Z (o eixo da peça e o seu
//! pino compartilham a rotação). Nas cônicas e no diferencial o Z local aponta **para o vértice do
//! cone**, então o acoplamento entre elas é `−z1/z2` em qualquer par.

use crate::engrenagens::{calculo_par, posicoes_do_par};
use crate::kin::{CouplingKind, JFrame, JRef, JointKind};
use crate::*;
use fenkra_core::Document;
use fenkra_solidos::engrenagens::{engrenagem_documento, ParamEngrenagem, TipoEngrenagem};
use fenkra_solidos::engrenagens_conicas::{calculo_conica, calculo_sem_fim, conica_documento, coroa_documento, giro_de_engrenamento, sem_fim_documento, ParamConica, ParamSemFim};
use fenkra_solidos::transmissao_calc::{calcular_cadeia, Entrada, Estagio, ResultadoCadeia, TipoEstagio};
use fenkra_solidos::transmissao_corrente::{calculo_roda, corrente_documento, correia_documento, polia_documento, roda_documento, ParamLacoCorrente, ParamLacoCorreia, ParamPolia, ParamRoda, TipoPolia};
use fenkra_solidos::transmissao_dados::{self as dados, FamiliaCorreia};
use std::f64::consts::PI;
use std::path::Path;

/// Nome do grupo da árvore que recebe os elementos de transmissão.
const GRUPO: &str = "Transmissão";
/// Sobra do pino além da peça, de cada lado (mm).
const SOBRA_PINO: f64 = 5.;

/// O que o comando gera, com os parâmetros que a pessoa edita.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum Elemento {
    /// par de engrenagens externas (reta, helicoidal ou espinha de peixe) ou interna; o `tipo` do
    /// pinhão dá o tipo do par
    Engrenagens { pinhao: ParamEngrenagem, dentes_coroa: u32 },
    /// par de cônicas retas a 90° (o par de dentes está no `ParamConica`)
    Conicas { par: ParamConica },
    /// diferencial: duas laterais (`dentes`) e dois satélites (`dentes_par`). Com `caixa_solta`, a caixa
    /// gira e leva os satélites; `giro_relativo` (rpm) é o giro das laterais em relação a ela
    Diferencial {
        lateral: ParamConica,
        #[serde(default)]
        caixa_solta: bool,
        #[serde(default)]
        giro_relativo: f64,
    },
    SemFim { p: ParamSemFim },
    Corrente { corrente: String, dentes1: u32, dentes2: u32, centros: f64, furo1: f64, furo2: f64 },
    Correia { designacao: String, dentes1: u32, dentes2: u32, diametro1: f64, diametro2: f64, centros: f64, largura: f64, furo1: f64, furo2: f64 },
}

impl Elemento {
    pub fn rotulo(&self) -> &'static str {
        match self {
            Elemento::Engrenagens { pinhao, .. } => match pinhao.tipo {
                TipoEngrenagem::Helicoidal if pinhao.espinha_peixe => "Engrenagens espinha de peixe",
                TipoEngrenagem::Helicoidal => "Engrenagens helicoidais",
                TipoEngrenagem::Interna => "Engrenagem interna",
                _ => "Engrenagens retas",
            },
            Elemento::Conicas { .. } => "Engrenagens cônicas",
            Elemento::Diferencial { .. } => "Diferencial cônico",
            Elemento::SemFim { .. } => "Sem-fim e coroa",
            Elemento::Corrente { .. } => "Corrente",
            Elemento::Correia { .. } => "Correia",
        }
    }

    /// Estágio do cálculo de transmissão (B343) deste elemento.
    pub fn estagio(&self) -> Estagio {
        match self {
            Elemento::Engrenagens { pinhao, dentes_coroa } => {
                let t = match pinhao.tipo {
                    TipoEngrenagem::Helicoidal if pinhao.espinha_peixe => TipoEstagio::EspinhaDePeixe,
                    TipoEngrenagem::Helicoidal => TipoEstagio::Helicoidal,
                    TipoEngrenagem::Interna => TipoEstagio::Interna,
                    TipoEngrenagem::Cremalheira => TipoEstagio::Cremalheira,
                    TipoEngrenagem::Reta => TipoEstagio::Externa,
                };
                Estagio::novo(t, pinhao.dentes as f64, *dentes_coroa as f64)
            }
            Elemento::Conicas { par } => Estagio::novo(TipoEstagio::Conica, par.dentes as f64, par.dentes_par as f64),
            Elemento::Diferencial { caixa_solta: true, .. } => Estagio::novo(TipoEstagio::Diferencial, 1., 1.),
            Elemento::Diferencial { lateral, .. } => Estagio::novo(TipoEstagio::Conica, lateral.dentes as f64, lateral.dentes_par as f64),
            Elemento::SemFim { p } => {
                let lam = (std::f64::consts::PI * p.modulo * p.entradas as f64 / (std::f64::consts::PI * p.diametro_primitivo)).atan().to_degrees();
                let mut e = Estagio::novo(TipoEstagio::SemFim, p.entradas as f64, p.dentes_coroa as f64);
                e.angulo_avanco = lam;
                e.angulo_pressao = p.angulo_pressao;
                e
            }
            Elemento::Corrente { dentes1, dentes2, .. } => Estagio::novo(TipoEstagio::Corrente, *dentes1 as f64, *dentes2 as f64),
            Elemento::Correia { designacao, dentes1, dentes2, diametro1, diametro2, .. } => match dados::correia(designacao).map(|c| c.familia) {
                Some(FamiliaCorreia::EmV) => Estagio::novo(TipoEstagio::CorreiaV, *diametro1, *diametro2),
                _ => Estagio::novo(TipoEstagio::CorreiaDentada, *dentes1 as f64, *dentes2 as f64),
            },
        }
    }
}

/// Registro gravado na montagem: o elemento, a entrada do cálculo e o que foi criado.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct RegistroTransmissao {
    pub id: Id,
    pub nome: String,
    pub elemento: Elemento,
    /// rotações por minuto na junta motora e torque de entrada (N·m): alimentam o cálculo e a simulação
    pub rpm: f64,
    pub torque: f64,
    pub comps: Vec<Id>,
    pub juntas: Vec<Id>,
    pub acoplamentos: Vec<Id>,
    /// junta que recebe o `rpm`; a cadeia escreve nela o que o estágio anterior entrega
    #[serde(default)]
    pub motora: Option<Id>,
}

/// Uma peça do conjunto antes de entrar na montagem.
#[derive(Clone, Debug)]
pub struct PecaLayout {
    pub nome: String,
    pub chave: String,
    pub doc: Document,
    pub lugar: Placement,
    /// eixo fixo (pino)
    pub fixa: bool,
}

/// Conjunto calculado, sem tocar na montagem.
#[derive(Clone, Debug)]
pub struct Layout {
    pub pecas: Vec<PecaLayout>,
    /// (índice do eixo, índice da peça): uma junta de revolução cada
    pub juntas: Vec<(usize, usize)>,
    /// (junta a, junta b, razão): giro de b = razão × giro de a
    pub acoplamentos: Vec<(usize, usize, f64)>,
    /// junta que recebe o comando de giro
    pub motora: usize,
    /// outras juntas comandadas, com o giro delas (rpm)
    pub comandos: Vec<(usize, f64)>,
    /// peças que andam como uma só
    pub rigidos: Vec<Vec<usize>>,
    /// as juntas são cilíndricas (giram e deslizam no eixo): a dupla hélice em V centra o par sozinha,
    /// sem retenção axial
    pub folga_axial: bool,
    pub distancia_centros: f64,
    /// giro da última peça por giro da motora, com o sinal
    pub razao: f64,
    pub avisos: Vec<String>,
}

fn pino(nome: &str, furo: f64, comprimento: f64) -> Result<Document, String> {
    fenkra_solidos::transmissao_corrente::pino_documento(nome, furo, comprimento)
}

/// Peça e o eixo dela: o eixo tem a mesma rotação (Z comum) e atravessa a peça de `z0` a `z1` (locais).
fn peca_com_eixo(pecas: &mut Vec<PecaLayout>, juntas: &mut Vec<(usize, usize)>, peca: PecaLayout, furo: f64, z0: f64, z1: f64) -> Result<usize, String> {
    let f = if furo > 0. { furo } else { 8. };
    let lugar = peca.lugar;
    let eixo = pino(&format!("Eixo Ø{f}"), f, z1 - z0)?;
    pecas.push(PecaLayout { nome: format!("Eixo {}", pecas.iter().filter(|p| p.fixa).count() + 1), chave: format!("eixo-d{f}-l{:.2}", z1 - z0), doc: eixo, lugar: Placement { origin: lugar.point([0., 0., z0]), rot: lugar.rot }, fixa: true });
    let i_eixo = pecas.len() - 1;
    pecas.push(peca);
    let i_peca = pecas.len() - 1;
    juntas.push((i_eixo, i_peca));
    Ok(juntas.len() - 1)
}

fn peca(doc: Document, chave: String, lugar: Placement) -> PecaLayout {
    PecaLayout { nome: doc.props.description.clone(), chave, doc, lugar, fixa: false }
}

/// Placement de uma cônica: vértice em `vertice`, eixo do corpo `u` (do vértice para fora), geratriz
/// de contato `contato` e `theta` o azimute local do contato (0 = dente no contato, π/z = vão).
fn lugar_conico(vertice: V3, altura: f64, u: V3, contato: V3, theta: f64) -> Placement {
    let zl = [-u[0], -u[1], -u[2]];
    let proj = sub(contato, [dot(contato, u) * u[0], dot(contato, u) * u[1], dot(contato, u) * u[2]]);
    let w = unit(proj);
    // X local = w girado de −theta em torno de Z local
    let xl = rotvec(w, [zl[0] * -theta, zl[1] * -theta, zl[2] * -theta]);
    let yl = cross(zl, xl);
    Placement { origin: [vertice[0] + altura * u[0], vertice[1] + altura * u[1], vertice[2] + altura * u[2]], rot: [xl, yl, zl] }
}

/// Peça girada de `rho` em torno do Z do mundo na origem (para o segundo satélite).
fn girar_z(p: &Placement, rho: f64) -> Placement {
    p.moved([0.; 3], [0., 0., rho], [0.; 3])
}

fn nome_curto(s: &str) -> String {
    s.to_string()
}

/// Monta o conjunto do elemento: peças, juntas, acoplamentos e a razão.
pub fn montar_layout(e: &Elemento) -> Result<Layout, String> {
    let mut pecas = vec![];
    let mut juntas = vec![];
    let mut avisos = vec![];
    let mut acopl = vec![];
    let (distancia, razao, motora);
    let (mut comandos, mut rigidos) = (vec![], vec![]);
    match e {
        Elemento::Engrenagens { pinhao, dentes_coroa } => {
            if pinhao.tipo == TipoEngrenagem::Cremalheira {
                return Err("o par com cremalheira usa o comando de pares de engrenagens (inserir par)".into());
            }
            let par = calculo_par(pinhao, *dentes_coroa)?;
            let (l1, l2) = posicoes_do_par(&par, *dentes_coroa);
            let d1 = engrenagem_documento(&par.pinhao)?;
            let d2 = engrenagem_documento(&par.coroa)?;
            let (k1, k2) = (par.pinhao.chave_arquivo(), par.coroa.chave_arquivo());
            let furo = pinhao.furo;
            let w = pinhao.largura;
            let j1 = peca_com_eixo(&mut pecas, &mut juntas, peca(d1, k1, l1), furo, -SOBRA_PINO, w + SOBRA_PINO)?;
            let j2 = peca_com_eixo(&mut pecas, &mut juntas, peca(d2, k2, l2), furo, -SOBRA_PINO, w + SOBRA_PINO)?;
            acopl.push((j1, j2, par.razao));
            distancia = par.distancia_centros;
            razao = par.razao;
            motora = j1;
            avisos.extend(par.avisos);
        }
        Elemento::Conicas { par } => {
            let c1 = calculo_conica(par)?;
            let pg = ParamConica { dentes: par.dentes_par, dentes_par: par.dentes, ..par.clone() };
            let c2 = calculo_conica(&pg)?;
            let (u1, u2) = ([0., 0., 1.], [1., 0., 0.]);
            let delta1 = c1.angulo_cone.to_radians();
            let contato = [delta1.cos() * u1[0] + delta1.sin() * u2[0], delta1.cos() * u1[1] + delta1.sin() * u2[1], delta1.cos() * u1[2] + delta1.sin() * u2[2]];
            let l1 = lugar_conico([0.; 3], c1.altura_vertice, u1, contato, 0.);
            let l2 = lugar_conico([0.; 3], c2.altura_vertice, u2, contato, PI / pg.dentes as f64);
            let hub = |p: &ParamConica| (p.furo * 1.2).max(p.modulo * 3.);
            let d1 = conica_documento(par)?;
            let d2 = conica_documento(&pg)?;
            let (k1, k2) = (format!("conica-m{}-z{}-{}-f{}", par.modulo, par.dentes, par.dentes_par, par.furo), format!("conica-m{}-z{}-{}-f{}", pg.modulo, pg.dentes, pg.dentes_par, pg.furo));
            let j1 = peca_com_eixo(&mut pecas, &mut juntas, peca(d1, k1, l1), par.furo, -hub(par) - SOBRA_PINO, c1.profundidade_dentes + SOBRA_PINO)?;
            let j2 = peca_com_eixo(&mut pecas, &mut juntas, peca(d2, k2, l2), pg.furo, -hub(&pg) - SOBRA_PINO, c2.profundidade_dentes + SOBRA_PINO)?;
            let r = -(par.dentes as f64) / par.dentes_par as f64;
            acopl.push((j1, j2, r));
            distancia = 0.;
            razao = r;
            motora = j1;
            avisos.extend(c1.avisos.iter().map(|a| format!("peça 1: {a}")));
            avisos.extend(c2.avisos.iter().map(|a| format!("peça 2: {a}")));
        }
        Elemento::Diferencial { lateral, caixa_solta, giro_relativo } => {
            let (zs, zp) = (lateral.dentes, lateral.dentes_par);
            if zs % 2 != 0 {
                return Err("o diferencial com dois satélites pede laterais com número par de dentes".into());
            }
            let sat = ParamConica { dentes: zp, dentes_par: zs, ..lateral.clone() };
            let cs = calculo_conica(lateral)?;
            let cp = calculo_conica(&sat)?;
            let ds = cs.angulo_cone.to_radians();
            let (uz, ux) = ([0., 0., 1.], [1., 0., 0.]);
            let contato1 = [ds.sin() * ux[0], 0., ds.cos()];
            let contato2 = [ds.sin(), 0., -ds.cos()];
            let sat_gap = PI / zp as f64;
            let l_s1 = lugar_conico([0.; 3], cs.altura_vertice, uz, contato1, 0.);
            let l_p1 = lugar_conico([0.; 3], cp.altura_vertice, ux, contato1, sat_gap);
            // o segundo contato do satélite cai a meia volta do primeiro: vão se z do satélite é par
            let theta2 = if zp % 2 == 0 { 0. } else { PI / zs as f64 };
            let l_s2 = lugar_conico([0.; 3], cs.altura_vertice, [0., 0., -1.], contato2, theta2);
            let l_p2 = girar_z(&l_p1, PI);
            let hub = |p: &ParamConica| (p.furo * 1.2).max(p.modulo * 3.);
            let dl = conica_documento(lateral)?;
            let dsat = conica_documento(&sat)?;
            let kl = format!("conica-m{}-z{}-{}-f{}", lateral.modulo, zs, zp, lateral.furo);
            let ks = format!("conica-m{}-z{}-{}-f{}", sat.modulo, zp, zs, sat.furo);
            let js1 = peca_com_eixo(&mut pecas, &mut juntas, PecaLayout { nome: "Lateral 1".into(), ..peca(dl.clone(), kl.clone(), l_s1) }, lateral.furo, -hub(lateral) - SOBRA_PINO, cs.profundidade_dentes + SOBRA_PINO)?;
            let jp1 = peca_com_eixo(&mut pecas, &mut juntas, PecaLayout { nome: "Satélite 1".into(), ..peca(dsat.clone(), ks.clone(), l_p1) }, sat.furo, -hub(&sat) - SOBRA_PINO, cp.profundidade_dentes + SOBRA_PINO)?;
            let js2 = peca_com_eixo(&mut pecas, &mut juntas, PecaLayout { nome: "Lateral 2".into(), ..peca(dl, kl, l_s2) }, lateral.furo, -hub(lateral) - SOBRA_PINO, cs.profundidade_dentes + SOBRA_PINO)?;
            let jp2 = peca_com_eixo(&mut pecas, &mut juntas, PecaLayout { nome: "Satélite 2".into(), ..peca(dsat, ks, l_p2) }, sat.furo, -hub(&sat) - SOBRA_PINO, cp.profundidade_dentes + SOBRA_PINO)?;
            // com a caixa presa: lateral1 → satélite (−zs/zp) → lateral2 (−zp/zs); o satélite 2 acompanha o 1
            acopl.push((js1, jp1, -(zs as f64) / zp as f64));
            acopl.push((jp1, js2, -(zp as f64) / zs as f64));
            acopl.push((js1, jp2, -(zs as f64) / zp as f64));
            distancia = 0.;
            if *caixa_solta {
                // os acoplamentos valem entre juntas, e toda junta aqui é medida em relação ao eixo dela.
                // Prender os eixos na caixa muda o referencial sem mudar a conta: com a caixa girando
                // θc, as laterais ficam em θc ∓ α, e daí sai ω caixa = (ω1 + ω2) / 2
                let eixos: Vec<usize> = [js1, jp1, js2, jp2].iter().map(|j| juntas[*j].0).collect();
                for i in &eixos {
                    pecas[*i].fixa = false;
                }
                let raio = (cs.diametro_topo / 2.).max(cp.altura_vertice + hub(&sat) + SOBRA_PINO / 2.) + 1.;
                let comprimento = 2. * (cp.diametro_topo / 2. + 2.);
                let doc = fenkra_solidos::transmissao_corrente::caixa_documento("Caixa do diferencial", 2. * raio + 8., 2. * raio, comprimento)?;
                let chave = format!("caixa-d{:.2}-l{:.2}", 2. * raio, comprimento);
                let lugar = Placement { origin: [0., 0., -comprimento / 2.], ..Default::default() };
                // o eixo da caixa fica depois da ponta do eixo da lateral 2, que corre na mesma reta
                let ponta = comprimento / 2. - cs.altura_vertice - hub(lateral) - SOBRA_PINO - 1.;
                let jc = peca_com_eixo(&mut pecas, &mut juntas, PecaLayout { nome: "Caixa".into(), ..peca(doc, chave, lugar) }, lateral.furo, ponta - 2. * SOBRA_PINO, ponta)?;
                let mut grupo = eixos;
                grupo.push(juntas[jc].1);
                rigidos.push(grupo);
                comandos.push((js1, *giro_relativo));
                razao = 1.;
                motora = jc;
            } else {
                razao = -1.;
                motora = js1;
                avisos.push("diferencial: com a caixa presa, a lateral 2 gira em sentido contrário; solte a caixa para ver ω caixa = (ω1 + ω2) / 2".into());
            }
        }
        Elemento::SemFim { p } => {
            let c = calculo_sem_fim(p)?;
            let dw = sem_fim_documento(p)?;
            let dc = coroa_documento(p)?;
            let coroa_p = fenkra_solidos::engrenagens_conicas::coroa_param(p)?;
            let b2 = c.largura_coroa;
            // coroa no eixo Z, com a posição de fase que mantém o dente no vão do sem-fim
            let r2 = p.modulo * p.dentes_coroa as f64 / 2.;
            let rho_w = (b2 / 2.) * coroa_p.angulo_helice.to_radians().tan() / r2;
            let l_coroa = Placement::default().moved([0.; 3], [0., 0., -rho_w], [0.; 3]);
            // sem-fim: eixo Z local ao longo de +Y do mundo, X local em +X, centro axial na altura da coroa
            let base = Placement { origin: [c.distancia_centros, -p.comprimento / 2., b2 / 2.], rot: [[1., 0., 0.], [0., 0., -1.], [0., 1., 0.]] };
            let spin = giro_de_engrenamento(p);
            let l_worm = Placement { origin: base.origin, rot: base.rot.map(|v| rotvec(v, [base.rot[2][0] * spin, base.rot[2][1] * spin, base.rot[2][2] * spin])) };
            let kw = format!("semfim-m{}-e{}-d{}-l{}-f{}", p.modulo, p.entradas, p.diametro_primitivo, p.comprimento, p.furo);
            let kc = format!("coroa-semfim-m{}-e{}-d{}-z{}-b{}-f{}", p.modulo, p.entradas, p.diametro_primitivo, p.dentes_coroa, b2, p.furo_coroa);
            let jw = peca_com_eixo(&mut pecas, &mut juntas, peca(dw, kw, l_worm), p.furo, -SOBRA_PINO, p.comprimento + SOBRA_PINO)?;
            let jc = peca_com_eixo(&mut pecas, &mut juntas, peca(dc, kc, l_coroa), p.furo_coroa, -SOBRA_PINO, b2 + SOBRA_PINO)?;
            // eixos do mundo: sem-fim +Y, coroa +Z: com o filete de mão direita o giro da coroa tem o sinal oposto
            let r = -(p.entradas as f64) / p.dentes_coroa as f64;
            acopl.push((jw, jc, r));
            distancia = c.distancia_centros;
            razao = r;
            motora = jw;
            avisos.extend(c.avisos);
        }
        Elemento::Corrente { corrente, dentes1, dentes2, centros, furo1, furo2 } => {
            // a largura pode ser provisória (219, B395): a peça leva o aviso e o painel também
            let (c, aviso_largura) = dados::corrente_para_gerar(corrente)?;
            avisos.extend(aviso_largura);
            let (n1, n2) = ((*dentes1).min(*dentes2), (*dentes1).max(*dentes2));
            let elos = dados::elos_da_corrente(c.passo, n1, n2, *centros);
            let cc = dados::distancia_corrigida_corrente(c.passo, n1, n2, elos).ok_or("a corrente é curta demais para as duas rodas")?;
            if (cc - centros).abs() > c.passo {
                avisos.push(format!("distância entre centros corrigida: {cc:.2} mm (o pedido, {centros:.2}, não fecha em elos inteiros)"));
            }
            let r1 = ParamRoda::nova(corrente, n1, *furo1);
            let r2 = ParamRoda::nova(corrente, n2, *furo2);
            let k1 = calculo_roda(&r1)?;
            calculo_roda(&r2)?;
            let z_roda = (c.largura - k1.largura_dente) / 2.;
            let d1 = roda_documento(&r1)?;
            let d2 = roda_documento(&r2)?;
            let l1 = Placement { origin: [0., 0., z_roda], ..Default::default() };
            let l2 = Placement { origin: [cc, 0., z_roda], ..Default::default() };
            let w = k1.largura_dente + (furo1 * 0.8).max(4.);
            let j1 = peca_com_eixo(&mut pecas, &mut juntas, peca(d1, r1.chave_arquivo(), l1), *furo1, -SOBRA_PINO - z_roda, w + SOBRA_PINO)?;
            let j2 = peca_com_eixo(&mut pecas, &mut juntas, peca(d2, r2.chave_arquivo(), l2), *furo2, -SOBRA_PINO - z_roda, w + SOBRA_PINO)?;
            let corr = corrente_documento(&ParamLacoCorrente { corrente: corrente.clone(), dentes1: n1, dentes2: n2, elos, centros: cc, com_rolos: false })?;
            pecas.push(PecaLayout { nome: format!("Corrente {} ({elos} elos)", c.designacao), chave: format!("corrente-{}-z{n1}-z{n2}-e{elos}", c.designacao.replace(' ', "")), doc: corr, lugar: Placement::default(), fixa: true });
            // a menor é a motora; a razão é o sentido igual: n1 / n2
            let r = n1 as f64 / n2 as f64;
            acopl.push((j1, j2, r));
            distancia = cc;
            razao = r;
            motora = j1;
            avisos.push(format!("{} elos, distância corrigida {cc:.2} mm; perfil da roda simplificado (preliminar)", elos));
        }
        Elemento::Correia { designacao, dentes1, dentes2, diametro1, diametro2, centros, largura, furo1, furo2 } => {
            let c = dados::correia(designacao).ok_or_else(|| format!("correia '{designacao}' fora da biblioteca"))?;
            if !c.liberada() {
                return Err(format!("{}: dado {}: a geração só usa linhas conferidas em duas fontes", c.designacao, c.conferencia.rotulo()));
            }
            let (pol1, pol2, d1, d2, espessura, cc) = match c.familia {
                FamiliaCorreia::Dentada => {
                    let (n1, n2) = ((*dentes1).min(*dentes2), (*dentes1).max(*dentes2));
                    let (d1, d2) = (dados::diametro_polia_dentada(c.passo, n1), dados::diametro_polia_dentada(c.passo, n2));
                    let l = dados::comprimento_correia(d1, d2, *centros);
                    let nt = dados::dentes_da_correia(c.passo, l);
                    let cc = dados::distancia_da_correia(d1, d2, nt as f64 * c.passo).ok_or("a correia é curta demais para as duas polias")?;
                    avisos.push(format!("correia de {nt} dentes, centros {cc:.2} mm; polia lisa e correia sem dentes desenhados (a altura do dente não tem duas fontes)"));
                    (
                        ParamPolia { tipo: TipoPolia::Dentada { passo: c.passo, dentes: n1, largura: *largura }, furo: *furo1 },
                        ParamPolia { tipo: TipoPolia::Dentada { passo: c.passo, dentes: n2, largura: *largura }, furo: *furo2 },
                        d1,
                        d2,
                        (0.5 * c.passo).max(1.),
                        cc,
                    )
                }
                FamiliaCorreia::EmV => {
                    let (d1, d2) = (diametro1.min(*diametro2), diametro1.max(*diametro2));
                    avisos.push("correia em V: seção em V não desenhada na correia (anel liso); ângulo do sulco 38° (uma fonte)".into());
                    (
                        ParamPolia { tipo: TipoPolia::EmV { topo: c.largura_topo, altura: c.altura, diametro: d1 }, furo: *furo1 },
                        ParamPolia { tipo: TipoPolia::EmV { topo: c.largura_topo, altura: c.altura, diametro: d2 }, furo: *furo2 },
                        d1,
                        d2,
                        c.altura / 2.,
                        *centros,
                    )
                }
            };
            let altura_polia = |p: &ParamPolia| match p.tipo {
                TipoPolia::Dentada { largura, .. } => largura + 3.,
                TipoPolia::EmV { topo, .. } => topo + 6.,
            };
            let (h1, h2) = (altura_polia(&pol1), altura_polia(&pol2));
            let dp1 = polia_documento(&pol1)?;
            let dp2 = polia_documento(&pol2)?;
            let (l1, l2) = (Placement::default(), Placement { origin: [cc, 0., 0.], ..Default::default() });
            let j1 = peca_com_eixo(&mut pecas, &mut juntas, peca(dp1, format!("polia-{}-d{:.2}-f{furo1}", designacao.replace(' ', ""), d1), l1), *furo1, -SOBRA_PINO, h1 + SOBRA_PINO)?;
            let j2 = peca_com_eixo(&mut pecas, &mut juntas, peca(dp2, format!("polia-{}-d{:.2}-f{furo2}", designacao.replace(' ', ""), d2), l2), *furo2, -SOBRA_PINO, h2 + SOBRA_PINO)?;
            // a correia envolve as polias: o anel começa no raio externo (o primitivo) e tem a espessura dada
            let largura_cor = match c.familia {
                FamiliaCorreia::Dentada => *largura,
                FamiliaCorreia::EmV => c.largura_topo,
            };
            let corr = correia_documento(&ParamLacoCorreia { d1, d2, centros: cc, espessura, largura: largura_cor })?;
            let z_cor = match c.familia {
                FamiliaCorreia::Dentada => 1.5,
                FamiliaCorreia::EmV => 3.,
            };
            pecas.push(PecaLayout { nome: format!("Correia {}", c.designacao), chave: format!("correia-{}-d{:.2}-{:.2}-c{:.2}", designacao.replace(' ', ""), d1, d2, cc), doc: corr, lugar: Placement { origin: [0., 0., z_cor], ..Default::default() }, fixa: true });
            let r = d1 / d2;
            acopl.push((j1, j2, r));
            distancia = cc;
            razao = r;
            motora = j1;
        }
    }
    let _ = nome_curto("");
    let folga_axial = matches!(e, Elemento::Engrenagens { pinhao, .. } if pinhao.espinha_peixe && pinhao.tipo == TipoEngrenagem::Helicoidal);
    Ok(Layout { pecas, juntas, acoplamentos: acopl, motora, comandos, rigidos, folga_axial, distancia_centros: distancia, razao, avisos })
}

/// Rolos da corrente de um registro, no mundo, na posição da simulação agora: andam pelo laço com a
/// rotação da roda motora (a corrente avança o arco do primitivo). Devolve também o diâmetro do rolo.
/// A viewport desenha esses pontos por cima da corrente (a corrente em si é um corpo estático de placas).
pub fn rolos_da_corrente(asm: &Assembly, reg: &RegistroTransmissao) -> Option<(Vec<V3>, f64)> {
    let Elemento::Corrente { corrente, dentes1, dentes2, .. } = &reg.elemento else { return None };
    let (c, _) = dados::corrente_para_gerar(corrente).ok()?;
    let (n1, n2) = ((*dentes1).min(*dentes2), (*dentes1).max(*dentes2));
    let (roda1, roda2) = (asm.node(*reg.comps.get(1)?)?.placement, asm.node(*reg.comps.get(3)?)?.placement);
    let centros = norm(sub(roda2.origin, roda1.origin));
    let cam = fenkra_solidos::transmissao_corrente::Caminho::novo(dados::diametro_primitivo_roda(c.passo, n1) / 2., dados::diametro_primitivo_roda(c.passo, n2) / 2., centros).ok()?;
    let pl: HashMap<Id, Placement> = asm.nodes.iter().map(|n| (n.id, n.placement)).collect();
    let junta = asm.joints.iter().find(|j| Some(&j.id) == reg.juntas.first())?;
    let graus = crate::kin::measure(junta, &pl)?.0;
    let fase = fenkra_solidos::transmissao_corrente::avanco_da_corrente(cam.r1, graus);
    let z = c.largura / 2.;
    let n = dados::elos_da_corrente(c.passo, n1, n2, centros).max(1) as usize;
    let pontos = cam.elos(n, fase).iter().map(|q| roda1.point([q.x, q.y, z])).collect();
    Some((pontos, c.rolo))
}

/// Lei do comando de giro: graus no tempo `t`.
fn comando_de_giro(rpm: f64) -> String {
    format!("{}*t", rpm * 6.)
}

/// Todos os elementos da montagem como uma cadeia, na ordem em que foram gerados, com a entrada do
/// primeiro. Sem elemento nenhum não há cadeia.
pub fn cadeia_da_montagem(asm: &Assembly) -> Option<Result<ResultadoCadeia, String>> {
    let primeiro = asm.transmissoes.first()?;
    let estagios: Vec<Estagio> = asm.transmissoes.iter().map(|r| r.elemento.estagio()).collect();
    Some(calcular_cadeia(&estagios, Entrada { rpm: primeiro.rpm, torque: primeiro.torque }))
}

/// Leva a cadeia para a simulação: cada elemento depois do primeiro passa a receber o que o anterior
/// entrega, no comando da junta motora e na entrada do cálculo.
pub fn aplicar_cadeia(asm: &mut Assembly) -> Result<ResultadoCadeia, String> {
    let r = cadeia_da_montagem(asm).ok_or("a montagem não tem elementos de transmissão")??;
    for k in 1..asm.transmissoes.len() {
        let (rpm, torque) = (r.estagios[k].rpm_entrada, r.estagios[k].torque_entrada);
        let reg = &mut asm.transmissoes[k];
        reg.rpm = rpm;
        reg.torque = torque;
        let motora = reg.motora.or(reg.juntas.first().copied());
        if let Some(j) = asm.joints.iter_mut().find(|j| Some(j.id) == motora) {
            j.drive_angle = Some(comando_de_giro(rpm));
        }
    }
    Ok(r)
}

/// Calcula o elemento ao vivo: estágio, resultado com a entrada dada.
pub fn calcular_elemento(e: &Elemento, entrada: Entrada) -> Result<ResultadoCadeia, String> {
    calcular_cadeia(&[e.estagio()], entrada)
}

#[derive(Clone, Debug)]
pub struct RelatorioTransmissao {
    pub registro: Id,
    pub comps: Vec<Id>,
    pub juntas: Vec<Id>,
    pub razao: f64,
    pub distancia_centros: f64,
    pub avisos: Vec<String>,
}

fn gravar(dir: &Path, chave: &str, doc: &Document) -> Result<(String, String), String> {
    let pasta = dir.join("transmissao");
    let destino = pasta.join(format!("{chave}.fkp"));
    if !destino.exists() {
        std::fs::create_dir_all(&pasta).map_err(|e| format!("{}: {e}", pasta.display()))?;
        std::fs::write(&destino, doc.to_json()).map_err(|e| format!("{}: {e}", destino.display()))?;
    }
    Ok((format!("transmissao/{chave}.fkp"), doc.props.description.clone()))
}

/// Insere o elemento na montagem: peças, eixos fixos, juntas, acoplamentos, comando de giro e o
/// registro editável. `dir` é a pasta da montagem. A transformação `base` leva o conjunto para onde
/// ele nasce (normalmente a identidade).
pub fn inserir_transmissao(asm: &mut Assembly, dir: Option<&Path>, e: &Elemento, rpm: f64, torque: f64, base: &Placement) -> Result<RelatorioTransmissao, String> {
    let dir = dir.ok_or("salve a montagem antes: as peças da transmissão são gravadas na pasta dela")?;
    let lay = montar_layout(e)?;
    let calc = calcular_elemento(e, Entrada { rpm, torque }).map_err(|m| format!("cálculo: {m}"))?;
    let _ = calc;
    let arquivos: Vec<(String, String)> = lay.pecas.iter().map(|p| gravar(dir, &p.chave, &p.doc)).collect::<Result<_, _>>()?;
    let pasta = match asm.nodes.iter().find(|n| matches!(n.kind, NodeKind::Group) && n.name == GRUPO).map(|n| n.id) {
        Some(g) => g,
        None => asm.add_group(GRUPO, None),
    };
    let tem_fixo = asm.nodes.iter().any(|n| !matches!(n.kind, NodeKind::Group) && n.fixed);
    let mut comps = vec![];
    for (p, (arq, nome)) in lay.pecas.iter().zip(&arquivos) {
        let id = asm.add_component(if p.fixa { &p.nome } else { nome }, NodeKind::Part { path: arq.clone() }, base.then(&p.lugar), Some(pasta));
        if let Some(n) = asm.node_mut(id) {
            n.fixed = p.fixa;
        }
        comps.push(id);
    }
    let _ = tem_fixo;
    let f = JFrame::from_axis([0.; 3], [0., 0., 1.]);
    let mut juntas = vec![];
    for (ie, ip) in &lay.juntas {
        let tipo = if lay.folga_axial { JointKind::Cylindrical } else { JointKind::Revolute };
        juntas.push(asm.add_joint(tipo, JRef { comp: comps[*ie], frame: f }, JRef { comp: comps[*ip], frame: f }));
    }
    let mut acopl = vec![];
    for (a, b, r) in &lay.acoplamentos {
        acopl.push(asm.add_coupling(CouplingKind::Gear, juntas[*a], juntas[*b], &format!("{r}")));
    }
    for g in &lay.rigidos {
        asm.add_rigid(g.iter().map(|i| comps[*i]).collect());
    }
    for (k, giro) in lay.comandos.iter().copied().chain([(lay.motora, rpm)]) {
        if let Some(j) = asm.joints.iter_mut().find(|j| j.id == juntas[k]) {
            j.drive_angle = Some(comando_de_giro(giro));
        }
    }
    let id = asm.alloc();
    asm.transmissoes.push(RegistroTransmissao { id, nome: e.rotulo().to_string(), elemento: e.clone(), rpm, torque, comps: comps.clone(), juntas: juntas.clone(), acoplamentos: acopl, motora: Some(juntas[lay.motora]) });
    Ok(RelatorioTransmissao { registro: id, comps, juntas, razao: lay.razao, distancia_centros: lay.distancia_centros, avisos: lay.avisos })
}

/// Edita um elemento já gerado: apaga o que o registro criou e gera o conjunto de novo com os novos
/// parâmetros, no mesmo lugar (a posição do primeiro eixo é mantida) e com o mesmo registro.
pub fn editar_transmissao(asm: &mut Assembly, dir: Option<&Path>, registro: Id, novo: &Elemento, rpm: f64, torque: f64) -> Result<RelatorioTransmissao, String> {
    let i = asm.transmissoes.iter().position(|r| r.id == registro).ok_or("elemento de transmissão não encontrado")?;
    let antigo = asm.transmissoes[i].clone();
    let lugar_antigo = antigo.comps.first().and_then(|c| asm.node(*c)).map(|n| n.placement).unwrap_or_default();
    // a posição do primeiro eixo é preservada: base = antigo ∘ (novo primeiro eixo)⁻¹
    let lay = montar_layout(novo)?;
    let base = lugar_antigo.then(&lay.pecas[0].lugar.inverse());
    let backup = asm.clone();
    for c in &antigo.comps {
        asm.delete(*c);
    }
    asm.transmissoes.retain(|r| r.id != registro);
    match inserir_transmissao(asm, dir, novo, rpm, torque, &base) {
        Ok(mut rel) => {
            // mantém o id do registro e a posição na lista
            if let Some(r) = asm.transmissoes.iter_mut().find(|r| r.id == rel.registro) {
                r.id = registro;
            }
            let ultimo = asm.transmissoes.len() - 1;
            asm.transmissoes.swap(i.min(ultimo), ultimo);
            rel.registro = registro;
            Ok(rel)
        }
        Err(e) => {
            *asm = backup;
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solve::solve_with;
    use std::collections::HashMap;

    fn pasta(nome: &str) -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!("fenkra_transmissao_{nome}"));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn em_t(a: &mut Assembly, t: f64) -> bool {
        let mut extra = HashMap::new();
        extra.insert("t".to_string(), t);
        solve_with(a, &extra).converged
    }

    fn angulo_de(a: &Assembly, junta: Id, t: f64) -> f64 {
        let mut b = a.clone();
        assert!(em_t(&mut b, t));
        let pl: HashMap<Id, Placement> = b.nodes.iter().map(|n| (n.id, n.placement)).collect();
        crate::kin::measure(b.joints.iter().find(|j| j.id == junta).unwrap(), &pl).unwrap().0
    }

    /// Giro de uma peça em volta do Z do mundo, da posição de `a` até a de `b`, em graus.
    fn giro_em_z(a: &Assembly, b: &Assembly, c: Id) -> f64 {
        let ang = |m: &Assembly| {
            let p = m.node(c).unwrap().placement.dir([1., 0., 0.]);
            p[1].atan2(p[0]).to_degrees()
        };
        let d = ang(b) - ang(a);
        (d + 540.).rem_euclid(360.) - 180.
    }

    #[test]
    fn diferencial_de_caixa_solta_gira_na_media_das_laterais() {
        let dir = pasta("caixa_solta");
        let dif = Elemento::Diferencial { lateral: ParamConica { dentes: 24, dentes_par: 16, folga_dentes: 0.1, ..Default::default() }, caixa_solta: true, giro_relativo: 20. };
        let mut a = Assembly::new("Diferencial");
        let r = inserir_transmissao(&mut a, Some(&dir), &dif, 60., 5., &Placement::default()).unwrap();
        assert_eq!(a.rigid.len(), 1, "os eixos dos satélites e das laterais andam com a caixa");
        assert_eq!(a.joints.len(), 5);
        assert_eq!(a.nodes.iter().filter(|n| n.fixed && !matches!(n.kind, NodeKind::Group)).count(), 1, "só o eixo da caixa fica preso");
        let lugares = |t: f64| {
            let mut b = a.clone();
            assert!(em_t(&mut b, t), "t = {t}");
            b
        };
        let (zero, depois) = (lugares(0.), lugares(0.05));
        // 60 rpm = 360°/s na caixa e 20 rpm = 120°/s das laterais em relação a ela: 18° e 6° em 0,05 s
        let caixa = r.comps[r.comps.len() - 1];
        let laterais: Vec<Id> = [1, 5].iter().map(|k| r.comps[*k]).collect();
        let (c, l1, l2) = (giro_em_z(&zero, &depois, caixa), giro_em_z(&zero, &depois, laterais[0]), giro_em_z(&zero, &depois, laterais[1]));
        assert!((c - 18.).abs() < 1e-3, "caixa: {c}");
        assert!(((l1 + l2) / 2. - c).abs() < 1e-3, "ω caixa = (ω1 + ω2) / 2: {l1} {l2} {c}");
        assert!(((l1 - l2).abs() - 12.).abs() < 1e-3, "as laterais se afastam ±6°: {l1} {l2}");
        // no cálculo a saída é a média das laterais: gira com a caixa, no mesmo sentido
        let calc = calcular_elemento(&dif, Entrada { rpm: 60., torque: 5. }).unwrap();
        assert!((calc.rpm_saida - 60.).abs() < 1e-9, "{}", calc.rpm_saida);
        // caixa presa continua como era, sem grupo rígido
        let presa = Elemento::Diferencial { lateral: ParamConica { dentes: 24, dentes_par: 16, ..Default::default() }, caixa_solta: false, giro_relativo: 0. };
        let mut b = Assembly::new("Presa");
        inserir_transmissao(&mut b, Some(&dir), &presa, 60., 5., &Placement::default()).unwrap();
        assert!(b.rigid.is_empty());
    }

    #[test]
    fn caixa_solta_nao_bate_nos_dentes_enquanto_gira() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let dir = pasta("caixa_interferencia");
        let dif = Elemento::Diferencial { lateral: ParamConica { dentes: 24, dentes_par: 16, folga_dentes: 0.1, ..Default::default() }, caixa_solta: true, giro_relativo: 20. };
        let mut a = Assembly::new("Diferencial");
        inserir_transmissao(&mut a, Some(&dir), &dif, 60., 5., &Placement::default()).unwrap();
        for t in [0., 0.02] {
            let mut b = a.clone();
            assert!(em_t(&mut b, t));
            let i = crate::view::interferences(&b, Some(&dir), &mut crate::load::PartCache::default(), 4.0);
            // os eixos dos satélites entram na caixa de propósito: é ela que os segura
            let grupo = &a.rigid[0].comps;
            let batidas: Vec<_> = i.iter().filter(|x| !(grupo.contains(&x.0) && grupo.contains(&x.1))).collect();
            assert!(batidas.is_empty(), "t = {t}: {i:?}");
        }
    }

    #[test]
    fn cadeia_de_dois_estagios_soma_e_segue_para_a_simulacao() {
        let dir = pasta("cadeia");
        let mut a = Assembly::new("Redutor");
        assert!(cadeia_da_montagem(&a).is_none());
        inserir_transmissao(&mut a, Some(&dir), &par_reto(), 600., 5., &Placement::default()).unwrap();
        let r2 = inserir_transmissao(&mut a, Some(&dir), &par_reto(), 1., 1., &Placement { origin: [0., 200., 0.], ..Default::default() }).unwrap();
        let c = cadeia_da_montagem(&a).unwrap().unwrap();
        assert_eq!(c.estagios.len(), 2);
        // duas reduções de −1/2: um quarto do giro, no mesmo sentido da entrada
        assert!((c.voltas_total - 0.25).abs() < 1e-9 && (c.rpm_saida - 150.).abs() < 1e-6, "{c:?}");
        assert!(c.torque_saida > 5. * 3.5 && c.torque_saida < 20., "o torque sobe, menos a perda: {}", c.torque_saida);
        let aplicado = aplicar_cadeia(&mut a).unwrap();
        assert_eq!(aplicado, c);
        assert!((a.transmissoes[1].rpm + 300.).abs() < 1e-6, "o segundo recebe o que o primeiro entrega: {}", a.transmissoes[1].rpm);
        // o segundo par anda na simulação com a entrada nova: 0,01 s a −300 rpm = −18°, e a saída +9°
        let (j_in, j_out) = (r2.juntas[0], r2.juntas[1]);
        assert_eq!(a.transmissoes[1].motora, Some(j_in));
        assert!((angulo_de(&a, j_in, 0.01) + 18.).abs() < 1e-3);
        assert!((angulo_de(&a, j_out, 0.01) - 9.).abs() < 1e-3);
    }

    fn par_reto() -> Elemento {
        Elemento::Engrenagens { pinhao: ParamEngrenagem { furo: 8., ..Default::default() }, dentes_coroa: 40 }
    }

    #[test]
    fn insere_juntas_acoplamento_e_comando_de_giro_com_a_relacao_certa() {
        let dir = pasta("par");
        let mut a = Assembly::new("Redutor");
        let r = inserir_transmissao(&mut a, Some(&dir), &par_reto(), 600., 5., &Placement::default()).unwrap();
        // 2 eixos + 2 engrenagens; 2 juntas de revolução; 1 acoplamento −20/40; motora com 3600°/s
        assert_eq!(r.comps.len(), 4);
        assert_eq!(a.joints.len(), 2);
        assert_eq!(a.couplings.len(), 1);
        assert_eq!(a.couplings[0].ratio, "-0.5");
        assert!((r.razao + 0.5).abs() < 1e-12 && (r.distancia_centros - 60.).abs() < 1e-9);
        let motora = a.joints.iter().find(|j| j.drive_angle.is_some()).unwrap();
        assert_eq!(motora.drive_angle.as_deref(), Some("3600*t"));
        assert_eq!(a.transmissoes.len(), 1);
        // a engrenagem 2 fica a 60 mm do eixo 1 e a simulação anda com a relação: 0,01 s = 36° e −18°
        let (j1, j2) = (a.joints[0].id, a.joints[1].id);
        assert!(em_t(&mut a.clone(), 0.));
        let (g1, g2) = (angulo_de(&a, j1, 0.01), angulo_de(&a, j2, 0.01));
        assert!((g1 - 36.).abs() < 0.05 && (g2 + 18.).abs() < 0.05, "{g1} {g2}");
        // grava e relê: o registro editável viaja com a montagem
        let b = Assembly::from_json(&a.to_json()).unwrap();
        assert_eq!(b.transmissoes, a.transmissoes);
        // sem pasta, recusa e não insere nada
        let mut c = Assembly::new("x");
        assert!(inserir_transmissao(&mut c, None, &par_reto(), 600., 5., &Placement::default()).is_err());
        assert!(c.nodes.is_empty());
    }

    #[test]
    fn editar_regera_no_mesmo_lugar_com_a_nova_relacao() {
        let dir = pasta("editar");
        let mut a = Assembly::new("Redutor");
        let base = Placement::default().moved([100., 50., 0.], [0.; 3], [0.; 3]);
        let r = inserir_transmissao(&mut a, Some(&dir), &par_reto(), 600., 5., &base).unwrap();
        let primeiro = a.node(r.comps[0]).unwrap().placement.origin;
        // edita: coroa de 60 dentes e módulo 2,5
        let novo = Elemento::Engrenagens { pinhao: ParamEngrenagem { furo: 8., modulo: 2.5, ..Default::default() }, dentes_coroa: 60 };
        let r2 = editar_transmissao(&mut a, Some(&dir), r.registro, &novo, 900., 5.).unwrap();
        assert_eq!(r2.registro, r.registro);
        assert_eq!(a.transmissoes.len(), 1);
        assert_eq!(a.transmissoes[0].elemento, novo);
        assert_eq!(a.joints.len(), 2);
        assert_eq!(a.couplings.len(), 1);
        assert_eq!(a.couplings[0].ratio, format!("{}", -20. / 60.));
        assert!((r2.distancia_centros - 2.5 * 80. / 2.).abs() < 1e-9);
        let primeiro2 = a.node(r2.comps[0]).unwrap().placement.origin;
        assert!(norm(sub(primeiro, primeiro2)) < 1e-9, "o eixo 1 não saiu do lugar: {primeiro:?} {primeiro2:?}");
        assert_eq!(a.joints.iter().find(|j| j.drive_angle.is_some()).unwrap().drive_angle.as_deref(), Some("5400*t"));
        // edição inválida volta a montagem ao que era
        let antes = a.clone();
        let ruim = Elemento::Engrenagens { pinhao: ParamEngrenagem { dentes: 3, ..Default::default() }, dentes_coroa: 60 };
        assert!(editar_transmissao(&mut a, Some(&dir), r.registro, &ruim, 900., 5.).is_err());
        assert_eq!(a, antes);
    }

    #[test]
    fn espinha_de_peixe_vem_sem_trava_axial_e_sem_interferir() {
        let dir = pasta("espinha");
        let e = Elemento::Engrenagens { pinhao: ParamEngrenagem { tipo: TipoEngrenagem::Helicoidal, espinha_peixe: true, angulo_helice: 20., furo: 8., largura: 12., ..Default::default() }, dentes_coroa: 36 };
        let mut a = Assembly::new("V");
        let r = inserir_transmissao(&mut a, Some(&dir), &e, 300., 2., &Placement::default()).unwrap();
        assert!(a.mates.is_empty(), "nenhuma restrição axial: a dupla hélice é quem trava");
        assert!(a.joints.iter().all(|j| matches!(j.kind, JointKind::Cylindrical)), "juntas cilíndricas: sem retenção axial");
        assert!((r.razao + 20. / 36.).abs() < 1e-12);
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        // sem interferir na posição inicial e depois de girar
        for t in [0., 0.03] {
            let mut b = a.clone();
            assert!(em_t(&mut b, t));
            let i = crate::view::interferences(&b, Some(&dir), &mut crate::load::PartCache::default(), 0.5);
            assert!(i.is_empty(), "t = {t}: {i:?}");
        }
    }

    #[test]
    fn conicas_e_diferencial_engrenam_sem_interferir() {
        let dir = pasta("conicas");
        let par = Elemento::Conicas { par: ParamConica { dentes: 20, dentes_par: 20, ..Default::default() } };
        let mut a = Assembly::new("Cônicas");
        let r = inserir_transmissao(&mut a, Some(&dir), &par, 600., 5., &Placement::default()).unwrap();
        assert_eq!(a.couplings[0].ratio, "-1");
        assert!((r.razao + 1.).abs() < 1e-12);
        // os eixos das duas peças são perpendiculares
        let eixos: Vec<V3> = a.joints.iter().map(|j| a.node(j.b.comp).unwrap().placement.dir([0., 0., 1.])).collect();
        assert!(dot(eixos[0], eixos[1]).abs() < 1e-9, "{eixos:?}");
        let dif = Elemento::Diferencial { lateral: ParamConica { dentes: 24, dentes_par: 16, folga_dentes: 0.1, ..Default::default() }, caixa_solta: false, giro_relativo: 0. };
        let mut d = Assembly::new("Diferencial");
        inserir_transmissao(&mut d, Some(&dir), &dif, 600., 5., &Placement::default()).unwrap();
        assert_eq!(d.joints.len(), 4);
        assert_eq!(d.couplings.len(), 3);
        // zs ímpar não fecha com dois satélites
        let ruim = Elemento::Diferencial { lateral: ParamConica { dentes: 21, dentes_par: 12, ..Default::default() }, caixa_solta: false, giro_relativo: 0. };
        assert!(inserir_transmissao(&mut Assembly::new("x"), Some(&dir), &ruim, 600., 5., &Placement::default()).is_err());
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        // o ajuste do flanco por spline deixa um resíduo de 2 a 3 mm³ nos pares cônicos com ângulos
        // diferentes de 45° (medido: 0,03 a 2,4); fase errada por 0,03 rad já passa de 7 mm³, e meio dente de 20 mm³
        for (nome, asm) in [("par", &a), ("diferencial", &d)] {
            for t in [0., 0.02] {
                let mut b = asm.clone();
                assert!(em_t(&mut b, t));
                let i = crate::view::interferences(&b, Some(&dir), &mut crate::load::PartCache::default(), 4.0);
                assert!(i.is_empty(), "{nome} t = {t}: {i:?}");
            }
        }
        // controle: com a lateral 2 inclinada de 0,05 rad o encaixe falha de verdade
        let mut c = d.clone();
        let sat1 = d.joints[2].b.comp;
        let o = c.node(sat1).unwrap().placement.origin;
        let n = c.node_mut(sat1).unwrap();
        n.placement = n.placement.moved([0.; 3], [0.05, 0., 0.], o);
        let i = crate::view::interferences(&c, Some(&dir), &mut crate::load::PartCache::default(), 4.0);
        assert!(!i.is_empty(), "o controle deveria interferir");
    }

    #[test]
    fn sem_fim_engrena_sem_interferir_e_gira_na_relacao() {
        let dir = pasta("semfim");
        let p = ParamSemFim { comprimento: 13., ..Default::default() };
        let mut a = Assembly::new("Sem-fim");
        let r = inserir_transmissao(&mut a, Some(&dir), &Elemento::SemFim { p: p.clone() }, 300., 1., &Placement::default()).unwrap();
        assert!((r.razao + 1. / 30.).abs() < 1e-12 && (r.distancia_centros - 40.).abs() < 1e-9);
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        for t in [0., 0.01, 0.02] {
            let mut b = a.clone();
            assert!(em_t(&mut b, t));
            let i = crate::view::interferences(&b, Some(&dir), &mut crate::load::PartCache::default(), 2.0);
            assert!(i.is_empty(), "t = {t}: {i:?}");
        }
    }

    #[test]
    fn os_rolos_da_corrente_andam_pelo_laco_com_a_roda_motora() {
        let dir = pasta("rolos");
        let e = Elemento::Corrente { corrente: "ISO 08B-1".into(), dentes1: 17, dentes2: 34, centros: 400., furo1: 10., furo2: 12. };
        let mut a = Assembly::new("Corrente");
        inserir_transmissao(&mut a, Some(&dir), &e, 300., 5., &Placement::default()).unwrap();
        let reg = a.transmissoes[0].clone();
        let (p0, rolo) = rolos_da_corrente(&a, &reg).unwrap();
        assert!((rolo - 8.51).abs() < 1e-9 && p0.len() == 90);
        // 0,05 s a 300 rpm = 90° da roda menor: cada rolo anda r1·π/2 pelo laço
        let mut b = a.clone();
        assert!(em_t(&mut b, 0.05));
        let (p1, _) = rolos_da_corrente(&b, &b.transmissoes[0].clone()).unwrap();
        let r1 = fenkra_solidos::transmissao_dados::diametro_primitivo_roda(12.7, 17) / 2.;
        let roda1 = b.node(reg.comps[1]).unwrap().placement;
        let c = norm(sub(b.node(reg.comps[3]).unwrap().placement.origin, roda1.origin));
        let cam = fenkra_solidos::transmissao_corrente::Caminho::novo(r1, fenkra_solidos::transmissao_dados::diametro_primitivo_roda(12.7, 34) / 2., c).unwrap();
        // cada rolo está onde o laço o põe depois de avançar r1 · 90° (conta independente da função)
        let esperado: Vec<V3> = cam.elos(90, r1 * std::f64::consts::FRAC_PI_2).iter().map(|q| roda1.point([q.x, q.y, 7.75 / 2.])).collect();
        for (q, e) in p1.iter().zip(&esperado) {
            assert!(norm(sub(*q, *e)) < 1e-6, "{q:?} {e:?}");
        }
        assert!(norm(sub(p1[0], p0[0])) > 1., "o rolo 0 saiu do lugar");
        // sem corrente no registro, nada a desenhar
        let reto = inserir_transmissao(&mut a.clone(), Some(&dir), &par_reto(), 600., 5., &Placement::default());
        assert!(reto.is_ok());
        let mut c = Assembly::new("x");
        inserir_transmissao(&mut c, Some(&dir), &par_reto(), 600., 5., &Placement::default()).unwrap();
        assert!(rolos_da_corrente(&c, &c.transmissoes[0].clone()).is_none());
    }

    #[test]
    fn corrente_e_correia_com_distancia_corrigida_e_razao() {
        let dir = pasta("corrente");
        let e = Elemento::Corrente { corrente: "ISO 08B-1".into(), dentes1: 17, dentes2: 34, centros: 400., furo1: 10., furo2: 12. };
        let lay = montar_layout(&e).unwrap();
        // 2 eixos + 2 rodas + a corrente; o acoplamento é 17/34 no mesmo sentido
        assert_eq!(lay.pecas.len(), 5);
        assert!((lay.razao - 0.5).abs() < 1e-12 && lay.distancia_centros > 400. && lay.distancia_centros < 415.);
        let mut a = Assembly::new("Corrente");
        inserir_transmissao(&mut a, Some(&dir), &e, 300., 5., &Placement::default()).unwrap();
        assert_eq!(a.couplings[0].ratio, "0.5");
        let (j1, j2) = (a.joints[0].id, a.joints[1].id);
        let (g1, g2) = (angulo_de(&a, j1, 0.02), angulo_de(&a, j2, 0.02));
        assert!((g2 / g1 - 0.5).abs() < 1e-3, "{g1} {g2}");
        // correia dentada: 20/40 dentes GT2, centros pedidos 100
        let c = Elemento::Correia { designacao: "GT2".into(), dentes1: 20, dentes2: 40, diametro1: 0., diametro2: 0., centros: 100., largura: 6., furo1: 5., furo2: 5. };
        let lay = montar_layout(&c).unwrap();
        assert_eq!(lay.pecas.len(), 5);
        assert!((lay.razao - 0.5).abs() < 1e-12);
        // dado sem duas fontes não gera
        let v = Elemento::Correia { designacao: "V B".into(), dentes1: 0, dentes2: 0, diametro1: 80., diametro2: 160., centros: 300., largura: 0., furo1: 12., furo2: 12. };
        assert!(montar_layout(&v).is_err());
        let ruim = Elemento::Corrente { corrente: "ANSI 40".into(), dentes1: 17, dentes2: 34, centros: 400., furo1: 10., furo2: 12. };
        assert!(montar_layout(&ruim).is_err());
        // o cálculo do elemento usa o mesmo estágio: 17/34 por corrente
        let r = calcular_elemento(&e, Entrada { rpm: 300., torque: 5. }).unwrap();
        assert!((r.razao_total - 2.).abs() < 1e-12 && (r.rpm_saida - 150.).abs() < 1e-9);
    }
}
