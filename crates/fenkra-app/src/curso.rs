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
    /// a peça aberta tem este nome (peça de exemplo)
    Exemplo(&'static str),
    /// sem rastro no documento: a pessoa confirma
    Manual,
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
    a("sol_esboco", "Esboço do perfil", "Todo sólido começa num desenho 2D fechado sobre um plano.", Solidos, Iniciante, None, None, &[
        p("Crie um sketch no plano frontal e desenhe um retângulo fechado.", "sketch", Feature("sketch", 1)),
        p("Cote o retângulo: cada medida vira um parâmetro que você pode mudar depois.", "sk_cota", Manual),
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
    a("sup_criar", "Superfícies a partir de curvas", "Extrusão, varredura e seções geram superfícies a partir das curvas.", Superficies, Intermediario, None, None, &[
        p("Crie uma superfície extrudada a partir de uma curva.", "sup_extrudada", NSuperficies(1)),
        p("Crie uma segunda superfície por seções entre duas curvas.", "sup_multisecoes", NSuperficies(2)),
    ]),
    a("sup_juntar", "Juntar e engrossar", "Superfícies que se tocam são juntadas e podem virar um sólido.", Superficies, Avancado, Some("Placa simples"), None, &[
        p("Junte as superfícies que se tocam numa só.", "op_juntar", Manual),
        p("Engrosse a superfície para obter um sólido.", "hib_engrossar", Feature("volume_de_superficie", 1)),
    ]),
    a("sup_verificar", "Verificar a continuidade", "Zebra e curvatura mostram se a transição entre superfícies é suave.", Superficies, Avancado, None, None, &[
        p("Ligue a zebra: listras que se quebram indicam só encontro de posição.", "an_zebra", Manual),
        p("Confira o mapa de curvatura: cor contínua indica transição suave.", "an_curvatura", Manual),
    ]),
    // ---- Montagem
    a("mon_inserir", "Inserir peças", "A montagem começa trazendo as peças para o mesmo espaço.", Montagem, Iniciante, None, None, &[
        p("Insira a primeira peça na montagem.", "asm_inserir", Componentes(1)),
        p("Insira a segunda peça.", "asm_inserir", Componentes(2)),
    ]),
    a("mon_restringir", "Restringir", "Restrições dizem como uma peça se posiciona em relação à outra.", Montagem, Iniciante, None, None, &[
        p("Aplique uma coincidência entre duas faces.", "rest_coincidencia", Restricoes(1)),
        p("Aplique uma segunda restrição, como distância ou paralelismo.", "rest_distancia", Restricoes(2)),
    ]),
    a("mon_verificar", "Verificar a montagem", "Antes de documentar, procure peças que se atravessam.", Montagem, Intermediario, None, None, &[
        p("Rode a verificação de interferências e leia o resultado.", "rest_interferencia", Manual),
        p("Mova uma peça e confira se as restrições seguram o resto.", "", Manual),
    ]),
    a("mon_documentar", "Documentar a montagem", "Lista de peças e balões saem direto da estrutura da montagem.", Montagem, Intermediario, None, None, &[
        p("Gere a lista de peças.", "asm_lista", Manual),
        p("Numere os itens da montagem.", "asm_numerar", Manual),
    ]),
    // ---- Chapa
    a("ch_base", "Chapa base", "A parede base define espessura, raio de dobra e fator K de toda a peça.", Chapa, Iniciante, None, Some("espessura"), &[
        p("Crie a parede base a partir de um perfil fechado.", "sm_parede", Feature("sm_parede", 1)),
        p("Confira espessura, raio e fator K nos parâmetros da chapa.", "sm_parametros", Manual),
    ]),
    a("ch_abas", "Abas e dobras", "Aba e bainha dobram a borda da chapa com o raio certo.", Chapa, Iniciante, Some("Suporte de chapa"), None, &[
        p("Abra a peça de exemplo Suporte de chapa.", "exemplo", Exemplo("Suporte de chapa")),
        p("Observe a aba na árvore e mude o comprimento dela.", "sm_aba", Feature("sm_aba", 1)),
        p("Observe a bainha de 180 graus na borda oposta.", "sm_bainha", Feature("sm_bainha", 1)),
    ]),
    a("ch_recortes", "Recortes e alívios", "Recortes normais à chapa e alívios de canto evitam rasgos na dobra.", Chapa, Intermediario, None, None, &[
        p("Faça um recorte normal à chapa.", "sm_recorte", Feature("sm_recorte", 1)),
        p("Confira o alívio de canto onde duas dobras se encontram.", "sm_alivio_canto", Manual),
    ]),
    a("ch_planificar", "Planificação", "Desdobrar mostra a chapa plana que vai para o corte.", Chapa, Intermediario, Some("Bandeja curva"), None, &[
        p("Abra a peça de exemplo Bandeja curva.", "exemplo", Exemplo("Bandeja curva")),
        p("Desdobre a chapa e leia o comprimento planificado.", "sm_desdobrar", Manual),
    ]),
    a("ch_desenho", "Mapa de dobras e desenho", "A chapa planificada vira vista de desenho com a tabela de dobras.", Chapa, Avancado, None, None, &[
        p("Crie uma vista planificada da peça no desenho.", "des_planificada", Manual),
        p("Insira a tabela de dobras na folha.", "des_tab_dobras", Manual),
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
        p("Gere a lista de corte com comprimentos e ângulos.", "st_corte", Manual),
        p("Gere a montagem com um componente por membro.", "st_montagem", Manual),
    ]),
    // ---- Desenho
    a("des_folha", "Folha e carimbo", "O desenho nasce numa folha com formato, escala e carimbo.", Desenho, Iniciante, None, None, &[
        p("Crie um desenho novo; ele nasce com uma folha.", "des_novo", Folhas(1)),
        p("Escolha o formato da folha e preencha o carimbo.", "des_folha", Manual),
    ]),
    a("des_vistas", "Vistas", "As vistas projetam o modelo 3D e continuam ligadas a ele.", Desenho, Iniciante, Some("Placa paramétrica"), None, &[
        p("Insira a vista principal da peça.", "des_vista", Vistas(1)),
        p("Insira as três vistas ortogonais.", "des_tres_vistas", Vistas(3)),
    ]),
    a("des_cotas", "Cotas e anotações", "Cotas leem a medida do modelo; notas e símbolos explicam o resto.", Desenho, Intermediario, None, None, &[
        p("Coloque uma cota horizontal.", "des_cota_h", Cotas(1)),
        p("Acrescente uma nota ou um símbolo.", "des_nota", Manual),
    ]),
    a("des_tabelas", "Tabelas e exportação", "Tabelas leem o documento; a exportação gera o arquivo que vai para a fábrica.", Desenho, Intermediario, None, None, &[
        p("Insira uma tabela de materiais.", "des_bom", Tabelas(1)),
        p("Exporte a folha em PDF.", "des_pdf", Manual),
    ]),
];

pub const TRILHAS: &[Trilha] = &[
    Trilha { id: "primeiros_passos", titulo: "Primeiros passos", resumo: "Do esboço ao sólido acabado, em quatro aulas curtas.", nivel: Iniciante, aulas: &["sol_esboco", "sol_volume", "sol_furo", "sol_acabamento"] },
    Trilha { id: "solidos", titulo: "Modelagem sólida", resumo: "Tudo o que a bancada de sólidos faz, na ordem em que se usa.", nivel: Intermediario, aulas: &["sol_esboco", "sol_volume", "sol_furo", "sol_acabamento", "sol_repetir", "sol_casca"] },
    Trilha { id: "superficies", titulo: "Superfícies", resumo: "Curvas, superfícies, união e verificação de continuidade.", nivel: Avancado, aulas: &["sup_curvas", "sup_criar", "sup_juntar", "sup_verificar"] },
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
        Bench::FormaLivre => "superficies",
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
}

impl<'a> Contexto<'a> {
    pub fn so_documento(doc: &'a Document) -> Self {
        Contexto { doc, componentes: 0, restricoes: 0, folhas: 0, vistas: 0, cotas: 0, tabelas: 0 }
    }
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
            Exemplo(nome) => c.doc.name == nome,
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
        crate::examples::all().into_iter().find(|e| e.name == nome).unwrap_or_else(|| panic!("exemplo {nome}")).doc
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
        for b in [Bench::Solidos, Bench::Superficies, Bench::Montagem, Bench::Chapa, Bench::Estrutural, Bench::Desenho] {
            let t = trilha_da_bancada(b);
            assert!(!t.aulas.is_empty());
            assert_eq!(aulas_da_trilha(t).len(), aulas_da_bancada(b).len(), "{b:?}");
            assert!(aulas_da_trilha(t).iter().all(|a| a.bancada == b));
        }
    }

    #[test]
    fn exemplos_icones_e_midias_citados_existem() {
        let exemplos: Vec<_> = crate::examples::all();
        let icones: Vec<&str> = crate::ajuda_cmd::comandos().iter().map(|i| i.icon).collect();
        let roteiros = std::fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/media/roteiros")).unwrap().filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().trim_end_matches(".json").to_string()).collect::<Vec<_>>();
        for a in AULAS {
            if let Some(ex) = a.exemplo {
                assert!(exemplos.iter().any(|e| e.name == ex), "{}: exemplo {ex} não existe", a.id);
            }
            if let Some(m) = a.midia {
                assert!(roteiros.iter().any(|r| r == m), "{}: roteiro de mídia {m} não existe", a.id);
            }
            for (i, p) in a.passos.iter().enumerate() {
                if !p.icone.is_empty() && p.icone != "exemplo" {
                    assert!(icones.contains(&p.icone), "{} passo {i}: ícone {} não é de nenhum comando", a.id, p.icone);
                }
                if let Exemplo(n) = p.cond {
                    assert!(exemplos.iter().any(|e| e.doc.name == n), "{} passo {i}: nenhuma peça de exemplo se chama {n}", a.id);
                    assert_eq!(a.exemplo.and_then(|x| exemplos.iter().find(|e| e.name == x)).map(|e| e.doc.name.as_str()), Some(n), "{}: a peça do passo é a do exemplo da aula", a.id);
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
        assert_eq!(e.proxima_da_trilha(t).unwrap().id, "sol_esboco");
        e.pular_aula(aula("sol_esboco").unwrap());
        assert_eq!(e.proxima_da_trilha(t).unwrap().id, "sol_volume");
        assert_eq!(e.andamento_trilha(t), (1, 4));
        // "próxima" segue a ordem a partir da aula atual e só volta ao começo no fim
        assert_eq!(e.proxima_depois(t, "sol_volume").unwrap().id, "sol_furo");
        e.pular_aula(aula("sol_furo").unwrap());
        e.pular_aula(aula("sol_acabamento").unwrap());
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
