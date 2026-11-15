// SPDX-License-Identifier: LGPL-2.1-or-later
//! B176: saídas da estrutura para máquinas de perfil e de tubo.
//!
//! - **STEP do tubo cortado**: um arquivo por item da lista de corte, com o sólido já cortado (pontas,
//!   encaixes, furos) no referencial local do membro (Z ao longo da barra, começando em 0). É o que os
//!   CAM de corte de tubo importam.
//! - **DSTV NC1**: um arquivo por item, no formato da "Standard Description for Steel Structure Pieces
//!   for the Numerical Controls" (DSTV, 7ª edição, julho de 1998). Bloco ST com o cabeçalho (pedido,
//!   desenho, fase, peça, aço, quantidade, perfil, código do perfil, comprimento, cotas da seção,
//!   massa e área de pintura por metro, cortes inclinados da alma e da aba), bloco BO com os furos
//!   passantes achados no sólido e EN.
//!
//! - **AK**: contorno externo das pontas recortadas, no tubo retangular com encaixe e nos perfis
//!   abertos I, U e L com entalhe (`contornos_abertos`).
//! - **Tubo redondo**: os furos radiais vão na face v desenrolada (x ao longo da barra, y = arco na
//!   superfície externa a partir da geratriz de baixo, girando para +X).
//!
//! Fora desta versão, e dito no relatório de cada arquivo: contorno AK do tubo redondo com sela,
//! entalhe interno (IK) e furos oblíquos.

use super::*;

/// Código do perfil no DSTV: I, U, L, B (chapa), RU (barra redonda), RO (tubo redondo), M (tubo
/// retangular), C, T, SO (especial).
pub fn codigo_dstv(k: SectionKind) -> &'static str {
    match k {
        SectionKind::PerfilI => "I",
        SectionKind::PerfilU => "U",
        SectionKind::Cantoneira => "L",
        SectionKind::BarraChata => "B",
        SectionKind::BarraRedonda => "RU",
        SectionKind::TuboRedondo => "RO",
        SectionKind::TuboQuadrado | SectionKind::TuboRetangular => "M",
        SectionKind::PerfilT => "T",
        SectionKind::BarraQuadrada => "SO",
    }
}

/// Cotas do cabeçalho: altura, largura da aba, espessura da aba, espessura da alma (mm).
pub fn cotas_dstv(k: SectionKind, d: &[f64]) -> [f64; 4] {
    let g = |i: usize| d.get(i).copied().unwrap_or(0.);
    match k {
        // altura, aba, alma, mesa
        SectionKind::PerfilI | SectionKind::PerfilU | SectionKind::PerfilT => [g(0), g(1), g(3), g(2)],
        SectionKind::Cantoneira => [g(0), g(1), g(2), g(2)],
        SectionKind::TuboQuadrado => [g(0), g(0), g(1), g(1)],
        // largura, altura, parede
        SectionKind::TuboRetangular => [g(1), g(0), g(2), g(2)],
        SectionKind::TuboRedondo => [g(0), g(0), g(1), g(1)],
        SectionKind::BarraRedonda | SectionKind::BarraQuadrada => [g(0), g(0), 0., 0.],
        // chapa: largura, 0, 0, espessura
        SectionKind::BarraChata => [g(0), 0., 0., g(1)],
    }
}

/// Furo passante achado no sólido, já nas coordenadas da face do DSTV.
#[derive(Clone, Debug, PartialEq)]
pub struct FuroDstv {
    /// face: v (alma, frente), o (aba de cima), u (aba de baixo), h (atrás)
    pub face: char,
    pub x: f64,
    pub y: f64,
    pub diametro: f64,
}

/// Peça de um item para a máquina.
#[derive(Clone)]
pub struct PecaNc {
    pub item: usize,
    pub designacao: String,
    pub material: String,
    pub quantidade: usize,
    pub kind: SectionKind,
    pub dims: Vec<f64>,
    pub comprimento: f64,
    /// massa (kg/m; chapa: kg/m²) e área de pintura (m²/m; chapa: m²/m²)
    pub kg_m: f64,
    pub pintura_m: f64,
    /// cortes inclinados em graus: alma início, alma fim, aba início, aba fim
    pub cortes: [f64; 4],
    pub furos: Vec<FuroDstv>,
    /// contorno externo (AK) por face, já fechado (o primeiro ponto repete no fim), em (x, y) da face
    pub contornos: Vec<(char, Vec<[f64; 2]>)>,
    /// o que o arquivo não descreve (encaixe, furo de tubo redondo, furo oblíquo)
    pub faltas: Vec<String>,
    /// sólido cortado no referencial local
    pub forma: Shape,
}

