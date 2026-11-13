// SPDX-License-Identifier: LGPL-2.1-or-later
//! Formatos que o conversor conhece, reconhecidos pelo conteúdo. A extensão é só uma dica: um STEP
//! renomeado continua sendo STEP, e o relatório avisa da divergência.

use crate::ErroConv;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Formato {
    Step,
    Iges,
    Brep,
    /// desenho 2D: vira sketches, sem passar por provedor de geometria
    Dxf,
    /// árvore paramétrica aberta (B266): JSON, sem geometria
    Arvore,
    Stl,
    Obj,
    Ply,
    /// 3MF: nome de identificador não começa por número
    Tmf,
    Amf,
    Gltf,
    Glb,
    Vrml,
    X3d,
    Collada,
    /// USD em texto; só gravação
    Usda,
    /// pacote USDZ; só gravação
    Usdz,
    /// IFC4 facetado: STEP físico, mas de outro esquema
    Ifc,
    /// desenho 2D: vira sketches, como o DXF
    Svg,
    /// FBX binário ou texto; só leitura (formato fechado: o Fenkra não grava)
    Fbx,
}

impl Formato {
    /// Formatos de malha e visualização, que passam pela cena de [`crate::malha`].
    pub const MALHA: [Formato; 14] = [Formato::Stl, Formato::Obj, Formato::Ply, Formato::Tmf, Formato::Amf, Formato::Gltf, Formato::Glb, Formato::Vrml, Formato::X3d, Formato::Collada, Formato::Usda, Formato::Usdz, Formato::Ifc, Formato::Fbx];

    /// Todos os formatos que o conversor conhece, na ordem da tela.
    pub const TODOS: [Formato; 20] = [
        Formato::Step,
        Formato::Iges,
        Formato::Brep,
        Formato::Dxf,
        Formato::Svg,
        Formato::Ifc,
        Formato::Arvore,
        Formato::Stl,
        Formato::Obj,
        Formato::Ply,
        Formato::Tmf,
        Formato::Amf,
        Formato::Gltf,
        Formato::Glb,
        Formato::Vrml,
        Formato::X3d,
        Formato::Collada,
        Formato::Usda,
        Formato::Usdz,
        Formato::Fbx,
    ];

    pub fn e_malha(self) -> bool {
        Self::MALHA.contains(&self)
    }

    /// Formato de desenho plano, que vive de esboços e não de sólidos.
    pub fn e_2d(self) -> bool {
        matches!(self, Formato::Dxf | Formato::Svg)
    }

