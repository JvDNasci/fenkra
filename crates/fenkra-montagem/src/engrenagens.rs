//! Pares de engrenagens (B28): gera as duas peças da biblioteca de engrenagens
//! (`fenkra_solidos::engrenagens`), insere na montagem engrenadas e cria a restrição de movimento.
//!
//! O tipo do `ParamEngrenagem` recebido dá o **tipo do par**:
//! - `Reta`: pinhão reto z1 + coroa reta z2, eixos paralelos, centros a `(d1 + d2)/2`;
//! - `Helicoidal`: idem, com a coroa de hélice contrária (mesmo módulo e ângulo);
//! - `Interna`: pinhão reto z1 dentro de uma coroa interna z2 > z1, centros a `(d2 − d1)/2`;
//! - `Cremalheira`: pinhão reto z1 sobre uma cremalheira, linha primitiva tangente ao primitivo.
//!
//! O pinhão tem o eixo Z do componente na origem do mundo, com o dente 0 virado para a coroa; a coroa
//! entra girada de meio passo para os dentes encaixarem. A folga (backlash) dada vale para o par: cada
//! peça tira metade dela da espessura do dente. As peças ficam em `<pasta>/engrenagens/<chave>.fkp`
//! (reaproveitadas se já existirem) e entram no grupo "Engrenagens".

use crate::*;
use fenkra_solidos::engrenagens::{calculo, engrenagem_documento, CalculoEngrenagem, ParamEngrenagem, TipoEngrenagem};
use std::f64::consts::PI;
use std::path::Path;

/// Nome do grupo da árvore que recebe as engrenagens.
const GRUPO: &str = "Engrenagens";
/// Diferença mínima de dentes entre a coroa interna e o pinhão antes de avisar de interferência.
const DIFERENCA_INTERNA: u32 = 10;

