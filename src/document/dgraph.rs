use bimap::BiMap;
use rolling::input::staticinfrastructure as rolling_inf;
use std::collections::{HashMap, HashSet};
use ordered_float::OrderedFloat;
use crate::document::model::*;
use crate::document::objects::*;
use crate::document::topology::*;
use crate::document::mileage;
use matches::matches;
use nalgebra_glm as glm;

pub type ModelNodeId = Pt;
pub type ModelObjectId = PtA;

pub mod allpaths;

#[derive(Debug)]
pub struct DGraph {
    pub rolling_inf :rolling_inf::StaticInfrastructure, 
    pub node_ids :BiMap<rolling_inf::NodeId, Pt>,
    pub switch_ids :BiMap<rolling_inf::ObjectId, Pt>,
    pub object_ids :BiMap<rolling_inf::ObjectId, PtA>,
    pub detector_ids :BiMap<rolling_inf::NodeId, PtA>,
    pub tvd_edges :HashMap<rolling_inf::ObjectId, Vec<(rolling_inf::NodeId, rolling_inf::NodeId)>>,
    pub tvd_entry_nodes :HashMap<rolling_inf::ObjectId, Vec<rolling_inf::NodeId>>,
    pub edge_lines :HashMap<(rolling_inf::NodeId, rolling_inf::NodeId), Vec<PtC>>,
    pub mileage :HashMap<rolling_inf::NodeId, f64>,
    pub all_paths :(f64, Vec<allpaths::Path>),
}



impl DGraph {
    pub fn mileage_at(&self, a :rolling_inf::NodeId, b :rolling_inf::NodeId, param :f64) -> Option<f64> {
        let km_a = *self.mileage.get(&a)?;
        let km_b = *self.mileage.get(&b)?;
        Some(glm::lerp_scalar(km_a,km_b,param))
    }
}

pub fn edge_length(rolling_inf :&rolling_inf::StaticInfrastructure, a :rolling_inf::NodeId, b: rolling_inf::NodeId) -> Option<f64> {
    match rolling_inf.nodes[a].edges {
        rolling_inf::Edges::Single(bx,d) if b == bx => Some(d),
        rolling_inf::Edges::Switchable(objid) => {
            if let rolling_inf::StaticObject::Switch { left_link, right_link, .. } = rolling_inf.objects[objid] {
                if left_link.0 == b { Some(left_link.1) }
                else if right_link.0 == b { Some(right_link.1) }
                else { None }
            } else { None }
        }
        _ => None,
    }
}

pub struct DGraphBuilder {
    dgraph :rolling_inf::StaticInfrastructure,
    edge_tracks :HashMap<(rolling_inf::NodeId, rolling_inf::NodeId), Interval>,
}

#[derive(Debug)]
pub struct Interval {
    track_idx: usize,
    start: f64,
    end :f64,
}

