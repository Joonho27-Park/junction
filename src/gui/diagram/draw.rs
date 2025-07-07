use const_cstr::*;
use matches::matches;
use backend_glfw::imgui::*;
use nalgebra_glm as glm;

use crate::config::*;
use crate::app::*;
use crate::document::dgraph::DGraph;
use crate::gui::widgets;
use crate::gui::widgets::Draw;
use crate::document::dispatch::*;
use crate::document::model::*;
use crate::document::analysis::*;
use crate::document::*;
use crate::gui::diagram::DiagramViewAction;
use crate::gui::infrastructure::draw::highlight_node;
use crate::document::infview::InfView;

pub fn diagram(config :&Config, graphics :&DispatchOutput, draw :&Draw, view :&DiagramViewport) {
    let col_res = config.color_u32(RailUIColorName::GraphBlockReserved);
    let col_box = config.color_u32(RailUIColorName::GraphBlockBorder);
    let col_occ = config.color_u32(RailUIColorName::GraphBlockOccupied);

    let col_train_front = config.color_u32(RailUIColorName::GraphTrainFront);
    let col_train_rear = config.color_u32(RailUIColorName::GraphTrainRear);

    unsafe {
        for block in &graphics.diagram.blocks {
            if block.reserved.0 < block.occupied.0 {
                ImDrawList_AddRectFilled(draw.draw_list,
                     to_screen(draw, view, block.reserved.0, block.pos.0),
                     to_screen(draw, view, block.occupied.0, block.pos.1),
                     col_res, 0.0, 0);
            }
            // Occupied
            ImDrawList_AddRectFilled(draw.draw_list,
                     to_screen(draw, view, block.occupied.0, block.pos.0),
                     to_screen(draw, view, block.occupied.1, block.pos.1),
                     col_occ, 0.0, 0);

            // Reserved after
            if block.reserved.1 > block.occupied.1 {
                ImDrawList_AddRectFilled(draw.draw_list,
                     to_screen(draw, view, block.occupied.1, block.pos.0),
                     to_screen(draw, view, block.reserved.1, block.pos.1),
                     col_res, 0.0, 0);

            }

            ImDrawList_AddRect(draw.draw_list,
                to_screen(draw, view, block.reserved.0, block.pos.0),
                to_screen(draw, view, block.reserved.1, block.pos.1),
                col_box, 0.0, 0, 1.0);

            let ra = to_screen(draw,view,block.reserved.0, block.pos.0) - draw.pos;
            let rb = to_screen(draw,view,block.reserved.1, block.pos.1) - draw.pos;
            if igIsItemHovered(0) {
                if ra.x <= draw.mouse.x && draw.mouse.x <= rb.x && ra.y <= draw.mouse.y && draw.mouse.y <= rb.y {
                    igBeginTooltip();
                    widgets::show_text(&format!("TVD section reserved t={:.1} -> t={:.1}", 
                                                block.reserved.0, block.reserved.1));
                    igEndTooltip();
                }
            }
        }
    }

    for graph in &graphics.diagram.trains {
        for s in &graph.segments {


            let (mut p1, mut p2) = (Polyline::new(), Polyline::new());
            p1.add_bezier_interpolated(
                             to_screen(draw, view, s.start_time + 0.0/3.0*s.dt, s.kms[0]),
                             to_screen(draw, view, s.start_time + 1.0/3.0*s.dt, s.kms[1]),
                             to_screen(draw, view, s.start_time + 2.0/3.0*s.dt, s.kms[2]),
                             to_screen(draw, view, s.start_time + 3.0/3.0*s.dt, s.kms[3])
                             );
            p2.add_bezier_interpolated(
                             to_screen(draw, view, s.start_time + 0.0/3.0*s.dt, s.end_kms[0]),
                             to_screen(draw, view, s.start_time + 1.0/3.0*s.dt, s.end_kms[1]),
                             to_screen(draw, view, s.start_time + 2.0/3.0*s.dt, s.end_kms[2]),
                             to_screen(draw, view, s.start_time + 3.0/3.0*s.dt, s.end_kms[3]),
                             );

            //Polyline::draw_triangulate_monotone_y(&p1,&p2,draw,col_train_rear);
            p1.draw_path(draw, col_train_rear);
            p2.draw_path(draw, col_train_rear);
        }
    }
}

