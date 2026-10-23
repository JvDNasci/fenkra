//! Ícones SVG (assets/icons, gerados por scripts/gerar_icones.py e editáveis no VectorCraft/Inkscape).

use eframe::egui::{self, ImageSource};
use crate::i18n::tr;

// Lista gerada a partir de assets/icons (scripts/gerar_icones.py).
pub fn get(name: &str) -> ImageSource<'static> {
    match name {
        "abrir_projeto" => egui::include_image!("../../../assets/icons/abrir_projeto.svg"),
        "ajustar" => egui::include_image!("../../../assets/icons/ajustar.svg"),
        "an_curvatura" => egui::include_image!("../../../assets/icons/an_curvatura.svg"),
        "an_sombreado" => egui::include_image!("../../../assets/icons/an_sombreado.svg"),
        "an_zebra" => egui::include_image!("../../../assets/icons/an_zebra.svg"),
        "aparencia" => egui::include_image!("../../../assets/icons/aparencia.svg"),
        "arestas_comando" => egui::include_image!("../../../assets/icons/arestas_comando.svg"),
        "arestas_selecionar" => egui::include_image!("../../../assets/icons/arestas_selecionar.svg"),
        "arestas_todas" => egui::include_image!("../../../assets/icons/arestas_todas.svg"),
        "asm_grupo" => egui::include_image!("../../../assets/icons/asm_grupo.svg"),
        "asm_inserir" => egui::include_image!("../../../assets/icons/asm_inserir.svg"),
        "asm_nova" => egui::include_image!("../../../assets/icons/asm_nova.svg"),
        "asm_numerar" => egui::include_image!("../../../assets/icons/asm_numerar.svg"),
        "asm_peca_nova" => egui::include_image!("../../../assets/icons/asm_peca_nova.svg"),
        "asm_repeticao" => egui::include_image!("../../../assets/icons/asm_repeticao.svg"),
        "asm_subconjunto" => egui::include_image!("../../../assets/icons/asm_subconjunto.svg"),
        "asm_substituir" => egui::include_image!("../../../assets/icons/asm_substituir.svg"),
        "bool_intersecao" => egui::include_image!("../../../assets/icons/bool_intersecao.svg"),
        "bool_subtrair" => egui::include_image!("../../../assets/icons/bool_subtrair.svg"),
        "bool_unir" => egui::include_image!("../../../assets/icons/bool_unir.svg"),
        "canto" => egui::include_image!("../../../assets/icons/canto.svg"),
        "casca" => egui::include_image!("../../../assets/icons/casca.svg"),
        "chanfro" => egui::include_image!("../../../assets/icons/chanfro.svg"),
        "chanfro_dist" => egui::include_image!("../../../assets/icons/chanfro_dist.svg"),
        "chanfro_dist_angulo" => egui::include_image!("../../../assets/icons/chanfro_dist_angulo.svg"),
        "chanfro_dist_dist" => egui::include_image!("../../../assets/icons/chanfro_dist_dist.svg"),
        "circulo3d" => egui::include_image!("../../../assets/icons/circulo3d.svg"),
        "circulo3p" => egui::include_image!("../../../assets/icons/circulo3p.svg"),
        "coletor" => egui::include_image!("../../../assets/icons/coletor.svg"),
        "composer" => egui::include_image!("../../../assets/icons/composer.svg"),
        "conexao" => egui::include_image!("../../../assets/icons/conexao.svg"),
        "config" => egui::include_image!("../../../assets/icons/config.svg"),
        "conica" => egui::include_image!("../../../assets/icons/conica.svg"),
        "cont_g0" => egui::include_image!("../../../assets/icons/cont_g0.svg"),
        "cont_g1" => egui::include_image!("../../../assets/icons/cont_g1.svg"),
        "cont_g2" => egui::include_image!("../../../assets/icons/cont_g2.svg"),
        "cota_angulo" => egui::include_image!("../../../assets/icons/cota_angulo.svg"),
        "cota_diametro" => egui::include_image!("../../../assets/icons/cota_diametro.svg"),
        "cota_horizontal" => egui::include_image!("../../../assets/icons/cota_horizontal.svg"),
        "cota_raio" => egui::include_image!("../../../assets/icons/cota_raio.svg"),
        "cota_vertical" => egui::include_image!("../../../assets/icons/cota_vertical.svg"),
        "curva_extraida" => egui::include_image!("../../../assets/icons/curva_extraida.svg"),
        "curva_intersecao" => egui::include_image!("../../../assets/icons/curva_intersecao.svg"),
        "curva_paralela" => egui::include_image!("../../../assets/icons/curva_paralela.svg"),
        "curva_proj_sup" => egui::include_image!("../../../assets/icons/curva_proj_sup.svg"),
        "curva_projetada" => egui::include_image!("../../../assets/icons/curva_projetada.svg"),
        "escala" => egui::include_image!("../../../assets/icons/escala.svg"),
        "espelho" => egui::include_image!("../../../assets/icons/espelho.svg"),
        "espessura" => egui::include_image!("../../../assets/icons/espessura.svg"),
        "espiral" => egui::include_image!("../../../assets/icons/espiral.svg"),
        "estilo" => egui::include_image!("../../../assets/icons/estilo.svg"),
        "exemplo" => egui::include_image!("../../../assets/icons/exemplo.svg"),
        "ext_ate_face" => egui::include_image!("../../../assets/icons/ext_ate_face.svg"),
        "ext_ate_plano" => egui::include_image!("../../../assets/icons/ext_ate_plano.svg"),
        "ext_ate_superficie" => egui::include_image!("../../../assets/icons/ext_ate_superficie.svg"),
        "ext_ate_ultimo" => egui::include_image!("../../../assets/icons/ext_ate_ultimo.svg"),
        "ext_cego" => egui::include_image!("../../../assets/icons/ext_cego.svg"),
        "ext_dois_sentidos" => egui::include_image!("../../../assets/icons/ext_dois_sentidos.svg"),
        "ext_passante" => egui::include_image!("../../../assets/icons/ext_passante.svg"),
        "ext_saida_filete" => egui::include_image!("../../../assets/icons/ext_saida_filete.svg"),
        "ext_simetrico" => egui::include_image!("../../../assets/icons/ext_simetrico.svg"),
        "extrair" => egui::include_image!("../../../assets/icons/extrair.svg"),
        "extrusao" => egui::include_image!("../../../assets/icons/extrusao.svg"),
        "filete" => egui::include_image!("../../../assets/icons/filete.svg"),
        "filete_completo" => egui::include_image!("../../../assets/icons/filete_completo.svg"),
        "filete_face_face" => egui::include_image!("../../../assets/icons/filete_face_face.svg"),
        "filete_variavel" => egui::include_image!("../../../assets/icons/filete_variavel.svg"),
        "filtro" => egui::include_image!("../../../assets/icons/filtro.svg"),
        "furo" => egui::include_image!("../../../assets/icons/furo.svg"),
        "furo_cego" => egui::include_image!("../../../assets/icons/furo_cego.svg"),
        "furo_contrafurado" => egui::include_image!("../../../assets/icons/furo_contrafurado.svg"),
        "furo_escareado" => egui::include_image!("../../../assets/icons/furo_escareado.svg"),
        "furo_passante" => egui::include_image!("../../../assets/icons/furo_passante.svg"),
        "furo_rebaixado" => egui::include_image!("../../../assets/icons/furo_rebaixado.svg"),
        "furo_roscado" => egui::include_image!("../../../assets/icons/furo_roscado.svg"),
        "furo_simples" => egui::include_image!("../../../assets/icons/furo_simples.svg"),
        "helice" => egui::include_image!("../../../assets/icons/helice.svg"),
        "hib_costurar" => egui::include_image!("../../../assets/icons/hib_costurar.svg"),
        "hib_dividir" => egui::include_image!("../../../assets/icons/hib_dividir.svg"),
        "hib_engrossar" => egui::include_image!("../../../assets/icons/hib_engrossar.svg"),
        "hib_fechar" => egui::include_image!("../../../assets/icons/hib_fechar.svg"),
        "hib_malha" => egui::include_image!("../../../assets/icons/hib_malha.svg"),
        "hib_preencher" => egui::include_image!("../../../assets/icons/hib_preencher.svg"),
        "idioma" => egui::include_image!("../../../assets/icons/idioma.svg"),
        "junta_cilindrica" => egui::include_image!("../../../assets/icons/junta_cilindrica.svg"),
        "junta_esferica" => egui::include_image!("../../../assets/icons/junta_esferica.svg"),
        "junta_parafuso" => egui::include_image!("../../../assets/icons/junta_parafuso.svg"),
        "junta_planar" => egui::include_image!("../../../assets/icons/junta_planar.svg"),
        "junta_ponto_curva" => egui::include_image!("../../../assets/icons/junta_ponto_curva.svg"),
        "junta_prismatica" => egui::include_image!("../../../assets/icons/junta_prismatica.svg"),
        "junta_revolucao" => egui::include_image!("../../../assets/icons/junta_revolucao.svg"),
        "junta_rigida" => egui::include_image!("../../../assets/icons/junta_rigida.svg"),
        "linha_angulo" => egui::include_image!("../../../assets/icons/linha_angulo.svg"),
        "linha_bissetriz" => egui::include_image!("../../../assets/icons/linha_bissetriz.svg"),
        "linha_direcao" => egui::include_image!("../../../assets/icons/linha_direcao.svg"),
        "linha_eixo" => egui::include_image!("../../../assets/icons/linha_eixo.svg"),
        "linha_normal" => egui::include_image!("../../../assets/icons/linha_normal.svg"),
        "linha_planos" => egui::include_image!("../../../assets/icons/linha_planos.svg"),
        "linha_pontos" => egui::include_image!("../../../assets/icons/linha_pontos.svg"),
        "linha_tangente" => egui::include_image!("../../../assets/icons/linha_tangente.svg"),
        "macro" => egui::include_image!("../../../assets/icons/macro.svg"),
        "mec_auto" => egui::include_image!("../../../assets/icons/mec_auto.svg"),
        "mec_simular" => egui::include_image!("../../../assets/icons/mec_simular.svg"),
        "mostrar" => egui::include_image!("../../../assets/icons/mostrar.svg"),
        "multisecoes" => egui::include_image!("../../../assets/icons/multisecoes.svg"),
        "multisecoes_remover" => egui::include_image!("../../../assets/icons/multisecoes_remover.svg"),
        "navegacao" => egui::include_image!("../../../assets/icons/navegacao.svg"),
        "nervura" => egui::include_image!("../../../assets/icons/nervura.svg"),
        "nova_peca" => egui::include_image!("../../../assets/icons/nova_peca.svg"),
        "novo_projeto" => egui::include_image!("../../../assets/icons/novo_projeto.svg"),
        "ocultar" => egui::include_image!("../../../assets/icons/ocultar.svg"),
        "ocultos" => egui::include_image!("../../../assets/icons/ocultos.svg"),
        "op_adicionar" => egui::include_image!("../../../assets/icons/op_adicionar.svg"),
        "op_aparar" => egui::include_image!("../../../assets/icons/op_aparar.svg"),
        "op_contorno" => egui::include_image!("../../../assets/icons/op_contorno.svg"),
        "op_curar" => egui::include_image!("../../../assets/icons/op_curar.svg"),
        "op_desaparar" => egui::include_image!("../../../assets/icons/op_desaparar.svg"),
        "op_desmontar" => egui::include_image!("../../../assets/icons/op_desmontar.svg"),
        "op_dividir" => egui::include_image!("../../../assets/icons/op_dividir.svg"),
        "op_extrapolar" => egui::include_image!("../../../assets/icons/op_extrapolar.svg"),
        "op_intersecao" => egui::include_image!("../../../assets/icons/op_intersecao.svg"),
        "op_juntar" => egui::include_image!("../../../assets/icons/op_juntar.svg"),
        "op_multi_extrair" => egui::include_image!("../../../assets/icons/op_multi_extrair.svg"),
        "op_remover" => egui::include_image!("../../../assets/icons/op_remover.svg"),
        "op_unir" => egui::include_image!("../../../assets/icons/op_unir.svg"),
        "ortho" => egui::include_image!("../../../assets/icons/ortho.svg"),
        "padrao_circular" => egui::include_image!("../../../assets/icons/padrao_circular.svg"),
        "padrao_linear" => egui::include_image!("../../../assets/icons/padrao_linear.svg"),
        "padrao_pontos" => egui::include_image!("../../../assets/icons/padrao_pontos.svg"),
        "parametro" => egui::include_image!("../../../assets/icons/parametro.svg"),
        "pasta" => egui::include_image!("../../../assets/icons/pasta.svg"),
        "personalizar" => egui::include_image!("../../../assets/icons/personalizar.svg"),
        "plano" => egui::include_image!("../../../assets/icons/plano.svg"),
        "plano_2linhas" => egui::include_image!("../../../assets/icons/plano_2linhas.svg"),
        "plano_3pontos" => egui::include_image!("../../../assets/icons/plano_3pontos.svg"),
        "plano_angulo" => egui::include_image!("../../../assets/icons/plano_angulo.svg"),
        "plano_curva" => egui::include_image!("../../../assets/icons/plano_curva.svg"),
        "plano_deslocado" => egui::include_image!("../../../assets/icons/plano_deslocado.svg"),
        "plano_equacao" => egui::include_image!("../../../assets/icons/plano_equacao.svg"),
        "plano_medio" => egui::include_image!("../../../assets/icons/plano_medio.svg"),
        "plano_meio" => egui::include_image!("../../../assets/icons/plano_meio.svg"),
        "plano_normal_curva" => egui::include_image!("../../../assets/icons/plano_normal_curva.svg"),
        "plano_paralelo_ponto" => egui::include_image!("../../../assets/icons/plano_paralelo_ponto.svg"),
        "plano_ponto_linha" => egui::include_image!("../../../assets/icons/plano_ponto_linha.svg"),
        "plano_ponto_normal" => egui::include_image!("../../../assets/icons/plano_ponto_normal.svg"),
        "plano_tangente" => egui::include_image!("../../../assets/icons/plano_tangente.svg"),
        "pmi_angulo" => egui::include_image!("../../../assets/icons/pmi_angulo.svg"),
        "pmi_diametro" => egui::include_image!("../../../assets/icons/pmi_diametro.svg"),
        "pmi_linear" => egui::include_image!("../../../assets/icons/pmi_linear.svg"),
        "pmi_nota" => egui::include_image!("../../../assets/icons/pmi_nota.svg"),
        "pmi_raio" => egui::include_image!("../../../assets/icons/pmi_raio.svg"),
        "pmi_referencia" => egui::include_image!("../../../assets/icons/pmi_referencia.svg"),
        "pmi_rugosidade" => egui::include_image!("../../../assets/icons/pmi_rugosidade.svg"),
        "pmi_solda" => egui::include_image!("../../../assets/icons/pmi_solda.svg"),
        "pmi_tolerancia" => egui::include_image!("../../../assets/icons/pmi_tolerancia.svg"),
        "polilinha3d" => egui::include_image!("../../../assets/icons/polilinha3d.svg"),
        "ponto_centro" => egui::include_image!("../../../assets/icons/ponto_centro.svg"),
        "ponto_coord" => egui::include_image!("../../../assets/icons/ponto_coord.svg"),
        "ponto_curva" => egui::include_image!("../../../assets/icons/ponto_curva.svg"),
        "ponto_deslocado" => egui::include_image!("../../../assets/icons/ponto_deslocado.svg"),
        "ponto_entre" => egui::include_image!("../../../assets/icons/ponto_entre.svg"),
        "ponto_intersecao" => egui::include_image!("../../../assets/icons/ponto_intersecao.svg"),
        "ponto_intersecao_curvas" => egui::include_image!("../../../assets/icons/ponto_intersecao_curvas.svg"),
        "ponto_plano" => egui::include_image!("../../../assets/icons/ponto_plano.svg"),
        "ponto_proj_curva" => egui::include_image!("../../../assets/icons/ponto_proj_curva.svg"),
        "ponto_projetado" => egui::include_image!("../../../assets/icons/ponto_projetado.svg"),
        "ponto_superficie" => egui::include_image!("../../../assets/icons/ponto_superficie.svg"),
        "ponto_tangente" => egui::include_image!("../../../assets/icons/ponto_tangente.svg"),
        "prim_caixa" => egui::include_image!("../../../assets/icons/prim_caixa.svg"),
        "prim_cilindro" => egui::include_image!("../../../assets/icons/prim_cilindro.svg"),
        "prim_cone" => egui::include_image!("../../../assets/icons/prim_cone.svg"),
        "prim_cunha" => egui::include_image!("../../../assets/icons/prim_cunha.svg"),
        "prim_esfera" => egui::include_image!("../../../assets/icons/prim_esfera.svg"),
        "prim_prisma" => egui::include_image!("../../../assets/icons/prim_prisma.svg"),
        "prim_toro" => egui::include_image!("../../../assets/icons/prim_toro.svg"),
        "r_coincidente" => egui::include_image!("../../../assets/icons/r_coincidente.svg"),
        "r_concentrica" => egui::include_image!("../../../assets/icons/r_concentrica.svg"),
        "r_fixa" => egui::include_image!("../../../assets/icons/r_fixa.svg"),
        "r_horizontal" => egui::include_image!("../../../assets/icons/r_horizontal.svg"),
        "r_igual" => egui::include_image!("../../../assets/icons/r_igual.svg"),
        "r_paralela" => egui::include_image!("../../../assets/icons/r_paralela.svg"),
        "r_perpendicular" => egui::include_image!("../../../assets/icons/r_perpendicular.svg"),
        "r_ponto_medio" => egui::include_image!("../../../assets/icons/r_ponto_medio.svg"),
        "r_simetrica" => egui::include_image!("../../../assets/icons/r_simetrica.svg"),
        "r_tangente" => egui::include_image!("../../../assets/icons/r_tangente.svg"),
        "r_vertical" => egui::include_image!("../../../assets/icons/r_vertical.svg"),
        "ranhura" => egui::include_image!("../../../assets/icons/ranhura.svg"),
        "rasgo_varrido" => egui::include_image!("../../../assets/icons/rasgo_varrido.svg"),
        "recorte" => egui::include_image!("../../../assets/icons/recorte.svg"),
        "reforco" => egui::include_image!("../../../assets/icons/reforco.svg"),
        "rest_alinhar" => egui::include_image!("../../../assets/icons/rest_alinhar.svg"),
        "rest_angulo" => egui::include_image!("../../../assets/icons/rest_angulo.svg"),
        "rest_atualizar" => egui::include_image!("../../../assets/icons/rest_atualizar.svg"),
        "rest_auto" => egui::include_image!("../../../assets/icons/rest_auto.svg"),
        "rest_coincidencia" => egui::include_image!("../../../assets/icons/rest_coincidencia.svg"),
        "rest_conjunta" => egui::include_image!("../../../assets/icons/rest_conjunta.svg"),
        "rest_contato" => egui::include_image!("../../../assets/icons/rest_contato.svg"),
        "rest_distancia" => egui::include_image!("../../../assets/icons/rest_distancia.svg"),
        "rest_fixar" => egui::include_image!("../../../assets/icons/rest_fixar.svg"),
        "rest_interferencia" => egui::include_image!("../../../assets/icons/rest_interferencia.svg"),
        "rest_paralelo" => egui::include_image!("../../../assets/icons/rest_paralelo.svg"),
        "rest_perpendicular" => egui::include_image!("../../../assets/icons/rest_perpendicular.svg"),
        "revolucao" => egui::include_image!("../../../assets/icons/revolucao.svg"),
        "rosca" => egui::include_image!("../../../assets/icons/rosca.svg"),
        "saida" => egui::include_image!("../../../assets/icons/saida.svg"),
        "salvar" => egui::include_image!("../../../assets/icons/salvar.svg"),
        "sel_aresta" => egui::include_image!("../../../assets/icons/sel_aresta.svg"),
        "sel_corpo" => egui::include_image!("../../../assets/icons/sel_corpo.svg"),
        "sel_face" => egui::include_image!("../../../assets/icons/sel_face.svg"),
        "sel_filtro" => egui::include_image!("../../../assets/icons/sel_filtro.svg"),
        "sel_vertice" => egui::include_image!("../../../assets/icons/sel_vertice.svg"),
        "sk_aparar" => egui::include_image!("../../../assets/icons/sk_aparar.svg"),
        "sk_arco3" => egui::include_image!("../../../assets/icons/sk_arco3.svg"),
        "sk_arco_centro" => egui::include_image!("../../../assets/icons/sk_arco_centro.svg"),
        "sk_auto_cota" => egui::include_image!("../../../assets/icons/sk_auto_cota.svg"),
        "sk_auto_restricao" => egui::include_image!("../../../assets/icons/sk_auto_restricao.svg"),
        "sk_checar" => egui::include_image!("../../../assets/icons/sk_checar.svg"),
        "sk_circulo" => egui::include_image!("../../../assets/icons/sk_circulo.svg"),
        "sk_circulo3" => egui::include_image!("../../../assets/icons/sk_circulo3.svg"),
        "sk_concluir" => egui::include_image!("../../../assets/icons/sk_concluir.svg"),
        "sk_construcao" => egui::include_image!("../../../assets/icons/sk_construcao.svg"),
        "sk_converter" => egui::include_image!("../../../assets/icons/sk_converter.svg"),
        "sk_copiar" => egui::include_image!("../../../assets/icons/sk_copiar.svg"),
        "sk_cota" => egui::include_image!("../../../assets/icons/sk_cota.svg"),
        "sk_espelhar" => egui::include_image!("../../../assets/icons/sk_espelhar.svg"),
        "sk_estender" => egui::include_image!("../../../assets/icons/sk_estender.svg"),
        "sk_filete" => egui::include_image!("../../../assets/icons/sk_filete.svg"),
        "sk_girar" => egui::include_image!("../../../assets/icons/sk_girar.svg"),
        "sk_grade" => egui::include_image!("../../../assets/icons/sk_grade.svg"),
        "sk_linha" => egui::include_image!("../../../assets/icons/sk_linha.svg"),
        "sk_mover" => egui::include_image!("../../../assets/icons/sk_mover.svg"),
        "sk_offset" => egui::include_image!("../../../assets/icons/sk_offset.svg"),
        "sk_poligono" => egui::include_image!("../../../assets/icons/sk_poligono.svg"),
        "sk_polilinha" => egui::include_image!("../../../assets/icons/sk_polilinha.svg"),
        "sk_ponto" => egui::include_image!("../../../assets/icons/sk_ponto.svg"),
        "sk_rasgo" => egui::include_image!("../../../assets/icons/sk_rasgo.svg"),
        "sk_retangulo" => egui::include_image!("../../../assets/icons/sk_retangulo.svg"),
        "sk_retangulo_centro" => egui::include_image!("../../../assets/icons/sk_retangulo_centro.svg"),
        "sk_selecionar" => egui::include_image!("../../../assets/icons/sk_selecionar.svg"),
        "sk_snap" => egui::include_image!("../../../assets/icons/sk_snap.svg"),
        "sketch" => egui::include_image!("../../../assets/icons/sketch.svg"),
        "sm_aba" => egui::include_image!("../../../assets/icons/sm_aba.svg"),
        "sm_bainha" => egui::include_image!("../../../assets/icons/sm_bainha.svg"),
        "sm_desdobrar" => egui::include_image!("../../../assets/icons/sm_desdobrar.svg"),
        "sm_estampo" => egui::include_image!("../../../assets/icons/sm_estampo.svg"),
        "sm_furo_flangeado" => egui::include_image!("../../../assets/icons/sm_furo_flangeado.svg"),
        "sm_joggle" => egui::include_image!("../../../assets/icons/sm_joggle.svg"),
        "sm_parametros" => egui::include_image!("../../../assets/icons/sm_parametros.svg"),
        "sm_parede" => egui::include_image!("../../../assets/icons/sm_parede.svg"),
        "spine" => egui::include_image!("../../../assets/icons/spine.svg"),
        "spline" => egui::include_image!("../../../assets/icons/spline.svg"),
        "spline_controle" => egui::include_image!("../../../assets/icons/spline_controle.svg"),
        "st_biblioteca" => egui::include_image!("../../../assets/icons/st_biblioteca.svg"),
        "st_canto_aparar" => egui::include_image!("../../../assets/icons/st_canto_aparar.svg"),
        "st_canto_encaixe" => egui::include_image!("../../../assets/icons/st_canto_encaixe.svg"),
        "st_canto_meia" => egui::include_image!("../../../assets/icons/st_canto_meia.svg"),
        "st_canto_nenhum" => egui::include_image!("../../../assets/icons/st_canto_nenhum.svg"),
        "st_corte" => egui::include_image!("../../../assets/icons/st_corte.svg"),
        "st_membro" => egui::include_image!("../../../assets/icons/st_membro.svg"),
        "st_placa" => egui::include_image!("../../../assets/icons/st_placa.svg"),
        "st_reforco" => egui::include_image!("../../../assets/icons/st_reforco.svg"),
        "sup_cilindro" => egui::include_image!("../../../assets/icons/sup_cilindro.svg"),
        "sup_concordancia" => egui::include_image!("../../../assets/icons/sup_concordancia.svg"),
        "sup_deslocada" => egui::include_image!("../../../assets/icons/sup_deslocada.svg"),
        "sup_esfera" => egui::include_image!("../../../assets/icons/sup_esfera.svg"),
        "sup_extraida" => egui::include_image!("../../../assets/icons/sup_extraida.svg"),
        "sup_extrudada" => egui::include_image!("../../../assets/icons/sup_extrudada.svg"),
        "sup_multisecoes" => egui::include_image!("../../../assets/icons/sup_multisecoes.svg"),
        "sup_preenchida" => egui::include_image!("../../../assets/icons/sup_preenchida.svg"),
        "sup_revolucao" => egui::include_image!("../../../assets/icons/sup_revolucao.svg"),
        "sup_tubo" => egui::include_image!("../../../assets/icons/sup_tubo.svg"),
        "sup_varredura" => egui::include_image!("../../../assets/icons/sup_varredura.svg"),
        "sweep_corrigido" => egui::include_image!("../../../assets/icons/sweep_corrigido.svg"),
        "sweep_fixo" => egui::include_image!("../../../assets/icons/sweep_fixo.svg"),
        "sweep_frenet" => egui::include_image!("../../../assets/icons/sweep_frenet.svg"),
        "vista_direita" => egui::include_image!("../../../assets/icons/vista_direita.svg"),
        "vista_frente" => egui::include_image!("../../../assets/icons/vista_frente.svg"),
        "vista_iso" => egui::include_image!("../../../assets/icons/vista_iso.svg"),
        "vista_topo" => egui::include_image!("../../../assets/icons/vista_topo.svg"),
        "xf_afinidade" => egui::include_image!("../../../assets/icons/xf_afinidade.svg"),
        "xf_escala" => egui::include_image!("../../../assets/icons/xf_escala.svg"),
        "xf_girar" => egui::include_image!("../../../assets/icons/xf_girar.svg"),
        "xf_mover" => egui::include_image!("../../../assets/icons/xf_mover.svg"),
        "xf_simetria" => egui::include_image!("../../../assets/icons/xf_simetria.svg"),
        _ => egui::include_image!("../../../assets/icons/config.svg"),
    }
}

