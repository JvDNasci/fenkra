//! Barra de comandos em **blocos** (estilo faixa): cada grupo tem nome embaixo e linhas separadoras.
//! Comandos parecidos formam **famílias**: o botão mostra a última variação usada e a setinha ▾
//! abre as outras. Grupos podem ficar no topo, na lateral, flutuar ou ser ocultados (clique direito).

use crate::app::Cmd;
use crate::geo_ui::{self, GeoKind};
use fenkra_core::{BoolOp, PrimKind};
use crate::icons::tool_button;
use crate::render::DisplayMode;
use crate::settings::{Dock, Group, Settings};
use crate::sketch_ui::{CKind, Tool};
use eframe::egui::{self, RichText};
use crate::i18n::tr;

/// Ação pedida por um botão de ferramenta.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Act {
    /// modo da seleção em caixa (0 Ctrl+arrastar, 1 janela, 2 cruzada, 3 pela direção)
    SelMode(u8),
    /// abre a janela de personalização dos grupos
    Customize,
    /// abre a janela de automação (macros)
    Macros,
    Cmd(Cmd),
    Fit,
    View(f32, f32),
    Normal,
    Ortho,
    HiddenEnv,
    Settings,
    Dock(Group, Dock),
    /// começou a arrastar um grupo encaixado pela alça (retângulo do bloco)
    DockDrag(Group, egui::Rect),
    ClearPicks,
    SkTool(Tool),
    SkConstruction,
    SkToggleConstruction,
    SkCheck,
    SkGrid,
    SkSnap,
    SkAutoConstraints,
    SkAutoDims,
    SkFinish,
    /// modo de exibição (análise: zebra, curvatura)
    Display(DisplayMode),
}

/// Botão: ícone, nome, descrição breve, descrição avançada, ação.
#[derive(Clone, Copy)]
pub struct Item {
    pub icon: &'static str,
    pub name: &'static str,
    pub brief: &'static str,
    pub advanced: &'static str,
    pub act: Act,
}

