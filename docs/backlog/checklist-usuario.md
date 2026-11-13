# O que depende de você

Atualizado em 13/11/2026. Para responder, mande só os números e as letras (ex.: "1A, 2B, 3A"). Onde há
dado a trazer, mande o valor e de onde veio.

## Decisões (responda com a letra)

1. **Licença do projeto (ADR 0003).** Quem é o titular dos direitos e qual contato aparece no projeto?
   - A: você, pessoa física, com seu e-mail. *(recomendado enquanto não houver empresa)*
   - B: uma empresa (diga razão social e CNPJ).
2. **Contribuições (B206, já com DCO no CONTRIBUTING).** Mantém DCO (assinatura nos commits) ou troca por CLA?
   - A: DCO. *(recomendado: simples e sem cessão de direitos)*
   - B: CLA, se você pensa em dupla licença ou versão comercial fechada no futuro.
3. **Nome Fenkra (B207).** Como seguir com a marca?
   - A: você faz a busca no INPI, USPTO e EUIPO (classes 9 e 42) e leva as perguntas do relatório a um advogado. *(recomendado antes de divulgar)*
   - B: adiar até o primeiro lançamento público.
4. **Logo e ícones (B203, trava o B314).** Mande ferramenta, prompt, data e plano usados no `fenix-ref.png`, e escolha:
   - A: manter o logo com a procedência registrada. *(recomendado se a ferramenta permite uso comercial)*
   - B: refazer com um designer, com cessão de direitos clara.
5. **Termos das APIs dos CADs de origem (B267, trava o B298).** Como fazer o exportador por script?
   - A: só script aberto que o usuário roda na própria licença, sem binário ligado à API e sem engenharia reversa. Eu redijo o ADR e você confere os contratos. *(recomendado)*
   - B: não fazer exportador por script; ficar só com STEP.
6. **Primeiro CAD de origem do exportador (B298).** Qual software de origem você usa e tem licença para testar? Diga o nome só para mim; no projeto ele fica sem marca.
7. **DWG e outros formatos (B211).**
   - A: DWG por LibreDWG como processo externo (GPL fora do executável) e 3DM por openNURBS. *(recomendado)*
   - B: DWG pela biblioteca comercial ODA (paga).
   - C: não ler DWG por enquanto; ficar com DXF.
8. **MCP com modelo de verdade (B287).** Qual cliente e qual modelo usar no teste final?
   - A: Claude Desktop ou Claude Code com o servidor `fenkra-mcp`. *(recomendado)*
   - B: um modelo local (diga qual).
9. **Roleta de comandos (B372).** Abra Shift+W em cada bancada e diga o que tirar ou pôr. Se preferir:
   - A: aprovo como está.
   - B: mando a lista de ajustes.
10. **Gravador de IFC próprio (B134, ADR 0082).** O agente escreveu o IFC em Rust em vez da biblioteca externa citada no item.
    - A: aprovo. *(recomendado: sem dependência pesada)*
    - B: quero a biblioteca externa.
11. **Análise no `.fkv` simplificado (B400).** Hoje sai sem interferências e folgas, com caixa para ligar (gera em ~50 s em vez de ~127 s).
    - A: mantém desligada por padrão. *(recomendado)*
    - B: sempre ligada.
12. **Peça de fora do projeto na montagem.** Com projeto aberto ela é barrada.
    - A: permitir com aviso e cópia para o projeto. *(recomendado)*
    - B: manter barrada.
13. **Patentes (B153).** A busca em fonte pública não achou bloqueio, mas não é parecer jurídico.
    - A: contratar busca de liberdade de operação antes de vender. *(recomendado)*
    - B: seguir sem busca enquanto for software livre e gratuito.
14. **Skills externas aprovadas.** Instalo agora `silent-failure-hunter` e `cpp-reviewer`, lendo o conteúdo antes, sem hooks nem MCP?
    - A: sim, agora.
    - B: depois da onda 4.

## Dados que só você consegue trazer

15. **Corrente 219 (B354, B395):** largura interna entre placas internas. As fontes dão 4,6 e 5,00 mm. Hoje está 4,6 provisório. Medida de uma corrente real ou ficha do fabricante.
16. **Terminal rotular do kart (B397):** espessura da carcaça e diâmetro da haste, de uma ficha confiável ou da peça real.
17. **Opcionais da biblioteca** (só se tiver catálogo em mãos; senão eu sigo procurando):
    - altura da cabeça k dos parafusos AN7 e AN8;
    - comprimento L do patim HGH15CA (as fontes dão 60,5 e 61,4 mm);
    - altura KW da porca de fixação M22x1,5 do cilindro (norma até 11, catálogo 7).
18. **Rosca M16x2 que dava "não cabe":** se ainda falhar, mande o arquivo ou a configuração.

## Testes físicos

19. **Corte em V para dobrar tubo (B328, B333):** dobrar uma peça real com o gabarito e dizer se fechou no ângulo.
20. **Gabarito de marcação (B329):** imprimir e conferir a escala com uma régua.
21. **Protótipo em metalon:** montar e comparar com o modelo.

## Teste com mouse (B41)

22. Em cada linha, diga ok ou o que falhou:
    - Arrastar um recurso na árvore; menu "⋯" da árvore (soltar, minimizar, fechar).
    - Painéis: arrastar até as bordas, redimensionar, minimizar, fechar; soltar fora da janela (outro monitor) e voltar.
    - Roleta Shift+W: rolar e arrastar sem mexer na câmera.
    - X e Cancelar de um comando novo somem com ele da árvore.
    - PMI: Ctrl, Shift e F2; duplo clique na nota e na seta; arrastar o início da seta; setas com várias faces.
    - Montagem: inserir peça pela prévia e depois de salvar; abrir peça e voltar; Reancorar restrição perdida.
    - Biblioteca: inserir em furo, em ponto e solto; campo Pega do rebite; curso do cilindro.
    - Fixadores inteligentes: arrastar itens da pilha, cerca, seleção por cor.
    - Desenho: esboço na folha, balões automáticos (janela, Shift+clique), nós das chamadas, Cancelar na barra de progresso.
    - Rosca do furo, alças da chapa, planificar, dobrar e desdobrar animado com a ordem por arraste.
    - Navegação por teclado (setas, Page Up/Down, Alt+↑/↓ na árvore) e anel de foco com Tab.
    - Tooltips animados, dica simples e F1 (ajuda do comando).
    - Abrir `.fkp`, `.fka`, `.fkd` e `.fkv` por duplo clique no Windows (registre em Configurações) e o `.fkv` no celular.
    - Mouse 3D, se você tiver um.

## Também para você

- Apagar à mão a pasta vazia `C:\JOO~1`, que um agente criou por engano (a trava de segurança não me deixa remover pastas na raiz do disco).
