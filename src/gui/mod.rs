pub mod widgets;
mod mainmenu;
mod keys;
pub mod windows;

mod infrastructure;
mod plan;
mod diagram;
mod dispatch;

pub use backend_glfw::imgui::ImVec2;

use crate::app::*;
use crate::document::*;
use crate::document::analysis::Analysis;
use crate::document::infview::InfView;
use crate::config::Config;
use crate::gui::windows::sidebar;

use const_cstr::*;
/*
fn render_main_content(
    config: &Config,
    analysis: &mut Analysis,
    inf_view: &mut InfView,
    dispatch_view: &mut Option<DispatchView>,
    inf_canvas: &mut Option<widgets::Draw>,
    diagram_split: &mut Option<f32>,
) {
    if dispatch_view.is_none() {
        let d = infrastructure::inf_view(config, analysis, inf_view, dispatch_view);
        *inf_canvas = Some(d);

        unsafe {
            use backend_glfw::imgui::*;
            let pos = igGetCursorPos_nonUDT2().into();
            let frameh = igGetFrameHeight();
            let framespace = igGetFrameHeightWithSpacing() - frameh;
            igSetCursorPos(pos + ImVec2 { x: 2.0*framespace, y : -frameh-3.0*framespace });
            let new_dispatchview = dispatch::dispatch_select_bar(config, &None, analysis);
            if let Some(nd) = new_dispatchview { 
                if nd.is_some() { *dispatch_view = nd; }
            }
            igSetCursorPos(pos);
        }
    } else {
        if diagram_split.is_none() { 
            *diagram_split = Some(0.5); 
        } 

        widgets::Splitter::vertical(diagram_split.as_mut().unwrap())
            .left(const_cstr!("inf_canv").as_ptr(), || {
                let d = infrastructure::inf_view(config, analysis, inf_view, dispatch_view); 
                *inf_canvas = Some(d);
            })
            .right(const_cstr!("dia_dptch").as_ptr(), || {
                if let Some(d) = dispatch::dispatch_view(config, inf_canvas.as_ref(), inf_view,
                                                         analysis, dispatch_view.as_mut().unwrap() ) {
                    *dispatch_view = d;
                }
            });
    }
}
*/
pub fn main(app :&mut App) -> bool {

    // keyboard commands (ctrl+s for save, etc. + a/s/d for tool selection)
    keys::keys(app);

    let mut inf_canvas = None;
    // Main window
    widgets::in_root_window(|| {
        // top menu bar
        mainmenu::main_menu(app);

        // Three main window arrangements:
        // 1. Infrastructure only (diagram_view = None)
        // 2. Manual dispatch view (diagram_view = Some(DispatchView::Manual(...)))
        // 3. Auto-dispatch view (diagram_view = Some(DispatchView::Auto(...)))
        let config = &app.config;
        let analysis = &mut app.document.analysis;
        let inf_view = &mut app.document.inf_view;
        let dispatch_view = &mut app.document.dispatch_view;
        
        // Infrastructure view only (dispatch is now handled by dockable window)
        let d = infrastructure::inf_view(config, analysis, inf_view, dispatch_view);
        inf_canvas = Some(d);
    });

    // Other windows
    windows::logview::view_log(&mut app.windows.log, &app.log);
    app.windows.debug = windows::debug::debug_window(app.windows.debug, &app, 
                                                     inf_canvas.as_ref(), &app.document.inf_view );
    windows::vehicles::edit_vehicles_window(&mut app.windows.vehicles, &mut app.document);
    windows::config::edit_config_window(&mut app.windows.config, &mut app.config);
    
    // 사이드바 창 렌더링 (오른쪽 고정)
    app.windows.sidebar.render(&mut app.document, Some(match app.windows.dispatch_window.dock_position {
        crate::gui::windows::dispatch_window::DockPosition::None => sidebar::DockPosition::None,
        crate::gui::windows::dispatch_window::DockPosition::Top => sidebar::DockPosition::Top,
        crate::gui::windows::dispatch_window::DockPosition::Bottom => sidebar::DockPosition::Bottom,
        crate::gui::windows::dispatch_window::DockPosition::Left => sidebar::DockPosition::Left,
        crate::gui::windows::dispatch_window::DockPosition::Right => sidebar::DockPosition::Right,
    }));
    
    // Dispatch 창 렌더링 (dockable)
            app.windows.dispatch_window.render(&mut app.document, &app.config, inf_canvas.as_ref(), Some(&app.windows.sidebar));
    
    // 사이드바에서 config 창 열기 요청 처리
    if app.windows.sidebar.open_config {
        app.windows.config = true;
        app.windows.sidebar.open_config = false;
    }

    app.windows.import_window.draw(&mut app.document.analysis);
    if let Some(win) = &mut app.windows.synthesis_window { if !win.draw(&mut app.document.analysis) {
        app.windows.synthesis_window = None; }}

    // Quit dialog
    let really_quit = if app.windows.quit {
		if app.document.fileinfo.unsaved {
			windows::quit::quit_window(&mut app.document, &mut app.windows)
		} else { true }
	} else { false };

    !really_quit
}
