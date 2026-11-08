// SPDX-License-Identifier: LGPL-2.1-or-later
//! Texto da árvore e dos selos que não pode virar ruído (B429, B430).

use std::path::Path;

/// Nome do arquivo, sem a pasta. O caminho inteiro fica na dica de quem mostra o nome.
pub fn nome_do_arquivo(p: &Path) -> String {
    p.file_name().map_or_else(|| p.display().to_string(), |n| n.to_string_lossy().into_owned())
}

/// O selo de estado se desenha sobre a folha? Só quando a vista não está atualizada: "Atualizada" é o
/// normal e, em cada vista, cobria a vizinha e o desenho.
pub fn selo_na_folha(atualizada: bool) -> bool {
    !atualizada
}

/// Rótulo da peça na árvore da montagem: o código do componente e, ao lado, o que a peça é (B429). O
/// nome do documento costuma começar pelo próprio código ("CH-08 Suporte do mancal"): o código não se
/// repete. Instância numerada ("CH-08:2") leva o mesmo nome da peça.
pub fn rotulo_componente(codigo: &str, nome: &str) -> String {
    let (cod, n) = (codigo.trim(), nome.trim());
    let base = cod.split(':').next().unwrap_or(cod);
    let resto = match n.get(..base.len()) {
        Some(ini) if ini.eq_ignore_ascii_case(base) && !n[base.len()..].starts_with(|c: char| c.is_alphanumeric()) => n[base.len()..].trim_start_matches([' ', '-', '\u{2013}', '\u{b7}', '_']).trim(),
        _ => n,
    };
    if resto.is_empty() || resto.eq_ignore_ascii_case(cod) {
        cod.to_string()
    } else {
        format!("{cod} {resto}")
    }
}

/// Nome do tipo de comando para o cabeçalho do painel. O núcleo guarda o tipo como identificador
/// colado ("AbaEntrePerfis"), que também é a base do nome padrão do comando ("AbaEntrePerfis1"); na
/// interface o cabeçalho mostrava esse identificador cru, sem espaço e sem tradução. Aqui ele vira o
/// nome legível que a barra de comandos usa e que tem versão em inglês. Tipo que já é texto
/// ("Extrusão", "Dobra por linha") passa como está.
pub fn tipo_legivel(t: &str) -> &str {
    match t {
        "MultiSeções" => "Multi-seções",
        "MultiSeçõesRemovida" => "Multi-seções removida",
        "PadrãoLinear" => "Padrão linear",
        "PadrãoCircular" => "Padrão circular",
        "PadrãoPontos" => "Padrão por pontos",
        "PreencherFronteiras" => "Preencher fronteiras",
        "CorpoLink" => "Corpo ligado",
        "ParedeBase" => "Parede base",
        "FuroFlangeado" => "Furo flangeado",
        "RecorteChapa" => "Recorte na chapa",
        "AbaPerfil" => "Aba por perfil",
        "AbaCurva" => "Aba curva",
        "AbaLaço" => "Aba por laço",
        "AlívioCanto" => "Alívio de canto",
        "FechaCanto" => "Fechamento de canto",
        "AbaEntrePerfis" => "Aba entre perfis",
        "RedobrarParcial" => "Redobrar parcial",
        "DesdobrarParcial" => "Desdobrar parcial",
        "ConverterEmChapa" => "Converter em chapa",
        "RasgoCircular" => "Rasgo circular",
        "AbaSuperfície" => "Aba por superfície",
        "Intersecao" => "Interseção",
        outro => outro,
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn arquivo_sem_pasta_e_selo_so_quando_ha_aviso() {
        assert_eq!(nome_do_arquivo(Path::new("C:/Users/x/proj/DW-QUADRO.fkd")), "DW-QUADRO.fkd");
        assert_eq!(nome_do_arquivo(Path::new("sem_pasta.fkd")), "sem_pasta.fkd");
        assert!(!selo_na_folha(true) && selo_na_folha(false));
    }

    #[test]
    fn nome_ao_lado_do_codigo_sem_repetir() {
        assert_eq!(rotulo_componente("CH-08", "Suporte do motor"), "CH-08 Suporte do motor");
        assert_eq!(rotulo_componente("CH-08", "ch-08"), "CH-08");
        assert_eq!(rotulo_componente("CH-08", "  "), "CH-08");
        // o nome do documento já começa pelo código
        assert_eq!(rotulo_componente("CH-08", "CH-08 Suporte do mancal"), "CH-08 Suporte do mancal");
        assert_eq!(rotulo_componente("CH-08:2", "CH-08 Suporte do mancal"), "CH-08:2 Suporte do mancal");
        assert_eq!(rotulo_componente("CH-08", "CH-08"), "CH-08");
        assert_eq!(rotulo_componente("QUADRO", "Quadro soldado"), "QUADRO soldado");
        // só vale como código inteiro: "CH-1" não é o começo de "CH-10"
        assert_eq!(rotulo_componente("CH-1", "CH-10 Suporte"), "CH-1 CH-10 Suporte");
    }
    #[test]
    fn tipo_do_comando_aparece_legivel_e_traduzido() {
        // identificadores colados que o cabeçalho do painel de chapa mostrava crus
        let colados = ["MultiSeções", "MultiSeçõesRemovida", "PadrãoLinear", "PadrãoCircular", "PadrãoPontos", "PreencherFronteiras", "CorpoLink", "ParedeBase", "FuroFlangeado", "RecorteChapa", "AbaPerfil", "AbaCurva", "AbaLaço", "AlívioCanto", "FechaCanto", "AbaEntrePerfis", "RedobrarParcial", "DesdobrarParcial", "ConverterEmChapa", "RasgoCircular", "AbaSuperfície", "Intersecao"];
        for c in colados {
            let l = tipo_legivel(c);
            assert!(l.contains(' ') || l.contains('-') || l == "Interseção", "{c} continua colado: {l}");
            assert!(crate::i18n::tem_traducao(l), "{l} sem tradução");
        }
        assert_eq!(tipo_legivel("AbaEntrePerfis"), "Aba entre perfis");
        assert_eq!(tipo_legivel("Extrusão"), "Extrusão");
    }
}
