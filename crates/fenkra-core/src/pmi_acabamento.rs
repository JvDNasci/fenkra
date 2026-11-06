// SPDX-License-Identifier: LGPL-2.1-or-later
//! Acabamento superficial estruturado e solda por sistema (B190).
//!
//! Rugosidade deixa de ser "Ra mais processo": o símbolo guarda remoção de material (obrigatória,
//! proibida ou indiferente), contorno inteiro, parâmetro, limites superior e inferior, comprimento
//! de amostragem, direção das estrias (lay), sobremetal, família de norma e textura de área. As
//! normas mudam a posição dos campos e o desenho do símbolo, não o significado dos valores, e é
//! por isso que o dado é um só e a norma só escolhe o jeito de escrever.
//!
//! Solda ganha o sistema do símbolo (A: linha cheia mais tracejada; B: duas linhas cheias; AWS:
//! uma linha, lado da seta abaixo) e o texto do cordão da bancada Estrutural vira o comprimento
//! `n×l (e)` do símbolo. Nenhuma das duas tem entidade semântica no AP242: a troca de dados leva a
//! apresentação gráfica (`apresentacao_grafica`), que é o desenho do símbolo como polilinhas 3D.

use crate::doc::Id;
use crate::pmi::{Pmi, PmiKind, TargetClass};
use serde::{Deserialize, Serialize};

/// Remoção de material pedida pelo símbolo.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum Remocao {
    /// qualquer processo (V aberto)
    #[default]
    Qualquer,
    /// remoção obrigatória (V fechado por uma barra)
    Obrigatoria,
    /// remoção proibida (círculo no V)
    Proibida,
}

impl Remocao {
    pub const ALL: [Remocao; 3] = [Remocao::Qualquer, Remocao::Obrigatoria, Remocao::Proibida];
    pub fn label(self) -> &'static str {
        match self {
            Remocao::Qualquer => "Qualquer processo",
            Remocao::Obrigatoria => "Remoção obrigatória",
            Remocao::Proibida => "Remoção proibida",
        }
    }
}

/// Família de norma que decide onde os campos ficam.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum NormaRug {
    #[default]
    Iso1302,
    Iso21920,
    AsmeY1436,
}

impl NormaRug {
    pub const ALL: [NormaRug; 3] = [NormaRug::Iso1302, NormaRug::Iso21920, NormaRug::AsmeY1436];
    pub fn label(self) -> &'static str {
        match self {
            NormaRug::Iso1302 => "ISO 1302",
            NormaRug::Iso21920 => "ISO 21920",
            NormaRug::AsmeY1436 => "ASME Y14.36",
        }
    }
}

/// Direção das estrias da superfície.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum Lay {
    #[default]
    Nenhuma,
    Paralela,
    Perpendicular,
    Cruzada,
    Multidirecional,
    Circular,
    Radial,
    /// sem direção (pontilhada ou granulada)
    Pontual,
}

impl Lay {
    pub const ALL: [Lay; 8] = [Lay::Nenhuma, Lay::Paralela, Lay::Perpendicular, Lay::Cruzada, Lay::Multidirecional, Lay::Circular, Lay::Radial, Lay::Pontual];
    pub fn simbolo(self) -> &'static str {
        match self {
            Lay::Nenhuma => "",
            Lay::Paralela => "=",
            Lay::Perpendicular => "⊥",
            Lay::Cruzada => "X",
            Lay::Multidirecional => "M",
            Lay::Circular => "C",
            Lay::Radial => "R",
            Lay::Pontual => "P",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Lay::Nenhuma => "sem direção indicada",
            Lay::Paralela => "paralela ao plano de projeção",
            Lay::Perpendicular => "perpendicular ao plano de projeção",
            Lay::Cruzada => "cruzada",
            Lay::Multidirecional => "multidirecional",
            Lay::Circular => "circular em torno do centro",
            Lay::Radial => "radial a partir do centro",
            Lay::Pontual => "sem direção (granulada)",
        }
    }
}

