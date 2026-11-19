// SPDX-License-Identifier: LGPL-2.1-or-later
//! Converter sólido em chapa (B35): reconhece a espessura constante e as dobras de um corpo.
//!
//! Um corpo de chapa é uma casca de espessura `t` constante. O reconhecimento usa só a topologia
//! da malha (tipo, normal, área e ponto de cada face):
//!
//! 1. a espessura é a menor distância entre dois planos paralelos "de costas" um para o outro (o
//!    material fica entre eles);
//! 2. as faces principais são as planas que têm esse par e os cilindros coaxiais cujos raios
//!    diferem de `t` (o raio interno e o externo de uma dobra);
//! 3. a prova de espessura constante é o volume: com `t` constante, o volume do corpo é a metade
//!    da área das faces principais vezes `t`. Um bloco, por exemplo, não passa.
//!
//! O ângulo de cada dobra sai da área da face cilíndrica interna: `área = raio × largura × ângulo`.

use super::*;
use fenkra_kernel::Mesh;

/// Uma dobra reconhecida: raio interno, ângulo (graus) e largura ao longo do eixo, mais o eixo
/// (direção, um ponto dele na coordenada axial zero e a faixa axial coberta) para o mapa de dobras.
#[derive(Clone, Debug, PartialEq)]
pub struct DobraReconhecida {
    pub raio: f64,
    pub angulo: f64,
    pub largura: f64,
    pub eixo: V3,
    pub origem: V3,
    pub faixa: [f64; 2],
}

/// O que se reconheceu do corpo.
#[derive(Clone, Debug)]
pub struct AnaliseChapa {
    pub espessura: f64,
    pub dobras: Vec<DobraReconhecida>,
    /// índices (na ordem das faces da malha) das faces planas principais
    pub planas: Vec<usize>,
}

impl AnaliseChapa {
    /// O menor raio interno das dobras (o que a chapa precisa suportar); `None` sem dobras.
    pub fn raio_minimo(&self) -> Option<f64> {
        self.dobras.iter().map(|d| d.raio).fold(None, |m, r| Some(m.map_or(r, |x: f64| x.min(r))))
    }

    /// Texto curto do que foi reconhecido, para o aviso do comando.
    pub fn resumo(&self) -> String {
        let n = self.dobras.len();
        if n == 0 {
            return format!("chapa reconhecida: espessura {:.2} mm, sem dobras (chapa plana)", self.espessura);
        }
        let lista: Vec<String> = self.dobras.iter().map(|d| format!("{:.0}° r{:.2}", d.angulo, d.raio)).collect();
        format!("chapa reconhecida: espessura {:.2} mm, {} dobra(s) ({})", self.espessura, n, lista.join(", "))
    }
}

pub(super) fn pontos_da_face(m: &Mesh, f: usize) -> Vec<V3> {
    let ini = m.face_ranges.get(f).copied().unwrap_or(0) as usize;
    let fim = m.face_ranges.get(f + 1).copied().map(|x| x as usize).unwrap_or(m.indices.len());
    let mut v: Vec<V3> = vec![];
    for &i in &m.indices[ini.min(m.indices.len())..fim.min(m.indices.len())] {
        let i = i as usize * 3;
        if i + 2 < m.positions.len() {
            v.push([m.positions[i] as f64, m.positions[i + 1] as f64, m.positions[i + 2] as f64]);
        }
    }
    v
}

