use newtui::layout::{changed_panes, Direction, LayoutTree, PaneId, Rect, MAX_RATIO, MIN_RATIO};
use newtui::{Component, Explorer, Fingerprint, Flow, Key, Named, PropertyOutcome, Row, View};

fn rect(x: u16, y: u16, width: u16, height: u16) -> Rect {
    Rect {
        x,
        y,
        width,
        height,
    }
}

fn nested(direction: Direction) -> LayoutTree {
    let other = match direction {
        Direction::Horizontal => Direction::Vertical,
        Direction::Vertical => Direction::Horizontal,
    };
    let mut tree = LayoutTree::single(10);
    assert!(tree.split(10, 20, direction, 0.8));
    assert!(tree.split(10, 30, other, 0.5));
    tree
}

// GUARD: bsp_ratio_resize_round_trip_and_paths_are_exact — tests/mutations.rs must show it red.
#[test]
fn bsp_ratio_resize_round_trip_and_paths_are_exact() {
    let mut tree = nested(Direction::Horizontal);
    let original = tree.clone();
    let large = rect(3, 5, 200, 24);
    let small = rect(3, 5, 100, 24);
    let expected = vec![
        (10, rect(3, 5, 159, 11)),
        (30, rect(3, 17, 159, 12)),
        (20, rect(163, 5, 40, 24)),
    ];
    assert_eq!(tree.rects(large), expected);
    assert_eq!(
        tree.rects(small),
        vec![
            (10, rect(3, 5, 79, 11)),
            (30, rect(3, 17, 79, 12)),
            (20, rect(83, 5, 20, 24)),
        ]
    );
    assert_eq!(tree.rects(large), expected);
    assert_eq!(tree, original, "projection never rewrites stored ratios");

    let borders = tree.splits(large);
    assert_eq!(borders.len(), 2);
    assert_eq!(borders[0].path, Vec::<bool>::new());
    assert_eq!(borders[0].direction, Direction::Horizontal);
    assert_eq!(borders[0].pos, 162);
    assert_eq!(borders[0].area, large);
    assert_eq!(borders[0].ratio.to_bits(), 0.8_f32.to_bits());
    assert_eq!(borders[1].path, vec![false]);
    assert_eq!(borders[1].direction, Direction::Vertical);
    assert_eq!(borders[1].pos, 16);
    assert_eq!(borders[1].area, rect(3, 5, 159, 24));
    for border in &borders {
        assert!(tree.set_ratio_at(&border.path, border.ratio));
    }
    assert_eq!(tree, original);

    assert!(tree.set_ratio_at(&[false], 0.25));
    assert_eq!(tree.splits(large)[0], borders[0]);
    assert_eq!(tree.rects(large)[2], expected[2]);
    assert_eq!(tree.rects(large)[0].1.height, 5);
    assert_eq!(tree.rects(large)[1].1.height, 18);
    assert_eq!(changed_panes(&expected, &tree.rects(large)), vec![10, 30]);
}

#[test]
fn invalid_edits_leave_the_whole_tree_unchanged() {
    let mut tree = nested(Direction::Horizontal);
    let original = tree.clone();
    for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert!(!tree.set_ratio_at(&[], value));
        assert!(!tree.split(20, 40, Direction::Vertical, value));
        assert_eq!(tree, original);
    }
    for path in [&[true][..], &[false, true], &[false, false], &[true, true]] {
        assert!(!tree.set_ratio_at(path, 0.4));
        assert_eq!(tree, original);
    }
    assert!(!tree.split(99, 40, Direction::Vertical, 0.5));
    for duplicate in [10, 20, 30] {
        assert!(!tree.split(20, duplicate, Direction::Vertical, 0.5));
    }
    assert_eq!(tree, original);
    assert!(!LayoutTree::single(10).set_ratio_at(&[], 0.5));
}

#[test]
fn finite_ratios_clamp_and_host_ids_are_preserved() {
    let mut tree = LayoutTree::single(usize::MAX);
    assert!(tree.split(usize::MAX, 0, Direction::Vertical, -f32::MAX));
    let area = rect(0, 0, 12, 200);
    assert_eq!(tree.splits(area)[0].ratio.to_bits(), MIN_RATIO.to_bits());
    assert_eq!(tree.rects(area)[0].0, usize::MAX);
    assert_eq!(tree.rects(area)[1].0, 0);
    for value in [f32::MAX, 1.0, MAX_RATIO.next_up()] {
        assert!(tree.set_ratio_at(&[], value));
        assert_eq!(tree.splits(area)[0].ratio.to_bits(), MAX_RATIO.to_bits());
    }
    for value in [-f32::MAX, -0.0, 0.0, MIN_RATIO.next_down()] {
        assert!(tree.set_ratio_at(&[], value));
        assert_eq!(tree.splits(area)[0].ratio.to_bits(), MIN_RATIO.to_bits());
    }
    assert!(tree.set_ratio_at(&[], MIN_RATIO.next_up()));
    assert_eq!(
        tree.splits(area)[0].ratio.to_bits(),
        MIN_RATIO.next_up().to_bits()
    );
}

