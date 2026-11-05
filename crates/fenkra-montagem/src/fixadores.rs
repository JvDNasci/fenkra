// SPDX-License-Identifier: LGPL-2.1-or-later
//! Fixadores inteligentes (B29): preenche os furos de uma montagem com parafuso, arruelas e porca
//! da biblioteca pública (`fenkra_solidos::biblioteca`).
//!
//! Como funciona:
//! 1. cada peça visível dá a sua `hole_table`; furos passantes cujo diâmetro casa com a folga
//!    escolhida de um tamanho métrico (±0,05 mm) e furos roscados ("M6", "M8x1.25"...) viram
//!    candidatos;
//! 2. candidatos de peças diferentes com eixos colineares no mundo (0,05 mm / 0,5°) formam a
//!    **pilha** que um mesmo parafuso atravessa;
//! 3. a pega é a distância entre a boca de entrada e a saída da pilha ao longo do eixo; o
//!    comprimento é a pega + arruelas + porca + 2 passos, arredondado para cima na série da
//!    família (em furo roscado: a profundidade útil arredondada para baixo, sem porca);
//! 4. as peças do fixador são gravadas em `<pasta>/fixadores/` (reaproveitadas se já existirem) e
//!    entram na montagem já na posição final, com restrições de eixo (concêntrico) e de contato;
//! 5. eixo e contato juntos tiram 5 dos 6 graus de liberdade: sobra o giro em torno do próprio eixo,
//!    que nenhuma das duas enxerga. Por isso cada peça da pilha ganha uma **trava de giro** (B62),
//!    uma restrição de paralelismo entre dois planos que contêm o eixo. A trava é uma restrição
//!    comum: suprimir na árvore devolve o giro; `travar_giro: false` nem a cria.
//!
//! Convenção do fixador (ver `fenkra_solidos::fixadores`): eixo Z, face de apoio em z = 0, corpo
//! para -Z. Com `u` o sentido de entrada do parafuso (da boca para a saída), o parafuso e as
//! peças do lado da porca têm Z = -u e a arruela sob a cabeça tem Z = +u (o corpo dela fica do lado
//! de fora da peça).

use crate::load::{full_path, PartCache};
use crate::*;
use fenkra_core::{Document, PartType};
use fenkra_kernel::Shape;
use fenkra_solidos::biblioteca::{self, Biblioteca, Familia, Tamanho};
use fenkra_solidos::fixadores::{gerar, situacao_do_selo, SituacaoSelo};
use fenkra_solidos::{prof_escareado, FormaFuro};
use fenkra_solidos::hardware;
use std::collections::HashMap;
use std::path::Path;

/// Tolerância de diâmetro entre o furo e a folga da tabela (mm).
const TOL_DIAMETRO: f64 = 0.05;
/// Tolerância de distância entre eixos coaxiais (mm).
const TOL_EIXO: f64 = 0.05;
/// Tolerância angular entre eixos paralelos (graus).
const TOL_ANGULO: f64 = 0.5;
/// Nome do grupo da árvore que recebe os fixadores.
const GRUPO: &str = "Fixadores";
/// Nome das restrições que tiram o giro dos fixadores.
const TRAVA: &str = "Trava de giro";

#[derive(Clone, Debug, PartialEq)]
pub struct OpcoesFixadores {
    /// família do parafuso ("iso4762", "iso4017", "din933")
    pub familia_parafuso: String,
    /// arruela sob a cabeça (e, com porca, também sob ela)
    pub arruela: bool,
    pub porca: bool,
    pub familia_porca: String,
    pub familia_arruela: String,
    /// folga do furo de passagem: 0 = fino, 1 = normal, 2 = folgado
    pub folga: u8,
    /// só os furos destes componentes (vazio = todos os visíveis)
    pub componentes: Vec<Id>,
    /// tira o giro em torno do eixo: cada peça da pilha nasce com 0 graus de liberdade
    pub travar_giro: bool,
    /// só analisa: devolve o relatório e as marcas dos furos sem inserir nem gravar nada
    pub so_previa: bool,
    /// comprimento pedido (mm) no lugar do automático; cada pilha usa o valor da série do tamanho
    /// dela mais próximo do pedido (a alça 3D e a lista só oferecem valores existentes)
    pub comprimento: Option<f64>,
}

