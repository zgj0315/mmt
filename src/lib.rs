use chrono::prelude::*;
use exif::{In, Reader, Tag};
use std::env;
use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::io::ErrorKind;
use std::path::Path;
use walkdir::{DirEntry, WalkDir};

pub struct Config {
    pub src_path: String,
    pub dst_path: String,
}

pub fn parse_config(args: &[String]) -> Result<Config, &'static str> {
    if args.len() != 3 {
        Err("arguments count must be 3.")
    } else {
        let src_path = args[1].clone();
        let dst_path = args[2].clone();
        Ok(Config { src_path, dst_path })
    }
}

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

pub fn read_exif(path: &str) -> Result<String, &'static str> {
    let file = File::open(path);
    let file = match file {
        Ok(file) => file,
        Err(e) => match e.kind() {
            ErrorKind::NotFound => return Err("file not found."),
            _other_error => return Err("read failed."),
        },
    };
    let exif = match Reader::new().read_from_container(&mut BufReader::new(&file)) {
        Ok(exif) => exif,
        Err(e) => {
            println!("{}", e);
            return Err("read exif failed");
        }
    };
    match exif.get_field(Tag::DateTime, In::PRIMARY) {
        Some(data_time) => Ok(data_time.display_value().to_string()),
        None => Err("not have datetime"),
    }
}

pub fn is_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with("."))
        .unwrap_or(false)
}

pub fn is_media(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s: &str| s.to_lowercase().ends_with(".jpg") || s.to_lowercase().ends_with(".jpeg"))
        .unwrap_or(false)
}

pub fn copy_to_dst(dst: &str, file: &str, date_time: &str) {
    let dt = Utc
        .datetime_from_str(date_time, "%Y-%m-%d %H:%M:%S")
        .unwrap();
    let path_str = format!(
        "{}/{}/{}{:02}/{}{:02}{:02}",
        dst,
        dt.year(),
        dt.year(),
        dt.month(),
        dt.year(),
        dt.month(),
        dt.day()
    );
    let file_path = Path::new(file);
    let file_name = file_path.file_name().unwrap().to_str().unwrap();
    let file_dst_str = format!("{}/{}", path_str, file_name);
    fs::create_dir_all(&path_str).unwrap();
    fs::copy(&file, &file_dst_str).unwrap();
    println!("copy {} to {}", file, file_dst_str);
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
