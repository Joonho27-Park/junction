use std::collections::{HashMap, HashSet};
use log::*;
use const_cstr::const_cstr;
use crate::document::model::*;
use crate::document::objects::*;
use crate::app::*;
use crate::gui::widgets;
use std::sync::mpsc;
use std::fs;

use std::fmt;

pub enum ExportError {
    FileError(String),
    ConversionError(String),
}

impl fmt::Display for ExportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExportError::FileError(msg) => write!(f, "File error: {}", msg),
            ExportError::ConversionError(msg) => write!(f, "Conversion error: {}", msg),
        }
    }
}

pub struct ExportWindow {
    pub open: bool,
    state: ExportState,
    thread: Option<mpsc::Receiver<ExportState>>,
    thread_pool: BackgroundJobs,
}

impl ExportWindow {
    pub fn new(thread_pool: BackgroundJobs) -> Self {
        ExportWindow {
            open: false,
            state: ExportState::ChooseFile,
            thread: None,
            thread_pool: thread_pool,
        }
    }
}

#[derive(Debug)]
pub enum ExportState {
    Ping,                    // 초기 상태
    ChooseFile,              // 파일 선택 대기
    Converting,              // 변환 중
    ConversionError(String), // 변환 오류
    Success(String),         // 성공 (파일명)
}

impl ExportWindow {
    pub fn open(&mut self) { self.open = true; }

    pub fn update(&mut self) {
        while let Some(Ok(msg)) = self.thread.as_mut().map(|rx| rx.try_recv()) {
            println!("export window new state: {:?}", msg);
            self.state = msg;
        }
    }

    pub fn draw(&mut self, model: &Model) {
        if !self.open { return; }
        use backend_glfw::imgui::*;
        unsafe {
            widgets::next_window_center_when_appearing();
            igBegin(const_cstr!("Export to railML file").as_ptr(), &mut self.open as _, 0 as _);

            match &self.state {
                ExportState::ChooseFile => {
                    if igButton(const_cstr!("Browse for file...").as_ptr(),
                                ImVec2 { x: 120.0, y: 0.0 }) {
                        if let Some(filename) = tinyfiledialogs::save_file_dialog("Save railML file", "") {
                            // 파일명에 .railml 확장자가 없으면 추가
                            let filename = if filename.ends_with(".railml") {
                                filename
                            } else {
                                format!("{}.railml", filename)
                            };
                            self.background_export_file(filename, model.clone());
                        }
                    }
                },
                ExportState::Converting => {
                    widgets::show_text("Converting model to railML...");
                },
                ExportState::Success(filename) => {
                    widgets::show_text(&format!("Successfully exported to: {}", filename));
                    if igButton(const_cstr!("Close").as_ptr(), ImVec2 { x: 80.0, y: 0.0 }) {
                        self.close();
                    }
                },
                ExportState::ConversionError(error) => {
                    widgets::show_text(&format!("Error: {}", error));
                    if igButton(const_cstr!("Close").as_ptr(), ImVec2 { x: 80.0, y: 0.0 }) {
                        self.close();
                    }
                },
                _ => { widgets::show_text(&format!("{:?}", self.state)); },
            }

            igEnd();
        }
    }

    pub fn background_export_file(&mut self, filename: String, model: Model) {
        info!("Starting background export to railML file {:?}", filename);
        let (tx, rx) = mpsc::channel();
        self.thread = Some(rx);
        self.thread_pool.execute(move || { export_railml_file(filename, model, tx); });
    }

    pub fn close(&mut self) {
        self.open = false;
        self.state = ExportState::ChooseFile;
        self.thread = None;
    }
}

