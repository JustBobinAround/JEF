use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{
    error::Error,
    io,
    time::{Duration, Instant},
};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};

use std::sync::{Arc, Mutex};

use crate::jef::{
    opener::{open, returning_terminal_at, open_terminal},
    flags::Flag,
};

use super::opener::special_open;

type SharedList = Arc<Mutex<Vec<Arc<String>>>>;

macro_rules! write_bar {
    ($var:ident, $to_write:expr) => {
        $var = vec![
            Spans::from(vec![
                Span::raw($to_write),
            ]),
        ]; 
    };
}


struct StatefulList {
    state: ListState,
    items: SharedList,
}

impl StatefulList {
    fn with_items(items: SharedList) -> StatefulList {
        StatefulList {
            state: ListState::default(),
            items,
        }
    }

    // returns 0 if lock fails - use with caution
    fn quick_ref_items_len(&self) -> isize {
        let items = self.items.clone();
        if let Ok(items) = items.lock() {
            return items.len() as isize;            
        } else {
            return 0;
        };
    }

    fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= (self.quick_ref_items_len() - 1) as usize {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    (self.quick_ref_items_len() - 1) as usize
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

}

/// This struct holds the current state of the app. In particular, it has the `items` field which is a wrapper
/// around `ListState`. Keeping track of the items state let us render the associated widget with its state
/// and have access to features such as natural scrolling.
///
/// Check the event handling at the bottom to see how to change the state on incoming events.
/// Check the drawing logic for items on how to specify the highlighting style for selected items.
enum AppState {
    Fuzzy,
    FuzzyNorm,
    Match,
    MatchNorm,
    Message,
    Visual,
    Normal,
    Command,
    Shell,
    Exit,
}
struct App {
    flag: Arc<Mutex<Flag>>,
    items: StatefulList,
    browser_items: StatefulList,
    search_term: Arc<Mutex<String>>,
    app_state: AppState,
    cmd: String,
    last_char: Option<char>,
}

impl App {
    fn from(flag: Arc<Mutex<Flag>>, items: SharedList, browser_paths: SharedList, search_term: Arc<Mutex<String>>) -> App {
        App {
            flag,
            items: StatefulList::with_items(items),
            browser_items: StatefulList::with_items(browser_paths),
            search_term,
            app_state: AppState::Normal,
            cmd: String::new(),
            last_char: None,
        }
    }

    pub fn get_selected_item<B: Backend>(&mut self, terminal:&mut Terminal<B>) {
        let items = self.browser_items.items.clone();
        let selected = self.browser_items.state.selected().unwrap_or_default();
        if let Ok(items) = items.lock(){ 
            if let Some(item) = items.get(selected){
                let item = &*item.clone();
                self.check_and_open(terminal, item);
            }
        };
    }
    
    pub fn get_selected_browser_item<B: Backend>(&mut self, terminal: &mut Terminal<B>) {
        let items = self.items.items.clone();
        let selected = self.items.state.selected().unwrap_or_default();
        if let Ok(items) = items.lock(){ 
            if let Some(item) = items.get(selected){
                let item = &*item.clone();
                self.check_and_open(terminal, item);
            }
        };
    }

    fn check_and_open<B: Backend>(&mut self, terminal:&mut Terminal<B>, item: &String) {
        if let Ok(metadata) = std::fs::metadata(item){
            self.app_state = AppState::Normal;
            if metadata.is_dir(){
                std::env::set_current_dir(item).unwrap();
            }
            if metadata.is_file() {
                open(terminal, item.clone());
            }
        }
    }

