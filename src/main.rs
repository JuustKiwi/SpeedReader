use std::env;
use std::fs;
use std::ffi::{ CString, CStr };
use std::os::raw::{ c_char, c_int, c_float };
use std::time::{ Duration, Instant };
use std::io;

use crossterm::{
    event::{ self, Event, KeyCode },
    execute,
    terminal::{ disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen },
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{ Alignment, Constraint, Direction, Layout },
    style::{ Color, Style, Modifier },
    text::{ Line, Span },
    widgets::{ Block, Borders, Paragraph },
    Terminal,
};

unsafe extern "C" {
    fn load_pdf_session( file_path: *const c_char, start_page: c_int, end_page: c_int ) -> bool;
    fn load_pdf_chapter( file_path: *const c_char, target_chapter: c_int ) -> bool; // <--- ADD THIS
    fn get_next_word( buffer: *mut c_char, max_len: c_int, orp_index: *mut c_int, delay_multiplier: *mut c_float ) -> bool;
}

struct RsvpWord {
    text: String,
    orp_index: usize,
    delay_mult: f32,
}

fn parse_color( color_str: &str ) -> Color {
    match color_str.to_lowercase().as_str() {
        "red" => Color::Red,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "green" => Color::Green,
        "white" => Color::White,
        "gray" => Color::Gray,
        "magenta" => Color::Magenta,
        "cyan" => Color::Cyan,
        _ => Color::White,
    }
}

// Reads ~/.config/speedreader.conf
fn load_config() -> ( f32, Color, Color ) {
    let mut wpm = 350.0;
    let mut h_color = Color::Red;
    let mut t_color = Color::White;

    if let Ok( home ) = env::var( "HOME" ) {
        let config_path = format!( "{}/.config/speedreader.conf", home );
        if let Ok( content ) = fs::read_to_string( config_path ) {
            for line in content.lines() {
                let parts: Vec<&str> = line.split( '=' ).collect();
                if parts.len() == 2 {
                    let key = parts[ 0 ].trim();
                    let val = parts[ 1 ].trim();
                    match key {
                        "wpm" => if let Ok( v ) = val.parse::<f32>() { wpm = v; },
                        "highlight_color" => h_color = parse_color( val ),
                        "text_color" => t_color = parse_color( val ),
                        _ => {}
                    }
                }
            }
        }
    }
    ( wpm, h_color, t_color )
}

