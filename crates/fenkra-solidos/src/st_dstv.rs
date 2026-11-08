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
//! Fora desta versão, e dito no relatório de cada arquivo: contorno externo (AK) dos entalhes e dos
//! encaixes, furos no tubo redondo (que pedem a planificação da superfície) e furos oblíquos.

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
    }
    [0, 1].map(|k| {
        let m = geo3d::unit(soma[k])?;
        normais[k].iter().all(|u| geo3d::dot(*u, m) > 0.99996).then_some(m)
    })
}

/// Ângulo do corte visto num plano: desvio da normal da ponta em relação ao eixo, só pela componente
/// `c` (0 = x, vista da aba; 1 = y, vista da alma). Positivo no sentido anti-horário.
fn angulo_no_plano(n: V3, c: usize) -> f64 {
    let a = n[c].atan2(n[2].abs()).to_degrees();
    (a * 1000.).round() / 1000.
}

/// Furos passantes do sólido: faces cilíndricas internas com eixo na direção X ou Y da seção.
fn furos(sh: &Shape, kind: SectionKind, faltas: &mut Vec<String>) -> Vec<FuroDstv> {
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
            if !faltas.iter().any(|x| x.contains("redondo")) {
                faltas.push("furo em perfil redondo (pede a planificação da superfície)".into());
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
        if pontas.iter().any(Option::is_none) {
            faltas.push("ponta que não é plana (encaixe ou entalhe): o contorno AK não é gravado".into());
        }
        let cortes = [
            pontas[0].map(|n| angulo_no_plano(n, 1)).unwrap_or(0.),
            pontas[1].map(|n| angulo_no_plano(n, 1)).unwrap_or(0.),
            pontas[0].map(|n| angulo_no_plano(n, 0)).unwrap_or(0.),
            pontas[1].map(|n| angulo_no_plano(n, 0)).unwrap_or(0.),
        ];
        let furos = furos(&g.shape, m.kind, &mut faltas);
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
        let f = furos(&furada, SectionKind::PerfilI, &mut faltas);
        assert_eq!(f, vec![FuroDstv { face: 'v', x: 300., y: 120., diametro: 18. }], "{faltas:?}");
        assert!(faltas.is_empty());
    }

    #[test]
    fn angulo_da_ponta_no_plano_da_alma() {
        let s = 45f64.to_radians().sin();
        assert!((angulo_no_plano([0., s, s], 1) - 45.).abs() < 1e-9);
        assert!(angulo_no_plano([0., s, s], 0).abs() < 1e-9);
    }
}
