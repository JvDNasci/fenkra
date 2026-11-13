// SPDX-License-Identifier: LGPL-2.1-or-later
//! Gerar elemento de transmissão (B340): um comando só para engrenagens, cônicas, diferencial,
//! sem-fim, corrente e correia.
//!
//! - **Prévia 3D** do conjunto inteiro (peças e eixos) como fantasma, nas posições em que
//!   [`fenkra_montagem::transmissao::inserir_transmissao`] as põe (a conta é a mesma: `montar_layout`).
//! - **Cálculo ao vivo** (B343): relação, voltas, velocidade angular, torque, potência, rendimento e
//!   perda, com cadeia de vários estágios; o rpm de entrada vira o comando de giro da junta motora.
//! - **Adaptação de projeto**: a pessoa fixa distância, razão, módulo ou espaço e o comando propõe o
//!   que cabe (`transmissao_proposta`).
//! - **Usinagens** (B345) sobre a engrenagem padronizada.
//! - **Edição**: os elementos já gerados ficam na lista; "Editar" recarrega os parâmetros e "Aplicar
//!   edição" gera de novo no mesmo lugar.

use crate::app::{fmt, App};
use crate::i18n::tr;
use eframe::egui::{self, RichText};
use fenkra_kernel::Mesh;
use fenkra_montagem::transmissao::{editar_transmissao, inserir_transmissao, montar_layout, Elemento, Layout};
use fenkra_core::Id;
use fenkra_montagem::Placement;
use fenkra_solidos::engrenagens::{ParamEngrenagem, TipoEngrenagem};
use fenkra_solidos::engrenagens_conicas::{ParamConica, ParamSemFim};
use fenkra_solidos::transmissao_calc::{calcular_cadeia as calcular_cadeia_com, Entrada, Estagio, ResultadoCadeia, TipoEstagio, REFERENCIA};
use fenkra_solidos::transmissao_dados::{self as dados, FamiliaCorreia};
use fenkra_solidos::transmissao_proposta::{propor, Familia, Proposta, Restricoes};
use fenkra_solidos::usinagens::{GeoBase, Usinagem};
use fenkra_solidos::recompute;
use std::sync::Arc;

/// Opacidade das peças fantasma.
const ALFA: f32 = 0.42;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tipo {
    Retas,
    Helicoidais,
    EspinhaDePeixe,
    Interna,
    Conicas,
    Diferencial,
    SemFim,
    Corrente,
    Correia,
}

impl Tipo {
    pub const ALL: [Tipo; 9] = [Tipo::Retas, Tipo::Helicoidais, Tipo::EspinhaDePeixe, Tipo::Interna, Tipo::Conicas, Tipo::Diferencial, Tipo::SemFim, Tipo::Corrente, Tipo::Correia];

    pub fn rotulo(self) -> &'static str {
        match self {
            Tipo::Retas => "Retas",
            Tipo::Helicoidais => "Helicoidais",
            Tipo::EspinhaDePeixe => "Espinha de peixe",
            Tipo::Interna => "Interna",
            Tipo::Conicas => "Cônicas 90°",
            Tipo::Diferencial => "Diferencial",
            Tipo::SemFim => "Sem-fim e coroa",
            Tipo::Corrente => "Corrente",
            Tipo::Correia => "Correia",
        }
    }

    fn engrenagem(self) -> bool {
        matches!(self, Tipo::Retas | Tipo::Helicoidais | Tipo::EspinhaDePeixe | Tipo::Interna)
    }
}

/// Malhas do conjunto da prévia, para a chave dos parâmetros que as geram.
pub struct MalhasLayout {
    pub chave: String,
    pub layout: Layout,
    pub malhas: Vec<Mesh>,
}

/// Estado do comando.
#[derive(Clone)]
pub struct TransUi {
    pub tipo: Tipo,
    pub pinhao: ParamEngrenagem,
    pub dentes_coroa: u32,
    pub conica: ParamConica,
    pub diferencial: ParamConica,
    pub semfim: ParamSemFim,
    pub corrente: String,
    pub correia: String,
    pub z1: u32,
    pub z2: u32,
    pub centros: f64,
    pub furo1: f64,
    pub furo2: f64,
    pub dp1: f64,
    pub dp2: f64,
    pub largura_correia: f64,
    /// entrada do cálculo e da simulação
    pub rpm: f64,
    pub torque: f64,
    /// estágios seguintes ao gerado, só para o cálculo da cadeia
    pub extra: Vec<Estagio>,
    pub restr: Restricoes,
    pub usa_distancia: bool,
    pub usa_razao: bool,
    pub usa_modulo: bool,
    pub usa_espaco: bool,
    pub espaco: f64,
    pub familia: Familia,
    pub propostas: Vec<Proposta>,
    pub editando: Option<Id>,
    pub msg: Vec<String>,
    pub malhas: Option<Arc<MalhasLayout>>,
    /// distância dos dois pontos clicados no 3D (mm), se houver
    pub pontos: Option<f64>,
    /// pontos clicados no 3D; `medindo` diz que o próximo clique entra aqui
    pub marcas: Vec<[f64; 3]>,
    pub medindo: bool,
    /// diferencial: a caixa gira, e `giro_relativo` (rpm) é o das laterais em relação a ela
    pub caixa_solta: bool,
    pub giro_relativo: f64,
}

impl Default for TransUi {
    fn default() -> Self {
        TransUi {
            tipo: Tipo::Retas,
            pinhao: ParamEngrenagem { furo: 8., ..Default::default() },
            dentes_coroa: 40,
            conica: ParamConica::default(),
            diferencial: ParamConica { dentes: 24, dentes_par: 16, ..Default::default() },
            semfim: ParamSemFim { comprimento: 13., ..Default::default() },
            corrente: "ISO 08B-1".into(),
            correia: "GT2".into(),
            z1: 17,
            z2: 34,
            centros: 400.,
            furo1: 10.,
            furo2: 12.,
            dp1: 80.,
            dp2: 160.,
            largura_correia: 6.,
            rpm: 600.,
            torque: 5.,
            extra: vec![],
            restr: Restricoes { distancia: Some(90.), razao: Some(2.), angulo_pressao: 20., ..Default::default() },
            usa_distancia: true,
            usa_razao: true,
            usa_modulo: false,
            usa_espaco: false,
            espaco: 150.,
            familia: Familia::Retas,
            propostas: vec![],
            editando: None,
            msg: vec![],
            malhas: None,
            pontos: None,
            marcas: vec![],
            medindo: false,
            caixa_solta: false,
            giro_relativo: 20.,
        }
    }
}

