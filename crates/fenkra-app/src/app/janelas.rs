// SPDX-License-Identifier: LGPL-2.1-or-later
//! Menu Janela (B440): alternar o documento, abrir outro numa janela nova e arrumar as janelas lado
//! a lado ou em cascata.
//!
//! Ainda não há vários documentos abertos na mesma janela (B363). Enquanto isso, "vários documentos"
//! são várias janelas do Fenkra: cada uma é um processo deste executável, aberto com o arquivo na
//! linha de comando (como o duplo clique, B389). Esta janela guarda os processos que ela mesma abriu
//! e só mexe neles e nela própria: nunca nas janelas de outra sessão do programa.
//!
//! O arranjo é uma conta pura sobre a área de trabalho do monitor (sem a barra de tarefas):
//! - lado a lado: grade com `ceil(sqrt(n))` colunas; cada linha divide a largura entre as janelas
//!   dela, então a última linha, com menos janelas, ocupa a largura toda e nada fica descoberto;
//! - cascata: todas do mesmo tamanho (70% da área), cada uma deslocada um degrau na diagonal; quando
//!   o degrau sairia da área, a escada recomeça do canto.

use super::*;
use std::path::{Path, PathBuf};
use std::process::Child;
use std::sync::Mutex;

/// Janelas abertas por esta (processo filho e arquivo).
static FILHAS: Mutex<Vec<(Child, PathBuf)>> = Mutex::new(Vec::new());

/// Retângulo em pixels da tela: x, y, largura, altura.
pub(crate) type Ret = [i32; 4];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Arranjo {
    LadoALado,
    Cascata,
}

/// Degrau da cascata, em pixels (cabe a barra de título da janela de trás).
const DEGRAU: i32 = 32;

/// Posição de cada uma das `n` janelas dentro de `area`.
pub(crate) fn arranjar(n: usize, area: Ret, modo: Arranjo) -> Vec<Ret> {
    let [x0, y0, w, h] = area;
    if n == 0 || w <= 0 || h <= 0 {
        return vec![];
    }
    match modo {
        Arranjo::LadoALado => {
            let cols = (n as f64).sqrt().ceil() as usize;
            let linhas = n.div_ceil(cols);
            let mut v = Vec::with_capacity(n);
            for l in 0..linhas {
                let nessa = (n - l * cols).min(cols);
                let (ya, yb) = (y0 + h * l as i32 / linhas as i32, y0 + h * (l as i32 + 1) / linhas as i32);
                for c in 0..nessa {
                    let (xa, xb) = (x0 + w * c as i32 / nessa as i32, x0 + w * (c as i32 + 1) / nessa as i32);
                    v.push([xa, ya, xb - xa, yb - ya]);
                }
            }
            v
        }
        Arranjo::Cascata => {
            let (cw, ch) = (w * 7 / 10, h * 7 / 10);
            let degraus = ((w - cw).min(h - ch) / DEGRAU).max(0) as usize + 1;
            (0..n).map(|i| {
                let k = (i % degraus) as i32 * DEGRAU;
                [x0 + k, y0 + k, cw, ch]
            }).collect()
        }
    }
}

/// Tira da lista as janelas que já foram fechadas.
fn vivas() -> Vec<(u32, PathBuf)> {
    let mut f = FILHAS.lock().unwrap_or_else(|e| e.into_inner());
    f.retain_mut(|(c, _)| matches!(c.try_wait(), Ok(None)));
    f.iter().map(|(c, p)| (c.id(), p.clone())).collect()
}

/// Abre `arquivo` numa janela nova deste mesmo executável.
fn abrir_em_nova_janela(arquivo: &Path) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let c = std::process::Command::new(exe).arg(arquivo).spawn().map_err(|e| e.to_string())?;
    FILHAS.lock().unwrap_or_else(|e| e.into_inner()).push((c, arquivo.to_path_buf()));
    Ok(())
}

#[cfg(windows)]
mod so {
    use super::Ret;
    use windows_sys::Win32::Foundation::{HWND, LPARAM, RECT};
    use windows_sys::Win32::UI::WindowsAndMessaging::*;

    struct Busca {
        pid: u32,
        achada: HWND,
    }

    unsafe extern "system" fn cada(h: HWND, l: LPARAM) -> windows_sys::core::BOOL {
        let b = unsafe { &mut *(l as *mut Busca) };
        let mut pid = 0u32;
        unsafe { GetWindowThreadProcessId(h, &mut pid) };
        // a janela principal: do processo, visível e sem dona (diálogos têm dona)
        if pid == b.pid && unsafe { IsWindowVisible(h) } != 0 && unsafe { GetWindow(h, GW_OWNER) }.is_null() {
            b.achada = h;
            return 0;
        }
        1
    }

