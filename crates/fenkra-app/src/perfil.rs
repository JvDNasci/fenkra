// SPDX-License-Identifier: LGPL-2.1-or-later
//! Perfil de hardware e modo leve (B236).
//!
//! O modo leve não tem caminho de código próprio: é só uma combinação de ajustes que já existem
//! (malha Rascunho, sem MSAA, simplificar ao girar, iluminação simples, 2 fios no malhador,
//! animações desligadas). Volume, massa e medidas não mudam, porque vêm da forma exata e não da malha.
//! A sugestão nasce do hardware medido na partida, aparece uma vez e nunca é imposta.

use crate::render::Msaa;
use crate::settings::Settings;
use eframe::egui_wgpu::{self, wgpu};
use fenkra_core::PresetMalha;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// Limites abaixo dos quais a máquina é considerada fraca (docs/benchmark/detalhe/kernel.md).
pub const RAM_MIN_GB: f64 = 8.0;
pub const LOGICOS_MAX: usize = 4;
/// Fios do malhador no modo leve.
pub const FIOS_LEVE: usize = 2;

/// O que foi medido na máquina. Fica só aqui e no registro local de partida.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hardware {
    /// adaptador de software (tipo `Cpu`)
    pub software: bool,
    /// backend OpenGL
    pub opengl: bool,
    /// memória física total em GB, quando o sistema informa
    pub ram_gb: Option<f64>,
    pub logicos: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Motivo {
    Software,
    OpenGl,
    Memoria,
    Processadores,
}

impl Motivo {
    /// Texto em português; passe por `tr` ao exibir.
    pub fn texto(self) -> &'static str {
        match self {
            Motivo::Software => "adaptador gráfico por software",
            Motivo::OpenGl => "placa de vídeo usando OpenGL",
            Motivo::Memoria => "menos de 8 GB de memória",
            Motivo::Processadores => "até 4 processadores lógicos",
        }
    }
}

/// Por que o modo leve seria uma boa ideia nesta máquina (lista vazia: não sugerir).
pub fn motivos(h: &Hardware) -> Vec<Motivo> {
    let mut v = vec![];
    if h.software {
        v.push(Motivo::Software);
    }
    if h.opengl {
        v.push(Motivo::OpenGl);
    }
    if h.ram_gb.is_some_and(|g| g < RAM_MIN_GB) {
        v.push(Motivo::Memoria);
    }
    if h.logicos > 0 && h.logicos <= LOGICOS_MAX {
        v.push(Motivo::Processadores);
    }
    v
}

static MEDIDO: OnceLock<Hardware> = OnceLock::new();

pub fn medido() -> Option<Hardware> {
    MEDIDO.get().copied()
}

/// Mede e guarda o hardware a partir do adaptador escolhido. Chamado uma vez na partida; o
/// resultado só vai ao registro local de partida.
pub fn detectar(rs: &egui_wgpu::RenderState) -> Hardware {
    let i = rs.adapter.get_info();
    let h = Hardware { software: i.device_type == wgpu::DeviceType::Cpu, opengl: i.backend == wgpu::Backend::Gl, ram_gb: memoria_fisica_gb(), logicos: std::thread::available_parallelism().map_or(0, |n| n.get()) };
    crate::partida::registrar("perfil", &format!("software {} | opengl {} | memória {:?} GB | lógicos {}", h.software, h.opengl, h.ram_gb, h.logicos));
    let _ = MEDIDO.set(h);
    h
}

#[cfg(windows)]
fn memoria_fisica_gb() -> Option<f64> {
    #[repr(C)]
    struct MemoryStatusEx {
        length: u32,
        load: u32,
        total_phys: u64,
        avail_phys: u64,
        total_page: u64,
        avail_page: u64,
        total_virtual: u64,
        avail_virtual: u64,
        avail_ext: u64,
    }
    extern "system" {
        fn GlobalMemoryStatusEx(b: *mut MemoryStatusEx) -> i32;
    }
    let mut m = MemoryStatusEx { length: std::mem::size_of::<MemoryStatusEx>() as u32, load: 0, total_phys: 0, avail_phys: 0, total_page: 0, avail_page: 0, total_virtual: 0, avail_virtual: 0, avail_ext: 0 };
    // SAFETY: estrutura com o tamanho preenchido, como a função exige
    (unsafe { GlobalMemoryStatusEx(&mut m) } != 0).then(|| m.total_phys as f64 / (1u64 << 30) as f64)
}

#[cfg(not(windows))]
fn memoria_fisica_gb() -> Option<f64> {
    let t = std::fs::read_to_string("/proc/meminfo").ok()?;
    let kb: f64 = t.lines().find_map(|l| l.strip_prefix("MemTotal:"))?.split_whitespace().next()?.parse().ok()?;
    Some(kb / (1u64 << 20) as f64)
}

/// Ajustes de antes de ligar o modo leve, para o desligar devolver cada um ao que a pessoa tinha.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct Anterior {
    pub preset: PresetMalha,
    pub msaa: Msaa,
    pub simplificar_ao_girar: bool,
    pub iluminacao_realista: bool,
    pub animacoes: bool,
    /// B139: efeitos da vista de antes (configuração antiga sem o campo volta aos de fábrica)
    #[serde(default)]
    pub efeitos: crate::render_efeitos::Efeitos,
}