/// Cálculo do par, sem tocar na montagem (a janela mostra isto ao vivo).
#[derive(Clone, Debug, PartialEq)]
pub struct CalculoPar {
    pub pinhao: ParamEngrenagem,
    pub coroa: ParamEngrenagem,
    pub calc_pinhao: CalculoEngrenagem,
    pub calc_coroa: CalculoEngrenagem,
    /// distância entre os centros (cremalheira: altura do centro do pinhão sobre a linha primitiva)
    pub distancia_centros: f64,
    /// giro da coroa por giro do pinhão (negativo = sentido contrário); 0 na cremalheira
    pub razao: f64,
    /// avanço da cremalheira por volta do pinhão (mm); 0 nos demais
    pub mm_por_volta: f64,
    pub avisos: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RelatorioEngrenagens {
    pub pinhao: Id,
    pub coroa: Id,
    pub restricao: Id,
    pub distancia_centros: f64,
    pub razao: f64,
    pub avisos: Vec<String>,
}

/// Calcula o par: parâmetros das duas peças, distância entre centros e razão. `dentes_coroa` é
/// ignorado na cremalheira (quem manda é o comprimento).
pub fn calculo_par(pinhao: &ParamEngrenagem, dentes_coroa: u32) -> Result<CalculoPar, String> {
    let meia_folga = pinhao.folga_dentes / 2.;
    let p1 = ParamEngrenagem { tipo: if pinhao.tipo == TipoEngrenagem::Helicoidal { TipoEngrenagem::Helicoidal } else { TipoEngrenagem::Reta }, folga_dentes: meia_folga, ..pinhao.clone() };
    let mut avisos = vec![];
    let p2 = match pinhao.tipo {
        TipoEngrenagem::Reta => ParamEngrenagem { dentes: dentes_coroa, coef_correcao: 0., ..p1.clone() },
        TipoEngrenagem::Helicoidal => ParamEngrenagem { dentes: dentes_coroa, coef_correcao: 0., angulo_helice: -p1.angulo_helice, ..p1.clone() },
        TipoEngrenagem::Interna => {
            if dentes_coroa <= pinhao.dentes {
                return Err("a coroa interna precisa ter mais dentes que o pinhão".into());
            }
            if dentes_coroa - pinhao.dentes < DIFERENCA_INTERNA {
                avisos.push(format!("coroa interna com só {} dentes a mais que o pinhão: risco de interferência (use ao menos {DIFERENCA_INTERNA})", dentes_coroa - pinhao.dentes));
            }
            ParamEngrenagem { tipo: TipoEngrenagem::Interna, dentes: dentes_coroa, coef_correcao: 0., furo: 0., ..p1.clone() }
        }
        TipoEngrenagem::Cremalheira => ParamEngrenagem { tipo: TipoEngrenagem::Cremalheira, coef_correcao: 0., furo: 0., ..p1.clone() },
    };
    if pinhao.coef_correcao != 0. {
        avisos.push("com coeficiente de correção no pinhão a distância entre centros calculada não é a de trabalho".into());
    }
    let calc_pinhao = calculo(&p1).map_err(|e| format!("pinhão: {e}"))?;
    let calc_coroa = calculo(&p2).map_err(|e| format!("{}: {e}", if p2.tipo == TipoEngrenagem::Cremalheira { "cremalheira" } else { "coroa" }))?;
    let (d1, d2) = (calc_pinhao.diametro_primitivo, calc_coroa.diametro_primitivo);
    let (distancia_centros, razao, mm_por_volta) = match pinhao.tipo {
        TipoEngrenagem::Reta | TipoEngrenagem::Helicoidal => ((d1 + d2) / 2., -(p1.dentes as f64) / dentes_coroa as f64, 0.),
        TipoEngrenagem::Interna => ((d2 - d1) / 2., p1.dentes as f64 / dentes_coroa as f64, 0.),
        TipoEngrenagem::Cremalheira => (d1 / 2., 0., PI * d1),
    };
    avisos.extend(calc_pinhao.avisos.iter().map(|a| format!("pinhão: {a}")));
    avisos.extend(calc_coroa.avisos.iter().filter(|a| p2.tipo != TipoEngrenagem::Cremalheira || !a.contains("correção")).map(|a| format!("{}: {a}", if p2.tipo == TipoEngrenagem::Cremalheira { "cremalheira" } else { "coroa" })));
    Ok(CalculoPar { pinhao: p1, coroa: p2, calc_pinhao, calc_coroa, distancia_centros, razao, mm_por_volta, avisos })
}

/// Insere o par na montagem (pinhão na origem, coroa ao lado) e cria a restrição de movimento.
/// `dir` é a pasta da montagem, onde nasce `engrenagens/`.
pub fn inserir_par_engrenagens(asm: &mut Assembly, dir: Option<&Path>, pinhao: &ParamEngrenagem, dentes_coroa: u32) -> Result<RelatorioEngrenagens, String> {
    let dir = dir.ok_or("salve a montagem antes: as peças das engrenagens são gravadas na pasta dela")?;
    let par = calculo_par(pinhao, dentes_coroa)?;
    let (arq1, nome1) = gravar_peca(dir, &par.pinhao)?;
    let (arq2, nome2) = gravar_peca(dir, &par.coroa)?;
    let pasta = match asm.nodes.iter().find(|n| matches!(n.kind, NodeKind::Group) && n.name == GRUPO).map(|n| n.id) {
        Some(g) => g,
        None => asm.add_group(GRUPO, None),
    };

    let a = par.distancia_centros;
    let (pl1, pl2) = match pinhao.tipo {
        TipoEngrenagem::Reta | TipoEngrenagem::Helicoidal => {
            // dente 0 do pinhão (+X) aponta para a coroa; ali a coroa precisa de um vão (dentes em
            // ρ + 2πk/z2, vão centrado em π): ρ = π(1 + 1/z2)
            let rho = PI * (1. + 1. / dentes_coroa as f64);
            (Placement::default(), girado([a, 0., 0.], rho))
        }
        // o vão 0 da coroa interna está em +X; o centro dela fica atrás do pinhão
        TipoEngrenagem::Interna => (Placement::default(), girado([-a, 0., 0.], 0.)),
        TipoEngrenagem::Cremalheira => {
            // o pinhão fica sobre um vão central da cremalheira, com o dente 0 virado para ela (-Y)
            let passo = PI * par.coroa.modulo;
            let n = (par.coroa.comprimento_cremalheira / passo).floor();
            let c0 = (par.coroa.comprimento_cremalheira - (n - 1.) * passo) / 2.;
            let x = c0 + ((n / 2.).floor() - 0.5) * passo;
            (girado([x, a, 0.], -PI / 2.), Placement::default())
        }
    };
    let id1 = asm.add_component(&nome1, NodeKind::Part { path: arq1 }, pl1, Some(pasta));
    let id2 = asm.add_component(&nome2, NodeKind::Part { path: arq2 }, pl2, Some(pasta));

    let ref_a = |comp: Id, geom: Geom| MateRef { comp, geom, pick: None };
    let eixo = Geom::Axis { p: [0.; 3], d: [0., 0., 1.] };
    let (restricao, razao) = if pinhao.tipo == TipoEngrenagem::Cremalheira {
        // centro do pinhão a `a` da linha primitiva (plano y = 0 da cremalheira), eixos paralelos
        let linha = Geom::Plane { p: [0.; 3], n: [0., 1., 0.] };
        asm.add_mate(MateKind::Distance(num(a)), ref_a(id1, Geom::Point([0.; 3])), ref_a(id2, linha));
        asm.add_mate(MateKind::Parallel, ref_a(id1, eixo), ref_a(id2, eixo));
        let id = asm.add_mate(MateKind::RackPinion { mm_per_rev: num(par.mm_por_volta), refs: [[0.; 3]; 4] }, ref_a(id1, eixo), ref_a(id2, Geom::Axis { p: [0.; 3], d: [1., 0., 0.] }));
        (id, 0.)
    } else {
        asm.add_mate(MateKind::Distance(num(a)), ref_a(id1, eixo), ref_a(id2, eixo));
        let id = asm.add_mate(MateKind::Gear { ratio: num(par.razao), belt: false, refs: [[0.; 3]; 4] }, ref_a(id1, eixo), ref_a(id2, eixo));
        (id, par.razao)
    };
    asm.init_motion(restricao);
    Ok(RelatorioEngrenagens { pinhao: id1, coroa: id2, restricao, distancia_centros: a, razao, avisos: par.avisos })
}

/// Componente com a origem em `o` e girado de `rho` radianos em torno de Z.
fn girado(o: V3, rho: f64) -> Placement {
    Placement::default().moved(o, [0., 0., rho], [0.; 3])
}

/// Grava a peça (se ainda não existe) e devolve (caminho relativo à montagem, nome).
fn gravar_peca(dir: &Path, p: &ParamEngrenagem) -> Result<(String, String), String> {
    let doc = engrenagem_documento(p)?;
    let base = p.chave();
    let destino = dir.join("engrenagens").join(format!("{base}.fkp"));
    if !destino.exists() {
        std::fs::create_dir_all(dir.join("engrenagens")).map_err(|e| format!("{}: {e}", dir.join("engrenagens").display()))?;
        std::fs::write(&destino, doc.to_json()).map_err(|e| format!("{}: {e}", destino.display()))?;
    }
    Ok((format!("engrenagens/{base}.fkp"), doc.props.description.clone()))
}

/// Número como expressão: 0.5 → "0.5", -0.5 → "-0.5".
fn num(x: f64) -> String {
    format!("{x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pasta(nome: &str) -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!("fenkra_engrenagens_{nome}"));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn dist(a: &Assembly, i: Id, j: Id) -> f64 {
        norm(sub(a.node(i).unwrap().placement.origin, a.node(j).unwrap().placement.origin))
    }

    #[test]
    fn par_externo_m2_z20_z40() {
        let dir = pasta("externo");
        let mut a = Assembly::new("Redutor");
        let r = inserir_par_engrenagens(&mut a, Some(&dir), &ParamEngrenagem::default(), 40).unwrap();
        assert!((r.distancia_centros - 60.).abs() < 1e-9 && (r.razao + 0.5).abs() < 1e-12);
        assert!((dist(&a, r.pinhao, r.coroa) - 60.).abs() < 1e-9);
        let m = a.mates.iter().find(|m| m.id == r.restricao).unwrap();
        assert!(matches!(&m.kind, MateKind::Gear { ratio, belt: false, refs } if ratio == "-0.5" && refs[0] != [0.; 3]), "{:?}", m.kind);
        assert!(matches!(&a.mates.iter().find(|m| matches!(m.kind, MateKind::Distance(_))).unwrap().kind, MateKind::Distance(d) if d == "60"));
        for f in ["engrenagem-reta-m2-a20-b10-f0-z20-x0-d0.fkp", "engrenagem-reta-m2-a20-b10-f0-z40-x0-d0.fkp"] {
            assert!(dir.join("engrenagens").join(f).exists(), "{f}");
        }
        // o pinhão entra fixo (primeiro componente); o conjunto resolve sem mexer em nada
        assert!(a.node(r.pinhao).unwrap().fixed);
        assert!(solve::solve(&mut a).converged);
        assert!((dist(&a, r.pinhao, r.coroa) - 60.).abs() < 1e-3);
        // meio passo: coroa girada de π(1 + 1/z2) = 184,5° (vão voltado para o dente 0 do pinhão)
        let x = a.node(r.coroa).unwrap().placement.dir([1., 0., 0.]);
        let graus = (x[1].atan2(x[0]).to_degrees() + 360.) % 360.;
        assert!((graus - 184.5).abs() < 1e-6, "{graus}");
        // de novo: reaproveita os arquivos
        let n = a.nodes.len();
        inserir_par_engrenagens(&mut a, Some(&dir), &ParamEngrenagem::default(), 40).unwrap();
        assert_eq!(a.nodes.len(), n + 2);
        assert_eq!(a.nodes.iter().filter(|n| matches!(n.kind, NodeKind::Group)).count(), 1);
    }

    #[test]
    fn par_interno_helicoidal_e_cremalheira() {
        let dir = pasta("outros");
        // interna: centros a m(z2 − z1)/2, razão positiva
        let mut a = Assembly::new("Planetário");
        let p = ParamEngrenagem { tipo: TipoEngrenagem::Interna, dentes: 20, ..Default::default() };
        let r = inserir_par_engrenagens(&mut a, Some(&dir), &p, 60).unwrap();
        assert!((dist(&a, r.pinhao, r.coroa) - 40.).abs() < 1e-9 && (r.razao - 1. / 3.).abs() < 1e-12);
        assert!(solve::solve(&mut a).converged);
        assert!(inserir_par_engrenagens(&mut Assembly::new("x"), Some(&dir), &p, 20).is_err());
        assert!(!calculo_par(&p, 25).unwrap().avisos.is_empty(), "poucos dentes a mais");

        // helicoidal: a coroa tem a hélice contrária; centros pelo módulo transversal
        let h = ParamEngrenagem { tipo: TipoEngrenagem::Helicoidal, angulo_helice: 15., ..Default::default() };
        let c = calculo_par(&h, 40).unwrap();
        assert!((c.coroa.angulo_helice + 15.).abs() < 1e-12 && (c.distancia_centros - 60. / 15_f64.to_radians().cos()).abs() < 1e-9);

        // cremalheira: pinhão a r acima da linha primitiva, avanço π·d por volta
        let mut a = Assembly::new("Pinhão e cremalheira");
        let k = ParamEngrenagem { tipo: TipoEngrenagem::Cremalheira, ..Default::default() };
        let r = inserir_par_engrenagens(&mut a, Some(&dir), &k, 0).unwrap();
        assert!((r.distancia_centros - 20.).abs() < 1e-9);
        let pin = a.node(r.pinhao).unwrap().placement.origin;
        assert!((pin[1] - 20.).abs() < 1e-9 && pin[0] > 40. && pin[0] < 60., "{pin:?}");
        assert!(matches!(&a.mates.iter().find(|m| m.id == r.restricao).unwrap().kind, MateKind::RackPinion { mm_per_rev, .. } if (mm_per_rev.parse::<f64>().unwrap() - 40. * PI).abs() < 1e-9));
        assert!(solve::solve(&mut a).converged);
    }


    #[test]
    fn dentes_do_par_nao_se_interpenetram() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let dir = pasta("interferencia");
        let casos = [
            (ParamEngrenagem::default(), 40),
            (ParamEngrenagem { tipo: TipoEngrenagem::Helicoidal, angulo_helice: 15., ..Default::default() }, 40),
            (ParamEngrenagem { tipo: TipoEngrenagem::Interna, dentes: 20, ..Default::default() }, 60),
            (ParamEngrenagem { tipo: TipoEngrenagem::Cremalheira, ..Default::default() }, 0),
        ];
        for (p, z2) in casos {
            let mut a = Assembly::new("par");
            inserir_par_engrenagens(&mut a, Some(&dir), &p, z2).unwrap();
            let mut cache = crate::load::PartCache::default();
            let i = crate::view::interferences(&a, Some(&dir), &mut cache, 0.5);
            assert!(i.is_empty(), "{:?}: {i:?}", p.tipo);
        }
        // controle: com a coroa 2 mm mais perto, os dentes passam a se tocar
        let mut a = Assembly::new("par");
        let r = inserir_par_engrenagens(&mut a, Some(&dir), &ParamEngrenagem::default(), 40).unwrap();
        a.nodes.iter_mut().find(|n| n.id == r.coroa).unwrap().placement.origin[0] = 58.;
        let i = crate::view::interferences(&a, Some(&dir), &mut crate::load::PartCache::default(), 0.5);
        assert!(!i.is_empty(), "o controle deveria interferir");
    }
    #[test]
    fn entradas_invalidas_e_sem_pasta() {
        let mut a = Assembly::new("x");
        assert!(inserir_par_engrenagens(&mut a, None, &ParamEngrenagem::default(), 40).is_err());
        let dir = pasta("invalido");
        assert!(inserir_par_engrenagens(&mut a, Some(&dir), &ParamEngrenagem::default(), 3).is_err());
        assert!(a.nodes.is_empty(), "nada inserido quando falha");
    }
}
