//! Bounded terminal rendering. Only this module's own ANSI styling is emitted.
use super::{
    App, Screen,
    summary::{self, IndexSummary, Summary},
};
use crate::{records::decimal_tenths, work_records::WorkMeasurement, workspace::Work};
use console::{measure_text_width, truncate_str};

#[derive(Clone, Copy, Default)]
enum Tone {
    #[default]
    Normal,
    Heading,
    Dim,
    Selected,
    Warning,
}
struct Line {
    text: String,
    tone: Tone,
}
impl Line {
    fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            tone: Tone::Normal,
        }
    }
    fn styled(text: impl Into<String>, tone: Tone) -> Self {
        Self {
            text: text.into(),
            tone,
        }
    }
}

/// Strip terminal controls, escape sequences and directional overrides from data.
fn clean(text: &str) -> String {
    let mut chars = text.chars().peekable();
    let mut result = String::new();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            match chars.peek().copied() {
                Some('[') => {
                    chars.next();
                    for next in chars.by_ref() {
                        if ('@'..='~').contains(&next) {
                            break;
                        }
                    }
                }
                Some(']') | Some('P') | Some('^') | Some('_') => {
                    chars.next();
                    while let Some(next) = chars.next() {
                        if next == '\u{7}' {
                            break;
                        }
                        if next == '\u{1b}' && chars.peek() == Some(&'\\') {
                            chars.next();
                            break;
                        }
                    }
                }
                Some(_) => {
                    chars.next();
                }
                None => {}
            }
            continue;
        }
        if c.is_control()
            || matches!(c,'\u{200e}'|'\u{200f}'|'\u{202a}'..='\u{202e}'|'\u{2066}'..='\u{2069}'|'\u{2028}'|'\u{2029}')
        {
            continue;
        }
        result.push(c);
    }
    result
}
fn fit(text: &str, width: usize) -> String {
    let text = clean(text);
    let clipped = truncate_str(&text, width, if width > 1 { "…" } else { "" }).into_owned();
    let padding = width.saturating_sub(measure_text_width(&clipped));
    format!("{clipped}{}", " ".repeat(padding))
}
fn input_window(text: &str, cursor: usize, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    if width == 1 {
        return "▏".into();
    }
    let mut cursor = cursor.min(text.len());
    while !text.is_char_boundary(cursor) {
        cursor = cursor.saturating_sub(1);
    }
    let left = clean(&text[..cursor]);
    let right = clean(&text[cursor..]);
    let available = width.saturating_sub(1);
    if measure_text_width(&left) + measure_text_width(&right) <= available {
        return format!("{left}▏{right}");
    }
    let left_budget = available.saturating_sub(measure_text_width(&right).min(available / 3));
    let left = if measure_text_width(&left) > left_budget {
        let budget = left_budget.saturating_sub(1);
        let mut start = left.len();
        for (index, _) in left.char_indices().rev() {
            if measure_text_width(&left[index..]) > budget {
                break;
            }
            start = index;
        }
        if left_budget == 0 {
            String::new()
        } else {
            format!("…{}", &left[start..])
        }
    } else {
        left
    };
    let right_budget = available.saturating_sub(measure_text_width(&left));
    let right = truncate_str(
        &right,
        right_budget,
        if right_budget > 0 { "…" } else { "" },
    );
    format!("{left}▏{right}")
}
fn window(selected: usize, total: usize, count: usize) -> usize {
    selected
        .saturating_sub(count / 2)
        .min(total.saturating_sub(count))
}
fn title(screen: &Screen) -> &'static str {
    match screen {
        Screen::Works => "Trabalhos",
        Screen::Measurements => "Medições",
        Screen::Record => "Detalhe da medição",
        Screen::Files => "Arquivos originais",
        Screen::Hex => "Inspeção hexadecimal",
        Screen::Overview => "Auditoria",
        Screen::Help => "Ajuda",
        Screen::Browser => "Abrir arquivo FC2",
        Screen::WorkDetails => "Detalhes do trabalho",
        Screen::Points => "Pontos de amostragem",
        Screen::Warnings => "Avisos dos dados",
    }
}
fn note(lines: &mut Vec<Line>, text: impl Into<String>) {
    lines.push(Line::styled(text, Tone::Dim));
}
fn warning(lines: &mut Vec<Line>, text: impl Into<String>) {
    lines.push(Line::styled(text, Tone::Warning));
}
fn context_keys(app: &App, width: usize) -> &'static str {
    if app.destination.is_some() {
        return if width < 65 {
            " Enter Pasta · s Selecionar · Esc Cancelar"
        } else {
            " Enter Entrar · s Selecionar pasta · D Downloads · h Início · Esc Cancelar"
        };
    }
    if app.action_menu.is_some() {
        return " ↑↓ selecionar · ações indisponíveis explicam o motivo";
    }
    if app.screen == Screen::Works {
        return if width < 50 {
            " Espaço Marca · M Modo · e Exportar"
        } else {
            " Espaço Marca  A Visíveis  U Limpa  M Modo  Enter Medições"
        };
    }
    if width < 50 {
        return " Enter detalhe  / Busca  f Avisos  l Limpar";
    }
    match app.screen {
        Screen::Works => " Enter Medições  i Detalhes  w Avisos  / Busca  f Só avisos  l Limpar",
        Screen::Measurements => {
            " Enter Detalhe  6 Pontos  w Avisos  [ ] Grupos  u n Trabalho  / Busca"
        }
        Screen::Points => " Enter Filtrar ponto  [ ] Grupos  2 Medições  w Avisos  Esc Volta",
        Screen::Record => " t Bytes  Enter Origem Hex  w Avisos  Esc Medições",
        Screen::WorkDetails => " 2 Medições  6 Pontos  w Avisos  ↑↓ Rolar  Esc Volta",
        Screen::Warnings => " t Detalhes avisos  2 Medições  ↑↓ Rolar  Esc Volta",
        Screen::Browser => " Enter Abrir  d Diretório  O Caminho direto  / Busca  Esc Volta",
        Screen::Hex => " m FC2/Memória  g Offset  ↑↓ Linhas  PgUp PgDn Páginas  Esc Volta",
        Screen::Files => " / Busca  ↑↓ Selecionar  x Backup  Esc Volta",
        Screen::Overview | Screen::Help => {
            " ↑↓ Rolar  PgUp PgDn Páginas  1 Trabalhos  2 Medições  Esc Volta"
        }
    }
}

