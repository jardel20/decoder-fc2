# Plano de interpretação do formato

O decodificador avança de inspeção estrutural para interpretação validada. Cada
campo recebe nível de confiança, evidência e proveniência. Campos sem significado
comprovado permanecem brutos.

## Origem do conhecimento do formato

A estrutura descrita neste projeto foi inferida da observação de arquivos de
dados do próprio usuário, da comparação com exportações independentes dos mesmos
dados e de fontes públicas listadas em [references.md](references.md). O projeto
não decompila nem desmonta programas, aplicativos ou firmware da Falker, não
acessa suas plataformas online e não inclui código, firmware, manuais, logotipos
ou arquivos de dados da Falker. Regiões do arquivo que possam conter material de
firmware são apenas preservadas como bytes, sem análise de código. O projeto é
independente e não tem vínculo, patrocínio ou endosso da Falker.

## Princípios

- Abrir os arquivos originais somente para leitura.
- Validar tamanho, offsets e checksums antes de acessar conteúdo.
- Distinguir arquivos ativos de cópias antigas e regiões reservadas.
- Comparar registros com exportação independente do mesmo arquivo quando possível.
- Manter ausências vazias, zeros medidos e ordem original distintos.
- Bloquear exportação científica quando a fonte ou os campos necessários não forem
  validados.
- Não publicar arquivos experimentais nem valores de referência.

## Evolução

Implementar inspeção binária, parser do contêiner, seleção de arquivos ativos,
modelo de registros, validação, exportação e interfaces. Testes sintéticos cobrem
segurança do parser, mas só referências externas podem validar a interpretação
científica. Campos ainda desconhecidos seguem preservados para auditoria.
