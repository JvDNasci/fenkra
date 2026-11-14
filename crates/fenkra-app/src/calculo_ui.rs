// SPDX-License-Identifier: LGPL-2.1-or-later
//! Calculadoras de projeto (B110): painel com parafuso, ajuste forçado, mola e viga.
//!
//! As contas são de `fenkra_solidos::calculo_projeto` (fórmulas fechadas, com a referência de cada
//! uma no módulo). O painel mostra as entradas, os resultados com o fator de segurança pintado pelo
//! tema (dentro ou fora do exigido), os avisos, e leva sempre o rótulo de cálculo preliminar. No
//! parafuso há uma ação sobre o componente: "Ajustar o parafuso escolhido" põe o tamanho calculado na
//! pilha de fixadores do componente selecionado na árvore, com um desfazer. Nas outras abas o botão
//! "Inserir" põe na montagem a peça calculada (mola, barra da viga, eixo e cubo do ajuste), gravada em
//! `calculo/` ao lado da montagem, com as medidas como parâmetros e o resumo nas propriedades.

use crate::app::{fmt, App};
use crate::asm_ui::AsmState;
use crate::i18n::tr;
use crate::paineis::{self, Encaixe, Painel};
use eframe::egui::{self, Color32, Rect, RichText};
use fenkra_montagem::pilhas;
use fenkra_solidos::biblioteca;
use fenkra_solidos::calculo_projeto::*;
use fenkra_solidos::hardware::METRIC;

pub const PAINEL: &str = "calculo_projeto";

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Aba {
    #[default]
    Parafuso,
    Ajuste,
    Mola,
    Viga,
    /// B111
    Rolamento,
    Engrenagem,
    Estriado,
}

impl Aba {
    pub const TODAS: [Aba; 7] = [Aba::Parafuso, Aba::Ajuste, Aba::Mola, Aba::Viga, Aba::Rolamento, Aba::Engrenagem, Aba::Estriado];

    fn rotulo(self) -> &'static str {
        match self {
            Aba::Parafuso => "Parafuso",
            Aba::Ajuste => "Ajuste forçado",
            Aba::Mola => "Mola",
            Aba::Viga => "Viga",
            Aba::Rolamento => "Rolamento",
            Aba::Engrenagem => "Engrenagem",
            Aba::Estriado => "Estriado",
        }
    }
}

/// Seções oferecidas na viga, na ordem do seletor.
const SECOES: [&str; 4] = ["Retangular", "Circular", "Tubo", "Genérica (I e c)"];
const CARGAS: [(&str, Carga); 2] = [("Pontual", Carga::Pontual), ("Distribuída", Carga::Distribuida)];
const APOIOS: [(&str, Apoio); 2] = [("Biapoiada", Apoio::Biapoiada), ("Engastada", Apoio::Engastada)];
const AJUSTES: [&str; 6] = ["H7/k6", "H7/n6", "H7/p6", "H7/r6", "H7/s6", "H7/u6"];

/// Peça que o painel insere na montagem.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Inserir {
    Mola,
    Viga,
    Ajuste,
}

/// Grupo da árvore e pasta das peças inseridas pelas calculadoras.
const GRUPO: &str = "Cálculo de projeto";
const PASTA: &str = "calculo";

#[derive(Clone, Debug)]
pub struct Estado {
    pub aba: Aba,
    pub parafuso: EntradaParafuso,
    pub ajuste: EntradaAjuste,
    pub mola: EntradaMola,
    pub viga: EntradaViga,
    /// seção da viga e as suas duas medidas
    pub secao: usize,
    pub dims: [f64; 2],
    /// rolamento, engrenagem e estriado (B111)
    pub rot: crate::calculo_rotativos_ui::EstadoRotativos,
}

impl Default for Estado {
    fn default() -> Self {
        Estado { aba: Aba::Parafuso, parafuso: Default::default(), ajuste: Default::default(), mola: Default::default(), viga: Default::default(), secao: 0, dims: [20., 40.], rot: Default::default() }
    }
}

impl Estado {
    fn secao_atual(&self) -> Secao {
        let [a, b] = self.dims;
        match self.secao {
            0 => Secao::Retangular { b: a, h: b },
            1 => Secao::Circular { d: a },
            2 => Secao::Tubo { d_ext: a, d_int: b },
            _ => Secao::Generica { i: a, c: b },
        }
    }
}

pub fn abrir(st: &mut AsmState) {
    st.calculo = if st.calculo.is_some() { None } else { Some(Estado::default()) };
}

/// Linha de entrada: rótulo com a dica, e o campo numérico.
pub(crate) fn campo(ui: &mut egui::Ui, rotulo: &str, dica: &str, v: &mut f64, faixa: std::ops::RangeInclusive<f64>, sufixo: &str) {
    ui.label(tr(rotulo)).on_hover_text(tr(dica));
    ui.add(crate::ui_ext::dv(v).range(faixa).speed(0.5).suffix(sufixo).dica(tr(dica)));
    ui.end_row();
}

