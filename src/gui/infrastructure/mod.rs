pub mod draw;
pub mod menus;

use const_cstr::*;
use matches::matches;
use backend_glfw::imgui::*;
use nalgebra_glm as glm;

use crate::util;
use crate::app::App;
use crate::config::*;
use crate::document::*;
use crate::document::infview::*;
use crate::document::view::*;
use crate::document::interlocking::*;
use crate::document::model::*;
use crate::document::analysis::*;
use crate::document::dispatch::*;
use crate::document::objects::*;
use crate::gui::widgets;
use crate::gui::widgets::Draw;
use crate::config::RailUIColorName;
use crate::document::model::generate_unique_dispatch_name;

// 스위치 노드에 해당하는 스위치 객체를 생성하는 함수
fn create_switch_object_for_node(analysis: &mut Analysis, node_pt: Pt, vc: Vc, side: Side, inf_view: &mut InfView) {
    let node_ptc = glm::vec2(node_pt.x as f32, node_pt.y as f32);
    let obj_pt = round_coord(node_ptc);
    
    // 해당 위치에 스위치 객체가 이미 있는지 확인
    if !analysis.model().objects.contains_key(&obj_pt) {
        // insert_object와 동일한 로직으로 스위치 객체 생성
        // 스위치 설치 위치 계산
        // side에 따라 분기 방향의 맞은편에 설치
        let normal = glm::vec2(-vc.y as f32, vc.x as f32);
        let normal_len = glm::length(&normal);
        let normalized_normal = if normal_len > 0.0 { normal / normal_len } else { normal };
        
        // side가 Left면 normal 방향(반대편), Right면 -normal 방향(반대편)에 설치
        let offset = match side {
            Side::Left => -0.5 * normalized_normal,
            Side::Right => 0.5 * normalized_normal,
        };
        let final_loc = node_ptc + offset;
        
        // 열차 진행 방향과 같은 방향으로 tangent 설정
        let switch_tangent = glm::vec2(-vc.x, -vc.y);; // vc는 이미 열차 진행 방향
        
        // placed_angle 계산 (직선 방향 기준)
        let tangent_angle = (switch_tangent.y as f32).atan2(switch_tangent.x as f32);
        let angle_degrees = tangent_angle * 180.0 / std::f32::consts::PI;
        
        let switch_obj = Object {
            loc: final_loc,
            tangent: switch_tangent, // 열차 진행 방향과 같은 방향
            functions: vec![Function::Switch { id: None }],
            id: None,
            signal_props: None,
            switch_props: Some(SwitchProperties {
                switch_type: SwitchType::Single, // 기본값으로 Single 설정
            }),
            placed_angle: Some(angle_degrees),
        };
        
        // ID 입력 다이얼로그 표시 (insert_object와 동일한 방식)
        inf_view.id_input = Some(IdInputState {
            object: switch_obj.clone(),
            id: String::new(),
            position: final_loc,
            function_type: Function::Switch { id: None },
        });
    }
}

#[derive(Copy,Clone,Debug)]
pub enum Highlight {
    Ref(Ref),
    Tvd(usize),
}

// ID 텍스트 드래그 상태 관리
#[derive(Debug, Clone)]
pub struct IdDragState {
    pub is_dragging: bool,
    pub dragged_id: Option<String>,
    pub source_pta: Option<PtA>,
    pub drag_start_pos: Option<PtC>,
    pub drag_offset: glm::Vec2,
    pub drag_preview_pos: Option<PtC>,
    pub mouse_was_pressed: bool,  // 마우스가 이전 프레임에 눌려있었는지 추적
    pub id_clickable: bool,       // ID가 클릭 가능한 상태인지 (생성 직후에는 false)
    pub creation_time: Option<f64>, // TrackLabel 생성 시간 (클릭 방지용)
}

impl IdDragState {
    pub fn new() -> Self {
        Self {
            is_dragging: false,
            dragged_id: None,
            source_pta: None,
            drag_start_pos: None,
            drag_offset: glm::zero(),
            drag_preview_pos: None,
            mouse_was_pressed: false,
            id_clickable: false,  // 초기에는 클릭 불가능
            creation_time: None,
        }
    }
}


pub fn inf_view(config :&Config, 
                analysis :&mut Analysis,
                inf_view :&mut InfView,
                dispatch_view :&mut Option<DispatchView>) -> Draw {
    unsafe {
        let pos_before : ImVec2 = igGetCursorPos_nonUDT2().into();
        let size = igGetContentRegionAvail_nonUDT2().into();
        let draw = widgets::canvas(size,
                        config.color_u32(RailUIColorName::CanvasBackground),
                        const_cstr!("railwaycanvas").as_ptr());
        draw.begin_draw();
        scroll(inf_view);
        let mut preview_route = None;
        context_menu(analysis, inf_view, dispatch_view, &draw, &mut preview_route);
        interact(config, analysis, inf_view, &draw);
        

        
        draw_inf(config, analysis, inf_view, dispatch_view, &draw, preview_route);
        draw.end_draw();
        // Object properties panel is now handled by the sidebar
        // Toolbar moved to sidebar
        // let pos_after = igGetCursorPos_nonUDT2().into();
        // let framespace = igGetFrameHeightWithSpacing() - igGetFrameHeight();
        // igSetCursorPos(pos_before + ImVec2 { x: 2.0*framespace, y: 2.0*framespace });
        // inf_toolbar(analysis, inf_view);
        // igSetCursorPos(pos_after);
        draw
    }
}

fn draw_inf(config :&Config, analysis :&mut Analysis, inf_view :&mut InfView, 
            dispatch_view :&Option<DispatchView>,
            draw :&Draw, preview_route :Option<usize>) {

    let instant = {
        if let Some(dref) = dispatch_view_ref(dispatch_view) {
            inf_view.instant_cache.update(analysis, dref);
            inf_view.instant_cache.get(dref)
        } else { None }
    };

    draw::base(config, analysis, inf_view, instant, dispatch_view, draw);

    if let Some(instant) = instant {
        draw::state(config, instant, inf_view, draw);
        draw::trains(config, instant, inf_view, draw);
    }

    if let Some(r) = preview_route { draw::route(config, analysis, inf_view, draw, r); }
    
    // 이름 입력 다이얼로그 표시
    draw_id_input_dialog(analysis, inf_view);
}

fn scroll(inf_view :&mut InfView) { 
    unsafe {
        if inf_view.focused { return; }
        if !igIsItemHovered(0){ return; }
        let io = igGetIO();
        let wheel = (*io).MouseWheel;
        if wheel != 0.0 {
            inf_view.view.zoom(wheel);
        }
        if ((*io).KeyCtrl && igIsMouseDragging(0,-1.0)) || igIsMouseDragging(2,-1.0) {
            inf_view.view.translate((*io).MouseDelta);
        }
    }
}