pub fn export_railml_file(filename: String, model: Model, tx: mpsc::Sender<ExportState>) {
    if tx.send(ExportState::Converting).is_err() { return; }
    
    // Convert model to railML
    match convert_model_to_railml(&model) {
        Ok(railml_content) => {
            match fs::write(&filename, railml_content) {
                Ok(_) => {
                    let _ = tx.send(ExportState::Success(filename));
                },
                Err(e) => {
                    let _ = tx.send(ExportState::ConversionError(format!("File write error: {}", e)));
                },
            }
        },
        Err(e) => {
            let _ = tx.send(ExportState::ConversionError(format!("Conversion error: {}", e)));
        },
    }
}

pub fn convert_model_to_railml(model: &Model) -> Result<String, ExportError> {
    // Analyze the model to identify tracks and switches
    let mut tracks = Vec::new();
    
    // Find switches in the model
    let mut switches = Vec::new();
    for (pos, obj) in &model.objects {
        for function in &obj.functions {
            if let Function::Switch { .. } = function {
                switches.push((*pos, obj.clone()));
            }
        }
    }
    
    // Create tracks based on model structure
    if !switches.is_empty() {
        // Create main track with switches
        let main_track = create_main_track_with_switches(&switches, model);
        tracks.push(main_track);
        
        // Create secondary track that connects to switches
        let secondary_track = create_secondary_track(&switches);
        tracks.push(secondary_track);
    } else {
        // Create simple track structure
        let simple_track = create_simple_track_structure(model);
        tracks.push(simple_track);
    }
    
    // Generate railML XML
    let railml_content = generate_railml_xml(&tracks);
    Ok(railml_content)
}

fn create_main_track_with_switches(switches: &[(Pt, Object)], model: &Model) -> String {
    let mut track_xml = String::new();
    track_xml.push_str("        <track name=\"SP1\" id=\"track1\">\n");
    track_xml.push_str("            <trackTopology>\n");
    
    // Calculate track length from line segments
    let track_length = calculate_track_length(&model.linesegs);
    
    // Track begin
    track_xml.push_str("                <trackBegin pos=\"0.0\" id=\"tb1\">\n");
    track_xml.push_str("                    <openEnd id=\"b1\" />\n");
    track_xml.push_str("                </trackBegin>\n");
    
    // Track end
    track_xml.push_str(&format!("                <trackEnd pos=\"{}\" id=\"te1\">\n", track_length));
    track_xml.push_str("                    <openEnd id=\"b2\" />\n");
    track_xml.push_str("                </trackEnd>\n");
    
    // Connections (switches) - dynamically position based on model
    track_xml.push_str("                <connections>\n");
    
    for (i, (pos, obj)) in switches.iter().enumerate() {
        let switch_id = format!("sw{}", i + 1);
        // Calculate position based on actual model coordinates
        let switch_pos = calculate_position_on_track(pos, &model.linesegs, track_length);
        
        if i == 0 {
            track_xml.push_str(&format!("                    <switch id=\"{}\" pos=\"{}\">\n", switch_id, switch_pos));
            track_xml.push_str("                        <connection id=\"sw1c\" ref=\"tb2c\" course=\"left\" orientation=\"outgoing\" />\n");
            track_xml.push_str("                    </switch>\n");
        } else if i == 1 {
            track_xml.push_str(&format!("                    <switch id=\"{}\" pos=\"{}\">\n", switch_id, switch_pos));
            track_xml.push_str("                        <connection id=\"sw2c\" ref=\"te2c\" course=\"right\" orientation=\"incoming\" />\n");
            track_xml.push_str("                    </switch>\n");
        }
    }
    
    track_xml.push_str("                </connections>\n");
    track_xml.push_str("            </trackTopology>\n");
    
    // Add OCS elements dynamically
    track_xml.push_str("            <ocsElements>\n");
    
    // Add signals dynamically
    track_xml.push_str("                <signals>\n");
    let mut signal_id = 1;
    for (pos, obj) in &model.objects {
        for function in &obj.functions {
            if let Function::Signal { .. } = function {
                let signal_pos = calculate_position_on_track(pos, &model.linesegs, track_length);
                track_xml.push_str(&format!("                    <signal id=\"sig{}\" name=\"Signal {}\" pos=\"{}\" type=\"main\" dir=\"up\"/>\n", 
                                          signal_id, signal_id, signal_pos));
                signal_id += 1;
            }
        }
    }
    track_xml.push_str("                </signals>\n");
    
    // Add train detectors dynamically
    track_xml.push_str("                <trainDetectionElements>\n");
    let mut detector_id = 0;
    for (pos, obj) in &model.objects {
        for function in &obj.functions {
            if let Function::Detector = function {
                let detector_pos = calculate_position_on_track(pos, &model.linesegs, track_length);
                track_xml.push_str(&format!("                    <trainDetector id=\"d{}\" name=\"detector {}\" pos=\"{}\" />\n", 
                                          detector_id, detector_id + 1, detector_pos));
                detector_id += 1;
            }
        }
    }
    track_xml.push_str("                </trainDetectionElements>\n");
    
    track_xml.push_str("            </ocsElements>\n");
    track_xml.push_str("        </track>\n");
    
    track_xml
}

