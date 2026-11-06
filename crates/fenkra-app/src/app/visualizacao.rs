// SPDX-License-Identifier: LGPL-2.1-or-later
//! Exportar visualização (B181, B338): grava o pacote `.fkv` ou a página autocontida que o abre, a
//! partir da peça, da montagem ou do desenho abertos. O navegador não roda o kernel, então é este
//! arquivo, gerado aqui, que vai para a web. A geração não bloqueia nada de lá: o documento é só lido.

use super::*;
use fenkra_pacote::{de_desenho, de_montagem, de_peca, pagina_html, Opcoes, Pacote};

/// Fração de triângulos que a visualização simplificada mantém.
const SIMPLIFICADA: f64 = 0.3;

impl App {
    fn pacote_do_que_esta_aberto(&self, o: &Opcoes) -> Result<(Pacote, String), String> {
        if self.settings.bench == crate::settings::Bench::Desenho {
            if let Some(d) = &self.dwg {
                let pasta = d.path.as_ref().and_then(|p| p.parent()).ok_or_else(|| tr("Salve o desenho antes de exportar: as peças das vistas são lidas dos arquivos dele.").to_string())?;
                return de_desenho(&d.d, pasta, o).map(|p| (p, d.d.name.clone()));
            }
        }
        if self.in_asm() {
            let s = self.asm.as_ref().ok_or("sem montagem")?;
            let pasta = s.file.as_ref().and_then(|p| p.parent()).ok_or_else(|| tr("Salve a montagem antes de exportar: as peças são lidas dos arquivos dela.").to_string())?;
            return de_montagem(&s.asm, pasta, o).map(|p| (p, s.asm.name.clone()));
        }
        de_peca(&self.open.doc, o).map(|p| (p, self.open.doc.name.clone()))
    }

    /// `simplificada` ligada decima a malha e tira os pontos de captura exatos: protege a geometria.
    pub(super) fn exportar_visualizacao(&mut self, simplificada: bool) {
        let o = if simplificada { Opcoes { decimar: Some(SIMPLIFICADA), capturas: false, ..Default::default() } } else { Opcoes::default() };
        let (pacote, nome) = match self.pacote_do_que_esta_aberto(&o) {
            Ok(x) => x,
            Err(e) => return self.toast(format!("{}: {e}", tr("Falha ao exportar")), true),
        };
        let Some(caminho) = rfd::FileDialog::new()
            .add_filter(tr("Pacote de visualização"), &["fkv"])
            .add_filter(tr("Página autocontida"), &["html"])
            .set_file_name(format!("{nome}.fkv"))
            .save_file()
        else {
            return;
        };
        let bytes = match pacote.bytes() {
            Ok(b) => b,
            Err(e) => return self.toast(format!("{}: {e}", tr("Falha ao exportar")), true),
        };
        let e_html = caminho.extension().is_some_and(|e| e.eq_ignore_ascii_case("html") || e.eq_ignore_ascii_case("htm"));
        let r = if e_html { std::fs::write(&caminho, pagina_html(Some(&bytes))) } else { std::fs::write(&caminho, &bytes) };
        match r {
            Ok(()) => self.toast(format!("{}: {} ({} KB)", tr("Exportado"), caminho.display(), bytes.len() / 1024), false),
            Err(e) => self.toast(format!("{}: {e}", tr("Falha ao exportar")), true),
        }
    }
}
