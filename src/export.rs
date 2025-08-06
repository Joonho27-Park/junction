use std::collections::{HashMap, HashSet};
use log::*;
use const_cstr::const_cstr;
use crate::document::model::*;
use crate::document::objects::*;
use crate::app::*;
use crate::gui::widgets;
use std::sync::mpsc;
use std::fs;
use std::fmt;
use nalgebra_glm as glm;

// RailML 모델 import
use railmlio::model::*;

// ============================================================================
// 오류 처리
// ============================================================================

#[derive(Debug)]
pub enum ExportError {
    TrackAnalysisError(String),
    ObjectMappingError(String),
    RailMLGenerationError(String),
    XMLSerializationError(String),
    MissingRequiredData(String),
}

impl fmt::Display for ExportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExportError::TrackAnalysisError(msg) => write!(f, "Track analysis error: {}", msg),
            ExportError::ObjectMappingError(msg) => write!(f, "Object mapping error: {}", msg),
            ExportError::RailMLGenerationError(msg) => write!(f, "RailML generation error: {}", msg),
            ExportError::XMLSerializationError(msg) => write!(f, "XML serialization error: {}", msg),
            ExportError::MissingRequiredData(msg) => write!(f, "Missing required data: {}", msg),
        }
    }
}

// ============================================================================
// 트랙 분석 데이터 구조
// ============================================================================

#[derive(Debug)]
pub struct TrackAnalysis {
    pub segments: Vec<TrackSegment>,
    pub nodes: Vec<TrackNode>,
    pub switches: Vec<SwitchAnalysis>,
    pub signals: Vec<SignalInfo>,
    pub detectors: Vec<DetectorInfo>,
}

#[derive(Debug)]
pub struct TrackSegment {
    pub start: Pt,
    pub end: Pt,
    pub length: f64,
}

#[derive(Debug)]
pub struct TrackNode {
    pub position: Pt,
    pub node_type: NDType,
    pub connections: Vec<Pt>,
}

#[derive(Debug)]
pub struct SwitchAnalysis {
    pub position: Pt,
    pub node_side: Side,
    pub node_state: SwitchState,
    pub object_info: Option<SwitchObjectInfo>,
}

#[derive(Debug)]
pub struct SwitchObjectInfo {
    pub id: Option<String>,
    pub switch_type: Option<SwitchType>,
    pub direction: Option<SwitchDirection>,
}

#[derive(Debug)]
pub struct SignalInfo {
    pub position: Pt,
    pub id: Option<String>,
    pub signal_type: crate::document::objects::SignalType,
    pub signal_kind: crate::document::objects::SignalKind,
    pub direction: crate::document::objects::TrackDirection,
    pub has_distant: bool,
}

#[derive(Debug)]
pub struct DetectorInfo {
    pub position: Pt,
    pub id: Option<String>,
}

// ============================================================================
// 트랙 구조 분석
// ============================================================================

fn analyze_track_structure(model: &Model) -> Result<TrackAnalysis, ExportError> {
    println!("  Analyzing line connections...");
    // 1. linesegs에서 연결 관계 분석
    let connections = analyze_line_connections(&model.linesegs);
    println!("    Found {} connection points", connections.len());
    
    println!("  Identifying nodes...");
    // 2. 노드 식별 (끝점, 분기점, 연결점)
    let nodes = identify_nodes(&connections, &model.node_data);
    println!("    Identified {} nodes", nodes.len());
    
    println!("  Creating track segments...");
    // 3. 트랙 세그먼트 생성
    let segments = create_track_segments(&model.linesegs, &nodes);
    println!("    Created {} track segments", segments.len());
    
    // 4. 스위치, 신호, 검지기 정보 추출
    let switches = analyze_switches(model);
    let signals = extract_signal_info(&model.objects);
    let detectors = extract_detector_info(&model.objects);
    
    Ok(TrackAnalysis { segments, nodes, switches, signals, detectors })
}

