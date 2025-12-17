use biblearchive::BARFile;
use biblearchive_utils::details;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::exit;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// Specify the path to the BARFile to use
    #[arg(short, long)]
    file: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// List details about the BARFile
    Details,
}

fn main() {
    let args = Args::parse();
    let path = args.file.as_deref();
    if path.is_none() {
        println!("Path to BARFile not specified.");
        exit(1);
    }
    let path = path.unwrap();
    let bar = BARFile::open(path.to_str().unwrap());
    if let Err(error) = bar {
        println!("Error opening BARFile.");
        println!("{}", error);
        exit(1);
    }
    let bar = bar.unwrap();

    match &args.command {
        Some(Command::Details) => {
            details(bar);
        }
        None => (),
    }
}
