use std::{
    fs::{self, copy, create_dir_all, File},
    io::{BufReader, Read},
    path::Path,
};

use chrono::{DateTime, Local, TimeZone};
use exif::{In, Reader, Tag};
use rusqlite::Connection;
use walkdir::WalkDir;

#[derive(Debug)]
struct TblInputFile {
    file_path: String,
}
pub fn copy_file(suffix: &str, input_path: &Path, output_path: &Path) {
    let db_path = output_path.join("file.db");
    let conn = Connection::open(db_path).unwrap();
    put_dir_to_db(suffix, input_path, &conn);
    let sql = "SELECT file_path FROM tbl_input_file WHERE copy_time = 0";
    let mut stmt = conn.prepare(sql).unwrap();
    let tbl_input_file_iter = stmt
        .query_map([], |row| {
            Ok(TblInputFile {
                file_path: row.get(0).unwrap(),
            })
        })
        .unwrap();
    'walk_dir: for tbl_input_file in tbl_input_file_iter {
        let file_path = tbl_input_file.unwrap().file_path;
        let input_file_path = input_path.join(&file_path);
        let create_time = get_create_time(&input_file_path);
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
                let output_file_name = input_file_path.file_name().unwrap().to_str().unwrap();
                output_file_path = output_path_date.join(output_file_name);
            } else {
                let file_stem = input_file_path.file_stem().unwrap().to_str().unwrap();
                let extension = input_file_path.extension().unwrap().to_str().unwrap();
                output_file_path =
                    output_path_date.join(format!("{}_{}.{}", file_stem, count, extension));
            }
            if output_file_path.exists() {
                if is_same_file(&input_file_path, &output_file_path) {
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
        let sql = "UPDATE tbl_input_file SET copy_time = ?1 WHERE file_path = ?2";
        conn.execute(sql, (Local::now().timestamp_millis(), file_path))
            .unwrap();
    }
}

fn put_dir_to_db(suffix: &str, input_path: &Path, conn: &Connection) {
    // 初始化数据库表
    let sql = "CREATE TABLE IF NOT EXISTS tbl_input_file (
        file_path  TEXT PRIMARY KEY,
        copy_time  INTEGER NOT NULL
    )";
    conn.execute(sql, ()).unwrap();
    let sql = "SELECT * FROM tbl_input_file";
    let mut stmt = conn.prepare(sql).unwrap();
    if stmt.exists([]).unwrap() {
        // 数据库里不为空
        let sql = "SELECT * FROM tbl_input_file WHERE copy_time = -1";
        let mut stmt = conn.prepare(sql).unwrap();
        if !stmt.exists([]).unwrap() {
            // copy_time不存在-1，说明上次扫描完毕
            log::info!("cancel scan dir, all file in db.");
            return;
        }
    }
    // 遍历目录，写入数据库
    for entry in WalkDir::new(input_path) {
        let entry = entry.unwrap();
        let file_type = entry.file_type();
        if file_type.is_file() {
            let file_name = entry.file_name();
            if file_name
                .to_str()
                .unwrap()
                .to_ascii_lowercase()
                .ends_with(suffix)
            {
                let input_file_path = entry.path();
                let file_path = input_file_path
                    .strip_prefix(input_path.to_str().unwrap())
                    .unwrap()
                    .to_str()
                    .unwrap();
                log::info!("find file: {}", file_path);
                let sql = format!(
                    "SELECT file_path FROM tbl_input_file WHERE file_path = '{}'",
                    file_path
                );
                let mut stmt = conn.prepare(&sql).unwrap();
                if stmt.exists([]).unwrap() {
                } else {
                    let sql = "INSERT INTO tbl_input_file (
                    file_path, copy_time
                ) VALUES (
                    ?1, ?2
                )";
                    conn.execute(sql, (file_path, -1)).unwrap();
                }
            }
        }
    }
    // 如果扫描完成，将copy_time设置为零
    let sql = "UPDATE tbl_input_file SET copy_time = 0 WHERE copy_time = -1";
    conn.execute(sql, ()).unwrap();
}

