// SPDX-License-Identifier: LGPL-2.1-or-later
//! Abas Rolamento, Engrenagem e Estriado do painel Cálculo de projeto (B111).
//!
//! As contas são de `fenkra_solidos::calculo_rotativos` (vida L10 e estriado) e de
//! `fenkra_solidos::engrenagens_resistencia` (Lewis e Hertz). Os dados de capacidade do rolamento
//! vêm do fabricante: a pessoa os escreve (com a fonte) e grava na **camada do fabricante**, um JSON
//! em `calculo/camada_fabricante.json` ao lado da montagem, que volta quando o painel abre de novo.

use crate::calculo_ui::{avisos, campo, cor_fs, material, saida};
use crate::i18n::tr;
use eframe::egui::{self, RichText};
use fenkra_solidos::calculo_rotativos::*;
use fenkra_solidos::engrenagens::ParamEngrenagem;
use fenkra_solidos::engrenagens_resistencia::{verificar_par, EntradaVerificacao};
use fenkra_solidos::rolamentos::{rolamento, ROLAMENTOS};
use std::path::{Path, PathBuf};

/// Arquivo da camada do fabricante, dentro da pasta da montagem.
pub const ARQUIVO_CAMADA: &str = "calculo/camada_fabricante.json";

#[derive(Clone, Debug)]
pub struct EstadoRotativos {
    pub rolamento: EntradaRolamento,
    pub camada: CamadaFabricante,
    /// a camada já foi lida da pasta da montagem
    pub camada_lida: bool,
    pub estriado: EntradaEstriado,
    pub pinhao: ParamEngrenagem,
    pub dentes_coroa: u32,
    pub engrenagem: EntradaVerificacao,
}

impl Default for EstadoRotativos {
    fn default() -> Self {
        EstadoRotativos {
            rolamento: EntradaRolamento::default(),
            camada: CamadaFabricante::default(),
            camada_lida: false,
            estriado: EntradaEstriado::default(),
            pinhao: ParamEngrenagem { modulo: 2., dentes: 20, largura: 20., ..Default::default() },
            dentes_coroa: 40,
            engrenagem: EntradaVerificacao::default(),
        }
    }
}

pub fn caminho_camada(dir: &Path) -> PathBuf {
    dir.join(ARQUIVO_CAMADA)
}

/// Lê a camada da pasta da montagem (uma vez); arquivo ausente = camada vazia.
pub fn ler_camada(e: &mut EstadoRotativos, dir: Option<&Path>) -> Result<(), String> {
    if e.camada_lida {
        return Ok(());
    }
    e.camada_lida = true;
    let Some(dir) = dir else { return Ok(()) };
    let p = caminho_camada(dir);
    if !p.exists() {
        return Ok(());
    }
    let s = std::fs::read_to_string(&p).map_err(|er| format!("{}: {er}", p.display()))?;
    e.camada = CamadaFabricante::de_json(&s)?;
    if let Some(l) = e.camada.buscar(&e.rolamento.dados.designacao) {
        e.rolamento.dados = l.clone();
    }
    Ok(())
}

/// Grava os dados do rolamento atual na camada e o arquivo da camada.
pub fn gravar_camada(e: &mut EstadoRotativos, dir: Option<&Path>) -> Result<PathBuf, String> {
    let dir = dir.ok_or("salve a montagem antes: a camada do fabricante fica na pasta dela")?;
    let d = &e.rolamento.dados;
    if d.designacao.trim().is_empty() || !(d.c > 0. && d.c0 > 0.) {
        return Err("a linha da camada precisa de designação, C e C0".into());
    }
    if d.fonte.trim().is_empty() {
        return Err("informe a fonte de C e C0 antes de gravar na camada".into());
    }
    e.camada.gravar(d.clone());
    let p = caminho_camada(dir);
    if let Some(pai) = p.parent() {
        std::fs::create_dir_all(pai).map_err(|er| format!("{}: {er}", pai.display()))?;
    }
    std::fs::write(&p, e.camada.para_json()).map_err(|er| format!("{}: {er}", p.display()))?;
    Ok(p)
}

