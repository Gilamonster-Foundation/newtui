//! Small deterministic command line; selecting a fixture never contacts a service.

use crate::fixtures::{Kind, Scenario};

pub const HELP: &str = "newtui-catalog — Shawn's custom TUI widgets\n\n\
Usage: newtui-catalog [--item NAME] [--scenario NAME] [--theme NAME] [--width N]\n\n\
  --item       settings_panel, sparkline, butterfly, heat_meter, gauge, bar, core_grid\n\
  --scenario   normal, narrow, empty, error, long\n\
  --theme      dark, light\n\
  --width      Preview content columns, 1..200 (narrow starts at 8)\n\
  --list       Print shipped item IDs without opening a terminal\n\
  --help       Show this help\n\n\
Browse: arrows select/resize, / search, Enter interact, f fixture, t theme, r reset.\n\
Interact: F1 returns to catalog; Esc goes to the component. F2 fixture, F3 theme, F4 reset.\n\
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
            if !matches!(
                flag.as_str(),
                "--item" | "--scenario" | "--theme" | "--width"
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
    }

    #[test]
    fn invalid_capture_requests_fail_before_terminal_initialization() {
        for args in [
            &["--item", "diff"][..],
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
