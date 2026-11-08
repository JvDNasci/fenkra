// SPDX-License-Identifier: LGPL-2.1-or-later
//! A sessão de trabalho: uma peça aberta e a lista fechada de comandos sobre ela.

use crate::checador::{self, Tolerancias};
use crate::filho;
use crate::pasta::Pasta;
use crate::registro::{ler_comandos, Registro};
use crate::topologia::{self, Resumo};
use crate::{r6, r6v, ErroApi, API_VERSAO};
use fenkra_conversor::arvore::{self, Arquivo, Grandeza, Parametro, Peca};
use fenkra_core::{Document, FeatureKind, Unit};
use fenkra_kernel::Shape;
use fenkra_solidos::{final_shape, recompute, RecomputeResult, Status};
use serde_json::{json, Value};
use std::path::PathBuf;

/// Todos os comandos que existem. Nada fora desta lista roda, e nenhum deles interpreta código.
pub const COMANDOS: [&str; 29] = [
    "versao",
    "comandos",
    "nova_peca",
    "abrir",
    "salvar",
    "exportar",
    "arvore",
    "relatorio",
    "parametros",
    "definir_parametro",
    "criar_plano",
    "criar_esboco",
    "extrudar",
    "furar",
    "filetar",
    "chanfrar",
    "desfazer",
    "recalcular",
    "medir",
    "ver",
    "topologia",
    "features",
    "comparar",
    "reproduzir",
    "registro",
    "adicionar",
    "selecionar",
    "biblioteca",
    "verificar",
];

/// Os que mudam o modelo: são estes que entram no registro e que `reproduzir` aceita.
pub const COMANDOS_DE_MODELO: [&str; 11] = [
    "nova_peca",
    "definir_parametro",
    "criar_plano",
    "criar_esboco",
    "extrudar",
    "furar",
    "filetar",
    "chanfrar",
    "desfazer",
    "abrir",
    "adicionar",
];

const LEITURA_PECA: [&str; 2] = ["fkx", "fkp"];
const LEITURA_ALVO: [&str; 4] = ["step", "stp", "fkx", "fkp"];
const GRAVACAO_STEP: [&str; 2] = ["step", "stp"];

/// Onde roda a leitura de STEP.
#[derive(Clone, Debug)]
pub enum Isolamento {
    /// no próprio processo: testes e o filho
    Local,
    /// no programa indicado, chamado com `--filho`
    Processo(PathBuf),
}

/// A peça é uma árvore enquanto couber no formato aberto. Um `.fkp` com comando fora dele continua
/// medindo e exportando, mas não recebe item novo: gravar pela árvore perderia o que ficou de fora.
#[derive(Clone)]
enum Modelo {
    Arvore(Peca),
    Documento(Box<Document>),
}

struct Calculado {
    doc: Document,
    r: RecomputeResult,
}

pub struct Sessao {
    pasta: Pasta,
    isolamento: Isolamento,
    modelo: Option<Modelo>,
    calculado: Option<Calculado>,
    /// estado anterior a cada comando, para `desfazer`
    historico: Vec<Option<Modelo>>,
    /// liga o artefato da sequência de comandos
    registrar: bool,
    registro: Option<Registro>,
    ultima_comparacao: Option<Value>,
}

fn arquivo_de(peca: &Peca) -> Arquivo {
    Arquivo {
        formato: arvore::NOME_FORMATO.into(),
        format_version: fenkra_core::formato::Tipo::Arvore.versao_atual(),
        gerador: format!("Fenkra {}", env!("CARGO_PKG_VERSION")),
        unidades: "mm".into(),
        pecas: vec![peca.clone()],
        montagem: None,
        fora: vec![],
    }
}

fn texto(args: &Value, chave: &str) -> Result<String, ErroApi> {
    args[chave]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| ErroApi::argumento(format!("falta o texto `{chave}`")))
}

fn expressao(args: &Value, chave: &str) -> Result<String, ErroApi> {
    match &args[chave] {
        Value::String(s) => Ok(s.clone()),
        Value::Number(n) => Ok(n.to_string()),
        _ => Err(ErroApi::argumento(format!(
            "falta `{chave}` (número ou expressão)"
        ))),
    }
}

/// Acrescenta os campos do diagnóstico acionável (B419) ao status do item.
fn juntar(destino: &mut Value, diag: Value) {
    if let (Some(d), Value::Object(o)) = (destino.as_object_mut(), diag) {
        d.extend(o);
    }
}

