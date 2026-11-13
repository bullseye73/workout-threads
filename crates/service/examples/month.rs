//! 라이브러리의 월간 요약을 JSON으로 출력하는 개발용 도구.
//! cargo run -p workout-service --example month -- <라이브러리 폴더> <연> <월>
use std::path::PathBuf;
use workout_service::Library;

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let lib = Library::open(&PathBuf::from(&a[0])).expect("라이브러리 열기");
    let v = lib
        .month_view(a[1].parse().unwrap(), a[2].parse().unwrap())
        .expect("월간 요약");
    println!(
        "{} km, 라이딩 {}회, 날짜 {:?}",
        v.total_km,
        v.cycling.count,
        v.days.iter().map(|d| &d.date).collect::<Vec<_>>()
    );
}
