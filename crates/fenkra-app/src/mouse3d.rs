// SPDX-License-Identifier: LGPL-2.1-or-later
//! Mouse 3D (B322): navegação da câmera em 6 eixos a partir de aparelhos de seis graus de liberdade.
//!
//! Camadas, da mais próxima do aparelho à mais próxima da tela:
//! 1. `decodificar_relatorio`: bytes de um relatório HID viram movimento (eixos) ou botões.
//! 2. `Mouse3dConfig::processar`: zona morta, sensibilidade, inversão por eixo e eixo dominante.
//! 3. `aplicar_na_camera` e `Acao`: o movimento anda a câmera; os botões chamam ações de vista.
//!
//! A leitura do aparelho em si fica em `mouse3d_hid.rs` e alimenta `Mouse3d::empurrar`. Sem aparelho
//! nada chega e nada muda. A pesquisa está em `docs/pesquisa/mouse-3d.md`.

use crate::render::Camera;
use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Deflexão máxima que os aparelhos entregam nos relatórios (os de uso comum chegam perto de 350;
/// o valor é só a escala para -1..1 e o ajuste fino fica na sensibilidade).
pub const ESCALA_BRUTA: f32 = 350.;

/// Ordem dos eixos: translação x (direita), y (frente), z (cima) e rotação x (inclinar para a
/// frente), y (inclinar para o lado), z (torcer).
pub const NOMES_EIXOS: [&str; 6] = ["mover para os lados", "mover para frente", "subir e descer", "inclinar para a frente", "inclinar para o lado", "torcer"];

/// O que um botão do aparelho faz. Só ações de vista: o que muda a câmera ou o modo dela.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Acao {
    Nenhuma,
    Enquadrar,
    AlternarOrtogonal,
    VistaFrente,
    VistaTopo,
    VistaIsometrica,
}

impl Acao {
    pub const TODAS: [Acao; 6] = [Acao::Nenhuma, Acao::Enquadrar, Acao::AlternarOrtogonal, Acao::VistaFrente, Acao::VistaTopo, Acao::VistaIsometrica];
    pub fn nome(self) -> &'static str {
        match self {
            Acao::Nenhuma => "nenhuma",
            Acao::Enquadrar => "enquadrar tudo",
            Acao::AlternarOrtogonal => "alternar ortogonal",
            Acao::VistaFrente => "vista frontal",
            Acao::VistaTopo => "vista superior",
            Acao::VistaIsometrica => "vista isométrica",
        }
    }
    /// Efeito na câmera. `Enquadrar` precisa dos limites do modelo, que só a tela conhece: devolve
    /// `false` e quem chamou enquadra.
    pub fn aplicar(self, cam: &mut Camera) -> bool {
        match self {
            Acao::Nenhuma => true,
            Acao::Enquadrar => false,
            Acao::AlternarOrtogonal => {
                cam.ortho = !cam.ortho;
                true
            }
            Acao::VistaFrente => {
                cam.set_view(-std::f32::consts::FRAC_PI_2, 0.);
                true
            }
            Acao::VistaTopo => {
                cam.set_view(-std::f32::consts::FRAC_PI_2, 1.5706);
                true
            }
            Acao::VistaIsometrica => {
                cam.set_view(-0.785, 0.6155);
                true
            }
        }
    }
}

/// Opções do usuário; ficam em `Settings::mouse3d`.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Mouse3dConfig {
    /// usa o aparelho quando houver um (sem aparelho, nada muda)
    pub ativo: bool,
    /// multiplicador das três translações (0,1 a 4)
    pub sens_translacao: f32,
    /// multiplicador das três rotações (0,1 a 4)
    pub sens_rotacao: f32,
    /// inverte cada um dos seis eixos (ordem de `NOMES_EIXOS`)
    pub inverter: [bool; 6],
    /// só o eixo de maior deflexão age: evita girar quando a intenção era só aproximar
    pub eixo_dominante: bool,
    /// fração da deflexão ignorada em torno do repouso (0 a 0,5)
    pub zona_morta: f32,
    /// ação de cada botão, pelo número do botão (a lista pode ser menor que o número de botões)
    pub botoes: Vec<Acao>,
    /// comando de cada botão, pelo ícone do comando (o mesmo da busca Ctrl+K); vazio = vale `botoes`
    pub comandos: Vec<String>,
    /// na montagem, com uma peça escolhida, o aparelho move a peça em vez da vista
    pub mover_peca: bool,
}