/// Aba Rolamento. Devolve `true` quando a pessoa pede para gravar na camada.
pub fn aba_rolamento(ui: &mut egui::Ui, e: &mut EstadoRotativos) -> bool {
    let mut gravar = false;
    let r = &mut e.rolamento;
    egui::Grid::new("calc_rol_in").num_columns(2).show(ui, |ui| {
        ui.label(tr("Rolamento")).on_hover_text(tr("Designação. As medidas de contorno vêm da biblioteca; a capacidade de carga vem do fabricante."));
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut r.dados.designacao).desired_width(70.));
            egui::ComboBox::from_id_salt("calc_rol_lista").selected_text(tr("da biblioteca")).show_ui(ui, |ui| {
                for b in ROLAMENTOS {
                    if ui.selectable_label(r.dados.designacao == b.designacao, b.designacao).clicked() {
                        r.dados = match e.camada.buscar(b.designacao) {
                            Some(l) => l.clone(),
                            None => DadosFabricante { designacao: b.designacao.into(), corpo: r.dados.corpo, ..Default::default() },
                        };
                    }
                }
            });
        });
        ui.end_row();
        if let Some(b) = rolamento(&r.dados.designacao) {
            saida(ui, "Contorno", "Furo × externo × largura, da biblioteca (ISO 15).", format!("{} × {} × {} mm", b.d, b.dext, b.b), None);
        }
        ui.label(tr("Corpo rolante")).on_hover_text(tr("Esferas: expoente 3. Rolos: expoente 10/3."));
        egui::ComboBox::from_id_salt("calc_rol_corpo").selected_text(tr(r.dados.corpo.rotulo())).show_ui(ui, |ui| {
            for c in [CorpoRolante::Esferas, CorpoRolante::Rolos] {
                if ui.selectable_label(r.dados.corpo == c, tr(c.rotulo())).clicked() {
                    r.dados.corpo = c;
                }
            }
        });
        ui.end_row();
        campo(ui, "Capacidade dinâmica C", "Do catálogo do fabricante. Não vem embutida: cada fabricante dá a sua.", &mut r.dados.c, 0.0..=1e8, " N");
        campo(ui, "Capacidade estática C0", "Do catálogo do fabricante.", &mut r.dados.c0, 0.0..=1e8, " N");
        campo(ui, "Fator X", "Fator radial da carga equivalente P = X·Fr + Y·Fa, do catálogo.", &mut r.dados.x, 0.0..=10., "");
        campo(ui, "Fator Y", "Fator axial da carga equivalente, do catálogo. Zero sem carga axial.", &mut r.dados.y, 0.0..=10., "");
        campo(ui, "Fator X0", "Fator radial da carga estática P0 = X0·Fr + Y0·Fa, do catálogo.", &mut r.dados.x0, 0.0..=10., "");
        campo(ui, "Fator Y0", "Fator axial da carga estática, do catálogo.", &mut r.dados.y0, 0.0..=10., "");
        ui.label(tr("Fonte dos dados")).on_hover_text(tr("Fabricante e catálogo de onde vieram C, C0 e os fatores."));
        ui.add(egui::TextEdit::singleline(&mut r.dados.fonte).desired_width(180.));
        ui.end_row();
        campo(ui, "Carga radial Fr", "Carga perpendicular ao eixo.", &mut r.fr, 0.0..=1e8, " N");
        campo(ui, "Carga axial Fa", "Carga ao longo do eixo.", &mut r.fa, 0.0..=1e8, " N");
        campo(ui, "Rotação", "Rotação do anel que gira.", &mut r.rpm, 0.1..=1e6, " rpm");
        campo(ui, "Fator a1", "Confiabilidade: 1 para 90 % dos rolamentos chegarem à vida calculada. Use o valor do fabricante para outra confiabilidade.", &mut r.a1, 0.01..=1., "");
        campo(ui, "Vida exigida", "Horas de trabalho que o rolamento precisa durar.", &mut r.vida_exigida_h, 0.0..=1e7, " h");
        campo(ui, "Segurança estática exigida", "Menor s0 = C0/P0 aceito.", &mut r.s0_exigido, 0.1..=10., "");
    });
    ui.horizontal(|ui| {
        if ui.button(tr("Gravar na camada do fabricante")).on_hover_text(tr("Guarda C, C0, os fatores e a fonte desta designação no arquivo da camada, ao lado da montagem.")).clicked() {
            gravar = true;
        }
        ui.label(RichText::new(format!("{} {}", e.camada.linhas.len(), tr("na camada"))).weak().small());
    });
    match calcular_rolamento(&e.rolamento) {
        Err(m) => {
            ui.label(RichText::new(format!("✖ {}", tr(&m))).color(crate::theme::st_err()));
        }
        Ok(res) => {
            ui.separator();
            let r = &e.rolamento;
            egui::Grid::new("calc_rol_out").num_columns(2).show(ui, |ui| {
                saida(ui, "Carga equivalente P", "P = X·Fr + Y·Fa.", format!("{:.0} N", res.p), None);
                saida(ui, "Vida L10", "L10 = a1·(C/P)^p milhões de voltas (Lundberg e Palmgren).", format!("{:.1} × 10⁶ voltas", res.l10), None);
                saida(ui, "Vida L10h", "L10h = L10·10⁶ / (60·n).", format!("{:.0} h", res.l10h), Some(cor_fs(res.l10h, r.vida_exigida_h)));
                saida(ui, "P máx. para a vida exigida", "Maior carga equivalente que ainda dá a vida exigida.", format!("{:.0} N", res.p_max), None);
                saida(ui, "Segurança estática s0", "s0 = C0 / P0.", format!("{:.2}", res.s0), Some(cor_fs(res.s0, r.s0_exigido)));
            });
            avisos(ui, &res.avisos);
        }
    }
    gravar
}

