// SPDX-License-Identifier: LGPL-2.1-or-later
//! B148: recursos de plástico (boss, snap-fit, grelha e lip).
//!
//! Cada recurso é um comando só (`FeatureKind::Plastico`) com o tipo dentro. Os campos que a regra
//! plástica do documento governa (B198, ADR 0093) nascem com a fórmula dela (`plastico_saida`,
//! `plastico_raio`, `plastico_nervura`, `plastico_espessura`): mudar a regra muda todos os recursos.
//! Um número próprio no lugar da fórmula é a sobrescrita, que a tela só deixa fazer em modo avançado.
//!
//! O posicionamento vem de um sketch (pontos, linhas ou contorno), referência estável por id: se o
//! sketch some, o recálculo devolve o erro nomeado de referência perdida.

use crate::doc::Id;
use crate::regra_plastica::{Grandeza, RegraPlastica};
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum RecursoPlastico {
    Boss(Boss),
    Snap(Snap),
    Grelha(Grelha),
    Lip(Lip),
}

/// Boss: pino oco para parafuso ou inserto, em cada ponto do sketch, crescendo pela normal do plano.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Boss {
    /// os pontos deste sketch são os centros
    pub sketch: Id,
    /// diâmetro externo no topo, mm
    pub diametro: String,
    /// altura a partir do plano do sketch, mm
    pub altura: String,
    /// ângulo de saída da parede externa, graus (alarga para a base)
    pub saida: String,
    /// raio do filete na base, mm (0 = sem filete)
    pub raio: String,
    /// diâmetro do furo (0 = boss maciço)
    pub furo: String,
    /// profundidade do furo a partir do topo
    pub furo_prof: String,
    /// rebaixo para inserto no topo: diâmetro e profundidade (vazio = sem inserto)
    #[serde(default)]
    pub inserto: String,
    #[serde(default)]
    pub inserto_prof: String,
    /// nervuras de apoio: vazio = automático (4 quando a altura passa de 3 vezes o diâmetro)
    #[serde(default)]
    pub nervuras: String,
    /// espessura das nervuras
    pub nervura_esp: String,
    /// alcance radial da nervura além da base do boss (vazio = metade do diâmetro)
    #[serde(default)]
    pub nervura_larg: String,
    /// altura da nervura (vazio = 70% da altura do boss)
    #[serde(default)]
    pub nervura_alt: String,
    /// cresce para o outro lado do plano
    #[serde(default)]
    pub reverse: bool,
}

impl Boss {
    pub fn novo(sketch: Id, regra: Option<&RegraPlastica>) -> Boss {
        let f = |g: Grandeza, padrao: &str| if regra.is_some() { RegraPlastica::formula(g).to_string() } else { padrao.to_string() };
        Boss {
            sketch,
            diametro: "8".into(),
            altura: "10".into(),
            saida: f(Grandeza::Saida, "1"),
            raio: f(Grandeza::Raio, "0.5"),
            furo: "3".into(),
            furo_prof: "8".into(),
            inserto: String::new(),
            inserto_prof: String::new(),
            nervuras: String::new(),
            nervura_esp: f(Grandeza::Nervura, "1.2"),
            nervura_larg: String::new(),
            nervura_alt: String::new(),
            reverse: false,
        }
    }
}

/// Snap-fit em balanço: cada linha do sketch é a raiz de uma viga com gancho na ponta.
///
/// A viga cresce pela normal do plano a partir da linha (a largura é o comprimento da linha), com a
/// face de trás sobre a linha e o gancho para o lado escolhido. A deformação na raiz ao passar o
/// gancho, `1,5 t y / l²` (viga de seção constante, `l` da raiz até a face de retenção), é conferida
/// contra a admissível do material: acima dela o comando avisa na árvore.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Snap {
    /// as linhas deste sketch são as raízes
    pub sketch: Id,
    /// do plano até a ponta do gancho, mm
    pub comprimento: String,
    /// espessura da viga, mm
    pub espessura: String,
    /// quanto o gancho sobressai da viga (interferência), mm
    pub gancho: String,
    /// comprimento do gancho ao longo da viga (a rampa de entrada), mm
    pub gancho_alt: String,
    /// filete da raiz, mm (0 = canto vivo)
    pub raio: String,
    /// deformação admissível do material, %
    pub deformacao_adm: String,
    /// gancho para o outro lado da linha
    #[serde(default)]
    pub lado: bool,
    /// cresce para o outro lado do plano
    #[serde(default)]
    pub reverse: bool,
}