impl DGraphBuilder {
    pub fn convert(topology :&Topology) -> Result<DGraph, ()> {
        let mut m = DGraphBuilder::new();

        let tracks = &topology.tracks;
        let locs = &topology.locations;
        let trackobjects = &topology.trackobjects;

        // Create signals objects separately (they are not actually part of the "geographical" 
        // infrastructure network, they are merely pieces of state referenced by sight objects)
        let mut static_signals :HashMap<PtA, rolling_inf::ObjectId> = HashMap::new();
        let mut signal_cursors : HashMap<PtA, Cursor> = HashMap::new();
        let mut detector_nodes : HashSet<(rolling_inf::NodeId, rolling_inf::NodeId)> = HashSet::new();
        let mut object_ids = BiMap::new();
        let mut detector_ids = BiMap::new();
        let (node_ids, switch_ids, crossing_edges) = m.create_network(
            tracks, &locs, 
            |track_idx,mut cursor,dg| {
                let mut last_pos = 0.0;
                let mut objs :Vec<(f64,PtA,Function,Option<AB>)> = trackobjects[track_idx].clone();
                objs.sort_by_key(|(pos,_,_,_)| OrderedFloat(*pos));
                for (pos, id, func, dir) in objs {

                    // TODO stack overflow here
                    cursor = cursor.advance_single(&dg.dgraph, pos - last_pos).unwrap();

                    cursor = dg.insert_node_pair(cursor);

                    match func {
                        Function::Detector => { 
                            let (a,b) = cursor.nodes(&dg.dgraph);
                            detector_nodes.insert((a,b));
                            detector_ids.insert(a,id);
                            detector_ids.insert(b,id);
                        },
                        Function::MainSignal { has_distant, id: _ }=> { 
                            let c = if matches!(dir,Some(AB::B)) { cursor.reverse(&dg.dgraph) } else { cursor };
                            signal_cursors.insert(id,c); 

                            let (_cursor, obj) = dg.insert_object(c, 
                                  rolling_inf::StaticObject::Signal { has_distant: has_distant });
                            static_signals.insert(id, obj);
                            object_ids.insert(obj, id);
                        },
                        Function::ShiftingSignal { has_distant, id: _ }=> { 
                            let c = if matches!(dir,Some(AB::B)) { cursor.reverse(&dg.dgraph) } else { cursor };
                            signal_cursors.insert(id,c); 

                            let (_cursor, obj) = dg.insert_object(c, 
                                  rolling_inf::StaticObject::Signal { has_distant: has_distant });
                            static_signals.insert(id, obj);
                            object_ids.insert(obj, id);
                        },
                        Function::Switch => { 
                            // Switch는 현재 Detector와 동일하게 처리
                            let (a,b) = cursor.nodes(&dg.dgraph);
                            detector_nodes.insert((a,b));
                            detector_ids.insert(a,id);
                            detector_ids.insert(b,id);
                        },
                    }
                    last_pos = pos;
                }
            } );

        // Sight to signals
        for (id,cursor) in signal_cursors {
            let objid = static_signals[&id];
            let sight_dist = 200.0; // TODO configurable
            for (cursor,dist) in cursor.reverse(&m.dgraph).advance_nontrailing_truncate(&m.dgraph, sight_dist) {
                let cursor = cursor.reverse(&m.dgraph);
                m.insert_object(cursor, rolling_inf::StaticObject::Sight{
                    distance: dist, signal: objid,
                });
            }
        }

        // Train detectors
        for (node_idx,node) in m.dgraph.nodes.iter().enumerate() {
            if matches!(node.edges, rolling_inf::Edges::ModelBoundary) {
                detector_nodes.insert((node_idx, node.other_node));
            }
        }
        let (tvd_edges,tvd_entry_nodes) = route_finder::detectors_to_sections(&mut m.dgraph, 
                                                                              &detector_nodes,
                                                                              &crossing_edges)
            .expect("could not calc tvd sections.");

        let mut edge_lines :HashMap<(rolling_inf::NodeId, rolling_inf::NodeId), Vec<PtC>>
            = m.edge_tracks.into_iter()
            .map(|(edge,Interval { track_idx, start, end })| 
                 (edge, topology.interval_map(track_idx,start,end))).collect();

        let rev_edge_lines = edge_lines.iter().map(|((a,b),v)| ((*b,*a),{ let mut v= v.clone(); v.reverse(); v })).collect::<Vec<_>>();
        edge_lines.extend(rev_edge_lines.into_iter());

        let mileage = mileage::auto(&node_ids, &m.dgraph);
        //println!("MILEAGES {:?}", mileage);
        //mileage::test_lsq();
        //let mileage = std::iter::empty().collect();

        let all_paths_length = 100.0;
        let all_paths = (all_paths_length, allpaths::paths(&m.dgraph, all_paths_length));
        Ok(DGraph {
            rolling_inf: m.dgraph,
            node_ids: node_ids,
            switch_ids: switch_ids,
            object_ids: object_ids,
            detector_ids: detector_ids,
            tvd_edges: tvd_edges,
            tvd_entry_nodes: tvd_entry_nodes,
            edge_lines: edge_lines,
            mileage: mileage,
            all_paths: all_paths,
        })

    }

    pub fn new() -> DGraphBuilder {
        let model = rolling_inf::StaticInfrastructure {
            nodes: Vec::new(), 
            objects: Vec::new(),
        };
        DGraphBuilder { dgraph: model, edge_tracks: HashMap::new() }
    }

    pub fn new_object(&mut self, obj :rolling_inf::StaticObject) -> rolling_inf::ObjectId {
        let id  = self.dgraph.objects.len();
        self.dgraph.objects.push(obj);
        id
    }

