use newtui::components::linked_panes::{
    Correspondence, LinkMode, LinkedPanes, MappedLocation, MatchKind, PaneSide, Region,
};
use newtui::{properties, Component, Explorer, Flow, Key, View};

// GUARD: linked_cursors_follow_regions_without_losing_focus_roundtrips — tests/mutations.rs must show it red.
#[test]
fn linked_cursors_follow_regions_without_losing_focus_roundtrips() {
    let map = Correspondence::new(
        [20, 100],
        vec![Region {
            first: 0..10,
            second: 0..100,
        }],
    )
    .unwrap();
    let mut panes = LinkedPanes::new(map, [5, 5]);
    assert_eq!(panes.handle(Key::Down), Flow::Stay);
    assert_eq!(panes.position(PaneSide::First).cursor, Some(1));
    assert_eq!(panes.position(PaneSide::Second).cursor, Some(11));
    assert_eq!(panes.position(PaneSide::Second).offset, 7);
    assert_eq!(panes.relation().unwrap().mapping.kind, MatchKind::Region);
    let map = Correspondence::new([3, 2], vec![]).unwrap();
    let mut panes = LinkedPanes::new(map, [1, 1]);
    panes.set_mode(LinkMode::Proportional);
    panes.select(1);
    let before = panes.clone();
    panes.handle(Key::Tab);
    panes.handle(Key::BackTab);
    assert_eq!(
        panes, before,
        "focus changes must not round-trip a lossy mapping"
    );
}

// GUARD: saturated_navigation_preserves_the_other_selection_after_focus_changes — tests/mutations.rs must show it red.
#[test]
fn saturated_navigation_preserves_the_other_selection_after_focus_changes() {
    let mut panes = LinkedPanes::new(Correspondence::new([3, 2], vec![]).unwrap(), [1, 1]);
    panes.set_mode(LinkMode::Proportional);
    panes.select(1);
    panes.handle(Key::Tab);
    let before = panes.clone();
    panes.handle(Key::Up);
    assert_eq!(
        panes, before,
        "an arrow at the boundary did not move the active cursor"
    );
    panes.handle(Key::Home);
    assert_eq!(panes, before);
    // An explicit link-policy request reasserts the active side once, even
    // when the requested policy is already selected.
    panes.set_mode(LinkMode::Proportional);
    assert_eq!(panes.position(PaneSide::First).cursor, Some(0));
    assert_eq!(panes.relation().unwrap().source, PaneSide::Second);
}

// GUARD: missing_counterparts_keep_boundaries_instead_of_claiming_real_lines — tests/mutations.rs must show it red.
#[test]
fn missing_counterparts_keep_boundaries_instead_of_claiming_real_lines() {
    let map = Correspondence::new(
        [5, 2],
        vec![
            Region {
                first: 0..2,
                second: 0..2,
            },
            Region {
                first: 2..5,
                second: 2..2,
            },
        ],
    )
    .unwrap();
    let mut panes = LinkedPanes::new(map, [2, 2]);
    panes.select(4);
    assert_eq!(
        panes.relation().unwrap().mapping.target,
        MappedLocation::Boundary(2)
    );
    assert_eq!(panes.position(PaneSide::Second).cursor, Some(1));
    assert_eq!(panes.relation().unwrap().mapping.region, Some(1));
}

fn assert_bounds(panes: &LinkedPanes) {
    assert_eq!(panes.view().selection_count(), 1);
    for side in [PaneSide::First, PaneSide::Second] {
        let length = panes.correspondence().len(side);
        let position = panes.position(side);
        if length == 0 {
            assert_eq!(position.cursor, None);
            assert_eq!(position.offset, 0);
        } else {
            let cursor = position.cursor.unwrap();
            assert!(cursor < length);
            assert!(position.offset <= length.saturating_sub(position.height.max(1)));
            assert!(cursor >= position.offset);
            assert!(cursor - position.offset < position.height.max(1));
        }
    }
    if let Some(relation) = panes.relation() {
        let length = panes.correspondence().len(relation.source.other());
        match relation.mapping.target {
            MappedLocation::Row(row) => assert!(row < length),
            MappedLocation::Boundary(boundary) => assert!(boundary <= length),
        }
        if let Some(region) = relation.mapping.region {
            assert!(region < panes.correspondence().regions().len());
        }
    }
}

