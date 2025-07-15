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

#[derive(Copy,Clone,Debug)]
pub enum Highlight {
    Ref(Ref),
    Tvd(usize),
}

fn show_object_properties_panel(analysis: &mut Analysis, inf_view: &InfView) {
    use backend_glfw::imgui::*;
    use crate::gui::widgets;
    if inf_view.selection.len() != 1 {
        return;
    }
    if let Some(Ref::Object(pta)) = inf_view.selection.iter().next() {
        let (is_signal, is_switch, current_id, signal_props, switch_props, placed_angle) = match analysis.model().objects.get(pta) {
            Some(obj) => {
                let is_signal = obj.functions.iter().any(|f| matches!(f, Function::Signal { .. }));
                let is_switch = obj.functions.iter().any(|f| matches!(f, Function::Switch { .. }));
                let mut current_id = String::new();
                if let Some(id) = obj.id.as_ref() {
                    current_id = id.clone();
                } else {
                    for f in &obj.functions {
                        match f {
                            Function::Signal { id: Some(signal_id), .. } => {
                                current_id = signal_id.clone();
                                break;
                            },
                            Function::Switch { id: Some(switch_id) } => {
                                current_id = switch_id.clone();
                                break;
                            },
                            _ => {}
                        }
                    }
                }
                (is_signal, is_switch, current_id, obj.signal_props.clone(), obj.switch_props.clone(), obj.placed_angle)
            }
            None => return,
        };

        if is_signal || is_switch {
            unsafe {
                let io = igGetIO();
                let display_size = (*io).DisplaySize;
                let panel_size = ImVec2 { x: 300.0, y: 180.0 };
                let panel_pos = ImVec2 { x: display_size.x - panel_size.x - 20.0, y: 20.0 };
                igSetNextWindowPos(panel_pos, ImGuiCond__ImGuiCond_Always as _, ImVec2 { x: 0.0, y: 0.0 });
                igSetNextWindowSize(panel_size, ImGuiCond__ImGuiCond_Always as _);
                let mut open = true;
                if igBegin(const_cstr!("Object Properties").as_ptr(), &mut open as *mut bool, 0) {
                    if !open {
                        let selection = &mut (*(inf_view as *const _ as *mut InfView)).selection;
                        selection.clear();
                        igEnd();
                        return;
                    }
                    // Object Type 표시
                    let object_type = if is_signal { "Signal" } else if is_switch { "Switch" } else { "Unknown" };
                    widgets::show_text(&format!("Object Type: {}", object_type));
                    // ID 편집 기능
                    widgets::show_text("ID:");
                    igSameLine(0.0, 5.0);
                    // ID 입력 필드
                    let mut id_buffer = current_id.clone().into_bytes();
                    id_buffer.push(0);
                    id_buffer.extend((0..50).map(|_| 0u8));
                    if igInputText(const_cstr!("##object_id").as_ptr(), id_buffer.as_mut_ptr() as *mut _, id_buffer.len(), 0 as _, None, std::ptr::null_mut()) {
                        let terminator = id_buffer.iter().position(|&c| c == 0).unwrap();
                        id_buffer.truncate(terminator);
                        let new_id = String::from_utf8_unchecked(id_buffer);
                        if new_id != current_id {
                            analysis.edit_model(|m| {
                                if let Some(obj) = m.objects.get_mut(pta) {
                                    for f in &mut obj.functions {
                                        match f {
                                            Function::Signal { has_distant, id: ref mut signal_id } => {
                                                *signal_id = Some(new_id.clone());
                                            },
                                            Function::Switch { id: ref mut switch_id } => {
                                                *switch_id = Some(new_id.clone());
                                            },
                                            _ => {}
                                        }
                                    }
                                }
                                None
                            });
                        }
                    }
                    if is_signal {
                        if let Some(props) = &signal_props {
                            widgets::show_text("Type:");
                            igSameLine(0.0, 5.0);
                            let mut signal_type = props.signal_type.clone();
                            let signal_type_str = match signal_type {
                                SignalType::Home => "Home",
                                SignalType::Departure => "Departure",
                                SignalType::Shunting => "Shunting",
                            };
                            let signal_type_cstr = std::ffi::CString::new(signal_type_str).unwrap();
                            if igBeginCombo(const_cstr!("##signal_type").as_ptr(), signal_type_cstr.as_ptr(), 0) {
                                if igSelectable(const_cstr!("Home").as_ptr(), matches!(signal_type, SignalType::Home), 0 as _, ImVec2::zero()) {
                                    signal_type = SignalType::Home;
                                }
                                if igSelectable(const_cstr!("Departure").as_ptr(), matches!(signal_type, SignalType::Departure), 0 as _, ImVec2::zero()) {
                                    signal_type = SignalType::Departure;
                                }
                                if igSelectable(const_cstr!("Shunting").as_ptr(), matches!(signal_type, SignalType::Shunting), 0 as _, ImVec2::zero()) {
                                    signal_type = SignalType::Shunting;
                                }
                                igEndCombo();
                            }
                            widgets::show_text("Kind:");
                            igSameLine(0.0, 5.0);
                            let mut signal_kind = props.signal_kind.clone();
                            let signal_kind_str = match signal_kind {
                                SignalKind::Two => "Two",
                                SignalKind::Three => "Three",
                                SignalKind::Four => "Four",
                            };
                            let signal_kind_cstr = std::ffi::CString::new(signal_kind_str).unwrap();
                            if igBeginCombo(const_cstr!("##signal_kind").as_ptr(), signal_kind_cstr.as_ptr(), 0) {
                                if igSelectable(const_cstr!("Two").as_ptr(), matches!(signal_kind, SignalKind::Two), 0 as _, ImVec2::zero()) {
                                    signal_kind = SignalKind::Two;
                                }
                                if igSelectable(const_cstr!("Three").as_ptr(), matches!(signal_kind, SignalKind::Three), 0 as _, ImVec2::zero()) {
                                    signal_kind = SignalKind::Three;
                                }
                                if igSelectable(const_cstr!("Four").as_ptr(), matches!(signal_kind, SignalKind::Four), 0 as _, ImVec2::zero()) {
                                    signal_kind = SignalKind::Four;
                                }
                                igEndCombo();
                            }
                            widgets::show_text("Direction:");
                            igSameLine(0.0, 5.0);
                            let mut direction = props.direction;
                            let direction_str = match direction {
                                TrackDirection::Left => "Left",
                                TrackDirection::Right => "Right",
                            };
                            let direction_cstr = std::ffi::CString::new(direction_str).unwrap();
                            if igBeginCombo(const_cstr!("##signal_direction").as_ptr(), direction_cstr.as_ptr(), 0) {
                                if igSelectable(const_cstr!("Left").as_ptr(), matches!(direction, TrackDirection::Left), 0 as _, ImVec2::zero()) {
                                    direction = TrackDirection::Left;
                                }
                                if igSelectable(const_cstr!("Right").as_ptr(), matches!(direction, TrackDirection::Right), 0 as _, ImVec2::zero()) {
                                    direction = TrackDirection::Right;
                                }
                                igEndCombo();
                            }
                            if signal_type != props.signal_type || signal_kind != props.signal_kind || direction != props.direction {
                                analysis.edit_model(|m| {
                                    if let Some(obj) = m.objects.get_mut(pta) {
                                        if let Some(props) = &mut obj.signal_props {
                                            props.signal_type = signal_type;
                                            props.signal_kind = signal_kind;
                                            props.direction = direction;
                                        }
                                    }
                                    None
                                });
                            }

                        } else {
                            widgets::show_text("No signal properties set.");
                        }
                    }
                    if is_switch {
                        if let Some(props) = &switch_props {
                            widgets::show_text("Type:");
                            igSameLine(0.0, 5.0);
                            let mut switch_type = props.switch_type.clone();
                            let switch_type_str = match switch_type {
                                SwitchType::Single => "Single",
                                SwitchType::Double => "Double",
                            };
                            let switch_type_cstr = std::ffi::CString::new(switch_type_str).unwrap();
                            if igBeginCombo(const_cstr!("##switch_type").as_ptr(), switch_type_cstr.as_ptr(), 0) {
                                if igSelectable(const_cstr!("Single").as_ptr(), matches!(switch_type, SwitchType::Single), 0 as _, ImVec2::zero()) {
                                    switch_type = SwitchType::Single;
                                }
                                if igSelectable(const_cstr!("Double").as_ptr(), matches!(switch_type, SwitchType::Double), 0 as _, ImVec2::zero()) {
                                    switch_type = SwitchType::Double;
                                }
                                igEndCombo();
                            }
                            if switch_type != props.switch_type {
                                analysis.edit_model(|m| {
                                    if let Some(obj) = m.objects.get_mut(pta) {
                                        if let Some(props) = &mut obj.switch_props {
                                            props.switch_type = switch_type;
                                        }
                                    }
                                    None
                                });
                            }
                        } else {
                            widgets::show_text("No switch properties set.");
                        }
                    }
                }
                igEnd();
            }
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
        // Add the properties panel for selected object
        show_object_properties_panel(analysis, inf_view);
        let pos_after = igGetCursorPos_nonUDT2().into();
        let framespace = igGetFrameHeightWithSpacing() - igGetFrameHeight();
        igSetCursorPos(pos_before + ImVec2 { x: 2.0*framespace, y: 2.0*framespace });
        inf_toolbar(analysis, inf_view);
        igSetCursorPos(pos_after);
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
                            inf_view.selection.insert(r);
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
            inf_view.focused = true;
            return;
        }
        if let Some(mut obj) = obj {
            // 객체의 초기 위치를 마우스 위치로 설정
            obj.loc = inf_view.view.screen_to_world_ptc(draw.mouse);
            
            let moved = obj.move_to(analysis.model(), analysis, inf_view.view.screen_to_world_ptc(draw.mouse));
            
            // 미리보기 시에는 잘 보이는 색상 사용 (감지기는 배치 후에 배경색과 같아짐)
            let preview_color = config.color_u32(RailUIColorName::CanvasSymbol);
            
            obj.draw(draw.pos,&inf_view.view,draw.draw_list,
                    preview_color,&[],&config);

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
                        println!("interact_insert: focused == {:?}", inf_view.focused);
                        // MainSignal 또는 Switch인 경우 이름 입력 다이얼로그 표시
                        if let Some(Function::Signal { .. }) = obj.functions.first() {
                            match obj.signal_props.as_ref().map(|props| props.signal_type.clone()) {
                                Some(SignalType::Home) | Some(SignalType::Departure) => {
                                    println!("obj.loc a: {:?}", obj.loc);
                                    inf_view.id_input = Some(IdInputState {
                                        object: obj.clone(),  // move_to가 호출된 후의 obj 사용
                                        id: String::new(),
                                        position: obj.loc.clone(),
                                    });
                                },
                                Some(SignalType::Shunting) => {
                                    println!("obj.loc b: {:?}", obj.loc);
                                    inf_view.id_input = Some(IdInputState {
                                        object: obj.clone(),  // move_to가 호출된 후의 obj 사용
                                        id: String::new(),
                                        position: obj.loc.clone(),
                                    });
                                },
                                _ => {}
                            }
                        } else if obj.functions.iter().any(|f| matches!(f, Function::Switch { .. })) {
                            println!("obj.loc c: {:?}", obj.loc);
                            inf_view.id_input = Some(IdInputState {
                                object: obj.clone(),  // move_to가 호출된 후의 obj 사용
                                id: String::new(),
                                position: obj.loc.clone(),
                            });
                        } else {
                        // 그 외 객체는 바로 배치
                    analysis.edit_model(|m| {
                        m.objects.insert(round_coord(obj.loc), obj.clone());
                        None
                    });
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
                        switch_props: None,
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
            
            if igBegin(const_cstr!("Signal ID").as_ptr(), &mut open as _, 0 as _) {
                widgets::show_text("Enter signal ID:");
                
                // ID 입력 필드
                let mut id_buffer = id_input.id.clone().into_bytes();
                id_buffer.push(0);
                id_buffer.extend((0..50).map(|_| 0u8));
                
                if igInputText(const_cstr!("##id").as_ptr(), 
                              id_buffer.as_mut_ptr() as *mut _, 
                              id_buffer.len(),
                              0 as _, None, std::ptr::null_mut()) {
                    let terminator = id_buffer.iter().position(|&c| c == 0).unwrap();
                    id_buffer.truncate(terminator);
                    id_input.id = String::from_utf8_unchecked(id_buffer);
                }
                
                igSpacing();
                igSpacing();
                
                // 확인 버튼
                if igButton(const_cstr!("OK").as_ptr(), ImVec2 { x: 80.0, y: 0.0 }) {
                    should_confirm = true;
                }
                
                igSameLine(0.0, 10.0);
                
                // 취소 버튼
                if igButton(const_cstr!("Cancel").as_ptr(), ImVec2 { x: 80.0, y: 0.0 }) {
                    should_cancel = true;
                }
                
                // Enter 키로 확인, Escape 키로 취소
                if igIsKeyPressed(13 as _, false) { // Enter
                    should_confirm = true;
                }
                
                if igIsKeyPressed(27 as _, false) { // Escape
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
                println!("draw_id_input_dialog: focused == {:?}", inf_view.focused);
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
                }
                // Object를 모델에 추가
                analysis.edit_model(|m| {
                    m.objects.insert(round_coord(position), object);
                    None
                });
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

