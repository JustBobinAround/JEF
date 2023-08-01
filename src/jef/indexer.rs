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

use rayon::iter::{IntoParallelRefIterator,ParallelIterator};
use packed_simd::u8x16;
use jwalk::WalkDir;
use jwalk::DirEntry;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use std::thread;

type SharedList = Arc<Mutex<Vec<Arc<String>>>>;

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
pub fn init_browser(halt: Arc<Mutex<bool>>, search: Arc<Mutex<String>>) -> (thread::JoinHandle<()>, SharedList) {
    let shared_paths: SharedList = Arc::new(Mutex::new(Vec::new()));

    let thread_paths = shared_paths.clone();
    let browse_thread = thread::spawn(move || {
        let mut prev_dir = std::env::current_dir().unwrap_or_default();
        let mut prev_search_str = String::new();
        for entry in WalkDir::new(".").min_depth(1).max_depth(1).sort(true) {
            if let Ok(halt) = halt.lock(){
                if *halt {
                    break;
                }                    
            };
            if let Some((path, file_name, depth)) = get_file_and_path(entry) {
                let shared_paths = thread_paths.clone();
                if let Ok(mut shared_paths) = shared_paths.lock() {
                    shared_paths.push(path);
                };
            }
        }
        loop {
            if let Ok(halt) = halt.lock(){
                if *halt {
                    break;
                }                    
            };
            let mut current_search = String::new();
            if let Ok(search_check) = search.lock() {
                current_search = search_check.clone().to_lowercase();
            };
            if let Ok(current_dir) = std::env::current_dir() {
                if current_dir != prev_dir || current_search != prev_search_str{
                    let shared_paths = thread_paths.clone();
                    if let Ok(mut shared_paths) = shared_paths.lock() {
                        shared_paths.clear();
                    };
                    for entry in WalkDir::new(".").min_depth(1).max_depth(1).sort(true) {
                        if let Ok(halt) = halt.lock(){
                            if *halt {
                                break;
                            }                    
                        };
                        if current_search == ""{
                            if let Some((path, file_name, depth)) = get_file_and_path(entry) {
                                let shared_paths = thread_paths.clone();
                                if let Ok(mut shared_paths) = shared_paths.lock() {
                                    shared_paths.push(path);
                                };
                            }
                        } else {
                            if let Some((path, file_name, depth)) = get_file_and_path(entry) {
                                let file_name = file_name.to_lowercase();
                                if file_name.starts_with(&current_search) {
                                    let shared_paths = thread_paths.clone();
                                    if let Ok(mut shared_paths) = shared_paths.lock() {
                                        shared_paths.push(path);
                                    };
                                }
                            }
                        }

                    }

                    prev_dir = current_dir;
                    prev_search_str = current_search;
                }
            } else {
            }
            thread::sleep(Duration::from_millis(100));
        }
    });

    return (browse_thread, shared_paths);
}

