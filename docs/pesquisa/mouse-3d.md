# Mouse 3D (B322)

> Gerado por IA, sem revisão jurídica. Escrito de memória a partir de documentação pública; reabrir as
> fontes na revisão humana. Fontes em [procedência](../arquitetura/procedencia.md).

## 1. O aparelho

Um mouse 3D é um controlador de seis graus de liberdade: uma tampa que o dedo empurra, puxa, inclina e
torce. Entrega três translações e três rotações, mais botões. Não aponta; muda a vista e, em alguns
programas, a peça. Os do mercado vêm de poucos fabricantes e quase todos se apresentam ao sistema como
dispositivo HID (página de uso genérica, uso "controlador multieixo").

## 2. Três caminhos de integração

| Caminho | Como funciona | Prós | Contras |
|---|---|---|---|
| Driver e SDK do fabricante | o driver filtra o aparelho e o programa conversa com ele por uma API própria | recursos completos (perfis, botões nomeados), suporte do fabricante | exige instalar software proprietário; o app não abre sem ele |
| HID direto | o programa abre o dispositivo e lê os relatórios | sem driver, sem dependência externa | cada modelo tem relatórios um pouco diferentes; o driver do fabricante pode competir pelo aparelho |
| Projetos livres (spacenavd e libspnav, em Linux) | um daemon lê o aparelho e entrega eventos de movimento e botão por um soquete | simples, estável | só Linux e BSD |

Escolha do Fenkra: HID direto no Windows, sem exigir nada para abrir o app; sem aparelho, nada muda. O
protocolo de eventos do projeto livre (seis inteiros de movimento e um número de botão) serve de modelo
para o formato interno, e um leitor de soquete para Linux pode entrar depois pelo mesmo ponto de
entrada (`Mouse3d::empurrar`).

## 3. Formato dos relatórios (a conferir com aparelho real)

Formato mais comum: o primeiro byte é o número do relatório.

| Relatório | Conteúdo |
|---|---|
| 1 | translação: x, y, z, cada um inteiro de 16 bits com sinal, little-endian |
| 2 | rotação: x, y, z, no mesmo formato |
| 3 | botões: máscara de bits, um bit por botão |

Os modelos mais novos podem juntar movimento num relatório só. O decodificador devolve `None` para
relatório desconhecido, sem derrubar nada; esse é o sintoma a procurar se um modelo não responder.
A deflexão máxima fica perto de 350 por eixo; o Fenkra normaliza por 350 e limita a -1..1, e a
sensibilidade ajusta o resto.

## 4. Do eixo à câmera

1. Zona morta: a fração em torno do repouso é ignorada e o valor recomeça do zero depois dela, sem degrau.
2. Sensibilidade separada para translação e rotação, e inversão por eixo.
3. Eixo dominante (opcional): só o eixo de maior deflexão age, para aproximar sem girar.
4. Velocidade proporcional à deflexão: no máximo atravessar a distância até o alvo em um segundo,
   girar 1,6 rad/s e variar a distância em e^1,2 por segundo. Quadros longos são limitados a 0,1 s.

Mapeamento: x e y deslocam o alvo (para os lados e para frente), z aproxima ou afasta, inclinar para
a frente muda a elevação e inclinar para o lado gira em torno da vertical. A câmera do Fenkra não tem
rolagem, então "torcer" ainda não age na vista.

## 5. O que ficou de fora

- Movimento de peça na montagem (precisa da seleção da montagem; o mesmo `Movimento` serve).
- Botões chamando qualquer comando: hoje chamam ações de vista (enquadrar, ortogonal, frente, topo,
  isométrica).
- Leitor para Linux e macOS.
- Rolagem da câmera.

## 6. Depuração

- Sintoma: nada se mexe. Conferir se a opção está ligada, se outro programa (o driver do fabricante)
  tem o aparelho aberto de forma exclusiva e se o relatório chega (o decodificador ignora
  desconhecidos).
- Sintoma: a vista escorrega parada. Aumentar a zona morta.
- Sintoma: gira quando devia aproximar. Ligar o eixo dominante.
