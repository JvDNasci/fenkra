<!-- origem: contexto do usuário sobre fixadores automáticos; reescrito com palavras do Fenkra (B204) -->

# Fixadores automáticos (CTX-FIX)

> O texto original, gerado com assistente de IA, descrevia o comando de um software de terceiros. Foi
> substituído por este resumo do que o Fenkra precisa. Os nomes de comandos aqui são os do Fenkra.

## Duas peças, um papel cada

- **Biblioteca de normas:** o estoque de ferragens (parafusos, porcas, arruelas, pinos) das normas ISO, DIN,
  ANSI e de polegadas, todo paramétrico: diâmetro, comprimento, rosca, cabeça, material.
- **Fixadores automáticos:** o comando de montagem que olha os furos e escolhe o que colocar neles,
  consultando a biblioteca.

## O que o comando precisa

1. **Ler furos como recurso, não só como geometria.** Furo normalizado, furo simples e corte cilíndrico
   entram com seus dados (diâmetro, folga, rosca, passante, escareado). Um cilindro sem recurso próprio, em
   corpo importado, não é reconhecido e o aviso explica por quê.
2. **Escolher o fixador** que casa com a norma e o tamanho do furo; para furos sem norma, usar um tipo padrão
   configurável.
3. **Inserir já restrito:** o eixo do parafuso concêntrico ao furo e a cabeça (ou porca, ou arruela)
   encostada na face de apoio, sem o usuário criar restrições à mão.
4. **Montar a pilha de ferragens:** parafuso, arruela de pressão, arruela lisa e porca, cada item restrito
   ao anterior. A pilha é editável: acrescentar ou tirar arruelas, trocar o tipo de porca, ajustar o
   comprimento para atravessar várias peças.
5. **Seguir os padrões:** em um padrão de furos, os fixadores formam um padrão derivado, do mesmo tipo e
   tamanho, com a possibilidade de variar o comprimento em posições específicas.

## Fluxo de uso

1. Criar os furos nas peças (furo normalizado, simples ou cilíndrico).
2. Montar as peças deixando os furos alinhados.
3. Chamar Fixadores na montagem e escolher o que preencher (todos os furos, um padrão, faces ou
   componentes) e se a pilha leva porca e arruelas.
4. Ajustar o resultado: tipo, comprimento e pilha. Editar a peça da biblioteca por fora do comando não
   atualiza os dados da biblioteca; a edição passa sempre pelo comando.

## Limites e cuidados

- Furos em corpos importados sem recurso reconhecível não são preenchidos.
- Biblioteca mal configurada gera sugestão errada ou nenhuma. O comando deve dizer qual norma ou tamanho
  faltou, em vez de falhar calado.

## Como isso se liga ao Fenkra

O gerador de fixadores e a biblioteca de normas já estão em `crates/fenkra-solidos/src/biblioteca.rs`; a
inserção com restrições fica na bancada Montagem. A ficha de normas está em [fixadores.md](fixadores.md).
