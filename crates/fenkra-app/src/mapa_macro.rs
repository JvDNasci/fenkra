// SPDX-License-Identifier: LGPL-2.1-or-later
//! Mapa de macroprocessos por bancada (B325): a grade de um curso para cada bancada.
//!
//! Cada etapa diz o que dá para fazer, quais comandos entram e leva à aula do curso (B323), que por
//! sua vez abre a peça de exemplo quando há uma. É a porta de entrada do curso e do modo de
//! compatibilidade (B324). Os dados são texto e ícones: um teste garante que toda bancada tem mapa
//! e que toda etapa aponta para uma aula e para comandos que existem.

use crate::curso::{self, Aula};
use crate::settings::Bench;

/// Uma etapa do processo de modelagem da bancada.
#[derive(Clone, Copy, Debug)]
pub struct Etapa {
    /// aula do curso que ensina a etapa (o título da aula é o título da etapa)
    pub aula: &'static str,
    /// o que dá para fazer nesta etapa
    pub pode: &'static str,
    /// ícones dos comandos envolvidos
    pub comandos: &'static [&'static str],
}

#[derive(Clone, Copy, Debug)]
pub struct Mapa {
    pub bancada: Bench,
    pub resumo: &'static str,
    pub etapas: &'static [Etapa],
}

const fn e(aula: &'static str, pode: &'static str, comandos: &'static [&'static str]) -> Etapa {
    Etapa { aula, pode, comandos }
}