fn analyze_line_connections(linesegs: &im::HashSet<(Pt, Pt)>) -> HashMap<Pt, Vec<Pt>> {
    let mut connections = HashMap::new();
    
    for (start, end) in linesegs {
        connections.entry(*start).or_insert_with(Vec::new).push(*end);
        connections.entry(*end).or_insert_with(Vec::new).push(*start);
    }
    
    connections
}

fn identify_nodes(connections: &HashMap<Pt, Vec<Pt>>, node_data: &im::HashMap<Pt, NDType>) -> Vec<TrackNode> {
    let mut nodes = Vec::new();
    
    for (pos, connected_pts) in connections {
        let node_type = node_data.get(pos).cloned().unwrap_or_else(|| {
            // 연결 개수에 따라 기본 노드 타입 결정
            match connected_pts.len() {
                1 => NDType::OpenEnd,
                2 => NDType::Cont,
                _ => NDType::Err,
            }
        });
        
        nodes.push(TrackNode {
            position: *pos,
            node_type,
            connections: connected_pts.clone(),
        });
    }
    
    nodes
}

fn create_track_segments(linesegs: &im::HashSet<(Pt, Pt)>, nodes: &[TrackNode]) -> Vec<TrackSegment> {
    let mut segments = Vec::new();
    
    for (start, end) in linesegs {
        let length = calculate_segment_length(start, end);
        segments.push(TrackSegment {
            start: *start,
            end: *end,
            length,
        });
    }
    
    segments
}

fn calculate_segment_length(start: &Pt, end: &Pt) -> f64 {
    let dx = (end.x - start.x) as f64;
    let dy = (end.y - start.y) as f64;
    (dx * dx + dy * dy).sqrt() * 10.0 // 스케일 팩터 적용
}

fn analyze_switches(model: &Model) -> Vec<SwitchAnalysis> {
    println!("    Analyzing switches from node_data...");
    let mut switches = Vec::new();
    
    // 1. node_data에서 NDType::Sw 노드들 찾기
    for (pos, ndtype) in &model.node_data {
        if let NDType::Sw(side, state) = ndtype {
            println!("      Found switch at {:?} with side={:?}, state={:?}", pos, side, state);
            // 2. 해당 위치에 Function::Switch 객체가 있는지 확인
            let mut object_info = find_switch_object_at_position(model, pos);
            if object_info.is_some() {
                println!("        Found matching switch object");
            } else {
                println!("        No matching switch object found");
            }
            
            switches.push(SwitchAnalysis {
                position: *pos,
                node_side: *side,
                node_state: *state,
                object_info,
            });
        }
    }
    
    // 3. 스위치들을 위치 순으로 정렬하고 순차적으로 ID 할당
    switches.sort_by(|a, b| {
        a.position.x.cmp(&b.position.x).then(a.position.y.cmp(&b.position.y))
    });
    
    for (index, switch) in switches.iter_mut().enumerate() {
        // 객체 ID가 없으면 순차적으로 할당
        if switch.object_info.is_none() || switch.object_info.as_ref().unwrap().id.is_none() {
            let switch_id = format!("sw{}", index + 1);
            switch.object_info = Some(SwitchObjectInfo {
                id: Some(switch_id.clone()),
                switch_type: None,
                direction: None,
            });
            println!("        Assigned ID {} to switch at {:?}", switch_id, switch.position);
        }
    }
    
    println!("    Total switches found: {}", switches.len());
    switches
}

fn find_switch_object_at_position(model: &Model, pos: &Pt) -> Option<SwitchObjectInfo> {
    let pos_f = glm::vec2(pos.x as f32, pos.y as f32);
    
    for (obj_pos, obj) in &model.objects {
        let obj_pos_f = glm::vec2(obj_pos.x as f32, obj_pos.y as f32);
        let distance = glm::distance(&pos_f, &obj_pos_f);
        
        // 스위치 객체가 해당 위치 근처에 있는지 확인
        if distance <= 1.5 {
            for function in &obj.functions {
                if let Function::Switch { id } = function {
                    return Some(SwitchObjectInfo {
                        id: id.clone(),
                        switch_type: obj.switch_props.as_ref().map(|p| p.switch_type.clone()),
                        direction: obj.switch_props.as_ref().map(|p| p.direction.clone()),
                    });
                }
            }
        }
    }
    
    None
}

