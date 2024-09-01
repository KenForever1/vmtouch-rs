use std::ffi::CStr;
use std::os::raw::c_char;
use std::path::Path;
use std::path::PathBuf;
use glob::Pattern;

extern "C" {
    static number_of_ignores: i32;
    static ignore_list: *const *const c_char;
}

fn is_ignored(path: &str) -> i32 {
    if unsafe { number_of_ignores } == 0 {
        return 0;
    }

    let path_buf = PathBuf::from(path);
    let filename = match path_buf.file_name() {
        Some(name) => name.to_str().unwrap_or(""),
        None => "",
    };

    let mut match_found = 0;

    for i in 0..unsafe { number_of_ignores } {
        let ignore_pattern = unsafe {
            CStr::from_ptr(*ignore_list.offset(i as isize))
        };
        let ignore_pattern = ignore_pattern.to_str().unwrap_or("");

        if Pattern::new(ignore_pattern).unwrap().matches(filename) {
            match_found = 1;
            break;
        }
    }

    match_found
}

fn main() {
    // Example usage
    let path = "/some/path/to/file.txt";
    let result = is_ignored(path);
    println!("Is ignored: {}", result);
}