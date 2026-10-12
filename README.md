# FC2-RS

Ferramenta independente em Rust 2024 para inspecionar arquivos Falker FC2,
recuperar seus trabalhos ativos e exportar medições sem depender do Falker Leaf.
Licença [MIT](LICENSE). Não é um produto da Falker.

## Estado

O parser reconhece uma variante documentada do contêiner FS2 e valida a seleção
dos arquivos ativos antes da exportação. Compatibilidade com outros modelos,
firmwares e versões do software requer referências independentes adicionais.
Arquivos e valores experimentais não fazem parte desta distribuição.

## Compilar e instalar

Requer Rust/Cargo com suporte à edição 2024 e um linker da plataforma.
A compilação local foi feita com Rust 1.98.1; o mínimo efetivo de todas as
dependências não foi testado. `rust-version = "1.85"` indica a base da edição,
não certifica uma execução testada nessa versão.

```bash
cargo build --release --locked
cargo install --path . --locked
fc2 --help
```

Sem instalação, use `./target/release/fc2` no Linux/macOS ou
`target\release\fc2.exe` no Windows. Compile para o sistema de destino;
o executável construído no NixOS não foi certificado como portátil para Ubuntu.
O programa Rust não exige Node, Python, Excel nem bibliotecas da Falker.
O script opcional de normalização do XLSX requer Node.js.

### Distribuir como um único arquivo

O HTML de downloads está embutido no executável. Para usar a aplicação compilada,
não são necessários o código-fonte, Cargo, Node, Python, Excel ou Falker Leaf.
Compartilhe um executável específico para cada sistema e arquitetura.
As licenças da aplicação e dos ícones também estão embutidas: `fc2 license`.

No Windows, `cargo build --release --locked --target x86_64-pc-windows-msvc` usa o runtime
MSVC estático configurado em `.cargo/config.toml`. Compartilhe
`target/x86_64-pc-windows-msvc/release/fc2.exe`; iniciar sem argumentos
abre a TUI, e um caminho FC2 pode ser passado diretamente ao executável.

No Linux, o build portátil usa musl (instale o linker musl antes de compilar):

```bash
rustup target add x86_64-unknown-linux-musl
cargo build --release --locked --target x86_64-unknown-linux-musl
```

Compartilhe somente `target/x86_64-unknown-linux-musl/release/fc2` e mantenha sua
permissão de execução. `.github/workflows/binaries.yml` prepara esses builds
Windows/Linux quando acionado manualmente ou por uma tag `v*`; não publica dados
experimentais. Windows e os artefatos da automação ainda requerem execução real.

## Listar e baixar todos os trabalhos

### Interface no terminal (TUI)

```bash
./target/release/fc2 tui arquivo.FC2
# Com comparação externa do trabalho de referência
./target/release/fc2 tui arquivo.FC2 --profile local/perfil.json
# Sem argumento: abre um navegador de arquivos locais
./target/release/fc2 tui
# Também abre a TUI, sem subcomando
./target/release/fc2
```

A TUI abre com contagens, período, ausências e resumo do trabalho selecionado.
Permite buscar enquanto digita, navegar grupos e medições, examinar médias e
faixas por ponto, consultar avisos e revelar bytes/offsets sob demanda.
`:` abre o menu de ações; `i` mostra detalhes; `6` mostra pontos; `w` mostra
avisos. `e` exporta XLSX do trabalho selecionado; `E` exporta todos;
`?` mostra a ajuda. Também valida referências e exporta ZIP, backup bruto e JSON.
`Espaço` marca vários trabalhos; `A` marca os visíveis e `U` limpa as marcas.
`M` alterna completo/simples; `e` exporta os marcados (ou o atual sem marcas).
Simples é o padrão e gera somente XLSX; completo inclui os acompanhantes de auditoria.
As saídas usam os mesmos controles da CLI, sem sobrescrita.
`e` e `E` abrem um seletor de pastas em Downloads (ou na pasta equivalente
configurada no sistema). Enter entra na pasta destacada; `s` seleciona a pasta
atual. Confirme o caminho da subpasta nova antes de exportar. `D` volta a Downloads,
`h` abre a pasta pessoal e `d` permite informar outro caminho ou unidade Windows.
O destino escolhido é reutilizado enquanto a aplicação estiver aberta.
Requer terminal ANSI/VT interativo; pipes devem usar os outros comandos CLI.
Consulte [atalhos e fluxo completo](docs/tui.md).

