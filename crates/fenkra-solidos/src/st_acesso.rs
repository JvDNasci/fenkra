// SPDX-License-Identifier: LGPL-2.1-or-later
//! B136: escada de degraus, plataforma e guarda-corpo gerados por parâmetros e conferidos contra o
//! Anexo III da NR-12 (meios de acesso a máquinas e equipamentos, redação vigente). O gerador só
//! calcula linhas guia e perfis; a inserção põe no documento linhas de referência e comandos Membro
//! comuns, que depois se editam como qualquer outro membro e entram na lista de corte.
//!
//! Referencial local: origem no piso, no meio da largura, na frente do primeiro degrau; +Y é o sentido
//! de subida, +Z para cima, a largura ao longo de X.
//!
//! O que a NR-12 manda (itens do Anexo III) e é conferido aqui:
//! - 7 c: travessão superior de 1,10 m a 1,20 m do piso, dos dois lados;
//! - 7 d: travessão superior sem superfície plana (tubo ou barra redonda);
//! - 7 e: rodapé de no mínimo 0,20 m e travessão intermediário a 0,70 m do piso;
//! - 10 a: passarela e plataforma com largura útil de no mínimo 0,60 m;
//! - 11 (sem espelho): largura de 0,60 m (0,50 m em lance único abaixo de 1,50 m), profundidade de no
//!   mínimo 0,15 m, altura entre degraus de no máximo 0,25 m, 600 ≤ g + 2h ≤ 660 mm e plataforma de
//!   descanso de 0,60 m a cada 3,00 m de altura, no máximo;
//! - 12 (com espelho): largura igual à do 11, profundidade de no mínimo 0,20 m, altura entre degraus de
//!   0,20 m a 0,25 m e o mesmo descanso.
//!
//! A NBR 14718 (guarda-corpo de edificação) só entra pela altura mínima de 1,10 m, que é pública. Os
//! demais requisitos dela (vãos, parte escalável, cargas de ensaio) dependem do texto da norma, que não
//! é público: o relatório diz que não foram conferidos.

use super::*;

/// Escada com ou sem espelho (itens 12 e 11 do Anexo III).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TipoEscada {
    ComEspelho,
    SemEspelho,
}

/// Norma do guarda-corpo.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NormaGuarda {
    /// meio de acesso a máquina (NR-12, Anexo III, item 7)
    Nr12,
    /// guarda-corpo de edificação (NBR 14718): só a altura mínima é conferida
    Nbr14718,
}

/// Perfil de uma peça gerada (as cotas na ordem de `structural::dim_names`).
#[derive(Clone, Debug, PartialEq)]
pub struct Perfil {
    pub familia: String,
    pub designacao: String,
    pub kind: SectionKind,
    pub dims: Vec<f64>,
}

impl Perfil {
    pub fn novo(familia: &str, designacao: &str, kind: SectionKind, dims: &[f64]) -> Self {
        Perfil { familia: familia.into(), designacao: designacao.into(), kind, dims: dims.to_vec() }
    }
    /// Lado da seção na horizontal (largura) e na vertical (altura), como o membro a coloca.
    fn wh(&self) -> (f64, f64) {
        structural::section(self.kind, &self.dims).map(|(_, w, h, _)| (w, h)).unwrap_or((0., 0.))
    }
}

/// Guarda-corpo: alturas a partir do piso (ou da linha dos bocéis, na escada) e perfis.
#[derive(Clone, Debug, PartialEq)]
pub struct GuardaCorpo {
    pub norma: NormaGuarda,
    /// altura do topo do travessão superior
    pub altura: f64,
    /// altura do centro do travessão intermediário
    pub intermediario: f64,
    /// altura do rodapé
    pub rodape: f64,
    /// distância máxima entre montantes (escolha de projeto: a NR-12 não fixa)
    pub vao_montantes: f64,
    pub montante: Perfil,
    pub travessao: Perfil,
    /// espessura da barra do rodapé
    pub espessura_rodape: f64,
}

impl Default for GuardaCorpo {
    fn default() -> Self {
        let tubo = Perfil::novo("Tubo redondo", "33,7x2,6", SectionKind::TuboRedondo, &[33.7, 2.6]);
        GuardaCorpo { norma: NormaGuarda::Nr12, altura: 1100., intermediario: 700., rodape: 200., vao_montantes: 1500., montante: tubo.clone(), travessao: tubo, espessura_rodape: 4.76 }
    }
}

