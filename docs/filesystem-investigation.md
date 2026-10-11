# Sistema de arquivos ativo

A extração percorre a árvore ativa do sistema de arquivos embutido no contêiner.
A seleção considera estado, checksum e encadeamento; cópias históricas ou setores
sem referência não viram medições ativas automaticamente.

O parser verifica limites, IDs duplicados, CRCs, diretórios, cadeias e cobertura
entre arquivos. Arquivos reconstruídos mantêm seus segmentos físicos, hashes e
bytes originais. A exportação valida novamente essa proveniência antes de criar
saídas.

Configurações e arquivos de medições são interpretados separadamente. Um nome
ou contador ausente gera aviso; dados não são inferidos de cópias antigas. Setores
ou arquivos que não podem ser associados com segurança continuam no inventário
bruto ou bloqueiam a exportação científica.

## Limites

A compatibilidade está restrita à geometria que o código reconhece. Uma estrutura
desconhecida produz erro e requer outra amostra para investigação. Detalhes da
amostra privada usada durante o desenvolvimento não são distribuídos.
