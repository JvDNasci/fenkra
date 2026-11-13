// SPDX-License-Identifier: LGPL-2.1-or-later
//! B393: formas finais das peças guardadas em disco e calculadas em paralelo.
//!
//! O desenho de um conjunto só precisa da forma de cada peça, mas a carga refazia todos os comandos
//! de todas as peças, uma depois da outra (84 peças do kart: cerca de 50 s). Aqui:
//!
//! - a forma calculada vai para um arquivo binário cuja chave é o texto da peça mais a identidade do
//!   executável: abrir o mesmo conjunto de novo só lê os arquivos; mudar a peça ou o programa muda a
//!   chave e a forma é refeita;
//! - as peças que faltam são calculadas em várias threads, cada uma com o seu documento e a sua
//!   topologia (nada é dividido entre elas).
//!
//! Peça com malha lida de um arquivo externo a cada cálculo não entra no disco: a chave não veria a
//! mudança desse arquivo. `FENKRA_SEM_CACHE_FORMAS=1` desliga a leitura e a gravação.

use fenkra_core::{Document, FeatureKind};
use fenkra_kernel::Shape;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// FNV-1a de 64 bits: estável entre compilações, ao contrário do `DefaultHasher`.
pub fn fnv(h: &mut u64, bytes: &[u8]) {
    for b in bytes {
        *h ^= *b as u64;
        *h = h.wrapping_mul(0x100000001b3);
    }
}

/// Identidade do executável (caminho, tamanho e data): outro programa pode calcular diferente.
pub fn identidade_do_programa() -> &'static str {
    static ID: OnceLock<String> = OnceLock::new();
    ID.get_or_init(|| {
        let exe = std::env::current_exe().unwrap_or_default();
        let m = std::fs::metadata(&exe).ok();
        let data = m.as_ref().and_then(|m| m.modified().ok()).and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map_or(0, |d| d.as_nanos());
        format!("{}|{}|{}|{}", env!("CARGO_PKG_VERSION"), exe.display(), m.map_or(0, |m| m.len()), data)
    })
}

pub fn desligado() -> bool {
    std::env::var_os("FENKRA_SEM_CACHE_FORMAS").is_some()
}

/// Pasta das formas guardadas. `FENKRA_CACHE_FORMAS=<pasta>` troca o lugar (testes).
pub fn pasta() -> PathBuf {
    if let Some(p) = std::env::var_os("FENKRA_CACHE_FORMAS") {
        return PathBuf::from(p);
    }
    let base = std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    base.join("Fenkra").join("formas")
}

/// A peça pode ir para o disco? Não, se lê um arquivo externo a cada cálculo.
pub fn guardavel(doc: &Document) -> bool {
    !doc.features.iter().any(|f| matches!(&f.kind, FeatureKind::MeshImport(m) if m.dados.is_empty()))
}

/// Chave da forma: texto da peça e identidade do programa.
pub fn chave(texto: &str) -> String {
    let mut h = 0xcbf29ce484222325u64;
    fnv(&mut h, identidade_do_programa().as_bytes());
    fnv(&mut h, &[0]);
    fnv(&mut h, texto.as_bytes());
    // segunda passada com outra semente: 128 bits tornam uma colisão impensável
    let mut g = 0x84222325cbf29ce4u64;
    fnv(&mut g, texto.as_bytes());
    fnv(&mut g, identidade_do_programa().as_bytes());
    format!("{h:016x}{g:016x}")
}

fn arquivo(dir: &Path, chave: &str) -> PathBuf {
    dir.join(format!("{chave}.fkf"))
}

/// Forma guardada com essa chave. `Some(None)`: a peça foi calculada e não tem corpo visível.
pub fn ler(chave: &str) -> Option<Option<Shape>> {
    if desligado() {
        return None;
    }
    let dados = std::fs::read(arquivo(&pasta(), chave)).ok()?;
    if dados.is_empty() {
        return Some(None);
    }
    Shape::de_bin(&dados).ok().map(Some)
}

