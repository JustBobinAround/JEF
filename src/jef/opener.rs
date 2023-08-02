
use serde::{Serialize, Deserialize};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use tui::{
    backend::Backend,
    Terminal,
};
use std::{
    process::Command,
    env,
    fs,
    collections::HashSet,
};

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    app_rule: Vec<AppRule>,
    special_rule: SpecialRule,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppRule {
    app: String,
    tui: bool,
    file_types: HashSet<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SpecialRule{
    app: String,
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

[special_rule]
    app = "vim"
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

pub fn open_terminal<B: Backend>(terminal: &mut Terminal<B>) {

    if let Ok(Some(user)) = nix::unistd::User::from_uid(nix::unistd::getuid()) {
        disable_raw_mode().unwrap();
        let _ = execute!(
            std::io::stdout(),
            LeaveAlternateScreen,
            DisableMouseCapture
        );
        let _ = terminal.show_cursor();

            //.arg(command)
        let _ = execute!(std::io::stdout(), EnterAlternateScreen, EnableMouseCapture);
        if let Ok(size) = terminal.size() {
            let _ = terminal.resize(size);
        }
        println!("This shell was spawned from JEF! Use exit command to return.");
        let _status = Command::new(user.shell)
            .status()
            .expect("Failed to open shell");
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

pub fn special_open<B: Backend>(terminal: &mut Terminal<B>){
    let config = Config::default_config();
    if let Ok(Some(user)) = nix::unistd::User::from_uid(nix::unistd::getuid()) {
        disable_raw_mode().unwrap();
        let _ = execute!(
            std::io::stdout(),
            LeaveAlternateScreen,
            DisableMouseCapture
        );
        let _ = terminal.show_cursor();

        //.arg(command)
        let _ = execute!(std::io::stdout(), EnterAlternateScreen);
        if let Ok(size) = terminal.size() {
            let _ = terminal.resize(size);
        }
        let _status = Command::new(user.shell)
            .arg("-c")
            .arg(config.special_rule.app)
            .status()
            .expect("Failed to open special rule");
        let _ = execute!(
            std::io::stdout(),
            LeaveAlternateScreen,
        );
        let _ = enable_raw_mode();
        let _ = execute!(std::io::stdout(), EnterAlternateScreen, EnableMouseCapture);
        if let Ok(size) = terminal.size() {
            let _ = terminal.resize(size);
        }
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
        let _ = terminal.show_cursor();

            //.arg(command)
        let _ = execute!(std::io::stdout(), EnterAlternateScreen);
        if let Ok(size) = terminal.size() {
            let _ = terminal.resize(size);
        }
        println!("!{}", command);
        let _status = Command::new(user.shell)
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
        );
        
        let _ = enable_raw_mode();
        let _ = execute!(std::io::stdout(), EnterAlternateScreen, EnableMouseCapture);
        if let Ok(size) = terminal.size() {
            let _ = terminal.resize(size);
        }
    }
}


