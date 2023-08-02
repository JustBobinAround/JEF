/*
 * This file indexes a directory and allows the searching of the directory by any number of
 * characters. The indexing hash was self designed and allows seaching by any number of letters in
 * the file name so long as it is the correct order from the start of the name. This is probably
 * the most optimized piece of code I have ever written. Special thanks goes to the maintainer of
 * the jwalk lib, as the indexing speeds could not be possible without it. This section of the
 * program hevily uses rayon and simd processes to optimize where ever possible. As of the last
 * test, I indexed my entire root directory of 48gb in 3.4s and searched for the file_name
 * "main.rs" in 1.011s. Good luck understanding how it works...
 */

use std::{
    time::Duration,
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
    thread,
};
use rayon::iter::{
    IntoParallelRefIterator,
    ParallelIterator
};
use packed_simd::u8x16;
use jwalk::{
    WalkDir,
    DirEntry
};
use crate::jef::flags::Flag;



type SharedList = Arc<Mutex<Vec<Arc<String>>>>;
type SearchTerm = Arc<Mutex<String>>;

macro_rules! hash_it {
    (|$s:expr, $num_c:ident, $rolling_hash:ident | $custom_code:block ) => {
        let mut i: u32 = 255;
        $rolling_hash = 2;
        $num_c = 0;
        let mut last_c: u16 = 0;
        let s = $s.to_lowercase();

        for c in s.chars() {
            $num_c = (c as u16) << 8;
            $num_c = $num_c | last_c;
            $rolling_hash += $num_c as u32;
            $rolling_hash %= i;
            $custom_code
            i += 1;
            last_c = $num_c;
        }
    };
}
macro_rules! lock_as_mut {
    (|$var:ident | $custom_code: block) => {
        let $var = $var.clone();
        if let Ok(mut $var) = $var.lock(){
            $custom_code
        };
    };
}

macro_rules! lock_readonly {
    (|$var:ident | $custom_code: block) => {
        if let Ok($var) = $var.lock(){
            $custom_code
        };
    };
}

macro_rules! check_env {
    (|$prev_dir:ident| $custom_code: block) => {
        if let Ok(current_dir) = std::env::current_dir() {
            if current_dir != $prev_dir {
                $custom_code
                $prev_dir = current_dir;
            }
        }
    };
}
macro_rules! check_env_search {
    (|$prev_dir:ident, $prev_search_str:ident, $current_search:ident| $custom_code: block) => {
        if let Ok(current_dir) = std::env::current_dir() {
            if current_dir != $prev_dir || $current_search != $prev_search_str{
                $custom_code
                $prev_dir = current_dir;
                $prev_search_str = $current_search;
            }
        }
        
    };
}
pub fn init_browser(flag: Arc<Mutex<Flag>>, search: SearchTerm) -> (thread::JoinHandle<()>, SharedList) {
    let shared_paths: SharedList = Arc::new(Mutex::new(Vec::new()));
    let thread_paths = shared_paths.clone();

    let browse_thread = thread::spawn(move || {
        run_browser_thread(flag, thread_paths, search);
    });

    return (browse_thread, shared_paths);
}


fn run_browser_thread(flag: Arc<Mutex<Flag>>,
                      thread_paths: SharedList,
                      search: SearchTerm){
    let mut prev_dir = std::env::current_dir().unwrap_or_default();
    let mut prev_search_str = String::new();
    for entry in WalkDir::new(".").min_depth(1).max_depth(1).sort(true) {
        lock_readonly!(|flag|{
            if matches!(*flag, Flag::Halt) {
                break;
            }                    
        });
        if let Some((path, file_name, depth)) = get_file_and_path(entry) {
            lock_as_mut!(|thread_paths|{
                thread_paths.push(path);
            });
        }
    }
    loop {
        lock_readonly!(|flag|{
            if matches!(*flag, Flag::Halt) {
                break;
            }                    
        });
        let mut current_search = String::new();
        lock_readonly!(|search|{
            current_search = search.clone().to_lowercase();
        });
        check_env_search!(|prev_dir, prev_search_str, current_search|{
            lock_as_mut!(|thread_paths|{
                thread_paths.clear();
            });
            for entry in WalkDir::new(".").min_depth(1).max_depth(1).sort(true) {
                lock_readonly!(|flag|{
                    if matches!(*flag, Flag::Halt) {
                        break;
                    }                    
                });
                if current_search == ""{
                    if let Some((path, file_name, depth)) = get_file_and_path(entry) {
                        lock_as_mut!(|thread_paths|{
                            thread_paths.push(path);
                        });
                    }
                } else {
                    if let Some((path, file_name, depth)) = get_file_and_path(entry) {
                        let file_name = file_name.to_lowercase();
                        if file_name.starts_with(&current_search) {
                            lock_as_mut!(|thread_paths|{
                                thread_paths.push(path);
                            });
                        }
                    }
                }
            }
        });
        thread::sleep(Duration::from_millis(100));
    }
}

