use std::collections::{BTreeMap, BTreeSet};

// GUARD: catalog_lists_every_component_export — this is a guard; tests/mutations.rs must show it red.
#[test]
fn catalog_lists_every_component_export() {
    let component_manifest = include_str!("../src/components/mod.rs");
    let widget_manifest = include_str!("../src/widget/mod.rs");
    let catalog = include_str!("../docs/CATALOG.md");

    let exported_components: BTreeSet<&str> = component_manifest
        .lines()
        .filter_map(|line| line.trim().strip_prefix("pub mod "))
        .filter_map(|rest| rest.strip_suffix(';'))
        .collect();
    let listed_components: BTreeSet<&str> = catalog
        .lines()
        .filter_map(|line| line.trim().strip_prefix("<!-- component: "))
        .filter_map(|rest| rest.strip_suffix(" -->"))
        .collect();

    assert!(
        !exported_components.is_empty(),
        "the component manifest was not read"
    );
    assert!(
        !listed_components.is_empty(),
        "the catalogue contained no component entry"
    );
    assert_eq!(
        listed_components, exported_components,
        "a public component and its catalogue entry must exist together"
    );

    // A builder is the public item whose name matches its module. Supporting
    // seam types are re-exported too, but they are not independently usable
    // widgets and therefore do not owe catalogue entries of their own.
    let exported_widgets: BTreeSet<&str> = widget_manifest
        .lines()
        .filter_map(|line| line.trim().strip_prefix("pub use "))
        .filter_map(|rest| rest.strip_suffix(';'))
        .filter_map(|rest| rest.split_once("::"))
        .filter_map(|(module, item)| (module == item).then_some(item))
        .collect();
    let listed_widgets: BTreeSet<&str> = catalog
        .lines()
        .filter_map(|line| line.trim().strip_prefix("<!-- widget: "))
        .filter_map(|rest| rest.strip_suffix(" -->"))
        .collect();

    assert!(
        !exported_widgets.is_empty(),
        "the widget manifest yielded no public builder"
    );
    assert!(
        !listed_widgets.is_empty(),
        "the catalogue contained no widget entry"
    );
    assert_eq!(
        listed_widgets, exported_widgets,
        "a public widget and its catalogue entry must exist together"
    );

    let demos: BTreeMap<&str, &str> = catalog
        .lines()
        .filter_map(|line| line.trim().strip_prefix("<!-- demo: "))
        .filter_map(|rest| rest.strip_suffix(" -->"))
        .map(|declaration| {
            declaration
                .split_once(" = ")
                .expect("a demo declaration maps one catalogue entry to one tape")
        })
        .collect();
    assert!(
        !demos.is_empty(),
        "the catalogue contained no demo declaration"
    );

    let catalogued: BTreeSet<&str> = listed_components
        .iter()
        .chain(&listed_widgets)
        .copied()
        .collect();
    let demonstrated: BTreeSet<&str> = demos.keys().copied().collect();
    assert_eq!(
        demonstrated, catalogued,
        "every catalogue entry must name exactly one demo"
    );

    let demo_names: BTreeSet<&str> = demos.values().copied().collect();
    assert_eq!(
        demo_names.len(),
        demos.len(),
        "one recording cannot stand in for two different catalogue entries"
    );

    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    for (piece, demo) in demos {
        for extension in ["tape", "gif", "png"] {
            let relative = format!("../demos/{demo}.{extension}");
            assert!(
                catalog.contains(&format!("({relative})")),
                "the `{piece}` entry does not link its {extension} demo"
            );
            assert!(
                root.join("demos")
                    .join(format!("{demo}.{extension}"))
                    .is_file(),
                "the `{piece}` entry names missing demo file demos/{demo}.{extension}"
            );
        }
    }
}