fn status_json(s: Option<&Status>) -> Value {
    match s {
        Some(Status::Ok) | None => json!({ "status": "ok" }),
        Some(Status::Warning(m)) => json!({ "status": "aviso", "mensagem": m }),
        Some(Status::Error(m)) => json!({ "status": "erro", "mensagem": m }),
        Some(Status::Inactive) => json!({ "status": "inativo" }),
    }
}

/// Valida a árvore inteira pelo mesmo caminho de um arquivo aberto: esquema publicado, depois leitura.
fn validar(arquivo: &Value) -> Result<Peca, ErroApi> {
    let lido = arvore::ler_texto(&arquivo.to_string())
        .map_err(|e| ErroApi::novo("arvore", e.to_string()))?;
    lido.pecas
        .into_iter()
        .next()
        .ok_or_else(|| ErroApi::novo("arvore", "arquivo sem peça"))
}

impl Sessao {
    pub fn nova(pasta: Pasta, isolamento: Isolamento) -> Sessao {
        Sessao {
            pasta,
            isolamento,
            modelo: None,
            calculado: None,
            historico: vec![],
            registrar: false,
            registro: None,
            ultima_comparacao: None,
        }
    }

    /// Liga o artefato com a sequência de comandos (`comandos/sessao-NNN.jsonl`).
    pub fn com_registro(mut self) -> Sessao {
        self.registrar = true;
        self
    }

    pub fn pasta(&self) -> &Pasta {
        &self.pasta
    }

    /// Monta a sessão do filho a partir do pedido.
    pub(crate) fn de_pedido(pedido: &Value) -> Result<Sessao, ErroApi> {
        let mut s = Sessao::nova(Pasta::nova(texto(pedido, "pasta")?)?, Isolamento::Local);
        if let Some(t) = pedido["peca"].as_str() {
            s.modelo = Some(Modelo::Arvore(validar(
                &serde_json::from_str::<Value>(t)
                    .map_err(|e| ErroApi::novo("arvore", e.to_string()))?,
            )?));
        } else if let Some(t) = pedido["documento"].as_str() {
            s.modelo = Some(Modelo::Documento(Box::new(
                Document::from_json(t).map_err(|e| ErroApi::novo("leitura", e.to_string()))?,
            )));
        }
        Ok(s)
    }

    /// Executa um comando da lista. Só o que deu certo e muda o modelo entra no registro.
    pub fn executar(&mut self, comando: &str, args: &Value) -> Result<Value, ErroApi> {
        let args = if args.is_null() { &json!({}) } else { args };
        if !args.is_object() {
            return Err(ErroApi::argumento("`args` deve ser um objeto"));
        }
        let resultado = match comando {
            "versao" => Ok(
                json!({ "api": API_VERSAO, "programa": env!("CARGO_PKG_VERSION"), "kernel": fenkra_kernel::AVAILABLE }),
            ),
            "comandos" => Ok(json!({ "comandos": COMANDOS })),
            "nova_peca" => self.nova_peca(args),
            "abrir" => self.abrir(args),
            "salvar" => self.salvar(args),
            "exportar" => self.exportar(args),
            "arvore" => self.arvore(),
            "relatorio" => self.relatorio(),
            "parametros" => self.parametros(),
            "definir_parametro" => self.definir_parametro(args),
            "criar_plano" => self.criar_plano(args),
            "criar_esboco" => self.adicionar("esboco", "esboco", "Esboço", args),
            "extrudar" => self.adicionar("extrusao", "extrusao", "Extrusão", args),
            "furar" => self.adicionar(
                "furo",
                "furo",
                "Furo",
                &com_padrao(args, "forma", json!({ "tipo": "simples" })),
            ),
            "filetar" => self.adicionar("filete", "filete", "Filete", args),
            "chanfrar" => self.adicionar("chanfro", "chanfro", "Chanfro", args),
            "desfazer" => self.desfazer(),
            "recalcular" => self.recalcular(),
            "medir" => self.medir(),
            "ver" => self.ver(args),
            "topologia" => self.topologia(args),
            "features" => self.features(args),
            "comparar" => self.comparar(args),
            "reproduzir" => self.reproduzir(args),
            "registro" => Ok(self.registro_json()),
            "adicionar" => self.adicionar_tipo(args),
            "selecionar" => self.selecionar(args),
            "biblioteca" => crate::biblioteca::consultar(args),
            "verificar" => self.verificar(args),
            _ => Err(ErroApi::novo(
                "comando_desconhecido",
                format!("comando '{comando}' não existe; veja `comandos`"),
            )),
        }?;
        if self.registrar && COMANDOS_DE_MODELO.contains(&comando) {
            if self.registro.is_none() {
                self.registro = Some(Registro::novo(&self.pasta)?);
            }
            if let Some(r) = &mut self.registro {
                r.anotar(comando, args)?;
            }
        }
        Ok(resultado)
    }

