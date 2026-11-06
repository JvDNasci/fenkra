// SPDX-License-Identifier: LGPL-2.1-or-later
//! B112: motor de regras de fabricação (DFM) com perfis por processo.
//!
//! Um [`Perfil`] é um arquivo JSON com o nome do processo, a direção de abertura ou de construção
//! e uma lista de [`Regra`]s. Cada regra tem identificador, a entidade que mede (furo, bolso,
//! chapa, parede, saliência, canto, saída, undercut), a medida, os limites (`min` e `max`), a
//! severidade e o texto da mensagem. Os perfis que vêm com o programa ficam em `perfis_dfm/` e são
//! só o ponto de partida: a pessoa guarda um JSON com o mesmo nome na pasta de perfis dela e os
//! limites mudam sem recompilar ([`carregar_pasta`]).
//!
//! A varredura ([`varrer`]) mede a peça uma vez (só o que alguma regra do perfil pede) e devolve as
//! [`Violacao`]s: cada uma com a regra, o valor medido, o limite, o ponto, o índice da face (mesma
//! ordem da malha) e, quando o corpo informa dono e papel das faces, a [`FaceKey`] estável que
//! sobrevive ao recálculo ([`anexar_chaves`]). A tela pinta as faces citadas.
//!
//! Reaproveita o reconhecimento de features (`reconhece`) para furos, filetes e bolsos, e a geometria
//! de `dfm_geo` para espessura, saliência, canto vivo e saída. É a base de B113 (espessura,
//! saliência, canto vivo), B197 (saída e undercut), B166 e B167.

use crate::dfm_geo::{self as geo, Malha};
use crate::reconhece::{self, Tipo};
use fenkra_core::FaceKey;
use fenkra_kernel::Shape;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type V3 = [f64; 3];

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severidade {
    Info,
    Aviso,
    Erro,
}

impl Severidade {
    pub fn rotulo(self) -> &'static str {
        match self {
            Severidade::Info => "Informação",
            Severidade::Aviso => "Aviso",
            Severidade::Erro => "Erro",
        }
    }
}

/// O que a regra mede. Cada par (entidade, medida) tem o significado fixo abaixo; a regra só diz o
/// limite.
///
/// | entidade   | medida                | significado                                           |
/// |------------|-----------------------|-------------------------------------------------------|
/// | furo       | diametro              | diâmetro, mm                                          |
/// | furo       | profundidade_diametro | profundidade dividida pelo diâmetro                   |
/// | bolso      | profundidade_raio     | altura do canto arredondado dividida pelo raio dele   |
/// | chapa      | espessura             | espessura dominante da peça, mm                       |
/// | chapa      | raio_dobra_espessura  | raio do filete convexo dividido pela espessura        |
/// | chapa      | furo_espessura        | diâmetro do furo dividido pela espessura              |
/// | parede     | espessura             | espessura local (bola máxima) de cada face, mm        |
/// | saliencia  | angulo                | graus a partir da vertical da face sem apoio          |
/// | canto      | raio_interno          | raio do canto côncavo (0 = aresta viva), mm           |
/// | saida      | angulo                | ângulo de saída da face em relação à abertura, graus  |
/// | undercut   | area                  | área da face bloqueada ao longo da abertura, mm²      |
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Regra {
    pub id: String,
    pub entidade: String,
    pub medida: String,
    #[serde(default)]
    pub min: Option<f64>,
    #[serde(default)]
    pub max: Option<f64>,
    pub severidade: Severidade,
    /// mensagem com `{valor}` e `{limite}`
    pub texto: String,
    #[serde(default = "verdadeiro")]
    pub ativa: bool,
}

fn verdadeiro() -> bool {
    true
}