/// Linha de resultado: rótulo com a explicação e o valor, na cor dada.
pub(crate) fn saida(ui: &mut egui::Ui, rotulo: &str, dica: &str, valor: String, cor: Option<Color32>) {
    ui.label(tr(rotulo)).on_hover_text(tr(dica));
    let t = RichText::new(valor).strong();
    ui.label(match cor {
        Some(c) => t.color(c),
        None => t,
    });
    ui.end_row();
}

pub(crate) fn cor_fs(fs: f64, exigido: f64) -> Color32 {
    if fs >= exigido {
        crate::theme::st_ok()
    } else {
        crate::theme::st_err()
    }
}

pub(crate) fn avisos(ui: &mut egui::Ui, lista: &[String]) {
    for a in lista {
        ui.label(RichText::new(format!("⚠ {}", tr(a))).small().color(crate::theme::st_warn()));
    }
}

fn combo<T: Copy + PartialEq>(ui: &mut egui::Ui, id: &str, atual: &mut T, itens: &[(&str, T)]) {
    let nome = itens.iter().find(|(_, v)| v == atual).map(|(n, _)| *n).unwrap_or("");
    egui::ComboBox::from_id_salt(id).selected_text(tr(nome)).show_ui(ui, |ui| {
        for (n, v) in itens {
            if ui.selectable_label(v == atual, tr(n)).clicked() {
                *atual = *v;
            }
        }
    });
}

fn aba_parafuso(ui: &mut egui::Ui, e: &mut Estado, aplicar: &mut Option<String>, tem_pilha: bool) {
    let p = &mut e.parafuso;
    egui::Grid::new("calc_parafuso_in").num_columns(2).show(ui, |ui| {
        ui.label(tr("Tamanho")).on_hover_text(tr("Diâmetro nominal da rosca métrica de passo grosso."));
        egui::ComboBox::from_id_salt("calc_parafuso_tam").selected_text(&p.tamanho).show_ui(ui, |ui| {
            for m in METRIC {
                if ui.selectable_label(p.tamanho == m.name, m.name).clicked() {
                    p.tamanho = m.name.to_string();
                }
            }
        });
        ui.end_row();
        ui.label(tr("Classe")).on_hover_text(tr("Classe de resistência do parafuso de aço (ISO 898-1): define a tensão de prova Sp."));
        egui::ComboBox::from_id_salt("calc_parafuso_cls").selected_text(&p.classe).show_ui(ui, |ui| {
            for c in CLASSES_PARAFUSO {
                if ui.selectable_label(p.classe == c.nome, c.nome).clicked() {
                    p.classe = c.nome.to_string();
                }
            }
        });
        ui.end_row();
        campo(ui, "Tração por parafuso", "Carga de tração que cada parafuso suporta, em newtons.", &mut p.tracao, 0.0..=1e7, " N");
        campo(ui, "Corte por parafuso", "Carga cortante que cada parafuso suporta, em newtons. Zero se o parafuso só traciona.", &mut p.corte, 0.0..=1e7, " N");
        let mut planos = p.planos as f64;
        campo(ui, "Planos de corte", "Quantos planos de corte o parafuso atravessa: 1 numa junta simples, 2 num pino em forquilha.", &mut planos, 1.0..=2., "");
        p.planos = planos.round().clamp(1., 2.) as u32;
        campo(ui, "Fator de segurança exigido", "Menor fator de segurança aceito na tração e no corte.", &mut p.fs_exigido, 1.0..=10., "");
        campo(ui, "Pré-carga", "Fração de Sp·As aplicada no aperto: 0,75 para junta desmontável, 0,9 para permanente.", &mut p.fracao_pre_carga, 0.1..=1., "");
        campo(ui, "Fator de porca K", "Relaciona o torque à pré-carga, T = K·Fi·d. Cerca de 0,2 para aço seco; menor com lubrificante.", &mut p.fator_porca, 0.05..=0.5, "");
    });
    match calcular_parafuso(p) {
        Err(m) => {
            ui.label(RichText::new(format!("✖ {}", tr(&m))).color(crate::theme::st_err()));
        }
        Ok(r) => {
            ui.separator();
            let fs_min = r.fs_tracao.into_iter().chain(r.fs_corte).fold(f64::MAX, f64::min);
            egui::Grid::new("calc_parafuso_out").num_columns(2).show(ui, |ui| {
                saida(ui, "Área resistente As", "Área da seção que resiste à tração, pelos diâmetros primitivo e menor da rosca.", format!("{:.1} mm²", r.area), None);
                saida(ui, "Tensão de prova Sp", "Tensão que a classe suporta sem deformação permanente (ISO 898-1).", format!("{} MPa", fmt(r.tensao_prova)), None);
                if let Some(f) = r.fs_tracao {
                    saida(ui, "FS à tração", "Tensão de prova dividida pela tensão de tração, F/As.", format!("{f:.2}  ({:.0} MPa)", r.tensao_tracao), Some(cor_fs(f, p.fs_exigido)));
                }
                if let Some(f) = r.fs_corte {
                    saida(ui, "FS ao corte", "0,577·Sp dividido pela tensão de corte (critério de von Mises, com Sp no lugar do escoamento).", format!("{f:.2}  ({:.0} MPa)", r.tensao_corte), Some(cor_fs(f, p.fs_exigido)));
                }
                saida(ui, "Pré-carga", "Força de aperto da junta: fração de Sp·As.", format!("{:.0} N", r.pre_carga), None);
                saida(ui, "Torque de aperto", "T = K·Fi·d: o torque que produz a pré-carga.", format!("{:.1} N·m", r.torque), None);
                if let Some(rec) = &r.recomendado {
                    saida(ui, "Menor tamanho que atende", "Primeiro tamanho da tabela que cumpre o fator exigido nas verificações calculadas.", rec.clone(), Some(cor_fs(fs_min, p.fs_exigido)));
                }
            });
            avisos(ui, &r.avisos);
            ui.horizontal(|ui| {
                if let Some(rec) = r.recomendado.clone() {
                    if ui.button(tr("Usar o tamanho recomendado")).on_hover_text(tr("Põe o menor tamanho que atende no campo Tamanho.")).clicked() {
                        p.tamanho = rec;
                    }
                }
                let b = ui.add_enabled(tem_pilha, egui::Button::new(tr("Ajustar o parafuso escolhido")));
                if b.on_hover_text(tr("Põe o tamanho calculado na pilha de fixadores do componente escolhido na árvore, com um desfazer. Precisa de um parafuso inserido pelo comando Fixadores.")).clicked() {
                    *aplicar = Some(p.tamanho.clone());
                }
            });
        }
    }
}

