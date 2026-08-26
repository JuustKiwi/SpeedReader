use std::env;
use std::fs;
use ratatui::style::Color;

fn parse_color( color_str: &str ) -> Color {
    match color_str.to_lowercase().as_str() {
        "red" => Color::Red,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "green" => Color::Green,
        "gray" => Color::Gray,
        "magenta" => Color::Magenta,
        "cyan" => Color::Cyan,
        _ => Color::White,
    }
}

/// Loads preferences from ~/.config/speedreader.conf
pub fn load_config() -> ( f32, Color, Color ) {
    let mut wpm = 350.0;
    let mut h_color = Color::Red;
    let mut t_color = Color::White;

    if let Ok( home ) = env::var( "HOME" ) {
        let config_path = format!( "{}/.config/speedreader.conf", home );
        if let Ok( content ) = fs::read_to_string( config_path ) {
            for line in content.lines() {
                let parts: Vec<&str> = line.split( '=' ).collect();
                if parts.len() == 2 {
                    match parts[ 0 ].trim() {
                        "wpm" => if let Ok( v ) = parts[ 1 ].trim().parse::<f32>() { wpm = v; },
                        "highlight_color" => h_color = parse_color( parts[ 1 ].trim() ),
                        "text_color" => t_color = parse_color( parts[ 1 ].trim() ),
                        _ => {}
                    }
                }
            }
        }
    }
    ( wpm, h_color, t_color )
}