fn extract_signal_info(objects: &im::HashMap<PtA, Object>) -> Vec<SignalInfo> {
    println!("    Extracting signal information...");
    let mut signals = Vec::new();
    
    for (pos, obj) in objects {
        for function in &obj.functions {
            if let Function::Signal { has_distant, id } = function {
                println!("      Found signal at {:?} with id={:?}, has_distant={}", pos, id, has_distant);
                let signal_props = obj.signal_props.as_ref().unwrap_or(&SignalProperties {
                                    signal_type: crate::document::objects::SignalType::Home,
                signal_kind: crate::document::objects::SignalKind::Two,
                direction: crate::document::objects::TrackDirection::Right,
                });
                
                signals.push(SignalInfo {
                    position: glm::vec2(pos.x, pos.y),
                    id: id.clone(),
                    signal_type: signal_props.signal_type.clone(),
                    signal_kind: signal_props.signal_kind.clone(),
                    direction: signal_props.direction,
                    has_distant: *has_distant,
                });
            }
        }
    }
    
    println!("    Total signals found: {}", signals.len());
    signals
}

fn extract_detector_info(objects: &im::HashMap<PtA, Object>) -> Vec<DetectorInfo> {
    println!("    Extracting detector information...");
    let mut detectors = Vec::new();
    
    for (pos, obj) in objects {
        for function in &obj.functions {
            if let Function::Detector = function {
                println!("      Found detector at {:?} with id={:?}", pos, obj.id);
                detectors.push(DetectorInfo {
                    position: glm::vec2(pos.x, pos.y),
                    id: obj.id.clone(),
                });
            }
        }
    }
    
    println!("    Total detectors found: {}", detectors.len());
    detectors
}

// ============================================================================
// RailML 모델 생성
// ============================================================================

fn create_railml_model(
    analysis: &TrackAnalysis,
    switches: &[SwitchAnalysis],
    signals: &[SignalInfo],
    detectors: &[DetectorInfo]
) -> Result<RailML, ExportError> {
    println!("  Creating RailML model...");
    
    // 1. 메인 트랙 생성
    println!("    Creating main track...");
    let main_track = create_main_track(analysis)?;
    println!("    Main track created successfully");
    
    // 2. 분기 트랙들 생성
    println!("    Creating branch tracks...");
    let branch_tracks = create_branch_tracks(analysis)?;
    println!("    Created {} branch tracks", branch_tracks.len());
    
    // 3. RailML 구조체 생성
    let mut all_tracks = vec![main_track];
    all_tracks.extend(branch_tracks);
    println!("    Total tracks: {}", all_tracks.len());
    
    Ok(RailML {
        infrastructure: Some(Infrastructure { 
            tracks: all_tracks 
        }),
    })
}

fn create_main_track(analysis: &TrackAnalysis) -> Result<Track, ExportError> {
    // 1. 트랙 시작점과 끝점 찾기
    let (begin_node, end_node) = find_track_endpoints(&analysis.nodes)?;
    
    // 2. 스위치들을 위치 순으로 정렬하고 RailML Switch로 변환
    let mut railml_switches = Vec::new();
    for switch_analysis in &analysis.switches {
        railml_switches.push(convert_to_railml_switch(switch_analysis, &analysis.nodes));
    }
    
    // 3. 객체들을 RailML 구조로 변환
    let objects = convert_to_railml_objects(&analysis.signals, &analysis.detectors)?;
    
    // 4. Track 구조체 생성
    Ok(Track {
        id: "track1".to_string(),
        code: Some("SP1".to_string()),
        name: Some("Main Track".to_string()),
        description: None,
        begin: begin_node,
        end: end_node,
        switches: railml_switches,
        objects,
    })
}