/// Medidas que o motor sabe fazer: o que o arquivo do perfil pode pedir.
pub const MEDIDAS: &[(&str, &str)] = &[
    ("furo", "diametro"),
    ("furo", "profundidade_diametro"),
    ("bolso", "profundidade_raio"),
    ("chapa", "espessura"),
    ("chapa", "raio_dobra_espessura"),
    ("chapa", "furo_espessura"),
    ("parede", "espessura"),
    ("saliencia", "angulo"),
    ("canto", "raio_interno"),
    ("saida", "angulo"),
    ("undercut", "area"),
];

impl Regra {
    pub fn conhecida(&self) -> bool {
        MEDIDAS.iter().any(|(e, m)| *e == self.entidade && *m == self.medida)
    }

    /// Valor fora do limite? Devolve o limite violado.
    fn viola(&self, v: f64) -> Option<f64> {
        if let Some(lo) = self.min {
            if v < lo - 1e-9 {
                return Some(lo);
            }
        }
        if let Some(hi) = self.max {
            if v > hi + 1e-9 {
                return Some(hi);
            }
        }
        None
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Perfil {
    pub nome: String,
    /// cnc, chapa, injecao, aditivo ou texto livre
    pub processo: String,
    /// abertura do molde, direção da ferramenta ou direção de construção (para cima)
    #[serde(default = "eixo_z")]
    pub direcao: V3,
    pub regras: Vec<Regra>,
}

fn eixo_z() -> V3 {
    [0., 0., 1.]
}

impl Perfil {
    pub fn de_json(t: &str) -> Result<Perfil, String> {
        let p: Perfil = serde_json::from_str(t).map_err(|e| format!("perfil DFM inválido: {e}"))?;
        if p.nome.trim().is_empty() {
            return Err("perfil DFM sem nome".into());
        }
        if let Some(r) = p.regras.iter().find(|r| !r.conhecida()) {
            return Err(format!("regra '{}': medida desconhecida {}.{}", r.id, r.entidade, r.medida));
        }
        if let Some(r) = p.regras.iter().find(|r| matches!((r.min, r.max), (Some(a), Some(b)) if a > b)) {
            return Err(format!("regra '{}': mínimo maior que o máximo", r.id));
        }
        Ok(p)
    }

    pub fn para_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    pub fn regra(&self, id: &str) -> Option<&Regra> {
        self.regras.iter().find(|r| r.id == id)
    }

    /// Muda o limite de uma regra sem recompilar nada (a tela de edição usa isto).
    pub fn ajusta(&mut self, id: &str, min: Option<f64>, max: Option<f64>) -> bool {
        match self.regras.iter_mut().find(|r| r.id == id) {
            Some(r) => {
                r.min = min;
                r.max = max;
                true
            }
            None => false,
        }
    }

    fn usa(&self, entidade: &str) -> bool {
        self.regras.iter().any(|r| r.ativa && r.entidade == entidade)
    }
}

const FRESAGEM: &str = include_str!("../perfis_dfm/fresagem_cnc.json");
const CHAPA: &str = include_str!("../perfis_dfm/chapa_metalica.json");
const INJECAO: &str = include_str!("../perfis_dfm/injecao_plastica.json");
const ADITIVO: &str = include_str!("../perfis_dfm/aditivo_fdm.json");

/// Perfis que vêm com o programa.
pub fn perfis_padrao() -> Vec<Perfil> {
    [FRESAGEM, CHAPA, INJECAO, ADITIVO].iter().filter_map(|t| Perfil::de_json(t).ok()).collect()
}

/// Perfis padrão trocados pelos da pasta do usuário: um JSON com o mesmo nome substitui o de fábrica,
/// um nome novo entra na lista. Arquivo que não lê vai para o segundo valor, com o motivo.
pub fn carregar_pasta(pasta: &std::path::Path) -> (Vec<Perfil>, Vec<String>) {
    let mut v = perfis_padrao();
    let mut erros = vec![];
    let Ok(rd) = std::fs::read_dir(pasta) else { return (v, erros) };
    let mut arqs: Vec<_> = rd.filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "json")).collect();
    arqs.sort();
    for a in arqs {
        match std::fs::read_to_string(&a).map_err(|e| e.to_string()).and_then(|t| Perfil::de_json(&t)) {
            Ok(p) => match v.iter_mut().find(|q| q.nome == p.nome) {
                Some(q) => *q = p,
                None => v.push(p),
            },
            Err(e) => erros.push(format!("{}: {e}", a.display())),
        }
    }
    (v, erros)
}