struct Polyline {
    pub path :Vec<ImVec2>,
}

impl Polyline {
    pub fn draw_path(&self, draw :&Draw, col :u32) {
        unsafe {
            ImDrawList_AddPolyline(draw.draw_list, self.path.as_ptr(), self.path.len() as _, col, false, 2.0);
        }
    }
    pub fn draw_triangulate_monotone_y(p1 :&Polyline, p2 :&Polyline, draw :&Draw, col :u32) {
        if p1.path.len() <= 1 || p2.path.len() <= 1 { return; }
        let (mut i,mut j) = (0,0);
        while i+1 < p1.path.len() || j+1 < p2.path.len() {
            // advance one of the pointers
            let advance_p1 = if !(i+1 < p1.path.len()) { 
                false
            } else if !(j+1 < p2.path.len()) {
                true
            } else { p1.path[i+1].y < p2.path[j+1].y };

            if advance_p1 {
                unsafe { ImDrawList_AddTriangleFilled(draw.draw_list, p1.path[i], p1.path[i+1], p2.path[j], col); }
                i += 1;
            } else {
                unsafe { ImDrawList_AddTriangleFilled(draw.draw_list, p2.path[j], p2.path[j+1], p1.path[i], col); }
                j += 1;
            }
        }
    }
    pub fn new() -> Polyline { Polyline { path: Vec::with_capacity(8) } }
    pub fn add_bezier_interpolated(&mut self, p1 :ImVec2, y2 :ImVec2, y3 :ImVec2, p4 :ImVec2) {
        let tess_tol = 1.25;
        let p2 = (-5.0*p1 + 18.0*y2 - 9.0*y3 + 2.0*p4) / 6.0;
        let p3 = (-5.0*p4 + 18.0*y3 - 9.0*y2 + 2.0*p1) / 6.0;
        self.path.push(p1);
        self.path_bezier_to_casteljau(p1.x, p1.y, p2.x, p2.y, p3.x, p3.y, p4.x, p4.y, tess_tol, 0);
    }
    pub fn path_bezier_to_casteljau(&mut self, 
                                    x1 :f32, y1 :f32, x2 :f32, y2 :f32, 
                                    x3 :f32, y3 :f32, x4 :f32, y4 :f32, tess_tol :f32, level :i32) {
        let dx = x4 - x1;
        let dy = y4 - y1;
        let mut d2 = ((x2 - x4) * dy - (y2 - y4) * dx);
        let mut d3 = ((x3 - x4) * dy - (y3 - y4) * dx);
        d2 = if d2 >= 0.0 { d2 } else { -d2 };
        d3 = if d3 >= 0.0 { d3 } else { -d3 };
        if ((d2+d3) * (d2+d3) < tess_tol * (dx*dx + dy*dy))
        {
            self.path.push(ImVec2 { x: x4, y: y4 });
        }
        else if (level < 10)
        {
            let x12 = (x1+x2)*0.5;       let y12 = (y1+y2)*0.5;
            let x23 = (x2+x3)*0.5;       let y23 = (y2+y3)*0.5;
            let x34 = (x3+x4)*0.5;       let y34 = (y3+y4)*0.5;
            let x123 = (x12+x23)*0.5;    let y123 = (y12+y23)*0.5;
            let x234 = (x23+x34)*0.5;    let y234 = (y23+y34)*0.5;
            let x1234 = (x123+x234)*0.5; let y1234 = (y123+y234)*0.5;

            self.path_bezier_to_casteljau(x1,y1,        x12,y12,    x123,y123,  x1234,y1234, tess_tol, level+1);
            self.path_bezier_to_casteljau(x1234,y1234,  x234,y234,  x34,y34,    x4,y4,       tess_tol, level+1);
        }
    }
}

