use std::env;
use std::path::Path;
use std::process;

use multimedia::copy_file;
mod multimedia;

// cargo run raw ./input ./output/raw
// cargo run jpg ./input ./output/jpg
fn main() {
    let format = tracing_subscriber::fmt::format()
        .with_level(true)
        .with_target(false)
        .with_thread_ids(false)
        .with_thread_names(false);
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_writer(std::io::stdout)
        .with_ansi(true)
        .event_format(format)
        .init();
    // 处理输入参数，获取输入和输出路径
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        log::error!("arguments error!");
        log::error!("eg: {} [raw|jpg|heic] ./input ./output", args[0]);
        process::exit(1);
    }

    let input_type = &args[1];
    let input_path = Path::new(&args[2]);
    let output_path = Path::new(&args[3]);
    if !input_path.exists() {
        log::error!("input path {:?} not exists", input_path);
        process::exit(1);
    }
    if !output_path.exists() {
        log::error!("output path {:?} not exists", output_path);
        process::exit(1);
    }
    if input_type.eq("raw") {
        copy_file(".cr2", input_path, output_path);
    } else if input_type.eq("jpg") {
        copy_file(".jpg", input_path, output_path);
    } else if input_type.eq("heic") {
        copy_file(".heic", input_path, output_path);
    } else {
        log::error!("input type {:?} is not supported", input_type);
        process::exit(1);
    }
}
