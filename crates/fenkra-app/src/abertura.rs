// SPDX-License-Identifier: LGPL-2.1-or-later
//! Abrir uma peça: em qual bancada ela aparece e quando a câmera a enquadra (B423).

use crate::settings::Bench;
use fenkra_core::Document;

/// Bancada em que o conteúdo do documento abre (B403, B423). A bancada é onde estão os comandos da
/// peça: chapa dobrada abre em Chapa metálica, membros de perfil e tubo em Estrutural, malha de
/// subdivisão em Forma livre e peça feita só de superfícies em Superfícies. O resto abre em
/// Modelagem Sólida. Antes tudo isso abria na bancada que estivesse ativa (ou em Sólidos) e a pessoa
/// não via os comandos que criaram a peça.
pub fn bancada_do_conteudo(d: &Document) -> Bench {
    use fenkra_core::FeatureKind as K;
    if d.sheet.is_some() {
        return Bench::Chapa;
    }
    let ativas = || d.features.iter().map(|f| &f.kind);
    if ativas().any(|k| matches!(k, K::StructMember(_) | K::StructPlate(_) | K::StructGusset(_) | K::StructWeld(_))) {
        return Bench::Estrutural;
    }
    let superficie = |k: &K| matches!(k, K::ThickSurface(_) | K::CloseSurface(_) | K::SewSurface(_) | K::BoundaryFill(_));
    let solido = |k: &K| matches!(k, K::Extrude(_) | K::Revolve(_) | K::Primitive(_) | K::Sweep(_) | K::Loft(_) | K::Hole(_) | K::Shell(_) | K::Combine(_));
    // Forma livre só quando a malha de subdivisão é o último comando que faz corpo. Peça que nasce de
    // uma forma e segue com extrusões (o volante DI-01: aro em forma, raios extrudados) continua na
    // Modelagem Sólida, onde estão os comandos que a pessoa vai editar.
    let ultimo_corpo = d.features.iter().filter(|f| !f.suppressed).map(|f| &f.kind).filter(|k| matches!(k, K::Forma(_)) || solido(k)).last();
    if matches!(ultimo_corpo, Some(K::Forma(_))) {
        return Bench::FormaLivre;
    }
    let tem_superficie = ativas().any(superficie) || d.references.iter().any(|r| r.kind.class() == fenkra_core::RefClass::Surface);
    if tem_superficie && !ativas().any(solido) {
        return Bench::Superficies;
    }
    Bench::Solidos
}

/// Arquivo de documento passado na linha de comando (B389): o primeiro argumento que não é opção e
/// termina em `.fkp`, `.fka` ou `.fkd`. É o que o sistema entrega no duplo clique num arquivo
/// associado ao programa. O `.fkv` tem caminho próprio (`--abrir-fkv`).
pub fn arquivo_da_linha(args: &[String]) -> Option<std::path::PathBuf> {
    args.iter().filter(|a| !a.starts_with("--")).find_map(|a| {
        let p = std::path::Path::new(a);
        let ext = p.extension()?.to_string_lossy().to_lowercase();
        matches!(ext.as_str(), "fkp" | "fka" | "fkd").then(|| p.to_path_buf())
    })
}

/// Estado do enquadramento ao abrir. A câmera só pode enquadrar depois que a cena NOVA subiu à
/// placa de vídeo: enquadrar na hora usava a caixa da peça que acabou de sair, e a peça nova
/// aparecia grande, pequena ou cortada.
#[derive(Default, Debug)]
pub struct Enquadrar {
    /// aguardando o resultado do recálculo da peça recém-aberta
    pub ao_resultar: bool,
    /// o resultado chegou: enquadrar assim que a cena subir
    pub apos_cena: bool,
}

impl Enquadrar {
    /// Peça aberta: enquadra quando o primeiro cálculo dela chegar.
    pub fn peca_aberta(&mut self) {
        self.ao_resultar = true;
    }

    /// Chegou um resultado. `atual` diz se é do último pedido enviado (o da peça aberta).
    pub fn resultado(&mut self, atual: bool) {
        if atual && self.ao_resultar {
            self.ao_resultar = false;
            self.apos_cena = true;
        }
    }

    /// Pede o enquadramento para depois da próxima subida de cena.
    pub fn depois_da_cena(&mut self) {
        self.apos_cena = true;
    }

    /// A cena subiu: devolve true uma vez se é hora de enquadrar.
    pub fn cena_subiu(&mut self) -> bool {
        std::mem::take(&mut self.apos_cena)
    }
}

