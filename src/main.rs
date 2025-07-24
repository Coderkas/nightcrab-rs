use std::{
    env,
    fs::OpenOptions,
    io::{self},
    rc::Rc,
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

use crate::logic::weapons::{Attribute, ElementTypes, StatusAilment};

enum AppState {
    Navigating,
    Searching,
    Exiting,
}

struct App<'a> {
    weapon_data: Vec<Rc<Weapon<'a>>>,
    displayed_weapons: Vec<Rc<Weapon<'a>>>,
    table_state: TableState,
    app_state: AppState,
    search_str: String,
}

fn run(mut app: App, terminal: &mut DefaultTerminal) -> std::io::Result<()> {
    while let AppState::Navigating | AppState::Searching = app.app_state {
        terminal.draw(|frame| draw(&mut app, frame))?;
        app = handle_events(app)?;
    }
    Ok(())
}

fn draw(app: &mut App, frame: &mut Frame) {
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

    let weapon_table = generate_table(&app.displayed_weapons);
    frame.render_stateful_widget(weapon_table, layout[0], &mut app.table_state);
    frame.render_widget(&outer, layout[1]);
    frame.render_widget(
        Paragraph::new("Placeholder")
            .block(Block::default())
            .centered(),
        outer_area,
    );

    if let AppState::Searching = app.app_state {
        frame.render_widget(Clear, search_area[1]);
        frame.render_widget(
            Paragraph::new(app.search_str.clone()).block(Block::bordered().title("Search")),
            search_area[1],
        );
    }
}
fn handle_events(mut app: App) -> io::Result<App> {
    match event::read()? {
        Event::Key(key_event) if key_event.kind == KeyEventKind::Press => match app.app_state {
            AppState::Navigating => app = handle_navigation(app, key_event.code),
            AppState::Searching => app = handle_search(app, key_event.code, key_event.modifiers),
            _ => (),
        },
        _ => {}
    }
    Ok(app)
}

fn handle_navigation(mut app: App, key_code: KeyCode) -> App {
    match key_code {
        KeyCode::Char('q') => app.app_state = AppState::Exiting,
        KeyCode::Char('j') => {
            if app.table_state.offset() == app.weapon_data.len() - 1 {
                app.table_state.select_first();
            } else {
                app.table_state.select_next();
            }
        }
        KeyCode::Char('k') => {
            if app.table_state.offset() == 0 {
                app.table_state.select_last();
            } else {
                app.table_state.select_previous();
            }
        }
        KeyCode::Char('/') => {
            app.app_state = AppState::Searching;
        }
        _ => (),
    };
    app
}

fn handle_search(mut app: App, key_code: KeyCode, key_modifier: KeyModifiers) -> App {
    match key_code {
        KeyCode::Esc => {
            app.app_state = AppState::Navigating;
            app.displayed_weapons = app.weapon_data.clone();
        }
        KeyCode::Char(c) => {
            if let KeyModifiers::CONTROL = key_modifier {
                if c == 'f' {
                    app.displayed_weapons.clear();
                    app.weapon_data
                        .iter()
                        .filter(|w| w.scaling[6].is_some())
                        .for_each(|w| app.displayed_weapons.push(w.clone()));
                    app.app_state = AppState::Navigating;
                }
            } else {
                app.search_str.push(c);
            }
        }
        KeyCode::Backspace => {
            app.search_str.pop();
        }
        KeyCode::Enter => {
            app.app_state = AppState::Navigating;
        }
        _ => (),
    };

    if let AppState::Searching = app.app_state {
        app.table_state.select(
            app.displayed_weapons
                .iter()
                .position(|w| w.name.contains(&app.search_str)),
        );
    } else {
        app.search_str.clear();
    }
    app
}

fn generate_table<'a>(weapons: &Vec<Rc<Weapon>>) -> Table<'a> {
    let scale_ranks = ["S", "A", "B", "C", "D", "E", "N/A"];
    let ailments = [
        "Poison",
        "Scarlet Rot",
        "Blood loss",
        "Frostbite",
        "Sleep",
        "Madness",
        "Death blight",
        "N/A",
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

    let atk_grd_closure = |i: usize, w: &[ElementTypes; 6]| -> u8 {
        match &w[i] {
            ElementTypes::Physical(v)
            | ElementTypes::Magic(v)
            | ElementTypes::Fire(v)
            | ElementTypes::Lightning(v)
            | ElementTypes::Holy(v)
            | ElementTypes::Boost(v) => *v,
        }
    };

    for weapon in weapons {
        let (ailment_type, status_ailment) = match &weapon.status_ailment {
            Some(s_kind) => match s_kind {
                StatusAilment::Poison(val) => (ailments[0], val),
                StatusAilment::ScarletRot(val) => (ailments[1], val),
                StatusAilment::BloodLoss(val) => (ailments[2], val),
                StatusAilment::Frostbite(val) => (ailments[3], val),
                StatusAilment::Sleep(val) => (ailments[4], val),
                StatusAilment::Madness(val) => (ailments[5], val),
                StatusAilment::DeathBlight(val) => (ailments[6], val),
            },
            None => (ailments[7], &0),
        };

        let weapon_row = Row::new([
            weapon.name.to_owned(),
            weapon.attack_affinity.unwrap_or("Unknown").to_owned(),
            scale_ranks[scaling_closure(0, weapon)].to_owned(),
            scale_ranks[scaling_closure(1, weapon)].to_owned(),
            scale_ranks[scaling_closure(2, weapon)].to_owned(),
            scale_ranks[scaling_closure(3, weapon)].to_owned(),
            scale_ranks[scaling_closure(4, weapon)].to_owned(),
            scale_ranks[scaling_closure(5, weapon)].to_owned(),
            scale_ranks[scaling_closure(6, weapon)].to_owned(),
            scale_ranks[scaling_closure(7, weapon)].to_owned(),
            atk_grd_closure(0, &weapon.attack_power).to_string(),
            atk_grd_closure(0, &weapon.guarded_negation).to_string(),
            ailment_type.to_owned(),
            status_ailment.to_string(),
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
                        weapon_data.push(Rc::new(parse_weapon_data(
                            &weapon["data"]["staticDataEntity"],
                        )));
                    }
                    let app = App {
                        weapon_data: weapon_data.clone(),
                        displayed_weapons: weapon_data,
                        table_state: TableState::default().with_selected(Some(0)),
                        app_state: AppState::Navigating,
                        search_str: String::new(),
                    };
                    let result = run(app, &mut terminal);
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
