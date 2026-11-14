# ADR 0101: cena de apresentação no documento, com as câmeras nas vistas salvas

Data: 2026-11-14. Item: B182.

## Contexto

O B182 pede um nó Cena com câmeras salvas, luzes, ambiente (imagem panorâmica, degradê, cor, chão com
reflexo e sombra) e aparências por componente ou face, reutilizado pelas imagens de passo (B157), pelo
render (B124) e por outros itens, para que todos usem a mesma câmera e a mesma luz. O documento já
tinha vistas salvas (`Document::views`), e a vista já tinha a luz principal e a sombra do B139.

## Decisão

- `Document::cena: Option<Cena>` (`fenkra-core/src/cena.rs`), fora do arquivo quando ausente. Ausente
  é a cena de fábrica, igual à vista de antes; arquivos antigos abrem sem mudança.
- As **câmeras da cena são as vistas salvas** do documento. A cena guarda só o nome da câmera ativa.
  Assim não há duas listas de câmeras; guardar pela cena ou pelo menu de vistas dá no mesmo lugar, e
  `cena_ui::camera_da_cena` é a câmera que as imagens de passo e o render devem usar.
- Aparências da cena por corpo (coletor) ou por face pela referência estável (`FaceKey`, ADR 0010). A
  face guardada vai para a face atual mais parecida (mesmo dono, papel e tipo; direção e área mais
  próximas). Precedência na vista: face da cena > comando > corpo da cena > peça > coletor.
- A vista recebe a cena como `CenaVista` (números, copiável) no estilo. A luz principal fixa da cena
  vira também a direção do mapa de sombra do B139. A imagem panorâmica sobe numa textura no grupo 0
  (1 × 1 enquanto não houver), lida no fundo e, fora do nível Rápido, no reflexo das faces.
- Chão e reflexo são passadas a mais no shader comum: o chão são dois triângulos translúcidos depois
  das faces; o reflexo desenha a peça espelhada pelo plano do chão antes dela. Cada um tem
  interruptor; o reflexo sai no nível Rápido, ao girar com "Simplificar ao girar" e no modo leve; a
  sombra no chão só existe quando a sombra suave do quadro existe.
- Fundo de cor ou degradê da cena vale só nos modos sombreados; com imagem panorâmica, o fundo do
  estilo vira a cor média dela, para a regra de contraste (B425) manter a peça distinta.

## Consequências

- Apagar ou renomear a vista ativa pelo menu de vistas solta a câmera da cena (o painel da cena segue
  a renomeação feita nele).
- O reflexo não tem sombra nem oclusão (o ponto espelhado não está nos mapas) e some com a distância
  ao centro do chão, em vez de um recorte exato.
- Aparência por componente da montagem e o uso da cena publicada por outra peça ficam para os itens
  que consomem a cena (imagens de passo e render): o modelo já guarda `publicada`.
