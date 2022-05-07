use chrono::prelude::*;
use exif::{In, Reader, Tag};
use std::fs;
use std::io::Read;
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;
use walkdir::{DirEntry, WalkDir};

pub async fn read_file_list_and_input_buffer(
    src_dir: &String,
    file_buffer: Arc<Mutex<Vec<String>>>,
) {
    let walker = WalkDir::new(src_dir).into_iter();
    for entry in walker.filter_entry(|e| !is_hidden(e)) {
        loop {
            let mut file_list = file_buffer.lock().unwrap();
            if file_list.len() < 10 {
                file_list.push(entry.unwrap().path().display().to_string());
                drop(file_list);
                break;
            } else {
                drop(file_list);
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }
    }
}

pub async fn read_file_info_and_copy_file(dst_dir: &String, file_buffer: Arc<Mutex<Vec<String>>>) {
    let mut sleep_time = 0;
    while sleep_time < 7 {
        let mut file_path: String = String::from("");
        let mut file_list = file_buffer.lock().unwrap();
        if file_list.is_empty() {
            drop(file_list);
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            sleep_time += 1;
        } else {
            file_path = file_list[0].clone();
            file_list.remove(0);
            drop(file_list);
            sleep_time = 0;
        }

        if sleep_time == 0 {
            if is_media(&file_path) {
                match read_exif(&file_path) {
                    Ok(create_time) => {
                        copy_to_dst(dst_dir, &file_path, &create_time);
                    }
                    Err(e) => {
                        println!("read file: {} datetime failed: {}", file_path, e)
                    }
                };
            }
        }
    }
}

pub fn read_exif(path: &str) -> Result<String, &'static str> {
    let file = std::fs::File::open(path).unwrap();
    let exif = match Reader::new().read_from_container(&mut std::io::BufReader::new(&file)) {
        Ok(exif) => exif,
        Err(e) => {
            println!("{}, file: {}", e, path);
            return Err("read exif failed");
        }
    };
    match exif.get_field(Tag::DateTimeOriginal, In::PRIMARY) {
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

pub fn is_media(file_path: &String) -> bool {
    if file_path.to_lowercase().ends_with(".jpg")
        || file_path.to_lowercase().ends_with(".jpeg")
        || file_path.to_lowercase().ends_with(".cr2")
    {
        true
    } else {
        false
    }
}

pub fn copy_to_dst(dst_dir: &str, file_path: &str, create_time_str: &str) {
    let create_time = Utc
        .datetime_from_str(create_time_str, "%Y-%m-%d %H:%M:%S")
        .unwrap();
    let dst_dir = format!(
        "{}/{}/{}{:02}/{}{:02}{:02}",
        dst_dir,
        create_time.year(),
        create_time.year(),
        create_time.month(),
        create_time.year(),
        create_time.month(),
        create_time.day()
    );
    let src_file_path = Path::new(file_path);
    let src_file_name = src_file_path.file_name().unwrap().to_str().unwrap();
    let dst_file_path = format!("{}/{}", dst_dir, src_file_name);
    let dst_file_path = Path::new(&dst_file_path);
    let is_same = match is_same_file(src_file_path, dst_file_path) {
        Ok(is_same) => is_same,
        Err(_) => false,
    };
    if is_same {
        println!("same file: {:?}, {:?}", src_file_path, dst_file_path);
    } else {
        std::fs::create_dir_all(dst_dir).unwrap();
        std::fs::copy(src_file_path, dst_file_path).unwrap();
        println!("copy {:?} to {:?}", src_file_path, dst_file_path);
    }
}

pub fn is_same_file<P: AsRef<Path>, Q: AsRef<Path>>(p: P, q: Q) -> std::io::Result<bool> {
    let _len_p = fs::metadata(&p)?.len();
    let _len_q = fs::metadata(&q)?.len();
    if _len_p == _len_q {
        let mut _file_p = fs::File::open(&p)?;
        let mut _buffer_p = Vec::new();
        _file_p.read_to_end(&mut _buffer_p)?;
        let _md5_p = md5::compute(_buffer_p);
        let mut _file_q = fs::File::open(&q)?;
        let mut _buffer_q = Vec::new();
        _file_q.read_to_end(&mut _buffer_q)?;
        let _md5_q = md5::compute(_buffer_q);
        if _md5_p.eq(&_md5_q) {
            return Ok(true);
        }
    }
    Ok(false)
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
            let file_path = entry.unwrap().path().display().to_string();
            assert_eq!(is_media(&file_path), true);
        }
    }
    #[test]
    fn test_is_same_file() {
        assert!(!is_same_file("./README.md", "./Cargo.toml").unwrap());
    }
}
