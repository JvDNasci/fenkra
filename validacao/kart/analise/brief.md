# Brief da análise pós-kart

Responda em português do Brasil. Você analisa o que o projeto-teste do kart produziu, com olhar crítico.

Fontes, em `C:\Users\João\Projetos\projeto-cad\validacao\kart\`:
- `diario/` e `diario/diario.jsonl`: 138 capturas com peça, bancada, etapa e legenda.
- `relatorio.md`, `resultados.md`, `bugs.md`.

Leia a skill da sua bancada em `.claude/skills/fenkra-<bancada>/SKILL.md`. Olhe de verdade as imagens da sua área, abrindo cada arquivo. Se forem muitas quase iguais (`_pintado`, `_colorido`), abra as que comparam algo novo.

Atenção: a modelagem foi feita em código, não pela interface, e parte da montagem usa posições gravadas à mão. Separe defeito do produto de limite do teste. "ok" no relatório quer dizer que calculou, não que a peça está boa: confira na imagem se ela parece a peça real.

Escreva `validacao/kart/analise/<seu-arquivo>.md`, curto e direto, com:
1. **O que foi usado e o que não foi** na bancada, e por quê.
2. **O que deu certo:** com o arquivo da imagem.
3. **O que falhou:** com evidência.
4. **Deu certo, mas dá para melhorar:** clareza, visual, quantos passos, nomes.
5. **Melhores imagens para o vídeo:** até 6, com arquivo e uma frase do que mostrar.
6. **Regras para a skill da bancada:** aprendizados que valem para sempre, no máximo 5, em uma linha cada.
7. **Itens novos de backlog:** só o que não está em `bugs.md`, com título e prioridade.

Regras: não cite marcas proprietárias (escreva "softwares de modelagem CAD avançados"). Não invente: se não viu, diga que não viu. Não edite código, skills nem backlog; escreva só o seu arquivo. Sem commit. Seja econômico: cota apertada.

Relatório final: até 10 linhas, com os três achados mais importantes.
