use chrono::{DateTime, Local, TimeZone};
use exif::{In, Reader, Tag};
use rusqlite::Connection;
use std::{
    fs::{self, copy, create_dir_all, File},
    io::{BufReader, Read},
    path::Path,
};
use walkdir::WalkDir;

pub fn copy_raw_file(input_path: &Path, output_path: &Path) {
    let db_path = output_path.join("file.db");
    let conn = Connection::open(db_path).unwrap();
    let sql = "CREATE TABLE IF NOT EXISTS tbl_file (
        file_path  TEXT PRIMARY KEY,
        file_size  INTEGER NOT NULL,
        file_md5  TEXT NOT NULL
    )";
    conn.execute(sql, ()).unwrap();
    let walk_dir = WalkDir::new(input_path);
    'walk_dir: for entry in walk_dir {
        let entry = entry.unwrap();
        let file_type = entry.file_type();
        if file_type.is_file() {
            let file_name = entry.file_name();
            if file_name
                .to_str()
                .unwrap()
                .to_ascii_lowercase()
                .ends_with(".cr2")
            {
                let input_file_path = entry.path();
                let create_time = get_create_time(input_file_path);
                let yyyy = create_time.format("%Y").to_string();
                let yyyymm = create_time.format("%Y%m").to_string();
                let output_path = output_path.join(yyyy).join(yyyymm);
                if !output_path.exists() {
                    create_dir_all(&output_path).unwrap();
                }
                let mut output_file_path;
                let mut count = 0;
                'count_loop: loop {
                    let output_file_name = input_file_path.file_name().unwrap().to_str().unwrap();
                    if count == 0 {
                        output_file_path = output_path.join(output_file_name);
                    } else {
                        let (file_name, suffix) = output_file_name.rsplit_once(".").unwrap();
                        output_file_path =
                            output_path.join(format!("{}_{}.{}", file_name, count, suffix));
                    }
                    if output_file_path.exists() {
                        if is_same_file(input_file_path, &output_file_path) {
                            log::info!("file {:?} already exists", input_file_path);
                            continue 'walk_dir;
                        }
                        count += 1;
                    } else {
                        break 'count_loop;
                    }
                }
                log::info!("copying {:?} to {:?}", input_file_path, output_file_path);
                copy(input_file_path, output_file_path).unwrap();
            }
        }
    }
}

