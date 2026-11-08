// SPDX-License-Identifier: LGPL-2.1-or-later
//! Catálogo de normas (B241): o que o navegador da biblioteca mostra, sem depender da interface.
//!
//! Três serviços sobre as famílias de [`crate::biblioteca`]:
//! - **busca e filtros**: texto livre ("M8 sextavado", "8.8", "iso 4762"), sistema (métrico ou
//!   polegada), tipo de cabeça e material; o resultado já diz o tamanho achado, para a pessoa chegar
//!   ao item em poucos cliques;
//! - **árvore** Norma > Tipo > Família;
//! - **ficha** da família: cada cota com símbolo, nome e unidade, a tabela com uma linha por tamanho
//!   e as fontes. Toda família da biblioteca precisa abrir a ficha (teste no fim do arquivo).
//!
//! Também guarda os favoritos e o histórico de quem usa, como dado puro (a interface decide onde
//! gravar o arquivo de cada usuário).

use crate::biblioteca::{material_da_classe, Familia, Fonte, TipoFixador};
use serde::{Deserialize, Serialize};

/// Sistema de medidas de um tamanho ou de uma família.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sistema {
    Metrico,
    Polegada,
}

impl Sistema {
    pub fn rotulo(self) -> &'static str {
        match self {
            Sistema::Metrico => "Métrico",
            Sistema::Polegada => "Polegada",
        }
    }
}

/// Tipo de cabeça de uma família.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cabeca {
    /// cilíndrica com sextavado interno
    Allen,
    Sextavada,
    /// porca e arruela não têm cabeça
    Nenhuma,
}

impl Cabeca {
    pub fn rotulo(self) -> &'static str {
        match self {
            Cabeca::Allen => "Allen",
            Cabeca::Sextavada => "Sextavada",
            Cabeca::Nenhuma => "Sem cabeça",
        }
    }

    pub const TODAS: [Cabeca; 3] = [Cabeca::Allen, Cabeca::Sextavada, Cabeca::Nenhuma];
}

pub fn cabeca_de(f: &Familia) -> Cabeca {
    match f.tipo {
        TipoFixador::ParafusoAllen => Cabeca::Allen,
        TipoFixador::ParafusoSextavado => Cabeca::Sextavada,
        TipoFixador::Porca | TipoFixador::Arruela | TipoFixador::Rolamento => Cabeca::Nenhuma,
    }
}

/// Polegada quando a designação do tamanho não é métrica ("1/4-20 UNC", "#10-32", "-06").
pub fn sistema_de(f: &Familia) -> Sistema {
    // rolamento se designa pela série ("6005"), mas as medidas são em milímetros
    if f.tipo == TipoFixador::Rolamento {
        return Sistema::Metrico;
    }
    if f.tamanhos.iter().all(|t| t.designacao.starts_with('M') && t.diametro_nominal().is_some()) {
        Sistema::Metrico
    } else {
        Sistema::Polegada
    }
}

/// Entidade que publica a norma: a primeira palavra ("ISO", "DIN").
pub fn entidade_de(f: &Familia) -> String {
    f.norma.split_whitespace().next().unwrap_or("").to_string()
}

/// Material da peça gerada, pelo texto da classe ("Aço classe 8.8"); sem classe, "Aço".
pub fn material_de(f: &Familia) -> String {
    f.classe.as_deref().map(material_da_classe).unwrap_or_else(|| "Aço".into())
}

/// Família de material para o filtro: "Aço" ou "Aço inoxidável".
pub fn grupo_de_material(f: &Familia) -> &'static str {
    if material_de(f).contains("inoxidável") { "Aço inoxidável" } else { "Aço" }
}

