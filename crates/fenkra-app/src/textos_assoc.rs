// SPDX-License-Identifier: LGPL-2.1-or-later
//! Traduções da associação de `.fkp`, `.fka` e `.fkd` (B389), à parte de `i18n.rs`.

pub static TEXTOS: &[(&str, &str)] = &[
    ("Abrir peças, montagens e desenhos com duplo clique", "Open parts, assemblies and drawings with a double click"),
    ("Associa .fkp, .fka e .fkd ao Fenkra: o duplo clique no arquivo abre o programa já com ele. Vale só para o seu usuário do Windows e não pede administrador.", "Associates .fkp, .fka and .fkd with Fenkra: a double click on the file opens the program with it. It applies only to your Windows user and needs no administrator."),
    ("Os arquivos do Fenkra abrem pelo programa com duplo clique.", "Fenkra files open in the program with a double click."),
    ("Os arquivos do Fenkra não estão associados ao programa.", "Fenkra files are not associated with the program."),
    ("Associar arquivos do Fenkra", "Associate Fenkra files"),
    ("Grava seis chaves em HKCU\\Software\\Classes (três extensões e três classes). Só acontece com este clique.", "Writes six keys under HKCU\\Software\\Classes (three extensions and three classes). It only happens with this click."),
    ("Apaga as seis chaves que o botão ao lado criou.", "Deletes the six keys the button next to it created."),
];
