// SPDX-License-Identifier: LGPL-2.1-or-later
//! Relatório de conversão. É dado, para sobreviver ao processo filho e ir para o cofre; vira texto só
//! na hora de mostrar, e por isso as frases carregam modelo e argumentos em vez de texto pronto.

use serde::{Deserialize, Serialize};

/// Frase com lacunas `{0}`, `{1}`... A interface traduz o modelo e depois preenche.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Nota {
    pub modelo: String,
    pub args: Vec<String>,
}

impl Nota {
    pub(crate) fn nova(modelo: &'static str, args: &[String]) -> Nota {
        Nota { modelo: modelo.into(), args: args.to_vec() }
    }

    pub fn texto<'a>(&'a self, tr: &dyn Fn(&'a str) -> &'a str) -> String {
        let mut t = tr(&self.modelo).to_string();
        for (i, a) in self.args.iter().enumerate() {
            t = t.replace(&format!("{{{i}}}"), a);
        }
        t
    }
}

pub(crate) const AV_EXTENSAO: &str = "A extensão do arquivo não combina com o conteúdo, que é {0}.";
pub(crate) const AV_GRANDE: &str = "Peça com {0} mm: a unidade do arquivo pode estar errada.";
pub(crate) const AV_PEQUENA: &str = "Peça com {0} mm: a unidade do arquivo pode estar errada.";
pub(crate) const AV_ESCALA: &str = "Unidade trocada na importação: tudo multiplicado por {0}.";
pub(crate) const AV_CORES_REPARO: &str = "O reparo mudou as faces de {0}: as cores por face dessa peça foram perdidas.";
pub(crate) const AV_COMANDO: &str = "O comando {0} não calculou: {1}";
pub(crate) const AV_REANCORADAS: &str = "Referências ajustadas pela geometria: {0}.";
pub(crate) const FORA_ARVORE: &str = "Ficou de fora quando o arquivo foi gravado: {0}";
pub(crate) const FORA_BREP: &str = "Nomes, cores, camadas e estrutura: o BREP guarda só a forma.";
pub(crate) const FORA_BREP_UNIDADE: &str = "Unidade: o BREP não declara; os números foram lidos como milímetros.";
pub(crate) const FORA_AP203: &str = "Cores e camadas: o AP203 não prevê; arquivos desse esquema costumam vir sem elas.";
pub(crate) const FORA_PMI: &str = "PMI: {0} dimensões, {1} tolerâncias e {2} datums ficaram fora; o documento ainda não recebe PMI do STEP.";
pub(crate) const FORA_COR_FACE: &str = "Cor por face: {0} faces; o documento guarda uma cor por corpo.";
pub(crate) const FORA_CAMADAS: &str = "Camadas ({0}): o documento não tem camadas.";
pub(crate) const FORA_MONTAGEM: &str = "Estrutura de montagem: as peças viraram corpos de um documento, com o caminho no nome.";

/// Todos os modelos de frase, para a tradução da interface ser conferida.
pub const NOTAS: &[&str] = &[
    AV_EXTENSAO,
    AV_GRANDE,
    AV_PEQUENA,
    AV_ESCALA,
    AV_CORES_REPARO,
    AV_COMANDO,
    AV_REANCORADAS,
    FORA_ARVORE,
    FORA_BREP,
    FORA_BREP_UNIDADE,
    FORA_AP203,
    FORA_PMI,
    FORA_COR_FACE,
    FORA_CAMADAS,
    FORA_MONTAGEM,
];

/// O que aconteceu numa importação.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Relatorio {
    pub arquivo: String,
    pub hash: String,
    pub tamanho: u64,
    pub formato: String,
    pub provedor: String,
    /// versões do Fenkra e do kernel que converteram
    pub versao: String,
    pub esquema: String,
    pub unidades: String,
    /// fator aplicado quando a pessoa trocou a unidade
    pub escala: f64,
    pub produtos: usize,
    pub instancias: usize,
    pub corpos: usize,
    pub faces: u32,
    pub solidos: u32,
    pub volume: f64,
    pub area: f64,
    /// canto mínimo e máximo, em mm
    pub caixa: Option<([f64; 3], [f64; 3])>,
    pub reparo: String,
    pub faces_reparadas: u32,
    pub faces_costuradas: u32,
    /// faces que continuam inválidas depois do reparo
    pub faces_ruins: u32,
    /// peças com casca aberta ou faces soltas no fim
    pub nao_fecharam: Vec<String>,
    /// o que o arquivo tinha ou podia ter e não chegou ao documento
    pub nao_carrega: Vec<Nota>,
    pub avisos: Vec<Nota>,
}