/// Minúsculas, sem acento e sem espaço duplicado: a forma em que a busca compara.
pub fn normalizar(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' | 'Á' | 'À' | 'Â' | 'Ã' => 'a',
            'é' | 'è' | 'ê' | 'É' | 'Ê' => 'e',
            'í' | 'ì' | 'Í' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'Ó' | 'Ô' | 'Õ' => 'o',
            'ú' | 'ù' | 'ü' | 'Ú' => 'u',
            'ç' | 'Ç' => 'c',
            ',' => '.',
            c => c.to_ascii_lowercase(),
        })
        .collect()
}

/// Filtros do navegador. Campo vazio ou `None` não filtra.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Filtro {
    pub busca: String,
    pub sistema: Option<Sistema>,
    pub cabeca: Option<Cabeca>,
    /// "Aço" ou "Aço inoxidável"
    pub material: Option<String>,
    pub tipo: Option<TipoFixador>,
}

impl Filtro {
    pub fn vazio(&self) -> bool {
        *self == Filtro::default()
    }
}

/// Uma família que passou no filtro e, quando a busca citou um tamanho, qual.
#[derive(Clone, Debug, PartialEq)]
pub struct Achado<'a> {
    pub familia: &'a Familia,
    pub tamanho: Option<String>,
}

/// O token é a designação de um tamanho? "m8" acha "M8", mas "m8" não acha "M80"; "8" sozinho também
/// acha "M8" (e só ele).
fn tamanho_do_token(f: &Familia, tok: &str) -> Option<String> {
    f.tamanhos
        .iter()
        .find(|t| {
            let k = t.chave();
            k == tok || k.strip_prefix('m') == Some(tok) || t.original.as_deref().is_some_and(|o| crate::biblioteca::Tamanho::chave_de(o) == tok)
        })
        .map(|t| t.designacao.clone())
}

/// Texto em que as palavras da busca procuram: norma, id, tipo, cabeça, classe, material e sistema. O
/// nome longo da família fica de fora de propósito: "sextavado interno" do Allen faria o parafuso Allen
/// aparecer na busca por parafuso sextavado.
fn palheiro(f: &Familia) -> String {
    normalizar(&format!("{} {} {} {} {} {} {}", f.norma, f.id, f.tipo.rotulo(), cabeca_de(f).rotulo(), f.classe.clone().unwrap_or_default(), material_de(f), sistema_de(f).rotulo()))
}

/// Famílias que passam no filtro, na ordem da biblioteca. Cada palavra da busca precisa casar: com o
/// texto da família ou com a designação de um de seus tamanhos.
pub fn buscar<'a>(familias: &'a [Familia], filtro: &Filtro) -> Vec<Achado<'a>> {
    let palavras: Vec<String> = normalizar(&filtro.busca).split_whitespace().map(str::to_string).collect();
    let mut v = vec![];
    for f in familias {
        if filtro.sistema.is_some_and(|s| sistema_de(f) != s) || filtro.cabeca.is_some_and(|c| cabeca_de(f) != c) || filtro.tipo.is_some_and(|t| f.tipo != t) {
            continue;
        }
        if filtro.material.as_deref().is_some_and(|m| grupo_de_material(f) != m) {
            continue;
        }
        let texto = palheiro(f);
        let mut tamanho: Option<String> = None;
        let mut ok = true;
        for p in &palavras {
            // a designação de um tamanho vale primeiro ("m8", "8"); o resto casa com o texto da família
            if let Some(t) = tamanho_do_token(f, p) {
                tamanho = Some(t);
            } else if !texto.contains(p.as_str()) {
                ok = false;
                break;
            }
        }
        if ok {
            v.push(Achado { familia: f, tamanho });
        }
    }
    v
}

/// Nó da árvore Norma > Tipo > Família.
#[derive(Clone, Debug, PartialEq)]
pub struct NoTipo<'a> {
    pub tipo: TipoFixador,
    pub familias: Vec<&'a Familia>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NoNorma<'a> {
    pub entidade: String,
    pub tipos: Vec<NoTipo<'a>>,
}