pub(crate) fn material(ui: &mut egui::Ui, id: &str, m: &mut Material) {
    egui::ComboBox::from_id_salt(id).selected_text(tr(m.nome)).show_ui(ui, |ui| {
        for x in MATERIAIS {
            if ui.selectable_label(x.nome == m.nome, tr(x.nome)).clicked() {
                *m = *x;
            }
        }
    });
}

fn botao_inserir(ui: &mut egui::Ui, rotulo: &str, dica: &str, qual: Inserir, inserir: &mut Option<Inserir>) {
    if ui.button(tr(rotulo)).on_hover_text(tr(dica)).clicked() {
        *inserir = Some(qual);
    }
}

fn aba_ajuste(ui: &mut egui::Ui, e: &mut Estado, inserir: &mut Option<Inserir>) {
    let a = &mut e.ajuste;
    egui::Grid::new("calc_ajuste_in").num_columns(2).show(ui, |ui| {
        campo(ui, "Diâmetro da união", "Diâmetro nominal comum ao eixo e ao furo do cubo.", &mut a.d, 1.0..=500., " mm");
        campo(ui, "Diâmetro externo do cubo", "Maior diâmetro do cubo na região da união.", &mut a.d_cubo, 1.0..=1000., " mm");
        campo(ui, "Furo do eixo", "Diâmetro do furo central do eixo vazado. Zero para eixo maciço.", &mut a.d_furo_eixo, 0.0..=500., " mm");
        campo(ui, "Comprimento da união", "Comprimento em que o eixo e o cubo se tocam.", &mut a.comprimento, 1.0..=1000., " mm");
        ui.label(tr("Ajuste ISO 286")).on_hover_text(tr("Furo/eixo da ISO 286, como H7/p6. A interferência mínima e a máxima saem da tabela de desvios."));
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut a.ajuste).desired_width(70.));
            egui::ComboBox::from_id_salt("calc_ajuste_pre").selected_text(tr("usuais")).show_ui(ui, |ui| {
                for s in AJUSTES {
                    if ui.selectable_label(a.ajuste == s, s).clicked() {
                        a.ajuste = s.to_string();
                    }
                }
            });
        });
        ui.end_row();
        campo(ui, "Atrito μ", "Coeficiente de atrito da união: 0,12 a 0,15 para aço seco.", &mut a.mu, 0.02..=0.6, "");
        campo(ui, "Torque exigido", "Torque que a união precisa transmitir. Zero para só ver a capacidade.", &mut a.torque, 0.0..=1e6, " N·m");
        campo(ui, "Fator de segurança exigido", "Menor fator aceito nas tensões e no torque.", &mut a.fs_exigido, 1.0..=10., "");
        ui.label(tr("Material do cubo")).on_hover_text(tr("Valores típicos de módulo, Poisson e escoamento. Confira com o material real."));
        material(ui, "calc_ajuste_mc", &mut a.cubo);
        ui.end_row();
        ui.label(tr("Material do eixo")).on_hover_text(tr("Valores típicos de módulo, Poisson e escoamento. Confira com o material real."));
        material(ui, "calc_ajuste_me", &mut a.eixo);
        ui.end_row();
    });
    match calcular_ajuste(a) {
        Err(m) => {
            ui.label(RichText::new(format!("✖ {}", tr(&m))).color(crate::theme::st_err()));
        }
        Ok(r) => {
            ui.separator();
            egui::Grid::new("calc_ajuste_out").num_columns(2).show(ui, |ui| {
                saida(ui, "Interferência", "Diferença entre o eixo e o furo, diametral, do ajuste escolhido: mínima e máxima.", format!("{:.0} … {:.0} µm", r.interf_min_um, r.interf_max_um), None);
                saida(ui, "Pressão de contato", "Pressão na superfície da união pelos cilindros de parede espessa de Lamé, na interferência mínima e na máxima.", format!("{:.1} … {:.1} MPa", r.p_min, r.p_max), None);
                saida(ui, "Torque transmissível", "T = μ·p·π·d²·L/2 com a pressão mínima.", format!("{:.0} N·m", r.torque_min), r.fs_torque.map(|f| cor_fs(f, a.fs_exigido)));
                saida(ui, "Força axial", "F = μ·p·π·d·L com a pressão mínima: força para escorregar a união no sentido do eixo.", format!("{:.0} N", r.forca_axial_min), None);
                saida(ui, "FS do cubo", "Escoamento do cubo dividido pela tensão equivalente no furo, com a interferência máxima.", format!("{:.2}  ({:.0} MPa)", r.fs_cubo, r.tensao_cubo), Some(cor_fs(r.fs_cubo, a.fs_exigido)));
                saida(ui, "FS do eixo", "Escoamento do eixo dividido pela tensão equivalente na superfície, com a interferência máxima.", format!("{:.2}  ({:.0} MPa)", r.fs_eixo, r.tensao_eixo), Some(cor_fs(r.fs_eixo, a.fs_exigido)));
                if let Some(f) = r.fs_torque {
                    saida(ui, "FS do torque", "Capacidade de torque com a interferência mínima dividida pelo torque exigido.", format!("{f:.2}"), Some(cor_fs(f, a.fs_exigido)));
                }
            });
            avisos(ui, &r.avisos);
            botao_inserir(ui, "Inserir eixo e cubo", "Põe na montagem o eixo e o cubo da união, nas medidas nominais, com o ajuste ISO 286 e a interferência nas propriedades das peças.", Inserir::Ajuste, inserir);
        }
    }
}