impl TransUi {
    /// Elemento descrito pelos campos.
    pub fn elemento(&self) -> Elemento {
        match self.tipo {
            Tipo::Retas | Tipo::Helicoidais | Tipo::EspinhaDePeixe | Tipo::Interna => {
                let (tipo, espinha) = match self.tipo {
                    Tipo::Retas => (TipoEngrenagem::Reta, false),
                    Tipo::Helicoidais => (TipoEngrenagem::Helicoidal, false),
                    Tipo::EspinhaDePeixe => (TipoEngrenagem::Helicoidal, true),
                    _ => (TipoEngrenagem::Interna, false),
                };
                Elemento::Engrenagens { pinhao: ParamEngrenagem { tipo, espinha_peixe: espinha, ..self.pinhao.clone() }, dentes_coroa: self.dentes_coroa }
            }
            Tipo::Conicas => Elemento::Conicas { par: self.conica.clone() },
            Tipo::Diferencial => Elemento::Diferencial { lateral: self.diferencial.clone(), caixa_solta: self.caixa_solta, giro_relativo: self.giro_relativo },
            Tipo::SemFim => Elemento::SemFim { p: self.semfim.clone() },
            Tipo::Corrente => Elemento::Corrente { corrente: self.corrente.clone(), dentes1: self.z1, dentes2: self.z2, centros: self.centros, furo1: self.furo1, furo2: self.furo2 },
            Tipo::Correia => Elemento::Correia { designacao: self.correia.clone(), dentes1: self.z1, dentes2: self.z2, diametro1: self.dp1, diametro2: self.dp2, centros: self.centros, largura: self.largura_correia, furo1: self.furo1, furo2: self.furo2 },
        }
    }

    /// Recarrega os campos de um elemento já gerado (edição).
    pub fn carregar(&mut self, e: &Elemento) {
        match e {
            Elemento::Engrenagens { pinhao, dentes_coroa } => {
                self.tipo = match (pinhao.tipo, pinhao.espinha_peixe) {
                    (TipoEngrenagem::Helicoidal, true) => Tipo::EspinhaDePeixe,
                    (TipoEngrenagem::Helicoidal, false) => Tipo::Helicoidais,
                    (TipoEngrenagem::Interna, _) => Tipo::Interna,
                    _ => Tipo::Retas,
                };
                self.pinhao = pinhao.clone();
                self.dentes_coroa = *dentes_coroa;
            }
            Elemento::Conicas { par } => {
                self.tipo = Tipo::Conicas;
                self.conica = par.clone();
            }
            Elemento::Diferencial { lateral, caixa_solta, giro_relativo } => {
                self.tipo = Tipo::Diferencial;
                self.diferencial = lateral.clone();
                self.caixa_solta = *caixa_solta;
                self.giro_relativo = *giro_relativo;
            }
            Elemento::SemFim { p } => {
                self.tipo = Tipo::SemFim;
                self.semfim = p.clone();
            }
            Elemento::Corrente { corrente, dentes1, dentes2, centros, furo1, furo2 } => {
                self.tipo = Tipo::Corrente;
                (self.corrente, self.z1, self.z2, self.centros, self.furo1, self.furo2) = (corrente.clone(), *dentes1, *dentes2, *centros, *furo1, *furo2);
            }
            Elemento::Correia { designacao, dentes1, dentes2, diametro1, diametro2, centros, largura, furo1, furo2 } => {
                self.tipo = Tipo::Correia;
                (self.correia, self.z1, self.z2, self.dp1, self.dp2, self.centros, self.largura_correia, self.furo1, self.furo2) = (designacao.clone(), *dentes1, *dentes2, *diametro1, *diametro2, *centros, *largura, *furo1, *furo2);
            }
        }
    }

    /// Aplica uma proposta da adaptação de projeto aos campos.
    pub fn aplicar(&mut self, p: &Proposta) {
        match p.familia {
            Familia::Retas | Familia::Helicoidais => {
                self.tipo = if p.familia == Familia::Retas { Tipo::Retas } else { Tipo::Helicoidais };
                self.pinhao.modulo = p.modulo_ou_passo;
                self.pinhao.dentes = p.n1;
                self.dentes_coroa = p.n2;
                if p.familia == Familia::Helicoidais {
                    self.pinhao.angulo_helice = p.angulo_helice;
                }
            }
            Familia::Corrente => {
                self.tipo = Tipo::Corrente;
                self.corrente = p.designacao.clone();
                (self.z1, self.z2, self.centros) = (p.n1, p.n2, p.distancia);
            }
            Familia::CorreiaDentada => {
                self.tipo = Tipo::Correia;
                self.correia = p.designacao.clone();
                (self.z1, self.z2, self.centros) = (p.n1, p.n2, p.distancia);
            }
        }
    }

    fn restricoes(&self) -> Restricoes {
        Restricoes {
            distancia: self.usa_distancia.then_some(self.restr.distancia.unwrap_or(90.)),
            razao: self.usa_razao.then_some(self.restr.razao.unwrap_or(2.)),
            modulo: self.usa_modulo.then_some(self.restr.modulo.unwrap_or(2.)),
            espaco: self.usa_espaco.then_some(self.espaco),
            angulo_pressao: 20.,
        }
    }
}

/// Resultado do cálculo ao vivo do elemento e dos estágios extras.
pub fn calculo(t: &TransUi) -> Result<ResultadoCadeia, String> {
    let mut est = vec![t.elemento().estagio()];
    est.extend(t.extra.iter().cloned());
    calcular_cadeia_com(&est, Entrada { rpm: t.rpm, torque: t.torque })
}

fn chave(t: &TransUi) -> String {
    format!("{:?}", t.elemento())
}

/// Refaz as malhas da prévia quando os parâmetros mudam (não durante um arrasto de campo).
pub fn atualizar_malhas(t: &mut TransUi, ctx: &egui::Context) -> bool {
    let k = chave(t);
    if t.malhas.as_ref().is_some_and(|m| m.chave == k) {
        return false;
    }
    if ctx.input(|i| i.pointer.any_down()) && t.malhas.is_some() {
        return false;
    }
    t.malhas = montar_layout(&t.elemento())
        .ok()
        .and_then(|layout| {
            let malhas: Option<Vec<Mesh>> = layout.pecas.iter().map(|p| recompute(&p.doc).bodies.into_iter().next().map(|b| b.mesh)).collect();
            malhas.map(|malhas| Arc::new(MalhasLayout { chave: k, layout, malhas }))
        });
    true
}

/// Peças da prévia como fantasma na cena, nas posições finais.
pub fn na_cena(st: Option<&crate::asm_ui::AsmState>, v: &egui::Visuals, cena: &mut crate::render::Scene) {
    let Some(t) = st.and_then(|s| s.trans_open.as_ref()) else { return };
    let Some(m) = t.malhas.as_ref() else { return };
    let rgb = |c: egui::Color32| [c.r(), c.g(), c.b()];
    let cores = [crate::theme::accent(v), crate::theme::st_ok()];
    let mut n = 0;
    for (malha, peca) in m.malhas.iter().zip(&m.layout.pecas) {
        let cor = if peca.fixa {
            v.weak_text_color()
        } else {
            n += 1;
            cores[(n - 1) % 2]
        };
        let tr = fenkra_montagem::view::transformed(malha, &peca.lugar);
        crate::previa::malha_fantasma(&mut cena.ghost, &tr, rgb(cor), ALFA);
    }
}