```bash
fc2 works arquivo.FC2
fc2 export arquivo.FC2
fc2 export arquivo.FC2 --mode complete --output resultados
fc2 serve resultados --port 8712
```

Abra `http://127.0.0.1:8712`. A página lista os trabalhos, permite buscar por
nome/número e baixar cada XLSX ou o ZIP completo. Também é possível abrir
`resultados/index.html` diretamente no navegador, sem iniciar o servidor.
Não há CDN, upload, conta ou envio dos dados à internet. A seleção do FC2 é
feita pelo argumento da CLI; esta versão da página não tem formulário de upload.

O diretório de saída deve ser novo. Arquivos existentes não são sobrescritos.
Sem `--output`, a CLI cria uma subpasta nova em Downloads com nome baseado no FC2.
Falhas de um trabalho são registradas no manifesto e bloqueiam seu XLSX;
os demais resultados continuam disponíveis, com saída da CLI diferente de zero.
Falhas do contêiner, CRC ou árvore de arquivos bloqueiam a extração.

```bash
# Exportar somente um trabalho, pelo número do diretório
fc2 export arquivo.FC2 --work 135 --output resultado135
fc2 export arquivo.FC2 --work 135 --work 138 --mode simple --output somente-xlsx

# Preservar a ordem do arquivo em cada aba
fc2 export arquivo.FC2 --order stored --output resultados-origem

# Auditoria estruturada da leitura ativa
fc2 works arquivo.FC2 --output trabalhos.json

# Cópia integral: original, memória reconstruída e todos os arquivos ativos
fc2 extract arquivo.FC2 --output dados-brutos.zip
```

`--work` restringe planilhas, JSON de dados e arquivos brutos à seleção; essa
exportação parcial não inclui a imagem original nem a memória integral.
Os hashes e contagens gerais do sistema de arquivos continuam identificando
a fonte completa. `extract` e a exportação de todos os trabalhos incluem a
cópia integral, preservando inclusive regiões ainda não interpretadas.

## Como fica o XLSX

O layout conserva as colunas do exemplo em B8:J8: Medição, Ponto, Hora,
Data, Latitude, Longitude, Clorofila A, Clorofila B e Clorofila Total.
Um trabalho com um grupo usa a aba `Worksheet`; vários grupos usam abas
`Medidas 1`, `Medidas 2`, etc. Pontos de grupos diferentes não são misturados.
O cabeçalho identifica FC2-RS como o programa que gerou o arquivo.

Por padrão, os pontos são ordenados lexicograficamente, como na referência
(`1, 10, 11, …, 2`); a ordem original dentro de cada ponto é preservada.
O manifesto documenta a ordenação; o JSON de auditoria conserva a ordem do
arquivo. Outros modos de ordenação do Leaf ainda não foram comparados.

Índices em décimos mantêm os inteiros originais, sem recalcular o total.
Coordenadas mantêm o valor f32 promovido a f64; o formato da célula mostra
seis casas, sem arredondar o valor armazenado durante a decodificação.
Datas/horas zeradas e coordenadas NaN ficam vazias. Fuso horário, ponto de
referência e horário de geração não comprovados não são inventados.

Um trabalho não possui configuração com nome: o identificador do diretório
é mostrado com aviso, inclusive no cabeçalho da planilha.
O JSON complementar guarda bits, bytes, hashes, campos desconhecidos e
segmentos com offsets FC2 e endereços da memória de cada registro.
Consulte [o contrato de exportação](docs/xlsx-export.md).

## Inspeção binária