fn create_secondary_track(switches: &[(Pt, Object)]) -> String {
    let mut track_xml = String::new();
    track_xml.push_str("        <track name=\"SP2\" id=\"track2\">\n");
    track_xml.push_str("            <trackTopology>\n");
    
    // Track begin with connection to first switch
    track_xml.push_str("                <trackBegin pos=\"0.0\" id=\"tb2\">\n");
    track_xml.push_str("                    <connection id=\"tb2c\" ref=\"sw1c\" />\n");
    track_xml.push_str("                </trackBegin>\n");
    
    // Track end with connection to second switch
    track_xml.push_str("                <trackEnd pos=\"500.0\" id=\"te2\">\n");
    track_xml.push_str("                    <connection id=\"te2c\" ref=\"sw2c\" />\n");
    track_xml.push_str("                </trackEnd>\n");
    
    // No additional connections for secondary track
    track_xml.push_str("                <connections>\n");
    track_xml.push_str("                </connections>\n");
    track_xml.push_str("            </trackTopology>\n");
    track_xml.push_str("        </track>\n");
    
    track_xml
}

fn create_simple_track_structure(model: &Model) -> String {
    let mut track_xml = String::new();
    track_xml.push_str("        <track name=\"SP1\" id=\"track1\">\n");
    track_xml.push_str("            <trackTopology>\n");
    
    // Calculate track length from line segments
    let track_length = calculate_track_length(&model.linesegs);
    
    // Track begin
    track_xml.push_str("                <trackBegin pos=\"0.0\" id=\"tb1\">\n");
    track_xml.push_str("                    <openEnd id=\"b1\" />\n");
    track_xml.push_str("                </trackBegin>\n");
    
    // Track end
    track_xml.push_str(&format!("                <trackEnd pos=\"{}\" id=\"te1\">\n", track_length));
    track_xml.push_str("                    <openEnd id=\"b2\" />\n");
    track_xml.push_str("                </trackEnd>\n");
    
    // No connections
    track_xml.push_str("                <connections>\n");
    track_xml.push_str("                </connections>\n");
    track_xml.push_str("            </trackTopology>\n");
    
    // Add OCS elements dynamically
    track_xml.push_str("            <ocsElements>\n");
    
    // Add signals dynamically
    track_xml.push_str("                <signals>\n");
    let mut signal_id = 1;
    for (pos, obj) in &model.objects {
        for function in &obj.functions {
            if let Function::Signal { .. } = function {
                let signal_pos = calculate_position_on_track(pos, &model.linesegs, track_length);
                track_xml.push_str(&format!("                    <signal id=\"sig{}\" name=\"Signal {}\" pos=\"{}\" type=\"main\" dir=\"up\"/>\n", 
                                          signal_id, signal_id, signal_pos));
                signal_id += 1;
            }
        }
    }
    track_xml.push_str("                </signals>\n");
    
    // Add train detectors dynamically
    track_xml.push_str("                <trainDetectionElements>\n");
    let mut detector_id = 0;
    for (pos, obj) in &model.objects {
        for function in &obj.functions {
            if let Function::Detector = function {
                let detector_pos = calculate_position_on_track(pos, &model.linesegs, track_length);
                track_xml.push_str(&format!("                    <trainDetector id=\"d{}\" name=\"detector {}\" pos=\"{}\" />\n", 
                                          detector_id, detector_id + 1, detector_pos));
                detector_id += 1;
            }
        }
    }
    track_xml.push_str("                </trainDetectionElements>\n");
    
    track_xml.push_str("            </ocsElements>\n");
    track_xml.push_str("        </track>\n");
    
    track_xml
}

