use matches::matches;
use serde::{Serialize, Deserialize};

use crate::document::view::*;
use crate::document::model::*;
use crate::document::infview::{round_coord, unround_coord};

use crate::config::*;
use crate::util::*;
use backend_glfw::imgui::*;
use nalgebra_glm as glm;


#[derive(Clone)]
#[derive(Debug)]
#[derive(Serialize,Deserialize)]
pub struct Object {
    pub loc :PtC,
    pub tangent :Vc,
    pub functions :Vec<Function>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[derive(Serialize,Deserialize)]
//이곳에서 새로운 signal/detector추가
pub enum Function { MainSignal { has_distant :bool }, Detector , ShiftingSignal { has_distant :bool}, Switch }

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
//이곳에서 object의 속성 추가
pub enum ObjectState { SignalStop, SignalProceed, DistantStop, DistantProceed }

impl Object {
    pub fn move_to(&mut self, model :&Model, pt :PtC) -> Option<()> {
        if let Some((l,_param,(d1,d2))) = model.get_closest_lineseg(pt) {
            let (pt_on_line,_param) = project_to_line(pt, glm::vec2(l.0.x as _ ,l.0.y as _ ),
                                                 glm::vec2(l.1.x as _ ,l.1.y as _ ));
            let tangent : PtC = glm::vec2(l.1.x as f32 -l.0.x as f32 ,l.1.y as f32 -l.0.y as f32);
            let normal : PtC   = glm::vec2(-tangent.y,tangent.x);
            self.tangent = glm::vec2(tangent.x.round() as _, tangent.y.round() as _);

            if self.functions.iter().find(|c| matches!(c, Function::MainSignal { .. })).is_some() {
                    let factor = if glm::angle(&(pt_on_line - pt), &normal) > glm::half_pi() {
                        1.0 } else { -1.0 };
                    let offset = 0.25*normal*factor;
                    if factor > 0.0 { self.tangent *= -1; }
                    self.loc = glm::vec2(
                        pt_on_line.x.round(),
                        pt_on_line.y.round()
                    );
                    if !model.has_detector_at(self.loc) {
                        self.loc += offset;
                        return Some(());
                    }
                    self.loc += offset;
            } else if self.functions.iter().find(|c| matches!(c, Function::ShiftingSignal { .. })).is_some() {
                    let factor = if glm::angle(&(pt_on_line - pt), &normal) > glm::half_pi() {
                        1.0 } else { -1.0 };
                    let offset = 0.25*normal*factor;
                    if factor > 0.0 { self.tangent *= -1; }
                    self.loc = glm::vec2(
                        pt_on_line.x.round(),
                        pt_on_line.y.round()
                    );
                    if !model.has_detector_at(self.loc) {
                        self.loc += offset;
                        return Some(());
                    }
                    self.loc += offset;
            }else if self.functions.iter().find(|c| matches!(c, Function::Detector)).is_some() {
                self.loc = glm::vec2(
                    pt_on_line.x.round(),
                    pt_on_line.y.round()
                );
            }else if self.functions.iter().find(|c| matches!(c, Function::Switch)).is_some() {
                self.loc = pt_on_line;
            }

            None
        } else {
            self.loc = pt;
            Some(())
        }
    }

