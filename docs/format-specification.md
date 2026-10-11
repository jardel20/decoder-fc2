# Especificação observada

Este documento descreve somente as variantes implementadas no parser. As
interpretações científicas foram comparadas a uma referência independente
privada durante o desenvolvimento; seus arquivos e valores não são distribuídos.
Confirmação em uma variante não implica compatibilidade universal.

## Contêiner e sistema de arquivos

O perfil observado usa blocos físicos FS2, campos little-endian, payloads com
mapeamento para memória reconstruída e um sistema de arquivos encadeado. O leitor
valida assinatura, limites, geometria, preenchimento, trailers, CRCs e vínculos
ativos antes de decodificar os trabalhos. Consulte os módulos `parser` e
`filesystem` para as verificações executáveis.

Variantes não reconhecidas são rejeitadas com erro. A extensão do arquivo não
substitui a validação do conteúdo.

## Registros

O perfil de medição implementado usa registros de 48 bytes. Índices independentes
são inteiros little-endian em décimos; ponto, calendário, ID e coordenadas têm
campos separados. Veja [records-investigation.md](records-investigation.md) para
o mapa dos campos e seus limites de evidência.

Bytes desconhecidos, bits originais, ausências e segmentos de origem são
preservados. O decodificador não transforma índices instrumentais em concentração
de massa nem inventa valores ausentes.
