//! Descriptive plots of stored instrument indices; derived statistics never
//! modify the decoder or exports. Point identity includes its source group.
use eframe::egui::{self, Color32};
use egui_plot::{Bar, BarChart, Line, Plot, PlotPoint};
use fc2_rs::{explorer::LoadedDataset, workspace::Work};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Mode {
    Raw,
    #[default]
    Mean,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Variable {
    #[default]
    Total,
    A,
    B,
    All,
}
impl Variable {
    fn label(self) -> &'static str {
        match self {
            Self::Total => "Clorofila total",
            Self::A => "Clorofila a",
            Self::B => "Clorofila b",
            Self::All => "Todas",
        }
    }
    fn series(self) -> &'static [usize] {
        match self {
            Self::A => &[0],
            Self::B => &[1],
            Self::Total => &[2],
            Self::All => &[0, 1, 2],
        }
    }
}
const NAMES: [&str; 3] = ["Clorofila a", "Clorofila b", "Clorofila total"];
const COLORS: [Color32; 3] = [
    Color32::from_rgb(51, 158, 132),
    Color32::from_rgb(219, 152, 65),
    Color32::from_rgb(94, 135, 216),
];

#[derive(Clone, Debug, Default)]
struct Statistics {
    n: usize,
    mean: f64,
    m2: f64,
}
impl Statistics {
    fn push(&mut self, value: f64) {
        self.n += 1;
        let delta = value - self.mean;
        self.mean += delta / self.n as f64;
        self.m2 += delta * (value - self.mean);
    }
    fn standard_error(&self) -> Option<f64> {
        (self.n > 1).then(|| (self.m2.max(0.0) / (self.n - 1) as f64 / self.n as f64).sqrt())
    }
}

#[derive(Clone)]
struct Row {
    label: String,
    group_index: usize,
    group: u16,
    point: u16,
    ids: Vec<u32>,
    values: [f64; 3],
    exact: Option<[String; 3]>,
    errors: [Option<f64>; 3],
    warnings: usize,
}

fn rows(work: &Work) -> (Vec<Row>, Vec<Row>) {
    let mut raw = Vec::with_capacity(work.record_count());
    let mut means: Vec<(Row, [Statistics; 3])> = Vec::new();
    let mut positions = BTreeMap::new();
    for (group_index, group) in work.groups.iter().enumerate() {
        for record in &group.records {
            let values = [
                record.chlorophyll_a_tenths,
                record.chlorophyll_b_tenths,
                record.chlorophyll_total_tenths,
            ]
            .map(|value| f64::from(value) / 10.0);
            raw.push(Row {
                label: record.id.to_string(),
                group_index,
                group: group.group,
                point: record.point,
                ids: vec![record.id],
                values,
                exact: Some([
                    record.chlorophyll_a_index.clone(),
                    record.chlorophyll_b_index.clone(),
                    record.chlorophyll_total_index.clone(),
                ]),
                errors: [None; 3],
                warnings: record.warnings.len(),
            });
            let index = *positions
                .entry((group_index, record.point))
                .or_insert_with(|| {
                    let index = means.len();
                    means.push((
                        Row {
                            label: format!("G{} · P{}", group.group, record.point),
                            group_index,
                            group: group.group,
                            point: record.point,
                            ids: Vec::new(),
                            values: [0.0; 3],
                            exact: None,
                            errors: [None; 3],
                            warnings: 0,
                        },
                        std::array::from_fn(|_| Statistics::default()),
                    ));
                    index
                });
            let (row, statistics) = &mut means[index];
            row.ids.push(record.id);
            row.warnings += record.warnings.len();
            for (statistic, value) in statistics.iter_mut().zip(values) {
                statistic.push(value);
            }
        }
    }
    let means = means
        .into_iter()
        .map(|(mut row, statistics)| {
            row.values = statistics.each_ref().map(|statistic| statistic.mean);
            row.errors = statistics.each_ref().map(Statistics::standard_error);
            row
        })
        .collect();
    (raw, means)
}

