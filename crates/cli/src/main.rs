use clap::{Parser, Subcommand};
use std::path::PathBuf;
use workout_cli::prepare::prepare;

#[derive(Parser)]
#[command(
    name = "workout-cli",
    about = "운동 기록(FIT)과 사진으로 스레드 카드·글을 만든다"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// 폴더의 .fit과 사진으로 <사진>_card.jpg와 post_HHMM.txt를 만든다
    Prepare { folder: PathBuf },
}

fn main() -> anyhow::Result<()> {
    match Cli::parse().cmd {
        Cmd::Prepare { folder } => {
            let r = prepare(&folder)?;
            for (p, e) in &r.skipped_fit {
                eprintln!("경고: FIT 건너뜀 {} ({e})", p.display());
            }
            for p in &r.duplicate_fit {
                eprintln!("경고: 같은 기록이 이미 있어 건너뜀 {}", p.display());
            }
            for (p, e) in &r.failed_cards {
                eprintln!("경고: 카드 생성 실패 {} ({e})", p.display());
            }
            for p in &r.unmatched_photos {
                eprintln!("경고: 운동 시간과 맞지 않는 사진 {}", p.display());
            }
            println!("운동 {}개", r.activities);
            for p in r.cards.iter().chain(&r.posts) {
                println!("저장: {}", p.display());
            }
        }
    }
    Ok(())
}
