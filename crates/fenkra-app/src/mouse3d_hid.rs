// SPDX-License-Identifier: LGPL-2.1-or-later
//! Leitura do mouse 3D pelo HID genérico do sistema, sem driver do fabricante (B322).
//!
//! No Windows: lista os dispositivos HID, fica com os que se declaram controlador multieixo (página
//! de uso 0x01, uso 0x08), abre cada um só para leitura e lê os relatórios numa thread própria. Os
//! bytes passam por `mouse3d::decodificar_relatorio` e chegam à tela por um canal. Em outros sistemas
//! ou sem aparelho, `iniciar` devolve `None` e nada muda. Esta camada só foi compilada: o teste com
//! aparelho real é do usuário (ver `docs/pesquisa/mouse-3d.md`).

use crate::mouse3d::Evento;
use std::sync::mpsc::Receiver;

/// Procura um aparelho de seis eixos e começa a ler em segundo plano. `None` quando não há aparelho
/// ou o sistema não é suportado: o aplicativo segue normalmente.
#[cfg(not(windows))]
pub fn iniciar(_ctx: eframe::egui::Context) -> Option<Receiver<Evento>> {
    None
}

#[cfg(windows)]
pub fn iniciar(ctx: eframe::egui::Context) -> Option<Receiver<Evento>> {
    win::iniciar(ctx)
}

#[cfg(windows)]
mod win {
    use super::*;
    use crate::mouse3d::decodificar_relatorio;
    use windows_sys::Win32::Devices::DeviceAndDriverInstallation::{
        SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInterfaces, SetupDiGetClassDevsW, SetupDiGetDeviceInterfaceDetailW, DIGCF_DEVICEINTERFACE, DIGCF_PRESENT, SP_DEVICE_INTERFACE_DATA,
        SP_DEVICE_INTERFACE_DETAIL_DATA_W,
    };
    use windows_sys::Win32::Devices::HumanInterfaceDevice::{HidD_FreePreparsedData, HidD_GetHidGuid, HidD_GetPreparsedData, HidP_GetCaps, HIDP_CAPS};
    use windows_sys::Win32::Foundation::{CloseHandle, GENERIC_READ};
    use windows_sys::Win32::Storage::FileSystem::{CreateFileW, ReadFile, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING};

    /// Handle do arquivo do dispositivo, enviável para a thread de leitura.
    struct Alca(isize);
    unsafe impl Send for Alca {}

    /// Caminhos (UTF-16, terminados em zero) de todos os dispositivos HID presentes.
    fn caminhos() -> Vec<Vec<u16>> {
        let mut saida = Vec::new();
        unsafe {
            let mut guid = std::mem::zeroed();
            HidD_GetHidGuid(&mut guid);
            let info = SetupDiGetClassDevsW(&guid, std::ptr::null(), std::ptr::null_mut(), DIGCF_PRESENT | DIGCF_DEVICEINTERFACE);
            if info as isize == -1 || info as isize == 0 {
                return saida;
            }
            let mut i = 0u32;
            loop {
                let mut d: SP_DEVICE_INTERFACE_DATA = std::mem::zeroed();
                d.cbSize = std::mem::size_of::<SP_DEVICE_INTERFACE_DATA>() as u32;
                if SetupDiEnumDeviceInterfaces(info, std::ptr::null(), &guid, i, &mut d) == 0 {
                    break;
                }
                i += 1;
                let mut precisa = 0u32;
                SetupDiGetDeviceInterfaceDetailW(info, &d, std::ptr::null_mut(), 0, &mut precisa, std::ptr::null_mut());
                if precisa == 0 {
                    continue;
                }
                // buffer de u64 para o alinhamento da estrutura de tamanho variável
                let mut buf = vec![0u64; (precisa as usize).div_ceil(8)];
                let det = buf.as_mut_ptr() as *mut SP_DEVICE_INTERFACE_DETAIL_DATA_W;
                (*det).cbSize = if cfg!(target_pointer_width = "64") { 8 } else { 6 };
                if SetupDiGetDeviceInterfaceDetailW(info, &d, det, precisa, std::ptr::null_mut(), std::ptr::null_mut()) == 0 {
                    continue;
                }
                let inicio = std::ptr::addr_of!((*det).DevicePath) as *const u16;
                let mut n = 0;
                while *inicio.add(n) != 0 && n < 2048 {
                    n += 1;
                }
                let mut caminho: Vec<u16> = std::slice::from_raw_parts(inicio, n).to_vec();
                caminho.push(0);
                saida.push(caminho);
            }
            SetupDiDestroyDeviceInfoList(info);
        }
        saida
    }

    /// Abre o dispositivo para leitura; devolve o handle e o tamanho do relatório de entrada se ele
    /// se declara controlador multieixo (página 0x01, uso 0x08).
    fn abrir_se_multieixo(caminho: &[u16]) -> Option<(Alca, usize)> {
        unsafe {
            let h = CreateFileW(caminho.as_ptr(), GENERIC_READ, FILE_SHARE_READ | FILE_SHARE_WRITE, std::ptr::null(), OPEN_EXISTING, 0, std::ptr::null_mut());
            if h as isize == -1 || h as isize == 0 {
                return None;
            }
            let mut pre = std::mem::zeroed();
            if !HidD_GetPreparsedData(h, &mut pre) {
                CloseHandle(h);
                return None;
            }
            let mut caps: HIDP_CAPS = std::mem::zeroed();
            let ok = HidP_GetCaps(pre, &mut caps) >= 0;
            HidD_FreePreparsedData(pre);
            if ok && caps.UsagePage == 0x01 && caps.Usage == 0x08 && caps.InputReportByteLength > 0 {
                Some((Alca(h as isize), caps.InputReportByteLength as usize))
            } else {
                CloseHandle(h);
                None
            }
        }
    }

    pub fn iniciar(ctx: eframe::egui::Context) -> Option<Receiver<Evento>> {
        let aparelhos: Vec<(Alca, usize)> = caminhos().iter().filter_map(|c| abrir_se_multieixo(c)).collect();
        if aparelhos.is_empty() {
            return None;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        for (alca, tamanho) in aparelhos {
            let (tx, ctx) = (tx.clone(), ctx.clone());
            let _ = std::thread::Builder::new().name("mouse3d".into()).spawn(move || {
                let alca = alca;
                let mut buf = vec![0u8; tamanho.max(8)];
                loop {
                    let mut lidos = 0u32;
                    // leitura bloqueante: volta a cada relatório; erro (aparelho removido) encerra
                    let ok = unsafe { ReadFile(alca.0 as _, buf.as_mut_ptr() as _, buf.len() as u32, &mut lidos, std::ptr::null_mut()) };
                    if ok == 0 || lidos == 0 {
                        break;
                    }
                    if let Some(e) = decodificar_relatorio(&buf[..lidos as usize]) {
                        if tx.send(e).is_err() {
                            break;
                        }
                        ctx.request_repaint();
                    }
                }
                unsafe { CloseHandle(alca.0 as _) };
            });
        }
        Some(rx)
    }
}