/// Rolos das correntes da montagem, andando pelo laço com a roda motora (a corrente em si é um corpo
/// estático de placas; os rolos saem do corpo e são desenhados aqui na posição da simulação).
pub fn overlay(app: &mut App, ui: &mut egui::Ui, rect: egui::Rect, cam: &crate::render::Camera) {
    if app.settings.bench != crate::settings::Bench::Montagem {
        return;
    }
    let Some(st) = app.asm.as_ref() else { return };
    if let Some(t) = &st.trans_open {
        marcas_de_medida(t, ui, rect, cam);
    }
    if !st.asm.transmissoes.iter().any(|r| matches!(r.elemento, Elemento::Corrente { .. })) {
        return;
    }
    let v = ui.visuals().clone();
    let p = ui.painter_at(rect);
    let (cor, borda) = (crate::theme::accent(&v), v.window_stroke().color);
    for reg in st.asm.transmissoes.iter().filter(|r| matches!(r.elemento, Elemento::Corrente { .. })) {
        let Some((pontos, rolo)) = fenkra_montagem::transmissao::rolos_da_corrente(&st.asm, reg) else { continue };
        let proj = |q: [f64; 3]| cam.project(rect, glam::Vec3::new(q[0] as f32, q[1] as f32, q[2] as f32));
        for q in &pontos {
            let (Some(c), Some(b)) = (proj(*q), proj([q[0] + rolo / 2., q[1], q[2]])) else { continue };
            let r = (b - c).length().clamp(1.5, 12.);
            p.circle_filled(c, r, cor.gamma_multiply(0.85));
            p.circle_stroke(c, r, egui::Stroke::new(1., borda));
        }
    }
}

/// Pontos clicados para a distância, com a linha e a medida entre eles.
fn marcas_de_medida(t: &TransUi, ui: &egui::Ui, rect: egui::Rect, cam: &crate::render::Camera) {
    let v = ui.visuals();
    let p = ui.painter_at(rect);
    let cor = crate::theme::accent(v);
    let proj = |q: &[f64; 3]| cam.project(rect, glam::Vec3::new(q[0] as f32, q[1] as f32, q[2] as f32));
    let tela: Vec<egui::Pos2> = t.marcas.iter().filter_map(proj).collect();
    for (k, s) in tela.iter().enumerate() {
        p.circle_filled(*s, 5., cor);
        p.text(*s + egui::vec2(8., -8.), egui::Align2::LEFT_BOTTOM, (k + 1).to_string(), egui::FontId::proportional(crate::acessibilidade::fonte_3d(12.)), cor);
    }
    if let ([a, b], Some(d)) = (&tela[..], t.pontos) {
        p.line_segment([*a, *b], egui::Stroke::new(1.5, cor));
        p.text(*a + (*b - *a) / 2. + egui::vec2(0., -10.), egui::Align2::CENTER_BOTTOM, format!("{} mm", fmt(d)), egui::FontId::proportional(crate::acessibilidade::fonte_3d(13.)), crate::theme::readable(cor, v));
    }
}

/// Ponto que o clique pede para a medida entre eixos: o vértice, o centro da aresta circular, o meio da
/// aresta reta, e na parede de um furo o ponto do eixo na altura do clique.
fn ponto_do_clique(st: &mut crate::asm_ui::AsmState, p: crate::picking::Pick) -> Option<[f64; 3]> {
    use crate::picking::Pick;
    use fenkra_montagem::Geom;
    let clicado = match p {
        Pick::Vertex { pos, .. } => pos.map(|v| v as f64),
        Pick::Edge { mid, .. } => mid.map(|v| v as f64),
        Pick::Face { point, .. } => point.map(|v| v as f64),
        Pick::SkPoint { pos, .. } | Pick::CurveVtx { pos, .. } => return Some(pos.map(|v| v as f64)),
        _ => return None,
    };
    let Ok(r) = st.mate_ref(p) else { return Some(clicado) };
    let pl = st.asm.lugar(r.comp).unwrap_or_default();
    Some(match (p, r.geom) {
        (Pick::Vertex { .. }, _) => clicado,
        (Pick::Edge { .. }, Geom::Axis { p: o, d }) if circular(pl.point(o), pl.dir(d), clicado) => pl.point(o),
        (Pick::Face { .. }, Geom::Axis { p: o, d }) => no_eixo(pl.point(o), pl.dir(d), clicado),
        _ => clicado,
    })
}

/// O eixo da aresta passa longe do ponto dela? Numa aresta reta ele passa por cima.
fn circular(o: [f64; 3], d: [f64; 3], q: [f64; 3]) -> bool {
    fenkra_montagem::norm(fenkra_montagem::sub(no_eixo(o, d, q), q)) > 1e-6
}

/// Ponto do eixo mais perto de `q`.
fn no_eixo(o: [f64; 3], d: [f64; 3], q: [f64; 3]) -> [f64; 3] {
    let u = fenkra_montagem::unit(d);
    let k = fenkra_montagem::dot(fenkra_montagem::sub(q, o), u);
    [0, 1, 2].map(|i| o[i] + u[i] * k)
}

impl TransUi {
    /// Recebe um ponto clicado; o segundo fecha a medida e vira a distância pedida.
    pub fn marcar(&mut self, q: [f64; 3]) {
        if self.marcas.len() >= 2 {
            self.marcas.clear();
        }
        self.marcas.push(q);
        if let [a, b] = self.marcas[..] {
            let d = fenkra_montagem::norm(fenkra_montagem::sub(a, b));
            self.pontos = Some(d);
            self.usa_distancia = true;
            self.restr.distancia = Some(d);
            self.medindo = false;
        }
    }
}

/// Cliques na viewport com a medida de dois pontos ligada.
pub fn apos_viewport(app: &mut App) -> bool {
    if !app.asm.as_ref().is_some_and(|s| s.trans_open.as_ref().is_some_and(|t| t.medindo)) {
        return false;
    }
    let picks = std::mem::take(&mut app.picks);
    if let Some(st) = app.asm.as_mut() {
        for p in picks {
            if let Some(q) = ponto_do_clique(st, p) {
                if let Some(t) = st.trans_open.as_mut() {
                    t.marcar(q);
                }
            }
        }
    }
    true
}

fn campo(ui: &mut egui::Ui, rotulo: &str, v: &mut f64, r: std::ops::RangeInclusive<f64>, suf: &str, vel: f64, dica: &str) {
    ui.horizontal(|ui| {
        ui.label(tr(rotulo));
        let resp = ui.add(crate::ui_ext::dv(v).range(r).speed(vel).max_decimals(3).suffix(suf).dica(tr(dica)));
        crate::ui_ext::wheel(ui, &resp, v);
    });
}

fn inteiro(ui: &mut egui::Ui, rotulo: &str, v: &mut u32, r: std::ops::RangeInclusive<u32>, dica: &str) {
    ui.horizontal(|ui| {
        ui.label(tr(rotulo));
        let resp = ui.add(crate::ui_ext::dv(v).range(r).dica(tr(dica)));
        crate::ui_ext::wheel(ui, &resp, v);
    });
}

