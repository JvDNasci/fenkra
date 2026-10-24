//! Automação: macros em texto que criam e editam a peça. Uma instrução por linha; `#` comenta.
//! Os números aceitam expressões com os parâmetros da peça e as variáveis dos laços
//! (escreva sem espaços ou entre parênteses: `(L/2)`). Palavras em português ou inglês.
//!
//! ```text
//! param L = 120                      # parâmetro global (cria ou altera)
//! plano P1 deslocado XY 20           # plane NAME offset BASE DIST
//! sketch S1 em XY                    # sketch NAME on PLANE … fim / end
//!   retangulo -L/2 -30 L/2 30        # rect x1 y1 x2 y2
//!   circulo 0 0 8                    # circle cx cy r
//!   linha 0 0 10 10                  # line x1 y1 x2 y2
//!   ponto 20 0                       # point x y (centro de furo)
//! fim
//! extrusao E1 S1 10                  # extrude NAME SKETCH DIST
//! recorte C1 S2 5                    # cut NAME SKETCH DIST   (ou "passante")
//! furo F1 S3 6.6 passante            # hole NAME SKETCH DIAM DEPTH|passante
//! filete todas 2                     # fillet all R
//! chanfro todas 1                    # chamfer all D
//! padrao linear PL1 C1 X 4 20        # pattern linear NAME FEATURE AXIS COUNT SPACING
//! padrao circular PC1 C1 Z 6 360     # pattern circular NAME FEATURE AXIS COUNT ANGLE
//! espelho M1 C1 YZ                   # mirror NAME FEATURE PLANE
//! para i de 1 a 4                    # for i from 1 to 4 … fim
//!   circulo (i*20) 0 3
//! fim
//! ```

use crate::sketch::SketchData;
use crate::*;
use std::collections::HashMap;

/// Resultado: linhas de registro, ou o erro com o número da linha.
pub fn run(doc: &mut Document, src: &str) -> Result<Vec<String>, String> {
    let lines: Vec<(usize, Vec<String>)> = src
        .lines()
        .enumerate()
        .map(|(i, l)| (i + 1, l.split('#').next().unwrap_or("").split_whitespace().map(str::to_string).collect::<Vec<_>>()))
        .filter(|(_, t)| !t.is_empty())
        .collect();
    let mut log = vec![];
    let mut loops = HashMap::new();
    exec(doc, &lines, &mut loops, &mut log)?;
    Ok(log)
}

fn kw(t: &str, pt: &[&str]) -> bool {
    pt.iter().any(|k| t.eq_ignore_ascii_case(k))
}

/// Fim do bloco que começa em `start` (linha com `sketch` ou `para`), respeitando aninhamento.
fn block_end(lines: &[(usize, Vec<String>)], start: usize) -> Result<usize, String> {
    let mut depth = 0;
    for (k, (_, t)) in lines.iter().enumerate().skip(start) {
        if kw(&t[0], &["sketch", "para", "for"]) {
            depth += 1;
        } else if kw(&t[0], &["fim", "end"]) {
            depth -= 1;
            if depth == 0 {
                return Ok(k);
            }
        }
    }
    Err(format!("linha {}: bloco sem 'fim'", lines[start].0))
}

