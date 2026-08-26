use std::io;
use std::time::{ Duration, Instant };
use std::os::raw::c_int;

use crossterm::{
    event::{ self, Event, KeyCode, MouseEventKind, MouseButton, EnableMouseCapture, DisableMouseCapture },
    execute,
    terminal::{ disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen },
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{ Alignment, Constraint, Direction, Layout },
    style::{ Color, Style, Modifier },
    text::{ Line, Span, Text },
    widgets::{ Block, Borders, Paragraph },
    Terminal,
};

use crate::ffi::{ RsvpWord, sync_sr_progress };
use crate::config::load_config;

pub fn run_viewer_mode( words: &[RsvpWord], start_idx: usize, ext: &str ) -> Result< (), io::Error > {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    
    execute!( stdout, EnterAlternateScreen, EnableMouseCapture )?;
    let backend = CrosstermBackend::new( stdout );
    let mut terminal = Terminal::new( backend )?;

    let mut view_offset = start_idx;
    let mut cursor_idx = start_idx;
    let mut word_positions: Vec<(usize, u16, u16, u16)> = Vec::new();

    loop {
        if cursor_idx < view_offset { view_offset = cursor_idx; }
        
        let mut needs_scroll_down = false;
        let mut next_view_offset = view_offset;
        let mut recorded_next_line = false;
        let current_view_offset = view_offset;

        terminal.draw( |f| {
            let size = f.size();
            let inner_width = size.width.saturating_sub( 2 );
            let inner_height = size.height.saturating_sub( 2 );

            let mut lines = Vec::new();
            let mut current_spans = Vec::new();
            let mut cx = 0;
            let mut cy = 0;
            word_positions.clear();

            let mut actual_idx = view_offset;

            while actual_idx < words.len() && cy < inner_height {
                if actual_idx > 0 && actual_idx % 250 == 0 {
                    if !current_spans.is_empty() {
                        lines.push( Line::from( current_spans.clone() ) );
                        current_spans.clear();
                        cy += 1;
                        cx = 0;
                    }
                    if cy < inner_height { lines.push( Line::raw( "" ) ); cy += 1; }
                    if cy < inner_height {
                        let marker = format!( "[ Page {} | Word {} ]", ( actual_idx / 250 ) + 1, actual_idx );
                        lines.push( Line::from( Span::styled( marker, Style::default().fg( Color::DarkGray ).add_modifier( Modifier::BOLD ) ) ) );
                        cy += 1;
                    }
                    if cy < inner_height { lines.push( Line::raw( "" ) ); cy += 1; }
                }
                
                if cy >= inner_height { break; }

                let word = &words[ actual_idx ];
                let wlen = word.text.chars().count() as u16;

                if cx + wlen > inner_width && cx > 0 {
                    lines.push( Line::from( current_spans.clone() ) );
                    current_spans.clear();
                    cy += 1;
                    cx = 0;
                }
                
                if cy >= inner_height { break; }

                if cy > 0 && !recorded_next_line && actual_idx > current_view_offset {
                    next_view_offset = actual_idx;
                    recorded_next_line = true;
                }

                word_positions.push( ( actual_idx, cy, cx, cx + wlen ) );

                let style = if actual_idx == cursor_idx {
                    Style::default().bg( Color::White ).fg( Color::Black ).add_modifier( Modifier::BOLD )
                } else {
                    Style::default()
                };

                current_spans.push( Span::styled( format!( "{} ", word.text ), style ) );
                cx += wlen + 1;
                actual_idx += 1;
            }
            
            if !current_spans.is_empty() && cy < inner_height {
                lines.push( Line::from( current_spans ) );
            }

            if cursor_idx >= actual_idx && actual_idx < words.len() {
                needs_scroll_down = true;
            }

            let progress = format!( " Word: {}/{} | Page: {} | [Click/Arrows] Select | [Q] Quit ", cursor_idx, words.len(), (cursor_idx/250)+1 );
            let p = Paragraph::new( Text::from( lines ) )
                .block( Block::default().borders( Borders::ALL ).title( " READER MODE " ).title_alignment( Alignment::Center ).title_bottom( progress ) );
                
            f.render_widget( p, size );
        } )?;

        if needs_scroll_down {
            view_offset = next_view_offset;
            if view_offset <= current_view_offset { view_offset = current_view_offset + 1; }
            continue; 
        }
        
        if event::poll( Duration::from_millis( 50 ) )? {
            match event::read()? {
                Event::Key( key ) => {
                    match key.code {
                        KeyCode::Char( 'q' ) | KeyCode::Esc => break,
                        KeyCode::Left => cursor_idx = cursor_idx.saturating_sub( 1 ),
                        KeyCode::Right => cursor_idx = ( cursor_idx + 1 ).min( words.len().saturating_sub( 1 ) ),
                        KeyCode::Up => cursor_idx = cursor_idx.saturating_sub( 20 ),
                        KeyCode::Down => cursor_idx = ( cursor_idx + 20 ).min( words.len().saturating_sub( 1 ) ),
                        KeyCode::PageUp => cursor_idx = cursor_idx.saturating_sub( 250 ),
                        KeyCode::PageDown => cursor_idx = ( cursor_idx + 250 ).min( words.len().saturating_sub( 1 ) ),
                        _ => {}
                    }
                }
                Event::Mouse( mouse_event ) => {
                    if let MouseEventKind::Down( MouseButton::Left ) = mouse_event.kind {
                        let mx = mouse_event.column.saturating_sub( 1 );
                        let my = mouse_event.row.saturating_sub( 1 );
                        for &( idx, row, col_start, col_end ) in &word_positions {
                            if row == my && mx >= col_start && mx <= col_end {
                                cursor_idx = idx;
                                break;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    if ext == "sr" { unsafe { sync_sr_progress( cursor_idx as c_int ); } }

    disable_raw_mode()?;
    execute!( terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture )?;
    terminal.show_cursor()?;
    Ok( () )
}

pub fn run_rsvp_mode( words: &[RsvpWord], start_idx: usize, ext: &str ) -> Result< (), io::Error > {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!( stdout, EnterAlternateScreen )?;
    let backend = CrosstermBackend::new( stdout );
    let mut terminal = Terminal::new( backend )?;

    let mut current_idx = start_idx;
    let mut is_paused = true; 
    let ( mut wpm, highlight_color, text_color ) = load_config();
    let mut base_delay = Duration::from_secs_f32( 60.0 / wpm );
    let mut active_delay = base_delay;
    let mut last_tick = Instant::now();

    loop {
        terminal.draw( |f| {
            let size = f.size();
            let word = &words[ current_idx ];
            
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

            let progress = format!( " Word: {}/{} | Page: {} | WPM: {} ", current_idx + 1, words.len(), (current_idx/250)+1, wpm );
            let paragraph = Paragraph::new( text ).alignment( Alignment::Center )
                .block( Block::default().borders( Borders::ALL )
                .title( if is_paused { " PAUSED " } else { " READING " } ).title_alignment( Alignment::Center )
                .title_bottom( progress ) );

            let vertical_chunks = Layout::default().direction( Direction::Vertical )
                .constraints( [ Constraint::Percentage( 40 ), Constraint::Length( 3 ), Constraint::Percentage( 40 ), Constraint::Length( 1 ) ] ).split( size );

            f.render_widget( paragraph, vertical_chunks[ 1 ] );
            f.render_widget( Paragraph::new( "Controls: [Space] Play/Pause | [Up/Down] Speed | [Left/Right] Scrub | [Q] Quit" )
                .alignment( Alignment::Center ).style( Style::default().fg( Color::DarkGray ) ), vertical_chunks[ 3 ] );
        } )?;

        if event::poll( Duration::from_millis( 10 ) )? {
            if let Event::Key( key ) = event::read()? {
                match key.code {
                    KeyCode::Char( 'q' ) | KeyCode::Esc => break,
                    KeyCode::Char( ' ' ) => {
                        is_paused = !is_paused;
                        if is_paused && ext == "sr" { unsafe { sync_sr_progress( current_idx as c_int ); } }
                    },
                    KeyCode::Left => current_idx = current_idx.saturating_sub( 10 ),
                    KeyCode::Right => current_idx = ( current_idx + 10 ).min( words.len() - 1 ),
                    KeyCode::Up => { wpm += 25.0; base_delay = Duration::from_secs_f32( 60.0 / wpm ); }
                    KeyCode::Down => { if wpm > 50.0 { wpm -= 25.0; base_delay = Duration::from_secs_f32( 60.0 / wpm ); } }
                    _ => {}
                }
            }
        }

        if !is_paused && last_tick.elapsed() >= active_delay {
            current_idx += 1;
            if current_idx >= words.len() { break; }
            last_tick = Instant::now();
            active_delay = Duration::from_secs_f32( base_delay.as_secs_f32() * words[ current_idx ].delay_mult );
        }
    }

    if ext == "sr" { unsafe { sync_sr_progress( current_idx as c_int ); } }

    disable_raw_mode()?;
    execute!( terminal.backend_mut(), LeaveAlternateScreen )?;
    terminal.show_cursor()?;
    Ok( () )
}
