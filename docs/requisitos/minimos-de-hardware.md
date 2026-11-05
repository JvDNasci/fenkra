# Requisitos mínimos de PC e placa de vídeo

Itens B236 e B309. **Situação: proposta, ainda não medida em máquina fraca de referência.** Nenhum
número abaixo foi confirmado em um PC de verdade. A tabela vira "publicada" quando o roteiro de aceite
passar em cada faixa, nos três backends gráficos (Dx12, Vulkan e OpenGL).

| Item | Mínimo (com modo leve) | Recomendado |
|---|---|---|
| Sistema | Windows 10 64 bits | Windows 11 64 bits |
| Processador | x64, 2 núcleos e 4 fios | 4 núcleos e 8 fios ou mais |
| Memória | 4 GB, peças simples | 16 GB |
| Vídeo | Dx12 ou Vulkan; OpenGL 3.3 só com modo leve | placa com driver atual e 2 GB de memória |
| Disco | 200 MB livres | SSD |
| Tela | 1366x768 | 1920x1080 |
| Runtime | Visual C++ 2015-2022 x64 | igual |

## Como o Fenkra reage a uma máquina fraca

Na primeira execução o programa mede o adaptador gráfico (tipo e backend), a memória e os processadores
lógicos. Se o adaptador for de software ou OpenGL, houver menos de 8 GB ou até 4 processadores lógicos,
mostra uma caixa única oferecendo o **modo leve**. A resposta, qualquer que seja, encerra a sugestão: ela
nunca volta e o modo nunca é ligado sozinho. O interruptor fica em Configurações, Exibição, Gráficos.

O modo leve é só uma combinação de ajustes que já existem: malha Rascunho, sem anti-serrilhamento,
simplificar ao girar, iluminação simples, 2 fios no malhador e animações desligadas. Volume, massa e
medidas não mudam, porque vêm da forma exata. Desligar devolve cada ajuste ao valor que a pessoa tinha.

Itens da tabela do B236 que ainda não entram: atualização automática "ao soltar o controle" e tela
inicial sem fundo animado (a tela inicial hoje não tem fundo animado).

## Relatório de falhas (B309)

Se o programa parar por um erro interno, grava um relatório de texto **só no disco local**
(`%APPDATA%\Fenkra\relatorios\pendente.txt`). Na abertura seguinte a pessoa vê o conteúdo inteiro e
escolhe: copiar, guardar (com ou sem abrir a pasta) ou descartar. O programa não abre conexão nenhuma e
não tem endereço de envio: quem decide mandar o texto, e para onde, é a pessoa, por conta própria.

O relatório traz versão, sistema, hardware medido, mensagem e pilha do erro e o final do registro de
partida. **Não leva o modelo**: nem peças, nem sketches, nem nomes de arquivo (os argumentos da linha de
comando são cortados do registro). Pastas com o nome da conta viram `<usuario>`.

## Roteiro de aceite em máquina real

Em cada faixa de máquina e com `WGPU_BACKEND` igual a `dx12`, `vulkan` e `gl`: abrir o exemplo da placa,
modelar uma peça, girar a vista e salvar, sem travar. Registrar tempo até o primeiro quadro, memória do
processo e quadros por segundo ao girar. Preencher a tabela acima com o que foi medido.