/// Agrupa as famílias por entidade (ISO, DIN...) e, dentro dela, por tipo. Ordem alfabética nas
/// entidades; os tipos seguem a ordem do enum e as famílias, a da biblioteca.
pub fn arvore<'a>(familias: &[&'a Familia]) -> Vec<NoNorma<'a>> {
    let mut nos: Vec<NoNorma<'a>> = vec![];
    for f in familias {
        let e = entidade_de(f);
        let no = match nos.iter().position(|n| n.entidade == e) {
            Some(i) => i,
            None => {
                nos.push(NoNorma { entidade: e, tipos: vec![] });
                nos.len() - 1
            }
        };
        match nos[no].tipos.iter_mut().find(|t| t.tipo == f.tipo) {
            Some(t) => t.familias.push(f),
            None => nos[no].tipos.push(NoTipo { tipo: f.tipo, familias: vec![f] }),
        }
    }
    nos.sort_by(|a, b| a.entidade.cmp(&b.entidade));
    for n in &mut nos {
        n.tipos.sort_by_key(|t| ordem_do_tipo(t.tipo));
    }
    nos
}

fn ordem_do_tipo(t: TipoFixador) -> u8 {
    match t {
        TipoFixador::ParafusoAllen => 0,
        TipoFixador::ParafusoSextavado => 1,
        TipoFixador::Porca => 2,
        TipoFixador::Arruela => 3,
        TipoFixador::Rolamento => 4,
    }
}

/// Uma cota da norma: símbolo (como a norma escreve), o que mede e a unidade.
#[derive(Clone, Debug, PartialEq)]
pub struct Cota {
    pub simbolo: String,
    pub nome: String,
    pub unidade: String,
}

/// O que a cota mede, por símbolo e tipo de família. Frases próprias; o símbolo é o da norma.
pub fn nome_da_cota(tipo: TipoFixador, simbolo: &str) -> &'static str {
    match (tipo, simbolo) {
        (_, "P") => "passo da rosca",
        (_, "dk") => "diâmetro da cabeça",
        (TipoFixador::ParafusoAllen, "k") => "altura da cabeça",
        (TipoFixador::ParafusoSextavado, "k") => "altura da cabeça",
        (TipoFixador::ParafusoAllen, "s") => "abertura do sextavado interno",
        (TipoFixador::ParafusoSextavado | TipoFixador::Porca, "s") => "distância entre as faces da chave",
        (_, "b") => "comprimento roscado de referência",
        (TipoFixador::Porca, "m") => "altura da porca",
        (TipoFixador::Arruela, "d1") => "diâmetro interno",
        (TipoFixador::Arruela, "d2") => "diâmetro externo",
        (TipoFixador::Arruela, "h") => "espessura",
        (TipoFixador::Rolamento, "D") => "diâmetro externo",
        (TipoFixador::Rolamento, "B") => "largura",
        _ => "medida da norma",
    }
}

/// Uma linha da tabela: um tamanho, com os valores na ordem de [`Ficha::cotas`].
#[derive(Clone, Debug, PartialEq)]
pub struct LinhaTamanho {
    pub designacao: String,
    pub d_nominal: f64,
    pub valores: Vec<f64>,
    pub comprimentos: Vec<f64>,
    pub confirmado_por: u32,
    pub nota: Option<String>,
    pub massa_g: Option<f64>,
    pub pn: Option<String>,
}

/// Ficha completa de uma família.
#[derive(Clone, Debug, PartialEq)]
pub struct Ficha {
    pub id: String,
    pub titulo: String,
    pub norma: String,
    pub revisao: String,
    pub tipo: TipoFixador,
    pub cabeca: Cabeca,
    pub sistema: Sistema,
    pub material: String,
    pub acabamento: Option<String>,
    pub nivel: String,
    pub licenca: String,
    pub fontes: Vec<Fonte>,
    pub cotas: Vec<Cota>,
    pub linhas: Vec<LinhaTamanho>,
}