/// Parâmetros da escada.
#[derive(Clone, Debug, PartialEq)]
pub struct Escada {
    pub tipo: TipoEscada,
    /// desnível a vencer, do piso ao piso de chegada
    pub altura: f64,
    /// largura útil (entre as longarinas)
    pub largura: f64,
    /// altura entre degraus desejada; o gerador divide o desnível em partes iguais até ela
    pub espelho_max: f64,
    /// profundidade livre do degrau (g); `None` = pela fórmula 600 ≤ g + 2h ≤ 660, no meio da faixa
    pub piso: Option<f64>,
    /// comprimento da plataforma de descanso entre lances
    pub descanso: f64,
    /// altura máxima de um lance antes de um descanso (o Anexo III pede no máximo 3,00 m)
    pub lance_max: f64,
    pub longarina: Perfil,
    /// espessura da chapa do degrau
    pub espessura_degrau: f64,
    /// guarda-corpo dos dois lados (`None` = sem guarda-corpo)
    pub guarda: Option<GuardaCorpo>,
}

impl Default for Escada {
    fn default() -> Self {
        Escada {
            tipo: TipoEscada::ComEspelho,
            altura: 2000.,
            largura: 800.,
            espelho_max: 220.,
            piso: None,
            descanso: 800.,
            lance_max: 3000.,
            longarina: Perfil::novo("Barra chata", "200x9,5", SectionKind::BarraChata, &[9.5, 200.]),
            espessura_degrau: 6.35,
            guarda: Some(GuardaCorpo::default()),
        }
    }
}

/// Medidas resolvidas da escada.
#[derive(Clone, Debug, PartialEq)]
pub struct Dimensoes {
    /// número de alturas entre degraus (espelhos) no total
    pub espelhos: usize,
    /// altura entre degraus (h)
    pub h: f64,
    /// profundidade livre (g)
    pub g: f64,
    /// espelhos de cada lance
    pub lances: Vec<usize>,
    /// inclinação em graus
    pub angulo: f64,
    /// comprimento em planta, do primeiro bocel até a chegada
    pub comprimento: f64,
}

/// Item da norma que a escada ou o guarda-corpo não atende (ou não pôde ser conferido).
#[derive(Clone, Debug, PartialEq)]
pub struct Aviso {
    /// "NR-12 Anexo III 12 d", por exemplo
    pub item: String,
    pub texto: String,
    /// `false` = não conferido (informação), `true` = fora da norma
    pub fora: bool,
}

fn aviso(item: &str, texto: String) -> Aviso {
    Aviso { item: item.into(), texto, fora: true }
}

/// Divide o desnível em espelhos iguais e acha a profundidade pela fórmula do item 11 g.
pub fn dimensionar(e: &Escada) -> Result<Dimensoes, String> {
    if !(e.altura > 0. && e.largura > 0. && e.espelho_max > 0.) {
        return Err("altura, largura e espelho precisam ser positivos".into());
    }
    let espelhos = ((e.altura / e.espelho_max) - 1e-9).ceil().max(1.) as usize;
    let h = e.altura / espelhos as f64;
    // g + 2h no meio da faixa de 600 a 660 mm
    let g = e.piso.unwrap_or_else(|| {
        let min = if e.tipo == TipoEscada::ComEspelho { 200. } else { 150. };
        (630. - 2. * h).max(min)
    });
    if g <= 0. {
        return Err("profundidade do degrau precisa ser positiva".into());
    }
    let por_lance = ((e.lance_max / h) + 1e-9).floor().max(1.) as usize;
    let n_lances = espelhos.div_ceil(por_lance);
    let base = espelhos / n_lances;
    let resto = espelhos % n_lances;
    let lances: Vec<usize> = (0..n_lances).map(|i| base + usize::from(i < resto)).collect();
    // em planta: de cada lance só contam os pisos (um a menos que os espelhos), mais os descansos
    let comprimento = lances.iter().map(|m| (*m as f64 - 1.) * g).sum::<f64>() + (n_lances - 1) as f64 * (e.descanso + g);
    Ok(Dimensoes { espelhos, h, g, lances, angulo: (h / g).atan().to_degrees(), comprimento })
}

