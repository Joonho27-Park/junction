use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() != 2 {
        eprintln!("사용법: {} <바이너리_파일>", args[0]);
        eprintln!("예시: {} asdf", args[0]);
        process::exit(1);
    }
    
    let filename = &args[1];
    
    println!("=== 철도 시뮬레이션 바이너리 파일 분석기 ===");
    println!("분석 대상 파일: {}", filename);
    
    println!("\n이 기능은 별도의 binary_analyzer 크레이트에서 제공됩니다.");
    println!("사용법: cd binary_analyzer && cargo run -- --file {}", filename);
    
    process::exit(0);
} 