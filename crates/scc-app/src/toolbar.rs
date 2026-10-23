//! Barra de comandos em **blocos** (estilo faixa): cada grupo tem nome embaixo e linhas separadoras.
//! Comandos parecidos formam **famílias**: o botão mostra a última variação usada e a setinha ▾
//! abre as outras. Grupos podem ficar no topo, na lateral, flutuar ou ser ocultados (clique direito).

use crate::app::Cmd;
use crate::geo_ui::{self, GeoKind};
use scc_core::{BoolOp, PrimKind};
use crate::icons::tool_button;
use crate::render::DisplayMode;
use crate::settings::{Dock, Group, Settings};
use crate::sketch_ui::{CKind, Tool};
use eframe::egui::{self, RichText};
use crate::i18n::tr;

/// Ação pedida por um botão de ferramenta.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Act {
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
                it("asm_nova", "Nova", "Nova montagem", "Começa uma montagem vazia (arquivo .sccasm).", Cmd(C::AsmNew)),
                it("abrir_projeto", "Abrir", "Abrir montagem", "Abre um arquivo .sccasm.", Cmd(C::AsmOpen)),
                it("salvar", "Salvar", "Salvar montagem  (Ctrl+S)", "Grava a montagem; as peças ficam nos arquivos delas.", Cmd(C::AsmSave)),
            ]),
            Family("asm_inserir", vec![
                it("asm_inserir", "Inserir peça", "Inserir peça existente", "Escolha um ou mais arquivos .sccpart; o primeiro componente entra fixo.", Cmd(C::AsmInsert)),
                it("asm_subconjunto", "Subconjunto", "Inserir subconjunto", "Insere outra montagem (.sccasm) como um componente.", Cmd(C::AsmSubAsm)),
                it("asm_peca_nova", "Peça nova", "Peça nova no contexto", "Cria o arquivo da peça ao lado da montagem e abre para modelar.", Cmd(C::AsmNewPart)),
            ]),
            One(it("asm_grupo", "Grupo", "Grupo de organização", "Elemento vazio só para organizar a árvore (não muda a geometria).", Cmd(C::AsmGroup))),
            One(it("asm_substituir", "Substituir", "Substituir componente", "Troca a peça mantendo a posição e as restrições (as faces são reencontradas pelo ponto clicado).", Cmd(C::AsmReplace))),
            One(it("asm_repeticao", "Repetição", "Repetição inteligente de componente", "Copia o componente selecionado em padrão linear ou circular (ex.: parafusos); as cópias acompanham o original e mudam com a quantidade.", Cmd(C::AsmPattern))),
            One(it("asm_numerar", "Numerar", "Numeração automática", "Renumera os componentes em sequência, na ordem da árvore.", Cmd(C::AsmNumber))),
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
            One(it("sm_joggle", "Joggle", "Joggle (desvio em Z)", "Desloca a chapa além de uma linha por uma rampa, para sobrepor sem saliência.", Cmd(C::SmJoggle))),
            Family("sm_estampo", vec![
                it("sm_estampo", "Estampo", "Estampo (rebaixo/reforço)", "Rebaixa a chapa com o perfil fechado do sketch, mantendo a espessura.", Cmd(C::SmStamp)),
                it("sm_furo_flangeado", "Furo flangeado", "Furo flangeado", "Furo com colar dobrado nos pontos do sketch (rigidez).", Cmd(C::SmFlangedHole)),
            ]),
            One(it("sm_desdobrar", "Dobrar/Desdobrar", "Dobrar / desdobrar", "Alterna a peça entre dobrada e planificada (comprimento desenvolvido pelo fator K).", Cmd(C::SmUnfold))),
            One(it("recorte", "Recorte", "Recorte na chapa", "Recorte passante a partir do sketch (compatível com a planificação).", Cmd(C::Cut))),
        ],
        Group::Estrutural => vec![
            One(it("st_membro", "Membro", "Membro estrutural", "Perfil ao longo das linhas selecionadas (arame 3D): família, medida, ancoragem, rotação, deslocamentos e cantos.", Cmd(C::StMember))),
            Family("st_ligacao", vec![
                it("st_placa", "Placa", "Placa de extremidade", "Fecha a ponta do membro mais perto do vértice selecionado (base de fixação).", Cmd(C::StPlate)),
                it("st_reforco", "Reforço", "Reforço (gusset)", "Chapa triangular ou com chanfro de alívio no encontro de dois membros.", Cmd(C::StGusset)),
            ]),
            One(it("st_biblioteca", "Biblioteca", "Biblioteca de perfis", "Cadastre famílias de perfis padronizados (tamanhos, ancoragem e rotação padrão).", Cmd(C::StLibrary))),
            One(it("st_corte", "Lista de corte", "Lista de corte", "Comprimento e quantidade de cada barra, por perfil.", Cmd(C::StCutList))),
        ],
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
                ]),
                One(it("mec_auto", "Do conjunto", "Mecanismo a partir das restrições", "Converte as restrições de cada par de peças em juntas equivalentes (eixo + face = revolução…).", Cmd(C::AsmAutoJoints))),
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
                One(g(GeoKind::Polyline)),
                Family("wf_plano", fam(&geo_ui::PLANES)),
                Family("wf_circulo", fam(&[GeoKind::Circle, GeoKind::Circle3, GeoKind::Conic])),
                Family("wf_spline", fam(&[GeoKind::Spline, GeoKind::SplineControl, GeoKind::Connect, GeoKind::Corner])),
                Family("wf_helice", fam(&[GeoKind::Helix, GeoKind::Spiral, GeoKind::Spine])),
                Family("wf_projecao", fam(&[GeoKind::CurveProjSurf, GeoKind::CurveProject, GeoKind::PtProject])),
                One(g(GeoKind::CurveIntersect)),
                One(g(GeoKind::CurveParallel)),
                One(it("extrair", "Extrair", "Extrair a seleção (faces, arestas, vértices)", "Cada face, aresta ou vértice selecionado na viewport vira superfície, curva ou ponto de referência (várias faces = uma multi-extração).", Cmd(C::Extract))),
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
                Family("sup_varrer", vec![g(GeoKind::SurfSweep), g(GeoKind::SurfTube)]),
                Family("sup_multi", vec![g(GeoKind::SurfLoft), g(GeoKind::SurfBlend)]),
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
                    it("extrair", "Extrair", "Extrair a seleção (faces, arestas, vértices)", "Cada face, aresta ou vértice selecionado vira superfície, curva ou ponto de referência (várias faces = multi-extração).", Cmd(C::Extract)),
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
        Group::Selecao => vec![One(it("sel_filtro", "Limpar", "Limpar seleção  (Esc)", "Desmarca faces, arestas e vértices selecionados na viewport.", ClearPicks))],
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
            One(tool(Tool::Fillet)),
            Family("sk_aparar", vec![tool(Tool::Trim), tool(Tool::Extend)]),
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
pub fn show_group(ui: &mut egui::Ui, g: Group, s: &mut Settings, st: &ToolState, vertical: bool) -> Vec<Act> {
    let list = custom_entries(g, s);
    show_block(ui, Some(g), g.label().to_string(), list, s, st, vertical)
}

fn show_block(ui: &mut egui::Ui, g: Option<Group>, title: String, list: Vec<Entry>, s: &mut Settings, st: &ToolState, vertical: bool) -> Vec<Act> {
    let mut acts = vec![];
    let size = s.icon_size;
    let labels = s.show_labels && !vertical;
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
    let mut undock = None;
    let inner = if vertical {
        ui.vertical_centered(|ui| body(ui)).response
    } else {
        // bloco: botões em cima, nome do grupo embaixo
        ui.vertical(|ui| {
            let row = ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| body(ui)).response;
            // nome do grupo centralizado sob os botões, com a largura do próprio bloco
            let (r, resp) = ui.allocate_exact_size(egui::vec2(row.rect.width(), 14.), egui::Sense::drag());
            ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, &title, egui::FontId::proportional(10.5), ui.visuals().weak_text_color());
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
        .response
    };
    if let Some(g) = undock {
        acts.push(Act::Dock(g, Dock::Flutuante));
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
