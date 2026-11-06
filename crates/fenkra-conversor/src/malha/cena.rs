// SPDX-License-Identifier: LGPL-2.1-or-later
//! A cena: malhas, materiais e nós. Um grafo, como o modelo neutro: a peça usada quatro vezes é uma
//! malha e quatro nós que apontam para ela.

use crate::modelo::{compor, PROFUNDIDADE};
use crate::ErroConv;
use fenkra_kernel::IDENTIDADE;
use std::collections::{BTreeMap, HashMap};

/// Teto de ocorrências: um arquivo hostil faz o grafo crescer em potência com poucos bytes.
pub(crate) const LIMITE_OCORRENCIAS: usize = 1_000_000;

#[derive(Clone, Debug, PartialEq)]
pub struct Material {
    pub nome: String,
    /// vermelho, verde, azul e opacidade, de 0 a 1
    pub cor: [f32; 4],
    pub metal: f32,
    pub rugosidade: f32,
}

impl Material {
    pub fn de_cor(nome: &str, cor: [f32; 4]) -> Material {
        Material { nome: nome.into(), cor, metal: 0., rugosidade: 0.6 }
    }
}

/// Triângulos com um material só. Quem tem mais de um material vira várias malhas num nó.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Malha {
    pub nome: String,
    pub pos: Vec<[f64; 3]>,
    pub tris: Vec<[u32; 3]>,
    /// vazio, ou uma por vértice
    pub nrm: Vec<[f32; 3]>,
    /// vazio, ou uma por vértice
    pub cor_v: Vec<[u8; 4]>,
    pub material: Option<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct No {
    pub nome: String,
    /// matriz 3x4 por linhas, relativa ao pai
    pub local: [f64; 12],
    pub malhas: Vec<usize>,
    pub filhos: Vec<usize>,
}

impl No {
    pub fn novo(nome: &str) -> No {
        No { nome: nome.into(), local: IDENTIDADE, malhas: vec![], filhos: vec![] }
    }
}

/// Uma cena em milímetros, com Z para cima.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Cena {
    pub nome: String,
    pub malhas: Vec<Malha>,
    pub materiais: Vec<Material>,
    pub nos: Vec<No>,
    pub raizes: Vec<usize>,
    /// unidade que o arquivo declara, ou vazio quando ele não declara nenhuma
    pub unidade: String,
    /// o que o leitor achou e não traduziu, com a contagem
    pub ignorados: BTreeMap<String, usize>,
}

/// Uma malha num lugar da árvore: caminho de nomes, índice da malha e matriz acumulada.
pub type Ocorrencia = (Vec<String>, usize, [f64; 12]);

fn aplicar(m: &[f64; 12], p: [f64; 3]) -> [f64; 3] {
    [0, 1, 2].map(|i| m[i * 4] * p[0] + m[i * 4 + 1] * p[1] + m[i * 4 + 2] * p[2] + m[i * 4 + 3])
}

fn determinante(m: &[f64; 12]) -> f64 {
    m[0] * (m[5] * m[10] - m[6] * m[9]) - m[1] * (m[4] * m[10] - m[6] * m[8]) + m[2] * (m[4] * m[9] - m[5] * m[8])
}

/// A matriz só gira e desloca? Escala e espelho não cabem numa posição de montagem.
pub(crate) fn e_rigida(m: &[f64; 12]) -> bool {
    let linha = |i: usize| [m[i * 4], m[i * 4 + 1], m[i * 4 + 2]];
    let ponto = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let (a, b, c) = (linha(0), linha(1), linha(2));
    let perto = |x: f64, y: f64| (x - y).abs() < 1e-6;
    perto(ponto(a, a), 1.) && perto(ponto(b, b), 1.) && perto(ponto(c, c), 1.) && perto(ponto(a, b), 0.) && perto(ponto(a, c), 0.) && perto(ponto(b, c), 0.) && determinante(m) > 0.
}

/// Zeros e uns de rotações por 90 graus saem com ruído de 1e-16: limpa para a matriz ser exata.
fn limpar(m: &mut [f64; 12]) {
    for (i, v) in m.iter_mut().enumerate() {
        if i % 4 == 3 {
            continue;
        }
        let r = v.round();
        if (*v - r).abs() < 1e-12 {
            *v = r;
        }
    }
}

