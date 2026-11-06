# FC2 Explorer com Tauri

A interface em `desktop/` usa Tauri 2 e mantém a biblioteca `fc2-rs` como única
implementação de leitura e exportação. A interface anterior em `gui/` continua
acessível durante a migração. CLI, TUI e seus contratos de exportação permanecem
independentes da nova interface.

## Direção visual

A referência visual é o [instalador do Ubuntu](https://ubuntu.com/blog/how-we-designed-the-new-ubuntu-desktop-installer/):
controles simples, superfícies neutras, destaque laranja e tipografia Ubuntu
incorporada ao aplicativo. Não há vínculo com a Canonical ou com o Ubuntu.

A organização segue o explorador da GUI original: lista de trabalhos à esquerda,
resumo compacto e dados à direita. A área de trabalho ocupa toda a janela, sem
fluxo obrigatório por etapas ou telas promocionais. O conteúdo usa rolagem
própria e mantém as ações de abertura e exportação acessíveis.

- Trabalhos e medições simultâneos; selecionar um trabalho para visualizar não altera as caixas de exportação.
- Busca por nome/ID do trabalho e por ID/ponto/data/hora nas medições.
- Filtros de grupo e de registros com avisos, visíveis também nos gráficos e nas coordenadas.
- Ordenação por coluna na tabela, paginação, latitude/longitude e inspeção dos bytes de cada registro.
- Resumo com medições, grupos, pontos, coordenadas, avisos e período do trabalho.
- Metadados, SHA-256, configuração, limitações e avisos da leitura.
- Exportação direta em diálogo: XLSX simples, pacote completo ou JSON de um trabalho.
- Temas claro/escuro, atalhos, abertura por arrastar arquivo e argumento de linha de comando.

Os gráficos respeitam os filtros e mostram registros na ordem de origem,
independentemente da ordenação da tabela, sem inferir
réplicas ou médias. Grupos mantêm sua identidade; o total de clorofila não é
recalculado a partir de A+B. O gráfico de coordenadas ajusta os eixos
independentemente, não representa distâncias e informa o datum desconhecido.

## Desenvolvimento

Use Node.js 22.12+ e Rust estável recente. A crate declara Rust 1.90 como base;
a validação local usou Rust/Cargo 1.99, sem certificação do mínimo declarado.
No Linux, instale as dependências nativas do
[Tauri 2](https://v2.tauri.app/start/prerequisites/), incluindo GTK 3, WebKitGTK 4.1,
D-Bus e ferramentas de compilação. No Windows, são necessários MSVC/Windows SDK
e WebView2.

```bash
cd desktop
npm ci
npm run tauri dev
```

O servidor de desenvolvimento escuta somente em `127.0.0.1:1420`. A aplicação
compilada incorpora o frontend e não depende desse servidor nem de Node.js.

```bash
# Somente prévia visual no navegador, sem acesso aos arquivos nativos
npm run dev

# Executável desktop, sem gerar instaladores
npm run tauri build -- --no-bundle

# Pacotes para o sistema de compilação
npm run tauri build
```

O executável Linux fica em `desktop/src-tauri/target/release/fc2-desktop` e aceita
um caminho FC2 como primeiro argumento. Linux, Windows e macOS exigem compilação
no ambiente correspondente. Os instaladores configurados são DEB, RPM, AppImage
e NSIS; os pacotes finais ainda precisam de teste nas plataformas de destino.
Os scripts antigos de distribuição em `scripts/` continuam empacotando a GUI egui.

Em ambientes Nix, as bibliotecas podem existir no store sem estarem expostas ao
`pkg-config`. Entre em um ambiente de desenvolvimento que exponha GTK 3,
WebKitGTK 4.1, D-Bus, Wayland e seus arquivos `.pc`. A configuração do sistema
não precisa ser alterada pela aplicação.

## Integração e segurança dos dados

`desktop/src-tauri/src/lib.rs` contém os modelos de apresentação e os contratos
com o decoder. `runtime.rs` registra quatro comandos locais: `open_source`,
`get_work`, `export_selection` e `initial_path`. A implementação segue o
[IPC do Tauri 2](https://v2.tauri.app/develop/calling-rust/).

Leitura e exportação usam tarefas em segundo plano. A sessão mantém uma origem
imutável em `Arc<LoadedDataset>`; detalhes e exportação verificam seu SHA-256 para
rejeitar seleções de outra origem. A exportação reutiliza os controles existentes
de cobertura, integridade e escrita sem sobrescrita. O seletor de pasta cria um
novo subdiretório para XLSX/pacote completo; JSON usa um seletor de arquivo e
também recusa sobrescrita. Falhas parciais são mostradas com os trabalhos bloqueados.

O frontend tem uma política CSP local e não recebe permissões genéricas de
filesystem ou shell. Não há upload, telemetria, fontes remotas, CDN ou mapas
externos. Os testes automatizados e as capturas versionadas usam exclusivamente dados sintéticos.

## Verificação

Na raiz do repositório:

```bash
cargo test --locked
cargo fmt --manifest-path desktop/src-tauri/Cargo.toml -- --check
cargo test --manifest-path desktop/src-tauri/Cargo.toml --locked --no-default-features
cargo clippy --manifest-path desktop/src-tauri/Cargo.toml --locked --all-targets -- -D warnings
```

Dentro de `desktop/`:

```bash
npm run format:check
npm run build
npx playwright install chromium
npm test
```

Para usar um Chrome já instalado, informe `FC2_CHROME_PATH=/caminho/google-chrome`.
Os testes de navegador simulam a fronteira IPC: verificam o fluxo e a apresentação,
mas não substituem testes dos diálogos nativos. Os testes Rust usam um FC2
sintético completo e verificam leitura, grupos, ausências, total independente,
identidade da fonte, seleção inválida e recusa de sobrescrita.

Validação local: build de produção do frontend, testes do decoder, testes da
ponte Rust, testes de navegador, Clippy e compilação nativa Linux. O compositor
confirmou uma janela `fc2-desktop` com título `FC2 Explorer`. A interação manual
com os seletores nativos e os instaladores Windows/Linux não está certificada.

Os seis testes de interface cobrem o layout em 800 × 620, 1240 × 820 e
1900 × 1026, busca, filtros, ordenação, inspeção, exportação e recuperação de erros.
Em 1240 × 820, pelo menos 15 registros ficam visíveis junto à lista de trabalhos,
sem rolagem externa da janela.

A fonte Ubuntu Regular/Medium está incorporada, sem modificações, sob Ubuntu Font
Licence 1.0. Origem e avisos estão em `desktop/public/fonts/`.

![Explorador compacto do FC2 Explorer](images/tauri-workspace.png)
