use std::{
    env,
    fs::OpenOptions,
    process::{Command, Stdio},
    rc::Rc,
};

use ratatui::{
    DefaultTerminal, Frame,
    crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    layout::{Constraint, Flex, Layout, Rect},
    style::{Color, Style, Stylize},
    widgets::{Block, Clear, Paragraph, Row, Table, TableState},
};

use serde_json::Value;

mod logic;
use logic::http::send_web_request;
use logic::weapons::Weapon;
use logic::weapons::parse_weapon_data;

use crate::logic::weapons::{AttributeScaling, StatusAilment};

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
    curr_filter: usize,
    test_string: String,
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
    let [table_area, info_area] =
        Layout::horizontal([Constraint::Fill(1), Constraint::Max(50)]).areas(frame.area());
    let info_block = Block::bordered().title("Details");
    let info_inner = info_block.inner(info_area);

    let weapon_table = generate_table(&app.displayed_weapons);
    frame.render_stateful_widget(weapon_table, table_area, &mut app.table_state);
    frame.render_widget(&info_block, info_area);
    frame.render_widget(
        Paragraph::new(app.test_string.clone())
            .block(Block::default())
            .centered(),
        info_inner,
    );

    if matches!(app.app_state, AppState::Searching) {
        let popup_area = prepare_popup(Constraint::Percentage(30), Constraint::Length(9), frame);
        draw_filter_widget(app, frame, popup_area);
    }
}

fn prepare_popup(width: Constraint, height: Constraint, frame: &mut Frame) -> Rect {
    let [widget_area] = Layout::horizontal([width])
        .flex(Flex::Center)
        .areas(frame.area());
    let [widget_area] = Layout::vertical([height])
        .flex(Flex::Center)
        .areas(widget_area);
    let widget_block = Block::bordered();
    let inner_area = widget_block.inner(widget_area);
    frame.render_widget(Clear, inner_area);
    frame.render_widget(widget_block, widget_area);
    inner_area
}

fn draw_filter_widget(app: &mut App, frame: &mut Frame, popup_area: Rect) {
    let [search_area, _, checkboxes_area] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(1),
        Constraint::Length(3),
    ])
    .areas(popup_area);

    const AVAILABLE_FILTERS: [&str; 5] =
        ["Strength", "Dexterity", "Intelligence", "Faith", "Arcane"];
    const BORDERED_BOX_WIDTH: u16 = 3;
    const FILTER_NAME_PADDING: u16 = 3;
    let filter_lengths =
        AVAILABLE_FILTERS.map(|f| f.len() as u16 + BORDERED_BOX_WIDTH + FILTER_NAME_PADDING);

    let [_, checkboxes_area, _] = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Length(filter_lengths.iter().sum()),
        Constraint::Fill(1),
    ])
    .areas(checkboxes_area);

    frame.render_widget(
        Paragraph::new(app.search_str.clone()).block(Block::bordered().title("Search")),
        search_area,
    );

    let checkboxes_area =
        Layout::horizontal(Constraint::from_lengths(filter_lengths)).split(checkboxes_area);

    for i in 0..5 {
        let [box_area, title_area] = Layout::horizontal([
            Constraint::Length(BORDERED_BOX_WIDTH),
            Constraint::Length(filter_lengths[i] + FILTER_NAME_PADDING),
        ])
        .areas(checkboxes_area[i]);

        let checkbox_state = if i == app.curr_filter { "X" } else { " " };

        frame.render_widget(
            Paragraph::new(checkbox_state).block(Block::bordered()),
            box_area,
        );
        frame.render_widget(
            Paragraph::new(AVAILABLE_FILTERS[i])
                .block(Block::bordered().border_style(Style::new().black())),
            title_area,
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
        KeyCode::Char('s') => {
            let out_str = scan_screen("2413,1092 582x50");
            if out_str.is_empty() {
                app.test_string = String::from("No result");
            } else {
                app.table_state.select(
                    app.displayed_weapons
                        .iter()
                        .position(|w| w.name.to_lowercase() == out_str.trim().to_lowercase()),
                );
            }
        }
        KeyCode::Char('c') => {
            let out_str = scan_screen("2569,948 186x50");
            if out_str.to_lowercase() == "equipped" {
                let equipped_w = scan_screen("2408,1103 619x50");
                let new_w = scan_screen("3252,1101 619x50");
            }
        }
        _ => (),
    }
}

fn scan_screen(cords: &str) -> String {
    //grim -g 2413,1092 582x50 - | tesseract -l "eng" - -
    let output = Command::new("grim")
        .arg("-g")
        .arg(cords)
        .arg("-")
        .stdout(Stdio::piped())
        .spawn()
        .expect("Something went wrong")
        .stdout
        .expect("More went wrong");
    let ocr = Command::new("tesseract")
        .arg("-l")
        .arg("eng")
        .arg("-")
        .arg("-")
        .stdin(Stdio::from(output))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Second more went wrong");
    let ocr_result = ocr.wait_with_output().expect("Third even more went wrong");
    String::from_utf8(ocr_result.stdout).expect("Last wrong")
}

