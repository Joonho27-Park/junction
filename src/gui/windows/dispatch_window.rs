use backend_glfw::imgui::*;
use const_cstr::*;
use crate::gui::widgets;
use crate::document::Document;
use crate::document::analysis::Analysis;
use crate::document::infview::InfView;
use crate::document::dispatch::*;
use crate::document::DispatchView;
use crate::config::Config;
use crate::gui::dispatch;
use crate::gui::diagram::diagram_view;
use crate::gui::diagram;
use crate::gui::plan;
use crate::gui::widgets::Draw;
use crate::gui::windows::sidebar::{SidebarWindow, DockPosition as SidebarDockPosition};
use crate::util::VecMap;

pub struct DispatchWindow {
    pub is_open: bool,
    // Docking state
    pub is_docked: bool,
    pub dock_position: DockPosition,
    pub dock_threshold: f32, // Distance from edge to trigger docking
    // Mouse state tracking (saved during render)
    pub mouse_hovered: bool,
    pub mouse_position: ImVec2,
    pub mouse_in_dispatch_area: bool,
}

#[derive(Clone, Copy, PartialEq)]
pub enum DockPosition {
    None,
    Top,
    Bottom,
    Left,
    Right,
}

impl DispatchWindow {
    pub fn new() -> Self {
        Self {
            is_open: false, // 기본적으로 닫혀있음
            is_docked: true, // 기본적으로 dock되어 있음
            dock_position: DockPosition::Bottom, // 아래쪽에 dock
            dock_threshold: 50.0, // 50 pixels from edge to trigger docking
            // Mouse state tracking (saved during render)
            mouse_hovered: false,
            mouse_position: ImVec2 { x: 0.0, y: 0.0 },
            mouse_in_dispatch_area: false,
        }
    }

    pub fn render(&mut self, document: &mut Document, config: &Config, inf_canvas: Option<&Draw>, sidebar: Option<&SidebarWindow>) {
        if !self.is_open {
            return;
        }

        unsafe {
            let io = igGetIO();
            let display_size = (*io).DisplaySize;
            
            // Window flags
            let mut window_flags = ImGuiWindowFlags__ImGuiWindowFlags_NoCollapse as i32;
            
            if self.is_docked {
                // Docked window settings - fixed size
                window_flags |= ImGuiWindowFlags__ImGuiWindowFlags_NoMove as i32;
                window_flags |= ImGuiWindowFlags__ImGuiWindowFlags_NoResize as i32;
                
                // 메뉴바 높이 (File, Edit, View, Tools 메뉴)
                let menu_bar_height = 35.0; // 고정된 메뉴바 높이 + 여백
                
                match self.dock_position {
                    DockPosition::Bottom => {
                        let docked_pos = ImVec2 { x: 0.0, y: display_size.y - 300.0 };
                        let docked_size = ImVec2 { x: display_size.x, y: 300.0 };
                        igSetNextWindowPos(docked_pos, ImGuiCond__ImGuiCond_Always as _, ImVec2 { x: 0.0, y: 0.0 });
                        igSetNextWindowSize(docked_size, ImGuiCond__ImGuiCond_Always as _);
                    },
                    DockPosition::Top => {
                        let docked_pos = ImVec2 { x: 0.0, y: menu_bar_height };
                        let docked_size = ImVec2 { x: display_size.x, y: 300.0 };
                        igSetNextWindowPos(docked_pos, ImGuiCond__ImGuiCond_Always as _, ImVec2 { x: 0.0, y: 0.0 });
                        igSetNextWindowSize(docked_size, ImGuiCond__ImGuiCond_Always as _);
                    },
                    DockPosition::Left => {
                        let docked_pos = ImVec2 { x: 0.0, y: menu_bar_height };
                        let docked_size = ImVec2 { x: 400.0, y: display_size.y - menu_bar_height };
                        igSetNextWindowPos(docked_pos, ImGuiCond__ImGuiCond_Always as _, ImVec2 { x: 0.0, y: 0.0 });
                        igSetNextWindowSize(docked_size, ImGuiCond__ImGuiCond_Always as _);
                    },
                    DockPosition::Right => {
                        let docked_pos = ImVec2 { x: display_size.x - 400.0, y: menu_bar_height };
                        let docked_size = ImVec2 { x: 400.0, y: display_size.y - menu_bar_height };
                        igSetNextWindowPos(docked_pos, ImGuiCond__ImGuiCond_Always as _, ImVec2 { x: 0.0, y: 0.0 });
                        igSetNextWindowSize(docked_size, ImGuiCond__ImGuiCond_Always as _);
                    },
                    DockPosition::None => {}
                }
            } else {
                // Floating window settings
                igSetNextWindowSize(ImVec2 { x: 800.0, y: 400.0 }, ImGuiCond__ImGuiCond_FirstUseEver as _);
                igSetNextWindowPos(ImVec2 { x: 100.0, y: 200.0 }, ImGuiCond__ImGuiCond_FirstUseEver as _, ImVec2 { x: 0.0, y: 0.0 });
            }

            if igBegin(const_cstr!("Dispatch").as_ptr(), &mut self.is_open as *mut bool, window_flags) {
                // 마우스 호버 상태 확인 및 저장
                self.mouse_hovered = self.is_mouse_hovered();
                self.mouse_position = self.get_mouse_position();
                self.mouse_in_dispatch_area = self.is_mouse_in_dispatch_area(display_size);
                
                // Check for docking (after saving mouse state)
                let sidebar_dock_position = sidebar.map(|s| {
                    match s.dock_position {
                        SidebarDockPosition::None => DockPosition::None,
                        SidebarDockPosition::Top => DockPosition::Top,
                        SidebarDockPosition::Bottom => DockPosition::Bottom,
                        SidebarDockPosition::Left => DockPosition::Left,
                        SidebarDockPosition::Right => DockPosition::Right,
                    }
                });
                self.check_docking(display_size, sidebar_dock_position);
                
                // Dispatch content
                self.render_dispatch_content(document, config, inf_canvas);
            }
            igEnd();
        }
    }
    
