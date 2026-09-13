use newtui_corpus::{fixtures, model::Artifact};
use std::process::Command;

fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_newtui-corpus"))
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn the_rust_cli_regenerates_and_checks_the_shared_foreign_artifact() {
    let golden = include_bytes!("../fixtures/dial.json");
    let bytes = include_bytes!("../fixtures/dial.cbor");
    assert_eq!(fixtures::dial().unwrap().to_json().unwrap(), golden);
    assert_eq!(fixtures::dial().unwrap().canonical_bytes().unwrap(), bytes);
    let exported = run(&["export-dial"]);
    assert!(exported.status.success());
    assert_eq!(exported.stdout, golden);
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/dial.json");
    let checked = run(&["check", path]);
    assert!(checked.status.success());
    assert!(String::from_utf8(checked.stdout)
        .unwrap()
        .contains("36 observations, 24 transitions"));
    let canonical = run(&["canonical", path]);
    assert!(canonical.status.success());
    assert_eq!(canonical.stdout, bytes);
    assert_eq!(
        Artifact::from_json(golden).unwrap().id,
        "bafyr4ifzpp5ebpsr5scyoklyuwkcasqqmlscfpggqhioylqmxcaornrkau"
    );
}

#[test]
fn cli_errors_are_failures() {
    for args in [
        vec![],
        vec!["invalid"],
        vec!["check", "/no-such-newtui-corpus-file"],
    ] {
        let output = run(&args);
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8(output.stderr)
            .unwrap()
            .starts_with("newtui-corpus:"));
    }
}
