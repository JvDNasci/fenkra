// SPDX-License-Identifier: LGPL-2.1-or-later
//! B510: o recurso nasce com o nome no idioma ativo da interface, e trocar de idioma depois não
//! renomeia o que já existe.

use fenkra_core::*;

fn prim(kind: PrimKind) -> FeatureKind {
    FeatureKind::Primitive(Primitive { kind, plane: PlaneRef::Origin(OriginPlane::XY), a: "10".into(), b: "20".into(), c: "30".into(), d: "0".into(), op: BoolOp::Add })
}

fn nomes(d: &Document) -> Vec<String> {
    let mut v: Vec<String> = d.features.iter().map(|f| f.name.clone()).collect();
    v.extend(d.references.iter().map(|r| r.name.clone()));
    v.extend(d.collectors.iter().map(|c| c.name.clone()));
    v.extend(d.folders.iter().map(|f| f.name.clone()));
    v.extend(d.medidas.iter().map(|m| m.nome.clone()));
    v
}

/// Cria um conjunto típico de itens com os nomes padrão.
fn criar_itens(d: &mut Document) {
    d.add_feature(prim(PrimKind::Caixa));
    d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data: Default::default() }));
    d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "5");
    d.add_geometry(RefKind::PointCoords { x: "1".into(), y: "2".into(), z: "3".into(), frame: None });
    d.add_medida(ModoMedida::Item, vec![], vec![], false);
}

#[test]
fn recurso_nasce_no_idioma_ativo_e_nao_renomeia_ao_trocar() {
    // em português
    crate::i18n::set_english(false);
    let mut pt = Document::new("Peça");
    criar_itens(&mut pt);
    let n_pt = nomes(&pt);
    for esperado in ["Caixa1", "Sketch1", "Plano1", "Ponto1", "Medida1", "Corpo principal", "Geometria"] {
        assert!(n_pt.iter().any(|n| n == esperado), "faltou {esperado} em {n_pt:?}");
    }
    assert_eq!(pt.idioma, nomes::Idioma::Pt);

    // trocar para inglês não renomeia o que já existe
    crate::i18n::set_english(true);
    assert_eq!(nomes(&pt), n_pt, "trocar de idioma renomeou itens");
    // o item novo do mesmo documento nasce em inglês
    pt.add_feature(prim(PrimKind::Cilindro));
    assert!(pt.features.iter().any(|f| f.name == "Cylinder1"), "{:?}", nomes(&pt));
    assert!(pt.features.iter().any(|f| f.name == "Caixa1"));

    // documento novo em inglês
    let mut en = Document::new("Part");
    criar_itens(&mut en);
    let n_en = nomes(&en);
    for esperado in ["Box1", "Sketch1", "Plane1", "Point1", "Measure1", "Main body", "Geometry"] {
        assert!(n_en.iter().any(|n| n == esperado), "faltou {esperado} em {n_en:?}");
    }
    assert_eq!(en.idioma, nomes::Idioma::En);
    // o idioma do documento é gravado e volta ao abrir
    let lido = Document::from_json(&en.to_json()).unwrap();
    assert_eq!(lido.idioma, nomes::Idioma::En);

    // voltar para português não renomeia o documento em inglês
    crate::i18n::set_english(false);
    assert_eq!(nomes(&en), n_en);
    en.add_feature(prim(PrimKind::Esfera));
    assert!(en.features.iter().any(|f| f.name == "Esfera1"), "{:?}", nomes(&en));
}

#[test]
fn todo_nome_base_do_nucleo_tem_versao_em_ingles() {
    // os nomes-base de `FeatureKind::type_name`, dos rótulos que ele usa e dos itens que o núcleo cria
    let bases = [
        "Sketch", "Recorte", "Extrusão", "Ranhura", "Revolução", "Furo", "Filete", "Chanfro", "Rasgo", "Nervura", "MultiSeçõesRemovida", "MultiSeções", "Casca", "Saída", "Espessura",
        "PadrãoLinear", "PadrãoCircular", "PadrãoPontos", "Espelho", "Escala", "Reforço", "Rosca", "Unir", "Subtrair", "Intersecao", "Dividir", "Engrossar", "Fechar", "Costurar", "Forma",
        "PreencherFronteiras", "Malha", "CorpoLink", "ParedeBase", "Aba", "Bainha", "Desvio em Z", "Dobra por linha", "DobraConica", "Veneziana", "Estampo", "FuroFlangeado", "RecorteChapa",
        "AbaPerfil", "AbaCurva", "AbaLaço", "AlívioCanto", "FechaCanto", "AbaEntrePerfis", "RedobrarParcial", "DesdobrarParcial", "ConverterEmChapa", "RasgoCircular", "AbaSuperfície", "Membro",
        "Placa", "Cordão de solda", "Translação", "Rotação", "Simetria", "Afinidade", "Plano", "PlanoFace", "Ponto", "Linha", "Curva", "Superficie", "Corpo", "Corpo principal", "Geometria",
        "Copia", "Medida", "Pasta",
    ];
    let mut faltam = vec![];
    for b in bases {
        if crate::arvore_nomes::nome_padrao_em_ingles(b).is_none() {
            faltam.push(b);
        }
    }
    for k in PrimKind::ALL {
        if crate::arvore_nomes::nome_padrao_em_ingles(k.label()).is_none() {
            faltam.push(k.label());
        }
    }
    for o in OpDireta::ALL {
        if crate::arvore_nomes::nome_padrao_em_ingles(o.rotulo()).is_none() {
            faltam.push(o.rotulo());
        }
    }
    assert!(faltam.is_empty(), "sem nome em inglês: {faltam:?}");
    assert_eq!(crate::arvore_nomes::nome_padrao_em_ingles("Recorte").as_deref(), Some("Pocket"));
    assert_eq!(crate::arvore_nomes::nome_padrao_em_ingles("PadrãoLinear").as_deref(), Some("LinearPattern"));
    assert_eq!(crate::arvore_nomes::nome_padrao_em_ingles("Dobra por linha").as_deref(), Some("Sketched bend"));
}