fn handle_search(app: &mut App, key_code: KeyCode, key_modifier: KeyModifiers) {
    match key_code {
        KeyCode::Esc => {
            app.app_state = AppState::Navigating;
            app.displayed_weapons = app.weapon_data.clone();
            app.curr_filter = 0;
        }
        KeyCode::Char(c) if KeyModifiers::CONTROL == key_modifier => activate_filter(c, app),
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

fn activate_filter(key: char, app: &mut App) {
    let attribute_index: usize = match key {
        's' => 0,
        'd' => 1,
        'i' => 2,
        'f' => 3,
        'a' => 4,
        _ => return,
    };

    app.displayed_weapons.clear();
    if app.curr_filter == attribute_index {
        app.displayed_weapons = app.weapon_data.clone();
        app.curr_filter = 5;
        return;
    }
    app.curr_filter = attribute_index;

    app.weapon_data
        .iter()
        .filter(|w| w.scaling[attribute_index].is_some())
        .for_each(|w| app.displayed_weapons.push(w.clone()));

    app.displayed_weapons.sort_by(|p, c| {
        p.scaling[attribute_index]
            .as_ref()
            .unwrap_or(&AttributeScaling(6))
            .0
            .cmp(
                &c.scaling[attribute_index]
                    .as_ref()
                    .unwrap_or(&AttributeScaling(6))
                    .0,
            )
    });
}

fn generate_table<'a>(weapons: &'a Vec<Rc<Weapon>>) -> Table<'a> {
    const SCALE_RANKS: [&str; 7] = ["S", "A", "B", "C", "D", "E", "N/A"];
    const AILMENTS: [&str; 8] = [
        "Poison",
        "Scarlet Rot",
        "Blood loss",
        "Frostbite",
        "Sleep",
        "Madness",
        "Death blight",
        "N/A",
    ];
    const WIDTHS: [Constraint; 11] = [
        Constraint::Max(30),
        Constraint::Max(30),
        Constraint::Max(12),
        Constraint::Max(12),
        Constraint::Max(12),
        Constraint::Max(12),
        Constraint::Max(12),
        Constraint::Fill(1),
        Constraint::Fill(1),
        Constraint::Fill(1),
        Constraint::Fill(1),
    ];

    const HEADERS: [&str; 11] = [
        "Name",
        "Attack affinity",
        "Strength",
        "Dexterity",
        "Intelligence",
        "Faith",
        "Arcane",
        "Attack Power",
        "Guarded Negation",
        "Ailment Type",
        "Ailment value",
    ];

    let mut rows: Vec<Row> = Vec::with_capacity(weapons.len());

    for weapon in weapons {
        let (ailment_type, status_ailment) =
            weapon
                .status_ailment
                .as_ref()
                .map_or((AILMENTS[7], "0"), |s_kind| match s_kind {
                    StatusAilment::Poison(val) => (AILMENTS[0], val),
                    StatusAilment::ScarletRot(val) => (AILMENTS[1], val),
                    StatusAilment::BloodLoss(val) => (AILMENTS[2], val),
                    StatusAilment::Frostbite(val) => (AILMENTS[3], val),
                    StatusAilment::Sleep(val) => (AILMENTS[4], val),
                    StatusAilment::Madness(val) => (AILMENTS[5], val),
                    StatusAilment::DeathBlight(val) => (AILMENTS[6], val),
                });

        let [str_scl, dex_scl, int_scl, fai_scl, arc_scl] = &weapon.scaling;

        let weapon_row = Row::new([
            weapon.name,
            weapon.attack_affinity.unwrap_or("Unknown"),
            SCALE_RANKS[str_scl.as_ref().unwrap_or(&AttributeScaling(6)).0],
            SCALE_RANKS[dex_scl.as_ref().unwrap_or(&AttributeScaling(6)).0],
            SCALE_RANKS[int_scl.as_ref().unwrap_or(&AttributeScaling(6)).0],
            SCALE_RANKS[fai_scl.as_ref().unwrap_or(&AttributeScaling(6)).0],
            SCALE_RANKS[arc_scl.as_ref().unwrap_or(&AttributeScaling(6)).0],
            weapon.attack_power[0].0.as_str(),
            weapon.guarded_negation[0].0.as_str(),
            ailment_type,
            status_ailment,
        ]);
        rows.push(weapon_row);
    }
    Table::new(rows, WIDTHS)
        .header(Row::new(HEADERS).style(Style::new().bold()))
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
                    curr_filter: 5,
                    test_string: String::from("Placeholder"),
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
