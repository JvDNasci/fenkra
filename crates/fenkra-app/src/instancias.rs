// SPDX-License-Identifier: LGPL-2.1-or-later
//! Desenho das cópias da montagem na placa de vídeo (B125).
//!
//! A cena plana manda cada vértice de cada cópia num buffer único: cem peças iguais passavam de 600 MB
//! e batiam no maior buffer que a placa aceita. Aqui cada nível de cada protótipo sobe uma vez, direto
//! dos vetores da malha, e cada quadro só escreve a lista de cópias (96 bytes por cópia).
//!
//! A carga é leve: nada sobe antes de uma cópia pedir. A primeira subida de um protótipo é a do nível
//! mais grosso, para a peça aparecer logo, e a fina vem depois, dentro de uma cota por quadro. Sem o
//! nível pedido, a cópia usa o mais próximo que já está na placa. Passando do orçamento de memória, o
//! nível fino que ninguém usa neste quadro é descartado.

use crate::render::Camera;
use eframe::egui_wgpu::wgpu;
use fenkra_montagem::lod::{self, Nivel, Prototipo};
use glam::{Mat4, Vec3, Vec4};
use std::collections::HashMap;
use std::sync::Arc;
use wgpu::util::DeviceExt;

/// Bytes de malha subidos por quadro: mantém a subida longe do limite de 100 ms por quadro.
const COTA_QUADRO: u64 = 48 << 20;
/// Tempo de subida por quadro. Criar buffer custa por buffer, não só por byte: peças diferentes
/// demais de uma vez seguravam o quadro mesmo com pouca memória.
const TEMPO_QUADRO: std::time::Duration = std::time::Duration::from_millis(12);
/// Memória de malha que a montagem pode ocupar na placa. O wgpu não informa a memória livre.
const ORCAMENTO: u64 = 1 << 30;

/// O que o desenho precisa de uma cópia.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanciaGpu {
    /// três linhas da matriz de posição
    pub m: [[f32; 4]; 3],
    pub col: [f32; 4],
    /// rugosidade, metalicidade, opacidade, textura
    pub mat: [f32; 4],
    pub aresta: [f32; 4],
}

/// Uma cópia na cena.
#[derive(Clone, Debug)]
pub struct Copia {
    pub gpu: InstanciaGpu,
    /// esfera envolvente no mundo
    pub centro: Vec3,
    pub raio: f32,
    pub simplificada: bool,
}

/// As cópias de um protótipo.
#[derive(Clone)]
pub struct Lote {
    pub prototipo: Arc<Prototipo>,
    pub copias: Vec<Copia>,
}

/// Quantas cópias e triângulos o último quadro desenhou, por nível.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Contagem {
    pub copias: u64,
    pub triangulos: u64,
    pub bytes_placa: u64,
}

/// Um nível na placa: posições, normais, índices e arestas num buffer só.
struct NivelGpu {
    buf: wgpu::Buffer,
    /// onde cada parte começa e termina
    pos: std::ops::Range<u64>,
    nrm: std::ops::Range<u64>,
    idx: std::ops::Range<u64>,
    /// arestas vivas primeiro, depois as de tangência
    arestas: std::ops::Range<u64>,
    n_idx: u32,
    n_vivas: u32,
    n_todas: u32,
    bytes: u64,
    /// último quadro em que foi desenhado
    uso: u64,
}

#[derive(Default)]
struct ProtoGpu {
    niveis: Vec<Option<NivelGpu>>,
}

struct Desenho {
    proto: u64,
    nivel: usize,
    primeira: u32,
    qtd: u32,
}

/// Parâmetros do quadro que mudam a escolha do nível.
#[derive(Clone, Copy)]
pub struct Quadro {
    pub camera: Camera,
    pub tamanho: [u32; 2],
    pub limiar_px: f32,
}

#[derive(Default)]
pub struct Instanciador {
    protos: HashMap<u64, ProtoGpu>,
    lotes: Vec<Lote>,
    tangentes: bool,
    inst: Option<wgpu::Buffer>,
    cap: u64,
    desenhos: Vec<Desenho>,
    quadro: u64,
    /// ainda há nível pedido fora da placa: o próximo quadro continua subindo
    pub subindo: bool,
    /// protótipo maior que o maior buffer da placa, nem no nível mais grosso
    pub grande_demais: bool,
    pub contagem: Contagem,
}

