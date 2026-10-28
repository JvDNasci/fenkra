# ADR 0005: Núcleo + módulos como plugins

- **Status:** proposta
- **Data:** 2026-10-21

## Contexto
O Fenkra tem 7 módulos (sólidos, superfícies, montagem, desenho, chapas, soldas, gestão) e
recursos transversais (parâmetros, propriedades, links, BOM, biblioteca). Os módulos precisam evoluir
em ritmos diferentes, e módulos avançados (ex.: superfícies por pontos de controle) entram depois.

## Decisão
- **Núcleo** fornece: documento, árvore e linha de tempo, recompute, naming persistente, parâmetros,
  propriedades, links, armazenamento/revisões, seleção, desfazer e o registro de features.
- **Cada módulo é um plugin** que registra: tipos de feature, comandos, painéis e (se houver) tipos de documento.
- Toda feature implementa a mesma interface: entradas declaradas (que podem vir de link), parâmetros,
  `executar()` sem efeitos colaterais, histórico topológico de saída e propriedades geradas.
- Módulos se comunicam **só pelo modelo do documento** (geometria, parâmetros, propriedades), nunca chamando código um do outro.
  Ex.: o desenho lê a lista de corte como propriedades de corpos, sem depender do módulo de soldas.

## Alternativas consideradas
- **Monolito:** mais simples no início, mas acopla módulos e dificulta desativar ou substituir.
- **Processos separados por módulo:** isolamento máximo, custo de comunicação alto demais para geometria.

## Consequências
- ✅ Cada módulo pode ter versão mínima no MVP e crescer sem mexer nos outros.
- ✅ Integração com o desenho e a BOM é automática para qualquer módulo que gere corpos com propriedades.
- ⚠️ A interface de feature precisa estar bem definida antes do primeiro módulo.
