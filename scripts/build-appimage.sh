#!/usr/bin/env bash
# Build on Ubuntu 22.04 x86_64 with rustup/MSVC-independent Linux Rust.
set -euo pipefail
project_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "$project_dir"
if [[ $(uname -s) != Linux || $(uname -m) != x86_64 ]]; then
    printf '%s\n' 'Este script gera AppImage Linux x86_64.' >&2
    exit 1
fi
for tool in cargo curl readelf ldconfig sha256sum install mktemp awk unsquashfs file find; do
    command -v "$tool" >/dev/null || { printf 'Ferramenta ausente: %s\n' "$tool" >&2; exit 1; }
done
output_dir="$project_dir/local/appimage"
tools_dir="$project_dir/local/appimage-tools"
mkdir -p "$output_dir" "$tools_dir"
linuxdeploy=${FC2_LINUXDEPLOY:-"$tools_dir/linuxdeploy-x86_64.AppImage"}
if [[ ! -f "$linuxdeploy" ]]; then
    if [[ -n ${FC2_LINUXDEPLOY:-} ]]; then
        printf 'FC2_LINUXDEPLOY não encontrado: %s\n' "$linuxdeploy" >&2
        exit 1
    fi
    curl --fail --location --retry 3 'https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-x86_64.AppImage' --output "$linuxdeploy.download"
    chmod +x "$linuxdeploy.download"
    mv "$linuxdeploy.download" "$linuxdeploy"
fi
[[ -x "$linuxdeploy" ]] || { printf 'linuxdeploy não é executável: %s\n' "$linuxdeploy" >&2; exit 1; }
sha256sum "$linuxdeploy" > "$tools_dir/linuxdeploy.sha256"
# Execute the payload instead of the AppImage runtime. Host AppImage binfmt
# handlers can intercept exec inside Docker and try to resolve container paths
# from the host. A type-2 image appends SquashFS after the ELF section table.
image_header=$(LC_ALL=C readelf -h "$linuxdeploy")
section_start=$(awk '/Start of section headers:/ {print $5}' <<< "$image_header")
section_size=$(awk '/Size of section headers:/ {print $5}' <<< "$image_header")
section_count=$(awk '/Number of section headers:/ {print $5}' <<< "$image_header")
for value in "$section_start" "$section_size" "$section_count"; do
    [[ "$value" =~ ^[0-9]+$ ]] || { printf '%s\n' 'Cabeçalho ELF do linuxdeploy inválido.' >&2; exit 1; }
done
image_offset=$((section_start + section_size * section_count))
unsquashfs -s -o "$image_offset" "$linuxdeploy" >/dev/null
tool_dir=$(mktemp -d "$tools_dir/extracted-XXXXXXXX")
unsquashfs -no-progress -o "$image_offset" -d "$tool_dir/AppDir" "$linuxdeploy"
deploy_dir="$tool_dir/AppDir"
[[ -x "$deploy_dir/AppRun" ]] || { printf '%s\n' 'O linuxdeploy extraído não contém AppRun executável.' >&2; exit 1; }
APPDIR="$deploy_dir" "$deploy_dir/AppRun" --version
cargo build --manifest-path gui/Cargo.toml --release --locked --target x86_64-unknown-linux-gnu --target-dir local/appimage-target
binary="$project_dir/local/appimage-target/x86_64-unknown-linux-gnu/release/fc2-explorer"
# Nix ELF interpreters and store paths cannot be shipped as an ordinary AppImage.
elf_metadata=$(readelf -l -d "$binary")
if [[ "$elf_metadata" == *'/nix/store/'* ]]; then
    printf '%s\n' 'O binário depende do /nix/store. Gere o AppImage com Rust via rustup no Ubuntu ou pelo workflow Portable distribution.' >&2
    exit 1
fi
build_dir=$(mktemp -d "$output_dir/build-XXXXXXXX")
app_dir="$build_dir/AppDir"
install -Dm755 "$binary" "$app_dir/usr/bin/fc2-explorer"
install -Dm644 LICENSE "$app_dir/usr/share/licenses/fc2-explorer/LICENSE"
install -Dm644 licenses/Phosphor-Icons-MIT.txt "$app_dir/usr/share/licenses/fc2-explorer/Phosphor-Icons-MIT.txt"
install -Dm644 assets/app_icon.svg "$build_dir/fc2-explorer.svg"
# rfd loads DBus dynamically. Graphics/input libraries must match the host's
# EGL/Mesa and keyboard data: shipping Jammy Wayland breaks newer GPU drivers.
libraries=(libdbus-1.so.3)
host_libraries=('libwayland*.so*' 'libxkbcommon*.so*' 'libxcb*.so*' 'libX*.so*' 'libGL*.so*' 'libEGL*.so*' 'libOpenGL*.so*' 'libgbm*.so*' 'libdrm*.so*')
exclude_args=()
for pattern in "${host_libraries[@]}"; do
    exclude_args+=(--exclude-library "$pattern")
done
library_args=()
library_cache=$(ldconfig -p)
for library in "${libraries[@]}"; do
    library_path=$(awk -v name="$library" '$1 == name && /x86-64/ { print $NF; exit }' <<< "$library_cache")
    [[ -n "$library_path" ]] || { printf 'Biblioteca ausente para empacotamento: %s\n' "$library" >&2; exit 1; }
    library_args+=(--library "$library_path")
done
cd "$output_dir"
APPDIR="$deploy_dir" \
    "$deploy_dir/AppRun" --appdir "$app_dir" --executable "$binary" \
    --desktop-file "$project_dir/packaging/fc2-explorer.desktop" \
    --icon-file "$build_dir/fc2-explorer.svg" \
    "${library_args[@]}" "${exclude_args[@]}"
# Verify the staging directory before publishing the final image.
for pattern in "${host_libraries[@]}"; do
    if [[ -n $(find "$app_dir/usr/lib" -name "$pattern" -print -quit) ]]; then
        printf 'Biblioteca gráfica do host incluída indevidamente: %s\n' "$pattern" >&2
        exit 1
    fi
done
APPDIR="$deploy_dir" LDAI_OUTPUT="$output_dir/FC2-Explorer-x86_64.AppImage" \
    "$deploy_dir/AppRun" --appdir "$app_dir" "${exclude_args[@]}" --output appimage
test -s "$output_dir/FC2-Explorer-x86_64.AppImage"
chmod +x "$output_dir/FC2-Explorer-x86_64.AppImage"
sha256sum FC2-Explorer-x86_64.AppImage > FC2-Explorer-x86_64.AppImage.sha256
printf 'AppImage: %s/FC2-Explorer-x86_64.AppImage\n' "$output_dir"
