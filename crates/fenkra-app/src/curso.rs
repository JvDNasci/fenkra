// SPDX-License-Identifier: LGPL-2.1-or-later
//! Curso dentro do programa (B323): trilhas por nível e bancada, aulas curtas com passos que o
//! próprio documento confirma, peças de exemplo e progresso salvo.
//!
//! Tudo aqui é lógica pura, testável sem janela: o passo é reconhecido olhando o documento (e as
//! contagens da montagem e do desenho), nunca um clique em "feito". Os passos que não deixam rastro
//! no documento (olhar a zebra, conferir um valor) são `Cond::Manual`: a pessoa confirma.
//!
//! Princípios: a pessoa pode pular o passo ou a aula inteira a qualquer momento e retomar depois sem
//! perder nada; o curso nunca abre sozinho depois da escolha inicial; a narração é opcional e o texto
//! está sempre na tela. Nenhuma voz é gerada nem baixada: se existir `voz/<aula>` no pacote de mídia
//! (ADR 0034), a opção fica disponível; sem ela o curso funciona igual.

use crate::settings::Bench;
use fenkra_core::{BoolOp, Document, FeatureKind};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Nivel {
    Iniciante,
    Intermediario,
    Avancado,
}

impl Nivel {
    pub fn texto(self) -> &'static str {
        match self {
            Nivel::Iniciante => "Iniciante",
            Nivel::Intermediario => "Intermediário",
            Nivel::Avancado => "Avançado",
        }
    }
}