impl Malha {
    /// Vértices iguais viram um só. Leitores de triângulo solto (STL) e a malha do kernel, que repete o
    /// vértice em cada face, só fecham depois disso. A normal fica de fora: o vértice soldado tem várias.
    pub fn soldada(&self) -> Malha {
        let mut mapa: HashMap<[u64; 3], u32> = HashMap::new();
        let mut m = Malha { nome: self.nome.clone(), material: self.material, ..Default::default() };
        let novos: Vec<u32> = self
            .pos
            .iter()
            .enumerate()
            .map(|(i, p)| {
                // +0.0 faz -0.0 e 0.0 caírem na mesma chave
                let chave = p.map(|v| (v + 0.).to_bits());
                *mapa.entry(chave).or_insert_with(|| {
                    m.pos.push(*p);
                    if let Some(c) = self.cor_v.get(i) {
                        m.cor_v.push(*c);
                    }
                    (m.pos.len() - 1) as u32
                })
            })
            .collect();
        m.tris = self.tris.iter().map(|t| t.map(|i| novos[i as usize])).filter(|t| t[0] != t[1] && t[1] != t[2] && t[0] != t[2]).collect();
        m
    }

    /// Toda aresta tem exatamente duas faces? Só faz sentido na malha soldada.
    pub fn fechada(&self) -> bool {
        let mut arestas: HashMap<(u32, u32), u32> = HashMap::new();
        for t in &self.tris {
            for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
                *arestas.entry((a.min(b), a.max(b))).or_default() += 1;
            }
        }
        !arestas.is_empty() && arestas.values().all(|n| *n == 2)
    }

    /// Partes sem vértice em comum, cada uma com os seus triângulos. O kernel monta um sólido por
    /// casca: mandar duas juntas perderia a segunda.
    pub fn componentes(&self) -> Vec<Malha> {
        fn raiz(pai: &mut [u32], mut i: u32) -> u32 {
            while pai[i as usize] != i {
                pai[i as usize] = pai[pai[i as usize] as usize];
                i = pai[i as usize];
            }
            i
        }
        let mut pai: Vec<u32> = (0..self.pos.len() as u32).collect();
        for t in &self.tris {
            let a = raiz(&mut pai, t[0]);
            for v in [t[1], t[2]] {
                let b = raiz(&mut pai, v);
                pai[b as usize] = a;
            }
        }
        let mut grupos: BTreeMap<u32, Vec<[u32; 3]>> = BTreeMap::new();
        for t in &self.tris {
            grupos.entry(raiz(&mut pai, t[0])).or_default().push(*t);
        }
        grupos
            .into_values()
            .map(|tris| {
                let mut novo: HashMap<u32, u32> = HashMap::new();
                let mut m = Malha { nome: self.nome.clone(), material: self.material, ..Default::default() };
                for t in tris {
                    let t = t.map(|v| {
                        *novo.entry(v).or_insert_with(|| {
                            m.pos.push(self.pos[v as usize]);
                            if let Some(c) = self.cor_v.get(v as usize) {
                                m.cor_v.push(*c);
                            }
                            (m.pos.len() - 1) as u32
                        })
                    });
                    m.tris.push(t);
                }
                m
            })
            .collect()
    }

    /// Volume com sinal: positivo com as normais para fora.
    pub fn volume(&self) -> f64 {
        self.tris
            .iter()
            .map(|t| {
                let [a, b, c] = t.map(|i| self.pos[i as usize]);
                a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0]) + a[2] * (b[0] * c[1] - b[1] * c[0])
            })
            .sum::<f64>()
            / 6.
    }

    pub fn area(&self) -> f64 {
        self.tris
            .iter()
            .map(|t| {
                let [a, b, c] = t.map(|i| self.pos[i as usize]);
                let (u, v) = ([b[0] - a[0], b[1] - a[1], b[2] - a[2]], [c[0] - a[0], c[1] - a[1], c[2] - a[2]]);
                let n = [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]];
                (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt() / 2.
            })
            .sum()
    }

    /// Cópia na posição da matriz. Espelho inverte o sentido dos triângulos, e a normal só sobrevive à
    /// matriz rígida.
    pub(crate) fn transformada(&self, m: &[f64; 12]) -> Malha {
        let mut r = self.clone();
        r.pos = self.pos.iter().map(|p| aplicar(m, *p)).collect();
        if e_rigida(m) {
            let giro = [m[0], m[1], m[2], 0., m[4], m[5], m[6], 0., m[8], m[9], m[10], 0.];
            r.nrm = self.nrm.iter().map(|n| aplicar(&giro, n.map(f64::from)).map(|v| v as f32)).collect();
        } else {
            r.nrm.clear();
        }
        if determinante(m) < 0. {
            for t in &mut r.tris {
                t.swap(1, 2);
            }
        }
        r
    }

    /// Índices fora da lista de vértices, comuns em arquivo corrompido, viram erro aqui, uma vez, e
    /// nenhum gravador precisa conferir de novo.
    pub(crate) fn conferir(&self, formato: &str) -> Result<(), ErroConv> {
        let n = self.pos.len() as u32;
        if self.tris.iter().flatten().any(|i| *i >= n) {
            return Err(super::erro(formato, "triângulo com vértice inexistente"));
        }
        if self.pos.iter().flatten().any(|v| !v.is_finite()) {
            return Err(super::erro(formato, "vértice com número inválido"));
        }
        Ok(())
    }
}

