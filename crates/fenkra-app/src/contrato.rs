// SPDX-License-Identifier: LGPL-2.1-or-later
//! ADR 0107, onda 0: o descritor de feature (`FeatureSpec`).
//!
//! Cada comando diz, por dados, quais campos de entrada tem: o tipo que aceita (face, aresta, curva,
//! superfície, plano, eixo, ponto, sketch, corpo, feature), quantos itens (um, vários com mínimo e
//! máximo, ou uma cadeia), se a ordem importa, se é obrigatório e um validador. Do descritor saem, por
//! consultas, as quatro coisas que antes eram três regras espalhadas (`campos.rs`,
//! `entradas::campo_ativo`, `slots::fill`):
//!
//! - o **campo ativo**: a primeira entrada obrigatória ainda sem o mínimo, na ordem declarada;
//! - o **filtro de seleção** temporário do 3D: os tipos aceitos pelo campo ativo;
//! - a **ordem do Tab**: a ordem declarada das entradas;
//! - o estado **incompleto**: alguma entrada obrigatória abaixo do mínimo, ou um campo ainda com o
//!   marcador de "a escolher".
//!
//! O descritor é montado **a partir do valor atual do comando** (`FeatureSpec::de`): cada entrada
//! já traz `n`, quantos itens reais o campo tem. Assim o campo vazio é um estado explícito (`n` abaixo
//! do mínimo) e nenhum chamador compara `[0;3]`, id sentinela ou texto de erro.
//!
//! **Adaptador.** Os comandos que ainda não declararam suas entradas continuam com os marcadores
//! `UNSET_*` dentro do comando; o adaptador (`adaptadas`) lê esses marcadores e os transforma em
//! entradas do mesmo descritor, na ordem de prioridade de sempre. Quem consulta não sabe a diferença.
//! Conforme cada onda migra um comando, ele passa para `declaradas` e some do adaptador.

use crate::entradas::Espera;
use crate::i18n::tr;
use fenkra_core::*;

// ---------------------------------------------------------------- o vazio, num lugar só

/// O ponto 3D "ainda não escolhi" dos campos que guardam só coordenadas (aresta da aba, face da
/// rosca, linha da dobra). Fica num lugar só: trocar a representação (um `Option`, uma variante
/// `Vazio`) muda esta constante e este predicado, não as dezenas de comparações com `[0.; 3]`.
pub const PONTO_VAZIO: [f64; 3] = [0.; 3];

/// O ponto é o marcador de campo vazio?
pub fn ponto_vazio(p: &[f64; 3]) -> bool {
    *p == PONTO_VAZIO
}

fn n_ponto(p: &[f64; 3]) -> usize {
    usize::from(!ponto_vazio(p))
}

fn n_pontos<'a>(ps: impl IntoIterator<Item = &'a [f64; 3]>) -> usize {
    ps.into_iter().filter(|p| !ponto_vazio(p)).count()
}

fn n_id(id: Id) -> usize {
    usize::from(!id.a_escolher())
}

fn n_surf(s: &SurfRef) -> usize {
    usize::from(!matches!(s, SurfRef::Ref(id) if id.a_escolher()))
}

fn n_curva(c: &CurveRef) -> usize {
    usize::from(!matches!(c, CurveRef::Ref(id) if id.a_escolher()))
}

// ---------------------------------------------------------------- tipos

/// O que um campo de entrada aceita.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum TipoEntrada {
    Face,
    Aresta,
    Curva,
    Superficie,
    Plano,
    Eixo,
    Ponto,
    Sketch,
    Corpo,
    Feature,
}

impl TipoEntrada {
    /// O mesmo tipo na linguagem do painel (cor, rótulo, dica).
    pub fn espera(self) -> Espera {
        match self {
            TipoEntrada::Face => Espera::Face,
            TipoEntrada::Aresta => Espera::Aresta,
            TipoEntrada::Curva => Espera::Curva,
            TipoEntrada::Superficie => Espera::Superficie,
            TipoEntrada::Plano => Espera::Plano,
            TipoEntrada::Eixo => Espera::Eixo,
            TipoEntrada::Ponto => Espera::Ponto,
            TipoEntrada::Sketch => Espera::Perfil,
            TipoEntrada::Corpo | TipoEntrada::Feature => Espera::Corpo,
        }
    }

    /// Nome do tipo, no singular, para as mensagens ("esta caixa aceita arestas").
    pub fn rotulo(self) -> &'static str {
        match self {
            TipoEntrada::Face => "face",
            TipoEntrada::Aresta => "aresta",
            TipoEntrada::Curva => "curva",
            TipoEntrada::Superficie => "superfície",
            TipoEntrada::Plano => "plano",
            TipoEntrada::Eixo => "eixo",
            TipoEntrada::Ponto => "ponto",
            TipoEntrada::Sketch => "sketch",
            TipoEntrada::Corpo => "corpo",
            TipoEntrada::Feature => "recurso",
        }
    }

    /// Um campo deste tipo aceita o que foi clicado? Uma aresta é uma curva, uma face é uma
    /// superfície, uma face plana vira plano, uma aresta ou face pode dar eixo e ponto (centro), e
    /// uma curva clicada pode ser a origem de um perfil (o sketch dela).
    pub fn aceita(self, clicado: TipoEntrada) -> bool {
        use TipoEntrada::*;
        self == clicado
            || matches!(
                (self, clicado),
                (Curva, Aresta) | (Aresta, Curva) | (Superficie, Face) | (Face, Superficie) | (Superficie, Plano) | (Plano, Face) | (Eixo, Aresta | Curva | Face) | (Ponto, Aresta | Face) | (Sketch, Curva) | (Corpo, Feature) | (Feature, Corpo)
            )
    }
}

