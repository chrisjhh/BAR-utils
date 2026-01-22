use super::SearchArgs;
use bible_data::{BOOK_ABBREVS, BibleBookOrChapter, parse_book_abbrev};
use biblearchive::BARFile;
use regex::{Regex, RegexBuilder};
use std::{
    collections::HashMap,
    error::Error,
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
    match word_regexp(word) {
        Some(re) => Box::new(match_regex(re)),
        None => Box::new(|_| false),
    }
}

fn word_regexp(word: &str) -> Option<Regex> {
    // Get rid of any non alpha-numerics
    let safe = word.replace(|c: char| !c.is_ascii_alphanumeric() && c != ' ', "");
    // If word is all lower-case assume we want case-insensitive search
    let ignore_case = safe.chars().all(|c| c.is_ascii_lowercase());
    // Convert to a regex that will match on word boundaries
    let regex = format!(r"\b{}\b", safe);
    RegexBuilder::new(&regex)
        .case_insensitive(ignore_case)
        .build()
        .ok()
}

pub fn search<T: Read + Seek>(bar: BARFile<T>, params: &SearchArgs) -> i32 {
    match search_internal(bar, params) {
        Err(error) => {
            eprintln!("Error while performing search");
            eprintln!("{}", error);
            1
        }
        Ok(_) => 0,
    }
}

