use backend_glfw::imgui::*;
use const_cstr::*;
use crate::gui::widgets;
use crate::document::Document;
use crate::document::analysis::Analysis;
use crate::document::infview::InfView;
use crate::document::objects::*;
use crate::document::model::*;

pub struct SidebarWindow {
    pub is_open: bool,
    pub open_config: bool,
    // Docking state
    pub is_docked: bool,
    pub dock_position: DockPosition,
    pub dock_threshold: f32, // Distance from edge to trigger docking
    // Section states
    pub tools_open: bool,
    pub props_open: bool,
    pub info_open: bool,
    pub settings_open: bool,
    pub help_open: bool,
    // Movement tracking
    pub previous_pos: Option<ImVec2>,
    pub is_moving: bool,
}

#[derive(Clone, Copy, PartialEq)]
pub enum DockPosition {
    None,
    Left,
    Right,
}

impl SidebarWindow {
    pub fn new() -> Self {
        Self {
            is_open: true, // 기본적으로 켜져 있음
            open_config: false,
            is_docked: true, // 기본적으로 dock되어 있음
            dock_position: DockPosition::Right, // 오른쪽에 dock
            dock_threshold: 50.0, // 50 pixels from edge to trigger docking
            // Section states - Tools는 기본적으로 열려있음
            tools_open: true,
            props_open: false,
            info_open: false,
            settings_open: false,
            help_open: false,
            // Movement tracking
            previous_pos: None,
            is_moving: false,
        }
    }

    pub fn render(&mut self, document: &mut Document) {
        if !self.is_open {
            return;
        }

        unsafe {
            let io = igGetIO();
            let display_size = (*io).DisplaySize;
            
            // Check for docking
            self.check_docking(display_size);
            
            // Window flags
            let mut window_flags = ImGuiWindowFlags__ImGuiWindowFlags_NoCollapse as i32;
            
            if self.is_docked {
                // Docked window settings
                window_flags |= ImGuiWindowFlags__ImGuiWindowFlags_NoMove as i32;
                window_flags |= ImGuiWindowFlags__ImGuiWindowFlags_NoResize as i32;
                
                match self.dock_position {
                    DockPosition::Right => {
                        let docked_pos = ImVec2 { x: display_size.x - 350.0, y: 0.0 };
                        let docked_size = ImVec2 { x: 350.0, y: display_size.y };
                        igSetNextWindowPos(docked_pos, ImGuiCond__ImGuiCond_Always as _, ImVec2 { x: 0.0, y: 0.0 });
                        igSetNextWindowSize(docked_size, ImGuiCond__ImGuiCond_Always as _);
                    },
                    DockPosition::Left => {
                        let docked_pos = ImVec2 { x: 0.0, y: 0.0 };
                        let docked_size = ImVec2 { x: 350.0, y: display_size.y };
                        igSetNextWindowPos(docked_pos, ImGuiCond__ImGuiCond_Always as _, ImVec2 { x: 0.0, y: 0.0 });
                        igSetNextWindowSize(docked_size, ImGuiCond__ImGuiCond_Always as _);
                    },
                    DockPosition::None => {}
                }
            } else {
                // Floating window settings
                igSetNextWindowSize(ImVec2 { x: 350.0, y: 600.0 }, ImGuiCond__ImGuiCond_FirstUseEver as _);
                igSetNextWindowPos(ImVec2 { x: 800.0, y: 100.0 }, ImGuiCond__ImGuiCond_FirstUseEver as _, ImVec2 { x: 0.0, y: 0.0 });
            }

            if igBegin(const_cstr!("Sidebar").as_ptr(), &mut self.is_open as *mut bool, window_flags) {
                // Tools Section
                if render_section_header(const_cstr!("Tools").as_ptr(), 0.1, 0.2, 0.3, &mut self.tools_open) {
                    render_tools_tab(document);
                }

                // Object Properties Section
                if render_section_header(const_cstr!("Object Properties").as_ptr(), 0.2, 0.3, 0.4, &mut self.props_open) {
                    render_props_tab(document);
                }
                
                // File Information Section
                if render_section_header(const_cstr!("File Information").as_ptr(), 0.3, 0.4, 0.5, &mut self.info_open) {
                    render_info_tab(document);
                }
                
                // Settings Section
                if render_section_header(const_cstr!("Settings").as_ptr(), 0.4, 0.5, 0.6, &mut self.settings_open) {
                    render_settings_tab(self, document);
                }
                
                // Help Section
                if render_section_header(const_cstr!("Help").as_ptr(), 0.5, 0.6, 0.7, &mut self.help_open) {
                    render_help_tab();
                }
                
                // Undock button at the bottom when docked
                if self.is_docked {
                    igSpacing();
                    igSeparator();
                    if igButton(const_cstr!("Undock").as_ptr(), ImVec2 { x: 0.0, y: 0.0 }) {
                        self.is_docked = false;
                        self.dock_position = DockPosition::None;
                    }
                }
            }
            igEnd();
        }
    }
    
