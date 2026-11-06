// SPDX-License-Identifier: LGPL-2.1-or-later
//! Manipuladores 3D dos comandos de chapa (B249) e linha de dobra desenhada na face (B243).
//!
//! Esta parte é só dados e regras, sem egui, para os testes montarem o documento, recalcular e
//! conferir: `manipuladores` diz que alças o comando aberto tem e onde ficam; `aplicar` grava o valor
//! arrastado na expressão do campo *sem perder as variáveis* (`ang + 10` arrastado até 60 vira
//! `ang + 20`); `digitar` grava o texto digitado no rótulo. O desenho e o arrasto estão em
//! `chapa_manip_ui.rs`.

use fenkra_core::*;
use fenkra_solidos::chapa_linha::reescreve_expr;
use fenkra_solidos::geo3d::{self, V3};
use fenkra_solidos::RecomputeResult;
use std::collections::HashMap;

/// Campo de um comando de chapa que tem alça no 3D.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Campo {
    Angulo,
    Raio,
    /// recuo lateral no início e no fim da aresta
    Recuo1,
    Recuo2,
    /// largura da rampa (veneziana)
    Largura,
    /// desvio do joggle
    Desvio,
    /// rampa do joggle
    Rampa,
    /// folga da bainha
    Folga,
    /// folga entre as faces no fechamento de canto
    FolgaCanto,
}

impl Campo {
    pub fn rotulo(self) -> &'static str {
        match self {
            Campo::Angulo => "Ângulo",
            Campo::Raio => "Raio",
            Campo::Recuo1 => "Recuo do início",
            Campo::Recuo2 => "Recuo do fim",
            Campo::Largura => "Largura da rampa",
            Campo::Desvio => "Desvio",
            Campo::Rampa => "Rampa",
            Campo::Folga => "Folga da bainha",
            Campo::FolgaCanto => "Folga do canto",
        }
    }
    pub fn unidade(self) -> &'static str {
        if self == Campo::Angulo {
            "°"
        } else {
            "mm"
        }
    }
}

