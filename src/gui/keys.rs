use crate::app::App;
use crate::document::objects::*;
use crate::document::infview::*;
use crate::gui::mainmenu;
use crate::gui::infrastructure::delete_selection;
use crate::gui::windows::sidebar;
use crate::file;
use crate::document::*;

use log::*;
use backend_glfw::imgui::*;
use nalgebra_glm as glm;

/// 애플리케이션의 전역 키보드 단축키를 처리하고 해당하는 동작을 수행합니다.
///
/// 이 함수는 ImGui의 IO 이벤트를 확인하여 특정 키 조합이 눌렸을 때
/// 해당하는 동작(예: 저장, 열기, 실행 취소, 도구 선택 등)을 수행합니다.
/// `unsafe` 블록을 사용하여 ImGui의 C API에 직접 접근합니다.
///
/// # Arguments
///
/// * `app` - 키보드 입력에 따라 상태가 변경될 수 있는 메인 애플리케이션(`App`)의 가변 참조자입니다.
pub fn keys(app :&mut App) {
    unsafe {
        let io = igGetIO();

        // --- 실행 취소/다시 실행 단축키 ---
        /// `Ctrl+Z`를 누르면 마지막 작업을 실행 취소합니다.
        if (*io).KeyCtrl && !(*io).KeyShift && igIsKeyPressed('Z' as _, false) {
            app.document.analysis.undo();
        }
        /// `Ctrl+Shift+Z`를 누르면 실행 취소된 작업을 다시 실행합니다.
        if (*io).KeyCtrl && (*io).KeyShift && igIsKeyPressed('Z' as _, false) {
            app.document.analysis.redo();
        }
        /// `Ctrl+Y`를 누르면 실행 취소된 작업을 다시 실행합니다.
        if (*io).KeyCtrl && !(*io).KeyShift && igIsKeyPressed('Y' as _, false) {
            app.document.analysis.redo();
        }

        // --- 파일 저장 단축키 ---
        /// `Ctrl+S`를 누르면 현재 문서를 저장합니다.
        /// 파일 이름이 없거나 `Shift` 키를 함께 누르면 '다른 이름으로 저장' 대화상자를 엽니다.
        if (*io).KeyCtrl && igIsKeyPressed('S' as _, false) {
            match (&app.document.fileinfo.filename, (*io).KeyShift) {
                (None,_) | (_,true) => {
                    match file::save_interactive(app.document.analysis.model().clone()) {
                        Err(e) => { error!("Error saving file: {}", e); },
                        Ok(Some(filename)) => { app.document.set_saved_file(filename); },
                        _ => {},
                    }
                }
                (Some(filename),_) => {
                    match file::save(filename, app.document.analysis.model().clone()) {
                        Err(e) => { error!("Error saving file: {}", e); },
                        Ok(()) => { app.document.set_saved_file(filename.clone()); },
                        _ => {},
                    }
                },
            }
        }

        // --- 파일 열기 단축키 ---
        /// `Ctrl+O`를 누르면 파일 열기 대화상자를 통해 새 문서를 불러옵니다.
        if (*io).KeyCtrl && !(*io).KeyShift && igIsKeyPressed('O' as _, false) {
            mainmenu::load(app);
        }

        // --- UI 토글 단축키 ---
        /// `F2` 키를 누르면 사이드바의 표시 여부를 토글합니다.
        if igIsKeyPressed(290 as _, false) { // F2 key code
            app.windows.sidebar.is_open = !app.windows.sidebar.is_open;
        }


        // --- 도구 및 액션 단축키 ---
        // 다른 UI 항목(입력 필드 등)이 활성화되어 있지 않을 때만 동작합니다.
        if !igIsAnyItemActive() {
            /// `Ctrl+A`를 누르면 모든 객체를 선택합니다.
            if (*io).KeyCtrl && igIsKeyPressed('A' as _, false) {
                use std::collections::HashSet;
                use crate::document::model::Ref;
                let all_ids: HashSet<Ref> = app.document.analysis.model().objects.keys().map(|pt| Ref::Object(*pt)).collect();
                app.document.inf_view.selection = all_ids;
            /// `A` 키를 누르면 선택 도구(기본 상태)로 전환합니다.
            } else if igIsKeyPressed('A' as _, false) {
                app.document.inf_view.action = Action::Normal(NormalState::Default);
            }

            /// `Space` 키를 누르면 디스패치 시뮬레이션의 재생/일시정지를 토글합니다.
            if igIsKeyPressed(' ' as _, false) {
                if let Some(DispatchView::Manual(m)) 
                     | Some(DispatchView::Auto(AutoDispatchView { dispatch: Some(m), .. })) 
                         = &mut app.document.dispatch_view {
                    m.play = !m.play;
                }
            }

            /// `D` 키를 누르면 선로 그리기 도구로 전환합니다.
            if igIsKeyPressed('D' as _, false) {
                app.document.inf_view.action = Action::DrawingLine(None);
            }

            /// `S` 키를 누르면 객체 삽입 메뉴를 엽니다.
            if igIsKeyPressed('S' as _, false) {
                app.document.inf_view.action = Action::SelectObjectType;
            }

            /// `H` 키를 누르면 장내 신호기(Home Signal) 삽입 모드로 전환합니다.
            if igIsKeyPressed('H' as _, false) {
                app.document.inf_view.action = Action::InsertObject(Some(
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
                        placed_factor: None,
                    }
                ));
            }
            /// `E` 키를 누르면 출발 신호기(Departure Signal) 삽입 모드로 전환합니다.
            if igIsKeyPressed('E' as _, false) {
                app.document.inf_view.action = Action::InsertObject(Some(
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
                        placed_factor: None,
                    }
                ));
            }
            /// `U` 키를 누르면 입환 신호기(Shunting Signal) 삽입 모드로 전환합니다.
            if igIsKeyPressed('U' as _, false) {
                app.document.inf_view.action = Action::InsertObject(Some(
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
                        placed_factor: None,
                    }
                ));
            }
            /// `I` 키를 누르면 궤도 회로 절연(Section Insulator) 객체 삽입 모드로 전환합니다.
            if igIsKeyPressed('I' as _, false) {
                app.document.inf_view.action = Action::InsertObject(Some(
                    Object {
                        loc: glm::vec2(0.0, 0.0),
                        tangent: glm::vec2(1,0),
                        functions: vec![Function::Detector],
                        id: None,
                        signal_props: None,
                        switch_props: None,
                        placed_angle: None,
                        placed_factor: None,
                    }
                ));
            }
            /// `W` 키를 누르면 선로전환기(Switch) 삽입 모드로 전환합니다.
            if igIsKeyPressed('W' as _, false) {
                app.document.inf_view.action = Action::InsertObject(Some(
                    Object {
                        loc: glm::vec2(0.0, 0.0),
                        tangent: glm::vec2(1,0),
                        functions: vec![Function::Switch { id: None }],
                        id: None,
                        signal_props: None,
                        switch_props: None,
                        placed_angle: None,
                        placed_factor: None,
                    }
                ));
            }
            
            // 객체 선택 후, Delete 또는 Backspace 키를 누르면 선택된 객체를 삭제
            // Delete selected elements with Delete or Backspace key
            if igIsKeyPressed(ImGuiKey__ImGuiKey_Delete as _, false) {
                if !app.document.inf_view.selection.is_empty() {
                    use crate::gui::infrastructure::delete_selection;
                    delete_selection(&mut app.document.analysis, &mut app.document.inf_view);
                }
            }
            if igIsKeyPressed(ImGuiKey__ImGuiKey_Backspace as _, false) {
                /// `Backspace` 키를 누르면 선택된 객체를 삭제합니다.
                if !app.document.inf_view.selection.is_empty() {
                    use crate::gui::infrastructure::delete_selection;
                    delete_selection(&mut app.document.analysis, &mut app.document.inf_view);
                }
            }
            /// Mac의 `Delete` 키(Keycode 259)를 누르면 선택된 객체를 삭제합니다.
            /// (일반적인 Backspace와 동일하게 동작)
            if igIsKeyPressed(259, false) {
                if !app.document.inf_view.selection.is_empty() {
                    use crate::gui::infrastructure::delete_selection;
                    delete_selection(&mut app.document.analysis, &mut app.document.inf_view);
                }
            }
        }
    }
}
