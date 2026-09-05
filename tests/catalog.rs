use std::collections::BTreeSet;

// GUARD: catalog_lists_every_component_export — this is a guard; tests/mutations.rs must show it red.
#[test]
fn catalog_lists_every_component_export() {
    let manifest = include_str!("../src/components/mod.rs");
    let catalog = include_str!("../docs/CATALOG.md");

    let exported: BTreeSet<&str> = manifest
        .lines()
        .filter_map(|line| line.trim().strip_prefix("pub mod "))
        .filter_map(|rest| rest.strip_suffix(';'))
        .collect();
    let listed: BTreeSet<&str> = catalog
        .lines()
        .filter_map(|line| line.trim().strip_prefix("<!-- component: "))
        .filter_map(|rest| rest.strip_suffix(" -->"))
        .collect();

    assert!(!exported.is_empty(), "the component manifest was not read");
    assert!(
        !listed.is_empty(),
        "the catalogue contained no component entry"
    );
    assert_eq!(
        listed, exported,
        "a public component and its catalogue entry must exist together"
    );
}