fn interact(config :&Config, analysis :&mut Analysis, inf_view :&mut InfView, draw :&Draw) {
    // ID 텍스트 드래그 처리 (다른 상호작용보다 우선)
    handle_id_drag(config, analysis, inf_view, draw);
    
    match &inf_view.action {
        Action::Normal(normal) => { 
            let normal = *normal;
            interact_normal(config, analysis, inf_view, draw, normal); 
        },
        Action::DrawingLine(from) => { 
            let from = *from;
            interact_drawing(config, analysis, inf_view, draw, from); 
        },
        Action::InsertObject(obj) => { 
            let obj = obj.clone();
            interact_insert(config, analysis, inf_view, draw, obj); 
        },
        Action::SelectObjectType => {},
    }
}

fn interact_normal(config :&Config, analysis :&mut Analysis, 
                   inf_view :&mut InfView, draw :&Draw, state :NormalState) {
    // config
    // inf_view
    // analysis
    unsafe {
        if inf_view.focused { return; }
        let io = igGetIO();
        match state {
            NormalState::SelectWindow(a) => {
                let b = a + igGetMouseDragDelta_nonUDT2(0,-1.0).into();
                if igIsMouseDragging(0,-1.0) {
                    ImDrawList_AddRect(draw.draw_list, draw.pos + a, draw.pos + b,
                                       config.color_u32(RailUIColorName::CanvasSelectionWindow),
                                       0.0, 0, 1.0);
                } else {
                    set_selection_window(inf_view, analysis, a,b);
                    inf_view.action = Action::Normal(NormalState::Default);
                }
            },
            NormalState::DragMove(typ) => {
                if igIsMouseDragging(0,-1.0) {
                    let delta = inf_view.view.screen_to_world_ptc((*io).MouseDelta) -
                                inf_view.view.screen_to_world_ptc(ImVec2 { x:0.0, y: 0.0 });
                    match typ {
                        MoveType::Continuous => { if delta.x != 0.0 || delta.y != 0.0 {
                            move_selected_objects(analysis, inf_view, inf_view.view.screen_to_world_ptc((*io).MousePos) + glm::vec2(0.0,1.5)); }},
                        MoveType::Grid(p) => {
                            inf_view.action = 
                                Action::Normal(NormalState::DragMove(MoveType::Grid(p + delta)));
                        },
                    }
                } else {
                    inf_view.action = Action::Normal(NormalState::Default);
                }
            }
            NormalState::Default => {
                // 배경 클릭 차단: focused가 true면 return
                if inf_view.focused { return; }
                if !(*io).KeyCtrl && igIsItemHovered(0) && igIsMouseDragging(0,-1.0) {
                    if let Some((r,_)) = analysis.get_closest(
                            inf_view.view.screen_to_world_ptc(draw.mouse)) {
                        if !inf_view.selection.contains(&r) {
                            inf_view.selection = std::iter::once(r).collect();
                        }
                        if inf_view.selection.iter().any(|x| matches!(x, Ref::Node(_)) || matches!(x, Ref::LineSeg(_,_))) {
                            inf_view.action = Action::Normal(NormalState::DragMove(
                                    MoveType::Grid(glm::zero())));
                        } else {
                            inf_view.action = Action::Normal(NormalState::DragMove(MoveType::Continuous));
                        }
                    } else {
                        let a = (*io).MouseClickedPos[0] - draw.pos;
                        inf_view.action = Action::Normal(NormalState::SelectWindow(a));
                    }
                } else {
                    if igIsMouseReleased(0) {
                        // focused가 true면 배경 클릭 무시
                        if inf_view.focused { return; }
                    }
                    if igIsItemHovered(0) && igIsMouseReleased(0) {
                        if inf_view.focused { return; }
                        if !(*io).KeyShift { inf_view.selection.clear(); }
                        if let Some((r,_)) = analysis.get_closest(
                                inf_view.view.screen_to_world_ptc(draw.mouse)) {
                                                // Track segment 클릭 시 해당 TrackLabel 찾기
                    if let Ref::LineSeg(start_pt, end_pt) = r {
                        
                        
                        if let Some(track_label_pta) = find_track_label_for_segment(analysis, (start_pt, end_pt)) {
                                                    inf_view.selection.insert(Ref::Object(track_label_pta));
                    } else {
                            // TrackLabel을 찾지 못한 경우 기존 동작 유지
                            inf_view.selection.insert(r);
                        }
                    } else {
                        // Track segment가 아닌 경우 기존 동작 유지
                        inf_view.selection.insert(r);
                    }
                        }
                    }
                }
            },
        }
    }
}

pub fn set_selection_window(inf_view :&mut InfView, analysis :&Analysis, a :ImVec2, b :ImVec2) {
    let s = analysis.get_rect(inf_view.view.screen_to_world_ptc(a),
                         inf_view.view.screen_to_world_ptc(b))
                .into_iter().collect();
    inf_view.selection = s;
}

pub fn move_selected_objects(analysis :&mut Analysis, inf_view :&mut InfView, to :PtC) {
    let mut model = analysis.model().clone();
    let mut changed_ptas = Vec::new();

    // 선택된 객체들의 평균 위치 계산
    let mut total_pos = glm::vec2(0.0, 0.0);
    let mut count = 0;
    for id in inf_view.selection.iter() {
        match id {
            Ref::Object(pta) => {
                if let Some(obj) = model.objects.get(pta) {
                    total_pos += obj.loc;
                    count += 1;
                }
            },
            _ => {},
        }
    }

    if count > 0 {
        let avg_pos = total_pos / count as f32;
        let delta = to - avg_pos;

        for id in inf_view.selection.iter() {
            match id {
                Ref::Object(pta) => {
                    let mut obj = model.objects.get_mut(pta).unwrap().clone();
                    let moved = obj.move_to(&model, &analysis, to);
                    if let Some(_) = moved { return; }
                    let new_pta = round_coord(obj.loc);
                    model.objects.remove(pta);
                    model.objects.insert(new_pta,obj);
                    if *pta != new_pta { changed_ptas.push((*pta,new_pta)); }
                },
                _ => {},
            }
        }
    }

    let selection_before = inf_view.selection.clone();

    for (a,b) in changed_ptas {
        model_rename_object(&mut model,a,b);
        inf_view.selection.remove(&Ref::Object(a));
        inf_view.selection.insert(Ref::Object(b));
    }

    analysis.set_model(model, Some(EditClass::MoveObjects(selection_before)));
    analysis.override_edit_class(EditClass::MoveObjects(inf_view.selection.clone()));
}