/// Monta a ficha. Falha com a frase do problema se a família não tiver todas as cotas em todos os
/// tamanhos (o validador da biblioteca já impede, mas a ficha não confia em quem chama).
pub fn ficha(f: &Familia) -> Result<Ficha, String> {
    if f.fontes.is_empty() {
        return Err(format!("{}: sem fonte para citar na ficha", f.id));
    }
    let cotas: Vec<Cota> = f.campos.iter().map(|c| Cota { simbolo: c.clone(), nome: nome_da_cota(f.tipo, c).to_string(), unidade: f.unidade(c).to_string() }).collect();
    let mut linhas = vec![];
    for t in &f.tamanhos {
        let valores = f.campos.iter().map(|c| t.valor(c).ok_or_else(|| format!("{} {}: falta a cota {c}", f.id, t.designacao))).collect::<Result<Vec<_>, _>>()?;
        linhas.push(LinhaTamanho {
            designacao: t.designacao.clone(),
            d_nominal: t.diametro_nominal().ok_or_else(|| format!("{} {}: sem diâmetro nominal", f.id, t.designacao))?,
            valores,
            comprimentos: if f.tipo.e_parafuso() { f.comprimentos_de(t) } else { vec![] },
            confirmado_por: t.confirmado_por,
            nota: t.nota.clone(),
            massa_g: t.massa_g,
            pn: t.pn.clone(),
        });
    }
    Ok(Ficha {
        id: f.id.clone(),
        titulo: f.nome.clone(),
        norma: f.norma.clone(),
        revisao: f.revisao.clone(),
        tipo: f.tipo,
        cabeca: cabeca_de(f),
        sistema: sistema_de(f),
        material: material_de(f),
        acabamento: f.acabamento.clone(),
        nivel: f.nivel.clone(),
        licenca: f.licenca.clone(),
        fontes: f.fontes.clone(),
        cotas,
        linhas,
    })
}

/// Um item da lista de recentes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Recente {
    pub familia: String,
    pub tamanho: String,
}

/// Favoritos e histórico de quem usa o catálogo. É dado puro: `para_json`/`de_json` servem a quem
/// grava o arquivo do usuário.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Preferencias {
    /// ids de família
    #[serde(default)]
    pub favoritos: Vec<String>,
    /// mais recente primeiro
    #[serde(default)]
    pub recentes: Vec<Recente>,
}

/// Quantos itens o histórico guarda.
pub const LIMITE_RECENTES: usize = 12;

impl Preferencias {
    pub fn e_favorita(&self, id: &str) -> bool {
        self.favoritos.iter().any(|f| f == id)
    }

    /// Liga ou desliga o favorito. Devolve o estado novo.
    pub fn alternar_favorito(&mut self, id: &str) -> bool {
        match self.favoritos.iter().position(|f| f == id) {
            Some(i) => {
                self.favoritos.remove(i);
                false
            }
            None => {
                self.favoritos.push(id.to_string());
                true
            }
        }
    }

    /// Registra o uso de um item: vai para o topo, sem repetir, até o limite.
    pub fn registrar(&mut self, familia: &str, tamanho: &str) {
        self.recentes.retain(|r| !(r.familia == familia && r.tamanho == tamanho));
        self.recentes.insert(0, Recente { familia: familia.to_string(), tamanho: tamanho.to_string() });
        self.recentes.truncate(LIMITE_RECENTES);
    }

    pub fn para_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// Lê o arquivo; texto inválido volta às preferências vazias (arquivo do usuário nunca derruba o programa).
    pub fn de_json(texto: &str) -> Self {
        serde_json::from_str(texto).unwrap_or_default()
    }