/// Confere a escada e o guarda-corpo contra o Anexo III da NR-12.
pub fn verificar(e: &Escada, d: &Dimensoes) -> Vec<Aviso> {
    let mut v = vec![];
    let it = if e.tipo == TipoEscada::ComEspelho { "12" } else { "11" };
    let unico_baixo = d.lances.len() == 1 && e.altura < 1500.;
    let larg_min = if unico_baixo { 500. } else { 600. };
    if e.largura < larg_min - 1e-6 {
        v.push(aviso(&format!("NR-12 Anexo III {it} a"), format!("largura útil {:.0} mm abaixo de {larg_min:.0} mm", e.largura)));
    }
    match e.tipo {
        TipoEscada::ComEspelho => {
            if d.g < 200. - 1e-6 {
                v.push(aviso("NR-12 Anexo III 12 b", format!("profundidade {:.0} mm abaixo de 200 mm", d.g)));
            }
            if d.h < 200. - 1e-6 || d.h > 250. + 1e-6 {
                v.push(aviso("NR-12 Anexo III 12 d", format!("altura entre degraus {:.1} mm fora de 200 a 250 mm", d.h)));
            }
        }
        TipoEscada::SemEspelho => {
            if d.g < 150. - 1e-6 {
                v.push(aviso("NR-12 Anexo III 11 b", format!("profundidade {:.0} mm abaixo de 150 mm", d.g)));
            }
            if d.h > 250. + 1e-6 {
                v.push(aviso("NR-12 Anexo III 11 d", format!("altura entre degraus {:.1} mm acima de 250 mm", d.h)));
            }
            let b = d.g + 2. * d.h;
            if !(600. - 1e-6..=660. + 1e-6).contains(&b) {
                v.push(aviso("NR-12 Anexo III 11 g", format!("g + 2h = {b:.0} mm fora de 600 a 660 mm")));
            }
        }
    }
    let maior = d.lances.iter().copied().max().unwrap_or(0) as f64 * d.h;
    if maior > 3000. + 1e-6 {
        v.push(aviso(&format!("NR-12 Anexo III {it} e"), format!("lance de {maior:.0} mm sem plataforma de descanso (máximo 3000 mm)")));
    }
    if d.lances.len() > 1 && e.descanso < 600. - 1e-6 {
        v.push(aviso(&format!("NR-12 Anexo III {it} e"), format!("plataforma de descanso de {:.0} mm abaixo de 600 mm", e.descanso)));
    }
    match &e.guarda {
        Some(gc) => v.extend(verificar_guarda(gc)),
        None => v.push(aviso("NR-12 Anexo III 7", "meio de acesso sem sistema de proteção contra quedas".into())),
    }
    v
}

/// Confere só o guarda-corpo (também serve para plataforma e passarela).
pub fn verificar_guarda(gc: &GuardaCorpo) -> Vec<Aviso> {
    let mut v = vec![];
    match gc.norma {
        NormaGuarda::Nr12 => {
            if gc.altura < 1100. - 1e-6 || gc.altura > 1200. + 1e-6 {
                v.push(aviso("NR-12 Anexo III 7 c", format!("travessão superior a {:.0} mm, fora de 1100 a 1200 mm", gc.altura)));
            }
            if !matches!(gc.travessao.kind, SectionKind::TuboRedondo | SectionKind::BarraRedonda) {
                v.push(aviso("NR-12 Anexo III 7 d", "travessão superior com superfície plana: use tubo ou barra redonda".into()));
            }
            if gc.rodape < 200. - 1e-6 {
                v.push(aviso("NR-12 Anexo III 7 e", format!("rodapé de {:.0} mm abaixo de 200 mm", gc.rodape)));
            }
            if (gc.intermediario - 700.).abs() > 1e-6 {
                v.push(aviso("NR-12 Anexo III 7 e", format!("travessão intermediário a {:.0} mm; a norma pede 700 mm", gc.intermediario)));
            }
        }
        NormaGuarda::Nbr14718 => {
            if gc.altura < 1100. - 1e-6 {
                v.push(aviso("NBR 14718", format!("altura de {:.0} mm abaixo de 1100 mm", gc.altura)));
            }
            v.push(Aviso { item: "NBR 14718".into(), texto: "vãos, parte escalável e cargas de ensaio não conferidos".into(), fora: false });
        }
    }
    if gc.intermediario >= gc.altura || gc.intermediario <= gc.rodape {
        v.push(aviso("NR-12 Anexo III 7 e", "o travessão intermediário precisa ficar entre o rodapé e o superior".into()));
    }
    v
}

