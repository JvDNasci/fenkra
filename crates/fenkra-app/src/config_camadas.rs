// SPDX-License-Identifier: LGPL-2.1-or-later
//! Configurações em camadas no aplicativo (B100): classificação de cada opção, leitura e gravação
//! do `config.json` com backup, camadas de empresa e de projeto com chaves travadas, opções que
//! vão no arquivo do documento e perfil de interface.
//!
//! A lógica genérica (resolver camadas, travas, perfil, arquivo com backup) está em
//! `fenkra_core::config`; aqui ficam o que depende de [`Settings`].
//!
//! Onde ficam as camadas:
//! - programa: `Settings::default()`;
//! - empresa: `%PROGRAMDATA%\Fenkra\empresa.json` (ou o caminho da variável `FENKRA_CONFIG_EMPRESA`);
//! - projeto: `<pasta do projeto>\.fenkra\config.json`;
//! - usuário: `%APPDATA%\Fenkra\config.json` (só o que a pessoa mudou);
//! - documento: o campo `config` do arquivo da peça, só nas opções que pertencem a ele.

use crate::settings::Settings;
use fenkra_core::config::{self, Camada, Mapa, Resolvido};
use fenkra_core::Document;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

/// A que pertence cada opção.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Classe {
    /// vai no arquivo do documento quando a pessoa pede (cotagem, grade, anotações 3D, qualidade)
    Documento,
    /// jeito de trabalhar e aparência: entra no perfil de interface exportável
    Interface,
    /// depende da máquina (gráficos, desempenho): não viaja entre computadores
    Maquina,
    /// dados da sessão e conteúdo pessoal (recentes, macro, perfis cadastrados)
    Sessao,
}

use Classe::*;

/// Toda opção de [`Settings`] está aqui; o teste confere que nenhuma ficou de fora.
pub const CLASSIFICACAO: &[(&str, Classe)] = &[
    ("opcoes_importar", Sessao),
    ("opcoes_exportar", Sessao),
    ("impressao", Sessao),
    ("fatiador", Maquina),
    ("theme", Interface),
    ("ui_scale", Interface),
    ("gradient", Interface),
    ("bg_top", Interface),
    ("bg_bottom", Interface),
    ("display", Interface),
    ("curv_tipo", Interface),
    ("curv_raio", Interface),
    ("incl_eixo", Interface),
    ("incl_inverte", Interface),
    ("incl_min", Interface),
    ("edges_in_render", Interface),
    ("tangent_edges", Interface),
    ("ortho", Interface),
    ("filter", Interface),
    ("docks", Interface),
    ("families", Interface),
    ("nav", Interface),
    ("invert_zoom", Interface),
    ("zoom_no_cursor", Interface),
    ("sel_faces", Interface),
    ("sel_edges", Interface),
    ("sel_vertices", Interface),
    ("recent", Sessao),
    ("recent_files", Sessao),
    ("show_start", Interface),
    ("icon_size", Interface),
    ("show_labels", Interface),
    ("dim_standard", Documento),
    ("dim_decimals", Documento),
    ("dim_text_height", Documento),
    ("grid", Documento),
    ("grid_step", Documento),
    ("snaps", Interface),
    ("auto_constraints", Interface),
    ("snap_restringe", Interface),
    ("juntar_ao_soltar", Interface),
    ("auto_dims", Interface),
    ("dim_on_type", Interface),
    ("keys", Interface),
    ("english", Interface),
    ("custom", Interface),
    ("user_profiles", Sessao),
    ("bench", Sessao),
    ("auto_update", Interface),
    ("historico_mb", Maquina),
    ("editor_left", Interface),
    ("sketch_in_geometry", Interface),
    ("point_style", Interface),
    ("point_size", Interface),
    ("ref_line_width", Interface),
    ("ref_line_dash", Interface),
    ("hole_mark", Interface),
    ("tree_hidden", Interface),
    ("box_mode", Interface),
    ("wheel_mode", Interface),
    ("roda", Interface),
    ("user_colors", Interface),
    ("menu_theme", Interface),
    ("theme_name", Interface),
    ("pmi_style", Documento),
    ("flanged_holes", Sessao),
    ("macro_src", Sessao),
    ("graficos", Maquina),
    ("modo_leve", Maquina),
    ("leve_anterior", Maquina),
    ("animacoes", Maquina),
    ("acessibilidade", Interface),
    ("mouse3d", Maquina),
    ("sugestao_leve_vista", Sessao),
    ("curso", Interface),
    ("paineis", Interface),
    ("barra_fixos", Interface),
];

