// SPDX-License-Identifier: LGPL-2.1-or-later
//! Esteira de mídia de ajuda (B257): roteiros por comando, gravação com passo de tempo fixo e
//! pacote externo de GIFs lido sob demanda.
//!
//! O fluxo, de ponta a ponta:
//! 1. `docs/media/roteiros/<comando>.json` descreve o caso, a câmera e os campos que se movem
//!    (este módulo lê e interpreta; tudo é função pura do tempo, sem relógio real);
//! 2. `FENKRA_DEMO_GRAVAR=<comando>` roda o roteiro no app (`app/gravar.rs`), um quadro a cada
//!    1/fps de tempo do roteiro, e grava os quadros 360x220 em pasta temporária;
//! 3. `scripts/media/montar.py` junta os quadros em GIF (64 cores, até 150 KB), escreve o
//!    `manifesto.json` com o hash do roteiro e monta o `ajuda.pack`;
//! 4. o app abre o pacote só ao pairar sobre um botão (`pacote.rs`), com cache LRU, sem crescer
//!    o executável (`exibir.rs` desenha o quadro certo pelo tempo).
//!
//! Tudo aqui é testável sem janela: roteiro, interpolação, remendo de campo, hash e manifesto.

pub mod casos;
pub mod exibir;
pub mod gif;
pub mod pacote;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};

/// Medidas fixas da mídia (o aceite do B257).
pub const LARGURA: u32 = 360;
pub const ALTURA: u32 = 220;
pub const FPS: u32 = 12;
pub const CORES: u32 = 64;
pub const LIMITE_BYTES: u64 = 150 * 1024;

/// Um campo que se move de `de` até `ate` entre `t0` e `t1` (segundos do roteiro).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Animacao {
    /// ponteiro JSON dentro do comando serializado, ex.: `/Shell/thickness`
    pub campo: String,
    pub de: f64,
    pub ate: f64,
    #[serde(default)]
    pub t0: f64,
    pub t1: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Roteiro {
    pub versao: u32,
    /// nome do ícone do botão que ganha a mídia
    pub comando: String,
    /// caso montado em código (`casos::montar`)
    pub caso: String,
    #[serde(default = "tema_padrao")]
    pub tema: String,
    /// câmera fixa: yaw e pitch (graus), distância e alvo x, y, z (mm). Fixa de propósito: o
    /// enquadramento automático mudaria a cada valor animado e a mídia tremeria.
    pub camera: [f32; 6],
    /// tamanho da janela, em pontos
    #[serde(default = "janela_padrao")]
    pub janela: [f32; 2],
    /// recorte x, y, largura, altura (pontos); a mídia final é este recorte reduzido a 360x220
    #[serde(default = "recorte_padrao")]
    pub recorte: [f32; 4],
    #[serde(default = "fps_padrao")]
    pub fps: u32,
    pub duracao: f64,
    #[serde(default)]
    pub animacoes: Vec<Animacao>,
}

fn tema_padrao() -> String {
    "Escuro".into()
}
fn janela_padrao() -> [f32; 2] {
    [1180., 700.]
}
fn recorte_padrao() -> [f32; 4] {
    [230., 110., 720., 440.]
}
fn fps_padrao() -> u32 {
    FPS
}

impl Roteiro {
    pub fn de_json(texto: &str) -> Result<Roteiro, String> {
        let r: Roteiro = serde_json::from_str(texto).map_err(|e| format!("roteiro inválido: {e}"))?;
        r.validar()?;
        Ok(r)
    }

    pub fn validar(&self) -> Result<(), String> {
        if self.versao != 1 {
            return Err(format!("versão {} de roteiro desconhecida", self.versao));
        }
        if self.comando.is_empty() || self.caso.is_empty() {
            return Err("roteiro sem comando ou caso".into());
        }
        if self.camera[2] <= 0. {
            return Err("distância da câmera deve ser positiva".into());
        }
        if self.fps == 0 || self.fps > 30 {
            return Err("fps fora de 1 a 30".into());
        }
        if !(0.5..=8.).contains(&self.duracao) {
            return Err("duração fora de 0,5 a 8 s".into());
        }
        let [x, y, w, h] = self.recorte;
        if w <= 0. || h <= 0. || x < 0. || y < 0. || x + w > self.janela[0] || y + h > self.janela[1] {
            return Err("recorte fora da janela".into());
        }
        // o recorte tem de ter a proporção da mídia, para a redução não deformar
        if (w / h - LARGURA as f32 / ALTURA as f32).abs() > 0.05 {
            return Err("recorte fora da proporção 360:220".into());
        }
        for a in &self.animacoes {
            if !a.campo.starts_with('/') || a.t1 <= a.t0 || a.t0 < 0. || a.t1 > self.duracao {
                return Err(format!("animação inválida em {}", a.campo));
            }
        }
        Ok(())
    }

    /// Quantos quadros o roteiro grava.
    pub fn quadros(&self) -> usize {
        (self.duracao * self.fps as f64).round() as usize
    }