fn aba_mola(ui: &mut egui::Ui, e: &mut Estado, inserir: &mut Option<Inserir>) {
    let m = &mut e.mola;
    egui::Grid::new("calc_mola_in").num_columns(2).show(ui, |ui| {
        campo(ui, "Diâmetro do fio d", "Diâmetro do arame da mola.", &mut m.d_fio, 0.1..=50., " mm");
        campo(ui, "Diâmetro médio D", "Diâmetro da espira medido no centro do arame.", &mut m.d_medio, 0.5..=500., " mm");
        campo(ui, "Espiras ativas Na", "Espiras que de fato se deformam, sem as das pontas.", &mut m.espiras_ativas, 0.5..=200., "");
        campo(ui, "Módulo G", "Módulo de cisalhamento do arame: cerca de 79300 MPa para aço, 69000 para inox.", &mut m.g, 1000.0..=200_000., " MPa");
        ui.label(tr("Extremidades")).on_hover_text(tr("Decidem as espiras totais e a altura sólida da mola."));
        egui::ComboBox::from_id_salt("calc_mola_ext").selected_text(tr(m.extremidade.rotulo())).show_ui(ui, |ui| {
            for x in Extremidade::TODAS {
                if ui.selectable_label(m.extremidade == x, tr(x.rotulo())).clicked() {
                    m.extremidade = x;
                }
            }
        });
        ui.end_row();
        campo(ui, "Comprimento livre", "Comprimento da mola sem carga.", &mut m.comprimento_livre, 1.0..=2000., " mm");
        campo(ui, "Força de trabalho", "Força que a mola faz na condição de trabalho.", &mut m.forca, 0.0..=1e6, " N");
        campo(ui, "Tensão admissível", "Tensão de cisalhamento que o arame suporta. Use o valor do fornecedor do arame.", &mut m.tau_adm, 10.0..=3000., " MPa");
        campo(ui, "Fator de segurança exigido", "Menor fator aceito na condição de trabalho.", &mut m.fs_exigido, 1.0..=10., "");
    });
    match calcular_mola(m) {
        Err(msg) => {
            ui.label(RichText::new(format!("✖ {}", tr(&msg))).color(crate::theme::st_err()));
        }
        Ok(r) => {
            ui.separator();
            egui::Grid::new("calc_mola_out").num_columns(2).show(ui, |ui| {
                saida(ui, "Índice C", "C = D/d. A faixa usual é de 4 a 12.", format!("{:.2}", r.indice), None);
                saida(ui, "Constante k", "k = G·d⁴ / (8·D³·Na): força por milímetro de compressão.", format!("{:.3} N/mm", r.constante), None);
                saida(ui, "Compressão no trabalho", "Força de trabalho dividida pela constante.", format!("{:.2} mm", r.flecha_trabalho), None);
                saida(ui, "Fator de Wahl", "Corrige a tensão pela curvatura da espira e pelo cisalhamento direto.", format!("{:.3}", r.fator_wahl), None);
                saida(ui, "Tensão no trabalho", "τ = Kw·8·F·D / (π·d³).", format!("{:.0} MPa", r.tensao_trabalho), None);
                saida(ui, "FS no trabalho", "Tensão admissível dividida pela tensão no trabalho.", format!("{:.2}", r.fs_trabalho), Some(cor_fs(r.fs_trabalho, m.fs_exigido)));
                saida(ui, "Altura sólida", "Comprimento da mola com todas as espiras encostadas.", format!("{:.2} mm", r.altura_solida), None);
                saida(ui, "Força no sólido", "Força para comprimir a mola até encostar as espiras.", format!("{:.0} N", r.forca_solida), None);
                saida(ui, "FS no sólido", "Tensão admissível dividida pela tensão com a mola encostada.", format!("{:.2}", r.fs_solida), Some(cor_fs(r.fs_solida, 1.)));
            });
            avisos(ui, &r.avisos);
            botao_inserir(ui, "Inserir a mola", "Põe na montagem a mola no comprimento livre, com fio, diâmetro e espiras como parâmetros. As pontas saem sem encostar (modelo simplificado).", Inserir::Mola, inserir);
        }
    }
}

