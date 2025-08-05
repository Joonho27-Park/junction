#![allow(dead_code)]

use ordered_float::OrderedFloat;
use crate::model::*;
use std::collections::HashMap;
use log::*;

//
// For converting:
//
//
//
/// 트랙 연결을 나타내는 타입 별칭
/// ((트랙 인덱스, A/B 끝점), (노드 인덱스, 포트))
pub type TopoConnection = ((usize, AB), (usize,Port));

/// 위상학적 구조를 나타내는 메인 구조체
#[derive(Debug)]
pub struct Topological {
    pub tracks :Vec<TopoTrack>,      // <ocsElements> -> <signals>
    pub nodes :Vec<TopoNode>,        // <trackTopology> -> <trackBegin> | <trackEnd> -> <connection> | <bufferStop> | <openEnd> | <macroscopicNode>
                                    //                  -> <connections>             -> <switch> | <crossing> -> <connection>
    pub connections :Vec<TopoConnection>, // 트랙과 노드 간의 연결 관계
}

/// 개별 트랙을 나타내는 구조체
#[derive(Debug)]
pub struct TopoTrack {
    pub objects :Objects,    // <ocsElements> (signals, balises)
    pub length: f64,         // 트랙의 길이 (미터)
    pub offset :f64,         // 트랙의 시작점에서의 오프셋
}

/// 트랙의 끝점을 나타내는 열거형 (A 또는 B)
#[derive(Copy,Clone,PartialEq,Eq,Hash)]
#[derive(Debug)]
pub enum AB { A, B }

impl AB {
    /// 반대 끝점을 반환
    pub fn opposite(&self) -> AB {
        match self {
            AB::A => AB::B,
            AB::B => AB::A,
        }
    }
}

/// 노드의 포트를 나타내는 열거형
#[derive(Debug)]
#[derive(Copy,Clone,PartialEq,Eq,Hash)]
pub enum Port {
    Trunk,              // 메인 라인
    Left,               // 왼쪽 분기
    Right,              // 오른쪽 분기
    Crossing(AB, usize), // crossing (방향, 크로싱 ID)
    Single,             // 단일 연결
    ContA,              // 연속 연결 A
    ContB,              // 연속 연결 B
}

impl Port {
    /// 현재 포트에서 연결 가능한 다른 포트들과 방향을 반환
    pub fn other_ports(&self) -> Vec<(Port,isize)> {
        match self {
            Port::Trunk => vec![(Port::Left,1), (Port::Right,1)],           // 메인에서 양쪽 분기로
            Port::Left => vec![(Port::Right,-1), (Port::Trunk,1)],           // 왼쪽에서 오른쪽과 메인으로
            Port::Right => vec![(Port::Left,-1), (Port::Trunk,1)],           // 오른쪽에서 왼쪽과 메인으로
            Port::Single => vec![],                                          // 단일 연결은 추가 연결 없음
            Port::Crossing(_,_) => unimplemented!(),                         // crossing은 아직 구현되지 않음
            Port::ContA => vec![(Port::ContB,1)],                           // 연속 A에서 B로
            Port::ContB => vec![(Port::ContA,1)],                           // 연속 B에서 A로
        }
    }
}

/// 스위치의 측면을 나타내는 열거형
#[derive(Copy,Clone)]
#[derive(Debug)]
pub enum Side { Left, Right }

impl Side {
    // <switch> | <crossing> @trackContinueCourse
    /// 반대 측면을 반환
    pub fn opposite(&self) -> Self {
        match self {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        }
    }

    /// 측면을 포트로 변환
    pub fn to_port(&self) -> Port {
        match self {
            Side::Left => Port::Left,
            Side::Right => Port::Right,
        }
    }
}

/// 위상학적 노드의 타입을 나타내는 열거형
#[derive(Debug)]
pub enum TopoNode {
    // <trackTopology> -> <trackBegin> | <trackEnd> 의 children 중 하나
    BufferStop,         // 완충 정지점
    OpenEnd,           // 열린 끝
    MacroscopicNode,   // TODO preserve names for boundaries? // 거시적 노드 (경계용)

    // <trackTopology> -> <connections> 의 children 중 하나
    Switch(Side),      // 스위치 (측면 지정)
    Crossing(()),      // TODO crossing type // crossing (타입은 아직 미구현)
    Continuation,      // 연속 연결
}

/// 새로운 노드를 추가하고 인덱스를 반환
pub fn new_node(topo :&mut Topological, node :TopoNode) -> usize {
    let idx = topo.nodes.len();
    topo.nodes.push(node);
    idx
}

/// 새로운 트랙을 추가하고 인덱스를 반환
pub fn new_track(topo :&mut Topological, track :TopoTrack) -> usize {
    let idx = topo.tracks.len();
    topo.tracks.push(track);
    idx
}