pub const MAPAS: &[Mapa] = &[
    Mapa {
        bancada: Bench::Solidos,
        resumo: "Do desenho 2D ao sólido acabado, sempre guiado por parâmetros.",
        etapas: &[
            e("sol_camera", "Girar, mover e aproximar a vista, olhar de frente pelas vistas padrão e escolher faces.", &["vista_iso", "vista_frente", "vista_topo"]),
            e("sol_esboco", "Desenhar retângulos, círculos e arcos, ligar com restrições e cotar com parâmetros.", &["sketch", "sk_retangulo", "sk_cota"]),
            e("sol_volume", "Extrudar, revolucionar, varrer ou partir de um sólido básico como caixa e cilindro.", &["extrusao", "revolucao", "prim_caixa"]),
            e("sol_furo", "Furar com diâmetro e profundidade, ou tirar material com o perfil de um sketch.", &["furo_simples", "recorte", "furo_roscado"]),
            e("sol_acabamento", "Arredondar e chanfrar arestas, com raio constante ou variável.", &["arestas_selecionar", "chanfro"]),
            e("sol_repetir", "Repetir em fileira ou em círculo e espelhar por um plano.", &["padrao_linear", "padrao_circular", "espelho"]),
            e("sol_casca", "Esvaziar a peça deixando uma parede de espessura constante.", &["casca", "saida"]),
        ],
    },
    Mapa {
        bancada: Bench::Superficies,
        resumo: "Das curvas guia à superfície contínua, e de volta ao sólido.",
        etapas: &[
            e("sup_curvas", "Criar pontos, linhas, splines, hélices e curvas projetadas ou de interseção.", &["spline", "linha_pontos", "helice"]),
            e("sup_criar", "Gerar superfícies por extrusão, revolução, varredura e seções entre curvas.", &["sup_extrudada", "sup_varredura", "sup_multisecoes"]),
            e("sup_juntar", "Juntar, aparar e filetar superfícies, e transformar o conjunto em sólido.", &["op_juntar", "sup_filete", "hib_engrossar", "hib_fechar"]),
            e("sup_verificar", "Conferir continuidade G0, G1 e G2 com zebra e mapa de curvatura.", &["an_zebra", "an_curvatura"]),
        ],
    },
    Mapa {
        bancada: Bench::FormaLivre,
        resumo: "Da gaiola de nós à superfície lisa, e dela ao corpo sólido.",
        etapas: &[
            e("ff_primeira", "Criar uma caixa, esfera, cilindro ou plano de forma arrastando no plano.", &["forma_caixa", "forma_esfera", "forma_cilindro", "forma_plano"]),
            e("ff_moldar", "Puxar os nós da gaiola e alternar entre a gaiola e a superfície lisa.", &["forma_caixa", "forma_plano"]),
            e("ff_vincos", "Dar peso a um vinco para endurecer uma aresta sem perder a superfície lisa.", &["forma_caixa"]),
            e("ff_converter", "Converter a forma em corpo B-rep e seguir com filetes e furos.", &["forma_cilindro", "forma_esfera"]),
        ],
    },
    Mapa {
        bancada: Bench::Montagem,
        resumo: "Inserir, restringir, verificar e documentar um conjunto de peças.",
        etapas: &[
            e("mon_inserir", "Trazer peças e subconjuntos, ou criar uma peça já dentro da montagem.", &["asm_inserir", "asm_peca_nova", "asm_subconjunto"]),
            e("mon_restringir", "Posicionar com coincidência, distância, ângulo, engrenagem e outras restrições.", &["rest_coincidencia", "rest_distancia", "rest_fixar"]),
            e("mon_fixadores", "Preencher furos com parafusos, arruelas e porcas que saem do furo e da espessura.", &["asm_fixadores", "asm_conjunto", "asm_regras_cor"]),
            e("mon_verificar", "Procurar interferências e simular o movimento dos mecanismos.", &["rest_interferencia", "mec_simular"]),
            e("mon_documentar", "Gerar lista de peças e numerar os itens para o desenho.", &["asm_lista", "asm_numerar"]),
        ],
    },
    Mapa {
        bancada: Bench::Chapa,
        resumo: "Da chapa base à peça plana pronta para o corte e para o desenho de fabricação.",
        etapas: &[
            e("ch_base", "Definir espessura, raio e fator K e criar a parede base de um perfil fechado.", &["sm_parametros", "sm_parede"]),
            e("ch_abas", "Dobrar bordas com abas, bainhas, abas por perfil ou em aresta curva.", &["sm_aba", "sm_bainha"]),
            e("ch_recortes", "Recortar a chapa e aliviar os cantos onde as dobras se encontram.", &["sm_recorte", "sm_alivio_canto", "sm_fecha_canto", "sm_rasgo_circular"]),
            e("ch_planificar", "Desdobrar e dobrar de novo para ler o comprimento planificado.", &["sm_desdobrar", "sm_desdobrar_parcial"]),
            e("ch_desenho", "Levar a chapa planificada e a tabela de dobras para a folha de desenho.", &["des_planificada", "des_tab_dobras", "des_tab_furos"]),
        ],
    },
    Mapa {
        bancada: Bench::Estrutural,
        resumo: "Do esqueleto de linhas à lista de corte e à montagem das barras.",
        etapas: &[
            e("est_membros", "Escolher o perfil da biblioteca e aplicá-lo ao longo das linhas do esqueleto.", &["linha_pontos", "st_membro", "st_biblioteca"]),
            e("est_cantos", "Definir como os membros se encontram: meia-esquadria, topo, aparar ou encaixe.", &["st_canto_meia"]),
            e("est_placas", "Fechar pontas com placas e enrijecer os encontros com reforços.", &["st_placa", "st_reforco"]),
            e("est_corte", "Obter a lista de corte e gerar a montagem com um componente por membro.", &["st_corte", "st_montagem"]),
        ],
    },
    Mapa {
        bancada: Bench::Desenho,
        resumo: "Da folha em branco ao desenho exportado, com vistas ligadas ao modelo.",
        etapas: &[
            e("des_folha", "Escolher formato e escala e preencher o carimbo.", &["des_novo", "des_folha"]),
            e("des_vistas", "Inserir vistas principais, cortes, detalhes e a vista isométrica.", &["des_vista", "des_tres_vistas", "des_corte", "des_detalhe"]),
            e("des_cotas", "Cotar, indicar tolerâncias e acrescentar notas e símbolos de acabamento.", &["des_cota_h", "des_cota_d", "des_nota", "des_rugosidade"]),
            e("des_tabelas", "Inserir tabelas de materiais e exportar em PDF, SVG ou DXF.", &["des_bom", "des_pdf", "des_dxf"]),
        ],
    },
];

