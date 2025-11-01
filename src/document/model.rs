use nalgebra_glm as glm;
use crate::document::objects::*;
use crate::document::infview::*;
use crate::util::*;
use ordered_float::OrderedFloat;
use serde::{Serialize,Deserialize};

use std::sync::Arc;

/// 선로 좌/우 측을 나타낸다.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[derive(Serialize,Deserialize)]
pub enum Side {
    Left, Right
}

/// 분기기(스위치)의 상태.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[derive(Serialize,Deserialize)]
pub enum SwitchState {
    Straight,  // 직선 주행 상태
    Diverging, // 꺾임 주행 상태
}

impl Side {
    /// 반대편 측을 반환한다.
    pub fn opposite(&self) -> Side {
        match self {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        }
    }

    /// 측을 포트 표현으로 변환한다.
    pub fn as_port(&self) -> Port {
        match self {
            Side::Left => Port::Right,
            Side::Right => Port::Left,
        }
    }

    /// 측을 rolling 입력 포맷의 스위치 포지션으로 변환한다.
    pub fn as_switch_position(&self) -> rolling::input::staticinfrastructure::SwitchPosition {
        match self {
            Side::Left => rolling::input::staticinfrastructure::SwitchPosition::Left,
            Side::Right => rolling::input::staticinfrastructure::SwitchPosition::Right,
        }
    }
}



pub type Pt = glm::I32Vec2;
pub type PtA = glm::I32Vec2;
pub type PtC = glm::Vec2;
pub type Vc = Pt;


/// 차량(열차 편성)의 기본 스펙.
#[derive(Clone)]
#[derive(Debug)]
#[derive(Serialize,Deserialize)]
pub struct Vehicle {
    pub name :String,
    pub length: f32,
    pub max_acc :f32,
    pub max_brk :f32,
    pub max_vel :f32,
}

impl Default for Vehicle {
    /// 합리적인 기본 파라미터를 제공한다.
    fn default() -> Vehicle { Vehicle {
        name : "Vehicle 1".to_string(),
        length: 210.0,
        max_acc: 0.9,
        max_brk: 0.85,
        max_vel: 50.0,
    } }
}

/// 평면 교차기 유형.
#[derive(Debug,Copy,Clone, PartialEq, Eq)]
#[derive(Serialize,Deserialize)]
pub enum CrossingType { 
    Crossover,
    SingleSlip(Side), // LEft means switching left when traveling with increasing X coord.
    DoubleSlip,
}

/// 노드(접속점)의 유형.
#[derive(Debug,Copy,Clone, PartialEq, Eq)]
#[derive(Serialize,Deserialize)]
pub enum NDType { OpenEnd, BufferStop, Cont, Sw(Side, SwitchState), Crossing(CrossingType), Err }
// TODO crossing switchable, crossing orthogonal?, what settings does a crossing have?
// Assuming non-switched crossing for now.

/// 접속 포트를 추상화한다(선로 끝/분기/교차 포트 등).
#[derive(Debug,Copy,Clone,PartialEq,Eq,Hash)]
pub enum Port { End, ContA, ContB, Left, Right, Trunk, Err, Cross(AB,usize) }
// Crossing has AB as different sides of opposing ports, and usize as the different pairs of edges

impl Port {
    /// 같은 노드에서 서로 반대에 해당하는지 판정한다.
    pub fn is_opposite(&self, other: &Port) -> bool {
        match (self,other) {
            (Port::ContA, Port::ContB) => true,
            (Port::ContB, Port::ContA) => true,
            (Port::Left, Port::Trunk) => true,
            (Port::Right, Port::Trunk) => true,
            (Port::Trunk, Port::Left) => true,
            (Port::Trunk, Port::Right) => true,
            (Port::Cross(a,n),(Port::Cross(b,m))) => n == m && a != b,
            _ => false,
        }
    }
}


/// 선로의 양단을 구분하는 A/B 플래그.
#[derive(Debug,Copy,Clone,PartialEq,Eq,Hash)]
pub enum AB { A, B }

