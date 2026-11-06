// SPDX-License-Identifier: LGPL-2.1-or-later
//! Planificação exata da chapa (B86): contorno, furos, linhas de dobra, ferramentas, propriedades
//! do blank, verificação de sobreposição e DXF com camadas fixas.
//!
//! Como funciona: o documento é recalculado com a chapa planificada. O corpo plano é cortado pelo
//! plano médio da espessura; cada laço da seção vira contorno (laço externo) ou furo (laço dentro
//! de outro). Os laços viram linhas, arcos e círculos exatos (`chapa::chain_segs`), então o DXF
//! sai 1:1 e não em retalhos de reta. As linhas de dobra, a etiqueta e a ordem são as mesmas de
//! `RecomputeResult::bend_lines`, que também alimenta a vista e a tabela.

use crate::chapa;
use crate::geo3d::{self, V3};
use crate::{final_shape, passos_planificados, recompute, tabela_dobras, BendRow, Status};
use fenkra_core::{BoolOp, Document, FeatureKind, Id};
use fenkra_kernel::{Bool, Seg2};

/// Volume comum (mm³) acima do qual duas partes da planificada contam como sobrepostas.
pub const VOLUME_SOBREPOSTO: f64 = 0.01;

/// Camadas fixas do DXF da planificada.
pub const CAMADA_CONTORNO: &str = "CONTORNO";
pub const CAMADA_FUROS: &str = "FUROS";
pub const CAMADA_DOBRA_CIMA: &str = "DOBRA_CIMA";
pub const CAMADA_DOBRA_BAIXO: &str = "DOBRA_BAIXO";
pub const CAMADA_FERRAMENTA: &str = "FERRAMENTA";
pub const CAMADA_ROTULOS: &str = "ROTULOS";

/// Entidade 2D exata do blank (coordenadas em mm, no plano da chapa).
#[derive(Clone, Debug, PartialEq)]
pub enum Entidade {
    Linha([f64; 2], [f64; 2]),
    /// arco anti-horário, ângulos em graus
    Arco { centro: [f64; 2], raio: f64, ini: f64, fim: f64 },
    Circulo { centro: [f64; 2], raio: f64 },
    /// spline amostrada ou laço sem forma conhecida; o bool diz se é fechada
    Polilinha(Vec<[f64; 2]>, bool),
}

/// Linha de dobra na planificada, com a mesma ordem e etiqueta da vista e da tabela.
#[derive(Clone, Debug, PartialEq)]
pub struct DobraPlana {
    pub ordem: usize,
    pub rotulo: String,
    pub a: [f64; 2],
    pub b: [f64; 2],
    pub cima: bool,
    pub angulo: f64,
    pub raio: f64,
    pub desenvolvido: f64,
    /// pontos da linha quando a dobra é curva; vazio na dobra reta de `a` a `b`
    pub curva: Vec<[f64; 2]>,
}

/// Dois comandos que ocupam o mesmo lugar na chapa plana.
#[derive(Clone, Debug, PartialEq)]
pub struct Sobreposicao {
    pub a: Id,
    pub b: Id,
    pub nome_a: String,
    pub nome_b: String,
    pub volume: f64,
}

/// Comando de chapa que não deixou rastro nenhum na planificada.
#[derive(Clone, Debug, PartialEq)]
pub struct Ausente {
    pub feature: Id,
    pub nome: String,
    pub motivo: String,
}

/// Propriedades do blank.
#[derive(Clone, Debug, PartialEq)]
pub struct Propriedades {
    /// caixa do blank no plano (mm)
    pub largura: f64,
    pub altura: f64,
    /// área do blank (mm², com os furos descontados) = volume / espessura
    pub area: f64,
    pub volume: f64,
    /// massa em kg, quando o material da chapa tem densidade na tabela
    pub massa: Option<f64>,
    pub dobras: usize,
}

/// Resultado da planificação.
#[derive(Clone, Debug, PartialEq)]
pub struct Planificada {
    pub espessura: f64,
    pub material: String,
    pub contorno: Vec<Entidade>,
    pub furos: Vec<Entidade>,
    pub ferramenta: Vec<Entidade>,
    pub dobras: Vec<DobraPlana>,
    pub propriedades: Propriedades,
    pub sobreposicoes: Vec<Sobreposicao>,
    pub ausentes: Vec<Ausente>,
}