fn generate_railml_xml(tracks: &[String]) -> String {
    let mut xml = String::new();
    xml.push_str("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n");
    xml.push_str("<railml xmlns:xsd=\"http://www.w3.org/2001/XMLSchema\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" xmlns=\"http://www.railml.org/schemas/2013\">\n");
    xml.push_str("  <infrastructure>\n");
    xml.push_str("    <tracks>\n");
    
    for track in tracks {
        xml.push_str(track);
    }
    
    xml.push_str("    </tracks>\n");
    xml.push_str("  </infrastructure>\n");
    xml.push_str("</railml>\n");
    
    xml
} 

fn calculate_track_length(linesegs: &im::HashSet<(Pt, Pt)>) -> f64 {
    let mut total_length = 0.0;
    for (start, end) in linesegs {
        let dx = (end.x - start.x) as f64;
        let dy = (end.y - start.y) as f64;
        total_length += (dx * dx + dy * dy).sqrt();
    }
    total_length * 10.0 // Scale factor for railML coordinates
}

fn calculate_position_on_track(pos: &Pt, linesegs: &im::HashSet<(Pt, Pt)>, track_length: f64) -> f64 {
    // Find the closest line segment to the position
    let mut min_distance = f64::INFINITY;
    let mut closest_segment = None;
    let mut segment_position = 0.0;
    
    for (start, end) in linesegs {
        let distance = point_to_line_distance(pos, start, end);
        if distance < min_distance {
            min_distance = distance;
            closest_segment = Some((start, end));
        }
    }
    
    if let Some((start, end)) = closest_segment {
        // Calculate position along the track based on the closest segment
        let dx = (end.x - start.x) as f64;
        let dy = (end.y - start.y) as f64;
        let segment_length = (dx * dx + dy * dy).sqrt();
        
        // Calculate relative position on the segment
        let relative_pos = if segment_length > 0.0 {
            let px = (pos.x - start.x) as f64;
            let py = (pos.y - start.y) as f64;
            (px * dx + py * dy) / (dx * dx + dy * dy)
        } else {
            0.0
        };
        
        // Scale to railML coordinates
        segment_position = relative_pos * segment_length * 10.0;
    }
    
    // Ensure position is within track bounds
    segment_position.max(0.0).min(track_length)
}

fn point_to_line_distance(point: &Pt, line_start: &Pt, line_end: &Pt) -> f64 {
    let px = point.x as f64;
    let py = point.y as f64;
    let x1 = line_start.x as f64;
    let y1 = line_start.y as f64;
    let x2 = line_end.x as f64;
    let y2 = line_end.y as f64;
    
    let dx = x2 - x1;
    let dy = y2 - y1;
    
    if dx == 0.0 && dy == 0.0 {
        // Line segment is a point
        return ((px - x1) * (px - x1) + (py - y1) * (py - y1)).sqrt();
    }
    
    let t = ((px - x1) * dx + (py - y1) * dy) / (dx * dx + dy * dy);
    let t = t.max(0.0).min(1.0);
    
    let closest_x = x1 + t * dx;
    let closest_y = y1 + t * dy;
    
    ((px - closest_x) * (px - closest_x) + (py - closest_y) * (py - closest_y)).sqrt()
} 