/// Como o passo é reconhecido.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cond {
    /// pelo menos `n` comandos com esta etiqueta (ver `etiqueta`) ativos no documento
    Feature(&'static str, usize),
    /// pelo menos `n` elementos de referência que não são planos (pontos, linhas, curvas)
    Referencias(usize),
    /// pelo menos `n` superfícies
    NSuperficies(usize),
    Componentes(usize),
    Restricoes(usize),
    Folhas(usize),
    Vistas(usize),
    Cotas(usize),
    Tabelas(usize),
    /// a peça, a montagem ou o desenho aberto tem este nome (exemplo)
    Exemplo(&'static str),
    /// algum sketch tem uma cota (restrição dimensional que comanda a geometria)
    CotaSketch,
    /// a chapa está planificada (desdobrada)
    Planificada,
    /// pelo menos `n` superfícies resultantes de "Juntar"
    SuperficieJuntada(usize),
    /// alguma forma livre tem um vinco
    FormaVinco,
    /// alguma forma livre já vira corpo B-rep
    FormaConvertida,
    Notas(usize),
    /// vistas da chapa planificada no desenho
    VistasPlanas(usize),
    /// tabelas de dobras no desenho
    TabelasDobras(usize),
    /// o carimbo do desenho tem título ou desenhista
    Carimbo,
    /// o comando com este ícone foi acionado nesta sessão (passo cujo rastro não fica no modelo,
    /// como abrir a lista de peças ou exportar)
    Comando(&'static str),
    /// gesto de câmera ou de seleção feito na vista nesta sessão (`anotar`): girar, mover, zoom,
    /// cubo de vistas, selecionar
    Gesto(&'static str),
    /// sem rastro no documento: a pessoa confirma
    Manual,
}

/// Gestos da vista que a aula de câmera reconhece (B297). Quem navega anota; o curso lê no quadro.
pub const GESTOS: [&str; 5] = ["girar", "mover_vista", "zoom", "vista_cubo", "selecionar"];

static ANOTADOS: std::sync::Mutex<Vec<&'static str>> = std::sync::Mutex::new(Vec::new());

/// Anota um gesto da vista (uma vez por sessão; repetir não custa nada).
pub fn anotar(g: &'static str) {
    if let Ok(mut v) = ANOTADOS.lock() {
        if !v.contains(&g) {
            v.push(g);
        }
    }
}

/// Gestos anotados desde que o programa abriu.
pub fn anotados() -> Vec<&'static str> {
    ANOTADOS.lock().map(|v| v.clone()).unwrap_or_default()
}

#[derive(Clone, Copy, Debug)]
pub struct Passo {
    pub texto: &'static str,
    /// ícone do comando a destacar (vazio: nenhum)
    pub icone: &'static str,
    pub cond: Cond,
}

#[derive(Clone, Copy, Debug)]
pub struct Aula {
    pub id: &'static str,
    pub titulo: &'static str,
    pub resumo: &'static str,
    pub bancada: Bench,
    pub nivel: Nivel,
    /// nome da peça de exemplo (de `examples::all`) para abrir antes da aula
    pub exemplo: Option<&'static str>,
    /// roteiro da esteira de mídia (B257) que mostra o comando principal em GIF
    pub midia: Option<&'static str>,
    pub passos: &'static [Passo],
}

#[derive(Clone, Copy, Debug)]
pub struct Trilha {
    pub id: &'static str,
    pub titulo: &'static str,
    pub resumo: &'static str,
    pub nivel: Nivel,
    pub aulas: &'static [&'static str],
}

const fn p(texto: &'static str, icone: &'static str, cond: Cond) -> Passo {
    Passo { texto, icone, cond }
}

use Bench::*;
use Cond::*;
use Nivel::*;

#[allow(clippy::too_many_arguments)]
const fn a(id: &'static str, titulo: &'static str, resumo: &'static str, bancada: Bench, nivel: Nivel, exemplo: Option<&'static str>, midia: Option<&'static str>, passos: &'static [Passo]) -> Aula {
    Aula { id, titulo, resumo, bancada, nivel, exemplo, midia, passos }
}

/// Todas as aulas. A ordem aqui é a ordem em que aparecem nas listas.
pub const AULAS: &[Aula] = &[
    // ---- Sólidos
    a("sol_camera", "Câmera e seleção", "Antes de modelar: girar, mover e aproximar a vista, olhar de frente e escolher uma face.", Solidos, Iniciante, Some("Flange"), None, &[
        p("Gire a vista: arraste com o botão esquerdo no fundo (ou com o do meio).", "", Gesto("girar")),
        p("Mova a vista: arraste com o botão direito, ou com Shift e o esquerdo.", "", Gesto("mover_vista")),
        p("Aproxime com a roda: o ponto sob o cursor fica parado.", "", Gesto("zoom")),
        p("Clique numa face do cubo de vistas, no canto, para olhar a peça de frente.", "", Gesto("vista_cubo")),
        p("Clique numa face da peça para selecioná-la; ela acende na cor de destaque.", "", Gesto("selecionar")),
    ]),
    a("sol_esboco", "Esboço do perfil", "Todo sólido começa num desenho 2D fechado sobre um plano.", Solidos, Iniciante, None, None, &[
        p("Crie um sketch no plano frontal e desenhe um retângulo fechado.", "sketch", Feature("sketch", 1)),
        p("Cote o retângulo: cada medida vira um parâmetro que você pode mudar depois.", "sk_cota", CotaSketch),
    ]),
    a("sol_volume", "Dar volume ao esboço", "A extrusão empurra o perfil e cria o primeiro sólido.", Solidos, Iniciante, None, Some("extrusao"), &[
        p("Extrude o perfil fechado por uma distância.", "extrusao", Feature("extrusao", 1)),
        p("Mude a distância na árvore e veja o sólido recalcular.", "", Manual),
    ]),
    a("sol_furo", "Furos e recortes", "Furo e recorte tiram material; o recorte usa um sketch, o furo usa só um ponto e um diâmetro.", Solidos, Iniciante, None, None, &[
        p("Faça um furo numa face do sólido.", "furo_simples", Feature("furo", 1)),
        p("Desenhe outro perfil numa face e use Recorte para tirar o material.", "recorte", Feature("recorte", 1)),
    ]),
    a("sol_acabamento", "Arredondar e chanfrar", "Filete e chanfro suavizam as arestas vivas.", Solidos, Iniciante, None, None, &[
        p("Arredonde uma aresta com um filete.", "arestas_selecionar", Feature("filete", 1)),
        p("Quebre outra aresta com um chanfro.", "chanfro", Feature("chanfro", 1)),
    ]),
    a("sol_repetir", "Repetir e espelhar", "Padrão e espelho copiam comandos em vez de você refazê-los.", Solidos, Intermediario, None, None, &[
        p("Repita um comando em fileira com o padrão linear.", "padrao_linear", Feature("padrao", 1)),
        p("Espelhe o resultado em torno de um plano.", "espelho", Feature("espelho", 1)),
    ]),
    a("sol_casca", "Esvaziar uma peça", "A casca deixa só uma parede de espessura constante.", Solidos, Intermediario, Some("Caixa com casca"), Some("casca"), &[
        p("Abra a peça de exemplo Caixa com casca.", "exemplo", Exemplo("Caixa com casca")),
        p("Mude a espessura da casca na árvore e veja a parede mudar.", "casca", Manual),
    ]),
    // ---- Superfícies
    a("sup_curvas", "Curvas guia", "Curvas bem feitas são a base de toda superfície limpa.", Superficies, Intermediario, None, None, &[
        p("Crie uma curva guia, como uma spline por pontos.", "spline", Referencias(1)),
        p("Ajuste os pontos até a curva ficar suave, sem ondulação.", "", Manual),
    ]),
    a("sup_criar", "Superfícies a partir de curvas", "Extrusão, varredura e seções geram superfícies a partir das curvas.", Superficies, Intermediario, Some("Superfícies de exemplo"), None, &[
        p("Crie uma superfície extrudada a partir de uma curva.", "sup_extrudada", NSuperficies(1)),
        p("Crie uma segunda superfície por seções entre duas curvas.", "sup_multisecoes", NSuperficies(2)),
    ]),
    a("sup_juntar", "Juntar e engrossar", "Superfícies que se tocam são juntadas e podem virar um sólido.", Superficies, Avancado, Some("Superfícies de exemplo"), None, &[
        p("Junte as superfícies que se tocam numa só.", "op_juntar", SuperficieJuntada(1)),
        p("Engrosse a superfície para obter um sólido.", "hib_engrossar", Feature("volume_de_superficie", 1)),
    ]),
    a("sup_verificar", "Verificar a continuidade", "Zebra e curvatura mostram se a transição entre superfícies é suave.", Superficies, Avancado, Some("Superfícies de exemplo"), None, &[
        p("Ligue a zebra: listras que se quebram indicam só encontro de posição.", "an_zebra", Comando("an_zebra")),
        p("Confira o mapa de curvatura: cor contínua indica transição suave.", "an_curvatura", Comando("an_curvatura")),
    ]),
    // ---- Forma livre
    a("ff_primeira", "Primeira forma", "A forma livre nasce de uma gaiola de nós; a superfície lisa aparece por dentro dela.", FormaLivre, Iniciante, None, None, &[
        p("Crie uma caixa de forma: clique no plano e arraste o tamanho.", "forma_caixa", Feature("forma", 1)),
        p("Clique num nó da gaiola e puxe: a superfície lisa acompanha.", "", Manual),
    ]),
    a("ff_moldar", "Moldar a gaiola", "Poucos nós bem postos dão uma superfície limpa; alterne entre a gaiola e o liso para conferir.", FormaLivre, Iniciante, Some("Forma suave"), None, &[
        p("Abra a peça de exemplo Forma suave.", "exemplo", Exemplo("Forma suave")),
        p("Arraste dois ou três nós e alterne entre a gaiola e a superfície lisa.", "", Manual),
    ]),
    a("ff_vincos", "Vincos e arestas vivas", "Um vinco endurece uma aresta: o peso vai de suave (0) a viva (1).", FormaLivre, Intermediario, Some("Forma suave"), None, &[
        p("Abra a peça de exemplo Forma suave.", "exemplo", Exemplo("Forma suave")),
        p("Dê peso a um vinco numa aresta da gaiola para deixá-la viva.", "", FormaVinco),
    ]),
    a("ff_converter", "Da forma ao sólido", "Converter a forma em corpo B-rep deixa usar filetes, furos e o resto da modelagem.", FormaLivre, Avancado, Some("Forma suave"), None, &[
        p("Abra a peça de exemplo Forma suave.", "exemplo", Exemplo("Forma suave")),
        p("Converta a forma em corpo: escolha a continuidade e a tolerância.", "", FormaConvertida),
        p("Arredonde uma aresta do corpo convertido com um filete.", "arestas_selecionar", Feature("filete", 1)),
    ]),
    // ---- Montagem
    a("mon_inserir", "Inserir peças", "A montagem começa trazendo as peças para o mesmo espaço.", Montagem, Iniciante, None, None, &[
        p("Insira a primeira peça na montagem.", "asm_inserir", Componentes(1)),
        p("Insira a segunda peça.", "asm_inserir", Componentes(2)),
    ]),
    a("mon_restringir", "Restringir", "Restrições dizem como uma peça se posiciona em relação à outra.", Montagem, Iniciante, Some("Conjunto de exemplo"), None, &[
        p("Aplique uma coincidência entre duas faces.", "rest_coincidencia", Restricoes(1)),
        p("Aplique uma segunda restrição, como distância ou paralelismo.", "rest_distancia", Restricoes(2)),
    ]),
    a("mon_verificar", "Verificar a montagem", "Antes de documentar, procure peças que se atravessam.", Montagem, Intermediario, Some("Conjunto de exemplo"), None, &[
        p("Rode a verificação de interferências e leia o resultado.", "rest_interferencia", Comando("rest_interferencia")),
        p("Mova uma peça e confira se as restrições seguram o resto.", "", Manual),
    ]),
    a("mon_documentar", "Documentar a montagem", "Lista de peças e balões saem direto da estrutura da montagem.", Montagem, Intermediario, Some("Conjunto de exemplo"), None, &[
        p("Gere a lista de peças.", "asm_lista", Comando("asm_lista")),
        p("Numere os itens da montagem.", "asm_numerar", Comando("asm_numerar")),
    ]),
    // ---- Chapa
    a("ch_base", "Chapa base", "A parede base define espessura, raio de dobra e fator K de toda a peça.", Chapa, Iniciante, None, Some("espessura"), &[
        p("Crie a parede base a partir de um perfil fechado.", "sm_parede", Feature("sm_parede", 1)),
        p("Confira espessura, raio e fator K nos parâmetros da chapa.", "sm_parametros", Comando("sm_parametros")),
    ]),
    a("ch_abas", "Abas e dobras", "Aba e bainha dobram a borda da chapa com o raio certo.", Chapa, Iniciante, Some("Suporte de chapa"), None, &[
        p("Abra a peça de exemplo Suporte de chapa.", "exemplo", Exemplo("Suporte de chapa")),
        p("Observe a aba na árvore e mude o comprimento dela.", "sm_aba", Feature("sm_aba", 1)),
        p("Observe a bainha de 180 graus na borda oposta.", "sm_bainha", Feature("sm_bainha", 1)),
    ]),
    a("ch_recortes", "Recortes e alívios", "Recortes normais à chapa e alívios de canto evitam rasgos na dobra.", Chapa, Intermediario, None, None, &[
        p("Faça um recorte normal à chapa.", "sm_recorte", Feature("sm_recorte", 1)),
        p("Faça um alívio de canto onde duas dobras se encontram.", "sm_alivio_canto", Feature("sm_alivio", 1)),
    ]),
    a("ch_planificar", "Planificação", "Desdobrar mostra a chapa plana que vai para o corte.", Chapa, Intermediario, Some("Bandeja curva"), None, &[
        p("Abra a peça de exemplo Bandeja curva.", "exemplo", Exemplo("Bandeja curva")),
        p("Desdobre a chapa e leia o comprimento planificado.", "sm_desdobrar", Planificada),
    ]),
    a("ch_desenho", "Mapa de dobras e desenho", "A chapa planificada vira vista de desenho com a tabela de dobras.", Chapa, Avancado, None, None, &[
        p("Crie uma vista planificada da peça no desenho.", "des_planificada", VistasPlanas(1)),
        p("Insira a tabela de dobras na folha.", "des_tab_dobras", TabelasDobras(1)),
    ]),
    // ---- Estrutural
    a("est_membros", "Membros sobre o esqueleto", "Cada linha do esqueleto vira um perfil com a seção escolhida.", Estrutural, Iniciante, None, None, &[
        p("Desenhe as linhas do esqueleto da estrutura.", "linha_pontos", Referencias(1)),
        p("Crie os membros sobre as linhas, escolhendo a seção do perfil.", "st_membro", Feature("st_membro", 1)),
    ]),
    a("est_cantos", "Cantos e uniões", "O encontro de dois membros pode ser meia-esquadria, topo ou encaixe.", Estrutural, Intermediario, Some("Quadro de mesa"), None, &[
        p("Abra a peça de exemplo Quadro de mesa.", "exemplo", Exemplo("Quadro de mesa")),
        p("Troque o tipo de canto e veja o corte dos perfis mudar.", "st_canto_meia", Manual),
    ]),
    a("est_placas", "Placas e reforços", "Placas de ponta e reforços ligam e enrijecem os membros.", Estrutural, Intermediario, None, None, &[
        p("Coloque uma placa na ponta de um membro.", "st_placa", Feature("st_placa", 1)),
        p("Coloque um reforço entre dois membros.", "st_reforco", Feature("st_reforco", 1)),
    ]),
    a("est_corte", "Lista de corte e montagem", "A lista de corte sai da estrutura e a montagem separa cada membro.", Estrutural, Avancado, None, None, &[
        p("Gere a lista de corte com comprimentos e ângulos.", "st_corte", Comando("st_corte")),
        p("Gere a montagem com um componente por membro.", "st_montagem", Comando("st_montagem")),
    ]),
    // ---- Desenho
    a("des_folha", "Folha e carimbo", "O desenho nasce numa folha com formato, escala e carimbo.", Desenho, Iniciante, None, None, &[
        p("Crie um desenho novo; ele nasce com uma folha.", "des_novo", Folhas(1)),
        p("Preencha o carimbo: título e desenhista.", "", Carimbo),
    ]),
    a("des_vistas", "Vistas", "As vistas projetam o modelo 3D e continuam ligadas a ele.", Desenho, Iniciante, Some("Folha da placa"), None, &[
        p("Insira a vista principal da peça.", "des_vista", Vistas(1)),
        p("Insira as três vistas ortogonais.", "des_tres_vistas", Vistas(3)),
    ]),
    a("des_cotas", "Cotas e anotações", "Cotas leem a medida do modelo; notas e símbolos explicam o resto.", Desenho, Intermediario, Some("Folha da placa"), None, &[
        p("Coloque uma cota horizontal.", "des_cota_h", Cotas(1)),
        p("Acrescente uma nota ou um símbolo.", "des_nota", Notas(1)),
    ]),
    a("des_tabelas", "Tabelas e exportação", "Tabelas leem o documento; a exportação gera o arquivo que vai para a fábrica.", Desenho, Intermediario, Some("Folha da placa"), None, &[
        p("Insira uma tabela de materiais.", "des_bom", Tabelas(1)),
        p("Exporte a folha em PDF.", "des_pdf", Comando("des_pdf")),
    ]),
];

pub const TRILHAS: &[Trilha] = &[
    Trilha { id: "primeiros_passos", titulo: "Primeiros passos", resumo: "Da câmera ao sólido acabado, em seis aulas curtas.", nivel: Iniciante, aulas: &["sol_camera", "sol_esboco", "sol_volume", "sol_furo", "sol_repetir", "sol_acabamento"] },
    Trilha { id: "solidos", titulo: "Modelagem sólida", resumo: "Tudo o que a bancada de sólidos faz, na ordem em que se usa.", nivel: Intermediario, aulas: &["sol_camera", "sol_esboco", "sol_volume", "sol_furo", "sol_acabamento", "sol_repetir", "sol_casca"] },
    Trilha { id: "superficies", titulo: "Superfícies", resumo: "Curvas, superfícies, união e verificação de continuidade.", nivel: Avancado, aulas: &["sup_curvas", "sup_criar", "sup_juntar", "sup_verificar"] },
    Trilha { id: "forma_livre", titulo: "Forma livre", resumo: "Gaiola de nós, superfície lisa, vincos e conversão em sólido.", nivel: Intermediario, aulas: &["ff_primeira", "ff_moldar", "ff_vincos", "ff_converter"] },
    Trilha { id: "montagem", titulo: "Montagens", resumo: "Inserir, restringir, verificar e documentar.", nivel: Intermediario, aulas: &["mon_inserir", "mon_restringir", "mon_verificar", "mon_documentar"] },
    Trilha { id: "chapa", titulo: "Chapa metálica", resumo: "Da chapa base à planificação e ao desenho de fabricação.", nivel: Intermediario, aulas: &["ch_base", "ch_abas", "ch_recortes", "ch_planificar", "ch_desenho"] },
    Trilha { id: "estrutural", titulo: "Estruturas soldadas", resumo: "Esqueleto, perfis, cantos, placas e lista de corte.", nivel: Intermediario, aulas: &["est_membros", "est_cantos", "est_placas", "est_corte"] },
    Trilha { id: "desenho", titulo: "Desenho técnico", resumo: "Folha, vistas, cotas e tabelas.", nivel: Intermediario, aulas: &["des_folha", "des_vistas", "des_cotas", "des_tabelas"] },
    Trilha { id: "peca_ao_desenho", titulo: "Da peça ao desenho", resumo: "Modele uma peça simples e entregue a folha pronta.", nivel: Intermediario, aulas: &["sol_esboco", "sol_volume", "des_folha", "des_vistas", "des_cotas"] },
];

pub fn aula(id: &str) -> Option<&'static Aula> {
    AULAS.iter().find(|a| a.id == id)
}

