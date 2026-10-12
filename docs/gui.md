# FC2 Explorer — implementação e entrega

## Estado verificável

A implementação gráfica está integrada ao decoder. Após disponibilização das
dependências no cache, a compilação de testes e o Clippy passaram neste ambiente.
O usuário relatou execução da janela; a interação desktop não foi verificada
diretamente pelo agente. Não há captura de janela nem certificação de executável
portátil. Os resultados efetivamente executados estão em
[gui-validation.md](gui-validation.md).

Para concluir fora deste ambiente, siga
[o roteiro de validação desktop](desktop-validation.md), com scripts Linux e
Windows, logs de compilação, testes manuais e verificação de portabilidade.

No Linux, abrir FC2 e selecionar destinos de exportação usam um navegador de
arquivos interno, sem depender de XDG Desktop Portal ou Zenity. Dê duplo clique
nas pastas, pesquise pelo nome ou informe o caminho da pasta e clique em **Ir**.
Selecione o FC2 e clique em **Selecionar**, ou abra por duplo clique. Na exportação
XLSX, navegue até o destino e clique em **Usar esta pasta**. A listagem é carregada
em segundo plano. Windows e macOS continuam usando os diálogos nativos via rfd.

## Organização e contratos

- `src/explorer.rs`: carregamento, resumos e índices estáveis, acesso a registros e
  exportações revalidadas. O parser, filesystem, decoder, CLI e TUI foram preservados.
- `gui/Cargo.toml`: pacote desktop separado, dependência local do `fc2-rs`. O
  lockfile existente da CLI permanece intacto; o GUI lockfile precisa ser gerado
  pelo primeiro resolve real, não foi preenchido artificialmente.
- `gui/src/main.rs`: entrada nativa e arquivo inicial opcional/arrastar arquivo.
- `gui/src/app.rs`: explorador, detalhes, abas, virtualização, configuração e eventos.
- `gui/src/state.rs`: navegação, seleção, filtros, ordenação e caches sem egui.
- `gui/src/tasks.rs`: diálogos nativos, leitura e exportação fora do render.
- `gui/src/maps.rs`: Walkers, seleção de coordenadas reais, enquadramento e cache.
- `gui/src/icons.rs` e `theme.rs`: catálogo Phosphor/fontes embutidas e temas.
- `gui/tests/components.rs`: passes egui sem janela e sem rede, ainda não executados.
- `tests/explorer*_tests.rs` e `gui_state_tests.rs`: testes sintéticos executáveis
  no pacote principal; não dependem dos crates gráficos.
- `.github/workflows/gui.yml`: build/test/lint Windows/Linux e artefatos; a
  automação foi escrita, mas não executada remotamente nesta sessão.

`Arc<LoadedDataset>` mantém o conteúdo imutável durante a apresentação. A
identidade interna usa `(índice de trabalho, índice de grupo, índice de registro)`,
porque IDs armazenados podem se repetir. Índices internos não são exibidos como
IDs científicos. Navegação, busca e ordenação não alteram a exportação original.

## Trabalho coordenado

Foram usados seis subagentes reais, em rodadas limitadas pelos slots disponíveis:

1. `audit`: auditoria do código e contratos; depois primeira implementação de dados.
2. `core_review`: revisão independente do núcleo, equivalência com decoder e CRC.
3. `gui`: construção das telas, tabelas, navegação e integração dos workers.
4. `maps`: Walkers, marcadores, cache, modo local e avaliação PMTiles.
5. `design`: Phosphor, tokens e temas claro/escuro.
6. `quality`: testes de estado e revisão das APIs, concorrência e controles.

O coordenador integrou os módulos e corrigiu caches, APIs alteradas no egui,
cancelamento, prevenção de diálogos simultâneos e fechamento durante exportação.

## Dependências e compatibilidade

As versões diretas foram selecionadas após consulta às fontes oficiais:
`eframe = 0.36.2`, `egui_extras = 0.36.2`, `walkers = 0.61.0`,
`egui_phosphor_icons = 0.5.0`, `rfd = 0.17.2`. Walkers e Phosphor utilizam egui
0.36; eframe exige Rust 1.95. Transitive dependencies ainda precisam de resolve
real e lockfile. A escolha é compatível pelas versões declaradas, não um build
certificado. O core não ganhou dependências gráficas.

