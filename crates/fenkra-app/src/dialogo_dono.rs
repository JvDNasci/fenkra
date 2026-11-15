// SPDX-License-Identifier: LGPL-2.1-or-later
//! B476: o diálogo de arquivo nasce preso à janela principal.
//!
//! Sem dona, o diálogo nativo do Windows pode abrir atrás da janela do Fenkra (aconteceu no Salvar
//! como aberto pela biblioteca) e quem usa só vê a janela travada. Com a janela principal como dona
//! o sistema mantém o diálogo na frente dela e a bloqueia enquanto ele estiver aberto. Fora do
//! Windows nada muda: o diálogo segue sem dona, como antes.

use rfd::FileDialog;

/// Diálogo de arquivo novo, já com a janela principal como dona.
pub fn novo() -> FileDialog {
    com_dono(FileDialog::new())
}

/// Dá ao diálogo a janela principal deste processo como dona, se ela existir.
pub fn com_dono(d: FileDialog) -> FileDialog {
    #[cfg(windows)]
    if let Some(h) = so::janela_principal(std::process::id()) {
        return d.set_parent(&so::Dono(h));
    }
    d
}

#[cfg(windows)]
pub(crate) mod so {
    use raw_window_handle::{DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawWindowHandle, Win32WindowHandle, WindowHandle};
    use std::num::NonZeroIsize;
    use windows_sys::Win32::Foundation::{HWND, LPARAM};
    use windows_sys::Win32::UI::WindowsAndMessaging::{EnumWindows, GetWindow, GetWindowThreadProcessId, IsWindowVisible, GW_OWNER};

    /// Janela do sistema que serve de dona (só o número da janela).
    pub struct Dono(pub isize);

    impl HasWindowHandle for Dono {
        fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
            let n = NonZeroIsize::new(self.0).ok_or(HandleError::Unavailable)?;
            // o número vem do sistema e a janela principal vive mais que o diálogo, que é modal
            Ok(unsafe { WindowHandle::borrow_raw(RawWindowHandle::Win32(Win32WindowHandle::new(n))) })
        }
    }

    impl HasDisplayHandle for Dono {
        fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
            Ok(DisplayHandle::windows())
        }
    }

    struct Busca {
        pid: u32,
        achada: HWND,
    }

    unsafe extern "system" fn cada(h: HWND, l: LPARAM) -> windows_sys::core::BOOL {
        let b = unsafe { &mut *(l as *mut Busca) };
        let mut pid = 0u32;
        unsafe { GetWindowThreadProcessId(h, &mut pid) };
        // a janela principal: do processo, visível e sem dona (os diálogos têm dona)
        if pid == b.pid && unsafe { IsWindowVisible(h) } != 0 && unsafe { GetWindow(h, GW_OWNER) }.is_null() {
            b.achada = h;
            return 0;
        }
        1
    }

    /// Janela principal do processo `pid`, se houver.
    pub fn janela_principal(pid: u32) -> Option<isize> {
        let mut b = Busca { pid, achada: std::ptr::null_mut() };
        unsafe { EnumWindows(Some(cada), &mut b as *mut Busca as LPARAM) };
        (!b.achada.is_null()).then_some(b.achada as isize)
    }
}

#[cfg(all(test, windows))]
mod testes {
    use super::so::*;
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    /// A dona entrega ao diálogo exatamente a janela que recebeu.
    #[test]
    fn a_dona_entrega_a_janela_recebida() {
        let d = Dono(0x1234);
        match d.window_handle().expect("janela").as_raw() {
            RawWindowHandle::Win32(h) => assert_eq!(h.hwnd.get(), 0x1234),
            outra => panic!("esperava Win32, veio {outra:?}"),
        }
        assert!(Dono(0).window_handle().is_err(), "janela nula não vira dona");
    }

    /// Sem janela visível (teste sem interface) não há dona, e o diálogo segue sem ela.
    #[test]
    fn sem_janela_nao_ha_dona_e_o_dialogo_se_monta() {
        assert_eq!(janela_principal(std::process::id()), None);
        let _ = super::novo();
    }
}
