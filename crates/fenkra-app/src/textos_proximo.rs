// SPDX-License-Identifier: LGPL-2.1-or-later
//! Traduções da sugestão do próximo comando e do passo que falta (B441), à parte de `i18n.rs`.

pub static TEXTOS: &[(&str, &str)] = &[
    // B441: o que falta para o comando rodar
    ("Ainda não há corpo: este comando trabalha sobre um sólido.", "There is no body yet: this command works on a solid."),
    ("Ainda não há corpo: este comando trabalha sobre um sólido, e o sólido nasce de um perfil num sketch.", "There is no body yet: this command works on a solid, and the solid starts from a profile in a sketch."),
    ("Nenhum desenho aberto: as vistas, as cotas e as tabelas moram numa folha.", "No drawing is open: views, dimensions and tables live on a sheet."),
    ("A folha ainda não tem vistas: este comando parte de uma vista da peça.", "The sheet has no views yet: this command starts from a view of the part."),
    ("A peça ainda não tem parâmetros de chapa (espessura, raio de dobra e fator K).", "The part has no sheet metal parameters yet (thickness, bend radius and K-factor)."),
    ("Falta a parede base: a aba, a dobra e o recorte partem dela.", "The base wall is missing: flanges, bends and cuts start from it."),
    ("Falta a parede base, e ela nasce de um perfil num sketch.", "The base wall is missing, and it starts from a profile in a sketch."),
    ("A parede base nasce de um perfil num sketch, e a peça ainda não tem sketch.", "The base wall starts from a profile in a sketch, and the part has no sketch yet."),
    ("Este comando parte de um perfil num sketch, e a peça ainda não tem sketch.", "This command starts from a profile in a sketch, and the part has no sketch yet."),
    ("A booleana combina dois corpos, e a peça tem menos de dois.", "A boolean combines two bodies, and the part has fewer than two."),
    ("O membro segue linhas: desenhe o arame 3D ou um sketch com as linhas da estrutura.", "A member follows lines: draw the 3D wireframe or a sketch with the lines of the structure."),
    ("Ainda não há membros estruturais: placas, reforços, soldas, cantos e a lista de corte partem deles.", "There are no structural members yet: plates, gussets, welds, corners and the cut list start from them."),
    ("Restrições, juntas e recursos de montagem ligam duas peças, e a montagem tem menos de duas.", "Constraints, joints and assembly features connect two parts, and the assembly has fewer than two."),
    ("A montagem ainda não tem peças.", "The assembly has no parts yet."),
    ("Novo corpo", "New body"),
    ("Falta um passo", "A step is missing"),
    ("usar mesmo assim", "use it anyway"),
    ("Abre o comando como está; ele espera pelo que falta.", "Opens the command as it is; it waits for what is missing."),
    ("Falta um passo antes deste comando. Clique para ver qual.", "A step is missing before this command. Click to see which."),
    // B441: próximos do fluxo
    ("Próximo:", "Next:"),
    ("Próximos comandos mais prováveis no fluxo da bancada", "Most likely next commands in the workbench flow"),
    ("Esconder a sugestão", "Hide the suggestion"),
];
