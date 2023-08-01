use std::collections::HashSet;


use serde::{Serialize, Deserialize};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen, size, SetSize},
};
use tui::{
    backend::{Backend, CrosstermBackend},
    Frame, Terminal,
};
use std::{
    process::Command,
    env,
    fs,
};

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    app_rule: Vec<AppRule>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppRule {
    app: String,
    tui: bool,
    file_types: HashSet<String>,
}

impl Config{
    pub fn app_from_type(&mut self, extension: String) -> Option<AppRule>{
        for app_rule in &self.app_rule{
            if app_rule.file_types.contains(&extension) {
                return Some(app_rule.clone());
            }
        } 
        return None;
    } 
    pub fn default_config() -> Config{
        let mut config: Config = toml::from_str(r#"
[[app_rule]]
    app = "vim"
    tui = true
    file_types = ["txt"]
                                "#).unwrap();

        if let Some(home) = env::var_os("HOME"){
            if let Ok(home) = home.into_string() {
                let home = format!("{}/.config/jef/jef.toml",home);
                if let Ok(config_toml) = fs::read_to_string(home){
                    config = toml::from_str(&config_toml).unwrap();
                }
            }
        }
        return config;
    }

}

pub fn open<B: Backend>(terminal: &mut Terminal<B>, path: String){
    let extension = std::path::Path::new(&path);
    let extension = extension.extension();
    let extension = extension.and_then(std::ffi::OsStr::to_str).unwrap();

    let mut config = Config::default_config();
    if let Some(app_rule) = config.app_from_type(extension.to_string()){
        if app_rule.tui {
            open_tui_app(terminal, app_rule.app, &path);
        } else {
            let _ = open::with_detached(path, app_rule.app);            
        }
    } else {
        let _ = open::that_detached(path);
    }
}

pub fn open_tui_app<B: Backend>(terminal: &mut Terminal<B>,command: String, path: &String){
    disable_raw_mode().unwrap();
    let _ = execute!(
        std::io::stdout(),
        LeaveAlternateScreen,
        DisableMouseCapture
    );
    let _ = std::process::Command::new(command)
        .arg(path)
        .status()
        .expect("Failed to open shell");
    let _ = enable_raw_mode();
    let _ = execute!(std::io::stdout(), EnterAlternateScreen, EnableMouseCapture);
    if let Ok(size) = terminal.size() {
        let _ = terminal.resize(size);
    }
}

pub fn returning_terminal_at<B: Backend>(terminal: &mut Terminal<B>, command: &String) {

    if let Ok(Some(user)) = nix::unistd::User::from_uid(nix::unistd::getuid()) {
        disable_raw_mode().unwrap();
        let _ = execute!(
            std::io::stdout(),
            LeaveAlternateScreen,
            DisableMouseCapture
        );

            //.arg(command)
        let _ = execute!(std::io::stdout(), EnterAlternateScreen, EnableMouseCapture);
        if let Ok(size) = terminal.size() {
            let _ = terminal.resize(size);
        }
        println!("!{}", command);
        let status = Command::new(user.shell)
            .arg("-c")
            .arg(command)
            .status()
            .expect("Failed to open shell");
        let mut result = String::new();
        println!("\n Press enter to continue...");
        std::io::stdin().read_line(&mut result).expect("failed to readline");
        let _ = execute!(
            std::io::stdout(),
            LeaveAlternateScreen,
            DisableMouseCapture
        );
        
        let _ = enable_raw_mode();
        let _ = execute!(std::io::stdout(), EnterAlternateScreen, EnableMouseCapture);
        if let Ok(size) = terminal.size() {
            let _ = terminal.resize(size);
        }
    }
}
