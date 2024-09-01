use std::collections::BTreeSet;
use std::cmp::Ordering;
use std::os::unix::fs::MetadataExt;
use std::fs::Metadata;

// Define the DevAndInode struct
#[derive(Eq, PartialEq)]
struct DevAndInode {
    dev: u64,
    ino: u64,
}

// Implement Ord and PartialOrd for DevAndInode to allow ordering in BTreeSet
impl Ord for DevAndInode {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.ino.cmp(&other.ino) {
            Ordering::Equal => self.dev.cmp(&other.dev),
            other => other,
        }
    }
}

impl PartialOrd for DevAndInode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// Global BTreeSet to hold seen inodes
lazy_static::lazy_static! {
    static ref SEEN_INODES: std::sync::Mutex<BTreeSet<DevAndInode>> = std::sync::Mutex::new(BTreeSet::new());
}

// Function to add device and inode information to the set of known inodes
fn add_object(metadata: &Metadata) -> Result<(), Box<dyn std::error::Error>> {
    let newp = DevAndInode {
        dev: metadata.dev(),
        ino: metadata.ino(),
    };

    let mut seen_inodes = SEEN_INODES.lock().unwrap();
    if !seen_inodes.insert(newp) {
        return Err(Box::from("Insertion failed: out of memory or duplicate entry"));
    }

    Ok(())
}

fn main() {
    // Example usage
    let metadata = std::fs::metadata("some_file_path").expect("Unable to get metadata");
    add_object(&metadata).expect("Failed to add object");
}