
use walkdir::{DirEntry, WalkDir};

pub fn find_media(path: &str) {
    let walker = WalkDir::new(path).into_iter();
    for entry in walker.filter_entry(|e| !is_hidden(e)) {
        let entry = entry.unwrap();
        if is_media(&entry) {
            println!("path: {}", entry.path().display());
        }
    }
}

fn is_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with("."))
        .unwrap_or(false)
}

fn is_media(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s: &str| s.ends_with(".jpg") || s.ends_with(".jpeg"))
        .unwrap_or(false)
}
