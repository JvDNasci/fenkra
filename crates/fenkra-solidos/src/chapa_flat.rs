// SPDX-License-Identifier: LGPL-2.1-or-later
//! Chapa no estado planificado: o que cada comando faz quando a peça está desdobrada (B86).
//!
//! Regra: nenhum comando some na planificada. Ou ele entra no corpo (a aba por perfil vira uma
//! tira desenvolvida), ou ele vira marca de ferramenta (veneziana e estampo: a chapa plana não
//! muda, mas o contorno da ferramenta fica registrado para o DXF e para a vista), ou ele entra
//! no mapa de dobras. Quem consome as marcas lê `RecomputeResult::cut_outlines`.

use super::*;

const SEGMENTOS_CIRCULO: usize = 48;

/// Círculo fechado (primeiro ponto repetido no fim) no plano (x, y) em torno de `c`.
fn circulo3d(c: V3, x: V3, y: V3, r: f64) -> Vec<V3> {
    (0..=SEGMENTOS_CIRCULO)
        .map(|i| {
            let a = i as f64 / SEGMENTOS_CIRCULO as f64 * std::f64::consts::TAU;
            geo3d::add(c, geo3d::add(geo3d::mul(x, r * a.cos()), geo3d::mul(y, r * a.sin())))
        })
        .collect()
}

/// Um canto do perfil desenvolvido.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Canto {
    /// onde o arco começa, medido ao longo da tira desenvolvida
    pub inicio: f64,
    /// comprimento desenvolvido do canto (linha neutra)
    pub ba: f64,
    /// giro do canto (rad)
    pub giro: f64,
    /// raio interno do material nesse canto
    pub raio_interno: f64,
    /// a dobra fecha para o lado do perfil desenhado (a face de referência fica por dentro)
    pub cima: bool,
}

/// Perfil aberto desenvolvido numa tira reta.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Desenvolvimento {
    pub comprimento: f64,
    pub cantos: Vec<Canto>,
}

/// Desenvolve a cadeia da aba por perfil: os trechos retos (de tangente a tangente) mais o
/// comprimento neutro de cada canto. Os cantos seguem a mesma regra da geometria dobrada
/// (`round_corners`): raio `rad` na linha desenhada, limitado a 45 % do menor trecho vizinho.
/// `esquerda`: a espessura cresce para a esquerda da cadeia (lado 0 da parede fina); com isso a
/// linha desenhada é a face de fora num canto que vira para a esquerda e a de dentro no outro.
pub(crate) fn desenvolver_perfil(ch: &[[f64; 2]], rad: f64, t: f64, k_para: &dyn Fn(f64) -> f64, esquerda: bool) -> Desenvolvimento {
    let mut p: Vec<[f64; 2]> = vec![];
    for q in ch {
        if p.last().is_none_or(|l| (l[0] - q[0]).hypot(l[1] - q[1]) > 1e-9) {
            p.push(*q);
        }
    }
    let n = p.len();
    if n < 2 {
        return Desenvolvimento { comprimento: 0., cantos: vec![] };
    }
    let comp = |i: usize| (p[i + 1][0] - p[i][0]).hypot(p[i + 1][1] - p[i][1]);
    // por vértice interno: (recuo da tangente, giro, raio efetivo, vira para a esquerda)
    let mut canto: Vec<Option<(f64, f64, f64, bool)>> = vec![None; n];
    for i in 1..n - 1 {
        let (a, b, c) = (p[i - 1], p[i], p[i + 1]);
        let (l1, l2) = (comp(i - 1), comp(i));
        let v1 = [(b[0] - a[0]) / l1, (b[1] - a[1]) / l1];
        let v2 = [(c[0] - b[0]) / l2, (c[1] - b[1]) / l2];
        let giro = (v1[0] * v2[0] + v1[1] * v2[1]).clamp(-1., 1.).acos();
        if giro < 1e-3 || rad <= 1e-9 {
            continue;
        }
        let d = (rad * (giro / 2.).tan()).min(0.45 * l1.min(l2));
        let r_ef = d / (giro / 2.).tan();
        canto[i] = Some((d, giro, r_ef, v1[0] * v2[1] - v1[1] * v2[0] > 0.));
    }
    let mut s = 0.;
    let mut cantos = vec![];
    for i in 0..n - 1 {
        let d0 = canto[i].map(|c| c.0).unwrap_or(0.);
        let d1 = canto[i + 1].map(|c| c.0).unwrap_or(0.);
        s += (comp(i) - d0 - d1).max(0.);
        if let Some((_, giro, r_ef, esq)) = canto[i + 1] {
            // o centro do canto fica do lado do giro; se a espessura cresce para esse lado, a
            // linha desenhada é a face de fora do canto
            let fora = esq == esquerda;
            let r_in = if fora { (r_ef - t).max(0.01) } else { r_ef };
            let ba = giro * (r_in + k_para(r_in) * t);
            cantos.push(Canto { inicio: s, ba, giro, raio_interno: r_in, cima: !fora });
            s += ba;
        }
    }
    Desenvolvimento { comprimento: s, cantos }
}