impl AB {
    /// 반대 단을 반환한다.
    pub fn other(&self) -> AB {
        match self {
            AB::A => AB::B,
            AB::B => AB::A,
        }
    }

    /// A=+1, B=-1 부호를 제공한다.
    pub fn factor(&self) -> f64 {
        match self {
            AB::A =>  1.0,
            AB::B => -1.0,
        }
    }
}

/// 경로 지정 스펙: `from` → `to`, 대안 번호 포함.
#[derive(Copy, Clone)]
#[derive(Debug)]
#[derive(Hash, PartialEq, Eq)]
#[derive(Serialize,Deserialize)]
pub struct RouteSpec {
    pub from: Ref,
    pub to: Ref,
    pub alternative: usize,
}

/// 운전 명령.
#[derive(Copy, Clone)]
#[derive(Debug)]
#[derive(Serialize,Deserialize)]
pub enum Command {
    Train(usize, RouteSpec),
    Route(RouteSpec),
}

pub type Commands = Vec<(usize,(f64,Command))>;

/// 운전 명령 집합(디스패치). 삽입 시 시간 정렬을 유지한다.
#[derive(Serialize,Deserialize)]
#[derive(Debug, Clone)]
pub struct Dispatch {
    pub name :String,
    generation :usize,
    pub commands :Vec<(usize,(f64,Command))>,
}

impl Dispatch {
    /// 비어 있는 디스패치를 생성한다.
    pub fn new_empty(name :String) -> Dispatch {
        Dispatch {
            name: name,
            generation :0,
            commands :Vec::new(),
        }
    }

    /// 명령 벡터에서 디스패치를 구성한다(세대=길이).
    pub fn from_vec(name :String, commands :Vec<(usize, (f64,Command))>) -> Dispatch {
        let l = commands.len();
        Dispatch {
            name: name, 
            generation: l,
            commands: commands,
        }
    }

    /// 시간 기준으로 정렬을 유지하며 명령을 삽입하고 ID를 반환한다.
    pub fn insert(&mut self, t :f64, cmd :Command) -> usize {
        let id = self.generation;
        self.generation += 1;
        let idx = match self.commands.binary_search_by_key(&OrderedFloat(t),
                |(_,(t,_))| OrderedFloat(*t)) { Ok(i) | Err(i) => i };
        self.commands.insert(idx, (id,(t,cmd)));
        id 
    }

}

/// 운전 계획 스펙: 열차-방문 리스트와 방문 간 순서 제약.
#[derive(Clone, Debug)]
#[derive(Serialize,Deserialize)]
pub struct PlanSpec {
    pub name :String,
    pub trains: ImShortGenList<(Option<ListId>, ImShortGenList<Visit>)>,
    pub order :Vec<(VisitRef,VisitRef,Option<f64>)>,
}

impl PlanSpec {
    /// 비어 있는 계획을 생성한다.
    pub fn new_empty(name :String) -> Self {
        PlanSpec {
            name: name,
            trains: Default::default(),
            order: Default::default(),
        }
    }
}

pub type VisitRef = (ListId,ListId);

/// 방문: 위치 후보들과 체류 시간.
#[derive(Clone, Debug)]
#[derive(Serialize,Deserialize)]
pub struct Visit {
    pub locs :Vec<PlanLoc>,
    pub dwell :Option<f64>,
}

pub type PlanLoc = Result<Ref,PtC>;

pub type ListId = usize;

#[derive(Clone)]
#[derive(Debug)]
#[derive(Serialize,Deserialize)]
pub struct ShortGenList<T> {
    generation :ListId,
    list :Vec<(ListId,T)>,
}

/// Stupid persistent usize-indexed data structure, Vec-backed, 
/// always copies the whole Vec when editing after sharing. 
/// And iterates over the whole Vec to look up by usize-id.
/// 간단한 세대 기반 불변 리스트(usize ID). 공유 후 수정 시 전체 복사됨.
#[derive(Clone)]
#[derive(Debug)]
#[derive(Serialize,Deserialize)]
pub struct ImShortGenList<T>(Arc<ShortGenList<T>>);