    fn check_docking(&mut self, display_size: ImVec2) {
        if self.is_docked {
            return; // Already docked
        }
        
        let io = unsafe { igGetIO() };
        let mouse_pos: ImVec2 = unsafe { igGetMousePos_nonUDT2().into() };
        
        // Check if mouse left button is held down (dragging)
        if unsafe { !(*io).MouseDown[0] } {
            return; // Not dragging
        }
        
        // Check if we're actually dragging
        if unsafe { !igIsMouseDragging(0, 0.0) } {
            return; // Not dragging
        }
        
        // Check if the sidebar window is being dragged (hovered and dragging)
        if unsafe { !igIsWindowHovered(0) } {
            return; // Not dragging the sidebar window
        }
        
        // Check if mouse is near left edge
        if mouse_pos.x <= self.dock_threshold {
            self.is_docked = true;
            self.dock_position = DockPosition::Left;
        }
        // Check if mouse is near right edge
        else if mouse_pos.x >= display_size.x - self.dock_threshold {
            self.is_docked = true;
            self.dock_position = DockPosition::Right;
        }
    }
    

}



/// 정보 탭 렌더링
fn render_info_tab(document: &crate::document::Document) {
    unsafe {
        if let Some(filename) = &document.fileinfo.filename {
            widgets::long_text(&format!("Filename: {}", filename));
        } else {
            widgets::long_text("Filename: (Not saved)");
        }
        widgets::long_text(&format!("Saved: {}", !document.fileinfo.unsaved));
        
        igSpacing();
        igText(const_cstr!("Model Information").as_ptr());
        igSeparator();
        
        let model = document.analysis.model();
        widgets::long_text(&format!("Line segments: {}", model.linesegs.len()));
        widgets::long_text(&format!("Objects: {}", model.objects.len()));
        widgets::long_text(&format!("Vehicles: {}", model.vehicles.data().len()));
    }
}