/// Centro (no plano da seção, na altura do primeiro ponto) e raio da circunferência que passa
/// por três pontos da face cilíndrica, projetados no plano perpendicular ao eixo.
fn circulo(pts: &[V3], eixo: V3) -> Option<(V3, f64)> {
    let p0 = *pts.first()?;
    let u = geo3d::unit(geo3d::any_perp(eixo))?;
    let w = geo3d::unit(fenkra_kernel::cross(eixo, u))?;
    let plano = |p: V3| -> [f64; 2] {
        let d = geo3d::sub(p, p0);
        [geo3d::dot(d, u), geo3d::dot(d, w)]
    };
    let q: Vec<[f64; 2]> = pts.iter().map(|p| plano(*p)).collect();
    let dist = |a: [f64; 2], b: [f64; 2]| (a[0] - b[0]).hypot(a[1] - b[1]);
    let a = q[0];
    let b = *q.iter().max_by(|x, y| dist(a, **x).total_cmp(&dist(a, **y)))?;
    // terceiro ponto: o mais longe da corda a-b
    let ab = [b[0] - a[0], b[1] - a[1]];
    let n = ab[0].hypot(ab[1]);
    if n < 1e-9 {
        return None;
    }
    let c = *q.iter().max_by(|x, y| {
        let f = |p: &[f64; 2]| ((p[0] - a[0]) * ab[1] - (p[1] - a[1]) * ab[0]).abs() / n;
        f(x).total_cmp(&f(y))
    })?;
    let d = 2. * (a[0] * (b[1] - c[1]) + b[0] * (c[1] - a[1]) + c[0] * (a[1] - b[1]));
    if d.abs() < 1e-9 {
        return None;
    }
    let (a2, b2, c2) = (a[0] * a[0] + a[1] * a[1], b[0] * b[0] + b[1] * b[1], c[0] * c[0] + c[1] * c[1]);
    let cx = (a2 * (b[1] - c[1]) + b2 * (c[1] - a[1]) + c2 * (a[1] - b[1])) / d;
    let cy = (a2 * (c[0] - b[0]) + b2 * (a[0] - c[0]) + c2 * (b[0] - a[0])) / d;
    let r = (a[0] - cx).hypot(a[1] - cy);
    let centro = geo3d::add(p0, geo3d::add(geo3d::mul(u, cx), geo3d::mul(w, cy)));
    Some((centro, r))
}

struct Cil {
    eixo: V3,
    centro: V3,
    raio: f64,
    area: f64,
    largura: f64,
    faixa: [f64; 2],
}