fn interact_drawing(config :&Config, analysis :&mut Analysis, inf_view :&mut InfView, 
                    draw :&Draw, from :Option<Pt>) {
    unsafe {
        if inf_view.focused { return; }
        let color = config.color_u32(RailUIColorName::CanvasTrackDrawing);
        let pt_end_raw = inf_view.view.screen_to_world_pt(draw.mouse);
        let pt_end = util::clamp_pt(pt_end_raw);

        if pt_end_raw.x.abs() > 10_000 || pt_end_raw.y.abs() > 10_000 {
            inf_view.action = Action::DrawingLine(None);
            return;
        }
        // Draw preview
        if let Some(pt) = from {
            let pt = util::clamp_pt(pt);
            for (p1,p2) in util::route_line(pt, pt_end) {
                ImDrawList_AddLine(draw.draw_list, draw.pos + inf_view.view.world_pt_to_screen(p1),
                                                   draw.pos + inf_view.view.world_pt_to_screen(p2),
                                              color, 2.0);
            }

            if !igIsMouseDown(0) {
                if pt != pt_end {
                    let mut new_model = analysis.model().clone();
                    if let Some((p1,p2)) = is_boundary_extension(analysis, pt, pt_end) {
                        model_rename_node(&mut new_model, p1, p2);
                    }
                    for (p1,p2) in util::route_line(pt,pt_end) {
                        let unit = util::unit_step_diag_line(p1,p2);
                        for (pa,pb) in unit.iter().zip(unit.iter().skip(1)) {
                            new_model.linesegs.insert(util::order_ivec(*pa,*pb));
                        }
                    }
                    analysis.set_model(new_model, None);
                    inf_view.selection = std::iter::empty().collect();
                    
                    // 트랙 생성 완료 후 트랙의 중간 좌표를 계산하여 ID 입력창 띄우기
                    let track_mid = glm::vec2(
                        (pt.x + pt_end.x) as f32 / 2.0,
                        (pt.y + pt_end.y) as f32 / 2.0
                    );
                    let track_segment = (pt, pt_end);
                    create_track_with_id_input(analysis, inf_view, track_mid, None);
                }
                inf_view.action = Action::DrawingLine(None);
            }
        } else {
            if igIsItemHovered(0) && igIsMouseDown(0) {
                inf_view.action = Action::DrawingLine(Some(pt_end));
            }
        }
    }
}

fn is_boundary_extension(analysis :&Analysis, p1 :Pt, p2 :Pt) -> Option<(Pt,Pt)> {
    let locs = &analysis.data().topology.as_ref()?.1.locations;
    match (locs.get(&p1), locs.get(&p2)) {
        (Some((NDType::OpenEnd, _)), None) => { return Some((p1,p2)); }
        _ => {},
    }
    match (locs.get(&p2), locs.get(&p1)) {
        (Some((NDType::OpenEnd, _)), None) => { return Some((p2,p1)); }
        _ => {},
    }
    None
}

fn model_rename_node(model :&mut Model, a :Pt, b :Pt) {
    for (_,dispatch) in model.dispatches.iter_mut() {
        for (_,(_,command)) in dispatch.commands.iter_mut() {
            match command {
                Command::Train(_,r) | Command::Route(r) => {
                    if r.from == Ref::Node(a) {
                        r.from = Ref::Node(b);
                    }
                    if r.to == Ref::Node(a) {
                        r.to = Ref::Node(b);
                    }
                }
                _ => {}
            };
        }
    }

    for (_,p) in model.plans.iter_mut() {
        for (_,(_veh, visits)) in p.trains.iter_mut() {
            for (_,v) in visits.iter_mut() {
                for l in v.locs.iter_mut() {
                    if l == &Ok(Ref::Node(a)) {
                        *l = Ok(Ref::Node(b));
                    }
                }
            }
        }
    }
}

fn model_rename_object(model :&mut Model, a :PtA, b :PtA) {
    for (_,dispatch) in model.dispatches.iter_mut() {
        for (_,(_,command)) in dispatch.commands.iter_mut() {
            match command {
                Command::Train(_,r) | Command::Route(r) => {
                    if r.from == Ref::Object(a) {
                        r.from = Ref::Object(b);
                    }
                    if r.to == Ref::Object(a) {
                        r.to = Ref::Object(b);
                    }
                }
                _ => {}
            };
        }
    }

    for (_,p) in model.plans.iter_mut() {
        for (_,(_veh, visits)) in p.trains.iter_mut() {
            for (_,v) in visits.iter_mut() {
                for l in v.locs.iter_mut() {
                    if l == &Ok(Ref::Object(a)) {
                        *l = Ok(Ref::Object(b));
                    }
                }
            }
        }
    }
}


fn interact_insert(config :&Config, analysis :&mut Analysis, 
                   inf_view :&mut InfView, draw :&Draw, obj :Option<Object>) {
    unsafe {
        if inf_view.focused { return; }
        let io = igGetIO();
        if igIsMouseClicked(1, false) {
            // ID 드래그 중이 아닐 때만 focused 설정
            if !inf_view.track_label_drag.is_dragging {
                inf_view.focused = true;
            }
            return;
        }
        if let Some(mut obj) = obj {
            // 객체의 초기 위치를 마우스 위치로 설정
            obj.loc = inf_view.view.screen_to_world_ptc(draw.mouse);
            
            let moved = obj.move_to(analysis.model(), analysis, inf_view.view.screen_to_world_ptc(draw.mouse));
            
            // 미리보기 시에는 잘 보이는 색상 사용 (감지기는 배치 후에 배경색과 같아짐)
            let preview_color = config.color_u32(RailUIColorName::CanvasSymbol);
            
            obj.draw(draw.pos,&inf_view.view,draw.draw_list,
                    preview_color,&[],&config, Some(inf_view), analysis.model());

            // move_to가 성공했는지 확인 (Some(())이면 성공, None이면 실패)
            let placement_successful = moved.is_some();

            if !placement_successful {
                let p = draw.pos + inf_view.view.world_ptc_to_screen(obj.loc);
                //기존 Rectangle
                /*let window = ImVec2 { x: 12.0, y: 12.0 };
                ImDrawList_AddRect(draw.draw_list, p - window, p + window,
                                   config.color_u32(RailUIColorName::CanvasSymbolLocError),
                                   0.0,0,4.0);
                */
                //Circle로 바꿈
                /*let radius = 15.0;  // 원의 반지름
                ImDrawList_AddCircle(draw.draw_list, p, radius,
                   config.color_u32(RailUIColorName::CanvasSymbolLocError),
                   12, 4.0);
                */

                //X표시 적용
                let size = 10.0;
                // X표시 그리기: 두 개의 대각선
                // 왼쪽 위에서 오른쪽 아래로
                ImDrawList_AddLine(draw.draw_list, 
                   p + ImVec2 { x: -size, y: -size },  // 왼쪽 위
                   p + ImVec2 { x: size, y: size },    // 오른쪽 아래
                   config.color_u32(RailUIColorName::CanvasSymbolLocError),
                   4.0);

                // 오른쪽 위에서 왼쪽 아래로
                ImDrawList_AddLine(draw.draw_list, 
                   p + ImVec2 { x: size, y: -size },   // 오른쪽 위
                   p + ImVec2 { x: -size, y: size },   // 왼쪽 아래
                   config.color_u32(RailUIColorName::CanvasSymbolLocError),
                   4.0);
                            } else  {
                    if igIsMouseReleased(0) && !inf_view.focused {
        
                        // MainSignal 또는 Switch인 경우 이름 입력 다이얼로그 표시
                        if let Some(Function::Signal { .. }) = obj.functions.first() {
                            match obj.signal_props.as_ref().map(|props| props.signal_type.clone()) {
                                Some(SignalType::Home) | Some(SignalType::Departure) => {
                                    inf_view.id_input = Some(IdInputState {
                                        object: obj.clone(),  // move_to가 호출된 후의 obj 사용
                                        id: String::new(),
                                        position: obj.loc.clone(),
                                        function_type: Function::Signal { has_distant: false, id: None },
                                    });
                                },
                                Some(SignalType::Shunting) => {
                                    inf_view.id_input = Some(IdInputState {
                                        object: obj.clone(),  // move_to가 호출된 후의 obj 사용
                                        id: String::new(),
                                        position: obj.loc.clone(),
                                        function_type: Function::Signal { has_distant: false, id: None },
                                    });
                                },
                                _ => {}
                            }
                        } else if obj.functions.iter().any(|f| matches!(f, Function::Switch { .. })) {
                            // 스위치 객체인 경우 미리보기 그대로 설치
                                inf_view.id_input = Some(IdInputState {
                                object: obj.clone(),
                                id: String::new(),
                                position: obj.loc.clone(),
                                function_type: Function::Switch { id: None },
                            });
                        } else {
                        // 그 외 객체는 바로 배치
                    analysis.edit_model(|m| {
                        m.objects.insert(round_coord(obj.loc), obj.clone());
                        None
                    });
                    
                    // detector가 배치된 경우 track 분할 확인
                    if obj.functions.iter().any(|f| matches!(f, Function::Detector)) {
                        if let Some(track_to_split) = find_track_at_position(analysis, obj.loc) {
                            split_track_at_detector(analysis, inf_view, obj.loc, track_to_split);
                        }
                    }
                    }
                }
            }
        }
    }
}

