// SPDX-License-Identifier: LGPL-2.1-or-later
//! Assistente de Novo desenho (B252): formato, orientação, norma, método de projeção, escala geral
//! e origem (a peça aberta como padrão) viram um desenho pronto, com as três vistas sugeridas já
//! na escala e na posição certas. A interface só mostra o diálogo e chama [`NovoDesenho::criar`].

use crate::source::Models;
use crate::{Dir, Drawing, Format, Source, TitleStyle, V2, ViewKind};
use serde::{Deserialize, Serialize};

/// Método de projeção das vistas (ISO 5456-2).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum Projecao {
    /// 1º diedro (ISO, ABNT): a superior fica abaixo da frente e a lateral esquerda à direita
    #[default]
    Primeiro,
    /// 3º diedro (ANSI): a superior fica acima da frente e a lateral direita à direita
    Terceiro,
}

impl Projecao {
    pub const ALL: [Projecao; 2] = [Projecao::Primeiro, Projecao::Terceiro];
    pub fn label(self) -> &'static str {
        match self {
            Projecao::Primeiro => "1º diedro (ISO, ABNT)",
            Projecao::Terceiro => "3º diedro (ANSI)",
        }
    }
    /// Nome curto, para a legenda da folha.
    pub fn curto(self) -> &'static str {
        match self {
            Projecao::Primeiro => "1º diedro",
            Projecao::Terceiro => "3º diedro",
        }
    }
    /// Direções das vistas projetadas a partir da frente: (superior, lateral à direita).
    pub fn vistas_da_frente(self) -> (Dir, Dir) {
        match self {
            Projecao::Primeiro => (Dir::Topo, Dir::Esquerda),
            Projecao::Terceiro => (Dir::Topo, Dir::Direita),
        }
    }
    /// Deslocamento (dx, dy) da vista projetada em relação à mãe, em múltiplos de `gap`, para a
    /// superior e para a lateral. A folha tem y para cima.
    pub fn posicoes(self) -> ([f64; 2], [f64; 2]) {
        match self {
            Projecao::Primeiro => ([0., -1.], [1., 0.]),
            Projecao::Terceiro => ([0., 1.], [1., 0.]),
        }
    }
    /// Direção da vista projetada pela posição do clique (dx, dy) em relação à mãe. Da frente: no
    /// 1º diedro abaixo é a superior e à direita a lateral esquerda; no 3º, acima é a superior e à
    /// direita a lateral direita. De outras vistas segue o 1º diedro (como sempre foi).
    pub fn dir_projetada(self, base: Dir, dx: f64, dy: f64) -> Dir {
        let horizontal = dx.abs() > dy.abs();
        match self {
            Projecao::Terceiro if base == Dir::Frente => match (horizontal, dx > 0., dy > 0.) {
                (true, true, _) => Dir::Direita,
                (true, false, _) => Dir::Esquerda,
                (false, _, true) => Dir::Topo,
                (false, _, false) => Dir::Base,
            },
            _ => {
                if horizontal {
                    match (base, dx > 0.) {
                        (Dir::Frente, true) => Dir::Esquerda,
                        (Dir::Frente, false) => Dir::Direita,
                        (Dir::Esquerda, true) => Dir::Tras,
                        (Dir::Direita, false) => Dir::Tras,
                        (_, true) => Dir::Esquerda,
                        (_, false) => Dir::Direita,
                    }
                } else if dy < 0. {
                    Dir::Topo
                } else {
                    Dir::Base
                }
            }
        }
    }
}

/// Escalas de prateleira para a escala automática (maior primeiro).
pub const ESCALAS_AUTO: [f64; 10] = [10., 5., 2., 1., 0.5, 0.2, 0.1, 0.05, 0.02, 0.01];

/// Respostas do diálogo de Novo desenho.
#[derive(Clone, PartialEq, Debug)]
pub struct NovoDesenho {
    pub nome: String,
    pub formato: Format,
    pub retrato: bool,
    pub norma: TitleStyle,
    pub projecao: Projecao,
    /// escala geral; `None` = automática (a maior que cabe)
    pub escala: Option<f64>,
    /// peça ou montagem de origem das vistas; `None` = folha em branco
    pub origem: Option<Source>,
}

