use newtui_catalog::{
    fixtures::{TickClock, ENTRIES},
    options::{Options, HELP},
    Catalog,
};
use ratatui::crossterm::event::{self, Event, KeyEventKind};
use std::io;
use std::time::Instant;

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
    let mut clock = TickClock::new(Instant::now());
    let mut dirty = true;
    let result = (|| loop {
        dirty |= catalog.advance(clock.take_due(Instant::now()));
        if dirty {
            terminal.draw(|frame| catalog.render(frame))?;
            dirty = false;
        }
        if event::poll(clock.timeout(Instant::now()))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    if catalog.handle(key) {
                        return Ok(());
                    }
                    dirty = true;
                }
                Event::Resize(_, _) => dirty = true,
                _ => {}
            }
        }
    })();
    ratatui::restore();
    result
}
