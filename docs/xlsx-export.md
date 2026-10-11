# Exportação XLSX

Cada trabalho é exportado em um arquivo XLSX. Grupos independentes ocupam abas
separadas; a ordem pode seguir os registros armazenados ou uma referência aprovada.
Os cabeçalhos e unidades são descritos na própria exportação.

Índices usam os inteiros originais em décimos e a precisão da célula é apenas
apresentação. Total é lido do seu campo próprio. Coordenadas preservam o valor
binário decodificado, enquanto o formato de exibição pode limitar as casas visíveis.
Ausências viram células vazias; zero medido continua zero. A auditoria JSON mantém
bytes brutos, bits e offsets.

O modo simples escreve somente XLSX. O modo completo inclui os arquivos de
proveniência, auditoria e download. Ambos aplicam os mesmos controles de origem,
cobertura e validação científica e não sobrescrevem saídas existentes.

Uma referência privada foi usada durante o desenvolvimento para comparar o
layout. Seu arquivo e seus dados não são incluídos nesta distribuição.
