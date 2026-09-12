//! Small deterministic command line; selecting a fixture never contacts a service.

use crate::fixtures::{DiffPreview, Kind, Scenario};
use newtui::DiffGeometry;

pub const HELP: &str = "newtui-catalog — Shawn's custom TUI widgets\n\n\
Usage: newtui-catalog [--item NAME] [--scenario NAME] [--theme NAME] [--width N]\n\n\
  --item       settings_panel, sparkline, butterfly, heat_meter, gauge, bar, core_grid, diff\n\
  --scenario   normal, narrow, empty, error, long\n\
  --theme      dark, light\n\
  --width      Preview content columns, 1..200 (narrow starts at 8)\n\
  --geometry   Diff layout: unified, split, stat\n\
  --row-offset / --column-offset   Diff source window, starting at 0\n\
  --expanded   Expand all diff context runs\n\
  --list       Print shipped item IDs without opening a terminal\n\
  --help       Show this help\n\n\
Browse: arrows select/resize, / search, Enter interact, f fixture, t theme, r reset.\n\
Interact: F1 returns to catalog; Esc goes to the component. F2 fixture, F3 theme, F4 reset.\n\
Diff: g layout, e context, n notice, up/down rows, Shift-left/right columns, Home reset scroll.\n\
Ctrl-C exits from any mode. All examples use fixed local sample data.\n";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Theme {
    #[default]
    Dark,
    Light,
}

impl Theme {
    pub fn name(self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
        }
    }
    pub fn next(self) -> Self {
        match self {
            Self::Dark => Self::Light,
            Self::Light => Self::Dark,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Options {
    pub item: Kind,
    pub scenario: Scenario,
    pub theme: Theme,
    pub width: u16,
    pub diff: DiffPreview,
    pub list: bool,
    pub help: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            item: Kind::Settings,
            scenario: Scenario::Normal,
            theme: Theme::Dark,
            width: 48,
            diff: DiffPreview::default(),
            list: false,
            help: false,
        }
    }
}

impl Options {
    pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut options = Self::default();
        let mut args = args.into_iter();
        let mut explicit_width = false;
        while let Some(flag) = args.next() {
            if flag == "--help" || flag == "-h" {
                options.help = true;
                continue;
            }
            if flag == "--list" {
                options.list = true;
                continue;
            }
            if flag == "--expanded" {
                options.diff.expanded = true;
                continue;
            }
            if !matches!(
                flag.as_str(),
                "--item"
                    | "--scenario"
                    | "--theme"
                    | "--width"
                    | "--geometry"
                    | "--row-offset"
                    | "--column-offset"
            ) {
                return Err(format!("unknown option `{flag}`"));
            }
            let value = args.next().ok_or_else(|| format!("{flag} needs a value"))?;
            match flag.as_str() {
                "--item" => {
                    options.item = Kind::from_name(&value)
                        .ok_or_else(|| format!("unknown item `{value}`; use --list"))?
                }
                "--scenario" => {
                    options.scenario = Scenario::parse(&value)
                        .ok_or_else(|| format!("unknown scenario `{value}`"))?
                }
                "--theme" => {
                    options.theme = match value.as_str() {
                        "dark" => Theme::Dark,
                        "light" => Theme::Light,
                        _ => return Err(format!("unknown theme `{value}`")),
                    }
                }
                "--width" => {
                    options.width = value
                        .parse::<u16>()
                        .ok()
                        .filter(|width| (1..=200).contains(width))
                        .ok_or_else(|| "--width must be an integer from 1 to 200".to_string())?;
                    explicit_width = true;
                }
                "--geometry" => {
                    options.diff.geometry = match value.as_str() {
                        "unified" => DiffGeometry::Unified,
                        "split" => DiffGeometry::Split,
                        "stat" => DiffGeometry::Stat,
                        _ => return Err(format!("unknown diff geometry `{value}`")),
                    };
                }
                "--row-offset" | "--column-offset" => {
                    let offset = value
                        .parse::<usize>()
                        .map_err(|_| format!("{flag} must be a nonnegative integer"))?;
                    if flag == "--row-offset" {
                        options.diff.row_offset = offset;
                    } else {
                        options.diff.column_offset = offset;
                    }
                }
                _ => unreachable!(),
            }
        }
        if options.scenario == Scenario::Narrow && !explicit_width {
            options.width = 8;
        }
        Ok(options)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Options, String> {
        Options::parse(args.iter().map(ToString::to_string))
    }

    #[test]
    fn deterministic_selection_and_narrow_override() {
        let options =
            parse(&["--item", "bar", "--scenario", "narrow", "--theme", "light"]).unwrap();
        assert_eq!(
            (options.item, options.scenario, options.theme, options.width),
            (Kind::Bar, Scenario::Narrow, Theme::Light, 8)
        );
        assert_eq!(
            parse(&["--width", "3", "--scenario", "narrow"])
                .unwrap()
                .width,
            3
        );
        assert_eq!(parse(&["--item", "settings"]).unwrap().item, Kind::Settings);
        let diff = parse(&[
            "--item",
            "diff",
            "--geometry",
            "split",
            "--expanded",
            "--row-offset",
            "2",
            "--column-offset",
            "4",
        ])
        .unwrap();
        assert_eq!(diff.item, Kind::Diff);
        assert_eq!(
            diff.diff,
            DiffPreview {
                geometry: DiffGeometry::Split,
                row_offset: 2,
                column_offset: 4,
                expanded: true
            }
        );
        for (name, geometry) in [
            ("unified", DiffGeometry::Unified),
            ("stat", DiffGeometry::Stat),
        ] {
            assert_eq!(
                parse(&["--item", "diff", "--geometry", name])
                    .unwrap()
                    .diff
                    .geometry,
                geometry
            );
        }
    }

    #[test]
    fn invalid_capture_requests_fail_before_terminal_initialization() {
        for args in [
            &["--item", "unknown"][..],
            &["--geometry", "stacked"],
            &["--row-offset", "-1"],
            &["--column-offset", "x"],
            &["--theme", "rainbow"],
            &["--scenario", "random"],
            &["--width", "0"],
            &["--width", "201"],
            &["--width", "x"],
            &["--item"],
            &["--unknown"],
        ] {
            assert!(parse(args).is_err(), "{args:?}");
        }
        assert!(parse(&["--help"]).unwrap().help);
        assert!(parse(&["--list"]).unwrap().list);
    }
}