#[derive(Default)]
pub struct ChartPane {
    pub mode: Mode,
    pub variable: Variable,
    source: Option<Arc<LoadedDataset>>,
    work: Option<usize>,
    raw: Vec<Row>,
    means: Vec<Row>,
}
impl ChartPane {
    pub fn show(&mut self, ui: &mut egui::Ui, loaded: &Arc<LoadedDataset>, work_index: usize) {
        let Some(work) = loaded.dataset.works.get(work_index) else {
            ui.label("Trabalho indisponível.");
            return;
        };
        if self.work != Some(work_index)
            || !self
                .source
                .as_ref()
                .is_some_and(|source| Arc::ptr_eq(source, loaded))
        {
            (self.raw, self.means) = rows(work);
            self.source = Some(loaded.clone());
            self.work = Some(work_index);
        }
        self.render(
            ui,
            work,
            egui::Id::new((&loaded.dataset.source_sha256, work_index)),
        );
    }

    fn render(&mut self, ui: &mut egui::Ui, work: &Work, identity: egui::Id) {
        let mut zoom = 1.0_f32;
        let mut reset = false;
        ui.horizontal_wrapped(|ui| {
            ui.selectable_value(&mut self.mode, Mode::Mean, "Média por ponto ± EP");
            ui.selectable_value(&mut self.mode, Mode::Raw, "Todos os pontos")
                .on_hover_text(
                    "Mostrar todas as medidas individuais, identificadas pelo ID da leitura",
                );
            egui::ComboBox::from_id_salt("chlorophyll-chart-variable")
                .selected_text(self.variable.label())
                .show_ui(ui, |ui| {
                    for variable in [Variable::Total, Variable::A, Variable::B, Variable::All] {
                        ui.selectable_value(&mut self.variable, variable, variable.label());
                    }
                });
            if crate::icons::icon_button(ui, crate::icons::Glyph::Plus, "Aproximar gráfico")
                .clicked()
            {
                zoom = 1.25;
            }
            if crate::icons::icon_button(ui, crate::icons::Glyph::Minus, "Afastar gráfico")
                .clicked()
            {
                zoom = 0.8;
            }
            if crate::icons::button(ui, crate::icons::Glyph::ArrowsOut, "Enquadrar").clicked() {
                reset = true;
            }
        });
        let rows = if self.mode == Mode::Raw {
            &self.raw
        } else {
            &self.means
        };
        ui.small(format!("{} leituras · {} pontos por grupo · índices instrumentais. Role horizontalmente para ver todas as barras.", self.raw.len(), self.means.len()));
        if self.mode == Mode::Mean {
            ui.small("Média calculada por grupo e ponto. EP = desvio padrão amostral / √n; n = 1: EP indisponível. Leituras com avisos são mantidas.");
        }
        if rows.is_empty() {
            ui.label("Este trabalho não possui leituras para representar.");
            return;
        }
        let plot_height = (ui.available_height() - 110.0).max(140.0);
        let series = self.variable.series();
        ui.horizontal(|ui| {
            let font = egui::FontId::proportional(14.0);
            let title = "Legenda:";
            let width = std::iter::once(title)
                .chain(series.iter().map(|&v| NAMES[v]))
                .map(|label| {
                    ui.painter()
                        .layout_no_wrap(label.into(), font.clone(), ui.visuals().text_color())
                        .size()
                        .x
                })
                .sum::<f32>()
                + ui.spacing().item_spacing.x * series.len() as f32;
            ui.add_space(((ui.available_width() - width) / 2.0).max(0.0));
            ui.label(egui::RichText::new(title).size(14.0));
            for &variable in series {
                ui.label(
                    egui::RichText::new(NAMES[variable])
                        .color(COLORS[variable])
                        .size(14.0),
                );
            }
        });
        let plot_width = ui
            .available_width()
            .max(rows.len() as f32 * if series.len() == 3 { 72.0 } else { 48.0 } + 90.0);
        egui::ScrollArea::both()
            .max_height(ui.available_height())
            .auto_shrink([false, false])
            .id_salt(("charts-scroll", identity, self.mode, self.variable))
            .show(ui, |ui| {
                ui.set_min_width(plot_width);
                let error_color = ui.visuals().text_color();
                let plot = Plot::new(("chlorophyll-bars", identity, self.mode, self.variable))
                    .width(plot_width)
                    .height(plot_height)
                    .show_axes([false, true])
                    .show_grid([false, false])
                    .allow_zoom(true)
                    .allow_double_click_reset(true)
                    .y_axis_label("Índice instrumental")
                    .include_y(0.0)
                    .include_x(-0.8)
                    .include_x(rows.len() as f64 + 0.3);
                let plot = if reset { plot.reset() } else { plot };
                let response = plot.show(ui, |plot_ui| {
                    if zoom != 1.0 {
                        plot_ui.zoom_bounds(egui::vec2(zoom, zoom), plot_ui.plot_bounds().center());
                    }
                    for (position, &variable) in series.iter().enumerate() {
                        let width = if series.len() == 3 { 0.24 } else { 0.65 };
                        let offset = if series.len() == 3 {
                            (position as f64 - 1.0) * 0.26
                        } else {
                            0.0
                        };
                        let bars = rows
                            .iter()
                            .enumerate()
                            .map(|(index, row)| {
                                Bar::new(index as f64 + offset, row.values[variable]).width(width)
                            })
                            .collect();
                        plot_ui.bar_chart(
                            BarChart::new(NAMES[variable], bars)
                                .color(COLORS[variable])
                                .allow_hover(false),
                        );
                        if self.mode == Mode::Mean {
                            for (index, row) in rows.iter().enumerate() {
                                if let Some(error) = row.errors[variable] {
                                    let x = index as f64 + offset;
                                    let low = row.values[variable] - error;
                                    let high = row.values[variable] + error;
                                    for points in [
                                        vec![[x, low], [x, high]],
                                        vec![[x - width * 0.3, low], [x + width * 0.3, low]],
                                        vec![[x - width * 0.3, high], [x + width * 0.3, high]],
                                    ] {
                                        plot_ui.line(
                                            Line::new("", points)
                                                .color(error_color)
                                                .width(1.5)
                                                .allow_hover(false),
                                        );
                                    }
                                }
                            }
                        }
                    }
                });
                let (_, labels_rect) = ui.allocate_space(egui::vec2(plot_width, 75.0));
                for (index, row) in rows.iter().enumerate() {
                    let x = response
                        .transform
                        .position_from_point(&PlotPoint::new(index as f64, 0.0))
                        .x;
                    if x < ui.clip_rect().left() - 90.0 || x > ui.clip_rect().right() {
                        continue;
                    }
                    let galley = ui.painter().layout_no_wrap(
                        row.label.clone(),
                        egui::FontId::proportional(12.0),
                        ui.visuals().text_color(),
                    );
                    let mut text = egui::epaint::TextShape::new(
                        egui::pos2(x - 3.0, labels_rect.top() + 3.0),
                        galley,
                        ui.visuals().text_color(),
                    );
                    text.angle = std::f32::consts::FRAC_PI_4;
                    ui.painter().add(text);
                }
                ui.small(if self.mode == Mode::Raw {
                    "ID da leitura (categorias na ordem original)"
                } else {
                    "Grupo · ponto (ordem da primeira leitura)"
                });
                if let Some(cursor) = response.response.hover_pos() {
                    'rows: for (index, row) in rows.iter().enumerate() {
                        for (position, &variable) in series.iter().enumerate() {
                            let offset = if series.len() == 3 {
                                (position as f64 - 1.0) * 0.26
                            } else {
                                0.0
                            };
                            let width = if series.len() == 3 { 0.24 } else { 0.65 };
                            let left = response.transform.position_from_point(&PlotPoint::new(
                                index as f64 + offset - width / 2.0,
                                0.0,
                            ));
                            let right = response.transform.position_from_point(&PlotPoint::new(
                                index as f64 + offset + width / 2.0,
                                row.values[variable],
                            ));
                            let hit =
                                egui::Rect::from_two_pos(left, right).expand2(egui::vec2(0.0, 2.0));
                            if hit.contains(cursor) {
                                egui::Tooltip::for_widget(&response.response)
                                    .at_pointer()
                                    .gap(12.0)
                                    .width(330.0)
                                    .show(|ui| {
                                        tooltip(ui, row, variable, self.mode, work);
                                    });
                                break 'rows;
                            }
                        }
                    }
                }
            });
    }
}

