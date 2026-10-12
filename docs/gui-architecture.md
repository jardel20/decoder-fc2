# Arquitetura da GUI

A interface gráfica é um pacote Cargo independente em `gui/`, dependente de
`fc2-rs` pelo caminho local. Isso mantém o manifesto e o lockfile da CLI/TUI
independentes de egui, eframe, Walkers e dependências de janela. O decoder continua
utilizável sem a GUI e não foi substituído.

## Contrato de dados

`fc2_rs::explorer::LoadedDataset::load(&Path)` lê uma fonte usando os limites do
leitor existente e chama `workspace::decode`. `from_bytes` oferece a mesma
decodificação para bytes já carregados. Ambos retornam erro para estruturas não
suportadas; não fabricam trabalhos ou registros.

`LoadedDataset` preserva caminho, bytes de entrada e `Dataset` integral. Calcula
uma vez os resumos por trabalho, índices dos registros e coordenadas disponíveis.
A GUI pode compartilhar `Arc<LoadedDataset>` com tarefas em segundo plano sem
clonar a árvore decodificada a cada frame.

`WorkKey` é o índice do trabalho; `RecordKey` contém índices de trabalho, grupo e
registro. Esses índices seguem a ordem do decoder. IDs instrumentais e números
de trabalhos podem ser duplicados em fontes anômalas, portanto não servem como
chave única da interface. `work` e `record` retornam `Option`, validando cada
índice antes de acessar a estrutura.

`WorkSummary` contém quantidades de registros, grupos, registros com ambas as
coordenadas interpretadas e registros com avisos. Não confunde grupos com pontos,
nem avisos estruturais com validação científica.

## Geografia e validação

`GeoPoint` referencia um registro real. Inclui somente pares latitude/longitude
finitos dentro dos limites geográficos. O decoder preserva os bits originais dos
dois `f32`; valores não interpretáveis permanecem ausentes e acompanhados de
avisos. Não se inventam coordenadas nem se usa `(0, 0)` para representar ausência.

O datum não foi confirmado. Os pontos apresentados são coordenadas decodificadas
dentro dos limites, não uma declaração universal de precisão geográfica. A
validação de CRC verifica setores do sistema de arquivos; a equivalência com uma
referência científica continua limitada ao trabalho e arquivo validados.

## Exportação

`LoadedDataset::export` converte índices selecionados nos números de trabalhos e
reutiliza `export_directory_options`: verifica a origem, a cobertura ativa e os
campos dos registros. Seleções vazias, índices inválidos, duplicatas e números
ambíguos são rejeitados. O modo simples exporta XLSX; o completo usa os artefatos
de auditoria existentes. O manifesto informa falhas por trabalho, que precisam
ser apresentadas mesmo quando outros trabalhos foram exportados com sucesso.

`export_json` gera um documento de **auditoria estrutural** de um trabalho, com
hash de fonte, limitações e modelo completo, inclusive bytes e campos ainda
desconhecidos. Antes da escrita, reconfirma o SHA e a igualdade dos dados com uma
nova decodificação da fonte. Usa a escrita atômica existente e nunca sobrescreve
um arquivo. JSON de auditoria não implica equivalência científica.

CSV científico continua condicionado ao perfil de referência da API existente.
A camada de exploração não acrescenta exportação CSV que ignore esse requisito.

## Responsividade e limites

Leitura, decodificação, diálogos e exportação devem ocorrer em workers, comunicando
resultados à thread gráfica por canais. A interface renderiza a árvore carregada,
listas e tabelas virtualizadas, sem decodificar a fonte a cada atualização.

A API atual lê e decodifica integralmente até 64 MiB, conforme a proteção do leitor
existente. Os bytes brutos e representações hexadecimais aumentam o uso de memória;
a GUI não promete um decoder incremental que o núcleo ainda não implementa.

`tests/explorer_tests.rs` usa um sistema de arquivos sintético para verificar
ordem de grupos, índices fora dos limites, GPS ausente, preservação de origem e
valores nulos em JSON, prevenção de sobrescrita, seleção de trabalhos e rejeição
de dados alterados. Fixtures sintéticas verificam a integração e não comprovam
novas interpretações do formato real.
