# Validação

O projeto valida as etapas estruturais e científicas separadamente. Testes
sintéticos cobrem entradas malformadas, limites, checksums, encadeamento,
proveniência, campos ausentes e serialização. Fixtures artificiais testam o
programa; não demonstram compatibilidade com um equipamento real.

Durante o desenvolvimento, uma exportação independente do software do fabricante
foi usada para validar campos de uma amostra privada. A amostra, sua exportação,
identificadores, valores, coordenadas e hashes permanecem fora do repositório.
Não se declara equivalência para campos ou trabalhos sem comparação independente.

## Reproduzir

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --locked
cargo build --release --locked
```

Para validar um arquivo próprio, use `fc2 validate` com uma referência do mesmo
arquivo, não editada. O perfil gerado fica vinculado aos hashes das entradas e é
revalidado antes da exportação. Ausências permanecem vazias e diferenças são
reportadas sem corrigir a fonte.