impl Cena {
    /// Material com esta cor: repetir a cor repete o material, e a lista fica curta.
    pub fn material(&mut self, nome: &str, cor: [f32; 4]) -> usize {
        if let Some(i) = self.materiais.iter().position(|m| m.cor == cor) {
            return i;
        }
        self.materiais.push(Material::de_cor(nome, cor));
        self.materiais.len() - 1
    }

    /// Conta algo que o leitor viu e não traduziu.
    pub(crate) fn ignorar(&mut self, oque: &str, quantos: usize) {
        debug_assert!(super::ITENS_FORA.contains(&oque), "termo sem tradução: {oque}");
        if quantos > 0 {
            *self.ignorados.entry(oque.to_string()).or_insert(0) += quantos;
        }
    }

    /// Acrescenta uma malha e um nó de topo com ela.
    pub(crate) fn raiz_com(&mut self, m: Malha) -> usize {
        let mut no = No::novo(&m.nome);
        no.malhas.push(self.malhas.len());
        self.malhas.push(m);
        self.nos.push(no);
        self.raizes.push(self.nos.len() - 1);
        self.nos.len() - 1
    }

    /// Todas as malhas nos seus lugares. Para em [`LIMITE_OCORRENCIAS`] e no limite de profundidade:
    /// ciclo não trava, e o segundo valor sai `false`.
    pub fn ocorrencias(&self) -> (Vec<Ocorrencia>, bool) {
        let mut saida = Vec::new();
        let mut inteiro = true;
        for &r in &self.raizes {
            self.descer(r, &mut Vec::new(), IDENTIDADE, 0, &mut saida, &mut inteiro);
        }
        (saida, inteiro)
    }

    fn descer(&self, i: usize, caminho: &mut Vec<String>, pai: [f64; 12], nivel: usize, saida: &mut Vec<Ocorrencia>, inteiro: &mut bool) {
        let Some(no) = self.nos.get(i) else { return };
        if nivel > PROFUNDIDADE || saida.len() >= LIMITE_OCORRENCIAS {
            *inteiro = false;
            return;
        }
        let local = compor(&pai, &no.local);
        caminho.push(no.nome.clone());
        for &m in &no.malhas {
            saida.push((caminho.clone(), m, local));
        }
        for &f in &no.filhos {
            self.descer(f, caminho, local, nivel + 1, saida, inteiro);
        }
        caminho.pop();
    }

    /// Tudo no lugar, para formatos sem árvore. O nome de cada malha é o do nó que a usa.
    pub fn no_lugar(&self) -> Vec<Malha> {
        self.ocorrencias()
            .0
            .into_iter()
            .filter_map(|(caminho, m, local)| {
                let mut r = self.malhas.get(m)?.transformada(&local);
                if let Some(nome) = caminho.iter().rev().find(|n| !n.is_empty()) {
                    r.nome.clone_from(nome);
                }
                Some(r)
            })
            .collect()
    }

    /// Multiplica os comprimentos: vértices e deslocamentos. As rotações não mudam.
    pub fn escalar(&mut self, k: f64) {
        if k == 1. {
            return;
        }
        for m in &mut self.malhas {
            for p in &mut m.pos {
                *p = p.map(|v| v * k);
            }
        }
        for n in &mut self.nos {
            for i in [3, 7, 11] {
                n.local[i] *= k;
            }
        }
    }

    /// Põe um nó acima de todas as raízes. Serve para trocar o eixo de cima na leitura.
    pub(crate) fn embrulhar(&mut self, local: [f64; 12]) {
        let mut no = No::novo("");
        no.local = local;
        no.filhos = std::mem::take(&mut self.raizes);
        self.nos.push(no);
        self.raizes = vec![self.nos.len() - 1];
    }