/// Dedução de dobra do canto (2 × recuo externo − comprimento desenvolvido).
fn deducao(c: &Canto, t: f64) -> f64 {
    let ossb = if c.giro < 179.9_f64.to_radians() { (c.giro / 2.).tan() * (c.raio_interno + t) } else { c.raio_interno + t };
    2. * ossb - c.ba
}

impl Ctx<'_> {
    fn marca_ferramenta(&mut self, f: &Feature, laços: Vec<Vec<V3>>) {
        self.cut_outlines.push(CutOutline { feature: f.id, folded: vec![], flat: laços, dobra: None });
    }

    /// Veneziana na planificada: a chapa não muda, mas a rampa não some. O retângulo da
    /// ferramenta (largura × comprimento da linha) vai para as marcas.
    pub(super) fn veneziana_planificada(&mut self, f: &Feature, lv: &SmLouver) -> Result<Option<String>, String> {
        let k = |e: KernelError| e.0;
        let w = ev(&lv.width, &self.vars, "largura")?;
        let b = self.body(f.target).ok_or("a veneziana precisa de uma chapa")?.clone();
        let a = self.map_point(lv.a, f.flat_space, true);
        let c = self.map_point(lv.b, f.flat_space, true);
        let mid = geo3d::lerp(a, c, 0.5);
        let (_, n0) = b.surface_near(mid).map_err(k)?;
        let e = geo3d::sub(c, a);
        let len = geo3d::norm(e);
        let e = geo3d::unit(e).ok_or("os dois pontos da linha coincidem")?;
        let mut lado = geo3d::unit(fenkra_kernel::cross(n0, e)).ok_or("linha normal à chapa")?;
        if lv.flip {
            lado = geo3d::mul(lado, -1.);
        }
        let (p0, p1) = (a, geo3d::add(a, geo3d::mul(e, len)));
        let ret = vec![p0, p1, geo3d::add(p1, geo3d::mul(lado, w)), geo3d::add(p0, geo3d::mul(lado, w)), p0];
        self.marca_ferramenta(f, vec![ret]);
        Ok(Some("planificada: veneziana marcada pelo contorno da ferramenta".into()))
    }

    /// Estampo na planificada (tipos 0, 1, 2 e 3): a chapa plana não leva o copo; o contorno da
    /// ferramenta vai para as marcas.
    pub(super) fn estampo_planificado(&mut self, f: &Feature, st: &SmStamp, frame: &Frame, sk: &EvalSketch, t: f64, largura: f64) -> Result<Option<String>, String> {
        let aqui = |q: &[f64; 2]| self.map_point(frame.point(q[0], q[1]), f.flat_space, true);
        let fecha = |mut v: Vec<V3>| {
            if v.first() != v.last() {
                if let Some(p) = v.first().copied() {
                    v.push(p);
                }
            }
            v
        };
        let mut laços: Vec<Vec<V3>> = vec![];
        match st.kind {
            1 => {
                for ch in sk.open.iter().chain(loops_points(&sk.loops).iter()) {
                    laços.push(ch.iter().map(aqui).collect());
                }
            }
            2 => {
                let r0 = if largura > 0. { largura } else { 8. * t } / 2.;
                for q in &sk.points {
                    laços.push(circulo3d(aqui(q), frame.x, frame.y(), r0));
                }
            }
            3 => {
                let r0 = if largura > 0. { largura } else { 2. * t }.max(t);
                for q in &sk.points {
                    laços.push(circulo3d(aqui(q), frame.x, frame.y(), r0));
                }
            }
            _ => {
                for lp in loops_points(&sk.loops) {
                    laços.push(fecha(lp.iter().map(aqui).collect()));
                }
            }
        }
        if laços.is_empty() {
            return Err("o estampo não tem geometria no sketch (perfil fechado, linhas ou pontos)".into());
        }
        self.marca_ferramenta(f, laços);
        Ok(Some("planificada: estampo marcado pelo contorno da ferramenta".into()))
    }

    /// Estampo por punção na planificada: o copo não existe na chapa plana. A pegada do punção
    /// (a seção dele pelo plano da chapa) vai para as marcas de ferramenta, como nos outros estampos.
    pub(super) fn puncao_planificado(&mut self, f: &Feature, punch: &Shape, chapa: &Shape) -> Result<Option<String>, String> {
        let k = |e: KernelError| e.0;
        // o punção desenhado com a chapa dobrada acompanha as dobras até a planificada
        let punch = self.map_tool(punch.clone(), f.flat_space, true)?;
        let (plo, phi) = punch.bbox().ok_or("punção sem dimensões")?;
        let (on, n) = chapa.surface_near(geo3d::lerp(plo, phi, 0.5)).map_err(k)?;
        let meia = geo3d::dist(plo, phi) + 10.;
        let mut laços: Vec<Vec<V3>> = vec![];
        for mut l in punch.section_loops(on, n, meia).map_err(k)? {
            if l.len() < 3 {
                continue;
            }
            if l.first() != l.last() {
                l.push(l[0]);
            }
            laços.push(l);
        }
        if laços.is_empty() {
            return Err("o punção não toca a chapa: não há pegada para marcar na planificada".into());
        }
        self.marca_ferramenta(f, laços);
        Ok(Some("planificada: punção marcado pela pegada na chapa".into()))
    }

    /// Aba por perfil na planificada: cada cadeia vira uma tira reta com o comprimento
    /// desenvolvido (trechos retos mais o comprimento neutro de cada canto), na espessura da
    /// chapa, saindo do ponto inicial do perfil para fora da chapa. A largura segue o
    /// comprimento do comando (l1 para a frente, l2 para trás). Cada canto vira uma linha de dobra.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn aba_perfil_planificada(&mut self, f: &Feature, frame: &Frame, sk: &EvalSketch, sp: SheetValues, r: f64, l1: f64, l2: f64, esquerda: bool) -> Result<Option<String>, String> {
        let k = |e: KernelError| e.0;
        let corpo = self.body(f.target).ok_or("a aba por perfil precisa de uma parede de chapa antes")?.clone();
        let centro = corpo.mass().center;
        let (on, n) = corpo.surface_near(centro).map_err(k)?;
        let n = geo3d::unit(n).ok_or("a chapa não tem normal")?;
        let topo = geo3d::dot(on, n);
        let dir = geo3d::unit(fenkra_kernel::cross(frame.normal, n)).ok_or("o plano do perfil é paralelo à chapa: desenhe o perfil num plano que corte a chapa")?;
        let w = geo3d::unit(frame.normal).ok_or("plano do perfil sem normal")?;
        if geo3d::dot(w, n).abs() > 1e-6 {
            // a aba é extrudada pela normal do plano do perfil e só prende na aresta da chapa quando
            // essa normal fica no plano da chapa; inclinada, a faixa sai do plano e não tem linha de
            // dobra com a chapa para desenvolver
            return Err(format!("o plano do perfil está {:.1}° fora da perpendicular à chapa: a planificação exata pede o perfil num plano perpendicular a ela (o plano que contém a normal da chapa)", geo3d::dot(w, n).abs().min(1.).asin().to_degrees()));
        }
        let k_para = |ri: f64| sp.k_for(ri);
        let largura = l1 + l2;
        if largura <= 0. {
            return Err("a aba por perfil precisa de largura maior que zero na planificada".into());
        }
        let meio = topo - sp.t / 2.;
        for ch in sk.open.iter().filter(|c| c.len() >= 2) {
            let p0 = frame.point(ch[0][0], ch[0][1]);
            let p_fim = frame.point(ch[ch.len() - 1][0], ch[ch.len() - 1][1]);
            // para fora da chapa: o ponto inicial fica na borda, longe do centro da chapa
            let mut fora = geo3d::dot(geo3d::sub(p0, centro), dir);
            if fora.abs() < 1e-6 {
                fora = geo3d::dot(geo3d::sub(p_fim, p0), dir);
            }
            let d = if fora < 0. { geo3d::mul(dir, -1.) } else { dir };
            let des = desenvolver_perfil(ch, r + sp.t / 2., sp.t, &k_para, esquerda);
            if des.comprimento <= 1e-6 {
                return Err("a cadeia do perfil tem comprimento nulo".into());
            }
            // base da tira: ponto inicial levado para o fundo da chapa
            let o = geo3d::sub(p0, geo3d::mul(n, geo3d::dot(p0, n) - (topo - sp.t)));
            let c0 = geo3d::sub(o, geo3d::mul(w, l2));
            let fim = geo3d::add(c0, geo3d::mul(d, des.comprimento));
            let pts = [c0, fim, geo3d::add(fim, geo3d::mul(n, sp.t)), geo3d::add(c0, geo3d::mul(n, sp.t))];
            let tira = Shape::poly_prism(&pts, geo3d::mul(w, largura)).map_err(k)?;
            self.apply(f.target, tira, BoolOp::Add, f.id)?;
            // uma linha de dobra por canto, no plano médio da chapa
            let base = geo3d::sub(c0, geo3d::mul(n, geo3d::dot(c0, n) - meio));
            for c in &des.cantos {
                let linha = |du: f64| {
                    let a = geo3d::add(base, geo3d::mul(d, du));
                    [a, geo3d::add(a, geo3d::mul(w, largura))]
                };
                let bl = BendLine {
                    start: linha(c.inicio),
                    center: linha(c.inicio + c.ba / 2.),
                    end: linha(c.inicio + c.ba),
                    angle: c.giro.to_degrees(),
                    radius: c.raio_interno,
                    up: c.cima,
                    feature: f.id,
                    allowance: c.ba,
                    deduction: deducao(c, sp.t),
                    eixo: None,
                    curva: vec![],
                };
                self.cut_outlines.push(CutOutline { feature: f.id, folded: vec![], flat: vec![], dobra: Some(bl) });
            }
        }
        Ok(None)
    }
}

