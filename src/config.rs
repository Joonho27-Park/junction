use confy;
use std::collections::HashMap;
use lazy_static::*;
use const_cstr::const_cstr;
use backend_glfw::imgui::*;
use palette;
use num_derive::FromPrimitive;
use log::*;
use enum_map::{enum_map, Enum, EnumMap};
use serde::{Serialize, Deserialize};

type Color = palette::rgb::Rgba;


// Named color choices
// based on Vicos 
// https://orv.banenor.no/orv/lib/exe/fetch.php?media=brukerveiledninger:symbolkatalog_vicos_-_iup-00-s-20385_00e_001.pdf
//
//
//  platform: turquoise
//
//  train routes
//    free tvd: gray
//    occupied tvd: red
//    reserved tvd: green
//    overlap tvd: orange
//  (add blink when releasing, if release has any delay (currently not modelled))
//
//  Shunting route:
//    reserved tvd: yellow
//    occupied tvd: turquoise
//  (add blink when releasing, if release has any delay (currently not modelled))
//
//  Operator has blocked route: two red cross-bars over track
//
//  switch:
//    remove track part which is not connected 
//        ---         /--
//     ------   vs. -/   ---- 
//    operator blocked switch: red box
//    switch locked/blocked (by route?): small yellow track section
//    providing flank protection: blue dot in fouling point
//
//  main signals
//    not used: gray triangle outline
//    part of train route: red triangle outline
//    proceed: green triangle outline
//    blocked by operator(?): red filled triangle
//    other: automatic etc...
//
//  shunting signal:
//    arrow instead of triangle
//
//
//


lazy_static! {
    pub static ref COLORNAMES :EnumMap<RailUIColorName, const_cstr::ConstCStr> = {
        enum_map! {
                RailUIColorName::CanvasBackground => const_cstr!("Canvas background"),
                RailUIColorName::CanvasGridPoint => const_cstr!("Canvas grid point"),
                RailUIColorName::CanvasSymbol => const_cstr!("Canvas symbol"),
                RailUIColorName::CanvasSymbolSelected => const_cstr!("Canvas symbol selected"),
                RailUIColorName::CanvasSymbolLocError => const_cstr!("Canvas symbol location error"),
                RailUIColorName::CanvasSignalStop => const_cstr!("Canvas signal stop"),
                RailUIColorName::CanvasSignalProceed => const_cstr!("Canvas signal proceed"),
                RailUIColorName::CanvasDetector => const_cstr!("Canvas detector"),
                RailUIColorName::CanvasTrack => const_cstr!("Canvas track"),
                RailUIColorName::CanvasTrackDrawing => const_cstr!("Canvas drawing track"),
                RailUIColorName::CanvasTrackSelected => const_cstr!("Canvas track selected"),
                RailUIColorName::CanvasNode => const_cstr!("Canvas node"),
                RailUIColorName::CanvasNodeSelected => const_cstr!("Canvas node selected"),
                RailUIColorName::CanvasNodeError => const_cstr!("Canvas node error"),
                RailUIColorName::CanvasTrain => const_cstr!("Canvas train "),
                RailUIColorName::CanvasTrainSight => const_cstr!("Canvas train sighted signal"),
                RailUIColorName::CanvasTVDFree => const_cstr!("Canvas TVD free"),
                RailUIColorName::CanvasTVDOccupied => const_cstr!("Canvas TVD occupied"),
                RailUIColorName::CanvasTVDReserved => const_cstr!("Canvas TVD reserved"),
                RailUIColorName::CanvasRoutePath => const_cstr!("Canvas route path"),
                RailUIColorName::CanvasRouteSection => const_cstr!("Canvas route section"),
                RailUIColorName::CanvasSelectionWindow => const_cstr!("Canvas selection window"),
                RailUIColorName::CanvasText => const_cstr!("Canvas text"),
                RailUIColorName::GraphBackground => const_cstr!("Graph background"),
                RailUIColorName::GraphTimeSlider => const_cstr!("Graph time slider"),
                RailUIColorName::GraphTimeSliderText => const_cstr!("Graph time slider text"),
                RailUIColorName::GraphBlockBorder => const_cstr!("Graph block border"),
                RailUIColorName::GraphBlockReserved => const_cstr!("Graph block reserved"),
                RailUIColorName::GraphBlockOccupied => const_cstr!("Graph block occupied"),
                RailUIColorName::GraphTrainFront => const_cstr!("Graph train front"),
                RailUIColorName::GraphTrainRear => const_cstr!("Graph train rear"),
                RailUIColorName::GraphCommandRoute => const_cstr!("Graph command route"),
                RailUIColorName::GraphCommandTrain => const_cstr!("Graph command train"),
                RailUIColorName::GraphCommandError => const_cstr!("Graph command error"),
                RailUIColorName::GraphCommandBorder => const_cstr!("Graph command border"),
        }
    };
}

