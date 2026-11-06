// SPDX-License-Identifier: LGPL-2.1-or-later
//! Dobra de tubo no nó: a parte geométrica (B326). Acha as cadeias de pernas ligadas por nós de
//! dobra, calcula cada nó (`st_dobra::calcular`), orienta as seções para o tubo ser contínuo e
//! gera o arco; também monta o tubo desenvolvido para a lista de corte (B331).
//!
//! Como funciona no kernel: cada perna é uma extrusão da seção ao longo da linha guia. No modo
//! dobradeira, cada perna recua pelo comprimento de tangente `T = R·tan(θ/2)` no nó, a seção da
//! ponta recuada é girada por `θ` em torno de um eixo que passa pelo centro da curva (`revolve`),
//! e perna, arco e perna são fundidos num sólido só. Nos modos em V a geometria do rasgo ainda não
//! existe (B328, B333): o 3D mostra o canto vivo (meia-esquadria) e o aviso diz isso.

use super::*;
use crate::st_dobra::{self, DobraDoTubo, ResultadoDobra, TuboDobrado};

/// Nó de dobra entre duas pernas do mesmo comando, já calculado.
#[derive(Clone, Debug)]
pub(crate) struct NoDobra {
    /// perna por onde o tubo chega ao nó e perna por onde sai
    pub antes: usize,
    pub depois: usize,
    pub p: V3,
    /// direção de percurso ao chegar e ao sair
    pub t_in: V3,
    pub t_out: V3,
    /// eixo da dobra (perpendicular ao plano das duas pernas), no sentido que leva `t_in` a `t_out`
    pub eixo: V3,
    pub theta: f64,
    /// eixo X da seção ao chegar no nó
    pub x_in: V3,
    pub bend: TubeBend,
    pub res: ResultadoDobra,
}

/// Pernas encadeadas por nós de dobra, na ordem do tubo.
#[derive(Clone, Debug)]
pub(crate) struct CadeiaDobra {
    pub pernas: Vec<usize>,
    pub nos: Vec<NoDobra>,
}

#[derive(Default, Debug)]
pub(crate) struct PlanoDobra {
    pub cadeias: Vec<CadeiaDobra>,
    /// perna -> motivo de não gerar sólido
    pub erros: HashMap<usize, String>,
    /// eixo X da seção que cada perna de cadeia deve usar
    pub x: HashMap<usize, V3>,
}

impl PlanoDobra {
    /// Modo do nó entre duas pernas, se houver.
    pub fn modo_do_par(&self, a: usize, b: usize) -> Option<BendMode> {
        self.cadeias.iter().flat_map(|c| &c.nos).find(|n| (n.antes == a && n.depois == b) || (n.antes == b && n.depois == a)).map(|n| n.bend.mode)
    }
}

/// Pernas fundidas por nós de dobradeira: (índices em ordem, nós entre elas).
pub(crate) fn blocos_fundidos(c: &CadeiaDobra) -> Vec<(Vec<usize>, Vec<usize>)> {
    let mut out: Vec<(Vec<usize>, Vec<usize>)> = vec![];
    let mut run = (vec![c.pernas[0]], vec![]);
    for (k, no) in c.nos.iter().enumerate() {
        if no.bend.mode == BendMode::Dobradeira && no.res.erros.is_empty() {
            run.0.push(no.depois);
            run.1.push(k);
        } else {
            out.push(std::mem::replace(&mut run, (vec![no.depois], vec![])));
        }
    }
    out.push(run);
    out
}