/// Passo de um comando na planificada: a ferramenta e o que ela faz com o corpo.
#[derive(Clone)]
pub struct PassoPlanificado {
    pub feature: Id,
    pub op: BoolOp,
    pub ferramenta: Shape,
}

/// Todas as ferramentas que os comandos aplicam ao corpo com a peça planificada, na ordem. Serve
/// à verificação de sobreposição e à de comando ausente (`planificada::planificar`).
pub fn passos_planificados(doc: &Document) -> Vec<PassoPlanificado> {
    let mut d = doc.clone();
    if let Some(s) = d.sheet.as_mut() {
        s.flat = true;
    }
    let Ok(vars) = d.eval_params() else { return vec![] };
    let mut cx = Ctx {
        doc: &d,
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
        aviso_cantos: RefCell::new(None),
    };
    let mut out = vec![];
    for f in d.active_features().map(|(_, f)| f) {
        cx.steps.remove(&f.id);
        let _ = cx.run(f);
        if let Some(passos) = cx.steps.get(&f.id) {
            for (s, op) in passos {
                out.push(PassoPlanificado { feature: f.id, op: *op, ferramenta: s.clone() });
            }
        }
    }
    out
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn perfil_em_l_desenvolve_trechos_e_um_canto() {
        // L de 18 mm de altura e 10 de base, canto a 90° com raio de chapa 2 e espessura 2
        let cadeia = [[0., 0.], [0., 18.], [10., 18.]];
        let k = |_: f64| 0.44;
        let d = desenvolver_perfil(&cadeia, 3., 2., &k, true);
        assert_eq!(d.cantos.len(), 1);
        let c = d.cantos[0];
        // raio efetivo 3 na linha desenhada; vira à direita e a espessura cresce à esquerda: dentro
        assert!(c.cima && (c.raio_interno - 3.).abs() < 1e-9, "{c:?}");
        let ba = std::f64::consts::FRAC_PI_2 * (3. + 0.44 * 2.);
        assert!((c.ba - ba).abs() < 1e-9);
        // trechos retos: (18 - 3) + (10 - 3) mais o canto
        assert!((d.comprimento - (15. + 7. + ba)).abs() < 1e-9, "{}", d.comprimento);
        assert!((c.inicio - 15.).abs() < 1e-9);
        // espessura do outro lado: a linha desenhada é a face de fora, raio interno 3 - 2
        let d2 = desenvolver_perfil(&cadeia, 3., 2., &k, false);
        assert!(!d2.cantos[0].cima && (d2.cantos[0].raio_interno - 1.).abs() < 1e-9, "{:?}", d2.cantos[0]);
    }

    #[test]
    fn perfil_reto_nao_tem_canto() {
        let d = desenvolver_perfil(&[[0., 0.], [5., 0.], [12., 0.]], 3., 2., &|_| 0.44, true);
        assert!(d.cantos.is_empty());
        assert!((d.comprimento - 12.).abs() < 1e-9);
    }
}
