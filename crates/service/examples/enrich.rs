//! 실제 Nominatim·Open-Meteo로 운동 하나의 장소·경로·날씨를 채워 보는 개발용 도구.
//! cargo run -p workout-service --example enrich -- <라이브러리 폴더> <운동 id>
use std::path::PathBuf;
use workout_service::Library;

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let mut lib = Library::open(&PathBuf::from(&a[0])).expect("라이브러리 열기");
    let p = lib.enrich_activity(&a[1]).expect("채우기");
    println!("장소: {:?}", p.place);
    println!("경로: {:?}", p.route_points);
    println!("날씨: {:?}", p.weather);
    println!("오류: {:?}", p.enrich_error);
    println!("---\n{}", p.full_text);
}
