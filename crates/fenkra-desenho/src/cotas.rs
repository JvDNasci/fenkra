// SPDX-License-Identifier: LGPL-2.1-or-later
//! Cotagem inteligente (B107): o que foi clicado propõe o tipo da cota, Tab troca entre os tipos que
//! servem para aquele alvo, e as opções de exibição (diâmetro, entre parênteses, inspeção) e a
//! precisão da tolerância ficam na própria cota. A lógica fica aqui, sem interface, para ser
//! testada: a tela só pega o alvo sob o cursor e desenha a cota viva.

use crate::source::Axes;
use crate::{Dim, DimKind, V2};
use fenkra_core::Id;
use serde::{Deserialize, Serialize};

/// Opções de exibição da cota além do texto e da tolerância.
#[derive(Clone, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub struct DimOpcoes {
    /// escreve o valor linear como diâmetro (Ø)
    #[serde(default)]
    pub como_diametro: bool,
    /// entre parênteses: cota de referência
    #[serde(default)]
    pub parenteses: bool,
    /// valor dentro de um oval: cota de inspeção
    #[serde(default)]
    pub inspecao: bool,
    /// casas decimais só da tolerância (None = as do valor)
    #[serde(default)]
    pub precisao_tol: Option<u8>,
    /// meia cota (B149): peça simétrica cotada até o eixo; só a seta do lado medido, a linha passa do
    /// eixo e o valor é o dobro do medido
    #[serde(default)]
    pub meia: bool,
    /// cota de ângulo escrita como comprimento do arco (B149): raio × ângulo, com o sinal ⌒
    #[serde(default)]
    pub arco: bool,
}

impl DimOpcoes {
    pub fn padrao(&self) -> bool {
        *self == DimOpcoes::default()
    }
}

/// O que o cursor achou numa vista (coordenadas da vista, mm do modelo).
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Alvo {
    /// vértice
    Ponto(V2),
    /// trecho reto de aresta
    Aresta { a: V2, b: V2 },
    /// arco ou círculo: centro e um ponto dele
    Arco { centro: V2, ponto: V2 },
}

/// Por que não há âncora sob o cursor (o cursor mostra a frase).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Motivo {
    /// o cursor está fora de qualquer vista
    ForaDeVista,
    /// está sobre a vista, mas longe de aresta ou vértice
    SemAresta,
    /// a vista está em erro ou ainda sem projeção
    VistaSemProjecao,
    /// o alvo é de outra vista da cota em andamento
    OutraVista,
}

impl Motivo {
    pub fn texto(self) -> &'static str {
        match self {
            Motivo::ForaDeVista => "Passe o cursor sobre uma vista",
            Motivo::SemAresta => "Sem aresta ou vértice aqui",
            Motivo::VistaSemProjecao => "A vista não tem projeção válida",
            Motivo::OutraVista => "Use a mesma vista da primeira seleção",
        }
    }
}

fn horizontal(a: V2, b: V2) -> bool {
    (b[1] - a[1]).abs() <= (b[0] - a[0]).abs() * 0.02
}

fn vertical(a: V2, b: V2) -> bool {
    (b[0] - a[0]).abs() <= (b[1] - a[1]).abs() * 0.02
}

fn lineares(a: V2, b: V2) -> Vec<DimKind> {
    let proposto = if horizontal(a, b) {
        DimKind::Horizontal
    } else if vertical(a, b) {
        DimKind::Vertical
    } else {
        DimKind::Aligned
    };
    let mut v = vec![proposto];
    for k in [DimKind::Aligned, DimKind::Horizontal, DimKind::Vertical] {
        if !v.contains(&k) {
            v.push(k);
        }
    }
    v
}

/// Tipos que servem para a seleção atual, o proposto primeiro. Tab percorre a lista.
pub fn tipos(sel: &[Alvo]) -> Vec<DimKind> {
    match sel {
        [Alvo::Arco { .. }] => vec![DimKind::Radius, DimKind::Diameter],
        [Alvo::Aresta { a, b }] => lineares(*a, *b),
        [Alvo::Ponto(a), Alvo::Ponto(b)] => lineares(*a, *b),
        // duas arestas que se cruzam: o ângulo entre elas
        [Alvo::Aresta { .. }, Alvo::Aresta { .. }] => vec![DimKind::Angle],
        _ => vec![],
    }
}

