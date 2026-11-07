// SPDX-License-Identifier: LGPL-2.1-or-later
//! Subconjunto flexível: o mecanismo dele posto na montagem de cima (B422).
//!
//! Na montagem de cima um subconjunto é um corpo só, com as posições gravadas no `.fka` dele. Por
//! isso a direção do kart virava aberta no próprio `.fka` e ficava reta no conjunto.
//!
//! Aqui a montagem de cima guarda, por nó de subconjunto, o valor de cada junta de dentro. Ao ler o
//! subconjunto, [`aplicar`] comanda essas juntas e resolve a cópia em memória: o `.fka` não muda, e
//! duas instâncias do mesmo arquivo podem ficar em posições diferentes. A vista, as interferências
//! e o pacote leem a mesma posição.
//!
//! Só o primeiro nível é flexível, e uma restrição da montagem de cima segue vendo o subconjunto
//! como um corpo só.

use crate::kin::{Grau, JointKind};
use crate::*;

/// Valor de uma junta de dentro do subconjunto `no`, nesta montagem.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct PosicaoFlex {
    pub no: Id,
    pub junta: Id,
    /// graus, na referência dos comandos da junta
    #[serde(default)]
    pub giro: Option<f64>,
    /// mm
    #[serde(default)]
    pub curso: Option<f64>,
}

/// Junta do subconjunto que dá para comandar de fora.
#[derive(Clone, PartialEq, Debug)]
pub struct JuntaFlex {
    pub id: Id,
    pub nome: String,
    pub grau: Grau,
}

/// Graus comandáveis das juntas do subconjunto. Ponto em curva e esférica não têm um valor único.
pub fn juntas(sub: &Assembly) -> Vec<JuntaFlex> {
    let mut lista = vec![];
    // a esférica declara o giro para medir e limitar, mas ele não diz em torno de qual eixo ela está
    for j in sub.joints.iter().filter(|j| !j.suppressed && !matches!(j.kind, JointKind::Spherical)) {
        for g in j.kind.graus().iter().filter(|g| matches!(g, Grau::Giro | Grau::Curso)) {
            lista.push(JuntaFlex { id: j.id, nome: j.name.clone(), grau: *g });
        }
    }
    lista
}

/// Maior avanço de um comando entre duas soluções (graus ou mm).
const PASSO: f64 = 5.;
/// Teto de passos, para um valor absurdo não travar a composição.
const PASSOS_MAX: f64 = 144.;

/// Põe o mecanismo na posição pedida, resolvendo a cópia em memória. Devolve `false` se ele não
/// chegou lá: a cópia fica na última posição boa, nunca numa solução que não fechou.
///
/// O caminho é feito em passos, como na simulação. Um salto grande tira o mecanismo do ramo em que
/// ele está: os tirantes da direção não fecham, e a solução espalha o erro pelas peças. Todas as
/// juntas andam juntas, porque comandar uma por vez deixaria cada solução desfazer a anterior.
pub fn aplicar(sub: &mut Assembly, posicoes: &[PosicaoFlex]) -> bool {
    if posicoes.is_empty() {
        return true;
    }
    let lugares: std::collections::HashMap<Id, Placement> = sub.nodes.iter().map(|n| (n.id, n.placement)).collect();
    let inicio: Vec<(PosicaoFlex, f64, f64)> = posicoes
        .iter()
        .filter_map(|p| {
            let (giro, curso) = crate::kin::measure(sub.joints.iter().find(|j| j.id == p.junta)?, &lugares)?;
            Some((*p, giro, curso))
        })
        .collect();
    let falta = inicio.iter().map(|(p, g, c)| (p.giro.map_or(0., |v| (v - g).abs())).max(p.curso.map_or(0., |v| (v - c).abs()))).fold(0., f64::max);
    let passos = (falta / PASSO).ceil().clamp(1., PASSOS_MAX) as usize;
    for k in 1..=passos {
        let f = k as f64 / passos as f64;
        for (p, g, c) in &inicio {
            let Some(j) = sub.joints.iter_mut().find(|j| j.id == p.junta) else { continue };
            if let Some(v) = p.giro {
                j.drive_angle = Some(valor_cota(g + (v - g) * f));
            }
            if let Some(v) = p.curso {
                j.drive_len = Some(valor_cota(c + (v - c) * f));
            }
        }
        let boas = sub.nodes.clone();
        if !crate::solve::solve(sub).converged {
            sub.nodes = boas;
            return false;
        }
    }
    true
}

