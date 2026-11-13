//! 라이브러리에 폴더·파일을 가져오는 개발용 도구.
//! cargo run -p workout-service --example import -- <라이브러리 폴더> <경로>...
use std::path::PathBuf;
use workout_service::Library;

fn main() {
    let mut args = std::env::args().skip(1);
    let root = PathBuf::from(args.next().expect("라이브러리 폴더"));
    let paths: Vec<PathBuf> = args.map(PathBuf::from).collect();
    let mut lib = Library::open(&root).expect("라이브러리 열기");
    let r = lib.import_paths(&paths).expect("가져오기");
    println!("{}", serde_json::to_string_pretty(&r).unwrap());
}
