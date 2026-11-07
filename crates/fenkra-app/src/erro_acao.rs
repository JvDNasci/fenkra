//! Erros acionáveis (B419): item, campo, causa e sugestão, com um código que não muda.
//!
//! O kernel e os comandos devolvem a falha como texto. A pessoa precisa de mais: qual item quebrou,
//! em que campo, por quê, o que fazer, e o botão certo para fazer. Este módulo junta a causa
//! estrutural (`ErrorKind`, decidida pelo documento) com o que o texto da falha diz (cordão de
//! solda, cadeia de superfície, valor inválido, contorno aberto) e devolve um [`Acionavel`].

use fenkra_core::Id;
use fenkra_solidos::erro::ErrorKind;

/// A ação que o Assistente de reparo coloca em primeiro lugar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Acao {
    /// reabrir o comando para escolher a entrada de novo
    RefazerReferencia,
    /// ir ao comando que falhou antes e corrigir aquele primeiro
    CorrigirAntes(Id),
    /// reabrir o comando para mudar um valor
    AjustarValor,
    /// reabrir o sketch ou a seleção de curvas para fechar ou reordenar o contorno
    RefazerContorno,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Acionavel {
    /// código estável, para relatório, teste e automação
    pub codigo: &'static str,
    /// nome do item que quebrou
    pub item: String,
    /// campo citado entre aspas simples na mensagem, se houver
    pub campo: Option<String>,
    /// o que a mensagem de origem diz
    pub causa: String,
    /// o que fazer, em uma frase
    pub sugestao: &'static str,
    pub acao: Acao,
}

/// Primeiro trecho entre aspas simples da mensagem (o kernel cita o campo ou o contorno assim).
fn campo_citado(msg: &str) -> Option<String> {
    let ini = msg.find('\'')? + 1;
    let fim = msg[ini..].find('\'')? + ini;
    let c = msg[ini..fim].trim();
    (!c.is_empty() && c.chars().count() <= 40).then(|| c.to_string())
}

/// Monta o diagnóstico de um item que falhou. `tipo` é a causa estrutural (se o documento a deu).
pub fn diagnosticar(tipo: Option<ErrorKind>, item: &str, msg: &str) -> Acionavel {
    let m = msg.to_lowercase();
    let (codigo, sugestao, acao) = match tipo {
        Some(ErrorKind::ReferenciaPerdida { .. }) | Some(ErrorKind::GeometriaPerdida) => (
            "E-REF-001",
            "A face, aresta ou item que o comando usava não existe mais. Refaça a referência escolhendo a nova entrada no 3D.",
            Acao::RefazerReferencia,
        ),
        Some(ErrorKind::PaiFalhou { causa }) => (
            "E-DEP-001",
            "Este comando depende de outro que falhou antes. Corrija aquele primeiro: este volta sozinho.",
            Acao::CorrigirAntes(causa),
        ),
        _ if m.contains("cordão") || m.contains("cordao") || m.contains("solda") => (
            "E-SOL-001",
            "Confira a perna do cordão e se as duas faces se encontram; aumente a perna ou escolha de novo as faces do cordão.",
            Acao::RefazerReferencia,
        ),
        _ if m.contains("cadeia") || m.contains("concordância") || m.contains("bordas se tocam") || m.contains("tangente") => (
            "E-SUP-001",
            "A cadeia de bordas precisa ser contínua, na ordem e sem folga. Reescolha as bordas ou costure a superfície antes.",
            Acao::RefazerContorno,
        ),
        _ if m.contains("aberta") || m.contains("fechado") || m.contains("fechada") || m.contains("contorno") => (
            "E-SKT-001",
            "O contorno precisa estar fechado. Feche a curva no sketch ou junte as curvas com Juntar.",
            Acao::RefazerContorno,
        ),
        _ if m.contains("precisa ser") || m.contains("inválid") || m.contains("maior que") || m.contains("menor que") || m.contains("negativo") || m.contains("preencha") => (
            "E-VAL-001",
            "Um valor do comando está fora do que a geometria aceita. Reabra o comando e ajuste o campo citado.",
            Acao::AjustarValor,
        ),
        _ => (
            "E-GER-001",
            "O comando não conseguiu terminar com estes dados. Reabra, confira as entradas e, se persistir, suprima e siga.",
            Acao::AjustarValor,
        ),
    };
    Acionavel { codigo, item: item.to_string(), campo: campo_citado(msg), causa: msg.to_string(), sugestao, acao }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn referencia_perdida_manda_refazer_a_referencia() {
        let a = diagnosticar(Some(ErrorKind::ReferenciaPerdida { ausente: Id(7) }), "Filete 1", "referência perdida: filete");
        assert_eq!(a.codigo, "E-REF-001");
        assert_eq!(a.acao, Acao::RefazerReferencia);
        assert_eq!(a.item, "Filete 1");
    }

    #[test]
    fn pai_que_falhou_manda_corrigir_o_pai_primeiro() {
        let a = diagnosticar(Some(ErrorKind::PaiFalhou { causa: Id(3) }), "Casca", "qualquer");
        assert_eq!((a.codigo, a.acao), ("E-DEP-001", Acao::CorrigirAntes(Id(3))));
    }

    #[test]
    fn cordao_e_cadeia_de_superficie_tem_codigo_proprio() {
        let c = diagnosticar(Some(ErrorKind::Propria), "Solda", "o cordão de solda saiu sem volume: confira a perna e as faces");
        assert_eq!(c.codigo, "E-SOL-001");
        let s = diagnosticar(Some(ErrorKind::Propria), "Concordância", "as bordas se tocam — a concordância precisa de afastamento");
        assert_eq!((s.codigo, s.acao), ("E-SUP-001", Acao::RefazerContorno));
    }

    #[test]
    fn valor_invalido_cita_o_campo_e_pede_ajuste() {
        let a = diagnosticar(Some(ErrorKind::Propria), "Filete", "o 'raio' precisa ser positivo");
        assert_eq!(a.codigo, "E-VAL-001");
        assert_eq!(a.campo.as_deref(), Some("raio"));
        assert_eq!(a.acao, Acao::AjustarValor);
    }

    #[test]
    fn contorno_aberto_pede_fechar() {
        let a = diagnosticar(None, "Extrusão", "'Perfil' é aberta: o perfil de um sólido precisa de contorno fechado");
        assert_eq!((a.codigo, a.campo.as_deref()), ("E-SKT-001", Some("Perfil")));
    }

    #[test]
    fn texto_desconhecido_cai_no_codigo_geral_com_sugestao() {
        let a = diagnosticar(None, "X", "falha interna do kernel");
        assert_eq!(a.codigo, "E-GER-001");
        assert!(!a.sugestao.is_empty());
    }
}
