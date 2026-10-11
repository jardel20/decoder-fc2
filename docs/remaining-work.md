# Limites e trabalho restante

O parser preserva os bytes do contêiner, do sistema de arquivos e dos campos ainda
não interpretados. Isso permite inspeção e auditoria, mas não equivale a decodificar
semanticamente todos os dados do equipamento.

A comparação externa disponível durante o desenvolvimento não cobre todos os
modelos, firmwares, versões do software, calendários ou configurações. Para ampliar
a compatibilidade, são necessários arquivos e exportações independentes pareados,
com autorização para análise local. Pares antes/depois de uma única medição ajudam
a distinguir campos, contadores e checksums.

Permanecem sem interpretação os campos reservados dos registros, várias
configurações globais, épocas de timestamps e possíveis dados de sensores ou
calibração. Sistemas operacionais além do ambiente Linux ainda requerem testes
reais antes de anunciar suporte verificado.