/// Normal externa média de cada ponta plana (referencial local, Z ao longo da barra).
fn normais_das_pontas(sh: &Shape) -> [Option<V3>; 2] {
    let Ok(mesh) = sh.mesh_angle(0.5, 0.35) else { return [None, None] };
    let p = |i: u32| {
        let k = i as usize * 3;
        [mesh.positions[k] as f64, mesh.positions[k + 1] as f64, mesh.positions[k + 2] as f64]
    };
    let (mut zmin, mut zmax) = (f64::MAX, f64::MIN);
    for v in mesh.positions.chunks(3) {
        zmin = zmin.min(v[2] as f64);
        zmax = zmax.max(v[2] as f64);
    }
    let meio = (zmin + zmax) / 2.;
    let mut soma: [V3; 2] = [[0.; 3]; 2];
    let mut normais: [Vec<V3>; 2] = [vec![], vec![]];
    let mut pontos: [Vec<V3>; 2] = [vec![], vec![]];
    for t in mesh.indices.chunks(3) {
        let (a, b, c) = (p(t[0]), p(t[1]), p(t[2]));
        let n = fenkra_kernel::cross(geo3d::sub(b, a), geo3d::sub(c, a));
        let l = geo3d::norm(n);
        if l < 1e-9 {
            continue;
        }
        let u = geo3d::mul(n, 1. / l);
        if u[2].abs() < 0.02 {
            continue;
        }
        let k = usize::from((a[2] + b[2] + c[2]) / 3. > meio);
        soma[k] = geo3d::add(soma[k], n);
        normais[k].push(u);
        pontos[k].extend([a, b, c]);
    }
    // plana: todas as normais na mesma direção e todos os pontos no mesmo plano (um entalhe tem
    // degraus paralelos, com a mesma normal em planos diferentes)
    [0, 1].map(|k| {
        let m = geo3d::unit(soma[k])?;
        let (d0, d1) = pontos[k].iter().map(|q| geo3d::dot(*q, m)).fold((f64::MAX, f64::MIN), |(a, b), d| (a.min(d), b.max(d)));
        (normais[k].iter().all(|u| geo3d::dot(*u, m) > 0.99996) && d1 - d0 < 0.05).then_some(m)
    })
}

/// Ângulo do corte visto num plano: desvio da normal da ponta em relação ao eixo, só pela componente
/// `c` (0 = x, vista da aba; 1 = y, vista da alma). Positivo no sentido anti-horário.
fn angulo_no_plano(n: V3, c: usize) -> f64 {
    let a = n[c].atan2(n[2].abs()).to_degrees();
    (a * 1000.).round() / 1000.
}

/// Furo radial em tubo ou barra redonda, na superfície desenrolada (face v do DSTV).
///
/// Como funciona: o furo tem de ser radial, isto é, o eixo dele é perpendicular ao eixo da barra e
/// passa por ele (a menos de 0,05 mm). A superfície externa é desenrolada a partir da geratriz de
/// baixo (−Y local), girando no sentido de +X: o furo fica em x = posição ao longo da barra e
/// y = R·φ, com R o raio externo e φ o ângulo do lado do furo. Um furo diametral (que atravessa as
/// duas paredes) vem em duas faces cilíndricas, uma em cada parede, e vira dois furos a meia volta.
/// Fora disso (furo inclinado ou fora do centro) não há furo radial: volta `None`. O raio vem da cota
/// da seção (a caixa do OpenCASCADE é folgada pela tolerância).
fn furo_radial(c: &fenkra_kernel::Cylinder, ponto: V3, lo: V3, hi: V3, raio: f64) -> Option<FuroDstv> {
    if c.dir[2].abs() > 1e-3 {
        return None;
    }
    let centro = [(lo[0] + hi[0]) / 2., (lo[1] + hi[1]) / 2.];
    // distância entre o eixo do furo (no plano XY) e o eixo da barra
    let d2 = [c.dir[0], c.dir[1]];
    let n = (d2[0] * d2[0] + d2[1] * d2[1]).sqrt();
    let (ux, uy) = (d2[0] / n, d2[1] / n);
    let (ox, oy) = (c.origin[0] - centro[0], c.origin[1] - centro[1]);
    if (ox * uy - oy * ux).abs() > 0.05 {
        return None;
    }
    // o lado do furo: o da parede onde está a face
    let lado = (ponto[0] - centro[0]) * ux + (ponto[1] - centro[1]) * uy;
    let (sx, sy) = if lado >= 0. { (ux, uy) } else { (-ux, -uy) };
    // φ a partir de −Y, crescendo para +X
    let mut phi = sx.atan2(-sy);
    if phi < 0. {
        phi += 2. * std::f64::consts::PI;
    }
    // posição ao longo da barra: o ponto do eixo do furo mais perto do ponto da face
    let t = geo3d::dot(geo3d::sub(ponto, c.origin), c.dir);
    let o = geo3d::add(c.origin, geo3d::mul(c.dir, t));
    let r2 = |v: f64| (v * 100.).round() / 100.;
    Some(FuroDstv { face: 'v', x: r2(o[2] - lo[2]), y: r2(raio * phi), diametro: r2(2. * c.radius) })
}