/// Quantos itens o campo guarda.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cardinalidade {
    /// exatamente um
    Um,
    /// lista com mínimo e, se houver, máximo
    Varios { min: usize, max: Option<usize> },
    /// uma cadeia de curvas ou arestas ligadas ponta a ponta (conta como um item)
    Cadeia,
}

impl Cardinalidade {
    pub fn min(self) -> usize {
        match self {
            Cardinalidade::Um | Cardinalidade::Cadeia => 1,
            Cardinalidade::Varios { min, .. } => min,
        }
    }

    pub fn max(self) -> Option<usize> {
        match self {
            Cardinalidade::Um | Cardinalidade::Cadeia => Some(1),
            Cardinalidade::Varios { max, .. } => max,
        }
    }

    /// Campo que acumula itens com a caixa de seleção (e não um slot de um elemento).
    pub fn e_lista(self) -> bool {
        !matches!(self, Cardinalidade::Um)
    }
}

/// Por que um item foi recusado. A chave é o texto em português (traduzido por `tr`) com `{}` onde
/// entram os argumentos, na ordem; nada de `format!` que a tradução não alcança.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Motivo {
    pub chave: &'static str,
    pub args: Vec<String>,
}

impl Motivo {
    pub fn novo(chave: &'static str, args: Vec<String>) -> Self {
        Motivo { chave, args }
    }

    /// A mensagem no idioma ativo.
    pub fn texto(&self) -> String {
        let mut s = tr(self.chave).to_string();
        for a in &self.args {
            s = s.replacen("{}", a, 1);
        }
        s
    }
}

/// O que a seleção ofereceu a um campo: o tipo, de onde veio e como se chama. É o que o validador
/// lê (o campo já tem os itens que entraram antes).
#[derive(Clone, PartialEq, Debug)]
pub struct Candidato {
    pub tipo: TipoEntrada,
    /// sketch, comando ou coletor de onde o item vem
    pub origem: Option<Id>,
    pub nome: String,
}

/// Validador de um campo: recusa o candidato (com o motivo) ou aceita. `no_campo` são os itens que
/// o campo já tem.
pub type Validador = fn(&Candidato, &[Candidato]) -> Result<(), Motivo>;

/// Um campo de entrada de um comando.
#[derive(Clone, Debug)]
pub struct Entrada {
    /// identificador estável: o do widget (`salt`) na tela, o das mensagens e dos testes
    pub id: &'static str,
    pub rotulo: &'static str,
    /// dica de três linhas (o que faz, faixa, efeito); vazia onde o painel ainda usa a dela
    pub dica: &'static str,
    pub tipo: TipoEntrada,
    /// tipos a mais que o campo aceita (o filete aceita face e escolhe as arestas do contorno)
    pub tambem: Vec<TipoEntrada>,
    pub card: Cardinalidade,
    /// a ordem dos itens importa (seções do loft, nó de tubos): a caixa mostra a posição e deixa
    /// reordenar
    pub ordenado: bool,
    pub obrigatoria: bool,
    /// o campo aceita junção na hora (subitens unidos no próprio campo) e cadeia tangente
    pub juncao: bool,
    /// o painel mostra "Faces escolhidas: n" para este campo
    pub contar: bool,
    /// com este campo ativo o 3D só deixa clicar nos tipos que ele aceita (filtro temporário)
    pub filtra: bool,
    pub validador: Option<Validador>,
    /// quantos itens reais o campo tem agora (o estado "vazio" é `n < min`)
    pub n: usize,
    /// vem da ficha declarada do comando (tem widget próprio); falso nas entradas que o adaptador lê
    /// dos marcadores "a escolher"
    pub declarada: bool,
}

impl Entrada {
    pub fn nova(id: &'static str, rotulo: &'static str, tipo: TipoEntrada, card: Cardinalidade, n: usize) -> Self {
        Entrada { id, rotulo, dica: "", tipo, tambem: vec![], card, ordenado: false, obrigatoria: true, juncao: false, contar: false, filtra: false, validador: None, n, declarada: true }
    }

    pub fn ordenada(mut self) -> Self {
        self.ordenado = true;
        self
    }

    pub fn opcional(mut self) -> Self {
        self.obrigatoria = false;
        self
    }

    pub fn contando(mut self) -> Self {
        self.contar = true;
        self
    }

    pub fn filtrando(mut self) -> Self {
        self.filtra = true;
        self
    }

    pub fn com_juncao(mut self) -> Self {
        self.juncao = true;
        self
    }

    pub fn tambem(mut self, t: TipoEntrada) -> Self {
        self.tambem.push(t);
        self
    }