/// Reconhece a chapa na malha de um corpo de `volume` mm³. Erra com o motivo quando o corpo não
/// tem espessura constante.
pub fn analisar_malha(m: &Mesh, volume: f64) -> Result<AnaliseChapa, String> {
    let info = &m.face_info;
    if info.is_empty() || volume <= 0. {
        return Err("o corpo está vazio".into());
    }
    let planas: Vec<usize> = (0..info.len()).filter(|i| info[*i].kind == 0).collect();
    // dois planos paralelos só formam espessura se as faces se enfrentam: as projeções delas no
    // plano se sobrepõem (a lateral da peça não enfrenta a face da aba vizinha)
    let pts: Vec<Vec<V3>> = planas.iter().map(|i| pontos_da_face(m, *i)).collect();
    let extensao = |k: usize, u: V3, w: V3| -> Option<([f64; 2], [f64; 2])> {
        let p = &pts[k];
        if p.is_empty() {
            return None;
        }
        let f = |e: V3| p.iter().fold((f64::MAX, f64::MIN), |(lo, hi), q| (lo.min(geo3d::dot(*q, e)), hi.max(geo3d::dot(*q, e))));
        let (a, b) = (f(u), f(w));
        Some(([a.0, a.1], [b.0, b.1]))
    };
    let se_enfrentam = |a: usize, b: usize| -> bool {
        let n = info[planas[a]].dir;
        let Some(u) = geo3d::unit(geo3d::any_perp(n)) else { return false };
        let Some(w) = geo3d::unit(fenkra_kernel::cross(n, u)) else { return false };
        let (Some(ea), Some(eb)) = (extensao(a, u, w), extensao(b, u, w)) else { return false };
        let sobra = |x: [f64; 2], y: [f64; 2]| x[1].min(y[1]) - x[0].max(y[0]);
        sobra(ea.0, eb.0) > 1e-6 && sobra(ea.1, eb.1) > 1e-6
    };
    // distância entre os planos de duas faces planas paralelas que se enfrentam
    let espaco = |a: usize, b: usize| -> Option<f64> {
        if geo3d::dot(info[planas[a]].dir, info[planas[b]].dir).abs() < 0.9999 || !se_enfrentam(a, b) {
            return None;
        }
        // de costas: normais opostas e o material entre as duas. As faces de uma fenda (o rasgo do
        // canto de uma caixa convertida) também se enfrentam, mas com ar entre elas, e não são espessura
        let (na, nb) = (info[planas[a]].dir, info[planas[b]].dir);
        let ab = geo3d::dot(geo3d::sub(info[planas[b]].point, info[planas[a]].point), na);
        if geo3d::dot(na, nb) > 0. || ab > 0. {
            return None;
        }
        let d = ab.abs();
        (d > 1e-6).then_some(d)
    };
    // espessura: a menor distância entre faces planas paralelas que se enfrentam
    let mut t = f64::MAX;
    for a in 0..planas.len() {
        for b in (a + 1)..planas.len() {
            if let Some(d) = espaco(a, b) {
                t = t.min(d);
            }
        }
    }
    if t == f64::MAX {
        return Err("o corpo não tem faces planas paralelas que se enfrentam: não é uma chapa".into());
    }
    let perto = |a: f64, b: f64| (a - b).abs() <= 0.02 * b.max(1e-6);
    // faces planas principais: têm uma face na frente à distância da espessura
    let mut principais: Vec<usize> = vec![];
    for a in 0..planas.len() {
        if (0..planas.len()).any(|b| b != a && espaco(a, b).is_some_and(|d| perto(d, t))) {
            principais.push(planas[a]);
        }
    }
    // cilindros ajustados (raio e eixo)
    let mut cils: Vec<(usize, Cil)> = vec![];
    for i in (0..info.len()).filter(|i| info[*i].kind == 1) {
        let pts = pontos_da_face(m, i);
        let Some(eixo) = geo3d::unit(info[i].dir) else { continue };
        let Some((centro, raio)) = circulo(&pts, eixo) else { continue };
        let (mut lo, mut hi) = (f64::MAX, f64::MIN);
        for p in &pts {
            let s = geo3d::dot(*p, eixo);
            lo = lo.min(s);
            hi = hi.max(s);
        }
        cils.push((i, Cil { eixo, centro, raio, area: info[i].area, largura: hi - lo, faixa: [lo, hi] }));
    }
    // dobras: pares coaxiais com raios que diferem de t
    let mut dobras: Vec<DobraReconhecida> = vec![];
    let mut area_cil = 0.;
    let mut usados = vec![false; cils.len()];
    for a in 0..cils.len() {
        for b in 0..cils.len() {
            if a == b || usados[a] || usados[b] || cils[a].1.raio >= cils[b].1.raio {
                continue;
            }
            let (ci, ce) = (&cils[a].1, &cils[b].1);
            let d = geo3d::sub(ce.centro, ci.centro);
            let fora_do_eixo = geo3d::sub(d, geo3d::mul(ci.eixo, geo3d::dot(d, ci.eixo)));
            let coaxial = geo3d::dot(ci.eixo, ce.eixo).abs() > 0.9999 && geo3d::dot(fora_do_eixo, fora_do_eixo).sqrt() < 0.01 * ce.raio.max(1.);
            if coaxial && perto(ce.raio - ci.raio, t) && perto(ce.largura, ci.largura) {
                usados[a] = true;
                usados[b] = true;
                area_cil += ci.area + ce.area;
                let origem = geo3d::sub(ci.centro, geo3d::mul(ci.eixo, geo3d::dot(ci.centro, ci.eixo)));
                dobras.push(DobraReconhecida { raio: ci.raio, angulo: (ci.area / (ci.raio * ci.largura.max(1e-9))).to_degrees(), largura: ci.largura, eixo: ci.eixo, origem, faixa: ci.faixa });
            }
        }
    }
    // prova da espessura constante: volume = metade da área das faces principais × t
    let area_planas: f64 = principais.iter().map(|i| info[*i].area).sum();
    let esperado = (area_planas + area_cil) / 2. * t;
    if (esperado - volume).abs() > 0.1 * volume {
        return Err(format!("a espessura não é constante: com {t:.2} mm de espessura o corpo teria {esperado:.0} mm³, e ele tem {volume:.0} mm³"));
    }
    dobras.sort_by(|a, b| a.raio.total_cmp(&b.raio).then(a.angulo.total_cmp(&b.angulo)));
    Ok(AnaliseChapa { espessura: t, dobras, planas: principais })
}