/// Uma regra descumprida.
#[derive(Clone, Debug, PartialEq)]
pub struct Violacao {
    pub regra: String,
    pub severidade: Severidade,
    pub texto: String,
    pub valor: f64,
    pub limite: f64,
    /// ponto para o realce e para olhar no 3D
    pub ponto: V3,
    /// faces envolvidas, na ordem da malha do corpo (a primeira é a principal)
    pub faces: Vec<usize>,
    /// chave estável da face principal, quando o corpo informou dono e papel (`anexar_chaves`)
    pub chave: Option<FaceKey>,
}

#[derive(Clone, Debug, Default)]
pub struct Relatorio {
    pub perfil: String,
    pub violacoes: Vec<Violacao>,
    /// o que não pôde ser medido (peça grande demais, malha aberta...)
    pub avisos: Vec<String>,
    /// espessura local de cada amostra, quando alguma regra a pediu (alimenta o mapa de B113)
    pub espessura: Vec<geo::Amostra>,
    /// classificação por face, quando alguma regra pediu saída ou undercut
    pub saida: Vec<geo::FaceSaida>,
}

impl Relatorio {
    pub fn contar(&self, s: Severidade) -> usize {
        self.violacoes.iter().filter(|v| v.severidade == s).count()
    }

    pub fn das_regras(&self, id: &str) -> impl Iterator<Item = &Violacao> {
        let id = id.to_string();
        self.violacoes.iter().filter(move |v| v.regra == id)
    }

    /// Faces para realçar, sem repetir, com a maior severidade de cada uma.
    pub fn faces_realcadas(&self) -> Vec<(usize, Severidade)> {
        let mut m: HashMap<usize, Severidade> = HashMap::new();
        for v in &self.violacoes {
            for f in &v.faces {
                let e = m.entry(*f).or_insert(v.severidade);
                *e = (*e).max(v.severidade);
            }
        }
        let mut r: Vec<_> = m.into_iter().collect();
        r.sort();
        r
    }

    pub fn resumo(&self) -> String {
        format!("{}: {} erros, {} avisos, {} informações", self.perfil, self.contar(Severidade::Erro), self.contar(Severidade::Aviso), self.contar(Severidade::Info))
    }
}

/// Liga a chave estável a cada violação, pela face principal.
pub fn anexar_chaves(r: &mut Relatorio, v: &crate::refs::View) {
    for x in &mut r.violacoes {
        if let Some(f) = x.faces.first() {
            x.chave = crate::refs::promote_face(v, *f);
        }
    }
}

fn texto(r: &Regra, valor: f64, limite: f64) -> String {
    r.texto.replace("{valor}", &format!("{valor:.2}")).replace("{limite}", &format!("{limite:.2}"))
}

/// Acima disso a malha não é medida por inteiro: a espessura usa passo.
pub const AMOSTRAS_ESPESSURA: usize = 4000;
/// Tolerância das faixas do mapa de espessura.
pub const TOL_FAIXA: f64 = 0.05;
/// Fração da área de uma face que precisa estar fora do limite para a face ser acusada.
pub const FRACAO_FACE: f64 = 0.05;