impl NovoDesenho {
    /// Padrão: A3 paisagem, ISO 7200, 1º diedro, escala automática, a origem dada.
    pub fn padrao(nome: &str, origem: Option<Source>) -> Self {
        NovoDesenho { nome: nome.into(), formato: Format::A3, retrato: false, norma: TitleStyle::Iso7200, projecao: Projecao::Primeiro, escala: None, origem }
    }

    /// A norma escolhida traz o que costuma acompanhá-la: ANSI usa folhas ANSI e 3º diedro; ISO e
    /// ABNT usam a série A e o 1º diedro. O que a pessoa já ajustou de outra família é trocado.
    pub fn aplicar_norma(&mut self, n: TitleStyle) {
        self.norma = n;
        let ansi = matches!(self.formato, Format::AnsiA | Format::AnsiB | Format::AnsiC | Format::AnsiD | Format::AnsiE);
        match n {
            TitleStyle::Ansi => {
                self.projecao = Projecao::Terceiro;
                if !ansi {
                    self.formato = if self.formato == Format::A4 { Format::AnsiA } else { Format::AnsiB };
                }
            }
            TitleStyle::Iso7200 | TitleStyle::Abnt => {
                self.projecao = Projecao::Primeiro;
                if ansi {
                    self.formato = if self.formato == Format::AnsiA { Format::A4 } else { Format::A3 };
                }
            }
            TitleStyle::Simples => {}
        }
    }

    /// Tamanho da folha em mm, já com a orientação.
    pub fn tamanho(&self) -> V2 {
        let s = self.formato.size();
        if self.retrato { [s[1], s[0]] } else { s }
    }

    /// Área útil para as vistas: dentro da moldura, acima da legenda e com folga das zonas.
    fn area_util(&self) -> (V2, V2) {
        let [w, h] = self.tamanho();
        ([30., 55.], [w - 20., h - 25.])
    }