#[test]
fn empty_single_and_maximum_domains_remain_bounded_in_both_directions() {
    for lengths in [
        [0, 0],
        [0, 1],
        [1, 0],
        [1, 1],
        [1, 8],
        [8, 1],
        [3, 8],
        [usize::MAX, usize::MAX - 1],
    ] {
        for heights in [[0, 0], [1, 2], [usize::MAX, usize::MAX]] {
            let mut panes =
                LinkedPanes::new(Correspondence::new(lengths, vec![]).unwrap(), heights);
            for mode in [LinkMode::Locked, LinkMode::Proportional, LinkMode::Unlinked] {
                panes.set_mode(mode);
                for side in [PaneSide::First, PaneSide::Second] {
                    panes.set_focus(side);
                    for key in [
                        Key::End,
                        Key::Down,
                        Key::PageDown,
                        Key::PageUp,
                        Key::Home,
                        Key::Up,
                    ] {
                        panes.handle(key);
                        assert_bounds(&panes);
                    }
                    panes.select(usize::MAX);
                    assert_bounds(&panes);
                    panes.resize([0, usize::MAX]);
                    assert_bounds(&panes);
                    panes.resize(heights);
                }
            }
        }
    }
}

#[test]
fn gap_ties_use_actual_previous_endpoints_and_preserve_host_indices() {
    let map = Correspondence::new(
        [8, 10],
        vec![
            Region {
                first: 0..2,
                second: 0..3,
            },
            Region {
                first: 4..8,
                second: 7..10,
            },
        ],
    )
    .unwrap();
    let mut panes = LinkedPanes::new(map, [2, 2]);
    // Between actual rows1and4: row2chooses the preceding row1, not boundary2.
    panes.select(2);
    let mapping = panes.relation().unwrap().mapping;
    assert_eq!(mapping.kind, MatchKind::Gap);
    assert_eq!(mapping.region, Some(0));
    assert_eq!(mapping.target, MappedLocation::Row(2));
    panes.select(3);
    assert_eq!(panes.relation().unwrap().mapping.region, Some(1));
    panes.set_focus(PaneSide::Second);
    panes.select(5); // Equidistant from actual endpoints2and7? Row4is the tie below.
    assert_eq!(panes.relation().unwrap().mapping.region, Some(1));
    let map = Correspondence::new(
        [5, 7],
        vec![
            Region {
                first: 0..1,
                second: 0..2,
            },
            Region {
                first: 4..5,
                second: 5..7,
            },
        ],
    )
    .unwrap();
    let mut panes = LinkedPanes::new(map, [1, 1]);
    panes.select(2); // Equal distance from rows0and4.
    assert_eq!(panes.relation().unwrap().mapping.region, Some(0));
    assert_eq!(
        panes.relation().unwrap().mapping.target,
        MappedLocation::Row(0)
    );
}

#[test]
fn malformed_and_crossing_correspondences_are_rejected_without_reordering() {
    use newtui::components::linked_panes::CorrespondenceError as Error;
    assert_eq!(
        Correspondence::new(
            [4, 4],
            vec![Region {
                first: core::ops::Range { start: 3, end: 2 },
                second: 0..1
            }]
        ),
        Err(Error::Range {
            region: 0,
            side: PaneSide::First
        })
    );
    assert_eq!(
        Correspondence::new(
            [4, 4],
            vec![Region {
                first: 0..1,
                second: 0..5
            }]
        ),
        Err(Error::Range {
            region: 0,
            side: PaneSide::Second
        })
    );
    assert_eq!(
        Correspondence::new(
            [0, 0],
            vec![Region {
                first: 0..0,
                second: 0..0
            }]
        ),
        Err(Error::EmptyRegion { region: 0 })
    );
    assert_eq!(
        Correspondence::new(
            [4, 4],
            vec![
                Region {
                    first: 0..2,
                    second: 2..4
                },
                Region {
                    first: 2..4,
                    second: 0..2
                }
            ]
        ),
        Err(Error::Order {
            region: 1,
            side: PaneSide::Second
        })
    );
    assert_eq!(
        Correspondence::new(
            [4, 4],
            vec![
                Region {
                    first: 0..2,
                    second: 0..2
                },
                Region {
                    first: 1..4,
                    second: 2..4
                }
            ]
        ),
        Err(Error::Order {
            region: 1,
            side: PaneSide::First
        })
    );
    let map = Correspondence::new(
        [1, 3],
        vec![
            Region {
                first: 0..0,
                second: 0..2,
            },
            Region {
                first: 0..1,
                second: 2..3,
            },
        ],
    )
    .unwrap();
    let mut panes = LinkedPanes::new(map, [1, 1]);
    assert_eq!(panes.relation().unwrap().mapping.region, Some(1));
    panes.set_focus(PaneSide::Second);
    panes.select(0);
    assert_eq!(panes.relation().unwrap().mapping.region, Some(0));
    assert_eq!(
        panes.relation().unwrap().mapping.target,
        MappedLocation::Boundary(0)
    );
}

