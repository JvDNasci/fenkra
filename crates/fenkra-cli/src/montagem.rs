// SPDX-License-Identifier: LGPL-2.1-or-later
//! Montagem pela sessão (B420): peças salvas na pasta entram como componentes, ganham restrições e
//! posição, e a lista de peças sai da mesma montagem que o aplicativo abre (`.fka`).
//!
//! O agente não tem clique: cada elemento de uma restrição é dado por um ponto no mundo sobre a face
//! (plana vira plano, cilíndrica vira eixo), por um plano ou eixo da origem da peça, ou por um ponto.
//! A peça de cada componente é lida e calculada uma vez por arquivo e fica guardada fora do histórico.

use crate::{r6, r6v, ErroApi};
use fenkra_core::Id;
use fenkra_kernel::Shape;
use fenkra_montagem::bom::{bom, BomLevel};
use fenkra_montagem::load::geom_at;
use fenkra_montagem::solve::{solve, SolveReport};
use fenkra_montagem::view::posicionado;
use fenkra_montagem::{unit, Assembly, Geom, MateKind, MateRef, NodeKind, Placement, Sentido, V3};
use serde_json::{json, Value};
use std::collections::HashMap;

/// A montagem em edição e os nomes que o agente deu a componentes e restrições.
#[derive(Clone)]
pub(crate) struct Montagem {
    pub asm: Assembly,
    pub ids: Vec<(String, Id)>,
}

/// Forma de cada arquivo de peça, no sistema da própria peça.
#[derive(Default)]
pub(crate) struct Formas(pub HashMap<String, Shape>);

fn xyz(v: &Value, chave: &str) -> Result<V3, ErroApi> {
    let l = v.as_array().filter(|l| l.len() == 3).ok_or_else(|| ErroApi::argumento(format!("`{chave}` é [x, y, z]")))?;
    let mut p = [0.; 3];
    for (i, x) in l.iter().enumerate() {
        p[i] = x.as_f64().ok_or_else(|| ErroApi::argumento(format!("`{chave}` é [x, y, z] em números")))?;
    }
    Ok(p)
}

fn valor(v: &Value, chave: &str) -> Result<String, ErroApi> {
    match v {
        Value::String(s) if !s.trim().is_empty() => Ok(s.clone()),
        Value::Number(n) => Ok(n.to_string()),
        _ => Err(ErroApi::argumento(format!("falta `{chave}` (número ou expressão)"))),
    }
}

fn eixo_origem(nome: &str) -> Option<V3> {
    match nome {
        "X" | "x" => Some([1., 0., 0.]),
        "Y" | "y" => Some([0., 1., 0.]),
        "Z" | "z" => Some([0., 0., 1.]),
        _ => None,
    }
}

fn normal_do_plano(nome: &str) -> Option<V3> {
    match nome {
        "XY" => Some([0., 0., 1.]),
        "XZ" => Some([0., -1., 0.]),
        "YZ" => Some([1., 0., 0.]),
        _ => None,
    }
}

fn rotacao(v: &Value) -> Result<[[f64; 3]; 3], ErroApi> {
    let l = v.as_array().filter(|l| l.len() == 3).ok_or_else(|| ErroApi::argumento("`rotacao` é a matriz 3 × 3, linha a linha"))?;
    let mut r = [[0.; 3]; 3];
    for (i, linha) in l.iter().enumerate() {
        r[i] = xyz(linha, "rotacao")?;
    }
    // linhas ortonormais: matriz torta deformaria a peça na tela e mentiria nas medidas
    for i in 0..3 {
        for j in 0..3 {
            let d: f64 = (0..3).map(|k| r[i][k] * r[j][k]).sum();
            let esperado = if i == j { 1. } else { 0. };
            if (d - esperado).abs() > 1e-6 {
                return Err(ErroApi::argumento("`rotacao` precisa ser uma rotação: linhas unitárias e perpendiculares"));
            }
        }
    }
    Ok(r)
}

fn relatorio_solucao(m: &Montagem, r: &SolveReport) -> Value {
    let falhas: Vec<String> = r.failed.iter().map(|id| m.nome_de(*id)).collect();
    json!({ "convergiu": r.converged, "residuo": r6(r.residual), "graus_de_liberdade": r.dof, "falhas": falhas })
}