fn aba_viga(ui: &mut egui::Ui, e: &mut Estado, inserir: &mut Option<Inserir>) {
    egui::Grid::new("calc_viga_in").num_columns(2).show(ui, |ui| {
        ui.label(tr("Apoio")).on_hover_text(tr("Biapoiada: apoiada nas duas pontas. Engastada: presa numa ponta e livre na outra."));
        combo(ui, "calc_viga_apoio", &mut e.viga.apoio, &APOIOS);
        ui.end_row();
        ui.label(tr("Carga")).on_hover_text(tr("Pontual: força no meio do vão (biapoiada) ou na ponta (engastada). Distribuída: carga uniforme em todo o vão."));
        combo(ui, "calc_viga_carga", &mut e.viga.carga, &CARGAS);
        ui.end_row();
        let (rotulo, dica, sufixo) = match e.viga.carga {
            Carga::Pontual => ("Força", "Força aplicada, em newtons.", " N"),
            Carga::Distribuida => ("Carga distribuída", "Carga por milímetro de vão, em N/mm.", " N/mm"),
        };
        campo(ui, rotulo, dica, &mut e.viga.valor, 0.0..=1e7, sufixo);
        campo(ui, "Vão", "Distância entre os apoios, ou comprimento da viga engastada.", &mut e.viga.vao, 1.0..=100_000., " mm");
        ui.label(tr("Seção")).on_hover_text(tr("Forma da seção transversal. Genérica aceita o momento de inércia e a fibra extrema de um perfil pronto."));
        egui::ComboBox::from_id_salt("calc_viga_secao").selected_text(tr(SECOES[e.secao])).show_ui(ui, |ui| {
            for (i, s) in SECOES.iter().enumerate() {
                if ui.selectable_label(e.secao == i, tr(s)).clicked() {
                    e.secao = i;
                    e.dims = match i {
                        0 => [20., 40.],
                        1 => [30., 0.],
                        2 => [30., 26.],
                        _ => [106_667., 20.],
                    };
                }
            }
        });
        ui.end_row();
        let (r1, d1, s1) = [("Base b", "Largura da seção.", " mm"), ("Diâmetro", "Diâmetro da seção.", " mm"), ("Diâmetro externo", "Diâmetro externo do tubo.", " mm"), ("Inércia I", "Momento de inércia em torno do eixo da flexão.", " mm⁴")][e.secao];
        campo(ui, r1, d1, &mut e.dims[0], 0.001..=1e9, s1);
        match e.secao {
            0 => campo(ui, "Altura h", "Altura da seção, na direção da carga.", &mut e.dims[1], 0.001..=1e6, " mm"),
            2 => campo(ui, "Diâmetro interno", "Diâmetro interno do tubo.", &mut e.dims[1], 0.0..=1e6, " mm"),
            3 => campo(ui, "Fibra extrema c", "Distância do eixo neutro à fibra mais afastada.", &mut e.dims[1], 0.001..=1e6, " mm"),
            _ => {}
        }
        campo(ui, "Módulo E", "Módulo de elasticidade: cerca de 210000 MPa para aço, 70000 para alumínio.", &mut e.viga.e, 1000.0..=500_000., " MPa");
        campo(ui, "Escoamento", "Tensão de escoamento do material.", &mut e.viga.sy, 10.0..=3000., " MPa");
        campo(ui, "Fator de segurança exigido", "Menor fator aceito na flexão.", &mut e.viga.fs_exigido, 1.0..=10., "");
    });
    e.viga.secao = e.secao_atual();
    match calcular_viga(&e.viga) {
        Err(m) => {
            ui.label(RichText::new(format!("✖ {}", tr(&m))).color(crate::theme::st_err()));
        }
        Ok(r) => {
            ui.separator();
            egui::Grid::new("calc_viga_out").num_columns(2).show(ui, |ui| {
                saida(ui, "Inércia I", "Momento de inércia da seção.", format!("{:.0} mm⁴", r.inercia), None);
                saida(ui, "Módulo resistente W", "W = I/c.", format!("{:.0} mm³", r.modulo_resistente), None);
                saida(ui, "Momento máximo", "Maior momento fletor da viga.", format!("{:.0} N·mm", r.momento_max), None);
                saida(ui, "Cortante máximo", "Maior esforço cortante da viga.", format!("{:.0} N", r.cortante_max), None);
                saida(ui, "Flecha máxima", "Maior deslocamento vertical. O limite usual de serviço é L/250.", format!("{:.3} mm  (L/{:.0})", r.flecha_max, e.viga.vao / r.flecha_max.max(1e-12)), None);
                saida(ui, "Tensão máxima", "σ = M·c/I na fibra mais afastada.", format!("{:.1} MPa", r.tensao_max), None);
                saida(ui, "FS à flexão", "Escoamento dividido pela tensão máxima.", format!("{:.2}", r.fs), Some(cor_fs(r.fs, e.viga.fs_exigido)));
            });
            avisos(ui, &r.avisos);
            if e.secao < 3 {
                botao_inserir(ui, "Inserir a barra", "Põe na montagem a barra com a seção e o vão calculados.", Inserir::Viga, inserir);
            } else {
                ui.label(RichText::new(tr("A seção genérica não tem forma: escolha retangular, circular ou tubo para inserir a barra.")).weak().small());
            }
        }
    }
}