pub fn trilha(id: &str) -> Option<&'static Trilha> {
    TRILHAS.iter().find(|t| t.id == id)
}

/// Aulas de uma trilha, na ordem.
pub fn aulas_da_trilha(t: &Trilha) -> Vec<&'static Aula> {
    t.aulas.iter().filter_map(|i| aula(i)).collect()
}

/// Aulas de uma bancada, na ordem.
pub fn aulas_da_bancada(b: Bench) -> Vec<&'static Aula> {
    AULAS.iter().filter(|a| a.bancada == b).collect()
}

/// Trilha completa de uma bancada.
pub fn trilha_da_bancada(b: Bench) -> &'static Trilha {
    let id = match b {
        Bench::Solidos => "solidos",
        Bench::Superficies => "superficies",
        Bench::FormaLivre => "forma_livre",
        Bench::Montagem => "montagem",
        Bench::Chapa => "chapa",
        Bench::Estrutural => "estrutural",
        Bench::Desenho => "desenho",
    };
    trilha(id).expect("toda bancada tem trilha")
}

// ------------------------------------------------------------------ reconhecimento

/// O que o curso enxerga do trabalho da pessoa. Montado pelo aplicativo a cada quadro.
pub struct Contexto<'a> {
    pub doc: &'a Document,
    pub componentes: usize,
    pub restricoes: usize,
    pub folhas: usize,
    pub vistas: usize,
    pub cotas: usize,
    pub tabelas: usize,
    pub notas: usize,
    pub vistas_planas: usize,
    pub tabelas_dobras: usize,
    /// o carimbo do desenho tem título ou desenhista
    pub carimbo: bool,
    /// nomes da montagem e do desenho abertos (peças de exemplo compostas)
    pub nomes: [&'a str; 2],
    /// ícones dos comandos acionados nesta sessão
    pub usados: &'a [&'static str],
}