    pub fn nome(self) -> &'static str {
        match self {
            Formato::Step => "STEP",
            Formato::Iges => "IGES",
            Formato::Brep => "BREP",
            Formato::Dxf => "DXF",
            Formato::Arvore => "Árvore paramétrica",
            Formato::Stl => "STL",
            Formato::Obj => "OBJ",
            Formato::Ply => "PLY",
            Formato::Tmf => "3MF",
            Formato::Amf => "AMF",
            Formato::Gltf => "glTF",
            Formato::Glb => "GLB",
            Formato::Vrml => "VRML",
            Formato::X3d => "X3D",
            Formato::Collada => "Collada",
            Formato::Usda => "USD",
            Formato::Usdz => "USDZ",
            Formato::Ifc => "IFC",
            Formato::Svg => "SVG",
            Formato::Fbx => "FBX",
        }
    }

    pub fn extensoes(self) -> &'static [&'static str] {
        match self {
            Formato::Step => &["step", "stp"],
            Formato::Iges => &["igs", "iges"],
            Formato::Brep => &["brep"],
            Formato::Dxf => &["dxf"],
            Formato::Arvore => &["fkx"],
            Formato::Stl => &["stl"],
            Formato::Obj => &["obj"],
            Formato::Ply => &["ply"],
            Formato::Tmf => &["3mf"],
            Formato::Amf => &["amf"],
            Formato::Gltf => &["gltf"],
            Formato::Glb => &["glb"],
            Formato::Vrml => &["wrl", "vrml"],
            Formato::X3d => &["x3d"],
            Formato::Collada => &["dae"],
            Formato::Usda => &["usda", "usd"],
            Formato::Usdz => &["usdz"],
            Formato::Ifc => &["ifc"],
            Formato::Svg => &["svg"],
            Formato::Fbx => &["fbx"],
        }
    }

    pub fn por_extensao(caminho: &Path) -> Option<Formato> {
        let e = caminho.extension()?.to_string_lossy().to_lowercase();
        Formato::TODOS.into_iter().find(|f| f.extensoes().contains(&e.as_str()))
    }

    fn por_conteudo(conteudo: &[u8]) -> Option<Formato> {
        let texto = String::from_utf8_lossy(&conteudo[..conteudo.len().min(512)]);
        let texto = texto.trim_start_matches('\u{feff}').trim_start();
        if texto.starts_with("ISO-10303-21") {
            Some(if e_ifc(conteudo) { Formato::Ifc } else { Formato::Step })
        } else if texto.starts_with("DBRep_DrawableShape") || texto.starts_with("CASCADE Topology") {
            Some(Formato::Brep)
        } else if texto.starts_with('{') && tem_marca_da_arvore(conteudo) {
            Some(Formato::Arvore)
        } else if e_iges(conteudo) {
            Some(Formato::Iges)
        } else if e_dxf(conteudo) {
            Some(Formato::Dxf)
        } else {
            por_conteudo_de_malha(conteudo)
        }
    }
}

/// IFC é STEP físico: quem diz o que ele é, é o esquema do cabeçalho.
fn e_ifc(conteudo: &[u8]) -> bool {
    let texto = String::from_utf8_lossy(&conteudo[..conteudo.len().min(4096)]).to_uppercase();
    texto.split_once("FILE_SCHEMA").is_some_and(|(_, resto)| resto.trim_start().trim_start_matches(['(', '\'', ' ']).starts_with("IFC"))
}

/// Primeira etiqueta de um XML, sem declaração, comentário nem doctype.
fn etiqueta_xml(conteudo: &[u8]) -> Option<String> {
    let texto = String::from_utf8_lossy(&conteudo[..conteudo.len().min(8192)]);
    let mut t = texto.trim_start_matches('\u{feff}').trim_start();
    while let Some(resto) = t.strip_prefix('<') {
        if resto.starts_with('?') || resto.starts_with('!') {
            let fim = if resto.starts_with("!--") { "-->" } else { ">" };
            t = resto.split_once(fim)?.1.trim_start();
        } else {
            return Some(resto.chars().take_while(|c| c.is_alphanumeric() || *c == ':' || *c == '_').collect());
        }
    }
    None
}

/// Malhas têm assinaturas fracas: as mais firmes vêm primeiro, e o OBJ, que não tem nenhuma, por último.
fn por_conteudo_de_malha(conteudo: &[u8]) -> Option<Formato> {
    use crate::malha::{obj, stl, zip};
    if zip::e_zip(conteudo) {
        let nomes = zip::nomes(conteudo);
        let termina = |fim: &str| nomes.iter().any(|n| n.to_lowercase().ends_with(fim));
        return if termina(".model") {
            Some(Formato::Tmf)
        } else if termina(".amf") {
            Some(Formato::Amf)
        } else if termina(".usda") || termina(".usdc") || termina(".usd") {
            Some(Formato::Usdz)
        } else {
            None
        };
    }
    if crate::malha::fbx_binario(conteudo) || crate::malha::fbx_texto(conteudo) {
        return Some(Formato::Fbx);
    }
    if conteudo.starts_with(b"glTF") {
        return Some(Formato::Glb);
    }
    if conteudo.starts_with(b"#VRML") {
        return Some(Formato::Vrml);
    }
    if conteudo.starts_with(b"#usda") {
        return Some(Formato::Usda);
    }
    if conteudo.starts_with(b"ply\n") || conteudo.starts_with(b"ply\r") {
        return Some(Formato::Ply);
    }
    let etiqueta = etiqueta_xml(conteudo);
    if crate::plano::parece_svg(etiqueta.as_deref()) {
        return Some(Formato::Svg);
    }
    match etiqueta.as_deref() {
        Some("amf") => return Some(Formato::Amf),
        Some("X3D") => return Some(Formato::X3d),
        Some("COLLADA") => return Some(Formato::Collada),
        _ => {}
    }
    let texto = String::from_utf8_lossy(&conteudo[..conteudo.len().min(4096)]);
    if texto.trim_start().starts_with('{') && texto.contains("\"asset\"") {
        return Some(Formato::Gltf);
    }
    if stl::e_texto(conteudo) || stl::e_binario(conteudo) {
        return Some(Formato::Stl);
    }
    obj::parece(conteudo).then_some(Formato::Obj)
}

