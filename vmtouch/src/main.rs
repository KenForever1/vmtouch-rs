mod error;
mod fileop;
mod helper;

use error::{Error, Result};
use helper::page_size;
use libc::{mincore, mmap, munmap, MAP_FAILED, MAP_SHARED, O_NOATIME, PROT_READ};
use nix::fcntl::{posix_fadvise, PosixFadviseAdvice};
use nix::sys::mman::{msync, MsFlags};
use std::fs::OpenOptions;
use std::io::{self, ErrorKind};
use std::os::fd::AsRawFd;
use std::os::raw::c_void;
use std::os::unix::fs::OpenOptionsExt;
use std::ptr;

use clap::Parser;

/// a mem tool named vmtouch
#[derive(Default, Parser, Debug)]
#[command(name = "vmtouch", version, about, long_about = None)]
struct Args {
    #[arg(short = 'e', long, default_value = "false")]
    o_evict: bool,
    #[arg(short = 'v', long, default_value = "true")]
    o_verbose: bool,
    #[arg(short = 't', long, default_value = "true")]
    o_touch: bool,
    #[arg(short = 'l', long, default_value = "false")]
    o_lock: bool,
    #[arg(short = 'm', long, default_value_t = std::u32::MAX)]
    o_max_file_size: u32,
    #[arg(short = 'q', long, default_value = "false")]
    o_quiet: bool,
}

#[derive(Default)]
struct VmToucher {
    args: Args,
    junk_counter: usize,
    offset: i64,
    max_len: i64,
    total_pages: i64,
    total_pages_in_core: i64,
    page_size: usize,
}

impl VmToucher {
    fn new(args: Args) -> Self {
        Self {
            args: args,
            page_size: page_size(),
            ..Default::default()
        }
    }

    fn show(&self) {
        let total_pages_in_core_size = self.total_pages_in_core * self.page_size as i64;
        let total_pages_size = self.total_pages * self.page_size as i64;
        let total_pages_in_core_perc =
            ((total_pages_in_core_size as f64) / (total_pages_size as f64)) * 100.0;

        if !self.args.o_quiet {
            // printf("           Files: %" PRId64 "\n", total_files);
            // printf("     Directories: %" PRId64 "\n", total_dirs);

            println!(
                "{}",
                format!(
                    "Resident Pages: {} {} {} {} {:.3}",
                    self.total_pages,
                    self.total_pages_in_core,
                    total_pages_size,
                    total_pages_in_core_size,
                    total_pages_in_core_perc
                )
            )
        }
    }

    fn vmtouch_file(&mut self, path: &str) -> Result<()> {
        // 通过使用 O_NOATIME，你可以减少不必要的磁盘 I/O 操作，这对于提高性能和减少存储设备磨损都是非常有益的
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(O_NOATIME)
            .open(path)?;

        let len_of_file: i64;

        match fileop::is_block_device(path) {
            Ok(true) => {
                len_of_file = fileop::get_block_device_size(path)?;
            }
            Ok(false) => {
                len_of_file = file.metadata()?.len() as i64;
            }
            Err(e) => {
                return Err(Error::IoErr(e));
            }
        }

        if len_of_file > self.args.o_max_file_size as i64 {
            return Err(Error::ParamsErr(format!(
                "file {} len beyond max file size",
                path
            )));
        }

        let len_of_range: i64;

        if self.max_len > 0 && (self.offset + self.max_len) < len_of_file {
            len_of_range = self.max_len;
        } else if self.offset >= len_of_file {
            return Err(error::Error::ParamsErr(format!(
                "file {} smaller than offset, skipping",
                path
            )));
        } else {
            len_of_range = len_of_file - self.offset;
        }

        // 不能使用Mmap，因为在mlock时需要手动控制是否要munmap
        // let mmap = unsafe {
        //     MmapOptions::new()
        //         .len(len_of_range as usize)
        //         .offset(self.offset as i64)
        //         .map(&file)?
        // };
        // let mem = mmap.as_ptr();

        let fd = file.as_raw_fd();
        let mem = unsafe {
            mmap(
                ptr::null_mut(),
                len_of_range as usize,
                PROT_READ,
                MAP_SHARED,
                fd,
                self.offset as i64,
            )
        };

        if mem == MAP_FAILED {
            Err(Error::IoErr(io::Error::new(
                ErrorKind::Other,
                format!(
                    "unable to mmap file {} ({})",
                    path,
                    io::Error::last_os_error()
                ),
            )))?
        }

        if !helper::aligned_p(mem as *const u8) {
            Err(Error::IoErr(io::Error::new(
                ErrorKind::Other,
                format!("mmap({}) wasn't page aligned", path),
            )))?
        }

        let pages_in_range = helper::bytes2pages(len_of_range as usize);
        self.total_pages += pages_in_range as i64;

        let fd = file.as_raw_fd();

        if self.args.o_evict {
            let _ = self.evict_mem(fd, len_of_range, path);
        } else {
            let _ = self.other_mem(mem, pages_in_range);
        }

        if self.args.o_lock {
            if self.args.o_verbose {
                println!("Locking {}", path);
            }
            // let _ = mmap.lock();
            if unsafe { libc::mlock(mem, len_of_range as usize) } != 0 {
                // Convert errno to an error string
                // let err_msg = std::io::Error::last_os_error().to_string();
                // eprintln!("mlock: {} ({})", path, err_msg);
                Err(Error::IoErr(std::io::Error::last_os_error()))?
            }
        }

        if !self.args.o_lock {
            let ret = unsafe { munmap(mem as *mut c_void, len_of_range as usize) };

            if ret != 0 {
                Err(std::io::Error::last_os_error())?;
            }
        }

        Ok(())
    }

