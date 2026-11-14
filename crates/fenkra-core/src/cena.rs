// SPDX-License-Identifier: LGPL-2.1-or-later
//! Cena de apresentação (B182): câmera ativa, luzes, ambiente e aparências por corpo ou por face.
//!
//! A cena mora no documento e é a mesma para a vista, as imagens de passo e o render: quem desenha
//! lê daqui a câmera, a luz principal (a que projeta a sombra do B139) e o ambiente. Nada aqui
//! depende da placa de vídeo; a viewport traduz para o uniforme dela.
//!
//! As câmeras são as vistas salvas do documento (`Document::views`): a cena só guarda o nome da
//! ativa, então guardar uma câmera na cena ou no menu de vistas dá no mesmo lugar.
//!
//! As aparências por face usam a referência estável de face (`FaceKey`, ADR 0010): dono, papel e
//! assinatura geométrica. Depois de editar o comando que criou a face, a cor vai para a face atual
//! mais parecida com a guardada ([`achar_face`]).

use crate::doc::{Appearance, FaceKey, Id, NamedView};
use serde::{Deserialize, Serialize};

/// De onde vem a luz.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum TipoLuz {
    /// presa à câmera: acima e à esquerda de quem olha, gira junto com a vista
    #[default]
    DaCamera,
    /// fixa no mundo, numa direção: a sombra fica parada ao girar a vista
    Direcional,
}

/// Uma luz da cena. A primeira luz ligada é a principal e é a única que projeta sombra.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Luz {
    pub nome: String,
    pub tipo: TipoLuz,
    /// direção que vai do modelo para a luz (luz direcional), Z para cima
    pub direcao: [f32; 3],
    pub cor: [u8; 3],
    /// 1 = intensidade de fábrica da luz principal
    pub intensidade: f32,
    pub ligada: bool,
}

impl Default for Luz {
    fn default() -> Self {
        Luz { nome: "Luz principal".into(), tipo: TipoLuz::DaCamera, direcao: [0.35, -0.45, 0.82], cor: [255, 255, 255], intensidade: 1., ligada: true }
    }
}

/// Fundo da cena.
#[derive(Clone, PartialEq, Debug, Default, Serialize, Deserialize)]
pub enum Fundo {
    /// o fundo do tema (a peça continua distinta dele)
    #[default]
    Tema,
    Cor([u8; 3]),
    /// de baixo para cima
    Degrade([u8; 3], [u8; 3]),
    /// imagem panorâmica equirretangular (PNG ou JPG), girada em torno de Z
    Panoramica { caminho: String, rotacao_graus: f32, exposicao: f32 },
}

/// Ambiente: fundo, luz de ambiente e chão. Cada efeito que custa desenho tem interruptor próprio.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Ambiente {
    pub fundo: Fundo,
    /// força da luz de ambiente (1 = de fábrica)
    pub ambiente: f32,
    /// chão sob a peça, na altura mais baixa da caixa envolvente
    pub chao: bool,
    /// o chão reflete a peça (desenha a peça mais uma vez, espelhada)
    pub reflexo_ligado: bool,
    /// 0 sem reflexo, 1 espelho
    pub reflexo: f32,
    /// o chão recebe a sombra da luz principal (usa o mapa de sombra do B139)
    pub sombra_no_chao: bool,
}

impl Default for Ambiente {
    fn default() -> Self {
        Ambiente { fundo: Fundo::Tema, ambiente: 1., chao: false, reflexo_ligado: false, reflexo: 0.3, sombra_no_chao: true }
    }
}

/// O que recebe uma aparência da cena.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum AlvoAparencia {
    /// corpo (coletor) da peça
    Corpo(Id),
    /// face pela referência estável
    Face(FaceKey),
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct AparenciaCena {
    pub alvo: AlvoAparencia,
    pub aparencia: Appearance,
}

/// Nó Cena do documento.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Cena {
    /// a montagem e as outras peças podem usar esta cena (imagens de passo, render)
    pub publicada: bool,
    /// nome da vista salva que as imagens de passo e o render usam
    pub camera_ativa: Option<String>,
    pub luzes: Vec<Luz>,
    pub ambiente: Ambiente,
    pub aparencias: Vec<AparenciaCena>,
}

impl Default for Cena {
    /// Cena de fábrica: uma luz presa à câmera, fundo do tema, sem chão. É exatamente a vista de
    /// antes do B182, então ligar a cena não muda nada até a pessoa mexer.
    fn default() -> Self {
        Cena { publicada: false, camera_ativa: None, luzes: vec![Luz::default()], ambiente: Ambiente::default(), aparencias: vec![] }
    }
}

