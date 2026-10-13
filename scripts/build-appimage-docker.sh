#!/usr/bin/env bash
# Ubuntu build isolated from the host toolchain and existing Cargo artifacts.
set -euo pipefail
project_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
command -v docker >/dev/null || { printf '%s\n' 'Docker não encontrado.' >&2; exit 1; }
mkdir -p "$project_dir/local/appimage"
docker run --rm -it --platform linux/amd64 \
    -v "$project_dir:/source:ro" \
    -v "$project_dir/local/appimage:/out" \
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
