// SPDX-License-Identifier: LGPL-2.1-or-later
//! B426: a conversão da forma livre em B-rep leva de 3 a 12 s e era refeita a cada cálculo do zero
//! (abrir a peça, reconstruir, exportar, a montagem e o desenho lendo a forma final). Aqui o resultado
//! fica guardado pela impressão digital da entrada: a gaiola já resolvida, a continuidade, a
//! tolerância e a versão do conversor. Mesma entrada, mesma saída, sem refazer.
//!
//! Dois níveis:
//! - **memória**: as últimas conversões do processo, em texto B-rep (a forma do OpenCASCADE não
//!   atravessa threads, o texto sim);
//! - **disco**: `%LOCALAPPDATA%\Fenkra\cache_forma`, para a peça reaberta não converter de novo. Só
//!   as conversões mais recentes ficam; arquivo ilegível é ignorado e a conversão roda.
//!
//! O progresso da conversão em andamento fica em [`andamento`], para a interface mostrar sobre a peça.

use crate::forma_brep::{self, RegiaoRelatorio, RelatorioForma};
use fenkra_core::subd::Gaiola;
use fenkra_kernel::Shape;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

/// Sobe quando a conversão muda o resultado: invalida o que foi guardado antes.
const VERSAO: &str = "forma-brep-1";
const NA_MEMORIA: usize = 8;
const NO_DISCO: usize = 32;

#[derive(Clone, Serialize, Deserialize)]
struct Guardado {
    brep: String,
    nomes: Vec<i32>,
    rel: RelatorioSalvo,
}

/// O relatório em forma de arquivo (o da conversão não é serializável e não precisa ser).
#[derive(Clone, Serialize, Deserialize)]
struct RelatorioSalvo {
    pedida: u8,
    remendos: usize,
    exatos: usize,
    regioes: Vec<(Option<u32>, [f64; 3], usize, u8, f64, f64, f64, f64, Option<String>)>,
    desvio_max: f64,
    amostras: Vec<([f64; 3], f64)>,
    solido: bool,
    faces: u32,
    nomes: Vec<i32>,
    tempo_ms: f64,
}

impl From<&RelatorioForma> for RelatorioSalvo {
    fn from(r: &RelatorioForma) -> Self {
        RelatorioSalvo {
            pedida: r.pedida,
            remendos: r.remendos,
            exatos: r.exatos,
            regioes: r.regioes.iter().map(|x| (x.no, x.ponto, x.valencia, x.continuidade, x.desvio, x.erro_g0, x.erro_g1, x.erro_g2, x.motivo.clone())).collect(),
            desvio_max: r.desvio_max,
            amostras: r.amostras.clone(),
            solido: r.solido,
            faces: r.faces,
            nomes: r.nomes.clone(),
            tempo_ms: r.tempo_ms,
        }
    }
}

impl From<&RelatorioSalvo> for RelatorioForma {
    fn from(r: &RelatorioSalvo) -> Self {
        RelatorioForma {
            pedida: r.pedida,
            remendos: r.remendos,
            exatos: r.exatos,
            regioes: r
                .regioes
                .iter()
                .map(|x| RegiaoRelatorio { no: x.0, ponto: x.1, valencia: x.2, continuidade: x.3, desvio: x.4, erro_g0: x.5, erro_g1: x.6, erro_g2: x.7, motivo: x.8.clone() })
                .collect(),
            desvio_max: r.desvio_max,
            amostras: r.amostras.clone(),
            solido: r.solido,
            faces: r.faces,
            nomes: r.nomes.clone(),
            tempo_ms: r.tempo_ms,
        }
    }
}

static MEMORIA: Mutex<Vec<(String, Guardado)>> = Mutex::new(Vec::new());

/// Impressão digital da entrada da conversão.
pub fn chave(g: &Gaiola, continuidade: u8, tol: f64) -> String {
    let mut h = blake3::Hasher::new();
    h.update(VERSAO.as_bytes());
    h.update(serde_json::to_string(g).unwrap_or_default().as_bytes());
    h.update(&[continuidade]);
    h.update(&tol.to_le_bytes());
    h.finalize().to_hex()[..32].to_string()
}

fn pasta() -> Option<PathBuf> {
    if std::env::var_os("FENKRA_SEM_CACHE_FORMA").is_some() {
        return None;
    }
    let base = std::env::var_os("FENKRA_CACHE_FORMA").map(PathBuf::from).or_else(|| std::env::var_os("LOCALAPPDATA").map(|a| PathBuf::from(a).join("Fenkra").join("cache_forma")))?;
    std::fs::create_dir_all(&base).ok()?;
    Some(base)
}