fn inf_toolbar(analysis :&mut Analysis, inf_view :&mut InfView) {
    unsafe  {
    // 마우스 커서 버튼: 선택 모드
    if toolbar_button(
        const_cstr!("\u{f245}").as_ptr(), 
                      matches!(inf_view.action, Action::Normal(_)), true) {
        inf_view.action = Action::Normal(NormalState::Default);
    }
    if igIsItemHovered(0) {
        igBeginTooltip();
        widgets::show_text("\u{f245} select (A)\nSelect tracks, nodes and objects. Drag to move.");
        igEndTooltip();
    }

    igSameLine(0.0,-1.0);

    // 객체삽입 버튼: 객체 삽입(신호기, 궤도 분리, 선로 전환기)
    let current_icon = get_current_object_icon(inf_view);
    if toolbar_button(current_icon,
                      matches!(inf_view.action, Action::InsertObject(_)) || 
                      matches!(inf_view.action, Action::SelectObjectType), true) {
        inf_view.action = Action::SelectObjectType;
    }
    if igIsItemHovered(0) {
        igBeginTooltip();
        widgets::show_text("\u{f637} insert object (S)\nOpens a drop-down menu for selecting an object type.\nInsert the object by clicking a position.");
        igEndTooltip();
    }
    // Fly-out menu
    if matches!(&inf_view.action, Action::SelectObjectType) {
        let button_pos = igGetItemRectMin();
        let button_size = igGetItemRectSize();
        let menu_pos = ImVec2 { x: button_pos.x, y: button_pos.y + button_size.y - 1.0 };
        
        igSetNextWindowPos(menu_pos, 0 as _, ImVec2 { x: 0.0, y: 0.0 });
        igSetNextWindowSize(ImVec2 { x: 170.0, y: 0.0 }, 0 as _);
        
        let window_flags = ImGuiWindowFlags__ImGuiWindowFlags_NoMove as i32 | 
                          ImGuiWindowFlags__ImGuiWindowFlags_NoResize as i32 |
                          ImGuiWindowFlags__ImGuiWindowFlags_NoCollapse as i32 |
                          ImGuiWindowFlags__ImGuiWindowFlags_NoTitleBar as i32;
        
        if igBegin(const_cstr!("ObjectMenu").as_ptr(), std::ptr::null_mut(), window_flags) {
            inf_view.focused = true; // 메뉴 열릴 때 true
            // Home Signal (H)
            if igSelectable(const_cstr!("\u{f637} Home Signal (H)").as_ptr(), false, 0 as _, ImVec2::zero()) {
                inf_view.action = Action::InsertObject(Some(
                    Object {
                        loc: glm::vec2(0.0, 0.0),
                        tangent: glm::vec2(1,0),
                        functions: vec![Function::Signal { has_distant: false, id: None }],
                        id: None,
                        signal_props: Some(SignalProperties {
                            signal_type: SignalType::Home,
                            signal_kind: SignalKind::Two,
                            direction: TrackDirection::Right,
                        }),
                        switch_props: None,
                        placed_angle: None,
                    }
                ));
                inf_view.focused = false;
            }
            // Departure Signal (E)
            if igSelectable(const_cstr!("\u{f5b0} Departure Signal (E)").as_ptr(), false, 0 as _, ImVec2::zero()) {
                inf_view.action = Action::InsertObject(Some(
                    Object {
                        loc: glm::vec2(0.0, 0.0),
                        tangent: glm::vec2(1,0),
                        functions: vec![Function::Signal { has_distant: false, id: None }],
                        id: None,
                        signal_props: Some(SignalProperties {
                            signal_type: SignalType::Departure,
                            signal_kind: SignalKind::Two,
                            direction: TrackDirection::Right,
                        }),
                        switch_props: None,
                        placed_angle: None,
                    }
                ));
                inf_view.focused = false;
            }
            // Shunting Signal (U)
            if igSelectable(const_cstr!("\u{f061} Shunting Signal (U)").as_ptr(), false, 0 as _, ImVec2::zero()) {
                inf_view.action = Action::InsertObject(Some(
                    Object {
                        loc: glm::vec2(0.0, 0.0),
                        tangent: glm::vec2(1,0),
                        functions: vec![Function::Signal { has_distant: false, id: None }],
                        id: None,
                        signal_props: Some(SignalProperties {
                            signal_type: SignalType::Shunting,
                            signal_kind: SignalKind::Two,
                            direction: TrackDirection::Right,
                        }),
                        switch_props: None,
                        placed_angle: None,
                    }
                ));
                inf_view.focused = false;
            }
            
            // Section Insulator (I)
            if igSelectable(const_cstr!("\u{f715} Section Insulator (I)").as_ptr(), false, 0 as _, ImVec2::zero()) {
                inf_view.action = Action::InsertObject(Some(
                    Object {
                        loc: glm::vec2(0.0, 0.0),
                        tangent: glm::vec2(1,0),
                        functions: vec![Function::Detector],
                        id: None,
                        signal_props: None,
                        switch_props: None,
                        placed_angle: None,
                    }
                ));
                inf_view.focused = false;
            }
            
            // Switch (W)
            if igSelectable(const_cstr!("\u{f126} Switch (W)").as_ptr(), false, 0 as _, ImVec2::zero()) {
                inf_view.action = Action::InsertObject(Some(
                    Object {
                        loc: glm::vec2(0.0, 0.0),
                        tangent: glm::vec2(1,0),
                        functions: vec![Function::Switch { id: None }],
                        id: None,
                        signal_props: None,
                        switch_props: Some(SwitchProperties {
                            switch_type: SwitchType::Single,
                        }),
                        placed_angle: None,
                    }
                ));
                inf_view.focused = false;
            }
            

            
            igEnd();
            // 메뉴가 닫힐 때는 igBegin이 false가 되므로 아래에서 처리
        } else {
            inf_view.focused = false;
        }
    }

    igSameLine(0.0,-1.0);

    //  pencil 버튼: tack 그리기
    if toolbar_button(const_cstr!("\u{f303}").as_ptr(), 
                      matches!(inf_view.action, Action::DrawingLine(_)), true ) {
        inf_view.action = Action::DrawingLine(None);
        inf_view.focused = false;
    }
    if igIsItemHovered(0) {
        igBeginTooltip();
        widgets::show_text("\u{f303} draw tracks (D)\nClick and drag to create new tracks.");
        igEndTooltip();
    }
    igSameLine(0.0,-1.0);

    // 되돌리기 버튼
    if toolbar_button(const_cstr!("\u{f0e2}").as_ptr(), false, analysis.can_undo()) {
        analysis.undo();
    }
    if igIsItemHovered(0) {
        igBeginTooltip();
        widgets::show_text("\u{f0e2} undo (CTRL-Z)\nUndo the previous action.");
        igEndTooltip();
    }
    igSameLine(0.0,-1.0);

    // 다시하기 버튼
    if toolbar_button(const_cstr!("\u{f01e}").as_ptr(), false, analysis.can_redo()) {
        analysis.redo();
    }
    if igIsItemHovered(0) {
        igBeginTooltip();
        widgets::show_text("\u{f01e} redo (CTRL-Y)\nRedo the previously undone action.");
        igEndTooltip();
    }
    }
}

