// SPDX-License-Identifier: LGPL-2.1-or-later
//! A lista fechada de ferramentas. Cada uma aponta para um comando de [`fenkra_cli::Sessao`]; o que
//! não está aqui o agente não alcança, mesmo que a sessão tenha o comando (`topologia` com caminho
//! livre, `reproduzir`).

use fenkra_cli::topologia::SECOES;
use fenkra_cli::ErroApi;
use serde_json::{json, Value};

pub struct Ferramenta {
    pub nome: &'static str,
    pub(crate) titulo: &'static str,
    pub(crate) descricao: &'static str,
    pub comando: &'static str,
    /// devolve dado lido de um STEP: exige o aceite da pessoa
    pub le_step: bool,
    /// não muda modelo nem arquivo
    pub(crate) somente_leitura: bool,
    pub(crate) esquema: fn() -> Value,
    pub traduzir: fn(&Value) -> Result<Value, ErroApi>,
}

impl Ferramenta {
    pub fn json(&self) -> Value {
        json!({
            "name": self.nome, "title": self.titulo, "description": self.descricao, "inputSchema": (self.esquema)(),
            "annotations": { "readOnlyHint": self.somente_leitura, "destructiveHint": false, "openWorldHint": false },
        })
    }
}

fn igual(a: &Value) -> Result<Value, ErroApi> {
    Ok(a.clone())
}

fn sem_args() -> Value {
    json!({ "type": "object", "additionalProperties": false, "properties": {} })
}

fn objeto(propriedades: Value, exigidos: &[&str]) -> Value {
    json!({ "type": "object", "properties": propriedades, "required": exigidos })
}

/// Valor de cota: número em mm ou expressão com parâmetros.
fn valor(descricao: &str) -> Value {
    json!({ "type": ["string", "number"], "description": descricao })
}

fn identificacao() -> Value {
    json!({ "id": { "type": "string", "description": "id do item; ausente, o servidor cria" }, "nome": { "type": "string" } })
}

fn juntar(mut a: Value, b: Value) -> Value {
    if let (Some(x), Some(y)) = (a.as_object_mut(), b.as_object()) {
        x.extend(y.iter().map(|(k, v)| (k.clone(), v.clone())));
    }
    a
}

fn termino() -> Value {
    json!({ "type": "object", "description": "tipo: cego e simetrico (distancia), dois_lados (a, b), atravessar, ate_plano (plano), ate_proxima, ate_ultima" })
}

fn plano() -> Value {
    json!({ "description": "\"XY\", \"XZ\", \"YZ\" ou {\"referencia\": id de criar_plano}. Eixos do esboço: XY usa x=X, y=Y, normal +Z; XZ usa x=X, y=Z, normal -Y; YZ usa x=Y, y=Z, normal +X" })
}

fn arestas() -> Value {
    json!({ "description": "\"todas\", {\"geradas_por\": id} ou lista de pontos [x, y, z], um sobre cada aresta (o meio, nunca um vértice)" })
}

fn arquivo_e_sobrescrever(descricao: &str) -> Value {
    objeto(
        json!({ "arquivo": { "type": "string", "description": descricao }, "sobrescrever": { "type": "boolean", "description": "true troca um arquivo que já existe; sem isto, ele é recusado" } }),
        &["arquivo"],
    )
}

fn e_ver() -> Value {
    objeto(
        json!({
            "vista": { "enum": ["frente", "tras", "topo", "base", "esquerda", "direita", "iso", "todas"], "description": "padrão iso; todas junta frente, topo, direita e iso" },
            "corte": {
                "type": "object",
                "description": "tira a metade da peça de um lado do plano eixo = posição",
                "properties": {
                    "eixo": { "enum": ["x", "y", "z"] },
                    "posicao": { "type": "number", "description": "mm" },
                    "lado": { "enum": ["menor", "maior"], "description": "qual metade fica; padrão menor" }
                },
                "required": ["eixo", "posicao"]
            },
            "zoom": { "type": "number", "description": "1 enquadra a peça; de 0,1 a 50" },
            "largura": { "type": "integer", "description": "pixels, de 64 a 2400; padrão 800" },
            "altura": { "type": "integer", "description": "pixels, de 64 a 2400; padrão 600" },
            "arestas": { "type": "boolean", "description": "desenha as arestas; padrão verdadeiro" },
            "fundo": { "enum": ["claro", "escuro"] }
        }),
        &[],
    )
}