/// TrackEndConnection을 TopoNode로 변환
pub fn topo_node_type(n :TrackEndConnection) -> TopoNode {
    match n {
        TrackEndConnection::BufferStop => TopoNode::BufferStop,
        TrackEndConnection::OpenEnd => TopoNode::OpenEnd,
        TrackEndConnection::MacroscopicNode(_) => TopoNode::MacroscopicNode,
        _ => panic!(),
    }
}

/// 위상학적 변환 중 발생할 수 있는 오류들
#[derive(Debug)]
pub enum TopoConvErr {
    SwitchConnectionMissing(String),      // 스위치 연결이 누락됨
    SwitchConnectionTooMany(String),     // 스위치 연결이 너무 많음
    SwitchCourseUnknown(String),         // 스위치 코스가 알 수 없음
    SwitchOrientationInvalid(String),    // 스위치 방향이 유효하지 않음
    UnmatchedConnection(String,String),  // 매칭되지 않는 연결
}

/// 스위치 정보를 담는 구조체
#[derive(Debug)]
pub struct TopoSwitchInfo {
    // <connection>
    connref: (Id,IdRef),        // @id @ref 연결 참조
    deviating_side :Side,       // 분기되는 측면
    switch_geometry :Side,      // @radius > @trackContinueRadius 이면 반대 측면 선택
    dir :AB,                    // @orientation 방향 (A 또는 B)
    pos :f64,                   // @pos 위치 (오프셋)
}