    pub fn draw(&self, pos :ImVec2, view :&View, draw_list :*mut ImDrawList, c :u32, state :&[ObjectState], config :&Config) {
        // 유틸: ImVec2 * f32
            fn mul_imvec2(v: ImVec2, f: f32) -> ImVec2 {
                ImVec2 { x: v.x * f, y: v.y * f }
            }

        unsafe {
            let p = pos + view.world_ptc_to_screen(self.loc);
            let scale = 5.0;
            // TODO can this be simplified?
            let tangent = ImVec2 { x: scale * self.tangent.x as f32,
                                   y: scale * -self.tangent.y as f32 };
            let normal  = ImVec2 { x: scale * -self.tangent.y as f32,
                                   y: scale * -self.tangent.x as f32 };

            for f in self.functions.iter() {
                match f {
                    Function::Detector => {
                        ImDrawList_AddLine(draw_list, p - normal, p + normal, c, 5.0);
                        /*
                        배경 색갈과 같은 감지기. thickness는 4.0으로 하여 좀 더 잘 보이도록 설정
                        let detector_color = config.color_u32(RailUIColorName::CanvasDetector);
                        ImDrawList_AddLine(draw_list, p - normal, p + normal, detector_color, 4.0);
                        */    
                    },
                    Function::MainSignal { has_distant } => {
                        // base
                        ImDrawList_AddLine(draw_list, p + normal, p - normal, c, 2.0);

                        let stem = if *has_distant { 2.0 } else { 1.0 };
                        ImDrawList_AddLine(draw_list, p, p + stem*tangent, c, 2.0);

                        for s in state.iter() {
                            match s {
                                ObjectState::SignalStop => {
                                    let c = config.color_u32(RailUIColorName::CanvasSignalStop);
                                    ImDrawList_AddCircleFilled(draw_list, p + stem*tangent + tangent, scale, c, 8);
                                },
                                ObjectState::SignalProceed => {
                                    let c = config.color_u32(RailUIColorName::CanvasSignalProceed);
                                    ImDrawList_AddCircleFilled(draw_list, p + stem*tangent + tangent, scale, c, 8);
                                },
                                ObjectState::DistantStop if *has_distant => {
                                    let c = config.color_u32(RailUIColorName::CanvasSignalStop);
                                    ImDrawList_AddCircleFilled(draw_list, p + 1.5*tangent + normal, scale*0.8, c, 8);
                                },
                                ObjectState::DistantProceed => {
                                    let c = config.color_u32(RailUIColorName::CanvasSignalProceed);
                                    ImDrawList_AddCircleFilled(draw_list, p + 1.5*tangent + normal, scale*0.8, c, 8);
                                },
                                _ => {},
                            };
                        }

                        // distant
                        if *has_distant {
                            ImDrawList_AddCircle(draw_list, p + 1.5*tangent + normal, scale*0.8, c, 8, 2.0);
                        }
                        // main signal
                        ImDrawList_AddCircle(draw_list, p + stem*tangent + tangent, scale, c, 8, 2.0);
                    },
                    Function::ShiftingSignal { has_distant } => {
                    // 신호기 수평 라인
                    ImDrawList_AddLine(draw_list, p + normal, p - normal, c, 2.0);

                    // 수직 기둥
                    let stem = if *has_distant { 2.0 } else { 1.0 };
                    ImDrawList_AddLine(draw_list, p, p + mul_imvec2(tangent, stem), c, 2.0);

                    // tangent의 수직 벡터 계산 (한 번만 계산해서 재사용)
                    let normal = ImVec2 { x: tangent.y, y: -tangent.x };
                    let normal_len = (normal.x * normal.x + normal.y * normal.y).sqrt();
                    let n = if normal_len > 0.0 {
                        ImVec2 { x: normal.x / normal_len, y: normal.y / normal_len }
                    } else { ImVec2 { x: 0.0, y: 1.0 } };

                    // 신호기 수평 라인 길이의 1/2만큼 normal 방향으로 이동 (한 번만 계산)
                    let offset = scale * 1.0;

                    // 상태별 1/4 원(항상 tangent 방향 기준 0~π/2)
                    for s in state.iter() {
                        // 신호 상태에 따라 색상을 결정
                        let color = match s {
                            ObjectState::SignalStop => config.color_u32(RailUIColorName::CanvasSignalStop),
                            ObjectState::SignalProceed => config.color_u32(RailUIColorName::CanvasSignalProceed),
                            ObjectState::DistantStop if *has_distant => config.color_u32(RailUIColorName::CanvasSignalStop),
                            ObjectState::DistantProceed => config.color_u32(RailUIColorName::CanvasSignalProceed),
                            _ => continue, // 해당 상태가 아니면 건너뜀
                        };

                        // 신호 상태에 따라 1/4 원의 중심(base) 위치를 결정
                        let base = match s {
                            ObjectState::DistantStop | ObjectState::DistantProceed => 
                                p + mul_imvec2(tangent, 1.5) + mul_imvec2(n, offset),
                            _ => 
                                p + mul_imvec2(tangent, stem) + mul_imvec2(n, offset),
                        };

                        // 신호 상태에 따라 1/4 원의 반지름(크기)을 결정
                        let size = match s {
                            ObjectState::DistantStop | ObjectState::DistantProceed => scale * 1.5,
                            _ => scale * 2.0,
                        };

                        // tangent 벡터(신호기 방향)를 정규화(길이 1로 만듦)
                        let tangent_len = (tangent.x * tangent.x + tangent.y * tangent.y).sqrt();
                        let t = if tangent_len > 0.0 {
                            ImVec2 { x: tangent.x / tangent_len, y: tangent.y / tangent_len }
                        } else { ImVec2 { x: 1.0, y: 0.0 } };

                        // tangent 벡터의 각도를 구함 (신호기 방향이 기준이 됨)
                        let t_angle = t.y.atan2(t.x);

                        // 아크의 시작 각도: tangent 방향
                        let a0 = t_angle;
                        // 아크의 끝 각도: 시작 각도에서 90도(π/2) 더한 값
                        let a1 = a0 + std::f32::consts::FRAC_PI_2;

                        // 아크를 그릴 때 사용할 세그먼트 개수(곡선의 부드러움)
                        let num_segments = 16;

                        // 아크의 시작점 좌표 계산
                        let start_pt = ImVec2 {
                            x: base.x + size * a0.cos(),
                            y: base.y + size * a0.sin(),
                        };

                        // 그리기 전에 패스(경로) 초기화
                        ImDrawList_PathClear(draw_list);

                        // 1/4 원 아크를 패스에 추가 (base를 중심, size를 반지름, a0~a1 각도)
                        ImDrawList_PathArcTo(draw_list, base, size, a0, a1, num_segments);

                        // 아크의 끝점에서 중심(base)까지 직선을 패스에 추가
                        ImDrawList_PathLineTo(draw_list, base);

                        // 중심(base)에서 아크의 시작점까지 직선을 패스에 추가
                        ImDrawList_PathLineTo(draw_list, start_pt);

                        // 패스를 채워서 닫힌 도형을 그림 (색상: color)
                        ImDrawList_PathFillConvex(draw_list, color);
                    }

                    // 원거리 신호기 테두리 1/4 원(항상 tangent 방향 기준 0~π/2)
                    if *has_distant {
                        // 1/4 원의 중심(base) 위치를 계산 (tangent 방향 + normal 방향)
                        let base = p + mul_imvec2(tangent, 1.5) + mul_imvec2(n, offset);
                        // 1/4 원의 반지름(크기) 설정
                        let size = scale * 1.5;
                        // tangent 벡터(신호기 방향)를 정규화
                        let tangent_len = (tangent.x * tangent.x + tangent.y * tangent.y).sqrt();
                        let t = if tangent_len > 0.0 {
                            ImVec2 { x: tangent.x / tangent_len, y: tangent.y / tangent_len }
                        } else { ImVec2 { x: 1.0, y: 0.0 } };
                        // tangent 벡터의 각도
                        let t_angle = t.y.atan2(t.x);
                        // 아크의 시작/끝 각도
                        let a0 = t_angle;
                        let a1 = a0 + std::f32::consts::FRAC_PI_2;
                        let num_segments = 16;
                        // 아크의 시작점 좌표 계산
                        let start_pt = ImVec2 {
                            x: base.x + size * a0.cos(),
                            y: base.y + size * a0.sin(),
                        };
                        // 패스 초기화
                        ImDrawList_PathClear(draw_list);
                        // 1/4 원 아크 추가
                        ImDrawList_PathArcTo(draw_list, base, size, a0, a1, num_segments);
                        // 아크 끝점에서 중심까지 직선
                        ImDrawList_PathLineTo(draw_list, base);
                        // 중심에서 아크 시작점까지 직선
                        ImDrawList_PathLineTo(draw_list, start_pt);
                        // 외곽선만 그림(채우지 않음)
                        ImDrawList_PathStroke(draw_list, c, true, 2.0);
                    }

                    // 메인 신호기 테두리 1/4 원(항상 tangent 방향 기준 0~π/2)
                    // 중심(base) 위치는 stem만큼 tangent 방향 + normal 방향으로 이동
                    let base = p + mul_imvec2(tangent, stem) + mul_imvec2(n, offset);
                    // 반지름(크기) 설정
                    let size = scale * 2.0;
                    // tangent 벡터 정규화
                    let tangent_len = (tangent.x * tangent.x + tangent.y * tangent.y).sqrt();
                    let t = if tangent_len > 0.0 {
                        ImVec2 { x: tangent.x / tangent_len, y: tangent.y / tangent_len }
                    } else { ImVec2 { x: 1.0, y: 0.0 } };
                    // tangent 벡터의 각도
                    let t_angle = t.y.atan2(t.x);
                    // 아크의 시작/끝 각도
                    let a0 = t_angle;
                    let a1 = a0 + std::f32::consts::FRAC_PI_2;
                    let num_segments = 16;
                    // 아크의 시작점 좌표 계산
                    let start_pt = ImVec2 {
                        x: base.x + size * a0.cos(),
                        y: base.y + size * a0.sin(),
                    };
                    // 패스 초기화
                    ImDrawList_PathClear(draw_list);
                    // 1/4 원 아크 추가
                    ImDrawList_PathArcTo(draw_list, base, size, a0, a1, num_segments);
                    // 아크 끝점에서 중심까지 직선
                    ImDrawList_PathLineTo(draw_list, base);
                    // 중심에서 아크 시작점까지 직선
                    ImDrawList_PathLineTo(draw_list, start_pt);
                    // 외곽선만 그림(채우지 않음)
                    ImDrawList_PathStroke(draw_list, c, true, 2.0);
                },
                Function::Switch => {
                    // Switch를 위한 간단한 X 모양 그리기
                    let switch_size = scale * 1.5;
                    ImDrawList_AddLine(draw_list, 
                        p + ImVec2 { x: -switch_size, y: -switch_size },
                        p + ImVec2 { x: switch_size, y: switch_size },
                        c, 3.0);
                    ImDrawList_AddLine(draw_list, 
                        p + ImVec2 { x: switch_size, y: -switch_size },
                        p + ImVec2 { x: -switch_size, y: switch_size },
                        c, 3.0);
                }
            }

            }
        }
    }
}