// GUARD: ten_available_cells_follow_the_stored_f32_proportion — tests/mutations.rs must show it red.
#[test]
fn ten_available_cells_follow_the_stored_f32_proportion() {
    for (ratio, first, second) in [
        (0.1, 1, 9),
        (0.5, 5, 5),
        (0.7, 7, 3),
        (0.7_f32.next_down(), 6, 4),
        (0.7_f32.next_up(), 7, 3),
        (0.9, 9, 1),
        (0.9_f32.next_down(), 8, 2),
        (0.9_f32.next_up(), 9, 1),
    ] {
        let mut tree = LayoutTree::single(1);
        assert!(tree.split(1, 2, Direction::Horizontal, ratio));
        let panes = tree.rects(rect(0, 0, 11, 1));
        assert_eq!((panes[0].1.width, panes[1].1.width), (first, second));
        assert_eq!(
            tree.splits(rect(0, 0, 11, 1))[0].ratio.to_bits(),
            ratio.clamp(MIN_RATIO, MAX_RATIO).to_bits()
        );
    }
}

#[test]
fn empty_and_tiny_areas_keep_leaf_identity_and_structural_paths() {
    for direction in [Direction::Horizontal, Direction::Vertical] {
        let mut tree = nested(direction);
        for area in [
            rect(7, 9, 0, 0),
            rect(7, 9, 0, 4),
            rect(7, 9, 4, 0),
            rect(7, 9, 1, 1),
        ] {
            let panes = tree.rects(area);
            assert_eq!(
                panes.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
                vec![10, 30, 20]
            );
            let borders = tree.splits(area);
            assert_eq!(borders.len(), 2);
            assert_eq!(borders[0].path, Vec::<bool>::new());
            assert_eq!(borders[1].path, vec![false]);
            for border in borders {
                assert!(tree.set_ratio_at(&border.path, border.ratio));
            }
            assert_eq!(partition_error(&tree, area), None);
        }
    }
    let mut tree = LayoutTree::single(4);
    assert!(tree.split(4, 8, Direction::Horizontal, 0.5));
    assert_eq!(
        tree.rects(rect(7, 9, 1, 1)),
        vec![(4, rect(7, 9, 0, 1)), (8, rect(8, 9, 0, 1))]
    );
    assert_eq!(tree.splits(rect(7, 9, 1, 1))[0].pos, 7);
    assert_eq!(
        tree.rects(rect(7, 9, 2, 1)),
        vec![(4, rect(7, 9, 0, 1)), (8, rect(8, 9, 1, 1))]
    );
    assert_eq!(
        tree.rects(rect(7, 9, 3, 1)),
        vec![(4, rect(7, 9, 1, 1)), (8, rect(9, 9, 1, 1))]
    );
}

#[test]
fn half_open_coordinate_edges_saturate_without_wrapping() {
    let tree = nested(Direction::Horizontal);
    let edge = u16::MAX;
    assert_eq!(
        LayoutTree::single(7).rects(rect(edge, 4, 1, 8)),
        vec![(7, rect(edge, 4, 0, 8))]
    );
    assert_eq!(
        LayoutTree::single(7).rects(rect(4, edge, 8, 1)),
        vec![(7, rect(4, edge, 8, 0))]
    );
    for area in [
        rect(edge, edge, edge, edge),
        rect(edge - 3, edge - 4, 200, 200),
        rect(0, 0, edge, edge),
    ] {
        assert_eq!(partition_error(&tree, area), None);
    }
    assert_eq!(
        tree.rects(rect(edge, edge, edge, edge)),
        vec![
            (10, rect(edge, edge, 0, 0)),
            (30, rect(edge, edge, 0, 0)),
            (20, rect(edge, edge, 0, 0))
        ]
    );
    assert!(tree
        .splits(rect(edge, edge, edge, edge))
        .iter()
        .all(|border| border.pos == edge && border.area.width == 0 && border.area.height == 0));
}