/// 스위치에서 위상학적 정보를 추출
pub fn switch_info(sw :Switch) -> Result<TopoSwitchInfo,TopoConvErr> {
    match sw {
        Switch::Switch { id, pos, connections, track_continue_course, track_continue_radius, .. } => {
            // connections는 model.rs의 SwitchConnection 벡터
            match connections.as_slice() {
                &[] => Err(TopoConvErr::SwitchConnectionMissing(id)), // 연결이 없으면 오류
                &[ref connection] =>  {
                    // 스위치 코스 결정: 연결의 코스 또는 트랙 연속 코스의 반대
                    // connection.course: SwitchConnectionCourse (Left/Right/Straight)
                    // track_continue_course.opposite(): Straight -> None, Left -> Right, Right -> Left
                    let sw_course = connection.course
                        .or(track_continue_course.and_then(|c| c.opposite()))
                        .ok_or(TopoConvErr::SwitchCourseUnknown(id.clone()))?;

                    // 분기되는 측면 결정
                    // to_side(): Left -> Side::Left, Right -> Side::Right, Straight -> None
                    // Straight인 경우 unwrap()에서 패닉 발생 가능성 있음
                    let deviating_side = sw_course.to_side().unwrap();
                    
                    // 스위치 기하학적 측면 결정 (반지름 비교)
                    // connection.radius > track_continue_radius이면 반대 측면 선택
                    let switch_geometry = if connection.radius.unwrap_or(0.0) > 
                                            track_continue_radius.unwrap_or(std::f64::INFINITY) {
                        sw_course.opposite().unwrap().to_side().unwrap()
                    } else { sw_course.to_side().unwrap() };

                    // TopoSwitchInfo 구조체 생성 및 반환
                    Ok(
                        TopoSwitchInfo {
                            connref: (connection.id.clone(), connection.r#ref.clone()), // 연결 참조 정보
                            deviating_side: deviating_side,                             // 분기되는 측면 (Left/Right)
                            switch_geometry: switch_geometry,                           // 스위치 기하학적 측면
                            pos: pos.offset,                                           // 스위치 위치 (오프셋)
                            dir: match connection.orientation { 
                                ConnectionOrientation::Outgoing => AB::A,               // 진출 방향 -> A 끝점
                                ConnectionOrientation::Incoming => AB::B,               // 진입 방향 -> B 끝점
                                _ => { return Err(TopoConvErr::SwitchOrientationInvalid(id.clone())); }, // 기타 방향은 오류
                            },
                        }
                    )
                },
                _ => Err(TopoConvErr::SwitchConnectionTooMany(id)), // 연결이 너무 많으면 오류
            }
        },
        Switch::Crossing { .. } => unimplemented!(), // crossing은 아직 구현되지 않음
    }
}

/// RailML 문서를 위상학적 구조로 변환하는 메인 함수
pub fn convert_railml_topo(doc :RailML) -> Result<Topological,TopoConvErr> {
    // 위상학적 구조 초기화
    let mut topo = Topological {
        tracks: Vec::new(),
        nodes :Vec::new(),
        connections: Vec::new(),
    };

    // 명명된 트랙 포트와 노드 포트를 추적하기 위한 해시맵
    let mut named_track_ports :HashMap<(String,String), (usize, AB)> = HashMap::new();
    let mut named_node_ports  :HashMap<(String,String), (usize, Port)> = HashMap::new();

    // 인프라가 있는 경우 처리
    if let Some(inf) = doc.infrastructure {
        // <infrastructure>
        for mut track in inf.tracks {
            // <tracks> 목록
            // 새로운 트랙 생성
            let mut track_idx = new_track(&mut topo, TopoTrack {
                objects: Objects::empty(),
                offset: 0.0,
                length: 0.0,
            });

            let mut current_offset = 0.0;

            // 트랙 시작점 처리
            track_end(track.begin.connection, (track_idx, AB::A), &mut topo, &mut named_track_ports);
            
            // 스위치들을 위치 순으로 정렬
            track.switches.sort_by_key(|s| match s { 
                Switch::Switch { pos, .. } | Switch::Crossing { pos, .. } => OrderedFloat(pos.offset) });

            // 각 스위치 처리
            for sw in track.switches {
                debug!("Switch info a. {:?} ", sw);
                let sw_info = switch_info(sw)?;
                debug!("Switch info b. {:?}", sw_info);
                
                // 현재 트랙의 길이 설정
                topo.tracks[track_idx].length = sw_info.pos - current_offset;

                // 스위치 노드 생성
                let nd = new_node(&mut topo, TopoNode::Switch(sw_info.switch_geometry));
                named_node_ports.insert(sw_info.connref, (nd, sw_info.deviating_side.to_port()));
                
                // 포트 할당 (방향에 따라 A/B 포트 결정)
                let (mut a_port, mut b_port) = (Port::Trunk, sw_info.deviating_side.opposite().to_port());
                if sw_info.dir == AB::B { std::mem::swap(&mut a_port, &mut b_port); }

                // 현재 트랙을 스위치에 연결
                topo.connections.push(((track_idx,AB::B), (nd, a_port)));
                
                // 새로운 트랙 생성 (스위치 이후)
                track_idx = new_track(&mut topo, TopoTrack {
                    objects: Objects::empty(),
                    offset: sw_info.pos,
                    length: 0.0
                });
                
                // 새 트랙을 스위치에 연결
                topo.connections.push(((track_idx,AB::A), (nd, b_port)));
                current_offset = sw_info.pos;
            }

            // 트랙 끝점 처리
            track_end(track.end.connection, (track_idx, AB::B), &mut topo, &mut named_track_ports);
            topo.tracks[track_idx].length = track.end.pos.offset - current_offset;
        }
    }

    // Match track ports with node ports.
    // 명명된 트랙 포트와 노드 포트 매칭
    println!("now matching named node track ports");
    println!("node ports {:?}", named_node_ports);
    println!("track ports {:?}", named_track_ports);

    // 노드 포트와 트랙 포트 연결
    for ((c,r),nd_port) in named_node_ports {
        let x = (r,c);
        let tr_port = named_track_ports.remove(&x)
            .ok_or(TopoConvErr::UnmatchedConnection(x.1,x.0))?;
        topo.connections.push((tr_port,nd_port));
    }

    // TODO track contiunations,i .e. connetions track->track.
    // 트랙 연속 연결 처리 (트랙->트랙 연결)
    while named_track_ports.len() > 0 {
        let key = named_track_ports.keys().next().unwrap().clone();
        let ((c1,c2),(t1_idx,ab1)) = named_track_ports.remove_entry(&key).unwrap();
        let (t2_idx,ab2) = named_track_ports.remove(&(c2.clone(),c1.clone()))
            .ok_or(TopoConvErr::UnmatchedConnection(c1,c2))?;

        // 연속 노드 생성
        let n = new_node(&mut topo, TopoNode::Continuation);
        topo.connections.push(((t1_idx,ab1),(n,Port::ContA)));
        topo.connections.push(((t2_idx,ab2),(n,Port::ContB)));
    }

    // 디버그 출력
    debug!("CONNECTIONS {:?}", topo.connections);
    for c in &topo.connections {
        debug!("{:?}", c);
    }

    Ok(topo)
}

/// 트랙 끝점을 처리하는 함수
pub fn track_end(conn :TrackEndConnection, 
                 (track_idx,side) :(usize,AB),
                 topo :&mut Topological,
                 named_track_ports :&mut HashMap<(String,String),(usize,AB)>) {
    match conn {
        // BufferStop, OpenEnd, MacroscopicNode 인 경우
        n @ TrackEndConnection::BufferStop | 
        n @ TrackEndConnection::OpenEnd |
        n @ TrackEndConnection::MacroscopicNode(_) => {
            let nd = new_node(topo, topo_node_type(n));
            topo.connections.push(((track_idx,side),(nd, Port::Single)));
        },
        // 연결인 경우 명명된 포트로 등록
        TrackEndConnection::Connection(from,to) => {
            named_track_ports.insert((from,to),(track_idx, side));
        },
    };
}

