    /// Texto do que foi registrado até agora, para o recurso do servidor.
    pub fn registro_json(&self) -> Value {
        match &self.registro {
            Some(r) => json!({ "arquivo": r.caminho(), "comandos": r.linhas() }),
            None => json!({ "arquivo": null, "comandos": [] }),
        }
    }

    // ------------------------------------------------------------------ modelo

    fn trocar(&mut self, novo: Modelo) {
        self.historico.push(self.modelo.take());
        self.modelo = Some(novo);
        self.calculado = None;
    }

    fn peca(&self) -> Result<&Peca, ErroApi> {
        match &self.modelo {
            Some(Modelo::Arvore(p)) => Ok(p),
            Some(Modelo::Documento(_)) => Err(ErroApi::novo("arvore_incompleta", "esta peça tem comandos fora do formato aberto: ela mede, muda parâmetro e exporta, mas não recebe item novo")),
            None => Err(ErroApi::novo("sem_peca", "nenhuma peça aberta; use `nova_peca` ou `abrir`")),
        }
    }

    fn calcular(&mut self) -> Result<&Calculado, ErroApi> {
        if self.calculado.is_none() {
            let doc = match &self.modelo {
                Some(Modelo::Arvore(p)) => {
                    arvore::ler_peca(p)
                        .map_err(|e| ErroApi::novo("arvore", e.to_string()))?
                        .doc
                }
                Some(Modelo::Documento(d)) => (**d).clone(),
                None => {
                    return Err(ErroApi::novo(
                        "sem_peca",
                        "nenhuma peça aberta; use `nova_peca` ou `abrir`",
                    ))
                }
            };
            let r = recompute(&doc);
            self.calculado = Some(Calculado { doc, r });
        }
        Ok(self.calculado.as_ref().expect("acabou de calcular"))
    }

    fn forma(&mut self) -> Result<Shape, ErroApi> {
        let c = self.calcular()?;
        final_shape(&c.doc)
            .ok_or_else(|| ErroApi::novo("sem_solido", "a peça ainda não tem sólido"))
    }

    fn nova_peca(&mut self, args: &Value) -> Result<Value, ErroApi> {
        let nome = args["nome"].as_str().unwrap_or("Peça").to_string();
        self.trocar(Modelo::Arvore(Peca {
            id: "peca".into(),
            nome: nome.clone(),
            parametros: vec![],
            corpos: vec![],
            referencias: vec![],
            arvore: vec![],
            ativos: None,
        }));
        Ok(json!({ "nome": nome }))
    }

    fn abrir(&mut self, args: &Value) -> Result<Value, ErroApi> {
        let arquivo = texto(args, "arquivo")?;
        let conteudo = self.pasta.ler_texto(&arquivo, &LEITURA_PECA)?;
        let (modelo, fora) = if arquivo.to_ascii_lowercase().ends_with(".fkx") {
            let lido =
                arvore::ler_texto(&conteudo).map_err(|e| ErroApi::novo("arvore", e.to_string()))?;
            let fora = lido.fora.clone();
            let [peca] = <[Peca; 1]>::try_from(lido.pecas)
                .map_err(|_| ErroApi::novo("arvore", "este comando abre arquivo de uma peça só"))?;
            (Modelo::Arvore(peca), fora)
        } else {
            let doc = Document::from_json(&conteudo)
                .map_err(|e| ErroApi::novo("leitura", format!("'{arquivo}': {e}")))?;
            if doc.formato_futuro() {
                return Err(ErroApi::novo(
                    "formato",
                    "o arquivo veio de um Fenkra mais novo",
                ));
            }
            let e = arvore::exportar_peca(&doc);
            if e.fora.is_empty() {
                (Modelo::Arvore(e.peca), vec![])
            } else {
                let fora = e.fora.iter().map(|f| f.texto()).collect();
                (Modelo::Documento(Box::new(doc)), fora)
            }
        };
        self.trocar(modelo);
        Ok(
            json!({ "arquivo": arquivo, "editavel": matches!(self.modelo, Some(Modelo::Arvore(_))), "fora": fora }),
        )
    }

    fn arvore(&mut self) -> Result<Value, ErroApi> {
        let peca = self.peca()?;
        serde_json::to_value(arquivo_de(peca)).map_err(|e| ErroApi::novo("arvore", e.to_string()))
    }

