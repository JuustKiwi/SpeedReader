#include <iostream>
#include <vector>
#include <string>
#include <sstream>
#include <cstring>
#include <poppler/cpp/poppler-document.h>
#include <poppler/cpp/poppler-page.h>


static std::vector<std::string> session_words;
static size_t current_word_index = 0;

extern "C" {

bool load_pdf_session( const char* file_path, int start_page, int end_page ) {
	session_words.clear();
	current_word_index = 0;

	poppler::document* doc = poppler::document::load_from_file( file_path );
	if ( !doc ) {
		return false;
	}

	int total_pages = doc->pages();
	
	if ( start_page < 1 ) {
		start_page = 1;
	}
	if ( end_page > total_pages || end_page < start_page ) {
		end_page = total_pages;
	}

	for ( int i = start_page - 1; i < end_page; i++ ) {
		poppler::page* p = doc->create_page( i );
		if ( p ) {
			poppler::ustring ustr = p->text();
			std::vector<char> utf8_bytes = ustr.to_utf8();
			std::string text( utf8_bytes.begin(), utf8_bytes.end() );
			
			std::istringstream iss( text );
			std::string word;
			while ( iss >> word ) {
				session_words.push_back( word );
			}
			delete p;
		}
	}
	
	delete doc;
	return true;
}

int get_total_words() {
	return session_words.size();
}

bool get_next_word( char* buffer, int max_len, int* orp_index, float* delay_multiplier ) {
	if ( current_word_index >= session_words.size() ) {
		return false;
	}

	std::string word = session_words[ current_word_index ];
	current_word_index++;

	int len = word.length();
	
	// 1. Calculate the Optimal Recognition Point
	if ( len == 1 ) {
		*orp_index = 0;
	} else if ( len <= 3 ) {
		*orp_index = 1;
	} else if ( len <= 5 ) {
		*orp_index = 2;
	} else if ( len <= 9 ) {
		*orp_index = 3;
	} else {
		*orp_index = 4;
	}

	// 2. Calculate dynamic reading delay based on punctuation
	*delay_multiplier = 1.0f;
	char last_char = word.back();
	
	if ( last_char == ',' ) {
		*delay_multiplier = 1.5f; // Pause slightly for commas
	} else if ( last_char == '.' || last_char == '?' || last_char == '!' || last_char == ';' ) {
		*delay_multiplier = 2.0f; // Pause longer for sentence endings
	}

	strncpy( buffer, word.c_str(), max_len - 1 );
	buffer[ max_len - 1 ] = '\0';

	return true;
}

} // extern "C"
