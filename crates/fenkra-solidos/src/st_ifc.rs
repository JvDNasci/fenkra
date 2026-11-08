// SPDX-License-Identifier: LGPL-2.1-or-later
//! B134: estrutura de aço em IFC4. Cada trecho de membro vira um `IfcMember`, cada placa e reforço um
//! `IfcPlate` e cada cordão sólido um `IfcFastener` de solda. O corpo é a malha do sólido já cortado
//! (meia-esquadria, encaixe, furos), uma vez por item da lista de corte num `IfcRepresentationMap`, e
//! cada peça entra no seu lugar pelo referencial local do membro. O perfil vai como
//! `IfcMaterialProfileSet` com a seção paramétrica da norma IFC (I, U, L, T, tubo, barra), e as
//! quantidades de base (comprimento e volume) num `IfcElementQuantity`: quem abre o arquivo vê o
//! mesmo perfil e o mesmo comprimento da lista de corte.
//!
//! O arquivo é texto STEP físico escrito aqui, sem biblioteca externa (ADR 0082).

use super::*;
use std::collections::BTreeMap;
use std::fmt::Write as _;

/// Resultado da exportação: o texto e quantas peças de cada tipo entraram.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct IfcEstrutura {
    pub texto: String,
    pub membros: usize,
    pub placas: usize,
    pub soldas: usize,
}

/// Texto STEP: aspas dobradas, barra invertida dobrada e fora do ASCII pela forma `\X2\`.
fn codificar(t: &str) -> String {
    let mut s = String::new();
    for c in t.chars() {
        match c {
            '\'' => s.push_str("''"),
            '\\' => s.push_str("\\\\"),
            c if c.is_ascii() && !c.is_control() => s.push(c),
            c => {
                s.push_str("\\X2\\");
                let mut u = [0u16; 2];
                for q in c.encode_utf16(&mut u) {
                    let _ = write!(s, "{q:04X}");
                }
                s.push_str("\\X0\\");
            }
        }
    }
    s
}

/// Real do STEP físico: sempre com ponto.
fn real(v: f64) -> String {
    let v = if v.abs() < 1e-12 { 0. } else { v };
    let t = format!("{}", (v * 1e6).round() / 1e6);
    if t.contains('.') || t.contains('E') || t.contains('e') {
        t
    } else {
        format!("{t}.")
    }
}

/// Identificador global de 22 letras (alfabeto de 64 do IFC), único no arquivo.
fn guid(n: usize, semente: u64) -> String {
    const ALFABETO: &[u8; 64] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz_$";
    let mut h = blake3::Hasher::new();
    h.update(&semente.to_le_bytes());
    h.update(&(n as u64).to_le_bytes());
    let b = h.finalize();
    let mut s = String::with_capacity(22);
    // a primeira letra só vai até 3 (o GUID tem 128 bits)
    s.push(ALFABETO[(b.as_bytes()[0] & 3) as usize] as char);
    for k in 1..22 {
        s.push(ALFABETO[(b.as_bytes()[k] & 63) as usize] as char);
    }
    s
}

struct Saida {
    s: String,
    n: u64,
    g: usize,
    semente: u64,
}

impl Saida {
    fn linha(&mut self, corpo: impl AsRef<str>) -> u64 {
        self.n += 1;
        let _ = writeln!(self.s, "#{}={};", self.n, corpo.as_ref());
        self.n
    }

    fn id(&mut self) -> String {
        self.g += 1;
        guid(self.g, self.semente)
    }

    fn ponto(&mut self, p: V3) -> u64 {
        self.linha(format!("IFCCARTESIANPOINT(({},{},{}))", real(p[0]), real(p[1]), real(p[2])))
    }

    fn direcao(&mut self, d: V3) -> u64 {
        self.linha(format!("IFCDIRECTION(({},{},{}))", real(d[0]), real(d[1]), real(d[2])))
    }

    fn eixos(&mut self, o: V3, z: V3, x: V3) -> u64 {
        let (o, z, x) = (self.ponto(o), self.direcao(z), self.direcao(x));
        self.linha(format!("IFCAXIS2PLACEMENT3D(#{o},#{z},#{x})"))
    }
}