impl Default for OpcoesFixadores {
    fn default() -> Self {
        OpcoesFixadores { familia_parafuso: "iso4762".into(), arruela: true, porca: true, familia_porca: "iso4032".into(), familia_arruela: "iso7089".into(), folga: 1, componentes: vec![], travar_giro: true, so_previa: false, comprimento: None }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RelatorioFixadores {
    /// parafusos inseridos
    pub fixadores: usize,
    /// ("M6x35", quantidade), na ordem em que apareceram
    pub por_tamanho: Vec<(String, usize)>,
    pub arruelas: usize,
    pub porcas: usize,
    /// pilhas que já tinham um fixador (ou outra peça de hardware) no eixo
    pub pulados: usize,
    /// furos sem tamanho compatível e outros problemas, uma frase cada
    pub avisos: Vec<String>,
    /// uma marca por furo analisado, para a viewport mostrar o estado de cada um
    pub marcas: Vec<MarcaFuro>,
}

/// O que acontece com um furo no preenchimento.
#[derive(Clone, Debug, PartialEq)]
pub enum EstadoFuro {
    /// recebe fixador (ou receberia, na prévia)
    Entra {
        tamanho: String,
        comprimento: f64,
        /// ponta do parafuso no mundo e sentido da cabeça para a ponta (a alça de comprimento fica aí)
        ponta: V3,
        eixo: V3,
        /// comprimentos que existem na família para o tamanho
        serie: Vec<f64>,
    },
    /// o eixo já tem fixador
    JaTem,
    /// não recebe; o texto diz o motivo
    SemTamanho(String),
}

/// Estado de um furo, na posição dele no mundo.
#[derive(Clone, Debug, PartialEq)]
pub struct MarcaFuro {
    pub componente: String,
    /// centro da boca e sentido para dentro do material, no mundo
    pub pos: V3,
    pub dir: V3,
    pub diametro: f64,
    pub estado: EstadoFuro,
}

impl RelatorioFixadores {
    fn conta(&mut self, rotulo: String) {
        self.fixadores += 1;
        match self.por_tamanho.iter_mut().find(|(r, _)| *r == rotulo) {
            Some((_, n)) => *n += 1,
            None => self.por_tamanho.push((rotulo, 1)),
        }
    }
}

/// Furo candidato, já no mundo.
#[derive(Clone, Debug)]
struct Furo {
    comp: Id,
    nome: String,
    /// centro da boca
    boca: V3,
    /// sentido da boca para dentro do material (unitário)
    dir: V3,
    diametro: f64,
    /// comprimento útil ao longo de `dir` a partir da boca (espessura, ou profundidade do roscado cego)
    espessura: f64,
    roscado: bool,
    /// designação do tamanho ("M6"); `None` = passante sem tamanho compatível
    tamanho: Option<String>,
    /// casa com a folga escolhida (ou é roscado): serve de âncora da pilha
    ancora: bool,
    /// forma da boca (rebaixo, cone) e suas medidas, como na tabela de furos
    forma: FormaFuro,
    d_boca: f64,
    prof_boca: f64,
    angulo: f64,
}

impl Furo {
    fn marca(&self, estado: EstadoFuro) -> MarcaFuro {
        MarcaFuro { componente: self.nome.clone(), pos: self.boca, dir: self.dir, diametro: self.diametro, estado }
    }
}

/// Preenche os furos da montagem com fixadores. `dir` é a pasta da montagem (onde nasce
/// `fixadores/`); sem ela não há onde gravar as peças.
pub fn preencher_furos(asm: &mut Assembly, dir: Option<&Path>, cache: &mut PartCache, op: &OpcoesFixadores) -> Result<RelatorioFixadores, String> {
    preencher_com(asm, dir, cache, op, biblioteca::publica())
}

/// Como [`preencher_furos`], com as famílias de `lib` (a pública mesclada com empresa e usuário).
pub fn preencher_com(asm: &mut Assembly, dir: Option<&Path>, cache: &mut PartCache, op: &OpcoesFixadores, lib: &Biblioteca) -> Result<RelatorioFixadores, String> {
    let dir = dir.ok_or("salve a montagem antes: as peças dos fixadores são gravadas na pasta dela")?;
    let fam_p = lib.familia(&op.familia_parafuso).filter(|f| f.tipo.e_parafuso()).ok_or_else(|| format!("família de parafuso desconhecida: {}", op.familia_parafuso))?;
    let fam_arr = lib.familia(&op.familia_arruela);
    let fam_porca = lib.familia(&op.familia_porca);
    let mut rel = RelatorioFixadores::default();

    // 1) furos candidatos de todos os componentes
    let mut furos: Vec<Furo> = vec![];
    let comps: Vec<(Id, String, Placement, String)> = asm
        .nodes
        .iter()
        .filter(|n| n.visible && (op.componentes.is_empty() || op.componentes.contains(&n.id)))
        .filter_map(|n| match &n.kind {
            NodeKind::Part { path } => Some((n.id, n.name.clone(), n.placement, path.clone())),
            _ => None,
        })
        .collect();
    for (id, nome, pl, path) in comps {
        let lp = match cache.get(&full_path(Some(dir), &path), true) {
            Ok(l) => l,
            Err(e) => {
                rel.avisos.push(format!("{nome}: não foi possível ler a peça ({e})"));
                continue;
            }
        };
        for r in fenkra_solidos::hole_table(&lp.doc) {
            let roscado = !r.thread.trim().is_empty();
            let dir_w = unit(pl.dir(r.dir));
            let boca = pl.point(r.pos);
            let (tamanho, ancora) = if roscado {
                match tamanho_da_rosca(fam_p, &r.thread) {
                    Some(t) => (Some(t), true),
                    None => {
                        let m = format!("{nome}: rosca {} sem tamanho na família {}", r.thread.trim(), fam_p.norma);
                        rel.marcas.push(MarcaFuro { componente: nome.clone(), pos: boca, dir: dir_w, diametro: r.diameter, estado: EstadoFuro::SemTamanho(m.clone()) });
                        rel.avisos.push(m);
                        continue;
                    }
                }
            } else if r.through {
                reconhecer(fam_p, r.diameter, op.folga)
            } else {
                // furo cego liso: não recebe fixador
                continue;
            };
            let espessura = if roscado && !r.through {
                Some(r.depth)
            } else {
                lp.shape.as_ref().and_then(|s| espessura_ao_longo(s, r.pos, r.dir, r.diameter))
            };
            let Some(espessura) = espessura.filter(|e| *e > 1e-6) else {
                let m = format!("{nome}: não foi possível medir a espessura do furo Ø{}", num(r.diameter));
                rel.marcas.push(MarcaFuro { componente: nome.clone(), pos: boca, dir: dir_w, diametro: r.diameter, estado: EstadoFuro::SemTamanho(m.clone()) });
                rel.avisos.push(m);
                continue;
            };
            // o rebaixo ou o cone tem de estar centrado no furo; se o centro não bate, não se insere deslocado
            if matches!(r.forma, FormaFuro::Rebaixo | FormaFuro::Escareado) && lp.shape.as_ref().is_some_and(|s| !boca_concentrica(s, &r)) {
                let m = format!("{nome}: a boca do furo Ø{} não é concêntrica com o furo; fixador não inserido", num(r.diameter));
                rel.marcas.push(MarcaFuro { componente: nome.clone(), pos: boca, dir: dir_w, diametro: r.diameter, estado: EstadoFuro::SemTamanho(m.clone()) });
                rel.avisos.push(m);
                continue;
            }
            furos.push(Furo { comp: id, nome: nome.clone(), boca, dir: dir_w, diametro: r.diameter, espessura, roscado, tamanho, ancora, forma: r.forma, d_boca: r.d_boca, prof_boca: r.prof_boca, angulo: r.angulo });
        }
    }

    // 2) pilhas: furos de eixos colineares
    let mut pilhas: Vec<Vec<Furo>> = vec![];
    for f in furos {
        match pilhas.iter_mut().find(|p| colineares(p[0].boca, p[0].dir, f.boca, f.dir)) {
            Some(p) => p.push(f),
            None => pilhas.push(vec![f]),
        }
    }

    let eixos_existentes = eixos_das_restricoes(asm);
    let mut arquivos: HashMap<String, (String, String)> = HashMap::new();
    let mut grupo: Option<Id> = asm.nodes.iter().find(|n| matches!(n.kind, NodeKind::Group) && n.name == GRUPO).map(|n| n.id);

    for mut pilha in pilhas {
        // um membro por componente (o primeiro; prefere o que tem tamanho)
        let mut vistos: Vec<Id> = vec![];
        pilha.sort_by_key(|f| f.tamanho.is_none());
        pilha.retain(|f| {
            let novo = !vistos.contains(&f.comp);
            vistos.push(f.comp);
            novo
        });
        // de volta à ordem da árvore
        let ordem = |c: Id| asm.nodes.iter().position(|n| n.id == c).unwrap_or(usize::MAX);
        pilha.sort_by_key(|f| ordem(f.comp));

        if let Some(f) = pilha.iter().find(|f| f.tamanho.is_none()) {
            let m = format!("{}: furo Ø{} sem tamanho compatível na família {} (folga {})", f.nome, num(f.diametro), fam_p.norma, nome_folga(op.folga));
            recusa(&mut rel, &pilha, m);
            continue;
        }
        let Some(ancora) = pilha.iter().find(|f| f.ancora) else {
            let f = &pilha[0];
            let m = format!("{}: furo Ø{} não casa com a folga {} (é outra folga de {})", f.nome, num(f.diametro), nome_folga(op.folga), f.tamanho.as_deref().unwrap_or("?"));
            recusa(&mut rel, &pilha, m);
            continue;
        };
        let desig = ancora.tamanho.clone().unwrap_or_default();
        if pilha.iter().any(|f| f.tamanho.as_deref() != Some(desig.as_str())) {
            let m = format!("{}: furos coaxiais de tamanhos diferentes ({})", ancora.nome, pilha.iter().map(|f| f.tamanho.clone().unwrap_or_default()).collect::<Vec<_>>().join(", "));
            recusa(&mut rel, &pilha, m);
            continue;
        }
        let ids: Vec<Id> = pilha.iter().map(|f| f.comp).collect();

        // já tem fixador (ou outro hardware) neste eixo?
        let (p0, u) = (pilha[0].boca, pilha[0].dir);
        let ocupado = eixos_existentes.iter().any(|(c, p, d)| !ids.contains(c) && colineares(p0, u, *p, *d) && e_hardware(asm, cache, dir, *c));
        if ocupado {
            rel.pulados += 1;
            rel.marcas.extend(pilha.iter().map(|f| f.marca(EstadoFuro::JaTem)));
            continue;
        }

        let Some(t_p) = fam_p.tamanho(&desig) else {
            let m = format!("{}: {} não tem o tamanho {desig}", ancora.nome, fam_p.norma);
            recusa(&mut rel, &pilha, m);
            continue;
        };
        let passo = t_p.valor("P").unwrap_or(0.);

        // 3) geometria da pilha ao longo de u (u = sentido do furo do primeiro componente)
        let mut ini = (f64::MAX, 0usize, V3::default());
        let mut fim = (f64::MIN, 0usize, V3::default());
        for (i, f) in pilha.iter().enumerate() {
            let s0 = dot(sub(f.boca, p0), u);
            let (a, b) = (f.boca, [0, 1, 2].map(|k| f.boca[k] + f.dir[k] * f.espessura));
            let (sa, sb) = (s0, s0 + dot(f.dir, u) * f.espessura);
            let (lo, pt_lo, hi, pt_hi) = if sa <= sb { (sa, a, sb, b) } else { (sb, b, sa, a) };
            if lo < ini.0 {
                ini = (lo, i, pt_lo);
            }
            if hi > fim.0 {
                fim = (hi, i, pt_hi);
            }
        }
        let (entrada, saida) = (ini.2, fim.2);
        let pega = fim.0 - ini.0;
        let (c_ent, c_sai) = (pilha[ini.1].comp, pilha[fim.1].comp);
        let roscada = pilha.iter().any(|f| f.roscado);
        let com_porca = op.porca && !roscada;

        let esp_arruela = |on: bool| -> Option<f64> { on.then(|| fam_arr.and_then(|f| f.tamanho(&desig)).and_then(|t| t.valor("h"))).flatten() };
        let h1 = esp_arruela(op.arruela);
        let h2 = esp_arruela(op.arruela && com_porca);
        let altura_porca = if com_porca { fam_porca.and_then(|f| f.tamanho(&desig)).and_then(|t| t.valor("m")) } else { None };
        if op.arruela && h1.is_none() {
            rel.avisos.push(format!("arruela {} não existe na família {}: {} segue sem arruela", desig, op.familia_arruela, ancora.nome));
        }
        if com_porca && altura_porca.is_none() {
            rel.avisos.push(format!("porca {} não existe na família {}: {} segue sem porca", desig, op.familia_porca, ancora.nome));
        }
        let com_porca = com_porca && altura_porca.is_some();
        let (w1, w2, m) = (h1.unwrap_or(0.), if com_porca { h2.unwrap_or(0.) } else { 0. }, if com_porca { altura_porca.unwrap_or(0.) } else { 0. });

        // assento: a cabeça (ou a arruela sob ela) desce até o fundo do rebaixo ou do cone da boca de
        // entrada; o mesmo vale para a porca (ou a arruela dela) na boca de saída
        let cos30 = 30_f64.to_radians().cos();
        let d_cabeca = t_p.valor("dk").or_else(|| t_p.valor("s").map(|s| s / cos30)).unwrap_or(0.);
        let d_arruela = fam_arr.and_then(|f| f.tamanho(&desig)).and_then(|t| t.valor("d2"));
        let d_porca = fam_porca.and_then(|f| f.tamanho(&desig)).and_then(|t| t.valor("s")).map(|s| s / cos30);
        let item_ent = if h1.is_some() { d_arruela.unwrap_or(d_cabeca) } else { d_cabeca };
        let item_sai = if com_porca { if h2.is_some() { d_arruela.or(d_porca) } else { d_porca } } else { None };
        let mut recuos = [0.; 2];
        for (k, (f, ponto, item)) in [(&pilha[ini.1], entrada, Some(item_ent)), (&pilha[fim.1], saida, item_sai)].into_iter().enumerate() {
            let Some(item) = item else { continue };
            match assento(f, ponto, item) {
                Ok(z) => recuos[k] = z,
                Err(e) => rel.avisos.push(e),
            }
        }
        let desloca = |o: V3, k: f64| [0, 1, 2].map(|i| o[i] + u[i] * k);
        let (entrada, saida) = (desloca(entrada, recuos[0]), desloca(saida, -recuos[1]));
        let pega = pega - recuos[0] - recuos[1];
        if pega <= 1e-6 {
            let m = format!("{}: o rebaixo come toda a espessura da pilha; nada para o fixador prender", ancora.nome);
            recusa(&mut rel, &pilha, m);
            continue;
        }

        let serie = fam_p.comprimentos_de(t_p);
        let comprimento = if roscada {
            // profundidade útil arredondada para baixo
            match serie.iter().copied().filter(|l| *l <= pega + w1 + 1e-9).last() {
                Some(l) => l,
                None => {
                    let m = format!("{}: profundidade útil de {} mm é menor que o menor {} {desig} da série", ancora.nome, num(pega), fam_p.norma);
                    recusa(&mut rel, &pilha, m);
                    continue;
                }
            }
        } else {
            let precisa = pega + w1 + w2 + m + 2. * passo;
            match serie.iter().copied().find(|l| *l >= precisa - 1e-9) {
                Some(l) => l,
                None => {
                    let Some(l) = serie.last().copied() else { continue };
                    rel.avisos.push(format!("{}: a pega pede L ≥ {} mm, mas {} {desig} vai só até {} mm", ancora.nome, num((precisa * 10.).round() / 10.), fam_p.norma, num(l)));
                    l
                }
            }
        };

        let comprimento = match op.comprimento.and_then(|l| mais_proximo(&serie, l)) {
            Some(l) => {
                if (l - comprimento).abs() > 1e-9 && !roscada && l < pega + w1 + w2 + m {
                    rel.avisos.push(format!("{}: L{} não atravessa a pilha (pega + arruelas + porca = {} mm)", ancora.nome, num(l), num(((pega + w1 + w2 + m) * 10.).round() / 10.)));
                }
                l
            }
            None => comprimento,
        };
        // ponta do parafuso: a cabeça apoia w1 para fora da face de entrada e o corpo segue para dentro
        let ponta = [0, 1, 2].map(|i| entrada[i] + u[i] * (comprimento - w1));
        let entra = EstadoFuro::Entra { tamanho: desig.clone(), comprimento, ponta, eixo: u, serie: serie.clone() };
        if op.so_previa {
            rel.marcas.extend(pilha.iter().map(|f| f.marca(entra.clone())));
            rel.conta(format!("{desig}x{}", num(comprimento)));
            continue;
        }

        // 4) peças e restrições
        let mut velhos: Vec<String> = vec![];
        let mut peca = |fam: &Familia, t: &Tamanho, l: Option<f64>| -> Result<(String, String), String> { arquivo_fixador(dir, fam, t, l, &mut arquivos, &mut velhos) };
        let (arq_parafuso, nome_parafuso) = peca(fam_p, t_p, Some(comprimento))?;
        let pasta = *grupo.get_or_insert_with(|| asm.add_group(GRUPO, None));

        let eixo_mundo = (entrada, [-u[0], -u[1], -u[2]]);
        let (cima, baixo) = (u, [-u[0], -u[1], -u[2]]);
        let ponto = |o: V3, d: V3, k: f64| [0, 1, 2].map(|i| o[i] + d[i] * k);

        // parafuso: Z = -u, com a cabeça apoiada na arruela (ou na face da peça)
        let origem_parafuso = ponto(entrada, baixo, w1);
        let id_parafuso = asm.add_component(&nome_parafuso, NodeKind::Part { path: arq_parafuso }, orientado(origem_parafuso, baixo), Some(pasta));
        // concêntrico ao furo de entrada
        concentrico(asm, c_ent, id_parafuso, eixo_mundo);
        // quem gira junto com quem: o parafuso segue a peça; arruelas e porca seguem o parafuso
        let trava = |asm: &mut Assembly, a: Id, b: Id| {
            if op.travar_giro {
                trava_de_giro(asm, a, b, eixo_mundo);
            }
        };
        trava(asm, c_ent, id_parafuso);
        let face_entrada = (entrada, baixo); // normal de fora da peça
        let arruela_cabeca = fam_arr.zip(h1).and_then(|(f, _)| f.tamanho(&desig).map(|t| (f, t)));
        if let Some((fam_a, t_a)) = arruela_cabeca {
            let (arq, nome) = peca(fam_a, t_a, None)?;
            let id = asm.add_component(&nome, NodeKind::Part { path: arq }, orientado(entrada, cima), Some(pasta));
            concentrico(asm, id_parafuso, id, eixo_mundo);
            trava(asm, id_parafuso, id);
            apoio(asm, c_ent, face_entrada, id, (entrada, cima));
            // cabeça sobre a face de fora da arruela
            let fora = ponto(entrada, baixo, w1);
            apoio(asm, id, (fora, baixo), id_parafuso, (fora, cima));
            rel.arruelas += 1;
        } else {
            apoio(asm, c_ent, face_entrada, id_parafuso, (entrada, cima));
        }

        if let (true, Some(fam_n)) = (com_porca, fam_porca) {
            // (componente de apoio, sua face de fora, ponto da face): a peça ou a arruela da saída
            let mut apoio_porca = (c_sai, (saida, cima), saida);
            let arruela_saida = fam_arr.zip(h2).and_then(|(f, h)| f.tamanho(&desig).map(|t| (f, t, h)));
            if let Some((fam_a, t_a, h)) = arruela_saida {
                let (arq, nome) = peca(fam_a, t_a, None)?;
                let id = asm.add_component(&nome, NodeKind::Part { path: arq }, orientado(saida, baixo), Some(pasta));
                concentrico(asm, id_parafuso, id, eixo_mundo);
                trava(asm, id_parafuso, id);
                apoio(asm, c_sai, (saida, cima), id, (saida, baixo));
                let fora = ponto(saida, cima, h);
                apoio_porca = (id, (fora, cima), fora);
                rel.arruelas += 1;
            }
            if let Some(t_n) = fam_n.tamanho(&desig) {
                let (arq, nome) = peca(fam_n, t_n, None)?;
                let id = asm.add_component(&nome, NodeKind::Part { path: arq }, orientado(apoio_porca.2, baixo), Some(pasta));
                concentrico(asm, id_parafuso, id, eixo_mundo);
                trava(asm, id_parafuso, id);
                apoio(asm, apoio_porca.0, apoio_porca.1, id, (apoio_porca.2, baixo));
                rel.porcas += 1;
            }
        }
        rel.avisos.extend(velhos);
        rel.marcas.extend(pilha.iter().map(|f| f.marca(entra.clone())));
        rel.conta(format!("{desig}x{}", num(comprimento)));
    }
    Ok(rel)
}

/// Placement com o eixo Z do componente apontando para `z` e a origem em `origem`.
fn orientado(origem: V3, z: V3) -> Placement {
    let z = unit(z);
    let x = any_perp(z);
    Placement { origin: origem, rot: [x, cross(z, x), z] }
}

fn eixo_local(a: &Assembly, comp: Id, p: V3, d: V3) -> Geom {
    let pl = a.node(comp).map(|n| n.placement).unwrap_or_default();
    Geom::Axis { p: pl.to_local(p), d: unit(pl.dir_to_local(d)) }
}

fn plano_local(a: &Assembly, comp: Id, p: V3, n: V3) -> Geom {
    let pl = a.node(comp).map(|n| n.placement).unwrap_or_default();
    Geom::Plane { p: pl.to_local(p), n: unit(pl.dir_to_local(n)) }
}

/// Restrição concêntrica entre `a` e `b` pelo eixo (ponto, direção) do mundo.
fn concentrico(asm: &mut Assembly, a: Id, b: Id, eixo: (V3, V3)) {
    let (ga, gb) = (eixo_local(asm, a, eixo.0, eixo.1), eixo_local(asm, b, eixo.0, eixo.1));
    asm.add_mate(MateKind::Coincident { flip: false }, MateRef { comp: a, geom: ga, pick: None }, MateRef { comp: b, geom: gb, pick: None });
}

/// Contato entre a face de `a` (ponto, normal de fora do sólido) e a de `b`, de frente uma para a outra.
fn apoio(asm: &mut Assembly, a: Id, fa: (V3, V3), b: Id, fb: (V3, V3)) {
    let (ga, gb) = (plano_local(asm, a, fa.0, fa.1), plano_local(asm, b, fb.0, fb.1));
    asm.add_mate(MateKind::Contact, MateRef { comp: a, geom: ga, pick: None }, MateRef { comp: b, geom: gb, pick: None });
}

/// Tira o giro de `b` em torno do eixo, mantendo o ângulo de agora em relação a `a`. Paralelismo
/// entre planos que contêm o eixo: o resíduo tem derivada cheia no ponto de partida, o que não
/// acontece com um ângulo de 0° (o cosseno é plano ali, e o giro continuaria contado como livre).
/// Planos, e não eixos, para não confundir `eixos_das_restricoes`, que procura o eixo do fixador.
fn trava_de_giro(asm: &mut Assembly, a: Id, b: Id, eixo: (V3, V3)) {
    let normal = any_perp(unit(eixo.1));
    let (ga, gb) = (plano_local(asm, a, eixo.0, normal), plano_local(asm, b, eixo.0, normal));
    let nome = asm.unique_pub(TRAVA);
    let id = asm.add_mate(MateKind::Parallel, MateRef { comp: a, geom: ga, pick: None }, MateRef { comp: b, geom: gb, pick: None });
    if let Some(m) = asm.mates.iter_mut().find(|m| m.id == id) {
        m.name = nome;
    }
}

/// Grava a peça do fixador (se ainda não existe) e devolve (caminho relativo à montagem, nome).
/// Peça já gravada que a biblioteca atual superou entra em `velhos` (o arquivo não é regravado:
/// a montagem pode ter restrições apoiadas nele).
fn arquivo_fixador(dir: &Path, fam: &Familia, t: &Tamanho, l: Option<f64>, memo: &mut HashMap<String, (String, String)>, velhos: &mut Vec<String>) -> Result<(String, String), String> {
    let base = match l {
        Some(l) => format!("{}_{}x{}", fam.id, t.designacao, num(l)),
        None => format!("{}_{}", fam.id, t.designacao),
    };
    if let Some(r) = memo.get(&base) {
        return Ok(r.clone());
    }
    let doc = gerar(fam, &t.designacao, l)?;
    let rel = format!("fixadores/{base}.fkp");
    let destino = dir.join("fixadores").join(format!("{base}.fkp"));
    if destino.exists() {
        let lida = std::fs::read_to_string(&destino).ok().and_then(|j| Document::from_json(&j).ok());
        if let Some(SituacaoSelo::Desatualizada(porque)) = lida.as_ref().map(situacao_do_selo) {
            velhos.push(format!("{rel}: peça desatualizada ({porque}); apague o arquivo para gerar de novo"));
        }
    } else {
        std::fs::create_dir_all(dir.join("fixadores")).map_err(|e| format!("{}: {e}", dir.join("fixadores").display()))?;
        std::fs::write(&destino, doc.to_json()).map_err(|e| format!("{}: {e}", destino.display()))?;
    }
    let r = (rel, doc.props.description.clone());
    memo.insert(base, r.clone());
    Ok(r)
}

/// Eixos (componente, ponto e direção no mundo) de todas as restrições ativas.
fn eixos_das_restricoes(a: &Assembly) -> Vec<(Id, V3, V3)> {
    let mut v = vec![];
    for m in a.mates.iter().filter(|m| !m.suppressed) {
        for r in [&m.a, &m.b] {
            if let (Geom::Axis { p, d }, Some(n)) = (r.geom, a.node(r.comp)) {
                v.push((r.comp, n.placement.point(p), unit(n.placement.dir(d))));
            }
        }
    }
    v
}

/// O componente é uma peça de hardware (parafuso, pino...)?
fn e_hardware(a: &Assembly, cache: &mut PartCache, dir: &Path, comp: Id) -> bool {
    let Some(NodeKind::Part { path }) = a.node(comp).map(|n| n.kind.clone()) else { return false };
    cache.get(&full_path(Some(dir), &path), false).is_ok_and(|p| p.doc.props.part_type == PartType::Hardware)
}

/// Eixos (ponto e direção) na mesma reta: paralelos ou antiparalelos até 0,5° e a menos de 0,05 mm.
fn colineares(pa: V3, da: V3, pb: V3, db: V3) -> bool {
    if norm(cross(da, db)) > TOL_ANGULO.to_radians().sin() {
        return false;
    }
    let d = sub(pb, pa);
    let k = dot(d, da);
    norm([0, 1, 2].map(|i| d[i] - da[i] * k)) <= TOL_EIXO
}

/// Valor da série mais próximo de `l` (o maior, em empate). `None` com a série vazia.
pub fn mais_proximo(serie: &[f64], l: f64) -> Option<f64> {
    serie.iter().copied().min_by(|a, b| (a - l).abs().total_cmp(&(b - l).abs()).then(b.total_cmp(a)))
}

/// Recusa uma pilha: o aviso vai para a lista e cada furo dela ganha a marca com o motivo.
fn recusa(rel: &mut RelatorioFixadores, pilha: &[Furo], motivo: String) {
    rel.marcas.extend(pilha.iter().map(|f| f.marca(EstadoFuro::SemTamanho(motivo.clone()))));
    rel.avisos.push(motivo);
}

/// Tamanho do furo passante e se ele é âncora da pilha. Primeiro vale a faixa `furo_min..furo_max`
/// declarada pela família (prendedores sem folga de tabela); depois a folga escolhida dos métricos;
/// por fim as outras folgas, que reconhecem o tamanho mas não ancoram.
fn reconhecer(fam: &Familia, diametro: f64, folga: u8) -> (Option<String>, bool) {
    let declarado = fam.tamanhos.iter().find(|t| t.furo_min.is_some() && fam.faixa_furo(t).is_some_and(|(a, b)| diametro >= a - TOL_DIAMETRO && diametro <= b + TOL_DIAMETRO));
    if let Some(t) = declarado {
        return (Some(t.designacao.clone()), true);
    }
    match casa_folga(diametro, folga) {
        Some(t) => (Some(t.to_string()), true),
        None => (casa_outra_folga(diametro).map(str::to_string), false),
    }
}

/// Profundidade em que um item de diâmetro `item` assenta na boca do furo `f` quando a boca da pilha
/// é a dele: no fundo do rebaixo, ou no cone onde o diâmetro dele é igual ao do item. Zero: na face.
/// `Err` quando o item não cabe (assenta na face e o aviso explica).
fn assento(f: &Furo, ponto: V3, item: f64) -> Result<f64, String> {
    if f.forma == FormaFuro::Reto || norm(sub(f.boca, ponto)) > 1e-3 || item <= 0. {
        return Ok(0.);
    }
    match f.forma {
        // contrafuro: o fundo é cônico e a cabeça apoia no ombro do rebaixo
        FormaFuro::Rebaixo | FormaFuro::Contrafuro if item <= f.d_boca + 1e-6 => Ok(f.prof_boca),
        FormaFuro::Escareado if item < f.d_boca - 1e-6 => Ok(prof_escareado(f.d_boca, item.max(f.diametro), f.angulo)),
        _ => Err(format!("{}: o item de Ø{} não cabe na boca de Ø{}; assenta na face", f.nome, num(item), num(f.d_boca))),
    }
}

/// O rebaixo (ou cone) é concêntrico com o furo? Quatro retas paralelas ao eixo descem a meia
/// distância entre a parede do furo e a do rebaixo; todas têm de achar o fundo na profundidade
/// esperada. Rebaixo deslocado deixa algumas retas na face (profundidade zero).
fn boca_concentrica(forma: &Shape, r: &fenkra_solidos::HoleRow) -> bool {
    let dir = unit(r.dir);
    let (e1, e2) = (any_perp(dir), cross(dir, any_perp(dir)));
    let raio = (r.d_boca / 2. + r.diameter / 2.) / 2.;
    let esperado = match r.forma {
        FormaFuro::Rebaixo => r.prof_boca,
        FormaFuro::Escareado => prof_escareado(r.d_boca, 2. * raio, r.angulo),
        _ => return true,
    };
    let fora = 1.;
    for v in [e1, e2, e1.map(|x| -x), e2.map(|x| -x)] {
        let q: V3 = [0, 1, 2].map(|i| r.pos[i] + v[i] * raio - dir[i] * fora);
        // sem acerto: a reta passou direto (peça fina demais ao redor); não prova nada
        if let Ok(Some((pt, _))) = forma.ray_hit(q, dir) {
            if (dot(sub(pt, r.pos), dir) - esperado).abs() > 0.2 {
                return false;
            }
        }
    }
    true
}

/// Tamanho cuja folga escolhida casa com o diâmetro do furo.
fn casa_folga(diametro: f64, folga: u8) -> Option<&'static str> {
    hardware::METRIC.iter().find(|m| (m.clearance[folga.min(2) as usize] - diametro).abs() <= TOL_DIAMETRO).map(|m| m.name)
}

/// Tamanho cuja folga (qualquer uma das três) casa com o diâmetro do furo.
fn casa_outra_folga(diametro: f64) -> Option<&'static str> {
    hardware::METRIC.iter().find(|m| m.clearance.iter().any(|c| (c - diametro).abs() <= TOL_DIAMETRO)).map(|m| m.name)
}