/// `Dispatch n` 형식의 고유 이름을 생성.
pub fn generate_unique_dispatch_name(dispatches: &ImShortGenList<Dispatch>) -> String {
        let mut used_numbers = std::collections::HashSet::new(); // dispatch 이름 생성에 이미 쓰인 숫자들 담음
        for (_id, d) in dispatches.iter() { // 모든 dispatch들을 선회
            if let Some(stripped) = d.name.strip_prefix("Dispatch ") { // 이름이 Dispatch로 시작되는지 확인
                if let Ok(n) = stripped.parse::<usize>() { // Dispatch 이름 뒤에 숫자 파싱
                    used_numbers.insert(n); // 파싱 성공하면 그 숫자 넣음
                }
            }
        }
        let mut n = 1; // 시작 값을 1로 설정
        while used_numbers.contains(&n) {
            n += 1; // Dispatch n에 n이 1부터 시작해서, 이미 쓰인 값이면 값을 더해나감
        }
        format!("Dispatch {}", n) // 최종적인 숫자를 Dispatch n 형태로 만듦
}
/// `Plan n` 형식의 고유 이름을 생성.
pub fn generate_unique_plan_name(plans: &ImShortGenList<PlanSpec>) -> String {
    let mut used_numbers = std::collections::HashSet::new();
    for (_id, p) in plans.iter() {
        if let Some(stripped) = p.name.strip_prefix("Plan ") {
            if let Ok(n) = stripped.parse::<usize>() {
                used_numbers.insert(n);
            }
        }
    }
    let mut n = 1;
    while used_numbers.contains(&n) {
        n += 1;
    }
    format!("Plan {}", n) //Plan도 Dispatch랑 같은 방법으로 만듦(generate_unique_dispatch_name과 방식 같음)
}

impl<T :Clone> ImShortGenList<T> {
    pub fn next_id(&self) -> usize {
        self.0.generation
    }

    pub fn data(&self) -> &[(usize,T)] {
        &self.0.list
    }

    pub fn insert(&mut self, t :T) -> ListId {
        let pos = self.0.list.len();
        self.insert_at(pos, t)
    }

    pub fn insert_before(&mut self, idx :ListId, t :T) -> ListId {
        let pos = self.0.list.iter().position(|(i,_)| *i == idx).unwrap_or(self.0.list.len());
        self.insert_at(pos, t)
    }

    fn insert_at(&mut self, pos :usize, t: T) -> ListId {
        let inner = Arc::make_mut(&mut self.0);
        let id = inner.generation;
        inner.generation += 1;
        inner.list.insert(pos, (id,t));
        id
    }

    pub fn get(&self, id :ListId) -> Option<&T> {
        self.0.list.iter().find(|c| c.0 == id).map(|c| &c.1)
    }

    pub fn get_mut(&mut self, id :ListId) -> Option<&mut T> {
        Arc::make_mut(&mut self.0).list.iter_mut().find(|c| c.0 == id).map(|c| &mut c.1)
    }

    pub fn remove(&mut self, id :ListId) -> Option<T> {
        let pos = self.0.list.iter().position(|c| c.0 == id)?;
        let inner = Arc::make_mut(&mut self.0);
        Some(inner.list.remove(pos).1)
    }

    pub fn iter(&self) -> impl Iterator<Item = &(usize, T)> {
        self.0.list.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&usize, &mut T)> {
        Arc::make_mut(&mut self.0).list.iter_mut().map(|(a,b)| (&*a, b))
    }

    pub fn new() -> Self {
        Self(Arc::new(ShortGenList { generation: 0, list :Vec::new() }))
    }

}

impl<T:Clone> Default for ImShortGenList<T> {
    fn default() -> Self { ImShortGenList(Arc::new(ShortGenList { generation: 0, list :Vec::new() })) }
}

