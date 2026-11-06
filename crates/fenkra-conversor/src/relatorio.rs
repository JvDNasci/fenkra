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
            // nesta frase o argumento é um termo do conversor, e não um nome do arquivo
            let a = if self.modelo == FORA_MALHA_ITENS { tr(a) } else { a };
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
pub(crate) const FORA_DXF: &str = "Fora do sketch: {0}.";
pub(crate) const AV_DXF_SEM_UNIDADE: &str = "O DXF não declara a unidade: os números foram lidos como milímetros. Confira a caixa e escolha a unidade do arquivo.";
pub(crate) const AV_DXF_JUNTADOS: &str = "{0} pontas a menos de {1} mm viraram o mesmo ponto.";
pub(crate) const FORA_IGES: &str = "Entidades que o leitor de IGES não traduziu: {0}.";
pub(crate) const FORA_MALHA: &str = "Geometria exata: {0} guarda só triângulos; o corpo é facetado, sem cilindros nem arestas exatas.";
pub(crate) const FORA_MALHA_ITENS: &str = "Fora do documento: {0} ({1}).";
pub(crate) const AV_MALHA_SEM_UNIDADE: &str = "O {0} não declara a unidade: os números foram lidos como milímetros. Confira a caixa e escolha a unidade do arquivo.";
pub(crate) const AV_SUPERFICIES: &str = "{0} ficou como superfície de referência: o reparo não chegou a um sólido fechado.";
pub(crate) const AV_DEFEITOS_FORA: &str = "Defeitos demais para listar: {0} de {1} estão na lista.";

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
    FORA_DXF,
    AV_DXF_SEM_UNIDADE,
    AV_DXF_JUNTADOS,
    FORA_IGES,
    FORA_MALHA,
    FORA_MALHA_ITENS,
    AV_MALHA_SEM_UNIDADE,
    AV_SUPERFICIES,
    AV_DEFEITOS_FORA,
];

/// Teto da lista de defeitos: ela vai dentro do documento, com o traço de cada um.
pub(crate) const LIMITE_DEFEITOS: u32 = 200;

/// Fotografia de uma forma, ou da soma das peças, antes ou depois do reparo.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Contagem {
    pub faces: u32,
    pub faces_ruins: u32,
    pub solidos: u32,
    /// cascas que não formam sólido
    pub abertas: u32,
    pub soltas: u32,
    pub arestas_livres: u32,
    /// maior tolerância encontrada, em mm
    pub tolerancia: f64,
}

impl Contagem {
    pub(crate) fn somar(&mut self, o: &Contagem) {
        self.faces += o.faces;
        self.faces_ruins += o.faces_ruins;
        self.solidos += o.solidos;
        self.abertas += o.abertas;
        self.soltas += o.soltas;
        self.arestas_livres += o.arestas_livres;
        self.tolerancia = self.tolerancia.max(o.tolerancia);
    }

    pub fn tem_defeito(&self) -> bool {
        self.faces_ruins > 0 || self.abertas > 0 || self.soltas > 0
    }
}

/// Antes e depois de uma peça que tinha defeito, ou que ainda tem.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LinhaReparo {
    pub peca: String,
    pub antes: Contagem,
    pub depois: Contagem,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TipoDefeito {
    FaceInvalida,
    ArestaLivre,
    FaceSolta,
}

impl TipoDefeito {
    pub fn nome(self) -> &'static str {
        match self {
            TipoDefeito::FaceInvalida => "Face inválida",
            TipoDefeito::ArestaLivre => "Aresta livre",
            TipoDefeito::FaceSolta => "Face solta",
        }
    }
}

/// Defeito que sobrou na peça, com o traço para a tela realçar.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Defeito {
    pub tipo: TipoDefeito,
    pub peca: String,
    /// índice da face ou da aresta na peça
    pub indice: u32,
    /// segmentos em mm, no lugar da primeira ocorrência da peça
    pub segmentos: Vec<[[f32; 3]; 2]>,
}

