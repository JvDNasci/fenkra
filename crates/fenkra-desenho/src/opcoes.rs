// SPDX-License-Identifier: LGPL-2.1-or-later
//! Opções de exportação (B252): quais folhas, cor ou preto e branco e mapeamento de espessura de
//! linha. Valem para PDF, SVG e DXF: as opções mexem nas primitivas da folha, antes de cada
//! formato escrever o seu arquivo, então os três saem iguais entre si.

use crate::export::{dxf, pdf, svg};
use crate::layout::{Item, Prim};
use crate::{Drawing, V2};
use fenkra_core::Id;

/// Como as cores saem no arquivo.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Cores {
    /// cores do desenho (estilos, vistas e sombreado)
    #[default]
    Colorido,
    /// cada cor vira o cinza de mesma luminosidade
    Cinza,
    /// linhas e textos pretos; preenchimentos escuros viram pretos e os claros somem
    PretoBranco,
}

impl Cores {
    pub const ALL: [Cores; 3] = [Cores::Colorido, Cores::Cinza, Cores::PretoBranco];
    pub fn label(self) -> &'static str {
        match self {
            Cores::Colorido => "Colorido",
            Cores::Cinza => "Tons de cinza",
            Cores::PretoBranco => "Preto e branco",
        }
    }
}

/// Opções do diálogo de exportação.
#[derive(Clone, PartialEq, Debug)]
pub struct Opcoes {
    /// folhas exportadas; vazio = todas
    pub folhas: Vec<Id>,
    pub cores: Cores,
    /// multiplicador das espessuras de linha (1 = como no desenho)
    pub fator_espessura: f64,
    /// espessura mínima (mm) depois do multiplicador; 0 = sem mínimo
    pub espessura_minima: f64,
}

impl Default for Opcoes {
    fn default() -> Self {
        Opcoes { folhas: vec![], cores: Cores::Colorido, fator_espessura: 1., espessura_minima: 0. }
    }
}

impl Opcoes {
    /// Folhas do desenho que entram no arquivo, na ordem do desenho.
    pub fn folhas_de<'a>(&self, d: &'a Drawing) -> Vec<&'a crate::Sheet> {
        d.sheets.iter().filter(|s| self.folhas.is_empty() || self.folhas.contains(&s.id)).collect()
    }

    /// Aplica cor e mapeamento de espessura às primitivas de uma folha.
    pub fn aplicar(&self, items: &[Item]) -> Vec<Item> {
        let lum = |c: [u8; 3]| (0.299 * c[0] as f64 + 0.587 * c[1] as f64 + 0.114 * c[2] as f64) / 255.;
        let mut out = Vec::with_capacity(items.len());
        for it in items {
            let mut n = it.clone();
            match (&it.prim, self.cores) {
                (_, Cores::Colorido) => {}
                (Prim::Fill { .. }, Cores::PretoBranco) => {
                    // sem cor própria o preenchimento já é preto (pontas de seta); o sombreado claro some
                    if it.color.is_some_and(|c| lum(c) > 0.6) {
                        continue;
                    }
                    n.color = None;
                }
                (_, Cores::PretoBranco) => n.color = None,
                (_, Cores::Cinza) => {
                    n.color = it.color.map(|c| {
                        let g = (lum(c) * 255.).round() as u8;
                        [g, g, g]
                    })
                }
            }
            if let Prim::Line { style, .. } = &it.prim {
                if (self.fator_espessura - 1.).abs() > 1e-9 || self.espessura_minima > 0. {
                    let w = it.width.unwrap_or(style.width()) * self.fator_espessura;
                    n.width = Some(w.max(self.espessura_minima));
                }
            }
            out.push(n);
        }
        out
    }
}

/// Nome do arquivo de uma folha quando há mais de uma em SVG ou DXF: `base-<folha>.ext`.
pub fn nome_da_folha(path: &str, folha: &str) -> String {
    let limpo: String = folha.chars().map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect();
    match path.rfind('.') {
        Some(i) => format!("{}-{limpo}{}", &path[..i], &path[i..]),
        None => format!("{path}-{limpo}"),
    }
}