impl Snap {
    pub fn novo(sketch: Id, regra: Option<&RegraPlastica>) -> Snap {
        let f = |g: Grandeza, padrao: &str| if regra.is_some() { RegraPlastica::formula(g).to_string() } else { padrao.to_string() };
        Snap {
            sketch,
            comprimento: "18".into(),
            espessura: f(Grandeza::Parede, "2"),
            gancho: "1".into(),
            gancho_alt: "3".into(),
            raio: f(Grandeza::Raio, "0.5"),
            deformacao_adm: "2".into(),
            lado: false,
            reverse: false,
        }
    }
}

/// Grelha de ventilação: a região fechada do sketch, menos a borda, é vazada através da parede, e
/// ficam barras (nervuras) paralelas com o passo pedido, opcionalmente cruzadas.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Grelha {
    /// o perfil fechado deste sketch é a região da grelha
    pub sketch: Id,
    /// faixa cheia que fica junto ao contorno, mm
    pub borda: String,
    /// distância entre os eixos das barras, mm
    pub passo: String,
    /// largura de cada barra
    pub nervura: String,
    /// direção das barras no plano, graus a partir do X do sketch
    #[serde(default)]
    pub angulo: String,
    /// barras também na direção perpendicular
    #[serde(default)]
    pub cruzada: bool,
    /// profundidade do vazado; vazio = atravessa a parede (até sair do material)
    #[serde(default)]
    pub profundidade: String,
    /// vaza para o lado da normal do plano (o padrão entra contra a normal)
    #[serde(default)]
    pub reverse: bool,
}

impl Grelha {
    pub fn novo(sketch: Id, regra: Option<&RegraPlastica>) -> Grelha {
        Grelha {
            sketch,
            borda: "2".into(),
            passo: "5".into(),
            nervura: if regra.is_some() { RegraPlastica::formula(Grandeza::Nervura).to_string() } else { "1.2".into() },
            angulo: "0".into(),
            cruzada: false,
            profundidade: String::new(),
            reverse: false,
        }
    }
}

/// Lip e ranhura com folga, na borda de uma caixa partida em duas.
///
/// O sketch traz o contorno externo da parede no plano da borda. O lip é um anel que sobe da borda,
/// entre `recuo` e `recuo + largura` para dentro do contorno. A ranhura é o encaixe do outro lado: o
/// mesmo anel alargado pela folga dos dois lados e afundado pela altura mais a folga, para o lip da
/// outra metade entrar sem encostar.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Lip {
    /// o perfil fechado deste sketch é o contorno externo da parede
    pub sketch: Id,
    /// ranhura (tira material) em vez de lip (acrescenta)
    #[serde(default)]
    pub ranhura: bool,
    /// distância do contorno externo até a face de fora do lip, mm
    pub recuo: String,
    /// largura do lip, mm
    pub largura: String,
    /// altura do lip (a ranhura afunda isso mais a folga), mm
    pub altura: String,
    /// folga de cada lado e no fundo, mm
    pub folga: String,
    /// cresce (ou afunda) para o outro lado do plano
    #[serde(default)]
    pub reverse: bool,
}

impl Lip {
    pub fn novo(sketch: Id, regra: Option<&RegraPlastica>, ranhura: bool) -> Lip {
        let meia = if regra.is_some() { "plastico_espessura/2" } else { "1" };
        Lip { sketch, ranhura, recuo: meia.into(), largura: meia.into(), altura: "2".into(), folga: "0.1".into(), reverse: false }
    }
}

/// Altura acima da qual o boss pede nervuras, em diâmetros.
pub const BOSS_NERVURA_DIAMETROS: f64 = 3.;

