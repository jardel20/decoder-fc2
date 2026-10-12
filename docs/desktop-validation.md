# Concluir a validação fora do ambiente do agente

A GUI ainda não foi compilada nem executada neste ambiente: faltam dependências
no cache e o acesso ao registro do Cargo falha. Os scripts abaixo fazem os checks
completos no terminal normal, interrompem na primeira falha e guardam o log em
`local/desktop-validation/<data>/build.log`. Não abrem nem exportam dados FC2.
O primeiro `cargo fetch` da GUI cria `gui/Cargo.lock`; execuções seguintes exigem
esse lockfile. Inclua-o no controle de versão após uma compilação bem-sucedida.
Referência: [Cargo fetch](https://doc.rust-lang.org/cargo/commands/cargo-fetch.html).

## 1. Compilar e testar no NixOS atual

No terminal normal da sessão gráfica:

```bash
cd /home/jardel/Documents/GitHub/Decoder_FC2
nix develop /etc/nixos#rust-desktop
bash scripts/validate-desktop.sh
```

O shell `rust-desktop` já existe na configuração local e fornece as bibliotecas
desktop. Confira a versão de Rust apresentada: a GUI exige pelo menos 1.95.
Se for mais antiga, o log identificará isso; não altere a configuração do sistema
como parte deste roteiro. O script não instala toolchains nem pacotes globais.

Se concluir com sucesso, ainda dentro do mesmo shell, abra a GUI:

```bash
cargo run --manifest-path gui/Cargo.toml --locked -- caminho/arquivo.FC2
```

Para abrir sem arquivo inicial, omita `-- caminho/arquivo.FC2`. Para executar o
binário release produzido pelo script, use o caminho informado ao final dele.
No Linux x86_64 usual: `gui/target/x86_64-unknown-linux-gnu/release/fc2-explorer`.

## 2. Compilar e testar no Windows

Pré-requisitos: Rust stable **1.95 ou superior**, `rustfmt`, Clippy, ferramentas
MSVC C++ e Windows SDK. O instalador do Visual Studio oferece a carga
**Desenvolvimento para desktop com C++**; veja os
[pré-requisitos oficiais do Rust para MSVC](https://rust-lang.github.io/rustup/installation/windows-msvc.html).
Use um checkout que contenha estas alterações; o repositório remoto pode ainda
não conter os arquivos da GUI.

No PowerShell, na pasta do projeto, com Rust instalado via rustup:

```powershell
rustup update stable
rustup default stable
rustup component add rustfmt clippy
rustup target add x86_64-pc-windows-msvc
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\validate-desktop.ps1
```

`ExecutionPolicy Bypass` vale somente para esse processo, sem modificar a
política persistente. Para abrir o executável após a compilação:

```powershell
.\gui\target\x86_64-pc-windows-msvc\release\fc2-explorer.exe
```

Também aceita um FC2 como argumento. CLI/TUI continuam disponíveis em
`target\x86_64-pc-windows-msvc\release\fc2.exe`.

## 3. Testar a janela com um FC2 real

Registre plataforma, versão do Rust, resultado e mensagem de erro para cada item.
Estes são testes pendentes, não resultados já obtidos:

- Abrir a janela sem FC2; cancelar o diálogo e reabri-lo.
- Abrir o FC2 real; conferir trabalhos e contagens com a CLI do mesmo checkout.
- Pesquisar, ordenar, selecionar vários trabalhos e conferir o painel lateral.
- Abrir por botão e duplo clique; voltar preservando seleção e pesquisa.
- Percorrer Medições, Mapa, Metadados e Exportação; selecionar registros,
  pesquisar, filtrar por grupo e conferir bytes/offsets no inspetor.
- Alternar temas claro/escuro e redimensionar a janela.
- Conferir OpenStreetMap sempre habilitado, testar com e sem internet e verificar zoom,
  deslocamento e seleção. Ausência de coordenadas deve produzir estado vazio.
  Limites numéricos válidos não confirmam o datum nem a exatidão geográfica.
- Exportar um e vários trabalhos em modo simples: somente XLSX. Conferir destino
  sugerido Downloads e escolha de outra pasta. Conferir também modo completo e
  JSON estrutural, erros de escrita e ausência de sobrescrita silenciosa.
- Comparar o XLSX do trabalho coberto pelo exemplo com a referência existente.
  Essa referência não valida todos os demais trabalhos nem campos desconhecidos.
- Abrir arquivo inválido e confirmar mensagem de erro; reabrir um válido.
- Confirmar responsividade durante leitura/exportação e tratamento ao fechar.
- Verificar CLI e TUI: `fc2 --help` e `fc2 tui caminho/arquivo.FC2`.

Não envie FC2, XLSX de pesquisa ou screenshots contendo dados privados ao GitHub.
Os logs de compilação podem conter caminhos locais: revise antes de compartilhar.

## 4. Validar distribuição como arquivo único

No Windows, copie **somente** `fc2-explorer.exe` para outra pasta e depois para
um computador Windows sem Rust e sem Falker Leaf. Teste abertura, leitura e
exportação. O runtime C está configurado como estático; isso não substitui o
teste real do executável e dos diálogos no computador de destino.

No Linux, compilar no NixOS não prova portabilidade para Ubuntu. Teste o binário
fora do dev shell e em outra distribuição. Um ELF pode depender de bibliotecas
do sistema ou de caminhos `/nix/store`; a falha nesse teste exige trabalho de
empacotamento. Um AppImage seria uma alternativa de arquivo único, mas **não foi
implementado nem validado**. O workflow Ubuntu também não garante compatibilidade
com todas as distribuições ou versões de glibc.

## 5. Usar o GitHub para gerar Windows e Linux

O workflow `.github/workflows/gui.yml` oferece execução manual em **Actions →
FC2 Explorer desktop → Run workflow** depois que os arquivos forem enviados ao
repositório. Ele compila em Windows e Ubuntu, roda testes/lints e disponibiliza
os artefatos. Não exige publicar nem enviar o FC2. Reveja `git status` e o diff
preparado antes de commitar, incluindo somente código, documentos, scripts e
lockfiles. Este ambiente não pode escrever em `.git` nem executar a publicação.

Um workflow verde verifica compilação e testes automatizados; os testes físicos
da janela, dados reais e distribuição acima continuam necessários.

## 6. Retornar os resultados para correção

Se o script falhar, informe o caminho do `build.log` produzido (ou copie a
mensagem e as últimas linhas). O log permanece no projeto para análise local.
Não apague nem regenere lockfiles para tentar esconder uma falha: registre-a.
Após corrigir código, repita o script e atualize `docs/gui-validation.md` somente
com resultados efetivamente observados.
