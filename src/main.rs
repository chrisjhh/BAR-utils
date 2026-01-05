use biblearchive::BARFile;
use biblearchive_utils::{Args, Command, details, search, verse};
use clap::{CommandFactory, Parser};
use std::fs;
use std::process::exit;

fn main() {
    let args = Args::parse();
    // First see if an explicit path has been specified
    let mut path = args.file;
    if path.is_none() {
        // Try to get the path from the data dir and the version
        if let Some(dir) = args.datadir {
            if !fs::exists(&dir).unwrap_or(false) {
                eprintln!(
                    "Path specified for datadir does not exist: {}",
                    dir.to_string_lossy()
                );
                exit(1);
            }
            if let Some(version) = args.ver {
                // Try .bar file extension fist
                let mut bar_path = dir.clone();
                bar_path.push(format!("{}.bar", version));
                if fs::exists(&bar_path).unwrap_or(false) {
                    path = Some(bar_path);
                } else {
                    // Try .ibar file extension next
                    let mut ibar_path = dir.clone();
                    ibar_path.push(format!("{}.ibar", version));
                    if fs::exists(&ibar_path).unwrap_or(false) {
                        path = Some(ibar_path);
                    } else {
                        eprintln!(
                            "Cannot find version {} in directory {}.",
                            version,
                            dir.to_string_lossy()
                        );
                        exit(1);
                    }
                }
            } else {
                eprintln!("No path to BARFile or version from datadir specified.");
                exit(1);
            }
        } else {
            eprintln!("Path to BARFile not specified.");
            exit(1);
        }
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
        Some(Command::Search(params)) => search(bar, params),
        None => {
            eprintln!("No command specified.");
            let mut cmd = Args::command().bin_name("bar");
            let _ = cmd.print_help();
            1
        }
    };
    exit(status);
}