fn toolbar_button(name :*const i8, selected :bool, enabled :bool) -> bool {
        unsafe {
        if selected {
            let c1 = ImVec4 { x: 0.4, y: 0.65,  z: 0.4, w: 1.0 };
            let c2 = ImVec4 { x: 0.5, y: 0.85, z: 0.5, w: 1.0 };
            let c3 = ImVec4 { x: 0.6, y: 0.9,  z: 0.6, w: 1.0 };
            igPushStyleColor(ImGuiCol__ImGuiCol_Button as _, c1);
            igPushStyleColor(ImGuiCol__ImGuiCol_ButtonHovered as _, c1);
            igPushStyleColor(ImGuiCol__ImGuiCol_ButtonActive as _, c1);
        }
        if !enabled {
            igPushDisable();
            igPushStyleVarFloat(ImGuiStyleVar__ImGuiStyleVar_Alpha as _, 0.5);

        }
        let clicked = igButton( name , ImVec2 { x: 0.0, y: 0.0 } );
        if !enabled {
            igPopStyleVar(1);
            igPopDisable();
        }
        if selected {
            igPopStyleColor(3);
        }
        clicked
    }
}

// 객체 삽입 버튼: 현재 선택된 객체의 아이콘 반환
fn get_current_object_icon(inf_view :&InfView) -> *const i8 {
    match &inf_view.action {
        Action::InsertObject(Some(obj)) => {
            if let Some(Function::Signal { has_distant, .. }) = obj.functions.first() {
                if *has_distant {
                    const_cstr!("\u{f5b0}").as_ptr() // Departure Signal
                } else {
                    const_cstr!("\u{f637}").as_ptr() // Home Signal
                }
            } else if obj.functions.contains(&Function::Detector) {
                const_cstr!("\u{f715}").as_ptr() // Section Insulator
            } else if obj.functions.contains(&Function::Switch { id: None }) {
                const_cstr!("\u{f126}").as_ptr() // Switch
            } else {
                const_cstr!("\u{f637}").as_ptr() // Default: Home Signal
            }
        },
        Action::SelectObjectType => {
            const_cstr!("\u{f637}").as_ptr() // Default: Home Signal
        },
        _ => {
            const_cstr!("\u{f637}").as_ptr() // Default: Home Signal
        },
    }
}

fn context_menu(analysis :&mut Analysis, 
                inf_view :&mut InfView,
                dispatch_view :&mut Option<DispatchView>,
                draw :&Draw, preview_route :&mut Option<usize>) {
    unsafe {
        static mut WAS_CTX_POPUP_OPEN: bool = false;
        let is_ctx_popup_open = igBeginPopup(const_cstr!("ctx").as_ptr(), 0 as _);
        if is_ctx_popup_open {
            inf_view.focused = true; // 팝업 열릴 때 true
            context_menu_contents(analysis, inf_view, dispatch_view, preview_route);
            igEndPopup();
        }
        // 팝업이 닫히는 순간 감지
        if WAS_CTX_POPUP_OPEN && !is_ctx_popup_open {
            inf_view.focused = false;
        }
        WAS_CTX_POPUP_OPEN = is_ctx_popup_open;

        if igIsItemHovered(0) && igIsMouseClicked(1, false) {
            if let Some((r,_)) = analysis.get_closest(inf_view.view.screen_to_world_ptc(draw.mouse)) {
                if !inf_view.selection.contains(&r) {
                    inf_view.selection = std::iter::once(r).collect();
                }
            }
            igOpenPopup(const_cstr!("ctx").as_ptr());
        }
    }
}

fn selection_title(inf_view :&InfView) -> String {
    if inf_view.selection.len() == 0 {
        format!("No selection")
    }
    else if inf_view.selection.len() == 1 {
        match inf_view.selection.iter().next() {
            Some(Ref::LineSeg(a,b)) => format!("Line segment from ({},{}) to ({},{})", a.x, a.y, b.x, b.y),
            Some(Ref::Node(pt)) => format!("Node at ({},{})", pt.x, pt.y),
            Some(Ref::Object(pt)) => format!("Object at ({:.1},{:.1})", pt.x as f32 / 10.0, pt.y as f32 / 10.0),
            None => unreachable!(),
        }
    }
    else {
        let (mut n_linesegs, mut n_nodes, mut n_objects) = (0,0,0);
        for x in inf_view.selection.iter() {
            match x {
                Ref::LineSeg(_,_) => { n_linesegs += 1; },
                Ref::Node(_) => { n_nodes += 1; },
                Ref::Object(_) => { n_objects += 1; },
            }
        }
        if n_nodes == 0 && n_objects == 0 { format!("Selection: {} line segments.", n_linesegs) }
        else if n_linesegs == 0 && n_objects == 0 { format!("Selection: {} nodes.", n_nodes) }
        else if n_linesegs == 0 && n_nodes == 0 { format!("Selection: {} objects.", n_objects) }
        else {
            format!("Selection: {} entities.", inf_view.selection.len())
        }
    }
}

fn context_menu_contents(analysis :&mut Analysis, inf_view :&mut InfView,
                         dispatch_view :&mut Option<DispatchView>,
                         preview_route :&mut Option<usize>) {
    unsafe {
    widgets::show_text(&selection_title(inf_view));

    widgets::sep();
    if !inf_view.selection.is_empty() {
        if igSelectable(const_cstr!("Delete").as_ptr(), false, 0 as _, ImVec2::zero()) {
            delete_selection(analysis, inf_view);
        }
    }
    widgets::sep();
    if inf_view.selection.len() == 1 {
        let thing = inf_view.selection.iter().nth(0).cloned().unwrap();
        context_menu_single(analysis, dispatch_view ,thing,preview_route);
    }
    }
}

