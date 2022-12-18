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
    let sql = "CREATE TABLE IF NOT EXISTS tbl_output_file (
        file_path  TEXT PRIMARY KEY,
        file_size  INTEGER NOT NULL,
        file_md5  TEXT NOT NULL
    )";
    conn.execute(sql, ()).unwrap();
    let sql = "CREATE TABLE IF NOT EXISTS tbl_input_file (
        file_path  TEXT PRIMARY KEY
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
                let file_path = input_file_path
                    .strip_prefix(input_path.to_str().unwrap())
                    .unwrap()
                    .to_str()
                    .unwrap();
                if !need_copy(file_path, &conn) {
                    log::info!("already copied file {:?}", input_file_path);
                    continue 'walk_dir;
                }
                let create_time = get_create_time(input_file_path);
                let yyyy = create_time.format("%Y").to_string();
                let yyyymm = create_time.format("%Y%m").to_string();
                let output_path_date = output_path.join(yyyy).join(yyyymm);
                if !output_path_date.exists() {
                    create_dir_all(&output_path_date).unwrap();
                }
                let mut output_file_path;
                let mut count = 0;
                'count_loop: loop {
                    if count == 0 {
                        let output_file_name =
                            input_file_path.file_name().unwrap().to_str().unwrap();
                        output_file_path = output_path_date.join(output_file_name);
                    } else {
                        let file_stem = input_file_path.file_stem().unwrap().to_str().unwrap();
                        let extension = input_file_path.extension().unwrap().to_str().unwrap();
                        output_file_path =
                            output_path_date.join(format!("{}_{}.{}", file_stem, count, extension));
                    }
                    if output_file_path.exists() {
                        let file_path = output_file_path
                            .strip_prefix(output_path.to_str().unwrap())
                            .unwrap()
                            .to_str()
                            .unwrap();
                        if is_same_file(input_file_path, &output_file_path, file_path, &conn) {
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
    file_size: usize,
    file_md5: String,
}
fn get_output_file_from_db(file_path: &str, conn: &Connection) -> Option<TblFile> {
    let sql = format!(
        "SELECT file_size, file_md5 FROM tbl_output_file WHERE file_path = '{}'",
        file_path
    );
    let mut stmt = conn.prepare(&sql).unwrap();
    let rows = stmt
        .query_map([], |row| {
            Ok(TblFile {
                file_size: row.get(0).unwrap(),
                file_md5: row.get(1).unwrap(),
            })
        })
        .unwrap();
    for tbl_file in rows {
        let tbl_file = tbl_file.unwrap();
        return Some(tbl_file);
    }
    None
}
fn is_same_file(input_path: &Path, output_path: &Path, file_path: &str, conn: &Connection) -> bool {
    if input_path == output_path {
        log::info!("same file, {:?}", input_path);
        return false;
    }
    let tbl_file = get_output_file_from_db(file_path, &conn);
    let input_size = fs::metadata(input_path).unwrap().len();
    let output_size = match &tbl_file {
        Some(tbl_file) => tbl_file.file_size as u64,
        None => fs::metadata(output_path).unwrap().len(),
    };
    if input_size != output_size {
        log::info!("diff size, {:?} and {:?}", input_path, output_path);
        return false;
    }
    let mut file_a = File::open(input_path).unwrap();
    let mut buf = Vec::new();
    file_a.read_to_end(&mut buf).unwrap();
    let md5_a = format!("{:X}", md5::compute(buf));
    let md5_b = match tbl_file {
        Some(tbl_file) => tbl_file.file_md5,
        None => {
            let mut file_b = File::open(output_path).unwrap();
            let mut buf = Vec::new();
            file_b.read_to_end(&mut buf).unwrap();
            let md5 = format!("{:X}", md5::compute(buf));
            let sql = "INSERT INTO tbl_output_file (
                file_path, file_size, file_md5
            ) VALUES (
                ?1, ?2, ?3
            )";
            conn.execute(sql, (file_path, output_size, &md5)).unwrap();
            md5
        }
    };
    if md5_a == md5_b {
        log::info!("same md5, {:?} and {:?}", input_path, output_path);
        return true;
    } else {
        return false;
    }
}

fn need_copy(file_path: &str, conn: &Connection) -> bool {
    let sql = format!(
        "SELECT file_path FROM tbl_input_file WHERE file_path = '{}'",
        file_path
    );
    let mut stmt = conn.prepare(&sql).unwrap();
    if stmt.exists([]).unwrap() {
        return false;
    } else {
        let sql = "INSERT INTO tbl_input_file (
            file_path
        ) VALUES (
            ?1
        )";
        conn.execute(sql, (file_path,)).unwrap();
        return true;
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
        // let path_a = Path::new("/Volumes/photo/original/2022/202202/20220205/IMG_2455.CR2");
        // let path_b = Path::new("/Volumes/photo/original/2022/202202/20220205/IMG_2455.CR2");
        // println!("same: {}", is_same_file(path_a, path_b));
    }

    // cargo test lib::tests::test_code -- --nocapture
    #[test]
    fn test_code() {
        let path = Path::new("./output/");
        let path_date = path.join("2022").join("202212");
        let path_str = path.to_str().unwrap();
        let path_date_str = path_date.to_str().unwrap();
        println!("path_str: {}, path_date_str: {}", path_str, path_date_str);
        let date = path_date_str.replace(path_str, "");
        println!("date: {}", date);
        let date = path_date.strip_prefix(path_str).unwrap();
        println!("date: {}", date.to_str().unwrap());
    }
}