/// O IGES não tem assinatura: é texto em cartões de 80 colunas, com a seção na coluna 73 e o número da
/// linha depois dela. A primeira linha é da seção de início (S) ou, sem ela, da global (G).
fn e_iges(conteudo: &[u8]) -> bool {
    let linha = conteudo.split(|b| *b == b'\n').next().unwrap_or_default();
    let linha = linha.strip_suffix(b"\r").unwrap_or(linha);
    linha.len() == 80 && matches!(linha[72], b'S' | b'G') && linha[73..].iter().all(|b| *b == b' ' || b.is_ascii_digit())
}

/// O DXF abre com o par "0, SECTION", depois de comentários (código 999), ou com a marca do binário.
fn e_dxf(conteudo: &[u8]) -> bool {
    if fenkra_dxf::e_binario(conteudo) {
        return true;
    }
    let texto = String::from_utf8_lossy(&conteudo[..conteudo.len().min(4096)]);
    let mut linhas = texto.lines().map(str::trim).filter(|l| !l.is_empty());
    while let (Some(codigo), Some(valor)) = (linhas.next(), linhas.next()) {
        match codigo {
            "999" => continue,
            "0" => return valor == "SECTION",
            _ => return false,
        }
    }
    false
}

/// O campo `formato` pode estar em qualquer lugar do objeto: quem escreve o JSON escolhe a ordem.
fn tem_marca_da_arvore(conteudo: &[u8]) -> bool {
    let marca = crate::arvore::NOME_FORMATO.as_bytes();
    conteudo.windows(marca.len()).any(|w| w == marca)
}

