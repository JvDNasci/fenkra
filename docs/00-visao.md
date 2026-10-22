# SolidCadCraft — Documento de Visão

> Rascunho v0.2. Revisar e ajustar.

## 1. Problema

Os CADs paramétricos comerciais (CATIA, SolidWorks, Fusion) são caros, fechados e cada vez mais
dependentes de assinatura e nuvem. A principal alternativa aberta, o FreeCAD, é funcional, mas:

- é **pesado**: a interface trava durante recompute e montagens grandes ficam lentas;
- o **desenho técnico 3D → 2D é fraco**: lento, atualiza o tempo todo, perde cotas quando o modelo muda;
- **estruturas soldadas e chapas** são limitadas;
- **links entre peças** são frágeis e difíceis de gerenciar;
- **não tem gestão de projeto**: estrutura de produto, revisões e controle de links ficam por conta do usuário.

## 2. Proposta

O **SolidCadCraft** é um CAD 3D paramétrico aberto, de abordagem **simples a intermediária**,
com módulos avançados acrescentados depois. Seus pilares:

1. **Sketches paramétricos com parâmetros e cálculos do usuário.** Qualquer cota pode ser uma
   expressão; parâmetros podem ser locais, da peça, da montagem ou da geometria mestre.
2. **Projeto relacional + geometria mestre.** Uma peça ou esqueleto *mestre* publica geometria e
   parâmetros; as peças derivadas se ligam a ela. Mudou o mestre, as peças acompanham.
3. **Links entre peças com regras claras.** Todo link tem estado (ativo, congelado, quebrado),
   política de atualização e aparece num gerenciador de links.
4. **Features pensadas para serem atualizadas por links.** Toda feature aceita como entrada
   geometria ou parâmetro vindo de outra peça, e se atualiza quando o link muda.
5. **2D e 3D sincronizados, mas sob controle do usuário.** Cada vista do desenho é um bloco que pode
   estar **ligado** ao 3D ou **travado**, para não ficar recalculando a todo momento.
6. **Lista de peças e propriedades em todos os módulos.** Toda peça e montagem tem PN, descrição,
   material, tipo de peça e parâmetros personalizados, e isso alimenta o desenho e a BOM.
7. **Interface minimalista** com features o mais completas possível.
8. **Desempenho**: a interface nunca congela; montagens grandes têm modos de visualização leves.

## 3. Módulos

| Módulo | Escopo inicial |
|---|---|
| **Modelagem Sólida** | Extrusão, revolução, ângulo de saída, nervura por caminho, recorte, furo, furo inteligente, rosca paramétrica |
| **Superfícies Complexas** | Loft, curvas 3D, continuidade suave, análise de zebra e de desvio. *Futuro:* superfícies por pontos de controle (estilo Blender/subdivisão) integradas ao sólido |
| **Montagem e Mecanismos** | Restrições geométricas clássicas, fixo e graus de liberdade, drives de mecanismo, roscas entre peças ligadas, modos visualizar / contexto / editar |
| **Desenho Técnico** | Leve; vistas inteligentes, cortes, seções, cotagem, geometria 2D e transformações simples; cada vista é um bloco ligado ou travado ao 3D |
| **Chapas Metálicas** | Flanges (incluindo flange por aresta e sobre superfícies), dobras, planificação, totalmente parametrizado e ligado à geometria mestre |
| **Estruturas Soldadas** | Membros sobre esqueleto, bibliotecas de perfis cadastráveis, tipos de união/canto, lista de corte |
| **Gestão de Projeto** | Estrutura de produto, revisões simples com congelamento, histórico de versões, gestão de links |

**Transversal a todos:** parâmetros e propriedades, links e projeto relacional, BOM/lista de peças,
integração com o desenho 2D e biblioteca de hardware.

## 4. Usuários-alvo

| Perfil | Necessidade principal |
|---|---|
| Projetista mecânico | Peças, montagens, mecanismos e desenhos de fabricação |
| Caldeiraria / chapas | Chapas parametrizadas, planificação, desenho com dobras |
| Serralheria / estruturas metálicas | Estruturas soldadas, lista de corte, pesos |
| Pequenas equipes de engenharia | Estrutura de produto, revisões e controle de links sem PDM caro |
| Maker / impressão 3D | Modelagem rápida e formatos de malha |

## 5. Diferenciais

- **Roscas paramétricas** de verdade (geometria helicoidal ou cosmética), com normas e ligadas
  entre peças: o furo roscado e o parafuso compartilham a especificação.
- **Biblioteca pública e editável de hardware e perfis**: fixadores, perfis estruturais com dimensões,
  propriedades de seção e **peso por metro**, mantida pela comunidade em formato aberto, com
  bibliotecas locais da empresa por cima.
- **Projeto relacional simples**: geometria mestre, publicações explícitas e gerenciador de links,
  sem a fragilidade de referências soltas.
- **Desenho com vistas travadas ou ligadas**: leve mesmo em projetos grandes.
- **Gestão de projeto embutida**: revisões e estrutura de produto sem servidor.
- Mais leve que o FreeCAD; gratuito e aberto, sem assinatura nem nuvem obrigatória.
- Normas e perfis brasileiros desde o início (ABNT, perfis nacionais).

## 6. Não-objetivos (por enquanto)

- CAM / geração de G-code
- Simulação (FEA, CFD) e análise dinâmica de mecanismos (forças, torques)
- Superfícies classe A e renderização fotorrealista
- Leitura de formatos nativos fechados (SLDPRT, CATPart, F3D, IPT). A troca é feita por formatos neutros.
- PDM multiusuário com servidor, workflow de aprovação e permissões
- Colaboração em tempo real / nuvem
- BIM / arquitetura

## 7. Formatos

| Tipo | Importar | Exportar |
|---|---|---|
| 3D sólido | STEP (AP203/214/242), IGES, BREP | STEP, IGES, BREP |
| 3D malha | STL, OBJ, 3MF, PLY | STL, OBJ, 3MF |
| 2D | DXF, SVG | DXF, PDF (vetorial), SVG, PNG |
| Dados | CSV, XLSX (parâmetros, BOM) | CSV, XLSX (BOM, lista de corte) |

DWG fica como item a avaliar (ver [reuso open source](reuso-opensource.md)).

## 8. Escopo do MVP

A estratégia é entregar **uma versão funcional de cada módulo**, com menu minimalista e poucas
features, mas completas em opções. Antes disso vem a **fundação compartilhada** (documento,
parâmetros, propriedades, links, armazenamento e revisões), porque todos os módulos dependem dela.
Detalhes no [roadmap](roadmap.md).

## 9. Licença

Proposta: **LGPL-2.1-or-later** para o código ([ADR 0003](adr/0003-licenca.md)) e licença aberta de
dados para a biblioteca pública ([ADR 0008](adr/0008-biblioteca-publica.md)).

## 10. Critérios de sucesso do MVP

- Uma geometria mestre controla 3 peças derivadas; alterar um parâmetro do mestre atualiza peças,
  montagem e desenho.
- Furo roscado e parafuso da biblioteca ligados: trocar M8 por M10 atualiza os dois.
- Montagem de mecanismo simples (ex.: biela-manivela) movida por um drive.
- Chapa com flanges, planificada e detalhada no desenho com tabela de dobras.
- Estrutura soldada com lista de corte e peso total no desenho.
- Revisão A congelada, revisão B editada, e as duas abertas lado a lado.
- Montagem de 1.000 componentes navegável em modo visualização sem travar.