/// Furos passantes do sólido: faces cilíndricas internas com eixo na direção X ou Y da seção.
fn furos(sh: &Shape, kind: SectionKind, raio: f64, faltas: &mut Vec<String>) -> Vec<FuroDstv> {
    let (Ok(top), Some((lo, hi))) = (sh.topology(), sh.bbox()) else { return vec![] };
    let meio = [(lo[0] + hi[0]) / 2., (lo[1] + hi[1]) / 2.];
    let mut out: Vec<FuroDstv> = vec![];
    let mut obliquo = false;
    for f in top.faces.iter().filter(|f| f.kind == 1) {
        let Ok(c) = sh.cylinder_at(f.point) else { continue };
        if !c.internal || c.dir[2].abs() > 0.999 {
            continue;
        }
        if kind == SectionKind::TuboRedondo || kind == SectionKind::BarraRedonda {
            match furo_radial(&c, f.point, lo, hi, raio) {
                Some(furo) => {
                    if !out.iter().any(|q| q.face == furo.face && (q.x - furo.x).abs() < 0.05 && (q.y - furo.y).abs() < 0.05 && (q.diametro - furo.diametro).abs() < 0.05) {
                        out.push(furo);
                    }
                }
                None => obliquo = true,
            }
            continue;
        }
        // centro do furo: o ponto do eixo mais perto do ponto da face
        let t = geo3d::dot(geo3d::sub(f.point, c.origin), c.dir);
        let o = geo3d::add(c.origin, geo3d::mul(c.dir, t));
        let (face, y) = if c.dir[0].abs() > 0.999 {
            let face = if matches!(kind, SectionKind::TuboQuadrado | SectionKind::TuboRetangular) && f.point[0] < meio[0] { 'h' } else { 'v' };
            (face, o[1] - lo[1])
        } else if c.dir[1].abs() > 0.999 {
            (if f.point[1] > meio[1] { 'o' } else { 'u' }, o[0] - lo[0])
        } else {
            obliquo = true;
            continue;
        };
        let furo = FuroDstv { face, x: ((o[2] - lo[2]) * 100.).round() / 100., y: (y * 100.).round() / 100., diametro: ((2. * c.radius) * 100.).round() / 100. };
        // um furo costuma vir em duas meias faces: fica um só por face, posição e diâmetro
        if !out.iter().any(|q| q.face == furo.face && (q.x - furo.x).abs() < 0.05 && (q.y - furo.y).abs() < 0.05 && (q.diametro - furo.diametro).abs() < 0.05) {
            out.push(furo);
        }
    }
    if obliquo {
        faltas.push("furo oblíquo (fora das direções da alma e da aba)".into());
    }
    out.sort_by(|a, b| a.face.cmp(&b.face).then(a.x.total_cmp(&b.x)).then(a.y.total_cmp(&b.y)));
    out
}

/// Caixa envolvente pelos vértices da malha: justa nas faces planas, ao contrário de `bbox`.
fn caixa_justa(sh: &Shape) -> Option<(V3, V3)> {
    let mesh = sh.mesh_angle(0.2, 0.2).ok()?;
    let mut lo = [f64::MAX; 3];
    let mut hi = [f64::MIN; 3];
    for v in mesh.positions.chunks(3) {
        for k in 0..3 {
            lo[k] = lo[k].min(v[k] as f64);
            hi[k] = hi[k].max(v[k] as f64);
        }
    }
    (lo[0] <= hi[0]).then_some((lo, hi))
}

/// Onde a reta paralela a Z por (px, py) entra e sai do sólido: o menor e o maior Z em que ela fura
/// a malha. Triângulos de pé (paralelos a Z) não contam: as bordas deles já estão nos vizinhos.
fn raio_z(mesh: &fenkra_kernel::Mesh, px: f64, py: f64) -> Option<(f64, f64)> {
    let p = |i: u32| {
        let k = i as usize * 3;
        [mesh.positions[k] as f64, mesh.positions[k + 1] as f64, mesh.positions[k + 2] as f64]
    };
    let (mut a0, mut a1) = (f64::MAX, f64::MIN);
    for t in mesh.indices.chunks(3) {
        let (a, b, c) = (p(t[0]), p(t[1]), p(t[2]));
        let det = (b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1]);
        if det.abs() < 1e-9 {
            continue;
        }
        let l1 = ((px - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (py - a[1])) / det;
        let l2 = ((b[0] - a[0]) * (py - a[1]) - (px - a[0]) * (b[1] - a[1])) / det;
        let tol = 1e-7;
        if l1 < -tol || l2 < -tol || l1 + l2 > 1. + tol {
            continue;
        }
        let z = a[2] + l1 * (b[2] - a[2]) + l2 * (c[2] - a[2]);
        a0 = a0.min(z);
        a1 = a1.max(z);
    }
    (a0 <= a1).then_some((a0, a1))
}

/// Tira um ponto que está na reta entre o anterior e o seguinte (o laço é fechado).
fn sem_colineares(pts: Vec<[f64; 2]>) -> Vec<[f64; 2]> {
    let mut v = pts;
    loop {
        let n = v.len();
        if n <= 3 {
            return v;
        }
        let k = (0..n).find(|&i| {
            let (a, b, c) = (v[(i + n - 1) % n], v[i], v[(i + 1) % n]);
            let cruz = (b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1]);
            let l = ((c[0] - a[0]).powi(2) + (c[1] - a[1]).powi(2)).sqrt().max(1e-9);
            cruz.abs() / l < 0.005
        });
        match k {
            Some(i) => {
                v.remove(i);
            }
            None => return v,
        }
    }
}