fn context_menu_single(analysis :&mut Analysis, 
                       dispatch_view :&mut Option<DispatchView>,
                       thing :Ref, preview_route :&mut Option<usize>) {

    // Node editor
    if let Ref::Node(pt) = thing { 
        menus::node_editor(analysis, pt);
        widgets::sep();
    }

    // Object editor
    if let Ref::Object(pta) = thing { 
        menus::object_menu(analysis, pta);
        widgets::sep();
    }

    // Manual dispatch from boundaries and signals
    let action = menus::route_selector(analysis, dispatch_view, thing, preview_route);
    if let Some(routespec) = action {
        start_route(analysis, dispatch_view, routespec);
    }
    widgets::sep();

    // Add visits to auto dispatch
    menus::add_plan_visit(analysis, dispatch_view, thing);
}


pub fn delete_selection(analysis :&mut Analysis, inf_view :&mut InfView) {
    let mut new_model = analysis.model().clone();
    for x in inf_view.selection.drain() {
        new_model.delete(x);
    }
    analysis.set_model(new_model, None);
}

fn start_route(analysis :&mut Analysis, dispatch_view :&mut Option<DispatchView>, cmd :Command) {
    let mut model = analysis.model().clone();

    let (dispatch_idx,time) = match &dispatch_view {
        Some(DispatchView::Manual(m)) => (m.dispatch_idx, m.time),
        None | Some(DispatchView::Auto(_)) => {
            let name = generate_unique_dispatch_name(&model.dispatches);
            let dispatch_idx = model.dispatches.insert(Dispatch::new_empty(name));
            let time = 0.0;

            let mut m = ManualDispatchView::new(dispatch_idx);
            let autoplay = true; if autoplay { m.play = true; }
            *dispatch_view = Some(DispatchView::Manual(m));
            (dispatch_idx,time)
        },
    };

    let dispatch = model.dispatches.get_mut(dispatch_idx).unwrap();
    dispatch.insert(time as f64, cmd);
    analysis.set_model(model, None);
}

fn dispatch_view_ref(dispatch_view :&Option<DispatchView>) -> Option<DispatchRef> {
    match dispatch_view {
        Some(DispatchView::Manual(ManualDispatchView { dispatch_idx, time, .. })) => {
           Some((Ok(*dispatch_idx),*time as _))
        },
        Some(DispatchView::Auto(AutoDispatchView { plan_idx,
            dispatch: Some(ManualDispatchView { dispatch_idx, time, .. }), .. })) => {
           Some((Err((*plan_idx, *dispatch_idx)), *time as _))
        },
        _ => { return None; },
    }
}

fn is_id_duplicate(analysis: &Analysis, new_id: &str, exclude_position: Option<PtC>, function_type: &Function) -> bool {
    if new_id.is_empty() {
        return false; // Empty IDs are allowed
    }

    for (pos, obj) in analysis.model().objects.iter() {
        // Skip the object being edited (if any)
        if let Some(exclude_pos) = exclude_position {
            if round_coord(exclude_pos) == *pos {
                continue;
            }
        }

        // Check all functions in the object for IDs
        for function in &obj.functions {
            match (function, function_type) {
                // Only check for duplicates within the same function type
                (Function::Signal { id: Some(id), .. }, Function::Signal { .. }) => {
                    if id == new_id {
                        return true;
                    }
                },
                (Function::Switch { id: Some(id) }, Function::Switch { .. }) => {
                    if id == new_id {
                        return true;
                    }
                },
                _ => {} // Different function types can have the same ID
            }
        }
    }
    false
}

fn draw_id_input_dialog(analysis :&mut Analysis, inf_view :&mut InfView) {
    unsafe {
        if let Some(ref mut id_input) = inf_view.id_input {
            inf_view.focused = true;
            // 이미 열릴 때 true, 닫힐 때 false 처리되어 있음 (유지)
            // 중앙에 다이얼로그 표시
            let display_size = (*igGetIO()).DisplaySize;
            igSetNextWindowPos(ImVec2 { x: display_size.x/2.0, y: display_size.y/2.0}, 
                               ImGuiCond__ImGuiCond_Appearing as _, ImVec2 { x: 0.5, y: 0.5 });
            igSetNextWindowSize(ImVec2 { x: 300.0, y: 150.0}, ImGuiCond__ImGuiCond_Appearing as _);
            
            let mut open = true;
            let mut should_confirm = false;
            let mut should_cancel = false;
            
            // Check if current ID is duplicate (only within same function type)
            let is_duplicate = is_id_duplicate(analysis, &id_input.id, Some(id_input.position), &id_input.function_type);

            if igBegin(const_cstr!("Signal ID").as_ptr(), &mut open as _, 0 as _) {
                widgets::show_text("Enter signal ID:");
                
                // ID 입력 필드
                let mut id_buffer = id_input.id.clone().into_bytes();
                id_buffer.push(0);
                id_buffer.extend((0..50).map(|_| 0u8));

                let ok_enabled = !is_duplicate || id_input.id.is_empty();
                if !ok_enabled {
                    igPushStyleVarFloat(ImGuiStyleVar__ImGuiStyleVar_Alpha as _, 0.5);
                }

                if igInputText(const_cstr!("##id").as_ptr(), 
                              id_buffer.as_mut_ptr() as *mut _, 
                              id_buffer.len(),
                              ImGuiInputTextFlags__ImGuiInputTextFlags_EnterReturnsTrue as _,
                                None, std::ptr::null_mut()) {
                    if ok_enabled {
                        should_confirm=true;
                    }
                }
                let terminator = id_buffer.iter().position(|&c| c == 0).unwrap();
                id_buffer.truncate(terminator);
                id_input.id = String::from_utf8_unchecked(id_buffer);
                
                // Show duplicate warning if applicable
                if is_duplicate && !id_input.id.is_empty() {
                    igPushStyleColor(ImGuiCol__ImGuiCol_Text as _, ImVec4 { x: 1.0, y: 0.0, z: 0.0, w: 1.0 }); // Red color
                    widgets::show_text("Warning: This ID already exists!");
                    igPopStyleColor(1);
                }

                igSpacing();
                igSpacing();
                
                // 확인 버튼 - disable if duplicate
                if igButton(const_cstr!("OK").as_ptr(), ImVec2 { x: 80.0, y: 0.0 }) && ok_enabled {
                    should_confirm = true;
                }
                
                if !ok_enabled {
                    igPopStyleVar(1);
                }

                igSameLine(0.0, 10.0);
                
                // 취소 버튼
                if igButton(const_cstr!("Cancel").as_ptr(), ImVec2 { x: 80.0, y: 0.0 }) {
                    should_cancel = true;
                }
                
                // Enter 키로 확인, Escape 키로 취소
                //if igIsKeyPressed(13 as _, false) { // Enter
                    //should_confirm = true;
                //}
                
                if !igIsItemActive() && igIsKeyPressed(ImGuiKey__ImGuiKey_Escape as _, false) { // Escape
                    should_cancel = true;
                }
            }
            igEnd();
            
            if !open {
                should_cancel = true;
            }
            
            // 다이얼로그가 닫힌 후 처리
            if should_confirm {
                // 데이터를 복사해서 처리
                let mut object = id_input.object.clone();
                let id = id_input.id.clone();
                let position = id_input.position;
                // 이름을 Function에 설정
                if let Some(Function::Signal { has_distant, .. }) = object.functions.first() {
                    let new_function = Function::Signal {
                        has_distant: *has_distant, 
                        id: Some(id.clone())
                    };
                    object.functions = vec![new_function];
                } else if let Some(Function::Switch { .. }) = object.functions.first() {
                    let new_function = Function::Switch { id: Some(id.clone()) };
                    object.functions = vec![new_function];
                } else if let Some(Function::TrackLabel { .. }) = object.functions.first() {
                    // TrackLabel은 단순한 표시 객체로 생성
                    let new_function = Function::TrackLabel { 
                        id: Some(id.clone()),
                        display_text: Some(id.clone())
                    };
                    
                    // TrackLabel 생성 시 바운더리 제한 적용
                    let mut track_label = Object {
                        loc: position,
                        functions: vec![new_function.clone()],
                        ..Object::default()
                    };
                    
                    // 바운더리 제한을 적용하여 최종 위치 결정
                    if let Some(_factor) = track_label.move_to_with_factor(analysis.model(), analysis, position) {
                        // 바운더리 제한이 적용된 위치로 생성
                        analysis.edit_model(|m| {
                            m.objects.insert(round_coord(track_label.loc), track_label);
                            None
                        });
                    } else {
                        // 바운더리 제한이 적용되지 않은 경우 원래 위치로 생성
                        analysis.edit_model(|m| {
                            m.objects.insert(round_coord(position), track_label);
                            None
                        });
                    }
                } else {
                    // TrackLabel이 아닌 경우 기존 로직 사용
                    analysis.edit_model(|m| {
                        m.objects.insert(round_coord(position), object);
                        None
                    });
                }
                
                // TrackLabel이 생성된 경우 클릭 불가능한 상태로 설정
                if matches!(id_input.function_type, Function::TrackLabel { .. }) {
                    inf_view.track_label_drag.id_clickable = false;
                    inf_view.track_label_drag.creation_time = Some(unsafe { igGetTime() });
                }
                
                // ID 입력 상태 초기화
                inf_view.id_input = None;
                inf_view.focused = false; // 입력창 닫힐 때 포커스 false
            } else if should_cancel {
                // ID 입력 상태 초기화
                inf_view.id_input = None;
                inf_view.focused = false; // 입력창 닫힐 때 포커스 false
            }
        }
    }
}

