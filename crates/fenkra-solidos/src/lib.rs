// SPDX-License-Identifier: LGPL-2.1-or-later
//! Módulo Modelagem Sólida: executa a linha de tempo do documento sobre o kernel.
//!
//! Regras (RF-NUC-008, RF-NUC-024): um comando com erro **não** é apagado nem invalida o resto;
//! o corpo continua do estado anterior e o comando fica marcado com a mensagem do erro.

pub mod erro;
pub mod validade;
pub mod sentido_corte;
pub mod forma_brep;
pub mod forma_vinculo;
pub mod polimento;
pub mod geo3d;
pub mod regioes;
pub mod analise;
pub mod loft;
pub mod importacao;
pub mod corpo_malha;
pub mod mesh_io;
pub mod structural;
pub mod perfis_catalogo;
pub mod perfis_catalogo_tubos;
pub mod st_cantos;
pub mod st_limite;
pub mod st_encaixe;
pub mod st_encaixe_doc;
#[cfg(test)]
mod st_encaixe_tests;
pub mod st_dobra;
mod st_dobra_geo;
pub mod st_dobra_corte;
pub mod st_dobra_saida;
pub mod st_dobra_vista;
pub mod st_dobra_maquina;
mod st_dobra_v;
mod st_dobra_gomos;
pub use st_dobra_geo::tubos_dobrados;
pub mod hardware;
pub mod biblioteca;
pub mod catalogo;
pub mod fixadores;
pub mod rolamentos;
pub mod engrenagens;
pub mod engrenagens_dim;
pub mod transmissao_calc;
pub mod transmissao_dados;
pub mod engrenagens_conicas;
pub mod transmissao_corrente;
pub mod transmissao_proposta;
pub mod usinagens;
pub mod eixos_recursos;
pub mod eixos;
pub mod chaveta_rasgos;
pub mod calculo_projeto;
pub mod profile;
pub mod refs;
pub mod chapa;
pub mod massa;
pub mod termino;
mod edicao_direta;
pub mod reconhece;
pub mod dfm;
pub mod dfm_geo;
pub mod padrao;
pub mod furo_norma;
pub mod rosca_furo;
pub mod previa_leve;
pub mod chapa_linha;
mod chapa_dobra_curva;
mod chapa_flat;
mod arestas;
mod chapa_laco;
pub mod chapa_canto;
pub mod chapa_desdobra;
pub mod chapa_converte;
mod chapa_converte_mapa;
mod chapa_perfis;
#[cfg(test)]
mod chapa_canto_tests;
pub use chapa_laco::{laco_de_malha, recuos_extensao};
pub mod planificada;
pub mod nesting;
pub mod chapa_dobra_sim;
pub use chapa_flat::{passos_planificados, PassoPlanificado};
pub mod st_frames;
mod st_placas;
pub mod st_solda;
pub use st_solda::{weld_beads, CaminhoCordao, CordaoCalculado, TrechoCordao};

pub use engrenagens::{calculo as calculo_engrenagem, engrenagem_documento, CalculoEngrenagem, ParamEngrenagem, TipoEngrenagem};
pub use eixos::{chaveta_documento, eixo_documento, ParamEixo, RasgoChaveta, TrechoEixo};

use profile::{ev, eval_sketch, to3d, EvalSketch, Vars};
use fenkra_core::sketch::{Curve, Kind};
use fenkra_core::*;
use fenkra_kernel::{Bool, Frame, HoleShape, KernelError, Mesh, MassProps, Shape, Xform};
use geo3d::{Curve3, V3};
use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::Instant;

#[derive(Clone, Debug, PartialEq)]
pub enum Status {
    Ok,
    Warning(String),
    Error(String),
    /// Depois do marcador de rollback ou suprimido.
    Inactive,
}

impl Status {
    pub fn message(&self) -> Option<&str> {
        match self {
            Status::Warning(m) | Status::Error(m) => Some(m),
            _ => None,
        }
    }
}

pub struct BodyResult {
    pub collector: Id,
    pub color: [u8; 3],
    pub mesh: Mesh,
    pub mass: MassProps,
    pub faces: u32,
    /// Comando que gerou cada face (mesma ordem de `mesh.face_ranges`).
    pub face_owner: Vec<Id>,
    /// Papel de cada face para o comando que a gerou (paralelo a `face_owner`): 0 tampa inicial,
    /// 1 tampa final, 2+i lateral gerada pela aresta i do perfil, -1 desconhecido (ADR 0010).
    pub face_role: Vec<i32>,
}

impl BodyResult {
    /// Chave v2 da face `face` (índice na ordem de `mesh.face_ranges`, o mesmo do picking).
    /// `None` se o papel dela é desconhecido: nesse caso a referência fica só com o ponto.
    pub fn face_key(&self, face: usize) -> Option<FaceKey> {
        let info = self.mesh.face_info.get(face)?;
        let role = *self.face_role.get(face)?;
        (role >= 0).then(|| FaceKey { owner: self.face_owner[face], role, kind: refs::kind_of(info.kind), normal_or_axis: info.dir, area: info.area })
    }

    /// Chave v2 da aresta `edge` (índice na ordem de `mesh.edge_ranges`): as duas faces vizinhas.
    pub fn edge_key(&self, edge: usize) -> Option<EdgeKey> {
        let [p, q] = *self.mesh.edge_faces.get(edge)?;
        if p == fenkra_kernel::NO_FACE || q == fenkra_kernel::NO_FACE {
            return None;
        }
        Some(EdgeKey { a: self.face_key(p as usize)?, b: self.face_key(q as usize)? })
    }
}

/// Corpo de um coletor: forma, dono de cada face e papel de cada face (as duas listas na ordem
/// das faces do kernel).
type Body = (Shape, Vec<Id>, Vec<i32>);

/// Corpo recém-criado por `fid`: todas as faces são dele, sem papel conhecido.
fn fresh_body(s: Shape, fid: Id) -> Body {
    let n = s.face_count() as usize;
    (s, vec![fid; n], vec![-1; n])
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn unit(v: [f64; 3]) -> [f64; 3] {
    let l = dot(v, v).sqrt().max(1e-12);
    v.map(|x| x / l)
}
/// Diagonal aproximada dos perfis (para dimensionar ferramentas).
/// Pontos que representam o segmento (arcos e círculos amostrados) para testes de posição do perfil.
fn seg_points(sgm: &fenkra_kernel::Seg2) -> Vec<[f64; 2]> {
    use fenkra_kernel::Seg2;
    let circle = |c: [f64; 2], r: f64, a0: f64, sweep: f64| -> Vec<[f64; 2]> { (0..=48).map(|i| a0 + sweep * i as f64 / 48.).map(|t| [c[0] + r * t.cos(), c[1] + r * t.sin()]).collect() };
    match sgm {
        Seg2::Line(a, b) => vec![*a, *b],
        Seg2::Circle(c, r) => circle(*c, *r, 0., std::f64::consts::TAU),
        Seg2::Spline(p) => p.clone(),
        Seg2::Ellipse(c, m, r, t0, t1) => fenkra_core::projecao::elipse_amostras(*c, *m, *r, *t0, *t1),
        Seg2::Arc(a, m, b) => {
            // círculo pelos três pontos
            let (d, a2, m2, b2) = (2. * (a[0] * (m[1] - b[1]) + m[0] * (b[1] - a[1]) + b[0] * (a[1] - m[1])), a[0] * a[0] + a[1] * a[1], m[0] * m[0] + m[1] * m[1], b[0] * b[0] + b[1] * b[1]);
            if d.abs() < 1e-12 {
                return vec![*a, *m, *b];
            }
            let c = [(a2 * (m[1] - b[1]) + m2 * (b[1] - a[1]) + b2 * (a[1] - m[1])) / d, (a2 * (b[0] - m[0]) + m2 * (a[0] - b[0]) + b2 * (m[0] - a[0])) / d];
            let r = ((a[0] - c[0]).powi(2) + (a[1] - c[1]).powi(2)).sqrt();
            let ang = |p: &[f64; 2]| (p[1] - c[1]).atan2(p[0] - c[0]);
            let (ta, tm, tb) = (ang(a), ang(m), ang(b));
            let up = |x: f64| x.rem_euclid(std::f64::consts::TAU);
            // sentido anti-horário se o meio é alcançado antes do fim
            let sweep = if up(tm - ta) <= up(tb - ta) { up(tb - ta) } else { up(tb - ta) - std::f64::consts::TAU };
            circle(c, r, ta, sweep)
        }
    }
}

/// Revolução: o eixo precisa estar no plano do perfil e o perfil não pode cruzá-lo (o sólido se
/// autointerceptaria); antes o kernel recebia o pedido e devolvia lixo ou uma falha sem explicação.
fn check_revolve_profile(frame: &Frame, sk: &EvalSketch, origin: V3, dir: V3) -> Result<(), String> {
    let d = geo3d::unit(dir).ok_or("o eixo de revolução é nulo")?;
    let off = geo3d::sub(origin, frame.origin);
    if geo3d::dot(d, frame.normal).abs() > 1e-6 || geo3d::dot(off, frame.normal).abs() > 1e-4 {
        return Err("o eixo de revolução não está no plano do sketch do perfil — use um eixo desenhado no mesmo sketch ou contido no plano dele".into());
    }
    let (y, o2) = (frame.y(), [geo3d::dot(off, frame.x), geo3d::dot(off, frame.y())]);
    let d2 = [geo3d::dot(d, frame.x), geo3d::dot(d, y)];
    let side = |p: [f64; 2]| d2[0] * (p[1] - o2[1]) - d2[1] * (p[0] - o2[0]);
    let size = loops_size(&sk.loops).max(1.);
    let tol = 1e-6 * size;
    let (mut lo, mut hi) = (0f64, 0f64);
    for l in &sk.loops {
        for sgm in l {
            for p in seg_points(sgm) {
                let t = side(p);
                lo = lo.min(t);
                hi = hi.max(t);
            }
        }
    }
    for ch in &sk.open {
        for p in ch {
            let t = side(*p);
            lo = lo.min(t);
            hi = hi.max(t);
        }
    }
    if lo < -tol && hi > tol {
        return Err("o perfil cruza o eixo de revolução — deixe o perfil inteiro de um lado do eixo".into());
    }
    Ok(())
}

fn extent_name(e: &Extent) -> &'static str {
    match e {
        Extent::Blind(_) => "cego",
        Extent::Symmetric(_) => "simétrico",
        Extent::TwoSides(..) => "dois lados",
        Extent::ThroughAll => "através de tudo",
        Extent::UpTo(_) => "até plano",
        Extent::UpToNext => "até o próximo",
        Extent::UpToLast => "até o último",
        Extent::UpToSurface(_) => "até a superfície",
    }
}

fn loops_size(loops: &[fenkra_kernel::Loop2]) -> f64 {
    let (mut lo, mut hi) = ([f64::MAX; 2], [f64::MIN; 2]);
    let mut add = |p: [f64; 2], r: f64| {
        for i in 0..2 {
            lo[i] = lo[i].min(p[i] - r);
            hi[i] = hi[i].max(p[i] + r);
        }
    };
    for l in loops {
        for sgm in l {
            match *sgm {
                fenkra_kernel::Seg2::Line(a, b) => {
                    add(a, 0.);
                    add(b, 0.)
                }
                fenkra_kernel::Seg2::Arc(a, m, b) => {
                    add(a, 0.);
                    add(m, 0.);
                    add(b, 0.)
                }
                fenkra_kernel::Seg2::Circle(c, r) => add(c, r),
                fenkra_kernel::Seg2::Spline(ref p) => p.iter().for_each(|q| add(*q, 0.)),
                fenkra_kernel::Seg2::Ellipse(c, m, r, t0, t1) => fenkra_core::projecao::elipse_amostras(c, m, r, t0, t1).into_iter().for_each(|q| add(q, 0.)),
            }
        }
    }
    if lo[0] > hi[0] {
        return 1.;
    }
    ((hi[0] - lo[0]).powi(2) + (hi[1] - lo[1]).powi(2)).sqrt()
}

fn tags_for(owners: &[Id]) -> Vec<i32> {
    (0..owners.len() as i32).collect()
}

/// Converte as etiquetas devolvidas pelo kernel em donos; -1 = face nova do comando `fid`.
fn remap(owners: &[Id], out: &[i32], fid: Id) -> Vec<Id> {
    out.iter().map(|t| if *t >= 0 { owners.get(*t as usize).copied().unwrap_or(fid) } else { fid }).collect()
}

/// Papéis das faces do resultado: seguem o mesmo índice-origem que `remap`; face nova = -1.
fn remap_roles(roles: &[i32], out: &[i32]) -> Vec<i32> {
    out.iter().map(|t| if *t >= 0 { roles.get(*t as usize).copied().unwrap_or(-1) } else { -1 }).collect()
}

/// Dono e papel das faces do resultado de uma operação que propaga etiquetas.
fn remap_body(s: Shape, owners: &[Id], roles: &[i32], out: &[i32], fid: Id) -> Body {
    (s, remap(owners, out, fid), remap_roles(roles, out))
}

/// Geometria de exibição de um sketch (segmentos 3D).
pub struct SketchViz {
    pub feature: Id,
    /// plano do sketch (para alinhar a câmera)
    pub frame: Frame,
    pub lines: Vec<[f32; 3]>,
    pub construction: Vec<[f32; 3]>,
    pub points: Vec<[f32; 3]>,
    /// triângulos da sombra dos perfis fechados (lista sem índices)
    pub fill: Vec<[f32; 3]>,
    /// graus de liberdade restantes e se o solver convergiu
    pub dof: usize,
    pub converged: bool,
    /// posições atuais da geometria externa (o editor do sketch copia para o documento)
    pub link_pts: Vec<(Id, [f64; 2])>,
    pub link_r: Vec<(Id, f64)>,
    /// elipses externas: curva, semieixo maior, semieixo menor, t0 e t1
    pub link_e: Vec<(Id, [f64; 2], f64, f64, f64)>,
    /// vínculos que perderam a origem (o estado "→✕" do painel)
    pub link_quebrados: Vec<Id>,
    /// geometria externa que mudou de estrutura: o editor aplica o mesmo pedido no documento
    pub link_ops: Vec<fenkra_core::vinculo::OpVinculo>,
    /// cada curva do sketch no mundo (id, construção, polilinha) — subelementos selecionáveis
    pub ents: Vec<(Id, bool, Vec<[f32; 3]>)>,
    /// cada ponto do sketch no mundo (id, posição)
    pub pt_ids: Vec<(Id, [f32; 3])>,
}

/// Plano exibível (Origem ou Referências).
pub struct PlaneViz {
    pub id: Option<Id>,
    pub name: String,
    pub frame: Frame,
}

/// Ponto de referência exibível.
pub struct PointViz {
    pub id: Id,
    pub name: String,
    pub p: [f64; 3],
}

/// Linha/curva de referência exibível (polilinha).
pub struct CurveViz {
    pub id: Id,
    pub name: String,
    pub pts: Vec<[f64; 3]>,
    /// vértices de definição (pontas da linha, pontos da polilinha/spline) — selecionáveis
    pub verts: Vec<[f64; 3]>,
}

/// Faixa regrada que sai da curva. Com apoio, a normal e a tangente saem da superfície em cada ponto.
fn regrada(c: &geo3d::Curve3, apoio: Option<&Shape>, modo: RuledMode, vetor: Option<V3>, l: f64) -> Result<Shape, String> {
    use geo3d::*;
    if l == 0. {
        return Err("o comprimento precisa ser diferente de zero".into());
    }
    // reta tem dois pontos, e o apoio curvo pede mais
    let pts = if c.pts.len() < 24 { resample(c, 24) } else { c.pts.clone() };
    let curva = Curve3::poly(pts.clone(), false)?;
    let fora = apoio.map(|s| s.mass().center);
    let mut saida = vec![];
    for p in &pts {
        let w = match (modo, vetor, apoio) {
            (RuledMode::Vetor, Some(v), _) => v,
            (_, _, None) => return Err("este modo precisa da superfície de apoio".into()),
            (RuledMode::Normal, _, Some(s)) => s.surface_near(*p).map_err(|x| x.0)?.1,
            (_, _, Some(s)) => {
                let n = s.surface_near(*p).map_err(|x| x.0)?.1;
                let t = curva.at_dist(curva.nearest(*p).0).1;
                let w = unit(cross(n, t)).ok_or("borda perpendicular à superfície")?;
                // para fora da borda: a faixa continua a superfície
                if dot(w, sub(*p, fora.unwrap_or(*p))) < 0. { mul(w, -1.) } else { w }
            }
        };
        saida.push(add(*p, mul(w, l)));
    }
    Shape::loft_surface(&[pts, saida], true, false, None).map_err(|x| x.0)
}

/// Borda de `sh` que "olha" para `other`: o trecho com a menor distância média até a outra superfície.
/// Com `perto`, a escolha é da pessoa: fica o trecho mais perto do ponto.
fn escolher_borda(runs: &[Vec<V3>], other: &Shape, perto: Option<V3>) -> Result<usize, String> {
    use geo3d::*;
    let mut best: Option<(f64, usize)> = None;
    for (i, r) in runs.iter().enumerate() {
        // na escolha da pessoa vale o ponto mais perto, e o trecho comprido não perde por ser comprido
        let m = match perto {
            Some(q) => r.iter().map(|p| dist(*p, q)).fold(f64::MAX, f64::min),
            None => {
                let mut acc = 0.;
                for p in r {
                    acc += dist(*p, other.surface_near(*p).map_err(|x| x.0)?.0);
                }
                acc / r.len() as f64
            }
        };
        if best.is_none_or(|b| m < b.0) {
            best = Some((m, i));
        }
    }
    best.map(|b| b.1).ok_or_else(|| "a superfície não tem borda livre para conectar".into())
}

/// Bordas livres de `sh`, divididas nos cantos vivos.
fn bordas_livres(sh: &Shape) -> Result<Vec<Vec<V3>>, String> {
    use geo3d::*;
    let loops = sh.boundary().map_err(|x| x.0)?;
    let mut out = vec![];
    for mut lp in loops {
        if lp.len() > 2 && dist(lp[0], *lp.last().unwrap()) < 1e-6 {
            lp.pop();
        }
        let n = lp.len();
        if n < 2 {
            continue;
        }
        let corner = |i: usize| {
            let (a, b, c) = (lp[(i + n - 1) % n], lp[i], lp[(i + 1) % n]);
            match (unit(sub(b, a)), unit(sub(c, b))) {
                (Some(u), Some(v)) => dot(u, v) < 0.87,
                _ => false,
            }
        };
        let cs: Vec<usize> = (0..n).filter(|i| corner(*i)).collect();
        let runs: Vec<Vec<V3>> = if cs.is_empty() {
            vec![lp.clone()]
        } else {
            (0..cs.len())
                .map(|k| {
                    let (s0, s1) = (cs[k], cs[(k + 1) % cs.len()]);
                    let mut v = vec![];
                    let mut i = s0;
                    loop {
                        v.push(lp[i]);
                        if i == s1 && v.len() > 1 {
                            break;
                        }
                        i = (i + 1) % n;
                        if v.len() > n + 1 {
                            break;
                        }
                    }
                    v
                })
                .collect()
        };
        out.extend(runs.into_iter().filter(|r| r.len() >= 2));
    }
    Ok(out)
}

/// Filete (raio constante ou variável) nas arestas de uma superfície, pelos pontos médios.
fn edge_fillet(sh: &Shape, mids: &[[f64; 3]], r: f64, r2: Option<f64>) -> Result<Shape, String> {
    let tags: Vec<i32> = (0..sh.face_count() as i32).collect();
    let tol = sh.bbox().map(|(a, c)| geo3d::dist(a, c) * 0.05).unwrap_or(1.).max(1.);
    let res = match r2 {
        Some(r2) => sh.edges_mode_tagged(0, r, r2, mids, &[], tol, &tags),
        None => sh.edges_tagged(r, mids, tol, false, &tags),
    };
    res.map(|x| x.0).map_err(|e| format!("filete da superfície: {} (raio grande demais ou arestas que não ligam duas faces)", e.0))
}

/// Encadeia curvas pelas pontas (na ordem que se encaixarem; inverte as que vierem ao contrário).
pub fn join_curves(mut cs: Vec<Curve3>) -> Result<Curve3, String> {
    if cs.is_empty() {
        return Err("escolha as curvas a juntar".into());
    }
    let size = cs.iter().map(|c| c.length()).sum::<f64>().max(1.);
    let tol = (1e-4 * size).max(1e-3);
    let first = cs.remove(0);
    let mut pts = first.pts.clone();
    if first.closed && cs.is_empty() {
        return Ok(first);
    }
    while !cs.is_empty() {
        let (s, e) = (pts[0], *pts.last().unwrap());
        let near = |a: V3, b: V3| geo3d::dist(a, b) <= tol;
        let Some((i, how)) = cs.iter().enumerate().find_map(|(i, c)| {
            let (cs0, ce) = (c.start(), c.end());
            if near(e, cs0) {
                Some((i, 0))
            } else if near(e, ce) {
                Some((i, 1))
            } else if near(s, ce) {
                Some((i, 2))
            } else if near(s, cs0) {
                Some((i, 3))
            } else {
                None
            }
        }) else {
            return Err(format!("{} curva(s) não tocam as outras nas pontas (tolerância {:.3} mm)", cs.len(), tol));
        };
        let mut q = cs.remove(i).pts;
        match how {
            0 => pts.extend(q.into_iter().skip(1)),
            1 => {
                q.reverse();
                pts.extend(q.into_iter().skip(1))
            }
            2 => {
                q.pop();
                q.extend(pts);
                pts = q;
            }
            _ => {
                q.reverse();
                q.pop();
                q.extend(pts);
                pts = q;
            }
        }
    }
    let closed = pts.len() > 3 && geo3d::dist(pts[0], *pts.last().unwrap()) <= tol;
    if closed {
        pts.pop();
    }
    Curve3::poly(pts, closed)
}

/// Valor de um elemento de geometria de referência.
#[derive(Clone)]
pub enum Geo {
    Plane(Frame),
    Point(V3),
    Curve(Curve3),
    Surface(Shape),
}

/// O que o cálculo apurou sobre uma referência. `text` vira aviso quando `warn`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RefNote {
    pub text: String,
    pub warn: bool,
    /// como as curvas se fecham (preenchimento)
    pub contour: Option<geo3d::ContourReport>,
    /// resultado da costura (juntar)
    pub join: Option<fenkra_kernel::SewReport>,
    /// erro de tangência medido no contorno, em graus
    pub g1_deg: Option<f64>,
    /// o que a extração alcançou: faces (ou arestas, no contorno) e arestas
    pub count: Option<(u32, u32)>,
    /// de onde sai a seta da distância: ponto sobre a superfície e a normal real ali
    pub arrow: Option<(V3, V3)>,
    /// ponto da curva escolhida pelo número: vira a âncora espacial dela
    pub anchor: Option<V3>,
    /// B276: as soluções possíveis no espaço (número, ponto): cada uma vira uma bolinha no 3D
    pub solucoes: Vec<(usize, V3)>,
    /// número da solução em uso
    pub solucao: Option<usize>,
    /// B277: pedaços do corte, com o que fica e o que sai
    pub regioes: Vec<regioes::RegiaoNota>,
    /// B34: números e mapa da análise
    pub analise: Option<analise::Analise>,
    /// B290: isocurvas pedidas no multi-seções
    pub iso: Vec<Vec<V3>>,
    /// B290: seções como o cálculo as recebeu, para os conectores andarem sobre elas
    pub secoes: Vec<geo3d::Curve3>,
    /// B291: pontos da spline já resolvidos, para o painel fixar a tangente onde ela está
    pub pontos: Vec<V3>,
    /// B290: seção que o cálculo percorre ao contrário, deduzida ou pedida
    pub sentidos: Vec<bool>,
    /// B290: seta de tensão de cada lado: ponto na borda, direção que sai dela e corda da ponte
    pub tensoes: Vec<(V3, V3, f64)>,
    /// B290: bordas que o Conectar pode usar em cada superfície, e qual está em uso
    pub bordas: [Vec<Vec<V3>>; 2],
    pub borda_usada: [usize; 2],
}

/// Superfície de referência exibível (malha).
pub struct SurfaceViz {
    pub id: Id,
    pub name: String,
    pub mesh: Mesh,
}

/// Superfície lisa de uma forma livre, para desenhar (B153).
#[derive(Clone, Debug)]
pub struct FormaViz {
    pub id: Id,
    pub malha: fenkra_core::subd::MalhaLisa,
    /// curvaturas estimadas por vértice (1/mm); vazio fora do mapa de curvatura (B346)
    pub curvatura: Vec<[f32; 2]>,
}

/// Contorno de um recorte de chapa nos dois estados: o desenhado no estado do comando e o levado
/// ao outro (inclusive pelo raio da dobra), cada um como polilinhas fechadas em 3D.
#[derive(Clone, Debug, PartialEq)]
pub struct CutOutline {
    pub feature: Id,
    pub folded: Vec<Vec<V3>>,
    pub flat: Vec<Vec<V3>>,
    /// Na planificada, o que o comando faz além de cortar também passa por aqui: veneziana e
    /// estampo guardam o contorno da ferramenta em `flat` (`folded` vazio), e cada canto da aba por
    /// perfil guarda a linha de dobra em `dobra` (as dobras do mapa vêm de `sheet_maps`).
    pub dobra: Option<BendLine>,
}

/// Onde a rosca fica, com o eixo e o sentido que a ferramenta usa, para a prévia desenhar a hélice.
#[derive(Clone, Debug, PartialEq)]
pub struct RoscaViz {
    pub feature: Id,
    /// início da rosca, sobre o eixo
    pub origin: V3,
    pub dir: V3,
    pub radius: f64,
    pub len: f64,
    pub pitch: f64,
    /// rosca à esquerda: a hélice gira ao contrário
    pub esquerda: bool,
    /// face interna (furo) ou externa (eixo)
    pub interna: bool,
    /// com sulco real no corpo; sem ele (cosmética, ou prévia leve) a face leva só a textura de rosca
    pub modelada: bool,
}

/// Linha de dobra na chapa planificada: começo, centro e fim do arco desenvolvido.
#[derive(Clone, Debug, PartialEq)]
pub struct BendLine {
    pub start: [V3; 2],
    pub center: [V3; 2],
    pub end: [V3; 2],
    /// ângulo (graus), raio interno e sentido (para cima = para o lado da face de referência)
    pub angle: f64,
    pub radius: f64,
    pub up: bool,
    /// comando que fez a dobra
    pub feature: Id,
    /// comprimento desenvolvido (linha neutra) e dedução de dobra: os mesmos da geometria
    pub allowance: f64,
    pub deduction: f64,
    /// eixo da dobra no espaço dobrado (as alças 3D e a linha de dobra desenhada leem daqui);
    /// `None` nas dobras que só existem na planificada (cantos da aba por perfil)
    pub eixo: Option<EixoDobra>,
    /// linha central quando a dobra é curva (cadeia curva); vazia na dobra reta, que `center` descreve
    pub curva: Vec<V3>,
}

/// Eixo de uma dobra no espaço dobrado.
#[derive(Clone, Debug, PartialEq)]
pub struct EixoDobra {
    /// pontas do eixo, na face interna, no começo do arco
    pub a: V3,
    pub b: V3,
    /// centro de curvatura na ponta `a`
    pub centro: V3,
    /// normal da chapa apontando para o centro de curvatura
    pub normal: V3,
    /// direção em que a aba sai (antes de dobrar)
    pub saida: V3,
}

impl BendLine {
    /// Rótulo para a planificação ("↑ 90° R2").
    pub fn label(&self) -> String {
        let f = |v: f64| {
            let s = format!("{v:.1}");
            s.trim_end_matches('0').trim_end_matches('.').replace('.', ",")
        };
        format!("{} {}° R{}", if self.up { "↑" } else { "↓" }, f(self.angle), f(self.radius))
    }
}

/// Chaves v2 calculadas no recompute para referências que só tinham o ponto (arquivo antigo).
/// O app grava no comando do documento; cada lista é paralela à do comando (mantém as chaves que
/// já existiam e acrescenta as novas).
#[derive(Clone, Debug, PartialEq)]
pub enum Promoted {
    /// `EdgeOp.keys`
    Edges(Vec<Option<EdgeKey>>),
    /// `face_keys` de Shell, Draft e Thickness
    Faces(Vec<Option<FaceKey>>),
}

pub struct RecomputeResult {
    /// o que o cálculo apurou sobre cada referência, além do status
    pub notes: HashMap<Id, RefNote>,
    pub bodies: Vec<BodyResult>,
    /// comando → chaves v2 promovidas neste cálculo (pontos antigos que resolveram)
    pub promoted: HashMap<Id, Promoted>,
    pub sketches: Vec<SketchViz>,
    pub planes: Vec<PlaneViz>,
    pub points: Vec<PointViz>,
    pub curves: Vec<CurveViz>,
    pub surfaces: Vec<SurfaceViz>,
    /// malha de tela de cada corpo e superfície que tiver uma, por coletor ou referência (B346)
    pub exibicao: HashMap<Id, Arc<Mesh>>,
    /// formas livres ainda sem B-rep
    pub formas: Vec<FormaViz>,
    /// relatório de cada forma convertida
    pub relatorios_forma: Vec<(Id, forma_brep::RelatorioForma)>,
    /// gaiolas com os vínculos aplicados (B303)
    pub formas_resolvidas: Vec<forma_vinculo::FormaResolvida>,
    pub status: HashMap<Id, Status>,
    pub params: HashMap<String, f64>,
    pub param_error: Option<String>,
    pub elapsed_ms: f64,
    pub kernel: String,
    /// linhas de dobra da chapa (no planificado)
    pub bend_lines: Vec<BendLine>,
    /// contorno de cada recorte de chapa nos dois estados, para a prévia simultânea
    pub cut_outlines: Vec<CutOutline>,
    /// comandos que não foram refeitos porque o cálculo parou antes (Atualizar esta feature ou
    /// Cancelar); a árvore os mostra como desatualizados
    pub desatualizados: Vec<Id>,
    /// o cálculo foi interrompido por Cancelar: o resultado é parcial e a interface deve
    /// descartá-lo e manter o último completo
    pub cancelado: bool,
    /// posição de cada rosca, para a prévia
    pub roscas: Vec<RoscaViz>,
    /// prévia leve dos comandos pesados em edição (B356)
    pub previas: Vec<previa_leve::PreviaViz>,
}

impl RecomputeResult {
    /// Causa estável de cada comando que falhou (B69), a partir do documento calculado.
    pub fn tipos_de_erro(&self, doc: &Document) -> HashMap<Id, erro::ErrorKind> {
        erro::classificar(doc, &self.status, self.param_error.as_deref())
    }
}

pub fn origin_frame(p: OriginPlane) -> Frame {
    match p {
        OriginPlane::XY => Frame { origin: [0.; 3], x: [1., 0., 0.], normal: [0., 0., 1.] },
        OriginPlane::XZ => Frame { origin: [0.; 3], x: [1., 0., 0.], normal: [0., -1., 0.] },
        OriginPlane::YZ => Frame { origin: [0.; 3], x: [0., 1., 0.], normal: [1., 0., 0.] },
    }
}

pub fn origin_axis(a: OriginAxis) -> [f64; 3] {
    match a {
        OriginAxis::X => [1., 0., 0.],
        OriginAxis::Y => [0., 1., 0.],
        OriginAxis::Z => [0., 0., 1.],
    }
}

/// Malha de exibição de uma forma na qualidade pedida (desvio pela diagonal da caixa envolvente).
pub fn tesselar(sh: &Shape, q: &QualidadeMalha) -> Result<Mesh, KernelError> {
    let diagonal = sh.bbox().map(|(a, b)| geo3d::dist(a, b)).unwrap_or(100.);
    sh.mesh_angle(q.desvio(diagonal), q.angulo_rad())
}

struct Ctx<'a> {
    doc: &'a Document,
    vars: Vars,
    /// qualidade das malhas que o cálculo gera (nunca muda o sólido)
    qualidade: QualidadeMalha,
    bodies: HashMap<Id, Body>,
    sketches: HashMap<Id, (Frame, Arc<EvalSketch>)>,
    /// referências v2 que o comando em execução calculou para pontos sem chave (promoção)
    promoted: RefCell<Option<Promoted>>,
    /// sólido gerado por cada comando (base para filete/chanfro por comando)
    status: HashMap<Id, Status>,
    /// ferramenta e operação de cada comando de forma (para padrões e espelho)
    tools: HashMap<Id, (Shape, BoolOp)>,
    /// geometria de referência já avaliada neste cálculo (cadeias de referências não recalculam)
    memo: RefCell<HashMap<Id, Geo>>,
    /// o que cada referência tem a dizer além do resultado
    notes: RefCell<HashMap<Id, RefNote>>,
    /// assinatura do último passo que mudou o corpo de cada coletor (cache de malha)
    body_sig: HashMap<Id, u64>,
    /// medidas da última aba de chapa (para os alívios)
    last_sheet: Option<fenkra_kernel::SheetEdgeInfo>,
    /// dobras de chapa já feitas (no espaço dobrado), para levar recortes e filetes entre o
    /// dobrado e o planificado
    sheet_maps: Vec<SheetMap>,
    /// todas as operações de cada comando, em ordem (o padrão repete soma e corte do estampo)
    steps: HashMap<Id, Vec<(Shape, BoolOp)>>,
    /// a próxima aba sai com as pontas em meia-esquadria
    miter_next: bool,
    /// contorno de cada recorte de chapa nos dois estados (dobrado e planificado)
    cut_outlines: Vec<CutOutline>,
    /// corpos de malha por coletor; ficam ao lado de `bodies`, sem `Shape`
    malhas: HashMap<Id, corpo_malha::CorpoMalha>,
    roscas: Vec<RoscaViz>,
    previas: Vec<previa_leve::PreviaViz>,
    /// comando em edição: a rosca modelada dele vira só linhas, sem construir a hélice (B351)
    leve: Option<Id>,
    relatorios_forma: Vec<(Id, forma_brep::RelatorioForma)>,
    formas_resolvidas: Vec<forma_vinculo::FormaResolvida>,
}

/// Uma dobra: referencial da aresta (no dobrado) e o giro que leva a aba do dobrado ao plano.
#[derive(Clone, Copy, Debug)]
struct SheetMap {
    c0: V3,
    ev: V3,
    l: f64,
    n: V3,
    u: V3,
    t: f64,
    /// eixo da dobra
    o: V3,
    axis: V3,
    /// ângulo (rad), desenvolvido da dobra e comprimento da aba
    a: f64,
    ba: f64,
    length: f64,
    /// a dobra vai para o lado de dentro do material (para baixo da face de referência)
    down: bool,
    /// comando que fez a dobra (preenchido por `Ctx::run`; vale `Id(0)` enquanto o comando roda)
    feature: Id,
}

impl SheetMap {
    fn rot(&self, p: V3, a: f64) -> V3 {
        geo3d::add(self.o, geo3d::rotate(geo3d::sub(p, self.o), self.axis, a))
    }
    /// Coordenadas (ao longo de u, da aresta, da normal) de um ponto no referencial da aresta.
    fn local(&self, p: V3) -> (f64, f64, f64) {
        let q = geo3d::sub(p, self.c0);
        (geo3d::dot(q, self.u), geo3d::dot(q, self.ev), geo3d::dot(q, self.n))
    }
    /// O ponto está na aba (dobrada ou planificada)?
    fn holds(&self, p: V3, flat: bool) -> bool {
        let m = 2. * self.t + 1.;
        let (su, se, sn) = if flat { self.local(p) } else { self.local(self.rot(p, -self.a)) };
        let umax = if flat { self.ba + self.length } else { self.length };
        let umin = if flat { -1e-6 } else { -self.length.max(1.) * 1e-6 };
        su >= umin && su <= umax + 1e-6 && se >= -1e-6 - m && se <= self.l + m && sn >= -self.t - m && sn <= m
    }
    fn to_flat(&self, p: V3) -> V3 {
        geo3d::add(self.rot(p, -self.a), geo3d::mul(self.u, self.ba))
    }
    /// Ponto da zona da dobra (entre o começo e o fim do arco desenvolvido) do planificado ao
    /// dobrado: a fibra neutra mantém o comprimento (φ = u / ρ) e a distância ao eixo é o raio
    /// interno mais a profundidade abaixo da face. `None` fora da zona.
    fn zone_fold(&self, p: V3) -> Option<V3> {
        if self.ba <= 1e-9 || self.a <= 1e-9 {
            return None;
        }
        let (su, se, sn) = self.local(p);
        if su <= 1e-9 || su >= self.ba - 1e-9 || sn > 1e-6 || sn < -self.t - 1e-6 {
            return None;
        }
        let phi = su / (self.ba / self.a);
        let rad = geo3d::dist(self.o, self.c0) - sn;
        let v = geo3d::rotate(geo3d::mul(self.n, -rad), self.axis, phi);
        Some(geo3d::add(geo3d::add(self.o, v), geo3d::mul(self.ev, se)))
    }
    /// O inverso de `zone_fold`: ponto do dobrado dentro do arco da dobra volta ao planificado.
    fn zone_unfold(&self, p: V3) -> Option<V3> {
        if self.ba <= 1e-9 || self.a <= 1e-9 {
            return None;
        }
        let q = geo3d::sub(p, self.o);
        let se = geo3d::dot(q, self.ev);
        let w = geo3d::sub(q, geo3d::mul(self.ev, se));
        let rad = geo3d::norm(w);
        if rad < 1e-9 {
            return None;
        }
        let b0 = geo3d::mul(self.n, -1.);
        let phi = geo3d::dot(fenkra_kernel::cross(b0, w), self.axis).atan2(geo3d::dot(b0, w));
        let sn = geo3d::dist(self.o, self.c0) - rad;
        if phi <= 1e-9 || phi >= self.a - 1e-9 || sn > 1e-6 || sn < -self.t - 1e-6 {
            return None;
        }
        let su = phi * (self.ba / self.a);
        Some(geo3d::add(geo3d::add(self.c0, geo3d::mul(self.u, su)), geo3d::add(geo3d::mul(self.ev, se), geo3d::mul(self.n, sn))))
    }
    fn to_folded(&self, p: V3) -> V3 {
        self.rot(geo3d::sub(p, geo3d::mul(self.u, self.ba)), self.a)
    }
    /// Laje plana da chapa entre `u0` e `u1` (distância desenvolvida a partir da aresta), larga
    /// o bastante para conter a aba nas duas direções.
    fn slab(&self, u0: f64, u1: f64, up: Option<f64>) -> Result<Shape, String> {
        let m = 2. * self.t + 1.;
        // acima da face de referência (lado de dentro da dobra) a laje só pode ir até perto do eixo:
        // além dele, o giro da fatia jogaria a ferramenta para o outro lado da dobra
        let up = up.unwrap_or(m);
        let c = |uu: f64, ee: f64| geo3d::add(geo3d::add(self.c0, geo3d::mul(self.u, uu)), geo3d::add(geo3d::mul(self.ev, ee), geo3d::mul(self.n, -self.t - m)));
        let pts = [c(u0, -m), c(u1, -m), c(u1, self.l + m), c(u0, self.l + m)];
        Shape::poly_prism(&pts, geo3d::mul(self.n, self.t + m + up)).map_err(|e| e.0)
    }
    /// Região da perna reta da aba (depois da dobra) como sólido, no dobrado ou no planificado.
    fn region(&self, flat: bool) -> Result<Shape, String> {
        if flat {
            self.slab(self.ba, self.ba + self.length, None)
        } else {
            self.slab(0., self.length, None)?.transformed(Xform::Rotate(self.o, self.axis, self.a.to_degrees())).map_err(|e| e.0)
        }
    }
    /// Zona da dobra em fatias: (u médio no planificado, ângulo médio, laje planificada da fatia).
    /// A fibra neutra mantém o comprimento: u = ρ·φ, com ρ = ba/a. As lajes são mais largas que a
    /// cunha da fatia na face externa, então fatias vizinhas se sobrepõem (sem frestas).
    fn bend_slices(&self) -> Result<Vec<(f64, f64, Shape)>, String> {
        if self.ba <= 1e-9 || self.a <= 1e-9 {
            return Ok(vec![]);
        }
        let rho = self.ba / self.a;
        let r_in = geo3d::dist(self.o, self.c0);
        let n = ((self.a.to_degrees() / 3.).ceil() as usize).clamp(1, 60);
        let du = self.ba / n as f64;
        let hw = du / 2. * (r_in + self.t) / rho.max(1e-6) + 1e-3;
        (0..n)
            .map(|i| {
                let um = (i as f64 + 0.5) * du;
                Ok((um, um / rho, self.slab((um - hw).max(0.), (um + hw).min(self.ba), Some(r_in * 0.5))?))
            })
            .collect()
    }
    /// Leva um pedaço da ferramenta que está na fatia (`um`, `phi`) entre os estados da chapa: o
    /// planificado desliza até o eixo e gira o ângulo da fatia em torno do eixo da dobra.
    fn move_slice(&self, part: Shape, um: f64, phi: f64, to_flat: bool) -> Result<Shape, String> {
        let k = |e: KernelError| e.0;
        if to_flat {
            part.transformed(Xform::Rotate(self.o, self.axis, -phi.to_degrees())).map_err(k)?.transformed(Xform::Translate(geo3d::mul(self.u, um))).map_err(k)
        } else {
            part.transformed(Xform::Translate(geo3d::mul(self.u, -um))).map_err(k)?.transformed(Xform::Rotate(self.o, self.axis, phi.to_degrees())).map_err(k)
        }
    }
}

/// Resultado guardado de um comando: o que ele produziu, válido enquanto as entradas que ele lê
/// (corpo de destino, sketches, outros comandos, corpos citados, referências) não mudarem.
#[derive(Clone)]
struct FeatEntry {
    sig: u64,
    status: Status,
    /// corpos que o comando pode ter mudado (destino e corpos citados): estado depois dele
    bodies: Vec<(Id, Option<Body>)>,
    promoted: Option<Promoted>,
    sketch: Option<(Frame, Arc<EvalSketch>)>,
    tool: Option<(Shape, BoolOp)>,
    last_sheet: Option<fenkra_kernel::SheetEdgeInfo>,
    /// dobras que o comando registrou (uma aba registra uma; dobra por linha, várias)
    sheet_maps: Vec<SheetMap>,
    outlines: Vec<CutOutline>,
    roscas: Vec<RoscaViz>,
    previas: Vec<previa_leve::PreviaViz>,
    relatorios_forma: Vec<(Id, forma_brep::RelatorioForma)>,
    formas_resolvidas: Vec<forma_vinculo::FormaResolvida>,
    steps: Vec<(Shape, BoolOp)>,
}

/// Cache entre cálculos, por dependência: editar um comando refaz só ele e quem lê o que ele
/// produziu; o resto (outros corpos, outros ramos) vem pronto. Também guarda malhas.
#[derive(Default)]
pub struct Cache {
    feats: HashMap<Id, FeatEntry>,
    /// malhas dos corpos, só das assinaturas do último cálculo (`limitar`)
    meshes: HashMap<(Id, u64), Mesh>,
    /// massa e faces de cada corpo: vêm da forma exata, então sobrevivem à troca de qualidade
    props: HashMap<(Id, u64), (MassProps, u32)>,
    fills: HashMap<(Id, u64), Vec<[f32; 3]>>,
    /// geometria de referência já avaliada: (assinatura, resultado) — só refaz o que mudou
    refs: HashMap<Id, (u64, Result<RefViz, String>)>,
    /// notas das referências guardadas em `refs`
    notes: HashMap<Id, RefNote>,
    /// quantos comandos foram reaproveitados no último cálculo (diagnóstico)
    pub reused: usize,
    /// qualidade pedida para as malhas; trocar re-tessela sem refazer nenhum comando
    pub qualidade: QualidadeMalha,
    /// qualidade com que as malhas guardadas (`meshes`, `fills`, `refs`) foram feitas
    meshes_q: Option<QualidadeMalha>,
    /// Cancelar real (B59): quando a interface liga esta bandeira, o cálculo para antes do próximo
    /// comando. O que já foi feito fica no cache, então calcular de novo continua de onde parou.
    pub cancelar: Option<Arc<std::sync::atomic::AtomicBool>>,
    /// Progresso do cálculo (B259): o total de comandos ativos e quantos já terminaram, para a
    /// interface mostrar a barra enquanto a thread de cálculo trabalha.
    pub progresso: Option<fenkra_core::trabalho::Controle>,
    /// Atualizar esta feature (B59): calcula só até este comando (inclusive); os de depois ficam
    /// listados em `desatualizados`. Vale para o próximo cálculo e é limpo ao terminar.
    pub ate: Option<Id>,
    /// Comando aberto para edição (B351): a rosca modelada dele não constrói a hélice, só devolve a
    /// posição (`RecomputeResult::roscas`) para a prévia desenhar linhas. Ao concluir, o app zera
    /// isto e o cálculo seguinte faz o sulco de verdade.
    pub leve: Option<Id>,
    /// malha de tela: o que a vista pede e o que já foi feito (B346)
    pub polir: polimento::Estado,
    /// B413: comandos calculados enquanto um comando estava aberto para edição (`leve`), sem conferir a
    /// validade para não pesar a prévia. Quem for reaproveitado depois é conferido nesse momento.
    sem_validar: std::collections::HashSet<Id>,
}

impl Cache {
    pub fn com_qualidade(qualidade: QualidadeMalha) -> Self {
        Cache { qualidade, ..Default::default() }
    }

    /// Esquece as versões que o último cálculo não usou e, se o que ficou passar de `teto` bytes,
    /// solta as malhas maiores. Solta só reaproveitamento: o corpo volta a ser tesselado.
    fn limitar(&mut self, vivas: &std::collections::HashSet<(Id, u64)>, fills_vivos: &std::collections::HashSet<(Id, u64)>, teto: usize) {
        self.meshes.retain(|k, _| vivas.contains(k));
        self.props.retain(|k, _| vivas.contains(k));
        self.fills.retain(|k, _| fills_vivos.contains(k));
        let fills: usize = self.fills.values().map(|f| std::mem::size_of_val(f.as_slice())).sum();
        let mut total = fills + self.meshes.values().map(Mesh::bytes).sum::<usize>();
        if total <= teto {
            return;
        }
        let mut por_tamanho: Vec<_> = self.meshes.iter().map(|(k, m)| (m.bytes(), *k)).collect();
        por_tamanho.sort_unstable_by_key(|(b, _)| std::cmp::Reverse(*b));
        for (bytes, k) in por_tamanho {
            if total <= teto {
                return;
            }
            self.meshes.remove(&k);
            total -= bytes;
        }
        if total > teto {
            self.fills.clear();
        }
    }
}

/// Teto de memória das malhas guardadas entre cálculos. Só as versões vivas ficam no cache, então
/// ele age em documentos muito grandes, trocando memória por tesselar de novo.
const TETO_CACHE_MALHAS: usize = 512 * 1024 * 1024;

/// Valor exibível de uma referência (guardado no cache).
#[derive(Clone)]
enum RefViz {
    Plane(Frame),
    Point([f64; 3]),
    Curve(Vec<[f64; 3]>),
    Surface(Mesh),
}

/// Assinatura só da geometria de um comando: nome, visibilidade e aparência não mudam o sólido.
fn feature_sig(f: &Feature) -> String {
    let mut g = f.clone();
    g.name.clear();
    g.hidden = false;
    g.appearance = None;
    serde_json::to_string(&g).unwrap_or_default()
}

/// A referência (ou alguma entrada dela) lê faces/corpos? Então muda quando a linha do tempo muda.
fn ref_reads_bodies(doc: &Document, id: Id, depth: u32) -> bool {
    let Some(r) = doc.reference(id) else { return false };
    if depth > 24 {
        return true;
    }
    let js = serde_json::to_string(&r.kind).unwrap_or_default();
    if ["\"Face\"", "SurfExtract", "SurfMulti", "LineAxis", "\"Plane\"", "FacePlane", "\"Edge\"", "BodyEdge", "\"Vertex\""].iter().any(|k| js.contains(k)) {
        return true;
    }
    let mut ids = vec![];
    r.kind.clone().walk(&mut |x| ids.push(*x), &mut |_| {});
    ids.into_iter().any(|x| x != id && ref_reads_bodies(doc, x, depth + 1))
}

/// O texto cita o nome (palavra inteira)?
fn cites(text: &str, name: &str) -> bool {
    let word = |c: char| c.is_alphanumeric() || c == '_';
    text.match_indices(name).any(|(i, _)| {
        let before = text[..i].chars().next_back();
        let after = text[i + name.len()..].chars().next();
        !before.is_some_and(word) && !after.is_some_and(word)
    })
}

/// Valores dos parâmetros citados no texto (só eles entram na assinatura de quem os usa).
fn vars_sig(text: &str, vars: &Vars) -> String {
    let mut v: Vec<(&String, u64)> = vars.iter().filter(|(k, _)| cites(text, k)).map(|(k, v)| (k, v.to_bits())).collect();
    v.sort();
    format!("{v:?}")
}

/// Assinatura de uma referência: a definição dela, das entradas dela e dos parâmetros que usam.
fn ref_sig(doc: &Document, id: Id, vars: &Vars, depth: u32) -> String {
    let Some(r) = doc.reference(id) else { return format!("?{}", id.0) };
    let js = serde_json::to_string(&r.kind).unwrap_or_default();
    let mut out = format!("{}{}{}", id.0, js, vars_sig(&js, vars));
    if depth < 24 {
        let mut ids = vec![];
        r.kind.clone().walk(&mut |x| ids.push(*x), &mut |_| {});
        for x in ids.into_iter().filter(|x| *x != id) {
            out.push_str(&ref_sig(doc, x, vars, depth + 1));
        }
    }
    out
}

fn hash_of(parts: &[&str], prev: u64) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    prev.hash(&mut h);
    for p in parts {
        p.hash(&mut h);
    }
    h.finish()
}

/// Pontos mais próximos entre os segmentos ab e cd.
fn seg_closest(a: V3, b: V3, c: V3, d: V3) -> (V3, V3) {
    use geo3d::*;
    let (u, v, w) = (sub(b, a), sub(d, c), sub(a, c));
    let (aa, bb, cc, dd, ee) = (dot(u, u), dot(u, v), dot(v, v), dot(u, w), dot(v, w));
    let den = aa * cc - bb * bb;
    let mut s = if den > 1e-12 { ((bb * ee - cc * dd) / den).clamp(0., 1.) } else { 0. };
    let mut t = if cc > 1e-12 { ((bb * s + ee) / cc).clamp(0., 1.) } else { 0. };
    if aa > 1e-12 {
        s = ((bb * t - dd) / aa).clamp(0., 1.);
    }
    if cc > 1e-12 {
        t = ((bb * s + ee) / cc).clamp(0., 1.);
    }
    (add(a, mul(u, s)), add(c, mul(v, t)))
}

/// B-spline cúbica uniforme pelo polígono de controle (aberta: presa nas pontas).
fn bspline(ctrl: &[V3], closed: bool) -> Result<Curve3, String> {
    use geo3d::*;
    if ctrl.len() < 3 {
        return Err("o polígono de controle precisa de pelo menos 3 pontos".into());
    }
    let mut p: Vec<V3> = ctrl.to_vec();
    if closed {
        p.extend_from_slice(&ctrl[..3]);
    } else {
        p.insert(0, ctrl[0]);
        p.insert(0, ctrl[0]);
        p.push(*ctrl.last().unwrap());
        p.push(*ctrl.last().unwrap());
    }
    let mut out = vec![];
    for i in 0..p.len() - 3 {
        for k in 0..24 {
            let t = k as f64 / 24.;
            let (t2, t3) = (t * t, t * t * t);
            let b0 = (1. - t).powi(3) / 6.;
            let b1 = (3. * t3 - 6. * t2 + 4.) / 6.;
            let b2 = (-3. * t3 + 3. * t2 + 3. * t + 1.) / 6.;
            let b3 = t3 / 6.;
            out.push(add(add(mul(p[i], b0), mul(p[i + 1], b1)), add(mul(p[i + 2], b2), mul(p[i + 3], b3))));
        }
    }
    if closed {
        out.push(out[0]);
    } else {
        out.push(*ctrl.last().unwrap());
    }
    Curve3::poly(out, closed)
}

impl Ctx<'_> {
    fn plane(&self, p: &PlaneRef, depth: u32) -> Result<Frame, String> {
        match p {
            PlaneRef::Origin(o) => Ok(origin_frame(*o)),
            PlaneRef::Datum(id) => match self.geo(*id, depth + 1)? {
                Geo::Plane(f) => Ok(f),
                _ => Err(format!("'{}' não é um plano", self.doc.name_of(*id))),
            },
        }
    }

    /// Eixo (ponto, direção unitária).
    fn axis(&self, a: &AxisRef) -> Result<([f64; 3], [f64; 3]), String> {
        self.axis_d(a, 0)
    }

    fn axis_d(&self, a: &AxisRef, depth: u32) -> Result<([f64; 3], [f64; 3]), String> {
        match *a {
            AxisRef::Origin(ax) => Ok(([0.; 3], origin_axis(ax))),
            AxisRef::SketchLine { sketch, entity } => {
                let (af, ask) = &self.sketch(sketch)?;
                let (a, b) = ask.lines.get(&entity).ok_or("linha do eixo não existe mais — reedite o eixo")?;
                let (pa, pb) = (af.point(a[0], a[1]), af.point(b[0], b[1]));
                Ok((pa, unit([pb[0] - pa[0], pb[1] - pa[1], pb[2] - pa[2]])))
            }
            AxisRef::Ref(id) => {
                let c = self.curve(&CurveRef::Ref(id), depth + 1)?;
                let d = if c.is_straight() { geo3d::sub(c.end(), c.start()) } else { c.at_dist(0.).1 };
                Ok((c.start(), geo3d::unit(d).ok_or("eixo de comprimento zero")?))
            }
            AxisRef::Fixed { origin, dir } => Ok((origin, geo3d::unit(dir).ok_or("eixo de comprimento zero")?)),
            AxisRef::Face { point, key } => self.face_axis(point, key.as_ref()).map(|(o, v, _)| (o, v)),
            AxisRef::Edge { mid, key } => {
                let pts = self.live_edge(mid, key.as_ref())?;
                // aresta circular: o eixo do círculo, não a corda
                if let Some((centro, normal, _)) = geo3d::circle_fit(&pts) {
                    return Ok((centro, normal));
                }
                let c = Curve3::poly(pts, false)?;
                if !c.is_straight() {
                    return Err("a aresta do eixo não é reta nem circular".into());
                }
                Ok((c.start(), geo3d::unit(geo3d::sub(c.end(), c.start())).ok_or("eixo de comprimento zero")?))
            }
            AxisRef::Planes { a, b } => {
                use geo3d::*;
                let (fa, fb) = (self.plane(&a, depth + 1)?, self.plane(&b, depth + 1)?);
                let u = cross(fa.normal, fb.normal);
                let uu = dot(u, u);
                if uu < 1e-12 {
                    return Err("os planos são paralelos".into());
                }
                let (da, db) = (dot(fa.normal, fa.origin), dot(fb.normal, fb.origin));
                Ok((mul(add(mul(cross(fb.normal, u), da), mul(cross(u, fa.normal), db)), 1. / uu), geo3d::unit(u).unwrap()))
            }
        }
    }

    /// Ponto da face viva: pela chave (dono + papel), ou pelo ponto, que tem de estar sobre a face.
    fn live_face(&self, p: V3, key: Option<&FaceKey>) -> Result<V3, String> {
        let Some(k) = key else {
            let on = self.bodies.values().any(|(b, _, _)| b.surface_near(p).is_ok_and(|(q, _)| geo3d::dist(q, p) < 1e-3));
            return if on { Ok(p) } else { Err("a face não existe mais no modelo — reedite".into()) };
        };
        let mut lost = refs::Lost::None;
        for (b, owners, roles) in self.bodies.values() {
            let topo = b.topology().map_err(|e| e.0)?;
            match refs::find_face(&refs::View { topo: &topo, owners, roles }, k, Some(p)) {
                Ok(j) => return Ok(topo.faces[j].point),
                Err(refs::Lost::None) => {}
                Err(l) => lost = l,
            }
        }
        let who = format!("a face de '{}' ({})", self.doc.name_of(k.owner), refs::role_name(k.role));
        Err(match lost {
            refs::Lost::Ambiguous(n) => format!("referência perdida: {who} tem {n} candidatas; escolha de novo"),
            refs::Lost::None => format!("referência perdida: {who} não existe mais; escolha de novo"),
        })
    }

    /// Polilinha atual da aresta: pela chave, ou pelo ponto médio.
    fn live_edge(&self, mid: V3, key: Option<&EdgeKey>) -> Result<Vec<V3>, String> {
        let mut lost = refs::Lost::None;
        for (b, owners, roles) in self.bodies.values() {
            let topo = b.topology().map_err(|e| e.0)?;
            let v = refs::View { topo: &topo, owners, roles };
            let found = match key {
                Some(k) => refs::find_edge(&v, k, Some(mid)),
                None => refs::edge_near(&v, mid, 0.1).ok_or(refs::Lost::None),
            };
            match found {
                Ok(e) => return b.edge_at(topo.edge_mids[e]).map_err(|e| e.0),
                Err(refs::Lost::None) => {}
                Err(l) => lost = l,
            }
        }
        Err(match lost {
            refs::Lost::Ambiguous(n) => format!("referência perdida: a aresta tem {n} candidatas; escolha de novo"),
            refs::Lost::None => match key {
                Some(k) => format!("referência perdida: a aresta entre '{}' ({}) e '{}' ({}) não existe mais; escolha de novo", self.doc.name_of(k.a.owner), refs::role_name(k.a.role), self.doc.name_of(k.b.owner), refs::role_name(k.b.role)),
                None => "referência perdida: a aresta não existe mais; escolha de novo".into(),
            },
        })
    }

    /// Eixo da face em `p`: origem, direção e altura.
    fn face_axis(&self, p: V3, key: Option<&FaceKey>) -> Result<(V3, V3, f64), String> {
        use geo3d::*;
        let q = self.live_face(p, key)?;
        for (b, _, _) in self.bodies.values() {
            if let Ok(cy) = b.cylinder_at(q) {
                return Ok((add(cy.origin, mul(cy.dir, cy.h0)), cy.dir, (cy.h1 - cy.h0).max(1.)));
            }
            if let Ok(fp) = b.face_props(q) {
                if fp.kind == 2 {
                    return Ok((fp.origin, fp.dir, 2. * fp.radius.max(5.)));
                }
            }
        }
        // face plana: a normal dela, saindo do ponto escolhido
        if key.is_some() {
            for (b, _, _) in self.bodies.values() {
                if let Ok((s, n)) = b.surface_near(q) {
                    if dist(s, q) < 1e-3 {
                        return Ok((self.on_face(p, key)?, n, 20.));
                    }
                }
            }
        }
        Err("a face escolhida não é plana, cilíndrica nem cônica".into())
    }

    /// Ponto clicado numa face: leva `p` pela normal até a face viva e o assenta nela. Assim
    /// acompanha a face quando ela anda, e referência perdida vira erro como em `live_face`.
    fn on_face(&self, p: V3, key: Option<&FaceKey>) -> Result<V3, String> {
        use geo3d::*;
        let q = self.live_face(p, key)?;
        for (b, _, _) in self.bodies.values() {
            let Ok((s, n)) = b.surface_near(q) else { continue };
            if dist(s, q) > 1e-3 {
                continue;
            }
            let levado = add(p, mul(n, dot(sub(q, p), n)));
            return Ok(b.surface_near(levado).map_or(levado, |x| x.0));
        }
        Ok(q)
    }

    /// Origem do plano de face. O plano segue a face; a face que sumiu vira erro. Sem chave vale
    /// qualquer face do dono no mesmo plano: a face pode ter sido dividida ou coberta no ponto
    /// clicado por um comando depois do sketch, e isso não a faz sumir.
    fn face_plane_origin(&self, origin: V3, n: V3, owner: Option<Id>, key: Option<&FaceKey>) -> Result<V3, String> {
        use geo3d::*;
        let Some(who) = owner.or(key.map(|k| k.owner)) else { return Ok(origin) };
        for (b, owners, roles) in self.bodies.values() {
            let topo = b.topology().map_err(|e| e.0)?;
            for (j, f) in topo.faces.iter().enumerate() {
                let same = match key {
                    Some(k) => owners.get(j) == Some(&k.owner) && roles.get(j) == Some(&k.role),
                    None => owners.get(j) == Some(&who),
                };
                if !same {
                    continue;
                }
                if f.kind == 0 {
                    let h = dot(sub(f.point, origin), n);
                    if dot(f.dir, n).abs() > 0.99 && (key.is_some() || h.abs() < 1e-3) {
                        return Ok(add(origin, mul(n, h)));
                    }
                } else if key.is_some() || b.surface_near(origin).is_ok_and(|(q, _)| dist(q, origin) < 1e-3) {
                    return Ok(origin);
                }
            }
        }
        Err(format!("referência perdida: o plano ficava numa face de '{}' que não existe mais; escolha outra face", self.doc.name_of(who)))
    }

    fn point(&self, p: &PointRef, depth: u32) -> Result<V3, String> {
        match *p {
            PointRef::Origin => Ok([0.; 3]),
            PointRef::Fixed(q) => Ok(q),
            PointRef::Ref(id) => match self.geo(id, depth + 1)? {
                Geo::Point(q) => Ok(q),
                _ => Err(format!("'{}' não é um ponto", self.doc.name_of(id))),
            },
            PointRef::SketchPoint { sketch, point } => {
                let (fr, es) = &self.sketch(sketch)?;
                let q = es.pts.get(&point).ok_or("o ponto do sketch não existe mais — reedite")?;
                Ok(fr.point(q[0], q[1]))
            }
            PointRef::Vertex { point, edge } => {
                let c = self.live_edge(point, Some(&edge))?;
                let (a, b) = (c[0], *c.last().unwrap());
                Ok(if geo3d::dist(a, point) <= geo3d::dist(b, point) { a } else { b })
            }
            PointRef::Center { point, edge } => {
                let c = self.live_edge(point, Some(&edge))?;
                geo3d::circle_fit(&c).map(|x| x.0).ok_or_else(|| "a aresta do centro não é mais circular".to_string())
            }
            PointRef::OnFace { point, key } => self.on_face(point, Some(&key)),
            PointRef::CurveVertex { curve, index } => {
                if depth > 32 {
                    return Err("referência circular".into());
                }
                // polilinhas/splines: o ponto de definição; demais: início, fim ou meio da curva
                if let Some(r) = self.doc.reference(curve) {
                    if let RefKind::Polyline { points, .. } | RefKind::Spline { points, .. } | RefKind::SplineControl { points, .. } = &r.kind {
                        let p = points.get(index).ok_or("o vértice da curva não existe mais")?;
                        return self.point(p, depth + 1);
                    }
                    // curva ligada ou isolada: a cópia guarda os vértices publicados, na ordem; sem este
                    // ramo o vértice caía no início/fim/meio e o trecho saía com comprimento zero
                    if let RefKind::Linked { .. } | RefKind::Isolated { .. } = &r.kind {
                        let c = self.curve(&CurveRef::Ref(curve), depth + 1)?;
                        return c.pts.get(index).copied().ok_or_else(|| format!("a curva '{}' tem {} vértices; o trecho pede o vértice {}", r.name, c.pts.len(), index + 1));
                    }
                }
                let c = self.curve(&CurveRef::Ref(curve), depth + 1)?;
                Ok(match index {
                    0 => c.start(),
                    1 => c.end(),
                    _ => c.pts.get(c.pts.len() / 2).copied().unwrap_or(c.start()),
                })
            }
        }
    }

    fn points(&self, ps: &[PointRef], depth: u32) -> Result<Vec<V3>, String> {
        ps.iter().map(|p| self.point(p, depth)).collect()
    }

    fn curve(&self, c: &CurveRef, depth: u32) -> Result<Curve3, String> {
        match c {
            CurveRef::Axis(a) => {
                let d = origin_axis(*a);
                Curve3::line(geo3d::mul(d, -500.), geo3d::mul(d, 500.))
            }
            CurveRef::Edge(pts) => {
                let closed = pts.len() > 2 && geo3d::dist(pts[0], *pts.last().unwrap()) < 1e-6;
                Curve3::poly(pts.clone(), closed)
            }
            CurveRef::BodyEdge { pts, key } => {
                let p = self.live_edge(pts[pts.len() / 2], Some(key))?;
                let closed = p.len() > 2 && geo3d::dist(p[0], *p.last().unwrap()) < 1e-6;
                Curve3::poly(p, closed)
            }
            CurveRef::SketchLine { sketch, entity } => {
                let (af, ask) = &self.sketch(*sketch)?;
                let (a, b) = ask.lines.get(entity).ok_or("a linha do sketch não existe mais — reedite")?;
                Curve3::line(af.point(a[0], a[1]), af.point(b[0], b[1]))
            }
            CurveRef::Ref(id) => match self.geo(*id, depth + 1)? {
                Geo::Curve(c) => Ok(c),
                _ => Err(format!("'{}' não é uma linha ou curva", self.doc.name_of(*id))),
            },
            CurveRef::SketchCurve { sketch, curve } => {
                let (fr, es) = &self.sketch(*sketch)?;
                let pts = es.curves.get(curve).ok_or("a curva do sketch não existe mais — reedite")?;
                let p3: Vec<V3> = pts.iter().map(|q| fr.point(q[0], q[1])).collect();
                if p3.len() == 2 {
                    return Curve3::line(p3[0], p3[1]);
                }
                let closed = p3.len() > 2 && geo3d::dist(p3[0], *p3.last().unwrap()) < 1e-6;
                let p3 = if closed { p3[..p3.len() - 1].to_vec() } else { p3 };
                Curve3::poly(p3, closed)
            }
            CurveRef::Segment { curve, index } => {
                let a = self.point(&PointRef::CurveVertex { curve: *curve, index: *index }, depth + 1)?;
                let b = self.point(&PointRef::CurveVertex { curve: *curve, index: *index + 1 }, depth + 1)?;
                Curve3::line(a, b)
            }
        }
    }

    /// Ponto da superfície mais próximo de `p` (projeção no plano tangente) e a normal.
    fn surf_at(&self, sf: &SurfRef, p: V3, depth: u32) -> Result<(V3, V3), String> {
        let (o, n) = match sf {
            SurfRef::Plane(pr) => {
                let f = self.plane(pr, depth)?;
                (f.origin, f.normal)
            }
            SurfRef::Face { key: Some(_), .. } => return self.surface_shape(sf, depth)?.surface_near(p).map_err(|e| e.0),
            SurfRef::Face { point, normal, .. } => (*point, geo3d::unit(*normal).ok_or("normal da face nula")?),
            SurfRef::Ref(id) => return self.surface(*id, depth)?.surface_near(p).map_err(|e| e.0),
        };
        Ok((geo3d::sub(p, geo3d::mul(n, geo3d::dot(geo3d::sub(p, o), n))), n))
    }

    fn surface(&self, id: Id, depth: u32) -> Result<Shape, String> {
        match self.geo(id, depth + 1)? {
            Geo::Surface(s) => Ok(s),
            _ => Err(format!("'{}' não é uma superfície", self.doc.name_of(id))),
        }
    }

    /// Forma da superfície de entrada (face do corpo, superfície de referência).
    fn surface_shape(&self, sf: &SurfRef, depth: u32) -> Result<Shape, String> {
        match sf {
            SurfRef::Ref(id) => self.surface(*id, depth),
            SurfRef::Face { point, key, .. } => self.face_of_bodies(self.live_face(*point, key.as_ref())?),
            SurfRef::Plane(_) => Err("use 'Plano deslocado' para deslocar um plano".into()),
        }
    }

    /// Tamanho que cobre o modelo inteiro (para planos usados como superfície de corte).
    fn model_size(&self) -> f64 {
        let mut m = 500.;
        for (b, _, _) in self.bodies.values() {
            if let Some((a, c)) = b.bbox() {
                m = f64::max(m, geo3d::dist(a, c) * 2. + 100.);
            }
        }
        m
    }

    /// Superfície de entrada como forma: plano vira face plana grande; face/superfície como estão.
    fn cut_shape(&self, sf: &SurfRef, depth: u32) -> Result<Shape, String> {
        match sf {
            SurfRef::Plane(p) => {
                let fr = self.plane(p, depth)?;
                Shape::plane_face(fr.origin, fr.normal, self.model_size()).map_err(|e| e.0)
            }
            _ => self.surface_shape(sf, depth),
        }
    }

    /// Transformação de geometria em coordenadas do kernel.
    fn xform(&self, x: &GeoXform, d: u32, e: &dyn Fn(&String, &str) -> Result<f64, String>) -> Result<Xform, String> {
        Ok(match x {
            GeoXform::Translate { dir, dist } => {
                let (_, v) = self.axis_d(dir, d)?;
                Xform::Translate(geo3d::mul(geo3d::unit(v).ok_or("direção nula")?, e(dist, "distância")?))
            }
            GeoXform::Rotate { axis, angle } => {
                let (o, v) = self.axis_d(axis, d)?;
                Xform::Rotate(o, v, e(angle, "ângulo")?)
            }
            GeoXform::Mirror { plane } => {
                let f = self.plane(plane, d)?;
                Xform::Mirror(f.origin, f.normal)
            }
            GeoXform::Scale { center, factor } => Xform::Scale(self.point(center, d)?, e(factor, "fator")?),
        })
    }

    /// Concordância entre duas bordas: seções de Hermite (G1 cúbica, G2 quíntica com curvatura
    /// nula nas pontas) que saem tangentes às superfícies de apoio; G0 liga em linha reta.
    #[allow(clippy::too_many_arguments)]
    fn blend(&self, id: Id, a: &CurveRef, sa: Option<&SurfRef>, b: &CurveRef, sb: Option<&SurfRef>, cont: Continuity, tensoes: [f64; 2], d: u32) -> Result<Shape, String> {
        use geo3d::*;
        const N: usize = 24;
        let (ca, cb) = (self.curve(a, d)?, self.curve(b, d)?);
        let pa = resample(&ca, N);
        let mut pb = resample(&cb, N);
        // mesmo sentido nas duas bordas
        if dist(pa[0], pb[0]) + dist(pa[N - 1], pb[N - 1]) > dist(pa[0], pb[N - 1]) + dist(pa[N - 1], pb[0]) {
            pb.reverse();
        }
        let sha = match sa {
            Some(s) => Some(self.cut_shape(s, d)?),
            None => None,
        };
        let shb = match sb {
            Some(s) => Some(self.cut_shape(s, d)?),
            None => None,
        };
        let tan = |v: &[V3], i: usize| unit(sub(v[(i + 1).min(N - 1)], v[i.saturating_sub(1)])).unwrap_or([1., 0., 0.]);
        // direção que sai da borda, no plano tangente da superfície, apontando para o outro lado
        let out = |sh: &Option<Shape>, v: &[V3], i: usize, toward: V3| -> Result<Option<V3>, String> {
            let Some(sh) = sh else { return Ok(None) };
            let (_, n) = sh.surface_near(v[i]).map_err(|x| x.0)?;
            let mut w = unit(cross(n, tan(v, i))).ok_or("borda tangente à normal")?;
            if dot(w, sub(toward, v[i])) < 0. {
                w = mul(w, -1.);
            }
            Ok(Some(w))
        };
        let mut sections = vec![];
        for i in 0..N {
            let chord = dist(pa[i], pb[i]);
            if chord < 1e-6 {
                return Err("as bordas se tocam — a concordância precisa de afastamento".into());
            }
            let sec = if cont == Continuity::G0 {
                // sem seta, mas a etiqueta de continuidade precisa saber onde fica a ponte
                if i == N / 2 {
                    let d = unit(sub(pb[i], pa[i])).unwrap_or([1., 0., 0.]);
                    self.notes.borrow_mut().entry(id).or_default().tensoes = vec![(pa[i], d, chord), (pb[i], mul(d, -1.), chord)];
                }
                vec![pa[i], pb[i]]
            } else {
                let da = out(&sha, &pa, i, pb[i])?.unwrap_or_else(|| unit(sub(pb[i], pa[i])).unwrap());
                let db = out(&shb, &pb, i, pa[i])?.unwrap_or_else(|| unit(sub(pa[i], pb[i])).unwrap());
                if i == N / 2 {
                    self.notes.borrow_mut().entry(id).or_default().tensoes = vec![(pa[i], da, chord), (pb[i], db, chord)];
                }
                let pts = hermite(pa[i], mul(da, chord * tensoes[0]), [0.; 3], pb[i], mul(db, -chord * tensoes[1]), [0.; 3], matches!(cont, Continuity::G2 | Continuity::G3));
                pts.into_iter().step_by(4).chain(std::iter::once(pb[i])).collect::<Vec<_>>()
            };
            sections.push(sec);
        }
        // as seções vão de uma borda à outra; o loft liga uma seção à seguinte
        Shape::loft_surface(&sections, cont == Continuity::G0, false, None).map_err(|x| x.0)
    }

    /// Prolonga a borda em `p` com uma faixa regrada na normal à borda, no plano tangente da superfície.
    fn extend_normal(&self, sh: &Shape, p: V3, len: f64) -> Result<Shape, String> {
        use geo3d::*;
        if len <= 0. {
            return Err("na normal à borda o comprimento precisa ser positivo".into());
        }
        let b = Curve3::poly(sh.edge_at(p).map_err(|x| x.0)?, false)?;
        let inner = sh.mass().center;
        let out: Vec<V3> = (0..b.pts.len())
            .map(|i| {
                let (q, t) = (b.pts[i], b.at_dist(b.nearest(b.pts[i]).0).1);
                let (_, n) = sh.surface_near(q).map_err(|x| x.0)?;
                let mut w = unit(cross(n, t)).ok_or("borda tangente à normal")?;
                if dot(w, sub(q, inner)) < 0. {
                    w = mul(w, -1.);
                }
                Ok(add(q, mul(w, len)))
            })
            .collect::<Result<_, String>>()?;
        let strip = Shape::loft_surface(&[b.pts.clone(), out], true, false, None).map_err(|x| x.0)?;
        sh.compound(&strip).sewn(1e-3).map_err(|x| x.0)
    }

    /// Curva entre as soluções: a mais perto da âncora ou a do número. O número fora da lista é erro,
    /// nunca "a última". Quem foi pelo número deixa o ponto da curva para virar âncora: a ordem das
    /// soluções é do kernel e muda quando o modelo muda.
    fn pick_curve(&self, id: Id, mut cs: Vec<Vec<V3>>, which: usize, near: Option<V3>) -> Result<Vec<V3>, String> {
        let n = cs.len();
        let pontos: Vec<(usize, V3)> = cs.iter().enumerate().filter(|(_, c)| !c.is_empty()).map(|(i, c)| (i, c[c.len() / 2])).collect();
        if let Some(p) = near {
            let gap = |c: &Vec<V3>| Curve3::poly(c.clone(), false).map_or(f64::MAX, |k| geo3d::dist(k.nearest(p).1, p));
            let i = (0..n).min_by(|a, b| gap(&cs[*a]).total_cmp(&gap(&cs[*b]))).ok_or("sem soluções")?;
            self.publicar_solucoes(id, pontos, Some(i));
            return Ok(cs.swap_remove(i));
        }
        self.publicar_solucoes(id, pontos, (which < n).then_some(which));
        if which >= n {
            return Err(format!("a solução {which} não existe: há {n} (de 0 a {})", n - 1));
        }
        let c = cs.swap_remove(which);
        if n > 1 {
            self.notes.borrow_mut().entry(id).or_default().anchor = Some(c[c.len() / 2]);
        }
        Ok(c)
    }

    /// Soluções possíveis da referência: a tela as mostra como bolinhas. Só faz sentido com mais de uma.
    fn publicar_solucoes(&self, id: Id, pontos: Vec<(usize, V3)>, atual: Option<usize>) {
        if pontos.len() > 1 {
            let mut n = self.notes.borrow_mut();
            let nota = n.entry(id).or_default();
            nota.solucoes = pontos;
            nota.solucao = atual;
        }
    }

    /// Extração refeita a partir do ponto clicado, com a propagação gravada.
    fn extract_faces(&self, p: V3, prop: Propagation, complement: bool) -> Result<(Shape, u32, u32), String> {
        let code = match prop {
            Propagation::Nenhuma => 0,
            Propagation::Ponto => 1,
            Propagation::Tangente => 2,
            Propagation::Completo => 3,
        };
        for (b, _, _) in self.bodies.values() {
            if b.surface_near(p).is_ok_and(|(q, _)| geo3d::dist(q, p) < 1e-3) {
                return b.face_propagate(p, code, complement).map_err(|e| e.0);
            }
        }
        Err("a face não existe mais no modelo — reedite".into())
    }

    /// Contorno a partir da aresta mais perto do ponto: nenhuma = só ela; ponto = o laço dela;
    /// tangente = o trecho sem canto vivo; completo = a borda inteira, que precisa ser um laço só.
    fn boundary_from(&self, sh: &Shape, p: V3, prop: Propagation) -> Result<(Curve3, u32), String> {
        const TOL: f64 = 1e-3;
        let es = sh.free_edges().map_err(|x| x.0)?;
        let cs: Vec<Curve3> = es.into_iter().map(|e| Curve3::poly(e, false)).collect::<Result<_, _>>()?;
        let gap = |c: &Curve3| geo3d::dist(c.nearest(p).1, p);
        let start = (0..cs.len()).min_by(|a, b| gap(&cs[*a]).total_cmp(&gap(&cs[*b]))).ok_or("sem bordas")?;
        let set = match prop {
            Propagation::Nenhuma => vec![start],
            Propagation::Ponto => geo3d::chain_indices(&cs, start, TOL),
            Propagation::Tangente => geo3d::tangent_chain(&cs, start, TOL),
            Propagation::Completo => {
                let laco = geo3d::chain_indices(&cs, start, TOL);
                if laco.len() != cs.len() {
                    return Err(format!("a borda tem {} arestas fora do laço clicado: uma curva só não cobre a borda inteira, use 'ponto'", cs.len() - laco.len()));
                }
                laco
            }
        };
        let n = set.len() as u32;
        Ok((join_curves(set.into_iter().map(|i| cs[i].clone()).collect())?, n))
    }

    /// Guarda o que a extração alcançou; se mudou desde que foi feita, vira aviso.
    fn count_note(&self, id: Id, n: u32, edges: u32, expected: Option<u32>, what: &str) {
        let mut note = RefNote { count: Some((n, edges)), ..Default::default() };
        if let Some(e) = expected.filter(|e| *e != n) {
            note.warn = true;
            note.text = format!("a extração tinha {e} {what} e agora tem {n}: confira a propagação");
        }
        self.notes.borrow_mut().insert(id, note);
    }

    /// Preenchimento: confere o contorno, fecha as folgas dentro da tolerância e monta as condições.
    fn fill(&self, me: Id, curves: &[CurveRef], conds: &[FillCond], points: &[PointRef], tol: f64, d: u32, e: &dyn Fn(&String, &str) -> Result<f64, String>) -> Result<Shape, String> {
        let mut cs: Vec<Curve3> = curves.iter().map(|c| self.curve(c, d)).collect::<Result<_, _>>()?;
        let rep = geo3d::contour_report(&cs, tol);
        self.notes.borrow_mut().insert(me, RefNote { contour: Some(rep.clone()), ..Default::default() });
        if !rep.open_ends.is_empty() {
            return Err(format!("contorno aberto: {} ponta(s) solta(s), maior folga {:.3} mm (tolerância {:.3} mm)", rep.open_ends.len(), rep.max_gap, tol));
        }
        if rep.loops > 1 {
            return Err(format!("as curvas formam {} contornos separados: o preenchimento fecha um só", rep.loops));
        }
        geo3d::snap_contour(&mut cs, &rep);
        let (mut orders, mut weights, mut support) = (vec![], vec![], Shape::empty());
        for (i, c) in cs.iter().enumerate() {
            let cond = conds.get(i).cloned().unwrap_or_default();
            let order = match cond.cont {
                Continuity::G0 => 0,
                Continuity::G1 => 1,
                Continuity::G2 | Continuity::G3 => 2,
            };
            let w = e(&cond.weight, "peso")?;
            if w < 1. {
                return Err(format!("o peso da curva {} precisa ser 1 ou mais", i + 1));
            }
            orders.push(order);
            weights.push(w);
            if order > 0 {
                let s = match &cond.support {
                    Some(s) => self.cut_shape(s, d)?,
                    None => self.fill_support(me, c, i, d)?,
                };
                support = if support.is_null() { s } else { support.compound(&s) };
            }
        }
        let pts: Vec<Vec<V3>> = cs.into_iter().map(|c| c.pts).collect();
        let (sh, _, g1, _) = Shape::fill_surface_x(&pts, &orders, &weights, &support, &self.points(points, d)?).map_err(|x| x.0)?;
        if orders.iter().any(|o| *o > 0) {
            let deg = g1.to_degrees();
            let mut n = self.notes.borrow_mut();
            let note = n.entry(me).or_default();
            note.g1_deg = Some(deg);
            if deg > 0.5 {
                note.warn = true;
                note.text = format!("tangência no contorno com erro de {deg:.2} graus (limite 0,5)");
            }
        }
        Ok(sh)
    }

    /// Face vizinha de uma curva sem apoio escolhido: a superfície de referência em que ela está. Se
    /// houver mais de uma, quem decide é a pessoa.
    fn fill_support(&self, me: Id, c: &Curve3, i: usize, d: u32) -> Result<Shape, String> {
        let deps = self.doc.dependents(me);
        let on = |sh: &Shape| {
            [0., 0.5, 1.].iter().all(|t| {
                let p = c.at_ratio(*t).0;
                sh.surface_near(p).is_ok_and(|(q, _)| geo3d::dist(p, q) < 0.15)
            })
        };
        let mut found: Vec<Shape> = self
            .doc
            .references
            .iter()
            .filter(|r| r.id != me && !deps.contains(&r.id) && r.kind.class() == RefClass::Surface)
            .filter_map(|r| self.surface(r.id, d).ok())
            .filter(on)
            .collect();
        match found.len() {
            0 => Err(format!("a curva {} pede uma face de apoio: escolha a face vizinha", i + 1)),
            1 => Ok(found.remove(0)),
            n => Err(format!("a curva {} encosta em {n} superfícies: escolha a face de apoio", i + 1)),
        }
    }

    /// Cópia da face de algum corpo que passa pelo ponto.
    fn face_of_bodies(&self, p: V3) -> Result<Shape, String> {
        for (b, _, _) in self.bodies.values() {
            if let Ok((q, _)) = b.surface_near(p) {
                if geo3d::dist(q, p) < 1e-3 {
                    return b.face_copy(p).map_err(|e| e.0);
                }
            }
        }
        Err("a face não existe mais no modelo — reedite".into())
    }

    fn surf_frame(&self, sf: &SurfRef, depth: u32) -> Result<Frame, String> {
        match sf {
            SurfRef::Plane(pr) => self.plane(pr, depth),
            SurfRef::Face { point, normal, key: None, .. } => geo3d::frame_from(*point, *normal, None),
            SurfRef::Face { point, key: Some(_), .. } => {
                let (q, n) = self.surface_shape(sf, depth)?.surface_near(*point).map_err(|e| e.0)?;
                geo3d::frame_from(q, n, None)
            }
            SurfRef::Ref(id) => {
                let sh = self.surface(*id, depth)?;
                let (q, n) = sh.surface_near(sh.mass().center).map_err(|e| e.0)?;
                geo3d::frame_from(q, n, None)
            }
        }
    }

    /// Ponto e tangente numa posição da curva.
    fn on_curve(&self, c: &Curve3, at: &OnCurve, reverse: bool, name: &str) -> Result<(V3, V3), String> {
        let len = c.length();
        let mut s = match at {
            OnCurve::Distance(e) => ev(e, &self.vars, &format!("{name} · distância"))?,
            OnCurve::Ratio(e) => ev(e, &self.vars, &format!("{name} · proporção"))? * len,
        };
        if s < -1e-9 || s > len + 1e-9 {
            return Err(format!("posição fora da curva (comprimento {:.3})", len));
        }
        if reverse {
            s = len - s;
        }
        Ok(c.at_dist(s))
    }

    /// Avalia um elemento de geometria de referência (plano, ponto, curva ou superfície), com memória.
    fn geo(&self, id: Id, depth: u32) -> Result<Geo, String> {
        if let Some(g) = self.memo.borrow().get(&id) {
            return Ok(g.clone());
        }
        let g = self.geo_eval(id, depth)?;
        self.memo.borrow_mut().insert(id, g.clone());
        Ok(g)
    }

    fn geo_eval(&self, id: Id, depth: u32) -> Result<Geo, String> {
        use geo3d::*;
        use RefKind::*;
        if depth > 48 {
            return Err("referências em ciclo".into());
        }
        let r = self.doc.reference(id).ok_or_else(|| format!("referência perdida (#{}) — reedite ou substitua", id.0))?;
        let e = |x: &String, what: &str| ev(x, &self.vars, &format!("{} · {what}", r.name));
        let d = depth + 1;
        let line_of = |c: &Curve3| -> Result<(V3, V3), String> {
            if !c.is_straight() {
                return Err("esperava uma linha reta".into());
            }
            Ok((c.start(), geo3d::unit(geo3d::sub(c.end(), c.start())).ok_or("linha de comprimento zero")?))
        };
        Ok(match &r.kind {
            OffsetPlane { base, offset } => Geo::Plane(self.plane(base, d)?.offset(e(offset, "deslocamento")?)),
            FacePlane { origin, x, normal, owner, key } => Geo::Plane(Frame { origin: self.face_plane_origin(*origin, *normal, *owner, key.as_ref())?, x: *x, normal: *normal }),
            AnglePlane { base, axis, angle } => {
                let b = self.plane(base, d)?;
                let (o, dd) = self.axis_d(axis, d)?;
                let a = e(angle, "ângulo")?.to_radians();
                let rot = |v: [f64; 3]| geo3d::rotate(v, dd, a);
                // o eixo é levado para dentro do plano base (giro em torno da linha no plano base)
                let h = geo3d::dot(geo3d::sub(b.origin, o), b.normal);
                let o = [0, 1, 2].map(|i| o[i] + b.normal[i] * h);
                Geo::Plane(Frame { origin: o, x: rot(b.x), normal: rot(b.normal) })
            }
            PlaneParallelPoint { base, point } => Geo::Plane(Frame { origin: self.point(point, d)?, ..self.plane(base, d)? }),
            AxisSystem { origin, x, z } => {
                let o = self.point(origin, d)?;
                let (dz, dx) = (self.axis_d(z, d)?.1, self.axis_d(x, d)?.1);
                let xo = geo3d::sub(dx, geo3d::mul(dz, geo3d::dot(dx, dz)));
                if norm(xo) < 1e-9 {
                    return Err("os eixos X e Z do sistema são paralelos".into());
                }
                Geo::Plane(frame_from(o, dz, Some(xo))?)
            }
            PlaneThreePoints { a, b, c } => {
                let (pa, pb, pc) = (self.point(a, d)?, self.point(b, d)?, self.point(c, d)?);
                let n = cross(geo3d::sub(pb, pa), geo3d::sub(pc, pa));
                if norm(n) < 1e-9 {
                    return Err("os três pontos estão alinhados ou coincidentes".into());
                }
                Geo::Plane(frame_from(pa, n, Some(geo3d::sub(pb, pa)))?)
            }
            PlaneTwoLines { a, b } => {
                let (pa, da) = line_of(&self.curve(a, d)?)?;
                let (pb, db) = line_of(&self.curve(b, d)?)?;
                let mut n = cross(da, db);
                if norm(n) < 1e-9 {
                    n = cross(da, geo3d::sub(pb, pa));
                    if norm(n) < 1e-9 {
                        return Err("as linhas são a mesma reta".into());
                    }
                }
                let nu = geo3d::unit(n).unwrap();
                if geo3d::dot(geo3d::sub(pb, pa), nu).abs() > 1e-6 * (1. + dist(pa, pb)) {
                    return Err("as linhas não são coplanares".into());
                }
                Geo::Plane(frame_from(pa, nu, Some(da))?)
            }
            PlanePointLine { point, line } => {
                let p = self.point(point, d)?;
                let (o, dd) = line_of(&self.curve(line, d)?)?;
                let n = cross(dd, geo3d::sub(p, o));
                if norm(n) < 1e-9 {
                    return Err("o ponto está sobre a linha".into());
                }
                Geo::Plane(frame_from(o, n, Some(dd))?)
            }
            PlaneCurve { curve } => Geo::Plane(self.curve(curve, d)?.plane()?),
            PlaneNormalCurve { curve, at, reverse, point, project } => {
                let c = self.curve(curve, d)?;
                let (p, t) = match point {
                    Some(q) => {
                        let q = self.point(q, d)?;
                        let (s0, foot) = c.nearest(q);
                        let off = dist(q, foot);
                        if !*project && off > 1e-3 {
                            return Err(format!("o ponto está a {off:.3} mm da curva: use 'projetado' ou escolha um ponto dela"));
                        }
                        (foot, c.at_dist(s0).1)
                    }
                    None => self.on_curve(&c, at, *reverse, &r.name)?,
                };
                Geo::Plane(frame_from(p, t, None)?)
            }
            PlaneTangentSurface { surface, point } => {
                let (p, n) = self.surf_at(surface, self.point(point, d)?, d)?;
                Geo::Plane(frame_from(p, n, None)?)
            }
            PlaneEquation { a, b, c, d: dd } => {
                let n = [e(a, "A")?, e(b, "B")?, e(c, "C")?];
                let l2 = geo3d::dot(n, n);
                if l2 < 1e-18 {
                    return Err("A, B e C não podem ser todos zero".into());
                }
                Geo::Plane(frame_from(mul(n, e(dd, "D")? / l2), n, None)?)
            }
            PlaneMeanPoints { points } => Geo::Plane(fit_plane(&self.points(points, d)?)?.0),

            PointCoords { x, y, z, frame } => {
                let (x, y, z) = (e(x, "X")?, e(y, "Y")?, e(z, "Z")?);
                Geo::Point(match frame {
                    Some(p) => {
                        let f = self.plane(p, d)?;
                        add(f.point(x, y), mul(f.normal, z))
                    }
                    None => [x, y, z],
                })
            }
            PointOnCurve { curve, at, reverse } => Geo::Point(self.on_curve(&self.curve(curve, d)?, at, *reverse, &r.name)?.0),
            PointOnPlane { plane, u, v } => Geo::Point(self.plane(plane, d)?.point(e(u, "u")?, e(v, "v")?)),
            PointOnSurface { surface, u, v } => Geo::Point(self.surf_frame(surface, d)?.point(e(u, "u")?, e(v, "v")?)),
            PointCenter { curve } => Geo::Point(fit_circle(&self.curve(curve, d)?)?.0),
            PointTangent { curve, dir, which } => {
                let (_, dv) = self.axis_d(dir, d)?;
                let pontos = tangent_points(&self.curve(curve, d)?, dv)?;
                self.publicar_solucoes(id, pontos.iter().copied().enumerate().collect(), Some((*which).min(1)));
                Geo::Point(pontos[(*which).min(1)])
            }
            PointBetween { a, b, ratio } => Geo::Point(lerp(self.point(a, d)?, self.point(b, d)?, e(ratio, "proporção")?)),

            LinePoints { a, b, start, end } => {
                let (pa, pb) = (self.point(a, d)?, self.point(b, d)?);
                let dd = geo3d::unit(geo3d::sub(pb, pa)).ok_or("os dois pontos coincidem")?;
                Geo::Curve(Curve3::line(geo3d::sub(pa, mul(dd, e(start, "prolongar início")?)), add(pb, mul(dd, e(end, "prolongar fim")?)))?)
            }
            LineDir { point, dir, start, end } => {
                let p = self.point(point, d)?;
                let (_, dv) = self.axis_d(dir, d)?;
                Geo::Curve(Curve3::line(add(p, mul(dv, e(start, "início")?)), add(p, mul(dv, e(end, "fim")?)))?)
            }
            LineAngle { curve, point, support, angle, start, end } => {
                let c = self.curve(curve, d)?;
                let (s, q) = c.nearest(self.point(point, d)?);
                let (_, t) = c.at_dist(s);
                let n = self.plane(support, d)?.normal;
                let tp = geo3d::unit(geo3d::sub(t, mul(n, geo3d::dot(t, n)))).ok_or("a curva é perpendicular ao plano de apoio nesse ponto")?;
                let dv = geo3d::rotate(tp, n, e(angle, "ângulo")?.to_radians());
                Geo::Curve(Curve3::line(add(q, mul(dv, e(start, "início")?)), add(q, mul(dv, e(end, "fim")?)))?)
            }
            LineNormalSurface { surface, point, start, end } => {
                let (q, n) = self.surf_at(surface, self.point(point, d)?, d)?;
                Geo::Curve(Curve3::line(add(q, mul(n, e(start, "início")?)), add(q, mul(n, e(end, "fim")?)))?)
            }
            Polyline { points, closed } => Geo::Curve(Curve3::poly(self.points(points, d)?, *closed)?),
            Circle { center, plane, radius, start, end } => {
                let c = self.point(center, d)?;
                let f = Frame { origin: c, ..self.plane(plane, d)? };
                Geo::Curve(circle_curve(c, &f, e(radius, "raio")?, e(start, "ângulo inicial")?.to_radians(), e(end, "ângulo final")?.to_radians())?)
            }
            Circle3 { a, b, c, full } => Geo::Curve(circle3_curve(self.point(a, d)?, self.point(b, d)?, self.point(c, d)?, *full)?),
            Spline { points, closed, tangents } => {
                let p = self.points(points, d)?;
                self.notes.borrow_mut().entry(id).or_default().pontos = p.clone();
                Geo::Curve(spline_tan(&p, *closed, tangents)?)
            }
            Helix { origin, axis, radius, pitch, height, taper, left } => {
                let (_, ax) = self.axis_d(axis, d)?;
                Geo::Curve(helix(self.point(origin, d)?, ax, e(radius, "raio")?, e(pitch, "passo")?, e(height, "altura")?, e(taper, "conicidade")?, *left)?)
            }
            Spiral { center, plane, r0, r1, turns } => {
                let c = self.point(center, d)?;
                let f = Frame { origin: c, ..self.plane(plane, d)? };
                Geo::Curve(spiral(c, &f, e(r0, "raio inicial")?, e(r1, "raio final")?, e(turns, "voltas")?)?)
            }
            Conic { center, plane, kind, a, b, extent } => {
                let c = self.point(center, d)?;
                let f = Frame { origin: c, ..self.plane(plane, d)? };
                Geo::Curve(conic(c, &f, *kind, e(a, "a")?, e(b, "b")?, e(extent, "extensão")?)?)
            }
            Connect { a, a_end, b, b_end, cont, tension } => {
                let (ca, cb) = (self.curve(a, d)?, self.curve(b, d)?);
                let pa = if *a_end { ca.end() } else { ca.start() };
                let pb = if *b_end { cb.end() } else { cb.start() };
                let l = dist(pa, pb) * e(tension, "tensão")?;
                if dist(pa, pb) < 1e-9 {
                    return Err("as pontas escolhidas já se tocam".into());
                }
                let (ta, tb) = (ca.end_tangent(*a_end), cb.end_tangent(*b_end));
                let pts = match cont {
                    Continuity::G0 => vec![pa, pb],
                    Continuity::G1 => hermite(pa, mul(ta, l), [0.; 3], pb, mul(tb, -l), [0.; 3], false),
                    Continuity::G2 => hermite(pa, mul(ta, l), mul(ca.end_curvature(*a_end), l * l), pb, mul(tb, -l), mul(cb.end_curvature(*b_end), l * l), true),
                    // a 3ª derivada troca de sinal na chegada (a curva 2 é percorrida para dentro)
                    Continuity::G3 => geo3d::hermite7(
                        pa,
                        mul(ta, l),
                        mul(ca.end_curvature(*a_end), l * l),
                        mul(ca.end_jerk(*a_end), l * l * l),
                        pb,
                        mul(tb, -l),
                        mul(cb.end_curvature(*b_end), l * l),
                        mul(cb.end_jerk(*b_end), -l * l * l),
                    ),
                };
                Geo::Curve(Curve3::poly(pts, false)?)
            }
            Corner { a, b, radius } => Geo::Curve(corner(&self.curve(a, d)?, &self.curve(b, d)?, e(radius, "raio")?)?),
            PointCurvePlane { curve, plane, which } => {
                let c = self.curve(curve, d)?;
                let f = self.plane(plane, d)?;
                let h = |q: V3| geo3d::dot(geo3d::sub(q, f.origin), f.normal);
                let mut hits = vec![];
                for w in c.pts.windows(2) {
                    let (ha, hb) = (h(w[0]), h(w[1]));
                    if ha.abs() < 1e-12 {
                        hits.push(w[0]);
                    } else if ha * hb < 0. {
                        hits.push(lerp(w[0], w[1], ha / (ha - hb)));
                    }
                }
                if h(c.end()).abs() < 1e-12 {
                    hits.push(c.end());
                }
                if hits.is_empty() {
                    return Err("a curva não cruza o plano".into());
                }
                let i = (*which).min(hits.len() - 1);
                self.publicar_solucoes(id, hits.iter().copied().enumerate().collect(), Some(i));
                Geo::Point(hits[i])
            }
            PointCurveCurve { a, b } => {
                let (ca, cb) = (self.curve(a, d)?, self.curve(b, d)?);
                let mut best = (f64::MAX, [0.; 3]);
                for wa in ca.pts.windows(2) {
                    for wb in cb.pts.windows(2) {
                        let (p, q) = seg_closest(wa[0], wa[1], wb[0], wb[1]);
                        let dd = dist(p, q);
                        if dd < best.0 {
                            best = (dd, lerp(p, q, 0.5));
                        }
                    }
                }
                Geo::Point(best.1)
            }
            PointProject { point, surface } => Geo::Point(self.surf_at(surface, self.point(point, d)?, d)?.0),
            PointProjectCurve { point, curve } => Geo::Point(self.curve(curve, d)?.nearest(self.point(point, d)?).1),
            PointOffset { point, dir, dist: dd } => {
                let (_, v) = self.axis_d(dir, d)?;
                Geo::Point(add(self.point(point, d)?, mul(v, e(dd, "distância")?)))
            }
            LinePlanes { a, b, length } => {
                let (fa, fb) = (self.plane(a, d)?, self.plane(b, d)?);
                let u = cross(fa.normal, fb.normal);
                let uu = dot(u, u);
                if uu < 1e-12 {
                    return Err("os planos são paralelos".into());
                }
                let (da, db) = (dot(fa.normal, fa.origin), dot(fb.normal, fb.origin));
                let p0 = mul(add(mul(cross(fb.normal, u), da), mul(cross(u, fa.normal), db)), 1. / uu);
                let dir = geo3d::unit(u).unwrap();
                // centra perto das origens dos planos
                let m = lerp(fa.origin, fb.origin, 0.5);
                let c = add(p0, mul(dir, dot(sub(m, p0), dir)));
                let l = e(length, "comprimento")? / 2.;
                Geo::Curve(Curve3::line(sub(c, mul(dir, l)), add(c, mul(dir, l)))?)
            }
            LineBisector { a, b, length } => {
                let (ca, cb) = (self.curve(a, d)?, self.curve(b, d)?);
                let (pa, da) = line_of(&ca)?;
                let (pb, db) = line_of(&cb)?;
                let (p, q) = seg_closest(sub(pa, mul(da, 1e4)), add(pa, mul(da, 1e4)), sub(pb, mul(db, 1e4)), add(pb, mul(db, 1e4)));
                let v = lerp(p, q, 0.5);
                let side = |c: &Curve3, dd: V3| {
                    let far = if dist(c.start(), v) > dist(c.end(), v) { c.start() } else { c.end() };
                    if dot(sub(far, v), dd) >= 0. { dd } else { mul(dd, -1.) }
                };
                let bis = geo3d::unit(add(side(&ca, da), side(&cb, db))).ok_or("as linhas são opostas")?;
                Geo::Curve(Curve3::line(v, add(v, mul(bis, e(length, "comprimento")?)))?)
            }
            LineAxis { face } => {
                for (b, _, _) in self.bodies.values() {
                    if let Ok(cy) = b.cylinder_at(*face) {
                        let o = add(cy.origin, mul(cy.dir, cy.h0));
                        let t = add(cy.origin, mul(cy.dir, cy.h1.max(cy.h0 + 1.)));
                        return Ok(Geo::Curve(Curve3::line(o, t)?));
                    }
                    if let Ok(fp) = b.face_props(*face) {
                        if fp.kind == 2 {
                            return Ok(Geo::Curve(Curve3::line(fp.origin, add(fp.origin, mul(fp.dir, 2. * fp.radius.max(5.))))?));
                        }
                    }
                }
                return Err("a face escolhida não é cilíndrica nem cônica".into());
            }
            AxisLine { axis, length } => {
                let (o, v) = self.axis_d(axis, d)?;
                let natural = match axis {
                    AxisRef::Face { point, key } => self.face_axis(*point, key.as_ref())?.2,
                    AxisRef::Edge { mid, key } => Curve3::poly(self.live_edge(*mid, key.as_ref())?, false)?.length(),
                    _ => 100.,
                };
                let l = if length.trim().is_empty() { natural } else { e(length, "comprimento")? };
                Geo::Curve(Curve3::line(o, add(o, mul(v, l)))?)
            }
            PlanePointNormal { point, line } => {
                let c = self.curve(line, d)?;
                let p = self.point(point, d)?;
                let (s0, _) = c.nearest(p);
                let (_, t) = c.at_dist(s0);
                Geo::Plane(frame_from(p, t, None)?)
            }
            PlaneMid { a, b } => {
                let (fa, fb) = (self.plane(a, d)?, self.plane(b, d)?);
                let nb = if dot(fa.normal, fb.normal) < 0. { mul(fb.normal, -1.) } else { fb.normal };
                let n = geo3d::unit(add(fa.normal, nb)).ok_or("planos opostos")?;
                let proj = sub(fa.origin, mul(nb, dot(sub(fa.origin, fb.origin), nb)));
                Geo::Plane(frame_from(lerp(fa.origin, proj, 0.5), n, Some(fa.x))?)
            }
            CurveProject { curve, plane } => {
                let c = self.curve(curve, d)?;
                let f = self.plane(plane, d)?;
                let pts = c.pts.iter().map(|q| sub(*q, mul(f.normal, dot(sub(*q, f.origin), f.normal)))).collect();
                Geo::Curve(Curve3::poly(pts, c.closed)?)
            }
            SplineControl { points, closed } => Geo::Curve(bspline(&self.points(points, d)?, *closed)?),
            CurveCopy { curve } => Geo::Curve(self.curve(curve, d)?),
            CurveJoin { curves } => {
                let cs: Vec<Curve3> = curves.iter().map(|c| self.curve(c, d)).collect::<Result<_, _>>()?;
                Geo::Curve(join_curves(cs)?)
            }
            SurfExtract { face, prop, complement, expected, key, .. } => {
                let (sh, nf, ne) = self.extract_faces(self.live_face(*face, key.as_ref())?, *prop, *complement)?;
                self.count_note(id, nf, ne, *expected, "faces");
                Geo::Surface(sh)
            }
            SurfOffset { surface, dist: dd, both, join } => {
                let sh = self.surface_shape(surface, d)?;
                let j = match join {
                    OffsetJoin::Automatico => 0,
                    OffsetJoin::Arredondado => 1,
                    OffsetJoin::Plano => 2,
                };
                let v = e(dd, "distância")?;
                // o kernel desloca pela normal orientada da face, e é essa normal que a seta mostra
                let from = if let SurfRef::Face { point, .. } = surface { *point } else { sh.mass().center };
                let arrow = sh.surface_near(from).map_err(|x| x.0)?;
                self.notes.borrow_mut().insert(id, RefNote { arrow: Some(arrow), ..Default::default() });
                let one = sh.offset_surface_join(v, j).map_err(|x| x.0)?;
                if *both {
                    Geo::Surface(one.compound(&sh.offset_surface_join(-v, j).map_err(|x| x.0)?))
                } else {
                    Geo::Surface(one)
                }
            }
            SurfConnect { a, b, cont, tension, tension2, near } => {
                let (sa, sb) = (self.cut_shape(a, d)?, self.cut_shape(b, d)?);
                let (ra, rb) = (bordas_livres(&sa)?, bordas_livres(&sb)?);
                let (ia, ib) = (escolher_borda(&ra, &sb, near[0])?, escolher_borda(&rb, &sa, near[1])?);
                let (ea, eb) = (ra[ia].clone(), rb[ib].clone());
                {
                    let mut n = self.notes.borrow_mut();
                    let n = n.entry(id).or_default();
                    n.bordas = [ra, rb];
                    n.borda_usada = [ia, ib];
                }
                let t1 = e(tension, "tensão")?;
                let t2 = tension2.as_ref().map_or(Ok(t1), |t| e(t, "tensão 2"))?;
                Geo::Surface(self.blend(id, &CurveRef::Edge(ea), Some(a), &CurveRef::Edge(eb), Some(b), *cont, [t1, t2], d)?)
            }
            SurfSweepX { kind, spine, solution, smooth, sections } => {
                let n = e(sections, "seções")?.round().clamp(4., 200.) as usize;
                let ev = |s: &String, w: &str| e(s, w);
                // cada solução é outra superfície: a bolinha fica sobre ela, no meio da caixa envolvente
                let total = geo3d::solucoes_da_varredura(kind);
                if total > 1 {
                    let meio = |k: usize| {
                        let sh = self.sweep_x(kind, spine.as_ref(), k, *smooth, n, d, &ev).ok()?;
                        let (a, b) = sh.bbox()?;
                        sh.surface_near(lerp(a, b, 0.5)).ok().map(|x| (k, x.0))
                    };
                    self.publicar_solucoes(id, (0..total).filter_map(meio).collect(), Some(*solution));
                }
                Geo::Surface(self.sweep_x(kind, spine.as_ref(), *solution, *smooth, n, d, &ev)?)
            }
            SurfFillet { a, b, flip_a, flip_b, radius, radius2 } => {
                let (sa, sb) = (self.cut_shape(a, d)?, self.cut_shape(b, d)?);
                let cut = sa.section(&sb).map_err(|x| x.0)?;
                let line = cut.into_iter().max_by_key(|c| c.len()).ok_or("as superfícies não se cruzam: o filete precisa da junta entre elas")?;
                // ponto do meio da junta (pelo comprimento)
                let mid = Curve3::poly(line.clone(), false).map(|c| c.at_ratio(0.5).0).unwrap_or(line[line.len() / 2]);
                // cada uma é aparada pela outra prolongada (as larguras não precisam bater)
                let pa = sa.split_side(&sb, !*flip_a, true).map_err(|x| x.0)?;
                let pb = sb.split_side(&sa, !*flip_b, true).map_err(|x| x.0)?;
                let joined = pa.compound(&pb).sewn(1e-3).map_err(|x| x.0)?;
                Geo::Surface(edge_fillet(&joined, &[mid], e(radius, "raio")?, radius2.as_ref().map(|r| e(r, "raio final")).transpose()?)?)
            }
            SurfEdgeFillet { surface, edges, radius, radius2 } => {
                if edges.is_empty() {
                    return Err("clique nas arestas da superfície (entre faces juntadas) e use 'Usar seleção'".into());
                }
                let sh = self.cut_shape(surface, d)?;
                // superfície de referência não tem dono por face: a aresta tem de estar onde foi
                // clicada, e o ponto exato dela é o que vai ao kernel, que aceitaria a vizinha
                let topo = sh.topology().map_err(|x| x.0)?;
                let mut mids = vec![];
                for p in edges {
                    let q = topo.edge_mids.iter().min_by(|a, b| dist(**a, *p).total_cmp(&dist(**b, *p))).filter(|q| dist(**q, *p) < 0.1);
                    mids.push(*q.ok_or_else(|| format!("referência perdida: a aresta em ({:.2}, {:.2}, {:.2}) não existe mais; escolha de novo", p[0], p[1], p[2]))?);
                }
                Geo::Surface(edge_fillet(&sh, &mids, e(radius, "raio")?, radius2.as_ref().map(|r| e(r, "raio final")).transpose()?)?)
            }
            SurfFill { curves, conds, points, tol } => Geo::Surface(self.fill(id, curves, conds, points, e(tol, "tolerância")?, d, &|s: &String, w: &str| e(s, w))?),
            SurfExtrude { curve, dir, length, length2, until, until2 } => {
                let c = self.curve(curve, d)?;
                let (_, v) = self.axis_d(dir, d)?;
                let v = unit(v).ok_or("direção nula")?;
                let (l1, l2) = (e(length, "comprimento")?, e(length2, "comprimento 2")?);
                if until.is_none() && until2.is_none() {
                    // limite 2: a curva recua para o outro lado antes de extrudar
                    let pts: Vec<V3> = c.pts.iter().map(|p| sub(*p, mul(v, l2))).collect();
                    Geo::Surface(Shape::extrude_curve(&pts, v, l1 + l2).map_err(|x| x.0)?)
                } else {
                    // cada lado sai da curva: até o elemento, ou pelo comprimento
                    let side = |dir: V3, len: f64, to: &Option<SurfRef>| -> Result<Option<Shape>, String> {
                        match to {
                            Some(t) => {
                                let long = Shape::extrude_curve(&c.pts, dir, self.model_size()).map_err(|x| x.0)?;
                                Ok(Some(long.clip_surface(&self.cut_shape(t, d)?, c.pts[c.pts.len() / 2], true).map_err(|x| x.0)?))
                            }
                            None if len.abs() < 1e-9 => Ok(None),
                            None => Ok(Some(Shape::extrude_curve(&c.pts, dir, len).map_err(|x| x.0)?)),
                        }
                    };
                    let (a, b) = (side(v, l1, until)?, side(mul(v, -1.), l2, until2)?);
                    Geo::Surface(match (a, b) {
                        (Some(a), Some(b)) => a.compound(&b).sewn(1e-3).map_err(|x| x.0)?,
                        (Some(x), None) | (None, Some(x)) => x,
                        (None, None) => return Err("os dois limites são zero".into()),
                    })
                }
            }
            SurfRevolve { curve, axis, angle, angle2 } => {
                let c = self.curve(curve, d)?;
                let (o, v) = self.axis_d(axis, d)?;
                let (a1, a2) = (e(angle, "ângulo")?, e(angle2, "ângulo 2")?);
                if (a1 + a2).abs() > 360. + 1e-9 {
                    return Err("os dois ângulos somam mais de uma volta".into());
                }
                // ângulo 2: a curva gira para trás antes de revolver
                let k = unit(v).ok_or("eixo nulo")?;
                let pts: Vec<V3> = c.pts.iter().map(|p| add(o, geo3d::rotate(sub(*p, o), k, -a2.to_radians()))).collect();
                Geo::Surface(Shape::revolve_curve(&pts, o, v, a1 + a2).map_err(|x| x.0)?)
            }
            SurfSphere { center, radius } => Geo::Surface(Shape::sphere_surface(self.point(center, d)?, e(radius, "raio")?).map_err(|x| x.0)?),
            SurfCylinder { point, dir, radius, start, end } => {
                let o = self.point(point, d)?;
                let (_, v) = self.axis_d(dir, d)?;
                let v = unit(v).ok_or("direção nula")?;
                Geo::Surface(Shape::cylinder_surface(o, v, e(radius, "raio")?, -e(start, "comprimento 1")?, e(end, "comprimento 2")?).map_err(|x| x.0)?)
            }
            SurfSweep { profile, spine, mode } => {
                let sp = self.curve(spine, d)?;
                let prof = match profile {
                    SweepProfile::Curve(c) => self.curve(c, d)?.pts,
                    SweepProfile::Circle(r) => {
                        // círculo normal à guia no início dela (tubo)
                        let fr = frame_from(sp.start(), sp.end_tangent(false), None)?;
                        let r = e(r, "raio")?;
                        let mut v: Vec<V3> = (0..48).map(|i| {
                            let a = i as f64 / 48. * std::f64::consts::TAU;
                            fr.point(r * a.cos(), r * a.sin())
                        }).collect();
                        v.push(v[0]);
                        v
                    }
                };
                let m = match mode {
                    SweepMode::Corrigido => 0,
                    SweepMode::Fixo => 1,
                    SweepMode::Frenet => 2,
                };
                Geo::Surface(Shape::sweep_surface(&prof, &sp.pts, m).map_err(|x| x.0)?)
            }
            SurfLoft { sections, guide, ruled, closed, opts } => {
                let mut curvas = vec![];
                for c in sections {
                    curvas.push(self.curve(c, d)?);
                }
                let mut tangentes = [0.; 2];
                for (t, x) in tangentes.iter_mut().zip(&opts.tangents) {
                    *t = if x.trim().is_empty() { 0. } else { e(x, "tangência")? };
                }
                let mut guias = vec![];
                for g in &opts.guides {
                    guias.push(self.curve(g, d)?);
                }
                {
                    let mut n = self.notes.borrow_mut();
                    let n = n.entry(id).or_default();
                    n.sentidos = loft::sentidos(&curvas, &opts.reversed);
                    n.secoes = curvas.clone();
                }
                let controle = !guias.is_empty() || !opts.connectors.is_empty() || opts.reversed.iter().any(|x| *x) || tangentes.iter().any(|t| *t != 0.);
                let sh = if controle {
                    loft::superficie(&loft::Entrada { secoes: curvas, guias, conectores: opts.connectors.clone(), invertidas: opts.reversed.clone(), regrada: *ruled, fechar: *closed, tangentes })?
                } else {
                    let cs: Vec<Vec<V3>> = curvas.into_iter().map(|c| c.pts).collect();
                    let g = match guide {
                        Some(g) => Some(self.curve(g, d)?.pts),
                        None => None,
                    };
                    Shape::loft_surface(&cs, *ruled, *closed, g.as_deref()).map_err(|x| x.0)?
                };
                if opts.iso > 0 {
                    let iso = sh.iso_curves(opts.iso).map_err(|x| x.0)?;
                    self.notes.borrow_mut().entry(id).or_default().iso = iso;
                }
                Geo::Surface(sh)
            }
            SurfRuled { curves, support, mode, dir, length, flip } => {
                let l = e(length, "comprimento")? * if *flip { -1. } else { 1. };
                let apoio = match support {
                    Some(s) => Some(self.cut_shape(s, d)?),
                    None => None,
                };
                let vetor = match mode {
                    RuledMode::Vetor => Some(unit(self.axis_d(dir, d)?.1).ok_or("direção nula")?),
                    _ => None,
                };
                let mut acc: Option<Shape> = None;
                for c in curves {
                    let faixa = regrada(&self.curve(c, d)?, apoio.as_ref(), *mode, vetor, l)?;
                    acc = Some(match acc {
                        Some(a) => a.compound(&faixa),
                        None => faixa,
                    });
                }
                Geo::Surface(acc.ok_or("escolha pelo menos uma curva")?.sewn(1e-3).map_err(|x| x.0)?)
            }
            SurfCheck { mode, a, b, curve, samples, scale } => {
                let n = e(samples, "amostras")?.round().clamp(2., 2000.);
                let esc = e(scale, "escala")?;
                let (res, pior, texto) = match mode {
                    CheckMode::Conexao => {
                        let (sa, sb) = (self.cut_shape(a, d)?, self.cut_shape(b, d)?);
                        let am = sa.connect_check(&sb, n as u32, esc).map_err(|x| x.0)?;
                        let c = analise::Conexao::de(am).ok_or_else(|| format!("nenhuma borda da primeira superfície chega a {esc} mm da segunda"))?;
                        let t = format!("folga {:.4} mm · ângulo {:.3}° · Δcurvatura {:.5} 1/mm", c.folga, c.angulo, c.dif_curvatura);
                        // o pior caso é o que mais passa da tolerância
                        let p = if c.folga > analise::TOL_FOLGA { c.onde[0] } else if c.angulo > analise::TOL_ANGULO { c.onde[1] } else { c.onde[2] };
                        (analise::Analise::Conexao(c), p, t)
                    }
                    CheckMode::Desvio => {
                        let (sa, sb) = (self.cut_shape(a, d)?, self.cut_shape(b, d)?);
                        let m = tesselar(&sa, &self.qualidade).map_err(|x| x.0)?;
                        let x = analise::Desvio::medir(m, &sb)?;
                        let (t, p) = (format!("desvio de {:.4} a {:.4} mm · RMS {:.4} mm", x.min, x.max, x.rms), x.pior);
                        (analise::Analise::Desvio(x), p, t)
                    }
                    CheckMode::Pente => {
                        let dentes = analise::pente(&self.curve(curve, d)?, n as usize, esc);
                        let top = analise::pico(&dentes).ok_or("curva sem pontos")?;
                        let (t, p) = (format!("maior curvatura {:.5} 1/mm", top.k), top.base);
                        (analise::Analise::Pente(dentes), p, t)
                    }
                };
                let mut notas = self.notes.borrow_mut();
                let nota = notas.entry(id).or_default();
                nota.analise = Some(res);
                nota.text = texto;
                Geo::Point(pior)
            }
            SurfBlend { a, sa, b, sb, cont, tension, tension2 } => {
                let t1 = e(tension, "tensão")?;
                let t2 = tension2.as_ref().map_or(Ok(t1), |t| e(t, "tensão 2"))?;
                Geo::Surface(self.blend(id, a, sa.as_ref(), b, sb.as_ref(), *cont, [t1, t2], d)?)
            }
            SurfJoin { items, tol, curves } => {
                let t = e(tol, "tolerância")?;
                let mut acc = Shape::empty();
                for it in items {
                    let x = self.cut_shape(it, d)?;
                    acc = if acc.is_null() { x } else { acc.compound(&x) };
                }
                if !acc.is_null() {
                    acc = acc.sewn(t).map_err(|x| x.0)?;
                }
                // as curvas entram encadeadas: cada cadeia vira um fio ao lado das faces
                let mut rest: Vec<Curve3> = curves.iter().map(|c| self.curve(c, d)).collect::<Result<_, _>>()?;
                while !rest.is_empty() {
                    let group = chain_indices(&rest, 0, t.max(1e-3));
                    let cs: Vec<Curve3> = group.iter().rev().map(|i| rest.remove(*i)).rev().collect();
                    let w = Shape::curve_shape(&join_curves(cs)?.pts).map_err(|x| x.0)?;
                    acc = if acc.is_null() { w } else { acc.compound(&w) };
                }
                if acc.is_null() {
                    return Err("escolha as superfícies a juntar".into());
                }
                let rep = acc.sew_report().map_err(|x| x.0)?;
                self.notes.borrow_mut().insert(id, RefNote { join: Some(rep), ..Default::default() });
                Geo::Surface(acc)
            }
            SurfHeal { surface, tol } => Geo::Surface(self.cut_shape(surface, d)?.healed(e(tol, "tolerância")?).map_err(|x| x.0)?),
            SurfSplit { surface, tool, flip, more, curve, more_flip } => {
                let mut sh = self.cut_shape(surface, d)?;
                let mut cut_any = false;
                if let Some(c) = curve {
                    // a curva sobre a superfície vira uma faixa na normal (corta dos dois lados)
                    let c = self.curve(c, d)?;
                    let mid = c.pts[c.pts.len() / 2];
                    let (_, n) = sh.surface_near(mid).map_err(|x| x.0)?;
                    let l = self.model_size();
                    let pts: Vec<V3> = c.pts.iter().map(|p| sub(*p, mul(n, l))).collect();
                    let cutter = Shape::extrude_curve(&pts, n, 2. * l).map_err(|x| x.0)?;
                    sh = sh.split_side(&cutter, !*flip, false).map_err(|x| x.0)?;
                    cut_any = true;
                }
                let mut missed = vec![];
                for (i, t) in std::iter::once(tool).chain(more.iter()).enumerate() {
                    let side = if i == 0 { *flip } else { more_flip.get(i - 1).copied().unwrap_or(*flip) };
                    match self.cut_shape(t, d).and_then(|x| sh.split_side(&x, !side, false).map_err(|x| x.0)) {
                        Ok(x) => {
                            sh = x;
                            cut_any = true;
                        }
                        // com outro corte já feito o resultado existe, mas quem não cortou é dito
                        Err(_) if cut_any => missed.push((i + 1).to_string()),
                        Err(e) => return Err(e),
                    }
                }
                if !missed.is_empty() {
                    self.notes.borrow_mut().insert(id, RefNote { warn: true, text: format!("o elemento de corte {} não cruza a superfície e não cortou nada", missed.join(", ")), ..Default::default() });
                }
                Geo::Surface(sh)
            }
            SurfTrim { a, b, flip_a, flip_b } => {
                let (sa, sb) = (self.cut_shape(a, d)?, self.cut_shape(b, d)?);
                let pa = sa.split_side(&sb, !*flip_a, false).map_err(|x| x.0)?;
                let pb = sb.split_side(&sa, !*flip_b, false).map_err(|x| x.0)?;
                Geo::Surface(pa.compound(&pb).sewn(1e-3).map_err(|x| x.0)?)
            }
            SurfRegions { surface, tools, curve, mutual, remove, extend, marks } => {
                let corte = regioes::Corte { surface, tools, curve: curve.as_ref(), mutual: *mutual, remove: *remove, extend: *extend, marks };
                Geo::Surface(self.regioes(id, corte, d)?)
            }
            SurfMulti { faces, keys } => {
                let mut acc = Shape::empty();
                for (i, p) in faces.iter().enumerate() {
                    let x = self.face_of_bodies(self.live_face(*p, keys.get(i).and_then(|k| k.as_ref()))?)?;
                    acc = if acc.is_null() { x } else { acc.compound(&x) };
                }
                if acc.is_null() {
                    return Err("escolha as faces a extrair".into());
                }
                Geo::Surface(acc)
            }
            SurfPart { surface, index } => Geo::Surface(self.cut_shape(surface, d)?.nth_face(*index as u32).map_err(|x| x.0)?),
            SurfUntrim { surface } => Geo::Surface(self.cut_shape(surface, d)?.untrimmed().map_err(|x| x.0)?),
            SurfExtrapolate { surface, near, length, curvature, more, edges, until, normal_edge, tangent_prop } => {
                let mut sh = self.cut_shape(surface, d)?;
                let len = e(length, "comprimento")?;
                let mut at: Vec<V3> = vec![];
                if edges.is_empty() {
                    for p in std::iter::once(near).chain(more.iter()) {
                        at.push(self.point(p, d)?);
                    }
                } else {
                    let free: Vec<Curve3> = sh.free_edges().map_err(|x| x.0)?.into_iter().map(|p| Curve3::poly(p, false)).collect::<Result<_, _>>()?;
                    for c in edges {
                        let mid = self.curve(c, d)?.at_ratio(0.5).0;
                        let gap = |c: &Curve3| dist(c.nearest(mid).1, mid);
                        let i = (0..free.len()).min_by(|a, b| gap(&free[*a]).total_cmp(&gap(&free[*b]))).ok_or("a superfície não tem bordas livres")?;
                        if gap(&free[i]) > 0.15 {
                            return Err("a curva escolhida não é borda da superfície".into());
                        }
                        let set = if *tangent_prop { tangent_chain(&free, i, 1e-3) } else { vec![i] };
                        for j in set {
                            let m = free[j].at_ratio(0.5).0;
                            if !at.iter().any(|q| dist(*q, m) < 1e-6) {
                                at.push(m);
                            }
                        }
                    }
                }
                // a parte original fica como âncora do lado que continua depois do corte
                let keep = sh.mass().center;
                let reach = if until.is_some() { self.model_size() } else { len };
                for p in at {
                    sh = if *normal_edge { self.extend_normal(&sh, p, reach)? } else { sh.extrapolated(p, reach, *curvature).map_err(|x| x.0)? };
                }
                if let Some(t) = until {
                    sh = sh.clip_surface(&self.cut_shape(t, d)?, keep, true).map_err(|x| x.0)?;
                }
                Geo::Surface(sh)
            }
            GeoTransform { source, xf, .. } => {
                let x = self.xform(xf, d, &|s: &String, w: &str| e(s, w))?;
                match self.geo(*source, d)? {
                    Geo::Point(p) => Geo::Point(xform_point(&x, p)),
                    Geo::Plane(f) => Geo::Plane(Frame { origin: xform_point(&x, f.origin), x: xform_dir(&x, f.x), normal: xform_dir(&x, f.normal) }),
                    Geo::Curve(c) => Geo::Curve(Curve3::poly(c.pts.iter().map(|p| xform_point(&x, *p)).collect(), c.closed)?),
                    Geo::Surface(sh) => Geo::Surface(sh.transformed(x).map_err(|x| x.0)?),
                }
            }
            CurveIntersect { a, b, which, near } => {
                let cs = self.cut_shape(a, d)?.section(&self.cut_shape(b, d)?).map_err(|x| x.0)?;
                if cs.is_empty() {
                    return Err("os elementos não se cruzam".into());
                }
                let c = self.pick_curve(id, cs, *which, *near)?;
                let closed = c.len() > 2 && dist(c[0], c[c.len() - 1]) < 1e-4;
                Geo::Curve(Curve3::poly(c, closed)?)
            }
            CurveProjectSurf { curve, surface, dir } => {
                let c = self.curve(curve, d)?;
                let along = match dir {
                    Some(a) => Some(unit(self.axis_d(a, d)?.1).ok_or("direção nula")?),
                    None => None,
                };
                let pts: Vec<V3> = match (surface, along) {
                    (SurfRef::Plane(p), None) => {
                        let f = self.plane(p, d)?;
                        c.pts.iter().map(|q| sub(*q, mul(f.normal, dot(sub(*q, f.origin), f.normal)))).collect()
                    }
                    (SurfRef::Plane(p), Some(v)) => {
                        let f = self.plane(p, d)?;
                        let den = dot(v, f.normal);
                        if den.abs() < 1e-9 {
                            return Err("a direção é paralela ao plano".into());
                        }
                        c.pts.iter().map(|q| add(*q, mul(v, dot(sub(f.origin, *q), f.normal) / den))).collect()
                    }
                    (_, None) => {
                        let sh = self.surface_shape(surface, d)?;
                        c.pts.iter().map(|q| sh.surface_near(*q).map(|(p, _)| p)).collect::<Result<_, _>>().map_err(|x| x.0)?
                    }
                    (_, Some(v)) => {
                        // cada ponto segue a direção até encontrar a superfície (os que não encontram ficam de fora)
                        let sh = self.surface_shape(surface, d)?;
                        let mut out = vec![];
                        for q in &c.pts {
                            if let Some((p, _)) = sh.ray_hit(*q, v).map_err(|x| x.0)? {
                                out.push(p);
                            }
                        }
                        if out.len() < 2 {
                            return Err("a curva não atinge a superfície nessa direção".into());
                        }
                        if out.len() < c.pts.len() {
                            self.notes.borrow_mut().insert(id, RefNote { warn: true, text: format!("projeção parcial: {} de {} pontos atingem a superfície", out.len(), c.pts.len()), ..Default::default() });
                            // a curva parcial não fecha, mesmo que a original feche
                            return Ok(Geo::Curve(Curve3::poly(out, false)?));
                        }
                        out
                    }
                };
                Geo::Curve(Curve3::poly(pts, c.closed)?)
            }
            CurveParallel { curve, support, d1, d2, flip, more, both, d_other } => {
                let c = if more.is_empty() {
                    self.curve(curve, d)?
                } else {
                    let mut cs = vec![self.curve(curve, d)?];
                    for m in more {
                        cs.push(self.curve(m, d)?);
                    }
                    join_curves(cs)?
                };
                let (a, b) = (e(d1, "distância inicial")?, e(d2, "distância final")?);
                let sh = match support {
                    SurfRef::Plane(_) => None,
                    _ => Some(self.surface_shape(support, d)?),
                };
                let total = c.length().max(1e-9);
                let n = c.pts.len();
                // um lado: `sgn` = +1 lado escolhido, −1 o outro; `k` escala as distâncias
                let side_pts = |sgn: f64, da: f64, db: f64| -> Result<Vec<V3>, String> {
                    let mut s_acc = 0.;
                    let mut pts = vec![];
                    for i in 0..n {
                        if i > 0 {
                            s_acc += dist(c.pts[i - 1], c.pts[i]);
                        }
                        let q = c.pts[i];
                        let t = unit(sub(c.pts[(i + 1).min(n - 1)], c.pts[i.saturating_sub(1)])).ok_or("curva degenerada")?;
                        let nrm = match (&sh, support) {
                            (Some(sh), _) => sh.surface_near(q).map_err(|x| x.0)?.1,
                            (None, SurfRef::Plane(p)) => self.plane(p, d)?.normal,
                            _ => unreachable!(),
                        };
                        let mut side = unit(cross(nrm, t)).ok_or("a curva é normal ao apoio")?;
                        if *flip {
                            side = mul(side, -1.);
                        }
                        let dd = da + (db - da) * s_acc / total;
                        let mut p = add(q, mul(side, sgn * dd));
                        if let Some(sh) = &sh {
                            p = sh.surface_near(p).map_err(|x| x.0)?.0;
                        }
                        pts.push(p);
                    }
                    Ok(pts)
                };
                let one = side_pts(1., a, b)?;
                if *both {
                    if c.closed {
                        return Err("curva fechada: os dois lados dão duas curvas — use uma de cada lado".into());
                    }
                    let (oa, ob) = if d_other.trim().is_empty() { (a, b) } else {
                        let v = e(d_other, "distância do outro lado")?;
                        (v, v)
                    };
                    let mut other = side_pts(-1., oa, ob)?;
                    other.reverse();
                    let mut pts = one;
                    pts.extend(other);
                    Geo::Curve(Curve3::poly(pts, true)?)
                } else {
                    Geo::Curve(Curve3::poly(one, c.closed)?)
                }
            }
            CurveBoundary { surface, which, prop, near: Some(p), expected } => {
                let _ = which;
                let (c, n) = self.boundary_from(&self.cut_shape(surface, d)?, *p, *prop)?;
                self.count_note(id, n, n, *expected, "arestas");
                Geo::Curve(c)
            }
            CurveBoundary { surface, which, .. } => {
                let cs = self.cut_shape(surface, d)?.boundary().map_err(|x| x.0)?;
                if cs.is_empty() {
                    return Err("sem bordas".into());
                }
                let c = self.pick_curve(id, cs, *which, None)?;
                let closed = c.len() > 2 && dist(c[0], c[c.len() - 1]) < 1e-4;
                Geo::Curve(Curve3::poly(c, closed)?)
            }
            Linked { ext, .. } => {
                let x = self.doc.external.iter().find(|x| x.id == *ext).ok_or("link perdido")?;
                if x.state == LinkState::Desativado {
                    return Err("link desativado provisoriamente — reative pelo botão direito".into());
                }
                linked_geo(x.snapshot.as_ref().ok_or("link sem cópia — atualize os links")?)?
            }
            Isolated { geo } => linked_geo(geo)?,
            Spine { planes } => {
                let fs: Vec<Frame> = planes.iter().map(|p| self.plane(p, d)).collect::<Result<_, _>>()?;
                if fs.len() < 2 {
                    return Err("a spine precisa de pelo menos dois planos".into());
                }
                let mut pts: Vec<V3> = vec![];
                for w in fs.windows(2) {
                    let (p0, p1) = (w[0].origin, w[1].origin);
                    let chord = geo3d::sub(p1, p0);
                    let l = norm(chord);
                    if l < 1e-9 {
                        return Err("dois planos seguidos com a mesma origem".into());
                    }
                    // normais orientadas no sentido do percurso
                    let orient = |n: V3| if geo3d::dot(n, chord) < 0. { mul(n, -1.) } else { n };
                    let seg = hermite(p0, mul(orient(w[0].normal), l), [0.; 3], p1, mul(orient(w[1].normal), l), [0.; 3], false);
                    if !pts.is_empty() {
                        pts.pop();
                    }
                    pts.extend(seg);
                }
                Geo::Curve(Curve3::poly(pts, false)?)
            }
        })
    }

    /// Perfil de um comando: sketch, ou curva fechada e plana do wireframe (o plano sai da curva).
    fn sketch(&self, id: Id) -> Result<(Frame, Arc<EvalSketch>), String> {
        if let Some(s) = self.sketches.get(&id) {
            return Ok(s.clone());
        }
        if self.doc.feature(id).is_none() && (self.doc.reference(id).is_some() || self.doc.external.iter().any(|e| e.id == id)) {
            return self.wire_profile(id);
        }
        Err(match self.doc.feature(id) {
            Some(f) => format!("o sketch '{}' está inativo ou com erro", f.name),
            None => format!("referência perdida: sketch #{} não existe — reedite ou substitua a referência", id.0),
        })
    }

    /// Curva do wireframe como perfil: precisa ser fechada e plana; círculo vira círculo exato e
    /// curvas suaves (spline, cônica…) viram spline, cantos ficam retos.
    fn wire_profile(&self, id: Id) -> Result<(Frame, Arc<EvalSketch>), String> {
        use fenkra_kernel::Seg2;
        let name = self.doc.name_of(id);
        let c = match self.geo(id, 1)? {
            Geo::Curve(c) => c,
            _ => return Err(format!("'{name}' não é um sketch nem uma curva: escolha um sketch ou uma curva fechada do wireframe")),
        };
        let fr = c.plane().map_err(|e| format!("'{name}' não serve de perfil: {e} — escolha um sketch ou uma curva plana"))?;
        let uv = |p: V3| {
            let d = geo3d::sub(p, fr.origin);
            [geo3d::dot(d, fr.x), geo3d::dot(d, fr.y())]
        };
        let mut pts: Vec<V3> = c.pts.clone();
        let closed = c.closed || (pts.len() > 2 && geo3d::dist(pts[0], *pts.last().unwrap()) < 1e-6);
        if !closed {
            return Err(format!("'{name}' é aberta: o perfil de um sólido precisa de contorno fechado (feche a curva ou junte as curvas com 'Juntar')"));
        }
        if pts.len() > 2 && geo3d::dist(pts[0], *pts.last().unwrap()) < 1e-6 {
            pts.pop();
        }
        pts.dedup_by(|a, b| geo3d::dist(*a, *b) < 1e-9);
        let lp: Vec<Seg2> = if let Some((ctr, _, r)) = c.circle.filter(|_| c.closed) {
            vec![Seg2::Circle(uv(ctr), r)]
        } else {
            let p: Vec<[f64; 2]> = pts.iter().map(|q| uv(*q)).collect();
            let n = p.len();
            if n < 3 {
                return Err(format!("'{name}' tem pontos de menos para um perfil"));
            }
            // trechos suaves viram spline (sem facetar); cantos vivos ficam retos
            let mut segs = vec![];
            for run in fenkra_core::sketch::smooth_runs(&p, true) {
                if run.len() >= 4 {
                    segs.push(Seg2::Spline(run.iter().map(|i| p[*i]).collect()));
                } else {
                    for w in run.windows(2) {
                        segs.push(Seg2::Line(p[w[0]], p[w[1]]));
                    }
                }
            }
            segs
        };
        let es = EvalSketch { loops: vec![lp], ..Default::default() };
        Ok((fr, Arc::new(es)))
    }

    /// Polilinha 3D da geometria externa de um sketch, no estado atual do modelo.
    /// Aresta de corpo mais perto de `mid`: o corpo, o ponto médio dela, a fórmula e a polilinha.
    fn aresta_perto(&self, mid: V3) -> Result<(Shape, V3, Option<fenkra_kernel::EdgeFormula>, Vec<V3>), String> {
        let mut best: Option<(f64, Shape, V3, Option<fenkra_kernel::EdgeFormula>, Vec<V3>)> = None;
        for (b, _, _) in self.bodies.values() {
            let Ok(m) = tesselar(b, &self.qualidade) else { continue };
            for e in 0..m.edge_mids.len() / 3 {
                let q = [m.edge_mids[3 * e] as f64, m.edge_mids[3 * e + 1] as f64, m.edge_mids[3 * e + 2] as f64];
                let d = geo3d::dist(q, mid);
                if best.as_ref().is_none_or(|x| d < x.0) {
                    let s0 = m.edge_ranges[e] as usize;
                    let s1 = m.edge_ranges.get(e + 1).map(|x| *x as usize).unwrap_or(m.edges.len() / 6);
                    let mut pl: Vec<V3> = vec![];
                    for sg in m.edges.chunks_exact(6).take(s1).skip(s0) {
                        if pl.is_empty() {
                            pl.push([sg[0] as f64, sg[1] as f64, sg[2] as f64]);
                        }
                        pl.push([sg[3] as f64, sg[4] as f64, sg[5] as f64]);
                    }
                    best = Some((d, b.clone(), q, m.edge_formula(e), pl));
                }
            }
        }
        let (d, b, q, f, pl) = best.ok_or("nenhum corpo com arestas")?;
        // a aresta pode ter mudado um pouco; longe demais = perdida
        if d > 5. {
            return Err(format!("aresta não encontrada (a {d:.1} mm do lugar)"));
        }
        Ok((b, q, f, pl))
    }

    /// Forma exata da origem no plano do sketch; `None` se ela não tiver fórmula.
    fn link_exata(&self, l: &fenkra_core::sketch::SkLink, frame: &Frame) -> Result<Option<fenkra_core::projecao::Exata>, String> {
        use fenkra_core::projecao::{projeta, reconhece, Curva3};
        use fenkra_core::sketch::LinkSrc;
        let curva = match &l.source {
            LinkSrc::Edge { mid } => match self.aresta_perto(*mid)?.2 {
                Some(fenkra_kernel::EdgeFormula::Line { a, b }) => Curva3::Reta { a, b },
                Some(fenkra_kernel::EdgeFormula::Conic { c, u, v, t0, t1 }) => Curva3::Conica { c, u, v, t0, t1 },
                None => Curva3::Outra,
            },
            LinkSrc::SketchCurve { .. } | LinkSrc::Ref(_) if !l.intersect => reconhece(&self.link_world(l, frame)?),
            _ => Curva3::Outra,
        };
        Ok(projeta(&curva, &profile::plano_do(frame)))
    }

    fn link_world(&self, l: &fenkra_core::sketch::SkLink, frame: &Frame) -> Result<Vec<V3>, String> {
        use fenkra_core::sketch::LinkSrc;
        let k = |e: KernelError| e.0;
        // corte da forma pelo plano do sketch: o contorno mais longo
        let cut = |sh: &Shape| -> Result<Vec<V3>, String> {
            let face = Shape::plane_face(frame.origin, frame.normal, self.model_size()).map_err(k)?;
            let loops = sh.section(&face).map_err(k)?;
            let len = |l: &Vec<V3>| l.windows(2).map(|w| geo3d::dist(w[0], w[1])).sum::<f64>();
            Ok(loops.into_iter().max_by(|a, b| len(a).total_cmp(&len(b))).unwrap_or_default())
        };
        // pontos onde a polilinha atravessa o plano
        // com fórmula, o corte é exato; sem ela, fica a travessia da polilinha
        let crossings = |pts: &[V3]| -> Vec<V3> {
            let curva = fenkra_core::projecao::reconhece(pts);
            fenkra_core::projecao::corta(&curva, &profile::plano_do(frame)).unwrap_or_else(|| fenkra_core::sketch::plane_crossings(pts, frame.origin, frame.normal))
        };
        match &l.source {
            LinkSrc::Ref(id) => match self.geo(*id, 0)? {
                Geo::Point(p) => Ok(vec![p]),
                Geo::Curve(c) if l.intersect => Ok(crossings(&c.pts)),
                Geo::Curve(c) => Ok(c.pts),
                Geo::Surface(sh) => cut(&sh),
                Geo::Plane(_) => Err("plano não se projeta num sketch".into()),
            },
            LinkSrc::SketchCurve { sketch, curve } => {
                let (fr, es) = &self.sketch(*sketch)?;
                let pts = es.curves.get(curve).ok_or("a curva do outro sketch não existe mais")?;
                let p3: Vec<V3> = pts.iter().map(|q| fr.point(q[0], q[1])).collect();
                Ok(if l.intersect { crossings(&p3) } else { p3 })
            }
            LinkSrc::Edge { mid } => {
                let (b, q, _, pl) = self.aresta_perto(*mid)?;
                // a interseção pergunta ao kernel: é exata para qualquer tipo de curva
                if l.intersect {
                    return b.edge_plane(q, frame.origin, frame.normal).map_err(k);
                }
                Ok(pl)
            }
            LinkSrc::Vertex { pos: mid } => {
                let mut best: Option<(f64, V3)> = None;
                for (b, _, _) in self.bodies.values() {
                    let Ok(m) = tesselar(b, &self.qualidade) else { continue };
                    for v in m.vertices.chunks_exact(3) {
                        let p = [v[0] as f64, v[1] as f64, v[2] as f64];
                        let d = geo3d::dist(p, *mid);
                        if best.as_ref().is_none_or(|x| d < x.0) {
                            best = Some((d, p));
                        }
                    }
                }
                let (d, p) = best.ok_or("nenhum corpo com arestas")?;
                if d > 5. {
                    return Err(format!("vértice não encontrado (a {d:.1} mm do lugar)"));
                }
                Ok(vec![p])
            }
            LinkSrc::Face { point } => cut(&self.face_of_bodies(*point)?),
        }
    }

    fn body(&self, target: Id) -> Option<&Shape> {
        self.body_owned(target).map(|(s, _, _)| s)
    }

    /// Distância "passante": maior que o corpo inteiro.
    fn through(&self, target: Id) -> Result<f64, String> {
        let b = self.body(target).ok_or("'Passante' precisa de um corpo existente no coletor de destino")?;
        let (lo, hi) = b.bbox().ok_or("corpo sem dimensões")?;
        let mut d2 = 0.;
        for i in 0..3 {
            d2 += (hi[i] - lo[i]).powi(2);
        }
        Ok(d2.sqrt() * 2. + 10.)
    }

    fn extent(&self, e: &Extent, target: Id) -> Result<(f64, f64), String> {
        Ok(match e {
            Extent::Blind(d) => (ev(d, &self.vars, "distância")?, 0.),
            Extent::Symmetric(d) => {
                let d = ev(d, &self.vars, "distância")?;
                (d / 2., d / 2.)
            }
            Extent::TwoSides(a, b) => (ev(a, &self.vars, "distância 1")?, ev(b, &self.vars, "distância 2")?),
            Extent::ThroughAll => (self.through(target)?, 0.),
            Extent::UpTo(_) => return Err("'até plano' só vale para extrusão e recorte".into()),
            Extent::UpToNext => return Err("'até o próximo' não vale para este comando".into()),
            Extent::UpToSurface(_) => return Err("'até a superfície' não vale para este comando".into()),
            Extent::UpToLast => return Err("'até o último' não vale para este comando".into()),
        })
    }

    /// Tamanho de referência do modelo (para meios-espaços e prolongamentos).
    fn size_hint(&self) -> f64 {
        self.model_size().max(1000.)
    }

    /// Acrescenta sólidos sem fundir (membros e placas continuam peças separadas no corpo).
    fn add_separate(&mut self, f: &Feature, s: Shape) {
        let n = s.face_count() as usize;
        let next = match self.bodies.remove(&f.target) {
            Some((b, mut own, mut roles)) if !b.is_null() => {
                own.extend(std::iter::repeat_n(f.id, n));
                roles.extend(std::iter::repeat_n(-1, n));
                (b.compound(&s), own, roles)
            }
            _ => fresh_body(s, f.id),
        };
        self.bodies.insert(f.target, next);
    }

    /// Geometria de cada membro (eixos da seção, tamanho, prolongamentos — pontas ligadas ganham
    /// sobra para o tratamento de canto cortar).
    fn members(&self, m: &StructMember) -> Result<Vec<MemberGeom>, String> {
        let dims: Vec<f64> = m.dims.iter().map(|e| ev(e, &self.vars, "cota da seção")).collect::<Result<_, _>>()?;
        let (loops, w, h, cen) = structural::section(m.kind, &dims)?;
        // B253: espelhar a seção troca o lado dos perfis sem simetria (a caixa não muda)
        let (loops, cen) = if m.mirror { (structural::mirrored(&loops), [-cen[0], cen[1]]) } else { (loops, cen) };
        let wall = structural::wall(m.kind, &dims);
        let ang = ev(&m.angle, &self.vars, "rotação")?.to_radians();
        let off = [ev(&m.off_x, &self.vars, "deslocamento x")?, ev(&m.off_y, &self.vars, "deslocamento y")?];
        let a = structural::anchor_point(m.anchor, w, h, cen);
        let shift = [a[0] - off[0], a[1] - off[1]];
        let (e1, e2) = (ev(&m.ext1, &self.vars, "prolongamento 1")?, ev(&m.ext2, &self.vars, "prolongamento 2")?);
        let mut out = vec![];
        // polilinha com cantos vivos: cada trecho reto é um membro (os cantos são tratados entre eles)
        let mut pieces: Vec<Curve3> = vec![];
        for p in &m.paths {
            let c = self.curve(p, 0)?;
            let n = c.pts.len();
            let mut cut = vec![0];
            for i in 1..n.saturating_sub(1) {
                if let (Some(u), Some(v)) = (geo3d::unit(geo3d::sub(c.pts[i], c.pts[i - 1])), geo3d::unit(geo3d::sub(c.pts[i + 1], c.pts[i]))) {
                    if geo3d::dot(u, v) < 0.98 {
                        cut.push(i);
                    }
                }
            }
            cut.push(n - 1);
            if cut.len() <= 2 || c.circle.is_some() {
                pieces.push(c);
                continue;
            }
            for w in cut.windows(2) {
                let seg = c.pts[w[0]..=w[1]].to_vec();
                pieces.push(if seg.len() == 2 { Curve3::line(seg[0], seg[1])? } else { Curve3::poly(seg, false)? });
            }
        }
        for c in pieces {
            let (start, end) = (c.start(), c.end());
            let z = if c.is_straight() { geo3d::unit(geo3d::sub(end, start)).ok_or("linha guia sem comprimento")? } else { c.end_tangent(false) };
            let up = if z[2].abs() > 0.99 { [1., 0., 0.] } else { [0., 0., 1.] };
            let v = geo3d::unit(geo3d::sub(up, geo3d::mul(z, geo3d::dot(up, z)))).unwrap();
            let x0 = fenkra_kernel::cross(v, z);
            let x = geo3d::rotate(x0, z, ang);
            let y = fenkra_kernel::cross(z, x);
            out.push(MemberGeom { start, end, z, x, x0, y, w, h, wall, shift, ext: (e1, e2), loops: loops.clone(), pts: c.pts.clone(), straight: c.is_straight() });
        }
        Ok(out)
    }

    fn member_shape(&self, g: &MemberGeom) -> Result<Shape, String> {
        let k = |e: KernelError| e.0;
        let loops = structural::shifted(&g.loops, g.shift);
        if g.straight {
            let o = geo3d::sub(g.start, geo3d::mul(g.z, g.ext.0));
            let fr = Frame { origin: o, x: g.x, normal: g.z };
            let face = Shape::face(&fr, &loops).map_err(k)?;
            face.extrude(g.z, geo3d::dist(g.start, g.end) + g.ext.0 + g.ext.1, 0.).map_err(k)
        } else {
            let fr = Frame { origin: g.start, x: g.x, normal: g.z };
            let face = Shape::face(&fr, &loops).map_err(k)?;
            face.sweep(&g.pts, 0).map_err(k)
        }
    }

    /// Linhas de todos os membros estruturais ativos: (comando, índice da linha, posição do
    /// comando na árvore, geometria).
    fn all_member_lines(&self) -> Vec<(Id, usize, usize, MemberGeom)> {
        let mut out = vec![];
        for (k, f) in self.doc.features.iter().enumerate() {
            if !self.doc.is_active(k) {
                continue;
            }
            let FeatureKind::StructMember(m) = &f.kind else { continue };
            if let Ok(gs) = self.members(m) {
                out.extend(gs.into_iter().enumerate().map(|(i, g)| (f.id, i, k, g)));
            }
        }
        out
    }

    /// Regra do canto entre duas linhas (comando, linha, posição): (tratamento, `a` é o membro 1,
    /// folga). Manda o comando posterior: a tabela de cantos dele, senão o tratamento padrão
    /// (o anterior é o membro 1).
    fn joint_rule(&self, a: (Id, usize, usize), b: (Id, usize, usize)) -> Result<(CornerMode, bool, f64), String> {
        let (early, late) = if (a.2, a.1) <= (b.2, b.1) { (a, b) } else { (b, a) };
        let Some(FeatureKind::StructMember(m)) = self.doc.feature(late.0).map(|f| &f.kind) else { return Ok((CornerMode::Nenhum, true, 0.)) };
        // a regra pode estar no posterior (vale primeiro) ou no anterior
        let hit = m.joints.iter().find(|j| j.mine == late.1 && j.other == early.0 && j.other_path == early.1).map(|j| (j.mode, j.first, j.gap.clone()));
        let hit = hit.or_else(|| match self.doc.feature(early.0).map(|f| &f.kind) {
            Some(FeatureKind::StructMember(e)) => e.joints.iter().find(|j| j.mine == early.1 && j.other == late.0 && j.other_path == late.1).map(|j| (j.mode, !j.first, j.gap.clone())),
            _ => None,
        });
        let (mode, late_first, gap) = match hit {
            Some((mode, first, gap)) => (mode, first, gap.unwrap_or_else(|| m.gap.clone())),
            None => (m.corner, false, m.gap.clone()),
        };
        let gap = ev(&gap, &self.vars, "folga de solda")?;
        Ok((mode, if a == late { late_first } else { !late_first }, gap))
    }

    /// Mensagem de referência perdida (ou ambígua) de um comando: nunca se escolhe em silêncio.
    fn lost(&self, what: &str, f: &Feature, why: refs::Lost, who: String) -> String {
        match why {
            refs::Lost::None => format!("referência perdida: {what} '{}' — {who} não existe mais; escolha de novo no comando", f.name),
            refs::Lost::Ambiguous(n) => format!("referência perdida: {what} '{}' — {who} tem {n} candidatas; escolha de novo no comando", f.name),
        }
    }

    /// Pontos das faces escolhidas que vão para o kernel (ADR 0010): a face com chave v2 vira o
    /// ponto atual dela; a que só tem o ponto segue o ponto, e a chave dela é calculada para o app
    /// gravar no documento (`RecomputeResult.promoted`). Face perdida ou ambígua = erro.
    #[allow(clippy::too_many_arguments)]
    fn resolve_faces(&self, what: &str, f: &Feature, b: &Shape, owners: &[Id], roles: &[i32], pts: &[[f64; 3]], keys: &[Option<FaceKey>]) -> Result<Vec<[f64; 3]>, String> {
        if pts.is_empty() {
            return Ok(vec![]);
        }
        let k = |e: KernelError| e.0;
        let topo = b.topology().map_err(k)?;
        let v = refs::View { topo: &topo, owners, roles };
        let (mut out, mut all, mut gained) = (Vec::with_capacity(pts.len()), Vec::with_capacity(pts.len()), false);
        for (i, p) in pts.iter().enumerate() {
            match keys.get(i).and_then(|x| x.as_ref()) {
                Some(key) => match refs::find_face(&v, key, Some(*p)) {
                    Ok(j) => {
                        out.push(topo.faces[j].point);
                        all.push(Some(key.clone()));
                    }
                    Err(why) => {
                        let who = format!("a face de '{}' ({})", self.doc.name_of(key.owner), refs::role_name(key.role));
                        return Err(self.lost(what, f, why, who));
                    }
                },
                None => {
                    // sem chave, o ponto só vale a até 1 mm da face; fora disso é referência perdida
                    let Some(j) = b.face_index(*p, 1.0).map_err(k)? else {
                        let who = format!("a face que passava perto de ({:.2}, {:.2}, {:.2})", p[0], p[1], p[2]);
                        return Err(self.lost(what, f, refs::Lost::None, who));
                    };
                    out.push(*p);
                    let key = refs::promote_face(&v, j);
                    gained |= key.is_some();
                    all.push(key);
                }
            }
        }
        if gained {
            *self.promoted.borrow_mut() = Some(Promoted::Faces(all));
        }
        Ok(out)
    }

    /// Mesmo que `resolve_faces`, para arestas: a chave é o par de faces vizinhas e o ponto
    /// resolvido é o ponto médio atual da aresta.
    #[allow(clippy::too_many_arguments)]
    fn resolve_edges(&self, what: &str, f: &Feature, b: &Shape, owners: &[Id], roles: &[i32], mids: &[[f64; 3]], keys: &[Option<EdgeKey>]) -> Result<Vec<[f64; 3]>, String> {
        if mids.is_empty() {
            return Ok(vec![]);
        }
        let k = |e: KernelError| e.0;
        let topo = b.topology().map_err(k)?;
        let v = refs::View { topo: &topo, owners, roles };
        // ADR 0010: sem chave, o ponto só vale se a aresta continua onde estava. Antes a tolerância era
        // 5% da diagonal e o ponto antigo caía em outra aresta sem aviso (a 0,8 mm, por exemplo, o
        // encontro piso-parede do bolso). O ponto guardado é o médio exato da aresta no clique, então
        // geometria que não mudou casa a ~1e-9; 0,1 mm (bem abaixo do teto de 1 mm da ADR) só absorve
        // o arredondamento da gravação em texto. Qualquer deslocamento maior é referência perdida.
        // Exceção temporária: em chapa as faces ainda não têm papel (fase 2 da ADR 0010), e o filete
        // dobrado precisa seguir o ponto até a planificação, então vale a tolerância antiga.
        let tol = if self.doc.features.iter().any(|g| matches!(g.kind, FeatureKind::SmWall(_))) { b.bbox().map(|(a, c)| geo3d::dist(a, c) * 0.05).unwrap_or(1.).max(1.) } else { 0.1 };
        let (mut out, mut all, mut gained) = (Vec::with_capacity(mids.len()), Vec::with_capacity(mids.len()), false);
        for (i, p) in mids.iter().enumerate() {
            match keys.get(i).and_then(|x| x.as_ref()) {
                Some(key) => match refs::find_edge(&v, key, Some(*p)) {
                    Ok(e) => {
                        out.push(topo.edge_mids[e]);
                        all.push(Some(key.clone()));
                    }
                    Err(why) => {
                        let face = |fk: &FaceKey| format!("'{}' ({})", self.doc.name_of(fk.owner), refs::role_name(fk.role));
                        let who = format!("a aresta entre as faces {} e {}", face(&key.a), face(&key.b));
                        return Err(self.lost(what, f, why, who));
                    }
                },
                None => {
                    // sem chave e sem aresta a 1 mm do ponto: erro com sugestão, nunca a mais próxima
                    let Some(e) = refs::edge_near(&v, *p, tol) else {
                        let mut who = format!("a aresta que passava perto de ({:.2}, {:.2}, {:.2})", p[0], p[1], p[2]);
                        // K-13: em aresta circular o ponto médio fica do lado oposto da costura; diz qual é
                        if let Some((e, d)) = refs::edge_nearest(&v, *p) {
                            let m = topo.edge_mids[e];
                            who.push_str(&format!(" (a mais próxima está a {d:.2} mm, com ponto médio em ({:.4}, {:.4}, {:.4}); passe esse ponto ou escolha a aresta pela chave)", m[0], m[1], m[2]));
                        }
                        return Err(self.lost(what, f, refs::Lost::None, who));
                    };
                    // o ponto exato da aresta achada vai para o kernel, não o antigo
                    out.push(topo.edge_mids[e]);
                    let key = refs::promote_edge(&v, e);
                    gained |= key.is_some();
                    all.push(key);
                }
            }
        }
        if gained {
            *self.promoted.borrow_mut() = Some(Promoted::Edges(all));
        }
        Ok(out)
    }

    /// Filete/chanfro "mínimo": o kernel sempre segue a cadeia tangente; aqui o resultado só vale
    /// dentro de tubos em volta das arestas escolhidas (tampas normais às pontas), e fora dele
    /// o corpo fica como estava: resultado = (corpo − (tubos − cadeia)) ∪ (tubos ∩ cadeia).
    #[allow(clippy::too_many_arguments)]
    fn minimal_edges(&self, b: &Shape, owners: &[Id], roles: &[i32], s: &Shape, own_s: &[Id], role_s: &[i32], mids: &[[f64; 3]], v: f64, range: Option<(f64, f64)>, fid: Id) -> Result<(Shape, Vec<Id>, Vec<i32>), String> {
        let k = |e: KernelError| e.0;
        let size = b.bbox().map(|(a, c)| geo3d::dist(a, c)).unwrap_or(100.).max(1.);
        let m = tesselar(b, &self.qualidade).map_err(k)?;
        let nseg = m.edges.len() / 6;
        let r = 3. * v + 0.5;
        let mut tubes = Shape::empty();
        for q in mids {
            // aresta da malha com o ponto médio mais perto
            let best = (0..m.edge_mids.len() / 3)
                .map(|e| (e, geo3d::dist([m.edge_mids[e * 3] as f64, m.edge_mids[e * 3 + 1] as f64, m.edge_mids[e * 3 + 2] as f64], *q)))
                .min_by(|a, c| a.1.total_cmp(&c.1));
            let Some((e, _)) = best else { continue };
            let s0 = m.edge_ranges[e] as usize;
            let s1 = m.edge_ranges.get(e + 1).map(|x| *x as usize).unwrap_or(nseg);
            let mut pts: Vec<V3> = vec![];
            for g in m.edges.chunks_exact(6).take(s1).skip(s0) {
                if pts.is_empty() {
                    pts.push([g[0] as f64, g[1] as f64, g[2] as f64]);
                }
                pts.push([g[3] as f64, g[4] as f64, g[5] as f64]);
            }
            // filete parcial: só o trecho pedido da aresta (mm contados do começo dela)
            if let Some((a, c)) = range {
                pts = arestas::sub_polyline(&pts, a, c);
            }
            if pts.len() < 2 {
                continue;
            }
            let z = geo3d::unit(geo3d::sub(pts[1], pts[0])).ok_or("aresta degenerada")?;
            let fr = Frame { origin: pts[0], x: geo3d::any_perp(z), normal: z };
            let face = Shape::face(&fr, &[vec![fenkra_kernel::Seg2::Circle([0., 0.], r)]]).map_err(k)?;
            let straight = pts.len() == 2;
            let tube = if straight { face.extrude(z, geo3d::dist(pts[0], pts[1]), 0.).map_err(k)? } else { face.sweep(&pts, 0).map_err(k)? };
            tubes = if tubes.is_null() { tube } else { tubes.boolean(&tube, Bool::Fuse).map_err(k)? };
        }
        if tubes.is_null() {
            return Ok((s.clone(), own_s.to_vec(), role_s.to_vec()));
        }
        // tira o material que o filete removeria só dentro dos tubos
        let remove = tubes.boolean(s, Bool::Cut).map_err(k)?;
        let (r1, out1) = b.boolean_tagged(&remove, Bool::Cut, &tags_for(owners)).map_err(k)?;
        let (r1, own1, role1) = remap_body(r1, owners, roles, &out1, fid);
        // e põe o que ele acrescentaria (cantos côncavos), se houver
        let add = tubes.boolean(s, Bool::Common).map_err(k)?.boolean(&r1, Bool::Cut).map_err(k)?;
        if add.is_null() || add.mass().volume < 1e-6 * size * size * size {
            return Ok((r1, own1, role1));
        }
        let (r2, out2) = r1.boolean_tagged(&add, Bool::Fuse, &tags_for(&own1)).map_err(k)?;
        Ok(remap_body(r2, &own1, &role1, &out2, fid))
    }

    /// Varredura avançada: seções calculadas em planos normais à spine e unidas por multi-seções.
    #[allow(clippy::too_many_arguments)]
    fn sweep_x(&self, kind: &SweepKind, spine: Option<&CurveRef>, solution: usize, smooth: bool, n: usize, d: u32, e: &dyn Fn(&String, &str) -> Result<f64, String>) -> Result<Shape, String> {
        use geo3d::*;
        let gs: Vec<Curve3> = match kind {
            SweepKind::Explicit { guide1, guide2, .. } => std::iter::once(guide1).chain(guide2.as_ref()).map(|c| self.curve(c, d)).collect::<Result<_, _>>()?,
            k => k.guides().into_iter().map(|c| self.curve(c, d)).collect::<Result<_, _>>()?,
        };
        let mut sp = match spine {
            Some(s) => self.curve(s, d)?,
            None => gs[0].clone(),
        };
        if smooth {
            let pts = resample(&sp, 10);
            sp = spline(&pts, false)?;
        }
        let o: Vec<V3> = resample(&sp, n);
        let n = o.len();
        if n < 2 {
            return Err("a spine é curta demais".into());
        }
        let tan = |i: usize| unit(sub(o[(i + 1).min(n - 1)], o[i.saturating_sub(1)])).unwrap_or([0., 0., 1.]);
        // ponto da guia no plano da seção (o cruzamento mais perto da spine)
        let on_plane = |c: &Curve3, p: V3, t: V3| -> V3 {
            let mut best: Option<(f64, V3)> = None;
            for w in c.pts.windows(2) {
                let (a, b) = (dot(sub(w[0], p), t), dot(sub(w[1], p), t));
                if (a <= 0. && b >= 0.) || (a >= 0. && b <= 0.) {
                    let q = if (a - b).abs() < 1e-15 { w[0] } else { lerp(w[0], w[1], a / (a - b)) };
                    let dd = dist(q, p);
                    if best.is_none_or(|x| dd < x.0) {
                        best = Some((dd, q));
                    }
                }
            }
            best.map(|x| x.1).unwrap_or_else(|| c.nearest(p).1)
        };
        // referencial que minimiza a rotação (dupla reflexão)
        let t0 = tan(0);
        let g0 = on_plane(&gs[0], o[0], t0);
        let mut r = unit(sub(sub(g0, o[0]), mul(t0, dot(sub(g0, o[0]), t0)))).unwrap_or_else(|| any_perp(t0));
        let mut frames = vec![(o[0], r, cross(t0, r), t0)];
        for i in 1..n {
            let (ti_1, ri_1) = (frames[i - 1].3, frames[i - 1].1);
            let v1 = sub(o[i], o[i - 1]);
            let c1 = dot(v1, v1).max(1e-30);
            let rl = sub(ri_1, mul(v1, 2. / c1 * dot(v1, ri_1)));
            let tl = sub(ti_1, mul(v1, 2. / c1 * dot(v1, ti_1)));
            let ti = tan(i);
            let v2 = sub(ti, tl);
            let c2 = dot(v2, v2);
            r = if c2 < 1e-30 { rl } else { sub(rl, mul(v2, 2. / c2 * dot(v2, rl))) };
            r = unit(r).unwrap_or(rl);
            frames.push((o[i], r, cross(ti, r), ti));
        }
        let g = |k: usize, i: usize| on_plane(&gs[k], o[i], frames[i].3);
        let arc_n = 24;
        let mut sections: Vec<Vec<V3>> = vec![];
        for i in 0..n {
            let (oi, xi, yi, ti) = frames[i];
            let sec: Vec<V3> = match kind {
                SweepKind::Explicit { profile, guide2, .. } => {
                    let prof = self.curve(profile, d)?;
                    let (o0, x0, y0, z0) = frames[0];
                    let loc = |p: V3| {
                        let q = sub(p, o0);
                        (dot(q, x0), dot(q, y0), dot(q, z0))
                    };
                    let loc_i = |p: V3| {
                        let q = sub(p, oi);
                        (dot(q, xi), dot(q, yi))
                    };
                    // semelhança no plano da seção que leva as guias do início às de agora
                    let (z1, w1) = (loc(g(0, 0)), loc_i(g(0, i)));
                    let (a, b) = match guide2 {
                        Some(_) => {
                            let (z2, w2) = (loc(g(1, 0)), loc_i(g(1, i)));
                            let dz = (z2.0 - z1.0, z2.1 - z1.1);
                            let dw = (w2.0 - w1.0, w2.1 - w1.1);
                            let den = dz.0 * dz.0 + dz.1 * dz.1;
                            if den < 1e-18 {
                                return Err("as duas guias se tocam no início da varredura".into());
                            }
                            let a = ((dw.0 * dz.0 + dw.1 * dz.1) / den, (dw.1 * dz.0 - dw.0 * dz.1) / den);
                            let b = (w1.0 - (a.0 * z1.0 - a.1 * z1.1), w1.1 - (a.0 * z1.1 + a.1 * z1.0));
                            (a, b)
                        }
                        None => {
                            let m = z1.0 * z1.0 + z1.1 * z1.1;
                            if m < 1e-12 {
                                ((1., 0.), (w1.0 - z1.0, w1.1 - z1.1))
                            } else {
                                let a = ((w1.0 * z1.0 + w1.1 * z1.1) / m, (w1.1 * z1.0 - w1.0 * z1.1) / m);
                                (a, (0., 0.))
                            }
                        }
                    };
                    resample(&prof, arc_n.max(prof.pts.len().min(64)))
                        .into_iter()
                        .map(|p| {
                            let (u, v, w) = loc(p);
                            let (u2, v2) = (a.0 * u - a.1 * v + b.0, a.0 * v + a.1 * u + b.1);
                            add(add(add(oi, mul(xi, u2)), mul(yi, v2)), mul(ti, w))
                        })
                        .collect()
                }
                SweepKind::Linear { .. } => vec![g(0, i), g(1, i)],
                SweepKind::LinearDir { length, angle, .. } => {
                    let (l, a) = (e(length, "comprimento")?, e(angle, "ângulo")?.to_radians());
                    let p = g(0, i);
                    let dir = add(mul(xi, a.cos()), mul(yi, a.sin()));
                    vec![p, add(p, mul(dir, l))]
                }
                SweepKind::Circle3 { .. } => {
                    let c = circle3_curve(g(0, i), g(1, i), g(2, i), solution % 2 == 1)?;
                    resample(&c, arc_n)
                }
                SweepKind::CircleRadius { radius, .. } => {
                    let rr = e(radius, "raio")?;
                    let (p, q) = (g(0, i), g(1, i));
                    let m = lerp(p, q, 0.5);
                    let h = dist(p, q) / 2.;
                    if rr < h - 1e-9 {
                        return Err(format!("raio {rr} menor que a metade da distância entre as guias ({h:.3}) numa seção"));
                    }
                    let s = (rr * rr - h * h).max(0.).sqrt();
                    let perp = unit(cross(ti, sub(q, p))).ok_or("as guias se tocam numa seção")?;
                    let side = if solution % 2 == 0 { 1. } else { -1. };
                    let ctr = add(m, mul(perp, s * side));
                    let (u, v) = (unit(sub(p, ctr)).unwrap(), unit(sub(q, ctr)).unwrap());
                    let w = unit(cross(cross(u, v), u)).unwrap_or(perp);
                    let mut ang = dot(u, v).clamp(-1., 1.).acos();
                    if solution >= 2 {
                        ang -= std::f64::consts::TAU;
                    }
                    (0..arc_n).map(|k| {
                        let t = ang * k as f64 / (arc_n - 1) as f64;
                        add(ctr, add(mul(u, rr * t.cos()), mul(w, rr * t.sin())))
                    })
                    .collect()
                }
                SweepKind::CircleCenter { radius, .. } => {
                    let rr = e(radius, "raio")?;
                    let c = g(0, i);
                    let mut v: Vec<V3> = (0..arc_n).map(|k| {
                        let t = std::f64::consts::TAU * k as f64 / arc_n as f64;
                        add(c, add(mul(xi, rr * t.cos()), mul(yi, rr * t.sin())))
                    })
                    .collect();
                    v.push(v[0]);
                    v
                }
                SweepKind::Conic { height, rho, .. } => {
                    let (hh, rho) = (e(height, "altura do ápice")?, e(rho, "ρ")?);
                    if !(0.01..=0.99).contains(&rho) {
                        return Err("ρ precisa estar entre 0 e 1 (0,5 = parábola)".into());
                    }
                    let (p0, p2) = (g(0, i), g(1, i));
                    let perp = unit(cross(ti, sub(p2, p0))).ok_or("as guias se tocam numa seção")?;
                    let side = if solution % 2 == 0 { 1. } else { -1. };
                    let p1 = add(lerp(p0, p2, 0.5), mul(perp, hh * side));
                    let wgt = rho / (1. - rho);
                    (0..arc_n).map(|k| {
                        let t = k as f64 / (arc_n - 1) as f64;
                        let (b0, b1, b2) = ((1. - t) * (1. - t), 2. * t * (1. - t) * wgt, t * t);
                        let den = b0 + b1 + b2;
                        [0, 1, 2].map(|j| (b0 * p0[j] + b1 * p1[j] + b2 * p2[j]) / den)
                    })
                    .collect()
                }
            };
            sections.push(sec);
        }
        Shape::loft_surface(&sections, false, false, None).map_err(|x| format!("varredura: {}", x.0))
    }

    /// Membro (de comandos estruturais anteriores) com a ponta mais perto de `at`. Um ponto sobre a
    /// face de uma ponta (face clicada, B255) escolhe essa ponta mesmo que outra esteja mais perto.
    fn member_end(&self, at: V3, me: &Feature) -> Result<(MemberGeom, bool), String> {
        let mut best: Option<(f64, MemberGeom, bool)> = None;
        for f in self.doc.features.iter().take_while(|g| g.id != me.id) {
            let FeatureKind::StructMember(m) = &f.kind else { continue };
            for g in self.members(m)? {
                for (p, s) in [(g.start, true), (g.end, false)] {
                    let d = geo3d::sub(at, p);
                    let along = geo3d::dot(d, g.z);
                    let lateral = geo3d::dist(d, geo3d::mul(g.z, along));
                    // fora da face, vale a distância; sobre a face (plano da ponta e dentro da seção), vale o lateral
                    let on_face = along.abs() < 1e-3 && lateral <= (g.w * g.w + g.h * g.h).sqrt() / 2. + 1e-3;
                    let dd = if on_face { lateral * 1e-3 } else { geo3d::dist(p, at) + 1e-3 };
                    if best.as_ref().is_none_or(|b| dd < b.0) {
                        best = Some((dd, g.clone(), s));
                    }
                }
            }
        }
        best.map(|(_, g, s)| (g, s)).ok_or_else(|| "nenhum membro estrutural antes deste comando".into())
    }

    /// Dois membros que se encontram perto de `at`: (geometria, direção para dentro de cada um) e o ponto.
    fn member_pair(&self, at: V3, me: &Feature) -> Result<((MemberGeom, V3), (MemberGeom, V3), V3), String> {
        let mut ends: Vec<(f64, MemberGeom, V3, V3)> = vec![];
        for f in self.doc.features.iter().take_while(|g| g.id != me.id) {
            let FeatureKind::StructMember(m) = &f.kind else { continue };
            for g in self.members(m)? {
                ends.push((geo3d::dist(g.start, at), g.clone(), g.z, g.start));
                ends.push((geo3d::dist(g.end, at), g.clone(), geo3d::mul(g.z, -1.), g.end));
            }
        }
        ends.sort_by(|a, b| a.0.total_cmp(&b.0));
        let first = ends.first().cloned().ok_or("nenhum membro estrutural antes deste comando")?;
        let second = ends.iter().skip(1).find(|e| geo3d::dist(e.3, first.3) < 1e-4).cloned().ok_or("o ponto precisa ser o encontro de dois membros")?;
        Ok(((first.1, first.2), (second.1, second.2), first.3))
    }

    /// Parâmetros de chapa avaliados.
    fn sheet(&self) -> Result<SheetValues, String> {
        // com estilo escolhido (B283), cada campo vem do estilo ou da sobrescrita da peça
        let s = &self.doc.sheet_efetivo().ok_or("defina a chapa (espessura, raio, fator K) em Chapa › Parâmetros")?;
        let t = ev(&s.thickness, &self.vars, "espessura da chapa")?;
        let r = ev(&s.radius, &self.vars, "raio de dobra")?;
        let k0 = if s.method == 0 { ev(&s.k, &self.vars, "fator K")? } else { 0.44 };
        let allowance = if s.method == 0 { 0. } else { ev(&s.allowance, &self.vars, "dedução/tolerância de dobra")? };
        let relief = if s.relief.trim().is_empty() { None } else { Some(ev(&s.relief, &self.vars, "alívio padrão da chapa")?) };
        Ok(SheetValues { t, r, flat: s.flat || chapa_desdobra::desdobrada_agora(self.doc), method: s.method, k0, allowance, relief, relief_kind: s.relief_kind })
    }

    /// Aresta da aba/bainha por dono + papel (ADR 0010): com chave v2, o ponto vira o ponto médio
    /// atual da aresta lógica e some o risco de a aba cair na vizinha quando a peça muda; sem chave,
    /// a chave é calculada (promoção) e o ponto segue como está. A chave só vale no estado da chapa
    /// em que a aresta foi escolhida (o planificado tem outras faces); fora dele segue o ponto,
    /// levado de um estado ao outro. Devolve o ponto e o estado (planificado?) em que ele está.
    fn chapa_aresta(&self, what: &str, f: &Feature, p: V3, key: Option<&EdgeKey>, flat_now: bool) -> Result<(V3, bool), String> {
        if f.flat_space != flat_now {
            return Ok((p, f.flat_space));
        }
        let Some((b, owners, roles)) = self.body_owned(f.target) else { return Ok((p, f.flat_space)) };
        // sem chave, o clique pode estar longe do meio da aresta que uma aba vizinha esticou: o kernel
        // acha a aresta que passa pelo ponto e recusa se não houver nenhuma
        match self.resolve_edges(what, f, b, owners, roles, &[p], &[key.cloned()]) {
            Ok(pts) => Ok((pts[0], flat_now)),
            Err(_) if key.is_none() => Ok((p, flat_now)),
            Err(e) => Err(e),
        }
    }

    /// Uma dobra por linha: a linha `a`–`b` (centro da zona dobrada) numa face plana. Com `build`,
    /// devolve também a chapa dobrada; sempre devolve o mapa da dobra no espaço de `body`. A parte
    /// que gira anda `-ba` antes do giro: o comprimento desenvolvido sai da zona dobrada.
    #[allow(clippy::too_many_arguments)]
    fn bend_line(&self, body: &Shape, a: V3, b: V3, ang: f64, r: f64, kf: f64, t: f64, flip: bool, down: bool, delta: f64, build: bool) -> Result<(Option<Shape>, SheetMap), String> {
        let k = |e: KernelError| e.0;
        let mid = geo3d::lerp(a, b, 0.5);
        // normal para fora da face clicada: o material fica do lado oposto
        let (on, n) = body.surface_near(mid).map_err(k)?;
        let e = geo3d::unit(geo3d::sub(b, a)).ok_or("os dois pontos da linha coincidem")?;
        let mut side = geo3d::unit(fenkra_kernel::cross(n, e)).ok_or("linha normal à chapa")?;
        if flip {
            side = geo3d::mul(side, -1.);
        }
        // a linha vai para a face (um sketch pode estar afastado dela) e para o centro da zona
        let a = geo3d::add(geo3d::sub(a, geo3d::mul(n, geo3d::dot(geo3d::sub(a, on), n))), geo3d::mul(side, delta));
        let (lo, hi) = body.bbox().ok_or("corpo sem dimensões")?;
        let big = geo3d::dist(lo, hi) * 2. + 10.;
        let ba = chapa::compensacao(t, r, ang, kf).ba;
        let half_space = |d: f64| {
            let p = geo3d::add(a, geo3d::mul(side, d));
            let y = fenkra_kernel::cross(side, e);
            let o = geo3d::sub(geo3d::sub(p, geo3d::mul(e, big)), geo3d::mul(y, big));
            Shape::box_tool(o, e, side, [2. * big, 2. * big, big])
        };
        let moving = body.boolean(&half_space(ba / 2.).map_err(k)?, fenkra_kernel::Bool::Common).map_err(k)?;
        let p0 = geo3d::sub(a, geo3d::mul(side, ba / 2.));
        let section = body.boolean(&Shape::plane_face(p0, side, big).map_err(k)?, fenkra_kernel::Bool::Common).map_err(k)?;
        if section.is_null() || moving.is_null() {
            return Err("a linha da dobra não atravessa a chapa".into());
        }
        // alcance da chapa ao longo da linha (largura da dobra) e da parte que gira (comprimento)
        let corners = |s: &Shape| -> Vec<V3> {
            s.bbox().map(|(l, h)| (0..8).map(|c| [if c & 1 == 0 { l[0] } else { h[0] }, if c & 2 == 0 { l[1] } else { h[1] }, if c & 4 == 0 { l[2] } else { h[2] }]).collect()).unwrap_or_default()
        };
        let (mut e0, mut e1) = (f64::MAX, f64::MIN);
        for q in corners(&section) {
            let d = geo3d::dot(geo3d::sub(q, p0), e);
            e0 = e0.min(d);
            e1 = e1.max(d);
        }
        let smax = corners(&moving).iter().map(|q| geo3d::dot(geo3d::sub(*q, p0), side)).fold(ba, f64::max);
        let c0 = geo3d::add(p0, geo3d::mul(e, e0));
        let map = if down {
            SheetMap { c0: geo3d::sub(c0, geo3d::mul(n, t)), ev: e, l: e1 - e0, n: geo3d::mul(n, -1.), u: side, t, o: geo3d::sub(c0, geo3d::mul(n, t + r)), axis: fenkra_kernel::cross(n, side), a: ang.to_radians(), ba, length: smax - ba, down: true, feature: Id(0) }
        } else {
            SheetMap { c0, ev: e, l: e1 - e0, n, u: side, t, o: geo3d::add(c0, geo3d::mul(n, r)), axis: fenkra_kernel::cross(side, n), a: ang.to_radians(), ba, length: smax - ba, down: false, feature: Id(0) }
        };
        if !build {
            return Ok((None, map));
        }
        let fixed = body.boolean(&half_space(-ba / 2.).map_err(k)?, fenkra_kernel::Bool::Cut).map_err(k)?;
        // eixo: raio interno acima da face clicada (para cima) ou abaixo da outra face
        let (axis_p, axis_d) = if down { (geo3d::sub(p0, geo3d::mul(n, t + r)), fenkra_kernel::cross(n, side)) } else { (geo3d::add(p0, geo3d::mul(n, r)), fenkra_kernel::cross(side, n)) };
        let bend = section.revolve(axis_p, axis_d, ang, 0.).map_err(k)?;
        let moved = moving.transformed(Xform::Translate(geo3d::mul(side, -ba))).map_err(k)?.transformed(Xform::Rotate(axis_p, axis_d, ang)).map_err(k)?;
        let out = fixed.boolean(&bend, fenkra_kernel::Bool::Fuse).map_err(k)?.boolean(&moved, fenkra_kernel::Bool::Fuse).map_err(k)?;
        Ok((Some(out), map))
    }

    /// Aba/bainha: no estado planificado, a aresta clicada precisa existir na chapa plana — senão
    /// avisa que o planificado não é exato (aba sobre aba) em vez de inventar a geometria.
    fn sheet_tool(&mut self, f: &Feature, edge: V3, from_flat: bool, p: fenkra_kernel::SheetFlange, colide: bool) -> Result<Option<String>, String> {
        let k = |e: KernelError| e.0;
        let b = self.body(f.target).ok_or("a aba precisa de uma parede de chapa antes")?.clone();
        // a aresta foi escolhida num estado (dobrado ou planificado): leva ao estado atual
        let edge = self.map_point(edge, from_flat, p.flat);
        let (mut tool, info) = b.sm_flange(edge, p).map_err(k)?;
        self.last_sheet = Some(info);
        if std::mem::take(&mut self.miter_next) {
            // pontas a 45° no plano da aba: encontram a aba vizinha em meia-esquadria
            let e = geo3d::unit(geo3d::sub(info.ends[1], info.ends[0])).ok_or("aresta nula")?;
            let mid = geo3d::lerp(info.ends[0], info.ends[1], 0.5);
            let c = tool.mass().center;
            let v = geo3d::sub(c, mid);
            let d = geo3d::unit(geo3d::sub(v, geo3d::mul(e, geo3d::dot(v, e)))).ok_or("aba sem direção")?;
            let big = geo3d::dist(info.ends[0], info.ends[1]) + p.length * 2. + 10.;
            for (end, s) in [(info.ends[0], -1.), (info.ends[1], 1.)] {
                let m = geo3d::unit(geo3d::add(geo3d::mul(e, s), d)).ok_or("meia-esquadria")?;
                tool = tool.boolean(&half_space(end, m, big)?, Bool::Cut).map_err(k)?;
            }
        }
        // bainha (colide = false) aperta a chapa de propósito: só a aba comum é validada
        if colide && !p.flat {
            // abas vizinhas: a aba nova só encosta na chapa pela aresta; volume comum é colisão
            if let Some(msg) = chapa::colisao(&b, &tool, info.ends, p.off1, p.off2)? {
                return Err(msg);
            }
        }
        // registra a dobra no espaço dobrado (no planificado, volta pelas dobras anteriores)
        let back = |q: V3| if p.flat { self.map_point(q, true, false) } else { q };
        let (c0, c1) = (back(info.ends[0]), back(info.ends[1]));
        let (n, u) = if p.flat {
            let tip = |v: V3| geo3d::sub(back(geo3d::add(info.ends[0], v)), c0);
            (geo3d::unit(tip(info.normal)).unwrap_or(info.normal), geo3d::unit(tip(info.out)).unwrap_or(info.out))
        } else {
            (info.normal, info.out)
        };
        let a = p.angle.to_radians();
        self.sheet_maps.push(SheetMap {
            c0,
            ev: geo3d::unit(geo3d::sub(c1, c0)).unwrap_or([1., 0., 0.]),
            l: geo3d::dist(c0, c1),
            n,
            u,
            t: p.t,
            o: geo3d::add(c0, geo3d::mul(n, p.r)),
            axis: fenkra_kernel::cross(u, n),
            a,
            ba: chapa::compensacao(p.t, p.r, p.angle, p.k).ba,
            length: p.length,
            down: false,
            feature: Id(0),
        });
        // a lateral da aresta escolhida precisa ser a espessura da chapa (senão não é parede de chapa)
        if let Some(msg) = chapa::valida_lateral(info.thickness, p.t) {
            return Err(msg);
        }
        self.apply(f.target, tool, BoolOp::Add, f.id)?;
        Ok(None)
    }

    /// Face do perfil: perfis fechados (com parede fina, se pedida) ou, num perfil aberto, a
    /// parede fina em volta da cadeia (um lado, o outro, ou simétrica). Perfil aberto sem parede
    /// fina usa 1 mm e avisa.
    fn profile_face(&self, frame: &Frame, sk: &EvalSketch, thin: Option<&String>, side: u8) -> Result<(Shape, Option<String>), String> {
        let k = |e: KernelError| e.0;
        if !sk.loops.is_empty() {
            let mut face = Shape::face(frame, &sk.loops).map_err(k)?;
            if let Some(t) = thin {
                face = face.thin(ev(t, &self.vars, "espessura da parede")?).map_err(k)?;
            }
            return Ok((face, None));
        }
        if sk.open.is_empty() {
            return Err("o sketch não tem perfil (desenhe linhas, arcos ou splines)".into());
        }
        let (t, warn) = match thin {
            Some(t) => (ev(t, &self.vars, "espessura da parede")?, None),
            None => (1., Some("perfil aberto: feito como parede fina de 1 mm — ajuste em 'parede fina'".to_string())),
        };
        if t <= 0. {
            return Err("a espessura da parede precisa ser positiva".into());
        }
        let (a, b) = match side {
            1 => (0., -t),
            2 => (t / 2., -t / 2.),
            _ => (0., t),
        };
        let mut loops: Vec<fenkra_kernel::Loop2> = vec![];
        for ch in &sk.open {
            let mut p: Vec<[f64; 2]> = ch.clone();
            p.dedup_by(|x, y| (x[0] - y[0]).abs() + (x[1] - y[1]).abs() < 1e-9);
            let n = p.len();
            if n < 2 {
                continue;
            }
            // normais à esquerda, em esquadria nos vértices
            let segn = |i: usize| {
                let (q0, q1) = (p[i], p[i + 1]);
                let (dx, dy) = (q1[0] - q0[0], q1[1] - q0[1]);
                let l = (dx * dx + dy * dy).sqrt().max(1e-12);
                [-dy / l, dx / l]
            };
            let off = |d: f64| -> Vec<[f64; 2]> {
                (0..n)
                    .map(|i| {
                        let nm = if i == 0 {
                            segn(0)
                        } else if i == n - 1 {
                            segn(n - 2)
                        } else {
                            let (a1, a2) = (segn(i - 1), segn(i));
                            let m = [a1[0] + a2[0], a1[1] + a2[1]];
                            let l = (m[0] * m[0] + m[1] * m[1]).sqrt().max(1e-12);
                            let m = [m[0] / l, m[1] / l];
                            let c = (m[0] * a2[0] + m[1] * a2[1]).max(0.2);
                            [m[0] / c, m[1] / c]
                        };
                        [p[i][0] + nm[0] * d, p[i][1] + nm[1] * d]
                    })
                    .collect()
            };
            let mut ring = off(a);
            let mut back = off(b);
            back.reverse();
            ring.extend(back);
            let mut lp = vec![];
            for run in fenkra_core::sketch::smooth_runs(&ring, true) {
                if run.len() >= 4 {
                    lp.push(fenkra_kernel::Seg2::Spline(run.iter().map(|i| ring[*i]).collect()));
                } else {
                    for w in run.windows(2) {
                        lp.push(fenkra_kernel::Seg2::Line(ring[w[0]], ring[w[1]]));
                    }
                }
            }
            loops.push(lp);
        }
        Ok((Shape::face(frame, &loops).map_err(k)?, warn))
    }

    /// Aba ao longo de uma cadeia de pontos na borda da chapa: seção com o arco de dobra varrida
    /// pela cadeia (dobrado) ou a faixa desenvolvida (planificado). `want` força o lado da aba.
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::too_many_arguments)]
    fn curve_flange(&mut self, f: &Feature, pts: Vec<V3>, ang: f64, r: f64, len: f64, flip: bool, want: Option<V3>, side: u8) -> Result<Option<String>, String> {
        let k = |e: KernelError| e.0;
        let sp = self.sheet()?;
        if !(1.0..=179.0).contains(&ang) {
            return Err("ângulo da aba entre 1° e 179°".into());
        }
        let b = self.body(f.target).ok_or("a aba precisa de uma chapa")?.clone();
        // a aresta foi escolhida dobrada ou planificada: leva ao estado atual
        let mut pts = pts;
        pts.dedup_by(|a, b| geo3d::dist(*a, *b) < 1e-6);
        if pts.len() < 2 {
            return Err("aresta curta demais".into());
        }
        let mid = pts[pts.len() / 2];
        // normal da chapa: face perto da aresta, um pouco para dentro da peça
        let (lo, hi) = b.bbox().ok_or("chapa sem dimensões")?;
        let ctr = geo3d::lerp(lo, hi, 0.5);
        let inward = geo3d::unit(geo3d::sub(ctr, mid)).unwrap_or([0., 0., -1.]);
        let (_, n) = b.surface_near(geo3d::add(mid, geo3d::mul(inward, sp.t * 0.25))).map_err(k)?;
        // para fora (no plano da chapa), com o mesmo sentido ao longo da cadeia
        let tan = |i: usize| geo3d::unit(geo3d::sub(pts[(i + 1).min(pts.len() - 1)], pts[i.saturating_sub(1)])).unwrap_or([1., 0., 0.]);
        let im = pts.len() / 2;
        let w0 = geo3d::unit(fenkra_kernel::cross(tan(im), n)).ok_or("aresta normal à chapa")?;
        let s = if geo3d::dot(w0, geo3d::sub(mid, ctr)) >= 0. { 1. } else { -1. };
        let outs: Vec<V3> = (0..pts.len()).map(|i| geo3d::mul(geo3d::unit(fenkra_kernel::cross(tan(i), n)).unwrap_or(w0), s)).collect();
        // de que lado está a espessura (raio de teste a partir de dentro da chapa)
        let probe = geo3d::sub(mid, geo3d::mul(outs[im], sp.t * 0.5));
        let up_hit = b.ray_hit(geo3d::add(probe, geo3d::mul(n, -sp.t * 3.)), n).map_err(k)?.map(|(q, _)| geo3d::dot(geo3d::sub(q, probe), n));
        let mat_up = up_hit.is_some_and(|dd| dd > -sp.t * 0.5 && dd < sp.t * 1.5);
        let d = match want {
            Some(w) => if geo3d::dot(w, n) >= 0. { n } else { geo3d::mul(n, -1.) },
            None => if flip { geo3d::mul(n, -1.) } else { n },
        };
        // lado do material: a aba recua para dentro da chapa (a faixa entre a aresta e o recuo sai)
        let half = (ang.to_radians() / 2.).min(80f64.to_radians());
        let off = match side {
            1 => r * half.tan(),
            2 => (r + sp.t) * half.tan(),
            _ => 0.,
        };
        let pts: Vec<V3> = if off > 1e-6 {
            let back: Vec<V3> = pts.iter().zip(&outs).map(|(p, o)| geo3d::sub(*p, geo3d::mul(*o, off))).collect();
            let edge: Vec<V3> = pts.iter().zip(&outs).map(|(p, o)| geo3d::add(*p, geo3d::mul(*o, 0.5))).collect();
            let fr = Frame { origin: pts[0], x: outs[0], normal: n };
            let y = fenkra_kernel::cross(n, outs[0]);
            let loc = |p: V3| {
                let q = geo3d::sub(p, pts[0]);
                [geo3d::dot(q, outs[0]), geo3d::dot(q, y)]
            };
            let a2: Vec<[f64; 2]> = back.iter().map(|p| loc(*p)).collect();
            let mut b2: Vec<[f64; 2]> = edge.iter().map(|p| loc(*p)).collect();
            b2.reverse();
            let lp = [chapa::chain_segs(&a2), vec![fenkra_kernel::Seg2::Line(a2[a2.len() - 1], b2[0])], chapa::chain_segs(&b2), vec![fenkra_kernel::Seg2::Line(b2[b2.len() - 1], a2[0])]].concat();
            let strip = Shape::face(&fr, &[lp]).map_err(k)?.extrude(n, sp.t * 2. + 1., sp.t * 2. + 1.).map_err(k)?;
            self.apply(f.target, strip, BoolOp::Cut, f.id)?;
            back
        } else {
            pts.clone()
        };
        // face interna da dobra: a do lado para onde a aba sobe
        let shift = if (geo3d::dot(d, n) > 0.) == mat_up { geo3d::mul(d, sp.t) } else { [0.; 3] };
        let inner: Vec<V3> = pts.iter().map(|p| geo3d::add(*p, shift)).collect();
        let a = ang.to_radians();
        let cmp = chapa::compensacao(sp.t, r, ang, sp.k_for(r));
        let sobe = geo3d::dot(d, n) > 0.;
        if sp.flat {
            // planificada: faixa desenvolvida (arco pela linha neutra + parede) para fora
            let dev = cmp.ba + len;
            let fr = Frame { origin: inner[0], x: outs[0], normal: d };
            let loc = |p: V3| {
                let q = geo3d::sub(p, fr.origin);
                let y = fenkra_kernel::cross(fr.normal, fr.x);
                [geo3d::dot(q, fr.x), geo3d::dot(q, y)]
            };
            // começa 0,05 mm para dentro da chapa (fusão sem faces coincidentes)
            let p2: Vec<[f64; 2]> = inner.iter().zip(&outs).map(|(p, o)| loc(geo3d::sub(*p, geo3d::mul(*o, 0.05)))).collect();
            let mut q2: Vec<[f64; 2]> = inner.iter().zip(&outs).map(|(p, o)| loc(geo3d::add(*p, geo3d::mul(*o, dev)))).collect();
            q2.reverse();
            let lp = [chapa::chain_segs(&p2), vec![fenkra_kernel::Seg2::Line(p2[p2.len() - 1], q2[0])], chapa::chain_segs(&q2), vec![fenkra_kernel::Seg2::Line(q2[q2.len() - 1], p2[0])]].concat();
            let face = Shape::face(&fr, &[lp]).map_err(k)?;
            let tool = face.extrude(geo3d::mul(d, -1.), sp.t, 0.).map_err(k)?;
            self.apply(f.target, tool, BoolOp::Add, f.id)?;
            self.marca_cadeia(f.id, &inner, &outs, ang, r, sobe, &cmp, true);
            return Ok(None);
        }
        // seção no plano normal à aresta: arco da dobra (raio interno r, externo r+t) e parede
        // a normal da seção segue o sentido da varredura (senão o sólido sai do avesso);
        // com a normal trocada, o eixo v da seção vira −d
        let nsec = fenkra_kernel::cross(outs[0], d);
        let sv = if geo3d::dot(nsec, tan(0)) >= 0. { 1. } else { -1. };
        let fr = Frame { origin: inner[0], x: outs[0], normal: geo3d::mul(nsec, sv) };
        let arc = |rad: f64, t0: f64, t1: f64| -> Vec<[f64; 2]> {
            (0..=8).map(|i| {
                let t = t0 + (t1 - t0) * i as f64 / 8.;
                [rad * t.cos(), sv * (r + rad * t.sin())]
            }).collect()
        };
        let h = std::f64::consts::FRAC_PI_2;
        // ângulo da dobra medido a partir da base: o arco vai de −90° a −90° + a
        let inner_arc = arc(r, -h, -h + a);
        let outer_arc = arc(r + sp.t, -h + a, -h);
        let dir = [(-h + a).cos(), (-h + a).sin()];
        let up = [-dir[1], sv * dir[0]];
        let ie = inner_arc[inner_arc.len() - 1];
        let oe = outer_arc[0];
        let it = [ie[0] + up[0] * len, ie[1] + up[1] * len];
        let ot = [oe[0] + up[0] * len, oe[1] + up[1] * len];
        let arc3 = |v: &[[f64; 2]]| fenkra_kernel::Seg2::Arc(v[0], v[v.len() / 2], v[v.len() - 1]);
        // a seção entra 0,05 mm na chapa: a fusão encontra volume comum (sem faces coincidentes)
        let e0 = 0.05;
        let ob = outer_arc[outer_arc.len() - 1];
        let ib = inner_arc[0];
        let (ob2, ib2) = ([ob[0] - e0, ob[1]], [ib[0] - e0, ib[1]]);
        let lp = vec![
            arc3(&inner_arc),
            fenkra_kernel::Seg2::Line(ie, it),
            fenkra_kernel::Seg2::Line(it, ot),
            fenkra_kernel::Seg2::Line(ot, oe),
            arc3(&outer_arc),
            fenkra_kernel::Seg2::Line(ob, ob2),
            fenkra_kernel::Seg2::Line(ob2, ib2),
            fenkra_kernel::Seg2::Line(ib2, ib),
        ];
        let face = Shape::face(&fr, &[lp]).map_err(k)?;
        // cadeia reta: extrusão da seção (a varredura de Frenet numa reta sai inválida)
        let first = inner[0];
        let last = inner[inner.len() - 1];
        let axis = geo3d::sub(last, first);
        let straight = geo3d::unit(axis).is_some_and(|e| {
            inner.iter().all(|p| {
                let q = geo3d::sub(*p, first);
                geo3d::norm(geo3d::sub(q, geo3d::mul(e, geo3d::dot(q, e)))) < 1e-3
            })
        });
        let tool = if straight { face.extrude(geo3d::unit(axis).unwrap_or([1., 0., 0.]), geo3d::norm(axis), 0.).map_err(k)? } else { face.sm_chain_sweep(&inner, n).map_err(k)? };
        self.apply(f.target, tool, BoolOp::Add, f.id)?;
        self.marca_cadeia(f.id, &inner, &outs, ang, r, sobe, &cmp, false);
        Ok(None)
            
    }

    /// Leva um ponto do estado em que foi escolhido (`from_flat`) ao estado `to_flat`.
    fn map_point(&self, p: V3, from_flat: bool, to_flat: bool) -> V3 {
        if from_flat == to_flat || self.sheet_maps.is_empty() {
            return p;
        }
        let mut q = p;
        if to_flat {
            for m in self.sheet_maps.iter().rev() {
                if m.holds(q, false) {
                    q = m.to_flat(q);
                }
            }
        } else {
            for m in self.sheet_maps.iter() {
                if m.holds(q, true) {
                    q = m.to_folded(q);
                }
            }
        }
        q
    }

    /// Como `map_point`, mas os pontos que caem na zona de uma dobra também seguem o arco: serve a
    /// contornos que atravessam o raio (a prévia do recorte), onde `map_point` os deixaria parados.
    fn map_outline_point(&self, p: V3, from_flat: bool, to_flat: bool) -> V3 {
        if from_flat == to_flat || self.sheet_maps.is_empty() {
            return p;
        }
        let mut q = p;
        if to_flat {
            for m in self.sheet_maps.iter().rev() {
                // a zona vem antes: `holds` trata o arco como perna reta
                if let Some(z) = m.zone_unfold(q) {
                    q = z;
                } else if m.holds(q, false) {
                    q = m.to_flat(q);
                }
            }
        } else {
            for m in self.sheet_maps.iter() {
                if let Some(z) = m.zone_fold(q) {
                    q = z;
                } else if m.holds(q, true) {
                    q = m.to_folded(q);
                }
            }
        }
        q
    }

    /// Leva a ferramenta de um comando para o estado atual da chapa: o pedaço que cai em cada aba
    /// gira junto com ela (dobrado ↔ planificado); o resto fica onde está.
    fn map_tool(&self, tool: Shape, from_flat: bool, to_flat: bool) -> Result<Shape, String> {
        if from_flat == to_flat || self.sheet_maps.is_empty() || tool.is_null() {
            return Ok(tool);
        }
        let k = |e: KernelError| e.0;
        let order: Vec<&SheetMap> = if to_flat { self.sheet_maps.iter().rev().collect() } else { self.sheet_maps.iter().collect() };
        let mut tool = tool;
        let live = |s: &Shape| !s.is_null() && s.face_count() > 0;
        for m in order {
            // perna reta da aba: gira junto com ela
            let reg = m.region(from_flat)?;
            let leg = tool.boolean(&reg, Bool::Common).map_err(k)?;
            let mut moved_leg = None;
            let mut rest = tool.clone();
            if live(&leg) {
                rest = tool.boolean(&reg, Bool::Cut).map_err(k)?;
                moved_leg = Some(if to_flat {
                    leg.transformed(Xform::Rotate(m.o, m.axis, -m.a.to_degrees())).map_err(k)?.transformed(Xform::Translate(geo3d::mul(m.u, m.ba))).map_err(k)?
                } else {
                    leg.transformed(Xform::Translate(geo3d::mul(m.u, -m.ba))).map_err(k)?.transformed(Xform::Rotate(m.o, m.axis, m.a.to_degrees())).map_err(k)?
                });
            }
            // zona da dobra (planificado -> dobrado): cada fatia vai pelo arco, e o contorno atravessa
            // o raio da dobra. O que a ferramenta tem sobre a zona fora das lajes é descartado: é ar
            // dentro da dobra no planificado e cairia sobre a aba no dobrado.
            if from_flat && !to_flat && live(&rest) {
                let mut moved: Vec<Shape> = vec![];
                for (um, phi, slab) in m.bend_slices()? {
                    let part = rest.boolean(&slab, Bool::Common).map_err(k)?;
                    if live(&part) {
                        moved.push(m.move_slice(part, um, phi, false)?);
                    }
                }
                if !moved.is_empty() {
                    let outside = rest.boolean(&m.slab(0., m.ba, Some(1e4))?, Bool::Cut).map_err(k)?;
                    // as fatias se sobrepõem de propósito (sem frestas na face externa): funde tudo
                    // numa peça só, senão a ferramenta teria sólidos que se cruzam
                    let mut zone = moved[0].clone();
                    for p in moved.iter().skip(1).chain(live(&outside).then_some(&outside)) {
                        zone = zone.boolean(p, Bool::Fuse).map_err(k)?;
                    }
                    rest = zone;
                }
            }
            tool = match moved_leg {
                Some(l) if live(&rest) => rest.compound(&l),
                Some(l) => l,
                None => rest,
            };
        }
        Ok(tool)
    }

    /// O comando foi feito no outro estado da chapa (e não é ele mesmo um comando de chapa)?
    fn sheet_mismatch(&self, fid: Id) -> Option<(bool, bool)> {
        let sh = self.doc.sheet.as_ref()?;
        let f = self.doc.feature(fid)?;
        let own = matches!(
            f.kind,
            FeatureKind::SmWall(_)
                | FeatureKind::SmFlange(_)
                | FeatureKind::SmFlangeLoop(_)
                | FeatureKind::SmHem(_)
                | FeatureKind::SmJoggle(_)
                | FeatureKind::SmLouver(_)
                | FeatureKind::SmStamp(_)
                | FeatureKind::SmFlangedHole(_)
                | FeatureKind::SmCornerRelief(_)
                | FeatureKind::SmCornerClose(_)
                | FeatureKind::SmLoftFlange(_)
                | FeatureKind::SmUnfoldPartial(_)
                | FeatureKind::SmConvert(_)
                | FeatureKind::SmArcSlot(_)
                | FeatureKind::SmSurfFlange(_)
                | FeatureKind::SmProfileFlange(_)
                | FeatureKind::SmCurveFlange(_)
        );
        // recorte "normal à face" não acompanha a chapa pelas dobras: corta reto do plano do sketch
        let normal = matches!(&f.kind, FeatureKind::SmCut(c) if c.normal_face);
        (!own && !normal && f.flat_space != sh.flat && !self.sheet_maps.is_empty()).then_some((f.flat_space, sh.flat))
    }

    /// "Até o último": distância de `o` até o ponto do corpo mais distante na direção `dir`.
    fn last_dist(&self, target: Id, o: V3, dir: V3) -> Result<f64, String> {
        let b = self.body(target).ok_or("'até o último' precisa de um corpo no coletor")?;
        let (lo, hi) = b.bbox().ok_or("corpo sem dimensões")?;
        let mut t = f64::MIN;
        for c in 0..8 {
            let q = [if c & 1 == 0 { lo[0] } else { hi[0] }, if c & 2 == 0 { lo[1] } else { hi[1] }, if c & 4 == 0 { lo[2] } else { hi[2] }];
            t = t.max(dot(sub(q, o), dir));
        }
        if t <= 1e-6 {
            return Err("o corpo está todo do outro lado — inverta o sentido".into());
        }
        // a caixa envolvente pode passar um pouco do corpo: divide e fica com o trecho até a última parede
        Ok(t)
    }

    /// Superfície de término (até plano / superfície / próximo) como forma de corte e se ela deve ser
    /// estendida; `None` = término por distância. Mesmo critério para extrusão, recorte, furo e rosca.
    fn end_cutter(&self, e: &Extent, target: Id) -> Result<Option<(Shape, bool)>, String> {
        let k = |e: KernelError| e.0;
        let size = self.through(target).unwrap_or(0.).max(1e4);
        Ok(match e {
            Extent::UpTo(p) | Extent::UpToSurface(SurfRef::Plane(p)) => {
                let fr = self.plane(p, 0)?;
                Some((Shape::plane_face(fr.origin, fr.normal, size).map_err(k)?, false))
            }
            Extent::UpToSurface(sf) => Some((self.surface_shape(sf, 0)?, true)),
            Extent::UpToNext => Some((self.body(target).ok_or("'até o próximo' precisa de um corpo no coletor")?.clone(), false)),
            _ => None,
        })
    }

    /// Subtrai o sulco da rosca. A booleana do kernel pode falhar, ou devolver o corpo sem o sulco, sem
    /// erro (varredura de hélice contra o cilindro do furo): cada tentativa é conferida pelo volume e,
    /// se não cortou, desfeita e refeita no degrau seguinte da [`rosca_furo::ESCADA`]. Devolve `false`
    /// quando nenhum degrau cortou (o corpo fica como estava, sem sulco).
    fn corta_rosca(&mut self, target: Id, fid: Id, minimo: f64, ferramenta: &dyn Fn(usize) -> Result<Shape, String>) -> bool {
        for degrau in 0..rosca_furo::ESCADA.len() {
            let Ok(tool) = ferramenta(degrau) else { continue };
            let (corpo, ferr, passos) = (self.bodies.get(&target).cloned(), self.tools.get(&fid).cloned(), self.steps.get(&fid).cloned());
            let antes = corpo.as_ref().map_or(0., |b| b.0.mass().volume);
            if self.apply(target, tool, BoolOp::Cut, fid).is_ok() && antes - self.body(target).map_or(0., |b| b.mass().volume) >= minimo.max(1e-6) && self.body(target).is_some_and(|b| b.is_valid()) {
                return true;
            }
            // desfaz a tentativa: corpo, ferramenta guardada e passos voltam ao que eram
            match corpo {
                Some(c) => self.bodies.insert(target, c),
                None => self.bodies.remove(&target),
            };
            match ferr {
                Some(x) => self.tools.insert(fid, x),
                None => self.tools.remove(&fid),
            };
            match passos {
                Some(x) => self.steps.insert(fid, x),
                None => self.steps.remove(&fid),
            };
        }
        false
    }

    /// Comprimento de ferramenta que certamente passa da superfície de corte.
    fn reach(&self, cut: &Shape, from: V3, target: Id) -> f64 {
        let mut len = self.through(target).unwrap_or(0.);
        if let Some((a, b)) = cut.bbox() {
            for c in 0..8 {
                let q = [if c & 1 == 0 { a[0] } else { b[0] }, if c & 2 == 0 { a[1] } else { b[1] }, if c & 4 == 0 { a[2] } else { b[2] }];
                len = len.max(geo3d::dist(q, from) * 1.5 + 10.);
            }
        }
        len.min(1e5)
    }

    /// Corpo + dono (comando) de cada face, na ordem das faces do kernel.
    fn body_owned(&self, target: Id) -> Option<&Body> {
        self.bodies.get(&target).filter(|(s, _, _)| !s.is_null())
    }

    /// Aplica a ferramenta ao corpo do coletor; faces novas passam a pertencer ao comando `fid`.
    fn apply(&mut self, target: Id, tool: Shape, op: BoolOp, fid: Id) -> Result<(), String> {
        self.apply_roles(target, tool, op, fid, None)
    }

    /// Como `apply`, com o papel de cada face da ferramenta (extrusão e revolução): as faces que
    /// a ferramenta deixa no corpo levam esse papel, com o comando `fid` como dono.
    fn apply_roles(&mut self, target: Id, tool: Shape, op: BoolOp, fid: Id, tool_roles: Option<Vec<i32>>) -> Result<(), String> {
        let k = |e: KernelError| e.0;
        // a ferramenta levada entre o dobrado e o planificado perde a correspondência com os papéis
        let mismatch = self.sheet_mismatch(fid);
        let (tool, tool_roles) = match mismatch {
            Some((from, to)) => (self.map_tool(tool, from, to)?, None),
            None => (tool, tool_roles),
        };
        // guarda a ferramenta (furos com rosca somam várias)
        let kept = match self.tools.remove(&fid) {
            Some((t, o)) if o == op => t.compound(&tool),
            _ => tool.clone(),
        };
        self.tools.insert(fid, (kept, op));
        self.steps.entry(fid).or_default().push((tool.clone(), op));
        let next = match (self.body_owned(target), op) {
            (None, BoolOp::Add) => {
                let n = tool.face_count() as usize;
                let roles = tool_roles.filter(|r| r.len() == n).unwrap_or_else(|| vec![-1; n]);
                (tool, vec![fid; n], roles)
            }
            (None, _) => return Err("não há corpo no coletor de destino para esta operação".into()),
            (Some((b, owners, roles)), op) => {
                let kop = match op {
                    BoolOp::Add => Bool::Fuse,
                    BoolOp::Cut => Bool::Cut,
                    BoolOp::Intersect => Bool::Common,
                };
                let (s, out, src) = b.boolean_tagged_src(&tool, kop, &tags_for(owners)).map_err(k)?;
                // recorte desenhado no outro estado da chapa que não achou material depois de levado
                // para este (a aba por perfil não tem mapa de dobras): erro com o motivo, em vez de
                // sumir da planificada sem aviso (B86)
                if let (Some((from, to)), BoolOp::Cut) = (mismatch, op) {
                    if b.mass().volume - s.mass().volume < 1e-6 {
                        let onde = if to { "planificada" } else { "dobrada" };
                        let de = if from { "planificada" } else { "dobrada" };
                        return Err(format!("o recorte foi desenhado na chapa {de} e não encontrou material na chapa {onde}: a parte da chapa onde ele cai não tem mapa de dobras (aba por perfil, por exemplo). Refaça o recorte com a chapa {onde}"));
                    }
                }
                let own = remap(owners, &out, fid);
                // face do corpo: leva o papel que já tinha; face que veio da ferramenta: o papel dela
                let tool_roles = tool_roles.unwrap_or_default();
                let new_roles = out
                    .iter()
                    .zip(&src)
                    .map(|(t, sb)| {
                        if *t >= 0 {
                            roles.get(*t as usize).copied().unwrap_or(-1)
                        } else if *sb >= 0 {
                            tool_roles.get(*sb as usize).copied().unwrap_or(-1)
                        } else {
                            -1
                        }
                    })
                    .collect();
                (s, own, new_roles)
            }
        };
        self.bodies.insert(target, next);
        Ok(())
    }

    /// Repete as ferramentas dos comandos `sources` (ou o corpo inteiro) pelas transformações.
    fn replicate(&mut self, f: &Feature, sources: &[Id], xs: &[Xform]) -> Result<(), String> {
        let k = |e: KernelError| e.0;
        if sources.is_empty() {
            let b = self.body(f.target).ok_or("não há corpo para repetir")?.clone();
            let mut acc = Shape::empty();
            for x in xs {
                let c = b.transformed(*x).map_err(k)?;
                acc = if acc.is_null() { c } else { acc.compound(&c) };
            }
            return self.apply(f.target, acc, BoolOp::Add, f.id);
        }
        for src in sources {
            // todas as operações do comando, na ordem (estampo: soma e depois corte)
            let ops: Vec<(Shape, BoolOp)> = match self.steps.get(src) {
                Some(v) if v.len() > 1 => v.clone(),
                _ => vec![self.tools.get(src).cloned().ok_or_else(|| format!("'{}' não gera forma para repetir (ou vem depois)", self.doc.name_of(*src)))?],
            };
            for (tool, op) in ops {
                let mut acc = Shape::empty();
                for x in xs {
                    let c = tool.transformed(*x).map_err(k)?;
                    acc = if acc.is_null() { c } else { acc.compound(&c) };
                }
                self.apply(f.target, acc, op, f.id)?;
            }
        }
        Ok(())
    }

    /// Roda o comando e marca com ele as dobras que ele registrou (a tabela e as linhas de dobra
    /// dizem de qual comando cada dobra veio).
    fn run(&mut self, f: &Feature) -> Result<Option<String>, String> {
        let n0 = self.sheet_maps.len();
        chapa_desdobra::marca(f.id);
        let r = self.run_feature(f);
        chapa_desdobra::marca(Id(0));
        for m in self.sheet_maps.iter_mut().skip(n0) {
            m.feature = f.id;
        }
        r
    }

    fn run_feature(&mut self, f: &Feature) -> Result<Option<String>, String> {
        if let Some(r) = self.previa_leve(f) {
            return r;
        }
        let k = |e: KernelError| e.0;
        match &f.kind {
            FeatureKind::Sketch(s) => {
                let frame = profile::posiciona(self.plane(&s.plane, 0)?, &s.data.pos);
                // geometria externa (converter/projetar/interseção): reposiciona antes de resolver
                let mut link_warn = vec![];
                let mut quebrados = vec![];
                let mut link_ops = vec![];
                let data = if s.data.links.is_empty() {
                    std::borrow::Cow::Borrowed(&s.data)
                } else {
                    use fenkra_core::projecao::{projeta, reconhece};
                    use fenkra_core::sketch::{LinkPolicy, LinkShape};
                    use fenkra_core::vinculo::OpVinculo;
                    let mut d = s.data.clone();
                    // o documento não muda aqui: o que mudar de estrutura fica guardado para o editor
                    let mut aplica = |d: &mut SketchData, op: OpVinculo, nome: Option<&str>, warn: &mut Vec<String>| {
                        let ef = d.aplica_vinculo(&op);
                        if ef.perdidas > 0 {
                            warn.push(format!("geometria externa: a origem virou {}; {} restrição(ões) saíram", nome.unwrap_or("outra curva"), ef.perdidas));
                        }
                        if ef.sobras > 0 {
                            warn.push(format!("geometria externa: {} ponto(s) sem interseção ficaram no lugar porque têm restrição", ef.sobras));
                        }
                        if ef.refeito {
                            link_ops.push(op);
                        }
                    };
                    for l in &s.data.links {
                        // isolado fica como está, mesmo sem a origem
                        if l.policy == LinkPolicy::Isolado {
                            continue;
                        }
                        let soltos = matches!(l.shape, LinkShape::Poly { isolated: true, .. });
                        // a fórmula da origem manda, se a forma atual puder dar lugar a ela
                        if !soltos {
                            match self.link_exata(l, &frame) {
                                Ok(Some(forma)) if d.link_aceita_forma(l.id) => {
                                    aplica(&mut d, OpVinculo::Exata(l.id, forma), Some(forma.nome()), &mut link_warn);
                                    continue;
                                }
                                Ok(_) => {}
                                Err(e) => {
                                    quebrados.push(l.id);
                                    link_warn.push(format!("geometria externa: {e}"));
                                    continue;
                                }
                            }
                        }
                        match self.link_world(l, &frame) {
                            Ok(p3) if !p3.is_empty() => {
                                let y = frame.y();
                                let uv: Vec<[f64; 2]> = p3
                                    .iter()
                                    .map(|p| {
                                        let v = geo3d::sub(*p, frame.origin);
                                        [geo3d::dot(v, frame.x), geo3d::dot(v, y)]
                                    })
                                    .collect();
                                // sem fórmula na origem (corte de face, por exemplo), a forma vem dos pontos
                                let forma = if soltos { None } else { projeta(&reconhece(&p3), &profile::plano_do(&frame)).filter(|_| d.link_aceita_forma(l.id)) };
                                let op = match forma {
                                    _ if soltos => OpVinculo::Pontos(l.id, uv),
                                    Some(f) => OpVinculo::Exata(l.id, f),
                                    None => OpVinculo::Polilinha(l.id, uv),
                                };
                                aplica(&mut d, op, forma.as_ref().map(|f| f.nome()), &mut link_warn);
                            }
                            Ok(_) => {
                                quebrados.push(l.id);
                                link_warn.push("geometria externa sem interseção com o plano".to_string());
                            }
                            Err(e) => {
                                quebrados.push(l.id);
                                link_warn.push(format!("geometria externa: {e}"));
                            }
                        }
                    }
                    std::borrow::Cow::Owned(d)
                };
                let mut es = eval_sketch(&data, &self.vars)?;
                es.warnings.extend(link_warn);
                es.link_quebrados = quebrados;
                es.link_ops = link_ops;
                for l in data.links.iter().filter(|l| l.policy != fenkra_core::sketch::LinkPolicy::Isolado) {
                    use fenkra_core::sketch::{Curve, LinkShape};
                    match &l.shape {
                        LinkShape::Poly { points, .. } => es.link_pts.extend(points.iter().map(|p| (*p, data.pos(*p)))),
                        LinkShape::Line { a, b, .. } => es.link_pts.extend([*a, *b].map(|p| (p, data.pos(p)))),
                        LinkShape::Circle { center, curve } => {
                            es.link_pts.push((*center, data.pos(*center)));
                            if let Some(Curve::Circle { r, .. }) = data.curve(*curve).map(|c| &c.geom) {
                                es.link_r.push((*curve, *r));
                            }
                        }
                        LinkShape::Arc { center, s, e, .. } => es.link_pts.extend([*center, *s, *e].map(|p| (p, data.pos(p)))),
                        LinkShape::Ellipse { center, s, e, curve } => {
                            es.link_pts.extend([*center, *s, *e].map(|p| (p, data.pos(p))));
                            if let Some(Curve::Ellipse { major, minor, t0, t1, .. }) = data.curve(*curve).map(|c| &c.geom) {
                                es.link_e.push((*curve, *major, *minor, *t0, *t1));
                            }
                        }
                    }
                }
                let warn = (!es.warnings.is_empty()).then(|| es.warnings.join("; "));
                self.sketches.insert(f.id, (frame, Arc::new(es)));
                Ok(warn)
            }
            FeatureKind::Extrude(e) => {
                let (frame, sk) = &self.sketch(e.sketch)?;
                let (face, warn_open) = self.profile_face(frame, sk, e.thin.as_ref(), e.thin_side)?;
                let taper = ev(&e.taper, &self.vars, "ângulo de saída")?;
                // a saída só existe no término cego: com qualquer outro, o ângulo seria descartado
                if taper.abs() > 1e-9 && !matches!(e.extent, Extent::Blind(_)) {
                    return Err(format!("o ângulo de saída vale só com término 'cego' (este comando usa {}) — zere o ângulo ou mude o término", extent_name(&e.extent)));
                }
                // recorte entra no material (contra a normal); "inverter" troca o sentido
                let into = (e.op == BoolOp::Cut) != e.reverse;
                let n = match &e.dir {
                    // direção escolhida (linha/aresta/eixo): o recorte entra no sentido dela
                    Some(a) => {
                        let v = geo3d::unit(self.axis(a)?.1).ok_or("direção da extrusão nula")?;
                        // K-10: direção dentro do plano do sketch varre o perfil sobre ele mesmo (sólido sem
                        // espessura, inválido). O status ficava ok e os comandos seguintes herdavam o defeito
                        if geo3d::dot(v, frame.normal).abs() < 1e-6 {
                            return Err("a direção da extrusão está no plano do sketch: o perfil não ganharia espessura. Escolha uma direção que atravesse o plano ou desenhe o sketch em outro plano".into());
                        }
                        if e.op == BoolOp::Cut { v.map(|x| -x) } else { v }
                    }
                    None => frame.normal,
                };
                let dir = if into { n.map(|x| -x) } else { n };
                let _ = &warn_open;
                // ferramenta de cada lado (término, deslocamento e início deslocado: ver `termino.rs`);
                // papel de cada face só nas extrusões simples, nas demais fica desconhecido
                let _ = &sk.loops;
                let (tool, roles) = self.tool_extrusao(e, &face, frame, dir, loops_size(&sk.loops), taper, f.target)?;
                // corte que não alcança o corpo (sentido errado, ferramenta fora dele) não remove nada
                let vol0 = if e.op == BoolOp::Cut { self.body(f.target).map(|b| b.mass().volume) } else { None };
                self.apply_roles(f.target, tool, e.op, f.id, roles)?;
                if let Some(v0) = vol0 {
                    let v1 = self.body(f.target).map_or(0., |b| b.mass().volume);
                    if (v0 - v1).abs() <= 1e-9 * v0.abs().max(1.) {
                        // B392: confere o outro lado para dizer com certeza se 'inverter' resolve
                        let removeria = |t: &Shape| self.body(f.target).and_then(|b| b.boolean(t, Bool::Cut).ok()).is_some_and(|r| v0 - r.mass().volume > 1e-9 * v0.abs().max(1.));
                        let outro_lado = self.tool_extrusao(e, &face, frame, dir.map(|x| -x), loops_size(&sk.loops), taper, f.target).is_ok_and(|(t, _)| removeria(&t));
                        return Ok(Some(sentido_corte::aviso(outro_lado, e.reverse).into()));
                    }
                }
                Ok(warn_open)
            }
            FeatureKind::Revolve(r) => {
                let (frame, sk) = &self.sketch(r.sketch)?;
                let (origin, dir) = self.axis(&r.axis)?;
                let (face, _) = self.profile_face(frame, sk, r.thin.as_ref(), r.thin_side)?;
                check_revolve_profile(frame, sk, origin, dir)?;
                let ang = ev(&r.angle, &self.vars, "ângulo")?;
                // dois ângulos: o segundo vai para o outro lado do plano do sketch
                let (ang, start) = match &r.angle2 {
                    Some(a2) => {
                        let a2 = ev(a2, &self.vars, "ângulo 2")?;
                        if ang + a2 > 360. + 1e-9 {
                            return Err("os dois ângulos somam mais de 360°".into());
                        }
                        (ang + a2, -a2)
                    }
                    None if r.symmetric => (ang, -ang / 2.),
                    None => (ang, 0.),
                };
                let (tool, roles) = face.revolve_roles(origin, dir, ang, start).map_err(k)?;
                self.apply_roles(f.target, tool, r.op, f.id, Some(roles))?;
                Ok(None)
            }
            FeatureKind::Hole(h) => {
                let d = ev(&h.diameter, &self.vars, "diâmetro")?;
                // direção escolhida (linha, aresta, eixo) ou automática
                let custom = match &h.dir {
                    Some(a) => Some(geo3d::unit(self.axis_d(a, 0)?.1).ok_or("direção do furo nula")?),
                    None => None,
                };
                // cada furo: boca e direção entrando no material
                let mut seats: Vec<(V3, V3)> = vec![];
                if h.points.is_empty() {
                    let (frame, sk) = &self.sketch(h.sketch)?;
                    if sk.points.is_empty() {
                        return Err("o sketch do furo precisa de pelo menos um ponto (centro) — ou escolha pontos 3D".into());
                    }
                    let n = frame.normal.map(|x| -x);
                    for p in &sk.points {
                        seats.push((frame.point(p[0], p[1]), custom.unwrap_or(n)));
                    }
                } else {
                    let body = self.body(f.target).cloned();
                    for p in &h.points {
                        let o = self.point(p, 0)?;
                        match custom {
                            Some(v) => seats.push((o, v)),
                            None => {
                                // projetado na superfície mais perto, normal a ela, para dentro do material
                                let b = body.as_ref().ok_or("furo por pontos 3D precisa de um corpo para achar a superfície")?;
                                let (q, n) = b.surface_near(o).map_err(k)?;
                                seats.push((q, n.map(|x| -x)));
                            }
                        }
                    }
                }
                if h.reverse {
                    for s in &mut seats {
                        s.1 = s.1.map(|x| -x);
                    }
                }
                // furo inteligente: o centro fora da peça (sketch acima dela, ponto solto) desce pela
                // direção do furo até a superfície; dentro do material fica onde está
                if let Some(b) = self.body(f.target).cloned() {
                    let size = self.size_hint().max(1.);
                    for (i, (o, dir)) in seats.iter_mut().enumerate() {
                        let (q, n) = b.surface_near(*o).map_err(k)?;
                        let gap = geo3d::dot(geo3d::sub(*o, q), n);
                        if gap < 1e-6 * size {
                            continue;
                        }
                        match b.ray_hit(*o, *dir).map_err(k)? {
                            Some((hit, t)) if t > -1e-9 => *o = hit,
                            Some(_) => return Err(format!("o furo {} aponta para fora da peça — use 'inverter sentido'", i + 1)),
                            None => return Err(format!("o centro {} não encontra a peça na direção do furo (projete-o sobre a superfície ou mude a direção)", i + 1)),
                        }
                    }
                }
                let shape = match &h.kind {
                    HoleKind::Simples => HoleShape::Simple,
                    HoleKind::Rebaixado { diameter, depth } => HoleShape::Counterbore {
                        diameter: ev(diameter, &self.vars, "diâmetro do rebaixo")?,
                        depth: ev(depth, &self.vars, "profundidade do rebaixo")?,
                    },
                    HoleKind::Escareado { diameter, angle } => HoleShape::Countersink {
                        diameter: ev(diameter, &self.vars, "diâmetro do escareado")?,
                        angle: ev(angle, &self.vars, "ângulo do escareado")?,
                    },
                    HoleKind::Contrafurado { diameter, depth, angle } => HoleShape::Counterdrill {
                        diameter: ev(diameter, &self.vars, "diâmetro do contrafuro")?,
                        depth: ev(depth, &self.vars, "profundidade do contrafuro")?,
                        angle: ev(angle, &self.vars, "ângulo do contrafuro")?,
                    },
                };
                let tip = match &h.tip {
                    Some(t) => ev(t, &self.vars, "ângulo da ponta")?,
                    None => 0.,
                };
                let taper = ev(&h.taper, &self.vars, "conicidade")?;
                // B90: deslocamento da ponta, profundidade de rosca e folga no fundo
                let tip_off = self.mm_opcional(&h.extra.tip_offset, "deslocamento da ponta")?;
                let rosca_prof = if h.extra.thread_depth.trim().is_empty() { None } else { Some(ev(&h.extra.thread_depth, &self.vars, "profundidade da rosca")?) };
                let rosca_folga = self.mm_opcional(&h.extra.thread_relief, "folga no fundo da rosca")?;
                if rosca_prof.is_some_and(|p| p <= 0.) || rosca_folga < 0. {
                    return Err("a profundidade da rosca precisa ser positiva e a folga no fundo não pode ser negativa".into());
                }
                // B373: folgas em mm, passos ou porcentagem da profundidade útil, chanfro de entrada e ângulo de início
                let un_fundo = h.extra.thread_relief_unit.unwrap_or(UnidadeMedida::Passos);
                let topo_bruto = self.mm_opcional(&h.extra.thread_top_gap, "folga em cima da rosca")?;
                let chanfro_bruto = self.mm_opcional(&h.extra.thread_chamfer, "chanfro de entrada da rosca")?;
                let angulo_inicio = self.mm_opcional(&h.extra.thread_start_angle, "ângulo de início da rosca")?;
                if topo_bruto < 0. || chanfro_bruto < 0. {
                    return Err("a folga em cima e o chanfro de entrada da rosca não podem ser negativos".into());
                }
                let ref_escolhida = match &h.extra.thread_start_ref {
                    Some(a) => Some(geo3d::unit(self.axis_d(a, 0)?.1).ok_or("referência do ângulo de início nula")?),
                    None => None,
                };
                let em_mm = |valor: f64, un: UnidadeMedida, passo: f64, base: f64| match un {
                    UnidadeMedida::Mm => valor,
                    UnidadeMedida::Passos => valor * passo,
                    UnidadeMedida::Porcento => base * valor / 100.,
                };
                // folga no fundo em passos de rosca, para a profundidade útil `base` do furo
                let folga_passos = |base: f64, passo: f64| em_mm(rosca_folga, un_fundo, passo, base) / passo;
                let cut = self.end_cutter(&h.depth, f.target)?;
                let mut tool = Shape::empty();
                let mut depths = vec![];
                for (o, dir) in &seats {
                    // simétrico e dois lados: o furo vai dos dois lados da boca (antes só a primeira metade existia)
                    let mut back = 0.;
                    let depth = match (&cut, &h.depth) {
                        (Some((c, _)), _) => self.reach(c, *o, f.target),
                        (None, Extent::UpToLast) => self.last_dist(f.target, *o, *dir)?,
                        (None, x) => {
                            let (a, b) = self.extent(x, f.target)?;
                            back = b;
                            a
                        }
                    };
                    depths.push(depth);
                    HOLE_LOG.with(|l| {
                        if let Some(v) = l.borrow_mut().as_mut() {
                            let (forma, d_boca, prof_boca, angulo) = match shape {
                                HoleShape::Simple => (FormaFuro::Reto, 0., 0., 0.),
                                HoleShape::Counterbore { diameter, depth } => (FormaFuro::Rebaixo, diameter, depth, 0.),
                                HoleShape::Countersink { diameter, angle } => (FormaFuro::Escareado, diameter, prof_escareado(diameter, d, angle), angle),
                                HoleShape::Counterdrill { diameter, depth, angle } => (FormaFuro::Contrafuro, diameter, depth, angle),
                            };
                            let tolerancia = h.standard.as_deref().map(tolerancia_do_texto).unwrap_or_default();
                            let prof_rosca = h.thread.as_deref().and_then(hardware::thread).map_or(0., |sp| furo_norma::comprimento_de_rosca(depth, rosca_prof, folga_passos(depth, sp.pitch), sp.pitch).max(0.));
                            v.push(HoleRow { feature: f.id, prof_rosca, pos: *o, dir: *dir, diameter: d, depth, through: matches!(h.depth, Extent::ThroughAll | Extent::UpToLast), thread: h.thread.clone().unwrap_or_default(), forma, d_boca, prof_boca, angulo, tolerancia });
                        }
                    });
                    let depth_broca = furo_norma::profundidade_com_ponta(depth, tip_off);
                    if depth_broca <= 1e-9 {
                        return Err("o deslocamento da ponta anula a profundidade do furo".into());
                    }
                    let mut t = Shape::hole_tool_ext(*o, *dir, d, depth_broca, shape, tip, taper).map_err(k)?;
                    if let Some((c, ext)) = &cut {
                        t = t.clip_until(c, *o, *ext).map_err(k)?;
                    }
                    tool = if tool.is_null() { t } else { tool.compound(&t) };
                    if back > 0. {
                        let rev = dir.map(|x| -x);
                        let t2 = Shape::hole_tool_ext(*o, rev, d, back, HoleShape::Simple, 0., taper).map_err(k)?;
                        tool = tool.compound(&t2);
                    }
                }
                self.apply(f.target, tool, BoolOp::Cut, f.id)?;
                let Some(t) = &h.thread else { return Ok(None) };
                // B351: onde a rosca fica em cada furo. Cosmética e prévia leve só registram a posição
                // (linhas e textura); o sulco helicoidal só é construído ao concluir
                let Some(spec) = hardware::thread(t) else {
                    if h.thread_modeled {
                        return Err(format!("rosca '{t}' não encontrada na tabela de normas"));
                    }
                    return Ok(Some(format!("rosca cosmética {t}{}", if h.left_hand { " (esquerda)" } else { "" })));
                };
                let sulco = h.thread_modeled && self.leve != Some(f.id);
                let (lo, hi) = self.body(f.target).and_then(|b| b.bbox()).ok_or("corpo sem dimensões")?;
                let passante = !matches!(h.depth, Extent::Blind(_) | Extent::Symmetric(_) | Extent::TwoSides(..));
                let mut avisos: Vec<String> = vec![];
                if (d - spec.tap_drill).abs() > 0.3 * spec.pitch {
                    avisos.push(format!("Ø do furo {d:.2} ≠ {:.2} da broca de rosca {}", spec.tap_drill, spec.name));
                }
                let mut sem_espaco = 0;
                let mut sem_sulco = 0;
                for ((o, dir), depth) in seats.iter().zip(depths) {
                    let (o, dir) = (*o, *dir);
                    // extensão do material a partir do ponto, nos dois sentidos do eixo do furo:
                    // a rosca vai para o lado onde está a peça (o sketch pode estar na outra face)
                    let (mut fwd, mut back): (f64, f64) = (0., 0.);
                    for c in 0..8 {
                        let q = [if c & 1 == 0 { lo[0] } else { hi[0] }, if c & 2 == 0 { lo[1] } else { hi[1] }, if c & 4 == 0 { lo[2] } else { hi[2] }];
                        let t = dot(sub(q, o), dir);
                        fwd = fwd.max(t);
                        back = back.max(-t);
                    }
                    let (tdir, ext) = if fwd >= back { (dir, fwd) } else { (dir.map(|x| -x), back) };
                    // a rosca começa abaixo do rebaixo/escareado (começar no vazio faz a booleana falhar)
                    let cabeca = if fwd >= back { shape.head_depth(d) } else { 0. };
                    let lim = depth.min(ext);
                    if sulco && rosca_prof.is_some_and(|p| p > lim + 1e-9) {
                        return Err(format!("a profundidade da rosca ({}) passa da profundidade do furo ({})", rosca_prof.unwrap_or(0.), lim));
                    }
                    // B373: chanfro de entrada na boca (só furo sem rebaixo ou escareado: nele a entrada já é a boca larga)
                    let chanfro = em_mm(chanfro_bruto, h.extra.thread_chamfer_unit, spec.pitch, rosca_furo::profundidade_do_filete(spec.pitch));
                    if chanfro > 1e-9 && cabeca <= 1e-9 {
                        let cone = rosca_furo::chanfro_de_entrada(o, tdir, d, chanfro)?;
                        self.apply(f.target, cone, BoolOp::Cut, f.id)?;
                    }
                    let topo = em_mm(topo_bruto, h.extra.thread_top_gap_unit, spec.pitch, lim);
                    let folga = folga_passos(lim, spec.pitch);
                    let plano = rosca_furo::plano_com_topo(spec, cabeca, lim, ext, rosca_prof, folga, passante, topo);
                    // cosmética: a parede cilíndrica inteira que a norma manda roscar, colada no escareado;
                    // modelada (e a prévia dela): o trecho onde o sulco de fato fica
                    let (inicio, visivel) = if h.thread_modeled {
                        match plano {
                            Some(p) => (p.inicio, p.comprimento.min((lim - p.inicio).max(0.))),
                            None => (0., 0.),
                        }
                    } else {
                        (cabeca, furo_norma::comprimento_de_rosca(lim, rosca_prof, folga, spec.pitch) - cabeca)
                    };
                    if visivel < spec.pitch {
                        sem_espaco += 1;
                        continue;
                    }
                    let o = [0, 1, 2].map(|i| o[i] + tdir[i] * inicio);
                    self.roscas.push(RoscaViz { feature: f.id, origin: o, dir: tdir, radius: d / 2., len: visivel, pitch: spec.pitch, esquerda: h.left_hand, interna: true, modelada: sulco });
                    let Some(plano) = plano.filter(|_| sulco) else { continue };
                    // `o` já é o início do sulco; a boca do furo fica `plano.inicio` acima dele
                    let boca = [0, 1, 2].map(|i| o[i] - tdir[i] * plano.inicio);
                    let fase = rosca_furo::Fase { referencia: rosca_furo::referencia(tdir, ref_escolhida), angulo: angulo_inicio, esquerda: h.left_hand };
                    let ferramenta = |degrau: usize| -> Result<Shape, String> {
                        let mut tool = rosca_furo::ferramenta_do_furo(boca, tdir, spec, &plano, degrau, fase)?;
                        if let Some((c, ext)) = &cut {
                            // a rosca para na mesma superfície do furo
                            tool = tool.clip_until(c, boca, *ext).map_err(k)?;
                        }
                        Ok(tool)
                    };
                    // a booleana pode falhar ou devolver o corpo sem o sulco, e sem erro: confere e tenta de novo
                    let minimo = 0.35 * rosca_furo::volume_estimado(spec, d / 2., visivel);
                    if !self.corta_rosca(f.target, f.id, minimo, &ferramenta) {
                        sem_sulco += 1;
                    }
                }
                if sem_sulco > 0 {
                    avisos.push(format!("a rosca {} não removeu material em {sem_sulco} furo(s): confira o diâmetro do furo e a profundidade", spec.name));
                }
                if sem_espaco > 0 {
                    avisos.push(format!("a rosca {} não cabe em {sem_espaco} furo(s): menos de um passo depois do escareado e da folga", spec.name));
                }
                if !sulco && self.leve != Some(f.id) {
                    avisos.insert(0, format!("rosca cosmética {t}{}", if h.left_hand { " (esquerda)" } else { "" }));
                }
                Ok((!avisos.is_empty()).then(|| avisos.join("; ")))
            }
            FeatureKind::Sweep(w) => {
                let (frame, sk) = &self.sketch(w.sketch)?;
                if sk.loops.is_empty() {
                    return Err("o sketch do perfil não tem contorno fechado".into());
                }
                let mut face = Shape::face(frame, &sk.loops).map_err(k)?;
                if let Some(t) = &w.thin {
                    face = face.thin(ev(t, &self.vars, "espessura da parede")?).map_err(k)?;
                }
                let spine = self.curve(&w.spine, 0)?;
                let mode = match w.mode {
                    SweepMode::Corrigido => 0,
                    SweepMode::Fixo => 1,
                    SweepMode::Frenet => 2,
                };
                let tool = face.sweep(&spine.pts, mode).map_err(k)?;
                self.apply(f.target, tool, w.op, f.id)?;
                let mut avisos: Vec<String> = vec![];
                if !sk.open.is_empty() {
                    avisos.push(format!("{} perfil(is) aberto(s) do sketch foram descartados: só contornos fechados varrem", sk.open.len()));
                }
                if guia_com_cantos(&spine.pts) {
                    avisos.push(AVISO_GUIA_COM_CANTOS.into());
                }
                // B301: o perfil deve estar normal à curva no início; o ângulo fora disso entra no aviso
                if let Some(desvio) = varredura_desvio(&frame.normal, &spine.pts) {
                    if desvio > 3. {
                        avisos.push(format!("o perfil não está normal à curva guia (desvio de {desvio:.0}° no início): crie um plano normal à curva e desenhe o perfil nele"));
                    }
                }
                Ok((!avisos.is_empty()).then(|| avisos.join("; ")))
            }
            FeatureKind::Loft(l) => {
                let mut dropped: Vec<String> = vec![];
                // (referencial, contorno) de cada seção, já com a torção aplicada
                let mut secoes: Vec<(Frame, Vec<fenkra_kernel::Seg2>)> = vec![];
                for (i, id) in l.sections.iter().enumerate() {
                    let (frame, sk) = &self.sketch(*id)?;
                    if sk.loops.is_empty() {
                        return Err(format!("o sketch '{}' não tem contorno fechado", self.doc.name_of(*id)));
                    }
                    if sk.loops.len() > 1 {
                        dropped.push(format!("a seção '{}' tem {} contornos e só o primeiro entra", self.doc.name_of(*id), sk.loops.len()));
                    }
                    let mut lacos = sk.loops[0].clone();
                    // a alça da linha de conexão: o ponto de partida anda pelos vértices do contorno
                    let t = l.torcao.get(i).copied().unwrap_or(0);
                    if t != 0 && lacos.len() > 1 {
                        let passo = t.rem_euclid(lacos.len() as i32) as usize;
                        lacos.rotate_left(passo);
                    }
                    secoes.push((*frame, lacos));
                }
                let condicionado = !l.inicio.e_livre() || !l.fim.e_livre();
                if condicionado && (l.ruled || l.closed) {
                    return Err("condição de ponta só vale em loft suave e aberto: desligue 'reto' e 'fechado' ou use a condição livre".into());
                }
                let mut faces: Vec<Shape> = vec![];
                let n = secoes.len();
                for (i, (frame, lacos)) in secoes.iter().enumerate() {
                    let face = Shape::face(frame, std::slice::from_ref(lacos)).map_err(k)?;
                    // condição Normal: uma cópia da própria seção, afastada na normal, faz o loft
                    // sair perpendicular; a distância é a intensidade vezes um terço do vão
                    let (ponta, vizinha) = if i == 0 { (&l.inicio, 1) } else if i + 1 == n { (&l.fim, n.saturating_sub(2)) } else { (&PontaLoft::default(), i) };
                    let normal_pedida = n >= 2 && (i == 0 || i + 1 == n) && ponta.condicao == CondicaoPonta::Normal;
                    let extra = if normal_pedida {
                        let f = ev(if ponta.intensidade.trim().is_empty() { "1" } else { &ponta.intensidade }, &self.vars, "intensidade da ponta")?;
                        if !(0. ..=3.).contains(&f) {
                            return Err(format!("a intensidade da ponta precisa estar entre 0 e 3 (veio {f})"));
                        }
                        let outra = &secoes[vizinha].0;
                        let vao = geo3d::dist(frame.origin, outra.origin);
                        let sinal = if geo3d::dot(frame.normal, geo3d::sub(outra.origin, frame.origin)) >= 0. { 1. } else { -1. };
                        let d = f * vao / 3.;
                        (d > 1e-9).then(|| face.transformed(Xform::Translate(geo3d::mul(frame.normal, sinal * d)))).transpose().map_err(k)?
                    } else {
                        None
                    };
                    if i == 0 {
                        faces.push(face);
                        faces.extend(extra);
                    } else if i + 1 == n {
                        faces.extend(extra);
                        faces.push(face);
                    } else {
                        faces.push(face);
                    }
                }
                let mut comp = Shape::empty();
                for face in faces {
                    comp = if comp.is_null() { face } else { comp.compound(&face) };
                }
                let tool = comp.loft(l.ruled, l.closed, !l.torcao.iter().any(|t| *t != 0)).map_err(k)?;
                self.apply(f.target, tool, l.op, f.id)?;
                Ok((!dropped.is_empty()).then(|| format!("contornos descartados: {}", dropped.join("; "))))
            }
            FeatureKind::Primitive(pr) => {
                let fr = self.plane(&pr.plane, 0)?;
                let names = pr.kind.params();
                let mut vals = vec![];
                for (e, n) in [&pr.a, &pr.b, &pr.c, &pr.d].into_iter().zip(names) {
                    vals.push(if n.is_empty() { 0. } else { ev(e, &self.vars, n)? });
                }
                let kind = PrimKind::ALL.iter().position(|x| *x == pr.kind).unwrap_or(0) as u8;
                let tool = Shape::primitive(kind, &fr, &vals).map_err(k)?;
                self.apply(f.target, tool, pr.op, f.id)?;
                Ok(None)
            }
            FeatureKind::Shell(sh) if sh.faces.is_empty() && !sh.closed => Ok(Some("escolha na viewport as faces a remover — até lá a casca não é aplicada".into())),
            FeatureKind::Draft(d) if d.faces.is_empty() => Ok(Some("escolha na viewport as faces que recebem a saída — até lá nada é aplicado".into())),
            FeatureKind::Thickness(t) if t.faces.is_empty() => Ok(Some("escolha na viewport as faces a engrossar — até lá nada é aplicado".into())),
            FeatureKind::DirectEdit(d) if d.faces.is_empty() => Ok(Some("escolha na viewport as faces a editar — até lá nada é aplicado".into())),
            FeatureKind::DirectEdit(d) => self.edicao_direta(f, d),
            FeatureKind::Shell(sh) => {
                let t = ev(&sh.thickness, &self.vars, "espessura")?;
                let (b, owners, roles) = self.body_owned(f.target).ok_or("não há corpo para esvaziar")?;
                // a parede para dentro não cabe se for tão grossa quanto a menor dimensão do corpo
                if !sh.outward {
                    if let Some((lo, hi)) = b.bbox() {
                        let menor = (0..3).map(|i| hi[i] - lo[i]).fold(f64::INFINITY, f64::min);
                        if t >= menor - 1e-9 {
                            return Err(format!("espessura {t} grande demais para a peça: a menor dimensão do corpo é {menor}"));
                        }
                    }
                }
                let pts = self.resolve_faces("casca", f, b, owners, roles, &sh.faces, &sh.face_keys)?;
                let (s, out) = if sh.espessuras.is_empty() {
                    b.shell_tagged(&pts, t, sh.outward, &tags_for(owners)).map_err(k)?
                } else {
                    if sh.closed {
                        return Err("espessura por face precisa de uma face aberta".into());
                    }
                    let proprias = sh.espessuras.iter().map(|e| Ok((e.ponto, ev(&e.valor, &self.vars, "espessura da face")?))).collect::<Result<Vec<_>, String>>()?;
                    b.shell_faces_tagged(&pts, t, sh.outward, &proprias, &tags_for(owners)).map_err(k)?
                };
                // o kernel pode devolver o corpo maciço sem erro: casca que não tirou material falhou
                if !sh.outward && s.mass().volume >= b.mass().volume - 1e-9 * b.mass().volume.abs().max(1.) {
                    return Err("a casca não esvaziou o corpo: espessura grande demais para a peça".into());
                }
                let body = remap_body(s, owners, roles, &out, f.id);
                self.bodies.insert(f.target, body);
                Ok(None)
            }
            FeatureKind::Draft(d) => {
                let a = ev(&d.angle, &self.vars, "ângulo")?;
                let neutral = self.plane(&d.neutral, 0)?;
                let pull = if d.reverse { neutral.normal.map(|x| -x) } else { neutral.normal };
                let (b, owners, roles) = self.body_owned(f.target).ok_or("não há corpo para aplicar")?;
                let pts = self.resolve_faces("ângulo de saída", f, b, owners, roles, &d.faces, &d.face_keys)?;
                let (s, out) = b.draft_tagged(&pts, pull, a, &neutral, &tags_for(owners)).map_err(k)?;
                let body = remap_body(s, owners, roles, &out, f.id);
                self.bodies.insert(f.target, body);
                Ok(None)
            }
            FeatureKind::Thickness(t) => {
                // B07: o botão "remover" vale mais que o sinal; sinal negativo continua removendo (arquivos antigos)
                let v = ev(&t.value, &self.vars, "espessura")?;
                let v = if t.remove { -v.abs() } else { v };
                if v.abs() < 1e-9 {
                    return Ok(Some("espessura zero: nada a fazer".into()));
                }
                let (b, owners, roles) = self.body_owned(f.target).ok_or("não há corpo para aplicar")?;
                let pts = self.resolve_faces("espessura", f, b, owners, roles, &t.faces, &t.face_keys)?;
                let tool = b.faces_offset_tool(&pts, v).map_err(k)?;
                self.apply(f.target, tool, if v > 0. { BoolOp::Add } else { BoolOp::Cut }, f.id)?;
                Ok(None)
            }
            FeatureKind::Pattern(pt) => {
                // B91: posição (i, j) de cada instância, para pular por posição e não por ordem
                let (xs, keys) = self.instancias(&pt.kind)?;
                if xs.is_empty() {
                    return Ok(Some("padrão com uma só instância".into()));
                }
                // B91: instâncias puladas saem da lista; posição que não existe mais é avisada
                let (xs, aviso) = padrao::sem_puladas(xs, &keys, &pt.skip);
                if xs.is_empty() {
                    return Ok(Some("todas as instâncias do padrão estão puladas".into()));
                }
                self.replicate(f, &pt.sources, &xs)?;
                Ok(aviso)
            }
            FeatureKind::Mirror(m) => {
                let pl = self.plane(&m.plane, 0)?;
                self.replicate(f, &m.sources, &[Xform::Mirror(pl.origin, pl.normal)])?;
                Ok(None)
            }
            FeatureKind::SmWall(w) => {
                let sp = self.sheet()?;
                // parede de superfície: a superfície engrossada com a espessura (lado pelas setas)
                if let Some(sf) = &w.surface {
                    let sh = self.cut_shape(sf, 0)?;
                    let (t1, t2) = if w.mid { (sp.t / 2., sp.t / 2.) } else if w.reverse { (0., sp.t) } else { (sp.t, 0.) };
                    let tool = sh.thicken(t1, t2).map_err(k)?;
                    self.apply(f.target, tool, BoolOp::Add, f.id)?;
                    return Ok(None);
                }
                let (frame, sk) = &self.sketch(w.sketch)?;
                if sk.loops.is_empty() {
                    return Err("a parede base precisa de um perfil fechado".into());
                }
                let mut face = Shape::face(frame, &sk.loops).map_err(k)?;
                let dir = if w.reverse { frame.normal.map(|x| -x) } else { frame.normal };
                let off = if w.offset.trim().is_empty() { 0. } else { ev(&w.offset, &self.vars, "afastamento")? };
                if off.abs() > 1e-12 {
                    face = face.transformed(Xform::Translate(geo3d::mul(dir, off))).map_err(k)?;
                }
                // com papel em cada face (tampas e laterais), as arestas da parede ganham chave v2
                let (tool, roles) = if w.mid { face.extrude_roles(dir, sp.t / 2., sp.t / 2.) } else { face.extrude_roles(dir, sp.t, 0.) }.map_err(k)?;
                self.apply_roles(f.target, tool, BoolOp::Add, f.id, Some(roles))?;
                Ok(None)
            }
            FeatureKind::SmFlange(fl) => self.run_flange(f, fl, &ExtensaoAba::Inteira, false),
            FeatureKind::SmFlangeLoop(lp) => self.run_flange_loop(f, lp),
            FeatureKind::SmCornerClose(cc) => self.run_corner_close(f, cc),
            FeatureKind::SmLoftFlange(lf) => self.run_loft_flange(f, lf),
            FeatureKind::SmUnfoldPartial(u) => self.run_unfold_partial(f, u),
            FeatureKind::SmConvert(c) => self.run_convert(f, c),
            FeatureKind::SmJoggle(j) => {
                let sp = self.sheet()?;
                let depth = ev(&j.depth, &self.vars, "desvio")?;
                let mut run = ev(&j.run, &self.vars, "rampa")?;
                if let Some(a) = &j.angle {
                    let a = ev(a, &self.vars, "ângulo da rampa")?;
                    if !(1.0..=89.0).contains(&a) {
                        return Err("ângulo da rampa entre 1° e 89°".into());
                    }
                    run = depth.abs() / a.to_radians().tan();
                }
                let b = self.body(f.target).ok_or("o joggle precisa de uma parede de chapa")?.clone();
                // normal da face no meio da linha (as pontas podem cair numa aresta da chapa)
                let (_, n) = b.surface_near(geo3d::lerp(j.a, j.b, 0.5)).map_err(k)?;
                // linha pela interseção de um plano com a face
                let (ja, jb) = match &j.plane {
                    Some(pl) => {
                        let fr = self.plane(pl, 0)?;
                        let e = geo3d::unit(fenkra_kernel::cross(n, fr.normal)).ok_or("o plano é paralelo à chapa")?;
                        let w = fenkra_kernel::cross(e, n);
                        let den = geo3d::dot(w, fr.normal);
                        if den.abs() < 1e-9 {
                            return Err("o plano não corta a face".into());
                        }
                        let s0 = geo3d::dot(geo3d::sub(fr.origin, j.a), fr.normal) / den;
                        let a = geo3d::add(j.a, geo3d::mul(w, s0));
                        (a, geo3d::add(a, geo3d::mul(e, 10.)))
                    }
                    None => (j.a, j.b),
                };
                let j = &SmJoggle { a: ja, b: jb, ..j.clone() };
                let e = geo3d::unit(geo3d::sub(j.b, j.a)).ok_or("os dois pontos da linha coincidem")?;
                let mut side = geo3d::unit(fenkra_kernel::cross(n, e)).ok_or("linha normal à chapa")?;
                if j.flip {
                    side = geo3d::mul(side, -1.);
                }
                let (lo, hi) = b.bbox().ok_or("corpo sem dimensões")?;
                let big = geo3d::dist(lo, hi) * 2. + 10.;
                // parte da chapa além da linha (lado `side`)
                let start = geo3d::sub(j.a, geo3d::mul(e, big));
                let z = geo3d::mul(n, -1.);
                let y0 = fenkra_kernel::cross(z, e);
                let corner = |from: f64| {
                    // caixa ao longo de e (2·big), de `from` a `big` no lado, atravessando ±big na normal
                    let o = geo3d::add(geo3d::add(start, geo3d::mul(side, from)), geo3d::mul(n, big));
                    if geo3d::dot(y0, side) > 0. {
                        Shape::box_tool(o, e, z, [2. * big, big, 2. * big])
                    } else {
                        Shape::box_tool(geo3d::add(o, geo3d::mul(side, big)), e, z, [2. * big, big, 2. * big])
                    }
                };
                let beyond = corner(0.).map_err(k)?;
                let far = corner(run).map_err(k)?;
                let part = b.boolean(&beyond, fenkra_kernel::Bool::Common).map_err(k)?;
                let far_part = part.boolean(&far, fenkra_kernel::Bool::Common).map_err(k)?;
                // compensação: a rampa gasta material; a parte deslocada volta o que sobra
                let extra = if j.keep_length { (run * run + depth * depth).sqrt() - run } else { 0. };
                let moved = far_part.transformed(Xform::Translate(geo3d::add(geo3d::mul(n, -depth), geo3d::mul(side, -extra)))).map_err(k)?;
                // rampa: paralelogramo (seção) na linha, só na largura da chapa ao longo da linha
                let (plo, phi) = part.bbox().ok_or("a linha do joggle não corta a chapa")?;
                let (mut e0, mut e1) = (f64::MAX, f64::MIN);
                for c in 0..8 {
                    let q = [if c & 1 == 0 { plo[0] } else { phi[0] }, if c & 2 == 0 { plo[1] } else { phi[1] }, if c & 4 == 0 { plo[2] } else { phi[2] }];
                    let t = geo3d::dot(geo3d::sub(q, j.a), e);
                    e0 = e0.min(t);
                    e1 = e1.max(t);
                }
                let top = geo3d::add(j.a, geo3d::mul(e, e0));
                // mapa de dobras: as duas dobras do joggle, de canto vivo, nas duas pontas da rampa
                let rampa = run.hypot(depth);
                let estica = if j.keep_length { 0. } else { rampa - run };
                let giro = depth.abs().atan2(run).to_degrees();
                let cmp = chapa::compensacao(sp.t, 0., giro, sp.k_for(0.));
                let fim_rampa = geo3d::mul(e, e1 - e0);
                for (du, sobe) in [(0., depth < 0.), (run + estica, depth > 0.)] {
                    let p0 = geo3d::add(top, geo3d::mul(side, du));
                    let l = [p0, geo3d::add(p0, fim_rampa)];
                    self.marca_cadeia(f.id, &l, &[side, side], giro, 0., sobe, &cmp, sp.flat);
                }
                if sp.flat {
                    // planificada: a chapa fica plana; sem manter o comprimento, a rampa acrescenta material
                    let rest = b.boolean(&beyond, fenkra_kernel::Bool::Cut).map_err(k)?;
                    let longe = far_part.transformed(Xform::Translate(geo3d::mul(side, estica))).map_err(k)?;
                    let fundo = geo3d::mul(n, -sp.t);
                    let faixa = [top, geo3d::add(top, fundo), geo3d::add(geo3d::add(top, fundo), geo3d::mul(side, run + estica)), geo3d::add(top, geo3d::mul(side, run + estica))];
                    let ponte = Shape::poly_prism(&faixa, fim_rampa).map_err(k)?;
                    let plana = rest.boolean(&longe, fenkra_kernel::Bool::Fuse).map_err(k)?.boolean(&ponte, fenkra_kernel::Bool::Fuse).map_err(k)?;
                    self.bodies.insert(f.target, fresh_body(plana, f.id));
                    return Ok(Some("planificada: joggle desfeito".into()));
                }
                let pts = [
                    top,
                    geo3d::sub(top, geo3d::mul(n, sp.t)),
                    geo3d::add(geo3d::sub(top, geo3d::mul(n, sp.t + depth)), geo3d::mul(side, run)),
                    geo3d::add(geo3d::sub(top, geo3d::mul(n, depth)), geo3d::mul(side, run)),
                ];
                let ramp = Shape::poly_prism(&pts, geo3d::mul(e, e1 - e0)).map_err(k)?;
                let n_own = b.face_count() as usize;
                let rest = b.boolean(&beyond, fenkra_kernel::Bool::Cut).map_err(k)?;
                let r = rest.boolean(&moved, fenkra_kernel::Bool::Fuse).map_err(k)?.boolean(&ramp, fenkra_kernel::Bool::Fuse).map_err(k)?;
                let _ = n_own;
                self.bodies.insert(f.target, fresh_body(r, f.id));
                Ok(None)
            }
            FeatureKind::SmBend(bd) => {
                let sp = self.sheet()?;
                let ang = ev(&bd.angle, &self.vars, "ângulo")?;
                let r = match &bd.radius {
                    Some(x) => ev(x, &self.vars, "raio")?,
                    None => sp.r,
                };
                if !(0.1..=179.9).contains(&ang) {
                    return Err("ângulo da dobra entre 0,1° e 179,9°".into());
                }
                let b0 = self.body(f.target).ok_or("a dobra precisa de uma chapa")?.clone();
                // linhas: as do sketch (retas; curva suave vira a corda dentro da tolerância) ou a–b
                let mut notes: Vec<String> = vec![];
                let raw: Vec<(V3, V3)> = match bd.sketch {
                    Some(sid) => {
                        let (frame, sk) = &self.sketch(sid)?;
                        // curva forte: parede varrida ao longo da linha, não giro em torno de reta
                        if let Some(c) = chapa::cadeia_curva(&sk.open) {
                            let pts: Vec<V3> = c.iter().map(|p| self.map_point(frame.point(p[0], p[1]), f.flat_space, sp.flat)).collect();
                            return self.dobra_curva(f, pts, ang, r, bd);
                        }
                        let (ls, nt) = chapa::linhas_dobra(&sk.open)?;
                        notes.extend(nt);
                        ls.iter().map(|(p, q)| (frame.point(p[0], p[1]), frame.point(q[0], q[1]))).collect()
                    }
                    None => vec![(bd.a, bd.b)],
                };
                // o que foi escolhido no estado em que o comando nasceu vai para o estado atual
                let raw: Vec<(V3, V3)> = raw.iter().map(|(p, q)| (self.map_point(*p, f.flat_space, sp.flat), self.map_point(*q, f.flat_space, sp.flat))).collect();
                let e0 = geo3d::unit(geo3d::sub(raw[0].1, raw[0].0)).ok_or("os dois pontos da linha coincidem")?;
                let (_, n_ref) = b0.surface_near(geo3d::lerp(raw[0].0, raw[0].1, 0.5)).map_err(k)?;
                let mut side_ref = geo3d::unit(fenkra_kernel::cross(n_ref, e0)).ok_or("linha normal à chapa")?;
                if bd.flip {
                    side_ref = geo3d::mul(side_ref, -1.);
                }
                // todas na mesma direção e paralelas; ordem do lado fixo para o lado que gira
                let mut lines: Vec<(V3, V3)> = vec![];
                for (p, q) in &raw {
                    let e = geo3d::unit(geo3d::sub(*q, *p)).ok_or("os dois pontos da linha coincidem")?;
                    if geo3d::norm(fenkra_kernel::cross(e, e0)) > 0.01 {
                        return Err("as linhas de uma dobra precisam ser paralelas: faça um comando para cada direção".into());
                    }
                    lines.push(if geo3d::dot(e, e0) < 0. { (*q, *p) } else { (*p, *q) });
                }
                lines.sort_by(|x, y| geo3d::dot(geo3d::lerp(x.0, x.1, 0.5), side_ref).total_cmp(&geo3d::dot(geo3d::lerp(y.0, y.1, 0.5), side_ref)));
                let kloc = sp.k_for(r);
                let cmp = chapa::compensacao(sp.t, r, ang, kloc);
                // a linha marca o centro, o começo ou o fim da zona, ou o vértice de fora
                let delta = chapa_linha::deslocamento_posicao(bd.position, &cmp);
                for w in lines.windows(2) {
                    let gap = geo3d::dot(geo3d::sub(geo3d::lerp(w[1].0, w[1].1, 0.5), geo3d::lerp(w[0].0, w[0].1, 0.5)), side_ref);
                    if gap < cmp.ba - 1e-6 {
                        return Err(format!("as linhas de dobra estão a {gap:.2} mm uma da outra, menos que a zona dobrada ({:.2} mm): afaste-as ou reduza ângulo e raio", cmp.ba));
                    }
                }
                let mut cur = b0;
                let mut centers: Vec<V3> = vec![];
                let mut own: Vec<SheetMap> = vec![];
                for (p, q) in lines.iter() {
                    let (mut p, mut q) = (*p, *q);
                    let c_orig = geo3d::lerp(p, q, 0.5);
                    // com as dobras anteriores já feitas, a linha que fica além delas acompanha o giro
                    if !sp.flat {
                        for (m, c0) in own.iter().zip(&centers) {
                            if geo3d::dot(geo3d::sub(c_orig, *c0), side_ref) > 0. {
                                p = m.to_folded(p);
                                q = m.to_folded(q);
                            }
                        }
                    }
                    let (shape, mut map) = self.bend_line(&cur, p, q, ang, r, kloc, sp.t, bd.flip, bd.down, delta, !sp.flat)?;
                    centers.push(c_orig);
                    if let Some(sh) = shape {
                        cur = sh;
                    }
                    own.push(map);
                    if sp.flat {
                        // planificada: o mapa vai para o espaço dobrado, pelas dobras já registradas
                        let back = |q: V3| self.map_point(q, true, false);
                        let c0 = back(map.c0);
                        let c1 = back(geo3d::add(map.c0, geo3d::mul(map.ev, map.l)));
                        let tip = |v: V3| geo3d::sub(back(geo3d::add(map.c0, v)), c0);
                        map.n = geo3d::unit(tip(map.n)).unwrap_or(map.n);
                        map.u = geo3d::unit(tip(map.u)).unwrap_or(map.u);
                        map.c0 = c0;
                        map.ev = geo3d::unit(geo3d::sub(c1, c0)).unwrap_or(map.ev);
                        map.l = geo3d::dist(c0, c1);
                        map.o = geo3d::add(c0, geo3d::mul(map.n, r));
                        map.axis = fenkra_kernel::cross(map.u, map.n);
                    }
                    self.sheet_maps.push(map);
                }
                if sp.flat {
                    notes.insert(0, "planificada: dobra desfeita".into());
                } else {
                    self.bodies.insert(f.target, fresh_body(cur, f.id));
                }
                Ok((!notes.is_empty()).then(|| notes.join("; ")))
            }
            FeatureKind::SmLouver(lv) => {
                let sp = self.sheet()?;
                if sp.flat {
                    return self.veneziana_planificada(f, lv);
                }
                let w = ev(&lv.width, &self.vars, "largura")?;
                let h = ev(&lv.height, &self.vars, "altura")?;
                if h <= sp.t {
                    return Err("a altura da veneziana precisa ser maior que a espessura".into());
                }
                let b = self.body(f.target).ok_or("a veneziana precisa de uma chapa")?.clone();
                let mid = geo3d::lerp(lv.a, lv.b, 0.5);
                let (_, n0) = b.surface_near(mid).map_err(k)?;
                let e = geo3d::sub(lv.b, lv.a);
                let len = geo3d::norm(e);
                let e = geo3d::unit(e).ok_or("os dois pontos da linha coincidem")?;
                let mut side = geo3d::unit(fenkra_kernel::cross(n0, e)).ok_or("linha normal à chapa")?;
                if lv.flip {
                    side = geo3d::mul(side, -1.);
                }
                // levantar para fora da face clicada (ou para o outro lado: a origem vai para a outra face)
                let (o, n) = if lv.down { (geo3d::sub(lv.a, geo3d::mul(n0, sp.t)), geo3d::mul(n0, -1.)) } else { (lv.a, n0) };
                let at = |s: f64, z: f64| geo3d::add(geo3d::add(o, geo3d::mul(side, s)), geo3d::mul(n, z));
                // retira a chapa sob a rampa e põe a rampa (paralelogramo: o volume se conserva)
                let cut = Shape::poly_prism(&[at(0., 0.01), at(w, 0.01), at(w, -sp.t - 0.01), at(0., -sp.t - 0.01)], geo3d::mul(e, len)).map_err(k)?;
                let hood = Shape::poly_prism(&[at(0., h), at(w, 0.), at(w, -sp.t), at(0., h - sp.t)], geo3d::mul(e, len)).map_err(k)?;
                let out = b.boolean(&cut, fenkra_kernel::Bool::Cut).map_err(k)?.boolean(&hood, fenkra_kernel::Bool::Fuse).map_err(k)?;
                self.bodies.insert(f.target, fresh_body(out, f.id));
                Ok(None)
            }
            FeatureKind::SmStamp(st) => {
                let sp = self.sheet()?;
                let depth = ev(&st.depth, &self.vars, "profundidade")?;
                let width = if st.width.trim().is_empty() { 0. } else { ev(&st.width, &self.vars, "largura")? };
                // punção: o corpo do punção forma a chapa (casca com a espessura, aberta na base)
                if st.kind == 4 {
                    let pid = st.punch.ok_or("escolha o coletor com o corpo do punção")?;
                    let punch = self.body(pid).ok_or("o coletor do punção não tem corpo (ou vem depois)")?.clone();
                    let b = self.body(f.target).ok_or("o estampo precisa de uma chapa")?.clone();
                    // planificada: a chapa plana não leva o copo; a pegada do punção fica marcada
                    if sp.flat {
                        return self.puncao_planificado(f, &punch, &b);
                    }
                    let (plo, phi) = punch.bbox().ok_or("punção sem dimensões")?;
                    let pc = geo3d::lerp(plo, phi, 0.5);
                    let (on, n) = b.surface_near(pc).map_err(k)?;
                    // base do punção: a face dele encostada na chapa
                    let base = geo3d::add(on, geo3d::mul(n, 0.01));
                    let tags = vec![0; punch.face_count() as usize];
                    let (cup, _) = punch.shell_tagged(&[base], sp.t, false, &tags).map_err(k)?;
                    let cavity = punch.boolean(&cup, Bool::Cut).map_err(k)?;
                    let through = cavity.transformed(Xform::Translate(geo3d::mul(n, -sp.t))).map_err(k)?;
                    self.apply(f.target, through.compound(&cavity), BoolOp::Cut, f.id)?;
                    self.apply(f.target, cup, BoolOp::Add, f.id)?;
                    return Ok(None);
                }
                let (frame, sk) = &self.sketch(st.sketch)?;
                let frame = *frame;
                let p = if st.flip { frame.normal } else { frame.normal.map(|x| -x) };
                // planificada: a chapa plana não leva o copo; o contorno da ferramenta fica marcado
                if sp.flat {
                    return self.estampo_planificado(f, st, &frame, sk, sp.t, width);
                }
                match st.kind {
                    // nervura ao longo das curvas do sketch: seção trapezoidal varrida
                    1 => {
                        let w = if width > 0. { width } else { 4. * sp.t };
                        let chains: Vec<Vec<V3>> = sk.open.iter().chain(loops_points(&sk.loops).iter()).map(|c| c.iter().map(|q| frame.point(q[0], q[1])).collect()).collect();
                        if chains.is_empty() {
                            return Err("a nervura precisa de linhas ou curvas no sketch".into());
                        }
                        // B337: a booleana de uma nervura longa pode levar minutos; o erro vem antes
                        let _prazo = fenkra_kernel::limitar_tempo(60.);
                        // cada curva soma e depois corta: um erro no meio não pode deixar a soma na chapa
                        let antes = self.bodies.get(&f.target).cloned();
                        let feito = (|| -> Result<(), String> {
                        for spine in chains.iter().filter(|c| c.len() >= 2) {
                            let tan = geo3d::unit(geo3d::sub(spine[1], spine[0])).ok_or("curva nula")?;
                            let across = geo3d::unit(fenkra_kernel::cross(p, tan)).ok_or("curva normal à chapa")?;
                            let sec = Frame { origin: spine[0], x: across, normal: tan };
                            // y da seção = normal × x = p
                            let trap = |wb: f64, wt: f64, h: f64| -> fenkra_kernel::Loop2 {
                                let pts = [[-wb / 2., 0.], [wb / 2., 0.], [wt / 2., h], [-wt / 2., h], [-wb / 2., 0.]];
                                pts.windows(2).map(|q| fenkra_kernel::Seg2::Line(q[0], q[1])).collect()
                            };
                            let ydir = fenkra_kernel::cross(sec.normal, sec.x);
                            let s = if geo3d::dot(ydir, p) >= 0. { 1. } else { -1. };
                            let outer = Shape::face(&sec, &[trap(w + 2. * sp.t, w * 0.4 + 2. * sp.t, s * (depth + sp.t))]).map_err(k)?.sweep(spine, 0).map_err(k)?;
                            let inner = Shape::face(&sec, &[trap(w, w * 0.4, s * depth)]).map_err(k)?.sweep(spine, 0).map_err(k)?;
                            self.apply(f.target, outer, BoolOp::Add, f.id)?;
                            self.apply(f.target, inner, BoolOp::Cut, f.id)?;
                        }
                        Ok(())
                        })();
                        if let Err(e) = feito {
                            match antes {
                                Some(b) => self.bodies.insert(f.target, b),
                                None => self.bodies.remove(&f.target),
                            };
                            self.steps.remove(&f.id);
                            self.tools.remove(&f.id);
                            return Err(e);
                        }
                        Ok(chains.iter().any(|c| guia_com_cantos(c)).then(|| AVISO_GUIA_COM_CANTOS.to_string()))
                    }
                    // circular nos pontos: tronco de cone revolvido
                    2 => {
                        if sk.points.is_empty() {
                            return Err("o estampo circular precisa de pontos no sketch (centros)".into());
                        }
                        let d = if width > 0. { width } else { 8. * sp.t };
                        let r0 = d / 2.;
                        for q in &sk.points {
                            let o = frame.point(q[0], q[1]);
                            let x = geo3d::any_perp(p);
                            let fr = Frame { origin: o, x, normal: fenkra_kernel::cross(x, p) };
                            // no plano (raio, eixo): perfil fechado pelo eixo
                            let prof = |rb: f64, rt: f64, h: f64| -> fenkra_kernel::Loop2 {
                                let pts = [[0., 0.], [rb, 0.], [rt, h], [0., h], [0., 0.]];
                                pts.windows(2).map(|q| fenkra_kernel::Seg2::Line(q[0], q[1])).collect()
                            };
                            let ydir = fenkra_kernel::cross(fr.normal, fr.x);
                            let s = if geo3d::dot(ydir, p) >= 0. { 1. } else { -1. };
                            let outer = Shape::face(&fr, &[prof(r0 + sp.t, r0 * 0.6 + sp.t, s * (depth + sp.t))]).map_err(k)?.revolve(o, p, 360., 0.).map_err(k)?;
                            let inner = Shape::face(&fr, &[prof(r0, r0 * 0.6, s * depth)]).map_err(k)?.revolve(o, p, 360., 0.).map_err(k)?;
                            self.apply(f.target, outer, BoolOp::Add, f.id)?;
                            self.apply(f.target, inner, BoolOp::Cut, f.id)?;
                        }
                        Ok(None)
                    }
                    // reforço de dobra: nervura triangular no canto interno da dobra mais próxima
                    3 => {
                        if sp.flat {
                            return Ok(Some("planificada: reforço de dobra aparece só dobrado".into()));
                        }
                        if sk.points.is_empty() {
                            return Err("o reforço precisa de pontos no sketch (sobre a dobra)".into());
                        }
                        let w = if width > 0. { width } else { 2. * sp.t };
                        let leg = depth.max(sp.t * 2.);
                        for q in &sk.points {
                            let o = frame.point(q[0], q[1]);
                            // dobra mais perto do ponto
                            let m = self
                                .sheet_maps
                                .iter()
                                .min_by(|a, b| {
                                    let d = |m: &&SheetMap| {
                                        let (_, se, _) = m.local(o);
                                        geo3d::dist(o, geo3d::add(m.c0, geo3d::mul(m.ev, se.clamp(0., m.l))))
                                    };
                                    d(a).total_cmp(&d(b))
                                })
                                .copied()
                                .ok_or("não há dobra para reforçar")?;
                            let (_, se, _) = m.local(o);
                            let c = geo3d::add(m.c0, geo3d::mul(m.ev, se.clamp(0., m.l)));
                            // canto interno: base (para trás da dobra) e aba (depois do arco)
                            let a = geo3d::sub(c, geo3d::mul(m.u, leg));
                            let tip = m.to_folded(geo3d::add(c, geo3d::mul(m.u, m.ba + leg)));
                            let pts = [geo3d::sub(a, geo3d::mul(m.ev, w / 2.)), geo3d::sub(c, geo3d::mul(m.ev, w / 2.)), geo3d::sub(tip, geo3d::mul(m.ev, w / 2.))];
                            let rib = Shape::poly_prism(&pts, geo3d::mul(m.ev, w)).map_err(k)?;
                            self.apply(f.target, rib, BoolOp::Add, f.id)?;
                        }
                        Ok(None)
                    }
                    _ => {
                        if sk.loops.is_empty() {
                            return Err("o estampo precisa de um perfil fechado".into());
                        }
                        let face = Shape::face(&frame, &sk.loops).map_err(k)?;
                        // copo: perfil deslocado da espessura até o fundo, menos a cavidade
                        let outer = face.offset_face(sp.t).map_err(k)?.extrude(p, depth + sp.t, 0.).map_err(k)?;
                        let inner = face.extrude(p, depth, 0.).map_err(k)?;
                        self.apply(f.target, outer, BoolOp::Add, f.id)?;
                        self.apply(f.target, inner, BoolOp::Cut, f.id)?;
                        Ok(None)
                    }
                }
            }
            FeatureKind::SmFlangedHole(h) => {
                let sp = self.sheet()?;
                // tabela de furos flangeados: diâmetro e altura da linha escolhida
                let row = fenkra_core::FLANGED_HOLES.iter().find(|r| r.0 == h.standard);
                let dia = match row {
                    Some(r) => r.1,
                    None => ev(&h.diameter, &self.vars, "diâmetro")?,
                };
                let ht = match row {
                    Some(r) => r.2,
                    None => ev(&h.height, &self.vars, "altura do colar")?,
                };
                // centros: pontos do sketch ou pontos escolhidos na face (como um furo)
                let mut centers: Vec<(V3, V3)> = vec![];
                if h.points.is_empty() {
                    let (frame, sk) = &self.sketch(h.sketch)?;
                    if sk.points.is_empty() {
                        return Err("o sketch precisa de pontos (centros dos furos)".into());
                    }
                    let p = if h.flip { frame.normal } else { frame.normal.map(|x| -x) };
                    for q in &sk.points {
                        centers.push((frame.point(q[0], q[1]), p));
                    }
                } else {
                    let b = self.body(f.target).ok_or("o furo flangeado precisa de uma chapa")?.clone();
                    for q in &h.points {
                        let q = self.map_point(*q, f.flat_space, sp.flat);
                        let (on, n) = b.surface_near(q).map_err(k)?;
                        // o colar sai pelo outro lado da face escolhida (ou pelo mesmo, invertido)
                        centers.push((on, if h.flip { n } else { geo3d::mul(n, -1.) }));
                    }
                }
                let (mut ring, mut cut) = (Shape::empty(), Shape::empty());
                for (o, p) in centers {
                    let o2 = geo3d::add(o, geo3d::mul(p, sp.t));
                    let outer = Shape::hole_tool(o2, p, dia + 2. * sp.t, ht, HoleShape::Simple).map_err(k)?;
                    let inner = Shape::hole_tool(o2, p, dia, ht + 1., HoleShape::Simple).map_err(k)?;
                    let rg = outer.boolean(&inner, fenkra_kernel::Bool::Cut).map_err(k)?;
                    let c = Shape::hole_tool(geo3d::sub(o, geo3d::mul(p, 0.5)), p, dia, sp.t + ht + 1.5, HoleShape::Simple).map_err(k)?;
                    ring = if ring.is_null() { rg } else { ring.compound(&rg) };
                    cut = if cut.is_null() { c } else { cut.compound(&c) };
                }
                self.apply(f.target, ring, BoolOp::Add, f.id)?;
                self.apply(f.target, cut, BoolOp::Cut, f.id)?;
                Ok(None)
            }
            FeatureKind::SmCut(c) => {
                let sp = self.sheet()?;
                let (frame, sk) = &self.sketch(c.sketch)?;
                if sk.loops.is_empty() {
                    return Err("o recorte precisa de um perfil fechado".into());
                }
                let frame = *frame;
                // contorno nos dois estados, para a prévia simultânea (dobrado e planificado)
                {
                    let here: Vec<Vec<V3>> = loops_points(&sk.loops).iter().map(|l| l.iter().map(|q| frame.point(q[0], q[1])).collect()).collect();
                    let other = |from_flat: bool| -> Vec<Vec<V3>> {
                        if c.normal_face {
                            return here.clone();
                        }
                        here.iter().map(|l| l.iter().map(|q| self.map_outline_point(*q, from_flat, !from_flat)).collect()).collect()
                    };
                    let (folded, flat) = if f.flat_space { (other(true), here.clone()) } else { (here.clone(), other(false)) };
                    self.cut_outlines.push(CutOutline { feature: f.id, folded, flat, dobra: None });
                }
                let mut face = Shape::face(&frame, &sk.loops).map_err(k)?;
                let off = if c.offset.trim().is_empty() { 0. } else { ev(&c.offset, &self.vars, "folga")? };
                if off.abs() > 1e-9 {
                    face = face.offset_face(off).map_err(k)?;
                }
                // o sketch fica na face da chapa: corta para dentro (ou para o outro lado)
                let dir = if c.flip { frame.normal } else { frame.normal.map(|x| -x) };
                let big = sp.t * 4. + 2.;
                match c.mode {
                    1 => {
                        // com saída: paredes inclinadas a partir da face do sketch
                        let a = ev(&c.draft, &self.vars, "ângulo de saída")?;
                        let start = face.transformed(Xform::Translate(geo3d::mul(dir, -0.05))).map_err(k)?;
                        let tool = start.extrude_taper(dir, sp.t + 0.1, a).map_err(k)?;
                        self.apply(f.target, tool, BoolOp::Cut, f.id)?;
                    }
                    2 => {
                        // flangeado: colar dobrado em volta do recorte, saindo do outro lado
                        let ht = ev(&c.height, &self.vars, "altura do colar")?;
                        let exit = face.transformed(Xform::Translate(geo3d::mul(dir, sp.t))).map_err(k)?;
                        let outer = exit.offset_face(sp.t).map_err(k)?.extrude(dir, ht, 0.).map_err(k)?;
                        let inner = exit.extrude(dir, ht + 1., 0.).map_err(k)?;
                        let ring = outer.boolean(&inner, Bool::Cut).map_err(k)?;
                        self.apply(f.target, ring, BoolOp::Add, f.id)?;
                        let tool = face.extrude(dir, big, big).map_err(k)?;
                        self.apply(f.target, tool, BoolOp::Cut, f.id)?;
                    }
                    _ => {
                        let tool = face.extrude(dir, big, big).map_err(k)?;
                        self.apply(f.target, tool, BoolOp::Cut, f.id)?;
                    }
                }
                Ok(None)
            }
            FeatureKind::SmProfileFlange(pf) => {
                let sp = self.sheet()?;
                let (frame, sk) = &self.sketch(pf.sketch)?;
                let frame = *frame;
                if sk.open.is_empty() {
                    return Err("a aba por perfil precisa de uma cadeia aberta (linhas e arcos) no sketch".into());
                }
                let r = match &pf.radius {
                    Some(e) => ev(e, &self.vars, "raio")?,
                    None => sp.r,
                };
                if sp.flat {
                    let l1 = ev(&pf.l1, &self.vars, "comprimento 1")?;
                    let l2 = if pf.l2.trim().is_empty() { 0. } else { ev(&pf.l2, &self.vars, "comprimento 2")? };
                    return self.aba_perfil_planificada(f, &frame, sk, sp, r, l1, l2, !pf.flip);
                }
                // cantos arredondados no raio de dobra (linha média da chapa)
                let mut sk2 = (**sk).clone();
                sk2.loops.clear();
                sk2.open = sk.open.iter().map(|ch| round_corners(ch, r + sp.t / 2.)).collect();
                let t = format!("{}", sp.t);
                let (face, _) = self.profile_face(&frame, &sk2, Some(&t), if pf.flip { 1 } else { 0 })?;
                let l1 = ev(&pf.l1, &self.vars, "comprimento 1")?;
                let l2 = if pf.l2.trim().is_empty() { 0. } else { ev(&pf.l2, &self.vars, "comprimento 2")? };
                let tool = face.extrude(frame.normal, l1, l2).map_err(k)?;
                self.apply(f.target, tool, BoolOp::Add, f.id)?;
                Ok(None)
            }
            FeatureKind::SmCurveFlange(cf) => {
                let sp = self.sheet()?;
                if cf.edge.len() < 2 {
                    return Err("escolha uma aresta (reta ou curva) da chapa".into());
                }
                let len = ev(&cf.length, &self.vars, "comprimento")?;
                let ang = if cf.angle.trim().is_empty() { 90. } else { ev(&cf.angle, &self.vars, "ângulo")? };
                let r = match &cf.radius {
                    Some(e) => ev(e, &self.vars, "raio")?,
                    None => sp.r,
                };
                // a chave guarda uma aresta da cadeia; a polilinha é cache e acompanha o deslocamento dela
                let anchor = cf.edge[cf.edge.len() / 2];
                let (now, from) = self.chapa_aresta("aba curva", f, anchor, cf.edge_key.as_ref(), sp.flat)?;
                let shift = geo3d::sub(now, anchor);
                // no estado em que foi escolhida, a cadeia nasce de novo do B-rep pela tangência
                let viva = if cf.edge_key.is_some() && f.flat_space == sp.flat { self.cadeia_tangente(f.target, now, &cf.edge) } else { None };
                let pts: Vec<V3> = match viva {
                    Some(c) => c,
                    None => cf.edge.iter().map(|q| self.map_point(geo3d::add(*q, if from == f.flat_space && geo3d::norm(shift) > 1e-9 { shift } else { [0.; 3] }), from, sp.flat)).collect(),
                };
                self.curve_flange(f, pts, ang, r, len, cf.flip, None, cf.side)
            }
            FeatureKind::SmSurfFlange(sf) => {
                let sp = self.sheet()?;
                let len = ev(&sf.length, &self.vars, "comprimento (EOP)")?;
                let r = match &sf.radius {
                    Some(e) => ev(e, &self.vars, "raio")?,
                    None => sp.r,
                };
                let b = self.body(f.target).ok_or("a aba precisa de uma web (parede base)")?.clone();
                let fp = self.map_point(sf.face, f.flat_space, sp.flat);
                let (on, n) = b.surface_near(fp).map_err(k)?;
                // suporte que some (apagado, suprimido ou fora do alcance): falha explícita no comando
                let support = self.cut_shape(&sf.support, 0).map_err(|e| format!("o suporte da aba por superfície não está disponível ({e}): escolha o suporte de novo"))?;
                let (lo, hi) = b.bbox().ok_or("web sem dimensões")?;
                let big = geo3d::dist(lo, hi) * 2. + 10.;
                // OML: suporte ∩ plano da face base, só o trecho perto da web
                let plane = Shape::plane_face(on, n, big).map_err(k)?;
                let m = sp.t * 2. + r + 1.;
                let inside = |p: &V3| (0..3).all(|i| p[i] >= lo[i] - m && p[i] <= hi[i] + m);
                let step = (geo3d::dist(lo, hi) / 200.).max(0.05);
                // OML: maior trecho da interseção suporte × plano da web perto dela (subdividido)
                let oml = |sup: &Shape| -> Result<Vec<V3>, String> {
                    let mut best: Vec<V3> = vec![];
                    for cv in sup.section(&plane).map_err(k)? {
                        // subdivide (retas vêm só com as pontas) e fica com o trecho perto da web
                        let mut dense: Vec<V3> = vec![];
                        for w in cv.windows(2) {
                            let nseg = ((geo3d::dist(w[0], w[1]) / step).ceil() as usize).clamp(1, 4000);
                            for i in 0..nseg {
                                dense.push(geo3d::lerp(w[0], w[1], i as f64 / nseg as f64));
                            }
                        }
                        if let Some(l) = cv.last() {
                            dense.push(*l);
                        }
                        let part: Vec<V3> = dense.into_iter().filter(inside).collect();
                        if part.len() > best.len() {
                            best = part;
                        }
                    }
                    if best.len() < 2 {
                        return Err("o suporte não corta o plano da web perto dela".into());
                    }
                    Ok(best)
                };
                let mut support = support;
                let mut best = oml(&support)?;
                let mut mid = best[best.len() / 2];
                let (_, mut ns) = support.surface_near(mid).map_err(k)?;
                if sf.mode == 1 {
                    // aproximação: o suporte vira o plano tangente no meio da OML (dobra reta)
                    support = Shape::plane_face(mid, ns, big).map_err(k)?;
                    best = oml(&support)?;
                    mid = best[best.len() / 2];
                    ns = support.surface_near(mid).map_err(k)?.1;
                }
                let tan = geo3d::unit(geo3d::sub(best[(best.len() / 2 + 1).min(best.len() - 1)], best[(best.len() / 2).saturating_sub(1)])).ok_or("OML nula")?;
                let ctr = geo3d::lerp(lo, hi, 0.5);
                // relimita a web pelo suporte (fica o lado com o grosso do material)
                if !sf.keep_base {
                    let keep_pos = geo3d::dot(ns, geo3d::sub(ctr, mid)) > 0.;
                    let trimmed = b.split_side(&support, keep_pos, true).map_err(k)?;
                    if !trimmed.is_null() && trimmed.face_count() > 0 {
                        self.bodies.insert(f.target, fresh_body(trimmed, f.id));
                    }
                }
                // ângulo da aba: direção do suporte (perpendicular à OML) em relação à web
                let d = if sf.flip { geo3d::mul(n, -1.) } else { n };
                let mut u = geo3d::unit(fenkra_kernel::cross(ns, tan)).ok_or("suporte tangente à web")?;
                if geo3d::dot(u, d) < 0. {
                    u = geo3d::mul(u, -1.);
                }
                let mut out = geo3d::unit(fenkra_kernel::cross(tan, n)).ok_or("OML normal à web")?;
                if geo3d::dot(out, geo3d::sub(mid, ctr)) < 0. {
                    out = geo3d::mul(out, -1.);
                }
                let ang = if sf.mode == 2 {
                    // ângulo constante dado (a OML continua a do suporte)
                    if sf.angle.trim().is_empty() { 90. } else { ev(&sf.angle, &self.vars, "ângulo")? }
                } else {
                    geo3d::dot(u, d).atan2(geo3d::dot(u, out)).to_degrees()
                };
                // a OML fica na face e só onde há web (depois de relimitar)
                let web = self.body(f.target).cloned().unwrap_or(b);
                let pts: Vec<V3> = best
                    .iter()
                    .map(|q| geo3d::sub(*q, geo3d::mul(n, geo3d::dot(geo3d::sub(*q, on), n))))
                    .filter(|q| web.surface_near(*q).map(|(p, _)| geo3d::dist(p, *q) < 0.05).unwrap_or(false))
                    .collect();
                if pts.len() < 2 {
                    return Err("o suporte não encontra a borda da web".into());
                }
                self.curve_flange(f, pts, ang, r, len, sf.flip, Some(d), 0)
            }
            FeatureKind::SmArcSlot(sl) => {
                let sp = self.sheet()?;
                let rr = ev(&sl.radius, &self.vars, "raio do arco")?;
                let w = ev(&sl.width, &self.vars, "largura")?;
                let a0 = ev(&sl.start, &self.vars, "ângulo inicial")?.to_radians();
                let sw = ev(&sl.sweep, &self.vars, "varredura")?.to_radians();
                if w <= 0. || rr <= w / 2. || sw.abs() < 1e-6 {
                    return Err("raio maior que meia largura e varredura diferente de zero".into());
                }
                let b = self.body(f.target).ok_or("o rasgo precisa de uma chapa")?.clone();
                let c = self.map_point(sl.center, f.flat_space, sp.flat);
                let (on, n) = b.surface_near(c).map_err(k)?;
                let x = geo3d::any_perp(n);
                let fr = Frame { origin: on, x, normal: n };
                let pt = |r: f64, a: f64| [r * a.cos(), r * a.sin()];
                let (ro, ri) = (rr + w / 2., rr - w / 2.);
                let a1 = a0 + sw;
                let am = a0 + sw / 2.;
                let s = if sw > 0. { 1. } else { -1. };
                // ponta redonda: meio do semicírculo além do fim do arco
                let tip = |a: f64, ahead: f64| {
                    let c = pt(rr, a);
                    let t = [-a.sin() * ahead, a.cos() * ahead];
                    [c[0] + t[0] * w / 2., c[1] + t[1] * w / 2.]
                };
                let lp = vec![
                    fenkra_kernel::Seg2::Arc(pt(ro, a0), pt(ro, am), pt(ro, a1)),
                    fenkra_kernel::Seg2::Arc(pt(ro, a1), tip(a1, s), pt(ri, a1)),
                    fenkra_kernel::Seg2::Arc(pt(ri, a1), pt(ri, am), pt(ri, a0)),
                    fenkra_kernel::Seg2::Arc(pt(ri, a0), tip(a0, -s), pt(ro, a0)),
                ];
                let face = Shape::face(&fr, &[lp]).map_err(k)?;
                let tool = face.extrude(n, sp.t + 1., sp.t + 1.).map_err(k)?;
                self.apply(f.target, tool, BoolOp::Cut, f.id)?;
                Ok(None)
            }
            FeatureKind::SmCornerRelief(cr) => {
                let sp = self.sheet()?;
                let size = ev(&cr.size, &self.vars, "tamanho")?;
                let b = self.body(f.target).ok_or("o alívio precisa de uma chapa")?.clone();
                let at = self.map_point(cr.at, f.flat_space, sp.flat);
                // o canto fica numa aresta: um ponto um pouco para dentro (rumo ao centro da peça)
                // cai logo abaixo da face da chapa, que é a mais perto dele
                let (lo, hi) = b.bbox().ok_or("chapa sem dimensões")?;
                let inward = geo3d::unit(geo3d::sub(geo3d::lerp(lo, hi, 0.5), at)).unwrap_or([0., 0., -1.]);
                let (_, n) = b.surface_near(geo3d::add(at, geo3d::mul(inward, sp.t * 0.25))).map_err(k)?;
                let on = at;
                let top = geo3d::add(on, geo3d::mul(n, 1.));
                let z = geo3d::mul(n, -1.);
                let tool = match cr.kind {
                    1 => {
                        let x = geo3d::any_perp(z);
                        let y = fenkra_kernel::cross(z, x);
                        let o = geo3d::sub(geo3d::sub(top, geo3d::mul(x, size / 2.)), geo3d::mul(y, size / 2.));
                        Shape::box_tool(o, x, z, [size, size, sp.t * 2. + 2.]).map_err(k)?
                    }
                    2 => {
                        // gota: círculo no canto e um rasgo para dentro da chapa
                        let (lo, hi) = b.bbox().ok_or("chapa sem dimensões")?;
                        let v = geo3d::sub(geo3d::lerp(lo, hi, 0.5), on);
                        let v = geo3d::unit(geo3d::sub(v, geo3d::mul(n, geo3d::dot(v, n)))).unwrap_or(geo3d::any_perp(z));
                        let round = Shape::hole_tool(top, z, size, sp.t * 2. + 2., HoleShape::Simple).map_err(k)?;
                        let w = fenkra_kernel::cross(z, v);
                        let o = geo3d::sub(top, geo3d::mul(w, size * 0.25));
                        let slot = Shape::box_tool(o, w, z, [size * 0.5, size, sp.t * 2. + 2.]).map_err(k)?;
                        let slot = if geo3d::dot(fenkra_kernel::cross(z, w), v) < 0. { slot.transformed(Xform::Mirror(top, v)).map_err(k)? } else { slot };
                        round.compound(&slot)
                    }
                    _ => Shape::hole_tool(top, z, size, sp.t * 2. + 2., HoleShape::Simple).map_err(k)?,
                };
                self.apply(f.target, tool, BoolOp::Cut, f.id)?;
                Ok(None)
            }
            FeatureKind::SmHem(h) => {
                let sp = self.sheet()?;
                let gap = ev(&h.gap, &self.vars, "folga")?;
                let (angle, r) = match h.kind {
                    // gota: passa de 180° e fecha a ponta contra a chapa
                    1 => (200., (gap / 2.).max(sp.t * 0.5)),
                    // enrolada: três quartos de volta
                    2 => (270., (gap / 2.).max(sp.t)),
                    _ => (180., (gap / 2.).max(0.01)),
                };
                let p = fenkra_kernel::SheetFlange {
                    t: sp.t,
                    r,
                    angle,
                    length: ev(&h.length, &self.vars, "comprimento")?,
                    off1: 0.,
                    off2: 0.,
                    flip: h.flip,
                    flat: sp.flat,
                    k: sp.k_for(r),
                };
                let (h_edge, from) = self.chapa_aresta("bainha", f, h.edge, h.edge_key.as_ref(), sp.flat)?;
                self.sheet_tool(f, h_edge, from, p, false)
            }
            FeatureKind::StructMember(m) => {
                let mut geo = self.members(m)?;
                // B326: nós de dobra de tubo (pernas encadeadas num tubo só)
                let plano_dobra = self.plano_dobras(f, m, &geo);
                self.aplicar_dobras(&mut geo, &plano_dobra);
                let lines = self.all_member_lines();
                let my_k = self.doc.features.iter().position(|g| g.id == f.id).unwrap_or(0);
                // cantos com qualquer outra linha de membro (deste comando ou de outro)
                let diag = |g: &MemberGeom| (g.w * g.w + g.h * g.h).sqrt() + 1.;
                let mut joints: Vec<Vec<(V3, V3, V3, MemberGeom, CornerMode, bool, f64)>> = vec![vec![]; geo.len()];
                for i in 0..geo.len() {
                    let gi = geo[i].clone();
                    for (oid, oi, ok, og) in &lines {
                        if *oid == f.id && *oi == i {
                            continue;
                        }
                        // K-09: ponta que cai no meio de outra linha (T) também é canto; só Aparar e Encaixe
                        // valem, e é a ponta que chega que para no hospedeiro
                        let (p, di, dj, em_t) = match shared_end(&gi, og) {
                            Some((p, di, dj)) => (p, di, dj, false),
                            None => match t_junction(&gi, og) {
                                Some((p, di, dj)) => (p, di, dj, true),
                                None => continue,
                            },
                        };
                        let (mode, first, gap) = self.joint_rule((f.id, i, my_k), (*oid, *oi, *ok))?;
                        let (mode, first) = if em_t { (if matches!(mode, CornerMode::Aparar | CornerMode::Encaixe) { mode } else { CornerMode::Nenhum }, false) } else { (mode, first) };
                        // dobra de tubo: a dobradeira recua as pernas e põe um arco (nada a cortar); os
                        // demais modos cortam em esquadria (os rasgos em V são os itens B328 e B333)
                        let mode = if mode == CornerMode::Dobra {
                            match plano_dobra.no_do_par(i, *oi).filter(|_| *oid == f.id) {
                                None => continue,
                                Some(no) => match no.bend.mode {
                                    BendMode::Dobradeira => continue,
                                    // V múltiplo: as esquadrias ficam nos planos das articulações (B328);
                                    // a perna só ganha sobra na ponta
                                    BendMode::VComum | BendMode::CorteDobradeira if no.res.rasgos > 1 && no.res.erros.is_empty() => {
                                        let e = diag(&gi).max(diag(og));
                                        if geo3d::dist(p, gi.start) < 1e-4 {
                                            geo[i].ext.0 = geo[i].ext.0.max(e);
                                        } else {
                                            geo[i].ext.1 = geo[i].ext.1.max(e);
                                        }
                                        continue;
                                    }
                                    // gomos do tubo redondo: as pernas param nos planos das pontas do
                                    // polígono (`tratar_gomos`), sem esquadria única no nó
                                    BendMode::CorteSimples if no.res.gomos.is_some() && no.res.erros.is_empty() => {
                                        let e = diag(&gi).max(diag(og));
                                        if geo3d::dist(p, gi.start) < 1e-4 {
                                            geo[i].ext.0 = geo[i].ext.0.max(e);
                                        } else {
                                            geo[i].ext.1 = geo[i].ext.1.max(e);
                                        }
                                        continue;
                                    }
                                    _ => CornerMode::MeiaEsquadria,
                                },
                            }
                        } else {
                            mode
                        };
                        if mode == CornerMode::Nenhum {
                            continue;
                        }
                        // sobra na ponta para o tratamento cortar
                        let e = diag(&gi).max(diag(og));
                        if geo3d::dist(p, gi.start) < 1e-4 {
                            geo[i].ext.0 = geo[i].ext.0.max(e);
                        } else {
                            geo[i].ext.1 = geo[i].ext.1.max(e);
                        }
                        joints[i].push((p, di, dj, og.clone(), mode, first, gap));
                    }
                }
                let mid = |g: &MemberGeom| geo3d::lerp(g.start, g.end, 0.5);
                let mut shapes: Vec<Shape> = vec![];
                // B79: a falha de uma linha não derruba as outras; cada uma vira um aviso com o motivo
                let mut kept: Vec<usize> = vec![];
                let mut falhas: Vec<(usize, String)> = vec![];
                for (i, g) in geo.iter().enumerate() {
                    let built: Result<Shape, String> = (|| {
                    if let Some(e) = plano_dobra.erros.get(&i) {
                        return Err(e.clone());
                    }
                    let mut sh = self.member_shape(g)?;
                    for (p, di, dj, og, mode, first, gap) in &joints[i] {
                        match mode {
                            CornerMode::Nenhum | CornerMode::Dobra => {}
                            CornerMode::MeiaEsquadria => {
                                let n = geo3d::unit(geo3d::sub(*di, *dj)).ok_or("membros colineares não fazem meia-esquadria")?;
                                sh = sh.boolean(&half_space(geo3d::add(*p, geo3d::mul(n, gap / 2.)), n, self.size_hint())?, fenkra_kernel::Bool::Common).map_err(k)?;
                            }
                            CornerMode::Aparar | CornerMode::Encaixe if *first => {
                                // o membro 1 vai até a face de fora do membro 2
                                let hj = og.w.max(og.h) / 2.;
                                sh = sh.boolean(&half_space(geo3d::sub(*p, geo3d::mul(*di, hj)), *di, self.size_hint())?, fenkra_kernel::Bool::Common).map_err(k)?;
                            }
                            CornerMode::Aparar | CornerMode::Encaixe => {
                                // o membro 2 para no membro 1 (plano da face ou contorno, com folga)
                                let mut o2 = og.clone();
                                let hi = g.w.max(g.h) / 2.;
                                if geo3d::dist(*p, o2.start) < 1e-4 {
                                    o2.ext.0 = o2.ext.0.max(hi);
                                } else if geo3d::dist(*p, o2.end) < 1e-4 {
                                    o2.ext.1 = o2.ext.1.max(hi);
                                }
                                let prim = self.member_shape(&o2)?;
                                let prim = if *gap > 1e-9 { prim.offset_surface(*gap).map_err(k)? } else { prim };
                                sh = sh.clip_until(&prim, mid(g), *mode == CornerMode::Aparar).map_err(k)?;
                            }
                        }
                    }
                    Ok(sh)
                    })();
                    match built {
                        Ok(sh) => {
                            shapes.push(sh);
                            kept.push(i);
                        }
                        Err(e) => falhas.push((i, e)),
                    }
                }
                // B326: pernas ligadas por dobradeira viram um sólido só (perna + arco + perna)
                let mut fundidos: HashMap<usize, (f64, Frame)> = HashMap::new();
                let mut dobra_avisos: Vec<(String, Vec<String>)> = vec![];
                // B328, B333: nós com rasgo em V ou canto com gabarito recortam as duas pernas e
                // trazem os corpos que as ligam (dobradiça, peças do meio, setores)
                let mut extras_v: HashMap<(usize, usize), Vec<Shape>> = HashMap::new();
                for c in &plano_dobra.cadeias {
                    for no in c.nos.iter().filter(|n| st_dobra_geo::no_fundivel(n) && n.bend.mode != BendMode::Dobradeira) {
                        let (Some(pa), Some(pd)) = (kept.iter().position(|x| *x == no.antes), kept.iter().position(|x| *x == no.depois)) else { continue };
                        match self.tratar_no_v(&geo, no, &shapes[pa], &shapes[pd]) {
                            Ok(t) => {
                                shapes[pa] = t.antes;
                                shapes[pd] = t.depois;
                                extras_v.insert((no.antes, no.depois), t.extras);
                            }
                            Err(e) => {
                                for i in [no.antes, no.depois] {
                                    falhas.push((i, format!("dobra: {e}")));
                                }
                                for i in [no.antes, no.depois] {
                                    if let Some(q) = kept.iter().position(|x| *x == i) {
                                        kept.remove(q);
                                        shapes.remove(q);
                                    }
                                }
                            }
                        }
                    }
                }
                // B326: gomos do tubo redondo no corte simples: corta as pernas e traz as peças do meio
                let mut pecas_gomos: Vec<(String, st_dobra_gomos::PecaGomo)> = vec![];
                for c in &plano_dobra.cadeias {
                    for no in c.nos.iter().filter(|n| n.bend.mode == BendMode::CorteSimples && n.res.gomos.is_some() && n.res.erros.is_empty()) {
                        let (Some(pa), Some(pd)) = (kept.iter().position(|x| *x == no.antes), kept.iter().position(|x| *x == no.depois)) else { continue };
                        match self.tratar_gomos(&geo, no, no.res.juntas_gomos, &shapes[pa], &shapes[pd]) {
                            Ok((t, pecas)) => {
                                shapes[pa] = t.antes;
                                shapes[pd] = t.depois;
                                pecas_gomos.extend(pecas.into_iter().map(|p| (format!("{} {}–{}", m.designation, no.antes + 1, no.depois + 1), p)));
                            }
                            Err(e) => {
                                for i in [no.antes, no.depois] {
                                    falhas.push((i, format!("gomos: {e}")));
                                    if let Some(q) = kept.iter().position(|x| *x == i) {
                                        kept.remove(q);
                                        shapes.remove(q);
                                    }
                                }
                            }
                        }
                    }
                }
                for c in &plano_dobra.cadeias {
                    for n in &c.nos {
                        for a in &n.res.avisos {
                            // o mesmo aviso em vários nós aparece uma vez, com a lista das linhas
                            let nos = format!("{}–{}", n.antes + 1, n.depois + 1);
                            match dobra_avisos.iter_mut().find(|(t, _)| t == a) {
                                Some((_, l)) => l.push(nos),
                                None => dobra_avisos.push((a.clone(), vec![nos])),
                            }
                        }
                    }
                    for (run, arcos) in st_dobra_geo::blocos_fundidos(c) {
                        if run.len() < 2 || !run.iter().all(|i| kept.contains(i)) {
                            continue;
                        }
                        let fundir = || -> Result<Shape, String> {
                            let mut sh = shapes[kept.iter().position(|x| *x == run[0]).unwrap()].clone();
                            for (n, &k) in arcos.iter().enumerate() {
                                let no = &c.nos[k];
                                if no.bend.mode == BendMode::Dobradeira {
                                    sh = sh.boolean(&self.arco_do_no(&geo[no.antes], no)?, fenkra_kernel::Bool::Fuse).map_err(|e| e.0)?;
                                } else {
                                    for extra in extras_v.get(&(no.antes, no.depois)).into_iter().flatten() {
                                        sh = sh.boolean(extra, fenkra_kernel::Bool::Fuse).map_err(|e| e.0)?;
                                    }
                                }
                                sh = sh.boolean(&shapes[kept.iter().position(|x| *x == run[n + 1]).unwrap()], fenkra_kernel::Bool::Fuse).map_err(|e| e.0)?;
                            }
                            Ok(sh)
                        };
                        match fundir() {
                            Ok(sh) => {
                                let nos: Vec<st_dobra_geo::NoDobra> = arcos.iter().map(|k| c.nos[*k].clone()).collect();
                                let t = self.tubo_da_cadeia(f, m, &geo, &run, &nos);
                                // o referencial é o da ponta livre da primeira perna, apontando para dentro dela
                                let g0 = &geo[run[0]];
                                let no0 = &nos[0];
                                let livre = if geo3d::dist(g0.start, no0.p) < 1e-4 { g0.end } else { g0.start };
                                let frame = Frame { origin: livre, x: no0.x_in, normal: no0.t_in };
                                let pos = kept.iter().position(|x| *x == run[0]).unwrap();
                                shapes[pos] = sh;
                                fundidos.insert(run[0], (t.desenvolvido, frame));
                                for &i in &run[1..] {
                                    let q = kept.iter().position(|x| *x == i).unwrap();
                                    kept.remove(q);
                                    shapes.remove(q);
                                }
                            }
                            Err(e) => {
                                for &i in &run {
                                    falhas.push((i, format!("dobra: {e}")));
                                    if let Some(q) = kept.iter().position(|x| *x == i) {
                                        kept.remove(q);
                                        shapes.remove(q);
                                    }
                                }
                            }
                        }
                    }
                }
                if !falhas.is_empty() {
                    log_member_failures(f.id, &geo, &falhas);
                }
                if shapes.is_empty() && !falhas.is_empty() {
                    return Err(falhas.iter().map(|(i, e)| format!("linha {}: {e}", i + 1)).collect::<Vec<_>>().join("; "));
                }
                // emenda: barra maior que a matéria-prima
                let stock = self.doc.props.stock_length.trim();
                let warn = if stock.is_empty() {
                    None
                } else {
                    let lim = ev(stock, &self.vars, "comprimento máximo da barra")?;
                    let explicit = (ev(&m.ext1, &self.vars, "")?, ev(&m.ext2, &self.vars, "")?);
                    let long: Vec<String> = geo
                        .iter()
                        .map(|g| member_length(g, explicit))
                        .filter(|l| *l > lim + 1e-6)
                        .map(|l| format!("{:.0} mm ({} barras)", l, (l / lim).ceil()))
                        .collect();
                    (!long.is_empty() && lim > 0.).then(|| format!("⚠ emenda: barra maior que a matéria-prima de {lim:.0} mm — {}", long.join(", ")))
                };
                let warn = if dobra_avisos.is_empty() {
                    warn
                } else {
                    let juntos: Vec<String> = dobra_avisos.iter().map(|(a, l)| format!("linhas {}: {a}", l.join(", "))).collect();
                    let d = format!("⚠ dobra de tubo: {}", juntos.join(" | "));
                    Some(match warn {
                        Some(w) => format!("{d} | {w}"),
                        None => d,
                    })
                };
                let warn = if falhas.is_empty() {
                    warn
                } else {
                    let motivos = falhas.iter().map(|(i, e)| format!("linha {}: {e}", i + 1)).collect::<Vec<_>>().join("; ");
                    let base = format!("{MEMBRO_FALHOU} {} de {} linhas não geraram sólido — {motivos}", falhas.len(), geo.len());
                    Some(match warn {
                        Some(w) => format!("{base} | {w}"),
                        None => base,
                    })
                };
                if logging_members() {
                    let explicit = (ev(&m.ext1, &self.vars, "")?, ev(&m.ext2, &self.vars, "")?);
                    let profile = member_profile(m, &self.vars)?;
                    let name = format!("{} {}", m.family, m.designation);
                    for (&i, sh) in kept.iter().zip(&shapes) {
                        let g = &geo[i];
                        let (len, frame) = match fundidos.get(&i) {
                            Some((len, frame)) => (*len, *frame),
                            None => (member_length(g, explicit), Frame { origin: g.start, x: g.x, normal: g.z }),
                        };
                        log_struct(f, i, StructItem::Member, &name, profile.clone(), len, frame, sh, &m.material);
                    }
                    // peças do meio dos gomos: itens próprios da lista de corte (índice depois das linhas)
                    for (k, (nome, p)) in pecas_gomos.iter().enumerate() {
                        log_struct(f, geo.len() + k, StructItem::Member, &format!("{} {} (gomo)", m.family, nome), profile.clone(), p.comprimento, p.frame, &p.shape, &m.material);
                    }
                }
                let mut all = Shape::empty();
                for sh in shapes.iter().chain(pecas_gomos.iter().map(|(_, p)| &p.shape)) {
                    all = if all.is_null() { sh.clone() } else { all.compound(sh) };
                }
                self.tools.insert(f.id, (all.clone(), BoolOp::Add));
                self.add_separate(f, all);
                Ok(warn)
            }
            FeatureKind::StructPlate(pl) => self.struct_plate(f, pl),
            FeatureKind::StructGusset(gs) => self.struct_gusset(f, gs),
            FeatureKind::StructWeld(w) => self.struct_weld(f, w),
            FeatureKind::MoveBody(m) => {
                let (b, owners, roles) = self.body_owned(f.target).ok_or("não há corpo para mover")?;
                let rigid = match &m.xf {
                    BodyXf::Translate { dir, dist } => {
                        let v = geo3d::unit(self.axis(dir)?.1).ok_or("direção nula")?;
                        Some(Xform::Translate(geo3d::mul(v, ev(dist, &self.vars, "distância")?)))
                    }
                    BodyXf::Rotate { axis, angle } => {
                        let (o, v) = self.axis(axis)?;
                        Some(Xform::Rotate(o, v, ev(angle, &self.vars, "ângulo")?))
                    }
                    BodyXf::Symmetry { plane } => {
                        let fr = self.plane(plane, 0)?;
                        Some(Xform::Mirror(fr.origin, fr.normal))
                    }
                    BodyXf::Affinity { .. } => None,
                };
                match (rigid, &m.xf) {
                    (Some(x), _) => {
                        let (s, out) = b.transformed_tagged(x, &tags_for(owners)).map_err(k)?;
                        let body = remap_body(s, owners, roles, &out, f.id);
                        self.bodies.insert(f.target, body);
                    }
                    (None, BodyXf::Affinity { plane, fx, fy, fz }) => {
                        let fr = self.plane(plane, 0)?;
                        let kf = [ev(fx, &self.vars, "fator X")?, ev(fy, &self.vars, "fator Y")?, ev(fz, &self.vars, "fator Z")?];
                        let s = b.affinity(fr, kf).map_err(k)?;
                        self.bodies.insert(f.target, fresh_body(s, f.id));
                    }
                    _ => unreachable!(),
                }
                Ok(None)
            }
            FeatureKind::Scale(sc) => {
                let c = self.point(&sc.center, 0)?;
                let kf = ev(&sc.factor, &self.vars, "fator")?;
                let (b, owners, roles) = self.body_owned(f.target).ok_or("não há corpo para escalar")?;
                let (s, out) = b.transformed_tagged(Xform::Scale(c, kf), &tags_for(owners)).map_err(k)?;
                let body = remap_body(s, owners, roles, &out, f.id);
                self.bodies.insert(f.target, body);
                Ok(None)
            }
            FeatureKind::Stiffener(st) => {
                let (frame, sk) = &self.sketch(st.sketch)?;
                let frame = *frame;
                let segs: Vec<([f64; 3], [f64; 3])> = sk.lines.values().map(|(a, b)| (frame.point(a[0], a[1]), frame.point(b[0], b[1]))).collect();
                if segs.is_empty() {
                    return Err("o sketch do reforço precisa de linhas".into());
                }
                let t = ev(&st.thickness, &self.vars, "espessura")?;
                let depth = ev(&st.depth, &self.vars, "profundidade")?;
                // B11: um lado só = a parede inteira deslocada meia espessura para o lado da normal da linha
                let segs: Vec<([f64; 3], [f64; 3])> = match st.side {
                    1 | 2 => {
                        let sgn = if st.side == 1 { 1. } else { -1. };
                        segs.iter()
                            .map(|&(a, b)| {
                                let d = geo3d::unit(geo3d::sub(b, a)).unwrap_or([1., 0., 0.]);
                                let c = frame.normal;
                                let n = geo3d::unit([c[1] * d[2] - c[2] * d[1], c[2] * d[0] - c[0] * d[2], c[0] * d[1] - c[1] * d[0]]).unwrap_or([0.; 3]);
                                let m = [0, 1, 2].map(|i| n[i] * sgn * t / 2.);
                                ([0, 1, 2].map(|i| a[i] + m[i]), [0, 1, 2].map(|i| b[i] + m[i]))
                            })
                            .collect()
                    }
                    _ => segs,
                };
                let face = Shape::thick_segments(&segs, frame.normal, t).map_err(k)?;
                // entra no material como um furo (contra a normal); "inverter" troca o lado
                let dir = if st.reverse { frame.normal } else { frame.normal.map(|x| -x) };
                let tool = if st.to_body {
                    let body = self.body(f.target).ok_or("'até o corpo' precisa de um corpo no coletor")?;
                    face.extrude_to_next(dir, self.through(f.target)?, body).map_err(k)?
                } else {
                    face.extrude(dir, depth, 0.).map_err(k)?
                };
                self.apply(f.target, tool, BoolOp::Add, f.id)?;
                Ok(None)
            }
            FeatureKind::Thread(t) => {
                let spec = hardware::thread(&t.spec).ok_or_else(|| format!("rosca '{}' não encontrada na tabela de normas", t.spec))?;
                let b = self.body(f.target).ok_or("não há corpo para roscar")?;
                let cyl = b.cylinder_at(t.face).map_err(k)?;
                // B05: o início pode andar ao longo do eixo; o que sobra da face limita o comprimento
                let start = if t.start.trim().is_empty() { 0. } else { ev(&t.start, &self.vars, "início da rosca")? };
                if start < 0. || start >= cyl.h1 - cyl.h0 {
                    return Err("o início da rosca precisa ficar dentro da face cilíndrica".into());
                }
                let full = cyl.h1 - cyl.h0 - start;
                let len = match &t.end {
                    None => ev(&t.length, &self.vars, "comprimento")?.min(full),
                    Some(Extent::Blind(x)) => ev(x, &self.vars, "comprimento")?.min(full),
                    Some(Extent::UpToNext) => return Err("a rosca termina por comprimento, na face inteira ou até plano/superfície".into()),
                    // face inteira, com folga de 1/4 de passo (encostar na face vizinha faz a booleana falhar)
                    Some(_) => full - spec.pitch * 0.25,
                };
                let d = 2. * cyl.radius;
                let nominal = if cyl.internal { spec.tap_drill } else { spec.major };
                let note = ((d - nominal).abs() > 0.3 * spec.pitch).then(|| format!("Ø da face {d:.2} ≠ {nominal:.2} da rosca {}", spec.name));
                let o = [0, 1, 2].map(|i| cyl.origin[i] + cyl.dir[i] * (cyl.h0 + start));
                let sulco = t.modeled && self.leve != Some(f.id);
                self.roscas.push(RoscaViz { feature: f.id, origin: o, dir: cyl.dir, radius: cyl.radius, len, pitch: spec.pitch, esquerda: false, interna: cyl.internal, modelada: sulco });
                if !t.modeled {
                    return Ok(Some(note.unwrap_or_else(|| format!("rosca cosmética {}", spec.name))));
                }
                // prévia da edição: só a posição (linhas); o sulco é construído ao concluir
                if !sulco {
                    return Ok(note);
                }
                if len < spec.pitch {
                    return Err("comprimento menor que um passo".into());
                }
                let cut = match &t.end {
                    Some(e) => self.end_cutter(e, f.target)?,
                    None => None,
                };
                let ferramenta = |degrau: usize| -> Result<Shape, String> {
                    let tool = rosca_furo::ferramenta_no_degrau(o, cyl.dir, spec, len, cyl.internal, degrau)?;
                    match &cut {
                        Some((c, ext)) => tool.clip_until(c, o, *ext).map_err(k),
                        None => Ok(tool),
                    }
                };
                if !self.corta_rosca(f.target, f.id, 1e-6, &ferramenta) {
                    let aviso = format!("a rosca {} não removeu material: confira o diâmetro da face e o comprimento", spec.name);
                    return Ok(Some(match note {
                        Some(n) => format!("{n}; {aviso}"),
                        None => aviso,
                    }));
                }
                Ok(note)
            }
            FeatureKind::Combine(c) => {
                if c.tool == f.target {
                    return Err("o corpo ferramenta precisa ser de outro coletor".into());
                }
                let tool = self.body(c.tool).cloned().ok_or_else(|| format!("o coletor '{}' não tem corpo (ou vem depois)", self.doc.name_of(c.tool)))?;
                self.apply(f.target, tool, c.op, f.id)?;
                if !c.keep_tool {
                    self.bodies.remove(&c.tool);
                }
                Ok(None)
            }
            FeatureKind::SplitBody(sp) => {
                let b = self.body(f.target).ok_or("não há corpo para dividir")?.clone();
                let tool = self.cut_shape(&sp.surface, 0)?;
                let r = b.split_side(&tool, !sp.flip, matches!(sp.surface, SurfRef::Face { .. })).map_err(k)?;
                self.bodies.insert(f.target, fresh_body(r, f.id));
                Ok(None)
            }
            FeatureKind::ThickSurface(t) => {
                let sh = self.cut_shape(&t.surface, 0)?;
                let tool = sh.thicken(ev(&t.t1, &self.vars, "espessura 1")?, ev(&t.t2, &self.vars, "espessura 2")?).map_err(k)?;
                self.apply(f.target, tool, t.op, f.id)?;
                Ok(None)
            }
            FeatureKind::CloseSurface(c) => {
                let sh = self.cut_shape(&c.surface, 0)?;
                let tool = sh.closed_solid(ev(&c.tol, &self.vars, "tolerância")?).map_err(k)?;
                self.apply(f.target, tool, c.op, f.id)?;
                Ok(None)
            }
            FeatureKind::SewSurface(sw) => {
                let b = self.body(f.target).ok_or("não há corpo para costurar")?.clone();
                let sh = self.cut_shape(&sw.surface, 0)?;
                if sw.remove {
                    let r = b.split_side(&sh, !sw.flip, false).map_err(k)?;
                    self.bodies.insert(f.target, fresh_body(r, f.id));
                } else {
                    // volumes fechados entre a superfície e o corpo
                    let cells = b.compound(&sh).volume_cells(&[]).map_err(k)?;
                    self.apply(f.target, cells, BoolOp::Add, f.id)?;
                }
                Ok(None)
            }
            FeatureKind::Forma(fm) => {
                let (resolvida, aviso_vinculo) = self.resolver_forma(f.id, fm)?;
                let gaiola = resolvida.gaiola.clone();
                gaiola.inteira().topologia()?;
                if !fm.vinculos.vazio() {
                    self.formas_resolvidas.push(resolvida);
                }
                let Some(c) = fm.conversao.as_ref().filter(|c| !c.pausada) else { return Ok(aviso_vinculo) };
                let tol = ev(&c.tol, &self.vars, "tolerância")?;
                let (sh, nomes, rel) = forma_brep::converter(&gaiola, c.continuidade, tol)?;
                let aviso = [aviso_vinculo, forma_brep::resumo(&rel)].into_iter().flatten().reduce(|a, b| format!("{a}; {b}"));
                self.relatorios_forma.push((f.id, rel));
                if sh.is_solid() {
                    self.apply_roles(f.target, sh, BoolOp::Add, f.id, Some(nomes))?;
                } else {
                    let n = sh.face_count() as usize;
                    let corpo = match self.body(f.target) {
                        Some(b) => fresh_body(b.compound(&sh), f.id),
                        None => (sh, vec![f.id; n], nomes),
                    };
                    self.bodies.insert(f.target, corpo);
                }
                Ok(aviso)
            }
            FeatureKind::BoundaryFill(bf) => {
                let mut acc = Shape::empty();
                for t in &bf.tools {
                    let x = self.cut_shape(t, 0)?;
                    acc = if acc.is_null() { x } else { acc.compound(&x) };
                }
                if acc.is_null() {
                    return Err("escolha superfícies, faces ou planos que fechem volumes".into());
                }
                let cells = acc.volume_cells(&bf.exclude).map_err(k)?;
                self.apply(f.target, cells, bf.op, f.id)?;
                Ok(None)
            }
            FeatureKind::MeshImport(m) if m.corpo => self.importar_corpo_malha(f, m),
            FeatureKind::MeshOp(m) => self.operar_malha(f, m),
            FeatureKind::MeshImport(m) => {
                let tris = mesh_io::read(&m.path)?;
                let sh = Shape::from_mesh(&tris, m.solid).map_err(k)?;
                let n = tris.len() / 9;
                if sh.is_solid() {
                    self.apply(f.target, sh, BoolOp::Add, f.id)?;
                    Ok(Some(format!("{n} triângulos → sólido")))
                } else {
                    let r = match self.body(f.target) {
                        Some(b) => b.compound(&sh),
                        None => sh,
                    };
                    self.bodies.insert(f.target, fresh_body(r, f.id));
                    Ok(Some(format!("{n} triângulos → superfície aberta (a malha não fecha um volume)")))
                }
            }
            FeatureKind::LinkedBody(b) => {
                let sh = self.surface(b.geo, 0)?;
                // o B-rep de um corpo já volta sólido; casca fechada é costurada em sólido
                let sh = if sh.is_solid() { sh } else { sh.closed_solid(0.01).map_err(k)? };
                self.apply(f.target, sh, BoolOp::Add, f.id)?;
                Ok(None)
            }
            // arestas escolhidas, mas nenhuma ainda: não aplica nada (todas = só pela opção "Todas")
            FeatureKind::Fillet(o) | FeatureKind::Chamfer(o) if matches!(&o.edges, EdgeSel::Edges(m) if m.is_empty()) && matches!(o.mode, EdgeMode::Constant | EdgeMode::Variable(_) | EdgeMode::TwoDist(_) | EdgeMode::DistAngle(_)) => {
                Ok(Some("clique nas arestas na viewport (ou escolha 'Todas') — até lá nada é aplicado".into()))
            }
            FeatureKind::Fillet(o) | FeatureKind::Chamfer(o) => {
                let mapped;
                let o = match (self.sheet_mismatch(f.id), &o.edges) {
                    (Some((from, to)), EdgeSel::Edges(mids)) => {
                        let mut x = o.clone();
                        x.edges = EdgeSel::Edges(mids.iter().map(|q| self.map_point(*q, from, to)).collect());
                        // as chaves valem para o corpo como ele está, não para o levado entre dobrado e planificado
                        x.keys = vec![];
                        mapped = x;
                        &mapped
                    }
                    _ => o,
                };
                let v = ev(&o.value, &self.vars, "valor")?;
                let (b, owners, roles) = self.body_owned(f.target).ok_or("não há corpo para aplicar")?;
                let tags = tags_for(owners);
                let chamfer = matches!(f.kind, FeatureKind::Chamfer(_));
                match &o.mode {
                    EdgeMode::FaceFace(a, c) => {
                        let (s, out) = b.face_face_fillet_tagged(*a, *c, v, &tags).map_err(k)?;
                        let body = remap_body(s, owners, roles, &out, f.id);
                        self.bodies.insert(f.target, body);
                        return Ok(None);
                    }
                    EdgeMode::FullRound { mid, a, b: c } => {
                        let (fa, fc) = (b.face_props(*a).map_err(k)?, b.face_props(*c).map_err(k)?);
                        if fa.kind != 0 || fc.kind != 0 {
                            return Err("o filete completo precisa de duas faces laterais planas".into());
                        }
                        let width = geo3d::dot(geo3d::sub(fa.origin, fc.origin), fa.dir).abs();
                        let (s, out) = b.full_round_tagged(*mid, width, &tags).map_err(k)?;
                        let body = remap_body(s, owners, roles, &out, f.id);
                        self.bodies.insert(f.target, body);
                        return Ok(None);
                    }
                    _ => {}
                }
                // arestas escolhidas: a que tem chave v2 vira o ponto médio atual da aresta lógica; a que
                // só tem o ponto segue o ponto (e é promovida). Aresta perdida ou ambígua = erro.
                let picked = match &o.edges {
                    EdgeSel::Edges(m) if !m.is_empty() => Some(self.resolve_edges(if chamfer { "chanfro" } else { "filete" }, f, b, owners, roles, m, &o.keys)?),
                    // arestas criadas pelo comando: as que ficam entre duas faces que ele gerou (a aresta
                    // entre uma face dele e uma de outro comando já existia na peça)
                    EdgeSel::CreatedBy(id) => {
                        let topo = b.topology().map_err(k)?;
                        let mine = |fc: u32| fc != fenkra_kernel::NO_FACE && owners.get(fc as usize) == Some(id);
                        let mids: Vec<[f64; 3]> = (0..topo.edge_faces.len().min(topo.edge_mids.len())).filter(|e| mine(topo.edge_faces[*e][0]) && mine(topo.edge_faces[*e][1])).map(|e| topo.edge_mids[e]).collect();
                        if mids.is_empty() {
                            return Err(format!("'{}' não criou nenhuma aresta que sobre no corpo — escolha as arestas na viewport", self.doc.name_of(*id)));
                        }
                        Some(mids)
                    }
                    _ => None,
                };
                let tol = b.bbox().map(|(a, c)| (0..3).map(|i| (c[i] - a[i]).powi(2)).sum::<f64>().sqrt() * 0.05).unwrap_or(1.).max(1.);
                let (mode_code, v2) = match &o.mode {
                    EdgeMode::Variable(x) => (Some(0u8), ev(x, &self.vars, "raio final")?),
                    EdgeMode::TwoDist(x) => (Some(1), ev(x, &self.vars, "segunda distância")?),
                    EdgeMode::DistAngle(x) => (Some(2), ev(x, &self.vars, "ângulo")?),
                    EdgeMode::Asym(x) => (Some(3), ev(x, &self.vars, "raio da 2ª face")?),
                    _ => (None, 0.),
                };
                if mode_code.is_some() && picked.is_none() {
                    return Err("esta variação precisa das arestas escolhidas na viewport".into());
                }
                let partial = match &o.extra.partial {
                    Some([a, c]) => {
                        if picked.is_none() {
                            return Err("o trecho parcial precisa das arestas escolhidas na viewport".into());
                        }
                        Some((ev(a, &self.vars, "início do trecho")?, ev(c, &self.vars, "fim do trecho")?))
                    }
                    None => None,
                };
                // B49: pontos de controle do filete variável (posição em % -> 0 a 1; raio em mm)
                let mut controles: Vec<[f64; 2]> = vec![];
                if mode_code == Some(0) {
                    for (i, c) in o.extra.controls.iter().enumerate() {
                        let pos = ev(&c.at, &self.vars, &format!("posição do ponto {}", i + 1))?;
                        let raio = ev(&c.radius, &self.vars, &format!("raio do ponto {}", i + 1))?;
                        if !(pos > 0. && pos < 100.) {
                            return Err(format!("o ponto de controle {} fica em {pos}%: use um valor entre 0 e 100 (exclusive)", i + 1));
                        }
                        if raio <= 0. {
                            return Err(format!("o ponto de controle {} precisa de raio maior que zero", i + 1));
                        }
                        controles.push([pos / 100., raio]);
                    }
                    controles.sort_by(|a, c| a[0].total_cmp(&c[0]));
                }
                let run = |val: f64, val2: f64| match (&picked, mode_code) {
                    (Some(mids), Some(0)) if !controles.is_empty() => {
                        // a medida de apoio escala junto (quando o raio tentado é menor, os pontos também)
                        let fator = if v.abs() > 1e-12 { val / v } else { 1. };
                        let c: Vec<[f64; 2]> = controles.iter().map(|p| [p[0], p[1] * fator]).collect();
                        b.fillet_variable_tagged(val, val2, &c, o.extra.smooth, mids, tol, &tags)
                    }
                    (Some(mids), Some(m)) => b.edges_mode_tagged(m, val, val2, mids, &o.extra.flips, tol, &tags),
                    (Some(mids), None) => b.edges_tagged(val, mids, tol, chamfer, &tags),
                    (None, _) if chamfer => b.chamfer_tagged(val, &tags),
                    (None, _) => b.fillet_tagged(val, &tags),
                };
                let (s, out) = match run(v, v2) {
                    Ok(x) => x,
                    Err(e) => {
                        // a medida de apoio acompanha a tentada (menos o ângulo, que não escala)
                        let scales = matches!(mode_code, Some(0 | 1 | 3));
                        let fits = arestas::max_that_fits(v, |x| run(x, if scales { v2 * x / v } else { v2 }).is_ok());
                        return Err(self.explain_edge_failure(b, picked.as_deref(), chamfer, v.max(if scales { v2 } else { 0. }), fits, &e.0));
                    }
                };
                let (mut s, mut own, mut role) = remap_body(s, owners, roles, &out, f.id);
                // mínima ou parcial: o resultado da cadeia vale só junto das arestas escolhidas (e do trecho)
                if let (true, Some(mids)) = (o.prop == EdgeProp::Minima || partial.is_some(), &picked) {
                    let (s2, own2, role2) = self.minimal_edges(b, owners, roles, &s, &own, &role, mids, if mode_code == Some(2) { v } else { v.max(v2) }, partial, f.id)?;
                    s = s2;
                    own = own2;
                    role = role2;
                }
                self.bodies.insert(f.target, (s, own, role));
                Ok(None)
            }
        }
    }
}

/// A guia vira trechos retos no kernel (mesmo critério de `guia_suave`, em `features.inc`). Nesse caso a
/// seção não gira nos cantos: cada trecho sai como um prisma inclinado, e o sólido é aprovado (B337).
fn guia_com_cantos(pts: &[[f64; 3]]) -> bool {
    pts.windows(3).any(|q| match (geo3d::unit(geo3d::sub(q[1], q[0])), geo3d::unit(geo3d::sub(q[2], q[1]))) {
        (Some(a), Some(b)) => geo3d::dot(a, b).clamp(-1., 1.).acos() > 0.35,
        _ => false,
    })
}

const AVISO_GUIA_COM_CANTOS: &str = "a guia tem cantos e a seção não gira neles: depois de cada canto o perfil segue inclinado e o volume fica menor que o de um tubo; use uma guia suave";

/// Ângulo (graus) entre a normal do plano do perfil e a tangente da guia no primeiro trecho (B301).
fn varredura_desvio(normal: &[f64; 3], pts: &[[f64; 3]]) -> Option<f64> {
    let (a, b) = (pts.first()?, pts.iter().skip(1).find(|p| geo3d::dist(**p, pts[0]) > 1e-9)?);
    let t = geo3d::unit(geo3d::sub(*b, *a))?;
    let n = geo3d::unit(*normal)?;
    Some(geo3d::dot(n, t).abs().clamp(0., 1.).acos().to_degrees())
}

/// Recalcula o documento inteiro. Roda fora da thread da interface (RNF-001).
/// Superfície de cada forma livre que calculou. Fica fora do cache: a subdivisão leva menos que a busca.
fn formas_visiveis(doc: &Document, status: &HashMap<Id, Status>, resolvidas: &[forma_vinculo::FormaResolvida]) -> Vec<FormaViz> {
    doc.features
        .iter()
        .filter_map(|f| match &f.kind {
            FeatureKind::Forma(fm) if fm.conversao.as_ref().is_none_or(|c| c.pausada) => {
                // vínculo perdido é erro, mas a forma fica na tela onde estava: sumir seria mexer nela
                let ok = matches!(status.get(&f.id), Some(Status::Ok | Status::Warning(_)));
                if !ok && (fm.vinculos.vazio() || !matches!(status.get(&f.id), Some(Status::Error(_)))) {
                    return None;
                }
                let g = resolvidas.iter().find(|x| x.id == f.id).map_or(&fm.gaiola, |x| &x.gaiola);
                g.lisa(fm.niveis as u32).ok().map(|malha| FormaViz { id: f.id, malha, curvatura: vec![] })
            }
            _ => None,
        })
        .collect()
}

pub fn recompute(doc: &Document) -> RecomputeResult {
    recompute_cached(doc, &mut Cache::default())
}

/// Recalcula reaproveitando o cache: os comandos antes da primeira mudança não são refeitos.
/// Vale a configuração ativa da tabela de projeto (B102). Se o nome ativo não existe mais, calcula os
/// valores base e devolve o motivo em `param_error` (nunca troca por "a primeira" configuração).
pub fn recompute_cached(doc: &Document, cache: &mut Cache) -> RecomputeResult {
    if doc.tabela.ativa.is_empty() {
        return recompute_efetivo(doc, cache);
    }
    match doc.com_configuracao(&doc.tabela.ativa) {
        Ok(efetivo) => recompute_efetivo(&efetivo, cache),
        Err(e) => {
            let mut r = recompute_efetivo(doc, cache);
            r.param_error = Some(e.to_string());
            r
        }
    }
}

fn recompute_efetivo(doc: &Document, cache: &mut Cache) -> RecomputeResult {
    let t0 = Instant::now();
    let (vars, param_error) = match doc.eval_params() {
        // `espessura` vale em toda fórmula da chapa (raio = 1.15*espessura), do estilo ou do comando
        Ok(v) => (doc.sheet_efetivo().map(|s| s.vars_com_espessura(&v)).unwrap_or(v), None),
        Err(e) => (HashMap::new(), Some(e.to_string())),
    };
    // outra qualidade: as malhas guardadas não servem mais (os comandos continuam valendo)
    let qualidade_mudou = cache.meshes_q != Some(cache.qualidade);
    if qualidade_mudou {
        cache.meshes.clear();
        cache.fills.clear();
        cache.refs.clear();
        cache.meshes_q = Some(cache.qualidade);
    }
    let mut cx = Ctx {
        doc,
        vars,
        qualidade: cache.qualidade,
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
        malhas: HashMap::new(),
        roscas: vec![],
        previas: vec![],
        leve: cache.leve,
        relatorios_forma: vec![],
        formas_resolvidas: vec![],
        steps: HashMap::new(),
        miter_next: false,
        promoted: RefCell::new(None),
    };

    // 1º: sketches livres (pastas de geometria, sem linha do tempo); 2º: comandos dos coletores em ordem
    let order: Vec<usize> = (0..doc.features.len())
        .filter(|i| doc.is_free(&doc.features[*i]))
        .chain((0..doc.features.len()).filter(|i| !doc.is_free(&doc.features[*i])))
        .collect();
    // assinatura global: só o que vale para todos (links de outras peças, chapa, coletores).
    // Parâmetros e referências entram na assinatura de quem os usa — mudar um parâmetro refaz
    // só os comandos que citam esse parâmetro (e quem depende deles).
    let sheet_vars = serde_json::to_string(&(&doc.sheet, &doc.sheet_styles)).unwrap_or_default();
    let global = hash_of(
        &[
            &serde_json::to_string(&doc.external).unwrap_or_default(),
            &sheet_vars,
            &vars_sig(&sheet_vars, &cx.vars),
            &serde_json::to_string(&doc.collectors.iter().map(|c| c.id).collect::<Vec<_>>()).unwrap_or_default(),
        ],
        0,
    );
    // assinatura de todos os corpos (para quem lê faces de qualquer corpo)
    let all_bodies = |bs: &HashMap<Id, u64>| {
        let mut v: Vec<(u64, u64)> = bs.iter().map(|(k, v)| (k.0, *v)).collect();
        v.sort();
        format!("{v:?}")
    };
    let mut feat_out: HashMap<Id, u64> = HashMap::new();
    let mut seen: std::collections::HashSet<Id> = Default::default();
    let mut promoted: HashMap<Id, Promoted> = HashMap::new();
    cache.reused = 0;
    let ate_pos = cache.ate.take().and_then(|a| order.iter().position(|i| doc.features[*i].id == a));
    // forma com a gaiola aberta: os filhos esperam a conversão voltar
    let pausa = order.iter().position(|i| matches!(&doc.features[*i].kind, FeatureKind::Forma(f) if f.conversao.as_ref().is_some_and(|c| c.pausada)));
    let ate_pos = match (ate_pos, pausa) {
        (Some(a), Some(p)) => Some(a.min(p)),
        (a, p) => a.or(p),
    };
    let cancelar = cache.cancelar.clone();
    // as booleanas e a malha longas do OpenCASCADE também abandonam o cálculo quando a bandeira sobe
    let _guarda_kernel = cancelar.clone().map(fenkra_kernel::ligar_cancelamento);
    let progresso = cache.progresso.clone();
    if let Some(p) = &progresso {
        p.definir_total(order.iter().filter(|i| doc.is_active(**i)).count() as u32);
    }
    let mut parou = false;
    let mut cancelado = false;
    let mut desatualizados: Vec<Id> = vec![];
    for (pos, i) in order.into_iter().enumerate() {
        let f = &doc.features[i];
        seen.insert(f.id);
        if let Some(p) = &progresso {
            if pos > 0 {
                p.avancar();
            }
        }
        if !parou {
            if cancelar.as_ref().is_some_and(|c| c.load(std::sync::atomic::Ordering::Relaxed)) {
                parou = true;
                cancelado = true;
            } else if ate_pos.is_some_and(|a| pos > a) {
                parou = true;
            }
        }
        if parou {
            if doc.is_active(i) {
                desatualizados.push(f.id);
            }
            cx.status.insert(f.id, Status::Inactive);
            continue;
        }
        if !doc.is_active(i) {
            cx.status.insert(f.id, Status::Inactive);
            continue;
        }
        // entradas: o próprio comando, o corpo de destino, os comandos/corpos/referências citados
        let js = feature_sig(f);
        let mut ids = vec![];
        f.kind.clone().walk(&mut |x| ids.push(*x), &mut |_| {});
        let mut parts: Vec<String> = vec![js.clone(), vars_sig(&js, &cx.vars), format!("t{}", cx.body_sig.get(&f.target).copied().unwrap_or(0)), format!("{:?}", cx.last_sheet)];
        // rosca modelada em edição: o resultado leve não pode ser confundido com o completo
        if cache.leve == Some(f.id) && (rosca_furo::modelada(&f.kind) || previa_leve::cobre(&f.kind)) {
            parts.push("leve".into());
        }
        // desdobrar parcial: a mesma aba muda de resultado quando outro comando a desdobra
        if doc.features.iter().any(|g| matches!(g.kind, FeatureKind::SmUnfoldPartial(_))) {
            parts.push(format!("d{}", chapa_desdobra::desdobradas(doc).contains(&f.id)));
        }
        if let FeatureKind::Sketch(sk) = &f.kind {
            if let PlaneRef::Datum(pid) = sk.plane {
                parts.push(ref_sig(doc, pid, &cx.vars, 0));
            }
        }
        // membros estruturais: os cantos dependem das linhas dos outros membros
        if matches!(f.kind, FeatureKind::StructMember(_)) {
            for g in doc.features.iter().filter(|g| g.id != f.id && matches!(g.kind, FeatureKind::StructMember(_))) {
                parts.push(feature_sig(g));
                let mut gi = vec![];
                g.kind.clone().walk(&mut |x| gi.push(*x), &mut |_| {});
                for x in gi {
                    if doc.reference(x).is_some() {
                        parts.push(ref_sig(doc, x, &cx.vars, 0));
                    } else if let Some(sg) = feat_out.get(&x) {
                        parts.push(format!("f{}:{sg}", x.0));
                    }
                }
            }
            parts.push(doc.props.stock_length.clone());
        }
        let mut touched = vec![f.target];
        let mut all = ["\"Face\"", "\"Vertex\""].iter().any(|k| js.contains(k)) && !matches!(f.kind, FeatureKind::Sketch(_));
        // forma com vínculo lê a peça, inclusive aresta com chave, que nenhuma das palavras acima pega
        all |= matches!(&f.kind, FeatureKind::Forma(fm) if !fm.vinculos.vazio());
        if let FeatureKind::Sketch(sk) = &f.kind {
            // sketch: lê corpos se o plano ou a geometria externa vierem de faces/arestas
            let ext = serde_json::to_string(&sk.data.links).unwrap_or_default();
            all |= ext.contains("\"Edge\"") || ext.contains("\"Vertex\"") || ext.contains("\"Face\"");
            if let PlaneRef::Datum(pid) = sk.plane {
                all |= ref_reads_bodies(doc, pid, 0);
            }
        }
        for x in ids {
            if let Some(sg) = feat_out.get(&x) {
                parts.push(format!("f{}:{sg}", x.0));
            } else if doc.collectors.iter().any(|c| c.id == x) {
                if !touched.contains(&x) {
                    touched.push(x);
                }
                parts.push(format!("b{}:{}", x.0, cx.body_sig.get(&x).copied().unwrap_or(0)));
            } else if doc.reference(x).is_some() {
                parts.push(ref_sig(doc, x, &cx.vars, 0));
                all |= ref_reads_bodies(doc, x, 0);
            }
        }
        if all {
            parts.push(all_bodies(&cx.body_sig));
        }
        let sig = hash_of(&parts.iter().map(|x| x.as_str()).collect::<Vec<_>>(), global);
        match cache.feats.get(&f.id).filter(|e| e.sig == sig && !corpo_malha::sempre_roda(&f.kind)).cloned() {
            Some(e) => {
                // nada que ele lê mudou: reaproveita o que ele produziu
                cache.reused += 1;
                corpo_malha::ao_reaproveitar(&mut cx.malhas, f);
                let mut status_reusado = e.status.clone();
                cx.status.insert(f.id, e.status);
                if let Some(p) = e.promoted {
                    promoted.insert(f.id, p);
                }
                for (b, v) in e.bodies {
                    match v {
                        Some(v) => cx.bodies.insert(b, v),
                        None => cx.bodies.remove(&b),
                    };
                }
                // B413: calculado com a conferência adiada (comando aberto): confere agora, uma vez
                if cache.leve.is_none() && cache.sem_validar.remove(&f.id) {
                    let ruim = touched.iter().any(|b| validade::invalido(cx.body(*b)));
                    status_reusado = validade::com_aviso(status_reusado, ruim);
                    cx.status.insert(f.id, status_reusado.clone());
                    if let Some(ent) = cache.feats.get_mut(&f.id) {
                        ent.status = status_reusado;
                    }
                }
                if let Some(sk) = e.sketch {
                    cx.sketches.insert(f.id, sk);
                }
                match e.tool {
                    Some(t) => cx.tools.insert(f.id, t),
                    None => cx.tools.remove(&f.id),
                };
                cx.last_sheet = e.last_sheet;
                cx.sheet_maps.extend(e.sheet_maps.iter().copied());
                cx.cut_outlines.extend(e.outlines.iter().cloned());
                cx.roscas.extend(e.roscas.iter().cloned());
                cx.previas.extend(e.previas.iter().cloned());
                cx.relatorios_forma.extend(e.relatorios_forma.iter().cloned());
                cx.formas_resolvidas.extend(e.formas_resolvidas.iter().cloned());
                if e.steps.is_empty() {
                    cx.steps.remove(&f.id);
                } else {
                    cx.steps.insert(f.id, e.steps);
                }
            }
            None => {
                let maps_before = cx.sheet_maps.len();
                let outlines_before = cx.cut_outlines.len();
                let roscas_before = cx.roscas.len();
                let previas_before = cx.previas.len();
                let relatorios_before = cx.relatorios_forma.len();
                let resolvidas_before = cx.formas_resolvidas.len();
                *cx.promoted.borrow_mut() = None;
                let st = match cx.run(f) {
                    Ok(None) => Status::Ok,
                    Ok(Some(w)) => Status::Warning(w),
                    Err(e) => Status::Error(e),
                };
                // B413: o status ok não vê sólido inválido; confere os corpos que o comando mexeu. Com um
                // comando aberto para edição (prévia a cada tecla) a conferência espera: a conferência do
                // OpenCASCADE custa mais que muitas booleanas, e o comando é conferido ao ser reaproveitado
                let st = if matches!(f.kind, FeatureKind::Sketch(_)) || matches!(st, Status::Error(_)) {
                    st
                } else if cache.leve.is_some() {
                    cache.sem_validar.insert(f.id);
                    st
                } else {
                    cache.sem_validar.remove(&f.id);
                    let ruim = touched.iter().any(|b| validade::invalido(cx.body(*b)));
                    validade::com_aviso(st, ruim)
                };
                // cancelado no meio do kernel: o que veio é lixo (erro "cancelado" ou resultado parcial).
                // Não vai para o cache, para o próximo cálculo refazer este comando do zero.
                if matches!(st, Status::Error(_)) && cancelar.as_ref().is_some_and(|c| c.load(std::sync::atomic::Ordering::Relaxed)) {
                    parou = true;
                    cancelado = true;
                    desatualizados.push(f.id);
                    cx.status.insert(f.id, Status::Inactive);
                    continue;
                }
                cx.status.insert(f.id, st.clone());
                // chaves v2 calculadas para pontos antigos só valem se o comando deu certo
                let new_keys = cx.promoted.borrow_mut().take().filter(|_| !matches!(st, Status::Error(_)));
                if let Some(p) = &new_keys {
                    promoted.insert(f.id, p.clone());
                }
                cache.feats.insert(
                    f.id,
                    FeatEntry {
                        sig,
                        status: st,
                        promoted: new_keys,
                        bodies: touched.iter().map(|b| (*b, cx.bodies.get(b).cloned())).collect(),
                        sketch: cx.sketches.get(&f.id).cloned(),
                        tool: cx.tools.get(&f.id).cloned(),
                        last_sheet: cx.last_sheet,
                        sheet_maps: cx.sheet_maps[maps_before.min(cx.sheet_maps.len())..].to_vec(),
                        outlines: cx.cut_outlines[outlines_before.min(cx.cut_outlines.len())..].to_vec(),
                        roscas: cx.roscas[roscas_before.min(cx.roscas.len())..].to_vec(),
                        previas: cx.previas[previas_before.min(cx.previas.len())..].to_vec(),
                        relatorios_forma: cx.relatorios_forma[relatorios_before.min(cx.relatorios_forma.len())..].to_vec(),
                        formas_resolvidas: cx.formas_resolvidas[resolvidas_before.min(cx.formas_resolvidas.len())..].to_vec(),
                        steps: cx.steps.get(&f.id).cloned().unwrap_or_default(),
                    },
                );
            }
        }
        cx.memo.borrow_mut().clear();
        feat_out.insert(f.id, sig);
        // o sketch não muda corpo nenhum: quem usa o sketch já lê a assinatura dele
        if !matches!(f.kind, FeatureKind::Sketch(_)) {
            // prévia leve (B356): o corpo passou intacto, então a assinatura dele não depende dos campos do
            // comando; assim a malha em cache vale enquanto a pessoa edita, e o resultado leve continua
            // distinto do final para os comandos de depois
            let leve = cache.leve == Some(f.id) && previa_leve::cobre(&f.kind);
            for b in &touched {
                let nova = if leve { cx.body_sig.get(b).copied().unwrap_or(0).wrapping_mul(1_000_003).wrapping_add(0x1EE) } else { sig };
                cx.body_sig.insert(*b, nova);
            }
        }
    }
    cache.feats.retain(|id, _| seen.contains(id));
    // linhas de dobra no planificado: cada dobra leva as linhas pelas dobras anteriores
    let mut bend_lines: Vec<BendLine> = (0..cx.sheet_maps.len())
        .map(|i| {
            let m = cx.sheet_maps[i];
            let up_to = |q: V3| {
                let mut q = q;
                for mm in cx.sheet_maps[..i].iter().rev() {
                    if mm.holds(q, false) {
                        q = mm.to_flat(q);
                    }
                }
                q
            };
            let line = |du: f64| {
                let a = geo3d::add(m.c0, geo3d::mul(m.u, du));
                [up_to(a), up_to(geo3d::add(a, geo3d::mul(m.ev, m.l)))]
            };
            BendLine { curva: vec![], start: line(0.), center: line(m.ba / 2.), end: line(m.ba), angle: m.a.to_degrees().abs(), radius: geo3d::dist(m.o, m.c0), up: m.a > 0. && !m.down, feature: m.feature, eixo: Some(EixoDobra { a: m.c0, b: geo3d::add(m.c0, geo3d::mul(m.ev, m.l)), centro: m.o, normal: m.n, saida: m.u }), allowance: m.ba, deduction: 2. * (if m.a.abs() < 179.9_f64.to_radians() { (m.a.abs() / 2.).tan() * (geo3d::dist(m.o, m.c0) + m.t) } else { geo3d::dist(m.o, m.c0) + m.t }) - m.ba }
        })
        .collect();
    // cantos da aba por perfil (linhas soltas) entram no mapa; a ordem segue a dos comandos
    bend_lines.extend(cx.cut_outlines.iter().filter_map(|o| o.dobra.clone()));
    let ordem_cmd: HashMap<Id, usize> = doc.features.iter().enumerate().map(|(i, f)| (f.id, i)).collect();
    bend_lines.sort_by_key(|b| ordem_cmd.get(&b.feature).copied().unwrap_or(usize::MAX));
    let h = hash_of(&[&all_bodies(&cx.body_sig)], global);

    // planos: Origem + Referências. Todas entram no resultado (ocultas também: a cena filtra pela
    // visibilidade atual, então mostrar/ocultar não exige recálculo). Só reavalia a referência cuja
    // geometria mudou — ou, se ela lê faces de corpos, quando a linha do tempo mudou.
    let mut planes = vec![];
    for (p, n) in [(OriginPlane::XY, "XY"), (OriginPlane::XZ, "XZ"), (OriginPlane::YZ, "YZ")] {
        planes.push(PlaneViz { id: None, name: n.into(), frame: origin_frame(p) });
    }
    let tail = h;
    let (mut points, mut curves, mut surfaces) = (vec![], vec![], vec![]);
    cache.refs.retain(|id, _| doc.reference(*id).is_some());
    cache.notes.retain(|id, _| doc.reference(*id).is_some());
    cache.polir.esquecer_superficies(doc);
    for r in &doc.references {
        let key = hash_of(&[&ref_sig(doc, r.id, &cx.vars, 0)], if ref_reads_bodies(doc, r.id, 0) { tail } else { global });
        let v = match cache.refs.get(&r.id).filter(|(k, _)| *k == key) {
            Some((_, v)) => v.clone(),
            None => {
                // o plano de face vale para o fim da linha do tempo: a face pode ter sumido depois
                // de quem o usou, e o valor guardado por esse uso não diz isso
                if matches!(r.kind, RefKind::FacePlane { .. }) {
                    cx.memo.borrow_mut().remove(&r.id);
                }
                let v = cx.geo(r.id, 0).and_then(|g| {
                    Ok(match g {
                        Geo::Plane(fr) => RefViz::Plane(fr),
                        Geo::Point(p) => RefViz::Point(p),
                        Geo::Curve(c) => RefViz::Curve(c.pts),
                        Geo::Surface(sh) => {
                            cache.polir.lembrar_superficie(r.id, key, &sh);
                            RefViz::Surface(tesselar(&sh, &cx.qualidade).map_err(|e| e.0)?)
                        }
                    })
                });
                // erro de cálculo interrompido não vale como resultado: guardado, a referência ficaria
                // quebrada até mudar, e o pedido novo que cancelou este já vem recalcular
                let cancelado = cancelar.as_ref().is_some_and(|c| c.load(std::sync::atomic::Ordering::Relaxed));
                if v.is_ok() || !cancelado {
                    cache.refs.insert(r.id, (key, v.clone()));
                }
                match cx.notes.borrow().get(&r.id) {
                    Some(n) => cache.notes.insert(r.id, n.clone()),
                    None => cache.notes.remove(&r.id),
                };
                v
            }
        };
        match v {
            Ok(g) => {
                cx.status.insert(r.id, cache.notes.get(&r.id).filter(|n| n.warn).map_or(Status::Ok, |n| Status::Warning(n.text.clone())));
                match g {
                    RefViz::Plane(fr) => planes.push(PlaneViz { id: Some(r.id), name: r.name.clone(), frame: fr }),
                    RefViz::Point(p) => points.push(PointViz { id: r.id, name: r.name.clone(), p }),
                    RefViz::Curve(pts) => {
                        let verts = match &r.kind {
                            RefKind::Polyline { points, .. } | RefKind::Spline { points, .. } | RefKind::SplineControl { points, .. } => cx.points(points, 0).unwrap_or_default(),
                            _ => pts.first().copied().into_iter().chain(pts.last().copied()).collect(),
                        };
                        curves.push(CurveViz { id: r.id, name: r.name.clone(), pts, verts })
                    }
                    RefViz::Surface(mesh) => surfaces.push(SurfaceViz { id: r.id, name: r.name.clone(), mesh }),
                }
            }
            Err(e) => {
                cx.status.insert(r.id, Status::Error(e));
            }
        }
    }

    // sketches para exibição
    let mut fills_vivos = std::collections::HashSet::new();
    let mut sketches = vec![];
    for (id, (fr, es)) in &cx.sketches {
        let mut v = SketchViz {
            feature: *id,
            frame: *fr,
            lines: vec![],
            construction: vec![],
            points: vec![],
            fill: vec![],
            dof: es.report.dof,
            converged: es.report.converged,
            link_pts: es.link_pts.clone(),
            link_r: es.link_r.clone(),
            link_e: es.link_e.clone(),
            link_quebrados: es.link_quebrados.clone(),
            link_ops: es.link_ops.clone(),
            ents: es.curves.iter().map(|(cid, pts)| (*cid, es.cons.contains(cid), pts.iter().map(|q| to3d(fr, *q)).collect())).collect(),
            pt_ids: es.pts.iter().map(|(pid, q)| (*pid, to3d(fr, *q))).collect(),
        };
        // sombra: a face real (com furos internos) tesselada pelo kernel (com cache)
        if !es.loops.is_empty() {
            let key = (*id, hash_of(&[&format!("{:?}{:?}", es.loops, (fr.origin, fr.x, fr.normal))], 0));
            fills_vivos.insert(key);
            if let Some(f) = cache.fills.get(&key) {
                v.fill = f.clone();
            } else if let Ok(m) = Shape::face(fr, &es.loops).and_then(|f| tesselar(&f, &cx.qualidade)) {
                for t in m.indices.chunks_exact(3) {
                    for &i in t {
                        let k = i as usize * 3;
                        v.fill.push([m.positions[k], m.positions[k + 1], m.positions[k + 2]]);
                    }
                }
                cache.fills.insert(key, v.fill.clone());
            }
        }
        for (cons, pts) in &es.viz {
            let dst = if *cons { &mut v.construction } else { &mut v.lines };
            for w in pts.windows(2) {
                dst.push(to3d(fr, w[0]));
                dst.push(to3d(fr, w[1]));
            }
        }
        v.points = es.points.iter().map(|p| to3d(fr, *p)).collect();
        sketches.push(v);
    }

    // malhas dos corpos
    let mut vivas = std::collections::HashSet::new();
    let mut bodies = vec![];
    let mut formas_de_tela = vec![];
    for c in &doc.collectors {
        let Some((s, owners, roles)) = cx.body_owned(c.id) else { continue };
        let key = (c.id, cx.body_sig.get(&c.id).copied().unwrap_or(0));
        vivas.insert(key);
        formas_de_tela.push((c.id, key.1, s.clone()));
        let r = match cache.meshes.get(&key) {
            Some(m) => Ok(m.clone()),
            None => {
                // a forma guarda a malha da qualidade antiga, e o malhador a aceitaria com 10% de folga
                if qualidade_mudou {
                    s.discard_mesh().ok();
                }
                tesselar(&s, &cx.qualidade).inspect(|m| {
                    cache.meshes.insert(key, m.clone());
                })
            }
        };
        match r {
            Ok(mesh) => {
                let (mass, faces) = *cache.props.entry(key).or_insert_with(|| (s.mass(), s.face_count()));
                bodies.push(BodyResult {
                    collector: c.id,
                    color: c.color,
                    mass,
                    faces,
                    face_owner: owners.clone(),
                    face_role: roles.clone(),
                    mesh,
                });
            }
            Err(e) => {
                cx.status.insert(c.id, Status::Error(e.0));
            }
        }
    }
    bodies.extend(doc.collectors.iter().filter_map(|c| cx.malhas.get(&c.id).map(|m| corpo_malha::resultado(c, m))));
    cache.limitar(&vivas, &fills_vivos, TETO_CACHE_MALHAS);
    let mut formas = formas_visiveis(doc, &cx.status, &cx.formas_resolvidas);
    cache.polir.formas(doc, &mut formas, &cx.formas_resolvidas);
    let exibicao = {
        let corpos = formas_de_tela.into_iter().filter_map(|(id, sig, forma)| Some(polimento::Alvo { id, sig, forma, base: &bodies.iter().find(|b: &&BodyResult| b.collector == id)?.mesh }));
        let sups = surfaces.iter().filter_map(|sf: &SurfaceViz| cache.polir.superficie(sf.id).map(|(sig, forma)| polimento::Alvo { id: sf.id, sig, forma, base: &sf.mesh }));
        let alvos: Vec<_> = corpos.chain(sups).collect();
        cache.polir.aplicar(&alvos, &cx.qualidade, cancelar.as_deref())
    };

    RecomputeResult {
        notes: cache.notes.clone(),
        bodies,
        promoted,
        sketches,
        planes,
        points,
        curves,
        surfaces,
        formas,
        exibicao,
        status: cx.status,
        params: cx.vars,
        param_error,
        elapsed_ms: t0.elapsed().as_secs_f64() * 1000.,
        kernel: fenkra_kernel::version(),
        bend_lines,
        cut_outlines: cx.cut_outlines,
        desatualizados,
        cancelado,
        roscas: cx.roscas,
        previas: cx.previas,
        relatorios_forma: cx.relatorios_forma,
        formas_resolvidas: cx.formas_resolvidas,
    }
}

/// Publicações da peça (elementos e parâmetros marcados como publicados), já calculadas,
/// para outras peças copiarem por link.
pub fn publications(doc: &Document) -> Vec<(String, LinkedGeo)> {
    let r = recompute(doc);
    let mut out = vec![];
    for e in doc.references.iter().filter(|e| e.published) {
        if let Some(pl) = r.planes.iter().find(|p| p.id == Some(e.id)) {
            out.push((e.name.clone(), LinkedGeo::Plane { origin: pl.frame.origin, x: pl.frame.x, normal: pl.frame.normal }));
        } else if let Some(pt) = r.points.iter().find(|p| p.id == e.id) {
            out.push((e.name.clone(), LinkedGeo::Point(pt.p)));
        } else if let Some(c) = r.curves.iter().find(|c| c.id == e.id) {
            out.push((e.name.clone(), LinkedGeo::Curve(c.pts.clone())));
        } else if e.kind.class() == RefClass::Surface {
            // superfícies: a forma exata vai como texto B-rep
            if let Some(b) = surface_brep(doc, e.id) {
                out.push((e.name.clone(), LinkedGeo::Surface(b)));
            }
        }
    }
    for p in doc.params.iter().filter(|p| p.published) {
        if let Some(v) = r.params.get(&p.name) {
            out.push((p.name.clone(), LinkedGeo::Value(*v)));
        }
    }
    // corpos publicados: a forma exata (B-rep) — quem importa usa como superfície/corpo de referência
    if doc.collectors.iter().any(|c| c.published) {
        if let Ok(vars) = doc.eval_params() {
            let mut cx = Ctx {
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
        malhas: HashMap::new(),
        roscas: vec![],
        previas: vec![],
        leve: None,
        relatorios_forma: vec![],
        formas_resolvidas: vec![],
        steps: HashMap::new(),
        miter_next: false,
        promoted: RefCell::new(None),
            };
            for f in doc.active_features().map(|(_, f)| f) {
                let _ = cx.run(f);
            }
            for c in doc.collectors.iter().filter(|c| c.published) {
                if let Some(b) = cx.body(c.id).and_then(|b| b.to_brep().ok()) {
                    out.push((c.name.clone(), LinkedGeo::Surface(b)));
                }
            }
        }
    }
    out
}

/// Superfície de referência avaliada no fim da linha do tempo, em texto B-rep.
pub fn surface_brep(doc: &Document, id: Id) -> Option<String> {
    let vars = doc.eval_params().ok()?;
    let mut cx = Ctx {
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
        malhas: HashMap::new(),
        roscas: vec![],
        previas: vec![],
        leve: None,
        relatorios_forma: vec![],
        formas_resolvidas: vec![],
        steps: HashMap::new(),
        miter_next: false,
        promoted: RefCell::new(None),
    };
    for f in doc.active_features().map(|(_, f)| f) {
        let _ = cx.run(f);
    }
    match cx.geo(id, 0).ok()? {
        Geo::Surface(s) => s.to_brep().ok(),
        _ => None,
    }
}

/// Geometria de um membro estrutural.
#[derive(Clone, Debug)]
struct MemberGeom {
    start: V3,
    end: V3,
    /// eixo do membro e eixos da seção (x, y)
    z: V3,
    x: V3,
    /// eixo X da seção sem a rotação do comando (referência do arco de rotação)
    x0: V3,
    y: V3,
    w: f64,
    h: f64,
    /// espessura da parede do perfil (0 nas barras maciças)
    wall: f64,
    /// deslocamento da seção (ancoragem − deslocamentos)
    shift: [f64; 2],
    ext: (f64, f64),
    loops: Vec<fenkra_kernel::Loop2>,
    pts: Vec<V3>,
    straight: bool,
}

/// Ponta de `a` que cai no meio do eixo de `b` (junta em T): (ponto, direção para dentro de `a`,
/// direção de `b`). Fica de fora a ponta que já coincide com uma ponta de `b` (é `shared_end`) e a que
/// está fora da linha. A tolerância é de 0,05 mm do eixo.
fn t_junction(a: &MemberGeom, b: &MemberGeom) -> Option<(V3, V3, V3)> {
    let eixo = geo3d::sub(b.end, b.start);
    let comp = geo3d::norm(eixo);
    let u = geo3d::unit(eixo)?;
    for (pa, da) in [(a.start, a.z), (a.end, geo3d::mul(a.z, -1.))] {
        let t = geo3d::dot(geo3d::sub(pa, b.start), u);
        let perto = geo3d::norm(geo3d::sub(geo3d::sub(pa, b.start), geo3d::mul(u, t)));
        if perto < 0.05 && t > 1e-4 && t < comp - 1e-4 && geo3d::dot(da, u).abs() < 1. - 1e-6 {
            return Some((pa, da, b.z));
        }
    }
    None
}

/// Ponta comum de dois membros: (ponto, direção para dentro do primeiro, do segundo).
fn shared_end(a: &MemberGeom, b: &MemberGeom) -> Option<(V3, V3, V3)> {
    let close = |p: V3, q: V3| geo3d::dist(p, q) < 1e-4;
    let mz = |v: V3| geo3d::mul(v, -1.);
    for (pa, da) in [(a.start, a.z), (a.end, mz(a.z))] {
        for (pb, db) in [(b.start, b.z), (b.end, mz(b.z))] {
            if close(pa, pb) {
                return Some((pa, da, db));
            }
        }
    }
    None
}

/// Meio-espaço (caixa grande) do lado de `n` a partir do ponto `p`.
/// Cadeia aberta com os cantos trocados por arcos de raio `rad` (amostrados).
fn round_corners(p: &[[f64; 2]], rad: f64) -> Vec<[f64; 2]> {
    let n = p.len();
    if n < 3 || rad <= 1e-9 {
        return p.to_vec();
    }
    let unit2 = |v: [f64; 2]| {
        let l = (v[0] * v[0] + v[1] * v[1]).sqrt();
        (l > 1e-12).then(|| [v[0] / l, v[1] / l])
    };
    let mut out = vec![p[0]];
    for i in 1..n - 1 {
        let (a, b, c) = (p[i - 1], p[i], p[i + 1]);
        let (Some(v1), Some(v2)) = (unit2([b[0] - a[0], b[1] - a[1]]), unit2([c[0] - b[0], c[1] - b[1]])) else {
            out.push(b);
            continue;
        };
        let cosang = (v1[0] * v2[0] + v1[1] * v2[1]).clamp(-1., 1.);
        let turn = cosang.acos();
        if turn < 1e-3 {
            out.push(b);
            continue;
        }
        let l1 = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
        let l2 = ((c[0] - b[0]).powi(2) + (c[1] - b[1]).powi(2)).sqrt();
        let d = (rad * (turn / 2.).tan()).min(0.45 * l1.min(l2));
        let r = d / (turn / 2.).tan();
        let s0 = [b[0] - v1[0] * d, b[1] - v1[1] * d];
        // centro do arco: à esquerda (virando para a esquerda) ou à direita
        let left = v1[0] * v2[1] - v1[1] * v2[0] > 0.;
        let nrm = if left { [-v1[1], v1[0]] } else { [v1[1], -v1[0]] };
        let ctr = [s0[0] + nrm[0] * r, s0[1] + nrm[1] * r];
        let a0 = (s0[1] - ctr[1]).atan2(s0[0] - ctr[0]);
        let sgn = if left { 1. } else { -1. };
        for k in 0..=8 {
            let t = a0 + sgn * turn * k as f64 / 8.;
            out.push([ctr[0] + r * t.cos(), ctr[1] + r * t.sin()]);
        }
    }
    out.push(p[n - 1]);
    out
}

/// Pontos de cada laço fechado (linhas pelas pontas, arcos e círculos amostrados).
fn loops_points(loops: &[fenkra_kernel::Loop2]) -> Vec<Vec<[f64; 2]>> {
    loops
        .iter()
        .map(|lp| {
            let mut v: Vec<[f64; 2]> = vec![];
            for s in lp {
                match s {
                    fenkra_kernel::Seg2::Line(a, b) => {
                        v.push(*a);
                        v.push(*b);
                    }
                    fenkra_kernel::Seg2::Arc(a, m, b) => v.extend([*a, *m, *b]),
                    fenkra_kernel::Seg2::Circle(c, r) => v.extend((0..=32).map(|k| {
                        let t = k as f64 / 32. * std::f64::consts::TAU;
                        [c[0] + r * t.cos(), c[1] + r * t.sin()]
                    })),
                    fenkra_kernel::Seg2::Spline(p) => v.extend(p.iter().copied()),
                    fenkra_kernel::Seg2::Ellipse(c, m, r, t0, t1) => v.extend(fenkra_core::projecao::elipse_amostras(*c, *m, *r, *t0, *t1)),
                }
            }
            v.dedup_by(|x, y| (x[0] - y[0]).abs() + (x[1] - y[1]).abs() < 1e-9);
            v
        })
        .collect()
}

fn half_space(p: V3, n: V3, big: f64) -> Result<Shape, String> {
    let x = geo3d::any_perp(n);
    let y = fenkra_kernel::cross(n, x);
    let o = geo3d::sub(geo3d::sub(p, geo3d::mul(x, big)), geo3d::mul(y, big));
    Shape::box_tool(o, x, n, [2. * big, 2. * big, big]).map_err(|e| e.0)
}

/// Linha da lista de corte.
#[derive(Clone, Debug, PartialEq)]
pub struct CutRow {
    pub feature: Id,
    pub designation: String,
    pub length: f64,
    pub count: usize,
    /// barras de matéria-prima por peça (> 1 = precisa de emenda)
    pub bars: usize,
    /// número do item (o mesmo de `structural_groups`; 0 = membro sem sólido calculado)
    pub item: usize,
    /// membros da linha: (comando, trecho)
    pub members: Vec<(Id, usize)>,
    /// material do item (o da peça quando o membro não define o seu)
    pub material: String,
    /// ângulo de corte da ponta 1 (início da linha guia) e da ponta 2 (fim), em graus: 0 = corte
    /// reto, 45 = meia-esquadria de canto reto; `None` = ponta que não é plana (encaixe) ou sem sólido
    pub angle1: Option<f64>,
    pub angle2: Option<f64>,
    /// comprimento da ponta longa (maior extensão do sólido ao longo do eixo); só leitura, vem do sólido
    pub long: f64,
    /// tubo dobrado por dobradeira: `length` é o comprimento desenvolvido da barra reta (B331)
    pub tubo: Option<st_dobra::TuboDobrado>,
}

thread_local! {
    /// coleta dos sólidos estruturais durante `structural_members` (fora dela fica `None`)
    static MEMBER_LOG: RefCell<Option<Vec<MemberSolid>>> = const { RefCell::new(None) };
}

fn logging_members() -> bool {
    MEMBER_LOG.with(|l| l.borrow().is_some())
}

/// Prefixo do aviso de um membro que perdeu linhas (a interface o usa para pedir o contorno vermelho).
pub const MEMBRO_FALHOU: &str = "⚠ membro:";

/// Linha de membro que não gerou sólido: onde fica e por quê (B79).
#[derive(Clone, Debug, PartialEq)]
pub struct MemberFailure {
    pub feature: Id,
    /// índice da linha dentro do comando (a mesma numeração da lista de corte)
    pub line: usize,
    pub pts: Vec<V3>,
    pub reason: String,
}

thread_local! {
    static FAIL_LOG: RefCell<Option<Vec<MemberFailure>>> = const { RefCell::new(None) };
}

fn log_member_failures(f: Id, geo: &[MemberGeom], falhas: &[(usize, String)]) {
    FAIL_LOG.with(|l| {
        if let Some(v) = l.borrow_mut().as_mut() {
            v.extend(falhas.iter().map(|(i, e)| MemberFailure { feature: f, line: *i, pts: geo[*i].pts.clone(), reason: e.clone() }));
        }
    });
}

/// Linhas de membro que falharam no cálculo atual, para o contorno vermelho e o motivo no 3D.
/// Refaz os comandos estruturais; a interface só chama quando algum deles avisou a falha.
pub fn member_failures(doc: &Document) -> Vec<MemberFailure> {
    FAIL_LOG.with(|l| *l.borrow_mut() = Some(vec![]));
    let _ = final_shape(doc);
    FAIL_LOG.with(|l| l.borrow_mut().take()).unwrap_or_default()
}

/// Tipo de item da estrutura.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StructItem {
    Member,
    Plate,
    Gusset,
    /// cordão de solda sólido (B285)
    Weld,
}

/// Assinatura de um item: peças com a mesma assinatura são iguais (mesmo perfil, comprimento,
/// volume e caixa no referencial local — pontas diferentes separam os grupos).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StructSignature {
    pub kind: StructItem,
    /// família, designação, tipo e cotas avaliadas da seção (placa/reforço: só o nome)
    pub profile: String,
    /// comprimento em décimos de mm (0 = placa/reforço)
    pub length: i64,
    /// volume em mm³
    pub volume: i64,
    /// caixa local (mín. xyz, máx. xyz) em décimos de mm
    pub bbox: [i64; 6],
    /// material do item (vazio = o da peça); materiais diferentes nunca se agrupam (B80)
    pub material: String,
}

impl StructSignature {
    pub fn length_mm(&self) -> f64 {
        self.length as f64 / 10.
    }

    /// Texto estável da assinatura (chave do grupo nas propriedades das peças geradas).
    pub fn key(&self) -> String {
        let b = self.bbox.map(|v| v.to_string()).join(",");
        format!("{:?}|{}|L{}|V{}|B{}", self.kind, self.profile, self.length, self.volume, b)
    }
}

/// Sólido de um trecho de membro (ou de uma placa/reforço) no referencial local.
#[derive(Clone)]
pub struct MemberSolid {
    pub feature: Id,
    pub collector: Id,
    /// trecho dentro do comando (0 para placa e reforço)
    pub index: usize,
    pub kind: StructItem,
    /// nome para listas: "família designação", "Placa" ou "Reforço"
    pub name: String,
    /// referencial local no mundo: origem na ponta inicial, Z ao longo do membro, X/Y da seção
    pub frame: Frame,
    /// o sólido expresso em `frame`
    pub shape: Shape,
    pub signature: StructSignature,
}

/// Membro de um grupo.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StructRef {
    pub feature: Id,
    pub index: usize,
    pub frame: Frame,
}

/// Peças iguais da estrutura.
#[derive(Clone)]
pub struct StructGroup {
    /// número do item (1, 2, … na ordem de perfil e comprimento)
    pub item: usize,
    pub signature: StructSignature,
    pub name: String,
    /// sólido do primeiro membro, no referencial local
    pub shape: Shape,
    pub members: Vec<StructRef>,
}

/// Cotas da seção avaliadas, para a assinatura.
fn member_profile(m: &StructMember, vars: &Vars) -> Result<String, String> {
    let dims: Vec<String> = m.dims.iter().map(|e| ev(e, vars, "cota da seção").map(|v| format!("{v:.3}"))).collect::<Result<_, _>>()?;
    Ok(format!("{} {} {:?} [{}]", m.family, m.designation, m.kind, dims.join(", ")))
}

/// Décimos de mm; o deslocamento tira a quebra do arredondamento de valores comuns (x,05).
fn q10(v: f64) -> i64 {
    (v * 10. + 0.37).round() as i64
}

/// Leva o sólido do mundo para o referencial `fr`.
fn to_local(sh: &Shape, fr: &Frame) -> Result<Shape, String> {
    let k = |e: KernelError| e.0;
    let t = sh.transformed(Xform::Translate(geo3d::mul(fr.origin, -1.))).map_err(k)?;
    // linhas da matriz mundo→local = eixos do referencial
    let m = [fr.x, fr.y(), fr.normal];
    let tr = m[0][0] + m[1][1] + m[2][2];
    let ang = ((tr - 1.) / 2.).clamp(-1., 1.).acos();
    if ang.abs() < 1e-12 {
        return Ok(t);
    }
    let axis = if (std::f64::consts::PI - ang).abs() < 1e-6 {
        // 180°: eixo pela coluna dominante de (M + I)
        let c = (0..3).max_by(|i, j| m[*i][*i].total_cmp(&m[*j][*j])).unwrap();
        unit([m[0][c] + (c == 0) as u8 as f64, m[1][c] + (c == 1) as u8 as f64, m[2][c] + (c == 2) as u8 as f64])
    } else {
        unit([m[2][1] - m[1][2], m[0][2] - m[2][0], m[1][0] - m[0][1]])
    };
    t.transformed(Xform::Rotate([0.; 3], axis, ang.to_degrees())).map_err(k)
}

/// Registra um sólido estrutural (só durante `structural_members`).
#[allow(clippy::too_many_arguments)]
fn log_struct(f: &Feature, index: usize, kind: StructItem, name: &str, profile: String, length: f64, frame: Frame, shape: &Shape, material: &str) {
    if !logging_members() {
        return;
    }
    let Ok(local) = to_local(shape, &frame) else { return };
    let Some((lo, hi)) = local.bbox() else { return };
    let signature = StructSignature {
        kind,
        profile,
        length: (length * 10.).round() as i64,
        volume: local.mass().volume.round() as i64,
        bbox: [lo[0], lo[1], lo[2], hi[0], hi[1], hi[2]].map(q10),
        material: material.trim().to_string(),
    };
    let item = MemberSolid { feature: f.id, collector: f.target, index, kind, name: name.into(), frame, shape: local, signature };
    MEMBER_LOG.with(|l| {
        if let Some(v) = l.borrow_mut().as_mut() {
            v.push(item);
        }
    });
}

/// Um sólido por trecho de membro (com os cortes de canto e juntas), mais as placas e reforços,
/// cada um no seu referencial local.
pub fn structural_members(doc: &Document) -> Vec<MemberSolid> {
    if !doc.active_features().any(|(_, f)| matches!(f.kind, FeatureKind::StructMember(_) | FeatureKind::StructPlate(_) | FeatureKind::StructGusset(_) | FeatureKind::StructWeld(_))) {
        return vec![];
    }
    MEMBER_LOG.with(|l| *l.borrow_mut() = Some(vec![]));
    let _ = final_shape(doc);
    MEMBER_LOG.with(|l| l.borrow_mut().take()).unwrap_or_default()
}

/// Agrupa os sólidos de mesma assinatura; o número do item segue a ordem de perfil e comprimento.
pub fn group_members(items: &[MemberSolid]) -> Vec<StructGroup> {
    let mut by: std::collections::BTreeMap<StructSignature, StructGroup> = Default::default();
    for it in items {
        let r = StructRef { feature: it.feature, index: it.index, frame: it.frame };
        by.entry(it.signature.clone())
            .or_insert_with(|| StructGroup { item: 0, signature: it.signature.clone(), name: it.name.clone(), shape: it.shape.clone(), members: vec![] })
            .members
            .push(r);
    }
    by.into_values()
        .enumerate()
        .map(|(i, mut g)| {
            g.item = i + 1;
            g
        })
        .collect()
}

/// Grupos de peças iguais da estrutura (membros, placas e reforços), numerados.
pub fn structural_groups(doc: &Document) -> Vec<StructGroup> {
    group_members(&structural_members(doc))
}

/// Centro, no espaço da peça, do primeiro membro de cada item da lista de corte (âncora dos balões).
pub fn item_centers(doc: &Document) -> Vec<(usize, V3)> {
    let groups = structural_groups(doc);
    cut_list(doc)
        .iter()
        .filter(|r| r.item > 0)
        .filter_map(|r| {
            let g = groups.iter().find(|g| g.item == r.item)?;
            let (lo, hi) = g.shape.bbox()?;
            let c = [0, 1, 2].map(|i| (lo[i] + hi[i]) / 2.);
            let f = &g.members.first()?.frame;
            let y = f.y();
            Some((r.item, [0, 1, 2].map(|i| f.origin[i] + f.x[i] * c[0] + y[i] * c[1] + f.normal[i] * c[2])))
        })
        .collect()
}

/// Comprimento de corte de uma linha de membro (com os prolongamentos pedidos).
fn member_length(g: &MemberGeom, explicit: (f64, f64)) -> f64 {
    if g.straight {
        geo3d::dist(g.start, g.end) + explicit.0 + explicit.1
    } else {
        g.pts.windows(2).map(|w| geo3d::dist(w[0], w[1])).sum()
    }
}

/// Ângulo de corte das duas pontas de um membro, lido do próprio sólido (referencial local, Z ao
/// longo do eixo). Cada ponta é um plano: o ângulo é o desvio entre a normal desse plano e o eixo
/// (0 = corte reto, 45 = meia-esquadria de um canto de 90°). Ponta de encaixe (várias normais) = `None`.
pub fn end_angles(shape: &Shape) -> (Option<f64>, Option<f64>) {
    let Ok(mesh) = shape.mesh_angle(0.5, 0.35) else { return (None, None) };
    let p = |i: u32| {
        let k = i as usize * 3;
        [mesh.positions[k] as f64, mesh.positions[k + 1] as f64, mesh.positions[k + 2] as f64]
    };
    let (mut zmin, mut zmax) = (f64::MAX, f64::MIN);
    for v in mesh.positions.chunks(3) {
        zmin = zmin.min(v[2] as f64);
        zmax = zmax.max(v[2] as f64);
    }
    let zmid = (zmin + zmax) / 2.;
    // normais (área ponderada) das faces que não são paralelas ao eixo, por ponta
    let mut ends: [Vec<(V3, f64)>; 2] = [vec![], vec![]];
    for t in mesh.indices.chunks(3) {
        let (a, b, c) = (p(t[0]), p(t[1]), p(t[2]));
        let n = fenkra_kernel::cross(geo3d::sub(b, a), geo3d::sub(c, a));
        let area = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt() / 2.;
        if area < 1e-9 {
            continue;
        }
        let u = geo3d::mul(n, 1. / (2. * area));
        if u[2].abs() < 0.02 {
            continue;
        }
        let sign = if u[2] < 0. { -1. } else { 1. };
        let cz = (a[2] + b[2] + c[2]) / 3.;
        ends[usize::from(cz >= zmid)].push((geo3d::mul(u, sign), area));
    }
    let ang = |v: &[(V3, f64)]| -> Option<f64> {
        let tot: f64 = v.iter().map(|x| x.1).sum();
        if v.is_empty() || tot <= 0. {
            return None;
        }
        let mean = geo3d::unit(v.iter().fold([0.; 3], |s, (n, a)| geo3d::add(s, geo3d::mul(*n, *a))))?;
        // ponta plana: todas as normais apontam para o mesmo lado (tolerância de 0,5°)
        v.iter().all(|(n, _)| geo3d::dot(*n, mean) > 0.99996).then(|| (mean[2].clamp(-1., 1.).acos().to_degrees() * 10.).round() / 10.)
    };
    (ang(&ends[0]), ang(&ends[1]))
}

/// Lista de corte dos membros estruturais (comprimento de cada barra, agrupado por perfil e medida).
pub fn cut_list(doc: &Document) -> Vec<CutRow> {
    let Ok(vars) = doc.eval_params() else { return vec![] };
    let cx = Ctx {
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
        malhas: HashMap::new(),
        roscas: vec![],
        previas: vec![],
        leve: None,
        relatorios_forma: vec![],
        formas_resolvidas: vec![],
        steps: HashMap::new(),
        miter_next: false,
        promoted: RefCell::new(None),
    };
    let stock = ev(doc.props.stock_length.trim(), &cx.vars, "").ok().filter(|v| *v > 0.);
    let bars_of = |len: f64| stock.map(|s| (len / s).ceil().max(1.) as usize).unwrap_or(1);
    // mesmos grupos da estrutura: pontas diferentes viram linhas diferentes
    let groups = structural_groups(doc);
    let mut rows: Vec<CutRow> = groups
        .iter()
        .filter(|g| g.signature.kind == StructItem::Member)
        .map(|g| CutRow {
            feature: g.members[0].feature,
            designation: g.name.clone(),
            length: g.signature.length_mm(),
            count: g.members.len(),
            bars: bars_of(g.signature.length_mm()),
            item: g.item,
            members: g.members.iter().map(|m| (m.feature, m.index)).collect(),
            material: if g.signature.material.is_empty() { doc.props.material.clone() } else { g.signature.material.clone() },
            angle1: end_angles(&g.shape).0,
            angle2: end_angles(&g.shape).1,
            long: (g.signature.bbox[5] - g.signature.bbox[2]) as f64 / 10.,
            tubo: None,
        })
        .collect();
    // B331: o tubo dobrado em um sólido só leva as dobras, os raios e o modo na linha
    let tubos = tubos_dobrados(doc);
    if !tubos.is_empty() {
        for r in rows.iter_mut() {
            if let Some(&(f, i)) = r.members.first() {
                r.tubo = tubos.iter().find(|t| t.fundido && t.feature == f && t.linhas.first() == Some(&i)).cloned();
            }
        }
    }
    // comandos cujo sólido não foi calculado (erro no kernel) continuam na lista, só pelo comprimento
    let solid: std::collections::HashSet<Id> = groups.iter().flat_map(|g| g.members.iter().map(|m| m.feature)).collect();
    for f in doc.active_features().map(|(_, f)| f) {
        let FeatureKind::StructMember(m) = &f.kind else { continue };
        if solid.contains(&f.id) {
            continue;
        }
        let Ok(geo) = cx.members(m) else { continue };
        let explicit = (ev(&m.ext1, &cx.vars, "").unwrap_or(0.), ev(&m.ext2, &cx.vars, "").unwrap_or(0.));
        for (i, g) in geo.iter().enumerate() {
            let len = member_length(g, explicit);
            let len = (len * 10.).round() / 10.;
            let name = format!("{} {}", m.family, m.designation);
            match rows.iter_mut().find(|r| r.item == 0 && r.designation == name && (r.length - len).abs() < 0.05) {
                Some(r) => {
                    r.count += 1;
                    r.members.push((f.id, i));
                }
                None => rows.push(CutRow {
                    feature: f.id,
                    designation: name,
                    length: len,
                    count: 1,
                    bars: bars_of(len),
                    item: 0,
                    members: vec![(f.id, i)],
                    material: if m.material.trim().is_empty() { doc.props.material.clone() } else { m.material.trim().to_string() },
                    angle1: None,
                    angle2: None,
                    long: len,
                    tubo: None,
                }),
            }
        }
    }
    rows
}

/// Cantos entre linhas de membros estruturais: ((comando, linha), (comando, linha), ponto).
pub fn member_joints(doc: &Document) -> Vec<((Id, usize), (Id, usize), [f64; 3])> {
    let Ok(vars) = doc.eval_params() else { return vec![] };
    let cx = Ctx {
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
        malhas: HashMap::new(),
        roscas: vec![],
        previas: vec![],
        leve: None,
        relatorios_forma: vec![],
        formas_resolvidas: vec![],
        steps: HashMap::new(),
        miter_next: false,
        promoted: RefCell::new(None),
    };
    let lines = cx.all_member_lines();
    let mut out = vec![];
    for a in 0..lines.len() {
        for b in a + 1..lines.len() {
            if let Some((p, ..)) = shared_end(&lines[a].3, &lines[b].3) {
                out.push(((lines[a].0, lines[a].1), (lines[b].0, lines[b].1), p));
            }
        }
    }
    out
}

/// Cantos da estrutura (B37): cada ponto onde linhas de membros se encontram, com o tratamento que o
/// recompute aplica (regra própria ou padrão do comando), o membro que continua e o ângulo.
pub fn member_corners(doc: &Document) -> Vec<st_cantos::Canto> {
    let Ok(vars) = doc.eval_params() else { return vec![] };
    let cx = Ctx {
        relatorios_forma: vec![],
        formas_resolvidas: vec![],
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
        malhas: HashMap::new(),
        roscas: vec![],
        previas: vec![],
        leve: None,
        steps: HashMap::new(),
        miter_next: false,
        promoted: RefCell::new(None),
    };
    let lines = cx.all_member_lines();
    let perfil = |id: Id| match doc.feature(id).map(|f| &f.kind) {
        Some(FeatureKind::StructMember(m)) => format!("{} {}", m.family, m.designation),
        _ => String::new(),
    };
    let mut entradas = vec![];
    for a in 0..lines.len() {
        for b in a + 1..lines.len() {
            let Some((p, da, db)) = shared_end(&lines[a].3, &lines[b].3) else { continue };
            let (la, lb) = (&lines[a], &lines[b]);
            let Ok((modo, a_continua, folga)) = cx.joint_rule((la.0, la.1, la.2), (lb.0, lb.1, lb.2)) else { continue };
            // a outra ponta de cada linha é a que não está no canto
            let outra = |g: &MemberGeom| if geo3d::dist(g.start, p) < 1e-4 { g.end } else { g.start };
            entradas.push(st_cantos::EntradaCanto {
                a: (la.0, la.1),
                b: (lb.0, lb.1),
                ponto: p,
                dir_a: da,
                dir_b: db,
                fim_a: outra(&la.3),
                fim_b: outra(&lb.3),
                modo,
                a_continua,
                folga,
                explicito: st_cantos::tem_regra(doc, (la.0, la.1), (lb.0, lb.1)),
                perfil_a: perfil(la.0),
                perfil_b: perfil(lb.0),
            });
        }
    }
    st_cantos::montar_cantos(entradas)
}

/// Grupos de linhas contíguas do esqueleto (B226): índices em `paths`, cada grupo com as linhas que
/// dividem pontas entre si. Linha que não resolve fica sozinha no seu grupo.
pub fn skeleton_groups(doc: &Document, paths: &[CurveRef]) -> Vec<Vec<usize>> {
    let vars = doc.eval_params().unwrap_or_default();
    let cx = Ctx {
        notes: Default::default(),
        doc,
        vars,
        qualidade: Default::default(),
        bodies: HashMap::new(),
        sketches: HashMap::new(),
        status: HashMap::new(),
        tools: HashMap::new(),
        memo: RefCell::new(HashMap::new()),
        body_sig: HashMap::new(),
        last_sheet: None,
        sheet_maps: vec![],
        cut_outlines: vec![],
        malhas: HashMap::new(),
        roscas: vec![],
        previas: vec![],
        leve: None,
        relatorios_forma: vec![],
        formas_resolvidas: vec![],
        steps: HashMap::new(),
        miter_next: false,
        promoted: RefCell::new(None),
    };
    let nan = [f64::NAN; 3];
    let ends: Vec<(V3, V3)> = paths.iter().map(|p| cx.curve(p, 0).map(|c| (c.start(), c.end())).unwrap_or((nan, nan))).collect();
    structural::contiguous_groups(&ends, 1e-4)
}

/// Parâmetros de chapa já avaliados.
#[derive(Clone, Copy, Debug)]
struct SheetValues {
    t: f64,
    r: f64,
    flat: bool,
    /// dados do método de compensação, para recalcular o K com o raio de cada dobra
    method: u8,
    k0: f64,
    allowance: f64,
    /// largura do alívio padrão do estilo (a aba herda quando não define o próprio)
    relief: Option<f64>,
    relief_kind: u8,
}

impl SheetValues {
    /// Fator K da dobra com o raio próprio dela: nos métodos por dedução ou tolerância o K muda
    /// com o raio (a tabela vale a 90°), então o K da chapa só serve ao raio padrão.
    fn k_for(&self, r: f64) -> f64 {
        fenkra_core::sheet_k(self.method, self.k0, self.t, r, self.allowance)
    }
}

thread_local! {
    /// coleta dos furos durante `hole_table` (fora dela fica `None`)
    static HOLE_LOG: RefCell<Option<Vec<HoleRow>>> = const { RefCell::new(None) };
}

/// Como a boca do furo é: reta, com rebaixo de fundo plano, escareada (cone) ou com rebaixo de
/// fundo cônico.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FormaFuro {
    #[default]
    Reto,
    Rebaixo,
    Escareado,
    Contrafuro,
}

/// Linha da tabela de furos: centro da boca, direção, diâmetro, profundidade, rosca e a forma da
/// boca (B85). A boca é do lado de `pos`, voltada para fora do material (a cabeça assenta nela).
#[derive(Clone, Debug, PartialEq)]
pub struct HoleRow {
    pub feature: Id,
    pub pos: V3,
    pub dir: V3,
    pub diameter: f64,
    pub depth: f64,
    /// passante (atravessa tudo / até a última face)
    pub through: bool,
    pub thread: String,
    pub forma: FormaFuro,
    /// diâmetro do rebaixo ou da boca do cone (0 no furo reto)
    pub d_boca: f64,
    /// profundidade da boca contada da face: do rebaixo, do cone inteiro ou do contrafuro (0 no reto)
    pub prof_boca: f64,
    /// ângulo total do cone, em graus (0 quando não há cone)
    pub angulo: f64,
    /// classe de tolerância escrita na especificação do furo ("H7"); vazia quando não há
    pub tolerancia: String,
    /// comprimento da rosca medido da boca (0 quando o furo não é roscado)
    pub prof_rosca: f64,
}

/// Classe de tolerância ISO ("H7", "h6", "JS9") dentro de um texto de especificação: uma ou duas
/// letras seguidas de um ou dois algarismos. Palavras que começam em M (rosca métrica) não contam.
pub fn tolerancia_do_texto(texto: &str) -> String {
    let e_classe = |w: &str| {
        let letras = w.chars().take_while(|c| c.is_ascii_alphabetic()).count();
        let resto = &w[letras..];
        (1..=2).contains(&letras) && (1..=2).contains(&resto.len()) && resto.chars().all(|c| c.is_ascii_digit()) && !w.starts_with(['M', 'm'])
    };
    texto.split(|c: char| !c.is_ascii_alphanumeric()).find(|w| e_classe(w)).map(str::to_string).unwrap_or_default()
}

/// Profundidade do cone de um escareado de diâmetro `boca`, ângulo total `angulo` e furo `d`.
pub fn prof_escareado(boca: f64, d: f64, angulo: f64) -> f64 {
    if angulo <= 0. || angulo >= 180. || boca <= d {
        return 0.;
    }
    (boca - d) / 2. / (angulo.to_radians() / 2.).tan()
}

/// Todos os furos da peça (um por centro), na ordem da árvore.
pub fn hole_table(doc: &Document) -> Vec<HoleRow> {
    if !doc.active_features().any(|(_, f)| matches!(f.kind, FeatureKind::Hole(_))) {
        return vec![];
    }
    HOLE_LOG.with(|l| *l.borrow_mut() = Some(vec![]));
    let _ = final_shape(doc);
    HOLE_LOG.with(|l| l.borrow_mut().take()).unwrap_or_default()
}

/// Linha da tabela de dobras.
#[derive(Clone, Debug, PartialEq)]
pub struct BendRow {
    pub feature: Id,
    pub name: String,
    pub angle: f64,
    pub radius: f64,
    /// comprimento da dobra na linha neutra (bend allowance)
    pub allowance: f64,
    /// dedução de dobra (2 × recuo externo − allowance), para quem cota por fora
    pub deduction: f64,
    /// posição da dobra no mapa (1, 2, 3...): a mesma ordem das linhas de dobra da vista
    pub ordem: usize,
    /// etiqueta da vista ("↑ 90° R2"), igual à que a planificada mostra sobre a linha
    pub rotulo: String,
}

/// Prévia de um comando de chapa: o resultado que ele daria agora, sem gravar nada no documento.
#[derive(Clone, Debug)]
pub struct PreviaComando {
    /// malha do corpo com o comando aplicado (`None` quando o comando falha)
    pub malha: Option<Mesh>,
    /// `Ok`/`Warning` quando vale; `Error` traz a causa em texto (colisão com a aba vizinha, raio
    /// abaixo do mínimo...) para o painel mostrar em vermelho
    pub status: Status,
    pub volume: f64,
}

/// Aplica `kind` (com os valores padrão ou os que o painel tem agora) a uma cópia do documento e
/// devolve a malha resultante. O painel chama a cada mudança de campo: o documento real só ganha o
/// comando quando a pessoa confirma. Só mexe na cópia, então vale para qualquer comando.
pub fn previa_comando(doc: &Document, kind: FeatureKind) -> PreviaComando {
    let mut d = doc.clone();
    let id = d.add_feature(kind);
    let alvo = d.feature(id).map(|f| f.target);
    let r = recompute(&d);
    let status = r.status.get(&id).cloned().unwrap_or(Status::Inactive);
    let corpo = r.bodies.iter().find(|b| Some(b.collector) == alvo).or_else(|| r.bodies.first());
    match (&status, corpo) {
        (Status::Ok | Status::Warning(_), Some(b)) if !b.mesh.positions.is_empty() => PreviaComando { malha: Some(b.mesh.clone()), volume: b.mass.volume, status },
        _ => PreviaComando { malha: None, volume: 0., status },
    }
}

/// Contorno planificado de um recorte de chapa projetado (ortogonalmente) no plano do sketch dele,
/// em coordenadas do plano: serve a "projetar a planificada no sketch" para cotar contra ela.
pub fn contorno_planificado_no_sketch(doc: &Document, cut: Id) -> Result<Vec<Vec<[f64; 2]>>, String> {
    let f = doc.feature(cut).ok_or("recorte não encontrado")?;
    let FeatureKind::SmCut(c) = &f.kind else { return Err("o comando não é um recorte de chapa".into()) };
    let r = recompute(doc);
    let o = r.cut_outlines.iter().find(|o| o.feature == cut).ok_or("o recorte não tem contorno: ele está com erro, suprimido ou depois do marcador")?;
    let frame = r.sketches.iter().find(|s| s.feature == c.sketch).map(|s| s.frame).ok_or("o sketch do recorte não está disponível")?;
    let y = frame.y();
    Ok(o.flat.iter().map(|l| l.iter().map(|p| {
        let d = geo3d::sub(*p, frame.origin);
        [geo3d::dot(d, frame.x), geo3d::dot(d, y)]
    }).collect()).collect())
}

/// Tabela de dobras da peça de chapa (ângulo, raio, comprimento desenvolvido e dedução). Usa a
/// mesma compensação da geometria (`chapa::compensacao`) e o fator K do raio de cada dobra.
pub fn bend_table(doc: &Document) -> Vec<BendRow> {
    let Some(s) = &doc.sheet_efetivo() else { return vec![] };
    let Ok(vars) = doc.eval_params() else { return vec![] };
    let vars = s.vars_com_espessura(&vars);
    let num = |e: &String| ev(e, &vars, "").unwrap_or(0.);
    let (t, r0) = (num(&s.thickness), num(&s.radius));
    let k0 = if s.method == 0 { num(&s.k) } else { 0.44 };
    let allowance = if s.method == 0 { 0. } else { num(&s.allowance) };
    let mut out = vec![];
    for f in doc.active_features().map(|(_, f)| f) {
        // (ângulo, raio, quantidade de dobras iguais que o comando faz)
        let (ang, r, n) = match &f.kind {
            FeatureKind::SmFlange(fl) => (num(&fl.angle), fl.radius.as_ref().map(num).unwrap_or(r0), 1),
            FeatureKind::SmFlangeLoop(l) => (num(&l.angle), l.radius.as_ref().map(num).unwrap_or(r0), l.edges.len().max(1)),
            FeatureKind::SmHem(h) => (180., (num(&h.gap) / 2.).max(0.01), 1),
            FeatureKind::SmBend(b) => (num(&b.angle), b.radius.as_ref().map(num).unwrap_or(r0), 1),
            // abas por cadeia e por superfície também entram no mapa de dobras (ângulo e raio)
            FeatureKind::SmCurveFlange(c) => (if c.angle.trim().is_empty() { 90. } else { num(&c.angle) }, c.radius.as_ref().map(num).unwrap_or(r0), 1),
            FeatureKind::SmSurfFlange(sf) => (if sf.mode == 2 { num(&sf.angle) } else { 90. }, sf.radius.as_ref().map(num).unwrap_or(r0), 1),
            // joggle: duas dobras iguais e opostas, de canto vivo (sem raio modelado)
            FeatureKind::SmJoggle(j) => {
                let a = match &j.angle {
                    Some(a) => num(a),
                    None => num(&j.depth).atan2(num(&j.run).max(1e-9)).to_degrees(),
                };
                (a, 0., 2)
            }
            _ => continue,
        };
        let c = chapa::compensacao(t, r, ang, fenkra_core::sheet_k(s.method, k0, t, r, allowance));
        for i in 0..n {
            let name = if n > 1 { format!("{} ({}/{n})", f.name, i + 1) } else { f.name.clone() };
            out.push(BendRow { feature: f.id, name, angle: ang, radius: r, allowance: c.ba, deduction: c.deducao, ordem: out.len() + 1, rotulo: String::new() });
        }
    }
    out
}

/// Tabela de dobras do mapa: uma linha por linha de dobra da vista, na mesma ordem e com a mesma
/// etiqueta (`BendLine::label`). Os números vêm da própria dobra feita na geometria, então a
/// tabela, a vista e o DXF nunca discordam. `linhas` é `RecomputeResult::bend_lines`.
pub fn tabela_dobras(doc: &Document, linhas: &[BendLine]) -> Vec<BendRow> {
    let mut por_comando: HashMap<Id, usize> = HashMap::new();
    for l in linhas {
        *por_comando.entry(l.feature).or_default() += 1;
    }
    let mut visto: HashMap<Id, usize> = HashMap::new();
    linhas
        .iter()
        .enumerate()
        .map(|(i, l)| {
            let nome = doc.feature(l.feature).map(|f| f.name.clone()).unwrap_or_else(|| "dobra".into());
            let total = por_comando[&l.feature];
            let k = visto.entry(l.feature).or_default();
            *k += 1;
            let name = if total > 1 { format!("{nome} ({k}/{total})") } else { nome };
            BendRow { feature: l.feature, name, angle: l.angle, radius: l.radius, allowance: l.allowance, deduction: l.deduction, ordem: i + 1, rotulo: l.label() }
        })
        .collect()
}

/// Geometria guardada de um link (ou cópia isolada).
fn linked_geo(g: &LinkedGeo) -> Result<Geo, String> {
    Ok(match g {
        LinkedGeo::Plane { origin, x, normal } => Geo::Plane(Frame { origin: *origin, x: *x, normal: *normal }),
        LinkedGeo::Point(p) => Geo::Point(*p),
        LinkedGeo::Curve(pts) => Geo::Curve(Curve3::poly(pts.clone(), false)?),
        LinkedGeo::Surface(b) => Geo::Surface(Shape::from_brep(b).map_err(|e| e.0)?),
        LinkedGeo::Value(_) => return Err("publicação de valor vira parâmetro, não geometria".into()),
    })
}

/// Formato de arquivo que a exportação sabe gravar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FormatoExport {
    Step,
    Stl,
}

impl FormatoExport {
    /// Formato pela extensão, sem diferenciar maiúsculas. Recusa o resto: gravar STEP num arquivo
    /// `.obj` deixa um arquivo que nenhum programa abre, sem aviso.
    pub fn do_caminho(path: &str) -> Result<Self, String> {
        let ext = std::path::Path::new(path).extension().map(|e| e.to_string_lossy().to_lowercase());
        match ext.as_deref() {
            Some("step" | "stp") => Ok(Self::Step),
            Some("stl") => Ok(Self::Stl),
            Some(e) => Err(format!("formato de exportação não reconhecido: .{e} (use .step, .stp ou .stl)")),
            None => Err("o arquivo de exportação precisa de extensão: .step, .stp ou .stl".into()),
        }
    }
}

/// Exporta o corpo de um coletor (ou de todos) após recalcular; STL no preset normal.
pub fn export(doc: &Document, collector: Option<Id>, path: &str) -> Result<(), String> {
    export_com_preset(doc, collector, path, PresetStl::default())
}

/// Como `export`, escolhendo a malha do STL (o preset não muda o STEP, que guarda a forma exata).
pub fn export_com_preset(doc: &Document, collector: Option<Id>, path: &str, preset: PresetStl) -> Result<(), String> {
    let formato = FormatoExport::do_caminho(path)?;
    let vars = doc.eval_params().map_err(|e| e.to_string())?;
    let mut cx = Ctx {
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
        malhas: HashMap::new(),
        roscas: vec![],
        previas: vec![],
        leve: None,
        relatorios_forma: vec![],
        formas_resolvidas: vec![],
        steps: HashMap::new(),
        miter_next: false,
        promoted: RefCell::new(None),
    };
    for f in doc.active_features().map(|(_, f)| f) {
        let _ = cx.run(f);
    }
    let mut all = Shape::empty();
    for c in &doc.collectors {
        if collector.is_some_and(|x| x != c.id) {
            continue;
        }
        if let Some(b) = cx.body(c.id) {
            all = if all.is_null() { b.clone() } else { all.compound(b) };
        }
    }
    let com_malha = doc.collectors.iter().any(|c| collector.is_none_or(|x| x == c.id) && cx.malhas.contains_key(&c.id));
    if com_malha {
        return match formato {
            FormatoExport::Stl => corpo_malha::exportar_stl(&cx, collector, &all, path, preset),
            FormatoExport::Step => Err("corpo de malha não tem geometria exata: converta em sólido antes de gravar STEP".into()),
        };
    }
    if all.is_null() {
        return Err("nada para exportar".into());
    }
    let r = match formato {
        FormatoExport::Step => all.export_step(path),
        FormatoExport::Stl => {
            let q = preset.qualidade();
            let diagonal = all.bbox().map(|(a, b)| geo3d::dist(a, b)).unwrap_or(100.);
            all.export_stl(path, q.desvio(diagonal), q.angulo_rad())
        }
    };
    r.map_err(|e| e.0)
}

/// Forma final da peça (todos os corpos visíveis juntos) — usada pela montagem para achar faces
/// e eixos das restrições.
pub fn final_shape(doc: &Document) -> Option<Shape> {
    // B102: vale a configuração ativa; se ela não existe mais, não há forma (nunca a primeira linha)
    let efetivo = doc.efetivo().ok()?;
    let doc: &Document = &efetivo;
    let vars = doc.eval_params().ok()?;
    let mut cx = Ctx {
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
        malhas: HashMap::new(),
        roscas: vec![],
        previas: vec![],
        leve: None,
        relatorios_forma: vec![],
        formas_resolvidas: vec![],
        steps: HashMap::new(),
        miter_next: false,
        promoted: RefCell::new(None),
    };
    for f in doc.active_features().map(|(_, f)| f) {
        let _ = cx.run(f);
    }
    let mut all = Shape::empty();
    for c in doc.collectors.iter().filter(|c| c.visible) {
        if let Some(b) = cx.body(c.id) {
            all = if all.is_null() { b.clone() } else { all.compound(b) };
        }
    }
    (!all.is_null()).then_some(all)
}

/// Ponto fixo de construção na origem do sketch (referência para cotas).
pub fn origin_point(sk: &mut SketchData, al: &mut impl FnMut() -> Id) -> Id {
    let o = sk.add_point(al, 0., 0.);
    sk.point_mut(o).unwrap().fixed = true;
    sk.add_curve(al, Curve::Point { p: o }, true);
    o
}

/// Exemplo usado pelo comando "Nova peça de exemplo" e pelos testes:
/// placa parametrizada com rasgo, furos rebaixados M6 e filete.
pub fn example_document() -> Document {
    let s = |v: &str| v.to_string();
    let mut d = Document::new("Placa de exemplo");
    d.props.pn = s("FK-0001");
    d.props.description = s("Placa base");
    d.add_param("L", "120", Unit::Comprimento).unwrap();
    d.add_param("W", "L*0.6", Unit::Comprimento).unwrap();
    d.add_param("esp", "10", Unit::Comprimento).unwrap();
    d.add_param("margem", "12", Unit::Comprimento).unwrap();

    // sketch base: retângulo L × W centrado na origem + rasgo central, tudo por restrições
    let mut sk = SketchData::default();
    {
        let mut al = || d.alloc();
        let o = origin_point(&mut sk, &mut al);
        let (p, l) = sk.add_rect(&mut al, [-60., -36.], [60., 36.], false);
        let diag = sk.add_curve(&mut al, Curve::Line { a: p[0], b: p[2] }, true);
        sk.add_constraint(&mut al, Kind::Midpoint(o, diag));
        sk.add_constraint(&mut al, Kind::Length(l[0], s("L")));
        sk.add_constraint(&mut al, Kind::Length(l[1], s("W")));
        let (c1, c2, arc) = sk.add_slot(&mut al, [-20., 0.], [20., 0.], 6.);
        let axis = sk.curves.last().unwrap().id;
        sk.add_constraint(&mut al, Kind::Midpoint(o, axis));
        sk.add_constraint(&mut al, Kind::HorizontalPts(c1, c2));
        sk.add_constraint(&mut al, Kind::Distance(c1, c2, s("L/3")));
        sk.add_constraint(&mut al, Kind::Radius(arc, s("6")));
    }
    let base = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data: sk }));
    d.feature_mut(base).unwrap().name = s("Sketch base");
    let ex = d.add_feature(FeatureKind::Extrude(Extrude { sketch: base, op: BoolOp::Add, extent: Extent::Blind(s("esp")), reverse: false, taper: s("0"), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
    d.feature_mut(ex).unwrap().name = s("Placa");

    let topo = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "esp");
    d.references.last_mut().unwrap().name = s("PlanoTopo");
    // centros dos furos: quatro pontos cotados a partir da origem
    let mut sk = SketchData::default();
    {
        let mut al = || d.alloc();
        let o = origin_point(&mut sk, &mut al);
        for (sx, sy) in [(-1., -1.), (1., -1.), (1., 1.), (-1., 1.)] {
            let p = sk.add_point(&mut al, sx * 48., sy * 24.);
            sk.add_curve(&mut al, Curve::Point { p }, false);
            sk.add_constraint(&mut al, Kind::DistanceX(o, p, s("L/2-margem")));
            sk.add_constraint(&mut al, Kind::DistanceY(o, p, s("W/2-margem")));
        }
    }
    let skf = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Datum(topo), data: sk }));
    d.feature_mut(skf).unwrap().name = s("Centros dos furos");
    // sketches consumidos por comandos ficam ocultos, como nos CADs usuais
    d.feature_mut(base).unwrap().hidden = true;
    d.feature_mut(skf).unwrap().hidden = true;
    let m6 = hardware::metric("M6").unwrap();
    let furo = d.add_feature(FeatureKind::Hole(Hole {
        sketch: skf,
        kind: HoleKind::Rebaixado { diameter: m6.cbore_diameter.to_string(), depth: m6.cbore_depth.to_string() },
        diameter: m6.clearance[1].to_string(),
        depth: Extent::ThroughAll,
        thread: None,
        thread_modeled: false,
        standard: Some(s("M6 passagem normal")),
        points: vec![], dir: None, reverse: false, tip: None, taper: "0".into(), left_hand: false, extra: Default::default(),
    }));
    d.feature_mut(furo).unwrap().name = s("Furos M6");
    let fol = d.add_folder("Acabamentos");
    let fil = d.add_feature(FeatureKind::Fillet(EdgeOp { value: s("1"), edges: EdgeSel::AllOfBody, mode: EdgeMode::Constant, prop: Default::default(), keys: vec![], extra: Default::default() }));
    d.feature_mut(fil).unwrap().folder = Some(fol);
    d
}

#[cfg(test)]
mod chapa_estilo_tests;

#[cfg(test)]
mod chapa_laco_tests;

#[cfg(test)]
mod chapa_b35_tests;
#[cfg(test)]
mod chapa_fecha_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estrutural_quadro_cantos_placa_reforco() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let near = |a: f64, b: f64, tol: f64| (a - b).abs() <= tol * b.abs().max(1.);
        let s = |x: &str| x.to_string();
        let quadro = |corner: CornerMode| {
            let mut d = Document::new("quadro");
            let pts = [[0., 0., 0.], [400., 0., 0.], [400., 300., 0.], [0., 300., 0.]];
            let mut paths = vec![];
            for i in 0..4 {
                let l = d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed(pts[i]), b: PointRef::Fixed(pts[(i + 1) % 4]), start: s("0"), end: s("0") });
                paths.push(CurveRef::Ref(l));
            }
            d.add_feature(FeatureKind::StructMember(StructMember {
                paths,
                family: s("Tubo quadrado"),
                designation: s("40x3"),
                kind: SectionKind::TuboQuadrado,
                dims: vec![s("40"), s("3")],
                anchor: 0,
                angle: s("0"),
                off_x: s("0"),
                off_y: s("0"),
                ext1: s("0"),
                ext2: s("0"),
                corner,
                gap: s("0"),
                joints: vec![],
                material: String::new(),
                mirror: false,
            }));
            d
        };
        let a = 40. * 40. - 34. * 34.;
        // meia-esquadria conserva o comprimento da linha de centro
        let d = quadro(CornerMode::MeiaEsquadria);
        let v = vol(&d);
        assert!(near(v, a * 1400., 1e-3), "meia-esquadria {v}");
        // encaixe/aparar: os posteriores param nos anteriores (menos volume que as barras inteiras + sobras)
        for c in [CornerMode::Aparar, CornerMode::Encaixe] {
            let v2 = vol(&quadro(c));
            assert!(v2 > a * 1300. && v2 < a * 1500., "{c:?} {v2}");
        }
        let cl = cut_list(&d);
        assert_eq!(cl.iter().map(|r| r.count).sum::<usize>(), 4);
        assert_eq!(cl.len(), 2, "duas medidas: 400 e 300");
        // placa de extremidade e reforço num L aberto
        let mut d = Document::new("l");
        let l1 = d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed([0., 0., 0.]), b: PointRef::Fixed([0., 0., 500.]), start: s("0"), end: s("0") });
        let l2 = d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed([0., 0., 500.]), b: PointRef::Fixed([300., 0., 500.]), start: s("0"), end: s("0") });
        d.add_feature(FeatureKind::StructMember(StructMember {
            paths: vec![CurveRef::Ref(l1), CurveRef::Ref(l2)],
            family: s("Tubo quadrado"),
            designation: s("40x3"),
            kind: SectionKind::TuboQuadrado,
            dims: vec![s("40"), s("3")],
            anchor: 0,
            angle: s("0"),
            off_x: s("0"),
            off_y: s("0"),
            ext1: s("0"),
            ext2: s("0"),
            corner: CornerMode::MeiaEsquadria,
            gap: s("0"),
            joints: vec![],
            material: String::new(),
            mirror: false,
        }));
        let v0 = vol(&d);
        d.add_feature(FeatureKind::StructPlate(StructPlate::nova([0., 0., 0.], "8", "5")));
        let v1 = vol(&d);
        assert!(near(v1 - v0, 50. * 50. * 8., 1e-3), "placa {}", v1 - v0);
        d.add_feature(FeatureKind::StructGusset(StructGusset::novo([0., 0., 500.], "60", "6", "0")));
        let v2 = vol(&d);
        assert!(near(v2 - v1, 0.5 * 60. * 60. * 6., 1e-3), "reforço {}", v2 - v1);
    }

    /// K-09 (B387): travessa com canto Encaixe cuja ponta cai no MEIO do hospedeiro (junta em T) também
    /// é recortada; antes só a ponta coincidente com uma ponta do hospedeiro era tratada.
    #[test]
    fn encaixe_em_t_no_meio_da_longarina_recorta_a_ponta() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let s = |x: &str| x.to_string();
        let mk = |d: &mut Document, a: V3, b: V3, corner: CornerMode| {
            let l = d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed(a), b: PointRef::Fixed(b), start: s("0"), end: s("0") });
            d.add_feature(FeatureKind::StructMember(StructMember {
                paths: vec![CurveRef::Ref(l)],
                family: s("Tubo quadrado"),
                designation: s("40x3"),
                kind: SectionKind::TuboQuadrado,
                dims: vec![s("40"), s("3")],
                anchor: 0,
                angle: s("0"),
                off_x: s("0"),
                off_y: s("0"),
                ext1: s("0"),
                ext2: s("0"),
                corner,
                gap: s("0"),
                joints: vec![],
                material: String::new(),
                mirror: false,
            }));
        };
        let pr = |a: f64, b: f64, tol: f64| (a - b).abs() <= tol * b.abs().max(1.);
        let total = |c: CornerMode| {
            let mut d = Document::new("t");
            mk(&mut d, [0., 0., 0.], [1000., 0., 0.], CornerMode::MeiaEsquadria);
            mk(&mut d, [500., 0., 0.], [500., 300., 0.], c);
            let r = recompute(&d);
            r.bodies.iter().map(|b| b.mass.volume).sum::<f64>()
        };
        let a = 40. * 40. - 34. * 34.;
        let sem = total(CornerMode::Nenhum);
        assert!(pr(sem, a * 1300., 1e-3), "sem tratamento a barra entra no hospedeiro: {sem}");
        for c in [CornerMode::Encaixe, CornerMode::Aparar] {
            let com = total(c);
            assert!(sem - com > a * 15., "{c:?}: a ponta devia parar na face do hospedeiro ({com} contra {sem})");
        }
        // meia-esquadria não vale em T: continua como estava
        assert!(pr(total(CornerMode::MeiaEsquadria), sem, 1e-6));
    }

    #[test]
    fn estrutura_agrupa_pecas_iguais() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let s = |x: &str| x.to_string();
        let mut d = Document::new("quadro");
        let pts = [[0., 0., 0.], [400., 0., 0.], [400., 300., 0.], [0., 300., 0.]];
        let mut paths = vec![];
        for i in 0..4 {
            let l = d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed(pts[i]), b: PointRef::Fixed(pts[(i + 1) % 4]), start: s("0"), end: s("0") });
            paths.push(CurveRef::Ref(l));
        }
        let f = d.add_feature(FeatureKind::StructMember(StructMember {
            paths,
            family: s("Tubo quadrado"),
            designation: s("40x3"),
            kind: SectionKind::TuboQuadrado,
            dims: vec![s("40"), s("3")],
            anchor: 0,
            angle: s("0"),
            off_x: s("0"),
            off_y: s("0"),
            ext1: s("0"),
            ext2: s("0"),
            corner: CornerMode::MeiaEsquadria,
            gap: s("0"),
            joints: vec![],
            material: String::new(),
            mirror: false,
        }));
        // um sólido por trecho, no referencial local (Z ao longo do membro, seção centrada)
        let ms = structural_members(&d);
        assert_eq!(ms.len(), 4);
        for (m, len) in ms.iter().zip([400., 300., 400., 300.]) {
            let s = &m.signature;
            assert!((s.length_mm() - len).abs() < 1e-9, "{}", s.length_mm());
            // meia-esquadria: as faces de fora passam meia seção das pontas da linha de centro
            assert!((s.bbox[2] as f64 / 10. + 20.).abs() < 0.5 && (s.bbox[5] as f64 / 10. - len - 20.).abs() < 0.5, "{:?}", s.bbox);
            assert!(s.bbox[0] >= -210 && s.bbox[1] >= -210 && s.bbox[3] <= 210 && s.bbox[4] <= 210, "{:?}", s.bbox);
            // a meia-esquadria conserva o volume da barra reta
            let v = (40. * 40. - 34. * 34.) * len;
            assert!((m.shape.mass().volume - v).abs() < 1e-3 * v, "volume local {}", m.shape.mass().volume);
        }
        let gs = structural_groups(&d);
        assert_eq!(gs.len(), 2, "quatro membros iguais dois a dois");
        assert_eq!((gs[0].item, gs[1].item), (1, 2));
        assert!((gs[0].signature.length_mm() - 300.).abs() < 1e-9 && (gs[1].signature.length_mm() - 400.).abs() < 1e-9);
        assert!(gs.iter().all(|g| g.members.len() == 2 && g.members.iter().all(|m| m.feature == f)));
        let cl = cut_list(&d);
        assert_eq!(cl.iter().map(|r| (r.item, r.count)).collect::<Vec<_>>(), vec![(1, 2), (2, 2)]);
        assert_eq!(cl[0].members, vec![(f, 1), (f, 3)]);
        // outro tratamento num canto: os dois membros que ele toca deixam de ser iguais aos opostos
        if let Some(FeatureKind::StructMember(m)) = d.feature_mut(f).map(|f| &mut f.kind) {
            m.joints.push(StructJoint { mine: 1, other: f, other_path: 0, mode: CornerMode::Aparar, first: false, gap: None, bend: None });
        }
        let gs2 = structural_groups(&d);
        assert!(gs2.len() > 2, "pontas diferentes separam os grupos: {}", gs2.len());
        assert!(cut_list(&d).len() > 2);
        assert_eq!(gs2.iter().map(|g| g.members.len()).sum::<usize>(), 4);
        // placa e reforço entram como itens próprios
        let mut d = Document::new("l");
        let l1 = d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed([0., 0., 0.]), b: PointRef::Fixed([0., 0., 500.]), start: s("0"), end: s("0") });
        let l2 = d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed([0., 0., 500.]), b: PointRef::Fixed([300., 0., 500.]), start: s("0"), end: s("0") });
        d.add_feature(FeatureKind::StructMember(StructMember {
            paths: vec![CurveRef::Ref(l1), CurveRef::Ref(l2)],
            family: s("Tubo quadrado"),
            designation: s("40x3"),
            kind: SectionKind::TuboQuadrado,
            dims: vec![s("40"), s("3")],
            anchor: 0,
            angle: s("0"),
            off_x: s("0"),
            off_y: s("0"),
            ext1: s("0"),
            ext2: s("0"),
            corner: CornerMode::MeiaEsquadria,
            gap: s("0"),
            joints: vec![],
            material: String::new(),
            mirror: false,
        }));
        d.add_feature(FeatureKind::StructPlate(StructPlate::nova([0., 0., 0.], "8", "5")));
        d.add_feature(FeatureKind::StructGusset(StructGusset::novo([0., 0., 500.], "60", "6", "0")));
        let gs = structural_groups(&d);
        let kinds: Vec<StructItem> = gs.iter().map(|g| g.signature.kind).collect();
        assert_eq!(kinds, vec![StructItem::Member, StructItem::Member, StructItem::Plate, StructItem::Gusset]);
        assert!((gs[2].shape.mass().volume - 50. * 50. * 8.).abs() < 1., "placa");
        assert!((gs[3].shape.mass().volume - 0.5 * 60. * 60. * 6.).abs() < 1., "reforço");
        assert_eq!(cut_list(&d).len(), 2);
    }

    #[test]
    fn sketch_com_geometria_externa_associativa() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        use fenkra_core::sketch::LinkSrc;
        let s = |x: &str| x.to_string();
        let mut d = Document::new("assoc");
        let lado = d.add_param("L", "40", Unit::Comprimento).unwrap();
        let _ = lado;
        // linha de referência de (0,0,0) a (L,0,0)
        let la = d.add_geometry(RefKind::LinePoints { a: PointRef::Origin, b: PointRef::Fixed([40., 0., 0.]), start: s("0"), end: s("0") });
        let mut data = SketchData::default();
        let mut n = 5_000_000u64;
        let mut al = || {
            n += 1;
            Id(n)
        };
        let link = data.add_link(&mut al, LinkSrc::Ref(la), false, &[[0., 0.], [40., 0.]], true).unwrap();
        let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
        // a referência muda: o ponto final no sketch acompanha
        if let Some(r) = d.references.iter_mut().find(|r| r.id == la) {
            r.kind = RefKind::LinePoints { a: PointRef::Origin, b: PointRef::Fixed([70., 0., 0.]), start: s("0"), end: s("0") };
        }
        let r = recompute(&d);
        assert!(matches!(r.status[&sk], Status::Ok | Status::Warning(_)), "{:?}", r.status[&sk]);
        let v = r.sketches.iter().find(|v| v.feature == sk).unwrap();
        let xmax = v.construction.iter().map(|p| p[0]).fold(f32::MIN, f32::max);
        assert!((xmax - 70.).abs() < 1e-3, "a linha projetada foi para 70: {xmax}");
        let _ = link;
    }

    /// Elipse de semieixos 30 e 10 em XY, extrudada 20: a curva exata atravessa o sketch e o kernel.
    fn cilindro_eliptico() -> (Document, Id) {
        use fenkra_core::projecao::Exata;
        use fenkra_core::sketch::LinkSrc;
        let mut d = Document::new("elipse");
        let mut data = SketchData::default();
        let mut n = 9_000_000u64;
        let mut al = || {
            n += 1;
            Id(n)
        };
        let forma = Exata::Elipse { c: [5., 0.], maior: [30., 0.], menor: 10., t0: 0., t1: std::f64::consts::TAU };
        data.add_link_exata(&mut al, LinkSrc::Vertex { pos: [0.; 3] }, false, &forma, false);
        data.links.clear();
        let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
        d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind("10".into()), reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
        (d, sk)
    }

    /// Sketch em `plano` com um vínculo exato à aresta de `mid`.
    fn sketch_ligado(d: &mut Document, plano: OriginPlane, mid: [f64; 3], base: u64, inter: bool) -> (Id, Id) {
        use fenkra_core::projecao::Exata;
        use fenkra_core::sketch::LinkSrc;
        let mut data = SketchData::default();
        let mut n = base;
        let mut al = || {
            n += 1;
            Id(n)
        };
        let l = if inter {
            data.add_link_points(&mut al, LinkSrc::Edge { mid }, &[[0., 0.], [1., 0.]], true).unwrap()
        } else {
            data.add_link_exata(&mut al, LinkSrc::Edge { mid }, false, &Exata::Elipse { c: [0., 0.], maior: [2., 0.], menor: 1., t0: 0., t1: std::f64::consts::TAU }, true)
        };
        (d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(plano), data })), l)
    }

    #[test]
    fn elipse_exata_vira_solido_e_aresta_conhecida() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (d, sk) = cilindro_eliptico();
        let r = recompute(&d);
        assert!(r.status.values().all(|s| matches!(s, Status::Ok)), "{:?}", r.status);
        let v = r.bodies[0].mass.volume;
        let h = 10.;
        assert!((v - std::f64::consts::PI * 30. * 10. * h).abs() / v < 1e-6, "volume {v}");
        let f = (0..r.bodies[0].mesh.edge_mids.len() / 3).filter_map(|e| r.bodies[0].mesh.edge_formula(e)).find(|f| matches!(f, fenkra_kernel::EdgeFormula::Conic { .. }));
        let Some(fenkra_kernel::EdgeFormula::Conic { u, v, .. }) = f else { panic!("sem cônica: {f:?}") };
        let n = |a: [f64; 3]| (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
        assert!((n(u) - 30.).abs() < 1e-9 && (n(v) - 10.).abs() < 1e-9, "{u:?} {v:?}");
        let _ = sk;
    }

    #[test]
    fn projecao_de_aresta_eliptica_e_exata_e_acompanha() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        use fenkra_core::sketch::Curve;
        let (mut d, _) = cilindro_eliptico();
        // a aresta de cima: projetada em XY é a própria elipse, com o semieixo certo
        let (sk, _) = sketch_ligado(&mut d, OriginPlane::XY, [-25., 0., 10.], 9_100_000, false);
        let _ = recompute(&d);
        let r = recompute(&d);
        assert!(matches!(r.status[&sk], Status::Ok), "{:?}", r.status[&sk]);
        let v = r.sketches.iter().find(|v| v.feature == sk).unwrap();
        assert!(v.link_quebrados.is_empty());
        let (_, m, menor, t0, t1) = v.link_e[0];
        assert!((m[0].hypot(m[1]) - 30.).abs() < 1e-9 && (menor - 10.).abs() < 1e-9, "{m:?} {menor}");
        assert!((t1 - t0 - std::f64::consts::TAU).abs() < 1e-9);
        // a elipse de origem cresce no eixo menor: o vínculo acompanha sem escolher de novo
        let id = d.features[0].id;
        if let Some(FeatureKind::Sketch(s)) = d.feature_mut(id).map(|f| &mut f.kind) {
            if let Curve::Ellipse { minor, .. } = &mut s.data.curves[0].geom {
                *minor = 12.;
            }
        }
        let r = recompute(&d);
        let v = r.sketches.iter().find(|v| v.feature == sk).unwrap();
        assert!((v.link_e[0].2 - 12.).abs() < 1e-9, "{:?}", v.link_e);
    }

    #[test]
    fn interseccao_de_aresta_da_todos_os_pontos_exatos() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        // a elipse de baixo atravessa o plano XZ nas pontas do eixo maior: x = 5 ± 30
        let (mut d, _) = cilindro_eliptico();
        let (sk, _) = sketch_ligado(&mut d, OriginPlane::XZ, [-25., 0., 0.], 9_200_000, true);
        let r = recompute(&d);
        let v = r.sketches.iter().find(|v| v.feature == sk).unwrap();
        assert!(v.link_quebrados.is_empty(), "{:?}", r.status[&sk]);
        let mut xs: Vec<f64> = v.link_pts.iter().map(|p| p.1[0]).collect();
        xs.sort_by(f64::total_cmp);
        assert_eq!(xs.len(), 2);
        assert!((xs[0].abs() - 25.).abs() < 1e-9 || (xs[0].abs() - 35.).abs() < 1e-9, "{xs:?}");
        assert!((xs[1] - xs[0] - 60.).abs() < 1e-9, "{xs:?}");
    }

    #[test]
    fn vinculo_sem_origem_quebra_e_isolado_nao_avisa() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        use fenkra_core::sketch::LinkPolicy;
        let (mut d, _) = cilindro_eliptico();
        // nenhuma aresta perto: a origem sumiu
        let (sk, _) = sketch_ligado(&mut d, OriginPlane::XY, [500., 500., 500.], 9_300_000, false);
        let r = recompute(&d);
        assert_eq!(r.sketches.iter().find(|v| v.feature == sk).unwrap().link_quebrados.len(), 1);
        assert!(matches!(r.status[&sk], Status::Warning(_)), "{:?}", r.status[&sk]);
        // isolado: guarda a origem, não procura por ela
        if let Some(FeatureKind::Sketch(s)) = d.feature_mut(sk).map(|f| &mut f.kind) {
            s.data.links[0].policy = LinkPolicy::Isolado;
        }
        let r = recompute(&d);
        assert!(r.sketches.iter().find(|v| v.feature == sk).unwrap().link_quebrados.is_empty());
        assert!(matches!(r.status[&sk], Status::Ok), "{:?}", r.status[&sk]);
        // voltou a associar: o estado reaparece, sem ninguém escolher a aresta de novo
        if let Some(FeatureKind::Sketch(s)) = d.feature_mut(sk).map(|f| &mut f.kind) {
            s.data.links[0].policy = LinkPolicy::Associativo;
            if let fenkra_core::sketch::LinkSrc::Edge { mid } = &mut s.data.links[0].source {
                *mid = [-25., 0., 10.];
            }
        }
        let r = recompute(&d);
        let v = r.sketches.iter().find(|v| v.feature == sk).unwrap();
        assert!(v.link_quebrados.is_empty() && !v.link_e.is_empty());
    }

    /// O que o editor faz depois do cálculo: leva ao documento o que mudou de estrutura.
    fn aplica_no_documento(d: &mut Document, sk: Id, r: &RecomputeResult) -> usize {
        let ops = r.sketches.iter().find(|v| v.feature == sk).unwrap().link_ops.clone();
        if let Some(FeatureKind::Sketch(s)) = d.feature_mut(sk).map(|f| &mut f.kind) {
            for op in &ops {
                s.data.aplica_vinculo(op);
            }
        }
        ops.len()
    }

    #[test]
    fn origem_que_muda_de_tipo_refaz_o_vinculo_sem_quebrar() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        use fenkra_core::projecao::Exata;
        use fenkra_core::sketch::{Curve, Kind, LinkShape, LinkSrc};
        let (mut d, _) = cilindro_eliptico();
        // o vínculo foi feito quando a aresta era uma linha; agora ela é a elipse de cima
        let mut data = SketchData::default();
        let mut n = 9_500_000u64;
        let mut al = || {
            n += 1;
            Id(n)
        };
        data.add_link_exata(&mut al, LinkSrc::Edge { mid: [-25., 0., 10.] }, false, &Exata::Reta([0., 0.], [10., 0.]), false);
        let LinkShape::Line { curve, .. } = data.links[0].shape else { panic!() };
        data.add_constraint(&mut al, Kind::Length(curve, "10".into()));
        let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
        let r = recompute(&d);
        let v = r.sketches.iter().find(|v| v.feature == sk).unwrap();
        assert!(v.link_quebrados.is_empty(), "{:?}", r.status[&sk]);
        assert!(!v.link_e.is_empty(), "o cálculo já enxerga a elipse");
        // a cota de comprimento não serve à elipse, e isso é dito
        assert!(matches!(&r.status[&sk], Status::Warning(m) if m.contains("elipse")), "{:?}", r.status[&sk]);
        assert_eq!(aplica_no_documento(&mut d, sk, &r), 1);
        let Some(FeatureKind::Sketch(s)) = d.feature(sk).map(|f| &f.kind) else { panic!() };
        assert!(matches!(s.data.curve(curve).unwrap().geom, Curve::Ellipse { .. }), "a mesma curva, com o mesmo id");
        assert!(s.data.constraints.is_empty());
        let r = recompute(&d);
        assert!(matches!(r.status[&sk], Status::Ok), "{:?}", r.status[&sk]);
        assert_eq!(aplica_no_documento(&mut d, sk, &r), 0, "em dia, nada mais muda");
    }

    #[test]
    fn perfil_com_vinculo_que_mudou_de_tipo_extruda_a_forma_nova() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        use fenkra_core::projecao::Exata;
        use fenkra_core::sketch::LinkSrc;
        let (mut d, _) = cilindro_eliptico();
        let mut data = SketchData::default();
        let mut n = 9_510_000u64;
        let mut al = || {
            n += 1;
            Id(n)
        };
        data.add_link_exata(&mut al, LinkSrc::Edge { mid: [-25., 0., 10.] }, false, &Exata::Reta([0., 0.], [10., 0.]), false);
        let topo = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "10");
        let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Datum(topo), data }));
        let ex = d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind("5".into()), reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
        // sem tocar no documento: quem calcula sozinho (linha de comando, testes) também vê a elipse
        let r = recompute(&d);
        assert!(matches!(r.status[&ex], Status::Ok), "{:?}", r.status[&ex]);
        // cilindro elíptico de semieixos 30 e 10, altura 10 + 5
        let v = r.bodies.last().unwrap().mass.volume;
        let esperado = std::f64::consts::PI * 30. * 10. * 15.;
        assert!((v - esperado).abs() / esperado < 1e-3, "{v} contra {esperado}");
    }

    #[test]
    fn interseccao_que_passa_a_cortar_mais_vezes_ganha_o_ponto() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        use fenkra_core::sketch::{LinkShape, LinkSrc};
        let (mut d, _) = cilindro_eliptico();
        // só uma travessia guardada; a elipse de baixo corta o plano XZ em duas
        let mut data = SketchData::default();
        let mut n = 9_520_000u64;
        let mut al = || {
            n += 1;
            Id(n)
        };
        data.add_link_points(&mut al, LinkSrc::Edge { mid: [-25., 0., 0.] }, &[[-25., 0.]], true).unwrap();
        let LinkShape::Poly { points, .. } = data.links[0].shape.clone() else { panic!() };
        let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XZ), data }));
        let r = recompute(&d);
        let v = r.sketches.iter().find(|v| v.feature == sk).unwrap();
        assert_eq!(v.link_pts.len(), 2, "{:?}", r.status[&sk]);
        // o ponto que existia segue a travessia dele, e o novo nasce na outra
        let q = v.link_pts.iter().find(|p| p.0 == points[0]).unwrap().1;
        assert!((q[0] + 25.).abs() < 1e-9, "{q:?}");
        let novo = v.link_pts.iter().find(|p| p.0 != points[0]).unwrap().1;
        assert!((novo[0] - 35.).abs() < 1e-9, "{novo:?}");
        assert_eq!(aplica_no_documento(&mut d, sk, &r), 1);
        let r = recompute(&d);
        assert_eq!(aplica_no_documento(&mut d, sk, &r), 0);
        assert!(matches!(r.status[&sk], Status::Ok), "{:?}", r.status[&sk]);
    }

    #[test]
    fn sketch_com_origem_e_orientacao_proprias() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        use fenkra_core::sketch::SketchPos;
        let caixa = |pos: SketchPos| {
            let mut d = Document::new("pos");
            let mut data = SketchData::default();
            let mut n = 9_400_000u64;
            let mut al = || {
                n += 1;
                Id(n)
            };
            data.add_rect(&mut al, [0., 0.], [40., 10.], false);
            data.pos = pos;
            let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
            d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind("10".into()), reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
            let r = recompute(&d);
            assert!(r.status.values().all(|s| matches!(s, Status::Ok)), "{:?}", r.status);
            assert!((r.bodies[0].mass.volume - 4000.).abs() < 1e-6);
            r.bodies[0].mesh.positions.chunks_exact(3).fold(([f32::MAX; 3], [f32::MIN; 3]), |(mut lo, mut hi), p| {
                for i in 0..3 {
                    lo[i] = lo[i].min(p[i]);
                    hi[i] = hi[i].max(p[i]);
                }
                (lo, hi)
            })
        };
        let perto = |a: [f32; 3], b: [f32; 3]| (0..3).all(|i| (a[i] - b[i]).abs() < 1e-3);
        let (lo, hi) = caixa(SketchPos::default());
        assert!(perto(lo, [0., 0., 0.]) && perto(hi, [40., 10., 10.]), "{lo:?} {hi:?}");
        // origem própria: tudo anda junto no plano
        let (lo, hi) = caixa(SketchPos { origem: [100., -50.], ..Default::default() });
        assert!(perto(lo, [100., -50., 0.]) && perto(hi, [140., -40., 10.]), "{lo:?} {hi:?}");
        // inverter H e V juntos é meia volta, com a mesma normal
        let (lo, hi) = caixa(SketchPos { inverter_h: true, inverter_v: true, ..Default::default() });
        assert!(perto(lo, [-40., -10., 0.]) && perto(hi, [0., 0., 10.]), "{lo:?} {hi:?}");
        // inverter H sozinho vê o sketch por trás: a normal vira e a extrusão desce
        let (lo, hi) = caixa(SketchPos { inverter_h: true, ..Default::default() });
        assert!(perto(lo, [-40., 0., -10.]) && perto(hi, [0., 10., 0.]), "{lo:?} {hi:?}");
        // trocar H e V: o lado de 40 passa para y, e a normal também vira
        let (lo, hi) = caixa(SketchPos { trocar: true, ..Default::default() });
        assert!(perto(lo, [0., 0., -10.]) && perto(hi, [10., 40., 0.]), "{lo:?} {hi:?}");
    }

    /// Quarto de disco com abas nas duas retas: dobrado e planificado (com recorte e filete
    /// feitos no dobrado, que vão junto para o planificado).
    fn quarto_de_disco(flat: bool) -> (Document, Vec<Id>) {
        let s = |x: &str| x.to_string();
        let mut d = Document::new("chapa");
        d.sheet = Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), flat: false, ..Default::default() });
        let mut data = SketchData::default();
        let mut n = 7_000_000u64;
        let mut al = || {
            n += 1;
            Id(n)
        };
        let o = data.add_point(&mut al, 0., 0.);
        let a = data.add_point(&mut al, 100., 0.);
        let b = data.add_point(&mut al, 0., 100.);
        data.add_curve(&mut al, fenkra_core::sketch::Curve::Line { a: o, b: a }, false);
        data.add_curve(&mut al, fenkra_core::sketch::Curve::Arc { c: o, s: a, e: b }, false);
        data.add_curve(&mut al, fenkra_core::sketch::Curve::Line { a: b, b: o }, false);
        let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
        let mut ids = vec![d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }))];
        // aresta de cima das duas retas (ponto médio com o erro de float da malha)
        for e in [[50.00001, 0., 2.], [0., 50.00001, 2.]] {
            ids.push(d.add_feature(FeatureKind::SmFlange(SmFlange { edge: e, length: s("30"), angle: s("90"), radius: None, off1: s("5"), off2: s("5"), flip: false, relief: None, relief_kind: 0, side: 0, miter: false, min_gap: true, edge_key: None })));
        }
        // furo retangular na aba do lado y = 0 (feito no dobrado: a aba sobe em z)
        let cut = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XZ), [40., 15.], [60., 25.]);
        ids.push(d.add_feature(FeatureKind::Extrude(Extrude { sketch: cut, op: BoolOp::Cut, extent: Extent::ThroughAll, reverse: true, taper: s("0"), thin: None, dir: None, thin_side: 0, ends: Default::default() })));
        if let Some(sh) = d.sheet.as_mut() {
            sh.flat = flat;
        }
        (d, ids)
    }

    #[test]
    fn membro_por_polilinha_vira_trechos_retos() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let s = |x: &str| x.to_string();
        let mut d = Document::new("poli");
        let pl = d.add_geometry(RefKind::Polyline { points: vec![PointRef::Fixed([0., 0., 0.]), PointRef::Fixed([100., 0., 0.]), PointRef::Fixed([100., 0., 80.])], closed: false });
        let m = d.add_feature(FeatureKind::StructMember(StructMember {
            paths: vec![CurveRef::Ref(pl)],
            family: s("Tubo quadrado"),
            designation: s("40x3"),
            kind: SectionKind::TuboQuadrado,
            dims: vec![s("40"), s("3")],
            anchor: 0,
            angle: s("0"),
            off_x: s("0"),
            off_y: s("0"),
            ext1: s("0"),
            ext2: s("0"),
            corner: CornerMode::MeiaEsquadria,
            gap: s("0"),
            joints: vec![],
            material: String::new(),
            mirror: false,
        }));
        let r = recompute(&d);
        assert!(matches!(r.status[&m], Status::Ok), "{:?}", r.status[&m]);
        let v: f64 = r.bodies.iter().map(|b| b.mass.volume).sum();
        let a = 40. * 40. - 34. * 34.;
        assert!((v - a * 180.).abs() < 1e-3 * a * 180., "dois trechos retos em meia-esquadria: {v} × {}", a * 180.);
        assert_eq!(cut_list(&d).iter().map(|r| r.count).sum::<usize>(), 2);
    }

    /// K-02 (B380): polilinha ligada (aqui, isolada: mesma cópia de vértices) com `CurveRef::Segment`
    /// como caminho do membro. Antes o vértice 1 virava o fim da curva e o trecho saía de comprimento zero.
    #[test]
    fn trecho_de_polilinha_ligada_vira_membro() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let s = |x: &str| x.to_string();
        let mut d = Document::new("ligada");
        let pl = d.add_geometry(RefKind::Isolated { geo: LinkedGeo::Curve(vec![[0., 0., 0.], [100., 0., 0.], [100., 0., 80.]]) });
        let mk = |d: &mut Document, index: usize| {
            d.add_feature(FeatureKind::StructMember(StructMember {
                paths: vec![CurveRef::Segment { curve: pl, index }],
                family: s("Tubo quadrado"),
                designation: s("40x3"),
                kind: SectionKind::TuboQuadrado,
                dims: vec![s("40"), s("3")],
                anchor: 0,
                angle: s("0"),
                off_x: s("0"),
                off_y: s("0"),
                ext1: s("0"),
                ext2: s("0"),
                corner: CornerMode::Nenhum,
                gap: s("0"),
                joints: vec![],
                material: String::new(),
                mirror: false,
            }))
        };
        let (m0, m1, m2) = (mk(&mut d, 0), mk(&mut d, 1), mk(&mut d, 5));
        let r = recompute(&d);
        assert!(matches!(r.status[&m0], Status::Ok), "{:?}", r.status[&m0]);
        assert!(matches!(r.status[&m1], Status::Ok), "{:?}", r.status[&m1]);
        let a = 40. * 40. - 34. * 34.;
        let v: f64 = r.bodies.iter().map(|b| b.mass.volume).sum();
        assert!((v - a * 180.).abs() < 1e-3 * a * 180., "trechos de 100 e 80 mm: {v}");
        let len: Vec<f64> = { let mut l: Vec<f64> = cut_list(&d).iter().map(|c| c.length).collect(); l.sort_by(|a, b| a.total_cmp(b)); l };
        assert_eq!(len.len(), 2, "{len:?}");
        assert!((len[0] - 80.).abs() < 1e-6 && (len[1] - 100.).abs() < 1e-6, "{len:?}");
        // vértice que não existe: a mensagem diz qual
        match &r.status[&m2] {
            Status::Error(m) => assert!(m.contains("vértices"), "{m}"),
            o => panic!("esperava erro, veio {o:?}"),
        }
    }

    #[test]
    fn perfil_aberto_vira_parede_fina() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let s = |x: &str| x.to_string();
        let mk = |side: u8, thin: Option<&str>| {
            let mut d = Document::new("fino");
            // L aberto: (0,0) → (40,0) → (40,30)
            let sk = sk_lines(&mut d, PlaneRef::Origin(OriginPlane::XY), &[([0., 0.], [40., 0.]), ([40., 0.], [40., 30.])]);
            // as duas linhas precisam dividir o ponto do canto: coincidência
            if let Some(FeatureKind::Sketch(sk)) = d.feature_mut(sk).map(|f| &mut f.kind) {
                let ids: Vec<Id> = sk.data.points.iter().map(|p| p.id).collect();
                let mut n = 9_000_000u64;
                let mut al = || {
                    n += 1;
                    Id(n)
                };
                sk.data.add_constraint(&mut al, fenkra_core::sketch::Kind::Coincident(ids[1], ids[2]));
            }
            let e = d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind(s("10")), reverse: false, taper: s("0"), thin: thin.map(s), dir: None, thin_side: side, ends: Default::default() }));
            let r = recompute(&d);
            (r.status[&e].clone(), r.bodies.iter().map(|b| b.mass.volume).sum::<f64>())
        };
        // um lado (por dentro do L) e o outro (por fora): volumes conhecidos com cantos em esquadria
        let (st, v_in) = mk(0, Some("2"));
        assert!(matches!(st, Status::Ok), "{st:?}");
        assert!((v_in - (40. + 30. - 2.) * 2. * 10.).abs() < 1., "por dentro {v_in}");
        let (_, v_out) = mk(1, Some("2"));
        assert!((v_out - (40. + 30. + 2.) * 2. * 10.).abs() < 1., "por fora {v_out}");
        let (_, v_mid) = mk(2, Some("2"));
        assert!((v_mid - 70. * 2. * 10.).abs() < 1., "simétrica {v_mid}");
        // sem marcar parede fina: 1 mm e um aviso (não é erro)
        let (st, v1) = mk(0, None);
        assert!(matches!(st, Status::Warning(_)), "{st:?}");
        assert!(v1 > 0.);
    }

    #[test]
    fn chapa_planificada_leva_abas_recortes_e_filetes() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let vol = |d: &Document, ids: &[Id]| {
            let r = recompute(d);
            for i in ids {
                assert!(matches!(r.status[i], Status::Ok), "{:?}", r.status[i]);
            }
            r.bodies.iter().map(|b| b.mass.volume).sum::<f64>()
        };
        let (d0, ids0) = quarto_de_disco(false);
        let (d1, ids1) = quarto_de_disco(true);
        let (v0, v1) = (vol(&d0, &ids0), vol(&d1, &ids1));
        // o desenvolvido tem o mesmo volume (fator K conserva a fibra neutra; aqui só a ordem de grandeza)
        assert!((v0 - v1).abs() < 0.03 * v0, "dobrado {v0} × planificado {v1}");
        // sem o recorte, o volume é maior nos dois: o recorte valeu no planificado também
        let (mut d2, mut ids2) = quarto_de_disco(true);
        let cut = ids2.pop().unwrap();
        d2.features.retain(|f| f.id != cut);
        let v2 = vol(&d2, &ids2);
        let (mut d3, mut ids3) = quarto_de_disco(false);
        let cut3 = ids3.pop().unwrap();
        d3.features.retain(|f| f.id != cut3);
        let v3 = vol(&d3, &ids3);
        assert!((v3 - v0 - 400.).abs() < 1., "recorte no dobrado: {v3} − {v0}");
        // recorte feito no planificado (na aba deitada) aparece no dobrado
        let (mut d4, mut ids4) = quarto_de_disco(true);
        let c4 = ids4.pop().unwrap();
        d4.features.retain(|f| f.id != c4);
        let sk = sk_rect(&mut d4, PlaneRef::Origin(OriginPlane::XY), [40., -25.], [60., -15.]);
        ids4.push(d4.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Cut, extent: Extent::ThroughAll, reverse: true, taper: "0".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() })));
        assert!(d4.features.last().unwrap().flat_space);
        let v4_flat = vol(&d4, &ids4);
        assert!((v2 - v4_flat - 400.).abs() < 1., "recorte no planificado: {v2} − {v4_flat}");
        d4.sheet.as_mut().unwrap().flat = false;
        let v4_fold = vol(&d4, &ids4);
        assert!((v3 - v4_fold - 400.).abs() < 1., "o recorte do planificado vai para o dobrado: {v3} − {v4_fold}");
        // filete no canto da aba (aresta da espessura), feito no dobrado: arredonda no planificado também
        let (mut d5, ids5) = quarto_de_disco(false);
        let fil = d5.add_feature(FeatureKind::Fillet(EdgeOp { value: "5".into(), edges: EdgeSel::Edges(vec![[95., -3., 34.]]), mode: EdgeMode::Constant, prop: EdgeProp::Tangente, keys: vec![], extra: Default::default() }));
        let mut all5 = ids5.clone();
        all5.push(fil);
        let f_fold = vol(&d5, &all5);
        let corte = 25. * (1. - std::f64::consts::PI / 4.) * 2.;
        assert!((v0 - f_fold - corte).abs() < 1., "filete no dobrado: {v0} − {f_fold}");
        d5.sheet.as_mut().unwrap().flat = true;
        let f_flat = vol(&d5, &all5);
        assert!((v1 - f_flat - corte).abs() < 1., "o filete vai para o planificado: {v1} − {f_flat}");
        assert!(v2 - v1 > 20. * 10. * 2. * 0.9, "o recorte (20×10×2) precisa aparecer no planificado: {v2} − {v1}");
    }

    #[test]
    fn superficies_varredura_avancada_offset_filete_conectar_dividir() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let s = |x: &str| x.to_string();
        let area = |r: &RecomputeResult, id: Id| -> f64 {
            assert!(matches!(r.status[&id], Status::Ok | Status::Warning(_)), "{:?}", r.status[&id]);
            let m = &r.surfaces.iter().find(|v| v.id == id).expect("superfície").mesh;
            let p = |i: u32| [m.positions[i as usize * 3] as f64, m.positions[i as usize * 3 + 1] as f64, m.positions[i as usize * 3 + 2] as f64];
            m.indices.chunks_exact(3).map(|t| geo3d::norm(fenkra_kernel::cross(geo3d::sub(p(t[1]), p(t[0])), geo3d::sub(p(t[2]), p(t[0])))) / 2.).sum()
        };
        let line = |d: &mut Document, a: [f64; 3], b: [f64; 3]| CurveRef::Ref(d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed(a), b: PointRef::Fixed(b), start: s("0"), end: s("0") }));
        let mut d = Document::new("sup");
        let g1 = line(&mut d, [0., 0., 0.], [0., 0., 100.]);
        let g2 = line(&mut d, [20., 0., 0.], [20., 0., 100.]);
        let g3 = line(&mut d, [30., 0., 0.], [50., 0., 100.]);
        let sw = |kind: SweepKind, sol: usize| RefKind::SurfSweepX { kind, spine: None, solution: sol, smooth: false, sections: s("12") };
        let lin = d.add_geometry(sw(SweepKind::Linear { guide1: g1.clone(), guide2: g2.clone() }, 0));
        let half = d.add_geometry(sw(SweepKind::CircleRadius { guide1: g1.clone(), guide2: g2.clone(), radius: s("10") }, 0));
        let conic = d.add_geometry(sw(SweepKind::Conic { guide1: g1.clone(), guide2: g2.clone(), height: s("8"), rho: s("0.5") }, 1));
        // perfil explícito entre g1 e g3 (que se afasta): a seção acompanha e estica
        let prof = line(&mut d, [0., 0., 0.], [30., 0., 0.]);
        let expl = d.add_geometry(sw(SweepKind::Explicit { profile: prof, guide1: g1.clone(), guide2: Some(g3) }, 0));
        let r = recompute(&d);
        let near = |a: f64, b: f64, tol: f64| (a - b).abs() <= tol * b;
        assert!(near(area(&r, lin), 2000., 0.01), "linear {}", area(&r, lin));
        assert!(near(area(&r, half), std::f64::consts::PI * 10. * 100., 0.02), "meio cilindro {}", area(&r, half));
        assert!(area(&r, conic) > 2000., "cônica {}", area(&r, conic));
        assert!(near(area(&r, expl), 4000., 0.02), "explícito com 2 guias (trapézio 30→50) {}", area(&r, expl));
        // deslocada dos dois lados = duas cópias; filete entre duas superfícies que se cruzam
        let c_base = line(&mut d, [0., -50., 0.], [100., -50., 0.]);

        let base = d.add_geometry(RefKind::SurfExtrude { curve: c_base, dir: AxisRef::Origin(OriginAxis::Y), length: s("60"), length2: s("0"), until: None, until2: None });
        let c_wall = line(&mut d, [50., -60., -20.], [50., 20., -20.]);

        let wall = d.add_geometry(RefKind::SurfExtrude { curve: c_wall, dir: AxisRef::Origin(OriginAxis::Z), length: s("50"), length2: s("0"), until: None, until2: None });
        let off = d.add_geometry(RefKind::SurfOffset { surface: SurfRef::Ref(base), dist: s("5"), both: true, join: OffsetJoin::Automatico });
        let fil = d.add_geometry(RefKind::SurfFillet { a: SurfRef::Ref(base), b: SurfRef::Ref(wall), flip_a: false, flip_b: false, radius: s("5"), radius2: None });
        // conectar duas faixas coplanares separadas (bordas que se olham ficam ligadas)
        let c_s1 = line(&mut d, [0., 200., 0.], [40., 200., 0.]);

        let s1 = d.add_geometry(RefKind::SurfExtrude { curve: c_s1, dir: AxisRef::Origin(OriginAxis::Y), length: s("20"), length2: s("0"), until: None, until2: None });
        let c_s2 = line(&mut d, [0., 250., 0.], [40., 250., 0.]);

        let s2 = d.add_geometry(RefKind::SurfExtrude { curve: c_s2, dir: AxisRef::Origin(OriginAxis::Y), length: s("20"), length2: s("0"), until: None, until2: None });
        let con = d.add_geometry(RefKind::SurfConnect { a: SurfRef::Ref(s1), b: SurfRef::Ref(s2), cont: Continuity::G0, tension: s("1"), tension2: None, near: [None; 2] });
        // dividir pela curva que corta a faixa ao meio
        let cutc = line(&mut d, [20., 190., 0.], [20., 230., 0.]);
        let spl = d.add_geometry(RefKind::SurfSplit { surface: SurfRef::Ref(s1), tool: SurfRef::Plane(PlaneRef::Origin(OriginPlane::YZ)), flip: true, more: vec![], curve: Some(cutc), more_flip: vec![] });
        let r = recompute(&d);
        assert!(near(area(&r, off), 2. * 6000., 0.02), "offset dos dois lados {}", area(&r, off));
        let af = area(&r, fil);
        assert!(af > 1000. && af < 6000. + 4000., "filete entre superfícies {af}");
        assert!(near(area(&r, con), 40. * 30., 0.05), "conexão entre as bordas que se olham {}", area(&r, con));
        let asp = area(&r, spl);
        assert!(asp > 300. && asp < 700., "dividida pela curva: metade de 800 = {asp}");
    }

    #[test]
    fn tabela_de_furos_traz_forma_da_boca_e_tolerancia() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let s = |x: &str| x.to_string();
        let mut linhas = vec![];
        for (kind, standard) in [
            (HoleKind::Simples, Some(s("Ø6 H7"))),
            (HoleKind::Rebaixado { diameter: s("11"), depth: s("6.4") }, Some(s("M6 normal"))),
            (HoleKind::Escareado { diameter: s("12"), angle: s("90") }, None),
            (HoleKind::Contrafurado { diameter: s("11"), depth: s("5"), angle: s("118") }, None),
        ] {
            let mut d = Document::new("tf");
            let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [40., 40.]);
            d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind(s("10")), reverse: false, taper: s("0"), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
            d.add_feature(FeatureKind::Hole(Hole {
                sketch: Id(0),
                kind,
                diameter: s("6.6"),
                depth: Extent::ThroughAll,
                thread: None,
                thread_modeled: false,
                standard,
                points: vec![PointRef::Fixed([20., 20., 10.])],
                dir: None,
                reverse: false,
                tip: None,
                taper: s("0"),
                left_hand: false, extra: Default::default(),
            }));
            let t = hole_table(&d);
            assert_eq!(t.len(), 1);
            linhas.push(t[0].clone());
        }
        assert_eq!((linhas[0].forma, linhas[0].d_boca, linhas[0].prof_boca, linhas[0].tolerancia.as_str()), (FormaFuro::Reto, 0., 0., "H7"));
        assert_eq!((linhas[1].forma, linhas[1].d_boca, linhas[1].prof_boca, linhas[1].tolerancia.as_str()), (FormaFuro::Rebaixo, 11., 6.4, ""));
        // cone de 90 graus de Ø12 até Ø6,6: profundidade (12 - 6,6) / 2
        assert_eq!((linhas[2].forma, linhas[2].angulo), (FormaFuro::Escareado, 90.));
        assert!((linhas[2].prof_boca - 2.7).abs() < 1e-9, "{}", linhas[2].prof_boca);
        assert_eq!((linhas[3].forma, linhas[3].d_boca, linhas[3].prof_boca, linhas[3].angulo), (FormaFuro::Contrafuro, 11., 5., 118.));
        assert_eq!(tolerancia_do_texto("M8x1.25"), "");
        assert_eq!(tolerancia_do_texto("furo Ø10 JS9 liso"), "JS9");
        assert_eq!(tolerancia_do_texto("M6 normal"), "");
    }

    #[test]
    fn furo_projeta_centros_na_superficie() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let s = |x: &str| x.to_string();
        let base = || {
            let mut d = Document::new("fp");
            let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [40., 40.]);
            d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind(s("10")), reverse: false, taper: s("0"), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
            d
        };
        let hole = |pts: Vec<PointRef>| Hole {
            sketch: Id(0),
            kind: HoleKind::Rebaixado { diameter: s("10"), depth: s("3") },
            diameter: s("5"),
            depth: Extent::Blind(s("6")),
            thread: Some(s("M6x1")),
            thread_modeled: false,
            standard: None,
            points: pts,
            dir: None,
            reverse: false,
            tip: None,
            taper: s("0"),
            left_hand: false, extra: Default::default(),
        };
        let vol = |d: &Document| {
            let r = recompute(d);
            assert!(r.status.values().all(|x| !matches!(x, Status::Error(_))), "{:?}", r.status);
            r.bodies.iter().map(|b| b.mass.volume).sum::<f64>()
        };
        // ponto na face de cima e o mesmo ponto 30 mm acima: o mesmo furo (rosca + rebaixo juntos)
        let mut d1 = base();
        d1.add_feature(FeatureKind::Hole(hole(vec![PointRef::Fixed([20., 20., 10.])])));
        let mut d2 = base();
        d2.add_feature(FeatureKind::Hole(hole(vec![PointRef::Fixed([20., 20., 40.])])));
        let (v1, v2) = (vol(&d1), vol(&d2));
        assert!(v1 < 16000. - 1., "o furo cortou: {v1}");
        assert!((v1 - v2).abs() < 1e-3, "projetado: {v1} × {v2}");
        // com direção que não encontra a peça: erro que explica
        let mut d3 = base();
        let mut h = hole(vec![PointRef::Fixed([100., 20., 40.])]);
        h.dir = Some(AxisRef::Origin(OriginAxis::Z));
        let id = d3.add_feature(FeatureKind::Hole(h));
        let r = recompute(&d3);
        assert!(matches!(&r.status[&id], Status::Error(m) if m.contains("não encontra")), "{:?}", r.status[&id]);
    }

    #[test]
    fn filete_minimo_fica_so_na_aresta_escolhida() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let s = |x: &str| x.to_string();
        // caixa 40×20×10 com a aresta vertical de um canto arredondada (R5): a aresta de cima que
        // encosta nela continua tangente através do arredondado
        let mk = |prop: EdgeProp| {
            let mut d = Document::new("fm");
            let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [40., 20.]);
            d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind(s("10")), reverse: false, taper: s("0"), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
            d.add_feature(FeatureKind::Fillet(EdgeOp { value: s("5"), edges: EdgeSel::Edges(vec![[40., 20., 5.]]), mode: EdgeMode::Constant, prop: EdgeProp::Tangente, keys: vec![], extra: Default::default() }));
            // filete de 1 na aresta de cima do lado y = 20 (que continua tangente pelo arco do canto)
            d.add_feature(FeatureKind::Fillet(EdgeOp { value: s("1"), edges: EdgeSel::Edges(vec![[17.5, 20., 10.]]), mode: EdgeMode::Constant, prop, keys: vec![], extra: Default::default() }));
            d
        };
        let vol = |d: &Document| {
            let r = recompute(d);
            assert!(r.status.values().all(|x| !matches!(x, Status::Error(_))), "{:?}", r.status);
            r.bodies.iter().map(|b| b.mass.volume).sum::<f64>()
        };
        let v_t = vol(&mk(EdgeProp::Tangente));
        let v_m = vol(&mk(EdgeProp::Minima));
        // a mínima tira menos material (só a aresta escolhida, não a cadeia)
        assert!(v_m > v_t + 1e-3, "mínima {v_m} × tangente {v_t}");
        assert!(v_m < 40. * 20. * 10. - 1e-3, "a mínima também filetou: {v_m}");
    }

    #[test]
    fn membros_separados_com_cantos_e_emenda() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let s = |x: &str| x.to_string();
        let near = |a: f64, b: f64, tol: f64| (a - b).abs() <= tol * b.abs().max(1.);
        let mk = |paths: Vec<CurveRef>| StructMember {
            paths,
            family: s("Tubo quadrado"),
            designation: s("40x3"),
            kind: SectionKind::TuboQuadrado,
            dims: vec![s("40"), s("3")],
            anchor: 0,
            angle: s("0"),
            off_x: s("0"),
            off_y: s("0"),
            ext1: s("0"),
            ext2: s("0"),
            corner: CornerMode::MeiaEsquadria,
            gap: s("0"),
            joints: vec![],
            material: String::new(),
            mirror: false,
        };
        let vol = |d: &Document| recompute(d).bodies.iter().map(|b| b.mass.volume).sum::<f64>();
        let lado = |d: &mut Document, a: [f64; 3], b: [f64; 3]| CurveRef::Ref(d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed(a), b: PointRef::Fixed(b), start: s("0"), end: s("0") }));
        let mut d = Document::new("l");
        let l1 = lado(&mut d, [0., 0., 0.], [0., 0., 500.]);
        let l2 = lado(&mut d, [0., 0., 500.], [300., 0., 500.]);
        let f = d.add_feature(FeatureKind::StructMember(mk(vec![l1, l2])));
        let a = 40. * 40. - 34. * 34.;
        let v_one = vol(&d);
        assert!(near(v_one, a * 800., 1e-3), "um comando: {v_one}");
        // separa em dois comandos: o canto continua em meia-esquadria entre eles
        let ids = d.split_struct_member(f);
        assert_eq!(ids.len(), 2);
        let v_two = vol(&d);
        assert!(near(v_two, v_one, 1e-6), "dois comandos: {v_two}");
        assert_eq!(member_joints(&d).len(), 1);
        // regra própria do canto: sem tratamento → barras retas, sem sobra
        if let Some(FeatureKind::StructMember(m)) = d.feature_mut(ids[1]).map(|f| &mut f.kind) {
            m.joints.push(StructJoint { mine: 0, other: ids[0], other_path: 0, mode: CornerMode::Nenhum, first: false, gap: None, bend: None });
        }
        let v_none = vol(&d);
        assert!(near(v_none, a * 800., 1e-3), "sem tratamento: {v_none}");
        // aparar com o posterior (linha horizontal) como membro 1
        if let Some(FeatureKind::StructMember(m)) = d.feature_mut(ids[1]).map(|f| &mut f.kind) {
            m.joints[0] = StructJoint { mine: 0, other: ids[0], other_path: 0, mode: CornerMode::Aparar, first: true, gap: None, bend: None };
        }
        let r = recompute(&d);
        assert!(r.status.values().all(|x| !matches!(x, Status::Error(_))), "{:?}", r.status);
        // emenda: matéria-prima de 400 mm → a barra de 500 precisa de duas
        d.props.stock_length = s("400");
        let r = recompute(&d);
        assert!(matches!(&r.status[&ids[0]], Status::Warning(m) if m.contains("emenda")), "{:?}", r.status[&ids[0]]);
        let cl = cut_list(&d);
        assert!(cl.iter().any(|r| r.bars == 2) && cl.iter().any(|r| r.bars == 1), "{:?}", cl.iter().map(|r| (r.length, r.bars)).collect::<Vec<_>>());
    }

    #[test]
    fn conector_g3_e_canto_entre_linha_e_spline() {
        use geo3d::*;
        // G3: posição e derivadas nas pontas
        let (p0, d0, a0, j0) = ([0., 0., 0.], [10., 0., 0.], [0., 4., 0.], [0., 0., 2.]);
        let (p1, d1, a1, j1) = ([20., 5., 0.], [0., 10., 0.], [-3., 0., 0.], [1., 0., 0.]);
        let h = hermite7(p0, d0, a0, j0, p1, d1, a1, j1);
        assert!(dist(h[0], p0) < 1e-9 && dist(*h.last().unwrap(), p1) < 1e-9);
        let dt = 1. / 64.;
        let v0 = mul(sub(h[1], h[0]), 1. / dt);
        assert!(dist(v0, d0) < 0.5, "derivada inicial {v0:?}");
        // canto entre uma linha e uma spline que se encontram
        let ln = Curve3::line([0., 0., 0.], [20., 0., 0.]).unwrap();
        let sp = spline(&[[20., 0., 0.], [24., 8., 0.], [26., 20., 0.]], false).unwrap();
        let c = corner(&ln, &sp, 3.).unwrap();
        let (ctr, _, r) = c.circle.unwrap();
        assert!((r - 3.).abs() < 1e-9);
        // tangente às duas: centro a r da linha e a r da spline
        assert!((ctr[1].abs() - 3.).abs() < 1e-2, "centro {ctr:?}");
        let (_, q) = sp.nearest(ctr);
        assert!((dist(q, ctr) - 3.).abs() < 0.05, "distância à spline {}", dist(q, ctr));
        assert!(ctr[0] < 20., "canto do lado de dentro {ctr:?}");
    }

    #[test]
    fn solidos_com_perfil_do_wireframe() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let s = |x: &str| x.to_string();
        let ext = |d: &mut Document, prof: Id, h: &str| {
            d.add_feature(FeatureKind::Extrude(Extrude { sketch: prof, op: BoolOp::Add, extent: Extent::Blind(s(h)), reverse: false, taper: s("0"), thin: None, dir: None, thin_side: 0, ends: Default::default() }))
        };
        let vol = |d: &Document, f: Id| {
            let r = recompute(d);
            assert!(matches!(r.status[&f], Status::Ok | Status::Warning(_)), "{:?}", r.status[&f]);
            r.bodies.iter().map(|b| b.mass.volume).sum::<f64>()
        };
        let fx = |x: f64, y: f64, z: f64| PointRef::Fixed([x, y, z]);
        // polilinha fechada 30 x 20 num plano inclinado (plano sai da curva)
        let mut d = Document::new("wf1");
        let pl = d.add_geometry(RefKind::Polyline { points: vec![fx(0., 0., 0.), fx(30., 0., 0.), fx(30., 20., 20.), fx(0., 20., 20.)], closed: true });
        let e = ext(&mut d, pl, "10");
        let v = vol(&d, e);
        let area = 30. * (20f64 * 20. + 20. * 20.).sqrt();
        assert!((v - area * 10.).abs() < 1e-3 * area * 10., "polilinha: {v}");
        // círculo r = 5 → cilindro exato
        let mut d = Document::new("wf2");
        let c = d.add_geometry(RefKind::Circle { center: PointRef::Origin, plane: PlaneRef::Origin(OriginPlane::XY), radius: s("5"), start: s("0"), end: s("360") });
        let e = ext(&mut d, c, "4");
        let v = vol(&d, e);
        assert!((v - std::f64::consts::PI * 25. * 4.).abs() < 0.05, "círculo: {v}");
        // quatro linhas soltas (uma ao contrário) juntadas num contorno
        let mut d = Document::new("wf3");
        let ln = |d: &mut Document, a: [f64; 3], b: [f64; 3]| d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed(a), b: PointRef::Fixed(b), start: s("0"), end: s("0") });
        let l1 = ln(&mut d, [0., 0., 0.], [10., 0., 0.]);
        let l2 = ln(&mut d, [10., 0., 0.], [10., 10., 0.]);
        let l3 = ln(&mut d, [0., 10., 0.], [10., 10., 0.]);
        let l4 = ln(&mut d, [0., 10., 0.], [0., 0., 0.]);
        let j = d.add_geometry(RefKind::CurveJoin { curves: [l1, l3, l2, l4].map(CurveRef::Ref).to_vec() });
        let e = ext(&mut d, j, "5");
        let v = vol(&d, e);
        assert!((v - 500.).abs() < 1e-3, "juntar: {v}");
        // curva aberta: erro que explica o que falta
        let mut d = Document::new("wf4");
        let a = ln(&mut d, [0., 0., 0.], [10., 0., 0.]);
        let b = ln(&mut d, [10., 0., 0.], [10., 10., 0.]);
        let j = d.add_geometry(RefKind::CurveJoin { curves: vec![CurveRef::Ref(a), CurveRef::Ref(b)] });
        let e = ext(&mut d, j, "5");
        let r = recompute(&d);
        assert!(matches!(&r.status[&e], Status::Error(m) if m.contains("aberta")), "{:?}", r.status[&e]);
    }

    #[test]
    fn extrusao_de_perfil_com_spline() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        use fenkra_core::sketch::Curve;
        let s = |x: &str| x.to_string();
        let mut d = Document::new("spline");
        let mut data = SketchData::default();
        let mut n = 6_000_000u64;
        let mut al = || {
            n += 1;
            Id(n)
        };
        let p: Vec<Id> = [[0., 0.], [40., 0.], [34., 15.], [20., 24.], [0., 20.]].iter().map(|q| data.add_point(&mut al, q[0], q[1])).collect();
        data.add_curve(&mut al, Curve::Line { a: p[0], b: p[1] }, false);
        data.add_curve(&mut al, Curve::Spline { pts: vec![p[1], p[2], p[3], p[4]] }, false);
        data.add_curve(&mut al, Curve::Line { a: p[4], b: p[0] }, false);
        let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
        let ex = d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind(s("10")), reverse: false, taper: s("0"), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
        let r = recompute(&d);
        assert!(matches!(r.status[&ex], Status::Ok | Status::Warning(_)), "{:?}", r.status[&ex]);
        let v = r.bodies.iter().map(|b| b.mass.volume).sum::<f64>();
        // entre o triângulo (0,0)-(40,0)-(0,20) e a caixa 40x24
        assert!(v > 4000. && v < 9600., "volume com spline: {v}");
    }

    #[test]
    fn chapa_joggle_estampo_furo_flangeado_alivio() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        use std::f64::consts::PI;
        let near = |a: f64, b: f64, tol: f64| (a - b).abs() <= tol * b.abs().max(1.);
        let s = |x: &str| x.to_string();
        let base = || {
            let mut d = Document::new("chapa");
            d.sheet = Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), flat: false, ..Default::default() });
            let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [100., 50.]);
            d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }));
            d
        };
        let check = |d: &Document| {
            let r = recompute(d);
            for f in &d.features {
                assert_eq!(r.status[&f.id], Status::Ok, "{}: {:?}", f.name, r.status[&f.id]);
            }
            r
        };
        // joggle: o volume se conserva e a parte além da linha desce 3 mm
        let mut d = base();
        d.add_feature(FeatureKind::SmJoggle(SmJoggle { a: [60., 0., 2.], b: [60., 50., 2.], depth: s("3"), run: s("5"), flip: true, angle: None, plane: None, keep_length: false }));
        let r = check(&d);
        assert!(near(r.bodies[0].mass.volume, 10000., 1e-3), "joggle {}", r.bodies[0].mass.volume);
        let zmin = r.bodies[0].mesh.positions.chunks_exact(3).map(|p| p[2]).fold(f32::MAX, f32::min);
        assert!((zmin + 3.).abs() < 0.01, "desvio {zmin}");
        // dobra por linha a 90°: volume conservado (comprimento desenvolvido) e a ponta sobe
        let mut d = base();
        d.add_feature(FeatureKind::SmBend(SmBend { a: [60., 0., 2.], b: [60., 50., 2.], angle: s("90"), radius: None, flip: false, down: false, sketch: None, position: 0 }));
        let r = check(&d);
        assert!(near(r.bodies[0].mass.volume, 10000., 2e-3), "dobra por linha {}", r.bodies[0].mass.volume);
        let zmax = r.bodies[0].mesh.positions.chunks_exact(3).map(|p| p[2]).fold(f32::MIN, f32::max);
        assert!(zmax > 30., "a aba subiu {zmax}");
        // veneziana: volume conservado e abertura de 5 mm
        let mut d = base();
        d.add_feature(FeatureKind::SmLouver(SmLouver { a: [30., 10., 2.], b: [30., 40., 2.], width: s("12"), height: s("5"), flip: false, down: false }));
        let r = check(&d);
        assert!(near(r.bodies[0].mass.volume, 10000., 1e-3), "veneziana {}", r.bodies[0].mass.volume);
        let zmax = r.bodies[0].mesh.positions.chunks_exact(3).map(|p| p[2]).fold(f32::MIN, f32::max);
        assert!((zmax - 7.).abs() < 0.01, "altura da veneziana {zmax}");
        // estampo 20×20 com 3 mm de profundidade
        let mut d = base();
        let top = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "2");
        let sq = sk_rect(&mut d, PlaneRef::Datum(top), [40., 15.], [60., 35.]);
        d.add_feature(FeatureKind::SmStamp(SmStamp { sketch: sq, depth: s("3"), flip: false, kind: 0, width: String::new(), punch: None }));
        let r = check(&d);
        let outer = 400. + 80. * 2. + PI * 4.;
        assert!(near(r.bodies[0].mass.volume, 10000. + outer * 3. - 400. * 3., 1e-3), "estampo {}", r.bodies[0].mass.volume);
        // furo flangeado Ø10 com colar de 6 mm
        let mut d = base();
        let top = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "2");
        let mut data = SketchData::default();
        let mut n = 7_000_000u64;
        let mut al = || {
            n += 1;
            Id(n)
        };
        let p0 = data.add_point(&mut al, 50., 25.);
        data.add_curve(&mut al, fenkra_core::sketch::Curve::Point { p: p0 }, false);
        let sp = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Datum(top), data }));
        d.add_feature(FeatureKind::SmFlangedHole(SmFlangedHole { sketch: sp, diameter: s("10"), height: s("6"), flip: false, points: vec![], standard: String::new() }));
        let r = check(&d);
        assert!(near(r.bodies[0].mass.volume, 10000. + PI * 24. * 6. - PI * 25. * 2., 1e-3), "furo flangeado {}", r.bodies[0].mass.volume);
        // aba recuada 10 mm de cada lado com alívio de 3 mm
        let mut d = base();
        d.add_feature(FeatureKind::SmFlange(SmFlange { edge: [100., 25., 2.], length: s("20"), angle: s("90"), radius: None, off1: s("10"), off2: s("10"), flip: false, relief: None, relief_kind: 0, side: 0, miter: false, min_gap: true, edge_key: None }));
        let v_sem = check(&d).bodies[0].mass.volume;
        if let FeatureKind::SmFlange(fl) = &mut d.features.last_mut().unwrap().kind {
            fl.relief = Some(s("3"));
        }
        let v_com = check(&d).bodies[0].mass.volume;
        assert!(near(v_sem - v_com, 2. * 3. * 4. * 2., 1e-3), "alívio {}", v_sem - v_com);
    }

    #[test]
    fn chapa_recortes_estampos_alivios_abas_novas() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        use std::f64::consts::PI;
        let near = |a: f64, b: f64, tol: f64| (a - b).abs() <= tol * b.abs().max(1.);
        let s = |x: &str| x.to_string();
        let base = || {
            let mut d = Document::new("chapa");
            d.sheet = Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), flat: false, ..Default::default() });
            let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [100., 50.]);
            d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }));
            d
        };
        let check = |d: &Document| {
            let r = recompute(d);
            for f in &d.features {
                assert_eq!(r.status[&f.id], Status::Ok, "{}: {:?}", f.name, r.status[&f.id]);
            }
            r
        };
        let vol = |r: &RecomputeResult| r.bodies[0].mass.volume;
        // compensação: dedução de 4,4 mm a 90° (t = r = 2) → K equivalente
        let k = fenkra_core::sheet_k(1, 0.44, 2., 2., 4.4);
        assert!(near(PI / 2. * (2. + k * 2.), 8. - 4.4, 1e-9), "K da dedução {k}");
        // recorte normal 20×10: tira 400 mm³
        let mut d = base();
        let top = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "2");
        let sq = sk_rect(&mut d, PlaneRef::Datum(top), [20., 20.], [40., 30.]);
        let cut = d.add_feature(FeatureKind::SmCut(SmCut { sketch: sq, mode: 0, offset: String::new(), draft: s("0"), height: s("0"), flip: false, normal_face: false }));
        let r = check(&d);
        assert!(near(vol(&r), 10000. - 400., 1e-3), "recorte {}", vol(&r));
        // folga de 1 mm: 22×12
        if let FeatureKind::SmCut(c) = &mut d.feature_mut(cut).unwrap().kind {
            c.offset = s("1");
        }
        let r = check(&d);
        assert!(vol(&r) < 10000. - 400. - 100., "recorte com folga {}", vol(&r));
        // recorte flangeado: ganha o colar
        if let FeatureKind::SmCut(c) = &mut d.feature_mut(cut).unwrap().kind {
            c.offset = String::new();
            c.mode = 2;
            c.height = s("4");
        }
        let r = check(&d);
        assert!(vol(&r) > 10000. - 400. + 100., "colar do recorte {}", vol(&r));
        // estampo circular Ø16 no ponto e padrão linear de 3 (soma e corte repetidos)
        let mut d = base();
        let top = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "2");
        let mut data = SketchData::default();
        let mut n = 8_000_000u64;
        let mut al = || {
            n += 1;
            Id(n)
        };
        let p0 = data.add_point(&mut al, 25., 25.);
        data.add_curve(&mut al, fenkra_core::sketch::Curve::Point { p: p0 }, false);
        let sp = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Datum(top), data }));
        let st = d.add_feature(FeatureKind::SmStamp(SmStamp { sketch: sp, depth: s("3"), flip: false, kind: 2, width: s("16"), punch: None }));
        let v1 = vol(&check(&d));
        assert!(v1 > 10000., "estampo circular {v1}");
        d.add_feature(FeatureKind::Pattern(Pattern {
            kind: PatternKind::Linear { dir: AxisRef::Origin(OriginAxis::X), count: s("3"), spacing: s("25"), dir2: None, count2: s("1"), spacing2: s("0"), total: false },
            sources: vec![st], skip: Default::default(),
        }));
        let v3 = vol(&check(&d));
        assert!(near(v3 - 10000., 3. * (v1 - 10000.), 2e-2), "padrão do estampo {v3} {v1}");
        // nervura ao longo de uma linha
        let mut d = base();
        let top = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "2");
        let ln = sk_lines(&mut d, PlaneRef::Datum(top), &[([20., 10.], [80., 10.])]);
        d.add_feature(FeatureKind::SmStamp(SmStamp { sketch: ln, depth: s("2"), flip: false, kind: 1, width: s("6"), punch: None }));
        assert!(vol(&check(&d)) > 10000., "nervura");
        // alívio de canto redondo Ø6 no canto (100, 50)
        let mut d = base();
        d.add_feature(FeatureKind::SmCornerRelief(SmCornerRelief { at: [100., 50., 2.], size: s("6"), kind: 0 }));
        let r = check(&d);
        // rasgo circular: raio 15, largura 4, 90° (área = 4·15·π/2 + π·2²)
        let mut d2 = base();
        d2.add_feature(FeatureKind::SmArcSlot(SmArcSlot { center: [50., 25., 2.], radius: s("15"), width: s("4"), start: s("0"), sweep: s("90") }));
        let r2 = check(&d2);
        let area = 4. * 15. * PI / 2. + PI * 4.;
        assert!(near(10000. - vol(&r2), area * 2., 1e-2), "rasgo circular {}", 10000. - vol(&r2));
        assert!(near(10000. - vol(&r), PI * 9. / 4. * 2., 2e-2), "alívio de canto {}", 10000. - vol(&r));
        // bainha em gota e aba com material por dentro, pontas a 45° e alívio redondo
        let mut d = base();
        d.add_feature(FeatureKind::SmHem(SmHem { edge: [0., 25., 2.], length: s("6"), gap: s("2"), flip: false, kind: 1, edge_key: None }));
        check(&d);
        let mut d = base();
        d.add_feature(FeatureKind::SmFlange(SmFlange { edge: [100., 25., 2.], length: s("20"), angle: s("90"), radius: None, off1: s("5"), off2: s("5"), flip: false, relief: Some(s("3")), relief_kind: 1, side: 2, miter: true, min_gap: true, edge_key: None }));
        check(&d);
        // joggle pelo ângulo (45°) com compensação
        let mut d = base();
        d.add_feature(FeatureKind::SmJoggle(SmJoggle { a: [60., 0., 2.], b: [60., 50., 2.], depth: s("3"), run: s("5"), flip: true, angle: Some(s("45")), plane: None, keep_length: true }));
        check(&d);
        // aba por perfil: L aberto no plano XZ na ponta x = 100, puxado 50 mm em y
        let mut d = base();
        let pr = sk_lines(&mut d, PlaneRef::Origin(OriginPlane::XZ), &[([100., 2.], [100., 20.]), ([100., 20.], [110., 20.])]);
        d.add_feature(FeatureKind::SmProfileFlange(SmProfileFlange { sketch: pr, l1: s("0"), l2: s("50"), radius: None, flip: false }));
        let r = check(&d);
        assert!(vol(&r) > 10000. + 2. * 20. * 40., "aba por perfil {}", vol(&r));
        // parede afastada 5 mm e simétrica
        let mut d = Document::new("chapa");
        d.sheet = Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), flat: false, ..Default::default() });
        let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [10., 10.]);
        d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: s("5"), mid: true, surface: None }));
        let r = check(&d);
        let zmin = r.bodies[0].mesh.positions.chunks_exact(3).map(|p| p[2]).fold(f32::MAX, f32::min);
        assert!((zmin - 4.).abs() < 1e-3, "parede afastada e simétrica {zmin}");
    }

    #[test]
    fn chapa_parede_aba_bainha_e_planificado() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        use std::f64::consts::PI;
        let near = |a: f64, b: f64, tol: f64| (a - b).abs() <= tol * b.abs().max(1.);
        let s = |x: &str| x.to_string();
        let mut d = Document::new("chapa");
        d.sheet = Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), flat: false, ..Default::default() });
        let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [100., 50.]);
        d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }));
        d.add_feature(FeatureKind::SmFlange(SmFlange { edge: [100., 25., 2.], length: s("20"), angle: s("90"), radius: None, off1: s("0"), off2: s("0"), flip: false, relief: None, relief_kind: 0, side: 0, miter: false, min_gap: true, edge_key: None }));
        let r = recompute(&d);
        for f in &d.features {
            assert!(matches!(r.status[&f.id], Status::Ok | Status::Warning(_)), "{}: {:?}", f.name, r.status[&f.id]);
        }
        let v = r.bodies[0].mass.volume;
        let bend = PI / 4. * (16. - 4.) * 50.;
        assert!(near(v, 10000. + bend + 2000., 1e-3), "dobrada {v}");
        // a aba sobe: topo em z = 2 + R + 20
        let zmax = r.bodies[0].mesh.positions.chunks_exact(3).map(|p| p[2]).fold(f32::MIN, f32::max);
        assert!((zmax - 24.).abs() < 0.05, "altura {zmax}");
        // planificado: tira com o comprimento desenvolvido
        d.sheet.as_mut().unwrap().flat = true;
        let r = recompute(&d);
        let ba = PI / 2. * (2. + 0.44 * 2.);
        let vf = r.bodies[0].mass.volume;
        assert!(near(vf, 10000. + (ba + 20.) * 2. * 50., 1e-3), "plana {vf}");
        let tab = bend_table(&d);
        assert_eq!(tab.len(), 1);
        assert!((tab[0].allowance - ba).abs() < 1e-9);
        // bainha esmagada na aresta oposta (x = 0), dobrando para baixo
        d.sheet.as_mut().unwrap().flat = false;
        d.add_feature(FeatureKind::SmHem(SmHem { edge: [0., 25., 2.], length: s("8"), gap: s("0"), flip: true, kind: 0, edge_key: None }));
        let r = recompute(&d);
        let st = &r.status[&d.features.last().unwrap().id];
        assert_eq!(*st, Status::Ok, "{st:?}");
        assert!(r.bodies[0].mass.volume > v + 8. * 2. * 50. * 0.9, "bainha {}", r.bodies[0].mass.volume);
    }

    #[test]
    fn publicar_superficie_link_e_isolar() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let mut src = Document::new("Mestre");
        let sph = src.add_geometry(RefKind::SurfSphere { center: PointRef::Origin, radius: "10".into() });
        src.references.iter_mut().find(|r| r.id == sph).unwrap().published = true;
        let pubs = publications(&src);
        let (n, g) = pubs.iter().find(|(_, g)| matches!(g, LinkedGeo::Surface(_))).expect("superfície publicada");
        let mut d = Document::new("Filha");
        let id = d.add_link("Mestre", "mestre.fkp", n, g.clone());
        let r = recompute(&d);
        assert_eq!(r.status[&id], Status::Ok, "{:?}", r.status[&id]);
        // desativado: inativo até reativar
        let ext = d.external[0].id;
        d.external[0].state = LinkState::Desativado;
        assert!(matches!(recompute(&d).status[&id], Status::Error(_)));
        d.external[0].state = LinkState::Ativo;
        d.isolate_link(ext);
        assert!(d.external.is_empty());
        let r = recompute(&d);
        assert_eq!(r.status[&id], Status::Ok, "isolada: {:?}", r.status[&id]);
        assert!(r.surfaces.iter().any(|s| s.id == id));
    }

    #[test]
    fn entradas_consumidas_e_renomear() {
        let mut d = Document::new("c");
        let s = |x: &str| x.to_string();
        let a = d.add_geometry(RefKind::SurfSphere { center: PointRef::Origin, radius: s("10") });
        let b = d.add_geometry(RefKind::SurfSphere { center: PointRef::Fixed([15., 0., 0.]), radius: s("10") });
        let j = d.add_geometry(RefKind::SurfJoin { items: vec![SurfRef::Ref(a), SurfRef::Ref(b)], tol: s("0.01"), curves: vec![] });
        d.hide_consumed(j);
        assert!(!d.reference(a).unwrap().visible && !d.reference(b).unwrap().visible);
        d.delete(j, false);
        assert!(d.reference(a).unwrap().visible && d.reference(b).unwrap().visible, "apagar a operação reexibe as entradas");
        d.rename_item(a, "Bola").unwrap();
        assert_eq!(d.name_of(a), "Bola");
        assert!(d.rename_item(b, "Bola").is_err(), "nome repetido");
    }

    #[test]
    fn furos_e_terminos_novos() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        use std::f64::consts::PI;
        let near = |a: f64, b: f64, tol: f64| (a - b).abs() <= tol * b.abs().max(1.);
        let s = |x: &str| x.to_string();
        let v0 = vol(&caixa());
        let furo = |pts: Vec<PointRef>, kind: HoleKind, tip: Option<String>, depth: Extent| Hole {
            sketch: Id(0),
            kind,
            diameter: s("6"),
            depth,
            thread: None,
            thread_modeled: false,
            standard: None,
            points: pts,
            dir: None,
            reverse: false,
            tip,
            taper: s("0"),
            left_hand: false, extra: Default::default(),
        };
        // dois furos Ø6 × 5 por pontos 3D no topo (normal à superfície, para baixo)
        let topo = vec![PointRef::Fixed([-10., 0., 20.]), PointRef::Fixed([10., 0., 20.])];
        let mut d = caixa();
        d.add_feature(FeatureKind::Hole(furo(topo.clone(), HoleKind::Simples, None, Extent::Blind(s("5")))));
        let vf = vol(&d);
        assert!(near(vf, v0 - 2. * PI * 9. * 5., 1e-3), "furos 3D {vf}");
        // fundo em V tira mais que o fundo plano
        let mut d = caixa();
        d.add_feature(FeatureKind::Hole(furo(topo.clone(), HoleKind::Simples, Some(s("118")), Extent::Blind(s("5")))));
        assert!(vol(&d) < vf - 1., "fundo em V");
        // furo numa face lateral (x = 20): a normal da face manda o furo para -X
        let mut d = caixa();
        d.add_feature(FeatureKind::Hole(furo(vec![PointRef::Fixed([20., 0., 10.])], HoleKind::Simples, None, Extent::Blind(s("5")))));
        assert!(near(vol(&d), v0 - PI * 9. * 5., 1e-3), "furo lateral {}", vol(&d));
        // contrafurado passante
        let mut d = caixa();
        d.add_feature(FeatureKind::Hole(furo(vec![topo[0]], HoleKind::Contrafurado { diameter: s("10"), depth: s("3"), angle: s("90") }, None, Extent::ThroughAll)));
        let vc = vol(&d);
        assert!(vc < v0 - PI * 9. * 20. - PI * 16. * 3. + 1., "contrafurado {vc}");

        // extrusão até o último: do plano z = 30 para baixo até a base (z = 0)
        let mut d = caixa();
        let alto = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "30");
        let sk = sk_rect(&mut d, PlaneRef::Datum(alto), [-5., -5.], [5., 5.]);
        d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::UpToLast, reverse: true, taper: s("0"), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
        assert!(near(vol(&d), v0 + 100. * 10., 1e-6), "até o último {}", vol(&d));

        // extrusão inclinada por uma direção: o volume do prisma oblíquo é área × altura
        let mut d = Document::new("e");
        let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [10., 10.]);
        let l = d.add_geometry(RefKind::LinePoints { a: PointRef::Origin, b: PointRef::Fixed([10., 0., 10.]), start: s("0"), end: s("0") });
        d.add_feature(FeatureKind::Extrude(Extrude { thin_side: 0, sketch: sk, op: BoolOp::Add, extent: Extent::Blind(s("10")), reverse: false, taper: s("0"), thin: None, dir: Some(AxisRef::Ref(l)), ends: Default::default() }));
        assert!(near(vol(&d), 100. * 10. / 2f64.sqrt(), 1e-6), "oblíqua {}", vol(&d));

        // padrão linear por comprimento total: 3 bolsos em 20 mm = espaçamento 10
        let mut d = caixa();
        let top = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "20");
        let sp = sk_rect(&mut d, PlaneRef::Datum(top), [-16., -2.], [-12., 2.]);
        let pocket = d.add_feature(ext(sp, BoolOp::Cut, "5"));
        d.add_feature(FeatureKind::Pattern(Pattern {
            kind: PatternKind::Linear { dir: AxisRef::Origin(OriginAxis::X), count: s("3"), spacing: s("20"), dir2: None, count2: s("1"), spacing2: s("0"), total: true },
            sources: vec![pocket], skip: Default::default(),
        }));
        assert!(near(vol(&d), v0 - 240., 1e-6), "padrão por total {}", vol(&d));

        // mover o corpo: translação mantém o volume; afinidade multiplica por fx·fy·fz
        let mut d = caixa();
        d.add_feature(FeatureKind::MoveBody(MoveBody { xf: BodyXf::Translate { dir: AxisRef::Origin(OriginAxis::X), dist: s("50") } }));
        let r = recompute(&d);
        assert!(near(r.bodies[0].mass.volume, v0, 1e-6) && (r.bodies[0].mass.center[0] - 50.).abs() < 1e-6, "translação");
        let mut d = caixa();
        d.add_feature(FeatureKind::MoveBody(MoveBody { xf: BodyXf::Affinity { plane: PlaneRef::Origin(OriginPlane::XY), fx: s("2"), fy: s("1"), fz: s("0.5") } }));
        assert!(near(vol(&d), v0, 1e-3), "afinidade 2·1·0,5 {}", vol(&d));
    }

    #[test]
    fn curva_paralela_e_projecao_por_direcao() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let s = |x: &str| x.to_string();
        let mut d = Document::new("w");
        let l = d.add_geometry(RefKind::LinePoints { a: PointRef::Origin, b: PointRef::Fixed([30., 0., 0.]), start: s("0"), end: s("0") });
        let p = d.add_geometry(RefKind::CurveParallel { curve: CurveRef::Ref(l), support: SurfRef::Plane(PlaneRef::Origin(OriginPlane::XY)), d1: s("5"), d2: s("5"), flip: false, more: vec![], both: false, d_other: String::new() });
        let alto = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "10");
        let pr = d.add_geometry(RefKind::CurveProjectSurf { curve: CurveRef::Ref(l), surface: SurfRef::Plane(PlaneRef::Datum(alto)), dir: Some(AxisRef::Fixed { origin: [0.; 3], dir: [0., 1., 1.] }) });
        let r = recompute(&d);
        for id in [p, pr] {
            assert_eq!(r.status[&id], Status::Ok, "{:?}", r.status[&id]);
        }
        let c = r.curves.iter().find(|c| c.id == p).unwrap();
        assert!(c.pts.iter().all(|q| (q[1].abs() - 5.).abs() < 1e-9), "paralela a 5 mm");
        let c = r.curves.iter().find(|c| c.id == pr).unwrap();
        assert!(c.pts.iter().all(|q| (q[2] - 10.).abs() < 1e-9 && (q[1] - 10.).abs() < 1e-9), "projeção oblíqua");
    }

    fn mesh_area(m: &Mesh) -> f64 {
        let p = |i: u32| {
            let k = i as usize * 3;
            [m.positions[k] as f64, m.positions[k + 1] as f64, m.positions[k + 2] as f64]
        };
        m.indices
            .chunks_exact(3)
            .map(|t| {
                let (a, b, c) = (p(t[0]), p(t[1]), p(t[2]));
                geo3d::norm(geo3d::cross(geo3d::sub(b, a), geo3d::sub(c, a))) / 2.
            })
            .sum()
    }

    #[test]
    fn bancada_de_superficies() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        use std::f64::consts::PI;
        let near = |a: f64, b: f64, tol: f64| (a - b).abs() <= tol * b.abs().max(1.);
        let xy = || SurfRef::Plane(PlaneRef::Origin(OriginPlane::XY));
        let s = |x: &str| x.to_string();
        let mut d = Document::new("s");
        let sph = d.add_geometry(RefKind::SurfSphere { center: PointRef::Origin, radius: s("10") });
        let half = d.add_geometry(RefKind::SurfSplit { surface: SurfRef::Ref(sph), tool: xy(), flip: false, more: vec![], curve: None, more_flip: vec![] });
        let circ = d.add_geometry(RefKind::CurveIntersect { a: SurfRef::Ref(sph), b: xy(), which: 0, near: None });
        let cyl = d.add_geometry(RefKind::SurfCylinder { point: PointRef::Origin, dir: AxisRef::Origin(OriginAxis::Z), radius: s("10"), start: s("0"), end: s("20") });
        let bnd = d.add_geometry(RefKind::CurveBoundary { surface: SurfRef::Ref(cyl), which: 0, prop: Propagation::Nenhuma, near: None, expected: None });
        let moved = d.add_geometry(RefKind::GeoTransform {
            source: sph,
            class: RefClass::Surface,
            xf: GeoXform::Translate { dir: AxisRef::Origin(OriginAxis::X), dist: s("50") },
        });
        let p1 = d.add_geometry(RefKind::PointCoords { x: s("1"), y: s("0"), z: s("0"), frame: None });
        let pmov = d.add_geometry(RefKind::GeoTransform {
            source: p1,
            class: RefClass::Point,
            xf: GeoXform::Rotate { axis: AxisRef::Origin(OriginAxis::Z), angle: s("90") },
        });
        // duas bordas paralelas: tubo por varredura, multi-seções e concordância G1 entre duas faixas planas
        let la = d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed([0., 0., 0.]), b: PointRef::Fixed([0., 30., 0.]), start: s("0"), end: s("0") });
        let lb = d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed([20., 0., 10.]), b: PointRef::Fixed([20., 30., 10.]), start: s("0"), end: s("0") });
        let tube = d.add_geometry(RefKind::SurfSweep { profile: SweepProfile::Circle(s("2")), spine: CurveRef::Ref(la), mode: SweepMode::Corrigido });
        let loft = d.add_geometry(RefKind::SurfLoft { sections: vec![CurveRef::Ref(la), CurveRef::Ref(lb)], guide: None, ruled: false, closed: false, opts: Default::default() });
        let fa = d.add_geometry(RefKind::SurfExtrude { curve: CurveRef::Ref(la), dir: AxisRef::Origin(OriginAxis::X), length: s("-10"), length2: "0".into(), until: None, until2: None });
        let fb = d.add_geometry(RefKind::SurfExtrude { curve: CurveRef::Ref(lb), dir: AxisRef::Origin(OriginAxis::X), length: s("10"), length2: "0".into(), until: None, until2: None });
        let blend = d.add_geometry(RefKind::SurfBlend {
            a: CurveRef::Ref(la),
            sa: Some(SurfRef::Ref(fa)),
            b: CurveRef::Ref(lb),
            sb: Some(SurfRef::Ref(fb)),
            cont: Continuity::G1,
            tension: s("1"),
            tension2: None,
        });
        let join = d.add_geometry(RefKind::SurfJoin { items: vec![SurfRef::Ref(fa), SurfRef::Ref(blend), SurfRef::Ref(fb)], tol: s("0.01"), curves: vec![] });
        let bounds = d.add_geometry(RefKind::CurveBoundary { surface: SurfRef::Ref(join), which: 0, prop: Propagation::Nenhuma, near: None, expected: None });
        let r = recompute(&d);
        for id in [sph, half, circ, cyl, bnd, moved, pmov, tube, loft, fa, fb, blend, join, bounds] {
            assert_eq!(r.status[&id], Status::Ok, "{}: {:?}", d.name_of(id), r.status[&id]);
        }
        let area = |id: Id| mesh_area(&r.surfaces.iter().find(|s| s.id == id).unwrap().mesh);
        assert!(near(area(half), 2. * PI * 100., 0.02), "meia esfera {}", area(half));
        let len = |id: Id| r.curves.iter().find(|c| c.id == id).unwrap().pts.windows(2).map(|w| geo3d::dist(w[0], w[1])).sum::<f64>();
        assert!(near(len(circ), 2. * PI * 10., 0.02), "círculo {}", len(circ));
        assert!(near(len(bnd), 2. * PI * 10., 0.02), "borda {}", len(bnd));
        assert!(near(area(tube), 2. * PI * 2. * 30., 0.03), "tubo {}", area(tube));
        let p = r.points.iter().find(|p| p.id == pmov).unwrap().p;
        assert!(geo3d::dist(p, [0., 1., 0.]) < 1e-9, "{p:?}");
        let c = r.surfaces.iter().find(|s| s.id == moved).unwrap().mesh.positions.chunks(3).map(|v| v[0] as f64).sum::<f64>()
            / (r.surfaces.iter().find(|s| s.id == moved).unwrap().mesh.positions.len() / 3) as f64;
        assert!((c - 50.).abs() < 1., "esfera movida {c}");
        assert!(area(blend) > 22.36 * 30., "concordância G1 é mais longa que a reta {}", area(blend));

        // híbridos: engrossar, fechar, preencher fronteiras, dividir corpo
        let mut h = d.clone();
        h.add_feature(FeatureKind::ThickSurface(ThickSurface { surface: SurfRef::Ref(cyl), t1: s("2"), t2: s("0"), op: BoolOp::Add }));
        let v = vol(&h);
        assert!(near(v, PI * (144. - 100.) * 20., 0.02) || near(v, PI * (100. - 64.) * 20., 0.02), "engrossar {v}");
        let mut h = d.clone();
        h.add_feature(FeatureKind::CloseSurface(CloseSurface { surface: SurfRef::Ref(cyl), tol: s("0.01"), op: BoolOp::Add }));
        assert!(near(vol(&h), PI * 100. * 20., 0.01), "fechar {}", vol(&h));
        let mut h = d.clone();
        h.add_feature(FeatureKind::BoundaryFill(BoundaryFill { tools: vec![SurfRef::Ref(sph), xy()], exclude: vec![[0., 0., -5.]], op: BoolOp::Add }));
        assert!(near(vol(&h), 2. / 3. * PI * 1000., 0.02), "preencher {}", vol(&h));
        let mut h = caixa();
        let meio = h.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "5");
        h.add_feature(FeatureKind::SplitBody(SplitBody { surface: SurfRef::Plane(PlaneRef::Datum(meio)), flip: false }));
        assert!(near(vol(&h), 24000. * 0.75, 1e-6), "dividir {}", vol(&h));

        // malha STL (cubo de 10) → sólido
        let dir = std::env::temp_dir().join("fenkra_mesh_test.stl");
        let mut t = String::from("solid c\n");
        let v = |i: usize| [(i & 1) as f64 * 10., ((i >> 1) & 1) as f64 * 10., ((i >> 2) & 1) as f64 * 10.];
        for f in [[0, 2, 3, 1], [4, 5, 7, 6], [0, 1, 5, 4], [2, 6, 7, 3], [0, 4, 6, 2], [1, 3, 7, 5]] {
            for tri in [[f[0], f[1], f[2]], [f[0], f[2], f[3]]] {
                t += "facet normal 0 0 0\nouter loop\n";
                for k in tri {
                    let p = v(k);
                    t += &format!("vertex {} {} {}\n", p[0], p[1], p[2]);
                }
                t += "endloop\nendfacet\n";
            }
        }
        t += "endsolid c\n";
        std::fs::write(&dir, t).unwrap();
        let mut m = Document::new("m");
        m.add_feature(FeatureKind::MeshImport(MeshImport { path: dir.to_string_lossy().into(), solid: true, ..Default::default() }));
        let r = recompute(&m);
        assert!(matches!(r.status[&m.features[0].id], Status::Warning(_)), "{:?}", r.status[&m.features[0].id]);
        assert!(near(r.bodies[0].mass.volume, 1000., 1e-6), "malha {}", r.bodies[0].mass.volume);
    }

    #[test]
    fn terminos_unificados() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let near = |a: f64, b: f64, tol: f64| (a - b).abs() <= tol * b.abs().max(1.);
        let v0 = vol(&caixa());
        // extrusão de cima para baixo até a face do topo: pino de 10 × 10 × 20
        let mut d = caixa();
        let alto = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "40");
        let s = sk_rect(&mut d, PlaneRef::Datum(alto), [-5., -5.], [5., 5.]);
        d.add_feature(FeatureKind::Extrude(Extrude { thin_side: 0,
            sketch: s,
            op: BoolOp::Add,
            extent: Extent::UpToSurface(SurfRef::Face { point: [0., 0., 20.], normal: [0., 0., 1.], owner: None, key: None }),
            reverse: true,
            taper: "0".into(),
            thin: None, dir: None, ends: Default::default(),
        }));
        assert!(near(vol(&d), v0 + 2000., 1e-6), "até a face {}", vol(&d));

        // recorte do topo até um plano (z = 12) como superfície
        let mut d = caixa();
        let topo = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "20");
        let fundo = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "12");
        let s = sk_rect(&mut d, PlaneRef::Datum(topo), [-5., -5.], [5., 5.]);
        d.add_feature(FeatureKind::Extrude(Extrude { thin_side: 0,
            sketch: s,
            op: BoolOp::Cut,
            extent: Extent::UpToSurface(SurfRef::Plane(PlaneRef::Datum(fundo))),
            reverse: false,
            taper: "0".into(),
            thin: None, dir: None, ends: Default::default(),
        }));
        assert!(near(vol(&d), v0 - 800., 1e-6), "recorte até plano {}", vol(&d));

        // furo Ø6 do topo até o plano z = 12
        let mut d = caixa();
        let topo = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "20");
        let fundo = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "12");
        let mut data = SketchData::default();
        let mut n = 5_000_000u64;
        let mut al = || {
            n += 1;
            Id(n)
        };
        let p0 = data.add_point(&mut al, 0., 0.);
        data.add_curve(&mut al, fenkra_core::sketch::Curve::Point { p: p0 }, false);
        let sp = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Datum(topo), data }));
        let furo = Hole { sketch: sp, kind: HoleKind::Simples, diameter: "6".into(), depth: Extent::UpTo(PlaneRef::Datum(fundo)), thread: None, thread_modeled: false, standard: None, points: vec![], dir: None, reverse: false, tip: None, taper: "0".into(), left_hand: false, extra: Default::default() };
        let mut e = d.clone();
        e.add_feature(FeatureKind::Hole(furo.clone()));
        assert!(near(vol(&e), v0 - std::f64::consts::PI * 9. * 8., 1e-3), "furo até plano {}", vol(&e));
        // o mesmo furo "até o próximo" atravessa a parede inteira (20)
        let mut e = d.clone();
        e.add_feature(FeatureKind::Hole(Hole { depth: Extent::UpToNext, ..furo }));
        assert!(near(vol(&e), v0 - std::f64::consts::PI * 9. * 20., 1e-3), "furo até o próximo {}", vol(&e));

        // rosca: comprimento 4 < face inteira
        let mut d = caixa();
        let top = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "20");
        d.add_feature(FeatureKind::Primitive(Primitive { kind: PrimKind::Cilindro, plane: PlaneRef::Datum(top), a: "5".into(), b: "10".into(), c: "0".into(), d: "0".into(), op: BoolOp::Add }));
        let th = |end: Option<Extent>| Thread { face: [5., 0., 25.], spec: "M10x1.5".into(), length: "4".into(), modeled: true, end, start: String::new() };
        let mut a = d.clone();
        a.add_feature(FeatureKind::Thread(th(None)));
        let mut b = d.clone();
        b.add_feature(FeatureKind::Thread(th(Some(Extent::ThroughAll))));
        let (va, vb) = (vol(&a), vol(&b));
        assert!(vb < va - 1., "rosca completa {vb} vs 4 mm {va}");
    }

    #[test]
    fn cache_reaproveita_linha_do_tempo() {
        let mut d = Document::new("c");
        fenkra_core::macros::run(&mut d, fenkra_core::macros::EXAMPLE).unwrap();
        let mut cache = Cache::default();
        let a = recompute_cached(&d, &mut cache);
        assert_eq!(cache.reused, 0);
        let b = recompute_cached(&d, &mut cache);
        assert_eq!(cache.reused, d.features.len());
        assert_eq!(a.status, b.status);
        if fenkra_kernel::AVAILABLE {
            assert!((a.bodies[0].mass.volume - b.bodies[0].mass.volume).abs() < 1e-6);
        }
        // renomear não recalcula nada
        let n = d.features.len();
        d.features[n - 1].name.push('x');
        let c = recompute_cached(&d, &mut cache);
        assert_eq!(cache.reused, n);
        if fenkra_kernel::AVAILABLE {
            assert!((a.bodies[0].mass.volume - c.bodies[0].mass.volume).abs() < 1e-6);
        }
    }

    #[test]
    fn qualidade_alta_gera_mais_triangulos_sem_refazer_comandos() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let mut d = Document::new("cilindro");
        d.add_feature(FeatureKind::Primitive(Primitive { kind: PrimKind::Cilindro, plane: PlaneRef::Origin(OriginPlane::XY), a: "20".into(), b: "40".into(), c: "0".into(), d: "0".into(), op: BoolOp::Add }));
        let tris = |r: &RecomputeResult| r.bodies[0].mesh.indices.len() / 3;
        let rascunho = recompute_cached(&d, &mut Cache::com_qualidade(QualidadeMalha::RASCUNHO));
        let alta = recompute_cached(&d, &mut Cache::com_qualidade(QualidadeMalha::ALTA));
        assert!(tris(&alta) > tris(&rascunho), "alta {} vs rascunho {}", tris(&alta), tris(&rascunho));
        // trocar a qualidade no mesmo cache só re-tessela: todos os comandos vêm prontos
        let mut cache = Cache::com_qualidade(QualidadeMalha::RASCUNHO);
        let a = recompute_cached(&d, &mut cache);
        cache.qualidade = QualidadeMalha::ALTA;
        let b = recompute_cached(&d, &mut cache);
        assert_eq!(cache.reused, d.features.len());
        assert_eq!(tris(&b), tris(&alta));
        assert!(tris(&b) > tris(&a));
        assert!((a.bodies[0].mass.volume - b.bodies[0].mass.volume).abs() < 1e-6);
    }

    #[test]
    fn cache_por_dependencia_refaz_so_o_ramo_editado() {
        let s = |x: &str| x.to_string();
        let mut d = Document::new("dois corpos");
        let a_col = d.active_collector;
        let ska = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [20., 20.]);
        let exa = d.add_feature(FeatureKind::Extrude(Extrude { sketch: ska, op: BoolOp::Add, extent: Extent::Blind(s("10")), reverse: false, taper: s("0"), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
        let b_col = d.add_collector("B");
        d.active_collector = b_col;
        let skb = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [50., 0.], [70., 20.]);
        d.add_param("HB", "5", Unit::Comprimento).unwrap();
        d.add_feature(FeatureKind::Extrude(Extrude { sketch: skb, op: BoolOp::Add, extent: Extent::Blind(s("HB")), reverse: false, taper: s("0"), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
        let _ = a_col;
        let mut cache = Cache::default();
        recompute_cached(&d, &mut cache);
        // editar a extrusão do corpo A: o sketch A, o sketch B e a extrusão B vêm do cache
        if let Some(FeatureKind::Extrude(e)) = d.feature_mut(exa).map(|f| &mut f.kind) {
            e.extent = Extent::Blind(s("15"));
        }
        let r = recompute_cached(&d, &mut cache);
        assert_eq!(cache.reused, 3, "só a extrusão editada recalcula");
        // mudar o parâmetro HB refaz só quem o usa (a extrusão B)
        d.params.iter_mut().find(|p| p.name == "HB").unwrap().expr = s("8");
        recompute_cached(&d, &mut cache);
        assert_eq!(cache.reused, 3, "parâmetro: só a extrusão B recalcula");
        if fenkra_kernel::AVAILABLE {
            let va = r.bodies.iter().find(|b| b.collector != b_col).unwrap().mass.volume;
            assert!((va - 20. * 20. * 15.).abs() < 1e-3, "{va}");
        }
    }

    #[test]
    fn macro_de_exemplo_recalcula() {
        let mut d = Document::new("m");
        fenkra_core::macros::run(&mut d, fenkra_core::macros::EXAMPLE).unwrap();
        if fenkra_kernel::AVAILABLE {
            let r = recompute(&d);
            for f in &d.features {
                assert_eq!(r.status[&f.id], Status::Ok, "{}: {:?}", f.name, r.status[&f.id]);
            }
            // placa 120 × 70 × 8 menos 4 furos Ø8 (com filete de 1 mm em tudo)
            let v = r.bodies[0].mass.volume;
            assert!(v < 120. * 70. * 8. && v > 120. * 70. * 8. - 4. * std::f64::consts::PI * 16. * 8. - 2000., "{v}");
        }
    }

    #[test]
    fn modos_extras_superficies_links_e_booleanas() {
        let mut d = Document::new("t");
        let s_ = |x: &str| x.to_string();
        // linha de interseção XY ∩ XZ = eixo X
        let li = d.add_geometry(RefKind::LinePlanes { a: PlaneRef::Origin(OriginPlane::XY), b: PlaneRef::Origin(OriginPlane::XZ), length: s_("40") });
        let p10 = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "10");
        let pm = d.add_geometry(RefKind::PlaneMid { a: PlaneRef::Origin(OriginPlane::XY), b: PlaneRef::Datum(p10) });
        let sp = d.add_geometry(RefKind::SplineControl { points: vec![PointRef::Origin, PointRef::Fixed([10., 10., 0.]), PointRef::Fixed([20., 0., 5.])], closed: false });
        let ci = d.add_geometry(RefKind::PointCurvePlane { curve: CurveRef::Ref(sp), plane: PlaneRef::Origin(OriginPlane::YZ), which: 0 });
        let off = d.add_geometry(RefKind::PointOffset { point: PointRef::Origin, dir: AxisRef::Origin(OriginAxis::Z), dist: s_("7") });
        let bis = d.add_geometry(RefKind::LineBisector { a: CurveRef::Axis(OriginAxis::X), b: CurveRef::Axis(OriginAxis::Y), length: s_("10") });
        let r = recompute(&d);
        for id in [li, pm, sp, ci, off, bis] {
            assert_eq!(r.status[&id], Status::Ok, "{}: {:?}", d.name_of(id), r.status[&id]);
        }
        let c = r.curves.iter().find(|c| c.id == li).unwrap();
        assert!(c.pts.iter().all(|p| p[1].abs() < 1e-9 && p[2].abs() < 1e-9));
        assert!((r.planes.iter().find(|p| p.id == Some(pm)).unwrap().frame.origin[2] - 5.).abs() < 1e-9);
        assert!(geo3d::dist(r.points.iter().find(|p| p.id == off).unwrap().p, [0., 0., 7.]) < 1e-9);
        let b = r.curves.iter().find(|c| c.id == bis).unwrap();
        let e = b.pts.last().unwrap();
        assert!((e[0].abs() - e[1].abs()).abs() < 1e-6);

        if !fenkra_kernel::AVAILABLE {
            return;
        }
        // superfícies: extrusão de curva, preenchimento, deslocamento
        let se = d.add_geometry(RefKind::SurfExtrude { curve: CurveRef::Ref(sp), dir: AxisRef::Origin(OriginAxis::Z), length: s_("10"), length2: "0".into(), until: None, until2: None });
        let so = d.add_geometry(RefKind::SurfOffset { surface: SurfRef::Ref(se), dist: s_("2"), both: false, join: OffsetJoin::Automatico });
        let sq = d.add_geometry(RefKind::Polyline { points: vec![PointRef::Origin, PointRef::Fixed([10., 0., 0.]), PointRef::Fixed([10., 10., 3.]), PointRef::Fixed([0., 10., 0.])], closed: true });
        let sf = d.add_geometry(RefKind::SurfFill { curves: vec![CurveRef::Ref(sq)], conds: vec![], points: vec![], tol: "0.01".into() });
        let pp = d.add_geometry(RefKind::PointProject { point: PointRef::Fixed([5., 5., 20.]), surface: SurfRef::Ref(sf) });
        let r = recompute(&d);
        for id in [se, so, sq, sf, pp] {
            assert_eq!(r.status[&id], Status::Ok, "{}: {:?}", d.name_of(id), r.status[&id]);
        }
        assert_eq!(r.surfaces.len(), 3);
        assert!(r.points.iter().find(|p| p.id == pp).unwrap().p[2] < 5.);

        // links: publica um plano e um parâmetro e importa noutra peça
        let mut src = example_document();
        let pl = src.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "33");
        src.references.iter_mut().find(|r| r.id == pl).unwrap().published = true;
        src.params[0].published = true;
        let pubs = publications(&src);
        assert_eq!(pubs.len(), 2);
        let mut dst = Document::new("dst");
        for (n, g) in pubs {
            dst.add_link("Placa", "placa.fkp", &n, g);
        }
        let r = recompute(&dst);
        assert!(dst.references.iter().all(|x| r.status[&x.id] == Status::Ok));
        assert!(r.planes.iter().any(|p| (p.frame.origin[2] - 33.).abs() < 1e-9));
        assert_eq!(dst.params.len(), 1);

        // booleana entre corpos e até o próximo
        let mut d = caixa();
        let c2 = d.add_collector("Ferramenta");
        d.active_collector = c2;
        d.add_feature(FeatureKind::Primitive(Primitive { kind: PrimKind::Cilindro, plane: PlaneRef::Origin(OriginPlane::XY), a: s_("5"), b: s_("40"), c: s_("0"), d: s_("0"), op: BoolOp::Add }));
        let c1 = d.collectors[0].id;
        d.active_collector = c1;
        d.add_feature(FeatureKind::Combine(Combine { tool: c2, op: BoolOp::Cut, keep_tool: false }));
        let r = recompute(&d);
        assert_eq!(r.bodies.len(), 1);
        assert!((r.bodies[0].mass.volume - (24000. - std::f64::consts::PI * 25. * 20.)).abs() < 1.);

        // filete completo: nervura 4 mm de largura vira meia-cana
        let mut d = Document::new("t");
        let s1 = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [-2., -20.], [2., 20.]);
        d.add_feature(ext(s1, BoolOp::Add, "10"));
        d.add_feature(FeatureKind::Fillet(EdgeOp {
            value: s_("1"),
            edges: EdgeSel::AllOfBody,
            mode: EdgeMode::FullRound { mid: [0., 0., 10.], a: [2., 0., 5.], b: [-2., 0., 5.] },
            prop: Default::default(),
            keys: vec![], extra: Default::default()
        }));
        let v = vol(&d);
        assert!(v < 4. * 40. * 10. && v > 4. * 40. * 8., "filete completo {v}");
    }

    fn sk_rect(d: &mut Document, plane: PlaneRef, a: [f64; 2], b: [f64; 2]) -> Id {
        let mut data = SketchData::default();
        let mut n = 1_000_000u64 + d.features.len() as u64 * 1000;
        let mut al = || {
            n += 1;
            Id(n)
        };
        data.add_rect(&mut al, a, b, false);
        d.add_feature(FeatureKind::Sketch(Sketch { plane, data }))
    }

    fn sk_lines(d: &mut Document, plane: PlaneRef, segs: &[([f64; 2], [f64; 2])]) -> Id {
        let mut data = SketchData::default();
        let mut n = 2_000_000u64 + d.features.len() as u64 * 1000;
        let mut al = || {
            n += 1;
            Id(n)
        };
        for (a, b) in segs {
            let pa = data.add_point(&mut al, a[0], a[1]);
            let pb = data.add_point(&mut al, b[0], b[1]);
            data.add_curve(&mut al, fenkra_core::sketch::Curve::Line { a: pa, b: pb }, false);
        }
        d.add_feature(FeatureKind::Sketch(Sketch { plane, data }))
    }

    fn ext(sketch: Id, op: BoolOp, d: &str) -> FeatureKind {
        FeatureKind::Extrude(Extrude { sketch, op, extent: Extent::Blind(d.into()), reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() })
    }

    /// Caixa 40 × 30 × 20 centrada na origem (base em z = 0).
    fn caixa() -> Document {
        let mut d = Document::new("t");
        let s = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [-20., -15.], [20., 15.]);
        d.add_feature(ext(s, BoolOp::Add, "20"));
        d
    }

    fn vol(d: &Document) -> f64 {
        let r = recompute(d);
        for f in &d.features {
            // sketch de linhas abertas (reforço) avisa que não há perfil fechado
            let sketch_warn = matches!(f.kind, FeatureKind::Sketch(_)) && matches!(r.status[&f.id], Status::Warning(_));
            assert!(r.status[&f.id] == Status::Ok || sketch_warn, "{}: {:?}", f.name, r.status[&f.id]);
        }
        r.bodies.iter().map(|b| b.mass.volume).sum()
    }

    #[test]
    fn features_de_forma_acabamento_e_padroes() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let near = |a: f64, b: f64, tol: f64| (a - b).abs() <= tol * b.abs().max(1.);
        let v0 = vol(&caixa());
        assert!(near(v0, 24000., 1e-6));

        // casca abrindo o topo
        let mut d = caixa();
        d.add_feature(FeatureKind::Shell(Shell { espessuras: vec![], faces: vec![[0., 0., 20.]], thickness: "2".into(), outward: false, face_keys: vec![], closed: false }));
        let v = vol(&d);
        assert!(v < v0 * 0.5 && v > 0., "casca {v}");

        // ângulo de saída numa face lateral
        let mut d = caixa();
        d.add_feature(FeatureKind::Draft(Draft { faces: vec![[20., 0., 10.]], neutral: PlaneRef::Origin(OriginPlane::XY), angle: "5".into(), reverse: false, face_keys: vec![] }));
        assert!(!near(vol(&d), v0, 1e-6), "saída sem efeito");

        // espessura +5 no topo
        let mut d = caixa();
        d.add_feature(FeatureKind::Thickness(Thickness { faces: vec![[0., 0., 20.]], value: "5".into(), face_keys: vec![], remove: false }));
        assert!(near(vol(&d), v0 + 6000., 1e-6));

        // padrão linear de um bolso 4×4×5: 3 cópias ao longo de X
        let mut d = caixa();
        let top = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "20");
        let sp = sk_rect(&mut d, PlaneRef::Datum(top), [-16., -2.], [-12., 2.]);
        let pocket = d.add_feature(ext(sp, BoolOp::Cut, "5"));
        let v1 = vol(&d);
        assert!(near(v1, v0 - 80., 1e-6));
        d.add_feature(FeatureKind::Pattern(Pattern {
            kind: PatternKind::Linear { dir: AxisRef::Origin(OriginAxis::X), count: "3".into(), spacing: "8".into(), dir2: None, count2: "1".into(), spacing2: "0".into(), total: false },
            sources: vec![pocket], skip: Default::default(),
        }));
        assert!(near(vol(&d), v0 - 240., 1e-6));
        // espelho do bolso pelo YZ e padrão circular de 4 em volta de Z
        let mut e = d.clone();
        e.add_feature(FeatureKind::Mirror(Mirror { plane: PlaneRef::Origin(OriginPlane::YZ), sources: vec![pocket] }));
        assert!(near(vol(&e), v0 - 320., 1e-6));
        let mut e = d.clone();
        e.features.pop();
        e.add_feature(FeatureKind::Pattern(Pattern { kind: PatternKind::Circular { axis: AxisRef::Origin(OriginAxis::Z), count: "2".into(), angle: "360".into() }, sources: vec![pocket], skip: Default::default() }));
        assert!(near(vol(&e), v0 - 160., 1e-6));

        // primitiva: cilindro r5 h10 sobre o topo
        let mut d = caixa();
        let top = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "20");
        d.add_feature(FeatureKind::Primitive(Primitive { kind: PrimKind::Cilindro, plane: PlaneRef::Datum(top), a: "5".into(), b: "10".into(), c: "0".into(), d: "0".into(), op: BoolOp::Add }));
        assert!(near(vol(&d), v0 + std::f64::consts::PI * 250., 1e-3));
        // rosca externa modelada no cilindro (M10)
        let mut e = d.clone();
        e.add_feature(FeatureKind::Thread(Thread { face: [5., 0., 25.], spec: "M10x1.5".into(), length: "8".into(), modeled: true, end: None, start: String::new() }));
        let vt = vol(&e);
        assert!(vt < vol(&d) - 1., "rosca externa {vt}");

        // escala ×2
        let mut d = caixa();
        d.add_feature(FeatureKind::Scale(Scale { center: PointRef::Origin, factor: "2".into() }));
        assert!(near(vol(&d), v0 * 8., 1e-6));

        // extrusão com ângulo e parede fina
        let mut d = Document::new("t");
        let s = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [-10., -10.], [10., 10.]);
        let id = d.add_feature(ext(s, BoolOp::Add, "10"));
        if let Some(FeatureKind::Extrude(x)) = d.feature_mut(id).map(|f| &mut f.kind) {
            x.taper = "5".into();
        }
        let v = vol(&d);
        assert!(v < 4000. && v > 3000., "cônica {v}");
        if let Some(FeatureKind::Extrude(x)) = d.feature_mut(id).map(|f| &mut f.kind) {
            x.taper = "0".into();
            x.thin = Some("2".into());
        }
        let v = vol(&d);
        assert!(near(v, 80. * 2. * 10., 0.02), "parede fina {v}");

        // multi-seções: quadrado 20 em z=0 até quadrado 10 em z=20
        let mut d = Document::new("t");
        let a = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [-10., -10.], [10., 10.]);
        let pl = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "20");
        let b = sk_rect(&mut d, PlaneRef::Datum(pl), [-5., -5.], [5., 5.]);
        d.add_feature(FeatureKind::Loft(Loft { sections: vec![a, b], ruled: true, closed: false, op: BoolOp::Add, torcao: vec![], inicio: Default::default(), fim: Default::default() }));
        // tronco de pirâmide: h/3 (A1 + A2 + sqrt(A1 A2))
        assert!(near(vol(&d), 20. / 3. * (400. + 100. + 200.), 1e-3));

        // nervura: perfil 4×4 no YZ varrido 50 mm ao longo de X
        let mut d = Document::new("t");
        let s = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::YZ), [-2., -2.], [2., 2.]);
        let ln = d.add_geometry(RefKind::LinePoints { a: PointRef::Origin, b: PointRef::Fixed([50., 0., 0.]), start: "0".into(), end: "0".into() });
        d.add_feature(FeatureKind::Sweep(Sweep { sketch: s, spine: CurveRef::Ref(ln), mode: SweepMode::Corrigido, op: BoolOp::Add, thin: None }));
        assert!(near(vol(&d), 16. * 50., 1e-3));

        // reforço: linha de 20 mm, espessura 2, profundidade 10 (não precisa de corpo)
        let mut d = Document::new("t");
        let s = sk_lines(&mut d, PlaneRef::Origin(OriginPlane::XY), &[([-10., 0.], [10., 0.])]);
        d.add_feature(FeatureKind::Stiffener(Stiffener { sketch: s, thickness: "2".into(), depth: "10".into(), reverse: true, to_body: false, side: 0 }));
        let v = vol(&d);
        assert!(near(v, (20. * 2. + std::f64::consts::PI) * 10., 1e-3), "reforço {v}");

        // filete variável e chanfro por distância e ângulo numa aresta do topo
        for mode in [EdgeMode::Variable("4".into()), EdgeMode::TwoDist("3".into()), EdgeMode::DistAngle("30".into())] {
            let mut d = caixa();
            let op = EdgeOp { value: "2".into(), edges: EdgeSel::Edges(vec![[0., 15., 20.]]), mode: mode.clone(), prop: Default::default(), keys: vec![], extra: Default::default() };
            d.add_feature(if matches!(mode, EdgeMode::Variable(_)) { FeatureKind::Fillet(op) } else { FeatureKind::Chamfer(op) });
            assert!(vol(&d) < v0, "{mode:?}");
        }
    }

    #[test]
    fn rosca_modelada_remove_material() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let mut d = example_document();
        let id = d.features.iter().find(|f| matches!(f.kind, FeatureKind::Hole(_))).unwrap().id;
        if let Some(FeatureKind::Hole(h)) = d.feature_mut(id).map(|f| &mut f.kind) {
            h.thread = Some("M6".into());
            h.diameter = "5".into();
            h.thread_modeled = false;
        }
        let v0 = recompute(&d).bodies[0].mass.volume;
        if let Some(FeatureKind::Hole(h)) = d.feature_mut(id).map(|f| &mut f.kind) {
            h.thread_modeled = true;
        }
        let r = recompute(&d);
        assert_eq!(r.status[&id], Status::Ok);
        assert!(r.bodies[0].mass.volume < v0 - 1., "a rosca não tirou material");
    }

    #[test]
    fn geometria_de_referencia_planos_pontos_curvas() {
        let mut d = Document::new("geo");
        let x = |v: f64| v.to_string();
        let p1 = d.add_geometry(RefKind::PointCoords { x: x(10.), y: x(0.), z: x(0.), frame: None });
        let p2 = d.add_geometry(RefKind::PointCoords { x: x(0.), y: x(10.), z: x(0.), frame: None });
        let p3 = d.add_geometry(RefKind::PointCoords { x: x(0.), y: x(0.), z: x(10.), frame: None });
        let pl = d.add_geometry(RefKind::PlaneThreePoints { a: PointRef::Ref(p1), b: PointRef::Ref(p2), c: PointRef::Ref(p3) });
        let ln = d.add_geometry(RefKind::LinePoints { a: PointRef::Ref(p1), b: PointRef::Ref(p2), start: "0".into(), end: "0".into() });
        let mid = d.add_geometry(RefKind::PointOnCurve { curve: CurveRef::Ref(ln), at: OnCurve::Ratio("0.5".into()), reverse: false });
        let nc = d.add_geometry(RefKind::PlaneNormalCurve { curve: CurveRef::Ref(ln), at: OnCurve::Ratio("0.5".into()), reverse: false, point: None, project: false });
        let ci = d.add_geometry(RefKind::Circle3 { a: PointRef::Ref(p1), b: PointRef::Ref(p2), c: PointRef::Ref(p3), full: true });
        let ce = d.add_geometry(RefKind::PointCenter { curve: CurveRef::Ref(ci) });
        let eq = d.add_geometry(RefKind::PlaneEquation { a: "0".into(), b: "0".into(), c: "2".into(), d: "10".into() });
        let hx = d.add_geometry(RefKind::Helix { origin: PointRef::Origin, axis: AxisRef::Origin(OriginAxis::Z), radius: "5".into(), pitch: "2".into(), height: "10".into(), taper: "0".into(), left: false });
        let ruim = d.add_geometry(RefKind::PlaneThreePoints { a: PointRef::Ref(p1), b: PointRef::Ref(p1), c: PointRef::Ref(p2) });
        let r = recompute(&d);
        for id in [p1, pl, ln, mid, nc, ci, ce, eq, hx] {
            assert_eq!(r.status[&id], Status::Ok, "{}", d.name_of(id));
        }
        assert!(matches!(r.status[&ruim], Status::Error(_)));
        let pt = |id: Id| r.points.iter().find(|p| p.id == id).unwrap().p;
        assert!(geo3d::dist(pt(mid), [5., 5., 0.]) < 1e-9);
        let c = 10. / 3.;
        assert!(geo3d::dist(pt(ce), [c, c, c]) < 1e-6);
        let fr = |id: Id| r.planes.iter().find(|p| p.id == Some(id)).unwrap().frame;
        assert!((fr(eq).origin[2] - 5.).abs() < 1e-9);
        let n = fr(pl).normal;
        assert!((n[0] - n[1]).abs() < 1e-9 && (n[1] - n[2]).abs() < 1e-9);
        // apagar o ponto leva junto o que depende dele (dependentes)
        assert!(d.dependents(p1).contains(&ce));
    }

    #[test]
    fn exemplo_recalcula_sem_erros() {
        let d = example_document();
        let r = recompute(&d);
        assert!(r.param_error.is_none());
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        for f in &d.features {
            assert_eq!(r.status[&f.id], Status::Ok, "{}: {:?}", f.name, r.status[&f.id]);
        }
        assert_eq!(r.bodies.len(), 1);
        let b = &r.bodies[0];
        assert!(b.mass.volume > 0.);
        // cada face tem dono; furo, filete e placa aparecem como donos
        assert_eq!(b.face_owner.len(), b.mesh.face_ranges.len());
        let owners: std::collections::HashSet<Id> = b.face_owner.iter().copied().collect();
        for name in ["Placa", "Furos M6", "Filete1"] {
            let id = d.features.iter().find(|f| f.name == name).unwrap().id;
            assert!(owners.contains(&id), "{name} sem faces");
        }
    }

    #[test]
    fn extrusao_ate_plano_inclinado_e_filete_em_aresta() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let mut d = example_document();
        // plano a 30° em torno do eixo X, deslocado 20 mm acima, e extrusão do sketch base até ele
        let base = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "20");
        let inclinado = d.add_angle_plane(PlaneRef::Datum(base), AxisRef::Origin(OriginAxis::X), "30");
        let sk = d.features[0].id;
        if let Some(FeatureKind::Extrude(e)) = d.feature_mut(d.features[1].id).map(|f| &mut f.kind) {
            assert_eq!(e.sketch, sk);
            e.extent = Extent::UpTo(PlaneRef::Datum(inclinado));
        }
        let r = recompute(&d);
        let placa = &d.features[1];
        assert_eq!(r.status[&placa.id], Status::Ok, "{:?}", r.status[&placa.id]);
        // filete só em uma aresta escolhida (aresta inferior frontal: y = -36, z = 0), pelo ponto
        // médio exato dela, como o picking entrega (sem chave o ponto tem que cair na aresta)
        let mut sem_filete = d.clone();
        sem_filete.features.retain(|f| !matches!(f.kind, FeatureKind::Fillet(_)));
        let r_sem = recompute(&sem_filete);
        let mid = r_sem.bodies[0]
            .mesh
            .edge_mids
            .chunks_exact(3)
            .map(|q| [q[0] as f64, q[1] as f64, q[2] as f64])
            .min_by(|a, b| geo3d::dist(*a, [0., -36., 0.]).total_cmp(&geo3d::dist(*b, [0., -36., 0.])))
            .unwrap();
        let fil = d.features.iter().find(|f| matches!(f.kind, FeatureKind::Fillet(_))).unwrap().id;
        if let Some(FeatureKind::Fillet(o)) = d.feature_mut(fil).map(|f| &mut f.kind) {
            o.edges = EdgeSel::Edges(vec![mid]);
        }
        let r = recompute(&d);
        assert_eq!(r.status[&fil], Status::Ok, "{:?}", r.status[&fil]);
    }

    /// Troca o retângulo de um sketch (a "cota" do perfil muda; o sketch e os comandos continuam os mesmos).
    fn set_rect(d: &mut Document, sk: Id, a: [f64; 2], b: [f64; 2]) {
        let mut data = SketchData::default();
        let mut n = 3_000_000u64;
        let mut al = || {
            n += 1;
            Id(n)
        };
        data.add_rect(&mut al, a, b, false);
        if let Some(FeatureKind::Sketch(s)) = d.feature_mut(sk).map(|f| &mut f.kind) {
            s.data = data;
        }
    }

    /// Caixa 40 × 30 × 20 (z de 0 a 20) com um bolso retangular `x × 10` de 5 mm de fundo aberto na
    /// face de cima. Devolve (documento, extrusão da caixa, sketch do bolso, recorte).
    fn caixa_com_bolso(meio_x: f64) -> (Document, Id, Id, Id) {
        let mut d = Document::new("t");
        let base = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [-20., -15.], [20., 15.]);
        let caixa = d.add_feature(ext(base, BoolOp::Add, "20"));
        let topo = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "20");
        let sk = sk_rect(&mut d, PlaneRef::Datum(topo), [-meio_x, -5.], [meio_x, 5.]);
        let corte = d.add_feature(ext(sk, BoolOp::Cut, "5"));
        (d, caixa, sk, corte)
    }

    /// Índice da aresta da malha cujo ponto médio está mais perto de `p` (o que o picking faz).
    fn aresta_perto(b: &BodyResult, p: [f64; 3]) -> usize {
        let d = |e: usize| (0..3).map(|i| (b.mesh.edge_mids[e * 3 + i] as f64 - p[i]).powi(2)).sum::<f64>();
        (0..b.mesh.edge_mids.len() / 3).min_by(|a, c| d(*a).total_cmp(&d(*c))).unwrap()
    }

    fn filete(valor: &str, mids: Vec<[f64; 3]>, keys: Vec<Option<EdgeKey>>) -> FeatureKind {
        FeatureKind::Fillet(EdgeOp { value: valor.into(), edges: EdgeSel::Edges(mids), mode: EdgeMode::Constant, prop: Default::default(), keys, extra: Default::default() })
    }

    #[test]
    fn filete_com_chave_acompanha_a_aresta_quando_o_perfil_do_recorte_muda() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (mut d, _caixa, sk, corte) = caixa_com_bolso(5.);
        let r0 = recompute(&d);
        assert_eq!(r0.status[&corte], Status::Ok, "{:?}", r0.status[&corte]);
        let v_sem = r0.bodies[0].mass.volume;
        // o que o picking faz: a aresta entre a tampa de cima da caixa e a parede do bolso em x = 5
        let alvo = [5., 0., 20.];
        let e = aresta_perto(&r0.bodies[0], alvo);
        let key = r0.bodies[0].edge_key(e).expect("aresta entre duas faces com papel conhecido");
        assert_eq!(key.a.kind, 0, "plano");
        assert_eq!(key.b.kind, 0, "plano");
        let donos = [key.a.owner, key.b.owner];
        assert!(donos.contains(&corte), "uma das faces é parede do recorte: {donos:?}");
        // baseline sem filete com o perfil mudado (a parede vai de x = 5 para x = 10)
        let mut d_sem = d.clone();
        set_rect(&mut d_sem, sk, [-10., -5.], [10., 5.]);
        let v_sem_mudado = recompute(&d_sem).bodies[0].mass.volume;

        let fil = d.add_feature(filete("1", vec![alvo], vec![Some(key)]));
        let r1 = recompute(&d);
        assert_eq!(r1.status[&fil], Status::Ok, "{:?}", r1.status[&fil]);
        let retirado = (1. - std::f64::consts::FRAC_PI_4) * 10.; // r² (1 − π/4) × comprimento da aresta
        assert!((v_sem - r1.bodies[0].mass.volume - retirado).abs() < 0.2, "{v_sem} -> {}", r1.bodies[0].mass.volume);

        // muda a cota do perfil do recorte: a aresta anda 5 mm; o filete a segue (mesma aresta lógica)
        set_rect(&mut d, sk, [-10., -5.], [10., 5.]);
        let r2 = recompute(&d);
        assert_eq!(r2.status[&fil], Status::Ok, "{:?}", r2.status[&fil]);
        let v_com = r2.bodies[0].mass.volume;
        assert!(v_com < v_sem_mudado, "o filete tem que tirar material: {v_sem_mudado} -> {v_com}");
        assert!((v_sem_mudado - v_com - retirado).abs() < 0.2, "só a aresta lógica foi arredondada: {v_sem_mudado} -> {v_com}");

        // sem a chave (referência antiga por ponto) o mesmo cenário falha com erro claro, não em silêncio
        let (mut d_antigo, _, sk2, _) = caixa_com_bolso(5.);
        let fil2 = d_antigo.add_feature(filete("1", vec![alvo], vec![]));
        assert_eq!(recompute(&d_antigo).status[&fil2], Status::Ok);
        set_rect(&mut d_antigo, sk2, [-10., -5.], [10., 5.]);
        let st = recompute(&d_antigo).status[&fil2].clone();
        assert!(matches!(&st, Status::Error(m) if m.contains("existe mais")), "{st:?}");
    }

    #[test]
    fn chave_sem_candidata_e_erro_referencia_perdida() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (mut d, _, _, corte) = caixa_com_bolso(5.);
        let r0 = recompute(&d);
        let e = aresta_perto(&r0.bodies[0], [5., 0., 20.]);
        let mut key = r0.bodies[0].edge_key(e).unwrap();
        // dono que não existe mais: nenhuma face candidata
        key.b.owner = Id(987_654);
        let fil = d.add_feature(filete("1", vec![[5., 0., 20.]], vec![Some(key.clone())]));
        let v_sem = r0.bodies[0].mass.volume;
        let r = recompute(&d);
        match &r.status[&fil] {
            Status::Error(m) => assert!(m.starts_with("referência perdida:") && m.contains("filete"), "{m}"),
            s => panic!("deveria ficar em erro: {s:?}"),
        }
        // o comando com erro não apaga nada: o corpo fica como estava
        assert!((r.bodies[0].mass.volume - v_sem).abs() < 1e-6);
        // papel que o dono nunca teve
        key.b.owner = corte;
        key.b.role = 99;
        if let Some(FeatureKind::Fillet(o)) = d.feature_mut(fil).map(|f| &mut f.kind) {
            o.keys = vec![Some(key)];
        }
        assert!(matches!(&recompute(&d).status[&fil], Status::Error(m) if m.starts_with("referência perdida:")));

        // o mesmo vale para as faces de casca, ângulo de saída e espessura
        let (mut d, caixa, _, _) = caixa_com_bolso(5.);
        let falsa = FaceKey { owner: caixa, role: 77, kind: 0, normal_or_axis: [0., 0., 1.], area: 1. };
        let sh = d.add_feature(FeatureKind::Shell(Shell { espessuras: vec![], faces: vec![[0., 0., 20.]], thickness: "2".into(), outward: false, face_keys: vec![Some(falsa)], closed: false }));
        let r = recompute(&d);
        assert!(matches!(&r.status[&sh], Status::Error(m) if m.starts_with("referência perdida:") && m.contains("casca")), "{:?}", r.status[&sh]);
    }

    #[test]
    fn arquivo_antigo_sem_chaves_continua_igual_e_e_promovido() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (mut d, _, sk, _) = caixa_com_bolso(5.);
        let alvo = [5., 0., 20.];
        let fil = d.add_feature(filete("1", vec![alvo], vec![]));
        let r = recompute(&d);
        assert_eq!(r.status[&fil], Status::Ok, "{:?}", r.status[&fil]);
        let v_antigo = r.bodies[0].mass.volume;
        // o recompute calcula a chave para o app gravar
        let Some(Promoted::Edges(chaves)) = r.promoted.get(&fil).cloned() else { panic!("deveria promover: {:?}", r.promoted.keys().collect::<Vec<_>>()) };
        assert_eq!(chaves.len(), 1);
        assert!(chaves[0].is_some());
        // sem mexer no documento, o resultado de cada recompute é o mesmo (e o cache devolve a promoção)
        let mut cache = Cache::default();
        let a = recompute_cached(&d, &mut cache);
        let b = recompute_cached(&d, &mut cache);
        assert!(cache.reused > 0);
        assert_eq!(a.promoted.get(&fil), b.promoted.get(&fil));
        assert!((b.bodies[0].mass.volume - v_antigo).abs() < 1e-9);
        // gravada a promoção, o resultado é idêntico e o filete passa a acompanhar o perfil
        if let Some(FeatureKind::Fillet(o)) = d.feature_mut(fil).map(|f| &mut f.kind) {
            o.keys = chaves;
        }
        let r = recompute(&d);
        assert!((r.bodies[0].mass.volume - v_antigo).abs() < 1e-9);
        assert!(r.promoted.is_empty(), "nada novo para promover");
        set_rect(&mut d, sk, [-10., -5.], [10., 5.]);
        assert_eq!(recompute(&d).status[&fil], Status::Ok);
    }

    #[test]
    fn casca_com_chave_de_face_sobrevive_a_mudanca_de_altura() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let mut d = Document::new("t");
        let base = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [-20., -15.], [20., 15.]);
        let caixa = d.add_feature(ext(base, BoolOp::Add, "20"));
        let r0 = recompute(&d);
        let b = &r0.bodies[0];
        // tampa final da caixa = face de cima
        let topo = (0..b.face_role.len()).find(|i| b.face_owner[*i] == caixa && b.face_role[*i] == 1).expect("tampa final");
        let key = b.face_key(topo).unwrap();
        assert_eq!(key.normal_or_axis, [0., 0., 1.]);
        let sh = d.add_feature(FeatureKind::Shell(Shell { espessuras: vec![], faces: vec![[0., 0., 20.]], thickness: "2".into(), outward: false, face_keys: vec![Some(key)], closed: false }));
        let r = recompute(&d);
        assert_eq!(r.status[&sh], Status::Ok, "{:?}", r.status[&sh]);
        // a caixa fica mais alta: o ponto clicado (z = 20) já não está na face de cima, a chave sim
        if let Some(FeatureKind::Extrude(e)) = d.feature_mut(caixa).map(|f| &mut f.kind) {
            e.extent = Extent::Blind("30".into());
        }
        let r = recompute(&d);
        assert_eq!(r.status[&sh], Status::Ok, "{:?}", r.status[&sh]);
        let esperado = 40. * 30. * 30. - 36. * 26. * 28.; // casca de 2 mm aberta em cima
        assert!((r.bodies[0].mass.volume - esperado).abs() < 1.0, "{} vs {esperado}", r.bodies[0].mass.volume);
        // arquivo antigo (só o ponto) segue como antes, e a face sem chave é promovida
        let mut d = Document::new("t");
        let base = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [-20., -15.], [20., 15.]);
        d.add_feature(ext(base, BoolOp::Add, "20"));
        let sh = d.add_feature(FeatureKind::Shell(Shell { espessuras: vec![], faces: vec![[0., 0., 20.]], thickness: "2".into(), outward: false, face_keys: vec![], closed: false }));
        let r = recompute(&d);
        assert_eq!(r.status[&sh], Status::Ok);
        assert!(matches!(r.promoted.get(&sh), Some(Promoted::Faces(k)) if k.len() == 1 && k[0].is_some()));
    }

    #[test]
    fn referencia_perdida_nao_apaga_comando() {
        let mut d = example_document();
        let sk = d.features[0].id;
        d.delete(sk, false);
        let r = recompute(&d);
        let ex = &d.features[0];
        assert!(matches!(&r.status[&ex.id], Status::Error(m) if m.contains("referência perdida")));
    }

    #[test]
    fn parametro_muda_volume() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let mut d = example_document();
        let v1 = recompute(&d).bodies[0].mass.volume;
        d.params[2].expr = "20".into(); // esp
        let v2 = recompute(&d).bodies[0].mass.volume;
        assert!(v2 > v1 * 1.8);
    }
}

#[cfg(test)]
mod tests_spline_perfil {
    use super::*;
    use fenkra_core::sketch::Curve;

    fn doc_with(shared: bool) -> (Document, Id) {
        let mut d = Document::new("t");
        let mut data = SketchData::default();
        let mut n = 5_000_000u64;
        let mut al = || {
            n += 1;
            Id(n)
        };
        let p0 = data.add_point(&mut al, 0., 0.);
        let p1 = data.add_point(&mut al, 0., 50.);
        let p2 = data.add_point(&mut al, 35., 50.);
        let p3 = data.add_point(&mut al, 48., 20.);
        let p4 = data.add_point(&mut al, 50., 0.);
        data.add_curve(&mut al, Curve::Line { a: p4, b: p0 }, false);
        data.add_curve(&mut al, Curve::Line { a: p0, b: p1 }, false);
        if shared {
            data.add_curve(&mut al, Curve::Spline { pts: vec![p1, p2, p3, p4] }, false);
        } else {
            let q1 = data.add_point(&mut al, 0., 50.);
            let q4 = data.add_point(&mut al, 50., 0.);
            data.add_curve(&mut al, Curve::Spline { pts: vec![q1, p2, p3, q4] }, false);
        }
        let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
        let e = d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind("10".into()), reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
        (d, e)
    }

    #[test]
    fn perfil_com_spline_fecha() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        for shared in [true, false] {
            let (d, e) = doc_with(shared);
            let r = recompute(&d);
            assert_eq!(r.status[&e], Status::Ok, "shared={shared}: {:?}", r.status[&e]);
            assert!(r.bodies[0].mass.volume > 15000., "{}", r.bodies[0].mass.volume);
        }
    }
}

#[cfg(test)]
mod tests_abas_vizinhas {
    use super::*;

    #[test]
    fn duas_abas_vizinhas_planificam() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let s = |x: &str| x.to_string();
        for off in ["0", "0.0001"] {
            let mut d = Document::new("caixa");
            d.sheet = Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), flat: false, ..Default::default() });
            let mut data = SketchData::default();
            let mut n = 6_100_000u64;
            let mut al = || {
                n += 1;
                Id(n)
            };
            data.add_rect(&mut al, [0., 0.], [50., 50.], false);
            let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
            d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }));
            let fl = |e: [f64; 3]| FeatureKind::SmFlange(SmFlange { edge: e, length: s("20"), angle: s("90"), radius: None, off1: s(off), off2: s(off), flip: false, relief: None, relief_kind: 0, side: 0, miter: false, min_gap: true, edge_key: None });
            d.add_feature(fl([50., 25., 2.]));
            d.add_feature(fl([25., 50., 2.]));
            for flat in [false, true] {
                d.sheet.as_mut().unwrap().flat = flat;
                let r = recompute(&d);
                let bad: Vec<String> = d.features.iter().filter(|f| !matches!(r.status[&f.id], Status::Ok)).map(|f| format!("{}: {:?}", f.name, r.status[&f.id])).collect();
                assert!(bad.is_empty(), "{bad:?}");
                let v = r.bodies[0].mass.volume;
                // planificada com o mesmo volume da dobrada (só o canto sobreposto de 2×2×20 a menos)
                assert!((v - 9942.).abs() < 60., "off={off} flat={flat} vol={v}");
            }
        }
    }
}

#[cfg(test)]
mod tests_aba_curva {
    use super::*;

    #[test]
    fn aba_curva_com_raio_e_planificada() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let s = |x: &str| x.to_string();
        let mut d = Document::new("curva");
        d.sheet = Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), flat: false, ..Default::default() });
        let mut data = SketchData::default();
        let mut n = 6_200_000u64;
        let mut al = || {
            n += 1;
            Id(n)
        };
        // quarto de disco R50 com a borda curva
        let o = data.add_point(&mut al, 0., 0.);
        let a = data.add_point(&mut al, 50., 0.);
        let b = data.add_point(&mut al, 0., 50.);
        data.add_curve(&mut al, fenkra_core::sketch::Curve::Line { a: o, b: a }, false);
        data.add_curve(&mut al, fenkra_core::sketch::Curve::Arc { c: o, s: a, e: b }, false);
        data.add_curve(&mut al, fenkra_core::sketch::Curve::Line { a: b, b: o }, false);
        let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
        d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }));
        let edge: Vec<[f64; 3]> = (0..=24).map(|i| {
            let t = i as f64 / 24. * std::f64::consts::FRAC_PI_2;
            [50. * t.cos(), 50. * t.sin(), 2.]
        }).collect();
        d.add_feature(FeatureKind::SmCurveFlange(SmCurveFlange { edge, length: s("10"), flip: false, angle: s("90"), radius: None, side: 0, edge_key: None }));
        let r = recompute(&d);
        for f in &d.features {
            assert_eq!(r.status[&f.id], Status::Ok, "{}: {:?}", f.name, r.status[&f.id]);
        }
        let base = std::f64::consts::PI * 2500. / 4. * 2.;
        let vf = r.bodies[0].mass.volume;
        assert!(vf > base + 900., "dobrada {vf}");
        let zmax = r.bodies[0].mesh.positions.chunks_exact(3).map(|p| p[2]).fold(f32::MIN, f32::max);
        assert!((zmax - 14.).abs() < 0.1, "altura da aba {zmax}");
        d.sheet.as_mut().unwrap().flat = true;
        let r = recompute(&d);
        for f in &d.features {
            assert_eq!(r.status[&f.id], Status::Ok, "plana {}: {:?}", f.name, r.status[&f.id]);
        }
        let zmax = r.bodies[0].mesh.positions.chunks_exact(3).map(|p| p[2]).fold(f32::MIN, f32::max);
        assert!(zmax < 2.01, "planificada é plana {zmax}");
        let xmax = r.bodies[0].mesh.positions.chunks_exact(3).map(|p| p[0]).fold(f32::MIN, f32::max);
        let dev = 50. + std::f64::consts::FRAC_PI_2 * (2. + 0.44 * 2.) + 10.;
        assert!((xmax as f64 - dev).abs() < 0.5, "desenvolvido {xmax} ≈ {dev}");
        // lado do material: a face externa da aba fica em R = 50 + r + t, 50 + t ou 50
        d.sheet.as_mut().unwrap().flat = false;
        for (side, want) in [(0u8, 54.), (1, 52.), (2, 50.)] {
            if let FeatureKind::SmCurveFlange(cf) = &mut d.features.last_mut().unwrap().kind {
                cf.side = side;
            }
            let r = recompute(&d);
            for f in &d.features {
                assert_eq!(r.status[&f.id], Status::Ok, "lado {side} {}: {:?}", f.name, r.status[&f.id]);
            }
            let xmax = r.bodies[0].mesh.positions.chunks_exact(3).map(|p| p[0]).fold(f32::MIN, f32::max) as f64;
            assert!((xmax - want).abs() < 0.3, "lado {side}: externo {xmax} ≈ {want}");
        }
    }
}

#[cfg(test)]
mod tests_filete_planificado {
    use super::*;

    #[test]
    fn filete_na_ponta_da_aba_vizinha_planifica() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let s = |x: &str| x.to_string();
        let mut d = Document::new("caixa");
        d.sheet = Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), flat: false, ..Default::default() });
        let mut data = SketchData::default();
        let mut n = 6_300_000u64;
        let mut al = || {
            n += 1;
            Id(n)
        };
        data.add_rect(&mut al, [0., 0.], [50., 50.], false);
        let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
        d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }));
        let fl = |e: [f64; 3]| FeatureKind::SmFlange(SmFlange { edge: e, length: s("20"), angle: s("90"), radius: None, off1: s("0"), off2: s("0"), flip: false, relief: None, relief_kind: 0, side: 0, miter: false, min_gap: true, edge_key: None });
        d.add_feature(fl([50., 25., 2.]));
        d.add_feature(fl([25., 50., 2.]));
        let r = recompute(&d);
        // aresta da ponta da 2ª aba, junto da 1ª: (x ≈ 50, y = 52..54, z = 24)
        let m = &r.bodies[0].mesh;
        let mut best = (f64::MAX, [0.; 3]);
        for q in m.edge_mids.chunks_exact(3) {
            let p = [q[0] as f64, q[1] as f64, q[2] as f64];
            let dd = ((p[0] - 50.).powi(2) + (p[1] - 53.).powi(2) + (p[2] - 24.).powi(2)).sqrt();
            if dd < best.0 {
                best = (dd, p);
            }
        }
        assert!(best.0 < 0.5, "aresta da ponta {:?}", best);
        let v0 = r.bodies[0].mass.volume;
        d.add_feature(FeatureKind::Fillet(EdgeOp { value: s("3"), edges: EdgeSel::Edges(vec![best.1]), mode: Default::default(), prop: Default::default(), keys: vec![], extra: Default::default() }));
        let r = recompute(&d);
        let v1 = r.bodies[0].mass.volume;
        assert!(v1 < v0 - 1., "filete dobrado {v0} {v1}");
        d.sheet.as_mut().unwrap().flat = true;
        d.features.last_mut().unwrap().flat_space = false;
        let r = recompute(&d);
        let bad: Vec<String> = d.features.iter().filter(|f| !matches!(r.status[&f.id], Status::Ok)).map(|f| format!("{}: {:?}", f.name, r.status[&f.id])).collect();
        assert!(bad.is_empty(), "{bad:?}");
        let vf = r.bodies[0].mass.volume;
        // o filete continua no planificado: o volume cai o mesmo tanto
        let d2 = {
            let mut x = d.clone();
            x.features.pop();
            x
        };
        let vf0 = recompute(&d2).bodies[0].mass.volume;
        assert!((vf0 - vf - (v0 - v1)).abs() < 0.5, "filete planificado: {vf0} {vf} dobrado {}", v0 - v1);
    }

    #[test]
    fn filete_na_ponta_da_primeira_aba_planifica() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let s = |x: &str| x.to_string();
        let mut d = Document::new("caixa");
        d.sheet = Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), flat: false, ..Default::default() });
        let mut data = SketchData::default();
        let mut n = 6_400_000u64;
        let mut al = || {
            n += 1;
            Id(n)
        };
        data.add_rect(&mut al, [0., 0.], [50., 50.], false);
        let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
        d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }));
        let fl = |e: [f64; 3]| FeatureKind::SmFlange(SmFlange { edge: e, length: s("20"), angle: s("90"), radius: None, off1: s("0"), off2: s("0"), flip: false, relief: None, relief_kind: 0, side: 0, miter: false, min_gap: true, edge_key: None });
        d.add_feature(fl([50., 25., 2.]));
        d.add_feature(fl([25., 50., 2.]));
        let r = recompute(&d);
        // aresta da ponta da 2ª aba, junto da 1ª: (x ≈ 50, y = 52..54, z = 24)
        let m = &r.bodies[0].mesh;
        let mut best = (f64::MAX, [0.; 3]);
        for q in m.edge_mids.chunks_exact(3) {
            let p = [q[0] as f64, q[1] as f64, q[2] as f64];
            let dd = ((p[0] - 53.).powi(2) + (p[1] - 50.).powi(2) + (p[2] - 24.).powi(2)).sqrt();
            if dd < best.0 {
                best = (dd, p);
            }
        }
        assert!(best.0 < 0.5, "aresta da ponta {:?}", best);
        let v0 = r.bodies[0].mass.volume;
        d.add_feature(FeatureKind::Fillet(EdgeOp { value: s("3"), edges: EdgeSel::Edges(vec![best.1]), mode: Default::default(), prop: Default::default(), keys: vec![], extra: Default::default() }));
        let r = recompute(&d);
        let v1 = r.bodies[0].mass.volume;
        assert!(v1 < v0 - 1., "filete dobrado {v0} {v1}");
        d.sheet.as_mut().unwrap().flat = true;
        d.features.last_mut().unwrap().flat_space = false;
        let r = recompute(&d);
        let bad: Vec<String> = d.features.iter().filter(|f| !matches!(r.status[&f.id], Status::Ok)).map(|f| format!("{}: {:?}", f.name, r.status[&f.id])).collect();
        assert!(bad.is_empty(), "{bad:?}");
        let vf = r.bodies[0].mass.volume;
        // o filete continua no planificado: o volume cai o mesmo tanto
        let d2 = {
            let mut x = d.clone();
            x.features.pop();
            x
        };
        let vf0 = recompute(&d2).bodies[0].mass.volume;
        assert!((vf0 - vf - (v0 - v1)).abs() < 0.5, "filete planificado: {vf0} {vf} dobrado {}", v0 - v1);
    }
}

#[cfg(test)]
mod tests_aba_superficie {
    use super::*;

    #[test]
    fn aba_por_superficie_relimita_e_sobe() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let s = |x: &str| x.to_string();
        let mut d = Document::new("web");
        d.sheet = Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), flat: false, ..Default::default() });
        let mut data = SketchData::default();
        let mut n = 6_500_000u64;
        let mut al = || {
            n += 1;
            Id(n)
        };
        data.add_rect(&mut al, [0., 0.], [100., 50.], false);
        let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
        d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }));
        // suporte: plano vertical em x = 80
        let pl = d.add_offset_plane(PlaneRef::Origin(OriginPlane::YZ), "80");
        d.add_feature(FeatureKind::SmSurfFlange(SmSurfFlange { support: SurfRef::Plane(PlaneRef::Datum(pl)), face: [40., 25., 2.], length: s("15"), radius: None, flip: false, keep_base: false, mode: 0, angle: s("90") }));
        let r = recompute(&d);
        for f in &d.features {
            assert_eq!(r.status[&f.id], Status::Ok, "{}: {:?}", f.name, r.status[&f.id]);
        }
        let ps = &r.bodies[0].mesh.positions;
        let zmax = ps.chunks_exact(3).map(|p| p[2]).fold(f32::MIN, f32::max);
        let xmax = ps.chunks_exact(3).map(|p| p[0]).fold(f32::MIN, f32::max);
        assert!((zmax - 19.).abs() < 0.2, "a aba sobe 15 depois do raio {zmax}");
        assert!(xmax < 85., "a web foi relimitada pelo suporte {xmax}");
        let ymax = ps.chunks_exact(3).map(|p| p[1]).fold(f32::MIN, f32::max);
        assert!(ymax < 50.1, "a aba fica na largura da web {ymax}");
        d.sheet.as_mut().unwrap().flat = true;
        let r = recompute(&d);
        for f in &d.features {
            assert_eq!(r.status[&f.id], Status::Ok, "plana {}: {:?}", f.name, r.status[&f.id]);
        }
        let zmax = r.bodies[0].mesh.positions.chunks_exact(3).map(|p| p[2]).fold(f32::MIN, f32::max);
        assert!(zmax < 2.01, "planificada {zmax}");
        // aproximação (suporte plano: igual ao exato) e ângulo constante de 45°
        d.sheet.as_mut().unwrap().flat = false;
        for (mode, ang, lo, hi) in [(1u8, "90", 18.8, 19.2), (2, "45", 8., 16.)] {
            if let FeatureKind::SmSurfFlange(sf) = &mut d.features.last_mut().unwrap().kind {
                sf.mode = mode;
                sf.angle = ang.into();
            }
            let r = recompute(&d);
            for f in &d.features {
                assert_eq!(r.status[&f.id], Status::Ok, "modo {mode} {}: {:?}", f.name, r.status[&f.id]);
            }
            let zmax = r.bodies[0].mesh.positions.chunks_exact(3).map(|p| p[2]).fold(f32::MIN, f32::max) as f64;
            assert!(zmax > lo && zmax < hi, "modo {mode}: altura {zmax}");
        }
    }
}

#[cfg(test)]
mod tests_dobra_sketch {
    use super::*;

    fn sk_rect(d: &mut Document, a: [f64; 2], b: [f64; 2]) -> Id {
        let mut data = SketchData::default();
        let mut n = 7_000_000u64;
        let mut al = || {
            n += 1;
            Id(n)
        };
        data.add_rect(&mut al, a, b, false);
        d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }))
    }

    fn sk_lines(d: &mut Document, plane: PlaneRef, segs: &[([f64; 2], [f64; 2])]) -> Id {
        let mut data = SketchData::default();
        let mut n = 7_100_000u64 + d.features.len() as u64 * 1000;
        let mut al = || {
            n += 1;
            Id(n)
        };
        for (a, b) in segs {
            let pa = data.add_point(&mut al, a[0], a[1]);
            let pb = data.add_point(&mut al, b[0], b[1]);
            data.add_curve(&mut al, fenkra_core::sketch::Curve::Line { a: pa, b: pb }, false);
        }
        d.add_feature(FeatureKind::Sketch(Sketch { plane, data }))
    }

    /// Sketch com uma spline pelos pontos dados (coordenadas do plano).
    fn sk_spline(d: &mut Document, plane: PlaneRef, pts: &[[f64; 2]]) -> Id {
        let mut data = SketchData::default();
        let mut n = 3_000_000u64 + d.features.len() as u64 * 1000;
        let mut al = || {
            n += 1;
            Id(n)
        };
        let ids: Vec<Id> = pts.iter().map(|p| data.add_point(&mut al, p[0], p[1])).collect();
        data.add_curve(&mut al, fenkra_core::sketch::Curve::Spline { pts: ids }, false);
        d.add_feature(FeatureKind::Sketch(Sketch { plane, data }))
    }

    /// B03 e B77: dobra por linha com várias linhas de sketch, posição da dobra e spline quase reta;
    /// todas registram no mapa de dobras e as linhas da planificada caem onde o desenvolvimento manda.
    #[test]
    fn dobra_por_linha_sketch_posicao_e_mapa() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        use std::f64::consts::PI;
        let s = |x: &str| x.to_string();
        let ba = PI / 2. * (2. + 0.44 * 2.);
        let base = || {
            let mut d = Document::new("chapa");
            d.sheet = Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), flat: false, ..Default::default() });
            let sk = sk_rect(&mut d, [0., 0.], [100., 50.]);
            d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }));
            d
        };
        let bend = |sketch: Option<Id>, x: f64, position: u8| SmBend { a: [x, 0., 2.], b: [x, 50., 2.], angle: s("90"), radius: None, flip: false, down: false, sketch, position };
        let cx = |l: &BendLine| (l.center[0][0] + l.center[1][0]) / 2.;
        // duas linhas paralelas num sketch: duas dobras, volume conservado
        let mut d = base();
        let sk = sk_lines(&mut d, PlaneRef::Origin(OriginPlane::XY), &[([70., 0.], [70., 50.]), ([30., 0.], [30., 50.])]);
        let bid = d.add_feature(FeatureKind::SmBend(bend(Some(sk), 0., 0)));
        let r = recompute(&d);
        assert_eq!(r.status[&bid], Status::Ok, "{:?}", r.status[&bid]);
        assert!(((r.bodies[0].mass.volume - 10000.) / 10000.).abs() < 6e-3, "volume {}", r.bodies[0].mass.volume);
        assert_eq!(r.bend_lines.len(), 2, "uma linha de dobra por linha do sketch");
        assert!(r.bend_lines.iter().all(|l| (l.angle - 90.).abs() < 1e-6 && (l.radius - 2.).abs() < 1e-6));
        // planificada: as linhas voltam aos lugares do sketch (centro da zona) e a tabela soma as duas
        d.sheet.as_mut().unwrap().flat = true;
        let r = recompute(&d);
        assert!(matches!(r.status[&bid], Status::Warning(_)), "{:?}", r.status[&bid]);
        assert_eq!(r.bend_lines.len(), 2);
        let mut xs: Vec<f64> = r.bend_lines.iter().map(cx).collect();
        xs.sort_by(f64::total_cmp);
        assert!((xs[0] - 30.).abs() < 0.05 && (xs[1] - 70.).abs() < 0.05, "linhas planificadas {xs:?}");
        // posição: início da zona = a linha marca onde a dobra começa (centro recua metade do BA)
        for (pos, esperado) in [(1u8, 60. - ba / 2.), (2, 60. + ba / 2.), (3, 60. - ba / 2. + 4.)] {
            let mut d = base();
            d.sheet.as_mut().unwrap().flat = true;
            let id = d.add_feature(FeatureKind::SmBend(bend(None, 60., pos)));
            let r = recompute(&d);
            assert!(matches!(r.status[&id], Status::Warning(_)), "posição {pos}: {:?}", r.status[&id]);
            assert!((cx(&r.bend_lines[0]) - esperado).abs() < 0.05, "posição {pos}: {} esperado {esperado}", cx(&r.bend_lines[0]));
        }
        // spline quase reta: vira a corda, com aviso, e dobra
        let mut d = base();
        let sk = sk_spline(&mut d, PlaneRef::Origin(OriginPlane::XY), &[[50., 0.], [50.4, 25.], [50., 50.]]);
        let id = d.add_feature(FeatureKind::SmBend(bend(Some(sk), 0., 0)));
        let r = recompute(&d);
        assert!(matches!(&r.status[&id], Status::Warning(m) if m.contains("corda")), "{:?}", r.status[&id]);
        assert_eq!(r.bend_lines.len(), 1);
        // spline bem curva: a parede acompanha a linha e a linha de dobra fica curva
        let mut d = base();
        let sk = sk_spline(&mut d, PlaneRef::Origin(OriginPlane::XY), &[[50., 0.], [65., 25.], [50., 50.]]);
        let id = d.add_feature(FeatureKind::SmBend(bend(Some(sk), 0., 0)));
        let r = recompute(&d);
        assert!(matches!(&r.status[&id], Status::Warning(m) if m.contains("curva")), "{:?}", r.status[&id]);
        assert_eq!(r.bend_lines.len(), 1);
        assert!(r.bend_lines[0].curva.len() > 2, "linha de dobra curva");
        // duas curvas fortes num comando: recusa com a causa
        let mut d = base();
        let sk = sk_spline(&mut d, PlaneRef::Origin(OriginPlane::XY), &[[50., 0.], [65., 25.], [50., 50.]]);
        let sk2 = sk_spline(&mut d, PlaneRef::Origin(OriginPlane::XY), &[[70., 0.], [85., 25.], [70., 50.]]);
        let FeatureKind::Sketch(s2) = d.feature(sk2).unwrap().kind.clone() else { panic!() };
        let FeatureKind::Sketch(s1) = &mut d.feature_mut(sk).unwrap().kind else { panic!() };
        let mut todos = s1.data.clone();
        todos.points.extend(s2.data.points.iter().cloned());
        todos.curves.extend(s2.data.curves.iter().cloned());
        s1.data = todos;
        let id = d.add_feature(FeatureKind::SmBend(bend(Some(sk), 0., 0)));
        let r = recompute(&d);
        assert!(matches!(&r.status[&id], Status::Error(m) if m.contains("um comando para cada")), "{:?}", r.status[&id]);
        // linhas coladas: a zona dobrada não cabe
        let mut d = base();
        let sk = sk_lines(&mut d, PlaneRef::Origin(OriginPlane::XY), &[([50., 0.], [50., 50.]), ([52., 0.], [52., 50.])]);
        let id = d.add_feature(FeatureKind::SmBend(bend(Some(sk), 0., 0)));
        let r = recompute(&d);
        assert!(matches!(&r.status[&id], Status::Error(m) if m.contains("zona dobrada")), "{:?}", r.status[&id]);
        // compensação única: a tabela de dobras usa a mesma conta
        let c = chapa::compensacao(2., 2., 90., 0.44);
        assert!((c.ba - ba).abs() < 1e-12 && (c.deducao - (2. * 4. - ba)).abs() < 1e-9);
    }
}