#[test]
fn paths_address_both_subtrees_without_moving_the_other_side() {
    let mut tree = nested(Direction::Horizontal);
    assert!(tree.split(20, 40, Direction::Vertical, 0.3));
    let area = rect(0, 0, 200, 24);
    let before = tree.rects(area);
    let borders = tree.splits(area);
    assert_eq!(
        borders
            .iter()
            .map(|border| border.path.clone())
            .collect::<Vec<_>>(),
        vec![vec![], vec![false], vec![true]]
    );
    assert_eq!(borders[2].area, rect(160, 0, 40, 24));
    for border in &borders {
        assert!(tree.set_ratio_at(&border.path, border.ratio));
    }
    assert_eq!(tree.rects(area), before);
    assert!(tree.set_ratio_at(&[true], 0.7));
    assert_eq!(&tree.splits(area)[..2], &borders[..2]);
    assert_eq!(&tree.rects(area)[..2], &before[..2]);
    assert_eq!(changed_panes(&before, &tree.rects(area)), vec![20, 40]);
    assert_eq!(partition_error(&tree, area), None);
}

// GUARD: changed_projection_reports_only_changed_added_or_removed_ids — tests/mutations.rs must show it red.
#[test]
fn changed_projection_reports_only_changed_added_or_removed_ids() {
    let mut tree = nested(Direction::Horizontal);
    let area = rect(0, 0, 200, 24);
    let before = tree.rects(area);
    assert_eq!(changed_panes(&before, &before), Vec::<PaneId>::new());
    assert_eq!(changed_panes(&[], &[]), Vec::<PaneId>::new());
    let reversed: Vec<_> = before.iter().rev().copied().collect();
    assert_eq!(changed_panes(&before, &reversed), Vec::<PaneId>::new());
    assert_eq!(changed_panes(&[], &before), vec![10, 20, 30]);
    assert_eq!(changed_panes(&before, &[]), vec![10, 20, 30]);
    assert_eq!(
        changed_panes(&before, &tree.rects(rect(1, 1, 200, 24))),
        vec![10, 20, 30]
    );
    assert_eq!(
        changed_panes(&before, &tree.rects(rect(0, 0, 100, 24))),
        vec![10, 20, 30]
    );
    assert!(tree.set_ratio_at(&[], 0.8_f32.next_up()));
    assert_eq!(
        changed_panes(&before, &tree.rects(area)),
        Vec::<PaneId>::new(),
        "a stored ratio change need not move any cells"
    );
    assert!(tree.split(20, 5, Direction::Vertical, 0.5));
    assert_eq!(changed_panes(&before, &tree.rects(area)), vec![5, 20]);
}

fn overlaps(a: Rect, b: Rect) -> bool {
    a.width > 0
        && a.height > 0
        && b.width > 0
        && b.height > 0
        && a.x < b.x + b.width
        && b.x < a.x + a.width
        && a.y < b.y + b.height
        && b.y < a.y + a.height
}

// A geometric oracle: disjoint bounded tiles plus the declared one-cell
// dividers must cover exactly the parent area. It does not reimplement ratios.
fn partition_error(tree: &LayoutTree, input: Rect) -> Option<String> {
    let area = rect(
        input.x,
        input.y,
        input.width.min(u16::MAX - input.x),
        input.height.min(u16::MAX - input.y),
    );
    let mut tiles: Vec<_> = tree
        .rects(input)
        .into_iter()
        .map(|(_, area)| area)
        .collect();
    for border in tree.splits(input) {
        if border.area.width > 0 && border.area.height > 0 {
            tiles.push(match border.direction {
                Direction::Horizontal => rect(border.pos, border.area.y, 1, border.area.height),
                Direction::Vertical => rect(border.area.x, border.pos, border.area.width, 1),
            });
        }
    }
    let mut covered = 0_u64;
    for (index, tile) in tiles.iter().enumerate() {
        if tile.x < area.x
            || tile.y < area.y
            || u32::from(tile.x) + u32::from(tile.width) > u32::from(area.x) + u32::from(area.width)
            || u32::from(tile.y) + u32::from(tile.height)
                > u32::from(area.y) + u32::from(area.height)
        {
            return Some(format!("tile {tile:?} escapes {area:?}"));
        }
        if tiles[..index].iter().any(|other| overlaps(*tile, *other)) {
            return Some(format!("overlapping tile {tile:?}"));
        }
        covered += u64::from(tile.width) * u64::from(tile.height);
    }
    (covered != u64::from(area.width) * u64::from(area.height))
        .then(|| format!("covered {covered} cells in {area:?}"))
}