/// Contorno externo (bloco AK) dos perfis abertos I, U e L com entalhe ou recorte nas pontas.
///
/// Como funciona: cada face do DSTV é uma chapa do perfil (v = alma ou aba de pé, o e u = mesas ou
/// aba deitada). Para uma face, uma reta paralela ao eixo da barra corre pelo plano médio da chapa;
/// na posição `s` através da chapa (altura na alma, largura na mesa) ela entra no sólido em
/// `z0(s)` e sai em `z1(s)`. Com pontas planas, `z0` e `z1` são retas entre os vértices da malha,
/// então basta medir em cada vértice, um pouco antes e um pouco depois (o degrau de um entalhe fica
/// assim exato). O contorno é a borda do fim subindo e a do início descendo, no sentido
/// anti-horário que o DSTV pede, com x a partir do início da peça e y a partir da borda de baixo
/// (v: Y; o, u: X), como nos furos. A face de alma vê o perfil todo de lado (o entalhe das mesas
/// aparece nela); as mesas veem só a própria chapa.
///
/// A seção tem de estar na posição do modelo (sem giro): a caixa precisa bater com as cotas.
fn contornos_abertos(sh: &Shape, kind: SectionKind, dims: &[f64]) -> Vec<(char, Vec<[f64; 2]>)> {
    let g = |i: usize| dims.get(i).copied().unwrap_or(0.);
    let Ok(mesh) = sh.mesh_angle(0.2, 0.2) else { return vec![] };
    let Some((lo, hi)) = caixa_justa(sh) else { return vec![] };
    let (largura, altura) = (hi[0] - lo[0], hi[1] - lo[1]);
    let (b, h) = match kind {
        SectionKind::PerfilI | SectionKind::PerfilU => (g(1), g(0)),
        SectionKind::Cantoneira => (g(1), g(0)),
        _ => return vec![],
    };
    if (largura - b).abs() > 0.5 || (altura - h).abs() > 0.5 {
        return vec![];
    }
    let tem = |x: f64, y: f64| raio_z(&mesh, x, y).is_some();
    let mx = (lo[0] + hi[0]) / 2.;
    // (face, coordenada fixa, a reta corre em Y?)
    let faces: Vec<(char, f64, bool)> = match kind {
        SectionKind::PerfilI => {
            let tf = g(3);
            vec![('v', mx, true), ('o', hi[1] - tf / 2., false), ('u', lo[1] + tf / 2., false)]
        }
        SectionKind::PerfilU => {
            let (tw, tf) = (g(2), g(3));
            // alma à esquerda no modelo; espelhado, à direita
            let xa = if tem(lo[0] + tw / 2., (lo[1] + hi[1]) / 2.) { lo[0] + tw / 2. } else { hi[0] - tw / 2. };
            vec![('v', xa, true), ('o', hi[1] - tf / 2., false), ('u', lo[1] + tf / 2., false)]
        }
        _ => {
            let t = g(2);
            // a aba de pé atravessa a altura toda; a deitada, a largura toda
            let (x0, x1, y0, y1) = (lo[0] + t / 2., hi[0] - t / 2., lo[1] + t / 2., hi[1] - t / 2.);
            let xa = if tem(x0, y0) && tem(x0, y1) { x0 } else { x1 };
            if tem(x0, y0) && tem(x1, y0) {
                vec![('v', xa, true), ('u', lo[1] + t / 2., false)]
            } else {
                vec![('v', xa, true), ('o', hi[1] - t / 2., false)]
            }
        }
    };
    let eps = 0.01;
    let mut out = vec![];
    for (face, fixo, em_y) in faces {
        let (a, bb) = if em_y { (lo[1], hi[1]) } else { (lo[0], hi[0]) };
        // posições de quebra: os vértices da malha dentro da faixa
        let mut quebras: Vec<f64> = mesh.positions.chunks(3).map(|v| if em_y { v[1] as f64 } else { v[0] as f64 }).filter(|s| *s >= a - 1e-6 && *s <= bb + 1e-6).collect();
        quebras.push(a);
        quebras.push(bb);
        quebras.sort_by(f64::total_cmp);
        quebras.dedup_by(|x, y| (*x - *y).abs() < 1e-3);
        let medir = |s: f64| if em_y { raio_z(&mesh, fixo, s) } else { raio_z(&mesh, s, fixo) };
        // (s, z0, z1) dos dois lados de cada quebra
        let mut lados: Vec<(f64, f64, f64)> = vec![];
        for &s in &quebras {
            for d in [-eps, eps] {
                let q = (s + d).clamp(a + eps, bb - eps);
                if let Some((z0, z1)) = medir(q) {
                    lados.push((s.clamp(a, bb), z0, z1));
                }
            }
        }
        if lados.len() < 2 {
            continue;
        }
        let ponto = |z: f64, s: f64| [((z - lo[2]) * 100.).round() / 100., ((s - a) * 100.).round() / 100.];
        let mut pts: Vec<[f64; 2]> = lados.iter().map(|(s, _, z1)| ponto(*z1, *s)).collect();
        pts.extend(lados.iter().rev().map(|(s, z0, _)| ponto(*z0, *s)));
        pts.dedup();
        while pts.len() > 1 && pts.first() == pts.last() {
            pts.pop();
        }
        let mut pts = sem_colineares(pts);
        if pts.len() < 3 {
            continue;
        }
        let k0 = pts.iter().enumerate().min_by(|(_, p), (_, q)| (p[1], p[0]).partial_cmp(&(q[1], q[0])).unwrap_or(std::cmp::Ordering::Equal)).map(|(i, _)| i).unwrap_or(0);
        pts.rotate_left(k0);
        pts.push(pts[0]);
        out.push((face, pts));
    }
    out
}