Fontes: [eframe e MSRV](https://github.com/emilk/egui/blob/0.36.2/Cargo.toml),
[Phosphor/egui](https://docs.rs/crate/egui_phosphor_icons/latest),
[Walkers](https://docs.rs/crate/walkers/latest),
[diálogos rfd](https://github.com/PolyMeilex/rfd/blob/master/Cargo.toml).

## Compilar e executar

No diretório do projeto, com Rust 1.95+ e acesso ao registro Cargo:

```sh
cargo check --manifest-path gui/Cargo.toml
cargo run --manifest-path gui/Cargo.toml
cargo run --manifest-path gui/Cargo.toml -- caminho/arquivo.FC2
cargo build --manifest-path gui/Cargo.toml --release
```

O primeiro resolve gera `gui/Cargo.lock`; depois retenha-o e acrescente `--locked`
para builds reproduzíveis. CLI/TUI continuam independentes:

```sh
cargo build --release --locked
./target/release/fc2 tui arquivo.FC2
./target/release/fc2 works arquivo.FC2
```

No Windows x64, o projeto configura runtime MSVC estático:

```powershell
cargo build --manifest-path gui/Cargo.toml --release --target x86_64-pc-windows-msvc
.\gui\target\x86_64-pc-windows-msvc\release\fc2-explorer.exe
```

No Linux, são necessários OpenGL e ambiente X11/Wayland. O build utiliza `glow`;
os diálogos utilizam `xdg-desktop-portal` e o backend da sessão. Para compilar no
Ubuntu, os pacotes usuais são `pkg-config`, `libwayland-dev`, `libxkbcommon-dev` e
`libgl1-mesa-dev`. NixOS requer um ambiente de desenvolvimento com esses recursos.
Uma compilação NixOS não comprova execução em outras distribuições. Empacotamento
Linux portátil e validação física Windows continuam pendentes.

## Fluxo implementado em código

Abra um FC2 ou arraste o arquivo para a janela. A leitura verifica a estrutura
fora do render, preserva o arquivo anterior quando falha e não modifica o FC2.
Selecione um trabalho para atualizar detalhes e destaque no mapa; duplo clique
ou **Abrir trabalho** abre as abas **Medições**, **Mapa**, **Metadados** e
**Exportação**. **Voltar** preserva seleção, filtros e posição cartográfica.

As listas e tabelas são virtualizadas. Busca, grupos, avisos e ordenação por
colunas operam sobre chaves estáveis. O inspector conserva bytes, segmentos,
calendário bruto, bits GPS e campos sem interpretação, sem estimativas.

Marque trabalhos na lista ou com Espaço fora dos campos de texto. **Simples**
gera somente XLSX; **Completo** reutiliza pacote e auditoria do core. **JSON**
gera auditoria estrutural de um trabalho. Destino inicial é Downloads/equivalente,
com diálogo nativo; XLSX cria subpasta nova. Progresso mostra etapas reais e spinner,
sem porcentagem inventada. Falhas por trabalho não são tratadas como sucesso total.

Cancelar leitura é cooperativo: aguarda a tarefa/fechamento do diálogo e descarta
o resultado; não interrompe o decoder. Novos diálogos são bloqueados enquanto há
uma operação pendente. Fechar a janela durante exportação é recusado até terminar.
Exportação não oferece cancelamento que aparente desfazer escrita já iniciada.

## Limites científicos e pendências

- CRC valida setores/estrutura; não comprova equivalência científica com o Leaf.
- A GUI não infere modelo, firmware, timezone ou unidades de campos desconhecidos.
- Coordenadas são finitas/em limites, mas o datum não é comprovado; a sobreposição
  cartográfica é exploratória. Ausências não recebem pontos de exemplo.
- Mapas iniciam sem pedidos externos. Ativar OSM revela a região dos tiles;
  nenhum FC2 ou registro é enviado. Cache HTTP e atribuição são preservados.
- PMTiles foi avaliado, mas seletor/arquivo offline não estão implementados.
- CSV científico continua condicionado a perfil na CLI; não foi criado bypass.
- Janela, temas em execução, diálogos, interação do mapa/tiles, teclas e
  distribuição em um único executável ainda exigem build e testes reais.

Para uma captura real depois de obter as dependências (feature de eframe):

```sh
EFRAME_SCREENSHOT_TO=/tmp/fc2-explorer.png cargo run --manifest-path gui/Cargo.toml --features screenshots
```

Essa opção não foi usada para simular uma captura. Não há screenshot de GUI
funcional nesta entrega enquanto o build não puder ser executado.