    /// Tempo do roteiro no quadro `n`: sempre `n / fps`, sem relógio.
    pub fn tempo(&self, n: usize) -> f64 {
        n as f64 / self.fps as f64
    }

    /// Valor de uma animação no tempo `t` (suavizado nas pontas; fora do intervalo, segura o extremo).
    pub fn valor(a: &Animacao, t: f64) -> f64 {
        let u = ((t - a.t0) / (a.t1 - a.t0)).clamp(0., 1.);
        let s = u * u * (3. - 2. * u);
        a.de + (a.ate - a.de) * s
    }

    /// Aplica as animações do tempo `t` num comando serializado (ver `remendar`).
    pub fn aplicar(&self, kind: &mut Value, t: f64) -> Result<(), String> {
        for a in &self.animacoes {
            remendar(kind, &a.campo, Self::valor(a, t))?;
        }
        Ok(())
    }
}

/// Texto de um número para campo do comando: sem zeros sobrando, no máximo três casas.
pub fn texto_numero(v: f64) -> String {
    let s = format!("{v:.3}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() || s == "-0" {
        "0".into()
    } else {
        s.to_owned()
    }
}

/// Troca o valor do campo apontado por `ponteiro` (JSON Pointer). Campos de texto recebem o número
/// escrito; campos numéricos recebem o número. Outro tipo é erro, para o roteiro não escrever
/// onde não deve.
pub fn remendar(raiz: &mut Value, ponteiro: &str, v: f64) -> Result<(), String> {
    let alvo = raiz.pointer_mut(ponteiro).ok_or_else(|| format!("campo {ponteiro} não existe no comando"))?;
    match alvo {
        Value::String(_) => *alvo = Value::String(texto_numero(v)),
        Value::Number(_) => *alvo = serde_json::json!(v),
        _ => return Err(format!("campo {ponteiro} não é número nem texto")),
    }
    Ok(())
}

/// Hash FNV-1a de 64 bits, em hexadecimal, com CRLF contado como LF para não depender de como o
/// git baixou o arquivo. Serve para detectar mudança (não é criptográfico) e é o mesmo cálculo de
/// `scripts/media/montar.py`.
pub fn hash(bytes: &[u8]) -> String {
    let mut limpo = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\r' && bytes.get(i + 1) == Some(&b'\n') {
            i += 1;
            continue;
        }
        limpo.push(bytes[i]);
        i += 1;
    }
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in limpo {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{h:016x}")
}

/// Uma linha do manifesto.
#[derive(Clone, Debug, Deserialize)]
pub struct EntradaManifesto {
    pub id: String,
    pub hash_roteiro: String,
    pub gif: String,
    pub bytes_gif: u64,
    pub hash_gif: String,
    pub quadros: u32,
    #[serde(default)]
    pub largura: u32,
    #[serde(default)]
    pub altura: u32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Manifesto {
    pub versao: u32,
    pub midias: Vec<EntradaManifesto>,
}

/// Pasta `docs/media` do repositório (só para desenvolvimento e testes).
pub fn pasta_docs() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/media")
}

impl Manifesto {
    pub fn ler(pasta: &Path) -> Result<Manifesto, String> {
        let t = std::fs::read_to_string(pasta.join("manifesto.json")).map_err(|e| format!("manifesto: {e}"))?;
        serde_json::from_str(&t).map_err(|e| format!("manifesto inválido: {e}"))
    }
}

/// Roteiros de uma pasta, ordenados pelo nome do arquivo.
pub fn ler_roteiros(pasta: &Path) -> Result<Vec<(PathBuf, Roteiro)>, String> {
    let mut out = vec![];
    let mut arqs: Vec<PathBuf> = std::fs::read_dir(pasta)
        .map_err(|e| format!("{}: {e}", pasta.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    arqs.sort();
    for p in arqs {
        let t = std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))?;
        out.push((p.clone(), Roteiro::de_json(&t).map_err(|e| format!("{}: {e}", p.display()))?));
    }
    Ok(out)
}

#[cfg(test)]
mod testes {
    use super::*;