// 트랙 생성 시 ID 입력창을 띄우고, 입력받은 ID로 TrackLabel을 생성하는 함수
fn create_track_with_id_input(analysis: &mut Analysis, inf_view: &mut InfView, track_mid: PtC, suggested_id: Option<String>) {

    
    // TrackLabelObject 생성 준비 (단순한 표시용)
    let object = Object {
        loc: track_mid,
        tangent: glm::vec2(1, 0), // i32로 수정
        functions: vec![Function::TrackLabel { 
            id: None,
            display_text: None
        }],
        id: None,
        signal_props: None,
        switch_props: None,
        placed_angle: None,
    };
    inf_view.id_input = Some(IdInputState {
        object,
        id: suggested_id.unwrap_or_default(),
        position: track_mid,
        function_type: Function::TrackLabel { 
            id: None,
            display_text: None
        },
    });
    inf_view.focused = true;
}



// detector가 track 위에 올려졌을 때 해당 track을 찾는 함수
fn find_track_at_position(analysis: &Analysis, detector_pos: PtC) -> Option<(PtA, Object)> {

    let model = analysis.model();
    
    // detector 위치 근처의 track label object 찾기
    let mut closest_track = None;
    let mut min_distance = f32::INFINITY;
    
    for (pta, obj) in model.objects.iter() {
        if obj.functions.iter().any(|f| matches!(f, Function::TrackLabel { .. })) {
            // detector와 track label 사이의 거리 계산
            let distance = glm::distance(&obj.loc, &detector_pos);
            
            // 가장 가까운 TrackLabel 찾기
            if distance < min_distance {
                min_distance = distance;
                closest_track = Some((*pta, obj.clone()));
            }
        }
    }
    
    // 거리 임계값을 대폭 늘려서 더 멀리 있는 TrackLabel도 매칭되도록 함
    if let Some(track) = closest_track {
        if min_distance < 20.0 {
            return Some(track);
        }
    }
    None
}

// track segment를 클릭했을 때 해당 track의 TrackLabel을 찾는 함수
fn find_track_label_for_segment(analysis: &Analysis, segment: (Pt, Pt)) -> Option<PtA> {

    let model = analysis.model();

    // 해당 segment에 연결된 TrackLabel 찾기
    for (pta, obj) in model.objects.iter() {
                                if let Some(Function::TrackLabel { id: _, display_text: _ }) =
            obj.functions.first() {
            // TrackLabel은 이제 단순한 표시 객체이므로 segment 매칭은 하지 않음
            // 대신 위치 기반으로 가장 가까운 TrackLabel을 반환
            return Some(*pta);
        }
    }


    None
}

// 두 segment가 같은 track을 나타내는지 확인하는 함수
fn segments_match(segment1: (Pt, Pt), segment2: (Pt, Pt)) -> bool {

    
    // 정확히 일치하는 경우
    if (segment1.0 == segment2.0 && segment1.1 == segment2.1) ||
       (segment1.0 == segment2.1 && segment1.1 == segment2.0) {
        return true;
    }
    
    // segment1이 segment2의 부분집합인지 확인
    // segment2가 더 긴 track이고, segment1이 그 안에 포함되는지 확인
    let (s1_start, s1_end) = if segment1.0.x <= segment1.1.x {
        (segment1.0, segment1.1)
    } else {
        (segment1.1, segment1.0)
    };
    
    let (s2_start, s2_end) = if segment2.0.x <= segment2.1.x {
        (segment2.0, segment2.1)
    } else {
        (segment2.1, segment2.0)
    };
    

    // segment1이 segment2의 범위 안에 있는지 확인
    let x_in_range = s1_start.x >= s2_start.x && s1_end.x <= s2_end.x;
    let y_match = s1_start.y == s2_start.y && s1_end.y == s2_end.y;
    
    x_in_range && y_match
}

