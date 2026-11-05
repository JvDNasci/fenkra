<!-- origem: Smart fastener.txt (contexto do usuário, gerado com assistente de IA) -->

```text
isão geral: biblioteca de normas x fixadores inteligentes
biblioteca de normas:  
Biblioteca de componentes padrão (parafusos, porcas, arruelas, pinos etc.), com normas como ANSI Métrico, Polegadas, ISO, DIN etc.   
Esses componentes são totalmente parametrizados (diâmetro, comprimento, tipo de rosca, cabeça, material, etc.).

fixadores inteligentes:  
Funcionalidade de montagem que usa a biblioteca de normas para preencher automaticamente furos com fixadores (parafusos, parafusos com porca, arruelas etc.), com base nas dimensões e no tipo de furo. 

Em resumo: o biblioteca de normas é o “estoque” de ferragens; o fixadores inteligentes é o “cara” que olha os furos e escolhe o que colocar lá.

Pré-requisitos para funcionar
Suplemento biblioteca de normas ativado:

Ferramentas > Suplementos (Tools > Add-ins)

Marcar  biblioteca de normas / biblioteca de normas Library e confirmar. 

Configuração do biblioteca de normas:

Definir quais normas, tamanhos e tipos de fixadores estarão disponíveis.

Ajustar opções de arquivos (se os fixadores serão cópias ou configurações de uma peça existente). 

Furos compatíveis:

Furos criados com assistente de furos (Assistente de furação).

Furos simples e cortes cilíndricos como features separados. 

Não reconhece furos em corpos derivados/importados quando o cilindro não é um recurso separado (por exemplo, um boss com um círculo no sketch que vira sólido único). 

Como o fixadores inteligentes “pensa”
1. Leitura dos furos
Baseado em features:  
Ele não “vê” só geometria; ele lê o tipo de feature (assistente de furos, corte cilíndrico etc.). 

Dados do assistente de furos:  
O assistente de furos usa dados da biblioteca de normas: diâmetro de furo, folga, rosca, furo passante, escareado, roscado etc.
Esses dados permitem ao  saber qual fixador padrão se encaixa naquele furo. 

2. Escolha do fixador
Para furos do assistente de furos, o fixadores inteligentes tenta colocar parafusos/porcas/arruelas compatíveis com o padrão e o tamanho do furo. 

Para outros tipos de furos, você pode configurar tipos padrão de parafuso para serem usados como default. 

3. Criação dos mates
Cada fixador é inserido já com:

Mate concêntrico entre o corpo do parafuso e o furo.

Mate coincidente entre a cabeça/porca/arruela e a superfície de apoio. 

Isso faz com que o conjunto fique montado automaticamente, sem precisar criar mates manualmente para cada parafuso.

Pilhas de hardware (stacks): porcas e arruelas
O fixadores inteligentes pode criar pilhas de peças:

Parafuso + arruela de pressão + arruela lisa + porca, por exemplo. 

Cada componente da pilha recebe:

Mate concêntrico com o parafuso.

Mate coincidente com a superfície de apoio (face da peça). 

Você pode editar a pilha para:

Adicionar/remover arruelas.

Trocar tipo de porca.

Ajustar comprimento do parafuso para atravessar várias peças.

Configurações e padrões
Cada fixador inserido é uma peça configurável da biblioteca de normas:

Você pode mudar o comprimento, diâmetro, tipo de rosca, etc., via configuração. 

Em padrões de furos:

O fixadores inteligentes cria um padrão de fixadores derivado do padrão de furos.

Todos são do mesmo tipo e tamanho, mas você pode usar configurações diferentes (por exemplo, alguns parafusos mais longos em posições específicas). 

Fluxo de uso passo a passo
Passo 1: Criar os furos
Modelar as peças normalmente.

Usar assistente de furos para criar furos padronizados (furo passante, roscado, escareado etc.). 

Se for montagem, criar furos que atravessam múltiplos componentes, se necessário.

Passo 2: Montagem
Montar as peças na assembly.

Garantir que os furos estejam alinhados onde você quer colocar fixadores.

Passo 3: Ativar fixadores inteligentes
Com a biblioteca de normas ativa, usar o comando fixadores inteligentes na montagem (geralmente em Assemblies > fixadores inteligentes). 

Escolher:

Quais furos serão preenchidos (todos, um padrão específico, faces, componentes).

Se quer adicionar pilhas de hardware (porcas/arruelas).

Passo 4: Ajustar os fixadores
Abrir Edit fixadores inteligentes para:

Trocar tipo de parafuso (se a sugestão automática não for a desejada).

Ajustar comprimento.

Adicionar/remover arruelas e porcas. 

Evitar editar diretamente via Edit Sketch / Edit Feature na peça da biblioteca de normas, pois isso não atualiza o banco de dados da biblioteca de normas corretamente. 

Limitações importantes
Furos não suportados:

Furos em corpos importados sem features de furo reconhecíveis.

Furos “embutidos” em um sólido único sem feature cilíndrica separada. 

Dependência da biblioteca de normas:

Se a biblioteca de normas não estiver bem configurada (normas, tamanhos, arquivos), o fixadores inteligentes pode:

Sugerir fixadores errados.

Não encontrar fixador compatível.

Em termos bem diretos: “como ele funciona exatamente”
Você cria furos com assistente de furos ou features cilíndricas.

O  guarda os dados desses furos na biblioteca de normas (diâmetro, tipo, norma, rosca etc.). 

Quando você chama fixadores inteligentes na montagem, ele:

Varre os furos suportados.

Consulta a biblioteca de normas para saber qual fixador padrão combina com cada furo. 

Insere parafusos (e, se configurado, porcas e arruelas) automaticamente.

Cria mates concêntricos e coincidentes para montar tudo. 

Você ajusta detalhes finos (tipo de fixador, comprimento, pilha de hardware) via Edit fixadores inteligentes, mantendo tudo dentro da lógica da biblioteca de normas.

Se você quiser, posso te montar um “roteiro de uso” bem prático, tipo: passo a passo com exemplos de furos (escareado, roscado, atravessando várias peças) e que fixadores o fixadores inteligentes normalmente escolhe.
```