    /// Janela principal do processo `pid`.
    pub fn janela_do(pid: u32) -> Option<HWND> {
        let mut b = Busca { pid, achada: std::ptr::null_mut() };
        unsafe { EnumWindows(Some(cada), &mut b as *mut Busca as LPARAM) };
        (!b.achada.is_null()).then_some(b.achada)
    }

    /// Área de trabalho do monitor principal, sem a barra de tarefas.
    pub fn area_de_trabalho() -> Option<Ret> {
        let mut r = RECT { left: 0, top: 0, right: 0, bottom: 0 };
        let ok = unsafe { SystemParametersInfoW(SPI_GETWORKAREA, 0, &mut r as *mut RECT as *mut _, 0) };
        (ok != 0 && r.right > r.left && r.bottom > r.top).then_some([r.left, r.top, r.right - r.left, r.bottom - r.top])
    }

    pub fn posicionar(h: HWND, r: Ret) {
        unsafe {
            ShowWindow(h, SW_RESTORE);
            SetWindowPos(h, std::ptr::null_mut(), r[0], r[1], r[2], r[3], SWP_NOZORDER | SWP_NOACTIVATE | SWP_SHOWWINDOW);
        }
    }

    pub fn trazer_para_frente(h: HWND) {
        unsafe {
            ShowWindow(h, SW_RESTORE);
            SetForegroundWindow(h);
        }
    }
}

/// Arruma esta janela e as que ela abriu. Devolve quantas foram arrumadas.
#[cfg(windows)]
fn aplicar_arranjo(modo: Arranjo) -> usize {
    let pids: Vec<u32> = std::iter::once(std::process::id()).chain(vivas().into_iter().map(|(p, _)| p)).collect();
    let janelas: Vec<_> = pids.into_iter().filter_map(so::janela_do).collect();
    let Some(area) = so::area_de_trabalho() else { return 0 };
    for (h, r) in janelas.iter().zip(arranjar(janelas.len(), area, modo)) {
        so::posicionar(*h, r);
    }
    janelas.len()
}

#[cfg(not(windows))]
fn aplicar_arranjo(_: Arranjo) -> usize {
    0
}

#[cfg(windows)]
fn focar(pid: u32) {
    if let Some(h) = so::janela_do(pid) {
        so::trazer_para_frente(h);
    }
}

#[cfg(not(windows))]
fn focar(_: u32) {}

/// Nome curto de um caminho, para o menu.
fn nome_curto(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| p.display().to_string())
}

