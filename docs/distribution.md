# Distribuição portátil

O workflow **Portable distribution** (`.github/workflows/distribution.yml`) gera
dois artefatos: Windows x64 portátil e Linux x86_64 AppImage. Após enviar os
arquivos do projeto ao GitHub, abra Actions → Portable distribution → Run workflow.
Também é acionado por tags `v*`. Os resultados ficam nos artefatos da execução;
o workflow não publica Releases nem envia arquivos experimentais.

## Windows

Com Rust stable >= 1.95, Visual Studio C++ Build Tools e Windows SDK instalados:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\build-portable-windows.ps1
.\local\portable-windows\fc2-explorer.exe
```

Compartilhe só o EXE. GUI, TUI, fontes e ícone estão incorporados; o runtime C
MSVC é estático. O destino não precisa de Rust, Leaf ou pasta assets.

## Linux

Use Ubuntu 22.04 x86_64 como ambiente de construção (é também o ambiente do CI).
Rust deve ser instalado via rustup. Um binário compilado no NixOS pode conter
caminhos `/nix/store`; o script rejeita esse binário em vez de apresentar um
pacote que depende da instalação de origem. Use o workflow no GitHub ou um
ambiente Ubuntu separado para empacotar a partir do NixOS.

No Ubuntu, instale as ferramentas de construção:

```sh
sudo apt-get update
sudo apt-get install -y curl binutils file pkg-config desktop-file-utils ripgrep squashfs-tools libwayland-dev libxkbcommon-dev libxkbcommon-x11-dev libgl1-mesa-dev libdbus-1-dev libx11-dev libxcursor-dev libxi-dev libxrandr-dev
bash scripts/build-appimage.sh
chmod +x local/appimage/FC2-Explorer-x86_64.AppImage
./local/appimage/FC2-Explorer-x86_64.AppImage
```

### Compilar localmente usando Docker

No terminal do host, use o script que instala os pré-requisitos e gera o pacote:

```sh
bash scripts/build-appimage-docker.sh
```

O comando completo equivalente está abaixo.

Na raiz do projeto, este comando compila em Ubuntu 22.04 sem usar a toolchain
do host. A cópia de fontes inclui `licenses`, pois os avisos de licença são
incorporados por `include_str!` durante a compilação da GUI e CLI.

```sh
mkdir -p local/appimage
docker run --rm -it --platform linux/amd64 \
  -v "$PWD:/source:ro" -v "$PWD/local/appimage:/out" \
  -e FC2_UID="$(id -u)" -e FC2_GID="$(id -g)" \
  ubuntu:22.04 bash -ceu '
    apt-get update
    apt-get install -y build-essential curl ca-certificates binutils file pkg-config desktop-file-utils squashfs-tools libwayland-dev libxkbcommon-dev libxkbcommon-x11-dev libgl1-mesa-dev libdbus-1-dev libx11-dev libxcursor-dev libxi-dev libxrandr-dev
    curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs -o /tmp/rustup-init.sh
    sh /tmp/rustup-init.sh -y --profile minimal
    export PATH="/root/.cargo/bin:$PATH"
    mkdir -p /build
    tar -C /source --exclude=target --exclude=gui/target \
      -cf - Cargo.toml Cargo.lock .cargo src gui assets scripts packaging LICENSE licenses \
      | tar -xf - -C /build
    cd /build
    bash scripts/build-appimage.sh
    cp local/appimage/FC2-Explorer-x86_64.AppImage* /out/
    chown "$FC2_UID:$FC2_GID" /out/FC2-Explorer-x86_64.AppImage*
  '
```

O arquivo final aparece em `local/appimage/FC2-Explorer-x86_64.AppImage` no host.
Este procedimento Docker não foi executado integralmente na sessão do agente.

O script baixa linuxdeploy da distribuição oficial `continuous` na primeira
execução e reutiliza a ferramenta em `local/appimage-tools`. Registra seu
SHA-256; essa ferramenta pode variar entre downloads. Para fixar uma ferramenta
obtida anteriormente, use `FC2_LINUXDEPLOY=/caminho/linuxdeploy.AppImage`.
Cargo continua usando o lockfile do projeto. O pacote final recebe um arquivo
SHA-256 ao lado do AppImage. Fontes experimentais não são incluídas.

O empacotador é extraído com `unsquashfs` e seu `AppRun` é executado
diretamente, antes da compilação. Assim o build não depende de FUSE nem da
execução de um AppImage por um lançador do host, que pode não enxergar caminhos
como `/build` dentro do container. O AppImage original da ferramenta é preservado.

A montagem segue o [guia oficial linuxdeploy](https://docs.appimage.org/packaging-guide/from-source/linuxdeploy-user-guide.html).
Além das dependências ELF, o script solicita explicitamente DBus carregado
dinamicamente. Wayland, xkbcommon, X11/XCB, OpenGL/EGL, GBM e DRM são excluídos:
essas bibliotecas devem acompanhar os drivers e dados de teclado do sistema
de destino. O AppDir é verificado antes da criação do pacote para impedir
inclusão acidental dessas bibliotecas. Ícone e licença ficam
no AppDir. O pacote contém a mesma GUI e TUI; o botão TUI reabre o arquivo
AppImage externo, sem depender da montagem temporária da GUI.

O destino precisa de Linux x86_64 compatível com a glibc do build, bibliotecas
gráficas/de teclado da sessão e drivers
gráficos, sessão X11/Wayland, portal desktop para os diálogos e um terminal
instalado para o botão TUI. O mapa usa internet. Um AppImage não garante suporte
a qualquer distribuição: a [documentação oficial](https://docs.appimage.org/introduction/concepts.html)
recomenda construir num sistema compatível com o mais antigo que se quer suportar.
Ubuntu 22.04 é a base planejada; compatibilidade entre distribuições ainda precisa
ser testada. A execução no NixOS também permanece pendente de validação.

Se o sistema não permite FUSE, tente a execução com extração:

```sh
APPIMAGE_EXTRACT_AND_RUN=1 ./FC2-Explorer-x86_64.AppImage
```

## Estado da validação

Na primeira imagem gerada pelo usuário, o Mesa 26.2.4 do host tentou usar
Wayland do Ubuntu incluído no pacote. `ldd -r` identificou símbolos ausentes:
`wl_fixes_interface`, `wl_display_create_queue_with_name` e
`wl_display_dispatch_queue_timeout`. A cópia antiga de xkbcommon também não
reconhecia `dead_hamza` nos dados Compose mais novos do host. O build foi
corrigido para excluir a pilha gráfica/de teclado do pacote. Uma cópia
temporária do AppDir, sem essas bibliotecas, resolveu as dependências do Mesa
sem símbolos indefinidos, usando Wayland 1.26 do host. A abertura da janela com
o AppImage reconstruído ainda precisa de confirmação na sessão desktop.
Esse conflito também é documentado em um
[empacotador de referência](https://github.com/moonlight-stream/moonlight-qt/blob/master/scripts/build-appimage.sh).

A sintaxe Bash e a integração Rust foram verificadas localmente. A extração
do linuxdeploy usando o offset ELF/SquashFS foi executada no binário oficial
disponível: `--version` e `--list-plugins` passaram, com o plugin AppImage
encontrado. O teste do
lançador TUI cobre a seleção do AppImage externo e o fallback para execução
normal. A geração completa no container e a execução dos pacotes finais ainda
precisam ser confirmadas. A validação final deve
abrir FC2, testar diálogos/exportação, abrir TUI e fechar a GUI mantendo a TUI
ativa em máquinas Windows e Linux sem Rust/Leaf instalados.
