// SPDX-License-Identifier: LGPL-2.1-or-later
//! Traduções do B211: desenho 2D aberto como folha (DXF, DWG, SVG) e programa externo do DWG.

pub static TEXTOS: &[(&str, &str)] = &[
    ("Abrir desenho (.fkd, DXF, DWG, SVG)", "Open drawing (.fkd, DXF, DWG, SVG)"),
    ("Abre um desenho salvo, com as vistas recalculadas pelos arquivos de origem, ou um DXF, DWG ou SVG como folha nova, com um esboço por camada.", "Opens a saved drawing, with the views recomputed from their source files, or a DXF, DWG or SVG as a new sheet, with one sketch per layer."),
    ("Desenho 2D (DXF, DWG, SVG)", "2D drawing (DXF, DWG, SVG)"),
    ("Desenho aberto", "Drawing opened"),
    ("esboços", "sketches"),
    ("textos", "texts"),
    ("sem unidade no arquivo: valeu milímetro", "no unit in the file: millimeter was used"),
    ("Programas externos", "External programs"),
    ("LibreDWG (DWG)", "LibreDWG (DWG)"),
    ("Programa livre, instalado à parte, que o Fenkra chama para ler e gravar DWG. Ele roda em outro processo e não faz parte do Fenkra.", "Free program, installed separately, that Fenkra calls to read and write DWG. It runs in another process and is not part of Fenkra."),
    ("vazio: procura no PATH", "empty: searches the PATH"),
    ("Pasta com dwg2dxf e dxf2dwg. Vazia, o Fenkra procura no PATH e nas pastas de instalação de costume.", "Folder with dwg2dxf and dxf2dwg. When empty, Fenkra searches the PATH and the usual install folders."),
    ("Escolhe a pasta do LibreDWG.", "Chooses the LibreDWG folder."),
    ("Como instalar: baixe o pacote para Windows em ftp.gnu.org/gnu/libredwg (ou rode vcpkg install libredwg[tools]), descompacte e escolha aqui a pasta com dwg2dxf e dxf2dwg.", "How to install: download the Windows package from ftp.gnu.org/gnu/libredwg (or run vcpkg install libredwg[tools]), unzip it and choose here the folder with dwg2dxf and dxf2dwg."),
];