/// 도구 탭 렌더링
fn render_tools_tab(document: &mut Document) {
    unsafe {
        let inf_view = &mut document.inf_view;
        let analysis = &mut document.analysis;
        
        // Main toolbar buttons in one row
        // Select tool button
        if toolbar_button(const_cstr!("\u{f245}").as_ptr(), 
                         matches!(inf_view.action, crate::document::infview::Action::Normal(_)), true) {
            inf_view.action = crate::document::infview::Action::Normal(crate::document::infview::NormalState::Default);
        }
        if igIsItemHovered(0) {
            igBeginTooltip();
            widgets::show_text("Select tracks, nodes and objects. Drag to move.");
            igEndTooltip();
        }
        
        igSameLine(0.0, -1.0);
        
        // Insert Object button
        let current_icon = get_current_object_icon(inf_view);
        if toolbar_button(current_icon,
                         matches!(inf_view.action, crate::document::infview::Action::InsertObject(_)) || 
                         matches!(inf_view.action, crate::document::infview::Action::SelectObjectType), true) {
            inf_view.action = crate::document::infview::Action::SelectObjectType;
        }
        if igIsItemHovered(0) {
            igBeginTooltip();
            widgets::show_text("Opens a drop-down menu for selecting an object type.\nInsert the object by clicking a position.");
            igEndTooltip();
        }
        
        igSameLine(0.0, -1.0);
        
        // Draw tracks button
        if toolbar_button(const_cstr!("\u{f303}").as_ptr(), 
                         matches!(inf_view.action, crate::document::infview::Action::DrawingLine(_)), true) {
            inf_view.action = crate::document::infview::Action::DrawingLine(None);
        }
        if igIsItemHovered(0) {
            igBeginTooltip();
            widgets::show_text("Click and drag to create new tracks.");
            igEndTooltip();
        }
        
        igSameLine(0.0, -1.0);
        
        // Undo button
        if toolbar_button(const_cstr!("\u{f0e2}").as_ptr(), false, analysis.can_undo()) {
            analysis.undo();
        }
        if igIsItemHovered(0) {
            igBeginTooltip();
            widgets::show_text("Undo the previous action.");
            igEndTooltip();
        }
        
        igSameLine(0.0, -1.0);
        
        // Redo button
        if toolbar_button(const_cstr!("\u{f01e}").as_ptr(), false, analysis.can_redo()) {
            analysis.redo();
        }
        if igIsItemHovered(0) {
            igBeginTooltip();
            widgets::show_text("Redo the previously undone action.");
            igEndTooltip();
        }
        
        igSpacing();
        
        // Object insertion dropdown menu
        if matches!(&inf_view.action, crate::document::infview::Action::SelectObjectType) {
            igText(const_cstr!("Select Object Type:").as_ptr());
            igSeparator();
            
            // Home Signal (H)
            if igSelectable(const_cstr!("\u{f637} Home Signal (H)").as_ptr(), false, 0 as _, ImVec2::zero()) {
                inf_view.action = crate::document::infview::Action::InsertObject(Some(
                    crate::document::objects::Object {
                        loc: nalgebra_glm::vec2(0.0, 0.0),
                        tangent: nalgebra_glm::vec2(1,0),
                        functions: vec![crate::document::objects::Function::Signal { has_distant: false, id: None }],
                        id: None,
                        signal_props: Some(crate::document::objects::SignalProperties {
                            signal_type: crate::document::objects::SignalType::Home,
                            signal_kind: crate::document::objects::SignalKind::Two,
                            direction: crate::document::objects::TrackDirection::Right,
                        }),
                        switch_props: None,
                        placed_angle: None,
                    }
                ));
            }
            
            // Departure Signal (E)
            if igSelectable(const_cstr!("\u{f5b0} Departure Signal (E)").as_ptr(), false, 0 as _, ImVec2::zero()) {
                inf_view.action = crate::document::infview::Action::InsertObject(Some(
                    crate::document::objects::Object {
                        loc: nalgebra_glm::vec2(0.0, 0.0),
                        tangent: nalgebra_glm::vec2(1,0),
                        functions: vec![crate::document::objects::Function::Signal { has_distant: false, id: None }],
                        id: None,
                        signal_props: Some(crate::document::objects::SignalProperties {
                            signal_type: crate::document::objects::SignalType::Departure,
                            signal_kind: crate::document::objects::SignalKind::Two,
                            direction: crate::document::objects::TrackDirection::Right,
                        }),
                        switch_props: None,
                        placed_angle: None,
                    }
                ));
            }
            
            // Shunting Signal (U)
            if igSelectable(const_cstr!("\u{f061} Shunting Signal (U)").as_ptr(), false, 0 as _, ImVec2::zero()) {
                inf_view.action = crate::document::infview::Action::InsertObject(Some(
                    crate::document::objects::Object {
                        loc: nalgebra_glm::vec2(0.0, 0.0),
                        tangent: nalgebra_glm::vec2(1,0),
                        functions: vec![crate::document::objects::Function::Signal { has_distant: false, id: None }],
                        id: None,
                        signal_props: Some(crate::document::objects::SignalProperties {
                            signal_type: crate::document::objects::SignalType::Shunting,
                            signal_kind: crate::document::objects::SignalKind::Two,
                            direction: crate::document::objects::TrackDirection::Right,
                        }),
                        switch_props: None,
                        placed_angle: None,
                    }
                ));
            }
            
            // Section Insulator (I)
            if igSelectable(const_cstr!("\u{f715} Section Insulator (I)").as_ptr(), false, 0 as _, ImVec2::zero()) {
                inf_view.action = crate::document::infview::Action::InsertObject(Some(
                    crate::document::objects::Object {
                        loc: nalgebra_glm::vec2(0.0, 0.0),
                        tangent: nalgebra_glm::vec2(1,0),
                        functions: vec![crate::document::objects::Function::Detector],
                        id: None,
                        signal_props: None,
                        switch_props: None,
                        placed_angle: None,
                    }
                ));
            }
            
            // Switch (W)
            if igSelectable(const_cstr!("\u{f126} Switch (W)").as_ptr(), false, 0 as _, ImVec2::zero()) {
                inf_view.action = crate::document::infview::Action::InsertObject(Some(
                    crate::document::objects::Object {
                        loc: nalgebra_glm::vec2(0.0, 0.0),
                        tangent: nalgebra_glm::vec2(1,0),
                        functions: vec![crate::document::objects::Function::Switch { id: None }],
                        id: None,
                        signal_props: None,
                        switch_props: None,
                        placed_angle: None,
                    }
                ));
            }
        }
        
        igSpacing();
        igText(const_cstr!("Selection Tools").as_ptr());
        igSeparator();
        
        // Select All button
        if igButton(const_cstr!("Select All (Ctrl+A)").as_ptr(), ImVec2 { x: 0.0, y: 0.0 }) {
            use std::collections::HashSet;
            use crate::document::model::Ref;
            let all_ids: HashSet<Ref> = analysis.model().objects.keys().map(|pt| Ref::Object(*pt)).collect();
            inf_view.selection = all_ids;
        }
        
        // Clear Selection button
        if igButton(const_cstr!("Clear Selection").as_ptr(), ImVec2 { x: 0.0, y: 0.0 }) {
            inf_view.selection.clear();
        }
    }
}