/// Peça gerada: um comando Membro com uma ou mais linhas guia (polilinhas: trechos seguidos
/// dividem as pontas e ganham canto em meia-esquadria).
#[derive(Clone, Debug, PartialEq)]
pub struct Peca {
    /// nome do comando na árvore
    pub papel: String,
    pub perfil: Perfil,
    /// ancoragem da seção (0 centro, 2 topo no meio, 6 base no meio)
    pub ancora: u8,
    pub linhas: Vec<Vec<V3>>,
}

fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn soma(a: V3, b: V3, k: f64) -> V3 {
    [a[0] + b[0] * k, a[1] + b[1] * k, a[2] + b[2] * k]
}

/// Normal horizontal à direita do sentido de `d`.
fn direita(d: V3) -> V3 {
    let n = [d[1], -d[0], 0.];
    let l = (n[0] * n[0] + n[1] * n[1]).sqrt();
    if l < 1e-12 {
        [1., 0., 0.]
    } else {
        [n[0] / l, n[1] / l, 0.]
    }
}

/// Desloca a polilinha `k` mm para fora (direita ou esquerda) e `dz` para cima, com meia-esquadria
/// nos vértices.
fn deslocar(pts: &[V3], fora_direita: bool, k: f64, dz: f64) -> Vec<V3> {
    let s = if fora_direita { 1. } else { -1. };
    (0..pts.len())
        .map(|i| {
            let a = if i > 0 { Some(direita(sub(pts[i], pts[i - 1]))) } else { None };
            let b = if i + 1 < pts.len() { Some(direita(sub(pts[i + 1], pts[i]))) } else { None };
            let n = match (a, b) {
                (Some(a), Some(b)) => {
                    let m = [a[0] + b[0], a[1] + b[1], 0.];
                    let l = (m[0] * m[0] + m[1] * m[1]).sqrt().max(1e-12);
                    let c = (m[0] * a[0] + m[1] * a[1]) / l;
                    [m[0] / l / c.max(0.2), m[1] / l / c.max(0.2), 0.]
                }
                (Some(n), None) | (None, Some(n)) => n,
                _ => [1., 0., 0.],
            };
            let p = soma(pts[i], n, s * k);
            [p[0], p[1], p[2] + dz]
        })
        .collect()
}

/// Desloca a polilinha `k` mm na direção "para cima" de cada trecho (perpendicular a ele, no plano
/// vertical que o contém), com meia-esquadria nos vértices. Serve para a longarina, que corre abaixo
/// da linha dos bocéis sem coincidir com o rodapé (linhas iguais em membros diferentes viram canto).
fn deslocar_acima(pts: &[V3], k: f64) -> Vec<V3> {
    let cima = |a: V3, b: V3| -> V3 {
        let d = sub(b, a);
        let l = (d.iter().map(|c| c * c).sum::<f64>()).sqrt().max(1e-12);
        let d = [d[0] / l, d[1] / l, d[2] / l];
        let u = [-d[2] * d[0], -d[2] * d[1], 1. - d[2] * d[2]];
        let n = (u.iter().map(|c| c * c).sum::<f64>()).sqrt().max(1e-12);
        [u[0] / n, u[1] / n, u[2] / n]
    };
    (0..pts.len())
        .map(|i| {
            let a = (i > 0).then(|| cima(pts[i - 1], pts[i]));
            let b = (i + 1 < pts.len()).then(|| cima(pts[i], pts[i + 1]));
            let n = match (a, b) {
                (Some(a), Some(b)) => {
                    let m = [a[0] + b[0], a[1] + b[1], a[2] + b[2]];
                    let l = (m.iter().map(|c| c * c).sum::<f64>()).sqrt().max(1e-12);
                    let c = (m[0] * a[0] + m[1] * a[1] + m[2] * a[2]) / l;
                    [m[0] / l / c.max(0.2), m[1] / l / c.max(0.2), m[2] / l / c.max(0.2)]
                }
                (Some(n), None) | (None, Some(n)) => n,
                _ => [0., 0., 1.],
            };
            soma(pts[i], n, k)
        })
        .collect()
}