impl Default for Mouse3dConfig {
    fn default() -> Self {
        Mouse3dConfig {
            ativo: true,
            sens_translacao: 1.0,
            sens_rotacao: 1.0,
            inverter: [false; 6],
            eixo_dominante: false,
            zona_morta: 0.05,
            botoes: vec![Acao::Enquadrar, Acao::AlternarOrtogonal],
            comandos: vec![],
            mover_peca: true,
        }
    }
}

/// Movimento já tratado, em fração da velocidade máxima (-1 a 1 por eixo).
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct Movimento {
    pub eixos: [f32; 6],
}

impl Movimento {
    pub fn parado(&self) -> bool {
        self.eixos.iter().all(|v| v.abs() < 1e-6)
    }
}

impl Mouse3dConfig {
    /// Sensibilidades e zona morta dentro dos limites que a tela oferece (arquivo editado à mão).
    pub fn saneada(&self) -> Mouse3dConfig {
        let f = |v: f32, lo: f32, hi: f32, pad: f32| if v.is_finite() { v.clamp(lo, hi) } else { pad };
        Mouse3dConfig { sens_translacao: f(self.sens_translacao, 0.1, 4., 1.), sens_rotacao: f(self.sens_rotacao, 0.1, 4., 1.), zona_morta: f(self.zona_morta, 0., 0.5, 0.05), ..self.clone() }
    }

    /// Zona morta, sensibilidade, inversão e eixo dominante sobre a leitura bruta (-1..1 por eixo).
    pub fn processar(&self, bruto: [f32; 6]) -> Movimento {
        let c = self.saneada();
        let mut e = [0f32; 6];
        for i in 0..6 {
            let v = bruto[i].clamp(-1., 1.);
            // fora da zona morta o valor recomeça do zero, sem degrau
            let a = v.abs();
            let mut s = if a <= c.zona_morta { 0. } else { (a - c.zona_morta) / (1. - c.zona_morta) };
            s *= v.signum();
            s *= if i < 3 { c.sens_translacao } else { c.sens_rotacao };
            if c.inverter[i] {
                s = -s;
            }
            e[i] = s.clamp(-1., 1.);
        }
        if c.eixo_dominante {
            let (mut melhor, mut maior) = (0, 0f32);
            for (i, v) in e.iter().enumerate() {
                if v.abs() > maior {
                    maior = v.abs();
                    melhor = i;
                }
            }
            for (i, v) in e.iter_mut().enumerate() {
                if i != melhor {
                    *v = 0.;
                }
            }
        }
        Movimento { eixos: e }
    }

    pub fn acao_do_botao(&self, n: u8) -> Acao {
        self.botoes.get(n as usize).copied().unwrap_or(Acao::Nenhuma)
    }

    /// Ícone do comando ligado ao botão, se houver (tem prioridade sobre a ação de vista).
    pub fn comando_do_botao(&self, n: u8) -> Option<&str> {
        self.comandos.get(n as usize).map(String::as_str).filter(|c| !c.trim().is_empty())
    }
}

/// O que um botão faz na tela: um comando (a mesma ação da busca Ctrl+K) ou uma ação de vista.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Disparo {
    Comando(crate::toolbar::Act),
    Vista(Acao),
}

/// Resolve o botão: o comando gravado pelo ícone, achado na lista da busca de comandos; ícone que
/// não existe mais cai na ação de vista do botão.
pub fn disparo_do_botao(c: &Mouse3dConfig, n: u8) -> Disparo {
    if let Some(icone) = c.comando_do_botao(n) {
        if let Some(i) = crate::ajuda_cmd::comandos().into_iter().find(|i| i.icon == icone) {
            return Disparo::Comando(i.act);
        }
    }
    Disparo::Vista(c.acao_do_botao(n))
}