fn create_branch_tracks(analysis: &TrackAnalysis) -> Result<Vec<Track>, ExportError> {
    let mut branch_tracks = Vec::new();
    
    // 스위치가 있는 경우 분기 트랙 생성
    if !analysis.switches.is_empty() {
        // 실제 스위치 ID들을 수집 (이미 analyze_switches에서 할당됨)
        let switch_ids: Vec<String> = analysis.switches.iter()
            .map(|switch| {
                switch.object_info.as_ref()
                    .and_then(|obj| obj.id.as_ref())
                    .cloned()
                    .unwrap_or_else(|| {
                        // fallback: 위치 기반 ID
                        format!("sw_{}_{}", switch.position.x, switch.position.y)
                    })
            })
            .collect();
        
        // 첫 번째 스위치를 시작점, 마지막 스위치를 끝점으로 사용
        let default_begin = "sw1".to_string();
        let default_end = "sw2".to_string();
        let begin_switch_id = switch_ids.first().unwrap_or(&default_begin);
        let end_switch_id = switch_ids.last().unwrap_or(&default_end);
        
        let branch_track = Track {
            id: "track2".to_string(),
            code: Some("SP2".to_string()),
            name: Some("Branch Track".to_string()),
            description: None,
            begin: Node {
                id: "tb2".to_string(),
                pos: Position { offset: 0.0, mileage: None },
                connection: TrackEndConnection::Connection("tb2c".to_string(), format!("{}c", begin_switch_id)),
            },
            end: Node {
                id: "te2".to_string(),
                pos: Position { offset: 500.0, mileage: None },
                connection: TrackEndConnection::Connection("te2c".to_string(), format!("{}c", end_switch_id)),
            },
            switches: Vec::new(),
            objects: Objects::empty(),
        };
        branch_tracks.push(branch_track);
    }
    
    Ok(branch_tracks)
}

fn convert_to_railml_switch(switch_analysis: &SwitchAnalysis, track_nodes: &[TrackNode]) -> Switch {
    // 1. 스위치 ID 결정 (이미 analyze_switches에서 할당됨)
    let switch_id = switch_analysis.object_info.as_ref()
        .and_then(|obj| obj.id.as_ref())
        .cloned()
        .unwrap_or_else(|| {
            // fallback: 위치 기반 ID
            format!("sw_{}_{}", switch_analysis.position.x, switch_analysis.position.y)
        });
    
    // 2. 스위치 위치 계산
    let switch_pos = Position {
        offset: calculate_position_on_track(&switch_analysis.position, &switch_analysis.position, 1000.0),
        mileage: None,
    };
    
    // 3. 스위치 연결 정보 생성 (트랙 구조 분석 기반)
    let connections = create_switch_connections(switch_analysis, track_nodes, &switch_id);
    
    // 4. track_continue_course 결정 (node_state 기반)
    let track_continue_course = match switch_analysis.node_state {
        SwitchState::Straight => Some(SwitchConnectionCourse::Straight),
        SwitchState::Diverging => Some(match switch_analysis.node_side {
            Side::Left => SwitchConnectionCourse::Left,
            Side::Right => SwitchConnectionCourse::Right,
        }),
    };
    
    // 5. RailML Switch 구조체 생성
    Switch::Switch {
        id: switch_id,
        pos: switch_pos,
        name: None,
        description: None,
        length: None,
        connections,
        track_continue_course,
        track_continue_radius: None,
    }
}

