//! B64: arquivos de alphas anteriores continuam abrindo e recalculam com o mesmo volume.
//!
//! `fixtures/alpha28_caixa.fkp` é uma peça gravada pelo alpha.28 (sem `app_version`): caixa 40 x 30 x 20.
//! Quando o formato mudar, a fixture antiga fica e ganha um passo de migração em
//! `fenkra_core::formato`; este teste prova que ela ainda abre e dá o mesmo sólido.

mod common;

use common::*;
use fenkra_core::formato::{preparar, Tipo};
use fenkra_core::Document;

const ALPHA28: &str = include_str!("fixtures/alpha28_caixa.fkp");

#[test]
fn fixture_do_alpha28_abre_e_recalcula_com_o_mesmo_volume() {
    if !kernel_disponivel() {
        return;
    }
    assert!(!ALPHA28.contains("app_version"), "a fixture deve ser do formato antigo");
    let d = Document::from_json(ALPHA28).unwrap();
    assert!(!d.formato_futuro());
    confere("caixa alpha28", &d, &Esperado::prisma(40. * 30. * 20., [40., 30., 20.], [-20., -15., 0.]));
}

#[test]
fn salvar_grava_app_version_e_a_igualdade_ignora_a_versao() {
    let d = Document::from_json(ALPHA28).unwrap();
    let json = d.to_json();
    assert!(json.contains("\"app_version\""));
    let volta = Document::from_json(&json).unwrap();
    assert_eq!(volta.app_version.0, fenkra_core::formato::APP_VERSION);
    assert_eq!(volta, d);
}

#[test]
fn versao_futura_abre_tolerante_e_fica_marcada() {
    let mut v: serde_json::Value = serde_json::from_str(ALPHA28).unwrap();
    v["format_version"] = 99.into();
    v["campo_do_futuro"] = serde_json::json!({"x": 1});
    let p = preparar(Tipo::Peca, &v.to_string()).unwrap();
    assert!(p.futuro);
    let d = Document::from_json(&v.to_string()).unwrap();
    assert!(d.formato_futuro());
    assert_eq!(d.format_version, 99, "a versão do arquivo não pode ser reescrita em silêncio");
}