/// A seleção já basta para a cota viva seguir o mouse? (um ponto sozinho espera o segundo)
pub fn completa(sel: &[Alvo]) -> bool {
    !tipos(sel).is_empty()
}

/// Cruzamento das retas suportes de duas arestas (vértice do ângulo).
fn cruzamento(a: (V2, V2), b: (V2, V2)) -> Option<V2> {
    let (r, s) = ([a.1[0] - a.0[0], a.1[1] - a.0[1]], [b.1[0] - b.0[0], b.1[1] - b.0[1]]);
    let den = r[0] * s[1] - r[1] * s[0];
    if den.abs() < 1e-12 {
        return None;
    }
    let ap = [b.0[0] - a.0[0], b.0[1] - a.0[1]];
    let t = (ap[0] * s[1] - ap[1] * s[0]) / den;
    Some([a.0[0] + r[0] * t, a.0[1] + r[1] * t])
}

/// Pontos (na vista) que ancoram a cota do `tipo` para a seleção, na ordem que `Dim` espera.
pub fn pontos(sel: &[Alvo], tipo: DimKind) -> Option<Vec<V2>> {
    match (sel, tipo) {
        ([Alvo::Arco { centro, ponto }], DimKind::Radius | DimKind::Diameter) => Some(vec![*centro, *ponto]),
        ([Alvo::Aresta { a, b }], DimKind::Horizontal | DimKind::Vertical | DimKind::Aligned) => Some(vec![*a, *b]),
        ([Alvo::Ponto(a), Alvo::Ponto(b)], DimKind::Horizontal | DimKind::Vertical | DimKind::Aligned) => Some(vec![*a, *b]),
        ([Alvo::Aresta { a: a1, b: b1 }, Alvo::Aresta { a: a2, b: b2 }], DimKind::Angle) => {
            let v = cruzamento((*a1, *b1), (*a2, *b2))?;
            // ponta de cada aresta mais longe do vértice: o lado que a cota mede
            let longe = |a: V2, b: V2| if (a[0] - v[0]).hypot(a[1] - v[1]) > (b[0] - v[0]).hypot(b[1] - v[1]) { a } else { b };
            Some(vec![v, longe(*a1, *b1), longe(*a2, *b2)])
        }
        _ => None,
    }
}

/// Monta a cota da seleção no tipo pedido, com o afastamento dado.
pub fn montar(view: Id, ax: &Axes, sel: &[Alvo], tipo: DimKind, offset: f64) -> Option<Dim> {
    let pts = pontos(sel, tipo)?;
    let offset = if tipo == DimKind::Angle { 0. } else { offset };
    Some(Dim { id: Id(0), view, kind: tipo, anchors: pts.into_iter().map(|p| ax.unproject(p)).collect(), offset, ..Default::default() })
}

/// Próximo tipo da lista (Tab), voltando ao primeiro no fim.
pub fn proximo(lista: &[DimKind], atual: DimKind) -> DimKind {
    match lista.iter().position(|k| *k == atual) {
        Some(i) => lista[(i + 1) % lista.len()],
        None => lista.first().copied().unwrap_or(atual),
    }
}