impl<'a> Contexto<'a> {
    pub fn so_documento(doc: &'a Document) -> Self {
        Contexto { doc, componentes: 0, restricoes: 0, folhas: 0, vistas: 0, cotas: 0, tabelas: 0, notas: 0, vistas_planas: 0, tabelas_dobras: 0, carimbo: false, nomes: ["", ""], usados: &[] }
    }
}

fn tem_cota_de_sketch(doc: &Document) -> bool {
    doc.features.iter().filter(|f| !f.suppressed).any(|f| matches!(&f.kind, FeatureKind::Sketch(s) if s.data.constraints.iter().any(|c| !c.reference && c.kind.is_dimension())))
}

fn formas(doc: &Document) -> impl Iterator<Item = &fenkra_core::Forma> {
    doc.features.iter().filter(|f| !f.suppressed).filter_map(|f| match &f.kind {
        FeatureKind::Forma(x) => Some(x),
        _ => None,
    })
}

/// Etiqueta curta do tipo de comando, a que o curso reconhece. O que o curso não usa vira "outro".
pub fn etiqueta(k: &FeatureKind) -> &'static str {
    use FeatureKind as F;
    match k {
        F::Sketch(_) => "sketch",
        F::Extrude(e) if e.op == BoolOp::Add => "extrusao",
        F::Extrude(_) => "recorte",
        F::Revolve(_) => "revolucao",
        F::Hole(_) => "furo",
        F::Fillet(_) => "filete",
        F::Chamfer(_) => "chanfro",
        F::Shell(_) => "casca",
        F::Pattern(_) => "padrao",
        F::Mirror(_) => "espelho",
        F::ThickSurface(_) | F::CloseSurface(_) | F::SewSurface(_) | F::BoundaryFill(_) => "volume_de_superficie",
        F::SmWall(_) => "sm_parede",
        F::SmFlange(_) | F::SmProfileFlange(_) | F::SmCurveFlange(_) | F::SmFlangeLoop(_) | F::SmSurfFlange(_) => "sm_aba",
        F::SmHem(_) => "sm_bainha",
        F::SmCut(_) | F::SmArcSlot(_) => "sm_recorte",
        F::SmCornerRelief(_) | F::SmCornerClose(_) => "sm_alivio",
        F::Forma(_) => "forma",
        F::StructMember(_) => "st_membro",
        F::StructPlate(_) => "st_placa",
        F::StructGusset(_) => "st_reforco",
        F::StructWeld(_) => "st_solda",
        _ => "outro",
    }
}

fn quantos(doc: &Document, etq: &str) -> usize {
    doc.features.iter().filter(|f| !f.suppressed && etiqueta(&f.kind) == etq).count()
}

/// Nome da variante de um elemento de referência (ex.: `SurfExtrude`, `OffsetPlane`).
fn variante(r: &fenkra_core::RefElement) -> String {
    let d = format!("{:?}", r.kind);
    d.chars().take_while(|c| c.is_alphanumeric()).collect()
}

fn n_superficies(doc: &Document) -> usize {
    doc.references.iter().filter(|r| variante(r).starts_with("Surf")).count()
}

fn n_curvas_e_pontos(doc: &Document) -> usize {
    doc.references.iter().filter(|r| !variante(r).ends_with("Plane") && !variante(r).starts_with("Surf")).count()
}

impl Cond {
    /// O contexto mostra o passo cumprido? `Manual` nunca é reconhecido sozinho.
    pub fn reconhecida(self, c: &Contexto) -> bool {
        match self {
            Feature(etq, n) => quantos(c.doc, etq) >= n,
            Referencias(n) => n_curvas_e_pontos(c.doc) >= n,
            NSuperficies(n) => n_superficies(c.doc) >= n,
            Componentes(n) => c.componentes >= n,
            Restricoes(n) => c.restricoes >= n,
            Folhas(n) => c.folhas >= n,
            Vistas(n) => c.vistas >= n,
            Cotas(n) => c.cotas >= n,
            Tabelas(n) => c.tabelas >= n,
            Exemplo(nome) => c.doc.name == nome || c.nomes.contains(&nome),
            CotaSketch => tem_cota_de_sketch(c.doc),
            Planificada => c.doc.sheet.as_ref().is_some_and(|s| s.flat),
            SuperficieJuntada(n) => c.doc.references.iter().filter(|r| variante(r) == "SurfJoin").count() >= n,
            FormaVinco => formas(c.doc).any(|f| !f.gaiola.vincos.is_empty()),
            FormaConvertida => formas(c.doc).any(|f| f.conversao.is_some()),
            Notas(n) => c.notas >= n,
            VistasPlanas(n) => c.vistas_planas >= n,
            TabelasDobras(n) => c.tabelas_dobras >= n,
            Carimbo => c.carimbo,
            Comando(icone) => c.usados.contains(&icone),
            Gesto(g) => c.usados.contains(&g),
            Manual => false,
        }
    }
}