fn exec(doc: &mut Document, lines: &[(usize, Vec<String>)], loops: &mut HashMap<String, f64>, log: &mut Vec<String>) -> Result<(), String> {
    let mut k = 0;
    while k < lines.len() {
        let (ln, t) = &lines[k];
        let err = |m: String| format!("linha {ln}: {m}");
        let vars = || -> HashMap<String, f64> {
            let mut v = doc.eval_params().unwrap_or_default();
            v.extend(loops.iter().map(|(a, b)| (a.clone(), *b)));
            v
        };
        let num = |s: &str| -> Result<f64, String> { expr::eval_str(s, &vars()).map_err(|e| err(format!("'{s}': {e}"))) };
        let need = |n: usize| if t.len() < n { Err(err(format!("faltam argumentos em '{}'", t.join(" ")))) } else { Ok(()) };
        let head = t[0].to_lowercase();
        match head.as_str() {
            "param" | "parametro" | "parâmetro" => {
                need(4)?;
                let name = &t[1];
                let e = t[3..].join(" ");
                if let Some(p) = doc.params.iter_mut().find(|p| &p.name == name) {
                    p.expr = e;
                } else {
                    doc.add_param(name, &e, Unit::Comprimento).map_err(|x| err(x.to_string()))?;
                }
                log.push(format!("ƒ {name} = {}", t[3..].join(" ")));
            }
            "plano" | "plane" => {
                need(5)?;
                let base = plane_ref(doc, &t[3]).ok_or_else(|| err(format!("plano '{}' não existe", t[3])))?;
                let id = doc.add_offset_plane(base, &t[4]);
                rename_ref(doc, id, &t[1]);
                log.push(format!("▱ {}", t[1]));
            }
            "sketch" | "esboco" | "esboço" => {
                need(4)?;
                let plane = plane_ref(doc, &t[3]).ok_or_else(|| err(format!("plano '{}' não existe", t[3])))?;
                let end = block_end(lines, k)?;
                let mut data = SketchData::default();
                sketch_body(doc, &mut data, &lines[k + 1..end], loops)?;
                let id = doc.add_feature(FeatureKind::Sketch(Sketch { plane, data }));
                rename_feature(doc, id, &t[1]);
                log.push(format!("✏ {}", t[1]));
                k = end;
            }
            "extrusao" | "extrusão" | "extrude" | "recorte" | "cut" => {
                need(4)?;
                let sk = feature_id(doc, &t[2]).ok_or_else(|| err(format!("sketch '{}' não existe", t[2])))?;
                let cut = kw(&head, &["recorte", "cut"]);
                let extent = if kw(&t[3], &["passante", "through"]) { Extent::ThroughAll } else { Extent::Blind(t[3].clone()) };
                num_or_through(&t[3], &num)?;
                let id = doc.add_feature(FeatureKind::Extrude(Extrude {
                    sketch: sk,
                    op: if cut { BoolOp::Cut } else { BoolOp::Add },
                    extent,
                    reverse: false,
                    taper: "0".into(),
                    thin: None, dir: None,
                }));
                rename_feature(doc, id, &t[1]);
                log.push(format!("{} {}", if cut { "⬇" } else { "⬆" }, t[1]));
            }
            "furo" | "hole" => {
                need(5)?;
                let sk = feature_id(doc, &t[2]).ok_or_else(|| err(format!("sketch '{}' não existe", t[2])))?;
                num(&t[3])?;
                num_or_through(&t[4], &num)?;
                let depth = if kw(&t[4], &["passante", "through"]) { Extent::ThroughAll } else { Extent::Blind(t[4].clone()) };
                let id = doc.add_feature(FeatureKind::Hole(Hole { sketch: sk, kind: HoleKind::Simples, diameter: t[3].clone(), depth, thread: None, thread_modeled: false, standard: None, points: vec![], dir: None, reverse: false, tip: None, taper: "0".into(), left_hand: false }));
                rename_feature(doc, id, &t[1]);
                log.push(format!("◎ {}", t[1]));
            }
            "filete" | "fillet" | "chanfro" | "chamfer" => {
                need(3)?;
                num(&t[2])?;
                let op = EdgeOp { value: t[2].clone(), edges: EdgeSel::AllOfBody, mode: EdgeMode::Constant, prop: Default::default() };
                let fillet = kw(&head, &["filete", "fillet"]);
                doc.add_feature(if fillet { FeatureKind::Fillet(op) } else { FeatureKind::Chamfer(op) });
                log.push(format!("{} {}", if fillet { "◜" } else { "◸" }, t[2]));
            }
            "padrao" | "padrão" | "pattern" => {
                need(7)?;
                let src = feature_id(doc, &t[3]).ok_or_else(|| err(format!("comando '{}' não existe", t[3])))?;
                let axis = axis_of(&t[4]).ok_or_else(|| err(format!("eixo '{}' (use X, Y ou Z)", t[4])))?;
                num(&t[5])?;
                num(&t[6])?;
                let kind = if kw(&t[1], &["circular"]) {
                    PatternKind::Circular { axis, count: t[5].clone(), angle: t[6].clone() }
                } else {
                    PatternKind::Linear { dir: axis, count: t[5].clone(), spacing: t[6].clone(), dir2: None, count2: "1".into(), spacing2: "0".into(), total: false }
                };
                let id = doc.add_feature(FeatureKind::Pattern(Pattern { kind, sources: vec![src] }));
                rename_feature(doc, id, &t[2]);
                log.push(format!("⁂ {}", t[2]));
            }
            "espelho" | "mirror" => {
                need(4)?;
                let src = feature_id(doc, &t[2]).ok_or_else(|| err(format!("comando '{}' não existe", t[2])))?;
                let plane = plane_ref(doc, &t[3]).ok_or_else(|| err(format!("plano '{}' não existe", t[3])))?;
                let id = doc.add_feature(FeatureKind::Mirror(Mirror { plane, sources: vec![src] }));
                rename_feature(doc, id, &t[1]);
                log.push(format!("⇋ {}", t[1]));
            }
            "para" | "for" => {
                // para i de A a B   |   for i from A to B
                need(6)?;
                let var = t[1].clone();
                let (a, b) = (num(&t[3])?.round() as i64, num(&t[5])?.round() as i64);
                let end = block_end(lines, k)?;
                if (b - a).abs() > 10_000 {
                    return Err(err("laço grande demais".into()));
                }
                let step = if b >= a { 1 } else { -1 };
                let mut i = a;
                loop {
                    loops.insert(var.clone(), i as f64);
                    exec(doc, &lines[k + 1..end], loops, log)?;
                    if i == b {
                        break;
                    }
                    i += step;
                }
                loops.remove(&var);
                k = end;
            }
            "fim" | "end" => {}
            _ => return Err(err(format!("instrução desconhecida '{}'", t[0]))),
        }
        k += 1;
    }
    Ok(())
}