#[derive(Debug)]
pub struct Config {
    pub colors :EnumMap<RailUIColorName,Color>,
}


/// serde-friendly representation of the config struct
#[derive(Serialize,Deserialize)]
#[derive(Debug)]
pub struct ConfigString {
    pub colors :Vec<(String,String)>,  // name -> hex color
}

fn to_hex(c :Color) -> String {
    use palette::encoding::pixel::Pixel;
    let px  :[u8;4] = c.into_format().into_raw();
    format!("#{:02x}{:02x}{:02x}{:02x}", px[0],px[1],px[2],px[3])
}

fn from_hex(mut s :&str) -> Result<Color, ()> {
    // chop off '#' char
    if s.len() % 2 != 0 { s = &s[1..]; }
    if !(s.len() == 6 || s.len() == 8) { return Err(()); }
    // u8::from_str_radix(src: &str, radix: u32) converts a string
    // slice in a given base to u8
    let r: u8 = u8::from_str_radix(&s[0..2], 16).map_err(|_| ())?;
    let g: u8 = u8::from_str_radix(&s[2..4], 16).map_err(|_| ())?;
    let b: u8 = u8::from_str_radix(&s[4..6], 16).map_err(|_| ())?;
    let a = if s.len() == 8 {
        u8::from_str_radix(&s[6..8], 16).map_err(|_| ())?
    } else { 255u8 };

    Ok(Color::new(r as f32 / 255.0,
                  g as f32 / 255.0,
                  b as f32 / 255.0,
                  a as f32 / 255.0))
}

impl Default for ConfigString {
    fn default() -> Self {
        let c : Config = Default::default();
        c.to_config_string()
    }
}

impl Config {

    pub fn load() -> Self {
        let config_s : ConfigString = confy::load(env!("CARGO_PKG_NAME")).
            unwrap_or_else(|e| {
                error!("Could not load config file: {}", e);
                Default::default()
            });
        let config : Config = Config::from_config_string(&config_s);
        config
    }

    pub fn save(&self) {
        if let Err(e) = confy::store(env!("CARGO_PKG_NAME"), self.to_config_string()) {
            error!("Could not save config file: {}", e);
        }
    }


    pub fn to_config_string(&self) ->  ConfigString {
        let mut colors = Vec::new();
        unsafe {
            for (c,val) in self.colors.iter() {
                colors.push((std::str::from_utf8_unchecked(COLORNAMES[c].as_cstr().to_bytes()).to_string(), 
                             to_hex(*val)));
            }
        }

        ConfigString {
            colors: colors,
        }
    }

    pub fn from_config_string(cs :&ConfigString) -> Self {
        let mut colors = default_colors();
        for (name,col_hex) in cs.colors.iter() {
            for (col_choice, name_cstr) in COLORNAMES.iter() {
                unsafe {
                    if std::str::from_utf8_unchecked(name_cstr.as_cstr().to_bytes()) == name {
                        if let Ok(c) = from_hex(col_hex) {
                            colors[col_choice] = c;
                        }
                    }
                }
            }
        }

        Config {
            colors: colors,
        }
    }

