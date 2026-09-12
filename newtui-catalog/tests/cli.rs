use std::process::Command;

#[test]
fn help_and_registry_are_available_without_a_terminal() {
    let help = Command::new(env!("CARGO_BIN_EXE_newtui-catalog"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(help.status.success());
    assert!(String::from_utf8(help.stdout)
        .unwrap()
        .contains("F1 returns to catalog"));
    let list = Command::new(env!("CARGO_BIN_EXE_newtui-catalog"))
        .arg("--list")
        .output()
        .unwrap();
    assert!(list.status.success());
    let listed = String::from_utf8(list.stdout).unwrap();
    let ids: Vec<_> = listed.lines().collect();
    assert_eq!(
        ids,
        [
            "settings_panel",
            "sparkline",
            "butterfly",
            "heat_meter",
            "gauge",
            "bar",
            "core_grid"
        ]
    );
}

#[test]
fn unknown_item_is_an_explicit_error_not_a_default_screenshot() {
    let output = Command::new(env!("CARGO_BIN_EXE_newtui-catalog"))
        .args(["--item", "does-not-exist"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("unknown item `does-not-exist`"));
}