    pub fn new_object_at(&mut self, obj :rolling_inf::StaticObject, node: rolling_inf::NodeId) -> rolling_inf::ObjectId {
        let obj_id = self.new_object(obj);
        self.dgraph.nodes[node].objects.push(obj_id);
        obj_id
    }

    pub fn new_node_pair(&mut self) -> (rolling_inf::NodeId, rolling_inf::NodeId) {
        let (i1,i2) = (self.dgraph.nodes.len(), self.dgraph.nodes.len() +1);
        self.dgraph.nodes.push(rolling_inf::Node { other_node: i2,
            edges: rolling_inf::Edges::Nothing, objects: Default::default() });
        self.dgraph.nodes.push(rolling_inf::Node { other_node: i1,
            edges: rolling_inf::Edges::Nothing, objects: Default::default() });
        (i1,i2)
    }

    fn connect_linear(&mut self, na :rolling_inf::NodeId, nb :rolling_inf::NodeId, d :f64) {
        self.dgraph.nodes[na].edges = rolling_inf::Edges::Single(nb, d);
        self.dgraph.nodes[nb].edges = rolling_inf::Edges::Single(na, d);
    }

    fn split_edge(&mut self, a :rolling_inf::NodeId, b :rolling_inf::NodeId, second_dist :f64) -> (rolling_inf::NodeId, rolling_inf::NodeId) {
        let (na,nb) = self.new_node_pair();
        let reverse_dist = edge_length(&self.dgraph, b, a).unwrap();
        let forward_dist = edge_length(&self.dgraph, a, b).unwrap();
        let first_dist = reverse_dist - second_dist;
        self.replace_conn(a,b,na,first_dist);
        self.replace_conn(b,a,nb,second_dist);

        // TODO this seems overcomplicated
        for ((a1,a2),(b1,b2),split_dist) in vec![((a,na),(b,nb),first_dist),((b,nb),(a,na),second_dist)].into_iter() {
            if let Some(Interval { track_idx, start, end }) = self.edge_tracks.remove(&(a1,b1)) {
                self.edge_tracks.insert((a1,a2), Interval { track_idx, 
                    start: start, end: start+split_dist });
                self.edge_tracks.insert((b2,b1), Interval { track_idx, 
                    start: start+split_dist, end: end });
            }
        }
        (na,nb)
    }


    fn replace_conn(&mut self, a :rolling_inf::NodeId, b :rolling_inf::NodeId, x :rolling_inf::NodeId, d :f64) {
        use rolling_inf::Edges;
        match self.dgraph.nodes[a].edges {
            Edges::Single(bx,_dist) if b == bx => { self.dgraph.nodes[a].edges = Edges::Single(x,d); }
            Edges::Switchable(objid) => {
                if let rolling_inf::StaticObject::Switch { ref mut left_link, ref mut right_link, .. } = &mut self.dgraph.objects[objid] {
                    if left_link.0 == b { *left_link = (x,d); }
                    else if right_link.0 == b { *right_link = (x,d); }
                    else { panic!() }
                } else { panic!() }
            }
            _ => { panic!() },
        };
        self.dgraph.nodes[x].edges = Edges::Single(a,d);
    }

    pub fn insert_node_pair(&mut self, at :Cursor) -> Cursor {
        match at {
            Cursor::Node(x) => Cursor::Node(x),
            Cursor::Edge((a,b),d) => {
                let (na,nb) = self.split_edge(a,b,d);
                Cursor::Node(nb)
            },
        }
    }

    pub fn insert_object(&mut self, at :Cursor, obj :rolling_inf::StaticObject) -> (Cursor,rolling_inf::ObjectId) {
        if let Cursor::Node(a) = at {
            let objid = self.new_object_at(obj, a);
            (at,objid)
        } else {
            let at = self.insert_node_pair(at);
            self.insert_object(at, obj)
        }
    }