    pub fn sem_cores(&mut self) {
        self.materiais.clear();
        for m in &mut self.malhas {
            m.material = None;
            m.cor_v.clear();
        }
    }

    /// Cor por vértice igual em toda a malha é só a cor da peça: vira material, que todo formato leva.
    pub(crate) fn cor_uniforme_vira_material(&mut self) {
        for i in 0..self.malhas.len() {
            let Some(primeira) = self.malhas[i].cor_v.first().copied() else { continue };
            if self.malhas[i].cor_v.iter().all(|c| *c == primeira) {
                let cor = primeira.map(|v| f32::from(v) / 255.);
                let nome = format!("Cor {}", self.materiais.len() + 1);
                let k = self.material(&nome, cor);
                self.malhas[i].material = Some(k);
                self.malhas[i].cor_v.clear();
            }
        }
    }

    /// Peças de uma cor só, juntas num arquivo sem estrutura (PLY): se cada parte solta tem cor única,
    /// cada uma volta a ser uma peça com o seu material.
    pub(crate) fn separar_por_cor(&mut self) {
        for i in 0..self.malhas.len() {
            if self.malhas[i].cor_v.is_empty() {
                continue;
            }
            let partes = self.malhas[i].soldada().componentes();
            let uniforme = |m: &Malha| m.cor_v.windows(2).all(|w| w[0] == w[1]);
            if partes.len() < 2 || !partes.iter().all(uniforme) {
                continue;
            }
            let dono: Vec<usize> = (0..self.nos.len()).filter(|n| self.nos[*n].malhas.contains(&i)).collect();
            let raizes = std::mem::take(&mut self.raizes);
            for r in raizes {
                if !dono.contains(&r) {
                    self.raizes.push(r);
                }
            }
            for m in partes {
                self.raiz_com(m);
            }
            self.malhas[i].tris.clear();
        }
        self.cor_uniforme_vira_material();
    }

    /// Cor de uma malha: a do material ou a média dos vértices.
    pub fn cor_de(&self, m: &Malha) -> Option<[f32; 4]> {
        m.material.and_then(|i| self.materiais.get(i)).map(|k| k.cor).or_else(|| {
            if m.cor_v.is_empty() {
                return None;
            }
            let mut soma = [0f64; 4];
            for c in &m.cor_v {
                for i in 0..4 {
                    soma[i] += f64::from(c[i]);
                }
            }
            Some(soma.map(|s| (s / m.cor_v.len() as f64 / 255.) as f32))
        })
    }

    /// Tira os nós que só passam adiante. Formatos com camadas de embrulho (3MF, glTF com eixo trocado)
    /// não podem virar montagens de um filho só. Nó com filho compartilhado continua compartilhando:
    /// o que se copia é a referência.
    pub(crate) fn enxugar(&mut self) {
        for i in 0..self.nos.len() {
            let mut voltas = 0;
            while self.nos[i].malhas.is_empty() && self.nos[i].filhos.len() == 1 && voltas < PROFUNDIDADE {
                voltas += 1;
                let filho = self.nos[i].filhos[0];
                if filho == i || filho >= self.nos.len() {
                    break;
                }
                let f = self.nos[filho].clone();
                let n = &mut self.nos[i];
                n.local = compor(&n.local, &f.local);
                n.malhas = f.malhas;
                n.filhos = f.filhos;
                if n.nome.is_empty() {
                    n.nome = f.nome;
                }
            }
            limpar(&mut self.nos[i].local);
        }
    }

    /// Escala e espelho entram na malha: a posição de uma peça só gira e desloca. A cópia fica neste nó;
    /// quem usa o filho por outro caminho segue vendo o original.
    pub(crate) fn assentar(&mut self) {
        for i in 0..self.nos.len() {
            if e_rigida(&self.nos[i].local) {
                continue;
            }
            let matriz = self.nos[i].local;
            let mut so_este = Cena { nos: self.nos.clone(), raizes: vec![i], ..Default::default() };
            so_este.nos[i].local = IDENTIDADE;
            let (ocorrencias, _) = so_este.ocorrencias();
            let mut malhas = Vec::with_capacity(ocorrencias.len());
            for (_, m, local) in ocorrencias {
                let Some(origem) = self.malhas.get(m) else { continue };
                let copia = origem.transformada(&compor(&matriz, &local));
                malhas.push(self.malhas.len());
                self.malhas.push(copia);
            }
            let n = &mut self.nos[i];
            n.local = IDENTIDADE;
            n.malhas = malhas;
            n.filhos.clear();
        }
    }
}

