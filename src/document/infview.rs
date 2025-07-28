use std::collections::HashSet;
use crate::document::model::{Ref, PtA, PtC, Pt};
use nalgebra_glm as glm;
use crate::document::view::*;
use crate::document::objects::*;
use crate::document::dispatch;
use crate::gui::ImVec2;


#[derive(Debug)]
pub struct InfView {
    pub action :Action,
    pub selection :HashSet<Ref>,
    pub view :View,
    pub instant_cache: dispatch::InstantCache,
    // 이름 입력 상태
    pub id_input: Option<IdInputState>,
    pub focused: bool, // 입력창/팝업 등 포커스가 필요한 UI가 떠 있을 때 true
    // 처리된 스위치 노드 추적
    pub processed_switch_nodes: Option<Vec<Pt>>,
}

#[derive(Debug)]
pub struct IdInputState {
    pub object: Object,
    pub id: String,
    pub position: PtC,
    pub function_type: Function,
}

#[derive(Debug)]
pub enum Action {
    Normal(NormalState),
    DrawingLine(Option<Pt>),
    SelectObjectType,
    InsertObject(Option<Object>),
}


#[derive(Debug,Copy,Clone)]
pub enum NormalState {
    Default,
    SelectWindow(ImVec2),
    DragMove(MoveType),
}


#[derive(Debug,Copy,Clone)]
pub enum MoveType { Grid(PtC), Continuous }

impl InfView {
    pub fn default() -> Self {
        InfView {
            action: Action::Normal(NormalState::Default),
            selection: HashSet::new(),
            view: View::default(),
            instant_cache: dispatch::InstantCache::new(),
            id_input: None,
            focused: false,
            processed_switch_nodes: None,
        }
    }
}



pub fn unround_coord(p :PtA) -> PtC {
    let coeff = 10.0;
    glm::vec2(p.x as f32 / coeff, p.y as f32 / coeff)
}
pub fn round_coord(p :PtC) -> PtA {
    let coeff = 10.0;
    glm::vec2((p.x * coeff) as _, (p.y * coeff) as _)
}

