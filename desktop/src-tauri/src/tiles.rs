//! Map tiles are fetched here, not by the webview, so requests carry an identifying
//! User-Agent (OpenStreetMap tile policy), are cached on disk and never expose FC2 data.
use std::path::{Path, PathBuf};

pub const MAX_ZOOM: u8 = 19;

/// Rejects coordinates that are not valid slippy-map tile indexes.
pub fn validate(z: u8, x: u32, y: u32) -> Result<(), String> {
    if z > MAX_ZOOM {
        return Err("Nível de zoom inválido.".into());
    }
    let limit = 1u32 << z;
    if x >= limit || y >= limit {
        return Err("Tile fora do mapa.".into());
    }
    Ok(())
}

pub fn cache_path(root: &Path, z: u8, x: u32, y: u32) -> PathBuf {
    root.join(z.to_string())
        .join(x.to_string())
        .join(format!("{y}.png"))
}

pub fn cache_root() -> PathBuf {
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
        .join("fc2-explorer/tiles-desktop")
}

/// Returns the PNG bytes of a tile and whether they came from the network.
#[cfg(feature = "desktop")]
fn load(z: u8, x: u32, y: u32) -> Result<(Vec<u8>, bool), String> {
    use std::io::Read;
    validate(z, x, y)?;
    let path = cache_path(&cache_root(), z, x, y);
    if let Ok(bytes) = std::fs::read(&path)
        && bytes.starts_with(b"\x89PNG")
    {
        return Ok((bytes, false));
    }
    let url = format!("https://tile.openstreetmap.org/{z}/{x}/{y}.png");
    let mut response = ureq::get(&url)
        .header(
            "User-Agent",
            concat!(
                "FC2Explorer/",
                env!("CARGO_PKG_VERSION"),
                " (https://github.com/jardel20/Decoder_FC2)"
            ),
        )
        .call()
        .map_err(|e| format!("Tile indisponível: {e}"))?;
    let mut bytes = Vec::new();
    response
        .body_mut()
        .as_reader()
        .take(1_000_000)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if !bytes.starts_with(b"\x89PNG") {
        return Err("Resposta de tile inválida.".into());
    }
    if let Some(parent) = path.parent()
        && std::fs::create_dir_all(parent).is_ok()
    {
        let temporary = path.with_extension(format!("{:?}.tmp", std::thread::current().id()));
        if std::fs::write(&temporary, &bytes).is_ok() {
            let _ = std::fs::rename(&temporary, &path);
        }
    }
    Ok((bytes, true))
}

#[cfg(feature = "desktop")]
pub fn fetch(z: u8, x: u32, y: u32) -> Result<String, String> {
    use base64::Engine;
    let (bytes, _) = load(z, x, y)?;
    Ok(format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ))
}