impl Ctx<'_> {
    /// Confere que o corpo é chapa (espessura constante, face fixa plana e principal) e avisa o que
    /// reconheceu. O corpo não muda: os comandos de chapa seguintes passam a valer sobre ele.
    pub(super) fn run_convert(&mut self, f: &Feature, c: &fenkra_core::SmConvert) -> Result<Option<String>, String> {
        let k = |e: KernelError| e.0;
        // B516: a face fixa segue a face pela âncora (dono + papel), não pelo ponto
        let c = &if c.face != [0.; 3] {
            let face = self.ponto_na_face("conversão em chapa", f, "face", c.face, f.flat_space)?;
            fenkra_core::SmConvert { face, ..c.clone() }
        } else {
            c.clone()
        };
        let mut corpo = self.body(f.target).ok_or("o corpo a converter precisa existir antes deste comando")?.clone();
        let mut malha = tesselar(&corpo, &self.qualidade).map_err(k)?;
        let mut gerada = false;
        let mut face_padrao: Option<[f64; 3]> = None;
        let mut a = match analisar_malha(&malha, corpo.mass().volume) {
            Ok(a) => a,
            // B522: um sólido maciço não tem espessura constante. Com a espessura e as faces a abrir,
            // o comando esvazia o sólido e a caixa resultante é analisada como qualquer chapa; sem
            // elas, o motivo diz o que escolher, em vez de falhar sem explicação
            Err(e) => {
                let t = c.thickness.trim();
                if t.is_empty() {
                    return Err(format!("{e}. Para converter um sólido maciço, informe a espessura da chapa e escolha ao menos uma face a abrir"));
                }
                if c.open_faces.is_empty() {
                    return Err("escolha ao menos uma face a abrir: o corpo é maciço e, sem face aberta, não há como esvaziá-lo em paredes de chapa".into());
                }
                let esp = super::chapa_laco::ev_faixa(t, &self.vars, "espessura da chapa", 0.01, f64::INFINITY, "mm")?;
                let (lo, hi) = corpo.bbox().ok_or("corpo sem dimensões")?;
                let centro = geo3d::mul(geo3d::add(lo, hi), 0.5);
                let aberta = c.open_faces[0];
                self.esvaziar_para_chapa(f, c, esp)?;
                gerada = true;
                corpo = self.body(f.target).ok_or("o corpo sumiu depois de esvaziar")?.clone();
                malha = tesselar(&corpo, &self.qualidade).map_err(k)?;
                if c.face == [0.; 3] {
                    face_padrao = fundo_oposto(&malha, centro, aberta);
                }
                analisar_malha(&malha, corpo.mass().volume).map_err(|e| format!("depois de esvaziar o sólido: {e}"))?
            }
        };
        // B522: o sólido esvaziado nasce com a face fixa no fundo, oposto à primeira face aberta: é a
        // raiz da árvore de dobras, e as paredes laterais saem dela. Com a maior face como raiz (um
        // cubo tem seis iguais) o rasgo dos cantos caía numa parede e o sólido planificado saía inválido
        let c = &if let Some(fp) = face_padrao { fenkra_core::SmConvert { face: fp, ..c.clone() } } else { c.clone() };
        if !c.thickness.trim().is_empty() {
            let pedida = super::chapa_laco::ev_faixa(&c.thickness, &self.vars, "espessura esperada", 0.01, f64::INFINITY, "mm")?;
            if (pedida - a.espessura).abs() > 0.02 * pedida.max(1e-6) {
                return Err(format!("o corpo tem {:.2} mm de espessura, e não os {pedida:.2} mm pedidos", a.espessura));
            }
        }
        if c.face != [0.; 3] {
            let n = a.planas.iter().any(|i| {
                let fi = &malha.face_info[*i];
                geo3d::dot(geo3d::sub(c.face, fi.point), fi.dir).abs() < 1e-3 * a.espessura.max(1.)
            });
            if !n {
                return Err("a face fixa precisa ser uma face plana da chapa (não a lateral da espessura)".into());
            }
        }
        // cantos vivos (caixa com casca, chapa dobrada sem raio): as arestas viram dobras com o raio
        // da chapa, e os cantos entre paredes vizinhas ganham rasgo e alívio (`chapa_converte_vivas`)
        let mut feito = String::new();
        if self.sheet_maps.is_empty() {
            let r = self.sheet().map(|s| s.r).unwrap_or(a.espessura);
            if let Some(v) = super::chapa_converte_vivas::dobrar_arestas_vivas(&corpo, &malha, &a, c.face, r, &self.qualidade)? {
                corpo = v.corpo;
                malha = tesselar(&corpo, &self.qualidade).map_err(k)?;
                a = analisar_malha(&malha, corpo.mass().volume).map_err(|e| format!("depois de dobrar as arestas vivas: {e}"))?;
                self.bodies.insert(f.target, fresh_body(corpo.clone(), f.id));
                feito = format!("{} aresta(s) viva(s) viraram dobra de raio {r:.2}", v.dobras);
                if v.rasgos > 0 {
                    feito = format!("{feito}, {} canto(s) com rasgo e alívio", v.rasgos);
                }
            }
        }
        let mut aviso = if feito.is_empty() { a.resumo() } else { format!("{}; {feito}", a.resumo()) };
        // mapa de dobras (B35): cada dobra reconhecida vira um `SheetMap`, como as de uma aba, e na
        // chapa planificada o corpo é desdobrado por elas. Corpo que já veio de comandos de chapa
        // tem o mapa deles e não é mapeado de novo.
        if !a.dobras.is_empty() && self.sheet_maps.is_empty() {
            let (lo, hi) = corpo.bbox().ok_or("corpo sem dimensões")?;
            let grande = geo3d::dist(lo, hi) + 10.;
            let sp = self.sheet().ok();
            let kf = |r: f64| sp.as_ref().map_or(0.44, |s| s.k_for(r));
            let plano = sp.as_ref().is_some_and(|s| s.flat);
            match super::chapa_converte_mapa::mapear(&malha, &a, c.face, &kf, grande) {
                Ok(mapa) => {
                    if plano {
                        let plana = super::chapa_converte_mapa::planificar(&corpo, &mapa.maps)?;
                        let esperado = corpo.mass().volume - mapa.arcos + mapa.tiras;
                        let obtido = plana.mass().volume;
                        if plana.is_null() || (obtido - esperado).abs() > 0.01 * esperado {
                            return Err(format!("não consegui planificar o corpo: o volume plano deu {obtido:.0} mm³ e a conta dá {esperado:.0} mm³ (alguma parte é mais larga que a dobra que a prende?)"));
                        }
                        self.bodies.insert(f.target, fresh_body(plana, f.id));
                    }
                    aviso = format!("{aviso}; {} dobra(s) no mapa de dobras", mapa.maps.len());
                    self.sheet_maps.extend(mapa.maps);
                }
                Err(e) if plano => return Err(format!("o corpo não se planifica: {e}")),
                Err(e) => aviso = format!("{aviso}; sem mapa de dobras ({e})"),
            }
        }
        // sólido maciço convertido por aqui: a espessura veio do próprio comando, então o parâmetro
        // da chapa é que precisa combinar com ela (o aviso diz qual ajustar)
        if let Some(sp) = self.doc.sheet_efetivo() {
            let t = ev(&sp.thickness, &self.vars, "espessura da chapa")?;
            if (t - a.espessura).abs() > 0.02 * a.espessura {
                return Ok(Some(format!("{aviso}; a chapa dos parâmetros tem {t:.2} mm: use a espessura medida em Chapa › Parâmetros")));
            }
        }
        Ok(if gerada { None } else { Some(aviso) })
    }

    /// B522 (ver `fundo_oposto`): esvazia o sólido maciço pelas faces abertas, deixando paredes de espessura `esp`. É a
    /// mesma casca do comando Casca, mas a falta de face aberta ou a espessura que não cabe vêm com
    /// a mensagem da conversão.
    fn esvaziar_para_chapa(&mut self, f: &Feature, c: &fenkra_core::SmConvert, esp: f64) -> Result<(), String> {
        let (b, owners, roles) = self.body_owned(f.target).ok_or("não há corpo para converter")?;
        if let Some((lo, hi)) = b.bbox() {
            let menor = (0..3).map(|i| hi[i] - lo[i]).fold(f64::INFINITY, f64::min);
            if esp >= menor / 2. - 1e-9 {
                return Err(format!("espessura {esp} mm grande demais: a menor dimensão do corpo é {menor:.2} mm e as duas paredes opostas precisam caber nela"));
            }
        }
        let pts = self.resolve_faces("conversão em chapa", f, b, owners, roles, &c.open_faces, &[])?;
        let (s, out) = b.shell_tagged(&pts, esp, false, &tags_for(owners)).map_err(|e| format!("não consegui esvaziar o sólido com {esp} mm de parede: {}", e.0))?;
        if s.mass().volume >= b.mass().volume - 1e-9 * b.mass().volume.abs().max(1.) {
            return Err("as faces escolhidas não abriram o corpo: escolha faces do próprio sólido".into());
        }
        self.bodies.insert(f.target, remap_body(s, owners, roles, &out, f.id));
        Ok(())
    }
}