fn campos_engrenagens(ui: &mut egui::Ui, t: &mut TransUi) {
    campo(ui, "Módulo", &mut t.pinhao.modulo, 0.1..=50., " mm", 0.05, "Tamanho do dente: o diâmetro primitivo é módulo × dentes.");
    inteiro(ui, "Dentes do pinhão (z1)", &mut t.pinhao.dentes, 6..=400, "Dentes da engrenagem menor, a motora.");
    inteiro(ui, "Dentes da coroa (z2)", &mut t.dentes_coroa, 6..=400, "Dentes da engrenagem maior; z2 ÷ z1 é a relação de transmissão.");
    campo(ui, "Ângulo de pressão", &mut t.pinhao.angulo_pressao, 14.5..=30., "°", 0.1, "Inclinação do flanco do dente: 20° é o usual.");
    if matches!(t.tipo, Tipo::Helicoidais | Tipo::EspinhaDePeixe) {
        campo(ui, "Ângulo de hélice", &mut t.pinhao.angulo_helice, -45.0..=45., "°", 0.1, "Inclinação do dente em relação ao eixo; o sinal dá o sentido, e a coroa leva o contrário.");
    }
    campo(ui, "Largura", &mut t.pinhao.largura, 0.1..=1000., " mm", 0.5, "Espessura da engrenagem ao longo do eixo.");
    campo(ui, "Folga entre dentes (par)", &mut t.pinhao.folga_dentes, 0.0..=10., " mm", 0.01, "Jogo entre os dentes do par, dividido entre as duas peças.");
    if t.tipo == Tipo::EspinhaDePeixe {
        campo(ui, "Folga para impressão 3D", &mut t.pinhao.folga_impressao, 0.0..=5., " mm", 0.01, "Folga a mais para peças impressas: o dente fica mais fino e o pé mais fundo, e os dentes não prendem na camada.");
        ui.label(RichText::new(tr("A dupla hélice em V trava o par no sentido axial: não leva anel nem colar no eixo.")).weak().small());
    }
    if t.tipo != Tipo::Interna {
        campo(ui, "Furo central", &mut t.pinhao.furo, 0.0..=1000., " mm", 0.5, "Furo do eixo; também vira o diâmetro do eixo fixo da prévia.");
        campo(ui, "Correção do perfil (x)", &mut t.pinhao.coef_correcao, -1.0..=1., "", 0.01, "Desloca o perfil para evitar rebaixo nos pinhões com poucos dentes.");
    }
}

fn campos_conica(ui: &mut egui::Ui, p: &mut ParamConica, rotulo_par: &str) {
    campo(ui, "Módulo", &mut p.modulo, 0.1..=50., " mm", 0.05, "Módulo na extremidade maior do dente.");
    inteiro(ui, "Dentes desta peça", &mut p.dentes, 8..=200, "Dentes da cônica motora.");
    inteiro(ui, rotulo_par, &mut p.dentes_par, 8..=200, "Dentes da peça conjugada: define o ângulo do cone, e os eixos ficam a 90°.");
    campo(ui, "Ângulo de pressão", &mut p.angulo_pressao, 14.5..=30., "°", 0.1, "Inclinação do flanco do dente: 20° é o usual.");
    campo(ui, "Largura de face (0 = 1/3 da geratriz)", &mut p.largura, 0.0..=500., " mm", 0.5, "Comprimento do dente ao longo da geratriz do cone; o usual é até 1/3 dela.");
    campo(ui, "Folga entre dentes (par)", &mut p.folga_dentes, 0.0..=10., " mm", 0.01, "Jogo entre os dentes do par.");
    campo(ui, "Furo central", &mut p.furo, 0.0..=500., " mm", 0.5, "Furo do eixo.");
}

fn campos_sem_fim(ui: &mut egui::Ui, p: &mut ParamSemFim) {
    campo(ui, "Módulo axial", &mut p.modulo, 0.1..=30., " mm", 0.05, "Módulo medido ao longo do eixo do sem-fim; vale o mesmo módulo transversal da coroa.");
    inteiro(ui, "Entradas (filetes)", &mut p.entradas, 1..=4, "Quantos filetes o sem-fim tem: cada volta dele avança tantos dentes da coroa.");
    campo(ui, "Diâmetro primitivo do sem-fim", &mut p.diametro_primitivo, 5.0..=300., " mm", 0.5, "Diâmetro do cilindro primitivo do sem-fim; define o ângulo de avanço.");
    campo(ui, "Comprimento do sem-fim", &mut p.comprimento, 5.0..=300., " mm", 1., "Comprimento do trecho com filete: pelo menos 2 avanços.");
    inteiro(ui, "Dentes da coroa", &mut p.dentes_coroa, 17..=200, "Dentes da coroa; a relação é dentes da coroa ÷ entradas.");
    campo(ui, "Ângulo de pressão", &mut p.angulo_pressao, 14.5..=30., "°", 0.1, "Ângulo de pressão axial do filete.");
    campo(ui, "Furo do sem-fim", &mut p.furo, 0.0..=200., " mm", 0.5, "Furo do eixo no sem-fim.");
    campo(ui, "Furo da coroa", &mut p.furo_coroa, 0.0..=200., " mm", 0.5, "Furo do eixo na coroa.");
}

fn campos_corrente(ui: &mut egui::Ui, t: &mut TransUi) {
    ui.horizontal(|ui| {
        ui.label(tr("Corrente"));
        egui::ComboBox::from_id_salt("trans_corrente").selected_text(&t.corrente).show_ui(ui, |ui| {
            for c in dados::CORRENTES {
                let r = ui.selectable_label(t.corrente == c.designacao, format!("{} · {}", c.designacao, tr(c.conferencia.rotulo())));
                if r.on_hover_text(c.nota).clicked() {
                    t.corrente = c.designacao.into();
                }
            }
        });
    });
    if let Some(c) = dados::corrente(&t.corrente) {
        let cor = if c.liberada() { crate::theme::st_ok() } else { crate::theme::st_warn() };
        ui.label(RichText::new(format!("{}: p {} · rolo {} · largura {} · {} ({})", tr("Linha"), fmt(c.passo), fmt(c.rolo), if c.largura > 0. { fmt(c.largura) } else { "—".into() }, tr(c.conferencia.rotulo()), c.fontes)).small().color(cor));
        match dados::corrente_para_gerar(&t.corrente) {
            // linha com largura provisória (em `LARGURAS_PROVISORIAS`): gera com ela, como parâmetro, e diz isso
            Ok((_, Some(aviso))) => {
                ui.label(RichText::new(format!("⚠ {}", tr(&aviso))).small().color(crate::theme::st_warn()));
            }
            Ok((_, None)) => {}
            Err(_) => {
                ui.label(RichText::new(tr("Esta linha ainda não tem duas fontes: não gera peça.")).small());
            }
        }
    }
    inteiro(ui, "Dentes do pinhão (z1)", &mut t.z1, 9..=120, "Dentes da roda menor, a motora.");
    inteiro(ui, "Dentes da coroa (z2)", &mut t.z2, 9..=120, "Dentes da roda maior.");
    campo(ui, "Distância entre centros pedida", &mut t.centros, 10.0..=5000., " mm", 1., "A corrente tem elos inteiros: a distância final é corrigida para o número par de elos mais próximo.");
    campo(ui, "Furo do pinhão", &mut t.furo1, 0.0..=200., " mm", 0.5, "Furo do eixo da roda menor.");
    campo(ui, "Furo da coroa", &mut t.furo2, 0.0..=200., " mm", 0.5, "Furo do eixo da roda maior.");
    if let (Some(c), true) = (dados::corrente(&t.corrente), t.z1 > 0 && t.z2 > 0) {
        let (n1, n2) = (t.z1.min(t.z2), t.z1.max(t.z2));
        let elos = dados::elos_da_corrente(c.passo, n1, n2, t.centros);
        match dados::distancia_corrigida_corrente(c.passo, n1, n2, elos) {
            Some(cc) => ui.label(format!("{elos} {} · {} {} mm", tr("elos"), tr("distância corrigida"), fmt(cc))),
            None => ui.label(RichText::new(tr("Corrente curta demais para as duas rodas.")).small()),
        };
    }
}