    fn exemplo() -> Roteiro {
        Roteiro::de_json(
            r#"{"versao":1,"comando":"casca","caso":"casca","duracao":3.0,"camera":[35,25,160,0,0,15],
                "animacoes":[{"campo":"/Shell/thickness","de":1,"ate":7,"t0":0.5,"t1":2.5}]}"#,
        )
        .unwrap()
    }

    /// Cobertura por hash (aceite do B257): todo roteiro versionado tem mídia em dia no manifesto, o GIF
    /// cabe no limite, decodifica com a medida e os quadros do roteiro, e está no pacote.
    #[test]
    fn todo_roteiro_tem_midia_em_dia_no_manifesto_e_no_pacote() {
        let pasta = pasta_docs();
        let roteiros = ler_roteiros(&pasta.join("roteiros")).expect("roteiros");
        assert!(roteiros.len() >= 10, "esperava dez roteiros ou mais, achei {}", roteiros.len());
        let man = Manifesto::ler(&pasta).expect("manifesto: rode scripts/media/montar.py gerar-todos");
        assert_eq!(man.versao, 1);
        let mut pacote = pacote::Midias::abrir(Some(&pasta.join("ajuda.pack")));
        assert!(pacote.tem_pacote(), "falta docs/media/ajuda.pack: rode montar.py empacotar");
        for (arq, r) in &roteiros {
            let e = man.midias.iter().find(|e| e.id == r.comando).unwrap_or_else(|| panic!("{}: sem mídia no manifesto", r.comando));
            let bytes_roteiro = std::fs::read(arq).unwrap();
            assert_eq!(e.hash_roteiro, hash(&bytes_roteiro), "{}: roteiro mudou depois da mídia; gere de novo", r.comando);
            let gif_bytes = std::fs::read(pasta.join(&e.gif)).unwrap_or_else(|_| panic!("{}: GIF ausente", r.comando));
            assert_eq!(hash(&gif_bytes), e.hash_gif, "{}: GIF não confere com o manifesto", r.comando);
            assert!(gif_bytes.len() as u64 <= LIMITE_BYTES && e.bytes_gif == gif_bytes.len() as u64, "{}: {} bytes", r.comando, gif_bytes.len());
            let g = gif::ler(&gif_bytes).unwrap_or_else(|er| panic!("{}: {er}", r.comando));
            assert_eq!((g.largura as u32, g.altura as u32, g.quadros.len()), (LARGURA, ALTURA, r.quadros()), "{}", r.comando);
            assert_eq!(pacote.bytes(&r.comando).map(|b| hash(&b)), Some(e.hash_gif.clone()), "{}: pacote difere do manifesto", r.comando);
        }
        // nenhuma mídia órfã: o manifesto só tem comandos que têm roteiro
        for e in &man.midias {
            assert!(roteiros.iter().any(|(_, r)| r.comando == e.id), "{}: mídia sem roteiro", e.id);
        }
    }

    #[test]
    fn roteiro_tem_padroes_e_valida() {
        let r = exemplo();
        assert_eq!((r.fps, r.quadros()), (12, 36));
        assert_eq!(r.tempo(6), 0.5);
        assert!(Roteiro::de_json(r#"{"versao":2,"comando":"a","caso":"a","duracao":3,"camera":[0,0,1,0,0,0]}"#).is_err());
        assert!(Roteiro::de_json(r#"{"versao":1,"comando":"a","caso":"a","duracao":30,"camera":[0,0,1,0,0,0]}"#).is_err());
        // recorte fora da proporção deformaria o quadro
        assert!(Roteiro::de_json(r#"{"versao":1,"comando":"a","caso":"a","duracao":3,"camera":[0,0,1,0,0,0],"recorte":[0,0,400,400]}"#).is_err());
        assert!(Roteiro::de_json(r#"{"versao":1,"comando":"a","caso":"a","duracao":3,"camera":[0,0,1,0,0,0],"animacoes":[{"campo":"x","de":0,"ate":1,"t1":1}]}"#).is_err());
    }

    #[test]
    fn valor_segura_as_pontas_e_passa_pelo_meio() {
        let r = exemplo();
        let a = &r.animacoes[0];
        assert_eq!(Roteiro::valor(a, 0.), 1.);
        assert_eq!(Roteiro::valor(a, 0.5), 1.);
        assert!((Roteiro::valor(a, 1.5) - 4.).abs() < 1e-9);
        assert_eq!(Roteiro::valor(a, 3.), 7.);
        // monotônico: a mídia nunca "volta"
        let mut ant = 0.;
        for n in 0..r.quadros() {
            let v = Roteiro::valor(a, r.tempo(n));
            assert!(v >= ant);
            ant = v;
        }
    }

    #[test]
    fn remendo_escreve_texto_ou_numero_e_recusa_o_resto() {
        let mut v = serde_json::json!({"Shell": {"thickness": "3", "closed": false, "n": 2}});
        remendar(&mut v, "/Shell/thickness", 4.5).unwrap();
        assert_eq!(v["Shell"]["thickness"], "4.5");
        remendar(&mut v, "/Shell/n", 5.).unwrap();
        assert_eq!(v["Shell"]["n"], 5.);
        assert!(remendar(&mut v, "/Shell/closed", 1.).is_err());
        assert!(remendar(&mut v, "/Shell/nada", 1.).is_err());
        assert_eq!(texto_numero(7.0), "7");
        assert_eq!(texto_numero(2.50), "2.5");
        assert_eq!(texto_numero(-0.0001), "0");
    }

    #[test]
    fn hash_ignora_crlf_e_muda_com_o_conteudo() {
        assert_eq!(hash(b"a\r\nb\n"), hash(b"a\nb\n"));
        assert_ne!(hash(b"a\nb\n"), hash(b"a\nc\n"));
        // valor conhecido de FNV-1a 64 para "a": confere com o script em Python
        assert_eq!(hash(b"a"), "af63dc4c8601ec8c");
        assert_eq!(hash(b""), "cbf29ce484222325");
    }
}
