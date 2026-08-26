use std::os::raw::{ c_char, c_int, c_float };

/// Represents a single extracted word and its timing metadata.
pub struct RsvpWord {
    pub text: String,
    pub orp_index: usize,
    pub delay_mult: f32,
}

unsafe extern "C" {
    // Standard File Extractors
    pub fn load_pdf_session( file_path: *const c_char, start_page: c_int, end_page: c_int ) -> bool;
    pub fn load_pdf_chapter( file_path: *const c_char, target_chapter: c_int ) -> bool;
    pub fn load_txt_session( file_path: *const c_char ) -> bool;
    pub fn load_docx_session( file_path: *const c_char ) -> bool;
    pub fn load_epub_session( file_path: *const c_char ) -> bool;
    
    // Core Engine Interaction
    pub fn get_next_word( buffer: *mut c_char, max_len: c_int, orp_index: *mut c_int, delay_multiplier: *mut c_float ) -> bool;
    pub fn get_total_words() -> c_int;
    pub fn get_pdf_chapter_count( file_path: *const c_char ) -> c_int;

    // SpeedReader Binary Architecture
    pub fn compile_sr( output_path: *const c_char ) -> bool;
    pub fn load_sr_session( file_path: *const c_char ) -> bool;
    pub fn sync_sr_progress( word_index: c_int );
    pub fn add_sr_chapter( title: *const c_char, word_index: c_int ) -> bool;
    pub fn get_sr_saved_index() -> c_int;
    
    // SR Chapter Metadata
    pub fn get_sr_chapter_count() -> c_int;
    pub fn get_sr_chapter_title( index: c_int ) -> *const c_char;
    pub fn get_sr_chapter_word_by_index( index: c_int ) -> c_int;
    pub fn get_sr_chapter_word( chapter_num: c_int ) -> c_int;
}