/// Malha de um nível em forma de buffers: arestas vivas primeiro.
fn arestas_ordenadas(m: &fenkra_kernel::Mesh) -> (Vec<f32>, u32) {
    let mut vivas = Vec::with_capacity(m.edges.len());
    if m.edge_smooth.len() != m.edge_ranges.len() {
        vivas.extend_from_slice(&m.edges);
        let n = (vivas.len() / 3) as u32;
        return (vivas, n);
    }
    let nseg = m.edges.len() / 6;
    let trecho = |e: usize| {
        let s0 = m.edge_ranges[e] as usize;
        let s1 = m.edge_ranges.get(e + 1).map(|x| *x as usize).unwrap_or(nseg);
        &m.edges[s0 * 6..s1 * 6]
    };
    for (e, _) in m.edge_smooth.iter().enumerate().filter(|(_, s)| **s == 0) {
        vivas.extend_from_slice(trecho(e));
    }
    let n = (vivas.len() / 3) as u32;
    for (e, _) in m.edge_smooth.iter().enumerate().filter(|(_, s)| **s != 0) {
        vivas.extend_from_slice(trecho(e));
    }
    (vivas, n)
}

fn bytes_do_nivel(n: &Nivel) -> u64 {
    let m = &n.malha;
    (std::mem::size_of_val(m.positions.as_slice()) * 2 + std::mem::size_of_val(m.indices.as_slice()) + std::mem::size_of_val(m.edges.as_slice())) as u64
}

fn subir(dev: &wgpu::Device, n: &Nivel) -> Option<NivelGpu> {
    let m = &n.malha;
    let bytes = bytes_do_nivel(n);
    if bytes > dev.limits().max_buffer_size || m.indices.is_empty() {
        return None;
    }
    let (arestas, n_vivas) = arestas_ordenadas(m);
    let mut dados: Vec<u8> = Vec::with_capacity(bytes as usize);
    let mut parte = |b: &[u8]| {
        let inicio = dados.len() as u64;
        dados.extend_from_slice(b);
        inicio..dados.len() as u64
    };
    let pos = parte(bytemuck::cast_slice(&m.positions));
    let nrm = parte(bytemuck::cast_slice(&m.normals));
    let idx = parte(bytemuck::cast_slice(&m.indices));
    let ar = parte(bytemuck::cast_slice(&arestas));
    let buf = dev.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some("instancia"), contents: &dados, usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::INDEX });
    Some(NivelGpu { buf, pos, nrm, idx, arestas: ar, n_idx: m.indices.len() as u32, n_vivas, n_todas: (arestas.len() / 3) as u32, bytes, uso: 0 })
}

/// Planos do volume de visão (normal para dentro), da matriz de projeção e vista.
pub fn planos_de_visao(vp: Mat4) -> [Vec4; 6] {
    let (r0, r1, r2, r3) = (vp.row(0), vp.row(1), vp.row(2), vp.row(3));
    // profundidade do wgpu em [0, 1]: o plano de perto é a própria linha 2
    [r3 + r0, r3 - r0, r3 + r1, r3 - r1, r2, r3 - r2].map(|p| p / p.truncate().length().max(1e-12))
}

/// A esfera toca o volume de visão?
pub fn na_visao(planos: &[Vec4; 6], centro: Vec3, raio: f32) -> bool {
    planos.iter().all(|p| p.truncate().dot(centro) + p.w >= -raio)
}

/// Nível que esta cópia pede neste quadro.
pub fn nivel_pedido(p: &Prototipo, c: &Copia, q: &Quadro) -> usize {
    if c.simplificada {
        return p.mais_grosso();
    }
    // a parte da peça mais perto da câmera é a que mostra o erro
    let perto = (c.centro - q.camera.eye()).length() - c.raio;
    let px = q.camera.px_por_mm(perto, q.tamanho[1] as f32);
    lod::nivel_para_tela(&p.desvios(), px, q.limiar_px)
}

/// Nível já na placa mais próximo do pedido, preferindo o mais grosso (é o que aparece mais rápido).
fn nivel_na_placa(niveis: &[Option<NivelGpu>], pedido: usize) -> Option<usize> {
    (pedido..niveis.len()).chain((0..pedido).rev()).find(|n| niveis[*n].is_some())
}

