use mmt::find_media;
use mmt::read_input;


fn main() {
    match read_input(){
        Ok(path)=>find_media(&path),
        Err(e)=>panic!("{}", e),
    }
}
