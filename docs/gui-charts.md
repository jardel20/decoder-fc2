# Gráficos de clorofila

A aba **Gráficos** fica entre Mapa e Metadados no trabalho. Usa egui_plot 0.37.0,
compatível com egui 0.36.2 ([changelog oficial](https://github.com/emilk/egui_plot/blob/0.37.0/CHANGELOG.md)).
Permite escolher total, a, b ou **Todas**, com barras lado a lado e legenda
centralizada com o título **Legenda:**. A grade de fundo fica oculta.
A tela inicia em **Média por ponto ± EP**, com erro padrão automático. O botão
**Todos os pontos** mostra todas as medidas individuais, identificadas pelo ID,
sem erro padrão. Clique em **Média por ponto ± EP** para retornar às médias.
Os botões +/− aproximam e afastam;
**Enquadrar** restaura a visualização inicial. Zoom por gesto/roda e reset por
duplo clique também estão habilitados no egui_plot.

## Leituras de origem

Todas as leituras decodificadas entram nos cálculos, na ordem de origem.
IDs duplicados não são removidos. As leituras individuais continuam disponíveis
na aba Medições e no modo **Todos os pontos**. O eixo X é categórico, com rótulos
a 45°: grupo/ponto nas médias e ID da leitura nas medidas individuais.
Use rolagem e zoom para trabalhos longos.

## Média por ponto e erro padrão

As barras de erro são exibidas automaticamente quando o cálculo é possível.
Para pontos com uma única leitura, a média permanece visível, sem barra de erro.

Agrupamento por arquivo/grupo de origem e número do ponto; pontos em grupos
diferentes não são combinados. A ordem é a da primeira leitura de cada ponto.
Inclui todas as leituras decodificadas, zeros e leituras com avisos. O total é
o valor armazenado no FC2, nunca uma soma calculada de a e b.

Média e variância usam o algoritmo incremental de Welford em f64, após conversão
dos inteiros em décimos. A barra de erro representa média ± EP, onde
`EP = sqrt(sum((x - média)^2) / ((n - 1) * n))`, para n >= 2.
Esse EP descreve as leituras do ponto; não caracteriza por si só réplicas
biológicas ou incerteza de calibração. Para n = 1, EP é indisponível e não se
desenha barra de erro. Séries constantes com n >= 2 têm EP zero. Os limites
das barras de erro não são truncados em zero.

O balão mostra média, EP, n, avisos e todos os IDs de origem (com rolagem).
Média e EP usam seis casas decimais apenas na apresentação do balão; não são
arredondados antes do cálculo/desenho. O decoder e suas exportações permanecem
inalterados; as estatísticas derivadas existem somente para visualização.

## Validação

Testes sintéticos conferem a fórmula do EP, n = 1, zeros, séries constantes,
IDs duplicados, separação de grupos e total armazenado. Renderização headless
cobre os dois modos, quatro escolhas de variável, ambos os temas e estado vazio.
Rolagem, zoom, rótulos e balões ainda exigem inspeção visual na sessão desktop.