/// 편집 가능한 핵심 모델(인프라/오브젝트/노드/차량/디스패치/계획).
#[derive(Clone, Default)]
#[derive(Debug)]
#[derive(Serialize,Deserialize)]
pub struct Model {
    pub linesegs: im::HashSet<(Pt,Pt)>,
    pub objects: im::HashMap<PtA, Object>,
    pub node_data: im::HashMap<Pt, NDType>,
    pub vehicles :ImShortGenList<Vehicle>, 
    pub dispatches :ImShortGenList<Dispatch>,
    pub plans :ImShortGenList<PlanSpec>,
}


/// 모델 요소 참조(노드/선분/오브젝트).
#[derive(Hash,PartialEq,Eq)]
#[derive(Copy,Clone)]
#[derive(Debug)]
#[derive(Serialize,Deserialize)]
pub enum Ref {
    Node(Pt),
    LineSeg(Pt,Pt),
    Object(PtA),
}

fn closest_pts(pt :PtC) -> [(Pt,Pt);2] {
    let x_lo = pt.x.floor() as i32;
    let x_hi = pt.x.ceil()  as i32;
    let y_lo = pt.y.floor() as i32;
    let y_hi = pt.y.ceil()  as i32;
    
    [
        (glm::vec2(x_lo,y_lo),glm::vec2(x_hi,y_hi)),
        (glm::vec2(x_lo,y_hi),glm::vec2(x_hi,y_lo)),
    ]
}

pub fn corners(pt :PtC) -> Vec<Pt> {
    use itertools::iproduct;
    use nalgebra_glm::vec2; 
    iproduct!(
        [0.0,1.0].iter().map(|d| (pt.x + d).floor() as i32),
        [0.0,1.0].iter().map(|d| (pt.y + d).floor() as i32))
        .map(|(x,y)| vec2(x,y)).collect()
}

impl Model {
    /// 기본 차량 1개를 포함한 빈 모델 생성.
    pub fn empty() -> Self { 
        let mut model : Model = Default::default();
        model.vehicles.insert(Default::default());
        model
    }

    /// 좌표에 가장 가까운 오브젝트와 거리 제곱을 반환한다.
    pub fn get_closest_object<'a>(&'a self, pt :PtC) -> Option<((&'a PtA,&'a Object),f32)> {
        // TODO performance
        let (mut thing, mut dist_sqr) = (None, std::f32::INFINITY);
        for (p,o) in self.objects.iter() {
            let d = glm::length2(&(unround_coord(*p) - pt));
            if d < dist_sqr {
                thing = Some((p,o));
                dist_sqr = d;
            }
        }
        thing.map(|o| (o,dist_sqr))
    }

    /// 좌표에 가장 가까운 선분과 거리 정보를 반환한다.
    pub fn get_closest_lineseg(&self, pt :PtC) -> Option<((Pt,Pt),f32,(f32,f32))> {
        // TODO performance
        let (mut thing,mut dist_sqr,mut next_dist) = (None, std::f32::INFINITY, std::f32::INFINITY);
        for x1 in [pt.x.floor() as i32, (pt.x + 1.0).floor() as i32].iter().cloned() {
        for y1 in [pt.y.floor() as i32, (pt.y + 1.0).floor() as i32].iter().cloned() {
        for x2 in [pt.x.floor() as i32, (pt.x + 1.0).floor() as i32].iter().cloned() {
        for y2 in [pt.y.floor() as i32, (pt.y + 1.0).floor() as i32].iter().cloned() {
            let l = (glm::vec2(x1,y1),glm::vec2(x2,y2));
            if self.linesegs.contains(&l) {
                let (d,param) = dist_to_line_sqr(pt, 
                                         glm::vec2(l.0.x as _ ,l.0.y as _ ), 
                                         glm::vec2(l.1.x as _ ,l.1.y as _ ));
                if d < dist_sqr {
                    next_dist = dist_sqr;
                    dist_sqr = d;
                    thing = Some((l,param));
                } else if d < next_dist {
                    next_dist = d;
                }
            }
        }
        }
        }
        }
        thing.map(|(tr,param)| (tr,param,(dist_sqr,next_dist)))
    }

    /// 직사각형 영역과 교차하는 선분 리스트를 반환한다.
    pub fn get_linesegs_in_rect(&self, a :PtC, b :PtC) -> Vec<(Pt,Pt)> {
        let mut output = Vec::new();
        for (p1,p2) in &self.linesegs {
            if in_rect(glm::vec2(p1.x as _, p1.y as _), a, b) || 
               in_rect(glm::vec2(p2.x as _, p2.y as _), a, b) {
                output.push((*p1,*p2));
            }
        }
        output
    }

    /// 주어진 지점에 Detector 기능 오브젝트가 존재하는지 확인한다.
    pub fn has_detector_at(&self, pt: PtC) -> bool {
        let pt_rounded = round_coord(pt);
        if let Some(obj) = self.objects.get(&pt_rounded) {
            obj.functions.iter().any(|f| matches!(f, Function::Detector))
        } else {
            false
        }
    }

    /// 주어진 참조 대상(노드/선분/오브젝트)을 삭제한다.
    pub fn delete(&mut self, x :Ref) {
        match x {
            Ref::LineSeg(a,b) => { self.linesegs.remove(&(a,b)); },
            Ref::Node(a) => { self.node_data.remove(&a); },
            Ref::Object(p) => { self.objects.remove(&p); },
        }
    }


}