    fn other_mem(&mut self, mp: *mut c_void, num_page: usize) -> Result<()> {
        let mut mincore_vec = vec![0u8; num_page];

        let ret = unsafe { mincore(mp, num_page, mincore_vec.as_mut_ptr()) };

        if ret != 0 {
            Err(std::io::Error::last_os_error())?;
        }

        for (_, &val) in mincore_vec.iter().enumerate() {
            if val & 1 != 0 {
                self.total_pages_in_core += 1
            }
        }

        if self.args.o_verbose {
            // todo
        }

        if self.args.o_touch {
            // junk_counter
            for i in 0..num_page {
                // self.junk_counter += mp[i * page_size()] as usize;
                unsafe {
                    self.junk_counter += *(mp as *const u8).add(i * page_size()) as usize;
                }

                mincore_vec[i] = 1;
            }

            if self.args.o_verbose {
                // todo
            }
        }

        Ok(())
    }

    fn evict_mem(&self, fd: i32, len_of_range: i64, path: &str) -> Result<()> {
        if self.args.o_verbose {
            println!("Evicting {}", path);
        }

        #[cfg(any(target_os = "linux", target_os = "hpux"))]
        {
            let ret = posix_fadvise(
                fd,
                self.offset as i64,
                len_of_range as i64,
                PosixFadviseAdvice::POSIX_FADV_DONTNEED,
            );
            if let Err(err) = ret {
                eprintln!("unable to posix_fadvise file {} ({})", path, err);
            }
        }

        #[cfg(any(target_os = "freebsd", target_os = "solaris", target_os = "macos"))]
        {
            let ret = msync(mem, len_of_range, MsFlags::MS_INVALIDATE);
            if let Err(err) = ret {
                eprintln!("unable to msync invalidate file {} ({})", path, err);
            }
        }

        #[cfg(not(any(
            target_os = "linux",
            target_os = "hpux",
            target_os = "freebsd",
            target_os = "solaris",
            target_os = "macos"
        )))]
        {
            return Err(io::Error::new(
                ErrorKind::Other,
                "cache eviction not (yet?) supported on this platform",
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vmtouch_file() {
        let args = Args::default();
        let mut vmtoucher = VmToucher::new(args);
        let path = "./README.md";
        let _ = vmtoucher.vmtouch_file(path);
    }
}

fn main() -> Result<()> {
    let args = Args::parse();
    let mut vmtoucher = VmToucher::new(args);
    let path = "./a";
    let _ = vmtoucher.vmtouch_file(path);
    vmtoucher.show();
    Ok(())
}