    pub fn dica(mut self, d: &'static str) -> Self {
        self.dica = d;
        self
    }

    pub fn valida(mut self, v: Validador) -> Self {
        self.validador = Some(v);
        self
    }

    /// Falta item: obrigatório e abaixo do mínimo.
    pub fn vazio(&self) -> bool {
        self.obrigatoria && self.n < self.card.min()
    }

    /// Quantos itens ainda cabem (`None` = sem limite).
    pub fn cabem(&self) -> Option<usize> {
        self.card.max().map(|m| m.saturating_sub(self.n))
    }

    /// O campo aceita um clique deste tipo?
    pub fn aceita(&self, t: TipoEntrada) -> bool {
        self.tipo.aceita(t) || self.tambem.iter().any(|x| x.aceita(t))
    }

    /// O campo é uma lista que acumula (caixa de seleção) e não um slot de um elemento?
    pub fn e_lista(&self) -> bool {
        self.card.e_lista()
    }

    /// O campo de lista já está no máximo com `n` itens? (A cadeia não tem máximo: ela conta como um
    /// item mas guarda quantas curvas forem.)
    pub fn cheio(&self, n: usize) -> Option<Motivo> {
        match self.card {
            Cardinalidade::Varios { max: Some(m), .. } if n >= m => Some(Motivo::novo("Este campo já tem o máximo de {} itens.", vec![m.to_string()])),
            _ => None,
        }
    }

    /// Valida um candidato: o tipo, a quantidade que ainda cabe (`n_atual` itens reais no campo) e o
    /// validador do campo. `no_campo` são os itens que o campo já tem.
    pub fn validar(&self, c: &Candidato, no_campo: &[Candidato], n_atual: usize) -> Result<(), Motivo> {
        if !self.aceita(c.tipo) {
            return Err(Motivo::novo("Esta caixa aceita {}; o item clicado é {}.", vec![self.aceitos(), tr(c.tipo.rotulo()).to_string()]));
        }
        if let Some(m) = self.cheio(n_atual) {
            return Err(m);
        }
        match self.validador {
            Some(v) => v(c, no_campo),
            None => Ok(()),
        }
    }

    /// Os tipos aceitos, em texto ("arestas ou faces").
    pub fn aceitos(&self) -> String {
        let mut v: Vec<&str> = vec![tr(self.tipo.rotulo())];
        v.extend(self.tambem.iter().map(|t| tr(t.rotulo())));
        v.join(&format!(" {} ", tr("ou")))
    }
}

/// O que se consulta: um comando de `features` ou um elemento de `references`.
#[derive(Clone, Copy, Debug)]
pub enum Alvo<'a> {
    Feature(&'a FeatureKind),
    Ref(&'a RefKind),
}

/// O descritor de um comando, já ligado ao valor atual.
#[derive(Clone, Debug, Default)]
pub struct FeatureSpec {
    /// entradas na ordem declarada: é a ordem do Tab e a de preenchimento
    pub entradas: Vec<Entrada>,
}

impl FeatureSpec {
    /// O descritor do comando `alvo`: as entradas declaradas e, depois delas, as que o adaptador lê
    /// dos marcadores "a escolher" do comando.
    pub fn de(alvo: Alvo) -> Self {
        let mut entradas = declaradas(alvo);
        let declarados: Vec<&'static str> = entradas.iter().map(|e| e.id).collect();
        entradas.extend(adaptadas(alvo).into_iter().filter(|e| !declarados.contains(&e.id)));
        FeatureSpec { entradas }
    }

    /// O campo ativo: a primeira entrada obrigatória ainda sem o mínimo.
    pub fn ativa(&self) -> Option<&Entrada> {
        self.entradas.iter().find(|e| e.vazio())
    }

    /// Falta algum campo (o comando ainda espera a seleção).
    pub fn incompleto(&self) -> bool {
        self.ativa().is_some()
    }

    /// A ordem do Tab: os campos declarados do comando, na ordem declarada.
    pub fn ordem_tab(&self) -> Vec<&'static str> {
        self.entradas.iter().filter(|e| e.declarada).map(|e| e.id).collect()
    }

    /// Com o comando completo o filtro continua no campo de lista (a pessoa segue somando itens).
    fn do_filtro(&self) -> Option<&Entrada> {
        self.ativa().or_else(|| self.entradas.iter().find(|e| e.e_lista()))
    }

    pub fn entrada(&self, id: &str) -> Option<&Entrada> {
        self.entradas.iter().find(|e| e.id == id)
    }
}

// ---------------------------------------------------------------- consultas

/// O campo que o próximo clique no 3D preenche (`None` = o comando tem tudo o que precisa).
pub fn campo_ativo(alvo: Alvo) -> Option<Espera> {
    FeatureSpec::de(alvo).ativa().map(|e| e.tipo.espera())
}

/// O comando ainda espera a seleção?
pub fn incompleto(alvo: Alvo) -> bool {
    FeatureSpec::de(alvo).incompleto()
}

/// Id (do widget) do campo ativo, quando ele é um campo declarado. O adaptador não tem widget
/// próprio: devolve `None` e a ordem do painel (`campos.rs`) decide como sempre.
pub fn id_do_ativo(alvo: Alvo) -> Option<&'static str> {
    let s = FeatureSpec::de(alvo);
    let e = s.ativa()?;
    e.declarada.then_some(e.id)
}