/// Guarda a forma (ou a falta dela). Grava num temporário e renomeia: outro processo nunca lê um
/// arquivo pela metade.
pub fn gravar(chave: &str, forma: Option<&Shape>) {
    if desligado() {
        return;
    }
    let dados = match forma {
        Some(s) => match s.para_bin() {
            Ok(d) if !d.is_empty() => d,
            _ => return,
        },
        None => vec![],
    };
    let dir = pasta();
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    limpar_uma_vez(&dir);
    let tmp = dir.join(format!("{chave}.{}.tmp", std::process::id()));
    if std::fs::write(&tmp, &dados).is_ok() && std::fs::rename(&tmp, arquivo(&dir, chave)).is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
}

/// Dias que um arquivo guardado fica na pasta sem ser regravado.
const DIAS: u64 = 30;

/// Na primeira gravação do processo, apaga da pasta o que foi gravado há mais de [`DIAS`] (formas,
/// linhas das vistas e temporários largados): a pasta não cresce sem fim. Uma forma ainda usada só é
/// calculada de novo.
pub fn limpar_uma_vez(dir: &Path) {
    static FEITO: OnceLock<()> = OnceLock::new();
    FEITO.get_or_init(|| {
        let Ok(r) = std::fs::read_dir(dir) else { return };
        let limite = std::time::Duration::from_secs(DIAS * 86_400);
        for e in r.flatten() {
            let p = e.path();
            let nosso = p.extension().is_some_and(|x| x == "fkf" || x == "fkl" || x == "tmp");
            let velho = e.metadata().ok().and_then(|m| m.modified().ok()).and_then(|t| t.elapsed().ok()).is_some_and(|d| d > limite);
            if nosso && velho {
                let _ = std::fs::remove_file(p);
            }
        }
    });
}

/// Forma calculada noutra thread. A topologia nasceu nela e não é dividida com ninguém: passar a
/// posse para a thread que a pediu é seguro (a contagem de referências do OCCT é atômica).
struct Enviada(Option<Shape>);
unsafe impl Send for Enviada {}

/// Uma peça a calcular: caminho, texto e documento já lidos.
pub struct Pedido {
    pub caminho: PathBuf,
    pub texto: String,
    pub doc: Document,
}

