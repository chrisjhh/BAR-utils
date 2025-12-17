use biblearchive::BARFile;
use humansize;
use std::io::{Read, Seek};

macro_rules! oprintln {
    ($out:ident, $($arg:tt)*) => {
        $out.push(format!($($arg)*));
        println!($($arg)*);
    };
}

pub fn details<T: Read + Seek>(bar: BARFile<T>) -> Vec<String> {
    let mut output: Vec<String> = Vec::new();
    let mut pending: Vec<String> = Vec::new();
    oprintln!(output, "Version {}", bar.archive_version());
    oprintln!(output, "{}", bar.bible_version());
    oprintln!(
        output,
        "Size: {}",
        humansize::format_size(bar.len(), humansize::BINARY)
    );

    let mut all_present = true;
    for book in bar.books_in_order() {
        let mut chapters_present: Vec<u8> = Vec::new();
        for (i, chapter) in (1..).zip(book.chapters()) {
            if let Some(chapt) = chapter {
                if i != chapt.chapter_number() {
                    eprintln!(
                        "BAD DATA: Chapter numbers do not match {} != {}",
                        i,
                        chapt.chapter_number()
                    );
                }
                if chapt.book_number() != book.book_number() {
                    eprintln!(
                        "BAD DATA: Book number in chapter {} does not match {} != {}",
                        i,
                        chapt.book_number(),
                        book.book_number()
                    );
                }
                chapters_present.push(i);
            }
        }
        if chapters_present.len() == book.number_of_chapters() as usize {
            if all_present {
                pending.push(format!("{} ✓", book.book_name()))
            } else {
                oprintln!(output, "{} ✓", book.book_name());
            }
        } else {
            all_present = false;
            for line in pending.iter() {
                oprintln!(output, "{}", line);
            }
            pending.clear();
            oprintln!(
                output,
                "{} chapters present: {:?}",
                book.book_name(),
                chapters_present
            );
        }
    }
    if all_present && pending.len() == 66 {
        oprintln!(output, "All books present and complete");
    } else {
        for line in pending.iter() {
            oprintln!(output, "{}", line);
        }
    }
    output
}
