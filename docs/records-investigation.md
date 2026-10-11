# Registros de medição

O parser suporta registros de tamanho fixo e verifica seus limites e segmentos
de origem antes de interpretá-los. A ordem original e os bytes brutos são mantidos.
Campos não compreendidos continuam identificáveis no registro de auditoria.

## Campos implementados

Os offsets abaixo são relativos ao início do registro suportado:

| Offset | Tipo | Interpretação | Confiança |
|---|---|---|---|
| 0–4 | bytes | calendário, com representação bruta preservada | comparado a referência independente |
| 24–29 | três `u16` little-endian | índices A, B e total em décimos | comparado a referência independente |
| 30–31 | `u16` little-endian | ponto | comparado a referência independente |
| 32–33 | `u16` little-endian | ano bruto | referência limitada |
| 36–39 | `u32` little-endian | identificador do registro | comparado a referência independente |
| 40–47 | dois `f32` little-endian | coordenadas | comparadas considerando precisão de origem |

Os bytes 5–23 e o campo no offset 34 não têm semântica estabelecida. Datas
inválidas, valores não finitos e ausências não são corrigidos nem descartados;
os bytes e bits de origem permanecem disponíveis.

## Evidência e limites

A comparação independente foi feita com uma exportação fornecida pelo usuário.
A referência e os registros experimentais não fazem parte deste repositório.
A equivalência para outros modelos, firmwares e versões do software precisa de
referências adicionais. A semântica de campos reservados e de calibração segue
em aberto.
