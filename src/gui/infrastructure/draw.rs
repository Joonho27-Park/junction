use crate::app::*;
use crate::gui::widgets::Draw;
use crate::util;
use crate::document::model::*;
use crate::document::*;
use crate::document::analysis::*;
use crate::document::objects::*;
use crate::document::infview::*;
use crate::document::dispatch::*;
use crate::document::interlocking::*;
use crate::config::*;

use backend_glfw::imgui::*;
use nalgebra_glm as glm;
use matches::matches;
use std::collections::HashMap;

use rolling::input::staticinfrastructure as rolling_inf;
use crate::document::dgraph::*;

pub fn highlight_node(config :&Config, draw :&Draw, inf_view :&InfView, dgraph :&DGraph, node :rolling_inf::NodeId) {
    // first try node coords
    if let Some(pt) = dgraph.node_ids.get_by_left(&node) {
        box_around(config, draw, inf_view, glm::vec2(pt.x as f32, pt.y as f32));
    } else if let Some(pt) = dgraph.node_ids.get_by_left(&dgraph.rolling_inf.nodes[node].other_node){
        box_around(config, draw, inf_view, glm::vec2(pt.x as f32, pt.y as f32));
    } else {
        for obj_id in dgraph.rolling_inf.nodes[node].objects.iter() {
            if let Some(pta) = dgraph.object_ids.get_by_left(obj_id) {
                box_around(config, draw, inf_view, unround_coord(*pta));
            }
        }
    }
}

pub fn box_around(config :&Config, draw :&Draw, inf_view :&InfView, p :PtC) {
    let p = draw.pos + inf_view.view.world_ptc_to_screen(p);
    let window = ImVec2 { x: 8.0, y: 8.0 };
    unsafe {
    ImDrawList_AddRect(draw.draw_list, p - window, p + window,
                       config.color_u32(RailUIColorName::CanvasTrackDrawing),
                       0.0,0,5.0);
    }
}

