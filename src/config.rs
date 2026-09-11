use std::env;
use std::fs;
use ratatui::style::Color;

pub struct Config {
    pub wpm: f32,
    pub slow_wpm: f32,
    pub fast_wpm: f32,
    pub h_color: Color,
    pub t_color: Color,
}

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

/// Loads config from ~/.config/speedreader.conf
pub fn load_config() -> Config {
    let mut cfg = Config {
        wpm: 350.0,
        slow_wpm: 250.0,
        fast_wpm: 550.0,
        h_color: Color::Red,
        t_color: Color::White,
    };

    if let Ok( home ) = env::var( "HOME" ) {
        let config_path = format!( "{}/.config/speedreader.conf", home );
        if let Ok( content ) = fs::read_to_string( config_path ) {
            for line in content.lines() {
                let parts: Vec<&str> = line.split( '=' ).collect();
                if parts.len() == 2 {
                    match parts[ 0 ].trim() {
                        "wpm" => if let Ok( v ) = parts[ 1 ].trim().parse::<f32>() { cfg.wpm = v; },
                        "slow_wpm" => if let Ok( v ) = parts[ 1 ].trim().parse::<f32>() { cfg.slow_wpm = v; },
                        "fast_wpm" => if let Ok( v ) = parts[ 1 ].trim().parse::<f32>() { cfg.fast_wpm = v; },
                        "highlight_color" => cfg.h_color = parse_color( parts[ 1 ].trim() ),
                        "text_color" => cfg.t_color = parse_color( parts[ 1 ].trim() ),
                        _ => {}
                    }
                }
            }
        }
    }
    cfg
}