    /// Rotate through the event list.
    /// This only exists to simulate some kind of "progress"
    fn on_tick(&mut self) {
    }
}

pub fn explorer(flag: Arc<Mutex<Flag>>, paths: SharedList, browser_paths: SharedList, search_term: Arc<Mutex<String>>) -> Result<(), Box<dyn Error>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let tick_rate = Duration::from_millis(100);
    let app = App::from(flag, paths, browser_paths, search_term);
    let res = run_app(&mut terminal, app, tick_rate);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        io::stdout(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
    tick_rate: Duration,
) -> io::Result<()> {
    let mut last_tick = Instant::now();
    app.items.state.select(Some(0));
    app.browser_items.state.select(Some(0));
    while !matches!(app.app_state,AppState::Exit) {
        terminal.draw(|f| ui(f, &mut app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match app.app_state {
                    AppState::Fuzzy  => {handle_key_fuzzy(&mut app, key)},
                    AppState::FuzzyNorm => {handle_key_normal(terminal, &mut app, key)},
                    AppState::Match  => {handle_key_match(&mut app, key)},
                    AppState::MatchNorm => {handle_key_normal(terminal, &mut app, key)},
                    AppState::Normal => {handle_key_normal(terminal, &mut app, key)},
                    AppState::Visual => {},
                    AppState::Command => {handle_key_cmd(terminal, &mut app, key)},
                    AppState::Shell => {handle_key_cmd(terminal, &mut app, key)},
                    AppState::Message => {handle_key_normal(terminal, &mut app, key)}
                    AppState::Exit => {break},
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            app.on_tick();
            last_tick = Instant::now();
        }
    }
    return Ok(());
}

fn handle_key_match(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            let search_term = app.search_term.clone();
            if let Ok(mut search_term) = search_term.lock() {
                search_term.clear();
            };
            app.app_state = AppState::Normal;
        },
        //KeyCode::Left => app.items.unselect(),
        KeyCode::Char(c) => {
            let search_term = app.search_term.clone();
            if let Ok(mut search_term) = search_term.lock() {
                search_term.push(c);
            };
        },
        KeyCode::Backspace => {
            let search_term = app.search_term.clone();
            if let Ok(mut search_term) = search_term.lock() {
                search_term.pop();
            };
        },
        KeyCode::Enter => {
            reset_selection(app);
            app.app_state = AppState::MatchNorm;
        },

        KeyCode::Up => app.items.next(),
        KeyCode::Down => app.items.previous(),
        _ => {}
    }

}

fn handle_key_fuzzy(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            let search_term = app.search_term.clone();
            if let Ok(mut search_term) = search_term.lock() {
                search_term.clear();
            };
            app.app_state = AppState::Normal;
        },
        //KeyCode::Left => app.items.unselect(),
        KeyCode::Char(c) => {
            let search_term = app.search_term.clone();
            if let Ok(mut search_term) = search_term.lock() {
                search_term.push(c);
            };
        },
        KeyCode::Backspace => {
            let search_term = app.search_term.clone();
            if let Ok(mut search_term) = search_term.lock() {
                search_term.pop();
            };
        },
        KeyCode::Enter => {
            reset_selection(app);
            app.app_state = AppState::FuzzyNorm;
        },

        KeyCode::Up => app.items.next(),
        KeyCode::Down => app.items.previous(),
        _ => {}
    }

}
fn reset_selection(app: &mut App) {
    app.items.state.select(Some(0));
    app.browser_items.state.select(Some(0));
}

fn handle_key_normal<B: Backend>(terminal: &mut Terminal<B>, app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            let search = app.search_term.clone();
            if let Ok(mut search) = search.lock(){
                search.clear();
            };
            app.app_state = AppState::Normal;
        },
        //KeyCode::Left => app.items.unselect(),
        KeyCode::Char('f') => {
            app.app_state = AppState::Fuzzy;
        },
        KeyCode::Char('j') => {
            let mut count = 1;
            match app.cmd.parse::<i32>() {
                Ok(n) => count = n,
                Err(_e) => {},
            }
            for _i in 0..count{
                app.items.next();
                app.browser_items.next();
            }
            app.cmd.clear();
        },
        KeyCode::Char('k') => {
            let mut count = 1;
            match app.cmd.parse::<i32>() {
                Ok(n) => count = n,
                Err(_e) => {},
            }
            for _i in 0..count{
                app.items.previous(); 
                app.browser_items.previous();
            }
            app.cmd.clear();
        },
        KeyCode::Char(':') => {
            app.cmd.clear();
            app.app_state = AppState::Command;
        },
        KeyCode::Char('!') => {
            app.cmd.clear();
            app.app_state = AppState::Shell;
        },
        KeyCode::Char('$') => {
            special_open(terminal);
        },
        KeyCode::Char('#') => {
            open_terminal(terminal);
        },
        KeyCode::Char('/') => {
            app.app_state = AppState::Match;
        },
        KeyCode::Char(c) => {
            if !parse_cmd_num(app, c) {
                app.cmd.clear();
            }
        },
        KeyCode::Enter => {
            match app.cmd.parse::<i32>() {
                Ok(n) =>{
                    for _i in 0..n{
                        app.items.next();
                        app.browser_items.next();
                    }
                },
                Err(_e) => {
                    check_selection(terminal, app);
                    if let Ok(mut search) = app.search_term.lock(){
                        search.clear();
                    };
                },
            }
        },
        KeyCode::Backspace=> {
            let _ = std::env::set_current_dir("..");
        },
        KeyCode::Down => app.items.next(),
        KeyCode::Up => app.items.previous(),
        _ => {}
    }
}
fn check_selection<B: Backend>(terminal: &mut Terminal<B>, app: &mut App){
    app.get_selected_item(terminal);
    app.get_selected_browser_item(terminal);
    reset_selection(app);
}

