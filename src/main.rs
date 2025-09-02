use std::{
    array, env,
    fs::OpenOptions,
    io::Error,
    process::{Command, Stdio},
    rc::Rc,
    time::Duration,
};

use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    layout::{Constraint, Flex, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Clear, Paragraph, Row, Table, TableState, Widget, WidgetRef},
};

use serde_json::Value;

mod logic;
use logic::http::send_web_request;
use logic::weapons::Weapon;

use crate::logic::weapons::{Attribute, StatusAilment};

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
            table: TableWidget::new(data, area, &BaseState::Navigating),
            search: SearchWidget::new(popup.inner_area),
            popup,
            displayed_data: data.to_vec(),
            data: data.to_vec(),
        }
    }

    fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<(), (&str, Error)> {
        loop {
            if let Err(err) = terminal.draw(|frame| self.draw(frame)) {
                return Err(("Drawing frame failed with err: ", err));
            }

            match self.state.base {
                BaseState::Navigating => App::read_key(|key: KeyCode| self.navigate(key))?,
                BaseState::Searching => App::read_key(|key: KeyCode| self.search(key))?,
                BaseState::Scanning => {
                    self.scan();

                    if !event::poll(Duration::from_secs(2))
                        .map_err(|err| ("Error while waiting for input: ", err))?
                    {
                        continue;
                    }

                    App::read_key(|key: KeyCode| {
                        if KeyCode::Esc != key {
                            return;
                        }
                        self.state.base = BaseState::Navigating;
                        self.table.update_scan_active(&BaseState::Navigating);
                    })?;
                }
                BaseState::Exiting => break,
            }
        }
        Ok(())
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

    fn read_key(mut next_handler: impl FnMut(KeyCode)) -> Result<(), (&'static str, Error)> {
        match event::read() {
            Ok(Event::Key(KeyEvent {
                code: c,
                kind: KeyEventKind::Press | KeyEventKind::Repeat,
                ..
            })) => next_handler(c),
            Ok(_) => (),
            Err(err) => return Err(("Error while reading input: ", err)),
        }
        Ok(())
    }

    fn navigate(&mut self, key_code: KeyCode) {
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
                self.table.update_scan_active(&BaseState::Scanning);
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
        match App::scan_screen("2408,1103 620x50", &self.data) {
            Ok(matched_weapon) => self.table.update_upper(&matched_weapon),
            Err(err_str) => self.table.update_diagnostic(err_str),
        }

        match App::scan_screen("3252,1101 620x50", &self.data) {
            Ok(matched_weapon) => self.table.update_lower(&matched_weapon),
            Err(err_str) => self.table.update_diagnostic(err_str),
        }
    }

    fn scan_screen(cords: &str, weapons: &[Rc<Weapon<'a>>]) -> Result<Rc<Weapon<'a>>, String> {
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

        let scan_str = match String::from_utf8(tesser_out.stdout) {
            Err(_) => return Err(String::from("Failed to convert tesseract output to String")),
            Ok(res) if res.is_empty() => return Err(String::from("Scanned nothing")),
            Ok(res) => res.to_lowercase(),
        };

        weapons
            .iter()
            .find(|w| w.name.to_lowercase().trim().contains(scan_str.trim()))
            .map_or_else(
                || {
                    Err(format!(
                        "Could not find matching item\n Scanned: {scan_str}"
                    ))
                },
                |matched_weapon| Ok(Rc::clone(matched_weapon)),
            )
    }

    fn search(&mut self, key_code: KeyCode) {
        match key_code {
            KeyCode::Esc | KeyCode::Enter => {
                self.state.base = BaseState::Navigating;
                self.state.search.clear();
                self.search.update(self.state.search.clone());
                return;
            }
            KeyCode::Char(c) => self.state.search.push(c.to_ascii_lowercase()),
            KeyCode::Backspace => _ = self.state.search.pop(),
            _ => return,
        }

        self.state.table.select(
            self.displayed_data
                .iter()
                .position(|w| w.name.to_lowercase().contains(&self.state.search)),
        );
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
    upper: UIPair<WeaponDetailsWidget>,
    lower: UIPair<WeaponDetailsWidget>,
    diagnostic: UIPair<Paragraph<'a>>,
    info_block: UIPair<Block<'a>>,
}

impl<'a> TableWidget<'a> {
    fn new(data: &[Rc<Weapon<'a>>], area: Rect, app_state: &BaseState) -> Self {
        let [table_area, info_area] =
            Layout::horizontal([Constraint::Fill(1), Constraint::Max(50)]).areas(area);
        let info_block = Block::bordered().title(Line::from(vec![
            Span::from("Details ").fg(match app_state {
                BaseState::Scanning => Color::Yellow,
                _ => Color::White,
            }),
            Span::from("<V>").fg(Color::Blue),
        ]));
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
                widget: WeaponDetailsWidget::default(),
                area: upper_area,
            },
            lower: UIPair {
                widget: WeaponDetailsWidget::default(),
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
        const WIDTHS: [Constraint; 10] = [
            Constraint::Max(30),
            Constraint::Max(20),
            Constraint::Max(8),
            Constraint::Max(8),
            Constraint::Max(8),
            Constraint::Max(8),
            Constraint::Max(8),
            Constraint::Max(28),
            Constraint::Max(28),
            Constraint::Max(20),
        ];

        let filter_color: [Color; 6] = array::from_fn(|i| {
            if i == filtered_column {
                Color::Yellow
            } else {
                Color::White
            }
        });

        let headers: [Line; 10] = [
            Line::from(vec![
                Span::from("Name ").fg(filter_color[5]),
                Span::from("<N>").fg(Color::Blue),
            ]),
            Line::from("Attack affinity"),
            Line::from(vec![
                Span::from("Str ").fg(filter_color[0]),
                Span::from("<S>").fg(Color::Blue),
            ]),
            Line::from(vec![
                Span::from("Dex ").fg(filter_color[1]),
                Span::from("<D>").fg(Color::Blue),
            ]),
            Line::from(vec![
                Span::from("Int ").fg(filter_color[2]),
                Span::from("<I>").fg(Color::Blue),
            ]),
            Line::from(vec![
                Span::from("Fai ").fg(filter_color[3]),
                Span::from("<F>").fg(Color::Blue),
            ]),
            Line::from(vec![
                Span::from("Arc ").fg(filter_color[4]),
                Span::from("<A>").fg(Color::Blue),
            ]),
            Line::from("Attack Power"),
            Line::from("Guarded Negation"),
            Line::from("Status Ailment"),
        ];

        let rows: Vec<Row> = data
            .iter()
            .map(|weapon| {
                let weapon = weapon.as_ref();
                let status_ailment = match weapon.status_ailment {
                    Some((StatusAilment::Poison, s)) => format!("Poison {s}"),
                    Some((StatusAilment::ScarletRot, s)) => format!("Scarlet Rot {s}"),
                    Some((StatusAilment::BloodLoss, s)) => format!("Bloodloss {s}"),
                    Some((StatusAilment::Frostbite, s)) => format!("Frostbite {s}"),
                    Some((StatusAilment::Sleep, s)) => format!("Sleep {s}"),
                    Some((StatusAilment::Madness, s)) => format!("Madness {s}"),
                    Some((StatusAilment::DeathBlight, s)) => format!("Death Blight {s}"),
                    Some((StatusAilment::Unknown, s)) => format!("Unknown {s}"),
                    None => String::from("-"),
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
                    weapon
                        .attack_power
                        .map(|num| match num {
                            0..=9 => format!("  {num}"),
                            10..=99 => format!(" {num}"),
                            100.. => format!("{num}"),
                        })
                        .join(" "),
                    weapon
                        .guarded_negation
                        .map(|num| match num {
                            0..=9 => format!("  {num}"),
                            10..=99 => format!(" {num}"),
                            100.. => format!("{num}"),
                        })
                        .join(" "),
                    status_ailment,
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

    fn update_scan_active(&mut self, app_state: &BaseState) {
        self.info_block.widget = Block::bordered().title(Line::from(vec![
            Span::from("Details ").fg(match app_state {
                BaseState::Scanning => Color::Yellow,
                _ => Color::White,
            }),
            Span::from("<V>").fg(Color::Blue),
        ]));
    }

    fn update_upper(&mut self, content: &Weapon) {
        self.upper.widget = WeaponDetailsWidget::new(content);
    }

    fn update_lower(&mut self, content: &Weapon) {
        self.lower.widget = WeaponDetailsWidget::new(content);
    }

    fn update_diagnostic(&mut self, content: String) {
        self.diagnostic.widget = Paragraph::new(content).block(Block::default()).centered();
    }
}

struct WeaponDetailsWidget {
    name: String,
    details: Vec<(String, String)>,
}

impl WeaponDetailsWidget {
    fn new(weapon: &Weapon) -> Self {
        const DAMAGE_TYPES: [&str; 6] = ["Phy", "Mag", "Fire", "Light", "Holy", "Crit"];
        const SCALE_RANKS: [char; 7] = ['S', 'A', 'B', 'C', 'D', 'E', '-'];
        let mut details_vec = vec![(
            weapon.kind.unwrap_or("Unknown").to_owned(),
            weapon
                .attack_affinity
                .map_or_else(String::new, ToOwned::to_owned),
        )];
        let (mut dmg_index, mut scl_index): (usize, usize) = (0, 0);
        let (mut dmg_tmp, mut scl_tmp): (Option<String>, Option<String>) = (None, None);

        while dmg_index < weapon.attack_power.len() || scl_index < weapon.scaling.len() {
            if dmg_index >= weapon.attack_power.len() {
                dmg_tmp = Some(String::new());
            }
            if scl_index >= weapon.scaling.len() {
                scl_tmp = Some(String::new());
            }

            if dmg_tmp.is_none() {
                dmg_tmp = match weapon.attack_power[dmg_index] {
                    0 => None,
                    v => Some(format!("{}: {}", DAMAGE_TYPES[dmg_index], v)),
                };
                dmg_index += 1;
            }

            if scl_tmp.is_none() {
                let (scl_attr, scl_val) = &weapon.scaling[scl_index];
                scl_tmp = scl_val.map(|v| {
                    format!(
                        "{}: {}",
                        match scl_attr {
                            Attribute::Strength => "Str",
                            Attribute::Dexterity => "Dex",
                            Attribute::Intelligence => "Int",
                            Attribute::Faith => "Fai",
                            Attribute::Arcane => "Arc",
                        },
                        SCALE_RANKS[v]
                    )
                });
                scl_index += 1;
            }

            if let Some(dmg_str) = dmg_tmp.take()
                && let Some(scl_str) = scl_tmp.take()
            {
                details_vec.push((dmg_str, scl_str));
            }
        }

        details_vec.push((
            match &weapon.status_ailment {
                Some((t, v)) => format!(
                    "{}: {}",
                    match t {
                        StatusAilment::Poison => "Poison",
                        StatusAilment::ScarletRot => "Scarlet Rot",
                        StatusAilment::BloodLoss => "Blood Loss",
                        StatusAilment::Frostbite => "Frostbite",
                        StatusAilment::Sleep => "Sleep",
                        StatusAilment::Madness => "Madness",
                        StatusAilment::DeathBlight => "Death Blight",
                        StatusAilment::Unknown => "",
                    },
                    v
                ),
                None => String::new(),
            },
            String::new(),
        ));

        Self {
            name: weapon.name.to_string(),
            details: details_vec,
        }
    }
}

impl Default for WeaponDetailsWidget {
    fn default() -> Self {
        Self {
            name: String::from("No scanning"),
            details: Vec::default(),
        }
    }
}

impl WidgetRef for WeaponDetailsWidget {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        let center_offset = area
            .width
            .saturating_sub(self.name.len().try_into().unwrap_or(area.width))
            / 2;

        if center_offset == 0 {
            buf.set_stringn(
                area.x,
                area.y,
                &self.name,
                area.width.into(),
                Style::default(),
            );
        } else {
            buf.set_string(area.x + center_offset, area.y, &self.name, Style::default());
        }

        let [_, left_column, _, right_column, _] = Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Fill(10),
            Constraint::Length(1),
            Constraint::Fill(10),
            Constraint::Fill(1),
        ])
        .areas(area);

        for (i, (l_details, r_details)) in self.details.iter().enumerate() {
            buf.set_stringn(
                left_column.x,
                left_column.y + i.try_into().unwrap_or(0) + 2,
                l_details,
                left_column.width.into(),
                Style::default(),
            );
            buf.set_stringn(
                right_column.x,
                right_column.y + i.try_into().unwrap_or(0) + 2,
                r_details,
                right_column.width.into(),
                Style::default(),
            );
        }
    }
}

impl Widget for WeaponDetailsWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.render_ref(area, buf);
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
                let res = app.run(&mut terminal);
                ratatui::restore();
                if let Err((err_msg, err)) = res {
                    println!("{err_msg}{err}");
                }
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