/// Analisa o corpo de uma peça até antes do comando `ate` (que pode ser o próprio converter). Serve
/// ao painel, que mostra a espessura e as dobras e aplica à chapa do documento.
pub fn analisar_corpo(doc: &Document, ate: Id) -> Result<AnaliseChapa, String> {
    let mut d = doc.clone();
    let pos = d.features.iter().position(|f| f.id == ate).ok_or("comando não encontrado")?;
    d.features.truncate(pos);
    let r = recompute(&d);
    let b = r.bodies.first().ok_or("não há corpo para converter antes deste comando")?;
    analisar_malha(&b.mesh, b.mass.volume)
}

// ---------- B553: um corpo de chapa por peça ----------

/// Pontos que descrevem o laço (para testar quem está dentro de quem). Os arcos entram pelos três
/// pontos; o círculo, por oito pontos; a elipse, pelos extremos dos semieixos.
fn pontos_do_laco(l: &fenkra_kernel::Loop2) -> Vec<[f64; 2]> {
    use fenkra_kernel::Seg2;
    let mut v = vec![];
    for sg in l {
        match sg {
            Seg2::Line(a, _) => v.push(*a),
            Seg2::Arc(a, m, _) => v.extend([*a, *m]),
            Seg2::Circle(c, r) => v.extend((0..8).map(|i| {
                let t = i as f64 * std::f64::consts::TAU / 8.;
                [c[0] + r * t.cos(), c[1] + r * t.sin()]
            })),
            Seg2::Spline(p) => v.extend(p.iter().copied()),
            Seg2::Ellipse(c, ax, b, _, _) => {
                let n = [-ax[1], ax[0]];
                let ln = (n[0] * n[0] + n[1] * n[1]).sqrt().max(1e-12);
                let u = [n[0] / ln * b, n[1] / ln * b];
                v.extend([[c[0] + ax[0], c[1] + ax[1]], [c[0] + u[0], c[1] + u[1]], [c[0] - ax[0], c[1] - ax[1]], [c[0] - u[0], c[1] - u[1]]]);
            }
        }
    }
    v
}

