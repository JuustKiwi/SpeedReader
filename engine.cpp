#include <iostream>
#include <vector>
#include <string>
#include <sstream>
#include <cstring>
#include <regex>
#include <memory>

#include <poppler/cpp/poppler-document.h>
#include <poppler/cpp/poppler-page.h>

#include <poppler/GlobalParams.h>
#include <poppler/PDFDoc.h>
#include <poppler/Outline.h>
#include <poppler/Link.h>
#include <poppler/goo/GooString.h>
#include <poppler/Catalog.h>

static std::vector<std::string> session_words;
static size_t current_word_index = 0;

void flatten_outline( const std::vector<OutlineItem*>* items, std::vector<OutlineItem*>& flat_list ) {
	if ( !items ) return;
	for ( OutlineItem *item : *items ) {
		flat_list.push_back( item );
		if ( item->hasKids() ) {
			item->open(); 
			if ( item->getKids() ) {
				flatten_outline( item->getKids(), flat_list );
			}
		}
	}
}

bool get_metadata_chapter_bounds( const char* file_path, int target_chapter, int* start_page, int* end_page ) {
	if ( !globalParams ) {
		globalParams = std::make_unique<GlobalParams>();
	}
	
	auto goo_file = std::make_unique<GooString>( file_path );
	auto doc = std::make_unique<PDFDoc>( std::move( goo_file ) );
	
	if ( !doc->isOk() || !doc->getOutline() ) {
//		std::cout << "[DEBUG] Document failed to load or has no outline.\n";
		return false;
	}
	
	const auto *items = doc->getOutline()->getItems();
	if ( !items ) {
//		std::cout << "[DEBUG] Outline has no items.\n";
		return false;
	}
	
	std::vector<OutlineItem*> all_items;
	flatten_outline( items, all_items );
	
//	std::cout << "[DEBUG] Flattened TOC contains " << all_items.size() << " items.\n";
	
	int current_chapter = 0;
	int found_start = -1;
	int found_end = doc->getNumPages();
	
	std::regex chapter_regex( R"(^\s*(chapter\s+[0-9]+|chapter\s+[ivxlcdm]+|[ivxlcdm]+)\s*$)", std::regex_constants::icase );

	for ( OutlineItem *item : all_items ) {
		const std::vector<Unicode>& title_uni = item->getTitle();
		std::string title_str;
		for ( Unicode u : title_uni ) {
			title_str += (char)( u & 0xFF );
		}
		
		if ( std::regex_match( title_str, chapter_regex ) ) {
			current_chapter++;
			
			int page_num = -1;
			const LinkAction *action = item->getAction();
			if ( action && action->getKind() == actionGoTo ) {
				const LinkGoTo *goto_action = static_cast<const LinkGoTo*>( action );
				const LinkDest *dest = goto_action->getDest();
				
				std::unique_ptr<LinkDest> resolved_dest;
				if ( !dest ) {
					const GooString *named = goto_action->getNamedDest();
					if ( named ) {
						resolved_dest = doc->getCatalog()->findDest( named );
						dest = resolved_dest.get();
					}
				}
				
				if ( dest ) {
					page_num = dest->isPageRef() ? doc->findPage( dest->getPageRef() ) : dest->getPageNum();
				}
			}
			
//			std::cout << "[DEBUG] Found Chapter " << current_chapter << " ('" << title_str << "') -> Resolved Page: " << page_num << "\n";
			
			if ( current_chapter == target_chapter && page_num != -1 ) {
				found_start = page_num;
			} else if ( current_chapter == target_chapter + 1 && page_num != -1 ) {
				found_end = page_num - 1; 
				break; 
			}
		}
	}
	
	if ( found_start != -1 ) {
		*start_page = found_start;
		*end_page = found_end;
//		std::cout << "[DEBUG] SUCCESS! Extracting pages " << *start_page << " to " << *end_page << "...\n";
		return true;
	}
	
//	std::cout << "[DEBUG] FAILED to find chapter " << target_chapter << " with a valid page number.\n";
	return false;
}

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

bool load_pdf_chapter( const char* file_path, int target_chapter ) {
	int start_page = -1;
	int end_page = -1;
	
	if ( get_metadata_chapter_bounds( file_path, target_chapter, &start_page, &end_page ) ) {
		return load_pdf_session( file_path, start_page, end_page );
	}
	
	return false;
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

	*delay_multiplier = 1.0f;
	char last_char = word.back();
	
	if ( last_char == ',' ) {
		*delay_multiplier = 1.5f; 
	} else if ( last_char == '.' || last_char == '?' || last_char == '!' || last_char == ';' ) {
		*delay_multiplier = 2.0f; 
	}

	strncpy( buffer, word.c_str(), max_len - 1 );
	buffer[ max_len - 1 ] = '\0';

	return true;
}

} // extern "C"