/// `ver` nunca grava arquivo pelo MCP: a imagem volta na resposta.
fn t_ver(a: &Value) -> Result<Value, ErroApi> {
    let mut a = a.clone();
    if let Some(o) = a.as_object_mut() {
        o.remove("saida");
        o.remove("sobrescrever");
    }
    Ok(a)
}

fn e_salvar() -> Value {
    arquivo_e_sobrescrever("caminho .fkx ou .fkp relativo à pasta do projeto")
}

fn e_exportar() -> Value {
    arquivo_e_sobrescrever("caminho .step ou .stp relativo à pasta do projeto")
}

fn e_consultar() -> Value {
    objeto(
        json!({
            "arquivo": { "type": "string", "description": "STEP dentro da pasta do projeto" },
            "secoes": { "type": "array", "items": { "enum": SECOES }, "description": "ausente, vêm todas" },
            "limite": { "type": "integer", "description": "máximo de faces e arestas devolvidas" },
        }),
        &["arquivo"],
    )
}

fn e_topologia() -> Value {
    objeto(
        json!({ "secoes": { "type": "array", "items": { "enum": SECOES } }, "limite": { "type": "integer" } }),
        &[],
    )
}

fn e_nova() -> Value {
    objeto(json!({ "nome": { "type": "string" } }), &[])
}

fn e_abrir() -> Value {
    objeto(
        json!({ "arquivo": { "type": "string", "description": "peça .fkx ou .fkp dentro da pasta do projeto" } }),
        &["arquivo"],
    )
}

fn e_parametro() -> Value {
    objeto(
        json!({ "nome": { "type": "string" }, "expressao": valor("número ou expressão"), "grandeza": { "enum": ["comprimento", "angulo", "numero"] } }),
        &["nome", "expressao"],
    )
}

fn e_plano() -> Value {
    objeto(
        juntar(
            identificacao(),
            json!({ "base": plano(), "deslocamento": valor("distância ao longo da normal da base") }),
        ),
        &["deslocamento"],
    )
}

fn e_esboco() -> Value {
    let propriedades = json!({
        "plano": plano(),
        "pontos": { "type": "array", "description": "{id, x, y, fixo?}. Curvas que dividem um ponto usam o mesmo id: é assim que o perfil fecha" },
        "curvas": { "type": "array", "description": "{id, construcao?, geometria}, com geometria.tipo reta (a, b), circulo (centro, raio), arco (centro, inicio, fim, anti-horário), ponto (ponto), spline (pontos)" },
        "restricoes": { "type": "array", "description": "{tipo, entidades, valor?}. Cotas: comprimento, raio, diametro, distancia, distancia_x, distancia_y, angulo. Geométricas: horizontal, vertical, coincidente, paralela, perpendicular, igual, tangente, concentrica" },
    });
    objeto(
        juntar(identificacao(), propriedades),
        &["plano", "pontos", "curvas"],
    )
}

fn e_extrudar() -> Value {
    let propriedades = json!({
        "esboco": { "type": "string" }, "operacao": { "enum": ["unir", "cortar", "intersectar"] }, "termino": termino(),
        "inverter": { "type": "boolean", "description": "vai contra a normal do plano do esboço" },
    });
    objeto(
        juntar(identificacao(), propriedades),
        &["esboco", "operacao", "termino"],
    )
}

fn e_furar() -> Value {
    let propriedades = json!({
        "esboco": { "type": "string", "description": "esboço com um ponto em cada centro; o furo entra contra a normal do plano" },
        "diametro": valor("diâmetro em mm"), "termino": termino(), "inverter": { "type": "boolean" },
        "forma": { "type": "object", "description": "tipo: simples (padrão), rebaixado (diametro, profundidade), escareado (diametro, angulo), contrafurado (diametro, profundidade, angulo)" },
    });
    objeto(
        juntar(identificacao(), propriedades),
        &["esboco", "diametro", "termino"],
    )
}

fn e_filetar() -> Value {
    objeto(
        juntar(
            identificacao(),
            json!({ "raio": valor("raio em mm"), "arestas": arestas() }),
        ),
        &["raio", "arestas"],
    )
}

