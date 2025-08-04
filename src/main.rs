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
    widgets::{Block, Clear, Paragraph, Row, Table, TableState, Widget},
};

use serde_json::Value;

mod logic;
use logic::http::send_web_request;
use logic::weapons::Weapon;

use crate::logic::weapons::{Attribute, StatusAilment};

enum BaseState {
    Navigating,
    Searching,
    Exiting,
}

struct AppStates {
    base: BaseState,
    table: TableState,
    search: String,
    filter: usize,
}

impl AppStates {
    fn new() -> Self {
        Self {
            base: BaseState::Navigating,
            table: TableState::default().with_selected(Some(0)),
            search: String::new(),
            filter: 5,
        }
    }
}

struct App<'a> {
    displayed_data: Vec<Rc<Weapon<'a>>>,
    data: Vec<Rc<Weapon<'a>>>,
    state: AppStates,
    test_string: String,
    filter: FilterWidget<'a>,
    popup: PopupWidget<'a>,
}

impl<'a> App<'a> {
    fn new(data: Vec<Rc<Weapon<'a>>>, area: Rect) -> Self {
        let popup = PopupWidget::new(Constraint::Percentage(30), Constraint::Length(9), area);

        Self {
            displayed_data: data.clone(),
            data,
            state: AppStates::new(),
            test_string: String::from("Placeholder"),
            filter: FilterWidget::new(popup.inner_area),
            popup,
        }
    }

    fn run(&mut self, terminal: &mut DefaultTerminal) {
        while let BaseState::Navigating | BaseState::Searching = self.state.base {
            terminal
                .draw(|frame| self.draw(frame))
                .expect("Terminal rendering broke. Oh shit.");
            self.handle_events();
        }
    }

    fn draw(&mut self, frame: &mut Frame) {
        if matches!(self.state.base, BaseState::Searching) {
            frame.render_widget(Clear, self.popup.block.area);
            frame.render_widget(&self.popup.block.widget, self.popup.block.area);
            frame.render_widget(&self.filter.searchbar.widget, self.filter.searchbar.area);

            for (checkbox, label) in self.filter.checkboxes.iter().zip(self.filter.labels.iter()) {
                frame.render_widget(&checkbox.widget, checkbox.area);
                frame.render_widget(&label.widget, label.area);
            }
        }
    }