/// Campos estruturados do símbolo de rugosidade; o valor principal continua em `Roughness::ra`
/// (limite superior) e o processo em `Roughness::process`.
#[derive(Clone, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub struct RugExtra {
    #[serde(default)]
    pub remocao: Remocao,
    /// a mesma exigência vale para todo o contorno da peça (círculo na junção da barra)
    #[serde(default)]
    pub todo_contorno: bool,
    /// parâmetro: Ra, Rz, Rq, Rt… (vazio = Ra)
    #[serde(default)]
    pub param: String,
    /// limite inferior (o valor principal é o superior)
    #[serde(default)]
    pub limite_inf: String,
    /// o valor é o máximo permitido (regra do máximo), não o que 16 % das medidas podem passar
    #[serde(default)]
    pub maximo: bool,
    #[serde(default)]
    pub amostragem: String,
    #[serde(default)]
    pub lay: Lay,
    /// sobremetal de usinagem (mm)
    #[serde(default)]
    pub sobremetal: String,
    #[serde(default)]
    pub norma: NormaRug,
    /// textura de área (parâmetros S: Sa, Sq, Sz…) em vez de perfil
    #[serde(default)]
    pub area: bool,
}

impl RugExtra {
    /// Nenhum campo além do básico: o símbolo é o de sempre.
    pub fn basico(&self) -> bool {
        *self == RugExtra::default()
    }

    /// Parâmetro efetivo, com o padrão da família (Ra de perfil, Sa de área).
    pub fn parametro(&self) -> &str {
        let p = self.param.trim();
        if p.is_empty() {
            if self.area { "Sa" } else { "Ra" }
        } else {
            p
        }
    }
}

/// Texto do símbolo, campo por campo, já na ordem em que a norma o escreve.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct RugLinhas {
    /// linhas de valor, de cima para baixo (limite superior, depois o inferior)
    pub valores: Vec<String>,
    /// processo de fabricação, abaixo da barra
    pub metodo: String,
    /// símbolo das estrias, à direita
    pub lay: String,
    /// sobremetal, à esquerda (ISO) ou à direita (ASME)
    pub sobremetal: String,
    /// comprimento de amostragem quando a norma o põe fora da linha do valor
    pub amostragem: String,
}

/// Monta os campos. `ra` é o limite superior e `processo` o método de fabricação.
pub fn linhas(ra: &str, processo: &str, x: &RugExtra) -> RugLinhas {
    let p = x.parametro();
    let max = if x.maximo { " max" } else { "" };
    let sup = ra.trim();
    let inf = x.limite_inf.trim();
    let amost = x.amostragem.trim();
    let duas = !inf.is_empty() && !sup.is_empty();
    let mut valores = vec![];
    let com_u = |s: String| if duas { format!("U {s}") } else { s };
    if !sup.is_empty() {
        let mut s = format!("{p}{max} {sup}");
        // ISO 1302 escreve a amostragem antes do parâmetro, separada por barra
        if x.norma == NormaRug::Iso1302 && !amost.is_empty() {
            s = format!("{amost}/{s}");
        }
        valores.push(com_u(s));
    }
    if !inf.is_empty() {
        let s = format!("{p} {inf}");
        valores.push(if duas { format!("L {s}") } else { s });
    }
    let amostragem = if x.norma == NormaRug::Iso1302 || amost.is_empty() { String::new() } else if x.norma == NormaRug::Iso21920 { format!("λc {amost}") } else { amost.to_string() };
    RugLinhas { valores, metodo: processo.trim().to_string(), lay: x.lay.simbolo().to_string(), sobremetal: x.sobremetal.trim().to_string(), amostragem }
}

/// Texto numa linha só (árvore e dica).
pub fn resumo(ra: &str, processo: &str, x: &RugExtra) -> String {
    let l = linhas(ra, processo, x);
    let mut partes = vec![];
    if !l.valores.is_empty() {
        partes.push(l.valores.join(" · "));
    }
    if !l.amostragem.is_empty() {
        partes.push(l.amostragem.clone());
    }
    if !l.lay.is_empty() {
        partes.push(l.lay.clone());
    }
    if !l.sobremetal.is_empty() {
        partes.push(format!("+{}", l.sobremetal));
    }
    let mut s = format!("√ {}", partes.join("  "));
    if !l.metodo.is_empty() {
        s += &format!(" ({})", l.metodo);
    }
    s
}

/// O que o símbolo tem de errado.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum AvisoRug {
    /// o valor não é número
    ValorInvalido(String),
    /// limite inferior maior que o superior
    LimitesInvertidos,
    /// parâmetro de perfil (R…) em textura de área, ou de área (S…) em perfil
    ParametroDeOutraFamilia(String),
    /// remoção proibida e sobremetal juntos se contradizem
    SobremetalSemRemocao,
    /// sem valor e sem processo: o símbolo não diz nada
    SemEspecificacao,
}

fn numero(s: &str) -> Option<f64> {
    s.trim().replace(',', ".").parse::<f64>().ok().filter(|v| *v >= 0.)
}

