<!-- origem: Contexto PLM.txt (contexto do usuário, gerado com assistente de IA) -->

```text
Você está pensando grande, Joao—isso é coisa de quem quer que o FEKRA seja “plataforma”, não só software.
Vamos montar a ideia de um VPM do FEKRA como se fosse um produto real: local hoje, hosteável amanhã.

1. O que o “VPM do FEKRA” precisa ser
Ele não é só um banco de dados.

É um Product Data Backbone que:

Guarda estrutura de produto (BOM, árvore de montagem, instâncias)

Gerencia documentos CAD (peças, montagens, desenhos, versões)

Controla revisões e estados (Em edição, Liberado, Obsoleto)

Sincroniza com o FEKRA CAD (abrir, salvar, atualizar, comparar)

Pode rodar local (single user) ou em servidor (multiuser / cloud)

Pensa nele como:

“O lugar onde o FEKRA pergunta: qual é o produto oficial?”

2. Arquitetura geral (local e hosteável)
Camadas principais
Core de dados (VPM Core)

Banco de dados (SQLite local, depois PostgreSQL/MySQL em servidor)

Modelo de produto, revisões, links, usuários

Storage de arquivos

Local: pastas organizadas + hashes

Hosteável: storage em disco/objeto (tipo S3, MinIO, etc.)

API do VPM

REST/GraphQL ou gRPC

Endpoints para: produtos, revisões, documentos, estrutura, sync

Cliente CAD (FEKRA)

Módulo dentro do FEKRA que fala com o VPM

Pode operar em modo:

Local: VPM embutido (SQLite + filesystem)

Remoto: VPM em servidor (HTTP/gRPC)

3. Modelo de dados (o coração do VPM)
Você precisa de algumas entidades básicas:

Product

ID

Código

Nome

Tipo (peça, montagem, desenho, conjunto lógico)

Estado (In Work, Released, Obsolete)

Configurações/variantes (opcional)

Revision

ID

ProductID

Número de revisão (A, B, C… ou 1, 2, 3…)

Data

Autor

Estado (Em edição, Liberado)

Comentários

Document

ID

Tipo (FEKRAPart, FEKRAProd, FEKRADrw, PDF, STEP, etc.)

Caminho/URL no storage

Hash (para integridade)

Versão de arquivo (se quiser separar de revisão lógica)

Structure (BOM / árvore)

ParentProductID

ChildProductID

Quantidade

Posição (transformação 3D)

Configuração/variante (se aplicável)

Link CAD

Tipo (contextual, publicação, referência externa)

Origem (Product/Document)

Destino (Product/Document)

Metadados (tipo de relação)

User / Role / Permission

Para futuro multiusuário

Começa simples, mas já pensando em crescer.

4. Sincronização FEKRA ↔ VPM (como fazer na prática)
No FEKRA:
Você cria um módulo tipo:

FekraVPMClient

connect(local|remote)

openProduct(id)

saveProduct(id, changes)

sync()

compareLocalVsVPM()

Fluxo típico:
Usuário abre um produto no FEKRA:

FEKRA chama VPM: “me dá a revisão atual do Product X”

VPM devolve:

estrutura

documentos

metadados

Usuário edita geometria:

FEKRA marca o documento como “modificado”

FEKRA sabe qual Product/Revision está sendo alterado

Usuário salva:

FEKRA envia:

novo arquivo (part/prod/drw)

mudanças na estrutura (peças adicionadas/removidas)

VPM:

cria nova revisão ou atualiza a atual (dependendo da política)

atualiza BOM

guarda arquivo no storage

Sincronização:

FEKRA pode pedir:

“me mostra o que mudou entre minha sessão e o VPM”

VPM responde:

diferenças de estrutura

diferenças de revisões

conflitos (se outro usuário mexeu)

5. Local hoje, hosteável amanhã
Modo local (single user)
Banco: SQLite

Storage: pasta FekraVault/

API: pode ser só uma biblioteca interna (sem servidor)

Uso:

Desenvolvedor, freelancer, pequeno escritório

Tudo dentro da máquina

Modo servidor (multiuser / hosteável)
Banco: PostgreSQL ou MySQL

Storage: disco ou objeto (S3/MinIO)

API: servidor HTTP/gRPC

Autenticação: JWT, OAuth, etc.

Uso:

Empresa

Time de engenharia

Integração com ERP/MES

Se você desenhar o VPM como serviço com API, o FEKRA só precisa saber “onde está o VPM”:

localhost → modo local

https://vpm.fekra.com → modo hosteado

6. Estratégia de implementação (sem se perder)
Fase 1 – VPM Local Básico

SQLite

Storage em pasta

Modelo de dados mínimo (Product, Revision, Document, Structure)

Cliente FEKRA embutido

Sem usuários, sem permissões

Fase 2 – VPM Servidor

Migrar para PostgreSQL

Criar API REST/gRPC

Autenticação simples

FEKRA passa a falar via HTTP/gRPC

Fase 3 – PLM de verdade

Permissões, papéis, workflows

Estados de maturidade (In Work, Released, Obsolete)

Configuração/variantes

Integração com outros sistemas

7. O que torna isso “profissional” de verdade
Histórico completo de revisões

Estrutura de produto confiável

Sincronização robusta

Controle de quem mexeu em quê e quando

Possibilidade de trabalhar offline e sincronizar depois

API aberta para outros sistemas (CAM, ERP, etc.)

Se você fizer isso, o FEKRA deixa de ser “só um CAD” e vira plataforma de engenharia.


1. Esquema de banco de dados (VPM do FEKRA)
Tabelas principais
PRODUCT

id (PK, UUID)

code (string, único) — código do produto (ex.: FEK-000123)

name (string)

type (enum: part, assembly, drawing, logical)

description (text)

lifecycle_state (enum: in_work, released, obsolete)

created_at (timestamp)

updated_at (timestamp)

REVISION

id (PK, UUID)

product_id (FK → PRODUCT.id)

revision_code (string, ex.: A, B, C ou 1, 2, 3)

status (enum: draft, in_work, released, obsolete)

author_id (FK → USER.id)

created_at (timestamp)

released_at (timestamp, nullable)

comment (text)

DOCUMENT

id (PK, UUID)

product_id (FK → PRODUCT.id)

revision_id (FK → REVISION.id)

doc_type (enum: fekra_part, fekra_assembly, fekra_drawing, step, pdf, etc.)

storage_path (string) — caminho/URL no vault

hash (string) — integridade

size_bytes (bigint)

created_at (timestamp)

STRUCTURE (BOM / árvore de montagem)

id (PK, UUID)

parent_product_id (FK → PRODUCT.id)

parent_revision_id (FK → REVISION.id)

child_product_id (FK → PRODUCT.id)

child_revision_id (FK → REVISION.id, opcional)

quantity (numeric)

transform_matrix (JSON ou 16 floats) — posição/orientação na montagem

config_variant (string, opcional)

created_at (timestamp)

CAD_LINK

id (PK, UUID)

source_document_id (FK → DOCUMENT.id)

target_document_id (FK → DOCUMENT.id)

link_type (enum: contextual, publication, external_ref)

metadata (JSON)

created_at (timestamp)

USER

id (PK, UUID)

username (string, único)

name (string)

email (string)

role (enum: admin, engineer, viewer)

created_at (timestamp)

CHANGE_LOG (audit)

id (PK, UUID)

entity_type (string: product, revision, document, structure)

entity_id (UUID)

user_id (FK → USER.id)

action (string: create, update, delete, release)

payload (JSON)

created_at (timestamp)

2. Formato de API (REST) para o VPM
Convenção geral
Base: /api/v1

Autenticação: Bearer token (JWT)

Respostas: JSON

Erros: { "error": "message", "code": "ERROR_CODE" }

Endpoints principais
Produtos

GET /products

Query: code, type, state

Retorna lista de produtos

GET /products/{id}

Retorna produto + revisões básicas

POST /products

Payload:

json
{
  "code": "FEK-000123",
  "name": "Engrenagem 24 dentes",
  "type": "part",
  "description": "Engrenagem reta módulo 2",
  "lifecycle_state": "in_work"
}
PATCH /products/{id}

Atualiza nome, descrição, estado etc.

Revisões

GET /products/{id}/revisions

GET /revisions/{id}

POST /products/{id}/revisions

Cria nova revisão:

json
{
  "revision_code": "B",
  "status": "in_work",
  "author_id": "..."
}
POST /revisions/{id}/release

Muda status para released

Documentos

GET /products/{id}/documents

GET /revisions/{id}/documents

GET /documents/{id}

POST /documents

Upload via multipart ou URL:

json
{
  "product_id": "...",
  "revision_id": "...",
  "doc_type": "fekra_part",
  "storage_path": "vault/FEK-000123_A.fkpart",
  "hash": "..."
}
Estrutura (BOM)

GET /products/{id}/structure

Retorna árvore:

json
{
  "product_id": "...",
  "revision_id": "...",
  "children": [
    {
      "child_product_id": "...",
      "child_revision_id": "...",
      "quantity": 2,
      "transform_matrix": [...],
      "config_variant": null
    }
  ]
}
POST /structure

Cria relação pai-filho:

json
{
  "parent_product_id": "...",
  "parent_revision_id": "...",
  "child_product_id": "...",
  "child_revision_id": "...",
  "quantity": 1,
  "transform_matrix": [1,0,0,0, 0,1,0,0, 0,0,1,0, 0,0,0,1]
}
Sincronização com FEKRA

POST /sync/session

FEKRA abre sessão de edição:

json
{
  "product_id": "...",
  "revision_id": "...",
  "client_id": "FEKRA-LOCAL-01"
}
POST /sync/changes

FEKRA envia mudanças:

json
{
  "session_id": "...",
  "changes": [
    {
      "entity_type": "structure",
      "action": "add",
      "payload": { ... }
    },
    {
      "entity_type": "document",
      "action": "update",
      "payload": { ... }
    }
  ]
}
GET /sync/diff?product_id=...&revision_id=...

VPM retorna diferenças entre estado oficial e estado local (se você quiser comparar).

3. Fluxo de sincronização detalhado FEKRA ↔ VPM
Cenário: usuário abre, edita e salva uma montagem
1. Abrir produto
FEKRA chama:

GET /products/{id}

GET /products/{id}/revisions (pega a ativa)

GET /products/{id}/structure

GET /revisions/{id}/documents

FEKRA monta:

árvore de montagem

carrega arquivos do vault (via storage_path)

2. Iniciar sessão de edição
FEKRA chama:

POST /sync/session

VPM cria registro de sessão (opcional, mas bom pra multiusuário).

3. Usuário altera geometria e estrutura
FEKRA:

marca documentos como “dirty”

registra alterações na estrutura (peças adicionadas/removidas, transformações)

4. Salvar
FEKRA:

faz upload dos arquivos alterados (para o storage)

chama POST /documents para registrar novos arquivos

chama POST /sync/changes com:

novas relações de estrutura

remoções

atualizações

VPM:

valida:

se revisão está em estado editável

se não há conflito com outra sessão

aplica mudanças:

atualiza STRUCTURE

registra DOCUMENT

grava CHANGE_LOG

5. Opcional: criar nova revisão
FEKRA pode oferecer:

“Salvar como nova revisão”

FEKRA chama:

POST /products/{id}/revisions

VPM:

cria REVISION B

copia estrutura da revisão anterior

associa novos documentos

4. Stack técnica sugerida
Backend (VPM Server)
Linguagem:

Go (ótimo para serviços, rápido, simples)

ou Rust (mais hardcore, mas muito robusto)

ou Node.js (se você quiser algo mais rápido de iterar)

Minha preferência aqui: Go para um PLM/VPM.

Framework:

Go: Gin ou Echo para REST

Node: NestJS

Rust: Axum ou Actix

Banco de dados:

PostgreSQL (melhor escolha geral)

Local: SQLite (modo embutido)

Storage:

Local: sistema de arquivos (vault/ com subpastas por produto/revisão)

Hosteável:

MinIO (S3 compatível self-hosted)

ou S3 real (AWS) se for nuvem

Autenticação:

JWT

Possível integração futura com OAuth2/OpenID

Cliente (FEKRA CAD)
Linguagem: a mesma que você já usa no FEKRA (C++? C#? Rust?)

Criar um módulo:

FekraVpmClient

responsável por:

chamadas HTTP/gRPC

cache local

mapeamento Product/Revision/Document ↔ arquivos abertos

Protocolo:

REST (HTTP + JSON) para começar

gRPC se você quiser mais performance depois

Se você quiser, no próximo passo posso:

desenhar diagramas de sequência da sincronização

propor estrutura de pastas do vault

sugerir estratégia de versionamento de arquivos (hash, delta, etc.)

E também: conectar isso com aquele assistente de engrenagens que você quer—porque ele pode falar direto com o VPM para registrar cada engrenagem gerada como um Product paramétrico.
```
