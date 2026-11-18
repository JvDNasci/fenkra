// SPDX-License-Identifier: LGPL-2.1-or-later
//! Traduções das barras soltas e encaixáveis, dos painéis que mudam de tamanho e da árvore
//! transparente (B502, B503; ADR 0104), à parte de `i18n.rs`.

pub static TEXTOS: &[(&str, &str)] = &[
    // lugar de uma barra
    ("Encaixada no topo", "Docked at the top"),
    ("Encaixada à esquerda", "Docked on the left"),
    ("Encaixada à direita", "Docked on the right"),
    ("Encaixada embaixo", "Docked at the bottom"),
    ("Arraste para soltar a barra; solte numa borda para encaixar.", "Drag to undock the toolbar; drop it on an edge to dock it."),
    ("Fechar a barra. Ela volta pelo menu Janela, em Barras de ferramentas.", "Close the toolbar. It comes back from the Window menu, under Toolbars."),
    // menu Janela > Barras de ferramentas
    ("Arraste a alça de uma barra para soltá-la ou encaixá-la noutra borda.", "Drag a toolbar's handle to undock it or dock it on another edge."),
    ("Nova barra de comandos…", "New command toolbar…"),
    ("Cria uma caixa de comandos sua. Arraste os ícones da lista para dentro dela.", "Creates your own command box. Drag icons from the list into it."),
    ("Minha barra", "My toolbar"),
    ("Restaurar o layout desta bancada", "Restore this workbench's layout"),
    ("Volta as barras e o tamanho delas ao padrão desta bancada. As barras de comandos suas continuam.", "Puts the toolbars and their sizes back to this workbench's default. Your own command toolbars stay."),
    ("Restaurar o layout de todas as bancadas", "Restore the layout of all workbenches"),
    ("Volta as barras de todas as bancadas ao padrão. As barras de comandos suas continuam.", "Puts the toolbars of every workbench back to the default. Your own command toolbars stay."),
    // configurações
    ("Dica: arraste a alça de um grupo para soltá-lo e solte perto de uma borda para encaixar. O clique direito no grupo também escolhe o lugar. O lugar vale para a bancada atual.", "Tip: drag a group's handle to undock it and drop it near an edge to dock it. Right-clicking the group also picks its place. The place applies to the current workbench."),
    ("Barras soltas encaixam quando soltas perto de qualquer borda; arraste a alça de uma barra encaixada para soltá-la. Posições e tamanhos ficam salvos por bancada.", "Floating toolbars dock when dropped near any edge; drag a docked toolbar's handle to undock it. Positions and sizes are saved per workbench."),
    // caixa de comandos própria
    ("Caixa de comandos", "Command box"),
    ("Arraste os ícones da lista para a caixa. Arraste um ícone da caixa para mudar a ordem, ou para fora dela para tirá-lo.", "Drag icons from the list into the box. Drag an icon inside the box to reorder it, or out of the box to remove it."),
    ("Vazia: arraste comandos para cá.", "Empty: drag commands here."),
    ("Comando indisponível nesta versão. Arraste para fora da caixa para tirá-lo.", "Command not available in this version. Drag it out of the box to remove it."),
    ("buscar", "search"),
    // árvore transparente
    ("Arraste para mover a árvore sobre a vista", "Drag to move the tree over the view"),
    ("Voltar a árvore ao canto", "Put the tree back in the corner"),
    ("Devolve a árvore ao canto da vista, com a largura padrão", "Puts the tree back in the corner of the view, at the default width"),
    ("Arraste para mudar a largura da árvore", "Drag to change the tree's width"),
    // B507: painel do comando e assistente de reparo flutuam ou encaixam
    ("Flutuar sobre a vista", "Float over the view"),
    ("Flutuar: o painel sai da borda e fica sobre a vista, onde você o arrastar. A posição fica guardada.", "Float: the panel leaves the edge and stays over the view, wherever you drag it. Its position is remembered."),
    ("Encaixar: o painel volta para a borda e reserva espaço. Arrastar até uma borda faz o mesmo.", "Dock: the panel goes back to the edge and reserves space. Dragging it to an edge does the same."),
];