/// Confere o símbolo.
pub fn validar(ra: &str, processo: &str, x: &RugExtra) -> Vec<AvisoRug> {
    let mut v = vec![];
    for s in [ra, x.limite_inf.as_str(), x.amostragem.as_str(), x.sobremetal.as_str()] {
        if !s.trim().is_empty() && numero(s).is_none() {
            v.push(AvisoRug::ValorInvalido(s.trim().to_string()));
        }
    }
    if let (Some(s), Some(i)) = (numero(ra), numero(&x.limite_inf)) {
        if i > s {
            v.push(AvisoRug::LimitesInvertidos);
        }
    }
    let p = x.parametro();
    let de_area = p.starts_with('S');
    if de_area != x.area {
        v.push(AvisoRug::ParametroDeOutraFamilia(p.to_string()));
    }
    if x.remocao == Remocao::Proibida && !x.sobremetal.trim().is_empty() {
        v.push(AvisoRug::SobremetalSemRemocao);
    }
    if ra.trim().is_empty() && x.limite_inf.trim().is_empty() && processo.trim().is_empty() && x.remocao == Remocao::Qualquer {
        v.push(AvisoRug::SemEspecificacao);
    }
    v
}

/// Caminho 2D: pontos e se fecha.
pub type Caminho = (Vec<[f64; 2]>, bool);

/// Símbolo vetorial (sem textos), vértice na origem, `h` a altura do texto. O V tem a perna curta
/// à esquerda e a longa à direita; a barra sai da perna longa. Remoção obrigatória fecha o V com
/// uma linha entre as pontas; proibida põe um círculo dentro do V; todo o contorno põe um círculo
/// na junção da perna longa com a barra; textura de área desenha um losango na ponta da perna curta;
/// ISO 21920 sobrepõe um segundo traço acima da barra.
pub fn simbolo(x: &RugExtra, h: f64) -> Vec<Caminho> {
    let curta = [-0.5 * h, 1.0 * h];
    let longa_topo = [1.0 * h, 2.0 * h];
    let barra_fim = [longa_topo[0] + 3.0 * h, longa_topo[1]];
    let mut v: Vec<Caminho> = vec![(vec![curta, [0., 0.], longa_topo, barra_fim], false)];
    if x.remocao == Remocao::Obrigatoria {
        // a linha entre as pontas fecha o V em triângulo
        v.push((vec![curta, longa_topo], false));
    }
    if x.remocao == Remocao::Proibida {
        v.push((circulo([0.25 * h, 1.1 * h], 0.4 * h), true));
    }
    if x.todo_contorno {
        v.push((circulo(longa_topo, 0.45 * h), true));
    }
    if x.area {
        let c = [curta[0] - 0.55 * h, curta[1] + 0.35 * h];
        v.push((vec![[c[0], c[1] + 0.45 * h], [c[0] + 0.45 * h, c[1]], [c[0], c[1] - 0.45 * h], [c[0] - 0.45 * h, c[1]]], true));
    }
    if x.norma == NormaRug::Iso21920 {
        v.push((vec![[longa_topo[0], longa_topo[1] + 0.25 * h], [barra_fim[0], barra_fim[1] + 0.25 * h]], false));
    }
    v
}

fn circulo(c: [f64; 2], r: f64) -> Vec<[f64; 2]> {
    (0..24).map(|i| {
        let t = i as f64 / 24. * std::f64::consts::TAU;
        [c[0] + r * t.cos(), c[1] + r * t.sin()]
    }).collect()
}

/// Como o símbolo vai para a troca de dados.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ViaStep {
    /// entidade semântica do AP242
    Semantica,
    /// só a apresentação gráfica (polilinhas)
    Grafica,
}

/// Rugosidade e solda não têm entidade semântica de tolerância no AP242 (rugosidade tem apenas
/// propriedades de face genéricas, solda nenhuma): vão como apresentação gráfica.
pub fn via_step(k: &PmiKind) -> ViaStep {
    match k {
        PmiKind::Roughness { .. } | PmiKind::Weld { .. } => ViaStep::Grafica,
        _ => ViaStep::Semantica,
    }
}