pub fn render(app: &App, width: u16, height: u16) -> String {
    let width = usize::from(width).min(512);
    let height = usize::from(height).min(512);
    if width == 0 || height == 0 {
        return String::new();
    }
    let mut frame = Vec::new();
    frame.push(Line::styled(
        format!(
            " FC2-RS │ {} · {} marcados │ {}",
            app.export_mode_label()
                .split_whitespace()
                .next()
                .unwrap_or("Completo"),
            app.marked_works.len(),
            if app.action_menu.is_some() {
                "Ações"
            } else if app.destination.is_some() {
                "Destino de exportação"
            } else {
                title(&app.screen)
            }
        ),
        Tone::Heading,
    ));
    if height == 1 {
        return encode(frame, width);
    }
    let file = app
        .file
        .as_ref()
        .map(|f| {
            let name = f.path.file_name().unwrap_or_default().to_string_lossy();
            match f.path.parent().filter(|p| !p.as_os_str().is_empty()) {
                Some(parent) => format!("{name}  ·  {}", parent.display()),
                None => name.into_owned(),
            }
        })
        .unwrap_or_else(|| "Nenhum FC2 aberto · o para abrir um arquivo".into());
    if height >= 5 {
        frame.push(Line::styled(format!(" Arquivo  {file}"), Tone::Dim));
    }
    if height >= 12 {
        let summary = app
            .dataset()
            .map(|dataset| {
                format!(
                    "{} trabalhos · {} medições · {} grupos",
                    dataset.works.len(),
                    dataset
                        .works
                        .iter()
                        .map(|w| w.record_count())
                        .sum::<usize>(),
                    dataset.works.iter().map(|w| w.groups.len()).sum::<usize>()
                )
            })
            .unwrap_or_else(|| "Selecione um FC2 para inspecionar e exportar".into());
        frame.push(Line::styled(
            format!(" Dados    {summary}  │  v{}", env!("CARGO_PKG_VERSION")),
            Tone::Normal,
        ));
        frame.push(Line::styled(
            format!(
                " Destino  {}",
                if app.destination.is_some() {
                    &app.browser.directory
                } else {
                    &app.export_root
                }
                .display()
            ),
            Tone::Dim,
        ));
        frame.push(Line::styled("─".repeat(width), Tone::Dim));
    }
    let show_error = height >= 6 && app.prompt.as_ref().is_some_and(|p| p.error.is_some());
    let footer_count =
        if height >= 9 { 3 } else { 2 } + usize::from(show_error) + usize::from(height >= 12);
    let content_height = height.saturating_sub(frame.len() + footer_count);
    let body = if app.action_menu.is_some() {
        actions(app, width, content_height)
    } else {
        match &app.screen {
            Screen::Works => works(app, width, content_height),
            Screen::Measurements => measurements(app, width, content_height),
            Screen::Record => record(app, width, content_height),
            Screen::Files => files(app, width, content_height),
            Screen::Hex => hex(app, width, content_height),
            Screen::Overview => overview(app, content_height),
            Screen::Help => help(app, content_height),
            Screen::Browser => browser(app, width, content_height),
            Screen::WorkDetails => work_details(app, width, content_height),
            Screen::Points => points(app, width, content_height),
            Screen::Warnings => warnings(app, content_height),
        }
    };
    frame.extend(body.into_iter().take(content_height));
    while frame.len() < height.saturating_sub(footer_count) {
        frame.push(Line::new(""));
    }
    if height >= 12 {
        frame.push(Line::styled("─".repeat(width), Tone::Dim));
    }
    if let Some(prompt) = &app.prompt {
        if show_error {
            frame.push(Line::styled(
                format!(" {}", prompt.error.as_deref().unwrap_or("")),
                Tone::Warning,
            ));
        }
        if height >= 9 {
            frame.push(Line::styled(format!(" {}", prompt.label), Tone::Heading));
        }
        frame.push(Line::styled(
            format!(
                " {}",
                input_window(&prompt.value, prompt.cursor, width.saturating_sub(1))
            ),
            Tone::Selected,
        ));
    } else {
        frame.push(Line::styled(
            format!(
                " {}",
                if app.status.is_empty() {
                    "↑↓ navegar · Enter detalhe · ? ajuda"
                } else {
                    &app.status
                }
            ),
            if app.status.starts_with("Erro:") {
                Tone::Warning
            } else {
                Tone::Normal
            },
        ));
    }
    if height >= 9 && app.prompt.is_none() {
        frame.push(Line::styled(context_keys(app, width), Tone::Dim));
    }
    frame.push(Line::styled(
        if app.prompt.is_some() {
            if width < 44 {
                " Enter OK · Esc volta · Ctrl+U limpa"
            } else {
                " Enter confirmar  Esc cancelar  Ctrl+U limpar  Ctrl+C sair"
            }
        } else if app.destination.is_some() {
            " ↑↓ Navegar  / Buscar  d Caminho  Ctrl+C Sair"
        } else if app.action_menu.is_some() {
            " ↑↓ escolher · digite busca · Enter executar · Esc voltar"
        } else if app.screen == Screen::Browser {
            " D Downloads  h Início  : Ações  ? Ajuda  q Sair"
        } else if width < 44 {
            " : Ações  ? Ajuda  q Sair"
        } else {
            " e Exportar  E Todos  : Ações  o Abrir  ? Ajuda  q Sair"
        },
        Tone::Dim,
    ));
    frame.truncate(height);
    encode(frame, width)
}
fn encode(lines: Vec<Line>, width: usize) -> String {
    lines
        .into_iter()
        .map(|line| {
            let style = match line.tone {
                Tone::Normal => "\x1b[0m",
                Tone::Heading => "\x1b[1;36m",
                Tone::Dim => "\x1b[90m",
                Tone::Selected => "\x1b[30;47m",
                Tone::Warning => "\x1b[33m",
            };
            format!("{style}{}\x1b[0m", fit(&line.text, width))
        })
        .collect::<Vec<_>>()
        .join("\r\n")
}
fn empty(app: &App) -> Vec<Line> {
    let mut lines = vec![Line::new(
        " Abra um arquivo com o ou escolha um FC2 no navegador.",
    )];
    if let Some(error) = app.file.as_ref().and_then(|f| f.decode_error.as_ref()) {
        warning(&mut lines, format!(" Decodificação bloqueada: {error}"));
        note(
            &mut lines,
            " A inspeção hexadecimal e a auditoria continuam disponíveis.",
        );
    }
    lines
}

