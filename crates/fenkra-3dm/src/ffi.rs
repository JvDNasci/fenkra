// SPDX-License-Identifier: LGPL-2.1-or-later
//! Ponte Rust ⇄ C++ com o openNURBS. Só dado atravessa: nomes, cores, índices e o fluxo de números da
//! B-rep (formato descrito em `lib.rs`). Nenhum objeto do openNURBS sai do C++.

#[cxx::bridge(namespace = "fenkra3dm")]
pub mod bridge {
    struct CamadaBruta {
        nome: String,
        /// 0xRRGGBB
        cor: u32,
        /// índice da camada mãe; -1 = raiz
        pai: i32,
        visivel: bool,
    }

    struct ObjetoBruto {
        nome: String,
        /// índice em `camadas`; -1 = nenhuma
        camada: i32,
        /// 0xRRGGBB, valendo só com `cor_propria`
        cor: u32,
        cor_propria: bool,
        /// 0 = B-rep (em `brep`), 1 = malha (em `vertices` e `triangulos`)
        tipo: u8,
        /// alguma face saiu de superfície sem forma NURBS de mesmo parâmetro: os aparos foram levados
        /// ponto a ponto, não exatos
        aproximado: bool,
        brep: Vec<f64>,
        vertices: Vec<f64>,
        triangulos: Vec<u32>,
    }

    struct ArquivoBruto {
        camadas: Vec<CamadaBruta>,
        objetos: Vec<ObjetoBruto>,
        /// tipo de cada objeto do arquivo que a leitura não traduziu
        ignorados: Vec<String>,
        versao: i32,
        /// tolerância absoluta do modelo, em mm
        tolerancia: f64,
    }

    unsafe extern "C++" {
        include!("ponte.h");

        /// Lê o arquivo com a geometria já em milímetros.
        fn ler_3dm(caminho: &str) -> Result<ArquivoBruto>;
        /// Grava em milímetros; devolve os avisos (B-rep que o openNURBS não considera válida).
        fn gravar_3dm(caminho: &str, arquivo: &ArquivoBruto) -> Result<Vec<String>>;
        fn versao_opennurbs() -> String;
    }
}
