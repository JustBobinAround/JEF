#[cfg(test)]
mod tests;

mod jef;

use crate::jef::indexer::{
    init_indexer,
    init_browser,
    init_index_search,
};
use crate::jef::term_emu::{
    explorer as explorer,
};
use std::sync::{Arc, Mutex};

fn main() {
    let root = ".";
    let halt = Arc::new(Mutex::new(false));
    let search_term = Arc::new(Mutex::new(String::new()));

    let (index_thread, shared_file_map) = init_indexer(halt.clone(), root);
    let (browser_thread, browser_paths) = init_browser(halt.clone(), search_term.clone());
    let (search_thread, search) = init_index_search(halt.clone(), shared_file_map.clone(), search_term.clone());
    
    let _ = explorer(search, browser_paths, search_term);
    let halt = halt.clone(); 
    if let Ok(mut halt) = halt.lock(){
        *halt = true;
    };
    index_thread.join().unwrap();
    browser_thread.join().unwrap();
    search_thread.join().unwrap();
}