/// Designação do tamanho da família para a rosca do furo: a própria designação livre ("1/4-28 UNF")
/// quando a família a tem; senão o métrico ("M6x1", "M8" → "M6", "M8") da tabela de roscas.
fn tamanho_da_rosca(fam: &Familia, rosca: &str) -> Option<String> {
    if let Some(t) = fam.tamanho(rosca) {
        return Some(t.designacao.clone());
    }
    let r = rosca.trim().strip_prefix(['M', 'm'])?;
    let d: f64 = r.chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect::<String>().parse().ok()?;
    hardware::METRIC.iter().find(|m| m.name[1..].parse::<f64>().is_ok_and(|x| (x - d).abs() < 1e-6)).map(|m| m.name.to_string())
}

/// Distância da boca até a saída do furo, ao longo de `dir` (no sistema local da peça).
/// Sonda a peça com retas paralelas ao eixo, logo ao lado do furo, vindas de fora pelo lado da
/// saída; a mediana dos acertos vence peças com rebaixo ou chanfro num dos lados.
fn espessura_ao_longo(forma: &Shape, boca: V3, dir: V3, diametro: f64) -> Option<f64> {
    let dir = unit(dir);
    let (lo, hi) = forma.bbox()?;
    let mut longe = f64::MIN;
    for c in 0..8 {
        let q = [if c & 1 == 0 { lo[0] } else { hi[0] }, if c & 2 == 0 { lo[1] } else { hi[1] }, if c & 4 == 0 { lo[2] } else { hi[2] }];
        longe = longe.max(dot(sub(q, boca), dir));
    }
    if longe <= 1e-6 {
        return None;
    }
    let (e1, e2) = (any_perp(dir), cross(dir, any_perp(dir)));
    let mut s: Vec<f64> = vec![];
    for folga in [0.4, 0.9, 1.6] {
        for v in [e1, e2, e1.map(|x| -x), e2.map(|x| -x)] {
            let r = diametro / 2. + folga;
            let q: V3 = [0, 1, 2].map(|i| boca[i] + v[i] * r + dir[i] * (longe + 1.));
            if let Ok(Some((pt, _))) = forma.ray_hit(q, dir.map(|x| -x)) {
                let d = dot(sub(pt, boca), dir);
                if d > 0.05 {
                    s.push(d);
                }
            }
        }
    }
    if s.is_empty() {
        // sem acerto (peça fina demais ao redor): a caixa envolvente
        return Some(longe);
    }
    s.sort_by(|a, b| a.total_cmp(b));
    Some(s[s.len() / 2])
}