fn search_internal<T: Read + Seek>(
    bar: BARFile<T>,
    params: &SearchArgs,
) -> Result<Vec<String>, Box<dyn Error>> {
    let mut output: Vec<String> = Vec::new();
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
            let range;
            if s == "OT" {
                range = 1..=39;
            } else if s == "NT" {
                range = 40..=66;
            } else {
                let parts: Vec<&str> = s.split("..").collect();
                if parts.len() != 2 {
                    return Err(format!("Invalid argument for --include: {}", m).into());
                }
                let start = parse_book_abbrev(parts[0]);
                let end = parse_book_abbrev(parts[1]);
                if start.is_none() || end.is_none() {
                    return Err(format!("Invalid range for --include: {}", m).into());
                }
                let start = start.unwrap() as u32;
                let end = end.unwrap() as u32;
                if end < start {
                    return Err(format!(
                        "Invalid range for --include: {}. {} is after {}",
                        m, parts[0], parts[1]
                    )
                    .into());
                }
                range = (start + 1)..=(end + 1);
            }
            book_filters.push(match is_exclude {
                true => Box::new(exclude_item_range(range)),
                false => Box::new(include_item_range(range)),
            });
        } else {
            // It should be an individual book or a chapter
            match BibleBookOrChapter::parse(s) {
                Some(BibleBookOrChapter::Book(book)) => {
                    book_filters.push(match is_exclude {
                        true => Box::new(exclude_item(book.book_number())),
                        false => Box::new(include_item(book.book_number())),
                    });
                }
                Some(BibleBookOrChapter::Chapter(chapt)) => {
                    let book = chapt.book.book_number();
                    let chapter = chapt.chapter as u32;
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
                _ => return Err(format!("Invalid value for --include: {}.", m).into()),
            }
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
        if s.starts_with("/") && (s.ends_with("/") || s.ends_with("/i")) {
            let mut ignore_case = false;
            if s.ends_with("i") {
                ignore_case = true;
                s = &s[..s.len() - 1];
            }
            s = &s[1..s.len() - 1];
            let regex = RegexBuilder::new(s).case_insensitive(ignore_case).build();
            if regex.is_err() {
                return Err(format!("Invalid regexp in arg for --include: {}", s).into());
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
    let mut count = 0;
    let mut word_count = 0;
    // We should keep a wrod count (not just a verse count) if there is a single match to count
    let should_word_count = params.count && params.word.len() == 1 && params.matching.len() == 0;
    let word_matcher: Option<Regex> = if should_word_count {
        let word = &params.word[0];
        word_regexp(word)
    } else {
        None
    };
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
            let mut chapter_count = 0;
            let mut chapter_word_count = 0;
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
                if !params.count {
                    oprintln!(
                        output,
                        "{} {}:{} {}",
                        BOOK_ABBREVS[b as usize - 1],
                        c,
                        v,
                        verse
                    );
                }
                chapter_count += 1;
                count += 1;
                if should_word_count && word_matcher.is_some() {
                    let wc = word_matcher.as_ref().unwrap().find_iter(&verse).count();
                    chapter_word_count += wc;
                    word_count += wc;
                }
            }
            if params.count && chapter_count > 0 {
                // Display count if count is above threshold
                if params.threshold.is_none()
                    || chapter_count >= params.threshold.unwrap()
                    || chapter_word_count as u32 >= params.threshold.unwrap()
                {
                    let extra = if should_word_count {
                        format!(" (word count: {})", chapter_word_count)
                    } else {
                        "".to_string()
                    };
                    oprintln!(
                        output,
                        "{} {}: {}{}",
                        BOOK_ABBREVS[b as usize - 1],
                        c,
                        chapter_count,
                        extra
                    );
                }
            }
        }
    }
    if params.count {
        let extra = if should_word_count {
            format!(" (word count: {})", word_count)
        } else {
            "".to_string()
        };
        oprintln!(output, "Total: {}{}", count, extra);
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;

    fn barfile() -> BARFile<File> {
        BARFile::open("tests/data/KJV.ibar").unwrap()
    }

    #[test]
    fn test_ps119_without_commandments() {
        let params = SearchArgs {
            matching: vec![
                "!word",
                "!commandment",
                "!judgment",
                "!law",
                "!precept",
                "!statute",
                "!testimon",
                "!ordinance",
                "!way",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            word: vec![],
            include: vec!["Ps 119".to_string()],
            count: false,
            threshold: None,
        };
        let output = search_internal(barfile(), &params).unwrap();
        assert_eq!(
            output,
            vec![
                "Ps 119:90 Thy faithfulness is unto all generations: thou hast established the earth, and it abideth.",
                "Ps 119:122 Be surety for thy servant for good: let not the proud oppress me.",
                "Ps 119:132 Look thou upon me, and be merciful unto me, as thou usest to do unto those that love thy name."
            ]
        )
    }

    #[test]
    fn test_ps119_praise() {
        let params = SearchArgs {
            matching: vec!["/praise/".to_string()],
            word: vec![],
            include: vec!["Ps 119".to_string()],
            count: false,
            threshold: None,
        };
        let output = search_internal(barfile(), &params).unwrap();
        assert_eq!(
            output,
            vec![
                "Ps 119:7 I will praise thee with uprightness of heart, when I shall have learned thy righteous judgments.",
                "Ps 119:164 Seven times a day do I praise thee because of thy righteous judgments.",
                "Ps 119:171 My lips shall utter praise, when thou hast taught me thy statutes.",
                "Ps 119:175 Let my soul live, and it shall praise thee; and let thy judgments help me."
            ]
        )
    }

    #[test]
    fn test_count_sevens() {
        let params = SearchArgs {
            matching: vec![],
            word: vec!["seven".to_string()],
            include: vec!["NT".to_string()],
            count: true,
            threshold: Some(7),
        };
        let output = search_internal(barfile(), &params).unwrap();
        assert_eq!(
            output,
            vec![
                "Rev 1: 6 (word count: 12)",
                "Rev 15: 4 (word count: 8)",
                "Rev 17: 6 (word count: 8)",
                "Total: 65 (word count: 91)"
            ]
        )
    }

    #[test]
    fn test_establish_regex_range() {
        let params = SearchArgs {
            matching: vec!["/establish(ed|ing)?/".to_string()],
            word: vec![],
            include: vec!["Mt..Jn".to_string(), "1Pe..2Pe".to_string()],
            count: false,
            threshold: None,
        };
        let output = search_internal(barfile(), &params).unwrap();
        assert_eq!(
            output,
            vec![
                "Mt 18:17 But if he will not hear thee, then take with thee one or two more, that in the mouth of two or three witnesses every word may be established.",
                "2Pe 1:12 Wherefore I will not be negligent to put you always in remembrance of these things, though ye know them, and be established in the present truth.",
            ]
        )
    }
}
