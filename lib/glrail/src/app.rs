use crate::command_builder::*;
use crate::selection::*;
use crate::model::*;
use crate::infrastructure::*;
use crate::schematic::*;
use crate::background::*;
use crate::analysis;
use crate::scenario;
use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};


pub struct App {
    pub model :Model,
    pub command_builder: Option<CommandBuilder>,
    pub background: BackgroundUpdates,
    pub want_to_quit :bool,
    pub show_imgui_demo :bool,
}

pub enum AppAction {
    Model(ModelAction),
}

impl App {

    pub fn new() -> Self {
        App {
            model: Model::new_empty(),
            command_builder: None,
            background: BackgroundUpdates::new(),
            want_to_quit: false,
            show_imgui_demo: false,
        }
    }

    pub fn integrate(&mut self, action :AppAction) {
        match action {
            AppAction::Model(action) => {
        println!("integrate model action");
                let result = self.model.integrate(action);
                match result {
                    ModelUpdateResult::NoChange => {},
                    ModelUpdateResult::InfrastructureChanged => {
                        self.background.invalidate_inf(&mut self.model);
                    },
                    ModelUpdateResult::InterlockingChanged => {
                    },
                    ModelUpdateResult::ScenarioChanged(idx) => {
                        self.background.invalidate_scenario(idx, &mut self.model);
                    }
                }
            },
            _ => {},
        }
    }

    pub fn update_background_processes(&mut self) {
        self.background.poll_updates(&mut self.model);
    }

    pub fn save_dialog(&self) -> Result<(),()> {
       let filename = tinyfiledialogs::save_file_dialog("Save glrail document", "")
           .ok_or(())?;

       use std::fs::File;
       use std::path::Path;

       let json_path = Path::new(&filename);
       let mut json_file = File::create(json_path).map_err(|e|{
           println!("CREATE FILE ERROR {:?}", e);
           ()
       })?;

       let s = ron::ser::to_string_pretty(&self.model, Default::default())
           .map_err(|e| {
               println!("Serialize or write error: {:?}", e);
               ()
           })?;
       //write!(json_file, s);
       use std::io::Write;
       json_file.write_all(s.as_bytes()).unwrap();

       Ok(())
    }

    pub fn load_dialog(&mut self) -> Result<(),()> {
       let filename = tinyfiledialogs::open_file_dialog("Open glrail document", "", None)
           .ok_or(())?;

       use std::fs::File;
       use std::path::Path;

       let json_path = Path::new(&filename);
       let json_file = File::open(json_path).map_err(|_| ())?;
       let loaded_model : Model = ron::de::from_reader(json_file)
           .map_err(|e| {
               println!("Deserialize error: {:?}", e);
               ()
           })?;
       self.model = loaded_model;
       self.integrate(AppAction::Model(ModelAction::Inf(InfrastructureEdit::Invalidate)));
       Ok(())
    }

