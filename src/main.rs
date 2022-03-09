use mmt::*;
use std::env;
use std::process;
use walkdir::WalkDir;

fn main() {
    let args: Vec<String> = env::args().collect();
    let path = match parse_config(&args) {
        Ok(path) => path,
        Err(e) => {
            println!("{}\neg: {} /home/zhaogj/photo", e, args[0]);
            process::exit(1);
        }
    };
    let walker = WalkDir::new(path).into_iter();
    for entry in walker.filter_entry(|e| !is_hidden(e)) {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                println!("{}\nPlease check your path.", e);
                process::exit(1);
            }
        };

        if is_media(&entry) {
            let path_str = entry.path().display().to_string();
            println!("path: {}", &path_str);
            match read_exif(&path_str) {
                Ok(data_time) => {
                    println!("create time: {}", data_time);
                    copy_to_dst("/Users/zhaoguangjian/tmp", &path_str, &data_time);
                }
                Err(e) => {
                    println!("read datetime failed: {}", e)
                }
            };
        }
    }
}