impl Defeito {
    /// Ponto médio do traço: é o que o relatório escreve para achar o defeito sem a tela.
    pub fn centro(&self) -> [f32; 3] {
        let n = (self.segmentos.len() * 2).max(1) as f32;
        let soma = self.segmentos.iter().flatten().fold([0.; 3], |a: [f32; 3], p| [a[0] + p[0], a[1] + p[1], a[2] + p[2]]);
        soma.map(|v| v / n)
    }
}

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
    /// soma das peças antes e depois do reparo
    #[serde(default)]
    pub antes: Contagem,
    #[serde(default)]
    pub depois: Contagem,
    /// só as peças que tinham defeito ou continuam com ele
    #[serde(default)]
    pub reparo_pecas: Vec<LinhaReparo>,
    /// o que sobrou depois do reparo, até o limite da lista
    #[serde(default)]
    pub defeitos: Vec<Defeito>,
    /// DXF: cada camada lida e quantas curvas entraram
    #[serde(default)]
    pub camadas: Vec<(String, usize)>,
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
        s.push_str(&format!("\n{}:\n", tr("Antes e depois do reparo")));
        for (rotulo, antes, depois) in self.linhas_antes_depois() {
            s.push_str(&format!("- {}: {antes} -> {depois}\n", tr(rotulo)));
        }
        let lista = |titulo: &'a str, itens: Vec<String>| {
            if itens.is_empty() {
                return String::new();
            }
            let corpo: String = itens.iter().map(|i| format!("- {i}\n")).collect();
            format!("\n{}:\n{corpo}", tr(titulo))
        };
        s.push_str(&lista("Camadas lidas", self.camadas.iter().map(|(c, n)| format!("{c}: {n}")).collect()));
        s.push_str(&lista("Sólidos que não fecharam", self.nao_fecharam.clone()));
        let defeitos = self.defeitos.iter().map(|d| {
            let c = d.centro();
            format!("{}: {} ({:.3}, {:.3}, {:.3})", d.peca, tr(d.tipo.nome()), c[0], c[1], c[2])
        });
        s.push_str(&lista("Defeitos", defeitos.collect()));
        s.push_str(&lista("O que o formato não carrega", self.nao_carrega.iter().map(|n| n.texto(tr)).collect()));
        s.push_str(&lista("Avisos", self.avisos.iter().map(|n| n.texto(tr)).collect()));
        s
    }

    /// Texto em português, para o cofre do projeto.
    pub fn texto_pt(&self) -> String {
        self.texto(&|s| s)
    }

    /// JSON legível, para outro programa ler. É o mesmo dado que vai embutido no documento.
    pub fn json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    pub fn de_json(texto: &str) -> Option<Relatorio> {
        serde_json::from_str(texto).ok()
    }

    /// Página sozinha, sem nada de fora, para abrir em qualquer navegador ou mandar para alguém.
    pub fn html<'a>(&'a self, tr: &dyn Fn(&'a str) -> &'a str) -> String {
        let mut s = String::from("<!doctype html>\n<html><head><meta charset=\"utf-8\">\n");
        s.push_str(&format!("<title>{}</title>\n<style>{ESTILO}</style></head><body>\n", escapar(&self.arquivo)));
        s.push_str(&format!("<h1>{}</h1>\n<p class=\"sub\">{}</p>\n", escapar(tr("Relatório da importação")), escapar(&self.arquivo)));
        let mut resumo = String::new();
        let mut linha = |rotulo: &'a str, valor: String| resumo.push_str(&format!("<tr><th>{}</th><td>{}</td></tr>\n", escapar(tr(rotulo)), escapar(&valor)));
        linha("Hash", self.hash.clone());
        linha("Tamanho", format!("{} bytes", self.tamanho));
        linha("Formato", format!("{} ({})", self.formato, self.provedor));
        linha("Esquema", self.esquema.clone());
        linha("Unidades lidas", self.unidades.clone());
        linha("Convertido por", self.versao.clone());
        linha("Produtos", self.produtos.to_string());
        linha("Instâncias", self.instancias.to_string());
        linha("Corpos", self.corpos.to_string());
        linha("Volume", format!("{:.3} mm³", self.volume));
        linha("Área", format!("{:.3} mm²", self.area));
        if let Some(m) = self.medidas() {
            linha("Caixa envolvente", format!("{:.3} x {:.3} x {:.3} mm", m[0], m[1], m[2]));
        }
        linha("Reparo", tr(&self.reparo).to_string());
        s.push_str(&format!("<table>\n{resumo}</table>\n"));

        s.push_str(&format!("<h2>{}</h2>\n<table class=\"grade\">\n", escapar(tr("Antes e depois do reparo"))));
        s.push_str(&format!("<tr><th>{}</th><th>{}</th><th>{}</th></tr>\n", escapar(tr("Peça")), escapar(tr("Antes")), escapar(tr("Depois"))));
        for (rotulo, antes, depois) in self.linhas_antes_depois() {
            s.push_str(&format!("<tr><td>{}</td><td>{antes}</td><td>{depois}</td></tr>\n", escapar(tr(rotulo))));
        }
        s.push_str("</table>\n");
        for l in &self.reparo_pecas {
            s.push_str(&format!("<h3>{}</h3>\n<table class=\"grade\">\n", escapar(&l.peca)));
            for (rotulo, antes, depois) in linhas_de(&l.antes, &l.depois) {
                s.push_str(&format!("<tr><td>{}</td><td>{antes}</td><td>{depois}</td></tr>\n", escapar(tr(rotulo))));
            }
            s.push_str("</table>\n");
        }
        if !self.defeitos.is_empty() {
            s.push_str(&format!("<h2>{}</h2>\n<table class=\"grade\">\n", escapar(tr("Defeitos"))));
            s.push_str(&format!("<tr><th>{}</th><th>{}</th><th>{}</th></tr>\n", escapar(tr("Tipo")), escapar(tr("Peça")), escapar(tr("Posição"))));
            for d in &self.defeitos {
                let c = d.centro();
                s.push_str(&format!("<tr><td>{}</td><td>{}</td><td>{:.3}, {:.3}, {:.3}</td></tr>\n", escapar(tr(d.tipo.nome())), escapar(&d.peca), c[0], c[1], c[2]));
            }
            s.push_str("</table>\n");
        }
        let lista = |titulo: &'a str, itens: Vec<String>| {
            if itens.is_empty() {
                return String::new();
            }
            let corpo: String = itens.iter().map(|i| format!("<li>{}</li>\n", escapar(i))).collect();
            format!("<h2>{}</h2>\n<ul>\n{corpo}</ul>\n", escapar(tr(titulo)))
        };
        s.push_str(&lista("Camadas lidas", self.camadas.iter().map(|(c, n)| format!("{c}: {n}")).collect()));
        s.push_str(&lista("Sólidos que não fecharam", self.nao_fecharam.clone()));
        s.push_str(&lista("O que o formato não carrega", self.nao_carrega.iter().map(|n| n.texto(tr)).collect()));
        s.push_str(&lista("Avisos", self.avisos.iter().map(|n| n.texto(tr)).collect()));
        s.push_str("</body></html>\n");
        s
    }

    /// Rótulo, antes e depois da soma das peças, na ordem em que a tela e o HTML mostram.
    pub fn linhas_antes_depois(&self) -> Vec<(&'static str, String, String)> {
        linhas_de(&self.antes, &self.depois)
    }
}