fn parse_cmd_num(app: &mut App, c: char) -> bool {
    if c.is_numeric() {
        if let Some(last_c) = app.last_char {
            if last_c.is_numeric() {
                app.cmd.push(c);
            }else{
                return false;
            }
        } else {
            app.cmd.push(c);
        }
        return true;
    } else {
        return false;
    }
}
fn handle_key_cmd<B: Backend>(terminal:&mut Terminal<B>, app: &mut App, key: KeyEvent) {
    let mut ctrl = false;
    match key.modifiers {
        KeyModifiers::CONTROL => {ctrl = true;},
        _ => {},
    }
    match key.code {
        KeyCode::Esc => {
            app.app_state = AppState::Normal;
        },
        //KeyCode::Left => app.items.unselect(),
        KeyCode::Char(c) => {
            app.cmd.push(c);
        },
        KeyCode::Backspace => {
            app.cmd.pop();
        },
        
        KeyCode::Enter => {
            match app.app_state {
                AppState::Command => {
                    handle_cmd(app);
                },
                AppState::Shell => {
                    handle_shell(terminal, app)
                },
                _ => {},
            }
        }

        KeyCode::Down => app.items.next(),
        KeyCode::Up => app.items.previous(),
        _ => {}
    }
}

fn handle_shell<B: Backend>(terminal:&mut Terminal<B>, app: &mut App) {
    returning_terminal_at(terminal, &app.cmd);
    app.app_state = AppState::Normal;
}

fn handle_cmd(app: &mut App) {
    match &*app.cmd {
        "wq" => {app.app_state = AppState::Exit},
        "q" => {app.app_state = AppState::Exit},
        "q!" => {app.app_state = AppState::Exit},
        "debug" => {},
        _ => {app.app_state = AppState::Normal},
    }
    app.cmd.clear();
}

fn normal_widget<B: Backend> (f: &mut Frame<B>, app: &mut App) {
    let height = f.size().height as u32;
    let _width = f.size().width as u32;
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Ratio(height - 1, height), 
                     Constraint::Ratio(1, height)].as_ref())
        .split(f.size());

    let mut items: Vec<ListItem> = Vec::new();
    let shared_items = app.browser_items.items.clone();
    let i = match app.browser_items.state.selected() {
        Some(i) => {
            i
        }
        None => 0,
    };
    let mut i = -1 * (i as isize);
    if let Ok(shared_items) = shared_items.lock() {        
        for item in shared_items.clone(){
            let lines = &*item.clone();
            let mut start: String;
            if i == 0 {
                start = format!("{}  ",app.browser_items.state.selected().unwrap()).to_string();
            } else if i.abs() < 10 {
                start = format!(" {}  ",i.abs()).to_string();
            } else {
                start = format!(" {} ",i.abs()).to_string();
            }
            start.push_str(&lines);
            let lines = vec![Spans::from(start)];
            items.push(ListItem::new(lines).style(Style::default().fg(Color::White).bg(Color::Black)));
            i += 1;
        }
    };


    let current_dir = std::env::current_dir().unwrap_or_default();
    let title = format!("| {:?} |", current_dir);

    let items = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(
            Style::default()
                .bg(Color::LightGreen)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">>");
    
    let mut text = Vec::new();
    match app.app_state {
        AppState::Normal => {
            write_bar!(text, format!("NORMAL"));
        },
        AppState::Match => {
            if let Ok(search_term) = app.search_term.lock(){
                write_bar!(text, format!("/{}", &search_term));
            };
        },
        AppState::MatchNorm => {
            if let Ok(search_term) = app.search_term.lock(){
                write_bar!(text, format!("/{}", &search_term));
            };
        },
        AppState::Command => {
            write_bar!(text, format!(":{}",app.cmd));
        },
        AppState::Shell => {
            write_bar!(text, format!("!{}",app.cmd));
        },
        _ => {},
    }
    let label = Paragraph::new(text)
        .block(Block::default().borders(Borders::NONE))
        .style(Style::default().fg(Color::White).bg(Color::Black));
    // We can now render the item list

    f.render_stateful_widget(items, chunks[0], &mut app.browser_items.state);
    f.render_widget(label, chunks[1]);
}


