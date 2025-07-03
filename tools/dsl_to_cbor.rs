use serde::Serialize;
use serde_cbor;
use std::fs::File;
use std::collections::{HashSet, HashMap};

#[derive(Serialize, Hash, PartialEq, Eq, Debug,Clone)]
struct Pt(i32, i32);

#[derive(Serialize)]
struct Model {
    linesegs: HashSet<(Pt, Pt)>,
    gobjects: HashMap<Pt, String>,
    node_data: HashMap<Pt, String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 예시 좌표 (실제 좌표는 sample.cbor 구조에 맞게 조정)
    let a = Pt(0, 0);
    let b = Pt(1, 0);
    let c = Pt(2, 0);
    let d = Pt(3, 0);

    // 1. linesegs: (Pt, Pt) 쌍
    let mut linesegs = HashSet::new();
    linesegs.insert((a.clone(), b.clone()));
    linesegs.insert((b.clone(), c.clone()));
    linesegs.insert((c.clone(), d.clone()));

    // 2. gobjects: Pt → String
    let mut gobjects = HashMap::new();
    gobjects.insert(a.clone(), "Signal".to_string());
    gobjects.insert(c.clone(), "Detector".to_string());

    // 3. node_data: Pt → String
    let mut node_data = HashMap::new();
    node_data.insert(a.clone(), "BufferStop".to_string());
    node_data.insert(d.clone(), "BufferStop".to_string());

    let model = Model {
        linesegs,
        gobjects,
        node_data,
    };

    let mut file = File::create("output.cbor")?;
    serde_cbor::to_writer(&mut file, &model)?;
    println!("CBOR 파일로 저장 완료: output.cbor");
    Ok(())
}