/// Exporta com as opções: PDF com as folhas escolhidas em páginas; SVG e DXF um arquivo por folha
/// (o nome da folha entra no nome do arquivo quando são várias). Devolve os arquivos gravados.
pub fn salvar_com(d: &Drawing, m: &mut crate::source::Models, path: &str, o: &Opcoes) -> Result<Vec<String>, String> {
    let lower = path.to_lowercase();
    if !(lower.ends_with(".pdf") || lower.ends_with(".svg") || lower.ends_with(".dxf")) {
        return Err("formato não suportado (use .pdf, .svg ou .dxf)".into());
    }
    let folhas = o.folhas_de(d);
    if folhas.is_empty() {
        return Err("nenhuma folha escolhida".into());
    }
    let mut feitos = vec![];
    if lower.ends_with(".pdf") {
        let pages: Vec<(V2, Vec<Item>)> = folhas.iter().map(|s| (s.size(), o.aplicar(&crate::layout::sheet(d, s.id, m)))).collect();
        std::fs::write(path, pdf(&pages)).map_err(|e| e.to_string())?;
        feitos.push(path.to_string());
    } else {
        let svg_fmt = lower.ends_with(".svg");
        let varias = folhas.len() > 1;
        for s in folhas {
            let items = o.aplicar(&crate::layout::sheet(d, s.id, m));
            let alvo = if varias { nome_da_folha(path, &s.name) } else { path.to_string() };
            let txt = if svg_fmt { svg(&items, s.size()) } else { dxf(&items) };
            std::fs::write(&alvo, txt).map_err(|e| e.to_string())?;
            feitos.push(alvo);
        }
    }
    Ok(feitos)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{Align, Style};
    use crate::Format;

    fn linha(color: Option<[u8; 3]>) -> Item {
        Item { owner: None, prim: Prim::Line { pts: vec![[0., 0.], [10., 0.]], style: Style::Visible }, color, width: None, bold: false, alert: false }
    }

    #[test]
    fn preto_e_branco_zera_cores_e_descarta_sombreado_claro() {
        let escuro = Item { owner: None, prim: Prim::Fill { pts: vec![[0., 0.], [1., 0.], [1., 1.]] }, color: Some([20, 20, 20]), width: None, bold: false, alert: false };
        let claro = Item { color: Some([230, 230, 230]), ..escuro.clone() };
        let texto = Item { owner: None, prim: Prim::Text { pos: [0., 0.], text: "x".into(), size: 3., angle: 0., align: Align::Left }, color: Some([200, 0, 0]), width: None, bold: false, alert: false };
        let o = Opcoes { cores: Cores::PretoBranco, ..Default::default() };
        let r = o.aplicar(&[linha(Some([255, 0, 0])), escuro, claro, texto]);
        assert_eq!(r.len(), 3, "o sombreado claro some");
        assert!(r.iter().all(|i| i.color.is_none()), "tudo preto");
        // cinza: a cor vira o cinza de mesma luminosidade
        let g = Opcoes { cores: Cores::Cinza, ..Default::default() }.aplicar(&[linha(Some([255, 0, 0])), linha(None)]);
        assert_eq!(g[0].color, Some([76, 76, 76]));
        assert_eq!(g[1].color, None);
        // colorido não mexe em nada
        let c = Opcoes::default().aplicar(&[linha(Some([255, 0, 0]))]);
        assert_eq!(c[0], linha(Some([255, 0, 0])));
    }

    #[test]
    fn mapeamento_de_espessura_multiplica_e_respeita_o_minimo() {
        let o = Opcoes { fator_espessura: 2., ..Default::default() };
        assert_eq!(o.aplicar(&[linha(None)])[0].width, Some(1.0));
        let o = Opcoes { fator_espessura: 0.5, espessura_minima: 0.3, ..Default::default() };
        assert_eq!(o.aplicar(&[linha(None)])[0].width, Some(0.3));
        // espessura própria também é mapeada
        let propria = Item { width: Some(0.8), ..linha(None) };
        assert_eq!(Opcoes { fator_espessura: 0.5, ..Default::default() }.aplicar(&[propria])[0].width, Some(0.4));
        // sem opções a espessura fica como estava
        assert_eq!(Opcoes::default().aplicar(&[linha(None)])[0].width, None);
    }

    #[test]
    fn exporta_so_as_folhas_escolhidas_e_nomeia_os_arquivos() {
        let mut d = Drawing::new("Dois", Format::A4);
        let s1 = d.sheets[0].id;
        let s2 = d.add_sheet(Format::A3);
        let dir = std::env::temp_dir().join("fenkra_desenho_export_opcoes");
        std::fs::create_dir_all(&dir).unwrap();
        let base = dir.join("saida.svg").to_string_lossy().to_string();
        let mut m = crate::source::Models::new(None);
        // todas: um SVG por folha com o nome da folha
        let f = salvar_com(&d, &mut m, &base, &Opcoes::default()).unwrap();
        assert_eq!(f.len(), 2);
        assert!(f[0].ends_with("saida-Folha_1.svg") && f[1].ends_with("saida-Folha_2.svg"), "{f:?}");
        // só a segunda: arquivo com o nome pedido
        let f = salvar_com(&d, &mut m, &base, &Opcoes { folhas: vec![s2], ..Default::default() }).unwrap();
        assert_eq!(f, vec![base.clone()]);
        assert!(std::fs::read_to_string(&base).unwrap().contains("420"), "A3 tem 420 mm");
        // PDF com uma folha: uma página
        let pdf_path = dir.join("saida.pdf").to_string_lossy().to_string();
        salvar_com(&d, &mut m, &pdf_path, &Opcoes { folhas: vec![s1], ..Default::default() }).unwrap();
        let bytes = std::fs::read(&pdf_path).unwrap();
        assert!(String::from_utf8_lossy(&bytes).contains("/Count 1"));
        assert!(salvar_com(&d, &mut m, &pdf_path, &Opcoes { folhas: vec![Id(999)], ..Default::default() }).is_err());
        assert!(salvar_com(&d, &mut m, "x.png", &Opcoes::default()).is_err());
    }
}