/// Formato do arquivo pelo conteúdo, e se a extensão diz outra coisa.
pub fn detectar(caminho: &Path, conteudo: &[u8]) -> Result<(Formato, bool), ErroConv> {
    let Some(f) = Formato::por_conteudo(conteudo) else {
        return Err(ErroConv::Formato(format!(
            "'{}' não parece um formato que o Fenkra leia (STEP, IGES, BREP, DXF, árvore paramétrica ou malha aberta): exporte STEP AP214 ou AP242 no programa de origem",
            caminho.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()
        )));
    };
    Ok((f, Formato::por_extensao(caminho) != Some(f)))
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn conteudo_manda_e_extensao_diferente_e_avisada() {
        assert_eq!(detectar(Path::new("a.step"), b"ISO-10303-21;\nHEADER;").unwrap(), (Formato::Step, false));
        assert_eq!(detectar(Path::new("a.dat"), b"ISO-10303-21;\nHEADER;").unwrap(), (Formato::Step, true));
        assert_eq!(detectar(Path::new("a.brep"), b"DBRep_DrawableShape\n\nCASCADE Topology V3").unwrap(), (Formato::Brep, false));
    }

    #[test]
    fn arvore_e_reconhecida_com_o_campo_em_qualquer_lugar() {
        let tardio = format!("{{\"pecas\": [], \"gerador\": \"{}\", \"formato\": \"fenkra-arvore\"}}", "x".repeat(600));
        assert_eq!(detectar(Path::new("a.fkx"), tardio.as_bytes()).unwrap(), (Formato::Arvore, false));
        assert_eq!(detectar(Path::new("a.json"), tardio.as_bytes()).unwrap(), (Formato::Arvore, true));
        assert!(detectar(Path::new("a.json"), b"{\"outro\": 1}").is_err());
    }

    #[test]
    fn malhas_sao_reconhecidas_pelo_conteudo() {
        let casos: [(&[u8], Formato); 10] = [
            (b"ply\nformat ascii 1.0\n", Formato::Ply),
            (b"#VRML V2.0 utf8\n", Formato::Vrml),
            (b"#usda 1.0\n", Formato::Usda),
            (b"<?xml version=\"1.0\"?>\n<!-- x -->\n<amf unit=\"inch\"/>", Formato::Amf),
            (b"<?xml version=\"1.0\"?><!DOCTYPE X3D PUBLIC \"a\" \"b\"><X3D/>", Formato::X3d),
            (b"<COLLADA version=\"1.4.1\"/>", Formato::Collada),
            (b"{\"asset\": {\"version\": \"2.0\"}}", Formato::Gltf),
            (b"glTF\x02\0\0\0", Formato::Glb),
            (b"solid peca\n facet normal 0 0 1\n", Formato::Stl),
            (b"# comentario\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n", Formato::Obj),
        ];
        for (conteudo, f) in casos {
            assert_eq!(detectar(Path::new("a.dat"), conteudo).unwrap(), (f, true), "{}", f.nome());
        }
    }

    #[test]
    fn ifc_se_separa_do_step_pelo_esquema_e_svg_pela_etiqueta() {
        let ifc = b"ISO-10303-21;\nHEADER;\nFILE_SCHEMA(('IFC4'));\nENDSEC;";
        assert_eq!(detectar(Path::new("a.ifc"), ifc).unwrap(), (Formato::Ifc, false));
        let step = b"ISO-10303-21;\nHEADER;\nFILE_SCHEMA(('AUTOMOTIVE_DESIGN'));\nENDSEC;";
        assert_eq!(detectar(Path::new("a.step"), step).unwrap(), (Formato::Step, false));
        assert_eq!(detectar(Path::new("a.svg"), b"<?xml version=\"1.0\"?>\n<svg xmlns=\"x\"/>").unwrap(), (Formato::Svg, false));
    }

    #[test]
    fn stl_binario_se_reconhece_pelo_tamanho_e_texto_solto_nao_vira_obj() {
        let mut b = vec![0u8; 84 + 50];
        b[80] = 1;
        assert_eq!(detectar(Path::new("a.stl"), &b).unwrap(), (Formato::Stl, false));
        b.push(0);
        assert!(detectar(Path::new("a.stl"), &b).is_err());
        assert!(detectar(Path::new("a.obj"), b"isto nao e obj\nv 0 0 0\n").is_err());
    }

    #[test]
    fn pacote_zip_se_reconhece_pelo_que_tem_dentro() {
        let z = |nome: &str| crate::malha::zip::gravar(&[(nome, b"x")], false).unwrap();
        assert_eq!(detectar(Path::new("a.3mf"), &z("3D/3dmodel.model")).unwrap().0, Formato::Tmf);
        assert_eq!(detectar(Path::new("a.amf"), &z("peca.amf")).unwrap().0, Formato::Amf);
        assert_eq!(detectar(Path::new("a.usdz"), &z("modelo.usda")).unwrap().0, Formato::Usdz);
        assert!(detectar(Path::new("a.zip"), &z("leia.txt")).is_err());
    }

    #[test]
    fn texto_qualquer_e_recusado_com_orientacao() {
        let e = detectar(Path::new("peca.sldprt"), b"qualquer coisa").unwrap_err();
        assert!(e.to_string().contains("STEP AP214"), "{e}");
    }
}