#[cfg(test)]
pub(crate) mod testes {
    use super::*;

    /// Cubo de lado `l` com o canto na origem e as normais para fora.
    pub(crate) fn cubo(nome: &str, l: f64) -> Malha {
        let pos = (0..8).map(|i| [(i & 1) as f64 * l, ((i >> 1) & 1) as f64 * l, ((i >> 2) & 1) as f64 * l]).collect();
        let tris = vec![[0, 2, 3], [0, 3, 1], [4, 5, 7], [4, 7, 6], [0, 1, 5], [0, 5, 4], [2, 6, 7], [2, 7, 3], [0, 4, 6], [0, 6, 2], [1, 3, 7], [1, 7, 5]];
        Malha { nome: nome.into(), pos, tris, ..Default::default() }
    }

    #[test]
    fn cubo_fecha_com_volume_certo() {
        let m = cubo("c", 2.);
        assert!(m.fechada());
        assert!((m.volume() - 8.).abs() < 1e-12, "{}", m.volume());
        assert!((m.area() - 24.).abs() < 1e-12);
    }

    #[test]
    fn soldar_junta_triangulos_soltos_e_fecha() {
        let c = cubo("c", 1.);
        let solto = Malha { pos: c.tris.iter().flatten().map(|i| c.pos[*i as usize]).collect(), tris: (0..12).map(|i| [i * 3, i * 3 + 1, i * 3 + 2]).collect(), ..Default::default() };
        assert!(!solto.fechada());
        let s = solto.soldada();
        assert_eq!(s.pos.len(), 8);
        assert!(s.fechada());
    }

    #[test]
    fn componentes_separa_partes_sem_vertice_em_comum() {
        let mut m = cubo("c", 1.);
        let outro = cubo("d", 1.).transformada(&[1., 0., 0., 5., 0., 1., 0., 0., 0., 0., 1., 0.]);
        let base = m.pos.len() as u32;
        m.pos.extend(outro.pos);
        m.tris.extend(outro.tris.iter().map(|t| t.map(|i| i + base)));
        let partes = m.componentes();
        assert_eq!(partes.len(), 2);
        assert!(partes.iter().all(|p| p.fechada() && p.pos.len() == 8));
    }

    #[test]
    fn espelho_inverte_triangulos_e_mantem_volume_positivo() {
        let m = cubo("c", 1.).transformada(&[-1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0.]);
        assert!((m.volume() - 1.).abs() < 1e-12);
    }

    #[test]
    fn enxugar_tira_o_embrulho_e_mantem_a_peca_compartilhada() {
        let mut c = Cena::default();
        let peca = c.raiz_com(cubo("Pino", 1.));
        c.raizes.clear();
        for x in [10., 20.] {
            let mut n = No::novo("");
            n.local[3] = x;
            n.filhos = vec![peca];
            c.nos.push(n);
        }
        let (a, b) = (c.nos.len() - 2, c.nos.len() - 1);
        let mut topo = No::novo("Topo");
        topo.filhos = vec![a, b];
        c.nos.push(topo);
        c.raizes = vec![c.nos.len() - 1];
        c.enxugar();
        assert_eq!(c.nos[a].malhas, vec![0]);
        assert_eq!(c.nos[a].nome, "Pino");
        assert_eq!(c.ocorrencias().0.len(), 2);
        assert_eq!(c.malhas.len(), 1, "a malha continua uma só");
    }

    #[test]
    fn ciclo_de_nos_nao_trava() {
        let mut c = Cena::default();
        let mut a = No::novo("A");
        a.filhos = vec![1];
        let mut b = No::novo("B");
        b.filhos = vec![0];
        c.nos = vec![a, b];
        c.raizes = vec![0];
        c.enxugar();
        let (o, inteiro) = c.ocorrencias();
        assert!(o.is_empty() && !inteiro);
    }

    #[test]
    fn escala_no_no_entra_na_malha_e_vira_posicao_rigida() {
        let mut c = Cena::default();
        let i = c.raiz_com(cubo("c", 1.));
        c.nos[i].local = [2., 0., 0., 5., 0., 2., 0., 0., 0., 0., 2., 0.];
        c.assentar();
        assert_eq!(c.nos[i].local, IDENTIDADE);
        let l = c.no_lugar();
        assert!((l[0].volume() - 8.).abs() < 1e-12);
        assert_eq!(l[0].pos[0], [5., 0., 0.]);
    }
}