fn ponto_em_poligono(p: [f64; 2], poly: &[[f64; 2]]) -> bool {
    let mut dentro = false;
    let mut j = poly.len().wrapping_sub(1);
    for i in 0..poly.len() {
        let (a, b) = (poly[i], poly[j]);
        if (a[1] > p[1]) != (b[1] > p[1]) && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0] {
            dentro = !dentro;
        }
        j = i;
    }
    dentro
}

/// Quantos laços do perfil são contornos externos: laço dentro de um número par de outros (zero,
/// dois...) é contorno externo; dentro de um número ímpar é furo. Cada contorno externo daria um
/// corpo separado.
pub(super) fn contornos_externos(loops: &[fenkra_kernel::Loop2]) -> usize {
    let polis: Vec<Vec<[f64; 2]>> = loops.iter().map(pontos_do_laco).collect();
    (0..polis.len())
        .filter(|&i| {
            let Some(p) = polis[i].first().copied() else { return true };
            let profundidade = (0..polis.len()).filter(|&j| j != i && polis[j].len() > 2 && ponto_em_poligono(p, &polis[j])).count();
            profundidade % 2 == 0
        })
        .count()
}

/// Quantas peças separadas a malha tem: vértices na mesma posição e os três vértices de cada
/// triângulo são da mesma peça. Duas paredes que só se tocam continuam sendo uma peça.
pub(super) fn pecas_da_malha(m: &Mesh) -> usize {
    use std::collections::HashMap;
    let n = m.positions.len() / 3;
    if n == 0 {
        return 0;
    }
    fn raiz(pai: &mut [usize], mut x: usize) -> usize {
        while pai[x] != x {
            pai[x] = pai[pai[x]];
            x = pai[x];
        }
        x
    }
    fn une(pai: &mut [usize], a: usize, b: usize) {
        let (a, b) = (raiz(pai, a), raiz(pai, b));
        pai[a] = b;
    }
    let mut pai: Vec<usize> = (0..n).collect();
    let q = |v: f32| (v as f64 * 1000.).round() as i64;
    let mut vistos: HashMap<(i64, i64, i64), usize> = HashMap::new();
    for i in 0..n {
        let chave = (q(m.positions[3 * i]), q(m.positions[3 * i + 1]), q(m.positions[3 * i + 2]));
        match vistos.get(&chave) {
            Some(&j) => une(&mut pai, i, j),
            None => {
                vistos.insert(chave, i);
            }
        }
    }
    for t in m.indices.chunks_exact(3) {
        une(&mut pai, t[0] as usize, t[1] as usize);
        une(&mut pai, t[1] as usize, t[2] as usize);
    }
    let mut raizes: Vec<usize> = (0..n).map(|i| raiz(&mut pai, i)).collect();
    raizes.sort_unstable();
    raizes.dedup();
    raizes.len()
}