/// Deslocamento da peça na montagem por `dt` segundos, no referencial da vista: x para a direita da
/// tela, y para dentro dela, z para cima; inclinar para a frente gira em torno do eixo da direita,
/// inclinar para o lado em torno da direção de olhar e torcer em torno do eixo de cima. A velocidade
/// máxima atravessa um quarto da vista por segundo e gira 1,2 rad/s. Devolve (translação, vetor de
/// rotação), prontos para `Placement::moved`.
pub fn movimento_da_peca(cam: &Camera, m: &Movimento, dt: f32) -> Option<([f64; 3], [f64; 3])> {
    if m.parado() || !dt.is_finite() || dt <= 0. {
        return None;
    }
    let dt = dt.min(0.1);
    let [tx, ty, tz, rx, ry, rz] = m.eixos;
    let (r, u) = cam.basis();
    let f = u.cross(r);
    let t = (r * tx + f * ty + u * tz) * (cam.dist * 0.25 * dt);
    let w = (r * rx + f * ry + u * rz) * (1.2 * dt);
    let d = |v: Vec3| [v.x as f64, v.y as f64, v.z as f64];
    Some((d(t), d(w)))
}

/// Anda a câmera por `dt` segundos com o movimento dado. Translação: x e y deslocam o alvo no plano
/// da tela, z aproxima ou afasta; rotação: inclinar para a frente muda a elevação e inclinar para o
/// lado gira em torno da vertical. A câmera não tem rolagem, então "torcer" não age aqui.
pub fn aplicar_na_camera(cam: &mut Camera, m: &Movimento, dt: f32) {
    if m.parado() || !dt.is_finite() || dt <= 0. {
        return;
    }
    let dt = dt.min(0.1);
    let [tx, ty, tz, rx, ry, _rz] = m.eixos;
    // velocidade máxima: atravessar a vista (a distância ao alvo) em um segundo
    let (r, u) = cam.basis();
    let frente = Vec3::new(-cam.yaw.cos(), -cam.yaw.sin(), 0.);
    let _ = u;
    cam.target += (r * tx + frente * ty) * cam.dist * dt;
    cam.dist = (cam.dist * (tz * 1.2 * dt).exp()).clamp(0.5, 1e6);
    cam.pitch = (cam.pitch + rx * 1.6 * dt).clamp(-1.5706, 1.5706);
    cam.yaw -= ry * 1.6 * dt;
}

