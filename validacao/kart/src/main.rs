// SPDX-License-Identifier: LGPL-2.1-or-later
//! Projeto-teste B353: o kart inteiro, por código, sobre as APIs do Fenkra.
//! `kart` roda as peças; `kart CH` só as de código que começa com CH.
mod apoio;
mod cobertura;
mod conjunto;
mod formas;
mod mestre;
mod pecas;
mod quadro;
mod saidas;

use apoio::*;
use pecas as p;

const ACO: &str = "Aço AISI 4130";
const ACO_C: &str = "Aço 1020";
const AL: &str = "Alumínio 6061";

fn main() {
    std::fs::create_dir_all(modelo()).unwrap();
    let filtro = std::env::args().nth(1).unwrap_or_default();
    let mut r = Reg::default();
    let m = r.peca("MESTRE", "Desenho mestre", "Sketch", "", mestre::mestre).expect("mestre");
    let pubs = mestre::publicacoes(&m);
    println!("publicações: {}", pubs.len());
    let u = &pubs[..];
    macro_rules! pc {
        ($c:expr, $n:expr, $b:expr, $m:expr, $f:expr) => {
            if filtro.is_empty() || $c.starts_with(&filtro) {
                r.peca($c, $n, $b, $m, || $f);
            }
        };
    }
    pc!("CH-01", "Longarina esquerda", "Estrutural", ACO, quadro::longarina(u, "E"));
    pc!("CH-02", "Longarina direita", "Estrutural", ACO, quadro::longarina(u, "D"));
    pc!("CH-03", "Travessa dianteira", "Estrutural", ACO, quadro::reto(u, &["TR_DIANT"]));
    pc!("CH-04", "Montante central", "Estrutural", ACO, quadro::reto(u, &["TR_MONT"]));
    pc!("CH-05", "Travessa traseira", "Estrutural", ACO, quadro::reto(u, &["TR_TRAS"]));
    pc!("CH-06", "Arco dianteiro dos pedais", "Estrutural", ACO, quadro::arco(u));
    pc!("CH-07", "Tubo de apoio do banco", "Estrutural", ACO, quadro::reto(u, &["BANCO_AE"]));
    if filtro.is_empty() || "QUADRO".starts_with(&filtro) {
        if let Some(d) = r.peca("QUADRO", "Quadro soldado", "Estrutural", ACO, || quadro::quadro(u)) {
            quadro::relata_quadro(&d);
        }
    }
    pc!("CH-08", "Suporte do mancal", "Chapa", ACO_C, p::ch08(u));
    pc!("CH-09", "Suporte em C do pino-mestre", "Sólidos", ACO, p::ch09(u));
    pc!("CH-10", "Soquete de para-choque", "Estrutural", ACO, p::ch10(u));
    pc!("CH-11", "Apoio do motor", "Chapa", ACO_C, p::ch11(u));
    pc!("CH-12", "Suporte da coluna", "Chapa", ACO_C, p::ch12(u));
    pc!("CH-13", "Orelha de pedal", "Chapa", ACO_C, p::ch13(u));
    pc!("CH-14", "Suporte da pinça", "Chapa", ACO_C, p::ch14(u));
    pc!("EX-01", "Eixo traseiro", "Sólidos", ACO, p::ex01(u));
    pc!("EX-02", "Caixa do mancal", "Sólidos", AL, p::ex02(u));
    pc!("EX-05", "Cubo traseiro", "Sólidos", AL, p::ex05(u));
    pc!("EX-06", "Porta-coroa", "Sólidos", AL, p::porta(u, 75., 6));
    pc!("EX-07", "Porta-disco", "Sólidos", AL, p::porta(u, 65., 6));
    pc!("EX-08", "Disco de freio", "Sólidos", ACO_C, p::ex08(u));
    pc!("RD-01", "Aro dianteiro", "Sólidos", AL, p::aro(130.));
    pc!("RD-02", "Aro traseiro", "Sólidos", AL, p::aro(210.));
    pc!("RD-02A", "Aro sem os pinos (diagnóstico K-10)", "Sólidos", AL, cobertura::aro_etapa(210., 1));
    pc!("RD-02B", "Aro com furos e sem padrão de pinos (diagnóstico K-10)", "Sólidos", AL, cobertura::aro_etapa(210., 2));
    pc!("RD-03", "Pneu dianteiro", "Superfícies", "", p::pneu(u, "DR_D", "LR_D"));
    pc!("RD-04", "Pneu traseiro", "Superfícies", "", p::pneu(u, "DR_T", "LR_T"));
    pc!("DI-01", "Volante", "Forma livre", "", formas::di01(u));
    pc!("DI-02", "Cubo do volante", "Sólidos", AL, p::di02(u));
    pc!("DI-03", "Coluna de direção", "Estrutural", ACO, p::di03(u));
    pc!("DI-04", "Mancal inferior da coluna", "Sólidos", AL, p::di04(u));
    pc!("DI-05", "Placa dos braços", "Chapa", ACO_C, p::di05(u));
    pc!("DI-06", "Tirante", "Estrutural", ACO, p::di06(u));
    pc!("DI-08", "Manga de eixo esquerda", "Sólidos", ACO, p::di08(u, false));
    pc!("DI-08D", "Manga de eixo direita", "Sólidos", ACO, p::di08(u, true));
    pc!("DI-09", "Pino-mestre", "Sólidos", ACO, p::di09(u));
    pc!("DI-10", "Cubo dianteiro", "Sólidos", AL, p::di10(u));
    pc!("MO-01", "Volume do motor", "Sólidos", AL, p::mo01(u));
    pc!("MO-02", "Suporte do motor", "Sólidos", AL, p::mo02(u));
    pc!("TR-01", "Coroa de 82 dentes", "Transmissão", AL, p::roda_manual(u, "DP_COROA", "N_COROA", 90.));
    pc!("TR-02", "Pinhão de 11 dentes", "Transmissão", ACO, p::roda_manual(u, "DP_PINHAO", "N_PINHAO", 15.));
    pc!("TR-03", "Corrente de 106 elos", "Transmissão", ACO, p::corrente_manual(u));
    pc!("TR-04", "Protetor de corrente", "Chapa", ACO_C, p::tr04(u));
    pc!("EX-09", "Corpo da pinça", "Sólidos", AL, p::ex09(u));
    pc!("EX-10", "Pastilha", "Sólidos", ACO_C, p::ex10(u));
    pc!("EX-11", "Cilindro-mestre", "Sólidos", AL, p::ex11(u));
    pc!("EX-12", "Linha de freio", "Superfícies", ACO_C, p::ex12(u));
    pc!("PO-05", "Pedal de freio", "Chapa", ACO_C, p::pedal(70.));
    pc!("PO-06", "Pedal de acelerador", "Chapa", ACO_C, p::pedal(55.));
    pc!("PO-07", "Vareta do pedal", "Estrutural", ACO, p::barra("Vareta", ["0", "0", "0"], ["520", "0", "0"], "6"));
    pc!("PO-01", "Banco", "Forma livre", "", formas::po01(u));
    pc!("PO-02", "Chapa de reforço do banco", "Chapa", ACO_C, p::po02(u));
    pc!("PO-03", "Escora do banco", "Estrutural", ACO, p::po03(u));
    pc!("PO-04", "Assoalho furado", "Chapa", ACO_C, p::po04(u));
    pc!("PO-08", "Tanque", "Superfícies", "", p::po08(u));
    pc!("PR-01", "Para-choque superior", "Estrutural", ACO, formas::parachoque(u, true));
    pc!("PR-02", "Para-choque inferior", "Estrutural", ACO, formas::parachoque(u, false));
    pc!("PR-03", "Suporte de ligação", "Estrutural", ACO, formas::pr03(u));
    pc!("PR-04", "Barra lateral inferior", "Estrutural", ACO, formas::lateral(u, false));
    pc!("PR-05", "Barra lateral superior", "Estrutural", ACO, formas::lateral(u, true));
    pc!("PR-10", "Suporte da proteção traseira", "Estrutural", ACO, formas::pr10(u));
    pc!("PR-06", "Bico", "Forma livre", "", formas::pr06(u));
    pc!("PR-07", "Painel frontal", "Forma livre", "", formas::pr07(u));
    pc!("PR-08", "Pontão lateral", "Forma livre", "", formas::pr08(u));
    pc!("PR-09", "Proteção traseira", "Forma livre", "", formas::pr09(u));
    pc!("MO-03", "Escape", "Superfícies", ACO_C, p::mo03(u));
    // peças de biblioteca: ficam no fim da fila (B374, B375 em andamento)
    pc!("EX-03", "Rolamento do eixo 6010", "Biblioteca", "", fenkra_solidos::fixadores::fixador_documento("iso15_60", "6010", None));
    pc!("EX-04", "Chaveta", "Biblioteca", "", fenkra_solidos::eixos::chaveta_documento(50., 40.));
    pc!("DI-11", "Rolamento dianteiro 6005", "Biblioteca", "", fenkra_solidos::fixadores::fixador_documento("iso15_60", "6005", None));
    pc!("FX-01", "Parafuso M8", "Biblioteca", "", fenkra_solidos::fixadores::fixador_documento("iso4762", "M8", Some(40.)));
    pc!("FX-03", "Parafuso M6", "Biblioteca", "", fenkra_solidos::fixadores::fixador_documento("iso4762", "M6", Some(30.)));
    pc!("FX-02", "Porca autotravante M8 (contorno: porca comum)", "Biblioteca", "", fenkra_solidos::fixadores::fixador_documento("iso4032", "M8", None));
    pc!("FX-04", "Porca autotravante M6 (contorno: porca comum)", "Biblioteca", "", fenkra_solidos::fixadores::fixador_documento("iso4032", "M6", None));
    pc!("DI-07", "Terminal rotular (contorno: revolução)", "Biblioteca", "", p::rolamento(8., 22., 12.));
    pc!("CV-CHAPA1", "Assoalho com estampo, veneziana e colar", "Chapa", ACO_C, cobertura::assoalho_estampado());
    pc!("CV-CHAPA2", "Protetor com bainha e desvio", "Chapa", ACO_C, cobertura::protetor_bainha());
    pc!("CV-CHAPA3", "Caixa com alívio de canto", "Chapa", ACO_C, cobertura::caixa_alivio());
    pc!("CV-CHAPA4", "Canal convertido em chapa", "Chapa", ACO_C, cobertura::canal_convertido());
    pc!("CV-SOL1", "Furos, rosca modelada e espessura", "Sólidos", AL, cobertura::furos_e_rosca());
    pc!("CV-SOL2", "Nervura, escala e dividir", "Sólidos", AL, cobertura::nervura_escala());
    pc!("CV-SOL3", "Padrão por pontos e chanfro geral", "Sólidos", AL, cobertura::padrao_pontos());
    pc!("EX-12B", "Linha de freio, segunda tentativa", "Sólidos", ACO_C, cobertura::linha_varredura(u));
    pc!("CV-EST", "Placa, reforço e solda", "Estrutural", ACO, cobertura::estrutural_extra());
    pc!("CV-SUP1", "Extrusão e deslocada de superfície", "Superfícies", "", cobertura::superficies());
    pc!("CV-SUP2", "Preenchimento de contorno", "Superfícies", "", cobertura::preencher());
    if filtro == "ACAB" {
        saidas::pmi_geral(&mut r);
        saidas::desenhos(&mut r);
        return;
    }
    if filtro == "DIAG" {
        r.peca("RD-02A", "Aro sem os pinos (diagnóstico K-10)", "Sólidos", AL, || cobertura::aro_etapa(210., 1));
        r.peca("RD-02B", "Aro com furos e sem padrão de pinos (diagnóstico K-10)", "Sólidos", AL, || cobertura::aro_etapa(210., 2));
        r.peca("CV-EST", "Placa, reforço e solda", "Estrutural", ACO, cobertura::estrutural_extra);
        conjunto::teste_invertido();
        conjunto::testa_fixadores(&mut r);
        conjunto::testa_gerador(&mut r, u);
        return;
    }
    if filtro.is_empty() || filtro == "SC" {
        conjunto::kart(u, &mut r);
    }
    if filtro.is_empty() || filtro == "SAIDAS" {
        saidas::pmi(&mut r);
        saidas::pmi_geral(&mut r);
        saidas::desenhos(&mut r);
        saidas::pacote(&mut r);
        if let Err(e) = saidas::plm(&mut r) {
            println!("PLM falhou: {e}");
        }
    }
    let ok = r.linhas.values().filter(|l| l.ok).count();
    println!("=== {} peças, {} ok", r.linhas.len(), ok);
    r.grava_tabelas();
}