    fn salvar(&mut self, args: &Value) -> Result<Value, ErroApi> {
        let arquivo = texto(args, "arquivo")?;
        let sobrescrever = args["sobrescrever"].as_bool().unwrap_or(false);
        let fkx = arquivo.to_ascii_lowercase().ends_with(".fkx");
        let (conteudo, fora) = if fkx {
            match &self.modelo {
                Some(Modelo::Arvore(p)) => (
                    serde_json::to_string_pretty(&arquivo_de(p))
                        .map_err(|e| ErroApi::novo("gravacao", e.to_string()))?,
                    vec![],
                ),
                _ => {
                    let doc = self.calcular()?.doc.clone();
                    let e = arvore::exportar_peca(&doc);
                    let fora: Vec<String> = e.fora.iter().map(|f| f.texto()).collect();
                    (
                        serde_json::to_string_pretty(&e.arquivo())
                            .map_err(|e| ErroApi::novo("gravacao", e.to_string()))?,
                        fora,
                    )
                }
            }
        } else {
            (self.calcular()?.doc.to_json(), vec![])
        };
        self.pasta
            .gravar(&arquivo, &LEITURA_PECA, sobrescrever, |p| {
                std::fs::write(p, &conteudo).map_err(|e| ErroApi::novo("gravacao", e.to_string()))
            })?;
        Ok(json!({ "arquivo": arquivo, "fora": fora }))
    }

    fn exportar(&mut self, args: &Value) -> Result<Value, ErroApi> {
        let arquivo = texto(args, "arquivo")?;
        let sobrescrever = args["sobrescrever"].as_bool().unwrap_or(false);
        let forma = self.forma()?;
        self.pasta
            .gravar(&arquivo, &GRAVACAO_STEP, sobrescrever, |p| {
                let caminho = p
                    .to_str()
                    .ok_or_else(|| ErroApi::novo("gravacao", "caminho com caracteres inválidos"))?;
                Ok(forma.export_step(caminho)?)
            })?;
        Ok(json!({ "arquivo": arquivo }))
    }

    // ------------------------------------------------------------------ parâmetros

    fn parametros(&mut self) -> Result<Value, ErroApi> {
        let c = self.calcular()?;
        let lista: Vec<Value> = c.doc.params.iter().map(|p| {
            let grandeza = match p.unit {
                Unit::Comprimento => "comprimento",
                Unit::Angulo => "angulo",
                Unit::Numero => "numero",
            };
            json!({ "nome": p.name, "expressao": p.expr, "valor": c.r.params.get(&p.name).map(|v| r6(*v)), "grandeza": grandeza, "comentario": p.comment })
        }).collect();
        Ok(json!({ "parametros": lista }))
    }

    fn definir_parametro(&mut self, args: &Value) -> Result<Value, ErroApi> {
        let nome = texto(args, "nome")?;
        let valor = expressao(args, "expressao")?;
        let grandeza: Grandeza = match &args["grandeza"] {
            Value::Null => Grandeza::Comprimento,
            g => serde_json::from_value(g.clone())
                .map_err(|_| ErroApi::argumento("`grandeza` é comprimento, angulo ou numero"))?,
        };
        let novo = match self.modelo.clone() {
            Some(Modelo::Documento(mut d)) => {
                let p = d
                    .params
                    .iter_mut()
                    .find(|p| p.name == nome)
                    .ok_or_else(|| {
                        ErroApi::argumento(format!("parâmetro '{nome}' não existe nesta peça"))
                    })?;
                p.expr = valor.clone();
                d.eval_params()
                    .map_err(|e| ErroApi::novo("parametro", e.to_string()))?;
                Modelo::Documento(d)
            }
            _ => {
                let mut peca = self.peca()?.clone();
                match peca.parametros.iter_mut().find(|p| p.nome == nome) {
                    Some(p) => p.expressao = valor.clone(),
                    None => peca.parametros.push(Parametro {
                        nome: nome.clone(),
                        expressao: valor.clone(),
                        grandeza,
                        comentario: String::new(),
                    }),
                }
                let v = serde_json::to_value(arquivo_de(&peca))
                    .map_err(|e| ErroApi::novo("arvore", e.to_string()))?;
                let peca = validar(&v)?;
                let doc = arvore::ler_peca(&peca)
                    .map_err(|e| ErroApi::novo("arvore", e.to_string()))?
                    .doc;
                doc.eval_params()
                    .map_err(|e| ErroApi::novo("parametro", e.to_string()))?;
                Modelo::Arvore(peca)
            }
        };
        self.trocar(novo);
        let c = self.calcular()?;
        Ok(
            json!({ "nome": nome, "expressao": valor, "valor": c.r.params.get(&nome).map(|v| r6(*v)) }),
        )
    }

    // ------------------------------------------------------------------ árvore