impl Relatorio {
    /// Medidas da caixa envolvente, em mm.
    pub fn medidas(&self) -> Option<[f64; 3]> {
        self.caixa.map(|(a, b)| [b[0] - a[0], b[1] - a[1], b[2] - a[2]])
    }

    /// Texto para copiar ou guardar. `tr` traduz rótulos e modelos de frase.
    pub fn texto<'a>(&'a self, tr: &dyn Fn(&'a str) -> &'a str) -> String {
        let mut s = String::new();
        let mut linha = |rotulo: &'a str, valor: String| s.push_str(&format!("{}: {valor}\n", tr(rotulo)));
        linha("Arquivo", self.arquivo.clone());
        linha("Hash", self.hash.clone());
        linha("Tamanho", format!("{} bytes", self.tamanho));
        linha("Formato", format!("{} ({})", self.formato, self.provedor));
        linha("Esquema", self.esquema.clone());
        linha("Unidades lidas", self.unidades.clone());
        linha("Convertido por", self.versao.clone());
        linha("Produtos", self.produtos.to_string());
        linha("Instâncias", self.instancias.to_string());
        linha("Corpos", self.corpos.to_string());
        linha("Faces", self.faces.to_string());
        linha("Sólidos", self.solidos.to_string());
        linha("Volume", format!("{:.3} mm³", self.volume));
        linha("Área", format!("{:.3} mm²", self.area));
        if let Some(m) = self.medidas() {
            linha("Caixa envolvente", format!("{:.3} x {:.3} x {:.3} mm", m[0], m[1], m[2]));
        }
        linha("Reparo", tr(&self.reparo).to_string());
        linha("Faces reparadas", self.faces_reparadas.to_string());
        linha("Faces costuradas", self.faces_costuradas.to_string());
        linha("Faces ainda inválidas", self.faces_ruins.to_string());
        let lista = |titulo: &'a str, itens: Vec<String>| {
            if itens.is_empty() {
                return String::new();
            }
            let corpo: String = itens.iter().map(|i| format!("- {i}\n")).collect();
            format!("\n{}:\n{corpo}", tr(titulo))
        };
        s.push_str(&lista("Sólidos que não fecharam", self.nao_fecharam.clone()));
        s.push_str(&lista("O que o formato não carrega", self.nao_carrega.iter().map(|n| n.texto(tr)).collect()));
        s.push_str(&lista("Avisos", self.avisos.iter().map(|n| n.texto(tr)).collect()));
        s
    }

    /// Texto em português, para o cofre do projeto.
    pub fn texto_pt(&self) -> String {
        self.texto(&|s| s)
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn nota_traduz_o_modelo_e_depois_preenche() {
        let n = Nota::nova(FORA_PMI, &["1".into(), "2".into(), "3".into()]);
        assert!(n.texto(&|s| s).starts_with("PMI: 1 dimensões, 2 tolerâncias e 3 datums"));
        assert_eq!(n.texto(&|_| "{2}-{0}"), "3-1");
    }

    #[test]
    fn texto_traz_reparo_o_que_nao_fechou_e_o_que_ficou_fora() {
        let r = Relatorio {
            arquivo: "peça.step".into(),
            faces_reparadas: 3,
            nao_fecharam: vec!["Tampa".into()],
            nao_carrega: vec![Nota::nova(FORA_BREP, &[])],
            caixa: Some(([0.; 3], [1., 2., 3.])),
            ..Default::default()
        };
        let t = r.texto_pt();
        assert!(t.contains("Faces reparadas: 3") && t.contains("- Tampa") && t.contains("1.000 x 2.000 x 3.000 mm"), "{t}");
        assert!(t.contains("O que o formato não carrega:\n- Nomes"), "{t}");
    }

    #[test]
    fn relatorio_volta_igual_do_json() {
        let r = Relatorio { avisos: vec![Nota::nova(AV_EXTENSAO, &["STEP".into()])], ..Default::default() };
        assert_eq!(serde_json::from_str::<Relatorio>(&serde_json::to_string(&r).unwrap()).unwrap(), r);
    }
}