/// Decodifica um relatório HID de aparelho de seis eixos no formato mais comum: o primeiro byte é o
/// número do relatório. 1 = translação (3 inteiros de 16 bits, little-endian), 2 = rotação (idem),
/// 3 = botões (máscara de bits). Devolve `None` para relatório desconhecido ou curto.
pub fn decodificar_relatorio(b: &[u8]) -> Option<Evento> {
    let i16_em = |k: usize| -> Option<f32> { Some(i16::from_le_bytes([*b.get(k)?, *b.get(k + 1)?]) as f32 / ESCALA_BRUTA) };
    match *b.first()? {
        1 => Some(Evento::Translacao([i16_em(1)?, i16_em(3)?, i16_em(5)?])),
        2 => Some(Evento::Rotacao([i16_em(1)?, i16_em(3)?, i16_em(5)?])),
        3 => {
            let mut mascara = 0u32;
            for (k, byte) in b[1..].iter().take(4).enumerate() {
                mascara |= (*byte as u32) << (8 * k);
            }
            Some(Evento::Botoes(mascara))
        }
        _ => None,
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Evento {
    Translacao([f32; 3]),
    Rotacao([f32; 3]),
    Botoes(u32),
}

/// Estado do aparelho: últimos eixos lidos e botões. Quem lê o aparelho chama `empurrar`; a tela
/// pergunta `movimento` a cada quadro e `botoes_novos` para as transições.
#[derive(Default)]
pub struct Mouse3d {
    eixos: [f32; 6],
    botoes: u32,
    anteriores: u32,
    /// já chegou algum relatório (há aparelho): a interface só mostra o painel com isso
    pub presente: bool,
    fila: Option<std::sync::mpsc::Receiver<Evento>>,
    tentou: bool,
    /// a peça da montagem está sendo movida pelo aparelho (o desfazer entra uma vez por gesto)
    pub movendo_peca: bool,
    /// comandos pedidos pelos botões neste quadro, rodados depois dos painéis
    pub acoes: Vec<crate::toolbar::Act>,
}

impl Mouse3d {
    pub fn empurrar(&mut self, e: Evento) {
        self.presente = true;
        match e {
            Evento::Translacao(t) => self.eixos[..3].copy_from_slice(&t),
            Evento::Rotacao(r) => self.eixos[3..].copy_from_slice(&r),
            Evento::Botoes(m) => self.botoes = m,
        }
    }

    /// Procura o aparelho uma vez por execução; `ctx` acorda a tela quando chega movimento.
    pub fn iniciar(&mut self, ctx: &eframe::egui::Context) {
        if !self.tentou {
            self.tentou = true;
            self.fila = crate::mouse3d_hid::iniciar(ctx.clone());
        }
    }

    /// Passa para o estado tudo que o aparelho mandou desde o último quadro.
    pub fn drenar(&mut self) {
        let mut chegou = Vec::new();
        if let Some(rx) = &self.fila {
            while let Ok(e) = rx.try_recv() {
                chegou.push(e);
            }
        }
        for e in chegou {
            self.empurrar(e);
        }
    }

    pub fn bruto(&self) -> [f32; 6] {
        self.eixos
    }

    /// Botões que passaram de solto a apertado desde a última chamada.
    pub fn botoes_novos(&mut self) -> Vec<u8> {
        let novos = self.botoes & !self.anteriores;
        self.anteriores = self.botoes;
        (0..32u8).filter(|n| novos & (1 << n) != 0).collect()
    }
}

/// Seção das configurações: liga, sensibilidade, inversão por eixo, eixo dominante e botões.
pub fn painel(ui: &mut eframe::egui::Ui, c: &mut Mouse3dConfig) {
    use crate::i18n::tr;
    ui.label(eframe::egui::RichText::new(tr("Mouse 3D")).strong());
    ui.checkbox(&mut c.ativo, tr("usar mouse 3D quando houver um")).on_hover_text(tr("Sem aparelho conectado nada muda. Funciona pelo HID do sistema, sem o driver do fabricante."));
    if !c.ativo {
        return;
    }
    ui.add(crate::ui_ext::slider(&mut c.sens_translacao, 0.1..=4.0).text(tr("sensibilidade do movimento"))).on_hover_text(tr("Quanto a vista anda para a mesma força no aparelho."));
    ui.add(crate::ui_ext::slider(&mut c.sens_rotacao, 0.1..=4.0).text(tr("sensibilidade da rotação"))).on_hover_text(tr("Quanto a vista gira para a mesma inclinação do aparelho."));
    ui.add(crate::ui_ext::slider(&mut c.zona_morta, 0.0..=0.5).text(tr("zona morta"))).on_hover_text(tr("Parte do curso ignorada em torno do repouso, para a vista não escorregar sozinha."));
    ui.checkbox(&mut c.eixo_dominante, tr("só o eixo dominante")).on_hover_text(tr("Só o eixo mais forte age de cada vez, para aproximar sem girar."));
    ui.horizontal_wrapped(|ui| {
        for (i, nome) in NOMES_EIXOS.iter().enumerate() {
            ui.checkbox(&mut c.inverter[i], format!("{} {}", tr("inverter"), tr(nome)));
        }
    });
    ui.checkbox(&mut c.mover_peca, tr("na montagem, mover a peça escolhida")).on_hover_text(tr("Com uma peça escolhida na montagem, o aparelho move e gira a peça; sem peça escolhida, move a vista."));
    let todos = crate::ajuda_cmd::comandos();
    for n in 0..c.botoes.len().max(2) {
        if c.botoes.len() <= n {
            c.botoes.resize(n + 1, Acao::Nenhuma);
        }
        if c.comandos.len() <= n {
            c.comandos.resize(n + 1, String::new());
        }
        let atual = c.botoes[n];
        let cmd = todos.iter().find(|i| i.icon == c.comandos[n]);
        let rotulo = match cmd {
            Some(i) => format!("{}: {}", tr("comando"), tr(i.name)),
            None => tr(atual.nome()).to_string(),
        };
        eframe::egui::ComboBox::from_label(format!("{} {}", tr("botão"), n + 1)).selected_text(rotulo).height(320.).show_ui(ui, |ui| {
            for a in Acao::TODAS {
                if ui.selectable_label(cmd.is_none() && atual == a, tr(a.nome())).clicked() {
                    c.botoes[n] = a;
                    c.comandos[n].clear();
                }
            }
            ui.separator();
            ui.label(eframe::egui::RichText::new(tr("Comandos")).weak());
            for i in &todos {
                if ui.selectable_label(c.comandos[n] == i.icon, tr(i.name)).on_hover_text(tr(i.brief)).clicked() {
                    c.comandos[n] = i.icon.to_string();
                }
            }
        }).response.on_hover_text(tr("Ação de vista ou qualquer comando da busca (Ctrl+K)."));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rel(id: u8, v: [i16; 3]) -> Vec<u8> {
        let mut b = vec![id];
        for x in v {
            b.extend_from_slice(&x.to_le_bytes());
        }
        b
    }

    #[test]
    fn decodifica_translacao_rotacao_e_botoes() {
        match decodificar_relatorio(&rel(1, [350, -350, 175])) {
            Some(Evento::Translacao(t)) => assert!((t[0] - 1.).abs() < 1e-6 && (t[1] + 1.).abs() < 1e-6 && (t[2] - 0.5).abs() < 1e-6),
            o => panic!("{o:?}"),
        }
        assert!(matches!(decodificar_relatorio(&rel(2, [0, 0, 35])), Some(Evento::Rotacao(r)) if (r[2] - 0.1).abs() < 1e-6));
        assert_eq!(decodificar_relatorio(&[3, 0b101, 0, 0, 0]), Some(Evento::Botoes(0b101)));
        // desconhecido e curto não derrubam nada
        assert_eq!(decodificar_relatorio(&[9, 1, 2, 3]), None);
        assert_eq!(decodificar_relatorio(&[1, 0, 0]), None);
        assert_eq!(decodificar_relatorio(&[]), None);
    }

    #[test]
    fn zona_morta_nao_deixa_degrau_e_a_inversao_vale_por_eixo() {
        let c = Mouse3dConfig { zona_morta: 0.1, ..Default::default() };
        assert!(c.processar([0.09, 0., 0., 0., 0., 0.]).parado());
        let m = c.processar([0.1001, 0., 0., 0., 0., 0.]);
        assert!(m.eixos[0] > 0. && m.eixos[0] < 0.01, "sem degrau: {}", m.eixos[0]);
        assert!((c.processar([1., 0., 0., 0., 0., 0.]).eixos[0] - 1.).abs() < 1e-6);
        let mut inv = c.clone();
        inv.inverter[1] = true;
        let m = inv.processar([0.5, 0.5, 0., 0., 0., 0.]);
        assert!(m.eixos[0] > 0. && m.eixos[1] < 0., "só o eixo 1 inverte: {:?}", m.eixos);
    }

    #[test]
    fn sensibilidade_separa_translacao_de_rotacao_e_e_limitada() {
        let c = Mouse3dConfig { zona_morta: 0., sens_translacao: 0.5, sens_rotacao: 2., ..Default::default() };
        let m = c.processar([0.4, 0., 0., 0.4, 0., 0.]);
        assert!((m.eixos[0] - 0.2).abs() < 1e-6 && (m.eixos[3] - 0.8).abs() < 1e-6);
        // arquivo editado à mão com valores absurdos
        let c = Mouse3dConfig { sens_translacao: f32::NAN, sens_rotacao: 99., zona_morta: -3., ..Default::default() }.saneada();
        assert_eq!((c.sens_translacao, c.sens_rotacao, c.zona_morta), (1., 4., 0.));
    }

    #[test]
    fn eixo_dominante_deixa_so_o_maior() {
        let c = Mouse3dConfig { eixo_dominante: true, zona_morta: 0., ..Default::default() };
        let m = c.processar([0.2, 0., -0.7, 0.3, 0., 0.]);
        assert_eq!(m.eixos, [0., 0., -0.7, 0., 0., 0.]);
        assert!(c.processar([0.; 6]).parado());
    }

    #[test]
    fn leitura_simulada_move_a_camera_em_cada_eixo() {
        let base = Camera::default();
        let anda = |eixos: [f32; 6]| {
            let mut cam = base;
            aplicar_na_camera(&mut cam, &Movimento { eixos }, 0.05);
            cam
        };
        // translação para os lados e para frente deslocam o alvo, sem mexer na distância
        let c = anda([1., 0., 0., 0., 0., 0.]);
        assert!((c.target - base.target).length() > 1. && (c.dist - base.dist).abs() < 1e-4);
        let c = anda([0., 1., 0., 0., 0., 0.]);
        assert!((c.target - base.target).length() > 1.);
        // z muda a distância; os dois sentidos seguem sinais opostos
        assert!(anda([0., 0., 1., 0., 0., 0.]).dist > base.dist);
        assert!(anda([0., 0., -1., 0., 0., 0.]).dist < base.dist);
        // rotações mudam elevação e giro
        assert!(anda([0., 0., 0., 1., 0., 0.]).pitch > base.pitch);
        assert!(anda([0., 0., 0., 0., 1., 0.]).yaw < base.yaw);
        // parado, e quadro longo demais, não explodem
        assert!(anda([0.; 6]) == base);
        let mut cam = base;
        aplicar_na_camera(&mut cam, &Movimento { eixos: [1., 1., 1., 1., 1., 0.] }, 50.);
        assert!(cam.dist.is_finite() && cam.target.is_finite() && cam.pitch.abs() <= 1.5707);
    }

    #[test]
    fn pitch_nao_passa_dos_polos() {
        let mut cam = Camera::default();
        for _ in 0..200 {
            aplicar_na_camera(&mut cam, &Movimento { eixos: [0., 0., 0., 1., 0., 0.] }, 0.1);
        }
        assert!(cam.pitch <= 1.5706 + 1e-6);
    }

    #[test]
    fn botoes_chamam_acoes_so_na_transicao() {
        let mut m = Mouse3d::default();
        assert!(!m.presente && m.botoes_novos().is_empty());
        m.empurrar(Evento::Botoes(0b01));
        assert!(m.presente);
        assert_eq!(m.botoes_novos(), vec![0]);
        assert!(m.botoes_novos().is_empty(), "segurar o botão não repete");
        m.empurrar(Evento::Botoes(0b11));
        assert_eq!(m.botoes_novos(), vec![1]);
        m.empurrar(Evento::Botoes(0));
        assert!(m.botoes_novos().is_empty());
        m.empurrar(Evento::Botoes(0b10));
        assert_eq!(m.botoes_novos(), vec![1]);
        let c = Mouse3dConfig::default();
        assert_eq!(c.acao_do_botao(0), Acao::Enquadrar);
        assert_eq!(c.acao_do_botao(9), Acao::Nenhuma);
        let mut cam = Camera::default();
        assert!(Acao::AlternarOrtogonal.aplicar(&mut cam) && cam.ortho);
        assert!(Acao::VistaTopo.aplicar(&mut cam) && cam.pitch > 1.5);
        assert!(!Acao::Enquadrar.aplicar(&mut cam), "enquadrar precisa dos limites do modelo");
    }

    #[test]
    fn botao_ligado_a_comando_dispara_o_comando_e_icone_sumido_volta_a_vista() {
        let mut c = Mouse3dConfig::default();
        let item = crate::ajuda_cmd::comandos().into_iter().find(|i| i.name == "Filete").expect("Filete na busca");
        c.comandos = vec![String::new(), item.icon.into(), "icone_que_nao_existe".into()];
        assert_eq!(disparo_do_botao(&c, 0), Disparo::Vista(Acao::Enquadrar));
        assert_eq!(disparo_do_botao(&c, 1), Disparo::Comando(item.act));
        assert_eq!(disparo_do_botao(&c, 2), Disparo::Vista(Acao::Nenhuma));
        let volta: Mouse3dConfig = serde_json::from_str(&serde_json::to_string(&c).unwrap()).unwrap();
        assert_eq!(volta.comandos, c.comandos);
    }

    #[test]
    fn peca_anda_no_referencial_da_vista() {
        let mut cam = Camera::default();
        cam.set_view(-std::f32::consts::FRAC_PI_2, 0.);
        let (r, u) = cam.basis();
        let f = u.cross(r);
        // empurrar para a direita leva a peça para a direita da tela, sem girar
        let (t, w) = movimento_da_peca(&cam, &Movimento { eixos: [1., 0., 0., 0., 0., 0.] }, 0.05).unwrap();
        let tv = Vec3::new(t[0] as f32, t[1] as f32, t[2] as f32);
        assert!(tv.normalize().dot(r) > 0.999 && w == [0.; 3], "{t:?} {w:?}");
        assert!((tv.length() - cam.dist * 0.25 * 0.05).abs() < 1e-4);
        // torcer gira em torno do eixo de cima da tela
        let (t, w) = movimento_da_peca(&cam, &Movimento { eixos: [0., 0., 0., 0., 0., -1.] }, 0.05).unwrap();
        let wv = Vec3::new(w[0] as f32, w[1] as f32, w[2] as f32);
        assert!(t == [0.; 3] && wv.normalize().dot(-u) > 0.999 && (wv.length() - 0.06).abs() < 1e-5, "{w:?}");
        // a peça gira no lugar: o centro dela não sai, os cantos giram em volta
        let p = fenkra_montagem::Placement { origin: [10., 0., 0.], ..Default::default() };
        let q = p.moved([0.; 3], w, [10., 0., 0.]);
        assert!((q.origin[0] - 10.).abs() < 1e-9);
        let _ = f;
        assert!(movimento_da_peca(&cam, &Movimento::default(), 0.05).is_none());
    }

    #[test]
    fn eixos_chegam_pela_fila_e_sensibilidade_fica_salva() {
        let mut m = Mouse3d::default();
        m.empurrar(decodificar_relatorio(&rel(1, [350, 0, 0])).unwrap());
        m.empurrar(decodificar_relatorio(&rel(2, [0, -350, 0])).unwrap());
        assert_eq!(m.bruto(), [1., 0., 0., 0., -1., 0.]);
        // a configuração viaja pelo arquivo de configurações
        let mut s = crate::settings::Settings::default();
        s.mouse3d.sens_rotacao = 2.5;
        s.mouse3d.inverter[4] = true;
        s.mouse3d.botoes = vec![Acao::VistaTopo, Acao::Nenhuma, Acao::VistaIsometrica];
        let json = serde_json::to_string(&s).unwrap();
        let volta: crate::settings::Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(volta.mouse3d, s.mouse3d);
        // arquivo antigo, sem a seção, abre com os padrões
        let antigo: crate::settings::Settings = serde_json::from_str(r#"{"ui_scale": 1.1}"#).unwrap();
        assert_eq!(antigo.mouse3d, Mouse3dConfig::default());
    }
}
