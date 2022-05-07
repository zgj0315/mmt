use std::env;
use std::process;
use std::sync::Arc;
use std::sync::Mutex;

use mmt::read_file_info_and_copy_file;
use mmt::read_file_list_and_input_buffer;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // 处理输入参数，获取输入和输出路径
    let args: Vec<String> = env::args().collect();
    let src_dir: &String;
    let dst_dir: &String;
    if args.len() != 3 {
        println!("arguments error!");
        println!(
            "eg: {} /Users/zhaoguangjian/tmp/input /Users/zhaoguangjian/tmp/output",
            args[0]
        );
        process::exit(1);
    } else {
        src_dir = &args[1];
        dst_dir = &args[2];
    }

    // 待处理文件列表
    let file_buffer = Arc::new(Mutex::new(Vec::new()));
    // 异步方式获取文件列表，写入buffer
    let future_read = read_file_list_and_input_buffer(src_dir, file_buffer.clone());
    // 异步方式读取文件信息，copy文件
    let future_copy = read_file_info_and_copy_file(dst_dir, file_buffer.clone());
    // join
    tokio::join!(future_read, future_copy);
}
