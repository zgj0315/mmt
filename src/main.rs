use mmt::find_media;
use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("Please input like this: {} /home/zhaogj/photo", args[0]);
        process::exit(1);
    }
    find_media(&args[1]);
}