/// Pilha de fixadores do componente selecionado na árvore, se ele for de uma.
fn pilha_escolhida(st: &AsmState) -> Option<fenkra_core::Id> {
    st.sel.and_then(|s| pilhas::da_peca(&st.asm, s))
}

/// Põe o tamanho na pilha do componente escolhido, com um desfazer.
fn ajustar_pilha(st: &mut AsmState, tamanho: &str) -> Result<(), String> {
    let id = pilha_escolhida(st).ok_or("escolha na árvore um parafuso inserido pelo comando Fixadores")?;
    let dir = st.dir().ok_or("salve a montagem antes")?;
    st.push_undo();
    let t = tamanho.to_string();
    st.pilhas_diag = pilhas::editar(&mut st.asm, &dir, &mut st.cache, biblioteca::publica(), &[id], move |p| {
        p.tamanho = t.clone();
        p.seguir_furo = false;
    });
    st.solve();
    Ok(())
}

/// Peças da calculadora pedida, com o lugar de cada uma na montagem.
fn pecas_para_inserir(est: &Estado, qual: Inserir) -> Result<Vec<(fenkra_core::Document, fenkra_montagem::Placement)>, String> {
    use fenkra_solidos::calculo_pecas::*;
    Ok(match qual {
        Inserir::Mola => vec![(mola_documento(&est.mola)?, Default::default())],
        Inserir::Viga => vec![(viga_documento(&est.secao_atual(), est.viga.vao)?, Default::default())],
        Inserir::Ajuste => {
            let (eixo, cubo) = ajuste_documentos(&est.ajuste)?;
            // o cubo fica na união, a 10 mm da ponta do eixo
            let lugar = fenkra_montagem::Placement { origin: [0., 0., SOBRA_EIXO_AJUSTE], ..Default::default() };
            vec![(eixo, Default::default()), (cubo, lugar)]
        }
    })
}

/// Nome de arquivo a partir da descrição da peça.
fn arquivo(desc: &str) -> String {
    let s: String = desc.chars().map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else if c == ',' { 'p' } else { '-' }).collect();
    s.split('-').filter(|t| !t.is_empty()).collect::<Vec<_>>().join("-")
}

