use bible_data::parse_book_abbrev;
use biblearchive::BARFile;
use std::io::{Read, Seek};

pub fn verse<T: Read + Seek>(bar: BARFile<T>, verses: &Vec<String>) -> Vec<String> {
    let mut output: Vec<String> = Vec::new();
    for verse in verses {
        let book = parse_book_abbrev(verse);
        if book.is_none() {
            eprintln!("Invalid verse reference: {}", verse);
            continue;
        }
        let book = book.unwrap();
        let parts: Vec<&str> = verse.split(" ").collect();
        if parts.len() > 3 {
            eprintln!("Too many parts in verse reference: {}", verse);
            continue;
        }
        let refs: Vec<&str> = parts[1].split(":").collect();
        if refs.len() != 2 {
            eprintln!("Unexpected chapter:verse : {}", parts[1]);
            continue;
        }
        let chapt = refs[0].parse();
        if chapt.is_err() {
            eprintln!("Non-numeric chapter: {}", refs[0]);
            continue;
        }
        let chapt = chapt.unwrap();
        let verse_number = refs[1].parse();
        if verse_number.is_err() {
            eprintln!("Non-numeric verse: {}", refs[1]);
            continue;
        }
        let verse_number = verse_number.unwrap();

        if let Some(book) = bar.book((book + 1) as u8) {
            if let Some(chapt) = book.chapter(chapt) {
                if let Ok(verse_text) = chapt.verse_text(verse_number) {
                    oprintln!(output, "{} {}", verse, verse_text);
                }
            }
        }
    }
    output
}