/// B522: ponto da face plana mais afastada da face aberta, entre as paralelas a ela: o fundo da
/// caixa. `aberta` é um ponto da face aberta no corpo maciço; `centro`, o centro da caixa do corpo.
/// A normal da face aberta é a direção do centro para o ponto, no eixo mais próximo do plano dela.
fn fundo_oposto(m: &Mesh, centro: V3, aberta: V3) -> Option<V3> {
    // normal da face aberta: a face plana do corpo cheio não existe mais, então deduz-se pelo eixo
    // em que o ponto mais se afasta do centro (caixa alinhada aos eixos) e depois pela face plana
    // vizinha de maior afastamento
    let d = geo3d::sub(aberta, centro);
    let eixo = (0..3).max_by(|&i, &j| d[i].abs().total_cmp(&d[j].abs()))?;
    let mut n = [0.; 3];
    n[eixo] = if d[eixo] >= 0. { 1. } else { -1. };
    let mut melhor: Option<(f64, V3)> = None;
    for fi in m.face_info.iter() {
        if fi.kind != 0 || geo3d::dot(fi.dir, n).abs() < 0.9999 {
            continue;
        }
        let h = geo3d::dot(fi.point, n);
        if melhor.is_none_or(|(x, _)| h < x) {
            melhor = Some((h, fi.point));
        }
    }
    melhor.map(|(_, p)| p)
}