    fn render_dispatch_content(&mut self, document: &mut Document, config: &Config, inf_canvas: Option<&Draw>) {
        let analysis = &mut document.analysis;
        let inf_view = &mut document.inf_view;
        let dispatch_view = &mut document.dispatch_view;
        
        // Dispatch selection bar
        let new_dispatch = dispatch::dispatch_select_bar(config, dispatch_view, analysis);
        if let Some(nd) = new_dispatch {
            *dispatch_view = nd;
        }
        
        // Dispatch view content
        let mut new_dispatch_view = None;
        if let Some(dv) = dispatch_view {
            match dv {
                DispatchView::Manual(manual) => {
                    let graph = analysis.data().dispatch.vecmap_get(manual.dispatch_idx);
                    if let Some((_gen, graph)) = graph {
                        unsafe { igSameLine(0.0, -1.0); }
                        if let Some(action) = diagram_view(config, inf_canvas, inf_view, analysis, manual, graph, &mut self.is_docked) {
                            match action {
                                diagram::DiagramViewAction::DeleteCommand { id } => {
                                    analysis.edit_model(|m| {
                                        m.dispatches.get_mut(manual.dispatch_idx)?.commands.retain(|(x, _)| *x != id);
                                        None
                                    });
                                }
                                diagram::DiagramViewAction::MoveCommand { idx, id, t } => {
                                    analysis.edit_model(|m| {
                                        let commands = &mut m.dispatches.get_mut(manual.dispatch_idx)?.commands;
                                        for (c_id, (c_t, _)) in commands.iter_mut() {
                                            if *c_id == id { *c_t = t; }
                                        }
                                        commands.sort_by_key(|(_, (t, _))| ordered_float::OrderedFloat(*t));
                                        None
                                    });
                                }
                                diagram::DiagramViewAction::Close => {
                                    new_dispatch_view = Some(None);
                                }
                            }
                        }
                    }

                    if !analysis.model().dispatches.iter().any(|(id, _)| *id == manual.dispatch_idx) {
                        new_dispatch_view = Some(None);
                    }
                }
                DispatchView::Auto(auto) => {
                    let new_auto = plan::edit_plan(config, inf_canvas, inf_view, analysis, &mut self.is_docked, auto);
                    if let Some(new_dispatch) = new_auto {
                        new_dispatch_view = Some(new_dispatch);
                    }

                    if let Some(manual) = &mut auto.dispatch {
                        if let Some(Some((_gen, dispatches))) = analysis.data().plandispatches.get(auto.plan_idx) {
                            if let Some(graph) = dispatches.get(manual.dispatch_idx) {
                                diagram_view(config, inf_canvas, inf_view, analysis, manual, graph, &mut self.is_docked);
                            } else {
                                // Plan doesn't exist anymore.
                                if dispatches.len() > 0 {
                                    manual.dispatch_idx = dispatches.len() - 1;
                                } else {
                                    auto.dispatch = None;
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Apply changes to dispatch_view after the borrow is released
        if let Some(new_view) = new_dispatch_view {
            *dispatch_view = new_view;
        }
    }
    
    /// dispatch 창 위에 마우스가 있는지 확인
    pub fn is_mouse_hovered(&self) -> bool {
        unsafe {
            igIsWindowHovered(0)
        }
    }
    
    /// 마우스 위치를 가져오기
    pub fn get_mouse_position(&self) -> ImVec2 {
        unsafe {
            igGetMousePos_nonUDT2().into()
        }
    }
    
    /// 마우스가 dispatch 영역 내에 있는지 확인 (창 경계 기반)
    pub fn is_mouse_in_dispatch_area(&self, display_size: ImVec2) -> bool {
        let mouse_pos = self.get_mouse_position();
        let menu_bar_height = 35.0; // 고정된 메뉴바 높이 + 여백
        
        if self.is_docked {
            match self.dock_position {
                DockPosition::Bottom => {
                    mouse_pos.x >= 0.0 && mouse_pos.x <= display_size.x &&
                    mouse_pos.y >= display_size.y - 300.0 && mouse_pos.y <= display_size.y
                },
                DockPosition::Top => {
                    mouse_pos.x >= 0.0 && mouse_pos.x <= display_size.x &&
                    mouse_pos.y >= menu_bar_height && mouse_pos.y <= menu_bar_height + 300.0
                },
                DockPosition::Left => {
                    mouse_pos.x >= 0.0 && mouse_pos.x <= 400.0 &&
                    mouse_pos.y >= menu_bar_height && mouse_pos.y <= display_size.y
                },
                DockPosition::Right => {
                    mouse_pos.x >= display_size.x - 400.0 && mouse_pos.x <= display_size.x &&
                    mouse_pos.y >= menu_bar_height && mouse_pos.y <= display_size.y
                },
                DockPosition::None => false
            }
        } else {
            // 플로팅 창의 경우 ImGui가 자동으로 처리하므로 단순히 호버 상태 확인
            self.is_mouse_hovered()
        }
    }

    fn check_docking(&mut self, display_size: ImVec2, sidebar_dock_position: Option<DockPosition>) {
        if self.is_docked {
            return; // Already docked
        }
        
        let io = unsafe { igGetIO() };
        
        // Check if mouse left button is held down (dragging)
        if unsafe { !(*io).MouseDown[0] } {
            return; // Not dragging
        }
        
        if unsafe { !igIsMouseDragging(0, 0.0) } {
            return; // Not dragging
        }
        
        // Check if the dispatch window is being dragged (using saved hover state)
        if !self.mouse_hovered {
            return; // Not dragging the dispatch window
        }
        
        // Check if mouse is near edges for docking
        let menu_bar_height = 35.0;
        
        // Check if mouse is near bottom edge
        if self.mouse_position.y >= display_size.y - self.dock_threshold {
            self.is_docked = true;
            self.dock_position = DockPosition::Bottom;
        }
        // Check if mouse is near top edge (below menu bar)
        else if self.mouse_position.y <= menu_bar_height + self.dock_threshold && self.mouse_position.y >= menu_bar_height {
            self.is_docked = true;
            self.dock_position = DockPosition::Top;
        }
        // Check if mouse is near left edge (only if sidebar is not docked to left)
        else if self.mouse_position.x <= self.dock_threshold && sidebar_dock_position != Some(DockPosition::Left) {
            self.is_docked = true;
            self.dock_position = DockPosition::Left;
        }
        // Check if mouse is near right edge (only if sidebar is not docked to right)
        else if self.mouse_position.x >= display_size.x - self.dock_threshold && sidebar_dock_position != Some(DockPosition::Right) {
            self.is_docked = true;
            self.dock_position = DockPosition::Right;
        }
    }
} 