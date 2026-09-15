mod config;
mod ffi;
mod ui;

use std::env;
use std::ffi::{ CString, CStr };
use std::io;
use std::path::Path;
use std::os::raw::{ c_char, c_int };

use crate::ffi::*;
use crate::ui::{ run_viewer_mode, run_rsvp_mode };

fn print_usage() {
    eprintln!( "Usage:" );
    eprintln!( "  Read Standard:   speedreader <file> [-w <word_idx> | -p <virtual_page> | --view]" );
    eprintln!( "  Read PDF:        speedreader <file.pdf> [-c <chapter> | -p <start_page> <end_page> | --view]" );
    eprintln!( "  Read SR Binary:  speedreader <file.sr> [-c <chapter> | -w <word_idx> | -p <virtual_page> | --view]" );
    eprintln!( "  Compile Binary:  speedreader <file> --compile <out.sr>" );
    eprintln!( "  Manage Chapters: speedreader <file.sr> --add-chapter <word_idx> \"<Title>\"" );
    eprintln!( "  List Chapters:   speedreader <file.sr> --list-chapters" );
    eprintln!( "  View Stats:      speedreader <file> --stats" );
    eprintln!( "  Calculate Time:  speedreader <file> --calc <wpm>" );
    eprintln!( "  Calc Words:      speedreader --calc-words <wpm> <words> (Alias: -cw)" );
    eprintln!( "  Calc Pages:      speedreader --calc-pages <wpm> <pages> (Alias: -cp)" );
}

fn handle_compile( ext: &str, file_path: &CStr, out_path: &str ) {
    let c_out = CString::new( out_path ).unwrap();
    unsafe {
        let loaded = match ext {
            "txt" => load_txt_session( file_path.as_ptr() ),
            "docx" => load_docx_session( file_path.as_ptr() ),
            "epub" => load_epub_session( file_path.as_ptr() ),
            "pdf" => load_pdf_session( file_path.as_ptr(), 1, 999999 ), 
            _ => false,
        };
        
        if !loaded {
            eprintln!( "Error: Failed to load source file." );
            return;
        }
        
        if compile_sr( c_out.as_ptr() ) {
            println!( "Successfully compiled binary to {}", out_path );
        } else {
            eprintln!( "Error: Compilation failed." );
        }
    }
}

fn handle_add_chapter( file_path: &CStr, w_idx: c_int, title: &str ) {
    let c_title = CString::new( title ).unwrap();
    unsafe {
        if load_sr_session( file_path.as_ptr() ) {
            if add_sr_chapter( c_title.as_ptr(), w_idx ) {
                println!( "Chapter '{}' successfully added at word {}", title, w_idx );
            } else {
                eprintln!( "Error: Failed to add chapter. Maximum of 50 chapters reached." );
            }
        } else {
            eprintln!( "Error: Failed to load binary file. Make sure it is compiled properly." );
        }
    }
}

fn handle_list_chapters( file_path: &CStr, target_file: &str ) {
    unsafe {
        if !load_sr_session( file_path.as_ptr() ) {
            eprintln!( "Error: Failed to load .sr binary. Make sure the file exists and is compiled." );
            return;
        }
        let count = get_sr_chapter_count();
        if count == 0 {
            println!( "No chapters found in {}.", target_file );
            return;
        }
        
        println!( "\n Chapters in {}:", target_file );
        println!( "------------------------------------------------" );
        for i in 0..count {
            let c_title = get_sr_chapter_title( i );
            let title = if c_title.is_null() { "" } else { CStr::from_ptr( c_title ).to_str().unwrap_or( "" ) };
            let w_idx = get_sr_chapter_word_by_index( i );
            let page = ( w_idx / 250 ) + 1;
            println!( "  {}. {} (Word: {}, Page: {})", i + 1, title, w_idx, page );
        }
        println!( "------------------------------------------------\n" );
    }
}

fn print_time_estimate( wpm: f32, total_words: usize, label: &str ) {
    let mins = total_words as f32 / wpm;
    let hours = (mins / 60.0).floor() as i32;
    let minutes = (mins % 60.0).round() as i32;

    println!( "\nReading Estimate: {}", label );
    println!( "--------------------------------------" );
    println!( "Speed: {} WPM", wpm );
    println!( "Words: {}", total_words );
    println!( "Pages: {} (Virtual 250w/pg)", total_words / 250 );
    println!( "Time:  {} hours, {} minutes", hours, minutes );
    println!( "--------------------------------------\n" );
}

fn handle_calc_file( ext: &str, file_path: &CStr, target_file: &str, wpm: f32 ) {
    unsafe {
        let success = match ext {
            "txt" => load_txt_session( file_path.as_ptr() ),
            "docx" => load_docx_session( file_path.as_ptr() ),
            "epub" => load_epub_session( file_path.as_ptr() ),
            "sr" => load_sr_session( file_path.as_ptr() ),
            "pdf" => load_pdf_session( file_path.as_ptr(), 1, 999999 ), 
            _ => false,
        };
        
        if !success { 
            eprintln!( "Error: Failed to load file for calculation." );
            return; 
        }
        
        let total_words = get_total_words() as usize;
        print_time_estimate( wpm, total_words, target_file );
    }
}


