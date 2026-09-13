#include "engine.h"
#include <cstring>
#include <fstream>
#include <sys/mman.h>
#include <sys/stat.h>
#include <fcntl.h>
#include <unistd.h>

std::vector<std::string> session_words;
size_t current_word_index = 0;

static void* mapped_data = nullptr;
static size_t mapped_size = 0;
static std::vector<const char*> mmap_words;
static bool is_mmap_mode = false;
static SRHeader* current_header = nullptr;

void cleanup_session() {
    if ( mapped_data ) {
        munmap( mapped_data, mapped_size );
        mapped_data = nullptr;
    }
    mmap_words.clear();
    session_words.clear();
    current_word_index = 0;
    current_header = nullptr;
    is_mmap_mode = false;
}

void push_processed_word(const std::string& raw_word) {
    std::string temp = "";
    for (size_t i = 0; i < raw_word.length(); ++i) {
        unsigned char c = raw_word[i];
        
        // Split on standard ASCII hyphen
        if (c == '-') {
            temp += c;
            session_words.push_back(temp);
            temp = "";
        } 
        // Split on UTF-8 em-dash (E2 80 94)
        else if (c == 0xE2 && i + 2 < raw_word.length() && 
                 (unsigned char)raw_word[i+1] == 0x80 && 
                 (unsigned char)raw_word[i+2] == 0x94) {
            temp += raw_word[i];
            temp += raw_word[i+1];
            temp += raw_word[i+2];
            session_words.push_back(temp);
            temp = "";
            i += 2;
        } else {
            temp += c;
        }
    }
    if (!temp.empty()) {
        session_words.push_back(temp);
    }
}


extern "C" {

bool compile_sr( const char* output_path ) {
    if ( session_words.empty() ) return false;

    std::ofstream out( output_path, std::ios::binary );
    if ( !out ) return false;

    SRHeader header = {};
    header.magic[0] = 'S'; header.magic[1] = 'R'; header.magic[2] = '0'; header.magic[3] = '1';
    header.saved_index = 0;
    header.total_words = session_words.size();
    header.chapter_count = 0;

    out.write( reinterpret_cast<const char*>( &header ), sizeof( SRHeader ) );

    for ( const auto& w : session_words ) {
        out.write( w.c_str(), w.length() );
        char null_term = '\0';
        out.write( &null_term, 1 );
    }
    
    return true;
}

bool load_sr_session( const char* file_path ) {
    cleanup_session();
    is_mmap_mode = true;

    int fd = open( file_path, O_RDWR );
    if ( fd < 0 ) return false;

    struct stat sb;
    if ( fstat( fd, &sb ) == -1 ) { close( fd ); return false; }
    mapped_size = sb.st_size;

    mapped_data = mmap( nullptr, mapped_size, PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0 );
    close( fd ); 

    if ( mapped_data == MAP_FAILED ) {
        mapped_data = nullptr;
        return false;
    }

    current_header = reinterpret_cast<SRHeader*>( mapped_data );
    if ( strncmp( current_header->magic, "SR01", 4 ) != 0 ) return false;

    char* payload = reinterpret_cast<char*>( mapped_data ) + sizeof( SRHeader );
    size_t payload_size = mapped_size - sizeof( SRHeader );

    char* ptr = payload;
    char* end = payload + payload_size;
    while ( ptr < end ) {
        mmap_words.push_back( ptr );
        while ( ptr < end && *ptr != '\0' ) ptr++;
        ptr++;
    }

    current_word_index = 0;
    return true;
}

void sync_sr_progress( int word_index ) {
    if ( is_mmap_mode && current_header ) {
        current_header->saved_index = word_index;
        msync( mapped_data, sizeof( SRHeader ), MS_ASYNC );
    }
}

bool add_sr_chapter( const char* title, int word_index ) {
    if ( !is_mmap_mode || !current_header ) {
        return false;
    }

    if ( current_header->chapter_count >= 50 ) {
        return false;
    }
	
    int idx = current_header->chapter_count;
    current_header->chapters[ idx ].word_index = word_index;
    strncpy( current_header->chapters[ idx ].title, title, 63 );
    current_header->chapters[ idx ].title[ 63 ] = '\0';
    current_header->chapter_count++;

    msync( mapped_data, sizeof( SRHeader ), MS_ASYNC );
    return true;
}

int get_sr_saved_index() {
    if ( is_mmap_mode && current_header ) return current_header->saved_index;
       return 0;
}

int get_sr_chapter_word( int chapter_num ) {
    if ( !is_mmap_mode || !current_header ) {
        return -1;
    }

	
    if ( chapter_num >= 1 && chapter_num <= (int)current_header->chapter_count ) {
        return current_header->chapters[ chapter_num - 1 ].word_index;
    }

    return -1;
}

int get_sr_chapter_count() {
    if ( is_mmap_mode && current_header ) return current_header->chapter_count;
    return 0;
}

const char* get_sr_chapter_title( int index ) {
    if ( is_mmap_mode && current_header && index >= 0 && index < (int)current_header->chapter_count ) {
        return current_header->chapters[ index ].title;
    }
    return "";
}

int get_sr_chapter_word_by_index( int index ) {
    if ( is_mmap_mode && current_header && index >= 0 && index < (int)current_header->chapter_count ) {
        return current_header->chapters[ index ].word_index;
    }
    return -1;
}

int get_total_words() {
    return is_mmap_mode ? mmap_words.size() : session_words.size();
}

bool get_next_word( char* buffer, int max_len, int* orp_index, float* delay_multiplier ) {
    size_t total = is_mmap_mode ? mmap_words.size() : session_words.size();
    if ( current_word_index >= total ) return false;

    std::string word;
    if ( is_mmap_mode ) {
        word = mmap_words[ current_word_index ];
    } else {
        word = session_words[ current_word_index ];
    }
    
    current_word_index++;
    int len = word.length();
    
    if ( len == 1 ) *orp_index = 0;
    else if ( len <= 3 ) *orp_index = 1;
    else if ( len <= 5 ) *orp_index = 2;
    else if ( len <= 9 ) *orp_index = 3;
    else *orp_index = 4;

	*delay_multiplier = 1.0f;
    
    if (len > 0) {
        char last_char = word.back();
        if ( last_char == ',' || last_char == '-' ) *delay_multiplier = 1.5f; 
        else if ( last_char == '.' || last_char == '?' || last_char == '!' || last_char == ';' ) *delay_multiplier = 2.0f; 
    }

    // Slowdown if the word is too big
    if (len > 12) {
        *delay_multiplier += 0.8f; 
    } else if (len > 8) {
        *delay_multiplier += 0.4f; 
    }

    strncpy( buffer, word.c_str(), max_len - 1 );
    buffer[ max_len - 1 ] = '\0';

    return true;
}

} // extern "C"

