mod lib;
use std::env;
use std::path::Path;
use std::process;

use lib::copy_raw_file;

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
    if args.len() != 3 {
        log::error!("arguments error!");
        log::error!("eg: {} ./input ./output", args[0]);
        process::exit(1);
    }

    let input_path = Path::new(&args[1]);
    let output_path = Path::new(&args[2]);
    if !input_path.exists() {
        log::error!("input path {:?} not exists", input_path);
        process::exit(1);
    }
    if !output_path.exists() {
        log::error!("output path {:?} not exists", output_path);
        process::exit(1);
    }
    copy_raw_file(input_path, output_path);
}
