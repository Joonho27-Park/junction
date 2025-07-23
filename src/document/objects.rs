use matches::matches;
use serde::{Serialize, Deserialize};

use crate::document::view::*;
use crate::document::model::*;
use crate::document::infview::{round_coord, unround_coord};
use crate::document::analysis::Analysis;

use crate::config::*;
use crate::util::*;
use backend_glfw::imgui::*;
use nalgebra_glm as glm;


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Object {
    pub loc: PtC,
    pub tangent: Vc,
    pub functions: Vec<Function>,
    pub id: Option<String>,
    // 각 기능별 속성들을 별도로 관리
    pub signal_props: Option<SignalProperties>,
    pub switch_props: Option<SwitchProperties>,
    // 스위치가 배치될 때의 각도를 저장
    pub placed_angle: Option<f32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SignalProperties {
    pub signal_type: SignalType,
    pub signal_kind: SignalKind,
    pub direction: TrackDirection,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SwitchProperties {
    pub switch_type: SwitchType,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Function {
    Signal { has_distant: bool, id: Option<String> },
    Detector,
    Switch { id: Option<String> },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignalType {
    Home,       // 장내
    Departure,  // 출발
    Shunting,   // 입환
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignalKind {
    Two,   // 2현시
    Three, // 3현시
    Four,  // 4현시
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Copy)]
pub enum TrackDirection {
    Left,
    Right,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SwitchType {
    Single,     // 단동
    Double,     // 쌍동
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
//이곳에서 object의 속성 추가
pub enum ObjectState { SignalStop, SignalProceed, DistantStop, DistantProceed }

pub const SIGNAL_OFFSET: f32 = 0.35;
pub const SWITCH_OFFSET: f32 = 0.5;

impl Object {
    pub fn move_to(&mut self, model :&Model, analysis :&Analysis, pt :PtC) -> Option<()> {
        if let Some((l,_param,(d1,d2))) = model.get_closest_lineseg(pt) {
            let (pt_on_line,_param) = project_to_line(pt, glm::vec2(l.0.x as _ ,l.0.y as _ ),
                                                 glm::vec2(l.1.x as _ ,l.1.y as _ ));
            let tangent : PtC = glm::vec2(l.1.x as f32 -l.0.x as f32 ,l.1.y as f32 -l.0.y as f32);
            let normal : PtC   = glm::vec2(-tangent.y,tangent.x);
            self.tangent = glm::vec2(tangent.x.round() as _, tangent.y.round() as _);

            if self.functions.iter().find(|c| matches!(c, Function::Signal { has_distant, id: _ })).is_some() {
                    let factor = if glm::angle(&(pt_on_line - pt), &normal) > glm::half_pi() {
                        1.0 } else { -1.0 };
                    // 신호기-트랙(선로) 사이 offset 적용 (여기서 값 조정)
                    let normal_len = glm::length(&normal);
                    let n = if normal_len > 0.0 { normal / normal_len } else { normal };
                    let offset = SIGNAL_OFFSET * n * factor;
                    if factor > 0.0 {
                        self.tangent *= -1;
                    }
                    self.loc = glm::vec2(
                        (pt_on_line.x * 2.0).round() / 2.0,
                        (pt_on_line.y * 2.0).round() / 2.0
                    );
                    // 신호기는 detector가 있는 위치에만 배치 가능
                    if !model.has_detector_at(self.loc) {
                        return None;
                    }
                    self.loc += offset;
                    // 신호기가 성공적으로 배치될 때 기울기 출력 및 각도 저장
                    let tangent_angle = (self.tangent.y as f32).atan2(self.tangent.x as f32);
                    let angle_degrees = tangent_angle * 180.0 / std::f32::consts::PI;

                    // 이 신호기의 각도를 저장
                    self.placed_angle = Some(angle_degrees);

                    // 각도에 따라 TrackDirection 설정
                    if let Some(signal_props) = &mut self.signal_props {
                        let direction = if angle_degrees == -135.0 || angle_degrees == 135.0 || angle_degrees == 180.0 {
                            TrackDirection::Right
                        } else if angle_degrees == 0.0 || angle_degrees == 45.0 || angle_degrees == -45.0 {
                            TrackDirection::Left
                        } else {
                            // 기본값 (기존 방향 유지)
                            signal_props.direction
                        };
                        signal_props.direction = direction;
                    }
                    return Some(());
            } else if self.functions.iter().find(|c| matches!(c, Function::Detector)).is_some() {
                self.loc = glm::vec2(
                    (pt_on_line.x * 2.0).round() / 2.0,
                    (pt_on_line.y * 2.0).round() / 2.0
                );
                return Some(());
            } else if self.functions.iter().find(|c| matches!(c, Function::Switch { id: _ })).is_some() {
                // Switch는 NDType::Sw(side) 노드의 위치 반경 내에서만 배치 가능
                // 자동 설치 방식과 동일한 오프셋 적용
                let factor = if glm::angle(&(pt_on_line - pt), &normal) > glm::half_pi() {
                    1.0 } else { -1.0 };
                
                // 자동 설치 방식과 동일한 오프셋 계산
                let normal_len = glm::length(&normal);
                let normalized_normal = if normal_len > 0.0 { normal / normal_len } else { normal };
                let offset = 0.5 * normalized_normal * factor;
                
                let place_pos = glm::vec2(
                    (pt_on_line.x * 2.0).round() / 2.0,
                    (pt_on_line.y * 2.0).round() / 2.0
                ) + offset;
                
                let mut found = false;
                
                // 신호기와 동일하게 tangent 방향 조정
                if factor > 0.0 {
                    self.tangent *= -1;
                }

                // analysis를 통해 topology의 locations에서 스위치 노드 찾기
                if let Some((_, topology)) = analysis.data().topology.as_ref() {
                    for (sw_pt, (ndtype, _)) in topology.locations.iter() {
                        if let NDType::Sw(_, _) = ndtype {
                            let sw_pt_f = glm::vec2(sw_pt.x as f32, sw_pt.y as f32);
                            if glm::distance(&place_pos, &sw_pt_f) <= 1.2 {
                                found = true;
                                break;
                            }
                        }
                    }
                }
                if found {
                    self.loc = place_pos;
                    // 스위치가 성공적으로 배치될 때만 기울기 출력 및 각도 저장
                    let tangent_angle = (self.tangent.y as f32).atan2(self.tangent.x as f32);
                    let angle_degrees = tangent_angle * 180.0 / std::f32::consts::PI;

                    // 이 스위치의 각도를 저장
                    self.placed_angle = Some(angle_degrees);
                    return Some(());
                } else {
                    self.loc = place_pos;
                    return Some(());
                }
            }

            return None;
        } else {
            // 트랙이 없는 곳에서는 배치 불가
            None
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
                    // ===== 궤도 분리기(Detector) 렌더링 =====
                    // 궤도 분리기는 트랙을 가로지르는 두꺼운 수평선으로 표현
                    Function::Detector => {
                        // 트랙에 수직인 수평선을 그림 (p - normal에서 p + normal까지)
                        // 두께는 5.0으로 다른 객체보다 두껍게 설정
                        ImDrawList_AddLine(draw_list, p - normal, p + normal, c, 5.0);
                    },
                    // ===== 메인 신호기(MainSignal) 렌더링 =====
                    Function::Signal { has_distant, id } => {
                        // Determine rendering style by signal_props.signal_type
                        let style = self.signal_props.as_ref().map(|props| props.signal_type.clone());
                        match style {
                            Some(SignalType::Home) | Some(SignalType::Departure) => {
                                // 기존 MainSignal 렌더링 코드
                                // ===== 신호기 기본 구조 그리기 =====
                                // 1. 신호기 기둥(base): 트랙에 수직인 수평선
                                // p + normal에서 p - normal까지 그려서 트랙을 가로지르는 선
                                ImDrawList_AddLine(draw_list, p + normal, p - normal, c, 2.0);

                                // 2. 신호기 기둥 길이 결정: 원거리 신호기가 있으면 2.0, 없으면 1.0
                                let stem = if *has_distant { 2.0 } else { 1.0 };

                                // 3. 신호기 기둥: 트랙에서 신호등까지의 수직 기둥
                                // p(트랙 위치)에서 p + stem*tangent(신호등 위치)까지
                                ImDrawList_AddLine(draw_list, p, p + stem*tangent, c, 2.0);

                                // ===== 신호 상태별 색상 원 그리기 =====
                                // 각 신호 상태에 따라 다른 색상의 원을 그려서 신호 상태를 표시
                                for s in state.iter() {
                                    match s {
                                        ObjectState::SignalStop => {
                                            // 정지 신호: 빨간색 원
                                            let c = config.color_u32(RailUIColorName::CanvasSignalStop);
                                            // 위치: 기둥 끝에서 tangent 방향으로 한 단위 더 이동한 곳
                                            ImDrawList_AddCircleFilled(draw_list, p + stem*tangent + tangent, scale, c, 8);
                                        },
                                        ObjectState::SignalProceed => {
                                            // 진행 신호: 초록색 원
                                            let c = config.color_u32(RailUIColorName::CanvasSignalProceed);
                                            // 위치: 기둥 끝에서 tangent 방향으로 한 단위 더 이동한 곳
                                            ImDrawList_AddCircleFilled(draw_list, p + stem*tangent + tangent, scale, c, 8);
                                        },
                                        ObjectState::DistantStop if *has_distant => {
                                            // 원거리 정지 신호: 빨간색 원 (원거리 신호기가 있을 때만)
                                            let c = config.color_u32(RailUIColorName::CanvasSignalStop);
                                            // 위치: 기둥에서 1.5*tangent + normal 방향으로 이동한 곳
                                            ImDrawList_AddCircleFilled(draw_list, p + 1.5*tangent + normal, scale*0.8, c, 8);
                                        },
                                        ObjectState::DistantProceed => {
                                            // 원거리 진행 신호: 초록색 원
                                            let c = config.color_u32(RailUIColorName::CanvasSignalProceed);
                                            // 위치: 기둥에서 1.5*tangent + normal 방향으로 이동한 곳
                                            ImDrawList_AddCircleFilled(draw_list, p + 1.5*tangent + normal, scale*0.8, c, 8);
                                        },
                                        _ => {},
                                    };
                                }

                                // ===== 신호등 외곽선 그리기 =====
                                // 원거리 신호기 외곽선 (원거리 신호기가 있을 때만)
                                if *has_distant {
                                    // 위치: 기둥에서 1.5*tangent + normal 방향으로 이동한 곳
                                    // 크기: scale*0.8 (메인 신호기보다 작음)
                                    ImDrawList_AddCircle(draw_list, p + 1.5*tangent + normal, scale*0.8, c, 8, 2.0);
                                }

                                // 메인 신호기 외곽선
                                // 위치: 기둥 끝에서 tangent 방향으로 한 단위 더 이동한 곳
                                // 크기: scale (원거리 신호기보다 큼)
                                ImDrawList_AddCircle(draw_list, p + stem*tangent + tangent, scale, c, 8, 2.0);

                                // ===== 메인 신호기 ID 텍스트 렌더링 =====
                                // 신호기 ID가 있으면 신호기 기둥 아래에 텍스트를 표시
                                if let Function::Signal { id: Some(id), .. } = f {
                                  // ===== 신호기 방향에 따른 텍스트 위치 조정 =====
                                  // 신호기가 왼쪽을 향하는지 오른쪽을 향하는지에 따라 텍스트 위치가 달라짐
                                  let id_len = id.len() as f32;

                                  // X축 오프셋: 신호기 방향에 따라 다르게 적용
                                  let x_offset = if self.tangent.x < 0 {
                                    // 신호기가 왼쪽을 향할 때: 텍스트를 오른쪽으로 배치
                                    0.5 + id_len * 1.5 // 기본 오프셋 + 텍스트 길이에 비례한 추가 오프셋
                                  } else {
                                    // 신호기가 오른쪽을 향할 때: 텍스트를 왼쪽으로 배치
                                    - 3.0 - id_len * 9.0 // 기본 오프셋 + 텍스트 길이에 비례한 추가 오프셋
                                  };

                                  // Y축 오프셋: 신호기 방향과 관계없이 동일
                                  let y_offset = if self.tangent.x < 0 {
                                     -7.5 // 왼쪽을 향할 때 y offset -7.5
                                  } else {
                                     -7.5 // 오른쪽을 향할 때 y offset -7.5
                                  };

                                  // 최종 텍스트 위치 계산
                                  let screen_offset = ImVec2 { x: x_offset, y: y_offset };
                                  let text_pos = p + screen_offset;

                                  // 텍스트 색상: 검은색 (CanvasText)
                                  let text_color = config.color_u32(RailUIColorName::CanvasText);

                                  // CString으로 변환 (ImGui 텍스트 렌더링용)
                                  let text_ptr = std::ffi::CString::new(id.as_str()).unwrap();

                                  // 텍스트 렌더링
                                  ImDrawList_AddText(draw_list, text_pos, text_color, text_ptr.as_ptr(), std::ptr::null());
                            }
                            },
                            Some(SignalType::Shunting) => {
                                // 기존 ShuntingSignal 렌더링 코드
                                // ===== 입환신호기 기본 구조 그리기 =====
                                // 1. 신호기 수평 라인: 트랙에 수직인 수평선 (메인 신호기와 동일)
                                ImDrawList_AddLine(draw_list, p + normal, p - normal, c, 2.0);

                                // 2. 신호기 기둥 길이 결정: 원거리 신호기가 있으면 2.0, 없으면 1.0
                                let stem = if *has_distant { 2.0 } else { 1.0 };

                                // 3. 신호기 기둥: 트랙에서 신호등까지의 수직 기둥
                                ImDrawList_AddLine(draw_list, p, p + mul_imvec2(tangent, stem), c, 2.0);

                                // ===== 1/4 원 그리기를 위한 벡터 계산 =====
                                // tangent의 수직 벡터 계산 (한 번만 계산해서 재사용)
                                // tangent = (tx, ty)일 때 normal = (ty, -tx)로 계산 (90도 회전)
                                let normal = ImVec2 { x: tangent.y, y: -tangent.x };

                                // normal 벡터 정규화 (길이 1로 만듦)
                                let normal_len = (normal.x * normal.x + normal.y * normal.y).sqrt();
                                let n = if normal_len > 0.0 {
                                    ImVec2 { x: normal.x / normal_len, y: normal.y / normal_len }
                                } else { ImVec2 { x: 0.0, y: 1.0 } };

                                // 1/4 원의 중심점 오프셋: 신호기 수평 라인 길이의 1/2만큼 normal 방향으로 이동
                                let offset = scale * 1.0;

                                // ===== 신호 상태별 1/4 원 그리기 =====
                                // 입환신호기는 항상 tangent 방향 기준 0~π/2 (90도) 범위의 1/4 원을 그림
                                for s in state.iter() {
                                    // ===== 신호 상태에 따른 색상 결정 =====
                                    let color = match s {
                                        ObjectState::SignalStop => config.color_u32(RailUIColorName::CanvasSignalStop),      // 정지: 빨간색
                                        ObjectState::SignalProceed => config.color_u32(RailUIColorName::CanvasSignalProceed), // 진행: 초록색
                                        ObjectState::DistantStop if *has_distant => config.color_u32(RailUIColorName::CanvasSignalStop), // 원거리 정지: 빨간색
                                        ObjectState::DistantProceed => config.color_u32(RailUIColorName::CanvasSignalProceed),           // 원거리 진행: 초록색
                                        _ => continue, // 해당 상태가 아니면 건너뜀
                                    };

                                    // ===== 1/4 원의 중심(base) 위치 결정 =====
                                    let base = match s {
                                        ObjectState::DistantStop | ObjectState::DistantProceed =>
                                            // 원거리 신호: 기둥에서 1.5*tangent + normal 방향으로 이동
                                            p + mul_imvec2(tangent, 1.5) + mul_imvec2(n, offset),
                                        _ =>
                                            // 메인 신호: 기둥 끝에서 normal 방향으로 이동
                                            p + mul_imvec2(tangent, stem) + mul_imvec2(n, offset),
                                    };

                                    // ===== 1/4 원의 반지름(크기) 결정 =====
                                    let size = match s {
                                        ObjectState::DistantStop | ObjectState::DistantProceed => scale * 1.5, // 원거리 신호: 작은 크기
                                        _ => scale * 2.0, // 메인 신호: 큰 크기
                                    };

                                    // ===== 1/4 원 그리기 알고리즘 =====
                                    // tangent 벡터(신호기 방향)를 정규화(길이 1로 만듦)
                                    let tangent_len = (tangent.x * tangent.x + tangent.y * tangent.y).sqrt();
                                    let t = if tangent_len > 0.0 {
                                        ImVec2 { x: tangent.x / tangent_len, y: tangent.y / tangent_len }
                                    } else { ImVec2 { x: 1.0, y: 0.0 } };

                                    // tangent 벡터의 각도를 구함 (신호기 방향이 기준이 됨)
                                    // atan2(y, x)는 (x, y) 벡터의 각도를 반환
                                    let t_angle = t.y.atan2(t.x);

                                    // ===== 1/4 원의 각도 범위 계산 =====
                                    // 아크의 시작 각도: tangent 방향 (신호기가 향하는 방향)
                                    let a0 = t_angle;
                                    // 아크의 끝 각도: 시작 각도에서 90도(π/2) 더한 값
                                    // 즉, tangent 방향에서 90도 시계방향으로 회전한 지점까지
                                    let a1 = a0 + std::f32::consts::FRAC_PI_2;

                                    // 아크를 그릴 때 사용할 세그먼트 개수(곡선의 부드러움)
                                    let num_segments = 16;

                                    // ===== 1/4 원의 시작점 좌표 계산 =====
                                    // base를 중심으로 하고 size를 반지름으로 하는 원에서 a0 각도에 해당하는 점
                                    let start_pt = ImVec2 {
                                        x: base.x + size * a0.cos(),
                                        y: base.y + size * a0.sin(),
                                    };

                                    // ===== 1/4 원 그리기 =====
                                    // 그리기 전에 패스(경로) 초기화
                                    ImDrawList_PathClear(draw_list);

                                    // 1/4 원 아크를 패스에 추가 (base를 중심, size를 반지름, a0~a1 각도)
                                    ImDrawList_PathArcTo(draw_list, base, size, a0, a1, num_segments);

                                    // 아크의 끝점에서 중심(base)까지 직선을 패스에 추가
                                    ImDrawList_PathLineTo(draw_list, base);

                                    // 중심(base)에서 아크의 시작점까지 직선을 패스에 추가
                                    ImDrawList_PathLineTo(draw_list, start_pt);

                                    // 패스를 채워서 닫힌 도형을 그림 (색상: color)
                                    // 이렇게 하면 1/4 원 + 두 개의 직선으로 구성된 부채꼴 모양이 완성됨
                                    ImDrawList_PathFillConvex(draw_list, color);
                                }

                                // ===== 입환신호기 외곽선 그리기 =====
                                // 원거리 신호기 테두리 1/4 원 (원거리 신호기가 있을 때만)
                                if *has_distant {
                                    // ===== 원거리 신호기 외곽선 계산 =====
                                    // 1/4 원의 중심(base) 위치: 기둥에서 1.5*tangent + normal 방향으로 이동
                                    let base = p + mul_imvec2(tangent, 1.5) + mul_imvec2(n, offset);
                                    // 1/4 원의 반지름(크기): 메인 신호기보다 작음
                                    let size = scale * 1.5;

                                    // tangent 벡터 정규화
                                    let tangent_len = (tangent.x * tangent.x + tangent.y * tangent.y).sqrt();
                                    let t = if tangent_len > 0.0 {
                                        ImVec2 { x: tangent.x / tangent_len, y: tangent.y / tangent_len }
                                    } else { ImVec2 { x: 1.0, y: 0.0 } };

                                    // tangent 벡터의 각도
                                    let t_angle = t.y.atan2(t.x);

                                    // 아크의 시작/끝 각도 (항상 tangent 방향 기준 0~π/2)
                                    let a0 = t_angle;
                                    let a1 = a0 + std::f32::consts::FRAC_PI_2;
                                    let num_segments = 16;

                                    // 아크의 시작점 좌표 계산
                                    let start_pt = ImVec2 {
                                        x: base.x + size * a0.cos(),
                                        y: base.y + size * a0.sin(),
                                    };

                                    // ===== 원거리 신호기 외곽선 그리기 =====
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

                                // ===== 메인 신호기 외곽선 그리기 =====
                                // 메인 신호기 테두리 1/4 원 (항상 그려짐)
                                // 중심(base) 위치: 기둥 끝에서 normal 방향으로 이동
                                let base = p + mul_imvec2(tangent, stem) + mul_imvec2(n, offset);
                                // 반지름(크기): 원거리 신호기보다 큼
                                let size = scale * 2.0;

                                // tangent 벡터 정규화
                                let tangent_len = (tangent.x * tangent.x + tangent.y * tangent.y).sqrt();
                                let t = if tangent_len > 0.0 {
                                    ImVec2 { x: tangent.x / tangent_len, y: tangent.y / tangent_len }
                                } else { ImVec2 { x: 1.0, y: 0.0 } };

                                // tangent 벡터의 각도
                                let t_angle = t.y.atan2(t.x);

                                // 아크의 시작/끝 각도 (항상 tangent 방향 기준 0~π/2)
                                let a0 = t_angle;
                                let a1 = a0 + std::f32::consts::FRAC_PI_2;
                                let num_segments = 16;

                                // 아크의 시작점 좌표 계산
                                let start_pt = ImVec2 {
                                    x: base.x + size * a0.cos(),
                                    y: base.y + size * a0.sin(),
                                };

                                // ===== 메인 신호기 외곽선 그리기 =====
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

                                // ===== 입환신호기 ID 텍스트 렌더링 =====
                                // 신호기 ID가 있으면 신호기 기둥 아래에 텍스트를 표시
                                if let Function::Signal { id: Some(id), .. } = f {
                                    // ===== 신호기 방향에 따른 텍스트 위치 조정 =====
                                    // 신호기가 왼쪽을 향하는지 오른쪽을 향하는지에 따라 텍스트 위치가 달라짐
                                    let id_len = id.len() as f32;

                                    // X축 오프셋: 신호기 방향에 따라 다르게 적용
                                    let x_offset = if self.tangent.x < 0 {
                                        // 신호기가 왼쪽을 향할 때: 텍스트를 오른쪽으로 배치
                                        0.5 + id_len * 1.5 // 기본 오프셋 + 텍스트 길이에 비례한 추가 오프셋
                                    } else {
                                        // 신호기가 오른쪽을 향할 때: 텍스트를 왼쪽으로 배치
                                        - 3.0 - id_len * 9.0 // 기본 오프셋 + 텍스트 길이에 비례한 추가 오프셋
                                    };

                                    // Y축 오프셋: 신호기 방향과 관계없이 동일
                                    let y_offset = if self.tangent.x < 0 {
                                        -7.5 // 왼쪽을 향할 때 y offset -7.5
                                    } else {
                                        -7.5 // 오른쪽을 향할 때 y offset -7.5
                                    };

                                    // 최종 텍스트 위치 계산
                                    let screen_offset = ImVec2 { x: x_offset, y: y_offset };
                                    let text_pos = p + screen_offset;

                                    // 텍스트 색상: 검은색 (CanvasText)
                                    let text_color = config.color_u32(RailUIColorName::CanvasText);

                                    // CString으로 변환 (ImGui 텍스트 렌더링용)
                                    let text_ptr = std::ffi::CString::new(id.as_str()).unwrap();

                                    // 텍스트 렌더링
                                    ImDrawList_AddText(draw_list, text_pos, text_color, text_ptr.as_ptr(), std::ptr::null());
                                }
                            },
                            None => {
                                // ===== 기본 신호기 렌더링 =====
                                // 기본 신호기는 아무것도 그리지 않음 (signal_props가 없는 경우)
                            }
                        }
                    },
                    // ===== 선로전환기(Switch) 렌더링 =====
                    // base(수평선)와 stem(기둥)을 직사각형으로 교체, 원(신호등)은 외곽선만 그림
                    Function::Switch { id: _ } => {
                        let offset = -5.0; // normal 방향 offset

                        // 크기 설정
                        let stem_height = scale * 3.0;  // stem(세로 직사각형) 길이
                        let stem = 1.0;

                        // 이 스위치가 배치될 때의 각도 사용 (저장되지 않았으면 기본값 0도)
                        let angle_degrees = self.placed_angle.unwrap_or(0.0);

                        // 벡터 정규화
                        let tangent_len = (tangent.x * tangent.x + tangent.y * tangent.y).sqrt();
                        let t = if tangent_len > 0.0 {
                            ImVec2 { x: tangent.x / tangent_len, y: tangent.y / tangent_len }
                        } else { ImVec2 { x: 1.0, y: 0.0 } };
                        let normal_len = (normal.x * normal.x + normal.y * normal.y).sqrt();
                        let n = if normal_len > 0.0 {
                            ImVec2 { x: normal.x / normal_len, y: normal.y / normal_len }
                        } else { ImVec2 { x: 0.0, y: 1.0 } };

                        // stem(세로 직사각형) 중심:
                        let stem_center = p; // 마우스 포인터가 stem 중심

                        // 원(circle) 위치 및 크기 (stem_center 기준 normal 방향)
                        let circle_size = scale * 1.5;
                        let circle_diameter = circle_size * 2.0;
                        let stem_width = circle_diameter * 2.5;
                        let circle_offset = stem_width/2.0 + circle_size + 1.0;
                        // tangent unit vector 계산
                        let tangent_len = (tangent.x * tangent.x + tangent.y * tangent.y).sqrt();
                        let tangent_unit = if tangent_len > 0.0 {
                            ImVec2 { x: tangent.x / tangent_len, y: tangent.y / tangent_len }
                        } else {
                            ImVec2 { x: 1.0, y: 0.0 }
                        };
                        let circle_pos = stem_center + mul_imvec2(tangent_unit, circle_offset);

                        // stem(세로 직사각형) 네 꼭짓점 (width=circle_diameter*2.5, height=stem_height)
                        let stem_width = circle_diameter * 2.5;
                        let stem_t = t;
                        let stem_n = n;
                        let stem_tl = stem_center + mul_imvec2(stem_t, -stem_width/2.0) + mul_imvec2(stem_n, -stem_height/2.0);
                        let stem_tr = stem_center + mul_imvec2(stem_t, stem_width/2.0) + mul_imvec2(stem_n, -stem_height/2.0);
                        let stem_br = stem_center + mul_imvec2(stem_t, stem_width/2.0) + mul_imvec2(stem_n, stem_height/2.0);
                        let stem_bl = stem_center + mul_imvec2(stem_t, -stem_width/2.0) + mul_imvec2(stem_n, stem_height/2.0);

                        // stem(세로 직사각형) 그리기
                        ImDrawList_AddLine(draw_list, stem_tl, stem_tr, c, 2.0);
                        ImDrawList_AddLine(draw_list, stem_tr, stem_br, c, 2.0);
                        ImDrawList_AddLine(draw_list, stem_br, stem_bl, c, 2.0);
                        ImDrawList_AddLine(draw_list, stem_bl, stem_tl, c, 2.0);

                        // ===== 스위치 원(circle) 외곽선만 그리기 =====
                        ImDrawList_AddCircle(draw_list, circle_pos, circle_size, c, 20, 2.0);

                        // ===== 스위치 ID 텍스트 렌더링 =====
                        if let Function::Switch { id: Some(id) } = f {
                            // 기울기에 따른 offset 조정
                            let x_offset = if angle_degrees <= -40.0 && angle_degrees >= -50.0 {
                                -23.0  // -45도 근처일 때
                            } else if angle_degrees <= -130.0 && angle_degrees >= -140.0 {
                                -46.0  // -45도 근처일 때
                            } else {
                                3.5   // 기본값 (0도, 180도, 45도, -135도 등)
                            };

                            // y offset 조정: 135도와 -45도 근처에서는 0, 그 외에는 -1.5
                            let y_offset = if (angle_degrees >= 130.0 && angle_degrees <= 140.0) ||
                                        (angle_degrees >= -50.0 && angle_degrees <= -40.0) ||
                                        (angle_degrees >= 40.0 && angle_degrees <= 50.0) ||
                                        (angle_degrees >= -140.0 && angle_degrees <= -130.0) {
                                0.0   // 135도 또는 -45도 근처일 때
                            } else {
                                -1.5  // 그 외의 경우 (0도, 180도 등)
                            };

                            let offset_vec = ImVec2 { x: x_offset, y: y_offset };

                            // 기울기에 따라 꼭지점 선택
                            let text_pos = if angle_degrees >= 130.0 && angle_degrees <= 140.0 {
                                // 135도 근처: stem_tr
                                stem_tl + offset_vec
                            } else if angle_degrees >= -50.0 && angle_degrees <= -40.0 {
                                // -45도 근처: stem_bl
                                stem_tl + offset_vec
                            } else if angle_degrees >= 40.0 && angle_degrees <= 50.0 {
                                // 45도 근처: stem_br
                                stem_tl + offset_vec
                            } else if angle_degrees >= -140.0 && angle_degrees <= -130.0 {
                                // -135도 근처: stem_tl
                                stem_tl + offset_vec
                            } else {
                                // 그 외의 경우 (0도, 180도 등): 기존 로직 (n.y에 따라 stem_tr 또는 stem_bl)
                                (if n.y > 0.0 { stem_tr } else { stem_bl }) + offset_vec
                            };

                            let text_color = config.color_u32(RailUIColorName::CanvasText);
                            let text_ptr = std::ffi::CString::new(id.as_str()).unwrap();
                            ImDrawList_AddText(draw_list, text_pos, text_color, text_ptr.as_ptr(), std::ptr::null());
                        }
                    }
                }
            }
        }
    }
}