fn main() -> Result< (), io::Error > {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 4 {
        eprintln!( "Usage:" );
        eprintln!( "  By pages:   speedreader <pdf> -p <start_page> <end_page>" );
        eprintln!( "  By chapter: speedreader <pdf> -c <chapter_num>" );
        return Ok( () );
    }

    let target_pdf = &args[ 1 ];
    let mode = &args[ 2 ];
    let mut session_words: Vec<RsvpWord> = Vec::new();
    let file_path = CString::new( target_pdf.as_str() ).expect( "Failed to create CString" );

    unsafe {
        let success = if mode == "-p" || mode == "--pages" {
            let start: c_int = args[ 3 ].parse().unwrap_or( 1 );
            let end: c_int = args.get( 4 ).unwrap_or( &args[ 3 ] ).parse().unwrap_or( start );
            load_pdf_session( file_path.as_ptr(), start, end )
        } else if mode == "-c" || mode == "--chapter" {
            let chapter: c_int = args[ 3 ].parse().unwrap_or( 1 );
            load_pdf_chapter( file_path.as_ptr(), chapter )
        } else {
            eprintln!( "Invalid mode. Use -p for pages or -c for chapter." );
            return Ok( () );
        };

        if !success {
            eprintln!( "Failed to load PDF or find the specified pages/chapter." );
            return Ok( () );
        }( () );
   
        let mut buffer = vec![ 0u8; 256 ];
        let mut orp_index: c_int = 0;
        let mut delay: c_float = 0.0;

        loop {
            if !get_next_word( buffer.as_mut_ptr() as *mut c_char, 256, &mut orp_index, &mut delay ) {
                break;
            }
            let c_str = CStr::from_ptr( buffer.as_ptr() as *const c_char );
            if let Ok( rust_str ) = c_str.to_str() {
                session_words.push( RsvpWord {
                    text: rust_str.to_string(),
                    orp_index: orp_index as usize,
                    delay_mult: delay,
                } );
            }
        }
    }

    if session_words.is_empty() {
        eprintln!( "No words found in this page range." );
        return Ok( () );
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!( stdout, EnterAlternateScreen )?;
    let backend = CrosstermBackend::new( stdout );
    let mut terminal = Terminal::new( backend )?;

    let mut current_idx = 0;
    let mut is_paused = true; 
    
    let ( mut wpm, highlight_color, text_color ) = load_config();
    
    let mut base_delay = Duration::from_secs_f32( 60.0 / wpm );
    let mut active_delay = base_delay;
    let mut last_tick = Instant::now();

    loop {
        terminal.draw( |f| {
            let size = f.size();
            let word = &session_words[ current_idx ];
            
            let chars: Vec<char> = word.text.chars().collect();
            let orp = word.orp_index.min( chars.len().saturating_sub( 1 ) );

            let left_str: String = chars[ 0..orp ].iter().collect();
            let center_char: String = chars[ orp..orp + 1 ].iter().collect();
            let right_str: String = chars[ orp + 1.. ].iter().collect();

            let max_side = left_str.chars().count().max( right_str.chars().count() );
            let left_padded = format!( "{:>width$}", left_str, width = max_side );
            let right_padded = format!( "{:<width$}", right_str, width = max_side );

            let text = Line::from( vec![
                Span::styled( left_padded, Style::default().fg( text_color ) ),
                Span::styled( center_char, Style::default().fg( highlight_color ).add_modifier( Modifier::BOLD ) ),
                Span::styled( right_padded, Style::default().fg( text_color ) ),
            ] );

            let status_title = if is_paused { " PAUSED (Press Space) " } else { " READING " };
            let progress_title = format!( " Word: {}/{} | WPM: {} ", current_idx + 1, session_words.len(), wpm );

            let paragraph = Paragraph::new( text )
                .alignment( Alignment::Center )
                .block( Block::default()
                    .borders( Borders::ALL )
                    .title( status_title )
                    .title_alignment( Alignment::Center )
                    .title_bottom( progress_title ) );


            let vertical_chunks = Layout::default()
                .direction( Direction::Vertical )
                .constraints( [
                    Constraint::Percentage( 40 ),
                    Constraint::Length( 3 ),
                    Constraint::Percentage( 40 ),
                    Constraint::Length( 1 ),
                ] )
                .split( size );

            f.render_widget( paragraph, vertical_chunks[ 1 ] );

            let controls_text = Line::from( "Controls: [Space] Play/Pause | [Up/Down] Speed | [Left/Right] Scrub | [Q] Quit" );
            let controls_p = Paragraph::new( controls_text )
                .alignment( Alignment::Center )
                .style( Style::default().fg( Color::DarkGray ) );
            
            f.render_widget( controls_p, vertical_chunks[ 3 ] );
        } )?;

        if event::poll( Duration::from_millis( 10 ) )? {
            if let Event::Key( key ) = event::read()? {
                match key.code {
                    KeyCode::Char( 'q' ) | KeyCode::Esc => break,
                    KeyCode::Char( ' ' ) => is_paused = !is_paused,
                    KeyCode::Left => current_idx = current_idx.saturating_sub( 10 ),
                    KeyCode::Right => current_idx = ( current_idx + 10 ).min( session_words.len() - 1 ),
                    KeyCode::Up => {
                        wpm += 25.0;
                        base_delay = Duration::from_secs_f32( 60.0 / wpm );
                    }
                    KeyCode::Down => {
                        if wpm > 50.0 { 
                            wpm -= 25.0;
                            base_delay = Duration::from_secs_f32( 60.0 / wpm );
                        }
                    }
                    _ => {}
                }
            }
        }

        if !is_paused && last_tick.elapsed() >= active_delay {
            current_idx += 1;
            if current_idx >= session_words.len() {
                break;
            }
            last_tick = Instant::now();
            active_delay = Duration::from_secs_f32( base_delay.as_secs_f32() * session_words[ current_idx ].delay_mult );
        }
    }

    disable_raw_mode()?;
    execute!( terminal.backend_mut(), LeaveAlternateScreen )?;
    terminal.show_cursor()?;

    Ok( () )
}
