# ADR 0069: visualizador dentro do PLM, aberto por arquivo local, sem servidor (B339)

Data: 2026-11-06. Estado: aceita. Itens: B339, B214 (parte 3D), B130 (prévia lado a lado).

## Contexto

O ADR 0065 criou o pacote `.fkv` e a página `visualizador.html`, que abre por arquivo, por URL (`?src=`)
e por mensagem (`postMessage`), e deixou para o B339 ligar o visualizador ao PLM: pré-visualização
rápida de peça e de montagem sem abrir o Fenkra, comparação de versões, análise zonal e análise de
montagem. O egui não embute página web, então a interface do Fenkra precisa entregar a página a um
navegador de fora. O ADR 0065 também deixou a geração do pacote ao salvar como "avaliada, não ligada".

## Decisão

1. **Abrir por arquivo, não por servidor.** "Ver no navegador" grava uma página autocontida (a página
   do visualizador com o `.fkv` embutido em base64) em `.visualizacao/paginas/` e entrega o caminho ao
   navegador padrão do sistema (`rundll32 url.dll,FileProtocolHandler` no Windows, `open` no macOS,
   `xdg-open` no Linux). Não abre porta, não pede permissão de rede ao sistema e nada de fora lê o
   projeto. O servidor local em `127.0.0.1` foi estudado e **não adotado**: obriga a manter um processo
   vivo e uma porta livre, dispara o aviso do firewall no Windows, e qualquer programa da máquina
   conseguiria ler o que ele serve. Se um dia for preciso (página que busca o `.fkv` por `?src=`
   sem arquivo), ele escuta só em `127.0.0.1`, em porta efêmera, com um segredo aleatório na URL, e
   morre com o Fenkra. Hospedagem em servidor de equipe é item posterior do B339, fora desta decisão.
2. **O pacote é uma opção do projeto.** `.visualizacao.json` na raiz do projeto guarda
   `gerar_ao_salvar` (desligada por padrão, como recomendou o ADR 0065) e `simplificada` (decima a malha e
   tira os pontos de captura exatos). Os pacotes ficam em `.visualizacao/<id>-<hash>.fkv`, **um por
   versão**, nomeados pelo hash da versão. A versão é imutável, então o pacote dela nunca fica velho, e
   "comparar versões" é abrir dois arquivos que já existem. A poda guarda as 6 versões de trabalho mais
   recentes de cada documento e todas as de revisão liberada.
3. **A geração roda em segundo plano, com uma cópia do documento da versão pedida.** O laço da
   interface recolhe o resultado e é ele quem grava no projeto. Pedir para ver uma versão sem pacote
   gera na hora e abre quando fica pronto. A decisão de "o que falta gerar" mora em `fenkra-project`
   (`visualizacao`); quem enxerga o kernel é o aplicativo, porque `fenkra-pacote` fica acima do projeto.
4. **A análise de montagem vai calculada no manifesto** (`analise`): interferências pelo mesmo
   comando da montagem (volume comum) e folgas entre peças que não interferem, até 25 mm. A folga é um
   limite superior pela amostra de vértices medida contra o sólido do outro lado, e o campo `metodo`
   diz isso. Montagem com mais de 150 componentes de topo não é analisada e o pacote registra o motivo.
5. **Zonal, montagem e comparação são feitas na página**, sobre o pacote: nada recalcula. Região do
   produto por caixa e por volume, peças por faixa de um eixo, onde é usado, interferências e folgas,
   e comparação de dois pacotes pelos IDs estáveis (adicionado, removido, movido, alterado).
6. **Dados do PLM entram na página no momento de abrir** (estado, revisão, versão), num `<script
   type="application/json">` com o `<` escapado. Não vão no pacote: o estado muda depois da geração.
7. **Sobreposição colorida e prévia lado a lado no 3D** (B214, B130) usam uma comparação de malha sem
   kernel (`fenkra_project::sobreposicao`): cada triângulo é amostrado e medido contra a malha do outro
   lado por uma grade uniforme; fica "só deste lado" o que está além da tolerância (0,4 % da diagonal, no
   mínimo 0,05 mm). As camadas entram na camada translúcida da cena com cor pelo papel e pelo tema.

## Consequências

- Sem servidor, o `?src=` continua para quem hospeda o `.fkv`; abrir do PLM não depende dele.
- A página autocontida pesa cerca de 700 KB de three.js mais o pacote em base64 (33 % maior).
- Mudança menor que a tolerância não aparece na sobreposição; superfície só espelhada conta como igual.
- A folga por amostra de vértices pode passar um pouco da real entre arestas; entre faces planas ou
  cilindros ela é exata.
- O pacote só vale para a versão gravada: documento aberto e ainda não salvo não tem pacote.
