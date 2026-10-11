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