#[test]
fn unlinked_movement_resize_and_ignored_keys_have_distinct_contracts() {
    let map = Correspondence::new([8, 5], vec![]).unwrap();
    let mut panes = LinkedPanes::new(map, [3, 2]);
    panes.set_mode(LinkMode::Unlinked);
    for side in [PaneSide::First, PaneSide::Second] {
        panes.set_focus(side);
        let other = panes.position(side.other());
        for key in [
            Key::Down,
            Key::PageDown,
            Key::End,
            Key::Home,
            Key::PageUp,
            Key::Up,
        ] {
            panes.handle(key);
            assert_eq!(panes.position(side.other()), other);
            assert_eq!(panes.relation(), None);
            assert_bounds(&panes);
        }
    }
    for key in [
        Key::Enter,
        Key::Left,
        Key::Right,
        Key::Backspace,
        Key::Other,
        Key::Char('x'),
        Key::Ctrl('x'),
    ] {
        let before = panes.clone();
        assert_eq!(panes.handle(key), Flow::Stay);
        assert_eq!(panes, before);
        assert!(LinkedPanes::alphabet().contains(&key));
    }
    panes.select(4);
    let cursors = [
        panes.position(PaneSide::First).cursor,
        panes.position(PaneSide::Second).cursor,
    ];
    for heights in [[0, 0], [8, 8], [1, 3], [3, 2]] {
        panes.resize(heights);
        assert_eq!(
            [
                panes.position(PaneSide::First).cursor,
                panes.position(PaneSide::Second).cursor
            ],
            cursors
        );
        assert_bounds(&panes);
    }
    let before = panes.clone();
    assert_eq!(panes.handle(Key::Esc), Flow::Close(false));
    assert_eq!(panes, before);
}

// GUARD: linked_scroll_cursor_mode_and_correspondence_stay_in_the_fingerprint — tests/mutations.rs must show it red.
#[test]
fn linked_scroll_cursor_mode_and_correspondence_stay_in_the_fingerprint() {
    let seed = || LinkedPanes::new(Correspondence::new([8, 8], vec![]).unwrap(), [3, 3]);
    let mut low = seed();
    low.set_mode(LinkMode::Unlinked);
    low.select(2);
    let mut high = seed();
    high.set_mode(LinkMode::Unlinked);
    high.select(4);
    high.select(2);
    assert_eq!(
        low.position(PaneSide::First).cursor,
        high.position(PaneSide::First).cursor
    );
    assert_ne!(
        low.position(PaneSide::First).offset,
        high.position(PaneSide::First).offset
    );
    assert_ne!(low.view(), high.view());
    assert_ne!(low.fingerprint(), high.fingerprint());
    let mut moved = low.clone();
    moved.select(1);
    assert_ne!(low.fingerprint(), moved.fingerprint());
    let empty = || LinkedPanes::new(Correspondence::new([0, 0], vec![]).unwrap(), [0, 0]);
    let mut a = empty();
    let mut b = empty();
    a.set_mode(LinkMode::Proportional);
    b.set_mode(LinkMode::Unlinked);
    assert_ne!(a.fingerprint(), b.fingerprint());
    b.set_mode(LinkMode::Proportional);
    b.set_focus(PaneSide::Second);
    assert_ne!(a.fingerprint(), b.fingerprint());
    b.set_focus(PaneSide::First);
    b.resize([1, 0]);
    assert_ne!(a.fingerprint(), b.fingerprint());
    let full = LinkedPanes::new(
        Correspondence::new(
            [8, 8],
            vec![Region {
                first: 0..8,
                second: 0..8,
            }],
        )
        .unwrap(),
        [3, 3],
    );
    assert_ne!(full.fingerprint(), seed().fingerprint());
}