fn handle_stats( ext: &str, file_path: &CStr, target_file: &str ) {
    unsafe {
        let success = match ext {
            "txt" => load_txt_session( file_path.as_ptr() ),
            "docx" => load_docx_session( file_path.as_ptr() ),
            "epub" => load_epub_session( file_path.as_ptr() ),
            "sr" => load_sr_session( file_path.as_ptr() ),
            "pdf" => load_pdf_session( file_path.as_ptr(), 1, 999999 ), 
            _ => false,
        };
        
        if !success { return; }

        let total_words = get_total_words();
        let cfg = crate::config::load_config();
        let mins = total_words as f32 / cfg.wpm;

        println!( "\nDocument Statistics: " );
        println!( "----------------------------" );
        println!( "File:  {}", target_file );
        println!( "Words: {}", total_words );
        println!( "Pages: {} (Virtual 250w/pg)", total_words / 250 );
        if ext == "pdf" { 
            println!( "Chapters: {}", get_pdf_chapter_count( file_path.as_ptr() ) ); 
        }
        println!( "Speed: {} WPM (Slow: {}, Fast: {})", cfg.wpm, cfg.slow_wpm, cfg.fast_wpm );
        println!( "Time:  {} hours, {} minutes", (mins / 60.0).floor() as i32, (mins % 60.0).round() as i32 );
        println!( "----------------------------\n" );
    }
}

fn load_session( ext: &str, file_path: &CStr, args: &[String] ) -> ( Vec<RsvpWord>, usize ) {
    let mut session_words = Vec::new();
    let mut sr_saved_index = 0;

    unsafe {
        let success = match ext {
            "txt" => load_txt_session( file_path.as_ptr() ),
            "docx" => load_docx_session( file_path.as_ptr() ),
            "epub" => load_epub_session( file_path.as_ptr() ),
            "sr" => {
                let s = load_sr_session( file_path.as_ptr() );
                sr_saved_index = get_sr_saved_index() as usize;
                s
            },
            "pdf" => {
                if let Some( idx ) = args.iter().position( |a| a == "-c" || a == "--chapter" ) {
                    let chapter: c_int = args.get( idx + 1 ).and_then( |v| v.parse().ok() ).unwrap_or( 1 );
                    load_pdf_chapter( file_path.as_ptr(), chapter )
                } else if let Some( idx ) = args.iter().position( |a| a == "-p" || a == "--page" || a == "--pages" ) {
                    let start: c_int = args.get( idx + 1 ).and_then( |v| v.parse().ok() ).unwrap_or( 1 );
                    let end: c_int = args.get( idx + 2 ).and_then( |v| v.parse().ok() ).unwrap_or( start );
                    load_pdf_session( file_path.as_ptr(), start, end )
                } else {
                    load_pdf_session( file_path.as_ptr(), 1, 999999 ) 
                }
            },
            _ => false,
        };

        if !success { return ( session_words, sr_saved_index ); }

        let mut buffer = vec![ 0u8; 256 ];
        let mut orp = 0;
        let mut delay = 0.0;

        while get_next_word( buffer.as_mut_ptr() as *mut c_char, 256, &mut orp, &mut delay ) {
            if let Ok( rust_str ) = CStr::from_ptr( buffer.as_ptr() as *const c_char ).to_str() {
                session_words.push( RsvpWord { text: rust_str.to_string(), orp_index: orp as usize, delay_mult: delay } );
            }
        }
    }
    
    ( session_words, sr_saved_index )
}

fn determine_start_index( args: &[String], ext: &str, sr_saved_index: usize, total_words: usize ) -> usize {
    let mut start_idx = if ext == "sr" { sr_saved_index } else { 0 };

    if let Some( idx ) = args.iter().position( |a| a == "-c" || a == "--chapter" ) {
        if ext == "sr" {
            if let Some( val ) = args.get( idx + 1 ) {
                let chapter_num: c_int = val.parse().unwrap_or( 1 );
                unsafe {
                    let c_word = get_sr_chapter_word( chapter_num );
                    if c_word != -1 {
                        start_idx = c_word as usize;
                    } else {
                        eprintln!( "Warning: Chapter {} not found.", chapter_num );
                    }
                }
            }
        }
    } else if let Some( idx ) = args.iter().position( |a| a == "-w" || a == "--word" ) {
        if let Some( val ) = args.get( idx + 1 ) { start_idx = val.parse().unwrap_or( 0 ); }
    } else if let Some( idx ) = args.iter().position( |a| a == "-p" || a == "--page" || a == "--pages" ) {
        if ext != "pdf" {
            if let Some( val ) = args.get( idx + 1 ) {
                let p: usize = val.parse().unwrap_or( 1 );
                start_idx = p.saturating_sub( 1 ) * 250; 
            }
        }
    }
    
    start_idx.min( total_words.saturating_sub( 1 ) )
}