/// Afastamento que o mouse pede, com sinal (lado). `pts` são as âncoras na folha (mm) e `m` o
/// cursor na folha.
pub fn afastamento(tipo: DimKind, pts: &[V2], m: V2) -> f64 {
    match tipo {
        DimKind::Horizontal | DimKind::Vertical | DimKind::Aligned if pts.len() >= 2 => {
            let (p1, p2) = (pts[0], pts[1]);
            let n = match tipo {
                DimKind::Horizontal => [0., 1.],
                DimKind::Vertical => [-1., 0.],
                _ => {
                    let d = [p2[0] - p1[0], p2[1] - p1[1]];
                    let l = d[0].hypot(d[1]).max(1e-9);
                    [-d[1] / l, d[0] / l]
                }
            };
            let dot = |p: V2| p[0] * n[0] + p[1] * n[1];
            let (hi, lo) = (dot(p1).max(dot(p2)), dot(p1).min(dot(p2)));
            let c = dot(m);
            // acima do ponto mais alto: afastamento positivo; abaixo do mais baixo: negativo
            if c >= (hi + lo) / 2. {
                (c - hi).max(1.)
            } else {
                (c - lo).min(-1.)
            }
        }
        DimKind::Radius | DimKind::Diameter if pts.len() >= 2 => {
            // distância do cursor ao arco, com mínimo
            let r = (pts[1][0] - pts[0][0]).hypot(pts[1][1] - pts[0][1]);
            ((m[0] - pts[0][0]).hypot(m[1] - pts[0][1]) - r).max(3.)
        }
        _ => 0.,
    }
}