pub fn mapa(b: Bench) -> &'static Mapa {
    MAPAS.iter().find(|m| m.bancada == b).expect("toda bancada tem mapa")
}

impl Etapa {
    pub fn aula(&self) -> &'static Aula {
        curso::aula(self.aula).expect("etapa aponta para aula existente")
    }

    pub fn titulo(&self) -> &'static str {
        self.aula().titulo
    }

    /// Peça de exemplo da etapa, quando a aula tem uma.
    pub fn exemplo(&self) -> Option<&'static str> {
        self.aula().exemplo
    }
}

/// Nome da bancada, como nas abas.
pub fn nome_da_bancada(b: Bench) -> &'static str {
    match b {
        Bench::Solidos => "Modelagem Sólida",
        Bench::Superficies => "Superfícies",
        Bench::FormaLivre => "Forma livre",
        Bench::Montagem => "Montagem",
        Bench::Chapa => "Chapa metálica",
        Bench::Estrutural => "Estrutural",
        Bench::Desenho => "Desenho",
    }
}

pub const BANCADAS: [Bench; 7] = [Bench::Solidos, Bench::Superficies, Bench::FormaLivre, Bench::Montagem, Bench::Chapa, Bench::Estrutural, Bench::Desenho];

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn toda_bancada_tem_mapa_e_toda_etapa_leva_a_uma_aula_e_a_comandos_que_existem() {
        let icones: Vec<&str> = crate::ajuda_cmd::comandos().iter().map(|i| i.icon).collect();
        for b in BANCADAS {
            let m = mapa(b);
            assert!(m.etapas.len() >= 4, "{b:?}: mapa curto demais");
            for et in m.etapas {
                let a = curso::aula(et.aula).unwrap_or_else(|| panic!("{b:?}: aula {} não existe", et.aula));
                assert_eq!(a.bancada, b, "{b:?}: a aula {} é de outra bancada", a.id);
                assert!(!et.comandos.is_empty(), "{}: etapa sem comando", et.aula);
                for c in et.comandos {
                    assert!(icones.contains(c), "{}: comando {c} não existe na barra", et.aula);
                }
                if let Some(ex) = et.exemplo() {
                    assert!(crate::curso_exemplos::existe(ex), "{}: exemplo {ex}", et.aula);
                }
            }
        }
    }

    #[test]
    fn todas_as_aulas_da_bancada_aparecem_no_mapa_dela() {
        for b in BANCADAS {
            let no_mapa: Vec<&str> = mapa(b).etapas.iter().map(|e| e.aula).collect();
            for a in curso::aulas_da_bancada(b) {
                assert!(no_mapa.contains(&a.id), "{b:?}: a aula {} não tem etapa no mapa", a.id);
            }
        }
    }

    #[test]
    fn os_comandos_de_cada_etapa_ficam_na_bancada_do_mapa() {
        // o comando está em algum grupo de barra da bancada: o botão "abrir comando" faz sentido nela
        for b in BANCADAS {
            for et in mapa(b).etapas {
                for c in et.comandos {
                    assert!(crate::compat::grupo_do_comando(c).is_some(), "{c} não está em nenhum grupo da barra");
                }
            }
        }
    }

    #[test]
    fn textos_do_mapa_tem_traducao() {
        let mut faltam = vec![];
        for b in BANCADAS {
            if !crate::i18n::tem_traducao(mapa(b).resumo) {
                faltam.push(mapa(b).resumo);
            }
            for et in mapa(b).etapas {
                if !crate::i18n::tem_traducao(et.pode) {
                    faltam.push(et.pode);
                }
            }
        }
        assert!(faltam.is_empty(), "{faltam:#?}");
    }
}
