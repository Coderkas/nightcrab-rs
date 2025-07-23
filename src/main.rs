use std::{
    env,
    fs::OpenOptions,
    io::{self},
};

use ratatui::{
    DefaultTerminal, Frame,
    crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    layout::{Constraint, Direction, Layout},
    style::{Color, Style, Stylize},
    widgets::{Block, Clear, Paragraph, Row, Table, TableState},
};

use serde_json::Value;

mod logic;
use logic::http::send_web_request;
use logic::weapons::Weapon;
use logic::weapons::parse_weapon_data;

use crate::logic::weapons::{Attribute, StatusAilment};

enum AppState {
    Navigating,
    Searching,
    Exiting,
}

struct App<'a> {
    weapon_data: &'a Vec<Weapon>,
    displayed_weapons: Vec<&'a Weapon>,
    selected: &'a Weapon,
    selected_index: usize,
    names_state: TableState,
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
            .constraints([Constraint::Fill(1), Constraint::Max(50)])
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

        let weapon_table = generate_table(&self.weapon_data);
        frame.render_stateful_widget(weapon_table, layout[0], &mut self.names_state);
        frame.render_widget(&outer, layout[1]);
        frame.render_widget(
            Paragraph::new("Placeholder")
                .block(Block::default())
                .centered(),
            outer_area,
        );

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
                AppState::Searching => self.handle_search(key_event.code, key_event.modifiers),
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

    fn handle_search(&mut self, key_code: KeyCode, key_modifier: KeyModifiers) {
        match key_code {
            KeyCode::Esc => {
                self.state = AppState::Navigating;
            }
            KeyCode::Char(c) => {
                if let KeyModifiers::CONTROL = key_modifier {
                    if c == 'f' {
                        self.displayed_weapons = self
                            .weapon_data
                            .iter()
                            .filter(|x| x.scaling[6].is_some())
                            .collect();
                    }
                } else {
                    self.search_str.push(c);
                }
            }
            KeyCode::Backspace => {
                self.search_str.pop();
            }
            KeyCode::Enter => {
                self.state = AppState::Navigating;
            }
            _ => (),
        };

        if let AppState::Searching = self.state {
            let weapon_index = self
                .weapon_data
                .iter()
                .position(|x| x.name.contains(&self.search_str));
            self.names_state.select(weapon_index);
            match weapon_index {
                Some(i) => {
                    self.selected = &self.weapon_data[i];
                    self.selected_index = i;
                }
                None => {
                    self.selected = &self.weapon_data[0];
                    self.selected_index = 0;
                }
            };
        } else {
            self.search_str.clear();
        }
    }
}

fn generate_table(weapons: &Vec<Weapon>) -> Table {
    let scale_ranks = ["S", "A", "B", "C", "D", "E", "N/A"];
    let ailments = [
        "Poison".to_owned(),
        "Scarlet Rot".to_owned(),
        "Blood loss".to_owned(),
        "Frostbite".to_owned(),
        "Sleep".to_owned(),
        "Madness".to_owned(),
        "Death blight".to_owned(),
    ];
    let mut rows: Vec<Row> = Vec::with_capacity(weapons.len());
    let widths = [
        Constraint::Max(30),
        Constraint::Max(30),
        Constraint::Max(12),
        Constraint::Max(12),
        Constraint::Max(12),
        Constraint::Max(12),
        Constraint::Max(12),
        Constraint::Max(12),
        Constraint::Max(12),
        Constraint::Max(12),
        Constraint::Fill(1),
        Constraint::Fill(1),
        Constraint::Fill(1),
    ];

    let scaling_closure = |i: usize, w: &Weapon| -> usize {
        match &w.scaling[i] {
            Some(y) => match y {
                Attribute::Vigor(v)
                | Attribute::Mind(v)
                | Attribute::Endurance(v)
                | Attribute::Strength(v)
                | Attribute::Dexterity(v)
                | Attribute::Intelligence(v)
                | Attribute::Faith(v)
                | Attribute::Arcane(v) => *v,
            },
            None => 6,
        }
    };

    for weapon in weapons {
        let (ailment_type, status_ailment) = match &weapon.status_ailment {
            Some(s_kind) => match s_kind {
                StatusAilment::Poison(val) => (ailments[0].to_owned(), val.to_owned()),
                StatusAilment::ScarletRot(val) => (ailments[1].to_owned(), val.to_owned()),
                StatusAilment::BloodLoss(val) => (ailments[2].to_owned(), val.to_owned()),
                StatusAilment::Frostbite(val) => (ailments[3].to_owned(), val.to_owned()),
                StatusAilment::Sleep(val) => (ailments[4].to_owned(), val.to_owned()),
                StatusAilment::Madness(val) => (ailments[5].to_owned(), val.to_owned()),
                StatusAilment::DeathBlight(val) => (ailments[6].to_owned(), val.to_owned()),
            },
            None => ("N/A".to_owned(), "N/A".to_owned()),
        };

        let weapon_row = Row::new([
            weapon.name.to_owned(),
            weapon.attack_affinity.to_owned(),
            scale_ranks[scaling_closure(0, weapon)].to_owned(),
            scale_ranks[scaling_closure(1, weapon)].to_owned(),
            scale_ranks[scaling_closure(2, weapon)].to_owned(),
            scale_ranks[scaling_closure(3, weapon)].to_owned(),
            scale_ranks[scaling_closure(4, weapon)].to_owned(),
            scale_ranks[scaling_closure(5, weapon)].to_owned(),
            scale_ranks[scaling_closure(6, weapon)].to_owned(),
            scale_ranks[scaling_closure(7, weapon)].to_owned(),
            weapon.attack_power.physical.to_owned(),
            weapon.guarded_negation.physical.to_owned(),
            ailment_type,
            status_ailment,
        ]);
        rows.push(weapon_row);
    }
    Table::new(rows, widths)
        .header(
            Row::new([
                "Name",
                "Attack affinity",
                "Vigor",
                "Mind",
                "Endurance",
                "Strength",
                "Dexterity",
                "Intelligence",
                "Faith",
                "Arcane",
                "Attack Power",
                "Guarded Negation",
                "Ailment Type",
                "Ailment value",
            ])
            .style(Style::new().bold()),
        )
        .row_highlight_style(Style::new().italic().fg(Color::Black).bg(Color::White))
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
                        names_state: TableState::default().with_selected(Some(0)),
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
