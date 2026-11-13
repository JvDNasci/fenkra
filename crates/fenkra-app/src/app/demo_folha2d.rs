// SPDX-License-Identifier: LGPL-2.1-or-later
//! Demonstração do B211 para captura sem mouse.
//!
//! * `FENKRA_DEMO_FOLHA2D=<arquivo .dxf/.dwg/.svg>`: abre o arquivo como desenho, como o Abrir do desenho
//!   técnico. Com `1`, grava antes um DXF de prova (folha A3 com contorno, furo de centro, arco oculto, eixo
//!   fino e texto) e abre esse.

use super::App;
use fenkra_core::sketch::Curve;
use fenkra_core::Id;
use fenkra_desenho::esboco::{DonoEsboco, TipoLinha};
use fenkra_desenho::{Drawing, Format, Note, NoteKind};

/// DXF de prova: o mesmo desenho do teste de ida e volta, gravado pela exportação da folha.
fn dxf_de_prova() -> String {
    let mut d = Drawing::new("Prova", Format::A3);
    let folha = d.sheets[0].id;
    let id = d.add_esboco(DonoEsboco::Folha(folha));
    let mut n = 10_000;
    let mut al = move || {
        n += 1;
        Id(n)
    };
    if let Some(e) = d.esboco_mut(id) {
        e.cotas = false;
        e.dados.add_rect(&mut al, [60., 50.], [180., 130.], false);
        let c = e.dados.add_point(&mut al, 120., 90.);
        let furo = e.dados.add_curve(&mut al, Curve::Circle { c, r: 15. }, false);
        let (s, f, o) = (e.dados.add_point(&mut al, 220., 90.), e.dados.add_point(&mut al, 200., 110.), e.dados.add_point(&mut al, 200., 90.));
        let arco = e.dados.add_curve(&mut al, Curve::Arc { c: o, s, e: f }, false);
        let (a, b) = (e.dados.add_point(&mut al, 40., 90.), e.dados.add_point(&mut al, 250., 90.));
        let eixo = e.dados.add_curve(&mut al, Curve::Line { a, b }, false);
        e.definir_tipo(furo, TipoLinha::Centro);
        e.definir_tipo(arco, TipoLinha::Oculta);
        e.definir_tipo(eixo, TipoLinha::Fina);
    }
    let nota = d.alloc();
    d.notes.push(Note { id: nota, sheet: folha, kind: NoteKind::Text, pos: [70., 140.], text: "CHAPA 3 mm".into(), size: 5., ..Default::default() });
    let mut m = fenkra_desenho::source::Models::new(None);
    fenkra_desenho::export::dxf(&fenkra_desenho::layout::sheet(&d, folha, &mut m))
}

impl App {
    pub(super) fn demo_folha2d(&mut self) {
        let Ok(valor) = std::env::var("FENKRA_DEMO_FOLHA2D") else { return };
        let caminho = if valor == "1" {
            let p = std::env::temp_dir().join("fenkra_demo_folha2d.dxf");
            let _ = std::fs::write(&p, dxf_de_prova());
            p
        } else {
            std::path::PathBuf::from(valor)
        };
        match fenkra_conversor::folha::abrir_como_desenho(&caminho) {
            Ok((d, r)) => {
                self.dwg = Some(crate::dwg_ui::DwgState::new(d, None));
                self.toast(format!("{}: {} {}, {} {}", crate::i18n::tr("Desenho aberto"), r.esbocos, crate::i18n::tr("esboços"), r.textos, crate::i18n::tr("textos")), false);
            }
            Err(e) => self.toast(format!("{}: {e}", crate::i18n::tr("Não abriu o desenho")), true),
        }
        self.settings.bench = crate::settings::Bench::Desenho;
        self.started = true;
    }
}