use std::collections::HashSet;
/// 편집 동작 클래스(동일 클래스는 스택에서 병합 가능).
#[derive(Debug, PartialEq, Eq)]
pub enum EditClass {
    MoveObjects(HashSet<Ref>),
    CommandTime(usize,usize),
    VehicleName(usize),
    VehicleLen(usize),
    VehicleAcc(usize),
    VehicleBrk(usize),
    VehicleVel(usize),

    DispatchName(usize),
    PlanName(usize),
}



/// Undo/Redo 가능한 히스토리 스택.
pub struct Undoable<T, C> {
    stack :Vec<T>,
    pointer: usize,
    class :Option<C>,
}

impl<T : Clone + Default, C : Eq> Undoable<T,C> {
    /// 현재 포인터/총 길이 정보를 문자열로 반환한다.
    pub fn info(&self) -> String {
        format!("Undo stack {}/{}", self.pointer, self.stack.len()-1)
    }

    /// 기본값에서 시작하는 스택을 생성한다.
    pub fn new() -> Undoable<T,C> {
        Self::from(Default::default())
    }

    /// 초기값으로부터 스택을 생성한다.
    pub fn from(x :T) -> Undoable<T,C> {
        Undoable {
            stack: vec![x],
            pointer: 0,
            class: None,
        }
    }

    /// 현재 상태를 참조한다.
    pub fn get(&self) -> &T {
        &self.stack[self.pointer]
    }

    /// 새 상태를 기록한다. 동일 클래스 연속 편집은 덮어쓴다.
    pub fn set(&mut self, v :T, cl :Option<C>) {
        if cl.is_some() && self.class == cl {
            // replace the object if class matches
            self.stack[self.pointer] = v;
        } else {
            self.pointer += 1;
            self.stack.truncate(self.pointer);
            self.stack.push(v);
        }
        self.class = cl;
    }

    /// Undo 가능 여부.
    pub fn can_undo(&self) -> bool {
        self.pointer > 0
    }

    /// Redo 가능 여부.
    pub fn can_redo(&self) -> bool {
        self.pointer + 1 < self.stack.len()
    }

    /// 한 단계 Undo.
    pub fn undo(&mut self) -> bool {
        if self.pointer > 0 {
            self.pointer -= 1;
            self.class = None;
            true
        } else {
            false 
        }
    }

    /// 한 단계 Redo.
    pub fn redo(&mut self) -> bool {
        if self.pointer + 1 < self.stack.len() {
            self.pointer += 1;
            self.class = None;
            true
        } else {
            false 
        }
    }

    /// 현재 편집 클래스를 강제로 지정한다(스택 병합 제어).
    pub fn override_edit_class(&mut self, cl :C) {
        self.class = Some(cl);
    }
}
