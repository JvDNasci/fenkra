// SPDX-License-Identifier: LGPL-2.1-or-later
//! Exportar a árvore paramétrica (B266) e receber um arquivo com várias peças. A leitura passa pelo
//! painel de importação; aqui ficam só as pontas que precisam da aplicação.

use super::*;
use fenkra_conversor::arvore::{atualizar_pontos, exportar_montagem, exportar_peca, gravar_em_pasta, Arquivo, Conjunto};
use fenkra_conversor::ErroConv;

/// A peça aberta, com os pontos de aresta onde as arestas estão agora.
fn arquivo_da_peca(doc: &Document) -> Arquivo {
    let mut d = doc.clone();
    atualizar_pontos(&mut d);
    exportar_peca(&d).arquivo()
}

fn gravar(arquivo: &Arquivo, caminho: &std::path::Path) -> Result<(), ErroConv> {
    let texto = serde_json::to_string_pretty(arquivo).map_err(|e| ErroConv::Gravacao(e.to_string()))?;
    std::fs::write(caminho, texto).map_err(|e| ErroConv::Gravacao(e.to_string()))
}

impl App {
    fn arquivo_para_exportar(&self) -> Option<Result<(Arquivo, String), ErroConv>> {
        if !self.in_asm() {
            return Some(Ok((arquivo_da_peca(&self.open.doc), self.open.doc.name.clone())));
        }
        let s = self.asm.as_ref()?;
        let pasta = s.file.as_ref().and_then(|p| p.parent())?;
        Some(exportar_montagem(&s.asm, pasta).map(|a| (a, s.asm.name.clone())))
    }

    pub(super) fn exportar_arvore(&mut self) {
        let Some(pronto) = self.arquivo_para_exportar() else {
            self.toast(tr("Salve a montagem antes de exportar: as peças são lidas dos arquivos dela.").to_string(), true);
            return;
        };
        let (arquivo, nome) = match pronto {
            Ok(x) => x,
            Err(e) => return self.toast(format!("Falha ao exportar: {e}"), true),
        };
        let Some(caminho) = rfd::FileDialog::new().add_filter(tr("Árvore paramétrica"), &["fkx"]).set_file_name(format!("{nome}.fkx")).save_file() else { return };
        match gravar(&arquivo, &caminho) {
            Ok(()) if arquivo.fora.is_empty() => self.toast(format!("Exportado: {}", caminho.display()), false),
            Ok(()) => self.toast(tr("Árvore exportada, com itens de fora: {0}").replace("{0}", &arquivo.fora.join("; ")), true),
            Err(e) => self.toast(format!("Falha ao exportar: {e}"), true),
        }
    }

    /// Arquivo com várias peças não cabe em um documento: elas viram arquivos numa pasta escolhida.
    pub(super) fn receber_conjunto(&mut self, conjunto: Conjunto) {
        self.toast(tr("Este arquivo tem várias peças ou uma montagem: escolha uma pasta para elas.").to_string(), false);
        let Some(pasta) = rfd::FileDialog::new().set_title(tr("Pasta para as peças do arquivo")).pick_folder() else { return };
        match gravar_em_pasta(&conjunto, &pasta) {
            Ok(caminho) => self.open_path(&caminho),
            Err(e) => self.toast(e.to_string(), true),
        }
    }
}