    pub fn context_menu(&self) -> Option<CommandScreen> {
        match self.model.view.selection {
            Selection::Entity(EntityId::Track(track_id)) => self.model.inf.get_track(&track_id).map(|_t| {
                    CommandScreen::Menu(Menu { choices: vec![
                        ('p', format!("select mid pos"), |app| {
                            if let Selection::Entity(EntityId::Track(id)) = &app.model.view.selection { 
                                if let Some(Track { start_node, end_node, .. }) = app.model.inf.get_track(id) {
                                    let Node(n1_pos,_) = app.model.inf.get_node(&start_node.0).unwrap();
                                    let Node(n2_pos,_) = app.model.inf.get_node(&end_node.0).unwrap();
                                    app.model.select_pos(0.5*(n1_pos + n2_pos), *id);
                                }
                            }
                            None
                        }),
                    ]})
                }),
            Selection::Entity(EntityId::Node(node_id)) => match self.model.inf.get_node(&node_id) {
                Some(Node(_,NodeType::BufferStop)) | Some(Node(_, NodeType::Macro(_))) => {
                    Some(CommandScreen::Menu(Menu { choices: vec![
                        ('e', format!("extend end"), |app| {
                            let mut arguments = ArgumentListBuilder::new();
                            if let Selection::Entity(id) = &app.model.view.selection {
                                arguments.add_id_value("node", *id);
                            } else {
                                arguments.add_id("node");
                            }
                            arguments.add_float_default("length", 50.0);
                            arguments.set_action(Box::new(|app :&mut App,args :&ArgumentListBuilder| {
                                let id = *args.get_id("node").unwrap();
                                let l  = *args.get_float("length").unwrap();
                                if let EntityId::Node(id) = id {
                                    app.integrate(AppAction::Model(ModelAction::Inf(
                                            InfrastructureEdit::ExtendTrack(id, l))));
                                }
                            }));
                            Some(CommandScreen::ArgumentList(arguments))
                        }),
                        ('j', format!("join with node"), |app| {
                            let mut arguments = ArgumentListBuilder::new();
                            if let Selection::Entity(id) = &app.model.view.selection {
                                arguments.add_id_value("node1",*id);
                            } else {
                                arguments.add_id("node1");
                            }
                            arguments.add_id("node2");
                            arguments.set_action(Box::new(|app :&mut App, args :&ArgumentListBuilder| {
                                let n1 = *args.get_id("node1").unwrap();
                                let n2 = *args.get_id("node2").unwrap();
                                if let EntityId::Node(n1) = n1 {
                                    if let EntityId::Node(n2) = n2 {
                                        app.integrate(AppAction::Model(ModelAction::Inf(
                                                InfrastructureEdit::JoinNodes(n1, n2))));
                                    }
                                }
                            }));
                            Some(CommandScreen::ArgumentList(arguments))
                        }),
                    ]}))
                }, 
                _ => None,
            },
            Selection::Pos(pos,y,id) => {
                Some(CommandScreen::Menu(Menu { choices: vec![
                    ('k', format!("out left sw"), |app| { 
                        if let Selection::Pos(pos,_,track_id) = &app.model.view.selection {
                        app.integrate(AppAction::Model(ModelAction::Inf(
                            InfrastructureEdit::InsertNode(
                            *track_id, *pos, NodeType::Switch(Dir::Up, Side::Left), 50.0))));
                        }
                        None }),
                    ('K', format!("in right sw"), |app| { 
                        if let Selection::Pos(pos,_,track_id) = &app.model.view.selection {
                        app.integrate(AppAction::Model(ModelAction::Inf(
                            InfrastructureEdit::InsertNode(
                            *track_id, *pos, NodeType::Switch(Dir::Down, Side::Right), 50.0))));
                        }
                        None }),
                    ('j', format!("out right sw"), |app| { 
                        if let Selection::Pos(pos,_,track_id) = &app.model.view.selection {
                        app.integrate(AppAction::Model(ModelAction::Inf(
                            InfrastructureEdit::InsertNode(
                            *track_id, *pos, NodeType::Switch(Dir::Up, Side::Right), 50.0))));
                        }
                        None }),
                    ('J', format!("in left sw"), |app| { 
                        if let Selection::Pos(pos,_,track_id) = &app.model.view.selection {
                        app.integrate(AppAction::Model(ModelAction::Inf(
                            InfrastructureEdit::InsertNode(
                            *track_id, *pos, NodeType::Switch(Dir::Down, Side::Left), 50.0))));
                        }
                        None }),
                    ('w', format!("Home Signal"), |app| {
                        if let Selection::Pos(pos, _ , track_id) = &app.model.view.selection {
                            app.integrate(AppAction::Model(ModelAction::Inf(
                                    InfrastructureEdit::InsertObject(
                                        *track_id, *pos, ObjectType::Signal(Dir::Up)))));
                        }
                        None
                    }),
                    ('c', format!("Departure Signal"), |app| {
                        if let Selection::Pos(pos, _ , track_id) = &app.model.view.selection {
                            app.integrate(AppAction::Model(ModelAction::Inf(
                                    InfrastructureEdit::InsertObject(
                                        *track_id, *pos, ObjectType::Signal(Dir::Down)))));
                        }
                        None
                    }),
                    ('d', format!("Shunting Signal"), |app| {
                        if let Selection::Pos(pos, _ , track_id) = &app.model.view.selection {
                            app.integrate(AppAction::Model(ModelAction::Inf(
                                    InfrastructureEdit::InsertObject(
                                        *track_id, *pos, ObjectType::ShuntingSignal))));
                        }
                        None
                    }),
                    ('r', format!("Section Insulator"), |app| {
                        if let Selection::Pos(pos, _ , track_id) = &app.model.view.selection {
                            app.integrate(AppAction::Model(ModelAction::Inf(
                                    InfrastructureEdit::InsertObject(
                                        *track_id, *pos, ObjectType::Detector))));
                        }
                        None
                    }),
                    ('t', format!("Switch"), |app| {
                        if let Selection::Pos(pos, _ , track_id) = &app.model.view.selection {
                            app.integrate(AppAction::Model(ModelAction::Inf(
                                    InfrastructureEdit::InsertObject(
                                        *track_id, *pos, ObjectType::Switch))));
                        }
                        None
                    }),
                ]}))
            }
            _ => None,
        }
    }

