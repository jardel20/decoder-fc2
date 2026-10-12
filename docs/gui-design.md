# Sistema visual da GUI

`gui/src/theme.rs` concentra cores, espaçamentos, tipografia, altura dos controles
e arredondamento. `theme::apply(ctx, dark)` alterna os temas claro e escuro;
`theme::card(ui, ...)` cria painéis com a mesma borda e preenchimento. Os temas
usam superfícies neutras e um único destaque azul para seleção e foco.

`gui/src/icons.rs` é o catálogo de ícones. `install_fonts(ctx)` deve ser chamado
uma vez na inicialização. A fonte Phosphor é incorporada pela dependência, sem
download em tempo de execução. `text(Glyph)` produz um ícone regular de 20 px;
`button(ui, Glyph, label)` produz um único botão com ícone e texto visível,
utilizando famílias de fonte distintas. O botão participa da navegação normal
de teclado do egui e conserva um único alvo de clique.

Os identificadores foram conferidos no índice da versão 0.5.0. Esta versão não
oferece `FILE_BINARY`; a ação `Glyph::FileBinary` utiliza o ícone oficial
`BINARY`. Todos os ícones são Phosphor, sem glifos de outras famílias.

O aviso MIT dos desenhos Phosphor está em `licenses/Phosphor-Icons-MIT.txt`.
Antes de distribuir o executável, também devem ser conservados os avisos das
dependências incorporadas, incluindo a licença da crate de integração.

Referências consultadas:

- [API da integração Phosphor 0.5.0](https://docs.rs/egui_phosphor_icons/latest/egui_phosphor_icons/)
- [Catálogo de ícones](https://docs.rs/egui_phosphor_icons/latest/egui_phosphor_icons/icons/index.html)
- [Implementação e nomes das famílias de fonte](https://docs.rs/egui_phosphor_icons/latest/src/egui_phosphor_icons/lib.rs.html)
- [API de estilos egui 0.36.2](https://docs.rs/egui/latest/egui/style/struct.WidgetVisuals.html)
- [API de painéis egui](https://docs.rs/egui/latest/egui/containers/frame/struct.Frame.html)

Validação visual final requer executar a janela em ambos os temas. A definição
do tema e a verificação dos identificadores por documentação não substituem essa
validação nem uma compilação com as dependências efetivamente disponíveis.