impl Montagem {
    pub fn nova(nome: &str) -> Montagem {
        Montagem { asm: Assembly::new(nome), ids: vec![] }
    }

    fn nome_de(&self, id: Id) -> String {
        self.ids.iter().find(|(_, i)| *i == id).map_or_else(|| self.asm.name_of(id), |(n, _)| n.clone())
    }

    fn id_livre(&self, base: &str) -> String {
        (1..).map(|n| format!("{base}{n}")).find(|n| !self.ids.iter().any(|(x, _)| x == n)).expect("sempre há número livre")
    }

    fn componente(&self, args: &Value, chave: &str) -> Result<Id, ErroApi> {
        let nome = args[chave].as_str().ok_or_else(|| ErroApi::argumento(format!("falta `{chave}` (id do componente)")))?;
        self.ids
            .iter()
            .find(|(n, id)| n == nome && self.asm.node(*id).is_some())
            .map(|(_, id)| *id)
            .ok_or_else(|| ErroApi::novo("componente", format!("componente '{nome}' não existe; veja `montagem`")))
    }

    /// Insere a peça de `arquivo`. A primeira entra fixa, como num aplicativo de CAD.
    pub fn inserir(&mut self, args: &Value, arquivo: &str) -> Result<Value, ErroApi> {
        let id_agente = match args["id"].as_str() {
            Some(i) if self.ids.iter().any(|(n, _)| n == i) => return Err(ErroApi::argumento(format!("id '{i}' já existe na montagem"))),
            Some(i) => i.to_string(),
            None => self.id_livre("c"),
        };
        let nome = args["nome"].as_str().map(str::to_string).unwrap_or_else(|| {
            let base = std::path::Path::new(arquivo).file_stem().map_or_else(|| "Peça".into(), |s| s.to_string_lossy().to_string());
            let n = self.asm.nodes.iter().filter(|x| matches!(&x.kind, NodeKind::Part { path } if path == arquivo)).count();
            format!("{base}:{}", n + 1)
        });
        let mut lugar = Placement::default();
        if !args["origem"].is_null() {
            lugar.origin = xyz(&args["origem"], "origem")?;
        }
        if !args["rotacao"].is_null() {
            lugar.rot = rotacao(&args["rotacao"])?;
        }
        let fixo = args["fixo"].as_bool().unwrap_or(self.asm.nodes.is_empty());
        let id = self.asm.add_component(&nome, NodeKind::Part { path: arquivo.to_string() }, lugar, None);
        if let Some(n) = self.asm.node_mut(id) {
            n.fixed = fixo;
        }
        self.ids.push((id_agente.clone(), id));
        Ok(json!({ "id": id_agente, "nome": nome, "arquivo": arquivo, "fixo": fixo }))
    }

    /// Elemento de uma restrição, no sistema local da peça.
    fn elemento(&self, formas: &Formas, e: &Value, papel: &str) -> Result<MateRef, ErroApi> {
        if !e.is_object() {
            return Err(ErroApi::argumento(format!("`{papel}` é {{componente, face | plano | eixo | ponto}}")));
        }
        let comp = self.componente(e, "componente")?;
        let n = self.asm.node(comp).expect("componente achado");
        let NodeKind::Part { path } = &n.kind else { return Err(ErroApi::argumento("só peças entram em restrição")) };
        let lugar = n.placement;
        let (geom, pick) = if let Some(p) = e.get("face").filter(|v| !v.is_null()) {
            let mundo = xyz(p, "face")?;
            let local = lugar.to_local(mundo);
            let forma = formas.0.get(path).ok_or_else(|| ErroApi::novo("componente", "a peça do componente não foi carregada"))?;
            let sem_face = |m: String| ErroApi::novo("face", format!("nenhuma face do componente em {:?}: {m}", r6v(mundo)));
            // o ponto tem que estar sobre a face: longe dela, a face mais próxima seria um chute
            let (q, _) = forma.surface_near(local).map_err(|e| sem_face(e.0))?;
            let longe = (0..3).map(|i| (q[i] - local[i]).powi(2)).sum::<f64>().sqrt();
            if longe > 0.05 {
                return Err(sem_face(format!("a face mais próxima fica a {} mm", r6(longe))));
            }
            (geom_at(forma, local).map_err(sem_face)?, Some(local))
        } else if let Some(p) = e["plano"].as_str() {
            let n = normal_do_plano(p).ok_or_else(|| ErroApi::argumento("`plano` é XY, XZ ou YZ da peça"))?;
            (Geom::Plane { p: [0.; 3], n }, None)
        } else if let Some(x) = e["eixo"].as_str() {
            let d = eixo_origem(x).ok_or_else(|| ErroApi::argumento("`eixo` é X, Y ou Z da peça"))?;
            (Geom::Axis { p: [0.; 3], d }, None)
        } else if let Some(p) = e.get("ponto").filter(|v| !v.is_null()) {
            let local = lugar.to_local(xyz(p, "ponto")?);
            (Geom::Point(local), Some(local))
        } else {
            return Err(ErroApi::argumento(format!("`{papel}` precisa de face, plano, eixo ou ponto")));
        };
        Ok(MateRef { comp, geom, pick, viva: None })
    }

