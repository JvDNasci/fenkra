# Glossário — SolidCadCraft

Termos próprios do projeto. Use estes em vez de nomes comerciais de outros CADs.

## Documento e árvore

| Termo | Significado |
|---|---|
| Peça | Documento com features que geram um ou mais corpos |
| Montagem | Documento que posiciona componentes (peças e submontagens) |
| Geometria mestre | Documento-esqueleto com geometria e parâmetros que controlam outras peças |
| Desenho | Documento 2D com folhas, vistas, cotas e tabelas |
| Árvore | Organização do documento: Origem, Referências, Parâmetros, coletores, pastas e features |
| Origem | Ponto de origem, eixos X/Y/Z e planos XY/XZ/YZ fixos de todo documento |
| Pasta Referências | Pasta fixa com geometria de referência e geometria recebida por link |
| Pasta Parâmetros | Pasta fixa com os parâmetros do documento |
| Pasta personalizada | Pasta criada pelo usuário para organizar features; não muda a ordem de execução |
| Coletor de corpos | Contêiner nomeado que agrupa corpos; pode ser o destino ativo das features |
| Corpo | Sólido ou superfície independente |
| Feature / comando | Operação gravada que gera ou modifica geometria |
| Linha de tempo | Sequência dos comandos na ordem de execução |
| Marcador de rollback | Ponto da linha de tempo após o qual os comandos ficam desativados |

## Parâmetros e links

| Termo | Significado |
|---|---|
| Parâmetro | Valor com nome, unidade e expressão opcional |
| Propriedade | Dado de identificação: PN, descrição, material, tipo de peça, personalizadas |
| PN | Número da peça (part number) |
| Projeto relacional | Modo de projetar em que peças se atualizam por links a partir de mestres ou outras peças |
| Publicação | Elemento ou parâmetro que um documento disponibiliza para ser ligado por outros |
| Link | Ligação associativa a uma publicação: geometria, parâmetro, posição ou corpo |
| Link ativo / congelado / quebrado | Sincroniza / mantém o último valor / alvo não encontrado |
| Variável geométrica | Elemento com nome e tipo (ponto, curva, plano, superfície...) usável como entrada de features e em expressões |
| Redirecionar | Fazer uma variável ou link apontar para outro elemento, atualizando quem a usa |
| Substituir referência | Trocar a entrada de um comando mantendo o comando e seus dependentes |
| Edição não destrutiva | Modificar reeditando o objeto existente, sem apagar e recriar |
| Referência persistente | Identificação estável de face/aresta/vértice entre recomputes |

## Módulos

| Termo | Significado |
|---|---|
| Furo inteligente | Furo gerado a partir da norma e tamanho do fixador |
| Rosca paramétrica | Rosca definida por norma, tamanho, passo e classe; cosmética ou modelada |
| Nervura por caminho | Nervura gerada a partir de uma curva aberta, estendida até as paredes |
| Recorte | Remoção de material a partir de sketch |
| Junta | Relação cinemática entre componentes |
| Drive | Movimento imposto a uma junta para animar um mecanismo |
| Modos visualizar / contexto / editar | Níveis de carregamento de componentes em montagens |
| Vista ligada / travada / desatualizada | Estados do link de uma vista de desenho com o 3D |
| Flange base / de aresta / contínua | Features de chapa: inicial, em arestas, seguindo superfícies |
| Planificação | Chapa desdobrada para corte |
| Membro estrutural | Perfil varrido ao longo de um segmento do esqueleto |
| União | Tratamento do encontro de membros (meia-esquadria, topo, encaixe...) |
| Lista de corte | Tabela de membros com comprimento, ângulos, quantidade e peso |
| Versão | Instantâneo automático de um documento |
| Revisão | Versão liberada e congelada pelo usuário (A, B, C...) |
| Estrutura de produto | Árvore de itens do produto com PN, quantidade e revisão |

## Evitar (marcas ou termos de terceiros)

"FeatureManager", "SmartMates", "eDrawings", "PartDesign", "TechDraw" e outros nomes de produtos ou
recursos de fabricantes. Use os termos acima.
