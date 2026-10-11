mod common;
use std::process::Command;

fn fc2() -> Command {
    Command::new(env!("CARGO_BIN_EXE_fc2"))
}

#[test]
fn tui_refuses_unattended_pipes_without_ansi_or_source_changes() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("input.FC2");
    let bytes = common::fixture().0;
    std::fs::write(&source, &bytes).unwrap();
    let result = fc2().arg("tui").arg(&source).output().unwrap();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("terminal"));
    assert!(result.stdout.is_empty());
    assert_eq!(std::fs::read(source).unwrap(), bytes);
}

#[test]
fn decode_without_profile_and_out_of_bounds_hex_fail_without_output() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("in.FC2");
    let output = dir.path().join("out.csv");
    std::fs::write(&input, common::fixture().0).unwrap();
    let result = fc2()
        .arg("decode")
        .arg(&input)
        .arg("--output")
        .arg(&output)
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("--profile"));
    assert!(!output.exists());
    assert!(
        !fc2()
            .arg("hex")
            .arg(&input)
            .args(["--offset", "0xffffffffffffffff", "--length", "1"])
            .status()
            .unwrap()
            .success()
    );
}
#[test]
fn analyze_cannot_replace_input_and_batch_isolates_errors() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("valid.FC2");
    let bytes = common::fixture().0;
    std::fs::write(&input, &bytes).unwrap();
    std::fs::write(dir.path().join("bad.fc2"), b"bad").unwrap();
    assert!(
        !fc2()
            .arg("analyze")
            .arg(&input)
            .arg("--output")
            .arg(&input)
            .status()
            .unwrap()
            .success()
    );
    assert_eq!(std::fs::read(&input).unwrap(), bytes);
    let out = dir.path().join("batch");
    let result = fc2()
        .arg("batch")
        .arg(dir.path())
        .arg("--output")
        .arg(&out)
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(out.join("valid.FC2.analysis.json").exists());
    assert!(!out.join("bad.fc2.analysis.json").exists());
    let report: serde_json::Value =
        serde_json::from_slice(&std::fs::read(out.join("batch-report.json")).unwrap()).unwrap();
    assert_eq!(report["failures"], 1);
    assert_eq!(report["files"].as_array().unwrap().len(), 2);
}
#[test]
fn validate_and_decode_through_cli_with_source_hash() {
    let dir = tempfile::tempdir().unwrap();
    let (bytes, reference) = common::fixture();
    let input = dir.path().join("in.FC2");
    let source = dir.path().join("reference.source");
    let reference_path = dir.path().join("reference.json");
    let profile = dir.path().join("profile.json");
    let output = dir.path().join("measurements.json");
    std::fs::write(&input, bytes).unwrap();
    std::fs::write(&source, b"synthetic reference").unwrap();
    std::fs::write(&reference_path, serde_json::to_vec(&reference).unwrap()).unwrap();
    assert!(
        fc2()
            .arg("validate")
            .arg(&input)
            .arg("--reference")
            .arg(&reference_path)
            .arg("--reference-source")
            .arg(&source)
            .arg("--output")
            .arg(&profile)
            .status()
            .unwrap()
            .success()
    );
    assert!(
        fc2()
            .arg("decode")
            .arg(&input)
            .arg("--profile")
            .arg(&profile)
            .args(["--format", "json"])
            .arg("--output")
            .arg(&output)
            .status()
            .unwrap()
            .success()
    );
    let exported: serde_json::Value =
        serde_json::from_slice(&std::fs::read(output).unwrap()).unwrap();
    assert_eq!(exported["records"][0]["id"], 42);
    assert_eq!(exported["records"][0]["chlorophyll_total_index"], "16.9");
}