fn e_chanfrar() -> Value {
    objeto(
        juntar(
            identificacao(),
            json!({ "distancia": valor("distância em mm"), "arestas": arestas() }),
        ),
        &["distancia", "arestas"],
    )
}

fn e_comparar() -> Value {
    objeto(
        json!({
            "alvo": { "type": "string", "description": "STEP alvo dentro da pasta do projeto" },
            "tolerancias": { "type": "object", "description": "opcional: volume, area, centroide, caixa, inercia, distancia, furo_diametro, furo_posicao" },
        }),
        &["alvo"],
    )
}

/// Argumentos de cota podem vir como número: o `.fkx` só aceita texto.
fn cotas_em_texto(a: &Value, chaves: &[&str]) -> Result<Value, ErroApi> {
    let mut v = a.clone();
    for c in chaves {
        if let Some(n) = v[*c].as_f64() {
            v[*c] = json!(n.to_string());
        }
    }
    Ok(v)
}

fn t_furo(a: &Value) -> Result<Value, ErroApi> {
    cotas_em_texto(a, &["diametro"])
}

fn t_filete(a: &Value) -> Result<Value, ErroApi> {
    cotas_em_texto(a, &["raio"])
}

fn t_chanfro(a: &Value) -> Result<Value, ErroApi> {
    cotas_em_texto(a, &["distancia"])
}

/// A consulta recebe só STEP: esta porta não vira leitor de outro arquivo do projeto.
fn t_consultar(a: &Value) -> Result<Value, ErroApi> {
    so_step(a["arquivo"].as_str().unwrap_or_default())?;
    Ok(a.clone())
}

fn t_comparar(a: &Value) -> Result<Value, ErroApi> {
    so_step(a["alvo"].as_str().unwrap_or_default())?;
    let mut v = json!({ "alvo": a["alvo"] });
    if !a["tolerancias"].is_null() {
        v["tolerancias"] = a["tolerancias"].clone();
    }
    Ok(v)
}

fn t_topologia(a: &Value) -> Result<Value, ErroApi> {
    if !a["arquivo"].is_null() {
        return Err(ErroApi::argumento(
            "topologia_modelo só consulta a peça em edição; para um STEP, use consultar_step",
        ));
    }
    Ok(a.clone())
}

fn so_step(arquivo: &str) -> Result<(), ErroApi> {
    let a = arquivo.to_ascii_lowercase();
    if a.ends_with(".step") || a.ends_with(".stp") {
        Ok(())
    } else {
        Err(ErroApi::novo("formato", "informe um arquivo .step ou .stp"))
    }
}

const fn leitura(
    nome: &'static str,
    titulo: &'static str,
    descricao: &'static str,
    comando: &'static str,
    esquema: fn() -> Value,
    traduzir: fn(&Value) -> Result<Value, ErroApi>,
) -> Ferramenta {
    Ferramenta {
        nome,
        titulo,
        descricao,
        comando,
        le_step: false,
        somente_leitura: true,
        esquema,
        traduzir,
    }
}

const fn edicao(
    nome: &'static str,
    titulo: &'static str,
    descricao: &'static str,
    comando: &'static str,
    esquema: fn() -> Value,
    traduzir: fn(&Value) -> Result<Value, ErroApi>,
) -> Ferramenta {
    Ferramenta {
        nome,
        titulo,
        descricao,
        comando,
        le_step: false,
        somente_leitura: false,
        esquema,
        traduzir,
    }
}

fn e_selecionar() -> Value {
    objeto(
        json!({
            "alvo": { "enum": ["arestas", "faces"], "description": "padrão arestas" },
            "lado": { "description": "topo, base, frente, tras, esquerda, direita ou uma normal [x, y, z]: fica com as faces planas que olham para lá (e as arestas delas)" },
            "extrema": { "type": "boolean", "description": "padrão verdadeiro: só a camada mais afastada daquele lado (a face de cima, não os degraus)" },
            "tipo": { "type": "string", "description": "arestas: reta, circulo, arco, elipse; faces: plano, cilindro, cone, esfera, toro, livre" },
            "raio": { "type": "number", "description": "mm; filtra círculos, arcos e cilindros pelo raio" },
            "perto": { "description": "ponto [x, y, z]: fica com o que estiver mais perto dele (o meio da aresta, um ponto da face)" },
            "quantas": { "type": "integer", "description": "com `perto`, quantas devolver; padrão 1" },
        }),
        &[],
    )
}