/// Seção paramétrica do IFC para o tipo e as cotas do membro (em mm). `None` quando falta cota.
pub fn perfil_ifc(kind: SectionKind, d: &[f64], nome: &str) -> Option<String> {
    let need = structural::dim_names(kind).len();
    if d.len() < need || d.iter().take(need).any(|v| !(*v > 0.)) {
        return None;
    }
    let n = codificar(nome);
    let r = real;
    Some(match kind {
        SectionKind::TuboQuadrado => format!("IFCRECTANGLEHOLLOWPROFILEDEF(.AREA.,'{n}',$,{},{},{},$,$)", r(d[0]), r(d[0]), r(d[1])),
        SectionKind::TuboRetangular => format!("IFCRECTANGLEHOLLOWPROFILEDEF(.AREA.,'{n}',$,{},{},{},$,$)", r(d[0]), r(d[1]), r(d[2])),
        SectionKind::TuboRedondo => format!("IFCCIRCLEHOLLOWPROFILEDEF(.AREA.,'{n}',$,{},{})", r(d[0] / 2.), r(d[1])),
        // altura, aba, alma, mesa
        SectionKind::PerfilI => format!("IFCISHAPEPROFILEDEF(.AREA.,'{n}',$,{},{},{},{},$,$,$)", r(d[1]), r(d[0]), r(d[2]), r(d[3])),
        SectionKind::PerfilU => format!("IFCUSHAPEPROFILEDEF(.AREA.,'{n}',$,{},{},{},{},$,$,$)", r(d[0]), r(d[1]), r(d[2]), r(d[3])),
        SectionKind::PerfilT => format!("IFCTSHAPEPROFILEDEF(.AREA.,'{n}',$,{},{},{},{},$,$,$,$,$)", r(d[0]), r(d[1]), r(d[2]), r(d[3])),
        // aba a, aba b, espessura
        SectionKind::Cantoneira => format!("IFCLSHAPEPROFILEDEF(.AREA.,'{n}',$,{},{},{},$,$,$)", r(d[0]), r(d[1]), r(d[2])),
        SectionKind::BarraChata => format!("IFCRECTANGLEPROFILEDEF(.AREA.,'{n}',$,{},{})", r(d[0]), r(d[1])),
        SectionKind::BarraQuadrada => format!("IFCRECTANGLEPROFILEDEF(.AREA.,'{n}',$,{},{})", r(d[0]), r(d[0])),
        SectionKind::BarraRedonda => format!("IFCCIRCLEPROFILEDEF(.AREA.,'{n}',$,{})", r(d[0] / 2.)),
    })
}

/// Malha do sólido com os vértices soldados (o conjunto de faces fica fechado).
fn malha_soldada(sh: &Shape) -> Option<(Vec<V3>, Vec<[usize; 3]>)> {
    let m = sh.mesh_angle(0.1, 0.25).ok()?;
    let mut pos: Vec<V3> = vec![];
    let mut por: HashMap<[i64; 3], usize> = HashMap::new();
    let mut mapa = Vec::with_capacity(m.positions.len() / 3);
    for p in m.positions.chunks(3) {
        let v = [p[0] as f64, p[1] as f64, p[2] as f64];
        let k = v.map(|c| (c * 1e4).round() as i64);
        let i = *por.entry(k).or_insert_with(|| {
            pos.push(v);
            pos.len() - 1
        });
        mapa.push(i);
    }
    let tris: Vec<[usize; 3]> = m
        .indices
        .chunks(3)
        .map(|t| [mapa[t[0] as usize], mapa[t[1] as usize], mapa[t[2] as usize]])
        .filter(|t| t[0] != t[1] && t[1] != t[2] && t[0] != t[2])
        .collect();
    (!tris.is_empty()).then_some((pos, tris))
}

