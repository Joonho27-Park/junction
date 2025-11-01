// core model
pub mod model;
pub mod objects;

// derived data updates
pub mod analysis;

// derived data computation
pub mod dgraph;
pub mod topology;
pub mod interlocking;
pub mod history;
pub mod dispatch;
pub mod mileage;
pub mod plan;

// graphical view representation
pub mod infview;
pub mod view;
//pub mod diagram;

use crate::file;
use crate::app::*;
use model::*;
use infview::*;
use log::*;
use crate::util;
use crate::util::VecMap;
use nalgebra_glm as glm;
use backend_glfw::imgui::ImVec2;

/// 시뮬레이션 코드의 루트 컨테이너.
///
/// `model`의 파생 데이터(`analysis`), 파일 저장 상태, 인프라 보기(`InfView`),
/// 운전 시뮬레이션 보기, 시간 배속 등을 한 곳에서 관리한다.
///
/// # Fields
///
/// * `analysis` - `model`로부터 유도되는 파생 데이터와 백그라운드 갱신 로직
/// * `saved_model` - 최근 저장 시점의 모델 세대 번호(변경 감지용)
/// * `fileinfo` - 파일 경로/저장 여부 등 파일 메타데이터
/// * `inf_view` - 인프라(선로/신호/설비) 뷰 상태
/// * `dispatch_view` - 수동/자동 운전 다이어그램 보기 상태
/// * `time_multiplier` - 시뮬레이션 시간 배속
pub struct Document {
    pub analysis: analysis::Analysis,
    pub saved_model :usize,
    pub fileinfo :file::FileInfo,
    pub inf_view :InfView,
    pub dispatch_view :Option<DispatchView>,
    pub time_multiplier :f64,
}

impl BackgroundUpdates for Document {
    fn check(&mut self) {

        if *self.analysis.generation() != self.saved_model {
            self.fileinfo.set_unsaved();
        }

        self.analysis.check();
    }
}

impl Document {
    /// 빈 모델에서 시작하는 새 document를 생성한다.
    ///
    /// # Arguments
    ///
    /// * `bg` - 백그라운드 작업 큐 핸들
    ///
    /// # Returns
    ///
    /// 초기화된 `Document`.
    pub fn empty(bg :BackgroundJobs) -> Self {
        Self::from_model(model::Model::empty(), bg)
    }

    /// 주어진 `Model`을 기반으로 document를 생성한다.
    ///
    /// `analysis`를 초기화하고, 파일 정보/보기 상태/배속 값 등을 기본값으로 채운다.
    ///
    /// # Arguments
    ///
    /// * `model` - 초기 모델
    /// * `bg` - 백그라운드 작업 큐 핸들
    ///
    /// # Returns
    ///
    /// 초기화된 `Document`.
    pub fn from_model(model :model::Model, bg: BackgroundJobs) -> Self {
        Document {
            analysis: analysis::Analysis::from_model(model, bg),
            fileinfo: file::FileInfo::empty(),
            inf_view: InfView::default(),
            dispatch_view: None,
            time_multiplier: 15.0,
            saved_model: 0,
        }
    }

    /// 파일이 저장되었음을 기록하고, 저장 시점의 모델 세대(generation)를 동기화한다.
    ///
    /// # Arguments
    ///
    /// * `filename` - 저장된 파일 경로
    pub fn set_saved_file(&mut self, filename :String) {
        self.saved_model = *self.analysis.generation();
        self.fileinfo.set_saved_file(filename);
    }

}

/// 디스패치 모드.
///
/// 수동 조작 기반의 `Manual`과, Plan 기반의 `Auto`를 제공한다.
#[derive(Clone,Copy)]
pub enum DispatchView {
    Manual(ManualDispatchView),
    Auto(AutoDispatchView),
}

/// 수동 디스패치 뷰 상태.
///
/// 선택된 디스패치/재생 시간/뷰포트/선택 명령 등을 포함한다.
#[derive(Clone,Copy)]
pub struct ManualDispatchView {
    pub dispatch_idx :usize,
    pub time :f64,
    pub play :bool,
    pub action :ManualDispatchViewAction,
    pub viewport :Option<DiagramViewport>,
    pub selected_command :Option<usize>,
}

impl ManualDispatchView {
    /// 주어진 디스패치 인덱스(dispatch_idx)로 초기화한다.
    ///
    /// # Arguments
    ///
    /// * `idx` - `dispatches` 내 디스패치 인덱스
    ///
    /// # Returns
    ///
    /// 초기화된 `ManualDispatchView`.
    pub fn new(idx :usize) -> ManualDispatchView {
        ManualDispatchView {
            dispatch_idx: idx,
            time: 0.0,
            play: false,
            viewport: None,
            action: ManualDispatchViewAction::None,
            selected_command: None,
        }
    }
}

/// 수동 디스패치 뷰에서의 상호작용 상태.
#[derive(Clone,Copy)]
pub enum ManualDispatchViewAction {
    None,
    DragCommandTime {
        idx :usize,
        id :usize,
    }
}

/// 다이어그램 뷰의 시간/좌표 범위.
#[derive(Clone,Copy)]
pub struct DiagramViewport {
    pub time :(f64,f64),
    pub pos :(f64,f64),
}

/// 자동 운전 다이어그램 보기 상태.
///
/// 선택된 계획(`plan_idx`)과 액션, 필요 시 내장된 수동 보기 상태를 포함한다.
#[derive(Clone,Copy)]
pub struct AutoDispatchView {
    pub plan_idx :usize,
    pub action :PlanViewAction,
    pub dispatch :Option<ManualDispatchView>,
}



#[derive(Debug, Copy, Clone)]
#[derive(PartialEq, Eq, Hash)]
/// 열차 방문을 유일하게 식별하기 위한 키.
///
/// `train`/`visit` 인덱스와 선택적 `location`을 묶는다.
pub struct VisitKey { 
    pub train: usize, 
    pub visit: usize, 
    pub location: Option<usize> 
}

/// 계획 뷰 상호작용 상태.
#[derive(Clone,Copy)]
pub enum PlanViewAction {
    None,
    DragFrom(VisitKey, ImVec2),
    Menu(VisitKey, ImVec2),
}

impl UpdateTime for DispatchView {
    /// 재생 중일 때에만 내부 시간(`ManualDispatchView.time`)을 진행한다.
    fn advance(&mut self, dt :f64) {
        match self {
            DispatchView::Manual(m) |
            DispatchView::Auto(AutoDispatchView { dispatch: Some(m), .. }) 
                => { if m.play { m.time += dt; } },
            _ => {},
        }
    }
}

