//! Partida que sempre explica (B221) e contador de quadros (B222).
//!
//! O aplicativo é de janela (sem console): um pânico ou uma falha de GPU antes da primeira tela
//! fechava o programa em silêncio. Aqui ficam o registro `partida.log`, o gancho de pânico, a caixa
//! de mensagem traduzida e a nova tentativa com outros backends gráficos.

use eframe::egui_wgpu::{self, wgpu};
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

/// Quadros desenhados desde o início. Em repouso não deve crescer (B222).
static QUADROS: AtomicU64 = AtomicU64::new(0);

/// Chamado uma vez por quadro, no começo de `App::ui`.
pub fn contar_quadro() {
    QUADROS.fetch_add(1, Ordering::Relaxed);
}

pub fn quadros() -> u64 {
    QUADROS.load(Ordering::Relaxed)
}

/// Com `FENKRA_QUADROS=1`, registra a cada 5 s quantos quadros foram desenhados no intervalo.
/// Uma linha com `+0` prova que o aplicativo ficou em repouso de verdade.
pub fn medir_quadros_se_pedido() {
    if std::env::var_os("FENKRA_QUADROS").is_none() {
        return;
    }
    std::thread::spawn(|| {
        let mut antes = quadros();
        loop {
            std::thread::sleep(std::time::Duration::from_secs(5));
            let agora = quadros();
            registrar("quadros", &format!("+{} em 5 s (total {agora})", agora - antes));
            antes = agora;
        }
    });
}

/// Pasta de dados do Fenkra (a mesma das configurações).
fn pasta() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join("Fenkra"))
}

pub fn caminho_log() -> Option<PathBuf> {
    pasta().map(|p| p.join("partida.log"))
}

/// Acrescenta uma linha `[segundos] etapa: texto` ao registro. Nunca falha para quem chama.
pub fn registrar(etapa: &str, texto: &str) {
    let Some(p) = caminho_log() else { return };
    let _ = std::fs::create_dir_all(p.parent().unwrap_or(&p));
    let seg = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&p) {
        let _ = writeln!(f, "[{seg}] {etapa}: {texto}");
    }
}

/// Começa um registro novo a cada partida (guarda o anterior como `partida.anterior.log`).
fn girar_log() {
    if let Some(p) = caminho_log() {
        if std::fs::metadata(&p).map(|m| m.len() > 0).unwrap_or(false) {
            let _ = std::fs::rename(&p, p.with_file_name("partida.anterior.log"));
        }
    }
}

/// O idioma salvo, lido direto do arquivo de configurações (a partida roda antes de carregá-las).
fn ingles() -> bool {
    pasta()
        .and_then(|p| std::fs::read_to_string(p.join("config.json")).ok())
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v.get("english").and_then(|e| e.as_bool()))
        .unwrap_or(false)
}

/// Caixa de mensagem nativa; sem ela o erro só estaria no arquivo.
fn caixa(titulo: &str, corpo: &str) {
    let _ = rfd::MessageDialog::new().set_level(rfd::MessageLevel::Error).set_title(titulo).set_description(corpo).set_buttons(rfd::MessageButtons::Ok).show();
}

fn texto_log() -> String {
    caminho_log().map(|p| p.display().to_string()).unwrap_or_else(|| "%APPDATA%\\Fenkra\\partida.log".into())
}

/// Instala o gancho de pânico: grava local e pilha no registro e mostra uma mensagem clara.
pub fn instalar_gancho() {
    let anterior = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let onde = info.location().map(|l| format!("{}:{}", l.file(), l.line())).unwrap_or_else(|| "?".into());
        let msg = if let Some(s) = info.payload().downcast_ref::<&str>() { (*s).to_string() } else if let Some(s) = info.payload().downcast_ref::<String>() { s.clone() } else { "sem texto".into() };
        let fio = std::thread::current().name().unwrap_or("sem nome").to_string();
        registrar("panico", &format!("{msg} em {onde} (fio {fio})\n{}", std::backtrace::Backtrace::force_capture()));
        // pânico em fio auxiliar (recálculo, miniaturas) não derruba a interface: só o principal avisa
        if fio == "main" {
            let en = ingles();
            let titulo = if en { "Fenkra stopped unexpectedly" } else { "O Fenkra parou de forma inesperada" };
            let corpo = if en {
                format!("An internal error occurred and the program must close.\n\n{msg}\n({onde})\n\nThe technical details were saved in:\n{}", texto_log())
            } else {
                format!("Ocorreu um erro interno e o programa precisa fechar.\n\n{msg}\n({onde})\n\nOs detalhes técnicos foram gravados em:\n{}", texto_log())
            };
            caixa(titulo, &corpo);
        }
        anterior(info);
    }));
}

/// Um jeito de pedir o adaptador gráfico.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Modo {
    Padrao,
    Vulkan,
    OpenGl,
    Seguro,
}

impl Modo {
    pub fn nome(self) -> &'static str {
        match self {
            Modo::Padrao => "padrão",
            Modo::Vulkan => "Vulkan",
            Modo::OpenGl => "OpenGL",
            Modo::Seguro => "seguro (software)",
        }
    }

    /// Ordem das tentativas. Quem forçou um backend com `WGPU_BACKEND` não é contrariado: uma só.
    pub fn tentativas(seguro: bool, backend_forcado: bool) -> Vec<Modo> {
        if seguro {
            vec![Modo::Seguro]
        } else if backend_forcado {
            vec![Modo::Padrao]
        } else {
            vec![Modo::Padrao, Modo::Vulkan, Modo::OpenGl, Modo::Seguro]
        }
    }
}