    pub fn get_font_size(&self) -> f32 { 16.0 }
    pub fn get_font_filename(&self) -> Option<String> {
        use font_kit::source::SystemSource;
        use font_kit::family_name::FamilyName;
        use font_kit::properties::Properties;
        use font_kit::handle::Handle;
        let font = SystemSource::new().select_best_match(&[
                                                 //FamilyName::Title("Segoe UI".to_string()),
                                                 FamilyName::SansSerif],
                                                 &Properties::new()).ok()?;
        match font {
            Handle::Path { path, font_index } => {
                info!("Using font {:?}", path);
                let f = path.to_string_lossy().to_string();
                Some(f)
            },
            _ => { None }
        }

    }



    pub fn color_u32(&self, name :RailUIColorName) -> u32 {
        let c = self.colors[name];
        unsafe { igGetColorU32Vec4(ImVec4 { x: c.color.red,  y: c.color.green, 
            z: c.color.blue, w: c.alpha  }) }
    }
}

impl Default for Config {
    fn default() -> Config {
        Config {
            colors: default_colors(),
        }
    }
}

pub fn default_colors() -> EnumMap<RailUIColorName, Color> {
    use palette::named;
    let c = |nm :palette::Srgb<u8>| {
        let f :palette::Srgb<f32> = palette::Srgb::from_format(nm);
        let a :Color = f.into();
        a
    };
    
    // 커스텀 색상 정의 함수 (RGBA)
    let custom_color_rgba = |r: u8, g: u8, b: u8, a: u8| {
        let f = palette::Srgb::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0);
        let mut color: Color = f.into();
        color.alpha = a as f32 / 255.0;
        color
    };
    
    enum_map! {
        RailUIColorName::CanvasBackground => custom_color_rgba(0xFC, 0xF3, 0xD8, 0xFF),      // #FCF3D8
        RailUIColorName::CanvasGridPoint => custom_color_rgba(0xFF, 0xB7, 0x82, 0x50),       // #FFB78250
        RailUIColorName::CanvasSymbol => custom_color_rgba(0x44, 0x47, 0x5A, 0xFF),          // #44475A
        RailUIColorName::CanvasSymbolSelected => custom_color_rgba(0xA2, 0xAE, 0xD6, 0xFF),   // #A2AED6
        RailUIColorName::CanvasSymbolLocError => custom_color_rgba(0xFF, 0x55, 0x55, 0xFF),   // #FF5555
        RailUIColorName::CanvasSignalStop => custom_color_rgba(0xFF, 0x55, 0x55, 0xC8),       // #FF5555C8
        RailUIColorName::CanvasSignalProceed => custom_color_rgba(0x49, 0xE9, 0xA6, 0xC8),    // #49E9A6C8
        RailUIColorName::CanvasDetector => custom_color_rgba(0xFC, 0xF3, 0xD8, 0xFF),         // #FCF3D8
        RailUIColorName::CanvasTrack => custom_color_rgba(0x28, 0x2A, 0x36, 0xFF),            // #282A36
        RailUIColorName::CanvasTrackDrawing => custom_color_rgba(0x28, 0x2A, 0x36, 0xC8),     // #282A36C8
        RailUIColorName::CanvasTrackSelected => custom_color_rgba(0xA1, 0xAE, 0xD6, 0xFF),     // #A1AED6
        RailUIColorName::CanvasNode => custom_color_rgba(0x28, 0x2A, 0x36, 0xFF),             // #282A36
        RailUIColorName::CanvasNodeSelected => custom_color_rgba(0xA2, 0xAE, 0xD6, 0xFF),      // #A2AED6
        RailUIColorName::CanvasNodeError => custom_color_rgba(0xFF, 0x55, 0x55, 0xFF),         // #FF5555
        RailUIColorName::CanvasTrain => custom_color_rgba(0xFF, 0x7E, 0xAC, 0x96),            // #FF7EAC96
        RailUIColorName::CanvasTrainSight => custom_color_rgba(0x44, 0x47, 0x5A, 0xA7),       // #44475AA7
        RailUIColorName::CanvasTVDFree => custom_color_rgba(0xF8, 0xF8, 0xF2, 0xFF),          // #F8F8F2
        RailUIColorName::CanvasTVDOccupied => custom_color_rgba(0xFF, 0x55, 0x55, 0x82),      // #FF555582
        RailUIColorName::CanvasTVDReserved => custom_color_rgba(0xFF, 0xE5, 0x00, 0xC8),      // #FFE500C8
        RailUIColorName::CanvasRoutePath => custom_color_rgba(0x55, 0xFA, 0x7B, 0x82),        // #55FA7B82
        RailUIColorName::CanvasRouteSection => custom_color_rgba(0x8B, 0xE9, 0xFD, 0x82),     // #8BE9FD82
        RailUIColorName::CanvasSelectionWindow => custom_color_rgba(0x62, 0x72, 0xA4, 0xFF),   // #6272A4
        RailUIColorName::CanvasText => custom_color_rgba(0xF8, 0xF8, 0xF2, 0xFF),             // #F8F8F2
        RailUIColorName::GraphBackground => custom_color_rgba(0x3F, 0x3F, 0x3F, 0xFF),         // #3F3F3F
        RailUIColorName::GraphTimeSlider => custom_color_rgba(0x84, 0xAB, 0xB5, 0xFF),         // #84ABB5
        RailUIColorName::GraphTimeSliderText => custom_color_rgba(0xB9, 0xD4, 0xDA, 0xFF),     // #B9D4DA
        RailUIColorName::GraphBlockBorder => custom_color_rgba(0x00, 0x00, 0x00, 0xFF),        // #000000
        RailUIColorName::GraphBlockReserved => custom_color_rgba(0x53, 0x53, 0x53, 0xFF),      // #535353
        RailUIColorName::GraphBlockOccupied => custom_color_rgba(0x61, 0x61, 0x61, 0xFF),      // #616161
        RailUIColorName::GraphTrainFront => custom_color_rgba(0xFF, 0xFE, 0xFE, 0xCC),        // #FFFEFECC
        RailUIColorName::GraphTrainRear => custom_color_rgba(0xFF, 0xFE, 0xFE, 0xCC),          // #FFFEFECC
        RailUIColorName::GraphCommandRoute => custom_color_rgba(0x16, 0xB6, 0x73, 0xFF),      // #16B673
        RailUIColorName::GraphCommandTrain => custom_color_rgba(0xFF, 0x8B, 0xB3, 0xCC),      // #FF8BB3CC
        RailUIColorName::GraphCommandError => custom_color_rgba(0xE6, 0x41, 0x00, 0xFF),      // #E64100
        RailUIColorName::GraphCommandBorder => custom_color_rgba(0xFF, 0xFF, 0xFF, 0x00),     // #FFFFFF00
    }
}