impl Cena {
    /// Luz principal: a primeira ligada.
    pub fn luz_principal(&self) -> Option<&Luz> {
        self.luzes.iter().find(|l| l.ligada)
    }

    /// Câmera ativa entre as vistas salvas do documento.
    pub fn camera<'a>(&self, vistas: &'a [NamedView]) -> Option<&'a NamedView> {
        let n = self.camera_ativa.as_deref()?;
        vistas.iter().find(|v| v.name == n)
    }

    /// Aparência da cena para o corpo, se houver.
    pub fn aparencia_do_corpo(&self, corpo: Id) -> Option<Appearance> {
        self.aparencias.iter().rev().find(|a| a.alvo == AlvoAparencia::Corpo(corpo)).map(|a| a.aparencia)
    }

    /// Aparências por face.
    pub fn aparencias_de_face(&self) -> impl Iterator<Item = (&FaceKey, Appearance)> {
        self.aparencias.iter().filter_map(|a| match &a.alvo {
            AlvoAparencia::Face(k) => Some((k, a.aparencia)),
            _ => None,
        })
    }

    /// Põe (ou troca) a aparência de um alvo.
    pub fn definir_aparencia(&mut self, alvo: AlvoAparencia, aparencia: Appearance) {
        match self.aparencias.iter_mut().find(|a| a.alvo == alvo) {
            Some(a) => a.aparencia = aparencia,
            None => self.aparencias.push(AparenciaCena { alvo, aparencia }),
        }
    }

    pub fn tirar_aparencia(&mut self, alvo: AlvoAparencia) {
        self.aparencias.retain(|a| a.alvo != alvo);
    }

    /// Aparência de cada face de um corpo pelas referências estáveis: `chaves` traz a chave atual
    /// de cada face (na ordem das faces). Cada aparência guardada vai para a face mais parecida; a
    /// mais recente vence quando duas caem na mesma face.
    pub fn aparencias_das_faces(&self, chaves: &[Option<FaceKey>]) -> Vec<Option<Appearance>> {
        let mut out = vec![None; chaves.len()];
        for (k, a) in self.aparencias_de_face() {
            if let Some(i) = achar_face(k, chaves) {
                out[i] = Some(a);
            }
        }
        out
    }
}

/// Nome livre para uma câmera nova: "Câmera 1", "Câmera 2"...
pub fn nome_de_camera(vistas: &[NamedView], base: &str) -> String {
    (1..).map(|n| format!("{base} {n}")).find(|n| !vistas.iter().any(|v| &v.name == n)).unwrap()
}

/// Quanto a face `atual` (do recálculo de agora) parece a face guardada: `None` se dono, papel ou
/// tipo de superfície diferem; senão um número maior para a mais parecida em direção e área.
pub fn semelhanca(guardada: &FaceKey, atual: &FaceKey) -> Option<f64> {
    if guardada.owner != atual.owner || guardada.role != atual.role || guardada.kind != atual.kind {
        return None;
    }
    let (a, b) = (guardada.normal_or_axis, atual.normal_or_axis);
    let d = a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    // eixo de cilindro não tem sentido: vale o módulo
    let d = if guardada.kind == 0 { d } else { d.abs() };
    let area = (guardada.area - atual.area).abs() / guardada.area.abs().max(atual.area.abs()).max(1e-9);
    Some(d - area)
}