impl Planificada {
    /// A planificada serve para fabricar: nada some e nada se sobrepõe.
    pub fn valida(&self) -> bool {
        self.sobreposicoes.is_empty() && self.ausentes.is_empty()
    }

    /// Mensagens legíveis dos problemas (vazio quando está tudo certo).
    pub fn problemas(&self) -> Vec<String> {
        let mut v: Vec<String> = self.sobreposicoes.iter().map(|s| format!("sobreposição entre '{}' e '{}' ({:.2} mm³)", s.nome_a, s.nome_b, s.volume)).collect();
        v.extend(self.ausentes.iter().map(|a| format!("'{}' não aparece na planificada: {}", a.nome, a.motivo)));
        v
    }
}

/// Densidade (g/cm³) do material da tabela de chapas. Valores usuais de catálogo.
pub fn densidade(material: &str) -> Option<f64> {
    let d = match material {
        "Aço carbono (SAE 1010/1020)" | "Aço galvanizado" | "Aço alta resistência (S355)" => 7.85,
        "Aço inox 304" => 8.0,
        "Aço inox 430" => 7.7,
        "Alumínio 1100-H14" => 2.71,
        "Alumínio 5052-H32" => 2.68,
        "Alumínio 6061-T6" => 2.70,
        "Cobre" => 8.96,
        "Latão" => 8.5,
        _ => return None,
    };
    Some(d)
}

fn eh_comando_de_chapa(k: &FeatureKind) -> bool {
    matches!(
        k,
        FeatureKind::SmWall(_)
            | FeatureKind::SmFlange(_)
            | FeatureKind::SmHem(_)
            | FeatureKind::SmJoggle(_)
            | FeatureKind::SmBend(_)
            | FeatureKind::SmLouver(_)
            | FeatureKind::SmStamp(_)
            | FeatureKind::SmFlangedHole(_)
            | FeatureKind::SmCut(_)
            | FeatureKind::SmProfileFlange(_)
            | FeatureKind::SmCurveFlange(_)
            | FeatureKind::SmFlangeLoop(_)
            | FeatureKind::SmSurfFlange(_)
            | FeatureKind::SmArcSlot(_)
            | FeatureKind::SmCornerRelief(_)
            | FeatureKind::SmCornerClose(_)
            | FeatureKind::SmLoftFlange(_)
            | FeatureKind::SmUnfoldPartial(_)
            | FeatureKind::SmConvert(_)
    )
}

/// Área com sinal (shoelace) de um laço 2D.
fn area2(l: &[[f64; 2]]) -> f64 {
    let n = l.len();
    (0..n).map(|i| l[i][0] * l[(i + 1) % n][1] - l[(i + 1) % n][0] * l[i][1]).sum::<f64>() / 2.
}

fn dentro(l: &[[f64; 2]], p: [f64; 2]) -> bool {
    let n = l.len();
    let mut c = false;
    let mut j = n - 1;
    for i in 0..n {
        if (l[i][1] > p[1]) != (l[j][1] > p[1]) && p[0] < (l[j][0] - l[i][0]) * (p[1] - l[i][1]) / (l[j][1] - l[i][1]) + l[i][0] {
            c = !c;
        }
        j = i;
    }
    c
}

