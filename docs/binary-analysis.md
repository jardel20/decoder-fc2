# Análise binária

O leitor aceita a variante estrutural documentada pelo projeto e valida limites,
contagens, preenchimento e trailers antes de expor os payloads. A implementação
não altera a fonte. O relatório local completo da amostra foi mantido fora da
distribuição pública, em `local/private-reference-docs/`.

## Contêiner observado

Cada bloco físico tem 512 bytes: cabeçalho de 32 bytes, payload de 256 bytes,
preenchimento de 220 bytes e trailer de 4 bytes. Os campos multibyte desta
variante são little-endian. A assinatura observada é `FS2\n`; o número e total
de blocos precisam ser coerentes e os trailers precisam corresponder ao perfil
suportado. Esses elementos identificam uma variante implementada, não uma
especificação universal de todos os arquivos Falker.

O mapeamento traduz endereços da memória reconstruída em posições físicas dos
blocos. A origem de cada segmento é preservada para auditoria. Registros podem
atravessar fronteiras de payload; o leitor reúne segmentos somente após validar
a geometria e os limites.

## Limites

Bytes reservados, firmware e campos sem interpretação permanecem disponíveis
na inspeção bruta. Um valor plausível não confirma sua semântica. Consulte
[format-specification.md](format-specification.md) para campos implementados e
níveis de confiança.
