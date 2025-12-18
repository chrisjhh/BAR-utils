use biblearchive::BARFile;
use biblearchive_utils::{Args, Command, details, search, verse};
use clap::Parser;
use std::process::exit;

fn main() {
    let args = Args::parse();
    let path = args.file.as_deref();
    if path.is_none() {
        eprintln!("Path to BARFile not specified.");
        exit(1);
    }
    let path = path.unwrap();
    let bar = BARFile::open(path);
    if let Err(error) = bar {
        eprintln!("Error opening BARFile.");
        eprintln!("{}", error);
        exit(1);
    }
    let bar = bar.unwrap();

    let status = match &args.command {
        Some(Command::Details { compression }) => {
            details(bar, *compression);
            0
        }
        Some(Command::Verse { verses }) => verse(bar, verses),
        Some(Command::Search(params)) => {
            search(bar, params);
            0
        }
        None => {
            eprintln!("No command specified.");
            1
        }
    };
    exit(status);
}
