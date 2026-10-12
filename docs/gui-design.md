# Sistema visual da GUI

`gui/src/theme.rs` concentra cores, espaçamentos, tipografia, altura dos controles
e arredondamento. `theme::apply(ctx, dark)` alterna os temas claro e escuro;
`theme::card(ui, ...)` cria painéis com a mesma borda e preenchimento. Os temas
usam superfícies neutras e destaque verde discreto para foco e integridade
estrutural. O tema escuro inicia por padrão: fundo grafite, cartões mais escuros,
bordas suaves e abas ativas claras em cápsula, seguindo as referências visuais
fornecidas pelo usuário. O tema claro permanece disponível no botão de sol/lua
ao lado de **Info**, fora da janela informativa. O ícone indica o tema ativo;
o tooltip e o rótulo de acessibilidade explicam a ação de alternância.

O cabeçalho reúne marca e ações; o subtítulo aparece somente no explorador,
que exibe arquivo, contagens e CRC em uma faixa compacta extraída do decoder,
com quebra automática em janelas estreitas. A tela de trabalho possui cartão de
identificação compacto (nome, ID, contagens e estado na mesma faixa, com quebra
automática em janelas estreitas) e abas. Voltar fica no cabeçalho, evitando uma
linha adicional de navegação. Transições de opacidade de 180 ms acompanham telas,
abas e temas; o destaque das abas também é animado. O layout permanece estável
e os controles continuam utilizáveis durante a animação.
Em janelas compactas, o resumo selecionado aparece abaixo
do mapa; em janelas maiores, ocupa o painel direito. O rodapé mantém o estado da
operação e informa versão e tecnologias em uma única linha; mensagens extensas
e o destino completo ficam disponíveis ao passar o mouse. Os marcados possuem
seção fixa no topo, independente da busca. Cabeçalhos da tabela mostram ícones
Phosphor de filtro/ordenação e indicam a direção ativa. Mapas e tabelas continuam mostrando
dados reais; nenhuma coordenada ilustrativa das referências foi incorporada.

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
