use super::SearchArgs;
use bible_data::{BOOK_ABBREVS, parse_book_abbrev};
use biblearchive::BARFile;
use regex::Regex;
use std::{
    collections::HashMap,
    io::{Read, Seek},
    ops::RangeInclusive,
};

// Filters
fn exclude_all() -> impl Fn(bool, u32) -> bool {
    |_, _| false
}

fn exclude_item(book_number: u32) -> impl Fn(bool, u32) -> bool {
    move |input, book| match input {
        false => false,
        true => book != book_number,
    }
}

fn exclude_item_range(book_range: RangeInclusive<u32>) -> impl Fn(bool, u32) -> bool {
    move |input, book| match input {
        false => false,
        true => !book_range.contains(&book),
    }
}

fn include_item(book_number: u32) -> impl Fn(bool, u32) -> bool {
    move |input, book| match input {
        true => true,
        false => book == book_number,
    }
}

fn include_item_range(book_range: RangeInclusive<u32>) -> impl Fn(bool, u32) -> bool {
    move |input, book| match input {
        true => true,
        false => book_range.contains(&book),
    }
}

// Verse text filters
fn match_phrase(phrase: String) -> impl Fn(&str) -> bool {
    move |verse| verse.find(&phrase).is_some()
}

fn match_regex(regex: Regex) -> impl Fn(&str) -> bool {
    move |verse| regex.is_match(verse)
}

fn match_word(word: &str) -> Box<dyn Fn(&str) -> bool> {
    // Get rid of any non alpha-numerics
    let safe = word.replace(|c: char| !c.is_ascii_alphanumeric() && c != ' ', "");
    // Convert to a regex that will match on word boundaries
    let regex = format!(r"\b{}\b", safe);
    match Regex::new(&regex) {
        Ok(re) => Box::new(match_regex(re)),
        Err(_) => Box::new(|_| false),
    }
}

#[allow(unused_assignments)]
pub fn search<T: Read + Seek>(bar: BARFile<T>, params: &SearchArgs) {
    // Set up the filters required
    let mut book_filters: Vec<Box<dyn Fn(bool, u32) -> bool>> = Vec::new();
    let mut chapter_filters: HashMap<u32, Vec<Box<dyn Fn(bool, u32) -> bool>>> = HashMap::new();
    let mut match_filters: Vec<Box<dyn Fn(&str) -> bool>> = Vec::new();
    let mut must_match_filters: Vec<Box<dyn Fn(&str) -> bool>> = Vec::new();
    let mut exclude_filters: Vec<Box<dyn Fn(&str) -> bool>> = Vec::new();

    // Parse the arguments to populate the filters
    // Path includes for books and chapters
    for m in params.include.iter() {
        let is_exclude = m.starts_with("!");
        let mut s = &m[..];
        if is_exclude {
            s = &m[1..];
        } else if book_filters.is_empty() {
            // If first filter is an include assume everything is initially excluded
            book_filters.push(Box::new(exclude_all()));
        }
        // Check for range cases
        if s == "OT" || s == "NT" || (s.contains("..") && !s.contains(" ")) {
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
                true => Box::new(exclude_item_range(range)),
                false => Box::new(include_item_range(range)),
            });
        } else {
            // It should be an individual book or a chapter
            let book = parse_book_abbrev(s);
            if book.is_none() {
                eprint!("Invalid value for --include: {}.", m);
                return;
            }
            let book = (book.unwrap() + 1) as u32;
            if s.contains(" ") {
                // Chapter reference
                let parts: Vec<&str> = s.split(" ").collect();
                if parts.len() != 2 {
                    eprint!("Invalid argument for --include: {}", m);
                    return;
                }
                let chapter = parts[1].parse::<u32>();
                if chapter.is_err() {
                    eprint!("Invalid chapter in arg for --include: {}", m);
                    return;
                }
                let chapter = chapter.unwrap();
                let is_book_included = book_filters.iter().fold(true, |acc, f| f(acc, book));
                if !is_book_included && !is_exclude {
                    // We want to include a chapter from a book that is currently excluded
                    // First we need to include the book
                    book_filters.push(Box::new(include_item(book)));
                } else if !is_book_included {
                    // No need to exclude chapter from book that is already excluded
                    continue;
                }
                let mut filters = chapter_filters.get_mut(&book);
                if filters.is_none() {
                    let mut new_filters: Vec<Box<dyn Fn(bool, u32) -> bool>> = Vec::new();
                    if !is_exclude {
                        // If first filter is an include assume everything is initially excluded
                        new_filters.push(Box::new(exclude_all()));
                    }
                    chapter_filters.insert(book, new_filters);
                    filters = chapter_filters.get_mut(&book);
                }
                let filters = &mut filters.unwrap();
                filters.push(match is_exclude {
                    true => Box::new(exclude_item(chapter)),
                    false => Box::new(include_item(chapter)),
                });
            }
            book_filters.push(match is_exclude {
                true => Box::new(exclude_item(book)),
                false => Box::new(include_item(book)),
            });
        }
    }

    // Match and exclude filters for verses
    for m in params.matching.iter() {
        let m = m.to_string();
        let is_exclude = m.starts_with("!");
        let is_required = m.starts_with("+");
        let mut s = &m[..];
        if is_exclude || is_required {
            s = &m[1..];
        }
        // Test for regexp
        let filter: Box<dyn Fn(&str) -> bool>;
        if s.starts_with("/") && s.ends_with("/") {
            let s = s[1..s.len() - 1].to_string();
            let regex = Regex::new(&s);
            if regex.is_err() {
                eprint!("Invalid regexp in arg for --include: {}", s);
                return;
            }
            filter = Box::new(match_regex(regex.unwrap()));
        } else {
            filter = Box::new(match_phrase(s.to_string()));
        }
        if is_exclude {
            exclude_filters.push(filter);
        } else if is_required {
            must_match_filters.push(filter);
        } else {
            match_filters.push(filter);
        }
    }

    // Same again for words
    for m in params.word.iter() {
        let is_exclude = m.starts_with("!");
        let is_required = m.starts_with("+");
        let mut s = &m[..];
        if is_exclude || is_required {
            s = &m[1..];
        }
        let filter = match_word(s);
        if is_exclude {
            exclude_filters.push(filter);
        } else if is_required {
            must_match_filters.push(filter);
        } else {
            match_filters.push(filter);
        }
    }

    // Process the books, chapters and verses and find the matches
    // using the created filters
    for book in bar.books_in_order() {
        let b = book.book_number() as u32;
        let should_proccess = book_filters.iter().fold(true, |acc, f| f(acc, b));
        if !should_proccess {
            continue;
        }
        let book_chapt_filters = &chapter_filters.get_mut(&b);
        for chapter in book.chapters() {
            if chapter.is_none() {
                continue;
            }
            let chapter = chapter.unwrap();
            let c = chapter.chapter_number() as u32;
            if let Some(filters) = book_chapt_filters {
                let should_proccess = filters.iter().fold(true, |acc, f| f(acc, c));
                if !should_proccess {
                    continue;
                }
            }
            for (v, verse) in chapter.enumerated_verses() {
                let should_process = (match_filters.is_empty()
                    || match_filters.iter().any(|f| f(&verse)))
                    && must_match_filters.iter().all(|f| f(&verse))
                    && !exclude_filters.iter().any(|f| f(&verse));
                if !should_process {
                    continue;
                }
                println!("{} {}:{} {}", BOOK_ABBREVS[b as usize - 1], c, v, verse);
            }
        }
    }
}