pub fn command_icons(config :&Config, 
                     inf_canvas :Option<&Draw>,
                     inf_view :&InfView,
                     analysis :&Analysis, 
                     graphics :&DispatchOutput,
                     draw :&Draw, 
                     dv :&mut ManualDispatchView) -> Option<DiagramViewAction> {

    let mut action = None;

    let border_col = config.color_u32(RailUIColorName::GraphCommandBorder);
    let il = &analysis.data().interlocking.as_ref()?.1;
    let dgraph = &analysis.data().dgraph.as_ref()?.1;
    let dispatch = &graphics.dispatch;

    let mut prev_y = -std::f32::INFINITY;
    for (cmd_idx,(cmd_id,(cmd_t,cmd))) in dispatch.commands.iter().enumerate() {
        let route_idx = match cmd { 
            Command::Route(routespec) | Command::Train(_,routespec) => {
                il.find_route(routespec) 
            },
            Command::Signal(_,_) | Command::Switch(_,_) => {
                None // Signal과 Switch는 route와 관련이 없으므로 None
            }
        };

        let fill_color = match (cmd,route_idx) {
            (Command::Route(_) | Command::Train(_,_), None) => config.color_u32(RailUIColorName::GraphCommandError),
            (Command::Route(_),_) =>                           config.color_u32(RailUIColorName::GraphCommandRoute),
            (Command::Train(_,_),_) =>                         config.color_u32(RailUIColorName::GraphCommandTrain),
            (Command::Signal(_,_),_) =>                        config.color_u32(RailUIColorName::GraphCommandRoute),
            (Command::Switch(_,_),_) =>                        config.color_u32(RailUIColorName::GraphCommandRoute),
        };

        let km = match cmd {
            Command::Route(routespec) | Command::Train(_,routespec) => {
                route_idx.and_then(|r| dgraph.mileage.get(&il.routes[*r].start_node())).cloned()
            }
            Command::Signal(object_id, _) | Command::Switch(object_id, _) => {
                let pta = dgraph.object_ids.get_by_left(object_id);
                pta.and_then(|pta| find_object_km(dgraph, pta))
                // TODO km값 이상하게 들어감
            }
        };
        let km = km.unwrap_or(0.0);

        unsafe {
            let half_icon_size = ImVec2 { x: 8.0, y: 8.0 };
            let mut p = to_screen(draw, dv.viewport.as_ref().unwrap(), *cmd_t, km);
            //p.y = p.y.max(prev_y + 2.0*half_icon_size.y);
            ImDrawList_AddRectFilled(draw.draw_list, 
                                     p - half_icon_size, 
                                     p + half_icon_size, fill_color, 0.0, 0);
            ImDrawList_AddRect(draw.draw_list, 
                               p - half_icon_size, 
                               p + half_icon_size, border_col, 0.0, 0, 1.0);

            if igIsItemHovered(0) && (p-draw.pos-draw.mouse).length_sq() < 5.0*5.0 {

                if let Some(inf) = inf_canvas {
                    if let Some(node) = route_idx.map(|r| il.routes[*r].start_node()) {
                        inf.begin_draw();
                        highlight_node(config, inf, inf_view, dgraph, node);
                        inf.end_draw();
                    }
                }

                igBeginTooltip();
                match (cmd, route_idx) {
                    (Command::Route(_) | Command::Train(_,_) ,None) => {
                        widgets::show_text(&format!("Invalid route start/end points."));
                    }
                    (Command::Route(_),_) => {
                        widgets::show_text(&format!("Route request t={:.1}", cmd_t));
                    },
                    (Command::Train(v,_),_) => {
                        let v = analysis.model().vehicles.get(*v).map(|v| v.name.as_str())
                            .unwrap_or("Unknown vehicle");
                        widgets::show_text(&format!("{} entering t={:.1}", v, cmd_t));
                    },
                    (Command::Signal(signal_id, state),_) => {
                        let state_text = if *state { "proceed" } else { "stop" };
                        widgets::show_text(&format!("Signal {} {} t={:.1}", signal_id, state_text, cmd_t));
                    },
                    (Command::Switch(switch_id, position),_) => {
                        let pos_text = if *position { "on" } else { "off" };
                        widgets::show_text(&format!("Switch {} to {} t={:.1}", switch_id, pos_text, cmd_t));
                    },
                }
                igEndTooltip();

                if igIsMouseClicked(0,false) && matches!(dv.action, ManualDispatchViewAction::None) {
                    dv.action = ManualDispatchViewAction::DragCommandTime { idx: cmd_idx, id :*cmd_id };
                }

                if igIsMouseClicked(1, false) {
                    dv.selected_command = Some(*cmd_id);
                    igOpenPopup(const_cstr!("cmded").as_ptr());
                }
            }
            prev_y = p.y;
        }
    }
    action
}