/// Guarda-corpo ao longo de `piso` (polilinha na altura do piso ou da linha dos bocéis): rodapé com a
/// base sobre a linha, montantes do lado de fora encostados no rodapé, travessão superior sobre os
/// montantes e intermediário do lado de fora deles.
pub fn guarda_corpo(piso: &[V3], fora_direita: bool, gc: &GuardaCorpo, prefixo: &str) -> Vec<Peca> {
    if piso.len() < 2 {
        return vec![];
    }
    let (dm, _) = gc.montante.wh();
    let (dt, ht) = gc.travessao.wh();
    let t = gc.espessura_rodape;
    let mut out = vec![];
    let rodape = Perfil::novo("Barra chata", &format!("{:.0}x{}", gc.rodape, fmt_mm(t)), SectionKind::BarraChata, &[t, gc.rodape]);
    out.push(Peca { papel: format!("{prefixo}rodapé"), perfil: rodape, ancora: 6, linhas: vec![piso.to_vec()] });
    // montantes: nas pontas de cada trecho e no meio, a no máximo `vao_montantes`
    let eixo = deslocar(piso, fora_direita, t / 2. + dm / 2., 0.);
    let mut pes: Vec<V3> = vec![];
    for (j, w) in eixo.windows(2).enumerate() {
        // o número de vãos sai do trecho da borda, não do eixo deslocado (que cresce nos cantos)
        let l = (sub(piso[j + 1], piso[j]).iter().map(|c| c * c).sum::<f64>()).sqrt();
        let n = (l / gc.vao_montantes.max(1.) - 1e-9).ceil().max(1.) as usize;
        for k in 0..=n {
            let p = soma(w[0], sub(w[1], w[0]), k as f64 / n as f64);
            if !pes.iter().any(|q| (sub(*q, p).iter().map(|c| c * c).sum::<f64>()).sqrt() < 1.) {
                pes.push(p);
            }
        }
    }
    // o pé desce 150 mm para soldar na lateral da longarina ou da borda da plataforma
    let topo = gc.altura - ht;
    let montantes: Vec<Vec<V3>> = pes.iter().map(|p| vec![[p[0], p[1], p[2] - 150.], [p[0], p[1], p[2] + topo]]).collect();
    out.push(Peca { papel: format!("{prefixo}montantes"), perfil: gc.montante.clone(), ancora: 0, linhas: montantes });
    out.push(Peca { papel: format!("{prefixo}travessão superior"), perfil: gc.travessao.clone(), ancora: 0, linhas: vec![deslocar(piso, fora_direita, t / 2. + dm / 2., gc.altura - ht / 2.)] });
    out.push(Peca { papel: format!("{prefixo}travessão intermediário"), perfil: gc.travessao.clone(), ancora: 0, linhas: vec![deslocar(piso, fora_direita, t / 2. + dm + dt / 2., gc.intermediario)] });
    out
}

fn fmt_mm(v: f64) -> String {
    let t = format!("{:.2}", v);
    t.trim_end_matches('0').trim_end_matches('.').replace('.', ",")
}

/// Linha dos bocéis de um lado da escada: do piso até a chegada, com os descansos na horizontal.
pub fn linha_dos_bocéis(e: &Escada, d: &Dimensoes, x: f64) -> Vec<V3> {
    let mut pts = vec![[x, -d.g, 0.]];
    let (mut y, mut z) = (-d.g, 0.);
    for (i, m) in d.lances.iter().enumerate() {
        y += *m as f64 * d.g;
        z += *m as f64 * d.h;
        pts.push([x, y, z]);
        if i + 1 < d.lances.len() {
            y += e.descanso;
            pts.push([x, y, z]);
        }
    }
    pts
}