/// Círculo por três pontos.
fn circulo3(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> Option<([f64; 2], f64)> {
    let d = 2. * (a[0] * (b[1] - c[1]) + b[0] * (c[1] - a[1]) + c[0] * (a[1] - b[1]));
    if d.abs() < 1e-12 {
        return None;
    }
    let (a2, b2, c2) = (a[0] * a[0] + a[1] * a[1], b[0] * b[0] + b[1] * b[1], c[0] * c[0] + c[1] * c[1]);
    let ux = (a2 * (b[1] - c[1]) + b2 * (c[1] - a[1]) + c2 * (a[1] - b[1])) / d;
    let uy = (a2 * (c[0] - b[0]) + b2 * (a[0] - c[0]) + c2 * (b[0] - a[0])) / d;
    Some(([ux, uy], (a[0] - ux).hypot(a[1] - uy)))
}

/// O laço (sem o ponto final repetido) é um círculo?
fn laco_circular(p: &[[f64; 2]]) -> Option<([f64; 2], f64)> {
    if p.len() < 8 {
        return None;
    }
    let n = p.len() as f64;
    let c = [p.iter().map(|q| q[0]).sum::<f64>() / n, p.iter().map(|q| q[1]).sum::<f64>() / n];
    let r = p.iter().map(|q| (q[0] - c[0]).hypot(q[1] - c[1])).sum::<f64>() / n;
    let tol = (r * 2e-3).max(2e-3);
    // os pontos amostrados do círculo ficam todos no raio médio (a amostragem é de arcos exatos)
    p.iter().all(|q| ((q[0] - c[0]).hypot(q[1] - c[1]) - r).abs() < tol).then_some((c, r))
}

fn arco(a: [f64; 2], m: [f64; 2], b: [f64; 2]) -> Option<Entidade> {
    let (c, r) = circulo3(a, m, b)?;
    let ang = |p: [f64; 2]| (p[1] - c[1]).atan2(p[0] - c[0]).to_degrees();
    let (ia, im, ib) = (ang(a), ang(m), ang(b));
    // sentido pelo ponto do meio: anti-horário se a→m→b cresce
    let norm = |x: f64| x.rem_euclid(360.);
    let ccw = norm(im - ia) < norm(ib - ia);
    Some(if ccw { Entidade::Arco { centro: c, raio: r, ini: norm(ia), fim: norm(ib) } } else { Entidade::Arco { centro: c, raio: r, ini: norm(ib), fim: norm(ia) } })
}

/// Giro (rad) no vértice `b` entre os segmentos `a→b` e `b→c`.
fn giro_em(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> f64 {
    let (u, w) = ([b[0] - a[0], b[1] - a[1]], [c[0] - b[0], c[1] - b[1]]);
    let l = u[0].hypot(u[1]) * w[0].hypot(w[1]);
    if l < 1e-18 {
        0.
    } else {
        ((u[0] * w[0] + u[1] * w[1]) / l).clamp(-1., 1.).acos()
    }
}

/// Giro (~20°) a partir do qual um vértice é canto vivo. A amostragem de arcos do kernel gira no
/// máximo ~6° por vértice, então arco nunca passa disso.
const CANTO: f64 = 0.35;

/// Laço fechado (polilinha 2D amostrada) em entidades exatas. O laço é cortado nos cantos vivos;
/// cada trecho liso (retas e arcos tangentes) vira linhas e arcos por `chapa::chain_segs`.
fn laco_em_entidades(p: &[[f64; 2]]) -> Vec<Entidade> {
    // pontos a menos de 1 µm uns dos outros são o mesmo ponto (emendas do kernel)
    let mut v: Vec<[f64; 2]> = vec![];
    for q in p {
        if v.last().is_none_or(|l| (l[0] - q[0]).hypot(l[1] - q[1]) > 1e-3) {
            v.push(*q);
        }
    }
    while v.len() > 1 && (v[0][0] - v[v.len() - 1][0]).hypot(v[0][1] - v[v.len() - 1][1]) <= 1e-3 {
        v.pop();
    }
    if v.len() < 3 {
        return vec![];
    }
    if let Some((c, r)) = laco_circular(&v) {
        return vec![Entidade::Circulo { centro: c, raio: r }];
    }
    let n = v.len();
    let giro = |i: usize| giro_em(v[(i + n - 1) % n], v[i], v[(i + 1) % n]);
    let cantos: Vec<usize> = (0..n).filter(|i| giro(*i) > CANTO).collect();
    let mut out = vec![];
    let mut trecho = |pts: &[[f64; 2]]| {
        for s in chapa::chain_segs(pts) {
            match s {
                Seg2::Line(a, b) => out.push(Entidade::Linha(a, b)),
                Seg2::Arc(a, m, b) => out.push(arco(a, m, b).unwrap_or(Entidade::Linha(a, b))),
                Seg2::Spline(q) => out.push(Entidade::Polilinha(q, false)),
                Seg2::Circle(c, r) => out.push(Entidade::Circulo { centro: c, raio: r }),
                Seg2::Ellipse(c, m, r, t0, t1) => {
                    // o DXF da planificada não tem elipse: sai como polilinha fina
                    let maior = m[0].hypot(m[1]).max(1e-12);
                    let (ux, uy) = (m[0] / maior, m[1] / maior);
                    let q = (0..=64).map(|i| {
                        let t = t0 + (t1 - t0) * i as f64 / 64.;
                        let (a, b) = (maior * t.cos(), r * t.sin());
                        [c[0] + a * ux - b * uy, c[1] + a * uy + b * ux]
                    }).collect();
                    out.push(Entidade::Polilinha(q, false));
                }
            }
        }
    };
    if cantos.is_empty() {
        let mut fechado = v.clone();
        fechado.push(v[0]);
        trecho(&fechado);
        return out;
    }
    for (k, &c0) in cantos.iter().enumerate() {
        let c1 = cantos[(k + 1) % cantos.len()];
        let mut pts = vec![v[c0]];
        let mut i = c0;
        loop {
            i = (i + 1) % n;
            pts.push(v[i]);
            if i == c1 {
                break;
            }
        }
        trecho(&pts);
    }
    out
}

/// Planifica a peça de chapa do documento (planificada exata).
pub fn planificar(doc: &Document) -> Result<Planificada, String> {
    let sheet = doc.sheet.as_ref().ok_or("a peça não é de chapa: defina os parâmetros de chapa")?;
    let mut flat = doc.clone();
    flat.sheet.as_mut().unwrap().flat = true;
    let r = recompute(&flat);
    if let Some(e) = &r.param_error {
        return Err(format!("parâmetros inválidos: {e}"));
    }
    let shape = final_shape(&flat).ok_or("a chapa não gerou corpo na planificada")?;
    let vars = flat.eval_params().map_err(|e| e.to_string())?;
    let t = fenkra_core::expr::eval_str(&sheet.thickness, &vars).map_err(|e| format!("espessura: {e}"))?;

    // plano da chapa: normal da face mais próxima do centro, orientada para o eixo dominante
    let centro = shape.mass().center;
    let (on, nf) = shape.surface_near(centro).map_err(|e| e.0)?;
    let nf = geo3d::unit(nf).ok_or("a chapa não tem normal")?;
    // plano médio da chapa: o ponto da face descido meia espessura contra a normal externa dela
    let origem = geo3d::sub(on, geo3d::mul(nf, t / 2.));
    let mut n = nf;
    let dom = (0..3).max_by(|a, b| n[*a].abs().total_cmp(&n[*b].abs())).unwrap_or(2);
    if n[dom] < 0. {
        n = geo3d::mul(n, -1.);
    }
    let ref_ = if n[0].abs() < 0.9 { [1., 0., 0.] } else { [0., 1., 0.] };
    let ex = geo3d::unit(geo3d::sub(ref_, geo3d::mul(n, geo3d::dot(ref_, n)))).ok_or("plano degenerado")?;
    let ey = fenkra_kernel::cross(n, ex);
    let p2 = |p: V3| {
        let d = geo3d::sub(p, origem);
        [geo3d::dot(d, ex), geo3d::dot(d, ey)]
    };

    // laços da seção: o que está dentro de outro laço é furo
    let (lo, hi) = shape.bbox().ok_or("corpo sem dimensões")?;
    let meia = geo3d::dist(lo, hi) + 10.;
    let lacos3 = shape.section_loops(origem, n, meia).map_err(|e| e.0)?;
    let mut lacos: Vec<Vec<[f64; 2]>> = lacos3.iter().map(|l| l.iter().map(|p| p2(*p)).collect::<Vec<_>>()).filter(|l| l.len() >= 3).collect();
    let mut contorno = vec![];
    let mut furos = vec![];
    let mut externo = vec![false; lacos.len()];
    for (i, l) in lacos.iter().enumerate() {
        let prof = lacos.iter().enumerate().filter(|(j, o)| *j != i && area2(o).abs() > area2(l).abs() && dentro(o, l[0])).count();
        externo[i] = prof % 2 == 0;
    }
    if !externo.iter().any(|e| *e) {
        return Err("a seção da planificada não tem contorno externo".into());
    }
    // o canto inferior esquerdo do blank vira a origem do desenho
    let (mut mn, mut mx) = ([f64::MAX; 2], [f64::MIN; 2]);
    for (l, _) in lacos.iter().zip(&externo).filter(|(_, e)| **e) {
        for q in l.iter() {
            for k in 0..2 {
                mn[k] = mn[k].min(q[k]);
                mx[k] = mx[k].max(q[k]);
            }
        }
    }
    let p2 = |p: V3| {
        let q = p2(p);
        [q[0] - mn[0], q[1] - mn[1]]
    };
    for l in lacos.iter_mut() {
        for q in l.iter_mut() {
            *q = [q[0] - mn[0], q[1] - mn[1]];
        }
    }
    for (l, ext) in lacos.iter().zip(&externo) {
        if *ext {
            contorno.extend(laco_em_entidades(l));
        } else {
            furos.extend(laco_em_entidades(l));
        }
    }

    // marcas de ferramenta (veneziana e estampo) e linhas de dobra, vindas do cálculo
    let mut ferramenta = vec![];
    for o in &r.cut_outlines {
        let Some(f) = flat.feature(o.feature) else { continue };
        if !matches!(f.kind, FeatureKind::SmLouver(_) | FeatureKind::SmStamp(_)) {
            continue;
        }
        for l in &o.flat {
            let p: Vec<[f64; 2]> = l.iter().map(|q| p2(*q)).collect();
            let fechado = p.len() > 2 && (p[0][0] - p[p.len() - 1][0]).hypot(p[0][1] - p[p.len() - 1][1]) < 1e-6;
            if fechado {
                ferramenta.extend(laco_em_entidades(&p));
            } else {
                ferramenta.extend(p.windows(2).map(|w| Entidade::Linha(w[0], w[1])));
            }
        }
    }
    let tabela: Vec<BendRow> = tabela_dobras(&flat, &r.bend_lines);
    let dobras: Vec<DobraPlana> = r
        .bend_lines
        .iter()
        .zip(&tabela)
        .map(|(b, row)| DobraPlana { ordem: row.ordem, rotulo: row.rotulo.clone(), a: p2(b.center[0]), b: p2(b.center[1]), cima: b.up, angulo: b.angle, raio: b.radius, desenvolvido: b.allowance, curva: b.curva.iter().map(|q| p2(*q)).collect() })
        .collect();

    // sobreposição: duas ferramentas que somam material no mesmo lugar da chapa plana
    let passos = passos_planificados(doc);
    let nome = |id: Id| flat.feature(id).map(|f| f.name.clone()).unwrap_or_default();
    let somas: Vec<_> = passos.iter().filter(|p| p.op == BoolOp::Add && !p.ferramenta.is_null()).collect();
    let mut sobreposicoes = vec![];
    for i in 0..somas.len() {
        for j in i + 1..somas.len() {
            if somas[i].feature == somas[j].feature {
                continue;
            }
            let Ok(comum) = somas[i].ferramenta.boolean(&somas[j].ferramenta, Bool::Common) else { continue };
            if comum.is_null() {
                continue;
            }
            let v = comum.mass().volume;
            if v > VOLUME_SOBREPOSTO && !sobreposicoes.iter().any(|s: &Sobreposicao| s.a == somas[i].feature && s.b == somas[j].feature) {
                sobreposicoes.push(Sobreposicao { a: somas[i].feature, b: somas[j].feature, nome_a: nome(somas[i].feature), nome_b: nome(somas[j].feature), volume: v });
            }
        }
    }

    // comando ausente: erro na planificada, ou nenhum rastro (corpo, marca de ferramenta ou dobra)
    let mut ausentes = vec![];
    for (_, f) in flat.active_features() {
        if !eh_comando_de_chapa(&f.kind) {
            continue;
        }
        if let Some(Status::Error(m)) = r.status.get(&f.id) {
            ausentes.push(Ausente { feature: f.id, nome: f.name.clone(), motivo: m.clone() });
            continue;
        }
        let rastro = passos.iter().any(|p| p.feature == f.id)
            || r.cut_outlines.iter().any(|o| o.feature == f.id)
            || r.bend_lines.iter().any(|b| b.feature == f.id);
        if !rastro {
            ausentes.push(Ausente { feature: f.id, nome: f.name.clone(), motivo: "o comando não deixou geometria, marca de ferramenta nem dobra na planificada".into() });
        }
    }

    let volume = shape.mass().volume;
    let massa = densidade(&sheet.material).map(|d| volume * d * 1e-6);
    Ok(Planificada {
        espessura: t,
        material: sheet.material.clone(),
        contorno,
        furos,
        ferramenta,
        propriedades: Propriedades { largura: mx[0] - mn[0], altura: mx[1] - mn[1], area: volume / t, volume, massa, dobras: dobras.len() },
        dobras,
        sobreposicoes,
        ausentes,
    })
}

fn entidade_dxf(e: &Entidade) -> fenkra_dxf::Entidade {
    use fenkra_dxf::{Entidade as D, Vertice};
    match e {
        Entidade::Linha(a, b) => D::Linha(*a, *b),
        Entidade::Circulo { centro, raio } => D::Circulo { centro: *centro, raio: *raio },
        Entidade::Arco { centro, raio, ini, fim } => D::Arco { centro: *centro, raio: *raio, ini: *ini, fim: *fim },
        Entidade::Polilinha(pts, fechada) => D::Polilinha { vertices: pts.iter().map(|p| Vertice { p: *p, bojo: 0. }).collect(), fechada: *fechada },
    }
}

/// DXF R2000 da planificada: 1:1, em milímetros, com camadas fixas (`CAMADA_*`). As entidades já saem
/// exatas de `chapa::chain_segs`; o que sobra amostrado é spline, em polilinha.
pub fn dxf(p: &Planificada) -> String {
    use fenkra_dxf::{Anexo, Camada, Desenho, Entidade as D, Item, TipoLinha, Unidade};
    let mut d = Desenho {
        unidade: Unidade::Milimetro,
        tipos_linha: vec![TipoLinha { nome: "DASHED".into(), padrao: vec![6., -3.] }],
        camadas: vec![
            Camada::nova(CAMADA_CONTORNO, 7, "Continuous"),
            Camada::nova(CAMADA_FUROS, 3, "Continuous"),
            Camada::nova(CAMADA_DOBRA_CIMA, 1, "DASHED"),
            Camada::nova(CAMADA_DOBRA_BAIXO, 5, "DASHED"),
            Camada::nova(CAMADA_FERRAMENTA, 6, "Continuous"),
            Camada::nova(CAMADA_ROTULOS, 2, "Continuous"),
        ],
        ..Default::default()
    };
    for (camada, lista) in [(CAMADA_CONTORNO, &p.contorno), (CAMADA_FUROS, &p.furos), (CAMADA_FERRAMENTA, &p.ferramenta)] {
        d.itens.extend(lista.iter().map(|e| Item::novo(camada, entidade_dxf(e))));
    }
    for dobra in &p.dobras {
        let camada = if dobra.cima { CAMADA_DOBRA_CIMA } else { CAMADA_DOBRA_BAIXO };
        if dobra.curva.len() > 2 {
            d.itens.extend(dobra.curva.windows(2).map(|w| Item::novo(camada, D::Linha(w[0], w[1]))));
        } else {
            d.itens.push(Item::novo(camada, D::Linha(dobra.a, dobra.b)));
        }
        let meio = [(dobra.a[0] + dobra.b[0]) / 2. + 1., (dobra.a[1] + dobra.b[1]) / 2. + 1.];
        let texto = format!("{} {}", dobra.ordem, dobra.rotulo);
        d.itens.push(Item::novo(CAMADA_ROTULOS, D::Texto { pos: meio, altura: 2.5, angulo: 0., anexo: Anexo::Esquerda, texto }));
    }
    fenkra_dxf::gravar(&d)
}

/// Grava o DXF da planificada em `caminho`. Recusa gravar se a planificada tem sobreposição ou
/// comando ausente e `exigir_valida` está ligado (o arquivo iria para a máquina errado).
pub fn exportar_dxf(doc: &Document, caminho: &str, exigir_valida: bool) -> Result<Planificada, String> {
    let p = planificar(doc)?;
    if exigir_valida && !p.valida() {
        return Err(p.problemas().join("; "));
    }
    std::fs::write(caminho, dxf(&p)).map_err(|e| format!("não foi possível gravar {caminho}: {e}"))?;
    Ok(p)
}
