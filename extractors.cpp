#include "engine.h"
#include <iostream>
#include <sstream>
#include <regex>
#include <fstream>
#include <cstdio>
#include <memory>

#include <poppler/cpp/poppler-document.h>
#include <poppler/cpp/poppler-page.h>
#include <poppler/GlobalParams.h>
#include <poppler/PDFDoc.h>
#include <poppler/Outline.h>
#include <poppler/Link.h>
#include <poppler/goo/GooString.h>
#include <poppler/Catalog.h>

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
    if ( !globalParams ) globalParams = std::make_unique<GlobalParams>();
    
    auto goo_file = std::make_unique<GooString>( file_path );
    auto doc = std::make_unique<PDFDoc>( std::move( goo_file ) );
    
    if ( !doc->isOk() || !doc->getOutline() ) return false;
    
    const auto *items = doc->getOutline()->getItems();
    if ( !items ) return false;
    
    std::vector<OutlineItem*> all_items;
    flatten_outline( items, all_items );
    
    int current_chapter = 0;
    int found_start = -1;
    int found_end = doc->getNumPages();
    std::regex chapter_regex( R"(^\s*(chapter\s+[0-9]+|chapter\s+[ivxlcdm]+|[ivxlcdm]+)\s*$)", std::regex_constants::icase );

    for ( OutlineItem *item : all_items ) {
        const std::vector<Unicode>& title_uni = item->getTitle();
        std::string title_str;
        for ( Unicode u : title_uni ) title_str += (char)( u & 0xFF );
        
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
                if ( dest ) page_num = dest->isPageRef() ? doc->findPage( dest->getPageRef() ) : dest->getPageNum();
            }
            
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
        return true;
    }
    return false;
}

extern "C" {

bool load_pdf_session( const char* file_path, int start_page, int end_page ) {
    cleanup_session();
    
    poppler::document* doc = poppler::document::load_from_file( file_path );
    if ( !doc ) return false;

    int total_pages = doc->pages();
    if ( start_page < 1 ) start_page = 1;
    if ( end_page > total_pages || end_page < start_page ) end_page = total_pages;

    for ( int i = start_page - 1; i < end_page; i++ ) {
        poppler::page* p = doc->create_page( i );
        if ( p ) {
            poppler::ustring ustr = p->text();
            std::vector<char> utf8_bytes = ustr.to_utf8();
            std::string text( utf8_bytes.begin(), utf8_bytes.end() );
            
            std::istringstream iss( text );
            std::string word;
            while ( iss >> word ) push_processed_word( word );
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

bool load_txt_session( const char* file_path ) {
    cleanup_session();
    std::ifstream file( file_path );
    if ( !file.is_open() ) return false;

    std::string word;
    while ( file >> word ) push_processed_word( word );
    return session_words.size() > 0;
}

bool load_docx_session( const char* file_path ) {
    cleanup_session();
    std::string cmd = std::string( "unzip -p \"" ) + file_path + "\" word/document.xml 2>/dev/null";
    FILE* pipe = popen( cmd.c_str(), "r" );
    if ( !pipe ) return false;

    char buffer[ 1024 ];
    std::string xml_data;
    while ( fgets( buffer, sizeof( buffer ), pipe ) != nullptr ) xml_data += buffer;
    pclose( pipe );

    if ( xml_data.empty() ) return false;

	std::regex xml_tags( "<[^>]+>" );
    std::string plain_text = std::regex_replace( xml_data, xml_tags, " " );

	std::regex html_entities( "&[a-zA-Z0-9#]{1,10};" );
    plain_text = std::regex_replace( plain_text, html_entities, " " );

    std::istringstream iss( plain_text );
    std::string word;
    while ( iss >> word ) push_processed_word( word );
    return session_words.size() > 0;
}

bool load_epub_session( const char* file_path ) {
    cleanup_session();
    std::string cmd = std::string( "unzip -p \"" ) + file_path + "\" \"*.html\" \"*.xhtml\" \"*.htm\" 2>/dev/null";
    FILE* pipe = popen( cmd.c_str(), "r" );
    if ( !pipe ) return false;

    char buffer[ 2048 ];
    std::string html_data;
    while ( fgets( buffer, sizeof( buffer ), pipe ) != nullptr ) html_data += buffer;
    pclose( pipe );

    if ( html_data.empty() ) return false;

	std::regex html_tags( "<[^>]+>" );
    std::string plain_text = std::regex_replace( html_data, html_tags, " " );

	std::regex html_entities( "&[a-zA-Z0-9#]{1,10};" );
    plain_text = std::regex_replace( plain_text, html_entities, " " );

    std::istringstream iss( plain_text );
    std::string word;
    while ( iss >> word ) push_processed_word( word );

    return session_words.size() > 0;
}


int get_pdf_chapter_count( const char* file_path ) {
    if ( !globalParams ) globalParams = std::make_unique<GlobalParams>();
    
    auto goo_file = std::make_unique<GooString>( file_path );
    auto doc = std::make_unique<PDFDoc>( std::move( goo_file ) );
    
    if ( !doc->isOk() || !doc->getOutline() ) return 0;
    
    const auto *items = doc->getOutline()->getItems();
    if ( !items ) return 0;
    
    std::vector<OutlineItem*> all_items;
    flatten_outline( items, all_items );
    
    int count = 0;
    std::regex chapter_regex( R"(^\s*(chapter\s+[0-9]+|chapter\s+[ivxlcdm]+|[ivxlcdm]+)\s*$)", std::regex_constants::icase );

    for ( OutlineItem *item : all_items ) {
        const std::vector<Unicode>& title_uni = item->getTitle();
        std::string title_str;
        for ( Unicode u : title_uni ) title_str += (char)( u & 0xFF );
        
        if ( std::regex_match( title_str, chapter_regex ) ) count++;
    }
    return count;
}

} // extern "C"
