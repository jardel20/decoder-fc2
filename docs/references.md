# Fontes e limites da pesquisa

Consulta em 2026-10-11. As fontes explicam o equipamento ou estruturas
comparáveis; a especificação dos campos do FC2 depende também dos bytes
inspecionados e da referência vinculada ao mesmo hash.

- [Manual oficial CFL2060, revisão A, setembro de 2022](https://www.falker.com.br/index.php/download?file=MAN_CFL2060_br.pdf&product=clorofilog):
  seções 3.3–3.5 descrevem trabalhos, pontos e medidas; 3.8 trata de GPS;
  3.10–3.11 descrevem Leaf e transferência FC2; 4.3 menciona atualização `.fs2`.
  Não fornece offsets binários nem valida os campos propostos neste projeto.
- [Suporte oficial Falker](https://www.falker.com.br/br/suporte):
  distingue o CFL2060 e o modelo CFL1030. Não estender compatibilidade entre eles.
- [Falker: uso de clorofiLOG e exportação pelo Leaf](https://www.falker.com.br/br/blog-como-usar-o-medidor-de-clorofila-para-recomendar-adubao-nitrogenada):
  descreve exportação CSV e médias; a referência deve conter medidas individuais.
- [Falker: unidade de medição](https://www.falker.com.br/base/clorofila/clorofilog-cfl1030/qual-e-a-unidade-de-medicao-do-clorofilog/):
  explica ICF como índice adimensional. Página do CFL1030: não prova o formato FC2
  nem uma calibração equivalente para CFL2060. Não converter índices em massa.
- [Especificação UF2 original](https://github.com/microsoft/uf2):
  estrutura de blocos de 512 bytes com cabeçalho de 32 bytes, contagem,
  endereço e payload. O primeiro magic UF2 difere do FS2 observado.
  Semelhança estrutural não significa compatibilidade de firmware ou de medições.
- [Apache NuttX: SmartFS](https://github.com/apache/nuttx/blob/master/Documentation/components/filesystem/smartfs.rst)
  e [implementação SMART MTD](https://github.com/apache/nuttx/blob/master/drivers/mtd/smart.c):
  descrevem setores lógicos, sequências, estado, cadeias e integridade.
  São pistas para investigar o armazenamento. Não foi estabelecida a versão
  usada pelo aparelho nem equivalência exata dos cabeçalhos observados.
- [Rust 2024 e Rust 1.85](https://blog.rust-lang.org/2025/02/20/Rust-1.85.0/),
  [clap derive](https://docs.rs/clap/4.6.1/clap/_derive/),
  [operações de arquivos Rust](https://doc.rust-lang.org/std/fs/),
  [tempfile: persistência sem sobrescrita](https://docs.rs/tempfile/3.27.0/tempfile/struct.NamedTempFile.html#method.persist_noclobber):
  documentação das interfaces usadas. Compilação local: Rust 1.98.1.

Buscas por `FC2 Falker github parser`, `FS2 Falker clorofilog` e combinações
similares não localizaram, nesta pesquisa, uma implementação independente
verificável que especificasse as medições deste arquivo. Esse resultado não
prova a inexistência de implementações ou documentação adicional.
