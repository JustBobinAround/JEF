#[cfg(test)]
mod tests;

mod jef;

use crate::jef::{
    indexer::{
        init_indexer,
        init_browser,
        init_index_search,
    },
    term_emu::explorer,
    flags::Flag,
};
use std::sync::{Arc, Mutex};


fn main() {
    let root = ".";
    let flag = Arc::new(Mutex::new(Flag::Nothing));
    let search_term = Arc::new(Mutex::new(String::new()));

    let (index_thread, shared_file_map) = init_indexer(flag.clone(), root);
    let (browser_thread, browser_paths) = init_browser(flag.clone(), search_term.clone());
    let (search_thread, search) = init_index_search(flag.clone(), shared_file_map.clone(), search_term.clone());
    
    let _ = explorer(flag.clone(), search, browser_paths, search_term);
     
    if let Ok(mut flag) = flag.lock(){
        *flag = Flag::Halt;
    };
    index_thread.join().unwrap();
    browser_thread.join().unwrap();
    search_thread.join().unwrap();
}