fn create_switch_connections(switch_analysis: &SwitchAnalysis, track_nodes: &[TrackNode], switch_id: &str) -> Vec<SwitchConnection> {
    // RailML 표준에 따라 스위치당 1개의 connection만 생성
    // orientation과 course는 스위치의 분기 방향에 따라 결정
    
    let connection_id = format!("{}c", switch_id);
    
    // 스위치의 분기 방향에 따라 course 결정
    let course = match switch_analysis.node_side {
        Side::Left => SwitchConnectionCourse::Left,
        Side::Right => SwitchConnectionCourse::Right,
    };
    
    // orientation은 스위치의 분기 방향에 따라 결정
    // Left 스위치: outgoing (분기로 나가는 방향)
    // Right 스위치: incoming (분기에서 들어오는 방향)
    let orientation = match switch_analysis.node_side {
        Side::Left => ConnectionOrientation::Outgoing,
        Side::Right => ConnectionOrientation::Incoming,
    };
    
    // track reference는 분기 방향에 따라 결정
    let track_ref = match switch_analysis.node_side {
        Side::Left => "tb2c".to_string(),  // 분기 트랙의 시작점
        Side::Right => "te2c".to_string(), // 분기 트랙의 끝점
    };
    
    println!("      Creating switch connection: id={}, ref={}, course={:?}, orientation={:?}", 
             connection_id, track_ref, course, orientation);
    
    vec![SwitchConnection {
        id: connection_id,
        r#ref: track_ref,
        orientation,
        course: Some(course),
        radius: None,
        max_speed: None,
        passable: Some(true),
    }]
}

fn find_track_endpoints(nodes: &[TrackNode]) -> Result<(Node, Node), ExportError> {
    let mut endpoints = Vec::new();
    
    for node in nodes {
        if matches!(node.node_type, NDType::OpenEnd) {
            endpoints.push(node);
        }
    }
    
    if endpoints.len() < 2 {
        return Err(ExportError::MissingRequiredData("Need at least 2 endpoints for track".to_string()));
    }
    
    // 첫 번째와 마지막 엔드포인트를 시작점과 끝점으로 사용
    let begin_node = Node {
        id: "tb1".to_string(),
        pos: Position { offset: 0.0, mileage: None },
        connection: TrackEndConnection::OpenEnd,
    };
    
    let end_node = Node {
        id: "te1".to_string(),
        pos: Position { offset: 1000.0, mileage: None }, // 임시 길이
        connection: TrackEndConnection::OpenEnd,
    };
    
    Ok((begin_node, end_node))
}

fn convert_to_railml_objects(signals: &[SignalInfo], detectors: &[DetectorInfo]) -> Result<Objects, ExportError> {
    let mut railml_signals = Vec::new();
    
    for signal in signals {
        railml_signals.push(Signal {
            id: signal.id.clone().unwrap_or_else(|| format!("sig_{}_{}", signal.position.x, signal.position.y)),
            pos: Position {
                offset: calculate_position_on_track(&signal.position, &signal.position, 1000.0),
                mileage: None,
            },
            name: None,
            dir: match signal.direction {
                crate::document::objects::TrackDirection::Left => railmlio::model::TrackDirection::Up,
                crate::document::objects::TrackDirection::Right => railmlio::model::TrackDirection::Down,
            },
            sight: None,
            r#type: match signal.signal_type {
                crate::document::objects::SignalType::Home => railmlio::model::SignalType::Main,
                crate::document::objects::SignalType::Departure => railmlio::model::SignalType::Main,
                crate::document::objects::SignalType::Shunting => railmlio::model::SignalType::Shunting,
            },
        });
    }
    
    Ok(Objects {
        signals: railml_signals,
        balises: Vec::new(), // TODO: Detector를 Balise로 변환하거나 별도 구조 추가
    })
}

fn calculate_position_on_track(pos: &Pt, _linesegs: &Pt, track_length: f64) -> f64 {
    // 간단한 위치 계산 (실제로는 linesegs를 기반으로 정확한 위치 계산 필요)
    let base_pos = (pos.x as f64).abs() + (pos.y as f64).abs();
    (base_pos * 10.0).min(track_length)
}

// ============================================================================
// XML 생성
// ============================================================================

