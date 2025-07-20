use std::{
    env,
    fs::OpenOptions,
    io::{self},
};

use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Span, Text},
    widgets::{Block, Clear, List, ListDirection, ListState, Paragraph, Row, Table, Widget},
};

use serde_json::Value;

mod logic;
use logic::http::send_web_request;
use logic::weapons::Attribute;
use logic::weapons::Weapon;
use logic::weapons::parse_weapon_data;

enum AppState {
    Navigating,
    Searching,
    Exiting,
}

struct App<'a> {
    weapon_data: &'a Vec<Weapon<'a>>,
    selected: &'a Weapon<'a>,
    selected_index: usize,
    names_state: ListState,
    state: AppState,
    search_str: String,
}

impl<'a> App<'a> {
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> std::io::Result<()> {
        while let AppState::Navigating | AppState::Searching = self.state {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Max(50), Constraint::Fill(1)])
            .split(frame.area());
        let outer = Block::bordered().title("Details");
        let outer_area = outer.inner(layout[1]);

        let search_area = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(35),
                Constraint::Percentage(30),
                Constraint::Percentage(35),
            ])
            .split(frame.area());
        let search_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Fill(1),
                Constraint::Length(3),
                Constraint::Fill(1),
            ])
            .split(search_area[1]);

        let detail_view = DetailView {
            details: self.selected,
        };

        let mut names: Vec<&str> = Vec::new();
        for weapon in self.weapon_data {
            names.push(weapon.name);
        }
        let names_list = List::new(names)
            .block(Block::bordered().title("List"))
            .highlight_style(Style::new().italic())
            .direction(ListDirection::TopToBottom);

        //if let AppState::Navigating = self.state {
        frame.render_stateful_widget(names_list, layout[0], &mut self.names_state);
        frame.render_widget(&outer, layout[1]);
        frame.render_widget(&detail_view, outer_area);
        //}

        if let AppState::Searching = self.state {
            frame.render_widget(Clear, search_area[1]);
            frame.render_widget(
                Paragraph::new(self.search_str.clone()).block(Block::bordered().title("Search")),
                search_area[1],
            );
        }
    }

    fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => match self.state {
                AppState::Navigating => self.handle_navigation(key_event.code),
                AppState::Searching => self.handle_search(key_event.code),
                _ => (),
            },
            _ => {}
        }
        Ok(())
    }

    fn handle_navigation(&mut self, key_code: KeyCode) {
        match key_code {
            KeyCode::Char('q') => self.state = AppState::Exiting,
            KeyCode::Char('j') => {
                if self.selected_index == self.weapon_data.len() - 1 {
                    self.selected_index = 0;
                    self.names_state.select_first();
                } else {
                    self.selected_index += 1;
                    self.names_state.select_next();
                }
                self.selected = &self.weapon_data[self.selected_index];
            }
            KeyCode::Char('k') => {
                if self.selected_index == 0 {
                    self.selected_index = self.weapon_data.len() - 1;
                    self.names_state.select_last();
                } else {
                    self.selected_index -= 1;
                    self.names_state.select_previous();
                }
                self.selected = &self.weapon_data[self.selected_index];
            }
            KeyCode::Char('/') => {
                self.state = AppState::Searching;
            }
            _ => (),
        };
    }

    fn handle_search(&mut self, key_code: KeyCode) {
        match key_code {
            KeyCode::Esc => {
                self.search_str.clear();
                self.state = AppState::Navigating;
            }
            KeyCode::Char(c) => {
                self.search_str.push(c);
            }
            KeyCode::Backspace => {
                self.search_str.pop();
            }
            KeyCode::Enter => {
                self.state = AppState::Navigating;
            }
            _ => (),
        };
    }
}

struct DetailView<'a> {
    details: &'a Weapon<'a>,
}

// make more stylish, probably larger fonts
impl<'a> Widget for &DetailView<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let horizontal_center = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(20),
                Constraint::Percentage(60),
                Constraint::Percentage(20),
            ])
            .split(area);
        let vertical_center = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(20),
                Constraint::Min(10),
                Constraint::Min(10),
                Constraint::Percentage(20),
            ])
            .split(horizontal_center[1]);

        let widths = [Constraint::Percentage(50), Constraint::Percentage(50)];
        let rows = [
            Row::new([
                self.details.attack_power.physical.to_string(),
                self.details.guarded_negation.physical.to_string(),
            ]),
            Row::new([
                self.details.attack_power.magic.to_string(),
                self.details.guarded_negation.magic.to_string(),
            ]),
            Row::new([
                self.details.attack_power.fire.to_string(),
                self.details.guarded_negation.fire.to_string(),
            ]),
            Row::new([
                self.details.attack_power.lightning.to_string(),
                self.details.guarded_negation.lightning.to_string(),
            ]),
            Row::new([
                self.details.attack_power.holy.to_string(),
                self.details.guarded_negation.holy.to_string(),
            ]),
            Row::new([
                self.details.attack_power.boost.to_string(),
                self.details.guarded_negation.boost.to_string(),
            ]),
        ];

        let mut scalings: Vec<String> = Vec::new();
        for x in &self.details.scaling {
            match x {
                Attribute::Vigor(scale) => scalings.push(format!("Vigor: {}", scale)),
                Attribute::Mind(scale) => scalings.push(format!("Mind: {}", scale)),
                Attribute::Endurance(scale) => scalings.push(format!("Endurance: {}", scale)),
                Attribute::Strength(scale) => scalings.push(format!("Strength: {}", scale)),
                Attribute::Dexterity(scale) => scalings.push(format!("Dexterity: {}", scale)),
                Attribute::Intelligence(scale) => scalings.push(format!("Intelligence: {}", scale)),
                Attribute::Faith(scale) => scalings.push(format!("Faith: {}", scale)),
                Attribute::Arcane(scale) => scalings.push(format!("Arcane: {}", scale)),
                Attribute::Unknown => (),
            }
        }

        Widget::render(
            Table::new(rows, widths).header(Row::new(["Attack", "Guard"])),
            vertical_center[1],
            buf,
        );
        Widget::render(List::new(scalings), vertical_center[2], buf);
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
                        weapon_data: &weapon_data,
                        selected: &weapon_data[0],
                        selected_index: 0,
                        names_state: ListState::default().with_selected(Some(0)),
                        state: AppState::Navigating,
                        search_str: String::new(),
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