    fn id_livre(peca: &Peca, base: &str) -> String {
        (1..)
            .map(|n| format!("{base}{n}"))
            .find(|id| {
                !peca.arvore.iter().any(|i| i.id == *id)
                    && !peca.referencias.iter().any(|r| r.id == *id)
            })
            .expect("sempre há número livre")
    }

    fn adicionar(
        &mut self,
        tipo: &str,
        base: &str,
        nome_padrao: &str,
        args: &Value,
    ) -> Result<Value, ErroApi> {
        let peca = self.peca()?;
        let mut comando = args.as_object().cloned().unwrap_or_default();
        let id = comando
            .remove("id")
            .and_then(|v| v.as_str().map(str::to_string))
            .unwrap_or_else(|| Self::id_livre(peca, base));
        let nome = comando
            .remove("nome")
            .and_then(|v| v.as_str().map(str::to_string))
            .unwrap_or_else(|| format!("{nome_padrao} {}", peca.arvore.len() + 1));
        comando.insert("tipo".into(), json!(tipo));
        let mut v = serde_json::to_value(arquivo_de(peca))
            .map_err(|e| ErroApi::novo("arvore", e.to_string()))?;
        v["pecas"][0]["arvore"]
            .as_array_mut()
            .expect("a árvore é uma lista")
            .push(json!({ "id": id, "nome": nome, "comando": comando }));
        let nova = validar(&v)?;
        // recusa o que não monta (id de esboço errado, por exemplo) sem tocar no que já existe
        arvore::ler_peca(&nova).map_err(|e| ErroApi::novo("arvore", e.to_string()))?;
        self.trocar(Modelo::Arvore(nova));
        let c = self.calcular()?;
        let ultimo = c.doc.features.last();
        let mut saida = status_json(ultimo.and_then(|f| c.r.status.get(&f.id)));
        if let Some((f, Status::Error(m))) = ultimo.and_then(|f| c.r.status.get(&f.id).map(|s| (f, s))) {
            juntar(&mut saida, fenkra_solidos::erro_acionavel::diagnosticar(c.r.tipos_de_erro(&c.doc).get(&f.id).copied(), &f.name, m).json());
        }
        saida["id"] = json!(id);
        saida["volume"] = json!(r6(c.r.bodies.iter().map(|b| b.mass.volume).sum()));
        if tipo == "esboco" {
            if let Some(FeatureKind::Sketch(s)) = c.doc.features.last().map(|f| &f.kind) {
                let vars = c.doc.eval_params().unwrap_or_default();
                let rep = s.data.clone().solve(&vars, &[]);
                saida["graus_de_liberdade"] = json!(rep.dof);
                saida["convergiu"] = json!(rep.converged);
            }
        }
        Ok(saida)
    }

    /// Qualquer comando do formato da árvore pelo `tipo` (B420): revolução, casca, primitiva, padrão,
    /// espelho e o que o esquema ganhar depois. O esquema publicado valida o resto.
    fn adicionar_tipo(&mut self, args: &Value) -> Result<Value, ErroApi> {
        let tipo = texto(args, "tipo")?;
        if !tipos_do_esquema().contains(&tipo) {
            return Err(ErroApi::argumento(format!("tipo '{tipo}' não existe no formato da árvore; use {}", tipos_do_esquema().join(", "))));
        }
        let mut resto = args.clone();
        if let Some(o) = resto.as_object_mut() {
            o.remove("tipo");
        }
        let nome = rotulo_do_tipo(&tipo);
        self.adicionar(&tipo, &tipo, &nome, &resto)
    }

    /// Requisitos da peça (B418): os declarados nela e os que vierem em `requisitos`, um resultado
    /// por requisito. Não muda a peça.
    fn verificar(&mut self, args: &Value) -> Result<Value, ErroApi> {
        let extra: Vec<fenkra_core::requisitos::Requisito> = match &args["requisitos"] {
            Value::Null => vec![],
            v => serde_json::from_value(v.clone()).map_err(|e| ErroApi::argumento(format!("`requisitos`: {e}")))?,
        };
        let doc = self.calcular()?.doc.clone();
        let mut lista = doc.requisitos.clone();
        lista.extend(extra);
        if lista.is_empty() {
            return Err(ErroApi::argumento("a peça não declara requisitos; passe `requisitos` (massa, furos, folga, sem_interferencia, caixa, propagacao)"));
        }
        let res = fenkra_solidos::verificar::verificar(&doc, &lista);
        let falhas = res.iter().filter(|r| !r.ok).count();
        Ok(json!({
            "ok": falhas == 0,
            "falhas": falhas,
            "resultados": res.iter().map(|r| json!({ "nome": r.nome, "tipo": r.tipo, "ok": r.ok, "medido": r.medido, "detalhe": r.detalhe })).collect::<Vec<_>>(),
        }))
    }