/// Contorno externo (bloco AK) do tubo quadrado ou retangular com encaixe numa ou nas duas pontas.
///
/// Como funciona: a curva de encaixe (`st_encaixe_doc`, pele externa) é levada ao referencial local
/// do sólido do item (o mesmo dos furos: Z ao longo da barra, X e Y da seção). Cada ponto cai numa
/// das quatro faces pelo lado da caixa de que está mais perto (v = +X, h = −X, o = +Y, u = −Y), e
/// onde a curva passa de uma face para a vizinha entra o ponto do canto, com o Z interpolado pela
/// distância de cada ponto até a aresta. Em cada face o contorno é a tira da face fechada pelas duas
/// pontas, no sentido anti-horário que o DSTV pede para contorno externo: borda de baixo do início
/// ao fim, a ponta do fim subindo, borda de cima de volta e a ponta do início descendo. A ponta sem
/// encaixe só entra se for reta (sem corte inclinado no sólido); senão não há AK.
fn contornos_ak(doc: &Document, g: &StructGroup, kind: SectionKind, pontas_planas: &[Option<V3>; 2]) -> Vec<(char, Vec<[f64; 2]>)> {
    if !matches!(kind, SectionKind::TuboQuadrado | SectionKind::TuboRetangular) {
        return vec![];
    }
    let Some(r) = g.members.first() else { return vec![] };
    // caixa justa pelos vértices da malha (a caixa do OpenCASCADE vem folgada pela tolerância)
    let Some((lo, hi)) = caixa_justa(&g.shape) else { return vec![] };
    let Some(m) = crate::st_encaixe_doc::encaixes_da_estrutura(doc, &Default::default()).into_iter().find(|m| m.feature == r.feature && m.linha == r.index) else { return vec![] };
    let f = &r.frame;
    let fy = f.y();
    let local = |p: V3| {
        let d = geo3d::sub(p, f.origin);
        [geo3d::dot(d, f.x), geo3d::dot(d, fy), geo3d::dot(d, f.normal)]
    };
    let (mx, my) = ((lo[0] + hi[0]) / 2., (lo[1] + hi[1]) / 2.);
    let (ax, ay) = ((hi[0] - lo[0]) / 2., (hi[1] - lo[1]) / 2.);
    let face_de = |q: V3| {
        let (u, v) = ((q[0] - mx) / ax, (q[1] - my) / ay);
        if u.abs() >= v.abs() {
            if u > 0. { 'v' } else { 'h' }
        } else if v > 0. {
            'o'
        } else {
            'u'
        }
    };
    // canto entre duas faces vizinhas (X, Y), ou nada se forem opostas
    let canto = |a: char, b: char| -> Option<(f64, f64)> {
        let sx = |c: char| match c {
            'v' => Some(mx + ax),
            'h' => Some(mx - ax),
            _ => None,
        };
        let sy = |c: char| match c {
            'o' => Some(my + ay),
            'u' => Some(my - ay),
            _ => None,
        };
        Some((sx(a).or(sx(b))?, sy(a).or(sy(b))?))
    };
    // coordenadas da face no DSTV: x do início da peça; y da borda de baixo (v, h: Y; o, u: X)
    let na_face = |c: char, q: V3| -> [f64; 2] { [q[2] - lo[2], if matches!(c, 'v' | 'h') { q[1] - lo[1] } else { q[0] - lo[0] }] };
    // por ponta: face -> pontos (x, y) da face
    let mut por_ponta: Vec<(f64, std::collections::BTreeMap<char, Vec<[f64; 2]>>)> = vec![];
    for p in m.pontas() {
        let pts: Vec<V3> = p.corte.pontos.iter().map(|q| local(q.p3)).collect();
        if pts.len() < 3 {
            return vec![];
        }
        let mut mapa: std::collections::BTreeMap<char, Vec<[f64; 2]>> = Default::default();
        for i in 0..pts.len() {
            let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
            let (fa, fb) = (face_de(a), face_de(b));
            mapa.entry(fa).or_default().push(na_face(fa, a));
            if fa != fb {
                let Some((cx, cy)) = canto(fa, fb) else { return vec![] };
                let dist = |c: char, q: V3| if matches!(c, 'v' | 'h') { (q[1] - cy).abs() } else { (q[0] - cx).abs() };
                let (da, db) = (dist(fa, a), dist(fb, b));
                let t = if da + db > 1e-12 { da / (da + db) } else { 0.5 };
                let c3 = [cx, cy, a[2] + (b[2] - a[2]) * t];
                mapa.entry(fa).or_default().push(na_face(fa, c3));
                mapa.entry(fb).or_default().push(na_face(fb, c3));
            }
        }
        let z_medio = pts.iter().map(|q| q[2]).sum::<f64>() / pts.len() as f64;
        por_ponta.push((z_medio, mapa));
    }
    por_ponta.sort_by(|a, b| a.0.total_cmp(&b.0));
    let comprimento = hi[2] - lo[2];
    let (inicio, fim): (Option<&std::collections::BTreeMap<char, Vec<[f64; 2]>>>, Option<&std::collections::BTreeMap<char, Vec<[f64; 2]>>>) = match por_ponta.len() {
        2 => (Some(&por_ponta[0].1), Some(&por_ponta[1].1)),
        1 if por_ponta[0].0 - lo[2] < comprimento / 2. => (Some(&por_ponta[0].1), None),
        1 => (None, Some(&por_ponta[0].1)),
        _ => return vec![],
    };
    // ponta sem encaixe: só a reta (a normal plana tem de ser o eixo)
    let reta = |k: usize| pontas_planas[k].is_some_and(|n| n[2].abs() > 0.99999);
    if (inicio.is_none() && !reta(0)) || (fim.is_none() && !reta(1)) {
        return vec![];
    }
    let mut out = vec![];
    for c in ['v', 'o', 'u', 'h'] {
        let largura = if matches!(c, 'v' | 'h') { hi[1] - lo[1] } else { hi[0] - lo[0] };
        let ordenada = |mapa: Option<&std::collections::BTreeMap<char, Vec<[f64; 2]>>>, x_reto: f64| -> Vec<[f64; 2]> {
            let mut v = match mapa.and_then(|m| m.get(&c)) {
                Some(v) => v.clone(),
                None if mapa.is_none() => vec![[x_reto, 0.], [x_reto, largura]],
                None => vec![],
            };
            v.sort_by(|a, b| a[1].total_cmp(&b[1]));
            v.dedup_by(|a, b| (a[0] - b[0]).abs() < 1e-6 && (a[1] - b[1]).abs() < 1e-6);
            v
        };
        let (a, b) = (ordenada(inicio, 0.), ordenada(fim, comprimento));
        if a.len() < 2 || b.len() < 2 {
            return vec![];
        }
        let arred = |q: [f64; 2]| [(q[0] * 100.).round() / 100., (q[1] * 100.).round() / 100.];
        let mut pts: Vec<[f64; 2]> = vec![];
        pts.extend(b.iter().copied());
        pts.extend(a.iter().rev().copied());
        let mut pts: Vec<[f64; 2]> = pts.into_iter().map(arred).collect();
        pts.dedup();
        // começa no canto de baixo do início e fecha repetindo o primeiro ponto
        let k0 = pts.iter().enumerate().min_by(|(_, p), (_, q)| (p[1], p[0]).partial_cmp(&(q[1], q[0])).unwrap_or(std::cmp::Ordering::Equal)).map(|(i, _)| i).unwrap_or(0);
        pts.rotate_left(k0);
        pts.push(pts[0]);
        out.push((c, pts));
    }
    out
}