/// Mede a forma contra o perfil.
pub fn varrer(s: &Shape, perfil: &Perfil) -> Result<Relatorio, String> {
    let (lo, hi) = s.bbox().ok_or("a forma está vazia")?;
    let diag = (0..3).map(|k| (hi[k] - lo[k]).powi(2)).sum::<f64>().sqrt().max(1e-6);
    let mesh = s.mesh_angle(diag * 1e-3, 0.25).map_err(|e| e.0)?;
    let m = Malha::de(&mesh).ok_or("a malha não tem triângulos")?;
    let mut rel = Relatorio { perfil: perfil.nome.clone(), ..Default::default() };
    let ativas = |ent: &str, med: &str| perfil.regras.iter().filter(move |r| r.ativa && r.entidade == ent && r.medida == med).collect::<Vec<_>>();
    let push = |rel: &mut Relatorio, r: &Regra, valor: f64, limite: f64, ponto: V3, faces: Vec<usize>| {
        rel.violacoes.push(Violacao { regra: r.id.clone(), severidade: r.severidade, texto: texto(r, valor, limite), valor, limite, ponto, faces, chave: None });
    };

    // espessura: parede e chapa
    let precisa_esp = perfil.usa("parede") || perfil.usa("chapa");
    if precisa_esp {
        rel.espessura = geo::espessura(&m, AMOSTRAS_ESPESSURA);
        if rel.espessura.is_empty() {
            rel.avisos.push("espessura: nenhum ponto achou o lado de dentro (malha aberta?)".into());
        }
    }
    for r in ativas("parede", "espessura") {
        // por face: peso total, peso fora do limite, soma ponderada do valor e o pior ponto
        let mut por_face: HashMap<usize, (f64, f64, f64, f64, V3)> = HashMap::new();
        for a in &rel.espessura {
            let e = por_face.entry(a.face).or_insert((0., 0., 0., a.valor, a.ponto));
            e.0 += a.peso;
            if r.viola(a.valor).is_some() {
                e.1 += a.peso;
                e.2 += a.peso * a.valor;
                if (r.min.is_some() && a.valor < e.3) || (r.min.is_none() && a.valor > e.3) || r.viola(e.3).is_none() {
                    e.3 = a.valor;
                    e.4 = a.ponto;
                }
            }
        }
        let mut v: Vec<_> = por_face.into_iter().collect();
        v.sort_by_key(|(f, _)| *f);
        for (f, (total, fora, soma, _, ponto)) in v {
            // a bola perto da quina de uma face mede menos que a parede: só vale se uma parte
            // da face (5%) está fora do limite
            if fora <= 0. || fora < total * FRACAO_FACE {
                continue;
            }
            let valor = soma / fora;
            let lim = r.viola(valor).unwrap_or(r.min.or(r.max).unwrap_or(0.));
            push(&mut rel, r, valor, lim, ponto, vec![f]);
        }
    }

    // features reconhecidas: furo, bolso, chapa
    let precisa_feat = perfil.usa("furo") || perfil.usa("bolso") || perfil.usa("canto") || perfil.usa("chapa");
    let rec = if precisa_feat { Some(reconhece::analisar(s)) } else { None };
    if let Some(rc) = &rec {
        if let Some(p) = &rc.pulada {
            rel.avisos.push(format!("features: {p}"));
        }
    }
    let t_chapa = dominante(&rel.espessura);
    if let Some(rc) = &rec {
        for e in &rc.encontradas {
            let faces = e.faces.clone();
            match e.tipo {
                Tipo::FuroPassante | Tipo::FuroCego => {
                    for r in ativas("furo", "diametro") {
                        if let Some(l) = r.viola(e.medida) {
                            push(&mut rel, r, e.medida, l, e.centro, faces.clone());
                        }
                    }
                    if e.medida > 1e-9 {
                        for r in ativas("furo", "profundidade_diametro") {
                            let v = e.medida2 / e.medida;
                            if let Some(l) = r.viola(v) {
                                push(&mut rel, r, v, l, e.boca, faces.clone());
                            }
                        }
                    }
                    if let Some(t) = t_chapa {
                        for r in ativas("chapa", "furo_espessura") {
                            let v = e.medida / t;
                            if let Some(l) = r.viola(v) {
                                push(&mut rel, r, v, l, e.boca, faces.clone());
                            }
                        }
                    }
                }
                Tipo::Filete => {
                    if !e.convexo {
                        // canto interno com filete: o raio decide se a ferramenta cabe
                        for r in ativas("canto", "raio_interno") {
                            if let Some(l) = r.viola(e.medida) {
                                push(&mut rel, r, e.medida, l, e.centro, faces.clone());
                            }
                        }
                        // bolso: altura da parede do canto dividida pelo raio
                        if e.medida > 1e-9 {
                            let altura = altura_da_face(&m, &faces, e.direcao);
                            for r in ativas("bolso", "profundidade_raio") {
                                let v = altura / e.medida;
                                if let Some(l) = r.viola(v) {
                                    push(&mut rel, r, v, l, e.centro, faces.clone());
                                }
                            }
                        }
                    } else if let Some(t) = t_chapa {
                        for r in ativas("chapa", "raio_dobra_espessura") {
                            let v = e.medida / t;
                            if let Some(l) = r.viola(v) {
                                push(&mut rel, r, v, l, e.centro, faces.clone());
                            }
                        }
                    }
                }
                Tipo::Chanfro => {}
            }
        }
    }
    if let Some(t) = t_chapa {
        for r in ativas("chapa", "espessura") {
            if let Some(l) = r.viola(t) {
                push(&mut rel, r, t, l, centro_da_caixa(lo, hi), vec![]);
            }
        }
    }

    // canto vivo: aresta côncava sem filete (raio zero)
    let rs = ativas("canto", "raio_interno");
    if !rs.is_empty() {
        for c in geo::cantos_vivos(&mesh, &m, diag * 1e-3) {
            for r in &rs {
                if let Some(l) = r.viola(0.) {
                    push(&mut rel, r, 0., l, c.ponto, c.faces.to_vec());
                }
            }
        }
    }

    // saliência
    for r in ativas("saliencia", "angulo") {
        let lim = r.max.unwrap_or(45.);
        for f in geo::saliencia(&m, perfil.direcao, lim) {
            push(&mut rel, r, f.pior, lim, f.ponto, vec![f.face]);
        }
    }

    // saída e undercut
    let rs_saida = ativas("saida", "angulo");
    let rs_under = ativas("undercut", "area");
    if !rs_saida.is_empty() || !rs_under.is_empty() {
        let minimo = rs_saida.iter().filter_map(|r| r.min).fold(f64::MAX, f64::min);
        let minimo = if minimo == f64::MAX { 1. } else { minimo };
        rel.saida = geo::saida(&m, perfil.direcao, minimo);
        for f in rel.saida.clone().iter() {
            for r in &rs_saida {
                // face plana perpendicular à abertura (saída 90) e a que está bloqueada não entram aqui
                if f.saida_max >= 89.5 && f.saida_min >= 89.5 {
                    continue;
                }
                if let Some(l) = r.viola(f.saida_min) {
                    push(&mut rel, r, f.saida_min, l, f.ponto, vec![f.face]);
                }
            }
            for r in &rs_under {
                if let Some(l) = r.viola(f.area_undercut) {
                    push(&mut rel, r, f.area_undercut, l, f.ponto, vec![f.face]);
                }
            }
        }
    }
    rel.violacoes.sort_by(|a, b| b.severidade.cmp(&a.severidade).then(a.regra.cmp(&b.regra)));
    Ok(rel)
}