pub fn get_create_time(path: &Path) -> DateTime<Local> {
    let file = File::open(path).unwrap();
    let mut buf_reader = BufReader::new(file);
    let reader = Reader::new();
    match reader.read_from_container(&mut buf_reader) {
        Ok(exif) => {
            match exif.get_field(Tag::DateTimeOriginal, In::PRIMARY) {
                Some(date_time) => {
                    // 2022-01-23 12:42:12
                    let value = &date_time.display_value().to_string();
                    let value = value.replace("\"", "");
                    let mut split = "-";
                    if value.contains(".") {
                        split = ".";
                    }
                    let (year, value) = value.split_once(split).unwrap();
                    let (month, value) = value.split_once(split).unwrap();
                    let (day, value) = value.split_once(" ").unwrap();
                    let (hour, value) = value.split_once(":").unwrap();
                    let (minute, second) = value.split_once(":").unwrap();
                    let create_time = Local.with_ymd_and_hms(
                        year.parse().unwrap(),
                        month.parse().unwrap(),
                        day.parse().unwrap(),
                        hour.parse().unwrap(),
                        minute.parse().unwrap(),
                        second.parse().unwrap(),
                    );
                    match create_time {
                        chrono::LocalResult::None => {
                            log::error!(
                                "local result none, file: {:?}, date_time: {}",
                                path,
                                &date_time.display_value().to_string()
                            );
                            return Local.with_ymd_and_hms(2000, 1, 1, 0, 0, 0).unwrap();
                        }
                        chrono::LocalResult::Single(create_time) => return create_time,
                        chrono::LocalResult::Ambiguous(min, max) => {
                            log::error!(
                            "local result ambiguous, file: {:?}, date_time: {}, min: {}, max: {}",
                            path,
                            &date_time.display_value().to_string(),min,max
                        );
                            panic!();
                        }
                    }
                }
                None => {
                    log::error!("DateTimeOriginal not exist");
                    return Local.with_ymd_and_hms(2000, 1, 1, 0, 0, 0).unwrap();
                }
            }
        }
        Err(e) => {
            log::error!("get_create_time error: {:?}", e);
            return Local.with_ymd_and_hms(2000, 1, 1, 0, 0, 0).unwrap();
        }
    }
}

pub fn is_same_file(input_path: &Path, output_path: &Path) -> bool {
    if input_path == output_path {
        log::info!("same file, {:?}", input_path);
        return true;
    }
    let input_size = fs::metadata(input_path).unwrap().len();
    let output_size = fs::metadata(output_path).unwrap().len();
    if input_size != output_size {
        log::info!("diff size, {:?} and {:?}", input_path, output_path);
        return false;
    }
    let mut file = File::open(input_path).unwrap();
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).unwrap();
    let input_md5 = format!("{:X}", md5::compute(buf));
    let mut file = File::open(output_path).unwrap();
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).unwrap();
    let output_md5 = format!("{:X}", md5::compute(buf));
    if input_md5 == output_md5 {
        log::info!("same md5, {:?} and {:?}", input_path, output_path);
        return true;
    } else {
        return false;
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use rusqlite::Connection;

    use super::{copy_file, get_create_time, put_dir_to_db};

    // cargo test multimedia::tests::test_get_create_time -- --nocapture
    #[test]
    fn test_get_create_time() {
        let path = Path::new("./input/IMG_7705.CR2");
        let create_time = get_create_time(path);
        println!("cr2 create_time: {:?}", create_time);
        let path = Path::new("./input/IMG_2075.JPG");
        let create_time = get_create_time(path);
        println!("jpg create_time: {:?}", create_time);
    }

    // cargo test multimedia::tests::test_put_dir_to_db -- --nocapture
    #[test]
    fn test_put_dir_to_db() {
        let db_path = "./output/file.db";
        let conn = Connection::open(db_path).unwrap();
        let input_path = Path::new("./input");
        put_dir_to_db(".jpg", input_path, &conn);
        put_dir_to_db(".cr2", input_path, &conn);
    }

    // cargo test multimedia::tests::test_copy_file -- --nocapture
    #[test]
    fn test_copy_file() {
        let input_path = Path::new("./input");
        let output_path = Path::new("./output");
        copy_file(".jpg", input_path, output_path);
        copy_file(".cr2", input_path, output_path);
    }
}