/// Quantos itens o campo de lista do comando já tem, para o painel.
pub fn contagem(alvo: Alvo) -> Option<(Espera, usize)> {
    FeatureSpec::de(alvo).entradas.iter().find(|e| e.contar).map(|e| (e.tipo.espera(), e.n))
}

/// O filtro temporário do 3D: o que o campo ativo aceita.
pub fn filtro(alvo: Alvo) -> Option<crate::selecao::TiposTemporarios> {
    let s = FeatureSpec::de(alvo);
    let e = s.do_filtro().filter(|e| e.filtra)?;
    let aceita = |t: TipoEntrada| e.aceita(t);
    let faces = aceita(TipoEntrada::Face) || aceita(TipoEntrada::Superficie);
    let arestas = aceita(TipoEntrada::Aresta) || aceita(TipoEntrada::Curva);
    let vertices = aceita(TipoEntrada::Ponto);
    (faces || arestas || vertices).then_some(crate::selecao::TiposTemporarios { faces, arestas, vertices })
}

/// O tipo do que o usuário clicou, lido da seleção (`Seed`), para a caixa recusar o que não serve.
pub fn tipos_clicados(seed: &crate::geo_ui::Seed) -> Vec<TipoEntrada> {
    let mut v = vec![];
    if !seed.curves.is_empty() {
        v.push(TipoEntrada::Curva);
    }
    if seed.surface.is_some() {
        v.push(TipoEntrada::Superficie);
    }
    if !seed.planes.is_empty() {
        v.push(TipoEntrada::Plano);
    }
    if !seed.points.is_empty() {
        v.push(TipoEntrada::Ponto);
    }
    v
}

/// A seleção só tem coisas que o campo não aceita? Devolve a mensagem para o campo mostrar (o clique
/// de tipo errado não entra). Seleção vazia, ou com ao menos um item aceito, não é erro.
pub fn recusa_por_tipo(e: &Entrada, clicados: &[TipoEntrada]) -> Option<Motivo> {
    if clicados.is_empty() || clicados.iter().any(|t| e.aceita(*t)) {
        return None;
    }
    let achou: Vec<&str> = clicados.iter().map(|t| tr(t.rotulo())).collect();
    Some(Motivo::novo("Esta caixa aceita {}; o item clicado é {}.", vec![e.aceitos(), achou.join(", ")]))
}

/// O descritor do comando `id` do documento (feature ou referência), ligado ao valor atual.
pub fn spec_no_doc(doc: &Document, id: Id) -> Option<FeatureSpec> {
    match (doc.feature(id), doc.reference(id)) {
        (Some(f), _) => Some(FeatureSpec::de(Alvo::Feature(&f.kind))),
        (_, Some(r)) => Some(FeatureSpec::de(Alvo::Ref(&r.kind))),
        _ => None,
    }
}

/// A entrada do comando `id` a que o widget `salt` pertence, se o comando a declara.
pub fn entrada_do_doc(doc: &Document, id: Id, salt: &str) -> Option<Entrada> {
    let s = spec_no_doc(doc, id)?;
    let decl = match (doc.feature(id), doc.reference(id)) {
        (Some(f), _) => declaradas(Alvo::Feature(&f.kind)),
        (_, Some(r)) => declaradas(Alvo::Ref(&r.kind)),
        _ => return None,
    };
    s.entradas.into_iter().find(|e| salt_bate(salt, e.id) && decl.iter().any(|d| d.id == e.id))
}

/// Id do campo ativo declarado pelo comando `id` do documento.
pub fn id_do_ativo_no_doc(doc: &Document, id: Id) -> Option<&'static str> {
    match (doc.feature(id), doc.reference(id)) {
        (Some(f), _) => id_do_ativo(Alvo::Feature(&f.kind)),
        (_, Some(r)) => id_do_ativo(Alvo::Ref(&r.kind)),
        _ => None,
    }
}

/// A lista de curvas do campo `campo` do comando, para o motor do campo (cadeia tangente, junção)
/// mexer nela. Só os campos de curva com lista.
pub fn curvas_mut<'a>(doc: &'a mut Document, id: Id, campo: &str) -> Option<&'a mut Vec<CurveRef>> {
    if doc.reference(id).is_some() {
        let r = doc.references.iter_mut().find(|r| r.id == id)?;
        return match (&mut r.kind, campo) {
            (RefKind::SurfRuled { curves, .. }, "g_rc") => Some(curves),
            (RefKind::SurfLoft { sections, .. }, "g_sec") => Some(sections),
            (RefKind::CurveJoin { curves }, "g_cj") => Some(curves),
            (RefKind::SurfJoin { curves, .. }, "g_jc") => Some(curves),
            (RefKind::SurfExtrapolate { edges, .. }, "g_eb") => Some(edges),
            _ => None,
        };
    }
    match (&mut doc.feature_mut(id)?.kind, campo) {
        (FeatureKind::StructMember(m), "caminhos") => Some(&mut m.paths),
        _ => None,
    }
}

