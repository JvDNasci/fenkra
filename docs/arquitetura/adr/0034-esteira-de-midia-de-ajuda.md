# ADR 0034: esteira de mídia de ajuda gerada pelo próprio app (B257)

## Contexto

O tooltip em camadas (B60, B278) precisa de um GIF curto por comando. Desenhar à mão não escala (cerca de
200 comandos) e fica velho a cada mudança de interface. O app já tem o gancho das variáveis `FENKRA_DEMO*`
(abrem exemplos, fixam tema e não gravam preferências).

## Decisão

1. **Roteiro por comando**, em `docs/media/roteiros/<comando>.json`: caso montado em código
   (`midia/casos.rs`), câmera fixa, recorte e campos animados por ponteiro JSON dentro do comando
   serializado (`/Shell/thickness` de 1 a 8 entre 0,4 s e 2,6 s). O formato é pequeno, versionado e revisável.
2. **Gravação determinística**: `FENKRA_DEMO_GRAVAR=<comando>` roda o roteiro contando quadros (`n / fps`), nunca
   o relógio. A cada quadro o app aplica os valores, espera o recálculo e a cena assentarem, pede a captura ao
   eframe (`Context::request_screenshot`) e só então avança. Janela de tamanho fixo, tema fixo, animações
   desligadas. Os quadros saem recortados e reduzidos a 360x220 em PPM.
3. **Montagem fora do binário** (`scripts/media/montar.py`, só biblioteca padrão): paleta de 64 cores por corte
   pela mediana, quadros de diferença com transparência e recorte, GIF de até 150 KB, `manifesto.json` com hash do
   roteiro e do GIF. Hash FNV-1a de 64 bits (detecta mudança, não é criptográfico), com CRLF contado como LF.
4. **Pacote externo** `ajuda.pack` (cabeçalho `FKAJ`, índice JSON, GIFs concatenados) ao lado do executável,
   lido sob demanda com cache LRU de 16 mídias. O binário não cresce.
5. **Leitor de GIF próprio** (`midia/gif.rs`, cerca de 200 linhas), sem dependência nova: cada quadro vira textura
   só enquanto o botão é pairado e a textura é solta ao sair.
6. **Cobertura**: um teste falha se um roteiro não tem mídia em dia no manifesto (hash), se o GIF passa de
   150 KB, não decodifica com a medida e os quadros do roteiro, ou falta no pacote.

## Consequências

- Mídia nova = um JSON; `montar.py gerar-todos --exe <fenkra>` regera só o que mudou.
- A rosca cosmética não muda o volume; o teste de recálculo dos roteiros abre essa exceção por nome.
- O recorte é fixo em pontos da janela de 1180x700; mudança de layout do painel exige rever os recortes.
- Alternativa descartada: tira de quadros em PNG (um texture grande por mídia, 11 MB em RGBA).