/// Índice da face mais parecida com a guardada numa lista de chaves atuais (uma por face).
pub fn achar_face(guardada: &FaceKey, atuais: &[Option<FaceKey>]) -> Option<usize> {
    atuais.iter().enumerate().filter_map(|(i, k)| k.as_ref().and_then(|k| semelhanca(guardada, k)).map(|s| (i, s))).max_by(|a, b| a.1.total_cmp(&b.1)).map(|(i, _)| i)
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::doc::{Document, Finish};

    fn chave(owner: u64, role: i32, n: [f64; 3], area: f64) -> FaceKey {
        FaceKey { owner: Id(owner), role, kind: 0, normal_or_axis: n, area }
    }

    fn app(c: u8) -> Appearance {
        Appearance { color: [c, c, c], finish: Finish::Fosco, texture: None }
    }

    #[test]
    fn a_face_guardada_e_achada_depois_de_mudar_de_lugar() {
        let guardada = chave(5, 1, [0., 0., 1.], 100.);
        // a peça mudou: a face de cima agora é a quarta, com área um pouco maior
        let atuais = vec![Some(chave(5, 0, [0., 0., -1.], 120.)), None, Some(chave(9, 1, [0., 0., 1.], 120.)), Some(chave(5, 1, [0., 0., 1.], 120.))];
        assert_eq!(achar_face(&guardada, &atuais), Some(3));
        assert_eq!(achar_face(&chave(5, 4, [1., 0., 0.], 1.), &atuais), None);
        let mut c = Cena::default();
        c.definir_aparencia(AlvoAparencia::Face(guardada), app(10));
        assert_eq!(c.aparencias_das_faces(&atuais), vec![None, None, None, Some(app(10))]);
    }

    #[test]
    fn aparencia_troca_no_mesmo_alvo_e_some_ao_tirar() {
        let mut c = Cena::default();
        let k = chave(7, 1, [0., 0., 1.], 100.);
        c.definir_aparencia(AlvoAparencia::Face(k), app(10));
        c.definir_aparencia(AlvoAparencia::Face(k), app(20));
        c.definir_aparencia(AlvoAparencia::Corpo(Id(3)), app(30));
        assert_eq!(c.aparencias.len(), 2);
        assert_eq!(c.aparencias_de_face().next().map(|(_, a)| a.color[0]), Some(20));
        assert_eq!(c.aparencia_do_corpo(Id(3)).map(|a| a.color[0]), Some(30));
        c.tirar_aparencia(AlvoAparencia::Corpo(Id(3)));
        assert_eq!(c.aparencia_do_corpo(Id(3)), None);
    }

    #[test]
    fn camera_ativa_pelo_nome_da_vista_salva() {
        let v = |n: &str, yaw: f64| NamedView { name: n.into(), target: [1., 2., 3.], yaw, pitch: 0.4, dist: 250., ortho: false };
        let mut vistas = vec![v("Câmera 1", 0.1)];
        assert_eq!(nome_de_camera(&vistas, "Câmera"), "Câmera 2");
        vistas.push(v("Câmera 2", 0.7));
        let c = Cena { camera_ativa: Some("Câmera 2".into()), ..Default::default() };
        assert_eq!(c.camera(&vistas).map(|x| x.yaw), Some(0.7));
        // a vista apagada solta a câmera ativa sem quebrar nada
        vistas.pop();
        assert!(c.camera(&vistas).is_none());
        assert_eq!(Cena::default().luz_principal().map(|l| l.tipo), Some(TipoLuz::DaCamera));
    }

    /// A cena gravada no arquivo da peça e relida volta igual, e arquivo antigo abre sem cena.
    #[test]
    fn cena_gravada_e_relida_volta_igual() {
        let mut d = Document::new("Peça");
        let corpo = d.collectors[0].id;
        let mut c = Cena { publicada: true, camera_ativa: Some("Câmera 1".into()), ..Default::default() };
        c.luzes[0].tipo = TipoLuz::Direcional;
        c.luzes[0].direcao = [-0.6, -0.3, 0.75];
        c.luzes[0].cor = [255, 214, 170];
        c.luzes[0].intensidade = 1.4;
        c.luzes.push(Luz { nome: "Contraluz".into(), ligada: false, ..Default::default() });
        c.ambiente = Ambiente { fundo: Fundo::Panoramica { caminho: "estudio.png".into(), rotacao_graus: 35., exposicao: 1.2 }, ambiente: 0.8, chao: true, reflexo_ligado: true, reflexo: 0.45, sombra_no_chao: false };
        c.definir_aparencia(AlvoAparencia::Corpo(corpo), app(40));
        c.definir_aparencia(AlvoAparencia::Face(chave(9, 1, [0., 0., 1.], 50.)), app(90));
        d.views.push(NamedView { name: "Câmera 1".into(), target: [0.; 3], yaw: -0.785, pitch: 0.6, dist: 300., ortho: false });
        d.cena = Some(c.clone());
        let lido = Document::from_json(&d.to_json()).unwrap();
        assert_eq!(lido.cena.as_ref(), Some(&c));
        assert_eq!(lido.cena.as_ref().and_then(|c| c.camera(&lido.views)).map(|v| v.dist), Some(300.));
        // gravar de novo dá o mesmo texto: a cena não perde nem inventa campo
        assert_eq!(Document::from_json(&lido.to_json()).unwrap().cena, lido.cena);
        // peça sem cena não grava o campo, e arquivo antigo abre sem ela
        let mut sem = Document::new("Sem");
        sem.cena = None;
        assert!(!sem.to_json().contains("\"cena\""));
        assert!(Document::from_json(&sem.to_json()).unwrap().cena.is_none());
        // campos que faltam (arquivo de uma versão anterior da cena) ganham o padrão
        let velho: Cena = serde_json::from_str(r#"{"ambiente":{"chao":true}}"#).unwrap();
        assert!(velho.ambiente.chao && velho.ambiente.ambiente == 1. && velho.luzes.len() == 1);
    }
}
