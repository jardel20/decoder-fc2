//! Exploratory geographic rendering. Numeric GPS validation is not datum validation.
use std::path::PathBuf;

use crate::{
    icons::{self, Glyph},
    theme,
};
use eframe::egui::{self, Stroke};
use fc2_rs::explorer::{GeoPoint, LoadedDataset, RecordKey};
use walkers::{HttpOptions, HttpTiles, Map, MapMemory, Tiles, lon_lat};

const MERCATOR_LIMIT: f64 = 85.051_128_78;

pub struct MapPane {
    pub online: bool,
    memory: MapMemory,
    tiles: Option<HttpTiles>,
    context: egui::Context,
    viewport: egui::Vec2,
    cache: PathBuf,
    initialized: bool,
}

impl MapPane {
    pub fn new(context: &egui::Context) -> Self {
        let mut memory = MapMemory::default();
        let _ = memory.set_zoom(2.0);
        Self {
            online: true,
            memory,
            tiles: None,
            context: context.clone(),
            viewport: egui::vec2(640.0, 400.0),
            cache: cache_directory(),
            initialized: false,
        }
    }

    pub fn set_online(&mut self, online: bool) {
        self.online = online;
        if !online {
            // No further viewport requests are issued after opt-out. Requests
            // already in flight are managed by the library's drop lifecycle.
            self.tiles = None;
        }
    }

    pub fn center_on(&mut self, point: GeoPoint) {
        if renderable(&point) {
            self.memory
                .center_at(lon_lat(point.longitude, point.latitude));
            let _ = self.memory.set_zoom(17.0);
            self.initialized = true;
        }
    }

    /// Center on the geographic bounds, keeping the user-requested selection zoom.
    /// Empty or unrenderable sets leave the viewport untouched.
    pub fn center_points(&mut self, points: &[GeoPoint]) {
        let Some((longitude, latitude, _)) = fit_bounds(points, self.viewport) else {
            return;
        };
        self.memory.center_at(lon_lat(longitude, latitude));
        let _ = self.memory.set_zoom(17.0);
        self.initialized = true;
    }

    pub fn fit_points(&mut self, points: &[GeoPoint]) {
        let Some((longitude, latitude, zoom)) = fit_bounds(points, self.viewport) else {
            return;
        };
        self.memory.center_at(lon_lat(longitude, latitude));
        let _ = self.memory.set_zoom(zoom);
        self.initialized = true;
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        points: &[GeoPoint],
        selected_work: Option<usize>,
        selected_record: Option<RecordKey>,
        loaded: Option<&LoadedDataset>,
    ) -> Option<RecordKey> {
        if !self.initialized {
            self.fit_points(points);
        }
        ui.horizontal_wrapped(|ui| {
            if icons::button(ui, Glyph::Plus, "Ampliar").clicked() {
                let _ = self.memory.zoom_in();
            }
            if icons::button(ui, Glyph::Minus, "Reduzir").clicked() {
                let _ = self.memory.zoom_out();
            }
            if icons::button(ui, Glyph::ArrowsOut, "Enquadrar pontos").clicked() {
                self.fit_points(points);
            }
            ui.label(format!(
                "{} coordenadas • zoom {:.1}",
                points.len(),
                self.memory.zoom()
            ));
        });
        ui.small("Datum das coordenadas não confirmado; posição cartográfica exploratória. Arraste para mover; roda para zoom.");
        if self.online {
            ui.small("Tiles consultam um serviço externo e revelam a região visível. Sem rede, os pontos continuam disponíveis.");
        } else {
            ui.small("Modo local: pontos sem fundo cartográfico; nenhuma consulta de tiles.");
        }
        let unsupported = points.iter().filter(|point| !renderable(point)).count();
        if unsupported > 0 {
            ui.label(format!("{unsupported} coordenadas não representáveis em Web Mercator; consulte os valores na tabela."));
        }
        if points.is_empty() {
            ui.label("Este conjunto não contém coordenadas numéricas disponíveis e válidas. Nenhum ponto foi criado.");
        }
        // Keep attribution outside the map rectangle so it is never obscured by markers.
        ui.hyperlink_to(
            "© OpenStreetMap contributors",
            "https://www.openstreetmap.org/copyright",
        );
        let (_, canvas_rect) = ui.allocate_space(ui.available_size());
        self.viewport = canvas_rect.size();
        let mut canvas = ui.new_child(egui::UiBuilder::new().max_rect(canvas_rect));
        canvas.set_clip_rect(ui.clip_rect().intersect(canvas_rect));
        if self.online && self.tiles.is_none() {
            let options = HttpOptions {
                cache: Some(self.cache.clone()),
                user_agent: Some(walkers::HeaderValue::from_static(concat!(
                    "FC2Explorer/",
                    env!("CARGO_PKG_VERSION")
                ))),
                ..Default::default()
            };
            self.tiles = Some(HttpTiles::with_options(
                walkers::sources::OpenStreetMap,
                options,
                self.context.clone(),
            ));
        }
        let tiles = if self.online {
            self.tiles.as_mut().map(|tiles| tiles as &mut dyn Tiles)
        } else {
            None
        };
        let mut clicked = None;
        Map::new(tiles, &mut self.memory, lon_lat(0.0, 0.0))
            .zoom_with_ctrl(false)
            .show(&mut canvas, |ui, response, projector, _| {
                let clip = response.rect;
                let hover = response.hover_pos();
                let mut hits: Vec<(f32, &GeoPoint)> = Vec::new();
                // Linear painting avoids one egui widget and allocation per record.
                // Offscreen markers are culled before issuing drawing commands.
                for point in points.iter().filter(|point| renderable(point)) {
                    let position = projector
                        .project(lon_lat(point.longitude, point.latitude))
                        .to_pos2();
                    if !clip.expand(8.0).contains(position) {
                        continue;
                    }
                    let selected = selected_record == Some(point.key);
                    let related = selected_work == Some(point.key.work);
                    let color = if selected {
                        theme::MAP_SELECTED
                    } else if related {
                        theme::MAP_RELATED
                    } else {
                        theme::MAP_DEFAULT
                    };
                    let radius = if selected {
                        7.0
                    } else if related {
                        5.0
                    } else {
                        3.5
                    };
                    ui.painter().circle_filled(position, radius, color);
                    ui.painter().circle_stroke(
                        position,
                        radius,
                        Stroke::new(1.0, theme::MAP_MARKER_BORDER),
                    );
                    if let Some(pointer) = hover {
                        let distance = pointer.distance_sq(position);
                        if distance <= 100.0 {
                            hits.push((distance, point));
                        }
                    }
                }
                hits.sort_by(|(a, left), (b, right)| a.total_cmp(b).then(left.key.cmp(&right.key)));
                if !hits.is_empty() {
                    let selected_index = hits
                        .iter()
                        .position(|(_, point)| Some(point.key) == selected_record);
                    let mut index = selected_index
                        .or_else(|| {
                            hits.iter()
                                .position(|(_, point)| Some(point.key.work) == selected_work)
                        })
                        .unwrap_or(0);
                    if response.clicked() {
                        index = selected_index
                            .map(|index| (index + 1) % hits.len())
                            .unwrap_or(index);
                        clicked = Some(hits[index].1.key);
                    }
                    let point = hits[index].1;
                    egui::Tooltip::for_widget(response)
                        .at_pointer()
                        .gap(12.0)
                        .width(320.0)
                        .show(|ui| {
                            ui.spacing_mut().item_spacing.y = 4.0;
                            point_details(ui, loaded, point);
                            if hits.len() > 1 {
                                ui.separator();
                                ui.small(format!(
                                    "{} registros próximos; clique para alternar.",
                                    hits.len()
                                ));
                            }
                        });
                }
            });
        clicked
    }
}