/// Aplica o modo ao pedido de adaptador (o descritor de dispositivo fica como `main.rs` definiu).
pub fn aplicar_modo(cfg: &mut egui_wgpu::WgpuConfiguration, modo: Modo) {
    let egui_wgpu::WgpuSetup::CreateNew(novo) = &mut cfg.wgpu_setup else { return };
    match modo {
        Modo::Padrao => {}
        Modo::Vulkan => novo.instance_descriptor.backends = wgpu::Backends::VULKAN,
        Modo::OpenGl => novo.instance_descriptor.backends = wgpu::Backends::GL,
        Modo::Seguro => {
            novo.instance_descriptor.backends = wgpu::Backends::all();
            // prefere o adaptador por software; sem ele, o primeiro que houver
            novo.native_adapter_selector = Some(std::sync::Arc::new(|lista, _| {
                lista
                    .iter()
                    .find(|a| a.get_info().device_type == wgpu::DeviceType::Cpu)
                    .or_else(|| lista.first())
                    .cloned()
                    .ok_or_else(|| "nenhum adaptador gráfico encontrado".to_string())
            }));
        }
    }
}

/// Descreve o adaptador escolhido no registro. Devolve verdadeiro se for de software (lento).
pub fn registrar_adaptador(rs: &egui_wgpu::RenderState, modo: Modo) -> bool {
    let i = rs.adapter.get_info();
    registrar("adaptador", &format!("{} | backend {:?} | tipo {:?} | modo {}", i.name, i.backend, i.device_type, modo.nome()));
    i.device_type == wgpu::DeviceType::Cpu
}

/// Mensagem final de uma partida que não conseguiu abrir nenhuma janela.
pub fn falha_de_gpu(erros: &[(Modo, String)], backend_forcado: Option<String>) {
    let en = ingles();
    let mut detalhe = String::new();
    for (m, e) in erros {
        detalhe.push_str(&format!("- {}: {e}\n", m.nome()));
        registrar("falha-gpu", &format!("modo {}: {e}", m.nome()));
    }
    let dica = match (&backend_forcado, en) {
        (Some(b), false) => format!("A variável WGPU_BACKEND está definida como \"{b}\". Remova-a ou use um valor válido (vulkan, dx12, gl).\n\n"),
        (Some(b), true) => format!("The WGPU_BACKEND variable is set to \"{b}\". Remove it or use a valid value (vulkan, dx12, gl).\n\n"),
        (None, false) => "Atualize o driver da placa de vídeo e tente de novo; ou abra com fenkra.exe --seguro.\n\n".into(),
        (None, true) => "Update the graphics driver and try again; or start with fenkra.exe --seguro.\n\n".into(),
    };
    let (titulo, corpo) = if en {
        ("Fenkra could not start the graphics", format!("No graphics adapter could be started.\n\n{dica}Attempts:\n{detalhe}\nDetails were saved in:\n{}", texto_log()))
    } else {
        ("O Fenkra não conseguiu iniciar os gráficos", format!("Nenhum adaptador gráfico pôde ser iniciado.\n\n{dica}Tentativas:\n{detalhe}\nOs detalhes foram gravados em:\n{}", texto_log()))
    };
    caixa(titulo, &corpo);
}

/// `--diagnostico`: lista os adaptadores no registro e mostra um resumo, sem abrir a janela.
pub fn diagnostico() {
    let inst = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adaptadores = pollster_block(inst.enumerate_adapters(wgpu::Backends::all()));
    let mut resumo = String::new();
    for a in &adaptadores {
        let i = a.get_info();
        resumo.push_str(&format!("{} | {:?} | {:?}\n", i.name, i.backend, i.device_type));
    }
    if resumo.is_empty() {
        resumo.push_str("(nenhum adaptador)\n");
    }
    registrar("diagnostico", &resumo);
    let en = ingles();
    let rfd_titulo = if en { "Fenkra graphics diagnosis" } else { "Diagnóstico gráfico do Fenkra" };
    let _ = rfd::MessageDialog::new().set_level(rfd::MessageLevel::Info).set_title(rfd_titulo).set_description(format!("{resumo}\n{}", texto_log())).set_buttons(rfd::MessageButtons::Ok).show();
}

fn pollster_block<F: std::future::Future>(f: F) -> F::Output {
    // espera simples sem dependência extra: o futuro de enumeração é imediato no desktop
    use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
    fn clone(_: *const ()) -> RawWaker {
        RawWaker::new(std::ptr::null(), &VT)
    }
    fn nada(_: *const ()) {}
    static VT: RawWakerVTable = RawWakerVTable::new(clone, nada, nada, nada);
    let waker = unsafe { Waker::from_raw(RawWaker::new(std::ptr::null(), &VT)) };
    let mut cx = Context::from_waker(&waker);
    let mut f = Box::pin(f);
    loop {
        if let Poll::Ready(v) = f.as_mut().poll(&mut cx) {
            return v;
        }
        std::thread::yield_now();
    }
}

/// Começa o registro da partida.
pub fn iniciar() {
    girar_log();
    registrar("inicio", &format!("Fenkra {} | args {:?} | WGPU_BACKEND {:?}", env!("CARGO_PKG_VERSION"), std::env::args().skip(1).collect::<Vec<_>>(), std::env::var("WGPU_BACKEND").ok()));
    instalar_gancho();
    medir_quadros_se_pedido();
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn tentativas_respeitam_o_backend_forcado() {
        assert_eq!(Modo::tentativas(false, true), vec![Modo::Padrao]);
        assert_eq!(Modo::tentativas(true, false), vec![Modo::Seguro]);
        assert_eq!(Modo::tentativas(false, false).len(), 4);
    }

    #[test]
    fn contador_so_cresce_quando_chamado() {
        let a = quadros();
        contar_quadro();
        assert!(quadros() > a);
    }
}