#[derive(Clone, Copy, PartialEq)]
enum Action { Compile, AddChapter, ListChapters, Stats, CalcFile, CalcWords, CalcPages, Read }

struct CmdDef {
    flags: &'static [&'static str],
    action: Action,
    req_args: usize,
    requires_file: bool,
}

const COMMAND_DEFS: &[CmdDef] = &[
    CmdDef { flags: &["--compile"], action: Action::Compile, req_args: 1, requires_file: true },
    CmdDef { flags: &["--add-chapter", "--add_chapter", "-a"], action: Action::AddChapter, req_args: 2, requires_file: true },
    CmdDef { flags: &["--list-chapters", "-l", "--chapters"], action: Action::ListChapters, req_args: 0, requires_file: true },
    CmdDef { flags: &["--stats", "-s"], action: Action::Stats, req_args: 0, requires_file: true },
    CmdDef { flags: &["--calc"], action: Action::CalcFile, req_args: 1, requires_file: true },
    CmdDef { flags: &["--calc-words", "-cw"], action: Action::CalcWords, req_args: 2, requires_file: false },
    CmdDef { flags: &["--calc-pages", "-cp"], action: Action::CalcPages, req_args: 2, requires_file: false },
];


fn main() -> Result< (), io::Error > {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 || args.iter().any( |a| a == "-h" || a == "--help" ) {
        print_usage();
        return Ok( () );
    }

    let mut active_action = Action::Read;
    let mut matched_idx = 0;
    let mut active_def: Option<&CmdDef> = None;

    for def in COMMAND_DEFS {
        if let Some(idx) = args.iter().position(|a| def.flags.contains(&a.as_str())) {
            active_action = def.action;
            matched_idx = idx;
            active_def = Some(def);
            break;
        }
    }

    if let Some(def) = active_def {
        if args.len() <= matched_idx + def.req_args {
            eprintln!("Error: Missing arguments for {}. Expected {} argument(s).", args[matched_idx], def.req_args);
            return Ok(());
        }
        
        if def.requires_file && (args.len() < 2 || args[1].starts_with('-')) {
            eprintln!("Error: The {} command requires a target file.", args[matched_idx]);
            eprintln!("Usage: speedreader <file> {}", args[matched_idx]);
            return Ok(());
        }
    }
    else {
        if args.len() < 2 || args[1].starts_with('-') {
            eprintln!("Error: No target file provided for reading.");
            print_usage();
            return Ok(());
        }
    }
    
    match active_action {
        Action::CalcWords => {
            let wpm = args[matched_idx + 1].parse().unwrap_or(350.0);
            let words = args[matched_idx + 2].parse().unwrap_or(0);
            print_time_estimate(wpm, words, "Custom Word Count");
            return Ok(());
        }
        Action::CalcPages => {
            let wpm = args[matched_idx + 1].parse().unwrap_or(350.0);
            let pages = args[matched_idx + 2].parse().unwrap_or(0);
            print_time_estimate(wpm, pages * 250, "Custom Page Count");
            return Ok(());
        }
        _ => {}
    }

    let target_file = &args[1];
    let file_path = CString::new(target_file.as_str()).expect("String err");
    let ext = Path::new(target_file).extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();

    match active_action {
        Action::Compile => {
            handle_compile(&ext, &file_path, &args[matched_idx + 1]);
            return Ok(());
        }
        Action::AddChapter => {
            if ext != "sr" {
                eprintln!("Error: Chapter management is only supported for compiled .sr binary files.");
                return Ok(());
            }
            let title = &args[matched_idx + 2];
            if title.len() > 63 {
                eprintln!("Error: Chapter title is too long (maximum 63 bytes allowed).");
                return Ok(());
            }
            let w_idx = args[matched_idx + 1].parse().unwrap_or(0);
            handle_add_chapter(&file_path, w_idx, title);
            return Ok(());
        }
        Action::ListChapters => {
            if ext != "sr" {
                eprintln!("Error: Chapter listing is only supported for compiled .sr binary files.");
                return Ok(());
            }
            handle_list_chapters(&file_path, target_file);
            return Ok(());
        }
        Action::Stats => {
            handle_stats(&ext, &file_path, target_file);
            return Ok(());
        }
        Action::CalcFile => {
            let wpm = args[matched_idx + 1].parse().unwrap_or(350.0);
            handle_calc_file(&ext, &file_path, target_file, wpm);
            return Ok(());
        }
        Action::Read => {}
        _ => {}
    }


    let ( words, sr_saved_index ) = load_session( &ext, &file_path, &args );
    if words.is_empty() {
        eprintln!( "No words found or failed to load file." );
        return Ok( () );
    }

    let start_idx = determine_start_index( &args, &ext, sr_saved_index, words.len() );
    let is_view = args.iter().any( |a| a == "--view" || a == "-v" );
    
    if is_view {
        run_viewer_mode( &words, start_idx, &ext )?;
    } else {
        run_rsvp_mode( &words, start_idx, &ext )?;
    }

    Ok( () )
}