fn tooltip(ui: &mut egui::Ui, row: &Row, variable: usize, mode: Mode, work: &Work) {
    ui.spacing_mut().item_spacing.y = 4.0;
    ui.strong(NAMES[variable]);
    if mode == Mode::Raw {
        ui.label(format!("ID {}", row.ids[0]));
    }
    ui.label(format!("Grupo {} · ponto {}", row.group, row.point));
    ui.small(&work.groups[row.group_index].source_file);
    ui.separator();
    if let Some(exact) = &row.exact {
        ui.label(format!("Valor: {}", exact[variable]));
    } else {
        ui.label(format!("Média: {:.6}", row.values[variable]));
        ui.label(format!(
            "Erro padrão: {}",
            row.errors[variable]
                .map(|error| format!("{error:.6}"))
                .unwrap_or_else(|| "indisponível (n = 1)".into())
        ));
        ui.label(format!("n: {} leituras", row.ids.len()));
        ui.label("IDs das leituras:");
        egui::ScrollArea::vertical()
            .max_height(110.0)
            .show(ui, |ui| {
                ui.label(
                    row.ids
                        .iter()
                        .map(u32::to_string)
                        .collect::<Vec<_>>()
                        .join(", "),
                );
            });
    }
    ui.small(format!("Avisos nas leituras: {}", row.warnings));
}