pub fn base(config :&Config, analysis :&Analysis, inf_view :&InfView, 
            instant :Option<&Instant>,
            dispatch_view :&Option<DispatchView>, draw :&Draw) {
    let empty_state = HashMap::new();
    let object_states = if let Some(instant) = instant { 
        &instant.infrastructure.object_state } else { &empty_state };

    let m = analysis.model();
    let d = analysis.data();
    unsafe {

        let sel_window = if let Action::Normal(NormalState::SelectWindow(a)) = &inf_view.action {
            Some((*a, *a + igGetMouseDragDelta_nonUDT2(0,-1.0).into()))
        } else { None };

        // 이 부분이 정수 격자점에 대해 작은 원을 그리는
        let (lo,hi) = inf_view.view.points_in_view(draw.size);
        let color_grid = config.color_u32(RailUIColorName::CanvasGridPoint);
        for x in lo.x..=hi.x {
            for y in lo.y..=hi.y {
                let pt = inf_view.view.world_pt_to_screen(glm::vec2(x,y));
                ImDrawList_AddCircleFilled(draw.draw_list, draw.pos+pt, 3.0, color_grid, 4);
            }
        }

        let color_line = config.color_u32(RailUIColorName::CanvasTrack);
        let color_line_selected = config.color_u32(RailUIColorName::CanvasTrackSelected);
        for l in &m.linesegs {
            let p1 = inf_view.view.world_pt_to_screen(l.0);
            let p2 = inf_view.view.world_pt_to_screen(l.1);
            let selected = inf_view.selection.contains(&Ref::LineSeg(l.0,l.1));
            let preview = sel_window
                .map(|(a,b)| util::point_in_rect(p1,a,b) || util::point_in_rect(p2,a,b))
                .unwrap_or(false) ;
            let col = if selected || preview { color_line_selected } else { color_line };
            // [기본 track(선로) 두께]
            // 실제 선로(트랙)를 화면에 그릴 때 사용하는 두께. 전체 인프라 뷰에서 가장 기본이 되는 선로의 두께를 결정한다.
            ImDrawList_AddLine(draw.draw_list, draw.pos + p1, draw.pos + p2, col, 2.5);
        }

        let color_node = config.color_u32(RailUIColorName::CanvasNode);
        let color_node_selected = config.color_u32(RailUIColorName::CanvasNodeSelected);
        if let Some((_gen,topo)) = d.topology.as_ref() {
            use nalgebra_glm::{vec2, rotate_vec2, radians, vec1, normalize};
            for (pt0,(t,vc)) in &topo.locations {
                let selected = inf_view.selection.contains(&Ref::Node(*pt0));
                let preview = sel_window.map(|(a,b)| 
                         util::point_in_rect(inf_view.view.world_pt_to_screen(*pt0),a,b)).unwrap_or(false);
                let col = if selected || preview { color_node_selected } 
                            else { color_node };

                let pt :PtC = vec2(pt0.x as _ ,pt0.y as _ );
                let tangent :PtC = vec2(vc.x as _ ,vc.y as _ );
                match t {
                    NDType::OpenEnd => {
                        for angle in &[-45.0,45.0] {
                            // [OpenEnd 노드의 선 두께]
                            // 선로의 끝(OpenEnd)에서 양쪽으로 뻗어나가는 짧은 선(끝 표시)의 두께. 선로의 끝을 시각적으로 강조할 때 사용.
                            // 실제로는 화살표 모습으로 나타남!!
                            ImDrawList_AddLine(draw.draw_list,
                               draw.pos + inf_view.view.world_ptc_to_screen(pt),
                               draw.pos + inf_view.view.world_ptc_to_screen(pt) 
                                + util::to_imvec(8.0*rotate_vec2(&normalize(&tangent),radians(&vec1(*angle)).x)), col, 2.5);
                        }
                    },
                    NDType::Cont => {
                        ImDrawList_AddCircleFilled(draw.draw_list, 
                            draw.pos + inf_view.view.world_ptc_to_screen(pt), 4.0, col, 8);
                    },
                    NDType::Sw(side) => {
                        let angle = if matches!(side, Side::Left) { 45.0 } else { -45.0 };
                        let p1 = draw.pos + inf_view.view.world_ptc_to_screen(pt);
                        let p2 = p1 + util::to_imvec(15.0*normalize(&tangent));
                        let p3 = p1 + util::to_imvec(15.0*rotate_vec2(&(1.41*normalize(&tangent)), radians(&vec1(angle)).x));
                        ImDrawList_AddTriangleFilled(draw.draw_list, p1,p2,p3, col);
                    },
                    NDType::Err =>{
                        let p = draw.pos + inf_view.view.world_ptc_to_screen(pt);
                        let radius = 12.0;
                        // Draw error node as an empty (outline) circle
                        ImDrawList_AddCircle(draw.draw_list, p, radius, config.color_u32(RailUIColorName::CanvasNodeError), 16, 3.0);
                    },
                    NDType::BufferStop => {
                        let tangent = util::to_imvec(normalize(&tangent));
                        let normal = ImVec2 { x: -tangent.y, y: tangent.x };

                        let node = draw.pos + inf_view.view.world_ptc_to_screen(pt);
                        let pline :&[ImVec2] = &[node + 8.0*normal + 2.0*4.0 * tangent,
                                                 node + 8.0*normal,
                                                 node - 8.0*normal,
                                                 node - 8.0*normal + 2.0*4.0 * tangent];

                        // [BufferStop의 선 두께]
                        // BufferStop(종단 정지장치)에서 그려지는 다각형(Polyline)의 선 두께. 종단부의 시각적 강조에 사용.
                        ImDrawList_AddPolyline(draw.draw_list,pline.as_ptr(), pline.len() as i32, col, false, 2.5);

                    },
                    NDType::Crossing(type_) => {
                        let left_conn  = matches!(type_, CrossingType::DoubleSlip | CrossingType::SingleSlip(Side::Left));
                        let right_conn = matches!(type_, CrossingType::DoubleSlip | CrossingType::SingleSlip(Side::Right));

                        let tangenti = util::to_imvec(normalize(&tangent));
                        let normal = ImVec2 { x: tangenti.y, y: tangenti.x };

                        if right_conn {
                            let base = draw.pos + inf_view.view.world_ptc_to_screen(pt) - 4.0*normal - 2.0f32.sqrt()*2.0*tangenti;
                            let pline :&[ImVec2] = &[base - 8.0*tangenti,
                                                     base,
                                                     base + 8.0*util::to_imvec(rotate_vec2(&tangent, radians(&vec1(45.0)).x))];
                            // [Crossing의 선 두께]
                            // Crossing(건널목, 십자선 등)에서 그려지는 다각형(Polyline)의 선 두께. 교차점의 선로를 강조할 때 사용.
                            ImDrawList_AddPolyline(draw.draw_list,pline.as_ptr(), pline.len() as i32, col, false, 2.5);
                        }

                        if left_conn {
                            let base = draw.pos + inf_view.view.world_ptc_to_screen(pt) + 4.0*normal + 2.0f32.sqrt()*2.0*tangenti;
                            let pline :&[ImVec2] = &[base + 8.0*tangenti,
                                                     base,
                                                     base - 8.0*util::to_imvec(rotate_vec2(&tangent, radians(&vec1(45.0)).x))];
                            ImDrawList_AddPolyline(draw.draw_list,pline.as_ptr(), pline.len() as i32, col, false, 2.5);
                        }

                        if left_conn || right_conn {
                            let p = draw.pos + inf_view.view.world_ptc_to_screen(pt);
                            let pa = util::to_imvec(15.0*normalize(&tangent));
                            let pb = util::to_imvec(15.0*rotate_vec2(&normalize(&tangent), radians(&vec1(45.0)).x));
                            ImDrawList_AddTriangleFilled(draw.draw_list,p,p+pa,p+pb,col);
                            ImDrawList_AddTriangleFilled(draw.draw_list,p,p-pa,p-pb,col);
                        } else {
                            ImDrawList_AddCircleFilled(draw.draw_list, draw.pos + inf_view.view.world_ptc_to_screen(pt), 4.0, col, 8);
                        }
                    },
                }
            }
        }


        let color_obj = config.color_u32(RailUIColorName::CanvasSymbol);
        let color_obj_selected = config.color_u32(RailUIColorName::CanvasSymbolSelected);
        //placed된 후의 색깔 지정. -> canvas색깔과 같음.
        let color_detector = config.color_u32(RailUIColorName::CanvasDetector);

        // 1. 객체 정보 한 번에 수집
        let mut detectors = Vec::new();
        let mut signals = Vec::new();
        let mut switches = Vec::new();

        let empty = Vec::new(); // 루프 밖에 선언
        for (pta, obj) in &m.objects {
            let selected = inf_view.selection.contains(&Ref::Object(*pta));
            let preview = sel_window.map(|(a,b)| 
                util::point_in_rect(inf_view.view.world_ptc_to_screen(unround_coord(*pta)),a,b)).unwrap_or(false);
            let state = object_states.get(pta).unwrap_or(&empty);

            if obj.functions.iter().any(|f| matches!(f, Function::Detector)) {
                detectors.push((pta, obj, selected, preview, state));
            } else if obj.functions.iter().any(|f| matches!(f, Function::MainSignal { .. } | Function::ShiftingSignal { .. })) {
                signals.push((pta, obj, selected, preview, state));
            } else if obj.functions.iter().any(|f| matches!(f, Function::Switch)) {
                switches.push((pta, obj, selected, preview, state));
            }
        }

        // 2. 타입별로 draw
        for (pta, obj, selected, preview, state) in detectors {
            let col = if selected || preview { color_obj_selected } else { color_detector };
            obj.draw(draw.pos, &inf_view.view, draw.draw_list, col, state, config);
        }
        for (pta, obj, selected, preview, state) in signals {
            let col = if selected || preview { color_obj_selected } else { color_obj };
            obj.draw(draw.pos, &inf_view.view, draw.draw_list, col, state, config);
        }
        for (pta, obj, selected, preview, state) in switches {
            let col = if selected || preview { color_obj_selected } else { color_obj };
            obj.draw(draw.pos, &inf_view.view, draw.draw_list, col, state, config);
        }
    }
}

