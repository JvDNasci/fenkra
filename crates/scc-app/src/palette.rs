//! Paleta de cores: tabela RAL clássica (cores com nome), cores salvas pelo usuário e a
//! "área de transferência" de aparência (copiar/colar entre elementos).

use scc_core::Appearance;
use std::sync::Mutex;

/// Cores salvas pelo usuário (sincronizadas com as preferências pela aplicação).
pub static USER: Mutex<Vec<(String, [u8; 3])>> = Mutex::new(Vec::new());
/// O usuário salvou/alterou uma cor nesta passada (a aplicação grava nas preferências).
pub static DIRTY: Mutex<bool> = Mutex::new(false);
/// Aparência copiada.
pub static CLIP: Mutex<Option<Appearance>> = Mutex::new(None);

/// Tabela RAL clássica (valores sRGB aproximados).
pub const RAL: [(&str, [u8; 3]); 32] = [
    ("RAL 1003 Amarelo sinal", [249, 168, 0]),
    ("RAL 1013 Branco pérola", [227, 217, 198]),
    ("RAL 1015 Marfim claro", [230, 210, 181]),
    ("RAL 1021 Amarelo colza", [238, 186, 0]),
    ("RAL 1023 Amarelo tráfego", [247, 181, 0]),
    ("RAL 2004 Laranja puro", [226, 83, 3]),
    ("RAL 2008 Laranja vermelho claro", [237, 107, 33]),
    ("RAL 3000 Vermelho fogo", [167, 41, 32]),
    ("RAL 3003 Vermelho rubi", [134, 26, 34]),
    ("RAL 3020 Vermelho tráfego", [204, 12, 0]),
    ("RAL 4005 Lilás azulado", [118, 104, 154]),
    ("RAL 5002 Azul ultramar", [32, 33, 79]),
    ("RAL 5005 Azul sinal", [0, 78, 124]),
    ("RAL 5010 Azul genciana", [0, 79, 124]),
    ("RAL 5012 Azul claro", [0, 137, 182]),
    ("RAL 5015 Azul céu", [0, 124, 176]),
    ("RAL 5017 Azul tráfego", [0, 91, 140]),
    ("RAL 6005 Verde musgo", [17, 66, 50]),
    ("RAL 6011 Verde reseda", [104, 125, 94]),
    ("RAL 6018 Verde amarelado", [72, 164, 63]),
    ("RAL 6024 Verde tráfego", [0, 131, 81]),
    ("RAL 7001 Cinza prata", [138, 149, 151]),
    ("RAL 7012 Cinza basalto", [78, 87, 84]),
    ("RAL 7016 Cinza antracite", [56, 62, 66]),
    ("RAL 7024 Cinza grafite", [71, 74, 80]),
    ("RAL 7035 Cinza claro", [203, 208, 204]),
    ("RAL 7040 Cinza janela", [154, 155, 155]),
    ("RAL 8017 Marrom chocolate", [69, 50, 46]),
    ("RAL 9005 Preto profundo", [10, 10, 13]),
    ("RAL 9006 Alumínio branco", [161, 161, 160]),
    ("RAL 9007 Alumínio cinza", [135, 133, 129]),
    ("RAL 9010 Branco puro", [241, 236, 225]),
];