/// Grava as peças em `calculo/` e as insere no grupo "Cálculo de projeto", com um desfazer.
/// Devolve os componentes criados.
pub fn inserir_pecas(st: &mut AsmState, est: &Estado, qual: Inserir) -> Result<Vec<fenkra_core::Id>, String> {
    let pecas = pecas_para_inserir(est, qual)?;
    let dir = st.dir().ok_or("salve a montagem antes: as peças calculadas são gravadas na pasta dela")?;
    let pasta = dir.join(PASTA);
    std::fs::create_dir_all(&pasta).map_err(|e| format!("{}: {e}", pasta.display()))?;
    st.push_undo();
    let grupo = match st.asm.nodes.iter().find(|n| matches!(n.kind, fenkra_montagem::NodeKind::Group) && n.name == GRUPO).map(|n| n.id) {
        Some(g) => g,
        None => st.asm.add_group(GRUPO, None),
    };
    let mut ids = vec![];
    for (doc, lugar) in pecas {
        let nome = arquivo(&doc.props.description);
        let caminho = pasta.join(format!("{nome}.fkp"));
        std::fs::write(&caminho, doc.to_json()).map_err(|e| format!("{}: {e}", caminho.display()))?;
        ids.push(st.asm.add_component(&doc.props.description, fenkra_montagem::NodeKind::Part { path: format!("{PASTA}/{nome}.fkp") }, lugar, Some(grupo)));
    }
    st.solve();
    st.sel = ids.first().copied();
    Ok(ids)
}

pub fn painel(app: &mut App, ctx: &egui::Context, area: Rect) {
    if app.settings.bench != crate::settings::Bench::Montagem {
        return;
    }
    let Some(st) = app.asm.as_mut() else { return };
    let Some(mut est) = st.calculo.take() else { return };
    let cfg = Painel { nome: PAINEL.into(), titulo: tr("Cálculo de projeto").to_string(), padrao: paineis::abaixo_do_cubo(area, 380.), largura: 380., redimensiona: true };
    let posicao = app.settings.painel(PAINEL, Encaixe::Direita);
    let tem_pilha = pilha_escolhida(st).is_some();
    let mut fechar = false;
    let mut aplicar = None;
    let mut inserir = None;
    let mut gravar_camada = false;
    let mut aviso_camada = crate::calculo_rotativos_ui::ler_camada(&mut est.rot, st.dir().as_deref()).err().map(|m| (m, true));
    let saida = paineis::mostrar(ctx, area, &cfg, posicao, |ui| {
        if crate::app::done_bar(ui) {
            fechar = true;
        }
        ui.label(RichText::new(tr(AVISO_PRELIMINAR)).small().color(crate::theme::st_warn()));
        ui.horizontal_wrapped(|ui| {
            for a in Aba::TODAS {
                if ui.selectable_label(est.aba == a, tr(a.rotulo())).clicked() {
                    est.aba = a;
                }
            }
        });
        ui.separator();
        egui::ScrollArea::vertical().max_height(380.).auto_shrink([false, true]).show(ui, |ui| match est.aba {
            Aba::Parafuso => aba_parafuso(ui, &mut est, &mut aplicar, tem_pilha),
            Aba::Ajuste => aba_ajuste(ui, &mut est, &mut inserir),
            Aba::Mola => aba_mola(ui, &mut est, &mut inserir),
            Aba::Viga => aba_viga(ui, &mut est, &mut inserir),
            Aba::Rolamento => gravar_camada = crate::calculo_rotativos_ui::aba_rolamento(ui, &mut est.rot),
            Aba::Engrenagem => crate::calculo_rotativos_ui::aba_engrenagem(ui, &mut est.rot),
            Aba::Estriado => crate::calculo_rotativos_ui::aba_estriado(ui, &mut est.rot),
        });
    });
    if let Some(p) = saida.movido {
        app.settings.guardar_painel(PAINEL, p);
    }
    let mut aviso = aviso_camada.take();
    if gravar_camada {
        let dir = app.asm.as_ref().and_then(|st| st.dir());
        aviso = Some(match crate::calculo_rotativos_ui::gravar_camada(&mut est.rot, dir.as_deref()) {
            Ok(p) => (format!("{}: {}", tr("Camada do fabricante gravada"), p.display()), false),
            Err(m) => (tr(&m).to_string(), true),
        });
    }
    if let Some(t) = aplicar {
        let st = app.asm.as_mut().expect("montagem aberta");
        aviso = Some(match ajustar_pilha(st, &t) {
            Ok(()) => (format!("{} {t}", tr("Parafuso ajustado para")), false),
            Err(m) => (tr(&m).to_string(), true),
        });
    }
    if let Some(q) = inserir {
        let st = app.asm.as_mut().expect("montagem aberta");
        aviso = Some(match inserir_pecas(st, &est, q) {
            Ok(ids) => (format!("{}: {}", tr("Inserido na montagem"), ids.len()), false),
            Err(m) => (tr(&m).to_string(), true),
        });
    }
    if !fechar {
        app.asm.as_mut().expect("montagem aberta").calculo = Some(est);
    }
    if let Some((m, erro)) = aviso {
        app.toast(m, erro);
    }
}

