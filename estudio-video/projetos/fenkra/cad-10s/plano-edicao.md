# Plano de edição: <projeto>

## Inventário de mídia
| Arquivo | Resolução | fps | Duração | Observação |
|---|---|---|---|---|

## Storyboard
| # | Tempo | Enquadramento | Imagem (origem) | Texto na tela | Movimento | Transição | SFX |
|---|---|---|---|---|---|---|---|

## Pedidos
- IA (gerador-ia):
- Gráficos (motion):
- Áudio (sonoplasta):

## Passos executados (reproduzível)
```bash
./produzir.sh   # trilha, render 3D, mix com SFX (sfx_tempos.txt), cor, loudness, export e QA
```

## Licenças
| Recurso | Fonte | Licença | Comprovante |
|---|---|---|---|
| Trilha | ferramentas/trilha.py (tecnologico, semente 1) | original do estúdio | este repositório |
| SFX | biblioteca/sfx (gerar_sfx.py) | original do estúdio | biblioteca/sfx/LICENCA.txt |
| Fontes | Rubik Mono One, Space Mono (google/fonts) | SIL OFL 1.1 | biblioteca/fontes/tech/*/OFL.txt |
| Look | tech-limpo (lut.py) | original do estúdio | — |
| Gráficos 3D | graficos/render_cad.py | original do estúdio | — |

## Versões
| Versão | Data | O que mudou | Aprovado |
|---|---|---|---|