pub fn route(config :&Config, analysis :&Analysis, inf_view :&InfView, draw :&Draw, route_idx :usize) -> Option<()> { 
    unsafe {
        let il = &analysis.data().interlocking.as_ref()?.1;
        let dgraph = &analysis.data().dgraph.as_ref()?.1;
        let RouteInfo { route, path, ..} = &il.routes[route_idx];
        let color_path = config.color_u32(RailUIColorName::CanvasRoutePath);
        let color_section = config.color_u32(RailUIColorName::CanvasRouteSection);

        for sec in route.resources.sections.iter() {
            if let Some(edges) = dgraph.tvd_edges.get(sec) {
                for (a,b) in edges.iter() {
                    if let Some((v,_)) = util::get_symm(&dgraph.edge_lines, (*a,*b)) {
                        for (pt_a,pt_b) in v.iter().zip(v.iter().skip(1)) {
                            ImDrawList_AddLine(draw.draw_list,
                                               draw.pos + inf_view.view.world_ptc_to_screen(*pt_a),
                                               draw.pos + inf_view.view.world_ptc_to_screen(*pt_b),
                                               color_section, 2.0*3.0); // 갈 수 있는 경로 두께. 하늘색
                        }
                    }
                }
            }
        }

        for (a,b) in path {
            if let Some((v,_)) = util::get_symm(&dgraph.edge_lines, (*a,*b)) {
                for (pt_a,pt_b) in v.iter().zip(v.iter().skip(1)) {
                    ImDrawList_AddLine(draw.draw_list,
                                       draw.pos + inf_view.view.world_ptc_to_screen(*pt_a),
                                       draw.pos + inf_view.view.world_ptc_to_screen(*pt_b),
                                       color_path, 2.0*5.0); // 실제 경로 두께. 초록색
                }
            }
        }
        // TODO highlight end signal/boundary

        Some(())
    }
}

