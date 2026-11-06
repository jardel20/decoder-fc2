# Mapa da GUI

O módulo `gui/src/maps.rs` usa Walkers, com estado persistente entre telas,
marcadores associados a `RecordKey` e enquadramento por limites geográficos.
Um clique retorna a chave real do registro para a aplicação; nenhum ponto
artificial é usado. O mapa não muda o arquivo ou as coordenadas exportadas.

## Coordenadas e limites científicos

A camada de dados fornece somente pares finitos de latitude/longitude dentro
dos limites numéricos admitidos. Isso não comprova o datum geodésico. A GUI
mostra explicitamente **datum não confirmado**: a sobreposição é exploratória,
sem transformação inventada e sem declaração de equivalência WGS84.

Web Mercator não representa os polos. Coordenadas com latitude acima de
85,05112878 graus em módulo ficam fora da camada de marcadores; a quantidade
é informada, e seus valores continuam disponíveis na tabela. O enquadramento
trata travessia do antimeridiano. Os números nunca são arredondados no núcleo.

## Uso online e cache

Ao passar o cursor sobre um marcador, um tooltip ancorado ao cursor apresenta
os dados reais do registro: ID, trabalho, grupo, ponto, data/hora, índices
instrumentais, latitude/longitude e avisos. Campos ausentes mostram **—**. Em
sobreposições, o registro selecionado tem prioridade, seguido do trabalho
selecionado; clicar alterna os registros e atualiza o tooltip correspondente.
O antigo tooltip ancorado à área inteira do mapa foi removido.

A área de desenho é alocada e recortada explicitamente no retângulo disponível.
Os três painéis do explorador usam a mesma altura, preservando margens e bordas
inferiores; o mapa não cresce para fora do painel.

Clicar num trabalho, abri-lo ou clicar em sua aba **Mapa** centraliza a visualização
no centro dos limites dos pontos representáveis desse trabalho e aplica zoom
**17.0**. Se não houver coordenadas representáveis, centro e zoom anteriores são
preservados. O zoom fixo não garante que pontos muito dispersos apareçam todos
simultaneamente; **Enquadrar pontos** continua disponível para isso.

OpenStreetMap inicia habilitado por solicitação do usuário. O cliente HTTP é
criado ao renderizar o mapa, com ou sem coordenadas. A aplicação não oferece
controle para desativar o mapa, conforme solicitado pelo usuário; a região visível pode ser
inferida pelos pedidos ao provedor. Não são enviados FC2, planilhas, índices
ou identificadores de registros. Sem tiles, continuam disponíveis deslocamento,
zoom e seleção de pontos. Na ausência de coordenadas, o mapa mostra uma visão
geral sem marcadores e uma indicação explícita; nenhum ponto é inventado.

O cliente identifica-se como `FC2Explorer/<versão>`. Walkers utiliza sua
implementação oficial de cache HTTP, respeitando cabeçalhos do servidor:

- Linux: `$XDG_CACHE_HOME/fc2-explorer/tiles` ou `$HOME/.cache/fc2-explorer/tiles`.
- Windows: `%LOCALAPPDATA%/fc2-explorer/tiles`.
- macOS: `$HOME/Library/Caches/fc2-explorer/tiles`.
- Sem diretório pessoal identificado: diretório temporário do sistema.

São solicitados apenas tiles da visualização interativa. Não há pré-busca,
download de regiões ou distribuição de tiles do servidor público. A atribuição
© OpenStreetMap contributors permanece visível. Desabilitar o modo online
descarta o cliente e impede novos pedidos da visualização; pedidos previamente
em curso seguem o ciclo de descarte da biblioteca.

Falhas de rede não interrompem a leitura dos dados ou a camada de pontos.
O cliente de tiles é assíncrono; a GUI informa que o fundo depende de conexão,
mas não confirma conectividade só porque o controle está habilitado.

## Offline cartográfico

O modo local atual é uma camada geográfica sem fundo. Não promete reutilização
offline do cache HTTP. Walkers oferece `PmTiles` por feature, mas um seletor de
arquivo e um arquivo cartográfico com licença compatível ainda são necessários;
essa integração não está implementada. Não se deve criar arquivos offline
por raspagem dos tiles públicos de OpenStreetMap.

## APIs oficiais verificadas

Versão selecionada pelo projeto: Walkers 0.61.0, compatível com egui 0.36.
As páginas docs.rs desta versão estavam indisponíveis durante a investigação;
o código fonte oficial consultado em 12/10/2026 documentou as APIs usadas.
A confirmação da compilação da versão fixada depende da obtenção dos crates.

- [Map e callback de projeção](https://github.com/podusowski/walkers/blob/main/walkers/src/map.rs).
- [Estado, zoom e centralização](https://github.com/podusowski/walkers/blob/main/walkers/src/memory.rs).
- [Projeção longitude/latitude](https://github.com/podusowski/walkers/blob/main/walkers/src/projector.rs).
- [Cliente e opções HTTP](https://github.com/podusowski/walkers/blob/main/walkers/src/io/http.rs).
- [Política oficial de tiles](https://operations.osmfoundation.org/policies/tiles/).

## Verificações do módulo

Testes locais do código cobrem estado inicial sem downloader, limites vazios
e inválidos e enquadramento atravessando o antimeridiano. Não dependem da rede.
Execução efetiva será registrada pelo coordenador após a integração e build.
