use clap::Parser;
use libc::{mincore, munmap};
use memmap2::MmapOptions;
use std::fmt;
use std::fs::OpenOptions;
use std::os::raw::c_void;
use std::path::Path;
use tabled::{Table, Tabled};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // println!("args: {:?}", args);

    let mut print_data_vec: Vec<FincoreResult> = vec![];

    let mut total_cached_size = 0;

    for path in args.pathes {
        let path = Path::new(&path);

        let mut file_tasks: Vec<String> = vec![];

        if path.is_file() {
            let path_str = path.to_str().ok_or("path to str error")?;
            file_tasks.push(path_str.to_string());
        } else if let Some(parent) = path.parent() {
            if path.file_name() == Some(std::ffi::OsStr::new("*")) {
                for entry in std::fs::read_dir(parent)? {
                    let entry = entry?;
                    let path = entry.path();

                    if path.is_file() {
                        let path_str = path.to_str().ok_or("path to str error")?;
                        file_tasks.push(path_str.to_string());
                    }
                }
            }
        }

        file_tasks.iter().for_each(|path_str| {
            let fincore_res = match fincore(path_str, args.pages) {
                Ok(res) => res,
                Err(e) => {
                    panic!("{}", e);
                }
            };
            total_cached_size += fincore_res.cached_size;

            if args.pages && !fincore_res.cached_pages.0.is_empty() {
                println!(
                    "path: {}, cached_pages: {}",
                    fincore_res.path, fincore_res.cached_pages
                );
            }

            if !args.only_cached || fincore_res.cached > 0 {
                print_data_vec.push(fincore_res);
            }
        });
    }

    let table = Table::new(print_data_vec).to_string();
    println!("{}", table);

    if args.summarize {
        println!("total cached size: {} byte", total_cached_size);
    }
    Ok(())
}

/// a mem tool named fincore
#[derive(Parser, Debug)]
#[command(name = "fincore", version, about, long_about = None)]
struct Args {
    /// Path to the file
    #[arg(index = 1, required = true)]
    pathes: Vec<String>,
    #[arg(short, long, default_value = "false")]
    /// Print page index
    pages: bool,
    #[arg(short, long, default_value = "true")]
    /// When comparing multiple files, print a summary report
    summarize: bool,
    #[arg(short, long, default_value = "false")]
    /// Only print cached pages
    only_cached: bool,
}

#[derive(Clone, Default, Debug)]
struct DisplayVec(Vec<usize>);

// Implement Display trait for DisplayVec
impl fmt::Display for DisplayVec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Custom formatting logic
        let formatted = self
            .0
            .iter()
            .map(|num| num.to_string())
            .collect::<Vec<String>>()
            .join(", ");
        write!(f, "{}", formatted)
    }
}

#[derive(Clone, Default, Debug, Tabled)]
struct FincoreResult {
    path: String,
    file_size: i64,
    total_pages: i64,
    cached: i64,
    cached_size: i64,
    cached_percent: f64,
    #[tabled(skip)]
    cached_pages: DisplayVec,
}

fn fincore(path: &str, pages: bool) -> Result<FincoreResult, Box<dyn std::error::Error>> {
    let mut fincore_result = FincoreResult::default();

    let file = OpenOptions::new().read(true).write(true).open(path)?;

    let file_size = file.metadata()?.len() as usize;

    let mp = unsafe { MmapOptions::new().len(file_size).map(&file)? };

    let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) as usize };

    let num_page = (file_size + page_size - 1) / page_size;
    let mut mincore_vec = vec![0u8; num_page];

    let ret = unsafe {
        mincore(
            mp.as_ptr() as *mut c_void,
            num_page,
            mincore_vec.as_mut_ptr(),
        )
    };

    if ret != 0 {
        Err(std::io::Error::last_os_error())?;
    }

    let mut cached_pages: Vec<usize> = vec![];
    let mut cached = 0;
    for (i, &val) in mincore_vec.iter().enumerate() {
        if val & 1 != 0 {
            cached += 1;
            if pages {
                // save page_index
                cached_pages.push(i);
            }
        }
    }

    let total_pages = ((file_size as f64) / (page_size as f64)).ceil() as i32;

    let cached_percent = (cached as f64 / total_pages as f64) * 100.0;

    let cached_size = cached * page_size;

    // munmap不是必须的操作，因为Mmap对象drop时会进行munmap
    // let ret = unsafe {
    //     munmap(mp.as_ptr() as *mut c_void, file_size)
    // };

    // if ret != 0 {
    //     Err(std::io::Error::last_os_error())?;
    // }

    fincore_result.path = path.to_string();
    fincore_result.file_size = file_size as i64;
    fincore_result.total_pages = total_pages as i64;
    fincore_result.cached = cached as i64;
    fincore_result.cached_size = cached_size as i64;
    fincore_result.cached_percent = cached_percent as f64;
    fincore_result.cached_pages = DisplayVec(cached_pages);

    Ok(fincore_result)
}
