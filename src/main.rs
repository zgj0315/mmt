use mmt::*;
use std::env;
use std::process;
use walkdir::WalkDir;

fn main() {
    let args: Vec<String> = env::args().collect();
    let config = match parse_config(&args) {
        Ok(config) => config,
        Err(e) => {
            println!("{}\neg: {} /home/zhaogj/photo /home/zhaogj/export", e, args[0]);
            process::exit(1);
        }
    };
    let walker = WalkDir::new(config.src_dir).into_iter();
    for entry in walker.filter_entry(|e| !is_hidden(e)) {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                println!("{}\nPlease check your path.", e);
                process::exit(1);
            }
        };

        if is_media(&entry) {
            let file_path = entry.path().display().to_string();
            match read_exif(&file_path) {
                Ok(create_time) => {
                    copy_to_dst(&config.dst_dir, &file_path, &create_time);
                }
                Err(e) => {
                    println!("read datetime failed: {}", e)
                }
            };
        }
    }
}