/// Texto do valor com as opções de exibição: Ø no valor linear e parênteses.
pub fn envolver(op: &DimOpcoes, kind: DimKind, base: String) -> String {
    let linear = matches!(kind, DimKind::Horizontal | DimKind::Vertical | DimKind::Aligned);
    let b = if op.como_diametro && linear && !base.starts_with('Ø') { format!("Ø{base}") } else { base };
    if op.parenteses {
        format!("({b})")
    } else {
        b
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arco_propoe_raio_e_tab_troca_para_diametro() {
        let sel = [Alvo::Arco { centro: [0., 0.], ponto: [3., 0.] }];
        let t = tipos(&sel);
        assert_eq!(t, vec![DimKind::Radius, DimKind::Diameter]);
        assert_eq!(proximo(&t, DimKind::Radius), DimKind::Diameter);
        assert_eq!(proximo(&t, DimKind::Diameter), DimKind::Radius, "volta ao primeiro");
    }

    #[test]
    fn aresta_horizontal_vertical_e_inclinada_propoem_cada_uma_o_seu() {
        let h = tipos(&[Alvo::Aresta { a: [0., 0.], b: [10., 0.] }]);
        let v = tipos(&[Alvo::Aresta { a: [0., 0.], b: [0., 10.] }]);
        let i = tipos(&[Alvo::Aresta { a: [0., 0.], b: [10., 7.] }]);
        assert_eq!((h[0], v[0], i[0]), (DimKind::Horizontal, DimKind::Vertical, DimKind::Aligned));
        for l in [&h, &v, &i] {
            assert_eq!(l.len(), 3, "as três lineares estão na lista do Tab");
        }
    }

    #[test]
    fn um_ponto_espera_o_segundo_e_dois_pontos_medem_a_distancia() {
        assert!(!completa(&[Alvo::Ponto([0., 0.])]));
        let s = [Alvo::Ponto([0., 0.]), Alvo::Ponto([5., 5.])];
        assert!(completa(&s));
        assert_eq!(tipos(&s)[0], DimKind::Aligned);
        assert_eq!(pontos(&s, DimKind::Horizontal).unwrap(), vec![[0., 0.], [5., 5.]]);
    }

    #[test]
    fn duas_arestas_dao_o_angulo_no_cruzamento() {
        let s = [Alvo::Aresta { a: [0., 0.], b: [10., 0.] }, Alvo::Aresta { a: [0., 0.], b: [0., 10.] }];
        assert_eq!(tipos(&s), vec![DimKind::Angle]);
        let p = pontos(&s, DimKind::Angle).unwrap();
        assert_eq!(p, vec![[0., 0.], [10., 0.], [0., 10.]]);
        // paralelas não têm vértice
        let par = [Alvo::Aresta { a: [0., 0.], b: [10., 0.] }, Alvo::Aresta { a: [0., 5.], b: [10., 5.] }];
        assert!(pontos(&par, DimKind::Angle).is_none());
    }

    #[test]
    fn tipo_que_nao_serve_ao_alvo_nao_monta() {
        let arco = [Alvo::Arco { centro: [0., 0.], ponto: [3., 0.] }];
        assert!(pontos(&arco, DimKind::Horizontal).is_none());
        let ax = Axes::of(crate::Dir::Frente);
        let d = montar(Id(7), &ax, &arco, DimKind::Diameter, 6.).unwrap();
        assert_eq!((d.kind, d.view, d.anchors.len()), (DimKind::Diameter, Id(7), 2));
        assert!(montar(Id(7), &ax, &arco, DimKind::Vertical, 6.).is_none());
    }

    #[test]
    fn afastamento_segue_o_lado_do_mouse() {
        let pts = [[0., 0.], [20., 0.]];
        assert!((afastamento(DimKind::Horizontal, &pts, [10., 12.]) - 12.).abs() < 1e-9);
        assert!((afastamento(DimKind::Horizontal, &pts, [10., -9.]) + 9.).abs() < 1e-9);
        let r = [[0., 0.], [5., 0.]];
        assert!((afastamento(DimKind::Radius, &r, [20., 0.]) - 15.).abs() < 1e-9);
    }

    #[test]
    fn opcoes_de_exibicao_envolvem_o_valor() {
        let op = DimOpcoes { como_diametro: true, parenteses: true, ..Default::default() };
        assert_eq!(envolver(&op, DimKind::Horizontal, "12".into()), "(Ø12)");
        assert_eq!(envolver(&op, DimKind::Radius, "R3".into()), "(R3)", "raio não ganha Ø");
        assert_eq!(envolver(&DimOpcoes::default(), DimKind::Aligned, "7".into()), "7");
        assert!(DimOpcoes::default().padrao());
    }

    #[test]
    fn chamada_do_furo_vem_do_gerador_das_features() {
        let doc = fenkra_solidos::example_document();
        let dir = std::env::temp_dir().join("fenkra_desenho_cotas_furo");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("placa.fkp"), doc.to_json()).unwrap();
        let src = crate::Source { path: "placa.fkp".into(), display_state: String::new(), hidden: vec![] };
        let mut m = crate::source::Models::new(Some(dir));
        // o diâmetro do furo da peça de exemplo, pelo mesmo gerador
        let esperado = doc
            .features
            .iter()
            .find_map(|f| fenkra_core::pmi_feature::callout(&doc, f.id, true).filter(|c| c.hole_diameter.is_some()))
            .expect("a peça de exemplo tem furo");
        let d = esperado.hole_diameter.unwrap();
        let c = m.chamada_furo(&src, d).expect("achou pelo diâmetro");
        assert_eq!(c.text, esperado.text, "folha e 3D falam a mesma chamada");
        assert!(c.text.contains('Ø'), "{}", c.text);
        assert!(m.chamada_furo(&src, d + 5.).is_none(), "diâmetro sem furo não inventa chamada");
    }

    #[test]
    fn oval_da_inspecao_e_parenteses_aparecem_no_desenho() {
        let mut d = crate::Drawing::new("t", crate::Format::A3);
        let s = d.sheets[0].id;
        let src = crate::Source { path: "x.fkp".into(), display_state: String::new(), hidden: vec![] };
        let v = d.add_view(s, src, crate::ViewKind::Standard(crate::Dir::Frente), [100., 100.], 1.);
        let mut dm = Dim { id: Id(9), view: v, kind: DimKind::Horizontal, ..Default::default() };
        assert_eq!(crate::layout::dim_text(&d, &dm, 12.5), "12,5");
        dm.opcoes = DimOpcoes { como_diametro: true, parenteses: true, precisao_tol: Some(3), ..Default::default() };
        dm.tol_mode = crate::TolMode::Simetrica;
        dm.tol_a = 0.1;
        // valor com as casas do desenho (2 por padrão no desenho novo), tolerância com 3
        assert_eq!(crate::layout::dim_text(&d, &dm, 12.5), "(Ø12,5) ±0,1");
        dm.tol_a = 0.125;
        assert_eq!(crate::layout::dim_text(&d, &dm, 12.5), "(Ø12,5) ±0,125", "a tolerância tem a precisão própria");
        let r = crate::Drawing::from_json(&{ let mut x = d.clone(); x.dims.push(dm.clone()); x }.to_json()).unwrap();
        assert_eq!(r.dims[0].opcoes, dm.opcoes);
    }
}