/// Makes sure a tile is in the disk cache. Downloads are paced, in line with the
/// OpenStreetMap tile usage policy for bulk use. Returns true when it downloaded.
#[cfg(feature = "desktop")]
pub fn ensure(z: u8, x: u32, y: u32) -> Result<bool, String> {
    let (_, downloaded) = load(z, x, y)?;
    if downloaded {
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
    Ok(downloaded)
}

pub const MIN_OFFLINE_ZOOM: u8 = 10;
pub const MAX_OFFLINE_ZOOM: u8 = 18;
/// Upper bound for one offline download: a small area, never a bulk copy.
pub const MAX_OFFLINE_TILES: usize = 1200;

#[derive(serde::Deserialize, Clone, Copy, Debug)]
pub struct Bounds {
    pub south: f64,
    pub west: f64,
    pub north: f64,
    pub east: f64,
}

#[derive(serde::Serialize, Debug)]
pub struct Plan {
    pub tiles: Vec<(u8, u32, u32)>,
    pub min_zoom: u8,
    pub max_zoom: u8,
    pub cached: usize,
}

fn tile_x(lon: f64, z: u8) -> u32 {
    let n = f64::from(1u32 << z);
    (((lon + 180.0) / 360.0 * n).floor().max(0.0) as u32).min((1u32 << z) - 1)
}
fn tile_y(lat: f64, z: u8) -> u32 {
    let n = f64::from(1u32 << z);
    let y = (1.0 - lat.to_radians().tan().asinh() / std::f64::consts::PI) / 2.0 * n;
    (y.floor().max(0.0) as u32).min((1u32 << z) - 1)
}

/// Tiles covering `bounds` (plus a one-tile margin) from `MIN_OFFLINE_ZOOM` up to the
/// deepest zoom, at most `max_zoom`, that keeps the total within `limit`.
pub fn plan(bounds: Bounds, max_zoom: u8, limit: usize, root: &Path) -> Result<Plan, String> {
    const LIMIT: f64 = 85.051_128_78;
    let Bounds {
        south,
        west,
        north,
        east,
    } = bounds;
    if ![south, west, north, east].iter().all(|v| v.is_finite())
        || south > north
        || west > east
        || north.abs() > LIMIT
        || south.abs() > LIMIT
        || west < -180.0
        || east > 180.0
    {
        return Err("Região inválida.".into());
    }
    let ranges = |z: u8| {
        let (x0, x1) = (tile_x(west, z), tile_x(east, z));
        let (y0, y1) = (tile_y(north, z), tile_y(south, z));
        let max = (1u32 << z) - 1;
        (
            x0.saturating_sub(1),
            (x1 + 1).min(max),
            y0.saturating_sub(1),
            (y1 + 1).min(max),
        )
    };
    let count = |top: u8| {
        (MIN_OFFLINE_ZOOM..=top)
            .map(|z| {
                let (x0, x1, y0, y1) = ranges(z);
                (x1 - x0 + 1) as usize * (y1 - y0 + 1) as usize
            })
            .sum::<usize>()
    };
    let top = (MIN_OFFLINE_ZOOM..=max_zoom.min(MAX_ZOOM))
        .rev()
        .find(|&top| count(top) <= limit)
        .ok_or(
            "A região é extensa demais para uso offline; selecione um arquivo com área menor.",
        )?;
    let mut tiles = Vec::new();
    for z in MIN_OFFLINE_ZOOM..=top {
        let (x0, x1, y0, y1) = ranges(z);
        for x in x0..=x1 {
            for y in y0..=y1 {
                tiles.push((z, x, y));
            }
        }
    }
    let cached = tiles
        .iter()
        .filter(|&&(z, x, y)| cache_path(root, z, x, y).is_file())
        .count();
    Ok(Plan {
        tiles,
        min_zoom: MIN_OFFLINE_ZOOM,
        max_zoom: top,
        cached,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tile_indexes_are_validated_and_cached_under_the_root() {
        assert!(validate(0, 0, 0).is_ok());
        assert!(validate(17, 94_000, 87_000).is_ok());
        assert!(validate(20, 0, 0).is_err());
        assert!(validate(3, 8, 0).is_err());
        assert!(validate(3, 0, 8).is_err());
        let root = Path::new("/cache");
        assert_eq!(cache_path(root, 3, 2, 1), root.join("3/2/1.png"));
    }

    fn point(lon: f64, lat: f64) -> Bounds {
        Bounds {
            south: lat,
            west: lon,
            north: lat,
            east: lon,
        }
    }

    #[test]
    fn offline_plan_covers_the_region_and_respects_the_tile_limit() {
        let root = Path::new("/nonexistent-cache");
        // One point: a 3 x 3 neighbourhood per zoom, zoom 10 to 18.
        let single = plan(point(-45.00, -20.00), 18, MAX_OFFLINE_TILES, root).unwrap();
        assert_eq!((single.min_zoom, single.max_zoom), (10, 18));
        assert_eq!(single.tiles.len(), 9 * 9);
        assert!(
            single
                .tiles
                .iter()
                .all(|&(z, x, y)| validate(z, x, y).is_ok())
        );
        assert_eq!(single.cached, 0);
        // The point itself is inside the centre tile of every zoom.
        for z in 10..=18u8 {
            assert!(
                single
                    .tiles
                    .contains(&(z, tile_x(-45.00, z), tile_y(-20.00, z)))
            );
        }
        // A wider area lowers the deepest zoom instead of exceeding the limit.
        let wide = Bounds {
            south: -20.10,
            west: -45.10,
            north: -20.00,
            east: -44.90,
        };
        let reduced = plan_for(wide, root);
        assert!(reduced.max_zoom < 18 && reduced.tiles.len() <= MAX_OFFLINE_TILES);
        // Continental areas and invalid input are refused.
        let brazil = Bounds {
            south: -33.0,
            west: -73.0,
            north: 5.0,
            east: -34.0,
        };
        assert!(plan(brazil, 18, MAX_OFFLINE_TILES, root).is_err());
        assert!(plan(point(f64::NAN, 0.0), 18, 10, root).is_err());
        assert!(plan(point(0.0, 89.0), 18, 10, root).is_err());
    }

    fn plan_for(bounds: Bounds, root: &Path) -> Plan {
        plan(bounds, 18, MAX_OFFLINE_TILES, root).unwrap()
    }

    /// Network check: `cargo test --features desktop -- --ignored`.
    #[cfg(feature = "desktop")]
    #[test]
    #[ignore = "requires network access"]
    fn fetches_a_real_tile_with_identifying_user_agent() {
        let url = fetch(3, 2, 3).expect("tile");
        assert!(url.starts_with("data:image/png;base64,iVBOR"));
        assert!(cache_path(&cache_root(), 3, 2, 3).is_file());
    }
}
