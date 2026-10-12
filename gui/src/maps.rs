//! Exploratory geographic rendering. Numeric GPS validation is not datum validation.
use std::path::PathBuf;

use crate::{
    icons::{self, Glyph},
    theme,
};
use eframe::egui::{self, Stroke};
use fc2_rs::explorer::{GeoPoint, RecordKey};
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
        Self {
            online: false,
            memory: MapMemory::default(),
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
            self.initialized = true;
        }
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
            let mut online = self.online;
            if ui
                .checkbox(&mut online, "Tiles OpenStreetMap online")
                .changed()
            {
                self.set_online(online);
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
        self.viewport = ui.available_size();
        if self.online && self.tiles.is_none() && !points.is_empty() {
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
        let tiles = if self.online && !points.is_empty() {
            self.tiles.as_mut().map(|tiles| tiles as &mut dyn Tiles)
        } else {
            None
        };
        let mut clicked = None;
        Map::new(tiles, &mut self.memory, lon_lat(0.0, 0.0))
            .zoom_with_ctrl(false)
            .show(ui, |ui, response, projector, _| {
                let clip = response.rect;
                let hover = response.hover_pos();
                let mut hits: Vec<(f32, &GeoPoint)> = Vec::new();
                // Linear painting avoids one egui widget and allocation per record.
                // Offscreen markers are culled before issuing drawing commands.
                for point in points.iter().filter(|point| renderable(point)) {
                    let position = projector.project(lon_lat(point.longitude, point.latitude)).to_pos2();
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
                    let radius = if selected { 7.0 } else if related { 5.0 } else { 3.5 };
                    ui.painter().circle_filled(position, radius, color);
                    ui.painter().circle_stroke(position, radius, Stroke::new(1.0, theme::MAP_MARKER_BORDER));
                    if let Some(pointer) = hover {
                        let distance = pointer.distance_sq(position);
                        if distance <= 100.0 {
                            hits.push((distance, point));
                        }
                    }
                }
                hits.sort_by(|(a, left), (b, right)| a.total_cmp(b).then(left.key.cmp(&right.key)));
                if let Some((_, point)) = hits.first() {
                    response.clone().on_hover_text(format!(
                        "{} registros próximos; clique para selecionar/alternar\nLatitude: {}\nLongitude: {}\nDatum não confirmado",
                        hits.len(), point.latitude, point.longitude
                    ));
                    if response.clicked() {
                        let index = hits.iter().position(|(_, p)| Some(p.key) == selected_record)
                            .map(|index| (index + 1) % hits.len()).unwrap_or(0);
                        clicked = Some(hits[index].1.key);
                    }
                }
            });
        clicked
    }
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
    fn offline_start_creates_no_downloader() {
        let pane = MapPane::new(&egui::Context::default());
        assert!(!pane.online);
        assert!(pane.tiles.is_none());
    }
}