/// O `salt` do widget é o campo `id` do descritor? O widget pode levar o id do comando no fim
/// (`arestas12`): o resto depois do id tem de ser só dígitos.
pub fn salt_bate(salt: &str, id: &str) -> bool {
    salt.strip_prefix(id).is_some_and(|r| r.chars().all(|c| c.is_ascii_digit()))
}

// ---------------------------------------------------------------- entradas declaradas

fn varios(min: usize) -> Cardinalidade {
    Cardinalidade::Varios { min, max: None }
}

/// Entradas que o comando declara (as migradas). Cada `id` é o `salt` do widget que a desenha.
fn declaradas(alvo: Alvo) -> Vec<Entrada> {
    use TipoEntrada as T;
    match alvo {
        Alvo::Feature(k) => match k {
            FeatureKind::Shell(x) => vec![Entrada::nova("faces", "Faces", T::Face, varios(1), x.faces.len()).contando().filtrando()],
            FeatureKind::Draft(x) => vec![Entrada::nova("faces", "Faces", T::Face, varios(1), x.faces.len()).contando().filtrando()],
            FeatureKind::Thickness(x) => vec![Entrada::nova("faces", "Faces", T::Face, varios(1), x.faces.len()).contando().filtrando()],
            FeatureKind::DirectEdit(x) => vec![Entrada::nova("faces", "Faces", T::Face, varios(1), x.faces.len()).contando().filtrando()],
            FeatureKind::Thread(t) => vec![Entrada::nova("face", "Face", T::Face, Cardinalidade::Um, n_ponto(&t.face)).contando().filtrando()],
            // B243: dobra por linha, joggle e veneziana pedem a face fixa enquanto não têm a linha
            FeatureKind::SmBend(x) if x.sketch.is_none() => vec![Entrada::nova("face", "Face", T::Face, Cardinalidade::Um, usize::from(x.a != x.b))],
            FeatureKind::SmJoggle(x) if x.plane.is_none() => vec![Entrada::nova("face", "Face", T::Face, Cardinalidade::Um, usize::from(x.a != x.b))],
            FeatureKind::SmLouver(x) => vec![Entrada::nova("face", "Face", T::Face, Cardinalidade::Um, usize::from(x.a != x.b))],
            // B31: a aba curva vinda de outro modo ainda não tem a cadeia de arestas
            FeatureKind::SmCurveFlange(x) => vec![Entrada::nova("aresta", "Aresta", T::Aresta, Cardinalidade::Cadeia, usize::from(x.edge.len() >= 2)).tambem(T::Curva).tambem(T::Face)],
            // B514: o laço de abas aberto sem aresta espera o clique numa aresta
            FeatureKind::SmFlangeLoop(x) => vec![Entrada::nova("smloop_arestas", "Arestas", T::Aresta, varios(1), n_pontos(x.edges.iter())).tambem(T::Face)],
            FeatureKind::SmFlange(f) => vec![Entrada::nova("smaba_aresta", "Aresta", T::Aresta, Cardinalidade::Um, n_ponto(&f.edge)).tambem(T::Face)],
            FeatureKind::SmHem(h) => vec![Entrada::nova("smaba_aresta", "Aresta", T::Aresta, Cardinalidade::Um, n_ponto(&h.edge))],
            FeatureKind::Fillet(o) | FeatureKind::Chamfer(o) => match (&o.mode, &o.edges) {
                (EdgeMode::FaceFace(a, b), _) => vec![Entrada::nova("faces", "Faces", T::Face, Cardinalidade::Varios { min: 2, max: Some(2) }, n_pontos([a, b])).contando().filtrando()],
                (EdgeMode::FullRound { mid, a, b }, _) => vec![Entrada::nova("faces", "Faces", T::Face, Cardinalidade::Varios { min: 3, max: Some(3) }, n_pontos([mid, a, b])).contando().filtrando()],
                (_, EdgeSel::Edges(m)) => vec![Entrada::nova("arestas", "Arestas", T::Aresta, varios(1), m.len()).tambem(T::Face).contando().filtrando()],
                _ => vec![],
            },
            // as seções do loft: a ordem é a do corpo
            FeatureKind::Loft(l) => vec![Entrada::nova("secoes", "Seções", T::Sketch, varios(2), l.sections.iter().filter(|s| !s.a_escolher()).count()).ordenada().com_juncao()],
            FeatureKind::StructMember(m) => vec![Entrada::nova("caminhos", "Caminho", T::Curva, varios(1), m.paths.len()).com_juncao()],
            _ => vec![],
        },
        Alvo::Ref(k) => match k {
            RefKind::SurfSplit { surface, tool, .. } => vec![
                Entrada::nova("g_s", "Superfície", T::Superficie, Cardinalidade::Um, n_surf(surface)),
                Entrada::nova("g_t", "Cortar com", T::Superficie, Cardinalidade::Um, n_surf(tool)).tambem(T::Plano),
            ],
            RefKind::SurfTrim { a, b, .. } | RefKind::SurfFillet { a, b, .. } => vec![
                Entrada::nova("g_a", "Superfície 1", T::Superficie, Cardinalidade::Um, n_surf(a)),
                Entrada::nova("g_b", "Superfície 2", T::Superficie, Cardinalidade::Um, n_surf(b)),
            ],
            RefKind::SurfRuled { curves, .. } => vec![Entrada::nova("g_rc", "Curvas", T::Curva, varios(1), curves.iter().map(n_curva).sum()).com_juncao()],
            RefKind::CurveParallel { curve, support, .. } => vec![
                Entrada::nova("g_c", "Curva", T::Curva, Cardinalidade::Um, n_curva(curve)).com_juncao(),
                Entrada::nova("g_s", "Apoio", T::Superficie, Cardinalidade::Um, n_surf(support)),
            ],
            RefKind::SurfJoin { items, curves, .. } => vec![
                Entrada::nova("g_j", "Elementos", T::Superficie, varios(1), items.iter().map(n_surf).sum()),
                Entrada::nova("g_jc", "Curvas", T::Curva, varios(0), curves.iter().map(n_curva).sum()).opcional().com_juncao(),
            ],
            RefKind::SurfLoft { sections, .. } => vec![Entrada::nova("g_sec", "Seções", T::Curva, varios(2), sections.iter().map(n_curva).sum()).ordenada().com_juncao()],
            RefKind::CurveJoin { curves } => vec![Entrada::nova("g_cj", "Curvas", T::Curva, varios(1), curves.iter().map(n_curva).sum()).com_juncao()],
            _ => vec![],
        },
    }
}