/// Aba Engrenagem: verificação do par por Lewis (flexão) e Hertz (contato).
pub fn aba_engrenagem(ui: &mut egui::Ui, e: &mut EstadoRotativos) {
    egui::Grid::new("calc_eng_in").num_columns(2).show(ui, |ui| {
        campo(ui, "Módulo", "Módulo do par.", &mut e.pinhao.modulo, 0.2..=50., " mm");
        let mut z1 = e.pinhao.dentes as f64;
        campo(ui, "Dentes do pinhão", "Dentes da engrenagem menor, a motora.", &mut z1, 6.0..=400., "");
        e.pinhao.dentes = z1.round() as u32;
        let mut z2 = e.dentes_coroa as f64;
        campo(ui, "Dentes da coroa", "Dentes da engrenagem maior.", &mut z2, 6.0..=1000., "");
        e.dentes_coroa = z2.round() as u32;
        campo(ui, "Largura de face", "Largura dos dentes que se tocam.", &mut e.pinhao.largura, 1.0..=1000., " mm");
        campo(ui, "Ângulo de pressão", "Ângulo de pressão do perfil.", &mut e.pinhao.angulo_pressao, 14.5..=30., "°");
        campo(ui, "Potência", "Potência transmitida pelo par.", &mut e.engrenagem.potencia_kw, 0.0..=1e5, " kW");
        campo(ui, "Rotação do pinhão", "Rotação da engrenagem motora.", &mut e.engrenagem.rpm_pinhao, 0.1..=1e6, " rpm");
        ui.label(tr("Material do pinhão")).on_hover_text(tr("Valores típicos de módulo, Poisson e escoamento. Confira com o material real."));
        material(ui, "calc_eng_mp", &mut e.engrenagem.material_pinhao);
        ui.end_row();
        ui.label(tr("Material da coroa")).on_hover_text(tr("Valores típicos de módulo, Poisson e escoamento. Confira com o material real."));
        material(ui, "calc_eng_mc", &mut e.engrenagem.material_coroa);
        ui.end_row();
        campo(ui, "Pressão de contato admissível", "Do material mais fraco, do fornecedor. Zero para só ver a pressão.", &mut e.engrenagem.contato_admissivel, 0.0..=5000., " MPa");
        campo(ui, "Fator de segurança exigido", "Menor fator aceito à flexão e ao contato.", &mut e.engrenagem.fs_exigido, 1.0..=10., "");
    });
    let coroa = ParamEngrenagem { dentes: e.dentes_coroa, ..e.pinhao.clone() };
    match verificar_par(&e.pinhao, &coroa, &e.engrenagem) {
        Err(m) => {
            ui.label(RichText::new(format!("✖ {}", tr(&m))).color(crate::theme::st_err()));
        }
        Ok(r) => {
            ui.separator();
            let fs = e.engrenagem.fs_exigido;
            egui::Grid::new("calc_eng_out").num_columns(2).show(ui, |ui| {
                saida(ui, "Torque no pinhão", "T = P / ω.", format!("{:.2} N·m", r.torque_pinhao), None);
                saida(ui, "Força tangencial", "Ft = 2·T / d no primitivo.", format!("{:.0} N", r.forca_tangencial), None);
                saida(ui, "Velocidade no primitivo", "Entra no fator dinâmico de Barth.", format!("{:.2} m/s  (Kv {:.2})", r.velocidade, r.kv), None);
                saida(ui, "Flexão no pinhão", "σF = Kv·Ft / (b·m·Y), com Y de Lewis do dente gerado.", format!("{:.0} MPa  (Y {:.3})", r.flexao_pinhao, r.y_pinhao), None);
                saida(ui, "FS à flexão do pinhão", "Escoamento dividido pela tensão de flexão.", format!("{:.2}", r.fs_flexao_pinhao), Some(cor_fs(r.fs_flexao_pinhao, fs)));
                saida(ui, "Flexão na coroa", "σF da coroa.", format!("{:.0} MPa  (Y {:.3})", r.flexao_coroa, r.y_coroa), None);
                saida(ui, "FS à flexão da coroa", "Escoamento dividido pela tensão de flexão.", format!("{:.2}", r.fs_flexao_coroa), Some(cor_fs(r.fs_flexao_coroa, fs)));
                saida(ui, "Pressão de contato", "Hertz com os raios de curvatura da evolvente no primitivo.", format!("{:.0} MPa", r.contato), None);
                if r.fs_contato > 0. {
                    saida(ui, "FS ao contato", "Pressão admissível dividida pela de Hertz.", format!("{:.2}", r.fs_contato), Some(cor_fs(r.fs_contato, fs)));
                }
            });
            avisos(ui, &r.avisos);
        }
    }
}