// ------------------------------------------------------------------ progresso

/// Andamento de uma aula. Máscaras de bits: o bit `i` é o passo `i`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ProgressoAula {
    /// passos cumpridos (reconhecidos ou confirmados)
    pub feitos: u32,
    /// passos que a pessoa pulou
    pub puladas: u32,
    /// a aula inteira foi pulada
    pub aula_pulada: bool,
}

impl ProgressoAula {
    fn bit(i: usize) -> u32 {
        1u32 << i.min(31)
    }
    pub fn passo_feito(&self, i: usize) -> bool {
        self.feitos & Self::bit(i) != 0
    }
    pub fn passo_pulado(&self, i: usize) -> bool {
        self.puladas & Self::bit(i) != 0
    }
}

/// Tudo do curso que fica nas preferências da pessoa.
#[derive(Clone, PartialEq, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct EstadoCurso {
    /// perfil de fluxo escolhido (id de `compat::PERFIS`)
    pub perfil: Option<String>,
    /// a pergunta "de onde você vem?" já foi respondida ou dispensada
    pub perfil_visto: bool,
    /// tocar a narração, quando houver voz instalada
    pub audio: bool,
    pub aulas: Vec<(String, ProgressoAula)>,
    /// aula em andamento, para retomar
    pub atual: Option<String>,
}

impl EstadoCurso {
    pub fn prog(&self, id: &str) -> ProgressoAula {
        self.aulas.iter().find(|(i, _)| i == id).map(|(_, p)| *p).unwrap_or_default()
    }

    fn prog_mut(&mut self, id: &str) -> &mut ProgressoAula {
        if let Some(k) = self.aulas.iter().position(|(i, _)| i == id) {
            return &mut self.aulas[k].1;
        }
        self.aulas.push((id.to_string(), ProgressoAula::default()));
        &mut self.aulas.last_mut().expect("acabou de entrar").1
    }

    /// Primeiro passo que ainda não foi feito nem pulado.
    pub fn proximo_passo(&self, a: &Aula) -> Option<usize> {
        let pr = self.prog(a.id);
        (0..a.passos.len()).find(|&i| !pr.passo_feito(i) && !pr.passo_pulado(i))
    }

    /// Passos resolvidos (feitos ou pulados) e total.
    pub fn andamento(&self, a: &Aula) -> (usize, usize) {
        let pr = self.prog(a.id);
        ((0..a.passos.len()).filter(|&i| pr.passo_feito(i) || pr.passo_pulado(i)).count(), a.passos.len())
    }

    /// Todos os passos feitos ou pulados, ou a aula foi pulada.
    pub fn resolvida(&self, a: &Aula) -> bool {
        self.prog(a.id).aula_pulada || self.proximo_passo(a).is_none()
    }

    /// Todos os passos feitos de verdade (nenhum pulado).
    pub fn concluida(&self, a: &Aula) -> bool {
        let pr = self.prog(a.id);
        !pr.aula_pulada && (0..a.passos.len()).all(|i| pr.passo_feito(i))
    }

    /// Reconhece, em ordem, os passos que o documento já mostra cumpridos. Devolve se algo mudou.
    /// Só olha o primeiro passo pendente: os seguintes dependem dele.
    pub fn sincronizar(&mut self, a: &Aula, c: &Contexto) -> bool {
        let mut mudou = false;
        while let Some(i) = self.proximo_passo(a) {
            if !a.passos[i].cond.reconhecida(c) {
                break;
            }
            self.prog_mut(a.id).feitos |= ProgressoAula::bit(i);
            mudou = true;
        }
        mudou
    }

    /// A pessoa confirma um passo sem rastro no documento.
    pub fn confirmar(&mut self, a: &Aula, i: usize) {
        if i < a.passos.len() {
            self.prog_mut(a.id).feitos |= ProgressoAula::bit(i);
        }
    }

    pub fn pular_passo(&mut self, a: &Aula, i: usize) {
        if i < a.passos.len() && !self.prog(a.id).passo_feito(i) {
            self.prog_mut(a.id).puladas |= ProgressoAula::bit(i);
        }
    }

    pub fn pular_aula(&mut self, a: &Aula) {
        self.prog_mut(a.id).aula_pulada = true;
        if self.atual.as_deref() == Some(a.id) {
            self.atual = None;
        }
    }

    /// Volta a uma aula pulada, ou a um passo pulado, sem perder o que já foi feito.
    pub fn retomar(&mut self, a: &Aula) {
        let pr = self.prog_mut(a.id);
        pr.aula_pulada = false;
        pr.puladas = 0;
        self.atual = Some(a.id.to_string());
    }

    /// Recomeça a aula do zero.
    pub fn refazer(&mut self, a: &Aula) {
        *self.prog_mut(a.id) = ProgressoAula::default();
        self.atual = Some(a.id.to_string());
    }

    /// Próxima aula da trilha que ainda pede atenção (nem resolvida nem pulada).
    pub fn proxima_da_trilha(&self, t: &Trilha) -> Option<&'static Aula> {
        aulas_da_trilha(t).into_iter().find(|a| !self.resolvida(a))
    }

    /// Próxima aula pendente depois de `id`; se não houver, a primeira pendente da trilha.
    pub fn proxima_depois(&self, t: &Trilha, id: &str) -> Option<&'static Aula> {
        let v = aulas_da_trilha(t);
        let pos = v.iter().position(|a| a.id == id).map_or(0, |p| p + 1);
        v.iter().skip(pos).chain(v.iter().take(pos)).find(|a| a.id != id && !self.resolvida(a)).copied()
    }

    /// Aulas resolvidas e total da trilha.
    pub fn andamento_trilha(&self, t: &Trilha) -> (usize, usize) {
        let v = aulas_da_trilha(t);
        (v.iter().filter(|a| self.resolvida(a)).count(), v.len())
    }
}

/// Id da narração no pacote de mídia. Sem esse item no pacote, o curso segue só com texto.
pub fn id_voz(aula: &str) -> String {
    format!("voz/{aula}")
}

// ------------------------------------------------------------------ testes

#[cfg(test)]
mod testes {
    use super::*;

    fn doc_do_exemplo(nome: &str) -> Document {
        crate::curso_exemplos::peca(nome).unwrap_or_else(|| panic!("exemplo {nome}")).0
    }