impl Instanciador {
    /// Troca as cópias da cena. O que sobe para a placa fica, se o protótipo continuar na cena.
    pub fn definir(&mut self, lotes: Vec<Lote>, tangentes: bool) {
        self.protos.retain(|id, _| lotes.iter().any(|l| l.prototipo.id == *id));
        self.lotes = lotes;
        self.tangentes = tangentes;
    }

    pub fn tem_lotes(&self) -> bool {
        !self.lotes.is_empty()
    }

    fn bytes_na_placa(&self) -> u64 {
        self.protos.values().flat_map(|p| p.niveis.iter().flatten()).map(|n| n.bytes).sum()
    }

    /// Libera níveis finos parados até `falta` bytes caberem no orçamento. Devolve se coube.
    fn abrir_espaco(&mut self, falta: u64) -> bool {
        let mut usado = self.bytes_na_placa();
        while usado + falta > ORCAMENTO {
            let quadro = self.quadro;
            // o nível mais grosso de cada protótipo nunca sai: é a garantia de a peça aparecer
            let vitima = self
                .protos
                .iter()
                .flat_map(|(id, p)| {
                    let ultimo = p.niveis.iter().rposition(|n| n.is_some());
                    p.niveis.iter().enumerate().filter(move |(k, n)| n.as_ref().is_some_and(|x| x.uso != quadro) && Some(*k) != ultimo).map(move |(k, n)| (*id, k, n.as_ref().map_or(0, |x| x.uso)))
                })
                .min_by_key(|x| x.2);
            let Some((id, k, _)) = vitima else { return false };
            if let Some(n) = self.protos.get_mut(&id).and_then(|p| p.niveis[k].take()) {
                usado -= n.bytes;
            }
        }
        true
    }

    /// Escolhe o nível de cada cópia, sobe o que couber e escreve a lista de cópias.
    pub fn preparar(&mut self, dev: &wgpu::Device, fila: &wgpu::Queue, q: &Quadro) {
        self.quadro += 1;
        let planos = planos_de_visao(q.camera.view_proj(q.tamanho[0] as f32 / q.tamanho[1].max(1) as f32));
        // 1. o que cada cópia visível pede
        let mut pedidos: Vec<(usize, usize, usize)> = vec![];
        for (li, l) in self.lotes.iter().enumerate() {
            for (ci, c) in l.copias.iter().enumerate() {
                if na_visao(&planos, c.centro, c.raio) {
                    pedidos.push((li, ci, nivel_pedido(&l.prototipo, c, q)));
                }
            }
        }
        // 2. sobe primeiro o nível mais grosso de quem não tem nada, depois os pedidos
        self.subindo = false;
        self.grande_demais = false;
        let mut cota = COTA_QUADRO;
        let inicio = std::time::Instant::now();
        let mut faltas: Vec<(usize, usize)> = vec![];
        for &(li, _, pedido) in &pedidos {
            let p = &self.lotes[li].prototipo;
            let g = self.protos.entry(p.id).or_default();
            g.niveis.resize_with(p.niveis.len(), || None);
            if g.niveis.iter().all(Option::is_none) {
                faltas.push((li, p.mais_grosso()));
            }
            if g.niveis[pedido].is_none() {
                faltas.push((li, pedido));
            }
        }
        // do mais grosso para o mais fino, sem repetir
        faltas.sort_by_key(|(li, n)| (self.lotes[*li].prototipo.mais_grosso() - *n, self.lotes[*li].prototipo.id));
        faltas.dedup_by_key(|(li, n)| (self.lotes[*li].prototipo.id, *n));
        for (li, n) in faltas {
            let p = self.lotes[li].prototipo.clone();
            if self.protos.get(&p.id).is_some_and(|g| g.niveis[n].is_some()) {
                continue;
            }
            let bytes = bytes_do_nivel(&p.niveis[n]);
            let primeiro = self.protos.get(&p.id).is_none_or(|g| g.niveis.iter().all(Option::is_none));
            // o que sobra sobe no quadro seguinte; a peça grande demais para a cota passa sozinha
            if inicio.elapsed() >= TEMPO_QUADRO || (bytes > cota && cota < COTA_QUADRO) {
                self.subindo = true;
                continue;
            }
            if !self.abrir_espaco(bytes) && !primeiro {
                continue;
            }
            match subir(dev, &p.niveis[n]) {
                Some(g) => {
                    cota = cota.saturating_sub(bytes);
                    if let Some(x) = self.protos.get_mut(&p.id) {
                        x.niveis[n] = Some(g);
                    }
                }
                None if !p.niveis[n].malha.indices.is_empty() && primeiro => self.grande_demais = true,
                None => {}
            }
        }
        // 3. agrupa por protótipo e nível
        let mut grupos: HashMap<(u64, usize), Vec<InstanciaGpu>> = HashMap::new();
        for (li, ci, pedido) in pedidos {
            let l = &self.lotes[li];
            let Some(g) = self.protos.get(&l.prototipo.id) else { continue };
            if let Some(n) = nivel_na_placa(&g.niveis, pedido) {
                grupos.entry((l.prototipo.id, n)).or_default().push(l.copias[ci].gpu);
            }
        }
        // 4. escreve a lista
        self.desenhos.clear();
        let mut dados: Vec<InstanciaGpu> = Vec::with_capacity(grupos.values().map(Vec::len).sum());
        let mut chaves: Vec<(u64, usize)> = grupos.keys().copied().collect();
        chaves.sort_unstable();
        let mut contagem = Contagem::default();
        for k in chaves {
            let v = &grupos[&k];
            self.desenhos.push(Desenho { proto: k.0, nivel: k.1, primeira: dados.len() as u32, qtd: v.len() as u32 });
            dados.extend_from_slice(v);
            if let Some(n) = self.protos.get_mut(&k.0).and_then(|p| p.niveis[k.1].as_mut()) {
                n.uso = self.quadro;
                contagem.copias += v.len() as u64;
                contagem.triangulos += v.len() as u64 * n.n_idx as u64 / 3;
            }
        }
        contagem.bytes_placa = self.bytes_na_placa();
        self.contagem = contagem;
        let bytes = std::mem::size_of_val(dados.as_slice()) as u64;
        if bytes > self.cap || self.inst.is_none() {
            self.cap = bytes.max(4096).next_power_of_two();
            self.inst = Some(dev.create_buffer(&wgpu::BufferDescriptor { label: Some("copias"), size: self.cap, usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false }));
        }
        if let (Some(b), false) = (&self.inst, dados.is_empty()) {
            fila.write_buffer(b, 0, bytemuck::cast_slice(&dados));
        }
    }