    /// Faces ou arestas por lado, tipo e raio, devolvidas como pontos para filete, chanfro e casca.
    fn selecionar(&mut self, args: &Value) -> Result<Value, ErroApi> {
        let forma = self.forma()?;
        let t = topologia::analisar(&forma)?.json(&["faces", "arestas"], usize::MAX);
        crate::selecao::selecionar(&t, args)
    }

    fn criar_plano(&mut self, args: &Value) -> Result<Value, ErroApi> {
        let peca = self.peca()?;
        let id = args["id"]
            .as_str()
            .map(str::to_string)
            .unwrap_or_else(|| Self::id_livre(peca, "plano"));
        let nome = args["nome"]
            .as_str()
            .map(str::to_string)
            .unwrap_or_else(|| format!("Plano {}", peca.referencias.len() + 1));
        let base = if args["base"].is_null() {
            json!("XY")
        } else {
            args["base"].clone()
        };
        let mut v = serde_json::to_value(arquivo_de(peca))
            .map_err(|e| ErroApi::novo("arvore", e.to_string()))?;
        let referencia = json!({ "id": id, "nome": nome, "definicao": { "tipo": "plano_deslocado", "base": base, "deslocamento": expressao(args, "deslocamento")? } });
        match v["pecas"][0]["referencias"].as_array_mut() {
            Some(l) => l.push(referencia),
            None => v["pecas"][0]["referencias"] = json!([referencia]),
        }
        let nova = validar(&v)?;
        arvore::ler_peca(&nova).map_err(|e| ErroApi::novo("arvore", e.to_string()))?;
        self.trocar(Modelo::Arvore(nova));
        Ok(json!({ "id": id }))
    }

    fn desfazer(&mut self) -> Result<Value, ErroApi> {
        let anterior = self
            .historico
            .pop()
            .ok_or_else(|| ErroApi::novo("sem_historico", "nada para desfazer"))?;
        self.modelo = anterior;
        self.calculado = None;
        Ok(
            json!({ "itens": self.modelo.as_ref().map_or(0, |m| if let Modelo::Arvore(p) = m { p.arvore.len() } else { 0 }) }),
        )
    }

    // ------------------------------------------------------------------ cálculo e medidas

    fn recalcular(&mut self) -> Result<Value, ErroApi> {
        let ids: Vec<String> = match &self.modelo {
            Some(Modelo::Arvore(p)) => p.arvore.iter().map(|i| i.id.clone()).collect(),
            _ => vec![],
        };
        let c = self.calcular()?;
        let mut erro = c.r.param_error.is_some();
        let vars = c.doc.eval_params().unwrap_or_default();
        let tipos = c.r.tipos_de_erro(&c.doc);
        let itens: Vec<Value> = c
            .doc
            .features
            .iter()
            .enumerate()
            .map(|(i, f)| {
                let s = c.r.status.get(&f.id);
                erro |= matches!(s, Some(Status::Error(_)));
                let mut j = status_json(s);
                if let Some(Status::Error(m)) = s {
                    juntar(&mut j, fenkra_solidos::erro_acionavel::diagnosticar(tipos.get(&f.id).copied(), &f.name, m).json());
                }
                j["id"] = json!(ids.get(i).cloned().unwrap_or_else(|| f.id.0.to_string()));
                j["nome"] = json!(f.name);
                if let FeatureKind::Sketch(k) = &f.kind {
                    j["graus_de_liberdade"] = json!(k.data.clone().solve(&vars, &[]).dof);
                }
                j
            })
            .collect();
        let volume: f64 = c.r.bodies.iter().map(|b| b.mass.volume).sum();
        let area: f64 = c.r.bodies.iter().map(|b| b.mass.area).sum();
        let faces: u32 = c.r.bodies.iter().map(|b| b.faces).sum();
        let mut saida = json!({ "status": if erro { "erro" } else { "ok" }, "itens": itens, "corpos": c.r.bodies.len(), "volume": r6(volume), "area": r6(area), "faces": faces });
        if let Some(m) = &c.r.param_error {
            saida["erro_de_parametro"] = json!(m);
        }
        if let Some((lo, hi)) = final_shape(&c.doc).and_then(|s| s.bbox()) {
            saida["caixa"] = json!({ "min": r6v(lo), "max": r6v(hi) });
        }
        Ok(saida)
    }

    fn medir(&mut self) -> Result<Value, ErroApi> {
        let forma = self.forma()?;
        let resumo = topologia::analisar(&forma)?;
        Ok(resumo.json(&["caixa", "massa"], 0))
    }