    /// Cria o desenho: a folha configurada e, havendo origem, a frente e as duas projetadas dela
    /// (superior e lateral, conforme o método de projeção) na escala geral e centradas na área útil.
    pub fn criar(&self, models: &mut Models) -> Drawing {
        let mut d = Drawing::new(&self.nome, self.formato);
        d.sheets[0].portrait = self.retrato;
        d.title.style = self.norma;
        d.projection = self.projecao;
        let Some(src) = self.origem.clone() else { return d };
        let sheet = d.sheets[0].id;
        let (dir_sup, dir_lat) = self.projecao.vistas_da_frente();
        // extensões em escala 1 (mm do modelo) de cada vista
        let mut medidas = d.clone();
        // ids altos: o cache de projeções é por id e não pode confundir com as vistas reais
        medidas.reserve_ids(1 << 41);
        let mut ext = |kind: ViewKind| -> (V2, V2, V2) {
            let id = medidas.add_view(sheet, src.clone(), kind, [0., 0.], 1.);
            let p = models.proj(&medidas, id);
            (p.lo, p.hi, p.center)
        };
        let (fl, fh, fc) = ext(ViewKind::Standard(Dir::Frente));
        let (sl, sh_, sc) = ext(ViewKind::Standard(dir_sup));
        let (ll, lh, lc) = ext(ViewKind::Standard(dir_lat));
        let (fw, fhgt) = (fh[0] - fl[0], fh[1] - fl[1]);
        let shh = sh_[1] - sl[1];
        let lw = lh[0] - ll[0];
        let gap = 20.;
        // caixa do conjunto em escala s: largura = frente + folga + lateral; altura = frente + folga + superior
        let conjunto = |s: f64| (fw * s + gap + lw * s, fhgt * s + gap + shh * s);
        let (u0, u1) = self.area_util();
        let (uw, uh) = (u1[0] - u0[0], u1[1] - u0[1]);
        let escala = self.escala.unwrap_or_else(|| {
            if fw <= 0. || fhgt <= 0. {
                return 1.;
            }
            ESCALAS_AUTO.iter().copied().find(|s| {
                let (w, h) = conjunto(*s);
                w <= uw && h <= uh
            }).unwrap_or(ESCALAS_AUTO[ESCALAS_AUTO.len() - 1])
        });
        let (cw, ch) = conjunto(escala);
        // canto inferior esquerdo do conjunto, centrado na área útil
        let (x0, y0) = (u0[0] + (uw - cw) / 2., u0[1] + (uh - ch) / 2.);
        let (dsup, dlat) = self.projecao.posicoes();
        // centro da frente: 1º diedro tem a superior embaixo (frente no alto); 3º, em cima (frente embaixo)
        let frente_c = [x0 + fw * escala / 2., if dsup[1] < 0. { y0 + ch - fhgt * escala / 2. } else { y0 + fhgt * escala / 2. }];
        let sup_c = [frente_c[0], frente_c[1] + dsup[1] * ((fhgt + shh) * escala / 2. + gap)];
        let lat_c = [frente_c[0] + dlat[0] * ((fw + lw) * escala / 2. + gap), frente_c[1]];
        // a posição da vista é o ponto do modelo `center`; o centro da caixa fica deslocado dele
        let pos = |c: V2, lo: V2, hi: V2, ct: V2| [c[0] - ((lo[0] + hi[0]) / 2. - ct[0]) * escala, c[1] - ((lo[1] + hi[1]) / 2. - ct[1]) * escala];
        let f = d.add_view(sheet, src.clone(), ViewKind::Standard(Dir::Frente), pos(frente_c, fl, fh, fc), escala);
        // as projetadas se alinham à frente: a superior no x dela, a lateral no y dela
        let mut p_sup = pos(sup_c, sl, sh_, sc);
        let mut p_lat = pos(lat_c, ll, lh, lc);
        let pf = d.view(f).map(|v| v.pos).unwrap_or(frente_c);
        p_sup[0] = pf[0];
        p_lat[1] = pf[1];
        d.add_view(sheet, src.clone(), ViewKind::Projected { parent: f, dir: dir_sup }, p_sup, escala);
        d.add_view(sheet, src, ViewKind::Projected { parent: f, dir: dir_lat }, p_lat, escala);
        d
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout;

    #[test]
    fn projecao_da_frente_nos_dois_metodos() {
        // 1º diedro: abaixo = superior, à direita = lateral esquerda
        assert_eq!(Projecao::Primeiro.dir_projetada(Dir::Frente, 0., -30.), Dir::Topo);
        assert_eq!(Projecao::Primeiro.dir_projetada(Dir::Frente, 30., 0.), Dir::Esquerda);
        assert_eq!(Projecao::Primeiro.dir_projetada(Dir::Frente, -30., 0.), Dir::Direita);
        assert_eq!(Projecao::Primeiro.dir_projetada(Dir::Frente, 0., 30.), Dir::Base);
        // 3º diedro: acima = superior, à direita = lateral direita
        assert_eq!(Projecao::Terceiro.dir_projetada(Dir::Frente, 0., 30.), Dir::Topo);
        assert_eq!(Projecao::Terceiro.dir_projetada(Dir::Frente, 30., 0.), Dir::Direita);
        assert_eq!(Projecao::Terceiro.dir_projetada(Dir::Frente, -30., 0.), Dir::Esquerda);
        assert_eq!(Projecao::Terceiro.dir_projetada(Dir::Frente, 0., -30.), Dir::Base);
        // de outra vista o comportamento antigo vale nos dois
        assert_eq!(Projecao::Terceiro.dir_projetada(Dir::Esquerda, 30., 0.), Projecao::Primeiro.dir_projetada(Dir::Esquerda, 30., 0.));
    }

    #[test]
    fn norma_traz_formato_e_diedro_da_sua_familia() {
        let mut n = NovoDesenho::padrao("t", None);
        n.aplicar_norma(TitleStyle::Ansi);
        assert_eq!((n.projecao, n.formato), (Projecao::Terceiro, Format::AnsiB));
        n.aplicar_norma(TitleStyle::Abnt);
        assert_eq!((n.projecao, n.formato), (Projecao::Primeiro, Format::A3));
        n.formato = Format::A4;
        n.aplicar_norma(TitleStyle::Ansi);
        assert_eq!(n.formato, Format::AnsiA);
        n.aplicar_norma(TitleStyle::Iso7200);
        assert_eq!(n.formato, Format::A4);
        // a norma simples não mexe no que a pessoa escolheu
        n.projecao = Projecao::Terceiro;
        n.aplicar_norma(TitleStyle::Simples);
        assert_eq!((n.projecao, n.formato), (Projecao::Terceiro, Format::A4));
    }

    #[test]
    fn folha_em_branco_respeita_formato_orientacao_e_norma() {
        let mut n = NovoDesenho::padrao("Em branco", None);
        n.formato = Format::A4;
        n.retrato = true;
        n.norma = TitleStyle::Abnt;
        n.projecao = Projecao::Terceiro;
        let d = n.criar(&mut Models::new(None));
        assert!(d.views.is_empty());
        assert_eq!(d.sheets[0].size(), [210., 297.]);
        assert_eq!((d.title.style, d.projection, d.name.as_str()), (TitleStyle::Abnt, Projecao::Terceiro, "Em branco"));
    }

    fn com_peca(nome: &str) -> (std::path::PathBuf, Source) {
        let dir = std::env::temp_dir().join(nome);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("placa.fkp"), fenkra_solidos::example_document().to_json()).unwrap();
        (dir, Source { path: "placa.fkp".into(), display_state: String::new(), hidden: vec![] })
    }