    pub fn restringir(&mut self, formas: &Formas, args: &Value) -> Result<Value, ErroApi> {
        let tipo = args["tipo"].as_str().ok_or_else(|| ErroApi::argumento("falta `tipo`"))?;
        let invertida = args["invertida"].as_bool().unwrap_or(false);
        let kind = match tipo {
            "coincidente" => MateKind::Coincident { flip: invertida },
            "contato" => MateKind::Contact,
            "distancia" => MateKind::Distance(valor(&args["valor"], "valor")?),
            "angulo" => MateKind::Angle(valor(&args["valor"], "valor")?),
            "paralela" => MateKind::Parallel,
            "perpendicular" => MateKind::Perpendicular,
            t => return Err(ErroApi::argumento(format!("tipo '{t}' não existe; use coincidente, contato, distancia, angulo, paralela ou perpendicular"))),
        };
        let a = self.elemento(formas, &args["a"], "a")?;
        let b = self.elemento(formas, &args["b"], "b")?;
        if a.comp == b.comp {
            return Err(ErroApi::argumento("os dois elementos são do mesmo componente"));
        }
        if !fenkra_montagem::par_valido(&kind, &a.geom, &b.geom) {
            return Err(ErroApi::novo("restricao", format!("'{tipo}' não se aplica a esse par: ângulo, paralela e perpendicular pedem planos ou eixos")));
        }
        let id_agente = match args["id"].as_str() {
            Some(i) if self.ids.iter().any(|(n, _)| n == i) => return Err(ErroApi::argumento(format!("id '{i}' já existe na montagem"))),
            Some(i) => i.to_string(),
            None => self.id_livre("r"),
        };
        let id = self.asm.add_mate(kind, a, b);
        if let Some(m) = self.asm.mates.iter_mut().find(|m| m.id == id) {
            m.sentido = match args["alinhamento"].as_str() {
                Some("mesmo") => Sentido::Mesmo,
                Some("oposto") => Sentido::Oposto,
                None | Some("livre") => Sentido::Livre,
                Some(o) => return Err(ErroApi::argumento(format!("alinhamento '{o}': use mesmo, oposto ou livre"))),
            };
            m.lado_invertido = args["lado_invertido"].as_bool().unwrap_or(false);
        }
        self.ids.push((id_agente.clone(), id));
        let r = solve(&mut self.asm);
        let mut saida = relatorio_solucao(self, &r);
        saida["id"] = json!(id_agente);
        saida["status"] = json!(if r.failed.contains(&id) { "erro" } else { "ok" });
        Ok(saida)
    }

