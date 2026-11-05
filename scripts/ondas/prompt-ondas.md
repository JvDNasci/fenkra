Retomar a execução das ondas do backlog (docs/backlog/backlog.json) seguindo a skill fenkra-fluxo e o CLAUDE.md.
A aprovação já está dada para TODAS as ondas, de 0 a 4: não pare para pedir confirmação entre itens.
Limite: pare quando o consumo desta execução chegar a {{LIMITE}} da cota semanal (use a taxa de docs/backlog/custo-estimado.md para estimar). Ao chegar no limite, faça commit do que estiver pronto, atualize o backlog e encerre.

Antes de começar:
1. Mesclar na develop a branch worktree-agent-a97b1d1a08bf25b8c (B276, commit sem merge).
2. Remover as worktrees sem commit (a2d03989d459e3163, a2e4a573af38eb55f, a3e48dda41e84fa82, a6dacc68ce4e96412, a74755aaea5f27a2f, a872bd5f00ada02a9) e relançar os itens que elas estavam fazendo.
3. Marcar como feito no backlog.json, backlog.md e painel.html os itens já mesclados hoje: B16 B18 B19 B22 B23 B24 B25 B47 B48 B87 B88 B90 B91 B93 B94 B97 B185 B186 B187 B241 B242 B243 B244 B245 B246 B247 B248 B249 B253 B254 B255 B260 B261 B265 B275. Em andamento: B86 B89 B92 B95 B96.

Regras:
- Ordem onda 0 → 4; dentro da onda, P0 → P3.
- Um commit por item, citando o ID; cargo test --workspace verde antes de cada merge na develop.
- Tudo em segundo plano: nunca abrir o Fenkra na tela do usuário. Para verificar visualmente, rodar o app com FENKRA_DEMO_FOTO=<arquivo.png> (grava a janela e fecha sozinho), salvando as capturas em docs/backlog/capturas/.
- Depois de cada merge, acrescentar uma linha em docs/backlog/progresso.log: data/hora, ID, status, commit.
- Se algo falhar, registrar em progresso.log e seguir para o próximo item.