/// Forma da alça no 3D.
#[derive(Clone, Debug, PartialEq)]
pub enum Forma {
    /// ângulo: arco que parte de `ini` (direção a 0°) e gira em torno de `eixo` por `centro`
    Arco { centro: V3, eixo: V3, ini: V3, raio: f64, max: f64 },
    /// valor = comprimento ao longo de `dir` a partir de `base`, vezes `k`
    Seta { base: V3, dir: V3, k: f64 },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Manip {
    pub campo: Campo,
    pub expr: String,
    pub forma: Forma,
}

fn eval(e: &str, vars: &HashMap<String, f64>) -> Option<f64> {
    fenkra_core::expr::eval_str(e, vars).ok()
}

/// Expressão atual do campo (o raio sem valor próprio herda o da chapa).
pub fn expr_do_campo(doc: &Document, k: &FeatureKind, c: Campo) -> Option<String> {
    let chapa_r = || doc.sheet.as_ref().map(|s| s.radius.clone());
    match (k, c) {
        (FeatureKind::SmFlange(x), Campo::Angulo) => Some(x.angle.clone()),
        (FeatureKind::SmFlange(x), Campo::Raio) => x.radius.clone().or_else(chapa_r),
        (FeatureKind::SmFlange(x), Campo::Recuo1) => Some(x.off1.clone()),
        (FeatureKind::SmFlange(x), Campo::Recuo2) => Some(x.off2.clone()),
        (FeatureKind::SmHem(x), Campo::Folga) => Some(x.gap.clone()),
        (FeatureKind::SmCornerClose(x), Campo::FolgaCanto) => Some(x.gap.clone()),
        (FeatureKind::SmBend(x), Campo::Angulo) => Some(x.angle.clone()),
        (FeatureKind::SmBend(x), Campo::Raio) => x.radius.clone().or_else(chapa_r),
        (FeatureKind::SmJoggle(x), Campo::Desvio) => Some(x.depth.clone()),
        (FeatureKind::SmJoggle(x), Campo::Angulo) => x.angle.clone(),
        (FeatureKind::SmJoggle(x), Campo::Rampa) => x.angle.is_none().then(|| x.run.clone()),
        (FeatureKind::SmLouver(x), Campo::Largura) => Some(x.width.clone()),
        _ => None,
    }
}

/// Grava a expressão no campo. `false` se o comando não tem esse campo.
fn escreve(k: &mut FeatureKind, c: Campo, v: String) -> bool {
    match (k, c) {
        (FeatureKind::SmFlange(x), Campo::Angulo) => x.angle = v,
        (FeatureKind::SmFlange(x), Campo::Raio) => x.radius = Some(v),
        (FeatureKind::SmFlange(x), Campo::Recuo1) => x.off1 = v,
        (FeatureKind::SmFlange(x), Campo::Recuo2) => x.off2 = v,
        (FeatureKind::SmHem(x), Campo::Folga) => x.gap = v,
        (FeatureKind::SmCornerClose(x), Campo::FolgaCanto) => x.gap = v,
        (FeatureKind::SmBend(x), Campo::Angulo) => x.angle = v,
        (FeatureKind::SmBend(x), Campo::Raio) => x.radius = Some(v),
        (FeatureKind::SmJoggle(x), Campo::Desvio) => x.depth = v,
        (FeatureKind::SmJoggle(x), Campo::Angulo) if x.angle.is_some() => x.angle = Some(v),
        (FeatureKind::SmJoggle(x), Campo::Rampa) if x.angle.is_none() => x.run = v,
        (FeatureKind::SmLouver(x), Campo::Largura) => x.width = v,
        _ => return false,
    }
    true
}

/// Arrastou a alça até o valor `novo`: a expressão ganha o valor mantendo as variáveis. Devolve a
/// expressão gravada.
pub fn aplicar(doc: &mut Document, id: Id, c: Campo, novo: f64, vars: &HashMap<String, f64>) -> Option<String> {
    let atual_expr = expr_do_campo(doc, &doc.feature(id)?.kind, c)?;
    let atual = eval(&atual_expr, vars)?;
    let nova = reescreve_expr(&atual_expr, atual, novo);
    let f = doc.feature_mut(id)?;
    escreve(&mut f.kind, c, nova.clone()).then_some(nova)
}

/// Digitou o texto no rótulo 3D: vale como veio (aceita variáveis e expressões), se avalia.
pub fn digitar(doc: &mut Document, id: Id, c: Campo, texto: &str, vars: &HashMap<String, f64>) -> Result<(), String> {
    let t = texto.trim();
    if eval(t, vars).is_none() {
        return Err(format!("expressão inválida: {t}"));
    }
    let f = doc.feature_mut(id).ok_or("comando não encontrado")?;
    escreve(&mut f.kind, c, t.to_string()).then_some(()).ok_or_else(|| "o comando não tem esse campo".into())
}

/// Normal externa da face do corpo `alvo` mais perto do ponto `p`.
pub fn normal_perto(r: &RecomputeResult, alvo: Id, p: V3) -> Option<V3> {
    let b = r.bodies.iter().find(|b| b.collector == alvo)?;
    let (fi, _) = crate::picking::face_e_distancia(&b.mesh, p)?;
    let d = b.mesh.face_info.get(fi)?.dir;
    geo3d::unit(d)
}

/// Alças do comando `id`, com a geometria já calculada. Vazio se o comando não tem alça de chapa
/// ou a geometria ainda não existe (comando com erro, linha por definir).
pub fn manipuladores(doc: &Document, r: &RecomputeResult, id: Id, vars: &HashMap<String, f64>) -> Vec<Manip> {
    let Some(f) = doc.feature(id) else { return vec![] };
    let t = doc.sheet.as_ref().and_then(|s| eval(&s.thickness, vars)).unwrap_or(2.);
    let ex = |c: Campo| expr_do_campo(doc, &f.kind, c);
    let mut out = vec![];
    // eixo da dobra do comando, no espaço dobrado
    let eixo = r.bend_lines.iter().find(|b| b.feature == id).and_then(|b| b.eixo.clone().map(|e| (e, b.radius)));
    match &f.kind {
        FeatureKind::SmFlange(_) | FeatureKind::SmBend(_) | FeatureKind::SmHem(_) => {
            let Some((e, raio)) = eixo else { return out };
            let Some(ev) = geo3d::unit(geo3d::sub(e.b, e.a)) else { return out };
            let meio = geo3d::mul(geo3d::sub(e.b, e.a), 0.5);
            let centro = geo3d::add(e.centro, meio);
            let n = e.normal;
            if matches!(f.kind, FeatureKind::SmHem(_)) {
                // a folga é o diâmetro interno da dobra: a seta vai do centro até a face interna
                if let Some(x) = ex(Campo::Folga) {
                    out.push(Manip { campo: Campo::Folga, expr: x, forma: Forma::Seta { base: centro, dir: geo3d::mul(n, -1.), k: 2. } });
                }
                return out;
            }
            if let (Some(x), Some(axis)) = (ex(Campo::Angulo), geo3d::unit(fenkra_kernel::cross(e.saida, n))) {
                out.push(Manip { campo: Campo::Angulo, expr: x, forma: Forma::Arco { centro, eixo: axis, ini: geo3d::mul(n, -1.), raio: raio + t + 6., max: 179.9 } });
            }
            if let Some(x) = ex(Campo::Raio) {
                out.push(Manip { campo: Campo::Raio, expr: x, forma: Forma::Seta { base: centro, dir: geo3d::mul(n, -1.), k: 1. } });
            }
            if matches!(f.kind, FeatureKind::SmFlange(_)) {
                if let Some(x) = ex(Campo::Recuo1) {
                    out.push(Manip { campo: Campo::Recuo1, expr: x, forma: Forma::Seta { base: e.a, dir: ev, k: 1. } });
                }
                if let Some(x) = ex(Campo::Recuo2) {
                    out.push(Manip { campo: Campo::Recuo2, expr: x, forma: Forma::Seta { base: e.b, dir: geo3d::mul(ev, -1.), k: 1. } });
                }
            }
        }
        FeatureKind::SmJoggle(j) => {
            if geo3d::dist(j.a, j.b) < 1e-6 {
                return out;
            }
            let Some(n) = normal_perto(r, f.target, geo3d::lerp(j.a, j.b, 0.5)) else { return out };
            let Some(side) = fenkra_solidos::chapa_linha::lado_movel(j.a, j.b, n, j.flip) else { return out };
            let mid = geo3d::lerp(j.a, j.b, 0.5);
            if let Some(x) = ex(Campo::Desvio) {
                out.push(Manip { campo: Campo::Desvio, expr: x, forma: Forma::Seta { base: mid, dir: geo3d::mul(n, -1.), k: 1. } });
            }
            if let (Some(x), Some(axis)) = (ex(Campo::Angulo), geo3d::unit(fenkra_kernel::cross(side, geo3d::mul(n, -1.)))) {
                out.push(Manip { campo: Campo::Angulo, expr: x, forma: Forma::Arco { centro: mid, eixo: axis, ini: side, raio: 12., max: 89. } });
            }
            if let Some(x) = ex(Campo::Rampa) {
                out.push(Manip { campo: Campo::Rampa, expr: x, forma: Forma::Seta { base: mid, dir: side, k: 1. } });
            }
        }
        FeatureKind::SmCornerClose(cc) => {
            // as abas de 90° do documento, no espaço dobrado, e o canto mais perto do ponto escolhido
            let abas: Vec<_> = r.bend_lines.iter().filter_map(|b| fenkra_solidos::chapa_canto::aba_da_linha(b, t)).collect();
            let at = cc.at;
            if let (Some(x), Some((base, dir, k))) = (ex(Campo::FolgaCanto), fenkra_solidos::chapa_canto::alca_da_folga(&abas, at, cc.kind, cc.invert)) {
                out.push(Manip { campo: Campo::FolgaCanto, expr: x, forma: Forma::Seta { base, dir, k } });
            }
        }
        FeatureKind::SmLouver(l) => {
            if geo3d::dist(l.a, l.b) < 1e-6 {
                return out;
            }
            let Some(n) = normal_perto(r, f.target, geo3d::lerp(l.a, l.b, 0.5)) else { return out };
            let Some(side) = fenkra_solidos::chapa_linha::lado_movel(l.a, l.b, n, l.flip) else { return out };
            if let Some(x) = ex(Campo::Largura) {
                out.push(Manip { campo: Campo::Largura, expr: x, forma: Forma::Seta { base: geo3d::lerp(l.a, l.b, 0.5), dir: side, k: 1. } });
            }
        }
        _ => {}
    }
    out
}

/// Ângulo (graus) de `Forma::Arco` cujo ponto fica mais perto de `alvo` na tela. `projeta` leva um
/// ponto 3D à tela; devolve o ângulo de 0 a `max` com resolução de 0,5°.
pub fn angulo_mais_perto(forma: &Forma, alvo: [f32; 2], projeta: &dyn Fn(V3) -> Option<[f32; 2]>) -> Option<f64> {
    let Forma::Arco { centro, eixo, ini, raio, max } = forma else { return None };
    let mut melhor: Option<(f32, f64)> = None;
    let passos = (*max * 2.) as i32;
    for k in 0..=passos {
        let a = k as f64 / 2.;
        let p = geo3d::add(*centro, geo3d::mul(geo3d::rotate(*ini, *eixo, a.to_radians()), *raio));
        let Some(s) = projeta(p) else { continue };
        let d = ((s[0] - alvo[0]).powi(2) + (s[1] - alvo[1]).powi(2)).sqrt();
        if melhor.is_none_or(|(bd, _)| d < bd) {
            melhor = Some((d, a));
        }
    }
    melhor.map(|(_, a)| a)
}

/// Ponto do arco no ângulo `a` (graus).
pub fn ponto_do_arco(forma: &Forma, a: f64) -> Option<V3> {
    let Forma::Arco { centro, eixo, ini, raio, .. } = forma else { return None };
    Some(geo3d::add(*centro, geo3d::mul(geo3d::rotate(*ini, *eixo, a.to_radians()), *raio)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::historico::Historico;
    use fenkra_core::sketch::Curve;

    fn s(x: &str) -> String {
        x.into()
    }

    fn rect(d: &mut Document, a: [f64; 2], b: [f64; 2]) -> Id {
        let mut data = SketchData::default();
        let mut n = 9_000_000u64 + d.features.len() as u64 * 1000;
        let mut al = || {
            n += 1;
            Id(n)
        };
        let ids: Vec<Id> = [[a[0], a[1]], [b[0], a[1]], [b[0], b[1]], [a[0], b[1]]].iter().map(|p| data.add_point(&mut al, p[0], p[1])).collect();
        for i in 0..4 {
            data.add_curve(&mut al, Curve::Line { a: ids[i], b: ids[(i + 1) % 4] }, false);
        }
        d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }))
    }