pub fn time_slider(config :&Config, draw :&Draw, viewport :&DiagramViewport, t :f64) {
	unsafe {
		let c1 = config.color_u32(RailUIColorName::GraphTimeSlider);
		let c2 = config.color_u32(RailUIColorName::GraphTimeSliderText);

		// Draw the line
		ImDrawList_AddLine(draw.draw_list,
                           to_screen(draw, viewport, t, viewport.pos.0),
                           to_screen(draw, viewport, t, viewport.pos.1),
						   c1, 2.0);

		let text = format!("t = {:.3}", t);
		ImDrawList_AddText(draw.draw_list,
                           to_screen(draw, viewport, t, viewport.pos.0),
						   c2,
						   text.as_ptr() as _ , text.as_ptr().offset(text.len() as isize) as _ );
	}
}

pub fn to_screen(draw :&Draw, v :&DiagramViewport, t: f64, x :f64) -> ImVec2 {
    ImVec2 {
        x: draw.pos.x + draw.size.x*(((x - v.pos.0)/(v.pos.1 - v.pos.0)) as f32),
        y: draw.pos.y + draw.size.y*(((t - v.time.0)/(v.time.1 - v.time.0)) as f32),
    }
}

pub fn draw_interpolate(draw_list :*mut ImDrawList, p0 :ImVec2, y1 :ImVec2, y2 :ImVec2, p3 :ImVec2, col
:u32) {
    // https://web.archive.org/web/20131225210855/http://people.sc.fsu.edu/~jburkardt/html/bezier_inter polation.html
    let p1 = (-5.0*p0 + 18.0*y1 - 9.0*y2 + 2.0*p3) / 6.0;
    let p2 = (-5.0*p3 + 18.0*y2 - 9.0*y1 + 2.0*p0) / 6.0;
    unsafe {
    ImDrawList_AddBezierCurve(draw_list, p0,p1,p2,p3, col, 2.0, 0);
    }
}

// TODO 제대로 작동 안함
// pta: &PtA (오브젝트 좌표)
fn find_object_km(dgraph: &DGraph, pta: &PtA) -> Option<f64> {
    let pta_f = glm::vec2(pta.x as f32, pta.y as f32);
    let mut min_dist = std::f32::MAX;
    let mut best = None;
    for ((a, b), points) in &dgraph.edge_lines {
        for win in points.windows(2) {
            let (p1, p2) = (win[0], win[1]);
            let v = p2 - p1;
            let w = pta_f - p1;
            let t = (w.dot(&v) / v.dot(&v)).clamp(0.0, 1.0);
            //println!("t value: {:?}", t);
            let proj = p1 + v * t;
            let dist = glm::distance(&pta_f, &proj);
            if dist < min_dist {
                min_dist = dist;
                best = Some((a, b, t));
            }
        }
    }
    if let Some((a, b, t)) = best {
        let km_a = dgraph.mileage.get(a)?;
        let km_b = dgraph.mileage.get(b)?;
        Some((*km_a as f32 + ((*km_b - *km_a) as f32) * t) as f64)
    } else {
        None
    }
}