/// Botão de ferramenta: ícone grande com o nome embaixo (ou só o ícone).
/// Dica: descrição breve na hora; mantendo o mouse 5 s, aparece a descrição avançada.
#[allow(clippy::too_many_arguments)]
pub fn tool_button(ui: &mut egui::Ui, icon: &str, label: &str, size: f32, show_label: bool, selected: bool, brief: &str, advanced: &str) -> egui::Response {
    let (label, brief, advanced) = (tr(label), tr(brief), tr(advanced));
    let font = egui::FontId::proportional(11.5);
    let galley = show_label.then(|| ui.painter().layout_no_wrap(label.to_owned(), font, ui.visuals().text_color()));
    let text_w = galley.as_ref().map(|g| g.size().x).unwrap_or(0.);
    let text_h = galley.as_ref().map(|g| g.size().y + 2.).unwrap_or(0.);
    let w = (size + 10.).max(text_w + 10.);
    let h = size + 8. + text_h;
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::click());
    let resp = hover_help(ui, resp, label, brief, advanced);
    if ui.is_rect_visible(rect) {
        let v = ui.visuals();
        let fill = if selected {
            v.selection.bg_fill
        } else if resp.hovered() {
            v.widgets.hovered.weak_bg_fill
        } else {
            egui::Color32::TRANSPARENT
        };
        ui.painter().rect_filled(rect, 6., fill);
        let icon_rect = egui::Rect::from_center_size(egui::pos2(rect.center().x, rect.top() + 4. + size / 2.), egui::vec2(size, size));
        egui::Image::new(get(icon)).paint_at(ui, icon_rect);
        if let Some(g) = galley {
            let pos = egui::pos2(rect.center().x - g.size().x / 2., icon_rect.bottom() + 2.);
            ui.painter().galley(pos, g, v.text_color());
        }
    }
    resp
}

/// Dica em dois níveis: breve imediatamente; avançada depois de 5 s com o mouse parado em cima.
pub fn hover_help(ui: &egui::Ui, resp: egui::Response, title: &str, brief: &str, advanced: &str) -> egui::Response {
    let key = resp.id.with("hover_inicio");
    let now = ui.input(|i| i.time);
    if !resp.hovered() {
        ui.ctx().data_mut(|d| d.remove::<f64>(key));
        return resp;
    }
    let start = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(key, || now));
    let long = now - start >= 5.0 && !advanced.is_empty();
    if !advanced.is_empty() && !long {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(250));
    }
    resp.on_hover_ui(|ui| {
        ui.set_max_width(320.);
        ui.label(egui::RichText::new(title).strong());
        if !brief.is_empty() && brief != title {
            ui.label(brief);
        }
        if long {
            ui.separator();
            ui.label(egui::RichText::new(advanced).small());
        } else if !advanced.is_empty() {
            ui.label(egui::RichText::new(tr("(mantenha o mouse para mais detalhes)")).small().weak());
        }
    })
}