struct CheckedPanes(LinkedPanes);
impl Component for CheckedPanes {
    fn handle(&mut self, key: Key) -> Flow {
        let old = self.0.clone();
        let result = self.0.handle(key);
        assert_bounds(&self.0);
        if old.mode() == LinkMode::Unlinked && key != Key::Char('l') {
            assert_eq!(
                self.0.position(old.focus().other()),
                old.position(old.focus().other())
            );
        }
        result
    }
    fn view(&self) -> View {
        self.0.view()
    }
}

#[test]
fn the_bounded_linked_keyboard_walk_is_exhaustive() {
    let map = Correspondence::new(
        [5, 6],
        vec![
            Region {
                first: 0..2,
                second: 0..3,
            },
            Region {
                first: 2..4,
                second: 3..3,
            },
            Region {
                first: 4..5,
                second: 4..6,
            },
        ],
    )
    .unwrap();
    let escape = properties::escape_always_closes_without_applying();
    let selection = properties::selection_is_always_in_range();
    let report = Explorer::new(LinkedPanes::alphabet())
        .max_depth(64)
        .max_states(50_000)
        .explore(
            || CheckedPanes(LinkedPanes::new(map.clone(), [2, 2])),
            &[&escape, &selection],
        );
    assert!(report.exhausted, "{report}");
    assert!(report.is_clean(), "{report}");
    assert!(report.states > 100);
    eprintln!(
        "linked panes: {} states, {} transitions, exhausted={}",
        report.states, report.transitions, report.exhausted
    );
}

// GUARD: linked_panes_has_no_host_effect_entry_points — tests/mutations.rs must show it red.
#[test]
fn linked_panes_has_no_host_effect_entry_points() {
    let source = include_str!("../src/components/linked_panes.rs");
    for effect in [
        "std::fs",
        "std::io",
        "std::net",
        "std::process",
        "std::env",
        "std::thread",
        "std::time",
        "include!",
        "include_str!",
        "extern crate",
        "unsafe {",
    ] {
        assert!(
            !source.contains(effect),
            "component introduced a host effect path: {effect}"
        );
    }
    let imports: Vec<_> = source
        .lines()
        .filter(|line| line.starts_with("use "))
        .collect();
    assert_eq!(
        imports,
        [
            "use core::ops::Range;",
            "use crate::{Component, Flow, Key, Row, View};"
        ]
    );
    // Alongside the compiled leaf dependency gate, this pins the audited
    // standard-library boundary. It is not an effect system for arbitrary Rust.
}

#[test]
fn correspondence_errors_support_ordinary_host_error_propagation() {
    fn host() -> Result<(), Box<dyn core::error::Error>> {
        Correspondence::new(
            [0, 0],
            vec![Region {
                first: 0..0,
                second: 0..0,
            }],
        )?;
        Ok(())
    }
    assert_eq!(
        host().unwrap_err().to_string(),
        "region 0 is empty on both surfaces"
    );
    let range = Correspondence::new(
        [0, 1],
        vec![Region {
            first: 0..1,
            second: 0..1,
        }],
    )
    .unwrap_err();
    assert_eq!(range.to_string(), "region 0 has an invalid First range");
    let order = Correspondence::new(
        [2, 2],
        vec![
            Region {
                first: 0..2,
                second: 0..1,
            },
            Region {
                first: 1..2,
                second: 1..2,
            },
        ],
    )
    .unwrap_err();
    assert_eq!(
        order.to_string(),
        "region 1 overlaps or reverses the First order"
    );
}
