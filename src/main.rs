use std::{
    env,
    fs::OpenOptions,
    process::{Command, Stdio},
    rc::Rc,
    time::Duration,
};

use ratatui::{
    DefaultTerminal, Frame,
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
    layout::{Constraint, Flex, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Clear, Paragraph, Row, Table, TableState, Widget},
};

use serde_json::Value;

mod logic;
use logic::http::send_web_request;
use logic::weapons::Weapon;

use crate::logic::weapons::StatusAilment;

enum BaseState {
    Navigating,
    Searching,
    Scanning,
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
    state: AppStates,
    table: TableWidget<'a>,
    search: SearchWidget<'a>,
    popup: PopupWidget<'a>,
    displayed_data: Vec<Rc<Weapon<'a>>>,
    data: Vec<Rc<Weapon<'a>>>,
}

impl<'a> App<'a> {
    fn new(data: &[Rc<Weapon<'a>>], area: Rect) -> Self {
        let popup = PopupWidget::new(Constraint::Percentage(30), Constraint::Length(9), area);

        Self {
            state: AppStates::new(),
            table: TableWidget::new(data, area),
            search: SearchWidget::new(popup.inner_area),
            popup,
            displayed_data: data.to_vec(),
            data: data.to_vec(),
        }
    }

    fn run(&mut self, terminal: &mut DefaultTerminal) {
        while let BaseState::Navigating | BaseState::Searching | BaseState::Scanning =
            self.state.base
        {
            terminal
                .draw(|frame| self.draw(frame))
                .expect("Terminal rendering broke. Oh shit.");
            if matches!(self.state.base, BaseState::Scanning) {
                self.scan();
                if event::poll(Duration::from_secs(2)).unwrap_or_else(|err| {
                    panic!(
                        "Something went very wrong while waiting for keyboard input. Error: {err}"
                    )
                }) {
                    if let Event::Key(val) =
                        event::read().unwrap_or_else(|err| panic!("Something went very wrong while waiting for keyboard input. Error: {err}"))
                    {
                        if val.code == KeyCode::Esc {
                            self.state.base = BaseState::Navigating;
                        }
                    }
                }
            } else {
                self.handle_events();
            }
            if matches!(self.state.base, BaseState::Searching) {
                let area = terminal.get_frame().area();
                self.table = TableWidget::new(&self.displayed_data, area);
            }
        }
    }

    fn draw(&mut self, frame: &mut Frame) {
        frame.render_stateful_widget(
            &self.table.table.widget,
            self.table.table.area,
            &mut self.state.table,
        );
        frame.render_widget(&self.table.info_block.widget, self.table.info_block.area);
        frame.render_widget(&self.table.upper.widget, self.table.upper.area);
        frame.render_widget(&self.table.lower.widget, self.table.lower.area);
        frame.render_widget(&self.table.diagnostic.widget, self.table.diagnostic.area);

        if matches!(self.state.base, BaseState::Searching) {
            frame.render_widget(Clear, self.popup.block.area);
            frame.render_widget(&self.popup.block.widget, self.popup.block.area);
            frame.render_widget(&self.search.bar.widget, self.search.bar.area);
        }
    }

    fn handle_events(&mut self) {
        match event::read().unwrap_or_else(|err| {
            panic!("Something went very wrong while waiting for keyboard input. Error: {err}")
        }) {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                match self.state.base {
                    BaseState::Navigating => self.handle_navigation(key_event.code),
                    BaseState::Searching => self.search(key_event.code),
                    _ => (),
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
            KeyCode::Char('v') => {
                self.state.base = BaseState::Scanning;
            }
            KeyCode::Char('s') => self.filter(0),
            KeyCode::Char('d') => self.filter(1),
            KeyCode::Char('i') => self.filter(2),
            KeyCode::Char('f') => self.filter(3),
            KeyCode::Char('a') => self.filter(4),
            KeyCode::Char('n') => self.filter(5),
            _ => (),
        }
    }

    fn scan(&mut self) {
        match App::scan_screen("2569,948 186x50") {
            Ok(v) => {
                match App::scan_screen("2408,1103 620x50") {
                    Ok(scan_str) => self.table.update_upper(scan_str),
                    Err(err_str) => self.table.update_diagnostic(err_str),
                }
                if v.to_lowercase().contains("equipped") {
                    match App::scan_screen("3252,1101 620x50") {
                        Ok(scan_str) => self.table.update_lower(scan_str),
                        Err(err_str) => self.table.update_diagnostic(err_str),
                    }
                }
            }
            Err(err_str) => self.table.update_diagnostic(err_str),
        }
    }

    fn scan_screen(cords: &str) -> Result<String, String> {
        let Ok(grim) = Command::new("grim")
            .arg("-g")
            .arg(cords)
            .arg("-")
            .stdout(Stdio::piped())
            .spawn()
        else {
            return Err(String::from("Failed to start grim"));
        };

        let Some(grim_out) = grim.stdout else {
            return Err(String::from("Failed to pipe data from grim"));
        };

        let Ok(tesser) = Command::new("tesseract")
            .arg("-l")
            .arg("eng")
            .arg("-")
            .arg("-")
            .stdin(Stdio::from(grim_out))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        else {
            return Err(String::from("Failed to start tessearct"));
        };

        let Ok(tesser_out) = tesser.wait_with_output() else {
            return Err(String::from("Failed to pipe data from tesseract"));
        };

        String::from_utf8(tesser_out.stdout).map_or_else(
            |_| Err(String::from("Failed to convert tesseract output to String")),
            |v| {
                if v.is_empty() {
                    Err(String::from("Scanned nothing"))
                } else {
                    Ok(v)
                }
            },
        )
    }

    fn search(&mut self, key_code: KeyCode) {
        match key_code {
            KeyCode::Esc => {
                self.state.base = BaseState::Navigating;
                self.displayed_data = self.data.clone();
                self.state.filter = 0;
            }
            KeyCode::Char(c) => self.state.search.push(c),
            KeyCode::Backspace => _ = self.state.search.pop(),
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
        self.search.update(self.state.search.clone());
    }

    fn filter(&mut self, attribute_index: usize) {
        self.displayed_data.clear();
        if self.state.filter == attribute_index || attribute_index == 5 {
            self.displayed_data = self.data.clone();
            self.state.filter = 5;
        } else {
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
        self.table.update(&self.displayed_data, self.state.filter);
    }
}

struct UIPair<T: Widget + Default> {
    widget: T,
    area: Rect,
}

struct TableWidget<'a> {
    table: UIPair<Table<'a>>,
    upper: UIPair<Paragraph<'a>>,
    lower: UIPair<Paragraph<'a>>,
    diagnostic: UIPair<Paragraph<'a>>,
    info_block: UIPair<Block<'a>>,
}

impl<'a> TableWidget<'a> {
    fn new(data: &[Rc<Weapon<'a>>], area: Rect) -> Self {
        let [table_area, info_area] =
            Layout::horizontal([Constraint::Fill(1), Constraint::Max(50)]).areas(area);
        let info_block = Block::bordered().title("Details");
        let [upper_area, lower_area, diagnostic_area] = Layout::vertical([
            Constraint::Percentage(40),
            Constraint::Percentage(40),
            Constraint::Percentage(20),
        ])
        .areas(info_block.inner(info_area));

        Self {
            table: UIPair {
                widget: TableWidget::create_table(data, 5),
                area: table_area,
            },
            upper: UIPair {
                widget: Paragraph::new("").block(Block::default()).centered(),
                area: upper_area,
            },
            lower: UIPair {
                widget: Paragraph::new("").block(Block::default()).centered(),
                area: lower_area,
            },
            diagnostic: UIPair {
                widget: Paragraph::new("").block(Block::default()).centered(),
                area: diagnostic_area,
            },
            info_block: UIPair {
                widget: info_block,
                area: info_area,
            },
        }
    }

    fn create_table(data: &[Rc<Weapon<'a>>], filtered_column: usize) -> Table<'a> {
        const SCALE_RANKS: [char; 7] = ['S', 'A', 'B', 'C', 'D', 'E', '-'];
        const AILMENTS: [&str; 9] = [
            "Poison",
            "Scarlet Rot",
            "Blood loss",
            "Frostbite",
            "Sleep",
            "Madness",
            "Death blight",
            "???",
            "-",
        ];
        const WIDTHS: [Constraint; 11] = [
            Constraint::Max(30),
            Constraint::Max(30),
            Constraint::Max(8),
            Constraint::Max(8),
            Constraint::Max(8),
            Constraint::Max(8),
            Constraint::Max(8),
            Constraint::Fill(1),
            Constraint::Fill(1),
            Constraint::Fill(1),
            Constraint::Fill(1),
        ];
        let headers: [Line; 11] = [
            Line::from(vec![
                Span::from("Name ").fg(if filtered_column == 5 {
                    Color::Yellow
                } else {
                    Color::White
                }),
                Span::from("<N>").fg(Color::Blue),
            ]),
            Line::from("Attack affinity"),
            Line::from(vec![
                Span::from("Str ").fg(if filtered_column == 0 {
                    Color::Yellow
                } else {
                    Color::White
                }),
                Span::from("<S>").fg(Color::Blue),
            ]),
            Line::from(vec![
                Span::from("Dex ").fg(if filtered_column == 1 {
                    Color::Yellow
                } else {
                    Color::White
                }),
                Span::from("<D>").fg(Color::Blue),
            ]),
            Line::from(vec![
                Span::from("Int ").fg(if filtered_column == 2 {
                    Color::Yellow
                } else {
                    Color::White
                }),
                Span::from("<I>").fg(Color::Blue),
            ]),
            Line::from(vec![
                Span::from("Fai ").fg(if filtered_column == 3 {
                    Color::Yellow
                } else {
                    Color::White
                }),
                Span::from("<F>").fg(Color::Blue),
            ]),
            Line::from(vec![
                Span::from("Arc ").fg(if filtered_column == 4 {
                    Color::Yellow
                } else {
                    Color::White
                }),
                Span::from("<A>").fg(Color::Blue),
            ]),
            Line::from("Attack Power"),
            Line::from("Guarded Negation"),
            Line::from("Ailment Type"),
            Line::from("Ailment value"),
        ];

        let rows: Vec<Row> = data
            .iter()
            .map(|weapon| {
                let weapon = weapon.as_ref();
                let (ailment_type, status_ailment) = match weapon.status_ailment {
                    Some((StatusAilment::Poison, s)) => (AILMENTS[0], s),
                    Some((StatusAilment::ScarletRot, s)) => (AILMENTS[1], s),
                    Some((StatusAilment::BloodLoss, s)) => (AILMENTS[2], s),
                    Some((StatusAilment::Frostbite, s)) => (AILMENTS[3], s),
                    Some((StatusAilment::Sleep, s)) => (AILMENTS[4], s),
                    Some((StatusAilment::Madness, s)) => (AILMENTS[5], s),
                    Some((StatusAilment::DeathBlight, s)) => (AILMENTS[6], s),
                    Some((StatusAilment::Unknown, s)) => (AILMENTS[7], s),
                    None => (AILMENTS[8], 0),
                };

                let [str_scl, dex_scl, int_scl, fai_scl, arc_scl] = &weapon.scaling;

                Row::new([
                    String::from(weapon.name),
                    String::from(weapon.attack_affinity.unwrap_or("Unknown")),
                    String::from(SCALE_RANKS[str_scl.1.unwrap_or(6)]),
                    String::from(SCALE_RANKS[dex_scl.1.unwrap_or(6)]),
                    String::from(SCALE_RANKS[int_scl.1.unwrap_or(6)]),
                    String::from(SCALE_RANKS[fai_scl.1.unwrap_or(6)]),
                    String::from(SCALE_RANKS[arc_scl.1.unwrap_or(6)]),
                    weapon.attack_power[0].to_string(),
                    weapon.guarded_negation[0].to_string(),
                    String::from(ailment_type),
                    status_ailment.to_string(),
                ])
            })
            .collect();
        Table::new(rows, WIDTHS)
            .header(Row::new(headers).style(Style::new().bold()))
            .row_highlight_style(Style::new().italic().fg(Color::Black).bg(Color::White))
    }

    fn update(&mut self, data: &[Rc<Weapon<'a>>], filtered_column: usize) {
        self.table.widget = TableWidget::create_table(data, filtered_column);
    }

    fn update_upper(&mut self, content: String) {
        self.upper.widget = Paragraph::new(content).block(Block::default()).centered();
    }

    fn update_lower(&mut self, content: String) {
        self.lower.widget = Paragraph::new(content).block(Block::default()).centered();
    }

    fn update_diagnostic(&mut self, content: String) {
        self.diagnostic.widget = Paragraph::new(content).block(Block::default()).centered();
    }
}

struct PopupWidget<'a> {
    block: UIPair<Block<'a>>,
    inner_area: Rect,
}

impl PopupWidget<'_> {
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

struct SearchWidget<'a> {
    bar: UIPair<Paragraph<'a>>,
}

impl SearchWidget<'_> {
    fn new(popup_area: Rect) -> Self {
        Self {
            bar: UIPair {
                widget: Paragraph::new("").block(Block::bordered().title("Search")),
                area: popup_area,
            },
        }
    }

    fn update(&mut self, content: String) {
        self.bar.widget = Paragraph::new(content).block(Block::bordered().title("Search"));
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
                let mut app = App::new(&weapon_data, terminal.get_frame().area());
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
