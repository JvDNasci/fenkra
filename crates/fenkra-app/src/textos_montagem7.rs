// SPDX-License-Identifier: LGPL-2.1-or-later
//! Traduções da montagem: peça de fora do projeto copiada para ele ao inserir (B445).

pub static TEXTOS: &[(&str, &str)] = &[
    ("Peça de fora do projeto", "Part from outside the project"),
    ("não pertence ao projeto", "does not belong to the project"),
    (
        "Para entrar na montagem, a peça é copiada para a pasta do projeto junto com as peças de que ela depende. A cópia ganha PN e versão e mantém todo o histórico de comandos; o arquivo original não muda.",
        "To enter the assembly, the part is copied to the project folder along with the parts it depends on. The copy gets a PN and a version and keeps its whole feature history; the original file does not change.",
    ),
    ("Já existe um arquivo com este nome no projeto: a cópia vai se chamar", "A file with this name already exists in the project: the copy will be named"),
    ("Copiar para o projeto", "Copy to project"),
    (
        "Copia a peça e as dependências para a pasta do projeto, registra cada uma com PN e versão e põe a cópia na mão.",
        "Copies the part and its dependencies to the project folder, registers each one with a PN and a version and puts the copy in your hand.",
    ),
    ("A peça não entra na montagem e nada é copiado.", "The part does not enter the assembly and nothing is copied."),
    ("Nenhuma peça esperando a cópia", "No part waiting to be copied"),
    ("A cópia para o projeto falhou", "Copying to the project failed"),
    ("Peça copiada para o projeto:", "Part copied to the project:"),
    ("peça(s) usada(s) por link também copiada(s)", "part(s) used by links also copied"),
    ("Já havia no projeto um arquivo", "The project already had a file"),
    ("a cópia se chama", "the copy is named"),
    ("O PN", "The PN"),
    ("já era de outro documento; a cópia ficou com", "already belonged to another document; the copy got"),
    ("Referência sem arquivo, mantida como estava:", "Reference without a file, kept as it was:"),
];
