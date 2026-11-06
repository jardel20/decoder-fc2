#[path = "support/fixture.rs"]
mod fixture;
use std::sync::Arc;

#[test]
fn summary_and_detail_preserve_groups_missing_values_and_independent_total() {
    let loaded = fixture::loaded();
    let summary = fc2_desktop::summary(&loaded);
    assert_eq!(summary.records, 4);
    assert_eq!(summary.works[0].records, 3);
    assert_eq!(summary.works[0].points, 2);
    assert_eq!(summary.works[0].coordinates, 2);
    assert_eq!(summary.works[0].first_date.as_deref(), Some("2024-03-12"));
    let detail = fc2_desktop::work_detail(&loaded, &summary.sha256, 0).unwrap();
    assert_eq!(
        detail["groups"][0]["records"][0]["chlorophyll_total_index"],
        "16.9"
    );
    assert!(detail["groups"][1]["records"][0]["latitude"].is_null());
    assert!(detail["groups"][1]["records"][0]["date"].is_null());
    assert!(fc2_desktop::work_detail(&loaded, "stale source", 0).is_err());
    assert!(fc2_desktop::work_detail(&loaded, &summary.sha256, 99).is_err());
}

#[test]
fn exports_enforce_source_identity_selection_and_no_overwrite() {
    let loaded = Arc::new(fixture::loaded());
    let source = loaded.dataset.source_sha256.clone();
    let root = tempfile::tempdir().unwrap();
    let invalid = root.path().join("invalid");
    assert!(fc2_desktop::export(loaded.clone(), "stale", &[0], "simple", &invalid).is_err());
    assert!(fc2_desktop::export(loaded.clone(), &source, &[], "simple", &invalid).is_err());
    assert!(fc2_desktop::export(loaded.clone(), &source, &[999], "simple", &invalid).is_err());
    assert!(fc2_desktop::export(loaded.clone(), &source, &[0, 0], "simple", &invalid).is_err());
    assert!(fc2_desktop::export(loaded.clone(), &source, &[0, 1], "json", &invalid).is_err());
    assert!(fc2_desktop::export(loaded.clone(), &source, &[0], "unknown", &invalid).is_err());
    assert!(!invalid.exists());
    let path = root.path().join("output");
    let result = fc2_desktop::export(loaded.clone(), &source, &[0], "simple", &path).unwrap();
    assert_eq!(result.workbooks, 1);
    assert_eq!(result.failures, 0);
    assert!(fc2_desktop::export(loaded.clone(), &source, &[0], "simple", &path).is_err());
    let audit = root.path().join("audit.json");
    fc2_desktop::export(loaded.clone(), &source, &[0], "json", &audit).unwrap();
    assert!(fc2_desktop::export(loaded, &source, &[0], "json", &audit).is_err());
}

#[test]
fn locations_list_every_coordinate_with_source_identity() {
    let loaded = fixture::loaded();
    let source = loaded.dataset.source_sha256.clone();
    let summary = fc2_desktop::summary(&loaded);
    let points = fc2_desktop::locations(&loaded, &source).unwrap();
    assert_eq!(points.len(), summary.coordinates);
    assert!(
        points
            .iter()
            .all(|p| p.latitude.is_finite() && p.longitude.is_finite())
    );
    assert!(points.iter().all(|p| !p.work_name.is_empty()));
    assert!(fc2_desktop::locations(&loaded, "stale").is_err());
}