/// Chaves que o documento pode guardar. `qualidade_malha` é virtual: junta `graficos.preset` e
/// `graficos.livre`, a única parte dos gráficos que muda o resultado da peça.
pub const CHAVES_DOC: &[&str] = &["dim_standard", "dim_decimals", "dim_text_height", "grid", "grid_step", "pmi_style", "qualidade_malha"];

pub fn classe(chave: &str) -> Option<Classe> {
    CLASSIFICACAO.iter().find(|(k, _)| *k == chave).map(|(_, c)| *c)
}

pub fn chaves_da_classe(c: Classe) -> Vec<&'static str> {
    CLASSIFICACAO.iter().filter(|(_, x)| *x == c).map(|(k, _)| *k).collect()
}

// ---------------------------------------------------------------- Settings <-> mapa

pub fn mapa_de(s: &Settings) -> Mapa {
    match serde_json::to_value(s) {
        Ok(Value::Object(m)) => m,
        _ => Mapa::new(),
    }
}

pub fn settings_de(m: Mapa) -> Settings {
    serde_json::from_value::<Settings>(Value::Object(m)).unwrap_or_default()
}

/// As opções de documento de `s`, com a qualidade da malha no formato virtual.
pub fn chaves_doc_de(s: &Settings) -> Mapa {
    let m = mapa_de(s);
    let mut out = Mapa::new();
    for k in CHAVES_DOC {
        if let Some(v) = m.get(*k) {
            out.insert((*k).into(), v.clone());
        }
    }
    if let Some(g) = m.get("graficos") {
        let mut q = Mapa::new();
        for k in ["preset", "livre"] {
            if let Some(v) = g.get(k) {
                q.insert(k.into(), v.clone());
            }
        }
        out.insert("qualidade_malha".into(), Value::Object(q));
    }
    out
}

/// Aplica as opções de documento em `s`, ignorando as travadas por empresa ou projeto.
pub fn aplicar_chaves_doc(s: &mut Settings, doc: &Mapa, base: &Resolvido) {
    let mut m = mapa_de(s);
    for (k, v) in doc {
        if !CHAVES_DOC.contains(&k.as_str()) || base.travadas.contains_key(k) {
            continue;
        }
        if k == "qualidade_malha" {
            if let (Some(Value::Object(q)), Some(Value::Object(g))) = (Some(v), m.get_mut("graficos")) {
                for (qk, qv) in q {
                    g.insert(qk.clone(), qv.clone());
                }
            }
        } else {
            m.insert(k.clone(), v.clone());
        }
    }
    *s = settings_de(m);
}

// ---------------------------------------------------------------- estado global

#[derive(Default)]
pub struct Estado {
    pub empresa: Option<Camada>,
    pub projeto: Option<Camada>,
    pub caminho_projeto: Option<PathBuf>,
    /// avisos ainda não mostrados (arquivo ruim, falha ao gravar, camada de versão futura)
    pub avisos: Vec<String>,
    /// o que as camadas de baixo (programa, empresa, projeto) resolvem, sem o usuário
    pub base: Resolvido,
    /// documento ativo e se as opções dele estão valendo no lugar das do usuário
    uid: String,
    sobrepondo: bool,
    /// opções do usuário guardadas enquanto o documento sobrepõe as dele
    guardado: Mapa,
    ultima: Option<Settings>,
}

static ESTADO: std::sync::OnceLock<Mutex<Estado>> = std::sync::OnceLock::new();

fn estado() -> MutexGuard<'static, Estado> {
    ESTADO.get_or_init(|| Mutex::new(Estado::default())).lock().unwrap_or_else(|p| p.into_inner())
}

pub fn tirar_avisos() -> Vec<String> {
    std::mem::take(&mut estado().avisos)
}

// ---------------------------------------------------------------- caminhos

pub fn caminho_usuario() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join("Fenkra").join("config.json"))
}

pub fn caminho_empresa() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("FENKRA_CONFIG_EMPRESA") {
        return Some(PathBuf::from(p));
    }
    std::env::var_os("PROGRAMDATA").map(|a| PathBuf::from(a).join("Fenkra").join("empresa.json"))
}