fn point_details(ui: &mut egui::Ui, loaded: Option<&LoadedDataset>, point: &GeoPoint) {
    let Some(loaded) = loaded else {
        ui.label("Dados do registro indisponíveis.");
        return;
    };
    let Some(record) = loaded.record(point.key) else {
        ui.label("Registro indisponível.");
        return;
    };
    let Some(work) = loaded.work(point.key.work) else {
        ui.label("Trabalho indisponível.");
        return;
    };
    ui.label(egui::RichText::new(format!("Registro {}", record.id)).strong());
    ui.weak(&work.display_name);
    ui.separator();
    egui::Grid::new((
        "map-record-details",
        point.key.work,
        point.key.group,
        point.key.record,
    ))
    .num_columns(2)
    .min_row_height(24.0)
    .spacing([12.0, 3.0])
    .show(ui, |ui| {
        for (label, value) in [
            ("ID do trabalho", work.number.to_string()),
            (
                "Grupo",
                work.groups
                    .get(point.key.group)
                    .map(|group| group.group.to_string())
                    .unwrap_or_else(|| "—".into()),
            ),
            ("Ponto", record.point.to_string()),
            ("Data", record.date.clone().unwrap_or_else(|| "—".into())),
            ("Hora", record.time.clone().unwrap_or_else(|| "—".into())),
            ("Índice a", record.chlorophyll_a_index.clone()),
            ("Índice b", record.chlorophyll_b_index.clone()),
            ("Índice total", record.chlorophyll_total_index.clone()),
            (
                "Latitude",
                record
                    .latitude
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "—".into()),
            ),
            (
                "Longitude",
                record
                    .longitude
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "—".into()),
            ),
        ] {
            ui.weak(label);
            ui.label(value);
            ui.end_row();
        }
    });
    ui.separator();
    ui.label(egui::RichText::new(format!("Avisos ({})", record.warnings.len())).strong());
    if record.warnings.is_empty() {
        ui.small("Nenhum aviso neste registro.");
    } else {
        egui::ScrollArea::vertical()
            .id_salt((
                "map-warnings",
                point.key.work,
                point.key.group,
                point.key.record,
            ))
            .max_height(120.0)
            .show(ui, |ui| {
                for warning in &record.warnings {
                    ui.colored_label(
                        ui.visuals().warn_fg_color,
                        crate::warnings::message(warning).as_ref(),
                    )
                    .on_hover_text(warning);
                }
            });
    }
    ui.small("Datum não confirmado; localização cartográfica exploratória.");
}