impl Settings {
    /// Liga ou desliga o modo leve, combinando os ajustes existentes.
    pub fn definir_modo_leve(&mut self, ligar: bool) {
        if ligar == self.modo_leve {
            return;
        }
        if ligar {
            let g = &mut self.graficos;
            self.leve_anterior = Some(Anterior { preset: g.preset, msaa: g.msaa, simplificar_ao_girar: g.simplificar_ao_girar, iluminacao_realista: g.iluminacao_realista, animacoes: self.animacoes, efeitos: g.efeitos });
            g.escolher(PresetMalha::Rascunho);
            g.msaa = Msaa::Desligado;
            g.simplificar_ao_girar = true;
            g.iluminacao_realista = false;
            // B139: os efeitos nascem desligados no modo leve
            g.efeitos = crate::render_efeitos::Efeitos { escolhidos: g.efeitos.escolhidos, ..crate::render_efeitos::Efeitos::DESLIGADOS };
            self.animacoes = false;
        } else if let Some(a) = self.leve_anterior.take() {
            let g = &mut self.graficos;
            // direto, sem `escolher`: na Personalizada ela sobrescreveria os valores livres da pessoa
            g.preset = a.preset;
            g.msaa = a.msaa;
            g.simplificar_ao_girar = a.simplificar_ao_girar;
            g.iluminacao_realista = a.iluminacao_realista;
            g.efeitos = a.efeitos;
            self.animacoes = a.animacoes;
        }
        self.modo_leve = ligar;
    }

    /// B139: no adaptador de software os efeitos da vista nascem desligados, até a pessoa escolher.
    pub fn efeitos_do_adaptador(&mut self, h: &Hardware) {
        if h.software && !self.graficos.efeitos.escolhidos {
            self.graficos.efeitos = crate::render_efeitos::Efeitos::DESLIGADOS;
        }
    }

    /// Teto de fios do malhador pedido pelas configurações (`None`: padrão do kernel).
    pub fn fios_malhador(&self) -> Option<usize> {
        self.modo_leve.then_some(FIOS_LEVE)
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn forte() -> Hardware {
        Hardware { software: false, opengl: false, ram_gb: Some(16.0), logicos: 16 }
    }

    #[test]
    fn efeitos_desligados_no_modo_leve_e_no_software() {
        use crate::render_efeitos::{Efeitos, Nivel};
        let mut s = Settings::default();
        assert!(s.graficos.efeitos.sombra && s.graficos.efeitos.oclusao && s.graficos.efeitos.pbr);
        s.graficos.efeitos.nivel = Nivel::Alto;
        s.definir_modo_leve(true);
        let e = s.graficos.efeitos;
        assert!(e.nivel == Nivel::Rapido && !e.sombra && !e.oclusao && !e.pbr);
        s.definir_modo_leve(false);
        assert_eq!(s.graficos.efeitos.nivel, Nivel::Alto);
        assert!(s.graficos.efeitos.sombra);
        // software: desliga na partida, mas respeita a escolha da pessoa
        let mut t = Settings::default();
        t.efeitos_do_adaptador(&Hardware { software: true, ..forte() });
        assert_eq!(t.graficos.efeitos, Efeitos::DESLIGADOS);
        let mut u = Settings::default();
        u.efeitos_do_adaptador(&forte());
        assert_eq!(u.graficos.efeitos, Efeitos::default());
        u.graficos.efeitos.escolhidos = true;
        u.efeitos_do_adaptador(&Hardware { software: true, ..forte() });
        assert!(u.graficos.efeitos.sombra);
    }

    #[test]
    fn maquina_forte_nao_recebe_sugestao() {
        assert!(motivos(&forte()).is_empty());
        // memória desconhecida não vira motivo
        assert!(motivos(&Hardware { ram_gb: None, ..forte() }).is_empty());
    }

    #[test]
    fn cada_criterio_sugere_sozinho() {
        assert_eq!(motivos(&Hardware { software: true, ..forte() }), vec![Motivo::Software]);
        assert_eq!(motivos(&Hardware { opengl: true, ..forte() }), vec![Motivo::OpenGl]);
        assert_eq!(motivos(&Hardware { ram_gb: Some(7.9), ..forte() }), vec![Motivo::Memoria]);
        assert!(motivos(&Hardware { ram_gb: Some(8.0), ..forte() }).is_empty());
        assert_eq!(motivos(&Hardware { logicos: 4, ..forte() }), vec![Motivo::Processadores]);
        assert!(motivos(&Hardware { logicos: 5, ..forte() }).is_empty());
    }

    #[test]
    fn modo_leve_combina_ajustes_e_desfaz() {
        let mut s = Settings::default();
        s.graficos.msaa = Msaa::X8;
        s.graficos.escolher(PresetMalha::Alta);
        let antes = s.clone();
        s.definir_modo_leve(true);
        assert!(s.modo_leve);
        assert_eq!(s.graficos.preset, PresetMalha::Rascunho);
        assert_eq!(s.graficos.msaa, Msaa::Desligado);
        assert!(s.graficos.simplificar_ao_girar && !s.graficos.iluminacao_realista && !s.animacoes);
        assert_eq!(s.fios_malhador(), Some(FIOS_LEVE));
        s.definir_modo_leve(false);
        assert_eq!(s, antes);
        assert_eq!(s.fios_malhador(), None);
    }

    #[test]
    fn configuracao_antiga_abre_sem_modo_leve() {
        let s: Settings = serde_json::from_str(r#"{"ui_scale": 1.2}"#).unwrap();
        assert!(!s.modo_leve && s.animacoes && !s.sugestao_leve_vista);
    }
}
