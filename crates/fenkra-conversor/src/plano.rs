// SPDX-License-Identifier: LGPL-2.1-or-later
//! Formatos 2D de esboço (B211): DXF e SVG. Os dois passam pelo mesmo [`Desenho`] do crate DXF, então a
//! volta para o documento é uma só, em [`crate::dxf`], e cada formato só traduz a sintaxe.
//!
//! Na gravação, cada esboço vira uma camada com o nome dele, e a linha de construção fica de fora, porque
//! serve a quem desenha e não ao contorno. Os esboços saem nas coordenadas deles: dois esboços em planos
//! diferentes caem um sobre o outro, e quem precisa separá-los usa as camadas.

mod svg;

pub(crate) use svg::{gravar as gravar_svg, ler as ler_svg, parece as parece_svg};

use crate::{Capacidades, ErroConv, Formato, OpcoesExportar, OpcoesImportar, Registro};
use fenkra_core::sketch::{Curve, SketchData};
use fenkra_core::{Document, FeatureKind, OriginPlane, PlaneRef, Sketch};
use fenkra_dxf::{Camada, Desenho, Entidade, Item, Unidade};
use std::f64::consts::TAU;
use std::path::Path;

fn curvas(sk: &SketchData, camada: &str, saida: &mut Vec<Item>) {
    let a = |i| sk.pos(i);
    for c in sk.curves.iter().filter(|c| !c.construction) {
        let e = match &c.geom {
            Curve::Line { a: p, b: q } => Entidade::Linha(a(*p), a(*q)),
            Curve::Circle { c, r } => Entidade::Circulo { centro: a(*c), raio: *r },
            Curve::Arc { c, s, e } => {
                let (o, s, e) = (a(*c), a(*s), a(*e));
                let angulo = |p: [f64; 2]| (p[1] - o[1]).atan2(p[0] - o[0]).to_degrees();
                Entidade::Arco { centro: o, raio: (s[0] - o[0]).hypot(s[1] - o[1]), ini: angulo(s), fim: angulo(e) }
            }
            Curve::Point { p } => Entidade::Ponto(a(*p)),
            Curve::Spline { pts } => Entidade::Spline { grau: 3, controle: vec![], nos: vec![], pesos: vec![], ajuste: pts.iter().map(|p| a(*p)).collect(), fechada: false },
            Curve::Ellipse { c, major, minor, t0, t1, .. } => Entidade::Elipse { centro: a(*c), maior: *major, razao: minor / major[0].hypot(major[1]).max(1e-12), t0: *t0, t1: *t1 },
        };
        saida.push(Item::novo(camada, e));
    }
}

/// Os esboços do documento, um por camada, em milímetros.
pub(crate) fn desenho_do_documento(doc: &Document) -> Result<Desenho, ErroConv> {
    let mut d = Desenho { unidade: Unidade::Milimetro, ..Default::default() };
    for f in &doc.features {
        let FeatureKind::Sketch(s) = &f.kind else { continue };
        d.camadas.push(Camada::nova(&f.name, 7, "Continuous"));
        curvas(&s.data, &f.name, &mut d.itens);
    }
    if d.itens.is_empty() {
        return Err(ErroConv::Gravacao("o documento não tem esboço para gravar em 2D".into()));
    }
    Ok(d)
}

/// Grava os esboços do documento no formato 2D pedido.
pub(crate) fn gravar(doc: &Document, formato: Formato, caminho: &Path) -> Result<(), ErroConv> {
    let d = desenho_do_documento(doc)?;
    if formato == Formato::Dwg {
        return crate::dwg::de_dxf(&fenkra_dxf::gravar(&d), caminho);
    }
    let texto = if formato == Formato::Svg { gravar_svg(&d) } else { fenkra_dxf::gravar(&d) };
    std::fs::write(caminho, texto).map_err(|e| ErroConv::Gravacao(format!("não foi possível gravar '{}': {e}", caminho.display())))
}

/// Grava um círculo numa camada e olha o que voltou.
pub(crate) fn sondar(registro: &Registro, formato: Formato, pasta: &Path) -> Result<Capacidades, ErroConv> {
    const RAIO: f64 = 12.5;
    let mut d = Document::new("Sonda");
    let mut sk = SketchData::default();
    let mut al = || d.alloc();
    let c = sk.add_point(&mut al, 3., 4.);
    sk.add_curve(&mut al, Curve::Circle { c, r: RAIO }, false);
    let id = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data: sk }));
    if let Some(f) = d.features.iter_mut().find(|f| f.id == id) {
        f.name = "Sonda".into();
    }
    let caminho = pasta.join(format!("sonda.{}", formato.extensoes()[0]));
    let gravou = registro.gravar_documento(&d, &caminho, &OpcoesExportar::default()).is_ok() && caminho.is_file();
    let Ok(lido) = registro.importar(&caminho, &OpcoesImportar::default()) else { return Ok(Capacidades { gravar: gravou, ..Default::default() }) };
    let esboco = lido.doc.features.iter().find_map(|f| match &f.kind {
        FeatureKind::Sketch(s) => Some((f.name.as_str(), &s.data)),
        _ => None,
    });
    let exata = esboco.is_some_and(|(_, s)| s.curves.iter().any(|c| matches!(c.geom, Curve::Circle { r, .. } if (r - RAIO).abs() < 1e-9)));
    let unidade = lido.relatorio.avisos.iter().all(|a| !a.modelo.contains("não declara"));
    Ok(Capacidades { ler: true, gravar: gravou, exata, camadas: esboco.is_some_and(|(n, _)| n == "Sonda"), unidade, ..Default::default() })
}

/// Pontos de um arco de elipse, para quando uma transformação estica um círculo.
pub(crate) fn amostras(n: usize, volta: impl Fn(f64) -> [f64; 2]) -> Vec<[f64; 2]> {
    (0..n).map(|i| volta(i as f64 / n as f64 * TAU)).collect()
}
