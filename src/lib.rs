use exif::{In, Reader, Tag};
use std::env;
use std::fs::File;
use std::io;
use std::io::BufReader;
use std::io::{Error, ErrorKind};
use walkdir::{DirEntry, WalkDir};

pub fn read_input() -> Result<String, &'static str> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        return Err("Please input like this: {} /home/zhaogj/photo");
    }
    Ok(args[1].clone())
}

pub fn find_media(path: &str) {
    let walker = WalkDir::new(path).into_iter();
    for entry in walker.filter_entry(|e| !is_hidden(e)) {
        let entry = entry.unwrap();
        if is_media(&entry) {
            let path = entry.path().display().to_string();
            // println!("path: {}", &path);
            println!("create time: {}", read_exif(&path).unwrap());
        }
    }
}

pub fn read_exif(path: &str) -> Result<String, io::Error> {
    let file = File::open(path);
    let file = match file {
        Ok(file) => file,
        Err(e) => return Err(e),
    };

    let exif = Reader::new()
        .read_from_container(&mut BufReader::new(&file))
        .unwrap();
    match exif.get_field(Tag::DateTime, In::PRIMARY) {
        Some(data_time) => Ok(data_time.display_value().to_string()),
        None => Err(Error::new(ErrorKind::Other, "")),
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
        .map(|s: &str| s.to_lowercase().ends_with(".jpg") || s.to_lowercase().ends_with(".jpeg"))
        .unwrap_or(false)
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_hidden() {
        for entry in WalkDir::new(".gitignore") {
            let entry = entry.unwrap();
            assert_eq!(is_hidden(&entry), true);
        }
        for entry in WalkDir::new("Cargo.toml") {
            let entry = entry.unwrap();
            assert_eq!(is_hidden(&entry), false);
        }
    }

    #[test]
    fn test_is_media() {
        for entry in WalkDir::new("Cargo.toml") {
            let entry = entry.unwrap();
            assert_eq!(is_hidden(&entry), false);
        }
        for entry in WalkDir::new("./data/IMG_2075.JPG") {
            let entry = entry.unwrap();
            assert_eq!(is_media(&entry), true);
        }
    }
}
