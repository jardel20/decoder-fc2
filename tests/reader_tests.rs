use fc2_rs::reader;

#[test]
fn ranges_are_checked_even_with_overflow() {
    assert!(reader::slice(&[1, 2], usize::MAX, 2).is_err());
    assert!(reader::slice(&[1, 2], 3, 0).is_err());
    assert!(reader::slice(&[1, 2], 2, 0).unwrap().is_empty());
    assert!(reader::u32_le(&[1, 2], 0).is_err());
}
#[test]
fn hexdump_retains_absolute_offset_and_ascii() {
    let text = reader::hexdump(b"xABC\0", 1, 4).unwrap();
    assert!(text.starts_with("00000001"));
    assert!(text.contains("41 42 43 00"));
    assert!(text.contains("ABC."));
}
#[test]
fn sha256_known_vector() {
    assert_eq!(
        reader::hash(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}
#[test]
fn input_read_does_not_change_original() {
    let t = tempfile::tempdir().unwrap();
    let p = t.path().join("input.FC2");
    std::fs::write(&p, b"original").unwrap();
    assert_eq!(reader::read_file(&p).unwrap(), b"original");
    assert_eq!(std::fs::read(&p).unwrap(), b"original");
}
