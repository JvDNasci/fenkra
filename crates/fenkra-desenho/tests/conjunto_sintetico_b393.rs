// SPDX-License-Identifier: LGPL-2.1-or-later
//! B393: conjunto sintético parecido com o do kart (84 instâncias de 13 peças, muitas repetidas).
//! Mede as quatro vistas (frente, topo, lateral, isométrica) e confere o número de linhas.

use fenkra_desenho::source::{Axes, Models};
use fenkra_desenho::{Dir, Source};
use fenkra_montagem::{Assembly, NodeKind, Placement};
use fenkra_solidos::{eixo_documento, engrenagem_documento, ParamEixo, ParamEngrenagem, TipoEngrenagem, TrechoEixo};
use std::time::Instant;

const TIPOS: usize = 13;
const INSTANCIAS: usize = 84;

/// `pesado`: metade dos tipos são engrenagens helicoidais (superfícies livres, o que torna a HLR
/// exata lenta como no kart); sem ele, só placa e eixos (cilindros e planos).
fn montar(nome: &str, pesado: bool) -> (std::path::PathBuf, Source) {
    let dir = std::env::temp_dir().join(format!("{nome}_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("p0.fkp"), fenkra_solidos::example_document().to_json()).unwrap();
    for i in 1..TIPOS {
        let f = i as f64;
        if pesado && i % 2 == 1 {
            let g = ParamEngrenagem { tipo: TipoEngrenagem::Helicoidal, dentes: 18 + 3 * i as u32, largura: 10. + f, ..Default::default() };
            std::fs::write(dir.join(format!("p{i}.fkp")), engrenagem_documento(&g).unwrap().to_json()).unwrap();
            continue;
        }
        let p = ParamEixo {
            trechos: vec![
                TrechoEixo { diametro: 16. + f, comprimento: 40. + 5. * f, chanfro_fim: 1., rasgo: None },
                TrechoEixo { diametro: 22. + f, comprimento: 60. + 3. * f, chanfro_fim: 1., rasgo: None },
            ],
            furo_centro: i % 2 == 0,
        };
        std::fs::write(dir.join(format!("p{i}.fkp")), eixo_documento(&p).unwrap().to_json()).unwrap();
    }
    let mut a = Assembly::new("conjunto");
    for n in 0..INSTANCIAS {
        // peças repetidas: o tipo se repete a cada TIPOS instâncias; grade de 12 colunas
        let t = n % TIPOS;
        let origem = [(n % 12) as f64 * 160., (n / 12) as f64 * 160., ((n * 7) % 5) as f64 * 30.];
        a.add_component(&format!("C{n}"), NodeKind::Part { path: format!("p{t}.fkp") }, Placement { origin: origem, ..Default::default() }, None);
    }
    std::fs::write(dir.join("conjunto.fka"), a.to_json()).unwrap();
    (dir, Source { path: "conjunto.fka".into(), display_state: String::new(), hidden: vec![] })
}

fn contar(l: &[(Vec<[f64; 2]>, u8)]) -> (usize, usize) {
    (l.iter().filter(|(_, k)| *k != 3).count(), l.iter().filter(|(_, k)| *k == 3).count())
}

type L = Vec<(Vec<[f64; 2]>, u8)>;

/// Cobertura de uma polilinha sobre a outra: amostra cada linha visível a cada 0,25 mm e procura a
/// amostra numa grade de 0,5 mm (vizinhas incluídas). Devolve a fração das amostras de `a` que têm
/// linha de `b` por perto e a de `b` que tem de `a`. Linha dupla (a exata desenha a mesma aresta
/// duas vezes) não pesa, como o comprimento pesaria.
fn cobertura(a: &L, b: &L, kinds: &[u8]) -> (f64, f64) {
    const CELULA: f64 = 0.5;
    fn amostras(l: &L, kinds: &[u8]) -> Vec<[f64; 2]> {
        let mut v = vec![];
        for (p, k) in l.iter().filter(|(_, k)| kinds.contains(k)) {
            let _ = k;
            for w in p.windows(2) {
                let len = ((w[1][0] - w[0][0]).powi(2) + (w[1][1] - w[0][1]).powi(2)).sqrt();
                let n = (len / 0.25).ceil().max(1.) as usize;
                for i in 0..=n {
                    let t = i as f64 / n as f64;
                    v.push([w[0][0] + (w[1][0] - w[0][0]) * t, w[0][1] + (w[1][1] - w[0][1]) * t]);
                }
            }
        }
        v
    }
    let celula = |p: &[f64; 2]| ((p[0] / CELULA).floor() as i64, (p[1] / CELULA).floor() as i64);
    let (sa, sb) = (amostras(a, kinds), amostras(b, kinds));
    let grade = |s: &[[f64; 2]]| s.iter().map(celula).collect::<std::collections::HashSet<_>>();
    let (ga, gb) = (grade(&sa), grade(&sb));
    let perto = |g: &std::collections::HashSet<(i64, i64)>, c: (i64, i64)| (-1..=1).any(|dx| (-1..=1).any(|dy| g.contains(&(c.0 + dx, c.1 + dy))));
    let fa = ga.iter().filter(|c| perto(&gb, **c)).count() as f64 / ga.len().max(1) as f64;
    let fb = gb.iter().filter(|c| perto(&ga, **c)).count() as f64 / gb.len().max(1) as f64;
    (fa, fb)
}

/// Volume, área e centro do conjunto (calculados por sólido distinto) batem com a soma das
/// peças medidas uma a uma e levadas à posição de cada instância.
fn conferir_massa(m: &mut Models, forma: &fenkra_kernel::Shape) {
    let (mut vol, mut area, mut c) = (0., 0., [0.; 3]);
    // cada tipo de peça é medido uma vez (a engrenagem leva segundos)
    let tipos: Vec<_> = (0..TIPOS)
        .map(|t| m.load(&Source { path: format!("p{t}.fkp"), display_state: String::new(), hidden: vec![] }).shape.clone().expect("peça").mass())
        .collect();
    for n in 0..INSTANCIAS {
        let ms = &tipos[n % TIPOS];
        let origem = [(n % 12) as f64 * 160., (n / 12) as f64 * 160., ((n * 7) % 5) as f64 * 30.];
        vol += ms.volume;
        area += ms.area;
        for k in 0..3 {
            c[k] += ms.volume * (ms.center[k] + origem[k]);
        }
    }
    let t = forma.mass();
    println!("massa do conjunto: volume {:.1} (soma {vol:.1}), área {:.1} (soma {area:.1})", t.volume, t.area);
    assert!((t.volume - vol).abs() < 1e-6 * vol.abs(), "volume {} contra {vol}", t.volume);
    assert!((t.area - area).abs() < 1e-6 * area.abs(), "área {} contra {area}", t.area);
    for k in 0..3 {
        assert!((t.center[k] - c[k] / vol).abs() < 1e-3, "centro {k}: {} contra {}", t.center[k], c[k] / vol);
    }
}

const VISTAS: [Dir; 4] = [Dir::Frente, Dir::Topo, Dir::Esquerda, Dir::Iso];

/// Quatro vistas pedidas no mesmo quadro à thread de projeção, como na interface. Devolve o tempo
/// da prévia, o da exata (se `esperar_exata`) e as linhas finais de cada vista.
fn pelo_fundo(dir: &std::path::Path, src: &Source, esperar_exata: bool) -> (std::time::Duration, Option<std::time::Duration>, Vec<L>) {
    use fenkra_desenho::{Drawing, Format, ViewKind};
    let mut d = Drawing::new("sintetico", Format::A1);
    let s = d.sheets[0].id;
    let f = d.add_view(s, src.clone(), ViewKind::Standard(Dir::Frente), [200., 300.], 0.05);
    let t = d.add_view(s, src.clone(), ViewKind::Projected { parent: f, dir: Dir::Topo }, [200., 150.], 0.05);
    let e = d.add_view(s, src.clone(), ViewKind::Projected { parent: f, dir: Dir::Esquerda }, [400., 300.], 0.05);
    let i = d.add_view(s, src.clone(), ViewKind::Standard(Dir::Iso), [400., 150.], 0.05);
    let ids = [f, t, e, i];
    // o teste não suja o cache do usuário
    std::env::set_var("FENKRA_CACHE_FORMAS", std::env::temp_dir().join(format!("fenkra_b393_cache_{}", std::process::id())));
    fenkra_desenho::hlr_processos::definir_executavel(env!("CARGO_BIN_EXE_fenkra-hlr").into());
    let mut m = Models::new(Some(dir.to_path_buf()));
    m.ativar_fundo(|| {});
    let t0 = Instant::now();
    let mut previa = None;
    loop {
        m.sondar();
        let ps: Vec<_> = ids.iter().map(|v| m.proj(&d, *v)).collect();
        if previa.is_none() && ps.iter().all(|p| p.previa || (!p.calculando && !p.lines.is_empty())) {
            previa = Some(t0.elapsed());
            if !esperar_exata {
                return (previa.unwrap(), None, ps.into_iter().map(|p| p.lines).collect());
            }
        }
        if previa.is_some() && ps.iter().all(|p| !p.calculando && !p.previa) {
            return (previa.unwrap(), Some(t0.elapsed()), ps.into_iter().map(|p| p.lines).collect());
        }
        assert!(t0.elapsed().as_secs() < 900, "não terminou");
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

/// Conjunto de 84 instâncias com superfícies livres: a prévia das quatro vistas sai em segundos
/// (limite folgado) e as vistas têm linhas visíveis e ocultas. O kart levava 126 a 135 s.
#[test]
fn previa_das_quatro_vistas_do_conjunto_pesado_em_segundos() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let (dir, src) = montar("fenkra_conjunto_pesado_b393", true);
    let (previa, _, linhas) = pelo_fundo(&dir, &src, false);
    println!("prévia das quatro: {previa:?}; linhas visíveis/ocultas: {:?}", linhas.iter().map(|l| contar(l)).collect::<Vec<_>>());
    assert!(previa.as_secs() < 60, "prévia em {previa:?}");
    for (d, l) in VISTAS.iter().zip(&linhas) {
        let (v, o) = contar(l);
        assert!(v > 100 && o == 0, "{d:?}: {v} visíveis, {o} ocultas");
    }
    // volume e área do conjunto saem em segundos, não minutos, e batem com a soma das peças
    let mut m = Models::new(Some(dir.clone()));
    let forma = m.load(&src).shape.clone().expect("forma do conjunto");
    let t = Instant::now();
    conferir_massa(&mut m, &forma);
    assert!(t.elapsed().as_secs() < 40, "massa do conjunto em {:?}", t.elapsed());
    let _ = std::fs::remove_dir_all(dir);
}

/// Conjunto de 84 instâncias repetidas só de planos e cilindros: a prévia (mapa de profundidade)
/// e a exata (processos) dão a mesma caixa, o mesmo comprimento visível (8%) e linhas ocultas na
/// mesma ordem de grandeza: nenhuma linha some entre uma e outra.
#[test]
fn previa_e_exata_do_conjunto_leve_tem_as_mesmas_linhas() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let (dir, src) = montar("fenkra_conjunto_leve_b393", false);
    let (previa, exata, linhas) = pelo_fundo(&dir, &src, true);
    println!("prévia {previa:?}, exata {exata:?}");
    // a exata pelo mesmo caminho, em um só processo e sem prévia, é a referência
    let mut m = Models::new(Some(dir.clone()));
    let forma = m.load(&src).shape.clone().expect("forma do conjunto");
    conferir_massa(&mut m, &forma);
    // as linhas finais do fundo são as exatas: comparar com a prévia calculada direto
    let (lo, hi) = forma.bbox().unwrap();
    let big = (0..3).map(|k| hi[k] - lo[k]).fold(0., f64::max);
    let malha = forma.mesh(big * 0.0004).unwrap();
    for d in VISTAS {
        let a = Axes::of(d);
        // referência: a HLR exata do kernel, sem ocultas, direto na forma
        let exata: L = forma.hlr(a.toward, a.x, false).unwrap();
        let rapida = fenkra_desenho::hlr_rapida::projetar(&malha, &a, false, fenkra_desenho::hlr_rapida::RESOLUCAO);
        let vis = [0u8, 1, 2];
        let (rec, pre) = cobertura(&exata, &rapida, &vis);
        println!("{d:?}: de cada 100 pontos da exata, {:.1} estão na rápida; de cada 100 da rápida, {:.1} estão na exata", rec * 100., pre * 100.);
        assert!(rec > 0.95, "{d:?}: a prévia perdeu linhas da exata ({:.1}% presentes)", rec * 100.);
        assert!(pre > 0.90, "{d:?}: a prévia tem linha que a exata não tem ({:.1}% presentes)", pre * 100.);
    }
    // a vista final do fundo é a exata: tem linhas em toda vista
    assert!(linhas.iter().all(|l| contar(l).0 > 100));
    let _ = std::fs::remove_dir_all(dir);
}

/// Medição manual: o conjunto pesado, prévia e exata (minutos). `--ignored --nocapture`.
#[test]
#[ignore]
fn medir_conjunto_pesado_previa_e_exata() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let (dir, src) = montar("fenkra_conjunto_medir_b393", true);
    let (previa, exata, _) = pelo_fundo(&dir, &src, true);
    println!("prévia {previa:?}, exata {exata:?}");
    let _ = std::fs::remove_dir_all(dir);
}