    /// Posição nova (absoluta ou relativa), e o solucionador reaplica as restrições.
    pub fn mover(&mut self, args: &Value) -> Result<Value, ErroApi> {
        let comp = self.componente(args, "componente")?;
        let mut lugar = self.asm.lugar(comp).unwrap_or_default();
        if !args["origem"].is_null() {
            lugar.origin = xyz(&args["origem"], "origem")?;
        }
        if !args["rotacao"].is_null() {
            lugar.rot = rotacao(&args["rotacao"])?;
        }
        let t = if args["deslocar"].is_null() { [0.; 3] } else { xyz(&args["deslocar"], "deslocar")? };
        let (mut w, mut centro) = ([0.; 3], lugar.origin);
        if !args["girar"].is_null() {
            let g = &args["girar"];
            let eixo = match &g["eixo"] {
                Value::String(s) => eixo_origem(s).ok_or_else(|| ErroApi::argumento("`girar.eixo` é X, Y, Z ou [x, y, z]"))?,
                v => unit(xyz(v, "girar.eixo")?),
            };
            let ang = g["angulo"].as_f64().ok_or_else(|| ErroApi::argumento("`girar.angulo` em graus"))?.to_radians();
            w = eixo.map(|c| c * ang);
            if !g["centro"].is_null() {
                centro = xyz(&g["centro"], "girar.centro")?;
            }
        }
        lugar = lugar.moved(t, w, centro);
        if let Some(n) = self.asm.node_mut(comp) {
            n.placement = lugar;
        }
        let r = solve(&mut self.asm);
        let mut saida = relatorio_solucao(self, &r);
        let p = self.asm.lugar(comp).unwrap_or_default();
        saida["origem"] = json!(r6v(p.origin));
        saida["rotacao"] = json!(p.rot.map(r6v));
        saida["fixo"] = json!(self.asm.node(comp).is_some_and(|n| n.fixed));
        Ok(saida)
    }

    /// Componentes, posições, restrições, graus de liberdade e, com as formas, volume e caixa.
    pub fn estado(&self, formas: &Formas) -> Value {
        let mut a = self.asm.clone();
        let r = solve(&mut a);
        let dof = fenkra_montagem::solve::dof_per_comp(&a);
        let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
        let mut volume = 0.;
        let comps: Vec<Value> = a
            .nodes
            .iter()
            .filter_map(|n| {
                let NodeKind::Part { path } = &n.kind else { return None };
                let posta = formas.0.get(path).and_then(|s| posicionado(s.clone(), &n.placement));
                let mut c = json!({ "id": self.nome_de(n.id), "nome": n.name, "arquivo": path, "fixo": n.fixed, "origem": r6v(n.placement.origin), "rotacao": n.placement.rot.map(r6v), "graus_de_liberdade": dof.get(&n.id).copied().unwrap_or(0) });
                if let Some(s) = posta {
                    let v = s.mass().volume;
                    volume += v;
                    c["volume"] = json!(r6(v));
                    if let Some((l, h)) = s.bbox() {
                        for k in 0..3 {
                            lo[k] = lo[k].min(l[k]);
                            hi[k] = hi[k].max(h[k]);
                        }
                        c["caixa"] = json!({ "min": r6v(l), "max": r6v(h) });
                    }
                }
                Some(c)
            })
            .collect();
        let restricoes: Vec<Value> = a
            .mates
            .iter()
            .map(|m| json!({ "id": self.nome_de(m.id), "tipo": m.kind.label(), "a": self.nome_de(m.a.comp), "b": self.nome_de(m.b.comp), "ok": !r.failed.contains(&m.id) }))
            .collect();
        let mut v = json!({ "nome": a.name, "componentes": comps, "restricoes": restricoes, "solucao": relatorio_solucao(self, &r), "volume": r6(volume) });
        if lo[0] <= hi[0] {
            v["caixa"] = json!({ "min": r6v(lo), "max": r6v(hi) });
        }
        v
    }

    /// Lista de peças no nível pedido, numerada pela montagem.
    pub fn bom(&self, raiz: &std::path::Path, args: &Value, formas: &Formas) -> Result<Value, ErroApi> {
        let nivel = match args["nivel"].as_str() {
            None | Some("topo") => BomLevel::Topo,
            Some("pecas") => BomLevel::Pecas,
            Some("estruturada") => BomLevel::Estruturada,
            Some(o) => return Err(ErroApi::argumento(format!("nível '{o}': use topo, pecas ou estruturada"))),
        };
        let mut a = self.asm.clone();
        a.assign_items(Some(raiz));
        let linhas: Vec<Value> = bom(&a, Some(raiz), nivel)
            .iter()
            .map(|r| {
                let mut l = json!({ "item": r.item, "nome": r.name, "quantidade": r.qty, "arquivo": r.key, "pn": r.pn });
                if let Some(s) = formas.0.get(&r.key) {
                    l["volume_unitario"] = json!(r6(s.mass().volume));
                }
                l
            })
            .collect();
        let total: u32 = linhas.iter().filter_map(|l| l["quantidade"].as_u64()).sum::<u64>() as u32;
        Ok(json!({ "linhas": linhas, "itens": linhas.len(), "pecas": total }))
    }
}
