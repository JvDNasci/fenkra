# ADR 0018 — Conversor em processo filho, com a ponte XDE no kernel

## Contexto

A importação de STEP chamava o `STEPControl_Reader` dentro do programa. Isso traz dois problemas.
O leitor básico devolve só a forma: nomes, cores, camadas e a árvore de produtos ficam para trás. E o arquivo
vem de fora: uma falha de memória no C++ do OpenCASCADE, ou um laço sem fim, leva junto o trabalho aberto, e
nenhum tratamento de erro em Rust alcança esse caso.

## Decisão

O crate `fenkra-conversor` concentra formatos, opções, relatório e provedores. A conversão roda no binário
`fenkra-conv`, um processo por trabalho.

- **Ponte:** `cpp/xde.inc` lê e grava com `STEPCAFControl` e `DESTEP_Parameters` por chamada. `ModeloXde`
  guarda definição e ocorrência separadas: quatro parafusos iguais são um produto e quatro instâncias.
- **Fronteira entre processos:** só dado cruza. O pedido vai em JSON pela entrada padrão e a resposta volta
  num arquivo temporário, porque traz o documento inteiro e um pipe cheio travaria os dois lados. Como o
  `Shape` do kernel não é `Send` nem serializável, o filho entrega o `Document` pronto (a geometria já é
  texto B-rep dentro dele), o relatório e a malha da miniatura. O programa não toca no OpenCASCADE para ler.
- **Binário em `fenkra-app`:** `cargo run -p fenkra-app` só compila os binários do próprio pacote, e o Cargo
  estável não deixa um pacote depender do binário de outro. O `main` de `fenkra-conv` tem uma linha e a lógica
  fica em `fenkra_conversor::processo`. Por isso o `default-run` de `fenkra-app` é `fenkra`, e os testes de
  isolamento moram em `crates/fenkra-app/tests/`.
- **Reparo:** forma sã não é tocada, para a ida e volta preservar volume, área e centroide. O nível Nenhum
  analisa e relata mesmo assim.
- **Superfície aberta:** vira superfície de referência, não corpo. O comando de corpo ligado tamparia as
  bordas e inventaria um volume.
- **Relatório:** as frases guardam modelo e argumentos, para sobreviver ao processo e serem traduzidas na tela.

Provedor comercial futuro entra pelo trait `Provedor`, sem mudar diálogo nem relatório.

## Consequências

- Importar custa um processo novo e a ida da geometria por JSON. Para peça comum isso é pouco; um STEP muito
  grande paga o texto B-rep duas vezes em memória.
- O pacote de release leva `fenkra-conv.exe`, e o fechamento de DLLs parte dos dois executáveis, porque o XCAF
  só é usado pelo filho.
- O limite de tempo é por trabalho. Teto de memória ainda não existe: um arquivo que consuma toda a memória
  derruba o filho pelo sistema, não por um limite nosso.
- Peça com N ocorrências vira N corpos com cópia do B-rep. Importar como montagem com peças compartilhadas
  depende do item de destino da importação.
- PMI: o arquivo é contado e relatado, e `XdeWriteOpts::gdt` liga `SetDimTolMode`. O conteúdo semântico vem no
  item de AP242.
