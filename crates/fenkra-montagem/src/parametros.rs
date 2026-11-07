// SPDX-License-Identifier: LGPL-2.1-or-later
//! Parâmetros da montagem que vêm de um parâmetro publicado por uma peça (B409).
//!
//! `Assembly.params` é próprio do `.fka`: uma cota de restrição escrita com ele não enxerga o mestre,
//! e a dianteira do kart não andava com o entre-eixos. Aqui um parâmetro da montagem passa a copiar
//! o valor publicado por uma peça, como `add_link` faz dentro de uma peça. A fórmula da restrição
//! não muda: ela continua lendo um parâmetro comum, e quem o mantém em dia é [`atualizar`].
//!
//! O vínculo fica em [`Assembly::params_ligados`], pelo nome do parâmetro.

use crate::load::{full_path, PartCache};
use crate::*;
use fenkra_core::LinkedGeo;
use std::path::Path;

/// Parâmetro `nome` da montagem, copiado da publicação `publicacao` da peça `arquivo`.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct ParamLigado {
    pub nome: String,
    /// como no nó do componente: relativo à pasta da montagem
    pub arquivo: String,
    pub publicacao: String,
}

/// O que uma atualização fez.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RelatorioParametros {
    pub mudaram: Vec<String>,
    /// a peça ou a publicação sumiu: o parâmetro fica com o último valor
    pub pendurados: Vec<String>,
}

/// Valor publicado, se a publicação existe e é um número. Devolve também o nome de agora dela.
fn valor_publicado(arquivo: &Path, publicacao: &str, cache: &mut PartCache) -> Option<(String, f64)> {
    let p = cache.get(arquivo, false).ok()?;
    let atual = crate::publicacoes::nome_atual(&p.doc, publicacao).to_string();
    crate::publicacoes::com_apelidos(&p.doc).into_iter().find(|(n, _)| n == publicacao).and_then(|(_, g)| match g {
        LinkedGeo::Value(v) => Some((atual, v)),
        _ => None,
    })
}

/// Copia para a montagem o valor de cada parâmetro ligado.
pub fn atualizar(a: &mut Assembly, dir: Option<&Path>, cache: &mut PartCache) -> RelatorioParametros {
    let mut rel = RelatorioParametros::default();
    for k in 0..a.params_ligados.len() {
        let l = a.params_ligados[k].clone();
        let Some((atual, v)) = valor_publicado(&full_path(dir, &l.arquivo), &l.publicacao, cache) else {
            rel.pendurados.push(l.nome);
            continue;
        };
        a.params_ligados[k].publicacao = atual;
        let texto = format!("{v}");
        if let Some(p) = a.params.iter_mut().find(|p| p.name == l.nome).filter(|p| p.expr != texto) {
            p.expr = texto;
            rel.mudaram.push(l.nome);
        }
    }
    rel
}

/// Valores publicados pelas peças da montagem: (arquivo como no nó, publicação, valor).
pub fn disponiveis(a: &Assembly, dir: Option<&Path>, cache: &mut PartCache) -> Vec<(String, String, f64)> {
    let mut arquivos: Vec<&String> = a.nodes.iter().filter_map(|n| if let NodeKind::Part { path } = &n.kind { Some(path) } else { None }).collect();
    arquivos.sort();
    arquivos.dedup();
    let mut lista = vec![];
    for arquivo in arquivos {
        let Ok(p) = cache.get(&full_path(dir, arquivo), false) else { continue };
        for (nome, g) in fenkra_solidos::publications(&p.doc) {
            if let LinkedGeo::Value(v) = g {
                lista.push((arquivo.clone(), nome, v));
            }
        }
    }
    lista
}

impl Assembly {
    pub fn parametro_ligado(&self, nome: &str) -> Option<&ParamLigado> {
        self.params_ligados.iter().find(|l| l.nome == nome)
    }

    /// Cria um parâmetro com o valor publicado e o liga à origem. Devolve o nome dele.
    pub fn importar_parametro(&mut self, arquivo: &str, publicacao: &str, valor: f64) -> String {
        let livre = |a: &Assembly, n: &str| !a.params.iter().any(|p| p.name == n);
        let mut nome = publicacao.to_string();
        let mut k = 2;
        while !livre(self, &nome) {
            nome = format!("{publicacao}_{k}");
            k += 1;
        }
        self.params.push(AsmParam { name: nome.clone(), expr: format!("{valor}") });
        self.params_ligados.push(ParamLigado { nome: nome.clone(), arquivo: arquivo.into(), publicacao: publicacao.into() });
        nome
    }