impl RecursoPlastico {
    pub fn nome(&self) -> &'static str {
        match self {
            RecursoPlastico::Boss(_) => "Boss",
            RecursoPlastico::Snap(_) => "Snap-fit",
            RecursoPlastico::Grelha(_) => "Grelha",
            RecursoPlastico::Lip(l) if l.ranhura => "Ranhura de encaixe",
            RecursoPlastico::Lip(_) => "Lip",
        }
    }

    pub fn simbolo(&self) -> &'static str {
        match self {
            RecursoPlastico::Boss(_) => "⊚",
            RecursoPlastico::Snap(_) => "↾",
            RecursoPlastico::Grelha(_) => "▥",
            RecursoPlastico::Lip(_) => "⊓",
        }
    }

    pub fn sketch(&self) -> Id {
        match self {
            RecursoPlastico::Boss(b) => b.sketch,
            RecursoPlastico::Snap(s) => s.sketch,
            RecursoPlastico::Grelha(g) => g.sketch,
            RecursoPlastico::Lip(l) => l.sketch,
        }
    }

    /// Ids e expressões, para renomear parâmetros e remapear cópias.
    pub fn walk(&mut self, ids: &mut dyn FnMut(&mut Id), exprs: &mut dyn FnMut(&mut String)) {
        match self {
            RecursoPlastico::Boss(b) => {
                ids(&mut b.sketch);
                for e in [&mut b.diametro, &mut b.altura, &mut b.saida, &mut b.raio, &mut b.furo, &mut b.furo_prof, &mut b.inserto, &mut b.inserto_prof, &mut b.nervuras, &mut b.nervura_esp, &mut b.nervura_larg, &mut b.nervura_alt] {
                    if !e.trim().is_empty() {
                        exprs(e)
                    }
                }
            }
            RecursoPlastico::Snap(s) => {
                ids(&mut s.sketch);
                for e in [&mut s.comprimento, &mut s.espessura, &mut s.gancho, &mut s.gancho_alt, &mut s.raio, &mut s.deformacao_adm] {
                    exprs(e)
                }
            }
            RecursoPlastico::Grelha(g) => {
                ids(&mut g.sketch);
                for e in [&mut g.borda, &mut g.passo, &mut g.nervura, &mut g.angulo, &mut g.profundidade] {
                    if !e.trim().is_empty() {
                        exprs(e)
                    }
                }
            }
            RecursoPlastico::Lip(l) => {
                ids(&mut l.sketch);
                for e in [&mut l.recuo, &mut l.largura, &mut l.altura, &mut l.folga] {
                    exprs(e)
                }
            }
        }
    }

    /// Campos que a regra plástica governa: (grandeza, fórmula, nome do campo).
    pub fn campos_regra(&self) -> Vec<(Grandeza, &str, &'static str)> {
        match self {
            RecursoPlastico::Boss(b) => vec![(Grandeza::Saida, b.saida.as_str(), "saída do boss"), (Grandeza::Raio, b.raio.as_str(), "filete da base"), (Grandeza::Nervura, b.nervura_esp.as_str(), "nervura do boss")],
            RecursoPlastico::Snap(s) => vec![(Grandeza::Parede, s.espessura.as_str(), "espessura do snap"), (Grandeza::Raio, s.raio.as_str(), "filete da raiz")],
            RecursoPlastico::Grelha(g) => vec![(Grandeza::Nervura, g.nervura.as_str(), "barra da grelha")],
            // o lip segue a parede pela fórmula (metade dela), mas não é uma parede: nada a conferir
            RecursoPlastico::Lip(_) => vec![],
        }
    }

    /// Campos governados, mutáveis (para o comando nascer seguindo a regra).
    pub fn campos_regra_mut(&mut self) -> Vec<(Grandeza, &mut String)> {
        match self {
            RecursoPlastico::Boss(b) => vec![(Grandeza::Saida, &mut b.saida), (Grandeza::Raio, &mut b.raio), (Grandeza::Nervura, &mut b.nervura_esp)],
            RecursoPlastico::Snap(s) => vec![(Grandeza::Parede, &mut s.espessura), (Grandeza::Raio, &mut s.raio)],
            RecursoPlastico::Grelha(g) => vec![(Grandeza::Nervura, &mut g.nervura)],
            RecursoPlastico::Lip(_) => vec![],
        }
    }
}