fn status_bar<B: Backend>(terminal: Terminal<B>, app: &mut App) -> Vec<Spans<'static>>{
    let mut text = Vec::new();
    match app.app_state {
        AppState::Normal => {
            write_bar!(text, format!(""));
        },
        AppState::Match => {
            if let Ok(search_term) = app.search_term.lock(){
                write_bar!(text, format!("/{}", &search_term));
            };
        },
        AppState::MatchNorm => {
            if let Ok(search_term) = app.search_term.lock(){
                write_bar!(text, format!("/{}", &search_term));
            };
        },
        AppState::Command => {
            write_bar!(text, format!(":{}",app.cmd));
        },
        AppState::Shell => {
            write_bar!(text, format!("!{}",app.cmd));
        },
        _ => {},
    }
    return text;
}
 
fn fuzzy_widget<B: Backend> (f: &mut Frame<B>, app: &mut App) {
    let height = f.size().height as u32;
    let _width = f.size().width as u32;
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Ratio(height - 1, height), 
                     Constraint::Ratio(1, height)].as_ref())
        .split(f.size());

    let mut items: Vec<ListItem> = Vec::new();
    let shared_items = app.items.items.clone();
    let i = match app.items.state.selected() {
        Some(i) => {
            i
        }
        None => 0,
    };
    let mut i = -1 * (i as isize);
    if let Ok(shared_items) = shared_items.lock() {        
        for item in shared_items.clone(){
            let lines = &*item.clone();
            let mut start: String;
            if i.abs() < 10 {
                start = format!("{}  ",i.abs()).to_string();
            } else {
                start = format!("{} ",i.abs()).to_string();
            }
            start.push_str(&lines);
            let lines = vec![Spans::from(start)];
            items.push(ListItem::new(lines).style(Style::default().fg(Color::White).bg(Color::Black)));
            i += 1;
        }
    };

    // Create a List from all list items and highlight the currently selected one
    let mut text = vec![
        Spans::from(vec![
            Span::raw(format!("")),
        ]),
    ];
    if let Ok(search_term) = app.search_term.lock(){
        write_bar!(text, format!("FIND:{}", &search_term));
    };
    let current_dir = std::env::current_dir().unwrap_or_default();
    let title = format!("| {:?} |", current_dir);
    let items = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(
            Style::default()
                .bg(Color::LightGreen)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">>");
    let label = Paragraph::new(text)
        .block(Block::default().borders(Borders::NONE))
        .style(Style::default().fg(Color::White).bg(Color::Black));
    // We can now render the item list
    

    f.render_stateful_widget(items, chunks[0], &mut app.items.state);
    f.render_widget(label, chunks[1]);
}


fn ui<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    match app.app_state {
        AppState::Fuzzy   => fuzzy_widget(f, app),
        AppState::FuzzyNorm => fuzzy_widget(f, app),
        AppState::Match   => {normal_widget(f, app)},
        AppState::MatchNorm => {normal_widget(f, app)},
        AppState::Normal  => {normal_widget(f, app)},
        AppState::Message => {},
        AppState::Visual  => {},
        AppState::Command => {normal_widget(f, app)},
        AppState::Shell => {normal_widget(f, app)},
        AppState::Exit => {},
    }
}