fn ratios() -> [f32; 5] {
    [
        MIN_RATIO,
        MIN_RATIO.next_up(),
        0.5,
        MAX_RATIO.next_down(),
        MAX_RATIO,
    ]
}

const AREAS: [Rect; 7] = [
    Rect {
        x: 0,
        y: 0,
        width: 0,
        height: 0,
    },
    Rect {
        x: 0,
        y: 0,
        width: 1,
        height: 1,
    },
    Rect {
        x: 0,
        y: 0,
        width: 2,
        height: 3,
    },
    Rect {
        x: 3,
        y: 5,
        width: 8,
        height: 8,
    },
    Rect {
        x: 0,
        y: 0,
        width: 100,
        height: 24,
    },
    Rect {
        x: 0,
        y: 0,
        width: 200,
        height: 24,
    },
    Rect {
        x: u16::MAX - 3,
        y: u16::MAX - 4,
        width: 200,
        height: 24,
    },
];

// A finite host adapter for the continuous ratio input. No pane content or
// rendered text participates in state identity; the geometry stays standalone.
struct BoundedLayout {
    tree: LayoutTree,
    selected: usize,
    values: [usize; 2],
    area: usize,
}

impl BoundedLayout {
    fn new(direction: Direction) -> Self {
        let mut tree = nested(direction);
        assert!(tree.set_ratio_at(&[], 0.5));
        Self {
            tree,
            selected: 0,
            values: [2, 2],
            area: 0,
        }
    }
}

impl Component for BoundedLayout {
    fn handle(&mut self, key: Key) -> Flow {
        match key {
            Key::Tab => self.selected = 1 - self.selected,
            Key::PageDown => self.area = (self.area + 1) % AREAS.len(),
            Key::Left => self.values[self.selected] = self.values[self.selected].saturating_sub(1),
            Key::Right => self.values[self.selected] = (self.values[self.selected] + 1).min(4),
            _ => return Flow::Stay,
        }
        let path: &[bool] = if self.selected == 0 { &[] } else { &[false] };
        assert!(self
            .tree
            .set_ratio_at(path, ratios()[self.values[self.selected]]));
        Flow::Stay
    }

    fn view(&self) -> View {
        let mut view = View::titled("bounded geometry checks").row(Row::new(
            "partition",
            partition_error(&self.tree, AREAS[self.area]).unwrap_or_else(|| "ok".into()),
        ));
        for (index, border) in self.tree.splits(AREAS[self.area]).iter().enumerate() {
            view = view.row(Row::new(
                "stored ratio",
                if border.ratio.to_bits() == ratios()[self.values[index]].to_bits() {
                    "ok"
                } else {
                    "ratio drift"
                },
            ));
        }
        view
    }

    fn fingerprint(&self) -> Fingerprint {
        let mut fingerprint = Fingerprint::of(self.selected).and(self.area);
        for (index, border) in self.tree.splits(AREAS[self.area]).iter().enumerate() {
            fingerprint = fingerprint
                .and(self.values[index])
                .and(border.ratio.to_bits());
        }
        fingerprint
    }
}

// GUARD: bounded_bsp_ratios_are_exhaustively_clean — tests/mutations.rs must show it red.
#[test]
fn bounded_bsp_ratios_are_exhaustively_clean() {
    let geometry = Named::new(
        "derived geometry and ratios remain valid",
        |observation: &newtui::Observation<'_>| {
            let view = observation.view();
            if view.rows.len() != 3 {
                return PropertyOutcome::Violated("the three geometry checks disappeared".into());
            }
            match view.rows.iter().find(|row| row.value != "ok") {
                Some(row) => PropertyOutcome::Violated(format!("{}: {}", row.label, row.value)),
                None => PropertyOutcome::Held,
            }
        },
    );
    for direction in [Direction::Horizontal, Direction::Vertical] {
        let keys = [
            Key::Tab,
            Key::PageDown,
            Key::Left,
            Key::Right,
            Key::Other,
            Key::Ctrl('x'),
        ];
        let report = Explorer::new(keys).explore(|| BoundedLayout::new(direction), &[&geometry]);
        assert!(
            report.exhausted,
            "the finite ratio vocabulary must finish: {report}"
        );
        assert!(report.is_clean(), "{report}");
        assert_eq!(
            report.states,
            2 * 5 * 5 * AREAS.len(),
            "do not collapse distinct ratios that happen to draw the same cells"
        );
    }
}