fn render_section_header(title: *const i8, r: f32, g: f32, b: f32, state: &mut bool) -> bool {
    unsafe {
        // Set background color for section header
        let bg_color = ImVec4 { x: r, y: g, z: b, w: 0.3 };
        igPushStyleColor(ImGuiCol__ImGuiCol_Header as _, bg_color);
        igPushStyleColor(ImGuiCol__ImGuiCol_HeaderHovered as _, bg_color);
        igPushStyleColor(ImGuiCol__ImGuiCol_HeaderActive as _, bg_color);
        
        // Increase font scale for section headers
        igSetWindowFontScale(1.2);
        
        // Set flags based on state
        let mut flags = 0;
        if *state {
            flags |= ImGuiTreeNodeFlags__ImGuiTreeNodeFlags_DefaultOpen as i32;
        }
        
        // Render the collapsing header and get result
        let is_open = igCollapsingHeader(title, flags);
        
        // Update the state
        *state = is_open;
        
        // Restore font scale and colors
        igSetWindowFontScale(1.0);
        igPopStyleColor(3);
        
        is_open
    }
}

fn toolbar_button(name: *const i8, selected: bool, enabled: bool) -> bool {
    unsafe {
        if selected {
            let c1 = ImVec4 { x: 0.4, y: 0.65, z: 0.4, w: 1.0 };
            let c2 = ImVec4 { x: 0.5, y: 0.85, z: 0.5, w: 1.0 };
            let c3 = ImVec4 { x: 0.6, y: 0.9, z: 0.6, w: 1.0 };
            igPushStyleColor(ImGuiCol__ImGuiCol_Button as _, c1);
            igPushStyleColor(ImGuiCol__ImGuiCol_ButtonHovered as _, c1);
            igPushStyleColor(ImGuiCol__ImGuiCol_ButtonActive as _, c1);
        }
        if !enabled {
            igPushDisable();
            igPushStyleVarFloat(ImGuiStyleVar__ImGuiStyleVar_Alpha as _, 0.5);
        }
        let clicked = igButton(name, ImVec2 { x: 0.0, y: 0.0 });
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

fn get_current_object_icon(inf_view: &crate::document::infview::InfView) -> *const i8 {
    use crate::document::infview::Action;
    match &inf_view.action {
        Action::InsertObject(Some(obj)) => {
            for function in &obj.functions {
                match function {
                    crate::document::objects::Function::Signal { .. } => {
                        return const_cstr!("\u{f637}").as_ptr();
                    },
                    crate::document::objects::Function::Switch { .. } => {
                        return const_cstr!("\u{f126}").as_ptr();
                    },
                    crate::document::objects::Function::Detector => {
                        return const_cstr!("\u{f715}").as_ptr();
                    },
                }
            }
            const_cstr!("\u{f637}").as_ptr()
        },
        _ => const_cstr!("\u{f637}").as_ptr()
    }
}

/// 속성 탭 렌더링 (편집 기능 포함)
fn render_props_tab(document: &mut Document) {
    unsafe {
        // 선택된 객체 찾기
        let inf_view = &mut document.inf_view;
        
        if inf_view.selection.is_empty() {
            widgets::show_text("No object selected.");
            widgets::show_text("Select an object to edit its properties.");
            return;
        }

        // 첫 번째 선택된 객체 가져오기
        let selected_ref = inf_view.selection.iter().next().unwrap();
        let pta = match selected_ref {
            Ref::Object(pta) => *pta,
            _ => {
                widgets::show_text("Selected item is not an object.");
                return;
            }
        };

        // 객체 데이터를 미리 복사
        let obj_data = match document.analysis.model().objects.get(&pta) {
            Some(obj) => obj.clone(),
            None => {
                widgets::show_text("Object not found.");
                return;
            }
        };

        // 객체 타입 확인
        let (is_signal, is_switch, is_detector) = {
            let mut is_signal = false;
            let mut is_switch = false;
            let mut is_detector = false;
            
            for function in &obj_data.functions {
                match function {
                    Function::Signal { .. } => is_signal = true,
                    Function::Switch { .. } => is_switch = true,
                    Function::Detector => is_detector = true,
                }
            }
            (is_signal, is_switch, is_detector)
        };

        // 객체 타입 표시
        let object_type = if is_signal { "Signal" } else if is_switch { "Switch" } else if is_detector { "Detector" } else { "Unknown" };
        widgets::show_text(&format!("Object Type: {}", object_type));

        // ID 편집 기능
        widgets::show_text("ID:");
        igSameLine(0.0, 5.0);
        
        let current_id = {
            let mut id = String::new();
            for function in &obj_data.functions {
                match function {
                    Function::Signal { id: signal_id, .. } => {
                        if let Some(sid) = signal_id {
                            id = sid.clone();
                            break;
                        }
                    },
                    Function::Switch { id: switch_id } => {
                        if let Some(sid) = switch_id {
                            id = sid.clone();
                            break;
                        }
                    },
                    _ => {}
                }
            }
            id
        };

        // ID 입력 필드
        let mut id_buffer = current_id.clone().into_bytes();
        id_buffer.push(0);
        id_buffer.extend((0..50).map(|_| 0u8));
        if igInputText(const_cstr!("##object_id").as_ptr(), id_buffer.as_mut_ptr() as *mut _, id_buffer.len(), 0 as _, None, std::ptr::null_mut()) {
            let terminator = id_buffer.iter().position(|&c| c == 0).unwrap();
            id_buffer.truncate(terminator);
            let new_id = String::from_utf8_unchecked(id_buffer);
            if new_id != current_id {
                document.analysis.edit_model(|m| {
                    if let Some(obj) = m.objects.get_mut(&pta) {
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

        igSpacing();

        // 신호기 속성 편집
        if is_signal {
            igText(const_cstr!("Signal Properties").as_ptr());
            igSeparator();
            
            if let Some(props) = &obj_data.signal_props {
                // 신호 타입 편집
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

                // 신호 종류 편집
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

                // 방향 편집
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

                // 변경사항 적용
                if signal_type != props.signal_type || signal_kind != props.signal_kind || direction != props.direction {
                    document.analysis.edit_model(|m| {
                        if let Some(obj) = m.objects.get_mut(&pta) {
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
                if igButton(const_cstr!("Initialize Signal Properties").as_ptr(), ImVec2 { x: 0.0, y: 0.0 }) {
                    document.analysis.edit_model(|m| {
                        if let Some(obj) = m.objects.get_mut(&pta) {
                            obj.signal_props = Some(SignalProperties {
                                signal_type: SignalType::Home,
                                signal_kind: SignalKind::Two,
                                direction: TrackDirection::Left,
                            });
                        }
                        None
                    });
                }
            }
        }

        // 스위치 속성 편집
        if is_switch {
            igSpacing();
            igText(const_cstr!("Switch Properties").as_ptr());
            igSeparator();
            
            if let Some(props) = &obj_data.switch_props {
                // 스위치 타입 편집
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

                // 변경사항 적용
                if switch_type != props.switch_type {
                    document.analysis.edit_model(|m| {
                        if let Some(obj) = m.objects.get_mut(&pta) {
                            if let Some(props) = &mut obj.switch_props {
                                props.switch_type = switch_type;
                            }
                        }
                        None
                    });
                }

            } else {
                widgets::show_text("No switch properties set.");
                if igButton(const_cstr!("Initialize Switch Properties").as_ptr(), ImVec2 { x: 0.0, y: 0.0 }) {
                    document.analysis.edit_model(|m| {
                        if let Some(obj) = m.objects.get_mut(&pta) {
                            obj.switch_props = Some(SwitchProperties {
                                switch_type: SwitchType::Single,
                            });
                        }
                        None
                    });
                }
            }
        }

        // 위치 정보 표시
        igSpacing();
        igText(const_cstr!("Position Information").as_ptr());
        igSeparator();
        widgets::show_text(&format!("Position: ({:.1}, {:.1})", obj_data.loc.x, obj_data.loc.y));
        widgets::show_text(&format!("Tangent: ({:.1}, {:.1})", obj_data.tangent.x, obj_data.tangent.y));
        
        if let Some(angle) = obj_data.placed_angle {
            widgets::show_text(&format!("Placed Angle: {:.1}°", angle.to_degrees()));
        }

        // 객체 삭제 버튼
        igSpacing();
        igSeparator();
        if igButton(const_cstr!("Delete Object").as_ptr(), ImVec2 { x: 0.0, y: 0.0 }) {
            document.analysis.edit_model(|m| {
                m.objects.remove(&pta);
                None
            });
            inf_view.selection.clear();
        }
    }
}

/// 설정 탭 렌더링
fn render_settings_tab(sidebar: &mut SidebarWindow, document: &mut Document) {
    unsafe {
        // Configure Colors 버튼
        igText(const_cstr!("Configure Colors").as_ptr());
        igSeparator();
        
        if igButton(const_cstr!("Open Color Configuration").as_ptr(), ImVec2 { x: 0.0, y: 0.0 }) {
            // 색상 설정 창을 열기 위해 config 창을 토글
            sidebar.open_config = true;
        }
        

        igText(const_cstr!("View Settings").as_ptr());
        igSeparator();
        
        // 줌 레벨 표시 (View 구조체의 private 필드에 직접 접근할 수 없으므로 임시로 표시)
        widgets::show_text("Current Zoom: (View scale info)");
        
        // 뷰 중심점 표시 (View 구조체의 private 필드에 직접 접근할 수 없으므로 임시로 표시)
        widgets::show_text("View Center: (View translation info)");
        
        // 뷰 리셋 버튼
        if igButton(const_cstr!("Reset View").as_ptr(), ImVec2 { x: 0.0, y: 0.0 }) {
            document.inf_view.view = crate::document::view::View::default();
        }
    }
}

/// 도움말 탭 렌더링
fn render_help_tab() {
    unsafe {
        widgets::show_text("Keyboard Shortcuts:");
        igSpacing();
        widgets::show_text("F2 - Toggle Sidebar");
        widgets::show_text("Ctrl+S - Save");
        widgets::show_text("Ctrl+Z - Undo");
        widgets::show_text("Ctrl+Y - Redo");
        widgets::show_text("Ctrl+O - Open");
        
        igSpacing();
        igSeparator();
        igSpacing();
        
        widgets::show_text("Mouse Controls:");
        igSpacing();
        widgets::show_text("Left Click - Select objects");
        widgets::show_text("Right Click - Context menu");
        widgets::show_text("Scroll - Zoom in/out");
        widgets::show_text("Ctrl+Drag - Pan view");
    }
} 