    #[test]
    fn tres_vistas_sugeridas_cabem_na_folha_e_seguem_o_diedro() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (dir, src) = com_peca("fenkra_desenho_novo");
        for proj in Projecao::ALL {
            let mut n = NovoDesenho::padrao("Placa", Some(src.clone()));
            n.projecao = proj;
            let mut m = Models::new(Some(dir.clone()));
            let d = n.criar(&mut m);
            assert_eq!(d.views.len(), 3, "frente, superior e lateral");
            let (f, s, l) = (&d.views[0], &d.views[1], &d.views[2]);
            assert!(ESCALAS_AUTO.contains(&f.scale), "escala de prateleira: {}", f.scale);
            assert!(s.scale == f.scale && l.scale == f.scale);
            match proj {
                Projecao::Primeiro => assert!(s.pos[1] < f.pos[1], "superior abaixo da frente no 1º diedro"),
                Projecao::Terceiro => assert!(s.pos[1] > f.pos[1], "superior acima da frente no 3º diedro"),
            }
            assert!(l.pos[0] > f.pos[0], "lateral à direita da frente");
            assert!((s.pos[0] - f.pos[0]).abs() < 1e-9 && (l.pos[1] - f.pos[1]).abs() < 1e-9, "alinhadas à frente");
            assert!(matches!(&s.kind, ViewKind::Projected { dir, .. } if *dir == proj.vistas_da_frente().0));
            assert!(matches!(&l.kind, ViewKind::Projected { dir, .. } if *dir == proj.vistas_da_frente().1));
            // todas as vistas dentro da folha, sem se sobrepor
            let tam = n.tamanho();
            let mut caixas = vec![];
            for v in &d.views {
                let p = m.proj(&d, v.id);
                assert!(p.error.is_none(), "{:?}", p.error);
                let (lo, hi) = layout::view_rect(v, &p);
                assert!(lo[0] > 0. && lo[1] > 0. && hi[0] < tam[0] && hi[1] < tam[1], "{lo:?} {hi:?} fora da folha {tam:?}");
                caixas.push((lo, hi));
            }
            for i in 0..3 {
                for j in i + 1..3 {
                    let (a, b) = (caixas[i], caixas[j]);
                    let separadas = a.1[0] <= b.0[0] || b.1[0] <= a.0[0] || a.1[1] <= b.0[1] || b.1[1] <= a.0[1];
                    assert!(separadas, "vistas {i} e {j} se sobrepõem: {a:?} {b:?}");
                }
            }
        }
        // escala geral escolhida vale em vez da automática
        let mut n = NovoDesenho::padrao("Placa", Some(src));
        n.escala = Some(0.5);
        let d = n.criar(&mut Models::new(Some(dir)));
        assert!(d.views.iter().all(|v| v.scale == 0.5));
    }
}