pub fn caminho_projeto(pasta: &Path) -> PathBuf {
    pasta.join(".fenkra").join("config.json")
}

fn ler_camada(p: &Path, quem: &str, avisos: &mut Vec<String>) -> Option<Camada> {
    let l = config::ler_com_backup(p);
    if let Some(a) = l.aviso {
        avisos.push(format!("{quem}: {a}"));
    }
    let v = l.valor?;
    match Camada::ler(&v.to_string()) {
        Ok((c, aviso)) => {
            if let Some(a) = aviso {
                avisos.push(format!("{quem}: {a}"));
            }
            Some(c)
        }
        Err(e) => {
            avisos.push(format!("{quem}: {e}"));
            None
        }
    }
}

fn recalcular_base(e: &mut Estado) {
    let programa = mapa_de(&Settings::default());
    e.base = config::resolver(&programa, e.empresa.as_ref(), e.projeto.as_ref(), &Mapa::new(), None, CHAVES_DOC);
}

// ---------------------------------------------------------------- carregar e gravar

/// Lê as camadas e devolve as configurações em vigor. O que deu errado vira aviso (nada é perdido
/// em silêncio) e a execução segue com o que foi possível ler.
pub fn carregar() -> Settings {
    carregar_de(caminho_usuario().as_deref(), caminho_empresa().as_deref())
}

pub fn carregar_de(usuario: Option<&Path>, empresa: Option<&Path>) -> Settings {
    let mut e = estado();
    e.avisos.clear();
    e.uid.clear();
    e.sobrepondo = false;
    e.guardado.clear();
    let mut avisos = vec![];
    e.empresa = empresa.and_then(|p| ler_camada(p, "Configuração da empresa", &mut avisos));
    let u = usuario.map(config::ler_com_backup).unwrap_or_default();
    if let Some(a) = u.aviso {
        avisos.push(format!("config.json: {a}"));
    }
    let usuario_mapa = match u.valor {
        Some(Value::Object(m)) => m,
        _ => Mapa::new(),
    };
    recalcular_base(&mut e);
    let programa = mapa_de(&Settings::default());
    let r = config::resolver(&programa, e.empresa.as_ref(), e.projeto.as_ref(), &usuario_mapa, None, CHAVES_DOC);
    e.avisos.extend(avisos);
    e.ultima = None;
    settings_de(r.valores).migrar_tema()
}

/// O que vai para o `config.json`: só o que a pessoa mudou em relação às camadas de baixo. Com um
/// documento sobrepondo as opções dele, o arquivo leva as opções do usuário, não as do documento.
pub fn texto_do_usuario(s: &Settings) -> String {
    let e = estado();
    let mut atual = mapa_de(s);
    if e.sobrepondo {
        let mut tmp = s.clone();
        aplicar_chaves_doc(&mut tmp, &e.guardado, &Resolvido::default());
        atual = mapa_de(&tmp);
    }
    let dif = config::diferenca(&atual, &e.base);
    serde_json::to_string_pretty(&Value::Object(dif)).unwrap_or_default()
}

/// Grava o `config.json` com backup. Falha vira aviso para a interface, nunca silêncio.
pub fn gravar(s: &Settings) -> Result<(), String> {
    // demonstrações e testes (variáveis FENKRA_DEMO_*) não mexem nas preferências
    if std::env::vars().any(|(k, _)| k.starts_with("FENKRA_DEMO") || k == "FENKRA_ABRIR" || k == "FENKRA_BANCADA") {
        return Ok(());
    }
    let Some(p) = caminho_usuario() else { return Ok(()) };
    gravar_em(&p, s)
}

pub fn gravar_em(p: &Path, s: &Settings) -> Result<(), String> {
    let texto = texto_do_usuario(s);
    config::gravar_com_backup(p, &texto).inspect_err(|m| {
        let mut e = estado();
        let aviso = format!("Não consegui gravar as configurações: {m}. Elas continuam valendo nesta sessão.");
        if !e.avisos.contains(&aviso) {
            e.avisos.push(aviso);
        }
    })
}