pub fn trains(config :&Config, instant :&Instant, inf_view :&InfView, draw :&Draw) -> Option<()> { 
    let color = config.color_u32(RailUIColorName::CanvasTrain);
    let sight_color = config.color_u32(RailUIColorName::CanvasTrainSight);
    
    // 열차를 선로 위쪽에 위치시키기 위한 오프셋 거리
    let offset_distance = 0.5;
    
    for t in instant.trains.iter() {
        // 안전장치: train이 비어있으면 건너뛰기
        if t.lines.is_empty() {
            continue;
        }
        
        // 1. 각 점마다 보간 normal 계산
        let mut avg_normals = Vec::new();
        let n = t.lines.len() + 1; // 점 개수 = 선분 개수 + 1

        // 점 좌표 추출
        let mut points = Vec::new();
        if let Some((first, _)) = t.lines.first() {
            points.push(*first);
            for &(_, p2) in t.lines.iter() {
                points.push(p2);
            }
        }

        // points, avg_normals 생성 후
        if points.len() < 2 { continue; } // 최소 2개 점 필요

        // 각 점의 normal 계산
        for i in 0..n {
            // 안전장치: 인덱스 범위 체크
            if i >= points.len() {
                break;
            }
            
            // 이전 tangent
            let prev_tangent = if i > 0 && i < points.len() {
                let (p0, p1) = (points[i-1], points[i]);
                let diff = p1 - p0;
                let length = glm::length(&diff);
                if length > 0.0 {
                    glm::normalize(&diff)
                } else {
                    glm::vec2(1.0, 0.0) // 기본값
                }
            } else if points.len() > 1 {
                let diff = points[1] - points[0];
                let length = glm::length(&diff);
                if length > 0.0 {
                    glm::normalize(&diff)
                } else {
                    glm::vec2(1.0, 0.0) // 기본값
                }
            } else {
                glm::vec2(1.0, 0.0) // 기본값
            };
            
            // 다음 tangent
            let next_tangent = if i < n-1 && i+1 < points.len() {
                let (p0, p1) = (points[i], points[i+1]);
                let diff = p1 - p0;
                let length = glm::length(&diff);
                if length > 0.0 {
                    glm::normalize(&diff)
                } else {
                    glm::vec2(1.0, 0.0) // 기본값
                }
            } else if i > 0 && i < points.len() {
                let diff = points[i] - points[i-1];
                let length = glm::length(&diff);
                if length > 0.0 {
                    glm::normalize(&diff)
                } else {
                    glm::vec2(1.0, 0.0) // 기본값
                }
            } else {
                glm::vec2(1.0, 0.0) // 기본값
            };
            
            // 각각의 normal
            let prev_normal = glm::vec2(-prev_tangent.y, prev_tangent.x);
            let next_normal = glm::vec2(-next_tangent.y, next_tangent.x);
            
            // 평균 normal 계산 (안전장치 추가)
            let avg_normal_vec = prev_normal + next_normal;
            let avg_length = glm::length(&avg_normal_vec);
            let avg_normal = if avg_length > 0.0 {
                glm::normalize(&avg_normal_vec)
            } else {
                glm::vec2(0.0, 1.0) // 기본 위쪽 방향
            };
            
            avg_normals.push(avg_normal);
        }

        // 2. 각 선분을 그릴 때, 시작점과 끝점에서 각각 보간 normal로 offset
        for i in 0..t.lines.len() {
            // 반드시 avg_normals.len() > i+1, points.len() > i+1
            if i+1 >= avg_normals.len() || i+1 >= points.len() { break; }
            
            let (p1, p2) = t.lines[i];
            let n1 = avg_normals[i];
            let n2 = avg_normals[i+1];
            
            // 안전장치: NaN이나 Inf 체크
            if n1.x.is_nan() || n1.y.is_nan() || n2.x.is_nan() || n2.y.is_nan() ||
               n1.x.is_infinite() || n1.y.is_infinite() || n2.x.is_infinite() || n2.y.is_infinite() {
                continue;
            }
            
            let p1_offset = p1 + offset_distance * n1;
            let p2_offset = p2 + offset_distance * n2;
            
            unsafe {
                ImDrawList_AddLine(
                    draw.draw_list,
                    draw.pos + inf_view.view.world_ptc_to_screen(p1_offset),
                    draw.pos + inf_view.view.world_ptc_to_screen(p2_offset),
                    color, 2.0*5.0
                );
            }
        }

        // 시야선도 마지막 점의 보간 normal 사용
        if let Some(front) = t.get_front() {
            let n = avg_normals.last().copied().unwrap_or(glm::vec2(0.0, 1.0));
            
            // 안전장치: NaN이나 Inf 체크
            if !n.x.is_nan() && !n.y.is_nan() && !n.x.is_infinite() && !n.y.is_infinite() {
                let front_offset = front + offset_distance * n;
                for pta in t.signals_sighted.iter() {
                    unsafe {
                        ImDrawList_AddLine(
                            draw.draw_list,
                            draw.pos + inf_view.view.world_ptc_to_screen(front_offset),
                            draw.pos + inf_view.view.world_ptc_to_screen(unround_coord(*pta)),
                            sight_color, 2.0*2.0
                        );
                    }
                }
            }
        }
    }

    Some(())
}

pub fn state(config :&Config, instant :&Instant, inf_view :&InfView, draw :&Draw) {
    for (_tvd, status, lines) in instant.infrastructure.sections.iter() {
        let color = match status {
            SectionStatus::Occupied => config.color_u32(RailUIColorName::CanvasTVDOccupied),
            SectionStatus::Reserved => config.color_u32(RailUIColorName::CanvasTVDReserved),
            _ => config.color_u32(RailUIColorName::CanvasTVDFree),
        };

        for (p1,p2) in lines.iter() {
            unsafe {
                ImDrawList_AddLine(draw.draw_list,
                                   draw.pos + inf_view.view.world_ptc_to_screen(*p1),
                                   draw.pos + inf_view.view.world_ptc_to_screen(*p2),
                                   color, 2.0*2.0); // 갈예정(노란색), 차지하고 있는 궤도(빨간색
            }
        }
    }
}