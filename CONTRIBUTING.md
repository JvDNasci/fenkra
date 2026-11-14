# Como contribuir com o Fenkra

Obrigado por querer ajudar. Este guia diz o que precisamos para aceitar uma contribuição sem risco
de propriedade intelectual. Ele vale para código, dados da biblioteca, documentação, ícones e textos.

> A licença do projeto é LGPL-2.1-or-later ([ADR 0003](docs/arquitetura/adr/0003-licenca.md)). O titular
> é João Vitor Dias Nascimento.

## 1. Acordo de contribuição (CLA)

Antes do primeiro pull request, leia o [CLA](CLA.md) e escreva no PR a frase de aceite. Pelo CLA você continua
dono da sua contribuição e dá ao titular o direito de distribuí-la também sob outras licenças. Isso permite
oferecer depois uma solução integrada sem fechar o núcleo, que segue livre. A linha `Signed-off-by` (`git
commit -s`) continua bem-vinda, mas não substitui o aceite do CLA.

## 2. O que NÃO pode entrar

- Código GPL, AGPL, de licença desconhecida ou sem licença. Só entra código compatível com a
  LGPL-2.1-or-later (MIT, BSD, Apache-2.0 com a ressalva da ADR 0003, LGPL, ISC, zlib e similares).
  Conferir com `cargo deny check` quando a CI existir.
- Código, texto, imagens, ícones ou capturas de tela de softwares proprietários ou de suas ajudas
  e manuais. Estudar a documentação pública é permitido; reescrever com as próprias palavras também.
  Copiar não.
- Nomes de produtos ou marcas proprietárias no código, na interface e na documentação, nem como
  compatibilidade. Escreva "softwares de modelagem CAD avançados". Software livre confiável pode ser
  citado. Use o [glossário](docs/produto/glossario.md).
- Conhecimento obtido sob confidencialidade ou contrato: quem aceitou a licença de uso de um CAD
  comercial (EULA) ou assinou NDA não contribui com o que aprendeu sob esses termos.
- Engenharia reversa de formatos fechados.

## 3. Procedência

- Toda fonte consultada entra em [docs/arquitetura/procedencia.md](docs/arquitetura/procedencia.md).
- Código copiado de projeto de terceiros compatível precisa manter o aviso de copyright original e
  entrar em THIRD-PARTY-LICENSES.
- Dependência nova: informe a licença no PR e atualize THIRD-PARTY-LICENSES.

## 4. Contribuições geradas por IA

Se uma parte relevante foi escrita ou sugerida por uma ferramenta de IA, diga isso no PR e qual
ferramenta usou. Você continua responsável por revisar, por garantir que nada foi copiado de fonte
incompatível e por aceitar o CLA.

## 5. Dados de normas e catálogos

Siga a [política de dados](docs/biblioteca/POLITICA-DE-DADOS.md): medidas são fatos, mas nunca copie
layout, notas, texto ou figuras. Todo JSON da biblioteca declara fonte, página, nível (A, B, C ou X) e
licença. Nunca copie valores de fonte GPL.

## 6. Convenções do projeto

- Branch `develop`; um commit por item de backlog, em português, citando o ID (por exemplo `B151`).
- Texto novo de interface sempre com tradução em `crates/fenkra-app/src/i18n.rs`.
- Testes montam o documento em código, recalculam e conferem status, volume e caixa envolvente.
- `cargo test --workspace` precisa passar.
- Decisão de arquitetura vira ADR em `docs/arquitetura/adr/`.

## 7. Lista do pull request

- [ ] Aceitei o [CLA](CLA.md) (frase de aceite no primeiro PR).
- [ ] Escrevi a contribuição ou tenho o direito de enviá-la.
- [ ] Não há código GPL/AGPL, de licença desconhecida, nem material de produto proprietário.
- [ ] Não citei marcas proprietárias.
- [ ] Declarei uso de IA, se houve.
- [ ] Dados de normas têm fonte, página, nível e licença.
- [ ] Atualizei procedencia.md e THIRD-PARTY-LICENSES quando precisou.

> A checagem automática do aceite do CLA e o modelo de PR com essas caixas dependem da CI (B67), que
> ainda não existe; não há pasta `.github/` no repositório.