fn generate_railml_xml(railml: &RailML) -> Result<String, ExportError> {
    println!("  Generating RailML XML...");
    let mut xml = String::new();
    
    // 1. XML 헤더와 네임스페이스 추가
    xml.push_str("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n");
    xml.push_str("<railml xmlns:xsd=\"http://www.w3.org/2001/XMLSchema\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" xmlns=\"http://www.railml.org/schemas/2013\">\n");
    
    // 2. infrastructure 섹션 생성
    if let Some(infrastructure) = &railml.infrastructure {
        println!("    Serializing {} tracks", infrastructure.tracks.len());
        xml.push_str("  <infrastructure>\n");
        xml.push_str("    <tracks>\n");
        
        // 3. 각 track을 XML로 직렬화
        for (i, track) in infrastructure.tracks.iter().enumerate() {
            println!("      Serializing track {}: {}", i, track.id);
            xml.push_str(&serialize_track(track)?);
        }
        
        xml.push_str("    </tracks>\n");
        xml.push_str("  </infrastructure>\n");
    }
    
    // 4. XML 닫기 태그 추가
    xml.push_str("</railml>\n");
    
    println!("    XML generation completed, size: {} bytes", xml.len());
    Ok(xml)
}

fn serialize_track(track: &Track) -> Result<String, ExportError> {
    let mut xml = String::new();
    
    // 1. track 태그와 속성들 생성
    xml.push_str(&format!("      <track id=\"{}\"", track.id));
    if let Some(code) = &track.code {
        xml.push_str(&format!(" code=\"{}\"", code));
    }
    if let Some(name) = &track.name {
        xml.push_str(&format!(" name=\"{}\"", name));
    }
    if let Some(description) = &track.description {
        xml.push_str(&format!(" description=\"{}\"", description));
    }
    xml.push_str(">\n");
    
    // 2. trackTopology 섹션 생성
    xml.push_str("        <trackTopology>\n");
    
    // 3. trackBegin/trackEnd 직렬화
    xml.push_str(&serialize_node(&track.begin, "trackBegin")?);
    xml.push_str(&serialize_node(&track.end, "trackEnd")?);
    
    // 4. connections (switches) 직렬화
    if !track.switches.is_empty() {
        xml.push_str("          <connections>\n");
        for switch in &track.switches {
            xml.push_str(&serialize_switch(switch)?);
        }
        xml.push_str("          </connections>\n");
    }
    
    xml.push_str("        </trackTopology>\n");
    
    // 5. ocsElements (signals, detectors) 직렬화
    xml.push_str(&serialize_objects(&track.objects)?);
    
    xml.push_str("      </track>\n");
    
    Ok(xml)
}

