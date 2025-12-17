use biblearchive::BARFile;
use humansize;
use std::{
    io::{Read, Seek},
    time::Duration,
};

macro_rules! oprintln {
    ($out:ident, $($arg:tt)*) => {
        $out.push(format!($($arg)*));
        println!($($arg)*);
    };
}

pub fn details<T: Read + Seek>(bar: BARFile<T>, compression_details: bool) -> Vec<String> {
    let mut output: Vec<String> = Vec::new();
    let mut pending: Vec<String> = Vec::new();
    oprintln!(output, "Version {}", bar.archive_version());
    oprintln!(output, "{}", bar.bible_version());
    let file_size = bar.len();
    oprintln!(
        output,
        "Size: {}",
        humansize::format_size(file_size, humansize::BINARY)
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

    if compression_details {
        let mut compressed_size: u32 = 0;
        let mut uncompressed_size: u32 = 0;
        let mut decompress_time: Duration = Duration::from_secs(0);
        for book in bar.books() {
            for chapter in book.chapters() {
                if let Some(chapt) = chapter {
                    if let Ok(details) = chapt.details() {
                        compressed_size += details.compressed_size;
                        uncompressed_size += details.uncompressed_size;
                        decompress_time += details.decompress_time;
                    }
                }
            }
        }
        oprintln!(
            output,
            "Uncompressed size: {}",
            humansize::format_size(uncompressed_size, humansize::BINARY)
        );
        oprintln!(
            output,
            "Compressed size: {}",
            humansize::format_size(compressed_size, humansize::BINARY)
        );
        let compression = (uncompressed_size - file_size as u32) as f64 / uncompressed_size as f64;
        oprintln!(output, "Compression: {:.0}%", compression * 100.0);
        oprintln!(
            output,
            "Decompression time: {} ms",
            decompress_time.as_millis()
        );
        let speed = uncompressed_size as f64 / (decompress_time.as_secs_f64() * 1000.0);
        oprintln!(
            output,
            "Decompression speed: {}/ms",
            humansize::format_size(speed as u64, humansize::BINARY)
        );
    }

    output
}
