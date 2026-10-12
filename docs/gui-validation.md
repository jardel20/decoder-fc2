# GUI validation

## Dependency-backed checks on 2026-10-12

After dependencies became available in the local cache, GUI `cargo check` and
`cargo clippy --all-targets -- -D warnings` passed with the resolved lockfile.
Following the headless texture-delta fix, this command also passed:

```sh
cargo test --manifest-path gui/Cargo.toml --locked --offline --target x86_64-unknown-linux-gnu
```

Results: three map unit tests and one headless component test passed. The
component test renders both themes, embedded icons, cards and an empty offline
map. It explicitly clears texture deltas because it has no GPU painter; the
production eframe renderer remains responsible for applying those deltas.
Earlier cache/network limitations below describe the initial verification only.
Native-window execution, dialogs, release distribution and Windows remain
unverified by these automated checks.

After adding the in-app Linux filesystem picker, the same GUI Cargo test command
passed seven tests (five unit tests and two headless integration tests). Coverage
includes FC2 extension filtering, directory selection, invalid directories,
save-name containment and rendering of file/folder/JSON pickers without desktop
portals. GUI Clippy with all targets and warnings denied also passed. Native
dialogue failure was reported by the user; this environment cannot inspect the
session bus, so a missing portal is not a confirmed diagnosis.

## Scope and evidence

GUI navigation state lives in `gui/src/state.rs`, independently of egui. Integration tests in `tests/gui_state_tests.rs` include that production module and construct synthetic FC2 images that are decoded through the actual `LoadedDataset` API. Synthetic data tests application behavior; it does not confirm the real FC2 format or scientific equivalence.

Six navigation tests passed on 2026-10-12, with warnings denied:

- Installing a dataset resets prior file selection, errors, marked works and table caches.
- Selecting a work previews it; opening and returning preserve selection, search, table sorting and active tab.
- Map record keys link to the correct source-indexed work; invalid work, group and record indexes are ignored.
- Measurements filter by source group, warnings and search, preserve absent calendar values, and sort exact stored integers.
- Work search and ordering use stable source indexes rather than assuming stored identifiers are unique or sequential.

The checks were compiled using `rustc` against the current core source and already cached dependency artifacts, rather than reporting an unavailable Cargo GUI build as successful:

```sh
CARGO_PKG_VERSION=0.1.0 rustc --edition=2024 --test tests/gui_state_tests.rs \
  -L dependency=target/debug/deps \
  --extern fc2_rs=/tmp/libfc2_explorer_audit.rlib \
  -D warnings -o /tmp/fc2-gui-state-tests
/tmp/fc2-gui-state-tests
```

The temporary rlib is an environment-specific audit artifact, not a distributed dependency. A normal development environment runs these tests with Cargo after fetching dependencies.

## Data integrity review

`LoadedDataset` reuses the existing decoder. Record identity is a `(work, group, record)` source index and does not deduplicate equal stored IDs. Original bytes, offsets, raw coordinate bits, unknown fields and integer chlorophyll tenths remain in the decoded model. GUI sorting changes display order only.

XLSX and complete exports use the existing source and per-work validation gates. Structural JSON additionally checks the source hash and compares the selected work with a fresh decode before writing; existing output files cannot be overwritten silently. Structural JSON is labeled separately from scientific reference validation. The GUI does not offer unrestricted scientific CSV export without a reference profile.

A finite latitude/longitude inside numeric bounds is not confirmation of its geodetic datum. The maps module explicitly marks its cartographic placement as exploratory. No missing coordinates are replaced. Coordinates above the Web Mercator latitude limit remain in the table and are omitted from rendering with an explanatory notice.

## Remaining executable checks

The six state tests do not render a window, invoke native file dialogs, fetch tiles, or prove the GUI dependency versions compile together. Full GUI compilation and runtime validation require the declared crates to be available. Do not describe those checks as passed until actual execution succeeds.

When dependencies and a desktop session are available, run:

```sh
cargo fmt --all -- --check
cargo test
cargo check --manifest-path gui/Cargo.toml
cargo test --manifest-path gui/Cargo.toml
cargo clippy --manifest-path gui/Cargo.toml --all-targets -- -D warnings
cargo run --manifest-path gui/Cargo.toml
```

Manual desktop acceptance must cover native open/cancel, an invalid FC2, a real FC2, three-panel explorer, double-click/open/back, all detail tabs, theme changes, empty tables, missing GPS, offline markers, online tiles and attribution, XLSX/JSON success and errors, preserved values, and CLI/TUI regression execution. Tiles must remain opt-in and request only the current viewport; no bulk download is permitted.

Load cancellation is cooperative: results are discarded before/after decoding; an already-running decode is not forcibly interrupted. Export writes must not be presented as cancellable unless the worker actually supports cancellation. Only stage updates are implemented, not a fabricated percentage.

## Integration review of the desktop layer

A read-only review of `app.rs`, `tasks.rs`, `maps.rs`, `icons.rs` and `theme.rs` found the decoder API wired into native background loading, explorer selection, detail tabs, virtualized tables, record inspection and actual exports. Eight table columns are allocated and rendered. Work-detail point subsets are cached; table filters and queries are kept per work. Load is blocked during export, and export is blocked while another operation is busy. Separate pending IDs reject stale load results.

The [eframe 0.36.2 App API](https://docs.rs/eframe/0.36.2/eframe/trait.App.html) confirms the current `ui(&mut self, &mut Ui, &mut Frame)` callback and use of a central-panel frame. The [egui_extras 0.36.2 TableBuilder API](https://docs.rs/egui_extras/0.36.2/egui_extras/struct.TableBuilder.html) confirms the builder methods used for column allocation, identifiers, row sense and headers. Inspecting those selected APIs is not proof that the complete GUI compiles.

Points requiring desktop acceptance include native-dialog cancellation, keyboard Space selection when a widget has focus, viewport close behavior while an export is writing, narrow-window sizing and interactions with server/cache/network errors. Worker cancellation does not forcibly close an existing operating-system file dialog. Export result messages include per-work failures rather than claiming all selections succeeded.

## Coordinator final verification

The integrated core and presentation-independent state suite passed **119 tests**: 45 unit tests and 74 integration tests in 12 binaries. This includes all previous CLI/TUI/export checks, five explorer tests, three independent core regressions, and six navigation/cache tests. They were compiled using the actual Rust compiler and cached dependency rlibs with warnings denied. Core/CLI Clippy checks also passed against the declared core MSRV; this is not a GUI all-target lint result.

Formatting passed for both manifests, and git whitespace checks passed. Direct comparison against the private FC2 confirmed the new wrapper preserves the existing decoder output, record order and coordinate precision; the original file hash was unchanged. Experimental data were not added to the repository.

The coordinator corrected egui 0.36 global style APIs, text-edit focus handling, table cache source/work identity, native-dialog overlap and viewport close handling during export. These last graphical controls still require actual desktop execution. Map hit selection cycles through nearby overlapping records without inventing or collapsing source identities.

GUI Cargo check failed before compilation: offline resolution lacks eframe, and online resolution fails DNS for index.crates.io. Root Cargo commands similarly lack the console crate in the local cache. There is no generated GUI lockfile, native screenshot, GUI executable or successful GUI build to report. The three map tests and the headless egui component test are written but were not executed.