fn get_file_and_path<C: jwalk::ClientState>(entry: Result<DirEntry<C>, jwalk::Error>) -> Option<(Arc<String>, Arc<String>, u16)> {

    if entry.is_err() {
        return None;
    }

    let entry = entry.unwrap();
    let path = entry.path();
    let depth = entry.depth as u16;
    

    if let Some(file_name) = path.file_name() {
        let path_str: Arc<String>;
        let file_name_str: Arc<String>;
        if let Some(path) = path.to_str() {
            path_str = Arc::new(path.to_string());
        }else{
            return None;
        }

        if let Some(file_name) = file_name.to_str() {
            file_name_str = Arc::new(file_name.to_string());
        }else{
            return None;
        }

        return Some((path_str, file_name_str, depth));
    } else {
        return None;
    }

}


#[derive(Debug)]
pub struct FileMap {
    map: HashMap<u64, SharedList>,
    stack: u16,
    done_indexing: bool,
}
type SharedFileMap = Arc<Mutex<FileMap>>;

impl FileMap {
    fn new() -> FileMap {
        FileMap {
            map: HashMap::new(),
            stack: 0,
            done_indexing: false,
        }
    }
}

pub fn init_indexer(flag: Arc<Mutex<Flag>>, root: &str) -> (thread::JoinHandle<()>, SharedFileMap) {
    let shared_file_map: SharedFileMap = Arc::new(Mutex::new(FileMap::new()));

    let root = root.to_string().clone();
    let thread_map = shared_file_map.clone();
    let indexer_thread = thread::spawn(move || {
        run_index_thread(flag, thread_map, &root)
    });

    return (indexer_thread, shared_file_map);
}

fn run_index_thread(flag: Arc<Mutex<Flag>>,
                    thread_map: Arc<Mutex<FileMap>>,
                    root: &str){
    let mut prev_dir = PathBuf::default();
    loop{
        lock_readonly!(|flag|{
            if matches!(*flag, Flag::Halt){
                break;
            }
        });
        check_env!(|prev_dir|{
            lock_as_mut!(|thread_map|{
                thread_map.map.clear();
                thread_map.map.shrink_to(0);
            });
            index_directories(flag.clone(), &root, thread_map.clone());
        });
        thread::sleep(Duration::from_millis(200));
    }
}



pub fn init_index_search(flag: Arc<Mutex<Flag>>, 
                         shared_file_map: SharedFileMap, 
                         search: Arc<Mutex<String>>) -> (thread::JoinHandle<()>, SharedList) {
    let shared_paths: SharedList = Arc::new(Mutex::new(Vec::new()));
    let shared_file_map = shared_file_map.clone();

    let thread_map = shared_file_map.clone();
    let thread_paths = shared_paths.clone();

    let search_thread = thread::spawn(move ||{
        run_search_thread(flag, search, shared_file_map, thread_paths, thread_map);
    });
    return (search_thread, shared_paths);
}

fn run_search_thread(flag: Arc<Mutex<Flag>>,
                     search: Arc<Mutex<String>>,
                     shared_file_map: SharedFileMap,
                     thread_paths: Arc<Mutex<Vec<Arc<String>>>>,
                     thread_map: Arc<Mutex<FileMap>>){
    let mut last_search = String::new();
    let mut last_size:usize = 0;
    loop{
        lock_readonly!(|flag|{
            if matches!(*flag, Flag::Halt){
                break;
            }
        });
        lock_readonly!(|search|{
            let mut stack: u16 = 0;
            let mut size: usize = 0;
            lock_readonly!(|shared_file_map|{
                stack = shared_file_map.stack;
                size = shared_file_map.map.len();
            });
            if *search != last_search || size != last_size{
                lock_as_mut!(|thread_paths|{
                    thread_paths.clear();
                });
                last_search = search.clone();
                last_size = size;
                let hashes = get_possible_hashes(stack, &search);
                for hash in hashes {
                    check_index(thread_map.clone(), thread_paths.clone(), &hash, &search);
                }
            }

        });
        thread::sleep(Duration::from_millis(150));
    }

}


fn check_index(shared_file_map: SharedFileMap, shared_paths: SharedList, hash: &u64, search: &str) {
    lock_readonly!(|shared_file_map|{
        if let Some(str_ptr) = shared_file_map.map.get(&hash){
            lock_readonly!(|str_ptr|{
                str_ptr.par_iter().for_each(|dir| {
                    if starts_with_prefix_simd(last_chars_until_forward_slash(&dir),search) {
                        shared_paths.lock().unwrap().push(dir.clone());
                    }
                });
            });
        }
    });
}