fn period(summary: &Summary) -> String {
    match (&summary.first_date, &summary.last_date) {
        (Some(first), Some(last)) if first == last => first.clone(),
        (Some(first), Some(last)) => format!("{first} a {last}"),
        _ => "sem datas válidas".into(),
    }
}
fn mean(summary: &Option<IndexSummary>) -> String {
    summary
        .as_ref()
        .map(|s| format!("{:.2}", s.mean))
        .unwrap_or_else(|| "—".into())
}
fn range(summary: &Option<IndexSummary>) -> String {
    summary
        .as_ref()
        .map(|s| format!("{}–{}", decimal_tenths(s.min), decimal_tenths(s.max)))
        .unwrap_or_else(|| "—".into())
}
fn leaf_scope(work: &Work) -> String {
    if work
        .reference_validation
        .as_ref()
        .is_some_and(|r| r.accepted)
    {
        "Referência Leaf: correspondência aceita neste trabalho".into()
    } else {
        "Validação estrutural; sem comparação externa Leaf".into()
    }
}
fn human_warning(message: &str) -> String {
    match message {
        "calendar_all_zero_raw_values_preserved" => {
            "Data e hora ausentes; calendário bruto preservado".into()
        }
        "calendar_invalid_raw_values_preserved" => {
            "Data inválida; calendário bruto preservado".into()
        }
        "time_invalid_raw_values_preserved" => "Hora inválida; valor bruto preservado".into(),
        "latitude_nonfinite_raw_bits_preserved" => {
            "Latitude ausente; valor binário não finito preservado".into()
        }
        "longitude_nonfinite_raw_bits_preserved" => {
            "Longitude ausente; valor binário não finito preservado".into()
        }
        "latitude_out_of_range_raw_bits_preserved" => {
            "Latitude fora dos limites; bits originais preservados".into()
        }
        "longitude_out_of_range_raw_bits_preserved" => {
            "Longitude fora dos limites; bits originais preservados".into()
        }
        _ if message.starts_with("configuration missing:") => {
            "Configuração ausente: nome e contagens independentes indisponíveis".into()
        }
        _ if message.starts_with("configuration name unavailable;") => {
            "Nome do trabalho ausente; identificação do diretório preservada".into()
        }
        _ => message.replace('_', " "),
    }
}
fn record_warning(r: &WorkMeasurement) -> String {
    let mut parts = Vec::new();
    if r.date.is_none() {
        parts.push("data ausente");
    }
    if r.time.is_none() {
        parts.push("hora ausente");
    }
    if r.latitude.is_none() || r.longitude.is_none() {
        parts.push("GPS ausente");
    }
    if parts.is_empty() {
        r.warnings
            .first()
            .map(|s| human_warning(s))
            .unwrap_or_else(|| "sem avisos".into())
    } else {
        parts.join(" · ")
    }
}
fn work_preview(work: &Work, width: usize) -> Vec<Line> {
    let summary = Summary::work(work);
    let mut lines = vec![Line::styled(
        format!(" {} · {}", work.number, work.display_name),
        Tone::Heading,
    )];
    if width >= 68 {
        lines.push(Line::new(format!(
            " {} medições · {} grupos · {} pontos · {}",
            summary.records,
            summary.groups,
            summary.points,
            period(&summary)
        )));
        lines.push(Line::new(format!(
            " ICF calculado · médias A {}  B {}  Total {}",
            mean(&summary.a),
            mean(&summary.b),
            mean(&summary.total)
        )));
        lines.push(Line::new(format!(
            " Faixas ICF · A {}  B {}  Total {}",
            range(&summary.a),
            range(&summary.b),
            range(&summary.total)
        )));
        lines.push(Line::new(format!(
            " Sem data {} · hora {} · GPS {} · Leaf: {}",
            summary.missing_dates,
            summary.missing_times,
            summary.missing_gps,
            if work
                .reference_validation
                .as_ref()
                .is_some_and(|r| r.accepted)
            {
                "validado neste trabalho"
            } else {
                "não comparado"
            }
        )));
    } else {
        lines.push(Line::new(format!(
            " {} medições · {} grupos · {} pontos",
            summary.records, summary.groups, summary.points
        )));
        lines.push(Line::new(format!(" Período: {}", period(&summary))));
        note(&mut lines, " ICF calculado: média / faixa");
        for (label, index) in [
            ("A", &summary.a),
            ("B", &summary.b),
            ("Total", &summary.total),
        ] {
            lines.push(Line::new(format!(
                " {label}: {} / {}",
                mean(index),
                range(index)
            )));
        }
        lines.push(Line::new(format!(
            " Sem data {} · hora {} · GPS {}",
            summary.missing_dates, summary.missing_times, summary.missing_gps
        )));
        note(
            &mut lines,
            if work
                .reference_validation
                .as_ref()
                .is_some_and(|r| r.accepted)
            {
                " Leaf: referência aceita"
            } else {
                " Leaf: sem comparação externa"
            },
        );
    }
    if let Some(error) = work.errors.first() {
        warning(&mut lines, format!(" Bloqueio: {error}"));
    } else if let Some(message) = work.warnings.first() {
        warning(&mut lines, human_warning(message));
    } else if summary.warned_records > 0 {
        warning(
            &mut lines,
            format!(
                " {} medições com avisos · w para examinar",
                summary.warned_records
            ),
        );
    } else {
        note(&mut lines, " Sem avisos de disponibilidade neste trabalho.");
    }
    lines
}
fn work_line(work: &Work, selected: bool, marked: bool, width: usize) -> Line {
    let marker = if selected { "›" } else { " " };
    let checkbox = if marked { "[x]" } else { "[ ]" };
    let text = if width >= 69 {
        format!(
            "{marker} {checkbox} {:>3} {} {:>5} med {:>2} grp {:>3} av",
            work.number,
            fit(&work.display_name, width.saturating_sub(37)),
            work.record_count(),
            work.groups.len(),
            work.warning_count()
        )
    } else {
        format!(
            "{marker} {checkbox} {:>3} {} {:>4} med",
            work.number,
            fit(&work.display_name, width.saturating_sub(21)),
            work.record_count()
        )
    };
    Line::styled(
        text,
        if selected {
            Tone::Selected
        } else if !work.errors.is_empty() {
            Tone::Warning
        } else {
            Tone::Normal
        },
    )
}
fn works(app: &App, width: usize, height: usize) -> Vec<Line> {
    let Some(dataset) = app.file.as_ref().and_then(|f| f.dataset.as_ref()) else {
        return empty(app);
    };
    let summary = Summary::dataset(dataset);
    let visible = app.visible_works();
    let mut lines = Vec::new();
    if app.height < 12 {
        lines.push(Line::styled(
            format!(
                " FC2: {} trabalhos · {} medições · {} grupos",
                dataset.works.len(),
                summary.records,
                summary.groups
            ),
            Tone::Heading,
        ));
    }
    lines.push(Line::new(format!(
        " {} · sem data {} · sem GPS {}",
        period(&summary),
        summary.missing_dates,
        summary.missing_gps
    )));
    note(
        &mut lines,
        format!(
            " Exibidos {}/{} · busca: {}{}",
            visible.len(),
            dataset.works.len(),
            if app.search.is_empty() {
                "—"
            } else {
                &app.search
            },
            if app.only_warnings {
                " · somente avisos"
            } else {
                ""
            }
        ),
    );
    let available = height.saturating_sub(lines.len());
    if width >= 110 {
        let right_width = (width * 2 / 5).max(40);
        let left_width = width.saturating_sub(right_width + 3);
        let mut left = vec![Line::styled(" Trabalhos · ↑↓ selecionar", Tone::Dim)];
        let rows = available.saturating_sub(1);
        let start = window(app.work_index, visible.len(), rows);
        for (position, &index) in visible.iter().enumerate().skip(start).take(rows) {
            if let Some(work) = dataset.works.get(index) {
                left.push(work_line(
                    work,
                    position == app.work_index,
                    app.marked_works.contains(&work.number),
                    left_width,
                ));
            }
        }
        let right = app
            .selected_work()
            .map(|w| work_preview(w, right_width))
            .unwrap_or_else(|| vec![Line::new(" Nenhum trabalho corresponde ao filtro.")]);
        for i in 0..available {
            let a = left.get(i);
            let b = right.get(i);
            lines.push(Line::styled(
                format!(
                    "{} │ {}",
                    fit(a.map(|l| l.text.as_str()).unwrap_or(""), left_width),
                    fit(b.map(|l| l.text.as_str()).unwrap_or(""), right_width)
                ),
                a.map(|l| l.tone)
                    .unwrap_or_else(|| b.map(|l| l.tone).unwrap_or(Tone::Normal)),
            ));
        }
    } else {
        let preview = app
            .selected_work()
            .map(|w| work_preview(w, width))
            .unwrap_or_default();
        let preview_count = if available >= 10 {
            6
        } else if available >= 6 {
            3
        } else {
            0
        };
        let rows = available.saturating_sub(preview_count);
        let start = window(app.work_index, visible.len(), rows);
        for (position, &index) in visible.iter().enumerate().skip(start).take(rows) {
            if let Some(work) = dataset.works.get(index) {
                lines.push(work_line(
                    work,
                    position == app.work_index,
                    app.marked_works.contains(&work.number),
                    width,
                ));
            }
        }
        while lines.len() < height.saturating_sub(preview_count) {
            lines.push(Line::new(""));
        }
        if preview_count > 0 {
            for line in preview.iter().take(preview_count) {
                lines.push(Line::styled(line.text.clone(), line.tone));
            }
        }
        if visible.is_empty() {
            warning(
                &mut lines,
                " Nenhum trabalho corresponde ao filtro. l limpa os filtros.",
            );
        }
    }
    lines
}
fn measurements(app: &App, width: usize, height: usize) -> Vec<Line> {
    let Some(work) = app.selected_work() else {
        return empty(app);
    };
    let Some(group) = app.selected_group() else {
        return vec![Line::new(" Este trabalho não contém grupos de medições.")];
    };
    let visible = app.visible_records();
    let tabs = work
        .groups
        .iter()
        .map(|g| {
            if g.group == group.group {
                format!("[{}]", g.group)
            } else {
                g.group.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ");
    let mut lines = vec![Line::styled(
        format!(
            " {} · grupo {} · {}/{} medições",
            work.display_name,
            group.group,
            visible.len(),
            group.records.len()
        ),
        Tone::Heading,
    )];
    note(
        &mut lines,
        format!(
            " Grupos: {tabs} · [ ] alterna{}{}",
            app.point_filter
                .map(|p| format!(" · ponto {p}"))
                .unwrap_or_default(),
            if app.only_warnings {
                " · somente avisos"
            } else {
                ""
            }
        ),
    );
    let dated = width >= 75;
    let gps = width >= 106;
    note(
        &mut lines,
        if gps {
            "   Medição Ponto Data       Hora       ICF A   ICF B   Total    Latitude   Longitude"
        } else if dated {
            "   Medição Ponto Data       Hora       ICF A   ICF B   Total"
        } else {
            "   Medição Ponto    ICF A    ICF B    Total"
        },
    );
    let preview_rows = if height >= 9 { 3 } else { 0 };
    let rows = height.saturating_sub(lines.len() + preview_rows);
    let start = window(app.record_index, visible.len(), rows);
    for (position, &index) in visible.iter().enumerate().skip(start).take(rows) {
        if let Some(r) = group.records.get(index) {
            let marker = if position == app.record_index {
                "›"
            } else {
                " "
            };
            let mut text = if dated {
                format!(
                    "{marker} {:>8} {:>5} {:10} {:8} {:>7} {:>7} {:>7}",
                    r.id,
                    r.point,
                    r.date.as_deref().unwrap_or("—"),
                    r.time.as_deref().unwrap_or("—"),
                    r.chlorophyll_a_index,
                    r.chlorophyll_b_index,
                    r.chlorophyll_total_index
                )
            } else {
                format!(
                    "{marker} {:>8} {:>5} {:>8} {:>8} {:>8}",
                    r.id,
                    r.point,
                    r.chlorophyll_a_index,
                    r.chlorophyll_b_index,
                    r.chlorophyll_total_index
                )
            };
            if gps {
                text.push_str(&format!(
                    " {:>11} {:>11}",
                    r.latitude
                        .map(|v| format!("{v:.6}"))
                        .unwrap_or_else(|| "—".into()),
                    r.longitude
                        .map(|v| format!("{v:.6}"))
                        .unwrap_or_else(|| "—".into())
                ));
            }
            lines.push(Line::styled(
                text,
                if position == app.record_index {
                    Tone::Selected
                } else if !r.warnings.is_empty() {
                    Tone::Warning
                } else {
                    Tone::Normal
                },
            ));
        }
    }
    while lines.len() < height.saturating_sub(preview_rows) {
        lines.push(Line::new(""));
    }
    if preview_rows > 0 {
        if let Some(r) = app.selected_record() {
            note(
                &mut lines,
                format!(
                    " ID {} · ponto {} · {} · {}",
                    r.id,
                    r.point,
                    r.date.as_deref().unwrap_or("data ausente"),
                    r.time.as_deref().unwrap_or("hora ausente")
                ),
            );
            lines.push(Line::new(format!(
                " GPS: {} / {} · {}",
                r.latitude
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "ausente".into()),
                r.longitude
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "ausente".into()),
                record_warning(r)
            )));
        }
        let selection: Vec<_> = visible
            .iter()
            .filter_map(|&i| group.records.get(i))
            .collect();
        let summary = Summary::records(&selection);
        note(
            &mut lines,
            format!(
                " Seleção: {} pontos · médias ICF calculadas A {} B {} Total {}",
                summary.points,
                mean(&summary.a),
                mean(&summary.b),
                mean(&summary.total)
            ),
        );
    }
    if visible.is_empty() {
        warning(
            &mut lines,
            " Nenhuma medição corresponde ao filtro. l limpa os filtros.",
        );
    }
    lines
}
fn work_details(app: &App, width: usize, height: usize) -> Vec<Line> {
    let Some(work) = app.selected_work() else {
        return empty(app);
    };
    let mut lines = work_preview(work, width);
    note(
        &mut lines,
        " Grupos independentes: pontos reiniciam a numeração em cada grupo.",
    );
    for group in &work.groups {
        let s = Summary::group(group);
        lines.push(Line::styled(
            format!(
                " Grupo {} · {} medições · {} pontos · {}",
                group.group,
                s.records,
                s.points,
                period(&s)
            ),
            Tone::Heading,
        ));
        lines.push(Line::new(format!(
            " Médias ICF calculadas: A {} · B {} · Total {}",
            mean(&s.a),
            mean(&s.b),
            mean(&s.total)
        )));
        note(
            &mut lines,
            format!(
                " Faixas ICF: A {} · B {} · Total {}",
                range(&s.a),
                range(&s.b),
                range(&s.total)
            ),
        );
        note(
            &mut lines,
            format!(
                " Sem data {} · hora {} · GPS {} · {} medições com avisos",
                s.missing_dates, s.missing_times, s.missing_gps, s.warned_records
            ),
        );
    }
    for warning_text in &work.warnings {
        warning(&mut lines, human_warning(warning_text));
    }
    scroll(lines, app.scroll, height)
}
fn points(app: &App, width: usize, height: usize) -> Vec<Line> {
    let Some(group) = app.selected_group() else {
        return empty(app);
    };
    let points = summary::points(group);
    let mut lines = vec![Line::styled(
        format!(
            " Grupo {} · {} pontos · Enter filtra as medições do ponto",
            group.group,
            points.len()
        ),
        Tone::Heading,
    )];
    note(
        &mut lines,
        if width >= 95 {
            "  Ponto   N   Média A Média B Total    Faixa Total       Período"
        } else {
            "  Ponto   N   Média A Média B Total    Faixa Total"
        },
    );
    let preview_rows = if height >= 9 { 4 } else { 0 };
    let count = height.saturating_sub(lines.len() + preview_rows);
    let start = window(app.point_index, points.len(), count);
    for (position, p) in points.iter().enumerate().skip(start).take(count) {
        let s = &p.summary;
        let mut text = format!(
            "{} {:>5} {:>4} {:>7} {:>7} {:>7}  {:>13}",
            if position == app.point_index {
                "›"
            } else {
                " "
            },
            p.point,
            s.records,
            mean(&s.a),
            mean(&s.b),
            mean(&s.total),
            range(&s.total)
        );
        if width >= 95 {
            text.push_str(&format!("  {}", period(s)));
        }
        lines.push(Line::styled(
            text,
            if position == app.point_index {
                Tone::Selected
            } else {
                Tone::Normal
            },
        ));
    }
    while lines.len() < height.saturating_sub(preview_rows) {
        lines.push(Line::new(""));
    }
    if preview_rows > 0 {
        if let Some(p) = points.get(app.point_index) {
            let s = &p.summary;
            note(
                &mut lines,
                format!(
                    " Ponto {} do grupo {} · {} medições · {}",
                    p.point,
                    group.group,
                    s.records,
                    period(s)
                ),
            );
            lines.push(Line::new(format!(
                " Faixas ICF: A {} · B {} · Total {}",
                range(&s.a),
                range(&s.b),
                range(&s.total)
            )));
            note(
                &mut lines,
                format!(
                    " Sem data {} · hora {} · GPS {} · {} medições com avisos",
                    s.missing_dates, s.missing_times, s.missing_gps, s.warned_records
                ),
            );
        }
        note(
            &mut lines,
            " Médias e faixas são resumos calculados; medições originais permanecem intactas.",
        );
    }
    lines
}
fn warnings(app: &App, height: usize) -> Vec<Line> {
    let Some(work) = app.selected_work() else {
        return empty(app);
    };
    let s = Summary::work(work);
    let mut lines = vec![
        Line::styled(
            format!(
                " Avisos · {} · {} medições com avisos",
                work.display_name, s.warned_records
            ),
            Tone::Heading,
        ),
        Line::new(format!(
            " Sem data {} · sem hora {} · sem GPS {}",
            s.missing_dates, s.missing_times, s.missing_gps
        )),
    ];
    note(
        &mut lines,
        " Ausências permanecem vazias. Avisos de disponibilidade não significam corrupção.",
    );
    for error in &work.errors {
        warning(&mut lines, format!(" Bloqueio estrutural: {error}"));
    }
    for message in &work.warnings {
        warning(&mut lines, human_warning(message));
    }
    for group in &work.groups {
        let warned: Vec<_> = group
            .records
            .iter()
            .filter(|r| !r.warnings.is_empty())
            .collect();
        if warned.is_empty() {
            continue;
        }
        lines.push(Line::styled(
            format!(
                " Grupo {} · {} medições com avisos",
                group.group,
                warned.len()
            ),
            Tone::Heading,
        ));
        for r in warned {
            warning(
                &mut lines,
                format!(" ID {} · ponto {} · {}", r.id, r.point, record_warning(r)),
            );
            if app.technical_details {
                for message in &r.warnings {
                    note(&mut lines, format!("  {}", human_warning(message)));
                }
            }
        }
    }
    if work.warnings.is_empty() && work.errors.is_empty() && s.warned_records == 0 {
        note(&mut lines, " Nenhum aviso neste trabalho.");
    }
    scroll(lines, app.scroll, height)
}
fn record(app: &App, width: usize, height: usize) -> Vec<Line> {
    let Some(r) = app.selected_record() else {
        return vec![Line::new(" Selecione uma medição e pressione Enter.")];
    };
    let mut lines = vec![
        Line::styled(
            format!(
                " {} · grupo {} · medição {} · ponto {}",
                app.selected_work()
                    .map(|w| w.display_name.as_str())
                    .unwrap_or("Trabalho"),
                app.selected_group()
                    .map(|g| g.group.to_string())
                    .unwrap_or_else(|| "—".into()),
                r.id,
                r.point
            ),
            Tone::Heading,
        ),
        Line::new(format!(
            " Data: {}   Hora: {}   Fuso: {}",
            r.date.as_deref().unwrap_or("ausente"),
            r.time.as_deref().unwrap_or("ausente"),
            r.timezone.as_deref().unwrap_or("desconhecido")
        )),
        Line::new(format!(
            " ICF A: {}   ICF B: {}   ICF Total: {}",
            r.chlorophyll_a_index, r.chlorophyll_b_index, r.chlorophyll_total_index
        )),
        Line::new(format!(
            " Latitude: {}",
            r.latitude
                .map(|v| v.to_string())
                .unwrap_or_else(|| "ausente".into())
        )),
        Line::new(format!(
            " Longitude: {}",
            r.longitude
                .map(|v| v.to_string())
                .unwrap_or_else(|| "ausente".into())
        )),
    ];
    if let Some(group) = app.selected_group() {
        note(
            &mut lines,
            format!(
                " Ponto {} pertence ao grupo {}. Arquivo: {}",
                r.point, group.group, group.source_file
            ),
        );
    }
    if let Some(work) = app.selected_work() {
        note(&mut lines, leaf_scope(work));
    }
    note(
        &mut lines,
        " Índices instrumentais; o total mantém seu campo. Nenhum valor estimado.",
    );
    if r.warnings.is_empty() {
        note(&mut lines, " Avisos: nenhum.");
    } else {
        for message in &r.warnings {
            warning(&mut lines, human_warning(message));
        }
    }
    note(
        &mut lines,
        if app.technical_details {
            " t oculta bytes e rastreabilidade · Enter abre a origem no Hex"
        } else {
            " t mostra bytes e rastreabilidade · Enter abre a origem no Hex"
        },
    );
    if app.technical_details {
        lines.push(Line::styled(" Rastreabilidade binária", Tone::Heading));
        note(
            &mut lines,
            format!(
                " Calendário bruto: {}/{}/{} {:02}:{:02}:{:02}",
                r.raw_calendar.day,
                r.raw_calendar.month,
                r.raw_calendar.year,
                r.raw_calendar.hour,
                r.raw_calendar.minute,
                r.raw_calendar.second
            ),
        );
        note(
            &mut lines,
            format!(
                " Bits GPS: latitude 0x{:08x} · longitude 0x{:08x}",
                r.latitude_bits, r.longitude_bits
            ),
        );
        note(&mut lines, format!(" SHA-256 registro: {}", r.raw_sha256));
        note(
            &mut lines,
            format!(
                " Bytes desconhecidos @5..23: {} · @34: {}",
                r.unknown_prefix_hex, r.unknown_34
            ),
        );
        for segment in &r.segments {
            lines.push(Line::new(format!(
                " FC2 0x{:08x} ({}) / memória 0x{:08x} / {} bytes",
                segment.file_offset, segment.file_offset, segment.memory_address, segment.length
            )));
        }
        let chunk = if width >= 72 { 32 } else { 16 };
        for (index, bytes) in r.raw_hex.as_bytes().chunks(chunk).enumerate() {
            lines.push(Line::new(format!(
                " +0x{:02x}: {}",
                index * chunk / 2,
                String::from_utf8_lossy(bytes)
            )));
        }
    }
    scroll(lines, app.scroll, height)
}
fn actions(app: &App, width: usize, height: usize) -> Vec<Line> {
    let Some(menu) = &app.action_menu else {
        return Vec::new();
    };
    let visible = app.visible_actions();
    let mut lines = vec![
        Line::styled(
            " Ações · digite para buscar · Enter executa · Esc fecha",
            Tone::Heading,
        ),
        Line::new(format!(
            " Busca: {} · {} ações",
            if menu.search.is_empty() {
                "—"
            } else {
                &menu.search
            },
            visible.len()
        )),
    ];
    let preview_rows = if height >= 8 { 3 } else { 1 };
    let count = height.saturating_sub(lines.len() + preview_rows);
    let start = window(menu.index, visible.len(), count);
    for (position, action) in visible.iter().enumerate().skip(start).take(count) {
        let text = format!(
            "{} {:>2}  {}{}",
            if position == menu.index { "›" } else { " " },
            action.key,
            action.label,
            if action.enabled {
                ""
            } else {
                " · indisponível"
            }
        );
        lines.push(Line::styled(
            text,
            if position == menu.index {
                Tone::Selected
            } else if action.enabled {
                Tone::Normal
            } else {
                Tone::Dim
            },
        ));
    }
    while lines.len() < height.saturating_sub(preview_rows) {
        lines.push(Line::new(""));
    }
    if let Some(action) = visible.get(menu.index) {
        lines.push(Line::styled(
            format!(" {}", action.description),
            if action.enabled {
                Tone::Dim
            } else {
                Tone::Warning
            },
        ));
        if !action.enabled {
            warning(
                &mut lines,
                " Esta ação está indisponível no contexto atual.",
            );
        }
    }
    if width >= 65 {
        note(
            &mut lines,
            " Exportações preservam dados e evitam sobrescrever arquivos existentes.",
        );
    }
    lines
}

fn files(app: &App, width: usize, height: usize) -> Vec<Line> {
    let Some(dataset) = app.file.as_ref().and_then(|f| f.dataset.as_ref()) else {
        return empty(app);
    };
    let visible = app.visible_files();
    let mut lines = vec![Line::new(format!(
        " {} arquivos ativos · busca: {}",
        visible.len(),
        if app.search.is_empty() {
            "—"
        } else {
            &app.search
        }
    ))];
    note(
        &mut lines,
        " Arquivos brutos preservam medições, configurações e campos desconhecidos.",
    );
    let rows = height.saturating_sub(lines.len() + 4);
    let start = window(app.raw_file_index, visible.len(), rows);
    for (position, &index) in visible.iter().enumerate().skip(start).take(rows) {
        if let Some(file) = dataset.filesystem.files.get(index) {
            let text = format!(
                "{} {} {:>8} B",
                if position == app.raw_file_index {
                    "›"
                } else {
                    " "
                },
                fit(&file.path, width.saturating_sub(15)),
                file.size_bytes
            );
            lines.push(Line::styled(
                text,
                if position == app.raw_file_index {
                    Tone::Selected
                } else {
                    Tone::Normal
                },
            ));
        }
    }
    if let Some(file) = visible
        .get(app.raw_file_index)
        .and_then(|&i| dataset.filesystem.files.get(i))
    {
        note(&mut lines, format!(" SHA-256: {}", file.sha256));
        note(
            &mut lines,
            format!(
                " {} segmentos · tamanho {} bytes",
                file.segments.len(),
                file.size_bytes
            ),
        );
        if let Some(s) = file.segments.first() {
            note(
                &mut lines,
                format!(
                    " Origem: FC2 0x{:08x} · memória 0x{:08x}",
                    s.file_offset, s.memory_address
                ),
            );
        }
        note(
            &mut lines,
            format!(
                " Bytes iniciais: {}",
                file.raw_hex.get(..file.raw_hex.len().min(64)).unwrap_or("")
            ),
        );
    }
    if visible.is_empty() {
        warning(&mut lines, " Nenhum arquivo corresponde à busca.");
    }
    lines
}

fn hex(app: &App, width: usize, height: usize) -> Vec<Line> {
    let Some(file) = app.file.as_ref() else {
        return empty(app);
    };
    let bytes = if app.hex_memory {
        match file.image.as_ref() {
            Some(image) => &image.bytes,
            None => {
                return vec![Line::styled(
                    " Memória reconstruída indisponível; m retorna ao FC2.",
                    Tone::Warning,
                )];
            }
        }
    } else {
        &file.bytes
    };
    let offset = app.hex_offset.min(bytes.len());
    let count = if width >= 78 {
        16
    } else if width >= 46 {
        8
    } else {
        4
    };
    let mut lines = vec![Line::new(format!(
        " {} · {} bytes · offset 0x{offset:x} ({offset})",
        if app.hex_memory {
            "Memória reconstruída"
        } else {
            "FC2 original"
        },
        bytes.len()
    ))];
    note(
        &mut lines,
        " m alterna FC2/memória · g vai ao offset · ↑↓ linhas · PgUp/PgDn páginas",
    );
    let rows = height.saturating_sub(lines.len());
    let length = rows
        .saturating_mul(count)
        .min(bytes.len().saturating_sub(offset));
    for (index, chunk) in bytes[offset..offset + length].chunks(count).enumerate() {
        let address = offset + index * count;
        let values = chunk
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<Vec<_>>()
            .join(" ");
        let ascii = chunk
            .iter()
            .map(|&b| {
                if b.is_ascii_graphic() || b == b' ' {
                    char::from(b)
                } else {
                    '.'
                }
            })
            .collect::<String>();
        lines.push(Line::new(format!(
            " {address:08x}  {:width$}  |{ascii}|",
            values,
            width = count * 3 - 1
        )));
    }
    if offset == bytes.len() {
        note(&mut lines, " Fim do arquivo.");
    }
    lines
}

fn overview(app: &App, height: usize) -> Vec<Line> {
    let Some(file) = app.file.as_ref() else {
        return empty(app);
    };
    let mut lines = vec![
        Line::styled(" Evidências e limites da decodificação", Tone::Heading),
        Line::new(format!(" Arquivo: {}", file.path.display())),
        Line::new(format!(" Tamanho: {} bytes", file.bytes.len())),
    ];
    if let Some(dataset) = &file.dataset {
        lines.push(Line::new(format!(
            " SHA-256 FC2: {}",
            dataset.source_sha256
        )));
        lines.push(Line::new(format!(
            " SHA-256 memória: {}",
            dataset.memory_sha256
        )));
        lines.push(Line::new(format!(
            " {} trabalhos · {} arquivos · {} medições",
            dataset.works.len(),
            dataset.filesystem.files.len(),
            dataset.works.iter().map(Work::record_count).sum::<usize>()
        )));
        lines.push(Line::new(format!(
            " Setores: {} ativos · {} liberados · {} apagados · {} CRC verificados",
            dataset.filesystem.active_sector_count,
            dataset.filesystem.released_sector_count,
            dataset.filesystem.erased_sector_count,
            dataset.filesystem.crc_verified_count
        )));
        lines.push(Line::new(format!(
            " Sistema de arquivos: {}",
            dataset.filesystem.layout
        )));
        lines.push(Line::new(format!(" Validação: {}", dataset.validation)));
        let references: Vec<_> = dataset
            .works
            .iter()
            .filter(|w| w.reference_validation.as_ref().is_some_and(|r| r.accepted))
            .map(|w| w.number.to_string())
            .collect();
        if references.is_empty() {
            warning(
                &mut lines,
                " Comparação externa Leaf: nenhum trabalho validado por referência.",
            );
        } else {
            lines.push(Line::new(format!(
                " Referência Leaf aceita nos trabalhos: {}",
                references.join(", ")
            )));
        }
        note(
            &mut lines,
            " Outros trabalhos têm validação estrutural; equivalência Leaf não foi comprovada.",
        );
        if !dataset.unclassified_files.is_empty() {
            warning(
                &mut lines,
                format!(
                    " Arquivos sem interpretação científica: {}",
                    dataset.unclassified_files.len()
                ),
            );
            for path in &dataset.unclassified_files {
                note(&mut lines, format!("  {path}"));
            }
        }
        for work in &dataset.works {
            for error in &work.errors {
                warning(&mut lines, format!(" Trabalho {}: {error}", work.number));
            }
            for message in &work.warnings {
                warning(&mut lines, format!(" Trabalho {}: {message}", work.number));
            }
        }
        for limitation in &dataset.limitations {
            warning(&mut lines, format!(" Limite: {limitation}"));
        }
        for limitation in &dataset.filesystem.limitations {
            warning(&mut lines, format!(" Sistema de arquivos: {limitation}"));
        }
    }
    if let Some(error) = &file.decode_error {
        warning(&mut lines, format!(" Decodificação: {error}"));
    }
    if !file.overview.is_empty() {
        note(&mut lines, " Inspeção estrutural:");
        lines.extend(
            file.overview
                .lines()
                .map(|line| Line::new(format!(" {line}"))),
        );
    }
    scroll(lines, app.scroll, height)
}

fn help(app: &App, height: usize) -> Vec<Line> {
    let lines = vec![
        Line::styled(" Navegação", Tone::Heading),
        Line::new(" ↑↓ / j k    Selecionar item ou rolar detalhes"),
        Line::new(" PgUp PgDn   Navegar páginas"),
        Line::new(" Home End    Primeiro / último item"),
        Line::new(" 1 2 3 4 5   Trabalhos / Medições / Arquivos / Hex / Auditoria"),
        Line::new(" Tab         Alternar abas"),
        Line::new(" :           Menu de ações com busca e motivos de indisponibilidade"),
        Line::new(" Enter       Abrir trabalho, medição ou arquivo no navegador"),
        Line::new(" [ ]         Grupo anterior / próximo nas medições"),
        Line::new(" u / n       Trabalho anterior / próximo nas medições"),
        Line::new(" 6           Resumo por ponto do grupo; Enter filtra as medições"),
        Line::new(" i           Detalhes do trabalho ou da medição selecionada"),
        Line::new(" w           Avisos de data, GPS e configuração do trabalho"),
        Line::new(" t           Mostrar ou ocultar bytes e rastreabilidade técnica"),
        Line::new(" /           Busca em tempo real; Esc restaura o filtro anterior"),
        Line::new(" f           Mostrar somente trabalhos/medições com avisos"),
        Line::new(" l / Ctrl+L  Limpar busca, ponto e filtro de avisos"),
        Line::new(" Esc         Cancelar entrada ou voltar"),
        Line::new(" ?           Abrir esta ajuda"),
        Line::styled(" Arquivos e exportação", Tone::Heading),
        Line::new(" o           Escolher outro FC2 no navegador local"),
        Line::new(" O           Abrir FC2 pelo caminho completo"),
        Line::new(" d           Mudar diretório no navegador"),
        Line::new(" Espaço      Marcar/desmarcar trabalho para exportação"),
        Line::new(" A / U       Marcar todos os trabalhos visíveis / limpar todas as marcas"),
        Line::new(" M           Alternar exportação Completo / Simples"),
        Line::new(" e           Exportar trabalhos marcados; sem marcas, o trabalho atual"),
        Line::new(" E           Exportar todos os trabalhos no modo atual"),
        Line::new(" Completo    XLSX, ZIP, página de downloads e auditoria com dados brutos"),
        Line::new(" Simples     XLSX dos trabalhos selecionados, sem arquivos auxiliares"),
        Line::new(" Marcas ocultas por filtros também entram na exportação e."),
        Line::new(" x           Backup dos dados brutos"),
        Line::new(" a           Gravar relatório de análise"),
        Line::new(" p           Carregar perfil de validação de referência"),
        Line::new(" v           Validar referência JSON, XLSX original e gerar perfil"),
        Line::new(" J           Gravar JSON completo dos dados e da auditoria"),
        Line::new(" c           Exportar CSV restrito ao perfil de referência"),
        Line::new(" s           Alternar ordem original / pontos como o Leaf"),
        Line::new(" m / g       Hex: origem FC2/memória / ir ao offset decimal ou 0xhex"),
        Line::new(" q           Sair"),
        Line::new(" Ctrl+C      Sair e restaurar o terminal"),
        Line::new(" Ctrl+U      Limpar a entrada em edição"),
        Line::new(" ←→ Home End Editar posição do cursor em caminhos e buscas"),
        Line::new(" Backspace / Delete  Remover caractere antes / depois do cursor"),
        Line::styled(" Integridade", Tone::Heading),
        Line::new(" FC2 original permanece inalterado. Saídas existentes não são sobrescritas."),
        Line::new(" Ausências aparecem como — na tela e células vazias no XLSX."),
        Line::new(" Índices ICF são instrumentais. O total mantém seu próprio campo."),
        Line::new(" Médias e faixas são resumos calculados; os registros não são alterados."),
        Line::new(" Equivalência com o Leaf depende de referência do mesmo trabalho."),
        Line::new(" Campos científicos desconhecidos permanecem nos bytes e na auditoria."),
    ];
    scroll(lines, app.scroll, height)
}

fn browser(app: &App, width: usize, height: usize) -> Vec<Line> {
    let browser = &app.browser;
    let visible = app.visible_browser();
    let mut lines = vec![Line::new(format!(" {}", browser.directory.display()))];
    if let Some(destination) = &app.destination {
        let count = destination
            .selection
            .as_ref()
            .map(|s| s.len())
            .unwrap_or_else(|| app.dataset().map(|d| d.works.len()).unwrap_or(0));
        lines.push(Line::styled(
            format!(
                " Exportar {count} trabalho(s) · {}",
                match destination.mode {
                    crate::export::works::ExportMode::Simple => "Somente XLSX",
                    crate::export::works::ExportMode::Complete =>
                        "XLSX + auditoria e arquivos brutos",
                }
            ),
            Tone::Heading,
        ));
        note(
            &mut lines,
            " s seleciona ESTA pasta; Enter abre a pasta destacada.",
        );
        note(
            &mut lines,
            " Uma subpasta nova será criada; você confirma o nome antes de exportar.",
        );
    }
    note(
        &mut lines,
        format!(
            " Navegação local · / buscar · d caminho · busca: {}",
            if browser.search.is_empty() {
                "—"
            } else {
                &browser.search
            }
        ),
    );
    let rows = height.saturating_sub(lines.len());
    let start = window(browser.index, visible.len(), rows);
    for (position, &index) in visible.iter().enumerate().skip(start).take(rows) {
        if let Some(entry) = browser.entries.get(index) {
            let prefix = if entry.is_dir {
                "[pasta]"
            } else if entry
                .path
                .extension()
                .and_then(|s| s.to_str())
                .is_some_and(|s| s.eq_ignore_ascii_case("fc2"))
            {
                "[FC2]  "
            } else {
                "[arquivo]"
            };
            let text = format!(
                "{} {prefix} {}",
                if position == browser.index {
                    "›"
                } else {
                    " "
                },
                fit(
                    &entry.label,
                    width.saturating_sub(measure_text_width(prefix) + 3)
                )
            );
            lines.push(Line::styled(
                text,
                if position == browser.index {
                    Tone::Selected
                } else {
                    Tone::Normal
                },
            ));
        }
    }
    if visible.is_empty() {
        warning(&mut lines, " Nenhum arquivo ou pasta corresponde à busca.");
    }
    lines
}
fn scroll(lines: Vec<Line>, offset: usize, height: usize) -> Vec<Line> {
    let start = offset.min(lines.len().saturating_sub(height));
    lines.into_iter().skip(start).take(height).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn external_controls_are_removed_before_terminal_styling() {
        assert_eq!(
            clean("A\x1b[31mB\x1b[0m\x1b]2;evil\x07C\n\r\t\u{202e}D"),
            "ABCD"
        );
        assert_eq!(clean("x\x1b]8;;url\x1b\\y\x1b]8;;\x1b\\z"), "xyz");
    }
    #[test]
    fn clipping_handles_unicode_and_tiny_terminals() {
        for width in 0..20 {
            let value = fit("Medição 界 👩‍🔬 e\u{301}", width);
            assert!(measure_text_width(&value) <= width);
        }
        assert_eq!(fit("long", 0), "");
        assert_eq!(window(usize::MAX, 0, 0), 0);
    }
    #[test]
    fn prompt_keeps_the_edited_path_tail_visible() {
        let path = "/a/very/long/path/medição.FC2";
        assert!(input_window(path, path.len(), 18).ends_with("medição.FC2▏"));
        for width in 0..30 {
            assert!(
                measure_text_width(&input_window(
                    "/界/👩‍🔬/muito/longo/arquivo.FC2",
                    usize::MAX,
                    width
                )) <= width
            );
        }
    }
    #[test]
    fn every_screen_is_bounded_and_external_terminal_commands_are_absent() {
        let directory = tempfile::tempdir().unwrap();
        let mut app = App::new(directory.path()).unwrap();
        app.status = "unsafe\x1b[H\x1b]2;window\x07\u{202e}status".into();
        app.browser.entries.push(super::super::BrowserEntry {
            path: directory.path().join("sample.txt"),
            is_dir: false,
            label: "sample\x1b[2J\u{202e}.txt".into(),
        });
        app.prompt = Some(super::super::Prompt {
            label: "Exportar um arquivo em local seguro:".into(),
            value: "/tmp/very/long/path/medição.FC2".into(),
            action: super::super::PromptAction::Open,
            cursor: "/tmp/very/long/path/medição.FC2".len(),
            error: None,
            snapshot: None,
        });
        for screen in [
            Screen::Works,
            Screen::Measurements,
            Screen::Record,
            Screen::Files,
            Screen::Hex,
            Screen::Overview,
            Screen::Help,
            Screen::Browser,
            Screen::WorkDetails,
            Screen::Points,
            Screen::Warnings,
        ] {
            app.screen = screen;
            for width in [0, 1, 2, 10, 36, 80, 120] {
                for height in [0, 1, 2, 3, 8, 24] {
                    let rendered = render(&app, width, height);
                    assert!(rendered.lines().count() <= usize::from(height));
                    for line in rendered.lines() {
                        assert!(measure_text_width(line) <= usize::from(width));
                    }
                    assert!(!rendered.contains("\x1b[H"));
                    assert!(!rendered.contains("\x1b[2J"));
                    assert!(!rendered.contains("\x1b]"));
                    assert!(!rendered.contains('\u{202e}'));
                }
            }
        }
        app.screen = Screen::Browser;
        let rendered = render(&app, 80, 24);
        assert!(rendered.contains("Exportar um arquivo"));
        assert!(rendered.contains("medição.FC2▏"));
        assert!(rendered.contains("[arquivo]"));
    }
    #[test]
    fn work_preview_labels_calculated_statistics_and_preserves_absences() {
        use crate::{records::Segment, workspace::WorkGroup};
        let mut raw = [0u8; 48];
        raw[..5].copy_from_slice(&[1, 2, 3, 12, 3]);
        for (offset, value) in [(24, 123u16), (26, 45), (28, 169), (30, 1), (32, 24)] {
            raw[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
        }
        raw[36..40].copy_from_slice(&7u32.to_le_bytes());
        let first = crate::work_records::decode48(
            &raw,
            vec![Segment {
                file_offset: 0,
                memory_address: 0,
                length: 48,
            }],
        )
        .unwrap();
        raw[..5].fill(0);
        raw[32..34].fill(0);
        raw[36..40].copy_from_slice(&8u32.to_le_bytes());
        raw[40..44].copy_from_slice(&f32::NAN.to_le_bytes());
        raw[44..48].copy_from_slice(&f32::NAN.to_le_bytes());
        let second = crate::work_records::decode48(
            &raw,
            vec![Segment {
                file_offset: 48,
                memory_address: 48,
                length: 48,
            }],
        )
        .unwrap();
        let work = Work {
            directory: "/synthetic".into(),
            number: 1,
            name: Some("Nome completo sintético".into()),
            display_name: "Nome completo sintético".into(),
            name_source: "synthetic".into(),
            configuration_file: None,
            expected_groups: Some(1),
            groups: vec![WorkGroup {
                source_file: "/synthetic/1".into(),
                group: 1,
                source_sha256: String::new(),
                expected_records: Some(2),
                records: vec![first, second],
            }],
            reference_validation: None,
            errors: Vec::new(),
            warnings: Vec::new(),
        };
        let preview = encode(work_preview(&work, 80), 80);
        assert!(preview.contains("Nome completo sintético"));
        assert!(preview.contains("2024-03-12"));
        assert!(preview.contains("ICF calculado"));
        assert!(preview.contains("Faixas ICF"));
        assert!(preview.contains("Sem data 1"));
        assert!(preview.contains("GPS 1"));
        assert!(preview.contains("não comparado"));
        assert!(!preview.contains("SHA-256"));
    }
    #[test]
    fn action_menu_disabled_reason_and_prompt_errors_remain_visible() {
        let directory = tempfile::tempdir().unwrap();
        let mut app = App::new(directory.path()).unwrap();
        app.action_menu = Some(super::super::ActionMenu {
            index: 0,
            search: String::new(),
        });
        let actions = app.visible_actions();
        let index = actions.iter().position(|a| !a.enabled).unwrap();
        app.action_menu.as_mut().unwrap().index = index;
        let rendered = render(&app, 80, 24);
        assert!(rendered.contains("indisponível"));
        assert!(
            rendered.contains(
                &actions[index]
                    .description
                    .chars()
                    .take(18)
                    .collect::<String>()
            )
        );
        for (width, height) in [(0, 0), (1, 1), (10, 3), (40, 12), (80, 24), (120, 30)] {
            let rendered = render(&app, width, height);
            assert!(rendered.lines().count() <= usize::from(height));
            for line in rendered.lines() {
                assert!(measure_text_width(line) <= usize::from(width));
            }
        }
        app.action_menu = None;
        app.prompt(
            super::super::PromptAction::Open,
            "Abrir caminho",
            "abcdef.FC2".into(),
        );
        let prompt = app.prompt.as_mut().unwrap();
        prompt.cursor = 2;
        prompt.error = Some("Arquivo inexistente: corrija o caminho".into());
        let rendered = render(&app, 80, 24);
        assert!(rendered.contains("ab▏cdef.FC2"));
        assert!(rendered.contains("Arquivo inexistente"));
        assert_eq!(input_window("abcdef", 2, 20), "ab▏cdef");
    }

    #[test]
    fn work_checkbox_is_independent_of_cursor_and_bounded() {
        let work = Work {
            directory: "/trabalho135".into(),
            number: 135,
            name: Some("ALPHA".into()),
            display_name: "ALPHA".into(),
            name_source: "synthetic".into(),
            configuration_file: None,
            expected_groups: None,
            groups: Vec::new(),
            reference_validation: None,
            errors: Vec::new(),
            warnings: Vec::new(),
        };
        let cursor = work_line(&work, true, false, 80);
        assert!(cursor.text.starts_with("› [ ] 135"));
        let marked = work_line(&work, false, true, 80);
        assert!(marked.text.starts_with("  [x] 135"));
        assert!(!marked.text.contains('›'));
        for width in [0, 1, 4, 6, 20, 40, 68, 69, 80, 120] {
            for selected in [false, true] {
                for marked in [false, true] {
                    let rendered = encode(vec![work_line(&work, selected, marked, width)], width);
                    assert!(measure_text_width(&rendered) <= width);
                }
            }
        }
    }

    #[test]
    fn mode_and_total_marks_are_visible_from_opening_even_with_a_filter() {
        use crate::export::works::ExportMode;
        let directory = tempfile::tempdir().unwrap();
        let mut app = App::new(directory.path()).unwrap();
        assert!(
            render(&app, 100, 24)
                .lines()
                .next()
                .unwrap()
                .contains("Simples")
        );
        app.marked_works.extend([100, 200]);
        app.search = "apenas um trabalho".into();
        app.export_mode = ExportMode::Simple;
        let rendered = render(&app, 100, 24);
        let header = rendered.lines().next().unwrap();
        assert!(header.contains("Simples"));
        assert!(header.contains("2 marcados"));
        let narrow = render(&app, 36, 24);
        let narrow_header = narrow.lines().next().unwrap();
        assert!(narrow_header.contains("Simples"));
        assert!(narrow_header.contains("2 marcados"));
        app.screen = Screen::Works;
        assert!(context_keys(&app, 80).contains("Espaço"));
        assert!(context_keys(&app, 80).contains("M Modo"));
        for width in [0, 1, 10, 36, 80, 120] {
            for height in [0, 1, 3, 8, 24] {
                let rendered = render(&app, width, height);
                assert!(rendered.lines().count() <= usize::from(height));
                for line in rendered.lines() {
                    assert!(measure_text_width(line) <= usize::from(width));
                }
            }
        }
    }
}