fn get_file_and_path<C: jwalk::ClientState>(entry: Result<DirEntry<C>, jwalk::Error>) -> Option<(Arc<String>, Arc<String>, u16)> {

    if entry.is_err() {
        return None;
    }

    let entry = entry.unwrap();
    let path = entry
        .path();
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

pub fn init_indexer(halt: Arc<Mutex<bool>>, root: &str) -> (thread::JoinHandle<()>, SharedFileMap) {
    let shared_file_map: SharedFileMap = Arc::new(Mutex::new(FileMap::new()));

    let root = root.to_string().clone();
    let thread_map = shared_file_map.clone();
    let indexer_thread = thread::spawn(move || {
        let mut prev_dir = PathBuf::default();
        loop{
            if let Ok(halt) = halt.lock(){
                if *halt {
                    break;
                }
            };
            if let Ok(current_dir) = std::env::current_dir() {
                if current_dir != prev_dir {
                    let thread_map = thread_map.clone();
                    if let Ok(mut thread_map) = thread_map.lock(){
                        thread_map.map.clear();
                        thread_map.map.shrink_to(0);
                    };
                    index_directories(halt.clone(), &root, thread_map.clone());
                    prev_dir = current_dir;
                }
            }
            thread::sleep(Duration::from_millis(200));
        }
    });

    return (indexer_thread, shared_file_map);
}

use std::time::Duration;


pub fn init_index_search(halt: Arc<Mutex<bool>>, 
                         shared_file_map: SharedFileMap, 
                         search: Arc<Mutex<String>>) -> (thread::JoinHandle<()>, SharedList) {
    let shared_paths: SharedList = Arc::new(Mutex::new(Vec::new()));
    let shared_file_map = shared_file_map.clone();

    let thread_map = shared_file_map.clone();
    let thread_paths = shared_paths.clone();

    let search_thread = thread::spawn(move ||{
        let mut last_search = String::new();
        let mut last_size:usize = 0;
        loop{
            if let Ok(halt) = halt.lock(){
                if *halt {
                    break;
                }                    
            };
            if let Ok(search) = search.lock(){
                let mut stack: u16 = 0;
                let mut size: usize = 0;
                if let Ok(shared_file_map) = shared_file_map.lock() {
                    stack = shared_file_map.stack;
                    size = shared_file_map.map.len();
                };
                if *search != last_search || size != last_size{
                    let thread_paths = thread_paths.clone();
                    if let Ok(mut thread_paths) = thread_paths.lock(){
                        thread_paths.clear();
                    };
                    last_search = search.clone();
                    last_size = size;
                    let hashes = get_possible_hashes(stack, &search);
                    for hash in hashes {
                        check_index(thread_map.clone(), thread_paths.clone(), &hash, &search);
                    }
                }
            };
            thread::sleep(Duration::from_millis(150));
        }
    });
    return (search_thread, shared_paths);
}

fn check_index(shared_file_map: SharedFileMap, shared_paths: SharedList, hash: &u64, search: &str) {
    let shared_file_map = shared_file_map.clone();
    if let Ok(file_map) = shared_file_map.lock(){
        if let Some(str_ptr) = file_map.map.get(&hash){
            let str_ptr = str_ptr.clone();
            if let Ok(str_ptr) = str_ptr.lock(){
                str_ptr.par_iter().for_each(|dir| {
                    if starts_with_prefix_simd(last_chars_until_forward_slash(&dir),search) {
                        shared_paths.lock().unwrap().push(dir.clone());
                    }
                });
            };
        }
    };
}

pub fn get_dirs_2(max_stack: u16, file_map: HashMap<u64, SharedList>, s: &str) -> Arc<Mutex<Vec<Arc<String>>>> {
    let dirs: SharedList = Arc::new(Mutex::new(Vec::new()));
    let hashes = get_possible_hashes(max_stack, s);
    hashes.par_iter().for_each(|hash| {
        if let Some(str_ptr) = file_map.get(&hash){
            let str_ptr = str_ptr.clone();
            if let Ok(str_ptr) = str_ptr.lock(){
                str_ptr.par_iter().for_each(|dir| {
                    if starts_with_prefix_simd(last_chars_until_forward_slash(&dir),s) {
                        dirs.lock().unwrap().push(dir.clone());
                    }
                });
            };
        }
    });
    return dirs;
}
fn index_directories(halt: Arc<Mutex<bool>>, root: &str, shared_file_map: SharedFileMap) { 
    for entry in WalkDir::new(root).skip_hidden(false) {
        if let Ok(halt) = halt.lock(){
            if *halt {
                break;
            }                    
        };
        if let Some((path_as_string, file_name, stack)) = get_file_and_path(entry) {
            let shared_file_map = shared_file_map.clone();
            index_single_dir(shared_file_map, path_as_string, file_name, stack);
        }
    }
    let shared_file_map = shared_file_map.clone();
    if let Ok(mut file_map) = shared_file_map.lock() {
        file_map.done_indexing = true;
    };
}
fn index_single_dir(file_map: SharedFileMap, path: Arc<String>, file_name: Arc<String>, stack: u16) {
        if let Ok(mut file_map) = file_map.lock(){
            if stack > file_map.stack {
                file_map.stack = stack;
            }
            let hashset = get_hashset(stack, &*file_name);
            for hash in hashset {
                if let Some(list) = file_map.map.get(&hash) {
                    let list = list.clone();
                    {
                        if let Ok(mut list) = list.lock() {
                            list.push(path.clone());
                        };
                    }
                }else{
                    let list: Vec<Arc<String>> = vec![path.clone()];
                    let list = Arc::new(Mutex::new(list));
                    file_map.map.insert(hash, list);
                }
            }
        };
}


//indexes 48gb in under 4s on ssd. lol
pub fn index_directories_2(root: &str) -> (u16, HashMap<u64, SharedList>) {
    let mut file_map: HashMap<u64, SharedList> = HashMap::new();
    let mut max_stack: u16 = 0;

    for entry in WalkDir::new(root).skip_hidden(false) {
        if let Some((path_as_string, file_name, stack)) = get_file_and_path(entry) {
            //println!("{:?}", path_as_string);
            //let stack = count_stack_simd(path_as_string);
            if stack > max_stack {
                max_stack = stack;
            }
            let hashset = get_hashset(stack, &*file_name);
            for hash in hashset {
                if let Some(list) = file_map.get(&hash) {
                    let list = list.clone();
                    {
                        if let Ok(mut list) = list.lock() {
                            list.push(path_as_string.clone());
                        };
                    }
                }else{
                    let list: Vec<Arc<String>> = vec![path_as_string.clone()];
                    let list = Arc::new(Mutex::new(list));
                    file_map.insert(hash, list);
                }
            }
        }
    }
    return (max_stack, file_map);
}

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