fn num_or_through(s: &str, num: &dyn Fn(&str) -> Result<f64, String>) -> Result<(), String> {
    if kw(s, &["passante", "through"]) {
        Ok(())
    } else {
        num(s).map(|_| ())
    }
}

/// Linhas de dentro de `sketch … fim`, com laços.
fn sketch_body(doc: &mut Document, data: &mut SketchData, lines: &[(usize, Vec<String>)], loops: &mut HashMap<String, f64>) -> Result<(), String> {
    let mut k = 0;
    while k < lines.len() {
        let (ln, t) = &lines[k];
        let err = |m: String| format!("linha {ln}: {m}");
        let mut vars = doc.eval_params().unwrap_or_default();
        vars.extend(loops.iter().map(|(a, b)| (a.clone(), *b)));
        let n = |i: usize| -> Result<f64, String> {
            let s = t.get(i).ok_or_else(|| err(format!("faltam números em '{}'", t.join(" "))))?;
            expr::eval_str(s, &vars).map_err(|e| err(format!("'{s}': {e}")))
        };
        let head = t[0].to_lowercase();
        let mut al = || doc.alloc();
        match head.as_str() {
            "retangulo" | "retângulo" | "rect" | "rectangle" => {
                let (a, b) = ([n(1)?, n(2)?], [n(3)?, n(4)?]);
                data.add_rect(&mut al, a, b, false);
            }
            "circulo" | "círculo" | "circle" => {
                let (cx, cy, r) = (n(1)?, n(2)?, n(3)?);
                let c = data.add_point(&mut al, cx, cy);
                data.add_curve(&mut al, crate::sketch::Curve::Circle { c, r }, false);
            }
            "linha" | "line" => {
                let (x1, y1, x2, y2) = (n(1)?, n(2)?, n(3)?, n(4)?);
                let a = data.add_point(&mut al, x1, y1);
                let b = data.add_point(&mut al, x2, y2);
                data.add_curve(&mut al, crate::sketch::Curve::Line { a, b }, false);
            }
            "ponto" | "point" => {
                let (x, y) = (n(1)?, n(2)?);
                let p = data.add_point(&mut al, x, y);
                data.add_curve(&mut al, crate::sketch::Curve::Point { p }, false);
            }
            "para" | "for" => {
                if t.len() < 6 {
                    return Err(err("use: para i de A a B".into()));
                }
                let var = t[1].clone();
                let (a, b) = (n(3)?.round() as i64, n(5)?.round() as i64);
                let end = block_end(lines, k)?;
                let step = if b >= a { 1 } else { -1 };
                let mut i = a;
                loop {
                    loops.insert(var.clone(), i as f64);
                    sketch_body(doc, data, &lines[k + 1..end], loops)?;
                    if i == b {
                        break;
                    }
                    i += step;
                }
                loops.remove(&var);
                k = end;
            }
            "fim" | "end" => {}
            _ => return Err(err(format!("dentro do sketch use retangulo, circulo, linha, ponto ou para (achei '{}')", t[0]))),
        }
        k += 1;
    }
    Ok(())
}