fn campos_correia(ui: &mut egui::Ui, t: &mut TransUi) {
    ui.horizontal(|ui| {
        ui.label(tr("Correia"));
        egui::ComboBox::from_id_salt("trans_correia").selected_text(&t.correia).show_ui(ui, |ui| {
            for c in dados::CORREIAS {
                if ui.selectable_label(t.correia == c.designacao, format!("{} · {}", c.designacao, tr(c.conferencia.rotulo()))).on_hover_text(c.nota).clicked() {
                    t.correia = c.designacao.into();
                }
            }
        });
    });
    let familia = dados::correia(&t.correia).map(|c| c.familia);
    if let Some(c) = dados::correia(&t.correia) {
        let cor = if c.liberada() { crate::theme::st_ok() } else { crate::theme::st_warn() };
        ui.label(RichText::new(format!("{} ({})", tr(c.conferencia.rotulo()), c.fontes)).small().color(cor));
    }
    if familia == Some(FamiliaCorreia::EmV) {
        campo(ui, "Diâmetro da polia 1", &mut t.dp1, 10.0..=1000., " mm", 1., "Diâmetro externo da polia menor.");
        campo(ui, "Diâmetro da polia 2", &mut t.dp2, 10.0..=1000., " mm", 1., "Diâmetro externo da polia maior.");
    } else {
        inteiro(ui, "Dentes da polia 1", &mut t.z1, 9..=200, "Dentes da polia menor, a motora.");
        inteiro(ui, "Dentes da polia 2", &mut t.z2, 9..=200, "Dentes da polia maior.");
        campo(ui, "Largura da correia", &mut t.largura_correia, 2.0..=100., " mm", 0.5, "Largura da correia dentada e da polia.");
    }
    campo(ui, "Distância entre centros pedida", &mut t.centros, 20.0..=5000., " mm", 1., "A correia dentada tem dentes inteiros: a distância final é corrigida para fechar o comprimento.");
    campo(ui, "Furo da polia 1", &mut t.furo1, 0.0..=200., " mm", 0.5, "Furo do eixo da polia menor.");
    campo(ui, "Furo da polia 2", &mut t.furo2, 0.0..=200., " mm", 0.5, "Furo do eixo da polia maior.");
}

fn usinagens_da_engrenagem(ui: &mut egui::Ui, t: &mut TransUi) {
    let raio_raiz = fenkra_solidos::calculo_engrenagem(&ParamEngrenagem { usinagens: vec![], ..t.pinhao.clone() }).map(|c| c.diametro_raiz / 2.).unwrap_or(30.);
    let g = GeoBase { largura: t.pinhao.largura, raio_raiz, furo: t.pinhao.furo, cubo_proprio: None };
    ui.label(RichText::new(tr("Furo, cubo, rebaixo, alívio, rasgo e parafuso são usinagens sobre a peça padronizada: o vínculo com o catálogo fica gravado na peça. Valem para as duas engrenagens do par.")).weak().small());
    let mut tirar: Option<usize> = None;
    for (i, u) in t.pinhao.usinagens.iter_mut().enumerate() {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(tr(u.nome())).strong());
            match u {
                Usinagem::FuroEixo { diametro } => {
                    ui.add(crate::ui_ext::dv(diametro).range(1.0..=500.).speed(0.5).suffix(" mm").dica(tr("Diâmetro do furo do eixo; vale o maior entre ele e o furo central.")));
                }
                Usinagem::RasgoChaveta => {
                    ui.label(RichText::new(tr("DIN 6885 forma A pelo diâmetro do furo")).weak().small());
                }
                Usinagem::Cubo { diametro, comprimento } => {
                    ui.add(crate::ui_ext::dv(diametro).range(2.0..=500.).speed(0.5).prefix("Ø ").suffix(" mm").dica(tr("Diâmetro do cubo, acima do disco.")));
                    ui.add(crate::ui_ext::dv(comprimento).range(1.0..=500.).speed(0.5).prefix("L ").suffix(" mm").dica(tr("Comprimento do cubo ao longo do eixo.")));
                }
                Usinagem::Rebaixo { diametro, profundidade } => {
                    ui.add(crate::ui_ext::dv(diametro).range(2.0..=500.).speed(0.5).prefix("Ø ").suffix(" mm").dica(tr("Diâmetro do rebaixo na face inferior.")));
                    ui.add(crate::ui_ext::dv(profundidade).range(0.5..=500.).speed(0.5).prefix("p ").suffix(" mm").dica(tr("Profundidade do rebaixo.")));
                }
                Usinagem::FurosAlivio { quantidade, diametro, raio_circulo } => {
                    ui.add(crate::ui_ext::dv(quantidade).range(2..=16).prefix("n ").dica(tr("Quantidade de furos de alívio.")));
                    ui.add(crate::ui_ext::dv(diametro).range(1.0..=200.).speed(0.5).prefix("Ø ").suffix(" mm").dica(tr("Diâmetro de cada furo de alívio.")));
                    ui.add(crate::ui_ext::dv(raio_circulo).range(2.0..=500.).speed(0.5).prefix("R ").suffix(" mm").dica(tr("Raio do círculo dos centros, entre o cubo e a raiz.")));
                }
                Usinagem::ParafusoFixacao { rosca, rotacao } => {
                    ui.add(crate::ui_ext::dv(rosca).range(2.0..=24.).speed(1.).prefix("M ").dica(tr("Rosca métrica do parafuso de fixação; o furo sai na broca de macho (d − passo).")));
                    ui.add(crate::ui_ext::dv(rotacao).range(-180.0..=180.).speed(1.).suffix("°").dica(tr("Posição do furo em volta do eixo, a partir do rasgo (+X).")));
                }
            }
            if ui.small_button("✖").on_hover_text(tr("Remover usinagem")).clicked() {
                tirar = Some(i);
            }
        });
    }
    if let Some(i) = tirar {
        t.pinhao.usinagens.remove(i);
    }
    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt("trans_usinagem_nova").selected_text(tr("+ Usinagem")).show_ui(ui, |ui| {
            for n in Usinagem::NOMES {
                if ui.selectable_label(false, tr(n)).clicked() {
                    t.pinhao.usinagens.push(Usinagem::padrao(n, &g));
                }
            }
        });
        if !t.pinhao.usinagens.is_empty() && ui.button(tr("Voltar à peça padronizada")).on_hover_text(tr("Remove todas as usinagens: a peça volta a ser a do catálogo.")).clicked() {
            t.pinhao.usinagens.clear();
        }
    });
}