/// Peças da estrutura para a máquina: um por item de membro da lista de corte.
pub fn pecas_nc(doc: &Document) -> Vec<PecaNc> {
    let grupos = structural_groups(doc);
    let lista = cut_list(doc);
    let vars = doc.eval_params().unwrap_or_default();
    let mut out = vec![];
    for g in grupos.iter().filter(|g| g.signature.kind == StructItem::Member) {
        let Some(FeatureKind::StructMember(m)) = doc.feature(g.members[0].feature).map(|f| &f.kind) else { continue };
        let dims: Vec<f64> = m.dims.iter().map(|e| ev(e, &vars, "").unwrap_or(0.)).collect();
        let linha = lista.iter().find(|r| r.item == g.item);
        let material = linha.map(|r| r.material.clone()).unwrap_or_else(|| doc.props.material.clone());
        let densidade = fenkra_core::materiais::densidade(&material);
        // massa e pintura por metro, da seção nominal
        let (kg_m, pintura_m) = match structural::section(m.kind, &dims) {
            Ok((laços, ..)) if m.kind == SectionKind::BarraChata => {
                let _ = laços;
                (dims.get(1).copied().unwrap_or(0.) * densidade, 2.)
            }
            Ok((laços, ..)) => {
                let fr = Frame { origin: [0.; 3], x: [1., 0., 0.], normal: [0., 0., 1.] };
                let area = Shape::face(&fr, &laços).map(|f| f.mass().area).unwrap_or(0.);
                // pintura: só o contorno externo (o primeiro laço), pela lateral de 1 m de barra
                let lateral = Shape::face(&fr, &laços[..1])
                    .and_then(|f| f.extrude([0., 0., 1.], 1000., 0.))
                    .map(|p| p.mass().area - 2. * Shape::face(&fr, &laços[..1]).map(|f| f.mass().area).unwrap_or(0.))
                    .unwrap_or(0.);
                (area * densidade * 1e-3, lateral / 1e6)
            }
            Err(_) => (0., 0.),
        };
        let pontas = normais_das_pontas(&g.shape);
        let mut faltas = vec![];
        let contornos = match m.kind {
            _ if pontas.iter().all(Option::is_some) => vec![],
            SectionKind::PerfilI | SectionKind::PerfilU | SectionKind::Cantoneira => contornos_abertos(&g.shape, m.kind, &dims),
            _ => contornos_ak(doc, g, m.kind, &pontas),
        };
        if pontas.iter().any(Option::is_none) && contornos.is_empty() {
            faltas.push("ponta que não é plana (encaixe ou entalhe): o contorno AK não é gravado".into());
        }
        let cortes = [
            pontas[0].map(|n| angulo_no_plano(n, 1)).unwrap_or(0.),
            pontas[1].map(|n| angulo_no_plano(n, 1)).unwrap_or(0.),
            pontas[0].map(|n| angulo_no_plano(n, 0)).unwrap_or(0.),
            pontas[1].map(|n| angulo_no_plano(n, 0)).unwrap_or(0.),
        ];
        let furos = furos(&g.shape, m.kind, dims.first().copied().unwrap_or(0.) / 2., &mut faltas);
        out.push(PecaNc {
            item: g.item,
            designacao: if m.designation.trim().is_empty() { m.family.clone() } else { m.designation.clone() },
            material,
            quantidade: g.members.len(),
            kind: m.kind,
            dims,
            comprimento: linha.map(|r| r.length).unwrap_or(g.signature.length_mm()),
            kg_m,
            pintura_m,
            cortes,
            furos,
            contornos,
            faltas,
            forma: g.shape.clone(),
        });
    }
    out
}