impl App {
    /// Conteúdo do menu Janela.
    pub(super) fn menu_janela(&mut self, ui: &mut egui::Ui) {
        let atual = self.open.file.clone();
        let nome_atual = atual.as_deref().map(nome_curto).unwrap_or_else(|| self.open.doc.name.clone());
        ui.label(RichText::new(format!("{} {nome_atual}", tr("Nesta janela:"))).small().weak());
        let outros: Vec<PathBuf> = self.settings.recent_files.iter().map(PathBuf::from).filter(|p| p.exists() && Some(p) != atual.as_ref()).take(12).collect();
        ui.menu_button(tr("Alternar documento"), |ui| {
            if outros.is_empty() {
                ui.label(RichText::new(tr("Nenhum outro documento recente.")).weak());
            }
            for p in &outros {
                if ui.button(nome_curto(p)).on_hover_text(p.display().to_string()).clicked() {
                    // pergunta antes se a peça desta janela tem alterações por salvar
                    self.com_confirmacao(Acao::AbrirCaminho(p.clone()));
                    ui.close();
                }
            }
        });
        ui.menu_button(tr("Abrir em nova janela"), |ui| {
            let mut lista: Vec<PathBuf> = atual.iter().cloned().collect();
            lista.extend(outros.iter().cloned());
            if lista.is_empty() {
                ui.label(RichText::new(tr("Salve o documento para abri-lo em outra janela.")).weak());
            }
            for p in lista {
                if ui.button(nome_curto(&p)).on_hover_text(p.display().to_string()).clicked() {
                    if let Err(e) = abrir_em_nova_janela(&p) {
                        self.toast(e, true);
                    }
                    ui.close();
                }
            }
        });
        ui.separator();
        let filhas = vivas();
        let varias = !filhas.is_empty();
        let dica = tr("Arruma esta janela e as que foram abertas a partir dela. Janelas de outras sessões do programa não mudam.");
        if ui.add_enabled(varias, egui::Button::new(tr("Lado a lado"))).on_hover_text(dica).on_disabled_hover_text(tr("Abra outro documento em nova janela para arrumar as janelas.")).clicked() {
            aplicar_arranjo(Arranjo::LadoALado);
            ui.close();
        }
        if ui.add_enabled(varias, egui::Button::new(tr("Cascata"))).on_hover_text(dica).on_disabled_hover_text(tr("Abra outro documento em nova janela para arrumar as janelas.")).clicked() {
            aplicar_arranjo(Arranjo::Cascata);
            ui.close();
        }
        if varias {
            ui.separator();
            ui.label(RichText::new(tr("Janelas abertas daqui")).small().weak());
            for (pid, p) in filhas {
                if ui.button(nome_curto(&p)).on_hover_text(tr("Traz esta janela para a frente")).clicked() {
                    focar(pid);
                    ui.close();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(r: Ret) -> i64 {
        r[2] as i64 * r[3] as i64
    }

    fn cruza(a: Ret, b: Ret) -> bool {
        a[0] < b[0] + b[2] && b[0] < a[0] + a[2] && a[1] < b[1] + b[3] && b[1] < a[1] + a[3]
    }

    /// Lado a lado: as janelas cobrem a área inteira sem se sobrepor, para qualquer quantidade.
    #[test]
    fn lado_a_lado_cobre_a_area_sem_sobrepor() {
        let tela = [0, 0, 1920, 1040];
        for n in 1..=9 {
            let v = arranjar(n, tela, Arranjo::LadoALado);
            assert_eq!(v.len(), n);
            assert_eq!(v.iter().map(|r| area(*r)).sum::<i64>(), area(tela), "{n}");
            for i in 0..n {
                assert!(v[i][0] >= 0 && v[i][1] >= 0 && v[i][0] + v[i][2] <= 1920 && v[i][1] + v[i][3] <= 1040);
                for j in i + 1..n {
                    assert!(!cruza(v[i], v[j]), "{n}: {:?} {:?}", v[i], v[j]);
                }
            }
        }
        // duas janelas: uma de cada lado, na altura toda
        assert_eq!(arranjar(2, tela, Arranjo::LadoALado), vec![[0, 0, 960, 1040], [960, 0, 960, 1040]]);
        // a área não precisa começar no canto (barra de tarefas à esquerda)
        assert_eq!(arranjar(1, [60, 0, 1860, 1080], Arranjo::LadoALado), vec![[60, 0, 1860, 1080]]);
    }

    /// Cascata: mesmo tamanho, cada uma um degrau abaixo e à direita, sempre dentro da área.
    #[test]
    fn cascata_desce_em_degraus_dentro_da_area() {
        let tela = [0, 0, 1920, 1040];
        let v = arranjar(30, tela, Arranjo::Cascata);
        assert_eq!(v.len(), 30);
        assert_eq!(v[1][0] - v[0][0], DEGRAU);
        assert_eq!(v[1][1] - v[0][1], DEGRAU);
        for r in &v {
            assert_eq!([r[2], r[3]], [v[0][2], v[0][3]]);
            assert!(r[0] + r[2] <= 1920 && r[1] + r[3] <= 1040, "{r:?}");
        }
        // a escada recomeça do canto quando acaba o espaço
        assert!(v.iter().skip(1).any(|r| r[0] == 0 && r[1] == 0));
        assert!(arranjar(0, tela, Arranjo::Cascata).is_empty());
        assert!(arranjar(3, [0, 0, 0, 0], Arranjo::LadoALado).is_empty());
    }

    #[test]
    fn textos_do_menu_janela_tem_traducao() {
        for t in [
            "Janela",
            "Nesta janela:",
            "Alternar documento",
            "Nenhum outro documento recente.",
            "Abrir em nova janela",
            "Salve o documento para abri-lo em outra janela.",
            "Lado a lado",
            "Cascata",
            "Arruma esta janela e as que foram abertas a partir dela. Janelas de outras sessões do programa não mudam.",
            "Abra outro documento em nova janela para arrumar as janelas.",
            "Janelas abertas daqui",
            "Traz esta janela para a frente",
        ] {
            assert!(crate::i18n::tem_traducao(t), "{t}");
        }
    }
}