    /// `ver` (B417): a imagem da peça, por vista, com corte e zoom. Sem `saida`, a imagem volta em
    /// `imagem_base64` (é o que o MCP entrega ao agente como imagem); com `saida`, grava o PNG.
    fn ver(&mut self, args: &Value) -> Result<Value, ErroApi> {
        let pedido = crate::ver::pedido_do_json(args).map_err(ErroApi::argumento)?;
        let forma = self.forma()?;
        let (lo, hi) = forma.bbox().ok_or_else(|| ErroApi::novo("sem_solido", "a peça ainda não tem sólido"))?;
        let diag = ((hi[0] - lo[0]).powi(2) + (hi[1] - lo[1]).powi(2) + (hi[2] - lo[2]).powi(2)).sqrt();
        let malha = forma.mesh_angle((diag * 0.0015).max(0.005), 0.3)?;
        let m = crate::ver::Malha { posicoes: &malha.positions, normais: &malha.normals, indices: &malha.indices, arestas: &malha.edges };
        let imagem = crate::ver::renderizar(&m, &pedido);
        let bytes = crate::ver::png(&imagem).map_err(|e| ErroApi::novo("gravacao", e))?;
        let mut saida = crate::ver::descricao(&pedido, &imagem);
        saida["caixa"] = json!({ "min": r6v(lo), "max": r6v(hi) });
        if let Some(c) = pedido.corte {
            let (a, b) = (lo[c.eixo], hi[c.eixo]);
            if c.posicao <= a || c.posicao >= b {
                saida["aviso"] = json!(format!("o plano do corte ({} mm) fica fora da peça, que vai de {} a {} mm nesse eixo", c.posicao, r6(a), r6(b)));
            }
        }
        match args["saida"].as_str() {
            Some(arq) => {
                let sobrescrever = args["sobrescrever"].as_bool().unwrap_or(false);
                self.pasta.gravar(arq, &["png"], sobrescrever, |p| std::fs::write(p, &bytes).map_err(|e| ErroApi::novo("gravacao", e.to_string())))?;
                saida["arquivo"] = json!(arq);
            }
            None => {
                use base64::Engine;
                saida["imagem_base64"] = json!(base64::engine::general_purpose::STANDARD.encode(&bytes));
                saida["tipo"] = json!("image/png");
            }
        }
        Ok(saida)
    }

    fn relatorio(&mut self) -> Result<Value, ErroApi> {
        let mut r = self.recalcular()?;
        r["comparacao"] = self.ultima_comparacao.clone().unwrap_or(Value::Null);
        r["registro"] = self.registro_json()["arquivo"].clone();
        Ok(r)
    }

    /// Roda no filho, levando a peça atual se houver.
    fn no_filho(
        &mut self,
        programa: PathBuf,
        comando: &str,
        args: &Value,
    ) -> Result<Value, ErroApi> {
        let mut pedido = json!({ "comando": comando, "args": args, "pasta": self.pasta.raiz().to_string_lossy() });
        match &self.modelo {
            Some(Modelo::Arvore(p)) => {
                pedido["peca"] = json!(serde_json::to_string(&arquivo_de(p))
                    .map_err(|e| ErroApi::novo("arvore", e.to_string()))?)
            }
            Some(Modelo::Documento(d)) => pedido["documento"] = json!(d.to_json()),
            None => {}
        }
        filho::rodar(&programa, pedido, filho::LIMITE)
    }

    /// Forma de um arquivo da pasta: STEP direto, peça pelo cálculo.
    fn forma_do_arquivo(&self, arquivo: &str) -> Result<Shape, ErroApi> {
        let caminho = self.pasta.para_ler(arquivo, &LEITURA_ALVO)?;
        let minusculo = arquivo.to_ascii_lowercase();
        if minusculo.ends_with(".step") || minusculo.ends_with(".stp") {
            let texto = caminho
                .to_str()
                .ok_or_else(|| ErroApi::novo("leitura", "caminho com caracteres inválidos"))?;
            return Ok(Shape::import_step(texto)?);
        }
        let mut outra = Sessao::nova(self.pasta.clone(), Isolamento::Local);
        outra.abrir(&json!({ "arquivo": arquivo }))?;
        outra.forma()
    }