    pub fn main_menu(&mut self) {
        fn close(app :&mut App) -> Option<CommandScreen> { None }

       let main_menu = Menu {
           choices: vec![
               // TODO all of this stuff
               ('i', format!("maximaldesign"), |app| {
                   analysis::synthesis::add_maximal(&mut app.model.inf);
                   app.integrate(AppAction::Model(ModelAction::Inf(InfrastructureEdit::Invalidate)));
                   None
               }),
               ('y', format!("synthesize"), |app| {
                   let usages = app.model.scenarios.iter().filter_map(|s| {
                       match s {
                           scenario::Scenario::Usage(u,_) => Some(u.clone()),
                           _ => None,
                       }
                   }).collect::<Vec<_>>();
                   let objs = analysis::synthesis::synthesis(&app.model.inf, &usages, &app.model.vehicles, |t,o| { 
                       println!("Received score {}", t); true }).unwrap();
                   for o in objs {
                       println!("Adding {:?}", o);
                       app.model.inf.new_object(o);
                   }
                   app.integrate(AppAction::Model(ModelAction::Inf(InfrastructureEdit::Invalidate)));
                   None
               }),
               ('c', format!("context"), |app| { app.context_menu() }),
               ('l', format!("load"),    |app| { app.load_dialog().ok(); None }),
               ('s', format!("save"),    |app| { app.save_dialog().ok(); None }),
               ('q', format!("quit"),    |app| { app.want_to_quit = true; None } ),
               ('s', format!("selection"), |_| { 
                   Some(CommandScreen::Menu(Menu { choices: vec![
                       ('z', format!("none"),      |app| { app.model.view.selection = Selection::None; None }),
                       ('o', format!("object"),    |app| { 
                           //if let Some(id) = app.model.inf.any_object() {
                           //    app.model.view.selection = Selection::Entity(id);
                           //}
                           None 
                       }),
                       ('p', format!("pos"),       |app| { 
                           app.model.view.selection = Selection::None; 
                           None 
                       }),
                       ('r', format!("pos range"), |app| { app.model.view.selection = Selection::None; None }),
                       ('l', format!("path"),      |app| { app.model.view.selection = Selection::None; None }),
                       ('a', format!("area"),      |app| { app.model.view.selection = Selection::None; None }),
                   ]}))
               }),

               ('v', format!("view"), |_| { 
                   Some(CommandScreen::Menu(Menu { choices: vec![
                       ('a', format!("all"),       |app| { None }),
                       ('s', format!("selection"), |app| { None }),
                   ]}))
               }),

               ('o', format!("options"), |_| {
                   Some(CommandScreen::Menu(Menu { choices: vec![
                       ('d', format!("imgui debug window"), |app| { 
                           app.show_imgui_demo = !app.show_imgui_demo; 
                           None })
                   ]}))
               }),
           ]
       };
        self.command_builder = Some(CommandBuilder::new_menu(main_menu));
        if self.model.inf.num_entities() == 0 {
            if let CommandScreen::Menu(Menu { choices }) = self.command_builder.as_mut().unwrap().current_screen() {
                choices.push(('a', format!("add track"), |app| {
                    app.integrate(AppAction::Model(ModelAction::Inf(InfrastructureEdit::NewTrack(0.0,100.0))));
                    None
                }));
            }
        }
    }

    pub fn clicked_object(&mut self, id :Option<EntityId>, location: (f32,f32)) {
        println!("Clicked {:?} {:?}", id, location);
        if let Some(id) = id {
            if let Some(cb) = &mut self.command_builder {
                if let CommandScreen::ArgumentList(ref mut alb) = cb.current_screen() {
                    for (n,s,a) in &mut alb.arguments {
                        if let Arg::Id(ref mut optid) = a {
                            if let ArgStatus::NotDone = s {
                                *optid = Some(id);
                                break;
                            }
                        }
                    }
                }
            } else {
                // todo check if we are in pos selection mode.

                // TODO move into model? view?
                if let EntityId::Track(track_id) = id {
                    if let Derive::Ok(ref s) = self.model.schematic {
                        if let Some(pos) = s.x_to_pos(location.0) {
                            if let Some((pt,t)) = s.track_line_at(&track_id,pos) {
                                self.model.view.selection = Selection::Pos(pos, pt.1, track_id);
                            }
                        }
                    }
                } else { 
                    self.model.view.selection = Selection::Entity(id);
                }
            }
        }
    }

}