fn get_create_time(path: &Path) -> DateTime<Local> {
    let file = File::open(path).unwrap();
    let mut buf_reader = BufReader::new(file);
    let reader = Reader::new();
    match reader.read_from_container(&mut buf_reader) {
        Ok(exif) => match exif.get_field(Tag::DateTimeOriginal, In::PRIMARY) {
            Some(data_time) => {
                // 2022-01-23 12:42:12
                let value = &data_time.display_value().to_string();
                let (year, value) = value.split_once("-").unwrap();
                let (month, value) = value.split_once("-").unwrap();
                let (day, value) = value.split_once(" ").unwrap();
                let (hour, value) = value.split_once(":").unwrap();
                let (minute, second) = value.split_once(":").unwrap();
                let create_time = Local
                    .with_ymd_and_hms(
                        year.parse().unwrap(),
                        month.parse().unwrap(),
                        day.parse().unwrap(),
                        hour.parse().unwrap(),
                        minute.parse().unwrap(),
                        second.parse().unwrap(),
                    )
                    .unwrap();
                return create_time;
            }
            None => {
                log::error!("DateTimeOriginal not exist");
                return Local::now();
            }
        },
        Err(e) => {
            log::error!("get_create_time error: {:?}", e);
            return Local::now();
        }
    }
}
#[derive(Debug)]
struct TblFile {
    file_path: String,
    file_size: usize,
    file_md5: String,
}
fn get_output_file_from_db(file_path: &str, conn: &Connection) -> Option<TblFile> {
    let sql = format!("SELECT * FROM tbl_file WHERE file_path = '{}'", file_path);
    let mut stmt = conn.prepare(&sql).unwrap();
    let rows = stmt
        .query_map([], |row| {
            Ok(TblFile {
                file_path: row.get(0).unwrap(),
                file_size: row.get(1).unwrap(),
                file_md5: row.get(2).unwrap(),
            })
        })
        .unwrap();
    for tbl_file in rows {
        let tbl_file = tbl_file.unwrap();
        return Some(tbl_file);
    }
    None
}
fn is_same_file(path_a: &Path, path_b: &Path) -> bool {
    let size_a = fs::metadata(path_a).unwrap().len();
    let size_b = fs::metadata(path_b).unwrap().len();
    if size_a != size_b {
        log::info!("diff size, {:?} and {:?}", path_a, path_b);
        return false;
    }
    let mut file_a = File::open(path_a).unwrap();
    let mut file_b = File::open(path_b).unwrap();
    let mut buf = Vec::new();
    file_a.read_to_end(&mut buf).unwrap();
    let md5_a = md5::compute(buf);
    let mut buf = Vec::new();
    file_b.read_to_end(&mut buf).unwrap();
    let md5_b = md5::compute(buf);
    if md5_a == md5_b {
        log::info!("same md5, {:?} and {:?}", path_a, path_b);
        return true;
    } else {
        return false;
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs::{self, File},
        path::Path,
    };

    use chrono::{DateTime, Local};
    use walkdir::WalkDir;

    use super::{get_create_time, is_same_file};

    #[test]
    fn it_works() {
        let file = File::open("./data/IMG_2075.JPG").unwrap();
        let create_time = file.metadata().unwrap().created().unwrap();
        let create_time: DateTime<Local> = create_time.into();
        let last_modified = file.metadata().unwrap().modified().unwrap();
        let last_modified: DateTime<Local> = last_modified.into();
        println!(
            "create_time: {}, last_modified: {}",
            create_time, last_modified
        );
        let file = File::open("/Volumes/photo/export/2010/201008/20100828/IMG_5624.JPG").unwrap();
        let create_time = file.metadata().unwrap().created().unwrap();
        let create_time: DateTime<Local> = create_time.into();
        let last_modified = file.metadata().unwrap().modified().unwrap();
        let last_modified: DateTime<Local> = last_modified.into();
        println!(
            "create_time: {}, last_modified: {}",
            create_time, last_modified
        );
    }

    // cargo test lib::tests::walk_dir -- --nocapture
    #[test]
    fn walk_dir() {
        let dir = WalkDir::new("/Volumes/photo/original");
        for entry in dir {
            let entry = entry.unwrap();
            let file_type = entry.file_type();
            if file_type.is_file() {
                let file_name = entry.file_name();
                print!("file_name: {:?}", file_name);
                let depth = entry.depth();
                print!("depth: {:?}", depth);
                let path = entry.path();
                print!("path: {:?}", path);
                let metadata = entry.metadata().unwrap();
                let create_time = metadata.created().unwrap();
                let create_time: DateTime<Local> = create_time.into();
                let last_modified = metadata.modified().unwrap();
                let last_modified: DateTime<Local> = last_modified.into();
                println!(
                    "create_time: {}, last_modified: {}",
                    create_time, last_modified
                );
            }
        }
    }

    // cargo test lib::tests::copy_file -- --nocapture
    #[test]
    fn copy_file() {
        let file_ori = "./data/IMG_2075.JPG";
        let file_dst = "./data/IMG_2075_bak.JPG";
        fs::copy(file_ori, file_dst).unwrap();
    }

    // cargo test lib::tests::test_get_create_time -- --nocapture
    #[test]
    fn test_get_create_time() {
        let path = Path::new("/Volumes/photo/original/2022/202202/20220205/IMG_2455.CR2");
        let create_time = get_create_time(path);
        println!("create_time: {:?}", create_time);
    }

    // cargo test lib::tests::test_is_same_file -- --nocapture
    #[test]
    fn test_is_same_file() {
        let path_a = Path::new("/Volumes/photo/original/2022/202202/20220205/IMG_2455.CR2");
        let path_b = Path::new("/Volumes/photo/original/2022/202202/20220205/IMG_2455.CR2");
        println!("same: {}", is_same_file(path_a, path_b));
    }
}