fn renderable(point: &GeoPoint) -> bool {
    point.latitude.is_finite()
        && point.longitude.is_finite()
        && point.latitude.abs() <= MERCATOR_LIMIT
        && point.longitude.abs() <= 180.0
}

fn mercator_y(latitude: f64) -> f64 {
    (1.0 - latitude.to_radians().tan().asinh() / std::f64::consts::PI) / 2.0
}

fn fit_bounds(points: &[GeoPoint], viewport: egui::Vec2) -> Option<(f64, f64, f64)> {
    let mut longitudes: Vec<f64> = points
        .iter()
        .filter(|p| renderable(p))
        .map(|p| p.longitude.rem_euclid(360.0))
        .collect();
    if longitudes.is_empty() {
        return None;
    }
    longitudes.sort_by(f64::total_cmp);
    // Remove the largest gap on the circle, supporting points crossing the antimeridian.
    let mut largest_gap = -1.0;
    let mut start = 0;
    for index in 0..longitudes.len() {
        let next = if index + 1 == longitudes.len() {
            longitudes[0] + 360.0
        } else {
            longitudes[index + 1]
        };
        let gap = next - longitudes[index];
        if gap > largest_gap {
            largest_gap = gap;
            start = (index + 1) % longitudes.len();
        }
    }
    let longitude_span = 360.0 - largest_gap;
    let longitude = (longitudes[start] + longitude_span / 2.0 + 180.0).rem_euclid(360.0) - 180.0;
    let (mut y_min, mut y_max) = (f64::INFINITY, f64::NEG_INFINITY);
    for point in points.iter().filter(|p| renderable(p)) {
        let y = mercator_y(point.latitude);
        y_min = y_min.min(y);
        y_max = y_max.max(y);
    }
    let latitude = ((std::f64::consts::PI * (1.0 - y_min - y_max)).sinh())
        .atan()
        .to_degrees();
    let horizontal = ((viewport.x.max(100.0) - 48.0) as f64
        / (256.0 * (longitude_span / 360.0).max(1e-8)))
    .log2();
    let vertical =
        ((viewport.y.max(100.0) - 48.0) as f64 / (256.0 * (y_max - y_min).max(1e-8))).log2();
    Some((
        longitude,
        latitude,
        horizontal.min(vertical).clamp(0.0, 17.0),
    ))
}

fn cache_directory() -> PathBuf {
    #[cfg(target_os = "windows")]
    let base = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
    #[cfg(target_os = "macos")]
    let base = std::env::var_os("HOME").map(|home| PathBuf::from(home).join("Library/Caches"));
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")));
    base.unwrap_or_else(std::env::temp_dir)
        .join("fc2-explorer/tiles")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(longitude: f64, latitude: f64) -> GeoPoint {
        GeoPoint {
            key: RecordKey {
                work: 0,
                group: 0,
                record: 0,
            },
            longitude,
            latitude,
        }
    }

    #[test]
    fn empty_and_invalid_coordinates_have_no_bounds() {
        assert!(fit_bounds(&[], egui::vec2(800.0, 500.0)).is_none());
        assert!(
            fit_bounds(
                &[point(f64::NAN, 0.0), point(0.0, 90.0)],
                egui::vec2(800.0, 500.0)
            )
            .is_none()
        );
    }

    #[test]
    fn project_selection_centers_at_seventeen_and_missing_gps_preserves_view() {
        let mut pane = MapPane::new(&egui::Context::default());
        pane.center_points(&[point(-50.001, -20.0), point(-49.999, -20.0)]);
        assert_eq!(pane.memory.zoom(), 17.0);
        let center = pane.memory.detached().unwrap();
        assert!((center.x() + 50.0).abs() < 1e-9);
        assert!((center.y() + 20.0).abs() < 1e-9);
        let _ = pane.memory.set_zoom(12.0);
        pane.center_points(&[]);
        pane.center_points(&[point(f64::NAN, 0.0), point(0.0, 90.0)]);
        assert_eq!(pane.memory.zoom(), 12.0);
        assert_eq!(pane.memory.detached().unwrap(), center);
    }

    #[test]
    fn fit_crosses_antimeridian_without_centering_on_greenwich() {
        let (longitude, latitude, zoom) = fit_bounds(
            &[point(179.0, 10.0), point(-179.0, 10.0)],
            egui::vec2(800.0, 500.0),
        )
        .expect("valid bounds");
        assert!(longitude.abs() > 179.9);
        assert!((latitude - 10.0).abs() < 1e-9);
        assert!(zoom > 8.0);
    }

    #[test]
    fn online_default_is_lazy_and_can_be_disabled() {
        let mut pane = MapPane::new(&egui::Context::default());
        assert!(pane.online);
        assert!(pane.tiles.is_none());
        pane.set_online(false);
        assert!(!pane.online);
        assert!(pane.tiles.is_none());
    }
}