    pub fn create_network(&mut self,
        tracks: &[(f64, (Pt, Port), (Pt, Port))], // track length and line pieces
        nodes: &HashMap<Pt,(NDType, Vc)>,
        mut each_track: impl FnMut(usize,Cursor,&mut Self)) -> 
        (BiMap<rolling_inf::NodeId, Pt>,
         BiMap<rolling_inf::ObjectId, Pt>,
         HashSet<(rolling_inf::NodeId, rolling_inf::NodeId)>) {

        let mut node_ids = BiMap::new();
        let mut switch_ids = BiMap::new();
        let mut crossing_edges = HashSet::new();
        let mut ports :HashMap<(Pt,Port), rolling_inf::NodeId>  = HashMap::new();
        for (i,(len,a,b)) in tracks.iter().enumerate() {
            let (start_a,start_b) = self.new_node_pair();
            let (end_a,end_b) = self.new_node_pair();
            ports.insert(*a, start_a);
            self.connect_linear(start_b, end_a, *len);
            ports.insert(*b, end_b);
            self.edge_tracks.insert((start_b,end_a), Interval { track_idx: i, 
                start: 0.0, end: *len });
            each_track(i,Cursor::Node(start_b), self);
        }

        for (pt,(node,_)) in nodes.iter() {
            match node {
                NDType::BufferStop => {},
                NDType::OpenEnd => {
                    self.dgraph.nodes[ports[&(*pt, Port::End)]].edges =
                        rolling_inf::Edges::ModelBoundary;
                    node_ids.insert(ports[&(*pt,Port::End)], *pt);
                },
                NDType::Cont => {
                    self.connect_linear(ports[&(*pt, Port::ContA)],
                                        ports[&(*pt, Port::ContB)], 0.0);
                },
                NDType::Sw(side) => {
                    let sw_obj = self.new_object(rolling_inf::StaticObject::Switch {
                        left_link:  (ports[&(*pt,Port::Left)], 0.0),
                        right_link: (ports[&(*pt,Port::Right)], 0.0),
                        branch_side: side.as_switch_position(),
                    });

                    switch_ids.insert(sw_obj, *pt);

                    self.dgraph.nodes[ports[&(*pt, Port::Left)]].edges  = 
                        rolling_inf::Edges::Single(ports[&(*pt,Port::Trunk)], 0.0);
                    self.dgraph.nodes[ports[&(*pt, Port::Right)]].edges = 
                        rolling_inf::Edges::Single(ports[&(*pt,Port::Trunk)], 0.0);
                    self.dgraph.nodes[ports[&(*pt, Port::Trunk)]].edges =
                        rolling_inf::Edges::Switchable(sw_obj);
                },
                NDType::Crossing(type_) => {
                    let left_drivable  = matches!(type_, CrossingType::DoubleSlip | CrossingType::SingleSlip(Side::Left));
                    let right_drivable = matches!(type_, CrossingType::DoubleSlip | CrossingType::SingleSlip(Side::Right));

                    for (dir,drivable) in &[(AB::A, left_drivable), (AB::B, right_drivable)] {
                        if *drivable {
                            let sw_a = self.new_object(rolling_inf::StaticObject::Switch { 
                                left_link:  (ports[&(*pt, Port::Cross(dir.other(), 1))], 0.0),
                                right_link: (ports[&(*pt, Port::Cross(dir.other(), 0))], 0.0),
                                branch_side: Side::Left.as_switch_position(),
                            });
                            let sw_b = self.new_object(rolling_inf::StaticObject::Switch { 
                                left_link:  (ports[&(*pt, Port::Cross(*dir, 1))], 0.0),
                                right_link: (ports[&(*pt, Port::Cross(*dir, 0))], 0.0),
                                branch_side: Side::Right.as_switch_position(),
                            });

                            self.dgraph.nodes[ports[&(*pt, Port::Cross(*dir, 0))]].edges = rolling_inf::Edges::Switchable(sw_a);
                            self.dgraph.nodes[ports[&(*pt, Port::Cross(dir.other(), 1))]].edges = rolling_inf::Edges::Switchable(sw_b);
                        } else {
                            self.dgraph.nodes[ports[&(*pt, Port::Cross(*dir, 0))]].edges = 
                                rolling_inf::Edges::Single(ports[&(*pt, Port::Cross(dir.other(), 0))], 0.0);
                            self.dgraph.nodes[ports[&(*pt, Port::Cross(dir.other(), 1))]].edges = 
                                rolling_inf::Edges::Single(ports[&(*pt, Port::Cross(*dir, 1))], 0.0);
                        }
                    }

                    if !left_drivable && !right_drivable {
                        crossing_edges.insert((ports[&(*pt, Port::Cross(AB::A, 0))], ports[&(*pt, Port::Cross(AB::A, 1))]));
                    }

                },
                NDType::Err => {},
            }
        }
        (node_ids, switch_ids, crossing_edges)
    }
}