```bash
fc2 inspect arquivo.FC2
fc2 hex arquivo.FC2 --offset 0x0 --length 512
fc2 analyze arquivo.FC2 --output analysis.json
fc2 batch ./arquivos --output ./resultados-lote
```

`inspect` e `analyze` também descrevem entradas desconhecidas ou malformadas;
não declaram medições científicas válidas só pela presença de uma assinatura.
`batch` inspeciona FC2 imediatamente contidos na pasta, em ordem determinística,
mantendo hash/nome e isolando erros. Não faz busca recursiva.
Limites explícitos: entrada regular até 64 MiB; memória reconstruída até 8 MiB;
o sistema de arquivos atualmente exige a geometria observada de 1 MiB.

## Comparação explícita com uma referência Leaf

A referência deve corresponder exatamente ao mesmo FC2, sem edição dos
valores. O helper abaixo aceita somente o layout XLSX inspecionado, com uma
aba e valores simples; outros layouts exigem adaptação documentada.

```bash
mkdir -p local
node scripts/reference-from-xlsx.mjs referencia.xlsx arquivo.FC2 local/reference.json --confirm-same-source
fc2 validate arquivo.FC2 --reference local/reference.json --reference-source referencia.xlsx --output local/perfil.json --report local/validation-report.json
fc2 export arquivo.FC2 --mode complete --profile local/perfil.json --output local/trabalhos
```

O perfil vincula hashes, seleção e comparação. `export --profile` verifica
também se as medições correspondem aos arquivos **ativos** do trabalho;
uma cópia histórica coincidente não valida um registro ativo divergente.
O perfil não comprova os demais trabalhos.

```bash
fc2 decode arquivo.FC2 --profile local/perfil.json --order reference --output local/medicoes.csv
fc2 decode arquivo.FC2 --profile local/perfil.json --order reference --format json --output local/medicoes.json
fc2 decode arquivo.FC2 --profile local/perfil.json --order reference --format xlsx --output local/medicoes.xlsx
```

`decode` exige perfil e o revalida a cada execução. CSV usa UTF-8, separador
vírgula, ponto decimal e LF. JSON conserva índices como strings decimais exatas,
além dos inteiros em décimos. O total é um campo armazenado independente de A+B.

## Validação e contribuição

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --locked
cargo build --release --locked

node scripts/verify-workbooks.mjs arquivo.FC2 referencia.xlsx
```

Os testes usam fixtures sintéticas e não precisam de internet. O verificador
opcional de XLSX aceita caminhos locais informados como argumentos. Consulte
[critérios e método de validação](docs/validation.md).

Biblioteca modular: contêiner (`parser`), árvore/CRC (`filesystem`), registros
(`work_records`), trabalhos (`workspace`), comparação (`validator`), exportação
e downloads. `unsafe` é proibido. Dependências diretas: clap, serde, serde_json,
sha2, thiserror, tempfile e console. A crate console fornece teclado e terminal;
XLSX/ZIP são produzidos sem dependências adicionais.

Antes de contribuir com novos offsets ou variantes, forneça evidências,
hashes e comparação independente autorizada. Não publique dados experimentais
sem permissão. Crie fixtures sintéticas para falhas e documente compatibilidade
por modelo/firmware/amostra, sem transformar um exemplo em especificação geral.

## O que permanece desconhecido

Parte dos bytes de sensores/calibração, campo 34, configurações globais,
época dos timestamps de diretórios e algumas estruturas de exclusão ainda
não têm significado comprovado. Firmware e regiões reservadas são preservados
como bytes, não totalmente interpretados. Modelo/firmware exatos e versão do
Leaf não foram confirmados externamente. Windows e macOS ainda precisam de
execução real dos testes; Linux/NixOS é a plataforma verificada nesta sessão.

[Análise binária](docs/binary-analysis.md),
[especificação observada](docs/format-specification.md),
[sistema de arquivos](docs/filesystem-investigation.md),
[registros e metadados](docs/records-investigation.md),
[fontes](docs/references.md) e
[lacunas para compatibilidade geral](docs/remaining-work.md).
