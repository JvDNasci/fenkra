# ADR 0072: abrir o `.fkv` sem esforço, com HTML como saída padrão (B365)

Data: 2026-11-06. Estado: aceita. Item: B365.

## Contexto

Quem exportava uma visualização recebia um `.fkv` e não conseguia abri-lo: o arquivo é um pacote zip, o
navegador não o conhece, e a página que o lê (`visualizador.html`) não ia no pacote de release.

## Decisão

1. **A saída padrão é a página HTML autocontida** (o visualizador com o pacote em base64 dentro). Abre
   com duplo clique em qualquer navegador, sem rede e sem o Fenkra. O `.fkv` puro vira opção avançada
   do painel (arquivo menor, para hospedar ou para o visualizador solto). Ao gravar, o painel oferece
   "Abrir no navegador"; para um `.fkv` ele gera uma página temporária com o pacote dentro.
2. **O release leva `Fenkra Visualizador.html`**: a mesma página, sem pacote, com three.js embutido. Ela
   é gerada pelo próprio `fenkra.exe --gerar-visualizador <saida>`, porque o modelo em `web/` tem marcas
   (`/*@THREE@*/` etc.) a preencher: copiar o arquivo cru entregaria uma página sem o motor 3D. O
   `release.ps1` falha se alguma marca sobrar. A tela vazia da página explica como abrir um `.fkv`.
3. **A associação do `.fkv` no Windows aponta para o Fenkra, não para o navegador.** Um navegador não
   recebe um arquivo por argumento para entregar a uma página. O comando registrado é
   `fenkra.exe --abrir-fkv "%1"`, que valida o pacote, grava a página temporária e a entrega ao
   navegador padrão, sem subir a janela do programa. O registro vai em `HKCU\Software\Classes` (sem
   administrador), só por clique em Configurações, e o desfazer apaga as duas chaves criadas. O registro
   é feito por `reg.exe`, sem dependência nova.
4. **Celular:** o que se garante é abrir por link. Arquivo local depende do sistema (ver
   `docs/formatos/pacote-visualizacao.md`); a interface diz isso sem rodeios.

## Consequências

Nenhum servidor, porta ou instalador. O `.fkv` só abre por duplo clique em quem associou; em outro
computador, arrastar o arquivo para a página `Fenkra Visualizador.html` resolve.