fn centro_da_caixa(lo: V3, hi: V3) -> V3 {
    [(lo[0] + hi[0]) / 2., (lo[1] + hi[1]) / 2., (lo[2] + hi[2]) / 2.]
}

/// Espessura da faixa que cobre mais área: a "espessura da chapa".
pub fn dominante(a: &[geo::Amostra]) -> Option<f64> {
    geo::faixas(a, TOL_FAIXA).into_iter().max_by(|x, y| x.area.total_cmp(&y.area)).map(|f| f.media)
}

/// Extensão da face ao longo de `eixo`, em mm.
fn altura_da_face(m: &Malha, faces: &[usize], eixo: V3) -> f64 {
    let n = (eixo[0] * eixo[0] + eixo[1] * eixo[1] + eixo[2] * eixo[2]).sqrt();
    if n < 1e-12 {
        return 0.;
    }
    let (mut lo, mut hi) = (f64::MAX, f64::MIN);
    for t in m.tris.iter().filter(|t| faces.contains(&t.face)) {
        for p in &t.p {
            let h = (p[0] * eixo[0] + p[1] * eixo[1] + p[2] * eixo[2]) / n;
            lo = lo.min(h);
            hi = hi.max(h);
        }
    }
    if hi >= lo { hi - lo } else { 0. }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perfis_de_fabrica_leem_e_so_usam_medidas_conhecidas() {
        let v = perfis_padrao();
        assert_eq!(v.len(), 4, "algum JSON de fábrica não leu");
        for p in &v {
            assert!(!p.regras.is_empty(), "{}", p.nome);
            let mut ids: Vec<_> = p.regras.iter().map(|r| r.id.as_str()).collect();
            ids.sort();
            ids.dedup();
            assert_eq!(ids.len(), p.regras.len(), "id repetido em {}", p.nome);
        }
    }

    #[test]
    fn json_rejeita_medida_inventada_e_limite_trocado() {
        let ruim = r#"{"nome":"x","processo":"cnc","regras":[{"id":"a","entidade":"furo","medida":"cor","min":1,"severidade":"aviso","texto":"t"}]}"#;
        assert!(Perfil::de_json(ruim).unwrap_err().contains("desconhecida"));
        let troca = r#"{"nome":"x","processo":"cnc","regras":[{"id":"a","entidade":"furo","medida":"diametro","min":5,"max":1,"severidade":"aviso","texto":"t"}]}"#;
        assert!(Perfil::de_json(troca).unwrap_err().contains("mínimo maior"));
    }

    #[test]
    fn ida_e_volta_do_json_e_ajuste_de_limite() {
        let mut p = perfis_padrao().remove(0);
        let outra = Perfil::de_json(&p.para_json()).unwrap();
        assert_eq!(p, outra);
        let id = p.regras[0].id.clone();
        assert!(p.ajusta(&id, Some(7.), None));
        assert_eq!(p.regra(&id).unwrap().min, Some(7.));
        assert!(!p.ajusta("nao_existe", None, None));
    }

    #[test]
    fn pasta_do_usuario_substitui_o_perfil_de_fabrica_pelo_nome() {
        let dir = std::env::temp_dir().join(format!("dfm7_pasta_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut p = perfis_padrao().remove(0);
        let nome = p.nome.clone();
        let id = p.regras[0].id.clone();
        p.ajusta(&id, Some(123.), None);
        std::fs::write(dir.join("meu.json"), p.para_json()).unwrap();
        std::fs::write(dir.join("quebrado.json"), "{ nao e json").unwrap();
        let (v, erros) = carregar_pasta(&dir);
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(v.len(), 4);
        assert_eq!(v.iter().find(|q| q.nome == nome).unwrap().regra(&id).unwrap().min, Some(123.));
        assert_eq!(erros.len(), 1);
    }

    #[test]
    fn regra_viola_so_fora_do_intervalo() {
        let r = Regra { id: "a".into(), entidade: "parede".into(), medida: "espessura".into(), min: Some(1.), max: Some(4.), severidade: Severidade::Aviso, texto: "{valor} fora de {limite}".into(), ativa: true };
        assert_eq!(r.viola(0.5), Some(1.));
        assert_eq!(r.viola(5.), Some(4.));
        assert_eq!(r.viola(2.), None);
        assert_eq!(texto(&r, 0.5, 1.), "0.50 fora de 1.00");
    }
}