/// Calcula as formas finais em `threads` threads (0 = metade dos núcleos, até 6), guardando cada uma
/// no disco quando possível. `cancelar` interrompe entre uma peça e outra. Devolve na ordem pedida;
/// peça não calculada (cancelada) fica de fora.
pub fn calcular(pedidos: Vec<Pedido>, threads: usize, cancelar: Option<&std::sync::atomic::AtomicBool>) -> Vec<(PathBuf, Option<Shape>)> {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let n = pedidos.len();
    let t = if threads == 0 { std::thread::available_parallelism().map_or(2, |n| (n.get() / 2).clamp(1, 6)) } else { threads }.min(n).max(1);
    let proximo = AtomicUsize::new(0);
    let parar = || cancelar.is_some_and(|c| c.load(Ordering::Relaxed));
    let um = |i: usize| -> Enviada {
        let p = &pedidos[i];
        let forma = fenkra_solidos::final_shape(&p.doc);
        if guardavel(&p.doc) {
            gravar(&chave(&p.texto), forma.as_ref());
        }
        Enviada(forma)
    };
    let mut feitos: Vec<(usize, Enviada)> = if t == 1 {
        let mut v = vec![];
        for i in 0..n {
            if parar() {
                break;
            }
            v.push((i, um(i)));
        }
        v
    } else {
        std::thread::scope(|sc| {
            let ts: Vec<_> = (0..t)
                .map(|_| {
                    sc.spawn(|| {
                        let mut v = vec![];
                        while !parar() {
                            let i = proximo.fetch_add(1, Ordering::Relaxed);
                            if i >= n {
                                break;
                            }
                            v.push((i, um(i)));
                        }
                        v
                    })
                })
                .collect();
            ts.into_iter().flat_map(|t| t.join().unwrap_or_else(|e| std::panic::resume_unwind(e))).collect()
        })
    };
    feitos.sort_by_key(|x| x.0);
    let mut caminhos: Vec<Option<PathBuf>> = pedidos.into_iter().map(|p| Some(p.caminho)).collect();
    feitos.into_iter().map(|(i, e)| (caminhos[i].take().unwrap_or_default(), e.0)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::load::PartCache;

    fn pasta_teste(nome: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("fenkra_formas_disco_{nome}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn so_forma() -> PartCache {
        let mut c = PartCache::default();
        c.so_forma = true;
        c
    }

    fn com_parametro(d: &mut Document, mais: f64) {
        if let Some(p) = d.params.first_mut() {
            let v: f64 = p.expr.parse().unwrap_or(10.);
            p.expr = format!("{}", v + mais);
        }
    }

    /// Cinco peças: o exemplo com o primeiro parâmetro em cinco valores (formas diferentes).
    fn pecas(dir: &Path) -> Vec<PathBuf> {
        (0..5)
            .map(|k| {
                let mut d = fenkra_solidos::example_document();
                com_parametro(&mut d, k as f64);
                let f = dir.join(format!("p{k}.fkp"));
                std::fs::write(&f, d.to_json()).unwrap();
                f
            })
            .collect()
    }

    #[test]
    fn chave_muda_com_o_texto() {
        assert_ne!(chave("a"), chave("b"));
        assert_eq!(chave("a"), chave("a"));
        assert_eq!(chave("a").len(), 32);
    }

    #[test]
    fn formas_em_paralelo_iguais_as_de_uma_por_uma_e_depois_lidas_do_disco() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let dir = pasta_teste("paralelo");
        let cache_dir = dir.join("cache");
        std::env::set_var("FENKRA_CACHE_FORMAS", &cache_dir);
        let arquivos = pecas(&dir);
        let esperado: Vec<(f64, ([f64; 3], [f64; 3]))> = arquivos
            .iter()
            .map(|f| {
                let d = Document::from_json(&std::fs::read_to_string(f).unwrap()).unwrap();
                let s = fenkra_solidos::final_shape(&d).unwrap();
                (s.mass().volume, s.bbox().unwrap())
            })
            .collect();

        let mut c = so_forma();
        c.preparar_formas(&arquivos);
        // cada peça foi para o disco
        let guardadas = std::fs::read_dir(&cache_dir).unwrap().filter(|e| e.as_ref().is_ok_and(|e| e.path().extension().is_some_and(|x| x == "fkf"))).count();
        assert!(guardadas >= arquivos.len(), "{guardadas}");

        let conferir = |c: &mut PartCache| {
            for (f, (v, (lo, hi))) in arquivos.iter().zip(&esperado) {
                let s = c.forma(f).unwrap().expect("forma");
                assert!((s.mass().volume - v).abs() < 1e-6 * v.max(1.), "{} {v}", s.mass().volume);
                let (a, b) = s.bbox().unwrap();
                for k in 0..3 {
                    assert!((a[k] - lo[k]).abs() < 1e-6 && (b[k] - hi[k]).abs() < 1e-6, "caixa {a:?} {b:?} {lo:?} {hi:?}");
                }
            }
        };
        conferir(&mut c);
        // outro cache, mesmo programa: tudo sai do disco
        let mut lido = so_forma();
        lido.preparar_formas(&arquivos);
        conferir(&mut lido);
        // a peça muda: a chave muda e a forma é refeita
        let mut d = Document::from_json(&std::fs::read_to_string(&arquivos[0]).unwrap()).unwrap();
        com_parametro(&mut d, 20.);
        let novo = fenkra_solidos::final_shape(&d).unwrap().mass().volume;
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(&arquivos[0], d.to_json()).unwrap();
        let mut c3 = so_forma();
        assert!((c3.forma(&arquivos[0]).unwrap().unwrap().mass().volume - novo).abs() < 1e-6 * novo);
        assert!((novo - esperado[0].0).abs() > 1e-3, "o parâmetro muda o volume");
    }

    #[test]
    fn cancelado_nao_calcula_nada() {
        let flag = std::sync::atomic::AtomicBool::new(true);
        let d = fenkra_solidos::example_document();
        let pedidos = vec![Pedido { caminho: "a.fkp".into(), texto: d.to_json(), doc: d }];
        assert!(calcular(pedidos, 2, Some(&flag)).is_empty());
    }
}