    /// Tira o que a biblioteca atual não conhece mais (família removida ou tamanho que sumiu).
    pub fn limpar(&mut self, familias: &[Familia]) {
        self.favoritos.retain(|id| familias.iter().any(|f| &f.id == id));
        self.recentes.retain(|r| familias.iter().find(|f| f.id == r.familia).is_some_and(|f| f.tamanho(&r.tamanho).is_some()));
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::biblioteca::familias;

    fn achar(busca: &str) -> Vec<(String, Option<String>)> {
        buscar(familias(), &Filtro { busca: busca.into(), ..Default::default() }).into_iter().map(|a| (a.familia.id.clone(), a.tamanho)).collect()
    }

    /// B396: o rolamento entra no navegador como qualquer família, métrico, com ficha e o tamanho
    /// achado pela designação da série.
    #[test]
    fn rolamento_6010_aparece_com_ficha_e_e_metrico() {
        assert!(achar("rolamento 6010").contains(&("iso15_60".to_string(), Some("6010".to_string()))), "{:?}", achar("rolamento 6010"));
        let f = crate::biblioteca::familia("iso15_60").unwrap();
        assert_eq!(sistema_de(f), Sistema::Metrico);
        let fi = ficha(f).unwrap();
        assert_eq!(fi.cotas.iter().map(|c| c.nome.as_str()).collect::<Vec<_>>(), ["diâmetro externo", "largura"]);
        let l = fi.linhas.iter().find(|l| l.designacao == "6005").unwrap();
        assert_eq!((l.d_nominal, l.valores.clone()), (25., vec![47., 12.]));
        // o furo do alojamento com o externo do rolamento leva ao tamanho certo
        assert_eq!(f.tamanhos_para_furo(80., 0.05).iter().map(|t| t.designacao.as_str()).collect::<Vec<_>>(), ["6010"]);
    }

    #[test]
    fn m8_sextavado_aparece_com_o_tamanho_escolhido() {
        let r = achar("M8 sextavado");
        let ids: Vec<&str> = r.iter().map(|(i, _)| i.as_str()).collect();
        assert!(ids.contains(&"iso4017") && ids.contains(&"din933"), "{ids:?}");
        assert!(!ids.contains(&"iso4762"), "Allen não é sextavado");
        assert!(r.iter().all(|(_, t)| t.as_deref() == Some("M8")), "{r:?}");
    }

    #[test]
    fn m8_nao_acha_m80_e_numero_solto_acha_o_tamanho() {
        // nenhuma família tem M80, mas "m8" tem de casar só com M8
        let r = achar("m8");
        assert!(r.iter().all(|(_, t)| t.as_deref() == Some("M8")));
        assert_eq!(achar("8").iter().filter(|(_, t)| t.as_deref() == Some("M8")).count(), achar("m8").len());
    }

    #[test]
    fn busca_ignora_acento_caixa_e_acha_pela_norma_e_pela_classe() {
        assert_eq!(achar("PORCA").len(), 4);
        assert_eq!(achar("iso 4762").len(), 1);
        assert!(achar("8.8").iter().any(|(i, _)| i == "iso4762"));
        assert!(achar("inoxidavel").is_empty(), "nenhuma família é inox hoje");
        assert!(achar("zzz").is_empty());
    }

    #[test]
    fn chips_filtram_por_sistema_cabeca_e_material() {
        let f = |c: Filtro| buscar(familias(), &c).len();
        assert_eq!(f(Filtro { sistema: Some(Sistema::Metrico), ..Default::default() }), familias().len());
        assert_eq!(f(Filtro { sistema: Some(Sistema::Polegada), ..Default::default() }), 0);
        assert_eq!(f(Filtro { cabeca: Some(Cabeca::Allen), ..Default::default() }), 2);
        assert_eq!(f(Filtro { cabeca: Some(Cabeca::Sextavada), ..Default::default() }), 3);
        assert_eq!(f(Filtro { cabeca: Some(Cabeca::Nenhuma), ..Default::default() }), 6);
        assert_eq!(f(Filtro { material: Some("Aço".into()), ..Default::default() }), familias().len());
        assert_eq!(f(Filtro { material: Some("Aço inoxidável".into()), ..Default::default() }), 0);
        assert_eq!(f(Filtro { tipo: Some(TipoFixador::Arruela), ..Default::default() }), 1);
        assert_eq!(f(Filtro { tipo: Some(TipoFixador::Rolamento), ..Default::default() }), 1);
    }

    #[test]
    fn arvore_agrupa_por_entidade_e_tipo_sem_perder_familia() {
        let todas: Vec<&Familia> = familias().iter().collect();
        let a = arvore(&todas);
        let total: usize = a.iter().flat_map(|n| &n.tipos).map(|t| t.familias.len()).sum();
        assert_eq!(total, familias().len());
        assert_eq!(a.iter().map(|n| n.entidade.as_str()).collect::<Vec<_>>(), ["DIN", "ISO"]);
        let iso = a.iter().find(|n| n.entidade == "ISO").unwrap();
        assert_eq!(iso.tipos[0].tipo, TipoFixador::ParafusoAllen);
    }

    #[test]
    fn toda_familia_abre_a_ficha_completa() {
        for f in familias() {
            let fi = ficha(f).unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(fi.cotas.len(), f.campos.len(), "{}", f.id);
            assert_eq!(fi.linhas.len(), f.tamanhos.len(), "{}", f.id);
            assert!(!fi.fontes.is_empty() && !fi.norma.is_empty() && !fi.revisao.is_empty(), "{}", f.id);
            for c in &fi.cotas {
                assert_ne!(c.nome, "medida da norma", "{}: cota {} sem nome", f.id, c.simbolo);
            }
            for l in &fi.linhas {
                assert_eq!(l.valores.len(), fi.cotas.len(), "{} {}", f.id, l.designacao);
                assert_eq!(!l.comprimentos.is_empty(), f.tipo.e_parafuso(), "{} {}", f.id, l.designacao);
            }
        }
    }

    #[test]
    fn ficha_do_m8_sextavado_traz_todas_as_cotas_e_a_fonte() {
        let f = crate::biblioteca::familia("iso4017").unwrap();
        let fi = ficha(f).unwrap();
        assert_eq!(fi.cotas.iter().map(|c| c.simbolo.as_str()).collect::<Vec<_>>(), ["P", "s", "k"]);
        let m8 = fi.linhas.iter().find(|l| l.designacao == "M8").unwrap();
        assert_eq!(m8.valores, vec![1.25, 13., 5.3]);
        assert!(fi.fontes.len() >= 2);
    }

    #[test]
    fn favoritos_e_historico_sem_repeticao_e_com_limite() {
        let mut p = Preferencias::default();
        assert!(p.alternar_favorito("iso4762"));
        assert!(p.e_favorita("iso4762"));
        assert!(!p.alternar_favorito("iso4762"));
        for i in 0..20 {
            p.registrar("iso4017", &format!("M{}", 3 + i));
        }
        p.registrar("iso4017", "M5");
        assert_eq!(p.recentes.len(), LIMITE_RECENTES);
        assert_eq!(p.recentes[0].tamanho, "M5");
        assert_eq!(p.recentes.iter().filter(|r| r.tamanho == "M5").count(), 1);
        let lido = Preferencias::de_json(&p.para_json());
        assert_eq!(lido, p);
        assert_eq!(Preferencias::de_json("lixo"), Preferencias::default());
    }

    #[test]
    fn limpar_tira_o_que_a_biblioteca_nao_conhece() {
        let mut p = Preferencias::default();
        p.alternar_favorito("iso4762");
        p.alternar_favorito("sumiu");
        p.registrar("iso4762", "M6");
        p.registrar("iso4762", "M99");
        p.registrar("sumiu", "M6");
        p.limpar(familias());
        assert_eq!(p.favoritos, ["iso4762"]);
        assert_eq!(p.recentes.len(), 1);
    }
}