    fn topologia(&mut self, args: &Value) -> Result<Value, ErroApi> {
        if let (Isolamento::Processo(p), false) = (&self.isolamento, args["arquivo"].is_null()) {
            return self.no_filho(p.clone(), "topologia", args);
        }
        let secoes: Vec<String> = match &args["secoes"] {
            Value::Null => topologia::SECOES.iter().map(|s| s.to_string()).collect(),
            Value::Array(l) => l
                .iter()
                .map(|s| {
                    s.as_str()
                        .map(str::to_string)
                        .ok_or_else(|| ErroApi::argumento("`secoes` é uma lista de textos"))
                })
                .collect::<Result<_, _>>()?,
            _ => return Err(ErroApi::argumento("`secoes` é uma lista de textos")),
        };
        if let Some(s) = secoes
            .iter()
            .find(|s| !topologia::SECOES.contains(&s.as_str()))
        {
            return Err(ErroApi::argumento(format!(
                "seção desconhecida: '{s}'; use {}",
                topologia::SECOES.join(", ")
            )));
        }
        let forma = match args["arquivo"].as_str() {
            Some(a) => self.forma_do_arquivo(a)?,
            None => self.forma()?,
        };
        let limite = args["limite"]
            .as_u64()
            .map_or(topologia::LIMITE_PADRAO, |n| n as usize);
        let referencias: Vec<&str> = secoes.iter().map(String::as_str).collect();
        Ok(topologia::analisar(&forma)?.json(&referencias, limite))
    }

    fn features(&mut self, args: &Value) -> Result<Value, ErroApi> {
        if let (Isolamento::Processo(p), false) = (&self.isolamento, args["arquivo"].is_null()) {
            return self.no_filho(p.clone(), "features", args);
        }
        let forma = match args["arquivo"].as_str() {
            Some(a) => self.forma_do_arquivo(a)?,
            None => self.forma()?,
        };
        Ok(crate::features::features(&forma))
    }

    fn comparar(&mut self, args: &Value) -> Result<Value, ErroApi> {
        let t = Tolerancias::de_json(&args["tolerancias"])?;
        if let Isolamento::Processo(p) = &self.isolamento {
            let r = self.no_filho(p.clone(), "comparar", args)?;
            self.ultima_comparacao = Some(r.clone());
            return Ok(r);
        }
        let alvo: Resumo = topologia::analisar(&self.forma_do_arquivo(&texto(args, "alvo")?)?)?;
        let forma = match args["arquivo"].as_str() {
            Some(a) => self.forma_do_arquivo(a)?,
            None => self.forma()?,
        };
        let r = checador::comparar(&alvo, &topologia::analisar(&forma)?, &t);
        self.ultima_comparacao = Some(r.clone());
        Ok(r)
    }

    /// Refaz um registro nesta sessão. Só os comandos que mudam o modelo são aceitos: o arquivo é dado,
    /// e um registro alterado à mão não ganha nenhum poder além do que um agente já tem.
    fn reproduzir(&mut self, args: &Value) -> Result<Value, ErroApi> {
        let arquivo = texto(args, "arquivo")?;
        let linhas = ler_comandos(&self.pasta.ler_texto(&arquivo, &["jsonl"])?)?;
        if let Some((c, _)) = linhas
            .iter()
            .find(|(c, _)| !COMANDOS_DE_MODELO.contains(&c.as_str()))
        {
            return Err(ErroApi::novo(
                "formato",
                format!("o registro traz '{c}', que não é comando de modelo"),
            ));
        }
        for (comando, a) in &linhas {
            self.executar(comando, a)?;
        }
        Ok(json!({ "comandos": linhas.len() }))
    }
}

fn com_padrao(args: &Value, chave: &str, padrao: Value) -> Value {
    let mut v = args.clone();
    if v[chave].is_null() {
        v[chave] = padrao;
    }
    v
}

/// Os `tipo` de comando que o esquema publicado da árvore aceita, na ordem do esquema.
pub fn tipos_do_esquema() -> Vec<String> {
    let e: Value = serde_json::from_str(arvore::ESQUEMA).unwrap_or_default();
    e["$defs"]["comando"]["oneOf"]
        .as_array()
        .map(|l| {
            l.iter()
                .map(|v| match v["$ref"].as_str() {
                    Some(r) => e["$defs"][r.trim_start_matches("#/$defs/")]["properties"]["tipo"]["const"].clone(),
                    None => v["properties"]["tipo"]["const"].clone(),
                })
                .filter_map(|t| t.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// Nome do item na árvore quando o pedido não traz um: "casca" vira "Casca".
fn rotulo_do_tipo(tipo: &str) -> String {
    match tipo {
        "revolucao" => "Revolução".into(),
        "padrao" => "Padrão".into(),
        "esboco" => "Esboço".into(),
        "extrusao" => "Extrusão".into(),
        _ => {
            let mut c = tipo.chars();
            c.next().map(|p| p.to_uppercase().chain(c).collect()).unwrap_or_default()
        }
    }
}