// ---------------------------------------------------------------- adaptador dos marcadores

/// Entradas lidas dos marcadores `UNSET_*` do comando (os que ainda não declararam o descritor), na
/// ordem de prioridade de sempre: perfil, face, aresta, corpo, eixo, plano, curva, superfície, ponto,
/// e, dentro do mesmo tipo, na ordem em que o comando declara os campos.
fn adaptadas(alvo: Alvo) -> Vec<Entrada> {
    let falta = match alvo {
        Alvo::Feature(k) => crate::slots::vazios_em_ordem(k),
        Alvo::Ref(k) => crate::slots::vazios_em_ordem(k),
    };
    let mut v: Vec<(Espera, usize)> = falta.into_iter().enumerate().filter_map(|(i, u)| espera_do_marcador(u).map(|e| (e, i))).collect();
    v.sort_by_key(|(e, i)| (prioridade(*e), *i));
    v.into_iter().map(|(e, _)| marcador(e)).collect()
}

fn espera_do_marcador(u: u64) -> Option<Espera> {
    use crate::slots::*;
    Some(match u {
        x if x == UNSET_SKETCH.0 => Espera::Perfil,
        x if x == UNSET_AXIS.0 => Espera::Eixo,
        x if x == UNSET_PLANE.0 => Espera::Plano,
        x if x == UNSET_POINT.0 => Espera::Ponto,
        x if x == UNSET_CURVE.0 => Espera::Curva,
        x if x == UNSET_SURF.0 => Espera::Superficie,
        _ => return None,
    })
}

/// Ordem em que os campos de um comando são pedidos.
fn prioridade(e: Espera) -> u8 {
    match e {
        Espera::Perfil => 0,
        Espera::Face => 1,
        Espera::Aresta => 2,
        Espera::Corpo => 3,
        Espera::Eixo => 4,
        Espera::Plano => 5,
        Espera::Curva => 6,
        Espera::Superficie => 7,
        Espera::Ponto => 8,
    }
}

/// Uma entrada vazia do tipo `e`, vinda de um marcador. O `id` é o do tipo: os marcadores não têm
/// widget próprio, então o campo ativo deles segue a ordem do painel.
fn marcador(e: Espera) -> Entrada {
    let (id, tipo) = match e {
        Espera::Perfil => ("marcador_perfil", TipoEntrada::Sketch),
        Espera::Eixo => ("marcador_eixo", TipoEntrada::Eixo),
        Espera::Plano => ("marcador_plano", TipoEntrada::Plano),
        Espera::Ponto => ("marcador_ponto", TipoEntrada::Ponto),
        Espera::Curva => ("marcador_curva", TipoEntrada::Curva),
        Espera::Superficie => ("marcador_superficie", TipoEntrada::Superficie),
        Espera::Face => ("marcador_face", TipoEntrada::Face),
        Espera::Aresta => ("marcador_aresta", TipoEntrada::Aresta),
        Espera::Corpo => ("marcador_corpo", TipoEntrada::Corpo),
    };
    Entrada { declarada: false, ..Entrada::nova(id, e.rotulo(), tipo, Cardinalidade::Um, 0) }
}

// ---------------------------------------------------------------- clique na árvore de recursos