    /// Solta o parâmetro: o valor fica, e a pessoa volta a poder escrever nele.
    pub fn soltar_parametro(&mut self, nome: &str) {
        self.params_ligados.retain(|l| l.nome != nome);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::links::{self, Gatilho};
    use fenkra_core::{Document, Parameter, Unit};
    use std::path::PathBuf;

    fn pasta(nome: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("fenkra_parametros_{nome}"));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn mestre(dir: &Path, ee: f64, publicado: bool) {
        let mut d = Document::new("Mestre");
        let id = d.alloc();
        d.params.push(Parameter { published: publicado, ..Parameter::novo(id, "EE", &ee.to_string(), Unit::Comprimento) });
        std::fs::write(dir.join("mestre.fkp"), d.to_json()).unwrap();
    }

    /// Mestre e eixo, com o eixo à distância `EE / 2` da base. Devolve a montagem e o eixo.
    fn conjunto() -> (Assembly, Id) {
        let mut a = Assembly::new("Chassi");
        let m = a.add_component("Mestre", NodeKind::Part { path: "mestre.fkp".into() }, Placement::default(), None);
        a.node_mut(m).unwrap().fixed = true;
        let eixo = a.add_component("Eixo", NodeKind::Part { path: "eixo.fkp".into() }, Placement { origin: [0., 0., 40.], ..Default::default() }, None);
        a.importar_parametro("mestre.fkp", "EE", 100.);
        let plano = Geom::Plane { p: [0.; 3], n: [0., 0., 1.] };
        a.add_mate(MateKind::Distance("EE / 2".into()), MateRef { comp: m, geom: plano, pick: None, viva: None }, MateRef { comp: eixo, geom: plano, pick: None, viva: None });
        assert!(crate::solve::solve(&mut a).converged);
        (a, eixo)
    }

    fn altura(a: &Assembly, comp: Id) -> f64 {
        a.node(comp).unwrap().placement.origin[2].abs()
    }

    #[test]
    fn a_montagem_acompanha_o_parametro_do_mestre() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let dir = pasta("acompanha");
        mestre(&dir, 100., true);
        let (mut a, eixo) = conjunto();
        assert!((altura(&a, eixo) - 50.).abs() < 1e-6, "{}", altura(&a, eixo));

        // sem mudança no mestre, nada a fazer
        let mut cache = PartCache::default();
        assert_eq!(atualizar(&mut a, Some(&dir), &mut cache), RelatorioParametros::default());

        mestre(&dir, 160., true);
        let rel = atualizar(&mut a, Some(&dir), &mut PartCache::default());
        assert_eq!(rel.mudaram, ["EE"]);
        assert!(crate::solve::solve(&mut a).converged);
        assert!((altura(&a, eixo) - 80.).abs() < 1e-6, "{}", altura(&a, eixo));

        // o vínculo vai no arquivo da montagem
        let lida = Assembly::from_json(&a.to_json()).unwrap();
        assert_eq!(lida.parametro_ligado("EE").unwrap().publicacao, "EE");
    }

    #[test]
    fn subconjunto_acompanha_o_mestre_pela_montagem_de_topo() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let dir = pasta("sub");
        mestre(&dir, 100., true);
        let (sub, eixo) = conjunto();
        std::fs::write(dir.join("sub.fka"), sub.to_json()).unwrap();
        let mut topo = Assembly::new("Topo");
        topo.add_component("Sub", NodeKind::SubAssembly { path: "sub.fka".into() }, Placement::default(), None);

        mestre(&dir, 160., true);
        let rel = links::atualizar(&topo, Some(&dir), &mut PartCache::default(), Gatilho::Manual);
        assert_eq!(rel.subconjuntos, vec![dir.join("sub.fka")], "{rel:?}");
        let lido = Assembly::from_json(&std::fs::read_to_string(dir.join("sub.fka")).unwrap()).unwrap();
        assert!((altura(&lido, eixo) - 80.).abs() < 1e-6, "{}", altura(&lido, eixo));
    }

    #[test]
    fn publicacao_que_some_mantem_o_ultimo_valor() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let dir = pasta("sumiu");
        mestre(&dir, 100., false);
        let (mut a, _) = conjunto();
        let rel = atualizar(&mut a, Some(&dir), &mut PartCache::default());
        assert_eq!(rel.pendurados, ["EE"]);
        assert_eq!(a.params[0].expr, "100");
    }

    #[test]
    fn importar_nao_repete_nome_e_soltar_guarda_o_valor() {
        let mut a = Assembly::new("A");
        a.params.push(AsmParam { name: "EE".into(), expr: "1".into() });
        assert_eq!(a.importar_parametro("m.fkp", "EE", 7.), "EE_2");
        assert!(a.parametro_ligado("EE").is_none());
        a.soltar_parametro("EE_2");
        assert!(a.parametro_ligado("EE_2").is_none());
        assert_eq!(a.params[1].expr, "7");
    }

    #[test]
    fn lista_so_os_valores_publicados_das_pecas() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let dir = pasta("lista");
        mestre(&dir, 100., true);
        let (a, _) = conjunto();
        let lista = disponiveis(&a, Some(&dir), &mut PartCache::default());
        assert_eq!(lista, vec![("mestre.fkp".to_string(), "EE".to_string(), 100.)]);
    }
}