/// Troca a camada de projeto (ao abrir ou fechar um projeto).
pub fn definir_projeto(pasta: Option<&Path>) {
    let mut e = estado();
    if e.caminho_projeto.as_deref() == pasta {
        return;
    }
    e.caminho_projeto = pasta.map(Path::to_path_buf);
    let mut avisos = vec![];
    e.projeto = pasta.and_then(|p| {
        let c = caminho_projeto(p);
        c.exists().then(|| ler_camada(&c, "Configuração do projeto", &mut avisos)).flatten()
    });
    e.avisos.extend(avisos);
    recalcular_base(&mut e);
    e.ultima = None;
}

/// Repõe os valores travados em `s`. Devolve as chaves que a pessoa tinha mudado.
pub fn reforcar(s: &mut Settings) -> Vec<String> {
    let e = estado();
    if e.base.travadas.is_empty() {
        return vec![];
    }
    let mut m = mapa_de(s);
    let mexidas = config::reforcar_travas(&mut m, &e.base);
    if !mexidas.is_empty() {
        *s = settings_de(m);
    }
    mexidas
}

/// Quem travou o quê, para a página de configurações.
pub fn travadas() -> Vec<(String, String)> {
    estado().base.travadas.iter().map(|(k, o)| (k.clone(), o.rotulo().to_string())).collect()
}

pub fn camadas_presentes() -> (bool, bool) {
    let e = estado();
    (e.empresa.is_some(), e.projeto.is_some())
}

// ---------------------------------------------------------------- documento

pub fn documento_sobrepoe() -> bool {
    estado().sobrepondo
}

/// Liga ou desliga "estas opções valem só para este documento". Ligando, o documento guarda as
/// opções em vigor; desligando, ele as solta e a sessão volta às opções do usuário.
pub fn somente_documento(ligar: bool, s: &mut Settings, doc: &mut Document) {
    let mut e = estado();
    if ligar && !e.sobrepondo {
        e.guardado = chaves_doc_de(s);
        doc.config = chaves_doc_de(s);
        e.sobrepondo = true;
    } else if !ligar && e.sobrepondo {
        let guardado = std::mem::take(&mut e.guardado);
        let base = e.base.clone();
        aplicar_chaves_doc(s, &guardado, &base);
        doc.config.clear();
        e.sobrepondo = false;
    }
    e.ultima = None;
}

/// Roda a cada quadro, barato quando nada mudou: trava as chaves das camadas de cima, troca as
/// opções de documento quando o documento ativo muda e grava no documento o que a pessoa ajusta
/// enquanto ele sobrepõe as opções.
pub fn sincronizar(s: &mut Settings, doc: &mut Document) {
    let mut e = estado();
    if e.uid == doc.uid.0 && e.ultima.as_ref() == Some(&*s) {
        return;
    }
    let base = e.base.clone();
    // 1) travas de empresa e de projeto mandam
    if !base.travadas.is_empty() {
        let mut m = mapa_de(s);
        if !config::reforcar_travas(&mut m, &base).is_empty() {
            *s = settings_de(m);
        }
    }
    // 2) outro documento: ele sobrepõe as opções ou devolve as do usuário
    if e.uid != doc.uid.0 {
        e.uid = doc.uid.0.clone();
        if e.sobrepondo {
            let guardado = std::mem::take(&mut e.guardado);
            aplicar_chaves_doc(s, &guardado, &base);
            e.sobrepondo = false;
        }
        if !doc.config.is_empty() {
            e.guardado = chaves_doc_de(s);
            aplicar_chaves_doc(s, &doc.config.clone(), &base);
            e.sobrepondo = true;
        }
    } else if e.sobrepondo {
        // o documento sobrepõe: o que a pessoa ajusta vai para o arquivo dele
        let atual = chaves_doc_de(s);
        if atual != doc.config {
            doc.config = atual;
        }
    }
    e.ultima = Some(s.clone());
}

// ---------------------------------------------------------------- perfil de interface

pub fn exportar_perfil(s: &Settings) -> String {
    let chaves = chaves_da_classe(Interface);
    config::exportar_perfil(&mapa_de(s), &chaves, fenkra_core::formato::APP_VERSION)
}

/// Resultado da importação, para a interface contar à pessoa.
pub struct ResumoPerfil {
    pub aplicadas: usize,
    pub ignoradas: Vec<String>,
    pub travadas: Vec<String>,
    pub aviso: Option<String>,
}