fn painel_calculo(ui: &mut egui::Ui, t: &mut TransUi) {
    ui.horizontal(|ui| {
        ui.label(tr("Entrada"));
        ui.add(crate::ui_ext::dv(&mut t.rpm).range(-20000.0..=20000.).speed(10.).suffix(" rpm").dica(tr("Rotação da peça motora: vira o comando de giro da junta na simulação da montagem.")));
        ui.add(crate::ui_ext::dv(&mut t.torque).range(0.0..=100000.).speed(0.1).suffix(" N·m").dica(tr("Torque na entrada.")));
    });
    match calculo(t) {
        Err(e) => {
            ui.label(RichText::new(format!("⚠ {e}")).color(crate::theme::st_err()));
        }
        Ok(r) => {
            egui::Grid::new("trans_calc").num_columns(8).spacing([10., 3.]).show(ui, |ui| {
                for h in ["Estágio", "i", "Voltas", "η", "rpm saída", "T saída N·m", "P saída W", "Perda W"] {
                    ui.label(RichText::new(tr(h)).strong());
                }
                ui.end_row();
                for (k, e) in r.estagios.iter().enumerate() {
                    ui.label(format!("{} · {}", k + 1, tr(e.tipo.rotulo())));
                    ui.label(fmt(e.razao));
                    ui.label(fmt(e.voltas));
                    ui.label(fmt(e.rendimento));
                    ui.label(fmt(e.rpm_saida));
                    ui.label(fmt(e.torque_saida));
                    ui.label(fmt(e.potencia_saida));
                    ui.label(fmt(e.perda));
                    ui.end_row();
                }
                if r.estagios.len() > 1 {
                    ui.label(RichText::new(tr("Total")).strong());
                    ui.label(fmt(r.razao_total));
                    ui.label(fmt(r.voltas_total));
                    ui.label(fmt(r.rendimento_total));
                    ui.label(fmt(r.rpm_saida));
                    ui.label(fmt(r.torque_saida));
                    ui.label(fmt(r.potencia_saida));
                    ui.label(fmt(r.perda_total));
                    ui.end_row();
                }
            });
            ui.label(format!("{}: {} rad/s · {}: {} W", tr("Velocidade angular na saída"), fmt(r.omega_saida), tr("Potência de entrada"), fmt(r.potencia_entrada)));
            for e in &r.estagios {
                for a in &e.avisos {
                    ui.label(RichText::new(format!("⚠ {a}")).small());
                }
            }
        }
    }
    // estágios extras da cadeia
    let mut tirar: Option<usize> = None;
    for (i, e) in t.extra.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            ui.label(format!("{} {}", tr("Estágio"), i + 2));
            egui::ComboBox::from_id_salt(("trans_extra", i)).selected_text(tr(e.tipo.rotulo())).show_ui(ui, |ui| {
                for tp in TipoEstagio::ALL {
                    ui.selectable_value(&mut e.tipo, tp, tr(tp.rotulo()));
                }
            });
            ui.add(crate::ui_ext::dv(&mut e.n1).range(1.0..=2000.).speed(1.).prefix("n1 ").dica(tr("Dentes (ou diâmetro) da peça motora; no sem-fim, as entradas.")));
            ui.add(crate::ui_ext::dv(&mut e.n2).range(1.0..=2000.).speed(1.).prefix("n2 ").dica(tr("Dentes (ou diâmetro) da peça movida.")));
            let mut manual = e.rendimento.is_some();
            if ui.checkbox(&mut manual, "η").on_hover_text(tr("Usar um rendimento seu em vez do típico (valor típico, confira com o fabricante).")).changed() {
                e.rendimento = manual.then(|| e.tipo.rendimento_tipico());
            }
            if let Some(r) = e.rendimento.as_mut() {
                ui.add(crate::ui_ext::dv(r).range(0.05..=1.).speed(0.005).max_decimals(3));
            }
            if ui.small_button("✖").on_hover_text(tr("Remover estágio")).clicked() {
                tirar = Some(i);
            }
        });
    }
    if let Some(i) = tirar {
        t.extra.remove(i);
    }
    if ui.button(tr("+ Estágio")).on_hover_text(tr("Acrescenta outro par em série: a saída de um é a entrada do seguinte, e o total soma razões, rendimentos e perdas.")).clicked() {
        t.extra.push(Estagio::novo(TipoEstagio::Externa, 20., 40.));
    }
    ui.label(RichText::new(tr(REFERENCIA)).weak().small());
}

fn painel_adaptacao(ui: &mut egui::Ui, t: &mut TransUi) {
    ui.label(RichText::new(tr("Fixe o que limita o projeto e peça propostas: o comando diz o que cabe.")).weak().small());
    ui.horizontal_wrapped(|ui| {
        ui.checkbox(&mut t.usa_distancia, tr("Distância")).on_hover_text(tr("Distância entre os dois eixos (por exemplo, entre dois pontos do projeto)."));
        if t.usa_distancia {
            let mut a = t.restr.distancia.unwrap_or(90.);
            ui.add(crate::ui_ext::dv(&mut a).range(1.0..=5000.).speed(0.5).suffix(" mm"));
            t.restr.distancia = Some(a);
            let texto = if t.medindo { format!("{} {}/2", tr("clique no 3D:"), t.marcas.len() % 2 + 1) } else { tr("Medir no 3D").to_string() };
            if ui.selectable_label(t.medindo, texto).on_hover_text(tr("Clique em dois pontos da montagem: vértice, aresta ou furo. A medida entra na distância, e o furo vale pelo eixo dele.")).clicked() {
                t.medindo = !t.medindo;
                t.marcas.clear();
            }
        }
        ui.checkbox(&mut t.usa_razao, tr("Razão")).on_hover_text(tr("Razão z2 ÷ z1 desejada; 1 ou mais."));
        if t.usa_razao {
            let mut i = t.restr.razao.unwrap_or(2.);
            ui.add(crate::ui_ext::dv(&mut i).range(1.0..=20.).speed(0.02).max_decimals(3));
            t.restr.razao = Some(i);
        }
    });
    ui.horizontal_wrapped(|ui| {
        ui.checkbox(&mut t.usa_modulo, tr("Módulo")).on_hover_text(tr("Fixa o módulo das engrenagens."));
        if t.usa_modulo {
            let mut m = t.restr.modulo.unwrap_or(2.);
            ui.add(crate::ui_ext::dv(&mut m).range(0.1..=50.).speed(0.05).suffix(" mm"));
            t.restr.modulo = Some(m);
        }
        ui.checkbox(&mut t.usa_espaco, tr("Espaço")).on_hover_text(tr("Diâmetro externo máximo de qualquer roda: o que não cabe é descartado."));
        if t.usa_espaco {
            ui.add(crate::ui_ext::dv(&mut t.espaco).range(10.0..=2000.).speed(1.).suffix(" mm"));
        }
    });
    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt("trans_familia").selected_text(tr(t.familia.rotulo())).show_ui(ui, |ui| {
            for f in Familia::ALL {
                ui.selectable_value(&mut t.familia, f, tr(f.rotulo()));
            }
        });
        if ui.button(tr("Propor")).on_hover_text(tr("Calcula o que cabe nos limites, da melhor proposta para a pior.")).clicked() {
            let r = t.restricoes();
            t.propostas = propor(t.familia, &r, 6);
            if t.propostas.is_empty() {
                t.msg = vec![tr("Nada cabe nesses limites: afrouxe a distância, a razão ou o espaço.").to_string()];
            } else {
                t.msg.clear();
            }
        }
    });
    let mut escolhida: Option<Proposta> = None;
    for p in &t.propostas {
        let erro = if p.erro_distancia.abs() < 1e-9 && p.erro_razao.abs() < 1e-9 { tr("exato").to_string() } else { format!("Δa {} · Δi {}", fmt(p.erro_distancia), fmt(p.erro_razao)) };
        let texto = format!("{} · a {} · i {} · Ø {} · {erro}", p.descricao(), fmt(p.distancia), fmt(p.razao), fmt(p.diametro_maximo));
        if ui.button(texto).on_hover_text(tr("Aplica esta proposta aos campos do elemento.")).clicked() {
            escolhida = Some(p.clone());
        }
    }
    if let Some(p) = escolhida {
        t.aplicar(&p);
    }
}

