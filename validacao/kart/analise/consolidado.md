# Análise consolidada do kart (B353)

Cinco especialistas olharam as capturas da sua área e um sexto pesquisou tendências. Detalhes nos arquivos
desta pasta. Nada foi testado com mouse real.

## Veredito

O núcleo do Fenkra aguentou um produto inteiro: 70 peças, 7 subconjuntos, montagem de topo, desenhos, PMI e
PLM. O teste, porém, foi mais fraco do que o "ok" sugere: a modelagem foi feita em código, sem passar pela
interface nem pelo MCP, então ele prova o motor e ainda não mede intuitividade.

## Por bancada

| Bancada | Deu certo | Falhou | Dá para melhorar |
|---|---|---|---|
| Estrutural | Tubos dobrados limpos | Barra da bancada nunca usada; K-02 e K-09 | Sem captura de lista de corte nem de solda |
| Chapa | Dobras e furos parecem a peça real; painel de Aba é bom | Nenhuma imagem da chapa plana | Botão Planificar visível |
| Sólidos | Peças de usinagem com histórico | Aro com status ok e sólido possivelmente inválido (K-10) | Verificar validade a cada comando |
| Superfícies e Forma livre | Pneu e volante reconhecíveis | Abrem na bancada errada; varredura em spline (K-08) | Bico e pontão maciços; conversão lenta |
| Sketch | Esqueleto funciona | Quase nada visto: sem restrições nem graus de liberdade | Esqueleto ilegível, 89 publicações em lista plana |
| Montagem | Direção com juntas e tirantes no 3D | 1 restrição no kart inteiro; 9 itens fora dos subconjuntos; 0 fixadores | Separar fixado de restringido |
| Transmissão | Corrente pelo caminho das rodas | Coroa e pinhão sem dentes abertos; corrente invertida | Gerador aceitar a 219 |
| Desenho | Corte com hachura e planificada com tabela | Nenhuma cota; tabelas sobre o carimbo em 4 de 8 folhas | Escala automática e avisos |
| PMI e PLM | Balões casam com a árvore; revisão e pacote saíram | Legenda e nota sobrepostas | Progresso em tarefas longas |
| Interface | Assistente de reparo e painel com prévia translúcida | Pintura não chega na montagem (K-25) | Rótulos poluem, enquadramento, cores escuras |

## Bug novo mais sério

A massa da legenda e das propriedades usa uma lista antiga de materiais. A coroa em Alumínio 7075 aparece 2,8
vezes mais pesada, sem aviso (B414).

## Falhas do próprio teste

- Algumas legendas prometem o que a imagem não mostra (`0003`, `0019`, `0063`). Não usar no vídeo.
- A matriz de comandos conta chamadas, não uso real.
- Muitos achados de interferência são erro de posição do agente.

## Tendências de IA

Os ganhos medidos vêm do ciclo gerar, medir e corrigir, não do texto direto para CAD. Autonomia total ainda é
promessa, e a conferência por regras é onde há ganho no curto prazo. O caminho recomendado é não embutir IA
própria agora e fazer do Fenkra o melhor lugar para um agente externo trabalhar e provar que acertou: validade
no status, ferramenta `ver`, requisitos com `verificar`, erros acionáveis e MCP ampliado por ondas. A evidência
tem limites: só 4 das 23 fontes foram conferidas no texto do artigo, e parte dos números vem dos próprios
autores.

## O que mudou no processo

- Regras novas nas skills das bancadas, na seção "Lições do kart".
- Backlog: B379 a B412 (bugs do kart) e B413 a B431 (análise). Inclui a segunda rodada do kart pela interface
  (B416).