#[derive(Copy,Clone, Debug)]
pub enum Cursor {
    Node(rolling_inf::NodeId),
    Edge((rolling_inf::NodeId, rolling_inf::NodeId), f64), // remaining distance along edge
}

fn edge_multiplicity(e :&rolling_inf::Edges) -> usize {
    match e {
        rolling_inf::Edges::Switchable(_) => 2,
        rolling_inf::Edges::ModelBoundary |
        rolling_inf::Edges::Nothing => 0,
        rolling_inf::Edges::Single(_,_) => 1,
    }
}

fn out_edges(dg :&rolling_inf::StaticInfrastructure, e: &rolling_inf::NodeId) -> Vec<(rolling_inf::NodeId,f64)> {
    match dg.nodes[*e].edges {
        rolling_inf::Edges::Single(n,d) => vec![(n,d)],
        rolling_inf::Edges::Switchable(obj) => match dg.objects[obj] {
            rolling_inf::StaticObject::Switch { right_link, left_link, .. } => vec![right_link,left_link],
            _ => panic!(),
        },
        rolling_inf::Edges::ModelBoundary | rolling_inf::Edges::Nothing => vec![],
    }
}

impl Cursor {
    pub fn advance_single(&self, dg :&rolling_inf::StaticInfrastructure, l :f64) -> Option<Cursor> {
        if l <= 0.0 { return Some(*self); }
        match self {
            Cursor::Node(n) => {
                match dg.nodes[*n].edges {
                    rolling_inf::Edges::Single(b,d) => Cursor::Edge((*n,b),d).advance_single(dg, l),
                    _ => None,
                }
            }
            Cursor::Edge((a,b),d) => if *d > l {
                Some(Cursor::Edge((*a,*b), *d - l))
            } else {
                Cursor::Node(*b).advance_single(dg, l - *d)
            },
        }
    }

    pub fn advance_nontrailing_truncate(&self, dg :&rolling_inf::StaticInfrastructure, l :f64) -> Vec<(Cursor,f64)> {
        let mut output = Vec::new();
        let mut cursors = vec![(*self,l)];
        while let Some((cursor,d)) = cursors.pop() {
            match cursor {
                Cursor::Edge((a0,b0),nd0) => {
                    if nd0 >= d { 
                        output.push((Cursor::Edge((a0,b0),nd0-d), l)); // Done: Full length achieved
                    } else {
                        if edge_multiplicity(&dg.nodes[b0].edges) > 1 {
                            // Done: Trailing switch, truncate path here
                            output.push((Cursor::Edge((a0,b0), 0.0), l - (d - nd0)));
                        } else {
                            cursors.push((Cursor::Node(dg.nodes[b0].other_node), d - nd0));
                        }
                    }
                },
                Cursor::Node(a) => {
                    if edge_multiplicity(&dg.nodes[a].edges) > 0 {
                        for (b,nd) in out_edges(dg, &a) {
                            cursors.push((Cursor::Edge((a,b),nd), d));
                        }
                    } else {
                        output.push((Cursor::Node(a), l-d));
                    }
                },
            };
        }
        output
    }

    pub fn nodes(&self, dg :&rolling_inf::StaticInfrastructure) -> (rolling_inf::NodeId, rolling_inf::NodeId) {
        match self {
            Cursor::Node(n) => (*n, dg.nodes[*n].other_node),
            Cursor::Edge((a,b),_d) => (*a,*b),
        }
    }

    pub fn reverse(&self, dg :&rolling_inf::StaticInfrastructure) -> Cursor {
        match self {
            Cursor::Node(n) => Cursor::Node(dg.nodes[*n].other_node),
            Cursor::Edge((a,b),l) => Cursor::Edge((*b,*a), edge_length(dg, *a, *b).unwrap() - l),
        }
    }

}