/// Estrutura de aço da peça em IFC4 (unidade mm). Erro quando não há nenhum item estrutural.
pub fn ifc_da_estrutura(doc: &Document) -> Result<IfcEstrutura, String> {
    let itens = structural_members(doc);
    if itens.is_empty() {
        return Err("a peça não tem membros, placas nem soldas estruturais".into());
    }
    let grupos = group_members(&itens);
    let vars = doc.eval_params().unwrap_or_default();
    let semente = u64::from_le_bytes(blake3::hash(doc.name.as_bytes()).as_bytes()[..8].try_into().unwrap_or([0; 8]));
    let mut o = Saida { s: String::new(), n: 0, g: 0, semente };

    let mm = o.linha("IFCSIUNIT(*,.LENGTHUNIT.,.MILLI.,.METRE.)");
    let m2 = o.linha("IFCSIUNIT(*,.AREAUNIT.,$,.SQUARE_METRE.)");
    let m3 = o.linha("IFCSIUNIT(*,.VOLUMEUNIT.,$,.CUBIC_METRE.)");
    let rad = o.linha("IFCSIUNIT(*,.PLANEANGLEUNIT.,$,.RADIAN.)");
    let unidades = o.linha(format!("IFCUNITASSIGNMENT((#{mm},#{m2},#{m3},#{rad}))"));
    let origem = o.eixos([0.; 3], [0., 0., 1.], [1., 0., 0.]);
    let contexto = o.linha(format!("IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-5,#{origem},$)"));
    let corpo_ctx = o.linha(format!("IFCGEOMETRICREPRESENTATIONSUBCONTEXT('Body','Model',*,*,*,*,#{contexto},$,.MODEL_VIEW.,$)"));
    let nome_doc = if doc.name.trim().is_empty() { "Estrutura".to_string() } else { doc.name.clone() };
    let id = o.id();
    let projeto = o.linha(format!("IFCPROJECT('{id}',$,'{}',$,$,$,$,(#{contexto}),#{unidades})", codificar(&nome_doc)));
    let lugar_site = o.linha(format!("IFCLOCALPLACEMENT($,#{origem})"));
    let id = o.id();
    let site = o.linha(format!("IFCSITE('{id}',$,'Terreno',$,$,#{lugar_site},$,$,.ELEMENT.,$,$,$,$,$)"));
    let lugar_predio = o.linha(format!("IFCLOCALPLACEMENT(#{lugar_site},#{origem})"));
    let id = o.id();
    let predio = o.linha(format!("IFCBUILDING('{id}',$,'{}',$,$,#{lugar_predio},$,$,.ELEMENT.,$,$,$)", codificar("Edifício")));
    let lugar_andar = o.linha(format!("IFCLOCALPLACEMENT(#{lugar_predio},#{origem})"));
    let id = o.id();
    let andar = o.linha(format!("IFCBUILDINGSTOREY('{id}',$,'Pavimento',$,$,#{lugar_andar},$,$,.ELEMENT.,$)"));
    for (a, b) in [(projeto, site), (site, predio), (predio, andar)] {
        let id = o.id();
        o.linha(format!("IFCRELAGGREGATES('{id}',$,$,$,#{a},(#{b}))"));
    }

    let mut produtos: Vec<u64> = vec![];
    let mut por_material: BTreeMap<String, Vec<u64>> = BTreeMap::new();
    let mut por_perfil: BTreeMap<String, (String, Vec<u64>)> = BTreeMap::new();
    let mut materiais: HashMap<String, u64> = HashMap::new();
    let mut saida = IfcEstrutura::default();
    for g in &grupos {
        let Some((pos, tris)) = malha_soldada(&g.shape) else { continue };
        let pontos: Vec<String> = pos.iter().map(|p| format!("({},{},{})", real(p[0]), real(p[1]), real(p[2]))).collect();
        let lista = o.linha(format!("IFCCARTESIANPOINTLIST3D(({}),$)", pontos.join(",")));
        let faces: Vec<String> = tris.iter().map(|t| format!("({},{},{})", t[0] + 1, t[1] + 1, t[2] + 1)).collect();
        let conjunto = o.linha(format!("IFCTRIANGULATEDFACESET(#{lista},$,.T.,({}),$)", faces.join(",")));
        let forma = o.linha(format!("IFCSHAPEREPRESENTATION(#{corpo_ctx},'Body','Tessellation',(#{conjunto}))"));
        let mapa = o.linha(format!("IFCREPRESENTATIONMAP(#{origem},#{forma})"));
        let material = if g.signature.material.is_empty() { doc.props.material.trim().to_string() } else { g.signature.material.clone() };
        let material = if material.is_empty() { "Aço".to_string() } else { material };
        // perfil paramétrico: o do primeiro membro do grupo
        let perfil = (g.signature.kind == StructItem::Member)
            .then(|| match doc.feature(g.members[0].feature).map(|f| &f.kind) {
                Some(FeatureKind::StructMember(m)) => {
                    let dims: Vec<f64> = m.dims.iter().map(|e| ev(e, &vars, "").unwrap_or(0.)).collect();
                    perfil_ifc(m.kind, &dims, &format!("{} {}", m.family, m.designation).trim().to_string())
                }
                _ => None,
            })
            .flatten();
        for r in &g.members {
            let operador_o = o.ponto([0.; 3]);
            let operador = o.linha(format!("IFCCARTESIANTRANSFORMATIONOPERATOR3D($,$,#{operador_o},1.,$)"));
            let item = o.linha(format!("IFCMAPPEDITEM(#{mapa},#{operador})"));
            let rep = o.linha(format!("IFCSHAPEREPRESENTATION(#{corpo_ctx},'Body','MappedRepresentation',(#{item}))"));
            let definicao = o.linha(format!("IFCPRODUCTDEFINITIONSHAPE($,$,(#{rep}))"));
            let eixos = o.eixos(r.frame.origin, r.frame.normal, r.frame.x);
            let lugar = o.linha(format!("IFCLOCALPLACEMENT(#{lugar_andar},#{eixos})"));
            let nome = codificar(&g.name);
            let tag = g.item;
            let id = o.id();
            let p = match g.signature.kind {
                StructItem::Member => {
                    saida.membros += 1;
                    o.linha(format!("IFCMEMBER('{id}',$,'{nome}',$,$,#{lugar},#{definicao},'{tag}',.MEMBER.)"))
                }
                StructItem::Plate | StructItem::Gusset => {
                    saida.placas += 1;
                    let tipo = if g.signature.kind == StructItem::Plate { "Placa" } else { "Reforço" };
                    o.linha(format!("IFCPLATE('{id}',$,'{nome}',$,'{}',#{lugar},#{definicao},'{tag}',.USERDEFINED.)", codificar(tipo)))
                }
                StructItem::Weld => {
                    saida.soldas += 1;
                    o.linha(format!("IFCFASTENER('{id}',$,'{nome}',$,$,#{lugar},#{definicao},'{tag}',.WELD.)"))
                }
            };
            produtos.push(p);
            if g.signature.kind != StructItem::Weld {
                match &perfil {
                    Some(pf) => por_perfil.entry(format!("{material}|{}", g.name)).or_insert_with(|| (pf.clone(), vec![])).1.push(p),
                    None => por_material.entry(material.clone()).or_default().push(p),
                }
            }
            // quantidades de base: comprimento de corte (membros) e volume líquido
            let volume = g.signature.volume as f64 * 1e-9;
            let mut q = vec![];
            if g.signature.kind == StructItem::Member {
                q.push(o.linha(format!("IFCQUANTITYLENGTH('Length',$,$,{},$)", real(g.signature.length_mm()))));
            }
            q.push(o.linha(format!("IFCQUANTITYVOLUME('NetVolume',$,$,{},$)", real(volume))));
            let qto = match g.signature.kind {
                StructItem::Member => "Qto_MemberBaseQuantities",
                StructItem::Plate | StructItem::Gusset => "Qto_PlateBaseQuantities",
                StructItem::Weld => "Qto_FastenerBaseQuantities",
            };
            let lista_q: Vec<String> = q.iter().map(|k| format!("#{k}")).collect();
            let id = o.id();
            let conj = o.linha(format!("IFCELEMENTQUANTITY('{id}',$,'{qto}',$,$,({}))", lista_q.join(",")));
            let id = o.id();
            o.linha(format!("IFCRELDEFINESBYPROPERTIES('{id}',$,$,$,(#{p}),#{conj})"));
        }
    }
    if produtos.is_empty() {
        return Err("nenhum sólido estrutural pôde ser convertido em malha".into());
    }
    let mut material_de = |o: &mut Saida, nome: &str| -> u64 {
        if let Some(m) = materiais.get(nome) {
            return *m;
        }
        let m = o.linha(format!("IFCMATERIAL('{}',$,'Steel')", codificar(nome)));
        materiais.insert(nome.to_string(), m);
        m
    };
    for (chave, (perfil, objs)) in &por_perfil {
        let (material, nome) = chave.split_once('|').unwrap_or((chave, ""));
        let mat = material_de(&mut o, material);
        let pf = o.linha(perfil);
        let mp = o.linha(format!("IFCMATERIALPROFILE('{}',$,#{mat},#{pf},$,$)", codificar(nome)));
        let set = o.linha(format!("IFCMATERIALPROFILESET('{}',$,(#{mp}),$)", codificar(nome)));
        let lista: Vec<String> = objs.iter().map(|p| format!("#{p}")).collect();
        let id = o.id();
        o.linha(format!("IFCRELASSOCIATESMATERIAL('{id}',$,$,$,({}),#{set})", lista.join(",")));
    }
    for (material, objs) in &por_material {
        let mat = material_de(&mut o, material);
        let lista: Vec<String> = objs.iter().map(|p| format!("#{p}")).collect();
        let id = o.id();
        o.linha(format!("IFCRELASSOCIATESMATERIAL('{id}',$,$,$,({}),#{mat})", lista.join(",")));
    }
    let lista: Vec<String> = produtos.iter().map(|p| format!("#{p}")).collect();
    let id = o.id();
    o.linha(format!("IFCRELCONTAINEDINSPATIALSTRUCTURE('{id}',$,$,$,({}),#{andar})", lista.join(",")));
    saida.texto = format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION(('ViewDefinition [ReferenceView_V1.2]'),'2;1');\nFILE_NAME('{}','',(''),(''),'Fenkra','Fenkra','');\nFILE_SCHEMA(('IFC4'));\nENDSEC;\nDATA;\n{}ENDSEC;\nEND-ISO-10303-21;\n",
        codificar(&nome_doc),
        o.s
    );
    Ok(saida)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn perfis_parametricos_na_ordem_do_ifc() {
        // I: largura total (aba) antes da altura
        let i = perfil_ifc(SectionKind::PerfilI, &[200., 100., 4.3, 5.2], "W200x15").unwrap();
        assert_eq!(i, "IFCISHAPEPROFILEDEF(.AREA.,'W200x15',$,100.,200.,4.3,5.2,$,$,$)");
        let t = perfil_ifc(SectionKind::TuboRedondo, &[48.3, 3.2], "Tubo").unwrap();
        assert!(t.starts_with("IFCCIRCLEHOLLOWPROFILEDEF(.AREA.,'Tubo',$,24.15,3.2)"), "{t}");
        assert!(perfil_ifc(SectionKind::Cantoneira, &[50., 0.], "L").is_none());
    }

    #[test]
    fn guid_tem_22_letras_e_nao_repete() {
        let v: std::collections::HashSet<String> = (1..2000).map(|n| guid(n, 7)).collect();
        assert_eq!(v.len(), 1999);
        assert!(v.iter().all(|g| g.len() == 22 && "0123".contains(&g[..1])));
    }

    #[test]
    fn texto_com_acento_e_aspas() {
        assert_eq!(codificar("Aço d'água"), "A\\X2\\00E7\\X0\\o d''\\X2\\00E1\\X0\\gua");
        assert_eq!(real(3.), "3.");
        assert_eq!(real(-0.), "0.");
    }
}