fn e_topologia_sem_secoes() -> Value {
    sem_args()
}

fn e_verificar() -> Value {
    objeto(
        json!({
            "requisitos": {
                "type": "array",
                "description": "somam-se aos que a peça declara. Cada um: {nome, tipo, ...}. massa (min, max em kg); furos (diametro, quantidade, tol); folga (min em mm entre corpos); sem_interferencia (tol em mm³); caixa (max [x, y, z] em mm); propagacao (parametro, valor, medida: volume, caixa_x, caixa_y, caixa_z ou massa; esperado e tol opcionais)",
                "items": { "type": "object", "required": ["nome", "tipo"] }
            }
        }),
        &[],
    )
}

fn e_biblioteca() -> Value {
    objeto(
        json!({
            "busca": { "type": "string", "description": "filtra as famílias por nome, norma ou id" },
            "familia": { "type": "string", "description": "id da família: traz os tamanhos com medidas e furo de passagem" },
            "tamanho": { "type": "string", "description": "designação, como M6" },
            "furo": { "type": "number", "description": "mm: diz quais tamanhos da família passam num furo deste diâmetro" },
        }),
        &[],
    )
}

fn e_chapa() -> Value {
    objeto(
        json!({
            "espessura": valor("mm"), "raio": valor("raio interno de dobra padrão, mm"), "fator_k": valor("posição da linha neutra: 0 face interna, 0,5 meio"),
            "planificada": { "type": "boolean", "description": "true mostra e mede a peça desdobrada" },
            "material": { "type": "string" },
            "metodo": { "enum": ["fator_k", "deducao", "tolerancia"], "description": "compensação da dobra; com deducao ou tolerancia, informe `compensacao` (mm a 90°)" },
            "compensacao": valor("dedução ou tolerância de dobra a 90°, mm"),
            "alivio": valor("largura padrão do alívio de dobra"), "alivio_forma": { "enum": ["retangular", "redondo", "rasgo"] },
            "canto": valor("tamanho padrão do alívio de canto"),
        }),
        &[],
    )
}

fn componente_ref(descricao: &str) -> Value {
    json!({
        "type": "object", "description": descricao,
        "properties": {
            "componente": { "type": "string", "description": "id do componente" },
            "face": { "type": "array", "items": { "type": "number" }, "description": "ponto [x, y, z] no mundo sobre a face, na posição atual do componente: face plana vira plano, cilíndrica vira eixo" },
            "plano": { "enum": ["XY", "XZ", "YZ"], "description": "plano da origem da peça" },
            "eixo": { "enum": ["X", "Y", "Z"], "description": "eixo da origem da peça" },
            "ponto": { "type": "array", "items": { "type": "number" }, "description": "ponto [x, y, z] no mundo, preso ao componente" },
        },
        "required": ["componente"],
    })
}

fn matriz() -> Value {
    json!({ "type": "array", "description": "rotação 3 × 3, linha a linha: as linhas são os eixos X, Y e Z da peça no mundo" })
}

fn e_inserir() -> Value {
    objeto(
        juntar(
            identificacao(),
            json!({
                "arquivo": { "type": "string", "description": "peça .fkp da pasta do projeto (grave com salvar_peca)" },
                "origem": { "type": "array", "items": { "type": "number" }, "description": "posição [x, y, z] da origem da peça; padrão 0" },
                "rotacao": matriz(),
                "fixo": { "type": "boolean", "description": "fixo não se move; padrão: só o primeiro componente" },
            }),
        ),
        &["arquivo"],
    )
}

fn e_restringir() -> Value {
    objeto(
        json!({
            "id": { "type": "string" },
            "tipo": { "enum": ["coincidente", "contato", "distancia", "angulo", "paralela", "perpendicular"], "description": "contato: faces de frente uma para a outra; coincidente: mesmo plano, eixo ou ponto" },
            "a": componente_ref("elemento do primeiro componente"), "b": componente_ref("elemento do segundo componente"),
            "valor": valor("distância em mm ou ângulo em graus"),
            "invertida": { "type": "boolean", "description": "coincidente com as normais opostas" },
            "alinhamento": { "enum": ["mesmo", "oposto", "livre"], "description": "sentido das normais em distância, ângulo e paralela" },
            "lado_invertido": { "type": "boolean", "description": "mede a distância do outro lado da normal de a" },
        }),
        &["tipo", "a", "b"],
    )
}