impl Ctx<'_> {
    /// Calcula o plano de dobras de um comando de membro: quais nós são dobra, o ângulo, o
    /// resultado de cada um e a orientação das seções.
    pub(crate) fn plano_dobras(&self, f: &Feature, m: &StructMember, geo: &[MemberGeom]) -> PlanoDobra {
        let mut plano = PlanoDobra::default();
        let juntas: Vec<&StructJoint> = m.joints.iter().filter(|j| j.mode == CornerMode::Dobra && j.bend.is_some()).collect();
        if juntas.is_empty() {
            return plano;
        }
        // pares de pernas deste comando
        let mut pares: Vec<(usize, usize, TubeBend)> = vec![];
        for j in juntas {
            let (a, b) = (j.mine, j.other_path);
            if j.other != f.id {
                plano.erros.insert(a, "a dobra de tubo exige as duas linhas no mesmo comando (tubo único)".into());
                continue;
            }
            if a >= geo.len() || b >= geo.len() || a == b {
                continue;
            }
            if pares.iter().any(|(x, y, _)| (*x, *y) == (a, b) || (*x, *y) == (b, a)) {
                continue;
            }
            pares.push((a.min(b), a.max(b), j.bend.clone().unwrap_or_default()));
        }
        // a dobra precisa da linha guia no centro da seção
        let fora_do_centro: Vec<usize> = pares.iter().flat_map(|(a, b, _)| [*a, *b]).filter(|i| geo[*i].shift.iter().any(|s| s.abs() > 1e-9)).collect();
        for i in &fora_do_centro {
            plano.erros.insert(*i, "a dobra de tubo exige a linha guia no centro da seção (ancoragem 0 e sem deslocamento)".into());
        }
        let mut grau: HashMap<usize, usize> = HashMap::new();
        for (a, b, _) in &pares {
            *grau.entry(*a).or_default() += 1;
            *grau.entry(*b).or_default() += 1;
        }
        for (i, g) in &grau {
            if *g > 2 {
                plano.erros.insert(*i, "mais de duas dobras na mesma perna".into());
            }
        }
        pares.retain(|(a, b, _)| !plano.erros.contains_key(a) && !plano.erros.contains_key(b));
        let pares_ok: Vec<(usize, usize, TubeBend)> = pares
            .into_iter()
            .filter(|(a, b, _)| {
                if shared_end(&geo[*a], &geo[*b]).is_none() {
                    for i in [a, b] {
                        plano.erros.insert(*i, "as duas linhas da dobra não se tocam numa ponta".into());
                    }
                    false
                } else {
                    true
                }
            })
            .collect();
        // cadeias: começam nas pernas com um nó só
        let mut grau: HashMap<usize, Vec<usize>> = HashMap::new();
        for (k, (a, b, _)) in pares_ok.iter().enumerate() {
            grau.entry(*a).or_default().push(k);
            grau.entry(*b).or_default().push(k);
        }
        let mut usados = vec![false; pares_ok.len()];
        let mut pontas: Vec<usize> = grau.iter().filter(|(_, v)| v.len() == 1).map(|(i, _)| *i).collect();
        pontas.sort_unstable();
        let dims: Result<Vec<f64>, String> = m.dims.iter().map(|e| ev(e, &self.vars, "cota da seção")).collect();
        let material = if m.material.trim().is_empty() { self.doc.props.material.clone() } else { m.material.clone() };
        for inicio in pontas {
            let k0 = grau[&inicio][0];
            if usados[k0] {
                continue;
            }
            let mut pernas = vec![inicio];
            let mut ordem: Vec<usize> = vec![];
            let mut atual = inicio;
            loop {
                let Some(&k) = grau[&atual].iter().find(|k| !usados[**k]) else { break };
                usados[k] = true;
                let (a, b, _) = &pares_ok[k];
                let prox = if *a == atual { *b } else { *a };
                ordem.push(k);
                pernas.push(prox);
                atual = prox;
            }
            // percurso: direções, ângulos e orientação das seções
            let mut nos: Vec<NoDobra> = vec![];
            let mut x_atual: Option<V3> = None;
            let mut x_pernas: Vec<V3> = vec![];
            let mut falha: Option<String> = None;
            for (n, &k) in ordem.iter().enumerate() {
                let (ia, ib) = (pernas[n], pernas[n + 1]);
                let bend = pares_ok[k].2.clone();
                let Some((p, da, db)) = shared_end(&geo[ia], &geo[ib]) else { continue };
                let t_in = geo3d::mul(da, -1.);
                let t_out = db;
                let c = geo3d::dot(t_in, t_out).clamp(-1., 1.);
                let theta = c.acos().to_degrees();
                let Some(eixo) = geo3d::unit(fenkra_kernel::cross(t_in, t_out)) else {
                    falha = Some("as duas linhas são colineares: não há dobra".into());
                    break;
                };
                let x = *x_atual.get_or_insert_with(|| {
                    // primeira perna: a largura ou a altura fica no plano da dobra
                    let no_plano = geo3d::unit(fenkra_kernel::cross(eixo, t_in)).unwrap_or(eixo);
                    if bend.plane_dim == 0 {
                        no_plano
                    } else {
                        eixo
                    }
                });
                if n == 0 {
                    x_pernas.push(x);
                }
                let x_depois = geo3d::rotate(x, eixo, theta.to_radians());
                x_pernas.push(x_depois);
                x_atual = Some(x_depois);
                // dimensão da seção no plano da dobra, medida na geometria
                let plano_dim = if geo3d::dot(x, eixo).abs() > std::f64::consts::FRAC_1_SQRT_2 { 1 } else { 0 };
                let (res, avisos_x) = match &dims {
                    Err(e) => {
                        falha = Some(e.clone());
                        break;
                    }
                    Ok(d) => match st_dobra::entrada_do_no(&bend, m.kind, d, theta, &material, &self.doc.tube_rules, &self.vars) {
                        Err(e) => {
                            falha = Some(e);
                            break;
                        }
                        Ok(mut e) => {
                            e.plano = plano_dim;
                            let alinhado = geo3d::dot(x, eixo).abs() > 0.999 || geo3d::dot(x, eixo).abs() < 0.02;
                            (st_dobra::calcular(&e), (!alinhado).then(|| "a seção não está alinhada ao plano desta dobra: a dimensão no plano é aproximada".to_string()))
                        }
                    },
                };
                let mut res = res;
                res.avisos.extend(avisos_x);
                nos.push(NoDobra { antes: ia, depois: ib, p, t_in, t_out, eixo, theta, x_in: x, bend, res });
            }
            if let Some(e) = falha {
                for i in &pernas {
                    plano.erros.insert(*i, e.clone());
                }
                continue;
            }
            // as pernas precisam ser mais longas que as tangentes que consomem
            for (idx, &i) in pernas.iter().enumerate() {
                let len = geo3d::dist(geo[i].start, geo[i].end);
                let mut gasto = 0.;
                if idx > 0 {
                    gasto += nos[idx - 1].res.tangente;
                }
                if idx < nos.len() {
                    gasto += nos[idx].res.tangente;
                }
                if gasto > len + 1e-6 {
                    let msg = format!("o raio não cabe: a perna tem {len:.1} mm e as tangentes da dobra pedem {gasto:.1} mm (raio menor, ou perna mais longa)");
                    plano.erros.insert(i, msg);
                }
            }
            for n in &nos {
                if !n.res.erros.is_empty() {
                    let msg = n.res.erros.join("; ");
                    plano.erros.entry(n.antes).or_insert_with(|| msg.clone());
                    plano.erros.entry(n.depois).or_insert(msg);
                }
            }
            for (i, x) in pernas.iter().zip(x_pernas.iter()) {
                plano.x.insert(*i, *x);
            }
            plano.cadeias.push(CadeiaDobra { pernas, nos });
        }
        // linhas de um par que não entrou em nenhuma cadeia (ciclo)
        for (k, (a, b, _)) in pares_ok.iter().enumerate() {
            if !usados[k] {
                for i in [a, b] {
                    plano.erros.entry(*i).or_insert_with(|| "dobras em ciclo fechado não são suportadas".into());
                }
            }
        }
        plano
    }

    /// Aplica o plano às pernas: orienta as seções e recua as pontas que viram arco.
    pub(crate) fn aplicar_dobras(&self, geo: &mut [MemberGeom], plano: &PlanoDobra) {
        for (i, x) in &plano.x {
            let z = geo[*i].z;
            // x já é perpendicular ao percurso, e a perna é paralela a ele
            geo[*i].x = x_perpendicular(*x, z);
            geo[*i].y = fenkra_kernel::cross(z, geo[*i].x);
        }
        for c in &plano.cadeias {
            for no in c.nos.iter().filter(|n| n.bend.mode == BendMode::Dobradeira && n.res.erros.is_empty()) {
                for i in [no.antes, no.depois] {
                    if plano.erros.contains_key(&i) {
                        continue;
                    }
                    let t = no.res.tangente;
                    if geo3d::dist(no.p, geo[i].start) < 1e-4 {
                        geo[i].ext.0 = -t;
                    } else {
                        geo[i].ext.1 = -t;
                    }
                }
            }
        }
    }

    /// Arco da dobradeira de um nó: a seção da ponta recuada girada por θ em torno do centro.
    pub(crate) fn arco_do_no(&self, g: &MemberGeom, no: &NoDobra) -> Result<Shape, String> {
        let k = |e: KernelError| e.0;
        let t = no.res.tangente;
        let p1 = geo3d::sub(no.p, geo3d::mul(no.t_in, t));
        let loops = structural::shifted(&g.loops, g.shift);
        let fr = Frame { origin: p1, x: no.x_in, normal: no.t_in };
        let face = Shape::face(&fr, &loops).map_err(k)?;
        let para_dentro = geo3d::unit(fenkra_kernel::cross(no.eixo, no.t_in)).ok_or("dobra sem plano")?;
        let centro = geo3d::add(p1, geo3d::mul(para_dentro, no.res.raio));
        face.revolve(centro, no.eixo, no.theta, 0.).map_err(k)
    }

    /// O tubo desenvolvido de uma cadeia: comprimentos, dobras, marcas de início e fim.
    pub(crate) fn tubo_da_cadeia(&self, f: &Feature, m: &StructMember, geo: &[MemberGeom], pernas: &[usize], nos: &[NoDobra]) -> TuboDobrado {
        let explicit = (ev(&m.ext1, &self.vars, "").unwrap_or(0.), ev(&m.ext2, &self.vars, "").unwrap_or(0.));
        let mut comprimentos: Vec<f64> = vec![];
        for &i in pernas {
            let g = &geo[i];
            let mut len = geo3d::dist(g.start, g.end);
            // prolongamento do usuário só na ponta livre (a que não é nó da cadeia)
            let toca = |q: V3| nos.iter().any(|n| geo3d::dist(n.p, q) < 1e-4 && (n.antes == i || n.depois == i));
            if !toca(g.start) {
                len += explicit.0;
            }
            if !toca(g.end) {
                len += explicit.1;
            }
            comprimentos.push(len);
        }
        let soma: f64 = comprimentos.iter().sum();
        let mut dobras = vec![];
        for (k, no) in nos.iter().enumerate() {
            let r = &no.res;
            let inicio = (desenvolvido_ate(k, &comprimentos, nos) - r.tangente).max(0.);
            let fim = inicio + r.arco;
            dobras.push(DobraDoTubo { linhas: (no.antes, no.depois), ponto: no.p, resultado: r.clone(), inicio, fim });
        }
        let ajuste: f64 = nos.iter().map(|n| n.res.ajuste_corte).sum();
        let mut avisos: Vec<String> = vec![];
        let mut erros: Vec<String> = vec![];
        for n in nos {
            avisos.extend(n.res.avisos.iter().map(|a| format!("linhas {}–{}: {a}", n.antes + 1, n.depois + 1)));
            erros.extend(n.res.erros.iter().map(|a| format!("linhas {}–{}: {a}", n.antes + 1, n.depois + 1)));
        }
        TuboDobrado {
            feature: f.id,
            linhas: pernas.to_vec(),
            designacao: format!("{} {}", m.family, m.designation),
            material: if m.material.trim().is_empty() { self.doc.props.material.clone() } else { m.material.trim().to_string() },
            pernas: comprimentos,
            dobras,
            desenvolvido: soma + ajuste,
            soma_pernas: soma,
            avisos,
            erros,
            fundido: !nos.is_empty() && nos.iter().all(|n| n.bend.mode == BendMode::Dobradeira && n.res.erros.is_empty()),
        }
    }
}