/// Polilinhas 3D do símbolo: `origem` é a ponta da seta, `eixo_x` e `eixo_y` o plano da anotação
/// (unitários), `escala` converte as unidades de `h` em mm do modelo.
pub fn apresentacao_grafica(paths: &[Caminho], origem: [f64; 3], eixo_x: [f64; 3], eixo_y: [f64; 3], escala: f64) -> Vec<Vec<[f64; 3]>> {
    paths
        .iter()
        .map(|(pts, fecha)| {
            let mut v: Vec<[f64; 3]> = pts.iter().map(|q| [0, 1, 2].map(|k| origem[k] + (eixo_x[k] * q[0] + eixo_y[k] * q[1]) * escala)).collect();
            if *fecha && v.len() > 2 {
                v.push(v[0]);
            }
            v
        })
        .collect()
}

/// Rugosidades com a face que elas marcam (alvo de face), para listar o acabamento por face.
pub fn por_face(lista: &[Pmi]) -> Vec<(Id, String)> {
    lista
        .iter()
        .filter_map(|p| match &p.kind {
            PmiKind::Roughness { ra, process, extra } if p.targets.first().copied().flatten().is_some_and(|t| matches!(t.class, TargetClass::FacePlana | TargetClass::FaceCilindrica | TargetClass::FaceOutra)) => Some((p.id, resumo(ra, process, extra))),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::pmi::{WeldContour, WeldExtra, WeldKind};

    #[test]
    fn campos_na_ordem_da_norma() {
        let mut x = RugExtra { limite_inf: "0,8".into(), amostragem: "2,5".into(), lay: Lay::Perpendicular, sobremetal: "0,3".into(), ..Default::default() };
        let l = linhas("3,2", "fresado", &x);
        assert_eq!(l.valores, vec!["U 2,5/Ra 3,2", "L Ra 0,8"]);
        assert_eq!((l.metodo.as_str(), l.lay.as_str(), l.sobremetal.as_str()), ("fresado", "⊥", "0,3"));
        x.norma = NormaRug::AsmeY1436;
        let l = linhas("3,2", "", &x);
        assert_eq!(l.valores[0], "U Ra 3,2");
        assert_eq!(l.amostragem, "2,5", "ASME põe a amostragem fora da linha do valor");
        x.norma = NormaRug::Iso21920;
        assert_eq!(linhas("3,2", "", &x).amostragem, "λc 2,5");
        // só o superior: sem U/L
        let s = linhas("1,6", "", &RugExtra { maximo: true, param: "Rz".into(), ..Default::default() });
        assert_eq!(s.valores, vec!["Rz max 1,6"]);
    }

    #[test]
    fn documento_antigo_sem_extra_continua_basico() {
        assert!(RugExtra::default().basico());
        let k: PmiKind = serde_json::from_str(r#"{"Roughness":{"ra":"1,6","process":"fresado"}}"#).unwrap();
        match k {
            PmiKind::Roughness { extra, .. } => assert!(extra.basico()),
            _ => panic!(),
        }
        assert_eq!(resumo("1,6", "fresado", &RugExtra::default()), "√ Ra 1,6 (fresado)");
    }

    #[test]
    fn validacao() {
        let ok = RugExtra::default();
        assert!(validar("1,6", "", &ok).is_empty());
        let x = RugExtra { limite_inf: "3,2".into(), ..Default::default() };
        assert!(validar("1,6", "", &x).contains(&AvisoRug::LimitesInvertidos));
        assert!(validar("abc", "", &ok).contains(&AvisoRug::ValorInvalido("abc".into())));
        let area = RugExtra { area: true, param: "Ra".into(), ..Default::default() };
        assert!(validar("1,6", "", &area).contains(&AvisoRug::ParametroDeOutraFamilia("Ra".into())));
        assert!(validar("1,6", "", &RugExtra { area: true, ..Default::default() }).is_empty(), "área usa Sa por padrão");
        let proib = RugExtra { remocao: Remocao::Proibida, sobremetal: "0,5".into(), ..Default::default() };
        assert!(validar("1,6", "", &proib).contains(&AvisoRug::SobremetalSemRemocao));
        assert!(validar("", "", &ok).contains(&AvisoRug::SemEspecificacao));
    }

    #[test]
    fn simbolo_muda_com_remocao_contorno_e_area() {
        let base = simbolo(&RugExtra::default(), 3.5).len();
        assert_eq!(base, 1);
        assert!(simbolo(&RugExtra { remocao: Remocao::Obrigatoria, ..Default::default() }, 3.5).len() > base);
        let proib = simbolo(&RugExtra { remocao: Remocao::Proibida, ..Default::default() }, 3.5);
        assert!(proib.iter().any(|(_, f)| *f), "círculo fechado");
        let todo = simbolo(&RugExtra { todo_contorno: true, ..Default::default() }, 3.5);
        assert_eq!(todo.len(), 2);
        assert!(simbolo(&RugExtra { area: true, ..Default::default() }, 3.5).iter().any(|(p, f)| *f && p.len() == 4), "losango");
        let tudo = simbolo(&RugExtra { remocao: Remocao::Proibida, todo_contorno: true, area: true, norma: NormaRug::Iso21920, ..Default::default() }, 3.5);
        assert!(tudo.iter().all(|(p, _)| p.iter().all(|q| q[0].is_finite() && q[1].is_finite())));
        assert_eq!(tudo.len(), 5);
    }

    #[test]
    fn rugosidade_e_solda_vao_como_pmi_grafico() {
        let r = PmiKind::Roughness { ra: "1,6".into(), process: String::new(), extra: Default::default() };
        let w = PmiKind::Weld { kind: WeldKind::Filete, size: "a4".into(), all_around: false, extra: WeldExtra::default() };
        assert_eq!(via_step(&r), ViaStep::Grafica);
        assert_eq!(via_step(&w), ViaStep::Grafica);
        assert_eq!(via_step(&PmiKind::Linear), ViaStep::Semantica);
        let pl = apresentacao_grafica(&simbolo(&RugExtra::default(), 1.), [10., 0., 0.], [1., 0., 0.], [0., 0., 1.], 2.);
        assert_eq!(pl.len(), 1);
        // o vértice (0,0) cai na origem; o fim da barra, 4 u à direita e 2 u acima, em mm do modelo
        assert_eq!(pl[0][1], [10., 0., 0.]);
        assert_eq!(pl[0][3], [10. + 8., 0., 4.]);
    }

    #[test]
    fn sistemas_de_solda_mudam_as_linhas_de_referencia() {
        use crate::pmi::{weld_symbol, SistemaSolda, WeldPrim};
        let h = 3.5;
        let sym = |sis, ac: &str, pen| weld_symbol(WeldKind::Filete, "a4", false, &WeldExtra { sistema: sis, acabamento: ac.into(), penetracao: pen, contour: WeldContour::Convexo, ..Default::default() }, h);
        let tracejadas = |v: &[WeldPrim]| v.iter().filter(|p| matches!(p, WeldPrim::Dashed(_))).count();
        let ref_cheias = |v: &[WeldPrim]| v.iter().filter(|p| matches!(p, WeldPrim::Line(l) if l.len() == 2 && l[0][1] == l[1][1] && (l[0][1] == 0. || (l[0][1] + 0.8 * h).abs() < 1e-9) && l[1][0] - l[0][0] > 4. * h)).count();
        let a = sym(SistemaSolda::A, "", false);
        assert_eq!((tracejadas(&a), ref_cheias(&a)), (1, 1), "A: cheia e tracejada");
        let b = sym(SistemaSolda::B, "", false);
        assert_eq!((tracejadas(&b), ref_cheias(&b)), (0, 2), "B: duas cheias");
        let w = sym(SistemaSolda::Aws, "", false);
        assert_eq!(tracejadas(&w), 0);
        // o triângulo do filete (lado da seta) fica acima da linha no A e abaixo no AWS
        let ymax = |v: &[WeldPrim]| v.iter().filter_map(|p| if let WeldPrim::Fill(f) | WeldPrim::Line(f) = p { Some(f.iter().map(|q| q[1]).fold(f64::MIN, f64::max)) } else { None }).fold(f64::MIN, f64::max);
        let ymin = |v: &[WeldPrim]| v.iter().filter_map(|p| if let WeldPrim::Fill(f) | WeldPrim::Line(f) = p { Some(f.iter().map(|q| q[1]).fold(f64::MAX, f64::min)) } else { None }).fold(f64::MAX, f64::min);
        assert!(ymax(&a) > 0.5 * h && ymin(&w) < -0.5 * h);
        // acabamento e penetração só aparecem quando pedidos
        let com = sym(SistemaSolda::Aws, "G", true);
        assert!(com.iter().any(|p| matches!(p, WeldPrim::Text(_, t, _) if t == "G")));
        assert!(com.len() > w.len());
        // sem o contorno, a letra de acabamento não tem onde ficar
        let sem = weld_symbol(WeldKind::Filete, "a4", false, &WeldExtra { acabamento: "G".into(), ..Default::default() }, h);
        assert!(!sem.iter().any(|p| matches!(p, WeldPrim::Text(_, t, _) if t == "G")));
    }
}