/// O mecanismo do subconjunto no arquivo `arquivo` chega às posições do nó `no`?
pub fn alcanca(a: &Assembly, no: Id, arquivo: &std::path::Path) -> bool {
    let lido = std::fs::read_to_string(arquivo).ok().and_then(|t| Assembly::from_json(&t).ok());
    lido.is_none_or(|mut sub| aplicar(&mut sub, &a.flex_de(no)))
}

impl Assembly {
    /// Posições guardadas para o subconjunto do nó `no`.
    pub fn flex_de(&self, no: Id) -> Vec<PosicaoFlex> {
        self.flex.iter().filter(|p| p.no == no).copied().collect()
    }

    /// Valor pedido para um grau da junta, se há um.
    pub fn valor_flex(&self, no: Id, junta: Id, grau: Grau) -> Option<f64> {
        let p = self.flex.iter().find(|p| p.no == no && p.junta == junta)?;
        if grau == Grau::Giro {
            p.giro
        } else {
            p.curso
        }
    }

    /// Guarda o valor de um grau da junta; `None` devolve esse grau ao que está no `.fka`.
    pub fn definir_flex(&mut self, no: Id, junta: Id, grau: Grau, valor: Option<f64>) {
        let k = match self.flex.iter().position(|p| p.no == no && p.junta == junta) {
            Some(k) => k,
            None => {
                self.flex.push(PosicaoFlex { no, junta, giro: None, curso: None });
                self.flex.len() - 1
            }
        };
        if grau == Grau::Giro {
            self.flex[k].giro = valor;
        } else {
            self.flex[k].curso = valor;
        }
        self.flex.retain(|p| p.giro.is_some() || p.curso.is_some());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interferencia::{analisar, OpcoesInterf};
    use crate::kin::{JFrame, JRef, JointKind};
    use crate::load::PartCache;
    use crate::view::compor;
    use fenkra_core::{BoolOp, Document, Extent, Extrude, FeatureKind, OriginPlane, PlaneRef, Sketch, SketchData};
    use std::path::{Path, PathBuf};

    fn pasta(nome: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("fenkra_flex_{nome}"));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    /// Barra de 100 × 10 × 10 que sai da origem ao longo de X.
    fn barra() -> Document {
        let mut d = Document::new("Barra");
        let mut n = 8_200_000u64;
        let mut al = || {
            n += 1;
            Id(n)
        };
        let mut s = SketchData::default();
        s.add_rect(&mut al, [0., -5.], [100., 5.], false);
        let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data: s }));
        d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind("10".into()), reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
        d
    }

    /// Topo com o subconjunto "braço": base fixa e barra numa revolução em Z, com comando `t`.
    fn topo(dir: &Path) -> (Assembly, Id, Id) {
        std::fs::write(dir.join("barra.fkp"), barra().to_json()).unwrap();
        let mut sub = Assembly::new("Braço");
        let base = sub.add_component("Base", NodeKind::Part { path: "vazia.fkp".into() }, Placement::default(), None);
        sub.node_mut(base).unwrap().fixed = true;
        let b = sub.add_component("Barra", NodeKind::Part { path: "barra.fkp".into() }, Placement::default(), None);
        let f = JFrame::from_axis([0.; 3], [0., 0., 1.]);
        let j = sub.add_joint(JointKind::Revolute, JRef { comp: base, frame: f }, JRef { comp: b, frame: f });
        sub.joints[0].drive_angle = Some("t".into());
        std::fs::write(dir.join("braco.fka"), sub.to_json()).unwrap();
        let mut a = Assembly::new("Topo");
        let no = a.add_component("Braço", NodeKind::SubAssembly { path: "braco.fka".into() }, Placement::default(), None);
        (a, no, j)
    }

    /// Caixa do corpo `k` da vista composta.
    fn caixa(a: &Assembly, dir: &Path, cache: &mut PartCache, k: usize) -> ([f32; 3], [f32; 3]) {
        compor(a, Some(dir), cache, &mut vec![], None).unwrap().caixas[k]
    }

    #[test]
    fn o_mecanismo_do_subconjunto_vira_na_montagem_de_cima() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let dir = pasta("vira");
        let (mut a, no, j) = topo(&dir);
        let original = std::fs::read_to_string(dir.join("braco.fka")).unwrap();
        let mut cache = PartCache::default();
        let (lo, hi) = caixa(&a, &dir, &mut cache, 0);
        assert!((hi[0] - 100.).abs() < 1e-3 && (hi[1] - 5.).abs() < 1e-3 && lo[1] > -6., "reta: {lo:?} {hi:?}");

        // 90°: a barra aponta para Y, e o cache não pode devolver a posição antiga
        a.definir_flex(no, j, Grau::Giro, Some(90.));
        let (lo, hi) = caixa(&a, &dir, &mut cache, 0);
        assert!((hi[1] - 100.).abs() < 1e-3 && (hi[0] - 5.).abs() < 1e-3, "virada: {lo:?} {hi:?}");
        assert_eq!(std::fs::read_to_string(dir.join("braco.fka")).unwrap(), original, "o arquivo do subconjunto não muda");

        // a posição vai no arquivo da montagem de cima, e devolver volta ao que está no .fka
        let lida = Assembly::from_json(&a.to_json()).unwrap();
        assert_eq!(lida.valor_flex(no, j, Grau::Giro), Some(90.));
        a.definir_flex(no, j, Grau::Giro, None);
        assert!(a.flex.is_empty());
        assert!((caixa(&a, &dir, &mut cache, 0).1[0] - 100.).abs() < 1e-3);
    }

    #[test]
    fn duas_instancias_ficam_em_posicoes_diferentes() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let dir = pasta("duas");
        let (mut a, no, j) = topo(&dir);
        let outro = a.add_component("Braço 2", NodeKind::SubAssembly { path: "braco.fka".into() }, Placement { origin: [0., 0., 50.], ..Default::default() }, None);
        a.definir_flex(no, j, Grau::Giro, Some(90.));
        a.definir_flex(outro, j, Grau::Giro, Some(-90.));
        let mut cache = PartCache::default();
        assert!((caixa(&a, &dir, &mut cache, 0).1[1] - 100.).abs() < 1e-3);
        assert!((caixa(&a, &dir, &mut cache, 1).0[1] + 100.).abs() < 1e-3);
    }

    #[test]
    fn interferencia_ve_a_posicao_flexionada() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        // obstáculo no caminho da barra virada a 90°, longe da barra reta
        let dir = pasta("interf");
        let (mut a, no, j) = topo(&dir);
        a.add_component("Obstáculo", NodeKind::Part { path: "barra.fkp".into() }, Placement { origin: [-50., 50., 0.], ..Default::default() }, None);
        let mut cache = PartCache::default();
        assert!(analisar(&a, Some(&dir), &mut cache, &OpcoesInterf::default()).is_empty());
        a.definir_flex(no, j, Grau::Giro, Some(90.));
        assert_eq!(analisar(&a, Some(&dir), &mut cache, &OpcoesInterf::default()).len(), 1);
    }

    /// Quatro barras em que a manivela de 10 mm puxa um balancim de 40 mm por uma biela de 50 mm.
    fn quatro_barras() -> (Assembly, Id, Id) {
        let mut a = Assembly::new("Quatro barras");
        let parte = |n: &str| NodeKind::Part { path: n.into() };
        let base = a.add_component("Base", parte("b"), Placement::default(), None);
        a.node_mut(base).unwrap().fixed = true;
        let manivela = a.add_component("Manivela", parte("m"), Placement::default(), None);
        let balancim = a.add_component("Balancim", parte("c"), Placement::default(), None);
        let biela = a.add_component("Biela", parte("d"), Placement::default(), None);
        let z = |o: V3| JFrame::from_axis(o, [0., 0., 1.]);
        // pivôs fixos em (0, 0) e (50, 0); manivela em (10, 0), balancim em (50, 40) e a biela entre eles
        let m = a.add_joint(JointKind::Revolute, JRef { comp: base, frame: z([0.; 3]) }, JRef { comp: manivela, frame: z([0.; 3]) });
        a.add_joint(JointKind::Revolute, JRef { comp: base, frame: z([50., 0., 0.]) }, JRef { comp: balancim, frame: z([50., 0., 0.]) });
        a.add_joint(JointKind::Revolute, JRef { comp: manivela, frame: z([10., 0., 0.]) }, JRef { comp: biela, frame: z([10., 0., 0.]) });
        a.add_joint(JointKind::Revolute, JRef { comp: balancim, frame: z([50., 40., 0.]) }, JRef { comp: biela, frame: z([50., 40., 0.]) });
        assert!(crate::solve::solve(&mut a).converged);
        (a, m, biela)
    }

    fn posicao(j: Id, giro: f64) -> Vec<PosicaoFlex> {
        vec![PosicaoFlex { no: Id(1), junta: j, giro: Some(giro), curso: None }]
    }

    #[test]
    fn mecanismo_com_elos_chega_por_passos() {
        let (mut a, m, _) = quatro_barras();
        assert!(aplicar(&mut a, &posicao(m, 120.)));
        let pl: std::collections::HashMap<Id, Placement> = a.nodes.iter().map(|n| (n.id, n.placement)).collect();
        let (giro, _) = crate::kin::measure(a.joints.iter().find(|j| j.id == m).unwrap(), &pl).unwrap();
        assert!((giro - 120.).abs() < 1e-3, "{giro}");
    }

    #[test]
    fn mecanismo_que_nao_chega_fica_na_ultima_posicao_boa() {
        // biela curta demais para a manivela dar a volta: trava antes de 180°
        let (mut a, m, biela) = quatro_barras();
        a.joints.iter_mut().for_each(|j| j.limites = Default::default());
        let trava = a.joints[1].id;
        a.joints.iter_mut().find(|j| j.id == trava).unwrap().limites.angulo = crate::kin::Faixa::nova(-5., 5.);
        let antes = a.node(biela).unwrap().placement.origin;
        assert!(!aplicar(&mut a, &posicao(m, 150.)));
        let depois = a.node(biela).unwrap().placement.origin;
        assert!(norm(sub(depois, antes)) < 30., "ficou perto de onde estava, sem espalhar o erro: {depois:?}");
        let rep = crate::solve::solve(&mut a.clone());
        assert!(rep.residual.is_finite());
    }

    #[test]
    fn so_giro_e_curso_sao_comandaveis() {
        let mut a = Assembly::new("A");
        let p = a.add_component("P", NodeKind::Part { path: "p".into() }, Placement::default(), None);
        let q = a.add_component("Q", NodeKind::Part { path: "q".into() }, Placement::default(), None);
        let f = JFrame::from_axis([0.; 3], [0., 0., 1.]);
        let cil = a.add_joint(JointKind::Cylindrical, JRef { comp: p, frame: f }, JRef { comp: q, frame: f });
        a.add_joint(JointKind::Spherical, JRef { comp: p, frame: f }, JRef { comp: q, frame: f });
        let lista = juntas(&a);
        assert_eq!(lista.iter().map(|j| (j.id, j.grau)).collect::<Vec<_>>(), vec![(cil, Grau::Giro), (cil, Grau::Curso)]);
    }
}
