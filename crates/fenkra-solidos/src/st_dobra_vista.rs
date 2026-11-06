// SPDX-License-Identifier: LGPL-2.1-or-later
//! Vista do tubo reto para o desenho (B331): o tubo desenvolvido visto de lado, com o comprimento
//! total, as marcas de início e fim de cada dobra e a posição de cada rasgo em V ou do centro do
//! canto com gabarito. Vale para qualquer modo (a dobradeira também tem início e fim de dobra).
//!
//! O eixo X corre ao longo do tubo, medido da ponta da primeira perna, o mesmo `desenvolvido` da
//! lista de corte e da tabela de dobras. O desenho é próprio.

use crate::st_dobra::{nome_modo, TuboDobrado};
use crate::st_dobra_corte::{posicoes_dos_v, Andorinha, Camada, Desenho};
use fenkra_core::BendMode;

fn num(v: f64) -> String {
    let s = format!("{v:.2}");
    let s = s.trim_end_matches('0').trim_end_matches('.').to_string();
    s.replace('.', ",")
}

/// Largura do tubo na vista: a dimensão no plano da dobra, ou 30 mm quando o nó não a informa.
fn largura(t: &TuboDobrado) -> f64 {
    t.dobras.iter().map(|d| d.resultado.h_plano).find(|h| *h > 0.).unwrap_or(30.)
}

/// Vista lateral do tubo reto com as marcas. `Err` quando o tubo não tem dobra.
pub fn vista_reta(t: &TuboDobrado) -> Result<Desenho, String> {
    if t.dobras.is_empty() {
        return Err("o tubo não tem dobra".into());
    }
    let (comp, h) = (t.desenvolvido, largura(t));
    let mut d = Desenho { titulo: format!("Tubo reto · {}", t.designacao), ..Default::default() };
    d.retangulo(Camada::Contorno, 0., 0., comp, h);
    d.linha(Camada::Marcacao, [0., h / 2.], [comp, h / 2.], true);
    for (k, db) in t.dobras.iter().enumerate() {
        let r = &db.resultado;
        if !r.erros.is_empty() {
            continue;
        }
        // início e fim da zona de dobra
        for x in [db.inicio, db.fim] {
            d.linha(Camada::Dobra, [x, -2.], [x, h + 2.], true);
        }
        let meio = (db.inicio + db.fim) / 2.;
        d.texto(Camada::Cota, [meio - 14., h + 5.], 3., format!("D{} {}° R{}", k + 1, num(r.theta), num(r.raio)));
        d.texto(Camada::Cota, [meio - 14., h + 9.], 2.5, nome_modo(r.modo));
        match r.modo {
            BendMode::VComum | BendMode::CorteDobradeira => {
                let (w, par) = (r.abertura_v.unwrap_or(0.), r.parede);
                let mut posicoes = vec![];
                for (j, xc) in posicoes_dos_v(r, db.inicio, db.fim).into_iter().enumerate() {
                    d.poli(Camada::Corte, vec![[xc - w / 2., h], [xc, par], [xc + w / 2., h]], false);
                    // rótulo curto em três alturas alternadas (rasgos vizinhos ficam muito juntos); as
                    // posições completas vão numa linha só, embaixo
                    d.texto(Camada::Cota, [xc - 2., -5. - (j % 3) as f64 * 4.5], 2.5, format!("V{}", j + 1));
                    posicoes.push(format!("V{} x={}", j + 1, num(xc)));
                }
                d.texto(Camada::Cota, [db.inicio - 10., -33. - k as f64 * 5.], 2.5, posicoes.join(" · "));
            }
            BendMode::VComRaio => {
                if let Some(a) = Andorinha::de(r) {
                    d.poli(Camada::Corte, a.contorno_fechado().into_iter().map(|p| [db.inicio + p[0], p[1]]).collect(), true);
                    d.linha(Camada::Marcacao, [db.inicio + a.placa / 2., -2.], [db.inicio + a.placa / 2., h + 2.], true);
                    d.texto(Camada::Cota, [db.inicio, -6.], 2.5, format!("canto x={}", num(db.inicio)));
                }
            }
            _ => {}
        }
    }
    d.linha(Camada::Cota, [0., -22.], [comp, -22.], false);
    d.linha(Camada::Cota, [0., -24.], [0., -20.], false);
    d.linha(Camada::Cota, [comp, -24.], [comp, -20.], false);
    d.texto(Camada::Cota, [comp / 2. - 12., -28.], 3., format!("{} mm", num(comp)));
    Ok(d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::st_dobra::{calcular, DobraDoTubo, EntradaDobra};
    use fenkra_core::SectionKind;

    fn tubo(modo: BendMode, raio: f64, rasgos: usize) -> TuboDobrado {
        let e = EntradaDobra { tipo: SectionKind::TuboQuadrado, dims: vec![30., 1.2], theta: 90., modo, raio, regra: None, k: None, retorno: None, folga_raiz: None, rasgos, gomos: 1, plano: 0 };
        let r = calcular(&e);
        let inicio = 200. - r.tangente;
        let fim = inicio + r.arco;
        TuboDobrado {
            feature: fenkra_core::Id(1),
            linhas: vec![0, 1],
            designacao: "Tubo quadrado 30x1,2".into(),
            material: "Aço".into(),
            pernas: vec![200., 150.],
            dobras: vec![DobraDoTubo { linhas: (0, 1), ponto: [0.; 3], resultado: r.clone(), inicio, fim }],
            desenvolvido: 350. + r.ajuste_corte,
            soma_pernas: 350.,
            avisos: vec![],
            erros: vec![],
            fundido: true,
            pontos: vec![],
        }
    }

    #[test]
    fn a_vista_reta_traz_o_comprimento_e_as_marcas_de_cada_dobra() {
        for (modo, raio, rasgos) in [(BendMode::VComum, 40., 3), (BendMode::VComRaio, 30., 1), (BendMode::Dobradeira, 60., 1)] {
            let t = tubo(modo, raio, rasgos);
            let d = vista_reta(&t).unwrap();
            // o contorno tem o comprimento desenvolvido
            let (lo, hi) = d.caixa();
            assert!(lo[0] <= 0. && hi[0] >= t.desenvolvido, "{modo:?}: {lo:?} {hi:?}");
            let marcas: Vec<f64> = d.tracos.iter().filter(|x| x.camada == Camada::Dobra).map(|x| x.pts[0][0]).collect();
            assert!((marcas[0] - t.dobras[0].inicio).abs() < 1e-9 && (marcas[1] - t.dobras[0].fim).abs() < 1e-9, "{modo:?}: {marcas:?}");
            assert!(d.rotulos.iter().any(|r| r.texto.starts_with("D1 90°")), "{modo:?}");
        }
        // V comum com três rasgos: um triângulo de corte por rasgo, nas posições do passo
        let t = tubo(BendMode::VComum, 40., 3);
        let d = vista_reta(&t).unwrap();
        assert_eq!(d.tracos.iter().filter(|x| x.camada == Camada::Corte).count(), 3);
        assert!(vista_reta(&TuboDobrado { dobras: vec![], ..t }).is_err());
    }
}