/// Posição do nó `k` ao longo do tubo desenvolvido, contada do começo da primeira perna: a soma
/// das pernas até ele mais os ajustes das dobras anteriores.
fn desenvolvido_ate(k: usize, comprimentos: &[f64], nos: &[NoDobra]) -> f64 {
    let pernas: f64 = comprimentos.iter().take(k + 1).sum();
    let ajustes: f64 = nos.iter().take(k).map(|n| n.res.ajuste_corte).sum();
    pernas + ajustes
}

/// Tira de `x` a parte paralela a `z` e normaliza (tolera erro numérico acumulado).
fn x_perpendicular(x: V3, z: V3) -> V3 {
    geo3d::unit(geo3d::sub(x, geo3d::mul(z, geo3d::dot(x, z)))).unwrap_or(x)
}

/// Tubos dobrados do documento, na ordem dos comandos: um por cadeia de pernas ligadas por nós de
/// dobra de um mesmo comando. Só contas, sem sólido (B331).
pub fn tubos_dobrados(doc: &Document) -> Vec<TuboDobrado> {
    let Ok(vars) = doc.eval_params() else { return vec![] };
    if !doc.active_features().any(|(_, f)| matches!(&f.kind, FeatureKind::StructMember(m) if m.joints.iter().any(|j| j.mode == CornerMode::Dobra))) {
        return vec![];
    }
    let cx = Ctx::vazio(doc, vars);
    let mut out = vec![];
    for (_, f) in doc.active_features() {
        let FeatureKind::StructMember(m) = &f.kind else { continue };
        if !m.joints.iter().any(|j| j.mode == CornerMode::Dobra) {
            continue;
        }
        let Ok(mut geo) = cx.members(m) else { continue };
        let plano = cx.plano_dobras(f, m, &geo);
        cx.aplicar_dobras(&mut geo, &plano);
        for c in &plano.cadeias {
            let mut t = cx.tubo_da_cadeia(f, m, &geo, &c.pernas, &c.nos);
            for i in &c.pernas {
                if let Some(e) = plano.erros.get(i) {
                    if !t.erros.contains(e) {
                        t.erros.push(e.clone());
                    }
                }
            }
            out.push(t);
        }
    }
    out
}

impl<'a> Ctx<'a> {
    /// Contexto sem sólidos calculados, só para ler as geometrias dos membros.
    pub(crate) fn vazio(doc: &'a Document, vars: Vars) -> Ctx<'a> {
        Ctx {
            relatorios_forma: vec![],
            doc,
            vars,
            qualidade: Default::default(),
            bodies: HashMap::new(),
            sketches: HashMap::new(),
            status: HashMap::new(),
            tools: HashMap::new(),
            memo: RefCell::new(HashMap::new()),
            notes: Default::default(),
            body_sig: HashMap::new(),
            last_sheet: None,
            sheet_maps: vec![],
            cut_outlines: vec![],
            roscas: vec![],
            steps: HashMap::new(),
            miter_next: false,
            promoted: RefCell::new(None),
        }
    }
}
