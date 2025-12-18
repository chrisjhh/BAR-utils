use super::SearchArgs;
use bible_data::parse_book_abbrev;
use biblearchive::BARFile;
use std::{
    io::{Read, Seek},
    ops::RangeInclusive,
};

// Book filters
fn exclude_all() -> impl Fn(bool, u32) -> bool {
    |_, _| false
}

fn exclude_book(book_number: u32) -> impl Fn(bool, u32) -> bool {
    move |input, book| match input {
        false => false,
        true => book != book_number,
    }
}

fn exclude_book_range(book_range: RangeInclusive<u32>) -> impl Fn(bool, u32) -> bool {
    move |input, book| match input {
        false => false,
        true => !book_range.contains(&book),
    }
}

fn include_book(book_number: u32) -> impl Fn(bool, u32) -> bool {
    move |input, book| match input {
        true => true,
        false => book == book_number,
    }
}

fn include_book_range(book_range: RangeInclusive<u32>) -> impl Fn(bool, u32) -> bool {
    move |input, book| match input {
        true => true,
        false => book_range.contains(&book),
    }
}

#[allow(unused_assignments)]
pub fn search<T: Read + Seek>(bar: BARFile<T>, params: &SearchArgs) {
    // Process the book filters
    let mut book_filters: Vec<Box<dyn Fn(bool, u32) -> bool>> = Vec::new();
    for m in params.include.iter() {
        let is_exclude = m.starts_with("!");
        let mut s = &m[..];
        if is_exclude {
            s = &m[1..];
        } else if book_filters.is_empty() {
            book_filters.push(Box::new(exclude_all()));
        }
        // Check for range cases
        if s == "OT" || s == "NT" || s.contains("..") {
            let mut range = 1_u32..=66_u32;
            if s == "OT" {
                range = 1..=39;
            } else if s == "NT" {
                range = 40..=66;
            } else {
                let parts: Vec<&str> = s.split("..").collect();
                if parts.len() != 2 {
                    eprint!("Invalid argument for --include: {}", m);
                    return;
                }
                let start = parse_book_abbrev(parts[0]);
                let end = parse_book_abbrev(parts[1]);
                if start.is_none() || end.is_none() {
                    eprint!("Invalid range for --include: {}", m);
                    return;
                }
                let start = start.unwrap() as u32;
                let end = end.unwrap() as u32;
                if end < start {
                    eprint!(
                        "Invalid range for --include: {}. {} is after {}",
                        m, parts[0], parts[1]
                    );
                    return;
                }
                range = (start + 1)..=(end + 1);
            }
            book_filters.push(match is_exclude {
                true => Box::new(exclude_book_range(range)),
                false => Box::new(include_book_range(range)),
            });
        } else {
            // It should be an individual book or a chapter
            //TODO: Deal with chapters
            let book = parse_book_abbrev(s);
            if book.is_none() {
                eprint!("Invalid value for --include: {}.", m);
                return;
            }
            let book = book.unwrap() as u32;
            book_filters.push(match is_exclude {
                true => Box::new(exclude_book(book + 1)),
                false => Box::new(include_book(book + 1)),
            });
        }
    }
    for book in bar.books_in_order() {
        let i = book.book_number() as u32;
        let should_proccess = book_filters.iter().fold(true, |acc, f| f(acc, i));
        if !should_proccess {
            continue;
        }
        println!("Processing {}", book.book_name());
    }
}
