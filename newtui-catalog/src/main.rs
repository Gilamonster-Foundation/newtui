use newtui_catalog::{
    fixtures::ENTRIES,
    options::{Options, HELP},
    Catalog,
};
use ratatui::crossterm::event::{self, Event, KeyEventKind};
use std::io;

fn main() -> io::Result<()> {
    let options = match Options::parse(std::env::args().skip(1)) {
        Ok(options) => options,
        Err(error) => {
            eprintln!("{error}\nUse --help for available options.");
            std::process::exit(2);
        }
    };
    if options.help {
        print!("{HELP}");
        return Ok(());
    }
    if options.list {
        for entry in ENTRIES {
            println!("{}", entry.id);
        }
        return Ok(());
    }
    let mut catalog = Catalog::new(options);
    let mut terminal = ratatui::init();
    let result = (|| loop {
        terminal.draw(|frame| catalog.render(frame))?;
        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press && catalog.handle(key) => {
                return Ok(())
            }
            _ => {}
        }
    })();
    ratatui::restore();
    result
}
