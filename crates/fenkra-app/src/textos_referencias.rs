// SPDX-License-Identifier: LGPL-2.1-or-later
//! Traduções dos nomes de referência (B516): papéis de face, partes de furo e de aba, e os textos dos
//! campos que mostram o nome da face ou aresta no lugar da coordenada. À parte de `i18n.rs`.

pub static TEXTOS: &[(&str, &str)] = &[
    ("sem papel", "no role"),
    ("tampa inicial", "start cap"),
    ("tampa final", "end cap"),
    ("lateral", "side"),
    ("trecho", "segment"),
    ("parede da cabeça", "head wall"),
    ("boca", "mouth"),
    ("fundo", "bottom"),
    ("assento", "seat"),
    ("fundo do outro lado", "far bottom"),
    ("dobra interna", "inner bend"),
    ("dobra externa", "outer bend"),
    ("face interna", "inner face"),
    ("face externa", "outer face"),
    ("lado 1", "side 1"),
    ("lado 2", "side 2"),
    ("lado 1 da dobra", "bend side 1"),
    ("lado 2 da dobra", "bend side 2"),
    ("raiz", "root"),
    ("Face sem nome perto de", "Unnamed face near"),
    ("Aresta sem nome perto de", "Unnamed edge near"),
    ("Ponto sem nome em", "Unnamed point at"),
    ("O nome vem do comando que criou a face e do papel dela; ele segue a face quando a peça muda. Sem nome, a face ainda é achada pelo ponto, e ganha nome no próximo cálculo.", "The name comes from the command that created the face and its role; it follows the face when the part changes. Without a name the face is still found by the point, and gets a name on the next update."),
    ("O nome vem das duas faces vizinhas; ele segue a aresta quando a peça muda. Sem nome, a aresta ainda é achada pelo ponto, e ganha nome no próximo cálculo.", "The name comes from the two neighbouring faces; it follows the edge when the part changes. Without a name the edge is still found by the point, and gets a name on the next update."),
    ("Vértice de", "Vertex of"),
    ("Centro de", "Center of"),
    ("Ponto em", "Point on"),
    ("Eixo de", "Axis of"),
];