// track을 논리적으로 분할하는 함수
fn split_track_at_detector(analysis: &mut Analysis, inf_view: &mut InfView, 
                          detector_pos: PtC, original_track: (PtA, Object)) {

    let (original_pta, original_obj) = original_track;
    
    // 기존 track의 ID 추출
    let original_id = if let Some(Function::TrackLabel { id, display_text: _ }) = original_obj.functions.first() {
        id.as_ref().unwrap_or(&String::new()).clone()
    } else {
        String::new()
    };
    

    
    // 분할된 두 track의 중간 위치 계산
    let track_start = original_obj.loc;
    
    // 두 번째 track (detector 이후) - 새로운 TrackLabel 생성
    let track_direction = glm::normalize(&(detector_pos - track_start));
    let track_length = glm::distance(&track_start, &detector_pos);
    let second_track_end = detector_pos + track_direction * track_length;
    
    let second_track_mid = glm::vec2(
        (detector_pos.x + second_track_end.x) / 2.0,
        (detector_pos.y + second_track_end.y) / 2.0
    );
    

    
    // 두 번째 track ID 입력창만 띄우기 (첫 번째는 기존 ID 유지)
    // 기존 ID가 비어있으면 기본 ID 사용
    let base_id = if original_id.is_empty() { "TRACK".to_string() } else { original_id };
    let second_track_id = if base_id.ends_with("_B") {
        // 이미 "_B" 접미사가 있으면 "_C"로 변경
        base_id.trim_end_matches("_B").to_string() + "_C"
    } else {
        base_id + "_B"
    };
    

    create_track_with_id_input(analysis, inf_view, second_track_mid, Some(second_track_id));
}

// ID 텍스트 드래그 감지 및 처리
fn handle_id_drag(config: &Config, analysis: &mut Analysis, inf_view: &mut InfView, draw: &Draw) {
    unsafe {
        if inf_view.focused { 
            return; 
        }
        
        let io = igGetIO();
        let mouse_pos = inf_view.view.screen_to_world_ptc(draw.mouse);
        let is_mouse_pressed = (*io).MouseDown[0];
        
        // 드래그 중이 아닐 때: ID 텍스트 클릭 감지
        if !inf_view.track_label_drag.is_dragging {
            // 마우스가 방금 눌렸을 때만 클릭으로 감지 (이전 프레임에는 눌려있지 않았고, 현재 프레임에 눌려있음)
            if is_mouse_pressed && !inf_view.track_label_drag.mouse_was_pressed {
                let found_id = find_id_at_position(analysis, mouse_pos);
                if let Some((pta, id)) = found_id {
                    // ID가 클릭 가능한 상태일 때만 드래그 시작 (TrackLabel 선택하지 않음)
                    if inf_view.track_label_drag.id_clickable {
                        // TrackLabel 생성 후 0.5초 동안 클릭 방지
                        let current_time = unsafe { igGetTime() };
                        let can_click = if let Some(creation_time) = inf_view.track_label_drag.creation_time {
                            current_time - creation_time > 0.5
                        } else {
                            true
                        };
                        
                        if can_click {
                            inf_view.track_label_drag.is_dragging = true;
                            inf_view.track_label_drag.dragged_id = Some(id.clone());
                            inf_view.track_label_drag.source_pta = Some(pta);
                            inf_view.track_label_drag.drag_start_pos = Some(mouse_pos);
                            inf_view.track_label_drag.drag_preview_pos = Some(mouse_pos);
                        }
                    }
                }
            }
            
            // 마우스가 놓였을 때 ID를 클릭 가능한 상태로 변경 (드래그 중이 아닐 때만)
            if !is_mouse_pressed && inf_view.track_label_drag.mouse_was_pressed {
                if !inf_view.track_label_drag.id_clickable {
                    inf_view.track_label_drag.id_clickable = true;
                }
            }
        } else {
            // 드래그 중일 때: 미리보기 위치만 업데이트 (실제 이동은 하지 않음)
            inf_view.track_label_drag.drag_preview_pos = Some(mouse_pos);
            
            // 드래그 종료 감지 (마우스 버튼을 놓을 때만 실제 이동)
            if !is_mouse_pressed && inf_view.track_label_drag.mouse_was_pressed {
                if let Some(dragged_id) = &inf_view.track_label_drag.dragged_id {
                    if let Some(source_pta) = inf_view.track_label_drag.source_pta {
                        // 드롭 위치에서 새로운 TrackLabel 생성
                        handle_id_drop(analysis, inf_view, mouse_pos, dragged_id.clone(), source_pta);
                    }
                }
                
                // 드래그 상태 초기화
                inf_view.track_label_drag.is_dragging = false;
                inf_view.track_label_drag.dragged_id = None;
                inf_view.track_label_drag.source_pta = None;
                inf_view.track_label_drag.drag_start_pos = None;
                inf_view.track_label_drag.drag_preview_pos = None;
                inf_view.track_label_drag.drag_offset = glm::vec2(0.0, 0.0);
                // 드래그가 완료되면 ID를 다시 클릭 가능한 상태로 설정
                inf_view.track_label_drag.id_clickable = true;
            }
        }
        
        // 마우스 상태 업데이트
        inf_view.track_label_drag.mouse_was_pressed = is_mouse_pressed;
    }
}

// ID 드롭 처리
fn handle_id_drop(analysis: &mut Analysis, inf_view: &mut InfView, drop_pos: PtC, id: String, source_pta: PtA) {
    // 새로운 TrackLabel 생성 (단순한 표시 객체)
    let mut new_track_label = Object {
        loc: drop_pos,
        functions: vec![Function::TrackLabel { 
            id: Some(id.clone()),
            display_text: Some(id.clone())
        }],
        ..Object::default()
    };
    
    // Track 주변에 자동 배치
    let (final_loc, factor) = if let Some(factor) = new_track_label.move_to_with_factor(analysis.model(), analysis, drop_pos) {
        (new_track_label.loc, factor)
    } else {
        (drop_pos, 0.0)
    };
    
    analysis.edit_model(|m| {
        m.objects.insert(round_coord(final_loc), new_track_label);
        None
    });
    
    // 원본에서 ID 제거 (새 TrackLabel 생성 후)
    analysis.edit_model(|m| {
        if let Some(obj) = m.objects.get_mut(&source_pta) {
            for function in &mut obj.functions {
                if let Function::TrackLabel { id: ref mut existing_id, display_text: ref mut display } = function {
                    if existing_id.as_ref() == Some(&id) {
                        *existing_id = None;
                        *display = None;
                        break;
                    }
                }
            }
        }
        None
    });
    
    // ID 이동 후 클릭 불가능한 상태로 다시 설정
    inf_view.track_label_drag.id_clickable = false;
    inf_view.track_label_drag.creation_time = Some(unsafe { igGetTime() });
}

// 마우스 위치에서 ID 찾기
fn find_id_at_position(analysis: &Analysis, mouse_pos: PtC) -> Option<(PtA, String)> {
    let model = analysis.model();
    let mut closest_id = None;
    let mut min_distance = f32::INFINITY;
    let click_threshold = 1.0; // ID 텍스트 클릭 감지 거리를 더 엄격하게 설정 (1.5 -> 1.0)
    
    for (pta, obj) in model.objects.iter() {
        if obj.functions.iter().any(|f| matches!(f, Function::TrackLabel { .. })) {
            let distance = glm::distance(&obj.loc, &mouse_pos);
            
            if distance < min_distance && distance < click_threshold {
                // TrackLabel의 ID 찾기
                for function in &obj.functions {
                    if let Function::TrackLabel { id, display_text } = function {
                        if let Some(id_str) = id {
                            min_distance = distance;
                            closest_id = Some((*pta, id_str.clone()));
                        }
                    }
                }
            }
        }
    }
    
    closest_id
}