/// Peças da escada: longarinas, degraus, descansos e o guarda-corpo dos dois lados.
pub fn pecas_escada(e: &Escada) -> Result<(Dimensoes, Vec<Peca>), String> {
    let d = dimensionar(e)?;
    let (ts, hs) = e.longarina.wh();
    let xl = e.largura / 2. + ts / 2.;
    let mut out = vec![];
    let lados = [linha_dos_bocéis(e, &d, -xl), linha_dos_bocéis(e, &d, xl)];
    // a longarina fica logo abaixo da linha dos bocéis (topo nela), pelo eixo deslocado meia altura
    let longarinas = lados.iter().map(|l| deslocar_acima(l, -hs / 2.)).collect();
    out.push(Peca { papel: "Escada: longarinas".into(), perfil: e.longarina.clone(), ancora: 0, linhas: longarinas });
    // degraus: chapa entre as longarinas, com o topo na altura do degrau e o bocel na frente
    let chapa = Perfil::novo("Barra chata", &format!("{:.0}x{}", d.g, fmt_mm(e.espessura_degrau)), SectionKind::BarraChata, &[d.g, e.espessura_degrau]);
    let mut degraus = vec![];
    let mut descansos = vec![];
    let (mut y0, mut z0) = (0., 0.);
    for (i, m) in d.lances.iter().enumerate() {
        for k in 1..*m {
            let y = y0 + (k as f64 - 1.) * d.g + d.g / 2.;
            let z = z0 + k as f64 * d.h;
            degraus.push(vec![[-e.largura / 2., y, z], [e.largura / 2., y, z]]);
        }
        let ys = y0 + (*m as f64 - 1.) * d.g;
        z0 += *m as f64 * d.h;
        if i + 1 < d.lances.len() {
            descansos.push(vec![[-e.largura / 2., ys + e.descanso / 2., z0], [e.largura / 2., ys + e.descanso / 2., z0]]);
            y0 = ys + e.descanso + d.g;
        }
    }
    if !degraus.is_empty() {
        out.push(Peca { papel: "Escada: degraus".into(), perfil: chapa, ancora: 2, linhas: degraus });
    }
    if !descansos.is_empty() {
        let p = Perfil::novo("Barra chata", &format!("{:.0}x{}", e.descanso, fmt_mm(e.espessura_degrau)), SectionKind::BarraChata, &[e.descanso, e.espessura_degrau]);
        out.push(Peca { papel: "Escada: descansos".into(), perfil: p, ancora: 2, linhas: descansos });
    }
    if let Some(gc) = &e.guarda {
        // o rodapé fica sobre a longarina, no mesmo plano dela
        let gc = GuardaCorpo { espessura_rodape: ts, ..gc.clone() };
        out.extend(guarda_corpo(&lados[0], false, &gc, "Guarda-corpo esquerdo: "));
        out.extend(guarda_corpo(&lados[1], true, &gc, "Guarda-corpo direito: "));
    }
    Ok((d, out))
}

/// Gira em torno de Z por `graus` e leva para `origem`.
pub fn posicionar(p: V3, origem: V3, graus: f64) -> V3 {
    let (s, c) = graus.to_radians().sin_cos();
    [origem[0] + p[0] * c - p[1] * s, origem[1] + p[0] * s + p[1] * c, origem[2] + p[2]]
}