fn index_directories(flag: Arc<Mutex<Flag>>, root: &str, shared_file_map: SharedFileMap) { 
    for entry in WalkDir::new(root).skip_hidden(false) {
        lock_readonly!(|flag|{
            if matches!(*flag, Flag::Halt) {
                break;
            }
        });
        if let Some((path_as_string, file_name, stack)) = get_file_and_path(entry) {
            let shared_file_map = shared_file_map.clone();
            index_single_dir(shared_file_map, path_as_string, file_name, stack);
        }
    }
    lock_readonly!(|shared_file_map|{
        shared_file_map.done_indexing;
    });
}

fn index_single_dir(file_map: SharedFileMap, path: Arc<String>, file_name: Arc<String>, stack: u16) {
    lock_as_mut!(|file_map|{
        if stack > file_map.stack {
            file_map.stack = stack;
        }
        let hashset = get_hashset(stack, &*file_name);
        for hash in hashset {
            if let Some(list) = file_map.map.get(&hash) {
                lock_as_mut!(|list|{
                    list.push(path.clone());
                });
            }else{
                let list: Vec<Arc<String>> = vec![path.clone()];
                let list = Arc::new(Mutex::new(list));
                file_map.map.insert(hash, list);
            }
        }
    });
}


//indexes 48gb in under 4s on ssd. lol

pub fn last_chars_until_forward_slash(s: &str) -> &str {
    let slash_byte = b'/';
    let slash_simd = u8x16::splat(slash_byte);

    let mut i = s.len() as isize - 16;
    while i > 16 {
        let chunk = &s.as_bytes()[i as usize..];
        let chunk_simd = u8x16::from_slice_unaligned(chunk);

        let cmp_result = chunk_simd.eq(slash_simd);
        if cmp_result.any() {
            let mask = cmp_result.bitmask();
            let pos = mask.leading_zeros() as usize;
            return std::str::from_utf8(&chunk[chunk.len()-pos..]).unwrap();
        }

        i -= 16;
    }

    // Check the remaining part using linear search
    for j in 0..s.len() {
        if s.chars().nth(s.len()-j-1) == Some('/') {
            return &s[s.len()-j..];
        }
    }

    return s;
}



// [stack][char][hash]
// [u16  ][u16 ][u32 ] = [u64]
fn get_hashset(stack:u16, s: &str) -> Vec<u64>{
    let mut hashset: Vec<u64> = Vec::new();
    let mut _num_c: u16;
    let mut _rolling_hash: u32 = 2;
    hash_it!(|s,_num_c, _rolling_hash|{
        hashset.push(stack_hash(stack, _num_c, _rolling_hash));
    });
    return hashset;
}

// [stack][char][hash]
// [u16  ][u16 ][u32 ] = [u64]
pub fn get_hash(s: &str) -> (u16, u32) {
    let mut _num_c: u16;
    let mut _rolling_hash: u32;
    hash_it!(|s,_num_c, _rolling_hash|{});
    return (_num_c, _rolling_hash);
}


fn get_possible_hashes(max_stack: u16, s: &str) -> Vec<u64> {
    let mut hashset: Vec<u64> = Vec::new();
    let (num_c, rolling_hash) = get_hash(s);
    for i in 0..max_stack {
        hashset.push(stack_hash(i, num_c, rolling_hash));
    }
    return hashset;
}

fn stack_hash(stack: u16, num_c: u16, rolling_hash: u32) -> u64 {
    let upper_u32: u32 = (stack as u32) << 16;
    let lower_u32: u32 = num_c as u32;
    let upper_u64: u64 = ((upper_u32 | lower_u32) as u64) << 32;
    let lower_u64: u64 = rolling_hash as u64;
    return upper_u64 | lower_u64;
}

pub fn starts_with_prefix_simd(s: &str, prefix: &str) -> bool {
    let s = s.to_lowercase();
    let prefix = prefix.to_lowercase();

    let input_bytes = s.as_bytes();
    let prefix_bytes = prefix.as_bytes();
    let prefix_len = prefix_bytes.len();

    if prefix_len <= 16 {
        input_bytes.starts_with(prefix_bytes)
    } else {
        // If the prefix size is greater than 16 bytes, use SIMD search
        let mut i = 16;
        while i < prefix_len-16 {
            let input_chunk = u8x16::from_slice_unaligned(&input_bytes[i..(i+16)]);
            let prefix_chunk = u8x16::from_slice_unaligned(&prefix_bytes[i..(i+16)]);
            let cmp_result = input_chunk.eq(prefix_chunk);
            if cmp_result.all() {
                i += 16;
            } else {
                return false;
            }
        }

        // Check the remaining part using linear search
        if i < input_bytes.len() {
            return input_bytes[i..].starts_with(&prefix_bytes[i..]);
        }else{
            return false;
        }
    }
}

//searched a 48gb index in 1.011s for main.rs. This is crazy