#[cfg(test)]
mod tests {
    use super::*;
    fn work() -> Work {
        let groups = (0..2)
            .map(|group| {
                let records = (0..3)
                    .map(|index| {
                        let mut bytes = [0; 48];
                        // Intentionally duplicated IDs and point numbers across groups.
                        for (offset, value) in [
                            (24, 100_u16 + index * 100 + group * 1000),
                            (26, 50),
                            (28, 999),
                            (30, 1),
                        ] {
                            bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
                        }
                        bytes[36..40].copy_from_slice(&u32::from(index).to_le_bytes());
                        fc2_rs::work_records::decode48(
                            &bytes,
                            vec![fc2_rs::records::Segment {
                                file_offset: 0,
                                memory_address: 0,
                                length: 48,
                            }],
                        )
                        .unwrap()
                    })
                    .collect();
                fc2_rs::workspace::WorkGroup {
                    source_file: format!("group-{group}"),
                    group,
                    source_sha256: String::new(),
                    expected_records: None,
                    records,
                }
            })
            .collect();
        Work {
            directory: "synthetic".into(),
            number: 1,
            name: None,
            display_name: "Synthetic test".into(),
            name_source: String::new(),
            configuration_file: None,
            expected_groups: None,
            groups,
            reference_validation: None,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    #[test]
    fn means_keep_groups_separate_and_use_stored_total() {
        let (raw, means) = rows(&work());
        assert_eq!(raw.len(), 6);
        assert_eq!(means.len(), 2);
        assert_eq!(raw[0].ids, raw[3].ids); // Both preserved, not deduplicated.
        assert_eq!(means[0].values, [20.0, 5.0, 99.9]);
        assert!((means[1].values[0] - 120.0).abs() < 1e-12);
        assert!(means[0].warnings > 0); // Calendar warnings never discard indices.
    }

    #[test]
    fn charts_render_all_modes_variables_and_empty_state_in_both_themes() {
        let work = work();
        let (raw, means) = rows(&work);
        let mut pane = ChartPane {
            raw,
            means,
            ..Default::default()
        };
        let ctx = egui::Context::default();
        crate::icons::install_fonts(&ctx);
        for dark in [true, false] {
            crate::theme::apply(&ctx, dark);
            for mode in [Mode::Raw, Mode::Mean] {
                for variable in [Variable::Total, Variable::A, Variable::B, Variable::All] {
                    pane.mode = mode;
                    pane.variable = variable;
                    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                        pane.render(ui, &work, egui::Id::new("synthetic-chart"));
                    });
                    output.textures_delta.clear();
                    assert!(!output.shapes.is_empty());
                }
            }
        }
        pane.raw.clear();
        pane.means.clear();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            pane.render(ui, &work, egui::Id::new("empty-chart"));
        });
        output.textures_delta.clear();
    }
    #[test]
    fn standard_error_is_sample_sd_over_root_n() {
        let mut statistics = Statistics::default();
        statistics.push(10.0);
        assert_eq!(statistics.standard_error(), None);
        statistics.push(20.0);
        statistics.push(30.0);
        assert_eq!(statistics.mean, 20.0);
        assert!((statistics.standard_error().unwrap() - 10.0 / 3.0_f64.sqrt()).abs() < 1e-12);
    }
    #[test]
    fn zeros_are_real_values_and_constant_samples_have_zero_error() {
        let mut statistics = Statistics::default();
        statistics.push(0.0);
        statistics.push(0.0);
        assert_eq!(statistics.standard_error(), Some(0.0));
        assert_eq!(statistics.mean, 0.0);
    }
}