/// Total da cadeia na janela de simulação: o que os elementos da montagem somam, com o botão que leva
/// a saída de cada um para o comando do seguinte. Com a simulação em `t`, mostra o giro da saída.
pub fn cadeia_na_simulacao(ui: &mut egui::Ui, st: &mut crate::asm_ui::AsmState) {
    let Some(cadeia) = fenkra_montagem::transmissao::cadeia_da_montagem(&st.asm) else { return };
    crate::editor::section(ui, "Cadeia de transmissão");
    let r = match cadeia {
        Ok(r) => r,
        Err(e) => {
            ui.label(RichText::new(tr(&e)).color(crate::theme::st_err()));
            return;
        }
    };
    let entrada = r.estagios.first().map_or(0., |e| e.rpm_entrada);
    egui::Grid::new("sim_cadeia").num_columns(5).striped(true).show(ui, |ui| {
        for t in ["Estágio", "Entrada", "Saída", "Torque", "Rendimento"] {
            ui.label(RichText::new(tr(t)).small().strong());
        }
        ui.end_row();
        for (k, (e, reg)) in r.estagios.iter().zip(&st.asm.transmissoes).enumerate() {
            ui.label(RichText::new(format!("{} · {}", k + 1, tr(&reg.nome))).small());
            ui.label(RichText::new(format!("{} rpm", fmt(e.rpm_entrada))).small());
            ui.label(RichText::new(format!("{} rpm", fmt(e.rpm_saida))).small());
            ui.label(RichText::new(format!("{} N·m", fmt(e.torque_saida))).small());
            ui.label(RichText::new(format!("{} %", fmt(e.rendimento * 100.))).small());
            ui.end_row();
        }
        ui.label(RichText::new(tr("Total")).small().strong());
        ui.label(RichText::new(format!("{} rpm", fmt(entrada))).small().strong());
        ui.label(RichText::new(format!("{} rpm", fmt(r.rpm_saida))).small().strong());
        ui.label(RichText::new(format!("{} N·m", fmt(r.torque_saida))).small().strong());
        ui.label(RichText::new(format!("{} %", fmt(r.rendimento_total * 100.))).small().strong());
        ui.end_row();
    });
    let giro = r.rpm_saida * 6. * st.sim_t;
    ui.label(RichText::new(format!("{} {} · {} {} W · {} {}°", tr("relação total"), fmt(r.razao_total), tr("perda"), fmt(r.perda_total), tr("giro da saída em t:"), fmt(giro))).small());
    let seguidos = st.asm.transmissoes.iter().zip(&r.estagios).all(|(g, e)| (g.rpm - e.rpm_entrada).abs() < 1e-6);
    if st.asm.transmissoes.len() > 1 {
        if seguidos {
            ui.label(RichText::new(tr("✔ cada estágio recebe o que o anterior entrega")).small().color(crate::theme::st_ok()));
        } else if ui.button(tr("Levar a cadeia para os comandos")).on_hover_text(tr("A junta motora de cada estágio passa a girar com a saída do anterior.")).clicked() {
            st.push_undo();
            if let Err(e) = fenkra_montagem::transmissao::aplicar_cadeia(&mut st.asm) {
                st.aviso = Some((e, true));
            }
            st.sim_frames.clear();
            st.modified = true;
        }
    }
}