/// O item clicado na árvore de recursos, como os cliques do 3D que o campo ativo entende: o clique na
/// árvore e o clique no 3D entram no campo pelo mesmo caminho. Elementos de geometria entram como
/// `Pick::Geo`, planos da origem como `Pick::Plane`, e um sketch inteiro entra com as curvas dele (uma
/// `Pick::SkCurve` por curva). Lista vazia = a árvore não tem como dar este item a um campo (corpo,
/// pasta, parâmetro).
pub fn picks_da_arvore(doc: &Document, r: Option<&fenkra_solidos::RecomputeResult>, sel: &crate::app::Sel) -> Vec<crate::picking::Pick> {
    use crate::app::Sel;
    use crate::picking::Pick;
    match *sel {
        Sel::Ref(id) if doc.reference(id).is_some() => vec![Pick::Geo(id)],
        Sel::Origin(o) => vec![Pick::Plane(PlaneRef::Origin(o))],
        Sel::Feature(id) if matches!(doc.feature(id).map(|f| &f.kind), Some(FeatureKind::Sketch(_))) => {
            let Some(sk) = r.and_then(|r| r.sketches.iter().find(|s| s.feature == id)) else { return vec![] };
            sk.ents
                .iter()
                .filter(|(_, construcao, p)| !*construcao && !p.is_empty())
                .map(|(curve, _, p)| Pick::SkCurve { sketch: id, curve: *curve, at: p[p.len() / 2] })
                .collect()
        }
        _ => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::slots;

    fn fillet(mode: EdgeMode, edges: EdgeSel) -> FeatureKind {
        FeatureKind::Fillet(EdgeOp { value: "2".into(), mode, edges, prop: Default::default(), keys: vec![], extra: Default::default() })
    }

    #[test]
    fn filete_por_arestas_pede_arestas_ate_ter_uma() {
        let mut k = fillet(EdgeMode::Constant, EdgeSel::Edges(vec![]));
        let s = FeatureSpec::de(Alvo::Feature(&k));
        assert_eq!(s.ativa().map(|e| e.id), Some("arestas"));
        assert_eq!(campo_ativo(Alvo::Feature(&k)), Some(Espera::Aresta));
        assert!(incompleto(Alvo::Feature(&k)));
        assert_eq!(contagem(Alvo::Feature(&k)), Some((Espera::Aresta, 0)));
        if let FeatureKind::Fillet(o) = &mut k {
            o.edges = EdgeSel::Edges(vec![[1., 2., 3.]]);
        }
        assert_eq!(campo_ativo(Alvo::Feature(&k)), None);
        assert!(!incompleto(Alvo::Feature(&k)));
        // com o comando completo o filtro segue na lista, para a pessoa continuar somando
        let f = filtro(Alvo::Feature(&k)).unwrap();
        assert!(f.arestas && f.faces, "o filete aceita aresta e face (o contorno da face)");
    }

    #[test]
    fn filete_entre_faces_pede_duas_faces_e_nao_arestas() {
        let k = fillet(EdgeMode::FaceFace([1., 0., 0.], PONTO_VAZIO), EdgeSel::Edges(vec![]));
        let s = FeatureSpec::de(Alvo::Feature(&k));
        assert_eq!(s.ativa().map(|e| (e.id, e.n)), Some(("faces", 1)));
        assert_eq!(campo_ativo(Alvo::Feature(&k)), Some(Espera::Face));
        let k = fillet(EdgeMode::FaceFace([1., 0., 0.], [0., 1., 0.]), EdgeSel::Edges(vec![]));
        assert_eq!(campo_ativo(Alvo::Feature(&k)), None, "as duas faces escolhidas: completo");
        let k = fillet(EdgeMode::FullRound { mid: [1., 0., 0.], a: [0., 1., 0.], b: PONTO_VAZIO }, EdgeSel::AllOfBody);
        assert_eq!(FeatureSpec::de(Alvo::Feature(&k)).ativa().map(|e| e.card.min()), Some(3));
    }

    #[test]
    fn todas_as_arestas_nao_pede_clique() {
        let k = fillet(EdgeMode::Constant, EdgeSel::AllOfBody);
        assert!(FeatureSpec::de(Alvo::Feature(&k)).entradas.is_empty());
        assert_eq!(filtro(Alvo::Feature(&k)), None);
    }

    #[test]
    fn marcadores_viram_entradas_na_ordem_de_prioridade() {
        // a extrusão nova sem perfil: o adaptador lê o marcador do sketch
        let k = FeatureKind::Extrude(Extrude { thin_side: 0, sketch: slots::UNSET_SKETCH, op: BoolOp::Add, extent: Extent::Blind("10".into()), reverse: false, taper: "0".into(), thin: None, dir: None, ends: Default::default() });
        assert_eq!(campo_ativo(Alvo::Feature(&k)), Some(Espera::Perfil));
        assert!(incompleto(Alvo::Feature(&k)));
        assert_eq!(id_do_ativo(Alvo::Feature(&k)), None, "o marcador não tem widget próprio");
        // dois marcadores: o ponto vem depois da curva, como sempre
        let k = RefKind::LinePoints { a: slots::point(), b: slots::point(), start: "0".into(), end: "0".into() };
        assert_eq!(FeatureSpec::de(Alvo::Ref(&k)).entradas.len(), 2);
        assert_eq!(campo_ativo(Alvo::Ref(&k)), Some(Espera::Ponto));
    }

    #[test]
    fn superficies_declaram_seus_campos_na_ordem_do_painel() {
        let k = RefKind::SurfSplit { surface: slots::surf(), tool: slots::surf(), flip: false, more: vec![], curve: None, more_flip: vec![] };
        let s = FeatureSpec::de(Alvo::Ref(&k));
        assert_eq!(s.ordem_tab(), vec!["g_s", "g_t"]);
        assert_eq!(s.ativa().map(|e| e.id), Some("g_s"));
        assert_eq!(id_do_ativo(Alvo::Ref(&k)), Some("g_s"));
        // o primeiro campo preenchido: a vez passa à ferramenta
        let k = RefKind::SurfSplit { surface: SurfRef::Ref(Id(5)), tool: slots::surf(), flip: false, more: vec![], curve: None, more_flip: vec![] };
        assert_eq!(id_do_ativo(Alvo::Ref(&k)), Some("g_t"));
        // as duas preenchidas: completo, sem sobrar marcador
        let k = RefKind::SurfSplit { surface: SurfRef::Ref(Id(5)), tool: SurfRef::Ref(Id(6)), flip: false, more: vec![], curve: None, more_flip: vec![] };
        assert!(!incompleto(Alvo::Ref(&k)));
    }

    #[test]
    fn lista_com_marcador_continua_incompleta_pelo_adaptador() {
        // dois itens, um deles ainda vazio: o declarado conta 1 (mínimo 1 satisfeito) mas o marcador pede
        let k = RefKind::SurfJoin { items: vec![SurfRef::Ref(Id(4)), slots::surf()], tol: "0.01".into(), curves: vec![] };
        assert!(incompleto(Alvo::Ref(&k)));
        assert_eq!(campo_ativo(Alvo::Ref(&k)), Some(Espera::Superficie));
    }

    #[test]
    fn ordenado_e_juncao_saem_do_descritor() {
        let k = RefKind::SurfLoft { sections: vec![], guide: None, ruled: false, closed: false, opts: Default::default() };
        let s = FeatureSpec::de(Alvo::Ref(&k));
        let e = s.entrada("g_sec").unwrap();
        assert!(e.ordenado && e.juncao);
        assert_eq!(e.card.min(), 2);
        assert_eq!(s.ativa().map(|e| e.id), Some("g_sec"));
    }

    #[test]
    fn validacao_recusa_tipo_errado_e_excesso() {
        let e = Entrada::nova("arestas", "Arestas", TipoEntrada::Aresta, varios(1), 0).tambem(TipoEntrada::Face);
        let face = Candidato { tipo: TipoEntrada::Face, origem: None, nome: "Face 1".into() };
        let ponto = Candidato { tipo: TipoEntrada::Ponto, origem: None, nome: "P".into() };
        assert!(e.validar(&face, &[], 0).is_ok());
        let m = e.validar(&ponto, &[], 0).unwrap_err();
        assert!(m.texto().contains("aresta") && m.texto().contains("ponto"), "{}", m.texto());
        // a mensagem pela seleção clicada
        assert!(recusa_por_tipo(&e, &[TipoEntrada::Ponto]).is_some());
        assert!(recusa_por_tipo(&e, &[TipoEntrada::Ponto, TipoEntrada::Face]).is_none(), "ao menos um item aceito");
        assert!(recusa_por_tipo(&e, &[]).is_none());
        // quantidade: o campo de um item só não cabe mais
        let dois = Entrada::nova("faces", "Faces", TipoEntrada::Face, Cardinalidade::Varios { min: 2, max: Some(2) }, 2);
        assert!(dois.validar(&face, &[], 2).unwrap_err().texto().contains('2'));
        assert!(dois.validar(&face, &[], 1).is_ok());
        // validador do campo: só itens da mesma origem
        fn mesma_origem(c: &Candidato, no_campo: &[Candidato]) -> Result<(), Motivo> {
            match no_campo.first() {
                Some(p) if p.origem != c.origem => Err(Motivo::novo("Este item é de outra origem ({}).", vec![c.nome.clone()])),
                _ => Ok(()),
            }
        }
        let v = Entrada::nova("c", "C", TipoEntrada::Curva, varios(1), 1).valida(mesma_origem);
        let a = Candidato { tipo: TipoEntrada::Curva, origem: Some(Id(1)), nome: "A".into() };
        let b = Candidato { tipo: TipoEntrada::Curva, origem: Some(Id(2)), nome: "B".into() };
        assert!(v.validar(&b, std::slice::from_ref(&a), 1).is_err());
        assert!(v.validar(&a, std::slice::from_ref(&a), 1).is_ok());
    }

    #[test]
    fn salt_com_o_id_do_comando_bate_com_o_campo() {
        assert!(salt_bate("arestas", "arestas"));
        assert!(salt_bate("arestas12", "arestas"));
        assert!(!salt_bate("arestas_x", "arestas"));
        assert!(!salt_bate("secoes", "arestas"));
    }

    #[test]
    fn o_vazio_e_um_so() {
        assert!(ponto_vazio(&PONTO_VAZIO));
        assert!(!ponto_vazio(&[0., 0., 0.1]));
        let k = FeatureKind::Thread(base_thread());
        assert_eq!(campo_ativo(Alvo::Feature(&k)), Some(Espera::Face));
    }

    fn base_thread() -> Thread {
        Thread { face: PONTO_VAZIO, spec: "M10x1.5".into(), length: "20".into(), modeled: false, end: None, start: "0".into(), face_key: None }
    }
}