#[derive(Enum, Debug, PartialEq, Eq, Copy, Clone)]
#[derive(Serialize,Deserialize)]
pub enum RailUIColorName {
    CanvasBackground,
    CanvasGridPoint,
    CanvasSymbol,
    CanvasSymbolSelected,
    CanvasSymbolLocError,
    CanvasSignalStop,
    CanvasSignalProceed,
    CanvasDetector,
    CanvasTrack,
    CanvasTrackDrawing,
    CanvasTrackSelected,
    CanvasNode,
    CanvasNodeSelected,
    CanvasNodeError,
    CanvasTrain,
    CanvasTrainSight,
    CanvasTVDFree,
    CanvasTVDOccupied,
    CanvasTVDReserved,
    CanvasRoutePath,
    CanvasRouteSection,
    CanvasSelectionWindow,
    CanvasText,
    GraphBackground,
    GraphTimeSlider,
    GraphTimeSliderText,
    GraphBlockBorder,
    GraphBlockReserved,
    GraphBlockOccupied,
    GraphTrainFront,
    GraphTrainRear,
    GraphCommandRoute,
    GraphCommandTrain,
    GraphCommandError,
    GraphCommandBorder,
}

#[test]
pub fn colr_no() {
    //use num_traits::FromPrimitive;
    //let x :RailUIColorName = RailUIColorName::from_usize(2usize);
    //dbg!(x);
    //assert_eq!(x, RailUIColorName::CanvasSymbol);
}


