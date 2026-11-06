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

Abrir FC2 e selecionar destinos de exportação usam os diálogos do sistema via
rfd em todas as plataformas, conforme solicitado pelo usuário. No Linux, é
necessário um XDG Desktop Portal com backend de seleção de arquivos ou o fallback
Zenity suportado por rfd. As falhas registradas por esse backend são mostradas
na interface e no terminal; cancelar um diálogo não é tratado como erro. O
navegador interno anterior não é mais acionado pela aplicação.

`rfd 0.17.2` carrega `libdbus-1.so` dinamicamente. Em builds Linux nativos,
`gui/build.rs` detecta seu diretório com pkg-config ou pelo daemon instalado no
perfil do sistema e incorpora o caminho ao ELF. Isso permite localizar a
biblioteca separada do NixOS sem instalar Zenity nem alterar a configuração do
sistema. O portal e seu backend ainda precisam funcionar na sessão gráfica.
Cross-builds não recebem caminhos de bibliotecas do host. A presença do caminho
foi conferida com `readelf`, sem presumir portabilidade entre distribuições.

**Todos** seleciona todos os trabalhos do arquivo, inclusive fora da pesquisa.
O botão de limpar ao lado remove todas as marcações. Os marcados ficam em uma
seção própria no topo, com rolagem independente e ordem mais recente primeiro;
a busca afeta apenas os demais trabalhos. Essa ordem de apresentação não altera
os bytes, os valores nem a ordem original dos registros exportados.

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

O botão de seta circular ao lado de **Abrir FC2** recarrega o caminho atual do
disco, sem abrir o seletor. A aplicação não monitora alterações no disco em
segundo plano: importe novamente ou use **Recarregar** após substituir o FC2.
Cada leitura substitui os dados e limpa buscas, marcações e tabelas anteriores.
O rodapé informa o caminho e as quantidades carregadas; o nome no cabeçalho
mostra o caminho completo e SHA-256 ao passar o cursor. Se a nova leitura tiver
conteúdo idêntico ao anterior, o rodapé avisa para conferir o arquivo escolhido.
Arquivos do equipamento podem manter o mesmo nome e tamanho mesmo após novas
medições; a identificação do conteúdo usa SHA-256.
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
- OpenStreetMap inicia habilitado, conforme solicitado pelo usuário, e pode ser
  desativado nas configurações. A consulta revela a região dos tiles;
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
