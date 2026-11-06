// SPDX-License-Identifier: LGPL-2.1-or-later
//! PMI semântico do documento para o STEP AP242 (B191): cotas, tolerâncias geométricas, datums e alvos
//! de referência ligados às faces da forma gravada.
//!
//! O que o gravador do OpenCASCADE 8.0.1 não leva fica fora e volta em `fora`, uma nota por item, para
//! a interface dizer à pessoa o que não atravessou. O mapa do que atravessa e do que não atravessa
//! está em `docs/modulos/pmi-3d.md`.

use crate::Nota;
use fenkra_core::gdt::{separar_datums, valor_num, Modif, Zona};
use fenkra_core::pmi::{GdtChar, Pmi, PmiKind, PmiTarget, TargetClass};
use fenkra_core::pmi_alvos::FormaAlvo;
use fenkra_core::Document;
use fenkra_kernel::{AlvoDatumXde, AlvoXde, AnotacaoXde, DadoXde, FaceInfo, MaterialXde, ModToleranciaXde, ModZonaXde, Shape, TipoDimensaoXde, TipoToleranciaXde, ZonaXde};
use std::collections::HashMap;

pub const PMI_GRAFICO: &str = "PMI gráfico ({0}): rugosidade, solda, notas, balões e legenda ainda não vão no STEP; só cotas, tolerâncias, referências e alvos.";
pub const PMI_SEM_FACE: &str = "A anotação {0} não está ligada a uma face e ficou fora do STEP.";
pub const PMI_TIPO: &str = "A anotação {0} ({1}) não tem tipo equivalente no STEP gravado e ficou fora.";
pub const PMI_MODIF: &str = "Quadro {0}: o modificador {1} não vai no STEP.";
pub const PMI_ZONA: &str = "Quadro {0}: a zona {1} não atravessa o STEP; o gravador escreve só a forma cilíndrica ou esférica.";
pub const PMI_COMPOSTO: &str = "Quadro {0}: a segunda linha do quadro composto não vai no STEP.";
pub const PMI_DATUM_ORDEM: &str = "A referência {0} aparece em ordens diferentes nos quadros: o STEP guarda uma ordem só por referência.";
pub const PMI_DATUM_SEM: &str = "O quadro {0} cita a referência {1}, que não existe no modelo.";
pub const PMI_ALVO: &str = "Alvo {0}: o tipo {1} não existe no STEP gravado.";
pub const PMI_VALOR: &str = "A anotação {0} não tem um valor numérico e ficou fora do STEP.";

/// Todos os modelos de frase deste módulo, para a tradução da interface ser conferida.
pub const NOTAS_PMI: &[&str] = &[PMI_GRAFICO, PMI_SEM_FACE, PMI_TIPO, PMI_MODIF, PMI_ZONA, PMI_COMPOSTO, PMI_DATUM_ORDEM, PMI_DATUM_SEM, PMI_ALVO, PMI_VALOR];

/// O que foi para o modelo de troca e o que ficou de fora.
#[derive(Clone, Debug, Default)]
pub struct SaidaPmi {
    pub anotacoes: Vec<AnotacaoXde>,
    pub fora: Vec<Nota>,
}

fn nota(modelo: &'static str, args: &[&str]) -> Nota {
    Nota { modelo: modelo.into(), args: args.iter().map(|s| s.to_string()).collect() }
}

