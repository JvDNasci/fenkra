// SPDX-License-Identifier: LGPL-2.1-or-later
//! Traduções do encaixe de tubo com corte normal à parede, a boca de lobo (B574). Tabela própria para
//! não disputar `i18n.rs`.

pub static TEXTOS: &[(&str, &str)] = &[
    ("Encaixe do tubo (boca de lobo)", "Tube cope (fish mouth)"),
    ("Tipo de corte", "Cut type"),
    ("Booleana", "Boolean"),
    ("Normal à parede", "Normal to the wall"),
    ("Booleana: o outro tubo é subtraído e a face de corte cola na superfície dele; em ângulo, a borda da parede vira ponta fina. Normal à parede: como o corte de plasma ou o traçado à mão, a face de corte contém a normal da parede em cada ponto, e a borda mantém a espessura inteira.", "Boolean: the other tube is subtracted and the cut face sticks to its surface; at an angle the wall edge becomes a thin tip. Normal to the wall: like a plasma cut or a hand layout, the cut face contains the wall normal at every point and the edge keeps the full thickness."),
    ("Parede que a curva segue", "Wall the curve follows"),
    ("Automático: ponto a ponto, a parede que toca primeiro; nunca invade o outro tubo. Externa, interna e média escolhem uma só; a interferência, se houver, é medida e avisada.", "Automatic: point by point, the wall that touches first; it never enters the other tube. Outer, inner and middle pick one only; any interference is measured and reported."),
    ("Face do tubo que recebe", "Face of the receiving tube"),
    ("Superfície do tubo que recebe usada como encosto: face externa (a solda de sempre), meio da parede ou face interna (o tubo que chega entra até o furo do outro).", "Surface of the receiving tube used as the seat: outer face (the usual weld), mid-wall or inner face (the incoming tube reaches the bore of the other)."),
    ("Face externa", "Outer face"),
    ("Meio da parede", "Mid-wall"),
    ("Face interna", "Inner face"),
    ("padrão do membro", "member default"),
    ("Opções do encaixe só deste canto; vazio usa as do membro.", "Cope options for this corner only; empty uses the member's."),
    ("Usar as do membro", "Use the member's"),
    ("Vão de raiz máximo:", "Maximum root gap:"),
    ("interferência:", "interference:"),
    ("espessura da borda:", "edge thickness:"),
    ("Medidos na curva de corte contra o tubo que recebe. Vão e interferência vêm da normal dele; a espessura da borda é a parede inteira no corte normal.", "Measured on the cut curve against the receiving tube. Gap and interference come from its normal; the edge thickness is the full wall in the normal cut."),
    ("Corte normal à parede: o vão acima da tolerância pede chanfro na solda (a tolerância é o orçamento ou a parede, o que for maior).", "Cut normal to the wall: a gap above the tolerance calls for a weld bevel (the tolerance is the budget or the wall, whichever is larger)."),
    ("Gabaritos de encaixe…", "Cope templates…"),
    ("Grava, numa pasta, um DXF por tubo com o gabarito planificado da boca de lobo (curva de corte desenrolada na pele externa, em milímetros).", "Writes, to a folder, one DXF per tube with the flat template of the fish mouth (cut curve unrolled on the outer skin, in millimetres)."),
    ("Gabaritos gravados", "Templates written"),
    ("arquivos", "files"),
    ("Nenhum encaixe com corte normal à parede neste modelo", "No cope with a cut normal to the wall in this model"),
    ("Sem protótipo: o corte normal e o vão aceito vêm de hipótese de trabalho; confira em peça real.", "No prototype: the normal cut and the accepted gap are working assumptions; check on a real part."),
];