/// Letra sem acento (o arquivo DSTV é ASCII).
fn ascii(c: char) -> char {
    const DE: &str = "áàâãäéèêëíìîïóòôõöúùûüçñÁÀÂÃÄÉÈÊËÍÌÎÏÓÒÔÕÖÚÙÛÜÇÑ";
    const PARA: &str = "aaaaaeeeeiiiiooooouuuucnAAAAAEEEEIIIIOOOOOUUUUCN";
    match DE.chars().position(|x| x == c) {
        Some(i) => PARA.chars().nth(i).unwrap_or('_'),
        None if c.is_ascii() && !c.is_ascii_control() => c,
        None => '_',
    }
}

/// Texto livre do DSTV: ASCII, até 80 caracteres, sem quebra; espaço vira sublinhado para não separar.
fn livre(t: &str) -> String {
    let t: String = t.trim().chars().map(|c| if c.is_whitespace() { '_' } else { ascii(c) }).take(80).collect();
    if t.is_empty() {
        "-".into()
    } else {
        t
    }
}

/// Arquivo NC1 de uma peça.
pub fn nc1(p: &PecaNc, pedido: &str, desenho: &str) -> String {
    let mut s = String::from("ST\n");
    s += &format!("** Fenkra: item {} ({})\n", p.item, p.designacao);
    let c = cotas_dstv(p.kind, &p.dims);
    let linhas: Vec<String> = vec![
        livre(pedido),
        livre(desenho),
        "1".into(),
        p.item.to_string(),
        livre(&p.material),
        p.quantidade.to_string(),
        livre(&p.designacao),
        codigo_dstv(p.kind).into(),
        format!("{:.2}", p.comprimento),
        format!("{:.2}", c[0]),
        format!("{:.2}", c[1]),
        format!("{:.2}", c[2]),
        format!("{:.2}", c[3]),
        format!("{:.2}", 0.),
        format!("{:.3}", p.kg_m),
        format!("{:.3}", p.pintura_m),
        format!("{:.3}", p.cortes[0]),
        format!("{:.3}", p.cortes[1]),
        format!("{:.3}", p.cortes[2]),
        format!("{:.3}", p.cortes[3]),
        livre(&format!("item_{}", p.item)),
        "-".into(),
        "-".into(),
        "-".into(),
    ];
    for l in linhas {
        s += &format!("  {l}\n");
    }
    for f in &p.faltas {
        s += &format!("** sem descrição: {f}\n");
    }
    if !p.contornos.is_empty() {
        // contorno externo: a letra da face em cada linha, a referência da cota (u = borda de baixo)
        // no primeiro ponto, raio 0 (trechos retos)
        s += "AK\n";
        for (face, pts) in &p.contornos {
            for (i, q) in pts.iter().enumerate() {
                let r = if i == 0 { "u" } else { " " };
                s += &format!("  {face} {:>10.2}{r} {:>10.2} {:>8.2}\n", q[0], q[1], 0.);
            }
        }
    }
    if !p.furos.is_empty() {
        s += "BO\n";
        for f in &p.furos {
            let r = if matches!(f.face, 'v' | 'h') { 'u' } else { 's' };
            s += &format!("  {} {:>10.2}{} {:>10.2} {:>7.2} {:>6.2}\n", f.face, f.x, r, f.y, f.diametro, 0.);
        }
    }
    s += "EN\n";
    s
}

/// Nome de arquivo seguro para o item.
pub fn nome_do_arquivo(p: &PecaNc, ext: &str) -> String {
    let n: String = p.designacao.chars().map(|c| if c.is_alphanumeric() || c == '-' { c } else { '_' }).collect();
    format!("item{:02}_{n}.{ext}", p.item)
}