fn serialize_node(node: &Node, node_type: &str) -> Result<String, ExportError> {
    let mut xml = String::new();
    
    // 1. node_type 태그 생성 (trackBegin/trackEnd)
    xml.push_str(&format!("          <{} id=\"{}\" pos=\"{}\">\n", node_type, node.id, node.pos.offset));
    
    // 2. connection/bufferStop/openEnd 등 하위 요소 직렬화
    match &node.connection {
        TrackEndConnection::Connection(id, r#ref) => {
            xml.push_str(&format!("            <connection id=\"{}\" ref=\"{}\" />\n", id, r#ref));
        },
        TrackEndConnection::BufferStop => {
            xml.push_str(&format!("            <bufferStop id=\"{}\" />\n", node.id));
        },
        TrackEndConnection::OpenEnd => {
            xml.push_str(&format!("            <openEnd id=\"{}\" />\n", node.id));
        },
        TrackEndConnection::MacroscopicNode(ocp_ref) => {
            xml.push_str(&format!("            <macroscopicNode ocpRef=\"{}\" />\n", ocp_ref));
        },
    }
    
    xml.push_str(&format!("          </{}>\n", node_type));
    
    Ok(xml)
}

fn serialize_switch(switch: &Switch) -> Result<String, ExportError> {
    let mut xml = String::new();
    
    match switch {
        Switch::Switch { id, pos, name, description, length, connections, track_continue_course, track_continue_radius } => {
            // 1. switch 태그와 속성들 생성
            xml.push_str(&format!("            <switch id=\"{}\" pos=\"{}\"", id, pos.offset));
            if let Some(name) = name {
                xml.push_str(&format!(" name=\"{}\"", name));
            }
            if let Some(description) = description {
                xml.push_str(&format!(" description=\"{}\"", description));
            }
            if let Some(length) = length {
                xml.push_str(&format!(" length=\"{}\"", length));
            }
            if let Some(course) = track_continue_course {
                xml.push_str(&format!(" trackContinueCourse=\"{}\"", course));
            }
            if let Some(radius) = track_continue_radius {
                xml.push_str(&format!(" trackContinueRadius=\"{}\"", radius));
            }
            xml.push_str(">\n");
            
            // 2. connection 요소들 직렬화
            for connection in connections {
                xml.push_str(&format!("              <connection id=\"{}\" ref=\"{}\"", connection.id, connection.r#ref));
                xml.push_str(&format!(" orientation=\"{}\"", connection.orientation));
                if let Some(course) = &connection.course {
                    xml.push_str(&format!(" course=\"{}\"", course));
                }
                if let Some(radius) = connection.radius {
                    xml.push_str(&format!(" radius=\"{}\"", radius));
                }
                if let Some(max_speed) = connection.max_speed {
                    xml.push_str(&format!(" maxSpeed=\"{}\"", max_speed));
                }
                if let Some(passable) = connection.passable {
                    xml.push_str(&format!(" passable=\"{}\"", passable));
                }
                xml.push_str(" />\n");
            }
            
            xml.push_str("            </switch>\n");
        },
        Switch::Crossing { .. } => {
            // TODO: Crossing 직렬화 구현
            xml.push_str("            <!-- Crossing not implemented yet -->\n");
        },
    }
    
    Ok(xml)
}

fn serialize_objects(objects: &Objects) -> Result<String, ExportError> {
    let mut xml = String::new();
    
    // 1. ocsElements 태그 생성
    xml.push_str("        <ocsElements>\n");
    
    // 2. signals 섹션 생성 및 직렬화
    if !objects.signals.is_empty() {
        xml.push_str("          <signals>\n");
        for signal in &objects.signals {
            xml.push_str(&format!("            <signal id=\"{}\" pos=\"{}\"", signal.id, signal.pos.offset));
            if let Some(name) = &signal.name {
                xml.push_str(&format!(" name=\"{}\"", name));
            }
            xml.push_str(&format!(" dir=\"{}\"", signal.dir));
            if let Some(sight) = signal.sight {
                xml.push_str(&format!(" sight=\"{}\"", sight));
            }
            xml.push_str(&format!(" type=\"{}\"", signal.r#type));
            xml.push_str(" />\n");
        }
        xml.push_str("          </signals>\n");
    }
    
    // 3. trainDetectionElements 섹션 생성 및 직렬화
    if !objects.balises.is_empty() {
        xml.push_str("          <trainDetectionElements>\n");
        // TODO: Balise를 trainDetector로 변환하거나 별도 처리
        xml.push_str("            <!-- Train detectors not implemented yet -->\n");
        xml.push_str("          </trainDetectionElements>\n");
    }
    
    xml.push_str("        </ocsElements>\n");
    
    Ok(xml)
}

// ============================================================================
// 메인 변환 함수
// ============================================================================

pub fn convert_model_to_railml(model: &Model) -> Result<String, ExportError> {
    println!("Starting RailML export...");
    println!("Model data: linesegs={}, objects={}, node_data={}", 
             model.linesegs.len(), model.objects.len(), model.node_data.len());
    
    // 1. 트랙 구조 분석
    println!("Analyzing track structure...");
    let track_structure = analyze_track_structure(model)?;
    println!("Track analysis completed: {} segments, {} nodes", 
             track_structure.segments.len(), track_structure.nodes.len());
    
    // 2. 스위치 분석 (노드 타입 + 객체 정보 통합)
    println!("Analyzing switches...");
    let switches = analyze_switches(model);
    println!("Found {} switches", switches.len());
    
    // 3. 신호 및 검지기 분석
    println!("Extracting signal and detector information...");
    let signals = extract_signal_info(&model.objects);
    let detectors = extract_detector_info(&model.objects);
    println!("Found {} signals, {} detectors", signals.len(), detectors.len());
    
    // 4. RailML 모델 생성
    println!("Creating RailML model...");
    let railml_model = create_railml_model(&track_structure, &switches, &signals, &detectors)?;
    println!("RailML model created successfully");
    
    // 5. XML 생성
    println!("Generating XML...");
    let xml_content = generate_railml_xml(&railml_model)?;
    println!("XML generation completed, size: {} bytes", xml_content.len());
    
    Ok(xml_content)
}

// ============================================================================
// GUI 관련 코드 (기존 코드 유지)
// ============================================================================

pub struct ExportWindow {
    pub open: bool,
    state: ExportState,
    thread: Option<mpsc::Receiver<ExportState>>,
    thread_pool: BackgroundJobs,
}

impl ExportWindow {
    pub fn new(thread_pool: BackgroundJobs) -> Self {
        ExportWindow {
            open: false,
            state: ExportState::ChooseFile,
            thread: None,
            thread_pool: thread_pool,
        }
    }
}

#[derive(Debug)]
pub enum ExportState {
    Ping,
    ChooseFile,
    Converting,
    ConversionError(String),
    Success(String),
}

impl ExportWindow {
    pub fn open(&mut self) { self.open = true; }

    pub fn update(&mut self) {
        while let Some(Ok(msg)) = self.thread.as_mut().map(|rx| rx.try_recv()) {
            println!("export window new state: {:?}", msg);
            self.state = msg;
        }
    }

    pub fn draw(&mut self, model: &Model) {
        if !self.open { return; }
        use backend_glfw::imgui::*;
        unsafe {
            widgets::next_window_center_when_appearing();
            igBegin(const_cstr!("Export to railML file").as_ptr(), &mut self.open as _, 0 as _);

            match &self.state {
                ExportState::ChooseFile => {
                    if igButton(const_cstr!("Browse for file...").as_ptr(),
                                ImVec2 { x: 120.0, y: 0.0 }) {
                        if let Some(filename) = tinyfiledialogs::save_file_dialog("Save railML file", "") {
                            let filename = if filename.ends_with(".railml") {
                                filename
                            } else {
                                format!("{}.railml", filename)
                            };
                            self.background_export_file(filename, model.clone());
                        }
                    }
                },
                ExportState::Converting => {
                    widgets::show_text("Converting model to railML...");
                },
                ExportState::Success(filename) => {
                    widgets::show_text(&format!("Successfully exported to: {}", filename));
                    if igButton(const_cstr!("Close").as_ptr(), ImVec2 { x: 80.0, y: 0.0 }) {
                        self.close();
                    }
                },
                ExportState::ConversionError(error) => {
                    widgets::show_text(&format!("Error: {}", error));
                    if igButton(const_cstr!("Close").as_ptr(), ImVec2 { x: 80.0, y: 0.0 }) {
                        self.close();
                    }
                },
                _ => { widgets::show_text(&format!("{:?}", self.state)); },
            }

            igEnd();
        }
    }

    pub fn background_export_file(&mut self, filename: String, model: Model) {
        info!("Starting background export to railML file {:?}", filename);
        let (tx, rx) = mpsc::channel();
        self.thread = Some(rx);
        self.thread_pool.execute(move || { export_railml_file(filename, model, tx); });
    }

    pub fn close(&mut self) {
        self.open = false;
        self.state = ExportState::ChooseFile;
        self.thread = None;
    }
}

pub fn export_railml_file(filename: String, model: Model, tx: mpsc::Sender<ExportState>) {
    if tx.send(ExportState::Converting).is_err() { return; }
    
    match convert_model_to_railml(&model) {
        Ok(railml_content) => {
            match fs::write(&filename, railml_content) {
                Ok(_) => {
                    let _ = tx.send(ExportState::Success(filename));
                },
                Err(e) => {
                    let _ = tx.send(ExportState::ConversionError(format!("File write error: {}", e)));
                },
            }
        },
        Err(e) => {
            let _ = tx.send(ExportState::ConversionError(format!("Conversion error: {}", e)));
        },
    }
} 