/// Janela do comando.
pub fn janela(app: &mut App, ctx: &egui::Context) {
    let Some(st) = app.asm.as_mut() else { return };
    let Some(mut t) = st.trans_open.clone() else { return };
    let mut open = true;
    let (mut fechar, mut inserir, mut aplicar_edicao, mut classico) = (false, false, false, false);
    let mut editar: Option<Id> = None;
    let registros: Vec<(Id, String)> = st.asm.transmissoes.iter().map(|r| (r.id, r.nome.clone())).collect();
    let layout = montar_layout(&t.elemento());
    egui::Window::new(tr("⚙ Gerar elemento de transmissão")).id(egui::Id::new("janela_transmissao")).open(&mut open).default_width(500.).default_pos([16., 130.]).max_height(ctx.content_rect().height() - 170.).vscroll(true).show(ctx, |ui| {
        if crate::app::done_bar(ui) {
            fechar = true;
        }
        ui.label(RichText::new(tr("Um comando só para engrenagens, cônicas, diferencial, sem-fim, corrente e correia: gera o conjunto, insere eixos, juntas e a relação certa, e já anima na simulação da montagem.")).weak().small());
        if !registros.is_empty() {
            crate::editor::section(ui, "Elementos já gerados");
            for (id, nome) in &registros {
                ui.horizontal(|ui| {
                    let ativo = t.editando == Some(*id);
                    ui.label(if ativo { RichText::new(format!("● {nome}")).strong() } else { RichText::new(nome.as_str()) });
                    if ui.small_button(tr("Editar")).on_hover_text(tr("Recarrega os parâmetros deste elemento: ao aplicar, ele é gerado de novo no mesmo lugar.")).clicked() {
                        editar = Some(*id);
                    }
                });
            }
        }
        crate::editor::section(ui, "Tipo");
        ui.horizontal_wrapped(|ui| {
            for tp in Tipo::ALL {
                if ui.selectable_value(&mut t.tipo, tp, tr(tp.rotulo())).clicked() {
                    // a adaptação de projeto acompanha o tipo escolhido
                    t.familia = match tp {
                        Tipo::Helicoidais | Tipo::EspinhaDePeixe => Familia::Helicoidais,
                        Tipo::Corrente => Familia::Corrente,
                        Tipo::Correia => Familia::CorreiaDentada,
                        _ => Familia::Retas,
                    };
                }
            }
        });
        crate::editor::section(ui, "Medidas");
        match t.tipo {
            Tipo::Retas | Tipo::Helicoidais | Tipo::EspinhaDePeixe | Tipo::Interna => campos_engrenagens(ui, &mut t),
            Tipo::Conicas => campos_conica(ui, &mut t.conica, "Dentes da peça conjugada"),
            Tipo::Diferencial => {
                let mut p = t.diferencial.clone();
                campos_conica(ui, &mut p, "Dentes do satélite");
                t.diferencial = p;
                ui.label(RichText::new(tr("Duas laterais (dentes desta peça) e dois satélites; as laterais precisam de número par de dentes.")).weak().small());
                ui.checkbox(&mut t.caixa_solta, tr("Caixa solta")).on_hover_text(tr("A caixa gira no rpm de entrada e leva os satélites; presa, ela fica parada e a lateral 2 gira ao contrário."));
                if t.caixa_solta {
                    campo(ui, "Giro das laterais na caixa", &mut t.giro_relativo, -5000.0..=5000., " rpm", 1., "Zero é a reta: as duas laterais giram com a caixa. Fora disso, uma adianta e a outra atrasa o mesmo tanto.");
                }
            }
            Tipo::SemFim => campos_sem_fim(ui, &mut t.semfim),
            Tipo::Corrente => campos_corrente(ui, &mut t),
            Tipo::Correia => campos_correia(ui, &mut t),
        }
        if t.tipo.engrenagem() && t.tipo != Tipo::Interna {
            crate::editor::section(ui, "Usinagens sobre a peça padronizada");
            usinagens_da_engrenagem(ui, &mut t);
        }
        crate::editor::section(ui, "Adaptação de projeto");
        painel_adaptacao(ui, &mut t);
        crate::editor::section(ui, "Cálculo ao vivo");
        painel_calculo(ui, &mut t);
        match &layout {
            Err(e) => {
                ui.label(RichText::new(format!("⚠ {e}")).color(crate::theme::st_err()));
            }
            Ok(l) => {
                ui.label(format!("{}: {} · {}: {}", tr("Razão de giro"), fmt(l.razao), tr("Peças"), l.pecas.len()));
                if l.distancia_centros > 0. {
                    ui.label(format!("{}: {} mm", tr("Distância entre centros"), fmt(l.distancia_centros)));
                }
                for a in &l.avisos {
                    ui.label(RichText::new(format!("⚠ {a}")).small());
                }
            }
        }
        ui.add_space(4.);
        ui.horizontal_wrapped(|ui| {
            let ok = layout.is_ok();
            if let Some(id) = t.editando {
                let nome = registros.iter().find(|(i, _)| *i == id).map(|(_, n)| n.clone()).unwrap_or_default();
                if ui.add_enabled(ok, egui::Button::new(RichText::new(format!("{} {nome}", tr("Aplicar edição em"))).strong()).min_size(egui::vec2(160., 28.))).on_hover_text(tr("Gera o elemento de novo com os parâmetros acima, no mesmo lugar, e mantém o registro.")).clicked() {
                    aplicar_edicao = true;
                }
                if ui.button(tr("Cancelar edição")).clicked() {
                    t.editando = None;
                }
            }
            if ui.add_enabled(ok, egui::Button::new(RichText::new(tr("Inserir transmissão")).strong()).min_size(egui::vec2(160., 28.))).on_hover_text(tr("Grava as peças em “transmissao/” ao lado da montagem e insere eixos, juntas de revolução, o acoplamento com a relação certa e o comando de giro. Rodar de novo insere outro conjunto.")).clicked() {
                inserir = true;
            }
            if ui.button(tr("Eixo, chaveta e par avulso...")).on_hover_text(tr("Abre o gerador clássico: eixo escalonado, chaveta e salvar engrenagens avulsas.")).clicked() {
                classico = true;
            }
        });
        for m in &t.msg {
            ui.label(RichText::new(format!("⚠ {m}")).small());
        }
    });
    if !open || fechar {
        st.trans_open = None;
        st.fantasma_novo = true;
        return;
    }
    if let Some(id) = editar {
        if let Some(r) = st.asm.transmissoes.iter().find(|r| r.id == id).cloned() {
            t.carregar(&r.elemento);
            t.rpm = r.rpm;
            t.torque = r.torque;
            t.editando = Some(id);
        }
    }
    if atualizar_malhas(&mut t, ctx) {
        st.fantasma_novo = true;
    }
    let mut aviso: Option<(String, bool)> = None;
    if inserir || aplicar_edicao {
        let dir = st.dir();
        if dir.is_none() {
            aviso = Some((tr("Salve a montagem antes: as peças da transmissão são gravadas na pasta dela").to_string(), true));
        } else {
            let modificada = st.modified;
            st.push_undo();
            let e = t.elemento();
            let r = if aplicar_edicao { editar_transmissao(&mut st.asm, dir.as_deref(), t.editando.unwrap_or(Id(0)), &e, t.rpm, t.torque) } else { inserir_transmissao(&mut st.asm, dir.as_deref(), &e, t.rpm, t.torque, &Placement::default()) };
            match r {
                Ok(rel) => {
                    st.solve();
                    aviso = Some((format!("{} · {}", tr(if aplicar_edicao { "Elemento de transmissão editado" } else { "Elemento de transmissão inserido" }), fmt(rel.razao)), false));
                    t.msg = rel.avisos;
                    if inserir {
                        t.editando = Some(rel.registro);
                    }
                }
                Err(err) => {
                    if let Some(antes) = st.undo.pop() {
                        st.asm = antes;
                    }
                    st.modified = modificada;
                    st.dirty = true;
                    t.msg = vec![err.clone()];
                    aviso = Some((err, true));
                }
            }
        }
    }
    if classico && st.gears_open.is_none() {
        st.gears_open = Some(crate::asm_ui::GearsUi::default());
    }
    st.trans_open = Some(t);
    if let Some((msg, erro)) = aviso {
        app.toast(msg, erro);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn os_campos_viram_o_elemento_e_voltam() {
        let mut t = TransUi::default();
        for tp in Tipo::ALL {
            t.tipo = tp;
            let e = t.elemento();
            let mut u = TransUi::default();
            u.carregar(&e);
            assert_eq!(u.tipo, tp, "{tp:?}");
            assert_eq!(u.elemento(), e, "{tp:?}");
        }
    }

    #[test]
    fn calculo_ao_vivo_acompanha_os_campos_e_soma_os_estagios() {
        let mut t = TransUi::default();
        t.rpm = 1500.;
        t.torque = 10.;
        // 20/40 reta: i = 2, η = 0,98
        let r = calculo(&t).unwrap();
        assert!((r.razao_total - 2.).abs() < 1e-12 && (r.rpm_saida + 750.).abs() < 1e-9);
        // mudar a coroa muda o cálculo
        t.dentes_coroa = 60;
        assert!((calculo(&t).unwrap().razao_total - 3.).abs() < 1e-12);
        // um segundo estágio multiplica a razão e o rendimento
        t.extra.push(Estagio::novo(TipoEstagio::Corrente, 15., 45.));
        let c = calculo(&t).unwrap();
        assert!((c.razao_total - 9.).abs() < 1e-12 && (c.rendimento_total - 0.98 * 0.97).abs() < 1e-12);
        // o elemento de corrente tem o estágio de corrente
        t.tipo = Tipo::Corrente;
        t.extra.clear();
        assert!((calculo(&t).unwrap().razao_total - 2.).abs() < 1e-12);
    }

    #[test]
    fn aplicar_uma_proposta_escreve_nos_campos_e_a_previa_confere() {
        let mut t = TransUi::default();
        t.restr = Restricoes { distancia: Some(90.), razao: Some(2.), angulo_pressao: 20., ..Default::default() };
        let p = propor(Familia::Retas, &t.restricoes(), 1).remove(0);
        t.aplicar(&p);
        assert_eq!((t.pinhao.dentes, t.dentes_coroa), (p.n1, p.n2));
        let l = montar_layout(&t.elemento()).unwrap();
        assert!((l.distancia_centros - 90.).abs() < 1e-9 && (l.razao + 0.5).abs() < 1e-9);
        // corrente: a proposta traz a distância corrigida, e o layout da corrente a confirma
        t.familia = Familia::Corrente;
        t.restr.distancia = Some(400.);
        let p = propor(Familia::Corrente, &t.restricoes(), 1).remove(0);
        t.aplicar(&p);
        let l = montar_layout(&t.elemento()).unwrap();
        assert!((l.distancia_centros - p.distancia).abs() < 1e-6, "{} {}", l.distancia_centros, p.distancia);
    }
}
