// SPDX-License-Identifier: LGPL-2.1-or-later
//! Etapas 1 e 2: parâmetros mestres com os limites do regulamento e esqueleto publicado.
use crate::apoio::*;
use fenkra_core::*;
use fenkra_solidos::publications;

/// nome, expressão, mín, máx, comentário
const PARAMS: &[(&str, &str, &str, &str, &str)] = &[
    ("EE", "1040", "1010", "1070", "entre-eixos"),
    ("BT", "1170", "", "", "bitola traseira, centro a centro"),
    ("BD", "1040", "", "", "bitola dianteira, centro a centro"),
    ("LR_T", "210", "", "215", "largura da roda traseira"),
    ("LR_D", "130", "", "135", "largura da roda dianteira"),
    ("DR_T", "276", "", "300", "diâmetro da roda traseira"),
    ("DR_D", "260", "", "280", "diâmetro da roda dianteira"),
    ("LT", "BT+LR_T", "1340", "1400", "largura total traseira"),
    ("LD", "BD+LR_D", "", "", "largura total dianteira"),
    ("BAL_D", "640", "350", "680", "balanço dianteiro"),
    ("BAL_T", "380", "", "400", "balanço traseiro"),
    ("COMP", "EE+BAL_D+BAL_T", "", "2100", "comprimento total"),
    ("ALT", "620", "", "650", "altura sem o banco"),
    ("VAO", "30", "", "", "vão livre do quadro"),
    ("D_TUBO", "30", "", "", "tubo principal, diâmetro"),
    ("E_TUBO", "2.4", "", "", "tubo principal, parede"),
    ("R_DOBRA", "90", "", "", "raio de dobra do tubo principal, no centro"),
    ("PM", "640", "", "", "largura entre pinos-mestres"),
    ("CASTER", "12", "", "", "cáster"),
    ("D_EIXO", "50", "", "50", "eixo traseiro, diâmetro"),
    ("E_EIXO", "2.5", "1.9", "", "eixo traseiro, parede"),
    ("L_EIXO", "1040", "", "", "eixo traseiro, comprimento"),
    ("D_DISCO", "200", "", "", "disco de freio, diâmetro"),
    ("E_DISCO", "6", "", "", "disco de freio, espessura"),
    ("LARG_Q", "700", "", "", "largura do quadro entre longarinas, atrás"),
    ("LARG_QD", "560", "", "", "largura do quadro entre longarinas, na frente"),
    ("Z_Q", "VAO+D_TUBO/2", "", "", "altura do eixo das longarinas"),
    ("Z_EIXO_T", "DR_T/2", "", "", "altura do eixo traseiro"),
    ("Z_EIXO_D", "DR_D/2", "", "", "altura do eixo dianteiro"),
    ("D_COLUNA", "20", "18", "", "coluna de direção, diâmetro"),
    ("E_COLUNA", "2", "1.8", "", "coluna de direção, parede"),
    ("D_VOLANTE", "320", "", "", "volante, diâmetro"),
    ("N_PINHAO", "11", "11", "12", "dentes do pinhão"),
    ("N_COROA", "82", "", "", "dentes da coroa"),
    ("ELOS", "106", "94", "116", "elos da corrente"),
    ("PASSO", "7.774", "", "", "passo da corrente 219"),
    ("DP_PINHAO", "PASSO/sin(180/N_PINHAO)", "", "", "diâmetro primitivo do pinhão"),
    ("DP_COROA", "PASSO/sin(180/N_COROA)", "", "", "diâmetro primitivo da coroa"),
    ("DC", "213", "", "", "distância entre centros da corrente"),
    ("D_PC_SUP", "16", "16", "", "para-choque superior, diâmetro"),
    ("RETO_PC_SUP", "385", "375", "395", "para-choque superior, trecho reto"),
    ("Z_PC_SUP", "225", "200", "250", "para-choque superior, topo"),
    ("D_PC_INF", "20", "20", "", "para-choque inferior, diâmetro"),
    ("RETO_PC_INF", "305", "295", "315", "para-choque inferior, trecho reto"),
    ("Z_PC_INF", "90", "70", "110", "para-choque inferior, topo"),
    ("Y_LAT_INF", "500", "480", "520", "barra lateral inferior, distância ao centro"),
    ("Y_LAT_SUP", "560", "480", "600", "barra lateral superior, distância ao centro"),
    ("Z_LAT_SUP", "170", "160", "", "barra lateral superior, topo"),
    ("L_NARIZ", "1050", "1000", "", "bico, largura"),
    ("L_PAINEL", "280", "250", "300", "painel frontal, largura"),
    ("VAO_PONTAO", "40", "25", "60", "pontão, vão do solo"),
    ("VAO_TRAS", "30", "15", "50", "proteção traseira, vão até as rodas"),
    ("E_ASSOALHO", "1.5", "1.4", "", "assoalho, espessura"),
    ("D_FURO_ASS", "8", "", "10", "assoalho, furos"),
    ("E_REFORCO", "2", "1.5", "", "chapa de reforço do banco, espessura"),
    ("TANQUE_L", "8", "8", "", "tanque, litros"),
    ("MASSA_MIN", "150", "", "", "massa mínima com piloto, kg"),
];