    fn handle_events(&mut self) {
        match event::read().unwrap_or_else(|err| {
            panic!("Something went very wrong while waiting for keyboard input. Error: {err}")
        }) {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                match self.state.base {
                    BaseState::Navigating => self.handle_navigation(key_event.code),
                    BaseState::Searching => self.handle_search(key_event.code, key_event.modifiers),
                    BaseState::Exiting => (),
                }
            }
            _ => (),
        }
    }

    fn handle_navigation(&mut self, key_code: KeyCode) {
        match key_code {
            KeyCode::Char('q') => self.state.base = BaseState::Exiting,
            KeyCode::Char('j') => {
                if self.state.table.selected().unwrap_or(0) == self.data.len() - 1 {
                    self.state.table.select_first();
                } else {
                    self.state.table.select_next();
                }
            }
            KeyCode::Char('k') => {
                if self.state.table.selected().unwrap_or(0) == 0 {
                    self.state.table.select_last();
                } else {
                    self.state.table.select_previous();
                }
            }
            KeyCode::Char('/') => {
                self.state.base = BaseState::Searching;
            }
            KeyCode::Char('s') => {
                let out_str = App::scan_screen("2413,1092 582x50");
                if out_str.is_empty() {
                    self.test_string = String::from("No result");
                } else {
                    self.state.table.select(
                        self.displayed_data
                            .iter()
                            .position(|w| w.name.to_lowercase() == out_str.trim().to_lowercase()),
                    );
                }
            }
            KeyCode::Char('c') => {
                let out_str = App::scan_screen("2569,948 186x50");
                if out_str.to_lowercase() == "equipped" {
                    let equipped_w = App::scan_screen("2408,1103 619x50");
                    let new_w = App::scan_screen("3252,1101 619x50");
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

    fn handle_search(&mut self, key_code: KeyCode, key_modifier: KeyModifiers) {
        match key_code {
            KeyCode::Esc => {
                self.state.base = BaseState::Navigating;
                self.displayed_data = self.data.clone();
                self.state.filter = 0;
            }
            KeyCode::Char(c) if KeyModifiers::CONTROL == key_modifier => self.activate_filter(c),
            KeyCode::Char(c) => self.state.search.push(c),
            KeyCode::Backspace => {
                self.state.search.pop();
            }
            KeyCode::Enter => self.state.base = BaseState::Navigating,
            _ => (),
        }

        if matches!(self.state.base, BaseState::Searching) {
            self.state.table.select(
                self.displayed_data
                    .iter()
                    .position(|w| w.name.contains(&self.state.search)),
            );
        } else {
            self.state.search.clear();
        }
    }

    fn activate_filter(&mut self, key: char) {
        let attribute_index: usize = match key {
            's' => 0,
            'd' => 1,
            'i' => 2,
            'f' => 3,
            'a' => 4,
            _ => return,
        };

        self.displayed_data.clear();
        if self.state.filter == attribute_index {
            self.displayed_data = self.data.clone();
            self.state.filter = 5;
            return;
        }
        self.state.filter = attribute_index;

        self.data
            .iter()
            .filter(|w| w.scaling[attribute_index].1.is_some())
            .for_each(|w| self.displayed_data.push(w.clone()));

        self.displayed_data.sort_by(|p, c| {
            p.scaling[attribute_index]
                .1
                .unwrap_or(6)
                .cmp(&c.scaling[attribute_index].1.unwrap_or(6))
        });
    }

    // maybe circular reference if outsourcing to table? fuck...
    fn generate_table(&self) -> Table {
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

        let mut rows: Vec<Row> = Vec::with_capacity(self.data.len());

        for weapon in &self.data {
            let (ailment_type, status_ailment) =
                weapon
                    .status_ailment
                    .as_ref()
                    .map_or((AILMENTS[7], "0"), |s_kind| match s_kind {
                        (StatusAilment::Poison, val) => (AILMENTS[0], val),
                        (StatusAilment::ScarletRot, val) => (AILMENTS[1], val),
                        (StatusAilment::BloodLoss, val) => (AILMENTS[2], val),
                        (StatusAilment::Frostbite, val) => (AILMENTS[3], val),
                        (StatusAilment::Sleep, val) => (AILMENTS[4], val),
                        (StatusAilment::Madness, val) => (AILMENTS[5], val),
                        (StatusAilment::DeathBlight, val) => (AILMENTS[6], val),
                        (StatusAilment::Unknown, val) => (AILMENTS[7], val),
                    });

            let [str_scl, dex_scl, int_scl, fai_scl, arc_scl] = &weapon.scaling;

            let weapon_row = Row::new([
                weapon.name,
                weapon.attack_affinity.unwrap_or("Unknown"),
                SCALE_RANKS[str_scl.1.unwrap_or(6)],
                SCALE_RANKS[dex_scl.1.unwrap_or(6)],
                SCALE_RANKS[int_scl.1.unwrap_or(6)],
                SCALE_RANKS[fai_scl.1.unwrap_or(6)],
                SCALE_RANKS[arc_scl.1.unwrap_or(6)],
                weapon.attack_power[0].as_str(),
                weapon.guarded_negation[0].as_str(),
                ailment_type,
                status_ailment,
            ]);
            rows.push(weapon_row);
        }

        Table::new(rows, WIDTHS)
            .header(Row::new(HEADERS).style(Style::new().bold()))
            .row_highlight_style(Style::new().italic().fg(Color::Black).bg(Color::White))
    }
}

struct UIPair<T: Widget + Default> {
    widget: T,
    area: Rect,
}

struct TableWidget<'a> {
    table: UIPair<Table<'a>>,
    info_content: UIPair<Paragraph<'a>>,
    info_block: UIPair<Block<'a>>,
}

impl<'a> TableWidget<'a> {
    fn new(area: Rect) -> Self {
        let [table_area, info_area] =
            Layout::horizontal([Constraint::Fill(1), Constraint::Max(50)]).areas(area);
        let info_block = Block::bordered().title("Details");

        Self {
            table: UIPair {
                widget: Table::default(),
                area: table_area,
            },
            info_content: UIPair {
                widget: Paragraph::new("Placeholder")
                    .block(Block::default())
                    .centered(),
                area: info_block.inner(info_area),
            },
            info_block: UIPair {
                widget: info_block,
                area: info_area,
            },
        }
    }
}

struct PopupWidget<'a> {
    block: UIPair<Block<'a>>,
    inner_area: Rect,
}

impl<'a> PopupWidget<'a> {
    fn new(width: Constraint, height: Constraint, area: Rect) -> Self {
        let [widget_area] = Layout::horizontal([width]).flex(Flex::Center).areas(area);
        let [widget_area] = Layout::vertical([height])
            .flex(Flex::Center)
            .areas(widget_area);
        let widget_block = Block::bordered();
        let inner_area = widget_block.inner(widget_area);
        Self {
            block: UIPair {
                widget: widget_block,
                area: widget_area,
            },
            inner_area,
        }
    }
}

impl<T: Widget + Default> Default for UIPair<T> {
    fn default() -> Self {
        Self {
            widget: T::default(),
            area: Rect::default(),
        }
    }
}

struct FilterWidget<'a> {
    searchbar: UIPair<Paragraph<'a>>,
    checkboxes: [UIPair<Paragraph<'a>>; 5],
    labels: [UIPair<Paragraph<'a>>; 5],
}

impl<'a> FilterWidget<'a> {
    fn new(popup_area: Rect) -> Self {
        const LABELS: [&str; 5] = ["Strength", "Dexterity", "Intelligence", "Faith", "Arcane"];
        const BOX_WIDTH: u16 = 3;
        const LABEL_PADDING: u16 = 3;
        let label_lengths = LABELS.map(|f| f.len() as u16 + BOX_WIDTH + LABEL_PADDING);

        let [area_searchbar, _, area_labeled_boxes] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(3),
        ])
        .areas(popup_area);

        let [_, area_labeled_boxes, _] = Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Length(label_lengths.iter().sum()),
            Constraint::Fill(1),
        ])
        .areas(area_labeled_boxes);

        let area_labeled_boxes =
            Layout::horizontal(Constraint::from_lengths(label_lengths)).split(area_labeled_boxes);

        let mut labels: [UIPair<Paragraph>; 5] =
            core::array::from_fn(|_| UIPair::<Paragraph>::default());

        let mut checkboxes: [UIPair<Paragraph>; 5] =
            core::array::from_fn(|_| UIPair::<Paragraph>::default());

        for i in 0..5 {
            let [area_checkbox, area_label] = Layout::horizontal([
                Constraint::Length(BOX_WIDTH),
                Constraint::Length(label_lengths[i] + LABEL_PADDING),
            ])
            .areas(area_labeled_boxes[i]);

            checkboxes[i].widget = Paragraph::new(" ").block(Block::bordered());
            checkboxes[i].area = area_checkbox;
            labels[i].widget = Paragraph::new(LABELS[i])
                .block(Block::bordered().border_style(Style::new().black()));
            labels[i].area = area_label;
        }

        Self {
            searchbar: UIPair {
                widget: Paragraph::new("").block(Block::bordered().title("Search")),
                area: area_searchbar,
            },
            labels,
            checkboxes,
        }
    }
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
                    weapon_data.push(Rc::new(Weapon::new(&weapon["data"]["staticDataEntity"])));
                }
                let mut app = App::new(weapon_data, terminal.get_frame().area());
                app.run(&mut terminal);
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
