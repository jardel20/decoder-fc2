#!/usr/bin/env bash
# Run from a normal terminal with network access and desktop build libraries.
set -euo pipefail
project_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd -- "$project_dir"
log_dir="$project_dir/local/desktop-validation/$(date +%Y%m%d-%H%M%S)-$$"
mkdir -p -- "$log_dir"
exec > >(tee "$log_dir/build.log") 2>&1
trap 'status=$?; if (( status != 0 )); then printf "\nFalha (código %s). Log: %s/build.log\n" "$status" "$log_dir"; fi' EXIT
printf 'Validação desktop FC2 Explorer\nProjeto: %s\nLog: %s/build.log\n' "$project_dir" "$log_dir"
rustc --version
cargo --version
desktop_target="$(rustc -vV | sed -n 's/^host: //p')"
if [[ -z "$desktop_target" ]]; then
    printf 'Não foi possível identificar o target do compilador.\n' >&2
    exit 1
fi
run_step() {
    printf '\nEtapa: %s\n' "$1"
    shift
    "$@"
}
run_step 'Dependências do núcleo (lock existente)' cargo fetch --locked --target "$desktop_target"
if [[ -f gui/Cargo.lock ]]; then
    run_step 'Dependências GUI (lock existente)' cargo fetch --manifest-path gui/Cargo.toml --locked --target "$desktop_target"
else
    run_step 'Dependências GUI e primeiro lockfile' cargo fetch --manifest-path gui/Cargo.toml --target "$desktop_target"
fi
run_step 'Formatação do núcleo' cargo fmt --all -- --check
run_step 'Formatação GUI' cargo fmt --manifest-path gui/Cargo.toml --all -- --check
run_step 'Verificação do núcleo' cargo check --workspace --locked --target "$desktop_target"
run_step 'Testes do núcleo e estado GUI' cargo test --workspace --locked --target "$desktop_target"
run_step 'Clippy do núcleo' cargo clippy --workspace --all-targets --locked --target "$desktop_target" -- -D warnings
run_step 'Verificação GUI' cargo check --manifest-path gui/Cargo.toml --locked --target "$desktop_target"
run_step 'Testes GUI' cargo test --manifest-path gui/Cargo.toml --locked --target "$desktop_target"
run_step 'Clippy GUI' cargo clippy --manifest-path gui/Cargo.toml --all-targets --locked --target "$desktop_target" -- -D warnings
run_step 'Executável CLI/TUI' cargo build --release --locked --target "$desktop_target" --target-dir target
run_step 'Executável GUI' cargo build --manifest-path gui/Cargo.toml --release --locked --target "$desktop_target" --target-dir gui/target
printf '\nVerificações concluídas. GUI: %s/gui/target/%s/release/fc2-explorer\nLog: %s/build.log\n' "$project_dir" "$desktop_target" "$log_dir"
printf 'A janela e a portabilidade ainda precisam dos testes manuais em docs/desktop-validation.md.\n'