/// Alvo e distância com que uma vista nomeada abre. A vista guarda a câmera do dia em que foi
/// salva; se a peça cresceu, encolheu ou mudou de lugar desde então, a vista abria cortando a peça
/// ou mostrando um ponto. Nesses casos mantém os ângulos da vista e enquadra a peça; vista que já
/// mostra a peça inteira, em tamanho razoável, abre exatamente como foi salva.
pub fn vista_enquadrada(alvo: glam::Vec3, dist: f32, caixa: Option<(glam::Vec3, glam::Vec3)>) -> (glam::Vec3, f32) {
    let Some((lo, hi)) = caixa else { return (alvo, dist) };
    let centro = (lo + hi) / 2.;
    let diag = (hi - lo).length().max(1e-3);
    let ideal = (diag * 1.6).max(20.);
    let corta = dist < ideal * 0.6;
    let minuscula = dist > ideal * 3.;
    let longe = (alvo - centro).length() > diag * 0.5;
    if corta || minuscula || longe {
        (centro, ideal)
    } else {
        (alvo, dist)
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn chapa_abre_na_bancada_de_chapa_e_o_resto_em_solidos() {
        let d = Document::new("Peça");
        assert_eq!(bancada_do_conteudo(&d), Bench::Solidos);
        let mut c = Document::new("Chapa");
        c.sheet = Some(Default::default());
        assert_eq!(bancada_do_conteudo(&c), Bench::Chapa);
    }

    #[test]
    fn abre_na_bancada_do_conteudo_forma_superficie_e_tubos() {
        let (forma, _) = crate::curso_exemplos::peca("Forma suave").unwrap();
        assert_eq!(bancada_do_conteudo(&forma), Bench::FormaLivre);
        let (sup, _) = crate::curso_exemplos::peca("Superfícies de exemplo").unwrap();
        assert_eq!(bancada_do_conteudo(&sup), Bench::Superficies);
        let quadro = crate::examples::all().into_iter().find(|e| e.name == "Quadro de mesa").unwrap();
        assert_eq!(bancada_do_conteudo(&quadro.doc), Bench::Estrutural);
        // superfícies que viraram volume: a peça é um sólido
        let mut mista = sup.clone();
        let solido = crate::examples::all().into_iter().flat_map(|e| e.doc.features).find(|f| matches!(f.kind, fenkra_core::FeatureKind::Extrude(_))).unwrap();
        mista.add_feature(solido.kind.clone());
        assert_eq!(bancada_do_conteudo(&mista), Bench::Solidos);
        // forma seguida de extrusão (volante do kart): abre em Sólidos; antes abria em Forma livre
        let mut volante = forma.clone();
        volante.add_feature(solido.kind.clone());
        assert_eq!(bancada_do_conteudo(&volante), Bench::Solidos);
        // a extrusão suprimida não conta: o último corpo volta a ser a forma
        if let Some(f) = volante.features.last_mut() {
            f.suppressed = true;
        }
        assert_eq!(bancada_do_conteudo(&volante), Bench::FormaLivre);
    }

    #[test]
    fn linha_de_comando_acha_o_documento_e_ignora_opcoes() {
        let a = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(arquivo_da_linha(&a(&["--seguro", "a/Peça.FKP"])), Some("a/Peça.FKP".into()));
        assert_eq!(arquivo_da_linha(&a(&["m.fka"])), Some("m.fka".into()));
        assert_eq!(arquivo_da_linha(&a(&["d.fkd", "outro.fkp"])), Some("d.fkd".into()));
        assert_eq!(arquivo_da_linha(&a(&["--abrir-fkv", "x.fkv"])), None);
        assert_eq!(arquivo_da_linha(&a(&[])), None);
        assert_eq!(arquivo_da_linha(&a(&["leia.txt"])), None);
    }

    #[test]
    fn vista_nomeada_enquadra_quando_corta_ou_some_e_respeita_quando_serve() {
        use glam::Vec3;
        let caixa = Some((Vec3::splat(-50.), Vec3::splat(50.)));
        let ideal = (Vec3::splat(100.).length() * 1.6).max(20.);
        // já mostra a peça inteira: abre como foi salva
        assert_eq!(vista_enquadrada(Vec3::new(5., 0., 0.), ideal, caixa), (Vec3::new(5., 0., 0.), ideal));
        // salva de muito perto: corta a peça
        let (a, d) = vista_enquadrada(Vec3::ZERO, ideal * 0.2, caixa);
        assert_eq!((a, d), (Vec3::ZERO, ideal));
        // peça mudou de lugar: o alvo salvo ficou longe dela
        let longe = Some((Vec3::splat(950.), Vec3::splat(1050.)));
        let (a, _) = vista_enquadrada(Vec3::ZERO, ideal, longe);
        assert_eq!(a, Vec3::splat(1000.));
        // sem caixa (cena vazia): não mexe
        assert_eq!(vista_enquadrada(Vec3::X, 7., None), (Vec3::X, 7.));
    }

    #[test]
    fn enquadra_so_depois_do_resultado_da_peca_nova_e_da_cena() {
        let mut e = Enquadrar::default();
        e.peca_aberta();
        // resultado velho (de outro pedido) não dispara
        e.resultado(false);
        assert!(!e.cena_subiu());
        e.resultado(true);
        assert!(e.cena_subiu(), "com o resultado novo, a cena seguinte enquadra");
        assert!(!e.cena_subiu(), "enquadra uma vez só");
    }
}