fn nome_folga(folga: u8) -> &'static str {
    ["fina", "normal", "folgada"][folga.min(2) as usize]
}

/// 35.0 → "35", 1.6 → "1.6"
fn num(x: f64) -> String {
    if (x - x.round()).abs() < 1e-9 {
        format!("{}", x.round() as i64)
    } else {
        format!("{x}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fenkra_core::{sketch::Curve, BoolOp, Document, Extent, Extrude, FeatureKind, Hole, HoleKind, OriginPlane, PlaneRef, Sketch, SketchData};

    /// Placa de 60 × 60 com `esp` mm de espessura (z de 0 a esp) e um furo de Ø`diametro` no centro,
    /// aberto pelo topo. `roscado`: furo cego com rosca "M6" de `prof` mm; sem ele, passante.
    fn placa(esp: f64, diametro: f64, roscado: Option<f64>) -> Document {
        placa_com_boca(esp, diametro, roscado, HoleKind::Simples)
    }

    /// Como `placa`, com a boca do furo escolhida (rebaixo, escareado).
    fn placa_com_boca(esp: f64, diametro: f64, roscado: Option<f64>, kind: HoleKind) -> Document {
        let mut d = Document::new("Placa");
        let mut n = 7_000_000u64;
        let mut al = || {
            n += 1;
            Id(n)
        };
        let mut base = SketchData::default();
        base.add_rect(&mut al, [-30., -30.], [30., 30.], false);
        let sb = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data: base }));
        d.add_feature(FeatureKind::Extrude(Extrude { sketch: sb, op: BoolOp::Add, extent: Extent::Blind(num(esp)), reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0 }));
        let topo = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), &num(esp));
        let mut centro = SketchData::default();
        let p = centro.add_point(&mut al, 0., 0.);
        centro.add_curve(&mut al, Curve::Point { p }, false);
        let sc = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Datum(topo), data: centro }));
        let (depth, thread) = match roscado {
            Some(prof) => (Extent::Blind(num(prof)), Some("M6".to_string())),
            None => (Extent::ThroughAll, None),
        };
        d.add_feature(FeatureKind::Hole(Hole { sketch: sc, kind: kind, diameter: num(diametro), depth, thread, thread_modeled: false, standard: None, points: vec![], dir: None, reverse: false, tip: None, taper: "0".into(), left_hand: false }));
        d
    }

    fn pasta(nome: &str) -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!("fenkra_fixadores_{nome}"));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    /// Duas placas iguais empilhadas (a de baixo é a fixa; a de cima começa em z = 10).
    fn pilha_de_placas(dir: &Path, doc: &Document) -> (Assembly, Id, Id) {
        std::fs::write(dir.join("placa.fkp"), doc.to_json()).unwrap();
        let mut a = Assembly::new("Pilha");
        let baixo = a.add_component("Placa 1", NodeKind::Part { path: "placa.fkp".into() }, Placement::default(), None);
        let cima = a.add_component("Placa 2", NodeKind::Part { path: "placa.fkp".into() }, Placement { origin: [0., 0., 10.], ..Default::default() }, None);
        (a, baixo, cima)
    }

    fn pos(a: &Assembly, sufixo: &str) -> Placement {
        a.nodes.iter().find(|n| matches!(&n.kind, NodeKind::Part { path } if path.ends_with(sufixo))).unwrap_or_else(|| panic!("sem {sufixo}")).placement
    }

    #[test]
    fn duas_placas_um_parafuso_com_arruelas_e_porca() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let dir = pasta("pilha");
        let (mut a, _, _) = pilha_de_placas(&dir, &placa(10., 6.4, None));
        let mut cache = PartCache::default();
        let op = OpcoesFixadores { folga: 0, ..Default::default() };
        let rel = preencher_furos(&mut a, Some(&dir), &mut cache, &op).unwrap();
        // pega 20 + 1,6 + 1,6 + 5,2 + 2 = 30,4 → próximo L padrão: 35
        assert_eq!(rel.fixadores, 1, "{rel:?}");
        assert_eq!(rel.por_tamanho, vec![("M6x35".to_string(), 1)]);
        assert_eq!((rel.arruelas, rel.porcas, rel.pulados), (2, 1, 0));
        assert!(rel.avisos.is_empty(), "{:?}", rel.avisos);
        for f in ["iso4762_M6x35.fkp", "iso7089_M6.fkp", "iso4032_M6.fkp"] {
            assert!(dir.join("fixadores").join(f).exists(), "{f}");
        }
        assert_eq!(a.nodes.iter().filter(|n| matches!(&n.kind, NodeKind::Part { path } if path.starts_with("fixadores/"))).count(), 4);

        let rep = solve::solve(&mut a);
        assert!(rep.converged, "{rep:?}");
        // parafuso no eixo do furo (x = y = 0), cabeça para cima (o Z local aponta para a cabeça e o
        // corpo desce), apoiada na arruela que está sobre a face (z = 20)
        let p = pos(&a, "iso4762_M6x35.fkp");
        assert!(p.origin[0].hypot(p.origin[1]) < 0.01, "{:?}", p.origin);
        assert!(norm(sub(p.dir([0., 0., 1.]), [0., 0., 1.])) < 1e-6);
        assert!((p.origin[2] - 21.6).abs() < 0.01, "cabeça: {:?}", p.origin);
        // a porca fica sob a placa 1 (z = 0), depois da arruela de 1,6
        let porca = pos(&a, "iso4032_M6.fkp");
        assert!(porca.origin[0].hypot(porca.origin[1]) < 0.01);
        assert!((porca.origin[2] + 1.6).abs() < 0.01, "porca: {:?}", porca.origin);

        // chamar de novo não duplica
        let antes = a.nodes.len();
        let de_novo = preencher_furos(&mut a, Some(&dir), &mut cache, &op).unwrap();
        assert_eq!((de_novo.fixadores, de_novo.pulados), (0, 1), "{de_novo:?}");
        assert_eq!(a.nodes.len(), antes);
    }

    /// Componentes gravados em `fixadores/`.
    fn pecas_do_fixador(a: &Assembly) -> Vec<Id> {
        a.nodes.iter().filter(|n| matches!(&n.kind, NodeKind::Part { path } if path.starts_with("fixadores/"))).map(|n| n.id).collect()
    }

    fn comp_do_arquivo(a: &Assembly, trecho: &str) -> Id {
        a.nodes.iter().find(|n| matches!(&n.kind, NodeKind::Part { path } if path.contains(trecho))).unwrap().id
    }

    /// A peça `c` girada `graus` em torno do eixo do furo (z, passando pela origem).
    fn gira(a: &Assembly, c: Id, graus: f64) -> Placement {
        a.node(c).unwrap().placement.moved([0.; 3], [0., 0., graus.to_radians()], [0.; 3])
    }

    #[test]
    fn todo_fixador_nasce_com_zero_graus_de_liberdade() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let casos: [(&str, Document, OpcoesFixadores, usize); 4] = [
            ("gdl_completo", placa(10., 6.4, None), OpcoesFixadores { folga: 0, ..Default::default() }, 4),
            ("gdl_sem_arruela", placa(10., 6.6, None), OpcoesFixadores { arruela: false, ..Default::default() }, 2),
            ("gdl_sem_porca", placa(10., 6.6, None), OpcoesFixadores { porca: false, ..Default::default() }, 2),
            ("gdl_so_parafuso", placa(10., 6.6, None), OpcoesFixadores { arruela: false, porca: false, ..Default::default() }, 1),
        ];
        for (nome, doc, op, pecas) in casos {
            let dir = pasta(nome);
            let (mut a, _, cima) = pilha_de_placas(&dir, &doc);
            a.node_mut(cima).unwrap().fixed = true;
            let mut cache = PartCache::default();
            preencher_furos(&mut a, Some(&dir), &mut cache, &op).unwrap();
            let rep = solve::solve(&mut a);
            assert!(rep.converged, "{nome}: {rep:?}");
            assert_eq!(rep.dof, 0, "{nome}: a montagem inteira fica definida");
            let gdl = solve::dof_per_comp(&a);
            let ids = pecas_do_fixador(&a);
            assert_eq!(ids.len(), pecas, "{nome}");
            for c in ids {
                assert_eq!(gdl[&c], 0, "{nome}: {}", a.name_of(c));
            }
        }
        // furo roscado: parafuso e arruela, sem porca
        let dir = pasta("gdl_roscado");
        let mut a = Assembly::new("Rosca");
        std::fs::write(dir.join("bloco.fkp"), placa(30., 5., Some(18.)).to_json()).unwrap();
        a.add_component("Bloco", NodeKind::Part { path: "bloco.fkp".into() }, Placement::default(), None);
        preencher_furos(&mut a, Some(&dir), &mut PartCache::default(), &OpcoesFixadores::default()).unwrap();
        let rep = solve::solve(&mut a);
        assert!(rep.converged && rep.dof == 0, "{rep:?}");
    }

    #[test]
    fn deixar_o_giro_livre_e_opcional() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let dir = pasta("giro_livre");
        let (mut a, _, cima) = pilha_de_placas(&dir, &placa(10., 6.6, None));
        a.node_mut(cima).unwrap().fixed = true;
        let op = OpcoesFixadores { travar_giro: false, ..Default::default() };
        preencher_furos(&mut a, Some(&dir), &mut PartCache::default(), &op).unwrap();
        assert!(!a.mates.iter().any(|m| m.name.starts_with(TRAVA)));
        assert!(solve::solve(&mut a).converged);
        let gdl = solve::dof_per_comp(&a);
        for c in pecas_do_fixador(&a) {
            assert_eq!(gdl[&c], 1, "só o giro: {}", a.name_of(c));
        }
    }

    #[test]
    fn suprimir_a_trava_devolve_o_giro_e_a_pilha_gira_junta() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let dir = pasta("giro_trava");
        let (mut a, _, cima) = pilha_de_placas(&dir, &placa(10., 6.6, None));
        a.node_mut(cima).unwrap().fixed = true;
        preencher_furos(&mut a, Some(&dir), &mut PartCache::default(), &OpcoesFixadores::default()).unwrap();
        assert!(solve::solve(&mut a).converged);
        let parafuso = comp_do_arquivo(&a, "iso4762");
        let porca = comp_do_arquivo(&a, "iso4032");
        let x = |a: &Assembly, c: Id| a.node(c).unwrap().placement.dir([1., 0., 0.]);
        let (x0, xp0) = (x(&a, parafuso), x(&a, porca));

        // travado: puxar o parafuso em volta do eixo não o tira do lugar
        let alvo = gira(&a, parafuso, 40.);
        solve::solve_toward(&mut a, parafuso, alvo, 0.05);
        assert!(norm(sub(x(&a, parafuso), x0)) < 1e-4, "{:?}", x(&a, parafuso));

        // trava do parafuso suprimida: ele gira, e a porca (travada nele) vai junto
        let trava = a.mates.iter().find(|m| m.name.starts_with(TRAVA) && m.b.comp == parafuso).unwrap().id;
        a.mates.iter_mut().find(|m| m.id == trava).unwrap().suppressed = true;
        assert_eq!(solve::dof(&a), 1, "a pilha inteira gira como um conjunto");
        let alvo = gira(&a, parafuso, 40.);
        solve::solve_toward(&mut a, parafuso, alvo, 0.05);
        let giro = dot(x(&a, parafuso), x0);
        assert!(giro < 0.99, "girou: {giro}");
        assert!((dot(x(&a, porca), xp0) - giro).abs() < 1e-4, "a porca segue o parafuso");
    }

    #[test]
    fn placa_de_cima_virada_ainda_forma_a_pilha() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let dir = pasta("virada");
        let (mut a, _, cima) = pilha_de_placas(&dir, &placa(10., 6.6, None));
        // placa 2 de cabeça para baixo (ainda ocupa z de 10 a 20, mas o furo agora entra por baixo)
        a.node_mut(cima).unwrap().placement = Placement::default().moved([0., 0., 20.], [std::f64::consts::PI, 0., 0.], [0.; 3]);
        let mut cache = PartCache::default();
        let rel = preencher_furos(&mut a, Some(&dir), &mut cache, &OpcoesFixadores::default()).unwrap();
        assert_eq!(rel.por_tamanho, vec![("M6x35".to_string(), 1)], "{rel:?}");
        assert!(solve::solve(&mut a).converged);
        // a entrada é a face de cima da placa 2 (z = 20) + arruela de 1,6; a porca sai por baixo da placa 1
        let p = pos(&a, "iso4762_M6x35.fkp");
        assert!(p.origin[0].hypot(p.origin[1]) < 0.01 && (p.origin[2] - 21.6).abs() < 0.01, "{:?}", p.origin);
        assert!((pos(&a, "iso4032_M6.fkp").origin[2] + 1.6).abs() < 0.01);
    }

    #[test]
    fn sem_arruela_nem_porca_e_so_nos_escolhidos() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let dir = pasta("simples");
        let (mut a, baixo, _) = pilha_de_placas(&dir, &placa(10., 6.6, None));
        let mut cache = PartCache::default();
        // só a placa de baixo: pilha de uma peça, pega 10 + 2 passos = 12
        let op = OpcoesFixadores { arruela: false, porca: false, componentes: vec![baixo], ..Default::default() };
        let rel = preencher_furos(&mut a, Some(&dir), &mut cache, &op).unwrap();
        assert_eq!((rel.fixadores, rel.arruelas, rel.porcas), (1, 0, 0), "{rel:?}");
        assert_eq!(rel.por_tamanho, vec![("M6x12".to_string(), 1)]);
        assert!(solve::solve(&mut a).converged);
        // cabeça direto na face de cima da placa 1 (z = 10)
        let p = pos(&a, "M6x12.fkp");
        assert!(p.origin[0].hypot(p.origin[1]) < 0.01 && (p.origin[2] - 10.).abs() < 0.01, "{:?}", p.origin);
    }

    #[test]
    fn furo_roscado_cego_leva_parafuso_sem_porca() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let dir = pasta("roscado");
        let mut a = Assembly::new("Rosca");
        std::fs::write(dir.join("bloco.fkp"), placa(30., 5., Some(18.)).to_json()).unwrap();
        a.add_component("Bloco", NodeKind::Part { path: "bloco.fkp".into() }, Placement::default(), None);
        let mut cache = PartCache::default();
        let rel = preencher_furos(&mut a, Some(&dir), &mut cache, &OpcoesFixadores::default()).unwrap();
        // profundidade 18 + arruela 1,6 = 19,6 → arredonda para baixo: L16
        assert_eq!((rel.fixadores, rel.arruelas, rel.porcas), (1, 1, 0), "{rel:?}");
        assert_eq!(rel.por_tamanho, vec![("M6x16".to_string(), 1)]);
    }

    #[test]
    fn peca_gravada_com_biblioteca_antiga_avisa_sem_regravar() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let dir = pasta("selo_velho");
        let (mut a, _, _) = pilha_de_placas(&dir, &placa(10., 6.6, None));
        // um M6x35 de uma geração antiga já está na pasta
        std::fs::create_dir_all(dir.join("fixadores")).unwrap();
        let mut velho = fenkra_solidos::fixadores::fixador_documento("iso4762", "M6", Some(35.)).unwrap();
        velho.props.custom.iter_mut().find(|(k, _)| k.starts_with("Biblioteca: geração")).unwrap().1 = "1".into();
        let arq = dir.join("fixadores").join("iso4762_M6x35.fkp");
        std::fs::write(&arq, velho.to_json()).unwrap();
        let antes = std::fs::read_to_string(&arq).unwrap();
        let rel = preencher_furos(&mut a, Some(&dir), &mut PartCache::default(), &OpcoesFixadores::default()).unwrap();
        assert_eq!(rel.por_tamanho, vec![("M6x35".to_string(), 1)], "{rel:?}");
        assert!(rel.avisos.iter().any(|m| m.contains("iso4762_M6x35.fkp") && m.contains("desatualizada")), "{:?}", rel.avisos);
        assert_eq!(std::fs::read_to_string(&arq).unwrap(), antes, "não regrava");
    }

    #[test]
    fn furo_sem_tamanho_compativel_avisa() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let dir = pasta("aviso");
        let (mut a, _, _) = pilha_de_placas(&dir, &placa(10., 7.3, None));
        let mut cache = PartCache::default();
        let rel = preencher_furos(&mut a, Some(&dir), &mut cache, &OpcoesFixadores::default()).unwrap();
        assert_eq!(rel.fixadores, 0);
        assert_eq!(rel.avisos.len(), 1, "{:?}", rel.avisos);
        assert!(rel.avisos[0].contains("7.3"), "{:?}", rel.avisos);
        assert!(preencher_furos(&mut a, None, &mut cache, &OpcoesFixadores::default()).is_err(), "sem pasta não grava");
    }

    /// Família de teste (dados inventados): parafuso com faixa de furo própria, sem folga de tabela.
    const AERO_TESTE: &str = r#"{
      "id": "teste_pol", "nome": "Parafuso de teste", "norma": "TESTE 1", "revisao": "2020",
      "gerador": "parafuso_sextavado", "nivel": "A", "licenca": "teste",
      "fontes": [{ "rotulo": "teste", "url": "https://exemplo.invalid/t" }],
      "campos": ["P", "s", "k"],
      "tamanhos": [
        { "designacao": "-06", "d_nominal": 4.8, "valores": { "P": 0.8, "s": 8, "k": 3.5 }, "confirmado_por": 1,
          "comprimentos": [8, 10, 12], "furo_min": 4.8, "furo_max": 4.95 }
      ]
    }"#;

    fn biblioteca_de_teste() -> Biblioteca {
        let usuario = Biblioteca::de_jsons(biblioteca::Camada::Usuario, &[("teste.json", AERO_TESTE)]).unwrap();
        Biblioteca::mesclar(&[biblioteca::publica().clone(), usuario])
    }

    #[test]
    fn furo_de_49_mm_e_reconhecido_pela_faixa_do_prendedor() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let dir = pasta("faixa_furo");
        std::fs::write(dir.join("placa.fkp"), placa(5., 4.9, None).to_json()).unwrap();
        let monta = || {
            let mut a = Assembly::new("Faixa");
            a.add_component("Placa", NodeKind::Part { path: "placa.fkp".into() }, Placement::default(), None);
            a
        };
        let lib = biblioteca_de_teste();
        let op = OpcoesFixadores { familia_parafuso: "teste_pol".into(), arruela: false, porca: false, ..Default::default() };
        // com a família que declara furo_min..furo_max, o Ø4,9 vira "-06"; pega 5 + 2 passos = 6,6 → L8
        let rel = preencher_com(&mut monta(), Some(&dir), &mut PartCache::default(), &op, &lib).unwrap();
        assert_eq!(rel.por_tamanho, vec![("-06x8".to_string(), 1)], "{rel:?}");
        assert!(matches!(&rel.marcas[..], [MarcaFuro { estado: EstadoFuro::Entra { tamanho, comprimento, .. }, .. }] if tamanho == "-06" && *comprimento == 8.), "{:?}", rel.marcas);
        // com a família métrica o mesmo furo fica sem tamanho, e o motivo vai para a marca do furo
        let rel = preencher_furos(&mut monta(), Some(&dir), &mut PartCache::default(), &OpcoesFixadores::default()).unwrap();
        assert_eq!(rel.fixadores, 0);
        let [m] = &rel.marcas[..] else { panic!("{:?}", rel.marcas) };
        assert!(matches!(&m.estado, EstadoFuro::SemTamanho(t) if t.contains("4.9")), "{m:?}");
        assert_eq!(m.diametro, 4.9);
        assert!(m.pos[2] > 4.9 && m.dir[2] < -0.99, "boca em cima, sentido para dentro: {:?} {:?}", m.pos, m.dir);
    }

    #[test]
    fn previa_marca_os_furos_sem_inserir_nem_gravar() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let dir = pasta("previa");
        let (mut a, _, _) = pilha_de_placas(&dir, &placa(10., 6.6, None));
        let antes = a.nodes.len();
        let op = OpcoesFixadores { so_previa: true, ..Default::default() };
        let rel = preencher_furos(&mut a, Some(&dir), &mut PartCache::default(), &op).unwrap();
        assert_eq!((a.nodes.len(), a.mates.len()), (antes, 0));
        assert!(!dir.join("fixadores").exists(), "a prévia não grava peças");
        assert_eq!(rel.por_tamanho, vec![("M6x35".to_string(), 1)], "{rel:?}");
        // um furo por placa, os dois entram
        assert_eq!(rel.marcas.len(), 2);
        assert!(rel.marcas.iter().all(|m| matches!(&m.estado, EstadoFuro::Entra { tamanho, comprimento, .. } if tamanho == "M6" && *comprimento == 35.)));
        // aplicar e prever de novo: os dois furos já têm fixador
        let real = preencher_furos(&mut a, Some(&dir), &mut PartCache::default(), &OpcoesFixadores::default()).unwrap();
        assert_eq!(real.fixadores, 1);
        let de_novo = preencher_furos(&mut a, Some(&dir), &mut PartCache::default(), &op).unwrap();
        assert_eq!(de_novo.marcas.len(), 2);
        assert!(de_novo.marcas.iter().all(|m| m.estado == EstadoFuro::JaTem), "{:?}", de_novo.marcas);
    }

    #[test]
    fn cabeca_assenta_no_fundo_do_rebaixo() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let dir = pasta("rebaixo");
        std::fs::write(dir.join("placa.fkp"), placa_com_boca(10., 6.6, None, HoleKind::Rebaixado { diameter: "11".into(), depth: "6.4".into() }).to_json()).unwrap();
        let mut a = Assembly::new("Rebaixo");
        a.add_component("Placa", NodeKind::Part { path: "placa.fkp".into() }, Placement::default(), None);
        // sem arruela a cabeça (Ø10) cabe no rebaixo (Ø11) e desce até z = 10 - 6,4 = 3,6
        let op = OpcoesFixadores { arruela: false, ..Default::default() };
        let rel = preencher_furos(&mut a, Some(&dir), &mut PartCache::default(), &op).unwrap();
        // pega 10 - 6,4 = 3,6 + porca 5,2 + 2 passos = 10,8 → L12
        assert_eq!(rel.por_tamanho, vec![("M6x12".to_string(), 1)], "{rel:?}");
        assert!(rel.avisos.is_empty(), "{:?}", rel.avisos);
        assert!(solve::solve(&mut a).converged);
        let p = pos(&a, "iso4762_M6x12.fkp");
        assert!((p.origin[2] - 3.6).abs() < 0.01, "cabeça no fundo do rebaixo: {:?}", p.origin);
        // a porca fica sob a placa (z = 0)
        assert!(pos(&a, "iso4032_M6.fkp").origin[2].abs() < 0.01);

        // com arruela (Ø12 > Ø11) o item não cabe: assenta na face e avisa
        let mut b = Assembly::new("Rebaixo 2");
        b.add_component("Placa", NodeKind::Part { path: "placa.fkp".into() }, Placement::default(), None);
        let rel = preencher_furos(&mut b, Some(&dir), &mut PartCache::default(), &OpcoesFixadores::default()).unwrap();
        assert!(rel.avisos.iter().any(|m| m.contains("não cabe na boca")), "{:?}", rel.avisos);
        assert!(solve::solve(&mut b).converged);
        assert!((pos(&b, "iso4762_M6x25.fkp").origin[2] - 11.6).abs() < 0.01, "{:?}", pos(&b, "iso4762_M6x25.fkp").origin);
    }

    #[test]
    fn cabeca_assenta_no_cone_do_escareado() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let dir = pasta("escareado");
        // cone de 90° de Ø12 em Ø6,6; a cabeça de Ø10 apoia onde o cone tem Ø10: (12 - 10) / 2 = 1 mm abaixo da face
        std::fs::write(dir.join("placa.fkp"), placa_com_boca(10., 6.6, None, HoleKind::Escareado { diameter: "12".into(), angle: "90".into() }).to_json()).unwrap();
        let mut a = Assembly::new("Cone");
        a.add_component("Placa", NodeKind::Part { path: "placa.fkp".into() }, Placement::default(), None);
        let op = OpcoesFixadores { arruela: false, ..Default::default() };
        let rel = preencher_furos(&mut a, Some(&dir), &mut PartCache::default(), &op).unwrap();
        assert_eq!(rel.fixadores, 1, "{rel:?}");
        assert!(solve::solve(&mut a).converged);
        let p = a.nodes.iter().find(|n| matches!(&n.kind, NodeKind::Part { path } if path.contains("iso4762"))).unwrap().placement;
        assert!((p.origin[2] - 9.).abs() < 0.01, "{:?}", p.origin);
    }

    #[test]
    fn rebaixo_fora_do_centro_e_recusado() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let dir = pasta("fora_do_centro");
        std::fs::write(dir.join("placa.fkp"), placa_com_boca(10., 6.6, None, HoleKind::Rebaixado { diameter: "11".into(), depth: "6.4".into() }).to_json()).unwrap();
        let mut cache = PartCache::default();
        let lp = cache.get(&full_path(Some(&dir), "placa.fkp"), true).unwrap();
        let linha = fenkra_solidos::hole_table(&lp.doc).remove(0);
        let forma = lp.shape.as_ref().unwrap();
        assert!(boca_concentrica(forma, &linha));
        // centro do furo 2 mm fora do centro do rebaixo: algumas retas caem na face
        let mut torto = linha.clone();
        torto.pos[0] += 2.;
        assert!(!boca_concentrica(forma, &torto));
        // escareado concêntrico passa no mesmo teste
        std::fs::write(dir.join("cone.fkp"), placa_com_boca(10., 6.6, None, HoleKind::Escareado { diameter: "12".into(), angle: "90".into() }).to_json()).unwrap();
        let lp = cache.get(&full_path(Some(&dir), "cone.fkp"), true).unwrap();
        let linha = fenkra_solidos::hole_table(&lp.doc).remove(0);
        assert!(boca_concentrica(lp.shape.as_ref().unwrap(), &linha));
    }

    #[test]
    fn comprimento_pedido_para_na_serie_e_a_ponta_acompanha() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let dir = pasta("alca");
        let (mut a, _, _) = pilha_de_placas(&dir, &placa(10., 6.6, None));
        let previa = |a: &mut Assembly, l: Option<f64>| {
            let op = OpcoesFixadores { so_previa: true, comprimento: l, ..Default::default() };
            preencher_furos(a, Some(&dir), &mut PartCache::default(), &op).unwrap()
        };
        let ponta_e_l = |rel: &RelatorioFixadores| match &rel.marcas[0].estado {
            EstadoFuro::Entra { comprimento, ponta, serie, .. } => (*comprimento, *ponta, serie.clone()),
            e => panic!("{e:?}"),
        };
        // automático: M6x35, e a série do M6 chega à ponta pelos dois lados
        let (l, ponta, serie) = ponta_e_l(&previa(&mut a, None));
        assert_eq!(l, 35.);
        assert_eq!((serie.first(), serie.last()), (Some(&12.), Some(&60.)));
        // a ponta fica L abaixo da cabeça: a cabeça apoia 1,6 acima da face de cima (z = 20)
        assert!((ponta[2] - (21.6 - 35.)).abs() < 1e-6 && ponta[0].hypot(ponta[1]) < 1e-6, "{ponta:?}");
        // pedido fora da série cai no valor mais próximo existente
        assert_eq!(ponta_e_l(&previa(&mut a, Some(37.))).0, 35.);
        assert_eq!(ponta_e_l(&previa(&mut a, Some(38.))).0, 40.);
        assert_eq!(ponta_e_l(&previa(&mut a, Some(1000.))).0, 60.);
        let (_, p40, _) = ponta_e_l(&previa(&mut a, Some(40.)));
        assert!((p40[2] - (21.6 - 40.)).abs() < 1e-6);
        // curto demais para atravessar a pilha avisa, mas obedece
        let rel = previa(&mut a, Some(20.));
        assert_eq!(ponta_e_l(&rel).0, 20.);
        assert!(rel.avisos.iter().any(|m| m.contains("não atravessa")), "{:?}", rel.avisos);
        // e vale na aplicação de verdade
        let op = OpcoesFixadores { comprimento: Some(40.), ..Default::default() };
        let real = preencher_furos(&mut a, Some(&dir), &mut PartCache::default(), &op).unwrap();
        assert_eq!(real.por_tamanho, vec![("M6x40".to_string(), 1)], "{real:?}");
    }

    #[test]
    fn mais_proximo_da_serie() {
        let s = [12., 16., 20.];
        assert_eq!(mais_proximo(&s, 14.), Some(16.), "empate vai para o maior");
        assert_eq!(mais_proximo(&s, 13.), Some(12.));
        assert_eq!(mais_proximo(&s, 0.), Some(12.));
        assert_eq!(mais_proximo(&s, 99.), Some(20.));
        assert_eq!(mais_proximo(&[], 5.), None);
    }

    #[test]
    fn auxiliares_de_tamanho_e_eixo() {
        assert_eq!(casa_folga(6.6, 1), Some("M6"));
        assert_eq!(casa_folga(6.6, 0), None);
        assert_eq!(casa_folga(6.64, 1), Some("M6"));
        assert_eq!(casa_outra_folga(7.0), Some("M6"));
        assert_eq!(casa_outra_folga(7.3), None);
        let iso = biblioteca::familia("iso4762").unwrap();
        assert_eq!(tamanho_da_rosca(iso, "M8x1.25").as_deref(), Some("M8"));
        assert_eq!(tamanho_da_rosca(iso, "M6").as_deref(), Some("M6"));
        assert_eq!(tamanho_da_rosca(iso, "M7"), None);
        assert_eq!(tamanho_da_rosca(iso, "G1/4"), None);
        // colinear: sentido contrário também vale; 0,1 mm de desvio ou 1° não
        assert!(colineares([0., 0., 0.], [0., 0., 1.], [0., 0., 30.], [0., 0., -1.]));
        assert!(!colineares([0., 0., 0.], [0., 0., 1.], [0.1, 0., 5.], [0., 0., 1.]));
        let g = 1_f64.to_radians();
        assert!(!colineares([0., 0., 0.], [0., 0., 1.], [0., 0., 5.], [g.sin(), 0., g.cos()]));
        assert_eq!(num(35.), "35");
        assert_eq!(num(1.6), "1.6");
        // o eixo Z da orientação aponta para onde se pediu e os eixos são ortonormais
        let p = orientado([1., 2., 3.], [0., 1., 0.]);
        assert!(norm(sub(p.dir([0., 0., 1.]), [0., 1., 0.])) < 1e-12);
        assert!(norm(sub(cross(p.rot[0], p.rot[1]), p.rot[2])) < 1e-12);
    }
}
