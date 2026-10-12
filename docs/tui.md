# Interface de terminal

```bash
fc2 tui [arquivo.FC2] [--profile perfil.json]
```

Sem arquivo, a interface abre o navegador local; com arquivo, mostra os trabalhos
reconhecidos. O resumo apresenta contagens, período disponível, ausências e
estatísticas identificadas como calculadas. Bytes e offsets aparecem sob demanda.

## Atalhos principais

| Tecla | Ação |
|---|---|
| `Espaço` | Marcar/desmarcar trabalho |
| `A` / `U` | Marcar os trabalhos visíveis / limpar seleção |
| `M` | Alternar modo simples/completo |
| `e` / `E` | Exportar marcados (ou atual) / todos |
| `1`–`6` | Trabalhos, medições, arquivos, hexadecimal, auditoria, pontos |
| `i` / `w` | Detalhes / avisos |
| `/` | Buscar; os resultados atualizam enquanto você digita |
| `f` / `l` | Filtrar avisos / limpar filtros |
| `:` / `?` | Menu pesquisável / ajuda |
| `q` / `Ctrl+C` | Sair |

A busca e os filtros alteram a visualização, não a quantidade exportada. As marcas
persistem quando a busca muda. A tela mostra o total marcado, inclusive itens
ocultos pelo filtro.

O modo simples é o padrão e gera somente XLSX. O completo também inclui auditoria,
arquivos brutos e página local. Ambos recusam sobrescrever a pasta de saída.

`e`/`E` abrem o seletor de destino em Downloads ou na localização equivalente
configurada pelo usuário. Ele mostra somente pastas: Enter entra; Backspace volta
à pasta pai; `/` busca; `s` seleciona a pasta atual; `d` ou `O` aceitam um caminho
direto (inclusive outra unidade Windows); `D` abre Downloads; `h` abre a pasta
pessoal. Em seguida, confirme ou edite o caminho da subpasta nova. Esc cancela a
confirmação e volta ao seletor; outro Esc cancela o seletor sem criar saídas nem
alterar os trabalhos marcados. Após exportar, a pasta escolhida é reutilizada.

O cabeçalho organiza arquivo, contagens gerais, versão, modo, seleção e destino.
O rodapé separa mensagens de estado, atalhos do contexto e ações gerais. Em
terminais pequenos, as linhas se adaptam ao espaço disponível.

Iniciar `fc2` sem argumentos abre o navegador de arquivos; `fc2 arquivo.FC2`
abre diretamente a TUI para esse arquivo. A aplicação não depende da pasta de
código-fonte ou do HTML externo depois de compilada.
