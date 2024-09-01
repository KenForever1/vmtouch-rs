use libc::{ioctl, S_IFBLK};
use std::fs::File;
use std::os::fd::AsRawFd;
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::io;
const BLKGETSIZE64: u64 = 0x80081272;


pub(crate) fn is_block_device<P: AsRef<Path>>(path: P) -> std::io::Result<bool> {
    let metadata = std::fs::metadata(path)?;
    let file_type = metadata.mode() & libc::S_IFMT;
    Ok(file_type == S_IFBLK)
}

pub(crate) fn get_block_device_size<P: AsRef<Path>>(path: P) -> io::Result<i64> {
    let file = File::open(path)?;
    let fd = file.as_raw_fd();
    let mut size: i64 = 0;

    // Perform the ioctl call to get the size
    let ret = unsafe { ioctl(fd, BLKGETSIZE64, &mut size) };

    if ret != 0 {
        return Err(io::Error::last_os_error());
    }

    Ok(size)
}


#[cfg(test)]
mod tests{
    use super::*;

    #[test]
    fn test_get_block_size(){
        println!("vmtouch");

        let path = "/dev/sda1"; // Replace with the path to your block device
        match is_block_device(path) {
            Ok(true) => println!("The file is a block device."),
            Ok(false) => println!("The file is not a block device."),
            Err(e) => println!("Failed to get metadata: {}", e),
        }
    
        match get_block_device_size(path) {
            Ok(size) => println!("The block device size is: {}", size),
            Err(e) => println!("Failed to get block device size: {}", e),
        }
    }
}