pub fn mestre() -> Result<Document, String> {
    let mut d = Document::new("mestre");
    for (n, e, lo, hi, c) in PARAMS {
        let un = match *n {
            "CASTER" => Unit::Angulo,
            "TANQUE_L" | "MASSA_MIN" | "N_PINHAO" | "N_COROA" | "ELOS" => Unit::Numero,
            _ => Unit::Comprimento,
        };
        d.add_param(n, e, un).map_err(|e| e.to_string())?;
        let p = d.params.last_mut().unwrap();
        p.min = (*lo).into();
        p.max = (*hi).into();
        p.comment = (*c).into();
        p.published = true;
    }
    let p = d.params.iter_mut().find(|p| p.name == "BD").unwrap();
    p.verificacao = "BD >= EE*2/3".into();
    let p = d.params.iter_mut().find(|p| p.name == "LT").unwrap();
    p.verificacao = "LT <= 1400".into();

    // planos do solo, dos eixos e do centro
    let eixo_t = d.add_offset_plane(oxy(), "Z_EIXO_T");
    renomeia_ref(&mut d, eixo_t, "PL_EIXO_T");
    publica_ref(&mut d, eixo_t);
    let eixo_d = d.add_offset_plane(oyz(), "EE");
    renomeia_ref(&mut d, eixo_d, "PL_EIXO_D");
    publica_ref(&mut d, eixo_d);
    let solo = d.add_offset_plane(oxy(), "0");
    renomeia_ref(&mut d, solo, "PL_SOLO");
    publica_ref(&mut d, solo);

    // longarinas: X para a frente a partir do eixo traseiro, Y lateral, Z a partir do solo
    for (lado, sn) in [("E", ""), ("D", "-")] {
        let y = |v: &str| format!("{sn}{v}");
        let a = ponto(&mut d, &format!("LG{lado}0"), "-100", &y("LARG_Q/2"), "Z_Q+35");
        let b = ponto(&mut d, &format!("LG{lado}1"), "120", &y("LARG_Q/2"), "Z_Q");
        let c = ponto(&mut d, &format!("LG{lado}2"), "EE*0.6", &y("LARG_Q/2"), "Z_Q");
        let e = ponto(&mut d, &format!("LG{lado}3"), "EE-170", &y("LARG_QD/2"), "Z_Q");
        let f = ponto(&mut d, &format!("LG{lado}4"), "EE+60", &y("LARG_QD/2"), "Z_Q");
        polilinha(&mut d, &format!("LONG_{lado}"), &[a, b, c, e, f]);
    }
    // travessas
    for (nome, x, hy) in [("TR_DIANT", "EE-40", "LARG_QD/2"), ("TR_MONT", "EE*0.42", "LARG_Q/2"), ("TR_TRAS", "120", "LARG_Q/2")] {
        let a = ponto(&mut d, &format!("{nome}_A"), x, &format!("-{hy}"), "Z_Q");
        let b = ponto(&mut d, &format!("{nome}_B"), x, hy, "Z_Q");
        polilinha(&mut d, nome, &[a, b]);
    }
    // arco dianteiro dos pedais
    let a = ponto(&mut d, "ARCO_0", "EE+60", "-LARG_QD/2", "Z_Q");
    let b = ponto(&mut d, "ARCO_1", "EE+190", "-200", "Z_Q+95");
    let c = ponto(&mut d, "ARCO_2", "EE+190", "200", "Z_Q+95");
    let e = ponto(&mut d, "ARCO_3", "EE+60", "LARG_QD/2", "Z_Q");
    polilinha(&mut d, "ARCO", &[a, b, c, e]);
    // apoios do banco
    for (nome, x, y0, y1, z1) in [("BANCO_AE", "300", "LARG_Q/2", "190", "Z_Q+170"), ("BANCO_AD", "300", "-LARG_Q/2", "-190", "Z_Q+170"), ("BANCO_FE", "EE*0.5", "LARG_Q/2", "190", "Z_Q+90"), ("BANCO_FD", "EE*0.5", "-LARG_Q/2", "-190", "Z_Q+90")] {
        let a = ponto(&mut d, &format!("{nome}_A"), x, y0, "Z_Q");
        let b = ponto(&mut d, &format!("{nome}_B"), x, y1, z1);
        polilinha(&mut d, nome, &[a, b]);
    }
    // posições de roda, pino-mestre, banco e motor
    for (n, x, y, z) in [
        ("RODA_TE", "0", "BT/2", "Z_EIXO_T"),
        ("RODA_TD", "0", "-BT/2", "Z_EIXO_T"),
        ("RODA_DE", "EE", "BD/2", "Z_EIXO_D"),
        ("RODA_DD", "EE", "-BD/2", "Z_EIXO_D"),
        ("PINO_E", "EE", "PM/2", "Z_EIXO_D"),
        ("PINO_D", "EE", "-PM/2", "Z_EIXO_D"),
        ("BANCO", "EE*0.33", "0", "Z_Q+60"),
        ("MOTOR", "DC*0.8", "-LARG_Q/2-120", "Z_EIXO_T+DC*0.6"),
    ] {
        let p = ponto(&mut d, n, x, y, z);
        publica_ref(&mut d, p);
    }
    Ok(d)
}

/// Liga parâmetros e curvas do mestre a uma peça. Devolve o id de cada curva pedida.
pub fn liga(d: &mut Document, m: &[(String, LinkedGeo)], params: &[&str], curvas: &[&str]) -> Result<Vec<Id>, String> {
    let mut out = vec![];
    for n in params.iter().chain(curvas.iter()) {
        let g = m.iter().find(|(p, _)| p == n).ok_or(format!("publicação {n} não existe"))?.1.clone();
        let id = d.add_link("mestre", "MESTRE.fkp", n, g);
        if curvas.contains(n) {
            out.push(id);
        } else {
            // contorno K-01: o link de valor nasce com sufixo 1 (mestre_EE1)
            d.rename_param(id, &format!("mestre_{n}")).map_err(|e| e.to_string())?;
        }
    }
    Ok(out)
}

pub fn publicacoes(d: &Document) -> Vec<(String, LinkedGeo)> {
    publications(d)
}
