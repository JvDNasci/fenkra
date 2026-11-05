# ADR 0029 — Configurações em camadas, da aplicação ao documento

## Contexto

Todas as opções moravam num `config.json` do usuário, sem distinção entre o que é da pessoa, da máquina, do
projeto ou do documento. Uma empresa não conseguia fixar a norma de cotagem; duas pessoas abriam a mesma peça
com cotas diferentes; um `config.json` corrompido era descartado em silêncio.

## Decisão

- **Camadas** (`fenkra_core::config`): programa < empresa < projeto < usuário, e o documento por cima só nas
  opções que são dele. O resolvedor trabalha com mapas JSON, sem conhecer a estrutura de configuração.
- **Trava:** empresa e projeto podem travar chaves; da camada que trava para cima o valor não muda, e vale a
  trava da camada mais baixa. Chave travada nunca é gravada no arquivo do usuário.
- **Classificação:** cada opção de `Settings` tem uma classe (documento, interface, máquina, sessão) em
  `config_camadas.rs`; um teste reprova opção nova sem classe.
- **Documento:** cotagem, grade, anotações 3D e qualidade da malha vão no campo `config` do arquivo da peça
  quando a pessoa liga "valem só para este documento". Ligado, o documento sobrepõe as opções do usuário e o
  `config.json` continua levando as do usuário.
- **Arquivo:** o `config.json` do usuário guarda só o que difere das camadas de baixo. Cada gravação passa por
  arquivo temporário e guarda o anterior em `.bak` (só se ele estava válido). Se a leitura falha, vale o
  backup, o arquivo ruim fica em `.corrompido` e a interface avisa; falha ao gravar também avisa.
- **Perfil de interface:** exporta e importa só as chaves de interface, com formato e versão. Chaves travadas
  e as que não são de interface ficam de fora da importação, e o resumo diz quantas.
- **Onde ficam:** empresa em `%PROGRAMDATA%\Fenkra\empresa.json` (ou `FENKRA_CONFIG_EMPRESA`), projeto em
  `.fenkra\config.json` na raiz do projeto.

## Consequências

- Mudar o padrão do programa chega a quem nunca mexeu na opção (o arquivo não a fixa mais).
- A trava é imposta a cada quadro por comparação, sem ícone de cadeado em cada controle ainda.
