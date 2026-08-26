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
        let ( wpm, _, _ ) = crate::config::load_config();
        let mins = total_words as f32 / wpm;

        println!( "\nDocument Statistics: " );
        println!( "----------------------------" );
        println!( "File:  {}", target_file );
        println!( "Words: {}", total_words );
        println!( "Pages: {} (Virtual 250w/pg)", total_words / 250 );
        if ext == "pdf" { 
            println!( "Chapters: {}", get_pdf_chapter_count( file_path.as_ptr() ) ); 
        }
        println!( "Speed: {} WPM", wpm );
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

fn main() -> Result< (), io::Error > {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 || args.iter().any( |a| a == "-h" || a == "--help" ) {
        print_usage();
        return Ok( () );
    }

    let target_file = &args[ 1 ];
    let file_path = CString::new( target_file.as_str() ).expect( "String err" );
    let ext = Path::new( target_file ).extension().and_then( |s| s.to_str() ).unwrap_or( "" ).to_lowercase();

    if let Some( idx ) = args.iter().position( |a| a == "--compile" ) {
        if let Some( out_path ) = args.get( idx + 1 ) {
            handle_compile( &ext, &file_path, out_path );
            return Ok( () ); 
        }
    }

    if let Some( idx ) = args.iter().position( |a| a == "--add-chapter" || a == "--add_chapter" || a == "-a" ) {
        if ext != "sr" {
            eprintln!( "Error: Chapter management is only supported for compiled .sr binary files." );
            return Ok( () ); 
        }
        if args.len() > idx + 2 {
            let w_idx: c_int = args[ idx + 1 ].parse().unwrap_or( 0 );
            let title = &args[ idx + 2 ];
            
            if title.len() > 63 {
                eprintln!( "Error: Chapter title is too long (maximum 63 bytes allowed)." );
                return Ok( () ); 
            }
            
            handle_add_chapter( &file_path, w_idx, title );
            return Ok( () ); 
        } else {
            eprintln!( "Error: Missing arguments for --add-chapter. Expected word index and title." );
            return Ok( () ); 
        }
    }
    
    if args.iter().any( |a| a == "--list-chapters" || a == "-l" || a == "--chapters" ) {
        if ext != "sr" {
            eprintln!( "Error: Chapter listing is only supported for compiled .sr binary files." );
            return Ok( () );
        }
        handle_list_chapters( &file_path, target_file );
        return Ok( () ); 
    }

    if args.iter().any( |a| a == "--stats" || a == "-s" ) {
        handle_stats( &ext, &file_path, target_file );
        return Ok( () ); 
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