#[derive(Clone)]
pub enum Entry {
    One(Item),
    /// família: chave (para lembrar a variação) + variações
    Family(&'static str, Vec<Item>),
}

/// Chave estável de uma entrada da barra (para ocultar, ordenar e montar grupos).
pub fn entry_key(e: &Entry) -> String {
    match e {
        Entry::One(i) => format!("{}|{}", i.icon, i.name),
        Entry::Family(k, _) => format!("fam:{k}"),
    }
}

/// Ícone e descrição de uma entrada (para listas).
pub fn entry_title(e: &Entry) -> (&'static str, &'static str) {
    match e {
        Entry::One(i) => (i.icon, i.brief),
        Entry::Family(_, v) => (v[0].icon, v[0].brief),
    }
}

/// Entradas do grupo com a personalização aplicada (ocultas fora, ordem do usuário).
pub fn custom_entries(g: Group, s: &Settings) -> Vec<Entry> {
    let mut list: Vec<Entry> = entries(g).into_iter().filter(|e| !s.custom.hidden.contains(&entry_key(e))).collect();
    if let Some((_, order)) = s.custom.order.iter().find(|(x, _)| *x == g) {
        list.sort_by_key(|e| order.iter().position(|k| *k == entry_key(e)).unwrap_or(usize::MAX));
    }
    list
}

/// Todas as entradas de todos os grupos (sem repetir).
pub fn all_entries() -> Vec<Entry> {
    let mut out: Vec<Entry> = vec![];
    for g in Group::ALL {
        for e in entries(g) {
            if !out.iter().any(|x| entry_key(x) == entry_key(&e)) {
                out.push(e);
            }
        }
    }
    out
}

/// Grupo do usuário na barra.
pub fn show_custom(ui: &mut egui::Ui, cg: &crate::settings::CustomGroup, s: &mut Settings, st: &ToolState, vertical: bool) -> Vec<Act> {
    let all = all_entries();
    let list: Vec<Entry> = cg.items.iter().filter_map(|k| all.iter().find(|e| entry_key(e) == *k).cloned()).collect();
    show_block(ui, None, cg.name.clone(), list, s, st, vertical)
}

const fn it(icon: &'static str, name: &'static str, brief: &'static str, advanced: &'static str, act: Act) -> Item {
    Item { icon, name, brief, advanced, act }
}

fn tool(t: Tool) -> Item {
    let (i, n, b, a) = t.info();
    it(i, n, b, a, Act::SkTool(t))
}

pub fn entries(g: Group) -> Vec<Entry> {
    use crate::app::Cmd as C;
    use Act::*;
    use Entry::{Family, One};
    match g {
        Group::Sketch => vec![One(it("sketch", "Sketch", "Novo sketch no plano selecionado", "Cria um sketch no plano ou referência selecionada (ou XY) e entra no modo de desenho, com a vista alinhada ao plano.", Cmd(C::Sketch)))],
        Group::Solidos => vec![
            Family("extrusao", vec![
                it("extrusao", "Extrusão", "Extrusão cega", "Empurra o perfil fechado do sketch ao longo da normal do plano por uma distância.", Cmd(C::Extrude)),
                it("ext_simetrico", "Plano médio", "Extrusão simétrica (plano médio)", "Metade da distância para cada lado do plano do sketch.", Cmd(C::ExtrudeMid)),
                it("ext_ate_plano", "Até plano", "Extrusão até um plano", "Termina exatamente num plano — da Origem ou de referência, inclusive inclinado.", Cmd(C::ExtrudeUpTo)),
                it("ext_saida_filete", "Saída + filetes", "Extrusão com saída e filetes (drafted filleted pad)", "Extruda com ângulo de saída e já arredonda as arestas criadas — ajuste os dois comandos depois.", Cmd(C::ExtrudeDraftFillet)),
            ]),
            Family("recorte", vec![
                it("recorte", "Recorte", "Recorte passante", "Remove material atravessando todo o corpo, a partir do perfil do sketch.", Cmd(C::Cut)),
                it("ext_cego", "Recorte cego", "Recorte com profundidade", "Remove material até uma profundidade.", Cmd(C::CutBlind)),
                it("ext_ate_plano", "Recorte até plano", "Recorte até um plano", "Remove material até um plano (inclusive inclinado).", Cmd(C::CutUpTo)),
            ]),
            Family("revolucao", vec![
                it("revolucao", "Revolução", "Girar o perfil em torno de um eixo (Shaft)", "Gira o perfil fechado em torno de um eixo da Origem, de uma linha do sketch ou de uma linha de referência.", Cmd(C::Revolve)),
                it("ranhura", "Ranhura", "Remover girando o perfil (Groove)", "Remove material girando o perfil em torno do eixo.", Cmd(C::Groove)),
            ]),
            Family("varredura", vec![
                it("nervura", "Nervura", "Varrer o perfil ao longo de uma curva (Rib)", "O perfil do sketch percorre uma curva guia (linha, spline, hélice, aresta) acrescentando material.", Cmd(C::Rib)),
                it("rasgo_varrido", "Rasgo", "Remover varrendo ao longo de uma curva (Slot)", "O perfil percorre a curva guia removendo material.", Cmd(C::Slot)),
            ]),
            Family("multisecoes", vec![
                it("multisecoes", "Multi-seções", "Sólido por vários perfis (loft)", "Transição suave entre dois ou mais perfis em planos diferentes.", Cmd(C::Loft)),
                it("multisecoes_remover", "Multi-seções -", "Remover por vários perfis", "Remove material com a transição entre os perfis.", Cmd(C::LoftCut)),
            ]),
            Family("primitivas", vec![
                it("prim_caixa", "Caixa", "Caixa", "Caixa centrada no plano escolhido; altura ao longo da normal.", Cmd(C::Prim(PrimKind::Caixa))),
                it("prim_cilindro", "Cilindro", "Cilindro", "Raio e altura, sobre o plano escolhido.", Cmd(C::Prim(PrimKind::Cilindro))),
                it("prim_esfera", "Esfera", "Esfera", "Raio, centro na origem do plano.", Cmd(C::Prim(PrimKind::Esfera))),
                it("prim_cone", "Cone", "Cone / tronco", "Raios da base e do topo e altura.", Cmd(C::Prim(PrimKind::Cone))),
                it("prim_toro", "Toro", "Toro", "Raio maior e menor.", Cmd(C::Prim(PrimKind::Toro))),
                it("prim_cunha", "Cunha", "Cunha", "Caixa com o topo mais estreito.", Cmd(C::Prim(PrimKind::Cunha))),
                it("prim_prisma", "Prisma", "Prisma poligonal", "Polígono regular extrudado (lados, raio, altura).", Cmd(C::Prim(PrimKind::Prisma))),
            ]),
            Family("booleana", vec![
                it("bool_unir", "Unir corpos", "Unir dois corpos", "Junta ao corpo ativo o corpo de outro coletor.", Cmd(C::Combine(BoolOp::Add))),
                it("bool_subtrair", "Subtrair corpo", "Subtrair um corpo", "Tira do corpo ativo o corpo de outro coletor.", Cmd(C::Combine(BoolOp::Cut))),
                it("bool_intersecao", "Interseção", "Interseção de corpos", "Fica só o volume comum aos dois corpos.", Cmd(C::Combine(BoolOp::Intersect))),
            ]),
            One(it("reforco", "Reforço", "Reforço / nervura de parede (Stiffener)", "Cada linha do sketch vira uma parede fina com espessura e profundidade.", Cmd(C::Stiffener))),
            Family("furo", vec![
                it("furo_simples", "Furo", "Furo de passagem", "Furo nos pontos de um sketch, com diâmetro de passagem para o parafuso escolhido.", Cmd(C::Hole)),
                it("furo_rebaixado", "Rebaixado", "Furo rebaixado", "Passagem + rebaixo para a cabeça do parafuso Allen.", Cmd(C::HoleCounterbore)),
                it("furo_escareado", "Escareado", "Furo escareado", "Passagem + escareado a 90° para parafuso de cabeça chata.", Cmd(C::HoleCountersink)),
                it("furo_roscado", "Roscado", "Furo roscado", "Broca de rosca + rosca cosmética da norma escolhida.", Cmd(C::HoleTapped)),
            ]),
        ],
        Group::Acabamento => vec![
            Family("filete", vec![
                it("arestas_selecionar", "Filete", "Filete nas arestas selecionadas", "Arredonda as arestas selecionadas — antes, ou clicando nelas com o comando aberto.", Cmd(C::Fillet)),
                it("filete_variavel", "Filete variável", "Filete com raio variável", "O raio muda do início ao fim de cada aresta escolhida.", Cmd(C::FilletVar)),
                it("filete_face_face", "Face-face", "Filete entre duas faces", "Clique em duas faces: arredonda a aresta entre elas.", Cmd(C::FilletFaceFace)),
                it("filete_completo", "Completo", "Filete completo (tritangente)", "Clique na face do meio e nas duas laterais: vira meia-cana.", Cmd(C::FilletFull)),
                it("arestas_todas", "Filete total", "Filete em todas as arestas", "Arredonda todas as arestas do corpo.", Cmd(C::FilletAll)),
            ]),
            Family("chanfro", vec![
                it("chanfro", "Chanfro", "Chanfro nas arestas selecionadas", "Chanfra as arestas selecionadas com a mesma distância.", Cmd(C::Chamfer)),
                it("chanfro_dist_dist", "2 distâncias", "Chanfro por duas distâncias", "Uma distância em cada face da aresta.", Cmd(C::ChamferTwoDist)),
                it("chanfro_dist_angulo", "Dist. + ângulo", "Chanfro por distância e ângulo", "Distância numa face e ângulo em relação a ela.", Cmd(C::ChamferDistAngle)),
                it("chanfro_dist", "Chanfro total", "Chanfro em todas as arestas", "Chanfra todas as arestas do corpo.", Cmd(C::ChamferAll)),
            ]),
            One(it("casca", "Casca", "Esvaziar o sólido (Shell)", "Abre as faces escolhidas e deixa paredes com espessura uniforme.", Cmd(C::Shell))),
            One(it("saida", "Saída", "Ângulo de saída (Draft)", "Inclina as faces escolhidas a partir de um plano neutro, para desmoldar.", Cmd(C::Draft))),
            One(it("espessura", "Espessura", "Engrossar ou afinar faces (Thickness)", "Acrescenta ou remove material nas faces planas escolhidas.", Cmd(C::Thickness))),
            One(it("rosca", "Rosca", "Rosca em eixo ou furo (Thread/Tap)", "Rosca cosmética ou modelada numa face cilíndrica, por norma.", Cmd(C::Thread))),
        ],
        Group::Padroes => vec![
            Family("padrao", vec![
                it("padrao_linear", "Padrão linear", "Padrão retangular", "Repete os comandos escolhidos (ou o corpo) em uma ou duas direções.", Cmd(C::PatternLinear)),
                it("padrao_circular", "Padrão circular", "Padrão circular", "Repete em volta de um eixo.", Cmd(C::PatternCircular)),
                it("padrao_pontos", "Por pontos", "Padrão por pontos (user pattern)", "Repete nos pontos escolhidos.", Cmd(C::PatternPoints)),
            ]),
            One(it("espelho", "Espelho", "Espelhar (Mirror)", "Espelha os comandos escolhidos (ou o corpo) por um plano.", Cmd(C::Mirror))),
            One(it("escala", "Escala", "Escalar a peça (Scale)", "Multiplica as medidas do corpo a partir de um centro.", Cmd(C::Scale))),
            Family("mover_corpo", vec![
                it("xf_mover", "Translação", "Mover o corpo (Translation)", "Move o corpo inteiro ao longo de uma direção.", Cmd(C::MoveBody(0))),
                it("xf_girar", "Rotação", "Girar o corpo (Rotation)", "Gira o corpo inteiro em torno de um eixo.", Cmd(C::MoveBody(1))),
                it("xf_simetria", "Simetria", "Simetria do corpo (Symmetry)", "Leva o corpo para o lado espelhado (sem cópia; para copiar use Espelho).", Cmd(C::MoveBody(2))),
                it("xf_afinidade", "Afinidade", "Afinidade (Affinity)", "Escala independente em X, Y e Z.", Cmd(C::MoveBody(3))),
            ]),
        ],
        Group::Referencias => {
            let fam = |ks: &[GeoKind]| -> Vec<Item> {
                ks.iter().map(|k| {
                    let (i, n, b, a) = k.info();
                    it(i, n, b, a, Cmd(C::Geo(*k)))
                }).collect()
            };
            vec![
                Family("plano", fam(&geo_ui::PLANES)),
                Family("ponto", fam(&geo_ui::POINTS)),
                Family("linha", fam(&geo_ui::LINES)),
                Family("curva", fam(&geo_ui::CURVES)),
                Family("superficie", fam(&geo_ui::SURFACES)),
            ]
        }
        Group::Montagem => vec![
            Family("asm_arquivo", vec![
                it("asm_nova", "Nova", "Nova montagem", "Começa uma montagem vazia (arquivo .fka).", Cmd(C::AsmNew)),
                it("abrir_projeto", "Abrir", "Abrir montagem", "Abre um arquivo .fka.", Cmd(C::AsmOpen)),
                it("salvar", "Salvar", "Salvar montagem  (Ctrl+S)", "Grava a montagem; as peças ficam nos arquivos delas.", Cmd(C::AsmSave)),
            ]),
            Family("asm_inserir", vec![
                it("asm_inserir", "Inserir peça", "Inserir peça existente", "Escolha um ou mais arquivos .fkp; o primeiro componente entra fixo.", Cmd(C::AsmInsert)),
                it("asm_subconjunto", "Subconjunto", "Inserir subconjunto", "Insere outra montagem (.fka) como um componente.", Cmd(C::AsmSubAsm)),
                it("asm_peca_nova", "Peça nova", "Peça nova no contexto", "Cria o arquivo da peça ao lado da montagem e abre para modelar.", Cmd(C::AsmNewPart)),
                it("asm_sub_selecao", "Subconjunto da seleção", "Subconjunto dos selecionados", "Os componentes escolhidos viram uma montagem nova (.fka), inserida no lugar deles; as restrições entre eles vão junto.", Cmd(C::AsmSubFromSel)),
            ]),
            Family("asm_recorte_peca", vec![
                it("asm_recorte_peca", "Recorte pela peça", "Recorte pela peça (entre peças)", "A peça escolhida na árvore (pino, aba, parafuso) recorta as peças clicadas: furo do pino, ranhura da aba. Segue as posições (associativo).", Cmd(C::AsmFeat(0))),
                it("asm_furo_pecas", "Furo entre peças", "Furo atravessando peças", "Clique na face onde o furo começa (Ctrl+clique nas outras peças): o mesmo furo passa por todas, alinhado.", Cmd(C::AsmFeat(1))),
            ]),
            One(it("asm_link", "Copiar geometria", "Copiar geometria de outra peça (link)", "Escolha na árvore a peça que recebe e clique numa face plana de outra peça: a face é publicada automaticamente e entra por link (sem referência circular).", Cmd(C::AsmLink))),
            One(it("asm_fixadores", "Fixadores", "Preencher furos com fixadores", "Insere parafuso, arruela e porca da biblioteca nos furos passantes (com a folga de um tamanho métrico) e nos roscados da montagem. Furos de peças diferentes no mesmo eixo recebem um parafuso só, com o comprimento tirado da espessura atravessada. Rodar de novo não duplica.", Cmd(C::AsmFasteners))),
            One(it("asm_grupo", "Grupo", "Grupo de organização", "Elemento vazio só para organizar a árvore (não muda a geometria).", Cmd(C::AsmGroup))),
            One(it("asm_substituir", "Substituir", "Substituir componente", "Troca a peça mantendo a posição e as restrições (as faces são reencontradas pelo ponto clicado).", Cmd(C::AsmReplace))),
            One(it("asm_repeticao", "Repetição", "Repetição inteligente de componente", "Copia o componente selecionado em padrão linear ou circular (ex.: parafusos); as cópias acompanham o original e mudam com a quantidade.", Cmd(C::AsmPattern))),
            One(it("asm_numerar", "Numerar", "Numeração automática", "Renumera os componentes em sequência, na ordem da árvore.", Cmd(C::AsmNumber))),
            One(it("asm_lista", "Lista de peças", "Lista de peças por nível", "Primeiro nível, só peças (todos os níveis) ou estruturada. Os números de item ficam fixos entre revisões e são os mesmos dos balões e da tabela do desenho.", Cmd(C::AsmBom))),
        ],
        Group::Restricoes => vec![
            One(it("rest_auto", "Automática", "Restrição automática", "Escolha dois elementos: o tipo mais lógico é aplicado (faces de frente = contato; eixos = coincidência…).", Cmd(C::AsmMate(crate::asm_ui::MateCmd::Auto)))),
            Family("rest_tipo", vec![
                it("rest_coincidencia", "Coincidência", "Coincidência", "Alinha eixos, junta pontos ou deixa faces coplanares (direta ou invertida).", Cmd(C::AsmMate(crate::asm_ui::MateCmd::Coincident))),
                it("rest_contato", "Contato", "Contato", "Faces encostadas, de frente uma para a outra, sem folga.", Cmd(C::AsmMate(crate::asm_ui::MateCmd::Contact))),
                it("rest_distancia", "Distância", "Distância", "Separação entre faces, eixos ou pontos (positiva, negativa ou nula; aceita fórmula).", Cmd(C::AsmMate(crate::asm_ui::MateCmd::Distance))),
                it("rest_angulo", "Ângulo", "Ângulo", "Inclinação entre faces ou linhas.", Cmd(C::AsmMate(crate::asm_ui::MateCmd::Angle))),
                it("rest_paralelo", "Paralelo", "Paralelismo", "Faces ou eixos paralelos (rígido).", Cmd(C::AsmMate(crate::asm_ui::MateCmd::Parallel))),
                it("rest_perpendicular", "Perpendicular", "Perpendicularidade", "Faces ou eixos a 90° (rígido).", Cmd(C::AsmMate(crate::asm_ui::MateCmd::Perpendicular))),
            ]),
            Family("rest_engrenagem", vec![
                it("rest_engrenagem", "Engrenagem", "Engrenagem", "Dois eixos: o giro de um vira o giro do outro pela razão (externas: −z1/z2).", Cmd(C::AsmMate(crate::asm_ui::MateCmd::Gear))),
                it("rest_polia", "Polia", "Polia (correia)", "Dois eixos girando no mesmo sentido pela razão dos diâmetros.", Cmd(C::AsmMate(crate::asm_ui::MateCmd::Pulley))),
                it("rest_cremalheira", "Cremalheira", "Pinhão e cremalheira", "Eixo do pinhão e aresta da cremalheira: avanço = mm por volta × voltas.", Cmd(C::AsmMate(crate::asm_ui::MateCmd::RackPinion))),
            ]),
            One(it("rest_fixar", "Fixar", "Fixar componente", "Tira todos os graus de liberdade: vira a referência global.", Cmd(C::AsmFix))),
            One(it("rest_conjunta", "Fixação conjunta", "Fixação conjunta", "As peças selecionadas passam a se mover como um corpo rígido.", Cmd(C::AsmRigid))),
            One(it("rest_alinhar", "Alinhar", "Alinhamento instantâneo", "Posiciona pelos dois elementos escolhidos sem guardar restrição (sem histórico).", Cmd(C::AsmMate(crate::asm_ui::MateCmd::AlignOnce)))),
            One(it("rest_atualizar", "Atualizar", "Resolver restrições", "Recalcula as posições de todos os componentes.", Cmd(C::AsmSolve))),
            One(it("rest_interferencia", "Interferências", "Verificar interferências", "Procura volumes em comum entre os componentes (colisões).", Cmd(C::AsmInterf))),
        ],
        Group::Chapa => vec![
            One(it("sm_parametros", "Parâmetros", "Parâmetros da chapa", "Espessura, raio de dobra e fator K da peça; tabela de dobras.", Cmd(C::SmParams))),
            One(it("sm_parede", "Parede base", "Parede base", "Primeira parede a partir de um perfil fechado do sketch selecionado (lê a espessura global).", Cmd(C::SmWall))),
            Family("sm_aba", vec![
                it("sm_aba", "Aba", "Aba de aresta", "Dobra a partir da aresta selecionada: comprimento, ângulo, raio, recuos laterais e alívio de canto.", Cmd(C::SmFlange)),
                it("sm_bainha", "Bainha", "Bainha", "Dobra de 180° na borda: esmagada (folga 0) ou aberta/circular.", Cmd(C::SmHem)),
            ]),
            Family("sm_linha", vec![
                it("sm_dobra_linha", "Dobra por linha", "Dobra por linha", "Escolha dois vértices na face: a chapa dobra nessa linha (linha de centro), com raio e fator K da chapa.", Cmd(C::SmBend)),
                it("sm_joggle", "Joggle", "Joggle (desvio em Z)", "Desloca a chapa além de uma linha por uma rampa, para sobrepor sem saliência.", Cmd(C::SmJoggle)),
            ]),
            Family("sm_estampo", vec![
                it("sm_estampo", "Estampo", "Estampo (rebaixo/reforço)", "Rebaixa a chapa com o perfil fechado do sketch, mantendo a espessura.", Cmd(C::SmStamp)),
                it("sm_furo_flangeado", "Furo flangeado", "Furo flangeado", "Furo com colar dobrado nos pontos do sketch (rigidez).", Cmd(C::SmFlangedHole)),
                it("sm_veneziana", "Veneziana", "Veneziana (lanceta)", "Escolha dois vértices na face: corte nessa linha e a chapa atrás levantada em rampa (ventilação).", Cmd(C::SmLouver)),
            ]),
            Family("sm_recorte", vec![
                it("sm_recorte", "Recorte", "Recorte na chapa", "Perfil fechado do sketch, normal à chapa: normal, com folga, com saída (ângulo) ou flangeado (colar).", Cmd(C::SmCut)),
                it("sm_alivio_canto", "Alívio de canto", "Alívio de canto", "Clique no canto (vértice): redondo, quadrado ou em gota, atravessando a chapa.", Cmd(C::SmCornerRelief)),
                it("sm_rasgo_circular", "Rasgo circular", "Rasgo circular", "Clique no centro (na face): rasgo em arco com raio, largura, ângulo inicial e varredura.", Cmd(C::SmArcSlot)),
            ]),
            Family("furo", vec![
                it("furo_simples", "Furo", "Furo de passagem", "Furo nos pontos de um sketch na face da chapa (vale dobrado e planificado).", Cmd(C::Hole)),
                it("furo_roscado", "Roscado", "Furo roscado", "Broca de rosca + rosca cosmética.", Cmd(C::HoleTapped)),
            ]),
            Family("filete", vec![
                it("arestas_selecionar", "Raio", "Raio nos cantos da chapa", "Arredonda as arestas escolhidas (cantos da chapa ou das abas); acompanha dobrado e planificado.", Cmd(C::Fillet)),
                it("chanfro", "Chanfro", "Chanfro nos cantos da chapa", "Chanfra as arestas escolhidas; acompanha dobrado e planificado.", Cmd(C::Chamfer)),
            ]),
            Family("sm_aba_perfil", vec![
                it("sm_aba_perfil", "Aba por perfil", "Aba por perfil (aba complexa)", "Cadeia aberta de um sketch (linhas e arcos) com os cantos no raio da chapa, puxada para os dois lados.", Cmd(C::SmProfileFlange)),
                it("sm_aba_curva", "Aba curva", "Aba em aresta curva", "Clique numa aresta (reta ou curva) da chapa: a aba segue as arestas tangentes com o raio de dobra.", Cmd(C::SmCurveFlange)),
                it("sm_aba_superficie", "Aba por superfície", "Aba por superfície (suporte)", "Escolha a superfície de suporte e clique na face da web: a web é relimitada pelo suporte e a aba sobe por ele com o raio (OML, EOP).", Cmd(C::SmSurfFlange)),
            ]),
            One(it("sm_desdobrar", "Dobrar/Desdobrar", "Dobrar / desdobrar", "Alterna a peça entre dobrada e planificada (comprimento desenvolvido pelo fator K).", Cmd(C::SmUnfold))),
        ],
        Group::DesArquivo => vec![
            One(it("des_novo", "Novo", "Novo desenho", "Folha A3 vazia; a origem das vistas é a peça ou montagem aberta (salva).", Cmd(C::Dwg(0)))),
            One(it("des_abrir", "Abrir", "Abrir desenho (.fkd)", "Abre um desenho salvo; as vistas se recalculam pelos arquivos de origem.", Cmd(C::Dwg(1)))),
            One(it("des_salvar", "Salvar", "Salvar desenho   Ctrl+S", "Grava o .fkd (caminhos das peças relativos à pasta do desenho).", Cmd(C::Dwg(2)))),
            Family("des_exportar", vec![
                it("des_pdf", "PDF", "Exportar PDF", "Todas as folhas, vetorial, uma página por folha no tamanho real.", Cmd(C::Dwg(3))),
                it("des_svg", "SVG", "Exportar SVG", "A folha atual em SVG (mm).", Cmd(C::Dwg(4))),
                it("des_dxf", "DXF", "Exportar DXF", "A folha atual em DXF R12, camadas por tipo de linha (visível, fina, oculta, centro).", Cmd(C::Dwg(5))),
            ]),
            One(it("des_folha", "Folha", "Nova folha", "Acrescenta uma folha no mesmo formato da atual.", Cmd(C::Dwg(50)))),
        ],
        Group::DesVistas => vec![
            One(it("des_tres_vistas", "3 vistas", "Frente, superior, lateral e isométrica", "Conjunto padrão em 1º diedro (ABNT/ISO E): clique onde fica a frente.", Cmd(C::Dwg(10)))),
            One(it("des_vista", "Vista", "Vista única", "Vista na direção escolhida no painel (frente, trás, superior, inferior, laterais).", Cmd(C::Dwg(11)))),
            One(it("des_projetada", "Projetada", "Vista projetada", "Selecione uma vista e clique ao lado: fica alinhada a ela.", Cmd(C::Dwg(12)))),
            One(it("des_iso", "Isométrica", "Vista isométrica", "Vista em perspectiva isométrica.", Cmd(C::Dwg(16)))),
            One(it("des_corte", "Corte", "Vista em corte", "Selecione a vista, trace a linha de corte (2 cliques) e posicione: face cortada hachurada.", Cmd(C::Dwg(13)))),
            One(it("des_detalhe", "Detalhe", "Vista de detalhe", "Selecione a vista, clique o centro e o raio, e posicione a ampliação (2× a escala).", Cmd(C::Dwg(14)))),
            One(it("des_planificada", "Planificada", "Vista planificada", "Chapa desdobrada vista de cima (peça de chapa como origem).", Cmd(C::Dwg(15)))),
            One(it("des_esquema", "Esquema", "Esquema cinemático", "Montagem com juntas como origem: símbolos das juntas, barras entre elas e terra nas peças fixas (direção do painel).", Cmd(C::Dwg(17)))),
            One(it("des_vista_camera", "Câmera", "Vista pela câmera do 3D", "Vista na direção em que a peça ou a montagem está no 3D agora (gire no 3D e clique na folha).", Cmd(C::Dwg(18)))),
        ],
        Group::DesCotas => vec![
            Family("des_cota", vec![
                it("des_cota_h", "Horizontal", "Cota horizontal", "Dois vértices (snap nas arestas projetadas).", Cmd(C::Dwg(20))),
                it("des_cota_v", "Vertical", "Cota vertical", "Dois vértices (snap nas arestas projetadas).", Cmd(C::Dwg(21))),
                it("des_cota_al", "Alinhada", "Cota alinhada", "Distância real entre dois vértices na vista.", Cmd(C::Dwg(22))),
            ]),
            One(it("des_cota_r", "Raio", "Cota de raio", "Clique num arco da vista.", Cmd(C::Dwg(23)))),
            One(it("des_cota_d", "Diâmetro", "Cota de diâmetro", "Clique num círculo da vista.", Cmd(C::Dwg(24)))),
            One(it("des_cota_ang", "Ângulo", "Cota angular", "Vértice e um ponto de cada lado.", Cmd(C::Dwg(25)))),
        ],
        Group::DesAnotacoes => vec![
            One(it("des_nota", "Nota", "Nota de texto", "Texto livre (várias linhas); chamada opcional no painel.", Cmd(C::Dwg(30)))),
            Family("des_balao", vec![
                it("des_balao", "Balão", "Balão de item", "Ponta na peça e posição do balão: número do item da lista de materiais.", Cmd(C::Dwg(31))),
                it("des_baloes_auto", "Balões auto", "Balões automáticos", "Um balão por peça da montagem, na ordem da lista de materiais.", Cmd(C::Dwg(35))),
            ]),
            One(it("des_rugosidade", "Rugosidade", "Símbolo de rugosidade", "√ com o valor (ex.: Ra 1,6) e chamada para a superfície.", Cmd(C::Dwg(32)))),
            One(it("des_solda", "Solda", "Símbolo de solda", "Linha de referência com o filete e a medida (ex.: a4).", Cmd(C::Dwg(33)))),
            One(it("des_centro", "Centro", "Marca de centro", "Cruz com linhas de centro traço-ponto.", Cmd(C::Dwg(34)))),
            One(it("des_gdt", "GD&T", "Tolerância geométrica", "Quadro de controle (ISO 1101) com chamada presa à aresta da vista: acompanha a peça e a vista.", Cmd(C::Dwg(36)))),
            One(it("des_datum", "Referência", "Símbolo de referência (datum)", "Triângulo na aresta e a letra no quadro; preso à vista.", Cmd(C::Dwg(37)))),
        ],
        Group::DesTabelas => vec![
            One(it("des_bom", "Materiais", "Lista de materiais (BOM)", "Itens da montagem com código, descrição, material e quantidade.", Cmd(C::Dwg(40)))),
            One(it("des_tab_furos", "Furos", "Tabela de furos", "Cada furo da peça com posição, diâmetro, profundidade e rosca.", Cmd(C::Dwg(41)))),
            One(it("des_tab_dobras", "Dobras", "Tabela de dobras", "Ângulo, raio, comprimento desenvolvido e dedução de cada dobra.", Cmd(C::Dwg(42)))),
            One(it("des_tab_corte", "Corte", "Lista de corte", "Perfis estruturais com comprimento e quantidade.", Cmd(C::Dwg(43)))),
        ],
        Group::Pmi => vec![
            Family("pmi_cota", vec![
                it("pmi_linear", "Cota", "Cota linear 3D", "Escolha dois vértices/arestas: a distância fica anotada no modelo e se atualiza com ele.", Cmd(C::Pmi(0))),
                it("pmi_raio", "Raio", "Cota de raio 3D", "Escolha uma aresta circular.", Cmd(C::Pmi(1))),
                it("pmi_diametro", "Diâmetro", "Cota de diâmetro 3D", "Escolha uma aresta circular.", Cmd(C::Pmi(2))),
                it("pmi_angulo", "Ângulo", "Cota angular 3D", "Escolha o vértice e dois pontos.", Cmd(C::Pmi(3))),
            ]),
            Family("pmi_anotacao", vec![
                it("pmi_nota", "Nota", "Nota com chamada", "Texto livre preso a um ponto do modelo.", Cmd(C::Pmi(4))),
                it("pmi_rugosidade", "Rugosidade", "Símbolo de rugosidade", "Ra e processo de fabricação na face escolhida.", Cmd(C::Pmi(5))),
                it("pmi_solda", "Solda", "Símbolo de solda ISO 2553", "Símbolo completo na junta escolhida: lado da seta/outro lado/dois lados, medida, comprimento, acabamento, todo o contorno, solda de campo e processo na cauda.", Cmd(C::Pmi(6))),
            ]),
            Family("pmi_gdt", vec![
                it("pmi_referencia", "Referência", "Referência (datum)", "Letra de referência na face escolhida.", Cmd(C::Pmi(7))),
                it("pmi_tolerancia", "Tolerância", "Tolerância geométrica", "Quadro ISO 1101: característica, tolerância (zona Ø, modificadores Ⓜ Ⓛ Ⓟ…), referências e quadro composto.", Cmd(C::Pmi(8))),
            ]),
            Family("pmi_legenda", vec![
                it("pmi_legenda", "Legenda 3D", "Legenda (carimbo) 3D", "Quadro com os dados da peça preenchidos sozinhos: nome, código, descrição, material, massa, data… (modelo editável).", Cmd(C::Pmi(9))),
                it("pmi_baloes", "Balões", "Balões automáticos (montagem)", "Um balão 3D por componente com o número fixo da lista de peças; refaz os balões anteriores.", Cmd(C::Pmi(10))),
            ]),
        ],
        Group::Estrutural => {
            let g = |k: GeoKind| {
                let (i, n, b, a) = k.info();
                it(i, n, b, a, Cmd(C::Geo(k)))
            };
            let fam = |ks: &[GeoKind]| -> Vec<Item> { ks.iter().map(|k| g(*k)).collect() };
            vec![
            // esqueleto: arame 3D direto daqui
            Family("wf_ponto", fam(&[GeoKind::PtCoords, GeoKind::PtOnCurve, GeoKind::PtBetween, GeoKind::PtOffset, GeoKind::PtCenter, GeoKind::PtProject])),
            Family("wf_linha", fam(&[GeoKind::LinePts, GeoKind::LineDir, GeoKind::LineNormal, GeoKind::LineAngle, GeoKind::LinePlanes])),
            One(g(GeoKind::Polyline)),
            Family("wf_plano", fam(&geo_ui::PLANES)),
            Family("wf_spline", fam(&[GeoKind::Spline, GeoKind::Connect, GeoKind::Corner, GeoKind::CurveJoin])),
            One(it("wf_repetir", "Repetir pontos", "Repetir pontos na curva", "Selecione uma curva e crie N pontos associativos ao longo dela (por quantidade ou espaçamento), com planos normais opcionais em cada ponto.", Cmd(C::PointRepeat))),
            One(it("st_membro", "Membro", "Membro estrutural", "Perfil ao longo das linhas selecionadas (arame 3D): família, medida, ancoragem, rotação, deslocamentos e cantos.", Cmd(C::StMember))),
            Family("st_ligacao", vec![
                it("st_placa", "Placa", "Placa de extremidade", "Fecha a ponta do membro mais perto do vértice selecionado (base de fixação).", Cmd(C::StPlate)),
                it("st_reforco", "Reforço", "Reforço (gusset)", "Chapa triangular ou com chanfro de alívio no encontro de dois membros.", Cmd(C::StGusset)),
            ]),
            One(it("st_biblioteca", "Biblioteca", "Biblioteca de perfis", "Cadastre famílias de perfis padronizados (tamanhos, ancoragem e rotação padrão).", Cmd(C::StLibrary))),
            One(it("st_corte", "Lista de corte", "Lista de corte", "Comprimento, quantidade e barras de matéria-prima (emendas) de cada perfil.", Cmd(C::StCutList))),
            One(it("st_montagem", "Gerar montagem", "Gerar montagem da estrutura", "Uma peça por grupo de peças iguais e uma montagem com as instâncias posicionadas, na pasta da peça (gerar de novo atualiza).", Cmd(C::StAssembly))),
            ]
        }
        Group::Cinematica => {
            let j = |icon, name, brief, adv, k: u8| it(icon, name, brief, adv, Cmd(C::AsmJoint(k)));
            vec![
                Family("junta", vec![
                    j("junta_revolucao", "Revolução", "Junta de revolução", "Gira em torno de um eixo (escolha os dois eixos/faces cilíndricas). Recebe comando de ângulo.", 0),
                    j("junta_prismatica", "Prismática", "Junta prismática", "Desliza ao longo de um trilho (eixos/arestas retas). Recebe comando de curso.", 1),
                    j("junta_cilindrica", "Cilíndrica", "Junta cilíndrica", "Gira e desliza no mesmo eixo.", 2),
                    j("junta_parafuso", "Parafuso", "Junta de parafuso", "Rotação e avanço ligados pelo passo.", 3),
                    j("junta_esferica", "Esférica", "Junta esférica", "Três rotações livres no ponto (escolha dois vértices/pontos).", 4),
                    j("junta_planar", "Planar", "Junta planar", "Duas translações e uma rotação sobre o plano (duas faces planas).", 5),
                    j("junta_rigida", "Rígida", "Junta rígida", "Sem graus de liberdade entre as duas peças.", 6),
                    j("junta_ponto_curva", "Ponto em curva", "Junta ponto em curva", "O vértice da segunda peça segue a aresta (trajetória) da primeira.", 7),
                    j("junta_came", "Came", "Junta de came", "Aresta do perfil do came e depois o seguidor (vértice ou aresta circular do rolete): o seguidor fica encostado no perfil.", 8),
                ]),
                One(it("mec_auto", "Do conjunto", "Mecanismo a partir das restrições", "Converte as restrições de cada par de peças em juntas equivalentes (eixo + face = revolução…).", Cmd(C::AsmAutoJoints))),
                One(it("composer", "Exibição", "Estados de exibição, explosão e animação", "Salve o que aparece (cores, ocultos, explosão, câmera), exploda a montagem e anime por quadros-chave — os estados alimentam o detalhamento.", Cmd(C::AsmComposer))),
                One(it("mec_simular", "Simular", "Simulação cinemática", "Comandos por cursor ou por leis no tempo (t), quadros, velocidade e aceleração; análise de graus de liberdade.", Cmd(C::AsmSimulate))),
            ]
        }
        Group::Wireframe => {
            let g = |k: GeoKind| {
                let (i, n, b, a) = k.info();
                it(i, n, b, a, Cmd(C::Geo(k)))
            };
            let fam = |ks: &[GeoKind]| -> Vec<Item> { ks.iter().map(|k| g(*k)).collect() };
            vec![
                Family("wf_ponto", fam(&[GeoKind::PtCoords, GeoKind::PtOnCurve, GeoKind::PtOnSurface, GeoKind::PtOnPlane, GeoKind::PtCenter, GeoKind::PtBetween, GeoKind::PtProject, GeoKind::PtCurvePlane, GeoKind::PtCurveCurve, GeoKind::PtOffset, GeoKind::PtTangent, GeoKind::PtProjectCurve])),
                Family("wf_linha", fam(&[GeoKind::LinePts, GeoKind::LineDir, GeoKind::LineNormal, GeoKind::LineAngle, GeoKind::LineTangent, GeoKind::LinePlanes, GeoKind::LineBisector, GeoKind::LineAxis])),
                One(it("wf_repetir", "Repetir pontos", "Repetir pontos na curva", "Selecione uma curva e crie N pontos associativos ao longo dela (por quantidade ou espaçamento), com planos normais opcionais em cada ponto.", Cmd(C::PointRepeat))),
                One(g(GeoKind::Polyline)),
                One(g(GeoKind::CurveJoin)),
                Family("wf_plano", fam(&geo_ui::PLANES)),
                Family("wf_circulo", fam(&[GeoKind::Circle, GeoKind::Circle3, GeoKind::Conic])),
                Family("wf_spline", fam(&[GeoKind::Spline, GeoKind::SplineControl, GeoKind::Connect, GeoKind::Corner])),
                Family("wf_helice", fam(&[GeoKind::Helix, GeoKind::Spiral, GeoKind::Spine])),
                Family("wf_projecao", fam(&[GeoKind::CurveProjSurf, GeoKind::CurveProject, GeoKind::PtProject])),
                One(g(GeoKind::CurveIntersect)),
                One(g(GeoKind::CurveParallel)),
                One(it("extrair", "Extrair", "Extrair a seleção (faces, arestas, vértices)", "Faces, arestas e vértices viram superfície, curva ou ponto de referência. Simples = um elemento por item; múltipla = um só. Propagação por ponto, tangente ou curvatura; o resultado pode ser publicado.", Cmd(C::Extract))),
            ]
        }
        Group::Superficies => {
            let g = |k: GeoKind| {
                let (i, n, b, a) = k.info();
                it(i, n, b, a, Cmd(C::Geo(k)))
            };
            vec![
                Family("sup_basica", vec![g(GeoKind::SurfExtrude), g(GeoKind::SurfRevolve), g(GeoKind::SurfSphere), g(GeoKind::SurfCylinder)]),
                One(g(GeoKind::SurfOffset)),
                Family("sup_varrer", vec![g(GeoKind::SurfSweep), g(GeoKind::SurfSweepX), g(GeoKind::SurfTube)]),
                Family("sup_multi", vec![g(GeoKind::SurfLoft), g(GeoKind::SurfBlend), g(GeoKind::SurfConnect)]),
                Family("sup_filete", vec![g(GeoKind::SurfFilletSS), g(GeoKind::SurfEdgeFillet)]),
                One(g(GeoKind::SurfFill)),
            ]
        }
        Group::Operacoes => {
            let g = |k: GeoKind| {
                let (i, n, b, a) = k.info();
                it(i, n, b, a, Cmd(C::Geo(k)))
            };
            vec![
                Family("op_juntar", vec![g(GeoKind::OpJoin), g(GeoKind::OpHeal)]),
                Family("op_dividir", vec![g(GeoKind::OpSplit), g(GeoKind::OpTrim)]),
                Family("op_extrair", vec![
                    it("extrair", "Extrair", "Extrair a seleção (faces, arestas, vértices)", "Faces, arestas e vértices viram superfície, curva ou ponto de referência. Simples = um elemento por item; múltipla = um só. Propagação por ponto, tangente ou curvatura; o resultado pode ser publicado.", Cmd(C::Extract)),
                    g(GeoKind::SurfExtract),
                    g(GeoKind::OpMulti),
                    g(GeoKind::OpBoundary),
                ]),
                Family("op_desmontar", vec![g(GeoKind::OpUntrim), g(GeoKind::OpPart)]),
                One(g(GeoKind::OpExtrapolate)),
                Family("xf", vec![g(GeoKind::XfTranslate), g(GeoKind::XfRotate), g(GeoKind::XfMirror), g(GeoKind::XfScale)]),
            ]
        }
        Group::Hibrido => vec![
            One(it("hib_dividir", "Dividir corpo", "Dividir o sólido por superfície (Split)", "Corta o corpo com uma superfície, face ou plano e mantém um lado.", Cmd(C::SplitBody))),
            Family("hib_engrossar", vec![
                it("hib_engrossar", "Engrossar", "Engrossar superfície (Thick Surface / Thicken)", "Transforma a superfície em sólido com espessura para um ou dois lados.", Cmd(C::ThickSurface)),
                it("hib_fechar", "Fechar", "Fechar superfície (Close Surface / Stitch)", "Costura as superfícies e tampa as bordas para formar um sólido.", Cmd(C::CloseSurface)),
            ]),
            One(it("hib_costurar", "Costurar", "Costurar superfície no corpo (Sew Surface)", "Acrescenta o volume entre a superfície e o corpo, ou remove o lado escolhido.", Cmd(C::SewSurface))),
            One(it("hib_preencher", "Preencher", "Preencher fronteiras (Boundary Fill)", "Sólido dos volumes fechados por superfícies, faces e planos; escolha as células que ficam de fora.", Cmd(C::BoundaryFill))),
            One(it("hib_malha", "Malha", "Importar malha STL/OBJ como B-rep", "Converte triângulos em faces costuradas; se fechar, vira sólido.", Cmd(C::MeshImport))),
        ],
        Group::Analise => vec![
            One(it("an_zebra", "Zebra", "Listras zebra (reflexo)", "Faixas de reflexo: onde elas quebram há só posição (G0); dobram = tangência (G1); contínuas = curvatura (G2).", Display(DisplayMode::Zebra))),
            One(it("an_curvatura", "Curvatura", "Mapa de curvatura", "Azul = plano, verde/amarelo = curvatura média, vermelho = raio pequeno.", Display(DisplayMode::Curvatura))),
            One(it("an_sombreado", "Normal", "Voltar ao sombreado com arestas", "Sai da análise.", Display(DisplayMode::SombreadoArestas))),
        ],
        Group::Organizacao => vec![
            One(it("parametro", "Parâmetro", "Novo parâmetro", "Parâmetro global da peça (nome = expressão), usável em qualquer cota ou comando.", Cmd(C::Param))),
            One(it("pasta", "Pasta", "Nova pasta", "Organiza comandos na árvore; não muda a ordem de execução.", Cmd(C::Folder))),
            One(it("macro", "Macro", "Automação por macro", "Escreva e execute macros que criam a peça (parâmetros, sketches, extrusões, furos, padrões, laços).", Macros)),
            One(it("coletor", "Coletor", "Novo coletor de corpos", "Grupo de corpos independente; vira o ativo para os próximos comandos.", Cmd(C::Collector))),
        ],
        Group::Selecao => vec![
            One(it("sel_filtro", "Limpar", "Limpar seleção  (Esc)", "Desmarca faces, arestas e vértices selecionados na viewport.", ClearPicks)),
            Family("sel_caixa", vec![
                it("sel_clique", "Clique", "Seleção por clique (Ctrl + arrastar = caixa)", "Clique seleciona; Shift/Ctrl somam. Ctrl + arrastar faz caixa: da esquerda para a direita só o que está inteiro dentro, da direita para a esquerda o que toca.", SelMode(0)),
                it("sel_caixa_janela", "Janela", "Caixa janela (inteiro dentro)", "Arrastar com o esquerdo faz a caixa: entra só o que está inteiramente dentro. Shift soma à seleção.", SelMode(1)),
                it("sel_caixa_cruzada", "Cruzada", "Caixa cruzada (parcialmente dentro)", "Arrastar com o esquerdo faz a caixa: entra tudo o que toca a caixa. Shift soma à seleção.", SelMode(2)),
                it("sel_caixa_auto", "Automática", "Caixa pela direção", "Arrastar com o esquerdo faz a caixa: para a direita = janela, para a esquerda = cruzada (como no AutoCAD).", SelMode(3)),
            ]),
        ],
        Group::Vista => vec![
            One(it("ajustar", "Ajustar", "Ajustar à tela  (F)", "Enquadra todo o modelo.", Fit)),
            Family("vistas", vec![
                it("vista_iso", "Iso", "Vista isométrica  (Ctrl+7)", "", View(-0.785, 0.6)),
                it("vista_frente", "Frente", "Vista frontal  (Ctrl+1)", "", View(-1.5708, 0.0)),
                it("vista_topo", "Topo", "Vista superior  (Ctrl+5)", "", View(-1.5708, 1.5706)),
                it("vista_direita", "Direita", "Vista lateral direita  (Ctrl+3)", "", View(0.0, 0.0)),
            ]),
            One(it("ortho", "Ortogonal", "Projeção ortogonal  (O)", "Perspectiva ou projeção ortogonal (ideal para sketches).", Ortho)),
        ],
        Group::SkCriacao => vec![
            One(tool(Tool::Line)),
            One(tool(Tool::Spline)),
            Family("sk_ret", vec![tool(Tool::Rect), tool(Tool::RectCenter)]),
            Family("sk_circ", vec![tool(Tool::Circle), tool(Tool::Circle3)]),
            Family("sk_arco", vec![tool(Tool::Arc3), tool(Tool::ArcCenter)]),
            Family("sk_formas", vec![tool(Tool::Slot), tool(Tool::Polygon)]),
            One(tool(Tool::Point)),
        ],
        Group::SkTransformar => vec![
            Family("sk_mover", vec![tool(Tool::Move), tool(Tool::Copy)]),
            One(tool(Tool::Mirror)),
            One(tool(Tool::Offset)),
            Family("sk_filete", vec![tool(Tool::Fillet), tool(Tool::Chamfer)]),
            Family("sk_aparar", vec![tool(Tool::Trim), tool(Tool::Extend)]),
            Family("sk_projetar", vec![tool(Tool::Convert), tool(Tool::Intersect)]),
            One(it("sk_converter", "Construção", "Converter seleção ↔ construção", "Transforma as curvas selecionadas em geometria de construção (tracejada) ou de volta em geometria normal.", SkToggleConstruction)),
        ],
        Group::SkCotagem => vec![Family("sk_cota", vec![
            tool(Tool::Dimension),
            tool(Tool::DimH),
            tool(Tool::DimV),
            tool(Tool::DimAngle),
            tool(Tool::DimRadius),
            tool(Tool::DimDiameter),
        ])],
        Group::SkRestricoes => vec![
            Family("r_coinc", vec![tool(Tool::Constr(CKind::Coincident)), tool(Tool::Constr(CKind::Midpoint)), tool(Tool::Constr(CKind::Fixed))]),
            Family("r_hv", vec![tool(Tool::Constr(CKind::Horizontal)), tool(Tool::Constr(CKind::Vertical))]),
            Family("r_par", vec![tool(Tool::Constr(CKind::Parallel)), tool(Tool::Constr(CKind::Perpendicular))]),
            Family("r_tan", vec![tool(Tool::Constr(CKind::Tangent)), tool(Tool::Constr(CKind::Concentric))]),
            One(tool(Tool::Constr(CKind::Equal))),
            One(tool(Tool::Constr(CKind::Symmetric))),
        ],
        Group::SkFerramentas => vec![
            One(tool(Tool::Select)),
            One(it("sk_checar", "Checar", "Checar contornos", "Destaca pontas abertas, pontos sobrepostos e conta os perfis fechados.", SkCheck)),
            One(it("sk_construcao", "Modo constr.", "Desenhar como construção  (X)", "As próximas curvas serão de construção.", SkConstruction)),
            One(it("sk_grade", "Grade", "Grade  (G)", "Mostra/oculta a grade do plano.", SkGrid)),
            One(it("sk_snap", "Snaps", "Snaps", "Atrai o cursor para pontos, origem e grade.", SkSnap)),
            One(it("sk_auto_restricao", "Auto restr.", "Restrições automáticas", "H/V em linhas quase alinhadas e coincidência ao clicar em pontos existentes.", SkAutoConstraints)),
            One(it("sk_auto_cota", "Auto cotas", "Cotas automáticas", "Cada forma criada recebe suas cotas.", SkAutoDims)),
            One(it("sk_concluir", "Concluir", "Concluir sketch  (Esc)", "Sai do modo sketch.", SkFinish)),
        ],
    }
}

/// Estado necessário para marcar botões selecionados.
pub struct ToolState {
    pub hidden_env: bool,
    pub sketch_tool: Option<Tool>,
    pub construction: bool,
    pub check: bool,
}

fn is_selected(a: Act, s: &Settings, st: &ToolState) -> bool {
    match a {
        Act::Ortho => s.ortho,
        Act::SkTool(t) => st.sketch_tool == Some(t),
        Act::SkConstruction => st.construction,
        Act::SkCheck => st.check,
        Act::SkGrid => s.grid,
        Act::SkSnap => s.snaps,
        Act::SkAutoConstraints => s.auto_constraints,
        Act::SkAutoDims => s.auto_dims,
        Act::HiddenEnv => st.hidden_env,
        Act::Display(m) => s.display == m,
        _ => false,
    }
}

/// Mostra um grupo; devolve as ações clicadas.
/// Índice do bloco desenhado nesta passada (alterna as cores do tema); zerado a cada quadro.
pub static BLOCK_IDX: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Cores do tema dos menus: (fundo par, fundo ímpar, destaque do nome do grupo).
pub fn theme_colors(theme: u8, dark: bool) -> Option<(egui::Color32, egui::Color32, egui::Color32)> {
    use egui::Color32 as C;
    if theme == 4 {
        return crate::theme::bands();
    }
    Some(match (theme, dark) {
        (1, true) => (C::from_rgb(28, 30, 34), C::from_rgb(44, 47, 54), C::from_rgb(170, 176, 188)),
        (1, false) => (C::from_rgb(226, 229, 234), C::from_rgb(240, 242, 246), C::from_rgb(70, 76, 88)),
        (2, true) => (C::from_rgb(26, 22, 24), C::from_rgb(52, 22, 28), C::from_rgb(228, 76, 76)),
        (2, false) => (C::from_rgb(244, 234, 234), C::from_rgb(232, 210, 212), C::from_rgb(170, 30, 36)),
        (3, true) => (C::from_rgb(20, 28, 42), C::from_rgb(30, 44, 68), C::from_rgb(100, 170, 255)),
        (3, false) => (C::from_rgb(226, 236, 250), C::from_rgb(240, 245, 252), C::from_rgb(30, 90, 190)),
        _ => return None,
    })
}

pub fn show_group(ui: &mut egui::Ui, g: Group, s: &mut Settings, st: &ToolState, vertical: bool) -> Vec<Act> {
    let list = custom_entries(g, s);
    show_block(ui, Some(g), g.label().to_string(), list, s, st, vertical)
}

fn show_block(ui: &mut egui::Ui, g: Option<Group>, title: String, list: Vec<Entry>, s: &mut Settings, st: &ToolState, vertical: bool) -> Vec<Act> {
    let mut acts = vec![];
    let size = s.icon_size;
    let labels = s.show_labels && !vertical;
    let menu_theme = s.menu_theme;
    let mut body = |ui: &mut egui::Ui| {
        for e in list.clone() {
            match e {
                Entry::One(i) => {
                    if tool_button(ui, i.icon, i.name, size, labels, is_selected(i.act, s, st), i.brief, i.advanced).clicked() {
                        acts.push(i.act);
                    }
                }
                Entry::Family(key, items) => {
                    let cur = s.family(key).min(items.len() - 1);
                    let i = items[cur];
                    let any_sel = items.iter().any(|x| is_selected(x.act, s, st));
                    let sel = is_selected(i.act, s, st) || any_sel;
                    if tool_button(ui, i.icon, i.name, size, labels, sel, i.brief, i.advanced).clicked() {
                        acts.push(i.act);
                    }
                    // setinha: abre as variações da família
                    let arrow = ui.add(egui::Button::new(RichText::new(tr("▾")).size(11.)).frame(false).min_size(egui::vec2(10., size)));
                    let arrow = arrow.on_hover_text(tr("variações"));
                    egui::Popup::menu(&arrow).show(|ui| {
                        for (k, v) in items.iter().enumerate() {
                            let r = ui.horizontal(|ui| {
                                ui.add(egui::Image::new(crate::icons::get(v.icon)).fit_to_exact_size(egui::vec2(22., 22.)));
                                ui.selectable_label(k == cur, RichText::new(tr(v.brief)))
                            });
                            if r.inner.clicked() {
                                s.set_family(key, k);
                                acts.push(v.act);
                                ui.close();
                            }
                        }
                    });
                }
            }
        }
        if g == Some(Group::Vista) {
            let r = tool_button(ui, "estilo", "Estilo", size, labels, false, "Estilo de exibição", "Sombreado com/sem arestas, linhas ocultas, arame, raio-X ou semi-renderizado.");
            egui::Popup::menu(&r).show(|ui| {
                for m in DisplayMode::ALL {
                    ui.radio_value(&mut s.display, m, m.label());
                }
                ui.separator();
                ui.checkbox(&mut s.edges_in_render, tr("arestas no semi-renderizado"));
                ui.checkbox(&mut s.tangent_edges, tr("arestas de tangência (fim dos raios)"));
            });
            let r = tool_button(ui, "filtro", "Exibir", size, labels, false, "Filtro de exibição", "Escolha quais tipos de elemento aparecem na viewport.");
            egui::Popup::menu(&r).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside).show(|ui| {
                let f = &mut s.filter;
                ui.label(RichText::new(tr("Mostrar na viewport")).strong());
                ui.checkbox(&mut f.corpos, tr("Corpos"));
                ui.checkbox(&mut f.sketches, tr("Sketches"));
                ui.checkbox(&mut f.pontos, tr("Pontos de sketch"));
                ui.checkbox(&mut f.planos_origem, tr("Planos da Origem"));
                ui.checkbox(&mut f.planos_ref, tr("Planos de referência"));
                ui.checkbox(&mut f.eixos, tr("Eixos da Origem"));
                ui.checkbox(&mut f.rotulos, tr("Rótulos dos planos"));
            });
            if tool_button(ui, "ocultos", "Ocultos", size, labels, st.hidden_env, "Ambiente de ocultos  (Shift+H)", "Mostra só os itens ocultos, para reexibir o que precisar.").clicked() {
                acts.push(Act::HiddenEnv);
            }
            if tool_button(ui, "config", "Ajustes", size, labels, false, "Configurações", "Tema, fundo, navegação, barras, cotagem e sketch.").clicked() {
                acts.push(Act::Settings);
            }
        }
        if g == Some(Group::Selecao) {
            for (icon, name, flag) in [("sel_face", "Faces", 0), ("sel_aresta", "Arestas", 1), ("sel_vertice", "Vértices", 2)] {
                let on = [s.sel_faces, s.sel_edges, s.sel_vertices][flag];
                if tool_button(ui, icon, name, size, labels, on, &format!("Selecionar {}", name.to_lowercase()), "Filtro de seleção: liga/desliga este tipo de elemento ao clicar na viewport.").clicked() {
                    match flag {
                        0 => s.sel_faces = !s.sel_faces,
                        1 => s.sel_edges = !s.sel_edges,
                        _ => s.sel_vertices = !s.sel_vertices,
                    }
                }
            }
        }
    };
    let mut undock: Option<Group> = None;
    // tema: blocos com fundos alternados e o nome do grupo numa faixa de destaque
    let dark = ui.visuals().dark_mode;
    let theme = theme_colors(menu_theme, dark);
    let k = BLOCK_IDX.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let frame = match theme {
        Some((a, b, _)) => egui::Frame::new().fill(if k % 2 == 0 { a } else { b }).corner_radius(6.).inner_margin(egui::Margin::symmetric(5, 3)),
        None => egui::Frame::new(),
    };
    let accent = theme.map(|t| t.2);
    let inner = if vertical {
        frame.show(ui, |ui| ui.vertical_centered(|ui| {
            // alça: arrastar solta o grupo (fica sob o cursor e encaixa onde soltar)
            if let Some(g) = g {
                let (r, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width().max(30.), 10.), egui::Sense::drag());
                let c = accent.unwrap_or(ui.visuals().weak_text_color());
                for k in [-1., 0., 1.] {
                    ui.painter().circle_filled(r.center() + egui::vec2(k * 5., 0.), 1.6, c);
                }
                let resp = resp.on_hover_text(format!("{} — {}", title, tr("arraste para soltar o grupo; solte numa borda para encaixar")));
                if resp.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                }
                if resp.drag_started() {
                    undock = Some(g);
                }
            }
            // barra larga: os botões se distribuem em colunas
            if ui.available_width() > size * 2.6 {
                ui.horizontal_wrapped(|ui| body(ui));
            } else {
                body(ui);
            }
        }).response).inner
    } else {
        // bloco: botões em cima, nome do grupo embaixo
        frame.show(ui, |ui| ui.vertical(|ui| {
            let row = ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| body(ui)).response;
            // nome do grupo centralizado sob os botões, com a largura do próprio bloco
            let (r, resp) = ui.allocate_exact_size(egui::vec2(row.rect.width(), 14.), egui::Sense::drag());
            if let Some(c) = accent {
                ui.painter().rect_filled(r.shrink2(egui::vec2(2., 1.)), 3., c.gamma_multiply(0.22));
                ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, &title, egui::FontId::proportional(10.5), c);
            } else {
                ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, &title, egui::FontId::proportional(10.5), ui.visuals().weak_text_color());
            }
            // arrastar o nome do grupo solta o bloco (vira flutuante; encaixa de novo ao soltar perto de uma borda)
            if let Some(g) = g {
                let resp = resp.on_hover_text(tr("arraste para soltar o grupo; solte perto do topo ou das laterais para encaixar"));
                if resp.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                }
                if resp.drag_started() {
                    undock = Some(g);
                }
            }
        })
        .response)
        .inner
    };
    if let Some(g) = undock {
        acts.push(Act::DockDrag(g, inner.rect));
    }
    inner.context_menu(|ui| {
        ui.label(RichText::new(&title).strong());
        if let Some(g) = g {
            for d in Dock::ALL {
                if ui.radio(s.dock(g) == d, d.label()).clicked() {
                    acts.push(Act::Dock(g, d));
                    ui.close();
                }
            }
            ui.separator();
        }
        if ui.button(tr("⚙ Personalizar comandos…")).clicked() {
            acts.push(Act::Customize);
            ui.close();
        }
    });
    acts
}

/// Grupos visíveis com determinada posição: no modo sketch, os de sketch + Vista.
pub fn groups_at(s: &Settings, d: Dock, sketching: bool) -> Vec<Group> {
    Group::ALL
        .iter()
        .copied()
        .filter(|g| s.dock(*g) == d)
        // restrições ficam no menu rápido do botão direito (smart)
        .filter(|g| *g != Group::SkRestricoes)
        .filter(|g| if sketching { g.sketch_only() || *g == Group::Vista } else { !g.sketch_only() && g.in_bench(s.bench) })
        .collect()
}