pub fn importar_perfil(texto: &str, s: &mut Settings) -> Result<ResumoPerfil, String> {
    let base = estado().base.clone();
    let chaves = chaves_da_classe(Interface);
    let p = config::importar_perfil(texto, &chaves, &base)?;
    let mut m = mapa_de(s);
    let aplicadas = p.valores.len();
    for (k, v) in p.valores {
        m.insert(k, v);
    }
    *s = settings_de(m).migrar_tema();
    Ok(ResumoPerfil { aplicadas, ignoradas: p.ignoradas, travadas: p.travadas, aviso: p.aviso })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// O estado das camadas é global: os testes que o usam rodam um por vez.
    fn exclusivo() -> MutexGuard<'static, ()> {
        static T: Mutex<()> = Mutex::new(());
        T.lock().unwrap_or_else(|p| p.into_inner())
    }

    fn pasta(nome: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("fenkra_cfg_app_{nome}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn toda_opcao_esta_classificada_e_nenhuma_sobra() {
        let m = mapa_de(&Settings::default());
        for k in m.keys() {
            assert!(classe(k).is_some(), "a opção `{k}` não tem classe em CLASSIFICACAO");
        }
        for (k, _) in CLASSIFICACAO {
            assert!(m.contains_key(*k), "`{k}` está classificada mas não existe mais em Settings");
        }
        let mut vistos = std::collections::HashSet::new();
        assert!(CLASSIFICACAO.iter().all(|(k, _)| vistos.insert(*k)), "chave repetida");
    }

    #[test]
    fn chaves_de_documento_sao_as_classificadas_como_documento() {
        let mut esperadas: Vec<&str> = chaves_da_classe(Documento);
        esperadas.push("qualidade_malha");
        let mut reais: Vec<&str> = CHAVES_DOC.to_vec();
        esperadas.sort();
        reais.sort();
        assert_eq!(esperadas, reais);
        let d = chaves_doc_de(&Settings::default());
        assert_eq!(d.len(), CHAVES_DOC.len(), "{d:?}");
    }

    #[test]
    fn documento_aplica_e_devolve_as_opcoes_dele() {
        let mut s = Settings::default();
        let mut outro = s.clone();
        outro.dim_decimals = 4;
        outro.grid_step = 2.5;
        outro.graficos.preset = fenkra_core::PresetMalha::Alta;
        let doc = chaves_doc_de(&outro);
        aplicar_chaves_doc(&mut s, &doc, &Resolvido::default());
        assert_eq!(s.dim_decimals, 4);
        assert_eq!(s.grid_step, 2.5);
        assert_eq!(s.graficos.preset, fenkra_core::PresetMalha::Alta);
    }

    #[test]
    fn gravar_so_o_que_mudou_e_recarregar_da_o_mesmo() {
        let _x = exclusivo();
        let d = pasta("grava");
        let arq = d.join("config.json");
        let mut s = Settings::default();
        s.ui_scale = 1.25;
        s.dim_decimals = 5;
        gravar_em(&arq, &s).unwrap();
        let t = std::fs::read_to_string(&arq).unwrap();
        let v: Value = serde_json::from_str(&t).unwrap();
        let o = v.as_object().unwrap();
        assert_eq!(o.len(), 2, "só as duas mudanças vão para o arquivo: {t}");
        assert!(o.contains_key("ui_scale") && o.contains_key("dim_decimals"));
        let volta = carregar_de(Some(&arq), None);
        assert_eq!(volta.ui_scale, 1.25);
        assert_eq!(volta.dim_decimals, 5);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn config_corrompido_volta_do_backup_e_avisa() {
        let _x = exclusivo();
        let d = pasta("backup");
        let arq = d.join("config.json");
        let mut s = Settings::default();
        s.ui_scale = 1.5;
        gravar_em(&arq, &s).unwrap();
        s.ui_scale = 1.75;
        gravar_em(&arq, &s).unwrap();
        std::fs::write(&arq, "{ lixo").unwrap();
        let volta = carregar_de(Some(&arq), None);
        assert_eq!(volta.ui_scale, 1.5, "voltou o penúltimo gravado");
        let avisos = tirar_avisos();
        assert!(avisos.iter().any(|a| a.contains("restaurado o backup")), "{avisos:?}");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn empresa_trava_e_o_usuario_nao_consegue_mudar() {
        let _x = exclusivo();
        let d = pasta("empresa");
        let emp = d.join("empresa.json");
        let mut c = Camada::nova();
        c.valores.insert("dim_standard".into(), Value::from("ABNT"));
        c.valores.insert("dim_decimals".into(), Value::from(1));
        c.travadas = vec!["dim_standard".into()];
        std::fs::write(&emp, c.para_texto()).unwrap();
        let usuario = d.join("config.json");
        std::fs::write(&usuario, r#"{"dim_standard": "DIN", "dim_decimals": 3}"#).unwrap();
        let mut s = carregar_de(Some(&usuario), Some(&emp));
        assert_eq!(s.dim_standard, "ABNT", "trava da empresa");
        assert_eq!(s.dim_decimals, 3, "sem trava, o usuário vence a empresa");
        // a pessoa tenta mudar a chave travada na interface: a sincronização repõe
        s.dim_standard = "ISO".into();
        let mut doc = Document::new("t");
        sincronizar(&mut s, &mut doc);
        assert_eq!(s.dim_standard, "ABNT");
        assert!(travadas().iter().any(|(k, q)| k == "dim_standard" && q == "empresa"));
        // e a trava nunca é gravada no arquivo do usuário
        gravar_em(&usuario, &s).unwrap();
        let v: Value = serde_json::from_str(&std::fs::read_to_string(&usuario).unwrap()).unwrap();
        assert!(v.get("dim_standard").is_none());
        let _ = std::fs::remove_dir_all(&d);
        // limpa o estado global para os outros testes
        carregar_de(None, None);
    }

    #[test]
    fn perfil_de_interface_exporta_e_importa_sem_levar_dados_da_maquina() {
        let _x = exclusivo();
        carregar_de(None, None);
        let mut a = Settings::default();
        a.ui_scale = 1.4;
        a.icon_size = 40.;
        a.historico_mb = 123;
        a.recent_files = vec!["x.fkp".into()];
        let texto = exportar_perfil(&a);
        assert!(texto.contains("ui_scale"));
        assert!(!texto.contains("historico_mb") && !texto.contains("recent_files"));
        let mut b = Settings::default();
        let r = importar_perfil(&texto, &mut b).unwrap();
        assert!(r.aplicadas > 0);
        assert_eq!(b.ui_scale, 1.4);
        assert_eq!(b.icon_size, 40.);
        assert_ne!(b.historico_mb, 123, "máquina não viaja no perfil");
        assert!(importar_perfil("{\"formato\":\"x\"}", &mut b).is_err());
    }

    #[test]
    fn documento_sobrepoe_grava_no_arquivo_e_solta() {
        let _x = exclusivo();
        carregar_de(None, None);
        let mut s = Settings::default();
        let mut doc = Document::new("a");
        sincronizar(&mut s, &mut doc);
        assert!(!documento_sobrepoe());
        // liga: o documento passa a guardar as opções
        somente_documento(true, &mut s, &mut doc);
        assert!(documento_sobrepoe() && !doc.config.is_empty());
        s.dim_decimals = 6;
        sincronizar(&mut s, &mut doc);
        assert_eq!(doc.config["dim_decimals"], 6, "o ajuste foi para o arquivo do documento");
        // o config.json do usuário não leva o valor do documento
        let usuario: Value = serde_json::from_str(&texto_do_usuario(&s)).unwrap();
        assert!(usuario.get("dim_decimals").is_none(), "{usuario}");
        // o JSON do documento carrega as opções e reabre igual
        let reaberto = Document::from_json(&doc.to_json()).unwrap();
        assert_eq!(reaberto.config["dim_decimals"], 6);
        // outro documento sem opções próprias: voltam as do usuário
        let mut outro = Document::new("b");
        sincronizar(&mut s, &mut outro);
        assert_eq!(s.dim_decimals, Settings::default().dim_decimals);
        assert!(!documento_sobrepoe());
        // de volta ao primeiro: ele sobrepõe de novo
        sincronizar(&mut s, &mut doc.clone());
        assert_eq!(s.dim_decimals, 6);
        assert!(documento_sobrepoe());
        // solta
        let mut d2 = reaberto;
        somente_documento(false, &mut s, &mut d2);
        assert!(d2.config.is_empty());
        assert_eq!(s.dim_decimals, Settings::default().dim_decimals);
    }
}
