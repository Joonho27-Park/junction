#![allow(dead_code)]


use crate::topo::Side;



//
//
//
// original railml model
//
//
//


pub type Id = String;
pub type IdRef = String;

#[derive(Debug)]
pub struct RailML {
    pub infrastructure :Option<Infrastructure>,
    // 전체 railML 문서의 루트 구조.
    // 여기서는 <infrastructure> 요소를 가질 수 있으며, 없을 수도 있음 (Option)
}

#[derive(Debug)]
pub struct Infrastructure {
    pub tracks :Vec<Track>, // <infrastructure> 안에 포함되는 <tracks> 목록
}

#[derive(Debug)]
pub struct Track {
    pub id :Id,                   // <track> @id
    pub code :Option<String>,     // <track> @code
    pub name :Option<String>,     // <track> @name
    pub description :Option<String>, // <track> @description
    pub begin: Node,               // <track> <trackBegin>
    pub end :Node,                 // <track> <trackEnd>
    pub switches :Vec<Switch>,     // <track> <switches> -> <switch> | <crossing>
    pub objects :Objects,          // <track> <ocsElements>
}


#[derive(Debug)]
pub struct Node {
    pub id :Id,                       // <trackBegin> | <trackEnd> @id
    pub pos :Position,                 // <trackBegin> | <trackEnd> @pos
    pub connection :TrackEndConnection // <trackBegin> | <trackEnd> -> <connection> | <bufferStop> | <openEnd> | <macroscopicNode> 4개 중 하나
}

#[derive(Debug)]
pub enum TrackEndConnection {
    // Element rail:eTrackNode / 문서 참조
    // TODO: 필수 속성 추가 필요
    Connection(Id,IdRef),           // <connection> @id @ref
    BufferStop,                     // <bufferStop> @id
    OpenEnd,                        // <openEnd> @id
    MacroscopicNode(String),        // <macroscopicNode> @ocpRef
}

#[derive(Debug)]
pub enum Switch {
    // <trackTopology>
    Switch {
        // <connections> -> <switch>
        id :Id,                                                  // <switch> @id
        pos :Position,                                           // <switch> @pos
        name :Option<String>,                                    // <switch> @name
        description :Option<String>,                             // <switch> @description
        length: Option<f64>,                                     // <switch> @length
        connections :Vec<SwitchConnection>,                      // <switch> <connection>
        track_continue_course :Option<SwitchConnectionCourse>,   // <switch> @trackContinueCourse
        track_continue_radius :Option<f64>,                      // <switch> @trackContinueRadius
    },
    Crossing {
        // <connections> -> <crossing>
        id :Id,                                                 // <crossing> @id
        pos :Position,                                          // <crossing> @pos

        track_continue_course :Option<SwitchConnectionCourse>, // <crossing> @trackContinueCourse
        track_continue_radius :Option<f64>,                    // <crossing> @trackContinueRadius
        normal_position :Option<SwitchConnectionCourse>,       // <crossing> @normalPosition

        length: Option<f64>,                                    // <crossing> @length
        connections: Vec<SwitchConnection>,                     // <crossing> <connection>
    },
}

#[derive(Copy,Clone)]
#[derive(Debug)]
pub enum SwitchConnectionCourse { 
    Straight,  // 직진
    Left,      // 좌측 분기
    Right      // 우측 분기
}

impl SwitchConnectionCourse {
    pub fn opposite(&self) -> Option<SwitchConnectionCourse> {
        // 현재 분기 방향의 반대 방향 반환
        match self {
            SwitchConnectionCourse::Left => Some(SwitchConnectionCourse::Right),
            SwitchConnectionCourse::Right => Some(SwitchConnectionCourse::Left),
            _ => None,
        }
    }

    // model.rs에서 topo.rs로 변환하는 메서드
    pub fn to_side(&self) -> Option<Side> {
        // SwitchConnectionCourse -> Side 변환
        match self {
            SwitchConnectionCourse::Left => Some(Side::Left),
            SwitchConnectionCourse::Right => Some(Side::Right),
            _ => None,
        }
    }
}


#[derive(Debug)]
pub enum ConnectionOrientation { 
    Incoming,     // 진입 방향
    Outgoing,     // 진출 방향
    RightAngled,  // 직각 연결
    Unknown,      // 방향 정보 없음
    Other         // 기타
}

#[derive(Debug)]
pub struct SwitchConnection {
    // <connection> 속성 정의
    pub id :Id,                                  // <connection> @id
    pub r#ref :IdRef,                            // <connection> @ref
    pub orientation :ConnectionOrientation,      // <connection> @orientation
    pub course :Option<SwitchConnectionCourse>,  // <connection> @course
    pub radius :Option<f64>,                     // <connection> @radius
    pub max_speed :Option<f64>,                  // <connection> @maxSpeed
    pub passable :Option<bool>,                  // <connection> @passable
}

#[derive(Debug)]
pub struct Position {
    pub offset :f64,              // 위치 오프셋
    pub mileage :Option<f64>,     // 마일리지(선로상의 절대 위치)
}

#[derive(Debug)]
pub struct Objects {
    // <ocsElements>
    pub signals: Vec<Signal>,  // <signals> -> <signal>
    pub balises: Vec<Balise>,  // <balises> -> <balise> or <baliseGroup>
    // TODO: Detector 추가 필요
    // <trainDetectionElements> -> <trainDetector>
}

impl Objects {
    pub fn empty() -> Objects {
        // 비어 있는 Objects 생성
        Objects {
            signals :Vec::new(),
            balises :Vec::new(),
        }
    }
}

#[derive(Debug)]
pub struct Signal {
    id: Id,                      // <signal> @id
    pos: Position,               // <signal> @pos 위치 정보 (offset/mileage)
    name: Option<String>,        // <signal> @name
    dir: TrackDirection,         // <signal> @dir (Up/Down)
    sight: Option<f64>,          // <signal> @sight
    r#type: SignalType,          // <signal> @type (Main/Distant/...)
}

#[derive(Debug)]
pub enum SignalType { 
    Main,        // type="main"
    Distant,     // type="distant"
    Repeater,    // type="repeater"
    Combined,    // type="combined"
    Shunting     // type="shunting"
}
#[derive(Debug)]
pub enum SignalFunction { 
    Exit,        // function="exit"
    Home,        // function="home"
    Blocking,    // function="blocking"
    Intermediate // function="intermediate"
}
#[derive(Debug)]
pub enum TrackDirection { 
    Up,   // dir="up"
    Down  // dir="down"
}
#[derive(Debug)]
pub struct Balise {
}