/// Aba Estriado de flancos retos.
pub fn aba_estriado(ui: &mut egui::Ui, e: &mut EstadoRotativos) {
    let s = &mut e.estriado;
    egui::Grid::new("calc_est_in").num_columns(2).show(ui, |ui| {
        let mut n = s.n as f64;
        campo(ui, "Número de estrias", "Quantidade de estrias do eixo.", &mut n, 3.0..=40., "");
        s.n = n.round() as u32;
        campo(ui, "Diâmetro menor d", "Diâmetro do fundo das estrias.", &mut s.d, 1.0..=1000., " mm");
        campo(ui, "Diâmetro maior D", "Diâmetro do topo das estrias.", &mut s.d_maior, 1.0..=1000., " mm");
        campo(ui, "Chanfro", "Chanfro dos cantos, tirado duas vezes da altura de contato.", &mut s.chanfro, 0.0..=20., " mm");
        campo(ui, "Comprimento de contato", "Comprimento em que o cubo cobre as estrias.", &mut s.comprimento, 1.0..=2000., " mm");
        campo(ui, "Pressão admissível", "Pressão nos flancos que o par de materiais aguenta; depende de a união deslizar ou ficar fixa.", &mut s.p_adm, 1.0..=1000., " MPa");
        campo(ui, "Fator de divisão φ", "Parcela dos flancos que de fato carrega. 0,75 é o valor típico.", &mut s.phi, 0.1..=1., "");
        campo(ui, "Torque exigido", "Torque que o estriado precisa transmitir.", &mut s.torque, 0.0..=1e7, " N·m");
        campo(ui, "Cisalhamento admissível do eixo", "Para conferir o núcleo do eixo no diâmetro menor.", &mut s.tau_adm, 1.0..=2000., " MPa");
        campo(ui, "Fator de segurança exigido", "Menor fator aceito.", &mut s.fs_exigido, 1.0..=10., "");
    });
    match calcular_estriado(&e.estriado) {
        Err(m) => {
            ui.label(RichText::new(format!("✖ {}", tr(&m))).color(crate::theme::st_err()));
        }
        Ok(r) => {
            ui.separator();
            let fs = e.estriado.fs_exigido;
            egui::Grid::new("calc_est_out").num_columns(2).show(ui, |ui| {
                saida(ui, "Altura útil h", "h = (D − d)/2 − 2·chanfro.", format!("{:.2} mm", r.altura_util), None);
                saida(ui, "Raio médio", "rm = (D + d)/4.", format!("{:.2} mm", r.raio_medio), None);
                saida(ui, "Torque dos flancos", "T = φ·n·h·L·p·rm.", format!("{:.1} N·m", r.torque_flancos), None);
                saida(ui, "Pressão nos flancos", "Pressão com o torque exigido.", format!("{:.1} MPa", r.pressao), None);
                saida(ui, "FS dos flancos", "Torque dos flancos dividido pelo exigido.", format!("{:.2}", r.fs_flancos), Some(cor_fs(r.fs_flancos, fs)));
                saida(ui, "Cisalhamento no núcleo", "τ = 16·T / (π·d³).", format!("{:.1} MPa", r.tau_nucleo), None);
                saida(ui, "FS do núcleo", "Cisalhamento admissível dividido pelo do núcleo.", format!("{:.2}", r.fs_nucleo), Some(cor_fs(r.fs_nucleo, fs)));
            });
            avisos(ui, &r.avisos);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camada_grava_na_pasta_e_volta_ao_abrir() {
        let dir = std::env::temp_dir().join(format!("fenkra_camada_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut e = EstadoRotativos::default();
        assert!(gravar_camada(&mut e, Some(&dir)).is_err(), "sem C e C0");
        e.rolamento.dados = DadosFabricante { designacao: "6005".into(), c: 10_000., c0: 5_000., fonte: String::new(), ..Default::default() };
        assert!(gravar_camada(&mut e, Some(&dir)).unwrap_err().contains("fonte"));
        e.rolamento.dados.fonte = "catálogo do teste".into();
        let p = gravar_camada(&mut e, Some(&dir)).unwrap();
        assert!(p.ends_with("calculo/camada_fabricante.json") || p.ends_with("calculo\\camada_fabricante.json"));
        // um painel novo lê a camada e já preenche a designação escolhida
        let mut novo = EstadoRotativos::default();
        ler_camada(&mut novo, Some(&dir)).unwrap();
        assert_eq!(novo.rolamento.dados.c, 10_000.);
        assert_eq!(novo.rolamento.dados.fonte, "catálogo do teste");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn engrenagem_padrao_verifica() {
        let e = EstadoRotativos::default();
        let coroa = ParamEngrenagem { dentes: e.dentes_coroa, ..e.pinhao.clone() };
        let r = verificar_par(&e.pinhao, &coroa, &e.engrenagem).unwrap();
        assert!(r.fs_flexao_pinhao > 0. && r.contato > 0.);
    }
}