fn ler_disco(k: &str) -> Option<Guardado> {
    let t = std::fs::read_to_string(pasta()?.join(format!("{k}.json"))).ok()?;
    serde_json::from_str(&t).ok()
}

fn gravar_disco(k: &str, g: &Guardado) {
    let Some(p) = pasta() else { return };
    let Ok(t) = serde_json::to_string(g) else { return };
    // grava ao lado e troca: um processo lendo nunca vê o arquivo pela metade
    let tmp = p.join(format!("{k}.tmp{}", std::process::id()));
    if std::fs::write(&tmp, t).is_ok() {
        let _ = std::fs::rename(&tmp, p.join(format!("{k}.json")));
    }
    // poda: fica com as mais recentes
    if let Ok(rd) = std::fs::read_dir(&p) {
        let mut l: Vec<(std::time::SystemTime, PathBuf)> = rd
            .flatten()
            .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
            .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
            .collect();
        if l.len() > NO_DISCO {
            l.sort();
            for (_, f) in &l[..l.len() - NO_DISCO] {
                let _ = std::fs::remove_file(f);
            }
        }
    }
}

fn lembrar(k: &str, g: &Guardado) {
    if let Ok(mut m) = MEMORIA.lock() {
        m.retain(|(x, _)| x != k);
        m.push((k.to_string(), g.clone()));
        let n = m.len();
        if n > NA_MEMORIA {
            m.drain(..n - NA_MEMORIA);
        }
    }
}

/// Esquece as conversões guardadas na memória (o disco fica).
pub fn esquecer_memoria() {
    if let Ok(mut m) = MEMORIA.lock() {
        m.clear();
    }
}

fn da_memoria(k: &str) -> Option<Guardado> {
    MEMORIA.lock().ok()?.iter().find(|(x, _)| x == k).map(|(_, g)| g.clone())
}

fn abrir(g: &Guardado) -> Option<(Shape, Vec<i32>, RelatorioForma)> {
    let s = Shape::from_brep(&g.brep).ok()?;
    // a ordem das faces é a do texto: os nomes só valem se a contagem bate
    (s.face_count() as usize == g.nomes.len()).then(|| (s, g.nomes.clone(), RelatorioForma::from(&g.rel)))
}

/// De onde veio a última conversão pedida neste processo (para teste e para o relatório).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origem {
    Convertida,
    Memoria,
    Disco,
}

static ULTIMA: Mutex<Option<Origem>> = Mutex::new(None);

pub fn ultima_origem() -> Option<Origem> {
    ULTIMA.lock().ok().and_then(|o| *o)
}

/// [`forma_brep::converter`] com o resultado guardado. O relatório devolvido do cache traz o tempo
/// da conversão original; `tempo_ms` não mede a leitura.
pub fn converter(g: &Gaiola, continuidade: u8, tol: f64) -> Result<(Shape, Vec<i32>, RelatorioForma), String> {
    let k = chave(g, continuidade, tol);
    let marcar = |o| {
        if let Ok(mut u) = ULTIMA.lock() {
            *u = Some(o);
        }
    };
    if let Some(r) = da_memoria(&k).as_ref().and_then(abrir) {
        marcar(Origem::Memoria);
        return Ok(r);
    }
    if let Some(gd) = ler_disco(&k) {
        if let Some(r) = abrir(&gd) {
            lembrar(&k, &gd);
            marcar(Origem::Disco);
            return Ok(r);
        }
    }
    let (s, nomes, rel) = forma_brep::converter(g, continuidade, tol)?;
    if let Ok(brep) = s.to_brep() {
        let gd = Guardado { brep, nomes: nomes.clone(), rel: RelatorioSalvo::from(&rel) };
        lembrar(&k, &gd);
        gravar_disco(&k, &gd);
    }
    marcar(Origem::Convertida);
    Ok((s, nomes, rel))
}

/// Conversão em andamento: (feito, total) dos remendos e regiões. `None` fora de uma conversão.
pub mod andamento {
    use std::sync::atomic::{AtomicU32, Ordering};

    static FEITO: AtomicU32 = AtomicU32::new(0);
    static TOTAL: AtomicU32 = AtomicU32::new(0);

    pub fn comecar(total: u32) {
        FEITO.store(0, Ordering::Relaxed);
        TOTAL.store(total.max(1), Ordering::Relaxed);
    }
    pub fn passo() {
        FEITO.fetch_add(1, Ordering::Relaxed);
    }
    pub fn terminar() {
        TOTAL.store(0, Ordering::Relaxed);
    }
    pub fn atual() -> Option<(u32, u32)> {
        let t = TOTAL.load(Ordering::Relaxed);
        (t > 0).then(|| (FEITO.load(Ordering::Relaxed).min(t), t))
    }
}