fn plane_ref(doc: &Document, s: &str) -> Option<PlaneRef> {
    match s.to_uppercase().as_str() {
        "XY" => Some(PlaneRef::Origin(OriginPlane::XY)),
        "XZ" => Some(PlaneRef::Origin(OriginPlane::XZ)),
        "YZ" => Some(PlaneRef::Origin(OriginPlane::YZ)),
        _ => doc.references.iter().find(|r| r.name == s && r.kind.class() == RefClass::Plane).map(|r| PlaneRef::Datum(r.id)),
    }
}

fn axis_of(s: &str) -> Option<AxisRef> {
    match s.to_uppercase().as_str() {
        "X" => Some(AxisRef::Origin(OriginAxis::X)),
        "Y" => Some(AxisRef::Origin(OriginAxis::Y)),
        "Z" => Some(AxisRef::Origin(OriginAxis::Z)),
        _ => None,
    }
}

fn feature_id(doc: &Document, name: &str) -> Option<Id> {
    doc.features.iter().find(|f| f.name == name).map(|f| f.id)
}

fn rename_feature(doc: &mut Document, id: Id, name: &str) {
    if name == "_" || doc.name_taken(name) {
        return;
    }
    if let Some(f) = doc.feature_mut(id) {
        f.name = name.into();
    }
}

fn rename_ref(doc: &mut Document, id: Id, name: &str) {
    if name == "_" || doc.name_taken(name) {
        return;
    }
    if let Some(r) = doc.references.iter_mut().find(|r| r.id == id) {
        r.name = name.into();
    }
}

/// Macro de exemplo (placa com furos em padrão).
pub const EXAMPLE: &str = "# Placa com furos — edite e execute
param L = 120
param W = 70
param esp = 8
sketch Base em XY
  retangulo (-L/2) (-W/2) (L/2) (W/2)
fim
extrusao Placa Base esp
sketch Furos em XY
  para i de 0 a 3
    circulo (-45+i*30) 0 4
  fim
fim
recorte Rasgos Furos passante
filete todas 1
";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn macro_de_exemplo_cria_a_peca() {
        let mut d = Document::new("m");
        let log = run(&mut d, EXAMPLE).unwrap();
        assert!(log.len() >= 6, "{log:?}");
        assert!(d.params.iter().any(|p| p.name == "L"));
        let furos = d.features.iter().find(|f| f.name == "Furos").unwrap();
        let FeatureKind::Sketch(s) = &furos.kind else { panic!() };
        assert_eq!(s.data.curves.len(), 4);
        assert!(d.features.iter().any(|f| f.name == "Placa"));
        let e = run(&mut d, "extrusao X NaoExiste 5").unwrap_err();
        assert!(e.contains("linha 1"));
    }
}