/// Grava, na pasta, o NC1 e o STEP de cada item. Devolve os arquivos gravados.
pub fn gravar_saidas(doc: &Document, pasta: &std::path::Path) -> Result<Vec<std::path::PathBuf>, String> {
    let pecas = pecas_nc(doc);
    if pecas.is_empty() {
        return Err("a peça não tem membros estruturais calculados".into());
    }
    std::fs::create_dir_all(pasta).map_err(|e| e.to_string())?;
    let mut out = vec![];
    for p in &pecas {
        let nc = pasta.join(nome_do_arquivo(p, "nc1"));
        std::fs::write(&nc, nc1(p, &doc.name, &doc.name)).map_err(|e| e.to_string())?;
        out.push(nc);
        let st = pasta.join(nome_do_arquivo(p, "step"));
        p.forma.export_step(&st.to_string_lossy()).map_err(|e| e.0)?;
        out.push(st);
    }
    Ok(out)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn codigos_e_cotas_do_cabecalho() {
        assert_eq!(codigo_dstv(SectionKind::TuboRedondo), "RO");
        assert_eq!(codigo_dstv(SectionKind::BarraRedonda), "RU");
        assert_eq!(codigo_dstv(SectionKind::TuboRetangular), "M");
        // W200x15: altura 200, aba 100, mesa 5,2, alma 4,3
        assert_eq!(cotas_dstv(SectionKind::PerfilI, &[200., 100., 4.3, 5.2]), [200., 100., 5.2, 4.3]);
        assert_eq!(cotas_dstv(SectionKind::TuboRetangular, &[80., 40., 3.]), [40., 80., 3., 3.]);
        assert_eq!(livre("Aço S 275"), "Aco_S_275");
    }

    #[test]
    fn furo_passante_na_alma_vira_bo_na_face_v() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        // chapa de 10 mm em pé (alma): X é a espessura, Y a altura de 200, Z o comprimento de 1000
        let alma = Shape::box_tool([-5., 0., 0.], [1., 0., 0.], [0., 0., 1.], [10., 200., 1000.]).unwrap();
        let fr = Frame { origin: [-50., 120., 300.], x: [0., 1., 0.], normal: [1., 0., 0.] };
        let pino = Shape::face(&fr, &[vec![fenkra_kernel::Seg2::Circle([0., 0.], 9.)]]).unwrap().extrude([1., 0., 0.], 100., 0.).unwrap();
        let furada = alma.boolean(&pino, fenkra_kernel::Bool::Cut).unwrap();
        let mut faltas = vec![];
        let f = furos(&furada, SectionKind::PerfilI, 0., &mut faltas);
        assert_eq!(f, vec![FuroDstv { face: 'v', x: 300., y: 120., diametro: 18. }], "{faltas:?}");
        assert!(faltas.is_empty());
    }

    /// Tubo Ø60,3 × 3,6 de 1000 com um furo diametral Ø12 na direção X a 300 da ponta e um radial
    /// Ø10 na direção Y (só a parede de cima) a 700: na superfície desenrolada a partir da geratriz
    /// de baixo, o lado +X fica a R·π/2, o −X a R·3π/2 e o +Y a R·π, com R = 30,15.
    #[test]
    fn furo_de_tubo_redondo_vai_na_superficie_desenrolada() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let fr = Frame { origin: [0., 0., 0.], x: [1., 0., 0.], normal: [0., 0., 1.] };
        let tubo = Shape::face(&fr, &[vec![fenkra_kernel::Seg2::Circle([0., 0.], 30.15)], vec![fenkra_kernel::Seg2::Circle([0., 0.], 26.55)]]).unwrap().extrude([0., 0., 1.], 1000., 0.).unwrap();
        let fx = Frame { origin: [-50., 0., 300.], x: [0., 1., 0.], normal: [1., 0., 0.] };
        let pino = Shape::face(&fx, &[vec![fenkra_kernel::Seg2::Circle([0., 0.], 6.)]]).unwrap().extrude([1., 0., 0.], 100., 0.).unwrap();
        let fy = Frame { origin: [0., 10., 700.], x: [1., 0., 0.], normal: [0., 1., 0.] };
        let pino_y = Shape::face(&fy, &[vec![fenkra_kernel::Seg2::Circle([0., 0.], 5.)]]).unwrap().extrude([0., 1., 0.], 40., 0.).unwrap();
        let furado = tubo.boolean(&pino, fenkra_kernel::Bool::Cut).unwrap().boolean(&pino_y, fenkra_kernel::Bool::Cut).unwrap();
        let mut faltas = vec![];
        let f = furos(&furado, SectionKind::TuboRedondo, 30.15, &mut faltas);
        assert!(faltas.is_empty(), "{faltas:?}");
        let r = 30.15;
        let pi = std::f64::consts::PI;
        let esperado = [(300., r * pi / 2., 12.), (300., r * 3. * pi / 2., 12.), (700., r * pi, 10.)];
        assert_eq!(f.len(), 3, "{f:?}");
        for (x, y, d) in esperado {
            assert!(f.iter().any(|q| q.face == 'v' && (q.x - x).abs() < 0.2 && (q.y - y).abs() < 0.05 && (q.diametro - d).abs() < 0.01), "{x} {y} {d}: {f:?}");
        }
    }

    #[test]
    fn angulo_da_ponta_no_plano_da_alma() {
        let s = 45f64.to_radians().sin();
        assert!((angulo_no_plano([0., s, s], 1) - 45.).abs() < 1e-9);
        assert!(angulo_no_plano([0., s, s], 0).abs() < 1e-9);
    }
}