/// Põe as peças no documento: linhas de referência com pontas fixas e um comando Membro por peça.
/// Devolve os comandos criados.
pub fn inserir(doc: &mut Document, pecas: &[Peca], origem: V3, graus: f64) -> Vec<Id> {
    let mut ids = vec![];
    for p in pecas {
        let mut paths = vec![];
        for l in &p.linhas {
            let q: Vec<V3> = l.iter().map(|v| posicionar(*v, origem, graus)).collect();
            for w in q.windows(2) {
                paths.push(CurveRef::Ref(doc.add_geometry(RefKind::LinePoints { a: PointRef::Fixed(w[0]), b: PointRef::Fixed(w[1]), start: "0".into(), end: "0".into() })));
            }
        }
        let canto = if p.linhas.iter().any(|l| l.len() > 2) { CornerMode::MeiaEsquadria } else { CornerMode::Nenhum };
        let m = StructMember {
            paths,
            family: p.perfil.familia.clone(),
            designation: p.perfil.designacao.clone(),
            kind: p.perfil.kind,
            dims: p.perfil.dims.iter().map(|v| format!("{v}")).collect(),
            anchor: p.ancora,
            angle: "0".into(),
            off_x: "0".into(),
            off_y: "0".into(),
            ext1: "0".into(),
            ext2: "0".into(),
            corner: canto,
            gap: "0".into(),
            joints: vec![],
            material: String::new(),
            mirror: false,
        };
        let id = doc.add_feature(FeatureKind::StructMember(m));
        if let Some(f) = doc.feature_mut(id) {
            f.name = p.papel.clone();
        }
        ids.push(id);
    }
    ids
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn desnivel_dividido_em_espelhos_iguais_dentro_da_nr12() {
        let e = Escada { altura: 2000., ..Default::default() };
        let d = dimensionar(&e).unwrap();
        // 2000 / 220 → 10 espelhos de 200 mm; g = 630 − 400 = 230
        assert_eq!(d.espelhos, 10);
        assert!((d.h - 200.).abs() < 1e-9 && (d.g - 230.).abs() < 1e-9, "{d:?}");
        assert_eq!(d.lances, vec![10]);
        assert!(verificar(&e, &d).is_empty(), "{:?}", verificar(&e, &d));
    }

    #[test]
    fn acima_de_tres_metros_ganha_descanso() {
        let e = Escada { altura: 4400., ..Default::default() };
        let d = dimensionar(&e).unwrap();
        assert_eq!(d.espelhos, 20);
        assert_eq!(d.lances, vec![10, 10]);
        assert!(verificar(&e, &d).is_empty());
        let l = linha_dos_bocéis(&e, &d, 0.);
        assert_eq!(l.len(), 4);
        assert!((l[3][2] - 4400.).abs() < 1e-9 && (l[1][2] - l[2][2]).abs() < 1e-9);
        // sem descanso, o lance único passa de 3 m
        let sem = Escada { lance_max: 10000., ..e };
        let d = dimensionar(&sem).unwrap();
        assert!(verificar(&sem, &d).iter().any(|a| a.item == "NR-12 Anexo III 12 e"));
    }

    #[test]
    fn regras_do_anexo_iii_reprovam_o_que_esta_fora() {
        let e = Escada { tipo: TipoEscada::SemEspelho, altura: 1000., largura: 450., espelho_max: 260., piso: Some(140.), guarda: None, ..Default::default() };
        let d = dimensionar(&e).unwrap();
        let itens: Vec<String> = verificar(&e, &d).into_iter().map(|a| a.item).collect();
        // 1000/260 → 4 espelhos de 250: altura no limite; g = 140 < 150; g + 2h = 640 dentro
        for i in ["NR-12 Anexo III 11 a", "NR-12 Anexo III 11 b", "NR-12 Anexo III 7"] {
            assert!(itens.iter().any(|x| x == i), "falta {i} em {itens:?}");
        }
        assert!(!itens.iter().any(|x| x == "NR-12 Anexo III 11 d" || x == "NR-12 Anexo III 11 g"), "{itens:?}");
        let gc = GuardaCorpo { altura: 1000., rodape: 150., intermediario: 600., travessao: Perfil::novo("Tubo quadrado", "30x2", SectionKind::TuboQuadrado, &[30., 2.]), ..Default::default() };
        let v: Vec<String> = verificar_guarda(&gc).into_iter().map(|a| a.item).collect();
        assert_eq!(v.iter().filter(|x| *x == "NR-12 Anexo III 7 e").count(), 2, "{v:?}");
        assert!(v.contains(&"NR-12 Anexo III 7 c".to_string()) && v.contains(&"NR-12 Anexo III 7 d".to_string()));
        let nbr = GuardaCorpo { norma: NormaGuarda::Nbr14718, ..Default::default() };
        let v = verificar_guarda(&nbr);
        assert!(v.iter().all(|a| !a.fora), "{v:?}");
    }

    #[test]
    fn montantes_no_vao_maximo_e_nas_pontas() {
        let gc = GuardaCorpo::default();
        let p = guarda_corpo(&[[0., 0., 0.], [3000., 0., 0.], [3000., 2000., 0.]], true, &gc, "");
        let m = p.iter().find(|x| x.papel == "montantes").unwrap();
        // 3000/1500 → 3 pés; 2000/1500 → 2 vãos, 3 pés, um em comum
        assert_eq!(m.linhas.len(), 5);
        let topo = p.iter().find(|x| x.papel == "travessão superior").unwrap();
        // centro do travessão a 1100 − 33,7/2 do piso
        assert!((topo.linhas[0][0][2] - (1100. - 33.7 / 2.)).abs() < 1e-9);
    }
}