/// `FENKRA_DEMO_ASM=calculo[_ajuste|_mola|_viga]`: o painel aberto na aba pedida.
pub fn demo(st: &mut AsmState, nome: &str) {
    let mut e = Estado::default();
    e.aba = match nome {
        "calculo_ajuste" => Aba::Ajuste,
        "calculo_mola" => Aba::Mola,
        "calculo_viga" => Aba::Viga,
        "calculo_rolamento" => Aba::Rolamento,
        "calculo_engrenagem" => Aba::Engrenagem,
        "calculo_estriado" => Aba::Estriado,
        _ => Aba::Parafuso,
    };
    if e.aba == Aba::Rolamento {
        // dado de exemplo, como a pessoa o escreveria a partir de um catálogo (não é dado embutido)
        e.rot.camada_lida = true;
        e.rot.rolamento.dados = fenkra_solidos::calculo_rotativos::DadosFabricante { designacao: "6005".into(), fonte: "exemplo da demonstração".into(), c: 10_000., c0: 5_000., ..Default::default() };
        e.rot.rolamento.fr = 1500.;
    }
    if nome == "calculo_inserido" {
        // B110: mola, eixo e cubo do ajuste e a barra da viga já inseridos na montagem
        for q in [Inserir::Mola, Inserir::Ajuste] {
            if let Err(m) = inserir_pecas(st, &e, q) {
                st.aviso = Some((m, true));
            }
        }
        e.aba = Aba::Mola;
    }
    st.calculo = Some(e);
}

#[cfg(test)]
mod tests {
    use super::*;
    use fenkra_montagem::Assembly;

    #[test]
    fn abrir_liga_e_desliga() {
        let mut st = AsmState::new(Assembly::new("t"), None);
        abrir(&mut st);
        assert!(st.calculo.is_some());
        abrir(&mut st);
        assert!(st.calculo.is_none());
    }

    #[test]
    fn secao_da_viga_segue_o_seletor() {
        let mut e = Estado::default();
        assert_eq!(e.secao_atual(), Secao::Retangular { b: 20., h: 40. });
        e.secao = 2;
        e.dims = [30., 26.];
        assert_eq!(e.secao_atual(), Secao::Tubo { d_ext: 30., d_int: 26. });
    }

    #[test]
    fn ajustar_sem_parafuso_escolhido_avisa() {
        let mut st = AsmState::new(Assembly::new("t"), None);
        assert!(ajustar_pilha(&mut st, "M8").unwrap_err().contains("Fixadores"));
        assert!(st.undo.is_empty());
    }

    /// B110: cada aba insere a peça calculada no grupo do cálculo, gravada em `calculo/`, com desfazer.
    #[test]
    fn inserir_pecas_grava_e_poe_no_grupo() {
        let mut st = AsmState::new(Assembly::new("t"), None);
        assert!(inserir_pecas(&mut st, &Estado::default(), Inserir::Mola).unwrap_err().contains("salve"));
        assert!(st.undo.is_empty());
        let dir = std::env::temp_dir().join(format!("fenkra_calc_inserir_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        st.file = Some(dir.join("m.fkasm"));
        let est = Estado::default();
        let mola = inserir_pecas(&mut st, &est, Inserir::Mola).unwrap();
        let ajuste = inserir_pecas(&mut st, &est, Inserir::Ajuste).unwrap();
        let viga = inserir_pecas(&mut st, &est, Inserir::Viga).unwrap();
        assert_eq!((mola.len(), ajuste.len(), viga.len()), (1, 2, 1));
        assert_eq!(st.undo.len(), 3);
        let grupos: Vec<_> = st.asm.nodes.iter().filter(|n| n.name == GRUPO).collect();
        assert_eq!(grupos.len(), 1, "um grupo só");
        for id in mola.iter().chain(&ajuste).chain(&viga) {
            let n = st.asm.node(*id).unwrap();
            assert_eq!(n.parent, Some(grupos[0].id));
            let fenkra_montagem::NodeKind::Part { path } = &n.kind else { panic!("peça") };
            assert!(path.starts_with("calculo/") && dir.join(path).exists(), "{path}");
        }
        // o cubo do ajuste fica na união, a 10 mm da ponta do eixo
        assert_eq!(st.asm.node(ajuste[1]).unwrap().placement.origin, [0., 0., 10.]);
        // a viga de seção genérica não tem forma para inserir
        let generica = Estado { secao: 3, ..Estado::default() };
        assert!(inserir_pecas(&mut st, &generica, Inserir::Viga).is_err());
        assert_eq!(arquivo("Mola de compressão Ø2 × Ø16 × 60"), "mola-de-compress-o-2-16-60");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn todo_rotulo_de_aba_e_toda_classe_existem() {
        for a in Aba::TODAS {
            assert!(!a.rotulo().is_empty());
        }
        assert!(AJUSTES.iter().all(|a| fenkra_tolerancias_ok(a)));
    }

    fn fenkra_tolerancias_ok(a: &str) -> bool {
        calcular_ajuste(&EntradaAjuste { ajuste: a.into(), ..Default::default() }).is_ok()
    }
}