    #[test]
    fn ids_unicos_e_trilhas_so_citam_aulas_que_existem() {
        let mut vistos = std::collections::HashSet::new();
        assert!(AULAS.iter().all(|a| vistos.insert(a.id)), "id de aula repetido");
        for t in TRILHAS {
            for id in t.aulas {
                assert!(aula(id).is_some(), "trilha {} cita a aula inexistente {id}", t.id);
            }
        }
        for a in AULAS {
            assert!(!a.passos.is_empty() && a.passos.len() <= 31, "{}: número de passos", a.id);
            assert!(TRILHAS.iter().any(|t| t.aulas.contains(&a.id)), "{} não está em nenhuma trilha", a.id);
        }
    }

    #[test]
    fn toda_bancada_tem_trilha_com_aulas_da_propria_bancada() {
        for b in [Bench::Solidos, Bench::Superficies, Bench::FormaLivre, Bench::Montagem, Bench::Chapa, Bench::Estrutural, Bench::Desenho] {
            let t = trilha_da_bancada(b);
            assert!(!t.aulas.is_empty());
            assert_eq!(aulas_da_trilha(t).len(), aulas_da_bancada(b).len(), "{b:?}");
            assert!(aulas_da_trilha(t).iter().all(|a| a.bancada == b));
        }
    }

    #[test]
    fn exemplos_icones_e_midias_citados_existem() {
        let icones: Vec<&str> = crate::ajuda_cmd::comandos().iter().map(|i| i.icon).collect();
        let roteiros = std::fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/media/roteiros")).unwrap().filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().trim_end_matches(".json").to_string()).collect::<Vec<_>>();
        for a in AULAS {
            if let Some(ex) = a.exemplo {
                assert!(crate::curso_exemplos::existe(ex), "{}: exemplo {ex} não existe", a.id);
            }
            if let Some(m) = a.midia {
                assert!(roteiros.iter().any(|r| r == m), "{}: roteiro de mídia {m} não existe", a.id);
            }
            for (i, p) in a.passos.iter().enumerate() {
                if !p.icone.is_empty() && p.icone != "exemplo" {
                    assert!(icones.contains(&p.icone), "{} passo {i}: ícone {} não é de nenhum comando", a.id, p.icone);
                }
                if let Exemplo(n) = p.cond {
                    let doc = crate::curso_exemplos::peca(n).map(|(d, _)| d.name);
                    assert_eq!(doc.as_deref(), Some(n), "{} passo {i}: nenhuma peça de exemplo se chama {n}", a.id);
                    assert_eq!(a.exemplo, Some(n), "{}: a peça do passo é a do exemplo da aula", a.id);
                }
            }
        }
    }

    #[test]
    fn todo_texto_do_curso_tem_traducao() {
        let mut faltam = vec![];
        let mut ver = |t: &str| {
            if !t.is_empty() && !crate::i18n::tem_traducao(t) {
                faltam.push(t.to_string());
            }
        };
        for a in AULAS {
            ver(a.titulo);
            ver(a.resumo);
            a.passos.iter().for_each(|p| ver(p.texto));
        }
        for t in TRILHAS {
            ver(t.titulo);
            ver(t.resumo);
        }
        for n in [Iniciante, Intermediario, Avancado] {
            ver(n.texto());
        }
        assert!(faltam.is_empty(), "sem tradução: {faltam:#?}");
    }

    #[test]
    fn peca_de_exemplo_cumpre_os_passos_de_chapa_e_estrutural() {
        // aula de abas: abrir o exemplo reconhece os três passos, em ordem
        let a = aula("ch_abas").unwrap();
        let mut e = EstadoCurso::default();
        let vazio = Document::new("Outra");
        assert!(!e.sincronizar(a, &Contexto::so_documento(&vazio)));
        assert_eq!(e.proximo_passo(a), Some(0));
        let doc = doc_do_exemplo("Suporte de chapa");
        assert!(e.sincronizar(a, &Contexto::so_documento(&doc)));
        assert!(e.concluida(a), "abrir o Suporte de chapa cumpre exemplo, aba e bainha");
        // estrutural
        let a = aula("est_cantos").unwrap();
        let mut e = EstadoCurso::default();
        e.sincronizar(a, &Contexto::so_documento(&doc_do_exemplo("Quadro de mesa")));
        assert_eq!(e.proximo_passo(a), Some(1), "o passo de trocar o canto é manual");
        assert!(doc_do_exemplo("Quadro de mesa").features.iter().any(|f| etiqueta(&f.kind) == "st_membro"));
    }

    #[test]
    fn passos_so_valem_em_ordem_e_cada_comando_e_reconhecido() {
        let a = aula("sol_volume").unwrap();
        let mut e = EstadoCurso::default();
        let flange = doc_do_exemplo("Flange");
        // o sketch e a extrusão do Flange cumprem o passo 0; o passo 1 é manual
        assert!(e.sincronizar(a, &Contexto::so_documento(&flange)));
        assert_eq!(e.andamento(a), (1, 2));
        assert_eq!(e.proximo_passo(a), Some(1));
        e.confirmar(a, 1);
        assert!(e.concluida(a));
        // sketch: um documento só com sketch não cumpre a extrusão
        let mut so_sketch = Document::new("S");
        so_sketch.features.push(flange.features[0].clone());
        let mut e2 = EstadoCurso::default();
        assert!(!e2.sincronizar(a, &Contexto::so_documento(&so_sketch)));
        let esboco = aula("sol_esboco").unwrap();
        assert!(e2.sincronizar(esboco, &Contexto::so_documento(&so_sketch)));
        assert_eq!(e2.proximo_passo(esboco), Some(1));
    }

    #[test]
    fn recorte_e_extrusao_sao_comandos_diferentes_para_o_curso() {
        let d = doc_do_exemplo("Placa paramétrica");
        let furos = quantos(&d, "furo");
        let recortes = quantos(&d, "recorte");
        let adds = quantos(&d, "extrusao");
        assert!(adds >= 1);
        assert_eq!(adds + recortes, d.features.iter().filter(|f| matches!(f.kind, FeatureKind::Extrude(_))).count());
        assert!(furos + recortes >= 1, "a placa de exemplo tem furos ou recortes");
    }

    #[test]
    fn pular_e_retomar_mantem_o_progresso_e_vai_e_volta_do_arquivo() {
        let a = aula("sol_acabamento").unwrap();
        let mut e = EstadoCurso::default();
        e.confirmar(a, 0);
        e.pular_passo(a, 1);
        e.atual = Some(a.id.to_string());
        assert!(e.resolvida(a) && !e.concluida(a), "passo pulado resolve a aula mas não a conclui");
        assert_eq!(e.andamento(a), (2, 2));
        // salvar e abrir de novo
        let json = serde_json::to_string(&e).unwrap();
        let mut volta: EstadoCurso = serde_json::from_str(&json).unwrap();
        assert_eq!(volta, e);
        // retomar devolve o passo pulado e mantém o feito
        volta.retomar(a);
        assert_eq!(volta.proximo_passo(a), Some(1));
        assert!(volta.prog(a.id).passo_feito(0));
        // pular a aula inteira não apaga o que existe, e retomar a reabre
        volta.pular_aula(a);
        assert!(volta.resolvida(a) && volta.atual.is_none());
        assert!(volta.prog(a.id).passo_feito(0));
        volta.retomar(a);
        assert!(!volta.prog(a.id).aula_pulada && volta.atual.as_deref() == Some(a.id));
        // refazer zera
        volta.refazer(a);
        assert_eq!(volta.andamento(a), (0, 2));
    }

