use std::{env, fs::OpenOptions, rc::Rc};

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

fn run(app: &mut App, terminal: &mut DefaultTerminal) {
    while let AppState::Navigating | AppState::Searching = app.app_state {
        terminal
            .draw(|frame| draw(app, frame))
            .expect("Terminal rendering broke. Oh shit.");
        handle_events(app);
    }
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

    if matches!(app.app_state, AppState::Searching) {
        frame.render_widget(Clear, search_area[1]);
        frame.render_widget(
            Paragraph::new(app.search_str.clone()).block(Block::bordered().title("Search")),
            search_area[1],
        );
    }
}
fn handle_events(app: &mut App) {
    match event::read().unwrap_or_else(|err| {
        panic!("Something went very wrong while waiting for keyboard input. Error: {err}")
    }) {
        Event::Key(key_event) if key_event.kind == KeyEventKind::Press => match app.app_state {
            AppState::Navigating => handle_navigation(app, key_event.code),
            AppState::Searching => handle_search(app, key_event.code, key_event.modifiers),
            AppState::Exiting => (),
        },
        _ => (),
    }
}

fn handle_navigation(app: &mut App, key_code: KeyCode) {
    match key_code {
        KeyCode::Char('q') => app.app_state = AppState::Exiting,
        KeyCode::Char('j') => {
            if app.table_state.selected().unwrap_or(0) == app.weapon_data.len() - 1 {
                app.table_state.select_first();
            } else {
                app.table_state.select_next();
            }
        }
        KeyCode::Char('k') => {
            if app.table_state.selected().unwrap_or(0) == 0 {
                app.table_state.select_last();
            } else {
                app.table_state.select_previous();
            }
        }
        KeyCode::Char('/') => {
            app.app_state = AppState::Searching;
        }
        _ => (),
    }
}

fn handle_search(app: &mut App, key_code: KeyCode, key_modifier: KeyModifiers) {
    match key_code {
        KeyCode::Esc => {
            app.app_state = AppState::Navigating;
            app.displayed_weapons = app.weapon_data.clone();
        }
        KeyCode::Char(c) if KeyModifiers::CONTROL == key_modifier => {
            activate_filter(c, &mut app.weapon_data, &mut app.displayed_weapons)
        }
        KeyCode::Char(c) => app.search_str.push(c),
        KeyCode::Backspace => {
            app.search_str.pop();
        }
        KeyCode::Enter => app.app_state = AppState::Navigating,
        _ => (),
    }

    if matches!(app.app_state, AppState::Searching) {
        app.table_state.select(
            app.displayed_weapons
                .iter()
                .position(|w| w.name.contains(&app.search_str)),
        );
    } else {
        app.search_str.clear();
    }
}

fn activate_filter<'a>(
    key: char,
    weapons: &mut Vec<Rc<Weapon<'a>>>,
    displayed: &mut Vec<Rc<Weapon<'a>>>,
) {
    displayed.clear();
    let attribute_index: usize = match key {
        'v' => 0,
        'm' => 1,
        'e' => 2,
        's' => 3,
        'd' => 4,
        'i' => 5,
        'f' => 6,
        'a' => 7,
        _ => return,
    };

    weapons
        .iter()
        .filter(|w| w.scaling[attribute_index].is_some())
        .for_each(|w| displayed.push(w.clone()));

    let scaling_closure = |i: usize, w: &Weapon| -> usize {
        w.scaling[i].as_ref().map_or(6, |y| match y {
            Attribute::Vigor(v)
            | Attribute::Mind(v)
            | Attribute::Endurance(v)
            | Attribute::Strength(v)
            | Attribute::Dexterity(v)
            | Attribute::Intelligence(v)
            | Attribute::Faith(v)
            | Attribute::Arcane(v) => *v,
        })
    };
    displayed.sort_by(|p, c| {
        scaling_closure(attribute_index, p).cmp(&scaling_closure(attribute_index, c))
    });
}

fn generate_table<'a>(weapons: &'a Vec<Rc<Weapon>>) -> Table<'a> {
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
        w.scaling[i].as_ref().map_or(6, |y| match y {
            Attribute::Vigor(v)
            | Attribute::Mind(v)
            | Attribute::Endurance(v)
            | Attribute::Strength(v)
            | Attribute::Dexterity(v)
            | Attribute::Intelligence(v)
            | Attribute::Faith(v)
            | Attribute::Arcane(v) => *v,
        })
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
        let (ailment_type, status_ailment) =
            weapon
                .status_ailment
                .as_ref()
                .map_or((ailments[7], &0), |s_kind| match s_kind {
                    StatusAilment::Poison(val) => (ailments[0], val),
                    StatusAilment::ScarletRot(val) => (ailments[1], val),
                    StatusAilment::BloodLoss(val) => (ailments[2], val),
                    StatusAilment::Frostbite(val) => (ailments[3], val),
                    StatusAilment::Sleep(val) => (ailments[4], val),
                    StatusAilment::Madness(val) => (ailments[5], val),
                    StatusAilment::DeathBlight(val) => (ailments[6], val),
                });

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

fn main() {
    let mut terminal = ratatui::init();

    let arg = env::args()
        .nth(1)
        .expect("Start option wasnt provide. Possible values are 'run' or 'update'");

    match arg.as_str() {
        "run" => {
            if let Ok(f) = OpenOptions::new()
                .write(false)
                .read(true)
                .open("weapons.json")
            {
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
                let mut app = App {
                    weapon_data: weapon_data.clone(),
                    displayed_weapons: weapon_data,
                    table_state: TableState::default().with_selected(Some(0)),
                    app_state: AppState::Navigating,
                    search_str: String::new(),
                };
                run(&mut app, &mut terminal);
                ratatui::restore();
            }
        }
        "update" => {
            OpenOptions::new()
                .write(true)
                .truncate(true)
                .create(true)
                .open("weapons.json")
                .expect("File");
            send_web_request();
        }
        _ => {
            println!("Unknown argument '{arg}' provided. Possible options are 'run', 'update'");
        }
    }
}