    /// Desenha as faces com o pipeline já escolhido.
    pub fn faces<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        let Some(inst) = &self.inst else { return };
        pass.set_vertex_buffer(2, inst.slice(..));
        for d in &self.desenhos {
            let Some(n) = self.protos.get(&d.proto).and_then(|p| p.niveis[d.nivel].as_ref()) else { continue };
            pass.set_vertex_buffer(0, n.buf.slice(n.pos.clone()));
            pass.set_vertex_buffer(1, n.buf.slice(n.nrm.clone()));
            pass.set_index_buffer(n.buf.slice(n.idx.clone()), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..n.n_idx, 0, d.primeira..d.primeira + d.qtd);
        }
    }

    /// Desenha as arestas com o pipeline já escolhido.
    pub fn arestas<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        let Some(inst) = &self.inst else { return };
        pass.set_vertex_buffer(1, inst.slice(..));
        for d in &self.desenhos {
            let Some(n) = self.protos.get(&d.proto).and_then(|p| p.niveis[d.nivel].as_ref()) else { continue };
            if n.arestas.is_empty() {
                continue;
            }
            pass.set_vertex_buffer(0, n.buf.slice(n.arestas.clone()));
            pass.draw(0..if self.tangentes { n.n_todas } else { n.n_vivas }, d.primeira..d.primeira + d.qtd);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fenkra_core::QualidadeMalha;
    use fenkra_kernel::Mesh;

    fn prototipo() -> Arc<Prototipo> {
        let m = |t: usize| Mesh { positions: vec![0., 0., 0., 100., 0., 0., 0., 100., 0.], indices: vec![0; t * 3], ..Default::default() };
        Prototipo::novo(vec![(m(1000), QualidadeMalha::NORMAL), (m(400), QualidadeMalha::RASCUNHO), (m(100), lod::LONGE)])
    }

    fn copia(centro: Vec3, simplificada: bool) -> Copia {
        let gpu = InstanciaGpu { m: [[1., 0., 0., 0.], [0., 1., 0., 0.], [0., 0., 1., 0.]], col: [1.; 4], mat: [0.; 4], aresta: [0.; 4] };
        Copia { gpu, centro, raio: 50., simplificada }
    }

    fn quadro(dist: f32) -> Quadro {
        Quadro { camera: Camera { dist, ..Default::default() }, tamanho: [1000, 800], limiar_px: lod::LIMIAR_PX }
    }

    #[test]
    fn copia_perto_pede_a_fina_e_longe_a_grossa() {
        let p = prototipo();
        let c = copia(Vec3::ZERO, false);
        assert_eq!(nivel_pedido(&p, &c, &quadro(300.)), 0);
        assert_eq!(nivel_pedido(&p, &c, &quadro(100_000.)), p.mais_grosso());
    }

    #[test]
    fn simplificada_fica_no_nivel_mais_grosso_mesmo_de_perto() {
        let p = prototipo();
        assert_eq!(nivel_pedido(&p, &copia(Vec3::ZERO, true), &quadro(300.)), p.mais_grosso());
    }

    #[test]
    fn fora_da_visao_nao_e_desenhada() {
        let q = quadro(300.);
        let planos = planos_de_visao(q.camera.view_proj(1.25));
        assert!(na_visao(&planos, Vec3::ZERO, 10.));
        assert!(!na_visao(&planos, Vec3::new(5000., -5000., 0.), 10.));
        assert!(!na_visao(&planos, q.camera.eye() + (q.camera.eye() - q.camera.target), 10.), "atrás da câmera");
    }

    #[test]
    fn esfera_que_encosta_na_borda_ainda_conta() {
        let q = quadro(300.);
        let planos = planos_de_visao(q.camera.view_proj(1.25));
        let longe = Vec3::new(5000., -5000., 0.);
        assert!(na_visao(&planos, longe, 8000.));
    }

    #[test]
    fn arestas_vivas_vem_primeiro() {
        let m = Mesh { edges: vec![1.; 6].into_iter().chain(vec![2.; 6]).chain(vec![3.; 6]).collect(), edge_ranges: vec![0, 1, 2], edge_smooth: vec![1, 0, 0], ..Default::default() };
        let (v, vivas) = arestas_ordenadas(&m);
        assert_eq!(vivas, 4, "dois segmentos vivos");
        assert_eq!(v.len(), 18);
        assert_eq!((v[0], v[6], v[12]), (2., 3., 1.));
    }

    /// Montagem de `n` placas em pasta temporária.
    fn estado(nome: &str, n: usize) -> crate::asm_ui::AsmState {
        let dir = std::env::temp_dir().join(format!("fenkra_inst_{nome}_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("placa.fkp"), fenkra_solidos::example_document().to_json()).unwrap();
        let mut a = fenkra_montagem::Assembly::new("g");
        for k in 0..n {
            a.add_component("Placa", fenkra_montagem::NodeKind::Part { path: "placa.fkp".into() }, fenkra_montagem::Placement { origin: [k as f64 * 150., 0., 0.], ..Default::default() }, None);
        }
        crate::asm_ui::AsmState::new(a, Some(dir.join("g.fka")))
    }

    fn cena(st: &crate::asm_ui::AsmState, curvatura: Option<crate::sup_analise::EscalaCurvatura>, hilite: Option<fenkra_core::Id>) -> crate::render::Scene {
        let (sel, filtro) = (crate::app::Sel::None, crate::settings::Filter::default());
        let o = crate::scene::SceneOpts { instancias: &st.instancias, forma_oculta: None, sombra_perfil: true, editing: None, sel: &sel, filter: &filtro, hidden_env: false, dark_edges: true, tangent_edges: false, hilite, hilite_extra: &[], line_w: 1., line_dash: 0, point_style: 0, curvatura: curvatura.map(Into::into), fantasma: None };
        crate::scene::build(&st.view_doc, st.view.as_ref().unwrap(), &o)
    }

    #[test]
    fn cena_da_montagem_nao_repete_a_malha() {
        let mut st = estado("cena", 5);
        st.refresh();
        let s = cena(&st, None, None);
        assert!(s.mesh.is_empty() && s.edges.is_empty(), "a malha fica no protótipo");
        assert_eq!(s.lotes.len(), 1);
        assert_eq!(s.lotes[0].copias.len(), 5);
        let (lo, hi) = s.bounds.unwrap();
        let caixa = s.lotes[0].prototipo.caixa;
        assert!((hi.x - lo.x - (600. + caixa.1[0] - caixa.0[0])).abs() < 1e-2, "a caixa da cena cobre as cinco cópias");
        assert_eq!(s.lotes[0].copias[3].gpu.m[0][3], 450., "posição na matriz");
    }

    #[test]
    fn matriz_da_copia_leva_a_malha_ao_mesmo_lugar_da_vista() {
        let mut st = estado("matriz", 2);
        let id = st.asm.nodes[1].id;
        let n = st.asm.node_mut(id).unwrap();
        n.placement.rot = [[0., -1., 0.], [1., 0., 0.], [0., 0., 1.]];
        n.placement.origin = [10., 20., 30.];
        st.refresh();
        let s = cena(&st, None, None);
        let c = &s.lotes[0].copias[1].gpu;
        let local = s.lotes[0].prototipo.fina();
        let mundo = &st.view.as_ref().unwrap().bodies[1].mesh;
        for k in [0usize, 7, 31] {
            let p = &local.positions[k * 3..k * 3 + 3];
            for r in 0..3 {
                let w = c.m[r][0] * p[0] + c.m[r][1] * p[1] + c.m[r][2] * p[2] + c.m[r][3];
                assert!((w - mundo.positions[k * 3 + r]).abs() < 1e-3);
            }
        }
    }

    #[test]
    fn componente_oculto_sai_da_lista_e_realce_muda_so_a_copia() {
        let mut st = estado("oculto", 3);
        let (a, b) = (st.asm.nodes[0].id, st.asm.nodes[1].id);
        st.asm.node_mut(b).unwrap().visible = false;
        st.refresh();
        let s = cena(&st, None, Some(a));
        assert_eq!(s.lotes[0].copias.len(), 2);
        assert_eq!(s.lotes[0].copias[0].gpu.col, s.lotes[0].copias[1].gpu.col, "o realce não pinta a peça");
        assert_ne!(s.lotes[0].copias[0].gpu.aresta, s.lotes[0].copias[1].gpu.aresta, "só as arestas da realçada acendem");
    }

    #[test]
    fn mapa_de_curvatura_volta_para_a_cena_plana() {
        let mut st = estado("curv", 2);
        st.refresh();
        let mut ajustes = crate::settings::Settings::default();
        ajustes.display = crate::render::DisplayMode::Curvatura;
        let s = cena(&st, crate::sup_analise::escala_da_tela(&ajustes, &eframe::egui::Visuals::dark()), None);
        assert!(s.lotes.is_empty() && !s.mesh.is_empty());
    }

    #[test]
    fn composicao_em_fundo_entrega_o_mesmo_que_a_composicao_direta() {
        let ctx = eframe::egui::Context::default();
        let mut st = estado("fundo", 4);
        st.refresh_fundo(&ctx);
        assert!(st.compondo && st.view.is_none(), "o quadro não espera a composição");
        let t = std::time::Instant::now();
        while st.compondo && t.elapsed().as_secs() < 120 {
            st.refresh_fundo(&ctx);
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(!st.compondo, "a resposta chegou");
        let fundo = st.view.take().unwrap();
        assert!(st.instancias[0].prototipo.niveis.len() >= 2, "a thread gera os níveis");
        st.dirty = true;
        st.refresh();
        let direta = st.view.as_ref().unwrap();
        assert_eq!(fundo.bodies.len(), 4);
        for (a, b) in fundo.bodies.iter().zip(&direta.bodies) {
            assert_eq!(a.mesh.positions, b.mesh.positions);
            assert_eq!(a.mass.volume, b.mass.volume);
        }
    }

    #[test]
    fn copia_acompanha_o_componente_sem_esperar_a_composicao() {
        let ctx = eframe::egui::Context::default();
        let mut st = estado("vivo", 2);
        st.refresh();
        let id = st.asm.nodes[1].id;
        st.asm.node_mut(id).unwrap().placement.origin = [999., 0., 0.];
        st.dirty = true;
        assert!(st.refresh_fundo(&ctx), "a cena refaz neste quadro");
        assert_eq!(st.instancias[1].posicao.origin[0], 999.);
    }

    #[test]
    fn sem_o_nivel_pedido_usa_o_mais_proximo_na_placa() {
        let sem: Vec<Option<NivelGpu>> = vec![None, None, None];
        assert_eq!(nivel_na_placa(&sem, 1), None);
    }
}