fn nome_de(p: &Pmi) -> String {
    if p.name.is_empty() {
        format!("#{}", p.id.0)
    } else {
        p.name.clone()
    }
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

fn norma(a: [f64; 3]) -> f64 {
    dot(a, a).sqrt()
}

/// A face da forma que a chave geométrica do alvo descreve: a de mesmo tipo e orientação com a menor
/// distância. A chave é a mesma que o PMI usa para reencontrar o elemento depois de o modelo mudar.
pub fn face_do_alvo(t: &PmiTarget, faces: &[FaceInfo]) -> Option<u32> {
    let mut melhor: Option<(f64, usize)> = None;
    for (i, f) in faces.iter().enumerate() {
        let pontos = match t.class {
            TargetClass::FacePlana if f.kind == 0 => {
                let c = dot(f.dir, t.dir);
                (c > 0.82).then(|| dot(sub(f.point, t.point), t.dir).abs() + 0.05 * norma(sub(f.point, t.point)) + (1. - c) * 50.)
            }
            // cilindro ou aresta circular: o ponto da face está à distância do raio do eixo do alvo
            TargetClass::FaceCilindrica | TargetClass::ArestaCircular if f.kind == 1 && norma(t.dir) > 0.5 => {
                let eixo = [t.dir[0] / norma(t.dir), t.dir[1] / norma(t.dir), t.dir[2] / norma(t.dir)];
                let c = dot(f.dir, eixo).abs();
                let radial = norma(cross(sub(f.point, t.point), eixo));
                (c > 0.9).then(|| (radial - t.radius).abs() + (1. - c) * 50.)
            }
            TargetClass::FaceOutra if f.kind >= 2 => Some(norma(sub(f.point, t.point))),
            _ => None,
        };
        if let Some(p) = pontos {
            if melhor.is_none_or(|(m, _)| p < m) {
                melhor = Some((p, i));
            }
        }
    }
    // o ponto pode estar a alguns milímetros da âncora; o que não pode é ser outro elemento
    melhor.filter(|(p, _)| *p < 5.).map(|(_, i)| i as u32)
}

fn modif_xde(m: Modif) -> Result<ModToleranciaXde, Modif> {
    Ok(match m {
        Modif::EstadoLivre => ModToleranciaXde::EstadoLivre,
        Modif::PlanoTangente => ModToleranciaXde::PlanoTangente,
        Modif::Estatistica => ModToleranciaXde::Estatistica,
        Modif::ZonaComum => ModToleranciaXde::ZonaComum,
        Modif::SecaoQualquer => ModToleranciaXde::SecaoQualquer,
        Modif::ElementoLinha => ModToleranciaXde::ElementoLinha,
        Modif::NaoConvexa => ModToleranciaXde::NaoConvexa,
        Modif::TodoContorno => ModToleranciaXde::TodoContorno,
        Modif::TodaSuperficie => ModToleranciaXde::TodaSuperficie,
        outro => return Err(outro),
    })
}

fn tipo_tolerancia(c: GdtChar) -> TipoToleranciaXde {
    match c {
        GdtChar::Retitude => TipoToleranciaXde::Retitude,
        GdtChar::Planeza => TipoToleranciaXde::Planeza,
        GdtChar::Circularidade => TipoToleranciaXde::Circularidade,
        GdtChar::Cilindricidade => TipoToleranciaXde::Cilindricidade,
        GdtChar::Paralelismo => TipoToleranciaXde::Paralelismo,
        GdtChar::Perpendicularidade => TipoToleranciaXde::Perpendicularidade,
        GdtChar::Angularidade => TipoToleranciaXde::Angularidade,
        GdtChar::Posicao => TipoToleranciaXde::Posicao,
        GdtChar::Concentricidade => TipoToleranciaXde::Coaxialidade,
        GdtChar::Simetria => TipoToleranciaXde::Simetria,
        GdtChar::PerfilLinha => TipoToleranciaXde::PerfilLinha,
        GdtChar::PerfilSuperficie => TipoToleranciaXde::PerfilSuperficie,
        GdtChar::BatimentoCircular => TipoToleranciaXde::BatimentoCircular,
        GdtChar::BatimentoTotal => TipoToleranciaXde::BatimentoTotal,
    }
}

fn alvo_xde(forma: FormaAlvo) -> Option<AlvoDatumXde> {
    match forma {
        FormaAlvo::Ponto => Some(AlvoDatumXde::Ponto),
        FormaAlvo::Linha => Some(AlvoDatumXde::Linha),
        FormaAlvo::Circulo => Some(AlvoDatumXde::Circulo),
        FormaAlvo::Retangulo => Some(AlvoDatumXde::Retangulo),
        FormaAlvo::Area => Some(AlvoDatumXde::Area),
        FormaAlvo::CirculoCurva | FormaAlvo::Curva => None,
    }
}

/// Desvios de uma cota: (inferior, superior) com sinal. Vazio nos dois = sem tolerância.
fn desvios(p: &Pmi) -> Option<(f64, f64)> {
    let (mais, menos) = (valor_num(&p.tol_plus), valor_num(&p.tol_minus));
    if mais.is_none() && menos.is_none() {
        return None;
    }
    Some((-menos.unwrap_or(0.).abs(), mais.unwrap_or(0.).abs()))
}

/// Converte o PMI do documento nas anotações semânticas do STEP, ligadas às faces de `forma`.
pub fn anotacoes_do_documento(doc: &Document, forma: &Shape) -> SaidaPmi {
    let mut s = SaidaPmi::default();
    let Ok(topo) = forma.topology() else { return s };
    let face = |p: &Pmi, i: usize| p.targets.get(i).copied().flatten().and_then(|t| face_do_alvo(&t, &topo.faces));

    // precedência de cada referência: a ordem em que aparece no primeiro quadro que a cita
    let mut posicao: HashMap<String, u32> = HashMap::new();
    let mut ordem_avisada: Vec<String> = vec![];
    for p in &doc.pmi {
        if let PmiKind::Gdt { datums, .. } = &p.kind {
            for (i, l) in separar_datums(datums).into_iter().enumerate() {
                let n = i as u32 + 1;
                match posicao.get(&l) {
                    None => {
                        posicao.insert(l, n);
                    }
                    Some(&antes) if antes != n && !ordem_avisada.contains(&l) => {
                        s.fora.push(nota(PMI_DATUM_ORDEM, &[&l]));
                        ordem_avisada.push(l);
                    }
                    _ => {}
                }
            }
        }
    }

    // datums primeiro: as tolerâncias os citam pelo índice
    let mut indice: HashMap<String, usize> = HashMap::new();
    let mut graficos = 0usize;
    for p in &doc.pmi {
        match &p.kind {
            PmiKind::Datum { letter } => {
                let Some(f) = face(p, 0) else {
                    s.fora.push(nota(PMI_SEM_FACE, &[&nome_de(p)]));
                    continue;
                };
                indice.insert(letter.clone(), s.anotacoes.len());
                s.anotacoes.push(AnotacaoXde { produto: 0, nome: letter.clone(), faces: vec![f], dado: DadoXde::Datum { letra: letter.clone(), posicao: posicao.get(letter).copied().unwrap_or(0), alvo: None } });
            }
            _ => {}
        }
    }
    for p in &doc.pmi {
        if let PmiKind::DatumTarget { alvo } = &p.kind {
            let Some(tipo) = alvo_xde(alvo.forma) else {
                s.fora.push(nota(PMI_ALVO, &[&alvo.identificador(), alvo.forma.label()]));
                continue;
            };
            let Some(f) = face(p, 0) else {
                s.fora.push(nota(PMI_SEM_FACE, &[&nome_de(p)]));
                continue;
            };
            let a = AlvoXde { tipo, numero: alvo.numero, comprimento: alvo.dim[0], largura: alvo.dim[1] };
            s.anotacoes.push(AnotacaoXde { produto: 0, nome: alvo.letra.clone(), faces: vec![f], dado: DadoXde::Datum { letra: alvo.letra.clone(), posicao: posicao.get(&alvo.letra).copied().unwrap_or(0), alvo: Some(a) } });
        }
    }

    for p in &doc.pmi {
        let nome = nome_de(p);
        match &p.kind {
            PmiKind::Linear => {
                let (Some(a), Some(b)) = (face(p, 0), face(p, 1)) else {
                    s.fora.push(nota(PMI_SEM_FACE, &[&nome]));
                    continue;
                };
                let alvos: Vec<Option<PmiTarget>> = p.targets.clone();
                let Some(v) = p.measure_targets(&alvos).or_else(|| p.measure(&p.anchors)) else {
                    s.fora.push(nota(PMI_VALOR, &[&nome]));
                    continue;
                };
                s.anotacoes.push(AnotacaoXde { produto: 0, nome, faces: vec![a], dado: DadoXde::Dimensao { tipo: TipoDimensaoXde::LocalDistanciaLinear, valor: v, tolerancia: desvios(p), faces2: vec![b] } });
            }
            PmiKind::Radius | PmiKind::Diameter => {
                let Some(f) = face(p, 0) else {
                    s.fora.push(nota(PMI_SEM_FACE, &[&nome]));
                    continue;
                };
                let alvos: Vec<Option<PmiTarget>> = p.targets.clone();
                let Some(v) = p.measure_targets(&alvos).or_else(|| p.measure(&p.anchors)) else {
                    s.fora.push(nota(PMI_VALOR, &[&nome]));
                    continue;
                };
                let tipo = if p.kind == PmiKind::Radius { TipoDimensaoXde::TamanhoRaio } else { TipoDimensaoXde::TamanhoDiametro };
                s.anotacoes.push(AnotacaoXde { produto: 0, nome, faces: vec![f], dado: DadoXde::Dimensao { tipo, valor: v, tolerancia: desvios(p), faces2: vec![] } });
            }
            PmiKind::Angle | PmiKind::Ordinate { .. } | PmiKind::Gap { .. } | PmiKind::ArcLength => {
                s.fora.push(nota(PMI_TIPO, &[&nome, p.kind.label()]));
            }
            PmiKind::Gdt { ch, tol, datums, extra } => {
                let Some(f) = face(p, 0) else {
                    s.fora.push(nota(PMI_SEM_FACE, &[&nome]));
                    continue;
                };
                let Some(valor) = valor_num(tol) else {
                    s.fora.push(nota(PMI_VALOR, &[&nome]));
                    continue;
                };
                let r = extra.resolvido();
                let (mut material, mut mod_zona, mut modificadores) = (MaterialXde::Nenhum, ModZonaXde::Nenhum, vec![]);
                for m in &r.mods {
                    match m {
                        Modif::Mmc => material = MaterialXde::Maximo,
                        Modif::Lmc => material = MaterialXde::Minimo,
                        Modif::Projetada => {
                            mod_zona = ModZonaXde::Projetada;
                            s.fora.push(nota(PMI_ZONA, &[&nome, "projetada"]));
                        }
                        outro => match modif_xde(*outro) {
                            // o gravador conhece o nome, mas não escreve estes dois
                            Ok(x @ (ModToleranciaXde::TodoContorno | ModToleranciaXde::TodaSuperficie)) => {
                                modificadores.push(x);
                                s.fora.push(nota(PMI_MODIF, &[&nome, outro.label()]));
                            }
                            Ok(x) => modificadores.push(x),
                            Err(m) => s.fora.push(nota(PMI_MODIF, &[&nome, m.label()])),
                        },
                    }
                }
                let zona = match r.zona {
                    Zona::Largura => ZonaXde::Largura,
                    Zona::Diametro => ZonaXde::Diametro,
                    Zona::Esfera => ZonaXde::Esfera,
                    Zona::NaoUniforme => {
                        mod_zona = ModZonaXde::NaoUniforme;
                        s.fora.push(nota(PMI_ZONA, &[&nome, "não uniforme"]));
                        ZonaXde::Largura
                    }
                };
                if extra.second.is_some() {
                    s.fora.push(nota(PMI_COMPOSTO, &[&nome]));
                }
                let mut refs = vec![];
                for l in separar_datums(datums) {
                    match indice.get(&l) {
                        Some(i) => refs.push(*i),
                        None => s.fora.push(nota(PMI_DATUM_SEM, &[&nome, &l])),
                    }
                }
                let valor_zona = valor_num(&extra.proj).unwrap_or(0.);
                s.anotacoes.push(AnotacaoXde {
                    produto: 0,
                    nome,
                    faces: vec![f],
                    dado: DadoXde::Tolerancia { tipo: tipo_tolerancia(*ch), valor, zona, material, mod_zona, valor_zona, modificadores, datums: refs },
                });
            }
            PmiKind::Datum { .. } | PmiKind::DatumTarget { .. } => {}
            _ => graficos += 1,
        }
    }
    if graficos > 0 {
        s.fora.push(nota(PMI_GRAFICO, &[&graficos.to_string()]));
    }
    s
}

#[cfg(test)]
mod testes {
    use super::*;

    fn face(kind: u8, dir: [f64; 3], point: [f64; 3]) -> FaceInfo {
        FaceInfo { kind, dir, area: 1., point }
    }

    #[test]
    fn alvo_plano_acha_a_face_paralela_mais_proxima() {
        let faces = [face(0, [0., 0., 1.], [0., 0., 10.]), face(0, [0., 0., -1.], [0., 0., 0.]), face(0, [1., 0., 0.], [5., 0., 5.])];
        let t = PmiTarget { class: TargetClass::FacePlana, point: [1., 1., 0.], dir: [0., 0., -1.], radius: 0., concave: false };
        assert_eq!(face_do_alvo(&t, &faces), Some(1));
        let t2 = PmiTarget { point: [1., 1., 10.], dir: [0., 0., 1.], ..t };
        assert_eq!(face_do_alvo(&t2, &faces), Some(0));
    }

    #[test]
    fn alvo_cilindrico_acha_o_furo_pelo_raio() {
        let faces = [face(1, [0., 0., 1.], [4., 0., 5.]), face(1, [0., 0., 1.], [9., 0., 5.])];
        let t = PmiTarget { class: TargetClass::FaceCilindrica, point: [0., 0., 5.], dir: [0., 0., 1.], radius: 9., concave: true };
        assert_eq!(face_do_alvo(&t, &faces), Some(1));
    }

    #[test]
    fn alvo_sem_correspondente_nao_liga() {
        let faces = [face(0, [1., 0., 0.], [0., 0., 0.])];
        let t = PmiTarget { class: TargetClass::FaceCilindrica, point: [0., 0., 0.], dir: [0., 0., 1.], radius: 3., concave: true };
        assert_eq!(face_do_alvo(&t, &faces), None);
    }
}
