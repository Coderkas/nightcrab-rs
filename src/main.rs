use std::{env, fs::OpenOptions, io};

use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Style, Stylize},
    symbols::border,
    text::{Line, Span, Text},
    widgets::{Block, Paragraph, Row, Table, Widget, Wrap},
};

use serde_json::Value;

mod logic;
use logic::http::send_web_request;
use logic::weapons::Weapon;
use logic::weapons::parse_weapon_data;

struct App<'a> {
    weapon_data: Vec<Weapon<'a>>,
    exit: bool,
}

impl<'a> App<'a> {
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> std::io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                if key_event.code == KeyCode::Char('q') {
                    self.exit();
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn exit(&mut self) {
        self.exit = true;
    }
}

impl<'a> Widget for &App<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut rows: Vec<Row> = Vec::new();
        let widths = [
            Constraint::Length(20),
            Constraint::Length(20),
            Constraint::Length(20),
            Constraint::Length(20),
            Constraint::Length(20),
        ];

        for weapon in &self.weapon_data {
            rows.push(Row::new([
                weapon.name,
                weapon.kind,
                weapon.attack_affinity,
                weapon.active,
            ]));
        }
        Table::new(rows, widths)
            .header(Row::new(["Name", "Type", "Attack type", "Skill"]).style(Style::new().bold()))
            .render(area, buf);
    }
}

fn main() -> std::io::Result<()> {
    let mut terminal = ratatui::init();

    let arg = env::args()
        .nth(1)
        .expect("Start option wasnt provide. Possible values are 'run' or 'update'");

    match arg.as_str() {
        "run" => {
            match OpenOptions::new()
                .write(false)
                .read(true)
                .open("weapons.json")
            {
                Ok(f) => {
                    let json_values: Result<Value, serde_json::Error> = serde_json::from_reader(f);
                    let weapon_json = &json_values
                        .expect("Failed in parsing json to serde value struct")["data"]["game"]["documents"]
                        ["wikiDocuments"]["documents"];
                    let mut weapon_data = Vec::new();
                    for weapon in weapon_json
                        .as_array()
                        .expect("Arrary of data.staticDataEnity wasnt an array")
                    {
                        weapon_data.push(parse_weapon_data(&weapon["data"]["staticDataEntity"]));
                    }
                    let mut app = App {
                        weapon_data: weapon_data,
                        exit: false,
                    };
                    let result = app.run(&mut terminal);
                    ratatui::restore();
                    result
                }
                Err(err) => Err(err),
            }
        }
        "update" => {
            OpenOptions::new()
                .write(true)
                .create(true)
                .open("weapons.json")
                .expect("File");
            send_web_request();
            Ok(())
        }
        _ => {
            println!(
                "Unknown argument '{}' provided. Possible options are 'run', 'update'",
                arg
            );
            Ok(())
        }
    }
}