fn linhas_de(a: &Contagem, d: &Contagem) -> Vec<(&'static str, String, String)> {
    let n = |rotulo, x: u32, y: u32| (rotulo, x.to_string(), y.to_string());
    vec![
        n("Faces", a.faces, d.faces),
        n("Faces inválidas", a.faces_ruins, d.faces_ruins),
        n("Sólidos", a.solidos, d.solidos),
        n("Cascas abertas", a.abertas, d.abertas),
        n("Faces soltas", a.soltas, d.soltas),
        n("Arestas livres", a.arestas_livres, d.arestas_livres),
        ("Maior tolerância", format!("{:.7} mm", a.tolerancia), format!("{:.7} mm", d.tolerancia)),
    ]
}

const ESTILO: &str = "body{font-family:system-ui,sans-serif;margin:2rem auto;max-width:52rem;padding:0 1rem;color:#222;background:#fff;line-height:1.45}\
h1{font-size:1.5rem;margin-bottom:0}h2{font-size:1.1rem;margin-top:1.8rem}h3{font-size:1rem}.sub{color:#666;margin-top:.2rem}\
table{border-collapse:collapse;width:100%}th,td{text-align:left;padding:.25rem .6rem;border-bottom:1px solid #ddd;vertical-align:top;overflow-wrap:anywhere}\
th{font-weight:600;color:#444}@media(prefers-color-scheme:dark){body{background:#1b1d21;color:#e4e4e4}th{color:#bbb}.sub{color:#999}th,td{border-color:#3a3d44}}";