    #[test]
    fn configuracao_antiga_abre_sem_curso_e_a_trilha_avanca() {
        let e: EstadoCurso = serde_json::from_str("{}").unwrap();
        assert_eq!(e, EstadoCurso::default());
        assert!(!e.perfil_visto && !e.audio);
        let t = trilha("primeiros_passos").unwrap();
        let mut e = e;
        assert_eq!(e.proxima_da_trilha(t).unwrap().id, "sol_camera");
        e.pular_aula(aula("sol_camera").unwrap());
        assert_eq!(e.proxima_da_trilha(t).unwrap().id, "sol_esboco");
        e.pular_aula(aula("sol_esboco").unwrap());
        assert_eq!(e.proxima_da_trilha(t).unwrap().id, "sol_volume");
        assert_eq!(e.andamento_trilha(t), (2, 6));
        // "próxima" segue a ordem a partir da aula atual e só volta ao começo no fim
        assert_eq!(e.proxima_depois(t, "sol_volume").unwrap().id, "sol_furo");
        for x in ["sol_furo", "sol_repetir", "sol_acabamento"] {
            e.pular_aula(aula(x).unwrap());
        }
        assert!(e.proxima_depois(t, "sol_volume").is_none(), "nada mais pendente fora a própria aula");
    }

    #[test]
    fn contagens_de_montagem_e_desenho_reconhecem_os_passos() {
        let doc = Document::new("M");
        let mut c = Contexto::so_documento(&doc);
        let mon = aula("mon_inserir").unwrap();
        let mut e = EstadoCurso::default();
        c.componentes = 1;
        e.sincronizar(mon, &c);
        assert_eq!(e.andamento(mon), (1, 2));
        c.componentes = 2;
        e.sincronizar(mon, &c);
        assert!(e.concluida(mon));
        let des = aula("des_vistas").unwrap();
        c.vistas = 3;
        e.sincronizar(des, &c);
        assert!(e.concluida(des), "três vistas cumprem a vista principal e as três vistas");
    }

    /// Todo passo da tabela que hoje depende de "Já fiz" tem de ser um que realmente não deixa rastro.
    #[test]
    fn so_ficam_manuais_os_passos_sem_rastro_reconhecivel() {
        let manuais: Vec<String> = AULAS.iter().flat_map(|a| a.passos.iter().enumerate().filter(|(_, p)| p.cond == Manual).map(move |(i, _)| format!("{}:{i}", a.id))).collect();
        // olhar, ajustar, conferir ou mover: só a pessoa sabe
        let esperados = ["sol_volume:1", "sol_casca:1", "sup_curvas:1", "mon_verificar:1", "est_cantos:1", "ff_primeira:1", "ff_moldar:1"];
        for m in &manuais {
            assert!(esperados.contains(&m.as_str()), "passo manual inesperado: {m}");
        }
        for id in ["mon_documentar", "mon_verificar", "sup_verificar", "sup_juntar", "ch_desenho", "des_tabelas", "des_cotas", "est_corte", "ch_planificar"] {
            let a = aula(id).unwrap();
            assert!(a.passos.iter().any(|p| p.cond != Manual), "{id} ainda é todo manual");
        }
    }

    #[test]
    fn condicoes_novas_leem_o_documento_e_o_contexto() {
        use fenkra_core::{Forma, RefKind};
        // chapa planificada
        let mut chapa = doc_do_exemplo("Suporte de chapa");
        let c = Contexto::so_documento(&chapa);
        assert!(!Planificada.reconhecida(&c));
        assert!(Feature("sm_aba", 1).reconhecida(&c));
        chapa.sheet.as_mut().unwrap().flat = true;
        assert!(Planificada.reconhecida(&Contexto::so_documento(&chapa)));
        // alívio de canto
        assert!(!Feature("sm_alivio", 1).reconhecida(&Contexto::so_documento(&chapa)));
        // cota de sketch: o retângulo do exemplo não tem; com uma cota passa
        let mut doc = doc_do_exemplo("Placa simples");
        assert!(!CotaSketch.reconhecida(&Contexto::so_documento(&doc)));
        for f in doc.features.iter_mut() {
            if let FeatureKind::Sketch(s) = &mut f.kind {
                let alvo = s.data.curves[0].id;
                s.data.constraints.push(fenkra_core::sketch::Constraint { id: fenkra_core::Id(9_900_001), kind: fenkra_core::sketch::Kind::Length(alvo, "50".into()), label: None, reference: false, flipped: false });
            }
        }
        assert!(CotaSketch.reconhecida(&Contexto::so_documento(&doc)));
        // cota de referência não comanda a geometria: não conta
        for f in doc.features.iter_mut() {
            if let FeatureKind::Sketch(s) = &mut f.kind {
                s.data.constraints.iter_mut().for_each(|c| c.reference = true);
            }
        }
        assert!(!CotaSketch.reconhecida(&Contexto::so_documento(&doc)));
        // superfície juntada: o exemplo tem duas soltas
        let (mut sup, _) = crate::curso_exemplos::peca("Superfícies de exemplo").unwrap();
        assert!(!SuperficieJuntada(1).reconhecida(&Contexto::so_documento(&sup)));
        let ids: Vec<_> = sup.references.iter().filter(|r| variante(r).starts_with("Surf")).map(|r| r.id).collect();
        sup.add_geometry(RefKind::SurfJoin { items: ids.into_iter().map(fenkra_core::SurfRef::Ref).collect(), tol: "0.01".into(), curves: vec![] });
        assert!(SuperficieJuntada(1).reconhecida(&Contexto::so_documento(&sup)));
        // forma: vinco e conversão
        let (mut forma, _) = crate::curso_exemplos::peca("Forma suave").unwrap();
        assert!(Feature("forma", 1).reconhecida(&Contexto::so_documento(&forma)));
        for f in forma.features.iter_mut() {
            if let FeatureKind::Forma(Forma { gaiola, conversao, .. }) = &mut f.kind {
                let (a, b) = (gaiola.faces[0].nos[0], gaiola.faces[0].nos[1]);
                gaiola.vincos.push(fenkra_core::subd::Vinco { a: a.min(b), b: a.max(b), peso: 1. });
                assert!(conversao.is_none());
            }
        }
        assert!(FormaVinco.reconhecida(&Contexto::so_documento(&forma)));
        assert!(!FormaConvertida.reconhecida(&Contexto::so_documento(&forma)));
        for f in forma.features.iter_mut() {
            if let FeatureKind::Forma(x) = &mut f.kind {
                x.conversao = Some(fenkra_core::ConversaoForma { continuidade: 1, tol: "0.01".into(), pausada: false });
            }
        }
        assert!(FormaConvertida.reconhecida(&Contexto::so_documento(&forma)));
        // contagens do desenho e comandos usados
        let vazio = Document::new("V");
        let usados = ["asm_lista"];
        let c = Contexto { notas: 1, vistas_planas: 1, tabelas_dobras: 1, carimbo: true, usados: &usados, ..Contexto::so_documento(&vazio) };
        assert!(Notas(1).reconhecida(&c) && VistasPlanas(1).reconhecida(&c) && TabelasDobras(1).reconhecida(&c) && Carimbo.reconhecida(&c));
        assert!(Comando("asm_lista").reconhecida(&c) && !Comando("asm_numerar").reconhecida(&c));
        assert!(!Notas(2).reconhecida(&c));
    }