fn e_mover() -> Value {
    objeto(
        json!({
            "componente": { "type": "string" },
            "origem": { "type": "array", "items": { "type": "number" }, "description": "posição absoluta da origem da peça" },
            "rotacao": matriz(),
            "deslocar": { "type": "array", "items": { "type": "number" }, "description": "translação relativa [dx, dy, dz]" },
            "girar": { "type": "object", "description": "{eixo: X, Y, Z ou [x, y, z], angulo em graus, centro?: [x, y, z]}; centro padrão é a origem da peça" },
        }),
        &["componente"],
    )
}

fn e_bom() -> Value {
    objeto(json!({ "nivel": { "enum": ["topo", "pecas", "estruturada"], "description": "padrão topo" } }), &[])
}

fn e_salvar_montagem() -> Value {
    arquivo_e_sobrescrever("caminho .fka (montagem que o aplicativo abre) ou .fkx (formato aberto: componentes e posições, sem restrições) relativo à pasta do projeto")
}

fn t_chapa(a: &Value) -> Result<Value, ErroApi> {
    cotas_em_texto(a, &["espessura", "raio", "fator_k", "compensacao", "alivio", "canto"])
}

pub const FERRAMENTAS: [Ferramenta; 31] = [
    Ferramenta { nome: "consultar_step", titulo: "Consultar STEP", descricao: "Lista caixa envolvente, massa, faces, arestas, planos, cilindros e furos de um STEP do projeto. Os planos trazem a normal para fora, as arestas trazem a fórmula, e os furos já vêm agrupados. Exige o aceite --liberar-step.", comando: "topologia", le_step: true, somente_leitura: true, esquema: e_consultar, traduzir: t_consultar },
    edicao("nova_peca", "Nova peça", "Começa uma peça vazia.", "nova_peca", e_nova, igual),
    edicao("abrir_peca", "Abrir peça", "Abre uma peça .fkx ou .fkp do projeto.", "abrir", e_abrir, igual),
    leitura("listar_parametros", "Listar parâmetros", "Parâmetros da peça, com expressão e valor.", "parametros", sem_args, igual),
    edicao("definir_parametro", "Definir parâmetro", "Cria ou muda um parâmetro. Use o nome nas cotas e comandos para a peça continuar editável.", "definir_parametro", e_parametro, igual),
    edicao("criar_plano", "Criar plano", "Plano deslocado de outro, para esboçar fora da origem.", "criar_plano", e_plano, igual),
    edicao("criar_esboco", "Criar esboço", "Esboço com geometria, restrições e cotas, no formato de esboço do .fkx. Devolve os graus de liberdade restantes.", "criar_esboco", e_esboco, igual),
    edicao("extrudar", "Extrudar", "Extrusão de um esboço: une, corta ou intersecta.", "extrudar", e_extrudar, igual),
    edicao("furar", "Furar", "Furo em cada ponto de um esboço.", "furar", e_furar, t_furo),
    edicao("filetar", "Filetar", "Filete nas arestas indicadas.", "filetar", e_filetar, t_filete),
    edicao("chanfrar", "Chanfrar", "Chanfro nas arestas indicadas.", "chanfrar", e_chanfrar, t_chanfro),
    edicao("desfazer", "Desfazer", "Volta o último comando que mudou a peça.", "desfazer", sem_args, igual),
    leitura("recalcular", "Recalcular", "Recalcula e devolve o status de cada item, volume, área e caixa.", "recalcular", sem_args, igual),
    leitura("medir", "Medir", "Volume, área, centroide, inércia e caixa da peça.", "medir", sem_args, igual),
    leitura("ver", "Ver a peça", "Imagem da peça em edição, sem aplicativo aberto: vista (frente, tras, topo, base, esquerda, direita, iso ou todas, que junta quatro), corte por plano (eixo, posição em mm, lado que fica) e zoom. A imagem volta como imagem; use para conferir o que montou antes de comparar. O corte pinta a face interna de laranja.", "ver", e_ver, t_ver),
    leitura("topologia_modelo", "Topologia da peça", "Faces, arestas, planos, cilindros e furos da peça em edição. Use os meios das arestas para filetar.", "topologia", e_topologia, t_topologia),
    Ferramenta { nome: "comparar_alvo", titulo: "Comparar com o alvo", descricao: "Checador de similaridade: compara a peça com o STEP alvo por volume, área, centroide, caixa, inércia, furos por diâmetro e distância entre superfícies, e diz onde diverge. Exige o aceite --liberar-step.", comando: "comparar", le_step: true, somente_leitura: true, esquema: e_comparar, traduzir: t_comparar },
    edicao("salvar_peca", "Salvar peça", "Grava a peça em .fkx ou .fkp. Nunca troca um arquivo que existe sem `sobrescrever`.", "salvar", e_salvar, igual),
    leitura("selecionar", "Selecionar faces ou arestas", "Seleção por sentido: as arestas da face de cima, os círculos de raio 4, as faces que olham para a frente. Devolve `pontos` prontos para `arestas` de filetar e chanfrar ou `faces` de criar_casca.", "selecionar", e_selecionar, igual),
    leitura("reconhecer_features", "Reconhecer features", "O que a geometria da peça em edição mostra: furos passantes e cegos (com rebaixo, escareado e ponta), filetes e chanfros, com medidas e centro. É inferência da forma, não histórico.", "features", e_topologia_sem_secoes, t_topologia),
    leitura("verificar", "Verificar requisitos", "Confere os requisitos da peça, um resultado por requisito com o que foi medido: massa, furos, folga e interferência entre corpos, caixa máxima e se um parâmetro trocado chega na forma. Serve de critério quando não há STEP alvo. Não muda a peça.", "verificar", e_verificar, igual),
    leitura("consultar_biblioteca", "Consultar biblioteca de normas", "Famílias de fixadores normalizados, tamanhos, medidas e a faixa do furo de passagem. Use antes de furar para um parafuso.", "biblioteca", e_biblioteca, igual),
    edicao("exportar_step", "Exportar STEP", "Grava o sólido em STEP. Nunca troca um arquivo que existe sem `sobrescrever`.", "exportar", e_exportar, igual),
    edicao("definir_chapa", "Definir chapa", "Liga a chapa metálica na peça ou muda espessura, raio de dobra, fator K e compensação. Só os campos informados mudam; a primeira vez parte de 2 mm, raio 2 e K 0,44. Com `planificada` verdadeiro a peça é medida e mostrada desdobrada. Depois use criar_parede_chapa, criar_aba, criar_bainha e dobrar_chapa.", "definir_chapa", e_chapa, t_chapa),
    edicao("nova_montagem", "Nova montagem", "Começa uma montagem vazia, ao lado da peça em edição. Peças entram por inserir_componente a partir de arquivos .fkp da pasta.", "nova_montagem", e_nova, igual),
    edicao("inserir_componente", "Inserir componente", "Põe uma peça .fkp da pasta na montagem, na posição dada. A primeira entra fixa. Devolve o id do componente.", "inserir_componente", e_inserir, igual),
    edicao("restringir", "Restringir componentes", "Restrição entre dois componentes: coincidente, contato, distância, ângulo, paralela ou perpendicular. Cada elemento é uma face (ponto sobre ela no mundo), um plano ou eixo da origem da peça, ou um ponto. Resolve na hora e devolve os graus de liberdade que sobram e o que falhou.", "restringir", e_restringir, igual),
    edicao("mover_componente", "Mover componente", "Muda a posição de um componente (absoluta, deslocamento ou giro) e resolve as restrições de novo: o que está preso volta ao lugar.", "mover_componente", e_mover, igual),
    leitura("ver_montagem", "Estado da montagem", "Componentes com posição, graus de liberdade, volume e caixa de cada um; restrições com o que falhou; volume e caixa do conjunto.", "montagem", sem_args, igual),
    leitura("lista_de_pecas", "Lista de peças", "Lista de peças (BOM) da montagem: item, nome, quantidade, arquivo, PN e volume unitário, no nível topo, pecas (todos os níveis somados) ou estruturada.", "bom", e_bom, igual),
    edicao("salvar_montagem", "Salvar montagem", "Grava a montagem em .fka, que o aplicativo abre com as restrições, ou em .fkx. Nunca troca um arquivo que existe sem `sobrescrever`.", "salvar_montagem", e_salvar_montagem, igual),
];