    fn base() -> Document {
        let mut d = Document::new("chapa");
        d.sheet = Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), flat: false, ..Default::default() });
        let sk = rect(&mut d, [0., 0.], [100., 50.]);
        d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }));
        d
    }

    fn aba(d: &mut Document, ang: &str) -> Id {
        d.add_feature(FeatureKind::SmFlange(SmFlange { edge: [100., 25., 2.], length: s("20"), angle: s(ang), radius: None, off1: s("5"), off2: s("5"), flip: false, relief: None, relief_kind: 0, side: 0, miter: false, min_gap: true, edge_key: None }))
    }

    fn campos(m: &[Manip]) -> Vec<Campo> {
        m.iter().map(|x| x.campo).collect()
    }

    /// Aba, bainha, dobra por linha, joggle e veneziana: cada um mostra as alças do seu tipo, na
    /// geometria certa.
    #[test]
    fn cada_comando_de_chapa_tem_as_suas_alcas() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let mut d = base();
        let a = aba(&mut d, "90");
        let h = d.add_feature(FeatureKind::SmHem(SmHem { edge: [0., 25., 2.], length: s("8"), gap: s("2"), flip: false, kind: 1, edge_key: None }));
        let b = d.add_feature(FeatureKind::SmBend(SmBend { a: [60., 0., 2.], b: [60., 50., 2.], angle: s("45"), radius: None, flip: true, down: false, sketch: None, position: 0 }));
        let r = recompute_ok(&d);
        let v = r.params.clone();

        let ma = manipuladores(&d, &r, a, &v);
        assert_eq!(campos(&ma), vec![Campo::Angulo, Campo::Raio, Campo::Recuo1, Campo::Recuo2]);
        // arco da aba: centro no eixo da dobra (x = 100, z = 2 + raio 2 = 4), no meio da aresta (y = 25)
        let Forma::Arco { centro, .. } = &ma[0].forma else { panic!("ângulo é arco") };
        assert!(geo3d::dist(*centro, [100., 25., 4.]) < 0.05, "centro do arco {centro:?}");
        // raio: seta do centro da dobra para baixo, com o raio da chapa herdado (2)
        assert_eq!(ma[1].expr, "2");
        // recuos: as setas saem das duas pontas da aresta, uma para cada lado
        let (Forma::Seta { base: b1, dir: d1, .. }, Forma::Seta { base: b2, dir: d2, .. }) = (&ma[2].forma, &ma[3].forma) else { panic!() };
        assert!((b1[1] - b2[1]).abs() > 39. && geo3d::dot(*d1, *d2) < -0.99, "{b1:?} {b2:?}");

        let mh = manipuladores(&d, &r, h, &v);
        assert_eq!(campos(&mh), vec![Campo::Folga], "bainha");
        let mb = manipuladores(&d, &r, b, &v);
        assert_eq!(campos(&mb), vec![Campo::Angulo, Campo::Raio], "dobra por linha");

        let mut d2 = base();
        let j = d2.add_feature(FeatureKind::SmJoggle(SmJoggle { a: [60., 0., 2.], b: [60., 50., 2.], depth: s("3"), run: s("6"), flip: true, angle: None, plane: None, keep_length: false }));
        let l = d2.add_feature(FeatureKind::SmLouver(SmLouver { a: [30., 10., 2.], b: [30., 40., 2.], width: s("12"), height: s("5"), flip: false, down: false }));
        let r2 = recompute_ok(&d2);
        let v2 = r2.params.clone();
        let mj = manipuladores(&d2, &r2, j, &v2);
        assert_eq!(campos(&mj), vec![Campo::Desvio, Campo::Rampa], "joggle pela rampa");
        // o desvio desce pela normal da face (-z) a partir do meio da linha
        let Forma::Seta { base, dir, .. } = &mj[0].forma else { panic!() };
        assert!(geo3d::dist(*base, [60., 25., 2.]) < 1e-6 && (dir[2] + 1.).abs() < 1e-9, "{base:?} {dir:?}");
        let ml = manipuladores(&d2, &r2, l, &v2);
        assert_eq!(campos(&ml), vec![Campo::Largura], "veneziana");
        // joggle pelo ângulo troca a rampa pelo arco
        if let Some(f) = d2.feature_mut(j) {
            if let FeatureKind::SmJoggle(x) = &mut f.kind {
                x.angle = Some(s("45"));
            }
        }
        let r3 = recompute_ok(&d2);
        assert_eq!(campos(&manipuladores(&d2, &r3, j, &r3.params.clone())), vec![Campo::Desvio, Campo::Angulo]);
    }

    fn recompute_ok(d: &Document) -> RecomputeResult {
        let r = fenkra_solidos::recompute(d);
        for f in &d.features {
            assert!(matches!(r.status[&f.id], fenkra_solidos::Status::Ok), "{}: {:?}", f.name, r.status[&f.id]);
        }
        r
    }

    /// Arrastar muda a expressão do campo sem perder a variável; desfazer volta um passo.
    #[test]
    fn arrastar_preserva_variaveis_e_desfazer_volta_um_passo() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let mut d = base();
        d.add_param("ang_base", "40", Unit::Angulo).unwrap_or_else(|e| panic!("{e:?}"));
        let a = aba(&mut d, "ang_base + 10");
        let r = recompute_ok(&d);
        let vars = r.params.clone();
        assert!((vars["ang_base"] - 40.).abs() < 1e-9);

        let antes = d.clone();
        let nova = aplicar(&mut d, a, Campo::Angulo, 60., &vars).unwrap();
        assert_eq!(nova, "ang_base + 20", "a variável fica, só o termo numérico muda");
        let r2 = recompute_ok(&d);
        let FeatureKind::SmFlange(fl) = &d.feature(a).unwrap().kind else { panic!() };
        assert!((eval(&fl.angle, &r2.params).unwrap() - 60.).abs() < 1e-9);
        // o parâmetro não foi tocado
        assert_eq!(d.params.iter().find(|p| p.name == "ang_base").unwrap().expr, "40");
        // a peça mudou de verdade: a aba a 60° é mais alta que a 50°
        let zmax = |r: &RecomputeResult| r.bodies[0].mesh.positions.chunks_exact(3).map(|p| p[2]).fold(f32::MIN, f32::max);
        assert!(zmax(&r2) > zmax(&r) + 1., "{} deveria ser maior que {}", zmax(&r2), zmax(&r));

        // desfazer: o histórico devolve o documento de antes, com a expressão original
        let mut h = Historico::new(16);
        h.registrar(antes, &d);
        let (volta, _) = h.desfazer(d.clone()).expect("um passo para desfazer");
        let FeatureKind::SmFlange(fl) = &volta.feature(a).unwrap().kind else { panic!() };
        assert_eq!(fl.angle, "ang_base + 10");

        // raio sem valor próprio nasce da expressão da chapa; digitar aceita variável e rejeita lixo
        aplicar(&mut d, a, Campo::Raio, 3., &vars).unwrap();
        let FeatureKind::SmFlange(fl) = &d.feature(a).unwrap().kind else { panic!() };
        assert_eq!(fl.radius.as_deref(), Some("3"));
        assert!(digitar(&mut d, a, Campo::Recuo1, "ang_base / 8", &vars).is_ok());
        assert!(digitar(&mut d, a, Campo::Recuo1, "ang_x +", &vars).is_err());
        // campo que o comando não tem
        assert!(aplicar(&mut d, a, Campo::Largura, 4., &vars).is_none());
    }

    /// B293: o fechamento de canto tem a alça da folga nos três tipos. A seta sai da face que não
    /// anda e a ponta cai na face da peça que fecha: arrastar a folga recalcula o canto e o volume
    /// que fecha o vão diminui (a peça recua), sem perder a variável.
    #[test]
    fn fechamento_de_canto_tem_a_alca_da_folga() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        for tipo in [0u8, 1, 2] {
            let mut d = base();
            d.add_param("folga_c", "0.5", Unit::Comprimento).unwrap_or_else(|e| panic!("{e:?}"));
            for e in [[100., 25., 2.], [50., 50., 2.]] {
                d.add_feature(FeatureKind::SmFlange(SmFlange { edge: e, length: s("20"), angle: s("90"), radius: None, off1: s("0"), off2: s("0"), flip: false, relief: None, relief_kind: 0, side: 0, miter: false, min_gap: true, edge_key: None }));
            }
            let id = d.add_feature(FeatureKind::SmCornerClose(SmCornerClose { at: [100., 50., 2.], kind: tipo, gap: s("folga_c"), ratio: s("1"), narrow: false, propagate: false, invert: false }));
            let r = recompute_ok(&d);
            let vars = r.params.clone();
            let m = manipuladores(&d, &r, id, &vars);
            assert_eq!(campos(&m), vec![Campo::FolgaCanto], "tipo {tipo}");
            let Forma::Seta { base, dir, k } = &m[0].forma else { panic!("a folga é seta") };
            assert!((geo3d::norm(*dir) - 1.).abs() < 1e-9, "direção unitária");
            // a base fica junto ao canto (100, 50), do lado de fora do vão
            assert!(geo3d::dist(*base, [100., 50., base[2]]) < 6., "tipo {tipo}: base {base:?}");
            let antes = r.bodies[0].mass.volume;
            let nova = aplicar(&mut d, id, Campo::FolgaCanto, 2., &vars).unwrap();
            assert_eq!(nova, "folga_c + 1.5", "tipo {tipo}: a variável fica e só o termo numérico muda");
            let r2 = recompute_ok(&d);
            assert!((r2.params["folga_c"] - 0.5).abs() < 1e-9, "o parâmetro não foi tocado");
            assert!(r2.bodies[0].mass.volume < antes - 1e-3, "tipo {tipo}: folga maior recua as peças ({} contra {antes})", r2.bodies[0].mass.volume);
            // a ponta da seta, com a folga nova, fica a 2 mm da base na direção da alça
            let m2 = manipuladores(&d, &r2, id, &r2.params.clone());
            let Forma::Seta { base: b2, dir: d2, k: k2 } = &m2[0].forma else { panic!() };
            assert!(geo3d::dist(*base, *b2) < 1e-6 && geo3d::dot(*dir, *d2) > 0.999 && (k - k2).abs() < 1e-9, "a seta não muda de lugar quando a folga muda");
        }
    }

    /// O arco do ângulo: o ponto sob o mouse dá o ângulo, e o ponto do arco volta ao mesmo lugar.
    #[test]
    fn arco_devolve_o_angulo_do_ponto_sob_o_mouse() {
        let forma = Forma::Arco { centro: [0., 0., 0.], eixo: [0., 0., 1.], ini: [1., 0., 0.], raio: 10., max: 179.9 };
        // vista de cima: x e y viram a tela
        let proj = |p: V3| Some([p[0] as f32 * 4., p[1] as f32 * 4.]);
        let alvo = ponto_do_arco(&forma, 63.).unwrap();
        let a = angulo_mais_perto(&forma, [alvo[0] as f32 * 4., alvo[1] as f32 * 4.], &proj).unwrap();
        assert!((a - 63.).abs() <= 0.5, "{a}");
    }
}