    #[test]
    fn aula_de_comando_avanca_em_ordem_quando_o_comando_e_usado() {
        let a = aula("mon_documentar").unwrap();
        let doc = Document::new("M");
        let mut e = EstadoCurso::default();
        // numerar antes de gerar a lista não pula o primeiro passo
        let c = Contexto { usados: &["asm_numerar"], ..Contexto::so_documento(&doc) };
        assert!(!e.sincronizar(a, &c));
        assert_eq!(e.proximo_passo(a), Some(0));
        let c = Contexto { usados: &["asm_numerar", "asm_lista"], ..Contexto::so_documento(&doc) };
        assert!(e.sincronizar(a, &c));
        assert!(e.concluida(a), "com os dois comandos usados a aula se conclui sozinha");
    }

    #[test]
    fn aulas_da_forma_livre_estao_na_trilha_da_bancada() {
        let t = trilha_da_bancada(Bench::FormaLivre);
        assert_eq!(t.id, "forma_livre");
        assert_eq!(aulas_da_trilha(t).len(), 4);
        assert!(aulas_da_trilha(t).iter().all(|a| a.bancada == Bench::FormaLivre));
        // a trilha de superfícies não carrega mais as aulas da forma livre
        assert!(aulas_da_trilha(trilha_da_bancada(Bench::Superficies)).iter().all(|a| a.bancada == Bench::Superficies));
    }

    /// B297, aceite: cada lição inicial conclui simulando os passos na ordem em que a pessoa faria:
    /// os gestos anotados pela vista, o documento crescendo comando a comando e as contagens da
    /// montagem. Nenhum passo é dado por "já fiz", fora os manuais que só a pessoa sabe.
    #[test]
    fn licoes_iniciais_concluem_simulando_os_passos() {
        use fenkra_core::*;
        let mut e = EstadoCurso::default();
        // câmera e seleção: cada gesto anotado cumpre o seu passo, na ordem
        let cam = aula("sol_camera").unwrap();
        let vazio = Document::new("Flange");
        let mut feitos: Vec<&'static str> = vec![];
        for (i, g) in GESTOS.iter().enumerate() {
            assert_eq!(e.proximo_passo(cam), Some(i));
            feitos.push(g);
            e.sincronizar(cam, &Contexto { usados: &feitos, ..Contexto::so_documento(&vazio) });
        }
        assert!(e.concluida(cam), "a aula de câmera conclui só com os gestos");
        anotar("girar");
        assert!(anotados().contains(&"girar"));

        // primeiro sketch cotado, extrusão, furo e padrão: o documento cresce como na tela
        let mut d = Document::new("Aula");
        let mut dados = SketchData::default();
        let mut al = || d.alloc();
        let (_, lados) = dados.add_rect(&mut al, [0., 0.], [40., 20.], false);
        let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data: dados.clone() }));
        let esboco = aula("sol_esboco").unwrap();
        e.sincronizar(esboco, &Contexto::so_documento(&d));
        assert_eq!(e.proximo_passo(esboco), Some(1), "sem cota o esboço ainda não acabou");
        let mut al = || d.alloc();
        dados.add_constraint(&mut al, fenkra_core::sketch::Kind::Length(lados[0], "40".into()));
        if let Some(Feature { kind: FeatureKind::Sketch(s), .. }) = d.feature_mut(sk) {
            s.data = dados;
        }
        e.sincronizar(esboco, &Contexto::so_documento(&d));
        assert!(e.concluida(esboco), "o sketch cotado conclui a aula");

        let volume = aula("sol_volume").unwrap();
        let ext = d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind("10".into()), reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
        e.sincronizar(volume, &Contexto::so_documento(&d));
        e.confirmar(volume, 1);
        assert!(e.concluida(volume));

        let furo = aula("sol_furo").unwrap();
        d.add_feature(FeatureKind::Hole(Hole { sketch: sk, kind: HoleKind::Simples, diameter: "5".into(), depth: Extent::ThroughAll, thread: None, thread_modeled: false, standard: None, points: vec![], dir: None, reverse: false, tip: None, taper: "0".into(), left_hand: false, extra: Default::default() }));
        e.sincronizar(furo, &Contexto::so_documento(&d));
        assert_eq!(e.proximo_passo(furo), Some(1));
        let mut dados = SketchData::default();
        let mut al = || d.alloc();
        dados.add_rect(&mut al, [25., 5.], [35., 15.], false);
        let sk2 = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data: dados }));
        d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk2, op: BoolOp::Cut, extent: Extent::Blind("4".into()), reverse: true, taper: "0".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
        e.sincronizar(furo, &Contexto::so_documento(&d));
        assert!(e.concluida(furo));

        let repetir = aula("sol_repetir").unwrap();
        d.add_feature(FeatureKind::Pattern(Pattern { kind: PatternKind::Linear { dir: AxisRef::Origin(OriginAxis::X), count: "3".into(), spacing: "50".into(), dir2: None, count2: "1".into(), spacing2: "0".into(), total: false }, sources: vec![ext], skip: vec![] }));
        e.sincronizar(repetir, &Contexto::so_documento(&d));
        assert_eq!(e.proximo_passo(repetir), Some(1));
        d.add_feature(FeatureKind::Mirror(Mirror { plane: PlaneRef::Origin(OriginPlane::YZ), sources: vec![ext] }));
        e.sincronizar(repetir, &Contexto::so_documento(&d));
        assert!(e.concluida(repetir));

        // primeira montagem: inserir duas peças e restringir duas vezes
        let m = Document::new("Conjunto de exemplo");
        for (id, fim) in [("mon_inserir", (2, 0)), ("mon_restringir", (2, 2))] {
            let a = aula(id).unwrap();
            for passo in 0..=fim.0.max(fim.1) {
                let c = Contexto { componentes: passo.min(fim.0), restricoes: passo.min(fim.1), ..Contexto::so_documento(&m) };
                e.sincronizar(a, &c);
            }
            assert!(e.concluida(a), "{id} não concluiu");
        }
    }

    #[test]
    fn o_curso_funciona_sem_pacote_de_audio() {
        // sem pacote de mídia, nenhuma voz existe e isso não impede nada
        for a in AULAS {
            let _ = crate::midia::exibir::tem(&id_voz(a.id));
        }
        let e = EstadoCurso { audio: true, ..Default::default() };
        let a = aula("sol_esboco").unwrap();
        assert_eq!(e.proximo_passo(a), Some(0));
    }
}
