#ifndef ENGINE_H
#define ENGINE_H

#include <vector>
#include <string>
#include <cstdint>

#pragma pack(push, 1)

struct SRChapter {
    uint32_t word_index;
    char title[64];
};

#pragma pack(pop)

struct SRHeader {
    char magic[4];           // "SR01" Identifies it as a SpeedReader file
    uint32_t saved_index;    // The word you stopped reading at
    uint32_t total_words;    // Total words in the payload
    uint32_t chapter_count;  // How many custom chapters exist
    SRChapter chapters[50];  // Pre-allocated space for up to 50 chapters
};


extern std::vector<std::string> session_words;
extern size_t current_word_index;

void cleanup_session();

void push_processed_word( const std::string& raw_word );

// Exposed to rust
extern "C" {
    bool load_pdf_session( const char* file_path, int start_page, int end_page );
    bool load_pdf_chapter( const char* file_path, int target_chapter );
    bool load_txt_session( const char* file_path );
    bool load_docx_session( const char* file_path );
    bool load_epub_session( const char* file_path );
    int get_pdf_chapter_count( const char* file_path );
    
    int get_total_words();
    bool get_next_word( char* buffer, int max_len, int* orp_index, float* delay_multiplier );
    
    bool compile_sr( const char* output_path );
    bool load_sr_session( const char* file_path );
    void sync_sr_progress( int word_index );
    bool add_sr_chapter( const char* title, int word_index );
    int get_sr_saved_index();
    int get_sr_chapter_word( int chapter_num );
    
    int get_sr_chapter_count();
    const char* get_sr_chapter_title( int index );
    int get_sr_chapter_word_by_index( int index );
}

#endif // ENGINE_H