/// Texto do arquivo vai para o HTML: nome de peça é dado de fora e não pode virar marcação.
fn escapar(t: &str) -> String {
    t.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
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

    fn com_defeito() -> Relatorio {
        Relatorio {
            arquivo: "<casca> & cia.step".into(),
            antes: Contagem { faces: 6, soltas: 6, ..Default::default() },
            depois: Contagem { faces: 6, solidos: 1, ..Default::default() },
            reparo_pecas: vec![LinhaReparo { peca: "Tampa".into(), ..Default::default() }],
            defeitos: vec![Defeito { tipo: TipoDefeito::ArestaLivre, peca: "Tampa".into(), indice: 3, segmentos: vec![[[0., 0., 0.], [2., 4., 6.]]] }],
            ..Default::default()
        }
    }

    #[test]
    fn html_e_sozinho_escapa_o_nome_e_traz_antes_depois_e_defeitos() {
        let h = com_defeito().html(&|s| s);
        assert!(h.starts_with("<!doctype html>") && h.trim_end().ends_with("</html>"));
        assert!(h.contains("&lt;casca&gt; &amp; cia.step") && !h.contains("<casca>"), "{h}");
        assert!(h.contains("<td>Faces soltas</td><td>6</td><td>0</td>"), "{h}");
        assert!(h.contains("<td>Aresta livre</td><td>Tampa</td><td>1.000, 2.000, 3.000</td>"), "{h}");
        assert!(!h.contains("http"), "a página não depende de nada de fora");
        let traduzida = com_defeito().html(&|s| if s == "Defeitos" { "Defects" } else { s });
        assert!(traduzida.contains("<h2>Defects</h2>"));
    }

    #[test]
    fn json_volta_igual_e_texto_mostra_antes_e_depois() {
        let r = com_defeito();
        assert_eq!(Relatorio::de_json(&r.json()), Some(r.clone()));
        assert_eq!(Relatorio::de_json("nada"), None);
        let t = r.texto_pt();
        assert!(t.contains("- Faces soltas: 6 -> 0") && t.contains("Tampa: Aresta livre (1.000, 2.000, 3.000)"), "{t}");
    }

    #[test]
    fn relatorio_antigo_sem_os_campos_novos_ainda_le() {
        let mut v = serde_json::to_value(Relatorio::default()).unwrap();
        for campo in ["antes", "depois", "reparo_pecas", "defeitos", "camadas"] {
            v.as_object_mut().unwrap().remove(campo);
        }
        assert_eq!(serde_json::from_value::<Relatorio>(v).unwrap(), Relatorio::default());
    }

    #[test]
    fn relatorio_volta_igual_do_json() {
        let r = Relatorio { avisos: vec![Nota::nova(AV_EXTENSAO, &["STEP".into()])], ..Default::default() };
        assert_eq!(serde_json::from_str::<Relatorio>(&serde_json::to_string(&r).unwrap()).unwrap(), r);
    }
}
