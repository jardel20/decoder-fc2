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
sudo apt-get install -y curl binutils pkg-config desktop-file-utils ripgrep libwayland-dev libxkbcommon-dev libxkbcommon-x11-dev libgl1-mesa-dev libdbus-1-dev libx11-dev libxcursor-dev libxi-dev libxrandr-dev
bash scripts/build-appimage.sh
chmod +x local/appimage/FC2-Explorer-x86_64.AppImage
./local/appimage/FC2-Explorer-x86_64.AppImage
```

O script baixa linuxdeploy da distribuição oficial `continuous` na primeira
execução e reutiliza a ferramenta em `local/appimage-tools`. Registra seu
SHA-256; essa ferramenta pode variar entre downloads. Para fixar uma ferramenta
obtida anteriormente, use `FC2_LINUXDEPLOY=/caminho/linuxdeploy.AppImage`.
Cargo continua usando o lockfile do projeto. O pacote final recebe um arquivo
SHA-256 ao lado do AppImage. Fontes experimentais não são incluídas.

A montagem segue o [guia oficial linuxdeploy](https://docs.appimage.org/packaging-guide/from-source/linuxdeploy-user-guide.html).
Além das dependências ELF, o script solicita explicitamente bibliotecas de
DBus, X11, Wayland e xkbcommon carregadas dinamicamente. Ícone e licença ficam
no AppDir. O pacote contém a mesma GUI e TUI; o botão TUI reabre o arquivo
AppImage externo, sem depender da montagem temporária da GUI.

O destino precisa de Linux x86_64 compatível com a glibc do build, drivers
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

A sintaxe Bash e a integração Rust foram verificadas localmente. O teste do
lançador TUI cobre a seleção do AppImage externo e o fallback para execução
normal. O empacotamento real requer linuxdeploy e acesso de rede, indisponíveis
na sessão atual; o workflow ainda não foi executado. A validação final deve
abrir FC2, testar diálogos/exportação, abrir TUI e fechar a GUI mantendo a TUI
ativa em máquinas Windows e Linux sem Rust/Leaf instalados.
